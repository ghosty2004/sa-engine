# CWaterLevel (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/WaterLevel.cpp` (pin 20d8d2a). Vertices `m_aVertices` 0xC22910 (stride 0x14: +0 x i16,
+2 y i16, +4 CRenPar), quads `WaterQuads` 0xC21C90 (stride 0xA: 4 x u16 vertex index, +8 u16 flags), triangles `WaterTriangles`
0xC22854 (stride 8: 3 x u16, +6 u16 flags). Counters NumWaterTriangles 0xC22884, NumWaterQuads 0xC22888, NumWaterVertices 0xC2288C.
CRenPar lerp helper 0x6E5280 (cdecl (out*, CRenPar a, CRenPar b, float t): every float field = `a * (1 - t) + b * t`, x87 order
`(1 - t)` first, bytes +0xC/+0xD of the result zeroed).

## 0x6E6EF0 CWaterLevel::CalculateWavesOnlyForCoordinate  (gr_deviations = 1)
Row WaterLevel.cpp:698 (`// Wave normal calculation - seems broken but maybe R* just knows something that we don't`).
- Original bytes (cdecl (i32 x, i32 y, float big, float small, float* outWave, float* colorMult, float* glare, CVector* n), plain `ret`):
  x, y made non-negative (`neg` if < 0, 0x6E6EF9/0x6E6F07); `m = WaveMulX[(x/2) & 7] (0x8D38C8) * WaveMulY[(y/2) & 7] (0x8D38E8) *
  CWeather::Wavyness (0xC812E8)`; fx = (float)x, fy = (float)y. Sin/cos via the 256-entry table 0xBB3E00: `idx = ftol(phase *
  40.743664f [0x85A778]) & 0xFF` for sin, `ftol(phase * 40.743664f + 64.0f [0x859A44]) & 0xFF` for cos. Steps use `(timeMs -
  m_nWaterTimeOffset 0xC228A4) % P` (unsigned div), converted with fild (+2^32 [0x858C54] if negative).
  1. P = 5000: `ph = s * 0.0012566370f [0x872020] + (fy + fx) * 0.09817477f [0x87201C]` (the coordinate SUM is multiplied once,
     0x6E6F6F-0x6E6F79). `*outWave += sin * 2.0f [0x871FF8] * m * big`; `d = -(cos * 2.0f * m * big * 0.09817477f)`; n = (d, d, 1.0)
     written directly (0x6E6FE1-0x6E6FFA).
  2. P = 3500: `ph = (s * 0.0017951959f [0x872018] + fy * 0.12083049f [0x872014]) + fx * 0.24166098f [0x872010]`;
     `*outWave += sin * 1.0f [0x871FFC] * m * small`; `d = cos * 1.0f * m * small * 0.24166098f`; n.x += d, n.y += d.
  3. P = 3000: `ph = s * 0.0020943952f [0x87200C] + fy * 0.31415927f [0x872008]` (x unused); `*outWave += sin * 0.5f [0x872000] * m *
     small`; n.x += cos * 0.5f * m * small * 0.31415927f.
  `CVector::Normalise(n)` 0x59C910. `g1 = ((n.z + n.y) + n.x) * 0.577f [0x872004]`; `*colorMult = max(g1, 0) * [0x8D3924] (.data,
  0.65f) + [0x8D3920] (.data, 0.27f)` (keeps g1 when g1 >= 0 or NaN). `g2 = (n.z * 0.577f + n.y * 0.577f) + 0.577f * n.x` (recomputed,
  distributed, 0x6E7182-0x6E719F), written to *glare first, then `t = g2 * 8.0f [0x859000] - 5.0f [0x858C80]`, clamped to
  [0, 0.99f [0x862CD0]] (NaN passes through), `*glare = t * CWeather::SunGlare (0xC812E0)`.
- Verdict for the row: REFUTED - gr reproduces the same "broken" normal ((d, d) for the first two waves, x only for the third).
  Class: harmless.
- Found in passing (CONFIRMED, keep-original; float-order differences that change the low bits):
  1. Phase 1 uses `(fy + fx) * c` (gr: `fX * wx + fY * wy`); phase 2 adds the y term before the x term (gr: x first).
  2. Table index: original multiplies by 40.743664f and adds 64.0 for cos; gr's CMaths::GetSinFast divides by the LUT step and
     GetCosFast adds PI/2 in radians before dividing.
  3. glare uses g2 (distributed products) while colorMult uses g1; gr uses one value `(x + y + z) * 0.577f` for both.
  4. colorMult constants are the mutable .data globals 0x8D3924 / 0x8D3920, not literals.

