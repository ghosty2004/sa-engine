# CCollision helpers (COLL), full specs

## 0x50d590 CCollision::sub_50D590 (proposed: GetNearestDistanceOfPedSphereToCameraNearClip)  (catalogue: unknown/REA, port level 4)
Name evidence: only caller is `CCollision::CheckCameraCollisionPeds` 0x415320; gta-reversed has the same logic as
a file-local free function `GetNearestDistanceOfPedSphereToCameraNearClip(CPed*)` (Collision.cpp:2585, no
address, called from CheckCameraCollisionPeds which tests `> 0.f`).
- Convention / signature: `__cdecl float (CPed* ped [esp+4])`, result in ST0 (an f32 value, see below), plain
  `ret`. Frame `sub esp, 0xc`; preserves ESI/EDI; the argument slot itself is reused as a float temporary.
- Callers: 1 (0x415320). Callees: `CPedModelInfo::AnimatePedColModelSkinnedWorld` 0x4c7170
  (`__thiscall CColModel*(CPedModelInfo* this, RpClump* clump)`, `ret 4`; return value ignored).
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xa9b0c8 (read, indexed); `TheCamera.m_nActiveCam` 0xb6f081 (u8);
  `TheCamera.m_aCams` 0xb6f19c (stride 0x238); `Scene.m_pRwCamera` 0xc1703c.
- Fields: `ped+0x22` int16 `m_nModelIndex` (movsx); `ped+0x18` `m_pRwClump`; `mi+0x34` `CPedModelInfo::
  m_pHitColModel`; `colModel+0x2c` `m_pColData`; `colData+8` `m_pSpheres` (CColSphere stride 0x14: centre x,y,z
  +0,+4,+8, radius +0xc); `cam+0x190/0x194/0x198` `m_vecFront` x,y,z; `cam+0x19c/0x1a0/0x1a4` `m_vecSource` x,y,z;
  `rwCamera+0x80` `nearPlane` (RwCameraGetNearClipPlane).
- Behaviour:
  1. `mi = ms_modelInfoPtrs[ped.m_nModelIndex]`; `best = 999999.0f` (stored to a local, 0x50d5aa);
     `mi->AnimatePedColModelSkinnedWorld(ped->m_pRwClump)`.
  2. `cam = &TheCamera.m_aCams[m_nActiveCam]`; `sp = mi->m_pHitColModel->m_pColData->m_pSpheres`.
  3. `off = (F.z*S.z + F.y*S.y) + F.x*S.x` (0x50d5e1..0x50d607, stored f32); `nearClip = rwCamera->nearPlane`
     (stored f32). ST0 = `best` (loaded from the local).
  4. Exactly 12 spheres, unrolled 6 per iteration, 2 iterations (EDI = 2), pointers advance 0x78. For each sphere
     `s` in index order 0..11 compute `v` with this operation order (F = front):
     - spheres 0 and 1 of each group of 6 (indices 0,1,6,7): `v = ((c.z*F.z + c.x*F.x) + F.y*c.y)`
     - spheres 2..5 of each group (indices 2-5, 8-11): `v = ((c.z*F.z + c.y*F.y) + c.x*F.x)`
     then `v = ((v - off) - s.radius) - nearClip`; `fst` to a f32 temp; compare with the running minimum in ST0:
     if `v < best` (strict; NaN false) then `best` is replaced by the f32 temp re-loaded (`fstp st0; fld temp`),
     otherwise `best` stays.
  5. Return with `best` in ST0 (either the 999999.0f start value or an f32-rounded sphere value).
- Edge cases: assumes the hit col model has at least 12 spheres (no count check; `CPedModelInfo` hit models are
  built with 12). No null checks (m_pHitColModel, colData, Scene.m_pRwCamera). The NaN rule means a NaN distance
  never becomes the minimum.
- gta-reversed: present as the file-local helper above, not hooked. Differences: starts at `FLT_MAX` instead of
  999999.0f (a ped whose spheres are all farther than 999999 returns 999999.0 in the original), iterates
  `m_nNumSpheres` instead of a fixed 12, uses `Dot` (x,y,z sum order) for both dot products and `std::min`
  (NaN handling differs). Port the bytes.
- Open questions: none.

