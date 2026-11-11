# CAERadioTrackManager (AUDIO) - RE-SPEC-REST group A

## 0x4E94C0 CAERadioTrackManager::ChooseIdentIndex  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall int32 (this = ECX unused, int8 stationId [esp+4])`, `ret 4`, result in EAX. ECX is never read.
- Callers: QueueUpTracksForStation 0x4EA670. Callees: CAEAudioUtility::GetRandomNumberInRange<int32> 0x4D9C10 (cdecl (min, max),
  inclusive: `min + (int)((rand() & 0xFFFF) * k * (max - min + 1))` via 0x821B40), CStats::GetStatValue 0x558E40.
- Globals (R): ident range table `int32 [station][2]` at 0x8C8FB0 (stride 8: {first, last}; .data, values per station 0..11:
  0x782/0x782, 0xDC/0xE6, 0x164/0x16C, 0x1F6/0x200, 0x333/0x33A, 0x782/0x782, 0x44C/0x456, 0x4E0/0x4EA, 0x56F/0x576, 0x5FC/0x605,
  0x6A2/0x6A9, 0x71D/0x724; entry 12 overlaps the music-count table 0x8C9010), CAERadioTrackManager::m_nIdentIndexHistory 0xB62980
  (int32 [station][8], stride 0x20).
- Behaviour:
  1. s = (int32)(int8)stationId (movsx, 0x4E94C6). If table[s].first == 0x782 (the "no track" marker) return -1 (0x4E94D0-0x4E94D9).
  2. Loop (0x4E94E1): r = GetRandomNumberInRange(table[s].first, table[s].last) (both re-read each iteration).
  3. If stationId == 6 (RADIO_MODERN_HIP_HOP, byte compare 0x4E94F9) and r == 0x44C (1100): if GetStatValue(0x152 = 338,
     STAT_ARE_YOU_GOING_TO_SAN_FIERRO_MISSION_ACCOMPLISHED) == 0.0f [0x858B50] -> reroll (goto 2). The compare is
     `fcomp [0x858B50]; fnstsw; test ah,0x44; jnp` (0x4E9512-0x4E9520): taken only for exact equality (C3 alone); NaN (C3|C2) does
     not reroll.
  4. n = table[s].last - table[s].first - 1 (signed, 0x4E9530-0x4E9539); limit = (n > 8) ? 8 : n (0x4E9540-0x4E954A).
     for j = 0 .. limit-1: if history[s][j] == r -> reroll (goto 2). (limit <= 0: no check.)
  5. Return r.
- Edge cases: no iteration cap - if every candidate is in the history the loop spins forever (cannot happen with these tables since
  limit <= range-1). History length checked is min(range-1, 8), so at least one candidate always remains.
- gta-reversed: plugin::CallAndReturn stub (AERadioTrackManager.cpp:696), not reversed.

## 0x4E9570 CAERadioTrackManager::ChooseAdvertIndex  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall int32 (this = ECX unused, int8 stationId [esp+4])`, `ret 4`, result EAX.
- Callers: QueueUpTracksForStation 0x4EA670, CheckForTrackConcatenation 0x4EA930, Service 0x4EB9A0. Callees:
  GetRandomNumberInRange<int32> 0x4D9C10 (inclusive).
- Globals (R): advert range {first = [0x8C8B88] (.data, 0x42), last = [0x8C8B8C] (0x86)} (re-read every iteration); per-station
  advert blacklist `int32 [station][23]` at 0x8CADD0 (stride 0x5C); m_nAdvertIndexHistory 0xB620C0 (int32 [station][40], stride 0xA0).
- Behaviour: s = (int8)stationId sign-extended (0x4E9571).
  1. r = GetRandomNumberInRange([0x8C8B88], [0x8C8B8C]) (0x4E9582-0x4E9590).
  2. for j = 0..22 (`cmp ecx,0x17; jl`): if blacklist[s][j] == r -> goto 1 (0x4E95A0-0x4E95AB).
  3. for j = 0..39 (`cmp edx,0x28`): if history[s][j] == r -> goto 1 (0x4E95C0-0x4E95CB).
  4. Return r.
- Edge cases: unbounded retry loop; all 40 history slots are always compared (unused slots hold -1, never a valid advert).
- gta-reversed: plugin::CallAndReturn stub (AERadioTrackManager.cpp:701).

## 0x4E95E0 CAERadioTrackManager::ChooseDJBanterIndexFromList  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall int32 (this = ECX unused, int8 stationId [esp+4], int32 (*list)[2] [esp+8])`, `ret 8`, result EAX.
  `list` points to a per-station {first, last} range table (stride 8). ECX is never read.
