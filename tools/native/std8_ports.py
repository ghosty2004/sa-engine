#!/usr/bin/env python3
"""std8_ports.py: writes docs/arch/std8-ports.csv, the ports STD-8 wrote or registered.

Rows (sorted by address): every `#[hook]` of an `std8-*` batch; the `#[native]` registrations STD-8 added (listed
below by address: new ports and existing ports it registered); the table-driven C++ static initialisers of
crates/sa-core/src/static_inits.rs and the hand-written ones of static_init_kinds.rs. Columns: address, catalogue
name, kind, batch, file. Deterministic.
"""
import csv
import glob
import re

NATIVES = {
    0x4241A0, 0x4241B0, 0x4464C0, 0x4D9E10, 0x4E44F0, 0x4E4600, 0x50AE10, 0x611550, 0x665270, 0x6F4000,
    0x463750, 0x463770, 0x5DB2B0, 0x5DB290, 0x46B260, 0x5A7570, 0x72E860,
}

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
    if b.startswith('std8-') or (k == 'native' and a in NATIVES):
        rows[a] = (k, nm, b, f)
t = open('crates/sa-core/src/static_inits.rs').read()
for m in re.finditer(r'\((0x[0-9a-f]+), "([^"]+)", (\w+)<', t):
    rows.setdefault(int(m.group(1), 16), ('native-static-init:' + m.group(3), m.group(2), '',
                                          'crates/sa-core/src/static_inits.rs'))
t = open('crates/sa-core/src/static_init_kinds.rs').read()
for m in re.finditer(r'NativeReg \{\s*addr: (0x[0-9A-F]+),\s*name: "([^"]+)"', t):
    rows.setdefault(int(m.group(1), 16), ('native-static-init', m.group(2), '',
                                          'crates/sa-core/src/static_init_kinds.rs'))
missing = NATIVES - set(rows)
assert not missing, 'natives not found: ' + ', '.join(hex(a) for a in sorted(missing))
w = csv.writer(open('docs/arch/std8-ports.csv', 'w', newline=''), lineterminator='\n')
w.writerow(['address', 'name', 'kind', 'batch', 'file'])
for a in sorted(rows):
    k, nm, b, f = rows[a]
    w.writerow(['0x%X' % a, nm, k, b, f])
print('docs/arch/std8-ports.csv:', len(rows), 'rows')
