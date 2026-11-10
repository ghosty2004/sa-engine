# CSprite (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Sprite.cpp` (pin 20d8d2a). gr CRect layout: +0 left, +4 top, +8 right, +0xC bottom.

## 0x70E1C0 CSprite::Set4Vertices2D-CRect  (gr_deviations = 2)
Rows Sprite.cpp:260 (`default: NOTSA_UNREACHABLE();` in the per-vertex colour switch) and :269 (`assert(false)` in the unused
overload 0x70E2D0; deviates=no, not this function).
- Original bytes: `__cdecl void (RwIm2DVertex* v [esp+4], const CRect& r [esp+8], const CRGBA& c1 [+0xC], c2 [+0x10], c3 [+0x14],
  c4 [+0x18])`, fully unrolled, no loop or switch (0x70E1C0-0x70E2C8). Vertex stride 0x1C: x +0, y +4, z +8, rhw +0xC, colour +0x10
  (ARGB: `a << 24 | r << 16 | g << 8 | b`), u +0x14, v +0x18. z = CSprite::m_f2DNearScreenZ 0xC4B8D8, rhw = m_fRecipNearClipPlane
  0xC4B8D0 for all four. v0 = (left, bottom, u 0, v 0, c3); v1 = (right, bottom, 1, 0, c4); v2 = (right, top, 1, 1, c2);
  v3 = (left, top, 0, 1, c1). Store order: v0, v1, v2, v3.
- Verdict: REFUTED - gr's loop + switch produces the same vertices and colours (c3 = bottomLeft, c4 = bottomRight, c2 = topRight,
  c1 = topLeft); the default is unreachable in both. Row :269: REFUTED (debug only, other function). Class: harmless.
