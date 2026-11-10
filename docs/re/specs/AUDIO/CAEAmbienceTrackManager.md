# CAEAmbienceTrackManager (AUDIO) - full specs (RE-SPEC-3, group U1)

Instance `AEAmbienceTrackManager` 0x8AC15C, size 0x2C. Layout (gr AEAmbienceTrackManager.h, confirmed by the offsets below):
+0 u8 m_OverrideRadio, +1 u8 m_LastAmbienceOverrodeRadio, +2 u8 m_StartAmbienceAtBeginning, +3 i8 m_AmbienceRadioStation
(eRadioID, -1 = RADIO_INVALID), +4 i32 m_HwClientHandle (channel, read as u16), +8 i32 m_AmbienceStatus (0 STARTING,
1 WAITING_TO_PLAY, 2 READY_TO_PLAY, 3 PLAYING, 4 STOPPING, 5 STOPPING_SILENCED, 6 STOPPING_CHANNELS_STOPPED,
7 WAITING_TO_STOP, 8 STOPPED), +0xC i32 m_SpecialMissionAmbienceTrackID, +0x10 i32 m_PrevAmbiencePlayTimeMs,
+0x14 i32 m_PrevAmbienceStopTimeMs, +0x18 f32 m_Volume, +0x1C f32 m_FreqFactor, +0x20 i32 m_RequestedSettings.TrackID,
+0x24 i32 PlayingTrackID, +0x28 u8 TrackFlags. `AEAudioHardware` = 0xB5F8B8, `AudioEngine` = 0xB6BC90.

## 0x4D6CC0 CAEAmbienceTrackManager::StopAmbienceTrack  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void (this = ECX)`, plain `ret`, no stack use. Callers: UpdateAmbienceTrackAndVolume 0x4D6E60
  (4 sites), CAudioEngine::StopAmbienceTrack 0x507220. Callees: none.
- Behaviour: `s = this->+8`; if `s` is 0, 1, 2 or 3 (0x4D6CC5-0x4D6CD6) then `+8 = 4` (STOPPING) (0x4D6CD8); otherwise the
  status is left unchanged. Then unconditionally `+0 = 0`, `+1 = 0`, `+2 = 0` (0x4D6CDF-0x4D6CE4).
- gta-reversed: StopAmbienceTrack + NOTSA ChangeStatusToStopped (same switch, same three clears). Matches. Open questions: none.

## 0x4D6DA0 CAEAmbienceTrackManager::StartTrackPlayback  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void (this = ECX)`, `ret` via tail jump. Caller: Service 0x4D76C0.
- Callees (all `__thiscall` on ECX = &AEAudioHardware 0xB5F8B8): DisableBassEq 0x4D94D0, SetChannelVolume 0x4D8870,
  SetChannelFlags 0x4D9500, StartTrackPlayback 0x4D8F30 (tail `jmp` at 0x4D6DE1).
- Behaviour: `DisableBassEq()`; `SetChannelVolume((u16)this->+4 zero-extended, 0, this->+0x18 (f32 bits), 0)`
  (push order 0, vol, 0, ch at 0x4D6DB6-0x4D6DBB); `SetChannelFlags((u16)+4, 0, 0x21)` (0x4D6DCC); tail-call
  `AEAudioHardware.StartTrackPlayback()`.
- gta-reversed: same (flags = FLAG_SECONDARY_GROUP | FLAG_IS_NOT_STREAM = 0x21). Open questions: none.

## 0x4D6DF0 CAEAmbienceTrackManager::CheckForPause  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (this = ECX)`, plain `ret`. Caller: Service 0x4D76C0.
- Globals read: CTimer::m_UserPause 0xB7CB49 (u8), CTimer::m_CodePause 0xB7CB48 (u8), FrontEndMenuManager+0xE9 0xBA6831
  (u8, gr m_bMainMenuSwitch).
- Behaviour: `f = (m_UserPause || m_CodePause || menu+0xE9) ? 0 (bits 0 = 0.0f, 0x4D6E19) : this->+0x1C` (tested in that order,
  0x4D6DF1-0x4D6E0E). Then calls `AEAudioHardware.SetChannelFrequencyScalingFactor((u16)+4, 0, f)` 0x4D8960 TWICE with
  identical arguments (0x4D6E34, 0x4D6E48). The value is moved as raw dword bits (no x87).
- gta-reversed: identical (also calls it twice). Open questions: none.

