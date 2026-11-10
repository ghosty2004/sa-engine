#!/usr/bin/env python3
"""opcases.py OUTDIR [STATUS...] : per-opcode evidence for the CRunningScript::ProcessCommandsXToY switches (RE-SPEC-3).

For every opcode of catalog/script-commands.csv whose gta-reversed status is in STATUS (default: unregistered
unimplemented handler) and that has a `case` in the original switch, writes the case block disassembly into
OUTDIR/opcases/<range_fn>.txt. A case block runs from the case address to the next distinct case/default label of
the same switch (or the end of the extent); shared tails reached by `jmp` are NOT included (follow them with
dis.py LO-HI). Each section header carries: opcode, name, gr status / source, opcodes sharing the block, parameter
reads from catalog/script-params.csv, usage counts from catalog/script-usage.csv. Memory operands are annotated with
f32/f64 values and catalogue global names; call targets with catalogue names.
Also writes OUTDIR/opcases/index.csv (opcode,name,gr_status,range_fn,case,block_end,insns,used).
Run: .venv/bin/python -I tools/specs/opcases.py OUTDIR [unregistered unimplemented handler]"""
import csv
import os
import struct
import sys
from collections import defaultdict

import capstone
import pefile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
C = ROOT + '/catalog/'
out = sys.argv[1] + '/opcases'
want = set(sys.argv[2:]) or {'unregistered', 'unimplemented', 'handler'}
os.makedirs(out, exist_ok=True)
BASE = 0x400000
pe = pefile.PE(ROOT + '/re/gta/1.0-US/gta_sa.exe')
img = pe.get_memory_mapped_image()
secs = [(s.Name.rstrip(b'\0').decode(), s.VirtualAddress + BASE, s.VirtualAddress + BASE + s.Misc_VirtualSize)
        for s in pe.sections]


def sec(a):
    for n, lo, hi in secs:
        if lo <= a < hi:
            return n


def rd(a, n):
    return img[a - BASE:a - BASE + n]


iat = {}
for e in getattr(pe, 'DIRECTORY_ENTRY_IMPORT', []):
    for i in e.imports:
        iat[i.address] = e.dll.decode() + '!' + (i.name.decode() if i.name else '#%d' % i.ordinal)
names = {}
for r in csv.DictReader(open(C + 'procedures.csv')):
    names[int(r['address'], 16)] = r['name'] or r['gr_name'] or r['ghidra_name'] or ''
for r in csv.DictReader(open(C + 'state.csv')):
    if r['name']:
        names[int(r['address'], 16)] = r['name']
gname = {}
for r in csv.DictReader(open(C + 'rwsets.csv')):
    if r['name']:
        gname[int(r['address'], 16)] = r['name']
ext = {r['address']: r for r in csv.DictReader(open(C + 'extents.csv'))}
params = {r['opcode']: r for r in csv.DictReader(open(C + 'script-params.csv'))}
usage = {r['opcode']: r for r in csv.DictReader(open(C + 'script-usage.csv'))}
cmds = list(csv.DictReader(open(C + 'script-commands.csv')))
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
md.detail = True


def memval(d, size):
    n = sec(d)
    s = []
    if d in gname:
        s.append(gname[d])
    if n in ('.rdata', '.data'):
        b = rd(d, 8)
        if size == 8:
            s.append('[%s] f64=%r' % (n, struct.unpack('<d', b)[0]))
        else:
            s.append('[%s] u32=0x%x f32=%r' % (n, struct.unpack('<I', b[:4])[0], struct.unpack('<f', b[:4])[0]))
    return (' ; ' + ' '.join(s)) if s else ''


