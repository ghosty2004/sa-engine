"""Mechanical translator: straight-line x87 RW camera builders (0x7EE740, 0x7EEDE0) -> Rust.

Usage: x87tr.py START STOP FNNAME > out.rs
Registers holding addresses are tracked at translation time (camera C, LTM M, stack S, data D);
integer data moves become runtime u32 temporaries; the x87 stack becomes calls on `Fpu`.
"""
import re
import subprocess
import sys

EXE = '/home/ghosty2004/sa-engine/re/gta/1.0-US/gta_sa.exe'
start, stop, fname = sys.argv[1], sys.argv[2], sys.argv[3]
out = subprocess.run(['objdump', '-d', '-M', 'intel', '--start-address=' + start, '--stop-address=' + stop, EXE],
                     capture_output=True, text=True).stdout
ins = []
for line in out.splitlines():
    m = re.match(r'\s+([0-9a-f]+):\t((?:[0-9a-f]{2} )+)\s*\t(\S+)\s*(.*)$', line)
    if not m:
        continue
    a = int(m.group(1), 16)
    b = [int(x, 16) for x in m.group(2).split()]
    ins.append((a, b, m.group(3), m.group(4).strip()))
idx = {a: i for i, (a, _, _, _) in enumerate(ins)}

R32 = ['eax', 'ecx', 'edx', 'ebx', 'esp', 'ebp', 'esi', 'edi']
LOW = {'al': 'eax', 'cl': 'ecx', 'dl': 'edx', 'bl': 'ebx'}
reg = {'esp': ('S', 0)}
stack = {}  # entry-relative stack address -> value (for call args)
code = []
tmp = [0]
written_locals = set()


def T():
    tmp[0] += 1
    return 't%d' % tmp[0]


def emit(s):
    code.append('    ' + s)


def addr_of(op):
    op = op.replace('DWORD PTR ', '').replace('BYTE PTR ', '').replace('WORD PTR ', '')
    m = re.match(r'ds:(0x[0-9a-f]+)$', op)
    if m:
        return ('D', int(m.group(1), 16))
    m = re.match(r'\[(\w+)(?:([+-])(0x[0-9a-f]+|\d+))?\]$', op)
    assert m, op
    r = m.group(1)
    off = int(m.group(3), 0) if m.group(3) else 0
    if m.group(2) == '-':
        off = -off
    v = reg[r]
    assert v[0] in ('C', 'M', 'S'), (op, v)
    return (v[0], v[1] + off)


def loc_index(o):
    return (o + 0x80) // 4


def rd_f32(a):
    k, o = a
    if k == 'C':
        return 'rd::<f32>(c, %#x)' % o
    if k == 'M':
        return 'rd::<f32>(m, %#x)' % o
    if k == 'S':
        if o == 4 and o not in written_locals:
            raise Exception('float read of arg')
        assert o in written_locals, ('uninit local read', hex(o))
        return 'f32::from_bits(loc[%d])' % loc_index(o)
    if k == 'D':
        return 'g::<f32>(%#x).read()' % o


def rd_u32(a):
    k, o = a
    if k == 'C':
        return 'rd::<u32>(c, %#x)' % o
    if k == 'M':
        return 'rd::<u32>(m, %#x)' % o
    if k == 'S':
        assert o in written_locals, ('uninit local read', hex(o))
        return 'loc[%d]' % loc_index(o)


def wr_u32(a, v):
    k, o = a
    if k == 'C':
        emit('wr::<u32>(c, %#x, %s);' % (o, v))
    elif k == 'M':
        emit('wr::<u32>(m, %#x, %s);' % (o, v))
    elif k == 'S':
        written_locals.add(o)
        emit('loc[%d] = %s;' % (loc_index(o), v))
    else:
        raise Exception(a)


def wr_f32(a, v):
    k, o = a
    if k == 'S':
        written_locals.add(o)
        emit('loc[%d] = %s.to_bits();' % (loc_index(o), v))
    else:
        emit('wr::<f32>(%s, %#x, %s);' % ({'C': 'c', 'M': 'm'}[k], o, v))


