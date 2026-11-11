# CVehicle (VEH) - full specs (writer group E)

Offsets: plugin-sdk `CVehicle.h` VALIDATE_OFFSETs (gr `Entity/Vehicle/Vehicle.h` declares the same members).
Used here: +0x14 m_matrix, +0x18 m_pRwClump, +0x22 model, +0x36 type/status byte (status = bits 3..7),
+0x45C m_nAlarmState u16, +0x460 m_pDriver, +0x4AB m_nAmmoInClip u8, +0x4D4 m_nGunFiringTime u32, +0x594
m_nVehicleSubType, +0x598 m_nPreviousRemapTxd i16, +0x59A m_nRemapTxd i16, +0x59C m_pRemapTexture.

## Virtual-slot name corrections (CVehicle vtable 0x871E80)
The catalogue names these `CVehicle::vfunc_N`. Slot order of gr's CEntity/CPhysical/CVehicle virtual declarations
(Entity.h: dtor 0, Add 1, Add(rect) 2, Remove 3, SetIsStatic 4 = 0x403E20, SetModelIndex 5 = 0x6D6A40, ...,
DeleteRwObject 8 = 0x6D6410, ..., SpecialEntityPreCollisionStuff 15 = 0x6D6640, ...; CPhysical
ProcessEntityCollision 22; Vehicle.h from ProcessControlCollisionCheck 23 to Load 65) matches every known
address in the vtable (e.g. slot 28 = ProcessOpenDoor 0x6D56C0, slot 50 = Fix 0x6D6390 as commented in gr,
slot 57 = CanPedStepOutCar 0x6D1F30), so the names below are evidence-backed. Each `ret N` matches the gr arity.
The U32/eDoors pairs (slots 29-38) cannot be told apart by the bytes; their order follows gr.

| addr | slot | gr name | bytes |
|---|---|---|---|
| 0x6A6290 | 21 | FlagToDestroyWhenNextProcessed | `ret` |
| 0x6D6270 | 23 | ProcessControlCollisionCheck(bool) | `ret 4` |
| 0x6D6280 | 24 | ProcessControlInputs(uint8) | `ret 4` |
| 0x6D6290 | 25 | GetComponentWorldPosition(int32, CVector&) | `ret 8` (out vector untouched) |
| 0x6D62A0 | 26 | IsComponentPresent(int32) | AL = 0, `ret 4` |
| 0x6B5820 | 27 | OpenDoor(CPed*, int32, eDoors, float, bool) | `ret 0x14` |
| 0x6B5830 | 29 | GetDooorAngleOpenRatioU32(uint32) | ST0 = 0.0f ([0x858B50]), `ret 4` |
| 0x6D56B0 | 30 | GetDooorAngleOpenRatio(eDoors) | ST0 = 0.0f ([0x858B50]), `ret 4` |
| 0x6D62F0 | 31 | IsDoorReadyU32 | AL = 0, `ret 4` |
| 0x6D62B0 | 32 | IsDoorReady | AL = 0, `ret 4` |
| 0x6D6300 | 33 | IsDoorFullyOpenU32 | AL = 0, `ret 4` |
| 0x6D62C0 | 34 | IsDoorFullyOpen | AL = 0, `ret 4` |
| 0x6D6310 | 35 | IsDoorClosedU32 | AL = 0, `ret 4` |
| 0x6D62D0 | 36 | IsDoorClosed | AL = 0, `ret 4` |
| 0x6D6320 | 37 | IsDoorMissingU32 | AL = 0, `ret 4` |
| 0x6D62E0 | 38 | IsDoorMissing | AL = 0, `ret 4` |
| 0x6B5840 | 39 | IsOpenTopCar() | AL = 0, `ret` |
| 0x6D6330 | 40 | RemoveRefsToVehicle(CEntity*) | `ret 4` |
| 0x6D6340 | 41 | BlowUpCar(CEntity*, bool) | `ret 8` |
| 0x6B5850 | 42 | BlowUpCarCutSceneNoExtras(bool, bool, bool, bool) | `ret 0x10` |
| 0x6D6350 | 43 | SetUpWheelColModel(CColModel*) | AL = 0, `ret 4` |
| 0x6D6360 | 44 | BurstTyre(uint8, bool) | AL = 0, `ret 8` |
| 0x6D6370 | 45 | IsRoomForPedToLeaveCar(uint32, CVector*) | AL = 0, `ret 8` |
| 0x6A6280 | 47 | GetRideAnimData() | EAX = 0 (null), `ret` |
| 0x6D6380 | 48 | SetupSuspensionLines() | `ret` |
| 0x6B5860 | 49 | AddMovingCollisionSpeed(CVector&) -> CVector (RVO) | see below, `ret 8` |
| 0x6D6390 | 50 | Fix() | `ret` (catalogue name already Fix) |
| 0x6D63A0 | 51 | SetupDamageAfterLoad() | `ret` |
| 0x6D63B0 | 52 | DoBurstAndSoftGroundRatios() | `ret` |
| 0x6D63C0 | 54 | PlayCarHorn() | `ret` |
| 0x6D63D0 | 55 | GetNumContactWheels() | EAX = 4, `ret` |
| 0x6D63E0 | 56 | VehicleDamage(float, eVehicleCollisionComponent, CEntity*, CVector*, CVector*, eWeaponType) | `ret 0x18` |
| 0x6B5880 | 61 | SetTowLink(CVehicle*, bool) | AL = 0, `ret 8` |
| 0x6B5890 | 62 | BreakTowLink() | AL = 0, `ret` |
| 0x6D3D50 | 63 | FindWheelWidth(bool) | ST0 = 0.25f ([0x858C84]), `ret 4` |

