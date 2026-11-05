#!/usr/bin/env python3
"""CAT-11: per-function state file and progress report.

catalog/state.csv holds one row per catalogue function (procedures.csv: game .text + _rwcseg;
a `.HOODLUM` body is part of its stub's row):

    address, name, subsystem, state, source, evidence, updated, gr_deviations

`state` follows docs/PLAN.md section 5: unknown, specified, implemented, L1, L2, L3, excluded.

  progress.py merge [catalog-dir]
      Derive the catalogue state of every function and merge it into state.csv:
        excluded rows (CAT-5)              -> excluded (evidence = class)
        gr-reversed                        -> specified, source gta-reversed
        gr-unreversed / gr-called-only /
        unknown with a name                -> specified, source REA (the original must be analysed)
        unknown / called-only without name -> unknown
      A row whose state is above `specified` (implemented, L1, L2, L3) is never overwritten: only
      its name/subsystem are refreshed from the catalogue. Such a row whose address left the
      catalogue is kept and reported as an orphan. `updated` changes only when the state,
      source or evidence of a row changes (date from $SA_STATE_DATE, default today), so a
      rerun on the same catalogue writes a byte-identical file.

      `gr_deviations` (CAT-14) = number of catalog/gr-deviations.csv rows for the function whose
      `deviates` is yes or maybe (gta-reversed differs or may differ from the original there; check
      those rows before porting). Recomputed on every merge; it never changes `updated`.

  progress.py set ADDRESS STATE EVIDENCE [catalog-dir]
      Record a state change (for example `implemented` when a hook is registered, `L1`/`L2`/`L3`
      when the oracle comparison passed). EVIDENCE must say where the proof is.

  progress.py report [catalog-dir]
      Write catalog/progress.md: counts per subsystem and state, and F1 progress = share of
      non-excluded functions at L2 or L3.

Without a command: merge, then report. Run with .venv/bin/python -I.
"""
import csv
import datetime
import os
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
STATES = ['unknown', 'specified', 'implemented', 'L1', 'L2', 'L3', 'excluded']
ABOVE_SPECIFIED = ('implemented', 'L1', 'L2', 'L3')
AT_LEAST_L2 = ('L2', 'L3')
COLS = ['address', 'name', 'subsystem', 'state', 'source', 'evidence', 'updated', 'gr_deviations']


def today():
    return os.environ.get('SA_STATE_DATE') or datetime.date.today().isoformat()


def read_state(cat):
    p = cat / 'state.csv'
    if not p.exists():
        return {}
    return {int(r['address'], 16): r for r in csv.DictReader(open(p))}


def write_state(cat, rows):
    with open(cat / 'state.csv', 'w', newline='') as f:
        w = csv.DictWriter(f, COLS, lineterminator='\n')
        w.writeheader()
        for a in sorted(rows):
            w.writerow({k: rows[a].get(k, '') for k in COLS})


def derive(cat):
    """{address: row without `updated`} from procedures.csv + classification.csv."""
    cls = {int(r['address'], 16): r for r in csv.DictReader(open(cat / 'classification.csv'))}
    out = {}
    for p in csv.DictReader(open(cat / 'procedures.csv')):
        a = int(p['address'], 16)
        c = cls.get(a)
        name = p['gr_name'] or (c['name'] if c else '')
        st = p['state']
        if st == 'excluded':
            state, source, ev = 'excluded', '', f"CAT-5 class {p['class']}"
        elif st == 'gr-reversed':
            state, source, ev = 'specified', 'gta-reversed', 'gta-reversed hook, reversed'
        elif name:
            state, source = 'specified', 'REA'
            ev = {'gr-unreversed': 'gta-reversed hook, not reversed (.Reversed = false)',
                  'gr-called-only': 'named, called from gta-reversed only (plugin::Call / thunk)'}.get(
                st, f"named by CAT-5/CAT-12 (class {p['class'] or 'game'})")
        else:
            state, source = 'unknown', 'REA'
            ev = {'gr-called-only': 'called from gta-reversed (plugin::Call / thunk), no name',
                  'gr-unreversed': 'gta-reversed hook without a name'}.get(
                st, f"unnamed (class {p['class'] or 'game'})")
        out[a] = {'address': f'{a:#08x}', 'name': name, 'subsystem': p['subsystem'], 'state': state,
                  'source': source, 'evidence': ev}
    return out


def gr_deviation_counts(cat):
    """{address: rows of catalog/gr-deviations.csv with deviates yes/maybe} (CAT-14)."""
    p = cat / 'gr-deviations.csv'
    n = Counter()
    if p.exists():
        for r in csv.DictReader(open(p)):
            if r['address'] and r['deviates'] != 'no':
                n[int(r['address'], 16)] += 1
    return n


