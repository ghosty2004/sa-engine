# CEventGotKnockedOverByCar (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B1BD0 CEventGotKnockedOverByCar::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 43 (0x2B) (0x4B1BD0).

## 0x4B1BE0 CEventGotKnockedOverByCar::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B1BE0).

## 0x4B1BF0 CEventGotKnockedOverByCar::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 55.0f [0x85ADBC] (0x4B1BF0).

## 0x4B1C00 CEventGotKnockedOverByCar::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 37 (0x25) (0x4B1C00).

## 0x4B67A0 CEventGotKnockedOverByCar::~CEventGotKnockedOverByCar  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B1C10 FUN_004b1c10 (unnamed complete dtor: stores vtbl 0x85B618 then 0x85AB38) (0x4B67A4); if `flags & 1` (0x4B67A9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B67B6-0x4B67CE); `pool->m_byteMap[idx] |= 0x80` (0x4B67D3); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B67D6-0x4B67DE). Returns this. Element size 0x44.

## 0x4B7960 CEventGotKnockedOverByCar::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B797F); if p == 0 return 0 (0x4B7992); else `CEventGotKnockedOverByCar::CEventGotKnockedOverByCar` 0x4B1B60 on p with args (this+0x14) (0x4B799A) and return its EAX (the ctor returns p).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B1B60 CEventGotKnockedOverByCar::CEventGotKnockedOverByCar  (catalogue: specified/REA, port level 2)
- `__thiscall (this, CVehicle* vehicle [esp+4])`, `ret 4`, SEH: base inline (+4 = 0, +8 = 0, +0xC = 1, +0xE = 200, +0x10 = -1), vtbl = 0x85B618 (0x4B1BA2),
  +0x14 m_Vehicle = vehicle; if non-null `RegisterReference(vehicle, &m_Vehicle)` 0x571B70 (0x4B1BAD). Returns this.

## 0x4B1C70 CEventGotKnockedOverByCar::AffectsPed  (catalogue: specified/REA, port level 1)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`: if m_Vehicle (+0x14) == 0 -> 0; if `ped->IsPlayer()` 0x5DF8F0 -> 0; else return `ped->IsAlive()` 0x5E0170 (AL).

## 0x4B1CA0 CEventGotKnockedOverByCar::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall CEntity* ()`, `ret`: returns `m_Vehicle ? m_Vehicle->m_pDriver (+0x460) : 0` (0x4B1CA0-0x4B1CAE).
