# CCoverPoint: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 helper A (groups J1/J6).

## 0x698E70 CCoverPoint::CanAccommodateAnotherPed  (gr_deviations = 3)
Original bytes (`__thiscall bool ()`, `ret`): t = (int8)this+0; t == 0 -> `xor al,al` (AL only, 0x698E9F); t == 2 (VEHICLE) -> 1 if `+0x14 == 0` or `+0x18 != 0`,
else 0 (full EAX, 0x698E88-0x698E99); other types -> `sete al` of `+0x14 == 0` (EAX zeroed first, 0x698E7D-0x698E84).
Row CoverPoint.cpp:22 (bug-note: name typo `CanAccomodateAnotherPed` fixed): Verdict REFUTED (naming only). Class: harmless.
Row CoverPoint.cpp:28 (`notsa::IsFixBugs() ? all_of(peds, IsNull)`): Verdict CONFIRMED - the original returns `!p0 || p1` for vehicles (a vehicle point with
both slots used still "accommodates"; original bug). Class: keep-original.
Row CoverPoint.cpp:29 (notsa helper inside the FixBugs branch): Verdict REFUTED (only reached with FixBugs). Class: harmless.
