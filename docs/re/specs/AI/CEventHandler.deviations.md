# CEventHandler: gta-reversed deviation notes (AI, group I2)

Source: re/gta-reversed/source/game_sa/Events/EventHandler.cpp (pin 20d8d2a). `this+0x24` = m_EventResponseTask, `this+0` = m_Ped.
General finding for the Compute*Response functions below: the original writes m_EventResponseTask only on the paths that build a
task (or explicitly on TASK_NONE); the "null source" and "unknown task id" paths return WITHOUT touching it, whereas gr's
`m_EventResponseTask = [&]{...}()` lambda pattern always assigns (nullptr). A port must keep the previous value on those paths.

## 0x4B99F0 CEventHandler::ComputeSexyPedResponse  (gr_deviations = 1)
Row EventHandler.cpp:2232 (`NOTSA_UNREACHABLE(); // Not sure`, default of the m_TaskId switch).
- Original bytes (`__thiscall void (CEventSexyPed* e, CTask*, CTask*)`, `ret 0xC`): if `e->m_SexyPed (+0x14) == 0` -> return without
  writing (0x4B9A12 -> 0x4B9A89); `id = (i16)e->m_TaskId (+0xE)`: id == 0xC8 (TASK_NONE) -> `+0x24 = 0` (0x4B9A82); id == 0x4BC
  (GANG_HASSLE_PED) -> `p = new(0x2C)`, `+0x24 = p ? CTaskGangHasslePed(p, sexyPed, 0, 10000, 30000) 0x65FED0 : 0`; any other id ->
  return without writing (0x4B9A24 -> 0x4B9A89).
- Verdict: CONFIRMED (gr hits NOTSA_UNREACHABLE for other ids and assigns nullptr for a null m_SexyPed; the original leaves
  m_EventResponseTask unchanged in both cases). Class: keep-original.

## 0x4C19A0 CEventHandler::ComputePedThreatResponse  (gr_deviations = 1)
Row EventHandler.cpp:1822 (`NOTSA_UNREACHABLE();`, default of the m_TaskId switch).
- Original bytes: null `e->m_AcquaintancePed (+0x14)` -> exit 0x4C241D without writing (0x4C19C9); the id dispatch (compare tree
  0x39F / 0x19D, byte table 0x4C2450 + jump table 0x4C2434 for ids 0xC8..0x19C, and a second table 0x4C2528 for ids > 0x39F) sends
  every unhandled id to the same exit 0x4C241D (`ja 0x4C241D` at 0x4C19FF and 0x4C1D57). Writes to +0x24 happen only at 0x4C1A13
  (TASK_NONE, writes 0), 0x4C1CC0, 0x4C230F, 0x4C2320, 0x4C23E5, 0x4C23F8 and 0x4C241A (allocation failure, writes 0).
- Verdict: CONFIRMED (default and null-ped leave the field unchanged; gr assigns nullptr / traps). Class: keep-original.
- Open question: the inner `return nullptr` sub-cases of gr (e.g. GANG_DRIVEBY preconditions, KILL_CRIMINAL, SEQUENCE) were not
  re-checked one by one; any of them that jumps to 0x4C241D in the bytes also leaves the field unchanged.

## 0x4C2610 CEventHandler::ComputePotentialPedCollideResponse  (gr_deviations = 2)
Row EventHandler.cpp:2056 (FIX_BUGS block: sqrt vs squared radius test).
- Original bytes (0x4C2776-0x4C27CB): `v = leaderPos - pedPos` via VectorSub 0x40FE60, then `v.z = 0` (0x4C27A9); `m = |v|^2`
  (0x406DA0); `r = seekTask->+0x1C`; condition `(m - r*r) < r*r` (fld r; fmul r; fxch; fsub; fcomp; `test ah,5; jp`) -> moveState = 4.
- Verdict: CONFIRMED that the original uses the squared form (gr's `#else` branch), with one more difference: the original distance
  is 2D (z cleared) while gr's `SquaredMagnitude()` is 3D. Class: keep-original.
Row EventHandler.cpp:2077 (`NOTSA_UNREACHABLE();`).
- Original: null `e->m_ped (+0x20)` and any id other than 0xC8 / 0x395 / 0x4B9 exit at 0x4C281E without writing +0x24.
  Additional difference: after constructing CTaskComplexAvoidOtherPedWhileWandering(ped, &e->m_targetPoint, moveState) 0x66A100 the
  original sets `task->+0x5C |= 4` when a gang-follower task exists (0x4C2815) - also when the allocation failed (writes to null+0x5C,
  ORIGINAL BUG); gr passes the flag as a ctor argument instead.
- Verdict: CONFIRMED. Class: keep-original.

## 0x4C2FC0 CEventHandler::ComputeVehicleDamageResponse  (gr_deviations = 1)
Row EventHandler.cpp:2523 (`if (!notsa::IsFixBugs() || m_Ped->m_pVehicle)`).
- Original bytes (0x4C3317-0x4C33A6): when the kill task is not appropriate: `veh = m_Ped->m_pVehicle`; if veh and `veh->IsDriver(ped)`
  -> `new(0x2C) CTaskComplexCarDriveMissionFleeScene(veh)` 0x4B8920; else -> `new(0x40) CTaskComplexSmartFleeEntity(veh, false,
  [0xC18CEC], 1000000 [0x86F674], 1000 [0x86F678], [0xC18CF0])` 0x65C430 - allocated even when veh is null (flee from a null entity).
- Verdict: CONFIRMED (the original has no vehicle check before the SmartFlee allocation; gr skips it under FIX_BUGS). Class: keep-original.
