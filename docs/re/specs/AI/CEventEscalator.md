# CEventEscalator (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5FF840 CEventEscalator::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 74 (0x4A) (0x5FF840).

## 0x5FF850 CEventEscalator::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x5FF850).

## 0x5FF870 CEventEscalator::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 56 (0x38) (0x5FF870).

## 0x602BE0 CEventEscalator::~CEventEscalator  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5FF860 thunk_FUN_004ac010 (0x602BE3); if `flags & 1` (0x602BE8) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x602BF0); returns this (0x602BF8). Proposed name: `thunk_FUN_004ac010::`scalar deleting destructor'` (the complete dtor it calls is thunk_FUN_004ac010).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B25E0 CEventEscalator::TakesPriorityOver  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this, const CEvent* other [esp+4])`, `ret 4`: `t1 = other->GetEventType()` (vtbl+4, called first, 0x4B25EA); `t2 = this->GetEventType()` (vtbl+4,
  0x4B25F3); returns AL = (t1 != t2) (0x4B25F9).

## 0x602B90 CEventEscalator::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CEvent* ()`, `ret`, SEH: `p = CEvent::operator new(0xC)` 0x4B5620; null -> 0; else `CEvent::CEvent(p)` 0x4ABFC0, `p->vtbl = 0x86CD00` (0x602BC8); return p.
  (Nothing is copied from `this`.)
