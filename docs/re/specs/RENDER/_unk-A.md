# RENDER: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x49ea70 ListItem_c::~ListItem_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall`; tail `jmp 0x4a8dc0` (0x49ea70). Open question: 0x4a8dc0 is unnamed in the catalogue.

## 0x4a11c0 ListItem_c-derived scalar deleting destructor (proposed)  (catalogue: unknown/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `ListItem_c::~ListItem_c 0x49ea70(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a11c0).

## 0x4a18f0 FxEmitterBP_c::~FxEmitterBP_c (proposed)  (catalogue: specified/REA, port level 4)
- `__thiscall void ()`: `[this] = 0x85a788`; tail `jmp 0x4a9d20` FxPrimBP_c::~FxPrimBP_c (base destructor) (0x4a18f0).

## 0x4a2b20 FxEmitterBP_c::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 5)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxEmitterBP_c::~FxEmitterBP_c 0x4a18f0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a2b20).

## 0x4a2b90 FxEmitter_c::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxEmitter_c::~FxEmitter_c(this)` 0x4a2bb0; `if (flags & 1) free(this)` 0x8214bd; EAX = this.

## 0x4a2bb0 FxEmitter_c::~FxEmitter_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall`; `jmp FxPrim_c::~FxPrim_c 0x4a9f40` (no vtable store here).

## 0x4a4950 FxInfoManager_c::~FxInfoManager_c (empty) (proposed)  (catalogue: specified/REA, port level 0)
- Empty: single `ret` (0x4a4950). Port: no-op.

## 0x4a4b40 FxInfoEmRate_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall void (FILESTREAM file [esp+4], i32 version [esp+8] (unused))`, `ret 8`: `ecx = this+8; call [[this+8]+4](file)` (m_InterpInfo.Load, 0x4a4b4b).

## 0x4a4b60 FxInfoEmRate_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void (float currentTime, float mult, float totalTime, float len, bool useConst, EmissionInfo_t* info)`, `ret 0x18`.
- `v10 = currentTime - totalTime` (0x4a4b63/0x4a4b6c, stored f32); `m_InterpInfo(+8).GetVal(values[16], currentTime)` 0x4a8470.
- `if (useConst)`: `info->m_fCount = values[0] * totalTime` (0x4a4b8d/0x4a4b95).
- else if `v10 >= 0.0f [0x858b50]` or NaN (`fcomp; test ah,5; jp`, 0x4a4ba7-0x4a4bb2): `info->m_fCount = GetVal(0, currentTime, totalTime)` 0x4a85c0 (0x4a4bf6).
- else: `info->m_fCount = GetVal(0, len, -v10)` (0x4a4bc8); then `info->m_fCount = GetVal(0, currentTime, v10 + totalTime) + info->m_fCount` (`fadd [edi]` 0x4a4be9).
- gta-reversed: same (Fx/Blueprints/FxInfoEmRate.cpp). `mult` unused.

## 0x4a4c90 FxInfoEmSize_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40: `m_InterpInfo(+8)->Load(file)` via vtable slot 1, `ret 8`.

## 0x4a4cb0 FxInfoEmSize_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void (float t, float mult, float totalTime, float len, bool useConst, EmissionInfo_t* info)`, `ret 0x18`.
- `m_InterpInfo(+8).GetVal(values, t)` 0x4a89c0; `info->m_fRadius(+4) = values[0]` (x87 fld/fstp), `+8 = values[1]`, `+0x14 = values[2]`, `+0xc = values[3]`, `+0x18 = values[4]`, `+0x10 = values[5]`, `+0x1c = values[6]` (integer moves, 0x4a4ccd-0x4a4cf7): SizeMin = (v1,v3,v5), SizeMax = (v2,v4,v6). Matches gr.

## 0x4a4d70 FxInfoEmSpeed_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a4d90 FxInfoEmSpeed_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Same frame as 0x4a4cb0; `info->m_Speed(+0x20) = values[0]` (fld/fstp), `info->m_fSpeedBias(+0x24) = values[1]` (0x4a4db1/0x4a4db4).

## 0x4a4e30 FxInfoEmDir_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a4e50 FxInfoEmDir_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Same frame as 0x4a4cb0; `info->m_Direction = (values[0] (fld/fstp), values[1], values[2])` at +0x28/+0x2c/+0x30.

