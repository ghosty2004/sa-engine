# CEventVehicleCollision (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4AC8F0 CEventVehicleCollision::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 1 (0x1) (0x4AC8F0).

## 0x4AC900 CEventVehicleCollision::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AC900).

## 0x4AC910 CEventVehicleCollision::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 58 (0x3A) (0x4AC910).

## 0x4AC980 CEventVehicleCollision::TakesPriorityOver  (catalogue: specified/REA, port level 0)
- `__thiscall bool (1 stack arg)`, `ret 4`: returns true (AL=1; upper EAX bits unchanged) (0x4AC980).

## 0x4B5930 CEventVehicleCollision::~CEventVehicleCollision  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4AC920 FUN_004ac920 (unnamed complete dtor: stores vtbl 0x85AC28 then 0x85AB38) (0x4B5934); if `flags & 1` (0x4B5939) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B5946-0x4B595E); `pool->m_byteMap[idx] |= 0x80` (0x4B5963); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B5966-0x4B596E). Returns this. Element size 0x44.
