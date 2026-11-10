# CPostEffects (RENDER) - full specs

Common: render states are set with `RwEngineInstance (0xC97B24)->+0x20 (state, value)` (RwRenderStateSet). State ids used:
1 TEXTURERASTER, 2 TEXTUREADDRESS, 6 ZTESTENABLE, 8 ZWRITEENABLE, 9 TEXTUREFILTER, 0xA SRCBLEND, 0xB DESTBLEND,
0xC VERTEXALPHAENABLE, 0xE FOGENABLE, 0x15 STENCILENABLE, 0x16 STENCILFAIL, 0x17 STENCILZFAIL, 0x18 STENCILPASS,
0x19 STENCILFUNCTION, 0x1A STENCILFUNCTIONREF. `Scene` 0xC17038 (`Scene.m_pRwCamera` = [0xC1703C]; camera +0x60 frame buffer
raster, +0x80 near clip). RwRaster: +0x0C width, +0x10 height, +0x14 depth. Im2D vertex buffer `aRadiosityVertexBuffer` 0xC5F958
(RwIm2DVertex, stride 0x1C: +0 x, +4 y, +8 z, +0xC rhw, +0x10 colour (ARGB dword), +0x14 u, +0x18 v); indices
`aTempBufferIndices` 0xC4B958 (u16), counters uiTempBufferVerticesStored 0xC4B950 / uiTempBufferIndicesStored 0xC4B954.
Heat-haze globals: pRasterFrontBuffer 0xC402D8, m_HeatHazeFXIntensity 0x8D50E8, m_HeatHazeFXSpeedMin 0x8D50EC / Max 0x8D50F0,
m_HeatHazeFXRandomShift 0xC402C0, ScanSizeX/Y 0xC40304/0xC40308, RenderSizeX/Y 0xC4030C/0xC40310, hpX 0xC3FE08, hpY 0xC3FB38,
hpS 0xC3F868 (int32[180]), byte 0xC402BB (no gr name; "debug full-screen haze" switch, see step 6).
`rnd(n)` below = `ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * n)` with `n` an int (fimul).

## 0x701780 CPostEffects::HeatHazeFX  (catalogue: specified/REA, port level 15)
- Convention: `__cdecl void (float intensity [esp+4], bool alphaMaskMode [esp+8])`, plain `ret`.
- Callers: CPostEffects::Render 0x7046E0. Callees: RwCameraClear 0x7EE340, ImmediateModeRenderStatesStore 0x700CC0 / Set 0x700D70 /
  ReStore 0x700E00, DrawQuad 0x700EC0 (cdecl (x1, y1, x2, y2, r, g, b, a, raster)), Fx_c::Render 0x49E650 (thiscall on g_fx
  0xA9AE00, (RwCamera*, bool)), HeatHazeFXInit 0x701450, RwCameraEndUpdate 0x7EE180, RwRasterPushContext 0x7FB060,
  RwRasterRenderFast 0x7FAF50, RwRasterPopContext 0x7FB110, RsCameraBeginUpdate 0x619450, `rand`, `_ftol`, 0x734EA0
  (Im2D RenderIndexedPrimitive thunk, cdecl (primType, verts, numVerts, indices, numIndices)).
