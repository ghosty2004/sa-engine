#!/usr/bin/env python3
"""CAT-7: call graph, global read/write sets, function extents and leaf-first port order.

Two steps:

  callgraph.py ghidra [catalog-dir]
      Writes catalog/raw/ghidra/{remove,create}.txt and runs the Ghidra headless dump
      (tools/catalog/ghidra/DumpCat7.java, -readOnly) on the analysed project in
      target/ghidra (created with `callgraph.py analyse` on first use). Output:
      catalog/raw/ghidra/functions.jsonl (function bodies + Ghidra flow references).

  callgraph.py build [catalog-dir]
      Reads the exe, the catalogue, gta-reversed, plugin-sdk and the Ghidra dump, and writes
      catalog/extents.csv, callgraph.csv, rwsets.csv, indirect-calls.csv, port-order.csv
      and catalog/raw/cat7-stats.json. Deterministic: no timestamps, sorted output.

  callgraph.py spotcheck N SEED
      Verification: N random reversed functions vs their gta-reversed source.

  callgraph.py analyse
      One-time import + auto-analysis of the exe into target/ghidra (about 20 minutes).

Method and blind spots: docs/re/cat7-callgraph.md. Run with .venv/bin/python -I.
"""
import bisect
import collections
import csv
import glob
import json
import os
import re
import struct
import subprocess
import sys
from pathlib import Path

import capstone
import pefile
from capstone import x86_const as X

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / 're' / 'gta' / '1.0-US' / 'gta_sa.exe'
GR = ROOT / 're' / 'gta-reversed'
PSDK = ROOT / 're' / 'plugin-sdk'
GHIDRA = Path(os.environ.get('GHIDRA_HOME', '/opt/ghidra'))
PROJ_DIR = ROOT / 'target' / 'ghidra'
PROJ_NAME = 'gta_sa_10us'
SCRIPT_DIR = Path(__file__).resolve().parent / 'ghidra'

BASE = 0x400000
GAME_TEXT = (0x401000, 0x857000)
RWCSEG = (0x857000, 0x858000)
HOODLUM = (0x1556000, 0x1577000)
CODE = [(0x401000, 0x858000), (0xCB1000, 0x1301000), HOODLUM]
# data sections an absolute operand can name (section label, lo, hi)
DATA = [('.rdata', 0x858000, 0x8A4000), ('.data', 0x8A4000, 0xC9E000), ('_TEXT_HA', 0xC9E000, 0xCAF000),
        ('_rwdseg', 0xCAF000, 0xCB0000), ('.rsrc', 0xCB0000, 0xCB1000), ('.data2', 0x1301000, 0x1556000)]
LIVE_STATES = ('gr-reversed', 'gr-unreversed', 'gr-called-only', 'unknown')
# edges that make a function depend on another for porting (leaf-first order)
ORDER_KINDS = ('call', 'tailcall', 'virtual', 'virtual-override', 'indirect-ghidra', 'eh-unwind')
REMOVE_CLASSES = ('excluded:mis-split', 'excluded:protection')
CXX_FRAME_HANDLER = 0x8215A0
FUNCINFO_MAGIC = (0x19930520, 0x19930521, 0x19930522)

R32 = {X.X86_REG_EAX: 'eax', X.X86_REG_ECX: 'ecx', X.X86_REG_EDX: 'edx', X.X86_REG_EBX: 'ebx',
       X.X86_REG_ESP: 'esp', X.X86_REG_EBP: 'ebp', X.X86_REG_ESI: 'esi', X.X86_REG_EDI: 'edi'}
# sub-registers -> the 32-bit register they alias (a write kills the full register's token)
ALIAS = {}
for full, subs in ((X.X86_REG_EAX, (X.X86_REG_AX, X.X86_REG_AL, X.X86_REG_AH)),
                   (X.X86_REG_ECX, (X.X86_REG_CX, X.X86_REG_CL, X.X86_REG_CH)),
                   (X.X86_REG_EDX, (X.X86_REG_DX, X.X86_REG_DL, X.X86_REG_DH)),
                   (X.X86_REG_EBX, (X.X86_REG_BX, X.X86_REG_BL, X.X86_REG_BH)),
                   (X.X86_REG_ESI, (X.X86_REG_SI,)), (X.X86_REG_EDI, (X.X86_REG_DI,)),
                   (X.X86_REG_EBP, (X.X86_REG_BP,)), (X.X86_REG_ESP, (X.X86_REG_SP,))):
    ALIAS[full] = full
    for s in subs:
        ALIAS[s] = full
CALLER_SAVED = (X.X86_REG_EAX, X.X86_REG_ECX, X.X86_REG_EDX)


def hx(v):
    return f'{v:#x}'


def section_of(a):
    for name, lo, hi in DATA:
        if lo <= a < hi:
            return name
    return None


def in_code(a):
    return any(lo <= a < hi for lo, hi in CODE)


# ============================================================================ inputs
class Image:
    def __init__(self, exe):
        self.pe = pefile.PE(str(exe))
        self.img = self.pe.get_memory_mapped_image()
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.md.detail = True
        self.iat = {}
        for e in getattr(self.pe, 'DIRECTORY_ENTRY_IMPORT', []):
            for i in e.imports:
                self.iat[i.address] = e.dll.decode() + '!' + (i.name.decode() if i.name else f'#{i.ordinal}')
        for e in getattr(self.pe, 'DIRECTORY_ENTRY_DELAY_IMPORT', []):
            for i in e.imports:
                self.iat[i.address] = e.dll.decode() + '!' + (i.name.decode() if i.name else f'#{i.ordinal}')

    def rd(self, va, n):
        return self.img[va - BASE:va - BASE + n]

    def u32(self, va):
        return struct.unpack('<I', self.rd(va, 4))[0]

    def insns(self, lo, hi):
        """Linear sweep of [lo, hi). Ghidra body ranges start at instruction boundaries and
        hold only instructions, so this decodes exactly the body. Undecodable bytes: skip 1."""
        code = self.rd(lo, hi - lo)
        off = 0
        while off < len(code):
            last = None
            for ins in self.md.disasm(code[off:], lo + off):
                last = ins
                yield ins
            off = (last.address + last.size - lo) if last else off + 1


