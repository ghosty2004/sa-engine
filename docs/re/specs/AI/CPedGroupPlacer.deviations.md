# CPedGroupPlacer: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x5FD810 CPedGroupPlacer::PlaceGroup  (gr_deviations = 1)
Original bytes (`__thiscall bool (this, int32 pedType [esp+4], uint32 numPeds [esp+8], const CVector* origin [esp+0xC], uint32 allocType
[esp+0x10])`, `ret 0x10`): if allocType > 5 (unsigned) -> default; jump table 0x5FD848 = {0x5FD820, 0x5FD820, 0x5FD829, 0x5FD829,
0x5FD83F, 0x5FD832}: 0/1 (FOLLOW_ANY_MEANS/FOLLOW_LIMITED) -> tail-jump PlaceFormationGroup 0x5FC9B0; 2/3 (STAND_STILL/CHAT) ->
tail-jump PlaceChatGroup 0x5FCE80; 5 (RANDOM) -> arg4 slot := 5, tail-jump PlaceRandomGroup 0x5FD330; 4 (SIT_IN_LEADER_CAR) and > 5 ->
`mov al, [esp+0xC]`; `ret 0x10` (0x5FD83F): returns the LOW BYTE OF THE origin POINTER as the bool (garbage, usually non-zero),
upper EAX bits = allocType.
Row PedGroupPlacer.cpp:37 (`default: NOTSA_UNREACHABLE();`):
- Verdict: CONFIRMED - gr aborts for allocType 4 or > 5; the original returns `(uint8)origin` without placing anyone. Class: keep-original.