## 0x6E7D40 CWaterLevel::AddWaterLevelTriangle  (gr_deviations = 3)
Rows :914 (`// NOTSA: to verify the above.`), :915 / :916 (asserts; debug only).
- Original bytes (cdecl (X1, Y1, CRenPar P1, X2, Y2, P2, X3, Y3, P3, u32 flags), plain `ret`): return if `X1 == X2 && X1 == X3`
  (0x6E7D4A-0x6E7D54) or `Y1 == Y2 && Y1 == Y3` (0x6E7D62-0x6E7D6C). v0..v2 = AddWaterLevelVertex 0x6E5A40 (x, y, P) in order 1, 2, 3.
  Ordering uses the STORED vertex coords (i16, signed compares): if y(v0) == y(v1): (x(v0) < x(v1) ? v0, v1 : v1, v0), v2;
  elif y(v0) == y(v2): (x(v0) < x(v2) ? v0, v2 : v2, v0), v1; else (x(v1) < x(v2) ? v1, v2 : v2, v1), v0 (0x6E7DEF-0x6E7E73).
  Flags word +6: bit0 cleared, bit1 = !(flags & 1), bit2 = (flags & 2) != 0 (0x6E7E97-0x6E7ED7). NumWaterTriangles++.
- Verdict (all three): REFUTED - gr's ordering is the same partition (it also reads the stored i16 coords via SortableVtx). Class: harmless.
  (gr's CWaterTriangle constructor bit layout was not re-checked against the +6 bit assignments above.)

## 0x6E7EF0 CWaterLevel::AddWaterLevelQuad  (gr_deviations = 3)
Rows :887 (`// NOTSA: to verify the above.`), :888 / :889 (asserts).
- Original bytes: return if all four X equal (0x6E7EFB-0x6E7F10) or all four Y equal (0x6E7F1E-0x6E7F33). Four vertices added in
  order 1..4. Then (0x6E8033-0x6E8109): minY = the minimum stored y of the four (signed i16); each vertex in order v0, v1, v2, v3 is
  appended to list A if its y == minY, else to list B (A at [esp+0x1C], B at [esp+0x24], two slots each, no bounds check). Each list
  pair is then ordered by x (swap only if x[0] > x[1], `jle`, 0x6E812B / 0x6E8152). Quad = (A0, A1, B0, B1) (0x6E8167-0x6E817C). Flags
  as for triangles (+8 word). NumWaterQuads++.
- Verdict for :887: CONFIRMED (algorithm differs) - gr fully sorts the four vertices by (y, x) (DoVtxSortAndGetRange, rng::sort).
  Results are equal for 2 + 2 quads (two vertices on the minimum y line); with 1 or 3 vertices at minY the original's partition
  differs (and 3 or 4 at minY overflow list A into list B's slots). Class: keep-original (port the partition). Asserts: REFUTED, harmless.

## 0x6EAE80 CWaterLevel::WaterLevelInitialise  (gr_deviations = 1)
Row :155 (`// NOTSA: Code @ 0x6EB5F4` - LoadTextures split out of this function).
- Original bytes 0x6EB5F4-0x6EB681: PushCurrentTxd; SetCurrentTxd(FindTxdSlot("particle" 0x85C648)); for texWaterclear256 0xC228AC,
  texSeabd32 0xC228B4, texWaterwake 0xC228BC: if null -> RwTextureRead(name, NULL) 0x7F3AC0 and store; raster global (0xC228A8,
  0xC228B0, 0xC228B8) = `*(RwTexture*)` (+0 raster); tail `jmp` PopCurrentTxd 0x7316B0.
- Verdict: REFUTED - same code, only moved into a helper. Class: harmless.
- Found in passing (CONFIRMED, keep-original):
  1. The function starts by zeroing `m_BlockPolyInfo` 0xC21B70 (0x48 dwords, `rep stosd`, 0x6EAE8E-0x6EAE98); gr's
     WaterLevelInitialise does not (matters on re-initialisation).
  2. The file name is chosen by `m_nWaterConfiguration` 0xC228A0: 0 -> "DATA\water.dat" (0x872148), 1 -> "DATA\water1.dat"
     (0x872158); any other value leaves the stack buffer uninitialised (0x6EAEBC `jne 0x6EAF40`). gr uses water.dat for != 1.
  3. `OpenFile(name, "r")` is retried in an endless loop while the handle is < 0 (0x6EAF57 `jl 0x6EAF40`); gr opens once.
  4. The data-line parser between 0x6EAF80 and 0x6EB5D3 was not compared with gr's stringstream parser (out of the row's scope).

## 0x6EBEC0 CWaterLevel::RenderFlatWaterRectangle  (gr_deviations = 1)
Row :481 (`#ifdef FIX_BUGS` - gr splits along Y when `maxY - minY > BigPolySize`).
- Original bytes: entry is a .HOODLUM detour (0x6EBEC0 `jmp 0x4010CA`: `al = bSplitBigPolys [0x8D37F4]`, jmp back 0x6EBEC5).
  If bSplitBigPolys: `maxX - minX > 0xA8` (signed, 0x6EBEE0) -> X split at `(minX + maxX) / 2` (cdq/sar, truncation toward 0); else
  `Y2 - Y1 > 0xA8` (0x6EC1AB, unsigned order of the arguments, no min/max) -> Y split at `(Y1 + Y2) / 2`. Each split computes the
  mid CRenPar values with the lerp helper 0x6E5280 and the constant t = 0.5f (pushes 0x3F000000) and calls
  RenderFlatWaterRectangle (0x6EC03C, 0x6EC18F, 0x6EC303, 0x6EC453) on both halves. Otherwise layers 0 and 1 via
  RenderFlatWaterRectangle_OneLayer 0x6E9940 (0x6EC509, 0x6EC5B8).
- Verdict: CONFIRMED - with FIX_BUGS gr uses |Y2 - Y1|; the original tests the signed `Y2 - Y1` (#else branch). Class: keep-original.
- Found in passing (CONFIRMED gr LOGIC ERROR, keep-original): gr's split calls SplitWaterRectangleAlongXLine/YLine, which (a) use
  `t = (split - min) / (max - min)` instead of exactly 0.5 (differs when min + max is odd) and (b) recurse through
  RenderWaterRectangle, which can switch to the high-detail path; the original recurses into RenderFlatWaterRectangle only.

## 0x6ECF00 CWaterLevel::SplitWaterTriangleAlongXLine  (gr_deviations = 2)
Rows :366 (`assert(Y1 == Y2)`, debug only - REFUTED, harmless) and :437 (`NOTSA_UNREACHABLE("Triangle has no 90deg corner")`).
- Original bytes: .HOODLUM detour (0x6ECF05 `jmp 0x404CA0`: `eax = [esp+0x8C]` = X3, jmp 0x6ECF0A). `if (X1 == X3)` (0x6ECF1A,
  `jne 0x6ED2E6`) -> branch A (triangle 0x6ED094, rectangle 0x6ED1EA, triangle 0x6ED2D6); otherwise branch B (triangle 0x6ED47F,
  rectangle 0x6ED5D9, triangle 0x6ED6B8) WITHOUT testing `X2 == X3`. t = `(float)(XS - X1) / (X2 - X1)` (fild / fidiv,
  0x6ECF20/0x6ECF42); the split y is integer `(Y3 - Y1) * (XS - X1) / (X2 - X1)` (imul / idiv).
- Verdict for :437: CONFIRMED - where gr aborts (debug) or has undefined behaviour (release `__assume(false)`), the original runs
  branch B. Class: keep-original.
- Open question: the per-call argument lists of the six render calls were not re-derived (0x6ED000-0x6ED6B8); gr's branch B passes
  `P12` twice to RenderWaterRectangle (WaterLevel.cpp:419-423), which looks like a gr typo and must be checked against the pushes at
  0x6ED54C-0x6ED5D8 before porting this function from gr.

## 0x6ED9A0 CWaterLevel::RenderBoatWakes  (gr_deviations = 1)
Row Boat.cpp:342 (`// NOTSA: Moved from 0x6ED9A0` - gr's CBoat::RenderAllWakePointBoats).
- Original bytes: TEXTURERASTER = waterwakeRaster 0xC228B8 (0x6ED9A9-0x6ED9BB); CBoat::FillBoatList 0x6F2710; for i = 0..3
  (`cmp bx, 4`, 0x6EDD38) over CBoat::apFrameWakeGeneratingBoats 0xC27994, stopping at the first null (0x6ED9EC); the per-boat wake
  rendering is inlined (0x6ED9F2-0x6EDD2B, CWaterLevel::RenderWakeSegment 0x6EA260). Then if `uiTempBufferVerticesStored` 0xC4B950 != 0:
  Common::LittleTest 0x541330, `RwIm3DTransform(TempVertexBuffer 0xC4D958, n, NULL, 1)`, on success
  `RwIm3DRenderIndexedPrimitive(3, aTempBufferIndices 0xC4B958, uiTempBufferIndicesStored)` and RwIm3DEnd; both counters zeroed.
- Verdict: REFUTED - relocation only; the outer structure matches gr. Class: harmless. The inlined per-boat body was not compared
  line by line with gr's CBoat::RenderWakePoints (confidence medium).

## 0x6EE5A0 CWaterLevel::SplitWaterTriangleAlongYLine  (gr_deviations = 2)
- Row :443 (`if (DontRenderYSplitTri) return; // NOTSA`): no such test in the original (0x6EE5A0 starts with the computation). gr's
  flag defaults to false (WaterLevel.h:203). CONFIRMED, harmless (debug switch, off by default).
- Row :453 (`// Same result as original code, but much easier.`, deviates=no): the original does NOT delegate to
  SplitWaterTriangleAlongXLine. It computes `t = (float)(splitY - Y1) / (Y3 - Y1)` (0x6EE5B9-0x6EE5C7), branches on `Y1 >= Y3`
  (0x6EE5D7 -> 0x6EED72) and on a vertex equality (0x6EE5DF -> 0x6EEA48), and renders the pieces itself with 26 lerp calls (0x6E5280),
  5 RenderWaterTriangle (0x6EE240) and 3 RenderWaterRectangle (0x6EC5D0) calls. gr's single X-line split produces different pieces
  (different render calls, recursion and lerp parameters). Verdict: CONFIRMED (gr's claim is wrong). Class: keep-original; this
  function needs a full spec before it can be ported (1216 instructions, never executed in the coverage corpus).
