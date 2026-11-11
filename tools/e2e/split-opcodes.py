#!/usr/bin/env python3
"""Per-opcode case counts of the script switch hooks in an L2 / split-run spool (PORT-SCRIPT-2).

    split-opcodes.py SA_ORACLE SPOOL [--ported]

Runs `sa-oracle l2 show SPOOL heads` and counts, per `CRunningScript::ProcessCommandsXToY` hook, the cases by
their first stack argument (the opcode). With --ported, marks the opcodes that have a Rust case in
crates/sa-script (the `PORTED` lists, read from the sources next to this script). A split run whose
split-report verdict for a switch is PASS compared every one of these cases (up to its per-hook limit),
so the listed ported opcodes are the per-case evidence of a partial switch (ADR-002).
"""
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def ported():
    out = {}
    for f in (ROOT / "crates/sa-script/src").glob("cmd*.rs"):
        m = re.search(r"pub const PORTED: &\[u16\] = &\[(.*?)\];", f.read_text(), re.S)
        if m:
            out[f.stem] = {int(x, 16) for x in re.findall(r"0x[0-9A-Fa-f]+", m.group(1))}
    return out


def main():
    if len(sys.argv) < 3:
        print(__doc__, file=sys.stderr)
        sys.exit(2)
    oracle, spool = sys.argv[1], sys.argv[2]
    want_ported = "--ported" in sys.argv[3:]
    head = re.compile(r"^(CRunningScript::ProcessCommands(\d+)To\d+) frame \d+ .*?args \[0x([0-9a-f]+)")
    counts = defaultdict(Counter)
    p = subprocess.Popen([oracle, "l2", "show", spool, "heads"], stdout=subprocess.PIPE, text=True, errors="replace")
    for line in p.stdout:
        if not line.startswith("CRunningScript::ProcessCommands"):
            continue
        m = head.match(line)
        if m:
            counts[(m.group(1), int(m.group(2)))][int(m.group(3), 16)] += 1
    p.wait()
    pt = ported() if want_ported else {}
    for (name, lo), c in sorted(counts.items(), key=lambda k: k[0][1]):
        mine = pt.get(f"cmd{lo}", set())
        ops = " ".join(f"{op:04X}{'*' if op in mine else ''}x{n}" for op, n in sorted(c.items()))
        print(f"{name}\tcases={sum(c.values())}\t{ops}")
    if want_ported:
        print("(* = opcode with a Rust case)")


if __name__ == "__main__":
    main()
