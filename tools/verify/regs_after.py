import sys, re
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sadis import md, rd
from callsites import sites, claims
def first_reads(site):
    found={}
    dead=set()
    for i in md.disasm(rd(site+5,120), site+5):
        try: rr,ww=i.regs_access()
        except Exception: rr,ww=[],[]
        r={i.reg_name(x) for x in rr}; w={i.reg_name(x) for x in ww}
        ops=i.op_str.split(',')
        if i.mnemonic in('xor','sub') and len(ops)==2 and ops[0].strip()==ops[1].strip(): r=set()
        for fam,names in (('ecx',{'ecx','cx','cl','ch'}),('edx',{'edx','dx','dl','dh'})):
            if fam in dead or fam in found: continue
            if r & names: found[fam]=f"{i.address:#x} {i.mnemonic} {i.op_str}"
            elif fam in w: dead.add(fam)
        if i.mnemonic in('ret','call') or i.mnemonic.startswith('j'): break
    return found
for c in claims:
    a=int(c[0],16)
    for s,k in sites(a):
        if k!='call': continue
        f=first_reads(s)
        if f: print(c[0], c[2], f"site {s:#x}", f)
