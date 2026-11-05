# CCollision: gta-reversed deviation notes (COLL)

Rows from `catalog/gr-deviations.csv`, checked against the original bytes (RE-SPEC-1, writer B). Several
functions also show differences that are not CAT-14 rows; they are listed under "Other differences" because a
port that copies gr would diverge. Float notation: `a*b` etc. are x87 ops in the listed order.

## 0x411ec0 CCollision::CalculateColPointInsideBox  (gr_deviations = 1)
- Collision.cpp:121 "Original (0x411EC0): pick the axis with the SMALLEST distance to a face (shallowest exit)"
  - Original bytes: centre = `(max+min)*0.5f` (0x858b8c) per axis; `d = point - centre` (0x411efc..0x411f14);
    per axis `dist = (d <= 0 || NaN) ? point - min : max - point` (0x411f20 `test ah,0x41; jne`). X wins if
    `distX < distY && distX < distZ` (0x411f87/0x411fa1, strict, NaN fails); else Y wins if
    `distY < distX && distY < distZ` (0x411fff/0x41200e); else Z. Normal component = `+1.0` if `d > 0`, else
    `-1.0` (NaN gives -1.0); depth = winning `dist`.
  - Verdict: REFUTED. gr's axis selection (including ties going to Z) is the same.
  - Class: harmless.
- Other differences (not rows):
  - gr starts with `colPoint = {}`; the original writes ONLY point (+0..+0xb), normal (+0x10..+0x1b) and depth
    (+0x28). Surface/piece/lighting A/B (+0x20..+0x25), +0xc, +0x1c and +0x26 are left untouched. CONFIRMED,
    keep-original.
  - NaN handling: original treats `d` NaN as `<= 0` (both for the distance pick and the normal sign); gr's C++
    `<=` treats NaN as false. keep-original.

## 0x412130 CCollision::ProcessSphereBox  (gr_deviations = 3)
- Collision.cpp:160 "Some of the original code, to give you an idea:" (gr replaces the original 3x3x3 branch
  tree by TestSphereBox + per-axis classification)
  - Original bytes: rejection tests `c+r < min` / `c-r > max` per axis interleaved with the classification
    (`min <= c` then `c <= max`); every path rejects before the end, so the set of rejected inputs equals gr's
    TestSphereBox. Outside case (0x412599..0x412671): `p` = clamped corner; `dir = centre - p` (VectorSub
    0x40fe60); `distSq = (x*x + y*y) + z*z` (0x406da0); proceed only if `distSq < minDistSq` (strict,
    0x4125d1 `test ah,5; jp`); `dist = sqrt(distSq)`; proceed if `dist <= radius` (0x4125e9 `test ah,0x41; jp`:
    LESS OR EQUAL; NaN rejects); point = p; normal = `dir * (1.0f/dist)` (0x4119d0: `k = 1.0f [0x858624] / dist`
    then three multiplies); surfA/pieceA/lightA = sphere +0x10/+0x11/+0x12 (u16 + u8 copy);
    surfB/pieceB/lightB = box +0x18/+0x19/+0x1a; depth = `radius - dist`; `minDistSq = distSq` (f32).
  - Verdict: CONFIRMED. gr differs in three ways: (1) gr rejects `dist == radius` (`dist >= r -> false`), the
    original accepts it (depth 0); (2) gr computes the normal as `dir / dist` (three divisions), the original as
    `dir * (1/dist)` (different rounding); (3) gr never copies pieceA/pieceB (+0x21, +0x24), the original copies
    both. NaN classification also differs (original `min <= c` false -> MIN side; gr -> INSIDE).
  - Class: keep-original.
- Collision.cpp:279 "Original (0x412130): face-normal from CalculateColPointInsideBox, depth includes radius"
  - Original bytes (0x412672..0x4126f1): `CalculateColPointInsideBox(box, centre, colp)` writes straight into the
    OUTPUT colpoint; `colp.depth = colp.depth + radius`; `colp.point = colp.point - colp.normal*radius`
    (0x40fec0 vec*float, then VectorSub); surfaces copied as above (u16+u8, i.e. including pieces);
    `*minDistSq = 0`; return 1. No `distSq < minDistSq` test in this path.
  - Verdict: REFUTED for the formula (same normal, point and depth). The piece-type omission noted above also
    applies to this path (CONFIRMED there).
  - Class: harmless (formula); keep-original (pieces).
- Collision.cpp:289 "Original sets it to 0 for inside hits"
  - Original bytes: 0x4126e5 `mov dword [eax], 0`.
  - Verdict: REFUTED (gr matches). Class: harmless.

