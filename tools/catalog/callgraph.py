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

  callgraph.py gaps [catalog-dir]
      CAT-12: append call targets outside the catalogue to catalog/extra-code-addresses.csv
      (exit 3 = rows added, rerun the pipeline from merge_ghidra.py; 0 = catalogue closed).

  callgraph.py fallthrough [catalog-dir]
      CAT-16: append the mid-function entries that `build` found (catalog/fallthrough.csv, decision
      mis-split) to catalog/fallthrough-splits.csv (exit 3 = rows added, rerun the pipeline; 4 = an
      undecided case needs a hand row; 0 = done).

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
PROJ_DIR = Path(os.environ.get('SA_GHIDRA_PROJ', ROOT / 'target' / 'ghidra'))  # Ghidra rejects '.' path elements
PROJ_NAME = 'gta_sa_10us'
SCRIPT_DIR = Path(__file__).resolve().parent / 'ghidra'

BASE = 0x400000
GAME_TEXT = (0x401000, 0x857000)
RWCSEG = (0x857000, 0x858000)
HOODLUM = (0x1556000, 0x1577000)
PROT_TEXT = (0xCB1000, 0x1301000)
CODE = [(0x401000, 0x858000), (0xCB1000, 0x1301000), HOODLUM]
# data sections an absolute operand can name (section label, lo, hi)
DATA = [('.rdata', 0x858000, 0x8A4000), ('.data', 0x8A4000, 0xC9E000), ('_TEXT_HA', 0xC9E000, 0xCAF000),
        ('_rwdseg', 0xCAF000, 0xCB0000), ('.rsrc', 0xCB0000, 0xCB1000), ('.data2', 0x1301000, 0x1556000)]
LIVE_STATES = ('gr-reversed', 'gr-unreversed', 'gr-called-only', 'unknown')
# edges that make a function depend on another for porting (leaf-first order)
ORDER_KINDS = ('call', 'tailcall', 'virtual', 'virtual-override', 'virtual-typed', 'virtual-typed-override',
               'indirect-ghidra', 'eh-unwind', 'fallthrough')
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
X87_STORES = ('fst', 'fstp', 'fist', 'fistp', 'fisttp', 'fbstp', 'fnstcw', 'fstcw', 'fnstsw', 'fstsw',
              'fnstenv', 'fstenv', 'fnsave', 'fsave')


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
        name = r['name'] if 'name' in r else (r['gr_name'] or (cls[a]['name'] if a in cls else ''))
        out[a] = dict(state=r['state'], cls=r['class'] or ('game' if r['state'] != 'excluded' else ''),
                      sub=r['subsystem'], name=name, ghidra=r['ghidra_name'])
    return out


