# CTaskSimpleHitHead (AI) - RE-SPEC-AI-2 group J3

## 0x653080 CTaskSimpleHitHead::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x1F4` (0x653080); `ret` -> returns 500 (TASK_SIMPLE_HIT_HEAD). Vtable: slot 4 (+0x10) of vtable 0x86F1D0 (CTaskSimpleHitHead).

## 0x655E30 CTaskSimpleHitHead::~CTaskSimpleHitHead  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleHitHead::~CTaskSimpleHitHead 0x653090 (0x655E33); if `flags & 1` (0x655E38) calls CTask::operator delete 0x61A5B0 (this) (0x655E40, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F1D0 (CTaskSimpleHitHead).
