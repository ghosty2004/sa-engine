# CTaskComplexWaitForBus (AI) - RE-SPEC-AI-2 group J3

## 0x632920 CTaskComplexWaitForBus::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xED` (0x632920); `ret` -> returns 237 (TASK_COMPLEX_WAIT_FOR_BUS). Vtable: slot 4 (+0x10) of vtable 0x86E18C (CTaskComplexWaitForBus).

## 0x632A40 CTaskComplexWaitForBus::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`: returns `this->m_pSubTask` (+0x8) (0x632A40); ped unused. Vtable: slot 10 (+0x28) of vtable 0x86E18C (CTaskComplexWaitForBus).
