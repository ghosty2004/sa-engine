"""AI-GEN: generate sa-ai ports of the repetitive AI function shapes from the 1.0 US exe bytes.

Usage: .venv/bin/python -I tools/gen/ai_patterns.py [--check]

Reads `re/gta/1.0-US/gta_sa.exe` (never copied into the repo) and the catalogue, and writes
`crates/sa-ai/src/gen_*.rs` plus `target/ai-gen/{report,rest}.tsv`. The output is a pure function of
the exe, the catalogue and the set of addresses already ported in the repository (any `#[hook(addr`
in `crates/*/src`, and every ``/// `Name` (0xADDR)`` doc line in `crates/sa-ai/src` outside the
generated files), so two runs give byte-identical files. `--check` regenerates in memory and fails
if a generated file differs from the one on disk.

How a function is accepted (instruction by instruction, nothing is pattern-guessed):
1. Every instruction of every path of the body is decoded with capstone and executed by a small
   symbolic x86 interpreter that knows only an exact subset: 32-bit `mov`/`movzx`/`movsx`/`lea`/
   `xor`/`and`/`or`/`add`/`sub`/`inc`/`dec`/`neg`/`not`/`shl`/`shr`/`sar`/`imul`/`cdq`, byte stores
   and loads through AL/CL/DL/BL, `test`/`cmp` + `jcc`/`setcc`, `push`/`pop`, direct calls, virtual
   calls `call [vtbl + N]` on the object in ECX, direct tail jumps, forward branches, bounded
   `jmp [reg*4 + table]` switch dispatch (the table is read from the exe), `ret`/`ret N`, and
   `fld dword ptr [m]; ret` (ST0 result). PORT-AI-8 added: the x87 subset MSVC emits for float code
   (`fld`/`fild`/`fst(p)` m32, `fld qword`, register and m32 `fadd`/`fsub(r)`/`fmul`/`fdiv(r)` with their
   pop forms, `fxch`, `fchs`/`fabs`/`fsqrt`/`fsin`/`fcos`/`fpatan`, `fld1`/`fldz`, `fcom(p)`/`fcompp` +
   `fnstsw ax; test ah, M; jcc`, `call _ftol`, calls whose result is read from ST0, an ST0 result at `ret`),
   reads of AH/CH/DH/BH, 8/16-bit shifts by a constant, `sbb r, r` after `neg`/`cmp`, and stack objects
   (once a local's address escapes the locals live in a Rust `frame` array). Any other instruction, an
   unknown register or flag value reaching a use, a loop, the address of an argument slot, an
   unbalanced stack, a callee-saved register not restored, or `fs:[0]` not restored rejects the
   function.
2. Calls: the callee's stack cleanup is read from its own `ret N` instructions (all of them must
   agree; a callee ending in a tail jump inherits its target's); a callee that cleans up nothing
   gets its argument count from the caller's `add esp, N` / `pop reg` after the call (straight-line
   instructions that leave ESP alone may sit between), else from its other call sites (`cdecl_arity`:
   a consistent cleanup there, confirmed by a majority of sites or by the callee's own argument reads).
   Outstanding pushes below the arguments belong to an enclosing call (nested evaluation) and are allowed
   when the arity is proven that way. The callee's first block must not read EAX/EDX/EBX/
   ESI/EDI/EBP before writing them (no register-passed arguments). ECX is passed (thiscall) when
   the callee's first block reads it.
3. Only functions of the AI families (procedures.csv subsystem AI, or a CTask*/CEvent*/CPedGroup*/
   CDecisionMaker*/attractor class name) are emitted, filed by the families in FAMILIES (any other
   accepted body goes to `gen_bodies`); the classes other agents own (EXCLUDE), the addresses in
   `ai_patterns_exclude.txt` and every address already ported are skipped.

The generated Rust contains only semantics: offsets, constants, vtable addresses and the
original addresses of callees, never code bytes. Each generated file carries a host test that
re-reads the exe and checks a 64-bit FNV-1a hash of every accepted body (so a generated port
cannot silently drift from the bytes it was matched against).
"""
import csv
import os
import re
import subprocess
import sys

import capstone
import pefile
from capstone import x86_const as X

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
EXE = os.environ.get('SA_GTA_EXE') or os.path.expanduser('~/sa-engine/re/gta/1.0-US/gta_sa.exe')
OUT_DIR = os.path.join(ROOT, 'crates', 'sa-ai', 'src')
REPORT_DIR = os.path.join(ROOT, 'target', 'ai-gen')

pe = pefile.PE(EXE, fast_load=True)
IMG = pe.get_memory_mapped_image()
BASE = pe.OPTIONAL_HEADER.ImageBase
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
md.detail = True


def rd(a, n):
    return IMG[a - BASE:a - BASE + n]


def u32_at(a):
    return int.from_bytes(rd(a, 4), 'little')


def insn_at(a):
    return next(md.disasm(rd(a, 16), a), None)


HOODLUM = 0x1556000
M32 = 0xFFFFFFFF

# ---------------------------------------------------------------- catalogue

GR = {}
for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'functions.csv'))):
    GR[int(r['address'], 16)] = f"{r['owner']}::{r['name']}"
PROCS = {}
for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'procedures.csv'))):
    PROCS[int(r['address'], 16)] = r
RANGES = {}
for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'extents.csv'))):
    rs = []
    for part in r['ranges'].split(';'):
        if part:
            s, e = part.split('-')
            rs.append((int(s, 16), int(e, 16)))
    RANGES[int(r['address'], 16)] = rs


def catname(a):
    if a in GR:
        return GR[a]
    p = PROCS.get(a)
    if p is None:
        return None
    return p['name'] or p['ghidra_name'] or None


AI_CLASS = re.compile(r'^(CTask|CEvent|CPedGroup|CDecisionMaker|CDecision|CPedAttractor|C\w*Attractor\b|'
                      r'CTaskAllocator|CInformFriendsEventQueue|CInformGroupEventQueue|CPedIntelligence|'
                      r'CAcquaintance|CEntitySeekPos)')
# Classes other agents port (none since PORT-AI-7: AI-2's c-bug tasks, event handler responses and group
# intelligence and AI-3's vehicle/chase tasks are merged; what they ported is skipped as already ported).
EXCLUDE = re.compile(r'(?!)')


def cls_of(name):
    return name.split('::')[0] if name and '::' in name else ''


def method_of(name):
    return name.split('::')[-1] if name else ''


def neighbour_class(a, addrs_sorted, idx):
    """The classes of the nearest named neighbours (for unnamed / vt_ functions)."""
    out = []
    for step in (-1, 1):
        j = idx + step
        while 0 <= j < len(addrs_sorted):
            n = catname(addrs_sorted[j]) or ''
            c = cls_of(n)
            if c and not c.startswith('vt_') and not n.startswith('FUN_'):
                out.append(c)
                break
            j += step
    return out


def already_ported():
    """Addresses already ported anywhere (hooked or ported-unhookable), outside the generated files."""
    done = set()
    for dirpath, _, files in os.walk(os.path.join(ROOT, 'crates')):
        if '/target' in dirpath:
            continue
        for f in sorted(files):
            if not f.endswith('.rs') or (f.startswith('gen_') and dirpath.endswith(os.path.join('sa-ai', 'src'))):
                continue
            s = open(os.path.join(dirpath, f), encoding='utf-8').read()
            for m in re.finditer(r'#\[(?:sa_core::)?(?:hook|native)\(\s*addr\s*=\s*0x([0-9A-Fa-f]+)', s):
                done.add(int(m.group(1), 16))
            if os.sep + 'sa-ai' + os.sep in dirpath + os.sep:
                for m in re.finditer(r'^/// `[^`]*` \(0x([0-9A-Fa-f]+)', s, re.M):
                    done.add(int(m.group(1), 16))
    return done


# INF-20 readers: entries whose first bytes other code reads (sa-hook rejects hooks on them).
def read_entries():
    out = {}
    path = os.path.join(ROOT, 'catalog', 'code-data-reads.csv')
    for l in open(path).read().splitlines()[1:]:
        m = re.match(r'(0x[0-9a-f]+),', l)
        cells = re.findall(r'(?:^|,)("(?:[^"]*)"|[^,]*)', l)
        try:
            target, size = int(cells[6], 16), int(cells[7])
        except (ValueError, IndexError):
            continue
        out.setdefault(target, []).append((int(m.group(1), 16), size))
    return out


READS = read_entries()

# ---------------------------------------------------------------- callee facts

_pops = {}


def body_ranges(a):
    return RANGES.get(a) or []


def linear(a, e):
    out = []
    p = a
    while p < e:
        i = insn_at(p)
        if i is None:
            break
        out.append(i)
        p += i.size
    return out


# CRT functions outside the catalogue extents (their `ret` read off the bytes by hand): `rand` and `free`
# are cdecl.
CRT_POPS = {0x821B1E: 0, 0x8214BD: 0}
# CRT `rand`: called through `crate::common::rand` with the original return address (`orig::at_site`).
RAND = 0x821B1E
FTOL = 0x821B40


def pops(a, depth=0):
    """Stack bytes the function at `a` removes on return (from its own `ret N`), or None."""
    if a in CRT_POPS:
        return CRT_POPS[a]
    if a in _pops:
        return _pops[a]
    res = None
    rs = body_ranges(a)
    if not rs:
        i = insn_at(a)
        if i and i.mnemonic == 'jmp' and i.op_str.startswith('0x') and depth < 4:
            res = pops(int(i.op_str, 16), depth + 1)
        _pops[a] = res
        return res
    vals = set()
    tails = set()
    for s, e in rs:
        for i in linear(s, e):
            if i.mnemonic == 'ret':
                vals.add(int(i.op_str, 16) if i.op_str else 0)
            elif i.mnemonic == 'jmp' and i.op_str.startswith('0x'):
                t = int(i.op_str, 16)
                if not any(s2 <= t < e2 for s2, e2 in rs):
                    tails.add(t)
    if depth < 4:
        for t in sorted(tails):
            if t in RANGES or catname(t):
                v = pops(t, depth + 1)
                if v is not None:
                    vals.add(v)
    if len(vals) == 1:
        res = vals.pop()
    _pops[a] = res
    return res


REG32 = {X.X86_REG_EAX: 'eax', X.X86_REG_ECX: 'ecx', X.X86_REG_EDX: 'edx', X.X86_REG_EBX: 'ebx',
         X.X86_REG_ESP: 'esp', X.X86_REG_EBP: 'ebp', X.X86_REG_ESI: 'esi', X.X86_REG_EDI: 'edi'}
REG8 = {X.X86_REG_AL: 'eax', X.X86_REG_CL: 'ecx', X.X86_REG_DL: 'edx', X.X86_REG_BL: 'ebx'}
REG8H = {X.X86_REG_AH: 'eax', X.X86_REG_CH: 'ecx', X.X86_REG_DH: 'edx', X.X86_REG_BH: 'ebx'}
REG16 = {X.X86_REG_AX: 'eax', X.X86_REG_CX: 'ecx', X.X86_REG_DX: 'edx', X.X86_REG_BX: 'ebx',
         X.X86_REG_SI: 'esi', X.X86_REG_DI: 'edi', X.X86_REG_BP: 'ebp'}
PARENT = {}
for k, v in REG32.items():
    PARENT[k] = v
for k, v in REG8.items():
    PARENT[k] = v
for k, v in REG16.items():
    PARENT[k] = v
for k, v in {X.X86_REG_AH: 'eax', X.X86_REG_CH: 'ecx', X.X86_REG_DH: 'edx', X.X86_REG_BH: 'ebx'}.items():
    PARENT[k] = v

_reads = {}


def entry_reads(a):
    """Registers the callee's first block reads before writing (following a .HOODLUM stub)."""
    if a in _reads:
        return _reads[a]
    p = a
    i = insn_at(p)
    if i and i.mnemonic == 'jmp' and i.op_str.startswith('0x'):
        p = int(i.op_str, 16)
    written, read = set(), set()
    n = 0
    for i in md.disasm(rd(p, 200), p):
        n += 1
        if i.mnemonic == 'xor' and len(i.operands) == 2 and i.operands[0].type == X.X86_OP_REG and \
                i.operands[0].reg == i.operands[1].reg:
            written.add(PARENT.get(i.operands[0].reg))
            continue
        # `mov edi, edi` (the 2-byte alignment no-op before a loop) reads nothing that matters
        if i.mnemonic == 'mov' and len(i.operands) == 2 and i.operands[0].type == X.X86_OP_REG and \
                i.operands[1].type == X.X86_OP_REG and i.operands[0].reg == i.operands[1].reg:
            continue
        if i.mnemonic == 'push' and i.operands[0].type == X.X86_OP_REG and \
                PARENT.get(i.operands[0].reg) in ('ebx', 'esi', 'edi', 'ebp'):
            continue
        try:
            rr, ww = i.regs_access()
        except capstone.CsError:
            rr, ww = [], []
        for r in rr:
            pr = PARENT.get(r)
            if pr and pr != 'esp' and pr not in written:
                read.add(pr)
        for r in ww:
            pr = PARENT.get(r)
            if pr:
                written.add(pr)
        if i.mnemonic in ('ret', 'jmp', 'call') or i.mnemonic.startswith('j') or n > 40:
            break
    _reads[a] = read
    return read


_ecx = {}


def only_pushes_ecx(a):
    """True when the callee's only use of ECX before writing it is `push ecx` as its first instruction (MSVC's
    4-byte local slot) and nothing else in its first block reads ECX (PORT-AI-8)."""
    i = insn_at(a)
    if i is None or i.mnemonic != 'push' or i.op_str != 'ecx':
        return False
    for j in md.disasm(rd(a + i.size, 200), a + i.size):
        try:
            rr, ww = j.regs_access()
        except capstone.CsError:
            return False
        if any(PARENT.get(r) == 'ecx' for r in rr):
            return False
        if any(PARENT.get(r) == 'ecx' for r in ww):
            return True
        if j.mnemonic in ('ret', 'jmp') or j.mnemonic.startswith('j'):
            return False
        if j.mnemonic == 'call':
            # the callee may take ECX; the CRT's `rand` does not (and clobbers it: ECX is dead after it)
            return j.op_str.startswith('0x') and int(j.op_str, 16) == RAND
    return False


