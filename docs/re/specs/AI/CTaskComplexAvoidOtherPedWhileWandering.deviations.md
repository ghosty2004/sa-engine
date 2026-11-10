# CTaskComplexAvoidOtherPedWhileWandering (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

Layout (gr header, size 0x60): +0xC m_PedToAvoid, +0x10 m_StartPt, +0x1C m_TargetPt, +0x28 m_DetourTargetPt, +0x34 m_NewTargetPt, +0x40 m_Timer,
+0x4C m_DontQuitYetTimer, +0x58 m_MoveState, +0x5C bits (0 m_IsDoingIK, 1 m_WantsToQuit, 2 m_IsMovingTarget).

## 0x66A100 CTaskComplexAvoidOtherPedWhileWandering::Constructor  (gr_deviations = 1)
- TaskComplexAvoidOtherPedWhileWandering.cpp:38 `//VVV NOTSA ARGS VVV// bool isMovingTarget` (header default false):
  Original: `__thiscall this (CPed* ped, const CVector* target, eMoveState moveState)`, `ret 0xC`. `CTaskComplex::CTaskComplex` 0x61A3B0; `+0xC = ped`;
  `vtbl = 0x86FEC4`; `m_TargetPt = *target` and `m_DetourTargetPt = *target` (dword copies, 0x66A136-0x66A160); timers `+0x40, +0x44 = 0, +0x48, +0x49 = 0,
  +0x4C, +0x50 = 0, +0x54, +0x55 = 0`; `+0x58 = moveState`; `+0x5C &= 0xF8` (bits 0-2 cleared, others kept); `if (ped) RegisterReference(ped, this+0xC)`.
  m_StartPt (+0x10) and m_NewTargetPt (+0x34) are NOT written (pool garbage), whereas gr value-initialises them to zero.
- Verdict: REFUTED for the marked row (default false = bit 2 cleared, as in the original). Found in passing: gr zero-initialises +0x10/+0x34
  (original leaves them) - harmless unless read before being set (UNCLEAR; not checked in the readers).

## 0x6721B0 CTaskComplexAvoidOtherPedWhileWandering::ControlSubTask  (gr_deviations = 1, plus 2 helper rows)
Original: `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH.
- TaskComplexAvoidOtherPedWhileWandering.cpp:82 `return new CTaskSimpleStandStill{ CGeneral::GetRandomNumberInRange(500, 2500) }`: Original
  (after `ped->m_pIntelligence->[+0x274] > [0x86C938] (= 30)` signed, 0x672270-0x672282, and QuitIK 0x66A230): `p = CTask::operator new(0x20)` FIRST;
  only if p != 0: `t = GetRandomNumberInRange(0, 2000)` 0x407180 `+ 500` and `CTaskSimpleStandStill(p, t, 0, 0, 8.0f)` 0x62F310
  (0x67228C-0x6722C8). So the value is `500 + trunc(r * 2000 / 32768)` and no rand() is consumed when the task pool is full.
  Verdict: CONFIRMED (helper formula; gr also may evaluate rand before the allocation). Class: keep-original.
- TaskComplexAvoidOtherPedWhileWandering.cpp:116 (`dyn_cast_if_present<CTaskComplexGangFollower>(FindTaskByType(TASK_COMPLEX_GANG_FOLLOWER))`): Original
  0x672491-0x6724EA: `t = intel->FindTaskByType(0x4B7)` 0x600EE0; if t and `leader = t->[+0x10]` != 0: `off = t->CalculateOffsetPosition()` 0x65ED40
  (computed BEFORE the leader position is read), `m_TargetPt (+0x1C) = leaderPos + off` (VectorAdd-style helper 0x40FE30 (out, a, b), dword copy).
  Verdict: REFUTED (same result; the type check is redundant). Class: harmless.
- TaskComplexAvoidOtherPedWhileWandering.cpp:123 `notsa::cast<CTaskSimpleGoToPoint>(m_pSubTask)->UpdatePoint(m_TargetPt)`: Original 0x6724FD-0x67250D:
  `CTaskSimpleGoToPoint::UpdatePoint(m_pSubTask, &m_DetourTargetPt (this+0x28), 0.5f [0x86FC78], 0)` 0x645700 - the DETOUR target, not m_TargetPt.
  Verdict: CONFIRMED (gr passes the wrong point; the marker was classified harmless/no). Class: keep-original.
- Found in passing, CONFIRMED, keep-original: gr cpp:112 `if (!m_IsMovingTarget) return DoQuit();` - the original does NOT quit here: after the
  200 ms timer restart (0x672478-0x672488) `test byte [esi+0x5C], 4; je 0x6724ED` only SKIPS the gang-follower target update and still calls
  ComputeDetourTarget 0x672180 (0x6724ED-0x6724F7), quitting only if that fails (0x672306).
