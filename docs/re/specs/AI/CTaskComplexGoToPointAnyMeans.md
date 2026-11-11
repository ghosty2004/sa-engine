# CTaskComplexGoToPointAnyMeans (AI) - RE-SPEC-AI-2 group J2

## 0x66B780 CTaskComplexGoToPointAnyMeans::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x396` (0x66B780); `ret` -> returns 918 (TASK_COMPLEX_GO_TO_POINT_ANY_MEANS). Vtable: slot 4 (+0x10) of vtable 0x86FF68 (CTaskComplexGoToPointAnyMeans).

## 0x6705B0 CTaskComplexGoToPointAnyMeans::~CTaskComplexGoToPointAnyMeans  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexGoToPointAnyMeans::Destructor 0x66B830 (0x6705B3); if `flags & 1` (0x6705B8) calls CTask::operator delete 0x61A5B0 (this) (0x6705C0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FF68 (CTaskComplexGoToPointAnyMeans).
