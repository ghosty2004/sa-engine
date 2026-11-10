# CAEMFDecoder (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Loaders/AEMFDecoder.cpp` (pin 20d8d2a).

## 0x4E7C70 CAEMFDecoder::InitLibrary  (gr_deviations = 1)
Original: `__cdecl bool ()` (static), AL result, plain `ret`. Flag `quickTimeInitialized` u8 0xB61D70.
Row AEMFDecoder.cpp:288 ("This is the original function", QuickTime code kept in a comment; gr uses Media Foundation).
- Original bytes: if flag != 0 -> (0x4E7C92) flag = 1, return 1. Else `r = 0x8195C0(0)` (QuickTime InitializeQTML, cdecl,
  i16 result); if `r != 0` return 0; `r = 0x819820()` (EnterMovies, i16); if `r != 0` return 0; flag = 1; return 1.
  (Return value is `(ax == 0)` via `sete`, 0x4E7CA0.)
- Verdict: CONFIRMED - the commented reconstruction matches the bytes, but gr's live code initialises Media Foundation
  instead of QuickTime. Class: keep-original for the control flow (flag + two init calls); the backend is a platform choice
  for the port (no QuickTime on the target) - record it in the divergence ledger.
