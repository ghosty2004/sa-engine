# CTaskComplexArrestPed (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x68B9F0 CTaskComplexArrestPed::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 1101 (0x44D) (0x68B9F0).

## 0x68CF60 CTaskComplexArrestPed::~CTaskComplexArrestPed  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x68BA00 CTaskComplexArrestPed::Destructor (0x68CF63); if `flags & 1` (0x68CF68) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x68CF70); returns this (0x68CF78).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x68CD70 CTaskComplexArrestPed::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* ()`, `ret`, SEH frame: `p = CTask::operator new(0x24)` 0x61A5A0 (cdecl, task pool, 0x68CD8B); if p == 0 return 0 (0x68CDA1); else `CTaskComplexArrestPed::Constructor` 0x68B990 on p with args (this+0x10) (0x68CDA9) and return its EAX (the ctor returns p).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x68CF80 CTaskComplexArrestPed::CreateSubTask  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`; SEH frame. Field +0x10 = ped to arrest (`T`). Dispatch: `taskType == 0x38B` direct; `taskType > 0x38B` via compares; else index `taskType - 0x2BF` (unsigned <= 0x7E) into byte table 0x68D2C4 / jump table 0x68D2B0. Allocation failures -> 0.
- By taskType:
  - `0x2BF` (DRAG_PED_FROM_CAR): `new(0x54) CTaskComplexDragPedFromCar(T, 100000)` 0x640430.
  - `0x2D2` (CAR_OPEN_DRIVER_DOOR): `new(0x50) CTaskComplexOpenDriverDoor(T->m_pVehicle)` 0x640390.
  - `0x2D8` (CAR_OPEN_PASSENGER_DOOR): `new(0x50) CTaskComplexOpenPassengerDoor(T->m_pVehicle, 8)` 0x6403E0.
  - `0x33D` (WAIT_UNTIL_PED_OUT_CAR): `VectorSub(&D, &T position, &ped position)` 0x40FE60 (before the allocation); `new(0x1C) CTaskSimpleWaitUntilPedIsOutCar(T, &D)` 0x649450.
  - `0x38B` (SEEK_ENTITY): `r = (T->+0x474 & 0x40 (bIsBeingArrested)) ? [0x870980]=4.0f : [0x87097C]=3.0f`; `new(0x4C) CTaskComplexSeekEntity<Standard>(T, 50000, 1000, r, [0x859E30]=2.0f, [0x859E34]=2.0f, 1, 1)` 0x46AC10.
  - `0x3E8` (KILL_PED_ON_FOOT): `new(0x38) CTaskComplexKillPedOnFoot(T, -1, 0, 0, 0, 1)` 0x620E30.
  - `0x3EB` (DESTROY_CAR): `new(0x20) CTaskComplexDestroyCar(T->m_pVehicle, 0, 0, 0)` 0x621C00.
  - `0x44C` (ARREST_PED): if `T->m_pVehicle && CVehicle::IsDriver(T->m_pVehicle, T)` 0x6D1C40: `veh->+0x428 |= 0x20`; `veh->+0x36 = (veh->+0x36 & 7) | 0x48` (entity status bits set to 9, type bits kept). Then `new(0x14) CTaskSimpleArrestPed(T)` 0x68B620.
  - anything else (table entry 4) -> 0.
