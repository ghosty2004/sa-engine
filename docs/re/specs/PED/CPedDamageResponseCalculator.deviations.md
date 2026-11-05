# CPedDamageResponseCalculator: gta-reversed deviation notes (PED)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x4B5AC0 CPedDamageResponseCalculator::ComputeDamageResponse  (gr_deviations = 2)
Original bytes (`__thiscall void(CPed* ped [esp+4], CPedDamageResponse* resp [esp+8], bool speak [esp+0xC])`, `ret 0xC`, SEH frame
for the stack CEventDamage). Order: return if `resp->m_bDamageCalculated (+0xA)`; zero +0/+4 (f32), +8/+9 (u8), set +0xA = +0xB = 1;
AccountForPedDamageStats 0x4AD430, AccountForPedArmour 0x4AD550, ComputeWillKillPed 0x4B3210 (speak);
if `m_pDamager (+0x0)` and (`== FindPlayerPed(-1)` or `== FindPlayerVehicle(-1, 0)`) and `!= ped` and `resp+0 + resp+4 > 3.0f`
(0x858B3C; NaN/<= skip): `CWorld::Players[PlayerInFocus].m_nHavocCaused (+0x140)++`; if `ped == FindPlayerPed(-1)`:
`FindPlayerPed(-1)->AnnoyPlayerPed(0)`; if `ped->IsPlayer()` and damager is a ped (type bits `[+0x36] & 7 == 3`): stack
`CEventDamage(damager, 0, m_weaponType (+0xC), m_bodyPart (+0x8), 0, 0, (ped->flags1 >> 8) & 0xFFFFFF01)` (0x4AD830; the last
push keeps the upper bits, the callee uses the low byte = bInVehicle) -> `MakeGroupRespondToPlayerTakingDamage` 0x60A110 -> event
dtor 0x4AD960. If `resp+0 + resp+4 > 0.0f` (else return): bleeding (below); if `ped == FindPlayerPed(-1)`:
`Players[PlayerInFocus].m_nLastTimeEnergyLost (+0xE4) = time`; `ped->m_nLastWeaponDamage (+0x760, u8) = (u8)m_weaponType`;
if damager: `ped->m_pLastEntityDamage (+0x764) = damager` + `RegisterReference` (0x571B70, the old one is NOT cleaned up),
`ped->+0x768 = time`; if damager is a ped and `IsPlayer()`: `ms_groups[FindPlayerPed(-1)->m_pPlayerData->m_nPlayerGroup (+0x38)]
.m_groupMembership.IsMember(ped)` (0x5F6A10) with the result discarded.
Row PedDamageResponseCalculator.cpp:252 (IsBleedingWeapon inlined at 0x4B5C2A):
- Original bytes: `if (!ped->IsPlayer() && (weapon == 4 || weapon == 8 || weapon == 9)) ped->+0x72F = 200` (0x4B5C2A-0x4B5C47).
- Verdict: REFUTED (same logic). Class: harmless.
Row PedDamageResponseCalculator.cpp:315 (original calls `IsMember(ped)` and ignores the result):
- Original bytes: 0x4B5C9E-0x4B5CD6 as described above.
- Verdict: REFUTED (IsMember only reads; omitting it is not observable). Class: harmless.
