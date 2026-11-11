# CTaskComplexOpenPassengerDoor (AI) - RE-SPEC-AI-2 group J2

## 0x640410 CTaskComplexOpenPassengerDoor::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x2D8` (0x640410); `ret` -> returns 728 (TASK_COMPLEX_CAR_OPEN_PASSENGER_DOOR). Vtable: slot 4 (+0x10) of vtable 0x86EB3C.

## 0x640420 CTaskComplexOpenPassengerDoor::~CTaskComplexOpenPassengerDoor  (catalogue: specified/REA, port level 3)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86EB3C` (0x640420); tail-jump to base dtor CTaskComplexEnterCar::Destructor 0x63DFA0 (0x640426). No other member cleanup.

## 0x643CC0 CTaskComplexOpenPassengerDoor::~CTaskComplexOpenPassengerDoor  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexOpenPassengerDoor::~CTaskComplexOpenPassengerDoor 0x640420 (0x643CC3); if `flags & 1` (0x643CC8) calls CTask::operator delete 0x61A5B0 (this) (0x643CD0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86EB3C.
