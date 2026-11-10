# CCarGenerator (POP) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/CarGenerator.cpp` (pin 20d8d2a). CCarGenerator (0x20): +0 m_nModelId (int16),
+2/+3 colours (int8), +4/+6/+8 m_vecPosn (int16 x8 compressed), +0xA m_nAngle (int8), +0xB m_nAlarmChance, +0xC
m_nDoorLockChance, +0xD flag bits, +0xE/+0x10 min/max delay, +0x14 m_nNextGenTime, +0x18 m_nVehicleHandle (int16),
+0x1A m_nGenerateCount (u16), +0x1C m_nIplId, +0x1D m_bIsUsed. RNG background: docs/re/cat14-gr-deviations.md finding 1.

## 0x6F2E50 CCarGenerator::Setup  (gr_deviations = 1)
Original: `__thiscall void (CCarGenerator* this = ECX, CVector posn [esp+4..+0xC], float angle [+0x10], int32 modelId [+0x14],
int16 color1 [+0x18], int16 color2 [+0x1C], uint8 forceSpawn [+0x20], uint8 alarm [+0x24], uint8 doorLock [+0x28],
uint16 minDelay [+0x2C], uint16 maxDelay [+0x30], uint8 iplId [+0x34], bool ignorePopLimit [+0x38])`, `ret 0x38` (0x6F2F34).
Row CarGenerator.cpp:387 `constexpr float magic = 256.0f / 360.0f; // 0x8722E8 original expression 128.0f / 180.0f`
- Original bytes: `m_nAngle = (int8)_ftol(angle * [0x8722E8])` (0x6F2E88-0x6F2E9B). `0x8722E8 = 0.71111107f` (bits 0x3F360B60).
  The correctly rounded f32 of 256/360 (and of 128/180) is 0x3F360B61, one ulp larger. Position: `x/y/z = (int16)_ftol(c * 8.0f
  [0x859000])` (0x6F2E55-0x6F2E92).
- Verdict: CONFIRMED (gr's constexpr yields 0x3F360B61; the original constant is 0x3F360B60, so `angle*k` can truncate to a
  different byte at exact boundaries). Class: keep-original - use the f32 bit pattern 0x3F360B60.
Other stores (match gr): colours `(int8)c` or 0xFF when the int16 arg is 0xFFFF (0x6F2EAB-0x6F2ECE); +0xD flags =
`(old & 0xE5) | ((forceSpawn & 1) << 1) | ((ignorePopLimit & 1) << 4)` then `& 0xFE` (0x6F2EDF-0x6F2F1B); handle = -1 (0x6F2F0D);
next gen time = now + 1 (0x6F2F22); generate count = 0 (0x6F2F26); iplId (0x6F2F2C); m_bIsUsed = 1 (0x6F2F2F).

## 0x6F34D0 CCarGenerator::DoInternalProcessing  (gr_deviations = 4)
Original: `__thiscall void (CCarGenerator* this = ECX)` (EDI = this in the tail), SEH frame, plain `ret` (0x6F3E5A).
Row CarGenerator.cpp:318 `if (CGeneral::GetRandomNumberInRange(0, 100) < m_nAlarmChance)`
- Original bytes: `CGeneral::GetRandomNumberInRange(0, 100)` real call 0x407180 (cdecl, 0x6F3D7F); `if (r < (uint8)+0xB)` (signed
  int compare, `jge` 0x6F3D8D) `veh+0x45C (int16 alarm state) = -1` (0x6F3D8F).
- Verdict: CONFIRMED (gr inline lerp formula differs from 0x407180). Class: keep-original (call 0x407180).
Row CarGenerator.cpp:321 `if (CGeneral::GetRandomNumberInRange(0, 100) < m_nDoorLockChance)`
- Original bytes: second 0x407180(0, 100) call (0x6F3D9C); `if (r < (uint8)+0xC) veh+0x4F8 (m_nDoorLock) = 2` (CARLOCK_LOCKED,
  0x6F3DAC). Verdict: CONFIRMED. Class: keep-original.
Row CarGenerator.cpp:337/339/341 (generate count comment + `#ifdef FIX_BUGS`)
- Original bytes: `ecx = movzx m_nGenerateCount; cmp ecx, -1; jge skip` (0x6F3E2C-0x6F3E32): signed compare of a value
  0..65535 with -1, always >= so the decrement at 0x6F3E34 never runs.
- Verdict: CONFIRMED for the FIX_BUGS branch (gr default build decrements finite counts); the `#else` text matches the
  original. Class: keep-original (never decrement). The bug-note row (:339) is commentary only (REFUTED as a difference).
Tail (match gr): colours copied when both != 0xFF, else if `(int16)m_nModelId < -1` the vehicle's colours are saved back
(0x6F3DB6-0x6F3DEC); `SetClumpAlpha(veh->m_pRwClump, 0)` 0x732B00; handle = `(index << 8) | flags[index]` with index =
`(veh - pool.objects) / 0xA18` (magic 0x6572EC3, 0x6F3E04-0x6F3E22); `m_nNextGenTime = now + 4` inline (0x6F3E3F; same as gr
CalcNextGen 0x6F2E40).