def securom_thunk(im, t):
    """SecuROM code called from game bodies, outside the catalogue (CAT-12):
    (a) anything in the protection `.text` (0xCB1000-0x1301000, the SecuROM section of the no-CD
        build; never game code);
    (b) `.HOODLUM` code that rewrites its own return address (`add dword ptr [esp(+k)], imm`
        before the first ret/jmp, within 16 instructions): it decodes a key into a register and
        returns past junk bytes behind the call site. Checked on the bytes."""
    if PROT_TEXT[0] <= t < PROT_TEXT[1]:
        return True
    for n, ins in enumerate(im.md.disasm(im.rd(t, 160), t)):
        if ins.mnemonic == 'add' and ins.op_str.startswith('dword ptr [esp') and \
                ins.operands[1].type == X.X86_OP_IMM:
            return True
        if ins.mnemonic in ('ret', 'jmp') and not ins.op_str.startswith('dword ptr [esp'):
            return False
        if n >= 16:
            return False
    return False


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
    """{class: (vtable address, n virtuals)} from RH_ScopedVirtualClass (in .cpp files and, since
    CAT-13, the 8 declared in headers)."""
    out = {}
    for f in sorted(glob.glob(str(gr / 'source' / '**' / '*.cpp'), recursive=True)) + \
            sorted(glob.glob(str(gr / 'source' / '**' / '*.h'), recursive=True)):
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
         raw / 'remove.txt', raw / 'create.txt', out], PROJ_DIR / 'dump.log')  # log has timestamps: kept out of catalog/
    summary = [l for l in open(PROJ_DIR / 'dump.log') if 'CAT7' in l]
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
        if (HOODLUM[0] <= t < HOODLUM[1] or PROT_TEXT[0] <= t < PROT_TEXT[1]) and securom_thunk(im, t):
            return 'excluded:protection-thunk'
        if HOODLUM[0] <= t < HOODLUM[1]:
            return 'noncatalogue:hoodlum'
        if PROT_TEXT[0] <= t < PROT_TEXT[1]:
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
    stats = collections.Counter()
    hoodlum_body_of = dict(stubs)   # stub -> .HOODLUM body (calls may target the body directly)

    def slot_impl(c, slot):
        a, n = vt[c]
        if slot >= n:
            return None
        v = im.u32(a + 4 * slot)
        return v if in_code(v) else None

    # CAT-13: receiver types (validated layouts, typed globals, return types)
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import cat13_types as CT
    TypeCtx.layouts = CT.Layouts(GR, cat)
    TypeCtx.vclasses = set(vt)
    TypeCtx.gtypes = CT.global_types(GR, PSDK, TypeCtx.vclasses)
    rnames = CT.return_types(GR, TypeCtx.vclasses)
    TypeCtx.rettype = {}
    for a_, p_ in procs.items():
        nm_ = re.sub(r'-.*$', '', p_['name'])
        own_, _, fn_ = nm_.rpartition('::')
        rc_ = rnames.get((own_.split('::')[-1], fn_))
        if rc_:
            TypeCtx.rettype[a_] = rc_
            for h_ in (hoodlum_body_of.get(a_),):
                if h_:
                    TypeCtx.rettype[h_] = rc_
    stats['cat13_validated_layouts'] = sum(1 for c in TypeCtx.layouts.vsize if c in TypeCtx.layouts.decl
                                           and TypeCtx.layouts.validated(c))
    stats['cat13_typed_globals'] = len(TypeCtx.gtypes)
    stats['cat13_typed_returns'] = len(TypeCtx.rettype)
    TYPED_FANOUT = 16

    ehstubs = eh_funclets(im, procs)

    # ---- function extents
    entry_set = set(procs) | set(dump)
    # CAT-17: hand-proven continuation points that no static flow reaches (switch tables Ghidra did
    # not resolve, SecuROM stubs that "return" to a computed address); catalog/extent-seeds.csv
    seeds = {}
    if (cat / 'extent-seeds.csv').exists():
        for r in csv.DictReader(open(cat / 'extent-seeds.csv')):
            seeds[int(r['address'], 16)] = [int(x, 16) for x in r['seeds'].split(';') if x]
    funcs = {}
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
        extra = complete_body(im, f.ranges, entry_set, {a, f.hood} - {None}, seeds.get(a, ()))
        if a in seeds:
            stats['seeded_functions'] += 1
        if extra:
            f.ranges = merge_ranges(f.ranges + extra)
            f.source += '+seeded' if a in seeds else '+completed'
            stats['completed_functions'] += 1
            stats['completed_bytes'] += sum(hi - lo for lo, hi in extra)
        funcs[a] = f
        stats['extent:' + f.source] += 1

    # ---- per-function analysis
    edges = collections.defaultdict(int)      # (caller, callee, kind) -> sites
    ecls, ename = {}, {}
    rw = collections.defaultdict(set)         # (func, addr) -> {(size, access, mode)}
    indirect = []                             # (func, site, form, resolution, detail)
    codeimm = set()                           # (func, site, value): game-code immediates, no proc (CAT-12b)
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
        TypeCtx.owner = owner if owner and TypeCtx.layouts.validated(owner) else None
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
                    typed = None
                    if op0.type == X.X86_OP_MEM and op0.mem.index == 0 and op0.mem.base:
                        tok = state.get(ALIAS.get(op0.mem.base))
                        if tok == ('VT',) and op0.mem.disp % 4 == 0:
                            res = ('slot', op0.mem.disp // 4)
                        elif tok and tok[0] == 'VTC' and op0.mem.disp % 4 == 0 and op0.mem.disp >= 0:
                            typed = (tok[1], op0.mem.disp // 4)
                    elif op0.type == X.X86_OP_REG:
                        tok = state.get(ALIAS.get(op0.reg))
                        if tok and tok[0] == 'SLOT':
                            res = ('slot', tok[1])
                        elif tok and tok[0] == 'SLOTC':
                            typed = (tok[1], tok[2])
                    own_ok = bool(typed) and typed[0] in vt and typed[1] < vt[typed[0]][1]
                    if typed and (own_ok or any(typed[1] < vt[dc][1] for dc in derived.get(typed[0], ()))):
                        # CAT-13: virtual call on an object of static class C (typed member / global /
                        # return value): C's implementation plus the overrides of its derived classes
                        c13, slot = typed
                        # a base without the slot in its known vtable (abstract CTask: 6 of 9 slots
                        # declared): the derived classes' implementations only
                        impl = slot_impl(c13, slot) if own_ok else None
                        over = sorted({slot_impl(dc, slot) for dc in derived.get(c13, ())} - {impl, None})
                        tgts = ([impl] if impl is not None and impl != 0x82263A else [])
                        if len(over) <= TYPED_FANOUT:
                            tgts += over
                        for t_ in tgts:
                            cc = canon(t_)
                            edge(a, cc, 'virtual-typed' if t_ == impl else 'virtual-typed-override', cls_of(cc),
                                 name_of(cc))
                        n_ind_res += 1
                        indirect.append((a, ins.address, f'call {form}', 'virtual-typed' if tgts else 'virtual-typed-wide',
                                         (f'{c13} vtable {vt[c13][0]:#x} slot {slot}' if own_ok else
                                          f'{c13} slot {slot} (not in its own known vtable)') +
                                         (f' -> {impl:#x}' if impl is not None else '') +
                                         (f'; {len(over)} override(s) in derived classes' if over else '') +
                                         ('' if len(over) <= TYPED_FANOUT else f' (more than {TYPED_FANOUT}: no edges)')))
                    elif res and owner in vt:
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
                        if typed:
                            why = f'virtual call on static class {typed[0]} slot {typed[1]} (CAT-13), ' + (
                                'no vtable known for the class' if typed[0] not in vt else 'slot outside its vtable')
                        elif btok == ('VPTR',) and op0.mem.disp % 4 == 0:
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
                    if ins.mnemonic in X87_STORES:
                        # capstone reports x87 store memory operands as R or with no flags at all;
                        # these instructions only write their memory operand (CAT-15).
                        acc = 'W'
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
                    elif GAME_TEXT[0] <= v < RWCSEG[1]:
                        codeimm.add((a, ins.address, v))

        summ[a] = dict(n_insns=len(ins_all), direct_calls=n_direct, indirect_calls=n_ind,
                       indirect_resolved=n_ind_res, unresolved_jumps=n_cjmp_unres, jumps_outside=n_jout,
                       writes_through_pointer=wtp, ebp_frame=int(ebp_frame))

    # ---- CAT-16: fall-through into another catalogue entry
    fall = fallthroughs(im, cat, procs, funcs, live, dump, edges, entry_set)
    for x in fall:
        if x['decision'] == 'fallthrough':
            edge(x['faller'], x['target'], 'fallthrough', cls_of(x['target']), name_of(x['target']))
    stats['cat16_noreturn_functions'] = len(fall.noreturn)
    stats['cat16_by_decision'] = dict(sorted(collections.Counter(x['decision'] for x in fall).items()))

    return dict(im=im, procs=procs, funcs=funcs, edges=edges, ecls=ecls, ename=ename, rw=rw,
                indirect=indirect, codeimm=codeimm, summ=summ, gnames=gnames, gsorted=gsorted, stats=stats, live=live,
                stubs=stubs, dump=dump, vt=vt, hood2stub=hood2stub, fall=fall)


# ---------------------------------------------------------------------------- CAT-16 fall-through
# Ghidra's genuinely non-returning MSVC runtime functions (the names DumpCat7.java keeps flagged)
NORETURN_NAMES = ('_abort', '__exit', '_exit', '__CxxThrowException@8', '_longjmp', '__amsg_exit')
NORETURN_IMPORTS = ('KERNEL32.dll!ExitProcess', 'KERNEL32.dll!TerminateProcess')
TERMINATORS = (X.X86_INS_RET, X.X86_INS_RETF, X.X86_INS_JMP, X.X86_INS_INT3, X.X86_INS_HLT, X.X86_INS_UD2)
SPLITS_FILE = 'fallthrough-splits.csv'


class Fall(list):
    noreturn = ()


def noreturn_set(im, dump):
    """Entries of functions that never return: the named MSVC runtime ones, MSVC `exit` (same shape as
    `__exit` = doexit(code, 1, 0), but doexit(code, 0, 0); Ghidra leaves it unnamed), and, to a fixpoint,
    every function whose body has no ret, leaves its body only towards a non-returning function and
    calls at least one."""
    nr = {a for a, j in dump.items() if j['name'] in NORETURN_NAMES and not j['thunk']}
    iat = {a for a, n in im.iat.items() if n in NORETURN_IMPORTS}

    def head(a, n):
        return [(i.mnemonic, i.op_str) for _k, i in zip(range(n), im.md.disasm(im.rd(a, 32), a))]
    ex = [a for a, j in dump.items() if j['name'] == '__exit']
    if ex:
        h = head(ex[0], 4)
        if [x for x, _ in h] == ['push', 'push', 'push', 'call'] and h[1] == ('push', '1'):
            want = [h[0], ('push', '0'), h[2], h[3][0]]
            doexit = h[3][1]
            for a in sorted(dump):
                g = head(a, 4)
                if len(g) == 4 and g[:3] == want[:3] and g[3][0] == 'call' and \
                        int(g[3][1], 16) == int(doexit, 16):
                    nr.add(a)

    def target(ins):
        op = ins.operands[0] if ins.operands else None
        if op is not None and op.type == X.X86_OP_IMM:
            return op.imm & 0xFFFFFFFF
        if op is not None and op.type == X.X86_OP_MEM and op.mem.base == 0 and op.mem.index == 0:
            return ('iat', op.mem.disp & 0xFFFFFFFF)
        return None

    def nr_call(ins):
        t = target(ins)
        return t[1] in iat if isinstance(t, tuple) else t in nr
    # per body: None when it can return (ret, or a jump that leaves the body to something other than a
    # direct target), else (direct jump targets outside the body, call targets)
    summ = {}
    for a in sorted(dump):
        body = dump[a]['body']
        jo, ct, ok = set(), set(), bool(body)
        for lo, hi in body:
            for ins in im.insns(lo, hi):
                if ins.id in (X.X86_INS_RET, X.X86_INS_RETF):
                    ok = False
                elif ins.group(capstone.CS_GRP_JUMP):
                    t = target(ins)
                    if not isinstance(t, int):
                        ok = False
                    elif not any(l2 <= t < h2 for l2, h2 in body):
                        jo.add(t)
                elif ins.group(capstone.CS_GRP_CALL):
                    ct.add(target(ins))
            if not ok:
                break
        if ok:
            summ[a] = (jo, ct)
    changed = True
    while changed:
        changed = False
        for a, (jo, ct) in summ.items():
            if a not in nr and jo <= nr and any(t[1] in iat if isinstance(t, tuple) else t in nr for t in ct):
                nr.add(a)
                changed = True
    return nr, nr_call


def code_xrefs(im, targets):
    """{target: sorted sites} of every rel32 call / jmp / jcc and every 4-byte little-endian occurrence
    (pointers, immediates) anywhere in the image that names one of `targets`. Byte-level scan, so it
    over-approximates (never misses a rel32 or absolute reference)."""
    out = collections.defaultdict(set)
    if not targets:
        return out
    img = bytes(im.img)
    for lo, hi in CODE:
        seg = img[lo - BASE:hi - BASE]
        for m in re.finditer(rb'[\xe8\xe9]|\x0f[\x80-\x8f]', seg, re.S):
            p = m.start()
            n = len(m.group(0))
            if p + n + 4 > len(seg):
                continue
            t = (lo + p + n + 4 + struct.unpack_from('<i', seg, p + n)[0]) & 0xFFFFFFFF
            if t in targets:
                out[t].add((lo + p, 'rel32'))
    for t in targets:
        v = struct.pack('<I', t)
        i = img.find(v)
        while i >= 0:
            out[t].add((BASE + i, 'u32'))
            i = img.find(v, i + 1)
    return out


def fallthroughs(im, cat, procs, funcs, live, dump, edges, entry_set):
    """Every instruction of a live body whose fall-through address leaves the body and is another
    catalogue entry. Decision per case:
      noreturn-call  the instruction is a call to a non-returning function: no flow, no edge
      fallthrough    the entry is referenced from outside the faller (call, jump, pointer, gta-reversed
                     call): a real function that the faller shares as its tail -> `fallthrough` edge
      mis-split      the entry has no reference outside the faller and the instruction before it is
                     ordinary code (not a call, not nop/int3 padding): a mid-function label; it is
                     merged into the faller by `callgraph.py fallthrough` (catalog/fallthrough-splits.csv)
      undecided      anything else: needs a hand decision in catalog/fallthrough-splits.csv"""
    nr, nr_call = noreturn_set(im, dump)
    cand = []
    for a in live:
        f = funcs[a]
        own = [(lo, hi) for lo, hi in f.ranges]
        for lo, hi in own:
            insl = list(im.insns(lo, hi))
            for k, ins in enumerate(insl):
                if ins.id in TERMINATORS:
                    continue
                n = ins.address + ins.size
                if any(l2 <= n < h2 for l2, h2 in own) or n not in entry_set or n in (a, f.hood):
                    continue
                # dead code after a call to a non-returning function (Ghidra keeps it in the body when
                # it does not know the callee never returns): walk back to the start of the straight
                # line; a branch target or the range start makes it live
                tg = {t for x in (a, f.hood) if x in dump for _s, t, _k in dump[x]['jumps']}
                for i2 in (i for l2, h2 in own for i in im.insns(l2, h2)):
                    if i2.group(capstone.CS_GRP_JUMP) and i2.operands and i2.operands[0].type == X.X86_OP_IMM:
                        tg.add(i2.operands[0].imm & 0xFFFFFFFF)
                j, dead = k, None
                while j > 0 and insl[j].address not in tg:
                    p = insl[j - 1]
                    if p.group(capstone.CS_GRP_CALL) and nr_call(p):
                        dead = p
                        break
                    if p.id in TERMINATORS:
                        break
                    j -= 1
                cand.append((a, dead or ins, n))
    xr = code_xrefs(im, {n for _a, _i, n in cand})
    gcall = collections.defaultdict(set)
    for fa, j in dump.items():
        for s, t, _k in j['calls']:
            gcall[t].add(s)
    fall = Fall()
    fall.noreturn = nr
    for a, ins, n in cand:
        rs = funcs[a].ranges
        inside = lambda s: any(lo <= s < hi for lo, hi in rs)  # noqa: E731
        refs = sorted({f'{k} {s:#x}' for s, k in xr.get(n, ()) if not inside(s)} |
                      {f'ghidra-call {s:#x}' for s in gcall.get(n, ()) if not inside(s)} |
                      {f'{k} from {c:#x}' for (c, t, k) in edges if t == n and c != a})
        if n in procs and procs[n]['state'] == 'gr-called-only':
            refs.append('called from gta-reversed (plugin::Call)')
        own_jumps = sorted(s for s, _k in xr.get(n, ()) if inside(s))
        last = f'{ins.mnemonic} {ins.op_str}'.strip()
        nrc = ins.group(capstone.CS_GRP_CALL) and nr_call(ins)
        ev = ((f'{ins.address:#x} `{last}` never returns; the dead bytes after it run into {n:#x}; ' if nrc else
               f'{ins.address:#x} `{last}` falls into {n:#x}; ')
              + (f'refs: {", ".join(refs[:6])}' + (f' (+{len(refs) - 6})' if len(refs) > 6 else '')
                 if refs else 'no reference outside the faller')
              + (f'; also jumped to from the faller at {",".join(hex(s) for s in own_jumps)}' if own_jumps else ''))
        if nrc:
            dec = 'noreturn-call'
        elif refs:
            dec = 'fallthrough'
        elif not ins.group(capstone.CS_GRP_CALL) and ins.id not in (X.X86_INS_NOP,):
            dec = 'mis-split'
        else:
            dec = 'undecided'
        fall.append(dict(faller=a, site=ins.address, insn=last, target=n, decision=dec, evidence=ev))
    return fall


def cmd_fallthrough(cat):
    """CAT-16 fixpoint step: append every `mis-split` fall-through target of catalog/fallthrough.csv to
    catalog/fallthrough-splits.csv (classify_unknowns.py classifies those rows excluded:mis-split, so
    the next pass merges them into the faller's extent). Exit 3 when rows were added (rerun the
    pipeline), 4 when a case is undecided (add a hand row), 0 when nothing is left to merge."""
    path = cat / SPLITS_FILE
    have = {}
    if path.exists():
        have = {int(r['address'], 16): r for r in csv.DictReader(open(path))}
    new, bad = {}, []
    for r in csv.DictReader(open(cat / 'fallthrough.csv')):
        t = int(r['target'], 16)
        if r['decision'] == 'mis-split' and t not in have:
            new[t] = {'address': hx(t), 'host': r['faller'], 'rule': 'cat16-auto',
                      'evidence': f"mid-function label of {r['faller']} {r['faller_name']}: {r['evidence']}"}
        elif r['decision'] == 'undecided':
            bad.append(r)
    if new:
        rows = sorted(list(have.values()) + list(new.values()), key=lambda x: int(x['address'], 16))
        write_csv(path, ['address', 'host', 'rule', 'evidence'],
                  [[x['address'], x['host'], x['rule'], x['evidence']] for x in rows])
        print(f'fallthrough: merged {len(new)} mid-function entries into their host ({path})')
        sys.exit(3)
    if bad:
        for r in bad:
            print('fallthrough: undecided, add a hand row to', path, dict(r))
        sys.exit(4)
    n = sum(1 for r in csv.DictReader(open(cat / 'fallthrough.csv')) if r['decision'] == 'fallthrough')
    print(f'fallthrough: {n} fall-through edges, {len(have)} merged mid-function entries, nothing new')


# ---------------------------------------------------------------------------- body completion
def merge_ranges(rs):
    out = []
    for lo, hi in sorted(rs):
        if out and lo <= out[-1][1]:
            out[-1] = (out[-1][0], max(out[-1][1], hi))
        else:
            out.append((lo, hi))
    return out


def complete_body(im, ranges, entries, own, seeds=()):
    """Recursive descent from every direct branch target and conditional-jump fall-through of
    the Ghidra body that the body does not cover (Ghidra left the bytes undecoded or decoded
    them misaligned), and from the hand-proven `seeds` (catalog/extent-seeds.csv). Stops at other
    functions' entries, at code already in the body and at ret/jmp. Returns the added [lo, hi)
    ranges."""
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
    work.extend(seeds)
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
    """Pre-decoded transfer facts of one instruction (cached by address).
    mov = (dst, kind, src, disp): kind 'reg' (mov r, r2), 'mem' (mov r, [r2 + disp]), 'abs' (mov r, [disp]),
    'imm' (mov r, imm), 'lea' (lea r, [r2 + disp]), 'leaabs' (lea r, [disp]) or None (anything else)."""
    d = _DESC.get(ins.address)
    if d is None:
        is_call = bool(ins.group(capstone.CS_GRP_CALL))
        mov = None
        target = None
        if is_call and ins.operands and ins.operands[0].type == X.X86_OP_IMM:
            target = ins.operands[0].imm & 0xFFFFFFFF
        if ins.id in (X.X86_INS_MOV, X.X86_INS_LEA) and len(ins.operands) == 2 and \
                ins.operands[0].type == X.X86_OP_REG and ins.operands[0].reg in R32:
            s_ = ins.operands[1]
            lea = ins.id == X.X86_INS_LEA
            if s_.type == X.X86_OP_REG and s_.reg in R32 and not lea:
                mov = (ins.operands[0].reg, 'reg', s_.reg, 0)
            elif s_.type == X.X86_OP_MEM and s_.mem.index == 0 and s_.mem.base in R32 and s_.mem.segment == 0:
                mov = (ins.operands[0].reg, 'lea' if lea else 'mem', s_.mem.base, s_.mem.disp)
            elif s_.type == X.X86_OP_MEM and s_.mem.index == 0 and s_.mem.base == 0 and s_.mem.segment in (0, X.X86_REG_DS):
                mov = (ins.operands[0].reg, 'leaabs' if lea else 'abs', 0, s_.mem.disp & 0xFFFFFFFF)
            elif s_.type == X.X86_OP_IMM and not lea:
                mov = (ins.operands[0].reg, 'imm', 0, s_.imm & 0xFFFFFFFF)
            else:
                mov = (ins.operands[0].reg, None, 0, 0)
        writes = tuple(sorted({ALIAS.get(r, r) for r in ins.regs_access()[1]})) if not is_call else ()
        d = _DESC[ins.address] = (is_call, mov, writes, target)
    return d


class TypeCtx:
    """CAT-13 receiver typing for one function: owner class of `this` and the typed sources
    (cat13_types.py). Tokens: OBJ C (pointer to an object of static class C), VTC C (its vptr),
    SLOTC C n (slot n of it)."""
    layouts = None
    gtypes = {}
    rettype = {}
    vclasses = set()
    owner = None

    @classmethod
    def member(cls, c, disp):
        if cls.layouts is None or c is None:
            return None
        return cls.layouts.member_at(c, disp)


def transfer(ins, state):
    """Register tokens after `ins`: THIS (ecx at a member function's entry), VT (*THIS),
    SLOT n (VT[n]), VPTR (first dword of any other object), and the CAT-13 typed tokens OBJ C,
    VTC C, SLOTC C n. Must-analysis: anything written in a way not modelled loses its token."""
    is_call, mov, writes, target = _desc(ins)
    state = dict(state)
    if is_call:
        for r in CALLER_SAVED:
            state.pop(r, None)
        rc = TypeCtx.rettype.get(target) if target is not None else None
        if rc:
            state[X.X86_REG_EAX] = ('OBJ', rc)
        return state
    new = dst = None
    if mov:
        dst, kind, src, disp = mov
        if kind == 'reg':
            new = state.get(src)
        elif kind in ('mem', 'lea'):
            bt = state.get(src)
            obj = TypeCtx.owner if bt == ('THIS',) else (bt[1] if bt and bt[0] == 'OBJ' else None)
            if kind == 'lea':
                m = TypeCtx.member(obj, disp) if obj and disp else None
                if obj and disp == 0:
                    new = ('OBJ', obj)
                elif m and m[0] == 'obj' and m[1] in TypeCtx.vclasses:
                    new = ('OBJ', m[1])
            elif bt == ('THIS',) and disp == 0:
                new = ('VT',)
            elif bt == ('VT',) and disp % 4 == 0 and disp >= 0:
                new = ('SLOT', disp // 4)
            elif bt and bt[0] == 'OBJ' and disp == 0:
                new = ('VTC', bt[1])
            elif bt and bt[0] == 'VTC' and disp % 4 == 0 and disp >= 0:
                new = ('SLOTC', bt[1], disp // 4)
            elif obj:
                m = TypeCtx.member(obj, disp)
                if m and m[0] == 'ptr' and m[1] in TypeCtx.vclasses:
                    new = ('OBJ', m[1])
                elif m and m[0] == 'obj' and m[1] in TypeCtx.vclasses:
                    new = ('VTC', m[1])   # first dword of an embedded polymorphic object
            elif disp == 0 and src != dst:
                new = ('VPTR',)
        elif kind == 'abs':
            g = TypeCtx.gtypes.get(disp)
            if g and g[0] == 'ptr':
                new = ('OBJ', g[1])
            elif g and g[0] == 'obj':
                new = ('VTC', g[1])
        elif kind in ('imm', 'leaabs'):
            g = TypeCtx.gtypes.get(disp)
            if g and g[0] == 'obj':
                new = ('OBJ', g[1])
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

    # CAT-16: every fall-through out of a body into another catalogue entry, with its decision
    write_csv(cat / 'fallthrough.csv', ['faller', 'faller_name', 'site', 'instruction', 'target', 'target_name',
                                        'target_state', 'decision', 'evidence'],
              [[hx(x['faller']), procs[x['faller']]['name'], hx(x['site']), x['insn'], hx(x['target']),
                procs[x['target']]['name'] if x['target'] in procs else r['dump'][x['target']]['name'],
                procs[x['target']]['state'] if x['target'] in procs else 'noncatalogue', x['decision'],
                x['evidence']] for x in sorted(r['fall'], key=lambda x: (x['faller'], x['site']))])

    write_csv(cat / 'indirect-calls.csv', ['function', 'site', 'instruction', 'resolution', 'detail'],
              [[hx(a), hx(s), form, res, det] for a, s, form, res, det in sorted(r['indirect'])])

    # branch / vtable targets that are not catalogue procedures (catalogue gaps, CAT-12)
    gap = collections.defaultdict(lambda: [0, set(), set(), ''])
    for (c, t, kind), n in r['edges'].items():
        if r['ecls'][(c, t, kind)].startswith('noncatalogue') and isinstance(t, int):
            gap[t][0] += n
            gap[t][1].add(kind)
            gap[t][2].add(c)
            gap[t][3] = r['ecls'][(c, t, kind)]
    write_csv(cat / 'raw' / 'cat7-noncatalogue-targets.csv', ['address', 'class', 'sites', 'kinds', 'callers'],
              [[hx(t), v[3], v[0], ';'.join(sorted(v[1])), ';'.join(hx(c) for c in sorted(v[2]))]
               for t, v in sorted(gap.items())])

    # immediates naming game code that is not a procedure (callback candidates, CAT-12b data_refs.py)
    write_csv(cat / 'raw' / 'cat7-code-immediates.csv', ['function', 'site', 'value'],
              [[hx(a), hx(s), hx(v)] for a, s, v in sorted(r['codeimm'])])

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
    st['noncatalogue_targets'] = len(gap)
    pt = collections.Counter()
    for (c, t, kind), n in r['edges'].items():
        if r['ecls'][(c, t, kind)] == 'excluded:protection-thunk':
            pt[t] += n
    st['securom_call_thunks'] = {'targets': len(pt), 'sites': sum(pt.values())}
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


# ============================================================================ gaps
def cmd_gaps(cat):
    """CAT-12 fixpoint step: append every game-code call / tail-call target that CAT-7 found
    outside the catalogue to catalog/extra-code-addresses.csv (merge_ghidra.py adds those rows).
    Exit status 3 when rows were added (the pipeline must rerun from build_catalog), 0 when the
    catalogue is closed. Other gaps (vtable slots, .HOODLUM / protection code that is not a
    SecuROM thunk) mean a rule is missing: exit 4."""
    path = cat / 'extra-code-addresses.csv'
    have = {}
    if path.exists():
        have = {int(r['address'], 16): r for r in csv.DictReader(open(path))}
    new, bad = {}, []
    for r in csv.DictReader(open(cat / 'raw' / 'cat7-noncatalogue-targets.csv')):
        a = int(r['address'], 16)
        kinds = set(r['kinds'].split(';'))
        if r['class'] == 'noncatalogue' and kinds <= {'call', 'tailcall'} and a not in have:
            new[a] = {'address': hx(a), 'rule': 'cat7-' + '+'.join(sorted(kinds)),
                      'evidence': f"{r['sites']} site(s) in catalogued bodies: {r['callers']}"}
        else:
            bad.append(r)
    if new:
        rows = sorted(list(have.values()) + list(new.values()), key=lambda x: int(x['address'], 16))
        write_csv(path, ['address', 'rule', 'evidence'], [[x['address'], x['rule'], x['evidence']] for x in rows])
        print(f'gaps: added {len(new)} call targets to {path}')
        sys.exit(3)
    if bad:
        for r in bad:
            print('gaps: not closable by a call-target row:', dict(r))
        sys.exit(4)
    print('gaps: 0 call/tail-call/vtable targets outside the catalogue')


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in ('ghidra', 'build', 'analyse', 'spotcheck', 'gaps',
                                                       'fallthrough'):
        sys.exit(__doc__)
    if sys.argv[1] == 'spotcheck':
        return cmd_spotcheck(ROOT / 'catalog', int(sys.argv[2]), int(sys.argv[3]))
    cat = Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / 'catalog'
    {'ghidra': lambda: cmd_ghidra(cat), 'build': lambda: cmd_build(cat), 'gaps': lambda: cmd_gaps(cat),
     'fallthrough': lambda: cmd_fallthrough(cat), 'analyse': cmd_analyse}[sys.argv[1]]()


if __name__ == '__main__':
    main()
