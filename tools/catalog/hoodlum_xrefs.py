#!/usr/bin/env python3
"""Find direct E8/E9 rel32 branches whose target lies in a given section.
Usage: hoodlum_xrefs.py <gta_sa.exe> <section-name>"""
import struct, sys
from collections import Counter
import pefile
pe = pefile.PE(sys.argv[1], fast_load=True); base = pe.OPTIONAL_HEADER.ImageBase
secs = {s.Name.rstrip(b'\0').decode() + f'@{s.VirtualAddress:#x}': s for s in pe.sections}
tgt = [s for k, s in secs.items() if k.startswith(sys.argv[2] + '@')][0]
lo, hi = base + tgt.VirtualAddress, base + tgt.VirtualAddress + tgt.Misc_VirtualSize
for k, s in secs.items():
    data = s.get_data(); va0 = base + s.VirtualAddress; hits = Counter()
    for i in range(len(data) - 5):
        if data[i] in (0xE8, 0xE9):
            t = va0 + i + 5 + struct.unpack_from('<i', data, i + 1)[0]
            if lo <= t < hi: hits[data[i]] += 1
    if hits: print(k, 'call', hits[0xE8], 'jmp', hits[0xE9])
