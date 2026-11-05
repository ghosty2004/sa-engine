# CClumpModelInfo: gta-reversed deviation notes (STREAM)

Source: `re/gta-reversed/source/game_sa/Models/ClumpModelInfo.cpp` (pin 20d8d2a).

## 0x4C4F70 CClumpModelInfo::SetClump  (gr_deviations = 1)
Row ClumpModelInfo.cpp:196 (comment "Average R* dev from 2003").
- Original bytes (0x4C50B0-0x4C50F3): per vertex, RpSkinGetVertexBoneWeights is called again,
  `s = ((w1 + w0) + w2) + w3` (fld [+4], fadd [+0], fadd [+8], fadd [+0xC]), `r = 1.0f / s` (fdivr), then
  each weight `*= r`, stored as f32. The vertex count is re-read from the geometry every iteration.
  Afterwards `hierarchy->flags = 0x3000` (0x4C50FA). Same arithmetic as gr (w0 + w1 is commutative).
- Verdict: REFUTED (comment only).
- Class: harmless.
