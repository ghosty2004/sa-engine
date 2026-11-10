#!/usr/bin/env python3
"""Find the instructions that write an address: evidence for tools/e2e/l2-extras.txt lines.

Usage:
  l2-writers.py ROOT index                       build target/l2writers/index.tsv (once)
  l2-writers.py ROOT who FN ADDR[+LEN] ...       writers of each range, those in FN's static call
                                                 closure first
  l2-writers.py ROOT undeclared REPORT [--fn F]  read `sa-oracle l2 split-report … --undeclared` (or
                                                 `l2 report … --undeclared`) lines, coalesce each
                                                 function's ranges and list the writers of each span

The index holds every instruction of every catalogue function body (catalog/extents.csv ranges,
.HOODLUM bodies included) whose memory operand is written (capstone access flags; string stores
excluded: their target is in EDI) and whose displacement lies in the game's writable sections
0x8A4000-0xC9E000: (insn, function, disp, size, base/index, mnemonic text). A direct operand (no
base/index) writes [disp, disp+size); a based or indexed operand writes somewhere at or above disp
(an array element or a field of a static object), so it is listed for spans at most 0x10000 above.
The static call closure is catalog/callgraph.csv (call/tailcall edges); writers reached only through
an unresolved indirect call (function pointers, vtables, RW plugin callbacks) appear in the global
list, marked `outside closure`.
"""
import bisect
import csv
import os
import sys
from collections import defaultdict

LO, HI = 0x8A4000, 0xC9E000


def load_pe(root):
    import pefile

    pe = pefile.PE(os.path.join(root, "re/gta/1.0-US/gta_sa.exe"), fast_load=True)
    base = pe.OPTIONAL_HEADER.ImageBase
    secs = [(base + s.VirtualAddress, s.get_data()) for s in pe.sections]
    return secs


def read(secs, a, n):
    for va, data in secs:
        if va <= a < va + len(data):
            return data[a - va : a - va + n]
    return b""


def build_index(root):
    import capstone
    from capstone import x86

    secs = load_pe(root)
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    md.detail = True
    out = os.path.join(root, "target/l2writers")
    os.makedirs(out, exist_ok=True)
    rows, refs = [], []
    bodies = []
    with open(os.path.join(root, "catalog/extents.csv")) as f:
        for r in csv.DictReader(f):
            fn = int(r["address"], 16)
            for rng in r["ranges"].split(";"):
                if rng:
                    lo, hi = (int(x, 16) for x in rng.split("-"))
                    bodies.append((fn, lo, hi))
    # procedures without a Ghidra extent (the statically linked CRT, mostly): linear sweep up to the
    # next procedure (at most 0x2000 bytes)
    have = {b[0] for b in bodies}
    with open(os.path.join(root, "catalog/procedures.csv")) as f:
        starts = sorted(int(r["address"], 16) for r in csv.DictReader(f))
    for k, a in enumerate(starts):
        if a not in have:
            end = starts[k + 1] if k + 1 < len(starts) else a + 0x100
            bodies.append((a, a, min(end, a + 0x2000) - 1))
    for fn, lo, hi in bodies:
        code = read(secs, lo, hi - lo + 1)
        for ins in md.disasm(code, lo):
            if ins.mnemonic.startswith(("stos", "movs", "rep", "lods", "cmps", "scas")):
                continue
            for op in ins.operands:
                # address taken: `push imm` / `mov reg, imm` / `lea reg, [disp]` (kind 2): the
                # object is written through that pointer by this function or a callee
                if op.type == x86.X86_OP_IMM and LO <= (op.imm & 0xFFFFFFFF) < HI:
                    refs.append((ins.address, fn, op.imm & 0xFFFFFFFF, 0, 2, f"{ins.mnemonic} {ins.op_str}"))
                    continue
                if (ins.mnemonic == "lea" and op.type == x86.X86_OP_MEM and op.mem.base == 0
                        and op.mem.index == 0 and LO <= (op.mem.disp & 0xFFFFFFFF) < HI):
                    refs.append((ins.address, fn, op.mem.disp & 0xFFFFFFFF, 0, 2, f"{ins.mnemonic} {ins.op_str}"))
                    continue
                if op.type != x86.X86_OP_MEM or not (op.access & capstone.CS_AC_WRITE):
                    continue
                d = op.mem.disp & 0xFFFFFFFF
                if not (LO <= d < HI):
                    continue
                based = op.mem.base != 0 or op.mem.index != 0
                rows.append((ins.address, fn, d, op.size, int(based), f"{ins.mnemonic} {ins.op_str}"))
    rows += refs
    rows.sort(key=lambda x: (x[2], x[0]))
    with open(os.path.join(out, "index.tsv"), "w") as f:
        for r in rows:
            f.write(f"{r[0]:#x}\t{r[1]:#x}\t{r[2]:#x}\t{r[3]}\t{r[4]}\t{r[5]}\n")
    print(f"{len(rows)} writing instructions -> {out}/index.tsv")


