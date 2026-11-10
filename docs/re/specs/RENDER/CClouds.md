# CClouds (RENDER) - full specs

## 0x7154B0 CClouds::RenderBottomFromHeight  (catalogue: specified/REA, port level 6)
- Convention: `__cdecl void ()`, plain `ret` (0x715F3D). Frame `sub esp,0x44` + push ebp/ebx/esi/edi.
- Caller: Game::RenderScene 0x53DF40.
- Callees: `_ftol` 0x821B40 (truncation), RwRenderStateSet via `RwEngineInstance` 0xC97B24 `+0x20` (cdecl (state, value)),
  RwIm3DTransform 0x7EF450, RwIm3DRenderIndexedPrimitive 0x7EF550, RwIm3DEnd 0x7EF520, CSprite::InitSpriteBuffer 0x70CFB0,
  CSprite::CalcScreenCoors 0x70CE30, CSprite::RenderBufferedOneXLUSprite 0x70E4A0, CSprite::FlushSpriteBuffer 0x70CF20.
- Globals: CTimeCycle::m_CurrentColours fluffy-cloud bottom colour u16 R/G/B at 0xB7C502/0xB7C504/0xB7C506 (+0x62/+0x64/+0x66),
  m_fCloudAlpha f32 0xB7C538 (+0x98); TheCamera (camera position `C` = `TheCamera.m_matrix` [0xB6F03C] ? matrix+0x30 : 0xB6F02C, re-read at
  every use); CTimer::ms_fTimeStep 0xB7CB5C; CWeather::Wind 0xC812F0; `windShift` f32 0xC6E954 (read/write); gpCloudMaskTex 0xC6AA7C;
  TempVertexBuffer 0xC4D958 (RwIm3DVertex 0x24: +0 x, +4 y, +8 z, +0x18 colour 0xAARRGGBB, +0x1C u, +0x20 v), aTempBufferIndices
  0xC4B958 (u16), uiTempBufferIndicesStored 0xC4B954, uiTempBufferVerticesStored 0xC4B950.
  Tables (f32, raw dwords from the image, must be copied exactly):
  `T80` 0x8D5658 (80 entries): 3eb851ec 3df5c28f 3f70a3d7 3eae147b 3f547ae1 3f400000 3e8f5c29 3dcccccd 3cf5c28f 3f59999a 3f051eb8
  3f6b851f 3eb851ec 3f547ae1 3db851ec 3e851eb8 3edc28f6 3e8a3d71 3f733333 3ca3d70a 3f23d70a 3e6b851f 3efae148 3f0ccccd 3e99999a 3f6147ae
  3ebd70a4 3f70a3d7 3f147ae1 3f2b851f 3e570a3d 3ef0a3d7 3ec28f5c 3cf5c28f 3e851eb8 3f75c28f 3f3d70a4 3f28f5c3 3f7d70a4 3de147ae 3f59999a
  3e947ae1 3f147ae1 3e99999a 3e99999a 3eb851ec 3ec7ae14 3da3d70a 3f147ae1 3ea8f5c3 3e23d70a 3f428f5c 3eae147b 3f75c28f 3f30a3d7 3f35c28f
  3f400000 3ec7ae14 3ef5c28f 3f666666 3f4ccccd 3f428f5c 3f30a3d7 3e8f5c29 3e6b851f 3e947ae1 3f400000 3f3851ec 3e0f5c29 3f3ae148 3f170a3d
  3eb33333 3f666666 3e3851ec 3f451eb8 3d23d70a 3ef5c28f 3ebd70a4 3f4f5c29 3f2b851f.
  `TY30` 0x8D57A0 (30): 54 105 203 65 213 154 169 54 76 10 119 204 176 154 132 190 200 37 193 154 71 111 234 12 195 165 104 94 23 222 (exact).
  `TX30` 0x8D5818 (30): 100 50 156 123 232 12 76 203 164 120 64 164 94 195 125 71 243 123 176 65 132 54 94 12 234 167 165 200 100 40 (exact).