- Callers: ChooseDJBanterIndex 0x4EA2D0 (5 sites; lists 0x8C8CB0, 0x8C8D70, 0x8C8DD0, 0x8C8E30, 0x8C8E90, 0x8C8EF0, 0x8C8F50).
  Callees: GetRandomNumberInRange<int32> 0x4D9C10 (inclusive).
- Globals (R): the GENERIC DJ banter range table 0x8C8CB0 (always this one for the history depth, whatever `list` is),
  m_nDJBanterIndexHistory 0xB61D78 (int32 [station][15], stride 0x3C).
- Behaviour:
  1. s = (int8)stationId sign-extended (0x4E95E1). first = list[s].first; if first == 0x782 return -1 (0x4E95F2).
  2. n = list[s].last - first + 1 (0x4E95FE-0x4E9604); r = GetRandomNumberInRange(0, n-1) (0x4E9609).
  3. If n <= 0 (`test esi,esi; jle`, 0x4E9621) return -1 (the code tests the low byte of the counter, which is 0 here, then returns
     the -1 still held in EBP: 0x4E9690-0x4E96A6).
  4. depth = (0x8C8CB0[s].last - 0x8C8CB0[s].first) - 1 (0x4E9623-0x4E9636); limit = (depth > 15) ? 15 : depth.
  5. for i = 0 .. n-1: cand = ((i + r) % n) + first (signed `cdq; idiv`, 0x4E9648-0x4E9650); if no j in 0..limit-1 has
     history[s][j] == cand -> return cand (0x4E9660 `jge` -> 0x4E969D). Otherwise next i.
  6. All n candidates in history: return -1 (0x4E9687).
- Edge cases: the history depth comes from the generic table, not from `list` (original behaviour; keep). With limit <= 0 the first
  candidate (r + first) is returned.
- gta-reversed: plugin::CallMethodAndReturn stub (AERadioTrackManager.cpp:716).

## 0x4E9820 CAERadioTrackManager::StopRadio  (catalogue: specified/REA, port level 7)
- Convention: `__thiscall void (this = ECX, tVehicleAudioSettings* settings [esp+4], bool duringPause [esp+8])`, `ret 8`.
  The body is split by a relocated block: 0x4E9CA6 `jmp 0x406AAD` (`mov al, [0xB7CB49]`; `jmp 0x4E9CAB`). Registers: EBP = 0,
  EBX = -1 throughout.
- Callers (8): CheckForPause 0x4EA590, CheckForStationRetune 0x4EB660 (2), Service 0x4EB9A0, CAudioEngine::StopRadio 0x506F70
  (tail call), CAudioEngine::Reset 0x507A90, ::Shutdown 0x507CB0, ::PreloadCutsceneTrack 0x507E30, ::PreloadBeatTrack 0x507F40.
  Callees: CAudioEngine::ReportFrontendAudioEvent 0x506EA0 (this = AudioEngine 0xB6BC90), GetRandomNumberInRange<int32> 0x4D9C10.
- Globals (R): CTimer::m_UserPause 0xB7CB49, m_CodePause 0xB7CB48, m_snTimeInMilliseconds 0xB7CB84,
  m_snTimeInMillisecondsPauseMode 0xB7CB7C, CClock::ms_nGameClockDays 0xB70154, ms_nGameClockHours 0xB70153; table 0x8CAD50
  (int32 per talk-show index: 0x26160, 0x28C58, 0x2B750, 0x56EA0, ...).
- Fields (gr names): +0x00 m_bInitialised, +0x01 m_bDisplayStationName, +0x03 m_bEnabledInPauseMode, +0x06 m_bRetuneJustStarted,
  +0x16/+0x17 m_nSavedGameClockDays/Hours (u8), +0x18 + 4*s m_aListenTimes[s], +0x58 m_nSavedTimeMs, +0x5C m_nRetuneStartedTime,
  +0x68 m_nMode, +0x6C m_nStationsListed, +0x70 m_nStationsListDown, +0x74 m_nSavedRadioStationId, +0x78 m_iRadioStationMenuRequest,
  +0x7C m_iRadioStationScriptRequest; m_ActiveSettings: +0xC4 TrackQueue[0], +0xC8 [1], +0xCC [2], +0xD8 CurrTrackID, +0xE0
  PlayTime, +0xE4 TrackLengthMs, +0xE9 StationID (int8), +0xEA BassSetting (u8), +0xF0/+0xF1/+0xF2 TrackTypes[0..2], +0xF5
  CurrTrackType, +0xF7 TrackIndices[0]; m_aRadioState[s] at +0x100 + 0x2C*s (R[s] below): +0x00/+0x04/+0x08 m_aElapsed[0..2], +0x0C
  m_iTimeInPauseModeInMs, +0x10 m_iTimeInMs, +0x14 m_iTrackPlayTime, +0x18/+0x1C/+0x20 m_aTrackQueue[0..2], +0x24/+0x25/+0x26
  m_aTrackTypes[0..2], +0x27 m_nGameClockDays, +0x28 m_nGameClockHours. s = (int8)StationID, re-read (movsx) before EVERY store.
