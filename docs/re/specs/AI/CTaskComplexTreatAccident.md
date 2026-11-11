# CTaskComplexTreatAccident (AI) - RE-SPEC-AI-2 group J2

## 0x658AD0 CTaskComplexTreatAccident::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x65` (0x658AD0); `ret` -> returns 101 (TASK_COMPLEX_TREAT_ACCIDENT). Vtable: slot 4 (+0x10) of vtable 0x86F4EC (CTaskComplexTreatAccident).

## 0x658AE0 CTaskComplexTreatAccident::~CTaskComplexTreatAccident  (catalogue: specified/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86F4EC` (CTaskComplexTreatAccident) (0x658AE0); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x658AE6). No other member cleanup.

## 0x659E70 CTaskComplexTreatAccident::~CTaskComplexTreatAccident  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexTreatAccident::~CTaskComplexTreatAccident 0x658AE0 (0x659E73); if `flags & 1` (0x659E78) calls CTask::operator delete 0x61A5B0 (this) (0x659E80, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F4EC (CTaskComplexTreatAccident).
