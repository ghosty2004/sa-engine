#!/usr/bin/env python3
"""What the exe image contributes to a native sa-game run (PURE-1, ADR-005).

Reads the first-access traces `tools/pure/arena_trace.c` writes (one or more `trace.<pid>` files, any number of
runs; a byte counts as needed when its first access in any run was a read, or a write unless `--no-rmw`, since a
read-modify-write faults as a write) and the exe image, and reports:

- per section: bytes read first, and how many of them are non-zero in the image (a zero initial value needs no
  data: an owned static starts at zero anyway);
- the read bytes widened to data objects (object starts = `catalog/data-tables.csv` tables, `catalog/rwsets.csv`
  addresses, the hex literals of the Rust sources (`address_census.py --csv`), and pointers stored in the data
  sections; an object runs to the next start, at most one page), classified as IAT cells, vtables / function
  tables (code addresses: regenerated from the catalogue), pointer tables, strings, floats, data the CRT / third-party
  code alone references, and other data;
- code bytes (`.text`, `.HOODLUM`) read as data, matched with `catalog/code-data-reads.csv`;
- the pages left open after the fault budget (their later first reads are not seen, so every non-zero object on such
  a data page counts as needed).

`--keep FILE` writes the ranges an image cut down to the union would hold, for `arena_trace.c`'s `SA_IMAGE_KEEP`
hold-out check; `--objects FILE` lists the union's objects.

    .venv/bin/python -I tools/pure/arena_usage.py --census target/pure1/census.csv target/pure1/tr-*.[0-9]*
"""
import argparse
import bisect
import collections
import csv
import os
import struct
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
sys.path.insert(0, os.path.join(ROOT, 'tools', 'lib'))
import exe  # noqa: E402

DATA = [('.rdata', 0x858000, 0x8A4000), ('.data', 0x8A4000, 0x8E4000), ('_TEXT_HA/_rwdseg/.rsrc', 0xC9E000, 0xCB1000)]
CODE = [('.text/_rwcseg', 0x401000, 0x858000), ('.HOODLUM', 0x1556000, 0x1577000)]
IMAGE = (0x401000, 0x1577000)
# the CRT's initialiser tables (__xi / __xc), run by sa-game's cinit
CRT_INIT = (0x8A4000, 0x8A5A30)


def load_trace(path):
    d = open(path, 'rb').read()
    magic, lo, hi, npages = struct.unpack('<4I', d[:16])
    assert magic == 0x31525450, path
    n = hi - lo
    st = d[16:16 + n]
    faults = struct.unpack(f'<{npages}I', d[16 + n:16 + n + 4 * npages])
    sat = d[16 + n + 4 * npages:16 + n + 5 * npages]
    return lo, st, faults, sat


def byte(a):
    return exe.IMG[a - exe.BASE]


def u32(a):
    return exe.u32_at(a)


def in_data(a):
    return any(lo <= a < hi for _, lo, hi in DATA)


