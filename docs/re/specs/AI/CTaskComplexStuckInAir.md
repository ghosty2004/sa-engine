# CTaskComplexStuckInAir (AI) - RE-SPEC-AI-2 group J5

## 0x67FA60 CTaskComplexStuckInAir::~CTaskComplexStuckInAir  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0067ba70 0x67BA70 (0x67FA63); if `flags & 1` (0x67FA68) calls CTask::operator delete 0x61A5B0 (this) (0x67FA70, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x870608 (CTaskComplexStuckInAir).
