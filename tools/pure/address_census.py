#!/usr/bin/env python3
"""Census of the original 1.0 US addresses written into the Rust sources (PURE-1, ADR-005).

Every hex literal in `crates/**/*.rs` (comments and string literals stripped) whose value lies in the
original image (0x401000..0x1577000) is classified by

- section: code (`.text` / `_rwcseg`), `.rdata` (split into IAT cells, vtables from
  `catalog/data-tables.csv`, other constants), `.data` (initialised), `.bss`, `_TEXT_HA`/`_rwdseg`/`.rsrc`,
  the no-CD image copy, `.HOODLUM`;
- context: `hook` (`#[hook]` / `#[native]` address), `orig` (an `orig!` callee address), `orig-cell`
  (`cell` / `table` forms: a data cell holding a function pointer), `at-site` (caller attribution),
  `globals` (a `globals!` declaration), `raw` (an argument of an address helper: `g`, `gf`, `p8`,
  `GPtr::from_addr`, ...), `const` (a named constant), `other` (comparisons, tables, pokes, ...);
- crate, and whether the file is test / tooling code (crates that only exist for phase A or the oracle).

Plain Python with `pefile` for the section table and the import table (`.venv/bin/python -I`).
Prints Markdown tables; `--csv OUT` also writes one row per occurrence.
"""
import argparse
import collections
import csv
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
sys.path.insert(0, os.path.join(ROOT, 'tools', 'lib'))

TOOLING = {'sa-hook', 'sa-oracle', 'sa-oracle-format', 'sa-launcher', 'sa-testkit', 'sa-udp-proxy', 'sa-macros'}

SECTIONS = [
    ('code', 0x401000, 0x858000),
    ('.rdata', 0x858000, 0x8A4000),
    ('.data', 0x8A4000, 0x8E4000),
    ('.bss', 0x8E4000, 0xC9E000),
    ('_TEXT_HA/_rwdseg/.rsrc', 0xC9E000, 0xCB1000),
    ('nocd-image', 0xCB1000, 0x1556000),
    ('.HOODLUM', 0x1556000, 0x1577000),
]
HELPERS = {
    'g', 'gu', 'pa', 'p8', 'gf', 'gfu', 'gx', 'gr32', 'gr32u', 'gr8', 'grp', 'gw32', 'gw32u', 'gr16u', 'gw16u',
    'gr8u', 'gw8u', 'g32', 'rdata', 'from_addr', 'gp', 'gb', 'gw', 'gv', 'addr', 'at', 'p', 'gptr',
}
HEX = re.compile(r'0x[0-9A-Fa-f_]+')
STR = re.compile(r'b?"(?:\\.|[^"\\])*"')


def section(a):
    for name, lo, hi in SECTIONS:
        if lo <= a < hi:
            return name
    return None


def load_tables():
    vt, iat = set(), set()
    p = os.path.join(ROOT, 'catalog', 'data-tables.csv')
    for r in csv.DictReader(open(p)):
        if r['kind'].startswith('vtable'):
            vt.add(int(r['start'], 16))
    try:
        import exe
        pe = exe.pe
        pe.parse_data_directories()
        for d in pe.DIRECTORY_ENTRY_IMPORT:
            for imp in d.imports:
                iat.add(imp.address)
    except Exception as e:  # no exe: IAT cells stay "other constants"
        print(f'note: no import table ({e})', file=sys.stderr)
    return vt, iat


def subkind(a, sec, vt, iat):
    if sec == '.rdata':
        if a in iat:
            return '.rdata IAT'
        if a in vt:
            return '.rdata vtable'
        return '.rdata const'
    return sec


def strip(line, in_block):
    """Code part of a line (no comments, no string literals); tracks /* */ blocks."""
    out = ''
    i = 0
    while i < len(line):
        if in_block:
            j = line.find('*/', i)
            if j < 0:
                return out, True
            i, in_block = j + 2, False
            continue
        if line.startswith('//', i):
            break
        if line.startswith('/*', i):
            in_block = True
            i += 2
            continue
        out += line[i]
        i += 1
    return STR.sub('""', out), in_block


GEN_CHECK = re.compile(r'^\s*\(0x[0-9A-Fa-f_]+,\s*0x[0-9A-Fa-f]{16},\s*&\[')


def context(code, pos, in_globals):
    before = code[:pos]
    if GEN_CHECK.match(code):
        return 'gen-check'
    if re.search(r'#\[(hook|native)\b', code) or re.search(r'\baddr\s*=\s*$', before):
        return 'hook'
    if re.search(r'\bat_site\s*\(\s*$', before):
        return 'at-site'
    if in_globals and re.search(r'@\s*$', before):
        return 'globals'
    if re.search(r'\b(cell|table)\s+$', before):
        return 'orig-cell'
    if re.search(r'=\s*(cdecl|stdcall|thiscall|fastcall|reg\(""\))\s+((site|callsite)\s+)?$', before):
        return 'orig'
    m = re.search(r'([A-Za-z_][A-Za-z0-9_]*)\s*(::<[^()]*>)?\s*\(\s*(\d+\s*\+\s*)?$', before)
    if m and m.group(1) in HELPERS:
        return 'raw'
    if re.search(r'\bconst\s+[A-Z0-9_]+\s*:\s*[^=]+=\s*(\[)?\s*$', before) or re.search(r'\bconst\b', before):
        return 'const'
    return 'other'


