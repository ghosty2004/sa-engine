# CTaskSimpleChoking (AI) - RE-SPEC-FINAL (helper B)

## 0x620480 CTaskSimpleChoking::sub_620480 (proposed: CTaskSimpleChoking::FinishAnimCB)  (catalogue: unknown/REA, port level 0)
- `static void __cdecl (CAnimBlendAssociation* assoc, void* data [esp+8])`, plain `ret`. Referenced as a callback pointer by CTaskSimpleChoking::ProcessPed 0x620490 (2 sites) and UpdateChoke 0x620660.
- Behaviour: `data->m_pAnim (+0xC) = 0; data->m_bIsFinished (+0x19) = 1` (0x620484/0x62048B). Layout per gr TaskSimpleChoking.h (+8 m_pAttacker, +0xC m_pAnim, +0x10 m_nTimeRemaining, +0x14 m_nTimeStarted, +0x18 m_bIsTeargas, +0x19 m_bIsFinished).

## 0x620490 CTaskSimpleChoking::ProcessPed  (catalogue: specified/REA, port level 9)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`. Fields (gr TaskSimpleChoking.h): +8 m_pAttacker, +0xC m_pAnim, +0x10 m_nTimeRemaining (u32), +0x14 m_nTimeStarted, +0x18 m_bIsTeargas, +0x19 m_bIsFinished.
- Behaviour:
  1. `if m_bIsFinished` -> return 1.
  2. Face the attacker: `if m_pAttacker && !CPed::IsPlayer(ped) && !m_bIsTeargas`: `F = ped->m_matrix` (forward at +0x10/+0x14/+0x18, dereferenced without NULL check), `P = ped position`, `A = attacker position` (matrix pos or placement); `d = A - P` (dx, dy, dz); `dot = (dz*F.z + dy*F.y) + dx*F.x`; `fcomp [0x858B50]=0.0f; test ah, 0x41; jne skip`: only when `dot > 0` (attacker in front; equal/NaN skip): `ped->+0x55C (m_fAimingRotation) = fpatan(st1 = -dx, st0 = dy) = atan2(-dx, dy)` (fxch/fchs/fxch/fpatan, f32 store, 0x62050C-0x620514).
  3. `dt = _ftol2([0xB7CB5C CTimer::ms_fTimeStep] * [0x858B38]=0.02f * [0x858C4C]=1000.0f)` 0x821B40 (truncation); `m_nTimeRemaining = (m_nTimeRemaining > dt) ? m_nTimeRemaining - dt : 0` (unsigned `jbe`).
  4. `if m_pAnim == NULL`: `a = CAnimManager::BlendAnimation(ped->m_pRwClump, 0, 0x15, 4.0f)` 0x4D4610; `m_pAnim = a`; `a->SetFinishCallback(0x620480 (FinishAnimCB), this)`; `a->m_Speed (+0x24) = (float)_rand() * [0x858C7C]=3.0518509e-05f * [0x858C84]=0.25f + [0x858F34]=0.75f` (fild of the int, f32 store).
     Else if `m_nTimeRemaining == 0`: if `m_pAnim->m_AnimId (+0x2C, u16) == 0x15`: `m_pAnim->SetFinishCallback(0x4D1390 (empty), NULL)`; `a = BlendAnimation(clump, 0, 0xA, 4.0f)`; `m_pAnim = a`; `a->SetFinishCallback(0x620480, this)`; `e = now ([0xB7CB84]) - m_nTimeStarted`; `m_nTimeRemaining = e`; `r = CGeneral::GetRandomNumberInRange(8000, 12000)` 0x407180 (original int version; gr's differs - cat14 finding 1); if `e < r` (unsigned) keep `m_nTimeRemaining = e`; else `m_nTimeRemaining = GetRandomNumberInRange(8000, 12000)` (a SECOND, independent draw). Else (another anim playing): `m_pAnim->m_BlendDelta (+0x1C) = -4.0f; m_pAnim->m_Flags |= 4`.
  5. Always (all paths that reach here, including "time remaining > 0"): `CPed::Say(ped, 0x154, 0, 1.0f, 0, 0, 0)` 0x5EFFE0; return 0.
