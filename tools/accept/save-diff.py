#!/usr/bin/env python3
"""Compare two GTA SA 1.0 save files (GTASAsfN.b) block by block (SAVE-1, PLAN F5).

  save-diff.py A.b B.b [--bytes N] [--mask]

The blocks are located in A at its "BLOCK" markers (sa-formats save.rs: 28 blocks, each introduced by the 5
bytes "BLOCK", no sizes); B is split at the same offsets when its markers sit at the same places, else at its
own. Prints, per block, the size and the differing byte ranges (offsets relative to the block's data start, so
they can be matched with the block's Save function), then the padding / checksum tail. Exit 0 when the files
are byte-identical, 1 otherwise, 2 on a usage or format error.

--mask (docs/arch/save-port.md, ledger SAVE-1): the bytes the 1.0 save code takes from uninitialised stack
locals (the structure bytes `Construct` never writes) are located in A's layout and allowed to differ, and so
are the padding bytes that copy them (the padding at stream position p repeats the work buffer byte of p -
0xC800). Exit 3 when every difference is such a residue byte (and both checksums are valid), 1 otherwise.
"""
import sys

NAMES = ["SIMPLE_VARIABLES", "SCRIPTS", "POOLS", "GARAGES", "GAMELOGIC", "PATHS", "PICKUPS", "PHONEINFO",
         "RESTART", "RADAR", "ZONES", "GANGS", "CAR_GENERATORS", "PED_GENERATORS", "AUDIO_SCRIPT_OBJECT",
         "PLAYERINFO", "STATS", "SET_PIECES", "STREAMING", "PED_TYPES", "TAGS", "IPLS", "SHOPPING", "GANGWARS",
         "STUNTJUMPS", "ENTRY_EXITS", "RADIOTRACKS", "USER3DMARKERS"]
DATA_SIZE = 202748


def markers(b):
    """Offsets of the 28 block markers, in order: each found after the previous block's start."""
    out, pos = [], 0
    for _ in NAMES:
        i = b.find(b"BLOCK", pos)
        if i < 0:
            break
        out.append(i)
        pos = i + 5
    return out


def ranges(a, b, lo, hi):
    """Differing [start, end) ranges of a[lo:hi] vs b[lo:hi]."""
    out, start = [], None
    for i in range(lo, hi):
        d = i >= len(a) or i >= len(b) or a[i] != b[i]
        if d and start is None:
            start = i
        elif not d and start is not None:
            out.append((start, i))
            start = None
    if start is not None:
        out.append((start, hi))
    return out


SIMPLE_VARS_WRITTEN = [(0x0, 0x4), (0x1B, 0x1C), (0x68, 0x69), (0x6C, 0x90), (0x90, 0x91), (0x94, 0xAE), (0xB0, 0xC9),
                       (0xCC, 0xD1), (0xD4, 0xE2), (0xE4, 0xEF), (0x11C, 0x11D), (0x11E, 0x12E), (0x130, 0x137)]


def u32(b, o):
    return int.from_bytes(b[o:o + 4], "little")


def residue_mask(a, ma):
    """Positions of `a` that hold stack residue in a 1.0 save (see --mask), or None when a block cannot be parsed."""
    mask = bytearray(len(a))

    def struct(base, size, holes):
        for h in holes:
            if h < size:
                mask[base + h] = 1

    # block 0: CSimpleVariablesSaveStructure (0x138)
    b0 = ma[0] + 5
    written = bytearray(0x138)
    for lo, hi in SIMPLE_VARS_WRITTEN:
        written[lo:hi] = b"\1" * (hi - lo)
    nul = a.index(0, b0 + 4, b0 + 0x138) - b0
    written[4:nul + 1] = b"\1" * (nul + 1 - 4)
    struct(b0, 0x138, [k for k in range(0x138) if not written[k]])
    # block 2: pools (player peds: CPed 4 + 0x18C, CPlayerPed 4 + 0x84; objects: 4 + 4 + 4 + 0x30)
    p = ma[2] + 5
    n = u32(a, p)
    p += 4
    for _ in range(n):
        ty = u32(a, p + 8)
        p += 12
        struct(p + 4, 0x18C, [0x183, 0x18A, 0x18B])
        p += 4 + 0x18C
        if ty == 0:
            struct(p + 4, 0x84, [5, 6, 7, 0x81, 0x82, 0x83])
            p += 4 + 0x84
    n = u32(a, p)
    p += 4
    for _ in range(n):
        struct(p + 12, 0x30, [0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x19, 0x25, 0x26, 0x27])
        p += 12 + 0x30
    if p != ma[3]:
        return None
    # block 3: garages (statics 0x27, stored cars 0x1400, NumGarages x CSaveGarage 0x50)
    g0 = ma[3] + 5
    n = u32(a, g0)
    p = g0 + 0x27 + 0x1400
    for _ in range(n):
        nul = a.index(0, p + 0x44, p + 0x50) - p
        struct(p, 0x50, [3] + list(range(nul + 1, 0x4C)) + [0x4D, 0x4E, 0x4F])
        p += 0x50
    if p != ma[4]:
        return None
    # block 15: CPlayerInfo (4 + 0x28), block 23: CGangWars (4 + 0x58)
    struct(ma[15] + 5 + 4, 0x28, [7, 0x11, 0x12, 0x13, 0x25])
    struct(ma[23] + 5 + 4, 0x58, [1, 2, 3, 0x4D, 0x4E, 0x4F])
    # the padding after the last block (USER3DMARKERS, 5 x 0x1C) repeats the work buffer
    end = ma[27] + 5 + 5 * 0x1C
    for q in range(end, DATA_SIZE):
        mask[q] = mask[q - 0xC800]
    return mask


