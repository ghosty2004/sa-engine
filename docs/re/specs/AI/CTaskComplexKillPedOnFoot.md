# CTaskComplexKillPedOnFoot (AI) - RE-SPEC-AI-2 group J2

## 0x620EE0 CTaskComplexKillPedOnFoot::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3E8` (0x620EE0); `ret` -> returns 1000 (TASK_COMPLEX_KILL_PED_ON_FOOT). Vtable: slot 4 (+0x10) of vtable 0x86D894 (CTaskComplexKillPedOnFoot).

## 0x625E20 CTaskComplexKillPedOnFoot::~CTaskComplexKillPedOnFoot  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00620ef0 0x620EF0 (0x625E23); if `flags & 1` (0x625E28) calls CTask::operator delete 0x61A5B0 (this) (0x625E30, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D894 (CTaskComplexKillPedOnFoot).

## 0x626A80 CTaskComplexKillPedOnFoot::~CTaskComplexKillPedOnFoot  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexKillPedOnFootStealth::Destructor 0x620FF0 (0x626A83); if `flags & 1` (0x626A88) calls CTask::operator delete 0x61A5B0 (this) (0x626A90, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D8C0 (CTaskComplexKillPedOnFootStealth).
