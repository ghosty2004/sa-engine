# CTaskSimpleCarCloseDoorFromInside: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 groups J5/J7 (lead).

## 0x646410 CTaskSimpleCarCloseDoorFromInside::ComputeAnimID  (gr_deviations = 1)
Original bytes: `__thiscall void (this, AssocGroupId* outGroup [esp+4], AnimationId* outAnim [esp+8])`, `ret 8`. idx = m_nTargetDoor (+0x14) - 8;
if idx > 3 unsigned (0x64641A) nothing is stored into *outAnim; else jump table 0x646474: door 8 -> 0x170, 9 -> 0x172, 10 -> 0x16F,
11 -> 0x171 (0x646426-0x64643E). Then always `*outGroup = m_vehicleAnimGroups[veh(+0x10)->handling(+0x384)->+0xDE].GetGroup(*outAnim)`
(0x646444-0x64646C; 0xC1CDC0 + 0x94*group, GetGroup 0x6E3B00), reading *outAnim back from memory.
Row TaskSimpleCarCloseDoorFromInside.cpp:71 (`default: NOTSA_UNREACHABLE();`):
- Original bytes: for any other door the caller's *outAnim value is kept and still used for GetGroup (0x646447).
- Verdict: CONFIRMED (gr aborts; the original keeps the stale anim id). Mapping 8/9/10/11 = 368/370/367/369 matches gr's enum names.
- Class: keep-original (only reachable with an invalid door index).