- Behaviour:
  1. Only if m_nMode (+0x68) is 0, 1 or 2 (0x4E9825-0x4E983A), else skip to step 6:
     a. if !(m_UserPause || m_CodePause) or duringPause: m_nMode = 3 (0x4E9840-0x4E985A). (Paused and !duringPause: mode unchanged.)
     b. Reset R[s] in this exact store order (0x4E9861-0x4E9927): Elapsed[0] = 0; TrackPlayTime = -1; TrackQueue[0] = -1;
        TrackTypes[0] = 6; Elapsed[1] = 0; TrackPlayTime = -1; TrackQueue[1] = -1; TrackTypes[1] = 6; Elapsed[2] = 0; TrackPlayTime = -1;
        TrackQueue[2] = -1; TrackTypes[2] = 6. Then R[s].TimeInMs = m_snTimeInMilliseconds; R[s].GameClockDays = ms_nGameClockDays;
        R[s].GameClockHours = ms_nGameClockHours.
     c. p = R[s].TimeInPauseModeInMs; if p >= 0 and s != 0 and s != 13: m_aListenTimes[s] += m_snTimeInMillisecondsPauseMode - p
        (0x4E997C-0x4E999B).
     d. If StationID == 13 (RADIO_OFF): this+0x33C (= R[13].Elapsed[0]) = 0; go to step 6 (0x4E99A5-0x4E99AF).
     e. R[s].Elapsed[0] = TrackLengthMs - PlayTime - 100 (0x4E99B4-0x4E99CB).
     f. Jump table 0x4E9D8C on (int8)CurrTrackType (+0xF5), unsigned `ja` above 7 -> step 6:
        - 0, 1, 2, 5 (ident/advert/banter/outro, 0x4E99E9): R[s].TrackPlayTime = PlayTime; R[s].TrackQueue[0] = CurrTrackID;
          R[s].TrackTypes[0] = CurrTrackType.
        - 3 INTRO (0x4E9A33): if StationID == 11: this+0x2E8 (= R[11].Elapsed[1]) = table 0x8CAD50[(int8)TrackIndices[0]]; else
          R[s].Elapsed[1] = 0x249F0 (150000). Then R[s].Elapsed[2] = 0x1388 (5000); R[s].TrackPlayTime = PlayTime; then the
          "two-entry" save below with three slots.
        - 4 TRACK and 7 USER_TRACK (0x4E9BB5): R[s].Elapsed[1] = 0x1388; R[s].TrackPlayTime = PlayTime; then the two-slot save.
        - 6 NONE: nothing.
        Save for INTRO (0x4E9A8F): if TrackQueue[0] == CurrTrackID: R[s].Queue[0..2] = TrackQueue[0..2], R[s].Types[0..2] =
        TrackTypes[0..2] (interleaved queue/type per slot); else: R[s].Queue[0] = CurrTrackID, Types[0] = CurrTrackType, Queue[1] =
        TrackQueue[0], Types[1] = TrackTypes[0], Queue[2] = TrackQueue[1], Types[2] = TrackTypes[1].
        Save for TRACK (0x4E9BE1): same but only two slots: equal -> Queue/Types[0..1] = TrackQueue/Types[0..1]; else Queue[0] =
        CurrTrackID, Types[0] = CurrTrackType, Queue[1] = TrackQueue[0], Types[1] = TrackTypes[0].
  6. (0x4E9C9F) m_bInitialised = 0; m_bEnabledInPauseMode = 0. If (m_UserPause || m_CodePause) and !duringPause:
     m_iRadioStationMenuRequest = -1; m_nRetuneStartedTime = 0 (0x4E9CAB-0x4E9CC3).
  7. If settings == NULL (0x4E9CCA): if duringPause { m_nStationsListed = 0; m_nStationsListDown = 0; m_bRetuneJustStarted = 0 };
     return.
  8. settings != NULL: m_nStationsListed = 0; m_nStationsListDown = 0; m_iRadioStationScriptRequest = -1; m_bDisplayStationName = 0;
     m_bRetuneJustStarted = 0; AudioEngine.ReportFrontendAudioEvent(0x23 AE_FRONTEND_RADIO_RETUNE_STOP, 0.0f (int 0 pushed),
     1.0f (imm 0x3F800000)) (0x4E9CD2-0x4E9CF0). If StationID == -1 -> StationID = 13. settings+0x1A (RadioStation) = StationID;
     settings+0x06 (BassSetting) = this+0xEA.
  9. If !(m_nMode == 7 && m_bInitialised == 0 && m_nStationsListed == 0 && m_nStationsListDown == 0): m_nSavedTimeMs =
     m_snTimeInMilliseconds; m_nSavedGameClockDays = ms_nGameClockDays; m_nSavedGameClockHours = ms_nGameClockHours;
     m_nSavedRadioStationId = (int32)(int8)StationID (0x4E9D16-0x4E9D4C). (m_bInitialised was cleared in step 6 and the list
     counters in step 8, so this reduces to m_nMode != 7.)
  10. If StationID == 0 (AA): StationID = (int8)GetRandomNumberInRange(1, 13) (0x4E9D59-0x4E9D66). Return.