All are `__thiscall` (ECX = this, unused). "AL = 0" is `xor al, al` (upper EAX bits unchanged; callers test AL).
For 0x6B5840 / 0x6B5890 / 0x6D63D0 / 0x6A6280 there are no stack args. Callers: slot 47 by CTaskSimpleCarDrive::
ProcessPed and DoDriveByShootings; slot 49 by CPhysical::ApplyCollision* (3 fns); slot 41 by CPlayerInfo::
BlowUpRCBuggy, ProcessDelayedExplosion, InflictDamage; others mostly have no direct callers.

### 0x6B5860 CVehicle::vfunc_49 = AddMovingCollisionSpeed  (catalogue: specified/REA, port level 0)
- `__thiscall CVector* (CVehicle* this, CVector* ret [esp+4] /*RVO*/, CVector& point [esp+8])`, `ret 8`.
- Behaviour: `ret->x = ret->y = ret->z = 0` (integer zero stores); returns `ret` in EAX (`mov eax,[esp+4]`).

## 0x4F4CE0 CVehicle::sub_4F4CE0 (proposed: CVehicle::IsAlarmActivated)  (catalogue: unknown/REA, port level 0)
- Evidence: `this` is a vehicle (callers CAutomobile::ProcessAI 0x6B4800 and CVehicle::DoVehicleLights 0x6E1A60;
  reads +0x45C m_nAlarmState and the status byte +0x36); the result is the negation of gr's inline
  `CVehicle::CanUpdateHornCounter()` (Vehicle.h:754).
- Convention: `__thiscall bool (const CVehicle* this)`, returns EAX = 0 or 1 (full register), plain `ret`.
- Behaviour: `a = m_nAlarmState (u16)`; return `a != 0 && a != 0xFFFF && (status byte & 0xF8) != 0x28`
  (status != 5 STATUS_WRECKED).
- gta-reversed: `CanUpdateHornCounter()` compares the uint16 with `-1` after integer promotion, which is never
  true (65535 != -1); the bytes compare with 0xFFFF. gr logic error - a port must compare with 0xFFFF.

## 0x6D0BC0 CVehicle::SetRemapTexDictionary  (catalogue: specified/REA, port level 10)
- Convention: `__thiscall void (CVehicle* this, int32 txdId [esp+4])`, `ret 4`.
- Behaviour: if `txdId == (int32)m_nPreviousRemapTxd` (sign-extended) return. If `txdId == -1`:
  `m_pRemapTexture = null; CTxdStore::RemoveRef(m_nPreviousRemapTxd /*sign-extended*/); m_nPreviousRemapTxd = -1`.
  Then (for any txdId != previous) `m_nRemapTxd = (int16)txdId`.
- gta-reversed: Vehicle.cpp:1147, identical.

## 0x6D33B0 CVehicle::RemoveObjectsCB-Object  (catalogue: specified/REA, port level 11)
- Convention: `__cdecl RwObject* (RwObject* object, void* data)`; returns `object` (EAX) in all paths (also after
  the atomic was destroyed). Used as an RwFrameForAllObjects callback by RemoveObjectsCB-Frame 0x6D3420 and
  RemoveReplacementUpgrade 0x6D39E0.
- Behaviour: only if `object->type (byte +0) == 1` (rpATOMIC): `id = CVisibilityPlugins::GetAtomicId(atomic)`;
  `*(int32*)data = id` (always written); if `id & 0x800` return. Else: `mi = CVisibilityPlugins::GetModelInfo(atomic)`;
  `frame = atomic->parent (+4)`; `RpClumpRemoveAtomic(atomic->clump (+0x3C), atomic)`; `RpAtomicDestroy(atomic)`;
  if `CVisibilityPlugins::GetFrameHierarchyId(frame) == 0`: `RwFrameDestroy(frame)`; if `mi`: `mi->RemoveRef()`.
