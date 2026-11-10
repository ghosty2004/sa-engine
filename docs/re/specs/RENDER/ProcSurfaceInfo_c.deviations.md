# ProcSurfaceInfo_c (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Plant/ProcSurfaceInfo.cpp`. ProcSurfaceInfo_c (ProcSurfaceInfo.h, offsets confirmed by
the bytes): +0x00 m_SurfaceId (u8), +0x04 m_ModelIndex (int32), +0x08 m_fSpacing, +0x0C m_fSquaredSpacingRadius
(= 1/spacing^2), +0x10 m_fSquaredMinDistance, +0x14/+0x18 min/max rot (rad), +0x1C/+0x20 min/max scale, +0x24/+0x28 min/max
scale Z, +0x2C/+0x30 z offset min/max, +0x34 m_Align (bool), +0x38 m_fUseGrid (float), +0x3C m_Objects (TList_c).
ProcObjectListItem: +0x08 m_Obj, +0x0C m_LocTri, +0x10 m_bAllocatedMatrix. CPlantLocTri: +0x00 V1, +0x0C V2, +0x18 V3,
+0x47 m_nLighting. `g_procObjMan` 0xBB7CB0 (+0 m_numAllocatedMatrices), its free list m_ObjectsList at 0xBBC8B8.
`t` below = `(float)rand() * 3.0518509e-05f [0x858C7C]` (1/32767, rand() 0x821B1E, no mask).

## 0x5A32D0 ProcSurfaceInfo_c::AddObject  (gr_deviations = 5)
Row ProcSurfaceInfo.cpp:74 ("Original function has some exception handling built in"):
- Original bytes 0x5A32D0-0x5A32E5 / 0x5A3374, 0x5A33FE, 0x5A35A4, 0x5A37F3: an SEH frame whose state only guards the
  `new` + ctor pairs (unwind = pool delete 0x5A1F20 / 0x4040A0) and the stack CMatrix destructors (0x59ACD0).
- Verdict: REFUTED (no behaviour without C++ exceptions). Class: harmless.

Rows :113, :114, :115, :116 (`lerp(min, max, GetRandomNumberInRange(0.f, 1.f))` for xyScale, zScale, rotation, zOffset):
- Original bytes 0x5A3476-0x5A355A, four rand() calls in this order, each `v = (max - min) * t + min` (x87: `fld max;
  fsub min; fmul t; fadd min`, stored f32):
  xyScale = (+0x20 - +0x1C)*t + +0x1C (0x5A3493-0x5A34A5); zScale = (+0x28 - +0x24)*t + +0x24 (0x5A34C8-0x5A34DA);
  rotation = (+0x18 - +0x14)*t + +0x14 (0x5A3501-0x5A350F); zOffset = (+0x30 - +0x2C)*t + +0x2C (0x5A3537-0x5A355A).
- gr: `lerp(a, b, t) = b*t + a*(1 - t)` (common.h:197); same t, different rounding.
- Verdict: CONFIRMED (all four). Class: keep-original.

Found in passing (CONFIRMED gr logic errors, keep-original):
- Scale block (0x5A3786-0x5A37B0): `[esp+0x1F]` holds `isObject = (modelInfo->+0x10 (i16) != -1)` (setne at 0x5A333D).
  The original SKIPS the scale block for objects (`cmp [esp+0x1F], 0; jne 0x5A3848`), i.e. only BUILDINGS are scaled;
  and it scales when `xyScale != 1.0f || zScale != 1.0f` (`test ah,0x44; jp` at 0x5A379F, then `jnp` skip at 0x5A37B0;
  NaN counts as "not equal"). gr writes `!isBuilding && (xy != 1 && z != 1)` - both the entity kind and the `&&` are
  wrong. Inside: needs `rwObject (+0x18) != 0` and `rwObject->parent (+4) + 0x10 != 0` (0x5A37B6-0x5A37CC);
  `m = CMatrix(GetModellingMatrix(), false)` (0x46A2D0, 0x59C050); `0x5A2E60(m, xy, xy, z)` (scale XYZ);
  `m.pos.z = m.pos.z - (minBound.z * zScale - minBound.z)` (0x5A3803-0x5A381F); `m.UpdateRW()` 0x59BBB0;
  `obj->UpdateRwFrame()` 0x532B00; `~CMatrix`.