## 0x412aa0 CCollision::ProcessLineSphere  (gr_deviations = 1)
- Collision.cpp:926 "A little different from the original, but same effect" (normal computation)
  - Original bytes: `t` hit -> `ip = d*t + start` per component (0x412b93..0x412bd8), `n = ip - centre`, then
    `CVector::Normalise` 0x59c910 (`len2 = (x*x + y*y) + z*z`; if `len2 <= 0` set x = 1.0 only; else
    `k = 1.0f/sqrt(len2)`, multiply). gr `(ip - centre).Normalized()` performs the same operations.
  - Verdict: REFUTED for the normal.
  - Class: harmless.
- Other differences (not rows), all keep-original:
  - Quadratic: original `a = (dz*dz + dx*dx) + dy*dy` (0x412abe..0x412ace), `m = centre - start`,
    `b = -((mz*dz + my*dy) + mx*dx)`, `disc = b*b - (((mz*mz + my*my) + mx*mx) - r*r) * a`; reject if `disc < 0`
    (NaN continues); `t = (-b - sqrt(disc)) / a`; reject if `t < 0` (NaN continues), if `t > 1` (NaN continues),
    if `!(t < *depth)` (NaN rejects). gr sums dot products in x,y,z order (different rounding), and adds early
    rejections `c <= 0` (line origin inside or ON the sphere), `b > 0` and `a <= 0`. For `c == 0` with `b < 0`
    the original hits at t = 0; gr rejects.
  - Fields: original writes surfB/pieceB/lightB = sphere +0x10/+0x11/+0x12 and sets surfA (+0x20) = 0 and
    pieceA (+0x21) = 0; it does NOT touch lightA (+0x22). gr sets lightA = 0 and does not copy pieceB nor clear
    pieceA.
  - `ms_iProcessLineNumCrossings` (0x9655d0) += 2 in both.

## 0x413960 CCollision::ProcessDiscCollision  (gr_deviations = 1)
- Collision.cpp:1242 "Original (0x413960): hitK = sqrt(radius² - dx² - dy²) + cp.z"
  - Original bytes: `cp = matBA * tri.point` (0x59c890), `n = Multiply3x3(matBA, tri.normal)` (0x59c790).
    Test 1: `|(n.y*t.y + n.z*t.z) + n.x*t.x| >= 0.77f` (0x858c10) or NaN -> disk branch. Test 2: with
    `dx = cp.x - c.x`, `dy = cp.y - c.y`: `|((cp.z - c.z)*t.z + dy*t.y) + dx*t.x| >= disk.thickness` (+0x20) or NaN
    -> disk branch. Line branch (0x413a1f): `hitK = sqrt((r*r - dy*dy) - dx*dx) + cp.z` with NO clamp; if
    `hitK < *lineRatio` or NaN -> return 0; else `*lineCollision = 1`, `*lineRatio = hitK`,
    `lineColPoint = tri` via 0x40fc80 + depth copy, return 0. Disk branch (0x413a79): if `disk.piece (+0x11) <
    0x11` (unsigned) and `tri.depth > disk.depth` (strict; NaN fails): copy (0x40fc80), copy depth,
    `surfB (+0x23) = 0x3c`, return 1; else return 0.
  - Verdict: CONFIRMED. gr computes `sqrt(max(0, r² - dx² - dy²))` (subtraction order dx then dy; the original
    subtracts dy first) and clamps to 0, so when the point lies outside the disk radius gr produces a finite
    hitK (can set lineCollision) where the original gets NaN and rejects. gr also sums the dot products in
    x,y,z order.
  - Class: keep-original.
- Note: 0x40fc80 (CColPoint copy) copies +0..+0xb, +0x10..+0x1b, +0x20..+0x25 and +0x28 only (not +0xc, +0x1c,
  +0x26/27); gr's defaulted `operator=` copies all 0x2c bytes. Harmless unless something reads the padding.

## 0x415620 CCollision::RayPolyPOP  (gr_deviations = 1)
- Collision.cpp:2678 `NOTSA_UNREACHABLE(); // Unused`
  - Original bytes: a real body (0x415620, 107-line bundle). Callers: none (no call xrefs; `catalog/data-refs.csv`
    only lists rejected references from inside its own body).
  - Verdict: CONFIRMED (gr has no implementation), but dead code in the original.
  - Class: harmless (do not port; if ever needed it requires a new REA spec).

## 0x415950 CCollision::Closest3  (gr_deviations = 1)
- Collision.cpp:499 `NOTSA_UNREACHABLE();`
  - Original bytes: real body; only caller is `ClosestPointOnPoly` 0x418150, which itself has no callers.
  - Verdict: CONFIRMED (no gr body), dead code. Class: harmless.

