# CTaskComplexWanderMedic (AI) - RE-SPEC-AI-2 group J3

## 0x658810 CTaskComplexWanderMedic::GetWanderType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x2` (0x658810); `ret` -> returns 2. Vtable: slot 11 (+0x2C) of vtable 0x86F48C.

## 0x658820 CTaskComplexWanderMedic::ScanForStuff  (catalogue: specified/REA, port level 0)
- Empty body: `ret 4` (0x658820). Vtable: slot 12 (+0x30) of vtable 0x86F48C.

## 0x658830 CTaskComplexWanderMedic::~CTaskComplexWanderMedic  (catalogue: specified/REA, port level 4)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00658850 0x658850 (0x658833); if `flags & 1` (0x658838) calls CTask::operator delete 0x61A5B0 (this) (0x658840, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86F48C.
