# CTaskComplexGoToBoatSteeringWheel (AI) - RE-SPEC-AI-2 group J2

## 0x6490F0 CTaskComplexGoToBoatSteeringWheel::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x33E` (0x6490F0); `ret` -> returns 830 (TASK_COMPLEX_GO_TO_BOAT_STEERING_WHEEL). Vtable: slot 4 (+0x10) of vtable 0x86F018 (CTaskComplexGoToBoatSteeringWheel).

## 0x64CD80 CTaskComplexGoToBoatSteeringWheel::~CTaskComplexGoToBoatSteeringWheel  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexGoToBoatSteeringWheel::Destructor 0x649100 (0x64CD83); if `flags & 1` (0x64CD88) calls CTask::operator delete 0x61A5B0 (this) (0x64CD90, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F018 (CTaskComplexGoToBoatSteeringWheel).