class Db:
    def __init__(self, root):
        self.root = root
        self.rows = []
        with open(os.path.join(root, "target/l2writers/index.tsv")) as f:
            for line in f:
                i, fn, d, sz, b, txt = line.rstrip("\n").split("\t")
                self.rows.append((int(d, 16), int(sz), int(b), int(i, 16), int(fn, 16), txt))
        self.keys = [r[0] for r in self.rows]
        self.edges = defaultdict(set)
        with open(os.path.join(root, "catalog/callgraph.csv")) as f:
            for r in csv.DictReader(f):
                if r["kind"] in ("call", "tailcall"):
                    self.edges[int(r["caller"], 16)].add(int(r["callee"], 16))
        self.names = {}
        with open(os.path.join(root, "catalog/procedures.csv")) as f:
            for r in csv.DictReader(f):
                self.names[int(r["address"], 16)] = r["name"] or r["gr_name"] or r["ghidra_name"]
        self._cl = {}

    def closure(self, fn):
        """{function: call depth from fn} over the static call graph (BFS)."""
        if fn not in self._cl:
            seen, q = {fn: 0}, [fn]
            for f in q:
                for c in sorted(self.edges.get(f, ())):
                    if c not in seen:
                        seen[c] = seen[f] + 1
                        q.append(c)
            self._cl[fn] = seen
        return self._cl[fn]

    def writers(self, a, n):
        """(row, exact) for instructions that can write [a, a+n)."""
        out = []
        i = bisect.bisect_right(self.keys, a + n - 1)
        j = bisect.bisect_left(self.keys, a - 0x10000)
        for r in self.rows[j:i]:
            d, sz, based = r[0], r[1], r[2]
            if d + sz > a and d < a + n:
                out.append((r, True))
            elif based and d <= a:
                out.append((r, False))
        return out


def fmt_writers(db, fn, a, n, limit=4):
    cl = db.closure(fn)
    ws = db.writers(a, n)
    # exact (direct overlap) before based; in closure before outside; based: nearest disp first
    # exact writers first, then the nearest object base below the span (distance class), then call depth
    ws.sort(key=lambda x: (not x[1] or x[0][2] == 2, max(a - x[0][0], 0).bit_length(), cl.get(x[0][4], 99), a - x[0][0]))

    lines = []
    for (d, sz, based, ins, f, txt), exact in ws[:limit]:
        lines.append(
            f"      {ins:#x} in {f:#x} {db.names.get(f, '?')}: {txt}"
            f"{f'  [depth {cl[f]}]' if f in cl else '  [outside closure]'}{'' if exact else f'  [based, disp {d:#x}]'}"
        )
    if len(ws) > limit:
        lines.append(f"      … {len(ws) - limit} more")
    if not ws:
        lines.append("      no absolute-displacement writer (written through a pointer)")
    return lines


def coalesce(ranges, gap=16):
    out = []
    for a, n in sorted(ranges):
        if out and a <= out[-1][1] + gap:
            out[-1][1] = max(out[-1][1], a + n)
        else:
            out.append([a, a + n])
    return out


