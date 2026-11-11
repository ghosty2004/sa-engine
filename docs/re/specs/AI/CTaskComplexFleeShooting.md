# CTaskComplexFleeShooting (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x65D2A0 CTaskComplexFleeShooting::`scalar deleting destructor'  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x65C9A0 CTaskComplexFleeShooting::Destructor (0x65D2A3); if `flags & 1` (0x65D2A8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x65D2B0); returns this (0x65D2B8).