def is_string(a, end):
    n = 0
    while a + n < end:
        c = byte(a + n)
        if c == 0:
            return n >= 2
        if not (32 <= c < 127 or c in (9, 10, 13)):
            return False
        n += 1
    return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('traces', nargs='+')
    ap.add_argument('--no-rmw', dest='rmw', action='store_false',
                    help='count bytes written first as not needed (assumes no read-modify-write instruction wrote them)')
    ap.add_argument('--census', help='address_census.py --csv output (object starts from the Rust sources)')
    ap.add_argument('--objects', help='write the needed objects (the union) as CSV')
    ap.add_argument('--keep', help='write the ranges an image built from the union would hold (union objects, every '
                    'IAT cell, the code dwords read as data) for arena_trace.c\'s SA_IMAGE_KEEP check')
    args = ap.parse_args()

    need = None
    written = None
    sat_pages = set()
    lo0 = None
    for t in args.traces:
        lo, st, faults, sat = load_trace(t)
        lo0 = lo
        if need is None:
            need = bytearray(len(st))
            written = bytearray(len(st))
        for i, s in enumerate(st):
            if s == 1:
                need[i] = 1
            elif s == 2:
                written[i] = 1
        sat_pages |= {lo + p * 4096 for p, v in enumerate(sat) if v}

    def needed(a):
        # a write fault can be a read-modify-write (`add [x], eax`, `or byte [x], 1`): the page-fault error code says
        # only "write", so a byte written first counts as well (it matters only when its initial value is non-zero)
        return need[a - lo0] == 1 or (args.rmw and written[a - lo0] == 1)

    print(f'Traces: {len(args.traces)}\n')
    print('### Bytes whose first access was a read (or a write, unless --no-rmw), dword marks, per section\n')
    print('| section | image bytes (raw) | non-zero in image | read first | of them non-zero in image | pages left open |')
    print('|---|---|---|---|---|---|')
    for name, lo, hi in DATA + CODE:
        raw = [byte(a) for a in range(lo, hi)]
        nz = sum(1 for b in raw if b)
        rf = [a for a in range(lo, hi) if needed(a)]
        rfnz = sum(1 for a in rf if byte(a))
        so = sum(1 for p in sat_pages if lo <= p < hi)
        print(f'| {name} | {hi - lo:,} | {nz:,} | {len(rf):,} | {rfnz:,} | {so} |')

    # object starts
    starts = set()
    table_end = {}
    table_kind = {}
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'data-tables.csv'))):
        s, e = int(r['start'], 16), int(r['end'], 16)
        starts.add(s)
        table_end[s] = e
        table_kind[s] = r['kind'].split(':')[0]
    readers = collections.defaultdict(set)
    names = {}
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'rwsets.csv'))):
        try:
            a = int(r['address'], 16)
        except ValueError:
            continue
        if in_data(a):
            starts.add(a)
            readers[a].add(int(r['function'], 16))
            if r['name']:
                names[a] = r['name']
    if args.census:
        for r in csv.DictReader(open(args.census)):
            a = int(r['addr'], 16)
            if in_data(a):
                starts.add(a)
    for _, lo, hi in DATA:
        for a in range(lo, hi - 3, 4):
            v = u32(a)
            if in_data(v):
                starts.add(v)
    cls = {}
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'classification.csv'))):
        cls[int(r['address'], 16)] = r['class']
    iat = set()
    exe.pe.parse_data_directories()
    for d in exe.pe.DIRECTORY_ENTRY_IMPORT:
        for imp in d.imports:
            iat.add(imp.address)
    for _, lo, hi in DATA:
        starts.add(lo)
    order = sorted(starts)

    def obj_of(a):
        i = bisect.bisect_right(order, a) - 1
        s = order[i]
        nxt = order[i + 1] if i + 1 < len(order) else s + 4096
        sec_hi = next(hi for _, lo, hi in DATA if lo <= s < hi)
        e = min(table_end.get(s, nxt), s + 4096, sec_hi)
        if a >= e:  # outside a table that ended before the next start: a gap object
            return a & ~3, min(nxt, (a & ~3) + 4)
        return s, e

    def kind(s, e):
        if any(c in iat for c in range(s - (s % 4), e, 4)):
            return 'IAT cells (replaced by the import table)'
        k = table_kind.get(s, '')
        if CRT_INIT[0] <= s < CRT_INIT[1]:
            return 'vtables / function tables (code addresses)'
        if k.startswith('vtable') or k.startswith('fnptr') or k in ('crt-init', 'script-range-fns', 'rw-node-def', 'rw-device'):
            return 'vtables / function tables (code addresses)'
        rd = readers.get(s)
        if rd and all(cls.get(f, 'game').startswith('excluded:') for f in rd):
            return 'referenced only by CRT / third-party / protection code'
        if is_string(s, e):
            return 'strings'
        n = e - s
        dw = [u32(a) for a in range(s, e - 3, 4)]
        if dw and sum(1 for v in dw if v) and all(v == 0 or IMAGE[0] <= v < IMAGE[1] for v in dw):
            return 'pointer tables / pointers (image addresses)'
        if n == 4:
            ex = (u32(s) >> 23) & 0xFF
            if 0 < ex < 255:
                return 'single floats / dwords'
        if n == 8:
            return 'doubles / 8-byte constants'
        return 'other data (tables, structs, initial values)'

    def report(title, objs):
        by = collections.defaultdict(lambda: [0, 0, 0])
        rows = []
        for (s, e) in sorted(objs):
            k = kind(s, e)
            nz = sum(1 for a in range(s, e) if byte(a))
            sec = next(name for name, lo, hi in DATA if lo <= s < hi)
            by[(sec, k)][0] += 1
            by[(sec, k)][1] += e - s
            by[(sec, k)][2] += nz
            rows.append((s, e, sec, k, nz))
        print(f'\n### {title}\n')
        print('| section | kind | objects | bytes | non-zero bytes |')
        print('|---|---|---|---|---|')
        tot = [0, 0, 0]
        for (sec, k), v in sorted(by.items(), key=lambda kv: (kv[0][0], -kv[1][1])):
            print(f'| {sec} | {k} | {v[0]:,} | {v[1]:,} | {v[2]:,} |')
            for i in range(3):
                tot[i] += v[i]
        print(f'| **total** | | {tot[0]:,} | {tot[1]:,} | {tot[2]:,} |')
        carry = sum(v[1] for (sec, k), v in by.items()
                    if not k.startswith('IAT') and not k.startswith('vtables') and not k.startswith('referenced only'))
        print(f'\nTo carry into the code (all kinds but IAT cells, vtables / function tables and CRT / third-party-only '
              f'data): {carry:,} bytes.')
        uniq = [k for k in by if not k[1].startswith(('IAT', 'vtables', 'referenced only', 'pointer tables'))]
        print(f'Unique to the exe (strings, floats, other data; pointer tables, vtables / function tables and IAT '
              f'cells are regenerated): {sum(by[k][1] for k in uniq):,} bytes, {sum(by[k][2] for k in uniq):,} non-zero.')
        return rows

    def with_pointees(objs):
        """Adds the objects the pointer-holding objects point at (strings of a string table, ...), transitively."""
        todo = list(objs)
        while todo:
            s, e = todo.pop()
            for a in range(s, e - 3, 4):
                v = u32(a)
                if in_data(v) and any(byte(x) for x in range(v, min(v + 4, IMAGE[1]))):
                    o = obj_of(v)
                    if o not in objs:
                        objs[o] = 1
                        todo.append(o)
        return objs

    objs = {}
    for _, lo, hi in DATA:
        for a in range(lo, hi):
            if needed(a) and byte(a):
                objs[obj_of(a)] = 1
    # a page left open after the fault budget hides its later first reads: every non-zero object on it counts
    for p in sat_pages:
        if in_data(p):
            for a in range(p, p + 4096):
                if byte(a):
                    objs[obj_of(a)] = 1
    rows = report('Needed data objects, measured (an object holding a byte read first that is non-zero in the image, '
                  'and every non-zero object on a page left open)', objs)
    dyn = dict(objs)

    # static, the whole game: every data object the original's game code (not CRT / third-party / protection) reads
    # or takes the address of, plus what the pointers in those objects reach
    sobjs = {}
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'rwsets.csv'))):
        try:
            a, f = int(r['address'], 16), int(r['function'], 16)
        except ValueError:
            continue
        if not in_data(a) or r['access'] == 'W' or cls.get(f, 'game').startswith('excluded:'):
            continue
        o = obj_of(a)
        if any(byte(x) for x in range(o[0], o[1])):
            sobjs[o] = 1
    if args.census:
        for r in csv.DictReader(open(args.census)):
            a = int(r['addr'], 16)
            if r['test'] == 'False' and in_data(a) and r['context'] in ('raw', 'const', 'other', 'globals', 'orig-cell'):
                o = obj_of(a)
                if any(byte(x) for x in range(o[0], o[1])):
                    sobjs[o] = 1
    with_pointees(sobjs)
    report('Needed data objects, static upper bound (every object the game code of the original reads or takes '
           'the address of, the Rust literals, and what their pointers reach; non-zero objects only)', sobjs)
    both = dict(sobjs)
    both.update(dyn)
    missing = [o for o in dyn if o not in sobjs]
    print(f'\nMeasured objects outside the static set: {len(missing)} '
          f'({sum(e - s for s, e in missing):,} bytes)')
    rows = report('Needed data objects, union of the measured and the static sets', both)
    for i, (s0, e0, sec, k, nz) in enumerate(rows):
        rows[i] = (s0, e0, sec, k, nz, 'measured' if (s0, e0) in dyn else 'static')
    # owning subsystem of each object of the union: the most common catalogue subsystem of the game functions that
    # reference it (procedures.csv), for the per-subsystem step plan
    subsys = {}
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'procedures.csv'))):
        subsys[int(r['address'], 16)] = r['subsystem'] or 'UNK'
    votes = collections.defaultdict(collections.Counter)
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'rwsets.csv'))):
        try:
            a, f = int(r['address'], 16), int(r['function'], 16)
        except ValueError:
            continue
        if in_data(a) and not cls.get(f, 'game').startswith('excluded:'):
            votes[obj_of(a)][subsys.get(f, 'UNK')] += 1
    per = collections.defaultdict(lambda: [0, 0, 0])
    for o in both:
        k = kind(*o)
        if k.startswith(('IAT', 'vtables', 'referenced only', 'pointer tables')):
            continue
        sub = votes[o].most_common(1)[0][0] if votes[o] else '(reached by pointer only)'
        per[sub][0] += 1
        per[sub][1] += o[1] - o[0]
        per[sub][2] += sum(1 for a in range(*o) if byte(a))
    print('\n### Unique data of the union by owning subsystem\n')
    print('| subsystem | objects | bytes | non-zero bytes |')
    print('|---|---|---|---|')
    for sub, v in sorted(per.items(), key=lambda kv: -kv[1][1]):
        print(f'| {sub} | {v[0]:,} | {v[1]:,} | {v[2]:,} |')

    allo = {}
    for _, lo, hi in DATA:
        for a in range(lo, hi):
            if byte(a):
                allo[obj_of(a)] = 1
    report('Every data object with a non-zero byte (the whole initialised image, for comparison)', allo)

    # code bytes read as data
    cdr = collections.defaultdict(list)
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'code-data-reads.csv'))):
        t, n = int(r['target'], 16), int(r['size'])
        for a in range(t, t + n):
            cdr[a].append(r['reader'])
    print('\n### Code bytes read as data\n')
    print('| section | dwords read first | of them known protection reads (code-data-reads.csv) |')
    print('|---|---|---|')
    for name, lo, hi in CODE:
        dws = sorted({a & ~3 for a in range(lo, hi) if needed(a)})
        known = sum(1 for a in dws if any(x in cdr for x in range(a, a + 4)))
        print(f'| {name} | {len(dws)} | {known} |')
    print('\nAddresses: ' + ', '.join(f'{a:#x}' for a in sorted({a & ~3 for name, lo, hi in CODE
                                                                     for a in range(lo, hi) if needed(a)})))
    print(f'\nPages left open after the fault budget: {len(sat_pages)} '
          f'({", ".join(f"{p:#x}" for p in sorted(sat_pages)[:40])}{" ..." if len(sat_pages) > 40 else ""})')
    if args.keep:
        with open(args.keep, 'w') as fh:
            for s0, e0, *_ in rows:
                fh.write(f'{s0:x} {e0:x}\n')
            for c in sorted(iat):
                fh.write(f'{c:x} {c + 4:x}\n')
            for name, lo, hi in CODE:
                for a in sorted({a & ~3 for a in range(lo, hi) if needed(a)}):
                    fh.write(f'{a:x} {a + 4:x}\n')
    if args.objects:
        with open(args.objects, 'w', newline='') as fh:
            w = csv.writer(fh)
            w.writerow(['start', 'end', 'section', 'kind', 'nonzero', 'seen', 'name'])
            for s, e, sec, k, nz, seen in rows:
                w.writerow([f'{s:#x}', f'{e:#x}', sec, k, nz, seen, names.get(s, '')])


if __name__ == '__main__':
    main()
