# CEventDeadPed (AI) - RE-SPEC-AI-2 helper A (groups J1/J6)

Addresses are 1.0 US. Every claim cites the instruction address that proves it; the disassembly is the authority.

## 0x4ADF20 CEventDeadPed::GetEventType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 11 (0xB) (0x4ADF20).

## 0x4ADF30 CEventDeadPed::GetLifeTime  (catalogue: specified/REA, port level 0)
- `__thiscall` no stack args, `ret`: returns 0 (EAX; null / 0) (0x4ADF30).

## 0x4ADF40 CEventDeadPed::GetSourceEntity  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret`: returns the dword at this+0x14 (0x4ADF40).

## 0x4ADF50 CEventDeadPed::GetLocalSoundLevel  (catalogue: specified/REA, port level 0)
- `__thiscall float ()`, `ret`: returns ST0 = 60.0f [0x858B34] (0x4ADF50).

## 0x4ADF60 CEventDeadPed::GetEventPriority  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()` (ECX unused), `ret`: returns 15 (0xF) (0x4ADF60).

## 0x4B5E00 CEventDeadPed::~CEventDeadPed  (catalogue: specified/REA, port level 2)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x4ADF70 CEventDeadPed::~CEventDeadPed (0x4B5E04); if `flags & 1` (0x4B5E09) frees the slot in the event pool `*(CPool*)[0xB744AC]` (ms_pEventPool; CEvent::operator delete inlined): `idx = (int32)(this - pool->m_pObjects) / 0x44` (signed magic division 0x78787879 >> 5, rounding toward zero, 0x4B5E16-0x4B5E2E); `pool->m_byteMap[idx] |= 0x80` (0x4B5E33); `if (idx < pool->m_nFirstFree) pool->m_nFirstFree = idx` (pool+0xC, signed `jge`, 0x4B5E36-0x4B5E3E). Returns this. Element size 0x44.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x4ADEA0 CEventDeadPed::CEventDeadPed  (catalogue: specified/REA, port level 2)
- `__thiscall CEventDeadPed* (this, CPed* ped [esp+4], bool bAvoid [esp+8], uint32 deathTimeMs [esp+0xC])`, `ret 0xC`, SEH frame.
- CEvent/CEventEditableResponse base inlined: +4 m_nTimeActive = 0 (0x4ADEC1), +8 m_bValid = 0 (0x4ADEC4), +0xC m_bAddToEventGroup = 1 (0x4ADEC7),
  +0xE int16 m_TaskId = 200 (0xC8, 0x4ADECB), +0x10 int16 = -1 (0x4ADED1); vtbl = 0x85AE00 (0x4ADEEC); +0x14 m_ped = ped; +0x18 u8 = bAvoid;
  +0x1C m_deathTimeInMs = arg3 (0x4ADEF2-0x4ADEF7); if ped != 0 `CEntity::RegisterReference(ped, &m_ped)` 0x571B70 (0x4ADEFD). Returns this.

## 0x4ADF70 CEventDeadPed::~CEventDeadPed  (catalogue: specified/REA, port level 1)
- Complete dtor, `__thiscall void ()`, SEH: vtbl = 0x85AE00 (0x4ADF8D); if m_ped (+0x14) `CEntity::CleanUpOldReference(m_ped, &m_ped)` 0x571A00 (0x4ADFA6);
  vtbl = 0x85AB38 (CEvent base, 0x4ADFAF).

## 0x4B4830 CEventDeadPed::AffectsPed  (catalogue: specified/REA, port level 2)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`. dead = this+0x14 (m_ped). Returns in AL (false paths `xor al,al` / `xor eax,eax`, true `mov eax,1`).
- Behaviour:
  1. dead == 0 -> false (0x4B4838). ped == dead -> false (0x4B4847). `!ped->IsAlive()` 0x5E0170 -> false (0x4B4856).
  2. `g_ikChainMan.IsLooking(ped)` 0x6181A0 (ECX = 0xC15448) and `g_ikChainMan.GetLookAtEntity(ped)` 0x6181D0 == dead -> false (0x4B4862-0x4B4879).
  3. dead->m_nPhysicalFlags (+0x40) bit 8 (bSubmergedInWater) -> false (0x4B4885).
  4. Positions: P = dead->m_matrix ? matrix+0x30 : dead+4; Q likewise for ped (0x4B488E-0x4B48A7). x87: dx = P.x - Q.x, dy, dz (0x4B48AA-0x4B48C0);
     `d2 = (dz*dz + dy*dy) + dx*dx` (0x4B48C3-0x4B48D1), stored as f32 into the arg slot [esp+0xC] (0x4B48D3).
  5. If dead ped flags +0x474 bit 8 (bKilledByStealth) is CLEAR (0x4B48BD/0x4B48D7): discard the 3 diffs and go to 7.
  6. Bit set: `dot = (dz*fwd.z + dy*fwd.y) + dx*fwd.x` with fwd = ped->m_matrix+0x10 (ped matrix dereferenced without a null check; only the
     position used the fallback, 0x4B48D9-0x4B48E8); `fcomp 0.0f [0x858B50]`, `test ah,5; jnp` -> false iff dot < 0 (NaN continues, 0x4B48F5).
     Then `t = 8.0f [0x85AB0C] * 8.0f [0x85AB0C]`; `fcomp d2`; `test ah,5; jp` -> false iff 64 < d2 (equal/NaN continue, 0x4B490C).
  7. `t = 20.0f [0x85AB08] * 20.0f [0x85AB08]`; `fcomp d2`; `test ah,0x41; jne` -> return 1 iff 400 > d2 strictly (NaN -> 0) (0x4B4927-0x4B4933).
- gta-reversed: EventDeadPed.cpp:21 (not hooked) returns false when the dead ped was NOT killed by stealth; the original then still returns
  `400 > d2` (step 5 -> 7). gr is wrong there; the port follows the bytes. Stealth branch matches gr. Open questions: none.

## 0x4B6E70 CEventDeadPed::CloneEditable  (catalogue: specified/REA, port level 3)
- `__thiscall CEventEditableResponse* ()`, `ret`, SEH: `p = CEventPool::New(ms_pEventPool [0xB744AC])` 0x4B5570; null -> return 0 (0x4B6EA2);
  else `CEventDeadPed::CEventDeadPed(p, this+0x14 m_ped, (u8)this+0x18 zero-extended, this+0x1C)` 0x4ADEA0 (0x4B6EB4), return p.