## 0x415a40 CCollision::ClosestSquaredDistanceBetweenFiniteLines  (catalogue: specified/gta-reversed, port level 0)
Written by the RE-SPEC-1 coordinator because gta-reversed's live code uses a different algorithm (Ericson's
`ClosestPtSegmentSegment`); see CCollision.deviations.md. gr's commented-out "Original code below" block
(Collision.cpp:624) has the right structure, but not the operation order or the compare details given here.
- Convention / signature: `__cdecl float (const CVector* p0, const CVector* q0, const CVector* q1,
  const CVector* u, float uu)`; result in ST0 (not rounded to f32 by a store: it is the last `faddp`); plain `ret`.
  Segment 1 = `p0 .. p0+u` (u is a direction, not an end point; gr's header calls it `line1End`), segment 2 =
  `q0 .. q1`, `uu` = the caller's `u·u`. Frame `sub esp, 0x40`; the argument slots are reused as temporaries.
  No callees, no globals besides two constants.
- Callers: 1 (`CCollision::SphereCastVersusVsPoly` 0x415cf0, 3 sites: 0x416075, 0x41609d, 0x4160cc).
- Constants: `0x858c14` f32 9.99999975e-6 (`SMALL`), `0x858b50` f32 0.0.
- Behaviour (all intermediate values stored as f32 unless noted "reg"):
  1. `v = q1 - q0` (x, y, z in that order, each `q1.c - q0.c`); `w = p0 - q0`.
  2. `b = (v.z*u.z + v.y*u.y) + v.x*u.x`; `c = (v.x*v.x + v.z*v.z) + v.y*v.y`;
     `d = (w.z*u.z + w.y*u.y) + w.x*u.x`; `e = (w.x*v.x + w.z*v.z) + w.y*v.y`.
  3. `D = c*uu - b*b` (0x415b03..0x415b15: `fld c; fmul uu; fld b; fmul b; fsubp` = `c*uu - b*b`). Initialise
     `sD = D`, `tD = D`.
  4. If `D < SMALL` (strict; NaN or `>=` goes to step 5) [parallel]: `sD = 1.0f`, `sN = 0.0f`, `tN = e`, `tD = c`;
     go to step 6.
  5. `sN = e*b - d*c` (reg); `tN = e*uu - d*b` (stored).
     - if `sN < 0` (strict): `sN = 0.0f`, `tN = e`, `tD = c`.
     - else if `sN > D` (i.e. not `sN <= D`; NaN sN keeps NaN): `sN = D` (f32 reload), `tN = e + b`, `tD = c`.
  6. Clamp t:
     - if `tN < 0` (strict): `tN = 0`; `x = -d` (`fchs`), then the s-clamp below.
     - else if `tN > tD` (not `tN <= tD`, so NaN keeps): `tN = tD`; `x = b - d` (`fld b; fsub d`), then the
       s-clamp below.
     - else keep `sN` as is (no s-clamp).
     s-clamp on `x`: if `x < 0` (strict) → `sN = 0.0f`; else if `x <= uu` (or NaN) → `sN = x`, `sD = uu`;
     else (`x > uu`) → `sN = sD` (the current `sD`: 1.0f after step 4, otherwise `D`).
  7. `sc = (|sN| < SMALL) ? 0.0f : sN / sD` (reg); `tc = (|tN| < SMALL) ? 0.0f : tN / tD` (reg). `fabs` of the
     register value; `<` strict, NaN → takes the division.
  8. Result, in this order (0x415c6e..0x415cea): `tvx = v.x*tc` (stored), `tvy = v.y*tc` (stored), `tvz = tc*v.z`
     (reg); `Px = sc*u.x + w.x` (stored), `Py = sc*u.y + w.y` (reg), `Pz = sc*u.z (stored) + w.z` (stored);
     `dx = Px - tvx` (stored), `dy = Py - tvy` (stored), `dz = Pz - tvz` (reg);
     return `(dz*dz + dy*dy) + dx*dx` in ST0.
- Edge cases: `uu == 0` and `c == 0` give `D = 0 < SMALL` → parallel path, no division by zero; `tD = c = 0` with
  `|tN| >= SMALL` divides by zero (inf/NaN propagates). The register intermediates (`sN`, `sc`, `tc`, `Py`, `tvz`,
  `dz`) keep the extended exponent range (precision control is 24-bit), so they differ from an all-f32 version
  only on overflow/underflow.
- gta-reversed: hooked and "reversed", but the live code is Ericson's algorithm (different results near
  degeneracy and different rounding). Port this spec, not gr.
- Open questions: none.

