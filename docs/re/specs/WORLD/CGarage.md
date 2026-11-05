# CGarage (WORLD) - full specs, writer E (RE-SPEC-1)

## Common notes

Layout (gta-reversed `Garage.h`, VALIDATE_SIZE 0xD8; all offsets confirmed by the bytes):
+0x00 CVector m_vPosn (corner), +0x0C CVector2D m_vDirectionA (unit), +0x14 CVector2D m_vDirectionB
(unit), +0x1C f32 m_fTopZ, +0x20 f32 m_fWidth (length along A), +0x24 f32 m_fHeight (length along B),
+0x28 f32 Left (min x), +0x2C Right (max x), +0x30 Front (min y), +0x34 Back (max y) (axis-aligned bounds),
+0x38 f32 m_fDoorPosition (0 closed .. 1 open), +0x3C u32 m_nTimeToOpen, +0x40 CVehicle* m_pTargetCar,
+0x44 char m_anName[8], +0x4C u8 m_nType, +0x4D u8 m_nDoorState, +0x4E u8 m_nFlags, +0x4F u8
m_nOriginalType, +0x50 CAEDoorAudioEntity m_GarageAudio (0x88: vtable +0x50, m_Entity +0x54,
CAESound m_tempSound +0x58..+0xCB, m_nTime +0xCC, m_nMovingDoorTime +0xD0, m_nField84 +0xD4).

m_nFlags bits (gr names): 0x01 m_b0x1 (used by Update as "close-without-delivery" marker), 0x02 m_bInactive,
0x04 m_bUsedRespray, 0x08 m_bDoorOpensUp, 0x10 m_bDoorGoesIn, 0x20 m_bCameraFollowsPlayer, 0x40
m_bDoorClosed, 0x80 m_bRespraysAlwaysFree.

Door states: 0 CLOSED, 1 OPEN, 2 CLOSING, 3 OPENING, 4 WAITING_PLAYER_TO_EXIT, 5 CLOSED_DROPPED_CAR.
Types: see gr `eGarageType` (1 ONLY_TARGET_VEH, 2/3/4 BOMBSHOP_TIMED/ENGINE/REMOTE, 5 PAYNSPRAY,
11 UNKN_CLOSESONTOUCH, 14 OPEN_FOR_TARGET_FREEZE_PLAYER, 15 SCRIPT_ONLY_OPEN, 16..18 and 24..32, 39..42
safehouses, 19 SCRIPT_CONTROLLED, 20 STAY_OPEN_WITH_CAR_INSIDE, 21 CLOSE_WITH_CAR_DONT_OPEN_AGAIN,
23 SCRIPT_OPEN_FREEZE_WHEN_CLOSING, 33..35 IMPOUND_LS/SF/LV, 36..38 tuning, 43 BURGLARY, 44 HANGAR_AT400,
45 HANGAR_ABANDONED_AIRPORT). Type 13 is unnamed in gr but special-cased by the bytes.

Other layouts used: CEntity +0x14 CMatrix* m_matrix (NULL possible), +0x04 placement position (used when
m_matrix is NULL: "entity position" below always means `m_matrix ? &m_matrix->pos (+0x30) : &ent+4`),
+0x22 i16 m_nModelIndex, +0x36 status byte (`(b & 0xF8) == 0x28` means status 5 = STATUS_WRECKED).
CMatrix: right +0x00, forward +0x10, up +0x20 (up.z = +0x28), pos +0x30. CPhysical +0x44 m_vecMoveSpeed.
CVehicle: +0x428.. flag bytes (gr bit order: +0x428 bit 0x08 bIsLocked, +0x42D bit 0x20 bImpounded,
+0x42E bit 0x80 bDisableParticles, +0x42F bit 0x01 bHasBeenResprayed), +0x434..0x437 colours 1..4,
+0x4A4 u8 m_nCreatedBy (2 = MISSION_VEHICLE), +0x4A8 bits 0..2 m_nBombOnBoard, +0x4B0 f32 m_fDirtLevel,
+0x4C0 f32 m_fHealth, +0x4CC m_pWhoInstalledBombOnMe, +0x590 i32 vehicle class (0 automobile, 9 bike),
+0x594 i32 subclass (9 bike, 10 BMX). CAutomobile +0x868 autoFlags (bit 0x02 bShouldNotChangeColour),
+0x8E4 f32 m_fBurningTimer. CBike +0x7BC f32 m_fBurningTimer. CColModel (from CEntity::GetColModel
0x535300) +0x2C CCollisionData*: +0x00 i16 m_nNumSpheres, +0x08 CColSphere* (stride 0x14: centre +0x0,
radius +0xC). CBaseModelInfo +0x14 CColModel*, CColModel +0x24 bounding-sphere radius.
Vehicle pool: `CPools::ms_pVehiclePool` (0xB74494): +0 objects base, +4 flag bytes (sign bit = free slot),
+8 size; object stride 0xA18. Unless stated, pool loops run from index size-1 DOWN to 0.

x87: precision control 24 bit. Compare idioms: `test ah,5; jp X` = X when NOT(a < b) (NaN goes to X);
`test ah,5; jnp X` = X when a < b; `test ah,0x41; je X` = X when a > b; `test ah,0x41; jne X` = X when
a <= b or unordered. "Entity sphere loop" = for each col sphere i: `w = (*matrix) * sphere[i].centre`
via `CMatrix::operator*(const CMatrix&, const CVector&)` 0x59C890 (cdecl, hidden result pointer), then a
call to `IsPointInsideGarage(w, r_i)` 0x4487D0.

Register preservation the ORIGINAL callers rely on (relevant while some functions are still original):
0x4487D0 must preserve ECX and EDX (callers 0x448BE0, 0x448EE0, 0x449BA0, 0x44A240 keep the sphere array /
garage pointer in EDX/ECX across it); 0x448740 must preserve EDX (callers 0x448990, 0x449C50 keep
pointers in EDX). Both originals only touch EAX.

Proposed names for the 13 unnamed helpers come from gr `Garage.h` declarations of not-yet-reversed methods
(signatures match), checked against what the code computes and how CGarage::Update uses it.

## 0x4470E0 CGarage::CGarage  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CGarage* (this = ECX)`; returns EAX = this; plain `ret`.
- Callers: `__init_CGarages__aGarages` 0x849E90 (array construction). Callees: none.
- Behaviour (order): `*(u32*)(this+0x60) = 0` (m_GarageAudio.m_tempSound.m_PhysicalEntity);
  `*(u32*)(this+0x50) = 0x85998C` (CAEDoorAudioEntity vtable); `+0xCC = 0; +0xD0 = 0; +0xD4 = 0`.
  Nothing else is initialised (aGarages is in .bss, zero at start).
- gta-reversed: `CGarage() = default`; the inline member defaults of CAESound/CAEAudioEntity in gr
  initialise more fields. Port only these 5 stores.
- Open questions: none.

## 0x447110 CGarage::~CGarage  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall void (this = ECX)`; tail jump.
- Callers: `__init_CGarages__aGarages` 0x849E90 (registered for array unwind), `__dtor_CGarages__aGarages`
  0x856130. Callees: 0x4EF660 (catalogue "CAESound", the CAESound destructor) by tail call.
- Behaviour: `*(u32*)(this+0x50) = 0x859988` (CAEAudioEntity base vtable); `ECX = this + 0x58`;
  `jmp 0x4EF660` (destroys m_GarageAudio.m_tempSound).
- gta-reversed: `= default`. Same effect.
- Open questions: none.

## 0x447600 CGarage::InitDoorsAtStart  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (this = ECX)`; plain `ret`.
- Callers: CGarages::Init_AfterRestart 0x448B60. Callees: none.
- Behaviour:
  1. `m_nFlags = (m_nFlags & 0x39) | 0x40` (clears 0x02 inactive, 0x04 used-respray, 0x80; sets 0x40).
  2. `m_nDoorState = 0; m_nTimeToOpen = 0`.
  3. Switch on m_nType (index type-1 into 0x44764C, table 0x447640):
     - types 2,3,4,5: `m_nDoorState = 1 (OPEN); m_fDoorPosition = 1.0f`.
     - type 13: nothing more (doorPos untouched).
     - types 1, 6..12, 14..45: `m_nDoorState = 0; m_fDoorPosition = 0.0f`.
     - type 0 or > 45: nothing more (doorPos untouched).
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x447720 CGarage::RightModTypeForThisGarage (proposed; was sub_447720)  (catalogue: unknown/REA, port level 0)
- Evidence for the name: gr Garage.h declares `bool RightModTypeForThisGarage(CVehicle*)`; the body tests
  the vehicle's handling flags LOW_RIDER / STREET_RACER against the three tuning garage types; Update
  calls it only for types 36..38.
