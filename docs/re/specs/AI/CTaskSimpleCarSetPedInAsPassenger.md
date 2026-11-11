# CTaskSimpleCarSetPedInAsPassenger (AI) - RE-SPEC-3 group I4

Vtable 0x86EE04. Layout (gr TaskSimpleCarSetPedInAsPassenger.h, size 0x20): +0x8 m_bIsFinished, +0xC m_pAnim, +0x10 m_pTargetVehicle
(registered reference), +0x14 m_nTargetDoor, +0x18 m_pUtility (CTaskUtilityLineUpPedWithCar*), +0x1C m_bWarpingInToCar (u8),
+0x1D m_nDoorFlagsToClear (u8), +0x1E m_nNumGettingInToClear (u8).

## 0x647060 CTaskSimpleCarSetPedInAsPassenger::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x32B` (0x647060); `ret` -> returns 811 (TASK_SIMPLE_CAR_SET_PED_IN_AS_PASSENGER). Vtable: slot 4 (+0x10) of vtable 0x86EE04.

## 0x647070 CTaskSimpleCarSetPedInAsPassenger::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x647070; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86EE04.

## 0x647080 CTaskSimpleCarSetPedInAsPassenger::~CTaskSimpleCarSetPedInAsPassenger  (catalogue: specified/REA, port level 1)
- `__thiscall void (this)`, plain `ret`, SEH (handler 0x8426C8). Complete-object destructor (name correct). Callers: 9 (script
  commands 0x47D210 / 0x489500 for stack objects incl. EH unwind, CTaskComplexLeaveCarAsPassengerWait::SetPedInZerothPassengerSeat 0x63B540, ...).
- Behaviour: `vtbl = 0x86EE04` (0x64709D); `if (m_pTargetVehicle) m_pTargetVehicle->CleanUpOldReference(&m_pTargetVehicle)` 0x571A00
  (0x6470A3-0x6470B6); then CTaskSimple dtor 0x61A3A0 (`vtbl = 0x86D48C`). The pointer is not cleared.
- gta-reversed: TaskSimpleCarSetPedInAsPassenger.cpp:28 dtor (`CEntity::SafeCleanUpRef`) matches.

## 0x64B5B0 CTaskSimpleCarSetPedInAsPassenger::~CTaskSimpleCarSetPedInAsPassenger  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleCarSetPedInAsPassenger::~CTaskSimpleCarSetPedInAsPassenger 0x647080 (0x64B5B3); if `flags & 1` (0x64B5B8) calls CTask::operator delete 0x61A5B0 (this) (0x64B5C0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86EE04.

## 0x64B5D0 CTaskSimpleCarSetPedInAsPassenger::ProcessPed  (catalogue: specified/REA, port level 25)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`, always returns AL = 1. SEH (handler 0x843013; the `mov eax, fs:[0]` is moved to a
  no-CD island at 0x401DB5). Frame holds a local CTaskSimpleCarSetPedOut (at body [esp+0x10]).
- Callees: CEntity::CleanUpOldReference 0x571A00, RegisterReference 0x571B70, CPed::IsPlayer 0x5DF8F0, CPlayerPed::ClearAdrenaline 0x60A070,
  CWorld::FindPlayerPed 0x56E210, CCrime::ReportCrime 0x532010, CCarEnterExit::ComputePassengerIndexFromCarDoor 0x64F1E0,
  CTaskManager::FindActiveTaskByType 0x681740, CCarEnterExit::ComputeTargetDoorToExit 0x64F110, CTaskSimpleCarSetPedOut ctor 0x6478B0 /
  ProcessPed 0x647D10 / dtor 0x647950, CVehicle::AddPassenger (seat) 0x6D14D0 / (auto) 0x6D13A0, CPed::UpdateStatEnteringVehicle 0x5E01A0,
  CPed::SetMoveState 0x5DEC00, CPed::SetPedState 0x5E4500, CVehicle::ClearGettingInFlags 0x6D3040, CPed::RemoveWeaponWhenEnteringVehicle
  0x5E6370, CCarEnterExit::RemoveGetInAnims 0x64F6E0, CCarEnterExit::AddInCarAnim 0x64F720, CTaskUtilityLineUpPedWithCar::ProcessPed 0x6513A0,
  CTask::operator new 0x61A5A0, CTaskSimpleCarDrive ctor 0x63C340, CTaskManager::SetTask 0x681AF0.
