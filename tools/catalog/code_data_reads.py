#!/usr/bin/env python3
"""INF-20: instructions that read the first bytes of a function entry as data.

    .venv/bin/python -I tools/catalog/code_data_reads.py [--exe gta_sa.exe] [--out catalog/code-data-reads.csv]

The no-CD protection decodes pointers and keys from game code bytes, e.g. at 0x4017AE:
`pushfd; stc; mov eax,[0x156067C]; sbb eax,[0x55C410]; mov eax,[eax]; popfd`. A hook that rewrites
the entry of 0x55C410 changes the decoded pointer. sa-hook's install test
(`table::tests::no_hook_patches_bytes_read_as_data`) intersects every hook's patched bytes with this
table.

Scan: every executable section of the image (game `.text`, `_rwcseg`, the no-CD `.text` at 0xCB1000,
`.init`, the executable `.data` at 0x1301000 and `.HOODLUM`). At every offset whose dword lies in the
first WINDOW bytes of a catalogued function entry (`catalog/procedures.csv`), each instruction that
decodes from up to 10 bytes before and contains that dword as its displacement or immediate is a
candidate:

- `mem`: a memory operand (read, write or `lea`) whose displacement is the dword; `abs` when it has no
  base and no index register (the only form the protection uses), `idx` otherwise.
- `imm`: an immediate that points *inside* an entry (offset > 0). An immediate equal to the entry itself
  is a function pointer (callbacks, `atexit`, vtable setup) and is not listed; relative branch targets
  are not operands of this kind and are never listed.

Each candidate gets two plausibility flags, because a decode that starts in the middle of another
instruction is meaningless:

- `chunk`: decodes in line from a `pushfd`/`stc`/`clc` at most 24 bytes before (the shape of every
  protection chunk; criterion from INF-10's tools/e2e/cov-data-reads.py).
- `sweep`: reached by a linear decode from the closest catalogued function start at or before it in
  game `.text`, or from the start of a 4 KiB-aligned block elsewhere (the decoder resynchronises within a
  few instructions on x86).

Output columns: reader,section,len,insn,kind,disp_off,target,size,entry,entry_off,chunk,sweep.
`disp_off` is the offset of the 32-bit field inside the instruction (for a redirect of `abs` reads).
Deterministic: two runs give identical output.
"""
import bisect
import csv
import struct
import sys
from pathlib import Path

import pefile
from capstone import CS_ARCH_X86, CS_MODE_32, Cs
from capstone.x86 import X86_OP_IMM, X86_OP_MEM, X86_REG_INVALID

ROOT = Path(__file__).resolve().parents[2]
LO, HI = 0x401000, 0x857000
WINDOW = 24
BRANCH_GROUPS = {1, 2, 3, 7}  # jump, call, ret, branch_relative


def main():
    args = sys.argv[1:]
    opt = lambda k, d=None: args[args.index(k) + 1] if k in args else d
    exe = opt("--exe", str(ROOT / "re/gta/1.0-US/gta_sa.exe"))
    out = Path(opt("--out", str(ROOT / "catalog/code-data-reads.csv")))
    pe = pefile.PE(exe, fast_load=True)
    base = pe.OPTIONAL_HEADER.ImageBase
    img = pe.get_memory_mapped_image()
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    md.detail = True
    mdq = Cs(CS_ARCH_X86, CS_MODE_32)
    byte = lambda a: img[a - base]

    entries = []
    with open(ROOT / "catalog/procedures.csv") as f:
        for row in csv.DictReader(f):
            a = int(row["address"], 16)
            if LO <= a < HI:
                entries.append(a)
    entries.sort()
    window = {}
    for i, e in enumerate(entries):
        nxt = entries[i + 1] if i + 1 < len(entries) else HI
        for o in range(min(WINDOW, nxt - e)):
            window[e + o] = (e, o)

    def in_line(s, a):
        for i in mdq.disasm(img[s - base:a - base + 16], s):
            if i.address >= a:
                return i.address == a
        return False

    def chunk(a):
        for k in range(1, 24):
            if byte(a - k) in (0x9C, 0xF8, 0xF9) and in_line(a - k, a):
                return True
        return False

    def sweep(a, sec_va):
        if LO <= a < HI:
            k = bisect.bisect_right(entries, a) - 1
            s = entries[k] if k >= 0 else max(sec_va, a & ~0xFFF)
        else:
            s = max(sec_va, a & ~0xFFF)
        pos = s
        while pos <= a:
            i = next(mdq.disasm(img[pos - base:pos - base + 16], pos, 1), None)
            if i is None:
                pos += 1  # undecodable byte (data in code): step over it
                continue
            if pos == a:
                return True
            pos += i.size
        return False

    rows = set()
    for s in pe.sections:
        if not s.Characteristics & 0x20000000:
            continue
        name = s.Name.rstrip(b"\0").decode()
        va = base + s.VirtualAddress
        end = va + s.Misc_VirtualSize
        sec = f"{name}@{va:#x}"
        for j in range(va + 1, end - 3):
            d = struct.unpack_from("<I", img, j - base)[0]
            hit = window.get(d)
            if hit is None:
                continue
            e, eo = hit
            for k in range(1, 11):
                st = j - k
                if st < va:
                    break
                ins = next(md.disasm(img[st - base:st - base + 16], st, 1), None)
                if ins is None or ins.size < k + 4 or ins.mnemonic == "nop":
                    continue
                kinds = []
                for op in ins.operands:
                    if op.type == X86_OP_MEM and ins.disp_offset == k and op.mem.disp & 0xFFFFFFFF == d:
                        m = op.mem
                        absolute = m.base == X86_REG_INVALID and m.index == X86_REG_INVALID
                        kinds.append(("mem-abs" if absolute else "mem-idx", op.size))
                    elif (op.type == X86_OP_IMM and ins.imm_offset == k and op.imm & 0xFFFFFFFF == d
                          and eo > 0 and not set(ins.groups) & BRANCH_GROUPS):
                        kinds.append(("imm", 4))
                if ins.mnemonic == "lea":
                    kinds = [("lea", 4)] if kinds else []
                for kind, size in kinds:
                    rows.add((st, sec, ins.size, f"{ins.mnemonic} {ins.op_str}", kind, k, d, size, e, eo,
                              int(chunk(st)), int(sweep(st, va))))
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "w", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["reader", "section", "len", "insn", "kind", "disp_off", "target", "size", "entry",
                    "entry_off", "chunk", "sweep"])
        for r in sorted(rows):
            w.writerow([f"{r[0]:#x}", r[1], r[2], r[3], r[4], r[5], f"{r[6]:#x}", r[7], f"{r[8]:#x}", r[9],
                        r[10], r[11]])
    n = len(rows)
    print(f"{out}: {n} candidates, {sum(r[10] for r in rows)} in protection chunks, "
          f"{sum(r[11] for r in rows)} on a linear sweep")
    return 0


if __name__ == "__main__":
    sys.exit(main())
