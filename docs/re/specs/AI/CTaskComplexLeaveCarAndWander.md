# CTaskComplexLeaveCarAndWander (AI) - RE-SPEC-AI-2 group J2

## 0x63BE40 CTaskComplexLeaveCarAndWander::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x2C3` (0x63BE40); `ret` -> returns 707 (TASK_COMPLEX_LEAVE_CAR_AND_WANDER). Vtable: slot 4 (+0x10) of vtable 0x86E880 (CTaskComplexLeaveCarAndWander).

## 0x63BEB0 CTaskComplexLeaveCarAndWander::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x63BEB0); ped unused. Vtable: slot 10 (+0x28) of vtable 0x86E880 (CTaskComplexLeaveCarAndWander).