def main():
    root, cmd = sys.argv[1], sys.argv[2]
    if cmd == "index":
        build_index(root)
        return
    db = Db(root)
    if cmd == "who":
        fn = int(sys.argv[3], 16)
        for spec in sys.argv[4:]:
            a, _, n = spec.partition("+")
            a, n = int(a, 16), int(n or "1", 0)
            print(f"{a:#x}+{n:#x}")
            print("\n".join(fmt_writers(db, fn, a, n)))
        return
    if cmd == "writes":
        # writes HOOK FN[,FN..] [MAXGAP]: extras lines for every direct (absolute-address) store in the
        # static call closure of FN (an indirect callee the catalogue cannot resolve), merged into ranges
        hook, fns = sys.argv[3], [int(x, 16) for x in sys.argv[4].split(",")]
        gap = int(sys.argv[5], 0) if len(sys.argv) > 5 else 0
        cl = {}
        for fn in fns:
            for f, d in db.closure(fn).items():
                cl[f] = min(d, cl.get(f, 99))
        st = sorted((r for r in db.rows if r[2] == 0 and r[4] in cl), key=lambda r: (r[0], cl[r[4]]))
        rngs = []
        for d, sz, _, ins, f, txt in st:
            if rngs and d <= rngs[-1][1] + gap:
                rngs[-1][1] = max(rngs[-1][1], d + sz)
                rngs[-1][2].append((ins, f, txt))
            else:
                rngs.append([d, d + sz, [(ins, f, txt)]])
        for a, e, w in rngs:
            ins, f, txt = w[0]
            more = f" (+{len(w) - 1} more stores)" if len(w) > 1 else ""
            print(f"{hook}@0x{a:X}:0x{e - a:X}:RW  # {ins:#x} {db.names.get(f, '?')}: {txt}{more}")
        return
    if cmd == "draft":
        # draft REPORT NAME [GAP]: one extras line per coalesced undeclared span of hook NAME (dword
        # aligned), commented with the best writer candidate; to be reviewed before use
        name = sys.argv[4]
        gap = int(sys.argv[5], 0) if len(sys.argv) > 5 else 16
        rs, fa = [], None
        with open(sys.argv[3]) as f:
            for line in f:
                p = line.split()
                if line.startswith("undeclared ") and p[2] == name:
                    fa = int(p[1], 16)
                    a, _, n = p[3].partition("+")
                    rs.append((int(a, 16), int(n)))
        for s0, e0 in coalesce(rs, gap):
            a, e = s0 & ~3, (e0 + 3) & ~3
            w = fmt_writers(db, fa, s0, e0 - s0, limit=1)[0].strip()
            print(f"{name}@0x{a:X}:0x{e - a:X}:RW  # {w}")
        return
    if cmd == "undeclared":
        only = None
        if "--fn" in sys.argv:
            only = {int(x, 16) for x in sys.argv[sys.argv.index("--fn") + 1].split(",")}
        per = defaultdict(list)
        names, syms = {}, {}
        with open(sys.argv[3]) as f:
            for line in f:
                if not line.startswith("undeclared "):
                    continue
                p = line.split()
                fa = int(p[1], 16)
                a, _, n = p[3].partition("+")
                per[fa].append((int(a, 16), int(n)))
                names[fa] = p[2]
                syms[int(a, 16)] = p[5] if len(p) > 5 and not p[5].startswith("runs") else ""
        for fa in sorted(per):
            if only and fa not in only:
                continue
            spans = coalesce(per[fa])
            print(f"{fa:#x} {names[fa]}: {len(spans)} span(s)")
            for s, e in spans:
                print(f"    {s:#x}..{e:#x} (+{e - s:#x}) {syms.get(s, '')}")
                print("\n".join(fmt_writers(db, fa, s, e - s)))
        return
    sys.exit(__doc__)


if __name__ == "__main__":
    main()
