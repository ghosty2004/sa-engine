# CLines (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Lines.cpp`, `Lines.h` (pin 20d8d2a). Colours are passed as uint32 0xRRGGBBAA (CRGBA order
R = byte 3 ... A = byte 0); RW vertices take ARGB.

## 0x6FF460 CLines::RenderLineNoClipping  (gr_deviations = 1)
Original: `__cdecl void (float sx, sy, sz, ex, ey, ez [esp+4..+0x18], uint32 startColor [+0x1C], uint32 endColor [+0x20])`; ends with
a tail `jmp RwIm3DEnd` (0x6FF4EA) or `ret` (0x6FF4EF).
Row Lines.h:12 ("NOTSA wrappers": CVector/CRGBA overloads that forward to the float version).
- Original bytes: colours rotated to ARGB `(c << 24) | (c >> 8)` (0x6FF460-0x6FF47A, same as gr). The two vertices live in a STATIC
  buffer in .data: vertex 0 pos 0xC3EFF0/0xC3EFF4/0xC3EFF8, colour 0xC3F008; vertex 1 pos 0xC3F014/0xC3F018/0xC3F01C, colour 0xC3F02C
  (RxObjSpace3DVertex stride 0x24; other fields not written). Then `LittleTest()` (0x6FF4C2), `if (RwIm3DTransform(0xC3EFF0, 2, NULL, 0))
  { RwIm3DRenderLine(0, 1); RwIm3DEnd(); }`.
- Verdict: REFUTED - the wrappers only forward arguments; no behavioural difference. Class: harmless.
  Found in passing: gr builds the vertices on the stack (other fields zeroed), the original in the static .data buffer 0xC3EFF0 (its
  normal/uv fields keep whatever is there; unused because the transform flags are 0). Harmless unless something else reads 0xC3EFF0.

## 0x6FF4F0 CLines::RenderLineWithClipping  (gr_deviations = 1)
Original: same argument list as 0x6FF460; tail `jmp RwIm3DEnd` (0x6FF781) or `ret` (0x6FF78C).
Row Lines.cpp:39 ("NOTE: Original computes `from + (to - from) * t`, so the components may be off by 1"; gr uses `lerp(startRGBA, endRGBA, t)`).
- Original bytes:
  1. `d = sqrt((dz*dz + dy*dy) + dx*dx)` with `dx = sx - ex`, `dy = sy - ey`, `dz = sz - ez` (0x6FF4F0-0x6FF51B);
     `v = d * 0.4f + 1.0f` ([0x858EE8] = 0x3ECCCCCD, [0x858624], 0x6FF523-0x6FF529); `if (!(7.0f > v) && !NaN) v = 7.0f`
     (`test ah,5; jp`, equal -> 7.0, 0x6FF52F-0x6FF540); `n = (int16)_ftol(v)` (0x6FF549, used as `bp`); if `n <= 0` skip to the draw.
  2. Per channel ints: `dA = eA - sA`, `dR = eR - sR`, `dG = eG - sG`, `dB = eB - sB` (0x6FF59E-0x6FF5CA) converted with fild; s* also as
     floats. Per segment i (0..n-1, 0x6FF630-0x6FF73C): `t0 = (float)i / (float)n`, `t1 = (float)(i+1) / (float)n` (fild/fdiv);
     each vertex colour channel `c = _ftol(d_c * t + s_c)` in the order A, R, G, B, packed `A<<24 | R<<16 | G<<8 | B`; position
     `p = (end - start) * t + start` per component ((end - start) precomputed and stored f32, 0x6FF606-0x6FF626). Vertices written to
     TempBufferVertices 0xC4D958 (stride 0x24: pos +0, colour +0x18), two per segment.
  3. `LittleTest()`; `if (RwIm3DTransform(0xC4D958, 2n, NULL, 0)) { RwIm3DRenderIndexedPrimitive(1 /*LINELIST*/, 0x8D503C, 2n); RwIm3DEnd(); }`.
- Verdict: CONFIRMED - gr's CRGBA `lerp` is not the original's `_ftol(from + (to - from) * t)` per channel, so channels can differ by 1.
  Class: keep-original.
  Found in passing (CONFIRMED, keep-original): segment count uses `d * 0.4f` (gr `d / 2.5f`) and the distance sum order
  `(dz^2 + dy^2) + dx^2` - can change n at the boundaries.
