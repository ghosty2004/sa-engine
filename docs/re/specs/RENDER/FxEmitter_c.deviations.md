# FxEmitter_c (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Fx/FxEmitter.cpp`. EmissionInfo_t (edi = arg1): +0x04 m_fRadius, +0x08/0x0C/0x10
m_SizeMin, +0x14/0x18/0x1C m_SizeMax, +0x20 m_Speed, +0x24 m_fSpeedBias, +0x28 m_Direction (x,y,z), +0x34 m_fAngleMin,
+0x38 m_fAngleMax, +0x3C m_fLife, +0x40 m_fLifeBias, +0x44 m_Pos, +0x50 m_fRotationMinAngle, +0x54 m_fRotationMaxAngle.
FxEmitterPrt_c (esi): +0x08 total life, +0x0C current life, +0x10 pos, +0x1C velocity, +0x28 system, +0x2C..0x2F mult
colour, +0x30 mult size, +0x31 mult rot, +0x32..0x34 rand r/g/b, +0x35 brightness, +0x36 rotZ, +0x37 local, +0x38 rotation.

## 0x4A2580 FxEmitter_c::CreateParticle  (gr_deviations = 14)
Original convention: `__thiscall` (this = ECX), 7 stack args, `ret 0x1C`; entry detours through 0x406067 (`mov ecx,
g_fxMan 0xA9AE80; jmp 0x4A258E`) before `FxManager_c::GetParticle(0)` 0x4A93C0 - same behaviour.

The original never calls a CGeneral random helper here. Every random value is one of two inlined forms, each one
`rand()` (0x821B1E) followed by `cdq; idiv 10000 (0x2710)` (signed remainder, rand >= 0 so `r = rand() % 10000`):
- U = `(float)r * 9.9999997e-05f [0x858FC4]` (range [0, 0.9999]);
- S = `(float)r * 0.00019999999f [0x85A7D0] - 1.0f [0x858624]` (range [-1, 0.9998]).
gr's `GetRandomNumberInRange(0.0f, 1.0f)` / `(0.0f, 2.0f) - 1` / `(0, 256)` use `rand()/32767` lerps (cat14 finding 1),
so every row below is a different RNG mapping.

| gr row | gr | original (addresses) |
|---|---|---|
| :204-206 m_RandR/G/B | `GetRandomNumberInRange(0, 256)` (int, 0..255) | `ftol(U * 255.0f [0x859AAC])` -> bytes +0x32/+0x33/+0x34 (0x4A2675-0x4A26ED), range 0..254 |
| :210 m_CurrentRotation | `rnd01 * (max - min) + min` | `U * (+0x54 - +0x50) + +0x50` -> +0x38 (0x4A2702-0x4A2735; note the multiply is `U * (max-min)`, U loaded first) |
| :219-221 box offset x/y/z | `rnd01 * (SizeMax - SizeMin) + SizeMin` | same shape with U, x then y then z (0x4A283C-0x4A28B3); x, y stored f32, z stays in st0 |
| :225-227 sphere dir x/y/z | `rnd(0,2) - 1` | S, x, y, z order (0x4A2759-0x4A27C4), each stored f32 |
| :235 radius | `invDist * rnd01 * m_fRadius` | `inv = 1.0 (f64 0x85A310) / sqrt((z*z + y*y) + x*x)` (0x4A27D1-0x4A27F4); if `m_fRadius < 0` (0x4A27CB, `test ah,5; jnp`) `k = inv * m_fRadius` (no rand call), else `k = (U * inv) * m_fRadius` (U then `* inv` (stored f32 at +0x4C), 0x4A27FC-0x4A281F); NaN radius takes the rand path |
| :246 randomAngle | `rnd(0, TWO_PI)` | `U * 6.2831802f [0x85A7C0]` stored f32 (0x4A2931-0x4A2952) |
| :250 angle between min/max | `lerp(minRad, maxRad, rnd01)` | `U * (max*k - min*k) + min*k`, k = 0.017453279f [0x85A7BC] (0x4A2956-0x4A298D) |
| :269 speed | `(rnd(0,2) - 1) * SpeedBias + Speed` | `S * +0x24 + +0x20` (0x4A2A8C-0x4A2AB3) |

- Verdict: CONFIRMED for all 14 rows. Class: keep-original (use `rand() % 10000` with the two scale constants; the
  rand() call order is: life, R, G, B, rotation, [box x, y, z | sphere x, y, z, (radius if m_fRadius >= 0)],
  [randomAngle, minmax angle, speed when no velocity override]).

Found in passing (CONFIRMED, keep-original):
- Life (0x4A25C7-0x4A2600): `((float)(rand()%10000) * 0.00019999999f - 1.0f) * LifeBias + Life) * mult.m_fLife (+0x18)`;
  gr divides by 5000.0f (different rounding).
- Box/sphere choice (0x4A2743-0x4A2753): `fabs(m_fRadius)` compared with f64 0.0010000000474974513 [0x85A7C8]
  (= (double)0.001f); box path when `|r| < 0.001` or NaN.
- Velocity direction (0x4A2991-0x4A2A0F) uses the 256-entry sine table 0xBB3E00 (CMaths): `sin(a) = tab[ftol(a *
  40.743664f [0x85A778]) & 0xFF]`, `cos(a) = tab[ftol(a*40.743664f + 64.0f [0x859A44]) & 0xFF]`.
  `dir = (cos(rnd) * sin(mm), cos(mm), sin(rnd) * sin(mm))`. Direction source (0x4A2A13-0x4A2A21): `fcomp
  m_Direction.x, 10.0f [0x85862C]; test ah,0x41; jne` -> when `x < 10`, `x == 10` or NaN the normalised m_Direction
  (RwV3dNormalize 0x7ED9B0) is used; only `x > 10` uses the particle position. Matches gr (`x <= 10`) except for NaN
  (original: direction, gr: position).
