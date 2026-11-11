# CTaskSimpleCarGetIn: gta-reversed deviation notes (AI) - RE-SPEC-REST group B

Source: `re/gta-reversed/source/game_sa`. Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question.

## 0x64B2D0 CTaskSimpleCarGetIn::StartAnim  (gr_deviations = 1)
Row TaskSimpleCarGetIn.cpp:63 (`default: NOTSA_UNREACHABLE("Invalid door: {}", m_door)`):
- Original bytes: `idx = m_door (+0x1C) - 8`; `idx > 10` unsigned (0x64B2DA) -> default; else byte table 0x64B3B4 = {0,1,2,3,5,5,5,5,5,5,4}
  into jump table 0x64B39C: door 8 -> 0x168, 9 -> 0x16A, 10 -> 0x167, 11 -> 0x169, 18 -> 0x16B (same anim ids as gr: RHS_0, RHS_1, LHS_0,
  LHS_1, BIKE_FRONT); doors 12-17 and every other value -> default 0x64B311: `animId = [esp+0x10]` = the stack slot of the `ped`
  argument, i.e. the CPed* pointer value is used as the AnimationId. Execution continues: GetGroup(animId) 0x6E3B00 on
  m_vehicleAnimGroups[veh->handling->+0xDE] (0xC1CDC0, stride 0x94), the CONVCARANIMS (0x6B) driver-door fix-up, BlendAnimation
  0x4D4610(ped clump, group, animId, 4.0f) and SetFinishCallback(FinishAnimCarGetInCB 0x6467F0, this).
- Verdict: CONFIRMED (gr aborts; the original proceeds with a garbage anim id). Only reachable with an invalid door.
- Class: keep-original (harmless for valid doors).