## 0x416ba0 CCollision::ProcessSphereTriangle  (catalogue: specified/gta-reversed, port level 1)
Written by the RE-SPEC-1 coordinator because gta-reversed's live code uses `ClosestPtPointTriangle`, a different
algorithm with a different test order (see CCollision.deviations.md). The frame offsets below are relative to
the entry ESP (`E`); Ghidra's `local_XX` names in the bundle are the same offsets.
- Convention / signature: `__cdecl bool (const CColSphere* s [E+4], const CompressedVector* verts [E+8],
  const CColTriangle* tri [E+0xc], const CColTrianglePlane* plane [E+0x10], CColPoint* cp [E+0x14],
  float* maxTouchSq [E+0x18])`; returns AL (0/1; the upper EAX bytes are garbage); plain `ret`. Saves
  EBX/EBP/ESI/EDI.
- Callers: 1 (`CCollision::ProcessColModels` 0x4185c0, 3 sites).
- Callees (all `__cdecl` unless noted): `0x411590` decompress vertex (`out.c = (float)(int16)in.c * 0.0078125f`,
  x, y, z); `CVector::CrossProduct` 0x59c730 (`out = a × b`, x = `b.z*a.y - a.z*b.y`, y = `a.z*b.x - b.z*a.x`,
  z = `a.x*b.y - b.x*a.y`); `VectorSub` 0x40fe60 (`out = a - b`, returns out); `CVector::Magnitude` 0x4082c0
  (`__thiscall`, ST0 = `sqrt((x*x + y*y) + z*z)`); `0x40fec0` (`out = vec * f`, each `f*vec.c`, returns out);
  `0x40fe30` (`out = a + b`, each `a.c + b.c`, returns out); `CVector::Normalise` 0x59c910 (`__thiscall`;
  `l2 = (x*x + y*y) + z*z`; if `l2 <= 0` only `x = 1.0f` (y, z untouched), NaN → normal path;
  else `k = 1.0f/sqrt(l2)` and x, y, z `*= k`).
- Constants: `0x858bfc` f32 1/4096 (plane normal scale), `0x858b88` f32 1/128 (plane distance and vertex scale),
  `0x858b50` f32 0.0, `0x858624` f32 1.0.
- Layouts: CColSphere centre +0/+4/+8, radius +0xc, surface/piece +0x10 (word), lighting +0x12. CColTriangle
  vertex indices u16 +0/+2/+4 (`movzx`), material byte +6. CColTrianglePlane int16 normal +0/+2/+4 (`movsx`),
  int16 distance +6. Vertex stride 6 (`verts + idx*6`). CColPoint point +0, normal +0x10, surfaceA/pieceA
  +0x20/+0x21, lightingA +0x22, surfaceB +0x23, pieceB +0x24, depth +0x28.
