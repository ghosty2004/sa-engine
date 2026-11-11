# CTaskComplexDestroyCar (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x628780 CTaskComplexDestroyCar::~CTaskComplexDestroyCar  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x621CB0 CTaskComplexDestroyCar::Destructor (0x628783); if `flags & 1` (0x628788) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x628790); returns this (0x628798).
