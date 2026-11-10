#!/usr/bin/env python3
"""check_consts.py SPEC.md... : mechanical spot check of constant claims in spec files.

Finds float-literal `0xADDR = <number>` / `[0xADDR]=<number>` / `0xADDR (<number>)` claims where ADDR is in .rdata/.data and
compares the number with the f32 (and, if that fails, the f64 / u32 / i32) stored there. Prints mismatches.
A claim matches when the relative difference is < 1e-5 (spec text rounds floats)."""
import os
import re
import struct
import sys

import pefile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
pe = pefile.PE(ROOT + '/re/gta/1.0-US/gta_sa.exe', fast_load=True)
img = pe.get_memory_mapped_image()
BASE = 0x400000
DATA = [(s.VirtualAddress + BASE, s.VirtualAddress + BASE + s.Misc_VirtualSize) for s in pe.sections
        if s.Name.rstrip(b'\0') in (b'.rdata', b'.data')]
PAT = re.compile(r'\[?(0x[0-9A-Fa-f]{6})\]?\s*(?:=|\()\s*(-?[0-9][0-9.]*(?:e[-+]?\d+)?)\s*(f|d)?\b')
# second notation (RE-SPEC-3): `<number>f [0xADDR]` / `<number> (0xADDR)` (value first, address in brackets/parens)
PAT2 = re.compile(r'(?<![\w.])(-?[0-9][0-9.]*(?:e[-+]?\d+)?)(f|d)?\s*[\[(](0x[0-9A-Fa-f]{6})[\])]')


def vals(a):
    b = img[a - BASE:a - BASE + 8]
    out = [struct.unpack('<f', b[:4])[0], struct.unpack('<I', b[:4])[0], struct.unpack('<i', b[:4])[0]]
    if len(b) == 8:
        out.append(struct.unpack('<d', b)[0])
    out += [b[0], struct.unpack('<h', b[:2])[0]]
    return out


tot = bad = 0
for path in sys.argv[1:]:
    for ln, line in enumerate(open(path), 1):
        ms = [(m.group(1), m.group(2), m.group(3), line[m.end(2):m.end(2) + 1]) for m in PAT.finditer(line)]
        ms += [(m.group(3), m.group(1), m.group(2), '') for m in PAT2.finditer(line)]
        for addr, num, suf, nxt in ms:
            a = int(addr, 16)
            if not any(lo <= a < hi for lo, hi in DATA):
                continue
            # only float literals (`1.5`, `2e-3`, `3.0f`); integers are usually stores to globals, `1/32768` a ratio
            if nxt == '/' or not ('.' in num or 'e' in num or suf):
                continue
            try:
                v = float(num.rstrip('.'))
            except ValueError:
                continue
            tot += 1
            if not any(abs(x - v) <= 1e-5 * max(1.0, abs(v)) for x in vals(a)):
                bad += 1
                print(f'{os.path.relpath(path, ROOT)}:{ln}: {addr} claimed {num}, '
                      f'f32={vals(a)[0]!r} u32={vals(a)[1]:#x}')
print(f'{tot} constant claims checked, {bad} mismatches')
