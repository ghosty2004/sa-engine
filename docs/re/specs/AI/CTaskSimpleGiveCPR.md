# CTaskSimpleGiveCPR (AI) - RE-SPEC-AI-2 group J3

## 0x658890 CTaskSimpleGiveCPR::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x66` (0x658890); `ret` -> returns 102 (TASK_SIMPLE_GIVE_CPR). Vtable: slot 4 (+0x10) of vtable 0x86F4C8 (CTaskSimpleGiveCPR).

## 0x659E10 CTaskSimpleGiveCPR::~CTaskSimpleGiveCPR  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006588a0 0x6588A0 (0x659E13); if `flags & 1` (0x659E18) calls CTask::operator delete 0x61A5B0 (this) (0x659E20, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F4C8 (CTaskSimpleGiveCPR).
