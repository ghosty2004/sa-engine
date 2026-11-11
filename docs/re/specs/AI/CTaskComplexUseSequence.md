# CTaskComplexUseSequence (AI) - RE-SPEC-AI-2 group J2

## 0x635490 CTaskComplexUseSequence::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x113` (0x635490); `ret` -> returns 275 (TASK_COMPLEX_USE_SEQUENCE). Vtable: slot 4 (+0x10) of vtable 0x86E518 (CTaskComplexUseSequence).

## 0x639F90 CTaskComplexUseSequence::~CTaskComplexUseSequence  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006396c0 0x6396C0 (0x639F93); if `flags & 1` (0x639F98) calls CTask::operator delete 0x61A5B0 (this) (0x639FA0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86E518 (CTaskComplexUseSequence).