def ecx_first_read(a):
    """True when the first instruction touching ECX/CX/CL in a linear sweep of the callee's extent
    (stub target first) reads it: the callee takes ECX even if its first block does not read it."""
    if a in _ecx:
        return _ecx[a]
    rs = sorted(body_ranges(a))
    i = insn_at(a)
    if not rs and i and i.mnemonic == 'jmp' and i.op_str.startswith('0x'):
        rs = sorted(body_ranges(int(i.op_str, 16))) or [(int(i.op_str, 16), int(i.op_str, 16) + 200)]
    res = False
    for s, e in rs:
        hit = None
        for i in linear(s, e):
            if i.mnemonic == 'xor' and len(i.operands) == 2 and i.operands[0].type == X.X86_OP_REG and \
                    i.operands[0].reg == i.operands[1].reg:
                if PARENT.get(i.operands[0].reg) == 'ecx':
                    hit = False
                    break
                continue
            try:
                rr, ww = i.regs_access()
            except capstone.CsError:
                continue
            if any(PARENT.get(r) == 'ecx' for r in rr):
                hit = True
                break
            if any(PARENT.get(r) == 'ecx' for r in ww):
                hit = False
                break
        if hit is not None:
            res = hit
            break
    _ecx[a] = res
    return res


# ---------------------------------------------------------------- symbolic execution


class Reject(Exception):
    pass


def C(v):
    return ('c', v & M32)


def is_c(e):
    return e[0] == 'c'


def has_unknown(e):
    if e[0] in ('unk', 'in'):
        return True
    return any(has_unknown(x) for x in e[1:] if isinstance(x, tuple))


def bop(op, a, b):
    if is_c(a) and is_c(b):
        x, y = a[1], b[1]
        if op == 'add':
            return C(x + y)
        if op == 'sub':
            return C(x - y)
        if op == 'and':
            return C(x & y)
        if op == 'or':
            return C(x | y)
        if op == 'xor':
            return C(x ^ y)
        if op == 'mul':
            return C(x * y)
        if op == 'shl':
            return C(x << (y & 31))
        if op == 'shr':
            return C(x >> (y & 31))
        if op == 'sar':
            sx = x - (1 << 32) if x & 0x80000000 else x
            return C(sx >> (y & 31))
        if op == 'mulhi':
            sx = x - (1 << 32) if x & 0x80000000 else x
            sy = y - (1 << 32) if y & 0x80000000 else y
            return C((sx * sy) >> 32)
    if op in ('add', 'or', 'xor', 'sub', 'shl', 'shr', 'sar') and is_c(b) and b[1] == 0:
        return a
    if op in ('add', 'or', 'xor') and is_c(a) and a[1] == 0:
        return b
    if op == 'and' and is_c(b) and b[1] == M32:
        return a
    if op == 'mul' and is_c(b) and b[1] == 1:
        return a
    if op == 'and' and a == b:
        return a
    if op == 'and' and is_c(b) and b[1] == 0:
        return C(0)
    if op == 'or' and a[0] == 'bin' and a[1] == 'and' and is_c(a[3]) and is_c(b) and b[1] == 0 and False:
        return a
    # `(A & M1 | B) & M2` with M1 & M2 == 0 is `B & M2`: a byte/word register write over an unknown input
    # register (`mov al, [m]` while EAX is the caller's) read back through the same narrow register. Only
    # applied when A is unknown, so bodies the interpreter accepted before keep their exact text (PORT-AI-8).
    if op == 'and' and is_c(b) and a[0] == 'bin' and a[1] == 'or':
        x_, y_ = a[2], a[3]
        if x_[0] == 'bin' and x_[1] == 'and' and is_c(x_[3]) and x_[3][1] & b[1] == 0 and has_unknown(x_[2]):
            return bop('and', y_, b)
        # `(A & M1 | Y) & M2` with Y confined to bits outside M2 is `A & (M1 & M2)` (`xor al, al` after a
        # byte write over the caller's EAX)
        if x_[0] == 'bin' and x_[1] == 'and' and is_c(x_[3]) and has_unknown(x_[2]) and \
                y_[0] == 'bin' and y_[1] == 'and' and is_c(y_[3]) and y_[3][1] & b[1] == 0:
            return bop('and', x_[2], C(x_[3][1] & b[1]))
    return ('bin', op, a, b)


def depth(e):
    if not isinstance(e, tuple):
        return 0
    return 1 + max([depth(x) for x in e[1:] if isinstance(x, tuple)] or [0])


class St:
    def __init__(s):
        s.r = {'eax': ('in', 'eax'), 'ecx': ('this',), 'edx': ('in', 'edx'), 'ebx': ('in', 'ebx'),
               'esi': ('in', 'esi'), 'edi': ('in', 'edi'), 'ebp': ('in', 'ebp')}
        s.esp = 0
        s.stack = {0: ('ret',)}
        s.fs0 = ('fs0',)
        s.flags = None
        s.st0 = None
        s.pushes = []  # (offset, kind) since the last call
        s.visited = set()
        s.prologue = True
        s.argmax = -1
        s.conds = []
        s.fp = []        # x87 register stack: value numbers, ST0 last (PORT-AI-8)
        s.fpend = None   # (out list, index) of the last call (its ST0 result may be read next)
        s.fcmp = None    # value number of the last x87 compare (`fnstsw ax`)
        s.esc = False    # a local's address escaped: locals live in the Rust `frame` array from then on

    def copy(s):
        n = St()
        n.r = dict(s.r)
        n.esp = s.esp
        n.stack = dict(s.stack)
        n.fs0 = s.fs0
        n.flags = s.flags
        n.st0 = s.st0
        n.pushes = list(s.pushes)
        n.visited = set(s.visited)
        n.prologue = s.prologue
        n.argmax = s.argmax
        n.conds = list(s.conds)
        n.fp = list(s.fp)
        n.fpend = s.fpend
        n.fcmp = s.fcmp
        n.esc = s.esc
        return n


class Fn:
    def __init__(self, addr, body, ranges):
        self.addr = addr
        self.body = body
        self.ranges = ranges
        self.nv = 0
        self.calls = {}   # key -> (addr|slot, conv, nargs, kind)
        self.uses_this = False
        self.paths = 0
        self.rets = set()  # pop counts
        self.retkinds = set()
        self.insns = {}   # addr -> size, every executed instruction (for the body hash)
        self.tail = False
        self.ninsn = 0

    def new(self):
        self.nv += 1
        return self.nv

    def inside(self, t):
        return any(s <= t < e for s, e in self.ranges)


def regval(st, fn, reg):
    if reg in REG32:
        nm = REG32[reg]
        if nm == 'esp':
            raise Reject('esp as a value')
        v = st.r[nm]
        if v == ('this',):
            fn.uses_this = True
        return v
    if reg in REG8:
        v = st.r[REG8[reg]]
        if v == ('this',):
            fn.uses_this = True
        return bop('and', v, C(0xFF))
    if reg in REG16:
        v = st.r[REG16[reg]]
        if v == ('this',):
            fn.uses_this = True
        return bop('and', v, C(0xFFFF))
    if reg in REG8H:
        # AH / CH / DH / BH read (writes stay rejected) (PORT-AI-8)
        v = st.r[REG8H[reg]]
        if v == ('this',):
            fn.uses_this = True
        return bop('and', bop('shr', v, C(8)), C(0xFF))
    raise Reject('register %d' % reg)


def setreg(st, reg, val):
    if reg in REG32:
        nm = REG32[reg]
        if nm == 'esp':
            raise Reject('esp written')
        st.r[nm] = val
    elif reg in REG8:
        nm = REG8[reg]
        st.r[nm] = bop('or', bop('and', st.r[nm], C(0xFFFFFF00)), bop('and', val, C(0xFF)))
    elif reg in REG16:
        nm = REG16[reg]
        st.r[nm] = bop('or', bop('and', st.r[nm], C(0xFFFF0000)), bop('and', val, C(0xFFFF)))
    else:
        raise Reject('register write %d' % reg)


def opsize(op):
    return op.size


# ---------------------------------------------------------------- stack objects (PORT-AI-8)
# Once the address of a local escapes (`lea r, [esp + N]`, `mov r, esp`), every local slot lives in a Rust
# array `frame` (FRAME_DWORDS dwords; entry ESP + FRAME_LO is its first byte): the known slot values are
# flushed to it, and from then on every local read is a load from it, every local write (also pushes) a store
# to it, so callees that write through the escaped pointer are seen. `fb` is its 32-bit address.
FRAME_LO = -0x400
FRAME_DWORDS = -FRAME_LO // 4


def frame_addr(off):
    if off >= 0:
        raise Reject('address of an argument slot')
    if off < FRAME_LO:
        raise Reject('frame too large')
    return bop('add', ('frame',), C(off - FRAME_LO))


def escape(st, fn, out):
    if st.esc:
        return
    st.esc = True
    fn.frame = True
    for off in sorted(st.stack):
        v = st.stack[off]
        if FRAME_LO <= off < 0 and not has_unknown(v) and v[0] not in ('ret', 'fs0', 'espval'):
            out.append(('store', 4, ('frame',), off - FRAME_LO, v))


def memaddr(st, fn, op):
    """('stack', off) for esp-relative operands, ('fs', 0) for fs:[0], else ('mem', base_expr, disp)."""
    m = op.mem
    if m.segment == X.X86_REG_FS:
        if m.base == 0 and m.index == 0 and m.disp == 0:
            return ('fs', 0)
        raise Reject('fs operand')
    if m.segment not in (0, X.X86_REG_DS, X.X86_REG_SS):
        raise Reject('segment')
    if m.base == X.X86_REG_ESP:
        if m.index:
            raise Reject('indexed stack')
        return ('stack', st.esp + m.disp)
    base = None
    if m.base:
        base = regval(st, fn, m.base)
    if m.index:
        if m.index == X.X86_REG_ESP:
            raise Reject('esp index')
        iv = bop('mul', regval(st, fn, m.index), C(m.scale))
        base = iv if base is None else bop('add', base, iv)
    disp = m.disp & M32
    if base is None:
        return ('mem', None, disp)
    if base[0] == 'bin' and base[1] == 'add' and is_c(base[3]):
        disp = (disp + base[3][1]) & M32
        base = base[2]
    return ('mem', base, disp)


def stack_slot(st, fn, off, size):
    if off % 4 and size == 4:
        raise Reject('unaligned stack read')
    slot = off - off % 4
    if slot in st.stack:
        v = st.stack[slot]
    elif slot >= 4:
        k = (slot - 4) // 4
        v = ('arg', k)
        st.argmax = max(st.argmax, k)
    else:
        v = ('unk', 'stack')
    if v == ('ret',):
        raise Reject('return address read')
    if size == 4:
        return v
    sh = (off % 4) * 8
    return bop('and', bop('shr', v, C(sh)), C((1 << (size * 8)) - 1))


def load(st, fn, out, op, size=None, signed=False):
    size = size or op.size
    a = memaddr(st, fn, op)
    if a[0] == 'stack' and st.esc and a[1] < 0 and not (size == 4 and st.stack.get(a[1]) == ('fs0',)):
        # (the saved SEH link stays symbolic: no callee writes the caller's EH record)
        frame_addr(a[1])
        a = ('mem', ('frame',), a[1] - FRAME_LO)
    if a[0] == 'stack':
        v = stack_slot(st, fn, a[1], size)
        if signed and size < 4:
            v = ('sx', size, v)
        return v
    if a[0] == 'fs':
        return st.fs0
    if a[1] is not None and has_unknown(a[1]):
        raise Reject('load through an unknown pointer')
    n = fn.new()
    out.append(('load', n, size, signed, a[1], a[2]))
    return ('v', n)


def store(st, fn, out, op, val, size=None):
    size = size or op.size
    a = memaddr(st, fn, op)
    if a[0] == 'stack' and st.esc and a[1] < 0:
        off = a[1]
        frame_addr(off)
        if has_unknown(val):
            raise Reject('unknown value stored')
        out.append(('store', size, ('frame',), off - FRAME_LO, val))
        st.stack[off - off % 4] = val if size == 4 and off % 4 == 0 else ('unk', 'partial')
        return
    if a[0] == 'stack' and (size != 4 or a[1] % 4) and FRAME_LO <= a[1] < 0:
        # a byte/word local: from now on the locals live in the frame (PORT-AI-8)
        escape(st, fn, out)
        return store(st, fn, out, op, val, size)
    if a[0] == 'stack':
        off = a[1]
        if size != 4 or off % 4:
            raise Reject('partial stack write')
        if off == 0:
            raise Reject('return address written')
        st.stack[off] = val
        return
    if a[0] == 'fs':
        st.fs0 = val
        return
    if a[1] is not None and has_unknown(a[1]):
        raise Reject('store through an unknown pointer')
    if has_unknown(val):
        raise Reject('unknown value stored')
    out.append(('store', size, a[1], a[2], val))


def operand(st, fn, out, op, size=None):
    if op.type == X.X86_OP_REG:
        return regval(st, fn, op.reg)
    if op.type == X.X86_OP_IMM:
        return C(op.imm)
    if op.type == X.X86_OP_MEM:
        return load(st, fn, out, op, size)
    raise Reject('operand')


def mat(fn, out, v):
    """Binds a deep pure expression to a `let` (keeps the emitted expressions small)."""
    if depth(v) >= 3 and v[0] == 'bin':
        n = fn.new()
        out.append(('let', n, v))
        return ('v', n)
    return v


def write_op(st, fn, out, op, val):
    val = mat(fn, out, val)
    if op.type == X.X86_OP_REG:
        setreg(st, op.reg, val)
    elif op.type == X.X86_OP_MEM:
        store(st, fn, out, op, val)
    else:
        raise Reject('write operand')


def push(st, val, kind, out=None):
    st.esp -= 4
    st.stack[st.esp] = val
    st.pushes.append((st.esp, kind))
    if st.esc and out is not None and not has_unknown(val) and st.esp >= FRAME_LO and val[0] not in ('fs0', 'espval'):
        out.append(('store', 4, ('frame',), st.esp - FRAME_LO, val))


def pop(st, fn=None, out=None):
    if st.esp >= 0:
        raise Reject('pop above the frame')
    v = st.stack.get(st.esp, ('unk', 'pop'))
    if st.esc and out is not None and st.esp >= FRAME_LO and not has_unknown(v) and v != ('fs0',):
        # a callee may have written the slot through an escaped pointer
        n = fn.new()
        out.append(('load', n, 4, False, ('frame',), st.esp - FRAME_LO))
        v = ('v', n)
    st.pushes = [p for p in st.pushes if p[0] != st.esp]
    del st.stack[st.esp]
    st.esp += 4
    return v


