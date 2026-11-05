# CLoadingScreen: gta-reversed deviation notes (STREAM)

Source: `re/gta-reversed/source/game_sa/LoadingScreen.cpp` (pin 20d8d2a). Globals: m_bActive 0xBAB318,
m_bFading 0xBAB31C, m_bLegalScreen 0xBAB31D, m_bFadeInNextSplashFromBlack 0xBAB31E,
m_bFadeOutCurrSplashToBlack 0xBAB31F, m_FadeAlpha 0xBAB320 (u8), m_PauseTime 0xBAB32C,
m_PercentLoaded 0xBAB330, m_TimeBarAppeared 0xBAB338, m_timeSinceLastScreen 0xBAB340,
m_aSplashes 0xBAB35C (7 x CSprite2d), m_currDisplayedSplash 0x8D093C (i32, initial -1),
m_numChunksLoaded 0x8D0940, m_chunkBarAppeared 0x8D0944.

## 0x5900B0 CLoadingScreen::LoadSplashes  (gr_deviations = 4)
Rows :112 ("Edit in Mobile"), :127 (NOTSA std::shuffle), :142 (NOTSA_UNREACHABLE for the logo id),
:144 / :146 (format_to_sz).
- :127 Original bytes (0x5900EB-0x59017D): after `srand(QueryPerformanceCounter.LowPart)` (0x821B11):
  `pool[15] = {0..14}`, `rem = 14`; for k = 0..14: `idx = (k == 0) ? 0 :
  _ftol((rand() & 0xFFFF) * (1/32768.f) * (rem + 1))` (fimul); `out[k] = pool[idx]`; if `idx < rem`
  shift `pool[idx+1 .. rem]` down one byte (`rep movsb`, count rem - idx); `rem--`. After the loop
  `out[14] = pool[0]` (redundant). That is 14 rand() calls and a random order of 1..14.
  Then for i = 0..6: name = `useSplashId ? (id == 1 ? "nvidia" : "eax") : (i == 0 ? "title_pc_US" :
  sprintf("loadsc%d", out[i]))`, `m_aSplashes[i].SetTexture(name)`.
  gr shuffles only indices 1..6 of a 7-entry array with `std::mt19937{random_device}` and does not call
  rand() at all.
  Verdict CONFIRMED. Class keep-original (splashes 1..6 come from loadsc1..loadsc14, and 14 rand() calls
  are consumed).
- :142 Original: `cmp bl, 1; jne` -> any id other than 1 gives "eax" (0x590188-0x5901B3). gr aborts for
  ids other than 0/1. Verdict CONFIRMED (edge only). Class keep-original.
- :112, :144, :146 Verdict REFUTED (comment; `sprintf(buf, "nvidia"/"eax")` and
  `sprintf(buf, "loadsc%d", out[i])` with 0x821BB5 give the same strings). Class harmless.
- Found in passing (CONFIRMED, keep-original): the TXD slot is created as `AddTxdSlot("loadscs")`
  (string 0x866CC4, 0x5900C5) and loaded from "loadscs.txd" (0x866CF0) in "MODELS\\TXD\\"; gr names the slot
  "loadsc0" (LoadingScreen.cpp:116 and the lookup at :69). Slot names are observable through FindTxdSlot.

## 0x590370 CLoadingScreen::RenderLoadingBar  (gr_deviations = 1)
Rows :208 (NOTSA) and :209 (TODO) inside `#ifdef PRINT_LOADMSG`.
- PRINT_LOADMSG is not defined in the gr tree, so gr only logs; the original prints nothing.
- Verdict: REFUTED for both rows.
- Class: harmless.
- Found in passing (CONFIRMED, keep-original): gr sets `m_TimeBarAppeared = GetClockTime()` at the top,
  before the legal-screen / percent tests, with real time. The original (0x590485-0x5904BE) does it only
  after DrawBarChart, i.e. only when `!m_bLegalScreen && percent > 0 && percent < 100` (NaN returns early),
  and only if `m_TimeBarAppeared == 0`; the value is
  `(float)(u32)RsTimer() * 0.001f [0x858CDC] - m_PauseTime` (multiply by the f32 constant, not divide by
  1000; unsigned conversion via fild + 2^32 fix-up).

## 0x590AC0 CLoadingScreen::DoPCScreenChange  (gr_deviations = 1)
Rows :327 ("Edit in Mobile") and :334 (`IsFixBugs()` wrap of m_currDisplayedSplash).
- :334 Original: `if (!isLastOne) ++m_currDisplayedSplash` (`inc dword [0x8D093C]`, 0x590ADE), no wrap.
  Verdict CONFIRMED. Class keep-original.
- :327 Verdict REFUTED (comment). Class harmless.
- Found in passing (CONFIRMED, keep-original): the loading-tune amplitude. Original (0x590B60-0x590BC5),
  for i = 0..49: `m_FadeAlpha = (u8)_ftol((float)i * 5.0f)`; if `isLastOne || m_bFadeInNextSplashFromBlack`:
  `amp = m_bFadeOutCurrSplashToBlack ? (255.0f - (float)m_FadeAlpha) * (1/255.f) [0x859A3C] : 1.0f`;
  `AudioEngine.ServiceLoadingTune(amp)`. gr uses `(255 - alpha) / 25.0f` (10x larger) and requires
  `isLastOne && m_bFadeOutCurrSplashToBlack`. Also gr wraps the 20-frame black loop in a NOTSA
  FastLoader `NoFading` option (off by default). On isLastOne the original inlines Shutdown: m_bActive = 0,
  delete the 7 sprites, `FindTxdSlot("loadscs")` (string 0x866CC4), RemoveTxd + RemoveTxdSlot when found (0x590CAE-0x590CF6).

## 0x590D00 CLoadingScreen::NewChunkLoaded  (gr_deviations = 1)
Rows :373 ("Edit in Mobile") and :395 (`#ifdef FIX_BUGS` splash-time condition).
- :395 Original (0x590D86-0x590DAF): `delta = now - m_timeSinceLastScreen` with
  `now = (float)(u32)RsTimer() * 0.001f` (no pause subtraction). If `m_currDisplayedSplash == 0`:
  `delta < 5.5f` -> DisplayPCScreen (tail jump); else (**any non-zero splash, including -1**):
  `delta < 5.0f` -> DisplayPCScreen; otherwise DoPCScreenChange(false), `m_timeSinceLastScreen = now`,
  and `if m_chunkBarAppeared == -1: m_chunkBarAppeared = m_numChunksLoaded`. NaN compares as "less".
  gr's FIX_BUGS branch drops the copyright case and uses FastLoader `ScreenChangeTime`; gr's #else branch
  treats -1 as "change now". Verdict CONFIRMED. Class keep-original.
- :373 Verdict REFUTED (comment). Class harmless.
- Found in passing (keep-original): `m_PercentLoaded = (float)(loaded - appeared) * 100.0f /
  (int)(140 - appeared)` (fild, fmul, fidiv; 0x590D20-0x590D3F). The clock uses `* 0.001f`, while gr's
  GetClockTime divides by 1000.0f (one-ulp differences possible). The display is unconditional in the
  original (gr gates it on FastLoader NoLoadScreen/NoLoadBar, off by default).
