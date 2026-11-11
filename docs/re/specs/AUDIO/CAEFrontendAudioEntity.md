# CAEFrontendAudioEntity (AUDIO) - RE-SPEC-REST group A

## 0x4DD4A0 CAEFrontendAudioEntity::AddAudioEvent  (catalogue: specified/REA, port level 5)
- Convention: `__thiscall void (this = ECX: CAEFrontendAudioEntity*, int32 event [esp+4], float volumeBoost [esp+8], float speed
  [esp+0xC])`, `ret 0xC`. SEH frame (handler 0x83BF3B), `sub esp,0xd4`, EBX = 0 and ESI = this throughout. Executed under SA:MP.
  The epilogue is relocated: 0x4DEC7B `jmp 0x406A86` (EH state = -1) `jmp 0x4DEC80`.
- Callers (9): CAudioEngine::ReportFrontendAudioEvent 0x506EA0 (tail call), StartLoadingTune 0x507410, Service 0x507750 (2),
  InitialisePostLoading 0x5078F0, Reset 0x507A90 (2), ResetSoundEffects 0x507C30 (2), Shutdown 0x507CB0 (2),
  PreloadCutsceneTrack 0x507E30, PreloadBeatTrack 0x507F40.
- Callees: CAEAudioHardware (this = AEAudioHardware 0xB5F8B8) ::IsSoundBankLoaded 0x4D88C0 (bank, slot), ::LoadSoundBank 0x4D88A0
  (bank, slot), ::IsSoundLoaded 0x4D8EF0 (bank, sfx, slot); CAESoundManager (this = AESoundManager 0xB62CB0) ::RequestNewSound
  0x4EFB10 (CAESound*) -> CAESound*, ::AreSoundsPlayingInBankSlot 0x4EF520 (slot) -> int16, ::AreSoundsOfThisEventPlayingForThisEntity
  0x4EF570 (event, entity) -> int16, ::CancelSoundsOfThisEventPlayingForThisEntity 0x4EFB90 (event, entity); CAESound::Initialise
  0x4EFE50; CAESound::~CAESound 0x4EF660 (local, at exit); CAETwinLoopSoundEntity::Initialise 0x4F28A0, ::PlayTwinLoopSound
  0x4F2CB0, ::StopSoundAndForget 0x4F2960; CAEAudioUtility::ResolveProbability 0x4D9C80, GetRandomNumberInRange<int32> 0x4D9C10.
- Globals (R): CAEAudioEntity::m_pAudioEventVolumes 0xBD00F8 (int8*), CTimer::m_FrameCounter 0xB7CB4C (u32),
  CTimer::m_snTimeInMilliseconds 0xB7CB84, CTimer::m_UserPause 0xB7CB49, m_CodePause 0xB7CB48.
- Fields of this: +0x7C m_bAmplifierWakeUp u8, +0x7D m_nbLoadingTuneSeed int8, +0x80 m_BulletPassCount int16, +0x82 m_BulletPassBank
  int16, +0x84 m_nLastFrameGeneral u32, +0x88 m_nLastFrameMissionComplete u32, +0x8C m_nLastFrameBulletPass u32, +0x90
  m_nLastTimeCarRespray, +0x94 m_nLatestTimerCount, +0x98 m_pAmplifierWakeUp, +0x9C m_objRetune (CAETwinLoopSoundEntity), +0x144
  m_objRetunePaused; twin+0x88 (this+0x124 / this+0x1CC) int16 = "twin loop active" flag (nonzero while playing; gr has no name).
- Locals: one CAESound `snd` at [esp+0x38] (only snd+0x08 m_PhysicalEntity = 0 is written before the switch, 0x4DD58F; destroyed by
  0x4EF660 at the common exit 0x4DEC71). Position constants written at entry (0x4DD4C3-0x4DD584): XM = (-1,0,0), XP = (1,0,0),
  YP = (0,1,0), BLR = (-0.1,-1,0), BLF = (-0.1,1,0), BRR = (0.1,-1,0), BRF = (0.1,1,0) (immediates 0xBF800000, 0x3F800000,
  0xBDCCCCCD, 0x3DCCCCCD).
- Volume (before the range check, 0x4DD59D-0x4DD5C3): vol = (float)(int32)(int8)m_pAudioEventVolumes[event] (fild) + volumeBoost
  (fadd f32), stored f32. (Read happens for any event value, even out of range.)
