# CTaskComplexCarDriveMission (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x643260 CTaskComplexCarDriveMission::~CTaskComplexCarDriveMission  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63CCC0 CTaskComplexCarDriveMission::Destructor (0x643263); if `flags & 1` (0x643268) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x643270); returns this (0x643278).
