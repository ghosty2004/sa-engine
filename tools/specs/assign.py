#!/usr/bin/env python3
"""assign.py WORKDIR : split WORKDIR/work.csv into writer groups (WORKDIR/assign-<G>.txt), one class per group,
balanced by instruction count. The RE-SPEC-2 grouping is fixed below (class -> group); unnamed functions go
to their subsystem's group. Each assign file lists `address kind level insns name` lines, port level order."""
import csv
import sys
from collections import defaultdict

W = sys.argv[1]
GROUPS = {
    'A': {'CAM': ['CCam', 'CIdleCam', 'CMBlur', 'CHandShaker']},
    'B': {'CAM': ['CCamera', '*'], 'INPUT': ['*']},
    'C': {'VEH': ['CAutomobile', 'CBoat', 'CQuadBike', 'CMonsterTruck', 'CTrailer', 'CDamageManager', 'CDoor',
                  'cHandlingDataMgr']},
    'D': {'VEH': ['CBike', 'CBmx', 'CHeli', 'CPlane']},
    'E': {'VEH': ['*']},
    'F': {'PED': ['*'], 'ANIM': ['*']},
    'G': {'WEAPON': ['*']},
}


def group_of(sub, cls):
    for g, m in GROUPS.items():
        if cls in m.get(sub, []):
            return g
    for g, m in GROUPS.items():
        if '*' in m.get(sub, []):
            return g
    raise SystemExit(f'no group for {sub} {cls}')


out = defaultdict(list)
for r in csv.DictReader(open(W + '/work.csv')):
    cls = r['name'].split('::')[0] if '::' in r['name'] else ''
    out[group_of(r['subsystem'], cls)].append(r)
for g, rows in sorted(out.items()):
    with open(f'{W}/assign-{g}.txt', 'w') as f:
        for r in rows:
            f.write(f"{r['address']} {r['kind']} L{r['level']} n={r['insns']} {r['subsystem']} {r['name']}\n")
    print(g, len(rows), sum(int(r['insns'] or 0) for r in rows),
          sum(r['kind'] == 'deviation-note' for r in rows))