def scan(vt, iat):
    rows = []
    for dirpath, dirs, files in os.walk(os.path.join(ROOT, 'crates')):
        dirs[:] = [d for d in dirs if d != 'target']
        for f in sorted(files):
            if not f.endswith('.rs'):
                continue
            path = os.path.join(dirpath, f)
            rel = os.path.relpath(path, ROOT)
            crate = rel.split(os.sep)[1]
            test_file = f'{os.sep}tests{os.sep}' in rel or f.endswith('_tests.rs') or f == 'tests.rs'
            in_block, in_test, in_globals, depth_g = False, test_file, False, 0
            for n, line in enumerate(open(path, encoding='utf-8', errors='replace'), 1):
                code, in_block = strip(line, in_block)
                if re.match(r'\s*#\[cfg\(.*\btest\b', code) and 'not(test' not in code:
                    in_test = True
                if re.search(r'\bglobals!\s*\{', code):
                    in_globals, depth_g = True, 0
                if in_globals:
                    depth_g += code.count('{') - code.count('}')
                for m in HEX.finditer(code):
                    try:
                        a = int(m.group(0).replace('_', ''), 16)
                    except ValueError:
                        continue
                    sec = section(a)
                    if sec is None:
                        continue
                    rows.append({
                        'crate': crate, 'file': rel, 'line': n, 'addr': a,
                        'section': subkind(a, sec, vt, iat),
                        'context': context(code, m.start(), in_globals),
                        'test': in_test or crate in TOOLING,
                    })
                if in_globals and depth_g <= 0 and '}' in code:
                    in_globals = False
    return rows


def table(title, rows, key_row, key_col, cols=None):
    cnt = collections.Counter((key_row(r), key_col(r)) for r in rows)
    rk = sorted({k[0] for k in cnt}, key=lambda k: -sum(v for (a, _), v in cnt.items() if a == k))
    ck = cols or sorted({k[1] for k in cnt})
    out = [f'### {title}', '', '| | ' + ' | '.join(ck) + ' | total |', '|---' * (len(ck) + 2) + '|']
    for k in rk:
        vals = [cnt.get((k, c), 0) for c in ck]
        out.append(f'| {k} | ' + ' | '.join(f'{v:,}' for v in vals) + f' | {sum(vals):,} |')
    tot = [sum(cnt.get((k, c), 0) for k in rk) for c in ck]
    out.append('| **total** | ' + ' | '.join(f'{v:,}' for v in tot) + f' | {sum(tot):,} |')
    return '\n'.join(out) + '\n'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--csv')
    args = ap.parse_args()
    vt, iat = load_tables()
    rows = scan(vt, iat)
    game = [r for r in rows if not r['test']]
    secs = ['code', '.rdata vtable', '.rdata IAT', '.rdata const', '.data', '.bss', '_TEXT_HA/_rwdseg/.rsrc',
            'nocd-image', '.HOODLUM']
    ctxs = ['hook', 'gen-check', 'orig', 'orig-cell', 'at-site', 'globals', 'raw', 'const', 'other']
    print(f'Occurrences: {len(rows):,} ({len(game):,} in game code, {len(rows) - len(game):,} in tests / tooling crates)\n')
    print(table('Game code: context x section (occurrences)', game, lambda r: r['context'], lambda r: r['section'], secs))
    print(table('Game code: crate x section (occurrences)', game, lambda r: r['crate'], lambda r: r['section'], secs))
    print(table('Game code: crate x context (occurrences)', game, lambda r: r['crate'], lambda r: r['context'], ctxs))
    dist = {}
    for r in game:
        dist.setdefault(r['section'], set()).add(r['addr'])
    print('### Distinct addresses in game code by section\n')
    print('| section | distinct addresses |\n|---|---|')
    for s in secs:
        print(f'| {s} | {len(dist.get(s, ())):,} |')
    data_ctx = [r for r in game if r['context'] not in ('hook', 'orig', 'at-site', 'gen-check')]
    d2 = {}
    for r in data_ctx:
        d2.setdefault(r['section'], set()).add(r['addr'])
    print('\n### Distinct addresses used as data (not hook / gen-check / orig / at-site)\n')
    print('| section | distinct addresses |\n|---|---|')
    for s in secs:
        print(f'| {s} | {len(d2.get(s, ())):,} |')
    if args.csv:
        with open(args.csv, 'w', newline='') as fh:
            w = csv.DictWriter(fh, fieldnames=['crate', 'file', 'line', 'addr', 'section', 'context', 'test'])
            w.writeheader()
            for r in rows:
                w.writerow({**r, 'addr': f'{r["addr"]:#x}'})


if __name__ == '__main__':
    main()
