# CTaskComplexEnterCarAsPassengerWait (AI) - RE-SPEC-FINAL (helper B)

## 0x640730 CTaskComplexEnterCarAsPassengerWait::ControlSubTask  (catalogue: specified/REA, port level 6)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; always returns `m_pSubTask` (+8). Fields: +0xC m_pTargetVehicle, +0x10 m_pPedToWaitFor, +0x20 (u8) "said line" flag.
- Behaviour:
  1. Speech (only while `+0x20 == 0`): if `m_pPedToWaitFor != NULL`: if `CPed::IsPlayer(m_pPedToWaitFor)` and `CTaskManager::FindActiveTaskByType(&ped->m_pIntelligence->m_TaskMgr (+0x47C, +4), 0x327 TASK_SIMPLE_CAR_GET_IN)` 0x681740 != NULL -> `+0x20 = 1` and `CPed::Say(ped, 0x23, 0, 1.0f, 0, 0, 0)`. Else if `m_pPedToWaitFor == NULL`: `g = CPedGroups::GetPedsGroup(ped)` 0x5F7E80; if g and `l = CPedGroupMembership::GetLeader(&g->+8)` 0x5F69A0 and `CPed::IsPlayer(l)` and the same FindActiveTaskByType(0x327) is non-NULL -> `+0x20 = 1` and `Say(ped, 0x28, 0, 1.0f, 0, 0, 0)`.
  2. If `m_pTargetVehicle != NULL` and `m_pSubTask->GetTaskType() == 0x387` (TASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL): `found = CPedGeometryAnalyser::ComputeClosestSurfacePoint(ped, m_pTargetVehicle, &L)` 0x5F3B70 (cdecl; AL). `d = L - subtask->m_vecTargetPoint (+0x10/+0x14/+0x18)`; `q = (dz*dz + dy*dy) + dx*dx`; `fcomp [0x859038]=0.025f`. The following `je` (0x640849) tests the ZF of `test al, al` (x87 ops do not touch EFLAGS), i.e. branches on `found`:
     - found: if `q > 0.025` (`test ah, 0x41; jne` skip) -> `CTaskComplexGoToPointAndStandStill::GoToPoint(subtask, &L, 1.5f (0x3FC00000), [0x86FC88]=2.0f, false)` 0x46FE60.
     - not found: if `q > 0.025` (computed from L, which may not have been written by the callee - original behaviour) -> `r = CModelInfo::ms_modelInfoPtrs[vehicle->m_nModelIndex]->m_pColModel (+0x14)->+0x24 (bounding radius) + [0x858CE8]=1.5f` (f32 store); `GoToPoint(subtask, &vehicle position, r, 2.0f, false)`.
  3. Return m_pSubTask.

