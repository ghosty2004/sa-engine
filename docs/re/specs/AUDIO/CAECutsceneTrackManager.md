# CAECutsceneTrackManager (AUDIO) - full specs (RE-SPEC-3, group U1)

Layout (gr AECutsceneTrackManager.h, size 0x30): +0 u8 m_bPlayRequest, +1 u8 m_bPaused, +4 i32 m_nAudioChannel, +8 i32
m_nStatus (S0..S10), +0xC u32 m_nTrackId, +0x14 u32 _f14. AEAudioHardware = 0xB5F8B8 (ECX for all hardware calls).

## 0x4DBF10 CAECutsceneTrackManager::StartTrackPlayback  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (this = ECX)`, tail `jmp` 0x4D8F30. Caller: Service 0x4DBFB0.
- Behaviour: `DisableBassEq()` 0x4D94D0; `SetChannelFlags((u16)+4 zero-extended, 0, 0x27)` 0x4D9500 (0x4DBF23); `SetChannelVolume(
  (u16)+4, 0, -3.0f (imm 0xC0400000), 0)` 0x4D8870 (0x4DBF3A); tail `AEAudioHardware.StartTrackPlayback()` 0x4D8F30.
  Note the order: flags BEFORE volume (the ambience manager 0x4D6DA0 does volume first).
- gta-reversed: same (0x27 = SECONDARY_GROUP | UNDUCKABLE | CLAMP_VOL_TO_NEG | IS_NOT_STREAM). Open questions: none.

## 0x4DBF60 CAECutsceneTrackManager::CheckForPause  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (this = ECX)`, plain `ret`. Caller: Service 0x4DBFB0.
- Behaviour: `f = (CTimer::m_UserPause 0xB7CB49 || CTimer::m_CodePause 0xB7CB48 || this->+1) ? 0.0f : 1.0f` (imm 0x3F800000,
  0x4DBF78; 0 at 0x4DBF82); `SetChannelFrequencyScalingFactor((i32)(i16)+4 (movsx, 0x4DBF8E), 0, f)` 0x4D8960, called once.
- gta-reversed: same. Open questions: none.

## 0x4DC0E0 CAECutsceneTrackManager::PreloadCutsceneTrack  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall void (this = ECX; i16 trackId [esp+4], bool wait [esp+8])`, `ret 8`. Callers:
  PreloadBeatTrack 0x4DC130 (tail), CAudioEngine::PreloadCutsceneTrack 0x507E30 (tail, 3 sites).
- Behaviour: `+0xC = (i32)(i16)trackId` (0x4DC0EA); `+0x14 = 0`; `+8 = 0` (S0). If `wait != 0` (byte compare, 0x4DC0ED): loop
  `{ t = AEAudioHardware.GetTrackPlayTime() 0x4D8F60; this->Service(t) 0x4DBFB0; AEAudioHardware.Service() 0x4D9870 } while
  (+8 != 2)` (0x4DC11C) - a busy wait with no timeout.
- Edge cases: if the track never reaches status 2 the game hangs (original behaviour).
- gta-reversed: same. Open questions: none.
