# CEventVehicleDied (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B0D70 CEventVehicleDied::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 35 (0x23) (0x4B0D70).

## 0x4B0D80 CEventVehicleDied::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B0D80).

## 0x4B0D90 CEventVehicleDied::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 72 (0x48) (0x4B0D90).

## 0x4B6520 CEventVehicleDied::~CEventVehicleDied  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B0DA0 FUN_004b0da0 (unnamed complete dtor: stores vtbl 0x85B400 then 0x85AB38) (0x4B6524); if `flags & 1` (0x4B6529) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B6536-0x4B654E); `pool->m_byteMap[idx] |= 0x80` (0x4B6553); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6556-0x4B655E). Returns this. Element size 0x44.
