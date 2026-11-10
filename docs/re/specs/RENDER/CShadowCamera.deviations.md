# CShadowCamera (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/ShadowCamera.cpp` (pin 20d8d2a). CShadowCamera: +0 m_pRwCamera (RwCamera*: +0x60 frame-buffer
raster, +0x80 nearPlane); RwRaster +0xC width (i32). Render states via `RwEngineInstance (0xC97B24)->+0x20` (RwRenderStateSet);
`+0x18` = RwIm2DGetNearScreenZ value. Im2DRenderQuad 0x705A20 (cdecl x1, y1, x2, y2, z, rhw, uvOffset).

## 0x706070 CShadowCamera::RasterResample  (gr_deviations = 1)
Row ShadowCamera.cpp:391 (`return camRaster; // Original code returns the camera's raster even if it couldn't render into it`).
- Original bytes (thiscall, `ret 4`): `if (!m_pRwCamera) return <EAX = 0>` (0x706076-0x706080; EAX holds the null camera). camRaster =
  `cam->+0x60` (0x706084); BEFORE BeginUpdate it computes `size = (float)camRaster->width` (fild, f32), `uv = 0.5f [0x872670] / size`,
  `rhw = 1.0f [0x858624] / cam->nearPlane` (fdivr). If `RwCameraBeginUpdate(cam)` 0x7EE190 returns 0 -> return camRaster (0x7060B7 ->
  0x706159). Else states SRCBLEND(0xA)=2, DESTBLEND(0xB)=1, ZTESTENABLE(6)=0, TEXTUREFILTER(9)=2, TEXTURERASTER(1)=source; quad
  (0, 0, size, size, nearScreenZ, rhw, uv); ZTEST=1, SRCBLEND=5, DESTBLEND=6; EndUpdate; return camRaster.
- Verdict: REFUTED - gr keeps the original behaviour (the comment documents it). Class: harmless.

## 0x706170 CShadowCamera::RasterBlur  (gr_deviations = 3)
- Row :426 (`if (!numPasses) { // NOTSA  return camRaster; }`). Original bytes (thiscall (RwRaster* blur [esp+4], i32 numPasses
  [esp+8]), `ret 8`): null camera -> return 0 (0x706176-0x706180). `numPasses <= 0` (signed `jle`, 0x7061BC) skips the loop to
  0x706316, which writes `cam->+0x60 = camRaster` and returns `blur` ([esp+0x1C] = arg 1, 0x70631C). So with 0 passes the original
  returns the BLUR raster; gr returns camRaster. Verdict: CONFIRMED. Class: keep-original.
- Row :444 (`// Set original raster` - `RwCameraSetRaster(cam, camRaster)` at the top of each pass). Original 0x7061D0-0x7061D6
  `cam->+0x60 = camRaster` every iteration. REFUTED, harmless.
- Row :479 (`// Use original raster` after the loop). Original 0x706316-0x706321 restores `cam->+0x60 = camRaster`. REFUTED, harmless.
- Notes (no difference): per pass i (EBP, 0..numPasses-1): BeginUpdate; on i == 0 only, the states 0xA=2, 0xB=1, 6=0, 9=2; raster = blur;
  quad uv `(0.5f [0x872670] + 0.5f [0x858B8C]) * (1.0f / blurWidth)` (0x706234-0x706248; gr divides `1.0 / brwidth`, the same value);
  EndUpdate. Then `cam->+0x60 = blur`; BeginUpdate; raster = camRaster; uv `(0.5f - 0.5f) * inv` = 0; on the last pass ZTEST=1,
  SRCBLEND=5, DESTBLEND=6 (0x7062CE `numPasses - 1 == i`); EndUpdate. The quad size is the blur raster width (both passes).

## 0x706330 CShadowCamera::DrawBlurryRaster2  (gr_deviations = 2)
Rows :487 (`// Save original`) and :514 (`// Restore to original`).
- Original bytes (thiscall (RwRaster* src), `ret 4`): no null check of m_pRwCamera. `size = (float)src->width` (0x706336),
  `saved = cam->+0x60` (0x706349), `rhw = 1.0f / cam->nearPlane`, `cam->+0x60 = src` (0x706352); if BeginUpdate: states 0xA=1, 0xB=3,
  6=0, 9=2, 1=saved; quad (0, 0, size, size, z, rhw, 0); restore states; EndUpdate. Always `cam->+0x60 = saved` (0x706404); return src.
- Verdict (both): REFUTED - gr saves and restores the same raster at the same points. Class: harmless.