## 0x4D6E60 CAEAmbienceTrackManager::UpdateAmbienceTrackAndVolume  (catalogue: specified/REA, port level 9)
- Convention: `__thiscall void (this = ECX)`, plain `ret` (several epilogues). Frame `sub esp,0x24` + 4 pushes. Caller:
  Service 0x4D76C0. The extent includes a .text detour stub at 0x40154A (`mov ecx,0xB6BC90; jmp 0x4D7196`) reached from
  0x4D7191; it only loads ECX = &AudioEngine and keeps the flags of `test al,al` at 0x4D718F - same behaviour inline.
- Callees: CVector::Magnitude 0x4082C0 (thiscall -> ST0), CGameLogic::LaRiotsActiveHere 0x441C10 (cdecl -> AL),
  StopAmbienceTrack 0x4D6CC0, CAEAudioEnvironment::GetDistanceAttenuation 0x4D7F20 (cdecl (f32) -> ST0),
  CAEAudioEnvironment::GetPositionRelativeToCamera 0x4D80B0 (cdecl (CVector* out, CVector* in), caller pops 8),
  AudioEngine (ECX = 0xB6BC90): ReportFrontendAudioEvent 0x506EA0, StopRadio 0x506F70, IsRadioOn 0x506FD0,
  GetCurrentRadioStationID 0x507040 (-> AL), IsVehicleRadioActive 0x507050, StartRadio 0x507DC0;
  CStats::FindMostFavoriteRadioStation 0x558FA0 (-> AL), CCullZones::CamNoRain 0x72DDB0, CCullZones::PlayerNoRain 0x72DDC0.
- Globals: CWeather::Sandstorm 0xC812F4 (f32), CWeather::UnderWaterness 0xC8132C (f32), CGame::currArea 0xB72914 (i32),
  CAudioZones::m_NumActiveSpheres 0xB6DCC0, m_aActiveSpheres 0xB6DC94 (i32[]), m_aSpheres 0xB6EBA8 (stride 0x1C: +8 i16 zone,
  +0xC CVector centre), m_NumActiveBoxes 0xB6DCBC, m_aActiveBoxes 0xB6DC6C, m_aBoxes 0xB6DCD0 (stride 0x18: +8 i16 zone);
  TheCamera 0xB6F028: +0x14 m_matrix (0xB6F03C), +4 placement pos (0xB6F02C), +0xE4 f32 (0xB6F10C, gr m_fDistanceToWater),
  +0xE8 f32 (0xB6F110, gr m_fHeightOfNearestWater). Camera position = `m_matrix ? m_matrix+0x30 : 0xB6F02C`; only `.z` (+8) is read.
- Locals: `prevStation = this->+3` (i8, 0x4D6E68), `isSandstorm = 0` (u8), `volume = -6.0f` (imm 0xC0C00000, 0x4D6E7D),
  `freqTarget` (f32, only set on the water path), `zonePos` (CVector, only set on the sphere path), `track = -1` (EDI).
