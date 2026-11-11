# CTaskSimpleDieInCar (AI) - RE-SPEC-AI-2 group J3

## 0x62FC60 CTaskSimpleDieInCar::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xD6` (0x62FC60); `ret` -> returns 214 (TASK_SIMPLE_DIE_IN_CAR). Vtable: slot 4 (+0x10) of vtable 0x86DDE0.

## 0x6375B0 CTaskSimpleDieInCar::~CTaskSimpleDieInCar  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0062fc70 0x62FC70 (0x6375B3); if `flags & 1` (0x6375B8) calls CTask::operator delete 0x61A5B0 (this) (0x6375C0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DDE0.

## 0x6398F0 CTaskSimpleDieInCar::ProcessPed  (catalogue: specified/REA, port level 11)
- Tail-jump thunk (0x6398F0): `jmp CTaskSimpleDie::ProcessPed 0x6397E0` with all registers and stack args unchanged (same convention as the target). Vtable: slot 7 (+0x1C) of vtable 0x86DDE0.
