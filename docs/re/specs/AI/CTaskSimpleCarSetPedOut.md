# CTaskSimpleCarSetPedOut (AI) - RE-SPEC-3 group I3

Vtable 0x86EEB8.

## 0x647930 CTaskSimpleCarSetPedOut::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x330; ret` -> 816 (TASK_SIMPLE_CAR_SET_PED_OUT). Slot 4. gr matches.

## 0x647940 CTaskSimpleCarSetPedOut::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x647940; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86EEB8.

## 0x6479B0 CTaskSimpleCarSetPedOut::PositionPedOutOfCollision  (catalogue: specified/REA, port level 3)
- `__cdecl void (CPed* ped, CVehicle* veh, int32 door)` (plain `ret`). `veh = veh ? veh : ped->m_pVehicle`; return if still null.
- Heading: `h = veh->m_matrix ? atan2(-fwd.x, fwd.y)` (`fld fwd.x; fchs; fld fwd.y; fpatan`, stored f32) `: veh->+0x10`. `ped->m_fAimingRotation
  (+0x55C) = h`, `ped->m_fCurrentRotation (+0x558) = h`; `ped->m_matrix ? CMatrix::SetRotateZOnly(h) 0x59B020 : ped->+0x10 = h`.
- Boat (`veh->m_nVehicleType == 5`):
  1. If `veh->m_pHandlingData->+0xCC & 0x400`: `pedPos += veh up (matrix +0x20) * 0.5f [0x858B8C]` (per component, f32).
  2. If `ped->vtbl[13](0)` (vtbl+0x34; the entity collision test, returns colliding): if `veh->+0x42B & 0x40`: `pedPos.z -= 0.3f [0x858C24]`; if
     the test (re-run) now returns 0, return; else restore `z += 0.3f` and call `ped->PositionPedOutOfCollision(door, veh, true)` 0x5E0820 and
     return. Otherwise (bit clear): `pedPos -= ped forward * 0.3f`; if the test still collides: `pedPos += ped forward * 0.3f` (0x40FE90 then
     CVector::operator+= 0x411A00) and `PositionPedOutOfCollision(door, veh, true)`; return.
  3. Not colliding: `ped->m_vecMoveSpeed = veh->m_vecMoveSpeed * 0.9f [0x858C20]`; `ped->m_vecMoveSpeed.z -= 0.1f [0x858B1C]`; `ped->+0x46C |= 1`;
     if `ped->+0x568 == 0`: `ped->+0x568 = veh` and `RegisterReference(veh, &ped->+0x568)`.
- Other vehicles: `CWorld::pIgnoreEntity [0xB7CD68] = veh`; `hit = 0`; for dz in (-0.2f, +0.2f) (0x858CC4 = 0.2f): `e =
  CWorld::TestSphereAgainstWorld(CVector(P.x, P.y, P.z + dz), 0.4f, veh, true, true, false, false, false, false)` 0x569E20; if e and
  `e != veh->+0xFC` -> hit = 1. If `!CWorld::GetIsLineOfSightClear(vehPos, pedPos, true, false, false, true, false, false, false)` or hit:
  `ped->PositionPedOutOfCollision(door, veh, true)`. `pIgnoreEntity = 0`.
- gta-reversed: not reversed.

## 0x647D10 CTaskSimpleCarSetPedOut::ProcessPed  (catalogue: specified/REA, port level 24)
- `__thiscall bool (this, CPed* ped)`, `ret 4`, SEH; always returns 1.
- 1. ped bInVehicle = 0 (`+0x46C &= ~0x100`); `ped->+0x1C |= 1` (uses collision); `ped->UpdateStatLeavingVehicle()` 0x5E01B0.
- 2. If !m_bKnockedOffBike: `PositionPedOutOfCollision(ped, nullptr, m_nTargetDoor)` (vehicle argument 0 -> ped->m_pVehicle is used).
  `CCarEnterExit::RemoveCarSitAnim(ped)` 0x64F680; `ped->RestartNonPartialAnims()` 0x5DED50.
