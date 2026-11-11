# CTaskComplexEnterCar: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

Layout used (gr TaskComplexEnterCar.h): +0xC m_Car, +0x10 flag byte (bit0 m_bAsDriver, bit1 m_bQuitAfterOpeningDoor, bit2
m_bQuitAfterDraggingPedOut, bit3 m_bCarryOnAfterFallingOff), +0x14 m_TargetDoor, +0x3C m_bAborting, +0x48 m_CruiseSpeed (f32).
Vehicle +0x3D0 = autopilot cruise speed (u8), +0x460 m_pDriver.

## 0x63E040 CTaskComplexEnterCar::CreateSubTask  (gr_deviations = 3: 1 maybe, 2 no)
Original dispatch (`__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH; result slot [esp+0xC] = 0 at
0x63E068): compare tree 0x325 / 0x2CF / 0xD0 / 0xC8 / 0xCB (0x63E060-0x63E0A0), jump table 0x63E960 for 0x320..0x324 (`ja` 0x63E326),
`0x330`, jump table 0x63E974 for 0x326..0x32C (entry 3 = 0x63E947 = no case) (0x63E4B1-0x63E4D0), further compares from 0x63E64B.
Unmatched types jump to 0x63E947 and return the slot value 0.
Row TaskComplexEnterCar.cpp:735 (`NOTSA_UNREACHABLE("taskType = {}", taskType)`):
- Original: returns nullptr (0x63E0A0 / 0x63E326 / 0x63E4CA -> 0x63E947). Verdict: CONFIRMED. Class: keep-original.
Row TaskComplexEnterCar.cpp:666 (bug note on `!bInVehicle` for the player, 0x63E7A4):
- Original (TASK_FINISHED tail, 0x63E7A2-0x63E841): player branch = `IsPlayer() && !(ped flags +0x46C bit 8, bInVehicle) && m_Car &&
  m_Car->m_pDriver && !(flags +0x10 & 2)` -> if `!(m_CruiseSpeed < 0.0f)` (`test ah,1`; NaN skips) `m_Car->cruiseSpeed (u8 +0x3D0) =
  ftol(m_CruiseSpeed)`; otherwise (0x63E7F6) `!IsPlayer() && bInVehicle && m_Car && !(m_CruiseSpeed < 0)` -> same store. Returns 0.
  The note (player tests !bInVehicle) is accurate: REFUTED as a deviation for that part.
- BUT gr requires `m_bQuitAfterOpeningDoor` to be SET, the original requires bit1 CLEAR (`test byte [esi+0x10],2; jne skip` at 0x63E7C9).
  Verdict for that sub-expression: CONFIRMED. Class: keep-original.
Row TaskComplexEnterCar.cpp:715 (assert): REFUTED (debug only). Class: harmless.

## 0x63E990 CTaskComplexEnterCar::CreateNextSubTask  (gr_deviations = 4: 3 maybe, 1 no)
Row TaskComplexEnterCar.cpp:134 (`NOTSA_UNREACHABLE` in the m_bAborting branch):
- Original (m_bAborting +0x3C != 0, 0x63E9AB-0x63EA49): EDI = 0; subtask types 0xC8, 0xCB, 0xD0, 0x2C0, 0x2CB, 0x2CF, 0x320..0x328 and the
  byte-table cases (0x63F900 / 0x63F8F4 for 0x32A..0x33D, entry 0 = 0x63E9ED) return CreateSubTask(0x516 TASK_FINISHED); byte-table entry 1
  -> 0x63EA50; entry 2 and everything else -> return EDI = 0 (0x63EA1F / 0x63F8B8).
- Verdict: CONFIRMED (gr aborts, original returns nullptr). Class: keep-original.
Row :142 (inlined-helpers comment): REFUTED (comment only). Class: harmless.
Row :193 (`TASK_SIMPLE_CAR_OPEN_LOCKED_DOOR_FROM_OUTSIDE`: set cruise speed, then STAND_STILL; "does not make sense"):
- Original case 0x63ED58 (jump table 0x63F914 entry 3): `if (m_Car && !(m_CruiseSpeed < 0.0f [0x858B50])) m_Car->cruiseSpeed (u8 +0x3D0) =
  ftol(m_CruiseSpeed)` (NaN skips, as in gr); return `CreateSubTask(0xCB, ped)` (0x63ED58-0x63ED89).
- Verdict: REFUTED (gr matches the bytes; byte-sized store via ftol). Class: harmless.
Row :355 (`NOTSA_UNREACHABLE` at the end of the main switch):
- Original: types outside the handled set return EDI = 0 (e.g. `ja 0x63F8B6` at 0x63EBD6, 0x63F8B8). Verdict: CONFIRMED. Class: keep-original.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x63ACC0 CTaskComplexEnterCar::CreateTaskUtilityLineUpPedWithCar  (gr_deviations = 2)
Original bytes (`__thiscall void (this, CPed* ped [esp+4])`, `ret 4`, SEH): P = ped->m_matrix ? matrix+0x30 : ped+4 (0x63ACDF-0x63ACEA);
`CCarEnterExit::GetPositionToOpenCarDoor(&tmp, m_Car (+0xC), m_TargetDoor (+0x14))` 0x64E740 (cdecl, hidden result pointer); `off = tmp - P` component-wise x, y, z,
each `fld; fsub; fstp` f32 (0x63ACFF-0x63AD19); `p = operator new(0x1C)` 0x82119A (0x63AD1D); if p: `CTaskUtilityLineUpPedWithCar::Constructor(p, &off, 600 (0x258), 0,
m_TargetDoor)` 0x64FBB0 (0x63AD47) else 0; `this+0x38 (m_LineUpUtility) = result` (0x63AD54).
Row TaskComplexEnterCar.cpp:781 (`if (m_LineUpUtility && notsa::IsFixBugs())` placement-new reuse):
- Original bytes: no test of the old pointer; 0x63AD54 overwrites +0x38 unconditionally - an existing line-up utility leaks (never destroyed or freed).
- Verdict: CONFIRMED (only with FixBugs on; gr default path matches the original allocation). Class: keep-original (keep the leak semantics; Rust may drop the old box
  only if it is unobservable - it is: the object has no side effects in its dtor besides freeing; record in the ledger if changed).
Row TaskComplexEnterCar.cpp:790 (`assert(!m_LineUpUtility)`):
- Original bytes: no check (0x63ACD8-0x63AD54).
- Verdict: REFUTED (debug-only assert, no release behaviour). Class: harmless.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x63F970 CTaskComplexEnterCar::CreateNextSubTask_AfterSimpleCarAlign  (gr_deviations = 3)
Original bytes: `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH (0x58-byte frame). Only the rows below are re-checked here; the rest of the body is gr-reversed.
Rows TaskComplexEnterCar.cpp:373 / :374 (`GetRandomNumberInRange(2'000, 3'000)` / `(6'000, 8'000)` for the g_ikChainMan.LookAt time):
- Original bytes: the helper is INLINED. For each occupant: passengers first, `ebp = 0x464 .. 0x480` step 4 (m_apPassengers[0..7], 0x64014B-0x64020B), then the driver
  (+0x460, 0x640211-0x6402C7). If `ped->m_pIntelligence->IsFriendlyWith(occ)` 0x601BC0 or `CPedGroups::AreInSameGroup(ped, occ)` 0x5F7F40:
  `t = 2000 - ftol((rand() & 0xFFFF) * 3.05175781e-05f [0x858B14] * -1000.0f [0x859948])` (0x6401AC-0x6401E8; `_rand` 0x821B1E, `_ftol` 0x821B40 truncation), else
  `t = 6000 - ftol((rand() & 0xFFFF) * 3.05175781e-05f [0x858B14] * -2000.0f [0x86EAA8])` (0x640182-0x6401A5). That equals the original helper 0x407180 formula
  (`min + trunc(r/32768 * (max-min))`, the negation is exact), i.e. ranges 2000..2999 / 6000..7999. LookAt 0x618970 (ECX = 0xC15448) args in order:
  ("TaskEnterCar" 0x86EA98, occ, ped, t, 5, 0, 0, 0.5f, 350, 3, 0) (0x6401D4-0x6401F3).
- Verdict: CONFIRMED (gr's helper uses `lerp(min, max-1, rand()/32767.0f)`, cat14 finding 1). Class: keep-original (2 rows).
Row TaskComplexEnterCar.cpp:487 (`assert(m_DraggedPed)`):
- Original bytes: no check; after the SkipState branch, 0x63FF9D-0x63FFA0 dereferences `[esi+0x40]` (m_DraggedPed) -> `+0x47C` unconditionally (null would fault).
- Verdict: REFUTED (debug-only assert; release behaviour identical, both crash on null). Class: harmless.
Additional difference found: cpp:475 `CEntity::ChangeEntityReference(m_DraggedPed, ComputeSlowJackedPed(...))`; the original (0x63FE54-0x63FE69) stores the new pointer into +0x40
directly and only calls `RegisterReference` 0x571B70 when non-null - the OLD reference is never cleaned up (no call to 0x571A00 in this function). Verdict: CONFIRMED,
keep-original (the stale registered slot is the original's behaviour).