def sti(s):
    if s in ('st', 'st(0)'):
        return 0
    m = re.match(r'st\((\d)\)$', s)
    return int(m.group(1))


pc = 0
esp = -0  # entry-relative esp (entry esp = 0, return address at 0, arg at +4)
reg['esp'] = ('S', 0)
done = False
steps = 0
while not done:
    steps += 1
    assert steps < 5000
    a, b, mn, ops = ins[pc]
    pc += 1
    o = [x.strip() for x in ops.split(',')] if ops else []
    emit('// %06x %s %s' % (a, mn, ops))
    if mn == 'sub' and o[0] == 'esp':
        esp -= int(o[1], 0)
        reg['esp'] = ('S', esp)
    elif mn == 'add' and o[0] == 'esp':
        esp += int(o[1], 0)
        reg['esp'] = ('S', esp)
    elif mn == 'push':
        esp -= 4
        reg['esp'] = ('S', esp)
        if o[0] in R32:
            stack[esp] = reg.get(o[0])
        else:
            stack[esp] = ('imm', int(o[0], 0))
    elif mn == 'pop':
        esp += 4
        reg['esp'] = ('S', esp)
    elif mn == 'ret':
        done = True
    elif mn == 'mov' and o[0] in R32 and o[1] in R32:
        reg[o[0]] = reg[o[1]]
    elif mn == 'mov' and o[0] in R32 and re.match(r'0x[0-9a-f]+$|\d+$', o[1]):
        reg[o[0]] = ('imm', int(o[1], 0))
    elif mn == 'mov' and o[0] in R32:
        ad = addr_of(o[1])
        if ad == ('S', 4) and 4 not in written_locals:
            reg[o[0]] = ('C', 0)
        elif ad == ('C', 4):
            t = T()
            emit('let %s = rd::<u32>(c, 4);' % t)
            reg[o[0]] = ('frame', t)
        else:
            t = T()
            emit('let %s: u32 = %s;' % (t, rd_u32(ad)))
            reg[o[0]] = ('rt', t)
    elif mn == 'mov' and o[1] in R32:
        ad = addr_of(o[0])
        v = reg[o[1]]
        assert v[0] == 'rt', (a, v)
        wr_u32(ad, v[1])
    elif mn == 'mov' and o[1] in LOW:
        ad = addr_of(o[0])
        v = reg[LOW[o[1]]]
        assert v[0] == 'rt'
        k, off = ad
        emit('wr::<u8>(%s, %#x, %s as u8);' % ({'C': 'c', 'M': 'm'}[k], off, v[1]))
    elif mn == 'lea':
        ad = addr_of(o[1])
        reg[o[0]] = ad
    elif mn == 'add' and o[0] in R32:
        v = reg[o[0]]
        n = int(o[1], 0)
        if v[0] == 'frame' and n == 0x50:
            emit('let m = p8(%s).byte_add(0x50);' % v[1])
            reg[o[0]] = ('M', 0)
        elif v[0] in ('C', 'M', 'S'):
            reg[o[0]] = (v[0], v[1] + n)
        else:
            raise Exception((hex(a), v))
    elif mn == 'sar':
        v = reg[o[0]]
        assert v[0] == 'rt' and o[1] == '0x1f'
        t = T()
        emit('let %s = ((%s as i32) >> 31) as u32;' % (t, v[1]))
        reg[o[0]] = ('rt', t)
    elif mn == 'inc' and o[0] in LOW:
        v = reg[LOW[o[0]]]
        t = T()
        emit('let %s = (%s & !0xFF) | u32::from((%s as u8).wrapping_add(1));' % (t, v[1], v[1]))
        reg[LOW[o[0]]] = ('rt', t)
    elif mn == 'dec' and o[0] == 'ebp':
        v = reg['ebp']
        reg['ebp'] = ('imm', v[1] - 1)
    elif mn == 'jne':
        tgt = int(o[0], 0)
        if reg['ebp'][1] != 0:
            pc = idx[tgt]
    elif mn == 'xor' and o[0] == o[1]:
        reg[o[0]] = ('rt', '0u32')
    elif mn == 'call':
        tgt = int(o[0], 0)
        if tgt == 0x7EDB90:
            v = stack[esp]
            assert v[0] == 'rt'
            emit('f.push(X87::from_f64_exact(inv_sqrt(f32::from_bits(%s))));' % v[1])
        elif tgt == 0x7EDD90:
            args = [stack[esp + 4 * k] for k in range(4)]

            def pv(v):
                if v[0] in ('C', 'M'):
                    return '%s.byte_add(%#x).cast()' % ({'C': 'c', 'M': 'm'}[v[0]], v[1])
                if v[0] == 'imm':
                    return '%d' % v[1]
                raise Exception(v)
            emit('transform_points(%s, %s, %s, %s);' % tuple(pv(x) for x in args))
            for r in ('eax', 'ecx', 'edx'):
                reg[r] = ('dead',)
        else:
            raise Exception(hex(tgt))
    # x87
    elif mn == 'fld':
        if o[0].startswith('st'):
            emit('f.push(f.st(%d));' % sti(o[0]))
        else:
            emit('f.push(x(%s));' % rd_f32(addr_of(o[0])))
    elif mn in ('fst', 'fstp') and not o[0].startswith('st'):
        t = T()
        emit('let %s = f.st(0).to_f32();' % t)
        wr_f32(addr_of(o[0]), t)
        if mn == 'fstp':
            emit('f.pop();')
    elif mn == 'fstp':
        emit('f.stp(%d);' % sti(o[0]))
    elif mn == 'fchs':
        emit('f.set(0, f.st(0).fchs());')
    elif mn == 'fxch':
        emit('f.xch(%d);' % sti(o[0]))
    elif mn in ('fadd', 'fsub', 'fmul', 'faddp', 'fsubp', 'fsubrp', 'fmulp'):
        if len(o) == 1:  # memory operand, st0 dest
            m_ = {'fadd': 'fadd_m', 'fsub': 'fsub_m', 'fmul': 'fmul_m'}[mn]
            emit('f.set(0, f.st(0).%s(%s));' % (m_, rd_f32(addr_of(o[0]))))
        else:
            op0, op1 = b[0], b[1]
            i = op1 & 7
            kind = (op1 >> 3) & 7  # 0 add 1 mul 4 / 5 sub forms
            if op0 == 0xD8:  # st0 = st0 op st(i)
                f_ = {0: 'fadd', 1: 'fmul', 4: 'fsub', 5: 'fsubr', 6: 'fdiv', 7: 'fdivr'}[kind]
                emit('f.set(0, f.st(0).%s(f.st(%d)));' % (f_, i))
            elif op0 in (0xDC, 0xDE):  # st(i) = st(i) op st0 (Intel encoding semantics)
                # DC/DE C0+i add, C8+i mul, E0+i: st(i) = st0 - st(i), E8+i: st(i) = st(i) - st0
                if kind == 0:
                    e = 'f.st(%d).fadd(f.st(0))' % i
                elif kind == 1:
                    e = 'f.st(%d).fmul(f.st(0))' % i
                elif kind == 4:
                    e = 'f.st(0).fsub(f.st(%d))' % i
                elif kind == 5:
                    e = 'f.st(%d).fsub(f.st(0))' % i
                else:
                    raise Exception(hex(a))
                emit('f.set(%d, %s);' % (i, e))
                if op0 == 0xDE:
                    emit('f.pop();')
            else:
                raise Exception(hex(a))
    elif mn in ('nop',):
        pass
    else:
        raise Exception('unhandled %x %s %s' % (a, mn, ops))

print('// generated from the bytes %s..%s by x87tr.py; entry-relative stack slots in `loc`' % (start, stop))
print('\n'.join(code))
print('// final esp', esp)
