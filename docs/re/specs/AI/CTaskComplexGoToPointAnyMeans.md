# CTaskComplexGoToPointAnyMeans (AI) - RE-SPEC-AI-2 group J2

## 0x66B780 CTaskComplexGoToPointAnyMeans::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x396` (0x66B780); `ret` -> returns 918 (TASK_COMPLEX_GO_TO_POINT_ANY_MEANS). Vtable: slot 4 (+0x10) of vtable 0x86FF68 (CTaskComplexGoToPointAnyMeans).

## 0x6705B0 CTaskComplexGoToPointAnyMeans::~CTaskComplexGoToPointAnyMeans  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexGoToPointAnyMeans::Destructor 0x66B830 (0x6705B3); if `flags & 1` (0x6705B8) calls CTask::operator delete 0x61A5B0 (this) (0x6705C0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FF68 (CTaskComplexGoToPointAnyMeans).

## 0x66D1E0 CTaskComplexGoToPointAnyMeans::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x34) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexGoToPointAnyMeans::CTaskComplexGoToPointAnyMeans(this+0x18, &this->+0xC (CVector*), this+0x20, this+0x1C, this+0x24) 0x66B790; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x6705D0 CTaskComplexGoToPointAnyMeans::CreateSubTask  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`; SEH frame (entry `nop; jmp` .HOODLUM stub). Fields: +0xC target point, +0x18 move state, +0x1C target radius, +0x20 vehicle to use, +0x24 car model id. Allocation failures -> 0.
- By taskType:
  - `0x2C6` (TASK_COMPLEX_CAR_DRIVE_TO_POINT): `new(0x3C) CTaskComplexDriveToPoint(+0x20, &target, -1.0f, 0, -1, -1.0f, 0)` 0x63CE00.
  - `0x2C0` (LEAVE_CAR): `new(0x34) CTaskComplexLeaveCar(ped->m_pVehicle, 0, 0, true, false)` 0x63B8C0.
  - `0x2BD` (ENTER_CAR_AS_DRIVER): `new(0x50) CTaskComplexEnterCarAsDriver(+0x20)` 0x6402F0.
  - `0x340` (TASK_SIMPLE_CREATE_CAR_AND_GET_IN): `new(0x34) CTaskSimpleCreateCarAndGetIn(&ped position (matrix pos or placement), +0x24)` 0x6493E0.
  - `0x38A` (FOLLOW_NODE_ROUTE): `new(0x60) CTaskComplexFollowNodeRoute(+0x18, &target, +0x1C, [0x86FCA4]=3.0f, [0x86FCA8]=2.0f, false, -1, true)` 0x66EA30.
  - `0x387` (GO_TO_POINT_AND_STAND_STILL): `new(0x28) CTaskComplexGoToPointAndStandStill(+0x18, &target, +0x1C, [0x86FC88]=2.0f, false, false)` 0x668120.
  - `0x516` or anything else -> 0.

## 0x6728A0 CTaskComplexGoToPointAnyMeans::CreateNextSubTask  (catalogue: specified/REA, port level 5)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. CreateSubTask = 0x6705D0 (thiscall (type, ped)).
- Behaviour by `t = m_pSubTask->GetTaskType()`:
  - `0x2BD` (TASK_COMPLEX_ENTER_CAR_AS_DRIVER): `ped->bInVehicle (+0x46C & 0x100) && ped->m_pVehicle (+0x58C)` -> `CreateSubTask(0x2C6 TASK_COMPLEX_CAR_DRIVE_TO_POINT)`, else `CreateSubTask(0x38A TASK_COMPLEX_FOLLOW_NODE_ROUTE)`.
  - `0x2C0` (TASK_COMPLEX_LEAVE_CAR) -> `CreateSubTask(0x387)`.
  - `0x2C6` -> `CreateSubTask(0x2C0)`.
  - `0x340` (TASK_SIMPLE_CREATE_CAR_AND_GET_IN): `ped->bInVehicle` -> `CreateSubTask(0x2C6)`, else `CreateSubTask(0x38A)` (vehicle pointer not tested here).
  - `0x387` or `0x38A` -> `CreateSubTask(0x516)`.
  - `0x516` or anything else -> 0.

## 0x6729C0 CTaskComplexGoToPointAnyMeans::CreateFirstSubTask  (catalogue: specified/REA, port level 5)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Field +0x20 = vehicle to use.
- Behaviour: if `+0x20 != NULL`: return `CreateSubTask(this, (ped->m_pVehicle (+0x58C) != NULL && ped->bInVehicle (+0x46C & 0x100)) ? 0x2C6 (TASK_COMPLEX_CAR_DRIVE_TO_POINT) : 0x2BD (TASK_COMPLEX_ENTER_CAR_AS_DRIVER), ped)` 0x6705D0. Else: if `ped->m_pVehicle && ped->bInVehicle && CVehicle::IsDriver(ped->m_pVehicle, ped)` 0x6D1C40 -> `CreateSubTask(0x2C6, ped)`, else `CreateSubTask(0x38A TASK_COMPLEX_FOLLOW_NODE_ROUTE, ped)`.

## 0x672A50 CTaskComplexGoToPointAnyMeans::ControlSubTask  (catalogue: specified/REA, port level 10)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. Fields: +0xC..+0x14 target point, +0x20 vehicle to use (registered reference), +0x24 int (-1 = no car-creation timer), +0x28/+0x2C/+0x30/+0x31 CTaskTimer {start, interval, started, reset}.
- Behaviour: `s = m_pSubTask`; if `s->GetTaskType() != 0x38A` (TASK_COMPLEX_FOLLOW_NODE_ROUTE) -> return s.
  1. Timer: if `+0x24 != -1` and `!started` -> `start = now ([0xB7CB84]); interval = 3000 (0xBB8); started = 1`. If `started` (also when +0x24 == -1 but the timer was started earlier): `if reset` -> `start = now; reset = 0`; if `(u32)now >= start + interval` -> `r = CreateSubTask(this, 0x340 TASK_SIMPLE_CREATE_CAR_AND_GET_IN, ped)`; then restart the timer (`interval = 3000; started = 1; start = now`); return r.
  2. Otherwise: `d = pedPos - target`; `q = (dz*dz + dx*dx) + dy*dy`; `t = [0x86FD04]=50.0f * 50.0f`; `fcompp` (t vs q): if not `t < q` (i.e. q <= 2500 or NaN) -> return s.
  3. `v = 0x5FFEE0(&ped->m_pIntelligence->+0xD4)` (vehicle-scanner query, thiscall; returns a CVehicle*); if NULL or `v == +0x20` -> return s. If `!CCarEnterExit::IsVehicleStealable(v, ped)` 0x6510D0 -> return s. Else `+0x20 = v` (raw store; the old reference is NOT cleaned up); `r = CreateSubTask(this, 0x2BD, ped)`; `CEntity::RegisterReference(v, &+0x20)` 0x571B70; return r.
