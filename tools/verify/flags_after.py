import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sadis import md, rd
from callsites import sites, claims
from capstone import x86
SETTERS=('add','sub','cmp','test','and','or','xor','inc','dec','neg','shl','shr','sar','sbb','adc','fcomi','fucomi','sahf','popf','bt','imul','mul','rol','ror')
for c in claims:
    a=int(c[0],16)
    for s,k in sites(a):
        if k!='call': continue
        for i in md.disasm(rd(s+5,60), s+5):
            m=i.mnemonic
            if m.startswith('j') and m!='jmp' or m.startswith('set') or m.startswith('cmov') or m in('adc','sbb','pushf','lahf'):
                print(c[0],c[2],hex(s),'FLAGS USED',hex(i.address),m,i.op_str); break
            if m.startswith(SETTERS) or m in ('call','ret','jmp'): break
