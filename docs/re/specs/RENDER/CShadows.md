# CShadows (RENDER) - full specs

Entity basics: +0x04 placement pos, +0x14 m_matrix (CMatrix: +0 right, +0x10 up, +0x20 at, +0x30 pos), +0x1C flags (bit 0 m_bUsesCollision,
bit 16 m_bDontCastShadowsOn), +0x22 m_nModelIndex (i16), +0x2C m_nScanCode (u16), +0x2F m_nAreaCode (u8). "Ensure matrix" = if m_matrix == null:
AllocateMatrix 0x54F560; placement.UpdateMatrix(m_matrix) 0x54F1B0. Camera position C = TheCamera.m_matrix (0xB6F03C) ? matrix+0x30 : 0xB6F02C.
CShadows::CastShadowEntityXY 0x7086B0 is not specified in this file yet (see the RE-SPEC-3 R2 report).

## 0x706F60 CShadows::AddPermanentShadow  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (u8 type [esp+4], RwTexture* tex [+8], CVector* posn [+0xC], float frontX [+0x10], float frontY [+0x14],
  float sideX [+0x18], float sideY [+0x1C], int16 intensity [+0x20], u8 r [+0x24], u8 g [+0x28], u8 b [+0x2C], float drawDistance [+0x30],
  u32 time [+0x34], float upDistance [+0x38])`, plain `ret`.
- Callers: Fx_c::AddBlood 0x49EB00 (2), CObject::ObjectFireDamage 0x5A1580, CPed::DoFootLanded 0x5E5380 (2), CPed::ProcessControl 0x5E8CD0,
  CTaskSimpleDead::ProcessPed 0x630600, CTaskSimpleJump::Launch 0x679B80 (2), CExplosion::AddExplosion 0x736A50. Callees: none.
- Globals: `aPermanentShadows` 0xC4AC30, 48 x 0x38 (+0 pos, +0xC frontX, +0x10 frontY, +0x14 sideX, +0x18 sideY, +0x1C gr m_fZDistance,
  +0x20 gr m_fScale, +0x24 m_nTimeCreated, +0x28 m_nTimeDuration, +0x2C texture, +0x30 intensity i16, +0x32 type u8, +0x33/+0x34/+0x35
  r/g/b, +0x36 flag bits); CTimer::m_snTimeInMilliseconds 0xB7CB84.
- Behaviour:
  1. Slot = first i in 0..47 with `type == 0` (loop unrolled x8, 0x706F70-0x706FDF).
  2. If none: choose the slot with the smallest `m_nTimeCreated` (unsigned `<`, start 0xFFFFFFFF, first wins ties) among shadows with
     `frontX^2 + frontY^2 < 0.25f [0x858C84]` AND `sideX^2 + sideY^2 < 0.25f` (strict; each sum is `a*a + b*b` in x87, NaN fails). If no
     shadow qualifies, return without adding (0x7072CC).
  3. Fill (0x7072D2-0x70737B): type, texture, pos (3 dwords), front, side, intensity (u16), r, g, b, `+0x1C = drawDistance`,
     `+0x28 = time`, `+0x20 = upDistance`, `+0x24 = CTimer::m_snTimeInMilliseconds`. The flag byte +0x36 is not written (keeps the
     previous occupant's bits).
- gta-reversed: raw call. Note gr's field names: the original stores drawDistance into gr's `m_fZDistance` (+0x1C) and upDistance into
  `m_fScale` (+0x20); UpdatePermanentShadows reads them back with that meaning. Open questions: none.

## 0x709CF0 ShadowRenderTriangleCB  (catalogue: specified/REA, port level 8)
- Convention: register-only, non-standard: EAX = `CVector* normal`, EBX = `CVector* tri` (3 vertices, object space), EDI =
  `_ProjectionParam* param`; plain `ret`; returns EAX = EBX (the triangle pointer, so never null). Preserves EBX/EDI (only ESI pushed).
  gr wraps it with inline asm (Shadows.cpp:1254).
- Caller: CShadows::CastShadowEntityXYZ 0x70A040. Callees: RwV3dTransformPoints 0x7EDD90 (x2), RenderBuffer::StartStoring 0x708290
  (cdecl (nIdx, nVtx, RwImVertexIndex** outIdx, RwIm3DVertex** outVtx)), RenderBuffer::StopStoring 0x7077A0, `_ftol` 0x821B40.
- _ProjectionParam (0x98): +0 at (CVector), +0xC invMatrix (RwMatrix), +0x4C shadowValue (u8), +0x50 fade (RwBool), +0x54 numIm3DBatch,
  +0x58 entityMatrix (RwMatrix). RwIm3DVertex 0x24: +0 x,y,z, +0x18 colour, +0x1C u, +0x20 v.
- Behaviour:
  1. `W[0..2] = RwV3dTransformPoints(tri, 3, &param->entityMatrix)` (world space).
  2. `d = (at.y*n.y + at.z*n.z) + at.x*n.x` (loads in this order, 0x709D15-0x709D36); if `d > 0` (strict) return (only triangles
     facing against the projection direction, or NaN, continue).
  3. `S[0..2] = RwV3dTransformPoints(W, 3, &param->invMatrix)` (projector space: x,y = uv, z = depth).
  4. Reject (return) if all three `S.z < 0`; if all three `S.x < 0`; if all three `S.x > 1.0f`; if all three `S.y < 0`; if all three
     `S.y > 1.0f` (each test strict, NaN never counts; tested in this order, 0x709D5E-0x709E6E).
  5. `StartStoring(3, 3, &idx, &vtx)`. Vertex k (k = 0..2, base vtx + 0x24*k): pos = `(W[k].x, W[k].y, W[k].z + 0.06f [0x859934])`,
     u = S[k].x, v = S[k].y.
  6. Colour per vertex: if `param->fade != 0`: `t = 1.0f - S[k].z * S[k].z`; `c = (t < 0) ? 0 : (u8)_ftol((float)param->shadowValue * t)`
     (`test ah,5; jp` takes the multiply path for t > 0, t == 0 and NaN); else `c = shadowValue`. Colour dword =
     `c | c<<8 | c<<16 | c<<24`.
  7. `idx[0..2] = 0, 1, 2` (u16); `StopStoring()`; return EBX.
- gta-reversed: asm thunk only. Open questions: none.

## 0x70A040 CShadows::CastShadowEntityXYZ  (catalogue: specified/REA, port level 9)
- Convention: `__cdecl void (CEntity* e [esp+4], CVector* posn [+8], float frontX [+0xC], float frontY [+0x10], float sideX [+0x14],
  float sideY [+0x18], int16 intensity [+0x1C], u8 r [+0x20], u8 g [+0x24], u8 b [+0x28], float zDistance [+0x2C], float scale [+0x30],
  CPolyBunch** pp [+0x34], CRealTimeShadow* rt [+0x38])`, plain `ret`. The only caller passes a 15th arg (pDayNightIntensity) which is
  ignored. posn, sideX/sideY, r/g/b, zDistance, scale, pp are NOT read either (only e, frontX, frontY, intensity, rt).
- Caller: CastRealTimeShadowSectorList 0x70A7E0. Callees: CRealTimeShadow::GetShadowCamera 0x7059E0 (thiscall),
  CCollision::CalculateTrianglePlanes(CColModel*) 0x418580, RwMatrixRotate 0x7F1FD0, CPlaceable::AllocateMatrix 0x54F560 +
  CSimpleTransform::UpdateMatrix 0x54F1B0 (ensure-matrix, x2), CMatrix::CopyToRwMatrix 0x59B8B0 (x2), RwMatrixInvert 0x7F2070 (x2),
  RwMatrixScale 0x7F22C0, RwMatrixTranslate 0x7F2450, RwV3dTransformPoints 0x7EDD90, CColSphere::Set 0x40FD10,
  CCollisionData::GetTrianglePoint 0x40F5E0 (x3), CCollision::TestSphereTriangle 0x4165B0, ShadowRenderTriangleCB 0x709CF0.
- Behaviour:
  1. If `rt == null` return. `cam = rt->GetShadowCamera()`. `cm = ms_modelInfoPtrs[e->m_nModelIndex]->m_pColModel` (+0x14);
     `cd = cm->m_pColData` (+0x2C); if null return. `CalculateTrianglePlanes(cm)`.
  2. Projection param P (local, 0x98 bytes; numIm3DBatch left uninitialised): `M = RwCameraGetFrame(cam->m_pRwCamera)->modelling`
     (16-dword copy); `RwMatrixRotate(&M, &(1,0,0), -45.0f, 1 PRECONCAT)`; `P.at = M.at`.
     `P.entityMatrix = e->GetMatrix()` (ensure + CopyToRwMatrix). `RwMatrixInvert(&P.invMatrix, &M)`.
     `vw = cam->m_pRwCamera->+0x68` (viewWindow.x); `s.x = s.y = -0.5f [0x858F40] / (0.9f [0x858C20] * vw)`, `s.z = 1.0f / (vw * 0.8f
     [0x858C98])`; `RwMatrixScale(&P.invMatrix, &s, 2 POSTCONCAT)`; `RwMatrixTranslate(&P.invMatrix, &(0.5f, 0, 0), 2 POSTCONCAT)`.
     `P.fade = 0`, `P.shadowValue = (u8)intensity`. (A dead store of 2.0f at a local just below the sphere, 0x70A2B5.)
  3. Local sphere: `E = e->GetMatrix()` copied (ensure + CopyToRwMatrix), `Ei = RwMatrixInvert(E)`;
     `c = rt->m_baseSphere.center` (rt+0x3C); `c.x = frontX * -1.1f [0x871654] + c.x`, `c.y = c.y + frontY * -1.1f`, `c.z = c.z - 0.5f`;
     `L = RwV3dTransformPoints(&c, 1, &Ei)`; `sphere.Set(2.0f, L, 0, 0, 0xFF)`.
  4. For i = 0 .. cd->m_nNumTriangles - 1 (i16 at +4, re-read each iteration; skip all if <= 0): tri = cd->m_pTriangles + 8*i
     (u16 vA +0, vB +2, vC +4); `T0..T2 = GetTrianglePoint(vA/vB/vC)`; if `TestSphereTriangle(sphere, cd->m_pVertices, tri,
     cd->m_pTrianglePlanes + 10*i)`: `N = (i16 plane+0, +2, +4) * 0.000244140625f [0x858BFC]` (stored f32); `o = N * 0.028f [0x872720]`
     (o.z stored f32, o.x/o.y in x87); `Tk += o` for k = 0, 1, 2 (each component stored f32); `ShadowRenderTriangleCB(EAX = &N, EBX = T,
     EDI = &P)`; a null return would stop the loop (never happens).
- gta-reversed: raw function-pointer call (14-arg signature). Open questions: none.

## 0x70A630 CShadows::CastShadowSectorList<CPtrListSingleLink<CPhysical*>>  (catalogue: specified/REA, port level 7)
- Convention: `__cdecl void (CPtrListSingleLink* list [esp+4], float aX [+8], float aY [+0xC], float bX [+0x10], float bY [+0x14],
  CVector* posn [+0x18], float frontX, frontY, sideX, sideY [+0x1C..+0x28], int16 intensity [+0x2C], u8 r, g, b [+0x30..+0x38],
  float zDistance [+0x3C], float scale [+0x40], CPolyBunch** pp [+0x44], u8* pDayNight [+0x48], int shadowType [+0x4C])`, plain `ret`.
  The list arg slot is reused to hold the next node.
- Caller: CShadows::RenderStoredShadows 0x70A960. Callees: AllocateMatrix 0x54F560 +
  UpdateMatrix 0x54F1B0, entity vtable +0x24 (CEntity::GetBoundRect, thiscall with hidden CRect* result), CastShadowEntityXY 0x7086B0.
- Globals: CWorld::ms_nCurrentScanCode 0xB7CD78 (u16), CGame::currArea 0xB72914, CModelInfo::ms_modelInfoPtrs 0xA9B0C8.
- Behaviour: if `list->head == null` return. For each node (node+0 = entity, node+4 = next, read before processing):
  1. Skip if `e->m_nScanCode (+0x2C) == ms_nCurrentScanCode`; else set it.
  2. Skip unless `e->flags (+0x1C) & 1` (m_bUsesCollision).
  3. Skip unless `(u8)e->m_nAreaCode (+0x2F) == currArea` or `== 13`.
  4. Ensure matrix; skip unless `matrix.at.z (+0x28) > 0.97f [0x872724]` (strict).
  5. `rc = e->GetBoundRect()` (CRect: +0 left, +4 top, +8 right, +0xC bottom); require `aX < rc.right`, `bX > rc.left`, `aY < rc.top`,
     `bY > rc.bottom` (all strict; NaN fails), tested in this order.
  6. `cb = colModel bounding box` (`ms_modelInfoPtrs[modelIndex]->m_pColModel`); `p = e position` (matrix pos or placement pos);
     require `cb.max.z (+0x14) + p.z > posn->z - zDistance` and `cb.min.z (+8) + p.z < posn->z` (strict).
  7. `CastShadowEntityXY(e, aX, aY, bX, bY, posn, frontX, frontY, sideX, sideY, intensity, r, g, b, zDistance, scale, pp, pDayNight,
     shadowType)`.
- Note: unlike CastRealTimeShadowSectorList this variant does not test m_bDontCastShadowsOn (bit 16) but does test the 0.97 uprightness.
- gta-reversed: raw call. Open questions: none.

## 0x70A7E0 CShadows::CastRealTimeShadowSectorList<CPtrListSingleLink<CPhysical*>>  (catalogue: specified/REA, port level 10)
- Convention: as 0x70A630 but the last three args are `CPolyBunch** pp [+0x44]`, `CRealTimeShadow* rt [+0x48]`, `u8* pDayNight [+0x4C]`.
- Caller: CShadows::RenderStoredShadows 0x70A960. Callee: CastShadowEntityXYZ 0x70A040.
- Behaviour: same loop and tests as 0x70A630 steps 1-3, 5, 6, with these differences: after the collision flag it also skips
  entities with `byte (+0x1E) & 1` (flags bit 16, m_bDontCastShadowsOn, 0x70A837); there is NO ensure-matrix and NO at.z test (the z tests
  still read `matrix ? matrix->pos : placement pos`). Call: `CastShadowEntityXYZ(e, posn, frontX, frontY, sideX, sideY, intensity, r, g, b,
  zDistance, scale, pp, rt, pDayNight)` (15 args, `add esp,0x3C`; corner args not passed).
- gta-reversed: raw call. Open questions: none.

## 0x70B730 CShadows::GeneratePolysForStaticShadow  (catalogue: specified/REA, port level 8)
- Convention: `__cdecl void (int16 index [esp+4])`, plain `ret`. The arg slot is reused as a float/int temporary.
- Caller: CShadows::StoreStaticShadow 0x70BA00. Callees: `floor` 0x8219F0 (CRT, double arg on the stack, result in ST0) and `_ftol`
  0x821B40 (x8), CWorld::ClearScanCodes 0x563470, CastPlayerShadowSectorList 0x70A470.
- Globals: aStaticShadows 0xC4A030 (0x40 each: +0 id, +4 m_pPolyBunch, +8 created, +0xC pos, +0x18 frontX, +0x1C frontY, +0x20 sideX,
  +0x24 sideY, +0x28 zDistance, +0x2C scale, +0x30 texture, +0x34 intensity, +0x36 type, +0x37..+0x39 r,g,b, +0x3A justCreated,
  +0x3B rendered, +0x3C temporary, +0x3D dayNightIntensity), CWorld::ms_nCurrentScanCode 0xB7CD78 (u16), CWorld::ms_aSectors 0xB7D0B8
  (120x120, 8 bytes each; +0 = buildings list).
- Behaviour (s = aStaticShadows[(i16)index]):
  1. `ex = |s.sideX| + |s.frontX|`, `ey = |s.sideY| + |s.frontY|` (x87); `minX = p.x - ex`, `maxX = p.x + ex`, `minY = p.y - ey`,
     `maxY = p.y + ey` (stored f32; p = local copy of s.pos).
  2. Sector range, each `v = floor((double)(c * 0.02f [0x858B38] + 60.0f [0x858B34]))` then `_ftol`:
     `x0 = (v(minX) > 0) ? v(minX) : 0`, `y0 = (v(minY) > 0) ? v(minY) : 0`, `x1 = (v(maxX) < 119) ? v(maxX) : 119`,
     `y1 = (v(maxY) < 119) ? v(maxY) : 119` (the value is floored twice from the stored f32; 119 = 0x77).
  3. Scan code: if `ms_nCurrentScanCode < 0xFFFF` increment it, else `ClearScanCodes(); ms_nCurrentScanCode = 1`.
  4. For y = y0 .. y1, for x = x0 .. x1 (inclusive, signed): sector = `ms_aSectors[clamp(y,0,119)*120 + clamp(x,0,119)]` (clamp via
     `x <= 0 ? 0 : x` then `>= 119 ? 119`); `CastPlayerShadowSectorList(&sector.buildings, minX, minY, maxX, maxY, &p, s.frontX,
     s.frontY, s.sideX, s.sideY, 0, 0, 0, 0, s.zDistance, s.scale, &s.m_pPolyBunch, &s.dayNightIntensity, 0)` (19 args).
- gta-reversed: raw call. Open questions: none.

## 0x70BA00 CShadows::StoreStaticShadow  (catalogue: specified/REA, port level 9)
- Convention: `__cdecl bool (u32 id [esp+4], u8 type [+8], RwTexture* tex [+0xC], CVector* posn [+0x10], float frontX [+0x14],
  float frontY [+0x18], float sideX [+0x1C], float sideY [+0x20], int16 intensity [+0x24], u8 r [+0x28], u8 g [+0x2C], u8 b [+0x30],
  float zDistance [+0x34], float scale [+0x38], float drawDistance [+0x3C], bool temporary [+0x40], float upDistance [+0x44])`,
  plain `ret`, AL result. The posn / intensity / colour / drawDistance arg slots are overwritten as temporaries.
- Entry: `0x70BA00: jmp 0x401EFA`, a .text detour that computes EAX = `*(u32*)([0x15606FC] + [0x65BD2D] + 1)` = `*(CMatrix**)0xB6F03C`
  (TheCamera.m_matrix; 0x265D3252 + 0xDA59BDE9 + 1 wraps to 0xB6F03C) with flags preserved, then `jmp 0x70BA05`. Port: `eax = TheCamera.m_matrix`.
- Callers: CPickups::DoCollectableEffects 0x455E20, CPickups::DoMineEffects 0x4560E0, CTrafficLights::DisplayActualLight 0x49DAB0,
  CFireManager::Update 0x53AF00, CTaskSimpleDead::ProcessPed 0x630600, CEntity::ProcessLightsForEntity 0x6FC7A0 (2),
  StoreShadowForVehicle 0x70BDA0 (2), StoreCarLightShadow 0x70C500, StoreShadowForPole 0x70C750, UpdatePermanentShadows 0x70C950.
  Callees: CStaticShadow::Free 0x707670 (thiscall), GeneratePolysForStaticShadow 0x70B730, `_ftol` 0x821B40.
- Behaviour (C = camera position, matrix pos or placement pos 0xB6F02C):
  1. `d2 = (p.y - C.y)^2 + (p.x - C.x)^2` (y term first; stored f32). If `!(drawDistance^2 > d2)`: if `drawDistance != 0` (NaN counts
     as != 0) return 1 (too far, reported as stored). If `drawDistance^2 > d2` and `drawDistance != 0`: `d = sqrt(d2)`, `h = drawDistance *
     0.75f [0x858F34]`; if `!(d < h)`: `f = 1.0f - (d - h) * (4.0f [0x858B90] / drawDistance)`; `intensity = _ftol((int)(i16)intensity * f)`,
     `r = (u8)_ftol(r * f)`, `g = ...`, `b = ...` (in this order). (drawDistance == 0 skips the test and the fade.)
  2. Existing entry: first i in 0..47 with `s[i].id == id && s[i].m_pPolyBunch != null`:
     - `same = |p.x - s.pos.x| < upDistance && |p.y - s.pos.y| < upDistance`, or else `|p.x - s.pos.x| < 0.05f [0x858C28] &&
       |p.y - s.pos.y| < 0.05f && |p.z - s.pos.z| < 2.0f [0x858CA0] && frontX, frontY, sideX, sideY all exactly equal (`test ah,0x44`)`.
     - same: refresh only texture, type, intensity, r, g, b, scale, zDistance, `created = now` (CTimer::m_snTimeInMilliseconds 0xB7CB84),
       `justCreated = 1`, `temporary = arg`; position and front/side are NOT updated; return 1.
     - not same: `s[i].Free()` and fill slot i (step 4).
  3. New entry: first i with `m_pPolyBunch == null`; if none return 1 (0x70BB6B).
  4. Fill: type, texture, intensity, r, g, b, zDistance, scale, id, pos = *posn, front, side, `justCreated = 1`, `temporary`,
     `created = now`; `GeneratePolysForStaticShadow(i)`; return `m_pPolyBunch != null`. (rendered and dayNightIntensity are not written.)
- gta-reversed: raw call. Open questions: none.

## 0x70BDA0 CShadows::StoreShadowForVehicle  (catalogue: specified/REA, port level 10)
- Convention: `__cdecl void (CVehicle* v [esp+4], int vehShadowType [esp+8])`, plain `ret`.
- Callers: CAutomobile::PreRender 0x6AAB50, CBike::PreRender 0x6BD090, CBmx::PreRender 0x6C0810, CHeli::PreRender 0x6C5420,
  CPlane::PreRender 0x6C94A0. Callees: CStencilShadows::GraphicsHighQuality 0x70F9B0, CEntity::GetColModel 0x535300,
  CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (-1, 0)), CVector::CrossProduct(out, a, b) 0x59C730, StoreStaticShadow 0x70BA00,
  StoreShadowToBeRendered (texture overload) 0x707390, `_ftol` 0x821B40.
- Globals: gpShadowCarTex 0xC403E0, gpShadowHeliTex 0xC403E8, gpShadowBikeTex 0xC403EC, gpShadowBaronTex 0xC403F0; shadow strength
  dword 0xB7C4E8 (low word = CTimeCycle m_nShadowStrength); TheCamera matrix 0xB6F03C / placement 0xB6F02C; f32 at 0xB6F118
  (TheCamera + 0xF0, LOD distance multiplier); CCutsceneMgr::ms_running 0xB5F851; CTimer::ms_fTimeStep 0xB7CB5C.
- Behaviour (M = v->m_matrix, assumed non-null after step 2; P = local copy of v's position):
  1. If `GraphicsHighQuality()` return. `tex = gpShadowCarTex`; `s = dword[0xB7C4E8]`; if `(u16)s == 0` return. `dd = 4.5f`.
  2. `dx = P.x - C.x`, `dy = P.y - C.y`; `d2 = dx*dx + dy*dy` (stored f32). If cutscene running: `d2 = d2 / ((L * L) * 4.0f)` (L = f32 0xB6F118).
  3. By `vehShadowType`: 2, 3, 4 -> `max = 144.0f`, limit 20736.0f; 5 -> `max = 288.0f`, limit 82944.0f; other -> `max = 18.0f`,
     limit 324.0f. Return unless `d2 < limit` (strict).
  4. `d = sqrt(d2)`, `h = max * 0.75f`; if `!(d < h)`: `s = _ftol((1.0f - (d - h) / (max * 0.25f)) * (int)(i16)s)`.
  5. Size: `bb = GetColModel()->bbox`; `sy = bb.max.y - bb.min.y`, `sx = bb.max.x - bb.min.x` (stored f32); `k = 1.0f`. Per model (i16):
     - A (417, 425, 447, 469, 487, 497): `k = 0.5f`, `sx *= 3.0f`, `sy *= 1.4f`.
     - B (465, 501): `k = 0.2f`, `sy *= 1.5f`, `sx = sx + sx`.
     - C (448, 461, 462, 481, 509, 510, 521, 522): `k = 0.05f`, `sy *= 1.2f`.
     - D (463, 468, 523): `k = 0.03f`, `sy *= 1.5f`.
     - 539: return (no shadow). -2: `sy *= 0.9f`, `sx *= 0.4f` (k unchanged). Others: unchanged.
     (Tables: byte map 0x70C480 for 417..469 into 0x70C46C, byte map 0x70C4C8 for 487..510 into 0x70C4B8.)
  6. `o = ((sy * 0.5f) - bb.max.y) * k`; `P.x = P.x - o * M.up.x`, `P.y = P.y - o * M.up.y` (stored f32).
  7. By vehShadowType (table 0x70C4E0, values > 5 keep the car texture): 0 car; 1 bike: `l = |f32 v+0x648| * 5.092958f [0x8722D8] + 1.0f`;
     if `(v byte +0x36 & 0xF8) == 0x20` (status 4) and `|M.right.z| > 0.6f`: `sx = (|M.right.z| * 4.0f + l) * sx`, else `sx = l * sx`;
     tex = bike. 2 heli: tex = heli; if `v == FindPlayerVehicle(-1, 0)`: `dd = 50.0f`. 3 and 5: tex = baron, `s = dword[0xB7C4E8]` again
     (full strength, discards the step-4 fade); player vehicle -> `dd = 50.0f`. 4: tex = baron, `sy *= 1.5f`, `sx *= 2.2f [0x87272C]`.
  8. `fr = (M.up.x, M.up.y)`; `c = CrossProduct(M.up, (0,0,1))`; `side = (c.x, c.y)`; `len = sqrt(c.y*c.y + c.x*c.x)`; if `len < 0.5f`:
     `side = (c.x * (1/len) * 0.5f, (1/len) * c.y * 0.5f)`. If `M.at.z < 0` (strict): `side = -side`.
     Type 1: if `|M.right.z| > 0.6f [0x858CC8]`: `side = (M.at.x, M.at.y)`. Type 2: if `|M.right.z| > 0.57f [0x872728]`: `side = (M.at.x, M.at.y)`;
     if `|M.up.z| > 0.57f`: `fr = (M.at.x, M.at.y)`.
  9. `flag = (v byte +0x42C & 0x80) ? 1 : (v == FindPlayerVehicle(-1, 0))`.
     `a = sx * 0.5f`, `b = sy * 0.5f`; args `frontX = b * fr.x`, `frontY = b * fr.y`, `sideX = a * side.x`, `sideY = a * side.y`, and when
     `!(M.at.z > 0)` (at.z <= 0 or NaN) sideX/sideY are negated again (so at.z < 0 ends un-negated, at.z == 0 negated).
  10. If `|v->m_vecMoveSpeed| * ms_fTimeStep > 0.1f` (speed = `sqrt((x*x + y*y) + z*z)`) or flag: `StoreShadowToBeRendered(1, tex, &P,
      frontX, frontY, sideX, sideY, s, s, s, s, dd, flag (as drawOnWater), 1.0f, nullptr, flag)`.
      Else: `StoreStaticShadow((u32)v + 1, 1, tex, &P, frontX, frontY, sideX, sideY, s, s, s, s, 4.5f, 1.0f, 0.0f, false, 0.1f)`.
- gta-reversed: raw call. Open questions: names of v+0x648 (bike lean value) and v+0x42C bit 7 (forces the dynamic path) not resolved
  from gr headers in this pass (0x70C075, 0x70C26C).

## 0x70C950 CShadows::UpdatePermanentShadows  (catalogue: specified/REA, port level 10)
- Convention: `__cdecl void ()`, plain `ret`. Caller: CGame::Process 0x53BEE0. Callees: CShadows::StoreStaticShadow 0x70BA00 (cdecl,
  17 args), CFireManager::StartFire (pos overload, thiscall on gFireManager 0xB71F80) 0x539F00, `_ftol` 0x821B40.
- Globals: aPermanentShadows 0xC4AC30 (layout in AddPermanentShadow), CTimer::m_snTimeInMilliseconds 0xB7CB84, CTimer::m_FrameCounter 0xB7CB4C.
- Behaviour:
  1. For each shadow i (0..47) with `type != 0`: `e = now - created` (u32); `D = duration`.
     - If `e >= D` (unsigned): `type = 0`, next.
     - `q = (3*D) >> 2` (32-bit `lea`, may wrap). If `e < q`: colour/intensity unchanged (intensity passed zero-extended).
       Else `f = 1.0f - (float)(u32)(e - q) / (float)(u32)(D >> 2)` (u32->float via fild + 4294967296.0f [0x858C54] when negative);
       `b' = _ftol(b * f)`, `g' = _ftol(g * f)`, `r' = _ftol(r * f)`, `int' = _ftol((i16)intensity * f)` (computed in this order).
     - `ok = StoreStaticShadow(id = (u32)&shadow (its address), type, texture, &pos, frontX, frontY, sideX, sideY, int, r, g, b,
       zDistance = +0x1C, scale = 1.0f, drawDistance = 40.0f, temporary = false, upDistance = 0.0f)`.
     - If `!ok && type != 8`: `type = 0`.
  2. If `(m_FrameCounter & 3) == 0` (0x70CA90): for each shadow i (0..47) with `type == 5`:
     a. `type_i = 6`, `created_i = now`, `duration_i = 2000`.
     b. Nearest neighbour: best = 3.0f; for each k (0..47) with type 4 or 8: `d = sqrt((dz*dz + dy*dy) + dx*dx)` with `d? = pos_i - pos_k`;
        if `d < best` (strict): best = d, K = k. If found: `type_K = 7`; `StartFire(pos_K, 1.8f, 0, nullptr, 2000, 0, 1)`.
        Then `u = (pos_K.x - pos_i.x, pos_K.y - pos_i.y)`, best = 3.0f [0x858B3C]; for each m with type 4 or 8: `d` as above;
        if `d < best` and `(pos_m.y - pos_i.y) * u.y + (pos_m.x - pos_i.x) * u.x < 0` (strict): best = d, M = m. If found: `type_M = 7`,
        `StartFire(pos_M, 1.8f, 0, nullptr, 2000, 0, 1)` (fires spread in two opposite directions).
  3. Then (still inside the every-4th-frame block) every shadow with `type == 7` becomes `type = 5`.
- gta-reversed: raw call. Open questions: none.


## 0x7086B0 CShadows::CastShadowEntityXY  (catalogue: specified/REA, port level 6)
(RE-SPEC-FINAL; supersedes the "not specified yet" note in this file's header.)
- Convention: `__cdecl void (CEntity* e [esp+4], float aX [+8], float aY [+0xC], float bX [+0x10], float bY [+0x14], CVector* posn [+0x18],
  float frontX [+0x1C], float frontY [+0x20], float sideX [+0x24], float sideY [+0x28], int16 intensity [+0x2C], u8 r [+0x30], u8 g [+0x34],
  u8 b [+0x38], float zDistance [+0x3C], float scale [+0x40], CPolyBunch** pp [+0x44], u8* pDayNight [+0x48], int shadowType [+0x4C])`,
  plain `ret`. Frame: `sub esp,0x74` + 4 pushes, so inside the body arg k (1-based) is at `[esp + 0x84 + 4k]` (e = [esp+0x88], posn = [esp+0x9C],
  frontX [0xA0], frontY [0xA4], sideX [0xA8], sideY [0xAC], intensity [0xB0], r/g/b [0xB4/0xB8/0xBC], zDistance [0xC0], scale [0xC4],
  pp [0xC8], pDayNight [0xCC], shadowType [0xD0]). **The four corner args aX/aY/bX/bY are never read** (no access to [esp+0x8C..0x98]).
- Callers: CastPlayerShadowSectorList 0x70A470, CastShadowSectorList 0x70A630. Callees: CCollision::CalculateTrianglePlanes(CColModel*)
  0x418580, CPlaceable::AllocateMatrix 0x54F560 + CSimpleTransform::UpdateMatrix 0x54F1B0 ("ensure matrix", 16 sites; only the first can
  allocate), CCollisionData::GetTrianglePoint 0x40F5E0 (thiscall on colData: `(CVector* out, int vertexIndex)`, x3), RwIm3DTransform 0x7EF450,
  RwIm3DRenderIndexedPrimitive 0x7EF550, RwIm3DEnd 0x7EF520, CShadows::AffectColourWithLighting 0x707850, RenderBuffer::StopStoring
  0x7077A0, `_ftol` 0x821B40.
- Globals (all .data, scratch, zero-initialised): guard dword 0xC4B8C8; quad corners `Q[4]` = CVector array 0xC4B898 (stride 0xC; only x,y
  written: Q0 0xC4B898/0xC4B89C, Q1 0xC4B8A4/0xC4B8A8, Q2 0xC4B8B0/0xC4B8B4, Q3 0xC4B8BC/0xC4B8C0); clip buffer A: positions 0xC4B7A8
  (CVector[], stride 0xC), uv 0xC4B6B8 (stride 0xC, u at +0, v at +4); clip buffer B: positions 0xC4B820, uv 0xC4B730 (same layout; B.z at
  0xC4B828 + 0xC*k). Render buffer: uiTempBufferVerticesStored 0xC4B950, uiTempBufferIndicesStored 0xC4B954, aTempBufferIndices 0xC4B958
  (u16), TempVertexBuffer 0xC4D958 (RwIm3DVertex 0x24: +0 x, +4 y, +8 z, +0x18 colour dword, +0x1C u, +0x20 v), s_IndicesToBeStored
  0xC40410, s_VerticesToBeStored 0xC40414, g_ShadowVertices 0xC403A8 (u16 fan index table), CShadows::pEmptyBunchList 0xC403D8,
  CModelInfo::ms_modelInfoPtrs 0xA9B0C8.
- Data layouts: CColModel* = modelInfo+0x14; colData = colModel+0x2C. CCollisionData: +4 numTriangles (i16, read with movsx), +7 flags (bit 1 =
  bHasFaceGroups), +0x18 m_pTriangles (CColTriangle 8 bytes: u16 vA/vB/vC at +0/+2/+4, +7 light byte), +0x1C m_pTrianglePlanes (10 bytes:
  i16 normal x/y/z at +0/+2/+4). Face groups (only when flag bit 1): count = dword at `m_pTriangles - 4`; group k (k = 0..count-1) is at
  `m_pTriangles - 4 - 0x1C*(k+1)` (so k = 0 is the group nearest the count dword; 0x708B81-0x708B86): +0 min (x,y,z), +0xC max (x,y,z),
  +0x18 first (i16), +0x1A last (i16, inclusive). CPolyBunch: +0 CVector pos[7], +0x54 next, +0x58 numVerts (i16), +0x5A u8 u[7], +0x61 u8 v[7].
  Entity matrix M = e->m_matrix (+0x14): right +0, up +0x10, pos +0x30 (placement pos e+4 when null; never null after the first ensure).
- Behaviour:
  1. Static-local guard (0x7086B0-0x7086D7): `if !(g & 1) g |= 1; if !(g & 2) g |= 2; if !(g & 4) g |= 4` on [0xC4B8C8] (trivial ctors,
     nothing else runs).
  2. `cm = ms_modelInfoPtrs[(i16)e->m_nModelIndex]->m_pColModel`; `cd = cm->m_pColData`; return if `cd == null` or `cd->numTriangles == 0`
     (u16 compare, 0x708702). `CalculateTrianglePlanes(cm)`. Ensure matrix.
  3. Project the shadow box into entity-local XY (each value stored f32; operand order as loaded, all `a*b + c*d` sums are commutative):
     `fR = frontY*M.right.y + frontX*M.right.x` [esp+0x30] (0x708751-0x708768); `fU = frontX*M.up.x + frontY*M.up.y` [esp+0x2C];
     `sR = sideY*M.right.y + sideX*M.right.x` [esp+0x24]; `sU = sideX*M.up.x + sideY*M.up.y` [esp+0x20]; `dx = posn.x - M.pos.x`,
     `dy = posn.y - M.pos.y` (stored); `cR = dy*M.right.y + dx*M.right.x` (stored); `cU = dy*M.up.y + dx*M.up.x` (x87 only, 0x708903-0x708911).
     Corners (0x708913-0x708991): `Q0 = ((cR + fR) - sR, (fU + cU) - sU)`, `Q1 = ((cR + sR) + fR, (sU + cU) + fU)`,
     `Q2 = (sR + (cR - fR), ((cU - fU) + sU))`, `Q3 = ((cR - fR) - sR, (cU - fU) - sU)` (the `cR - fR` and `cU - fU` partials are computed
     once; `cU - fU` is rounded to f32 through a store at 0x708971, `cR - fR` stays in x87).
     `dz = posn.z - M.pos.z` [esp+0x2C] (0x7089A6).
  4. Local bounding rectangle (0x7089B0-0x708B3C), with `lt(a,b)` = strict `a < b` (NaN false):
     `minX = lt(m01, m23) ? m01 : m23` where `m23 = lt(Q2.x, Q3.x) ? Q2.x : Q3.x` and `m01 = lt(Q0.x, Q1.x) ? Q0.x : Q1.x`;
     `maxX` the same with strict `>`; `minY`, `maxY` the same on the y components. (Ties take the second operand.)
  5. Groups: `nGroups = (cd->flags & 2) ? *(i32*)(m_pTriangles - 4) : 1`; if `nGroups <= 0` return. For g = 0 .. nGroups-1:
     - with face groups: skip the group unless `maxX > grp.min.x` && `minX < grp.max.x` && `maxY > grp.min.y` && `minY < grp.max.y`
       (strict, tested in this order 0x708B89-0x708BC6; no z test); range `i = grp.first .. grp.last` (inclusive; both movsx);
     - without: range `i = 0 .. (i16)cd->numTriangles - 1` (re-read each group).
  6. Per triangle i (counter at [esp+0x34], colData pointer re-read from [esp+0x28]):
     a. `nz = (float)plane[i].normal.z * 0.000244140625f [0x858BFC]`; skip unless `|nz| > 0.1f [0x858B1C]` (strict, 0x708C11-0x708C24).
     b. `V0, V1, V2 = GetTrianglePoint(tri.vA / vB / vC)` (model space) into [esp+0x54], [esp+0x60], [esp+0x6C].
     c. Overlap rejects, each "skip unless any of V0, V1, V2 (in this order, short-circuit) satisfies": `V.x > minX`; `V.x < maxX`;
        `V.y > minY`; `V.y < maxY`; `V.z < dz`; then `zLow = dz - zDistance` (stored f32 [esp+0x30], 0x708D64) and `V.z > zLow`
        (all strict; 0x708C6F-0x708D9E).
     d. Clip the quad against the triangle (Sutherland-Hodgman, three passes). Init (0x708DA4-0x708E61): `A[0..3].xy = Q0..Q3` (z not
        written), `Auv[0..3] = (0,0), (1,0), (1,1), (0,1)` (stores 0 / 0x3F800000). For a pass with edge P->R and input X (n points) into
        output Y: `ex = R.x - P.x`, `ey = R.y - P.y` (x87); `side(p) = (p.x - P.x)*ey - (p.y - P.y)*ex` (product order exactly so; the value
        is stored f32 to [esp+0x14] before the compare). State s = 0, count m = 0; for j = 0 .. n-1 with `d = side(X[j])`:
        - `d > 0.0f [0x858B50]` (inside): if s == 2 emit `I(X[j-1], X[j])`; then emit `X[j]` (copy xy and uv); s = 1.
        - else (d <= 0 or NaN): if s == 1 emit `I(X[j-1], X[j])`; s = 2.
        Closing edge: `d0 = side(X[0])` (stored); emit `I(X[n-1], X[0])` if `(d0 > 0 && s == 2) || (d0 <= 0 && d0 not NaN && s == 1)`
        (0x709034-0x709058: the second test uses `test ah,0x41; jp`, so NaN does not emit here, unlike in the loop).
        `I(p, c)` with `dp = side(p)` recomputed and `d` the current point's stored value: `t = |dp| / (|d| + |dp|)` (fabs of both; the
        division is `|dp|` over the sum), `s1 = 1.0f [0x858624] - t`; output `uv = (s1*p.u + t*c.u, s1*p.v + t*c.v)`, `xy = (t*c.x +
        s1*p.x, t*c.y + s1*p.y)` (each pair summed once, commutative; values stored f32 per component). Only x, y, u, v are written; z
        stays whatever the buffer held.
        Passes: 1) edge V0->V1, A (n = 4; closing uses A[3] explicitly, 0x70905E) -> B; 2) edge V1->V2, B (n1) -> A; 3) edge V2->V0,
        A (n2) -> B (n3). If a pass's input count is 0 its loop is skipped and its closing test runs with s = 0, so it never emits
        (the x87 still holds the previous pass's ex/ey; harmless). So only points with side > 0 for all three edges survive, i.e. the
        triangle must have the winding for which its interior is on the positive side of `side()` for V0->V1->V2; triangles of the other
        winding produce nothing.
     e. If `n3 < 3` skip the triangle (0x7096A0).
     f. Height from the triangle plane (0x7096A9-0x7097DD): `nx = (float)plane.x * 2^-12`, `ny = (float)plane.y * 2^-12` (stored f32),
        `nz` likewise (x87); `D = (V0.z*nz + ny*V0.y) + nx*V0.x` (stored f32 [esp+0x14]); `r = -1.0f [0x858C1C] / nz`; for k = 0 .. n3-1:
        `B[k].z = ((ny*B[k].y + nx*B[k].x) - D) * r` (the loop is unrolled by 4; same arithmetic for every k).
     g. To world (0x709800-0x7098D5), per k, ensure matrix each time: `x0 = B[k].x` (saved), `B[k].x = (M.up.x*B[k].y + M.right.x*x0) +
        M.pos.x`; `B[k].y = (M.up.y*B[k].y + x0*M.right.y) + M.pos.y`; `B[k].z = M.pos.z + B[k].z`. (No `at` axis and no rotation of z:
        assumes an upright entity; the callers require at.z > 0.97.)
     h. `*pDayNight = triangle[i].light` (byte +7 of the CColTriangle, 0x7098DB-0x7098F1); written before the paths below.
     i. If `pp != null` (poly-bunch path, 0x709902): `bunch = pEmptyBunchList`; if null skip the triangle; `pEmptyBunchList = bunch->next`;
        `bunch->next = *pp; *pp = bunch` (push front); `bunch->numVerts = (i16)n3`; for k: `pos[k] = B[k]` (3 dwords), `u[k] = (u8)_ftol(Buv[k].u
        * 200.0f [0x858A48])`, `v[k] = (u8)_ftol(Buv[k].v * 200.0f)`. `scale`, intensity and colours are not used on this path.
     j. Else (immediate path, 0x70998C): `nIdx = 3*n3 - 6`. If `indicesStored + nIdx >= 0x1000` flush; then if `verticesStored + n3 >= 0x800`
        flush. Flush = `if (verticesStored != 0 && RwIm3DTransform(TempVertexBuffer, verticesStored, null, 1) != null)
        { RwIm3DRenderIndexedPrimitive(3 /*TRILIST*/, aTempBufferIndices, indicesStored); RwIm3DEnd(); }` then both counters = 0.
        `s_IndicesToBeStored = nIdx; s_VerticesToBeStored = n3`. `AffectColourWithLighting(shadowType, *pDayNight, r, g, b, &o1, &o2, &o3)`
        (locals [esp+0x11..0x13]); `colour = ((((i32)intensity << 8 | o1) << 8 | o2) << 8) | o3` (intensity sign-extended, only its low byte
        survives as the top byte). Vertices at `TempVertexBuffer + 0x24*verticesStored`, k = 0 .. n3-1: `x = B[k].x`, `y = B[k].y`,
        `z = B[k].z + 0.06f [0x859934]`, `colour`, `u = scale * Buv[k].u`, `v = scale * Buv[k].v` (normal fields not written). Then copy
        nIdx u16 from g_ShadowVertices to `aTempBufferIndices + indicesStored` (rep movsd + movsw) and call `RenderBuffer::StopStoring()`.
- Edge cases: the two paths differ in uv scale (200 fixed vs `scale`); a group/triangle range with first > last does nothing; NaN corners make the
  min/max picks take the second operand; `*pDayNight` is overwritten for every surviving triangle (the last one wins).
- gta-reversed: raw call only (Shadows.cpp:649); no reference code. Open questions: none for behaviour. Clip-buffer z lanes are never
  initialised by this function (they are overwritten in step f before use).
