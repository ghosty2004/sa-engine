#!/usr/bin/env python3
"""INF-10: find coverage points (and hook entries) whose bytes the protection reads as data.

    tools/e2e/cov-data-reads.py POINTS [--hooks FILE] [--exe gta_sa.exe] [--all]

The protection's spliced chunks compute pointers and keys from code bytes, e.g.
`9C F9 A1 7C 06 56 01 1B 05 10 C4 55 00 8B 00 9D` = pushfd; stc; mov eax,[0x156067C];
sbb eax,[0x55C410]; mov eax,[eax]; popfd. An int3 on 0x55C410 (or a hook patch over it) changes
the key and the game goes wrong. This scan decodes, at every executable offset where a dword in game
`.text` follows a disp32 ModRM/SIB or a moffs opcode, the instruction that would read it; it keeps
non-indexed memory reads located in game `.text` or `.HOODLUM` that decode in line from a pushfd/stc/clc a
few bytes before (the chunks' shape; this drops misaligned decodes of ordinary code and the second
`.text`, which is never executed). For each POINTS line whose address lies in a read range it prints
the read, and, when the point is a `jmp rel32` whose target is a point reached by no other direct
branch, that target as the proxy for catalog/coverage-exclude.csv. --hooks FILE (one address per
line, e.g. from `sa-oracle l3 --print-modes ports`): also report hook entries whose first 8 bytes
overlap a read (a patched entry changes the key). --all: print every protection read.
"""
import struct
import sys

import pefile
from capstone import CS_ARCH_X86, CS_MODE_32, Cs
from capstone.x86 import X86_OP_MEM, X86_REG_INVALID

LO, HI = 0x401000, 0x857000
HOODLUM = (0x1556000, 0x1577000)
MODRM_DISP32 = {0x05, 0x0D, 0x15, 0x1D, 0x25, 0x2D, 0x35, 0x3D}


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return 2
    opt = lambda k, d=None: args[args.index(k) + 1] if k in args else d
    exe = opt("--exe", "/home/ghosty2004/sa-engine/re/gta/1.0-US/gta_sa.exe")
    pe = pefile.PE(exe, fast_load=True)
    base = pe.OPTIONAL_HEADER.ImageBase
    img = pe.get_memory_mapped_image()
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    md.detail = True
    mdq = Cs(CS_ARCH_X86, CS_MODE_32)
    b = lambda a: img[a - base]

    def decode(a):
        return next(mdq.disasm(img[a - base:a - base + 16], a, 1), None)

    def in_line(s, a):
        for i in mdq.disasm(img[s - base:a - base + 16], s):
            if i.address >= a:
                return i.address == a
        return False

    def prologue(a):
        for k in range(1, 24):
            if b(a - k) in (0x9C, 0xF8, 0xF9) and in_line(a - k, a):
                return a - k
        return None

    reads = set()
    for s in pe.sections:
        if not s.Characteristics & 0x20000000:
            continue
        va, end = base + s.VirtualAddress, base + s.VirtualAddress + s.Misc_VirtualSize
        if not (LO <= va < HI or va == HOODLUM[0]):
            continue
        for j in range(va + 1, end - 3):
            d = struct.unpack_from("<I", img, j - base)[0]
            if not LO <= d < HI:
                continue
            p = b(j - 1)
            if p not in MODRM_DISP32 and not 0xA0 <= p <= 0xA3 and not (b(j - 2) & 0xC7 == 0x04 and p & 7 == 5):
                continue
            for k in range(1, 9):
                st = j - k
                if st < va:
                    break
                ins = next(md.disasm(img[st - base:st - base + 16], st, 1), None)
                if ins is None or ins.disp_offset != k or ins.mnemonic in ("lea", "nop"):
                    continue
                for op in ins.operands:
                    m = op.mem
                    if (op.type == X86_OP_MEM and op.access & 1 and m.base == X86_REG_INVALID and m.index == X86_REG_INVALID
                            and m.segment == X86_REG_INVALID and m.disp & 0xFFFFFFFF == d):
                        pro = prologue(st)
                        if pro is not None:
                            reads.add((st, ins.bytes.hex(" "), f"{ins.mnemonic} {ins.op_str}", d, op.size, pro))
    reads = sorted(reads)
    print(f"protection reads of game .text: {len(reads)}")
    if "--all" in args:
        for r in reads:
            print(f"  {r[0]:#x} {r[1]:<20} {r[2]:<36} reads {r[3]:#x}+{r[4]} (prologue {r[5]:#x})")
    pts = {}
    for l in open(args[0]):
        if l.startswith("#") or not l.strip():
            continue
        f = l.split()
        pts[int(f[0], 16)] = (f[2], f[3])
    jmp_in = {}
    for s in pe.sections:
        if not s.Characteristics & 0x20000000:
            continue
        va = base + s.VirtualAddress
        for i in range(va, va + s.Misc_VirtualSize - 6):
            o = b(i)
            if o in (0xE8, 0xE9):
                t = (i + 5 + struct.unpack_from("<i", img, i + 1 - base)[0]) & 0xFFFFFFFF
            elif o == 0x0F and 0x80 <= b(i + 1) <= 0x8F:
                t = (i + 6 + struct.unpack_from("<i", img, i + 2 - base)[0]) & 0xFFFFFFFF
            else:
                continue
            jmp_in.setdefault(t, []).append(i)
    hit = {}
    for r in reads:
        for a in range(r[3], r[3] + r[4]):
            if a in pts:
                hit.setdefault(a, []).append(r)
    print(f"points inside protection reads: {len(hit)}")
    for a in sorted(hit):
        kind, func = pts[a]
        ins = decode(a)
        proxy = "-"
        if ins and ins.bytes[0] == 0xE9 and ins.size == 5:
            t = (a + 5 + struct.unpack_from("<i", ins.bytes, 1)[0]) & 0xFFFFFFFF
            others = [x for x in jmp_in.get(t, []) if x != a]
            proxy = f"{t:#x}" + (f" (other branches in: {', '.join(hex(x) for x in others)})" if others else "")
        print(f"{a:#x} {'entry' if kind == 'F' else 'block'} of {func}: {ins.bytes.hex(' ') if ins else '?'} "
              f"({ins.mnemonic + ' ' + ins.op_str if ins else '?'}); proxy {proxy}")
        for r in hit[a]:
            print(f"    read at {r[0]:#x}: {r[1]} {r[2]} (prologue at {r[5]:#x})")
    if opt("--hooks"):
        hooks = [int(l.split("=")[0], 16) for l in open(opt("--hooks")) if l.strip()]
        for h in sorted(hooks):
            for r in reads:
                if h < r[3] + r[4] and r[3] < h + 8:
                    print(f"hook entry {h:#x}: bytes {r[3]:#x}+{r[4]} read at {r[0]:#x} ({r[2]})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
