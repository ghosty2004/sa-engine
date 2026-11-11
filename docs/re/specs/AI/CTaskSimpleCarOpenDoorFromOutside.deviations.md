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

## 0x64D710 CTaskSimpleCarOpenDoorFromOutside::ProcessPed  (gr_deviations = 3)
Bytes summary: `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`. Fields: +8 m_finished, +0xC m_anim, +0x10 m_veh, +0x14 m_door,
+0x18 m_disallowPlayerDriverToExitCar, +0x19 m_hasSetPlayerCanExitCarFlag, +0x20 m_doorOpenAngleRatio. Return 1 if finished or veh == 0;
DontOpenDoorWhenGettingIn flag test = bit 6 of the dword at 0xC1CDC8 + 0x94*group (0x64D737-0x64D744).

Row TaskSimpleCarOpenDoorFromOutside.cpp:136 (`NOTE/TODO: Inlined? Double check if m_veh`):
- Original bytes: after StartAnim 0x64AD90 the original re-checks `m_veh != 0` (0x64D7A4), then `m_veh->m_pDriver (+0x460) != 0`, then
  `driver->IsPlayer()` 0x5DF8F0, then `+0x18 != 0`, then `ped != 0`, then `ped->+0x598 == 6` (PED_TYPE_COP) - the same conditions and
  order as gr.
- Verdict: REFUTED. - Class: harmless.

Row TaskSimpleCarOpenDoorFromOutside.cpp:138 (FIX_BUGS block; original = #else):
- Original bytes: 0x64D7D1-0x64D7E4: `m_veh->m_pDriver->+0x474 &= ~0x4000000` (bCanExitCar = false on the vehicle's driver), `+0x19 = 1`.
- Verdict: CONFIRMED (FIX_BUGS uses FindPlayerPed(); the #else branch matches the bytes).
- Class: keep-original.

Row TaskSimpleCarOpenDoorFromOutside.cpp:166 (`default: NOTSA_UNREACHABLE()` in the door -> eDoors switch):
- Original bytes: jump table 0x64D8CC: door 8 -> 3, 9 -> 5, 10 -> 2, 11 -> 4; any other door (0x64D85B) uses `[esp+0x18]`, the slot that
  holds the f32 bit pattern of `m_anim->m_CurrentTime (+0x20)`, as the door id passed to the OpenDoor virtual (+0x6C).
- Verdict: CONFIRMED (gr aborts; the original passes garbage). Only reachable for invalid doors.
- Class: keep-original.

Extra (NOT a catalogued row, real gr deviation): the branch condition is inverted in gr. Original 0x64D80D-0x64D82B:
`t = m_anim->+0x20; T = timing dword at 0xC1CDE0 + 0x94*group` (start/open-start timing); `if (t >= T or unordered (test ah,5; jp))
or veh->+0x590 (m_nVehicleType) != 0` -> ProcessOpenDoor path; only an AUTOMOBILE (type 0) with `t < T` takes the visual OpenDoor path:
`ratio = (1.0f [0x858624] - t / T) * m_doorOpenAngleRatio` (x87: fld t, fdiv T, fsubr 1.0, fmul +0x20) and vfunc +0x6C(ped, door,
doorId, ratio, 0). gr writes `if (m_veh->IsAutomobile() || time >= timing) ProcessOpenDoor else OpenDoor` (cpp:149), i.e. automobiles
never get the visual OpenDoor path in gr and non-automobiles do. Also in the ProcessOpenDoor path the original computes the group with
`GetGroup(m_anim->+0x2C (i16 anim id))` 0x6E3B00 (0x64D889), not via ComputeAnimID, then vfunc +0x70(ped, door, group, (i16)anim id,
t). Verdict CONFIRMED, class keep-original.