## 0x415a40 CCollision::ClosestSquaredDistanceBetweenFiniteLines  (gr_deviations = 1)
- Collision.cpp:624 "Original code below:"
  - Original bytes (0x415a40..0x415cf0, `__cdecl float(const CVector* s1p0, const CVector* s2p0, const CVector*
    s2p1, const CVector* u, float a)`, ST0 return): Dan Sunday's segment-segment algorithm, as in gr's
    commented-out block: `v = s2p1 - s2p0`, `w = s1p0 - s2p0`, `b = u.v`, `c = v.v`, `d = u.w`, `e = v.w`,
    `D = a*c - b*b`; parallel test `D < 1e-5f` (0x858c14 = 9.99999975e-6); clamps of sN/tN against 0 (0x858b50)
    and sD/tD; final `sc = |sN| < 1e-5 ? 0 : sN/sD`, `tc = |tN| < 1e-5 ? 0 : tN/tD`; returns `|w + sc*u - tc*v|²`.
    Exact operation order must be taken from the disassembly (e.g. the decompile shows dot products summed in
    y,z,x order).
  - Verdict: CONFIRMED. gr's live code calls `ClosestPtSegmentSegment` (Ericson), a different algorithm with
    different results near degeneracy and different rounding. The port must implement the Sunday version from the
    bytes; gr's comment block is only a guide (it uses a double `0.0` compare and x,y,z sum order).
  - Class: keep-original. Needs a full REA spec before porting (not in this batch).

## 0x416450 CCollision::ProcessSphereSphere  (gr_deviations = 1)
- Collision.cpp:1506 "Original code did it differently (This way sqrt is only used when there's a collision)"
  - Original bytes: `d = A.c - B.c`; `distSq = (d.z*d.z + d.x*d.x) + d.y*d.y` (0x416476..0x416490);
    `touch = sqrt(distSq) - B.r` (stored f32); `depth = A.r - touch` (before the clamp); if `touch < 0`
    -> `touch = 0`; `touchSq = touch*touch`; reject unless `touchSq < *maxTouch` (strict, NaN rejects); reject
    unless `touch < A.r` (strict); `CVector::Normalise(d)`; `point = A.c - n*touch` per component; normal = n;
    A fields = A +0x10..0x12, B fields = B +0x10..0x12; `colp.depth = depth`; `*maxTouch = touchSq`. No
    `(rA+rB)²` pre-test.
  - Verdict: CONFIRMED. gr adds `distSq >= (rA+rB)²  -> false`, which can reject cases at the f32 rounding
    boundary that the original accepts (the original's own test is `sqrt(distSq) - rB < rA`), and sums distSq in
    x,y,z order. Results otherwise match.
  - Class: keep-original.

## 0x416ba0 CCollision::ProcessSphereTriangle  (gr_deviations = 1)
- Collision.cpp:1633 "Original: guard against touching exactly at the surface (touchDist == 0 leads to NaN normal)"
  - Original bytes: a different algorithm from gr's `ClosestPtPointTriangle` (plane projection plus region/edge
    tests using `CVector::Magnitude` 0x4082c0, CrossProduct 0x59c730 and helper 0x411590; constants 0x858b50 = 0,
    0x858624 = 1.0, 0x858b88, 0x858bfc). Epilogue (0x417396..0x41745b): `distSq = dist*dist`; reject unless
    `dist < sphere.r` (strict), then reject unless `distSq < *maxTouch` (strict); normal = `Normalise(centre -
    ip)` (a zero vector becomes (1,0,0), no rejection); point = ip; surfA/pieceA/lightA = sphere +0x10..0x12;
    `surfB (+0x23) = tri.material (byte +6)`, `pieceB (+0x24) = 0`, lightB (+0x25) NOT written; depth =
    `r - dist`; `*maxTouch = dist*dist`.
  - Verdict: CONFIRMED. The claimed guard does not exist in the original (dist == 0 still collides, normal
    (1,0,0)). gr also differs in algorithm, test order, and writes lightB.
  - Class: keep-original. gr's code is not usable as a port source; a full REA spec of 0x416ba0 is needed.

## 0x417bf0 CCollision::ProcessVerticalLine  (gr_deviations = 2)
- Collision.cpp:1809 `&& (!doShootThroughCheck || ...) // NOTSA: Also do this check` (commented out in gr)
  - Original bytes: argument 7 (`doShootThroughCheck`, [esp+0x1c] at entry) is never read.
  - Verdict: REFUTED (gr has the line commented out, so both ignore the flag). Class: harmless.
- Collision.cpp:1834 "No collisions closer to line origin than originally"
  - Original bytes: 0x417e02..0x417e0d `if (!(localDist < *maxTouch)) return 0` (strict; NaN returns 0).
  - Verdict: REFUTED (gr `>= -> false` is the same). Class: harmless.
