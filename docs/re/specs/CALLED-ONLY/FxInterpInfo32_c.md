# FxInterpInfo32_c (CALLED-ONLY) — RE-SPEC-REST

Layout as FxInterpInfo_c (see CALLED-ONLY/FxInterpInfoFloat_c.md); keys are **signed i16** arrays.

## 0x4A89C0 FxInterpInfo32_c::GetVal(float* out, float time)  (catalogue: specified/REA, port level 0)
- `__thiscall`, `out` [esp+4], f32 `time` [esp+8], `ret 8`. Callers: FxInfoEmSize_c 0x4A4CB0, FxInfoEmSpeed_c 0x4A4D90, FxInfoEmDir_c
  0x4A4E50, FxInfoNoise_c 0x4A5300, FxInfoForce_c 0x4A54E0, FxInfoFriction_c 0x4A55E0 (`GetValue`).
- Byte-for-byte the same code as FxInterpInfoU255_c::GetVal 0x4A8800 (offset +0x1C0) except:
  - key loads are `movsx` (signed i16) instead of `movzx` (0x4A89E6, 0x4A8A9F, 0x4A8B06, 0x4A8B0E);
  - keys are scaled by `[0x858CDC]=0.001f` instead of 1/256 (0x4A89F2, 0x4A8AAD, 0x4A8B1E, 0x4A8B28).
  - Times are still `movzx` u16 × `[0x859AA0]=0.00390625f` (looped period 0x4A8A24, search 0x4A8A6B, t0 0x4A8ADA).
- Behaviour: steps 1-5 of 0x4A8800 with key decode `(float)(i16)key * 0.001f`.
- gta-reversed: `plugin::CallMethod<0x4A89C0>` only (FxInterpInfo32.cpp).
