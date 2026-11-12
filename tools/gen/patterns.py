"""Exact byte-pattern port generator (AI-GEN's interpreter, generalised): a symbolic x86 interpreter over a
fixed instruction subset, the Rust emitter, and the driver for every subsystem but AI.

Usage: .venv/bin/python -I tools/gen/patterns.py [--check]
       (`tools/gen/ai_patterns.py`, the sa-ai driver, keeps its own copy of the interpreter)

Reads `re/gta/1.0-US/gta_sa.exe` (never copied into the repo) and the catalogue. The driver selects every
catalogue function whose subsystem (`procedures.csv`, or `unk-classification-proposal.csv` for UNK-game
rows) maps to a port crate (SUBSYS_CRATE), skips the AI class families (AI_NAMES), every address already
registered (`#[hook]`/`#[native]`) anywhere in `crates/` outside this generator's own files, every address
listed in `tools/gen/exclude_inflight.txt` (master and the agents' branches and worktrees,
`tools/gen/inflight.py`), every `catalog/state.csv` row at implemented/L1/L2/L3/excluded not set by this
generator and AUDIO-GEN's scope, and writes per crate `src/generated.rs` (the module hub: helpers, `HOOKS`,
`BODIES`), `src/<class>_generated.rs` (a content class with CLASS_MIN functions) and
`src/generated_<family>.rs` (the rest), `crates/sa-oracle/src/specs_generated.rs` (L1 specs of the
`L1-pure` hooks) and `target/gen-patterns/{report,rest}.tsv`, `counts.txt`. Every hook is registered
unverified (docs/verify/HOWTO.md: batch `gen2-<subsystem>-<n>`, route, hazards, recs). Two runs give
byte-identical files; `--check` regenerates in memory and exits 1 if a generated file differs.
Design notes and counts: `docs/arch/gen-all.md`.

How a function is accepted (instruction by instruction, nothing is pattern-guessed):
1. Every instruction of every path of the body is decoded with capstone and executed by a small
   symbolic x86 interpreter that knows only an exact subset: 32-bit `mov`/`movzx`/`movsx`/`lea`/
   `xor`/`and`/`or`/`add`/`sub`/`inc`/`dec`/`neg`/`not`/`shl`/`shr`/`sar`/`imul`/`cdq`, byte stores
   and loads through AL/CL/DL/BL, `test`/`cmp` + `jcc`/`setcc`, `push`/`pop`, direct calls, virtual
   calls `call [vtbl + N]` on the object in ECX, direct tail jumps, forward branches, bounded
   `jmp [reg*4 + table]` switch dispatch (the table is read from the exe), `ret`/`ret N`, and
   `fld dword ptr [m]; ret` (ST0 result). Any other instruction, an unknown register or flag value
   reaching a use, a loop, a stack address escaping, an unbalanced stack, a callee-saved register
   not restored, or `fs:[0]` not restored rejects the function.
2. Calls: the callee's stack cleanup is read from its own `ret N` instructions (all of them must
   agree; a callee ending in a tail jump inherits its target's); a callee that cleans up nothing
   gets its argument count from the caller's `add esp, N` / `pop reg` right after the call, and
   no argument may be outstanding otherwise. The callee's first block must not read EAX/EDX/EBX/
   ESI/EDI/EBP before writing them (no register-passed arguments). ECX is passed (thiscall) whenever
   it holds a known value; a callee that reads ECX anywhere while it is unknown rejects the function.

The generated Rust contains only semantics: offsets, constants, vtable addresses and the
original addresses of callees, never code bytes. Each crate's `gen_all.rs` carries a host test that
re-reads the exe and checks a 64-bit FNV-1a hash of every accepted body.
"""
import csv
import os
import re
import sys

import capstone
import pefile
from capstone import x86_const as X

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
EXE = os.environ.get('SA_GTA_EXE') or os.path.expanduser('~/sa-engine/re/gta/1.0-US/gta_sa.exe')

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


def cls_of(name):
    return name.split('::')[0] if name and '::' in name else ''


def method_of(name):
    return name.split('::')[-1] if name else ''


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


def pops(a, depth=0):
    """Stack bytes the function at `a` removes on return (from its own `ret N`), or None."""
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
        s.fpu = []  # X87_EXT: the x87 register stack, top last
        s.pushes = []  # (offset, kind) since the last call
        s.visited = set()
        s.prologue = True
        s.argmax = -1
        s.conds = []

    def copy(s):
        n = St()
        n.r = dict(s.r)
        n.esp = s.esp
        n.stack = dict(s.stack)
        n.fs0 = s.fs0
        n.flags = s.flags
        n.st0 = s.st0
        n.fpu = list(s.fpu)
        n.pushes = list(s.pushes)
        n.visited = set(s.visited)
        n.prologue = s.prologue
        n.argmax = s.argmax
        n.conds = list(s.conds)
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
        self.changes = set()  # of 'ecx'/'edx': registers some path returns changed

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


def push(st, val, kind):
    st.esp -= 4
    st.stack[st.esp] = val
    st.pushes.append((st.esp, kind))


def pop(st):
    if st.esp >= 0:
        raise Reject('pop above the frame')
    v = st.stack.get(st.esp, ('unk', 'pop'))
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


def after_call(st):
    st.r['ecx'] = ('unk', 'ecx')
    st.r['edx'] = ('unk', 'edx')
    st.flags = None
    for k in list(st.stack):
        if k < st.esp:
            del st.stack[k]
    st.pushes = [p for p in st.pushes if p[1] != 'arg']


CC = {'je': 'eq', 'jne': 'ne', 'jl': 'lt', 'jge': 'ge', 'jg': 'gt', 'jle': 'le', 'jb': 'ult', 'jae': 'uge',
      'ja': 'ugt', 'jbe': 'ule', 'js': 'neg', 'jns': 'nneg'}
NEG = {'eq': 'ne', 'ne': 'eq', 'lt': 'ge', 'ge': 'lt', 'gt': 'le', 'le': 'gt', 'ult': 'uge', 'uge': 'ult',
       'ugt': 'ule', 'ule': 'ugt', 'neg': 'nneg', 'nneg': 'neg'}


def cond_of(st, cc):
    f = st.flags
    if f is None:
        raise Reject('unknown flags')
    kind = f[0]
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


MAXPATHS = 48
MAXINSN = 400


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
        if mn == 'nop':
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
                st.flags = None
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
        elif mn in ('test', 'cmp'):
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
            push(st, v, kind)
        elif mn == 'pop':
            v = pop(st)
            o = ops[0]
            if o.type != X.X86_OP_REG:
                raise Reject('pop to memory')
            setreg(st, o.reg, v)
        elif mn == 'call':
            o = ops[0]
            if o.type == X.X86_OP_IMM:
                tgt = o.imm & M32
                if fn.inside(tgt):
                    raise Reject('call into the body')
                do_call(fn, st, out, ('d', tgt), nxt)
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
        elif X87_EXT and mn in X87_MN:
            x87_step(st, fn, out, i)
        elif mn == 'fld':
            if not ops or ops[0].type != X.X86_OP_MEM or ops[0].size != 4:
                raise Reject('fld form')
            st.st0 = load(st, fn, out, ops[0], 4)
            pc = nxt
            j = insn_at(pc)
            if j.mnemonic != 'ret':
                raise Reject('fld not followed by ret')
            continue
        else:
            raise Reject('insn %s' % mn)
        pc = nxt


