# CTaskComplexCarDrive (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x63C9C0 CTaskComplexCarDrive::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 725 (0x2D5) (0x63C9C0).

## 0x642F80 CTaskComplexCarDrive::~CTaskComplexCarDrive  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63CA40 CTaskComplexCarDrive::Destructor (0x642F83); if `flags & 1` (0x642F88) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x642F90); returns this (0x642F98).
