# RenderWare 3.6 RpLight - pass-3 specs, writer A (RE-SPEC-RW-3)

Statically linked RW 3.6 world-plugin code (baworld light.c). gta-reversed only has `plugin::Call*` thunks. All
`__cdecl`. Extends docs/re/specs/RW/RpLight.md (pass 1/2, read-only).

## Layouts used (offsets proven by the instruction addresses given)

`RpLight`: +0x1 u8 subType (light type, 0x751ead), +0x2 u8 flags (0x751eb2), +0x14 f32 radius (0x751e8c), +0x18
RwRGBAReal color (r +0x18, g +0x1c, b +0x20; 0x751e92/0x751e8f/0x751e99), +0x28 f32 minusCosAngle (0x751ae5, 0x751d50).
Light plugin registry 0x8d62f8 (0x751e15); LIGHTMODULE engine data: light freelist at `G + [0xc925d8]` (0x7521b0),
open counter `[0xc925dc]` (0x7521f4).
Constants: `[0x858b50] = 0.0f`, `[0x858fe4] = 1.5707964f`, `[0x858cb8] = 3.1415927f`, `[0x858624] = 1.0f`, `[0x858b8c] =
0.5f`; acos polynomial (fdlibm e_acosf): pS0 `[0x85f0a0] = 0.16666667f`, pS1 `[0x874e80] = 0.3255658f` (subtracted), pS2
`[0x874e84] = 0.20121253f`, pS3 `[0x874e88] = 0.040055536f` (subtracted), pS4 `[0x874e8c] = 0.00079153504f`, pS5 `[0x874e90]
= 3.479331e-05f`, qS1 `[0x874e70] = 2.403395f` (subtracted), qS2 `[0x874e74] = 2.0209458f`, qS3 `[0x874e78] = 0.68828398f`
(subtracted), qS4 `[0x874e7c] = 0.077038154f`, pio2_lo `[0x874e6c] = 7.54979e-08f`, pio2_hi `[0x874e68] = 1.5707963f`, pi
`[0x874e64] = 3.1415925f`.

## 0x751ae0 RpLightGetConeAngle  (catalogue: specified/REA, port level 1)
- `f32 (const RpLight* l)` (ST0). `x = f32(-l->minusCosAngle)` (0x751ae5..0x751aea), `ix = bits(x) & 0x7fffffff`;
  inlined fdlibm `acosf(x)` with the f32 constants above:
  - `ix >= 0x3f800000` (0x751afd): `x > 0` (signed int test of the raw bits, 0x751b04) -> 0.0, else -> pi `[0x858cb8]`
    (NaN/|x| > 1 return 0 or pi, not NaN).
  - `ix < 0x3f000000` (|x| < 0.5): `ix <= 0x23000000` -> pi/2 `[0x858fe4]` (0x751b2a); else `z = x*x`, `p = z*(pS0 + z*(-pS1 +
    z*(pS2 + z*(-pS3 + z*(pS4 + z*pS5)))))` evaluated Horner-style from pS5 (0x751b3a..0x751b68), `q = 1 + z*(-qS1 + z*(qS2 +
    z*(-qS3 + z*qS4)))` (0x751b6a..0x751b8a), `r = p/q`; result `pio2_hi - (x - (pio2_lo - x*r))` (0x751b92..0x751ba0).
  - `x < 0` (0x751baa): `z = f32((1 + x) * 0.5)`, `s = _rwSqrt(z)` (0x7edb30), `r = p(z)/q(z)`; result `pi - 2*(s + (r*s -
    pio2_lo))` (0x751bd0..0x751c4b; `pi [0x874e64]`).
  - else: `z = f32((1 - x) * 0.5)`, `s = _rwSqrt(z)`, `df = bits(s) & 0xfffff000`, `c = (z - df*df) / (s + df)`; result
    `2*(df + (r*s + c))` (0x751c53..0x751d10).
- librw: absent (librw stores the angle differently).

## 0x751d20 RpLightSetConeAngle  (catalogue: specified/REA, port level 0)
- `RpLight* (RpLight* l, f32 angle)`: `angle < 0.0` -> NULL (0x751d24..0x751d2f, exactly-C0 test); `angle > pi/2
  [0x858fe4]` -> NULL (0x751d35..0x751d42); else `l->minusCosAngle = f32(-fcos(angle))` (x87 `fcos`, 0x751d48..0x751d50),
  return l. NaN passes both tests and stores NaN.
- librw: absent.

## 0x751e10 RpLightStreamGetSize  (catalogue: specified/REA, port level 1)
- `_rwPluginRegistryGetSize(0x8d62f8, l) + 0x30` (0x751e1a..0x751e22).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x751e30 RpLightStreamWrite  (catalogue: specified/REA, port level 4)
- `RpLight* (const RpLight* l, RwStream* s)`: `Header(s, 0x12 LIGHT, plugins + 0x30, 0x36003, 0xffff)`, `Header(s, 1, 0x18)`;
  raw 0x18 bytes `{radius, r, g, b, minusCosAngle, (subType << 16) | flags}` (0x751e8c..0x751eca); plugin data chunks
  (0x751ee3); failure -> NULL, else l.
- librw: light.cpp:Light::streamWrite matches in layout (librw converts its stored angle back to minusCos).

## 0x752060 _rpLightChunkInfoRead  (catalogue: specified/REA, port level 1)
- `info* (RwStream* s, info* out /*0x18*/, i32* size)`: `FindChunk(s, 1, &len, NULL)` fail -> NULL; zero 6 dwords; read 0x18
  != 0x18 -> NULL; `*size = len + 0xc`; `RwStreamSkip(s, len - 0x18)` (0x7520b8, result ignored); return out.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7521b0 cb_LIGHTMODULE_RwEngine_Close  (catalogue: specified/REA, port level 0)
- `RwFreeListForAllUsed(lightFL, _rpLightDestructor 0x752210, NULL)` (0x7521c6), `RwFreeListDestroy(lightFL)`, slot = 0,
  `[0xc925dc] -= 1`; return instance.
- librw: absent.

## 0x752210 _rpLightDestructor  (catalogue: specified/REA, port level 1)
- `void (RpLight* l, void* data)`: `_rwPluginRegistryDeInitObject(0x8d62f8, l)`, `_rwObjectHasFrameReleaseFrame(l)`,
  `freeListFree(lightFL, l)` (0x75221b..0x752236). Note: unlike RpLightDestroy it does not remove the light from a world.
- librw: absent.