- Behaviour (veh = m_pTargetVehicle):
  1. If `ped flags (+0x46C) & 0x100` (bInVehicle) and m_Parent (+4) != 0: `m_Parent->GetTaskType()` (vtbl+0x10) - result discarded
     (0x64B5F0-0x64B60A).
  2. `if (ped->m_pVehicle (+0x58C)) CleanUpOldReference(&ped->m_pVehicle)`; `ped->m_pVehicle = veh`; `veh->RegisterReference(&ped->m_pVehicle)`
     (no null check of veh, 0x64B60D-0x64B629). `ped[+0x55C] = ped[+0x558]` (0x64B634-0x64B63C); `ped flags |= 0x100` (bInVehicle, 0x64B644).
  3. If IsPlayer: `ped->m_pPlayerData (+0x480)->byte +0x8D = 1`; `ClearAdrenaline()` (0x64B653-0x64B662).
  4. If IsPlayer (called again) and `!(veh byte +0x42A & 2)`: set that bit; `CCrime::ReportCrime(6, veh, FindPlayerPed(-1))` (cdecl, 0x64B674-0x64B69C).
  5. Seat (EBP): if `veh byte +0x429 & 2`: seat = -1 (0x64B6A2-0x64B6AA). Else `seat = ComputePassengerIndexFromCarDoor(veh, m_nTargetDoor)`;
     if seat != -1 and `occ = veh->m_apPassengers[seat] (+0x464 + 4*seat)` is neither ped nor 0, and occ's task manager (intelligence +0x47C, +4)
     has NO active task of type 0x2C0, 0x338, 0x2C1, 0x33A (FindActiveTaskByType, in that order, each re-reading occ): build a local
     `CTaskSimpleCarSetPedOut(veh, ComputeTargetDoorToExit(veh, occ), 1)` (the `push 1` at 0x64B770 is left on the stack by the cdecl call
     and becomes the ctor's third argument), set its byte +0x11 = 1, `local.ProcessPed(occ)`, destroy it (0x64B6B2-0x64B7B7).
  6. If m_bWarpingInToCar: copy the vehicle position (matrix +0x30 or placement +4; loaded z, y, x into x87) into the ped's position
     (matrix +0x30/+0x34/+0x38 if the ped has a matrix, else placement +4/+8/+0xC) (0x64B7BC-0x64B7FB).
  7. `seat != -1 ? veh->AddPassenger(ped, seat) : veh->AddPassenger(ped)` (0x64B7FE-0x64B810).
  8. `ped->UpdateStatEnteringVehicle()`; `ped->SetMoveState(0)`; `ped->vtbl[0x5C/4]()` (slot 23, no args); `ped[+0x1C] &= ~1`
     (m_bUsesCollision off) (0x64B815-0x64B82C).
  9. If `(int16)veh[+0x45C] == -1` set it to 15000 (0x3A98) (0x64B830-0x64B83E). `ped->SetPedState(0x32)` (PEDSTATE_DRIVING 50).
  10. If m_nDoorFlagsToClear: `veh->ClearGettingInFlags(flags)`; if m_nNumGettingInToClear: `veh byte +0x485 -= n` (m_nNumGettingIn)
      (0x64B850-0x64B86C).
  11. `ped->RemoveWeaponWhenEnteringVehicle(0)`; `ped flags (+0x46C) bit 13 (0x2000)` = `!(veh byte +0x429 & 2)` (0x64B87B-0x64B899).
  12. `RemoveGetInAnims(ped)`; `AddInCarAnim(veh, ped, 0)` (cdecl, 0x64B89F-0x64B8AB).
  13. If !m_bWarpingInToCar: `m_pUtility->ProcessPed(ped, veh, 0)` (no null check, 0x64B8BA-0x64B8C4).
  14. `t = new(0x60) CTaskSimpleCarDrive(veh, m_pUtility, 0)` (0 on failure); `ped->m_pIntelligence->m_TaskMgr (+4).SetTask(t, 4, 0)`
      (0x64B8C9-0x64B90E). Return 1.
- gta-reversed: cpp:35 `plugin::CallMethodAndReturn` - port from these bytes.
- Open questions: semantic names of ped +0x558/+0x55C, ped vtable slot 23 (0x64B829), vehicle bytes +0x429/+0x42A bit 1 and +0x45C
  (behaviour above is complete without them).

## 0x649D90 CTaskSimpleCarSetPedInAsPassenger::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x20) 0x61A5A0 (cdecl); if p == NULL the constructor is skipped and EAX = 0 (see the trailing stores). Else p->CTaskSimpleCarSetPedInAsPassenger(this+0x10, this+0x14, this+0x18) 0x646FE0; then outside the NULL check (0x649DDA): p+0x1C/+0x1D/+0x1E = this+0x1C/+0x1D/+0x1E (bytes). On allocation failure EAX = 0 and these writes hit 0x1C.. (original bug, keep); return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