- Other differences (not rows), keep-original:
  - See-through filter is INVERTED in gr. Original (0x417cf3..0x417d10, same for boxes/triangles): when
    `doSeeThroughCheck` is set, a primitive is processed only if `IsSeeThrough(surface)` (0x55e6b0) returns
    0, i.e. see-through surfaces are skipped. gr processes only see-through surfaces.
  - Line transform: original computes `R^T * (p - pos)` inline (e.g. `x' = (dx*m[0] + dz*m[2]) + dy*m[1]`,
    0x417c33..0x417ccc) for both endpoints and does NOT force the end point's x/y to the start point's. gr uses
    `Invert(transform)` and overwrites `end.x/end.y`.
  - The stored poly is a function-static object at 0x9659fc (init guard bit 0x965a28 & 1, valid flag 0x965a20
    cleared before the triangle loop), not a local; copied to `outColPoly` (11 dwords) only if valid and
    non-null, then its 3 vertices are transformed with `CMatrix::operator*` 0x59c890.
  - Counts are read with `movsx` (signed int16).

## 0x418100 CCollision::ClosestPointsOnPoly  (gr_deviations = 1)
- Collision.cpp:1867 `NOTSA_UNREACHABLE();`
  - Original bytes: real body; 0 callers, no data references. Verdict: CONFIRMED (no gr body), dead code.
  - Class: harmless.

## 0x418150 CCollision::ClosestPointOnPoly  (gr_deviations = 1)
- Collision.cpp:1872 `NOTSA_UNREACHABLE();`
  - Original bytes: real body (calls Closest3 0x415950); 0 callers. Verdict: CONFIRMED, dead code.
  - Class: harmless.

## 0x4181b0 CCollision::SphereCastVsCaches  (gr_deviations = 2)
- Collision.cpp:2901 `assert(!numIn || in[0].ent);`
  - Original bytes: no check. Verdict: REFUTED (assert). Class: harmless.
- Collision.cpp:2958 `NOTSA_UNREACHABLE();` (switch default on entry type)
  - Original bytes: 0x4183de `movzx eax, byte [entry-4]` (type); `dec; je` 1 = TRIANGLE (0x418480), `dec; je`
    2 = SPHERE (0x41843b), `dec; jne 0x418536` 3 = BOX; any other type (including 0, the type of entity-head
    entries) falls to 0x418536 and simply continues the loop.
  - Verdict: CONFIRMED. The original silently skips type 0 / unknown types; gr reaches `NOTSA_UNREACHABLE`
    (e.g. for a head entry whose type is NONE).
  - Class: keep-original.
- Other differences (not rows), keep-original:
  - Back-face triangles (`idx >= 0x7fff`, 0x418484 `cmp di,0x7fff; jae`): the original calls
    `SphereCastVersusVsPoly(spB_os, spA_os, ...)` with the two spheres SWAPPED (0x4184f2/0x4184fb) on triangle
    `0xffff - idx`; gr passes `(spA_os, spB_os)` for both cases.
  - Output entries: for a hit the original writes only `type` (+0) and the matching index field (+8 tri,
    +0xa sphere, +0xc box) into `out[numOut]`, increments, then pre-clears the NEXT slot (`ent (+4) = 0`,
    `type = 0`, 0x418524). gr copies the whole 16-byte input entry.
  - Entity without col data (`GetColModel()->m_pColData == 0`): the original only sets its colData local to 0 and
    keeps iterating the following entries with stale triangle/vertex pointers (sphere/box entries would then
    dereference null+8 / null+0xc). gr skips the region. Never observed in practice.

## 0x4185c0 CCollision::ProcessColModels  (gr_deviations = 28; 39 rows mapped)

Note: the bundle's `## gr-deviations` section actually lists 39 rows (header says 28); all 39 are covered below in bundle order.
Stack args after the 4 pushes: transformA +0x104, cmA +0x108, transformB +0x10C, cmB +0x110, sphereCPs +0x114, lineCPs +0x118, maxTouchDist +0x11C, bReturnAll +0x120. Statics: sphA 0x967F40 (128 x 0x14), collSphA 0x967D40 (128), sphB 0x967340 (128 x 0x14), collSphB 0x967140 (128), collBoxB 0x967040 (64), collTrisB 0x9666E0 (600), line flags 0x9666D0 (16 bytes), lines 0x9664D0 (16 x 0x20), line idx 0x966490 (16), collTriA 0x965B30 (600), collBoxA 0x965A30 (64). AtoB = 0x968988 (Invert(B)*A), BtoA = 0x968940 (Invert(A)*B).

