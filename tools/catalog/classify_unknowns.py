#!/usr/bin/env python3
"""CAT-5: classify every `unknown` game-.text procedure of gta_sa.exe 1.0 US.

Reads the exe and the catalogue (procedures.csv from merge_ghidra.py,
functions.csv/callees.csv from build_catalog.py) and writes
<catalog-dir>/classification.csv with one row per unknown procedure:

    address, class, subsystem, name, evidence

Classes (see docs/re/cat5-unknowns.md for the evidence behind each):
    excluded:eh-funclet          MSVC C++ EH unwind funclet / catch block, listed in a
                                 FuncInfo reached from a `mov eax, FuncInfo; jmp
                                 ___CxxFrameHandler` stub
    excluded:crt                 statically linked MSVC runtime (CRT, EH runtime, delayimp)
    excluded:linker-thunk        linker-generated import (`jmp [IAT]`) and delay-load thunks
    excluded:protection          SecuROM displaced-code fragment (not a function)
    excluded:mis-split           Ghidra split a piece of another function into its own
                                 procedure (switch case, in-place tail); not a function
    excluded:thirdparty:<lib>    statically linked third-party library code
    RW                           RenderWare library code
    game                         real game code; subsystem from a name/vtable/callers if
                                 that can be established, else UNK-game
    unknown                      nothing could be proven (evidence column says why)

Usage: classify_unknowns.py <gta_sa.exe> <catalog-dir> [gta-reversed-dir] [plugin-sdk-dir]
       (defaults: re/gta-reversed and re/plugin-sdk next to the catalog dir's parent)
Needs capstone + pefile (.venv). Run with .venv/bin/python -I.
"""
import bisect
import collections
import csv
import glob
import os
import re
import struct
import sys
from pathlib import Path

import capstone
import pefile
from capstone import x86_const as X

sys.path.insert(0, str(Path(__file__).resolve().parent))
import subsystems as SUB  # noqa: E402  (RULES / classify for owner -> subsystem)
from vtables import VTables, implementation_name  # noqa: E402  (validated slot names, CAT-12)

BASE = 0x400000
GAME_TEXT = (0x401000, 0x857000)
RWCSEG = (0x857000, 0x858000)
HOODLUM = (0x1556000, 0x1577000)
TEXT_HA = (0xC9E000, 0xCAF000)
CODE_LO, CODE_HI = 0x401000, 0x858000

# Third-party link blocks inside the RenderWare part of .text. Bounds are the first and
# last procedure of the block; each block must contain its identifying strings and no
# gta-reversed / plugin-sdk known function (both checked at run time, see check_block).
THIRD_PARTY = [
    # (class suffix, first proc, end (exclusive), identifying-string regex)
    ('d3dx9', 0x7652AE, 0x7C45E0,
     r'D3DX_VERSION|DisableD3DXPSGP|Software\\Microsoft\\Direct3D|shader version|'
     r'preprocessor|vs_3_0|D3DX'),
    ('libpng-1.0.12+zlib-1.1.3', 0x7CFE00, 0x7EB5C0,
     r'libpng|IHDR|IDAT|PLTE|chunk|zlib|incorrect header check|invalid (block type|'
     r'distance code|window size|bit length repeat|literal/length code)|tree'),
    ('quicktime-qtml', 0x818430, 0x81E406,
     r'QuickTime\.qts|_CallComponent|theQuickTimeDispatcher'),
]
ZLIB_RE = re.compile(r'incorrect (header|data) check|invalid (window size|block type|stored block|'
                     r'bit length repeat|distance code|literal/length code)|too many length|'
                     r'unknown compression method|need dictionary|(lengths|distance|literal/length) tree')
CXX_FRAME_HANDLER_FID = '___CxxFrameHandler'
FUNCINFO_MAGIC = (0x19930520, 0x19930521, 0x19930522)


