# cBuoyancy: gta-reversed deviation notes (PHYS)

## 0x6c34e0 cBuoyancy::AddSplashParticles  (gr_deviations = 1)
- Buoyancy.cpp:295 `float fRand = CGeneral::GetRandomNumberInRange(0.0F, 0.5F);`
  - Original bytes: 0x6c38af `call rand` 0x821b1e; `fild`; `fmul [0x858c7c]` (3.0518509e-5f = 1/32767);
    `fmul [0x858b8c]` (0.5f); the result scales the splash direction which is added to the point
    (0x6c38e1..0x6c392f) before `FxSystem_c::AddParticle` (g_fx.m_WaterSplash at 0xa9ae38).
  - Verdict: REFUTED. With `min = 0` gr's `0.5f*t + 0.0f*(1-t)` is bit-identical to the original `t*0.5f`
    (`0*(1-t)` is +0 and adding +0 is exact), and `t` is computed the same way (`rand() * (1/32767.0f)`).
  - Class: harmless (a port may still call the shared helper only if it reproduces `t*0.5f` exactly; the
    original inlines the formula, it does not call 0x41bd90).
