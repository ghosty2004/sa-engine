# CTaskComplexSmartFleeEntity (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

## 0x65C430 CTaskComplexSmartFleeEntity::Constructor  (gr_deviations = 1)
- Tasks/TaskTypes/TaskComplexSmartFleeEntity.cpp:33 `// VVV NOTSA ARGS VVV // eMoveState moveState` (default PEDMOVE_SPRINT in the header):
  Original: `__thiscall this (CEntity* fleeFrom, bool scream, float safeDistance, int32 fleeTime, int32 posCheckPeriod, float posChangeTolerance)`,
  `ret 0x18` (6 args). Stores (0x65C452-0x65C497): `+0xC = fleeFrom`, `+0x1C = fleeTime`, `+0x20 = safeDistance`, byte `+0x24 = scream`,
  `+0x28 = posCheckPeriod`, `+0x2C = posChangeTolerance`, `vtbl = 0x86F784`, `+0x30 = 7` (move state hard-coded: PEDMOVE_SPRINT), timer
  `+0x34 = 0, +0x38 = 0, +0x3C = 0, +0x3D = 0`; then `if (fleeFrom) CEntity::RegisterReference(fleeFrom, this+0xC)` 0x571B70.
- Verdict: REFUTED (gr's extra argument defaults to PEDMOVE_SPRINT = 7, the value the original hard-codes; original callers never pass it). Class: harmless.
