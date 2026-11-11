# CEventHitByWaterCannon (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B12E0 CEventHitByWaterCannon::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 64 (0x40) (0x4B12E0).

## 0x4B12F0 CEventHitByWaterCannon::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B12F0).

## 0x4B1300 CEventHitByWaterCannon::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 55.0f [0x85ADBC] (0x4B1300).

## 0x4B1310 CEventHitByWaterCannon::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 64 (0x40) (0x4B1310).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B65C0 CEventHitByWaterCannon::~CEventHitByWaterCannon  (catalogue: specified/REA, port level 1)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls 0x4B1320 (unnamed complete dtor: `vtbl = 0x85AB38` only) with ECX = this;
  ECX is not saved and is reused as `this` (0x4B65D6). If flags & 1: event-pool free inlined (pool [0xB744AC], element 0x44, magic 0x78787879 >> 5, `m_byteMap[idx] |= 0x80`,
  `if (idx < m_nFirstFree) m_nFirstFree = idx`, 0x4B65CD-0x4B6600). Returns this.

## 0x4B77B0 CEventHitByWaterCannon::Clone  (catalogue: specified/REA, port level 1)
- `__thiscall CEvent* ()`, `ret`, no SEH: `p = CEventPool::New([0xB744AC])` 0x4B5570; null -> 0 (0x4B77C2). p+4 = 0, p+8 u8 = 0, vtbl = 0x85B488 (0x4B77CD),
  p+0xC..0x14 = this+0xC..0x14 (CVector m_point), p+0x18..0x20 = this+0x18..0x20 (CVector m_moveSpeed) (dword copies, 0x4B77D4-0x4B77FB). Returns p.
