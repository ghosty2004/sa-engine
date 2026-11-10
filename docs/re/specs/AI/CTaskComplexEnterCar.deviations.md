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