class Image:
    def __init__(self, exe):
        self.pe = pefile.PE(exe)
        self.img = self.pe.get_memory_mapped_image()
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.md.detail = True
        self.iat = {}
        for e in getattr(self.pe, 'DIRECTORY_ENTRY_IMPORT', []):
            for i in e.imports:
                self.iat[i.address] = e.dll.decode() + '!' + (i.name.decode() if i.name else f'#{i.ordinal}')
        self.delay_iat = {}
        for e in getattr(self.pe, 'DIRECTORY_ENTRY_DELAY_IMPORT', []):
            for i in e.imports:
                self.delay_iat[i.address] = e.dll.decode() + '!' + (i.name.decode() if i.name else f'#{i.ordinal}')

    def rd(self, va, n):
        return self.img[va - BASE:va - BASE + n]

    def u32(self, va):
        return struct.unpack('<I', self.rd(va, 4))[0]

    def cstr(self, va):
        z = self.rd(va, 96).split(b'\0')[0]
        if len(z) >= 3 and all(32 <= c < 127 or c in (9, 10, 13) for c in z):
            return z.decode('latin1')
        return None

    def insns(self, lo, hi):
        """Linear sweep of [lo, hi); skips one byte on undecodable input."""
        code = self.rd(lo, hi - lo)
        off = 0
        while off < len(code):
            last = None
            for ins in self.md.disasm(code[off:], lo + off):
                last = ins
                yield ins
            off = (last.address + last.size - lo) if last else off + 1

    def one(self, va):
        return next(self.md.disasm(self.rd(va, 16), va), None)


def ops_targets(ins):
    """Absolute addresses an instruction refers to (imm or [disp] with no base/index)."""
    out = []
    for op in ins.operands:
        if op.type == X.X86_OP_IMM:
            out.append(op.imm & 0xFFFFFFFF)
        elif op.type == X.X86_OP_MEM and op.mem.base == 0 and op.mem.index == 0:
            out.append(op.mem.disp & 0xFFFFFFFF)
    return out


class XrefIndex:
    """call / jmp / imm-or-disp references between procedures, plus data pointers."""

    def __init__(self, im, procs):
        self.calls = collections.defaultdict(set)   # target -> {source proc}
        self.jmps = collections.defaultdict(set)    # target -> {source proc}
        self.jsite = collections.defaultdict(set)   # target -> {jump instruction address}
        self.imms = collections.defaultdict(set)    # address -> {source proc}
        self.callees = collections.defaultdict(set)  # source proc -> {target}
        self.strrefs = collections.defaultdict(set)  # source proc -> {string}
        self.imprefs = collections.defaultdict(set)  # source proc -> {import}
        self.hoodlum_data = set()                    # procs touching .HOODLUM/.data2 keys
        spans = [(p, n) for p, n in zip(procs, procs[1:] + [CODE_HI]) if CODE_LO <= p < CODE_HI]
        spans += [(HOODLUM[0], HOODLUM[1]), (TEXT_HA[0], TEXT_HA[1])]  # sources only
        for p, end in spans:
            end = min(end, CODE_HI) if p < CODE_HI else end
            for ins in im.insns(p, end):
                if ins.group(capstone.CS_GRP_CALL) or ins.group(capstone.CS_GRP_JUMP):
                    if ins.operands and ins.operands[0].type == X.X86_OP_IMM:
                        t = ins.operands[0].imm & 0xFFFFFFFF
                        if ins.group(capstone.CS_GRP_CALL):
                            self.calls[t].add(p)
                        else:
                            self.jmps[t].add(p)
                            self.jsite[t].add(ins.address)
                        self.callees[p].add(t)
                        continue
                for v in ops_targets(ins):
                    if v in im.iat or v in im.delay_iat:
                        self.imprefs[p].add(im.iat.get(v) or im.delay_iat[v] + '(delay)')
                    elif CODE_LO <= v < CODE_HI:
                        self.imms[v].add(p)
                    elif 0x858000 <= v < 0xC9E000:
                        s = im.cstr(v)
                        if s:
                            self.strrefs[p].add(s)
                    if HOODLUM[0] <= v < HOODLUM[1] or 0x1301000 <= v < 0x1556000:
                        self.hoodlum_data.add(p)
        # aligned code pointers in .rdata/.data/_rwdseg (vtables, callback tables)
        self.dptr = collections.defaultdict(list)
        for s in im.pe.sections:
            va = s.VirtualAddress + BASE
            if s.Name.rstrip(b'\0') in (b'.rdata', b'.data', b'_rwdseg') and va < 0xCB0000:
                b = im.rd(va, s.Misc_VirtualSize)
                for o in range(0, len(b) - 3, 4):
                    v = struct.unpack_from('<I', b, o)[0]
                    if CODE_LO <= v < CODE_HI:
                        self.dptr[v].append(va + o)
        # aligned dwords inside .text (switch jump tables)
        self.tptr = collections.defaultdict(list)
        b = im.rd(CODE_LO, CODE_HI - CODE_LO)
        for o in range(0, len(b) - 3, 4):
            v = struct.unpack_from('<I', b, o)[0]
            if CODE_LO <= v < CODE_HI:
                self.tptr[v].append(CODE_LO + o)

    def refkind(self, a):
        return ''.join(k for k, d in (('C', self.calls), ('J', self.jmps), ('I', self.imms),
                                      ('D', self.dptr)) if a in d)


