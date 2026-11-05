#!/usr/bin/env python3
"""dis.py ADDR... : disassemble catalogue functions of gta_sa.exe 1.0 US over their extents, naming call
targets (catalogue names) and import cells. Also prints the hex of the first 16 bytes."""
import csv, sys, re
import pefile, capstone

import os
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
W = ROOT
pe = pefile.PE(ROOT + '/re/gta/1.0-US/gta_sa.exe', fast_load=False)
base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
imps = {}
for d in getattr(pe, 'DIRECTORY_ENTRY_IMPORT', []):
    for i in d.imports:
        imps[i.address] = (d.dll.decode() + '!' + (i.name.decode() if i.name else str(i.ordinal)))
names = {}
for r in csv.DictReader(open(W + '/catalog/procedures.csv')):
    a = int(r['address'], 16)
    names[a] = r['name'] or r['gr_name'] or r['ghidra_name'] or ''
ext = {}
for r in csv.DictReader(open(W + '/catalog/extents.csv')):
    ext[int(r['address'], 16)] = r['ranges']

def rd(va, n):
    return img[va - base: va - base + n]

md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)

def ann(ins):
    s = ins.op_str
    out = []
    for m in re.finditer(r'0x[0-9a-f]+', s):
        v = int(m.group(0), 16)
        if v in imps: out.append(imps[v])
        elif v in names and names[v]: out.append(names[v])
    if ins.mnemonic in ('call', 'jmp') and not out:
        try:
            v = int(s, 16)
            out.append(names.get(v, '?'))
        except ValueError:
            pass
    return ('   ; ' + ', '.join(out)) if out else ''

for arg in sys.argv[1:]:
    if '-' in arg:
        s0, e0 = [int(x, 16) for x in arg.split('-')]
        print(f'==== raw {s0:#x}-{e0:#x}')
        for ins in md.disasm(rd(s0, e0 - s0), s0):
            print(f'  {ins.address:08x}: {ins.mnemonic:6} {ins.op_str}{ann(ins)}')
        continue
    if arg.startswith('d'):
        a0, n = arg[1:].split(':')
        a0 = int(a0, 16); n = int(n)
        print(f'==== dwords {a0:#x}: ' + ' '.join(f'{int.from_bytes(rd(a0+4*k,4),"little"):#x}' for k in range(n)))
        continue
    if arg.startswith('b'):
        a0, n = arg[1:].split(':')
        a0 = int(a0, 16); n = int(n)
        print(f'==== bytes {a0:#x}: ' + rd(a0, n).hex(' '))
        continue
    a = int(arg, 16)
    print(f'==== {a:#x} {names.get(a, "")}  bytes: {rd(a, 16).hex(" ").upper()}')
    rngs = ext.get(a)
    if not rngs:
        rngs = f'{a:#x}-{a + 0x80:#x}'
    for rg in rngs.split(';'):
        s, e = [int(x, 16) for x in rg.split('-')]
        if e - s > 0x1200:
            e = s + 0x1200
            print('  (truncated)')
        print(f' -- {s:#x}-{e:#x}')
        for ins in md.disasm(rd(s, e - s), s):
            print(f'  {ins.address:08x}: {ins.mnemonic:6} {ins.op_str}{ann(ins)}')
