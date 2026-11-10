# CWaterLevel (RENDER) - full specs

Source for names: `re/gta-reversed/source/game_sa/WaterLevel.h` (pin 20d8d2a). All CWaterLevel members are static.
Water geometry: `m_aVertices` 0xC22910, CWaterVertex[1021], stride 0x14: +0 i16 x, +2 i16 y, +4 f32 z, +8 f32 bigWaves,
+0xC f32 smallWaves, +0x10 i8 flowX, +0x11 i8 flowY (flow is fixed point, /64 = *0.015625). `WaterQuads` 0xC21C90
(CWaterQuad, stride 0xA: u16 verts[4] +0..+6, flag byte +8 per gr bitfields: bit0 bToBeRendered, bit1 bInvisible, bit2 bLimitedDepth; not read by the part-1 functions),
`WaterTriangles` 0xC22854 (stride 8: u16 verts[3], flags +6). Counts: `NumWaterTriangles` 0xC22884, `NumWaterQuads`
0xC22888, `NumWaterVertices` 0xC2288C. Block grid: 12x12 blocks of 500 units covering -3000..3000;
`m_BlockPolyInfo` 0xC21B70 (u16[12][12], index blockX*12 + blockY; bits 14-15 type 0 none / 1 quad / 2 triangle /
3 combo, bits 0-13 id); `m_PolyCombos` 0xC215F8 (u16 entries in the same encoding, a run ends at an entry with type 0).
Waves: `faWaveMultipliersX` 0x8D38C8 / `faWaveMultipliersY` 0x8D38E8 (f32[8]), `m_nWaterTimeOffset` 0xC228A4,
sine LUT 0xBB3E00 (f32[256]). Flow: `m_CurrentFlow` 0xC22890, `m_CurrentDesiredFlow` 0xC22898 (CVector2D).
Fog: `ms_WaterFog` 0xC21188 (i16 minX[70] +0, minY[70] +0x8C, maxX[70] +0x118, maxY[70] +0x1A4, f32 z[70] +0x230),
`gWaterFogIndex` 0xC228E4, `gbPlayerIsInsideWaterFog` 0xC228E8, unnamed inside-fade f32 0xC228EC and timer f32 0xC228F0.
Several functions here are entered or continued through .HOODLUM stubs that load a global's address as
`*(dwordA ^ dwordB)`; each spec gives the resolved global.

