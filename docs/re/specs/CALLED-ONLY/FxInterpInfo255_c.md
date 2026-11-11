# FxInterpInfo255_c (CALLED-ONLY) — RE-SPEC-REST

Layout as FxInterpInfo_c (see CALLED-ONLY/FxInterpInfoFloat_c.md); keys are **signed i16** arrays (gr Load 0x5C1D30 stores `value * 128`).

## 0x4A8B80 FxInterpInfo255_c::GetVal(float* out, float time)  (catalogue: specified/REA, port level 0)
- `__thiscall`, `out` [esp+4], f32 `time` [esp+8], `ret 8`. Caller: FxInfoSpriteRect_c::GetValue 0x4A6140.
- Byte-for-byte the same code as FxInterpInfoU255_c::GetVal 0x4A8800 (offset +0x380) except:
  - key loads are `movsx` (signed i16) instead of `movzx` (0x4A8BA6, 0x4A8C5F, 0x4A8CC6, 0x4A8CCE);
  - keys are scaled by `[0x858B88]=0.0078125f` (1/128) instead of 1/256 (0x4A8BB2, 0x4A8C6D, 0x4A8CDE, 0x4A8CE8).
  - Times: `movzx` u16 × `[0x859AA0]=0.00390625f`.
- Behaviour: steps 1-5 of 0x4A8800 with key decode `(float)(i16)key * 0.0078125f`.
- gta-reversed: `plugin::CallMethod<0x4A8B80>` only (FxInterpInfo255.cpp).
