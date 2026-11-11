# CTaskSimpleArrestPed (AI) - RE-SPEC-AI-2 group J3

## 0x68B680 CTaskSimpleArrestPed::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x44C` (0x68B680); `ret` -> returns 1100 (TASK_SIMPLE_ARREST_PED). Vtable: slot 4 (+0x10) of vtable 0x870984 (CTaskSimpleArrestPed).

## 0x68CF40 CTaskSimpleArrestPed::~CTaskSimpleArrestPed  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleArrestPed::~CTaskSimpleArrestPed 0x68B690 (0x68CF43); if `flags & 1` (0x68CF48) calls CTask::operator delete 0x61A5B0 (this) (0x68CF50, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870984 (CTaskSimpleArrestPed).
