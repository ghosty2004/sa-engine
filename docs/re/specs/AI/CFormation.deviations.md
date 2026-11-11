# CFormation: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 helper A (groups J1/J6).

## 0x699FA0 CFormation::ReturnDestinationForPed  (gr_deviations = 2)
Original bytes (`__cdecl bool (CPed* ped [esp+4], CVector* out [esp+8])`, `ret`): for i = 0..23 (`cmp eax,0x18; jl`, 0x699FBB): if `peds[i]` (dword table 0xC1A4DC,
m_Peds.m_peds) == ped and `link = m_aFinalPedLinkToDestinations[i]` (0xC1A2C0) >= 0 (signed `jge`): `*out = destinations[link]` (CVector table 0xC1A31C, stride 12,
3 dword copies) and return AL = 1 (0x699FC3-0x699FE8). A matching ped with a negative link continues the loop. No match -> AL = 0 (`xor al,al`, upper EAX bits = 0, 0x699FC0).
Row Formation.cpp:18 (original-comment: the original iterates 24 times, not m_Peds.m_count): Verdict CONFIRMED (0x699FBB bound 0x18; reads past the 8-entry link array for
i >= 8 only when peds[i] == ped). Class: keep-original.
Row Formation.cpp:21 (`notsa::bugfixes::GenericOOB` bound): Verdict CONFIRMED (gr changes the bound only when the bugfix is on; default path = 24 like the original).
Class: keep-original (gr notes the OOB read is unreachable in practice because unused slots are null).

## 0x69B860 CFormation::FindCoverPoints  (gr_deviations = 2)
Original bytes: `__cdecl void (CVector pos (by value), float radius)`. m_Destinations.m_Count [0xC1A318] = 0; 6 dwords at 0xC1A43C (claimed flags) zeroed (`rep stosd`, 0x69B872-
0x69B886). Points are appended only while count < 24 (`cmp eax,0x18; jge`, 0x69B996 / 0x69BAC5).
Row Formation.cpp:305 (vehicle pool via GetAllValid, ascending):
- Original bytes: 0x69B888-0x69B9CE walks the vehicle pool [0xB74494] DOWNWARDS: `i = size; do { i--; if (byteMap[i] & 0x80) continue; obj = objects + i*0xA18 } while (i)`
  (`dec ebx` 0x69B8A7, free test `js` 0x69B8B4).
- Verdict: CONFIRMED - the order of appended cover points differs from gr, and with the 24-point cap the SET of kept points can differ. Class: keep-original.
Row Formation.cpp:329 (object pool via GetAllValid, ascending):
- Original bytes: 0x69B9D4-0x69BB0E walks the object pool [0xB7449C] downwards the same way (element 0x19C, `dec edi` 0x69B9F7, `js` 0x69BA00).
- Verdict: CONFIRMED. Class: keep-original.
