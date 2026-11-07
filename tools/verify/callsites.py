"""Verifier helper (docs/re/verify-l1-core-stream.md). For each claimed function: direct call/jmp sites and data references, and how the caller first uses
EAX/EDX after the call (read of the full register vs. only AL)."""
import sys, struct, re
import os, csv
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sadis import pe, img, BASE, md, rd
import capstone
from capstone import x86

text_lo, text_hi = 0x401000, 0x857000
blob = img[text_lo - BASE:text_hi - BASE]
whole = img

_root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
claims = [[r['address'], r['evidence'].split(':')[0], r['name']]
          for r in csv.DictReader(open(os.path.join(_root, 'catalog/state.csv')))
          if r['state'] == 'L1' and re.search('PORT-(CORE|STREAM)', r['evidence'])]

def sites(target):
    out = []
    for m in re.finditer(b'[\xe8\xe9]', blob):
        o = m.start()
        if o + 5 > len(blob):
            continue
        rel = struct.unpack_from('<i', blob, o + 1)[0]
        src = text_lo + o
        if (src + 5 + rel) & 0xffffffff == target:
            ins = list(md.disasm(blob[o:o + 5], src))
            if ins and ins[0].size == 5:
                out.append((src, 'call' if blob[o] == 0xe8 else 'jmp'))
    return out

def datarefs(target):
    pat = struct.pack('<I', target)
    out = []
    for m in re.finditer(re.escape(pat), whole):
        a = BASE + m.start()
        if text_lo <= a < text_hi:
            continue
        out.append(a)
    return out

REGS = {'eax': 'eax', 'ax': 'ax', 'al': 'al', 'ah': 'ah', 'edx': 'edx', 'dx': 'edx', 'dl': 'edx', 'dh': 'edx'}

def use_after(site):
    """First instruction after the call that reads eax/ax/al/edx (or overwrites eax)."""
    res = []
    eax_dead = edx_dead = False
    for i in md.disasm(rd(site + 5, 80), site + 5):
        try:
            rr, ww = i.regs_access()
        except Exception:
            rr, ww = [], []
        r = {i.reg_name(x) for x in rr}
        w = {i.reg_name(x) for x in ww}
        # xor eax,eax / sub eax,eax reads but kills
        if i.mnemonic in ('xor', 'sub') and len(i.operands) == 2 and i.op_str.split(',')[0].strip() == i.op_str.split(',')[1].strip():
            r = set()
        txt = f"{i.address:#x} {i.mnemonic} {i.op_str}"
        if not eax_dead:
            hit = r & {'eax', 'ax', 'ah'}
            if hit:
                res.append(('EAX-full' if hit & {'eax', 'ax'} else 'AH', txt)); eax_dead = True
            elif 'al' in r:
                res.append(('AL', txt)); eax_dead = True
            elif w & {'eax'}:
                eax_dead = True
            elif w & {'al','ax'} and not w & {'eax'}:
                pass
        if not edx_dead:
            if r & {'edx', 'dx', 'dl', 'dh'}:
                res.append(('EDX', txt)); edx_dead = True
            elif 'edx' in w:
                edx_dead = True
        if i.mnemonic in ('ret', 'jmp', 'call') or i.mnemonic.startswith('j'):
            if not eax_dead:
                res.append(('flow', txt))
            break
        if eax_dead and edx_dead:
            break
    return res

def main():
  for c in claims:
    a = int(c[0], 16)
    s = sites(a)
    d = datarefs(a)
    print(f"== {c[0]} {c[2] if len(c) > 2 else ''}: {len(s)} sites, data refs {[hex(x) for x in d][:6]}")
    for site, kind in s[:40]:
        u = use_after(site) if kind == 'call' else [('tail', '')]
        print(f"   {kind} {site:#x}: {u}")


if __name__ == '__main__':
    main()