- gta-reversed: raw call only (Vehicle.cpp:2295). Note gr's caller passes `&frameOfUpgrade` as data, which this
  callback overwrites with the atomic id.

## 0x6E38A0 CVehicle::vfunc_0 = deleting destructor  (catalogue: specified/REA, port level 15)
- Convention: `__thiscall CVehicle* (CVehicle* this, uint8 flags [esp+4])`, `ret 4`.
- Behaviour: `CVehicle::~CVehicle(this)` (0x6E2B40). If `flags & 1`: inline vehicle-pool free:
  `i = (this - pool->objects) / 0xA18` (signed magic multiply 0x06572EC3, `sar 6`, round toward zero);
  `pool->flags[i] |= 0x80`; `if (i < pool->firstFree (+0xC)) pool->firstFree = i`. Returns this.

## 0x73DF00 CVehicle::FireFixedMachineGuns  (catalogue: specified/REA, port level 19)
- Convention: thiscall, no args, plain `ret`.
- Callers: CCarCtrl::SteerAIBoatWithPhysicsAttackingPlayer, DoFixedMachineGuns (tail call).
- Callees: CMatrix::operator* 0x59C890, rand (x6), CWeapon::DoTankDoomAiming 0x73D1E0 (cdecl (CEntity* vehicle,
  CEntity* owner, CVector* start, CVector* end)), CWeapon::FireOneInstantHitRound 0x73AF00 (cdecl (CVector* start,
  CVector* end, int32 damage)), CAudioEngine::ReportWeaponEvent 0x506F40 (thiscall on AudioEngine 0xB6BC90,
  (149, 31, this), ret 0xC).
- Constants: 0x858B1C = 0.1f, 0x858624 = 1.0f, 0x858B34 = 60.0f, 0x859F00 = 0.015f, 0x858B38 = 0.02f.
- Behaviour:
  1. `now = m_snTimeInMilliseconds`; return if `now <= m_nGunFiringTime + 150` (unsigned).
  2. `fx, fy = m_matrix->forward.x/.y`; `L = sqrt(fy*fy + fx*fx)`; `if (0.1f > L) L = 0.1f` (NaN keeps NaN);
     `inv = 1.0f / L`; `m_nGunFiringTime = now`; `dx = inv*fx`, `dy = inv*fy` (stored f32), then `dx *= 60.0f`,
     `dy *= 60.0f` (stored f32).
  3. Gun A: `S = m_matrix * (2.0f, 2.5f, 1.0f)` (full transform); `E = (S.x + dx, S.y + dy, S.z)`.
     `r1 = rand(); r2 = rand();` `E.x += ((r2 & 0xFF) - 128) * 0.015f`; `E.y += ((r1 & 0xFF) - 128) * 0.015f`;
     `r3 = rand(); E.z += ((r3 & 0xFF) - 128) * 0.02f` (int -> fild -> fmul f32, each sum stored f32).
     `DoTankDoomAiming(this, m_pDriver, &S, &E)`; `FireOneInstantHitRound(&S, &E, 15)`.
  4. Gun B: same with `(-2.0f, 2.5f, 1.0f)` and three new rand() calls in the same order.
  5. `AudioEngine.ReportWeaponEvent(149, 31, this)`.
  6. `if (--m_nAmmoInClip == 0) { m_nAmmoInClip = 20; m_nGunFiringTime = m_snTimeInMilliseconds + 1400; }`.
- gta-reversed: `plugin::CallMethod` only (Vehicle.cpp:5923).

