# CTaskComplexWalkRoundBuildingAttempt (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

Source: `Tasks/TaskTypes/TaskComplexWalkRoundBuildingAttempt.cpp` (pin 20d8d2a). Float RNG in the original is inlined as
`(float)rand() * 3.0518509e-05f [0x858C7C]` (rand NOT masked, 1/32767) followed by span/offset arithmetic; gr's float helper is `max*t + min*(1-t)`.
Fields used: +0x2C m_normal (CVector), +0x49 flags (bit 0x02 m_routeHasPoints, bit 0x10 m_justAbort), +0x4A m_nAttempts (i8).
CPed +0x478 bit 0x40 = bIgnoreHeightCheckOnGotoPointTask.

## 0x654C80 CTaskComplexWalkRoundBuildingAttempt::CreateSubTask  (gr_deviations = 1, plus 1 assert row)
- cpp:87 `assert(approxEqual(...))`: not in the original (no such computation in the ACHIEVE_HEADING case). REFUTED as behaviour (debug only). harmless.
- cpp:127 `default: NOTSA_UNREACHABLE();`: Original (`__thiscall CTask* (this, int type [esp+4], CPed* ped [esp+8])`, `ret 8`): `eax = 0` (0x654C9F); type > 0x384 ->
  0x654E09 checks; type == 0x384 -> 0x654DC2; otherwise `type - 0xCB` > 0xDB (unsigned `ja`) -> 0x654E20 returns 0; the byte table 0x654F3C / jump table
  0x654F28 route the rest, unknown entries reach 0x654E1E `xor eax,eax`. Verdict: CONFIRMED (original returns nullptr). Class: keep-original.

## 0x656E70 CTaskComplexWalkRoundBuildingAttempt::CreateNextSubTask  (gr_deviations = 1)
- cpp:257 `default: NOTSA_UNREACHABLE();`: Original (`ret 4`): `edi = 0` (0x656E77); default (0x656F35-0x656F44): `ped->+0x478 &= ~0x40` and return EDI = 0.
  Verdict: CONFIRMED (returns nullptr after clearing the flag). Class: keep-original.
- Found in passing (CONFIRMED, keep-original): (a) m_justAbort (+0x49 & 0x10) path calls `CreateSubTask(0x516 TASK_FINISHED, ped = 0)` FIRST and only then clears
  the ped flag (0x656E7D-0x656E96); (b) subtask TASK_SIMPLE_STAND_STILL (0xCB): clears the flag, then `CreateSubTask(0x516, 0)` (0x656EE6-0x656EFA);
  (c) TASK_COMPLEX_FOLLOW_POINT_ROUTE (0x389) jumps to 0x656EF1, i.e. `CreateSubTask(0x516, 0)` WITHOUT clearing the flag (gr's TaskFinished clears it);
  (d) TASK_SIMPLE_ACHIEVE_HEADING (0x386) -> `CreateSubTask(0x389, ped = 0)` (0x656F47-0x656F50; gr passes ped - UNCLEAR whether CreateSubTask(0x389) reads ped);
  (e) 0x384, 0x19B, 0x1A5 and 0x1A6 -> `CreateSubTask(m_routeHasPoints ? 0x386 : 0x1A6, ped)` (0x656EC4 / 0x656F06; for 0x1A6 the same test).
  Effect (c): bIgnoreHeightCheckOnGotoPointTask stays set when the route task finishes.

## 0x656F80 CTaskComplexWalkRoundBuildingAttempt::ComputeCrapRoute  (gr_deviations = 3)
- cpp:140 `+ m_normal * GetRandomNumberInRange(1.f, 2.f)`: Original draws THIS rand first (0x656F89): `s1 = (rand() * 3.0518509e-05f) + 1.0f [0x858624]`;
  `nrm = m_normal * s1` (x, y, z, each stored f32, 0x656FBD-0x656FE7). Verdict: CONFIRMED (formula; gr's evaluation order of the two calls is not guaranteed).
- cpp:139 `+ right * GetRandomNumberInRange(-1.f, 1.f)`: `right = CrossProduct(m_normal, {0,0,1})` 0x59C730 (0x656FE1-0x656FFB); second rand (0x657017):
  `s2 = 2*(rand() * 3.0518509e-05f) - 1.0f` computed as `fmul k; fadd st(0),st(0); fsub 1.0f` (i.e. `2*(r*k) - 1`, 0x657030-0x657038); `right * s2`.
  Verdict: CONFIRMED. Class: keep-original.
- cpp:141 `^^^ Original order of addition was: m_normal, right, pedPos`: Original sums `(nrm + right*s2)` per component (0x657058-0x657076) and then adds the ped
  position whose z is `pos.z - 0.5f [0x858B8C]` (0x657084-0x6570AE). Verdict: CONFIRMED (gr adds pedPos first: different f32 rounding). Class: keep-original.

## 0x657BA0 CTaskComplexWalkRoundBuildingAttempt::ComputeRoute  (gr_deviations = 1)
- cpp:179 `moveDir *= GetRandomNumberInRange(-0.2f, 0.2f) + (float)m_nAttempts`: Original 0x657C42-0x657C83 (after ComputeMoveDir 0x654B80, per iteration):
  `s = (((float)rand() * 3.0518509e-05f) * 0.4f [0x858EE8]) - 0.2f [0x858CC4]`, then `fiadd (int)(int8)m_nAttempts` (movsx byte +0x4A); `moveDir.x *= s`,
  `.y *= s`, `.z *= s` (x87 order x, y, z). Verdict: CONFIRMED (formula). Class: keep-original.
