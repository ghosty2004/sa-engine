# CEventPlayerCommandToGroupGather (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x609270 CEventPlayerCommandToGroupGather::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 76 (0x4C) (0x609270).

## 0x6092E0 CEventPlayerCommandToGroupGather::~CEventPlayerCommandToGroupGather  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x609300 FUN_00609300 (unnamed complete dtor: stores vtbl 0x86D120) (0x6092E3); if `flags & 1` (0x6092E8) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x6092F0); returns this (0x6092F8).

## 0x609330 CEventPlayerCommandToGroupGather::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 45 (0x2D) (0x609330).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x609280 CEventPlayerCommandToGroupGather::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall`, `ret`, SEH: `p = CEvent::operator new(0x1C)` 0x4B5620; null -> 0; else `CEventPlayerCommandToGroup::Constructor(p, 2, this+0x18)` 0x4B23D0 (0x6092B4-0x6092BC),
  `p->vtbl = 0x86D120` (0x6092C1); return p.
