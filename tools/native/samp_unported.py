#!/usr/bin/env python3
"""SA:MP-executed functions that native sa-game cannot run yet (docs/samp/native-plan.md).

Inputs (all committed):
  catalog/samp-priority.csv   SAMP-1D: functions the original executes under samp.dll 0.3.7-R4
  catalog/state.csv           per-function state (PLAN §5)
  crates/**/*.rs              `#[hook(addr = …)]` / `#[native(addr = …)]` registrations (ports written but not
                              yet recorded in state.csv), as tools/native/mvp_gaps.py counts them

A function counts as "in Rust" when its state is implemented/L1/L2/L3 or it has a registration;
"verified" when its state is L1/L2/L3. Rows with role `init` (C++ static initialisers: in phase B their
results come from the exe's .data) are counted separately and not listed.

Outputs:
  docs/samp/native-unported.csv   one row per SA:MP-executed non-init function not in Rust, ordered by
                                  subsystem (largest gap first), then by hits (most executed first)
  stdout                          markdown tables for native-plan.md

Usage: .venv/bin/python -I tools/native/samp_unported.py [--root DIR]
"""
import collections
import csv
import os
import re
import sys

RUST = {'implemented', 'L1', 'L2', 'L3'}
VERIFIED = {'L1', 'L2', 'L3'}


def main():
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    if '--root' in sys.argv:
        root = sys.argv[sys.argv.index('--root') + 1]
    state = {}
    with open(os.path.join(root, 'catalog/state.csv'), newline='') as f:
        for r in csv.DictReader(f):
            state[int(r['address'], 16)] = r['state']
    registered = set()
    pat = re.compile(r'#\[(?:[a-z_]+::)*(?:hook|native)\(addr = (0x[0-9A-Fa-f]+)')
    for d, _, files in os.walk(os.path.join(root, 'crates')):
        if '/target' in d:
            continue
        for fn in files:
            if fn.endswith('.rs'):
                with open(os.path.join(d, fn), encoding='utf-8', errors='replace') as f:
                    registered.update(int(m, 16) for m in pat.findall(f.read()))
    with open(os.path.join(root, 'catalog/samp-priority.csv'), newline='') as f:
        rows = list(csv.DictReader(f))

    per = collections.defaultdict(lambda: collections.Counter())
    gaps = []
    init_total = init_rust = 0
    for r in rows:
        a = int(r['address'], 16)
        st = state.get(a, r['state'])
        in_rust = st in RUST or a in registered
        if r['role'] == 'init':
            init_total += 1
            init_rust += in_rust
            continue
        c = per[r['subsystem']]
        c['total'] += 1
        if in_rust:
            c['rust'] += 1
            if st in VERIFIED:
                c['verified'] += 1
            elif st == 'implemented' or a in registered:
                c['unverified'] += 1
        else:
            c['gap'] += 1
            c['gap_' + ('specified' if st == 'specified' else 'unknown')] += 1
            gaps.append((r['subsystem'], int(r['hits'] or 0), a, r['name'], st, r['in_sp']))

    order = sorted(per, key=lambda s: (-per[s]['gap'], s))
    rank = {s: i for i, s in enumerate(order)}
    gaps.sort(key=lambda g: (rank[g[0]], -g[1], g[2]))
    out = os.path.join(root, 'docs/samp/native-unported.csv')
    with open(out, 'w', newline='') as f:
        w = csv.writer(f, lineterminator='\n')
        w.writerow(['subsystem', 'address', 'name', 'state', 'hits', 'in_sp'])
        for s, hits, a, name, st, in_sp in gaps:
            w.writerow([s, f'0x{a:x}', name, st, hits, in_sp])

    tot = collections.Counter()
    print('| subsystem | executed (non-init) | in Rust | of which verified (L1–L3) | not in Rust | of which specified |')
    print('|---|---:|---:|---:|---:|---:|')
    for s in order:
        c = per[s]
        tot.update(c)
        print(f"| {s} | {c['total']} | {c['rust']} | {c['verified']} | {c['gap']} | {c['gap_specified']} |")
    print(f"| **total** | **{tot['total']}** | **{tot['rust']}** | **{tot['verified']}** | **{tot['gap']}** |"
          f" **{tot['gap_specified']}** |")
    print()
    print(f'static initialisers (role init): {init_total}, in Rust {init_rust}')
    print(f'wrote {os.path.relpath(out, root)} ({len(gaps)} rows)')


if __name__ == '__main__':
    main()
