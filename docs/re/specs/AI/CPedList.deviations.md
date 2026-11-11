# CPedList: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-AI-2 helper A (groups J1/J6).

## 0x69A3B0 CPedList::BuildListOfPedsOfPedType  (gr_deviations = 1)
Original bytes (`__thiscall void (this, int32 pedType [esp+4])`, `ret 4`): m_count (+0) = 0; walks the ped pool [0xB74490] from slot size-1 down to 0 (`dec edx` 0x69A3D6,
free test `byteMap & 0x80` 0x69A3DD, element 0x7C4); if `ped->m_nPedType (+0x598) == pedType` and m_count < 30 -> m_peds[m_count++] = ped (0x69A3E9-0x69A3FC); then zero
m_peds[m_count..29] (`rep stosd`, 0x69A405-0x69A419).
Row PedList.cpp:58 (NOTE: original iterates backwards): Verdict REFUTED - gr's code (cpp:60, `for (i = size; i-- > 0;)`) iterates downwards like the bytes. Class: harmless.