def merge(cat):
    old = read_state(cat)
    new = derive(cat)
    devs = gr_deviation_counts(cat)
    d = today()
    rows, kept, changed, orphans = {}, 0, 0, 0
    for a, r in new.items():
        o = old.get(a)
        if o and o['state'] in ABOVE_SPECIFIED:
            rows[a] = {**o, 'name': r['name'] or o['name'], 'subsystem': r['subsystem']}
            kept += 1
            continue
        same = o and all(o[k] == r[k] for k in ('state', 'source', 'evidence'))
        rows[a] = {**r, 'updated': o['updated'] if same else d}
        changed += 0 if same else 1
    for a, o in old.items():
        if a not in new and o['state'] in ABOVE_SPECIFIED:
            ev = o['evidence']
            if not ev.startswith('ORPHAN'):
                ev = 'ORPHAN: address no longer in the catalogue; ' + ev
            rows[a] = {**o, 'evidence': ev}
            orphans += 1
    for a, r in rows.items():
        r['gr_deviations'] = str(devs.get(a, 0))
    write_state(cat, rows)
    print(f'state.csv: {len(rows)} rows, {kept} kept above specified, {changed} new/changed, '
          f'{orphans} orphans')


def set_state(cat, addr, state, evidence):
    assert state in STATES, f'state must be one of {STATES}'
    rows = read_state(cat)
    a = int(addr, 16)
    assert a in rows, f'{addr} is not in state.csv (run merge first)'
    rows[a] = {**rows[a], 'state': state, 'evidence': evidence, 'updated': today()}
    write_state(cat, rows)
    print(f'{addr} {rows[a]["name"]}: {state}')


def report(cat):
    rows = read_state(cat)
    by = defaultdict(Counter)
    for r in rows.values():
        by[r['subsystem']][r['state']] += 1
    tot = Counter(r['state'] for r in rows.values())
    live = sum(n for s, n in tot.items() if s != 'excluded')
    done = sum(tot[s] for s in AT_LEAST_L2)
    with open(cat / 'progress.md', 'w') as f:
        f.write('# Port progress (generated by tools/catalog/progress.py from catalog/state.csv)\n\n')
        f.write(f'**F1 progress: {done} / {live} non-excluded functions at L2 or L3 = '
                f'{100.0 * done / live:.2f} %.**\n\n')
        f.write(f'Functions: {len(rows)} ({live} non-excluded, {tot["excluded"]} excluded). States per '
                f'docs/PLAN.md section 5. `specified` with source gta-reversed = ported from its C++; '
                f'source REA = the original has to be analysed first.\n\n')
        f.write('| Subsystem | ' + ' | '.join(STATES) + ' | total | ≥L2 % |\n|---|' + '---|' * (len(STATES) + 2)
                + '\n')
        for sid in sorted(by, key=lambda s: (s.startswith('X-'), -sum(by[s].values()), s)):
            n = [by[sid][s] for s in STATES]
            lv = sum(n) - by[sid]['excluded']
            pct = f'{100.0 * sum(by[sid][s] for s in AT_LEAST_L2) / lv:.1f}' if lv else '-'
            f.write(f'| {sid} | ' + ' | '.join(map(str, n)) + f' | {sum(n)} | {pct} |\n')
        f.write('| **total** | ' + ' | '.join(str(tot[s]) for s in STATES) +
                f' | {len(rows)} | {100.0 * done / live:.2f} |\n')
        src = Counter((r['state'], r['source']) for r in rows.values() if r['state'] == 'specified')
        f.write('\n`specified` by source: ' + ', '.join(f'{s} {n}' for (_st, s), n in sorted(src.items())) + '\n')
        above = sorted((a, r) for a, r in rows.items() if r['state'] in ABOVE_SPECIFIED)
        f.write(f'\n## Functions above `specified` ({len(above)})\n\n| address | name | subsystem | state | '
                f'evidence | updated |\n|---|---|---|---|---|---|\n')
        for a, r in above:
            f.write(f"| {r['address']} | {r['name']} | {r['subsystem']} | {r['state']} | {r['evidence']} | "
                    f"{r['updated']} |\n")
    print(open(cat / 'progress.md').read())


def main():
    args = sys.argv[1:]
    cmd = args[0] if args and args[0] in ('merge', 'report', 'set') else None
    if cmd == 'set':
        cat = Path(args[4]) if len(args) > 4 else ROOT / 'catalog'
        return set_state(cat, args[1], args[2], args[3])
    rest = args[1:] if cmd else args
    cat = Path(rest[0]) if rest else ROOT / 'catalog'
    if cmd in (None, 'merge'):
        merge(cat)
    if cmd in (None, 'report'):
        report(cat)


if __name__ == '__main__':
    main()