def call_args(st, fn, n):
    """The `n` dwords at the top of the stack (first argument first)."""
    out = []
    for k in range(n):
        off = st.esp + 4 * k
        if off >= 0:
            raise Reject('argument above the frame')
        v = st.stack.get(off, ('unk', 'arg'))
        if has_unknown(v):
            raise Reject('unknown call argument')
        out.append(v)
    return out


def outstanding(st):
    return [p for p in st.pushes if p[1] == 'arg']


def after_call(st, consumed=None):
    """Clobbers ECX/EDX/flags and the slots below ESP; the call's arguments (`consumed`, their push offsets;
    every outstanding argument when None) stop being outstanding. Arguments pushed earlier for an enclosing
    call (MSVC evaluates a nested call while the outer call's later arguments are already pushed) stay."""
    st.r['ecx'] = ('unk', 'ecx')
    st.r['edx'] = ('unk', 'edx')
    st.flags = None
    for k in list(st.stack):
        if k < st.esp:
            del st.stack[k]
    if consumed is None:
        st.pushes = [p for p in st.pushes if p[1] != 'arg']
    else:
        st.pushes = [p for p in st.pushes if p[1] != 'arg' or p[0] not in consumed]


CC = {'je': 'eq', 'jne': 'ne', 'jl': 'lt', 'jge': 'ge', 'jg': 'gt', 'jle': 'le', 'jb': 'ult', 'jae': 'uge',
      'ja': 'ugt', 'jbe': 'ule', 'js': 'neg', 'jns': 'nneg', 'jp': 'par', 'jnp': 'npar'}
NEG = {'eq': 'ne', 'ne': 'eq', 'lt': 'ge', 'ge': 'lt', 'gt': 'le', 'le': 'gt', 'ult': 'uge', 'uge': 'ult',
       'ugt': 'ule', 'ule': 'ugt', 'neg': 'nneg', 'nneg': 'neg',
       # x87 compares (`fnstsw ax; test ah, M; jcc`): `jne` = any masked bit, `jp` = even parity
       'fany': 'fnone', 'fnone': 'fany', 'fpar': 'fodd', 'fodd': 'fpar'}


def cond_of(st, cc):
    f = st.flags
    if f is None:
        raise Reject('unknown flags')
    kind = f[0]
    if kind == 'ftest':
        _, n, mask = f
        op = {'ne': 'fany', 'eq': 'fnone', 'par': 'fpar', 'npar': 'fodd'}.get(cc)
        if op is None:
            raise Reject('cc %s after an x87 status test' % cc)
        return (op, ('v', n), C(mask), 0)
    if cc in ('par', 'npar'):
        raise Reject('parity after integer flags')
    if kind == 'cmp':
        _, a, b, size = f
        if cc in ('neg', 'nneg'):
            raise Reject('sign after cmp')
        cnd = (cc, a, b, size)
    elif kind == 'test':
        _, a, b, size = f
        v = bop('and', a, b)
        if cc in ('eq', 'ne'):
            cnd = (cc, v, C(0), size)
        elif cc in ('neg', 'nneg', 'lt', 'ge'):
            cnd = ('lt' if cc in ('neg', 'lt') else 'ge', v, C(0), size)
        elif cc in ('le', 'gt'):
            cnd = (cc, v, C(0), size)
        else:
            raise Reject('cc %s after test' % cc)
    elif kind == 'res':
        _, v, size = f
        if cc in ('eq', 'ne'):
            cnd = (cc, v, C(0), size)
        elif cc in ('neg', 'nneg'):
            cnd = ('lt' if cc == 'neg' else 'ge', v, C(0), size)
        else:
            raise Reject('cc %s after arithmetic' % cc)
    else:
        raise Reject('flags')
    for x in cnd[1:3]:
        if has_unknown(x):
            raise Reject('unknown value in a condition')
    return cnd


def table_bound(st, reg_expr):
    """For `jmp [reg*4 + T]`: the upper bound K proven by the path condition `reg <=u K`."""
    for c in st.conds:
        if c[0] == 'ule' and c[1] == reg_expr and is_c(c[2]):
            return c[2][1]
    return None


MAXPATHS = 96
MAXINSN = 800


