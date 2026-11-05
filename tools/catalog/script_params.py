#!/usr/bin/env python3
"""CAT-10 part 2 / FMT-3: per-opcode handler and parameter reads, from gta_sa.exe 1.0 US.

The exe has no command table with parameter counts: every handler reads its own parameters.
CRunningScript::ProcessOneCommand (0x469EB0) dispatches `opcode & 0x7FFF` through the table at
0x8A6168 to ProcessCommands<N>To<N+99>; each of those switches on the opcode (jump table, some
with a byte index table).

For every opcode this tool symbolically executes the range function from its entry with the
opcode as the (constant) stack argument and the script as `this`: registers, stack slots and
flags are tracked as constants where possible, so the switch and later opcode tests (shared
handlers such as the LOCATE/IN_AREA helpers, which take the opcode as an argument) resolve to
the opcode's own path. Every path to a `ret` records its parameter reads:

  C<n>  CollectParameters(n)                    0x464080  n numeric parameters
  S<n>  StoreParameters(n)                      0x464370  n variable parameters (written)
  V     GetPointerToScriptVariable              0x464790  1 variable parameter
  T     ReadTextLabelFromScript                 0x463D50  1 string parameter
  G     GetIndexOfGlobalVariable                0x464700  1 global variable parameter
  *     ReadParametersForNewlyStartedScript     0x464500  numeric parameters up to a 0 type byte
  P     CollectNextParameterWithoutIncreasingPC 0x464250  peek (consumes nothing)
  A     ReadArrayInformation                    0x463CF0  (direct use: flagged)

Helper functions called with `this` in ecx are executed too (with the caller's stack
arguments). Loops are unrolled while their condition is decidable; an undecidable back edge
ends the path and is flagged.

Output: catalog/script-params.csv and the decoder's table crates/sa-formats/src/scm/opcodes.rs.
Usage: script_params.py <gta_sa.exe> <catalog-dir> <opcodes.rs>
"""
import csv
import sys as _sys
_sys.setrecursionlimit(100000)
import struct
import sys
from collections import Counter
from pathlib import Path

import capstone
from capstone import x86
import pefile

READERS = {0x464080: 'C', 0x464370: 'S', 0x464790: 'V', 0x463D50: 'T', 0x464700: 'G',
           0x464500: '*', 0x464250: 'P', 0x463CF0: 'A'}
# Stack bytes each reader pops (`ret N`).
READER_POP = {0x464080: 4, 0x464370: 4, 0x464790: 4, 0x463D50: 8, 0x464700: 0,
              0x464500: 4, 0x464250: 0, 0x463CF0: 12}
DISPATCH = 0x8A6168
# Handlers whose count the executor cannot follow, read by hand from the disassembly.
OVERRIDES = {
    # 0x47176C: SWITCH_START does CollectParameters(4) (0x471774) and then CollectParameters(bl)
    # with bl = 0x0E (0x4717B2); SWITCH_CONTINUED takes bl = 0x12 (0x4717BC).
    0x0871: (('C4', 'C14'), 'mov bl,0Eh at 0x4717B2'),
    0x0872: (('C18',), 'mov bl,12h at 0x4717BC'),
    # 0x47CE00: DECLARE_MISSION_FLAG steps m_IP over the type byte without reading it, then
    # reads a u16 (a 3-byte inline operand, whatever the type byte says).
    0x0180: (('R3',), 'm_IP += 1; u16 at 0x47CE00'),
    # 0x48FAEC: SAVE_STRING_TO_DEBUG_FILE copies 0x80 raw bytes from m_IP (no type byte).
    0x05B6: (('R128',), 'copies 0x80 bytes from m_IP at 0x48FAEC'),
}
THIS = 'THIS'
MAX_STEPS = 300000
MAX_PATHS = 64
MAX_CALL_DEPTH = 6

