# CEventLeaderEnteredCarAsDriver (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x48E280 CEventLeaderEnteredCarAsDriver::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 54 (0x36) (0x48E280).

## 0x48E290 CEventLeaderEnteredCarAsDriver::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x48E290).

## 0x48E300 CEventLeaderEnteredCarAsDriver::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns false (AL=0; upper EAX bits unchanged) (0x48E300).

## 0x48E310 CEventLeaderEnteredCarAsDriver::CanBeInterruptedBySameEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool (0 stack args)`, `ret`: returns true (AL=1; upper EAX bits unchanged) (0x48E310).

## 0x48E320 CEventLeaderEnteredCarAsDriver::~CEventLeaderEnteredCarAsDriver  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x48E220 FUN_0048e220 (unnamed complete dtor: stores vtbl 0x85A198) (0x48E323); if `flags & 1` (0x48E328) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x48E330); returns this (0x48E338).

## 0x48E340 CEventLeaderEnteredCarAsDriver::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 46 (0x2E) (0x48E340).

## 0x4B0EE0 CEventLeaderEnteredCarAsDriver::TakesPriorityOver  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4B0EE0).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x48E2A0 CEventLeaderEnteredCarAsDriver::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame: `p = CEvent::operator new(0x18)` 0x4B5620 (cdecl, event pool, 0x48E2BB); if p == 0 return 0 (0x48E2D1); else `CEventLeaderEnteredCarAsDriver::Constructor` 0x48E1C0 on p with args (this+0x14) (0x48E2D9) and return its EAX (the ctor returns p).