def checksum_ok(b):
    return len(b) == DATA_SIZE + 4 and sum(b[:DATA_SIZE]) & 0xFFFFFFFF == u32(b, DATA_SIZE)


def main(argv):
    show = 16
    args = [x for x in argv[1:]]
    use_mask = "--mask" in args
    if use_mask:
        args.remove("--mask")
    if "--bytes" in args:
        k = args.index("--bytes")
        show = int(args[k + 1])
        del args[k:k + 2]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    a, b = (open(p, "rb").read() for p in args)
    if a == b:
        print(f"IDENTICAL {len(a)} bytes")
        return 0
    print(f"sizes {len(a)} / {len(b)}")
    ma, mb = markers(a), markers(b)
    if len(ma) != len(NAMES):
        print(f"{args[0]}: only {len(ma)} block markers", file=sys.stderr)
        return 2
    if ma != mb:
        print(f"block offsets differ: {[(NAMES[i], x, y) for i, (x, y) in enumerate(zip(ma, mb)) if x != y][:4]}")
    # The last block ends where the user 3D markers end; we cannot know without a parser, so the tail after
    # the 28th marker is reported as one region together with the padding and the checksum.
    ends = ma[1:] + [DATA_SIZE]
    total = 0
    for k, name in enumerate(NAMES):
        lo, hi = ma[k] + 5, ends[k]
        r = ranges(a, b, lo, hi)
        if not r:
            continue
        n = sum(e - s for s, e in r)
        total += n
        print(f"{k:2} {name} (file {ma[k]:#x}, {hi - lo:#x} bytes): {n} bytes differ in {len(r)} ranges")
        for s, e in r[:show]:
            print(f"     +{s - lo:#06x}..+{e - lo:#06x} ({e - s}): {a[s:min(e, s + 12)].hex()} / {b[s:min(e, s + 12)].hex()}")
        if len(r) > show:
            print(f"     ... {len(r) - show} more")
    r = ranges(a, b, DATA_SIZE, max(len(a), len(b)))
    if r:
        print(f"checksum: {a[DATA_SIZE:].hex()} / {b[DATA_SIZE:].hex()}")
    print(f"{total} data bytes differ")
    if not use_mask:
        return 1
    mask = residue_mask(a, ma)
    if mask is None:
        print("mask: the pools or garages block of A did not parse")
        return 1
    if ma != mb:
        print("mask: block offsets differ")
        return 1
    bad = [i for i in range(DATA_SIZE) if a[i] != b[i] and not mask[i]]
    resid = sum(1 for i in range(DATA_SIZE) if a[i] != b[i] and mask[i])
    pad_ok = all(a[q] == a[q - 0xC800] for q in range(ma[27] + 5 + 5 * 0x1C, DATA_SIZE))
    print(f"mask: {sum(mask)} residue positions; {resid} differing bytes are residue, {len(bad)} are not; "
          f"checksums valid {checksum_ok(a)}/{checksum_ok(b)}; padding repeats the buffer: {pad_ok}")
    for i in bad[:show]:
        blk = max(k for k in range(len(NAMES)) if ma[k] <= i)
        print(f"     not residue: {NAMES[blk]} +{i - ma[blk] - 5:#x}: {a[i]:02x} / {b[i]:02x}")
    return 3 if not bad and checksum_ok(a) and checksum_ok(b) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