R32 = {}
for name, parent in [('eax', 'eax'), ('ax', 'eax'), ('al', 'eax'), ('ah', 'eax'),
                     ('ebx', 'ebx'), ('bx', 'ebx'), ('bl', 'ebx'), ('bh', 'ebx'),
                     ('ecx', 'ecx'), ('cx', 'ecx'), ('cl', 'ecx'), ('ch', 'ecx'),
                     ('edx', 'edx'), ('dx', 'edx'), ('dl', 'edx'), ('dh', 'edx'),
                     ('esi', 'esi'), ('si', 'esi'), ('edi', 'edi'), ('di', 'edi'),
                     ('ebp', 'ebp'), ('bp', 'ebp'), ('esp', 'esp')]:
    R32[name] = parent


class Exe:
    def __init__(self, path):
        pe = pefile.PE(path, fast_load=True)
        self.img = pe.get_memory_mapped_image()
        self.base = pe.OPTIONAL_HEADER.ImageBase
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.md.detail = True
        self.cache = {}
        self.retpop = {}

    def u32(self, va):
        return struct.unpack_from('<I', self.img, va - self.base)[0]

    def u8(self, va):
        return self.img[va - self.base]

    def insn(self, va):
        if va not in self.cache:
            off = va - self.base
            if off < 0 or off >= len(self.img) - 16:
                self.cache[va] = None
            else:
                self.cache[va] = next(self.md.disasm(self.img[off:off + 16], va), None)
        return self.cache[va]

    def callee_pop(self, va):
        """Bytes popped by the first `ret` reachable by a linear walk (stdcall/thiscall)."""
        if va in self.retpop:
            return self.retpop[va]
        seen = set()
        work = [va]
        pop = None
        while work and pop is None and len(seen) < 3000:
            a = work.pop()
            while a not in seen:
                seen.add(a)
                i = self.insn(a)
                if i is None:
                    break
                if i.mnemonic == 'ret':
                    pop = i.operands[0].imm if i.operands else 0
                    break
                if i.mnemonic == 'jmp':
                    if i.operands[0].type == x86.X86_OP_IMM:
                        a = i.operands[0].imm
                        continue
                    break
                if i.mnemonic.startswith('j') and i.operands[0].type == x86.X86_OP_IMM:
                    work.append(i.operands[0].imm)
                a = i.address + i.size
        self.retpop[va] = pop
        return pop


def mask(v, bits):
    return v & ((1 << bits) - 1)


class State:
    __slots__ = ('regs', 'stack', 'esp', 'flags')

    def __init__(self):
        self.regs = {}
        self.stack = {}
        self.esp = 0
        self.flags = None  # (kind, a, b) of the last cmp/test/sub/and, values may be None

    def copy(self):
        s = State()
        s.regs = dict(self.regs)
        s.stack = dict(self.stack)
        s.esp = self.esp
        s.flags = self.flags
        return s

    def key(self):
        return (frozenset(self.regs.items()), frozenset(self.stack.items()), self.esp, self.flags)


