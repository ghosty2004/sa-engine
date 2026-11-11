# CTaskSimpleCarOpenDoorFromOutside: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 groups J5/J7 (lead).

## 0x645FA0 CTaskSimpleCarOpenDoorFromOutside::ComputeAnimID_ToHook  (gr_deviations = 1)
Original bytes: `__thiscall void (this, AssocGroupId* outGroup [esp+4], AnimationId* outAnim [esp+8])`, `ret 8`. idx = m_nTargetDoor (+0x14) - 8;
idx > 3 unsigned (0x645FAA) -> no store; else jump table 0x646004: door 8 -> 0x164, 9 -> 0x166, 10 -> 0x163, 11 -> 0x165
(0x645FB6-0x645FCE). Then `*outGroup = m_vehicleAnimGroups[veh(+0x10)->handling(+0x384)->+0xDE].GetGroup(*outAnim)` (0x645FD4-0x645FFC).
Row TaskSimpleCarOpenDoorFromOutside.cpp:66 (comment "Originally @ 0x645FA0, but we've modified it a little"):
- Original bytes: same door->anim mapping as gr (356/358/355/357 = OPEN_RHS / OPEN_RHS_1 / OPEN_LHS / OPEN_LHS_1); gr returns a pair instead
  of writing through two pointers, and its `default: NOTSA_UNREACHABLE()` replaces the original "keep *outAnim and still call GetGroup".
- Verdict: REFUTED for valid doors (no behavioural difference); the default-path difference is the same as 0x646410 (invalid doors only).
- Class: harmless.