- Collision.cpp:1942 "Highest sphere-CP count the original emits. `1` is subtracted from `std::size(sphereCPs)`"
  - Original bytes: every advance is gated by `cmp esi,0x1F / jge` (0x418DF9, 0x418EC5, 0x418F0A, 0x4195CE, 0x419C77, 0x419DDE); the sentinel write after `inc` goes to CPs[n].depth with n <= 31 (0x418E0F, 0x418EDF, 0x418F1C, 0x4195EC, 0x419C97, 0x419DFE), so max count 31, max index written 31.
  - Verdict: REFUTED (maxSphereCPs = 31 mirrors the bytes)
  - Class: harmless
- Collision.cpp:1968 "Game didn't originally check for this, so I assume no models ever have more than 16 lines."
  - Original bytes: no bound on cdA.m_nNumLines (signed byte, up to 127) anywhere: line path writes 0x9664D0+i*0x20 (0x4196DC, 0x41970C) and 0x966490[i] (0x419744); disk path writes flag byte 0x9666D0[i] (0x419075). All tables are 16 entries; >16 lines overflow into 0x9666D0 / 0x9666E0 (collTrisB).
  - Verdict: REFUTED (claim true; MAX_LINES is only used by an assert)
  - Class: harmless
- Collision.cpp:1983 "Avoid out-of-bounds (Game originally didn't check)"
  - Original bytes: collSphA append 0x4187C9 and collSphB append 0x4188C9 have no compare; transform loops 0x418730 / 0x4187F1 also unbounded vs numSpheres (int16). >128 spheres overflow sphA/sphB (0x14 each) and the index tables.
  - Verdict: REFUTED (assert only; claim true)
  - Class: harmless
- Collision.cpp:2020 "trailing \"B-spheres vs A-tris/boxes\" block must iterate only the real spheres - the original"
  - Original bytes: B-disk hits are NOT appended. 0x4189B2 `mov edi,0x967140` then `mov [edi],esi; add edi,4; inc ebp` (0x418A23..0x418A28): raw disk index (not numSpheres+i) is written to collSphB[0..k-1], OVERWRITING the first k regular entries; count kept separately in [esp+0x50]; numCollSphB [esp+0x48] stays the regular count; cap 0x418A29 `cmp edi,0x967340` (128). Trailing block count = [esp+0x48]-[esp+0x50] (0x419B91, 0x419CCE; jle skips when <= 0), i.e. it iterates regular_count - k entries of the overwritten list (first k are raw disk indices used as sphB/m_pSpheres indices, last k regular hits dropped). First block (0x418D00..0x418D78), disk path (0x419150) and line path (0x4197A0) also loop only [esp+0x48] over the overwritten list.
  - Verdict: CONFIRMED (gr appends `numSpheres+diskIdx` after the regular entries and iterates all regular spheres; the subtraction formula is right but the layout it assumes is wrong). Only reachable when cdB.bUsesDisks and a B disk hits A's bbox.
  - Class: keep-original
- Collision.cpp:2038 "assert(numCollSphB < MAX_SPHERES); // Same bounds as regular spheres"
  - Original bytes: disk append has a real cap: 0x418A29 `cmp edi,0x967340 / jge 0x418A3E` (128 writes starting at collSphB[0]), cannot overflow. Placement difference covered by row 2020.
  - Verdict: REFUTED (assert, not behavioural)
  - Class: harmless
- Collision.cpp:2064 "assert(cdB.m_pTrianglePlanes);"
  - Original bytes: 0x418AF1 CalculateTrianglePlanes(cdB) called unconditionally (even when numTris == 0), planes ptr cached 0x418B07; no null check.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2103 "assert(cdA.m_pSpheres);"
  - Original bytes: 0x418CD6..0x418CED reads cdA.m_pSpheres[idx] (used for the piece test / surface copy) with no null check.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2104 "assert(sphereAIdx < cdA.m_nNumSpheres);"
  - Original bytes: no index check (0x418CD0).
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2129 "Original: ProcessSphereBox itself sets all surface fields (A=sphere, B=box);"
  - Original bytes: PSB 0x412130 writes cp+0x20..0x22 from sphere+0x10 and cp+0x23..0x25 from box+0x18 on both success paths (0x41262D/0x412645, 0x4126B7/0x4126D0).
  - Verdict: REFUTED (claim true)
  - Class: harmless
