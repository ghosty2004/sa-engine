# CTaskComplexKillPedFromBoat (AI) - RE-SPEC-AI-2 group J2

## 0x622820 CTaskComplexKillPedFromBoat::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x408` (0x622820); `ret` -> returns 1032 (TASK_KILL_PED_FROM_BOAT). Vtable: slot 4 (+0x10) of vtable 0x86DA98 (CTaskComplexKillPedFromBoat).

## 0x6298E0 CTaskComplexKillPedFromBoat::~CTaskComplexKillPedFromBoat  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexKillPedFromBoat::Destructor 0x622830 (0x6298E3); if `flags & 1` (0x6298E8) calls CTask::operator delete 0x61A5B0 (this) (0x6298F0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DA98 (CTaskComplexKillPedFromBoat).
