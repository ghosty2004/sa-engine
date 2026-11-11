# CTaskComplexGetUpAndStandStill (AI) - RE-SPEC-AI-2 group J2

## 0x678150 CTaskComplexGetUpAndStandStill::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xCE` (0x678150); `ret` -> returns 206 (TASK_COMPLEX_GET_UP_AND_STAND_STILL). Vtable: slot 4 (+0x10) of vtable 0x870404 (CTaskComplexGetUpAndStandStill).

## 0x67CA00 CTaskComplexGetUpAndStandStill::~CTaskComplexGetUpAndStandStill  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00678160 0x678160 (0x67CA03); if `flags & 1` (0x67CA08) calls CTask::operator delete 0x61A5B0 (this) (0x67CA10, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870404 (CTaskComplexGetUpAndStandStill).