## 0x6E7210 CWaterLevel::CalculateWavesOnlyForCoordinate2  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl void (int32 x [esp+4], int32 y [+8], float bigAmpl [+0xC], float smallAmpl [+0x10],
  float* pHeight [+0x14])`, plain `ret`. The arguments x and y are overwritten in place (|x|, |y|, then the float y).
  Mid-function `jmp 0x403A06` is a .HOODLUM stub: `pushfd; ecx = *( [0x1560788] ^ [0x4B0703] ); popfd; jmp 0x6E723A`;
  0xFF373C7D ^ 0xFFF514D9 = 0x00C228A4, i.e. ECX = `m_nWaterTimeOffset` (u32). A port reads the global directly.
- Callers: CWaterLevel::AddWaveToResult 0x6E81E0 (12 sites), 0x6E8550 (1 site). Callees: `_ftol` 0x821B40 (x3, truncation).
- Globals read: `faWaveMultipliersX` 0x8D38C8 (f32[8]), `faWaveMultipliersY` 0x8D38E8 (f32[8]), `CTimer::m_snTimeInMilliseconds`
  0xB7CB84 (u32), `m_nWaterTimeOffset` 0xC228A4, `CWeather::Wavyness` 0xC812E8 (f32), sine LUT 0xBB3E00 (f32[256], gr
  Maths.cpp `SIN_LUT`). Constants: 0x872020 = 0.00125663704 (2pi/5000), 0x87201C = 0.0981747732 (pi/32),
  0x85A778 = 40.7436638 (256/2pi), 0x871FF8 = 2.0, 0x872018 = 0.00179519586 (2pi/3500), 0x872014 = 0.120830491,
  0x872010 = 0.241660982, 0x871FFC = 1.0, 0x87200C = 0.00209439523 (2pi/3000), 0x872008 = 0.314159274, 0x872000 = 0.5,
  0x858C54 = 4294967296.0 (unsigned-fixup, dead because the remainders are < 5000).
- Behaviour:
  1. `if (x < 0) x = -x; if (y < 0) y = -y` (0x6E7210-0x6E7228; INT_MIN stays negative).
     `ix = ((x / 2) & 7)`, `iy = ((y / 2) & 7)` (signed division by `cdq; sub; sar 1`, 0x6E722C-0x6E7244).
  2. `W = faWaveMultipliersX[ix] * faWaveMultipliersY[iy] * Wavyness` (x87 register, 0x6E7249/0x6E7250/0x6E7265).
     `xf = (float)x` (register), `yf = (float)y` (stored f32 into the x argument slot [esp+4], 0x6E7273).
     `t1 = (u32)(m_snTimeInMilliseconds - m_nWaterTimeOffset) % 5000` (unsigned `div`, 0x6E725C-0x6E7263).
  3. Big wave: `a1 = (float)t1 * 0.00125663704 + (yf + xf) * 0.0981747732` (0x6E7289-0x6E729B);
     `s1 = SIN_LUT[_ftol(a1 * 40.7436638) & 0xFF]`; `r1 = s1 * 2.0 * W * bigAmpl + *pHeight` (0x6E72B1-0x6E72C9);
     `*pHeight = r1` (f32, r1 also kept as f32 local 0x6E72CB).
  4. Small wave 1: `t2 = (u32)(time - offset) % 3500`; `a2 = ((float)t2 * 0.00179519586 + yf * 0.120830491) + xf * 0.241660982`
     (0x6E72F4-0x6E730E); `r2 = SIN_LUT[_ftol(a2 * 40.7436638) & 0xFF] * 1.0 * W * smallAmpl + r1` (0x6E7320-0x6E7338);
     `*pHeight = r2` (fst f32).
  5. Small wave 2: `t3 = (u32)(time - offset) % 3000`; `a3 = (float)t3 * 0.00209439523 + yf * 0.314159274` (no x term,
     0x6E7361-0x6E7371); `*pHeight = SIN_LUT[_ftol(a3 * 40.7436638) & 0xFF] * 0.5 * W * smallAmpl + r2` (0x6E7383-0x6E7398;
     r2 is the unrounded register value, equal to the stored f32 under 24-bit precision).
- Edge cases: time and offset are re-read for each of the three remainders (0x6E72D1, 0x6E733E). The `& 0xFF` wraps the
  LUT index. The x87 stack is left balanced (two `fstp st(0)` at 0x6E739B).
- gta-reversed: `plugin::Call` only (WaterLevel.cpp:764). gr's declaration puts `pResultHeight` third, but its
  plugin::Call passes `(x, y, big, small, pHeight)`, which matches the bytes ([esp+0x18] after `push esi` is the pointer,
  0x6E72A8). Open questions: none.

## 0x6E7760 CWaterLevel::RenderWaterFog  (catalogue: specified/REA, port level 6)
- Convention / signature: `__cdecl void ()`; frame `sub esp, 0x58` + ebx/esi/edi(/ebp). Normal exit (0x6E7B0D) is a TAIL
  JUMP to CPostEffects::ImmediateModeRenderStatesReStore 0x700E00; the early exits (0x6E77B5, 0x6E7B18) are plain `ret`.
- Callers: Game::RenderEffects 0x53E170. Callees: CPostEffects::ImmediateModeRenderStatesStore 0x700CC0,
  ImmediateModeRenderStatesSet 0x700D70, ImmediateModeRenderStatesReStore 0x700E00, CPostEffects::DrawQuad 0x700EC0
  (cdecl, 9 args: x1, y1, x2, y2 floats, r, g, b, a, RwRaster*), RwIm3DTransform 0x7EF450, RwIm3DRenderPrimitive 0x7EF6B0,
  RwIm3DEnd 0x7EF520, `_ftol` 0x821B40 (truncation, x4), RwRenderStateSet through `RwEngineInstance` 0xC97B24 `+0x20` (x2).
- Globals read: `m_bWaterFog` 0x8D37D4 (u8, init 1), `m_bWaterFogScript` 0x8D37D5 (u8, init 1), `m_WaterFogDensity`
  0x8D37E0 (i32, init 26), `m_fWaterFogHeight` 0x8D37E4 (f32, init 12.0), fog colour 0x8D37E8 (r,g,b,a bytes, init
  FF FF FF 05; gr list name `m_WaterFogCol`), inside colour 0x8D37EC (init FF FF FF 28, `m_WaterFogInsideCol`),
  inside fade speed 0x8D37F0 = 0.04 (`m_fWaterFogInsideFadeSpeed`), `CWeather::UnderWaterness` 0xC8132C,
  `CPostEffects::m_fWaterFXStartUnderWaterness` 0x8D514C, `CWeather::WaterFogFXControl` 0xC81338, `CTimer::ms_fTimeStep`
  0xB7CB5C, `RsGlobal.maximumWidth` 0xC17044 / `maximumHeight` 0xC17048 (i32), `ms_WaterFog` 0xC21188 (minX i16[70] +0,
  minY +0x8C, maxX +0x118, maxY +0x1A4, z f32[70] +0x230). Read/written: `gWaterFogIndex` 0xC228E4 (u32),
  `gbPlayerIsInsideWaterFog` 0xC228E8 (u8), inside-fade 0xC228EC (f32, "fade"), inside-timer 0xC228F0 (f32, "timer")
  (both unnamed in gr). Written: `TempVertexBuffer` 0xC4D958 (RwIm3DVertex, stride 0x24: x +0, y +4, z +8, colour +0x18).
  Constants: 0x858624 = 1.0, 0x858B50 = 0.0, immediates 40.0f (0x42200000), 1.0f (0x3F800000).
- Behaviour:
  1. If `m_bWaterFog == 0` or `m_bWaterFogScript == 0` return (0x6E776D/0x6E7779; gWaterFogIndex NOT reset).
  2. If `!(UnderWaterness < m_fWaterFXStartUnderWaterness)` (0x6E7785 `test ah,1; je`; NaN counts as "<") ->
     `gWaterFogIndex = 0`, return. `n = _ftol((float)m_WaterFogDensity * WaterFogFXControl)` (fild i32 * f32, 0x6E7792);
     if `n == 0` -> `gWaterFogIndex = 0`, return.
  3. `nf = (float)n` (stored f32 0x6E77C2); `step = m_fWaterFogHeight / nf` (stored f32 0x6E77E1);
     `count = gWaterFogIndex` (0x6E77CD); `gWaterFogIndex = 0` (0x6E77DB).
  4. Inside-fog fade: if `gbPlayerIsInsideWaterFog != 0` (0x6E77BF): `fade = ms_fTimeStep * 0.04 [0x8D37F0] + fade` (stored);
     if `fade > 1.0` -> `fade = 1.0` (0x6E7805, equal/NaN keep); `timer = 40.0`. Else: `timer = timer - ms_fTimeStep` (stored);
     if `timer <= 0.0` (0x6E783C `test ah,0x41; jp`, NaN = skip): `timer = 0`, `fade = fade - ms_fTimeStep * 0.04` (stored),
     if `fade < 0.0` -> `fade = 0` (0x6E786E).
  5. `gbPlayerIsInsideWaterFog = 0` (0x6E7880, every frame; set again by SetUpWaterFog).
  6. If `fade > 0.0` (0x6E7886, `<=`/NaN skip to 7): `a = _ftol((float)insideCol.a * fade)`; `a = _ftol((float)a *
     WaterFogFXControl)` (0x6E7897-0x6E78BF); Store + Set states; `DrawQuad(0.0f, 0.0f, (float)RsGlobal.maximumWidth,
     (float)RsGlobal.maximumHeight, insideCol.r, insideCol.g, insideCol.b, a, NULL)` (0x6E78D0-0x6E78FE; r passed as the
     whole dword 0x8D37EC, the callee uses its low byte); ReStore. Then if `fade == 1.0` (0x6E7911 `test ah,0x44; jnp`)
     return (plain ret, no fog walls).
  7. Fog walls (0x6E7922): Store + Set states; RwRenderStateSet(6 = ZTESTENABLE, 1); RwRenderStateSet(1 = TEXTURERASTER, NULL).
     `layers = _ftol((1.0 - fade) * nf)` (0x6E7944-0x6E7964). `colour = (fogCol.a << 24) | (fogCol.r << 16) | (fogCol.g << 8) |
     fogCol.b` (built at 0x6E7A48-0x6E7A5F; fogCol re-read from 0x8D37E8 after each flush). `nv = 0`.
     For i = 0 .. count-1 (0x6E7984-0x6E7AD7; skipped entirely if `count <= 0`, signed): `z = ms_WaterFog.z[i]`;
     xs/ys from (float)(i16) minX/minY/maxX/maxY[i]; six vertices (two triangles) in this order:
     (minX,minY) (maxX,minY) (maxX,maxY) (minX,minY) (maxX,maxY) (minX,maxY) (local arrays at frame +0x28.. and +0x40..,
     0x6E79B7-0x6E7A0D). If `layers > 0` (0x6E7A11, signed): repeat `layers` times: write the six vertices
     (x, y, z, colour; other vertex fields untouched) to TempVertexBuffer[nv++]; when `nv == 0x7FE` (2046):
     `if (RwIm3DTransform(TempVertexBuffer, nv, NULL, 8)) { RwIm3DRenderPrimitive(3 = TRILIST); RwIm3DEnd(); }` and `nv = 0`
     (0x6E7A62-0x6E7A9B); after each layer `z = z + step` (stored f32, 0x6E7AA9).
  8. If `nv > 0` flush as above (0x6E7ADD-0x6E7B02). Tail-call ImmediateModeRenderStatesReStore.
- Edge cases: the vertex-batch flush is inside the per-vertex loop, so a layer may be split between batches (2046 = 341
  layers of 6 is exact, so triangles are never split). Fog alpha per layer is the constant fogCol.a; the layer COUNT
  carries the density. When fade == 1.0 the function returns before step 7 with states already restored.
- gta-reversed: `plugin::Call` only (WaterLevel.cpp:661). Open questions: none.

## 0x6E7B30 CWaterLevel::FillQuadsAndTrianglesList  (catalogue: specified/REA, port level 1)
- Convention / signature: `__cdecl void ()`, plain `ret`; frame `sub esp, 0x18` + ebx/ebp/esi/edi. The extent has two
  alignment gaps (0x6E7B5A-0x6E7B60, 0x6E7C3D-0x6E7C40) bridged by `jmp`s.
- Callers: CWaterLevel::WaterLevelInitialise 0x6EAE80. Callees: 0x6E5750 (gr list name
  `AddToQuadsAndTrianglesList(int32 blockX, int32 blockY, int32 polyIndex, uint32 type)`, cdecl, caller pops 0x10),
  called with type 1 (quad) and 2 (triangle).
- Globals read: `NumWaterQuads` 0xC22888, `NumWaterTriangles` 0xC22884, `WaterQuads` 0xC21C90 (u16 verts[4], stride 0xA),
  `WaterTriangles` 0xC22854 (u16 verts[3], stride 8), `m_aVertices` 0xC22910 (stride 0x14, +0 i16 x, +2 i16 y).
  Constants: 0x859A94 = 3000.0, 0x858B58 = 500.0.
- Behaviour:
  1. Outer loop X = 0, 500, .. 5500 (int, `cmp 0x1770; jl`, 0x6E7D29) with bx = 0..11 (ebp):
     `xMin = (float)X - 3000.0`, `xMax = 500.0 + xMin` (both kept on the x87 stack, 0x6E7B40-0x6E7B56).
  2. Inner loop Y = 0 .. 5500 step 500 with by = 0..11 (ebx, 0x6E7D02-0x6E7D15): `yMin = (float)Y - 3000.0` (stored f32
     0x6E7B73), `yMax = yMin + 500.0` (stored f32 0x6E7B7D).
  3. Quads (0x6E7B90-0x6E7C25), i = 0 .. NumWaterQuads-1 (count re-read after every quad, 0x6E7C1A; skipped if <= 0):
     with V0 = m_aVertices[q.verts[0]], V1 = [q.verts[1]], V2 = [q.verts[2]], the quad is added
     (`0x6E5750(bx, by, i, 1)`, 0x6E7C12) iff, tested in this order, `(float)V1.x > xMin` (0x6E7BA7, `test ah,0x41; jne`)
     && `(float)V0.x < xMax` (0x6E7BC8) && `(float)V2.y > yMin` (0x6E7BE8) && `(float)V0.y < yMax` (0x6E7C02).
     Strict compares; equality (shared edge) does not count.
  4. Triangles (0x6E7C40-0x6E7CFC), i = 0 .. NumWaterTriangles-1 (count read once, 0x6E7C2B): A = verts[0], B = verts[1],
     C = verts[2]; `yLo = min(A.y, C.y)` (i16 signed compare, 0x6E7C92), `yHi = max(A.y, C.y)` (0x6E7C9B); B.y is ignored.
     Added (`0x6E5750(bx, by, i, 2)`, 0x6E7CED) iff `(float)B.x > xMin` (0x6E7CB1) && `(float)A.x < xMax` (0x6E7CBA) &&
     `(float)yHi > yMin` (0x6E7CCE) && `(float)yLo < yMax` (0x6E7CDD).
- Edge cases: the bounding test assumes verts[0] holds the minimum x and verts[1] the maximum x (for triangles too); for
  triangles the y range only covers vertices 0 and 2. Quads are added in index order before triangles for every block.
- gta-reversed: `plugin::Call` only (WaterLevel.cpp:963). Open questions: none.

## 0x6E81E0 CWaterLevel::AddWaveToResult  (catalogue: specified/REA, port level 1)
- Convention / signature: `__cdecl void (float x [esp+4], float y [+8], float* pLevel [+0xC], float bigAmpl [+0x10],
  float smallAmpl [+0x14], CVector* pNormal [+0x18])`, plain `ret`. Frame `sub esp, 0x38` + ebx/ebp/esi/edi. The x and y
  argument slots are overwritten (y slot <- y*0.5 at 0x6E822A, x slot <- fy at 0x6E823A; on the upper-triangle + normal
  path the y slot is later overwritten with the int X0+2 at 0x6E8380). Locals below are named by offset from the entry
  esp E: h_a = E-0x38, h_b = E-0x34, h_c = E-0x30, fx = E-0x2C, xh = E-0x28.
- Callers: CWaterLevel::GetWaterLevel 0x6EB690. Callees: CRT `floor` 0x8219F0 (x4), `_ftol` 0x821B40 (x2),
  CalculateWavesOnlyForCoordinate2 0x6E7210 (3 calls per path, `(int X, int Y, big, small, float* acc)`, caller pops),
  0x59C730 CVector::CrossProduct (cdecl `(CVector* out, const CVector* a, const CVector* b)`, out = a x b, returns out),
  CVector::Normalise 0x59C910 (thiscall, ECX = pNormal).
- Constants: 0x858B8C = 0.5, 0x858624 = 1.0; immediates 2.0f (0x40000000), -2.0f (0xC0000000).
- Behaviour:
  1. `xh = x * 0.5` (stored f32, 0x6E81F4); `h_a = h_c = h_b = 0` (0x6E81FB-0x6E820B);
     `fx = xh - floor(xh)` (stored f32, 0x6E8218-0x6E821C); `yh = y * 0.5` (stored f32), `fy = yh - floor(yh)` (stored f32);
     `X0 = _ftol(floor(xh) + floor(xh))`, `Y0 = _ftol(floor(yh) + floor(yh))` (`fadd st0,st0`, 0x6E824A/0x6E825F) - X0/Y0
     are the even integer grid points.
  2. `s = fy + fx` compared with 1.0 (0x6E8269-0x6E8279). Branch first on `pNormal == NULL` (0x6E8281 `je`, flags of
     `test ebx,ebx`), then on `s < 1.0` (`test ah,5; jp`: `>=` or NaN takes the UPPER triangle).
  3. LOWER triangle (s < 1): `CalcWaves2(X0, Y0, big, small, &h_a)` = h00; `(X0+2, Y0, ..., &h_c)` = h10;
     `(X0, Y0+2, ..., &h_b)` = h01 (that call order, both paths 0x6E82A1/0x6E82B6/0x6E82CE and 0x6E8471/0x6E8482/0x6E8493).
     - With normal (0x6E82D3-0x6E832B): `dY = h01 - h00`, `dX = h10 - h00`;
       `*pLevel = ((fx * dX + dY * fy) + *pLevel) + h00` (that x87 order; stored f32).
       `pNormal = CrossProduct((2.0, 0, dX), (0, 2.0, dY))` (locals E-0x18 and E-0x24, 0x6E82FA-0x6E833B), copied dword-wise to
       *pNormal, then `pNormal->Normalise()`.
     - Without normal (0x6E8498-0x6E84AF, tail 0x6E852E): `*pLevel = (((h01 - h00) * fy + (h10 - h00) * fx) + *pLevel) + h00`.
  4. UPPER triangle (s >= 1 or NaN): `CalcWaves2(X0+2, Y0+2, ..., &h_a)` = h11; `(X0, Y0+2, ..., &h_c)` = h01;
     `(X0+2, Y0, ..., &h_b)` = h10 (0x6E8384/0x6E839A/0x6E83BA and 0x6E84CC/0x6E84E2/0x6E84FE).
     - With normal (0x6E83BF-0x6E8421): `a = h10 - h11`, `b = h01 - h11`;
       `*pLevel = (((1.0 - fx) * b + (1.0 - fy) * a) + *pLevel) + h11`.
       `pNormal = CrossProduct((-2.0, 0, b), (0, -2.0, a))` (locals E-0x24 and E-0x18), copy, Normalise.
     - Without normal (0x6E8503-0x6E8544): `*pLevel = (((h10 - h11) * (1.0 - fy) + (h01 - h11) * (1.0 - fx)) + *pLevel) + h11`.
  The cross products evaluate to (-2dX, -2dY, 4) and (2b, 2a, 4) before normalisation, computed by 0x59C730 as
  `(a.y*b.z - a.z*b.y, a.z*b.x - a.x*b.z, a.x*b.y - a.y*b.x)`.
- Edge cases: the three accumulators are zeroed before the calls because CalcWaves2 ADDS to *acc. Grid spacing is 2 units.
  NaN x/y lead to `_ftol` = 0x80000000 and the upper triangle. The result always adds to the incoming *pLevel.
- gta-reversed: `plugin::Call` only (WaterLevel.cpp:197-200), same argument order. Open questions: none.

## 0x6E8580 CWaterLevel::GetWaterLevelNoWaves  (catalogue: specified/REA, port level 1)
- Convention / signature: `__cdecl bool (CVector pos by value [esp+4]=x, [+8]=y, [+0xC]=z, float* pOutLevel [+0x10],
  float* pOutBigWaves [+0x14], float* pOutSmallWaves [+0x18])`, result in AL (0/1), plain `ret`. Prologue `push ecx`
  (one scratch dword at [esp+0x10] after the 4 register pushes) + ebx/ebp/esi/edi.
- Callers (15): CRunningScript::ProcessCommands2300To2399 0x4762D0, CCam::ProcessPedsDeadBaby 0x519250,
  CCamera::TryToStartNewCamMode 0x51E560 (4 sites), CEntity::PreRender 0x535FA0, CFireManager::StartFire 0x539F00,
  CTaskSimpleDead::ProcessPed 0x630600, CHeli::SendDownSwat 0x6C69C0, CWaterLevel::GetWaterDepth 0x6EA960,
  CWaterLevel::SetUpWaterFog 0x6EA9F0, CWaterLevel::GetWaterLevel 0x6EB690, CShadows::RenderStoredShadows 0x70A960,
  CBirds::Update 0x712330.
- Callees: CRT `floor` 0x8219F0 (double on stack -> ST0, x2), `_ftol` 0x821B40 (truncation, x2),
  TestQuadToGetWaterLevel 0x6E5BB0 and TestTriangleToGetWaterLevel 0x6E5E90 (both `__cdecl bool (poly*, float x, float y,
  float z, float* pLevel, float* pBig, float* pSmall)`, AL, caller pops 0x1C).
- Globals read: `m_BlockPolyInfo` 0xC21B70 (u16[12][12], row = blockX), `WaterQuads` 0xC21C90 (stride 0xA),
  `WaterTriangles` 0xC22854 (stride 8), `m_PolyCombos` 0xC215F8 (u16[]). Constants: 0x858F44 = 0.002 (f32), 0x858B44 = 6.0.
- Behaviour:
  1. `bx = _ftol(floor((double)(x * 0.002f + 6.0f)))` (0x6E8581-0x6E85A0: fld x, fmul [0x858F44], fadd [0x858B44], fstp
     qword, floor, ftol). `by` likewise from y (0x6E85A5-0x6E85C2).
  2. If `bx < 0 || bx >= 12 || by < 0 || by >= 12` (signed, 0x6E85C7-0x6E85E3) -> outside path (0x6E8733):
     `*pOutLevel = 0` (written WITHOUT a null check, 0x6E873D), `if (pOutBigWaves) *pOutBigWaves = 1.0f` (0x6E8745),
     `if (pOutSmallWaves) *pOutSmallWaves = 0` (0x6E8753); return 1.
  3. `info = m_BlockPolyInfo[bx*12 + by]` (u16, 0x6E85E9-0x6E85EF); `type = info >> 14`, `id = info & 0x3FFF`;
     jump table 0x6E8764 = {0x6E872B, 0x6E860C, 0x6E864D, 0x6E868B}:
     - type 0 (NONE): return 0 (0x6E872B).
     - type 1: return `TestQuad(&WaterQuads[id], x, y, z, pOutLevel, pOutBigWaves, pOutSmallWaves) != 0` (0x6E863A).
     - type 2: return `TestTriangle(&WaterTriangles[id], ...same args) != 0` (0x6E8678).
     - type 3 (COMBO, 0x6E868B): `p = &m_PolyCombos[id]`; if `(p[0] >> 14) == 0` return 0 (0x6E8696/0x6E86A5).
       Loop (0x6E86C0): `e = *p; t = e >> 14`; t == 1 -> TestQuad(&WaterQuads[e & 0x3FFF], args); t == 2 ->
       TestTriangle(&WaterTriangles[e & 0x3FFF], args); t == 3 is skipped (no call). If the called test returned nonzero
       -> return 1 (0x6E8718 -> 0x6E8759). `p++`; continue while `(p[0] >> 14) != 0` (0x6E8721); then return 0.
- Edge cases: the `ja 0x6E8692` default at 0x6E85FF is unreachable (type <= 3); it would use the raw float bits of y
  ([esp+0x1C]) as combo index. NaN coordinates: floor/ftol give 0x80000000 -> outside path. Ties at block borders
  are decided by floor. The outside path dereferences pOutLevel unconditionally (callers always pass it).
- gta-reversed: `plugin::CallAndReturn` only (WaterLevel.cpp:650); gr's default args (nullptr) for the two wave
  pointers match the null checks at 0x6E8737/0x6E874F. Open questions: none.

## 0x6E9D70 CWaterLevel::FindNearestWaterAndItsFlow  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl void ()`, plain `ret`. Entry `jmp 0x406B2A` is a .HOODLUM protection stub:
  `pushfd; eax = *( [0x54A2C1] ^ [0x1560844] ); popfd; jmp 0x6E9D75`; the xor of the two code/data dwords
  (0xEBF489E9 ^ 0xEB4279D5) is 0x00B6F03C = `TheCamera.m_matrix` (TheCamera 0xB6F028 + 0x14), so on entry
  EAX = TheCamera.m_matrix (CMatrix*). A port reads TheCamera.m_matrix directly. Frame `sub esp, 0x20`.
