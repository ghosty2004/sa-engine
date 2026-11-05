#!/usr/bin/env python3
"""select.py OUTDIR SUBSYSTEM... : pick the RE-SPEC work list for the given subsystems.

Rule (RE-SPEC-1, extended in RE-SPEC-2):
  full-spec       source != gta-reversed (REA / empty), or state == unknown, or a gta-reversed hook that is
                  not reversed (`.Reversed = false`), or "called from gta-reversed only" (plugin::Call*).
  deviation-note  any other gta-reversed function with gr_deviations > 0.
Rows already in catalog/specs-index.csv are skipped. Writes OUTDIR/work.csv (sorted by port level, then
address) and OUTDIR/addrs.txt (all addresses, for decomp.sh and bundle.py).
Run with: .venv/bin/python -I tools/specs/select.py OUTDIR ANIM CAM ...
"""
import csv
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
C = ROOT + '/catalog/'
out_dir = sys.argv[1]
subs = set(sys.argv[2:])
os.makedirs(out_dir, exist_ok=True)
po = {r['address']: r for r in csv.DictReader(open(C + 'port-order.csv'))}
ext = {r['address']: r for r in csv.DictReader(open(C + 'extents.csv'))}
done = {r['address'] for r in csv.DictReader(open(C + 'specs-index.csv'))}
rows = []
for r in csv.DictReader(open(C + 'state.csv')):
    if r['subsystem'] not in subs or r['address'] in done:
        continue
    ev = r['evidence']
    full = (r['source'] != 'gta-reversed' or r['state'] == 'unknown' or 'not reversed' in ev
            or 'called from gta-reversed only' in ev)
    if full:
        kind = 'full-spec'
    elif r['gr_deviations'] not in ('', '0'):
        kind = 'deviation-note'
    else:
        continue
    lvl = po.get(r['address'], {}).get('level', '')
    e = ext.get(r['address'], {})
    rows.append(dict(address=r['address'], name=r['name'], subsystem=r['subsystem'], kind=kind,
                     state=r['state'], source=r['source'], gr_dev=r['gr_deviations'], level=lvl,
                     size=e.get('size', ''), insns=e.get('n_insns', ''), evidence=ev[:80]))
rows.sort(key=lambda r: (int(r['level']) if r['level'].isdigit() else 999, int(r['address'], 16)))
with open(out_dir + '/work.csv', 'w', newline='') as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
    w.writeheader()
    w.writerows(rows)
open(out_dir + '/addrs.txt', 'w').write(''.join(r['address'] + '\n' for r in rows))
from collections import Counter
print(Counter((r['subsystem'], r['kind']) for r in rows), len(rows))