- Convention: `__thiscall bool (this = ECX, CVehicle* veh = [esp+4])`, AL, `ret 4`. Body .HOODLUM 0x1565260.
- Callers: CGarage::Update (2 sites). Callees: none.
- Globals: R `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8; R `gHandlingDataMgr.m_aVehicleHandling` 0xC2B9DC
  (= 0xC2B9C8 + 0x14), stride 0xE0.
- Behaviour:
  1. veh == NULL -> false.
  2. `mi = ms_modelInfoPtrs[(i16)veh->m_nModelIndex]`; `h = &m_aVehicleHandling[(u8)mi[+0x4A]]` (only the
     LOW BYTE of CVehicleModelInfo::m_nHandlingId is used, movzx byte); `f = *(u32*)(h + 0xD0)`
     (m_nHandlingFlags).
  3. type 36 (LOCO LOW CO): true iff `f & 0x2000000` (LOW_RIDER). type 37 (WHEEL ARCH ANGELS): true iff
     `f & 0x4000000` (STREET_RACER). type 38 (TRANSFENDER): true iff `(f & 0x6000000) == 0`. Other types:
     false.
- Open questions: none.

## 0x447D80 CGarage::CalcDistToGarageRectangleSquared (proposed; was sub_447d80)  (catalogue: unknown/REA, port level 0)
- Evidence: gr declares `float CalcDistToGarageRectangleSquared(float, float)`; the body is exactly a
  squared 2D distance to the axis-aligned rectangle Left/Right/Front/Back.
- Convention: `__thiscall float (this = ECX, float x = [esp+4], float y = [esp+8])`, result in ST0,
  `ret 8`.
- Callers: CGarage::Update (7 sites). Globals: R 0x858B50 f32 0.0.
- Behaviour:
  - `dx = (x < Left) ? x - Left : (x > Right) ? x - Right : 0.0f` (NaN x -> 0.0f; the "x > Right" test is
    `jne` on 0x41, so x == Right -> 0).
  - `dy` likewise with Front (+0x30) / Back (+0x34).
  - Result `ST0 = dy*dy + dx*dx` (dy squared first, then `fadd` of dx*dx; values stay in x87 registers,
    not rounded to f32 memory).
- Open questions: none.

## 0x448330 CGarage::NeatlyLineUpStoredCars (proposed; was sub_448330)  (catalogue: unknown/REA, port level 1)
- Evidence: gr declares `void NeatlyLineUpStoredCars(CStoredCar*)`; the body overwrites the stored cars'
  positions and packed forward vectors to place them in a row in the garage; Update calls it on the
  impound safehouse array right before RestoreCarsForThisImpoundingGarage.
- Convention: `__thiscall void (this = ECX, CStoredCar* cars = [esp+4])`, `ret 4`.
- Callers: CGarage::Update (impound, CLOSED). Callees: CVector::Normalise 0x59C910 (thiscall),
  `_ftol` 0x821B40 (CRT, truncation toward zero, input ST0, result EAX).
- Globals: R 0x858B8C f32 0.5, 0x858B50 f32 0.0, 0x858B90 f32 4.0, 0x858628 f32 100.0.
- Behaviour (all temporaries f32 in memory unless "reg"):
  1. `pz = posn.z + 0.5f`.
  2. `ax = (dirA.x*width)*0.5f`, `ay = (dirA.y*width)*0.5f`, `az(reg) = (width*0.0f)*0.5f`.
     `X(reg) = ax + posn.x`; `P.y = ay + posn.y`; `P.z = az + pz`.
  3. `bx = (height*dirB.x)*0.5f`, `by = (dirB.y*height)*0.5f`, `bz = (height*0.0f)*0.5f`.
     `P.x = bx + X`; `P.y = by + P.y`; `P.z = bz + P.z`. (P = garage centre at floor+0.5; the *0.0 terms
     only matter for inf/NaN sizes.)
  4. `V = (dirA.x, dirA.y, 0)`; `V.Normalise()`. `F = (V.y, -V.x, 0)` (kept in f32).
  5. `V4 = V * 4.0f` (each component, f32). `P = P - V4` (each component, f32).
  6. If `cars[0].m_wModelIndex == 0` return. Loop i = 0,1,2,... with the test order: top of loop
     `if (i >= 3) break;` then body, then `i++; if cars[i].m_wModelIndex == 0 break;`:
     `t = (float)i` (fild of the int); `cars[i].pos.x = V4.x*t + P.x`; `.y = V4.y*t + P.y`;
     `.z = t*V4.z + P.z`; `cars[i].packedFwdX (+0x3C) = (u8)_ftol(F.x*100.0f)`;
     `packedFwdY (+0x3D) = (u8)_ftol(F.y*100.0f)`; `packedFwdZ (+0x3E) = 0`.
- Edge cases: at most 3 cars are lined up (slot 3 untouched); a gap (model 0) stops the loop. Negative
  forward values wrap in the u8 (low byte of the int).
- Open questions: none.

## 0x448550 CGarage::RestoreCarsForThisHideOut (proposed; was sub_448550)  (catalogue: unknown/REA, port level 18)
- Evidence: gr declares `bool RestoreCarsForThisHideOut(CStoredCar*)`; Update calls it with the
  safehouse array of a hideout garage before opening; it clears bImpounded (the impound twin sets it).
- Convention: `__thiscall bool (this = ECX unused, CStoredCar* cars = [esp+4])`, AL, `ret 4`. Body
  .HOODLUM 0x156D8D0.
- Callees: CStoredCar::RestoreCar 0x447E40 (thiscall), CWorld::Add 0x563220 (cdecl).
- Behaviour: for k = 0..3: if `cars[k].m_wModelIndex != 0`: `v = cars[k].RestoreCar()`; if v != NULL:
  `v[+0x42D] &= ~0x20` (bImpounded = false); `CWorld::Add(v)`; `cars[k].m_wModelIndex = 0`.
  Then return true iff all 4 model indices are 0 (a car whose RestoreCar returned NULL stays stored and
  makes the result false).
- Open questions: none.

## 0x4485C0 CGarage::RestoreCarsForThisImpoundingGarage (proposed; was sub_4485c0)  (catalogue: unknown/REA, port level 18)
- Evidence: gr declares `bool RestoreCarsForThisImpoundingGarage(CStoredCar*)`; called by Update only in
  the impound branch; handles 3 slots, matching StoreCarInNearestImpoundingGarage's 3-car rotation.
- Convention: as 0x448550 (`ret 4`, this unused). Body .HOODLUM 0x1561490.
- Callees: RestoreCar 0x447E40, CWorld::Add 0x563220, CAutomobile::PlaceOnRoadProperly 0x6AF420,
  CBike::PlaceOnRoadProperly 0x6BEEB0.
- Behaviour: for k = 0..2: if model != 0: `v = RestoreCar()`; if v: `v[+0x42D] |= 0x20` (bImpounded);
  `CWorld::Add(v)`; `if (v->subclass (+0x594) == 0) CAutomobile::PlaceOnRoadProperly(v); else if
  (subclass == 9) CBike::PlaceOnRoadProperly(v);` model = 0. Return true iff models of slots 0..2 are all
  0 (slot 3 ignored).
- Open questions: none.

## 0x4486C0 CGarage::PlayerArrestedOrDied  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (this = ECX)`, plain `ret`. AL holds junk (gr declares a return of
  eGarageDoorState; the only caller ignores it).