- Dispatch: `cmp event,0x65; ja exit`; byte table 0x4DED2C -> dword table 0x4DECA0. Events 21-26, 39, 52-100 -> exit (no sound).
- Common sound pattern "S(slot, sfx, pos, spd, flags)": `snd.Initialise(slot, sfx, this, pos, vol, rollOff 1.0f, speedArg, doppler 1.0f,
  frameDelay 0, flags 0, speedVariance 0.0f, playTime 0)` (all args pushed explicitly; flags argument is always 0), then the
  listed field overrides written directly into snd (snd+0x1C m_Speed = `speed` argument where marked "spd=arg"; snd+0x56 m_Flags
  (u16) = flags; snd+0x0C m_Event; snd+0x20 m_SpeedVariance), then `AESoundManager.RequestNewSound(&snd)`. Unless noted
  speedArg = 1.0f. "BANK(b,s)" = `if (!IsSoundBankLoaded(b, s)) exit` (AL test). "GATE(f)" = `if (m_FrameCounter < this->f + 5)
  exit` (unsigned `jb`), then `this->f = m_FrameCounter`.
- Cases (event: actions) - bank 0x3B (59) slot 0 = FRONTEND_GAME, bank 0x3C (60) slot 1 = FRONTEND_MENU:
  | events | case | behaviour |
  |---|---|---|
  | 0 | 0x4DD5DB | BANK(0x3B,0); S(0,0x19,XM,spd=arg,0x151B); S(0,0x1A,XP,spd=arg,0x151B) |
  | 1 SELECT | 0x4DD712 | BANK(0x3C,1); S(1,6,XM,arg,0x151B); S(1,7,XP,arg,0x151B) |
  | 2 BACK | 0x4DD7F0 | BANK(0x3C,1); S(1,0,XM,arg,0x151B); S(1,1,XP,arg,0x151B) |
  | 3 HIGHLIGHT | 0x4DD88E | BANK(0x3C,1); S(1,4,XM,arg,0x111B); S(1,5,XP,arg,0x111B) |
  | 4 ERROR | 0x4DD9EA | BANK(0x3C,1); S(1,2,XM,arg,0x151B); S(1,3,XP,arg,0x151B) |
  | 5 NOISE_TEST | 0x4DD96C | BANK(0x3C,1); S(1,8,YP,arg,0x11B) |
  | 6, 18, 19, 20, 40 | 0x4DDA89 | BANK(0x3B,0); GATE(+0x84); S(0,0x1B,XM,arg,0x111B); S(0,0x1C,XP,arg,0x111B) |
  | 7, 8, 9, 10 | 0x4DDB40 | BANK(0x3B,0); GATE(+0x84); S(0,0x10,XM,arg,0x111B); S(0,0x11,XP,arg,0x111B) |
  | 11, 32 | 0x4DD676 | BANK(0x3B,0); S(0,0xE,XM,arg,0x151B); S(0,0xF,XP,arg,0x151B) |
  | 12, 13, 43 | 0x4DE6A6 | BANK(0x3B,0); GATE(+0x88); S(0,0x12,XM,arg,0x151B); S(0,0x13,XP,arg,0x151B) |
  | 14, 15, 17 | 0x4DDD27 | BANK(0x3B,0); GATE(+0x84); S(0,0x1B,XM, speedArg 0.841f (imm 0x3F574BC7), no override, 0x111B); same with 0x1C, XP |
  | 16 CAR_RESPRAY | 0x4DDBF7 | BANK(0x8F,5); c = ResolveProbability(0.5f); (a, b) = c ? (1.1892101f, 1.0f) : (1.0f, 1.1892101f) (imm 0x3F983809); S(5,0x1C,XM, speedArg a, no override, flags 0x110F, m_Event = 0x10); S(5,0x1C,XP, speedArg b, 0x110F, m_Event = 0x10); this+0x90 = m_snTimeInMilliseconds |
  | 27..30 BULLET_PASS | 0x4DDE81 / 0x4DDF69 / 0x4DE010 / 0x4DE0BC | if !IsSoundBankLoaded(this+0x82, 0x2A) exit; if m_FrameCounter < this+0x8C + 5 exit; if count (+0x80, signed) > 10: { if AreSoundsPlayingInBankSlot(0x2A) != 0 exit; bank = ++(+0x82); if bank > 0x1F (signed) bank = 0x1C; LoadSoundBank(bank, 0x2A); count = 0; exit } (shared 0x4DDEBF / 0x4DE0FA, 0x4DE13A). Else this+0x8C = m_FrameCounter; sfx = GetRandomNumberInRange(0, 2); S(0x2A, sfx, P, no override) with P = BLR (27), BLF (28), BRR (29), BRF (30); then m_Event = event (0x1B..0x1E), m_SpeedVariance = 0.03125f (imm 0x3D000000), m_Flags = 7; RequestNewSound; count++ (0x4DE1CC) |
  | 31 WAKEUP_AMPLIFIER | 0x4DE1D8 | if this+0x7C != 0 exit; BANK(0x3B,0); S(0,8,YP,arg,0x11F); p = RequestNewSound result; this+0x98 = p; this+0x7C = (p != 0) |
  | 33 TIMER_COUNT | 0x4DE26D | if AreSoundsOfThisEventPlayingForThisEntity(0x21, this) != 0: this+0x94 = now ms; exit. BANK(0x3B,0); S(0,4,XM,arg,0x110F, m_Event 0x21); S(0,5,XP,arg,0x110F, m_Event 0x21); this+0x94 = now ms |
  | 34 RADIO_RETUNE_START | 0x4DE381 | BANK(0x3B,0); twin = (m_UserPause or m_CodePause) ? this+0x144 : this+0x9C; if twin+0x88 != 0 exit; twin.Initialise(0, 2, 1, this, 200, 650, -1, -1); twin.PlayTwinLoopSound(YP, vol, 1.0f, 1.0f, 1.0f, flags = 0x15B paused / 0x14B normal) |
  | 35 RADIO_RETUNE_STOP | 0x4DE480 | if paused (either flag): behave as 36. Else if this+0x124 != 0: m_objRetune.StopSoundAndForget() |
  | 36 RETUNE_STOP_PAUSED | 0x4DE4AF | if this+0x1CC != 0: m_objRetunePaused.StopSoundAndForget() |
  | 37 RADIO_CLICK_ON | 0x4DE4CC | BANK(0x3B,0); S(0,0x17,YP, speed 1.0f, no override, 0x11B) |
  | 38 RADIO_CLICK_OFF | 0x4DE53D | BANK(0x3B,0); S(0,0x17,YP, speedArg 0.8909f (imm 0x3F641206), no override, 0x11B) |
  | 41 FIRE_FAIL_SNIPER | 0x4DE5AE | BANK(0x3B,0); S(0,0xA,YP,arg,3) |
  | 42 FIRE_FAIL_ROCKET | 0x4DE62A | BANK(0x3B,0); S(0,0xB,YP,arg,3) |
  | 44 RACE_321 | 0x4DE79D | BANK(0x3B,0); S(0,6,XM,arg,0x151B); S(0,7,XP,arg,0x151B) |
  | 45 RACE_GO | 0x4DE878 | BANK(0x3B,0); S(0,0xC,XM,arg,0x151B); S(0,0xD,XP,arg,0x151B) |
  | 46 BUY_CAR_MOD | 0x4DDE08 | BANK(0x3B,0); S(0,9,YP, no override, 0x11B, m_SpeedVariance = 0.0588f imm 0x3D70D845) |
  | 47 SCANNER_NOISE_START | 0x4DE9ED | BANK(0x3B,0); if AreSoundsOfThisEvent...(0x2F, this) exit; S(0,3,YP,arg,0x103, m_Event 0x2F) |
  | 48 SCANNER_NOISE_STOP | 0x4DEA87 | CancelSoundsOfThisEventPlayingForThisEntity(0x2F, this) |
  | 49 SCANNER_CLICK | 0x4DEA8F | BANK(0x3B,0); if AreSoundsOfThisEvent...(0x31, this) exit; S(0,0x18,YP,arg,0x103, m_Event 0x31) |
  | 50 LOADING_TUNE_START | 0x4DEB29 | k = (int8)this+0x7D; if !IsSoundLoaded(0x52, 2k, 2) exit; if !IsSoundLoaded(0x52, 2k+1, 5) exit; if AreSoundsOfThisEvent...(0x32, this) exit; S(2, 2k, XM, arg, 0x111B, m_Event 0x32); S(5, 2k+1, XP, arg, 0x111B, m_Event 0x32) |
  | 51 LOADING_TUNE_STOP | 0x4DEC64 | CancelSoundsOfThisEventPlayingForThisEntity(0x32, this) |
  | 101 MISSILE_LOCK | 0x4DE953 | BANK(0x3B,0); if AreSoundsOfThisEvent...(0x65, this) exit; S(0,0xD,YP,arg,0x101, m_Event 0x65) |
  Within each S the override store order is: m_Speed (if any), m_Flags, then m_Event / m_SpeedVariance as listed (the bullet-pass
  path stores m_Event, m_SpeedVariance, m_Flags). The second S of a pair reuses the same snd object (Initialise rewrites it).
