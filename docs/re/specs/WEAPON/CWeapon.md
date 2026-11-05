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
  C. bike class (9) without driver, seat geometry (cm = veh->GetColModel(), `r = rand() & 0xFF`; left/right call rand() BEFORE
     GetColModel (0x73EE19/0x73EF2E), the neither path calls GetColModel FIRST (0x73F047, rand at 0x73F04E)):
     - left (0x73EDE9): `L = (-cm.max.x (+0xC) - 0.25f, r*0.001f + (P.y - 0.05f), P.z + 0.63f)`; `S = M*L + mv*ts`;
       `E = M * (-range, P.y, P.z + 0.6f)`.
     - right (0x73EF02): `L = (cm.max.x + 0.25f, r*0.001f + (P.y - 0.18f), P.z + 0.52f)`; `S = M*L + mv*ts`; `E = M * (range, P.y, P.z + 0.5f)`.
     - neither (0x73F021): `L = (r*0.001f - 0.4f, cm.max.y (+0x10) + P.y + 0.2f, P.z + 0.55f)`; `S = M*L + mv*ts`; `E = M * (0, range, P.z + 0.5f)`.
     (`M*v` is the full matrix transform incl. position.)
  D. Common (0x73F2FF): `r1 = rand(), r2 = rand()`; `E.x += (r2 & 0xFF)*0.01f - 1.28f`, `E.y += (r1 & 0xFF)*0.01f - 1.28f` (FIRST rand to y),
     `E.z += (rand() & 0xFF)*0.01f - 1.28f` (third rand); each offset is `fild; fmul 0.01f; fsub 1.28f` then added to E.
     `DoDriveByAutoAiming(FindPlayerPed(-1), veh, &S, &E, false)` - the owner is always the PLAYER ped, whoever fires.
     `this->FireInstantHitFromCar2(S, E, veh, veh->m_pDriver)`; return true.
- gta-reversed: `plugin::CallMethodAndReturn` (unreversed). Re-check (H6) of 0x73F021-0x73F045: `mi->+0x3C == 5 ? struct+0 :
  struct+0x30` is computed twice into the (dead) arg slots S+0x54 (used for P.z) and S+0x50 (used for P.y); both are the
  same P, so `L = (r*0.001f - 0.4f, (cm.max.y + P.y) + 0.2f, P.z + 0.55f)` and `E = M*(0, range, P.z + 0.5f)` (0x73F0FC re-selects
  P the same way) are confirmed; the x87 order is `fld P.z; fadd 0.55; fld cm+0x10; fadd P.y; fadd 0.2; fild r; fmul 0.001;
  fsub 0.4`. The left/right seat formulas, the D-block rand order and the A/B paths were re-read too and match. Note ebp (wi) is
  overwritten by the seat pointer in the right path (0x73EF14), which therefore reads range via the saved wi at S+0x10.
  Open question: 0x73ECB0/0x73F1F0 `driver->+0x4A0` = m_apBones[6] (PED_NODE_RIGHT_HAND) if gr's CPed layout (m_apBones at
  0x488, consistent with +0x528 = m_pedIK.m_nFlags) holds; value used exactly as read. Confidence: medium-high.

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
     `h = GetAnimHierarchyFromSkinClump(ped->m_pRwClump (+0x18))`; `idx = RpHAnimIDGetIndex(h, ped->m_apBones[2 = PED_NODE_HEAD] (+0x490; array at +0x488, see VEH/CBike.md 0x6B7280 and CPed::DoesLOSBulletHitPed)->+0x14 m_nNodeId)`;
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
  and of `veh+0x429 & 4` (gr flag names not verified); values exact. (2) 0x73F545: resolved by the lead's cross-check: +0x490 is
  `m_apBones[2]` (PED_NODE_HEAD), not index 0; the code reads that node's `+0x14` (m_nNodeId).

## 0x73FB10 CWeapon::FireInstantHit  (catalogue: specified/REA, port level 19)
Full block-level spec with x87 order (rewritten by follow-up H6 from the bytes; the first-wave structural notes are folded in and
corrected where they were wrong, see "Corrections" at the end).
- Convention: `__thiscall bool (CEntity* firer [esp+4], CVector* origin [+8], CVector* muzzle [+0xC], CEntity* targetEnt [+0x10],
  CVector* target [+0x14], CVector* originOverride [+0x18], bool forceAim [+0x1C], bool doMuzzleFx [+0x20])`, `ret 0x20`
  (gr names: firingEntity, origin, muzzlePosn, targetEntity, target, originForDriveBy, arg6, muzzle). SEH frame (handler 0x848EBB,
  state byte F+0x1B0: 0 after the gunshot event, 1 after the whizzed-by event, 2 while the pellet CMatrix is live).
  Return: AL = 1 on the single exit path (0x74131E `mov al,1`); there is no other return, so the function ALWAYS returns true.
