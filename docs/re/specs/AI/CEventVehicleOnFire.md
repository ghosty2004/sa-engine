# CEventVehicleOnFire (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B1130 CEventVehicleOnFire::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 79 (0x4F) (0x4B1130).

## 0x4B1140 CEventVehicleOnFire::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B1140).

## 0x4B1150 CEventVehicleOnFire::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 60.0f [0x858B34] (0x4B1150).

## 0x4B1160 CEventVehicleOnFire::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 54 (0x36) (0x4B1160).

## 0x4B6570 CEventVehicleOnFire::~CEventVehicleOnFire  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B1170 FUN_004b1170 (unnamed complete dtor: stores vtbl 0x85B440 then 0x85AB38) (0x4B6574); if `flags & 1` (0x4B6579) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B6586-0x4B659E); `pool->m_byteMap[idx] |= 0x80` (0x4B65A3); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B65A6-0x4B65AE). Returns this. Element size 0x44.