- Callers (3): CEntryExit::TransitionFinished 0x4404A0, CGameLogic::RestorePlayerStuffDuringResurrection 0x442060,
  CWaterLevel::PreRenderWater 0x6EB710. Callees: none.
- Globals: read `NumWaterQuads` 0xC22888 (i32), `WaterQuads` 0xC21C90 (CWaterQuad, stride 0xA: u16 verts[4]),
  `m_aVertices` 0xC22910 (stride 0x14: +0 i16 x, +2 i16 y, +4 f32 z, +8 f32 bigWaves, +0xC f32 smallWaves,
  +0x10 i8 flowX, +0x11 i8 flowY), camera position. Written: TheCamera+0xE4 (0xB6F10C) and TheCamera+0xE8 (0xB6F110)
  (gr CCamera `m_fDistanceToWater` / `m_fHeightOfNearestWater`, consecutive floats at Camera.h:177-178),
  `m_CurrentDesiredFlow` 0xC22898 / 0xC2289C (CVector2D).
  Constants: 0x859A90 = -3000.0, 0x859A94 = 3000.0, 0x858B50 = 0.0, 0x85A7D4 = 0.015625, immediate 0x4B189680 = 10000000.0f.
- Behaviour:
  1. `P = matrix ? &matrix->pos (+0x30) : &TheCamera.placement.pos (0xB6F02C)`; `cx = P.x`, `cy = P.y` (0x6E9D78-0x6E9DA5).
     Locals: `bestWavyDist = 1e7f`, `bestDist = 1e7f`, `wavyZ = 0` (0x6E9DA9-0x6E9DB9).
  2. Range: go to the reset path if `cx <= -3000` (0x6E9D9F, `test ah,0x41; jnp`), `cx >= 3000` (0x6E9DD0, `test ah,1; je`),
     `cy <= -3000` (0x6E9DE5), `cy >= 3000` (0x6E9DFA). NaN passes all four checks. Reset path (0x6EA228):
     cam+0xE4 = 0, cam+0xE8 = 0, `m_CurrentDesiredFlow.y = 0`, `.x = 0`; return.
  3. If `NumWaterQuads <= 0` (signed, 0x6E9E12) skip to step 5. For each quad q in WaterQuads[0 .. NumWaterQuads-1]
     (forward, 0x6E9E23 .. 0x6EA206; no flag checks, triangles ignored); V0..V3 = m_aVertices[q.verts[0..3]]:
     a. `x0 = (float)V0.x` stored f32; if `cx < x0` (0x6E9E43, `test ah,5; jp` = taken when >= or NaN):
        `dx = x0 - cx`; else `x1 = (float)V1.x` (register): if `cx > x1` (0x6E9E73, `test ah,0x41; jne` -> 0 when cx <= x1
        or NaN) `dx = cx - x1` else `dx = 0.0`.
     b. Same for y with `y0 = (float)V0.y` (stored f32) and the max from `V2.y` (verts[2], 0x6E9EB6):
        `dy = y0 - cy` / `cy - y2` / 0.0.
     c. `dist = sqrt(dy*dy + dx*dx)` (0x6E9EE8-0x6E9EF2, kept in ST0, not stored).
     d. If `dist < bestWavyDist` (0x6E9EF8 fcom, NaN/>= skip to e): if ANY of V0.bigWaves, V0.smallWaves, V1.big,
        V1.small, V2.big, V2.small, V3.big, V3.small (that order, 0x6E9F07-0x6E9FB8) is != 0.0 (`test ah,0x44; jp`:
        unequal or NaN counts as nonzero): `bestWavyDist = dist` (fst f32, 0x6E9FCB), `wavyZ = V0.z` (raw dword, 0x6E9FC5).
     e. If `dist < bestDist` (0x6E9FD3, else pop and next quad): `bestDist = dist` (fstp f32, 0x6E9FE2). Squared
        distances to the four corners: `d0 = (cx - x0)^2 + (cy - y0)^2` (x0/y0 = the stored f32 values; x term first,
        0x6E9FEA-0x6EA01A, kept in ST0), `d1 = (cx - (float)V1.x)^2 + (cy - (float)V1.y)^2`
        (x term + y term, stored f32 0x6EA05A), `d2` with V2 (stored f32 0x6EA09C), `d3` with V3 (stored f32 0x6EA0C2).
        Corner pick (strict `<`, NaN = false): if `d0 < d1 && d0 < d2 && d0 < d3` -> V0 (0x6EA0CA-0x6EA0E9); else if
        `d1 < d2 && d1 < d3` -> V1 (0x6EA128-0x6EA144); else if `d2 < d3` -> V2 (0x6EA181); else V3.
        `m_CurrentDesiredFlow.x = (float)(i8)Vc.flowX * 0.015625f`, `.y = (float)(i8)Vc.flowY * 0.015625f` (fstp f32 each).
  4. Next quad (`edi += 0xA`, count down).
  5. cam+0xE4 = bestWavyDist (1e7 when no wavy quad was nearer), cam+0xE8 = wavyZ (0x6EA210-0x6EA21E). Return.
