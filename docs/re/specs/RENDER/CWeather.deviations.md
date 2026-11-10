# CWeather (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Weather.cpp` (pin 20d8d2a). `rand` 0x821B1E, `_ftol2` 0x821B40 (truncation), float RNG
scale 0x858C7C = 3.0518509e-05 (1/32767), original int helper 0x407180, float helper 0x41BD90 (see CBirds.deviations.md).
Globals: Rain 0xC81324, Sandstorm 0xC812F4, Wind 0xC812F0, WindDir 0xC813E0, InTunnelness 0xC81334, s_RainHazeAlpha 0xC81410.
TheCamera 0xB6F028 (+0x14 matrix 0xB6F03C, placement pos 0xB6F02C); 0xB6F9AC/0xB6F9B0/0xB6F9B4 = TheCamera+0x984..0x98C, the vector
gr reads as `m_mCameraMatrix.GetForward()` (the offset is not pinned by a gr VALIDATE_OFFSET).

## 0x72A820 CWeather::AddSandStormParticles  (gr_deviations = 3)
Rows :180 / :181 / :182 (`position.x/y += GetRandomNumberInRange(0, 40) - 20`, `z += GetRandomNumberInRange(0, 7) - 2`).
- Original bytes: P = camera pos; `P.x = fwd.x (0xB6F9AC) * 10.0f + P.x`, `P.y = fwd.y * 10.0f + P.y` (0x72A861-0x72A89F);
  `P.x = ((float)rand() * 3.0518509e-05f * 40.0f + P.x) - 20.0f` (0x72A8AB-0x72A8CE, constants 0x858A10 = 40.0, 0x858BA4 = 20.0),
  same for y (0x72A8D2-0x72A8F5), `P.z = (rand * 3.0518509e-05f * 7.0f + P.z) - 2.0f` (0x72A8F9-0x72A92D, 0x858F48 = 7.0,
  0x858CA0 = 2.0). Velocity `WindDir * 25.0f` (0x858FE8). `g_fx.m_Sand2 (0xA9AE1C)->AddParticle(&P, &vel, 0.0f, &mult, -1.0f, 1.2f,
  0.6f, 0)` 0x4AA440 with FxPrtMult_c(0.67, 0.65, 0.55, 0.25, 1.0, 0, 0.2) (0x72A823-0x72A847).
- Verdict (all three): CONFIRMED - gr's float RNG helper differs, and the original adds the random term to the coordinate BEFORE
  subtracting the bias (`(r + P) - 20`), gr computes `P + (r - 20)`. Class: keep-original.

## 0x72A9A0 CWeather::AddRain  (gr_deviations = 2)
- Row :130 (`radius = GetRandomNumberInRange(0.0f, maxRadius)`). Original 0x72ABA9-0x72ABC4: `0x41BD90(0.0f, maxRadius)` with
  `maxRadius = max(Rain * 10.0f, 40.0f) * 0.5f` (0x72AB22-0x72AB6F; keeps Rain*10 when 40 <= it or NaN). gr's float helper rounds
  differently. CONFIRMED, keep-original. (`numSpots = ftol(Rain * 5.0f)`, `perSpot = 15 - ftol(Rain * -2.0f)` match gr.)
- Row :158 (`s_RainHazeAlpha = clamp(step_to(a, Rain * 0.2f, 0.0025f) * MULT, 0, 1)`). Original 0x72AD6E-0x72ADE4:
  `t = Rain * 0.2f [0x858CC4]`; if `a < t`: `a += 0.0025f [0x86C340]` (no snap to t); then if `a > t` (re-tested with the new a):
  `a -= 0.0025f`; `a = max(a, 0)` (NaN kept); `a *= [0x8D5FF0]` (.data, 1.0f); if `1.0f < a`: a = 1.0f; store.
  gr's step_to snaps to t (`min(t, a + step)` / `max(t, a - step)`), so near the target gr settles exactly on t while the original
  steps up and immediately back down (a stays within one step below t). CONFIRMED, keep-original.

