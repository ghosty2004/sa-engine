#!/usr/bin/env python3
"""SAMP-1D: drop the coverage points that samp.dll 0.3.7-R4 writes, reads or tests.

    samp-points.py POINTS PATCHES_CSV SAMP_DLL OUT_POINTS OUT_EXCLUDED

POINTS is `sa-oracle coverage points` output (`ADDR BYTE KIND FUNC`). A point is dropped when its
address lies in a byte range samp.dll touches in game code, because an int3 there would either be
overwritten by samp (harmless, but the point then measures nothing), or change what samp sees:
* every row of samp-rs `docs/re/data/gta-patches-r4-annotated.csv` (patch sites, hooks, NOP runs,
  temporary patch/restore pairs, the 20-byte auth read at 0x4D3AA0, the three sites samp waits on
  until they hold an expected byte), `[addr, addr + size)`; rows listing several addresses are split;
* the table-driven patches (samp-rs `docs/re/subsystems/gta-patches.md` 4.2-4.4) read from samp.dll's
  .data: the 707 model-info pointer sites {u32 addr; u8 expected opcode} at 0x10114B10 (samp patches a
  site only when the opcode byte matches, so an int3 there would make it skip the site), the sector
  grid references (14 absolute pointers at 0x10116758, 56 + 11 imm32 operands at 0x101167C8 and
  0x10116988) and the 14 atomic-model-store references at 0x10116720. For each site the whole
  instruction is covered: [site, site + 8).
OUT_EXCLUDED lists `address,function,reason`.
"""
import csv
import re
import struct
import sys

import pefile

SAMP_TABLES = [
    # (name, va, count, record size)
    ("model-info pointer site (707 table)", 0x10114B10, 707, 5),
    ("sector grid absolute pointer", 0x10116758, 14, 4),
    ("sector grid imm32 operand", 0x101167C8, 56, 4),
    ("sector grid imm32 operand (+4)", 0x10116988, 11, 4),
    ("atomic model store reference", 0x10116720, 14, 4),
]
# Sites patched by samp code outside the CSV rows' own size (gta-patches.md 4.2 / 4.3).
EXTRA = [(0x40936A, 8, "sector grid operand (inline)"), (0x564DC7, 8, "sector grid operand (inline)"),
         (0x4C67AD, 8, "ped model store reference"), (0x5634A6, 8, "ped model store reference"),
         (0x5638DF, 8, "ped model store reference"), (0x56420F, 8, "ped model store reference"),
         (0x564283, 8, "ped model store reference")]


def csv_ranges(path):
    out = []
    for r in csv.DictReader(open(path, newline="")):
        size_s = r["size"].strip()
        m = re.match(r"(\d+)", size_s)
        if not m:
            continue
        size = int(m.group(1))
        kind = r["kind"].strip()
        for a in r["gta_addr"].split(","):
            # Dead-code rows carry prose ("0x411458 + 7 sites ..."): their leading address still counts.
            m = re.match(r"\s*(0x[0-9a-fA-F]+)", a)
            if not m:
                continue
            addr = int(m.group(1), 16)
            # 0x5543C0: "NOP 22 @0x5543C0 + NOP 2 @0x5543DC" spans 30 bytes; size column already 30.
            out.append((addr, max(size, 1), f"samp patch row {r['gta_addr']} ({kind})"))
    return out


def table_ranges(dll):
    pe = pefile.PE(dll, fast_load=True)
    base = pe.OPTIONAL_HEADER.ImageBase
    out = []
    for name, va, n, rec in SAMP_TABLES:
        data = pe.get_data(va - base, n * rec)
        for i in range(n):
            (addr,) = struct.unpack_from("<I", data, i * rec)
            out.append((addr, 8, name))
    return out + EXTRA


def main():
    points, patches, dll, out_points, out_ex = sys.argv[1:6]
    ranges = csv_ranges(patches) + table_ranges(dll)
    # Only game code ranges matter (data writes never hit a point anyway).
    ranges.sort()
    kept, dropped = [], []
    import bisect
    starts = [r[0] for r in ranges]
    maxlen = max(r[1] for r in ranges)
    for line in open(points):
        s = line.split("#")[0].split()
        if not s:
            kept.append(line)
            continue
        a = int(s[0], 16)
        hit = None
        i = bisect.bisect_right(starts, a)
        j = i - 1
        while j >= 0 and starts[j] > a - maxlen:
            st, sz, why = ranges[j]
            if st <= a < st + sz:
                hit = why
                break
            j -= 1
        if hit:
            dropped.append((a, s[3] if len(s) > 3 else "", hit))
        else:
            kept.append(line)
    with open(out_points, "w") as f:
        f.write(f"# samp-points.py: {len(dropped)} points in samp.dll-touched ranges removed\n")
        f.writelines(kept)
    with open(out_ex, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["address", "function", "reason"])
        for a, fn, why in dropped:
            w.writerow([f"{a:#x}", fn, why])
    print(f"samp ranges: {len(ranges)}; points dropped: {len(dropped)}; kept: "
          f"{sum(1 for l in kept if l.strip() and not l.startswith('#'))}")


if __name__ == "__main__":
    main()
