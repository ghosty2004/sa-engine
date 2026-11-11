# CTaskComplexDestroyCarArmed (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x628C60 CTaskComplexDestroyCarArmed::~CTaskComplexDestroyCarArmed  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x622010 CTaskComplexDestroyCarArmed::Destructor (0x628C63); if `flags & 1` (0x628C68) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x628C70); returns this (0x628C78).
