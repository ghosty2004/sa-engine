"""The original exe and the catalogue, read once for every generator (ADR-003 §3.5, `tools/lib`).

Reads `re/gta/1.0-US/gta_sa.exe` (never copied into the repo; `SA_GTA_EXE` overrides the path, a worktree
falls back to the main checkout's `re/`) and the catalogue CSVs. Offers the image (`rd`, `u32_at`), capstone
decoding (`md`, `insn_at`, `linear`), the catalogue's names and body extents (`catname`, `RANGES`,
`body_ranges`), and the callee facts the generators read off the bytes: `pops` (the callee's own `ret N`),
`entry_reads` / `ecx_first_read` / `only_pushes_ecx` (register-passed arguments), `arg_reads` and
`cdecl_arity` (stack argument counts), `call_sites` (every `call ADDR` in a catalogued body).

Import it as `sys.path.insert(0, <repo>/tools/lib)` then `import exe`.
"""
import csv
import os
import re

import capstone
import pefile
from capstone import x86_const as X

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
MAIN = ROOT.split('/.claude/worktrees/')[0]


def exe_path():
    if os.environ.get('SA_GTA_EXE'):
        return os.environ['SA_GTA_EXE']
    for r in (ROOT, MAIN):
        p = os.path.join(r, 're', 'gta', '1.0-US', 'gta_sa.exe')
        if os.path.exists(p):
            return p
    return os.path.expanduser('~/sa-engine/re/gta/1.0-US/gta_sa.exe')


EXE = exe_path()
pe = pefile.PE(EXE, fast_load=True)
IMG = pe.get_memory_mapped_image()
BASE = pe.OPTIONAL_HEADER.ImageBase
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
md.detail = True

HOODLUM = 0x1556000
M32 = 0xFFFFFFFF
GAME_TEXT = (0x401000, 0x857000)


def rd(a, n):
    return IMG[a - BASE:a - BASE + n]


def u32_at(a):
    return int.from_bytes(rd(a, 4), 'little')


def insn_at(a):
    return next(md.disasm(rd(a, 16), a), None)


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


# ---------------------------------------------------------------- catalogue

def catalog_path(name):
    return os.path.join(ROOT, 'catalog', name)


def read_csv(name):
    return list(csv.DictReader(open(catalog_path(name), encoding='utf-8')))


GR = {int(r['address'], 16): f"{r['owner']}::{r['name']}" for r in read_csv('functions.csv')}
PROCS = {int(r['address'], 16): r for r in read_csv('procedures.csv')}
RANGES = {}
for _r in read_csv('extents.csv'):
    _rs = []
    for part in _r['ranges'].split(';'):
        if part:
            s, e = part.split('-')
            _rs.append((int(s, 16), int(e, 16)))
    RANGES[int(_r['address'], 16)] = _rs


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


def read_entries():
    """INF-20 readers: entries whose first bytes other code reads (sa-hook rejects hooks on them)."""
    out = {}
    for l in open(catalog_path('code-data-reads.csv')).read().splitlines()[1:]:
        m = re.match(r'(0x[0-9a-f]+),', l)
        cells = re.findall(r'(?:^|,)("(?:[^"]*)"|[^,]*)', l)
        try:
            target, size = int(cells[6], 16), int(cells[7])
        except (ValueError, IndexError):
            continue
        out.setdefault(target, []).append((int(m.group(1), 16), size))
    return out


def body_ranges(a):
    return RANGES.get(a) or []


# ---------------------------------------------------------------- registers

REG32 = {X.X86_REG_EAX: 'eax', X.X86_REG_ECX: 'ecx', X.X86_REG_EDX: 'edx', X.X86_REG_EBX: 'ebx',
         X.X86_REG_ESP: 'esp', X.X86_REG_EBP: 'ebp', X.X86_REG_ESI: 'esi', X.X86_REG_EDI: 'edi'}
REG8 = {X.X86_REG_AL: 'eax', X.X86_REG_CL: 'ecx', X.X86_REG_DL: 'edx', X.X86_REG_BL: 'ebx'}
REG8H = {X.X86_REG_AH: 'eax', X.X86_REG_CH: 'ecx', X.X86_REG_DH: 'edx', X.X86_REG_BH: 'ebx'}
REG16 = {X.X86_REG_AX: 'eax', X.X86_REG_CX: 'ecx', X.X86_REG_DX: 'edx', X.X86_REG_BX: 'ebx',
         X.X86_REG_SI: 'esi', X.X86_REG_DI: 'edi', X.X86_REG_BP: 'ebp'}
