# CTaskSimpleCarGetOut: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x647330 CTaskSimpleCarGetOut::ComputeAnimID_ToHook  (gr_deviations = 2)
Original bytes (`__thiscall void (this, int32* outGroup [esp+4], int32* outAnimId [esp+8])`, `ret 8`; this: +0x10 m_isUpsideDown (u8),
+0x14 m_veh, +0x18 m_door):
  1. If `m_veh->m_nVehicleType (+0x590) == 0` (automobile): `m_isUpsideDown = CUpsideDownCarCheck::IsCarUpsideDown(&localTmp, m_veh) != 0`
     (0x463830, on an uninitialised stack object; 0x647336-0x647352). NOTE: this ASSIGNS, i.e. clears a previously set flag when the
     car is not upside down; gr only sets it to true (TaskSimpleCarGetOut.cpp:64-66). Additional CONFIRMED difference (no row).
  2. If m_isUpsideDown: `rz = m_veh->m_matrix->right.z (+8)` (no null check). Doors 10/11: clear the flag if `rz < -0.5f [0x858F40]`
     (`test ah,5; jp` keeps it on >= and on NaN; 0x647385-0x647399). Other doors: clear if `rz > 0.5f [0x858B8C]` (`test ah,0x41; jne`
     keeps it on <=, == and NaN; 0x64736D-0x647381). (gr's `>= -0.5f` / `<= 0.5f` yield false for NaN: NaN-only difference.)
  3. If still upside down: jump table 0x647454 (`door-8`, unsigned > 3 -> default): doors 8/9 -> `*outAnimId = 0x6D` (109
     CRAWLOUTRHS_1), `*outGroup = 0` (0x6473D3); doors 10/11 -> `*outAnimId = 0x6C` (108 CRAWLOUTRHS_0), `*outGroup = 0` (0x6473B8);
     any other door -> only `*outGroup = 0` (0x6473DD), *outAnimId untouched. Return.
  4. Else jump table 0x647464: 8 -> 0x176 (374 GETOUT_RHS_0), 9 -> 0x178 (376 RHS_1), 10 -> 0x175 (373 LHS_0), 11 -> 0x177 (375 LHS_1);
     other doors write nothing. Then `*outGroup = m_vehicleAnimGroups[m_veh->m_pHandling (+0x384)->animGroup (byte +0xDE)].GetGroup(*outAnimId)`
     (0x6E3B00, stride 0x94 from 0xC1CDC0; 0x647422-0x64744A).
Row TaskSimpleCarGetOut.cpp:94 (upside-down `default: NOTSA_UNREACHABLE()`):
- Original: other doors give `*outGroup = 0` and leave *outAnimId as the caller had it (0x6473AF -> 0x6473DD).
- Verdict: CONFIRMED. Class: keep-original.
Row TaskSimpleCarGetOut.cpp:111 (normal `default: NOTSA_UNREACHABLE()`):
- Original: *outAnimId untouched, *outGroup = GetGroup(caller's *outAnimId) (0x6473FB -> 0x647422).
- Verdict: CONFIRMED. Class: keep-original.
Door -> anim mappings of the valid cases match gr (checked against both jump tables and AnimationEnums.h).