- Behaviour:
  1. `deep = (Scene.camera->+0x60 (raster)->+0x14 (depth) != 16)` (0x701783-0x701796).
  2. If `alphaMaskMode` (0x7017AB-0x7018A6): `RwCameraClear(Scene.camera, &RwRGBA{0,0,0,0}, 2)`; Store; Set; SRCBLEND = 5,
     DESTBLEND = 2. If deep: STENCILENABLE = 1, STENCILFAIL = 1, STENCILZFAIL = 1, STENCILPASS = 3, STENCILFUNCTIONREF = 0,
     STENCILFUNCTION = 8. `DrawQuad(0, 0, (float)RsGlobal.maximumWidth [0xC17044], (float)RsGlobal.maximumHeight [0xC17048],
     0, 0, 0, 255, NULL)`. If deep: STENCILFUNCTIONREF = 1. `g_fx.Render(TheCamera.m_pRwCamera [0xB6F97C], true)`; ReStore.
     Else: `byte_C402BB = 0` (0x7018A8).
  3. Clamp: if `intensity < 0.0f` -> 0; else if `intensity > 1.0f` (`test ah,0x41; jne` keeps <, == and NaN) -> 1.0f.
  4. `HeatHazeFXInit()`; TEXTUREADDRESS = 3; `RwCameraEndUpdate(Scene.camera)`; `RwRasterPushContext(pRasterFrontBuffer)`;
     `RwRasterRenderFast(Scene.camera->+0x60, 0, 0)`; `RwRasterPopContext()`; `RsCameraBeginUpdate(Scene.camera)`;
     FOGENABLE = 0; both counters = 0; VERTEXALPHAENABLE = 1.
  5. `alpha = ftol((float)m_HeatHazeFXIntensity * intensity)`. If deep: STENCILPASS = 1, STENCILFUNCTIONREF = 1,
     STENCILFUNCTION = 3. ZTEST = 0, ZWRITE = 0, TEXTURERASTER = pRasterFrontBuffer, TEXTUREFILTER = 2, SRCBLEND = 5,
     DESTBLEND = 6.
  6. If `byte_C402BB != 0` (0x701E84): one quad with colour `alpha<<24 | 0xFF0000`: v0 (0,0), v1 ((float)W, 0), v2 (0, (float)H),
     v3 (W, H) (W/H = front buffer width/height via fild), z = `RwEngineInstance->+0x18`, and `rhw = 1/near`. ORIGINAL BUG:
     the rhw stores go to v0 (+0xC) and v1 (+0x28) twice (0x701EC5/0x701F1D and 0x701EEF/0x701F55); v2 and v3 rhw and all
     UVs keep whatever the buffer held. Indices `b, b+2, b+1, b+1, b+2, b+3`; counters += 4 / += 6. Go to step 8.
  7. Otherwise (0x7019FD), `dx = (RenderSizeX - ScanSizeX) / 2`, `dy = (RenderSizeY - ScanSizeY) / 2` (signed, toward zero),
     `col = alpha<<24 | 0xFFFFFF`. For `i = 0 .. 179`:
     a. `x = hpX[i]`, `y = hpY[i]`, `rx = x - dx`, `ry = y - dy`. If `RandomShift > 0`: `rx += rnd(2s) - s`, then
        `ry += rnd(2s) - s` (s = RandomShift re-read before each; two rand() calls, x first).
     b. If `rx < 0` { `x += dx; rx = 0` }. `W = raster->width`; if `rx > W - RenderSizeX` { `rx = W - RenderSizeX; x -= dx` }.
        If `ry < 0` { `y += dy; ry = 0` }. `H = raster->height`; if `ry > H - RenderSizeY` { `ry = H - RenderSizeY; y -= dy` }.
     c. Quad at vertex base b = uiVtx (all coordinates via fild, stored f32; z = `RwEngineInstance->+0x18`; rhw = `1.0f /
        Scene.camera->+0x80`, recomputed per vertex; colour col):
        v0 = (rx, ry) uv (x/W, y/H); v1 = (rx + RenderSizeX, ry) uv ((x + ScanSizeX)/W, y/H);
        v2 = (rx, ry + RenderSizeY) uv (x/W, (y + ScanSizeY)/H); v3 = (rx + RenderSizeX, ry + RenderSizeY)
        uv ((x + ScanSizeX)/W, (y + ScanSizeY)/H). Each uv is `(float)int / (float)int` with the raster size re-read via fild.
        Indices `b, b+2, b+1, b+1, b+2, b+3`; uiIdx += 6, uiVtx += 4.
     d. `hpY[i] -= ftol(CTimer::ms_fTimeStep [0xB7CB5C] * 0.5f [0x858B8C] * hpS[i])` (fimul). If the result is negative
        (`jns` not taken): `hpX[i] = rnd(W - ScanSizeX)`, `hpY[i] = H - ScanSizeY`, `hpS[i] = rnd(SpeedMax - SpeedMin) +
        SpeedMin` (two rand() calls, X first).
  8. Tail (0x701FBF): TEXTURERASTER = pRasterFrontBuffer; if `uiVtx != 0`: `0x734EA0(3 /*TRILIST*/, aRadiosityVertexBuffer,
     uiVtx, aTempBufferIndices, uiIdx)`. `uiVtx = 0` (uiIdx is not reset). FOGENABLE = 0, ZTEST = 1, ZWRITE = 1,
     TEXTURERASTER = 0, VERTEXALPHAENABLE = 0, SRCBLEND = 5, DESTBLEND = 6; if deep: STENCILENABLE = 0.
- Edge cases: no bound check on the vertex/index buffers (180 quads = 720 vertices). The UV of a clamped sample uses the
  adjusted `x`/`y`, the position uses the clamped `rx`/`ry`.
- gta-reversed: PostEffects.cpp:542 `plugin::Call` only. Open questions: none.

