# CVehiclePotentialCollisionScanner (VEH) - full specs (writer group E)

Layout (gr `Events/VehiclePotentialCollisionScanner.h`): +0 `CTaskTimer m_timer` (+0 m_nStartTime u32,
+4 m_nInterval i32, +8 m_bStarted, +9 m_bStopped).

## 0x603720 CVehiclePotentialCollisionScanner::ScanForVehiclePotentialCollisionEvents  (catalogue: specified/REA, port level 9)
- Convention: `__thiscall void (this, const CPed* ped [esp+4], CEntity** entities [esp+8], int32 count [esp+0xC])`,
  `ret 0xC`. The `entities` and `count` arguments are NOT read (gr passes the vehicle scanner list and 16).
  SEH frame for the event local.
- Callers: CEventScanner::ScanForEventsNow 0x6053D0, CEventScanner::ScanForEvents 0x607E30.
- Callees: CTaskManager::GetSimplestActiveTask 0x6819D0, CTask::IsGoToTask 0x61A360 (cdecl), CEntity::GetColModel
  0x535300, CVector::FromMultiply 0x59C670 (thiscall (const CMatrix&, const CVector&), ret 8, full transform),
  0x406DA0 (CVector::SquaredMagnitude, thiscall -> ST0 = (x*x + y*y) + z*z),
  CPedGeometryAnalyser::GetIsLineOfSightClear 0x5F5A30 (cdecl (const CPed&, const CVector&, CEntity&, float&) -> AL),
  CPedIntelligence::GetMoveStateFromGoToTask 0x601D70, CEventPotentialWalkIntoVehicle ctor 0x4AE320 (thiscall
  (CVehicle*, int32 moveState), ret 8), CEventGroup::Add 0x4AB420, event destructor 0x4AE3C0.
- Constants: 0x858C1C = -1.0f, 0x86C8E8 = 0.5f, 0x86C8DC = 5.0f, 0x86C8E0 = 0.5f.
- Behaviour:
  1. Timer: if `!m_bStarted`: `start = now; m_bStarted = 1; interval = 500`. If `m_bStopped`: `start = now;
     m_bStopped = 0`. If `now < start + interval` (unsigned) return. Then `start = now; interval = 500;
     m_bStarted = 1`.
  2. `task = ped->m_pIntelligence->m_TaskMgr.GetSimplestActiveTask()`; return if null or `!IsGoToTask(task)`.
  3. `veh = *(CVehicle**)(ped->m_pIntelligence + 0x120)` (the dword just before `m_pedScanner` 0x124, i.e. the
     vehicle scanner's closest-entity slot); return if null.
  4. `d = pedPos - vehPos` (x, y, z stored f32; positions = matrix pos or placement).
  5. `col = veh->GetColModel()`; `minW = veh->m_matrix * col->bbox.min`, `maxW = veh->m_matrix * col->bbox.max`
     (no null check on m_matrix); `up = m_matrix->up`.
     `dTop = -(up.x*maxW.x + (up.y*maxW.y + up.z*maxW.z))` (kept in ST0);
     `n = (up.x * -1.0f, up.y * -1.0f, up.z * -1.0f)` (stored f32);
     `dBot = -(n.x*minW.x + (minW.y*n.y + minW.z*n.z))` (stored f32).
     `inSlab = (up.z*p.z + up.y*p.y + up.x*p.x + dTop < 0.5f) && (n.y*p.y + n.z*p.z + n.x*p.x + dBot < 0.5f)`
     (p = ped position; NaN -> false).
  6. `range = (veh->m_nVehicleSubType == 6) ? 5.0f + 5.0f : 5.0f`. Return if `!inSlab`.
  7. If ped flag dword +0x474 bit 3 (flag #67 bHasJustLeftCar) is clear: return unless
     `(d.x*d.x + d.y*d.y) + d.z*d.z < range*range`. (When the flag is set the distance test is skipped.)
  8. `frac = 0`; if `GetIsLineOfSightClear(ped, *(CVector*)(task + 0xC) /*go-to target*/, veh, frac)` return;
     return if `frac <= 0.5f`.
  9. `CEventPotentialWalkIntoVehicle ev(veh, ped->m_pIntelligence->GetMoveStateFromGoToTask())`;
     `ped->m_pIntelligence->m_eventGroup (+0x68).Add(&ev, false)`; destroy ev.
- gta-reversed: `plugin::CallMethod` only (EventScanner.cpp:34).
- Open questions: the gr member name of CPedIntelligence +0x120 (offset and use are exact).