- Frame: F = ESP after the prologue (0x73FB2E; 0x1B4 bytes pushed). Args at F+0x1B8 (firer) .. F+0x1D4 (doMuzzleFx).
  Locals: E F+0x10 (CVector end point, also used as a direction), acc F+0x1C (f32), F+0x20 (wi pointer, later f32/int temp),
  F+0x24 (temp), hitEnt F+0x28 (CEntity*), S F+0x2C (CVector start), T F+0x38 (CVector temp / camera source / water point),
  A F+0x44, this F+0x50, F+0x54 (temp), B F+0x60, dir F+0x6C (CVector used by the muzzle FX), M F+0x78 (copy of *muzzle),
  F+0x84 (temp CVector), cp F+0x90 (CColPoint 0x2C; cp.point at +0, cp.pieceTypeB at +0x24 = F+0xB4), dists F+0xBC (float[15]),
  pelletMat F+0xF8 (CMatrix 0x48), whizz F+0x140 (CEventGunShotWhizzedBy), shot F+0x174 (CEventGunShot).
  Registers kept across blocks: EDI = firer, EBX = wi (until block 10), ESI = targetEnt (from 0x73FC7C), EBP = firer if it is a ped
  (`(firer->+0x36 & 7) == 3`), else 0.
- Callers (5): CWeapon::Fire 0x742300 (3 sites), CRunningScript::ProcessCommands1300To1399 0x48CDD0, 0x56EC80 (unnamed),
  CAutomobile::TankControl 0x6AE850, CVehicle::FirePlaneGuns 0x6D4D30 (2 sites). AL is always 1.
- Callees (address, convention): CPed::GetWeaponSkill(type) 0x5E3B60 (thiscall), CWeaponInfo::GetWeaponInfo 0x743C60 (cdecl(type, skill)),
  CPedIntelligence::GetTaskUseGun 0x600F70 (thiscall), RwHelper::GetAnimHierarchyFromSkinClump 0x734A40, RpHAnimIDGetIndex 0x7C51A0,
  RpHAnimHierarchyGetMatrixArray 0x7C5120, RwV3dTransformPoints 0x7EDD90 (cdecl(out, in, n, mat)), CVector::Normalise 0x59C910
  (thiscall, 8 sites), CrossProduct 0x59C730 (cdecl(out, a, b) = a x b, 6 sites), CPed::IsPlayer 0x5DF8F0 (6 sites),
  CPed::GetTransformedBonePosition 0x5E01C0 (thiscall(RwV3d&, bone, bool), ret 0xC), CPed::GetBonePosition 0x5E4280 (same shape),
  CWeapon::TargetWeaponRangeMultiplier 0x73B380 (cdecl(victim, owner) -> ST0), CGeneral::GetRandomNumberInRange<float> 0x41BD90
  (cdecl(min, max) -> ST0), CVector::Magnitude 0x4082C0 (thiscall -> ST0), rand 0x821B1E (6 sites), _ftol2 0x821B40 (truncation),
  CCamera::Find3rdPersonCamTargetVector 0x514970 (thiscall(float range, CVector src by value, CVector& outSource, CVector& outTarget),
  ret 0x18), CCam::Get_TwoPlayer_AimVector 0x513E40 (thiscall(CVector&), ret 4), CWorld::FindPlayerPed 0x56E210 (cdecl(-1)),
  CWeapon::DoDriveByAutoAiming 0x73D720 (cdecl(owner, veh, CVector* start, CVector* end, bool canAimVehicles)),
  CWeapon::DoDoomAiming 0x73CDC0 (cdecl(owner, CVector* start, CVector* end)), CPlaceable::AllocateMatrix 0x54F560 +
  CSimpleTransform::UpdateMatrix 0x54F1B0 ("ensure matrix", 5 sites), CBirds::HandleGunShot 0x712E40 (cdecl(const CVector* a,
  const CVector* b), 4 sites), CShadows::GunShotSetsOilOnFire 0x707550 (cdecl(a, b), 4 sites), CWorld::ProcessLineOfSight 0x56BA00
  (cdecl, 12 args, 4 sites, return value never used), CWeapon::CheckForShootingVehicleOccupant 0x73F480, CEventGunShot ctor 0x4AC610
  (thiscall(CEntity*, CVector start, CVector end (by value), bool), ret 0x20), CEventGunShotWhizzedBy ctor 0x4B1FD0
  (thiscall(CEntity*, const CVector&, const CVector&, bool), ret 0x10), GetEventGlobalGroup 0x4ABA50, CEventGroup::Add 0x4AB420
  (thiscall(CEvent*, bool)), CInterestingEvents::Add 0x602590 (thiscall on g_InterestingEvents 0xC0B058), CPointLights::AddLight
  0x7000E0 (cdecl, 0x38 bytes), Fx_c::TriggerGunshot 0x4A0DE0 (thiscall on g_fx 0xA9AE00, ret 0x10), CWeapon::AddGunshell 0x73A3E0
  (thiscall, ret 0x10), water line test 0x6E61B0 (cdecl(CVector a, CVector b by value, CVector* out) -> AL; see `_unattributed.md`),
  Fx_c::TriggerBulletSplash 0x4A10E0 (thiscall(const CVector&)), CAudioEngine::ReportBulletHit 0x506EC0 (thiscall on AudioEngine
  0xB6BC90), CWeapon::SetUpPelletCol 0x73C710 (callee pops 0x18: (int n, CEntity* owner, CEntity* victim, CVector& start,
  CColPoint& cp, CMatrix& out)), CPedModelInfo::AnimatePedColModelSkinned 0x4C6F70 (thiscall(RpClump*) -> CColModel*),
  CEntity::GetColModel 0x535300, CCollision::ProcessColModels 0x4185C0 (cdecl, 8 args), CWeapon::DoBulletImpact 0x73B550
  (thiscall(owner, victim, const CVector& start, const CVector& end, const CColPoint&, int32 incrementalHit), ret 0x18, 3 sites),
  CMatrix::~CMatrix 0x59ACD0, CWorld::ResetLineTestOptions 0x5631C0, ~CEventGunShotWhizzedBy 0x4B2070, ~CEventGunShot 0x4AC720.
