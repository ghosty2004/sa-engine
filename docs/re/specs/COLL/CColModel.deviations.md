# CColModel: gta-reversed deviation notes (COLL)

Rows from `catalog/gr-deviations.csv`, checked against the original bytes (RE-SPEC-1, writer B).

## 0x40f810 CColModel::AllocateData-void  (gr_deviations = 2; 5 rows mapped)
CAT-14 attached all rows of the three `CColModel::AllocateData` overloads to this address. Only ColModel.cpp:76
belongs to 0x40f810; lines 93/94/101 belong to `AllocateData(int32 x5, bool)` 0x40f870, line 127 to
`AllocateData(int32 size)` (no separate entry; its body is inlined into 0x40f870). They are resolved here anyway,
against 0x40f870's bytes.

Original 0x40f810 (stub -> 0x156deb0), for reference: `this+0x29 &= ~2` (clear `m_bIsSingleColDataAlloc`);
`p = operator new(0x30)` (0x82119a); if `p`: zero u16 +0,+2,+4, u8 +6, `byte +7 &= 0xf8` (bits 3..7 keep heap
garbage), zero dwords +8..+0x2c; `this+0x2c = p` (also stores 0 when `new` returned null). gr's
`new CCollisionData()` matches except that it zeroes the whole flag byte +7 (bits 3..7 are unused padding bits;
harmless).

- ColModel.cpp:76 `assert(m_pColData);`
  - Original bytes: 0x156dec7 `cmp eax, 0` / `je 0x156df05` stores null, no abort.
  - Verdict: REFUTED (assert only). Class: harmless.
- ColModel.cpp:93 "ORIGINAL ALIGNMENT LOGIC, with possible bug" and ColModel.cpp:94 "BUG?: Seems like it could
  clip the last 3 bytes of vertices array" (gr's comment quotes `(vertsOffset + vertsSize) & 0xFFFFFFFC`).
  - Original bytes (0x40f870 -> 0x1561730): 0x1561782 `edx = (spheresEnd + linesEnd + boxes + verts + 3)`,
    0x156178a `and edx, ~3` (ROUND UP, not down), allocation `CMemoryMgr::Malloc(trisOffset + numTris*8, 0)`
    (0x72f420). The triangle pointer at 0x1561820/0x1561827 uses the same `+3 & ~3` round-up (only when
    numVertices != 0; with 0 vertices the offset is already a multiple of 4 because sphere 20, line 32,
    disk 36, box 28 bytes are all multiples of 4).
  - Verdict: REFUTED. gr's quoted "original" formula is wrong; gr's actual code (`std::align`) rounds up exactly
    like the bytes. No clipping bug exists in the original.
  - Class: harmless.
- ColModel.cpp:101 `assert(trianglesOffset && trianglesOffset >= ...)`
  - Original bytes: no check. Verdict: REFUTED (assert). Class: harmless.
- ColModel.cpp:127 "R* used a cast here, leaving the shadow fields uninitialized"
  - Original bytes (0x40f870): after `CMemoryMgr::Malloc` (0x1561793) it writes only: +0 (numSpheres u16),
    +6 (numLines u8), +2 (numBoxes u16), +4 (numTriangles u16), +8 (spheres = base+0x30), +0x10 (lines/disks or
    0), +0xc (boxes or 0), +0x14 (vertices or 0), +0x18 (triangles or 0), +0x1c (trianglePlanes = 0), and
    bit 0 of +7 (`bUsesDisks`, via xor-merge at 0x1561853..0x1561860). +0x20..+0x2f (shadow tri/vert counts and
    pointers) and bits 1..7 of +7 (`bHasFaceGroups`, `bHasShadowInfo`, ...) are left as Malloc returned them.
  - Verdict: CONFIRMED. gr placement-news a `CCollisionData` (constructor zeroes all of them). The port keeps the
    original: do not initialise those fields (callers that load col files set them afterwards).
  - Class: keep-original.