# --------------------------------------------------------------------------- names
def gr_vtables(gr):
    """{vtable address: (class, n virtuals)} from RH_ScopedVirtualClass in gta-reversed."""
    out = {}
    for f in sorted(glob.glob(os.path.join(gr, 'source', '**', '*.cpp'), recursive=True)):
        for m in re.finditer(r'RH_ScopedVirtualClass\(\s*([\w:]+)\s*,\s*(0x[0-9A-Fa-f]+)\s*,\s*(\d+)',
                             open(f, errors='replace').read()):
            out[int(m.group(2), 16)] = (m.group(1), int(m.group(3)))
    return out


def psdk_names(root):
    """{address: (name, file)} for 1.0 US from plugin-sdk plugin_sa sources."""
    out = {}
    g = os.path.join(root, 'plugin_sa', 'game_sa')
    for f in sorted(glob.glob(g + '/meta/*.h')):
        t = open(f, errors='replace').read()
        for m in re.finditer(r'META_BEGIN(?:_OVERLOADED)?\(([^)]*?)\)(.*?)META_END', t, re.S):
            a = re.search(r'MvAddresses<\s*(0x[0-9A-Fa-f]+)', m.group(2))
            if a and int(a.group(1), 16):
                out.setdefault(int(a.group(1), 16), (m.group(1).split(',')[0].strip(), os.path.relpath(f, root)))
    for f in sorted(glob.glob(g + '/**/*.cpp', recursive=True)):
        t = open(f, errors='replace').read()
        for m in re.finditer(r'addrof(?:_o)?\(([^,)]+)[^=\n]*=\s*ADDRESS_BY_VERSION\((0x[0-9A-Fa-f]+)', t):
            if int(m.group(2), 16):
                out.setdefault(int(m.group(2), 16), (m.group(1).strip(), os.path.relpath(f, root)))
        for m in re.finditer(r'^[A-Za-z_][^\n;{}()]*?\b([A-Za-z_][\w:~]*)\s*\([^;{}]*\)\s*(?:const\s*)?\{\s*\n'
                             r'(.{0,400}?)\n\}', t, re.S | re.M):
            a = (re.search(r'(?:\)\)|Call\w*<[^>]*?)\s*(0x[0-9A-Fa-f]{6})\b', m.group(2))
                 or re.search(r'\)\s*(0x[0-9A-Fa-f]{6})\)\s*\(', m.group(2)))
            if a:
                out.setdefault(int(a.group(1), 16), (m.group(1), os.path.relpath(f, root)))
    return out


def owner_class(name):
    return name.split('::')[0] if '::' in name else ''


def subsystem_for(name):
    o = owner_class(name)
    if o:
        for sid, _cats, rx in SUB.COMPILED:
            if rx.match(o):
                return sid
    if name.startswith(('Rw', 'Rp', 'Rt', 'Rx', '_rw', '_rp')):
        return 'RW'
    return None


