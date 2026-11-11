# CTaskSimpleSetCharIgnoreWeaponRangeFlag (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4748D0 CTaskSimpleSetCharIgnoreWeaponRangeFlag::~CTaskSimpleSetCharIgnoreWeaponRangeFlag  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4748F0 CTaskSimpleSetCharIgnoreWeaponRangeFlag::Destructor (0x4748D3); if `flags & 1` (0x4748D8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x4748E0); returns this (0x4748E8).
