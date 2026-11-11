# CTaskComplexGoToPointAnyMeans (AI) - RE-SPEC-AI-2 group J2

## 0x66B780 CTaskComplexGoToPointAnyMeans::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x396` (0x66B780); `ret` -> returns 918 (TASK_COMPLEX_GO_TO_POINT_ANY_MEANS). Vtable: slot 4 (+0x10) of vtable 0x86FF68 (CTaskComplexGoToPointAnyMeans).

## 0x6705B0 CTaskComplexGoToPointAnyMeans::~CTaskComplexGoToPointAnyMeans  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexGoToPointAnyMeans::Destructor 0x66B830 (0x6705B3); if `flags & 1` (0x6705B8) calls CTask::operator delete 0x61A5B0 (this) (0x6705C0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FF68 (CTaskComplexGoToPointAnyMeans).

## 0x66D1E0 CTaskComplexGoToPointAnyMeans::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x34) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexGoToPointAnyMeans::CTaskComplexGoToPointAnyMeans(this+0x18, &this->+0xC (CVector*), this+0x20, this+0x1C, this+0x24) 0x66B790; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