# ---------------------------------------------------------------- x87 subset (GEN-ALL only)

# Off for `ai_patterns.py` (its output stays what AI-GEN verified); `gen_main` turns it on. The subset:
# `fld`/`fild` dword from memory, `fld1`/`fldz`, `fld st(i)`, `fchs`, `fadd`/`fsub`/`fsubr`/`fmul`/`fdiv`/
# `fdivr` with a dword memory operand (the X87 `_m` forms: a signalling-NaN memory operand is not a
# register operand), the commutative register forms `fadd`/`fmul`/`faddp`/`fmulp` with ST0, `fxch st(1)`,
# `fst`/`fstp` dword to memory (X87::to_f32, PC=24 as the game runs), `fstp st(0)`. The stack must be empty
# at every call and hold at most the ST0 result at `ret`; any other x87 instruction rejects.
X87_EXT = False
# GEN-ALL: constants >= 0x8000_0000 carry a `_u32` suffix (a bare literal cast `as i32` overflows).
BIG_SUFFIX = False
X87_MN = {'fld', 'fild', 'fld1', 'fldz', 'fchs', 'fadd', 'fsub', 'fsubr', 'fmul', 'fdiv', 'fdivr', 'faddp',
          'fmulp', 'fxch', 'fst', 'fstp'}


def fpu_get(st, k):
    if k >= len(st.fpu):
        raise Reject('x87 stack underflow')
    return st.fpu[-1 - k]


def x87_step(st, fn, out, i):
    mn, ops = i.mnemonic, i.operands
    sti = [o for o in ops if o.type == X.X86_OP_REG]
    mem = [o for o in ops if o.type == X.X86_OP_MEM]

    def streg(o):
        n = o.reg - X.X86_REG_ST0
        if not 0 <= n <= 7:
            raise Reject('x87 register')
        return n

    def m32():
        if len(mem) != 1 or mem[0].size != 4:
            raise Reject('x87 %s operand size' % mn)
        return load(st, fn, out, mem[0], 4)
    if len(st.fpu) > 6:
        raise Reject('x87 stack overflow')
    if mn == 'fld':
        if mem:
            st.fpu.append(('fx', 'ld', m32()))
        else:
            st.fpu.append(fpu_get(st, streg(sti[0])))
    elif mn == 'fild':
        st.fpu.append(('fx', 'ild', m32()))
    elif mn == 'fld1':
        st.fpu.append(('fx', 'one'))
    elif mn == 'fldz':
        st.fpu.append(('fx', 'zero'))
    elif mn == 'fchs':
        st.fpu[-1] = ('fx', 'chs', fpu_get(st, 0))
    elif mn in ('fadd', 'fsub', 'fsubr', 'fmul', 'fdiv', 'fdivr') and mem:
        a = fpu_get(st, 0)
        st.fpu[-1] = ('fx', mn + '_m', a, m32())
    elif mn in ('fadd', 'fmul') and len(sti) in (1, 2):
        # capstone: `fmul st(i)` = ST0 *= ST(i); `fmul st(i), st(0)` = ST(i) *= ST0
        d, s_ = (0, streg(sti[0])) if len(sti) == 1 else (streg(sti[0]), streg(sti[1]))
        if 0 not in (d, s_):
            raise Reject('x87 operands')
        v = ('fx', mn, fpu_get(st, d), fpu_get(st, s_))
        st.fpu[-1 - d] = v
    elif mn in ('faddp', 'fmulp') and len(sti) in (1, 2):
        d = streg(sti[0])
        if len(sti) == 2 and streg(sti[1]) != 0:
            raise Reject('x87 operands')
        if d == 0:
            raise Reject('x87 %s st(0)' % mn)
        v = ('fx', mn[:-1], fpu_get(st, d), fpu_get(st, 0))
        st.fpu[-1 - d] = v
        st.fpu.pop()
    elif mn == 'fxch':
        regs = [streg(o) for o in sti if streg(o) != 0] or [1]
        d = regs[0]
        a, b = fpu_get(st, 0), fpu_get(st, d)
        st.fpu[-1], st.fpu[-1 - d] = b, a
    elif mn in ('fst', 'fstp'):
        if mem:
            if mem[0].size != 4:
                raise Reject('x87 %s size' % mn)
            v = fpu_get(st, 0)
            store(st, fn, out, mem[0], ('f2u', v))
        elif mn == 'fstp' and sti and streg(sti[0]) == 0:
            fpu_get(st, 0)
        else:
            raise Reject('x87 %s form' % mn)
        if mn == 'fstp':
            st.fpu.pop()
    else:
        raise Reject('x87 %s form' % mn)


def do_call(fn, st, out, target, nxt, obj=None, vload=None):
    if st.fpu:
        raise Reject('x87 stack not empty at a call')
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
    for _ in range(4):
        nxt_i = insn_at(q)
        if nxt_i is None:
            break
        if nxt_i.mnemonic == 'add' and nxt_i.op_str.startswith('esp, '):
            cleanup = int(nxt_i.op_str[5:], 16)
            break
        if nxt_i.mnemonic == 'pop' and nxt_i.op_str == 'ecx':
            cleanup = 4
            break
        # only register moves that do not touch ESP may sit between a call and its cleanup
        if not (nxt_i.mnemonic == 'mov' and len(nxt_i.operands) == 2 and
                all(o.type == X.X86_OP_REG for o in nxt_i.operands) and 'esp' not in nxt_i.op_str):
            break
        q += nxt_i.size
    outs = outstanding(st)
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
            if uses_ecx:
                raise Reject('caller-cleaned callee reads ECX')
        else:
            nargs = 0
            conv = 'thiscall' if uses_ecx or not has_unknown(st.r['ecx']) else 'cdecl'
    if len(outs) != nargs:
        raise Reject('outstanding pushes %d != %d arguments' % (len(outs), nargs))
    # the arguments must be exactly the outstanding pushes, at the top of the stack
    if nargs and sorted(o[0] for o in outs) != [st.esp + 4 * k for k in range(nargs)]:
        raise Reject('arguments interleaved')
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
    after_call(st)


def tail(fn, st, out, tgt):
    if st.fpu:
        raise Reject('x87 stack not empty at a tail call')
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
    fn.changes |= {'ecx', 'edx'}