- Collision.cpp:2130 "the call-site assignments here in the original are redundant re-copies."
  - Original bytes: 0x418DCE..0x418DE1 copies cdA.m_pSpheres[idx]+0x10 (3 bytes) to cp+0x20; sphA[idx] surface was copied from the same source (0x418771..0x418781), so identical bytes. B fields not touched.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2131 "Original: `v61 < 31` (must leave room for +1 sentinel)"
  - Original bytes: 0x418DE4 bReturnAll; 0x418DF3 `cmp byte [sphere+0x11],2 / ja` (unsigned piece <= 2); 0x418DF9 `cmp esi,0x1F / jge`; then inc, flag=0, minTouch=1e24 (0x6753C21C), CPs[n].depth=-1. Else flag=1.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2148 "Original: `v61 < 31` (must leave room for +1 sentinel)"
  - Original bytes: same pattern at 0x418EB0..0x418EE8 (tri loop), no surface copy at the call site.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2161 "Original: `if (v61 >= 31) break;` caps total at 31 CPs"
  - Original bytes: 0x418F0A `cmp esi,0x1F / jge 0x418F40` exits the A-sphere loop; else inc + sentinel (0x418F1C).
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2170 "Transform all colpoints into world space (Originally not here)"
  - Original bytes: one pass 0x419968..0x4199D3 over CPs[0..n) (point via 0x59C890, normal via Multiply3x3, both with transformB), after the line/disk phase, covering first-block and disk-emitted CPs. gr's split (here + disk tail at 2353) covers the same range once.
  - Verdict: REFUTED (claim true, no behavioural effect)
  - Class: harmless
- Collision.cpp:2179 "original performs world-transforms inline during the disk loop's emit handling;"
  - Original bytes: claim inaccurate: the disk loop transforms only lineCPs[i] inline (0x419606..0x419659); emitted sphere CPs are transformed by the common pass 0x419968. gr's post-pass is equivalent.
  - Verdict: REFUTED (comment wrong, behaviour same)
  - Class: harmless
- Collision.cpp:2188 "assert(maxTouchDistances);"
  - Original bytes: dereferenced without check (0x418F6A, 0x419050).
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2189 "assert(lineCPs);"
  - Original bytes: dereferenced without check (0x41903E, 0x419768).
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2190 "assert(cdA.m_nNumLines <= MAX_LINES);"
  - Original bytes: no check; overflow of the 16-entry tables as in row 1968.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2197 "Mirror the original's quirk: the binary *writes* the caller's"
  - Original bytes: 0x418F60..0x418F7B: m_pDisks[i].center.z (+8, stride 0x24) = maxTouchDist[i] for all i < numLines.
  - Verdict: REFUTED (claim true, gr mirrors)
  - Class: harmless
- Collision.cpp:2200 "because the original transforms the already-poisoned disk center into"
  - Original bytes: poison loop ends 0x418F7B, sphere build 0x418FA0..0x419002 transforms the poisoned center with AtoB.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2211 "assert(cdA.m_nNumSpheres + cdA.m_nNumLines <= MAX_SPHERES);"
  - Original bytes: disk spheres written to sphA[0..numLines) (0x418F96 `edi=0x967F40`), not sphA[numSpheres+i]; numLines <= 127 so no overflow. gr's slot choice differs but sphA is not read again after this (last block uses sphB).
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2232 "Original resets the disk's CP depth sentinel EVERY iteration (0x418F2E / *local_d4 = 0xBF800000)."
  - Original bytes: true, but the address is 0x41907C (`mov [ecx],0xBF800000`, ecx=[esp+0x2C]=&CPs[n].depth), executed at the top of every disk iteration before the bbox test.
  - Verdict: REFUTED (behaviour mirrored; cited address wrong)
  - Class: harmless
- Collision.cpp:2247 "Original: strict <=, no epsilon"
  - Original bytes: 0x419214 `fcomp [maxTouch]; test ah,1; jne skip`: accept iff maxTouch <= hitK (ordered), NaN rejected. Compare matches gr. BUT hitK (0x4191F5..0x41920D) is sqrt((r*r - dy*dy) - dx*dx) + cpInA.z with NO max(0) clamp; gr line 2246 clamps (see Other differences).
  - Verdict: REFUTED (for the compare)
  - Class: harmless
- Collision.cpp:2279 "Original: strict <="
  - Original bytes: 0x41939D same compare/accept rule; hitK 0x41937E..0x419396 unclamped (gr line 2277 clamps). Emit test 0x4193B0..0x4193C0: emit iff 0.35f(0x858C0C)*r < hitK - oldMaxTouch.
  - Verdict: REFUTED (for the compare)
  - Class: harmless
