"""Merge the UNDECLARED rows of sa-hook's `callers` test into `tools/gen/patterns_callers.txt`.

Usage: RUST_MIN_STACK=268435456 cargo test -p sa-hook --lib callers_reading 2>&1 \
           | python3 -I tools/gen/patterns_callers.py

Every UNDECLARED hook must be a generated one (`target/gen-patterns/report.tsv`); its registers are added to
the file (existing rows are kept), and `tools/gen/patterns.py` then declares `preserve` or leaves the
function unhooked (when the original changes the register).
"""
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
PATH = os.path.join(ROOT, 'tools', 'gen', 'patterns_callers.txt')
HEAD = ('# tools/gen/patterns.py: generated hooks whose original callers read ECX/EDX after the call (sa-hook lib test\n'
        '# `callers_reading_ecx_edx_after_a_hook_are_declared`, UNDECLARED rows; tools/gen/patterns_callers.py).\n'
        '# patterns.py declares `preserve` when the original leaves the registers unchanged on every path,\n'
        '# and does not hook the function otherwise.\n')


def main():
    gen = {int(l.split('\t')[0], 16) for l in open(os.path.join(ROOT, 'target', 'gen-patterns', 'report.tsv'))
           if l.startswith('0x')}
    d = {}
    if os.path.exists(PATH):
        for l in open(PATH):
            l = l.split('#', 1)[0].strip()
            if l:
                a, regs = l.split()
                d[int(a, 16)] = set(regs.split(','))
    bad = []
    for l in sys.stdin:
        m = re.search(r'UNDECLARED: (0x[0-9a-f]+) <- \S+ (Ecx|Edx) read', l)
        if not m:
            continue
        a = int(m.group(1), 16)
        if a not in gen:
            bad.append(a)
            continue
        d.setdefault(a, set()).add(m.group(2).lower())
    with open(PATH, 'w') as f:
        f.write(HEAD)
        for a in sorted(d):
            f.write('0x%X %s\n' % (a, ','.join(sorted(d[a]))))
    if bad:
        print('UNDECLARED hooks that are not generated:', ', '.join('%#x' % a for a in sorted(set(bad))))
        sys.exit(1)


if __name__ == '__main__':
    main()
