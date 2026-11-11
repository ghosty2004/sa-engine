# CTaskComplexUseEffectRunning (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x636830 CTaskComplexUseEffectRunning::~CTaskComplexUseEffectRunning  (catalogue: specified/REA, port level 17)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x636850 FUN_00636850 (unnamed complete dtor: stores vtbl 0x86E594) (0x636833); if `flags & 1` (0x636838) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x636840); returns this (0x636848).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x636780 CTaskComplexUseEffectRunning::CTaskComplexUseEffectRunning  (catalogue: specified/REA, port level 3)
- `__thiscall T* (this, a1 [esp+4], a2 [esp+8])`, `ret 8`. Caller: CTaskComplexUseClosestFreeScriptedAttractor::CreateFirstSubTask 0x639530.
- Behaviour: `CTaskComplexUseEffect::CTaskComplexUseEffect(this, a1, a2)` 0x6321F0; `vtbl = 0x86E594; +0x1C = 6` (move state RUN); return this.
