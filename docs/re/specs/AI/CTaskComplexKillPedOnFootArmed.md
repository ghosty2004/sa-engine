# CTaskComplexKillPedOnFootArmed (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x626FA0 CTaskComplexKillPedOnFootArmed::~CTaskComplexKillPedOnFootArmed  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x621250 CTaskComplexKillPedOnFootArmed::Destructor (0x626FA3); if `flags & 1` (0x626FA8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x626FB0); returns this (0x626FB8).