## 0x4a4f00 FxInfoEmAngle_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a4f20 FxInfoEmAngle_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Same frame; interp is FxInterpInfoFloat_c::GetVal 0x4a8470; `info->m_fAngleMin(+0x34) = values[0]`, `m_fAngleMax(+0x38) = values[1]`.

## 0x4a4fc0 FxInfoEmLife_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a4fe0 FxInfoEmLife_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Same frame; interp FxInterpInfoU255_c::GetVal 0x4a8800; `info->m_fLife(+0x3c) = values[0]`, `m_fLifeBias(+0x40) = values[1]`.

## 0x4a5080 FxInfoEmPos_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5150 FxInfoEmWeather_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5170 FxInfoEmWeather_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Same frame as 0x4a4cb0 (`ret 0x18`, no time-mode prologue): `m_InterpInfo(+8).GetVal(values, currentTime)` 0x4a8470; `info+0x58 = values[0]` (fld/fstp), `+0x5c/+0x60/+0x64 = values[1..3]` (integer moves).

## 0x4a5220 FxInfoEmRotation_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5240 FxInfoEmRotation_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Same frame as 0x4a4cb0; `info->m_fRotationMinAngle(+0x50) = values[0]`, `m_fRotationMaxAngle(+0x54) = values[1]`.

## 0x4a52e0 FxInfoNoise_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5300 FxInfoNoise_c::GetValue (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void (float currentTime, float mult, float totalTime, float len, bool useConst, MovementInfo_t* info)`, `ret 0x18`.
- Time-mode prologue (common to the particle infos 0x4a5300..0x4a63b0): `if (!m_bTimeModeParticle [this+6]) mult = currentTime / len` (fdiv, stored f32 over the arg, 0x4a5314); else mult unchanged.
- `m_InterpInfo(+8).GetVal(&value, mult)` 0x4a89c0 (FxInterpInfo32_c).
- `in.x = (rand() % 10000) * 0.0002f [0x85a7d0] - 1.0f [0x858624]` (`idiv` signed remainder, fild, fmul, fsub, stored f32), then in.y, then in.z (three separate `_rand` 0x821b1e calls in x, y, z order).
- `RwV3dNormalize(&in, &in)` 0x7ed9b0 (return discarded); `s = value * totalTime`; `in.x = in.x*s`, `in.y = in.y*s`, `in.z = s*in.z` (stored f32).
- `length = RwV3dLength(&info->m_Vel [+0xc])` 0x7edac0 (stored f32); `vel += in` (x, y, z); `if (RwV3dLength(&vel) > 0.0f [0x858b50])` (`test ah,0x41`; NaN skips) `RwV3dNormalize(&vel, &vel)`; `vel.x = length*vel.x`, then y, z.
- gta-reversed matches (written as `/ 5000.0f`; bytes multiply by 0.0002f, not bit-identical for all inputs -> port must multiply).

## 0x4a54c0 FxInfoForce_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a54e0 FxInfoForce_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall`, 6 args as 0x4a5300, `ret 0x18`; time-mode prologue as 0x4a5300.
- `GetVal(values, mult)` 0x4a89c0; `vel.x = values[0]*totalTime + vel.x` (+0xc), then y (+0x10), z (+0x14) (0x4a5512-0x4a553d).

## 0x4a55c0 FxInfoFriction_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a55e0 FxInfoFriction_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Prologue as 0x4a5300. `GetVal(values, mult)`; `p = _CIpow(values[0], totalTime * 50.0f [0x858b40])` (0x822130, x87 register args, 0x4a5612-0x4a5620); `vel.x = p*vel.x`, `vel.y = p*vel.y`, `vel.z = p*vel.z` (p kept in ST0, unrounded between components).

