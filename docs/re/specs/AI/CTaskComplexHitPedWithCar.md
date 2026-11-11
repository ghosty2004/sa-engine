# CTaskComplexHitPedWithCar (AI) - RE-SPEC-AI-2 group J2

## 0x653A20 CTaskComplexHitPedWithCar::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x1F9` (0x653A20); `ret` -> returns 505 (TASK_COMPLEX_HIT_PED_WITH_CAR). Vtable: slot 4 (+0x10) of vtable 0x86F294 (CTaskComplexHitPedWithCar).

## 0x653A90 CTaskComplexHitPedWithCar::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x653A90); ped unused. Vtable: slot 10 (+0x28) of vtable 0x86F294 (CTaskComplexHitPedWithCar).