- Callers: CGarages::PlayerArrestedOrDied 0x449E60.
- Behaviour: switch on m_nType (index type-1 via 0x448704, table 0x4486FC):
  - types 2,3,4,5,13: `if (s == 0 || s == 2 || s == 3) m_nDoorState = 3 (OPENING)` (s = door state; 1, 4,
    5.. unchanged).
  - types 1, 6..12, 14..45: `if (s >= 1 && s <= 3) m_nDoorState = 2 (CLOSING)`.
  - type 0 / > 45: nothing.
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x448740 CGarage::IsPointInsideGarage(CVector)  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall bool (this = ECX, CVector p by value: x [esp+4], y [esp+8], z [esp+0xC])`, AL,
  `ret 0xC`. Uses a 4-byte local and overwrites the p.x arg slot. Touches only EAX/ECX.
- Callers: 7 (IsPointWithinHideOutGarage, IsPointWithinAnyGarage, 0x4495F0, RemoveCarsBlockingDoorNotInside,
  StoreAndRemoveCarsForThisHideOut 0x449900, 0x449A50, TidyUpGarage).
- Behaviour (false on the first failing test):
  1. `p.z < posn.z` -> false. 2. `p.z > TopZ` -> false.
  3. `ty = p.y - posn.y` (f32); `tx = p.x - posn.x` (f32, value kept in reg as stored);
     `a = tx*dirA.x + ty*dirA.y` (reg). `a < 0.0f` -> false; `a > width` -> false.
  4. `b = tx*dirB.x + ty*dirB.y`. `b < 0.0f` -> false; `b > height` -> false.
  5. true. (NaN passes every test.)
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x4487D0 CGarage::IsPointInsideGarage(CVector, float)  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall bool (this = ECX, CVector p: x [esp+4], y [esp+8], z [esp+0xC], float r
  [esp+0x10])`, AL, `ret 0x10`. 8 bytes of locals. Touches only EAX (ECX/EDX preserved, see Common notes).
- Callers: 10 (IsEntityEntirelyInside3D, IsEntityEntirelyOutside, IsEntityTouching3D,
  EntityHasASphereWayOutsideGarage, 0x449100, 0x449220, 0x4494F0, IsPointInAGarageCameraZone,
  TidyUpGarageClose, FindGarageForObject).
- Behaviour:
  1. `(posn.z - r) > p.z` -> false. 2. `(r + TopZ) < p.z` -> false.
  3. `ty = p.y - posn.y` (f32), `tx = p.x - posn.x` (f32); `a = tx*dirA.x + ty*dirA.y` (reg);
     `nr = -r` stored f32. `a < nr` -> false. `a > (r + width)` -> false.
  4. `b = tx*dirB.x + ty*dirB.y`; `b < nr` -> false; `b > (r + height)` -> false.
  5. true.
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x448BE0 CGarage::IsEntityEntirelyInside3D  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall bool (this = ECX, CEntity* e = [esp+4], float r = [esp+8])`, AL, `ret 8`.
- Callers: IsStaticPlayerCarEntirelyInside, CGarage::Update (3 sites). Callees: CEntity::GetColModel
  0x535300, CPlaceable::GetMatrix 0x411990 (allocates if needed), operator* 0x59C890, 0x4487D0.
- Behaviour (P = entity position, re-evaluated before each test):
  1. `(Left - r) > P.x` -> false; `(r + Right) < P.x` -> false; `(Front - r) > P.y` -> false;
     `(r + Back) < P.y` -> false; `(posn.z - r) > P.z` -> false; `(r + TopZ) < P.z` -> false.
  2. `cd = GetColModel()->m_pColData`; NULL -> true; numSpheres <= 0 -> true.
  3. For i (i16) = 0 .. numSpheres-1: `w = GetMatrix() * sphere[i].centre`;
     if `!IsPointInsideGarage(w, r - sphere[i].radius)` -> false. (Note: r MINUS radius.)
  4. true.
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x448D30 CGarage::IsEntityEntirelyOutside  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall bool (this = ECX, CEntity* e = [esp+4], float r = [esp+8])`, AL, `ret 8`.
- Callers: 0x448E50 (2), CGarage::Update (3). Callees: GetColModel 0x535300, CPlaceable::AllocateMatrix
  0x54F560, CSimpleTransform::UpdateMatrix 0x54F1B0, operator* 0x59C890, 0x4487D0.
- Behaviour:
  1. Quick rejection: if `(Left - r) < P.x` AND `(r + Right) > P.x` AND `(Front - r) < P.y` AND
     `(r + Back) > P.y` (strict, NaN makes the chain fail) -> return false. Otherwise go on.
  2. `cd = GetColModel()->m_pColData`; NULL -> true.
  3. For i (i16, signed compare with numSpheres) = 0 ..: if `e->m_matrix == NULL`:
     `AllocateMatrix(); e->placement(+4).UpdateMatrix(e->m_matrix)` (checked every iteration);
     `w = (*m_matrix) * sphere[i].centre`; if `IsPointInsideGarage(w, r + sphere[i].radius)` -> false.
  4. true.
- Edge: Z is not part of the quick rejection.
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x448E50 CGarage::IsPlayerOutsideGarage (proposed; was sub_448e50)  (catalogue: unknown/REA, port level 3)
- Evidence: gr declares `bool IsPlayerOutsideGarage(float fRadius)`; body = IsEntityEntirelyOutside on the
  player's vehicle or ped.
- Convention: `__thiscall bool (this = ECX, float r = [esp+4])`, AL, `ret 4`. Body .HOODLUM 0x1569180.
- Callers: CGarage::Update (5). Callees: CWorld::FindPlayerVehicle 0x56E0D0 (cdecl, args (-1, 0)),
  FindPlayerPed 0x56E210 (-1), IsEntityEntirelyOutside 0x448D30.
- Behaviour: `if (FindPlayerVehicle(-1, false)) return IsEntityEntirelyOutside(FindPlayerVehicle(-1,
  false), r); else return IsEntityEntirelyOutside(FindPlayerPed(-1), r);`
- Open questions: none.

## 0x448EE0 CGarage::IsEntityTouching3D  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall bool (this = ECX, CEntity* e = [esp+4])`, AL, `ret 4`.
- Callers: 0x449100, 0x449220, 0x4494F0, RemoveCarsBlockingDoorNotInside, TidyUpGarageClose,
  IsGarageEmpty, Update (2). Callees: GetColModel, GetMatrix 0x411990, operator*, 0x4487D0.
