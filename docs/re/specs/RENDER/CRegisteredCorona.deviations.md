# CRegisteredCorona (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/RegisteredCorona.cpp` (pin 20d8d2a). Fields used: +8 m_vPosn.z, +0xC m_dwId, +0x28 m_fFadeSpeed,
+0x2F m_Color.a, +0x30 m_FadedIntensity (u8), +0x31 m_bRegisteredThisFrame, +0x34 flags (bit0 m_bCheckObstacles, bit1 m_bOffScreen),
+0x35 m_bJustCreated, +0x36 flags (bit1 m_bOnlyFromBelow, bit4 m_bAttached), +0x38 m_pAttachedTo.

## 0x6FABF0 CRegisteredCorona::Update  (gr_deviations = 1)
Original: `__thiscall void ()`, plain `ret`.
Row RegisteredCorona.cpp:31 (`m_FadedIntensity = (uint8)notsa::step_to((float)m_FadedIntensity, (float)m_Color.a, fadeStep)`).
- Original bytes (0x6FACC1-0x6FAD4B): `a = m_Color.a`, `f = m_FadedIntensity`, unsigned byte compare `cmp bl, al` (0x6FACC8):
  - `a > f`: `t = f + ms_fTimeStep * m_fFadeSpeed` (x87: `ts * fade` then `fiadd f`, stored f32, 0x6FACDA-0x6FACEB);
    result = `a < t ? a : t` (`test ah,5; jnp`, 0x6FACEF-0x6FACFC).
  - `a < f`: `t = f - ts * fade` (stored f32); result = `a > t ? a : t` (`test ah,0x41; je`, 0x6FAD29-0x6FAD36).
  - then `m_FadedIntensity = (u8)_ftol(result)` (0x6FAD3A) and, if `CCoronas::bChangeBrightnessImmediately` (dword 0xC3E034) != 0,
    `m_FadedIntensity = a` (0x6FAD4B).
  - `a == f`: both stores are skipped (0x6FAD02 `jae 0x6FAD4E`), which leaves the same value.
- Verdict: REFUTED - gr's step_to (`x == to ? to : x < to ? min(to, x + step) : max(to, x - step)`, extensions/utility.hpp:444) gives the
  same results (tie and equality cases produce the same value; no NaN is possible from u8 inputs and finite steps). Class: harmless.
  The other paths match gr too: off-screen / only-from-below / blocked: `_ftol(max(0, f - ts * fade))` (0x6FAD8C-0x6FADBF, 0.0 chosen when
  `!(0.0 < v)` except NaN); `m_dwId == 2`: `LightsMult = max(0.6f, LightsMult - ts * 0.06f)` ([0x859934] = 0x3D75C28F, [0x858CC8] = 0.6f).
