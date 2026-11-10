# CFallingGlassPane (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/FallingGlassPane.cpp` (pin 20d8d2a). Fields: +0x30/+0x34/+0x38 matrix pos, +0x48 velocity,
+0x64 m_fGroundZ, +0x6D m_bExist, +0x6F m_f6F.

## 0x71AA10 CFallingGlassPane::Update  (gr_deviations = 1)
Row FallingGlassPane.cpp:39 (`g_fx.AddGlass(pos, color, CGeneral::GetRandomNumberInRange(0.02f, 0.06f), 1)` x4).
- Original bytes: when `pos.z < m_fGroundZ` (`fcomp [esi+0x64]; test ah,5; jp`, 0x71AB5E-0x71AB69, NaN -> no hit): m_bExist = 0
  (0x71AB8C), AudioEngine.ReportGlassCollisionEvent(0x7F, &(pos.x, pos.y, m_fGroundZ)) 0x506EE0; if `m_f6F == 0`: colour
  (255, 255, 255, 32) and four unrolled calls (0x71ABB7, 0x71ABF0, 0x71AC29, 0x71AC62) each computing
  `size = (float)rand() * 3.0518509e-05f [0x858C7C] * 0.04f [0x858CEC] + 0.02f [0x858B38]` (fild, fmul, fmul, fadd, stored f32 as the
  stack argument) then `Fx_c::AddGlass(&g_fx 0xA9AE00, pos, colour, size, 1)` 0x49F970.
- Verdict: CONFIRMED - gr's float GetRandomNumberInRange (lerp `max*t + min*(1-t)`) rounds differently from the inlined
  `rand*(1/32767)*(max-min)+min`. Class: keep-original.
