# CEventLeaderEntryExit (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x43E200 CEventLeaderEntryExit::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 77 (0x4D) (0x43E200).

## 0x43E210 CEventLeaderEntryExit::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x43E210).

## 0x43E280 CEventLeaderEntryExit::CanBeInterruptedBySameEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool (0 stack args)`, `ret`: returns true (AL=1; upper EAX bits unchanged) (0x43E280).

## 0x43E290 CEventLeaderEntryExit::~CEventLeaderEntryExit  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x43E2B0 FUN_0043e2b0 (unnamed complete dtor: stores vtbl 0x859540) (0x43E293); if `flags & 1` (0x43E298) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x43E2A0); returns this (0x43E2A8).

## 0x4B2390 CEventLeaderEntryExit::TakesPriorityOver  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4B2390).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x43E220 CEventLeaderEntryExit::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH: `p = CEvent::operator new(0x10)` 0x4B5620; null -> 0; else `CEventAreaCodes::Constructor(p, this+0xC m_ped)` 0x4B2190 (base ctor),
  then `p->vtbl = 0x859540` (0x43E25F); return p.

## 0x4B23A0 CEventLeaderEntryExit::AffectsPedGroup  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this, CPedGroup* group [esp+4])`, `ret 4`: returns EAX = 1 iff this+0xC (m_ped) != 0 and `(group+8).GetLeader()` 0x5F69A0 == this+0xC (re-read),
  else 0 (0x4B23A3-0x4B23C4).
