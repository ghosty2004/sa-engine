# CMirrors (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Mirrors.cpp` (pin 20d8d2a). Globals: CMirrors::pBuffer 0xC7C71C, pZBuffer 0xC7C720 (RwRaster*),
Scene.m_pRwCamera 0xC1703C.

## 0x7230A0 CMirrors::CreateBuffer  (gr_deviations = 1)
Original: `__cdecl void ()`, plain `ret` (0x72314F).
Row Mirrors.cpp:95 (`default: NOTSA_UNREACHABLE();` in the quality switch).
- Original bytes: if `pBuffer != NULL` return (0x7230A7). `depth = Scene.m_pRwCamera->frameBuffer (+0x60)->depth (+0x14)` (0x7230AD-0x7230B6).
  `q = g_fx.GetFxQuality()` (0x7230BE). If `q >= 1` (signed `jl`, 0x7230C6):
  `pBuffer = RwRasterCreate(1024, 512, depth, 5)` (0x7230D5, stored even if NULL); if non-NULL: `pZBuffer = RwRasterCreate(1024, 512,
  depth, 1)` (0x7230F3); if that is non-NULL return (0x723102); else `RwRasterDestroy(pBuffer); pBuffer = NULL` (0x72310B-0x723113).
  FALLBACK (reached for `q < 1` and for both failure cases above, 0x72311D): `pBuffer = RwRasterCreate(512, 256, depth, 5)`,
  `pZBuffer = RwRasterCreate(512, 256, depth, 1)` (results stored unchecked).
- Verdict: CONFIRMED - gr's switch has no fallback: for MEDIUM..VERY_HIGH a failed 1024x512 allocation leaves pBuffer NULL (`break`),
  while the original retries at 512x256; and quality values outside 0..3 hit NOTSA_UNREACHABLE in gr but take the low (q < 1) or high
  (q >= 1) path in the original. Class: keep-original.
