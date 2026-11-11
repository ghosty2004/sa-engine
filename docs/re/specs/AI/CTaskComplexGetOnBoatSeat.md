# CTaskComplexGetOnBoatSeat (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x64CE80 CTaskComplexGetOnBoatSeat::~CTaskComplexGetOnBoatSeat  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x649280 CTaskComplexGetOnBoatSeat::Destructor (0x64CE83); if `flags & 1` (0x64CE88) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x64CE90); returns this (0x64CE98).
