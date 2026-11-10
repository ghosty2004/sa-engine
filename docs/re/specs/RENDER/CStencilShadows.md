# CStencilShadows (RENDER) - full specs

CStencilShadowObject (0x1C, StencilShadowObject.h; meaning of +8/+0xC corrected from the bytes): +0x00 m_pOwner (CEntity*),
+0x04 m_NumShadowFaces (i16), +0x06 m_Type (1 OBJECT, 2 VEHICLE), +0x08 vertex CAPACITY (gr name m_FaceID; set to
`numShadowTriangles * 15` by the creator 0x711280 at 0x7112B8), +0x0C number of shadow-volume vertices written (gr name
m_SizeOfShadowFacesData; reset to 0 and incremented by RenderForVehicle/RenderForObject, read as the vertex count by Render),
+0x10 m_ShadowFacesData (CVector*, malloc'd `capacity * 12` bytes), +0x14 m_pNext, +0x18 m_pPrev.
Globals: pFirstAvailableStencilShadowObject 0xC6A168, pFirstActiveStencilShadowObject 0xC6A16C, s_ShadowTrianglePointsUnk
0xC6A170 (u16 edge pairs), s_ShadowTrianglePoints 0xC6A174 (CVector[]), s_TransformedShadowTrianglePoints 0xC6A178,
s_SunPosNrm 0x8D5244 (CVector 1.0, 1.0, -2.0; f32 data), uiTempBufferVerticesStored 0xC4B950, uiTempBufferIndicesStored 0xC4B954,
aTempBufferIndices 0xC4B958 (u16), TempVertexBuffer 0xC4D958 (RwIm3DVertex, stride 0x24: +0 x, +4 y, +8 z, +0x18 colour).
CCollisionData: +0x07 bit 0x04 bHasShadowInfo, +0x20 m_nNumShadowTriangles, +0x24 m_nNumShadowVertices, +0x2C m_pShadowTriangles
(8-byte entries, u16 a, b, c at +0/+2/+4).

Helper (register-argument) callees used below, not hooked by gr:
- `ToggleEdge` 0x70FA70: ECX = edge array (u16 pairs), EDX = u16* count, DI = vertex a (u16, register), [esp+4] = vertex b
  (u16), plain `ret` (caller pops). Searches i = 0..count-1 for (a,b) or (b,a); if found: when `count > 1` overwrite entry i with
  entry `count-1`, then `count--`; if not found: append (a,b) at index count, `count++` (no bound check).
- `ShadowDist2` 0x711160 (objects) and 0x7111F0 (vehicles), byte-identical: EBX = entity, ESI = CVector* camPos, returns ST0.
  `c = entity->TransformFromObjectSpace(colModel->boundSphere.center (+0x18))` (0x5334F0); `r = colModel->+0x24`;
  `d = sqrt((dz*dz + dy*dy) + dx*dx)` with `dx = cam.x - c.x` etc. (stored f32); `k = (d < r) ? -1.0f [0x858C1C] : 1.0f`;
  returns `k*(d - r)*(d - r)` (x87 order: `k * (d-r)`, then `* (d-r)`).
- `CreateFor` 0x711280: thiscall on a free object (ECX), args (CEntity* e, u8 type), `ret 8`; returns the object or 0.
  Fails unless `1 <= type <= 2`, colData present, `colData->+7 & 4`, `numShadowTriangles > 0`. Sets +4 = (u16)numTri, +8 = numTri*15,
  +0 = e, +6 = type, +0xC = 0, +0x10 = `CMemoryMgr::Malloc(numTri*15*12, 0)`, `e->RegisterReference(&obj->m_pOwner)`,
  `UpdateHierarchy(&pFirstAvailable, &pFirstActive, obj)` 0x710BC0.
- `FindActiveFor` 0x710BA0: cdecl (CEntity*), returns the active object whose owner is the entity, else 0.

## 0x70FAE0 CStencilShadows::RenderForVehicle  (catalogue: specified/REA, port level 5)
- Convention: `__cdecl bool (CStencilShadowObject* obj [esp+4])`, plain `ret`, AL. SEH frame (one CMatrix local).
- Callers: CStencilShadows::Process 0x711D90. Callees: CEntity::GetColModel 0x535300, AllocateMatrix 0x54F560 + UpdateMatrix
  0x54F1B0, CBike::CalculateLeanMatrix 0x6B7150, CMatrix::Invert 0x59B920 (cdecl (src, dst)), CMatrix::operator*(CVector)
  0x59C890 (cdecl (out, mat, vec), full affine transform), CCollisionData::GetShadTrianglePoint 0x40F640 (thiscall (out, i)),
  TransformPoints 0x54EEF0 (cdecl (out, n, mat, in)), ToggleEdge 0x70FA70, ~CMatrix 0x59ACD0.
- Behaviour:
  1. `e = obj->m_pOwner`. `L = 40.0f` if e is a vehicle (`+0x36 & 7 == 2`) with `+0x594` (vehicle type) == 4 or 3, else
     `L = 5.0f` (immediates, 0x70FB20/0x70FB2A).
  2. `cd = e->GetColModel()->+0x2C`; if `cd == 0` or `cd->+0x20 != (i16)obj->+4` return 0 (0x70FB4F).
  3. Ensure `e->m_matrix`; `M = e->m_matrix`; if e is a vehicle with `+0x590 == 9` (bike): `CalculateLeanMatrix()` and
     `M = e + 0x5CC` (0x70FB80-0x70FB96).
  4. `inv = Invert(M)` (local CMatrix, attach fields zeroed). `o = inv * (0,0,0)`, `s = inv * s_SunPosNrm` (both full transforms);
     `dz = s.z - o.z`, `dy = s.y - o.y`, `dx = s.x - o.x` (stored f32, in that order, 0x70FC73-0x70FCC3).
  5. `obj->+0xC = 0`; `edgeCount (u16) = 0`. For `i = 0 .. cd->+0x24 - 1`: `cd->GetShadTrianglePoint(&P[i], i)` (P =
     s_ShadowTrianglePoints). `TransformPoints(T, cd->+0x24, M, P)` (T = s_TransformedShadowTrianglePoints; world space).
  6. For each face `j = 0 .. numFaces-1` (`numFaces = (i16)obj->+4`), `(a, b, c)` = u16 at `cd->+0x2C + 8*j`:
     - Local edges `e1 = P[b] - P[a]`, `e2 = P[c] - P[a]` (each component stored f32); normal
       `n.x = e2.z*e1.y - e2.y*e1.z`, `n.y = e2.x*e1.z - e2.z*e1.x`, `n.z = e2.y*e1.x - e2.x*e1.y` (x87, products as written,
       n.x and n.y stored f32, n.z kept in ST0); `dot = (dz*n.z + n.y*dy) + n.x*dx` (0x70FEB3-0x70FED4).
     - If `dot >= 0` (`fcomp 0; test ah,1; jne` -> `dot < 0` or NaN go to the back branch): `ToggleEdge(a,b)`,
       `ToggleEdge(b,c)`, `ToggleEdge(c,a)`; then if `obj->+0xC + 3 >= obj->+8` stop the face loop (go to step 7);
       else append `T[a], T[c], T[b]` and `+0xC += 3`.
     - Else (back face): `ext = s_SunPosNrm * L` (UNtransformed sun vector, `x*L, y*L, z*L`, stored f32);
       `A = T[a] + ext`, `B = T[b] + ext`, `C = T[c] + ext` (component adds, stored f32); if `+0xC + 3 >= +8` stop the
       face loop; else append `A, C, B` and `+0xC += 3`.
  7. Silhouette (0x7100FF): `ext` recomputed the same way. For `k = 0 .. edgeCount-1`: `(a, b)` = edge k, `Va = T[a]`,
     `Vb = T[b]`, `A = Va + ext`, `B = Vb + ext`; if `+0xC + 6 >= +8` stop; else append `Va, Vb, A, Vb, B, A` and `+0xC += 6`.
  8. `~CMatrix(inv)`; return 1.
- Edge cases: capacity tests are `>=` (the last slot is never used). Edge list / point buffers are not bounds-checked.
  The front cap uses world points T (not extruded); the sun vector used for the facing test is the inverse-transformed
  point difference, but the extrusion uses the world-space s_SunPosNrm.
- gta-reversed: StencilShadows.cpp, hooked `.Reversed = false`, no body. Open questions: none.

## 0x710310 CStencilShadows::RenderForObject  (catalogue: specified/REA, port level 3)
- Convention: `__cdecl bool (CStencilShadowObject* obj)`, plain `ret`, AL; same callees as RenderForVehicle except no
  CBike::CalculateLeanMatrix.
- Behaviour: identical to RenderForVehicle steps 2-8 (same order of operations, same vertex orders `T[a], T[c], T[b]` /
  `A, C, B` / `Va, Vb, A, Vb, B, A`, same `>=` capacity tests), with these differences: no step 1 / bike branch;
  `M = e->m_matrix` after the ensure (0x710373); extrusion length is `60.0f [0x858B34]` (`ext = s_SunPosNrm * 60.0f`, at
  0x710785-0x7107A8 for back faces and 0x7108F6-0x71091E for the silhouette). Failure (no colData / count mismatch) returns
  0 at 0x710354.
- gta-reversed: hooked `.Reversed = false`, no body. Open questions: none.

## 0x710D50 CStencilShadows::Render  (catalogue: specified/REA, port level 7)
- Convention: `__cdecl bool (const CRGBA* color [esp+4])`, plain `ret`, always AL = 1.
- Callers: RenderStencilShadows 0x7113B0 (2 sites). Callees: `FlushIfFull` 0x710CC0 (CStencilShadows::sub_710CC0, cdecl
  (indices, vertices)), RwRenderStateSet via `RwEngineInstance->+0x20`, Common::LittleTest 0x541330, RwIm3DTransform 0x7EF450,
  RwIm3DRenderIndexedPrimitive 0x7EF550, RwIm3DEnd 0x7EF520.
- sub_710CC0 (0x710CC0): if `indices + uiIdx >= 0x1000` or `vertices + uiVtx >= 0x800`, and both counters are non-zero:
  `RwRenderStateSet(1 TEXTURERASTER, 0)`, `LittleTest()`, `if (RwIm3DTransform(TempVertexBuffer, uiVtx, 0, 0x18))
  { RwIm3DRenderIndexedPrimitive(3 TRILIST, aTempBufferIndices, uiIdx); RwIm3DEnd(); }`, then both counters = 0.
- Behaviour:
  1. `col = *color` (dword); `uiIdx = 0; uiVtx = 0`. Packed vertex colour `argb = a<<24 | r<<16 | g<<8 | b` (bytes of the
     CRGBA r,g,b,a; built with shifts at 0x710DBC-0x710DE0 and again at 0x711062-0x711091).
  2. For each object in the active list (`next` read at loop top from +0x14): `n = obj->+0xC`, `V = obj->+0x10`.
     `q = n / 6` (signed division toward zero, 0x710DA0). For `t = 0 .. q-1`: six vertices `V[6t .. 6t+5]`;
     `FlushIfFull(6, 6)`; then (counters re-read) `aTempBufferIndices[uiIdx + k] = uiIdx + k` (k = 0..5; the values come from
     the INDEX counter), `TempVertexBuffer[uiVtx + k].xyz = V[6t+k]`, `.colour (+0x18) = argb` (normal and UV fields are not
     written); `uiIdx += 6; uiVtx += 6`.
  3. If `(n / 3) & 1` (0x710FA4-0x710FB7): one triangle from `V[6q], V[6q+1], V[6q+2]` with `FlushIfFull(3, 3)` and 3
     indices/vertices the same way. Remaining vertices (n % 3) are ignored.
  4. After the list: if `uiIdx != 0` and `uiVtx != 0`: `RwRenderStateSet(1, 0)`, `LittleTest()`, `if (RwIm3DTransform(
     TempVertexBuffer 0xC4D958, uiVtx, NULL, 0x18 /*XYZ|RGBA*/)) { RwIm3DRenderIndexedPrimitive(3, 0xC4B958, uiIdx);
     RwIm3DEnd(); }`; both counters = 0. Return 1.
- gta-reversed: hooked `.Reversed = false`; the stub `plugin::Call<0x710D50>()` passes NO colour argument (gr bug when gr's
  own RenderStencilShadows calls it). The commented gr code iterates `m_SizeOfShadowFacesData / sizeof(CVector)` faces of 6
  (wrong unit; the original uses +0xC as a vertex count with the /6 and /3 split above). Open questions: none.

## 0x711760 CStencilShadows::RegisterStencilShadows  (catalogue: specified/REA, port level 3)
- Convention: `__cdecl bool (CVector* camPos [esp+4], bool doNotCreateNew [esp+8])`, plain `ret`, AL (1 normally, 0 when the
  free list runs empty while creating).
- Callers: CStencilShadows::Process 0x711D90. Callees: CStencilShadowObject::Destroy 0x711310 (thiscall), ShadowDist2 0x711160 /
  0x7111F0, CreateFor 0x711280, FindActiveFor 0x710BA0, CEntity::GetColModel 0x535300, CWorld::ClearScanCodes 0x563470, CRT
  `floor` 0x8219F0 (double), `_ftol` 0x821B40, model info vtable +0x10 (GetModelType).
- Globals: CPools::ms_pVehiclePool 0xB74494, CWorld::ms_nCurrentScanCode 0xB7CD78 (u16), CWorld::ms_aSectors 0xB7D0B8 (8-byte
  sectors, list at +0), CWorld::ms_aRepeatSectors+8 0xB992C0 (12-byte entries, object list), CGame::currArea 0xB72914,
  CModelInfo::ms_modelInfoPtrs 0xA9B0C8. Constants: 2500.0f [0x8598B0], 50.0f [0x858B40], 0.02f [0x858B38], 60.0f [0x858B34].
- "Bad object" test (used in several places, for owners/entities of type 4 = object): `!(e->+0x1C & 0x80)` (not m_bIsVisible)
  or `e->+0x140 & 0x40` (bIsExploded) or `e->+0x140 & 0x400` (bIsBroken).
- Behaviour:
  A. `doNotCreateNew != 0` (0x71176F): walk the active list (next read first); Destroy the object if its owner is null, or
     the owner is an object and "bad". Return 1.
  B. Otherwise:
     1. Walk the active list (0x7117CD-0x711866): Destroy if owner null or (owner object and bad). Else by `obj->+6`:
        type 1: `k = ShadowDist2_obj(owner)`; Destroy if `k > 2500` (`test ah,0x41; je`), else keep only if
        `owner->+0x2F (area) == currArea` or `== 13`, otherwise Destroy. Type 2: `k = ShadowDist2_veh(owner)`; Destroy if
        `k > 2500`. Other types: keep.
     2. If `pFirstAvailable == 0` return 1. Vehicle pool, slots `i = 0 .. size-1` ascending (stride 0xA18, flag byte `< 0` =
        free): skip unless colModel, colData, `colData->+7 & 4`, `veh->+0xB0 != 0` (m_pCollisionList head, i.e. in the world),
        and `FindActiveFor(veh) == 0`; then require `ShadowDist2_veh(veh) <= 2500` (NaN skips); if no free object return 0;
        `CreateFor(free, veh, 2)`; if the free list is now empty return 0.
     3. If `pFirstAvailable == 0` return 1. Sector rectangle (CWorld::GetSector inlined):
        `x0 = max(0, ftol(floor((cam.x - 50)*0.02 + 60)))`, `y0 = max(0, ftol(floor((cam.y - 50)*0.02 + 60)))`,
        `x1 = min(119, ftol(floor((cam.x + 50)*0.02 + 60)))`, `y1 = min(119, ...)` (each `v*0.02+60` is stored f32 and floor is
        called on the double of it; for the clamp the floor+ftol is evaluated twice). Scan code: if `ms_nCurrentScanCode
        >= 0xFFFF` then `ClearScanCodes(); code = 1` else `code++`. If `y0 > y1` return 1.
     4. For `y = y0 .. y1`, for `x = x0 .. x1` (inclusive; outer y, inner x):
        a. List `ms_aSectors[min(max(y,0),119)*120 + min(max(x,0),119)].+0` (buildings), then list
           `ms_aRepeatSectors[(y & 15)*16 + (x & 15)].+8` (objects). For every entity e in each list (next read first):
           skip if `e->+0x1C & 0x4000000` (m_bIsProcObject); skip if e is an object and bad; skip unless colModel, colData,
           `colData->+7 & 4`; skip if `e->+0x2C (scan code) == code`, else set it; skip unless area == currArea or 13;
           skip unless `ms_modelInfoPtrs[(i16)e->+0x22]->GetModelType() == 1`; skip if an active object already owns e
           (inline list walk); skip unless `ShadowDist2_obj(e) <= 2500` (NaN skips); if no free object return 0;
           `CreateFor(free, e, 1)`; if the free list is now empty return 0.
     5. Return 1.
- gta-reversed: StencilShadows.cpp:283 is an empty function (hooked `.Reversed = false`, so the original runs). Open questions:
  none.