- Collision.cpp:2296 "Original (IDA): `qmemcpy(v230, v80, sizeof(v230))` copies the working sphere/box"
  - Original bytes: 0x419468 `rep movsd` 11 dwords from CPs[n] to [esp+0xA4] once before the tri loop; 0x4194C1 resets only scratch depth (+0x28) to -1 per tri; minTouch [esp+0x30] not reset inside the tri loop (1e24 on entry).
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2303 "accumulation the original relies on AND seeds m_fDepth with the box/sphere depth"
  - Original bytes: as above (0x419468, 0x4194C1).
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2311 "Original: when the wheel rests on a roughly-horizontal face of an upright B (e.g. the roof"
  - Original bytes: 0x4194DC..0x41956D: plane normal = int16 * 0.000244140625 (0x858BFC); override triCP.normal iff nz > 0.3f (0x858C24, strict) and transformB+0x28 > 0.9f (0x858C20, strict); then PDC(triCP, BtoA, disk, CPs[n], &flag[i], &maxTouch[i], &lineCP[i]) at 0x4195A4.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2338 "original writes -1e8f here (tells caller the line is entirely miss)."
  - Original bytes: 0x41965E writes 0xCCBEBC20 (-1e8) when flag[i]==0, only on the bbox-pass path (bbox fail jumps to 0x419668).
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2347 "NOTE: we intentionally do NOT restore disk.center.z the original leaks it."
  - Original bytes: no later write to disk+8 anywhere in the function.
  - Verdict: REFUTED (gr mirrors the original)
  - Class: harmless
- Collision.cpp:2351 "transformed here. The original performs equivalent transforms inline"
  - Original bytes: see row 2179 (common pass 0x419968, not inline).
  - Verdict: REFUTED (comment wrong, behaviour same)
  - Class: harmless
- Collision.cpp:2365 "if (!CCollision::TestLineSphere(line, CColSphere{ cmB.m_boundSphere })) { // NOTSA: ..."
  - Original bytes: line path 0x4196B2..0x419962 has no such pre-check; in gr the line is commented out.
  - Verdict: REFUTED (dead code)
  - Class: harmless
- Collision.cpp:2403 "Originally game transformed all sphereCPs to world space here, but I've moved that into the above `if` stuff"
  - Original bytes: 0x419968..0x4199D3 (see row 2170).
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2418 "assert(cdA.m_pTrianglePlanes);"
  - Original bytes: 0x419A47..0x419A56 calls CalculateTrianglePlanes only if cmA.colData != 0; no planes null check.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2444 "Original (0x419B71): resets the working CP's depth sentinel to -1 here, BEFORE the loop."
  - Original bytes: the write is at 0x419B97 and is conditional: 0x419B70 `[esp+0x28]` (numCollTriA) == 0 jumps over it to 0x419CBE. gr writes it unconditionally. Only effect: CPs[n].depth (slot past the returned count) is -1 in gr vs possibly stale in the original when A has no colliding tris; PSB (0x411EC0 writes depth on all paths) does not read it.
  - Verdict: CONFIRMED
  - Class: harmless
- Collision.cpp:2465 "Original: `if (v200 >= 31) break;` caps total at 31 CPs"
  - Original bytes: 0x419C35 negate normal (x -1.0f 0x858C1C), 0x419C77 `cmp eax,0x1F / jge 0x419CBE` leaves the tri section to the box section.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2481 "Original: `if (v200 >= 31) break;` caps total at 31 CPs (`maxSphereCPs`)."
  - Original bytes: no pre-check before PSB in the B-sphere x A-box loop (0x419D30..0x419D57); only the post-hit check 0x419DDE. At n == 31 the original still runs PSB into CPs[31] and on a hit rewrites its surfaces/negates its normal before leaving to the next sphere; gr skips. CPs[31] is in-bounds and outside the returned count.
  - Verdict: CONFIRMED
  - Class: harmless
- Collision.cpp:2500 "Original: `if (v200 >= 31) break;` caps total at 31 CPs"
  - Original bytes: 0x419DDE `cmp eax,0x1F / jge 0x419E12` exits the box loop of this sphere only (next sphere continues); gr `break` same. Surfaces A=box+0x18 (0x419D67), B=cdB.m_pSpheres[idx]+0x10 (0x419D87), normal negated 0x419DA0.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2516 "Original does a full 3-byte swap of {surface,piece,lighting} A<->B:"
  - Original bytes: 0x419EA2..0x419ECD swaps word(+0x20) <-> word(+0x23) and byte(+0x22) <-> byte(+0x25), for CPs[n-added..n) after transformA point/Multiply3x3 normal.
  - Verdict: REFUTED
  - Class: harmless
- Collision.cpp:2525 "Originally here there was a loop to transform all CPs that were added in the above section"
  - Original bytes: 0x419E2C..0x419EDC, runs with start = n - [esp+0x30]; when the outer if is skipped [esp+0x30]=0 (0x4199E5) so it is a no-op. gr's in-block placement is equivalent.
  - Verdict: REFUTED
  - Class: harmless