- Behaviour:
  1. `r = ms_modelInfoPtrs[(i16)e->m_nModelIndex]->m_pColModel (+0x14)->boundRadius (+0x24)` (model-info
     col model, not GetColModel).
  2. Same six bound tests as IsEntityEntirelyInside3D step 1 with this r (false on failure).
  3. `cd = GetColModel()->m_pColData` (NO null check); numSpheres <= 0 -> false.
  4. For i (i32 counter, compared with sign-extended numSpheres each iteration): `w = GetMatrix() *
     centre`; if `IsPointInsideGarage(w, sphere[i].radius)` -> true.
  5. false.
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x449050 CGarage::EntityHasASphereWayOutsideGarage  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall bool (this = ECX, CEntity* e = [esp+4], float r = [esp+8])`, AL, `ret 8`.
- Callers: StoreAndRemoveCarsForThisHideOut 0x449900, 0x449A50. Callees: as 0x448D30.
- Behaviour: `cd = GetColModel()->m_pColData` (no null check); for i (i32 vs sign-extended count): ensure
  matrix (AllocateMatrix + UpdateMatrix if NULL, each iteration); `w = (*m_matrix) * centre`;
  if `!IsPointInsideGarage(w, r + sphere[i].radius)` -> true. After the loop -> false.
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x449100 CGarage::IsAnyOtherCarTouchingGarage (proposed; was sub_449100)  (catalogue: unknown/REA, port level 4)
- Evidence: gr declares `bool IsAnyOtherCarTouchingGarage(CVehicle* ignoredVehicle)`; pool scan excluding
  the argument, true when a car touches the garage.
- Convention: `__thiscall bool (this = ECX, CVehicle* ignore = [esp+4])`, AL, `ret 4`. Body .HOODLUM
  0x1566680.
- Callers: CGarage::Update (2, both with ignore = NULL).
- Behaviour: vehicle pool, index size-1 down to 0; skip free slots, NULL, `v == ignore`, wrecked status;
  if `IsEntityTouching3D(v)`: `cd = GetColModel()->m_pColData` (no null check); for each sphere:
  `w = (*v->m_matrix) * centre` (m_matrix used directly); if `IsPointInsideGarage(w, sphere.radius)` ->
  return true. End -> false.
- Open questions: none.

## 0x449220 CGarage::ThrowCarsNearDoorOutOfGarage (proposed; was sub_449220)  (catalogue: unknown/REA, port level 4)
- Evidence: gr declares `void ThrowCarsNearDoorOutOfGarage(CVehicle* ignoredVehicle)`; the body pushes
  touching cars outwards from the garage centre; Update calls it every frame while a door closes.
- Convention: `__thiscall void (this = ECX, CVehicle* ignore = [esp+4])`, `ret 4`. The function contains a
  .HOODLUM obfuscation detour (0x449226 -> 0x40209A -> 0x44922B) that only computes
  `EBP = *(u32*)(*(u32*)0x1560718 + *(u32*)0x5E7CBD)` = `*(0x00B74494)` = CPools::ms_pVehiclePool
  (flags preserved by pushfd/popfd).
- Callers: CGarage::Update (5). Callees: IsEntityTouching3D, GetColModel, operator*, 0x4487D0,
  CVector::Normalise 0x59C910.
- Globals: R 0x858B8C 0.5f, 0x858B38 f32 0.02 (0x3CA3D70A), CTimer::ms_fTimeStep 0xB7CB5C.
- Behaviour: pool size-1 down to 0; skip free, NULL, `v == ignore` (status not checked). If
  `IsEntityTouching3D(v)`: `cd = GetColModel()->m_pColData`; for sphere i: `w = (*m_matrix) * centre`;
  if `IsPointInsideGarage(w, 0.0f)` -> next sphere; else (sphere centre outside) do ONE push and leave the
  sphere loop:
  - `d.x = P.x - ((Right + Left)*0.5f)`, `d.y = P.y - ((Back + Front)*0.5f)` (f32), `d.z = 0`;
    `d.Normalise()`.
  - `kx = d.x*0.02f` (reg), `ky = d.y*0.02f` (reg), `kz = d.z*0.02f` (f32);
    `tx = kx*ts` (f32), `ty = ts*ky` (reg), `tz = kz*ts` (reg) with ts = CTimer::ms_fTimeStep;
    `moveSpeed.x = tx + moveSpeed.x`, `.y = ty + moveSpeed.y`, `.z = tz + moveSpeed.z` (each stored f32).
- Open questions: none.

## 0x4494F0 CGarage::IsAnyCarBlockingDoor (proposed; was sub_4494f0)  (catalogue: unknown/REA, port level 4)
- Evidence: gr declares `bool IsAnyCarBlockingDoor()`; true when a car touches the garage but not all its
  spheres are inside, i.e. it straddles the door; Update uses it before closing.
- Convention: `__thiscall bool (this = ECX)`, AL, plain `ret`. Body .HOODLUM 0x156D610.
- Behaviour: pool size-1 down to 0, skip free/NULL (no ignore, no status test); if
  `IsEntityTouching3D(v)`: for each sphere (no null check): `w = (*m_matrix) * centre`; if
  `!IsPointInsideGarage(w, sphere.radius)` -> return true. End -> false.
- Open questions: none.

## 0x4495F0 CGarage::CountCarsWithCenterPointWithinGarage (proposed; was sub_4495f0)  (catalogue: unknown/REA, port level 1)
- Evidence: gr declares `int32 CountCarsWithCenterPointWithinGarage(CVehicle* ignoredVehicle)`; body
  counts pool vehicles whose position passes IsPointInsideGarage.
- Convention: `__thiscall int32 (this = ECX, CVehicle* ignore = [esp+4])`, EAX, `ret 4`. Body .HOODLUM
  0x1561400.
- Behaviour: pool size-1 down to 0; skip free/NULL/`v == ignore`; `count += IsPointInsideGarage(P)` (the
  3-argument overload 0x448740). Return count.
- Open questions: none.

## 0x449690 CGarage::RemoveCarsBlockingDoorNotInside  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void (this = ECX)`, plain `ret`.
- Callers: CloseHideOutGaragesBeforeSave 0x44A170, Update. Callees: IsEntityTouching3D, 0x448740,
  CVehicle::CanBeDeleted 0x6D1180, CWorld::Remove 0x563280, virtual slot 0 (deleting destructor).
- Behaviour: pool size-1 down to 0; skip free/NULL; if `IsEntityTouching3D(v)` and
  `!IsPointInsideGarage(P)` and `!(v[+0x428] & 0x08)` (not bIsLocked) and `v->CanBeDeleted()`:
  `CWorld::Remove(v); v->vtbl[0](1)` (delete) and RETURN (at most one vehicle per call).
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x449A50 CGarage::StoreAndRemoveCarsForThisImpoundingGarage (proposed; was sub_449a50)  (catalogue: unknown/REA, port level 3)
- Evidence: gr declares `void StoreAndRemoveCarsForThisImpoundingGarage(CStoredCar*, int32 iMaxSlot)`;
  the body is byte-for-byte the same as StoreAndRemoveCarsForThisHideOut 0x449900 (compared instruction by
  instruction, only addresses differ); Update calls it only from the impound branch (with max 3).
- Convention: `__thiscall void (this = ECX, CStoredCar* cars = [esp+4], int32 maxSlot = [esp+8])`,
  `ret 8`.
- Callees: 0x448740, EntityHasASphereWayOutsideGarage 0x449050, CStoredCar::StoreCar 0x449760,
  CPlayerInfo::CancelPlayerEnteringCars 0x56E860 (this = &CWorld::Players[CWorld::PlayerInFocus],
  0xB7CD98 + u8[0xB7CD74]*0x190), CWorld::Remove 0x563280, vtbl[0](1).
- Behaviour:
  1. `cars[k].m_wModelIndex = 0` for k = 0..3.
  2. n = 0. Pool size-1 down to 0, skip free/NULL: if `IsPointInsideGarage(P)` and
     `v->m_nCreatedBy != 2`: `lim = (maxSlot < 4) ? maxSlot : 4` (signed); if `n < lim` and
     `!EntityHasASphereWayOutsideGarage(v, 1.0f)`: `cars[n].StoreCar(v); n++`. Then (always, for every
     inside non-mission vehicle) `Players[focus].CancelPlayerEnteringCars(v); CWorld::Remove(v);
     v->vtbl[0](1)`.
  3. If n < 4: `cars[k].m_wModelIndex = 0` for k = n..3.
- Edge: vehicles that do not fit are deleted, not stored.
- Open questions: none.

## 0x449C50 CGarage::TidyUpGarage  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (this = ECX)`, plain `ret`.
- Callers: CGarages::Update 0x44C8C0 (tail call). Callees: 0x448740, CWorld::Remove, vtbl[0](1).
- Behaviour: for i = size-1 DOWN TO 1 (slot 0 is never examined: `edi = size-1; if 0 stop; ...; edi--;
  while edi != 0`): skip free/NULL; require class (+0x590) == 0 or 9; `m = v->m_matrix` (no null test);
  if `IsPointInsideGarage(P)`: if wrecked status OR `m->up.z (+0x28) < 0.5f`: `CWorld::Remove(v);
  v->vtbl[0](1)`.
