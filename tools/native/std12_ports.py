#!/usr/bin/env python3
"""std12_ports.py: writes docs/arch/std12-ports.csv, the ports STD-12 wrote.

Rows (sorted by address): every `#[hook]` of an `std12-*` batch and the `#[native]` registrations STD-12 added (listed
below by address). Columns: address, catalogue name, kind, batch, file. Deterministic.
"""
import csv
import glob
import re

NATIVES = {0x563470}

current = {}
for f in sorted(glob.glob('crates/**/*.rs', recursive=True)):
    t = open(f, errors='ignore').read()
    for m in re.finditer(r'#\[(?:sa_core::|crate::)?(hook|native)\((addr = (0x[0-9A-Fa-f_]+).*?)\)\]', t, re.S):
        attrs = m.group(2)
        a = int(m.group(3).replace('_', ''), 16)
        nm = re.search(r'name = "([^"]+)"', attrs)
        b = re.search(r'unverified = "([^"]+)"', attrs)
        current.setdefault(a, (m.group(1), nm.group(1) if nm else '', b.group(1) if b else '', f))

rows = {}
for a, (k, nm, b, f) in current.items():
    if b.startswith('std12-') or (k == 'native' and a in NATIVES):
        rows[a] = (k, nm, b, f)
missing = NATIVES - set(rows)
assert not missing, 'natives not found: ' + ', '.join(hex(a) for a in sorted(missing))
w = csv.writer(open('docs/arch/std12-ports.csv', 'w', newline=''), lineterminator='\n')
w.writerow(['address', 'name', 'kind', 'batch', 'file'])
for a in sorted(rows):
    k, nm, b, f = rows[a]
    w.writerow(['0x%X' % a, nm, k, b, f])
print('docs/arch/std12-ports.csv:', len(rows), 'rows')
