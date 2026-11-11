# CEventCopCarBeingStolen (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B17C0 CEventCopCarBeingStolen::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 16 (0x10) (0x4B17C0).

## 0x4B17D0 CEventCopCarBeingStolen::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B17D0).

## 0x4B17E0 CEventCopCarBeingStolen::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 38 (0x26) (0x4B17E0).

## 0x4B66B0 CEventCopCarBeingStolen::~CEventCopCarBeingStolen  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B17F0 FUN_004b17f0 (unnamed complete dtor: stores vtbl 0x85B548 then 0x85AB38) (0x4B66B4); if `flags & 1` (0x4B66B9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B66C6-0x4B66DE); `pool->m_byteMap[idx] |= 0x80` (0x4B66E3); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B66E6-0x4B66EE). Returns this. Element size 0x44.

## 0x4B78B0 CEventCopCarBeingStolen::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B78CF); if p == 0 return 0 (0x4B78E2); else `CEventCopCarBeingStolen::Constructor` 0x4B1740 on p with args (this+0xC, this+0x10) (0x4B78EE) and return its EAX (the ctor returns p).