- Edges: slot 0 skipped (original bug, keep); a NULL m_matrix with a non-wrecked vehicle dereferences
  NULL+0x28; with pool size 0 the index wraps (not reachable in practice).
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x449D10 CGarage::TidyUpGarageClose  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void (this = ECX)`, plain `ret`.
- Callers: CGarages::Update (tail call). Callees: IsEntityTouching3D, GetColModel, operator*, 0x4487D0,
  CWorld::Remove, vtbl[0](1).
- Behaviour: i = size-1 DOWN TO 1 (slot 0 skipped, same loop shape as TidyUpGarage); skip free/NULL;
  require class 0 or 9 AND wrecked status; require `IsEntityTouching3D(v)`. Then:
  - if `m_nDoorState == 0`: remove and delete.
  - else: `flag = false`; `cd = GetColModel()->m_pColData`; if numSpheres <= 0 -> keep the vehicle; for
    every sphere (no early exit): `w = (*m_matrix) * centre`; if `!IsPointInsideGarage(w, radius)` ->
    flag = true. If flag: remove and delete.
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x44A660 CGarage::SlideDoorOpen (proposed; was sub_44a660)  (catalogue: unknown/REA, port level 6)
- Evidence: gr declares `bool SlideDoorOpen()`; body raises m_fDoorPosition toward 1 and emits
  AE_GARAGE_DOOR_OPENING/OPENED; Update moves to OPEN when it returns true.
- Convention: `__thiscall bool (this = ECX)`, AL, plain `ret`.
- Callees: 0x449FF0 (unnamed in the catalogue; gr's `FindDoorsWithGarage(CObject**, CObject**)`,
  thiscall ret 8; not in this assignment), CAEDoorAudioEntity::AddAudioEvent(event, CVector&, float,
  float) 0x4DC860 on `this+0x50`.
- Globals: R ms_fTimeStep, 0x8599AC f32 0.0011, 0x8599A8 f32 0.011, 0x858624 f32 1.0.
- Behaviour:
  1. `k = (type == 44 || type == 45) ? 0.0011f : 0.011f`; `doorPos = ts*k + doorPos` (f32 store).
  2. `d1 = d2 = NULL`. The test is `fcomp 1.0; test ah,1; jne moving` (C0 only), so NaN counts as
     "< 1" (still moving). If `doorPos >= 1.0f`: `doorPos = 1.0f`; `FindDoorsWithGarage(&d1, &d2)`; if d1: `AddAudioEvent(160
     /*AE_GARAGE_DOOR_OPENED*/, d1 position, 0.0f, 1.0f)`; return true.
  3. Else: `FindDoorsWithGarage(&d1, &d2)`; if d1: `AddAudioEvent(159 /*OPENING*/, d1 position, 0.0f,
     1.0f)`; return false.
- Open questions: none.

## 0x44A750 CGarage::SlideDoorClosed (proposed; was sub_44a750)  (catalogue: unknown/REA, port level 6)
- Evidence: gr declares `bool SlideDoorClosed()`; mirror of 0x44A660.
- Convention: as 0x44A660.
- Globals: 0x8599B4 f32 0.0013, 0x8599B0 f32 0.013, 0x858B50 0.0.
- Behaviour: `k = (type 44/45) ? 0.0013f : 0.013f`; `doorPos = doorPos - ts*k` (f32). If `doorPos <= 0.0f`
  (ordered): `doorPos = 0` (int store 0); FindDoors; if d1: AddAudioEvent(162 /*CLOSED*/, pos, 0.0f,
  1.0f); return true. Else (> 0 or NaN): FindDoors; if d1: AddAudioEvent(161 /*CLOSING*/, ...); false.
- Open questions: none.

## 0x44A830 CGarage::IsStaticPlayerCarEntirelyInside  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall bool (this = ECX)`, AL, plain `ret`. Contains a .HOODLUM detour 0x44A851 ->
  0x40214C (`ecx = [eax+0x590]`) -> 0x44A856.
- Callers: Update (3). Callees: FindPlayerVehicle (4 calls, (-1,0)), FindPlayerPed,
  CTaskManager::FindActiveTaskByType 0x681740, IsEntityEntirelyInside3D.
- Globals: 0x858B50 0.0, 0x858C58 f32 0.01, 0x858FC4 f32 0.0001.
- Behaviour:
  1. v = FPV; NULL -> false. If FPV.class != 0 and FPV.class != 9 -> false.
  2. `FindPlayerPed(-1)->m_pIntelligence (+0x47C) ->m_TaskMgr (+4) .FindActiveTaskByType(704
     /*TASK_COMPLEX_LEAVE_CAR*/)` non-NULL -> false.
  3. v = FPV; P = v position. `P.x < Left` -> false; `P.x > Right` -> false; `P.y < Front` -> false;
     `P.y > Back` -> false.
  4. For c in moveSpeed x, y, z: `a_c = (c < 0) ? -c : c` (fchs in register; NaN unchanged); `a_c > 0.01f`
     -> false.
  5. `s = (az*az + ay*ay) + ax*ax`; `s > 0.0001f` -> false.
  6. return `IsEntityEntirelyInside3D(v, 0.0f)`.
- gta-reversed: plugin::CallMethod only.
- Open questions: none.

## 0x44A9C0 CGarage::IsGarageEmpty  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall bool (this = ECX)`, AL, plain `ret`.
- Callers: Update (2). Callees: CWorld::FindObjectsIntersectingCube 0x568DD0 (cdecl, 10 args),
  IsEntityTouching3D.
- Behaviour: `A = (Left, Front, posn.z)`, `B = (Right, Back, TopZ)` (local f32 copies); `i16 n;
  CEntity* list[16]`; `FindObjectsIntersectingCube(&A, &B, &n, 16, list, false /*buildings*/, true
  /*vehicles*/, true /*peds*/, false /*objects*/, false /*dummies*/)`. For i (i16) < n: if
  `IsEntityTouching3D(list[i])` -> false. -> true.
- gta-reversed: Garage.cpp:203 has this exact code as dead code after plugin::CallMethod; it matches.
- Open questions: none.

## 0x44AA50 CGarage::Update  (catalogue: specified/REA, port level 19)
- Convention: `__thiscall void (this = ECX (kept in ESI), int32 garageId = [esp+4])`, `ret 4`. Frame
  0x1E8 bytes. EBX = 0 throughout (used as the literal 0 / false / 0.0f in pushes).
- Callers: CGarages::Update 0x44C8C0 (once per garage, garageId = array index).
- Callees (all via orig!): FindPlayerPed 0x56E210 (-1), FindPlayerVehicle 0x56E0D0 (-1, false) [called
  repeatedly instead of cached: every "FPV" below is a fresh call; they are pure lookups],
  FindPlayerCoors 0x56E010 (cdecl (CVector* out, -1), returns out; pure), FindPlayerWanted 0x56E230 (-1),
  CPad::GetPad 0x53FB70 (0), CGarages::TriggerMessage 0x447B80 (cdecl (tag, -1, 4000, -1)),
  CAudioEngine::ReportFrontendAudioEvent 0x506EA0 (thiscall on &AudioEngine 0xB6BC90, (event, 0.0f,
  1.0f)), CText::Get 0x6A0050 (this = &TheText 0xC1B340), CHud::SetHelpMessage 0x588BE0 (cdecl (text,
  false, false, true)), CStats::GetStatValue 0x558E40 / IncrementStat 0x55C180 / SetStatValue 0x55A070,
  CWanted::ClearWantedLevelAndGoOnParole 0x5625A0, CWorld::CallOffChaseForArea 0x566A60 (cdecl, 4 f32),
  CStreaming::RequestModel 0x4087E0, CGeneral::GetRandomNumberInRange<float> 0x41BD90 (cdecl (min,max) ->
  ST0; original formula, see docs/re/cat14-gr-deviations.md finding 1), `_rand` 0x821B1E, 0x420800
  (unnamed cdecl `float max(float a, float b)`: returns `(a > b) ? a : b`, b when unordered),
  CVehicleModelInfo::ChooseVehicleColour 0x4C8500, CVehicle::GetRemapIndex 0x6D0B70 / SetRemap 0x6D0C00 /
  GetVehicleAppearance 0x6D1080, CVehicle::DestroyVehicleAndDriverAndPassengers 0x6D2250 (cdecl),
  FxPrtMult_c::FxPrtMult_c 0x4AB290, FxSystem_c::AddParticle 0x4AA440, CGarages::IsCarSprayable 0x4479A0,
  GivePlayerDetonator 0x448660, FindSafeHouseIndexForGarageType 0x4489F0, CountCarsInHideoutGarage
  0x44A210, FindDoorsWithGarage 0x449FF0, StoreAndRemoveCarsForThisHideOut 0x449900 and every CGarage
  helper in this file.