### Other differences noticed
- B-disk index list layout (0x4189B2..0x418A3E) and every consumer that loops [esp+0x48]: gr appends `numSpheres+diskIdx` and loops over regular+disk entries in the first block, disk path and line path; original overwrites collSphB[0..k) with raw disk indices and loops the regular count only. gr's line path (2376) even reads cdB.m_pSpheres[numSpheres+diskIdx] for appended entries (out of bounds); original line path 0x4197A0 indexes m_pSpheres with whatever collSphB holds. Early-exit 0x418C88 and last-block gate 0x4199D5 also use the regular-only count. keep-original.
- hitK clamp: gr 2246 and 2277 use sqrt(max(0, ...)); original 0x419205 / 0x41938E use bare fsqrt, negative radicand -> NaN -> rejected by 0x41921A / 0x4193A3. With gr's clamp an off-disk hit gives hitK = cpInA.z and can be accepted. keep-original.
- collTrisB cap: original `cmp eax,0x257` (599) after the append (0x418C03, 0x418C6C); gr MAX_TRIS = 600. In the face-group path both only break the inner loop, so later groups keep appending: original overflows the 600-entry table from index 600 on (into collBoxB 0x967040), gr the same one entry later.
- collTriA (A tris vs B bound sphere): original caps at 599 (0x419A99 `cmp eax,0x257 / jge`); gr has no cap (2422-2426, static array of 600 can overflow).
- collBoxA: original has NO cap (0x419B52 `inc edi` only, 64-entry table at 0x965A30 can overflow); gr caps at 64.
- CalculateTrianglePlanes for B is called unconditionally in the original (0x418AF1); gr only when numTris != 0. No behavioural effect expected.
- Matrix inverse: original 0x59B920 is transpose + pos = -(((p.x*r) + p.y*f) + p.z*u) with x87 multiply-add before float store; gr Inverted() is the same algorithm; at most last-ulp differences.
- Disk path computes normalInA (Multiply3x3, 0x4191C4) in the sphere branch but never uses it; harmless.

### Open questions
- 0x4189B2: confirm with a runtime trace (B model with bUsesDisks and regular spheres) that disk hits overwrite collSphB[0..k) with raw disk indices; static reading is unambiguous, but it decides whether the port keeps gr's append layout.
- 0x418D27: first-block disk branch (idx >= numSpheres -> m_pDisks[idx-numSpheres]) is only reachable when a raw disk index >= B.numSpheres; check whether any shipped B model with disks has numSpheres == 0 or fewer spheres than disks.

## 0x419f00 CCollision::SphereCastVsEntity  (gr_deviations = 2)
- Collision.cpp:3003 "There was a bug (Noteably spBws's spRadius was set to spAws's, I've fixed that here)"
  - Original bytes: 0x41a01f and 0x41a05a both load the radius from `[esi+0xc]` where `esi` = first argument
    (spA); `CColSphere::Set` 0x40fd10 builds both object-space spheres with spA's radius (centres from the
    transformed CColLine endpoints, material 0, piece 0, lighting 0xff).
  - Verdict: CONFIRMED (gr uses spB's radius for spB). Class: keep-original.
- Collision.cpp:3050 `assert(!gbTryDoubleSidedCollision || triIdx <= SHRT_MAX);`
  - Original bytes: no check. Verdict: REFUTED (assert). Class: harmless.

## 0x41a5a0 CCollision::SphereVsEntity  (gr_deviations = 1)
- Collision.cpp:3370 `NOTSA_UNREACHABLE(); /* unused */`
  - Original bytes: real body (TestSphereBox, TestSphereTriangle, Invert...); 0 callers.
  - Verdict: CONFIRMED (no gr body), dead code. Class: harmless.

## 0x41ab20 CCollision::CheckCameraCollisionObjects  (gr_deviations = 1)
- Collision.cpp:3198 "Pirulax: At this point I'm certain R* devs were paid by lines written"
  - Original bytes: comment only.
  - Verdict: REFUTED (not behavioural). Class: harmless.
- Other differences (not rows), keep-original:
  - Ignore test: original skips the object if `CWorld::CameraToIgnoreThisObject(obj)` (0x563f40) is true
    (0x41ab82 `jne`) OR if `obj == CWorld::pIgnoreEntity` (0xb7cd68, 0x41ab8e `je`). gr skips only when BOTH hold
    (`&&`).
  - Bounding-sphere test: original builds the sphere from `CEntity::GetBoundCentre` 0x534250 and
    `ms_modelInfoPtrs[obj.modelIndex]->m_pColModel` radius (+0x24); `R = bound.r + spS.r`; continues only if
    `R*R > (dz*dz + dy*dy) + dx*dx` (strict, 0x41abfc `fcompp`, `test ah,0x41; jne skip`). gr's TestSphereSphere
    uses `<=` and x,y,z order.
  - Scan code: original compares/sets `obj+0x2c` against `CWorld::ms_nCurrentScanCode` (0xb7cd78) itself.