- Behaviour:
  1. Entry (0x4D6E85-0x4D6E8C): `+0 = 0`, `+2 = 0`, `+3 = -1`. (`+1` is not touched here.)
  2. If `this->+0xC >= 0` (signed, 0x4D6E97): `track = +0xC`, `+2 = 1`, `+0 = 1`, go to step 5 (zones are never evaluated,
     0x4D6EF3-0x4D6EF6 always jumps since track >= 0).
  3. Else if `!(Sandstorm > 0.0f [0x858B50])` (fcomp + `test ah,0x41`, NaN counts as <= 0; 0x4D6EB3) go to step 4.
     Else: if `currArea == 0 && !PlayerNoRain() && !CamNoRain()` (order 0x4D6EB5, 0x4D6EBE, 0x4D6EC7):
     `track = 167 (0xA7)`, `volume = log10-form`: x87 `fldlg2; fld Sandstorm; fyl2x` (= log10(2) * log2(Sandstorm)),
     `* 20.0f [0x858BA4]`, `- 6.0f [0x858B44]`, stored f32 (0x4D6ED0-0x4D6EEB). In every Sandstorm > 0 case `isSandstorm = 1`
     (0x4D6EEF). Then if `track != -1` go to step 5, else (indoors / no-rain zone) FALL THROUGH to step 4.
  4. Audio zones (0x4D6EFC): if `m_NumActiveSpheres > 0` (signed): `s = m_aActiveSpheres[0]`, `zone = (i16)m_aSpheres[s].+8`,
     `zonePos = m_aSpheres[s].+0xC..+0x14`. Else if `m_NumActiveBoxes > 0`: `zone = (i16)m_aBoxes[m_aActiveBoxes[0]].+8`
     (zonePos NOT written - stays uninitialised stack). Else go to step 5. If `zone < 0` (0x4D6F50) or `zone - 4 > 0x3F`
     unsigned (0x4D6F5F) go to step 5. Two-level jump table: byte index table 0x4D7680[zone-4], dword table 0x4D75DC.
     Cases (track ids decimal):
     - 4: track 143 (0x8F). `z = camera.z`; if `z <= 1372.0f [0x85EED8]` (fcomp + `test ah,0x41`; NaN takes this branch too):
       `volume = -6.0f [0x85A5C0] - (1372.0f - z) * 9.0f [0x85EED4]` (x87 `fld 1372; fsub z; fmul 9; fsubr -6`), else `volume = -6.0f`.
     - 5: track 140 (0x8C), `volume = DistVol`. 10: track 139 (0x8B), `volume = DistVol`.
       13: track 157 (0x9D), `volume = DistVol` AND `+0 = 1` (0x4D6FF0).
       DistVol: `GetPositionRelativeToCamera(&rel, &zonePos)`, `m = rel.Magnitude()`, `GetDistanceAttenuation((f32)(m *
       0.2f [0x858CC4]))`, `- 6.0f [0x858B44]`, stored f32.
     - 8:165, 12:168, 15:164, 17:146, 19:138, 20:136, 21:135, 23:148, 24:159, 25:158, 26:154, 28/29:147, 34:162, 36:155,
       39:163, 44:152, 48:137, 50:173, 51:156, 64:151, 66:170, 67:171.
     - 37: track 144 and `+2 = 1`. 41: track 169 and `+0 = 1`.
     - 30: `+3 = FindMostFavoriteRadioStation()` (AL). 52..62: `+3 = zone - 51` (52 -> 1 ... 62 -> 11; 0x4D7143-0x4D717E).
     - All other zones 4..67 (6, 7, 9, 11, 14, 16, 18, 22, 27, 31, 32, 33, 35, 38, 40, 42, 43, 45, 46, 47, 49, 63, 65): nothing.
  5. Radio handling (0x4D718C, `st = (i8)this->+3`):
     a. `st >= 0`: if `!IsRadioOn()` -> `StartRadio(st zero-extended, 2)` (bass CUT), return. Else if
        `GetCurrentRadioStationID() != st` (byte compare) -> `StopRadio(0, 0)`. Return.
     b. `st < 0` and `prevStation >= 0` (0x4D71E5): if `IsRadioOn()` -> `StopRadio(0, 0)`. Return.
     c. Else if `IsRadioOn()` (0x4D720A): if `+0 != 0`: `StopRadio(0, 0)`; `ReportFrontendAudioEvent(0x23 RADIO_RETUNE_STOP, 0.0f,
        1.0f)` (pushes 1.0f, 0, 0x23). Then `+1 = +0` (0x4D723C). Return.
     d. Else if `track >= 0` (0x4D724A): go to step 6. Else (`track == -1`) go to step 7.
  6. Ambience track (0x4D7250, status `S = +8`, STOPPED = 8):
     - `track == +0xC` (the mission track): if `S == 8`: `+0x20 = track, +8 = 0, +0x18 = -2.0f, +0x1C = 1.0f`. Else if
       `+0x20 != track`: StopAmbienceTrack(). (Equal: nothing.) Return.
     - otherwise: if `S == 8`: `+0x20 = track, +8 = 0, +0x18 = -34.0f, +0x1C = 1.0f`; return. Else if `+0x20 == track`:
       `v = +0x18 + 1.0f [0x858624]`; `+0x18 = (v < volume) ? v : volume` (fcom + `test ah,5; jnp`: equal or NaN store
       `volume`) (0x4D72BE-0x4D72D6). Else FadeOut: if `+0x18 > -40.0f [0x85A708]` (`test ah,0x41`): `+0x18 -= 0.4f [0x858EE8]`
       else StopAmbienceTrack(). Return.
  7. Water / riot ambience (0x4D72E0, no track):
     - If `!(UnderWaterness < 0.5f [0x858B8C])` (0x4D72F1; NaN takes the "< 0.5" branch): `volume = 14.0f`, `freqTarget =
       0.0625f`, go to 7b.
     - Else: `t = camera.z - TheCamera.+0xE8`; `r = sqrt(+0xE4 * +0xE4 + t * t)` (x87: `fld +0xE4; fmul +0xE4; fld t; fmul t;
       faddp` = `d2 + t2`), `r *= 0.5714285970f [0x85EED0]` (= 4/7), `volume = GetDistanceAttenuation((f32)r) - 12.0f
       [0x858CCC]` (stored f32), `freqTarget = 1.0f`. If `volume <= -34.0f [0x85EECC]` (or NaN) go to 7c, else 7b.
     - 7b (0x4D7370): if `S == 8`: `+0x20 = 174 (0xAE), +8 = 0, +0x18 = -34.0f, +0x1C = (UnderWaterness < 0.5f) ? 1.0f :
       0.0625f`. Else if `+0x20 != 174`: FadeOut (as in step 6, without returning). Else step `+0x18` toward `volume` by 1.0:
       if `volume > +0x18`: `+0x18 = min(+0x18 + 1.0f, volume)` (store `+0x18 + 1` only if strictly < volume); if
       `volume < +0x18`: `+0x18 = max(+0x18 - 1.0f, volume)` (store `+0x18 - 1` only if strictly > volume); equal/NaN: no store.
       Then (all three sub-cases, 0x4D742A) step `+0x1C` toward `freqTarget` by 0.25f [0x858C84] with the same rule. Return.
     - 7c (0x4D7494): if `LaRiotsActiveHere() && currArea == 0`: if `S == 8`: `+8 = 0, +0x20 = 166 (0xA6), +0x18 = -34.0f,
       +0x1C = 1.0f`; else if `+0x20 == 166`: `v = +0x18 + 1.0f; +0x18 = (v < -12.0f [0x858FD4]) ? v : -12.0f`; else FadeOut.
       Return.
       Otherwise (0x4D7526): if `S == 8` return. If `+0x20 == 174`: if `volume < -40.0f` (`test ah,5; jp`, strict, NaN no)
       StopAmbienceTrack(); then `+0x18 = -100.0f, +0x1C = 1.0f`; return. Else if `isSandstorm && +0x18 > -40.0f`:
       `+0x18 -= 0.4f; +0x1C = 1.0f`; return. Else StopAmbienceTrack(); if `+1 != 0 || IsVehicleRadioActive()`:
       `StartRadio(GetCurrentRadioStationID(), 0)` (bass NORMAL); `+0x1C = 1.0f`. Return.