def run(fn, st, pc, out):
    """Executes from pc; appends statements to out (a list); returns nothing (each path ends in ret)."""
    while True:
        if pc in st.visited:
            raise Reject('loop')
        st.visited.add(pc)
        if not fn.inside(pc):
            raise Reject('outside the body at %#x' % pc)
        i = insn_at(pc)
        if i is None:
            raise Reject('decode')
        fn.insns[pc] = i.size
        fn.ninsn += 1
        if fn.ninsn > MAXINSN:
            raise Reject('too long')
        mn = i.mnemonic
        ops = i.operands
        nxt = pc + i.size
        if i.prefix[0] or i.prefix[3]:
            raise Reject('lock/rep or address-size prefix')
        if mn == 'nop' or (mn == 'lea' and i.op_str in ('esp, [esp]', 'esp, [esp + 0]')):
            # (`lea esp, [esp]`: MSVC's alignment filler before a loop) (PORT-AI-8)
            pc = nxt
            continue
        if mn != 'push' and not (mn == 'mov' and ops[0].type == X.X86_OP_REG and ops[1].type == X.X86_OP_REG) \
                and not (mn == 'mov' and 'fs:' in i.op_str) and not (mn == 'sub' and i.op_str.startswith('esp,')):
            st.prologue = False
        if mn == 'mov':
            if ops[0].type == X.X86_OP_REG and ops[0].size not in (1, 2, 4):
                raise Reject('mov size')
            if ops[0].type == X.X86_OP_REG and ops[0].reg in (X.X86_REG_AH, X.X86_REG_CH, X.X86_REG_DH, X.X86_REG_BH):
                raise Reject('high byte')
            if ops[1].type == X.X86_OP_REG and ops[1].reg == X.X86_REG_ESP:
                if ops[0].type == X.X86_OP_MEM and memaddr(st, fn, ops[0])[0] == 'fs':
                    st.fs0 = ('espval', st.esp)
                    pc = nxt
                    continue
                if ops[0].type == X.X86_OP_REG and ops[0].size == 4:
                    v = frame_addr(st.esp)
                    escape(st, fn, out)
                    setreg(st, ops[0].reg, v)
                    pc = nxt
                    continue
                raise Reject('esp copied')
            v = operand(st, fn, out, ops[1])
            write_op(st, fn, out, ops[0], v)
        elif mn in ('movzx', 'movsx'):
            src = ops[1]
            if src.type == X.X86_OP_MEM:
                v = load(st, fn, out, src, src.size, mn == 'movsx')
            else:
                v = regval(st, fn, src.reg)
                if mn == 'movsx':
                    v = ('sx', src.size, v)
            if mn == 'movzx' and src.type == X.X86_OP_MEM and src.size < 4:
                pass
            setreg(st, ops[0].reg, v if ops[0].size == 4 else bop('and', v, C((1 << (8 * ops[0].size)) - 1)))
        elif mn == 'lea' and memaddr(st, fn, ops[1])[0] == 'stack':
            a = memaddr(st, fn, ops[1])
            v = frame_addr(a[1])
            escape(st, fn, out)
            setreg(st, ops[0].reg, v)
        elif mn == 'lea':
            a = memaddr(st, fn, ops[1])
            if a[0] != 'mem':
                raise Reject('lea of a stack address')
            setreg(st, ops[0].reg, C(a[2]) if a[1] is None else bop('add', a[1], C(a[2])))
        elif mn in ('xor', 'and', 'or', 'add', 'sub'):
            if mn == 'sub' and ops[0].type == X.X86_OP_REG and ops[0].reg == X.X86_REG_ESP:
                if ops[1].type != X.X86_OP_IMM or ops[1].imm % 4:
                    raise Reject('sub esp')
                for _ in range(ops[1].imm // 4):
                    push(st, ('unk', 'local'), 'local')
                pc = nxt
                continue
            if mn == 'add' and ops[0].type == X.X86_OP_REG and ops[0].reg == X.X86_REG_ESP:
                if ops[1].type != X.X86_OP_IMM or ops[1].imm % 4:
                    raise Reject('add esp')
                for _ in range(ops[1].imm // 4):
                    pop(st)
                st.flags = None
                pc = nxt
                continue
            size = ops[0].size
            if mn == 'xor' and ops[0].type == X.X86_OP_REG and ops[1].type == X.X86_OP_REG and ops[0].reg == ops[1].reg:
                v = C(0)
            else:
                a = operand(st, fn, out, ops[0])
                b = operand(st, fn, out, ops[1])
                if size < 4 and ops[1].type == X.X86_OP_IMM:
                    b = C(b[1] & ((1 << (8 * size)) - 1))
                v = bop(mn, a, b)
                if size < 4:
                    v = bop('and', v, C((1 << (8 * size)) - 1))
            write_op(st, fn, out, ops[0], v)
            st.flags = ('res', v, size) if mn in ('xor', 'and', 'or') or size == 4 else None
            if mn in ('add', 'sub') and size < 4:
                st.flags = ('res', v, size)
        elif mn == 'sbb' and ops[0].type == X.X86_OP_REG and ops[1].type == X.X86_OP_REG and \
                ops[0].reg == ops[1].reg and (ops[0].reg in REG32 or ops[0].reg in REG8):
            # `sbb r, r` = -CF: after `neg x` CF = (x != 0), after `cmp a, b` CF = (a <u b) (PORT-AI-8)
            f = st.flags
            if f is None:
                raise Reject('sbb without flags')
            if f[0] == 'negcf':
                cnd = ('ne', f[1], C(0), f[2])
            elif f[0] == 'cmp':
                cnd = ('ult', f[1], f[2], f[3])
            else:
                raise Reject('sbb after %s' % f[0])
            for x_ in cnd[1:3]:
                if has_unknown(x_):
                    raise Reject('unknown value in a condition')
            setreg(st, ops[0].reg, mat(fn, out, bop('sub', C(0), ('cond', cnd))))
            st.flags = None
        elif mn in ('inc', 'dec', 'neg', 'not'):
            size = ops[0].size
            a = operand(st, fn, out, ops[0])
            if mn == 'inc':
                v = bop('add', a, C(1))
            elif mn == 'dec':
                v = bop('sub', a, C(1))
            elif mn == 'neg':
                v = bop('sub', C(0), a)
            else:
                v = bop('xor', a, C(M32))
            if size < 4:
                v = bop('and', v, C((1 << (8 * size)) - 1))
            write_op(st, fn, out, ops[0], v)
            if mn in ('inc', 'dec'):
                st.flags = ('res', v, size)
            elif mn == 'neg':
                st.flags = ('negcf', a, size)
        elif mn in ('shl', 'shr', 'sar', 'sal') and ops[0].size in (1, 2) and \
                (len(ops) == 1 or ops[1].type == X.X86_OP_IMM):
            # 8/16-bit shifts by a constant (PORT-AI-8): the narrow value, shifted, written back narrow
            size = ops[0].size
            k = 1 if len(ops) == 1 else ops[1].imm & 31
            a = operand(st, fn, out, ops[0])
            mask = (1 << (8 * size)) - 1
            if mn == 'sar':
                v = bop('sar', ('sx', size, bop('and', a, C(mask))), C(k))
            else:
                v = bop('shl' if mn == 'sal' else mn, bop('and', a, C(mask)), C(k))
            v = bop('and', v, C(mask))
            write_op(st, fn, out, ops[0], v)
            st.flags = ('res', v, size) if k else st.flags
        elif mn in ('shl', 'shr', 'sar', 'sal'):
            if ops[0].size != 4:
                raise Reject('narrow shift')
            a = operand(st, fn, out, ops[0])
            if len(ops) == 1:
                b = C(1)
            elif ops[1].type == X.X86_OP_IMM:
                b = C(ops[1].imm)
            else:
                raise Reject('shift by register')
            v = bop('shl' if mn == 'sal' else mn, a, b)
            write_op(st, fn, out, ops[0], v)
            st.flags = ('res', v, 4) if b[1] & 31 else st.flags
        elif mn == 'imul':
            if len(ops) == 1:
                b = operand(st, fn, out, ops[0])
                if ops[0].size != 4:
                    raise Reject('narrow imul')
                a = st.r['eax']
                st.r['edx'] = mat(fn, out, bop('mulhi', a, b))
                st.r['eax'] = mat(fn, out, bop('mul', a, b))
            elif len(ops) == 2:
                if ops[0].size != 4:
                    raise Reject('narrow imul')
                setreg(st, ops[0].reg, bop('mul', operand(st, fn, out, ops[0]), operand(st, fn, out, ops[1])))
            else:
                if ops[0].size != 4:
                    raise Reject('narrow imul')
                setreg(st, ops[0].reg, bop('mul', operand(st, fn, out, ops[1]), C(ops[2].imm)))
            st.flags = None
        elif mn == 'cdq':
            st.r['edx'] = mat(fn, out, bop('sar', st.r['eax'], C(31)))
        elif mn in ('test', 'cmp') and not (ops[0].type == X.X86_OP_REG and ops[0].reg == X.X86_REG_AH and
                                             st.r['eax'][0] == 'unk' and isinstance(st.r['eax'][1], tuple)):
            size = ops[0].size
            a = operand(st, fn, out, ops[0])
            b = operand(st, fn, out, ops[1])
            if ops[1].type == X.X86_OP_IMM and size < 4:
                b = C(b[1] & ((1 << (8 * size)) - 1))
            st.flags = (mn, a, b, size)
        elif mn.startswith('set'):
            cc = CC.get('j' + mn[3:])
            if cc is None:
                raise Reject(mn)
            if ops[0].type != X.X86_OP_REG or ops[0].reg not in REG8:
                raise Reject('setcc target')
            setreg(st, ops[0].reg, ('cond', cond_of(st, cc)))
        elif mn == 'push':
            o = ops[0]
            if o.type == X.X86_OP_REG and o.reg == X.X86_REG_ESP:
                raise Reject('push esp')
            v = operand(st, fn, out, o, 4)
            kind = 'arg'
            if st.prologue:
                if o.type == X.X86_OP_REG and REG32.get(o.reg) in ('esi', 'edi', 'ebx', 'ebp') and v == ('in', REG32[o.reg]):
                    kind = 'save'
                elif o.type == X.X86_OP_REG and v == st.fs0:
                    kind = 'seh'
                elif pc in fn.seh_pcs:
                    kind = 'seh'
                elif o.type == X.X86_OP_REG and REG32.get(o.reg) == 'ecx':
                    kind = 'local'
            elif o.type == X.X86_OP_REG and REG32.get(o.reg) in ('esi', 'edi', 'ebx', 'ebp') and v == ('in', REG32[o.reg]):
                kind = 'save'
            push(st, v, kind, out)
        elif mn == 'pop':
            v = pop(st, fn, out)
            o = ops[0]
            if o.type != X.X86_OP_REG:
                raise Reject('pop to memory')
            setreg(st, o.reg, v)
        elif mn == 'call':
            o = ops[0]
            if o.type == X.X86_OP_IMM and o.imm & M32 == FTOL:
                # `_ftol`: ST0 (popped) -> EDX:EAX; only EAX is modelled (PORT-AI-8)
                if not st.fp:
                    raise Reject('_ftol without ST0')
                n = fn.new()
                out.append(('ftol', n, st.fp.pop()))
                st.r['eax'] = ('v', n)
                st.r['ecx'] = ('unk', 'ecx')
                st.r['edx'] = ('unk', 'edx')
                st.flags = None
                st.fpend = None
                fn.x87 = True
                pc = nxt
                continue
            if st.fp:
                raise Reject('x87 stack not empty at a call')
            st.fpend = None
            if o.type == X.X86_OP_IMM:
                tgt = o.imm & M32
                if fn.inside(tgt):
                    raise Reject('call into the body')
                do_call(fn, st, out, ('d', tgt), nxt)
                if out and out[-1][0] == 'call' and out[-1][2] == ('d', tgt):
                    st.fpend = (out, len(out) - 1)
                pc = nxt
                continue
            elif o.type == X.X86_OP_MEM and o.mem.base and not o.mem.index and o.mem.segment == 0:
                vt = regval(st, fn, o.mem.base)
                obj = st.r['ecx']
                if not (vt[0] == 'v' and any(s[0] == 'load' and s[1] == vt[1] and s[2] == 4 and s[4] == obj and s[5] == 0
                                             for s in out)):
                    raise Reject('indirect call not on the vtable of ECX')
                do_call(fn, st, out, ('vs', o.mem.disp), nxt, obj=obj, vload=vt[1])
            else:
                raise Reject('indirect call')
            # cdecl cleanup right after the call
        elif mn == 'jmp':
            o = ops[0]
            if o.type == X.X86_OP_IMM:
                tgt = o.imm & M32
                if fn.inside(tgt):
                    pc = tgt
                    continue
                tail(fn, st, out, tgt)
                return
            if o.type == X.X86_OP_MEM and o.mem.index and o.mem.scale == 4 and not o.mem.base and o.mem.segment == 0:
                switch(fn, st, out, o, pc)
                return
            if o.type == X.X86_OP_MEM and o.mem.base and not o.mem.index and o.mem.segment == 0:
                vtail(fn, st, out, o)
                return
            raise Reject('indirect jump')
        elif mn in CC:
            cnd = cond_of(st, CC[mn])
            tgt = ops[0].imm & M32
            fn.paths += 1
            if fn.paths > MAXPATHS:
                raise Reject('too many paths')
            sa, sb = st.copy(), st
            sa.conds = st.conds + [cnd]
            sb.conds = st.conds + [(NEG[cnd[0]],) + cnd[1:]]
            a_out, b_out = [], []
            run(fn, sa, tgt, a_out)
            run(fn, sb, nxt, b_out)
            out.append(('if', cnd, a_out, b_out))
            return
        elif mn == 'ret':
            n = ops[0].imm if ops else 0
            finish(fn, st, out, n)
            return
        elif mn == 'fld' and not st.fp and ops and ops[0].type == X.X86_OP_MEM and ops[0].size == 4 and \
                insn_at(nxt).mnemonic == 'ret':
            st.st0 = load(st, fn, out, ops[0], 4)
            pc = nxt
            continue
        elif mn == 'test' and ops[0].type == X.X86_OP_REG and ops[0].reg == X.X86_REG_AH and \
                ops[1].type == X.X86_OP_IMM and st.r['eax'][0] == 'unk' and isinstance(st.r['eax'][1], tuple) and \
                st.r['eax'][1][0] == 'fsw':
            st.flags = ('ftest', st.r['eax'][1][1], ops[1].imm & 0xFF)
        elif mn.startswith('f') or mn == 'wait':
            fpu(fn, st, out, i)
        else:
            raise Reject('insn %s' % mn)
        pc = nxt


# ---------------------------------------------------------------- x87 (PORT-AI-8)
# The register stack (`St.fp`, ST0 last) holds value numbers bound by `let vN` statements, in instruction
# order, so every memory read happens where the original reads it:
#   ('fld', n, kind, src)          kind 'f32' / 'f64' / 'i32' (src = ('mem', base, disp)), 'f32' / 'i32b' with
#                                  src = ('bits', u32 expression) for stack slots
#   ('fop', n, op, a, b)           vN = va.op(vb) (register operand), op in fadd / fsub / fmul / fdiv
#   ('fopm', n, op, a, src)        vN = va.op_m(f32 operand), op in fadd / fsub / fsubr / fmul / fdiv / fdivr
#   ('fun', n, op, a)              fchs / fabs / fsqrt / fsin / fcos, or the constants one / zero (a = None)
#   ('fpatan', n, y, x)            vN = vy.fpatan(vx)
#   ('fcmp', n, a, b), ('fcmpm', n, a, src)   vN: Cmp = va.fcom(..), src ('f64', mem) for `fcom qword`
#   ('fst', a, mem)                f32 store; ('fbits', n, a): vN = the f32 bits (a stack slot value)
#   ('ftol', n, a)                 vN = ftol32(va) as u32 (`call _ftol`)
# `fnstsw ax; test ah, M; jcc` becomes a condition on the compare (`test_any` / `test_parity`). A call whose
# result is read from ST0 (an x87 instruction needs ST0 with an empty stack right after the call) becomes a call
# on target ('df', addr), declared `-> f64` and loaded with `X87::from_f64_exact`.
FUN_OPS = {0xE0: 'fchs', 0xE1: 'fabs', 0xFA: 'fsqrt', 0xFE: 'fsin', 0xFF: 'fcos', 0xE8: 'one', 0xEE: 'zero'}
X87_TRANSCENDENTAL = {'fsqrt', 'fsin', 'fcos'}
X87_ARITH = {0: 'fadd', 1: 'fmul', 4: 'fsub', 5: 'fsubr', 6: 'fdiv', 7: 'fdivr'}


def fp_get(fn, st, out, i):
    """ST(i)'s value number; ST0 of an empty stack is the ST0 result of the call just before."""
    if i < len(st.fp):
        return st.fp[-1 - i]
    if i == 0 and not st.fp and st.fpend is not None:
        lst, idx = st.fpend
        if lst is not out or any(x[0] not in ('load', 'let') for x in out[idx + 1:]):
            raise Reject('ST0 result across statements')
        c = lst[idx]
        n = fn.new()
        lst[idx] = ('call', n, ('df', c[2][1])) + tuple(c[3:])
        fn.calls.pop((c[2], c[3], len(c[5])), None)
        fn.calls[(('df', c[2][1]), c[3], len(c[5]))] = True
        if st.r['eax'] == ('v', c[1]):
            st.r['eax'] = ('unk', 'eax')
        st.fpend = None
        st.fp.append(n)
        # the callee's ST0 may carry a 64-bit significand (an FPATAN result) that the f64 round trip of
        # `orig!` rounds: flag it for the verifier
        fn.transcendental = True
        return n
    raise Reject('x87 stack underflow')


def fp_set(st, i, n):
    st.fp[-1 - i] = n


def fpu(fn, st, out, i):
    b = bytes(i.bytes)
    op0 = b[0]
    if op0 == 0x9B:
        return
    if op0 < 0xD8 or op0 > 0xDF or len(b) < 2:
        raise Reject('x87 %s' % i.mnemonic)
    reg = b[1] >= 0xC0
    sub = (b[1] >> 3) & 7
    k = b[1] & 7
    fn.x87 = True

    def new(head, *rest):
        n = fn.new()
        out.append((head, n) + rest)
        return n

    def bin_reg(op, a, bb):
        # the value `a op bb`; fsubr / fdivr mean bb - a and bb / a
        if op == 'fsubr':
            return new('fop', 'fsub', bb, a)
        if op == 'fdivr':
            return new('fop', 'fdiv', bb, a)
        return new('fop', op, a, bb)

    def src_of(o):
        a = memaddr(st, fn, o)
        if a[0] == 'stack' and st.esc and a[1] < 0:
            frame_addr(a[1])
            return ('mem', ('frame',), a[1] - FRAME_LO)
        if a[0] == 'stack':
            v = stack_slot(st, fn, a[1], 4)
            if has_unknown(v):
                raise Reject('unknown x87 stack operand')
            return ('bits', v)
        if a[0] != 'mem':
            raise Reject('x87 memory operand')
        if a[1] is not None and has_unknown(a[1]):
            raise Reject('x87 operand through an unknown pointer')
        return a

    def compare(n, pop):
        st.fcmp = n
        if pop:
            st.fp.pop()

    if reg:
        if op0 == 0xD9:
            if sub == 0:      # fld st(i)
                st.fp.append(fp_get(fn, st, out, k))
            elif sub == 1:    # fxch st(i)
                x0, xk = fp_get(fn, st, out, 0), fp_get(fn, st, out, k)
                fp_set(st, 0, xk)
                fp_set(st, k, x0)
            elif b[1] in FUN_OPS:
                o = FUN_OPS[b[1]]
                if o in ('one', 'zero'):
                    st.fp.append(new('fun', o, None))
                else:
                    fp_set(st, 0, new('fun', o, fp_get(fn, st, out, 0)))
                    if o in X87_TRANSCENDENTAL:
                        fn.transcendental = True
            elif b[1] == 0xF3:  # fpatan: ST1 = atan2(ST1, ST0), pop
                x0, y = fp_get(fn, st, out, 0), fp_get(fn, st, out, 1)
                st.fp.pop()
                fp_set(st, 0, new('fpatan', y, x0))
                fn.transcendental = True
            else:
                raise Reject('x87 %s' % i.mnemonic)
            return
        if op0 == 0xDE and b[1] == 0xD9:  # fcompp
            a, bb = fp_get(fn, st, out, 0), fp_get(fn, st, out, 1)
            compare(new('fcmp', a, bb), True)
            st.fp.pop()
            return
        if op0 in (0xD8, 0xDC, 0xDE):
            if sub in (2, 3):
                if op0 != 0xD8:
                    raise Reject('x87 %s' % i.mnemonic)
                compare(new('fcmp', fp_get(fn, st, out, 0), fp_get(fn, st, out, k)), sub == 3)
                return
            op = X87_ARITH[sub]
            if op0 == 0xD8:   # ST0 = ST0 op ST(i)
                fp_set(st, 0, bin_reg(op, fp_get(fn, st, out, 0), fp_get(fn, st, out, k)))
                return
            # DC / DE: ST(i) = ST(i) op ST0; the sub/div names are reversed in these encodings
            # (DC E8+i FSUB: ST(i) - ST0, DC E0+i FSUBR: ST0 - ST(i))
            rop = {'fadd': 'fadd', 'fmul': 'fmul', 'fsub': 'fsubr', 'fsubr': 'fsub', 'fdiv': 'fdivr',
                   'fdivr': 'fdiv'}[op]
            fp_set(st, k, bin_reg(rop, fp_get(fn, st, out, k), fp_get(fn, st, out, 0)))
            if op0 == 0xDE:
                st.fp.pop()
            return
        if op0 == 0xDD and sub in (2, 3):  # fst / fstp st(i)
            fp_set(st, k, fp_get(fn, st, out, 0))
            if sub == 3:
                st.fp.pop()
            return
        if op0 == 0xDF and b[1] == 0xE0:  # fnstsw ax
            if getattr(st, 'fcmp', None) is None:
                raise Reject('fnstsw without a compare')
            st.r['eax'] = ('unk', ('fsw', st.fcmp))
            return
        raise Reject('x87 %s' % i.mnemonic)
    o = i.operands[0] if i.operands else None
    if o is None or o.type != X.X86_OP_MEM:
        raise Reject('x87 %s' % i.mnemonic)
    if op0 == 0xD8:      # ST0 op m32 / fcom m32
        m = src_of(o)
        if sub in (2, 3):
            compare(new('fcmpm', fp_get(fn, st, out, 0), m), sub == 3)
            return
        fp_set(st, 0, new('fopm', X87_ARITH[sub], fp_get(fn, st, out, 0), m))
        return
    if op0 == 0xD9 and sub == 0:   # fld m32
        st.fp.append(new('fld', 'f32', src_of(o)))
        return
    if op0 == 0xD9 and sub in (2, 3):  # fst / fstp m32
        v = fp_get(fn, st, out, 0)
        a = memaddr(st, fn, o)
        if a[0] == 'stack':
            if a[1] % 4 or a[1] == 0:
                raise Reject('x87 store to the return slot or unaligned')
            st.stack[a[1]] = ('v', new('fbits', v))
            if st.esc and a[1] < 0:
                frame_addr(a[1])
                out.append(('fst', v, ('mem', ('frame',), a[1] - FRAME_LO)))
        elif a[0] == 'mem':
            if a[1] is not None and has_unknown(a[1]):
                raise Reject('x87 store through an unknown pointer')
            out.append(('fst', v, a))
        else:
            raise Reject('x87 store')
        if sub == 3:
            st.fp.pop()
        return
    if op0 == 0xDD and sub == 0:   # fld m64
        m = src_of(o)
        if m[0] != 'mem':
            raise Reject('fld qword from the stack')
        st.fp.append(new('fld', 'f64', m))
        return
    if op0 == 0xDB and sub == 0:   # fild m32
        m = src_of(o)
        st.fp.append(new('fld', 'i32' if m[0] == 'mem' else 'i32b', m))
        return
    if op0 == 0xDC and sub in (2, 3):   # fcom qword
        m = src_of(o)
        if m[0] != 'mem':
            raise Reject('fcom qword on the stack')
        compare(new('fcmpm', fp_get(fn, st, out, 0), ('f64', m)), sub == 3)
        return
    raise Reject('x87 %s' % i.mnemonic)

ARITY = None


def cdecl_arity(a):
    """The argument count of a caller-cleaned callee from its call sites in every catalogued body: the
    `add esp, N` right after `call a`. Known only when every immediate cleanup has the same N and they outnumber
    the sites without one (a callee taking nothing whose sites sometimes carry a merged cleanup of an earlier
    call shows the reverse). PORT-AI-8."""
    global ARITY
    if ARITY is None:
        seen = {}
        for rs in RANGES.values():
            for s_, e_ in rs:
                prev = None
                for (_, _, mn, op) in md.disasm_lite(rd(s_, e_ - s_), s_):
                    if prev is not None:
                        k = int(op[5:], 16) if mn == 'add' and op.startswith('esp, ') else None
                        seen.setdefault(prev, []).append(k)
                        prev = None
                    if mn == 'call' and op.startswith('0x'):
                        prev = int(op, 16)
        ARITY = {}
        for t, ks in seen.items():
            imm = [k for k in ks if k is not None]
            if not imm or len(set(imm)) != 1 or imm[0] % 4:
                continue
            n = imm[0] // 4
            # a majority of immediate cleanups, or the callee's own reads reaching exactly its last argument
            if (len(imm) >= 2 and len(imm) > len(ks) - len(imm)) or arg_reads(t) == n:
                ARITY[t] = n
        for t, ks in seen.items():
            if len(ks) >= 2 and all(k is None for k in ks) and t in RANGES and pops(t) == 0 and \
                    arg_reads(t) == 0 and not entry_reads(t) & {'eax', 'edx', 'ebx', 'esi', 'edi', 'ebp'}:
                ZERO_ARITY.add(t)
    return ARITY.get(a, 0)


ZERO_ARITY = set()


def arg_reads(a):
    """How many stack arguments the body at `a` reads before its first `ret`, in address order (ESP tracked
    through push / pop / add / sub; 0 when unsure)."""
    depth = 0
    top = 0
    for s_, e_ in sorted(RANGES.get(a, [])):
        for i in md.disasm(rd(s_, e_ - s_), s_):
            mn = i.mnemonic
            for o in i.operands:
                if o.type == X.X86_OP_MEM and o.mem.base == X.X86_REG_ESP and not o.mem.index:
                    slot = o.mem.disp - depth
                    if slot >= 4 and slot % 4 == 0:
                        top = max(top, slot // 4)
            if mn == 'push':
                depth += 4
            elif mn == 'pop':
                depth -= 4
            elif mn in ('sub', 'add') and i.op_str.startswith('esp, ') and i.operands[1].type == X.X86_OP_IMM:
                depth += i.operands[1].imm if mn == 'sub' else -i.operands[1].imm
            elif mn in ('ret', 'jmp') or (mn != 'push' and 'esp' in i.op_str.split(',')[0] and mn not in ('sub', 'add')
                                          and not i.op_str.startswith('dword ptr [esp') and not i.op_str.startswith('byte ptr [esp')):
                return top if mn in ('ret', 'jmp') else 0
            if depth < 0:
                return 0
        return top
    return top


def do_call(fn, st, out, target, nxt, obj=None, vload=None):
    if target[0] == 'd':
        a = target[1]
        p = pops(a)
        regs = entry_reads(a)
        bad = regs & {'eax', 'edx', 'ebx', 'esi', 'edi', 'ebp'}
        if bad:
            raise Reject('callee %#x reads %s' % (a, ','.join(sorted(bad))))
        uses_ecx = 'ecx' in regs or ecx_first_read(a)
    else:
        p = None
        uses_ecx = True
    cleanup = None
    q = nxt
    for _ in range(6):
        nxt_i = insn_at(q)
        if nxt_i is None:
            break
        if nxt_i.mnemonic == 'add' and nxt_i.op_str.startswith('esp, '):
            cleanup = int(nxt_i.op_str[5:], 16)
            break
        if nxt_i.mnemonic == 'pop' and nxt_i.op_str == 'ecx':
            cleanup = 4
            break
        # only straight-line instructions that leave ESP and the stack alone may sit between a call and its
        # cleanup (register moves and arithmetic, loads and stores through other registers, x87 ops)
        writes_esp = bool(nxt_i.operands) and nxt_i.operands[0].type == X.X86_OP_REG and \
            nxt_i.operands[0].reg == X.X86_REG_ESP
        if writes_esp or 'ebp' in nxt_i.op_str or nxt_i.mnemonic in (
                'call', 'jmp', 'ret', 'push', 'pop', 'leave', 'pushfd', 'popfd') or nxt_i.mnemonic.startswith('j') or \
                nxt_i.prefix[0] or nxt_i.prefix[3]:
            break
        q += nxt_i.size
    outs = outstanding(st)
    proven = False
    cdecl_arity(0)
    if cleanup and cleanup > 4 * len(outs):
        # more than every outstanding push: the frame's own cleanup (an epilogue), not this call's
        cleanup = None
    if target[0] == 'vs':
        nargs = len(outs)
        conv = 'thiscall'
    elif p is None:
        raise Reject('callee %#x stack cleanup unknown' % target[1])
    elif p > 0:
        nargs = p // 4
        # pass ECX whenever it holds a known value: a callee may read it after its first block
        conv = 'thiscall' if uses_ecx or not has_unknown(st.r['ecx']) else 'stdcall'
    else:
        if cleanup:
            nargs = cleanup // 4
            conv = 'cdecl'
            if uses_ecx and not only_pushes_ecx(target[1]):
                raise Reject('caller-cleaned callee reads ECX')
            uses_ecx = False
        elif target[0] == 'd' and outs and cdecl_arity(target[1]):
            # the cleanup is merged into a later `add esp`: the arity of the callee's other call sites
            nargs = cdecl_arity(target[1])
            cleanup = 4 * nargs
            proven = True
            conv = 'cdecl'
            if uses_ecx and not only_pushes_ecx(target[1]):
                raise Reject('caller-cleaned callee reads ECX')
            uses_ecx = False
        elif target[0] == 'd' and outs and target[1] in ZERO_ARITY:
            # a callee that takes nothing (no site cleans up after it, it reads no argument slot) called while
            # an enclosing call's arguments are pushed
            nargs = 0
            proven = True
            conv = 'thiscall' if uses_ecx or not has_unknown(st.r['ecx']) else 'cdecl'
        else:
            nargs = 0
            conv = 'thiscall' if uses_ecx or not has_unknown(st.r['ecx']) else 'cdecl'
    if len(outs) < nargs:
        raise Reject('outstanding pushes %d != %d arguments' % (len(outs), nargs))
    # nesting is accepted only when the callee's argument count is proven (its own `ret N`, or the caller's
    # cleanup right after the call); a caller-cleaned callee without an immediate cleanup may have its cleanup
    # merged with a later call's
    if len(outs) > nargs and not (target[0] == 'd' and (p > 0 or cleanup or proven)):
        raise Reject('outstanding pushes %d != %d arguments' % (len(outs), nargs))
    # the arguments must be the outstanding pushes at the top of the stack; outstanding pushes below them
    # belong to an enclosing call and must all lie above them (nested argument evaluation)
    top = sorted(o[0] for o in outs)
    if nargs and top[:nargs] != [st.esp + 4 * k for k in range(nargs)]:
        raise Reject('arguments interleaved')
    consumed = set(top[:nargs])
    args = call_args(st, fn, nargs)
    this = None
    if conv == 'thiscall':
        this = st.r['ecx'] if obj is None else obj
        if has_unknown(this):
            raise Reject('unknown ECX at a call')
        if this == ('this',):
            fn.uses_this = True
    n = fn.new()
    key = (target, conv, nargs)
    if target == ('d', RAND):
        if nargs:
            raise Reject('rand with arguments')
        # the original return address names the draw (sa-hook's determinism hash keys on it)
        vload = ('site', nxt)
    else:
        fn.calls[key] = True
    out.append(('call', n, target, conv, this, args, vload))
    if conv in ('thiscall', 'stdcall'):
        for _ in range(nargs):
            pop(st)
    else:
        # a caller-cleaned callee may change its argument slots
        for k in range(nargs):
            st.stack[st.esp + 4 * k] = ('unk', 'argslot')
    st.r['eax'] = ('v', n)
    after_call(st, consumed)


def tail(fn, st, out, tgt):
    if st.fp:
        raise Reject('x87 stack at a tail jump')
    if st.esp != 0:
        raise Reject('tail jump with an unbalanced stack')
    check_restored(st)
    p = pops(tgt)
    if p is None:
        raise Reject('tail callee cleanup unknown')
    regs = entry_reads(tgt)
    bad = regs & {'eax', 'edx', 'ebx', 'esi', 'edi', 'ebp'}
    if bad:
        raise Reject('tail callee reads %s' % ','.join(sorted(bad)))
    if p == 0:
        # a caller-cleaned callee: it sees our caller's arguments; only allowed with no argument
        # slot read or written by us and a callee that reads none (unknowable): reject unless
        # the callee's own body never reads its stack arguments (checked by its pop count 0 and
        # thiscall use of ECX only when it is a known thiscall with zero arguments)
        if any(k > 0 for k in st.stack):
            raise Reject('tail call to a caller-cleaned callee after an argument write')
        if 'ecx' not in regs:
            raise Reject('tail call to a caller-cleaned callee that does not read ECX')
    nargs = p // 4
    if ecx_first_read(tgt):
        regs = regs | {'ecx'}
    conv = 'thiscall' if 'ecx' in regs or not has_unknown(st.r['ecx']) else 'stdcall'
    args = []
    for k in range(nargs):
        v = st.stack.get(4 + 4 * k, ('arg', k))
        if v[0] == 'arg':
            st.argmax = max(st.argmax, v[1])
        args.append(v)
    st.argmax = max(st.argmax, nargs - 1)
    this = st.r['ecx'] if conv == 'thiscall' else None
    if this is not None and has_unknown(this):
        raise Reject('unknown ECX at a tail call')
    if this == ('this',):
        fn.uses_this = True
    n = fn.new()
    fn.calls[(('d', tgt), conv, nargs)] = True
    out.append(('call', n, ('d', tgt), conv, this, args, None))
    fn.rets.add(p)
    fn.retkinds.add('u32')
    fn.argmax = max(getattr(fn, 'argmax', -1), st.argmax)
    out.append(('ret', ('v', n), 'u32'))
    fn.tail = True


# Stack arguments of the virtuals a body may tail-jump to (`jmp [vtbl + N]` on the object in ECX): the
# callee is the same method of another object (a sub-task's `MakeAbortable`, ...), so it takes the
# caller's own arguments, which it receives unchanged.
VTAIL_ARGS = {'MakeAbortable': 3, 'ProcessPed': 1, 'ControlSubTask': 1, 'CreateFirstSubTask': 1,
              'CreateNextSubTask': 1, 'SetPedPosition': 1}


def vtail(fn, st, out, o):
    if st.fp:
        raise Reject('x87 stack at a tail jump')
    nargs = VTAIL_ARGS.get(method_of(fn.name or ''))
    if nargs is None:
        raise Reject('virtual tail jump of an unknown method')
    if st.esp != 0:
        raise Reject('tail jump with an unbalanced stack')
    check_restored(st)
    if any(k > 0 for k in st.stack):
        raise Reject('virtual tail jump after an argument write')
    vt = regval(st, fn, o.mem.base)
    obj = st.r['ecx']
    if not (vt[0] == 'v' and any(x[0] == 'load' and x[1] == vt[1] and x[2] == 4 and x[4] == obj and x[5] == 0
                                 for x in out)):
        raise Reject('virtual tail jump not on the vtable of ECX')
    if has_unknown(obj):
        raise Reject('unknown ECX at a tail call')
    if obj == ('this',):
        fn.uses_this = True
    args = [('arg', k) for k in range(nargs)]
    st.argmax = max(st.argmax, nargs - 1)
    n = fn.new()
    target = ('vs', o.mem.disp)
    fn.calls[(target, 'thiscall', nargs)] = True
    out.append(('call', n, target, 'thiscall', obj, args, vt[1]))
    fn.rets.add(4 * nargs)
    fn.retkinds.add('u32')
    fn.argmax = max(getattr(fn, 'argmax', -1), st.argmax)
    out.append(('ret', ('v', n), 'u32'))
    fn.tail = True


def switch(fn, st, out, o, pc):
    if st.fp:
        raise Reject('x87 stack at a switch')
    idx = regval(st, fn, o.mem.index)
    k = table_bound(st, idx)
    if k is None or k > 40:
        raise Reject('switch without a proven bound')
    cases = []
    for j in range(k + 1):
        t = u32_at(o.mem.disp + 4 * j)
        if not fn.inside(t):
            raise Reject('switch target outside')
        s = st.copy()
        s.conds = st.conds + [('eq', idx, C(j), 4)]
        fn.paths += 1
        if fn.paths > MAXPATHS:
            raise Reject('too many paths')
        so = []
        run(fn, s, t, so)
        cases.append((j, so))
    fn.tables = getattr(fn, 'tables', []) + [(o.mem.disp, 4 * (k + 1))]
    # if-chain; the last case is the `else` (the bound makes it exhaustive)
    node = cases[-1][1]
    for j, so in reversed(cases[:-1]):
        node = [('if', ('eq', idx, C(j), 4), so, node)]
    out.extend(node)


def check_restored(st):
    for r in ('ebx', 'esi', 'edi', 'ebp'):
        if st.r[r] != ('in', r):
            raise Reject('%s not restored' % r)
    if st.fs0 != ('fs0',):
        raise Reject('fs:[0] not restored')


def finish(fn, st, out, n):
    if st.esp != 0:
        raise Reject('unbalanced stack at ret')
    check_restored(st)
    fn.rets.add(n)
    fn.argmax = max(getattr(fn, 'argmax', -1), st.argmax)
    if st.fp:
        if len(st.fp) != 1 or st.st0 is not None:
            raise Reject('x87 stack at ret')
        fn.retkinds.add('x87')
        out.append(('ret', ('v', st.fp[0]), 'x87v'))
        return
    if st.st0 is not None:
        fn.retkinds.add('x87')
        out.append(('ret', st.st0, 'x87'))
        return
    e = st.r['eax']
    if e == ('in', 'eax'):
        fn.retkinds.add('void')
        out.append(('ret', None, 'void'))
        return
    if e == ('bin', 'and', ('in', 'eax'), C(0xFFFFFF00)):
        e = ('bin', 'or', e, C(0))
    if e[0] == 'bin' and e[1] == 'or' and e[2] == ('bin', 'and', ('in', 'eax'), C(0xFFFFFF00)):
        low = e[3]
        if has_unknown(low):
            raise Reject('unknown AL at ret')
        fn.retkinds.add('u8')
        out.append(('ret', low, 'u8'))
        return
    # a C++ `bool` virtual (BOOL_METHODS): only AL is the result, whatever the upper EAX bytes hold (an x87
    # status word, a callee's leftovers) (PORT-AI-8)
    if method_of(getattr(fn, 'name', '') or '') in BOOL_METHODS and e[0] == 'bin' and e[1] == 'and' and \
            e[3] == C(0xFFFFFF00) and has_unknown(e[2]):
        fn.retkinds.add('u8')
        out.append(('ret', C(0), 'u8'))
        return
    if method_of(getattr(fn, 'name', '') or '') in BOOL_METHODS and e[0] == 'bin' and e[1] == 'or' and \
            e[2][0] == 'bin' and e[2][1] == 'and' and e[2][3] == C(0xFFFFFF00) and has_unknown(e[2]) and \
            not has_unknown(e[3]):
        fn.retkinds.add('u8')
        out.append(('ret', e[3], 'u8'))
        return
    if has_unknown(e):
        raise Reject('unknown EAX at ret')
    fn.retkinds.add('u32')
    out.append(('ret', e, 'u32'))


# Virtuals whose result is a C++ `bool` (AL): only AL is compared and returned (`-> u8`), so the upper
# EAX bytes a callee leaves (e.g. a sub-task's own `MakeAbortable`) are not part of the port's result.
BOOL_METHODS = {'MakeAbortable', 'ProcessPed', 'SetPedPosition', 'IsInterruptable', 'IsValid', 'AffectsPed',
                'AffectsPedGroup', 'TakesPriorityOver', 'CanBeInterruptedBySameEvent', 'IsCriminalEvent',
                'HasEditableResponse', 'DoInformVehicleOccupants', 'IsSimple'}


def is_dtor_name(m):
    return m.startswith('~') or m == 'Destructor' or m.startswith('__dtor_')


def analyse(a, name=''):
    body = a
    stub = False
    i = insn_at(a)
    if i and i.mnemonic == 'jmp' and i.size == 5 and i.op_str.startswith('0x') and int(i.op_str, 16) >= HOODLUM:
        body = int(i.op_str, 16)
        stub = True
    ranges = list(body_ranges(a))
    if not ranges:
        raise Reject('no extent')
    fn = Fn(a, body, ranges)
    fn.name = name
    fn.seh_pcs = set()
    hd = list(md.disasm(rd(body, 24), body))[:3]
    if len(hd) == 3 and hd[0].mnemonic == 'push' and hd[0].operands[0].type == X.X86_OP_IMM and hd[0].operands[0].imm & M32 == M32 and hd[1].mnemonic == 'push' and \
            hd[1].operands[0].type == X.X86_OP_IMM and hd[2].op_str == 'eax, dword ptr fs:[0]':
        fn.seh_pcs = {hd[0].address, hd[1].address}
    # the other MSVC order: `mov eax, fs:[0]; push -1; push handler` (PORT-AI-8)
    if len(hd) == 3 and hd[0].op_str == 'eax, dword ptr fs:[0]' and hd[1].mnemonic == 'push' and \
            hd[1].operands[0].type == X.X86_OP_IMM and hd[1].operands[0].imm & M32 == M32 and \
            hd[2].mnemonic == 'push' and hd[2].operands[0].type == X.X86_OP_IMM:
        fn.seh_pcs = {hd[1].address, hd[2].address}
    st = St()
    out = []
    run(fn, st, body, out)
    if len(fn.rets) != 1:
        raise Reject('paths return with different cleanups')
    kinds = fn.retkinds
    if method_of(name) in BOOL_METHODS and kinds <= {'u8', 'u32'}:
        kinds = {'u8'}
    if len(kinds) > 1:
        if kinds == {'void', 'u32'} or 'x87' in kinds or 'u8' in kinds:
            raise Reject('paths return different kinds')
    fn.ret = kinds.pop()
    fn.pop = fn.rets.pop()
    if fn.ret == 'void' and fn.tail:
        fn.ret = 'u32'
    # a complete destructor returns nothing: EAX after its tail/base call is whatever the base left
    # (often the caller's EAX passed through), and no caller reads it
    # a catalogue name that is only the class name (`C::C`) can be the destructor (the `~` is lost in
    # some catalogue names, e.g. 0x6389F0 `CTaskComplexSequence`): a constructor returns `this`
    m0, c0 = method_of(name), cls_of(name)
    rv0 = [x[1] for x in flatten(out) if x[0] == 'ret']
    bare_class = m0 == c0 or ('::' not in name and re.fullmatch(r'C[A-Z]\w*', name or '') is not None)
    looks_dtor = bare_class and not all(v == ('this',) for v in rv0)
    if fn.ret == 'u32' and (is_dtor_name(method_of(name)) or looks_dtor):
        rv = [x[1] for x in flatten(out) if x[0] == 'ret']
        calls = {x[1] for x in flatten(out) if x[0] == 'call'}
        if all(v is not None and v[0] == 'v' and v[1] in calls for v in rv):
            fn.ret = 'void'
    fn.stub = stub
    fn.out = out
    if fn.pop % 4:
        raise Reject('odd ret')
    nstack = fn.pop // 4
    if nstack == 0:
        nstack = fn.argmax + 1
        if nstack and fn.uses_this:
            raise Reject('ECX and caller-cleaned arguments')
        fn.conv = 'thiscall' if fn.uses_this or nstack == 0 else 'cdecl'
    else:
        if fn.argmax + 1 > nstack:
            raise Reject('reads beyond its arguments')
        fn.conv = 'thiscall'
    fn.nargs = nstack
    return fn


# ---------------------------------------------------------------- Rust emission


class Emit:
    def __init__(self, fn):
        self.fn = fn
        self.used = set()
        self.collect(fn.out)

    def collect_e(self, e):
        if not isinstance(e, tuple):
            return
        if e[0] == 'v':
            self.used.add(e[1])
        for x in e[1:]:
            if isinstance(x, tuple):
                self.collect_e(x)

    def collect(self, stmts):
        for s in stmts:
            if s[0] == 'load':
                self.collect_e(s[4])
            elif s[0] == 'let':
                self.collect_e(s[2])
            elif s[0] == 'store':
                self.collect_e(s[2])
                self.collect_e(s[4])
            elif s[0] == 'call':
                self.collect_e(s[4])
                for a in s[5]:
                    self.collect_e(a)
                if s[6] is not None:
                    self.used.add(s[6])
            elif s[0] == 'if':
                for x in s[1][1:3]:
                    self.collect_e(x)
                self.collect(s[2])
                self.collect(s[3])
            elif s[0] == 'ret':
                self.collect_e(s[1])
            elif s[0] in X87_STMTS:
                for x in s[1:]:
                    if isinstance(x, int) and not isinstance(x, bool):
                        self.used.add(x)
                    elif isinstance(x, tuple):
                        self.collect_e(x)

    def src(self, m, kind='f32'):
        """An x87 memory operand: ('mem', base, disp), ('bits', u32 expr) or ('f64', ('mem', ...))."""
        if m[0] == 'bits':
            return 'f32::from_bits(%s)' % self.e(m[1])
        if m[0] == 'f64':
            kind, m = 'f64', m[1]
        t = {'f32': 'f32', 'f64': 'f64', 'i32': 'i32'}[kind]
        if m[1] is None:
            return 'g::<%s>(%s).read()' % (t, hexlit(m[2]))
        return 'rd::<%s>(p(%s), %s)' % (t, self.e(m[1]), off(m[2]))

    def e(self, x):
        k = x[0]
        if k == 'c':
            return hexlit(x[1])
        if k == 'this':
            return 'this'
        if k == 'frame':
            return 'fb'
        if k == 'arg':
            return 'a%d' % x[1]
        if k == 'v':
            return 'v%d' % x[1]
        if k == 'sx':
            t = {1: 'i8', 2: 'i16'}[x[1]]
            return '(%s as %s as i32 as u32)' % (paren(self.e(x[2])), t)
        if k == 'cond':
            return 'u32::from(%s)' % self.cond(x[1])
        if k == 'bin':
            op, a, b = x[1], x[2], x[3]
            ea, eb = paren(self.e(a)), paren(self.e(b))
            if is_c(a) and op in ('add', 'sub', 'mul'):
                ea = '%s_u32' % hexlit(a[1])
            if op == 'add':
                return '%s.wrapping_add(%s)' % (ea, eb)
            if op == 'sub':
                return '%s.wrapping_sub(%s)' % (ea, eb)
            if op == 'mul':
                return '%s.wrapping_mul(%s)' % (ea, eb)
            if op in ('and', 'or', 'xor'):
                return '(%s %s %s)' % (ea, {'and': '&', 'or': '|', 'xor': '^'}[op], eb)
            if op in ('shl', 'shr'):
                if not is_c(b):
                    raise Reject('shift by a value')
                return '(%s %s %d)' % (ea, '<<' if op == 'shl' else '>>', b[1] & 31)
            if op == 'sar':
                return '(((%s as i32) >> %d) as u32)' % (ea, b[1] & 31)
            if op == 'mulhi':
                return '(((i64::from(%s as i32) * i64::from(%s as i32)) >> 32) as u32)' % (ea, eb)
        raise Reject('emit %r' % (x,))

    def cond(self, c):
        op, a, b, size = c
        if op in ('fany', 'fnone'):
            return '%s%s.test_any(%s)' % ('!' if op == 'fnone' else '', self.e(a), hexlit(b[1]))
        if op in ('fpar', 'fodd'):
            return '%s%s.test_parity(%s)' % ('!' if op == 'fodd' else '', self.e(a), hexlit(b[1]))
        ea, eb = self.e(a), self.e(b)
        ty = {1: ('u8', 'i8'), 2: ('u16', 'i16'), 4: ('u32', 'i32')}[size]
        # a constant beyond the narrow type's signed range gets a u32 suffix, so its cast truncates instead of
        # tripping `overflowing_literals` (PORT-AI-8; no earlier body has such a constant)
        lim = {1: 0x7F, 2: 0x7FFF, 4: 0x7FFFFFFF}[size]
        if is_c(a) and a[1] > lim and size < 4:
            ea = '%s_u32' % hexlit(a[1])
        if is_c(b) and b[1] > lim and size < 4:
            eb = '%s_u32' % hexlit(b[1])
        sym = {'eq': '==', 'ne': '!=', 'lt': '<', 'ge': '>=', 'gt': '>', 'le': '<=',
               'ult': '<', 'uge': '>=', 'ugt': '>', 'ule': '<='}[op]
        if op in ('eq', 'ne'):
            if size == 4:
                return '%s %s %s' % (ea, sym, eb)
            return '(%s as %s) %s (%s as %s)' % (ea, ty[0], sym, eb, ty[0])
        if op.startswith('u'):
            return '(%s as %s) %s (%s as %s)' % (ea, ty[0], sym, eb, ty[0])
        return '(%s as %s) %s (%s as %s)' % (ea, ty[1], sym, eb, ty[1])

    def ptr(self, base, disp):
        if base is None:
            return 'g::<{T}>(%s)' % hexlit(disp), None
        return 'p(%s)' % self.e(base), disp

    def stmts(self, ss, ind):
        out = []
        pad = '    ' * ind
        for s in ss:
            if s[0] == 'load':
                _, n, size, signed, base, disp = s
                t = {1: 'u8', 2: 'u16', 4: 'u32'}[size]
                if base is None:
                    rexp = 'g::<%s>(%s).read()' % (t, hexlit(disp))
                else:
                    rexp = 'rd::<%s>(p(%s), %s)' % (t, self.e(base), off(disp))
                if size < 4:
                    if signed:
                        rexp = '%s as %s as i32 as u32' % (rexp, {1: 'i8', 2: 'i16'}[size])
                    else:
                        rexp = 'u32::from(%s)' % rexp
                name = 'v%d' % n if n in self.used else '_v%d' % n
                out.append('%slet %s = %s;' % (pad, name, rexp))
            elif s[0] == 'let':
                out.append('%slet v%d = %s;' % (pad, s[1], self.e(s[2])))
            elif s[0] == 'store':
                _, size, base, disp, val = s
                t = {1: 'u8', 2: 'u16', 4: 'u32'}[size]
                ve = self.e(val)
                if size < 4:
                    ve = '%s as %s' % (paren(ve), t)
                if base is None:
                    out.append('%sg::<%s>(%s).write(%s);' % (pad, t, hexlit(disp), ve))
                else:
                    out.append('%swr::<%s>(p(%s), %s, %s);' % (pad, t, self.e(base), off(disp), ve))
            elif s[0] in X87_STMTS:
                out.append(pad + self.x87_stmt(s))
            elif s[0] == 'call':
                _, n, target, conv, this, args, vload = s
                if target == ('d', RAND):
                    ce = 'sa_core::orig::at_site(%s, rand) as u32' % hexlit(vload[1])
                    out.append('%slet v%d = %s;' % (pad, n, ce) if n in self.used else '%s%s;' % (pad, ce))
                    continue
                fname = callee_name(target, conv, len(args))
                te = None if this is None else (self.e(this) if target[0] in ('d', 'df') else 'p(%s)' % self.e(this))
                al = ([te] if te is not None else []) + [self.e(a) for a in args]
                ce = '%s(%s)' % (fname, ', '.join(al))
                if target[0] == 'df':
                    ce = 'X87::from_f64_exact(%s)' % ce
                if n in self.used:
                    out.append('%slet v%d = %s;' % (pad, n, ce))
                else:
                    out.append('%s%s;' % (pad, ce))
            elif s[0] == 'if':
                out.append('%sif %s {' % (pad, self.cond(s[1])))
                out.extend(self.stmts(s[2], ind + 1))
                if s[3]:
                    out.append('%s} else {' % pad)
                    out.extend(self.stmts(s[3], ind + 1))
                out.append('%s}' % pad)
            elif s[0] == 'ret':
                _, e, kind = s
                rk = self.fn.ret
                if rk == 'void':
                    out.append('%sreturn;' % pad)
                elif rk == 'u8':
                    ee = '%s_u32' % hexlit(e[1]) if e is not None and is_c(e) and e[1] > 0xFF else paren(self.e(e))
                    out.append('%sreturn %s as u8;' % (pad, ee))
                elif kind == 'x87v':
                    out.append('%sreturn %s;' % (pad, self.e(e)))
                elif rk == 'x87':
                    out.append('%sreturn x(f32::from_bits(%s));' % (pad, self.e(e)))
                else:
                    out.append('%sreturn %s;' % (pad, self.e(e)))
        return out

    def x87_stmt(self, s):
        k = s[0]
        if k == 'fld':
            _, n, kind, m = s
            if kind == 'f32':
                return 'let v%d = x(%s);' % (n, self.src(m))
            if kind == 'f64':
                return 'let v%d = X87::from_f64_exact(%s);' % (n, self.src(m, 'f64'))
            if kind == 'i32':
                return 'let v%d = xi(%s);' % (n, self.src(m, 'i32'))
            return 'let v%d = xi(%s as i32);' % (n, self.e(m[1]))
        if k == 'fop':
            _, n, op, a, b = s
            return 'let v%d = v%d.%s(v%d);' % (n, a, op, b)
        if k == 'fopm':
            _, n, op, a, m = s
            return 'let v%d = v%d.%s_m(%s);' % (n, a, op, self.src(m))
        if k == 'fun':
            _, n, op, a = s
            if op == 'one':
                return 'let v%d = X87::one();' % n
            if op == 'zero':
                return 'let v%d = X87::zero();' % n
            return 'let v%d = v%d.%s();' % (n, a, op)
        if k == 'fpatan':
            _, n, y, x_ = s
            return 'let v%d = v%d.fpatan(v%d);' % (n, y, x_)
        if k == 'fcmp':
            _, n, a, b = s
            return 'let v%d = v%d.fcom(v%d);' % (n, a, b)
        if k == 'fcmpm':
            _, n, a, m = s
            if m[0] == 'f64':
                return 'let v%d = v%d.fcom(X87::from_f64_exact(%s));' % (n, a, self.src(m))
            return 'let v%d = v%d.fcom(x(%s));' % (n, a, self.src(m))
        if k == 'fst':
            _, a, m = s
            if m[1] is None:
                return 'g::<f32>(%s).write(f32_of(v%d));' % (hexlit(m[2]), a)
            return 'wr::<f32>(p(%s), %s, f32_of(v%d));' % (self.e(m[1]), off(m[2]), a)
        if k == 'fbits':
            _, n, a = s
            return 'let v%d = f32_of(v%d).to_bits();' % (n, a)
        if k == 'ftol':
            _, n, a = s
            return 'let v%d = ftol32(v%d) as u32;' % (n, a)
        raise Reject('emit x87 %s' % k)


X87_STMTS = ('fld', 'fop', 'fopm', 'fun', 'fpatan', 'fcmp', 'fcmpm', 'fst', 'fbits', 'ftol')


def paren(s):
    if re.fullmatch(r'[A-Za-z0-9_]+', s) or (s.startswith('(') and s.endswith(')') and balanced(s[1:-1])):
        return s
    return '(%s)' % s


def balanced(s):
    d = 0
    for ch in s:
        if ch == '(':
            d += 1
        elif ch == ')':
            d -= 1
            if d < 0:
                return False
    return d == 0


def hexlit(v):
    v &= M32
    if v < 10:
        return str(v)
    h = '%X' % v
    if len(h) > 4:
        h = h[:-4] + '_' + h[-4:]
    return '0x' + h


def off(d):
    d &= M32
    if d & 0x80000000:
        return '-%s' % hexlit((1 << 32) - d)
    return hexlit(d)


def callee_name(target, conv, n):
    if target[0] == 'd':
        return 'c_%x_%s%d' % (target[1], conv[0], n)
    if target[0] == 'df':
        return 'c_%x_%s%d_st0' % (target[1], conv[0], n)
    return 'vslot_%x_%d' % (target[1], n)


def strip_returns(lines):
    """`return x;` as the last statement of the function becomes the tail expression."""
    if lines and lines[-1].startswith('    return'):
        l = lines[-1]
        if l == '    return;':
            lines.pop()
        else:
            lines[-1] = '    ' + l[len('    return '):-1]
    return lines


# ---------------------------------------------------------------- families


def family(name, fn):
    m = method_of(name)
    c = cls_of(name)
    out = fn.out
    flat = list(flatten(out))
    ncalls = sum(1 for s in flat if s[0] == 'call')
    nstores = sum(1 for s in flat if s[0] == 'store')
    nifs = sum(1 for s in flat if s[0] == 'if')
    rets = [s for s in flat if s[0] == 'ret']
    const_ret = all(s[1] is not None and is_c(s[1]) for s in rets)
    if m.startswith('~') or m in ('Destructor', "`scalar deleting destructor'") or m.startswith('__dtor_'):
        return 'dtor'
    if 'Clone' in m:
        return 'clone'
    if m in ('Constructor', c) or m == c.split('<')[0] or m.startswith('Constructor'):
        return 'ctor'
    if ncalls == 0 and nstores == 0 and nifs == 0 and len(rets) == 1:
        r = rets[0]
        if fn.ret in ('u32',) and const_ret:
            if m in ('GetTaskType', 'GetEventType', 'GetEventPriority', 'GetLifeTime', 'GetType', 'GetWanderType'):
                return 'const-id'
            return 'const'
        if fn.ret == 'u8' and const_ret:
            return 'const-bool'
        if fn.ret == 'void':
            return 'empty'
        if fn.ret == 'x87':
            return 'getter-f32'
        loads = [s for s in flat if s[0] == 'load']
        if len(loads) == 1 and r[1] in (('v', loads[0][1]),) or (len(loads) == 1 and fn.ret in ('u32', 'u8')):
            return 'getter'
        if fn.ret == 'u32' and r[1][0] == 'arg':
            return 'getter'
    if ncalls == 0 and nifs == 0 and nstores >= 1 and fn.ret in ('void', 'u32') and \
            all(s[0] in ('load', 'store', 'ret') for s in flat) and not re.search(r'Constructor|^C\w+$', m):
        if m.startswith('Set') or fn.nargs >= 1 or fn.ret == 'void':
            return 'setter'
    vt_stores = [s for s in flat if s[0] == 'store' and s[2] == ('this',) and s[3] == 0 and is_c(s[4])]
    if 'Clone' in m:
        return 'clone'
    if m in ('Constructor', 'Constructor-1', 'Constructor-2', 'Constructor-Anim', c) or m == c.split('<')[0] or \
            m.startswith('Constructor'):
        return 'ctor'
    if m.startswith('~') or m in ('Destructor', "`scalar deleting destructor'") or m.startswith('__dtor_'):
        return 'dtor'
    if m == 'MakeAbortable':
        return 'make-abortable'
    if m in ('CreateSubTask', 'CreateFirstSubTask', 'CreateNextSubTask', 'ControlSubTask'):
        return 'subtask'
    if name and name.startswith('__init_'):
        return 'static-init'
    if m in ('IsInterruptable', 'IsValid', 'AffectsPedGroup', 'AffectsPed', 'CanBeInterruptedBySameEvent',
             'TakesPriorityOver', 'IsCriminalEvent', 'HasEditableResponse', 'GetSourceEntity',
             'GetLocalSoundLevel', 'DoInformVehicleOccupants', 'ReportCriminalEvent', 'SetPedPosition',
             'StopTimer', 'Abort', 'ProcessPed', 'GetSubTask', 'IsSimple', 'CloneEditable', 'Flush'):
        return 'virtual-leaf'
    # unnamed / vt_ functions by shape
    if vt_stores and (fn.ret == 'u32' and rets and all(s[1] == ('this',) for s in rets)):
        return 'ctor'
    if vt_stores and ncalls >= 1:
        return 'dtor'
    if ncalls == 0 and nifs == 0 and len(flat) <= 6:
        return 'leaf'
    # any other AI body the interpreter executed completely (PORT-AI-8): helpers, thunks, geometry and
    # attractor computations, event and task methods outside the named families
    return 'body'


def nm_static(name):
    """Free functions and static initialisers (no object): cdecl when they take nothing."""
    return '::' not in name or name.startswith('__') or method_of(name).startswith('__')


def flatten(ss):
    for s in ss:
        yield s
        if s[0] == 'if':
            yield from flatten(s[2])
            yield from flatten(s[3])


FAMILY_FILE = {
    'const-id': 'gen_leaves', 'const': 'gen_leaves', 'const-bool': 'gen_leaves', 'empty': 'gen_leaves',
    'getter': 'gen_leaves', 'getter-f32': 'gen_leaves', 'setter': 'gen_leaves', 'leaf': 'gen_leaves',
    'virtual-leaf': 'gen_virtuals', 'ctor': 'gen_ctors', 'clone': 'gen_clones', 'dtor': 'gen_dtors',
    'make-abortable': 'gen_virtuals', 'subtask': 'gen_subtasks', 'static-init': 'gen_virtuals',
    'body': 'gen_bodies',
}
FILE_DOC = {
    'gen_leaves': 'constant `GetTaskType`/`GetEventType`/`GetEventPriority`/`GetLifeTime`, constant-bool\n'
                  '//! virtuals, empty bodies, field getters and setters',
    'gen_virtuals': 'small task/event virtuals (`IsValid`, `AffectsPed`, `ProcessPed`, `MakeAbortable`, ...)\n'
                    '//! and static initialisers',
    'gen_ctors': 'constructors (base constructor call, field stores, vtable store)',
    'gen_clones': '`Clone`/`CloneEditable` (pool `operator new`, null check, constructor with copied fields)',
    'gen_dtors': 'complete and scalar deleting destructors',
    'gen_subtasks': '`CreateSubTask`/`CreateFirstSubTask`/`CreateNextSubTask`/`ControlSubTask` bodies that only\n'
                    '//! select a task id or a constructor',
    'gen_bodies': 'other AI bodies outside the named families (helpers, thunks, geometry, attractor and\n'
                  '//! event/task methods)',
}


def ident(name, a):
    n = method_of(name) if name else 'fun'
    n = re.sub(r'[^A-Za-z0-9]+', '_', n)
    n = re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', n).lower().strip('_')
    cls = cls_of(name)
    cls = re.sub(r'^C(Task|Event)', r'\1', cls)
    cls = re.sub(r'[^A-Za-z0-9]+', '_', cls)
    cls = re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', cls).lower().strip('_')
    s = (cls + '_' + n) if cls else (n or 'fun')
    s = re.sub(r'_+', '_', s).strip('_')
    if not s or s[0].isdigit():
        s = 'f_' + s
    return 'g_%s_%x' % (s, a)


def expect_of(a):
    out, n = [], 0
    for i in md.disasm(rd(a, 32), a):
        out.append(i.bytes.hex(' ').upper())
        n += i.size
        if n >= 5:
            break
    return ' '.join(out), n


def hookable(a, fn):
    if not fn.stub:
        n = 0
        for i in md.disasm(rd(a, 64), a):
            n += i.size
            if i.mnemonic == 'ret' and n < 5:
                return 'the body up to its `ret` is shorter than the 5-byte patch'
            if i.mnemonic == 'jmp' and n <= 5 and i.size == 5 and n == 5:
                return 'the entry is a bare `jmp rel32` (sa-hook would treat it as a .HOODLUM stub)'
            if n >= 5:
                break
        # an inbound branch into the patched bytes (other than the entry itself)
        for pc, sz in fn.insns.items():
            pass
    for t, lst in READS.items():
        if a <= t < a + 5 or (t <= a < t + max(s for _, s in lst)):
            return 'protection code reads the patched bytes (INF-20, `catalog/code-data-reads.csv` reader %#x)' % lst[0][0]
    return True


# Hooks whose original callers keep using ECX/EDX after the call (sa-hook `callers` test): the
# original body leaves the register unchanged and the port must too.
PRESERVE = {
    0x4B1F80: 'ecx',
    # its caller 0x62DD41 passes the same ECX on to 0x628A70 (sa-hook's callers test, PORT-AI-8)
    0x6289F0: 'ecx',
}


# Writer/verifier split (docs/verify/HOWTO.md): a generated hook whose catalogue state is still
# `unknown`/`specified` (`catalog/state.csv`) is registered `unverified`, with the batch id
# `tools/gen/ai_patterns_unverified.txt` assigns it (`ADDR BATCH` lines). `--assign PREFIX` appends the
# missing ones in address order, in batches of at most BATCH_MAX (`PREFIX-<n>`, n after the highest one
# in use). A verifier promotes a hook by setting its state to L1/L2/L3 and deleting its line. A listed hook
# stays `unverified` while its state is below L1, including `implemented` (written, not verified: an
# integration records that state for every registered hook). Hooks generated before the split (AI-GEN, not
# listed, state `implemented` or above) carry no `unverified`.
BATCH_MAX = 60
VERIFIED_STATES = ('L1', 'L2', 'L3')
UNVERIFIED_FILE = os.path.join(ROOT, 'tools', 'gen', 'ai_patterns_unverified.txt')


def read_states():
    out = {}
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'state.csv'))):
        out[int(r['address'], 16)] = r['state']
    return out