def load_procs(cat):
    rows = list(csv.DictReader(open(cat / 'procedures.csv')))
    cls = {int(r['address'], 16): r for r in csv.DictReader(open(cat / 'classification.csv'))}
    out = {}
    for r in rows:
        a = int(r['address'], 16)
        name = r['gr_name'] or (cls[a]['name'] if a in cls else '')
        out[a] = dict(state=r['state'], cls=r['class'] or ('game' if r['state'] != 'excluded' else ''),
                      sub=r['subsystem'], name=name, ghidra=r['ghidra_name'])
    return out


def hoodlum_stubs(im, procs):
    """{stub address: .HOODLUM body address} for 5-byte `jmp rel32` stubs in game .text."""
    out = {}
    for a in procs:
        if GAME_TEXT[0] <= a < GAME_TEXT[1]:
            b = im.rd(a, 5)
            if b[0] == 0xE9:
                t = (a + 5 + struct.unpack('<i', b[1:5])[0]) & 0xFFFFFFFF
                if HOODLUM[0] <= t < HOODLUM[1]:
                    out[a] = t
    return out


# ---------------------------------------------------------------------------- names
def _lhs_name(lhs):
    s = lhs.rstrip()
    while True:
        s2 = re.sub(r'\[[^\[\]]*\]\s*$', '', s).rstrip()
        s2 = re.sub(r'\)\s*$', '', s2).rstrip()
        if s2 == s:
            break
        s = s2
    m = re.search(r'([A-Za-z_][\w:]*)\s*$', s)
    return m.group(1) if m else None


DECL_RE = re.compile(r'^(?P<lhs>[^=;/"]*?[\w)\]])\s*=\s*(?P<rhs>[^;]*?)(?P<addr>0x[0-9A-Fa-f]{6,7})\b(?P<tail>[^;]*);')


def global_names(gr, psdk, data_ok):
    """{address: (name, source)} for data globals named by gta-reversed or plugin-sdk."""
    gr_q, gr_bare, ps = {}, {}, {}

    def scan(files, root, sink_q, sink_bare):
        for f in files:
            cur_class = None
            for line in open(f, errors='replace'):
                m0 = re.match(r'^(?:class|struct)\s+(?:\w+\s+)*?(\w+)\s*(?:final\s*)?(?::|\{|$)', line)
                if m0:
                    cur_class = m0.group(1)
                st = line.strip()
                if st.startswith('//') or '0x' not in st:
                    continue
                m = DECL_RE.match(st)
                if not m:
                    continue
                rhs = m.group('rhs')
                if not re.search(r'\*\s*\)|StaticRef|reinterpret_cast|ADDRESS_BY_VERSION', rhs):
                    continue
                if re.search(r'\bCall\w*\b|ctor_|dtor_|_addr\b|addrof', st):
                    continue
                a = int(m.group('addr'), 16)
                if not data_ok(a):
                    continue
                n = _lhs_name(m.group('lhs'))
                if not n or n in ('auto', 'return'):
                    continue
                rel = os.path.relpath(f, root)
                if '::' in n:
                    sink_q.setdefault(a, (n, rel))
                elif line[:1] in (' ', '\t') and cur_class and sink_bare is not None:
                    sink_q.setdefault(a, (f'{cur_class}::{n}', rel))
                else:
                    (sink_bare if sink_bare is not None else sink_q).setdefault(a, (n, rel))

    if gr.is_dir():
        scan(sorted(glob.glob(str(gr / 'source' / '**' / '*.[ch]*'), recursive=True)), gr, gr_q, gr_bare)
    if psdk.is_dir():
        scan(sorted(glob.glob(str(psdk / 'plugin_sa' / 'game_sa' / '**' / '*.[ch]*'), recursive=True)),
             psdk, ps, None)
    out = {}
    for a, (n, f) in gr_bare.items():
        out[a] = (n, 'gta-reversed:' + f)
    for a, (n, f) in ps.items():
        out[a] = (n, 'plugin-sdk:' + f)
    for a, (n, f) in gr_q.items():
        out[a] = (n, 'gta-reversed:' + f)
    return out


def gr_vtables(gr):
    """{class: (vtable address, n virtuals)} from RH_ScopedVirtualClass."""
    out = {}
    for f in sorted(glob.glob(str(gr / 'source' / '**' / '*.cpp'), recursive=True)):
        for m in re.finditer(r'RH_ScopedVirtualClass\(\s*([\w:]+)\s*,\s*(0x[0-9A-Fa-f]+)\s*,\s*(\d+)',
                             open(f, errors='replace').read()):
            out.setdefault(m.group(1), (int(m.group(2), 16), int(m.group(3))))
    return out


def gr_bases(gr):
    """{class: primary base class} from gta-reversed class declarations."""
    out = {}
    rx = re.compile(r'^\s*(?:class|struct)\s+(?:NOTSA_\w+\s+|alignas\([^)]*\)\s+)*(\w+)\s*(?:final\s*)?:\s*'
                    r'(?:public|protected|private)?\s*(?:virtual\s+)?([\w:]+)', re.M)
    for f in sorted(glob.glob(str(gr / 'source' / '**' / '*.h'), recursive=True)):
        for m in rx.finditer(open(f, errors='replace').read()):
            out.setdefault(m.group(1), m.group(2).split('::')[-1])
    return out


# ============================================================================ ghidra step
def run(cmd, log):
    print('+', ' '.join(map(str, cmd)), file=sys.stderr)
    with open(log, 'w') as lf:
        r = subprocess.run([str(c) for c in cmd], stdout=lf, stderr=subprocess.STDOUT,
                           env={**os.environ, 'GHIDRA_HEADLESS_MAXMEM': os.environ.get('GHIDRA_HEADLESS_MAXMEM', '6G')})
    if r.returncode:
        sys.exit(f'command failed ({r.returncode}), see {log}')


def cmd_analyse():
    PROJ_DIR.mkdir(parents=True, exist_ok=True)
    run([GHIDRA / 'support' / 'analyzeHeadless', PROJ_DIR, PROJ_NAME, '-import', EXE, '-overwrite',
         '-max-cpu', '8'], PROJ_DIR / 'analyze.log')