- Edge cases: R[13] (RADIO_OFF) is a real slot of the array (14 entries); for StationID 13 step 1b writes it before step 1d.
  The AA -> random station replacement can yield 13 (RADIO_OFF) since the range is inclusive.
- gta-reversed: plugin::CallMethod stub (AERadioTrackManager.cpp:691).

## 0x4EA270 CAERadioTrackManager::ChooseMusicTrackIndex  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall int8 (this = ECX, int8 stationId [esp+4])`, `ret 4`, result in AL (upper EAX bits: see below).
- Callers: QueueUpTracksForStation 0x4EA670 (3 sites). Callees: CAERadioTrackManager::ChooseTalkRadioShow 0x4E8E40,
  GetRandomNumberInRange<int32> 0x4D9C10 (inclusive).
- Globals (R): music track count per station `int32 [station]` at 0x8C9010 (.data: 2, 12, 15, 17, 17, 16, 16, 15, 13, 16, 18, 31, 60,
  63), m_nMusicTrackIndexHistory 0xB62B40 (int8 [station][20], stride 20).
- Behaviour:
  1. If stationId == 11 (RADIO_TALK, 0x4EA274): tail-calls ChooseTalkRadioShow() (ECX still = this; `call` + `ret 4`, 0x4EA278) and
     returns its AL.
  2. s = (int8)stationId; loop (0x4EA286): r = GetRandomNumberInRange(0, count[s] - 1) (full int32 in EAX; the result byte is AL).
  3. limit = min(count[s] - 2, 20) (signed, 0x4EA2A2-0x4EA2AF; count re-read). for j (int8 counter BL) = 0 .. limit-1: if
     (int8)AL == history[s][j] (byte compare, 0x4EA2BB) -> goto 2.
  4. Return r (EAX = full random int; callers use AL).
- gta-reversed: plugin::CallAndReturn stub (AERadioTrackManager.cpp:706).

## 0x4EA2D0 CAERadioTrackManager::ChooseDJBanterIndex  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall int32 (this = ECX, int8 stationId [esp+4])`, `ret 4`, result EAX (-1 = none).
- Callers: QueueUpTracksForStation 0x4EA670. Callees: ChooseDJBanterIndexFromList 0x4E95E0 (thiscall, this passed on),
  CGameLogic::LaRiotsActiveHere 0x441C10, CAEAudioUtility::ResolveProbability 0x4D9C80 (cdecl, f32 prob pushed as immediate),
  CWeather::ForecastWeather 0x72A590 (cdecl (weatherType, numSteps)).
- Globals (R): m_nSpecialDJBanterPending 0xB62C73 (int8), m_nSpecialDJBanterIndex 0xB62C72 (int8), CGame::currArea 0xB72914,
  CClock::ms_nGameClockHours 0xB70153 (u8), tables: 0x8C8BF0 (int32 [station][2], pending == 0 special), 0x8C8C50 (int32 [station][2],
  pending == 1 special), 0x8CB280 (int32 [station][22], pending == 2 special), m_nDJBanterIndexHistory 0xB61D78 (int32 [station][15]),
  DJ lists 0x8C8CB0 (generic), 0x8C8D70 (evening), 0x8C8DD0 (morning), 0x8C8E30 (night / riots), 0x8C8E90 (rain), 0x8C8EF0 (dead,
  see below), 0x8C8F50 (fog).
