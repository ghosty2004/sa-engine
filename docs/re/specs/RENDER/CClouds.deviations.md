# CClouds (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Clouds.cpp` (pin 20d8d2a). Random helpers: `_rand` 0x821B1E (MSVC rand, 0..32767),
`_ftol` 0x821B40 (truncation). Constants: 0x858C7C = 3.0518509e-05f (1/32767), 0x858B14 = 3.0517578e-05f (1/32768).
gr's `CGeneral::GetRandomNumberInRange(float a, float b)` is `b*t + a*(1-t)` and the int version `(T)lerp(a, b-1, t)`
(docs/re/cat14-gr-deviations.md finding 1), so every row below that swaps an inlined original formula for the gr helper is a
rounding / value deviation.

## 0x713760 CClouds::MovingFog_Create  (gr_deviations = 6)
Original: `__cdecl void (CVector* posn [esp+4])`, plain `ret`. Slot search inlined: first i in 0..0x15D (350) with
`ms_mf.m_bFogSlots[i]` (0xC6C158 + i) == 0; none -> return (the `cmp esi,-1` at 0x71377E is dead). `P = *posn` copied BEFORE the
rand calls (0x713787-0x71379F). Per row (all `t = (float)rand() * 3.0518509e-05f`, x87, result stored f32; f32 arrays indexed by slot):
- Clouds.cpp:129 m_vecPosn.x: original `t * 116.0f [0x87283C] - 58.0f [0x866C50]` -> 0xC6C2B8 + 12i (0x7137A6-0x7137C5).
- Clouds.cpp:130 m_vecPosn.y: same formula -> 0xC6C2BC (0x7137CB-0x7137EA).
- Clouds.cpp:131 m_vecPosn.z: `t * 10.0f [0x85862C] - 5.0f [0x858C80]` -> 0xC6C2C0 (0x7137F0-0x71380F).
  Then `pos.c = P.c + pos.c` for x, y, z (0x713815-0x71383F).
- Clouds.cpp:134 m_fSize: `t * 6.0f [0x858B44] + 4.0f [0x858B90]` -> 0xC6D320 + 4i (0x713845-0x713864); m_fIntensity (0xC6D898) = 1.0f.
- Clouds.cpp:136 m_fMaxIntensity: `t * 12.0f [0x858CCC] + 8.0f [0x859000]` -> 0xC6DE10 (0x713876-0x713895).
- Clouds.cpp:137 m_Speed: `t * 0.70000005f [0x872838] + 0.5f [0x858B8C]` -> 0xC6E394 (0x71389C-0x7138BC); then m_bFogSlots[i] = 1.
- Verdict (each of the 6 rows): CONFIRMED (gr's lerp helper rounds differently from `t*range + min`; same rand() consumption order
  x, y, z, size, maxIntensity, speed). Class: keep-original.

## 0x715F40 CClouds::VolumetricClouds_Create  (gr_deviations = 8)
Original: `__cdecl void (CVector* posn [esp+4])` (returns with EAX = -1 on every path, callers ignore it), `sub esp,0x3C`.
Common prelude (0x715F47-0x715F9A): `r1 = (t * 4.0f [0x858B90]) + 1.0f`; `mn = 20.0f [0x858BA4] * r1`, `mxXY = r1 * 100.0f [0x858628]`,
`mxZ = 40.0f [0x858A10] * r1` (all stored f32).
- Clouds.cpp:1027 rand1To5: original `(t * 4.0f) + 1.0f` (above). CONFIRMED (gr lerp(1,5)).
- Clouds.cpp:1036 alpha (int, 0xC6BCF8 + 4i): original `36 - _ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f * -92.0f [0x872880])`
  (0x716007-0x716031 / 0x71625E-0x716288), i.e. 36 + trunc(r*92/32768), range 36..127. CONFIRMED (gr `GetRandomNumberInRange<int32>(36,128)`
  = lerp(36,127,r/32767): different values for the same rand()).
- Clouds.cpp:1039/1040/1041 size.x/y/z (0xC6B488 + 12i): original `(t * (mxXY - mn)) + mn` for x and y, `(t * (mxZ - mn)) + mn` for z
  (t first, then `fld mx; fsub mn; fmulp; fadd mn`, 0x71603E-0x7160B1; in the posn == null branch `(mx - mn)` is precomputed and stored
  f32 at 0x7161CC-0x716207, then `t * diff + mn`). CONFIRMED x3 (rounding).
