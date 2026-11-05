# CCarEnterExit: gta-reversed deviation notes (VEH, writer group E)

Source: `re/gta-reversed/source/game_sa/CarEnterExit.cpp` (pin 20d8d2a).

## 0x64E550 CCarEnterExit::ComputeDoorFlag  (gr_deviations = 2)
Rows CarEnterExit.cpp:110 and :119 (`default: NOTSA_UNREACHABLE(); // Originally return 0`).
- Original bytes: `eax = 0` first (0x64E554). Tandem branch taken when `bSettingFlags && (m_nVehicleType == 9 ||
  (m_pHandlingData->m_nModelFlags & 0x200))` (0x64E556-0x64E579). Tandem table (0x64E5D4/0x64E5E0, index doorId-8):
  8, 10, 18 -> 5; 9, 11 -> 10; doorId 12..17 and anything outside 8..18 -> 0 (jump to `ret` with EAX = 0, 0x64E5D2).
  Normal table (0x64E5EC/0x64E600): 8 -> 4, 9 -> 8, 10 -> 1, 11 -> 2, 18 -> 1, others -> 0.
- Verdict :110: CONFIRMED (gr asserts / UB for other door ids; original returns 0).
- Verdict :119: CONFIRMED (same, normal branch).
- Class: keep-original (return 0 for any other door id).

## 0x64E610 CCarEnterExit::ComputeOppositeDoorFlag  (gr_deviations = 2)
Rows CarEnterExit.cpp:133 and :142 (`default: NOTSA_UNREACHABLE(); // Originally return 0`).
- Original bytes: same structure as 0x64E550 (`eax = 0` at 0x64E614; same tandem test 0x64E616-0x64E639).
  Tandem table (0x64E694/0x64E6A0): 8, 10, 18 -> 5; 9, 11 -> 10; others -> 0. Normal table (0x64E6AC/0x64E6C0):
  8 -> 1, 9 -> 2, 10 -> 4, 11 -> 8, 18 -> 4, others -> 0.
- Verdict :133: CONFIRMED. Verdict :142: CONFIRMED.
- Class: keep-original.