PARENT = {}
for _m in (REG32, REG8, REG16, REG8H):
    PARENT.update(_m)


def is_self_xor(i):
    return i.mnemonic == 'xor' and len(i.operands) == 2 and i.operands[0].type == X.X86_OP_REG and \
        i.operands[0].reg == i.operands[1].reg


# ---------------------------------------------------------------- callee facts

# CRT functions outside the catalogue extents (their `ret` read off the bytes by hand): `rand` and `free`
# are cdecl.
CRT_POPS = {0x821B1E: 0, 0x8214BD: 0}
RAND = 0x821B1E
FTOL = 0x821B40
_pops = {}


def pops(a, depth=0, crt=True):
    """Stack bytes the function at `a` removes on return (from its own `ret N`), or None. `crt=False`
    leaves out CRT_POPS (patterns.py's historical reading)."""
    if crt and a in CRT_POPS:
        return CRT_POPS[a]
    key = (a, crt)
    if key in _pops:
        return _pops[key]
    res = None
    rs = body_ranges(a)
    if not rs:
        i = insn_at(a)
        if i and i.mnemonic == 'jmp' and i.op_str.startswith('0x') and depth < 4:
            res = pops(int(i.op_str, 16), depth + 1, crt)
        _pops[key] = res
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
                v = pops(t, depth + 1, crt)
                if v is not None:
                    vals.add(v)
    if len(vals) == 1:
        res = vals.pop()
    _pops[key] = res
    return res


_reads = {}


def entry_reads(a, skip_mov_self=True):
    """Registers the callee's first block reads before writing (following a .HOODLUM stub).
    `skip_mov_self`: `mov edi, edi` (the 2-byte alignment no-op) reads nothing that matters."""
    key = (a, skip_mov_self)
    if key in _reads:
        return _reads[key]
    p = a
    i = insn_at(p)
    if i and i.mnemonic == 'jmp' and i.op_str.startswith('0x'):
        p = int(i.op_str, 16)
    written, read = set(), set()
    n = 0
    for i in md.disasm(rd(p, 200), p):
        n += 1
        if is_self_xor(i):
            written.add(PARENT.get(i.operands[0].reg))
            continue
        if skip_mov_self and i.mnemonic == 'mov' and len(i.operands) == 2 and i.operands[0].type == X.X86_OP_REG and \
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
    _reads[key] = read
    return read


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
            if is_self_xor(i):
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


_arity = None
ZERO_ARITY = set()


def cdecl_arity(a):
    """The argument count of a caller-cleaned callee from its call sites in every catalogued body: the
    `add esp, N` right after `call a`. Known only when every immediate cleanup has the same N and they outnumber
    the sites without one (a callee taking nothing whose sites sometimes carry a merged cleanup of an earlier
    call shows the reverse). PORT-AI-8."""
    global _arity
    if _arity is None:
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
        _arity = {}
        for t, ks in seen.items():
            imm = [k for k in ks if k is not None]
            if not imm or len(set(imm)) != 1 or imm[0] % 4:
                continue
            n = imm[0] // 4
            # a majority of immediate cleanups, or the callee's own reads reaching exactly its last argument
            if (len(imm) >= 2 and len(imm) > len(ks) - len(imm)) or arg_reads(t) == n:
                _arity[t] = n
        for t, ks in seen.items():
            if len(ks) >= 2 and all(k is None for k in ks) and t in RANGES and pops(t) == 0 and \
                    arg_reads(t) == 0 and not entry_reads(t) & {'eax', 'edx', 'ebx', 'esi', 'edi', 'ebp'}:
                ZERO_ARITY.add(t)
    return _arity.get(a, 0)


_sites = None


def call_sites(a):
    """Every `call a` instruction in a catalogued body: (site address, the instructions after it up to the
    end of its range, at most 24)."""
    global _sites
    if _sites is None:
        _sites = {}
        for rs in RANGES.values():
            for s_, e_ in rs:
                for (addr, size, mn, op) in md.disasm_lite(rd(s_, e_ - s_), s_):
                    if mn == 'call' and op.startswith('0x'):
                        _sites.setdefault(int(op, 16), []).append((addr + size, e_))
    out = []
    for nxt, e_ in sorted(set(_sites.get(a, []))):
        out.append((nxt, linear(nxt, min(e_, nxt + 120))[:24]))
    return out
