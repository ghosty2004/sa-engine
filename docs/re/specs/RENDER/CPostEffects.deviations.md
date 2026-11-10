# CPostEffects (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/PostEffects.cpp`. `pRasterFrontBuffer` 0xC402D8 (RwRaster: +0x0C width,
+0x10 height). Heat-haze arrays (int32[180]): hpS 0xC3F868, hpY 0xC3FB38, hpX 0xC3FE08.

## 0x701450 CPostEffects::HeatHazeFXInit  (gr_deviations = 3)
Original layout: entry 0x701450 returns at once when `m_HeatHazeFXType (0xC402BC) == m_HeatHazeFXTypeLast (0x8D50E4)`
(0x70145F). Otherwise `switch (type)` through the jump table 0x701768 (types 0..4 -> 0x70147B, 0x701506, 0x701582,
0x70160D, 0x701615; `type > 4` unsigned skips to 0x701690). The per-type constants match gr (intensity, random
shift, speed min/max, scan/render sizes as `ftol(RsGlobal width (0xC17044, fild) * 0.0015625f [0x859520] * k)` and
`ftol(height (0xC17048) * 0.002232143f [0x859524] * k)`). At 0x701690 the code jumps through the .HOODLUM-style stub
0x401092 which performs `m_HeatHazeFXTypeLast = ebx (type)` (the target address is computed as
`[0x62B4B2] ^ [0x15605E8]` = 0xFFDDE733 ^ 0xFF50B7D7 = 0x008D50E4) and returns to 0x701696 -> loop 0x7016A0.

Rows PostEffects.cpp:535, 536, 537 (rng-helper, `hpX/hpY/hpS[i] = CGeneral::GetRandomNumberInRange(...)`):
- Original bytes 0x7016A0-0x701759, for `i = 0..179` (esi = 4*i, `cmp esi, 0x2D0`), three rand() calls in this order:
  - `hpX[i] = ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14 = 2^-15] * (int)(frontBuffer->width - m_HeatHazeFXScanSizeX))`
    (width re-read every iteration at 0x7016A0; fimul of the integer difference at 0x7016CA; no minimum is added).
  - `hpY[i] = ftol((rand() & 0xFFFF) * 2^-15 * (int)(frontBuffer->height - m_HeatHazeFXScanSizeY))` (0x7016D3-0x70171B).
  - `hpS[i] = ftol((rand() & 0xFFFF) * 2^-15 * (int)(m_HeatHazeFXSpeedMax - m_HeatHazeFXSpeedMin)) + m_HeatHazeFXSpeedMin`
    (0x70170F-0x70174A).
- gr: `GetRandomNumberInRange(ScanSizeX, width)` / `(ScanSizeY, height)` / `(SpeedMin, SpeedMax)` with gr's lerp
  formula. Besides the formula difference (cat14 finding 1), gr's X/Y ranges are wrong: the original range is
  `[0, width - ScanSizeX)` (the inlined helper with min = 0, max = width - scan), gr's is `[ScanSizeX, width)`, which
  shifts every haze sample by the scan size and can put the scan rectangle past the right/bottom edge.
- Verdict: CONFIRMED (all three rows; X and Y also a gr logic error). Class: keep-original.

## 0x7046E0 CPostEffects::Render  (gr_deviations = 2)
Row PostEffects.cpp:963 (notsa-helper `notsa::contains({PLANE, HELI, BOAT, TRAIN}, type)`): marked deviates=no; not a
behavioural difference (a set test). Not re-judged.

Row PostEffects.cpp:1095 (notsa-helper, `format_to_sz(gString, "Gallery\\gallery{}.jpg", ++photoIdx)` + `fs::exists`):
- Original bytes 0x705336-0x705390: `idx = 1; sprintf(gString 0xB71670, "Gallery\\gallery%d.jpg" [0x872644], idx)`
  (CRT 0x821BB5); `f = fopen(gString, "r" [0x85A53C])` (CRT 0x8232D8); `while (f) { idx++; sprintf(gString, fmt, idx);
  fclose(f) (0x82318B); f = fopen(gString, "r"); }`. Existence is "fopen for reading succeeds" (relative to the
  current directory set by `CFileMgr::SetDirMyDocuments` 0x538860 at 0x705331). Then
  `JPegCompressScreenToFile(TheCamera.m_pRwCamera [0xB6F97C], gString)` 0x5D0820 and `CFileMgr::SetDir("")` 0x5387D0.
- gr: same names and numbering (1, 2, ...), but uses `std::filesystem::exists`, which differs for entries that exist
  but cannot be opened (directories, no read permission: the original treats them as free and overwrites/fails).
- Verdict: CONFIRMED (minor). Class: harmless (port: use "open for read succeeds" to match exactly).

Found in passing (CONFIRMED gr logic error, keep-original): 0x7052D1-0x705309. When `s_WaitForOneFrame (0xC40321)` is
set and `CWeapon::ms_bTakePhoto (0xC8A7C1)` is set, the original CLEARS `s_WaitForOneFrame` (0x705309,
`mov byte [0xC40321], 0`) before `CTimer::Suspend` 0x5619D0. gr writes `s_WaitForOneFrame = true` there. The rest of
the photo block matches gr: `RenderWeaponPedsForPC` 0x732F30, `ms_weaponPedsForPC (0xC88224)` reset via 0x53E8D0
(thiscall), `CTimer::Resume` 0x561A00, `if (!byte [0xBA677B] (gr FrontEndMenuManager.m_bActivateMenuNextFrame)) { CSpecialFX::bSnapShotActive
(0xC7C714) = 1; SnapShotFrames (0xC7C710) = 0 }`, then `s_SavePhotoToGallery (0xC40320) = 0`, `ms_bTakePhoto = 0`,
`m_bSavePhotoFromScript (0xC402D0) = 0` (0x7053CE-0x7053DC).
