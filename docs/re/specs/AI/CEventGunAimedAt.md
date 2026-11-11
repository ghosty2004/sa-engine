# CEventGunAimedAt (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4B0770 CEventGunAimedAt::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 31 (0x1F) (0x4B0770).

## 0x4B0780 CEventGunAimedAt::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4B0780).

## 0x4B0790 CEventGunAimedAt::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret`: returns the dword at this+0x14 (0x4B0790).

## 0x4B07A0 CEventGunAimedAt::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 50 (0x32) (0x4B07A0).

## 0x4B63F0 CEventGunAimedAt::~CEventGunAimedAt  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4B07B0 CEventGunAimedAt::~CEventGunAimedAt (0x4B63F4); if `flags & 1` (0x4B63F9) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B6406-0x4B641E); `pool->m_byteMap[idx] |= 0x80` (0x4B6423); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B6426-0x4B642E). Returns this. Element size 0x44.

## 0x4B7630 CEventGunAimedAt::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEvent* ()`, `ret`, SEH frame (unwind deletes into the pool on a throwing ctor): `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570 (0x4B764F); if p == 0 return 0 (0x4B7662); else `CEventGunAimedAt::CEventGunAimedAt` 0x4B0700 on p with args (this+0x14) (0x4B766A) and return its EAX (the ctor returns p).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B0700 CEventGunAimedAt::CEventGunAimedAt  (catalogue: specified/REA, port level 2)
- `__thiscall CEventGunAimedAt* (this, CPed* aimedBy [esp+4])`, `ret 4`, SEH. Entry is the obfuscated `push -1; jmp 0x401BE0` stub, which pushes the SEH
  handler 0x839FA8 (computed from [0x15606C4]) and returns to 0x4B0707 - equivalent to a normal SEH prologue, no other effect.
- Inline base: +4 = 0, +8 u8 = 0, +0xC u8 = 1, +0xE i16 = 200, +0x10 i16 = -1 (0x4B071F-0x4B072F); vtbl = 0x85B330 (0x4B0742); +0x14 m_ped = aimedBy;
  if non-null `CEntity::RegisterReference(aimedBy, &m_ped)` 0x571B70 (0x4B074D). Returns this.

## 0x4B07B0 CEventGunAimedAt::~CEventGunAimedAt  (catalogue: specified/REA, port level 1)
- Complete dtor, SEH: vtbl = 0x85B330; if +0x14 `CEntity::CleanUpOldReference(m_ped, &m_ped)` 0x571A00; vtbl = 0x85AB38. Same instruction sequence as
  0x4ADF70 (only the vtable / SEH handler differ).

## 0x4B4EE0 CEventGunAimedAt::AffectsPed  (catalogue: specified/REA, port level 15)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`; returns AL (0 via `xor al,al` 0x4B4FC2, 1 via `mov al,1` 0x4B4FBB). Steps in order, each failing test -> 0:
  1. `ped->IsPlayer()` 0x5DF8F0 true -> 0 (0x4B4EF1).
  2. `t = ped->m_pIntelligence->m_TaskMgr.GetActiveTask()` 0x681720; if t != 0, GetActiveTask is CALLED AGAIN (0x4B4F12) and its `GetTaskType()` (vtbl+0x10)
     == 0x259 (TASK_COMPLEX_REACT_TO_GUN_AIMED_AT) -> 0 (0x4B4F23).
  3. m_ped (+0x14) == 0 -> 0 (0x4B4F2E).
  4. `ped->m_pIntelligence->IsInSeeingRange(&m_ped->GetPosition())` 0x600C60 (matrix+0x30 or +4) false -> 0 (0x4B4F51).
  5. `!ped->IsAlive()` 0x5E0170 -> 0 (0x4B4F5C).
  6. `CPedGroups::AreInSameGroup(ped, m_ped)` 0x5F7F40 (cdecl) true -> 0 (0x4B4F6D).
  7. If ped bInVehicle (+0x46C bit 8) and ped->m_pVehicle (+0x58C) and `veh->IsPassenger(ped)` 0x6D1BD0 and `veh->m_pDriver (+0x460) != 0` -> 0 (0x4B4F75-0x4B4F9C).
  8. If ped->m_nPedType (+0x598) == 6 (COP): `CCrime::ReportCrime(22 /*CRIME_AIM_GUN*/, ped, CWorld::FindPlayerPed(-1) 0x56E210)` 0x532010 (cdecl, 0x4B4FA7-0x4B4FB2).
  9. Return 1. (The cop crime report is a side effect of a query; keep it.)
- gta-reversed: not hooked. Open questions: none.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4B0810 CEventGunAimedAt::TakesPriorityOver  (catalogue: specified/REA, port level 3)
- `__thiscall bool (this, const CEvent* other [esp+4])`, `ret 4`. CEvent vtable slots: +4 GetEventType, +8 GetEventPriority, +0x28 GetSourceEntity.
- If m_ped (+0x14) != 0 and `m_ped->IsPlayer()` 0x5DF8F0 (0x4B0819-0x4B0822):
  - if `other->GetSourceEntity() == m_ped` (0x4B082C-0x4B0838): return `this->GetEventPriority() >= other->GetEventPriority()` (other's priority queried FIRST,
    signed `setge`, 0x4B083A-0x4B084A);
  - else t = `other->GetEventType()`: t in {9, 0x24, 0x27} -> return 1 (0x4B0857-0x4B0874); else return `CEvent::TakesPriorityOver(this, other)` 0x4ABC30.
- Otherwise (0x4B087A): return `this->GetEventPriority() >= other->GetEventPriority()` (other first).

## 0x4B09C0 CEventGunAimedAt::IsCriminalEvent  (catalogue: specified/REA, port level 1)
- `__thiscall bool ()`, `ret`: 1 iff m_ped (+0x14) != 0 and `m_ped->IsPlayer()` 0x5DF8F0 (0x4B09C0-0x4B09D0), else 0.

## 0x4B09E0 CEventGunAimedAt::ReportCriminalEvent  (catalogue: specified/REA, port level 1)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`: if `this->IsCriminalEvent()` (vtbl+0x1C, 0x4B09E2): `CPedType::PoliceDontCareAboutCrimesAgainstPedType(ped->m_nPedType
  (+0x598))` 0x608A40 (cdecl, result discarded) (0x4B09E9-0x4B09F9). No crime is reported (the query result is unused) - original behaviour.
