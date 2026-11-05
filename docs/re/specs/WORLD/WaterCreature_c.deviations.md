# WaterCreature_c: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x6e4160 WaterCreature_c::Init  (gr_deviations = 8)
Rows checked: 8 (CONFIRMED 8, REFUTED 0, UNCLEAR 0).
Notation: `t = (float)rand() * (1/32767) [0x858c7c]` (raw `fild` of rand() 0x821b1e, no mask). Every row below is a gr float `GetRandomNumberInRange` (lerp `max*t + min*(1-t)`, cat14 finding 1); the original inlines a different formula, so all are CONFIRMED / keep-original. Draw order is the same as gr.
- `WaterCreature_c.cpp:30` `vecPos->z = GetRandomNumberInRange(fMinZ, fMaxZ)`: original `z (+8 of *vecPos) = (maxZ - minZ) * t + minZ` (`fld [esp+0x6c]; fsub [esp+0x70]; fmul t; fadd [esp+0x70]`, 0x6e4215-0x6e4236).
- `:56` / `:57` direction x / y `(-1, 1)`: original `(t + t) - 1.0f [0x858624]` (0x6e42f9-0x6e4333).
- `:58` direction z `(-0.5, 0.5)`: original `t - 0.5f [0x858b8c]` (0x6e4337-0x6e4350).
- `:59` `fDist = (minDistFromFollowed, maxDistFromFollowed)`: original `(max - min) * t + min` (0x6e4354-0x6e4374), then the offset `dir * dist`.
- `:64` follower speed `(info.m_fMinSpeed, parent->m_fDefaultSpeed)`: original `(parentSpeed - minSpeed) * t + minSpeed` (0x6e4412-0x6e4425, joins the shared tail at 0x6e4476).
- `:70` heading `(-PI, PI)`: original `t * 6.2831855f [0x858cbc] - 3.1415927f [0x858cb8]` (0x6e443d-0x6e445c) stored to +0x2c.
- `:71` leader speed `(m_fMinSpeed, m_fMaxSpeed)`: original `(max - min) * t + min` (0x6e445f-0x6e448e).
- Verdict (all 8): CONFIRMED (rounding differs from gr's lerp). Class: keep-original.

## 0x6e4670 WaterCreature_c::Update  (gr_deviations = 10)
Rows checked: 11 (CONFIRMED 9, REFUTED 2, UNCLEAR 0).
Notation: `rm = rand() & 0xFFFF`, `k8 = 1/32768 [0x858b14]`, `t = (float)rand() * (1/32767) [0x858c7c]` (no mask), `ftol` = 0x821b40, `rand` = 0x821b1e. The original calls its own helpers 0x407180 (int) / 0x41bd90 (float) at some sites; a port calls the ported originals there.
- gr `source/game_sa/WaterCreature_c.cpp:112` (rng-helper): `if (GetRandomNumberInRange(0.0F, 100.0F) < 5.0F)` (follower branch).
  - Original bytes: `if (ftol(rm * k8 * 100.0f) < 5)` (integer compare, 0x6e4756-0x6e477c).
  - Verdict: CONFIRMED formula differs (integer truncation vs float lerp), although the accepted rand() set is the same (rand <= 1638 in both). Class: keep-original (port the original form).
- gr `WaterCreature_c.cpp:113` (rng-helper): `m_fDefaultSpeed = GetRandomNumberInRange(info.m_fMinSpeed, followed->m_fDefaultSpeed)`.
  - Original bytes: `m_fDefaultSpeed (+0x14) = (followed->m_fDefaultSpeed - minSpeed) * t + minSpeed` (`fld followed; fsub min; fmulp; fadd min`, 0x6e4788-0x6e47a9).
  - Verdict: CONFIRMED (lerp rounding). Class: keep-original.
- gr `WaterCreature_c.cpp:138` (rng-helper): `if (GetRandomNumberInRange(0.0F, 1.0F) < info.m_fChanceToRandomizeRotation)`.
  - Original bytes: `if (t < chance)` (ordered, `test ah,5; jp`, 0x6e493e-0x6e495a).
  - Verdict: REFUTED: gr's lerp(0, 1, t) = `1*t + 0*(1-t)` = t exactly, with the same t. Class: harmless.
- gr `WaterCreature_c.cpp:140` (rng-helper): `m_fHeading = GetRandomNumberInRange(-PI, PI)`.
  - Original bytes: `call CGeneral::GetRandomNumberInRange<float>(-3.1415927f, 3.1415927f)` (0x41bd90, 0x6e4966), result to +0x2c.
  - Verdict: CONFIRMED (gr's float helper differs from 0x41bd90). Class: keep-original (call the ported 0x41bd90).
- gr `WaterCreature_c.cpp:148` (rng-helper): `fRand = GetRandomNumberInRange(0.0F, 100.0F); if (fRand < 10.0F || bMove || m_bChangedDir)` inside `if (info.m_Speed > 0.0F)`.
  - Original bytes: `rand()` is called and `rm * k8` computed BEFORE the `m_Speed > 0` test (0x6e4999-0x6e49c0); only then `n = ftol(rm * k8 * 100.0f)`, `(float)n < 10.0f` (0x6e49c6-0x6e49d9).
  - Verdict: CONFIRMED. (a) The original consumes one rand() per update even when `m_Speed <= 0`; gr does not (RNG stream shifts). (b) Integer truncation form (accepted set equal: rand <= 3276). Class: keep-original.
- gr `WaterCreature_c.cpp:188/190/192` (rng-helper): `GetRandomNumberInRange(0, 1000) < 8`, then `(5, 15)` and `(1000, 2500)`.
  - Original bytes: `CGeneral::GetRandomNumberInRange_int32(0 /*esi*/, 1000)` (0x407180 at 0x6e4c5d), `< 8` signed; then `(5, 15)` -> byte +0xe (0x6e4c6e), `m_wSpeedChangeCurTime (+0x10) = 0`, `(1000, 2500)` -> word +0x12 (0x6e4c84).
  - Verdict (all three): CONFIRMED (gr's integer helper vs 0x407180). Class: keep-original.
- gr `WaterCreature_c.cpp:202/204` (rng-helper): `(35, 50)` and `(2000, 4500)` in the swim-away branch.
  - Original bytes: `0x407180(35, 50)` -> +0xe (0x6e4bda), cur time = 0, `0x407180(2000, 4500)` -> +0x12 (0x6e4bf0); entered when `NormaliseAndMag(pos - playerPos) < 5.0f` (0x6e4bc0-0x6e4bd0).
  - Verdict (both): CONFIRMED. Class: keep-original.
- gr `WaterCreature_c.cpp:240` (bug-note): "BUG? Is this remove needed here?" on `CWorld::Remove(m_pObject); CWorld::Add(m_pObject);`.
  - Original bytes: `CWorld::Remove` (0x563280, 0x6e4e8b) then `CWorld::Add` (0x563220, 0x6e4e94), after the matrix/frame updates.
  - Verdict: REFUTED (gr matches). Class: harmless.
