# CEventDontJoinPlayerGroup (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x6091A0 CEventDontJoinPlayerGroup::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 94 (0x5E) (0x6091A0).

## 0x6091B0 CEventDontJoinPlayerGroup::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x6091B0).

## 0x609220 CEventDontJoinPlayerGroup::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x609220).

## 0x609230 CEventDontJoinPlayerGroup::~CEventDontJoinPlayerGroup  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x609140 FUN_00609140 (unnamed complete dtor: stores vtbl 0x86D0E0) (0x609233); if `flags & 1` (0x609238) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x609240); returns this (0x609248).

## 0x609320 CEventDontJoinPlayerGroup::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 43 (0x2B) (0x609320).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x6091C0 CEventDontJoinPlayerGroup::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame: `p = CEvent::operator new(0x10)` 0x4B5620 (cdecl, event pool, 0x6091DB); if p == 0 return 0 (0x6091F1); else `CEventDontJoinPlayerGroup::Constructor` 0x6090E0 on p with args (this+0xC) (0x6091F9) and return its EAX (the ctor returns p).