- Globals: R/W LastGaragePlayerWasIn 0x96BFDC, LastTimeHelpMessage 0x96BFE0; W bCamShouldBeOutside
  0x96BFE4; R NoResprays 0x96C008, RespraysAreFree 0x96C009, BombsAreFree 0x96C00A;
  W TheCamera.m_pToGarageWeAreIn 0xB6F844 (TheCamera 0xB6F028 + 0x81C) and
  m_pToGarageWeAreInForHackAvoidFirstPerson 0xB6F848; R/W Players[PlayerInFocus].m_nMoney
  (0xB7CE50 + u8[0xB7CD74]*0x190, i32); R CTimer::m_snTimeInMilliseconds 0xB7CB84 (u32),
  m_FrameCounter 0xB7CB4C (low byte tested); R CStreaming::ms_aInfoForModel[364].m_LoadState u8 0x8E6940;
  R g_fx+0x20 0xA9AE20 (gr Fx_c::m_SmokeHuge, FxSystem_c*); R ms_vehicleColourTable 0xB4E480 (RwRGBA[]).
- Constants (f32, .rdata): 0x858B8C 0.5, 0x858B50 0.0, 0x858EE8 0.4, 0x858F4C 950.0, 0x859A44 64.0,
  0x85862C 10.0, 0x859A40 970.0, 0x859A3C 1/255 (0x3B808081), 0x858978 900.0, 0x8599D8 289.0,
  0x8599D4 225.0, 0x8599D0 16.0, 0x8599CC 4900.0, 0x8599C8 12.25, 0x858628 100.0, 0x858FE8 25.0,
  0x858CA0 2.0, 0x8599BC 4225.0, 0x8599B8 3600.0. Immediates: 1000.0, 0.25, 2.0, -3.0/3.0, 0.05, 0.4, 0.6,
  0.7, 1.0, 1.2, -1.0, 4000 (message ms), 2000 (0x7D0 door re-open delay), 18000 (0x4650 help gap).
- GXT tags: 0x859A48 "GA_1", 0x859A50 "GA_1B", 0x859A58 "GA_3", 0x859A34 "GA_22", 0x859A2C "GA_2",
  0x859A24 "GA_XX", 0x859A1C "GA_15", 0x859A14 "GA_16", 0x859A0C "GA_4", 0x859A04 "GA_5", 0x8599DC "GA_6",
  0x8599E4 "GA_6B", 0x8599EC "GA_7", 0x8599F4 "GA_7B", 0x8599FC "GA_8", 0x8599C0 "GA_21".

Shorthands used below: FPV = FindPlayerVehicle(-1, false); "player pos" = FindPlayerCoors(-1);
`C = ((Left+Right)*0.5, (Front+Back)*0.5)`; `distC2` = squared 2D distance from the player to C, computed
with FOUR FindPlayerCoors calls p1..p4 as
`dyA = p3.y - (Front+Back)*0.5f` (stored f32); `s = (p4.y - (Front+Back)*0.5f) * dyA`;
`s = s + ((p2.x - (Right+Left)*0.5f) * (p1.x - (Right+Left)*0.5f))` (the x factors in registers, sums in
the order shown: `fld Front; fadd Back; fmul 0.5; fsubr p.y`, `fld Right; fadd Left; ...`);
`rectD2(x,y)` = CalcDistToGarageRectangleSquared(x, y) 0x447D80.
`FREEZE` = `GetPad(0)->DisablePlayerControls (+0x10E) |= 0x04` (bPlayerAwaitsInGarage);
`UNFREEZE` = `&= ~0x04`. `COPS_OFF` = `FindPlayerWanted(-1)->m_Flags (+0x1E) |= 0x02`
(m_bPoliceBackOffGarage); `COPS_ON` = `&= ~0x02`. `CLOSE1` = `m_nFlags |= 1; m_nDoorState = 2`.
"return" = leave Update.

### U1. Camera block (runs before everything)
If `type != 13 && doorState <= 5 && FindPlayerPed(-1) != NULL && !(flags & 0x20)`:
1. `veh = FPV`; `ent = FindPlayerPed(-1)`; if `ped->m_nPedFlags (+0x46C) & 0x100` (bInVehicle) and
   `ped->m_pVehicle (+0x58C) != NULL` and `ped->m_pVehicle->m_nModelIndex == 571` (0x23B, KART) ->
   `ent = ped->m_pVehicle` (ped re-fetched for each read).
2. If `IsEntityEntirelyInside3D(ent, 0.25f)`: `bCamShouldBeOutside = 1; TheCamera.m_pToGarageWeAreIn =
   this`.
3. If veh: if `!IsEntityEntirelyOutside(veh, 0.0f)`: `m_pToGarageWeAreInForHackAvoidFirstPerson = this`.
   If `veh->m_nModelIndex == 423` (0x1A7, MRWHOOP) and, with P = veh position,
   `(Left - 0.5f) < P.x && (Right + 0.5f) > P.x && (Front - 0.5f) < P.y && (Back + 0.5f) > P.y`:
   `bCamShouldBeOutside = 1; m_pToGarageWeAreIn = this`.

### U2. Inactive / door-closed flag
1. If `(flags & 0x02) && doorState == 0` -> return.
2. If `flags & 0x08` (door opens up): clear bit 0x40 if `(state == 3 && doorPos > 0.4f) || state == 1`,
   otherwise set bit 0x40 (byte written back once).

### U3. Dispatch on type (index type-1 via 0x44C80C, table 0x44C7D4)
1 -> U5; 2,3,4 -> U4b; 5 -> U4a; 11 -> U6; 14,21 -> U7; 15 -> U8; 16..18, 24..32, 39..42, 44, 45 -> U9;
19 -> U8b; 20 -> U10; 23 -> U11; 33..35 -> U12; 36..38 -> U13; 43 -> U14; 6..10, 12, 13, 22, 0, >45 ->
return.

Shared tails: `T_OPENING` (state 3 for types 2..5): if `SlideDoorOpen()` -> state = 4; then if
`doorPos > 0.5f` (ordered): UNFREEZE; COPS_ON. return.
`T_SLIDE_OPEN` (0x44BCBE): if `SlideDoorOpen()` -> state = 1. return.
`T_WAIT_EXIT` (state 4 for types 2..5): if `IsPlayerOutsideGarage(0.0f)` -> state = 1. return.
`T_CLOSE_SIMPLE` (0x44C69D): if `SlideDoorClosed()` -> state = 0. return.
`T_CHASE` (0x44B3C1): `CallOffChaseForArea(Left - 10.0f, Front - 10.0f, Right + 10.0f, Back + 10.0f)`
(each computed `fld; fadd/fsub 10.0; fstp` f32); return.

### U4a. PAYNSPRAY (type 5)
Gate: `p = player pos`; unless `p.z < 950.0f` -> return. Switch state (table 0x44C83C, state > 4 ->
return): 0 -> A, 1 -> B, 2 -> C, 3 -> T_OPENING, 4 -> T_WAIT_EXIT.

B (OPEN):
1. If NoResprays -> return.
2. If `IsStaticPlayerCarEntirelyInside()`:
   - If `IsCarSprayable(FPV)`:
     - If `Players[focus].m_nMoney >= 100` (signed) or RespraysAreFree: `state = 2; FREEZE;
       FPV->m_fDirtLevel (+0x4B0) = 0`.
     - else: `TriggerMessage("GA_3", -1, 4000, -1); ReportFrontendAudioEvent(14
       /*AE_FRONTEND_CAR_NO_CASH*/, 0.0f, 1.0f); state = 4`.
   - else: `TriggerMessage(FPV->subclass == 10 ? "GA_1B" : "GA_1", -1, 4000, -1);
     ReportFrontendAudioEvent(15 /*CAR_IS_HOT*/, 0.0f, 1.0f); state = 4`.
   - then (all three): `COPS_OFF; LastGaragePlayerWasIn = garageId`.
   Else (not static inside): if `!IsPlayerOutsideGarage(0.0f)`: `COPS_OFF; LastGaragePlayerWasIn =
   garageId`; else if `garageId == LastGaragePlayerWasIn`: `COPS_ON`.
3. If FPV == NULL -> return. P = FPV position. If `rectD2(P.x, P.y) < 64.0f` -> T_CHASE, else return.

C (CLOSING):
1. If FPV: `ThrowCarsNearDoorOutOfGarage(FPV)`.
2. If `SlideDoorClosed()`: `ReportFrontendAudioEvent(16 /*CAR_RESPRAY*/, 0.0f, 1.0f); state = 0;
   m_nTimeToOpen = now + 2000`; `v = GetStatValue(176)`; `IncrementStat(177, v)`;
   `SetStatValue(176, 0.0f)` (gr names 176 STAT_KILLS_SINCE_LAST_CHECKPOINT, 177
   STAT_TOTAL_LEGITIMATE_KILLS).
