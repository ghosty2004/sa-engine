# CEventSpecial (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B1B10 CEventSpecial::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 42 (0x2A) (0x4B1B10).

## 0x4B1B20 CEventSpecial::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B1B20).

## 0x4B1B30 CEventSpecial::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4B1B30).

## 0x4B1B40 CEventSpecial::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 52 (0x34) (0x4B1B40).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B6750 CEventSpecial::~CEventSpecial  (catalogue: specified/REA, port level 1)
- Scalar deleting destructor, `ret 4`: calls 0x4B1B50 (unnamed complete dtor: `vtbl = 0x85AB38` only); ECX reused as `this`; if flags & 1 event-pool free inlined (pool
  [0xB744AC], element 0x44, 0x4B675D-0x4B6790). Returns this.

## 0x4B7920 CEventSpecial::CloneEditable  (catalogue: specified/REA, port level 1)
- `__thiscall`, `ret`, no SEH: `p = CEventPool::New([0xB744AC])` 0x4B5570; null -> 0; p+4 = 0, p+8 = 0, p+0xC = 1, p+0xE i16 = 200, p+0x10 i16 = -1, vtbl = 0x85B5D0
  (0x4B7931-0x4B7947); return p.