def switch(fn, st, out, o, pc):
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
    # ECX/EDX left as they came in (the hook may then preserve them for callers that read them)
    if not st.r['ecx'] == ('this',):
        fn.changes.add('ecx')
    if not st.r['edx'] == ('in', 'edx'):
        fn.changes.add('edx')
    fn.rets.add(n)
    fn.argmax = max(getattr(fn, 'argmax', -1), st.argmax)
    if st.fpu:
        if len(st.fpu) != 1 or st.st0 is not None:
            raise Reject('x87 stack depth %d at ret' % len(st.fpu))
        fn.retkinds.add('x87')
        out.append(('ret', st.fpu[0], 'x87'))
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
    fn.seh_pcs = set()
    hd = list(md.disasm(rd(body, 24), body))[:3]
    if len(hd) == 3 and hd[0].mnemonic == 'push' and hd[0].operands[0].type == X.X86_OP_IMM and hd[0].operands[0].imm & M32 == M32 and hd[1].mnemonic == 'push' and \
            hd[1].operands[0].type == X.X86_OP_IMM and hd[2].op_str == 'eax, dword ptr fs:[0]':
        fn.seh_pcs = {hd[0].address, hd[1].address}
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

    def e(self, x):
        k = x[0]
        if k == 'c':
            if BIG_SUFFIX and x[1] & 0x80000000:
                return hexlit(x[1]) + '_u32'
            return hexlit(x[1])
        if k == 'this':
            return 'this'
        if k == 'arg':
            return 'a%d' % x[1]
        if k == 'v':
            return 'v%d' % x[1]
        if k == 'sx':
            t = {1: 'i8', 2: 'i16'}[x[1]]
            return '(%s as %s as i32 as u32)' % (paren(self.e(x[2])), t)
        if k == 'cond':
            return 'u32::from(%s)' % self.cond(x[1])
        if k == 'f2u':
            return '%s.to_f32().to_bits()' % self.fx(x[1])
        if k == 'bin':
            op, a, b = x[1], x[2], x[3]
            ea, eb = paren(self.e(a)), paren(self.e(b))
            if is_c(a) and op in ('add', 'sub', 'mul') and not ea.endswith('_u32'):
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

    def fx(self, x):
        """An x87 register value (X87_EXT) as a `sa_math::x87::X87` expression."""
        k = x[1]
        if k == 'ld':
            return 'X87::from_f32(f32::from_bits(%s))' % self.e(x[2])
        if k == 'ild':
            return 'X87::from_i32(%s as i32)' % paren(self.e(x[2]))
        if k == 'one':
            return 'X87::one()'
        if k == 'zero':
            return 'X87::zero()'
        if k == 'chs':
            return '%s.fchs()' % self.fx(x[2])
        if k in ('fadd_m', 'fsub_m', 'fsubr_m', 'fmul_m', 'fdiv_m', 'fdivr_m'):
            return '%s.%s(f32::from_bits(%s))' % (self.fx(x[2]), k, self.e(x[3]))
        if k in ('fadd', 'fmul'):
            return '%s.%s(%s)' % (self.fx(x[2]), k, self.fx(x[3]))
        raise Reject('emit x87 %r' % (k,))

    def cond(self, c):
        op, a, b, size = c
        ea, eb = self.e(a), self.e(b)
        ty = {1: ('u8', 'i8'), 2: ('u16', 'i16'), 4: ('u32', 'i32')}[size]
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
            elif s[0] == 'call':
                _, n, target, conv, this, args, vload = s
                fname = callee_name(target, conv, len(args))
                te = None if this is None else (self.e(this) if target[0] == 'd' else 'p(%s)' % self.e(this))
                al = ([te] if te is not None else []) + [self.e(a) for a in args]
                ce = '%s(%s)' % (fname, ', '.join(al))
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
                    out.append('%sreturn %s as u8;' % (pad, paren(self.e(e))))
                elif rk == 'x87' and e[0] == 'fx':
                    out.append('%sreturn %s;' % (pad, self.fx(e)))
                elif rk == 'x87':
                    out.append('%sreturn x(f32::from_bits(%s));' % (pad, self.e(e)))
                else:
                    out.append('%sreturn %s;' % (pad, self.e(e)))
        return out


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


def nm_static(name):
    """Free functions and static initialisers (no object): cdecl when they take nothing."""
    return '::' not in name or name.startswith('__') or method_of(name).startswith('__')


def flatten(ss):
    for s in ss:
        yield s
        if s[0] == 'if':
            yield from flatten(s[2])
            yield from flatten(s[3])


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
def fnv(data):
    h = 0xcbf29ce484222325
    for b in data:
        h ^= b
        h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return h


def merge_spans(rng):
    out = []
    for pc, sz in rng:
        if out and out[-1][0] + out[-1][1] == pc:
            out[-1] = (out[-1][0], out[-1][1] + sz)
        else:
            out.append((pc, sz))
    return out



# ================================================================ driver: every subsystem but AI

GEN_MARK = 'Generated by `tools/gen/patterns.py`'
REPORT_DIR = os.path.join(ROOT, 'target', 'gen-patterns')
# The checkout holding the gitignored data (catalog/raw coverage): SA_ROOT, else the main checkout.
DATA_ROOT = os.environ.get('SA_ROOT') or ROOT.split('/.claude/worktrees/')[0]

# Subsystem (procedures.csv, or the RE-SPEC-UNK proposal for UNK-game rows) -> crate.
SUBSYS_CRATE = {
    'CORE': 'sa-core', 'STREAM': 'sa-stream', 'WORLD': 'sa-world', 'COLL': 'sa-world', 'PHYS': 'sa-world',
    'POP': 'sa-pop', 'SCRIPT': 'sa-script', 'WEAPON': 'sa-weapon', 'PED': 'sa-ped', 'INPUT': 'sa-input',
    'ANIM': 'sa-anim', 'VEH': 'sa-veh', 'HUD': 'sa-hud', 'CAM': 'sa-cam', 'AUDIO': 'sa-audio',
    'RW': 'sa-rwcore', 'RENDER': 'sa-render',
}
# The AI families stay with `ai_patterns.py` / sa-ai, whatever subsystem the catalogue gives them.
AI_NAMES = re.compile(r'^(CTask|CEvent|CPedGroup|CDecisionMaker|CDecision|CPedAttractor|C\w*Attractor\b|'
                      r'CTaskAllocator|CInformFriendsEventQueue|CInformGroupEventQueue|CPedIntelligence|'
                      r'CAcquaintance|CEntitySeekPos)')

FAMILIES = ['const', 'const-bool', 'empty', 'getter', 'getter-f32', 'setter', 'leaf', 'ctor', 'dtor',
            'pool', 'static-init', 'thunk', 'small']
