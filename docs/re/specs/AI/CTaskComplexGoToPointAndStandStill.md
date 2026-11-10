# CTaskComplexGoToPointAndStandStill (AI) - RE-SPEC-3 group I2
Layout: see CTaskComplexGoToPointAndStandStill.deviations.md.

## 0x668250 CTaskComplexGoToPointAndStandStill::SelectMoveState  (catalogue: specified/REA, port level 1)
- Convention oddity: `__thiscall` with `this` unused; stack args `(CTaskSimpleGoToPoint* goTo [esp+4], CPed* ped [esp+8], f32 walkRadius
  [esp+0xC], f32 runRadius [esp+0x10])`, `ret 0x10`.
- `dx = pedPos.x - goTo->+0xC; dy = pedPos.y - goTo->+0x10`; `d2 = dx*dx + dy*dy` (x term first, 2D); if `d2 < walkRadius*walkRadius`
  (strict) `goTo->m_moveState (+8) = 4`; else if `d2 < runRadius*runRadius` -> 6; else 7 (equal/NaN fall to the larger state)
  (0x668267-0x6682BA).

## 0x6682D0 CTaskComplexGoToPointAndStandStill::CreateSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, int32 type [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH. Returns the new task or 0.
- 0x2C0 LEAVE_CAR: `new(0x34) CTaskComplexLeaveCar(ped->m_pVehicle, 0, 0, true, false)`.
- 0xCA PAUSE: `new(0x18) CTaskSimplePause(1)`.
- 0xCB STAND_STILL: if `flags & 2` (stopExactly) and m_pSubTask and its type == 0x384:
  `d = DotProduct(m_vecTargetPoint - pedPos, ped->m_matrix->forward)` (VectorSub 0x40FE60 then 0x40FDB0; the forward pointer is
  `matrix + 0x10` with no null check), stored f32; `s = ped->+0x4DC` (f32);
  blend = (s < 0.01f [0x858C58]) ? 8.0f : (d <= 0.01f or NaN) ? 16.0f : min16(50.0f [0x858B40] / (((d / (s * 0.5f [0x858B8C])) - 1.0f)
  * CTimer::ms_fTimeStep [0xB7CB5C])) where min16 replaces values > 16.0f [0x8599D0] by 16.0f (x87 order: s*0.5; d/that; -1; *timestep;
  50/that). Return `new(0x20) CTaskSimpleStandStill(2000, false, true, blend)`.
  Otherwise `new(0x20) CTaskSimpleStandStill(1, false, false, 8.0f)`.
- 0x384 GO_TO_POINT: `p = new(0x30)`; `p ? CTaskSimpleGoToPoint(p, m_moveState, &m_vecTargetPoint, m_fRadius, false, flags & 1)` 0x667CD0;
  then (also when p == 0: ORIGINAL BUG) if `flags & 8` `p->+0x20 |= 0x10`; if m_moveState == 6 `SelectMoveState(p, ped, m_fMoveStateRadius,
  1e8f)`, if 7 `SelectMoveState(p, ped, m_fMoveStateRadius, 10.0f)`. Return p.
- 0x516 and any other: 0.

## 0x66DB80 CTaskComplexGoToPointAndStandStill::`scalar deleting destructor'  (catalogue: specified/REA, port level 1)
- call 0x668240 (vtbl = 0x86FDA8; jmp ~CTaskComplex); `if (flags & 1) CTask::operator delete(this)`; return this; `ret 4`.