## 0x4a56c0 FxInfoAttractPt_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a56e0 FxInfoAttractPt_c::GetValue (proposed)  (catalogue: specified/REA, port level 2)
- Prologue as 0x4a5300. `GetVal(values[4], mult)` 0x4a89c0.
- `in = (values[0]-pos.x, values[1]-pos.y, values[2]-pos.z)` (pos = info+0, stored f32); `length = RwV3dLength(&in)` (stored over the mult slot); `if (length > 0.0f [0x858b50]) RwV3dNormalize(&in,&in)`.
- `if (length > 0.1f [0x858b1c])` (`test ah,0x41`; NaN/<= skip, 0x4a5775): `s = values[0] * totalTime` (0x4a577a); `vel.x += in.x*s`, `vel.y += s*in.y`, `vel.z += s*in.z` (vel = info+0xc).
- Original bug: strength is values[0] (the point's x), not values[3]; gr keeps it.

## 0x4a59f0 FxInfoGroundCollide_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5a80 FxInfoWind_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5aa0 FxInfoWind_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Prologue as 0x4a5300. `GetVal(&value, mult)`; `s = (value * *[0xa9aeb0]) * totalTime` (0x4a5ad2-0x4a5ae8, kept in ST0); `vel.x = s*dir.x + vel.x` with `dir = *[0xa9aeac]` re-read per component, then y, z.

## 0x4a5b90 FxInfoJitter_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5bb0 FxInfoJitter_c::GetValue (proposed)  (catalogue: specified/REA, port level 2)
- Prologue as 0x4a5300. `GetVal(&value, mult)`; random `in` exactly as 0x4a5300 (x, y, z `rand()%10000 * 0.0002f [0x85a7d0] - 1.0f [0x858624]`); `RwV3dNormalize(&in,&in)`; `s = value*totalTime`; `in *= s` (in.x kept in ST0, y and z stored); `pos.x += in.x`, `pos.y += in.y`, `pos.z += in.z` (pos = info+0).

## 0x4a5d20 FxInfoRotSpeed_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5d40 FxInfoRotSpeed_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Prologue as 0x4a5300; `GetVal(values, mult)` 0x4a8470; `info+0x18 = values[0]` (fld/fstp), `+0x1c,+0x20,+0x24 = values[1..3]`.

## 0x4a5df0 FxInfoFloat_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5e10 FxInfoFloat_c::GetValue (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret 0x18`: `byte[info+0x28] = 1` (0x4a5e14). No interpolation.

## 0x4a5e70 FxInfoUnderwater_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5e90 FxInfoUnderwater_c::GetValue (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret 0x18`: `byte[info+0x29] = 1` (0x4a5e94).

## 0x4a5f10 FxInfoColour_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a5f30 FxInfoColour_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Prologue as 0x4a5300; `GetVal(values, mult)` 0x4a8800; for i = 0..3: `fld values[i]; call 0x821b40` (CRT `_ftol`, truncation) and store AL to `info+i` (r, g, b, a).

## 0x4a6010 FxInfoSize_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall void (FILESTREAM file, i32 version)`, `ret 8`: `if (version < 0x69) byte[this+0xe] = 2` (signed, 0x4a6010); then `m_InterpInfo(+8)->Load(file)` via vtable slot 1.

## 0x4a6030 FxInfoSize_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Prologue as 0x4a5300; `GetVal(values, mult)` 0x4a89c0; `info+0xc = values[0]` (fld/fstp), `info+0x10 = values[1]`; `if (byte[this+0xe] == 4)` { `+0x14 = values[2]`, `+0x18 = values[3]` } else both = 0 (integer 0).

## 0x4a6120 FxInfoSpriteRect_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a6140 FxInfoSpriteRect_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Prologue as 0x4a5300; `GetVal(values, mult)` 0x4a8b80; `info+0x1c = values[0]` (fld/fstp), `+0x20,+0x24,+0x28 = values[1..3]`.

## 0x4a61f0 FxInfoHeatHaze_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a6210 FxInfoHeatHaze_c::GetValue (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret 0x18`: `byte[info+0x36] = 1` (0x4a6214).

## 0x4a6290 FxInfoTrail_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a62b0 FxInfoTrail_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Prologue as 0x4a5300; `GetVal(values, mult)` 0x4a8800; `info+0x30 = values[0]`; `info+0x2c = 1`; `if (values[1] > 0.1f [0x858b1c])` (`test ah,0x41`, NaN -> 1) `info+0x2c = 2`.

## 0x4a6390 FxInfoFlat_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a63b0 FxInfoFlat_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Prologue as 0x4a5300; `GetVal(values[9], mult)` 0x4a8470; integer copies: right `info+0x38/+0x3c/+0x40 = v0..v2`, up `+0x48..+0x50 = v3..v5`, at `+0x58/+0x5c/+0x60 = v6..v8`; `byte[info+0x78] = 1` (m_bIsFlat); `RwMatrixUpdate(info+0x38)` 0x7f18a0 (cdecl, `add esp,0x44` cleans it with the frame).

## 0x4a64b0 FxInfoDir_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a65c0 FxInfoAnimTexture_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a67b0 FxInfoSelfLit_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret 8`, no body (0x4a67b0): SelfLit has no interpolation data to load.

## 0x4a67c0 FxInfoSelfLit_c::GetValue (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall`, `ret 0x18`: `byte[info+0x37] = 1` (0x4a67c4).

## 0x4a6840 FxInfoColourBright_c::Load (proposed)  (catalogue: specified/REA, port level 0)
- Same as 0x4a4b40.

## 0x4a6860 FxInfoColourBright_c::GetValue (proposed)  (catalogue: specified/REA, port level 1)
- Prologue as 0x4a5300; `GetVal(values, mult)` 0x4a8800 (FxInterpInfoU255_c); `_ftol(values[0..3])` 0x821b40 -> bytes info+0..+3; `_ftol(values[4])` -> byte info+7; `dword[info+8] = 2` (0x4a68d2).

## 0x4a6a70 FxInfoEmRate_c::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoEmRate_c::~FxInfoEmRate_c 0x4a6a90(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6a70).

## 0x4a6a90 FxInfoEmRate_c::~FxInfoEmRate_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4a8460(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6a90).

## 0x4a6ae0 FxInfoEmSize_c::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a6b00(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6ae0).

## 0x4a6b00 FxInfoEmSize_c::~FxInfoEmSize_c (proposed)  (catalogue: unknown/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6b00).

## 0x4a6b50 FxInfoEmSpeed_c::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoEmSpeed_c::~FxInfoEmSpeed_c 0x4a6b70(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6b50).

## 0x4a6b70 FxInfoEmSpeed_c::~FxInfoEmSpeed_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6b70).

## 0x4a6bc0 FxInfoEmDir_c::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoEmDir_c::~FxInfoEmDir_c 0x4a6be0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6bc0).

## 0x4a6be0 FxInfoEmDir_c::~FxInfoEmDir_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6be0).

## 0x4a6c30 FxInfoEmAngle_c::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoEmAngle_c::~FxInfoEmAngle_c 0x4a6c50(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6c30).

## 0x4a6c50 FxInfoEmAngle_c::~FxInfoEmAngle_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4a8460(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6c50).

## 0x4a6ca0 FxInfoEmLife_c::scalar deleting destructor (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoEmLife_c::~FxInfoEmLife_c 0x4a6cc0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6ca0).

## 0x4a6cc0 FxInfoEmLife_c::~FxInfoEmLife_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoU255_c::~FxInterpInfoU255_c 0x4a87f0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6cc0).

## 0x4a6d10 FxInfoEmPos_c::~FxInfoEmPos_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoEmPos_c::~FxInfoEmPos_c 0x4a6d30(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6d10).

## 0x4a6d30 FxInfoEmPos_c::~FxInfoEmPos_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4a8460(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6d30).

## 0x4a6d80 FxInfoEmWeather_c::~FxInfoEmWeather_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a6da0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6d80).

## 0x4a6df0 FxInfoEmRotation_c::~FxInfoEmRotation_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a6e10(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6df0).

## 0x4a6e60 FxInfoNoise_c::~FxInfoNoise_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a6e80(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6e60).

## 0x4a6ed0 FxInfoForce_c::~FxInfoForce_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoForce_c::~FxInfoForce_c 0x4a6ef0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6ed0).

## 0x4a6ef0 FxInfoForce_c::~FxInfoForce_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6ef0).

## 0x4a6f40 FxInfoFriction_c::~FxInfoFriction_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoFriction_c::~FxInfoFriction_c 0x4a6f60(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6f40).

## 0x4a6f60 FxInfoFriction_c::~FxInfoFriction_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6f60).

## 0x4a6fb0 FxInfoAttractPt_c::~FxInfoAttractPt_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoAttractPt_c::~FxInfoAttractPt_c 0x4a6fd0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a6fb0).

## 0x4a6fd0 FxInfoAttractPt_c::~FxInfoAttractPt_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a6fd0).

## 0x4a7020 FxInfoAttractLine_c::~FxInfoAttractLine_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoAttractLine_c::~FxInfoAttractLine_c 0x4a7040(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7020).

## 0x4a7040 FxInfoAttractLine_c::~FxInfoAttractLine_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7040).

## 0x4a7090 FxInfoGroundCollide_c::~FxInfoGroundCollide_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a70b0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7090).

## 0x4a7100 FxInfoGroundCollide_c::GetValue (proposed)  (catalogue: specified/REA, port level 8)
- `__thiscall void (float currentTime, float mult, float totalTime, float len, bool useConst, MovementInfo_t* info)`, `ret 0x18`; frame 0xac + esi/edi.
- Time-mode prologue as 0x4a5300 (result kept in a local, 0x4a710f-0x4a712c).
- `origin = info->m_Pos` (+0..+8, copied to a local); `dist = origin.z - info->m_Vel.z [+0x14]` (0x4a7159/0x4a715d); `CWorld::ProcessVerticalLine(origin, dist, colPoint, colEntity, 1, 0, 0, 0, 0, 0, 0)` 0x5674e0 (cdecl, `add esp,0x2c`). `if (!hit) return`.
- `if (colPoint.z > origin.z) return` (`fcomp; test ah,0x41; je`, 0x4a71b0; NaN continues).
- `if (info->m_Pos.z >= colPoint.z or unordered) return` (`test ah,5; jp`, 0x4a71c2).
- Because origin.z == m_Pos.z (same f32), the two tests together reject every input: the code from 0x4a71c8 (GetVal, random vector, reflection, `m_Pos.z = colPoint.z`, velocity rescale) is unreachable. Port: implement the line test and the two returns only (the ProcessVerticalLine call must stay; it may have side effects such as scan-code increments).
- Dead-code note for reference: the reflection uses `k = 2 * dot(normal, vel) * values[0]` (0x4a71e3-0x4a720b), not gr's `|normal|^2 * values[0]`; random vector as in 0x4a5300 scaled by a product with 5.0f [0x858c80] (gr: values[2] * totalTime * 5; operand offsets not re-derived).
- gta-reversed (Fx/Blueprints/FxInfoGroundCollide.cpp) has the same two early returns, so it is also dead there.

## 0x4a7390 FxInfoWind_c::~FxInfoWind_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a73b0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7390).

## 0x4a7400 FxInfoJitter_c::~FxInfoJitter_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a7420(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7400).

## 0x4a7470 FxInfoRotSpeed_c::~FxInfoRotSpeed_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a7490(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7470).

## 0x4a74e0 FxInfoFloat_c::~FxInfoFloat_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoFloat_c::~FxInfoFloat_c 0x4a7500(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a74e0).

## 0x4a7500 FxInfoFloat_c::~FxInfoFloat_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4a8460(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7500).

## 0x4a7550 FxInfoUnderwater_c::~FxInfoUnderwater_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoUnderwater_c::~FxInfoUnderwater_c 0x4a7570(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7550).

## 0x4a7570 FxInfoUnderwater_c::~FxInfoUnderwater_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoU255_c::~FxInterpInfoU255_c 0x4a87f0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7570).

## 0x4a75c0 FxInfoColour_c::~FxInfoColour_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoColour_c::~FxInfoColour_c 0x4a75e0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a75c0).

## 0x4a75e0 FxInfoColour_c::~FxInfoColour_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoU255_c::~FxInterpInfoU255_c 0x4a87f0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a75e0).

## 0x4a7630 FxInfoSize_c::~FxInfoSize_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoSize_c::~FxInfoSize_c 0x4a7650(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7630).

## 0x4a7650 FxInfoSize_c::~FxInfoSize_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7650).

## 0x4a76a0 FxInfoSpriteRect_c::~FxInfoSpriteRect_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a76c0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a76a0).

## 0x4a7710 FxInfoHeatHaze_c::~FxInfoHeatHaze_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a7730(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7710).

## 0x4a7780 FxInfoTrail_c::~FxInfoTrail_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoTrail_c::~FxInfoTrail_c 0x4a77a0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7780).

## 0x4a77a0 FxInfoTrail_c::~FxInfoTrail_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoU255_c::~FxInterpInfoU255_c 0x4a87f0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a77a0).

## 0x4a77f0 FxInfoFlat_c::~FxInfoFlat_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `0x4a7810(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a77f0).

## 0x4a7860 FxInfoDir_c::~FxInfoDir_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoDir_c::~FxInfoDir_c 0x4a7880(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7860).

## 0x4a7880 FxInfoDir_c::~FxInfoDir_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4a8460(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7880).

## 0x4a78d0 FxInfoAnimTexture_c::~FxInfoAnimTexture_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoAnimTexture_c::~FxInfoAnimTexture_c 0x4a78f0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a78d0).

## 0x4a78f0 FxInfoAnimTexture_c::~FxInfoAnimTexture_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a78f0).

## 0x4a7940 FxInfoColourRange_c::~FxInfoColourRange_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoColourRange_c::~FxInfoColourRange_c 0x4a7960(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7940).

## 0x4a7960 FxInfoColourRange_c::~FxInfoColourRange_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoU255_c::~FxInterpInfoU255_c 0x4a87f0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7960).

## 0x4a79b0 FxInfoSelfLit_c::~FxInfoSelfLit_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoSelfLit_c::~FxInfoSelfLit_c 0x4a79d0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a79b0).

## 0x4a79d0 FxInfoSelfLit_c::~FxInfoSelfLit_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoU255_c::~FxInterpInfoU255_c 0x4a87f0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a79d0).

## 0x4a7a20 FxInfoColourBright_c::~FxInfoColourBright_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoColourBright_c::~FxInfoColourBright_c 0x4a7a40(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7a20).

## 0x4a7a40 FxInfoColourBright_c::~FxInfoColourBright_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoU255_c::~FxInterpInfoU255_c 0x4a87f0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7a40).

## 0x4a7a90 FxInfoSmoke_c::~FxInfoSmoke_c (proposed)  (catalogue: specified/REA, port level 2)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInfoSmoke_c::~FxInfoSmoke_c 0x4a7ab0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a7a90).

## 0x4a7ab0 FxInfoSmoke_c::~FxInfoSmoke_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void ()`, SEH frame (state 0 while the member is destroyed): `FxInterpInfoU255_c::~FxInterpInfoU255_c 0x4a87f0(this+8)` (m_InterpInfo destructor); then `[this] = 0x85a7dc` (FxInfo_c vtable, inlined ~FxInfo_c) (0x4a7ab0).

## 0x4a8460 FxInterpInfoFloat_c::~FxInterpInfoFloat_c (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x85a968` (base-class vtable store of an inlined trivial destructor) (0x4a8460).

## 0x4a85c0 FxInterpInfoFloat_c::GetVal(i32 attrib, float time, float dt) (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall float (i32 a [esp+4], float t [esp+8], float dt [esp+0xc])`, ST0 return, `ret 0xc`. Fields: `n = (i8)this[+5]` (key count), `keys = (u16*)this[+8]` (time * 256), `vals = (float**)this[+0xc]` (per attribute).
- Times: `key(i) = keys[i] * 0.00390625f [0x859aa0]` (fild of the zero-extended u16; exact).
- `if (n == 1) return dt * vals[a][0]` (0x4a85cd-0x4a85dd).
- Otherwise `sum = 0.0f [0x858b50]`, `s = t - dt` (x87, 0x4a85e2/0x4a85e7).
- Loop 1 (0x4a8600): first `i` in `0..n-1` with `s < key(i)` (`fcomp; test ah,5; jnp` = C0 set). If none (`i == n`): return `dt * vals[a][n-1]` (0x4a8626-0x4a8640).
- Value at s: `i == 0` -> `vs = vals[a][0]` (0x4a8685); else `f = (s - key(i-1)) / (key(i) - key(i-1))` (0x4a8669-0x4a8671), `vs = f * (v[i] - v[i-1]) + v[i-1]` (0x4a8673-0x4a867c).
- Loop 2 from that `i` while `i < n` (0x4a86a0): `kc = key(i)`, `kp = key(i-1)` (both stored as f32 over the dt / a arg slots; `keys[i-1]` is read even when i == 0):
  - `kc == t` (`test ah,0x44; jnp`, 0x4a86d8): `sum = sum + ((v[i] - vs) * 0.5f [0x858b8c] + vs) * (kc - s)`; return sum.
  - `kc < t` (0x4a86e7 falls through): `sum = sum + ((v[i] - vs) * 0.5f + vs) * (kc - s)`; `s = kc; vs = v[i]`; `++i`.
  - `kc > t` (`test ah,0x41; je`, 0x4a8722): `vt = (t - kp) / (kc - kp) * (v[i] - v[i-1]) + v[i-1]`; `sum = sum + ((vt - vs) * 0.5f + vs) * (t - s)`; return sum.
  - Unordered compare (NaN): `++i` with nothing added (0x4a8722 not taken).
- If loop 2 runs out (`i == n`, 0x4a8784): `sum = sum + ((v[n-1] - vs) * 0.5f + vs) * (key(n-1) - s)` (this segment is zero-width because s == key(n-1)); the span from the last key to t is NOT added. Return sum.
- All arithmetic stays on the x87 stack except kc/kp (f32 stores, exact). Each add is `sum + area` (`faddp st(3)`).
- Edge cases: when s < key(0) and key(0) > t, the `kc > t` branch reads `keys[-1]` and `vals[a][-1]` (out-of-bounds; harmless only if key(0) == 0 and t >= 0, which the data normally gives). n == 0 returns `dt * vals[a][-1]`.

## 0x4a87f0 FxInterpInfoU255_c::~FxInterpInfoU255_c (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x85a968` (base-class vtable store of an inlined trivial destructor) (0x4a87f0).

## 0x4a89b0 FxInterpInfo32_c::~FxInterpInfo32_c (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x85a968` (base-class vtable store of an inlined trivial destructor) (0x4a89b0).

## 0x4a8b70 FxInterpInfo255_c::~FxInterpInfo255_c (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall void ()`: `[this] = 0x85a968` (base-class vtable store of an inlined trivial destructor) (0x4a8b70).

## 0x4a8d30 FxInterpInfoFloat_c::`scalar deleting destructor' (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInterpInfoFloat_c::~FxInterpInfoFloat_c 0x4a8460(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a8d30).

## 0x4a8d50 FxInterpInfoU255_c::`scalar deleting destructor' (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInterpInfoU255_c::~FxInterpInfoU255_c 0x4a87f0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a8d50).

## 0x4a8d70 FxInterpInfo32_c::`scalar deleting destructor' (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInterpInfo32_c::~FxInterpInfo32_c 0x4a89b0(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a8d70).

## 0x4a8d90 FxInterpInfo255_c::`scalar deleting destructor' (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxInterpInfo255_c::~FxInterpInfo255_c 0x4a8b70(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a8d90).

## 0x4a8de0 empty function (ret) (proposed)  (catalogue: unknown/REA, port level 0)
- Empty: single `ret` (0x4a8de0). Port: no-op.

## 0x4a9340 FxSystem_c::scalar deleting destructor (proposed)  (catalogue: unknown/REA, port level 4)
- `__thiscall void* (u32 flags)`, `ret 4`: `FxSystem_c::Destructor 0x4aa260(this)` (destructor); `if (flags & 1) 0x8214bd(this)` (operator delete / free); EAX = this (0x4a9340).

## 0x4a94e0 FxEmitterPrt_c::FxEmitterPrt_c (proposed)  (catalogue: specified/REA, port level 1)
- `__thiscall FxEmitterPrt_c* ()`: `ListItem_c::ListItem_c(this)` 0x4a8db0; EAX = this. No other field initialised.

## 0x4a9f20 FxPrim_c::FxPrim_c (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall FxPrim_c* ()`, EAX = this: `[+0] = 0x85a9e0` (FxPrim_c vtable), `[+4] = 0`, `[+8] = 0`, `byte[+0xc] = 1`.

## 0x4a9f40 FxPrim_c::~FxPrim_c (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall void ()`: `[+0] = 0x85a9e0`, `[+4] = 0`, `[+8] = 0` (EAX = 0).

## 0x4a9f70 FxSphere_c::FxSphere_c (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall FxSphere_c* ()`, EAX = this: `[this+0x10] = 0` (store at 0x4096e1, reached by jmp; extent piece 0x4096e1-0x4096ed belongs here). Center/radius (+0..+0xc) not initialised.

## 0x4ab250 ReadLine (FxTools wrapper) (proposed)  (catalogue: unknown/REA, port level 1)
- `jmp CFileMgr::ReadLine 0x5389b0` with the caller's stack args unchanged (cdecl `(FILESTREAM, char* buf, i32 size)`). gr Fx/FxTools ReadLine.

## 0x4ab270 FxPrtMult_c::FxPrtMult_c() (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall FxPrtMult_c* ()`, EAX = this: dwords +0..+0x18 = 1.0f (colour r,g,b,a, size, spin, life). `mov ecx, 0x3f800000` sits at 0x40a18c (relocated, extent piece belongs here).

## 0x4ab290 FxPrtMult_c::FxPrtMult_c(float r, float g, float b, float a, float size, float spin, float life) (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall`, EAX = this, `ret 0x1c`: `[+0]=r, [+4]=g, [+8]=b, [+0xc]=a, [+0x10]=size, [+0x14]=spin, [+0x18]=life` (integer moves).