- Behaviour (stored = f32 store/reload; reg = kept on the x87 stack):
  1. Plane: `n.c = (float)plane.n[c] * (1/4096)` for x, y, z (stored). `pd = ((n.z*s.z + n.y*s.y) + n.x*s.x) -
     (float)plane.d * (1/128)` (stored as `pd`), `apd = |pd|` (stored). If `apd > s.radius` (strict; equal and
     NaN continue) → return 0. `pd2 = pd*pd` (stored); if `pd2 > *maxTouchSq` (strict; equal/NaN continue) →
     return 0.
  2. `A, B, C` = decompressed `verts[tri.a]`, `verts[tri.b]`, `verts[tri.c]`.
  3. Edge frame: `ab = B - A` (stored x, y, z); `lenAB = sqrt((ab.x*ab.x + ab.z*ab.z) + ab.y*ab.y)` (stored);
     `inv = 1.0f / lenAB` (stored); `e = ab * inv` (each component `ab.c * inv`, stored); `m = e × n` (CrossProduct
     with a = e, b = n).
  4. `ac = C - A` (stored); `cE = (ac.x*e.x + ac.z*e.z) + ac.y*e.y` (stored);
     `cM = (ac.x*m.x + ac.z*m.z) + ac.y*m.y` (stored).
     `p = s.centre - A` (reg); `pE = (p.x*e.x + p.z*e.z) + p.y*e.y` (stored);
     `pM = (p.x*m.x + p.z*m.z) + p.y*m.y` (stored).
     2D coordinates: A = (0,0), B = (lenAB, 0), C = (cE, cM), P = (pE, pM).
  5. Edge-side tests (`>= 0`, NaN → false):
     - `s1 = pM*lenAB - pE*0.0f` (stored); `in1 = s1 >= 0`  (inside edge AB).
     - `t1 = pE*cM` (stored), `t2 = pM*cE` (stored); `in2 = (t1 - t2) >= 0`  (inside edge AC).
     - `cEl = cE - lenAB` (stored), `pEl = pE - lenAB` (stored); `in3 = (cEl*pM - pEl*cM) >= 0` (inside edge BC).
     `k = in1 + in2 + in3`.
     With the plane built by the game (`n = normalize((C-A) × (B-A))`, gr ColTrianglePlane.cpp) an interior point
     gives k = 3.
  6. Closest point `Q` and distance `dist` (stored f32):
     - **k = 3** (0x416ee9): `dist = apd`; `Q.x = s.x - n.x*pd`, `Q.y = s.y - n.y*pd` (products reg),
       `Q.z = s.z - (n.z*pd stored)`.
     - **k = 1** (0x4172e7): if `in1` → vertex C; else if `in2` → vertex B; else → vertex A.
       `dist = Magnitude(s.centre - V)` (VectorSub then Magnitude), `Q = V`.
     - **k = 2** (0x416f50), the outside edge is the one whose test failed:
       - outside AB (`!in1`): `L = lenAB*lenAB` (stored); `t = (pE*lenAB + pM*0.0f) / L` (stored).
         `t <= 0` (`<=`, NaN no) → vertex A; else `t >= 1` (not `t < 1`; NaN counts as `< 1`) → vertex B
         (vertex cases as in k = 1). Else: `q = s1 / sqrt(L)` (FDIVR), `dist = sqrt(q*q + pd2)`,
         `Q = A + ab*t`.
       - outside AC (`in1 && !in2`): `L = cM*cM + cE*cE` (stored); `t = (pM*cM + pE*cE) / L` (stored).
         `t <= 0` → vertex A; `t >= 1` → vertex C; else `q = (t2 - t1) / sqrt(L)`, `dist = sqrt(q*q + pd2)`,
         `Q = A + ac*t`.
       - outside BC (`in1 && in2`): `L = cM*cM + cEl*cEl` (stored); `t = (pEl*cEl + pM*cM) / L` (stored).
         `t <= 0` → vertex B; `t >= 1` → vertex C; else `q = (pM*cEl - pEl*cM) / sqrt(L)`,
         `dist = sqrt(q*q + pd2)`, `bc = C - B` (stored), `Q = B + bc*t`.
       In the edge cases `q` and `q*q + pd2` are register values and only `dist` is stored. `A + v*t` is
       `0x40fec0` then `0x40fe30` (each component `A.c + (t*v.c)` with the product stored f32).
     - **k = 0** (0x416ee3 → 0x417396): nothing is computed. `dist` is the stale `lenAB` and `Q` is
       **uninitialised stack** (E-0x12c/-0x128/-0x124). See the oddities below.
  7. Epilogue (0x417396): `d2 = dist*dist` (stored). If not `dist < s.radius` (strict; NaN → return 0) → return 0.
     If not `d2 < *maxTouchSq` → return 0. `nrm = s.centre - Q` (stored), `Normalise(nrm)`. Write `cp.point = Q`,
     `cp.normal = nrm`, word `cp+0x20 = s+0x10`, `cp+0x22 = s+0x12`, `cp+0x23 = tri.material` (+6),
     `cp+0x24 = 0`, `cp.depth = s.radius - dist`; `*maxTouchSq = d2`; return 1. `cp+0x25` (lightingB) is **not**
     written.
- Edge cases and oddities (keep them):
  - `dist == 0` still collides; the normal comes from Normalise's zero branch, giving `(1, ny, nz)` with ny/nz
    being whatever (usually 0) components were left. There is no `touchDist == 0` guard (gr's comment is wrong).
  - **k = 0 uses uninitialised Q**: this happens for a degenerate triangle with `lenAB == 0` (`inv = inf`,
    `e = NaN`, every test false), for a NaN anywhere, or when the stored plane's sign disagrees with the winding.
    Then `dist = lenAB` (0 for an A == B triangle, which passes both tests) and the contact point and normal are
    built from leftover stack bytes of the caller's frame.
  - The first rejection uses `>` (touching at exactly `radius` passes step 1 but fails step 7's strict `<`).
- gta-reversed: hooked and "reversed", but its body is a different algorithm (closest point via barycentric
  regions, squared-distance tests in a different order, `touchDistSq <= 0` rejection, lightingB written). Port
  this spec.
- Open questions:
  - 0x417396 (k = 0 path): what the uninitialised `Q` holds depends on the stack contents left by earlier calls
    inside `ProcessColModels` 0x4185c0. Needs a runtime measurement (oracle L2 with a degenerate triangle) before
    the port picks a value. A Rust port must at least reproduce `dist = lenAB` and the acceptance tests.
