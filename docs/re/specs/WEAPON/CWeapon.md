# CWeapon (WEAPON) - full specs

CWeapon (0x1C): +0 m_Type, +4 m_State, +8 m_AmmoInClip, +0xC m_TotalAmmo, +0x10 m_nTimer. Entity GetPosition() =
`m_matrix ? &m_matrix->pos (+0x30) : &placement.pos (+4)`; "ensure matrix" = `if (!m_matrix) { AllocateMatrix 0x54F560;
placement.UpdateMatrix(m_matrix) 0x54F1B0 }`.

## 0x73D1E0 CWeapon::DoTankDoomAiming  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl void (CEntity* vehicle [esp+4], CEntity* owner [+8], CVector* start [+0xC], CVector* end [+0x10])`,
  plain `ret`. Entry is `nop; jmp 0x40A181` (`sub esp,0xA0; jmp 0x73D1E6`) - a .text detour stub, same behaviour.
  Only `end->z` is written.
- Callers: CAutomobile::TankControl 0x6AE850, CVehicle::FireFixedMachineGuns 0x73DF00 (2 sites).
- Callees: CWorld::FindObjectsInRange 0x564A20 (cdecl, 11 args), CCollision::DistToLine 0x417610 (cdecl
  (const CVector& a, const CVector& b, const CVector& p) -> ST0), CPlaceable::AllocateMatrix 0x54F560 +
  CSimpleTransform::UpdateMatrix 0x54F1B0 (the inlined `GetMatrix()` "allocate if null" pattern, 14 sites).
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8. Constants: 10000.0f (imm 0x461C4000), 0x858B50 0.0f, 0x858B3C 3.0f,
  0x872C30 9000.0f, 0x858C24 0.3f.
- Entity fields: +0x04 placement pos, +0x14 m_matrix (pos at matrix+0x30/+0x34/+0x38), +0x22 m_nModelIndex (i16),
  +0x36 type (bits 0-2) / status (bits 3-7), +0x40 CPhysical flags (bit 29 = 0x20000000 bRenderScorched).
  ModelInfo +0x14 m_pColModel, colmodel +0x24 bounding-sphere radius.
- Behaviour:
  1. `dx = e.x - s.x`, `dy = e.y - s.y`, `dz = e.z - s.z` (s = *start, e = *end; each stored f32).
     Local copies `S2 = (s.x, s.y, 0)`, `E2 = (e.x, e.y, 0)` (z forced to 0 at 0x73D27B/0x73D286).
     `r = sqrt((dz*dz + dy*dy) + dx*dx)` (x87, the f32 locals).
     `FindObjectsInRange(*start, r, b2D = true, &count (i16), 15, list[15], buildings 0, vehicles 1, peds 0, objects 0, dummies 0)`.
  2. `dzn = (e.z - s.z) / sqrt((dz*dz + dy*dy) + dx*dx)` (numerator recomputed from memory, x87; stored f32).
     `closest = 10000.0f`. If `count <= 0` (signed i16) return.
  3. For i = 0 .. count-1 (i compared as i16), `c = list[i]`; skip if `c == vehicle`, `c == owner`, status (`+0x36 >> 3`)
     is 6 or 7 (STATUS_TRAIN_MOVING / TRAIN_NOT_MOVING), or (type == 2 and `+0x40 & 0x20000000`). Otherwise:
     a. Ensure matrices of `vehicle` and `c` (each read is preceded by "if m_matrix == null: AllocateMatrix();
        placement.UpdateMatrix(m_matrix)"; order: vehicle, c, vehicle, c, vehicle, c, vehicle, c ...; only the first one can
        allocate). V = vehicle matrix pos, C = c matrix pos.
     b. `d2 = sqrt((V.x - C.x)*(V.x - C.x) + (V.y - C.y)*(V.y - C.y))` (stored f32).
     c. `h = d2 * dzn` (stored f32); `v = V.z - (h + C.z)`; if `v < 0.0f` then `az = -(V.z - (h + C.z))` else
        `az = V.z - (h + C.z)` (recomputed after re-reading the matrices; NaN takes the non-negated branch); `az` stored f32.
     d. Continue only if `az * 3.0f < d2` (NaN fails).
     e. `P = c->GetPosition()` (matrix pos or placement pos, no allocation here) with `P.z = 0`;
        `rad = ms_modelInfoPtrs[c->m_nModelIndex]->m_pColModel->boundSphere.radius`;
        `dl = DistToLine(S2, E2, P)`; continue only if `rad * 3.0f > dl` (fcompp + `test ah,0x41`; equal/NaN fail).
     f. `d = sqrt(az*az + d2*d2)`; if `d < closest`: `closest = d` (stored f32), `best = i`.
  4. If `!(closest < 9000.0f)` return (also when no candidate passed; `best` is uninitialised otherwise but unused).
  5. `t = list[best]`, `T = t->GetPosition()` (no allocation). `L = sqrt(dx'^2 + dy'^2)` with `dx' = e.x - s.x`,
     `dy' = e.y - s.y` recomputed from memory (sum order dx'^2 + dy'^2);
     `D = sqrt((T.y - s.y)^2 + (T.x - s.x)^2)` (y term first); `hz = (T.z + 0.3f) - s.z`;
     `end->z = (L / D) * hz + s.z` (x87: `L/D` then multiplied by hz, then + s.z, stored f32).
