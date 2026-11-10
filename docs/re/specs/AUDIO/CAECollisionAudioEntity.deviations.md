# CAECollisionAudioEntity (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Entities/AECollisionAudioEntity.cpp` (pin 20d8d2a). CAESound layout used
below (gr AESound.h, size 0x74): +0xC m_Event, +0x10 m_ClientVariable, +0x14 m_Volume, +0x1C m_Speed, +0x56 u16 m_Flags.
AESoundManager = 0xB62CB0, RequestNewSound 0x4EFB10, CAESound::Initialise 0x4EFE50, m_pAudioEventVolumes = *(i8**)0xBD00F8.

## 0x4DA070 CAECollisionAudioEntity::ReportGlassCollisionEvent  (gr_deviations = 1)
Original: `__thiscall void (this; i32 event [esp+4], const CVector* pos [+8], u32 time [+0xC])`, `ret 0xC`.
Row AECollisionAudioEntity.cpp:375 (`default: NOTSA_UNREACHABLE()`).
- Original bytes: `event - 0x7B` > 5 (unsigned, 0x4DA07B) jumps straight to the epilogue (0x4DA16D): unknown events are
  ignored silently. Cases (jump table 0x4DA178): 0x7B -> sound 51; 0x7C -> 68; 0x7D -> 56 speed 0.75f; 0x7E -> 56;
  0x7F -> GetRandomNumberInRange(15, 18); 0x80 -> GetRandomNumberInRange(15, 18) speed 0.56f (0x3F0F5C29); default speed 1.0f.
  Then `this->+8 (m_tempSound).Initialise(2, id, this, *pos, (f32)(i8)eventVolumes[event], 1.5f, speed, 1.0f, 0, 0, 0, 0)`;
  if `time != 0` (unsigned `jbe`, 0x4DA133): sound `+0x1C = 0` (speed), `+0x10 = (f32)(u32)(time + CTimer::m_snTimeInMilliseconds)`
  (fild + 4294967296.0f [0x858C54] fix-up when negative), `+0xC = event`, `+0x56 = 4`. Then RequestNewSound(&m_tempSound).
- Verdict: CONFIRMED (gr aborts in debug builds). Class: keep-original (silent ignore). The rest matches gr.

## 0x4DA540 CAECollisionAudioEntity::UpdateLoopingCollisionSound  (gr_deviations = 2)
Original: `__thiscall void (this; CAESound* s [esp+4], CEntity* a, CEntity* b, u8 surfA, u8 surfB, f32 impulse, const CVector*
pos, bool forceLooping)`, `ret 0x20`.
Rows AECollisionAudioEntity.cpp:291 (speed `step_to(..., 0.1f)`) and :292 (volume `step_to(..., 1.0f)`).
- Original bytes: the VOLUME is stepped first (0x4DA74F-0x4DA7A4: target `vol` = `20 * log10(min(4/3 * x, 1)) + (i8)
  eventVolumes[0x74]`, `fldlg2/fyl2x` at 0x4DA70B-0x4DA728), then the SPEED (0x4DA7A6-0x4DA7FB, target `max(x, 0.75f) * 0.8f`,
  step 0.1f [0x858B1C]), then `SetPosition(*pos)` 0x4EF880. Step rule (both): if `target > cur`: `cur = min(cur + step, target)`;
  if `target < cur`: `cur = max(cur - step, target)`; equal or unordered (NaN): no store (`test ah,5; jp`, 0x4DA786/0x4DA7DD).
- Verdict: REFUTED (same values; gr steps speed before volume, independent fields; differs only for NaN). Class: harmless.

## 0x4DAA50 CAECollisionAudioEntity::ChooseCollisionSoundID  (gr_deviations = 3)
Original: `__thiscall i32 (this; u8 surface [esp+4])`, `ret 4`. gCollisionLookup rows at 0x8AD900 (stride 0x10): +8 MinSoundID,
+0xC MaxSoundID.
Rows :697 (NOTSA retry cap 100), :703, :704 (NOTSA fallback return Min).
- Original bytes: if `Min == Max` return Min (0x4DAA6B). Else loop forever: `id = GetRandomNumberInRange<int32>(Min, Max)`
  0x4D9C10 until `(u16)id != (u16)this->m_CollisionSoundIDHistory[surface]` (word at `this + 0x7C + 2*surface`, 16-bit compare
  `cmp ax, word`, 0x4DAA89). Return the full EAX.
- Verdict: CONFIRMED (gr gives up after 100 tries and returns Min; the original never gives up). Class: keep-original
  (unbounded loop; also keeps the RNG call count identical).

## 0x4DB450 CAECollisionAudioEntity::PlayLoopingCollisionSound  (gr_deviations = 6)
Original: `__thiscall void (this; CEntity* a [esp+4], CEntity* b, u8 surfA [+0xC], u8 surfB [+0x10], f32 impulse, const CVector*
pos [+0x18], bool forceLooping [+0x1C])`, `ret 0x1C`; local CAESound with EH frame.
Rows :239/:240/:241 (FIX #1406 comments), :245 (rbf bugfix switch):
- Original bytes: the sound choice calls `g_surfaceInfos` (0xB79538) IsAudioGrass 0x55EA50 (surfA), IsAudioGrass (surfB),
  IsAudioWater 0x55EAD0 (A), (B), IsAudioMetal 0x55EAF0 (A), (B) (0x4DB693-0x4DB6F2) with the RAW surface bytes, including
  the special audio values 0x3E/0x3F (AE_SURFACE_TYPE_*), i.e. out-of-range surface indices are passed through (OOB read
  of the surface-info table). Result: grass -> sound 0, water -> 3, metal -> 2, else 1.
- Verdict: CONFIRMED (gr changes this only when its bugfix switch is on). Class: keep-original (the port must reproduce the
  table read for 0x3E/0x3F; it reads whatever lies past the 0x3E? entries of g_surfaceInfos).
Row :244 ("If both surfaces are valid we preserve the original logic"): REFUTED - for valid surfaces gr's order matches the bytes.
Row :257 (assert): debug-only. REFUTED, harmless.
Other details checked (match gr): Initialise(2, id, this, *pos, vol, 2.0f, speed, 1.0f, 0, 4, 0, 0) (0x4DB713-0x4DB74F);
on success AddCollisionSoundToList(a, b, surfA, surfB, sound, 2) 0x4DAAC0; local CAESound dtor 0x4EF660.

## 0x4DBA10 CAECollisionAudioEntity::ReportCollision  (gr_deviations = 1)
Original: `__thiscall void (...)`, `ret 0x28`.
Row :629 (`default: NOTSA_UNREACHABLE("Invalid soundStatus")`).
- Original bytes: two blocks on `forceLooping` (0x4DBC83). forceLooping set: `st = GetCollisionSoundStatus(...)` 0x4DA830;
  if `st == 2` (LOOPING): entry `e = this + 0x20C + id*0x14`: `e.+8 (0x214) = m_snTimeInMilliseconds + 100`; if `e.+4 (0x210)`
  sound != 0: UpdateLoopingCollisionSound(sound, e.EntityA, e.EntityB, ..., forceLooping); else (ANY other value) ->
  PlayLoopingCollisionSound(..., forceLooping) (0x4DBD0D). forceLooping clear: `(i8)st - 1 == 0` -> PlayLooping(..., 0);
  `== 2` -> update (as above, last arg 0); ANY other value (0 or invalid) -> PlayOneShotCollisionSound 0x4DB150.
- Verdict: CONFIRMED (gr aborts; the original treats invalid statuses like INACTIVE). Class: keep-original.