def cmd_ghidra(cat):
    if not (PROJ_DIR / f'{PROJ_NAME}.gpr').exists():
        cmd_analyse()
    im = Image(EXE)
    procs = load_procs(cat)
    raw = cat / 'raw' / 'ghidra'
    raw.mkdir(parents=True, exist_ok=True)
    remove = sorted(a for a, p in procs.items() if p['cls'] in REMOVE_CLASSES)
    stubs = hoodlum_stubs(im, procs)
    create = sorted(set(a for a in procs if a not in set(remove)) | set(stubs.values()))
    (raw / 'remove.txt').write_text(''.join(f'{a:#x}\n' for a in remove))
    (raw / 'create.txt').write_text(''.join(f'{a:#x}\n' for a in create))
    out = raw / 'functions.jsonl'
    run([GHIDRA / 'support' / 'analyzeHeadless', PROJ_DIR, PROJ_NAME, '-process', EXE.name, '-readOnly',
         '-noanalysis', '-scriptPath', SCRIPT_DIR, '-postScript', 'DumpCat7.java',
         raw / 'remove.txt', raw / 'create.txt', out], raw / 'dump.log')
    summary = [l for l in open(raw / 'dump.log') if 'CAT7' in l]
    print(''.join(summary), end='')


# ============================================================================ build step
class Func:
    __slots__ = ('addr', 'ranges', 'source', 'hood', 'thunk')

    def __init__(self, addr):
        self.addr, self.ranges, self.source, self.hood, self.thunk = addr, [], '', None, None


def load_dump(path):
    d = {}
    for line in open(path):
        j = json.loads(line)
        a = int(j['entry'], 16)
        j['body'] = [(int(lo, 16), int(hi, 16) + 1) for lo, hi in j['body']]
        j['calls'] = [(int(s, 16), int(t, 16), k) for s, t, k in j['calls']]
        j['jumps'] = [(int(s, 16), int(t, 16), k) for s, t, k in j['jumps']]
        j['unres'] = [(int(s, 16), k) for s, k in j['unres']]
        d[a] = j
    return d


def eh_funclets(im, procs):
    """{ehhandler stub: [unwind/catch funclet addresses]} from MSVC FuncInfo tables."""
    text = im.rd(GAME_TEXT[0], GAME_TEXT[1] - GAME_TEXT[0])
    out = {}
    for m in re.finditer(rb'\xB8(....)\xE9(....)', text, re.S):
        a = GAME_TEXT[0] + m.start()
        if (a + 10 + struct.unpack('<i', m.group(2))[0]) & 0xFFFFFFFF != CXX_FRAME_HANDLER:
            continue
        fi = struct.unpack('<I', m.group(1))[0]
        magic, max_state, p_unwind, n_try, p_try = struct.unpack('<5I', im.rd(fi, 20))
        if magic not in FUNCINFO_MAGIC:
            continue
        fl = set()
        for i in range(max_state):
            act = struct.unpack('<iI', im.rd(p_unwind + 8 * i, 8))[1]
            if act:
                fl.add(act)
        for i in range(n_try):
            n_catch, p_ha = struct.unpack('<iI', im.rd(p_try + 20 * i + 12, 8))
            for j in range(n_catch):
                fl.add(struct.unpack('<IIiI', im.rd(p_ha + 16 * j, 16))[3])
        out[a] = sorted(fl)
    return out


def mem_is_abs(op):
    return op.type == X.X86_OP_MEM and op.mem.base == 0 and op.mem.segment in (0, X.X86_REG_DS)


