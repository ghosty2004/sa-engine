# CCoronas (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Coronas.cpp`. `aCoronas` 0xC3E058, 64 x 0x3C (CRegisteredCorona: +0x00 pos,
+0x0C m_dwId, +0x10 texture, +0x14 size, +0x1C farClip, +0x20 nearClip, +0x24 heightAboveGround, +0x2C r,g,b,a,
+0x30 fadedIntensity, +0x32 flareType, +0x33 usesReflection, +0x34 bit0 checkObstacles, +0x36 bit2
hasValidHeightAboveGround, +0x38 attachedTo). Render states are set through `RwEngineInstance` (0xC97B24) `+0x20`
(RwRenderStateSet).

## 0x6FAEC0 CCoronas::Render  (gr_deviations = 2)
Loop register `edi` = &corona+0x38 (starts 0xC3E090, 0x6FAF41). Only the two classified rows are judged; the three
`bug-note deviates=no` rows (Coronas.cpp:265, 291 "/256", 359) are comments, not behaviour.

Row Coronas.cpp:291 (rng-helper, `CGeneral::GetRandomNumberInRange(0.7f, 1.f) * (m_FadedIntensity / 256.f)`):
- Original bytes 0x6FB319-0x6FB35B: `r = rand()` (0x821B1E, CRT), then x87:
  `m = ((float)r * 3.0518509e-05f [0x858C7C = 1/32767]) * 0.3f [0x858C24] + 0.7f [0x858CB0]`;
  `m = m * (int)fadedIntensity (byte +0x30, fimul at 0x6FB351) * 1.5258789e-05f [0x859D5C = 2^-16]`, stored f32 at
  0x6FB35B. The rand() call happens after `RwRenderStateSet(6 ZTEST, 0)` (0x6FB316) and before
  `RwRenderStateSet(1 TEXTURERASTER, gpCoronaTexture[0]->raster)` (0x6FB35F).
  Per flare sprite the colour is `(u8)ftol((float)(color.c (u8) * def.ColorMult.c (i16, imul)) * m)` (0x6FB370-0x6FB3C7,
  ftol 0x821B40 = truncation), i.e. the integer product is formed first and the fixed-point /256 of ColorMult is folded
  into the 2^-16 factor.
- gr: inlined gr `GetRandomNumberInRange` (lerp form, see docs/re/cat14-gr-deviations.md finding 1) and
  `(float)c * mult * (ColorMult/256)` - same value in exact arithmetic, different rounding and a different range
  formula.
- Verdict: CONFIRMED. Class: keep-original (formula `rand*(1/32767)*0.3+0.7`, then `*faded*2^-16`, then
  `(int)(c*cm) * m` truncated).

Row Coronas.cpp:298 (notsa-unreachable, `default: NOTSA_UNREACHABLE()` in the flare-type switch):
- Original bytes 0x6FB2EE-0x6FB309: flareType (byte +0x32) == 0 skips the flare block (0x6FB2F3 -> 0x6FB605);
  1 -> `esi = 0x8D4B68` (SunFlareDef), 2 -> `esi = 0x8D4D88` (HeadLightsFlareDef); any other value leaves `esi`
  unchanged and continues (no trap). `esi` was last set at 0x6FAF35 from an uninitialised stack slot (frame offset
  0x2C, never written before), or by an earlier corona's flare loop (then it points at a terminator entry whose
  `Sprite` (+0x10) is 0, so the sprite loop at 0x6FB365 runs zero times). The rand() call and render-state calls
  still happen.
- Verdict: CONFIRMED (gr aborts, the original goes on with a stale pointer). Class: harmless (only types 0..2 are
  ever registered); a port should mirror "unknown type = no sprites, but still consume one rand()".

## 0x6FB630 CCoronas::RenderReflections  (gr_deviations = 1)
Row Coronas.cpp:422 (rbf, `notsa::bugfixes::PS2CoronaRendering ? onScrPos.z : RwIm2DGetNearScreenZMacro()`):
- Original bytes 0x6FB99E-0x6FB9A1: sprite z = `RwEngineInstance->+0x18` (near screen Z) unconditionally.
- Verdict: CONFIRMED (gr's optional bugfix changes it; with the flag off gr matches). Class: keep-original.

Found in passing (CONFIRMED gr logic errors, keep-original; the original's loop is 0x6FB6D5-0x6FB9BC, `esi` =
&corona+0x30 from 0xC3E088, `ebp` = index i, `bl` = 4):
- Entry filter (0x6FB6D5-0x6FB6F5): skip unless `m_dwId != 0` and (`fadedIntensity != 0` or `color.a != 0`) and
  `usesReflection (+0x33) != 0`. gr checks none of these.
- Height refresh (0x6FB75E-0x6FB7D9): if bit2 of +0x36 is set, refresh only when `((CTimer::m_FrameCounter
  [0xB7CB4C] + i) & 0xF) == 0`; if clear, always try. gr's `(i & 0xFF) + (frame & 0xFF) % 16 == 0` is an operator
  precedence error (true only for i = 0). The probe is `CWorld::ProcessVerticalLine(pos, -1000.0f (imm 0xC47A0000),
  &colPoint, &entity, 1, 0, 0, 0, 1, 0, 0)` (0x5674E0), not gr's FindGroundZFor3DCoord; on a hit `bit2 |= 4` (only on
  the "flag was clear" path, 0x6FB7CE) and `height(+0x24) = pos.z - colPoint.z` (0x6FB7D1-0x6FB7D9).
- 0x6FB7F0: proceeds only if `height < 20.0f [0x858BA4]` (NaN skips). gr compares the bool flag (`>= 20.f`, never true).
- 0x6FB81C: proceeds only if `pos.z - height <= camPos.z` (equal passes, NaN skips); gr skips on `>=`.
- Colour (0x6FB878-0x6FB977): `clip = farClip*0.75f [0x858F34]; if (55.0f [0x85ADBC] < clip) clip = 55` ; skip unless
  `z < clip`; `half = clip*0.5f`; `t = z < half ? 1.0f : 1 - (z - half)/half` clamped to [0,1] (`>1 -> 1`, `<0 -> 0`,
  NaN kept); `k = ftol(WetRoads*t*(20.0f - height)*230.0f [0x858F80]*0.05f [0x858C28])` (this order); channel =
  `(u8c * (i16)k) >> 8` (arithmetic shift). gr multiplies floats then truncates once (different rounding) and has a
  NOTSA debug override.
- Size: `w = sx*size*0.75f`, `h = (sy*size)*2` (fadd st0,st0); intensity 128, recipZ = `1/Scene.camera->nearClip (+0x80)`.
- Before the loop the original calls `CSprite::InitSpriteBuffer` 0x70CFB0 (0x6FB665); gr does not.
- WetRoads gate (0x6FB639-0x6FB645): returns (clearing bit2 of every corona) when `WetRoads <= 0`; NaN proceeds.
