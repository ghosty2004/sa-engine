# CTimeCycle (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/TimeCycle.cpp` (pin 20d8d2a).

## 0x5BBAC0 CTimeCycle::Initialise  (gr_deviations = 2)
Original: `__cdecl void ()` (gr declares `(bool padFile)`; the argument is never read), plain `ret` (0x5BC081).
- Row TimeCycle.cpp:41 (`if (!file) { // NOTSA` log + return). Original bytes: `CFileMgr::SetDir("DATA")`, `file = CFileMgr::OpenFile("TIMECYC.DAT",
  "rb")` (0x5BBADE), `SetDir("")` - no NULL check; the loop calls CFileLoader::LoadLine(file) 0x536F80 directly (0x5BBB21).
  Verdict: CONFIRMED (gr-only early return). Class: harmless (only reachable without the data file; the original would pass NULL on).
- Row TimeCycle.cpp:124 ("R* made a mistake in line 320"; `if (n < 51) NOTSA_LOG_WARN`). Original bytes: the sscanf result
  (CRT 0x8220AD, 0x5BBCE2) is never tested; values are stored unconditionally, so fields not parsed on a short line keep the previous
  line's values (same stack slots), as in gr. Verdict: REFUTED (gr only adds a log). Class: harmless.
Found in passing (verify-on-port notes, all matching gr unless stated): loops are weather 23 (counter at [esp+0x18] = 0x17) outer, hour 8
(ebp = 8) inner; lines starting with '/' or NUL are skipped; float fields are converted with `_ftol` (truncation): sizes
`_ftol(x * 10.0f + 0.5f)` ([0x85862C], [0x858B8C]), post-fx alphas `_ftol(a + a)`, directional mult `_ftol((float)(u8)_ftol(d) * 100.0f)`.
CONFIRMED (rounding-level, keep-original): m_vecDirnLightToSun = (cos(-2.356194496154785) * cos(0.7853981852531433),
sin(-2.356194496154785) * cos(0.7853981852531433), sin(0.7853981852531433)) with the f64 constants [0x86A6C8]/[0x86A6D0] and x87
fcos/fsin, stored f32 (0x5BC026-0x5BC060), then CVector::Normalise 0x59C910; gr evaluates `std::cos(-3.0f * PI / 4.0f)` etc. in float.
m_FogReduction (0xB79E48) and m_bExtraColourOn (0xB7C484) are cleared as dwords (0x5BC070/0x5BC075).
