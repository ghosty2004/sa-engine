# CPlaneTrails (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/PlaneTrails.cpp` (pin 20d8d2a). CTimeCycle::m_CurrentColours 0xB7C4A0: m_nSkyBottomRed +0x2A
(0xB7C4CA), Green +0x2C (0xB7C4CC), Blue +0x2E (0xB7C4CE), all u16. CPlaneTrails::aArray 0xC713A8, 3 x 0x100.

## 0x7173A0 CPlaneTrails::Render  (gr_deviations = 1)
Original: `__cdecl void ()`, plain `ret` (0x7174BB).
Row PlaneTrails.cpp:25 ("Originally `/ 256.f`, but that value is impossible for a byte, so `255.f` it is.").
- Original bytes: `v = 1 - Foggyness`, replaced by `1 - CloudCoverage` if `(1 - Cloud) < (1 - Fog)` (0x7173A3-0x7173D6); then replaced by
  `1 - Rain` unless `v < (1 - Rain)` (equal/NaN take `1 - Rain`, 0x7173DA-0x7173FB). `m = max(R, G, B)` of the sky-bottom u16 values
  (unsigned compares, 0x7173FB-0x717432); `mf = (float)m * 0.00390625f` ([0x859AA0] = 0x3B800000 = 1/256, 0x71743D); final = v if
  `mf > v`, else mf (recomputed, 0x717472-0x71747C; NaN takes mf). If `final > 0.0001f` ([0x858FC4], strict) call CPlaneTrail::Render
  0x717180 (thiscall, arg final) on each of the 3 trails.
- Verdict: CONFIRMED - gr scales by 1/255, the original by 1/256 (0x71743D/0x717476). Class: keep-original.
