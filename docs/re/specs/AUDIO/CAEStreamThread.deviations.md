# CAEStreamThread (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/AEStreamThread.cpp` (pin 20d8d2a).

## 0x4F1680 CAEStreamThread::Initialise  (gr_deviations = 2)
Original: `__thiscall bool (this; CAEStreamingChannel* ch [esp+4])`, `ret 4`, MSVC EH frame (0x83C11B) around `new`.
- Original bytes: `+0x10 = 1`, `+0x11 = +0x12 = +0x15 = +0x16 = 0` (u8); `+0x34 = ch->GetPlayingTrackID()` 0x4F1A60; `+0x28 =
  ch->vfunc(+0xC)` (GetPlayTime); `+0x30 = ch->GetActiveTrackID()` 0x4F1A40; `+0x2C = ch->vfunc(+0x10)` (GetLength);
  `+0 = CreateThread(NULL, 0, MainLoop 0x4F15C0, this, 4, &this->+4)`; `SetThreadPriority(+0, 0)` (no failure test);
  `InitializeCriticalSection(this + 0x38)`; `+8 = ch`; `+0xC = new(0x1C)` 0x82119A, constructed with 0x4E0930 when non-null
  (null stays null); `CAEMP3TrackLoader::Initialise(+0xC)` 0x4E0C50 (called even if null; result ignored); return `+0x10` (1).
Row :47 (`assert(m_nHandle)`): debug-only. REFUTED, harmless.
Row :50 (TODO comment about CRITICAL_SECTION): gr still calls InitializeCriticalSection - same as the original. REFUTED,
harmless. (gr's flag names: +0x10 m_bActive, +0x11 m_bThreadActive, +0x12 m_bNeedsService, +0x15 m_bPreparingStream,
+0x16 m_bStopRequest.)
