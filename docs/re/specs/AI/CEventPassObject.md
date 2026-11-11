# CEventPassObject (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x65DD50 CEventPassObject::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 89 (0x59) (0x65DD50).

## 0x65DD60 CEventPassObject::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x65DD60).

## 0x65DDE0 CEventPassObject::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x65DDE0).

## 0x65DDF0 CEventPassObject::~CEventPassObject  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x65DCF0 FUN_0065dcf0 (unnamed complete dtor: stores vtbl 0x86F8B8) (0x65DDF3); if `flags & 1` (0x65DDF8) calls 0x4B5630 CEvent::operator delete(this) (cdecl, 0x65DE00); returns this (0x65DE08).

## 0x65DE20 CEventPassObject::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 11 (0xB) (0x65DE20).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x65DD70 CEventPassObject::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame: `p = CEvent::operator new(0x14)` 0x4B5620 (cdecl, event pool, 0x65DD8B); if p == 0 return 0 (0x65DDA1); else `CEventPassObject::Constructor` 0x65DC70 on p with args (this+0xC, (u8)this+0x10 (zero-extended) (byte, mov)) (0x65DDAF) and return its EAX (the ctor returns p).