- Edge cases: `m_CurrentDesiredFlow` is only written when some quad improves bestDist (always the case for the first
  quad unless dist is NaN), and it is NOT reset in the in-range path. d0 is compared in extended precision against
  f32-stored d1..d3 (24-bit precision control makes the mantissa identical; exponent range differs only for overflow).
  dx/dy use quad corners verts[0] (min), verts[1] (max x), verts[2] (max y): assumes axis-aligned quads.
- gta-reversed: `plugin::Call` only (WaterLevel.cpp:645). Open questions: none.

## 0x6EA960 CWaterLevel::GetWaterDepth  (catalogue: specified/REA, port level 9)
- Convention / signature: `__cdecl bool (const CVector* pos [esp+4], float* pOutDepth [+8], float* pOutWaterLevel [+0xC],
  float* pOutGroundLevel [+0x10])`, AL, plain `ret`. Prologue `push ecx` (local `ground`) + `push esi` (esi = pos).
  The pos argument slot itself is reused as the f32 local `level` (`lea eax, [esp+0x14]` at 0x6EA970 = entry [esp+4]).
- Callers: WaterCreatureManager_c::Update 0x6E4F10. Callees: CWaterLevel::GetWaterLevelNoWaves 0x6E8580,
  0x6EA8A0 (unnamed; gr missing-list `GetGroundLevel(const CVector&, float*, ColData*, float)`: cdecl, AL; it calls
  CWorld::ProcessVerticalLine 0x5674E0 from `pos.z + range` and writes the hit z to *out on success).
