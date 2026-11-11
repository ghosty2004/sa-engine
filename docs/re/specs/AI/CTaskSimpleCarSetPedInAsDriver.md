# CTaskSimpleCarSetPedInAsDriver (AI) - RE-SPEC-3 group I4

Layout (gr TaskSimpleCarSetPedInAsDriver.h, size 0x1C): +0x8 m_bIsFinished, +0xC m_pAnim, +0x10 m_pTargetVehicle, +0x14 m_pUtility,
+0x18 m_bWarpingInToCar (u8), +0x19 m_nDoorFlagsToClear (u8), +0x1A m_nNumGettingInToClear (u8).

## 0x647150 CTaskSimpleCarSetPedInAsDriver::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x32C` (0x647150); `ret` -> returns 812 (TASK_SIMPLE_CAR_SET_PED_IN_AS_DRIVER). Vtable: slot 4 (+0x10) of vtable 0x86EE28.

## 0x647160 CTaskSimpleCarSetPedInAsDriver::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x647160; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86EE28.

## 0x64B930 CTaskSimpleCarSetPedInAsDriver::~CTaskSimpleCarSetPedInAsDriver  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00647170 0x647170 (0x64B933); if `flags & 1` (0x64B938) calls CTask::operator delete 0x61A5B0 (this) (0x64B940, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86EE28.

## 0x64B950 CTaskSimpleCarSetPedInAsDriver::ProcessPed  (catalogue: specified/REA, port level 25)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`, always returns AL = 1. SEH (handler 0x843059). Locals: a CTaskSimpleCarSetPedOut
  (body [esp+0x10]), a CEventGroupEvent (body [esp+0x10]) and a CEventCopCarBeingStolen (body [esp+0x28]).
- Callees: as in CTaskSimpleCarSetPedInAsPassenger::ProcessPed 0x64B5D0, plus CPedGroups::GetPedsGroup 0x5F7E80, CPedGroupMembership::IsLeader
  0x5F69C0, CEvent::operator new 0x4B5620, CEventLeaderEnteredCarAsDriver ctor 0x48E1C0, CEventGroupEvent ctor 0x4ADFD0 / dtor 0x4AE070,
  CPedGroupIntelligence::AddEvent 0x5F7470, CVehicle::SetDriver 0x6D16A0, CVehicle::IsLawEnforcementVehicle 0x6D2370, CEventCopCarBeingStolen
  ctor 0x4B1740 / dtor 0x4B17F0, GetEventGlobalGroup 0x4ABA50, CEventGroup::Add 0x4AB420.
- Behaviour (veh = m_pTargetVehicle):
  1. `if (ped->m_pVehicle) CleanUpOldReference`; `ped->m_pVehicle = veh`; `veh->RegisterReference(&ped->m_pVehicle)` (0x64B970-0x64B98A).
  2. Current driver `d = veh->m_pDriver (+0x460)`: if d != ped and d != 0 and none of the active tasks 0x2C0, 0x338, 0x2C1, 0x33A is found
     on d's task manager (each re-reads veh->m_pDriver): local `CTaskSimpleCarSetPedOut(veh, ComputeTargetDoorToExit(veh, d), 1)` (the
     third argument is the `push 1` left by the cdecl call at 0x64BA30), byte +0x11 = 1, `ProcessPed(d)`, destroy (0x64B98F-0x64BA76).
  3. `ped[+0x55C] = ped[+0x558]` (0x64BA7B). If m_bWarpingInToCar: copy vehicle position to ped position (same code as the passenger task,
     0x64BA87-0x64BAC6).
  4. `g = CPedGroups::GetPedsGroup(ped)`; if g and `g->membership.IsLeader(ped)` and `g byte/dword +0x34 == 0`: `e = CEvent::operator new
     (0x18)`; if e `CEventLeaderEnteredCarAsDriver(e, veh)`; local `CEventGroupEvent(ped, e)`; `g->intelligence (+0x30).AddEvent(&local)`;
     destroy local (deletes e) (0x64BAC9-0x64BB49).
  5. `veh->SetDriver(ped)` 0x6D16A0; `ped flags (+0x46C) |= 0x100` (bInVehicle) (0x64BB53-0x64BB6A).
  6. If IsPlayer: `ped->m_pPlayerData (+0x480)->byte +0x8D = 1`; `ClearAdrenaline()`. If IsPlayer and `!(veh byte +0x42A & 2)`: set it;
     `CCrime::ReportCrime(6, veh, FindPlayerPed(-1))` (0x64BB70-0x64BBBF).
  7. `UpdateStatEnteringVehicle()`; `SetMoveState(0)`; `ped->vtbl[0x5C/4]()`; `ped[+0x1C] &= ~1` (m_bUsesCollision off) (0x64BBC2-0x64BBD9).
  8. `if ((int16)veh[+0x45C] == -1) veh[+0x45C] = 15000` (0x64BBDD-0x64BBEB). `veh byte +0x428 bit 4 (0x10) = !(veh byte +0x42D & 2)`
     (0x64BBF4-0x64BC0D). `ped->SetPedState(0x32)` (PEDSTATE_DRIVING).
  9. Vehicle status (entity byte +0x36, status = bits 3..7, type = bits 0..2) (0x64BC1C-0x64BC65): player -> status 0 (STATUS_PLAYER);
     otherwise: if status == 2 and `ped->m_nCreatedBy (+0x484) != 2` keep it; else if `veh[+0x594] == 6` (vehicle sub-type) status = 7,
     else status = 3.
  10. If m_nDoorFlagsToClear `veh->ClearGettingInFlags(flags)`; if m_nNumGettingInToClear `veh byte +0x485 -= n` (0x64BC68-0x64BC84).
  11. `RemoveGetInAnims(ped)`; `AddInCarAnim(veh, ped, 1)` (cdecl); `RemoveWeaponWhenEnteringVehicle(0)` (0x64BC8A-0x64BCA3).
  12. `ped flags bit 13 (0x2000)`: cleared if `(veh byte +0x429 & 2) && veh->m_nModelIndex (+0x22, int16) != 0x1AF (431)`, else set
      (0x64BCA8-0x64BCD3).
  13. `t = new(0x60) CTaskSimpleCarDrive(veh, m_bWarpingInToCar ? 0 : m_pUtility, 0)` (0 on failure; 0x64BCD9-0x64BD34);
      `ped->m_pIntelligence->m_TaskMgr.SetTask(t, 4, 0)` (0x64BD38-0x64BD4A). Unlike the passenger task, m_pUtility->ProcessPed is not called.
  14. If IsPlayer and veh and `veh->IsLawEnforcementVehicle()`: local `CEventCopCarBeingStolen(ped, veh)`; `GetEventGlobalGroup()->Add(&local,
      0)`; destroy local (0x64BD4F-0x64BD9B). Return 1.
- gta-reversed: TaskSimpleCarSetPedInAsDriver ProcessPed is not reversed (plugin call / absent): port from these bytes.
- Open questions: names of ped +0x558/+0x55C, ped vtable slot 23 (0x64BBD6), vehicle bytes +0x428/+0x429/+0x42A/+0x42D, +0x45C,
  +0x594 == 6 and group +0x34 (0x64BAEA); behaviour above is complete without them.
