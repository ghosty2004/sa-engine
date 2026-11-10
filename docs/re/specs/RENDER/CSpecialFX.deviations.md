# CSpecialFX (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/SpecialFX.cpp` (pin 20d8d2a).

## 0x721660 CSpecialFX::Render2DFXs  (gr_deviations = 3)
Original: `__cdecl void ()`, plain `ret`.
- Row SpecialFX.cpp:135 (`notsa::format_to_sz(gString, "{}", CTimer::GetFrameCounter() % 64)`). Original bytes: `sprintf(gString 0xB71670,
  "%d" [0x859508], CTimer::m_FrameCounter & 0x3F)` (CRT 0x821BB5, 0x7216E4-0x7216F8), then AsciiToGxtChar(gString, gGxtString 0xC1B100).
  Verdict: REFUTED (same text for a non-negative value 0..63). Class: harmless.
- Rows SpecialFX.cpp:212 and :213 (`x = GetRandomNumberInRange<uint32>(RsGlobal.maximumWidth)`, `y = ...(maximumHeight)`). Original bytes
  (200 iterations, counter esi = 0xC8, 0x721B74-0x721BF9): `x = rand() % RsGlobal.maximumWidth` (CRT rand 0x821B1E, `cdq; idiv dword [0xC17044]`,
  remainder), then `y = rand() % RsGlobal.maximumHeight` (`idiv [0xC17048]`); rect `{left = (float)x, top = (float)(y + 2),
  right = (float)(x + 20), bottom = (float)y}` (memory order at the CRect: x, y+2, x+20, y; gr's `CRect{x, y, x+20, y+2}` ctor
  (left, bottom, right, top) gives the same layout); `CSprite2d::DrawRect(rect, CRGBA(255,255,255,64))` 0x727B60.
  Verdict: CONFIRMED - the original uses plain `rand() % max` (signed idiv), not CGeneral::GetRandomNumberInRange (and not the gr helper's
  float lerp formula; see docs/re/cat14-gr-deviations.md finding 1). Class: keep-original.
