# CTaskSimpleCarCloseDoorFromOutside: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x646600 CTaskSimpleCarCloseDoorFromOutside::ComputeAnimID  (gr_deviations = 1 yes/maybe; 1 notsa-decl no)
Original bytes (`__thiscall void (this, int32* outGroup [esp+4], int32* outAnimId [esp+8])`, `ret 8`): switch on `m_door (+0x14) - 8`
(unsigned `ja` for > 3, jump table 0x646664): jump table entries (0x646626, 0x64662E, 0x646616, 0x64661E) give 8 -> `*outAnimId = 0x17D` (381 CAR_CLOSE_RHS_0), 9 -> 0x17F (383 RHS_1),
10 -> 0x17C (380 LHS_0), 11 -> 0x17E (382 LHS_1) (case blocks 0x646616-0x64662E);
then (also for every other door value) `*outGroup = CVehicleAnimGroupData::m_vehicleAnimGroups[m_veh (+0x10)->m_pHandling (+0x384)
->m_nAnimGroup (byte +0xDE)].GetGroup(*outAnimId)` (0x6E3B00, stride 0x94 from 0xC1CDC0; 0x646634-0x64665C), reading *outAnimId back.
Row TaskSimpleCarCloseDoorFromOutside.cpp:64 (`default: NOTSA_UNREACHABLE();`):
- Original: for a door outside 8..11 nothing is written to *outAnimId; the caller's previous value is used for GetGroup and stays in
  *outAnimId (0x64660D -> 0x646634).
- Verdict: CONFIRMED (gr aborts/UB where the original continues with the caller's value). Class: keep-original.
Row TaskSimpleCarCloseDoorFromOutside.h:28 (notsa declaration): REFUTED (declaration only; signature matches the bytes). Class: harmless.
Note: the door -> anim mapping matches gr's cases (checked by value against AnimationEnums.h:543-546 and the jump table).