## 0x6408D0 CTaskComplexEnterCarAsPassengerWait::CreateSubTask  (catalogue: specified/REA, port level 6)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`; SEH frame (epilogue partly in a .HOODLUM detour 0x401E7E). Fields (gr ctor order (target, waitFor, forceFrontSeat, moveState)): +0xC m_pTargetVehicle, +0x10 m_pPedToWaitFor, +0x18 (u8) forceFrontSeat, +0x1C move state. Allocation failures give 0 except where noted.
- By taskType:
  - `0x2CF` (TASK_SIMPLE_CAR_DRIVE_TIMED): `new(0x70) CTaskSimpleCarDriveTimed(m_pTargetVehicle, 0)` 0x5FF940.
  - `0x2C8` (TASK_COMPLEX_ENTER_CAR_AS_PASSENGER_TIMED): `door = forceFrontSeat ? CCarEnterExit::ComputeTargetDoorToEnterAsPassenger(m_pTargetVehicle, 0) 0x64F190 : 0`; `p = new(0x2C)`; `EAX = p ? CTaskComplexEnterCarAsPassengerTimed(p, m_pTargetVehicle, door, -1, 1) : 0` 0x63B030; then `EAX->+0x1C = this->+0x1C` - executed also when EAX == 0 (original bug: NULL write to address 0x1C on allocation failure).
  - `0xF4` (TASK_COMPLEX_SEQUENCE): `seq = new(0x40) CTaskComplexSequence()` (NULL on failure; AddTask is then called on NULL); `seq->AddTask(new(0x28) CTaskComplexTurnToFaceEntityOrCoord(m_pTargetVehicle, [0x86FC7C]=0.5f, [0x86FC80]=0.2f))`; `seq->AddTask(new(0x20) CTaskSimpleStandStill(1000, false, false, 8.0f))`; return seq.
  - `0x341` (TASK_SIMPLE_WAIT_UNTIL_PED_IN_CAR): `new(0x10) CTaskSimpleWaitUntilPedIsInCar(m_pPedToWaitFor)` 0x649620.
  - `0x387` (TASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL): if `CPedGeometryAnalyser::ComputeClosestSurfacePoint(ped, m_pTargetVehicle, &L)` 0x5F3B70 -> `new(0x28) CTaskComplexGoToPointAndStandStill(+0x1C, &L, 1.5f, [0x86FC88]=2.0f, false, false)`; else `r = CModelInfo::ms_modelInfoPtrs[veh->m_nModelIndex]->m_pColModel->+0x24 (radius) + [0x858CE8]=1.5f` (f32) and `new(0x28) CTaskComplexGoToPointAndStandStill(+0x1C, &vehicle position, r, 2.0f, false, false)`.
  - `0x516` or anything else -> 0.

## 0x643E10 CTaskComplexEnterCarAsPassengerWait::CreateNextSubTask  (catalogue: specified/REA, port level 7)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. CreateSubTask = 0x6408D0 (thiscall (type, ped)). Fields: +0xC m_pTargetVehicle, +0x10 m_pPedToWaitFor, +0x14 attempt counter.
- Behaviour: if `m_pTargetVehicle == NULL` -> `CreateSubTask(0x516)`. Else by `t = m_pSubTask->GetTaskType()`:
  - `0x2CF` (TASK_SIMPLE_CAR_DRIVE_TIMED) -> `CreateSubTask(0x516)`.
  - `0xF4` (TASK_COMPLEX_SEQUENCE) or `0x341` (TASK_SIMPLE_WAIT_UNTIL_PED_IN_CAR) -> `CreateSubTask(m_pTargetVehicle ? 0x2C8 : 0x516)`.
  - `0x2C8` (TASK_COMPLEX_ENTER_CAR_AS_PASSENGER_TIMED): if `ped->bInVehicle (+0x46C & 0x100)` -> `CreateSubTask(0x516)`; else `+0x14 += 1`; if `(int32)+0x14 > 15` -> `CreateSubTask(0x516)`; else `CreateSubTask(m_pTargetVehicle ? 0xF4 : 0x516)`.
  - `0x387` (TASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL) -> `CreateSubTask(m_pPedToWaitFor ? 0x341 : 0x516)`.
  - anything else -> 0.

## 0x643F00 CTaskComplexEnterCarAsPassengerWait::CreateFirstSubTask  (catalogue: specified/REA, port level 7)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Fields: +0xC m_pTargetVehicle, +0x10 m_pPedToWaitFor.
- Behaviour: `if ped->bInVehicle (+0x46C & 0x100) && ped->m_pVehicle (+0x58C) != NULL && ped->m_pVehicle == m_pTargetVehicle && CVehicle::IsPassenger(m_pTargetVehicle, ped)` 0x6D1BD0 -> return `CreateSubTask(this, 0x2CF TASK_SIMPLE_CAR_DRIVE_TIMED, ped)` 0x6408D0. Else: `if m_pPedToWaitFor == NULL`: return `CreateSubTask(m_pTargetVehicle ? 0x2C8 (TASK_COMPLEX_ENTER_CAR_AS_PASSENGER_TIMED) : 0x516 (TASK_FINISHED), ped)`; else return `CreateSubTask(m_pTargetVehicle ? 0x387 (TASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL) : 0x516, ped)`.