## 0x702080 CPostEffects::Radiosity  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl void (int32 intensityLimit [esp+4], int32 filterPasses [esp+8], int32 renderPasses [esp+0xC],
  int32 intensity [esp+0x10])`, plain `ret`.
- Callers: CPostEffects::Render 0x7046E0 (2 sites). Callees: render states via RwEngineInstance, 0x734E90 (RwIm2DRenderPrimitive thunk, cdecl (primType = 4, verts = aRadiosityVertexBuffer, numVerts)).
- Globals: m_RadiosityPixelsX 0xC40314 and m_RadiosityPixelsY 0xC40318 (read as INT32 - gr types them as float),
  m_bRadiosityStripCopyMode 0x8D510B, m_bRadiosityLinearFilter 0x8D510A, m_bRadiosityDebug 0xC402CD,
  m_RadiosityFilterUCorrection 0x8D511C, m_RadiosityFilterVCorrection 0x8D5120 (int32, fild), pRasterFrontBuffer 0xC402D8,
  Scene.camera +0x80 (near clip), RsGlobal.maximumWidth/Height 0xC17044/0xC17048.
- Vertex pairs: every element below is a pair of Im2D vertices A (index k) and B (index k+1) written into
  aRadiosityVertexBuffer at `k = uiTempBufferVerticesStored`, then `uiTempBufferVerticesStored += 2`. A always gets
  x = y = z = 0 and `rhw = 1.0f / near` (recomputed per vertex); B gets z = 0 and the same rhw.
- Behaviour:
  1. `w = PixelsX`, `h = PixelsY` (ints). States: TEXTUREFILTER = 1, FOGENABLE = 0, ZTESTENABLE = 1, ZWRITEENABLE = 0,
     TEXTURERASTER = pRasterFrontBuffer, VERTEXALPHAENABLE = 0, SRCBLEND = 5, DESTBLEND = 6. `uiVtx = 0`.
  2. Vertex 0/1 (written directly at 0xC5F958/0xC5F974, colours not written): v0 = (0,0) uv (0,0); v1 = StripCopyMode ?
     ((float)maxWidth, (float)maxHeight) : ((float)w, (float)h), uv (1,1). Then `uiVtx = 0` again (0x7021A2), so this pair
     is overwritten by the first filter pair. ZTESTENABLE = 1.
  3. If `filterPasses > 0`: TEXTUREFILTER = 2; for each pass (loop unrolled x4 at 0x702205, remainder at 0x702590, same
     operations): A.uv = `((float)UCorr / (float)raster->width, (float)VCorr / (float)raster->height)` (x87 `fdivr` of the
     fild'ed raster size into the fild'ed correction); B.uv = `((float)w / (float)W, (float)h / (float)H)` (fild/fidiv
     with the OLD w, h); then `w = w / 2`, `h = h / 2` (signed, toward zero); B.xy = `((float)(w + 1), (float)(h + 1))`;
     both colours 0xFFFFFFFF.
  4. SRCBLEND = 5, DESTBLEND = 6. One more pair (0x70268A): B.xy = `((float)(w + 1), (float)(h + 1))` (uv not written),
     colour of both = `0x80000000 | L<<16 | L<<8 | L` with L = intensityLimit (built by `((L | 0xFFFF8000) << 8 | L) << 8 | L`).
  5. If `renderPasses <= 0` go to step 7. If `uiVtx > 2`: `0x734E90(4, aRadiosityVertexBuffer, uiVtx)`; `uiVtx = 0`.
     TEXTUREFILTER = m_bRadiosityLinearFilter ? 2 : 3.
  6. renderPasses pairs (unrolled x4 + remainder; StripCopyMode selects one of two copies of the loop that differ only in
     the colour): A uv (0,0); B.uv = `((float)w / (float)raster->width, (float)h / (float)raster->height)` (w, h = the
     final halved sizes); B.xy = `((float)maxWidth, (float)maxHeight)`; colour of both = (non-strip mode and
     m_bRadiosityDebug) ? 0xFFFFFFFF : `intensity << 24`.
  7. If `uiVtx > 2`: `0x734E90(4, aRadiosityVertexBuffer, uiVtx)`. `uiVtx = 0`; FOGENABLE = 0, ZTESTENABLE = 1,
     ZWRITEENABLE = 1, TEXTURERASTER = 0, VERTEXALPHAENABLE = 0, SRCBLEND = 5, DESTBLEND = 6.
- Edge cases: no bound check on the vertex buffer. The raster used for the uv divisions is pRasterFrontBuffer read once.
- gta-reversed: PostEffects.cpp:887 `plugin::Call<0x702080>()` passes NONE of the four arguments (gr bug if gr's own
  Render calls it; the original stack slots would be garbage). 0x734E90 is `jmp [RwEngineInstance->+0x30]`
  (RwIm2DRenderPrimitive), primitive type 4 = rwPRIMTYPETRISTRIP, so the vertex pairs form one triangle strip exactly as
  the original builds it. Open questions: none.

## 0x7030A0 CPostEffects::SpeedFX  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl void (float speed [esp+4])`, plain `ret` (the drawing path tail-jumps to
  ImmediateModeRenderStatesReStore 0x700E00).
