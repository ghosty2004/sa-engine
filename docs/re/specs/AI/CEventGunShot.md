# CEventGunShot (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4AC6B0 CEventGunShot::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 15 (0xF) (0x4AC6B0).

## 0x4AC6C0 CEventGunShot::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AC6C0).

## 0x4AC6D0 CEventGunShot::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret`: returns the dword at this+0x14 (0x4AC6D0).

## 0x4AC700 CEventGunShot::CanBeInterruptedBySameEvent  (catalogue: specified/REA, port level 0)
- `__thiscall bool (0 stack args)`, `ret`: returns true (AL=1; upper EAX bits unchanged) (0x4AC700).

## 0x4AC710 CEventGunShot::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 35 (0x23) (0x4AC710).

## 0x4AC830 CEventGunShot::ReportCriminalEvent  (catalogue: specified/REA, port level 0)
- `__thiscall void (1 stack arg)`: empty body (`ret 4` at 0x4AC830).

## 0x4B58E0 CEventGunShot::~CEventGunShot  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4AC720 FUN_004ac720 (unnamed complete dtor: stores vtbl 0x85ABE0 then 0x85AB38) (0x4B58E4); if `flags & 1` (0x4B58E9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B58F6-0x4B590E); `pool->m_byteMap[idx] |= 0x80` (0x4B5913); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B5916-0x4B591E). Returns this. Element size 0x44.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4AC6E0 CEventGunShot::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: ST0 = this+0x30 (u8 m_bHasNoSound) != 0 ? 0.0f [0x858B50] : 160.0f [0x858970] (0x4AC6E0-0x4AC6EE).