def dis(lo, hi):
    o = []
    for i in md.disasm(rd(lo, hi - lo), lo):
        c = ''
        for op in i.operands:
            if op.type == capstone.x86.X86_OP_MEM and op.mem.disp and (op.mem.disp & 0xffffffff) >= 0x401000:
                d = op.mem.disp & 0xffffffff
                if d in iat:
                    c += ' ; ' + iat[d]
                elif op.mem.base == 0 and op.mem.index == 0:
                    c += memval(d, op.size)
                elif d in gname:
                    c += ' ; table ' + gname[d]
            elif op.type == capstone.x86.X86_OP_IMM:
                v = op.imm & 0xffffffff
                if i.mnemonic == 'call' or i.mnemonic.startswith('j'):
                    if names.get(v):
                        c += ' ; ' + names[v]
                elif v >= 0x401000 and sec(v):
                    c += ' ; &' + (names.get(v) or gname.get(v) or '[%s]' % sec(v))
                elif 0x3a000000 <= v < 0x4f800000 or 0xba000000 <= v < 0xcf800000:
                    c += ' ; f32=%r' % struct.unpack('<f', struct.pack('<I', v))[0]
        o.append('%08x  %-44s%s' % (i.address, i.mnemonic + ' ' + i.op_str, c))
    return o


# labels per switch: every case address of that range fn (all statuses) plus its extent ends
labels = defaultdict(set)
for r in cmds:
    if r['orig_dispatch'] == 'case':
        labels[r['orig_range_fn']].add(int(r['orig_handler'], 16))
share = defaultdict(list)
for r in cmds:
    if r['orig_dispatch'] == 'case':
        share[(r['orig_range_fn'], r['orig_handler'])].append(r['opcode'])
ends = {}
for fn in labels:
    rngs = [[int(x, 16) for x in rg.split('-')] for rg in ext[fn]['ranges'].split(';')]
    ends[fn] = rngs
idx = []
per_fn = defaultdict(list)
for r in cmds:
    if r['orig_dispatch'] != 'case' or r['gr_status'] not in want:
        continue
    fn = r['orig_range_fn']
    a = int(r['orig_handler'], 16)
    hi = None
    for lo_, hi_ in ends[fn]:
        if lo_ <= a < hi_:
            hi = hi_
    nxt = [x for x in labels[fn] if x > a] + [hi or a + 0x200]
    b = min(nxt)
    lines = dis(a, b)
    u = usage.get(r['opcode'])
    used = (int(u['count_main']) + int(u['count_missions']) + int(u['count_streamed'])) if u else 0
    p = params.get(r['opcode'], {})
    hdr = (f"### {r['opcode']} {r['name']}  gr={r['gr_status']} {r['gr_source']}  case={r['orig_handler']} "
           f"block_end={b:#x} insns={len(lines)} shared_with={','.join(x for x in share[(fn, r['orig_handler'])] if x != r['opcode'])}\n"
           f"params: reads={p.get('reads', '')} n={p.get('params', '')} status={p.get('status', '')} {p.get('note', '')}\n"
           f"script usage: main={u['count_main'] if u else 0} missions={u['count_missions'] if u else 0} "
           f"streamed={u['count_streamed'] if u else 0}")
    per_fn[fn].append((a, hdr + '\n' + '\n'.join(lines) + '\n'))
    idx.append(dict(opcode=r['opcode'], name=r['name'], gr_status=r['gr_status'], range_fn=fn,
                    case=r['orig_handler'], block_end='%#x' % b, insns=len(lines), used=used))
for fn, secs_ in per_fn.items():
    nm = names.get(int(fn, 16), fn).split('::')[-1]
    with open(f'{out}/{fn}_{nm}.txt', 'w') as f:
        f.write(f'# {fn} {nm}: case blocks ({len(secs_)} opcodes)\n\n')
        for _, s in sorted(secs_):
            f.write(s + '\n')
with open(out + '/index.csv', 'w', newline='') as f:
    w = csv.DictWriter(f, fieldnames=list(idx[0].keys()))
    w.writeheader()
    w.writerows(idx)
print(len(idx), 'opcodes,', sum(x['insns'] for x in idx), 'insns,', len(per_fn), 'switches')
# opcodes whose number falls into a switch but that reach its default label (gta-reversed may still have a handler)
with open(out + '/defaults.csv', 'w', newline='') as f:
    w = csv.writer(f)
    w.writerow(['opcode', 'name', 'gr_status', 'range_fn', 'gr_source'])
    for r in cmds:
        if r['orig_dispatch'] == 'default':
            w.writerow([r['opcode'], r['name'], r['gr_status'], r['orig_range_fn'], r['gr_source']])