- Globals: R CTimer::m_snTimeInMilliseconds 0xB7CB84 (u32); TheCamera 0xB6F028: m_nActiveCam byte 0xB6F081, Cams[i].m_nMode
  (u16) at 0xB6F1A8 + 0x238*i, Cams[0] 0xB6F19C, Cams[0].m_vecFront 0xB6F32C, Cams[0].m_vecUp 0xB6F350..0xB6F358;
  .data tuning (read every call, keep reading the globals): fPlayerAimScale 0x8D6110 (0.75), fPlayerAimScaleDist 0x8D6114 (5.0),
  fPlayerAimRotRate 0x8D6118 (0.0062831854), SHOTGUN_SPREAD_RATE 0x8D611C (0.05), SHOTGUN_NUM_PELLETS 0x8D6120 (int 15),
  SPAS_NUM_PELLETS 0x8D6124 (int 4). R/W CWorld::fWeaponSpreadRate 0xB7CD64; W CWorld::pIgnoreEntity 0xB7CD68, bIncludeBikers 0xB7CD6F,
  bIncludeCarTyres 0xB7CD70, bIncludeDeadPeds 0xB7CD71; R/W byte 0xC8A80C (muzzle-flash toggle; = ms_PelletTestCol+0x30);
  R CModelInfo::ms_modelInfoPtrs 0xA9B0C8; A CWorld::m_aTempColPts 0xB9ACD0 (stride 0x2C); A CWeapon::ms_PelletTestCol 0xC8A7DC.
  Constants: 0x858624 1.0, 0x858628 100.0, 0x858B50 0.0, 0x858B8C 0.5, 0x858C58 0.01, 0x858CC4 0.2, 0x858B1C 0.1, 0x858EE8 0.4,
  0x858C7C 3.0518509e-05 (1/32767), 0x858CEC 0.04, 0x858C54 4294967296.0, 0x85AD74 0.33, 0x872C70 0.90909094, 0x858C98 0.8,
  0x858B3C 3.0, 0x858978 900.0; immediates 0x3F19999A 0.6, 0x3E99999A 0.3, 0x3DCCCCCD 0.1, 0x3E4CCCCD 0.2, 0x3F266666 0.65,
  0x3E800000 0.25, 0x3EE66666 0.45, 0x3E6147AE 0.22, 0x40400000 3.0.
- Fields: entity +0x14 m_matrix (right +0, forward +0x10), +0x22 m_nModelIndex (i16), +0x36 (type = &7, status = >>3), +0x40 physical
  flags (bit 8 = bSubmergedInWater), +0x44 m_vecMoveSpeed, +0xFC m_pAttachedTo. Ped: +0x46C flags (bit 8 bInVehicle, bit 26
  0x4000000 bIsDucking), +0x470 flags (0x2000000: gr bDoomAim, by bit count), +0x47C m_pIntelligence, +0x480 m_pPlayerData,
  +0x4A0 m_apBones[6] (PED_NODE_RIGHT_HAND, m_apBones starts at 0x488) -> AnimBlendFrameData +0x14 m_nNodeId, +0x4F8
  m_pGunflashObject, +0x528 m_pedIK.m_nFlags (bit 0 bGunReachedTarget), +0x58C m_pVehicle, +0x71A m_nWeaponAccuracy (u8).
  CPlayerPedData +0x2C m_fAttackButtonCounter (f32), +0x70 m_nTargetBone, +0x74 m_vecTargetBoneOffset. Vehicle +0x42D bit 2 (raw;
  gr name not verified), +0x460 m_pDriver, +0x594 m_nVehicleSubType (3 heli, 4 plane). CWeaponInfo +0x08 m_fWeaponRange,
  +0x18 flags (bit 0 bCanAim), +0x22 m_nDamage (read as i16, movsx), +0x38 m_fAccuracy, +0x40 m_fAnimLoopStart, +0x44 m_fAnimLoopEnd.
  CTaskSimpleUseGun +0xE (byte; gr layout puts m_SkipAim there, unverified).