class Exec:
    def __init__(self, exe):
        self.exe = exe
        self.may_memo = {}
        self.memo = {}

    # -- operand helpers -------------------------------------------------------------------
    def reg_name(self, i, r):
        return i.reg_name(r)

    def get_reg(self, st, i, r):
        name = self.reg_name(i, r)
        parent = R32.get(name)
        if parent is None:
            return None
        if parent == 'esp':
            return ('ESP', st.esp)
        v = st.regs.get(parent)
        if v is None:
            return None
        if name == parent:
            return v if not (isinstance(v, tuple) and v[0] in ('LO16', 'LO8')) else None
        if isinstance(v, tuple) and v[0] == 'LO16':
            if name in ('ax', 'bx', 'cx', 'dx', 'si', 'di', 'bp'):
                return v[1]
            return mask(v[1], 8) if name.endswith('l') else (mask(v[1] >> 8, 8) if name.endswith('h') else None)
        if isinstance(v, tuple) and v[0] == 'LO8':
            return v[1] if name.endswith('l') else None
        if not isinstance(v, int):
            return None
        if name in ('ax', 'bx', 'cx', 'dx', 'si', 'di', 'bp'):
            return mask(v, 16)
        if name.endswith('l'):
            return mask(v, 8)
        if name.endswith('h'):
            return mask(v >> 8, 8)
        return None

    def set_reg(self, st, i, r, v):
        name = self.reg_name(i, r)
        parent = R32.get(name)
        if parent is None:
            return
        if parent == 'esp':
            if isinstance(v, tuple) and v[0] == 'ESP':
                st.esp = v[1]
            return
        if name == parent:
            if v is None:
                st.regs.pop(parent, None)
            else:
                st.regs[parent] = v
            return
        old = st.regs.get(parent)
        if not isinstance(v, int):
            st.regs.pop(parent, None)
            return
        if not isinstance(old, int):
            if name in ('ax', 'bx', 'cx', 'dx', 'si', 'di', 'bp'):
                st.regs[parent] = ('LO16', mask(v, 16))
            elif name.endswith('l'):
                lo = old[1] if isinstance(old, tuple) and old[0] == 'LO16' else 0
                st.regs[parent] = ('LO16', (lo & 0xFF00) | mask(v, 8)) if isinstance(old, tuple) and old[0] == 'LO16' else ('LO8', mask(v, 8))
            else:
                st.regs.pop(parent, None)
            return
        if name in ('ax', 'bx', 'cx', 'dx', 'si', 'di', 'bp'):
            st.regs[parent] = (old & ~0xFFFF) | mask(v, 16)
        elif name.endswith('l'):
            st.regs[parent] = (old & ~0xFF) | mask(v, 8)
        else:
            st.regs[parent] = (old & ~0xFF00) | (mask(v, 8) << 8)

    def mem_addr(self, st, i, m):
        """('S', offset) for an esp/stack-relative address, an int for a constant, else None."""
        base = self.get_reg(st, i, m.base) if m.base else 0
        idx = self.get_reg(st, i, m.index) if m.index else 0
        if base is None or idx is None:
            return None
        if isinstance(idx, tuple) or not isinstance(idx, int):
            return None
        off = m.disp + idx * m.scale
        if isinstance(base, tuple) and base[0] == 'ESP':
            return ('S', base[1] + off)
        if isinstance(base, int):
            return (base + off) & 0xFFFFFFFF
        return None

    def read_op(self, st, i, op, size):
        if op.type == x86.X86_OP_IMM:
            return mask(op.imm, size * 8)
        if op.type == x86.X86_OP_REG:
            return self.get_reg(st, i, op.reg)
        if op.type == x86.X86_OP_MEM:
            a = self.mem_addr(st, i, op.mem)
            if isinstance(a, tuple):
                v = st.stack.get(a[1])
                if size == 4 or v is None:
                    return v if size == 4 else (mask(v, size * 8) if isinstance(v, int) else None)
                return mask(v, size * 8) if isinstance(v, int) else None
            if isinstance(a, int) and size == 1 and 0x401000 <= a < 0x857000:
                # Byte index tables inside .text (switch helpers).
                return self.exe.u8(a)
            if isinstance(a, int) and size == 4 and 0x401000 <= a < 0x857000:
                return self.exe.u32(a)
        return None

    def write_op(self, st, i, op, v):
        if op.type == x86.X86_OP_REG:
            self.set_reg(st, i, op.reg, v)
        elif op.type == x86.X86_OP_MEM:
            a = self.mem_addr(st, i, op.mem)
            if isinstance(a, tuple):
                if op.size == 4 and v is not None:
                    st.stack[a[1]] = v
                else:
                    st.stack.pop(a[1], None)

    # -- execution ---------------------------------------------------------------------------
    def run(self, va, st, depth, budget):
        """Returns (set of read sequences, flags). Each path ends at `ret`. A path that loops
        back to an (address, state) it has seen ends there and is flagged."""
        flags = set()
        results = set()
        work = [(va, st, (), set())]
        while work:
            va, st, seq, visits = work.pop()
            while True:
                budget[0] -= 1
                if budget[0] <= 0:
                    # Keep the reads of the unfinished path: the caller marks the result
                    # 'partial' and the decoder only uses it as a candidate checked by the data.
                    flags.add('budget')
                    return results | {seq}, flags
                i = self.exe.insn(va)
                if i is not None and i.mnemonic.startswith('j'):
                    # Every loop passes a jump: stop at a repeated (address, state) there.
                    vk = (va, st.key())
                    if vk in visits:
                        flags.add('loop-undecided')
                        break
                    visits.add(vk)
                if i is None:
                    flags.add(f'undecodable@{va:#x}')
                    break
                nxt = i.address + i.size
                mn = i.mnemonic
                ops = i.operands
                if mn == 'ret':
                    results.add(seq)
                    break
                if mn == 'call':
                    tgt = ops[0].imm if ops[0].type == x86.X86_OP_IMM else None
                    sub, pop = self.call(st, i, tgt, depth, budget, flags)
                    seq_list = [seq + t for t in sub] if sub else [seq]
                    st.esp += pop
                    for r in ('eax', 'ecx', 'edx'):
                        if tgt in READERS and r != 'eax':
                            continue
                        st.regs.pop(r, None)
                    if len(seq_list) > 1:
                        for s2 in seq_list[1:]:
                            if len(work) < MAX_PATHS:
                                work.append((nxt, st.copy(), s2, set(visits)))
                            else:
                                flags.add('path-limit')
                    seq = seq_list[0]
                    va = nxt
                    continue
                if mn == 'jmp':
                    if ops[0].type == x86.X86_OP_IMM:
                        va = ops[0].imm
                        continue
                    t = self.read_op(st, i, ops[0], 4)
                    if isinstance(t, int):
                        va = t
                        continue
                    flags.add(f'indirect-jump@{i.address:#x}')
                    break
                if mn.startswith('j'):
                    taken = self.cond(st, mn)
                    tgt = ops[0].imm
                    if taken is None:
                        if len(work) < MAX_PATHS:
                            work.append((tgt, st.copy(), seq, set(visits)))
                        else:
                            flags.add('path-limit')
                        va = nxt
                    else:
                        va = tgt if taken else nxt
                    continue
                self.step(st, i, flags)
                va = nxt
        return results, flags

    def may_read(self, va, seen=None):
        """Whether a function can reach a parameter reader through direct calls (static walk
        of its jumps). Functions that cannot are not executed."""
        if va in self.may_memo:
            return self.may_memo[va]
        if seen is None:
            seen = set()
        if va in seen:
            return False
        seen.add(va)
        self.may_memo[va] = False  # recursion guard
        found = False
        visited = set()
        work = [va]
        while work and not found and len(visited) < 20000:
            a = work.pop()
            while a not in visited:
                visited.add(a)
                i = self.exe.insn(a)
                if i is None or i.mnemonic == 'ret':
                    break
                if i.mnemonic == 'call' and i.operands[0].type == x86.X86_OP_IMM:
                    t = i.operands[0].imm
                    if t in READERS or self.may_read(t, seen):
                        found = True
                        break
                if i.mnemonic.startswith('j'):
                    if i.operands[0].type == x86.X86_OP_IMM:
                        if i.mnemonic == 'jmp':
                            a = i.operands[0].imm
                            continue
                        work.append(i.operands[0].imm)
                    else:
                        # Jump tables: be conservative.
                        found = found or i.mnemonic == 'jmp' and False
                        if i.mnemonic == 'jmp':
                            break
                a = i.address + i.size
        self.may_memo[va] = found
        return found

    def call(self, st, i, tgt, depth, budget, flags):
        """Read sequences contributed by a call and the bytes the callee pops."""
        if tgt in READERS:
            kind = READERS[tgt]
            if kind in 'CS':
                n = st.stack.get(st.esp)
                if isinstance(n, tuple) and n[0] == 'LO16':
                    n = n[1]
                if isinstance(n, int):
                    n = mask(n, 16)
                    tok = f'{kind}{n}'
                else:
                    tok = f'{kind}?'
                    flags.add('count-unknown')
            else:
                tok = kind
            return [(tok,)], READER_POP[tgt]
        pop = self.exe.callee_pop(tgt) if tgt is not None else None
        if pop is None:
            pop = 0
        if (tgt is not None and st.regs.get('ecx') == THIS and depth < MAX_CALL_DEPTH
                and self.may_read(tgt)):
            cst = State()
            cst.regs['ecx'] = THIS
            # Callee sees the return address at [0] and the caller's pushed args from [4].
            for k in range(8):
                v = st.stack.get(st.esp + 4 * k)
                if v is not None:
                    cst.stack[4 + 4 * k] = v
            res, fl = self.run(tgt, cst, depth + 1, budget)
            flags |= fl
            return sorted(res) if res else [()], pop
        return [()], pop

    def cond(self, st, mn):
        f = st.flags
        if f is None:
            return None
        kind, a, b = f
        if not isinstance(a, int) or not isinstance(b, int):
            return None
        sa = a - (1 << 32) if a & 0x80000000 else a
        sb = b - (1 << 32) if b & 0x80000000 else b
        if kind == 'cmp':
            r = {'je': a == b, 'jne': a != b, 'jb': a < b, 'jae': a >= b, 'ja': a > b, 'jbe': a <= b,
                 'jl': sa < sb, 'jge': sa >= sb, 'jg': sa > sb, 'jle': sa <= sb}
            return r.get(mn)
        if kind == 'test':
            v = a & b
            sv = v - (1 << 32) if v & 0x80000000 else v
            r = {'je': v == 0, 'jne': v != 0, 'js': sv < 0, 'jns': sv >= 0, 'jle': sv <= 0, 'jg': sv > 0}
            return r.get(mn)
        return None

    def step(self, st, i, flags):
        mn = i.mnemonic
        ops = i.operands
        keep = object()
        st_flags = keep
        # A handler that moves the script's instruction pointer itself (CRunningScript::m_IP
        # at +0x14) reads inline data the parameter readers do not see.
        if ops and ops[0].type == x86.X86_OP_MEM and ops[0].mem.disp == 0x14 and ops[0].mem.base \
                and self.get_reg(st, i, ops[0].mem.base) == THIS and mn in ('mov', 'add', 'sub', 'inc'):
            flags.add(f'writes-ip@{i.address:#x}')
        if mn == 'push':
            v = self.read_op(st, i, ops[0], 4)
            if ops[0].type == x86.X86_OP_IMM:
                v = mask(ops[0].imm, 32)
            st.esp -= 4
            if v is None:
                st.stack.pop(st.esp, None)
            else:
                st.stack[st.esp] = v
        elif mn == 'pop':
            v = st.stack.get(st.esp)
            st.esp += 4
            self.write_op(st, i, ops[0], v)
        elif mn in ('pushfd', 'pushal'):
            st.esp -= 4 if mn == 'pushfd' else 32
        elif mn in ('popfd', 'popal'):
            st.esp += 4 if mn == 'popfd' else 32
        elif mn == 'mov':
            v = self.read_op(st, i, ops[1], ops[0].size)
            self.write_op(st, i, ops[0], v)
        elif mn in ('movzx', 'movsx'):
            v = self.read_op(st, i, ops[1], ops[1].size)
            if isinstance(v, int) and mn == 'movsx' and v & (1 << (ops[1].size * 8 - 1)):
                v = mask(v - (1 << (ops[1].size * 8)), ops[0].size * 8)
            self.write_op(st, i, ops[0], v)
        elif mn == 'lea':
            a = self.mem_addr(st, i, ops[1].mem)
            if isinstance(a, tuple):
                self.write_op(st, i, ops[0], ('ESP', a[1]) if R32.get(i.reg_name(ops[0].reg)) == 'esp' else None)
                if R32.get(i.reg_name(ops[0].reg)) == 'esp':
                    st.esp = a[1]
            else:
                self.write_op(st, i, ops[0], a)
        elif mn in ('add', 'sub', 'and', 'or', 'xor', 'shl', 'shr', 'sar', 'imul') and len(ops) == 2:
            dst = ops[0]
            if dst.type == x86.X86_OP_REG and R32.get(i.reg_name(dst.reg)) == 'esp' and ops[1].type == x86.X86_OP_IMM:
                st.esp += ops[1].imm if mn == 'add' else -ops[1].imm if mn == 'sub' else 0
                return
            a = self.read_op(st, i, dst, dst.size)
            b = self.read_op(st, i, ops[1], dst.size)
            bits = dst.size * 8
            if mn == 'xor' and ops[1].type == x86.X86_OP_REG and dst.type == x86.X86_OP_REG and ops[1].reg == dst.reg:
                v = 0
            elif isinstance(a, int) and isinstance(b, int):
                v = {'add': a + b, 'sub': a - b, 'and': a & b, 'or': a | b, 'xor': a ^ b,
                     'shl': a << (b & 31), 'shr': a >> (b & 31),
                     'sar': (a - (1 << bits) if a >> (bits - 1) else a) >> (b & 31),
                     'imul': a * b}[mn]
                v = mask(v, bits)
            else:
                v = None
            self.write_op(st, i, dst, v)
            if mn in ('sub', 'and'):
                st_flags = ('cmp', a, b) if mn == 'sub' else ('test', a, b)
            elif v is not None:
                st_flags = ('test', v, mask(-1, 32))
            else:
                st_flags = None
        elif mn in ('inc', 'dec', 'neg', 'not'):
            a = self.read_op(st, i, ops[0], ops[0].size)
            bits = ops[0].size * 8
            v = None
            if isinstance(a, int):
                v = mask({'inc': a + 1, 'dec': a - 1, 'neg': -a, 'not': ~a}[mn], bits)
            self.write_op(st, i, ops[0], v)
            st_flags = ('test', v, mask(-1, 32)) if v is not None else None
        elif mn in ('cmp', 'test'):
            size = ops[0].size
            a = self.read_op(st, i, ops[0], size)
            b = self.read_op(st, i, ops[1], size)
            st.flags = (mn, a, b)
            return
        else:
            # Unknown effect: forget every register the instruction writes.
            try:
                _, written = i.regs_access()
            except capstone.CsError:
                written = []
            for r in written:
                name = i.reg_name(r)
                parent = R32.get(name)
                if parent and parent != 'esp':
                    st.regs.pop(parent, None)
            if not (mn.startswith(('f', 'set', 'cmov', 'nop', 'xchg', 'cdq', 'cwd')) or mn in ('wait',)):
                st_flags = None
        if st_flags is not keep:
            st.flags = st_flags


