# CEventVehicleDamage (AI) - RE-SPEC-3 group I2

## 0x4B1980 CEventVehicleDamage::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 0x31 (49). Slot 2 shared by CEventVehicleDamage 0x85B588, CEventVehicleDamageWeapon 0x86D640 and
  CEventVehicleDamageCollision 0x8710D0.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B1960 CEventVehicleDamage::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B1960).

## 0x4B1970 CEventVehicleDamage::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 55.0f [0x85ADBC] (0x4B1970).

## 0x4B6700 CEventVehicleDamage::~CEventVehicleDamage  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B1990 FUN_004b1990 (unnamed complete dtor: stores vtbl 0x85B588 then 0x85AB38) (0x4B6704); if `flags & 1` (0x4B6709) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B6716-0x4B672E); `pool->m_byteMap[idx] |= 0x80` (0x4B6733); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6736-0x4B673E). Returns this. Element size 0x44.