- Flag values (eSoundEnvironment): 0x151B = FORCED_FRONT|ROLLED_OFF|DUCKABLE|PAUSABLE|PLAY_PHYSICALLY|CANCELLABLE|FRONT_END;
  0x111B = same without ROLLED_OFF; 0x110F = FORCED_FRONT|DUCKABLE|PLAY_PHYSICALLY|REQUEST_UPDATES|CANCELLABLE|FRONT_END; 0x11B / 0x11F
  (+REQUEST_UPDATES); 0x103; 0x101; 7; 3; twin 0x14B / 0x15B (+PAUSABLE).
- gta-reversed: plugin::CallMethod stub (AEFrontendAudioEntity.cpp:44) followed by an "untested" body. Differences from the bytes:
  events 14/15/17 gr passes Speed 1.0 / Doppler 0.841 but the original passes 0.841 as the SPEED argument (doppler 1.0);
  event 38 gr Doppler 0.8909 but the original uses 0.8909 as speed; gr passes the flags through Initialise while the original
  passes 0 and stores m_Flags afterwards; gr's PlaySound helper ignores the "no speed override" detail for some cases. Port from the
  table above.

## 0x4DEDA0 CAEFrontendAudioEntity::UpdateParameters  (catalogue: specified/REA, port level 2)
- Convention: virtual (CAEAudioEntity vtable 0x862E54 slot 1), `__thiscall void (this = ECX, CAESound* sound [esp+4],
  int16 curPlayPos [esp+8])`, `ret 8`. `sub esp,0xc` locals. Executed under SA:MP.
