# CTaskSimpleGangDriveBy (AI) - RE-SPEC-AI-2 group J4

## 0x6218B0 CTaskSimpleGangDriveBy::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3FE` (0x6218B0); `ret` -> returns 1022 (TASK_SIMPLE_GANG_DRIVEBY). Vtable: slot 4 (+0x10) of vtable 0x86D944.

## 0x6275E0 CTaskSimpleGangDriveBy::~CTaskSimpleGangDriveBy  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006218c0 0x6218C0 (0x6275E3); if `flags & 1` (0x6275E8) calls CTask::operator delete 0x61A5B0 (this) (0x6275F0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D944.
