# CTaskComplexMedicTreatInjuredPed (AI) - RE-SPEC-AI-2 group J2

## 0x658C20 CTaskComplexMedicTreatInjuredPed::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x64` (0x658C20); `ret` -> returns 100 (TASK_COMPLEX_MEDIC_TREAT_INJURED_PED). Vtable: slot 4 (+0x10) of vtable 0x86F518 (CTaskComplexMedicTreatInjuredPed).

## 0x659FC0 CTaskComplexMedicTreatInjuredPed::~CTaskComplexMedicTreatInjuredPed  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexMedicTreatInjuredPed::~CTaskComplexMedicTreatInjuredPed 0x658C30 (0x659FC3); if `flags & 1` (0x659FC8) calls CTask::operator delete 0x61A5B0 (this) (0x659FD0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F518 (CTaskComplexMedicTreatInjuredPed).