- Callers: virtual only (CAESound callbacks). Callees: CAESound::SetPosition 0x4EF880 (thiscall, CVector passed BY VALUE in 12 stack
  bytes), CAESound::StopSoundAndForget 0x4EF850.
- Globals (R): CTimer::m_snTimeInMilliseconds 0xB7CB84 (u32). Constants: [0x85F104]=0.0028571428f (1/350), [0x858624]=1.0f,
  immediates -0.1f (0xBDCCCCCD), +0.1f (0x3DCCCCCD).
- Fields: sound+0x0C m_Event (int32); this+0x7C m_bAmplifierWakeUp (u8), +0x90 m_nLastTimeCarRespray (u32), +0x94 m_nLatestTimerCount
  (u32), +0x98 m_pAmplifierWakeUp (CAESound*).
- Behaviour:
  1. If sound == NULL return (0x4DEDAA).
  2. Two-level switch on e = m_Event - 0x10: unsigned e > 0x11 -> step 3; byte table 0x4DEF4C -> dword table 0x4DEF30:
     - 0x10 AE_FRONTEND_CAR_RESPRAY (0x4DEED3): if curPlayPos > 0 (signed 16-bit) and now > m_nLastTimeCarRespray + 1900 (0x76C;
       unsigned `jbe` skip) -> sound->StopSoundAndForget().
     - 0x21 AE_FRONTEND_TIMER_COUNT (0x4DEEED): if curPlayPos > 0 and now > m_nLatestTimerCount + 100 (unsigned) -> StopSoundAndForget().
     - 0x1B..0x1E BULLET_PASS_{LEFT_REAR, LEFT_FRONT, RIGHT_REAR, RIGHT_FRONT}: only if 0 <= curPlayPos <= 350 (0x15E, signed 16-bit):
       p = (float)(int32)curPlayPos (fild); t = p * [0x85F104]; t = t + t (`fadd st0,st0`); then
       REAR (0x1B, 0x1D): z = t - 1.0f (`fsub`, 0x4DEE05 / 0x4DEE68); FRONT (0x1C, 0x1E): z = 1.0f - t (`fsubr`, 0x4DEE9F).
       x = -0.1f for LEFT (0x1B, 0x1C), +0.1f for RIGHT. z rounded to f32 by fstp (0x4DEEA9).
       sound->SetPosition(CVector(x, 0.0f, z)) (0x4DEEAD-0x4DEECC: [edx] = x, [edx+4] = 0, [edx+8] = z).
     - every other event in range: nothing.
  3. If sound == m_pAmplifierWakeUp and curPlayPos == -1: m_pAmplifierWakeUp = NULL; m_bAmplifierWakeUp = 0 (0x4DEF0B-0x4DEF23).
- gta-reversed: plugin::CallMethod stub (AEFrontendAudioEntity.cpp:767) with an "untested" body that differs from the bytes:
  (a) it computes `curPlayPos / 350.f + curPlayPos / 350.f` (a division; the original multiplies by the f32 constant 1/350 then
  doubles - not bit-identical); (b) it calls `SetPosition({x, y, 0})` - the original passes the computed value as Z with Y = 0.
  Port from the bytes.
