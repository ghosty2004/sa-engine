# CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset> (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x661DC0 CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::Constructor  (gr_deviations = 1)
- SeekEntity/TaskComplexSeekEntity.h:85 `T_PosCalc seekPosCalculator = {} // NOTSA`:
  Original: `__thiscall this (CEntity* entity, int32 seekInterval, int32 scanInterval, float maxEntityDist2D, float moveStateRadius,
  float followNodeThresholdHeightChange, bool bPlayTiredAnim, bool bFaceEntityWhenDone)`, `ret 0x20` (8 args). Stores (0x661DE3-0x661E6D): +0xC entity, +0x10 seekInterval, +0x14 scanInterval, +0x18 maxEntityDist2D,
  +0x1C moveStateRadius, +0x20 = 0, +0x24 = followNodeThresholdHeightChange), `vtbl = 0x86FBB4`, timers `+0x28/+0x2C = 0, +0x30/+0x31 = 0,
  +0x34/+0x38 = 0, +0x3C/+0x3D = 0`; the embedded calculator is DEFAULT-constructed: `+0x40 = 0x86F8F8` (calculator vtable), offset
  `+0x44 = +0x48 = +0x4C = 0`; flags byte `+0x54 = (old & 0xF0) | ((bFaceEntityWhenDone & 1) << 1) | (bPlayTiredAnim & 1)`; `+0x50 = 6`
  (move state); `if (entity) RegisterReference(entity, this+0xC)`.
- Verdict: REFUTED (the original has no calculator argument and default-constructs it with a zero offset; gr's default `{}` is equivalent provided
  its CVector value-initialises to zero). Class: harmless. Note: `+0x20` is zeroed by the original between the args (0x661E12).
