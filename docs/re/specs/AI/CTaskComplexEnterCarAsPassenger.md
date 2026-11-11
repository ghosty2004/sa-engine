# CTaskComplexEnterCarAsPassenger (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x640370 CTaskComplexEnterCarAsPassenger::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 700 (0x2BC) (0x640370).

## 0x643C80 CTaskComplexEnterCarAsPassenger::~CTaskComplexEnterCarAsPassenger  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x640380 FUN_00640380 (unnamed complete dtor: stores vtbl 0x86EADC) (0x643C83); if `flags & 1` (0x643C88) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x643C90); returns this (0x643C98).
