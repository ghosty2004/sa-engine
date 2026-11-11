# CEventNewGangMember (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x608FD0 CEventNewGangMember::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 90 (0x5A) (0x608FD0).

## 0x608FE0 CEventNewGangMember::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x608FE0).

## 0x609050 CEventNewGangMember::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x609050).

## 0x609060 CEventNewGangMember::~CEventNewGangMember  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x609080 FUN_00609080 (unnamed complete dtor: stores vtbl 0x86D0A0) (0x609063); if `flags & 1` (0x609068) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x609070); returns this (0x609078).

## 0x609310 CEventNewGangMember::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 42 (0x2A) (0x609310).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x608FF0 CEventNewGangMember::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame: `p = CEvent::operator new(0x10)` 0x4B5620 (cdecl, event pool, 0x60900B); if p == 0 return 0 (0x609021); else `CEventNewGangMember::Constructor` 0x608F70 on p with args (this+0xC) (0x609029) and return its EAX (the ctor returns p).
