# CEventGunShotWhizzedBy (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B2030 CEventGunShotWhizzedBy::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 49 (0x31) (0x4B2030).

## 0x4B2040 CEventGunShotWhizzedBy::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B2040).

## 0x4B2050 CEventGunShotWhizzedBy::CanBeInterruptedBySameEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool (0 stack args)`, `ret`: returns true (AL=1; upper EAX bits unchanged) (0x4B2050).

## 0x4B2060 CEventGunShotWhizzedBy::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 36 (0x24) (0x4B2060).

## 0x4B6990 CEventGunShotWhizzedBy::~CEventGunShotWhizzedBy  (catalogue: specified/REA, port level 3)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B2070 FUN_004b2070 (unnamed complete dtor: stores vtbl 0x85B730) (0x4B6994); if `flags & 1` (0x4B6999) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B69A6-0x4B69BE); `pool->m_byteMap[idx] |= 0x80` (0x4B69C3); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B69C6-0x4B69CE). Returns this. Element size 0x44.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B7AC0 CEventGunShotWhizzedBy::CloneEditable  (catalogue: specified/REA, port level 4)
- `__thiscall`, `ret`, SEH: `p = CEventPool::New([0xB744AC])` 0x4B5570; null -> 0 (0x4B7AF2); else `CEventGunShotWhizzedBy::Constructor(p, this+0x14 m_firedBy,
  &this+0x18 startPoint, &this+0x24 endPoint, (u8)this+0x30 bHasNoSound zero-extended)` 0x4B1FD0 (0x4B7B08); return p.
