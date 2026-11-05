#!/usr/bin/env python3
"""bundle.py WORKDIR : build one evidence bundle per address in WORKDIR/addrs.txt into WORKDIR/bundles/<addr>.txt.
Needs WORKDIR/decomp.c from decomp.sh. Each bundle: catalogue header, callees, callers, rwsets, gr-deviations rows,
capstone disassembly over the extent (.HOODLUM body included; memory operands annotated with f32/f64 values and
global names) and the Ghidra decompile. Run: .venv/bin/python -I tools/specs/bundle.py WORKDIR (RE-SPEC-1/2)."""
import csv, struct, re, sys, os, collections
import pefile, capstone
ROOT=os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
S=sys.argv[1]
BASE=0x400000
pe=pefile.PE(ROOT+'/re/gta/1.0-US/gta_sa.exe'); img=pe.get_memory_mapped_image()
secs=[(s.Name.rstrip(b'\0').decode(),s.VirtualAddress+BASE,s.VirtualAddress+BASE+s.Misc_VirtualSize,s.SizeOfRawData) for s in pe.sections]
def sec(a):
    for n,lo,hi,raw in secs:
        if lo<=a<hi: return n
def rd(a,n): return img[a-BASE:a-BASE+n]
md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_32); md.detail=True
iat={}
for e in getattr(pe,'DIRECTORY_ENTRY_IMPORT',[]):
    for i in e.imports: iat[i.address]=e.dll.decode()+'!'+(i.name.decode() if i.name else '#%d'%i.ordinal)
cat=f'{ROOT}/catalog/'
state={r['address']:r for r in csv.DictReader(open(cat+'state.csv'))}
procs={r['address']:r for r in csv.DictReader(open(cat+'procedures.csv'))}
po={r['address']:r for r in csv.DictReader(open(cat+'port-order.csv'))}
ext={r['address']:r for r in csv.DictReader(open(cat+'extents.csv'))}
callees=collections.defaultdict(list); callers=collections.defaultdict(list)
for r in csv.DictReader(open(cat+'callgraph.csv')):
    callees[r['caller']].append(r); callers[r['callee']].append(r)
rws=collections.defaultdict(list); gname={}
for r in csv.DictReader(open(cat+'rwsets.csv')):
    rws[r['function']].append(r)
    if r['name']: gname[r['address']]=r['name']
devs=collections.defaultdict(list)
for r in csv.DictReader(open(cat+'gr-deviations.csv')):
    if r['address']: devs[r['address']].append(r)
lay={}
dec={}
cur=None;buf=[]
for line in open(S+'/decomp.c'):
    if line.startswith('// ==== '):
        if cur: dec[cur]=''.join(buf)
        cur=line.split()[2];buf=[line]
    else: buf.append(line)
if cur: dec[cur]=''.join(buf)
def nm(a):
    a='0x%x'%a if isinstance(a,int) else a
    s=state.get(a); p=procs.get(a)
    n=(s and s['name']) or (p and p['name']) or ''
    return n
def memval(a,size):
    n=sec(a)
    if not n: return ''
    s=[]
    g=gname.get('0x%x'%a)
    if g: s.append(g)
    if n in('.rdata','.data','_rdata','_data') or n.startswith('.') :
        b=rd(a,8)
        if len(b)>=4:
            u=struct.unpack('<I',b[:4])[0]; f=struct.unpack('<f',b[:4])[0]
            if size==8 and len(b)==8:
                s.append('[%s] f64=%r'%(n,struct.unpack('<d',b)[0]))
            elif size==10:
                s.append('[%s] (80-bit)'%n)
            else:
                s.append('[%s] u32=0x%x f32=%r'%(n,u,f))
    return ' ; '+' '.join(s) if s else ''
def disasm(addr):
    e=ext.get(addr)
    if not e: return '(no extent)\n'
    out=[]
    for rg in e['ranges'].split(';'):
        lo,hi=[int(x,16) for x in rg.split('-')]
        code=rd(lo,hi-lo); off=0
        while off<len(code):
            last=None
            for i in md.disasm(code[off:],lo+off):
                last=i
                c=''
                for op in i.operands:
                    if op.type==capstone.x86.X86_OP_MEM and op.mem.base==0 and op.mem.index==0 and op.mem.disp:
                        d=op.mem.disp&0xffffffff
                        if d in iat: c+=' ; '+iat[d]
                        else: c+=memval(d,op.size)
                    elif op.type==capstone.x86.X86_OP_MEM and op.mem.disp and (op.mem.disp&0xffffffff)>0x800000 and sec(op.mem.disp&0xffffffff):
                        d=op.mem.disp&0xffffffff; g=gname.get('0x%x'%d)
                        c+=' ; table %s%s'%('0x%x'%d,(' '+g) if g else '')
                    elif op.type==capstone.x86.X86_OP_IMM:
                        v=op.imm&0xffffffff
                        if i.mnemonic in('call','jmp') or (i.mnemonic.startswith('j')):
                            n=nm(v)
                            if n: c+=' ; '+n
                        elif v>=0x401000 and sec(v):
                            n=nm(v) or gname.get('0x%x'%v,'')
                            if n: c+=' ; &'+n
                            elif sec(v) in('.rdata','.data'): c+=' ; &[%s]'%sec(v)
                        elif i.mnemonic in('mov','push') and 0x3a000000<=v<0x4f800000 or 0xba000000<=v<0xcf800000:
                            c+=' ; f32=%r'%struct.unpack('<f',struct.pack('<I',v))[0]
                out.append('%08x  %-40s%s'%(i.address,(i.mnemonic+' '+i.op_str),c))
            off=(last.address+last.size-lo) if last else off+1
    return '\n'.join(out)+'\n'
os.makedirs(S+'/bundles',exist_ok=True)
for a in [l.strip() for l in open(S+'/addrs.txt') if l.strip()]:
    s=state[a]; p=procs.get(a,{}); e=ext.get(a,{})
    o=[f"# {a} {s['name']}  subsystem={s['subsystem']} state={s['state']} source={s['source']} level={po.get(a,{}).get('level')} gr_dev={s['gr_deviations']} role={s['role']}",
       f"evidence: {s['evidence']}", f"name_hint: {p.get('name_hint','')}  gr_name={p.get('gr_name','')} ghidra={p.get('ghidra_name','')}",
       f"extent: {e.get('ranges')} size={e.get('size')} insns={e.get('n_insns')} hoodlum_body={e.get('hoodlum_body')}",
       "## callees"]
    for r in callees.get(a,[]): o.append(f"  {r['callee']} {r['kind']} {r['callee_name'] or nm(r['callee'])} sites={r['sites']}")
    o.append("## callers (%d)"%len(callers.get(a,[])))
    for r in callers.get(a,[])[:12]: o.append(f"  {r['caller']} {r['kind']} {nm(r['caller'])} sites={r['sites']}")
    o.append("## rwsets")
    for r in rws.get(a,[]): o.append(f"  {r['address']} {r['section']} {r['name']} near={r['near']} size={r['size']} {r['access']} {r['mode']}")
    if devs.get(a):
        o.append("## gr-deviations")
        for r in devs[a]: o.append(f"  {r['kind']} deviates={r['deviates']} class={r['classification']} {r['file']}:{r['line']} gr_fn={r['gr_function']} :: {r['description'][:300]} || ev={r['evidence'][:200]}")
    o.append("## disassembly"); o.append(disasm(a))
    o.append("## ghidra decompile"); o.append(dec.get(a,'(none)'))
    open(f"{S}/bundles/{a}.txt",'w').write('\n'.join(o))
print('ok')
