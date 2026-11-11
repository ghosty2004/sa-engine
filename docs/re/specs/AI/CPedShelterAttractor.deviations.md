# CPedShelterAttractor: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 helper A (groups J1/J6).

## 0x5E9690 CPedShelterAttractor::ComputeAttractHeading  (gr_deviations = 1)
Original bytes (`__thiscall void (this unused, int32 bQueue [esp+4] unused, float* heading [esp+8])`, `ret 8`): `*heading = ((float)_rand() * 3.05185094e-05f [0x858C7C])
* 6.28318548f [0x858CBC]` (0x5E9691-0x5E96AE; no min term).
Row PedShelterAttractor.cpp:33 (`GetRandomNumberInRange(0.0f, TWO_PI)`): Verdict CONFIRMED (helper formula differs; the original uses rand()/32767 * 2pi). Class: keep-original.

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x5EF420 CPedShelterAttractor::GetDisplacement  (gr_deviations = 2)
Original bytes (`__cdecl const CVector* (int32 pedId)`; ms_displacements std::vector at 0xC09870 begin / 0xC09874 end, element 12 bytes, size via magic 0x2AAAAAAB >> 1):
if the vector is empty, five displacements are generated (rejection loop as in gr). Per candidate (0x5EF460-0x5EF4C8):
`rads = ((float)_rand() * 3.05185094e-05f [0x858C7C]) * 6.28318548f [0x858CBC]` stored f32; `radius = (float)_rand() * 3.05185094e-05f [0x858C7C]` then `fadd st0,st0`
(x2, kept on the x87 stack, not stored); `x = fcos(rads) * radius`, `y = fsin(rads) * radius` (x87 `fcos` / `fsin` on the f32 rads, each product stored f32).
Row PedShelterAttractor.cpp:13 (`rads = GetRandomNumberInRange(0.f, TWO_PI)`):
- Verdict: CONFIRMED (inlined `rand()/32767 * 2pi` vs gr's helper formula; also gr's std::cos/std::sin vs x87 fcos/fsin may round differently). Class: keep-original.
Row PedShelterAttractor.cpp:14 (`radius = GetRandomNumberInRange(0.f, 2.f)`):
- Verdict: CONFIRMED (original `t + t` with t = rand()/32767). Class: keep-original.
- Note: the two `_rand` calls happen in the order rads, radius (0x5EF462, 0x5EF47F), same as gr.
