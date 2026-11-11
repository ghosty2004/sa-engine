# CTaskSimpleClimb (AI) - RE-SPEC-AI-2 group J3

## 0x67A1C0 CTaskSimpleClimb::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xFE` (0x67A1C0); `ret` -> returns 254 (TASK_SIMPLE_CLIMB). Vtable: slot 4 (+0x10) of vtable 0x87059C (CTaskSimpleClimb).

## 0x67DBC0 CTaskSimpleClimb::~CTaskSimpleClimb  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleClimb::~CTaskSimpleClimb 0x67A1D0 (0x67DBC3); if `flags & 1` (0x67DBC8) calls CTask::operator delete 0x61A5B0 (this) (0x67DBD0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x87059C (CTaskSimpleClimb).