- Non-aligned position (0x5A36EA-0x5A3704): `pos.z = (pos.z - minBound.z) + zOffset` (that order); gr adds
  `(zOffset - minBound.z)`.
- Original bugs kept by both: when `RemoveHead` (0x4A8E70) succeeded but the function returns null (aligned and
  `normal.z < 0.95f [0x858EF0]` and `m_numAllocatedMatrices >= 200` at 0x5A3320; `CObject::nNoTempObjects (u16 0xBB4A70)
  >= 150` at 0x5A334C; allocation failure) the list item is not given back (leak of a free item).

## 0x5A3850 ProcSurfaceInfo_c::AddObjects  (gr_deviations = 7)
Row :182 ("Originally placed inside the below loop", `normal.Normalise()` once before the loop):
- Original bytes 0x5A3D89: in the random (non-grid) path `CVector::Normalise(&normal)` 0x59C910 is called inside the loop,
  once per accepted sample, before AddObject. The first call normalises the raw cross product; later calls re-normalise
  an already unit vector (can change the last bits).
- Verdict: CONFIRMED (minor rounding). Class: keep-original (normalise per accepted sample).

Row :185 (comment on srand/rand use):
- Original bytes 0x5A3C4E-0x5A3C7D: `seed = ftol((((V3.z + V3.y) + V3.x) + ((V2.z + V2.y) + V2.x)) + ((V1.z + V1.y) + V1.x)
  + (int)m_ModelIndex)` (fiadd of the dword at +4); `old = rand()`; `srand(seed)` (0x821B11). At the end
  (0x5A3DFA) `srand(old)`. gr sums `(V1 + V2 + V3)` component-wise first (different addition order -> different seed
  for some inputs).
- Verdict: CONFIRMED (seed rounding order). Class: keep-original.

Rows :196, :200, :201 (`GetRandomNumberInRange(0.f, 1.f)` for cutOff, offset1, offset2):
- Original bytes 0x5A3CB9-0x5A3CC6 (cutOff, only when `density < 1.0f`), 0x5A3CDB-0x5A3CEE (o1 stored f32),
  0x5A3CF2-0x5A3D09 (`o2 = t * o1`, kept in st0): each value is exactly `t`.
- gr: `lerp(0, 1, t) = 1*t + 0*(1-t)` = t exactly, with the same t (RAND_MAX_FLOAT_RECIPROCAL = 1/32767).
- Verdict: REFUTED (identical values for the (0, 1) range). Class: harmless.

Row :203 (FIX_BUGS candidate "result can fall outside of the triangle"):
- Original bytes 0x5A3D0D-0x5A3D85: `pos = (V1 + vec1*o1) + vec2*o2` with vec1 = V2 - V1, vec2 = V3 - V2, o2 = o1*t2
  (barycentric weights (1-o1, o1(1-t2), o1*t2), all in [0,1]: the point is inside the triangle). gr computes the same.
- Verdict: REFUTED (gr matches; the comment's concern is unfounded). Class: harmless.

Row :219 (grid start/end casts):
- Original bytes 0x5A3A27-0x5A3AA3: `inv = 1.0f / spacing`; `xCur = (float)ftol((float)ftol(minX*inv) * spacing)`;
  `xEnd = (float)ftol((float)(ftol(maxX*inv) + 1) * spacing)`; same for y (ftol 0x821B40 = truncation). gr matches.
- Verdict: REFUTED. Class: harmless.

Found in passing (CONFIRMED, keep-original):
- Grid loops (0x5A3AF0-0x5A3C0F): y restarts from yCur for every x column (`fld [esp+0x24]` = start y at 0x5A3AF0).
  gr's `for (yCur; ...)` never resets yCur, so gr only walks the first column - gr logic error.
- Centre (0x5A385D-0x5A38B4): `((V1 + V2) + V3) * 0.3333f [0x867028 = 0.33329999]` (not `/3`); gr divides by 3.