# Output file of a function whose content class has fewer than CLASS_MIN generated functions in its crate
# (static initialisers and other free functions always): one file per pattern family.
GFAMILY_FILE = {
    'const': 'generated_leaves', 'const-bool': 'generated_leaves', 'empty': 'generated_leaves',
    'getter': 'generated_leaves', 'getter-f32': 'generated_leaves', 'setter': 'generated_setters',
    'leaf': 'generated_leaf_logic', 'ctor': 'generated_ctors', 'dtor': 'generated_dtors',
    'pool': 'generated_pool', 'static-init': 'generated_inits', 'thunk': 'generated_thunks',
    'small': 'generated_small',
}
GFILE_DOC = {
    'generated_leaves': 'constant returns, constant `bool`s, empty bodies and field getters (no call, no store)',
    'generated_setters': 'field and global setters (stores only: no call, no branch)',
    'generated_leaf_logic': 'leaf functions with branches (no call)',
    'generated_ctors': 'constructors (field stores, vtable store, base constructor calls)',
    'generated_dtors': 'complete and scalar deleting destructors, `atexit` destructors of globals',
    'generated_pool': '`operator new` / `operator delete` of pooled classes',
    'generated_inits': 'static initialisers of globals (`__init_*`)',
    'generated_thunks': 'thunks and forwarders (one call, its result returned)',
    'generated_small': 'small functions with calls within the interpreter subset',
}
# A content class (C++ class, or the RenderWare object of an `Rw*`/`Rp*`/`Rt*` function) with at least this
# many generated functions in a crate gets its own file `<class>_generated.rs`.
CLASS_MIN = 3
RW_OBJ = re.compile(r'^_?((?:Rw|Rp|Rt|Rx|Rs)[A-Z][a-z0-9]+)')
# Writer/verifier split (docs/verify/HOWTO.md): every generated hook is unverified, in batches of at most
# BATCH_MAX hooks of one subsystem, `BATCH_PREFIX-<subsystem>-<n>`.
BATCH_PREFIX = 'gen2'
BATCH_MAX = 60
# Calls and statements a `small` function may have.
SMALL_CALLS = 4
SMALL_STMTS = 40
L1_SPECS = os.path.join(ROOT, 'crates', 'sa-oracle', 'src', 'specs_generated.rs')


def read_kv(path):
    out = {}
    if os.path.exists(path):
        for l in open(path):
            l = l.split('#', 1)[0].strip()
            if l:
                k, _, v = l.partition(' ')
                out[int(k, 16)] = v.strip()
    return out


# Hooks whose original callers read ECX/EDX after the call (sa-hook `callers` test output, `ADDR regs`):
# `preserve = regs` when every path of the original leaves those registers unchanged, else not hooked.
GCALLERS = read_kv(os.path.join(ROOT, 'tools', 'gen', 'patterns_callers.txt'))
# Generated but not registered (`ADDR reason`), e.g. sa-hook `patch_refs` findings.
GUNHOOK = read_kv(os.path.join(ROOT, 'tools', 'gen', 'patterns_unhook.txt'))


def ghookable(a, fn):
    """(True | reason, preserve registers or None)."""
    h = hookable(a, fn)
    if h is not True:
        return h, None
    if a in GUNHOOK:
        return GUNHOOK[a], None
    if a in GCALLERS:
        regs = GCALLERS[a]
        if set(regs.split(',')) & fn.changes:
            return ('original callers read %s after the call and the original changes it (sa-hook `callers`)'
                    % regs.replace(',', '/').upper()), None
        return True, regs
    return True, None


def load_proposal():
    out = {}
    path = os.path.join(ROOT, 'catalog', 'unk-classification-proposal.csv')
    if os.path.exists(path):
        for r in csv.DictReader(open(path)):
            out[int(r['address'], 16)] = r['proposed_subsystem']
    return out


def load_state():
    out = {}
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'state.csv'))):
        out[int(r['address'], 16)] = (r['state'], r['evidence'])
    return out


def load_coverage():
    """address -> recordings whose INF-10 coverage (`catalog/raw/coverage-KEY.csv` of the data checkout)
    shows a call, the same files `tools/verify/queue.py` reads."""
    out = {}
    d = os.path.join(DATA_ROOT, 'catalog', 'raw')
    if not os.path.isdir(d):
        return out
    for f in sorted(os.listdir(d)):
        if not (f.startswith('coverage-') and f.endswith('.csv')):
            continue
        key = f[len('coverage-'):-len('.csv')]
        if key.endswith('.hits') or key == 'all' or key.startswith('rust-'):
            continue
        try:
            for r in csv.DictReader(open(os.path.join(d, f))):
                if int(r.get('hits') or 0) > 0:
                    out.setdefault(int(r['address'], 16), []).append(key)
        except (KeyError, ValueError):
            continue
    return out


def audio_exclusions():
    """AUDIO-GEN's scope (`tools/gen/audio_patterns.py`): the sound output/decoding classes it leaves
    original, its excluded and ported-unhooked addresses and its skips (read from the script's source
    without importing it)."""
    import ast
    path = os.path.join(ROOT, 'tools', 'gen', 'audio_patterns.py')
    addrs, cls = set(), None
    if not os.path.exists(path):
        return addrs, cls
    for node in ast.parse(open(path).read()).body:
        if not isinstance(node, ast.Assign) or len(node.targets) != 1 or not isinstance(node.targets[0], ast.Name):
            continue
        k = node.targets[0].id
        if k in ('EXCLUDE_ADDR', 'PORTED_UNHOOKED', 'SKIP'):
            v = ast.literal_eval(node.value)
            addrs |= set(v)
        elif k == 'EXCLUDE_CLASS':
            cls = re.compile(ast.literal_eval(node.value.args[0]))
    return addrs, cls


def ported(skip_mark):
    """Addresses registered anywhere in `crates/` (`#[hook(addr` or `#[native(addr`, also over several
    lines), outside files carrying `skip_mark` in their head (the running generator's own output), plus
    every ``/// `Name` (0xADDR)`` doc line of sa-ai's hand ports (PORT-AI-1 ported-unhookable functions)."""
    done = set()
    for dirpath, dirs, files in os.walk(os.path.join(ROOT, 'crates')):
        dirs[:] = sorted(d for d in dirs if d != 'target')
        for f in sorted(files):
            if not f.endswith('.rs'):
                continue
            s = open(os.path.join(dirpath, f), encoding='utf-8').read()
            if skip_mark in s[:400]:
                continue
            for m in re.finditer(r'^[ \t]*#\[(?:[\w:]+::)?(?:hook|native)\(\s*addr\s*=\s*0x([0-9A-Fa-f_]+)', s, re.M):
                done.add(int(m.group(1).replace('_', ''), 16))
            if os.sep + 'sa-ai' + os.sep in dirpath + os.sep and not f.startswith('gen_'):
                for m in re.finditer(r'^/// `[^`]*` \(0x([0-9A-Fa-f]+)', s, re.M):
                    done.add(int(m.group(1), 16))
    return done


def generated_addrs(mark):
    """Addresses registered in the files carrying `mark` in their head (this generator's own output)."""
    out = set()
    for dirpath, dirs, files in os.walk(os.path.join(ROOT, 'crates')):
        dirs[:] = sorted(d for d in dirs if d != 'target')
        for f in sorted(files):
            if not f.endswith('.rs'):
                continue
            s = open(os.path.join(dirpath, f), encoding='utf-8').read()
            if mark not in s[:400]:
                continue
            for m in re.finditer(r'^[ \t]*#\[(?:[\w:]+::)?(?:hook|native)\(\s*addr\s*=\s*0x([0-9A-Fa-f_]+)', s, re.M):
                out.add(int(m.group(1).replace('_', ''), 16))
    return out