- Edge cases: D == 0 gives inf/NaN in end->z. The candidate list is capped at 15.
- gta-reversed: `plugin::Call` only (Weapon.cpp:1014). Open questions: none.

## 0x73D720 CWeapon::DoDriveByAutoAiming  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl void (CEntity* owner [esp+4], CVehicle* vehicle [+8], CVector* start [+0xC], CVector* end [+0x10],
  bool canAimVehicles [+0x14])`, plain `ret`. Writes `*end` only when a target is accepted.
- Callers: CWeapon::FireInstantHitFromCar 0x73EC40, CWeapon::FireInstantHit 0x73FB10.
- Callees: CWorld::FindObjectsInRange 0x564A20 (x2), CCollision::DistToLine 0x417610 (cdecl (start, end, point) -> ST0),
  CVehicle::GetPlaneGunsAutoAimAngle 0x6D3E00 (thiscall -> ST0).
- Constants: 10000.0f (imm), 0x858FCC 0.15f, 0x858C80 5.0f, 0x858B50 0.0f, 0x858B8C 0.5f, 0x8595EC 0.017453292f, 0x858FA0 2.5f.
- Fields: entity +0x14 matrix (pos +0x30), +0x04 placement pos (GetPosition = matrix ? &m->pos : &placement.pos, never
  allocates here), +0x36 type bits; ped +0x530 (gr m_nPedState; 0x36 DIE, 0x37 DEAD), +0xFC (CPhysical m_pAttachedTo);
  vehicle +0x594 (gr m_nVehicleType; 3 HELI, 4 PLANE).
- Behaviour:
  1. `d = *end - *start` (x87); `len = sqrt((dz*dz + dy*dy) + dx*dx)` stored f32. `if (owner == null) return` (0x73D777).
  2. `count1 = 0, count2 = 0` (dwords). `FindObjectsInRange(*start, len, b2D = true, &count1, 16, list, buildings 0, vehicles 0,
     peds 1, objects 0, dummies 0)`; if `canAimVehicles`: `FindObjectsInRange(*start, len, true, &count2, 16, &list[(i16)count1],
     0, vehicles 1, 0, 0, 0)`. list has 32 slots. `total = count1 + count2` (i16 compare). `best_score = 10000.0f`
     (this local reuses the slot of `len`).
  3. For i = 0 .. total-1 (`c = list[i]`):
     - skip if `c == owner`; if c is a ped (type 3): skip if `m_nPedState` is 0x36 or 0x37, or if `c->m_pAttachedTo == vehicle`.
     - `dl = DistToLine(*start, *end, c->GetPosition())`.
     - if `vehicle->m_nVehicleType` is not 3/4: `dv = |CP - VP|` (CP = c pos, VP = vehicle pos, sum order (z^2 + y^2) + x^2),
       `score = dl + dv * 0.15f`.
       else (heli/plane): `dv = |CP - VP|` (VP via the vehicle's matrix pointer read once into EDI); if `dv < 5.0f`
       `score = dl / 5.0f` else (incl. NaN) `score = dl / |CP - VP|` (recomputed).
     - `a = CP - *start`, `b = *end - *start`; `dot = (b.z*a.z + b.y*a.y) + b.x*a.x`; skip unless `dot > 0.0f` (NaN skips).
     - if `score < best_score` (strict; NaN no): `best_score = score` (stored f32), `best = i`.
  4. `ang = vehicle->GetPlaneGunsAutoAimAngle()` (called for every vehicle type, even with no candidate);
     `thr = (ang > 0.5f) ? tan(ang * 0.017453292f) (x87 fptan) : 2.5f` (`ang <= 0.5` or NaN -> 2.5f).
     If `!(best_score < thr)` return.
  5. `t = list[best]`, `T = t->GetPosition()`; `L = |*start - *end|`, `D = |*start - T|` (both as (z^2 + y^2) + x^2 of
     start-minus-other); `k = L / D` (stored f32); `v = T - *start` (vz stored f32, vx*k stored f32 before the add);
     `end->x = vx*k + start->x`, `end->y = vy*k + start->y`, `end->z = vz*k + start->z` (each stored f32; all three
     computed before any store).
- Edge cases: no candidate -> best_score stays 10000 and nothing changes unless thr > 10000 (tan close to 90 deg), in which
  case `best` is read uninitialised (local at [esp+0x30] in the loop frame); keep the call order identical, the
  practical case cannot occur for normal angles. D == 0 gives inf/NaN.
- gta-reversed: `plugin::Call` (Weapon.cpp:1019). Open questions: (1) 0x73DA4E: if total == 0 and thr > 10000 the
  target index is uninitialised stack - unreachable in practice (needs GetPlaneGunsAutoAimAngle > ~89.99 deg).

## 0x73E240 CWeapon::FindNearestTargetEntityWithScreenCoors  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl CEntity* (float screenX [esp+4], float screenY [+8], float range [+0xC], CVector point [+0x10..+0x18]
  (by value), float* outX [+0x1C], float* outY [+0x20])`, EAX result, plain `ret`. The arg slots screenX/screenY are
  overwritten with the pixel coordinates (irrelevant to callers).
