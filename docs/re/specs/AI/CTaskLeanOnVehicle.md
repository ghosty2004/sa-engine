# CTaskLeanOnVehicle (AI) - RE-SPEC-AI-2 group J3

## 0x661020 CTaskLeanOnVehicle::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4C1` (0x661020); `ret` -> returns 1217 (TASK_COMPLEX_LEAN_ON_VEHICLE). Vtable: slot 4 (+0x10) of vtable 0x86FAF4 (CTaskLeanOnVehicle).

## 0x6648A0 CTaskLeanOnVehicle::~CTaskLeanOnVehicle  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskLeanOnVehicle::Destructor 0x661030 (0x6648A3); if `flags & 1` (0x6648A8) calls CTask::operator delete 0x61A5B0 (this) (0x6648B0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FAF4 (CTaskLeanOnVehicle).

## 0x6610A0 CTaskLeanOnVehicle::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x2C) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskLeanOnVehicle::CTaskLeanOnVehicle(this+8, this+0xC, (u8)this+0x10) 0x660F90; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x6648C0 CTaskLeanOnVehicle::ProcessPed  (catalogue: specified/REA, port level 9)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`. Fields: +8 vehicle, +0xC lean time (int32, < 0 = no timer), +0x10 (u8) skip-intro, +0x14 m_pAnim, +0x18 last anim id (int32), +0x1C/+0x20/+0x24 CTaskTimer {start, interval, started}, +0x28 (u8), +0x29 (u8) stop requested, +0x2A (u8) finished. Anim group 0x34; ids 0x12F (in), 0x130 (loop), 0x131 (out). "Start A" below = `m_pAnim = CAnimManager::BlendAnimation(ped->m_pRwClump, 0x34, A, blend)` 0x4D4610 then `m_pAnim->SetFinishCallback(CTaskLeanOnVehicle::FinishAnimCB 0x661160, this)` 0x4CEBE0, then return 0.
- Behaviour:
  1. If `+8 == NULL` -> `+0x2A = 1`. `ped->+0x128 = +8` (vehicle as the ped's ignored-collision entity; stored even when NULL). `cur = m_pAnim ? (int32)(int16)m_pAnim->m_AnimId : -1`.
  2. If `+0x2A` -> `ped->+0x128 = 0`; return 1.
  3. `CPed::SetMoveState(ped, 1)` 0x5DEC00. If `+0x28 != 0 && RpAnimBlendClumpGetAssociation(ped->m_pRwClump, 0x131) == NULL` 0x4D68B0 -> `ped->+0x128 = 0`; return 1.
  4. If `+0x29` (stop): `cur == 0x12F` -> `m_pAnim->m_BlendDelta = -8.0f` (continue to 5); `cur == 0x130` -> `m_pAnim->SetDeleteCallback(0x4D1390, NULL)`; Start 0x131 with blend 1000.0f; `cur == 0x131` -> `m_pAnim->m_Speed (+0x24) = 3.0f` (continue).
  5. If `m_pAnim == NULL`: if `+0x18 == -1`: if `+0x10 == 0` -> Start 0x12F with blend 4.0f; else go to (a). If `+0x18 == 0x12F` -> (a): if `(int32)+0xC >= 0` -> timer `start = now ([0xB7CB84]); interval = +0xC; started = 1`; Start 0x130 with blend 1000.0f. Any other +0x18 -> return 0.
     If `m_pAnim != NULL`: if `timer.started && CTaskTimer::IsOutOfTime(this+0x1C)` 0x420E30 `&& m_pAnim->m_AnimId != 0x131` -> `m_pAnim->SetDeleteCallback(0x4D1390, NULL)`; Start 0x131 with blend 1000.0f.
  6. Return 0.
