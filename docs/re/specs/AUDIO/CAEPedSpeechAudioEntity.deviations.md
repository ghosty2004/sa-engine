# CAEPedSpeechAudioEntity (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Entities/AEPedSpeechAudioEntity.cpp/.h` (pin 20d8d2a). `Rnd` =
CAEAudioUtility::GetRandomNumberInRange<int32> 0x4D9C10. +0x92 i16 m_PedAudioType (2 = PED_TYPE_PLAYER, 4 = PED_TYPE_GFD).
s_Conversation i16[8] at 0xB613EC, s_ConversationLength i16 0xB613E8.

## 0x4E3A00 CAEPedSpeechAudioEntity::SetUpConversation  (gr_deviations = 1)
Original: `__cdecl void ()` (static), plain `ret`.
Row :400 (`else NOTSA_UNREACHABLE()` after the `r <= 10` branch).
- Original bytes: clears the 8 i16 entries at 0xB613EC (loop 0x4E3A10). Greetings by ped (+0x598 == 0x16 or 5 -> 0xA5, else
  0xA6) for s_pConversationPed2 (0xB6140C) into [0] and s_pConversationPed1 (0xB61410) into [1]. `r = Rnd(1, 10)`; `r - 1 > 9`
  (impossible) returns (0x4E3A88); 1..4 -> 0x4E3A9C, 5..8 -> 0x4E3B6C, 9..10 -> 0x4E3C0F (byte table 0x4E3C4C).
- Verdict: REFUTED (the default is unreachable in the original too; the original simply returns). Class: harmless.
Found in passing (CONFIRMED, keep-original): the coin flips use `Rnd(0, 1)` (0x4E3AA9, 0x4E3C1C), gr uses
`CGeneral::DoCoinFlip()` (lines 377, 397) - different RNG formula. In the 9..10 case: `s_ConversationLength = 1`,
`[3] = 0xA4` (DISMISS), `[2] = 0xAA + (Rnd(0,1) != 0)` (0xAB STATE_GOOD when 1, 0xAA STATE_BAD when 0; 0x4E3C24-0x4E3C39).

## 0x4E3CD0 CAEPedSpeechAudioEntity::GetVoice  (gr_deviations = 2)
Original: `__cdecl i16 (const char* name [esp+4], i16 type [esp+8])`, AX result, plain `ret`.
Row :420 (`notsa::find_value_or`): original does an inline strcmp loop over the per-type tables (first at 0x8AE6A8, stride
0x14, 0x4E3CEF-0x4E3E90) and returns the matching index; not found -> -1. Verdict: REFUTED, harmless.
Row :429 (`default: NOTSA_UNREACHABLE()`): original `(i32)type > 4` (unsigned, incl. 5 = SPC and negatives) returns
`AX = -1` (`or ax,0xFFFF`, 0x4E3E95). Verdict: CONFIRMED (gr aborts for invalid types). Class: keep-original.

## 0x4E4510 CAEPedSpeechAudioEntity::IsGlobalContextImportantForStreaming  (gr_deviations = 1)
Original: `__thiscall bool (this; i16 ctx [esp+4])`, `ret 4`.
Row :686 (NOTSA: context-info lookup instead of the switch).
- Original bytes: if `+0x92 == 4 || +0x92 == 2` return 1. `ctx - 0xD` (sign-extended) > 0x9E -> 0; else table 0x4E4554/0x4E454C:
  returns 1 exactly for ctx in {13, 15, 48..64, 125, 126, 127, 161..171}, 0 otherwise.
- Verdict: CONFIRMED (implementation differs; equal only if gr's LUT flags reproduce this set - the commented-out gr switch
  lists exactly this set). Class: keep-original (port the explicit set).

## 0x4E4600 CAEPedSpeechAudioEntity::IsGlobalContextImportantForInterupting  (gr_deviations = 2)
Original: `__stdcall bool (i16 ctx [esp+4])`, `ret 4` (no this).
Row :633 (NOTSA lookup): original returns 1 exactly for ctx in {13, 15, 125, 126, 127} (table 0x4E4630/0x4E4628, range
`ctx - 0xD <= 0x72`). Verdict: CONFIRMED (same caveat as above; the commented gr switch matches). Class: keep-original.
Row AEPedSpeechAudioEntity.h:300 (typo in the name): not behavioural. REFUTED, harmless.

## 0x4E46B0 CAEPedSpeechAudioEntity::IsGlobalContextImportantForWidescreen  (gr_deviations = 1)
Original: `__thiscall bool (this; i16 ctx)`, `ret 4`. If `+0x92 == 4 || == 2` return 1; return 1 for ctx 13, 15, 116 (0x74
HAVING_SEX) (`sub 0xD; sub 2; sub 0x65` chain, 0x4E46C8-0x4E46D5); else 0.
Row :666 (NOTSA lookup): Verdict: CONFIRMED (same caveat). Class: keep-original.

## 0x4E4760 CAEPedSpeechAudioEntity::GetVoiceForMood  (gr_deviations = 1)
Original: `__stdcall i16 (i16 mood [esp+4])`, `ret 4`.
- Original bytes: `r = Rnd(0, 1)` first; then `mood` (sign-extended) 0..9 via table 0x4E47B4: result = `r + 2*mood`
  (0 -> r, 1 -> r+2, ..., 9 -> r+0x12); any other mood (`> 9` unsigned, incl. negatives) -> `r + 10` (0x4E47AD).
Row :585 (`default: NOTSA_UNREACHABLE()`): CONFIRMED (original returns `r + 10` = the MOOD_CR pair). Class: keep-original.
Found in passing (CONFIRMED gr logic error, keep-original): gr returns `b ? VOICE_PLY_xx : VOICE_PLY_xx2` with `b = (r != 0)`, i.e.
the even voice when r = 1; the original returns `base + r` (the odd "2" voice when r = 1). Same RNG call, inverted pick.