## 0x72AF70 CWeather::RenderRainStreaks  (gr_deviations = 6)
Per streak s (EBX, 0..31, `cmp ebx, 0x20` 0x72B516):
- Rows :311 / :312 / :313 (offset x, y in [-0.2, 0.2], z in [-0.1, 0.1]) and :317 (`GetRandomNumberInRange(0.1f, 0.5f)` for the end
  point z). Original RNG CALL ORDER: r0 = rand() FIRST (0x72B361, `r0 * 3.0518509e-05f` kept at [esp+0x20], later
  `r0' * 0.4f + 0.1f`, 0x72B493-0x72B49D), then x = `rand * k * 0.4f - 0.2f` (0x72B394), y the same (0x72B3B7), z = `rand * k * 0.2f
  - 0.1f` (0x72B3DA). gr draws x, y, z, then the 0.1..0.5 value. CONFIRMED (different values and a different rand() order), keep-original.
- Row :333 (`streakPosZ[s] -= (int32)GetRandomNumberInRange(0.01f, 0.1f)`). Original 0x72B4BA-0x72B4EF:
  `streakPosZ[s] = ftol((float)streakPosZ[s] - (rand * k * 0.09f [0x85B324] + 0.01f [0x858C58]))` - the subtraction is done in
  float and THEN truncated (positive z decreases by 1 per frame, negative z stays). gr truncates the random value first ((int32) of
  a value < 0.1 = 0), so gr's streaks never move down. CONFIRMED - gr LOGIC ERROR. Class: keep-original.
- Row :334 (`streakStrength[s] = max(0, s - GetRandomNumberInRange(2, 5))`). Original 0x72B4EB-0x72B512: `r = 0x407180(2, 5)`;
  `if ((u8)strength > r) strength -= r else strength = 0` (signed `jg` on the zero-extended byte). Different RNG formula in gr.
  CONFIRMED, keep-original.
- Noted (no difference): the second vertex colour is `0xD2D2E6 | strength << 23` (0x72B45D), which equals gr's alpha
  `strength / 2` because bit 23 is already set in 0xD2.

## 0x72B630 CWeather::UpdateInTunnelness  (gr_deviations = 1)
Row :683 (`InTunnelness = step_to(InTunnelness, target, timeStep * 0.01f)`).
- Original bytes 0x72B7C6-0x72B84D: `target = min(dist * 0.01f [0x858C58], 1.0f)` (MULTIPLY by 0.01f; gr divides by 100.0f -
  different rounding); `d = target - InTunnelness`; `step = CTimer::ms_fTimeStep * 0.01f` (stored f32); if `|d| < step`:
  InTunnelness = target; else if `d < 0` (fcomp with 0.0, `test ah,0x41; jne` also taken for d == 0 or NaN): InTunnelness -= step;
  else InTunnelness += step.
- Verdict: CONFIRMED - gr's step_to snaps when `x + step >= to` (and also when |d| == step), the original adds/subtracts the step when
  |d| == step and only snaps when |d| < step; plus the 0.01f vs /100 difference. Low-bit differences only. Class: keep-original.

## 0x72B850 CWeather::Update  (gr_deviations = 6)
- Rows :390 / :393 / :402 (`notsa::contains` weather-type sets; deviates=no). The original type tests were not re-traced
  instruction by instruction here. UNCLEAR (low risk).
- Row :487 (`Rain = step_to(Rain, intensity * lerp, step)`). Original 0x72BC15-0x72BC92: `intensity = (float)((timeMs >> 13) & 3) *
  0.1f + 0.7f` (0x72BC15-0x72BC35), `target = intensity * w` (stored f32), `step = ms_fTimeStep * 0.005f [0x858B4C]`;
  `d = target - Rain`; `|d| < step` -> Rain = target; else `d > 0` -> Rain += step, else (d <= 0 or NaN) Rain -= step.
  CONFIRMED (same edge-rounding difference as UpdateInTunnelness), keep-original.
- Row :494 (Sandstorm, 0x72BC98-0x72BD0B): same scheme with the sandstorm weather (type 0x13) weight. CONFIRMED, keep-original.
- Row :549 (rbf GenericCrashing: `(!FindPlayerPed()) || area == 0`). Original 0x72C1C8-0x72C1EC: `if (CGame::currArea == 0)`
  then `FindPlayerPed(-1)` 0x56E210 and `byte [ped + 0x2F]` (m_nAreaCode) read WITHOUT a null check, then CamNoRain 0x72DDB0 /
  PlayerNoRain 0x72DDC0. CONFIRMED, keep-original (null player ped crashes in the original).