- Behaviour (s = (int8)stationId; cand = -1):
  1. Special banter by pending = (int8)[0xB62C73] (0x4EA2D0-0x4EA2E8):
     - 0: cand = 0x8C8BF0[s][0] (0x4EA348).
     - 1: idx = (int8)[0xB62C72]; if idx == 0 -> cand = 0x8C8C50[s][0]; else if idx == 1 and 0x8C8C50[s][0] != 0x8C8C50[s][1] ->
       cand = 0x8C8C50[s][1]; else (idx not 0/1, or the two equal) -> go to step 2 with cand = -1 (0x4EA30A-0x4EA33A).
     - 2: cand = 0x8CB280[s*22 + (int8)[0xB62C72]] (0x4EA2EE-0x4EA301).
     - other: go to the `cand != -1` test below with cand = -1 (-> step 2).
     Then (0x4EA34F) if cand == 0x782 -> cand = -1, go to step 2. If cand >= 0: if any of the 15 history[s][j] == cand -> cand = -1,
     go to step 2 (0x4EA360-0x4EA37B). Then if cand != -1 return cand (0x4EA37D; a negative cand other than -1 is returned as is).
  2. If stationId == 0 (RADIO_EMERGENCY_AA, 0x4EA38A): return FromList(0, LaRiotsActiveHere() ? 0x8C8E30 : 0x8C8CB0) (0x4EA38E-0x4EA3B2).
  3. If !ResolveProbability(0.6f) (imm 0x3F19999A, 0x4EA3BD) or CGame::currArea != 0 (0x4EA3D7): return -1.
  4. h = ms_nGameClockHours (read once, 0x4EA3DF). Weather lists:
     a. if ForecastWeather(16 RAINY_COUNTRYSIDE, 3) || ForecastWeather(8 RAINY_SF, 3): if ResolveProbability(0.5f) { r =
        FromList(s, 0x8C8E90); if r != -1 return r } -> step 5 (the fog test is skipped) (0x4EA3E5-0x4EA407, 0x4EA4C1-0x4EA4E6).
     b. else: calls ForecastWeather(0,3), (2,3), (13,3), (6,3), (11,3), (17,3) in this order, stopping at the first true
        (0x4EA40D-0x4EA46B). If one was true: `if (h >= 8 && h < 6)` (0x4EA46D-0x4EA475) - never true, so the 0.5 roll and list
        0x8C8EF0 (proposed: sunny list) are dead code. In all cases continue: if ForecastWeather(9 FOGGY_SF, 3) &&
        ResolveProbability(0.5f) { r = FromList(s, 0x8C8F50); if r != -1 return r } (0x4EA494-0x4EA4BF).
  5. Time of day (0x4EA4EC): if 6 <= h < 9 -> list 0x8C8DD0; else if 18 <= h < 21 -> 0x8C8D70; else if h >= 22 or h < 3 -> 0x8C8E30;
     else none. With a list: if ResolveProbability(0.3f) (imm 0x3E99999A) { r = FromList(s, list); if r != -1 return r }.
  6. Return FromList(s, 0x8C8CB0) (0x4EA56B).
- Original bug (keep): the impossible hour test at 0x4EA46D-0x4EA475 (`cmp bl,8; jb; cmp bl,6; jae`) makes the sunny-weather banter
  list unreachable; the six ForecastWeather calls still execute (they have no side effects beyond reading state).
- gta-reversed: plugin::CallAndReturn stub (AERadioTrackManager.cpp:711).

