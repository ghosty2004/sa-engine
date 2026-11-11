# CEventAcquaintancePedHateBadlyLit (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5FF2D0 CEventAcquaintancePedHateBadlyLit::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 63 (0x3F) (0x5FF2D0).

## 0x5FF350 CEventAcquaintancePedHateBadlyLit::CanBeInterruptedBySameEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool (0 stack args)`, `ret`: returns true (AL=1; upper EAX bits unchanged) (0x5FF350).

## 0x5FF360 CEventAcquaintancePedHateBadlyLit::~CEventAcquaintancePedHateBadlyLit  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5FF2C0 FUN_005ff2c0 (unnamed complete dtor: stores vtbl 0x86CB10) (0x5FF363); if `flags & 1` (0x5FF368) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x5FF370); returns this (0x5FF378).

## 0x5FF8A0 CEventAcquaintancePedHateBadlyLit::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 25 (0x19) (0x5FF8A0).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5FF2E0 CEventAcquaintancePedHateBadlyLit::CloneEditable  (catalogue: specified/REA, port level 4)
- `__thiscall CEvent* ()`, `ret`, SEH frame: `p = CEvent::operator new(0x28)` 0x4B5620 (cdecl, event pool, 0x5FF2FB); if p == 0 return 0 (0x5FF311); else `CEventAcquaintancePedHateBadlyLit::Constructor` 0x5FF250 on p with args (this+0x14, this+0x18, &this+0x1C) (0x5FF321) and return its EAX (the ctor returns p).