- Behaviour:
  1. Colour (0x7154B3-0x715559): `cr = 2*R + 20.0f [0x858BA4]` (fild R; `fadd st0,st0`; fadd), `cg = G * 1.5f [0x858CE8]`,
     `cb = B * 1.5f`; each: if `255.0f [0x859AAC] < v` -> 255.0f (`test ah,5; jp` keeps v when 255 > v, equal or NaN); then `_ftol` (ints).
  2. If `C.z < -90.0f [0x859CE8]` return (0x715575, `jnp`). `low = 160.0f`, `high = 190.0f` (imm). `d = ((C.z - 190.0f [0x858A20]) -
     10.0f [0x85862C]) * 0.3f [0x858C24]`; if `d > 0` (strict): `low = 160.0f [0x858970] + d`, `high = d + 190.0f`.
  3. `windShift = (ms_fTimeStep * CWeather::Wind) * 0.25f [0x858C84] + windShift` (stored f32).
  4. Render states: (8 ZWRITEENABLE, 0), (6 ZTESTENABLE, 1), (12 VERTEXALPHAENABLE, 1), (14 FOGENABLE, 0), (10 SRCBLEND, 5),
     (11 DESTBLEND, 6), (1 TEXTURERASTER, gpCloudMaskTex->raster) (0x7155EE-0x715651). `A = _ftol(m_fCloudAlpha)` (0x71565D).
  5. If `A != 0` (0x71566A; a negative A runs the loops): flat cloud quads (0x715670-0x715B52). `H = high - low` (x87, kept for the loop).
     Counters = 0. For `i = 1 .. 28` (loop test `(i+1) - 1 < 28` at 0x715B04):
     a. `o = (T80[(i+5)%80] * 0.5f + 1.0f) * windShift` (stored f32); `x = (T80[(i-1)%80] * 512.0f [0x8594D8] + o) - C.x`,
        `y = (T80[i%80] * 512.0f + o) - C.y` (x87). `x -= (float)((_ftol(x) & 0xFFFFFE00) + 0x100)` (stored f32), likewise y (kept).
        `P.x = x + C.x`, `P.y = y + C.y` (stored f32); `P.z = H * T80[(i+1)%80] + low` (stored f32). (`%` = signed idiv by 0x50.)
     b. `a = A`; `dz = P.z - C.z`; if `dz > 0`: `a = _ftol((1.0f - dz * 0.004f [0x859CE0]) * (int)A)` (fimul). If `a <= 0` skip.
     c. `m = max(|C.x - P.x|, |C.y - P.y|)` (abs by compare-and-negate, recomputed from memory; |dy| chosen when `|dx| <= |dy|` or
        NaN); `m *= 0.00390625f [0x859AA0]`; if `m > 0.75f [0x858F34]`: `a = _ftol((1.0f - (m - 0.75f) * 4.0f) * (int)a)`. If `a <= 0` skip.
     d. `sx = T80[(i+2)%80] * 100.0f + 60.0f [0x858B34]`, `sy = T80[(i+3)%80] * 100.0f + 60.0f`. With `v = verticesStored`,
        `n = indicesStored`: indices `[n..n+5] = v+2, v+1, v, v+2, v+3, v+1`. Vertices (z = P.z for all):
        v0 = (P.x - sx, P.y + sy), uv (0,0), colour `(a<<24)|(cr<<16)|(cg<<8)|cb`;
        v1 = (P.x + sx, P.y + sy), uv (0,1); v2 = (P.x - sx, P.y - sy), uv (1,0); v3 = (P.x + sx, P.y - sy), uv (1,1);
        v1..v3 colour = `((((a<<8) | R) << 8 | G) << 8) | B` with the RAW u16 timecycle values R, G, B (original quirk: only v0 gets
        the scaled/clamped colour; values > 255 would bleed into neighbouring bytes). `n += 6`, `v += 4` (globals updated).
  6. After the loop: if `n != 0`: `RwIm3DTransform(0xC4D958, v, nullptr, 0x19)`; if non-null:
     `RwIm3DRenderIndexedPrimitive(3 TRILIST, 0xC4B958, n)`, `RwIm3DEnd()`. Both counters = 0.
  7. Sprites (0x715B57): `CSprite::InitSpriteBuffer()`. If `A > 0` (signed): `H = high - low` (stored f32), `Af = (float)A`; for
     `i = 0 .. 29`:
     a. `ws = windShift * 0.5f`; `x = (ws + TX30[i]) - C.x`, `y = (ws + TY30[i]) - C.y`; `x -= (float)((_ftol(x) & 0xFFFFFF00) + 0x80)`,
        `y -= (int)((_ftol(y) & 0xFFFFFF00) + 0x80)` (fisub); `P.x = x + C.x`, `P.y = y + C.y`, `P.z = H * T80[i%80] + low`.
     b. `a = _ftol((1.0f - |C.z - P.z| * 0.004f) * Af)`; if `a <= 0` skip. `s = (T80[(i+1)%80] + 1.0f) * 12.0f [0x858CCC]`.
     c. `m = max(|C.x - P.x|, |C.y - P.y|) * 0.0078125f [0x858B88]`; if `!(m < 1.0f)`: `m = 1.0f` and fade; else if `m > 0.75f`: fade.
        Fade: `a = _ftol((1.0f - (m - 0.75f) * 4.0f) * (int)a)` (no skip when it reaches 0).
        If `!(m > 0.05f [0x858C28])` skip. If `m < 0.1f [0x858B1C]`: `a = _ftol(((m - 0.05f) * (int)a) * 20.0f)`.
     d. `if CalcScreenCoors(&P, &S, &w, &h, false, true)`: `RenderBufferedOneXLUSprite(S.x, S.y, S.z, w * s, h * s, R, G, B,
        256, 1.0f / S.z, (u8)a)` with the RAW timecycle R, G, B (low bytes). (w, h locals overlap the dead `high` and raw-R slots.)
  8. `FlushSpriteBuffer()`; render states (1 TEXTURERASTER, 0), (12, 0), (8 ZWRITE, 1), (6 ZTEST, 1). Return.
- Edge cases: `& 0xFFFFFE00` / `& 0xFFFFFF00` on negative ints floor to the lower multiple (two's complement), after `_ftol`
  truncation toward zero. FOGENABLE is set to 0 and never restored; SRC/DESTBLEND are not restored.
- gta-reversed: only a commented-out partial draft (Clouds.cpp:861) that sets ZWRITEENABLE/FOGENABLE to TRUE (bytes: 0) and stops
  after the render states. Port the bytes. Open questions: none.