def read_batches():
    out = {}
    if os.path.exists(UNVERIFIED_FILE):
        for l in open(UNVERIFIED_FILE):
            l = l.split('#', 1)[0].strip()
            if l:
                a, b = l.split()
                out[int(a, 16)] = b
    return out


def route_hazards(fn, name):
    """The verifier hints (docs/verify/HOWTO.md §1) from what the interpreter executed.

    route: `split` for static initialisers and `atexit` destructors (one-time code), `L2` otherwise (also for bodies that only
    return a constant: this generator writes no L1 specs, and their double run is trivially exact). hazards: `preserved-regs` (declared
    `preserve`), `ax-al-return` (an AL result with the caller's upper EAX), `heap-writes` (a store
    through a pointer, not to a fixed global), else `none`."""
    flat = list(flatten(fn.out))
    haz = []
    if fn.addr in PRESERVE:
        haz.append('preserved-regs')
    if fn.ret == 'u8':
        haz.append('ax-al-return')
    if any((x[0] == 'store' and x[2] not in (None, ('frame',))) or (x[0] == 'fst' and x[2][1] not in (None, ('frame',)))
           for x in flat):
        haz.append('heap-writes')
    if getattr(fn, 'transcendental', False):
        haz.append('x87-transcendental')
    if getattr(fn, 'frame', False):
        # the Rust frame starts zeroed where the original's stack holds garbage
        haz.append('uninit-locals')
    if (name or '').startswith(('__init_', '__dtor_')):
        route = 'split'
    else:
        route = 'L2'
    return route, ','.join(haz) or 'none'