def main():
    exe = Exe(sys.argv[1])
    catalog = Path(sys.argv[2])
    rs_out = Path(sys.argv[3])
    names = {}
    with open(catalog / 'script-commands.csv') as f:
        for row in csv.DictReader(f):
            names[int(row['opcode'], 16)] = row['name']
    ex = Exec(exe)
    rows = []
    for r in range(27):
        fva = exe.u32(DISPATCH + 4 * r)
        for op in range(r * 100, r * 100 + 100):
            name = names.get(op, '')
            st = State()
            st.regs['ecx'] = THIS
            st.stack[4] = op
            budget = [MAX_STEPS]
            res, fl = ex.run(fva, st, 0, budget)
            handler = case_address(ex, exe, fva, op)
            if op in OVERRIDES:
                res = {OVERRIDES[op][0]}
                fl = fl | {'override: ' + OVERRIDES[op][1]}
            seqs = sorted(res)
            reads = ' | '.join(' '.join(s) if s else '-' for s in seqs)
            summ = summarize(seqs)
            status, params = classify(summ, seqs)
            if 'budget' in fl and status == 'ok' and op not in OVERRIDES:
                status = 'partial'
            if handler is None:
                status = 'no-handler'
            rows.append([f'{op:04X}', name, f'{handler:#x}' if handler else '', reads, params,
                         status, ' '.join(sorted(fl))])
    with open(catalog / 'script-params.csv', 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['opcode', 'name', 'handler', 'reads', 'params', 'status', 'note'])
        w.writerows(rows)
    print(Counter(r[5] for r in rows))
    write_rust(rows, rs_out)


