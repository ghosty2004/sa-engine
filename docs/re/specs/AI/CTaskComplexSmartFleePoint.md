# CTaskComplexSmartFleePoint (AI) - RE-SPEC-3 group I3

Vtable 0x86F744.

## 0x65BDA0 CTaskComplexSmartFleePoint::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax,0x38E; ret` -> 910 (TASK_COMPLEX_SMART_FLEE_POINT). Slot 4. gr (hook `.Reversed = false`) matches.

## (sections below added by the lead for addresses the I3 writer left unwritten)
Layout (gr, confirmed): +0xC m_initalPos, +0x18 m_fleePoint, +0x24 m_doScream u8, +0x28 m_fleeTimeMs, +0x2C m_safeDist f32,
+0x30 m_moveState, +0x34 CTaskTimer {start, interval +0x38, started +0x3C, stopped +0x3D}, +0x40 m_fleeDir u8, +0x41
m_hasFleePointChanged u8.

## 0x65BE80 CTaskComplexSmartFleePoint::CreateSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, int32 type [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH. Returns 0 when an allocation fails.
- 0xCB: `SetDefaultTaskWanderDir(ped)` 0x65BE00; `new(0x20) CTaskSimpleStandStill(0, false, false, 8.0f)`.
- 0xF4 (SEQUENCE): only when `m_moveState == 6` (else 0): `seq = new(0x40) CTaskComplexSequence()` (may be 0 - AddTask then runs with
  ECX = 0, ORIGINAL BUG); `seq->AddTask(new(0x20) CTaskSimpleRunAnim(0, 0xA5, 4.0f, false))`; `seq->AddTask(new(0x1C) CTaskSimpleTired(2000))`
  0x630F20 (0 when that allocation fails); return seq.
- 0x2CA (LEAVE_ANY_CAR): `p = new(0x14)`; if p: inlined ctor `CTaskComplex()`, vtbl = 0x858EB0, `+0xC = 0`, `+0x10 = 0`, `+0x11 = 1`
  (CTaskComplexLeaveAnyCar(0, false, true)).
- 0x390 (WANDER): `p = new(0x28)`; if p: `CTaskComplexWander::CTaskComplexWander(p, m_moveState, m_fleeDir, false, 0.5f [0x86FCB0])` 0x66F450,
  then vtbl = 0x86F690.
- 0x516 and other: 0.
- gta-reversed: `.Reversed = false` (plugin::Call).

## 0x65C140 CTaskComplexSmartFleePoint::CreateFirstSubTask  (catalogue: specified/REA, port level 2)
- `m_initalPos = pedPos`. If ped bInVehicle and `ped->m_pVehicle`: return `CreateSubTask(0x2CA, ped)`. Else `m_fleeDir = (u8)
  CGeneral::GetNodeHeadingFromVector(P.x - m_fleePoint.x, P.y - m_fleePoint.y)` 0x53CDC0 (differences stored f32); return
  `CreateSubTask(0x390, ped)`.

## 0x65C1E0 CTaskComplexSmartFleePoint::ControlSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`. `t = m_pSubTask`.
- If `t->GetTaskType() != 0x390`: `if (IsLooking(ped)) AbortLookAt(ped, 250)`; return t.
- Wander subtask: if m_doScream `ped->Say(0x15B, 0, 0.1f, 0, 0, 0)`; `t->+0xC (moveState) = m_moveState`.
  - If m_hasFleePointChanged: clear it; if the timer is started: restart it (start = now, interval = m_fleeTimeMs, started = 1);
    `d = ComputeFleeDir(ped)` 0x65BE40; if `d != m_fleeDir`: `m_fleeDir = d`, `0x669D30(t, d)` (CTaskComplexWander::SetDirection: if the
    stored byte +0x10 differs, store it and set flag +0x24 |= 2).
  - Else if the timer is started and `CTaskTimer::IsOutOfTime(&+0x34)`: `t = CreateSubTask(0xCB, ped)` (returned instead of the wander task).
  - Else: `s2 = m_safeDist * m_safeDist` (stored f32); if `(dz*dz + dy*dy) + dx*dx > s2` for `d = m_fleePoint - pedPos` and also for
    `d = m_initalPos - pedPos` (both strict): `t = CreateSubTask(0xCB, ped)`.
