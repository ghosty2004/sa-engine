# CCheckpoint (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Checkpoint.cpp` (pin 20d8d2a). CCheckpoint: +0 m_Type (u16), +4 m_ID, +8..+0xB m_Colour (r,g,b,a),
+0xC m_PulsePeriod (u16), +0xE m_RotateRate (u16), +0x10 m_Pos, +0x1C m_Fwd, +0x28 m_PulseFraction, +0x2C m_Size, +0x34 m_MultiSize.

## 0x725C00 CCheckpoint::Render  (gr_deviations = 3)
Original: `__thiscall void ()` (ECX = this), plain `ret`. Dispatch: `movzx eax, word [this]` (0x725C08); `type > 8` -> return (0x72602F);
jump table 0x726038 = {0,1,2 -> 0x725C1C (tubes), 3 -> 0x725E66 (TORUS), 4,5,7,8 -> 0x725FE1, 6 -> 0x725F05}.
- Row Checkpoint.cpp:120 ("NOTE: Originally this would underflow"). Original bytes (TORUSTHROUGH, 0x725F05-0x725FDB): for i = 1..3:
  `alpha = (uint8)(m_Colour.a - (uint8)(i << 4))` (byte `sub dl, cl`, 0x725F9E-0x725FA6, wraps); `pos = m_Pos + ((float)i * m_Fwd) * m_MultiSize`
  per component (x87: `f = (float)i`, `f*Fwd.y`, `f*Fwd.z` stored f32, then each times m_MultiSize, then + m_Pos; 0x725F10-0x725F84);
  `C3dMarkers::PlaceMarker(m_ID, 4 /*TORUS*/, &pos, m_Size, r, g, b, alpha, m_PulsePeriod, m_PulseFraction, m_RotateRate, m_Fwd, 0)` 0x725120.
  After i = 3 the code falls into the common torus block 0x725FE1 (pos = m_Pos, alpha = m_Colour.a).
  Verdict: REFUTED - the comment describes the original (byte wrap), and gr's non-FixBugs path `(uint8)a` reproduces it. Class: harmless.
  Found in passing (CONFIRMED, keep-original, rounding-level): gr computes `m_Fwd * ((float)i * m_MultiSize)`; the original multiplies
  `(i * Fwd.c) * m_MultiSize`.
- Row Checkpoint.cpp:122 (`if constexpr (IsFixBugs()) { if (a < 0) return; }`). Verdict: CONFIRMED for FixBugs builds (skips the marker
  instead of wrapping). Class: keep-original.
- Row Checkpoint.cpp:153 (`default: NOTSA_UNREACHABLE`). Original: types > 8 (as u16) return silently (0x725C0F -> 0x72602F); gr breaks for
  NA (0x101) and asserts for other unknown values. Verdict: CONFIRMED only for invalid types (assert vs silent return). Class: harmless
  (port returns).