- Common sub-sequences used below:
  * LOS(a, b) = `ProcessLineOfSight(a, b, &cp, &hitEnt, 1,1,1,1,1, 0, 0, 1)` (buildings, vehicles, peds, objects, dummies,
    seeThrough 0, ignoreSomeObjects 0, shootThrough 1); always preceded by `HandleGunShot(a, b); GunShotSetsOilOnFire(a, b)`.
  * "E = S + v*R" pattern (0x73FF16, 0x74082C, 0x7409B1): `st=R; t=v.x*R (reg); E.y=v.y*R (f32); E.z=R*v.z (f32); E.x=S.x+t;
    E.y=E.y+S.y; E.z=S.z+E.z` (f32 stores).
  * Wobble W(U, V, r) (0x7400E7-0x7401D5 and 0x74057F-0x740676): `a = (float)time_u32 * fPlayerAimRotRate` (fild of the dword; if it is
    negative as i32, `+= 4294967296.0`; fmul f32). `uz = U.z*r` (reg), `uy = U.y*r` (f32), `ux = U.x*r` (f32); `s = fsin(a)` stored f32;
    `E.x = ux*s + E.x`; `E.y = E.y + uy*s`; `zacc = uz*s + E.z` (kept in a register, not stored); `vz = V.z*r` (reg), `vy = V.y*r` (f32),
    `vx = V.x*r` (f32); `c = fcos(a)` stored f32 (fcos of the same unrounded a); `E.x = vx*c + E.x`; `E.y = E.y + vy*c`;
    `E.z = vz*c + zacc`. Then `pd->m_fAttackButtonCounter = (float)(i16)wi->m_nDamage * 0.04f + pd->m_fAttackButtonCounter`
    (fild, fmul 0x858CEC, fadd, fstp) with pd = firer->m_pPlayerData.
  * Scatter(k) (0x74024F-0x7402EA and 0x740909-0x7409A8): `t1 = (float)rand()*k1` (f32 store), `t2 = (float)rand()*k1` (f32 store),
    `t3 = (float)rand()*k1` (reg), k1 = 3.0518509e-05; `ox = t3*0.4f - 0.2f`, `oy = t2*0.4f - 0.2f`, `oz = (t1*0.2f - 0.1f)*acc` (f32);
    `E.x = ox*acc + E.x`, `E.y = E.y + oy*acc`, `E.z = oz + E.z`. The FIRST rand() goes to z, the second to y, the third to x.

Behaviour, block by block (original order):
1. Setup 0x73FB10-0x73FC49. `skill = (firer && type(firer)==3) ? firer->GetWeaponSkill(this->m_Type) : 1`; `wi = GetWeaponInfo(m_Type,
   skill)` (firer may be null here only). `M = *muzzle` (F+0x78), `S = *origin`; `E = (0,0,0)`; `hitEnt = 0`; `acc = 0`;
   if `originOverride` (0x73FBD2): `S = *originOverride`. `task = 0`. Then (firer dereferenced without null check from 0x73FBE8) if
   firer is a ped (0x73FBFB): `EBP = firer`; `acc = (100.0f - (float)m_nWeaponAccuracy) / wi->m_fAccuracy` (fld wi+0x38; fild u8;
   fsubr 100; fdiv st1; f32 store); if `m_pPlayerData != 0 && (+0x46C & 0x4000000)`: `acc = acc*0.5f`; `task = GetTaskUseGun(intel)`.
2. Shotguns 0x73FC4E: if `m_Type` is 25, 26 or 27: `acc = 0`; `fWeaponSpreadRate = SHOTGUN_SPREAD_RATE / wi->m_fAccuracy` (f32).
   (Other weapons leave fWeaponSpreadRate as it is; it is normally 0 because ResetLineTestOptions clears it.)