- Clouds.cpp:1067/1068/1069 pos.x/y/z for posn == null (0xC6AC18 + 12i): original `((t * (d - (-d))) + (-d)) + C.c` for x, y and
  `((t * (q - (-q))) + (-q)) + C.z` with `d = m_fVolumetricCloudMaxDistance` (0xC6AA58), `q = d * 0.25f [0x858C84]`, the spans
  `d - (-d)` / `q - (-q)` stored f32 (0x71620B-0x71623F); C = TheCamera position (`TheCamera.m_matrix` 0xB6F03C ? matrix->pos :
  placement pos 0xB6F02C). Loop over ALL 180 (0xB4) slots, no free-slot check (0x716250-0x716361). CONFIRMED x3 (rounding).
- Class: keep-original for all 8 rows.
Found in passing (CONFIRMED, keep-original) - the posn != null branch (0x715FA4-0x71618A):
- The offset range is an INTEGER: `off = _ftol(r1 * 3.0f [0x858B3C])`; each offset component is
  `_ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f * (int)(off - (-off)) (fimul)) + (-off)` (integer add), then `fild` and `+ P.c`.
  gr uses a float range and `CVector::Random` (gr lerp helper).
- The running position P is a LOCAL copy of `*posn` (0x715FB2-0x715FBA, updated at 0x716178-0x716186); the caller's vector is never
  written. gr does `*posn = *posn + ...` (writes back). The caller (VolumetricCloudsRender) passes a local, so the visible effect is nil,
  but a port must not write through the pointer.
- Free-slot search per cloud: first i in `0 .. m_VolumetricCloudsUsedNum - 1` (0xC6AA5C, signed) with `bUsed[i]` (0xC6AAB0) == 0;
  none (or UsedNum <= 0) -> return. Order per slot: bUsed = 1, bJustCreated (0xC6AB64) = 1, alpha, size x/y/z, pos x/y/z.

## 0x716380 CClouds::VolumetricCloudsRender  (gr_deviations = 1)
Row Clouds.cpp:1273 ("This does differ a little from what they did originally, but it's fine") and the companion bug-note
Clouds.cpp:1275 ("Must be rendered in groups of 3"):
- Original bytes (0x7169F0-0x716AB2 and 0x716AD1-0x716AF5): vertices go straight into `TempVertexBuffer` 0xC4D958 (RwIm3DVertex,
  stride 0x24: x +0, y +4, z +8, colour +0x18 as 0xAARRGGBB, u +0x1C, v +0x20), counter in EBP. Per vertex k (18 per cloud):
  `x = size.x * modelX[k] + pos.x` (`fld size.x; fmul modelX; fadd pos.x`), same for y, z; colour = quad colour k/6; after each vertex,
  if the counter reaches exactly 0x7F2 (2034 = 678 triangles) it flushes: `RwIm3DTransform(0xC4D958, 2034, nullptr, 9)` 0x7EF450;
  if non-null `RwIm3DRenderPrimitive(3 TRILIST)` 0x7EF6B0 and `RwIm3DEnd()` 0x7EF520; counter = 0. After all clouds, the same flush
  if the counter > 0; then tail-jump to `CPostEffects::ImmediateModeRenderStatesReStore` 0x700E00.
- gr: RenderBuffer helpers, flushing whenever fewer than 3 slots remain, with gr's own buffer capacity.
- Verdict: CONFIRMED that the batching differs (flush points / number of draw calls), REFUTED as a visible difference: both submit whole
  triangles with identical vertex data and flags (XYZ|UV = 9, TRILIST) and the original's 2034 is a multiple of 3. Class: harmless.
Found in passing: the respawn offset uses `CGeneral::GetRandomNumberInRange<float>` 0x41BD90 three times in x, y, z order with
(-200, 200), (-200, 200), (-50, 50) (0x716641-0x716679); gr's `CVector::Random(min, max)` calls gr's helper (finding 1, unmarked
rng deviation, keep-original). The colour `min((sum of the 6 sky colour u16s) / 6 + 64, 255)` is computed with `imul 0x2AAAAAAB`
(signed division by 6, 0x7166C7) as in gr.
