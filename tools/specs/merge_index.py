#!/usr/bin/env python3
"""merge_index.py WORKDIR [--write] : check the writers' index fragments (WORKDIR/index-*.csv) and append them
to catalog/specs-index.csv (existing rows are kept; an address already present is not added again).

Checks: every work.csv address has exactly one row; spec_file exists and has a `## 0xADDR` / `### 0xADDR`
section or a `| 0xADDR |` table row (trivial stubs) for it
(address compared case-insensitively); kind matches work.csv; confidence in high/medium/low. Without
--write it only reports."""
import csv
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
W = sys.argv[1]
write = '--write' in sys.argv
FIELDS = ['address', 'name', 'subsystem', 'spec_file', 'kind', 'confidence']
work = {r['address']: r for r in csv.DictReader(open(W + '/work.csv'))}
idx_path = ROOT + '/catalog/specs-index.csv'
existing = {r['address'] for r in csv.DictReader(open(idx_path))}
rows, problems = {}, []
sections = {}
for frag in sorted(glob.glob(W + '/index-*.csv')):
    for r in csv.DictReader(open(frag)):
        a = r['address'].strip().lower()
        a = '0x%x' % int(a, 16)
        r['address'] = a
        if a in rows:
            problems.append(f'dup {a} in {frag} and earlier')
            continue
        if a not in work:
            problems.append(f'not in work list: {a} ({frag})')
            continue
        f = ROOT + '/' + r['spec_file']
        if f not in sections:
            txt = open(f).read() if os.path.exists(f) else ''
            sections[f] = {('0x%x' % int(m, 16)) for m in re.findall(r'^(?:##+ |\| )(0x[0-9a-fA-F]+)', txt, re.M)}
        if a not in sections[f]:
            problems.append(f'no section for {a} in {r["spec_file"]}')
        if r['kind'] != work[a]['kind']:
            problems.append(f'kind {r["kind"]} != {work[a]["kind"]} for {a}')
        if r['confidence'] not in ('high', 'medium', 'low'):
            problems.append(f'confidence {r["confidence"]!r} for {a}')
        r['name'] = r['name'] or work[a]['name']
        r['subsystem'] = work[a]['subsystem']
        rows[a] = {k: r[k] for k in FIELDS}
missing = sorted(set(work) - set(rows), key=lambda x: int(x, 16))
print(f'{len(rows)} rows, {len(missing)} missing, {len(problems)} problems')
for p in problems:
    print(' ', p)
if missing:
    print('missing:', ' '.join(missing))
if write:
    new = [rows[a] for a in sorted(rows, key=lambda x: int(x, 16)) if a not in existing]
    with open(idx_path, 'a', newline='') as fh:
        csv.DictWriter(fh, fieldnames=FIELDS, lineterminator='\n').writerows(new)
    print(f'appended {len(new)} rows to catalog/specs-index.csv')
