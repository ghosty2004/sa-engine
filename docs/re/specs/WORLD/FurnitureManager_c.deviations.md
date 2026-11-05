# FurnitureManager_c: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x5c0280 FurnitureManager_c::LoadFurniture  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 1, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Interior/FurnitureManager_c.cpp:63` (original-comment): "They originally used IDs, and searched for the sub-group each loop, so I'm replacing it with pointers instead"; gr keeps `grp`/`subGrp` pointers and calls `subGrp->AddFurniture(...)`.
  - Original bytes: keeps the group id in EBP (initially -1, 0x5c029e) and the sub-group id in a local. GROUP: `sscanf(l, "%s %s")` then `ebp = GetGroupId(name)` (0x5bfb40). SUBGROUP: `sscanf` 9 fields, `subId = GetSubGroupId(name)` (0x5bfbf0), `FurnitureGroup_c::AddSubGroup(&m_Groups[ebp] (this + ebp*0xc), subId, minW, minD, maxW, maxD, window, tall, steal)` (0x5910e0; the raw ints are passed, the `== 1` conversions gr applies are not visible here). ITEM: `sscanf` 6 fields, `CModelInfo::GetModelInfo(name, &modelIndex)` (0x4c5940); if found `FurnitureGroup_c::AddFurniture(&m_Groups[ebp], subId, modelIndex, id, wealthMin, wealthMax, maxAng)` (0x5c0230, which looks the sub-group up by id); if not found, a debug `sprintf` into a stack buffer (0x821bb5, string 0x86af1c) and nothing else. No checks on sscanf counts, group id -1 or AddSubGroup failure.
  - Verdict: REFUTED for well-formed data (same group/sub-group receives the item). Differences only on malformed data: gr's VERIFYs vs the original's unchecked use (e.g. group id -1 indexes `m_Groups[-1]`).
  - Class: harmless (port the id-based calls to keep malformed-data behaviour).
- gr `FurnitureManager_c.cpp:96` (notsa-unreachable): `NOTSA_UNREACHABLE("Unknown tag {}", tag);`
  - Original bytes: the tag (first `%s`, `sscanf` 0x8220ad) is compared with "GROUP:" (7 bytes incl. NUL, 0x86af78), "SUBGROUP:" (10, 0x86af6c), "ITEM:" (6, 0x86af48) via `repe cmpsb`; no match jumps to the next line (`jne 0x5c0472`, 0x5c03e0). Empty and `#` lines are skipped first (0x5c02c2-0x5c02cc).
  - Verdict: CONFIRMED. Unknown tags are silently ignored in the original.
  - Class: keep-original.
