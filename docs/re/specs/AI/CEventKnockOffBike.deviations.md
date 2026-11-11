# CEventKnockOffBike: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 helper A (groups J1/J6).

## 0x4B0020 CEventKnockOffBike::CalcForcesAndAnims  (gr_deviations = 3)
Original bytes: `__thiscall int32 (this, CPed* ped [esp+4])`, `ret 4`. `massRatio = ped->m_fMass / veh->m_fMass` (+0x8C, stored over the arg slot, 0x4B002B-0x4B004C);
EBP (returned anim id) initialised to 0xF (0x4B0047); numContactWheels BL = bike +0x804 (type 9) / automobile +0x960 (type 0) / 0. Switch on (int8)m_knockOffType (+0x36, `movsx`,
`cmp 0x37; ja` unsigned) via index table 0x4B0688 -> dword table 0x4B0674: 54 -> 0x4B007F, 49 -> 0x4B012A, 0/55 -> 0x4B0236 (return 0xBF), 53 -> 0x4B0266, everything else
(1..48, 50..52, and any value > 0x37 incl. negative) -> 0x4B02C5 (the knock-off direction switch, table 0x4B06E0).
Row EventKnockOffBike.cpp:203 (`default: NOTSA_UNREACHABLE();`, inner direction switch of the default type):
- Original bytes: 0x4B02C5-0x4B02CC `movsx eax, [esi+0x34]; cmp eax,3; ja 0x4B0667` -> returns EBP = 0xF with no move-speed / force / ignored-collision change.
- Verdict: CONFIRMED (gr aborts; the original returns 15). Class: keep-original.
Row EventKnockOffBike.cpp:218 (`ped->m_vecMoveSpeed.z += GetRandomNumberInRange(3.0f, randomNum) / 70.0f`, SKIDBACKFRONT = 49):
- Original bytes: 0x4B01CB calls the float helper 0x41BD90 with min = 3.0f [0x85AB2C] (sub type 0xA = BMX) or 3.0f [0x85AB20], max = 6.0f [0x85AB30] (BMX) or 5.0f [0x85AB24];
  then `fmul 0.0142857144f [0x85B328]` (multiplies by an f32 reciprocal of 70, NOT a division), `fadd ped+0x4C`, `fstp ped+0x4C` (0x4B01D0-0x4B01DC).
- Verdict: CONFIRMED (gr's helper formula differs, cat14 finding 1; and gr divides by 70.0f where the original multiplies by the rounded reciprocal, a second rounding
  difference). Class: keep-original.
Row EventKnockOffBike.cpp:251 (`force.z = GetRandomNumberInRange(3.0f, 5.0f)`, SKIDBACK_FALLR = 54):
- Original bytes: 0x4B00A2 float helper 0x41BD90(3.0f [0x85AB14], 5.0f [0x85AB18]), result is force.z (0x4B00A7); x/y = `(-0.3f [0x85AB10] * impact.{y,x}) * massRatio * m_damageIntensity`
  (0x4B00AB-0x4B00CE), passed by value to `CPhysical::ApplyMoveForce(x,y,z)` 0x4ABBA0.
- Verdict: CONFIRMED. Class: keep-original.
Additional difference found (not a catalogue row): direction > 3 (or negative) in the 49 / 54 cases leaves EBP = 0xF (0x4B00E9 / 0x4B01EF `ja` past the tables 0x4B06C0 /
0x4B06D0), whereas gr initialises `animId = ANIM_ID_KO_SKID_BACK` (0x1A). Verdict: CONFIRMED, keep-original. The 49 case calls `ped->IsPlayer()` 0x5DF8F0 and discards it
(0x4B0225), as gr does.
