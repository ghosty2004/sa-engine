"""What the original says about a function's signature (ADR-003 §3.2: one binding per address, its signature
taken from the original): calling convention, stack arguments, how wide the callee reads each argument, and
how its callers use the result. Built on `exe.py`; used by `tools/gen/bindings.py --facts / --resolve`.

All facts are read off the bytes; each is None when the bytes do not decide it.
"""
import collections

import capstone
from capstone import x86_const as X

import exe

FPU_PUSH = ('fld', 'fild', 'fldz', 'fld1', 'fldpi', 'fldl2e', 'fldln2', 'fldlg2', 'fldl2t')


def target(a):
    """The body's entry (a `.HOODLUM` stub's target)."""
    i = exe.insn_at(a)
    if i and i.mnemonic == 'jmp' and i.op_str.startswith('0x') and not exe.body_ranges(a):
        return int(i.op_str, 16)
    return a


def _site_use(insns):
    """How the instructions after one call use the result: 'st0', 'byte', 'word', 'dword', 'qword'
    (EDX:EAX), or None (not used before being overwritten / the path leaves)."""
    for i in insns:
        mn = i.mnemonic
        if mn.startswith('f') and mn not in ('fnstsw', 'fnstcw', 'fldcw', 'fwait'):
            # the first x87 instruction: a push starts a new value, anything else consumes ST0
            return None if mn in FPU_PUSH else 'st0'
        try:
            rr, ww = i.regs_access()
        except capstone.CsError:
            return None
        if exe.is_self_xor(i) and exe.PARENT.get(i.operands[0].reg) in ('eax', 'edx'):
            return None
        reads = [r for r in rr if exe.PARENT.get(r) in ('eax', 'edx')]
        # `push eax` / `mov [m], eax` etc.: what is read and how wide
        if reads:
            if any(exe.PARENT.get(r) == 'edx' for r in reads) and not any(exe.PARENT.get(r) == 'eax' for r in reads):
                return 'qword'
            r = [r for r in reads if exe.PARENT.get(r) == 'eax'][0]
            if r in exe.REG32:
                return 'dword'
            sign = '-s' if mn == 'movsx' else ('-u' if mn == 'movzx' else '')
            if r in exe.REG16:
                return 'word' + sign
            return 'byte' + sign
        if any(exe.PARENT.get(r) == 'eax' for r in ww):
            return None
        if mn in ('ret', 'call', 'jmp') or mn.startswith('j'):
            # flags / branches: a `call` clobbers EAX; a jump leaves the straight line
            return None
    return None


def ret_use(a):
    """Counter of how the call sites use the result ('byte' / 'word' with '-s' / '-u' when the first use
    sign- / zero-extends it)."""
    c = collections.Counter()
    for _, insns in exe.call_sites(a):
        c[_site_use(insns)] += 1
    return c


def arg_widths(a, stack_args):
    """For each stack argument slot (0-based, after the return address), the first access the body makes:
    (size in bytes, 'fpu' / 'int' / 'lea'), or None. ESP tracked through push / pop / add / sub, EBP frames
    (`push ebp; mov ebp, esp`) followed."""
    body = target(a)
    rs = sorted(exe.body_ranges(body)) or [(body, body + 400)]
    first = {}
    depth = 0
    ebp = None
    for s_, e_ in rs:
        for i in exe.linear(s_, e_):
            mn = i.mnemonic
            for o in i.operands:
                if o.type != X.X86_OP_MEM or o.mem.index:
                    continue
                if o.mem.base == X.X86_REG_ESP:
                    slot = o.mem.disp - depth
                elif o.mem.base == X.X86_REG_EBP and ebp is not None:
                    slot = o.mem.disp - ebp
                else:
                    continue
                if slot >= 4 and slot % 4 == 0 and (slot // 4 - 1) < stack_args:
                    k = slot // 4 - 1
                    kind = 'lea' if mn == 'lea' else ('fpu' if mn.startswith('f') else 'int')
                    first.setdefault(k, (o.size, kind))
            if mn == 'push':
                depth += 4
            elif mn == 'pop':
                depth -= 4
            elif mn in ('sub', 'add') and i.op_str.startswith('esp, ') and i.operands[1].type == X.X86_OP_IMM:
                depth += i.operands[1].imm if mn == 'sub' else -i.operands[1].imm
            elif mn == 'mov' and i.op_str == 'ebp, esp':
                ebp = depth
            elif mn in ('ret', 'jmp'):
                return [first.get(k) for k in range(stack_args)]
            elif 'esp' in i.op_str.split(',')[0] and mn not in ('push', 'pop', 'sub', 'add', 'call') \
                    and not i.op_str.split(',')[0].strip().endswith(']'):
                return [first.get(k) for k in range(stack_args)]
    return [first.get(k) for k in range(stack_args)]


def facts(a):
    """conv ('thiscall' / 'stdcall' / 'cdecl' / None), stack argument count, register reads, result use."""
    body = target(a)
    p = exe.pops(a)
    ecx = exe.ecx_first_read(a) and not exe.only_pushes_ecx(body)
    regs = exe.entry_reads(a) - {'ecx'}
    if p is None:
        conv, n = None, None
    elif p > 0:
        conv, n = ('thiscall' if ecx else 'stdcall'), p // 4
    else:
        conv = 'thiscall' if ecx else 'cdecl'
        n = exe.cdecl_arity(a) or (exe.arg_reads(body) if body in exe.RANGES else None)
        if conv == 'thiscall':
            n = exe.arg_reads(body) if body in exe.RANGES else None  # `ret` alone: no stack arguments to pop
    return {'conv': conv, 'stack': n, 'regs': sorted(regs), 'ret': ret_use(a),
            'widths': arg_widths(a, n) if n else []}
