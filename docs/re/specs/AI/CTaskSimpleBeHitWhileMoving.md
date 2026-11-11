# CTaskSimpleBeHitWhileMoving (AI) - RE-SPEC-FINAL (helper B)

## 0x61FE30 CTaskSimpleBeHitWhileMoving::CTaskSimpleBeHitWhileMoving  (catalogue: unknown/REA, port level 1)
- `__thiscall CTaskSimpleBeHitWhileMoving* (this, int32 a1 [esp+4], int32 a2 [esp+8])`, `ret 8`. No callers (inlined in Clone 0x623190).
- Behaviour: CTaskSimple ctor 0x61A390; `+8 = 0 (u8, finished); +0x14 = 0 (m_pAnim); vtbl = 0x86D7FC` (TASK_SIMPLE_BE_HIT_WHILE_MOVING); `+0xC = a1; +0x10 = a2`; return this.

## 0x61FE70 CTaskSimpleBeHitWhileMoving::~CTaskSimpleBeHitWhileMoving  (catalogue: unknown/REA, port level 1)
- Complete dtor, `__thiscall void (this)`, plain `ret`; SEH frame (.HOODLUM stub). Caller: scalar deleting dtor 0x625BA0.
- Behaviour: `vtbl = 0x86D7FC`; `if m_pAnim (+0x14)`: `m_pAnim->m_BlendDelta (+0x1C) = -4.0f` (0x61FEA4) then `m_pAnim->SetFinishCallback(0x4D1390 (empty default CB), NULL)` 0x4CEBE0 (0x61FEB3); then CTaskSimple::~CTaskSimple 0x61A3A0.

## 0x61FF30 CTaskSimpleBeHitWhileMoving::FinishAnimCB  (catalogue: unknown/REA, port level 0)
- `static void __cdecl (CAnimBlendAssociation* assoc, void* data [esp+8])`, plain `ret`: `data->+0x14 (m_pAnim) = 0; data->+8 (finished) = 1` (0x61FF34/0x61FF3B). No direct references besides the StartAnim helper 0x61FF40.

## 0x61FF40 CTaskSimpleBeHitWhileMoving::ProcessPed-helper (proposed: CTaskSimpleBeHitWhileMoving::StartAnim)  (catalogue: unknown/REA, port level 9)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. Caller: vt_86d7fc::ProcessPed 0x620290 (only when `m_pAnim (+0x14) == NULL`). Fields: +0xC hit kind (2/4 special), +0x10 direction (0..3), +0x14 m_pAnim. "Get(id)" = `RpAnimBlendClumpGetAssociation(ped->m_pRwClump, id)` 0x4D68B0; "Blend(id)" = `CAnimManager::BlendAnimation(ped->m_pRwClump, 0, id, 8.0f)` 0x4D4610; "Restart" = `m_pAnim->SetCurrentTime(0.0f)` 0x4CEA80, `m_pAnim->m_Flags |= 1`, `m_pAnim->m_Flags |= 8`.
- Every path ends with `m_pAnim->SetFinishCallback(0x61FF30 (FinishAnimCB), this)` 0x4CEBE0 - the callback address is produced by a .HOODLUM stub (0x401122: `[0x15605F8] ror (byte [0x698EFA] & 31)` = 0x61FF30). SetFinishCallback has no NULL check: paths that leave m_pAnim NULL write to address 0x30 (original latent bug, marked "NULL!" below).
- Behaviour:
  1. Gate (0x61FF44-0x61FF76): if `+0xC == 4 || +0xC == 2`: player -> continue to 2 only if `(_rand() & 7) == 0`; non-player -> continue if `_rand() & 1`, else continue if `(_rand() & 7) == 0`. Otherwise go to 4.
  2. Direction anim: `+0x10 == 1` -> id 0x19 (step 3); `== 2` -> `_rand() & 1` ? anim 0x22 (step P) : id 0x1A (step 3); `== 3` -> id 0x1B (step 3); any other -> if `+0xC == 2` id 0x14 (step 3); else `_rand() & 1` ? anim 0x25 (P) : (`_rand() & 1` ? anim 0x20 (P) : id 0x13 (step 3)).
     Step P (anim a): `m_pAnim = Get(a)`; if non-NULL and `!(m_pAnim->m_BlendDelta < 0)` (fcomp 0.0, `test ah, 5; jp`) -> Restart; else `m_pAnim = Blend(a)` then Restart. End.
  3. Step with id: if `ped->+0xFC (m_pAttachedTo) != 0` -> End (m_pAnim NULL!). `CPed::ClearLookFlag(ped)` 0x5E1950; `CPed::ClearAimFlag(ped)` 0x5DEF20. If `id != 0xBF`: `e = Get(id)`; if e: `e->SetCurrentTime(0)`, `e->m_Flags |= 1`, `e->m_BlendAmount (+0x18) = 0`, `e->m_BlendDelta = 8.0f` (m_pAnim NOT set -> NULL! at End); else `m_pAnim = Blend(id)`, e = NULL. If `id == 0xBF`: `e = (IsPlayer(ped) ? (Get(0x6E) ?: Get(0x6F)) : NULL)`.
     Then `ped->+0x744` (u32 time stamp) is set: player: if `e` and `e->m_AnimId` (u16) is 0x6E or 0x6F -> `_ftol2(((float)(u32)now + T) - e->m_CurrentTime*1000.0f + 100.0f)`; if `e` other anim -> `_ftol2(T + (float)(u32)now + 500.0f)`; if `e == NULL` -> `now + 1000 + r`. Non-player: if `e` -> `_ftol2(((float)r + T) + (float)(u32)now + 500.0f)`; else `now + 1000 + r`. Here `T = e->m_BlendHier->m_fTotalLength (+0x10) * [0x858C4C]=1000.0f`, `r = ((u16)ped->+0x20 + [0xB7CB4C CTimer::m_FrameCounter]) % 1000` (unsigned div), and u32->float conversions use fild plus `[0x858C54]=4294967296.0f` when the sign bit is set. End.
  4. Fallback (0x6201CE): if `!IsPlayer(ped) && !(_rand() & 1)` -> `CGeneral::GetRandomNumberInRange(1000, 3000)` 0x407180 (result discarded) -> End (m_pAnim NULL!). Else `a = {1: 0x1D, 2: 0x1E, 3: 0x1F}[+0x10]` (other values 0x1C); `m_pAnim = Get(a)`; if non-NULL and `!(BlendDelta < 0)` -> Restart; else `m_pAnim = Blend(a)` and Restart. End.
