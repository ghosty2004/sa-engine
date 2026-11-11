# CEventPedToFlee (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4AF2A0 CEventPedToFlee::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 26 (0x1A) (0x4AF2A0).

## 0x4AF2B0 CEventPedToFlee::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AF2B0).

## 0x4AF2C0 CEventPedToFlee::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 2 (0x2) (0x4AF2C0).

## 0x4B6210 CEventPedToFlee::~CEventPedToFlee  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4AF2D0 CEventPedToFlee::~CEventPedToFlee (0x4B6214); if `flags & 1` (0x4B6219) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B6226-0x4B623E); `pool->m_byteMap[idx] |= 0x80` (0x4B6243); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6246-0x4B624E). Returns this. Element size 0x44.

## 0x4B73D0 CEventPedToFlee::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B73EF); if p == 0 return 0 (0x4B7402); else `CEventPedToFlee::CEventPedToFlee` 0x4AF240 on p with args (this+0xC) (0x4B740A) and return its EAX (the ctor returns p).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4AF240 CEventPedToFlee::CEventPedToFlee  (catalogue: specified/REA, port level 2)
- Same as 0x4AF130 with vtbl = 0x85B178 (0x4AF272); +0xC m_ped registered via 0x571B70 (0x4AF27D). `ret 4`.

## 0x4AF2D0 CEventPedToFlee::~CEventPedToFlee  (catalogue: specified/REA, port level 1)
- Complete dtor; entry is the obfuscated `push -1; jmp 0x402297` stub (pushes the SEH handler and returns to 0x4AF2D7; no other effect). vtbl = 0x85B178; if +0xC
  `CleanUpOldReference` 0x571A00 (0x4AF306); vtbl = 0x85AB38.

## 0x4AF330 CEventPedToFlee::AffectsPed  (catalogue: specified/REA, port level 1)
- Identical to 0x4AF220: AL = `ped->IsAlive()` != 0, `ret 4`.