3. If FPV: `*(u32*)(FPV+0x8E4) = 0` (CAutomobile burning timer; written whatever the vehicle class:
   for a bike/boat this hits an unrelated offset inside the 0xA18 pool slot, keep) ;
   `FPV[+0x42E] |= 0x80` (bDisableParticles). -> T_CHASE.

A (CLOSED):
1. If NoResprays -> return. If `now <= m_nTimeToOpen` (unsigned) -> T_CHASE.
2. `state = 3`; `charged = false` (local); `hadWanted = FindPlayerWanted(-1)->m_WantedLevel (+0x2C) != 0`;
   if hadWanted: `charged = true; FindPlayerWanted(-1)->ClearWantedLevelAndGoOnParole()`.
3. If FPV != NULL and FPV class is 0 or 9:
   a. If `FPV->m_fHealth < 970.0f` -> charged = true.
   b. `FPV->m_fHealth = max(1000.0f, FPV->m_fHealth)` (0x420800, a = 1000.0, b = health).
   c. If class == 0: `*(u32*)(FPV+0x8E4) = 0` else `*(u32*)(FPV+0x7BC) = 0` (burning timers).
   d. `FPV->vtbl[50] ()` (+0xC8; CAutomobile::Fix 0x6A3440 / the bike's override).
   e. `IncrementStat(138 /*STAT_VEHICLE_RESPRAYS*/, 1.0f)`.
   f. If `FPV->m_matrix->up.z < 0.0f`: negate, in this order, up.x, up.y, up.z, right.x, right.y,
      right.z of FPV->m_matrix (each `fld; fchs; fstp`; forward untouched).
   g. `changed = false`; `v = FPV`. If `!(v->autoFlags (+0x868) & 0x02)` and `v->GetRemapIndex() < 0`:
      `ChooseVehicleColour(mi = ms_modelInfoPtrs[v->model], &c1, &c2, &c3, &c4, 1)`; `changed = c1 !=
      v[+0x434] || c2 != v[+0x435] || c3 != v[+0x436] || c4 != v[+0x437]` (short-circuit order);
      write c1..c4 to +0x434..+0x437; `v->SetRemap(-1)`. If changed: particle burst:
      - `FxPrtMult_c fx(1.0f, 0.0f, 0.0f, 0.6f, 0.7f, 1.0f, 0.4f)` (ctor args in that order; local), then
        `col = ms_vehicleColourTable[v[+0x434]]` (dword); `fx.r = (float)(col & 0xFF) * (1/255)`,
        `fx.g = (float)((col >> 8) & 0xFF) * (1/255)`, `fx.b = (float)((col >> 16) & 0xFF) * (1/255)`
        (fild of the int, fmul 0x859A3C, f32).
      - `vel = (0, 0, 0)`; 10 iterations: `pos = v position` (copied); `pos.x = Rnd(-3.0f, 3.0f) + pos.x`;
        `pos.y = Rnd(-3.0f, 3.0f) + pos.y`; `vel.z = Rnd(0.0f, 0.05f)` (three calls in this order);
        `(*(FxSystem_c**)0xA9AE20)->AddParticle(&pos, &vel, 0.0f, &fx, -1.0f, 1.2f, 0.6f, 0)`.
   h. `v->m_fDirtLevel = 0; v[+0x42E] &= 0x7F` (bDisableParticles off).
   (If FPV is NULL or of another class, `changed = hadWanted` instead and a..h are skipped.)
4. Messages:
   - if `flags & 0x80`: `TriggerMessage("GA_22", -1, 4000, -1)`.
   - else if charged && !RespraysAreFree: `m = money; if (m > 0) money = max(m - 100, 0)`;
     `IncrementStat(16 /*STAT_AUTO_REPAIR_AND_PAINTING_BUDGET*/, 100.0f)`;
     `TriggerMessage(hadWanted ? "GA_2" : "GA_XX", -1, 4000, -1)`.
   - else if changed: `TriggerMessage((rand() & 1) ? "GA_15" : "GA_16", -1, 4000, -1)` (rand called only
     here).
5. `flags |= 0x04` (m_bUsedRespray); if FPV: `FPV[+0x42F] |= 0x01` (bHasBeenResprayed). -> T_CHASE.

### U4b. BOMBSHOP_TIMED / ENGINE / REMOTE (types 2, 3, 4)
Switch state (table 0x44C850, > 4 -> return): 0 -> A, 1 -> B, 2 -> C, 3 -> T_OPENING, 4 -> T_WAIT_EXIT.

B (OPEN): if `!IsStaticPlayerCarEntirelyInside()` -> return; FPV NULL -> return; FPV subclass 9 or 10 ->
return. If `FPV[+0x4A8] & 7` (bomb already on board): `TriggerMessage("GA_5", -1, 4000, -1);
ReportFrontendAudioEvent(17 /*ALREADY_RIGGED*/, 0, 1); state = 4`; return. If `!BombsAreFree && money <
500`: `TriggerMessage("GA_4", ...); ReportFrontendAudioEvent(14, 0, 1); state = 4`; return. Else:
`state = 2; FREEZE; COPS_OFF`; return.

C (CLOSING): if FPV: ThrowCarsNearDoorOutOfGarage(FPV). If `SlideDoorClosed()`: `state = 0;
m_nTimeToOpen = now + 2000`. Then (every frame in this state) if `type == 4`: `RequestModel(364
/*detonator*/, 2 /*STREAMING_GAME_REQUIRED*/)`. return.

A (CLOSED):
1. If `now <= m_nTimeToOpen` -> return.
2. If `type == 4 && ms_aInfoForModel[364].m_LoadState != 1`: `RequestModel(364, 2)`; return.
3. `ReportFrontendAudioEvent(type == 2 ? 18 : type == 3 ? 19 : 20, 0.0f, 1.0f)`; `state = 3`.
4. If `!BombsAreFree`: `m = money; if (m > 0) money = max(m - 500, 0)`.
5. If FPV and class 0 or 9: `FPV[+0x4A8] = (FPV[+0x4A8] & ~7) | ((type - 1) & 7)`;
   `ped = FindPlayerPed(-1)`; `FPV->m_pWhoInstalledBombOnMe (+0x4CC) = ped`; if type == 4:
   `GivePlayerDetonator()`; `IncrementStat(127 /*STAT_KGS_OF_EXPLOSIVES_USED*/, 10.0f)`.
6. Help text, `mode = GetPad(0)->Mode (+0x10A, u16)`:
   type 2: mode <= 2 -> "GA_6", mode == 3 -> "GA_6B", else none; type 3: "GA_7" / "GA_7B"; type 4: "GA_8".
   `SetHelpMessage(TheText.Get(tag), false, false, true)`. return.

### U5. ONLY_TARGET_VEH (type 1)
Switch state (table 0x44C864, > 3 -> return): 0 -> A, 1 -> B, 2 -> C, 3 -> T_SLIDE_OPEN.

B (OPEN): `d = distC2`.
- If `d > 900.0f` (ordered): if `m_FrameCounter & 0x1F` -> return. If target == NULL -> CLOSE1; else if
  `!IsEntityTouching3D(target)` -> CLOSE1. return.
- Else: if `FPV == target` -> return; target NULL -> return; `!IsEntityEntirelyInside3D(target, 0.0f)` ->
  return; `ent = FPV ? FPV : FindPlayerPed(-1)`; `!IsEntityEntirelyOutside(ent, 2.0f)` -> return.
  `FREEZE; COPS_OFF; flags &= ~1; state = 2`. return.

C (CLOSING): if target: ThrowCarsNearDoorOutOfGarage(target). If `!SlideDoorClosed()` return. If
`flags & 1`: state = 0; return. If target: `state = 5; DestroyVehicleAndDriverAndPassengers(target);
m_pTargetCar = NULL` (no reference cleanup) else `state = 0`. Then `UNFREEZE; COPS_ON`. return.

A (CLOSED) (also used by type 20): if `FPV != target` or target NULL -> return; P = FPV position; if
`rectD2(P.x, P.y) < 64.0f` -> state = 3. return.

### U6. UNKN_CLOSESONTOUCH (type 11)
state 1: if `IsGarageEmpty()` -> state = 2. state 2: if `SlideDoorClosed()` -> state = 0; then if
`!IsGarageEmpty()` -> state = 3. state 3: T_SLIDE_OPEN. Others: nothing.

### U7. OPEN_FOR_TARGET_FREEZE_PLAYER (14) and CLOSE_WITH_CAR_DONT_OPEN_AGAIN (21)
Switch state (table 0x44C874, > 5 -> return): 0 -> A, 1 -> B, 2 -> C, 3 -> T_SLIDE_OPEN, 4 -> return,
5 -> D.
B (OPEN): `d = distC2`. If `d > 900.0f` or target NULL -> CLOSE1, return. If `target != FPV` -> return;
`!IsStaticPlayerCarEntirelyInside()` -> return; `IsAnyCarBlockingDoor()` -> return.
`FREEZE; COPS_OFF; flags &= ~1; state = 2`. return.
C (CLOSING): if target: ThrowCarsNearDoorOutOfGarage(target). `!SlideDoorClosed()` -> return. If flags & 1:
state = 0, return. If target: `state = 5; m_nTimeToOpen = now + 2000; m_pTargetCar = NULL` (no ref
cleanup, vehicle kept) else state = 0. `UNFREEZE; COPS_ON`. return.
A (CLOSED): `FPV != target` or target NULL -> return; P = FPV pos; if `rectD2(P) < 289.0f` -> state = 3.
D (DROPPED): if `type == 14 && now > m_nTimeToOpen` -> state = 3.

### U8. SCRIPT_ONLY_OPEN (15): state 3 -> T_SLIDE_OPEN; else nothing.
### U8b. SCRIPT_CONTROLLED (19): state 2 -> T_CLOSE_SIMPLE; state 3 -> T_SLIDE_OPEN; else nothing.

### U9. Hideouts (16..18, 24..32, 39..42, 44, 45)
`need = (type != 16) ? 4 : 2` (number of cars that makes the garage full). Switch state (table 0x44C88C,
> 3 -> return): 0 -> A, 1 -> B, 2 -> C, 3 -> T_SLIDE_OPEN.

B (OPEN): `d = rectD2(p1.x, p2.y)` (two FindPlayerCoors calls), kept as f32 local.
1. If `d > 225.0f` -> step 2. Else if `d <= 16.0f` or NaN -> step 3. Else if FPV == NULL or FPV subclass
   == 10 -> step 2, else step 3.
2. If `!IsAnyCarBlockingDoor()` -> state = 2, return. Else step 3.
3. If FPV: if `CountCarsWithCenterPointWithinGarage(FPV) >= need` and `IsPlayerOutsideGarage(0.25f)` ->
   state = 2, return.
4. If `d > 4900.0f` (ordered): `state = 2; RemoveCarsBlockingDoorNotInside()`. return.

C (CLOSING): `SlideDoorClosed()` (result ignored). If `!IsPlayerOutsideGarage(0.0f)` -> state = 3,
return. If `doorPos == 0.0f` (exactly; NaN fails): `state = 0`; if type != 44:
`StoreAndRemoveCarsForThisHideOut(aCarsInSafeHouse[FindSafeHouseIndexForGarageType(type)], 4)`.
return.

A (CLOSED):
1. p = player pos; unless `p.z < 950.0f` -> return.
2. `d = rectD2(p1.x, p2.y)`. If `d < 12.25f` -> step 3. Else unless `d < 100.0f` -> return; FPV NULL ->
   return; FPV subclass 10 -> return.
3. If FPV != NULL and type != 44 and `CountCarsInHideoutGarage(type) >= need` (garage full):
   `FindDoorsWithGarage(&d1, &d2)` (locals). For door = d1 then d2 (each only if non-NULL; a NULL d2 ends
   with return): with D = door position, V = FPV position, `(D.y - V.y)*(D.y - V.y) + (D.x - V.x)*(D.x
   - V.x) < 25.0f` -> help (d1 hit skips the d2 test). If neither is within 5 m -> return.
   Help: if `now - LastTimeHelpMessage > 18000` (unsigned) and `FPV->GetVehicleAppearance()` is not 3
   (HELI) and not 5 (PLANE): `SetHelpMessage(TheText.Get("GA_21"), false, false, true);
   LastTimeHelpMessage = now`. return.
4. Otherwise (FPV NULL, type 44, or not full): if type == 44 -> state = 3; else if
   `RestoreCarsForThisHideOut(aCarsInSafeHouse[FindSafeHouseIndexForGarageType(type)])` -> state = 3.
   return.

### U10. STAY_OPEN_WITH_CAR_INSIDE (20)
Table 0x44C89C: 0 -> U5.A, 1 -> B, 2 -> C, 3 -> T_SLIDE_OPEN.
B: unless `distC2 > 900.0f` -> return; target NULL -> return; `IsEntityEntirelyOutside(target, 0.0f)` ->
CLOSE1. return.
C: if target: ThrowCarsNearDoorOutOfGarage(target). T_CLOSE_SIMPLE.

### U11. SCRIPT_OPEN_FREEZE_WHEN_CLOSING (23)
state 1: target NULL -> return; `!IsEntityEntirelyInside3D(target, 0.0f)` -> return;
`IsAnyCarBlockingDoor()` -> return; `!IsPlayerOutsideGarage(0.0f)` -> return; `FREEZE; flags &= ~1;
state = 2`. state 2: if `SlideDoorClosed()`: `state = 0; UNFREEZE`. state 3: T_SLIDE_OPEN.

### U12. IMPOUND_LS / SF / LV (33..35)
`cars = aCarsInSafeHouse[FindSafeHouseIndexForGarageType(type)]` (looked up again at each use).
`inZ` = (with a fresh player pos each) `(TopZ - 2.0f) > p.z && p.z > posn.z`.
state 0 (CLOSED): `d = rectD2(p1.x, p2.y)`; compute inZ; unless `d < 3600.0f` -> return; !inZ -> return;
`NeatlyLineUpStoredCars(cars)`; if `RestoreCarsForThisImpoundingGarage(cars)` -> state = 1 (no door
animation). return.
state 1 or 2: `d = rectD2(...)`; compute inZ; if `d <= 4225.0f && inZ && state == 1` -> return. Else
`state = 0; StoreAndRemoveCarsForThisImpoundingGarage(cars, 3)`. return.
States 3..5: nothing.

### U13. Tuning garages (36..38)
Table 0x44C8AC: 0 -> A, 1 -> B, 2 -> C, 3 -> T_SLIDE_OPEN.
B (OPEN): unless `distC2 > 900.0f` -> return; if `m_FrameCounter & 0x1F` -> return. If
`RightModTypeForThisGarage(FPV)` and `IsEntityTouching3D(FPV)` -> return. If
`IsAnyOtherCarTouchingGarage(NULL)` -> return. CLOSE1. return.
C (CLOSING): if FPV: ThrowCarsNearDoorOutOfGarage(FPV). T_CLOSE_SIMPLE.
A (CLOSED): if `!RightModTypeForThisGarage(FPV)` -> return; P = FPV pos; if `rectD2(P) < 64.0f` ->
state = 3.

### U14. BURGLARY (43)
state 1: unless `distC2 > 900.0f` -> return; if `IsAnyOtherCarTouchingGarage(NULL)` -> return; state = 2.
state 2: U13.C. state 3: T_SLIDE_OPEN. Others nothing.

- Edge cases / oddities (all keep-original): FindPlayerVehicle/FindPlayerCoors are called many times
  per path (pure; the port may cache only if it proves identical results). m_pTargetCar is nulled without
  CleanUpOldReference (U5.C, U7.C). The CLOSING Pay'n'Spray writes CAutomobile+0x8E4 whatever the class.
  Impound doors never animate. Hideout CLOSING ignores SlideDoorClosed's result and compares doorPos with
  exactly 0.0. 0x420800 keeps NaN health (b returned when unordered).
- gta-reversed: `plugin::CallMethod<0x44AA50>` only; no code to compare.
- Open questions: none affecting behaviour. (Names: 0x449FF0 FindDoorsWithGarage is assumed from gr and
  not specified here; its out-parameter order (&d1 first) is taken from the call sites.)