3. Task path 0x73FC7A-0x73FDD2: if `task && task->+0xE`: `E = (wi->range, 0, 0)` (also stored in T); transform E in place by
   `GetMatrixArray(h)[RpHAnimIDGetIndex(h, firer->m_apBones[6]->m_nNodeId)]`, `h = GetAnimHierarchyFromSkinClump(firer->+0x18)`
   (RwV3dTransformPoints(&E, &E, 1, mat)). `d = E - S` computed z, y, x (fld E.z; fsub S.z; fld E.y; fsub S.y; fld E.x; fsub S.x) stored
   x,y,z to T and copied to dir; `dir.Normalise()` (0x73FD42). Ignore entity (0x73FD47): if `(+0x46C & 0x100) && m_pVehicle` ->
   `pIgnoreEntity = m_pVehicle`; else if `m_pAttachedTo && type(m_pAttachedTo) == 2` -> `pIgnoreEntity = m_pAttachedTo`; else
   unchanged. `bIncludeBikers = 1`; HandleGunShot(&S,&E); oil(&S,&E); LOS(S, E) (0x73FDC2); jump to 0x740B6E (block 9).
4. Ped firer with an explicit target 0x73FDD7-0x740345. 0x73FDD7: `EBP == 0` (not a ped) -> block 6. If `targetEnt == 0 && target == 0`
   -> block 5. If `!(firer->+0x528 & 1) && !forceAim` (0x73FDF2/0x73FDFB) -> jump DIRECTLY to block 9 (0x740B71): no LOS, E stays
   (0,0,0), hitEnt 0, dir and cp uninitialised (see oddities).
   4a. Aim point (0x73FE0A): if `target`: `E = *target`. Else (targetEnt): ped (type 3): if `firer->m_pPlayerData` (note: the FIRER's
       player data): `E = pd->m_vecTargetBoneOffset; targetEnt->GetTransformedBonePosition(E, pd->m_nTargetBone, false)`; else
       `targetEnt->GetBonePosition(E, 3, false)`. Non-ped: `E = targetEnt->GetPosition()`.
   4b. Direction (0x73FE81): `dx = E.x - S.x` (reg), `E.y = E.y - S.y` (f32 store), `dz = E.z - S.z` (reg);
       `q = (dx*dx + dy*dy) + dz*dz` (dy read back as f32); `len = sqrt(q)`; if `len < 0.01f` (strict; NaN -> sqrt branch) `len = 0.01f`
       (fld 0x858C58), else `len = fsqrt(q)` again; `inv = 1.0f / len` (fdivr); `E = (inv*dx, dy*inv, inv*dz)` (f32) and `dir = E`.
       `R = TargetWeaponRangeMultiplier(targetEnt, firer) * wi->range` (ST0 * f32 copy of range); "E = S + E*R" pattern (0x73FF16).
   4c. Player scatter (0x73FF1E): if `firer->m_pPlayerData` and `acc != 0.0f` (0x73FF6F: `fcomp 0.0; test ah,0x44; jnp` -> only exact
       equality skips; NaN and negatives continue):
       `k = 5.0f / range`; `scale = (1.0f < k) ? 1.0f : 5.0f/range` (fcompp 1.0 vs k; recomputed by fdivr when not taken; NaN -> recompute);
       `t = ((scale*acc) * pd->m_fAttackButtonCounter) * fPlayerAimScale`; `acc = t` (fst f32); `h = t*0.5f`;
       `lo = (0.2f < h) ? 0.2f : h` (stored f32); `r = GetRandomNumberInRange(lo, acc)` (cdecl, min = lo, max = acc; one RNG draw inside
       0x41BD90); `acc = r` (f32). Basis (mode read from Cams[0] at 0xB6F1A8, NOT the active cam): if mode is 0x35 or 0x37:
       `B = Cams[0].m_vecUp` (unnormalised), `A = Cross(Cams[0].m_vecFront, B)`, `A.Normalise()`. Else: `A = Cross(E, (0,0,1))`
       (E is the END POINT, not a direction; original behaviour), `A.Normalise()`, `B = Cross(E, A)`, `B.Normalise()`.
       Then W(A, B, acc) (sin with A, cos with B). -> 4e.
   4d. Non-player scatter (0x7401DA): only if `acc > 0.0f` (`test ah,0x41; jne` skips <=, NaN). If `targetEnt` is a ped and
       `targetEnt->IsPlayer()`: `m = |targetEnt->m_vecMoveSpeed|` (Magnitude); `acc = ((m > 0.33f ? 0.33f : Magnitude() again) *
       0.90909094f + 0.8f) * acc` (`fcomp 0.33; test ah,0x41; jne` -> m <= 0.33 or NaN calls Magnitude a second time). Then Scatter
       (3 rand() draws).
   4e. Ignore entity (0x7402EE): if `(+0x46C & 0x100) && m_pVehicle && !(m_pVehicle->+0x42D & 4)` -> `pIgnoreEntity = m_pVehicle`;
       else if `m_pAttachedTo && type == 2 && !(m_pAttachedTo->+0x42D & 4)` -> `pIgnoreEntity = m_pAttachedTo`; else unchanged.
       If `firer->IsPlayer()`: `bIncludeDeadPeds = 1`. -> block 8 (0x740B1D).
