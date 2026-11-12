#!/usr/bin/env python3
"""std7_ports.py BASE: writes docs/arch/std7-ports.csv, the ports STD-7 added (its own first-parent, non-merge commits after BASE;
the cherry-picked WRITER-MVP-L commits are left out).

One row per `#[hook]` / `#[native]` registration added by those commits: address, catalogue name, kind,
batch (the `unverified` id of a hook), crate file. Deterministic (sorted by address).
"""
import csv
import re
import subprocess
import sys

base = sys.argv[1] if len(sys.argv) > 1 else '8b8890d'
SKIP = ('599af1e', 'e4c30e5', 'babf676')  # WRITER-MVP-L, cherry-picked
shas = subprocess.run(['git', 'log', '--first-parent', '--no-merges', '--format=%h', base + '..HEAD'],
                      capture_output=True, text=True, check=True).stdout.split()
log = ''
for sha in shas:
    if any(sha.startswith(x) or x.startswith(sha) for x in SKIP):
        continue
    log += subprocess.run(['git', 'show', '--format=', '-p', '--unified=0', sha, '--', 'crates'],
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
# every hook whose batch is an STD-7 one counts too
for a, (k, nm, b, f) in current.items():
    if b.startswith('std7-'):
        rows.setdefault(a, (k, nm, b, f))
# the lone-`ret` natives (`lone_rets!` table in sa-core std7_natives.rs, one shared entry)
t = open('crates/sa-core/src/std7_natives.rs').read()
for m in re.finditer(r'\(0x([0-9A-F]{6}), "([^"]+)"\)', t):
    a = int(m.group(1), 16)
    current.setdefault(a, ('native-lone-ret', m.group(2), '', 'crates/sa-core/src/std7_natives.rs'))
    rows.setdefault(a, current[a])
# the script opcodes STD-7 ported (cases in the partial switches; the switch hooks are unchanged)
OPCODES = ['06D1', '06D2', '075C', '0767', '076A', '076C', '0776', '0777', '0793', '0794', '07AF', '07D3', '07F7',
           '07FB', '0814', '0827', '0871', '0872', '0874', '087B', '0884', '090F', '0914', '0928', '0929', '0958',
           '0959', '095A', '09AE', '09B4', '09B7', '09CA', '09E2', '09E7', '09E8', '0A17']
w = csv.writer(open('docs/arch/std7-ports.csv', 'w', newline=''), lineterminator='\n')
w.writerow(['address', 'name', 'kind', 'batch', 'file'])
n = 0
for a in sorted(rows):
    if a not in current:
        continue
    kind, name, batch, f = current[a]
    if kind == 'hook' and not batch.startswith('std7-'):
        continue  # another writer's registration this branch only kept or moved
    w.writerow(['0x%X' % a, name, kind, batch, f])
    n += 1
for op in OPCODES:
    w.writerow(['opcode ' + op, '', 'opcode', '', 'crates/sa-script/src/cmd%d00.rs' % (int(op, 16) // 100)])
    n += 1
print('docs/arch/std7-ports.csv:', n, 'rows')
