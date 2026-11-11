# CEventLeaderQuitEnteringCarAsDriver (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B1040 CEventLeaderQuitEnteringCarAsDriver::TakesPriorityOver  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4B1040).

## 0x63A130 CEventLeaderQuitEnteringCarAsDriver::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 57 (0x39) (0x63A130).

## 0x63A140 CEventLeaderQuitEnteringCarAsDriver::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x63A140).

## 0x63A1A0 CEventLeaderQuitEnteringCarAsDriver::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns false (AL=0; upper EAX bits unchanged) (0x63A1A0).

## 0x63A1B0 CEventLeaderQuitEnteringCarAsDriver::~CEventLeaderQuitEnteringCarAsDriver  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x63A1D0 FUN_0063a1d0 (unnamed complete dtor: stores vtbl 0x86E6B0) (0x63A1B3); if `flags & 1` (0x63A1B8) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x63A1C0); returns this (0x63A1C8).

## 0x63A1E0 CEventLeaderQuitEnteringCarAsDriver::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 48 (0x30) (0x63A1E0).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x63A150 CEventLeaderQuitEnteringCarAsDriver::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CEvent* ()`, `ret`, SEH: `p = CEvent::operator new(0xC)` 0x4B5620; null -> 0; else `CEvent::CEvent(p)` 0x4ABFC0, `p->vtbl = 0x86E6B0` (0x63A188); return p.