def build(cat):
    im = Image(EXE)
    procs = load_procs(cat)
    dump = load_dump(cat / 'raw' / 'ghidra' / 'functions.jsonl')
    stubs = hoodlum_stubs(im, procs)
    hood2stub = {}
    for s, t in sorted(stubs.items()):
        hood2stub.setdefault(t, s)
    entries = sorted(set(procs) | set(dump))
    live = sorted(a for a, p in procs.items() if p['state'] in LIVE_STATES)
    liveset = set(live)

    def canon(t):
        """Catalogue address for a branch target: .HOODLUM bodies map to their game stub."""
        return hood2stub.get(t, t)

    def cls_of(t):
        if t in procs:
            return procs[t]['cls']
        if HOODLUM[0] <= t < HOODLUM[1]:
            return 'noncatalogue:hoodlum'
        if 0xCB1000 <= t < 0x1301000:
            return 'noncatalogue:protection-text'
        return 'noncatalogue'

    def name_of(t):
        if t in procs:
            return procs[t]['name'] or procs[t]['ghidra']
        if t in dump:
            return dump[t]['name']
        return ''

    gnames = global_names(GR, PSDK, lambda a: section_of(a) is not None)
    gsorted = sorted(gnames)
    vt = gr_vtables(GR)
    bases = gr_bases(GR)
    derived = collections.defaultdict(set)
    for c in vt:
        b, seen = bases.get(c), set()
        while b and b not in seen:
            seen.add(b)
            derived[b].add(c)
            b = bases.get(b)
    vtaddr = {a: c for c, (a, _n) in vt.items()}

    def slot_impl(c, slot):
        a, n = vt[c]
        if slot >= n:
            return None
        v = im.u32(a + 4 * slot)
        return v if in_code(v) else None

    ehstubs = eh_funclets(im, procs)

    # ---- function extents
    entry_set = set(procs) | set(dump)
    funcs = {}
    stats = collections.Counter()
    for a in live:
        f = Func(a)
        if a in dump:
            f.ranges = list(dump[a]['body'])
            f.source = 'ghidra-created' if dump[a]['created'] else 'ghidra'
            if dump[a]['thunk']:
                f.thunk = int(dump[a]['thunk'], 16)
        else:
            nxt = entries[bisect.bisect_right(entries, a)] if a < entries[-1] else a + 1
            f.ranges = [(a, nxt)]
            f.source = 'fallback-span'
        if a in stubs and stubs[a] in dump:
            f.hood = stubs[a]
            f.ranges = f.ranges + list(dump[stubs[a]]['body'])
        f.ranges = sorted(set(f.ranges))
        extra = complete_body(im, f.ranges, entry_set, {a, f.hood} - {None})
        if extra:
            f.ranges = merge_ranges(f.ranges + extra)
            f.source += '+completed'
            stats['completed_functions'] += 1
            stats['completed_bytes'] += sum(hi - lo for lo, hi in extra)
        funcs[a] = f
        stats['extent:' + f.source] += 1

    # ---- per-function analysis
    edges = collections.defaultdict(int)      # (caller, callee, kind) -> sites
    ecls, ename = {}, {}
    rw = collections.defaultdict(set)         # (func, addr) -> {(size, access, mode)}
    indirect = []                             # (func, site, form, resolution, detail)
    summ = {}

    def edge(c, t, kind, cl, nm):
        edges[(c, t, kind)] += 1
        ecls[(c, t, kind)] = cl
        ename[(c, t, kind)] = nm

    for a in live:
        f = funcs[a]
        own = procs[a]['name']
        owner = own.split('::')[0] if '::' in own else None
        gh = [dump[x] for x in (a, f.hood) if x is not None and x in dump]
        gcalls = collections.defaultdict(set)
        for j in gh:
            for s, t, _k in j['calls']:
                gcalls[s].add(t)
        gjumps = {s for j in gh for s, _t, _k in j['jumps']}
        gunres = {s: k for j in gh for s, k in j['unres']}
        inbody = lambda t: any(lo <= t < hi for lo, hi in f.ranges)  # noqa: E731
        ins_all = [ins for lo, hi in f.ranges for ins in im.insns(lo, hi)]
        targets = set()
        for ins in ins_all:
            if (ins.group(capstone.CS_GRP_JUMP) and ins.operands and ins.operands[0].type == X.X86_OP_IMM):
                targets.add(ins.operands[0].imm & 0xFFFFFFFF)
        for j in gh:
            targets.update(t for _s, t, _k in j['jumps'])
        frames = {lo for lo in (a, f.hood) if lo is not None and im.rd(lo, 3) == b'\x55\x8b\xec'}
        ebp_frame = bool(frames)
        n_direct = n_ind = n_ind_res = n_cjmp_unres = wtp = n_jout = 0
        st_in = this_dataflow(ins_all, {a, f.hood} - {None}, gh)
        for ins in ins_all:
            state = st_in.get(ins.address) or {}
            is_call = ins.group(capstone.CS_GRP_CALL)
            is_jmp = ins.group(capstone.CS_GRP_JUMP)
            op0 = ins.operands[0] if ins.operands else None

            if (is_call or is_jmp) and op0 is not None and op0.type == X.X86_OP_IMM:
                t = op0.imm & 0xFFFFFFFF
                if is_call:
                    n_direct += 1
                    c = canon(t)
                    edge(a, c, 'call', cls_of(c), name_of(c))
                elif not inbody(t):
                    if t in procs or t in dump:
                        c = canon(t)
                        edge(a, c, 'tailcall', cls_of(c), name_of(c))
                    else:
                        n_jout += 1
                        indirect.append((a, ins.address, f'{ins.mnemonic} {t:#x}', 'jump-outside-body', ''))
            elif (is_call or is_jmp) and op0 is not None:
                # indirect call / jump
                form = ins.op_str
                if op0.type == X.X86_OP_MEM and mem_is_abs(op0) and op0.mem.index == 0 \
                        and (op0.mem.disp & 0xFFFFFFFF) in im.iat:
                    d = op0.mem.disp & 0xFFFFFFFF
                    edge(a, im.iat[d], 'import', 'import', im.iat[d])
                    if is_call:
                        n_direct += 1
                elif is_jmp and (ins.address in gjumps):
                    pass  # switch / computed jump resolved by Ghidra (targets are in the body)
                elif is_jmp:
                    n_cjmp_unres += 1
                    indirect.append((a, ins.address, f'jmp {form}', 'unresolved-computed-jump', ''))
                else:
                    n_ind += 1
                    res = None
                    # vtable slot of `this`
                    if op0.type == X.X86_OP_MEM and op0.mem.index == 0 and op0.mem.base:
                        tok = state.get(ALIAS.get(op0.mem.base))
                        if tok == ('VT',) and op0.mem.disp % 4 == 0:
                            res = ('slot', op0.mem.disp // 4)
                    elif op0.type == X.X86_OP_REG:
                        tok = state.get(ALIAS.get(op0.reg))
                        if tok and tok[0] == 'SLOT':
                            res = ('slot', tok[1])
                    if res and owner in vt:
                        slot = res[1]
                        impl = slot_impl(owner, slot)
                        detail = [f'{owner} vtable {vt[owner][0]:#x} slot {slot}']
                        if impl is not None:
                            c = canon(impl)
                            edge(a, c, 'virtual', cls_of(c), name_of(c))
                        for dc in sorted(derived.get(owner, ())):
                            di = slot_impl(dc, slot)
                            if di is not None and di != impl:
                                c = canon(di)
                                edge(a, c, 'virtual-override', cls_of(c), f'{dc}: {name_of(c)}')
                        n_ind_res += 1
                        indirect.append((a, ins.address, f'call {form}', 'this-vtable',
                                         '; '.join(detail) + (f' -> {impl:#x}' if impl is not None else
                                                              ' -> slot outside known vtable')))
                    elif ins.address in gcalls and all(in_code(t) for t in gcalls[ins.address]):
                        for t in sorted(gcalls[ins.address]):
                            c = canon(t)
                            edge(a, c, 'indirect-ghidra', cls_of(c), name_of(c))
                        n_ind_res += 1
                        indirect.append((a, ins.address, f'call {form}', 'ghidra-constant-propagation',
                                         ','.join(hx(t) for t in sorted(gcalls[ins.address]))))
                    else:
                        why = ''
                        btok = state.get(ALIAS.get(op0.mem.base)) if op0.type == X.X86_OP_MEM and op0.mem.base \
                            and op0.mem.index == 0 else None
                        if btok == ('VPTR',) and op0.mem.disp % 4 == 0:
                            why = f'virtual call pattern (mov r,[obj]; call [r+{op0.mem.disp:#x}]), slot ' \
                                  f'{op0.mem.disp // 4}, object type not known statically'
                        elif res and owner not in vt:
                            why = f'this-vtable slot {res[1]}, class {owner or "?"} has no known vtable'
                        elif op0.type == X.X86_OP_MEM and op0.mem.base == 0:
                            g = op0.mem.disp & 0xFFFFFFFF
                            why = f'function pointer at {g:#x}' + (f' ({gnames[g][0]})' if g in gnames else '')
                        indirect.append((a, ins.address, f'call {form}', 'unresolved', why))
                        edge(a, '', 'indirect-unresolved', '', '')

            # ---------------- memory operands
            for op in ins.operands:
                if op.type == X.X86_OP_MEM:
                    disp = op.mem.disp & 0xFFFFFFFF
                    absolute = op.mem.base == 0 and op.mem.segment in (0, X.X86_REG_DS)
                    sec = section_of(disp)
                    if ins.id == X.X86_INS_LEA:
                        if sec and absolute:
                            rw[(a, disp)].add((0, 'A', 'indexed' if op.mem.index else 'direct'))
                        elif sec and op.mem.base:
                            rw[(a, disp)].add((0, 'A', 'based'))
                        continue
                    acc = ('R' if op.access & capstone.CS_AC_READ else '') + \
                          ('W' if op.access & capstone.CS_AC_WRITE else '')
                    if sec and (absolute or (op.mem.base and disp >= 0x858000)):
                        if not (op.mem.segment == X.X86_REG_FS):
                            rw[(a, disp)].add((op.size, acc or 'R', 'direct' if absolute and not op.mem.index
                                               else ('indexed' if absolute else 'based')))
                    elif 'W' in acc and (op.mem.base or op.mem.index) and op.mem.segment != X.X86_REG_FS:
                        base = op.mem.base
                        if base == X.X86_REG_ESP or (base == X.X86_REG_EBP and ebp_frame):
                            continue
                        wtp += 1
                elif op.type == X.X86_OP_IMM and not (is_call or is_jmp):
                    v = op.imm & 0xFFFFFFFF
                    if section_of(v):
                        rw[(a, v)].add((0, 'A', 'imm'))
                    elif v in ehstubs or (in_code(v) and (v in procs or v in dump)):
                        c = canon(v)
                        if v in ehstubs:
                            for fl in ehstubs[v]:
                                if fl in dump:
                                    for s2, t2, _k in dump[fl]['calls']:
                                        cc = canon(t2)
                                        edge(a, cc, 'eh-unwind', cls_of(cc), name_of(cc))
                                    for ins2 in (i for lo, hi in dump[fl]['body'] for i in im.insns(lo, hi)):
                                        if ins2.id == X.X86_INS_JMP and ins2.operands[0].type == X.X86_OP_IMM:
                                            t2 = ins2.operands[0].imm & 0xFFFFFFFF
                                            if not any(lo <= t2 < hi for lo, hi in dump[fl]['body']):
                                                cc = canon(t2)
                                                edge(a, cc, 'eh-unwind', cls_of(cc), name_of(cc))
                        elif c != a:
                            edge(a, c, 'fnref', cls_of(c), name_of(c))

        summ[a] = dict(n_insns=len(ins_all), direct_calls=n_direct, indirect_calls=n_ind,
                       indirect_resolved=n_ind_res, unresolved_jumps=n_cjmp_unres, jumps_outside=n_jout,
                       writes_through_pointer=wtp, ebp_frame=int(ebp_frame))

    return dict(im=im, procs=procs, funcs=funcs, edges=edges, ecls=ecls, ename=ename, rw=rw,
                indirect=indirect, summ=summ, gnames=gnames, gsorted=gsorted, stats=stats, live=live,
                stubs=stubs, dump=dump, vt=vt, hood2stub=hood2stub)


# ---------------------------------------------------------------------------- body completion
def merge_ranges(rs):
    out = []
    for lo, hi in sorted(rs):
        if out and lo <= out[-1][1]:
            out[-1] = (out[-1][0], max(out[-1][1], hi))
        else:
            out.append((lo, hi))
    return out


def complete_body(im, ranges, entries, own):
    """Recursive descent from every direct branch target and conditional-jump fall-through of
    the Ghidra body that the body does not cover (Ghidra left the bytes undecoded or decoded
    them misaligned). Stops at other functions' entries, at code already in the body and at
    ret/jmp. Returns the added [lo, hi) ranges."""
    def covered(x, rs):
        i = bisect.bisect_right(rs, (x, float('inf'))) - 1
        return i >= 0 and rs[i][0] <= x < rs[i][1]
    rs = merge_ranges(ranges)
    added = []
    work = []
    for lo, hi in rs:
        last = None
        for ins in im.insns(lo, hi):
            last = ins
            if ins.group(capstone.CS_GRP_JUMP) and ins.operands and ins.operands[0].type == X.X86_OP_IMM:
                work.append(ins.operands[0].imm & 0xFFFFFFFF)
                if ins.id != X.X86_INS_JMP:
                    work.append(ins.address + ins.size)
        # a range that ends on an ordinary instruction falls through into undecoded bytes
        if last is not None and not last.group(capstone.CS_GRP_CALL) and \
                last.id not in (X.X86_INS_RET, X.X86_INS_RETF, X.X86_INS_JMP, X.X86_INS_INT3, X.X86_INS_HLT):
            work.append(last.address + last.size)
    seen = set()
    while work:
        t = work.pop()
        if t in seen or covered(t, rs) or (t in entries and t not in own) or \
                not (GAME_TEXT[0] <= t < RWCSEG[1] or HOODLUM[0] <= t < HOODLUM[1]):
            continue
        seen.add(t)
        x = t
        while True:
            ins = next(im.md.disasm(im.rd(x, 16), x), None)
            if ins is None:
                break
            added.append((x, x + ins.size))
            rs = merge_ranges(rs + [(x, x + ins.size)])
            if ins.group(capstone.CS_GRP_JUMP) and ins.operands and ins.operands[0].type == X.X86_OP_IMM:
                work.append(ins.operands[0].imm & 0xFFFFFFFF)
            if ins.id in (X.X86_INS_RET, X.X86_INS_JMP, X.X86_INS_INT3, X.X86_INS_HLT):
                break
            x += ins.size
            if covered(x, rs) or (x in entries and x not in own):
                break
    return merge_ranges(added)


# ---------------------------------------------------------------------------- `this` tracking
_DESC = {}


def _desc(ins):
    """Pre-decoded transfer facts of one instruction (cached by address)."""
    d = _DESC.get(ins.address)
    if d is None:
        is_call = bool(ins.group(capstone.CS_GRP_CALL))
        mov = None
        if ins.id == X.X86_INS_MOV and len(ins.operands) == 2 and ins.operands[0].type == X.X86_OP_REG \
                and ins.operands[0].reg in R32:
            s = ins.operands[1]
            if s.type == X.X86_OP_REG and s.reg in R32:
                mov = (ins.operands[0].reg, 'reg', s.reg, 0)
            elif s.type == X.X86_OP_MEM and s.mem.index == 0 and s.mem.base in R32 and s.mem.segment == 0:
                mov = (ins.operands[0].reg, 'mem', s.mem.base, s.mem.disp)
            else:
                mov = (ins.operands[0].reg, None, 0, 0)
        writes = tuple(sorted({ALIAS.get(r, r) for r in ins.regs_access()[1]})) if not is_call else ()
        d = _DESC[ins.address] = (is_call, mov, writes)
    return d


def transfer(ins, state):
    """Register tokens after `ins`: THIS (ecx at a member function's entry), VT (*THIS),
    SLOT n (VT[n]), VPTR (first dword of any other object). Must-analysis: anything written
    in a way not modelled loses its token."""
    is_call, mov, writes = _desc(ins)
    state = dict(state)
    if is_call:
        for r in CALLER_SAVED:
            state.pop(r, None)
        return state
    new = dst = None
    if mov:
        dst, kind, src, disp = mov
        if kind == 'reg':
            new = state.get(src)
        elif kind == 'mem':
            bt = state.get(src)
            if bt == ('THIS',) and disp == 0:
                new = ('VT',)
            elif bt == ('VT',) and disp % 4 == 0 and disp >= 0:
                new = ('SLOT', disp // 4)
            elif disp == 0 and src != dst:
                new = ('VPTR',)
    for r in writes:
        state.pop(r, None)
    if dst is not None and new is not None:
        state[dst] = new
    return state


def this_dataflow(ins_all, entries, gh):
    """Forward must-dataflow of transfer() over the body's control flow graph.
    Returns {instruction address: state before it}. Entry points start with ecx = THIS;
    instructions only reached through unmodelled flow start empty."""
    by = {i.address: i for i in ins_all}
    succ = collections.defaultdict(list)
    sw = collections.defaultdict(list)
    for j in gh:
        for s_, t, _k in j['jumps']:
            sw[s_].append(t)
    for i in ins_all:
        nxt = i.address + i.size
        if i.id in (X.X86_INS_RET, X.X86_INS_RETF):
            continue
        if i.group(capstone.CS_GRP_JUMP):
            op = i.operands[0] if i.operands else None
            if op is not None and op.type == X.X86_OP_IMM:
                t = op.imm & 0xFFFFFFFF
                if t in by:
                    succ[i.address].append(t)
            else:
                succ[i.address].extend(t for t in sw.get(i.address, ()) if t in by)
            if i.id == X.X86_INS_JMP:
                continue
        if nxt in by:
            succ[i.address].append(nxt)
    preds = collections.defaultdict(list)
    for s_, ts in succ.items():
        for t in ts:
            preds[t].append(s_)
    st_in, st_out = {}, {}
    visits = collections.Counter()
    work = collections.deque(sorted(by))
    queued = set(work)
    while work:
        x = work.popleft()
        queued.discard(x)
        if x in entries:
            new_in = {X.X86_REG_ECX: ('THIS',)}
        else:
            ps = [st_out[p] for p in preds[x] if p in st_out]
            if not ps:
                new_in = {}
            else:
                new_in = dict(ps[0])
                for q in ps[1:]:
                    new_in = {k: v for k, v in new_in.items() if q.get(k) == v}
            if not preds[x]:
                new_in = {}
        if x in st_in and st_in[x] == new_in and x in st_out:
            continue
        # transfer() is not monotone (VT can turn into VPTR when THIS is lost), so a loop can
        # oscillate; after 8 updates an instruction's state is pinned to empty (sound: less known)
        visits[x] += 1
        if visits[x] > 8:
            if st_in.get(x) == {} and x in st_out:
                continue
            new_in = {}
        st_in[x] = new_in
        out = transfer(by[x], new_in)
        if st_out.get(x) != out:
            st_out[x] = out
            for t in succ[x]:
                if t not in queued:
                    work.append(t)
                    queued.add(t)
    return st_in


# ============================================================================ port order
def port_order(live, edges):
    nodes = set(live)
    succ = collections.defaultdict(set)
    for (c, t, k), _n in edges.items():
        if k in ORDER_KINDS and t in nodes and c in nodes and t != c:
            succ[c].add(t)
    # iterative Tarjan
    index, low, on, stack, comp = {}, {}, set(), [], {}
    idx = 0
    ncomp = 0
    for root in sorted(nodes):
        if root in index:
            continue
        work = [(root, iter(sorted(succ[root])))]
        index[root] = low[root] = idx
        idx += 1
        stack.append(root)
        on.add(root)
        while work:
            v, it = work[-1]
            adv = False
            for w in it:
                if w not in index:
                    index[w] = low[w] = idx
                    idx += 1
                    stack.append(w)
                    on.add(w)
                    work.append((w, iter(sorted(succ[w]))))
                    adv = True
                    break
                elif w in on:
                    low[v] = min(low[v], index[w])
            if adv:
                continue
            work.pop()
            if work:
                low[work[-1][0]] = min(low[work[-1][0]], low[v])
            if low[v] == index[v]:
                while True:
                    w = stack.pop()
                    on.discard(w)
                    comp[w] = ncomp
                    if w == v:
                        break
                ncomp += 1
    csucc = collections.defaultdict(set)
    for v in nodes:
        for w in succ[v]:
            if comp[v] != comp[w]:
                csucc[comp[v]].add(comp[w])
    # Tarjan emits SCCs in reverse topological order: successors first
    level = {}
    for c in range(ncomp):
        level[c] = 1 + max((level[d] for d in csucc[c]), default=-1)
    members = collections.defaultdict(list)
    for v in nodes:
        members[comp[v]].append(v)
    # stable SCC ids: by (level, lowest member address)
    order = sorted(members, key=lambda c: (level[c], min(members[c])))
    sid = {c: i for i, c in enumerate(order)}
    return {v: (level[comp[v]], sid[comp[v]], len(members[comp[v]])) for v in nodes}


# ============================================================================ write
def write_csv(path, header, rows):
    with open(path, 'w', newline='') as f:
        w = csv.writer(f, lineterminator='\n')
        w.writerow(header)
        w.writerows(rows)


def cmd_build(cat):
    r = build(cat)
    procs, funcs, summ = r['procs'], r['funcs'], r['summ']
    live = r['live']

    rows = []
    for a in live:
        f, s, p = funcs[a], summ[a], procs[a]
        rows.append([hx(a), p['name'], p['state'], p['cls'], p['sub'], f.source,
                     hx(min(lo for lo, _ in f.ranges)), hx(max(hi for _, hi in f.ranges)),
                     sum(hi - lo for lo, hi in f.ranges), len(f.ranges),
                     ';'.join(f'{lo:#x}-{hi:#x}' for lo, hi in f.ranges),
                     hx(f.hood) if f.hood else '', s['n_insns'], s['direct_calls'], s['indirect_calls'],
                     s['indirect_resolved'], s['unresolved_jumps'], s['writes_through_pointer'], s['ebp_frame']])
    write_csv(cat / 'extents.csv',
              ['address', 'name', 'state', 'class', 'subsystem', 'body_source', 'start', 'end', 'size',
               'n_ranges', 'ranges', 'hoodlum_body', 'n_insns', 'direct_calls', 'indirect_calls',
               'indirect_resolved', 'unresolved_computed_jumps', 'writes_through_pointer', 'ebp_frame'], rows)

    def k(e):
        c, t, kind = e
        return (c, kind, t if isinstance(t, int) else -1, str(t))
    erows = []
    for e in sorted(r['edges'], key=k):
        c, t, kind = e
        erows.append([hx(c), hx(t) if isinstance(t, int) else t, kind, r['ecls'][e], r['ename'][e],
                      r['edges'][e]])
    write_csv(cat / 'callgraph.csv', ['caller', 'callee', 'kind', 'callee_class', 'callee_name', 'sites'], erows)

    gn, gs = r['gnames'], r['gsorted']

    def near(addr):
        i = bisect.bisect_right(gs, addr) - 1
        if i >= 0 and gs[i] != addr and addr - gs[i] < 0x400:
            return f'{gn[gs[i]][0]}+{addr - gs[i]:#x}'
        return ''
    order_acc = {'R': 0, 'W': 1, 'RW': 2, 'A': 3}
    rrows = []
    for (fa, ga) in sorted(r['rw']):
        for size, acc, mode in sorted(r['rw'][(fa, ga)], key=lambda x: (x[0], order_acc[x[1]], x[2])):
            rrows.append([hx(fa), hx(ga), section_of(ga), gn[ga][0] if ga in gn else '', near(ga),
                          size, acc, mode])
    write_csv(cat / 'rwsets.csv', ['function', 'address', 'section', 'name', 'near', 'size', 'access', 'mode'],
              rrows)

    write_csv(cat / 'indirect-calls.csv', ['function', 'site', 'instruction', 'resolution', 'detail'],
              [[hx(a), hx(s), form, res, det] for a, s, form, res, det in sorted(r['indirect'])])

    # branch / vtable targets in game code that are not catalogue procedures (catalogue gaps)
    gap = collections.defaultdict(lambda: [0, set()])
    for (c, t, kind), n in r['edges'].items():
        if r['ecls'][(c, t, kind)] == 'noncatalogue' and isinstance(t, int):
            gap[t][0] += n
            gap[t][1].add(kind)
    write_csv(cat / 'raw' / 'cat7-noncatalogue-targets.csv', ['address', 'sites', 'kinds'],
              [[hx(t), v[0], ';'.join(sorted(v[1]))] for t, v in sorted(gap.items())])

    po = port_order(live, r['edges'])
    write_csv(cat / 'port-order.csv', ['address', 'name', 'subsystem', 'level', 'scc_id', 'scc_size'],
              [[hx(a), procs[a]['name'], procs[a]['sub'], *po[a]]
               for a in sorted(live, key=lambda a: (po[a][0], po[a][1], a))])

    # ---- stats
    st = dict(r['stats'])
    kinds = collections.Counter()
    sites = collections.Counter()
    for (c, t, kind), n in r['edges'].items():
        kinds[kind] += 1
        sites[kind] += n
    exc = collections.Counter()
    for (c, t, kind), n in r['edges'].items():
        if kind in ('call', 'tailcall') and not (r['ecls'][(c, t, kind)] in ('game', 'RW')):
            exc[r['ecls'][(c, t, kind)]] += n
    st['edges_by_kind'] = dict(sorted(kinds.items()))
    st['sites_by_kind'] = dict(sorted(sites.items()))
    st['direct_call_sites_into_non_live'] = dict(sorted(exc.items()))
    st['imports'] = len({t for (c, t, k) in r['edges'] if k == 'import'})
    res = collections.Counter(x[3] for x in r['indirect'])
    st['indirect_sites_by_resolution'] = dict(sorted(res.items()))
    st['functions'] = len(live)
    st['functions_with_unresolved_indirect_calls'] = len({x[0] for x in r['indirect'] if x[3] == 'unresolved'})
    st['functions_writing_through_pointers'] = sum(1 for a in live if r['summ'][a]['writes_through_pointer'])
    st['rwsets_rows'] = len(rrows)
    st['rwsets_globals'] = len({ga for (_fa, ga) in r['rw']})
    st['rwsets_rows_named'] = sum(1 for x in rrows if x[3])
    st['global_names_known'] = len(gn)
    accs = collections.Counter(x[6] for x in rrows)
    st['rwsets_by_access'] = dict(sorted(accs.items()))
    lv = collections.Counter(v[0] for v in po.values())
    st['port_levels'] = max(lv) + 1
    st['port_level0'] = lv[0]
    sccs = collections.Counter(v[1] for v in po.values())
    big = sorted(((n, s) for s, n in sccs.items() if n > 1), reverse=True)
    st['sccs_nontrivial'] = len(big)
    st['largest_sccs'] = big[:5]
    st['funcs_in_nontrivial_sccs'] = sum(n for n, _ in big)
    # Ghidra vs capstone cross-check: Ghidra direct CALL refs missing from our edges
    miss = 0
    for a in live:
        f = funcs[a]
        for x in (a, f.hood):
            if x is None or x not in r['dump']:
                continue
            for s, t, kind in r['dump'][x]['calls']:
                if kind == 'UNCONDITIONAL_CALL' and t != r['stubs'].get(a):
                    # Ghidra marks tail jumps CALL_RETURN; those are our 'tailcall' edges
                    c = r['hood2stub'].get(t, t)
                    if (a, c, 'call') not in r['edges'] and (a, c, 'tailcall') not in r['edges'] \
                            and not any(lo <= t < hi for lo, hi in f.ranges):
                        miss += 1
    st['ghidra_direct_calls_not_in_edges'] = miss
    st['noncatalogue_game_targets'] = len(gap)
    (cat / 'raw').mkdir(exist_ok=True)
    (cat / 'raw' / 'cat7-stats.json').write_text(json.dumps(st, indent=1, sort_keys=True) + '\n')
    print(json.dumps(st, indent=1, sort_keys=True))


# ============================================================================ spot check
def source_body(path, owner, name):
    """Text of `owner::name(...) { ... }` in a gta-reversed source file (first definition)."""
    try:
        t = open(path, errors='replace').read()
    except OSError:
        return None
    q = rf'\b{re.escape(owner)}::{re.escape(name)}\s*\(' if owner else rf'^[^\n;]*\b{re.escape(name)}\s*\('
    for m in re.finditer(q, t, re.M):
        i = t.find('{', m.end())
        semi = t.find(';', m.end())
        if i < 0 or (0 <= semi < i):
            continue
        depth, j = 0, i
        while j < len(t):
            depth += {'{': 1, '}': -1}.get(t[j], 0)
            if depth == 0:
                return t[m.start():j + 1]
            j += 1
    return None


def cmd_spotcheck(cat, n, seed):
    """Compare n random gr-reversed functions with their gta-reversed source: every qualified
    function call (Class::Name) and every named global the source mentions must appear in
    callgraph.csv / rwsets.csv. Prints hits and misses; misses need a manual explanation."""
    import random
    hooks = list(csv.DictReader(open(cat / 'functions.csv')))
    by_name = collections.defaultdict(set)
    for h in hooks:
        by_name[f"{h['owner']}::{h['name']}" if h['owner'] else h['name']].add(int(h['address'], 16))
    gn = global_names(GR, PSDK, lambda a: section_of(a) is not None)
    gby = collections.defaultdict(set)
    for a, (nm, _src) in gn.items():
        gby[nm].add(a)
    edges = collections.defaultdict(set)
    for r in csv.DictReader(open(cat / 'callgraph.csv')):
        if r['callee'].startswith('0x'):
            edges[int(r['caller'], 16)].add(int(r['callee'], 16))
    rws = collections.defaultdict(set)
    for r in csv.DictReader(open(cat / 'rwsets.csv')):
        rws[int(r['function'], 16)].add(int(r['address'], 16))

    def g_hit(fa, addrs):
        """exact address, or an access inside the object (struct field / array element,
        up to 0x400 bytes past the named address)"""
        for g in addrs:
            if g in rws[fa]:
                return 'ok  '
            if any(0 < x - g < 0x400 for x in rws[fa]):
                return 'ok+ '
        return None
    ext = {int(r['address'], 16): r for r in csv.DictReader(open(cat / 'extents.csv'))}
    cand = sorted(int(h['address'], 16) for h in hooks if h['reversed'] == 'yes' and h['status'] in ('ok', 'stub-to-.HOODLUM'))
    rnd = random.Random(seed)
    picked, tries = [], 0
    while len(picked) < n and tries < 10000:
        tries += 1
        a = rnd.choice(cand)
        h = next(x for x in hooks if int(x['address'], 16) == a)
        body = source_body(GR / h['file'], h['owner'], h['name'])
        if body and body.count('\n') >= 4 and a in ext and a not in [p[0] for p in picked]:
            picked.append((a, h, body))
    tot_f = tot_g = miss_f = miss_g = 0
    for a, h, body in picked:
        own = h['owner']
        print(f"=== {a:#x} {own}::{h['name']}  ({h['file']}:{h['line']})  body {ext[a]['ranges']}")
        code = re.sub(r'//[^\n]*', '', body)
        fn_refs, g_refs = set(), set()
        for m in re.finditer(r'\b((?:[A-Za-z_]\w*::)*[A-Za-z_]\w*)\s*(\()?', code):
            tok, call = m.group(1), m.group(2)
            pre = code[max(0, m.start() - 2):m.start()]
            if pre.endswith('.') or pre.endswith('->'):
                continue  # member of some object, not a global / static
            if tok == f"{own}::{h['name']}":
                continue
            cands = [tok] + ([f'{own}::{tok}'] if own and '::' not in tok else [])
            if call:
                for c in cands:
                    if c in by_name:
                        fn_refs.add((c, frozenset(by_name[c])))
                        break
            else:
                for c in cands:
                    if c in gby:
                        g_refs.add((c, frozenset(gby[c])))
                        break
        for nm, addrs in sorted(fn_refs):
            tot_f += 1
            ok = bool(addrs & edges[a])
            miss_f += not ok
            print(f"  call   {'ok  ' if ok else 'MISS'} {nm} {','.join(hex(x) for x in sorted(addrs))}")
        for nm, addrs in sorted(g_refs):
            tot_g += 1
            ok = g_hit(a, addrs)
            miss_g += not ok
            print(f"  global {ok or 'MISS'} {nm} {','.join(hex(x) for x in sorted(addrs))}")
    print(f'TOTAL calls {tot_f - miss_f}/{tot_f} found, globals {tot_g - miss_g}/{tot_g} found')


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in ('ghidra', 'build', 'analyse', 'spotcheck'):
        sys.exit(__doc__)
    if sys.argv[1] == 'spotcheck':
        return cmd_spotcheck(ROOT / 'catalog', int(sys.argv[2]), int(sys.argv[3]))
    cat = Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / 'catalog'
    {'ghidra': lambda: cmd_ghidra(cat), 'build': lambda: cmd_build(cat), 'analyse': cmd_analyse}[sys.argv[1]]()


if __name__ == '__main__':
    main()
