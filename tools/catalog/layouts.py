#!/usr/bin/env python3
"""CAT-8: extract the struct size/offset assertions of gta-reversed into
catalog/layouts.csv (kind, type, member, value, file, line).

These are layout facts of the 1.0 US binary that Rust #[repr(C)] types must
match (checked later by generated const assertions).
Usage: layouts.py <gta-reversed> <catalog-dir>
"""
import csv
import re
import sys
from pathlib import Path

SIZE = re.compile(r'^\s*VALIDATE_SIZE\(\s*(.+?)\s*,\s*([^;]+?)\s*\)\s*;')
OFF = re.compile(r'^\s*VALIDATE_OFFSET\(\s*(.+?)\s*,\s*(\w+)\s*,\s*([^;]+?)\s*\)\s*;')


def value(expr):
    expr = expr.strip()
    try:
        return int(expr, 0)
    except ValueError:
        return expr  # symbolic (e.g. sizeof(int*), a constant): kept verbatim


def main():
    root, out = Path(sys.argv[1]), Path(sys.argv[2])
    rows = []
    for p in sorted((root / 'source').rglob('*')):
        if p.suffix not in ('.h', '.hpp', '.cpp'):
            continue
        for ln, line in enumerate(p.read_text(errors='replace').splitlines(), 1):
            if (m := SIZE.match(line)):
                rows.append(('size', m.group(1), '', value(m.group(2)), str(p.relative_to(root)), ln))
            elif (m := OFF.match(line)):
                rows.append(('offset', m.group(1), m.group(2), value(m.group(3)), str(p.relative_to(root)), ln))
    with open(out / 'layouts.csv', 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['kind', 'type', 'member', 'value', 'file', 'line'])
        for r in rows:
            v = r[3]
            w.writerow([r[0], r[1], r[2], f'{v:#x}' if isinstance(v, int) else v, r[4], r[5]])
    sym = sum(1 for r in rows if not isinstance(r[3], int))
    print(f"sizes={sum(r[0]=='size' for r in rows)} offsets={sum(r[0]=='offset' for r in rows)} symbolic={sym}")


if __name__ == '__main__':
    main()