## 0x741FD0 CVehicle::DoDriveByShootings  (catalogue: specified/REA, port level 22)
- Convention: thiscall, no args, plain `ret`. Split fragment at 0x402348 (`mov esi,[edi+0x460]`).
- Callers: CAutomobile::ProcessAI, CBike::ProcessAI, CQuadBike::ProcessAI, CBoat::ProcessControl.
- Behaviour:
  1. `drv = m_pDriver`; return if null. `info = drv->GetPlayerInfoForThisPlayerPed()`; return if null or
     `!info->m_bCanDoDriveBy (+0x153)`. `pad = drv->GetPadFromPlayer()`; return if null.
  2. `w = &drv->m_aWeapons[drv->m_nSelectedWepSlot (i8 +0x718)]` (+0x5A0 + slot*0x1C). Return unless
     `CWeaponInfo::GetWeaponInfo(w->type, 1)->m_nSlot (+0x14) == 4`.
  3. `fire = (m_nVehicleSubType == 10 /*BMX*/) ? (pad->GetCarGunFired() == 1) : (pad->GetCarGunFired() != 0)`
     (16-bit compares).
  4. `group = 0`. If `GetRideAnimData()` (vtable +0xBC, called twice) is non-null: `ride = true`,
     `group = rideData->AnimGroup (dword +0)`, anims L/R/B = 202/203/204. Else: B = 191 and L/R = 76/77 if
     vehicle flag byte +0x429 bit 3 (bLowVehicle) else 74/75.
  5. `w->Update(nullptr)` (0x73DB40, thiscall, ret 4).
  6. Look direction: `cam = TheCamera.m_aCams[TheCamera.m_nActiveCam (0xB6F081)]` (stride 0x238); if
     `cam.m_nMode (i16 at 0xB6F1A8 + 0x238*i) == 1` or `TheCamera byte +0x2F (0xB6F057) != 0` or `mode == 49`:
     `left = pad->GetLookLeft(); right = pad->GetLookRight()`; else `left = cam byte +0x17C (0xB6F1A4)`,
     `right = cam byte +0x17D`.
  7. Aim decision: if `ride`: aim = fire; else aim = left || right. If aim and `w->m_nAmmoTotal (+0xC) > 0`
     (signed): anim = left ? L : (right ? R : B) (B only reachable when ride). `a = RpAnimBlendClumpGetAssociation(
     drv->m_pRwClump, anim)`; if `a == null` or `a->m_fBlendDelta < 0`: `a = CAnimManager::BlendAnimation(clump,
     group, anim, 16.0f)`. If `a`: return if `a->m_nFlags & 1` or `a->m_fBlendAmount <= 0.99f` (0x862CD0).
     Then if `fire && now > w->m_nTimeForNextShot` (unsigned): `w->FireFromCar(this, left, right)` (thiscall,
     ret 0xC; the two flags are pushed as whole dwords from byte locals - only the low byte is defined);
     `w->m_nTimeForNextShot = now + 70`; `drv->DoGunFlash(250, false)` (0x5DF340). Return.
  8. Not aiming (or no total ammo): if `w->m_nAmmoTotal != 0`: `clip = GetWeaponInfo(type, 1)->m_nAmmoClip (i16
     +0x20)`; `w->m_nAmmoInClip = (uint32)total < (uint32)clip ? total : clip` (unsigned compare). Then for anims
     L, R (and B if ride): if the association exists, `m_fBlendDelta = -8.0f`.
- gta-reversed: `plugin::CallMethod` only (Vehicle.cpp:5928).
- Open questions: the meaning of `TheCamera` byte +0x2F and camera modes 1/49 (names) - numeric behaviour is exact.

## 0x6D0AF0 CVehicle::sub_6D0AF0  (catalogue: unknown/REA, port level 0)
- `__thiscall bool (CVehicle* this)`, plain `ret`, AL: `return *(u32*)(this+0x9EC) == 0xC47A0000` (an INTEGER compare with the bit pattern
  of -1000.0f - not a float compare: -1000.0f only, NaN/other encodings never match). Caller CVehicle::UpdateTrailerLink 0x6DFC50.
- gta-reversed: absent. Open questions: name of field +0x9EC (a 'not set = -1000' sentinel float of the trailer/tow code).

## 0x6E2D60 CVehicle::operator new(size_t,int32)  (catalogue: unknown/REA, port level 1)
- `__cdecl void* (size_t size [esp+4], int32 handle [esp+8])`, plain `ret`. Callers: CReplay::PlayBackThisFrameInterpolation 0x45F380 (11),
  CPools::LoadVehiclePool 0x5D2A20 (10).
- Behaviour: `p = vehiclePool->objects + (handle >> 8) * 0xA18` (pool [0xB74494]) computed BEFORE `CreateAtRef(handle)` 0x6D5DD0; return p.
  size unused.
- gta-reversed: CVehicle::operator new(size_t, int32) - same. Open questions: none.

## 0x6E2DD0 CVehicle::operator delete(void*,int32)  (catalogue: unknown/REA, port level 0)
- `__cdecl void (void* p [esp+4], int32 /*unused*/)`, plain `ret`: inlined CPool<CVehicle>::Delete: `i = (p - objects) / 0xA18` (magic `0x6572EC3`,
  `sar 6`); `flags[i] |= 0x80`; `if (i < firstFree) firstFree = i`. Pool [0xB74494].
- gta-reversed: same. Open questions: none.
