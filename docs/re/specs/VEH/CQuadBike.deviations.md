# CQuadBike: gta-reversed deviation notes (VEH)

Source: `re/gta-reversed/source/game_sa/Entity/Vehicle/QuadBike.cpp` (pin 20d8d2a).

## 0x6CEAD0 CQuadBike::PreRender  (gr_deviations = 1)
Convention: `__thiscall void (CQuadBike* this)`, plain `ret`. SEH frame for one local CMatrix.
Row QuadBike.cpp:89 ("Original code saves position of each matrix ... then restores it").
- Original bytes: `CAutomobile::PreRender()`; `GetWheelPosn(1, p, false)`;
  `SetTransmissionRotation(m_aCarNodes[+0x688], m_wheelPosition[1], m_wheelPosition[3], p /*by value*/, false)`;
  `GetWheelPosn(0, p, false)`. For each of +0x684, +0x680, +0x68C (if non-null): `mat.Attach(&frame->modelling,
  false)`; save `mat.pos`; then
  - +0x684: `mat.SetRotateY(f32(atan2(m_wheelPosition[0] - p.z, fabs(p.x))))` (fpatan, 0x6CEB87-0x6CEBBE);
  - +0x680: `mat.SetRotateY(f32(-atan2(m_wheelPosition[2] - p.z, fabs(p.x))))` (0x6CEC09-0x6CEC3D);
  - +0x68C: `mat.SetRotateZ(f32(QUAD_HBSTEER_ANIM_MULT [0x8D3454, data, -0.4f] * [+0x9A0]))` (0x6CEC88-0x6CECB4);
  then `mat.pos.x/y/z += saved.x/y/z` (fld/fadd/fstp each) and `mat.UpdateRW()`.
  SetRotateY/SetRotateZ (0x59B0A0 / 0x59B0E0) write a zero translation, so the result is `0.0f + saved`.
- gr: SetRotateYOnly / SetRotateZOnly (translation untouched).
- Verdict: REFUTED (same values; the only difference is that a saved -0.0f comes back as +0.0f in the original).
- Class: harmless (a port that wants bit parity can keep the save/add sequence).