5. Ped firer without target 0x74034A-0x7407BC. If `!firer->IsPlayer()` -> block 6. Mode of the ACTIVE camera
   (`Cams[m_nActiveCam].m_nMode`) must be 0x35, 0x37, 0x41 or 0x31, else -> block 6.
   `Find3rdPersonCamTargetVector(wi->range * 3.0f, S, T, E)` (T = camera source out, E = aim point out). `d = E - S` (z, y, x order,
   stored to A then dir); `dir.Normalise()`. E itself is not changed.
   If `acc != 0.0f` (0x740414, same test as 4c): `k = (5.0f / range) * 3.0f`; `scale = (1.0f < k) ? 1.0f : (5.0f/range)*3.0f`;
   `acc = ((scale*acc) * pd->m_fAttackButtonCounter) * fPlayerAimScale` (no 0.5, no GetRandomNumberInRange in this block). Basis by
   Cams[0].m_nMode: 0x35/0x37 -> `A = Cams[0].m_vecUp`, `B = Cross(Cams[0].m_vecFront, A)`, `B.Normalise()`; 0x31 ->
   `Cams[0].Get_TwoPlayer_AimVector(F+0x84)` (result is overwritten unread at 0x7405BF, only side effects matter), then as 0x35/0x37
   but the cross product is written via F+0x54 (same values); otherwise -> `B = Cross(E, (0,0,1))`, `B.Normalise()`, `A = Cross(E, B)`,
   `A.Normalise()`. Then W(B, A, acc) (sin with B, cos with A).
   Ignore entity (0x740679): if `(+0x46C & 0x100) && m_pVehicle && !(m_pVehicle->+0x42D & 4)` -> m_pVehicle; else if
   `m_pAttachedTo && type 2 && !(+0x42D & 4)` -> m_pAttachedTo; else `pIgnoreEntity = firer` (differs from 4e).
   `bIncludeDeadPeds = bIncludeCarTyres = bIncludeBikers = 1`; HandleGunShot(&T,&E); oil(&T,&E); LOS(T, E) from the CAMERA SOURCE.
   If `hitEnt` (0x74072D): `dist = sqrt(dy*dy + dx*dx)` with `dy = cp.y - T.y`, `dx = cp.x - T.x` (2D, f32 store);
   `if (TargetWeaponRangeMultiplier(hitEnt, firer) * range < dist) hitEnt = 0` (`test ah,5; jp` -> >= or NaN keeps it);
   if still hit: `CheckForShootingVehicleOccupant(&hitEnt, &cp, m_Type, &T, &E)` (0x7407B4). -> block 9 (0x740B71).
6. Vehicle firer 0x7407C1-0x7409F8 (type(firer) == 2; else block 7): `acc = 0.6f`; ensure matrix; `E = dir = firer.forward`.
   If status (`+0x36 >> 3`) is 0 (PLAYER) or 8 (REMOTE_CONTROLLED): "E = S + forward*range"; `aimVeh = (m_nVehicleSubType == 4 ||
   == 3)` (byte at F+0x24; the dword is pushed, upper bytes stale); `owner = (status == 8) ? FindPlayerPed(-1) : firer->m_pDriver`;
   `DoDriveByAutoAiming(owner, firer, &S, &E, aimVeh)`; `E = E - S` (x, y, z, each fld/fsub/fstp); `E.Normalise()`;
   `acc = (model == 447 || model == 469 || model == 564) ? 0.1f : 0.3f` (u16 compares 0x1BF, 0x1D5, 0x234).
   Then (all statuses, 0x740909): Scatter (3 rand() draws); `E.Normalise()`; `pIgnoreEntity = firer`; "E = S + E*range";
   -> 0x740B24 (block 8 WITHOUT setting bIncludeBikers).
7. Other firers 0x7409FD-0x740B17 (peds that reached here from block 4/5, objects, etc.): `F+0x20 = range`; ensure matrix;
   `E = M + forward*range`, x87: `z = range*fwd.z` (reg), `y = range*fwd.y` (f32), `x = range*fwd.x` (f32); `z = z + M.z`;
   `y = M.y + y`; `x = x + M.x`; stored x,y,z (via T) -> E. (The start of the shot stays S, the end is relative to the MUZZLE copy.)
   A second ensure-matrix at 0x740A7E is dead (the matrix pointer is non-null by then). `dir = forward`.
   If EBP (ped): if `(+0x470 & 0x2000000)` and `!(firer->IsPlayer() && (wi->+0x18 & 1))`: `DoDoomAiming(firer, &S, &E)` (0x740AD9).
   Ignore entity (0x740AE1): if `(+0x46C & 0x100) && m_pVehicle` -> m_pVehicle (no +0x42D test); else if `m_pAttachedTo` ->
   `pIgnoreEntity = m_pAttachedTo`, then if its type != 2 `pIgnoreEntity = firer`; else (no attachedTo) firer. Non-peds: firer.
