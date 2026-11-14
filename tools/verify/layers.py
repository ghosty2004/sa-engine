#!/usr/bin/env python3
"""Crate dependency order (ADR-003 §3.4): every crate under crates/ depends only on crates of a lower layer
(`tools/lib/layers.py`), and every crate has a layer.

Usage: python3 -I tools/verify/layers.py      (exit 1 on a violation; the writer gate's `layers` step)
"""
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
sys.path.insert(0, os.path.join(ROOT, 'tools', 'lib'))
import layers  # noqa: E402


def deps(crate):
    """Normal and build dependencies on workspace crates (dev-dependencies may reach any crate)."""
    t = open(os.path.join(ROOT, 'crates', crate, 'Cargo.toml')).read()
    out = set()
    for sec in re.split(r'\n(?=\[)', t):
        head = sec.split('\n', 1)[0]
        if head.endswith('dependencies]') and 'dev-' not in head:
            out |= set(re.findall(r'^(sa-[a-z0-9-]+)\s*=', sec, re.M))
    return out


def main():
    bad = []
    crates = sorted(c for c in os.listdir(os.path.join(ROOT, 'crates'))
                    if os.path.exists(os.path.join(ROOT, 'crates', c, 'Cargo.toml')))
    for c in crates:
        if layers.rank(c) is None:
            bad.append(f'{c}: no layer in tools/lib/layers.py')
            continue
        for d in sorted(deps(c)):
            if not layers.may_depend(c, d):
                bad.append(f'{c} (layer {layers.rank(c)}) depends on {d} (layer {layers.rank(d)})')
    print(f'layers: {len(crates)} crates, {len(bad)} violations')
    for b in bad:
        print('FAIL', b)
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