- Callers: CPostEffects::Render 0x7046E0. Callees: ImmediateModeRenderStatesStore 0x700CC0, ImmediateModeRenderStatesSet
  0x700D70, `rand` 0x821B1E, DrawQuad 0x700EC0, ImmediateModeRenderStatesReStore 0x700E00 (tail).
- Globals: TheCamera.ActiveCam (u8 0xB6F081), cam mode dword `0xB6F1C4 + 0x238*ActiveCam`; speed table 0x8D5190 (7 entries
  of 16 bytes: f32 threshold, i32 count, i32 shift, i32 jitter; data: (0.6,1,4,0) (0.7,2,4,0) (0.8,3,4,0) (0.9,3,4,0)
  (0.93,4,4,1) (0.96,4,4,2) (1.0,5,4,3)); ms_imf 0xC40150: U1 +0x14 (0xC40164), V1 +0x18 (0xC40168), U2 +0x1C (0xC4016C),
  V2 +0x20 (0xC40170), quad[k].u/v at 0xC401EC/0xC401F0, 0xC40208/0xC4020C, 0xC40224/0xC40228, 0xC40240/0xC40244;
  pRasterFrontBuffer 0xC402D8; m_SpeedFXAlpha 0x8D5104 (read as u8); RsGlobal.maximumWidth/Height 0xC17044/0xC17048.
- Constants: 0.004f [0x859CE0], 3.0518509e-05f [0x858C7C] (1/32767), 0.0025f [0x86C340].
- Behaviour:
  1. `mode = camMode`; `m1 = (mode == 1)`, `m2 = (mode == 2)`. `count = shift = jitter = 0`; for `i = 6 down to 0`: first
     entry with `!(speed < table[i].threshold)` (`test ah,1; je`) gives count/shift/jitter (0x7030DF-0x703120).
     If `m1 || m2`: `shift = shift / 2` (signed), `jitter = 0`. If `count <= 0` return (no state changes).
  2. Store + Set immediate-mode states. `jx = jy = 0`. If `jitter > 0`: `s = (float)jitter * 0.004f`; `JX = U2 * s`,
     `JY = V2 * s` (stored f32); `jx = (float)rand() * (1/32767) * JX`, then `jy = (float)rand() * (1/32767) * JY`.
  3. `a = (U2 * (float)shift) * 0.0025f`, `b = ((float)shift * V2) * 0.0025f` (stored f32). Offsets:
     `ox0 = a, ox2 = a, ox1 = -a, ox3 = -a, oy0 = b, oy1 = b, oy2 = -b, oy3 = -b`.
  4. Repeat `count` times:
     a. If m1: `jx = jy = 0` and `ox1 = ox3 = oy0 = oy1 = oy2 = oy3 = 0` (ox0, ox2 kept). If m2: `jx = jy = 0` and
        `ox0 = ox2 = oy0 = oy1 = oy2 = oy3 = 0` (ox1, ox3 kept). (Re-applied every pass.)
     b. Quad UVs (each `(base + offset) +/- jitter`, stored f32, in this order):
        `q0.u = (U1 + ox0) + jx`, `q0.v = (V1 + oy0) + jy`, `q1.u = (U2 + ox1) - jx`, `q1.v = (V1 + oy1) + jy`,
        `q2.u = (U1 + ox2) + jx`, `q2.v = (V2 + oy2) - jx` (ORIGINAL: the x jitter is used here, 0x70338C reads the copy of jx),
        `q3.u = (U2 + ox3) - jx`, `q3.v = (V2 + oy3) - jy`.
     c. `DrawQuad(0, 0, (float)maxWidth, (float)maxHeight, 255, 255, 255, (u8)m_SpeedFXAlpha, pRasterFrontBuffer)`.
     d. `ox0 += a; oy0 += b; ox1 -= a; oy1 += b; ox3 -= a; oy3 -= b; ox2 += a; oy2 -= b` (this order).
  5. Reset quad UVs: q0 (0,0), q1 (1,0), q2 (0,1), q3 (1,1); ReStore (tail jump).
- gta-reversed: PostEffects.cpp:843 `plugin::Call` only. Open questions: none.
