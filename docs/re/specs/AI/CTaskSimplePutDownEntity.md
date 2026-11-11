# CTaskSimplePutDownEntity (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x6935C0 CTaskSimplePutDownEntity::~CTaskSimplePutDownEntity  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6935E0 thunk_FUN_00691630 (0x6935C3); if `flags & 1` (0x6935C8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x6935D0); returns this (0x6935D8). Proposed name: `thunk_FUN_00691630::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_00691630).