- Then if `m_moveState == 6` and `!IsLooking(ped)` and `GetRandomNumberInRange(0, 100) <= 5`: `g_ikChainMan.LookAt("TaskSmartFleePoint"
  0x86F770, ped, nullptr, 2000, -1, &m_fleePoint, false, 0.25f, 500, 3, false)`. Return t.

## 0x65BDC0 CTaskComplexSmartFleePoint::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped, int32 priority, const CEvent* event)`, `ret 0xC`.
- If priority == 0 (URGENT? value 0): +0x28 = -1; +0x34 = m_snTimeInMilliseconds [0xB7CB84]; +0x38 = -1; +0x3C = 1 (u8) (restart the
  flee timer, gr m_Timer: start/interval/started). Then (always) return m_pSubTask (+8)->MakeAbortable(ped, priority, event)
  (vtable +0x18; result of the sub task is returned unchanged).

## 0x65BE00 CTaskComplexSmartFleePoint::SetDefaultTaskWanderDir  (catalogue: specified/REA, port level 1)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. t = ped->m_pIntelligence (+0x47C) -> +0x14 (task manager primary slot 4,
  TASK_PRIMARY_DEFAULT); if t != NULL and t->GetTaskType() (vtable +0x10) == 0x390 (TASK_COMPLEX_WANDER): 0x669D50(t, this->m_pSubTask
  (+8)) (thiscall on t, 1 arg; unnamed CTaskComplexWander helper that compares/copies the node-address fields +0x18.. from the given
  wander task - proposed `CTaskComplexWander::UpdateDir(CTask*)`, UNCLEAR).

## 0x65BE40 CTaskComplexSmartFleePoint::ComputeFleeDir  (catalogue: specified/REA, port level 2)
- `__thiscall int32 (this, CPed* ped [esp+4])`, `ret 4`, returns EAX from CGeneral::GetNodeHeadingFromVector 0x53CDC0 (cdecl (x, y)).
- p = ped->m_matrix (+0x14) ? &matrix->pos (+0x30) : &ped->placement.pos (+4). dy = p.y - this+0x1C (f32 fld/fsub/fstp) stored first,
  then dx = p.x - this+0x18; call GetNodeHeadingFromVector(dx, dy) (pushed as two f32: [esp] = dx, [esp+4] = dy). this+0x18 is the
  flee position (m_vecFleePos).

## 0x65C3C0 CTaskComplexSmartFleePoint::SetFleePosition  (catalogue: specified/REA, port level 0)
- `__thiscall void (this, const CVector* pos [esp+4], float safeDistance [esp+8], bool scream [esp+0xC])`, `ret 0xC`.
- If this+0x18 != pos.x or +0x1C != pos.y or +0x20 != pos.z or +0x2C != safeDistance (each `fcomp; test ah,0x44; jp` = not equal
  or NaN counts as different): +0x18/+0x1C/+0x20 = pos (raw dwords), +0x2C = safeDistance, +0x41 = 1 (u8, new-flee-pos flag),
  +0x24 = scream. Else only +0x24 = scream.

## 0x65CED0 CTaskComplexSmartFleePoint::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x44) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexSmartFleePoint::CTaskComplexSmartFleePoint(&this->m_vecFleePos (this+0x18), (u8)this+0x24, this+0x2C, this+0x28) 0x65BD20; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x65C0C0 CTaskComplexSmartFleePoint::CreateNextSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`.
- Behaviour: `t = m_pSubTask->GetTaskType()` then a cumulative `sub` chain (0x65C0CE-0x65C0DF): `t == 0xCB` (TASK_SIMPLE_STAND_STILL) -> return `CreateSubTask(this, 0xF4 TASK_COMPLEX_SEQUENCE, ped)` 0x65BE80; `t == 0xF4` -> return `CreateSubTask(0x516 TASK_FINISHED, ped)`; `t == 0x2CA` (TASK_COMPLEX_LEAVE_ANY_CAR) -> `this->+0x40 = ComputeFleeDir(this, ped)` 0x65BE40 (AL stored as u8) and return `CreateSubTask(0x390 TASK_COMPLEX_WANDER, ped)`; else return 0.