- Constants: 0x859014 = -100.0, immediate 0x41F00000 = 30.0f.
- Behaviour:
  1. `if (!GetWaterLevelNoWaves(pos->x, pos->y, pos->z, &level, NULL, NULL)) return 0` (CVector pushed by value,
     0x6EA966-0x6EA984).
  2. `ok = 0x6EA8A0(pos, &ground, NULL, 30.0f)` (0x6EA986-0x6EA993); `g = ok ? ground : -100.0` (x87, 0x6EA99B-0x6EA9A7).
  3. `if (pOutDepth) *pOutDepth = level - g` (0x6EA9B3-0x6EA9B9); `if (pOutWaterLevel) *pOutWaterLevel = level` (raw dword
     copy, 0x6EA9C7); `if (pOutGroundLevel) *pOutGroundLevel = g` (0x6EA9D1); return 1.
- Edge cases: when no ground is found the depth is measured to -100.0. `ground` is uninitialised only on the failure path,
  where it is not read.
- gta-reversed: `plugin::CallAndReturn` only (WaterLevel.cpp:655-657), same argument order. Open questions: none.

## 0x6EB810 CWaterLevel::RenderHighDetailWaterRectangle  (catalogue: specified/REA, port level 9)
- Convention / signature: `__cdecl void (int32 minX [esp+4], int32 maxX [+8], int32 Y1 [+0xC], int32 Y2 [+0x10],
  CRenPar P1 [+0x14], CRenPar P2 [+0x24], CRenPar P3 [+0x34], CRenPar P4 [+0x44])`, plain `ret` (0x6EBB07 / 0x6EBBDB /
  0x6EBEBA; caller pops 0x50). CRenPar = 16 bytes by value (+0 z, +4 bigWaves, +8 smallWaves, +0xC/+0xD i8 flowX/flowY, 2 pad).
  P1 = (minX,Y1), P2 = (maxX,Y1), P3 = (maxX,Y2), P4 = (minX,Y2) (gr naming). Registers after the prologue
  (0x6EB810-0x6EB823): EDI = minX, EBX = maxX, ESI = Y1, EBP = Y2. Argument slots minX (+0), maxX (+4), Y1 (+8) and P1 (+0x10)
  are reused as scratch; Y2's slot (+0xC) and P2..P4 stay intact and are re-read from the stack.
- Callers: CWaterLevel::RenderWaterRectangle 0x6EC5D0 (1). Callees: CCamera::IsSphereVisible 0x420C40 (x2, thiscall on
  TheCamera, `(const CVector&, float, RwMatrix*)`, `ret 0xC`), CWaterLevel::SetUpWaterFog 0x6EA9F0 (cdecl 4 ints),
  sub_6E91D0 (per-layer high detail rectangle renderer, gr's missing `RenderHighDetailWaterRectangle_OneLayer`, cdecl 13 args,
  x2), CWaterLevel::SplitWaterRectangleAlongXLine 0x6E73A0, lerp 0x6E5280 (`CRenPar* __cdecl (CRenPar* out, CRenPar a,
  CRenPar b, float t)`, returns `out` in EAX, caller pops 0x28; x4), CWaterLevel::RenderWaterRectangle 0x6EC5D0 (x2, recursion).
- Globals: TheCamera 0xB6F028, +0x970 (0xB6F998, u8 gr `m_bMirrorActive`), +0xA4C (0xB6FA74, gr `m_mMatInverse`),
  +0xA94 (0xB6FABC, gr `m_mMatMirrorInverse`). Constant 0x858B8C = 0.5f.
- Behaviour:
  1. Sphere (local CVector C at [esp+0x14] after the pushes, 0x6EB827-0x6EB896):
     `C.x = (float)(minX + maxX) * 0.5f` (fild of the int sum, stored f32 0x6EB849);
     `C.y = (float)(Y1 + Y2) * 0.5f` (stored f32 0x6EB86E); `C.z = P1.z` (raw dword, 0x6EB857/0x6EB866);
     `a = (float)(maxX - minX) * 0.5f`, `b = (float)(Y1 - Y2) * 0.5f` (both kept in x87, 0x6EB872-0x6EB880);
     `r = sqrt(b*b + a*a)` (0x6EB886-0x6EB890: b*b first, then a*a, `faddp`, `fsqrt`), stored f32 0x6EB896.
  2. `if (!TheCamera.IsSphereVisible(C, r, &m_mMatInverse))` (0x6EB8A5): return if `m_bMirrorActive == 0` (0x6EB8B5), else
     return if `!TheCamera.IsSphereVisible(C, r, &m_mMatMirrorInverse)` (0x6EB8CF/0x6EB8D6).
  3. `minY = min(Y1, Y2)`, `maxY = max(Y1, Y2)` (`cmp esi, ebp; jge` 0x6EB8DC: Y1 < Y2 -> minY = Y1, else minY = Y2; ties take Y2).
     `nx = (maxX - minX) / 2`, `ny = (maxY - minY) / 2` (signed, truncating toward zero, 0x6EB8EE-0x6EB90A).
     `tris = 2 * nx * ny` (0x6EB90C-0x6EB911); `verts = (nx + 1) * (ny + 1)` (0x6EB917-0x6EB91B).
  4. If `3 * tris >= 0x1000` (signed, 0x6EB925) or `verts >= 0x800` (signed, 0x6EB935) -> split (step 6).
  5. Render (0x6EB940-0x6EBB07): `SetUpWaterFog(minX, minY, maxX, maxY)` (0x6EB948); then
     `sub_6E91D0(minX, maxX, Y1, Y2, P1, P2, P3, P4, layer, tris, verts, nx, ny)` with layer 0 (0x6EBA1D) then layer 1
     (0x6EBAF8). Note the rectangle passed on is the original (minX, maxX, Y1, Y2), not the sorted Y pair. Return.
  6. Split (0x6EBB08): if `nx > ny` (signed, `cmp eax, ebp; jle` 0x6EBB0C):
     `SplitWaterRectangleAlongXLine(minX + 2 * (nx / 2), minX, maxX, Y1, Y2, P1, P2, P3, P4)` (0x6EBBCC; nx/2 truncating), return.
  7. Else (0x6EBBDC, inlined Y split): `splitY = minY + 2 * (ny / 2)` (0x6EBBDC-0x6EBBE3);
     `t = (float)(splitY - Y1) / (float)(Y2 - Y1)` (`fild` then `fidiv`, 0x6EBBF4/0x6EBC02, stored f32 0x6EBC06);
     `P24 = lerp(P2, P4, t)` (0x6EBC5F, result buffer = local sphere area), `P13 = lerp(P1, P3, t)` (0x6EBCBC);
     `RenderWaterRectangle(minX, maxX, Y1, splitY, P1, P2, P13, P24)` (0x6EBD4C);
     then the lerps are recomputed in the same order: `P24 = lerp(P2, P4, t)` (0x6EBDB3), `P13 = lerp(P1, P3, t)` (0x6EBE10,
     written into the P1 argument slot - P1 is not used afterwards);
     `RenderWaterRectangle(minX, maxX, splitY, Y2, P13, P24, P3, P4)` (0x6EBEAB). Return.