def case_address(ex, exe, fva, op):
    """The first address after the range function's switch dispatch for `op` (None when the
    switch's default is taken)."""
    st = State()
    st.regs['ecx'] = THIS
    st.stack[4] = op
    va = fva
    default = None
    for _ in range(80):
        i = exe.insn(va)
        mn = i.mnemonic
        if mn == 'ja':
            default = i.operands[0].imm
            taken = ex.cond(st, mn)
            if taken:
                return None
            va = i.address + i.size
            continue
        if mn == 'jmp' and i.operands[0].type == x86.X86_OP_MEM:
            t = ex.read_op(st, i, i.operands[0], 4)
            if t == default:
                return None
            return t
        if mn.startswith('j') or mn == 'call' or mn == 'ret':
            return None
        ex.step(st, i, set())
        va = i.address + i.size
    return None


def summarize(seqs):
    out = set()
    for s in seqs:
        n = 0
        var = False
        unknown = False
        for t in s:
            if t[0] in 'CS':
                if t[1:] == '?':
                    unknown = True
                else:
                    n += int(t[1:])
            elif t in ('V', 'T', 'G'):
                n += 1
            elif t == '*':
                var = True
            elif t == 'A':
                unknown = True
        out.add((n, var, unknown))
    return out


def classify(summ, seqs):
    if not seqs:
        return 'no-path', ''
    if len(summ) == 1:
        n, var, unk = next(iter(summ))
        if unk:
            return 'unknown-count', ''
        return 'ok', f'{n}{"+var" if var else ""}'
    return 'path-dependent', '/'.join(f'{n}{"+var" if v else ""}{"?" if u else ""}' for n, v, u in sorted(summ))


