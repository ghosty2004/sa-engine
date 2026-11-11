# CTaskComplexDie (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x6300B0 CTaskComplexDie::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 217 (0xD9) (0x6300B0).

## 0x630580 CTaskComplexDie::ControlSubTask  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret 4` (one ignored stack arg): returns the dword at this+0x8 (CTaskComplex::m_pSubTask) (0x630580).

## 0x637910 CTaskComplexDie::~CTaskComplexDie  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x6300C0 FUN_006300c0 (unnamed complete dtor: stores vtbl 0x86DE78) (0x637913); if `flags & 1` (0x637918) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x637920); returns this (0x637928).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x636060 CTaskComplexDie::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* ()`, `ret`, SEH: `p = CTask::operator new(0x28)` 0x61A5A0; null -> 0 (0x636091); else `CTaskComplexDie::Constructor` 0x630040 with args in order
  (this+0xC weaponType, +0x10 animGroup, +0x14 animId, +0x18 blendDelta (raw), +0x1C animSpeed (raw), +0x20 & 1, (+0x20 >> 1) & 1, +0x24 fallToDeathDir, (+0x20 >> 2) & 1)
  (0x636093-0x6360D1); returns its EAX.
