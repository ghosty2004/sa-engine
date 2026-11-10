# CAudioEngine (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/AudioEngine.cpp` (pin 20d8d2a).

## 0x5B9C60 CAudioEngine::Initialise  (gr_deviations = 1)
Row AudioEngine.cpp:108 (NOTSA: call `CAEAudioUtility::StaticInitialise()` before `AEAudioHardware.Initialise()`):
- Original bytes: call order CLoadingScreen::Pause 0x590310; CAEAudioHardware::Initialise 0x4D9930 (0x5B9C6D); ...
  CAERadioTrackManager::Initialise 0x5B9390; CAESoundManager::Initialise 0x5B9690; (file read via CFileMgr); CAEFrontendAudioEntity::
  Initialise 0x5B9AB0; SetEffectsFaderScalingFactor 0x506E50; CAEAudioUtility::StaticInitialise 0x5B97F0 (0x5B9DC6, i.e. AFTER the
  hardware); then the other StaticInitialise calls, CAEScriptAudioEntity / CAEPedlessSpeechAudioEntity / CAECollisionAudioEntity
  Initialise, CLoadingScreen::Continue.
- Verdict: CONFIRMED (order differs). In the original, GetCurrentTimeInMS 0x4D9E80 called before StaticInitialise divides by a zero
  performance frequency on the x87 (masked exception: +inf, `_ftol` gives the integer-indefinite value) instead of crashing; gr's
  integer path would fault. Class: keep-original order; the port's GetCurrentTimeInMS must reproduce the x87 result for
  freq = 0 (open question below).
- Open question: 0x4D9EA7 with frequency 0 - exact `_ftol` result for +inf/NaN (expected 0x8000000000000000) should be confirmed
  in the parity harness.