## 0x4EA670 CAERadioTrackManager::QueueUpTracksForStation  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall bool (this = ECX, int8 stationId [esp+4], int8* trackCount [esp+8], eTrackType type [esp+0xC],
  tRadioSettings* settings [esp+0x10])`, `ret 0x10`, result in AL (1 = queued, 0 = nothing queued). Entry 0x4EA670 is `jmp 0x4020D0`
  (relocated stub: `movsx eax, byte [esp+0xC]` = type, then `jmp 0x4EA675`). ECX (this) is passed unchanged to the Choose* calls.
- Callers: CheckForTrackConcatenation 0x4EA930 (4), TrackRadioStation 0x4EAC30 (15), ChooseTracksForStation 0x4EB180 (15).
  Callees: ChooseIdentIndex 0x4E94C0, ChooseAdvertIndex 0x4E9570, ChooseMusicTrackIndex 0x4EA270, ChooseDJBanterIndex 0x4EA2D0,
  CAEUserRadioTrackManager::SelectUserTrackIndex 0x4F3250 (this = AEUserRadioTrackManager 0xB6B970), GetRandomNumberInRange<int32>
  0x4D9C10 (cdecl (first, last), inclusive).
- Data tables (R, .data): music track id `int32 [station][31]` at 0x8C9040; intro range `int32 [station][31][2]` at 0x8C9610;
  outro range `int32 [station][31][2]` at 0x8CA1B0 ({first, last} pairs, stride 8, station stride 31*8).
- settings fields (tRadioSettings, gr AERadioTrackManager.h:20): +0x00 TrackQueue int32[5], +0x2C TrackTypes int8[5],
  +0x33 TrackIndices int8[5] (so byte +0x32 + n is TrackIndices[n-1]). Below `c` = *trackCount, re-read from memory at every use;
  "advance" = `++*trackCount` (byte inc).
- Behaviour: jump table 0x4EA918 on (int32)(int8)type, unsigned `cmp eax,5; ja` (0x4EA675): any type outside 0..5 (incl. negative)
  -> return 1 with nothing written (0x4EA90F).
  - 0 INDENT (0x4EA871): if stationId == 12 (RADIO_USER_TRACKS) return 0. q = ChooseIdentIndex(stationId); queue[c] = q; if queue[c]
    == -1 return 0 (slot keeps -1, count not advanced). types[c] = 0; advance; return 1.
  - 1 ADVERT (0x4EA8AD, no user-track check): queue[c] = ChooseAdvertIndex(stationId); types[c] = 1; advance; return 1.
  - 2 DJ_BANTER (0x4EA8DB): if stationId == 12 return 0. queue[c] = ChooseDJBanterIndex(stationId); if -1 return 0; types[c] = 2;
    advance; return 1.
  - 3 INTRO (0x4EA732): if stationId == 12 return 0. idx[c] = ChooseMusicTrackIndex(stationId) (AL); k = s*31 + idx[c] (idx
    sign-extended); queue[c] = Rand(intro[k].first, intro[k].last); types[c] = 3; advance; idx[c] = idx[c-1]; queue[c] =
    track[s*31 + idx[c]]; types[c] = 4; advance; idx[c] = idx[c-1]; queue[c] = Rand(outro[k'].first, outro[k'].last) with k' from the
    copied idx; types[c] = 5; advance; return 1 (three entries queued).
  - 4 TRACK (0x4EA688): if stationId == 12: q = SelectUserTrackIndex(); queue[c] = q; types[c] = 7 (USER_TRACK); idx[c] = low byte of
    queue[c]; advance; return 1. Otherwise: idx[c] = ChooseMusicTrackIndex(stationId); queue[c] = track[s*31 + idx[c]]; types[c] = 4;
    advance; idx[c] = idx[c-1]; then the OUTRO tail (0x4EA84D): queue[c] = Rand(outro[s*31 + idx[c]]); types[c] = 5; advance; return 1.
  - 5 OUTRO (0x4EA80A): if stationId == 12 return 0. idx[c] = ChooseMusicTrackIndex(stationId); queue[c] = Rand(outro[s*31 + idx[c]]);
    types[c] = 5; advance; return 1.
- Edge cases: no bound check on *trackCount (callers keep it < 5); a TRACK/INTRO request writes 2/3 slots. The order of stores
  (index first, then queue, then type, then the count byte) is as listed.
- gta-reversed: plugin::CallMethodAndReturn stub (AERadioTrackManager.cpp:686).

## 0x4EA930 CAERadioTrackManager::CheckForTrackConcatenation  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void (this = ECX: CAERadioTrackManager*)`, plain `ret`. One dword local; its byte +2 is the track counter
  `count` passed by address to QueueUpTracksForStation (`lea [esp+0x1a]` at the call sites); BL holds a separate copy.
- Callers: Service 0x4EB9A0. Callees: CAEUserRadioTrackManager::GetUserTrackPlayMode 0x4F3330 / SetUserTrackIndex 0x4F3340 /
  SelectUserTrackIndex 0x4F3250 (this = AEUserRadioTrackManager 0xB6B970), CAEAudioHardware::PlayTrack 0x4D8F10 and GetActiveTrackID
  0x4D8F80 (this = AEAudioHardware 0xB5F8B8), CAEAudioUtility::ResolveProbability 0x4D9C80, ChooseAdvertIndex 0x4E9570,
  QueueUpTracksForStation 0x4EA670 (args (station byte, &count, type, &m_ActiveSettings = this+0xC4)).
- Globals (R): FrontEndMenuManager.m_RadioMode 0xBA67F8 (int8, FrontEndMenuManager+0xB0).
- Fields (gr names; m_ActiveSettings at +0xC4): +0x08+s m_nTracksInARow[s] (int8 compare), +0xC4 TrackQueue[5] (int32),
  +0xDC PrevTrackID, +0xE0 PlayTime, +0xE8 TrackFlags (u8), +0xE9 StationID (int8), +0xF0 TrackTypes[5], +0xF6 PrevTrackType,
  +0xF7 TrackIndices[5], +0xFD PrevTrackIdx, +0x36C m_nUserTrackPlayMode (u8).
