#!/usr/bin/env python3
"""Merge the Ghidra procedure list (REA evidence bundle) with the gta-reversed
catalogue and classify every function of gta_sa.exe 1.0 US.

Usage: merge_ghidra.py <rea-evidence.json> <catalog-dir>
Writes <catalog-dir>/procedures.csv, imports.csv and appends to summary.md.
"""
import csv
import json
import sys
from collections import Counter
from pathlib import Path

REGIONS = [  # (lo, hi, name)
    (0x401000, 0x857000, 'game-text'),
    (0x857000, 0x858000, 'rw-cseg'),
    (0xC9E000, 0xCAF000, 'text-ha'),
    (0xCB1000, 0x12FB000, 'protection-text'),
    (0x12FB000, 0x1301000, 'init'),
    (0x1556000, 0x1577000, 'hoodlum'),
]


def region(va):
    for lo, hi, name in REGIONS:
        if lo <= va < hi:
            return name
    return 'other'


def main():
    ev, cat = sys.argv[1], Path(sys.argv[2])
    rows = next(r for r in json.load(open(ev))['records']
                if r['operation'] == 'list_procedures')['normalized_result']

    hooks = {}
    with open(cat / 'functions.csv') as f:
        for h in csv.DictReader(f):
            hooks.setdefault(int(h['address'], 16), h)
    stub_target = {}
    for a, h in hooks.items():
        if h['status'] == 'stub-to-.HOODLUM':
            stub_target[int(h['first_insn'].split()[-1], 16)] = a
    callees = {}
    with open(cat / 'callees.csv') as f:
        for c in csv.DictReader(f):
            callees[int(c['address'], 16)] = c

    imports, procs = [], []
    for r in rows:
        if r['procedure'].get('external'):
            imports.append(r['value'])
            continue
        va = int(r['address'], 16)
        reg = region(va)
        h = hooks.get(va) or hooks.get(stub_target.get(va, -1))
        if h:
            state = 'gr-reversed' if h['reversed'] == 'yes' else 'gr-unreversed'
            name = f"{h['owner']}::{h['name']}" if h['owner'] else h['name']
        elif va in callees:
            state, name = 'gr-called-only', ''
        else:
            state, name = 'unknown', ''
        procs.append({'address': f'{va:#08x}', 'region': reg, 'state': state,
                      'gr_name': name, 'ghidra_name': r['value'],
                      'thunk': 'yes' if r['procedure'].get('thunk') else 'no'})

    # Ghidra's auto-analysis misses many small functions reached only through
    # vtables; gta-reversed hook addresses that decode as code are functions too.
    seen = {int(p['address'], 16) for p in procs}
    for va, h in hooks.items():
        if va not in seen and h['status'] == 'ok':
            procs.append({'address': f'{va:#08x}', 'region': region(va),
                          'state': 'gr-reversed' if h['reversed'] == 'yes' else 'gr-unreversed',
                          'gr_name': f"{h['owner']}::{h['name']}" if h['owner'] else h['name'],
                          'ghidra_name': '(missed by ghidra)', 'thunk': 'no'})
    procs.sort(key=lambda p: int(p['address'], 16))

    with open(cat / 'procedures.csv', 'w', newline='') as f:
        w = csv.DictWriter(f, list(procs[0]))
        w.writeheader()
        w.writerows(procs)
    with open(cat / 'imports.csv', 'w') as f:
        f.write('import\n' + '\n'.join(sorted(imports)) + '\n')

    by = Counter((p['region'], p['state']) for p in procs)
    regions = sorted({p['region'] for p in procs})
    states = ['gr-reversed', 'gr-unreversed', 'gr-called-only', 'unknown']
    dlls = Counter(i.split('::')[0] for i in imports)
    with open(cat / 'summary.md', 'a') as f:
        f.write('\n## Ghidra procedures by region and gta-reversed state\n\n')
        f.write(f'Total procedures: {len(procs)} (+{len(imports)} imports)\n\n')
        f.write('| Region | ' + ' | '.join(states) + ' | total |\n|---|' + '---|' * (len(states) + 1) + '\n')
        for reg in regions:
            n = [by[(reg, s)] for s in states]
            f.write(f'| {reg} | ' + ' | '.join(map(str, n)) + f' | {sum(n)} |\n')
        f.write('\nImports by DLL: ' + ', '.join(f'{k} {v}' for k, v in dlls.most_common()) + '\n')
    print(open(cat / 'summary.md').read().split('## Ghidra')[1])


if __name__ == '__main__':
    main()
