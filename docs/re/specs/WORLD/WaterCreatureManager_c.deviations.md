# WaterCreatureManager_c: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x6e4040 WaterCreatureManager_c::GetRandomWaterCreatureId  (gr_deviations = 3)
Rows checked: 3 (CONFIRMED 3, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/WaterCreatureManager_c.cpp:47` (rng-helper): `auto randNum = CGeneral::GetRandomNumberInRange(0, 100);`
- gr `WaterCreatureManager_c.cpp:49` (rng-helper): `return CGeneral::GetRandomNumberInRange(0, 3);` (randNum < 80)
- gr `WaterCreatureManager_c.cpp:53` (rng-helper): `return CGeneral::GetRandomNumberInRange(3, 5);` (randNum < 90)
  - Original bytes (inline RNG, `rand` = 0x821b1e, `_ftol` = 0x821b40, k = 1/32768 = 3.0517578125e-05 [0x858b14]):
    - `n = ftol((float)(rand() & 0xFFFF) * k * 100.0 [0x858628])` (0x6e4041-0x6e405f).
    - `n < 80` (signed): return `ftol((rand() & 0xFFFF) * k * 3.0 [0x858b3c])` (tail `jmp _ftol`, 0x6e4069-0x6e408a) -> 0..2.
    - `n < 90`: return `3 - ftol((rand() & 0xFFFF) * k * -2.0 [0x858b18])` (0x6e4094-0x6e40be) -> 3..4 (ftol truncates toward zero).
    - else return `5 + (n >= 97)` (0x6e40c2-0x6e40cd): 5 TURTLE, 6 DOLPHIN.
    x87: `fild` of the masked int, `fmul` f32 k, `fmul` f32 range, `_ftol` truncation.
  - Verdict (all three rows): CONFIRMED. gr's integer helper maps rand() differently (cat14 finding 1); the value ranges are the same but the same rand() sequence gives different ids. The jellyfish draw uses `3 - ftol(r * -2)`, not `3 + ftol(r * 2)` (same result set, same mapping since trunc(-x) = -trunc(x)).
  - Class: keep-original.

## 0x6e4510 WaterCreatureManager_c::CanAddWaterCreatureAtPos  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/WaterCreatureManager_c.cpp:85` (original-comment): `const auto& creatureInfo = GetCreatureInfo(id); // Originally not assigned by reference, but copied to stack`.
  - Original bytes: `rep movsd` of 11 dwords (0x2c bytes) from `ms_waterCreatureInfos[id]` (0x8d3698 + id*0x2c) to a stack copy (0x6e4519-0x6e4538), done before the empty-list check.
  - Verdict on the row: REFUTED (copy vs reference reads the same constant data; no observable difference). Class: harmless.
- Found in passing (real difference, keep-original): the distance test. Original (0x6e4564-0x6e458e): `dx = pos.x - objPos.x; dy = pos.y - objPos.y; d2 = dy*dy + dx*dx` (2D only, x87 order: dy squared first, then dx*dx, `faddp`), then `m2 = info.m_fMinDistFromSameCreature (+0xc) * info.m_fMinDistFromSameCreature`; `fcompp` compares m2 with d2; `test ah,0x41; je 0x6e45a1` returns false when `m2 > d2` (i.e. `d2 < m2`, strict). NaN: unordered falls through (not rejected). gr uses the 3D squared distance and `<=` against the UNSQUARED minimum distance. CONFIRMED, keep-original.
  - Other details (match gr): list head at manager +0x1a0c; per node: `next = [node+4]` read first, type byte `[node+0xc]` compared with `id` (movzx, full int compare), followed-creature `[node+0x1c]` must be null, object `[node+8]`, position = matrix(+0x14) ? matrix+0x30 : obj+4. `ret 0x10` (thiscall, id + CVector by value).

## 0x6e4f10 WaterCreatureManager_c::Update  (gr_deviations = 4)
Rows checked: 4 (CONFIRMED 4, REFUTED 0, UNCLEAR 0).
Notation: `t = (float)rand() * (1/32767) [0x858c7c]` (raw rand 0x821b1e).
- gr `source/game_sa/WaterCreatureManager_c.cpp:137/138` (rng-helper): `direction.x/y = GetRandomNumberInRange(-1.0F, 1.0F)`.
  - Original bytes: `dir.x = (t + t) - 1.0f` (0x6e4f73-0x6e4f8e), `dir.y` likewise (0x6e4f92-0x6e4fb9), `dir.z = 0`, `CVector::Normalise(&dir)` (0x59c910).
- gr `:142` (rng-helper): `dist = GetRandomNumberInRange(25.0F, 40.0F)`.
  - Original bytes: `dist = t * 15.0f [0x858b48] + 25.0f [0x858fe8]` (0x6e4fc2-0x6e4fe2); `dir *= dist` component-wise; `pos = camPos + dir` (camera matrix 0xb6f03c +0x30, else 0xb6f02c).
- gr `:155` (rng-helper): `randNumToCreate = GetRandomNumberInRange(info.m_nMinCreated, info.m_nMaxCreated)` (integer).
  - Original bytes: `CGeneral::GetRandomNumberInRange_int32(minCreated (byte +4 of the stack copy of the info), maxCreated (byte +5))` (0x407180, 0x6e50d8); then `n = min(n, pool count)` via 0x4a8ec0 (signed compare, 0x6e50e9).
- Verdict (all 4): CONFIRMED (float rows: inline `(t+t)-1` / `t*15+25` vs gr's lerp rounding; integer row: gr helper vs 0x407180). Class: keep-original.
- Other details (match gr unless noted): runs only if `FindPlayerPed(0)->m_pPlayerData(+0x480)->m_nWaterCoverPerc (+0x8e, byte) > 50` (unsigned); `currTime - m_lastUpdateTime (+0x1a18) > 1000` is a SIGNED compare (`jle`, 0x6e4f4c; gr uses unsigned subtraction: differs only across timer wrap-around); pool count (0x4a8ec0 on +0x1a00) > 0; `!TheCamera.IsSphereVisible(pos, 3.0f)` (0x420d40); `CWaterLevel::GetWaterDepth(pos, &depth, &z, 0)`; `depth > 4.5f [0x863214]`; `GetRandomWaterCreatureId` (0x6e4040); `CanAddWaterCreatureAtPos` (0x6e4510).