- Behaviour:
  1. count = BL = 1 (0x4EA93C-0x4EA93F).
  2. User-track mode change (0x4EA935-0x4EAA02), only if StationID == 12: prev = this+0x36C; if prev != GetUserTrackPlayMode():
     if prev == 2 or GetUserTrackPlayMode() == 2 (second call, only when prev != 2): if PlayTime (+0xE0) != -4 {
     SetUserTrackIndex(TrackQueue[0]); TrackQueue[1] = SelectUserTrackIndex(); TrackTypes[1] = 7; TrackIndices[1] = low byte of
     TrackQueue[1]; BL = count = 2; PlayTrack(TrackQueue[0], TrackQueue[1], 0, TrackFlags, TrackTypes[0] == 7, TrackTypes[1] == 7) }.
     Then (whenever prev differed) this+0x36C = GetUserTrackPlayMode() (third call, 0x4EA9F8).
  3. q1 = TrackQueue[1] (+0xC8, read before the call); if AEAudioHardware.GetActiveTrackID() != q1 or q1 < 0 -> return (0x4EAA18-0x4EAA22).
  4. Advance (inline SwitchToNextTrack, 0x4EAA28-0x4EAA8F): PrevTrackID = TrackQueue[0]; PrevTrackType = TrackTypes[0]; PrevTrackIdx =
     TrackIndices[0]; for i = 0..3: TrackQueue[i] = TrackQueue[i+1], TrackTypes[i] = TrackTypes[i+1], TrackIndices[i] =
     TrackIndices[i+1] (interleaved per i); then TrackQueue[4] = -1, TrackTypes[4] = 6, TrackIndices[4] = -1.
  5. If TrackQueue[1] != -1 -> step 7 (0x4EAA95).
  6. Queue more (only TrackQueue[0] is valid now):
     - StationID == 12 (0x4EAAA1): if m_RadioMode == 0 and ResolveProbability(0.17f) (imm 0x3E2E147B): TrackQueue[BL] =
       ChooseAdvertIndex(12); TrackTypes[BL] = 1; BL++. Then twice: TrackQueue[BL] = SelectUserTrackIndex(); TrackTypes[BL] = 7;
       TrackIndices[BL] = low byte of TrackQueue[BL]; (BL++ only after the first). The stack `count` is not updated here.
     - else t = TrackTypes[0]: if t not in {3, 4, 5} -> QueueUp(INTRO 3) (0x4EAB29). Else if StationID == 0 -> QueueUp(DJ_BANTER 2)
       (0x4EAB3B). Else if m_nTracksInARow[StationID] < 2 (signed) and ResolveProbability(0.5f): { if ResolveProbability(0.5f)
       QueueUp(INDENT 0); QueueUp(INTRO 3) } (0x4EAB4A-0x4EAB91). Otherwise (count >= 2, or the first 0.5 roll failed): if
       ResolveProbability(0.5f) QueueUp(INDENT 0); if !QueueUp(DJ_BANTER 2) QueueUp(ADVERT 1) (0x4EAB93-0x4EABEB).
  7. PlayTrack(TrackQueue[0], TrackQueue[1], 0, TrackFlags (zero-extended), TrackTypes[0] == 7, TrackTypes[1] == 7) (0x4EABF0-0x4EAC20).
- Edge cases: in the INTRO call at 0x4EAB31 the pushed station dword is EAX = 0xFFFFFF00 | station (upper bits from `or eax,-1`
  at 0x4EAA7D); the callee only reads the low byte. ResolveProbability takes the f32 bit pattern pushed as an immediate.
- gta-reversed: plugin::CallMethod stub (AERadioTrackManager.cpp:361) with a commented-out partial draft; the draft differs from
  the bytes (it tests `utPlayMode != 0` instead of comparing with the saved +0x36C mode, only checks mode == 2 once, and stops
  before the queue refill) - port from the bytes.

## 0x4EAC30 CAERadioTrackManager::TrackRadioStation  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall bool (this = ECX, int8 stationId [esp+4], bool skipTrack [esp+8])`, `ret 8`, result AL (1 = resumed
  state queued into m_RequestedSettings, 0 = state too old / unusable). `sub esp,0x10` locals: byte `count` (L+3 = [esp+0x13] after
  the 4 pushes, passed by address to QueueUpTracksForStation), dword `sum` ([esp+0x14]); the argument slots are reused as scratch
  (`T` in the stationId slot [esp+0x24]).
- Callers: StartRadio 0x4EB3C0. Callees: QueueUpTracksForStation 0x4EA670 (settings = &m_RequestedSettings = this+0x88),
  ResolveProbability 0x4D9C80, AddMusicTrackIndexToHistory 0x4E96C0, CAEUserRadioTrackManager::SelectUserTrackIndex 0x4F3250
  (this = 0xB6B970).
- Globals (R): CClock::ms_nGameClockDays 0xB70154 (u8), ms_nGameClockMonth 0xB70155, ms_nGameClockHours 0xB70153 (u8),
  CClock::daysInMonth 0x8CCF24 (u8[12]), CTimer::m_snTimeInMilliseconds 0xB7CB84, FrontEndMenuManager.m_RadioMode 0xBA67F8.
