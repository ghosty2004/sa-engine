# CTaskSimpleSlideToCoord (AI) - RE-SPEC-AI-2 group J3

## 0x66C440 CTaskSimpleSlideToCoord::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3A6` (0x66C440); `ret` -> returns 934 (TASK_SIMPLE_SLIDE_TO_COORD). Vtable: slot 4 (+0x10) of vtable 0x86FFEC (CTaskSimpleSlideToCoord).

## 0x6712C0 CTaskSimpleSlideToCoord::~CTaskSimpleSlideToCoord  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0066c4c0 0x66C4C0 (0x6712C3); if `flags & 1` (0x6712C8) calls CTask::operator delete 0x61A5B0 (this) (0x6712D0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FFEC (CTaskSimpleSlideToCoord).