- Edge cases / original bugs:
  - Box audio zones 5, 10 and 13 compute DistVol from the uninitialised `zonePos` stack slot ([esp+0x1C], 0x4D6F30 path never
    writes it). Port: keep a never-initialised local (deterministic choice e.g. whatever the port's stack holds is not
    reproducible; record as an open question below).
  - `freqTarget` is only written on the water path, where it is always set before use.
- gta-reversed: hook `.Reversed = false` ("bad"). Its unhooked C++ differs from the bytes in these places (all CONFIRMED,
  port keeps the original):
  1. Sandstorm volume: gr `log10f(2) * log2f(Sandstorm)`; original multiplies by 20 and subtracts 6 (0x4D6EDF/0x4D6EE5).
  2. Zone evaluation condition: gr skips zones whenever `Sandstorm > 0`; original still evaluates zones when Sandstorm > 0
     but no sandstorm track was chosen (indoors / PlayerNoRain / CamNoRain, 0x4D6EF3).
  3. Zone 13: gr sets no volume; original sets DistVol (0x4D6FB8-0x4D6FF2).
  4. gr initialises zonePos to zero; original leaves it uninitialised for box zones.
  5. Branch condition inverted: gr does the water/riot path when `track != -1`; original does it when `track == -1`
     (0x4D724A `jl`), and the track path when `track >= 0`.
  6. Mission track while not stopped: gr stops when `TrackID == track`; original stops when `TrackID != track` (0x4D727B).
  7. Same-track volume ramp: gr `min(m_Volume + 1, m_Volume)` (no-op); original `min(m_Volume + 1, volume)` (0x4D72C4).
  8. Riot branch inverted: gr plays track 166 when `!(riots || !outside)`; original when `riots && currArea == 0`
     (0x4D749B/0x4D74A7).
  9. Water distance scale: gr `/ SQRT_3` (x 0.57735); original `* 0.5714285970f` (0x4D7338).
  gr-deviation rows (notsa::step_to lines 320/322): REFUTED for ordinary values - the inline code is a step-toward with the
  same up/down step; only NaN differs (original skips the store when the compare is unordered, 0x4D740B/0x4D7469). Harmless.
- Open questions: 0x4D7028 / 0x4D6FC7 with box zones 5/10/13: value of the uninitialised `zonePos` is whatever the stack
  held (not determinable statically).
