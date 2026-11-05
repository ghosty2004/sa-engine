# CStats: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x5d3b40 CStats::Save  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Stats.cpp:667` (notsa): "TODO: NOTSA: CGenericGameStorage::SaveDataToWorkBuffer(StatMessage);" followed by the per-entry `displayed` loop.
  - Original bytes: `IncrementStat(0xb1, GetStatValue(0xb0))` (0x558e40, 0x55c180), `SetStatValue(0xb0, 0.0f)` (0x55a070); Save (0x5d1270) of StatTypesFloat (0x148), StatTypesInt (0x37c), PedsKilledOfThisType (0x80), LastMissionPassedName (8); then `CStats::PopulateFavoriteRadioStationList()` (0x558ec0, at 0x5d3b9e); then FavoriteRadioStationList (0x38), TimesMissionAttempted (0x190); then 128 x `Save(0xb78202 + i*0x10, 1)`. Returns `AL = 1`.
  - Verdict: REFUTED for the flagged row (the per-entry loop matches).
  - Class: harmless.
- Found in passing (keep-original, CONFIRMED): the original calls `CStats::PopulateFavoriteRadioStationList()` (0x558ec0) right before saving FavoriteRadioStationList; gr's Save (Stats.cpp:657-672) does not, so gr saves whatever the list held last.

## 0x5d3bf0 CStats::Load  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Stats.cpp:682` (notsa): "TODO: NOTSA: CGenericGameStorage::LoadDataFromWorkBuffer(StatMessage);" followed by a per-entry loop loading `statMessage.displayed`.
  - Original bytes: Load (0x5d1300) of StatTypesFloat (0xb79380, 0x148 bytes), StatTypesInt (0xb79000, 0x37c), PedsKilledOfThisType (0xb78e90, 0x80), LastMissionPassedName (0xb78a00, 8), FavoriteRadioStationList (0xb78e58, 0x38), TimesMissionAttempted (0xb78cc8, 0x190), then 128 iterations of `Load(0xb78202 + i*0x10, 1)` (StatMessage[i].displayed) (0x5d3c49-0x5d3c62). Returns `AL = 1`.
  - Verdict: REFUTED (gr's loop is the original; the comment only mentions an alternative).
  - Class: harmless.
