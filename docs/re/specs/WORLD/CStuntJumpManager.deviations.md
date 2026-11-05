# CStuntJumpManager: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x49c490 CStuntJumpManager::Update  (gr_deviations = 2)
Rows checked: 3 (CONFIRMED 1, REFUTED 2, UNCLEAR 0).
- gr `source/game_sa/StuntJumpManager.cpp:113` (notsa-helper, deviates=no): `contains({BOAT, PLANE, HELI}, appearance)`. REFUTED: the original calls `CVehicle::GetVehicleAppearance` (0x6d1080) three times, interleaved with the driver test: `== 4` -> return (0x49c7a6), `veh->m_pDriver (+0x460) != player` -> return (0x49c7af), `== 5` -> return, `== 3` -> return (0x49c7c2-0x49c7d5). Same result (pure calls). Class: harmless.
- gr `StuntJumpManager.cpp:121` (bugfix-comment): comment about lordmau5's MTA grace-period fix; gr keeps `if (m_nNumEntitiesCollided > 0) return;`.
  - Original bytes: `byte [veh+0xb9] != 0` -> return (0x49c7db-0x49c7e3).
  - Verdict: REFUTED (comment only, gr keeps the original check). Class: harmless.
- gr `StuntJumpManager.cpp:130` (pool-iteration): `for (auto& j : mp_poolStuntJumps->GetAllValid()) { if (!j.start.IsPointWithin(pos)) continue; ...activate...; break; }`.
  - Original bytes: ASCENDING loop `ebp = 0 .. 0xff` with a hard-coded bound of 256 (`cmp ebp,0x100; jl`, 0x49c8d0-0x49c8d7), skip free slots (`js`), jump pointer from 0x49c3b0. On a hit (`CBoundingBox::IsPointWithin(&j->start, pos)`, 0x49c300): `m_jumpState (0xa9a898) = 1`, `mp_Active (0xa9a88c) = j`, `m_iTimer = 0`, `m_bHitReward = 0`, `if (!j->found (+0x41)) { found = 1; IncrementStat(0x90, 1.0f); }`, `CTimer::ms_fTimeScale = 0.3f`, `TheCamera.SetCamPositionForFixedMode(&mp_Active->camera (+0x30), &zero)`, `TheCamera.TakeControl(veh, 0xf, 2, 1)`; then the pool pointer is reloaded and the loop CONTINUES (no break).
  - Verdict: CONFIRMED. Order is the same (ascending), but the original does not stop at the first hit: if the vehicle is inside several start boxes, every one of them is activated in turn (each unfound one counts towards STAT 0x90) and the last one stays `mp_Active`. gr stops at the first.
  - Class: keep-original.
