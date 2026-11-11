# CTaskSimpleGoToPointFine (AI) - RE-SPEC-AI-2 group J3

## 0x65EEE0 CTaskSimpleGoToPointFine::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x39E` (0x65EEE0); `ret` -> returns 926 (TASK_SIMPLE_GO_TO_POINT_FINE). Vtable: slot 4 (+0x10) of vtable 0x86F964 (CTaskSimpleGoToPointFine).

## 0x6634E0 CTaskSimpleGoToPointFine::~CTaskSimpleGoToPointFine  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0065eef0 0x65EEF0 (0x6634E3); if `flags & 1` (0x6634E8) calls CTask::operator delete 0x61A5B0 (this) (0x6634F0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F964 (CTaskSimpleGoToPointFine).
