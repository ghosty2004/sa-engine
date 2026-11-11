# CEventScriptedAttractor (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x5FEF80 CEventScriptedAttractor::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 48 (0x30) (0x5FEF80).

## 0x5FEFF0 CEventScriptedAttractor::~CEventScriptedAttractor  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x5FEF70 FUN_005fef70 (unnamed complete dtor: stores vtbl 0x86C9F0) (0x5FEFF3); if `flags & 1` (0x5FEFF8) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x5FF000); returns this (0x5FF008).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5FEF90 CEventScriptedAttractor::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall`, `ret`, SEH: `p = CEvent::operator new(0x20)` 0x4B5620; null -> 0; else `CEventAttractor::Constructor(p, this+0x14 attractor, this+0x18 entity, 0)`
  0x4AF350 (third arg = the zeroed EAX, 0x5FEFC7), `p->vtbl = 0x86C9F0` (0x5FEFD4); return p.
