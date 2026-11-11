# CTaskSimpleDrownInCar (AI) - RE-SPEC-AI-2 group J3

## 0x62FF40 CTaskSimpleDrownInCar::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xD8` (0x62FF40); `ret` -> returns 216 (TASK_SIMPLE_DROWN_IN_CAR). Vtable: slot 4 (+0x10) of vtable 0x86DE54 (CTaskSimpleDrownInCar).

## 0x62FF50 CTaskSimpleDrownInCar::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x62FF50; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86DE54 (CTaskSimpleDrownInCar).

## 0x62FF60 CTaskSimpleDrownInCar::~CTaskSimpleDrownInCar  (catalogue: specified/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86DE54` (CTaskSimpleDrownInCar) (0x62FF60); tail-jump to base dtor CTaskSimple::~CTaskSimple 0x61A3A0 (0x62FF66). No other member cleanup.

## 0x6378F0 CTaskSimpleDrownInCar::~CTaskSimpleDrownInCar  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleDrownInCar::~CTaskSimpleDrownInCar 0x62FF60 (0x6378F3); if `flags & 1` (0x6378F8) calls CTask::operator delete 0x61A5B0 (this) (0x637900, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DE54 (CTaskSimpleDrownInCar).