- 3. If !m_bKnockedOffBike and !m_bFallingOutOfCar and `m_pTargetVehicle->+0x594 != 5`: `ped->m_vecMoveSpeed = (0,0,0)`.
- 4. If `ped->m_pVehicle`: if m_nDoorFlagsToClear `m_pTargetVehicle->ClearGettingOutFlags(flags)`; if m_nNumGettingInToClear
  `m_pTargetVehicle->m_nNumGettingIn -= n`. If `ped->m_pVehicle->m_pDriver == ped`: `ped->m_pVehicle->RemoveDriver(!m_bSwitchOffEngine)` 0x6D1950;
  `veh->+0x36 = (veh->+0x36 & 7) | 0x20`; `if (veh->+0x4F8 == 5) veh->+0x4F8 = 1`; if `ped->m_nPedType == 6` and `veh->IsLawEnforcementVehicle()`:
  `veh->ChangeLawEnforcerState(false)` 0x6D2330. Else `veh->RemovePassenger(ped)` 0x6D1610. Then if `CGarages::IsPointWithinAnyGarage(&pedPos copy)`
  0x448990: `veh->+0x4A8 &= ~0x18`; `veh->+0x428 &= ~0x40`.
- 5. If !m_bFallingOutOfCar and !m_bKnockedOffBike and `ped->m_pVehicle` is a bike (type 9) and `|v.x| < 0.1` and `|v.y| < 0.1` (f64 0.1
  [0x86EEE0], `fabs` of the f32 move speed, strict): `veh->+0x614 |= 0x10`.
- 6. Primary task: player -> `SetTask(new(0x1C) CTaskSimplePlayerOnFoot() 0x685750 (or 0), 4, false)`; else if `ped->+0x484 == 2` (mission ped) ->
  `SetTask(new(0x20) CTaskSimpleStandStill(999999, true, false, 8.0f) (or 0), 4, false)`; else `SetTask(CTaskComplexWander::GetWanderTaskByPedType(ped)
  0x673D00, 4, false)` (CTaskManager::SetTask 0x681AF0).
- 7. `ped->ReplaceWeaponWhenExitingVehicle()` 0x5E6490; `ped->+0x474 |= 8` (bHasJustLeftCar); `ped->m_nPedState (+0x530) = 1`; if m_pTargetVehicle
  and `veh->+0x40 & 0x8000000`: `ped->+0x40 |= 0x8000000`. ped->m_pVehicle is not cleared here.
- gta-reversed: not reversed (plugin::Call).

## 0x64BF70 CTaskSimpleCarSetPedOut::`scalar deleting destructor'  (catalogue name CTaskSimpleCarSetPedOut::~CTaskSimpleCarSetPedOut)  (catalogue: specified/REA, port level 2)
- NAME CORRECTION: slot 0 of vtable 0x86EEB8. `__thiscall this (uint32 flags [esp+4])`, `ret 4`, returns this (EAX).
- Calls the complete dtor 0x647950 (0x64BF73): SEH; `vtbl = 0x86EEB8`; if the vehicle [+8] != 0 `CEntity::CleanUpOldReference([+8], this+8)` 0x571A00; then ~CTaskSimple 0x61A3A0. Then `if (flags & 1) CTask::operator delete(this)` 0x61A5B0
  (test at 0x64BF78, call at 0x64BF80; cdecl, inlined task-pool delete, stride 0x80). Shape shared by every 11-insn task deleting dtor.
- gta-reversed: compiler-generated (virtual ~X + pool operator delete) - matches.

## (sections below added by the lead for addresses the I3 writer left unwritten)
Layout (gr, size 0x18): +8 m_pTargetVehicle, +0xC m_nTargetDoor, +0x10 m_bSwitchOffEngine, +0x11 m_bWarpingOutOfCar, +0x12
m_bFallingOutOfCar, +0x13 m_bKnockedOffBike, +0x14 m_nDoorFlagsToClear u8, +0x15 m_nNumGettingInToClear u8.

## 0x649F50 CTaskSimpleCarSetPedOut::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x18) 0x61A5A0 (cdecl); if p == NULL the constructor is skipped and EAX = 0 (see the trailing stores). Else p->CTaskSimpleCarSetPedOut(this+8, this+0xC, (u8)this+0x10) 0x6478B0; then outside the NULL check: p+0x11..+0x15 = this+0x11..+0x15 (5 bytes, in increasing order; NULL-write bug as 0x649D90); return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