- Callers: CCam::Get_TwoPlayer_AimVector 0x513E40, CCam::Process_Cam_TwoPlayer_InCarAndShooting 0x519810,
  CWeaponEffects::Render 0x742CF0.
- Callees: CDarkel::ThisPedShouldBeKilledForFrenzy 0x43D2F0, CDarkel::ThisVehicleShouldBeKilledForFrenzy 0x43D350 (cdecl(entity) -> AL),
  CWorld::FindPlayerVehicle 0x56E0D0 (cdecl(-1, false)), CSprite::CalcScreenCoors 0x70CE30 (cdecl(pos, &out, &w, &h, true, true) -> AL).
- Globals: `CPools::ms_pPedPool` 0xB74490 (objects +0, byte map +4 (bit 7 = free), size +8; CPed stride 0x7C4),
  `CPools::ms_pVehiclePool` 0xB74494 (stride 0xA18), `RsGlobal.maximumWidth` 0xC17044 / `maximumHeight` 0xC17048 (i32).
  Constants: 0x863E0C 0.06666667f, 0x858624 1.0f, 0x858B8C 0.5f.
- Behaviour:
  1. `maxD = 0.06666667f * (float)W` (stored f32; not W/15); `sx = (screenX + 1.0f) * (float)W * 0.5f` (fmul by the fild'ed W);
     `sy = (screenY + 1.0f) * H (fimul) * 0.5f`; both stored f32. `best = null`.
  2. Peds: for i = size-1 down to 0 (descending), skip free slots (`byteMap[i] & 0x80`); `p = &objects[i]`:
     skip if `p->m_nPedState (+0x530) == 0x37` (DEAD only) or `+0x46C & 0x100` (bInVehicle) or
     `!ThisPedShouldBeKilledForFrenzy(p)`. Then `Test(p)` (below).
  3. Vehicles: for i = size-1 down to 0, skip free slots; skip if `v == FindPlayerVehicle(-1, false)` (called per vehicle)
     or `!ThisVehicleShouldBeKilledForFrenzy(v)`. Then `Test(v)`.
  4. Return `best`.
  Test(e): `P = e->GetPosition()` (matrix pos or placement pos); `if (!CalcScreenCoors(P, &o, &w, &h, true, true)) return`;
  `d = sqrt((o.y - sy)^2 + (o.x - sx)^2)` (y term first); `if (!(d < maxD))` return (NaN rejects);
  `q = ((P.z-pt.z)^2 + (P.y-pt.y)^2) + (P.x-pt.x)^2`; continue only if `range*range > q` (equal/NaN reject);
  `maxD = d` (stored f32), `best = e`; if `outX != null`: `*outX = o.x / ((float)W * 0.5f) - 1.0f`,
  `*outY = o.y / ((float)H * 0.5f) - 1.0f` (outY is NOT null-checked, only outX).
- gta-reversed: has (unhooked, `.Reversed = false`) C++ at Weapon.cpp:1131. Differences vs bytes: (a) pools iterated
  ascending (`GetAllValid`) where the original goes from the highest slot down - with the strict `<` the winner on exact
  ties differs (rows Weapon.cpp:1162/:1172 CONFIRMED); (b) compares squared screen distances against `(W/15)^2` instead of
  `sqrt` against `W*0.06666667f` (rounding at the boundary); (c) writes the outputs only if both pointers are non-null
  (original: only outX is tested). Port keeps the original.
- Open questions: none.

## 0x73EC40 CWeapon::FireInstantHitFromCar  (catalogue: specified/REA, port level 20)
- Convention: `__thiscall bool (CVehicle* veh [esp+4], bool left [+8], bool right [+0xC])` (ECX = CWeapon*), `ret 0xC`, always
  returns AL = 1. Frame S = ESP after `sub esp,0x38` + 4 pushes (`this` saved at S+0x14, start S+0x18, end S+0x24, temp S+0x3C).
- Callers: CWeapon::FireFromCar 0x73FA20.
- Callees: CWeaponInfo::GetWeaponInfo 0x743C60 (type, 1), RwHelper::GetAnimHierarchyFromSkinClump 0x734A40, RpHAnimIDGetIndex 0x7C51A0,
  RpHAnimHierarchyGetMatrixArray 0x7C5120, RwV3dTransformPoints 0x7EDD90, CEntity::GetColModel 0x535300, `CMatrix * CVector` 0x59C890
  (cdecl(out, mat, vec)), rand, CWorld::FindPlayerPed 0x56E210, CWeapon::DoDriveByAutoAiming 0x73D720, CWeapon::FireInstantHitFromCar2
  0x73CBA0 (thiscall(CVector start, CVector end (by value), CVehicle*, CPed* driver)).
- Data: `wi` = GetWeaponInfo(m_Type, 1): +0x08 range f32, +0x24 fire offset CVector. `mi = ms_modelInfoPtrs[veh->m_nModelIndex]`:
  +0x3C vehicle type, +0x5C vehicle structure pointer; seat position `P = (mi->+0x3C == 5) ? struct+0 : struct+0x30` (CVector).
  `M` = veh->m_matrix (right +0x00, forward +0x10, pos); `mv` = veh->m_vecMoveSpeed (+0x44); `ts` = CTimer::ms_fTimeStep;
  bone transform `B(v)` = v transformed (RwV3dTransformPoints, 1 point, in place) by `GetMatrixArray(h)[RpHAnimIDGetIndex(h,
  driver->+0x4A0->+0x14)]`, `h = GetAnimHierarchyFromSkinClump(driver->m_pRwClump (+0x18))`, driver = veh->+0x460.
  Constants: 0x872C6C 0.63, 0x858CDC 0.001, 0x858C28 0.05, 0x858C84 0.25, 0x858CC8 0.6, 0x8665BC 0.52, 0x872C68 0.18, 0x858B8C 0.5,
  0x866CAC 0.55, 0x858CC4 0.2, 0x858EE8 0.4, 0x859BB4 1.8, 0x858B1C 0.1, 0x858C58 0.01, 0x872C64 1.28.
- Behaviour (start S and end E):
  A. `veh->+0x590 != 9` (not a bike class), 0x73F151: `o = wi->fireOffset`; if `right`: `o = (o.x*1.8f, o.y*1.8f, o.z*1.8f - 0.1f)`
     (each product stored f32). `S = B(o)` (driver is dereferenced without a null check); `S += mv * ts` (x, y, z products first).
     `E = S + d * wi->range` with `d = left ? -M.right : (right ? M.right : M.forward)` (left: `S - M.right*range`, computed as
     `S.x - range*M.right.x` etc.).
  B. bike class (9) with a driver, 0x73EC7C: same as A but without the 1.8/-0.1 scaling.
  C. bike class (9) without driver, seat geometry (cm = veh->GetColModel(), `r = rand() & 0xFF` drawn before GetColModel):
     - left (0x73EDE9): `L = (-cm.max.x (+0xC) - 0.25f, r*0.001f + (P.y - 0.05f), P.z + 0.63f)`; `S = M*L + mv*ts`;
       `E = M * (-range, P.y, P.z + 0.6f)`.
     - right (0x73EF02): `L = (cm.max.x + 0.25f, r*0.001f + (P.y - 0.18f), P.z + 0.52f)`; `S = M*L + mv*ts`; `E = M * (range, P.y, P.z + 0.5f)`.
     - neither (0x73F021): `L = (r*0.001f - 0.4f, cm.max.y (+0x10) + P.y + 0.2f, P.z + 0.55f)`; `S = M*L + mv*ts`; `E = M * (0, range, P.z + 0.5f)`.
     (`M*v` is the full matrix transform incl. position.)
  D. Common (0x73F2FF): `r1 = rand(), r2 = rand()`; `E.x += (r2 & 0xFF)*0.01f - 1.28f`, `E.y += (r1 & 0xFF)*0.01f - 1.28f` (FIRST rand to y),
     `E.z += (rand() & 0xFF)*0.01f - 1.28f` (third rand); each offset is `fild; fmul 0.01f; fsub 1.28f` then added to E.
     `DoDriveByAutoAiming(FindPlayerPed(-1), veh, &S, &E, false)` - the owner is always the PLAYER ped, whoever fires.
     `this->FireInstantHitFromCar2(S, E, veh, veh->m_pDriver)`; return true.
- gta-reversed: `plugin::CallMethodAndReturn` (unreversed). Open questions: (1) 0x73F021-0x73F03E: in path C-neither the seat
  base selection was read with some register stores elided in this analysis; the structure (type 5 -> +0, else +0x30) matches
  the other two paths but should be re-checked against the bytes. (2) 0x73ECB0/0x73F1F0: which bone `driver->+0x4A0` denotes
  (gr m_apBones index) is not resolved; value used exactly as read. Confidence: medium.

## 0x73F480 CWeapon::CheckForShootingVehicleOccupant  (catalogue: specified/REA, port level 17)
- Convention: `__cdecl bool (CEntity** pEntity [esp+4], CColPoint* cp [+8], eWeaponType type [+0xC] (unused),
  const CVector* origin [+0x10], const CVector* target [+0x14])`, plain `ret`. Entry `nop; jmp 0x4019CE` (.text stub that
  loads fs:[0]) then SEH frame (local CMatrix). Return: AL = 1 if an occupant was hit; otherwise AL is garbage (low byte of a
  pointer). All 3 callers (CBulletInfo::Update 0x73626F, CWeapon::FireInstantHit 0x7407B4, CWeapon::FireM16_1stPerson 0x741D9E)
  ignore AL, so a port may return false there.
- Callees: CColPoint copy/assign thunk 0x40FC80 (thiscall(const CColPoint&)), CColLine::CColLine 0x40EF50 (thiscall(start, end)),
  RwHelper::GetAnimHierarchyFromSkinClump 0x734A40, RpHAnimIDGetIndex 0x7C51A0, RpHAnimHierarchyGetMatrixArray 0x7C5120,
  RwV3dTransformPoints 0x7EDD90, CColSphere::Set 0x40FD10 (thiscall(radius, const CVector& c, u8 material, u8 piece, u8 light)),
  CCollision::ProcessLineSphere 0x412AA0 `(line, sphere, CColPoint&, float& minDist)`, CEntity::GetColModel 0x535300,
  CMatrix::Invert 0x59B920 `(const CMatrix* in, CMatrix* out)`, CMatrix copy ctor 0x59BCF0, operator*(CMatrix, CVector) 0x59C890
  `cdecl(out, mat, vec)`, CCollision::CalculateTrianglePlanes 0x418580, SurfaceInfos_c::IsGlass 0x55E790 (thiscall on
  g_surfaceInfos 0xB79538), CCollision::TestLineTriangle 0x413AC0 `(line, verts, tri, plane)`, CDamageManager::ProgressPanelDamage
  0x6C23C0 / GetPanelStatus 0x6C2180, CAutomobile::SetPanelDamage 0x6B1480, CMatrix::~CMatrix 0x59ACD0.
- Behaviour:
  1. `veh = *pEntity`; if `(veh->+0x36 & 7) != 2` return (nothing written).
  2. `saved = *cp` (local copy); `line = CColLine(*origin, *target)`; `minDist = 1.0f`; `hit = 0`.
  3. Occupants, in this order: driver (`veh+0x460`), then the 8 passenger slots `veh+0x464 + 4k` (k = 0..7). For each non-null
     ped with `ped+0x470 & 0x4000000`:
     `h = GetAnimHierarchyFromSkinClump(ped->m_pRwClump (+0x18))`; `idx = RpHAnimIDGetIndex(h, ped->m_apBones[0] (+0x490)->+0x14)`;
     `P = (0,0,0)` transformed by `GetMatrixArray(h)[idx]` (RwV3dTransformPoints, 0x40-byte matrices);
     `sphere.Set(0.2f, (P.x, P.y, P.z + 0.1f [0x858B1C]), 0, 9, 0xFF)`; if `ProcessLineSphere(line, sphere, *cp, minDist)`:
     `*pEntity = ped; hit = 1`. minDist is shared, so later occupants only hit if closer; cp holds the last hit.
  4. Windscreen (only if `veh+0x590 == 0`, the automobile class):
     `d = *target - *origin` (dx, dy stored f32); F = veh matrix forward (+0x10..+0x18), U = up (+0x20..+0x28):
     `(d.y*F.y + d.z*F.z) + d.x*F.x < 0.0f` required (NaN fails); then if `!((d.y*U.y + d.z*U.z) + d.x*U.x <= 0.0f)` require
     `veh+0x429 & 4`. `cm = veh->GetColModel(); data = cm->m_pColData (+0x2C)`; require `numTriangles (i16 +4) > 0`.
     `inv = Invert(veh->m_matrix)`; `line.start = inv * line.start`, `line.end = inv * line.end`; `CalculateTrianglePlanes(cm)`.
     For t = 0 .. numTriangles-1 (i16): `tri = data->triangles (+0x18) + 8t`; if `IsGlass(tri.material (+6))` and
     `TestLineTriangle(line, data->vertices (+0x14), tri, data->planes (+0x1C) + 0xA*t)`: windscreen hit -> stop scanning:
     `dm = veh+0x5A0`; if `dm.ProgressPanelDamage(4)`: { if `dm.GetPanelStatus(4) == 2` `dm.ProgressPanelDamage(4)`;
     `veh->SetPanelDamage(4, true)` }. (SetPanelDamage only runs when the first ProgressPanelDamage returned true.)
  5. If `!hit`: `*pEntity = veh` (unchanged) and `*cp = saved` (restore).
- gta-reversed: `plugin::CallAndReturn` (Weapon.cpp). Open questions: (1) 0x73F4FE/0x73F5FB: meaning of `ped+0x470 & 0x4000000`
  and of `veh+0x429 & 4` (gr flag names not verified); values exact. (2) 0x73F545: `m_apBones[0]` node identity (which bone) not
  resolved; the code reads index 0's `+0x14`.

## 0x73FB10 CWeapon::FireInstantHit  (catalogue: specified/REA, port level 19)
STRUCTURAL SPEC (block level). The function has 1716 instructions; this section fixes the control flow, the calls and their
arguments, the constants and the RNG draws, but the x87 operation order inside the aiming blocks listed under "Open
questions" was NOT transcribed and must be taken from the bytes (`tools/specs/dis.py 0x73FB10`) before porting.
- Convention: `__thiscall bool (CEntity* firer [esp+4], CVector* origin [+8], CVector* muzzle [+0xC], CEntity* targetEnt [+0x10],
  CVector* target [+0x14], CVector* originForDriveBy [+0x18], bool arg6 [+0x1C], bool doMuzzleFx [+0x20])`, `ret 0x20`.
  SEH frame (local CMatrix destroyed at 0x7411FB). Locals 0x198.
- Callers: CWeapon::Fire 0x742300 (several sites).
- Callees (in order of appearance): CPed::GetWeaponSkill(type) 0x5E3B60, CWeaponInfo::GetWeaponInfo 0x743C60, CPedIntelligence::GetTaskUseGun
  0x600F70, bone transform (GetAnimHierarchyFromSkinClump 0x734A40 / RpHAnimIDGetIndex / GetMatrixArray / RwV3dTransformPoints),
  CVector::Normalise, CBirds::HandleGunShot 0x712E40, CShadows::GunShotSetsOilOnFire 0x707550, CWorld::ProcessLineOfSight 0x56BA00
  (4 sites), CPed::GetTransformedBonePosition 0x5E01C0, CPed::GetBonePosition 0x5E4280, CWeapon::TargetWeaponRangeMultiplier 0x73B380,
  CGeneral::GetRandomNumberInRange(float) 0x41BD90, CrossProduct 0x59C730, CPed::IsPlayer, CVector::Magnitude 0x4082C0, rand (x6),
  CCamera::Find3rdPersonCamTargetVector 0x514970, CCam::Get_TwoPlayer_AimVector 0x513E40, CheckForShootingVehicleOccupant 0x73F480,
  FindPlayerPed, DoDriveByAutoAiming 0x73D720, DoDoomAiming 0x73CDC0, CEventGunShot ctor 0x4AC610 + GetEventGlobalGroup 0x4ABA50 +
  CEventGroup::Add 0x4AB420, CEventGunShotWhizzedBy ctor 0x4B1FD0 (+ Add), CInterestingEvents::Add 0x602590 (type 0x16),
  `_ftol2`, CPointLights::AddLight 0x7000E0, Fx_c::TriggerGunshot 0x4A0DE0, CWeapon::AddGunshell 0x73A3E0, CWaterLevel line test
  0x6E61B0 (x2, see `_unattributed-G.md`), Fx_c::TriggerBulletSplash 0x4A10E0, CAudioEngine::ReportBulletHit 0x506EC0, SetUpPelletCol
  0x73C710, CPedModelInfo::AnimatePedColModelSkinned 0x4C6F70, CEntity::GetColModel, CCollision::ProcessColModels 0x4185C0,
  CWeapon::DoBulletImpact 0x73B550 (4 sites), CWorld::ResetLineTestOptions 0x5631C0, event dtors 0x4B2070 / 0x4AC720.
- Globals written: CWorld::fWeaponSpreadRate 0xB7CD64, CWorld::pIgnoreEntity 0xB7CD68, CWorld::bIncludeBikers 0xB7CD6F,
  bIncludeCarTyres 0xB7CD70, bIncludeDeadPeds 0xB7CD71 (all reset by ResetLineTestOptions at the end), byte 0xC8A80C (toggle).
  .data tuning floats read: fPlayerAimScale 0x8D6110 (0.75), fPlayerAimScaleDist 0x8D6114 (5.0), fPlayerAimRotRate 0x8D6118
  (0.0062831854), SHOTGUN_SPREAD_RATE 0x8D611C (0.05), SHOTGUN_NUM_PELLETS 0x8D6120, SPAS_NUM_PELLETS 0x8D6124.
- Behaviour (blocks):
  1. Setup (0x73FB10-0x73FC4E): `wi = GetWeaponInfo(m_Type, firer is ped ? firer->GetWeaponSkill(m_Type) : 1)`; local start =
     `*origin`, local muzzle copy = `*muzzle`; if `originForDriveBy` non-null it replaces the start copy (0x73FBD4). Accuracy
     `acc = 0` unless the firer is a ped: `acc = (100.0f - (float)firer->m_nWeaponAccuracy (u8 +0x71A)) / wi->+0x38` (fsubr, fdiv);
     if `firer->m_pPlayerData (+0x480)` and `firer->+0x46C & 0x4000000` (ducking): `acc *= 0.5f`; `task = GetTaskUseGun(intel)`.
  2. Shotguns (types 25, 26, 27): `acc = 0`; `CWorld::fWeaponSpreadRate = SHOTGUN_SPREAD_RATE / wi->+0x38` (0x73FC5F-0x73FC72).
  3. Task-driven ped shot (task non-null and `task->+0xE` set, 0x73FC89): the local vector `(wi->+8 (range), 0, 0)` is transformed
     by the bone matrix of `firer->+0x4A0 ->+0x14` (0x73FC94-0x73FCFA) giving the end point; a normalised direction is formed
     from it (0x73FCFF-0x73FD42; operand details: open question 1); ignore entity = the ped's vehicle when
     `+0x46C & 0x100` and `+0x58C` set, else its `m_pAttachedTo (+0xFC)` if that is a vehicle; `bIncludeBikers = 1`;
     HandleGunShot(&start, &end), GunShotSetsOilOnFire; `ProcessLineOfSight(start, end, cp, hitEnt, 1,1,1,1,1, 0,0, 1)`; -> block 9.
  4. Ped firer without task (0x73FDD7..0x74034A): if targetEnt or target: aim at targetEnt (peds: GetTransformedBonePosition /
     GetBonePosition(3)) else target; range scaling via TargetWeaponRangeMultiplier; for player peds with no explicit target the
     aim uses camera modes (active cam mode 0x35 / 0x37 use cam+0x190 / +0x1B4 vectors at 0xB6F32C/0xB6F350, else cross products
     of the ped matrix), a time-based wobble `now * fPlayerAimRotRate` (0x7400E7-0x7401D5, scaled by 0.04f) and the scale
     `fPlayerAimScale * 0.5f` with `fPlayerAimScaleDist`, a GRN(float) draw (0x73FFED); for non-player peds with `acc > 0`:
     3 rand() draws giving the offset `((t*0.4f - 0.2f), (t*0.4f - 0.2f), (t*0.2f - 0.1f))` (0x74024F-0x7402B6, t = rand*1/32767,
     one draw per axis, x first) scaled by `min(|v|, 0.33f) * 0.90909094f + 0.8f` for player-attached cases (0x740213-0x740241).
     Ignore-entity selection (0x7402F4-0x74032A, `+0x42D & 4` tests on vehicles), `bIncludeDeadPeds = 1` for players. -> block 8.
  5. Player in 3rd-person mouse/aim modes (0x74034A): active cam mode in {0x35, 0x37, 0x41, 0x31}: Find3rdPersonCamTargetVector(range*3, ...),
     direction normalise, same scale/wobble scheme as block 4 with factor 3.0f; mode 0x31 uses CCam::Get_TwoPlayer_AimVector.
     Ignore entity / bIncludeDeadPeds / bIncludeCarTyres / bIncludeBikers set (0x740682-0x7406DF); HandleGunShot, oil;
     LOS (1,1,1,1,1,0,0,1); on hit: TargetWeaponRangeMultiplier test, CheckForShootingVehicleOccupant(&hitEnt, &cp, type, start, end)
     (0x7407B4) -> block 9.
  6. Vehicle firer (type 2, 0x7407C1): `acc = 0.6f`; driver/heli handling (vehicle subtype 8 / types 3, 4), DoDriveByAutoAiming
     (owner = FindPlayerPed(-1) for subtype 8), normalise; model 0x1BF / 0x1D5 -> 0.1f, model 0x234 -> 0.1f else 0.3f (0x7408E7-0x740901);
     three rand() draws (same formulas as block 4, 0x740909-0x740974); normalise; `pIgnoreEntity = firer`.
  7. Other firers (0x7409FD-0x740B17): DoDoomAiming(firer, &start, &end) for non-player peds without target (0x740AD9); ignore
     entity selection.
  8. Generic LOS (0x740B1D): `bIncludeBikers = 1`; HandleGunShot; oil; `ProcessLineOfSight(start, end, cp, hitEnt, 1,1,1,1,1,0,0,1)`.
  9. Events and effects (0x740B71-0x740F66): weapon type 0x17/0x11 special-case; `CEventGunShot` (0x4AC610; arguments: open question 5) added to the global group; if a ped was hit: `CEventGunShotWhizzedBy` added; `CInterestingEvents::Add(0x16, firer)`.
     Muzzle light block (entry tests at 0x740C66-0x740C78 include an unsigned `<= 0x10` compare; open question 5): `n = _ftol2(time * 900.0f)`; if `n < 50` toggle byte 0xC8A80C
     (only every other call continues); light colours by weapon class (0.65/0.25, 0.2/0.3, 0.2/0.25, 0.3/0.45 pairs at
     0x740CB6-0x740CF4); `CPointLights::AddLight(..., 3.0f, 0.25f, 0.22f, ...)`; Fx_c::TriggerGunshot (if doMuzzleFx); AddGunshell.
     Water (entry conditions 0x740E3A-0x740EA9: a byte range test 2..4 and IsPlayer; open question 6): `0x6E61B0(start, end, &p)` twice (0x740EEB, 0x740F36); on hit
     TriggerBulletSplash(p) and `ReportBulletHit(0, 0x27, p, 0)`.
  10. Impact (0x740F6A-0x7412E4): if `fWeaponSpreadRate == 0` or no hit or the hit is a wheel piece (13..16) of a vehicle ->
      single `DoBulletImpact(firer, hitEnt, &start, &end, &cp, 0)` (0x7412DF). Otherwise shotgun pellets:
      `n = SHOTGUN_NUM_PELLETS` (SPAS: SPAS_NUM_PELLETS); SetUpPelletCol(n, firer, hitEnt, end, cp, matrix); hit entity col model
      (peds: AnimatePedColModelSkinned) ; `ProcessColModels(pelletMatrix, ms_PelletTestCol, entMatrix, entCol, 0xB9ACD0 x2, ...)`;
      count pellet hits (`< 1.0f` tests, loop unrolled by 4); DoBulletImpact for each hit (incremental flag 1 after the first);
      for peds/vehicles: retry with another LOS (0x7412B5) when the fraction tests `>= 1.0f` / `< 0.5f` and a counter < 2.
  11. Exit: `CWorld::ResetLineTestOptions()`; destroy the event locals; return AL (hit flag).
- gta-reversed: `plugin::CallMethodAndReturn` (unreversed); gr's constants PLAYER_AIM_SCALE 0.75, PLAYER_AIM_SCALE_DIST 5.0,
  PLAYER_ANIM_ROT_RATE 0.0062832, SHOTGUN_SPREAD_RATE 0.05, SHOTGUN_NUM_PELLETS 15, SPAS_NUM_PELLETS 4 correspond to the .data
  values above (the original reads them from .data, so they could be patched; keep reading the globals).
- Open questions (must be resolved from the bytes before porting; confidence low):
  (1) 0x73FC94-0x73FD42 exact construction of the bone-local start vector. (2) 0x73FE0A-0x7402EE aim computation: x87 order,
  which vectors feed the cross products, wobble formula. (3) 0x74034A-0x740679 same for camera modes. (4) 0x7407C1-0x7409F8 vehicle
  aiming details (subtype tests, accuracy scaling of the random offset). (5) 0x740B71-0x740E3A event arguments and light colour
  selection table per weapon. (6) 0x740E3A-0x740F66 water-test conditions. (7) 0x740F6A-0x7412E4 pellet loop bookkeeping and the
  retry LOS. (8) Return value register (bl/al) assignments along all paths.
