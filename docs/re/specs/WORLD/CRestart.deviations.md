# CRestart: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x460a50 CRestart::FindClosestPoliceRestartPoint  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Restart.cpp:177` (notsa-unreachable): `NOTSA_UNREACHABLE(); /* Variables would've been left uninitialized */` when no restart point was chosen.
  - Original bytes: `ebx = -1` (0x460a59); after the loop `test bx,bx; jl 0x460c3e` (0x460c0b) jumps straight to the epilogue: `*outPos` and `*outAngle` are not written and the function returns normally.
  - Verdict: CONFIRMED. The original returns with the outputs untouched (callers keep their previous contents).
  - Class: keep-original.
  - Rest of the loop matches gr: `u16 i < NumberOfPoliceRestarts (0xa43268)`; skip if `CStats::GetStatValue(0xb5) < (float)PoliceRestartWhenToUse[i]` (ordered; NaN does not skip, 0x460b78-0x460b84); `d = sqrt(dz*dz + dy*dy + dx*dx)` with `point - restart` (x87: z squared first, then y, then x, 0x460b88-0x460bbb), stored f32; if `pointLevel != 0` and `GetLevelFromPosition(restart) != pointLevel` then `d *= 6.0f [0x858b44]`; accept if `d < best` (best starts 99999.8984375f = 0x47c34ff3). Note the override paths at 0x460a50-0x460b35 (bOverrideRestart, bOverrideRespawnBasePointForMission replaces the search point, ExtraPoliceStationRestart radius test `sqrt(dy*dy+dx*dx) < radius` only when radius > 0).
