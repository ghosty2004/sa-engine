# CTaskSimpleSetStayInSamePlace (AI) - RE-SPEC-AI-2 group J3

## 0x62F5B0 CTaskSimpleSetStayInSamePlace::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xCC` (0x62F5B0); `ret` -> returns 204 (TASK_SIMPLE_SET_STAY_IN_SAME_PLACE). Vtable: slot 4 (+0x10) of vtable 0x86DD50.

## 0x62F5C0 CTaskSimpleSetStayInSamePlace::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns true (1) in AL only (`mov al, 1` 0x62F5C0; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86DD50.

## 0x6374A0 CTaskSimpleSetStayInSamePlace::~CTaskSimpleSetStayInSamePlace  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0062f5d0 0x62F5D0 (0x6374A3); if `flags & 1` (0x6374A8) calls CTask::operator delete 0x61A5B0 (this) (0x6374B0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DD50.
