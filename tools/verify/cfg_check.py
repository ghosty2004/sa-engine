#!/usr/bin/env python3
"""Platform `cfg`s only in backends and the binding layer (ADR-003 §3.1, §4).

Usage: python3 -I tools/verify/cfg_check.py [--write-baseline]

Counts the platform conditions (`target_arch`, `target_os`, `target_family`, `target_env`,
`target_pointer_width`, `windows`, `unix`, `sa_phase_a`) in `cfg(..)` / `cfg_attr(..)` / `cfg!(..)` of every
`*.rs` under crates/ (tests, benches and examples included). Files of the backends and the binding layer
(BACKENDS below) may have any; every other file may have at most the count `tools/verify/cfg-baseline.txt`
records for it (the ones not yet moved behind a backend; the list may only shrink). Exit 1 when a file
exceeds its baseline or a new file has platform cfgs. `--write-baseline` rewrites the baseline from the tree.
"""
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
BASELINE = os.path.join(ROOT, 'tools', 'verify', 'cfg-baseline.txt')
COND = re.compile(r'\b(?:target_arch|target_os|target_family|target_env|target_pointer_width|windows|unix|sa_phase_a)\b')
CFG = re.compile(r'\bcfg(?:_attr)?!?\s*\(')
# The backends (one per platform) and the binding layer: the memory-access layer and the `orig!` / hook /
# register-convention machinery in sa-core, the in-process DLL and its launcher, the platform crates, the native
# and web front-end backends of sa-game and its platform adapters (D3D9 / DirectSound / COM / Win32 stand-ins).
BACKENDS = [
    'crates/sa-core/src/orig.rs', 'crates/sa-core/src/mem.rs', 'crates/sa-core/src/abi.rs',
    'crates/sa-core/src/regconv.rs', 'crates/sa-core/src/regconv_tests.rs', 'crates/sa-core/src/wabi.rs',
    'crates/sa-core/src/hooks.rs', 'crates/sa-macros/', 'crates/sa-hook/', 'crates/sa-launcher/',
    'crates/sa-platform/', 'crates/sa-net/', 'crates/sa-rw-wgpu/', 'crates/sa-udp-proxy/', 'crates/sa-oracle/',
    'crates/sa-oracle-format/', 'crates/phase_a_build.rs',
    'crates/sa-game/src/game_window.rs', 'crates/sa-game/src/web.rs', 'crates/sa-game/src/webfs.rs',
    'crates/sa-game/src/platform/',
]


def blank_strings_comments(src):
    out, i, n = [], 0, len(src)
    while i < n:
        if src.startswith('//', i):
            j = src.find('\n', i)
            j = n if j < 0 else j
            out.append(' ' * (j - i))
            i = j
        elif src.startswith('/*', i):
            j = src.find('*/', i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r'[^\n]', ' ', src[i:j]))
            i = j
        elif src[i] == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == '\\' else 1
            out.append(' ' * (j + 1 - i))
            i = j + 1
        else:
            out.append(src[i])
            i += 1
    return ''.join(out)


def count(path):
    s = blank_strings_comments(open(path, encoding='utf-8').read())
    n = 0
    for m in CFG.finditer(s):
        d, j = 0, m.end() - 1
        while j < len(s):
            d += {'(': 1, ')': -1}.get(s[j], 0)
            if d == 0:
                break
            j += 1
        n += len(COND.findall(s[m.end():j]))
    return n


def scan():
    out = {}
    for d, dirs, fs in os.walk(os.path.join(ROOT, 'crates')):
        dirs[:] = sorted(x for x in dirs if x != 'target')
        for f in sorted(fs):
            if f.endswith('.rs'):
                p = os.path.join(d, f)
                rel = os.path.relpath(p, ROOT)
                if any(rel == b or (b.endswith('/') and rel.startswith(b)) for b in BACKENDS):
                    continue
                n = count(p)
                if n:
                    out[rel] = n
    return out


def main():
    now = scan()
    if '--write-baseline' in sys.argv:
        with open(BASELINE, 'w') as fh:
            fh.write('# Platform cfg conditions per file outside the backends (tools/verify/cfg_check.py); may only shrink.\n')
            for rel, n in sorted(now.items()):
                fh.write(f'{n}\t{rel}\n')
        print(f'cfg: baseline written, {len(now)} files, {sum(now.values())} conditions')
        return 0
    base = {}
    for line in open(BASELINE):
        if line.strip() and not line.startswith('#'):
            n, rel = line.rstrip('\n').split('\t')
            base[rel] = int(n)
    bad = [f'{rel}: {n} platform cfg conditions, baseline {base.get(rel, 0)} (move them behind a backend, ADR-003 §3.1)'
           for rel, n in sorted(now.items()) if n > base.get(rel, 0)]
    print(f'cfg: {sum(now.values())} platform cfg conditions in {len(now)} files outside the backends '
          f'(baseline {sum(base.values())} in {len(base)}); {len(bad)} over')
    for b in bad:
        print('FAIL', b)
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