- Edge cases: the inlined Y split measures `t` from Y1 with denominator `Y2 - Y1` (correct for either Y order), while
  `splitY` is taken from minY. Split points use doubly truncated halves, so they are even offsets from minX/minY, not exact
  midpoints when nx/ny are odd. The sphere radius uses half the extents (bounding circle of the rectangle) but the sphere
  centre z is P1.z only.
- gta-reversed: `plugin::Call<0x6EB810>` only (WaterLevel.cpp:520, `.Reversed = false`). gr's standalone
  `SplitWaterRectangleAlongYLine` (0x6ED6D0, WaterLevel.cpp, commented "fully inlined into RenderWaterRectangle") computes
  `t = (splitAtY - minY) / (maxY - minY)`; the copy inlined here uses `(splitY - Y1) / (Y2 - Y1)`, which differs from gr when
  Y1 > Y2 (port the bytes; gr's helper must not be reused for this path).
- Open questions: none affecting behaviour (camera field names inferred from gr member order, see 0x6EDDC0).

## 0x6EDDC0 CWaterLevel::RenderHighDetailWaterTriangle  (catalogue: specified/REA, port level 10)
- Convention / signature: `__cdecl void (int32 X1 [esp+4], int32 Y1 [+8], CRenPar P1 [+0xC..+0x1B], int32 X2 [+0x1C],
  int32 Y2 [+0x20], CRenPar P2 [+0x24..+0x33], int32 X3 [+0x34], int32 Y3 [+0x38], CRenPar P3 [+0x3C..+0x4B])`, plain `ret`
  (0x6EE07F, caller pops 0x48). CRenPar = 16 bytes passed by value: +0 f32 z, +4 f32 bigWaves, +8 f32 smallWaves,
  +0xC i8 flowX, +0xD i8 flowY, +0xE/+0xF padding (copied as one dword).
  Register roles after the prologue (0x6EDDC0-0x6EDDDB): EDI = X1, EBP = X2, EBX = X3 (until 0x6EDE33), ESI = Y3 (until 0x6EDDFE).
  The X1 argument slot is reused as scratch (sums, then the vertex count) - harmless for a port.
- Callers: CWaterLevel::RenderWaterTriangle 0x6EE240 (1 site). Callees: CCamera::IsSphereVisible(const CVector&, float,
  RwMatrix*) 0x420C40 (thiscall on TheCamera 0xB6F028, `ret 0xC`, x2), sub_6E8780 (per-layer high detail triangle renderer,
  gr's missing `RenderHighDetailWaterTriangle_OneLayer`, cdecl 13 args, x2), CWaterLevel::SplitWaterTriangleAlongXLine 0x6ECF00.
- Globals: TheCamera 0xB6F028; TheCamera+0x970 (0xB6F998, u8, gr `m_bMirrorActive`); TheCamera+0xA4C (0xB6FA74, gr
  `m_mMatInverse`); TheCamera+0xA94 (0xB6FABC, gr `m_mMatMirrorInverse`). Constants 0x8594EC = 0.33333331f, 0x872168 = 0.71f.
- Behaviour:
  1. Visibility sphere (local CVector C at [esp+0x10] after the 4 pushes):
     `C.x = (float)(X1 + X2 + X3) * 0.33333331f` (int sum, `fild` 0x6EDDE8, `fmul [0x8594EC]` 0x6EDDF4, stored f32 0x6EDE02);
     `C.y = (float)(Y1 + Y2 + Y3) * 0.33333331f` (0x6EDE0B/0x6EDE1B, stored 0x6EDE21);
     `C.z = P1.z` (raw dword copy, 0x6EDDEC/0x6EDE17 - only the first vertex's z, not the average);
     `r = (float)(X2 - X1) * 0.71f` (int difference, `fild` 0x6EDE25, `fmul [0x872168]`, stored f32 0x6EDE2F).
  2. `if (!TheCamera.IsSphereVisible(C, r, &TheCamera.m_mMatInverse))` (0x6EDE3E): if `m_bMirrorActive == 0` return (0x6EDE4E);
     else `if (!TheCamera.IsSphereVisible(C, r, &TheCamera.m_mMatMirrorInverse))` return (0x6EDE64/0x6EDE6B).
  3. `n = (X2 - X1) / 2` (signed, truncating toward zero: cdq/sub/sar 0x6EDE71-0x6EDE78). `nn = n * n` (0x6EDE7E).
     `verts = 0; for (k = 1; k <= n + 1; k++) verts += k;` (loop 0x6EDE89-0x6EDE96; `verts = 0` if `n + 1 < 1`), i.e.
     `(n+1)(n+2)/2` for n >= 0.
  4. If `3 * nn >= 0x1000` (signed, 0x6EDE9F) or `verts >= 0x800` (signed, 0x6EDEAA) -> split (step 6).
  5. Render (0x6EDEB6-0x6EDFEE): `sub_6E8780(X1, Y1, P1, X2, Y2, P2, X3, Y3, P3, layer, nn, verts, n)` called twice, first with
     `layer = 0` (push 0 at 0x6EDEC1, call 0x6EDF43), then with `layer = 1` (0x6EDF5D, call 0x6EDFDF). Return.
  6. Split (0x6EDFEF-0x6EE075): `splitX = X1 + 2 * (n / 2)` (n halved again with truncation toward zero, 0x6EE063-0x6EE06B);
     `SplitWaterTriangleAlongXLine(splitX, X1, Y1, P1, X2, Y2, P2, X3, Y3, P3)` (0x6EE070), return.
- Edge cases: `n` and `splitX` use truncating signed division, so for odd `n` the split is not at the midpoint
  (e.g. X2 - X1 = 6: n = 3, splitX = X1 + 2). If X2 < X1 the radius is negative and `n` negative (verts = 0, nn > 0).
  The function never checks Y extents; it assumes the X1/X2 edge spans the triangle's width.
- gta-reversed: `plugin::CallAndReturn<..., 0x6EDDC0>` only (WaterLevel.cpp:802, `.Reversed = false`); the per-layer callee
  is listed as missing (`RenderHighDetailWaterTriangle_OneLayer(int32, int32, CRenPar, ..., int32, int32, int32, int32)`).
  gr does not name the three camera fields used here; the offsets match gr's `IsSphereVisibleInMirror` pattern
  (Camera.h:517-518).
- Open questions: none affecting behaviour (camera field names at 0xB6F998/0xB6FA74/0xB6FABC are inferred from gr's member
  order, not from VALIDATE_OFFSET).

## 0x6EF650 CWaterLevel::RenderWater  (catalogue: specified/REA, port level 11)
- Convention / signature: `__cdecl void ()`. Frame `sub esp, 0x5C` + 4 pushes. Exits: early `ret` (0x6F00CA) when water is
  not visible; normal path ends with a tail `jmp App::DefinedState 0x734650` (0x6F00BE). The entry contains a .HOODLUM
  detour (0x6EF66E `jmp 0x407EE4`): `pushfd; eax = *(*(u32*)0x15608B0 + *(u32*)0x42952A); popfd; jmp 0x6EF673`;
  0xC49EC4 + 0xFFFD89EC = 0xC228B0, so EAX = `CWaterLevel::seabd32Raster` (0xC228B0, RwRaster*). Behaviourally a plain load.
- Callers: Game::RenderScene 0x53DF40 (2 sites).
- Callees: CGame::CanSeeWaterFromCurrArea 0x53C4B0, CWaterLevel::SetCameraRange 0x6E9C80, App::DefinedState 0x734650,
  RwRenderStateSet via `RwEngineInstance(0xC97B24)->+0x20` (cdecl (state, value), caller pops), sub_6E6870 (flat sea-bed
  segment, gr missing `RenderSeaBedSegment(int32, int32, float, float, float, float)`), sub_6E6A10 (detailed sea-bed segment,
  gr missing `RenderDetailedSeaBedSegment`), Common::LittleTest 0x541330, RwIm3DTransform 0x7EF450, RwIm3DRenderIndexedPrimitive
  0x7EF550, RwIm3DEnd 0x7EF520, CRT `rand` 0x821B1E (x2), CRT `_ftol` 0x821B40 (truncation, x12),
  CWaterLevel::RenderWaterTriangle 0x6EE240, CWaterLevel::RenderWaterRectangle 0x6EC5D0 (x2), CWaterLevel::RenderBoatWakes 0x6ED9A0.
- Globals read: TheCamera m_matrix (0xB6F03C, TheCamera+0x14) / placement pos (0xB6F02C); CWaterLevel::m_NumBlocksOutsideWorldToBeRendered
  0xC215EC (u32, compared signed), m_BlocksToBeRenderedOutsideWorldX 0xC21560 / ...Y 0xC214D0 (int16[70]); NumWaterTriangles 0xC22884,
  WaterTriangles 0xC22854 (stride 8: +0/+2/+4 int16 vertex indices, +6 u8 flags, bit0 = bToBeRendered), NumWaterQuads 0xC22888,
  WaterQuads 0xC21C90 (stride 0xA: +0..+6 four int16 vertex indices, +8 flags bit0), m_aVertices 0xC22910 (stride 0x14: +0 int16 x,
  +2 int16 y, +4 CRenPar 16 bytes); CTimer::ms_fTimeStep 0xB7CB5C, CTimer::m_snTimeInMilliseconds 0xB7CB84, m_CurrentFlow 0xC22890 (x)
  / 0xC22894 (y), CWeather::Wavyness 0xC812E8, CTimeCycle::m_CurrentColours water RGBA floats 0xB7C508/0xB7C50C/0xB7C510/0xB7C514
  (gr m_fWaterRed/Green/Blue/Alpha), waterclear256Raster 0xC228A8, seabd32Raster 0xC228B0, u8 0xC228DC (unnamed debug flag, see
  edge cases), the TempBuffer globals uiTempBufferVerticesStored 0xC4B950, uiTempBufferIndicesStored 0xC4B954, TempVertexBuffer
  0xC4D958, aTempBufferIndices 0xC4B958.
  Mutable .data tunables (initial values from the image): f32 0x8D3934 = 600.0 (sea-bed detail distance), f32 0x8D392C = 0.04
  (flow scale), u32 0x8D3930 = 1 (texture address mode), f32 0x8D3928 = 0.01 (random jitter scale), f32 0x8D3834 = 0.1 (jitter
  sin/cos scale).
- Globals written: f32 accumulators 0x8D3824, 0x8D3828, 0x8D382C, 0x8D3830 (all initially 0.5; unnamed in gr), TextureShiftSecondU
  0xC21184, TextureShiftSecondV 0xC21180, TextureShiftFirstU 0xC2117C, TextureShiftFirstV 0xC21178, f32 0xC21170 and 0xC21174
  (unnamed, "WaterColor+4/+8" in gr's address map; read by sub_6E8780), WaterColor 0xC2116C (bytes r,g,b; a only in the debug
  path), WaterColorTriangle 0xC21168, WaterLayerAlpha[0] 0x8D3808 / [1] 0x8D380C (u32), the TempBuffer counters, the
  bToBeRendered bits of triangles and quads.
- Constants: 0x858B8C = 0.5f, 0x858B58 = 500.0f, 0x859A94 = 3000.0f, 0x859018 = 0.08f, 0x858624 = 1.0f, 0x858CEC = 0.04f,
  0x858C54 = 4294967296.0f (unsigned-conversion fix-up, unreachable here), 0x872174 = 0.0015339808f (2*pi/4096),
  0x872170 = 0.00076699042f (2*pi/8192), 0x87216C = 0.024f, 0x858C7C = 3.051851e-05f (1/32767), 0x858620 = 0.0625f,
  0x859AAC = 255.0f; immediates 0.04f (0x3D23D70A), 0.96f (0x3F75C28F), 1.0f, 500 (0x1F4).
- Behaviour:
  1. (0x6EF657) `if (!CGame::CanSeeWaterFromCurrArea()) return;` (plain return, no DefinedState).
     `SetCameraRange(); DefinedState();` (0x6EF664/0x6EF669).
  2. Sea-bed setup (0x6EF673-0x6EF6CB): `uiTempBufferIndicesStored = 0; uiTempBufferVerticesStored = 0;` (0x6EF67E/0x6EF684, before
     the first state call), then RwRenderStateSet: (1 TEXTURERASTER, seabd32Raster), (0xE FOGENABLE, 1), (0xA SRCBLEND, 5 SRCALPHA),
     (0xB DESTBLEND, 6 INVSRCALPHA), (0x1E ALPHATESTFUNCTIONREF, 0), (0xC VERTEXALPHAENABLE, 1).
  3. Sea-bed loop (0x6EF6D3-0x6EF871): `for (i = 0; i < (int)m_NumBlocksOutsideWorldToBeRendered; i++)` (count re-read each
     iteration, 0x6EF869):
     a. `X = (int16)BlocksX[i]`, `Y = (int16)BlocksY[i]` (movsx). `cx = ((float)X + 0.5f) * 500.0f - 3000.0f` (x87, kept),
        `cy = ((float)Y + 0.5f) * 500.0f - 3000.0f` (stored f32 [esp+0x30]). `cam = TheCamera.m_matrix ? &m_matrix->pos (+0x30)
        : &TheCamera.placement.pos` (0x6EF72F). `dx = cam.x - cx` (`fsubr`, extended), `dy = cam.y - cy`;
        `d = sqrt(dy*dy + dx*dx)` (0x6EF749-0x6EF753). `detailed = (d < *(f32*)0x8D3934)` (fcomp + `test ah,5; jnp`: true only for
        strictly less; equal / NaN -> false).
     b. Defaults: `xLo = 0.0f ([esp+0x1C]), xHi = 1.0f ([esp+0x14]), yLo = 0.0f ([esp+0x20]), yHi = 1.0f ([esp+0x18])`,
        `doY = false` (0x6EF76C-0x6EF788).
     c. If `X < 0 || Y < 0 || X >= 12 || Y >= 12` (signed, 0x6EF790-0x6EF79E): go to e with doX implicitly true (whole block).
        Else (block inside the 12x12 map grid): `doX = false; doY = false;`
        `if (X == 0) { xHi = 0.04f; doX = true; } else if (X == 11) { xLo = 0.96f; xHi = 1.0f; doX = true; }`
        `if (Y == 0) { yHi = 0.04f; doY = true; } else if (Y == 11) { yLo = 0.96f; yHi = 1.0f; doY = true; }` (0x6EF7A0-0x6EF7EE).
        If `!doX` skip e (0x6EF7F5).
     e. (0x6EF7F7) `seg(X, Y, xLo, xHi, 0.0f, 1.0f)` where `seg = detailed ? sub_6E6A10 : sub_6E6870`.
     f. If `doY` (0x6EF82A): `seg(X, Y, 0.0f, 1.0f, yLo, yHi)` (same choice of seg).
  4. Flush (0x6EF877-0x6EF8CE): if `uiTempBufferVerticesStored != 0`: `LittleTest()`; `if (RwIm3DTransform(TempVertexBuffer,
     uiTempBufferVerticesStored, NULL, 1 /*rwIM3D_VERTEXUV*/)) { RwIm3DRenderIndexedPrimitive(3 /*TRILIST*/, aTempBufferIndices,
     uiTempBufferIndicesStored); RwIm3DEnd(); }`. Then both counters = 0 (always). RwRenderStateSet(8 ZWRITEENABLE, 0),
     RwRenderStateSet(2 TEXTUREADDRESS, *(u32*)0x8D3930).
  5. Texture scroll (0x6EF8E2-0x6EF9E5), x87 with f/g kept extended:
     `f = ms_fTimeStep * m_CurrentFlow.x * *(f32*)0x8D392C`; `A = 0.08f * f + A` (A = f32 0x8D3824, stored);
     `g = ms_fTimeStep * m_CurrentFlow.y * *(f32*)0x8D392C`; `B = 0.08f * g + B` (0x8D3828);
     `if (A > 1.0f) A -= 1.0f;` `if (B > 1.0f) B -= 1.0f;` (fcomp + `test ah,0x41; jne skip`: equal, less and NaN do not wrap);
     `C = f * 0.04f + C` (0x8D382C, constant 0x858CEC), `D = g * 0.04f + D` (0x8D3830); then `C`, `D` wrapped the same way.
  6. Wave shifts (0x6EF9EB-0x6EFAF9): `ms = m_snTimeInMilliseconds`.
     `a = (float)(ms & 0xFFF) * 0.0015339808f` (the 2^32 fix-up at 0x6EFA04 is never taken: value is non-negative);
     `TextureShiftSecondU = sin(a) * Wavyness * 0.08f + A`; `TextureShiftSecondV = cos(a) * Wavyness * 0.08f + B` (`fsin`/`fcos`
     on the extended `a`, multiplication order as written).
     `b = (float)(ms & 0x1FFF) * 0.00076699042f` stored f32 [esp+0x20]; `TextureShiftFirstU = C` (dword copy 0x6EFA6E);
     `cb = cos(b)` stored f32 [esp+0x1C]; `TextureShiftFirstV = cb * 0.024f + D` (0x6EFA91).
     `r1 = rand(); W2 = (float)r1 * 3.051851e-05f * *(f32*)0x8D3928` -> f32 0xC21174 (0x6EFAB8);
     `r2 = rand(); W2 = sin(b) * *(f32*)0x8D3834 + W2` (stored again, 0x6EFAE3);
     `W1 = ((float)r2 * 3.051851e-05f * *(f32*)0x8D3928) + cb * *(f32*)0x8D3834` -> f32 0xC21170 (0x6EFAF9; the jitter scale is
     re-read from 0x8D3928 into [esp+0x24] before each rand). Both rand() calls advance the CRT RNG (2 per frame).
  7. Colours (0x6EFAFF-0x6EFB6D): `WaterColor.r = (u8)_ftol(m_fWaterRed)`, `.g = _ftol(m_fWaterGreen)`, `.b = _ftol(m_fWaterBlue)`
     (low byte of the truncated int); `WaterColorTriangle = *(u32*)&WaterColor` (copied after r,g,b, so it takes the current a byte);
     `alpha = _ftol(m_fWaterAlpha * 0.5f)`; `WaterLayerAlpha[1] = alpha`; `q = (alpha << 8) / (256 - alpha)` (signed `idiv`);
     `WaterLayerAlpha[0] = (q > 255) ? 255 : q` (signed compare, 0x6EFB66).
  8. Water setup (0x6EFB72-0x6EFBBA): counters = 0; RwRenderStateSet (1, waterclear256Raster), (0xE, 1), (0xA, 5), (0xB, 6).
  9. Triangles (0x6EFBBD-0x6EFCB5): `for (i = 0; i < (int)NumWaterTriangles; i++)` if `WaterTriangles[i].flags & 1`:
     `RenderWaterTriangle(V0.x, V0.y, V0.rp, V1.x, V1.y, V1.rp, V2.x, V2.y, V2.rp)` (Vk = m_aVertices[tri.verts[k]], x/y int16
     sign-extended), then `flags &= ~1` (0x6EFCA0).
  10. Quads (0x6EFCBB-0x6EFE6D): `for (i = 0; i < (int)NumWaterQuads; i++)` if `WaterQuads[i].flags & 1`:
      a. If `*(u8*)0xC228DC != 0` (debug colours, 0x6EFCDD): `WaterColor.r = _ftol((float)(i & 0xF) * 0.0625f * 255.0f)`,
         `.g = _ftol((float)((i / 16) & 0xF) * 0.0625f * 255.0f)`, `.b = _ftol((float)((i / 256) & 0xF) * 0.0625f * 255.0f)` (signed
         divisions), `.a = _ftol(m_fWaterAlpha)` (no 0.5 factor), `WaterColorTriangle = WaterColor`.
      b. `RenderWaterRectangle(x(v0), x(v1), y(v0), y(v2), rp(v0), rp(v1), rp(v2), rp(v3))` with vk = m_aVertices[quad.verts[k]]
         (0x6EFE4C; mapping proven below), then `flags &= ~1` (0x6EFE51).
  11. Outside-world surface (0x6EFE73-0x6F005E): `for (i = 0; i < (int)m_NumBlocksOutsideWorldToBeRendered; i++)`: `X = BlocksX[i]`,
      `Y = BlocksY[i]` (int16 signed compares); skip if `0 <= X < 12 && 0 <= Y < 12`. Else with `P = {z 0.0f, bigWaves 1.0f,
      smallWaves 0.0f, flowX 0, flowY 0}` (bytes +0xE/+0xF are uninitialised stack padding) for all four corners:
      `RenderWaterRectangle(_ftol((float)(X*500) - 3000.0f), _ftol((float)(X*500 + 500) - 3000.0f),
      _ftol((float)(Y*500) - 3000.0f), _ftol((float)(Y*500 + 500) - 3000.0f), P, P, P, P)` (0x6F004E; exact integers).
  12. Flush as in step 4 (0x6F0064-0x6F00AC, counters zeroed always), `RenderBoatWakes()` (0x6F00B2), tail-call `DefinedState()`.
- Quad argument mapping (10b): ESI = &WaterQuads[i] + 4. CRenPar copies are built deepest-first from `[esi+2]` = verts[3]
  (0x6EFD83, P4), `[esi]` = verts[2] (0x6EFD87, P3), `[esi-2]` = verts[1] (0x6EFD96, P2), `[esi-4]` = verts[0] (0x6EFDB3, P1);
  the ints pushed are `y(v2)` (0x6EFE26, from 0xC22912 + 20*verts[2] at 0x6EFDC8) = Y2, `y(v0)` (0x6EFE43) = Y1, `x(v1)`
  (0x6EFE44, from 0x6EFDF6) = maxX, `x(v0)` (0x6EFE48) = minX.
- Edge cases / oddities: 0xC228DC is never written by any catalogued function (only read here), so step 10a is dead in practice
  but must be kept. `alpha == 256` (m_fWaterAlpha in [512, 514)) divides by zero at 0x6EFB5A; larger alpha gives a negative
  WaterLayerAlpha[0]. The bToBeRendered bits are cleared only for primitives that were rendered. The detail-distance test uses
  the block centre and the camera's 2D distance.
- gta-reversed: `plugin::Call<0x6EF650>` only (WaterLevel.cpp:807, `.Reversed = false`); sea-bed segment helpers absent (listed as
  missing in WaterLevel.h). gr's `GetWaterLayerTexInfo` pairs layer 0 with TextureShiftFirst* and layer 1 with TextureShiftSecond*,
  consistent with the 0.04 / 0.08 scroll speeds here.
- Open questions: none affecting behaviour. Names of the 0x8D3824..0x8D3934 and 0xC21170/0xC21174/0xC228DC globals are not known.
