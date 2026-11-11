# CTaskGoToVehicleAndLean (AI) - RE-SPEC-3 group I2
Type 0x4C0 (1216), size 0x24 (Clone allocates 0x24; gr VALIDATE_SIZE 0x18 is wrong). Layout: +0xC m_Vehicle, +0x10
m_LeanAnimDurationInMs, +0x14 u8 "gangs" anim block ref held (gr calls it m_LeanOnVehicle), +0x15 u8 lean on the max-x side (written by
CalcTargetPos 0x664770), +0x18 CVector target position. Vehicle `+0x42E bit 0` = "a ped is leaning on / going to lean on this vehicle".
DoTidyUp 0x660F60 (not in this group).

## 0x660ED0 CTaskGoToVehicleAndLean::GetTaskType  (catalogue: specified/REA, port level 0)
- returns 0x4C0.

## 0x6621B0 CTaskGoToVehicleAndLean::Clone  (catalogue: specified/REA, port level 3)
- `p = new(0x24)`; if p `CTaskGoToVehicleAndLean(p, m_Vehicle, m_LeanAnimDurationInMs)` 0x660E60; return p / 0.

## 0x6644E0 CTaskGoToVehicleAndLean::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- call 0x660EE0; `if (flags & 1) CTask::operator delete`; return this.

## 0x664500 CTaskGoToVehicleAndLean::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped, int32 prio, const CEvent* ev)`, `ret 0xC`.
- prio == 2: `m_pSubTask->MakeAbortable(ped, 2, ev)` (result ignored, no null check); if m_Vehicle `veh->+0x42E &= ~1`; if m_pSubTask
  (re-read) `m_pSubTask->MakeAbortable(ped, 2, nullptr)` (a second abort call); return 1.
- else: `if (!m_pSubTask->MakeAbortable(ped, prio, ev)) return 0`; clear the vehicle bit; if m_pSubTask `MakeAbortable(ped, 2, nullptr)`;
  return 1.

## 0x664D40 CTaskGoToVehicleAndLean::CreateFirstSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`, SEH. ABORT = `if (m_pSubTask) m_pSubTask->MakeAbortable(ped, 2, nullptr)`; return 0.
- If m_Vehicle == 0 -> ABORT. If `veh->+0x42E & 1`: clear the bit (!) and ABORT. Else set the bit; `+0x18 = CalcTargetPos(&tmp, ped)`;
  `d = +0x18 - pedPos`; if `(dx*dx + dy*dy) + dz*dz < 1.0f [0x858624]` (strict) return 0 (no subtask; the vehicle bit stays set);
  else `p = new(0x28)`; return p ? `CTaskComplexGoToPointAndStandStill(p, 4, &+0x18, 0.05f, 2.0f, false, true)` : 0.

## 0x664E60 CTaskGoToVehicleAndLean::ControlSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped)`, `ret 4`.
- If m_Vehicle == 0: `if (m_pSubTask) MakeAbortable(ped, 2, nullptr)`; return 0. If `ped->intel->+0x274 > [0x86C938] (30)`: clear the
  vehicle bit, abort the subtask (prio 2) if any, return 0.
- `s = m_Vehicle->GetSpeed(pedPos - vehPos)` 0x542CE0 (CPhysical, point passed by value). TRY = `if (m_pSubTask->MakeAbortable(ped, 1, nullptr))
  { DoTidyUp(ped); return 0 }`. If `veh->m_pDriver` != 0 -> TRY; else if `sqrt((s.x*s.x + s.y*s.y) + s.z*s.z) > 0.01f [0x858C58]` -> TRY.
  (A failed TRY continues.)
- `t = CalcTargetPos(&tmp, ped)`; `e = t - +0x18`; if `(e.x*e.x + e.y*e.y) + e.z*e.z > 0.010000001f [0x86CD7C]` (strict) -> TRY.
- "gangs" anim block management on +0x14 (as CTaskGangHasslePed: ShouldLoadGangAnims, AddAnimBlockRef / RequestModel / RemoveAnimBlockRef).
  Return m_pSubTask.

## 0x660F60 CTaskGoToVehicleAndLean::DoTidyUp  (catalogue: specified/REA, port level 0)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. If this+0xC (vehicle) != NULL: byte vehicle+0x42E &= 0xFE (clear a vehicle
  flag bit 0 - gr CVehicle flag at +0x42E, the "ped leaning on it" bit). If this+8 (m_pSubTask) != NULL:
  m_pSubTask->MakeAbortable(ped, 2, NULL) (vtable +0x18; result ignored).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x664590 CTaskGoToVehicleAndLean::CreateNextSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; SEH frame. Fields: +0xC vehicle, +0x10 lean anim time/arg, +0x14 (u8) "lean" flag, +0x15 (u8) side flag.
- Behaviour:
  1. If `veh == NULL`: if `m_pSubTask` -> `m_pSubTask->MakeAbortable(ped, 2, NULL)` (vslot +0x18); return 0.
  2. `t = m_pSubTask->GetTaskType()`. If `t == 0x387` (TASK_COMPLEX_GO_TO_POINT_AND_STAND_STILL): `R = veh->m_matrix->right` (+0..+8, no NULL check); if `+0x15 == 0` -> `R = -R` (each component fchs, f32 stores); `a = GetRadianAngleBetweenPoints(R.x, R.y, 0, 0)` 0x53CBE0 stored as f32; `CGeneral::LimitRadianAngle(a)` 0x53CB50 is called but its ST0 result is discarded (`fstp st(0)`, 0x664653) - original quirk, the UNLIMITED angle is used; `p = new(0x18)`; NULL -> 0; return `CTaskSimpleAchieveHeading(p, a, [0x86FC7C]=0.5f, [0x86FC80]=0.2f)` 0x667E20.
  3. If `t == 0x386` (TASK_SIMPLE_ACHIEVE_HEADING) or `t == 0xCB` (TASK_SIMPLE_STAND_STILL): if `+0x14 != 0` -> `new(0x2C) CTaskLeanOnVehicle(veh, +0x10, 0)` 0x660F90; else `new(0x20) CTaskSimpleStandStill(500, false, false, 8.0f)` 0x62F310 (0 on allocation failure).
  4. Otherwise `CTaskGoToVehicleAndLean::DoTidyUp(this, ped)` 0x660F60 and return 0.
