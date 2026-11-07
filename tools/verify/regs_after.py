"""Verifier helper (V-9, docs/re/verify-l1-core-stream.md): for each direct call to a claimed L1
function, the first read of ECX/EDX after the call before a write (straight-line code only, stops at
the first branch or call).

Superseded for the hook table by the sa-hook test `callers::callers_reading_ecx_edx_after_a_hook_are_declared`
(crates/sa-hook/src/callers.rs, VER-2): every hook, every path after the call, byte-wise register
tracking, and the exceptions (`push ecx/edx` as stack allocation or save, `lea r, [r]` and other
no-ops, `xor/sub/sbb r, r`, `or r, -1`, partial-register idioms, E8 bytes off an instruction
boundary). This script is kept as the quick scan the audit used."""
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
