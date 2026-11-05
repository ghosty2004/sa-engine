# FxPrtMult_c (VEH) - full specs (writer group E)

Layout (gr `Fx/FxPrtMult.h`, size 0x1C): +0x00 RwRGBAReal colour (r, g, b, a f32), +0x10 m_fSize, +0x14 m_Rot,
+0x18 m_fLife.

## 0x4AB2D0 FxPrtMult_c::SetUp  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (FxPrtMult_c* this = ECX, float r, float g, float b, float a, float size, float rot,
  float life)` ([esp+4]..[esp+0x1C]), `ret 0x1C`.
- Callers: CAutomobile::ProcessHarvester 0x6A9680.
- Behaviour: copies the 7 dwords to +0x00..+0x18 in argument order (integer moves, no FPU). EAX is clobbered with
  `life` bits (no meaningful return).
- gta-reversed: `FxPrtMult.cpp:19`, identical.
