# CEventLeaderExitedCarAsDriver (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B0F70 CEventLeaderExitedCarAsDriver::TakesPriorityOver  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4B0F70).

## 0x4B8320 CEventLeaderExitedCarAsDriver::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 55 (0x37) (0x4B8320).

## 0x4B8330 CEventLeaderExitedCarAsDriver::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B8330).

## 0x4B8390 CEventLeaderExitedCarAsDriver::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns false (AL=0; upper EAX bits unchanged) (0x4B8390).

## 0x4B83A0 CEventLeaderExitedCarAsDriver::~CEventLeaderExitedCarAsDriver  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B83C0 FUN_004b83c0 (unnamed complete dtor: stores vtbl 0x85B898) (0x4B83A3); if `flags & 1` (0x4B83A8) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x4B83B0); returns this (0x4B83B8).

## 0x4B84D0 CEventLeaderExitedCarAsDriver::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 47 (0x2F) (0x4B84D0).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B8340 CEventLeaderExitedCarAsDriver::CloneEditable  (catalogue: specified/REA, port level 2)
- `__thiscall`, `ret`, SEH: `p = CEvent::operator new(0x14)` 0x4B5620; null -> 0; else `CEventEditableResponse::Constructor(p)` 0x4AC450, `p->vtbl = 0x85B898` (0x4B8378);
  return p. Nothing copied from `this`.