def fnv(data):
    h = 0xcbf29ce484222325
    for b in data:
        h ^= b
        h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return h


def main():
    check = '--check' in sys.argv
    done = already_ported()
    for l in open(os.path.join(ROOT, 'tools', 'gen', 'ai_patterns_exclude.txt')):
        l = l.split('#')[0].strip()
        if l:
            done.add(int(l, 16))
    addrs = sorted(a for a in PROCS)
    index = {a: k for k, a in enumerate(addrs)}
    cand = []
    for a in addrs:
        r = PROCS[a]
        name = catname(a)
        if r['subsystem'] != 'AI' and not (name and AI_CLASS.match(name)):
            continue
        if a in done:
            continue
        c = cls_of(name or '')
        if EXCLUDE.match(c):
            continue
        if not c or c.startswith('vt_') or (name or '').startswith('FUN_'):
            if any(EXCLUDE.match(nc) for nc in neighbour_class(a, addrs, index[a])):
                continue
        cand.append((a, name or 'FUN_%08x' % a))
    gens = {}
    rest = []
    seen = set()
    for a, name in cand:
        try:
            fn = analyse(a, name)
            fam = family(name, fn)
            if fam is None:
                raise Reject('not an AI-GEN family')
            if fam == 'dtor' and fn.ret == 'u32':
                # an unnamed destructor (by shape) returns nothing either (see analyse)
                rv = [x[1] for x in flatten(fn.out) if x[0] == 'ret']
                calls = {x[1] for x in flatten(fn.out) if x[0] == 'call'}
                if all(v is not None and v[0] == 'v' and v[1] in calls for v in rv):
                    fn.ret = 'void'
            em = Emit(fn)
            body = em.stmts(fn.out, 1)
            text = '\n'.join(body)
            if nm_static(name) and fn.conv == 'thiscall' and not fn.uses_this and fn.nargs == 0:
                fn.conv = 'cdecl'
            if re.search(r'\bthis\b', text) and fn.conv != 'thiscall':
                raise Reject('reads ECX in a caller-cleaned function')
            fn.uses_this = bool(re.search(r'\bthis\b', text))
            for m in sorted(set(re.findall(r'let (v\d+) =', text))):
                if len(re.findall(r'\b%s\b' % m, text)) == 1:
                    body = [re.sub(r'let %s =' % m, 'let _%s =' % m, l) for l in body]
        except Reject as ex:
            rest.append((a, name, str(ex)))
            continue
        except RecursionError:
            rest.append((a, name, 'recursion'))
            continue
        fid = ident(name, a)
        while fid in seen:
            fid += '_b'
        seen.add(fid)
        gens.setdefault(FAMILY_FILE[fam], []).append((a, name, fid, fn, fam, strip_returns(body)))
    states = read_states()
    batches = read_batches()
    want = []
    for recs in gens.values():
        for a, name, fid, fn, fam, body in recs:
            st = states.get(a, 'unknown')
            # `implemented` means written, not verified: a hook listed in UNVERIFIED_FILE stays unverified
            # until a verifier raises its state to L1/L2/L3 (and deletes its line).
            if (hookable(a, fn) is True and st not in VERIFIED_STATES
                    and (a in batches or st in ('unknown', 'specified'))):
                fn.unverified = batches.get(a)
                if fn.unverified is None:
                    want.append(a)
            else:
                fn.unverified = None
    if want:
        if '--assign' not in sys.argv:
            sys.exit('%d generated hooks have no batch in %s: run with --assign PREFIX' % (
                len(want), os.path.relpath(UNVERIFIED_FILE, ROOT)))
        prefix = sys.argv[sys.argv.index('--assign') + 1]
        used = [int(b.rsplit('-', 1)[1]) for b in batches.values() if b.rsplit('-', 1)[0] == prefix]
        # hand-written batches of the same prefix in the crates count too (no id is reused)
        for dirpath, _, fs in os.walk(os.path.join(ROOT, 'crates')):
            for f in sorted(fs):
                if f.endswith('.rs'):
                    txt = open(os.path.join(dirpath, f), encoding='utf-8').read()
                    used += [int(n) for n in re.findall(r'unverified = "%s-(\d+)"' % re.escape(prefix), txt)]
        n = max(used, default=0)
        with open(UNVERIFIED_FILE, 'a') as fh:
            for k, a in enumerate(sorted(want)):
                if k % BATCH_MAX == 0:
                    n += 1
                fh.write('0x%X %s-%d\n' % (a, prefix, n))
        batches = read_batches()
        for recs in gens.values():
            for a, name, fid, fn, fam, body in recs:
                if a in want:
                    fn.unverified = batches[a]
    files = {}
    report = []
    for f in sorted(FAMILY_FILE.values()):
        if f in files:
            continue
        files[f] = rustfmt(render(f, gens.get(f, []), report))
    os.makedirs(REPORT_DIR, exist_ok=True)
    bad = []
    for f, text in sorted(files.items()):
        path = os.path.join(OUT_DIR, f + '.rs')
        if check:
            old = open(path).read() if os.path.exists(path) else None
            if old != text:
                bad.append(f)
        else:
            with open(path, 'w') as fh:
                fh.write(text)
    with open(os.path.join(REPORT_DIR, 'report.tsv'), 'w') as fh:
        for row in sorted(report):
            print(*row, sep='\t', file=fh)
    with open(os.path.join(REPORT_DIR, 'rest.tsv'), 'w') as fh:
        for a, n, why in rest:
            print('0x%X' % a, n, why, sep='\t', file=fh)
    counts = {}
    for row in report:
        counts[(row[3], row[4] == 'hook')] = counts.get((row[3], row[4] == 'hook'), 0) + 1
    for k in sorted(counts):
        print('%-16s %-10s %d' % (k[0], 'hook' if k[1] else 'unhookable', counts[k]))
    print('generated', len(report), 'hooks', sum(1 for r in report if r[4] == 'hook'), 'rest', len(rest))
    if bad:
        print('differs:', ', '.join(bad))
        sys.exit(1)