# --------------------------------------------------------------------------- main
def main():
    exe, cat = sys.argv[1], Path(sys.argv[2])
    root = cat.resolve().parent
    gr = sys.argv[3] if len(sys.argv) > 3 else str(root / 're' / 'gta-reversed')
    psdk = sys.argv[4] if len(sys.argv) > 4 else str(root / 're' / 'plugin-sdk')

    im = Image(exe)
    rows = [r for r in csv.DictReader(open(cat / 'procedures.csv'))]
    prow = {int(r['address'], 16): r for r in rows}
    procs = sorted(prow)
    hooks = {int(h['address'], 16): h for h in csv.DictReader(open(cat / 'functions.csv'))}
    callee_rows = {int(c['address'], 16): c for c in csv.DictReader(open(cat / 'callees.csv'))}

    def owner(a):
        i = bisect.bisect_right(procs, a) - 1
        return procs[i] if i >= 0 else None

    def nxt(a):
        i = bisect.bisect_right(procs, a)
        return procs[i] if i < len(procs) else CODE_HI

    x = XrefIndex(im, procs)
    # 'excluded' is what subsystems.py writes back for rows classified here, so a rerun on
    # an already processed procedures.csv sees the same input set.
    unknown = [a for a in procs if prow[a]['state'] in ('unknown', 'excluded')
               and prow[a]['region'] in ('game-text', 'rw-cseg')]
    unk = set(unknown)
    known = {a for a in procs if a not in unk and prow[a]['region'] in ('game-text', 'rw-cseg')}
    psn = psdk_names(psdk) if os.path.isdir(psdk) else {}
    vt = gr_vtables(gr) if os.path.isdir(gr) else {}

    def src_of(a):
        if a in hooks:
            return hooks[a]['file']
        if a in callee_rows:
            return callee_rows[a]['first_site']
        return ''

    def is_rw_anchor(a):
        # WindowedMode.cpp hooks RenderWare D3D9-driver functions (named RenderWare::_rwD3D9*)
        return '/RenderWare/' in src_of(a) or 'WindowedMode' in src_of(a)

    out = {}

    def put(a, cls, ev, sub='', name=''):
        if a in unk and a not in out:
            out[a] = (cls, sub, name, ev)

    # ---------------------------------------------------------------- 1. C++ EH
    cfh = [a for a in procs if prow[a]['ghidra_name'] == CXX_FRAME_HANDLER_FID and CODE_LO <= a < CODE_HI]
    assert len(cfh) == 1, 'expected exactly one FID-named ___CxxFrameHandler'
    cfh = cfh[0]
    text = im.rd(CODE_LO, CODE_HI - CODE_LO)
    stubs = {}
    for m in re.finditer(rb'\xB8(....)\xE9(....)', text, re.S):
        a = CODE_LO + m.start()
        if (a + 10 + struct.unpack('<i', m.group(2))[0]) & 0xFFFFFFFF == cfh:
            stubs[a] = struct.unpack('<I', m.group(1))[0]
    actions, catches = {}, {}
    for s, fi in stubs.items():
        magic, max_state, p_unwind, n_try, p_try = struct.unpack('<5I', im.rd(fi, 20))
        assert magic in FUNCINFO_MAGIC, hex(fi)
        for i in range(max_state):
            _to, act = struct.unpack('<iI', im.rd(p_unwind + 8 * i, 8))
            if act:
                actions.setdefault(act, (fi, s, i))
        for i in range(n_try):
            _lo, _hi, _ch, n_catch, p_ha = struct.unpack('<iiiiI', im.rd(p_try + 20 * i, 20))
            for j in range(n_catch):
                h = struct.unpack('<IIiI', im.rd(p_ha + 16 * j, 16))[3]
                catches.setdefault(h, (fi, s, i))
    for a, (fi, s, i) in actions.items():
        put(a, 'excluded:eh-funclet', f'unwind action, state {i} of FuncInfo {fi:#x} '
            f'(ehhandler stub {s:#x} -> ___CxxFrameHandler {cfh:#x})')
    for a, (fi, s, i) in catches.items():
        put(a, 'excluded:eh-funclet', f'catch handler, try block {i} of FuncInfo {fi:#x} '
            f'(ehhandler stub {s:#x})')
    eh_lo = min(actions)

    # ---------------------------------------------------------------- 2. linker thunks
    for a in unknown:
        ins = im.one(a)
        if ins and ins.mnemonic == 'jmp' and ins.operands[0].type == X.X86_OP_MEM:
            d = ins.operands[0].mem.disp & 0xFFFFFFFF
            if d in im.iat:
                put(a, 'excluded:linker-thunk', f'import thunk jmp [{d:#x}] = {im.iat[d]}')
            elif d in im.delay_iat:
                put(a, 'excluded:linker-thunk', f'delay-load import thunk jmp [{d:#x}] = {im.delay_iat[d]}')
        # __imp_load_X: mov eax, offset __imp_X ; jmp __tailMerge_<dll>
        b = im.rd(a, 10)
        if b[0] == 0xB8 and b[5] == 0xE9:
            d = struct.unpack('<I', b[1:5])[0]
            if d in im.delay_iat:
                put(a, 'excluded:linker-thunk', f'delay-load stub (mov eax, __imp_{im.delay_iat[d]}; '
                    f'jmp __tailMerge)')

    # ---------------------------------------------------------------- 3. CRT block
    # The MSVC runtime is linked last: from the delay-load helper (delayhlp.cpp, FID
    # ULI::Unlink) to the first EH funclet. Every procedure in it is runtime code.
    fid = {a: prow[a]['ghidra_name'] for a in procs if CODE_LO <= a < CODE_HI
           if not prow[a]['ghidra_name'].startswith(('FUN_', 'thunk_FUN_', 'Unwind@', 'Catch@', 'switchD',
                                                       '(missed'))
           and '::Ordinal_' not in prow[a]['ghidra_name'] and '.DLL::' not in prow[a]['ghidra_name']
           and not prow[a]['ghidra_name'].startswith('DelayLoad_')}
    crt_lo = min(a for a, n in fid.items() if n == 'ULI::Unlink')
    crt_hi = eh_lo
    bad = [a for a in procs if crt_lo <= a < crt_hi and a in hooks]
    assert not bad, f'gta-reversed hooks inside CRT block: {bad}'
    crt_fid = sum(1 for a in fid if crt_lo <= a < crt_hi)
    for a in unknown:
        if crt_lo <= a < crt_hi:
            if a in fid:
                put(a, 'excluded:crt', f'Ghidra FID (MSVC library hash) match {fid[a]}; in MSVC runtime '
                    f'link block {crt_lo:#x}-{crt_hi:#x}')
            else:
                cs = x.callees.get(a, set())
                inb = sum(1 for t in cs if crt_lo <= t < crt_hi)
                out_t = sorted(t for t in cs if CODE_LO <= t < CODE_HI and not crt_lo <= t < crt_hi)
                ev = (f'in MSVC runtime link block {crt_lo:#x}-{crt_hi:#x} ({crt_fid} FID matches, no game/RW '
                      f'function); calls {inb} block functions')
                if out_t:
                    ev += '; also calls ' + ','.join(f'{t:#x}' for t in out_t[:3])
                put(a, 'excluded:crt', ev)

    # ---------------------------------------------------------------- 4. third-party blocks
    for libname, lo, hi, rx in THIRD_PARTY:
        rxc = re.compile(rx)
        members = [a for a in procs if lo <= a < hi]
        clash = [a for a in members if a in known or a in psn]
        assert not clash, f'{libname}: known functions inside block {[hex(c) for c in clash]}'
        seeds = {a: sorted(s for s in x.strrefs.get(a, ()) if rxc.search(s)) for a in members}
        seeds = {a: v for a, v in seeds.items() if v}
        assert seeds, f'{libname}: no identifying strings'
        # call-graph / vtable components inside the block
        par = {a: a for a in members}

        def f(v):
            while par[v] != v:
                par[v] = par[par[v]]
                v = par[v]
            return v
        mset = set(members)
        for d in (x.calls, x.jmps, x.imms):
            for t, srcs in d.items():
                to = owner(t)
                if to in mset:
                    for s_ in srcs:
                        if s_ in mset:
                            par[f(s_)] = f(to)
        # code pointers stored next to each other (vtables) join their targets
        runs = sorted((loc, t) for t in members for loc in x.dptr.get(t, ()))
        for (l1, t1), (l2, t2) in zip(runs, runs[1:]):
            if l2 - l1 == 4:
                par[f(t1)] = f(t2)
        seeded = {f(a) for a in seeds}
        zl = {a for a, v in seeds.items() if any(ZLIB_RE.search(s) for s in v)}
        for a in members:
            if a not in unk:
                continue
            if a in seeds:
                tag = 'zlib' if a in zl else libname.split('+')[0].split('-')[0]
                ev = f'{tag} strings: ' + ' | '.join(repr(s[:40]) for s in seeds[a][:3])
            elif f(a) in seeded:
                ev = 'call-graph/vtable component linked to identified functions'
            else:
                pa, na = max([m for m in members if m < a and m in seeds] or [lo]), \
                    min([m for m in members if m > a and m in seeds] or [hi])
                ev = (f'link order only: inside contiguous block between identified functions '
                      f'{pa:#x} and {na:#x}; refs={x.refkind(a) or "none"}')
            put(a, f'excluded:thirdparty:{libname}', f'{ev}; block {lo:#x}-{hi:#x}')

    # ---------------------------------------------------------------- 5. fragments / mis-splits
    # A "host" is the gta-reversed-known function whose address span [entry, next known
    # entry) contains an address. SecuROM replaced instructions inside host functions with
    # `jmp fragment`; the fragment runs the displaced instructions (often obfuscated with
    # pushfd/popfd and key reads from the protection .data) and jumps back into the host.
    known_sorted = sorted(known)

    def kowner(v):
        i = bisect.bisect_right(known_sorted, v) - 1
        return known_sorted[i] if i >= 0 else None

    frag_zone = (procs[0], min(a for a in procs if a in hooks))  # SecuROM fragment area at .text start

    def in_zone(v):
        return frag_zone[0] <= v < frag_zone[1]

    def frag_walk(entry, hosts, limit=96):
        """Follow straight-line code (and unconditional jmps) from entry.
        Returns (kind, address, hoodlum_data); kind 'back' = jmp into a host span."""
        pc, hood, seen = entry, False, set()
        for _ in range(limit):
            if pc in seen:
                return 'loop', pc, hood
            seen.add(pc)
            ins = im.one(pc)
            if ins is None:
                return 'bad', pc, hood
            for v in ops_targets(ins):
                if HOODLUM[0] <= v < HOODLUM[1] or 0x1301000 <= v < 0x1556000:
                    hood = True
            if ins.mnemonic in ('ret', 'retn', 'iret'):
                return 'ret', pc, hood
            if ins.mnemonic == 'jmp':
                op = ins.operands[0]
                if op.type != X.X86_OP_IMM:
                    return 'indirect', pc, hood
                t = op.imm & 0xFFFFFFFF
                if not (CODE_LO <= t < CODE_HI):
                    return 'out', t, hood
                if kowner(t) in hosts and t not in known and not in_zone(t):
                    return 'back', t, hood
                if t in known:
                    return 'tail', t, hood
                pc = t
                continue
            pc = ins.address + ins.size
        return 'long', pc, hood

    for a in unknown:
        if a in out:
            continue
        gname = prow[a]['ghidra_name']
        if gname.startswith('switchD_'):
            sw = int(re.match(r'switchD_([0-9a-f]+)', gname).group(1), 16)
            tbl = x.tptr.get(a, [])
            put(a, 'excluded:mis-split', f'Ghidra switch-case label of the switch at {sw:#x} in {kowner(sw):#x}'
                f'; jump-table slots {",".join(hex(t) for t in tbl[:2]) or "none found"}')
            continue
        kind = x.refkind(a)
        if not (kind == 'J' or in_zone(a) and kind == ''):
            continue
        sites = x.jsite.get(a, set())
        hosts = {kowner(j) for j in sites if not in_zone(j)}
        how, t, hood = frag_walk(a, hosts)
        src = ','.join(hex(j) for j in sorted(sites)[:3]) or 'nothing'
        hz = '; reads protection key data (.HOODLUM/.data2)' if hood else ''
        if in_zone(a):
            put(a, 'excluded:protection', f'SecuROM fragment area {frag_zone[0]:#x}-{frag_zone[1]:#x} (before the '
                f'first game function), only jumped to (from {src}); walk ends with {how} at {t:#x}{hz}')
        elif how == 'back':
            if kowner(a) in hosts:
                put(a, 'excluded:mis-split', f'piece of {kowner(a):#x}: lies in its span, only jumped to from it '
                    f'({src}) and jumps back into it at {t:#x}{hz}')
            else:
                put(a, 'excluded:protection', f'SecuROM displaced-code fragment in the span of {kowner(a):#x}: '
                    f'entered by jmp from {src} (host {",".join(hex(h) for h in sorted(hosts))}), jumps back '
                    f'into the host at {t:#x}{hz}')
        elif how == 'ret' and len(hosts) == 1 and kowner(a) in hosts and a % 16:
            put(a, 'excluded:mis-split', f'tail of {kowner(a):#x}: lies in its span, unaligned, only jumped '
                f'to from it ({src}), ends in ret{hz}')
        elif how == 'ret' and len(hosts) == 1 and a % 16 and (hood or a - (kowner(a) or 0) > 0):
            if hood or (prow.get(kowner(a)) or {}).get('thunk') == 'yes' or \
                    (kowner(a) in hooks and hooks[kowner(a)]['status'] == 'stub-to-.HOODLUM'):
                put(a, 'excluded:protection', f'SecuROM displaced epilogue in the dead span of '
                    f'{kowner(a):#x} (a .HOODLUM stub): unaligned, only jumped to from {src}, ends in ret{hz}')

    # The rest of the fragment area: SecuROM's own runtime (pushal/popal wrappers, calls
    # through pointers in _TEXT_HA, stubs into .HOODLUM) referenced only from inside the area.
    for a in unknown:
        if a in out or not in_zone(a):
            continue
        refs = x.calls.get(a, set()) | x.jmps.get(a, set()) | x.imms.get(a, set())
        if all(in_zone(r) for r in refs) and a not in x.dptr:
            put(a, 'excluded:protection', f'SecuROM runtime in fragment area {frag_zone[0]:#x}-{frag_zone[1]:#x}: '
                f'referenced only from inside the area ({",".join(hex(r) for r in sorted(refs)[:3]) or "no refs"})')

    # Re-entry points: Ghidra starts a procedure at every target of a fragment's jmp back.
    # Such an address is inside its host's body (not after padding), and nothing but
    # protection code (or the host itself) jumps to it.
    def protection_src(j):
        if in_zone(j) or HOODLUM[0] <= j < HOODLUM[1]:
            return True
        o = owner(j)
        return o in out and out[o][0] == 'excluded:protection'

    for a in unknown:
        if a in out or x.refkind(a) != 'J' or in_zone(a):
            continue
        sites = x.jsite.get(a, set())
        host = kowner(a)
        prev = im.rd(a - 1, 1)[0]
        mid_body = prev not in (0xCC, 0x90, 0xC3) or a % 16
        if host and mid_body and sites and all(protection_src(j) or kowner(j) == host for j in sites) \
                and any(protection_src(j) for j in sites):
            put(a, 'excluded:mis-split', f'SecuROM re-entry point inside {host:#x} '
                f'({prow[host]["gr_name"] or "known function"}): mid-body (previous byte {prev:#04x}), only '
                f'jumped to from protection code ({",".join(hex(j) for j in sorted(sites)[:3])})')

    # ---------------------------------------------------------------- 6. RenderWare
    # The RenderWare libraries are linked as one span that ends at the runtime block; it
    # starts at the first RenderWare thunk after the last non-RenderWare game function.
    last_game = max(a for a in set(hooks) | set(callee_rows) if a < crt_lo and not is_rw_anchor(a))
    rw_lo = min(a for a in set(hooks) | set(callee_rows) if a > last_game and is_rw_anchor(a))
    rw_hi = crt_lo
    tp = [(lo, hi) for _n, lo, hi, _r in THIRD_PARTY]
    rw_members = [a for a in procs if rw_lo <= a < rw_hi and not any(lo <= a < hi for lo, hi in tp)
                  and not (a in out and out[a][0].startswith('excluded:linker'))]
    rws = set(rw_members)
    par = {a: a for a in rw_members}

    def g(v):
        while par[v] != v:
            par[v] = par[par[v]]
            v = par[v]
        return v
    for d in (x.calls, x.jmps, x.imms):
        for t, srcs in d.items():
            to = owner(t)
            if to in rws:
                for s_ in srcs:
                    if s_ in rws:
                        par[g(s_)] = g(to)
    anchored = {g(a) for a in rw_members if a in known}
    nonrw_known = [a for a in rw_members if a in known and not is_rw_anchor(a)]
    assert not nonrw_known, f'non-RW gta-reversed functions inside RW span: {[hex(a) for a in nonrw_known]}'
    for a in rw_members:
        if a not in unk or a in out:
            continue
        nm = psn.get(a, ('', ''))[0]
        if nm and subsystem_for(nm) == 'RW':
            ev = f'plugin-sdk name {nm} ({psn[a][1]})'
        elif g(a) in anchored:
            ev = 'call-graph component contains gta-reversed RenderWare functions'
        else:
            pa = max(m for m in rw_members if m < a and m in known) if any(
                m < a and m in known for m in rw_members) else rw_lo
            na = min([m for m in rw_members if m > a and m in known] or [rw_hi])
            ev = (f'link order only: between RenderWare functions {pa:#x} and {na:#x}; '
                  f'refs={x.refkind(a) or "none"}')
        put(a, 'RW', f'{ev}; RenderWare span {rw_lo:#x}-{rw_hi:#x}', 'RW', nm)
    for a in unknown:
        if RWCSEG[0] <= a < RWCSEG[1]:
            put(a, 'RW', 'in _rwcseg (RenderWare code segment); uses _rwdseg data', 'RW')

    # ---------------------------------------------------------------- 7. game code
    vt_slot = {}
    for v, (cls, n) in vt.items():
        for i in range(n):
            vt_slot[im.u32(v + 4 * i)] = (cls, i, v)
    vts = VTables(exe, gr) if os.path.isdir(gr) else None
    slot_names = vts.slot_names() if vts else {}

    def proc_sub(p):
        """Subsystem of a known procedure (hook or classified)."""
        if p in hooks:
            h = hooks[p]
            return SUB.classify(h['category'], h['owner'], h['name'])
        if p in out and out[p][1]:
            return out[p][1]
        return None

    pending = []
    for a in unknown:
        if a in out:
            continue
        r = prow[a]
        ev = []
        if r['thunk'] == 'yes':
            ins = im.one(a)
            t = ins.operands[0].imm & 0xFFFFFFFF if ins and ins.mnemonic == 'jmp' and \
                ins.operands[0].type == X.X86_OP_IMM else None
            if t and HOODLUM[0] <= t < HOODLUM[1]:
                ev.append(f'game-.text stub to relocated .HOODLUM body {t:#x}')
            elif t:
                ev.append(f'jmp thunk to {t:#x}')
        refs = x.refkind(a)
        if not refs and r['thunk'] != 'yes':
            ev.append('no static reference found (dead code or reached through a computed pointer)')
        nm, sub = '', None
        if a in psn:
            nm = psn[a][0]
            sub = subsystem_for(nm)
            ev.insert(0, f'plugin-sdk name {nm} ({psn[a][1]})')
        if a in slot_names:
            vnm, vev = implementation_name(vts, a, slot_names[a])
            nm = nm or vnm
            sub = sub or subsystem_for(vnm)
            ev.insert(0, vev)
        elif a in vt_slot:
            cls, i, v = vt_slot[a]
            nm = nm or f'{cls}::vfunc_{i}'
            sub = sub or subsystem_for(f'{cls}::x')
            ev.insert(0, f'slot {i} of {cls} vtable {v:#x} (gta-reversed RH_ScopedVirtualClass)')
        if fid.get(a) and not nm:
            ev.append(f'Ghidra FID name {fid[a]} (outside the runtime block: not trusted)')
        if refs:
            ev.append(f'refs={refs}')
        pending.append((a, nm, sub, ev))

    # caller-based subsystem (unanimous callers), iterated so helpers of helpers resolve
    subs = {a: s for a, _n, s, _e in pending if s}
    for _ in range(4):
        changed = False
        for a, _nm, s, _ev in pending:
            if a in subs:
                continue
            cs = x.calls.get(a, set()) | x.jmps.get(a, set())
            ss = {proc_sub(c) or subs.get(c) for c in cs}
            if cs and len(ss) == 1 and None not in ss and 'RW' not in ss:
                subs[a] = ss.pop()
                changed = True
        if not changed:
            break
    for a, nm, s, ev in pending:
        sub = subs.get(a)
        if sub and not s:
            n = len(x.calls.get(a, set()) | x.jmps.get(a, set()))
            ev.append(f'subsystem from its {n} caller(s), all {sub}')
        put(a, 'game', '; '.join(ev) or 'real code', sub or 'UNK-game', nm)

    for a in unknown:
        put(a, 'unknown', 'no rule applied')

    with open(cat / 'classification.csv', 'w', newline='') as fh:
        w = csv.writer(fh)
        w.writerow(['address', 'class', 'subsystem', 'name', 'evidence'])
        for a in unknown:
            cls, sub, nm, ev = out[a]
            w.writerow([f'{a:#08x}', cls, sub, nm, ev])
    cnt = collections.Counter(out[a][0] for a in unknown)
    for k, v in cnt.most_common():
        print(f'{v:6d}  {k}')
    print(f'{len(unknown):6d}  total; CRT block {crt_lo:#x}-{crt_hi:#x}, RW span {rw_lo:#x}-{rw_hi:#x}, '
          f'fragment area {frag_zone[0]:#x}-{frag_zone[1]:#x}, {len(stubs)} EH stubs')


if __name__ == '__main__':
    main()
