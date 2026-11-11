# CEventVehicleToSteal (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4AF6D0 CEventVehicleToSteal::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 28 (0x1C) (0x4AF6D0).

## 0x4AF6E0 CEventVehicleToSteal::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AF6E0).

## 0x4AF6F0 CEventVehicleToSteal::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 7 (0x7) (0x4AF6F0).

## 0x4B62B0 CEventVehicleToSteal::~CEventVehicleToSteal  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4AF700 FUN_004af700 (unnamed complete dtor: stores vtbl 0x85B210 then 0x85AB38) (0x4B62B4); if `flags & 1` (0x4B62B9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B62C6-0x4B62DE); `pool->m_byteMap[idx] |= 0x80` (0x4B62E3); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B62E6-0x4B62EE). Returns this. Element size 0x44.

## 0x4B74B0 CEventVehicleToSteal::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B74CF); if p == 0 return 0 (0x4B74E2); else `CEventVehicleToSteal::Constructor` 0x4AF670 on p with args (this+0xC) (0x4B74EA) and return its EAX (the ctor returns p).