def write_rust(rows, path):
    lines = [
        '//! Generated by `tools/catalog/script_params.py` from gta_sa.exe 1.0 US (do not edit).',
        '//!',
        '//! Every opcode 0x0000..=0x0A8B that `CRunningScript::ProcessCommands*` handles:',
        '//! `(opcode, name, handler address, parameter reads, status)`. Reads: `C<n>` collected',
        '//! numbers, `S<n>` stored variables, `V` variable pointer, `T` text label, `G` global',
        '//! variable index, `*` arguments up to a 0 type byte, `P` peek (consumes nothing), `R<n>`',
        '//! n raw bytes the handler takes from `m_IP` itself (no type byte);',
        '//! `|` separates the reads of different paths; `?` marks a count the analysis could',
        '//! not determine. Names come from gta-reversed `eScriptCommands.h`.',
        '',
        '/// `(opcode, name, handler, reads, status)`.',
        'pub(crate) const OPCODES: &[(u16, &str, u32, &str, &str)] = &[',
    ]
    for op, name, handler, reads, params, status, note in rows:
        if status == 'no-handler':
            continue
        lines.append(f'    (0x{op}, "{name}", {handler}, "{reads}", "{status}"),')
    lines.append('];')
    path.write_text('\n'.join(lines) + '\n')


if __name__ == '__main__':
    main()