8. Generic LOS 0x740B1D: `bIncludeBikers = 1`; (0x740B24) HandleGunShot(&S,&E); oil(&S,&E); LOS(S, E); `add esp,0x40` (0x740B6E).
9. Events and effects 0x740B71-0x740F66.
   - `silenced = (m_Type == 0x17 || m_Type == 0x11)`; `shot = CEventGunShot(firer, S, E, silenced)`; `GetEventGlobalGroup()->Add(&shot, false)`.
   - `whizz = CEventGunShotWhizzedBy(firer, S, E, m_Type == 0x17)` (only the silenced pistol counts here) and Add(&whizz, false);
     unconditional. `g_InterestingEvents.Add(0x16, firer)`. If EBP: `firer->IsPlayer()` is called and its result discarded (0x740C3D).
   - 0x740C42: `S = *origin` again (the override from block 1 is dropped from here on).
   - Muzzle FX (0x740C55): if `doMuzzleFx` and `(u32)(m_Type - 22) <= 16`, table (bytes 0x741348, targets 0x741334):
     types 22,23,24,34 -> (a,b) = (0.2, 0.25); 25,26,27 -> (0.3, 0.45); 28,29,32 -> (0.2, 0.3); 30,31,38 -> flash test then
     (0.65, 0.25); 33,35,36,37 -> no FX. Flash test (0x740C8C): `n = _ftol2((wi->m_fAnimLoopEnd - wi->m_fAnimLoopStart) * 900.0f)`;
     if `n < 50`: `++byte[0xC8A80C]`; if the new value is odd -> no FX this shot. (a, b) are written to F+0x1C/F+0x20.
     `AddLight(0, M, (0,0,0), 3.0f, 0.25f, 0.22f, 0.0f, 0, false, nullptr)` (the zero vector is built in T; light colour is fixed).
     `gunflash = !(type(firer) == 3 && firer->m_pGunflashObject (+0x4F8))`; `g_fx.TriggerGunshot(firer, M, dir, gunflash)`.
     Shell: `G.x = M.x - dir.x*a` (fld dir.x; fmul a; fsubr M.x), same for y, z (stored to A); ensure matrix;
     `AddGunshell(firer, G, CVector2D(firer.right.x, firer.right.y), b)`.
   - Water (0x740E3A): with targetEnt: only if `1 < type(targetEnt) < 5` and `(targetEnt->+0x40 & 0x100)` -> test. Without targetEnt:
     if EBP && firer->IsPlayer() and `E.z < S.z` (`test ah,5; jnp`) -> test; otherwise (also when that compare fails) the status of
     the firer must be 0 or 8 and `E.z < S.z` (`jp` skips >= / NaN) -> test.
     Test (0x740EA3): if hitEnt: `0x6E61B0(S, cp.point, &T)`; if false -> block 10 (no second test). If no hitEnt:
     `0x6E61B0(S, E, &T)`; if false -> block 10. On true: `g_fx.TriggerBulletSplash(T)`;
     `AudioEngine.ReportBulletHit(nullptr, 39 (0x27, SURFACE_WATER_SHALLOW), T, 0.0f)`.
10. Impact 0x740F66-0x7412E4. `retry = 0` (F+0x24). Single impact (0x7412C2) if `!(fWeaponSpreadRate > 0.0f)` (`test ah,0x41`), or
   `hitEnt == 0`, or hitEnt is a vehicle and `13 <= cp.pieceTypeB <= 16` (unsigned byte, wheels):
   `DoBulletImpact(firer, hitEnt, M, E, cp, 0)` (start = MUZZLE copy). Otherwise pellet loop (0x740FB0):
   - `++retry`; `n = (m_Type == 27) ? SPAS_NUM_PELLETS : SHOTGUN_NUM_PELLETS` (int, F+0x20); pelletMat attach fields (F+0x138,
     F+0x13C) = 0; state 2; `SetUpPelletCol(n, firer, hitEnt, S, cp, pelletMat)`; `dists[0..14] = 1.0f` (rep stosd 15 dwords).
   - `cm = (type(hitEnt)==3) ? ms_modelInfoPtrs[hitEnt->m_nModelIndex]->AnimatePedColModelSkinned(hitEnt->m_pRwClump) :
     hitEnt->GetColModel()`; ensure hitEnt matrix; `ProcessColModels(pelletMat, ms_PelletTestCol, *hitEnt->m_matrix, *cm,
     m_aTempColPts, m_aTempColPts, dists, false)`.
   - Count (unrolled by 4 at 0x7410C0, remainder 0x741130; equivalent to): `hits = 0; last = 0; for i in 0..n: if dists[i] < 1.0f
     { hits++; last = i }` (NaN not counted). F+0x1C = hits.
   - For b in 0..n-1 (0x741160): if `dists[b] < 1.0f`: `DoBulletImpact(firer, hitEnt, M, m_aTempColPts[b].point, m_aTempColPts[b],
     (b == last) ? -hits : 1)`.
   - If hitEnt is neither ped nor vehicle (0x7411AA): `DoBulletImpact(firer, hitEnt, M, E, cp, 0)`; `hitEnt = 0`.
     Ped/vehicle (0x741211): retry when `(dists[0] == 1.0f || (float)hits / n < 0.5f)` (fild/fidiv; `== 1.0` test via `ah&0x44, jnp`;
     NaN dists[0] goes to the ratio test) and `retry < 2`: `pIgnoreEntity = hitEnt; S = cp.point; hitEnt = 0`;
     HandleGunShot(&S,&E); oil(&S,&E); LOS(S, E) (0x7412B5); no extra impact. Otherwise `hitEnt = 0`.
   - 0x7411EC: state 1; `pelletMat.~CMatrix()`; if `hitEnt` (only possible after a retry LOS hit) -> loop again from 0x740FB0 (no
     spread / wheel re-check); else exit. At most one retry.