def read_list(path):
    out = set()
    if os.path.exists(path):
        for l in open(path):
            l = l.split('#')[0].strip()
            if l:
                out.add(int(l, 16))
    return out


def gfamily(name, fn):
    m = method_of(name)
    c = cls_of(name)
    flat = list(flatten(fn.out))
    ncalls = sum(1 for s in flat if s[0] == 'call')
    nstores = sum(1 for s in flat if s[0] == 'store')
    nifs = sum(1 for s in flat if s[0] == 'if')
    rets = [s for s in flat if s[0] == 'ret']
    const_ret = all(s[1] is not None and is_c(s[1]) for s in rets)
    if is_dtor_name(m) or m == "`scalar deleting destructor'" or 'deleting destructor' in m:
        return 'dtor'
    if m.startswith('operator new') or m.startswith('operator delete'):
        return 'pool'
    if c and (m in ('Constructor', c) or m == c.split('<')[0] or m.startswith('Constructor')):
        return 'ctor'
    if (name or '').startswith('__init_'):
        return 'static-init'
    if ncalls == 0 and nstores == 0 and nifs == 0 and len(rets) == 1:
        r = rets[0]
        if fn.ret == 'u32' and const_ret:
            return 'const'
        if fn.ret == 'u8' and const_ret:
            return 'const-bool'
        if fn.ret == 'void':
            return 'empty'
        if fn.ret == 'x87':
            return 'getter-f32'
        return 'getter'
    if ncalls == 0 and nifs == 0 and nstores >= 1 and all(s[0] in ('load', 'store', 'ret', 'let') for s in flat):
        return 'setter'
    vt_stores = [s for s in flat if s[0] == 'store' and s[2] == ('this',) and s[3] == 0 and is_c(s[4])]
    if vt_stores and fn.ret == 'u32' and rets and all(s[1] == ('this',) for s in rets):
        return 'ctor'
    if vt_stores and ncalls >= 1 and fn.ret in ('void', 'u32'):
        return 'dtor'
    if ncalls == 0:
        return 'leaf'
    if ncalls == 1 and nstores == 0 and nifs == 0:
        call = [s for s in flat if s[0] == 'call'][0]
        if all(s[0] in ('call', 'ret', 'load', 'let') for s in flat) and \
                all(s[1] is None or s[1] == ('v', call[1]) for s in rets):
            return 'thunk'
    if ncalls <= SMALL_CALLS and len(flat) <= SMALL_STMTS:
        return 'small'
    return None


def x87_ops(e):
    if not isinstance(e, tuple):
        return 0
    n = 1 if e[0] == 'fx' and e[1] in ('fadd_m', 'fsub_m', 'fsubr_m', 'fmul_m', 'fdiv_m', 'fdivr_m',
                                       'fadd', 'fmul') else 0
    return n + sum(x87_ops(x_) for x_ in e[1:] if isinstance(x_, tuple))


def max_x87_ops(out):
    m = 0
    for s_ in flatten(out):
        for x_ in s_[1:]:
            if isinstance(x_, tuple):
                m = max(m, x87_ops(x_))
    return m




def gen_candidates(done):
    proposal = load_proposal()
    audio_addrs, audio_cls = audio_exclusions()
    out = []
    for a in sorted(PROCS):
        r = PROCS[a]
        sub = r['subsystem']
        if sub == 'UNK-game':
            sub = proposal.get(a, '')
        crate = SUBSYS_CRATE.get(sub)
        if crate is None or a in done:
            continue
        name = catname(a) or 'FUN_%08x' % a
        if AI_NAMES.match(name):
            continue
        if sub == 'AUDIO' and (a in audio_addrs or (audio_cls is not None and audio_cls.match(cls_of(name)))):
            continue
        out.append((a, name, sub, crate))
    return out


def content_class(name):
    c = cls_of(name)
    if c:
        return c
    m = RW_OBJ.match(name or '')
    return m.group(1) if m else ''


def snake(c):
    c = re.sub(r'<.*', '', c)
    c = re.sub(r'^C(?=[A-Z])', '', c)
    c = re.sub(r'[^A-Za-z0-9]+', '_', c)
    c = re.sub(r'([A-Z]+)([A-Z][a-z])', r'\1_\2', c)
    c = re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', c)
    c = re.sub(r'_+', '_', c).strip('_').lower()
    if not c or c[0].isdigit():
        c = 'c_' + c
    return c


def has_x87(e):
    if not isinstance(e, tuple):
        return False
    if e[0] in ('fx', 'f2u'):
        return True
    return any(has_x87(x_) for x_ in e[1:] if isinstance(x_, tuple))


def route_hazards(fn, fam, name, pres):
    """The verifier hints (docs/verify/HOWTO.md §1) from what the interpreter executed.

    route: `L1-pure` when the body reads, writes and calls nothing (only its arguments and constants)
    and uses no x87 (an L1 spec in `specs_generated.rs` drives it); `split` for one-time code (static
    initialisers and `atexit` destructors of globals run once per process); `L2` otherwise.
    hazards: `preserved-regs` (declared `preserve`), `ax-al-return` (an AL result with the caller's
    upper EAX), `heap-writes` (a store through a pointer, not to a fixed global), else `none`."""
    flat = list(flatten(fn.out))
    kinds = {s_[0] for s_ in flat}
    haz = []
    if pres:
        haz.append('preserved-regs')
    if fn.ret == 'u8':
        haz.append('ax-al-return')
    if any(s_[0] == 'store' and s_[2] is not None for s_ in flat):
        haz.append('heap-writes')
    x87 = fn.ret == 'x87' or any(has_x87(x_) for s_ in flat for x_ in s_[1:] if isinstance(x_, tuple))
    if not kinds & {'load', 'store', 'call'} and not x87:
        route = 'L1-pure'
    elif fam == 'static-init' or method_of(name).startswith('__dtor_') or (name or '').startswith('__dtor_'):
        route = 'split'
    else:
        route = 'L2'
    return route, ','.join(haz) or 'none'


