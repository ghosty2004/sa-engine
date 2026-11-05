# CPlayerPed: gta-reversed deviation notes (PED)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x60EA90 CPlayerPed::ProcessControl  (gr_deviations = 2)
Context (0x60F73F-0x60F858, group "keep up" chatter): `n = ms_groups[pd->m_nPlayerGroup].m_groupMembership.CountMembersExcludingLeader()`;
if n > 0 and `FindDistanceToNearestMember(0) > 20.0f` (0x858BA4; `test ah,0x41; jne` skips <=, NaN) and - **calling
FindDistanceToNearestMember a second time** (0x60F7A1) - `< 100.0f` and `CGame::currArea == 0`: `Say(n == 1 ? 0x9E : 0x9D, 0, 1.0f, 0, 0, 0)`;
then for member index 0..6: `m = GetMember(i)`; if m and `CGeneral::GetRandomNumberInRange(0.0f, 1.0f)` (0x41BD90) `< 0.5f` (`test ah,5; jp`):
`m->Say(0x5C, CGeneral::GetRandomNumberInRange(3000, 4500) /*0x407180, int*/, 1.0f, 0, 0, 0)`.
Row PlayerPed.cpp:1257 (`GetRandomNumberInRange(0.0f, 1.0f) < 0.5f`):
- Verdict: CONFIRMED (original float helper 0x41BD90 `(max-min)*t + min`; gr lerp rounding differs). Class: keep-original (call 0x41BD90's formula).
Row PlayerPed.cpp:1258 (`GetRandomNumberInRange(3000, 4500)`):
- Verdict: CONFIRMED (original int helper 0x407180 = `3000 + _ftol((rand() & 0xFFFF) * (1/32768) * 1500)`; gr's helper differs). Class: keep-original.
- Note: gr calls FindDistanceToNearestMember once; the original calls it twice (no state change expected; port both calls to be exact).
