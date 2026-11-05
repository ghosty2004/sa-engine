#!/usr/bin/env python3
"""Merge the Ghidra procedure list (REA evidence bundle) with the gta-reversed
catalogue and classify every function of gta_sa.exe 1.0 US.

Usage: merge_ghidra.py <rea-evidence.json> <catalog-dir> [gta_sa.exe] [gta-reversed]
Writes <catalog-dir>/procedures.csv, imports.csv and appends to summary.md.

Catalogue membership (CAT-4, extended by CAT-12): Ghidra procedures, plus every code address the
evidence names as a function entry that Ghidra's auto-analysis missed:
  hook       gta-reversed hook address that decodes as code or is a `.HOODLUM` stub
  callee     gta-reversed plugin::Call / raw-thunk target that decodes as code
  vtable     slot of a gta-reversed vtable (RH_ScopedVirtualClass) pointing into game code
  cat7-call  direct call / tail-call target found in a catalogued body by CAT-7, listed with
             its callers in catalog/extra-code-addresses.csv (callgraph.py gaps maintains it)
  data-*     entry reached only through data: pointer-table slot or code immediate (CAT-12b),
             listed in catalog/data-code-addresses.csv (data_refs.py maintains it)

.HOODLUM (CAT-9): a `.HOODLUM` procedure that is the body of a game-.text `jmp` stub is not a
separate function. Its row gets state `stub-body` and the stub's name; the function is counted
once, at the stub (callgraph.py gives the stub's extent = stub + body).
"""
import csv
import json
import struct
import sys
from collections import Counter
from pathlib import Path

import pefile

sys.path.insert(0, str(Path(__file__).resolve().parent))
from vtables import VTables  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
BASE = 0x400000
GAME_CODE = (0x401000, 0x858000)
HOODLUM = (0x1556000, 0x1577000)

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
    exe = sys.argv[3] if len(sys.argv) > 3 else ROOT / 're' / 'gta' / '1.0-US' / 'gta_sa.exe'
    gr = sys.argv[4] if len(sys.argv) > 4 else ROOT / 're' / 'gta-reversed'
    img = pefile.PE(str(exe), fast_load=True).get_memory_mapped_image()
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
    # vtables; gta-reversed hook addresses that decode as code are functions too (CAT-4), and so
    # are callees, gta-reversed vtable slots and CAT-7 call targets (CAT-12).
    seen = {int(p['address'], 16) for p in procs}

    def add(va, rule):
        if va in seen:
            return
        seen.add(va)
        h = hooks.get(va)
        if h:
            state = 'gr-reversed' if h['reversed'] == 'yes' else 'gr-unreversed'
            name = f"{h['owner']}::{h['name']}" if h['owner'] else h['name']
        elif va in callees:
            state, name = 'gr-called-only', ''
        else:
            state, name = 'unknown', ''
        procs.append({'address': f'{va:#08x}', 'region': region(va), 'state': state, 'gr_name': name,
                      'ghidra_name': f'(missed by ghidra: {rule})', 'thunk': 'no'})
        added[rule] += 1

    added = Counter()
    for va, h in sorted(hooks.items()):
        if h['status'] in ('ok', 'stub-to-.HOODLUM'):
            add(va, 'hook')
    for va, c in sorted(callees.items()):
        if c['status'] == 'ok':
            add(va, 'callee')
    vts = VTables(exe, gr)
    for cls, (vt, n) in sorted(vts.vt.items(), key=lambda kv: kv[1]):
        for i in range(n):
            va = struct.unpack_from('<I', img, vt + 4 * i - BASE)[0]
            if GAME_CODE[0] <= va < GAME_CODE[1]:
                add(va, 'vtable')
    extra = cat / 'extra-code-addresses.csv'
    if extra.exists():
        for r in csv.DictReader(open(extra)):
            add(int(r['address'], 16), 'cat7-call')

    # CAT-12b: entries reached only through data (pointer tables, code immediates), found by
    # data_refs.py and appended to catalog/data-code-addresses.csv
    data = cat / 'data-code-addresses.csv'
    if data.exists():
        for r in csv.DictReader(open(data)):
            add(int(r['address'], 16), 'data-' + r['kind'].split(':')[0])

    # CAT-9: .HOODLUM bodies of game-.text stubs are counted at the stub.
    stub_body = {}
    for va in sorted(seen):
        if GAME_CODE[0] <= va < GAME_CODE[1]:
            b = img[va - BASE:va - BASE + 5]
            if b[0] == 0xE9:
                t = (va + 5 + struct.unpack('<i', b[1:5])[0]) & 0xFFFFFFFF
                if HOODLUM[0] <= t < HOODLUM[1]:
                    stub_body.setdefault(t, va)
    stub_name = {}
    for p in procs:
        stub_name[int(p['address'], 16)] = p['gr_name']
    for p in procs:
        va = int(p['address'], 16)
        if va in stub_body:
            p['state'] = 'stub-body'
            p['gr_name'] = stub_name.get(stub_body[va], '') or f'(body of stub {stub_body[va]:#x})'
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
    nbody = sum(1 for p in procs if p['state'] == 'stub-body')
    ngame = sum(1 for p in procs if p['region'] in ('game-text', 'rw-cseg'))
    with open(cat / 'summary.md', 'a') as f:
        f.write('\n## Procedures by region and gta-reversed state\n\n')
        f.write(f'Procedure rows: {len(procs)} ({len(procs) - sum(added.values())} Ghidra, '
                f'{sum(added.values())} added: ' + ', '.join(f'{k} {v}' for k, v in sorted(added.items())) +
                f') (+{len(imports)} imports).\n\n')
        f.write(f'Each function is counted once. {nbody} `.HOODLUM` procedures are bodies of game-.text '
                f'`jmp` stubs: they are listed as `stub-body` and counted at the stub (CAT-9). The '
                f'catalogue proper is game-text + rw-cseg: **{ngame}** functions, the same total as '
                f'`subsystems.md` and the rows of `procedures.csv`.\n\n')
        f.write('| Region | ' + ' | '.join(states) + ' | functions | stub-body (counted at the stub) |\n|---|'
                + '---|' * (len(states) + 2) + '\n')
        for reg in regions:
            n = [by[(reg, s)] for s in states]
            f.write(f'| {reg} | ' + ' | '.join(map(str, n)) + f' | {sum(n)} | {by[(reg, "stub-body")]} |\n')
        tot = [sum(by[(r, s)] for r in regions) for s in states]
        f.write('| **total** | ' + ' | '.join(map(str, tot)) + f' | {sum(tot)} | {nbody} |\n')
        f.write('\nImports by DLL: ' + ', '.join(f'{k} {v}' for k, v in dlls.most_common()) + '\n')
    print(open(cat / 'summary.md').read().split('## Procedures by region')[1])


if __name__ == '__main__':
    main()
