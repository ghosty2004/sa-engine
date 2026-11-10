#!/usr/bin/env python3
"""assign3.py WORKDIR [COVERAGE_HITS_CSV] : RE-SPEC-3 writer groups.

Splits WORKDIR/work.csv into writer groups WORKDIR/assign-<G>.txt. Each line:
  `address kind Llevel n=insns hits=H SUBSYS name -> spec_file`
spec_file is fixed here, so writers never share a file:
  - CRunningScript::ProcessCommandsXToY  -> docs/re/specs/SCRIPT/commands/ProcessCommandsXToY.md
    (groups S1..S3; per-opcode case evidence from opcases.py)
  - named class                           -> <SUB>/<Class>.md (full) / <Class>.deviations.md
    (a class larger than the cap is split; part k > 1 goes to <Class>.p<k>.md / .p<k>.deviations.md)
  - __init_* / __dtor_*                    -> <SUB>/_static-init-<G>.md
  - unnamed / vt_XXXXXX                    -> <SUB>/_unattributed-<G>.md
Weights: full spec = insns, deviation note = 0.3 * insns; cap per group CAP (default 8000).
AI is split in two passes: functions hit by the coverage corpus first (groups I1..), then the rest (J1..);
when a class appears in both passes the J writer appends to the existing file.
Groups: R=RENDER, H=HUD+SAVE, P=POP, U=AUDIO, C=SCRIPT (other), S=SCRIPT switches, I/J=AI."""
import csv
import os
import sys
from collections import defaultdict

W = sys.argv[1]
cov = {}
if len(sys.argv) > 2:
    cov = {r['address']: int(r['hits']) for r in csv.DictReader(open(sys.argv[2]))}
CAP = int(os.environ.get('CAP', '8000'))
rows = list(csv.DictReader(open(W + '/work.csv')))
PFX = {'RENDER': 'R', 'HUD': 'H', 'SAVE': 'H', 'POP': 'P', 'AUDIO': 'U', 'SCRIPT': 'C', 'AI': 'I'}


def weight(r):
    n = int(r['insns'] or 0)
    return n if r['kind'] == 'full-spec' else 0.3 * n


def cls_of(r):
    n = r['name']
    if n.startswith('__init_') or n.startswith('__dtor_'):
        return '_static'
    if '::' in n and not n.startswith('vt_'):
        c = n.split('::')[0]
        return c if not c.startswith('vt_') else '_unattr'
    return '_unattr'


groups = defaultdict(list)  # G -> [(row, file)]
# script switches
sw = [r for r in rows if 'ProcessCommands' in r['name']]
sw.sort(key=lambda r: int(r['address'], 16))
opw = defaultdict(int)
ocsv = W + '/opcases/index.csv'
if os.path.exists(ocsv):
    for o in csv.DictReader(open(ocsv)):
        opw[o['range_fn']] += int(o['insns'])
bins = [[0, []] for _ in range(3)]
for r in sorted(sw, key=lambda r: -opw[r['address']]):
    b = min(bins, key=lambda b: b[0])
    b[0] += opw[r['address']] + 60
    b[1].append(r)
for k, (_, rs) in enumerate(bins, 1):
    for r in sorted(rs, key=lambda r: int(r['address'], 16)):
        fn = r['name'].split('::')[1]
        groups[f'S{k}'].append((r, f'docs/re/specs/SCRIPT/commands/{fn}.md'))
rest = [r for r in rows if 'ProcessCommands' not in r['name']]


def pack(prefix, rs, start=1):
    """class-preserving first-fit packing in port-level order of each class's lowest level."""
    by = defaultdict(list)
    for r in rs:
        by[(r['subsystem'], cls_of(r))].append(r)
    def key(k):
        lv = [int(r['level']) if r['level'].isdigit() else 999 for r in by[k]]
        return (-sum(cov.get(r['address'], 0) > 0 for r in by[k]), min(lv), k)
    out = []  # list of [weight, [(row, cls_key, part)]]
    for k in sorted(by, key=key):
        items = sorted(by[k], key=lambda r: int(r['address'], 16))
        tot = sum(weight(r) for r in items)
        if k[1] in ('_static', '_unattr') or tot <= CAP:
            chunks = [items] if k[1] not in ('_static', '_unattr') else [[r] for r in items]
        else:
            chunks, cur, cw = [], [], 0
            for r in items:
                if cur and cw + weight(r) > CAP:
                    chunks.append(cur)
                    cur, cw = [], 0
                cur.append(r)
                cw += weight(r)
            chunks.append(cur)
        for p, ch in enumerate(chunks, 1):
            w = sum(weight(r) for r in ch)
            tgt = next((b for b in out if b[0] + w <= CAP), None)
            if tgt is None:
                tgt = [0, []]
                out.append(tgt)
            tgt[0] += w
            tgt[1] += [(r, k, p if len(chunks) > 1 and k[1] not in ('_static', '_unattr') else 1) for r in ch]
    for i, (_, items) in enumerate(out, start):
        g = f'{prefix}{i}'
        for r, (sub, c), p in items:
            d = f'docs/re/specs/{sub}/'
            dev = r['kind'] == 'deviation-note'
            if c == '_static':
                f = d + f'_static-init-{g}.md'
            elif c == '_unattr':
                f = d + f'_unattributed-{g}.md'
            else:
                f = d + c + (f'.p{p}' if p > 1 else '') + ('.deviations.md' if dev else '.md')
            groups[g].append((r, f))
    return len(out)


for pfx in ('R', 'H', 'P', 'U', 'C'):
    pack(pfx, [r for r in rest if PFX[r['subsystem']] == pfx])
ai = [r for r in rest if r['subsystem'] == 'AI']
pack('I', [r for r in ai if cov.get(r['address'], 0) > 0])
pack('J', [r for r in ai if cov.get(r['address'], 0) == 0])
for g, items in sorted(groups.items()):
    with open(f'{W}/assign-{g}.txt', 'w') as fh:
        for r, f in sorted(items, key=lambda x: (x[1], int(x[0]['address'], 16))):
            fh.write(f"{r['address']} {r['kind']} L{r['level']} n={r['insns']} hits={cov.get(r['address'], 0)} "
                     f"{r['subsystem']} {r['name']} -> {f}\n")
    print(g, len(items), int(sum(weight(r) if 'ProcessCommands' not in r['name'] else opw[r['address']]
                                for r, _ in items)), sum(r['kind'] == 'deviation-note' for r, _ in items))
