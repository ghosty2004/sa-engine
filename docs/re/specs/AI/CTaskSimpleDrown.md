# CTaskSimpleDrown (AI) - RE-SPEC-AI-2 group J3

## 0x62FF00 CTaskSimpleDrown::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xD5` (0x62FF00); `ret` -> returns 213 (TASK_SIMPLE_DROWN). Vtable: slot 4 (+0x10) of vtable 0x86DE30 (CTaskSimpleDrown).

## 0x6378D0 CTaskSimpleDrown::~CTaskSimpleDrown  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0062ff10 0x62FF10 (0x6378D3); if `flags & 1` (0x6378D8) calls CTask::operator delete 0x61A5B0 (this) (0x6378E0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DE30 (CTaskSimpleDrown).
