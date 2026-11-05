# CPickups: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x4552d0 CPickups::PickedUpOyster  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Pickups.cpp:448` (original-comment): `FindPlayerInfo().m_nMoney += 100; // originally rewarded to the player 1.`
- gr `Pickups.cpp:454` (original-comment): `FindPlayerInfo().m_nMoney += 100'000;` on OY_ALL.
  - Original bytes: both rewards write the fixed address 0xb7ce50 = `CWorld::Players[0].m_nMoney` (Players 0xb7cd98 + 0xb8): `+= 0x64` (0x4552f3-0x455301) and `+= 0x186a0` (0x45533b-0x455348). gr's `FindPlayerInfo()` uses `Players[PlayerInFocus]`.
  - Verdict (both): CONFIRMED. The original always pays player 0 (differs in two-player mode when PlayerInFocus = 1).
  - Class: keep-original.
- Rest (matches gr): frontend event 0xd with (0, 1.0f); `IncrementStat(0xf3, 1.0f)`; compare `GetStatValue(0xf4) == GetStatValue(0xf3)` (ordered equal, `test ah,0x44; jp`); all: `TriggerMessage("OY_ALL" 0x859b1c, -1, 5000, -1)`; else `TriggerMessage("OY_ONE" 0x859b14, ftol(collected), 5000, ftol(total))` (`_ftol` truncation; total converted first, 0x45534f-0x455374).

## 0x455390 CPickups::PickedUpHorseShoe  (gr_deviations = 3)
Rows checked: 3 (CONFIRMED 2, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Pickups.cpp:429` (todo): `IncrementStat(STAT_LUCK, 1000.f / GetStatValue(STAT_TOTAL_HORSESHOES)); // TODO: Is this some inlined stuff?`
  - Original bytes: `GetStatValue(0xf2)` then `fdivr 1000.0f [0x858c4c]` (= 1000 / total), `IncrementStat(0xe9, result)` (0x4553b3-0x4553ce). Not an inlined helper; matches gr.
  - Verdict: REFUTED. Class: harmless.
- gr `Pickups.cpp:431` and `:437` (original-comment): `FindPlayerInfo().m_nMoney += 100` / `+= 100'000`.
  - Original bytes: both add to the fixed address 0xb7ce50 (`Players[0].m_nMoney`): `+= 0x64` (0x4553d3-0x4553e1), `+= 0x186a0` (0x45541b-0x455428).
  - Verdict (both): CONFIRMED. The original always pays player 0; gr pays `Players[PlayerInFocus]`.
  - Class: keep-original.
- Rest: messages "HO_ALL" (0x859b2c) / "HO_ONE" (0x859b24) as in PickedUpOyster; stats 0xf1 collected, 0xf2 total.

## 0x456a70 CPickups::PictureTaken  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Pickups.cpp:486` (original-comment): `FindPlayerInfo().m_nMoney += 100'000; // originally rewarded to the player 1.` (after removing the photographed pickup).
  - Original bytes: after the inline pickup removal, `edi = [0xb7ce50]; edi += 0x64; [0xb7ce50] = edi` (0x456c48-0x456c68): +100 to `Players[0].m_nMoney`.
  - Verdict: CONFIRMED, and larger than the comment says: the original pays 100, not 100000, and always to player 0. (The SN_ALL bonus, gr line 495, is `+= 0x186a0` to `Players[PlayerInFocus (0xb7cd74)]` (0x456cbb-0x456cdf), as in gr.)
  - Class: keep-original.
- Rest of the original (for reference): ascending scan of all 620 `aPickUps` (stride 0x20, 0x9788c0..0x97d640) with type byte `+0xa` == 0x14 (snapshot); `d = sqrt(dz*dz + dy*dy + dx*dx)` from the decompressed pickup position (int16 * 0.125) to TheCamera position; candidate if `(90.0f / CCamera::FindCamFOV()) * 20.0f > d` (0x456b1b-0x456b35), then `IsSphereVisible(pos, 0.2f, 0xb6fa74)` or (if byte 0xb6f998) `IsSphereVisible(pos, 0.2f, 0xb6fabc)`, and `d < best` (best starts 999999.875f). Remove inline: `CRadar::ClearBlipForEntity(7, (refIndex << 16) | slot)`, object removed and deleted, `+0x1c = 0`, `+0x1d |= 1`. Then frontend event 0xd, `IncrementStat(0xe7, 1.0f)`, compare with stat 0xe8, "SN_ALL" (0x859b7c) / "SN_ONE" (0x859b74).

## 0x458970 CPickups::CreateSomeMoney  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Pickups.cpp:113` (rng-helper): `coors.x += std::sinf(GetRandomNumberInRange(0.f, TWO_PI)) * 1.5f;`
- gr `Pickups.cpp:114` (rng-helper): `coors.y += std::cosf(GetRandomNumberInRange(0.f, TWO_PI)) * 1.5f;`
  - Original bytes: `a = (float)(rand() & 0xFF) * 0.024543693f [0x859bbc]` (= 2*PI/256, 256 discrete angles), `x += fsin(a) * 1.5f [0x858ce8]` with the x87 `fsin` instruction on the extended register (0x4589b0-0x4589d4); second, independent draw: `y += fcos(a2) * 1.5f` (0x4589d8-0x458a08). Then `z = FindGroundZFor3DCoord(x, y, z, &ok, 0) + 0.5f` (0x5696c0); if ok: `GenerateNewOne((x, y, z), MI_MONEY, 8, perWad + (rand() & 3), 0, 0, 0)` (0x456f20).
  - Verdict (both): CONFIRMED. The original uses the low 8 bits of rand() scaled by 2*PI/256 and x87 fsin/fcos; gr uses a float lerp over [0, 2*PI] and `sinf`/`cosf`.
  - Class: keep-original.
- Rest (matches gr): `wads = min(amount / 20 + 1, 7)` (magic-number signed division), `perWad = amount / wads` (`idiv`); loop `wads` times; coordinates accumulate across iterations.
