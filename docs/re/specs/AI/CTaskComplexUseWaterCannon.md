# CTaskComplexUseWaterCannon (AI) - RE-SPEC-AI-2 group J2

## 0x6597A0 CTaskComplexUseWaterCannon::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x6D` (0x6597A0); `ret` -> returns 109 (TASK_COMPLEX_USE_WATER_CANNON). Vtable: slot 4 (+0x10) of vtable 0x86F5D0 (CTaskComplexUseWaterCannon).

## 0x65A620 CTaskComplexUseWaterCannon::~CTaskComplexUseWaterCannon  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_006597b0 0x6597B0 (0x65A623); if `flags & 1` (0x65A628) calls CTask::operator delete 0x61A5B0 (this) (0x65A630, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F5D0 (CTaskComplexUseWaterCannon).