- Fields: m_RequestedSettings at +0x88: +0x88 TrackQueue[5], +0x8C TrackQueue[1], +0xA4 PlayTime, +0xB4 TrackTypes[5], +0xBB
  TrackIndices[5]; m_aRadioState[s] R at +0x100 + 0x2C*s (layout as in StopRadio 0x4E9820: Elapsed[3] +0, TimeInMs +0x10,
  TrackPlayTime +0x14, TrackQueue[3] +0x18, TrackTypes[3] +0x24, GameClockDays +0x27, GameClockHours +0x28). s = (int8)stationId.
- Behaviour:
  1. Age check (0x4EAC44-0x4EACAA): if R.GameClockHours >= 0 and R.GameClockDays >= 0 (signed bytes): d = ms_nGameClockDays -
     R.GameClockDays; if d < 0 { m = (int8)(ms_nGameClockMonth - 1); if m < 0 m += 12; d += daysInMonth[m] }; if d*24 -
     R.GameClockHours + ms_nGameClockHours > 5 -> return 0.
  2. T = m_snTimeInMilliseconds - R.TimeInMs; if T <= 7000 (0x1B58) T = 7000 (0x4EACB0-0x4EACC5). If skipTrack: e = R.Elapsed[0] + 1;
     if T <= e then T = e (0x4EACCA-0x4EACE2).
  3. Clear m_RequestedSettings: for i = 0..4: TrackQueue[i] = -1, TrackTypes[i] = 6, TrackIndices[i] = -1 (0x4EACE6-0x4EAD22).
     count = 0, sum = 0.
  4. for i = 0..2 (0x4EAD31): sum += R.Elapsed[i]; if T <= sum -> found slot i (step 5). After the loop (T > sum of all three):
     - T <= sum + 7000: if stationId == 12 QueueUp(12, TRACK 4) twice, else QueueUp(s, INTRO 3); PlayTime = min(T - sum, 5000)
       (`cmp ebp,0x1388; jl`, 0x4EAFD3-0x4EAFE9); return 1.
     - else T <= sum + 155000 (0x25D78): QueueUp(s, TRACK 4); PlayTime = T - sum - 5000 (0x4EB004-0x4EB01F); return 1.
     - else T <= sum + 160000 (0x27100): if stationId == 12: QueueUp(12, TRACK 4); if m_RadioMode == 0 and ResolveProbability(0.17f)
       (imm 0x3E2E147B) QueueUp(12, ADVERT 1). Else: QueueUp(s, OUTRO 5); AddMusicTrackIndexToHistory(s, TrackIndices[count-1]
       (zero-extended byte +0xBA+count)); if stationId == 0 QueueUp(0, DJ_BANTER 2); else if ResolveProbability(0.5f)
       { if ResolveProbability(0.5f) QueueUp(INDENT 0); QueueUp(INTRO 3) } else { if ResolveProbability(0.5f) QueueUp(INDENT 0);
       if !QueueUp(DJ_BANTER 2) QueueUp(ADVERT 1) }. Then PlayTime = T - sum - 155000 (0x4EB12F-0x4EB139); return 1.
     - else return 0.
  5. Found slot i (0x4EAD96): PlayTime = (i == 0) ? R.TrackPlayTime + T : R.Elapsed[i] - sum + T. Then jump table 0x4EB158 on
     type = (int8)R.TrackTypes[i] (unsigned, > 7 -> return 0):
     - 0, 1, 2 (0x4EADEE): TrackQueue[0] = R.TrackQueue[i]; TrackTypes[0] = type; count = 1; then stationId == 12 ? QueueUp(12, TRACK 4)
       : QueueUp(s, INTRO 3); return 1.
     - 5 OUTRO (0x4EAE44): same copy, count = 1; if stationId == 0 QueueUp(0, DJ_BANTER 2) else { if ResolveProbability(0.5f)
       QueueUp(INDENT 0); if !QueueUp(DJ_BANTER 2) QueueUp(ADVERT 1) }; return 1.
     - 3 INTRO (0x4EAED5): for j = 0..2: TrackQueue[j] = R.TrackQueue[i+j]; TrackTypes[j] = R.TrackTypes[i+j]; return 1.
     - 4 TRACK (0x4EAF80): same for j = 0..1; return 1.
     - 7 USER_TRACK (0x4EAF17): same for j = 0..1; if TrackQueue[1] (+0x8C) == -1: TrackQueue[1] = SelectUserTrackIndex();
       TrackTypes[1] = 7; TrackIndices[1] = low byte; return 1.
     - 6 NONE: return 0.
- Edge cases / original quirks: for INTRO/TRACK copies with i > 0 the source index i+j runs past R.TrackQueue[2] / R.TrackTypes[2]
  (reads R.TrackTypes bytes as a dword, and Days/Hours/next slot bytes as types) - keep. The copy paths never set TrackIndices.
  All QueueUp calls pass `count` by address and the result of QueueUp is used only where stated.
- gta-reversed: plugin::CallMethodAndReturn stub (AERadioTrackManager.cpp:681).
