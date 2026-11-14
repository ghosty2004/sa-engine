#!/usr/bin/env python3
"""catalog/state.csv against the code: a function is `implemented` / `L1` / `L2` / `L3` only when a port of it
is registered (DEDUP-1, ADR-003 §4).

Usage: python3 -I tools/verify/state_check.py [--list] [--fix]

"Registered" is what tools/verify/frontier.py counts: a `#[hook(addr = ...)]` / `#[native(addr = ...)]`
attribute anywhere under crates/, a row of the table-driven natives (sa-core `std7_natives.rs`,
`static_inits.rs`) or one of sa-game's address entries (crt.rs, rw_standin.rs, d3d9_shader.rs, com.rs,
vorbisfile.rs). A port that cannot be registered (an unhookable entry, a pure function verified at L1 on its
Rust body) counts when a Rust function's doc comment names its address (`/// `Name` (0xADDR)` right above
the `fn` with a body; an `orig!` declaration does not count). Rows above `specified` with neither are
inconsistent: the state claims a port that does not exist (e.g. a port that was planned but not written).

`--fix` lowers every inconsistent row to `specified` with tools/catalog/progress.py's evidence column
("state_check: no registered port"), which `run_all.sh` keeps. Exit 1 when inconsistent rows remain.
"""
import csv
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import frontier as F  # noqa: E402

ROOT = F.ROOT
STATE = os.path.join(ROOT, 'catalog', 'state.csv')
CLAIMS = ('implemented', 'L1', 'L2', 'L3')


def registered():
    reg = set()
    for rel, text in F.rust_files():
        reg |= {F.h(a) for a in F.ATTR.findall(text)}
        if rel in F.TABLE_FILES or rel in F.GAME_FILES:
            reg |= {F.h(a) for a in F.ROW.findall(text)}
    return reg


DOC_FN = re.compile(r'((?:^[ \t]*///[^\n]*\n)+)(?:^[ \t]*(?:#\[|//)[^\n]*\n)*^[ \t]*(?:pub(?:\([^)]*\))?\s+)?'
                    r'(?:const\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?fn\s+\w+([^{}]*?)\{', re.M)
ADDR = re.compile(r'\b0x([0-9A-Fa-f]{6,8})\b')


def documented_ports():
    out = set()
    for rel, text in F.rust_files():
        for m in DOC_FN.finditer(text):
            if '=>' in m.group(2):  # an `orig!` declaration, not a port
                continue
            out |= {int(a, 16) for a in ADDR.findall(m.group(1))}
    return out


def main():
    reg = registered() | documented_ports()
    rows = list(csv.DictReader(open(STATE, newline='')))
    bad = [r for r in rows if r['state'] in CLAIMS and int(r['address'], 16) not in reg]
    if '--fix' in sys.argv and bad:
        fields = list(rows[0].keys())
        ids = {r['address'] for r in bad}
        for r in rows:
            if r['address'] in ids:
                r['evidence'] = 'state_check: no Rust port (was %s); %s' % (r['state'], r['evidence'])
                r['state'] = 'specified'
                r['updated'] = '2026-11-14'
        with open(STATE, 'w', newline='') as fh:
            w = csv.DictWriter(fh, fields, lineterminator='\n')
            w.writeheader()
            w.writerows(rows)
        print(f'state_check: lowered {len(bad)} rows to specified')
        return 0
    print(f'state_check: {sum(r["state"] in CLAIMS for r in rows)} rows claim a port, {len(reg)} addresses '
          f'registered or documented as Rust ports, {len(bad)} inconsistent')
    if '--list' in sys.argv or len(bad) <= 40:
        for r in bad:
            print(f'FAIL {r["address"]} {r["name"]} {r["state"]}: no #[hook]/#[native] registers it ({r["evidence"][:80]})')
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
