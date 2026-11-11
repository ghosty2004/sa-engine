# CTaskAllocatorKillThreatsBasicRandomGroup (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5F6900 CTaskAllocatorKillThreatsBasicRandomGroup::~CTaskAllocatorKillThreatsBasicRandomGroup  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5F6920 FUN_005f6920 (unnamed complete dtor: stores vtbl 0x86C80C) (0x5F6903); if `flags & 1` (0x5F6908) calls 0x69D990 CTaskAllocator::operator delete(this) (cdecl, 0x5F6910); returns this (0x5F6918).
