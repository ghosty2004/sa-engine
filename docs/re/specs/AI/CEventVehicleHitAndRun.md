# CEventVehicleHitAndRun (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4AEA00 CEventVehicleHitAndRun::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 14 (0xE) (0x4AEA00).

## 0x4AEA10 CEventVehicleHitAndRun::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4AEA10).

## 0x4AEA30 CEventVehicleHitAndRun::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 100.0f [0x858628] (0x4AEA30).

## 0x4AEA40 CEventVehicleHitAndRun::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 14 (0xE) (0x4AEA40).

## 0x4B5FE0 CEventVehicleHitAndRun::~CEventVehicleHitAndRun  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4AEA50 FUN_004aea50 (unnamed complete dtor: stores vtbl 0x85AFA8 then 0x85AB38) (0x4B5FE4); if `flags & 1` (0x4B5FE9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B5FF6-0x4B600E); `pool->m_byteMap[idx] |= 0x80` (0x4B6013); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6016-0x4B601E). Returns this. Element size 0x44.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4AEA20 CEventVehicleHitAndRun::AffectsPed  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this unused, CPed* ped [esp+4])`, `ret 4`: returns `ped->IsAlive()` 0x5E0170 (EAX passed through, 0x4AEA24).

## 0x4B27B0 CEventVehicleHitAndRun::IsCriminalEvent  (catalogue: specified/REA, port level 1)
- `__thiscall bool ()`, `ret`: `d = (this+0x10 vehicle)->m_pDriver (+0x460)` (vehicle pointer dereferenced without null check, 0x4B27B3); AL = 1 iff d != 0 and
  `d->IsPlayer()` 0x5DF8F0, else 0.