def render(fname, recs, report):
    hooks = []
    callees = {}
    parts = []
    hashes = []
    consts = []
    for a, name, fid, fn, fam, body in recs:
        for (target, conv, n) in fn.calls:
            callees[(target, conv, n)] = True
        h = hookable(a, fn)
        rng = sorted(fn.insns.items())
        spans = ([(a, 5)] if fn.stub else []) + merge_spans(rng) + list(getattr(fn, 'tables', []))
        data = b''.join(rd(x, n) for x, n in spans)
        hashes.append((a, fid, fnv(data), spans))
        if fam in ('const-id', 'const', 'const-bool', 'empty') and fn.ret != 'x87':
            consts.append((fam, fid, fn, body))
        args = []
        if fn.conv == 'thiscall':
            args.append('%s: u32' % ('this' if fn.uses_this else '_this'))
        used_args = set()
        for line in body:
            for m in re.finditer(r'\ba(\d+)\b', line):
                used_args.add(int(m.group(1)))
        for k in range(fn.nargs):
            args.append('%sa%d: u32' % ('' if k in used_args else '_', k))
        ret = {'void': '', 'u8': ' -> u8', 'u32': ' -> u32', 'x87': ' -> X87'}[fn.ret]
        doc = '/// `%s` (0x%X%s): %s, %s, `ret%s`; %d instruction(s) matched.\n' % (
            name, a, ', stub to .HOODLUM 0x%X' % fn.body if fn.stub else '', fam, fn.conv,
            ' %d' % fn.pop if fn.pop else '', len(rng))
        s = doc
        if h is True:
            exp, _ = expect_of(a)
            pres = ', preserve = "%s"' % PRESERVE[a] if a in PRESERVE else ''
            if getattr(fn, 'unverified', None):
                route, haz = route_hazards(fn, name)
                pres += ', unverified = "%s", route = "%s", hazards = "%s"' % (fn.unverified, route, haz)
            s += '#[hook(addr = 0x%X, conv = %s, name = "%s", expect = "%s"%s)]\n' % (a, fn.conv, name, exp, pres)
            hooks.append(fid)
        else:
            s += '/// Not hookable: %s.\n' % h
            # phase B still reaches it through the native registry (STD-6)
            s += '#[sa_core::native(addr = 0x%X, conv = %s, name = "%s")]\n' % (a, fn.conv, name)
        s += 'pub fn %s(%s)%s {\n' % (fid, ', '.join(args), ret)
        if getattr(fn, 'frame', False):
            # the original's locals (stack objects whose address escapes), entry ESP - 0x%X first
            s += '    let mut frame = [0u32; 0x%X];\n' % FRAME_DWORDS
            s += '    let fb = frame.as_mut_ptr() as usize as u32;\n'
        s += '\n'.join(body) + ('\n' if body else '')
        s += '}\n'
        parts.append(s)
        report.append(('0x%X' % a, name, fid, fam, 'hook' if h is True else 'unhookable: ' + h,
                       'stub' if fn.stub else 'inline', fname, fn.conv, str(fn.nargs), fn.ret,
                       str(sum(1 for x in flatten(fn.out) if x[0] == 'call')),
                       str(sum(1 for x in flatten(fn.out) if x[0] == 'store'))))
    head = ('//! Generated by `tools/gen/ai_patterns.py` (AI-GEN) from the 1.0 US bytes: %s.\n'
            '//! Every body below was executed instruction by instruction by the generator\'s symbolic\n'
            '//! interpreter; see the script and `docs/arch/ai-gen.md`. Do not edit by hand: regenerate.\n'
            '#![allow(clippy::unreadable_literal, clippy::identity_op, clippy::needless_return)]\n'
            '#![allow(clippy::let_and_return, clippy::too_many_arguments, clippy::cast_lossless)]\n'
            '#![allow(clippy::precedence, clippy::collapsible_else_if, clippy::if_same_then_else)]\n'
            '#![allow(unused_parens, clippy::double_parens, clippy::nonminimal_bool, clippy::eq_op)]\n'
            '#![allow(clippy::unnecessary_cast, clippy::erasing_op)]\n\n'
            'use sa_core::abi::HookReg;\n'
            'use sa_core::{hook%s};\n\n'
            '#[allow(unused_imports)]\n'
            'use crate::common::*;\n\n'
            '/// A 32-bit game address as a pointer.\n'
            '#[allow(dead_code)]\n'
            'fn p(a: u32) -> GPtr<u8> {\n'
            '    GPtr::from_addr(a as usize)\n'
            '}\n\n'
            '/// A generated body: (address, FNV-1a of the matched bytes, matched spans `(va, len)`).\n'
            'pub type Body = (u32, u64, &\'static [(u32, u32)]);\n\n') % (FILE_DOC[fname], ', orig' if callees else '')
    s = head
    if callees:
        s += 'orig! {\n'
        for (target, conv, n) in sorted(callees, key=lambda k: (k[0][0], k[0][1], k[1], k[2])):
            nm = callee_name(target, conv, n)
            ps = ([('this: u32' if target[0] in ('d', 'df') else 'obj: GPtr<u8>')] if conv == 'thiscall' else []) + \
                ['a%d: u32' % k for k in range(n)]
            if target[0] in ('d', 'df'):
                cn = catname(target[1]) or 'FUN_%08x' % target[1]
                st0 = target[0] == 'df'
                s += '    /// `%s` (0x%X), %s, %d stack argument(s)%s\n' % (cn, target[1], conv, n, ', ST0 result' if st0 else '')
                s += '    fn %s(%s) -> %s = %s 0x%X => unported;\n' % (nm, ', '.join(ps), 'f64' if st0 else 'u32', conv, target[1])
            else:
                s += '    /// virtual slot +0x%X (`call [vtbl + 0x%X]` on ECX), %d stack argument(s)\n' % (target[1], target[1], n)
                s += '    fn %s(%s) -> u32 = thiscall vslot 0x%X => unported;\n' % (nm, ', '.join(ps), target[1])
        s += '}\n\n'
    s += '\n'.join(parts)
    s += '\npub const HOOKS: &[HookReg] = &[\n'
    for h in hooks:
        s += '    HOOK_%s,\n' % h.upper()
    s += '];\n'
    s += '\n/// (address, 64-bit FNV-1a of the matched bytes, the matched spans `(va, len)`: the stub, every\n'
    s += '/// executed instruction, switch tables): the bodies this file was generated from (`gen_check`).\n'
    s += 'pub const BODIES: &[Body] = &[\n'
    for a, fid, hv, spans in hashes:
        s += '    (0x%X, 0x%016X, &[%s]),\n' % (a, hv, ', '.join('(0x%X, %d)' % (x, n) for x, n in spans))
    s += '];\n'
    s += '\n#[cfg(all(test, not(sa_phase_a)))]\nmod tests {\n    use super::*;\n\n'
    s += '    /// Every body still hashes to what the generator matched (the exe from `$SA_GTA_EXE` or the default path).\n'
    s += '    #[test]\n    fn bodies_match_the_exe() {\n        crate::gen_check::check_bodies(BODIES);\n    }\n'
    fams = {}
    for fam, fid, fn, body in consts:
        fams.setdefault(fam, []).append((fid, fn, body))
    for fam in sorted(fams):
        s += '\n    /// Pattern `%s`: every port returns the constant the bytes load (no memory access).\n' % fam
        s += '    #[test]\n    fn pattern_%s() {\n' % fam.replace('-', '_')
        for fid, fn, body in fams[fam]:
            args = (['0'] if fn.conv == 'thiscall' else []) + ['0'] * fn.nargs
            call = '%s(%s)' % (fid, ', '.join(args))
            if fn.ret == 'void':
                s += '        %s;\n' % call
            else:
                val = body[-1].strip() if body else ''
                val = re.sub(r' as u8$', '', val)
                s += '        assert_eq!(%s, %s);\n' % (call, val)
        s += '    }\n'
    s += '}\n'
    return s


def rustfmt(text):
    """Format a generated file as the repository's rustfmt does (`rustfmt.toml`, the workspace edition)."""
    r = subprocess.run(['rustfmt', '--edition', '2024', '--config-path', os.path.join(ROOT, 'rustfmt.toml'),
                        '--emit', 'stdout'], input=text, capture_output=True, text=True, check=True)
    return r.stdout


def merge_spans(rng):
    out = []
    for pc, sz in rng:
        if out and out[-1][0] + out[-1][1] == pc:
            out[-1] = (out[-1][0], out[-1][1] + sz)
        else:
            out.append((pc, sz))
    return out


if __name__ == '__main__':
    sys.setrecursionlimit(10000)
    main()
