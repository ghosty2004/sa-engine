# CTaskSimpleEvasiveDive (AI) - RE-SPEC-AI-2 group J3

## 0x6535C0 CTaskSimpleEvasiveDive::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x1F7` (0x6535C0); `ret` -> returns 503 (TASK_SIMPLE_EVASIVE_DIVE). Vtable: slot 4 (+0x10) of vtable 0x86F244.

## 0x655F00 CTaskSimpleEvasiveDive::~CTaskSimpleEvasiveDive  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleEvasiveDive::~CTaskSimpleEvasiveDive 0x6535D0 (0x655F03); if `flags & 1` (0x655F08) calls CTask::operator delete 0x61A5B0 (this) (0x655F10, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F244.
