# CTaskComplexGoToPointAndStandStill: gta-reversed deviation notes (AI, group I2)
Layout: +0xC m_moveState, +0x10 m_vecTargetPoint, +0x1C m_fRadius, +0x20 m_fMoveStateRadius, +0x24 flags (bit0 mustOvershoot,
bit1 stopExactly, bit2 newTarget, bit3 trackingEntity, bit4 wasSuccessful). Vtable 0x86FDA8.

## 0x668120 CTaskComplexGoToPointAndStandStill::Constructor  (gr_deviations = 1)
Row cpp:32 (`/// NOTSA ARGS ///`: extra bTrackingEntity parameter).
- Original bytes (`__thiscall this (moveState, const CVector* target, f32 radius, f32 moveStateRadius, bool overshoot, bool stopExactly)`,
  `ret 0x18`): `CTaskComplex()`; vtbl = 0x86FDA8; m_moveState = arg1; `flags = (flags & 0xE4) | (stopExactly & 1) << 1 | (overshoot & 1)`
  - the flag byte is read from uninitialised pool memory: bits 2, 5, 6, 7 keep their old values, tracking (bit3) and
  wasSuccessful (bit4) are cleared (0x66812B-0x66815B). Inlined GoToPoint: update when `target != m_vecTargetPoint` (0x509760, x/y/z
  `!=`, NaN counts as different) OR `m_fRadius` is NaN (the bytes compare +0x1C with ITSELF, 0x66816A-0x668174 - the new radius is never
  compared, original bug) OR `m_fMoveStateRadius != moveStateRadius` (0x668176-0x668182): then target, radius, moveStateRadius are stored
  and `flags |= 4`. Then if stopExactly: `flags &= ~1`; `lim = moveState > 4 ? 1.0f [0x858624] : 0.5f [0x858B8C]`; if `lim > m_fRadius`
  (strict) `m_fRadius = lim` (NaN radius kept); `m_fMoveStateRadius = 0`.
- Verdict: CONFIRMED (gr has an extra ctor argument; the original always clears bit3). Additional differences found: the
  uninitialised bit 2 (newTarget) and the radius self-compare; gr's std::max replaces a NaN radius with lim. Class: keep-original.

## 0x66DBA0 CTaskComplexGoToPointAndStandStill::CreateNextSubTask  (gr_deviations = 2)
- Original bytes: `t = m_pSubTask->GetTaskType()`; 0xCB -> `CreateSubTask(0x516, ped)`; 0x2C0 -> ped bInVehicle ? CreateSubTask(0x516) :
  CreateSubTask(0x384); 0x384 -> `flags bit4 = (subtask->+0x20 bit 2)` (shl 2 / xor / and 0x10 / xor) then CreateSubTask(0xCB); any
  other type -> return 0 (0x66DC35).
- Row cpp:67 (notsa::cast ... gotoPointFlags.m_b03): Verdict REFUTED (same bit: goto-point flags byte +0x20 bit 2). Class: harmless.
- Row cpp:71 (NOTSA_UNREACHABLE default): Verdict CONFIRMED (original returns nullptr). Class: keep-original.
