# CTaskComplexUseMobilePhone (AI) - RE-SPEC-AI-2 group J3

## 0x6348D0 CTaskComplexUseMobilePhone::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x640` (0x6348D0); `ret` -> returns 1600 (TASK_COMPLEX_USE_MOBILE_PHONE). Vtable: slot 4 (+0x10) of vtable 0x86E454.

## 0x639660 CTaskComplexUseMobilePhone::~CTaskComplexUseMobilePhone  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006348e0 0x6348E0 (0x639663); if `flags & 1` (0x639668) calls CTask::operator delete 0x61A5B0 (this) (0x639670, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86E454.