11. Exit 0x7412E4: `ResetLineTestOptions()`; `~CEventGunShotWhizzedBy(whizz)`; state -1; `~CEventGunShot(shot)`; return 1.
- RNG draws per path (rand 0x821B1E unless noted): block 4c: 1 (inside GetRandomNumberInRange 0x41BD90); 4d: 3; 5: 0;
  6: 3; 3/7: 0; plus whatever callees draw (DoDriveByAutoAiming, DoDoomAiming, SetUpPelletCol, DoBulletImpact, Fx, AddGunshell).
- Oddities / original bugs (keep): (a) 4c/5 cross products use the end POINT E with (0,0,1), not a direction (0x74003D, 0x7404D5).
  (b) 4c/5 read Cams[0].m_nMode directly while block 5's entry test uses the active cam. (c) Block 4 early exit (0x73FE04): E =
  (0,0,0) goes into both events, dir (F+0x6C) is uninitialised stack when doMuzzleFx is set, cp is uninitialised and passed to the
  final DoBulletImpact (hitEnt 0). (d) Block 5 traces from the camera source but events/effects use S. (e) Block 7 end point is
  relative to *muzzle, block 6/4 relative to S. (f) S is reset to *origin at 0x740C42 before the water test and the pellet setup.
  (g) The bool args at 0x74089D (DoDriveByAutoAiming) and 0x740D8F (TriggerGunshot) are pushed as dwords from F+0x24 whose upper
  bytes are stale. (h) Dead IsPlayer call 0x740C3D; dead Get_TwoPlayer_AimVector result; dead ensure-matrix 0x740A7E.
  (i) dists has 15 slots; SHOTGUN_NUM_PELLETS/SPAS_NUM_PELLETS > 15 would overflow (values 15/4 in .data).
- gta-reversed: `RH_ScopedInstall(FireInstantHit, 0x73FB10, { .Reversed = false })`; the body is only `plugin::CallMethodAndReturn`
  plus six unused constexpr constants. Differences: PLAYER_ANIM_ROT_RATE = 0.0062832f is a different f32 from the .data value
  0.0062831854 (2*pi/1000) - port must read 0x8D6118; the other constants match .data. No C++ logic exists to compare.
- Corrections to the first-wave block notes: scatter draw order is z, y, x (first rand -> z), not x first; the 0.33/0.909/0.8 factor
  applies when the TARGET is a player ped; block 6 tests entity STATUS 0/8 (not "subtype 8") and passes `subtype in {3,4}` as
  canAimVehicles; block 5 has no GetRandomNumberInRange draw and its LOS starts at the camera source; CEventGunShotWhizzedBy is
  always raised; the per-weapon pairs are gunshell offset/size, the light colour is constant (0.25, 0.22, 0); DoBulletImpact's start
  is the muzzle copy M; the event silenced flag for the whizz event is type 0x17 only.
- Open questions: (1) 0x73FC89: meaning of CTaskSimpleUseGun +0xE (gr: m_SkipAim) - behaviour exact either way. (2) 0x740303/0x74068E:
  name of vehicle flag +0x42D bit 2. (3) 0x740AB1: +0x470 bit 25 named bDoomAim by counting gr bitfields (gr comments are off by
  one around bIsDucking); value exact. (4) Uninitialised dir/cp in the 0x73FE04 early-exit path cannot be reproduced bit-exactly;
  a port must pick a value (stack garbage in the original).
  Confidence: medium (all blocks transcribed from the bytes; names of a few flags unverified).
