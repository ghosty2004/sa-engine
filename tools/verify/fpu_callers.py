"""Verifier helper (V-12, docs/re/verify-l1-core-stream.md): for each function, its transitive callers
from catalog/callgraph.csv (direct calls, tail calls, function references, resolved virtual and indirect
calls), the direct callers, and the roots (callers nobody calls). Used to decide under which x87 control
word a function can run (PC=24 is forced by RenderWare's _rwProcessorInitialize 0x857404 at
RwEngineInit and _rwProcessorForceSinglePrecision 0x857432 per camera update).

Usage: fpu_callers.py ADDR [ADDR ...] (hex). Prints direct callers and roots with catalogue names."""
import csv
import os
import sys
from collections import defaultdict, deque

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')
KINDS = {'call', 'tailcall', 'fnref', 'virtual', 'virtual-override', 'virtual-typed',
         'virtual-typed-override', 'indirect-ghidra'}

names = {}
for r in csv.DictReader(open(os.path.join(ROOT, 'catalog/procedures.csv'))):
    a = int(r['address'], 16)
    names[a] = r['name'] or r.get('ghidra_name') or ''

callers = defaultdict(set)
for r in csv.DictReader(open(os.path.join(ROOT, 'catalog/callgraph.csv'))):
    if r['kind'] in KINDS:
        callers[int(r['callee'], 16)].add((int(r['caller'], 16), r['kind']))


def name(a):
    return f"{a:#x} {names.get(a, '?')}"


def ancestors(t):
    seen = {t}
    q = deque([t])
    while q:
        x = q.popleft()
        for c, _k in callers.get(x, ()):
            if c not in seen:
                seen.add(c)
                q.append(c)
    return seen


def main():
    for arg in sys.argv[1:]:
        t = int(arg, 16)
        anc = ancestors(t)
        roots = sorted(a for a in anc if a != t and not callers.get(a))
        print(f"== {name(t)}: {len(anc) - 1} transitive callers, {len(roots)} roots")
        for c, k in sorted(callers.get(t, ())):
            print(f"   direct {k}: {name(c)}")
        for r in roots:
            print(f"   root: {name(r)}")


if __name__ == '__main__':
    main()