def gen_main():
    global X87_EXT, BIG_SUFFIX
    X87_EXT = True
    BIG_SUFFIX = True
    check = '--check' in sys.argv
    done = ported(GEN_MARK)
    inflight = read_list(os.path.join(ROOT, 'tools', 'gen', 'exclude_inflight.txt'))
    state = load_state()
    own = generated_addrs(GEN_MARK)
    skipped_state = set()
    for a, (stt, ev) in state.items():
        # verified or ported elsewhere (also ported-unhookable rows) or excluded, not by this generator; a
        # row the catalogue records as `implemented` for this generator's own hook (an integration's
        # "written on master" evidence carries no GEN2 tag) is still this generator's
        if a in own and stt == 'implemented':
            continue
        if stt in ('implemented', 'L1', 'L2', 'L3', 'excluded') and 'GEN-ALL' not in ev and 'GEN2' not in ev:
            skipped_state.add(a)
    cand = gen_candidates(done | inflight | skipped_state)
    gens = []
    rest = []
    seen = set()
    for a, name, sub, crate in cand:
        try:
            fn = analyse(a, name)
            fam = gfamily(name, fn)
            if fam is None:
                raise Reject('not a generator family')
            if fam == 'static-init' and max_x87_ops(fn.out) > 1:
                # CRT initialisers run before D3D9 sets precision control 24: a chain of x87 ops would be
                # rounded at 53 bits there, X87 models 24 (one op then `fstp dword` is the same either way)
                raise Reject('x87 chain in a static initialiser (precision control 53 at CRT init)')
            if fam == 'dtor' and fn.ret == 'u32':
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
            rest.append((a, name, sub, str(ex)))
            continue
        except RecursionError:
            rest.append((a, name, sub, 'recursion'))
            continue
        fid = ident(name, a)
        while fid in seen:
            fid += '_b'
        seen.add(fid)
        h, pres = ghookable(a, fn)
        route, haz = route_hazards(fn, fam, name, pres)
        if pres and route != 'L1-pure':
            # sa-oracle `preserved_registers_match_the_hook_registrations`: a `preserve` hook needs an L1
            # spec comparing the register, and specs are generated for L1-pure bodies only
            h, pres = ('original callers read %s after the call; `preserve` needs an L1 spec comparing it, '
                       'generated for L1-pure bodies only' % GCALLERS[a].replace(',', '/').upper()), None
            route, haz = route_hazards(fn, fam, name, pres)
        gens.append({'a': a, 'name': name, 'fid': fid, 'fn': fn, 'fam': fam, 'body': strip_returns(body),
                     'sub': sub, 'crate': crate, 'hook': h, 'pres': pres, 'route': route, 'hazards': haz,
                     'batch': None, 'recs': []})
    # files: a content class with CLASS_MIN functions in a crate gets its own file, the rest one per family
    by_cls = {}
    for g in gens:
        by_cls.setdefault((g['crate'], content_class(g['name'])), []).append(g)
    for (crate, cls), gs in by_cls.items():
        for g in gs:
            if cls and len(gs) >= CLASS_MIN:
                g['file'] = snake(cls) + '_generated'
                g['cls'] = cls
            else:
                g['file'] = GFAMILY_FILE[g['fam']]
                g['cls'] = None
    # batches: per subsystem, in file then address order, at most BATCH_MAX hooks each (evenly split)
    cov = load_coverage()
    by_sub = {}
    for g in gens:
        if g['hook'] is True:
            by_sub.setdefault(g['sub'], []).append(g)
            g['recs'] = cov.get(g['a'], [])
    for sub, gs in sorted(by_sub.items()):
        gs.sort(key=lambda g: (g['crate'], g['file'], g['a']))
        nb = -(-len(gs) // BATCH_MAX)
        size = -(-len(gs) // nb)
        for k, g in enumerate(gs):
            g['batch'] = '%s-%s-%d' % (BATCH_PREFIX, sub.lower(), k // size + 1)
    per_crate = {}
    for g in gens:
        per_crate.setdefault(g['crate'], {}).setdefault(g['file'], []).append(g)
    files = {}
    report = []
    for crate in sorted(set(SUBSYS_CRATE.values())):
        fams = per_crate.get(crate, {})
        src = os.path.join(ROOT, 'crates', crate, 'src')
        mods = []
        for f in sorted(fams):
            recs = sorted(fams[f], key=lambda g: g['a'])
            path = os.path.join(src, f + '.rs')
            if os.path.exists(path) and GEN_MARK not in open(path).read()[:400]:
                raise SystemExit('%s exists and is not generated by this script' % os.path.relpath(path, ROOT))
            files[path] = grender(f, recs, report, crate)
            mods.append((f, recs))
        path = os.path.join(src, 'generated.rs')
        if os.path.exists(path) and GEN_MARK not in open(path).read()[:400]:
            raise SystemExit('%s exists and is not generated by this script' % os.path.relpath(path, ROOT))
        files[path] = grender_hub(crate, mods)
    files[L1_SPECS] = grender_specs([g for g in gens if g['hook'] is True and g['route'] == 'L1-pure'])
    bad = []
    for path, text in sorted(files.items()):
        text = rustfmt(text)
        if check:
            old = open(path).read() if os.path.exists(path) else None
            if old != text:
                bad.append(os.path.relpath(path, ROOT))
        else:
            with open(path, 'w') as fh:
                fh.write(text)
    # stale generated files (a family or class that is now empty, or an older generator's names)
    for crate in sorted(set(SUBSYS_CRATE.values())):
        src = os.path.join(ROOT, 'crates', crate, 'src')
        for f in sorted(os.listdir(src)):
            p_ = os.path.join(src, f)
            if f.endswith('.rs') and p_ not in files and GEN_MARK in open(p_).read()[:400]:
                if check:
                    bad.append(os.path.relpath(p_, ROOT))
                else:
                    os.remove(p_)
    os.makedirs(REPORT_DIR, exist_ok=True)
    with open(os.path.join(REPORT_DIR, 'report.tsv'), 'w') as fh:
        print('address', 'name', 'ident', 'family', 'hook', 'subsystem', 'crate', 'file', 'entry', 'conv',
              'stack_args', 'ret', 'calls', 'stores', 'batch', 'route', 'hazards', sep='\t', file=fh)
        for row in sorted(report, key=lambda r: int(r[0], 16)):
            print(*row, sep='\t', file=fh)
    with open(os.path.join(REPORT_DIR, 'rest.tsv'), 'w') as fh:
        print('address', 'name', 'subsystem', 'reason', sep='\t', file=fh)
        for a, n, sub, why in rest:
            print('0x%X' % a, n, sub, why, sep='\t', file=fh)
    counts = {}
    for row in report:
        k = (row[5], row[3], row[4] == 'hook')
        counts[k] = counts.get(k, 0) + 1
    with open(os.path.join(REPORT_DIR, 'counts.txt'), 'w') as fh:
        for k in sorted(counts):
            print('%-8s %-12s %-10s %d' % (k[0], k[1], 'hook' if k[2] else 'unhookable', counts[k]), file=fh)
        reasons = {}
        for a, n, sub, why in rest:
            w = re.sub(r'0x[0-9a-fA-F]+', 'X', why)
            reasons[w] = reasons.get(w, 0) + 1
        print('\nrejected, by reason:', file=fh)
        for w, n in sorted(reasons.items(), key=lambda kv: (-kv[1], kv[0])):
            print('%6d  %s' % (n, w), file=fh)
    batches = sorted({g['batch'] for g in gens if g['batch']})
    print('candidates', len(cand), 'generated', len(report), 'hooks', sum(1 for r in report if r[4] == 'hook'),
          'L1-pure', sum(1 for g in gens if g['hook'] is True and g['route'] == 'L1-pure'),
          'batches', len(batches), 'rest', len(rest), 'excluded in-flight', len(inflight))
    print('report:', os.path.relpath(REPORT_DIR, ROOT) + '/{report,rest}.tsv, counts.txt')
    if bad:
        print('differs:', ', '.join(bad))
        sys.exit(1)


def rustfmt(text):
    """The repository's rustfmt (edition 2024, `rustfmt.toml`) over a generated file, so the files on disk
    are exactly what `cargo fmt` would leave and `--check` compares formatted text."""
    import subprocess
    r = subprocess.run(['rustfmt', '--edition', '2024', '--config-path', os.path.join(ROOT, 'rustfmt.toml'),
                        '--emit', 'stdout'], input=text, capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit('rustfmt failed:\n' + r.stderr[:2000])
    return r.stdout


GEN_ALLOW = ('#![allow(clippy::unreadable_literal, clippy::identity_op, clippy::needless_return)]\n'
             '#![allow(clippy::let_and_return, clippy::too_many_arguments, clippy::cast_lossless)]\n'
             '#![allow(clippy::precedence, clippy::collapsible_else_if, clippy::if_same_then_else)]\n'
             '#![allow(unused_parens, clippy::double_parens, clippy::nonminimal_bool, clippy::eq_op)]\n'
             '#![allow(clippy::unnecessary_cast, clippy::erasing_op, clippy::neg_multiply)]\n')


def grender(fname, recs, report, crate):
    callees = {}
    parts = []
    hashes = []
    consts = []
    for g in recs:
        a, name, fid, fn, fam, body, sub = g['a'], g['name'], g['fid'], g['fn'], g['fam'], g['body'], g['sub']
        for (target, conv, n) in fn.calls:
            callees[(target, conv, n)] = True
        h, pres_regs = g['hook'], g['pres']
        rng = sorted(fn.insns.items())
        spans = ([(a, 5)] if fn.stub else []) + merge_spans(rng) + list(getattr(fn, 'tables', []))
        data = b''.join(rd(x_, n) for x_, n in spans)
        hashes.append((a, fid, fnv(data), spans))
        if fam in ('const', 'const-bool', 'empty') and fn.ret != 'x87':
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
        doc = '/// `%s` (0x%X%s, %s): %s, %s, `ret%s`; %d instruction(s) matched.\n' % (
            name, a, ', stub to .HOODLUM 0x%X' % fn.body if fn.stub else '', sub, fam, fn.conv,
            ' %d' % fn.pop if fn.pop else '', len(rng))
        s = doc
        if h is True:
            exp, _ = expect_of(a)
            pres = ', preserve = "%s"' % pres_regs if pres_regs else ''
            hints = ', unverified = "%s", route = "%s", hazards = "%s"' % (g['batch'], g['route'], g['hazards'])
            if g['recs']:
                hints += ', recs = "%s"' % ','.join(g['recs'])
            s += '#[hook(addr = 0x%X, conv = %s, name = "%s", expect = "%s"%s%s)]\n' % (
                a, fn.conv, name, exp, pres, hints)
        else:
            s += '/// Not hookable: %s.\n#[allow(dead_code)]\n' % h
        s += 'pub fn %s(%s)%s {\n' % (fid, ', '.join(args), ret)
        s += '\n'.join(body) + ('\n' if body else '')
        s += '}\n'
        parts.append(s)
        flat = list(flatten(fn.out))
        report.append(('0x%X' % a, name, fid, fam, 'hook' if h is True else 'unhookable: ' + h, sub, crate,
                       fname, 'stub' if fn.stub else 'inline', fn.conv, str(fn.nargs), fn.ret,
                       str(sum(1 for x_ in flat if x_[0] == 'call')),
                       str(sum(1 for x_ in flat if x_[0] == 'store')),
                       g['batch'] or '-', g['route'] if h is True else '-', g['hazards'] if h is True else '-'))
    cls = recs[0].get('cls')
    what = ('the generated ports of `%s` (every pattern family)' % cls) if cls else GFILE_DOC[fname]
    head = ('//! Generated by `tools/gen/patterns.py` from the 1.0 US bytes: %s.\n'
            '//! Every body below was executed instruction by instruction by the generator\'s symbolic\n'
            '//! interpreter; see the script and `docs/arch/gen-all.md`. Every hook is unverified (its batch,\n'
            '//! route and hazards are in the attribute, docs/verify/HOWTO.md). Do not edit by hand: regenerate.\n'
            % what) + GEN_ALLOW + '\n#[allow(unused_imports)]\nuse super::*;\n\n'
    s = head
    if callees:
        keys = sorted(callees, key=lambda k: (k[0][0], k[0][1], k[1], k[2]))
        for c0 in range(0, len(keys), 150):
            s += 'orig! {\n'
            for (target, conv, n) in keys[c0:c0 + 150]:
                nm = callee_name(target, conv, n)
                ps = ([('this: u32' if target[0] == 'd' else 'obj: GPtr<u8>')] if conv == 'thiscall' else []) + \
                    ['a%d: u32' % k for k in range(n)]
                if target[0] == 'd':
                    cn = catname(target[1]) or 'FUN_%08x' % target[1]
                    s += '    /// `%s` (0x%X), %s, %d stack argument(s)\n' % (cn, target[1], conv, n)
                    s += '    fn %s(%s) -> u32 = %s 0x%X => unported;\n' % (nm, ', '.join(ps), conv, target[1])
                else:
                    s += '    /// virtual slot +0x%X (`call [vtbl + 0x%X]` on ECX), %d stack argument(s)\n' % (
                        target[1], target[1], n)
                    s += '    fn %s(%s) -> u32 = thiscall vslot 0x%X => unported;\n' % (nm, ', '.join(ps), target[1])
            s += '}\n\n'
    s += '\n'.join(parts)
    s += '\n/// (address, 64-bit FNV-1a of the matched bytes, the matched spans `(va, len)`: the stub, every\n'
    s += '/// executed instruction, switch tables): the bodies this file was generated from.\n'
    s += 'pub const BODIES: &[Body] = &[\n'
    for a, fid, hv, spans in hashes:
        s += '    (0x%X, 0x%016X, &[%s]),\n' % (a, hv, ', '.join('(0x%X, %d)' % (x_, n) for x_, n in spans))
    s += '];\n'
    fams = {}
    for fam, fid, fn, body in consts:
        fams.setdefault(fam, []).append((fid, fn, body))
    if fams:
        s += '\n#[cfg(all(test, not(sa_phase_a)))]\nmod tests {\n    use super::*;\n'
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


def grender_hub(crate, mods):
    s = ('//! Generated by `tools/gen/patterns.py`: the generated ports of this crate, one module per content\n'
         '//! class or pattern family (`docs/arch/gen-all.md`), their registrations ([`HOOKS`]) and the matched\n'
         '//! bodies ([`BODIES`], checked against the exe by a host test). Do not edit by hand: regenerate.\n')
    s += GEN_ALLOW + '\n'
    s += '#[allow(unused_imports)]\nuse sa_core::abi::HookReg;\n'
    s += '#[allow(unused_imports)]\nuse sa_core::mem::{GPtr, GameData};\n'
    s += '#[allow(unused_imports)]\nuse sa_core::{hook, orig};\n'
    s += '#[allow(unused_imports)]\nuse sa_math::x87::X87;\n\n'
    for f, _ in mods:
        s += '#[path = "%s.rs"]\npub mod %s;\n' % (f, f)
    s += '''
/// A 32-bit game address as a pointer.
#[allow(dead_code)]
fn p(a: u32) -> GPtr<u8> {
    GPtr::from_addr(a as usize)
}

/// Reads field `T` at byte offset `off`.
#[allow(dead_code)]
fn rd<T: GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}

/// Writes field `T` at byte offset `off`.
#[allow(dead_code)]
fn wr<T: GameData>(p: GPtr<u8>, off: isize, v: T) {
    p.byte_add(off).cast::<T>().write(v)
}

/// A game global, addressed directly.
#[allow(dead_code)]
fn g<T: GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// `fld dword` of an f32.
#[allow(dead_code)]
fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

/// A generated body: (address, FNV-1a of the matched bytes, matched spans `(va, len)`).
pub type Body = (u32, u64, &'static [(u32, u32)]);

/// Every generated registration of this crate.
pub const HOOKS: &[HookReg] = &[
'''
    for f, recs in mods:
        for g in recs:
            if g['hook'] is True:
                s += '    %s::HOOK_%s,\n' % (f, g['fid'].upper())
    s += '];\n\n/// Every generated body of this crate.\npub const BODIES: &[&[Body]] = &[\n'
    for f, _ in mods:
        s += '    %s::BODIES,\n' % f
    s += '];\n'
    s += '''
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    extern crate std;
    use std::vec::Vec;

    fn exe() -> Option<Vec<u8>> {
        let path = std::env::var("SA_GTA_EXE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join("sa-engine/re/gta/1.0-US/gta_sa.exe")
            });
        std::fs::read(path).ok()
    }

    /// `n` bytes at virtual address `va` (section raw data; `None` outside every section).
    fn image(exe: &[u8], va: u32, n: u32) -> Option<Vec<u8>> {
        let u16_at = |o: usize| u16::from_le_bytes([exe[o], exe[o + 1]]) as usize;
        let u32_at = |o: usize| u32::from_le_bytes(exe[o..o + 4].try_into().unwrap());
        let pe = u32_at(0x3C) as usize;
        let (nsec, opt) = (u16_at(pe + 6), u16_at(pe + 20));
        let base = u32_at(pe + 24 + 28);
        for s in 0..nsec {
            let h = pe + 24 + opt + s * 40;
            let (vsize, va0, rsize, raw) = (
                u32_at(h + 8),
                base + u32_at(h + 12),
                u32_at(h + 16),
                u32_at(h + 20),
            );
            if va >= va0 && va + n <= va0 + vsize.max(rsize) {
                return Some(
                    (0..n)
                        .map(|k| {
                            let o = va + k - va0;
                            if o < rsize { exe[(raw + o) as usize] } else { 0 }
                        })
                        .collect(),
                );
            }
        }
        None
    }

    fn fnv(data: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in data {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h
    }

    /// Every generated body still hashes to what the generator matched (the exe from `$SA_GTA_EXE` or the
    /// default path; skipped without it).
    #[test]
    fn bodies_match_the_exe() {
        let Some(exe) = exe() else {
            std::eprintln!("note: gta_sa.exe not found, generated bodies not checked");
            return;
        };
        let mut bad = Vec::new();
        for (addr, hash, spans) in super::BODIES.iter().flat_map(|b| b.iter()) {
            let mut data = Vec::new();
            for (va, n) in spans.iter() {
                data.extend(
                    image(&exe, *va, *n).unwrap_or_else(|| panic!("{addr:#x}: span {va:#x} not in the image")),
                );
            }
            if fnv(&data) != *hash {
                bad.push(*addr);
            }
        }
        assert!(bad.is_empty(), "generated bodies that no longer match the exe: {bad:#x?}");
    }
}
'''
    return s



def grender_specs(gs):
    """`crates/sa-oracle/src/specs_generated.rs`: one L1 spec per generated `L1-pure` hook (no memory
    access, no call, no x87: every argument word is drawn at random and the result register compared)."""
    s = ('//! Generated by `tools/gen/patterns.py`: L1 specs of the generated `L1-pure` ports (bodies that\n'
         '//! read, write and call nothing: only their arguments and constants). Every stack argument (and\n'
         '//! `this`) is a random word; the result register (EAX, AL) and the preserved registers are compared.\n'
         '//! Do not edit by hand: regenerate.\n\n'
         '#[allow(unused_imports)]\nuse crate::call::Conv::*;\nuse crate::generate::Rng;\nuse crate::mspec::*;\n'
         '#[allow(unused_imports)]\nuse crate::specs::{ExpRet, RetKind::*, Spec};\n\n'
         '/// A small integer, a value below 0x8000 or any word.\n'
         'fn g_word(r: &mut Rng) -> u32 {\n'
         '    match r.below(3) {\n        0 => g_small_int(r),\n        1 => g_rand(r),\n'
         '        _ => r.next_u32(),\n    }\n}\n\n'
         'pub fn all() -> Vec<Spec> {\n    vec![\n')
    for g in sorted(gs, key=lambda g: g['a']):
        fn = g['fn']
        conv = {'cdecl': 'Cdecl', 'thiscall': 'Thiscall', 'stdcall': 'Stdcall'}[fn.conv]
        rk = {'u32': 'Eax', 'u8': 'Al', 'void': 'Void'}[fn.ret]
        nargs = fn.nargs + (1 if fn.conv == 'thiscall' else 0)
        path = '%s::generated::%s::%s' % (g['crate'].replace('-', '_'), g['file'], g['fid'])
        call = '%s(%s)' % (path, ', '.join('c.i(%d)' % k for k in range(nargs)))
        s += '        b(%s, %s, 0x%X, %s, %s)\n' % (json_str(g['fid']), json_str(g['name']), g['a'], conv, rk)
        s += '            .args(vec![%s])\n' % ', '.join(['A::I(g_word)'] * nargs)
        if g['pres']:
            s += '            .preserve(%s)\n' % ' | '.join('sa_core::abi::PRESERVE_%s' % r.upper()
                                                         for r in g['pres'].split(','))
        cv = 'c' if nargs else '_c'
        if fn.ret == 'void':
            s += '            .rust(|%s| {\n                %s;\n                ExpRet::Void\n            }),\n' % (cv, call)
        elif fn.ret == 'u8':
            s += '            .rust(|%s| ExpRet::Al(%s)),\n' % (cv, call)
        else:
            s += '            .rust(|%s| ExpRet::Eax(%s)),\n' % (cv, call)
    s += '    ]\n}\n'
    return s


def json_str(v):
    return '"%s"' % v.replace('\\', '\\\\').replace('"', '\\"')


if __name__ == '__main__':
    sys.setrecursionlimit(10000)
    gen_main()
