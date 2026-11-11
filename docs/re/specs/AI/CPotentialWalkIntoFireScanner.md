# CPotentialWalkIntoFireScanner (AI) - RE-SPEC-FINAL group A

## 0x603E70 CPotentialWalkIntoFireScanner::ScanForPotentialWalkIntoFire  (catalogue: unknown/REA, port level 9)
Name note: `this` is CEventScanner+0xC8 (m_nearbyFireScanner, CTaskTimer; call 0x607EBD). The function raises both CEventFireNearby
(vtable 0x85B6E8) and CEventPotentialWalkIntoFire (0x85B6A0); gr's name for the class is CNearbyFireScanner (Events/NearbyFireScanner.h),
proposed full name `CNearbyFireScanner::ScanForNearbyFires(const CPed* ped)`; the catalogue name is kept as the section title.
- Convention: `__thiscall void (CTaskTimer* this, CPed* ped [esp+4])`, `ret 4`. SEH frame (handler 0x83E8E0). Locals (E = esp at entry):
  distSq f32 [E-0x68], dz f32 [E-0x5C], CEventFireNearby [E-0x58], CEventPotentialWalkIntoFire [E-0x38]; the `ped` arg slot [E+4] is reused
  as `moveState`.
- Callees: CTaskManager::GetSimplestActiveTask 0x6819D0, GetActiveTask 0x681720; CTask::IsGoToTask 0x61A360 (cdecl); CFireManager::
  FindNearestFire 0x538F40 (thiscall on gFireManager 0xB71F80, (const CVector& pos, bool 0, bool 0)); CEventFireNearby ctor 0x4B1F10
  (thiscall (const CVector&), `ret 4`) / dtor 0x4B1F80; CEventPotentialWalkIntoFire ctor 0x4B1E20 (thiscall (const CVector& pos, float
  radius, int32 moveState), `ret 0xC`) / dtor 0x4B1F00; CEventGroup::Add 0x4AB420.
- Globals: now [0xB7CB84]=0; 20.0f [0x86C918]=20.0f; 2.0f [0x858CA0]=2.0f; 4.0f [0x86C91C]=4.0f.
- Behaviour:
  1. Timer (0x603E85-0x603EE9): if `!bStarted` { start = now; bStarted = 1; interval = 0 } (bStarted re-tested). If `bStopped` { start = now;
     bStopped = 0 }. Fresh `now` (0x603EC6): if `now < start + interval` (unsigned) return. Else start = now, interval = 100 (0x64), bStarted = 1.
  2. `t = GetSimplestActiveTask(intel+4)`; `moveState = (t && IsGoToTask(t)) ? t->+8 (CTaskSimpleGoTo::m_moveState, i32) : 1` (PEDMOVE_STILL).
  3. `active = GetActiveTask(intel+4)` (kept in EBP). `fire = gFireManager.FindNearestFire(pedPos, false, false)` with pedPos = matrix ?
     matrix->pos : placement.pos.
  4. If `fire` (0x603F58-0x603FF8): x87 `dx = fire.x - p.x; dy = fire.y - p.y; dz = fire.z - p.z` -> dz stored f32; `s = dy*dy; s += dx*dx;
     s += dz_f32*dz_f32` (order: dy^2 first, then dx^2, then dz^2 from memory, 0x603F7F-0x603F91); `fst` s -> f32 distSq. Compare the register s
     with `20.0f*20.0f` (fld 20; fmul 20; fxch; fcompp, st0 = s): continue only if `s < 400` (NaN fails). Then `fabs(dz_f32) < 2.0f`
     (fcomp, `test ah,5; jp`). Then CEventFireNearby(&fire->m_Position (+4)), `intel->m_eventGroup.Add(&ev, false)`, destroy.
  5. Walk-into-fire (0x603FFD-0x6040A4), all required in this order: `active != 0`; `active->GetTaskType() (vtbl+0x10) != 0x202` (514
     TASK_COMPLEX_WALK_ROUND_FIRE); `s2 = GetSimplestActiveTask(intel+4)` != 0 and `IsGoToTask(s2)`; `fire != 0`; `4.0f*4.0f > distSq_f32`
     (fld 4; fmul 4; fcomp distSq; `test ah,0x41; jne` -> need 16 > distSq, NaN fails); `fabs(dz_f32) < 2.0f`. Then
     CEventPotentialWalkIntoFire(&fire->m_Position, fire->m_Strength (+0x1C, f32 passed as dword), moveState), Add(&ev, false), destroy.
     Step 5 does not require step 4's 20 m test to have passed (only fire != 0).
- gta-reversed: absent (NearbyFireScanner.h only). Spec from the bytes.
- Open questions: none.
