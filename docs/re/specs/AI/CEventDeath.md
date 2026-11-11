# CEventDeath (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4ADE20 CEventDeath::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 10 (0xA) (0x4ADE20).

## 0x4ADE30 CEventDeath::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4ADE30).

## 0x4ADE40 CEventDeath::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 73 (0x49) (0x4ADE40).

## 0x4ADE80 CEventDeath::AffectsPed  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4ADE80).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4ADDF0 CEventDeath::CEventDeath  (catalogue: specified/REA, port level 0)
- `__thiscall CEventDeath* (this, bool bDrowning [esp+4])`, `ret 4`: +4 = 0, +8 u8 = 0 (0x4ADDF4-0x4ADDF7), vtbl = 0x85ADC0 (0x4ADDFE), +0xC m_bDrowning = arg (u8),
  +0x10 m_deathTimeInMs = CTimer::m_snTimeInMilliseconds [0xB7CB84] (0x4ADE07). Returns this (EAX = ECX on entry).

## 0x4B5DB0 CEventDeath::~CEventDeath  (catalogue: specified/REA, port level 1)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls 0x4ADE90 (unnamed complete dtor: only `[ecx] = 0x85AB38`)
  (0x4B5DB1); ECX is NOT saved and is used afterwards as `this` (relies on 0x4ADE90 leaving ECX intact; a Rust port just keeps `this`); if flags & 1 (0x4B5DB6): event-pool free
  inlined: `idx = (int32)(this - pool->m_pObjects) / 0x44` (magic 0x78787879 >> 5, 0x4B5DCA-0x4B5DD9), `m_byteMap[idx] |= 0x80` (0x4B5DE3),
  `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (0x4B5DE8-0x4B5DF0), pool = [0xB744AC]. Returns this.

## 0x4B6E30 CEventDeath::Clone  (catalogue: specified/REA, port level 1)
- `__thiscall CEvent* ()`, `ret`, no SEH: `p = CEventPool::New([0xB744AC])` 0x4B5570; null -> 0 (0x4B6E40). Inline ctor: p+4 = 0, p+8 = 0, vtbl 0x85ADC0,
  p+0xC = this+0xC (u8), p+0x10 = this+0x10 (death time copied, not re-read from the timer) (0x4B6E42-0x4B6E5C). Returns p.
