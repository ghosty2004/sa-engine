# CTaskComplexClimb (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x46A6B0 CTaskComplexClimb::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 267 (0x10B) (0x46A6B0).

## 0x46A6C0 CTaskComplexClimb::~CTaskComplexClimb  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x46A6E0 CTaskComplexClimb::~CTaskComplexClimb (0x46A6C3); if `flags & 1` (0x46A6C8) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x46A6D0); returns this (0x46A6D8).

## 0x46A6E0 CTaskComplexClimb::~CTaskComplexClimb  (catalogue: specified/REA, port level 3)
- Complete destructor, `__thiscall void ()`: `this->vtbl = 0x859DCC` (0x46A6E0); tail-jump 0x67A060 FUN_0067a060 (unnamed complete dtor: stores vtbl 0x870570) (0x46A6E6).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x46A650 CTaskComplexClimb::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* ()`, `ret`, SEH: `p = CTask::operator new(0x14)` 0x61A5A0; null -> 0; else `CTaskComplexJump::Constructor(p, 1)` 0x67A030 (0x46A685), `p->vtbl = 0x859DCC`;
  return p. Nothing copied from `this`.
