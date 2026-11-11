# CTaskComplexGoToPointShooting (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x66DF80 CTaskComplexGoToPointShooting::~CTaskComplexGoToPointShooting  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x66DFA0 thunk_FUN_00668870 (0x66DF83); if `flags & 1` (0x66DF88) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x66DF90); returns this (0x66DF98). Proposed name: `thunk_FUN_00668870::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_00668870).
