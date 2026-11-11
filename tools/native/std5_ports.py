#!/usr/bin/env python3
"""std5_ports.py BASE: writes docs/arch/std5-ports.csv, the ports STD-5 added (its own first-parent, non-merge commits after BASE).

One row per `#[hook]` / `#[native]` registration added by those commits: address, catalogue name, kind,
batch (the `unverified` id of a hook), crate file. Deterministic (sorted by address).
"""
import csv
import re
import subprocess
import sys

base = sys.argv[1] if len(sys.argv) > 1 else 'f6b0d46'
log = subprocess.run(['git', 'log', '--first-parent', '--no-merges', '-p', '--unified=0', base + '..HEAD', '--', 'crates'],
                     capture_output=True, text=True, check=True).stdout
rows = {}
cur = None
pending = None
for line in log.split('\n'):
    if line.startswith('+++ b/'):
        cur = line[6:]
        continue
    if line.startswith('+') and not line.startswith('+++'):
        t = line[1:]
        if pending is not None:
            pending += ' ' + t.strip()
        elif re.search(r'#\[(sa_core::|crate::)?(hook|native)\(', t):
            pending = t.strip()
        if pending is not None and pending.endswith(')]'):
            m = re.search(r'#\[(?:sa_core::|crate::)?(hook|native)\(addr = (0x[0-9A-Fa-f_]+).*?name = "([^"]+)"', pending)
            if m:
                addr = int(m.group(2).replace('_', ''), 16)
                b = re.search(r'unverified = "([^"]+)"', pending)
                rows.setdefault(addr, (m.group(1), m.group(3), b.group(1) if b else '', cur))
            pending = None
    elif not line.startswith('+'):
        pending = None
# the current registration of each address (kind, name, batch, file) from the tree
import glob
current = {}
for f in sorted(glob.glob('crates/**/*.rs', recursive=True)):
    t = open(f, errors='ignore').read()
    for m in re.finditer(r'#\[(?:sa_core::|crate::)?(hook|native)\((addr = (0x[0-9A-Fa-f_]+).*?)\)\]', t, re.S):
        attrs = m.group(2)
        a = int(m.group(3).replace('_', ''), 16)
        nm = re.search(r'name = "([^"]+)"', attrs)
        b = re.search(r'unverified = "([^"]+)"', attrs)
        current.setdefault(a, (m.group(1), nm.group(1) if nm else '', b.group(1) if b else '', f))
# every hook whose batch is an STD-5 one counts too (some were committed inside merge commits)
for a, (k, nm, b, f) in current.items():
    if b.startswith('std5-'):
        rows.setdefault(a, (k, nm, b, f))
w = csv.writer(open('docs/arch/std5-ports.csv', 'w', newline=''), lineterminator='\n')
w.writerow(['address', 'name', 'kind', 'batch', 'file'])
n = 0
for a in sorted(rows):
    if a not in current:
        continue
    kind, name, batch, f = current[a]
    w.writerow(['0x%X' % a, name, kind, batch, f])
    n += 1
print('docs/arch/std5-ports.csv:', n, 'rows')
