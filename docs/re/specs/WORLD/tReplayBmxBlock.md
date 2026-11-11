# tReplayBmxBlock (WORLD) - RE-SPEC-FINAL group A

## 0x45BFF0 tReplayBmxBlock  (catalogue: specified/REA, port level 19)
- Convention: `__thiscall void (block* this, * bike [esp+4], float interpolation [esp+8])`, `ret 8`. Caller: CReplay playback (1 site).
- Behaviour: `tReplayVehicleBlock::ExtractVehicleUpdateData(this, bike, interpolation)` 0x45BA90; `s = (float)(i8)this->+0x35 * [0x858B38]=0.02f`
  (fild of the sign-extended byte, fmul, fstp to an f32 local) then `bike->GetRideAnimData()` (vtable +0xBC, slot 47) `->+4 = s`
  (BarSteerAngle); `l = (float)(i8)this->+0x34 * 0.02f` then a second GetRideAnimData() call `->+8 = l` (LeanAngle);
  `bike->+0x5C8 = 0` (m_bLeanMatrixCalculated); `CBike::CalculateLeanMatrix(bike)` 0x6B7150.
- gta-reversed: ReplayBlockType.cpp:ExtractBmxUpdateData:CBmx:200 uses `(float)x / 50.0f`; the original multiplies by the f32 constant 0.02f (0x3CA3D70A, not exactly
  1/50): results can differ in the last bit for some inputs - port the multiply. Otherwise identical (same order). Open questions: none.
