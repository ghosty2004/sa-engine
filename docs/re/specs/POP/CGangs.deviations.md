# CGangs (POP) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Gangs.cpp` (pin 20d8d2a). CGangs::Gang 0xC091F0 (CGangInfo x10, 0x10 each: +0 int8
m_nPedModelOverride, +4/+8/+0xC weapons). CPopulation::m_TranslationArray (+0xD8 = 0x8D2618, dword per entry),
m_nNumPedsInGroup 0xC0ECC0 (int16), m_PedGroups 0xC0F358 (rows of 21 u16 = 0x2A bytes), CurrentWorldZone 0xC0FCBC.

## 0x5DE5A0 CGangs::ChooseGangPedModel  (gr_deviations = 1)
Original: `__cdecl int32 (int16 gangId [esp+4])` (read `movsx` 0x5DE5A2), EAX.
- Override: `if (Gang[g].m_nPedModelOverride (byte) != 0xFF)` (0x5DE5AC) return `m_PedGroups[T[g*3 + zone]][0]` (u16,
  0x5DE658-0x5DE66D), T = 0x8D2618, zone = CurrentWorldZone.
- Else: `n = (int16)m_nNumPedsInGroup[T[g*3]]` (zone 0, 0x5DE5BD); `CCarCtrl::InitSequence(n)` 0x421740 (cdecl); `for (i = 0;
  i < n (signed, n re-read from the zone-0 entry every iteration, 0x5DE623-0x5DE635); i++)`: `e = CCarCtrl::FindSequenceElement(i)`
  0x421770; `model = m_PedGroups[T[g*3 + CurrentWorldZone]][e]` (zone re-read per iteration, 0x5DE5F6); if
  `CStreaming::ms_aInfoForModel[model].m_nLoadState (0x8E4CD0 + 20*model) == 1` return model (0x5DE619). Returns -1 when
  n <= 0 (0x5DE5E6) or none is loaded (0x5DE639).
Row Gangs.cpp:59 `NOTE: The original sizes the random sequence using world zone 0, but looks the model up using the current world zone`
- Original bytes: as above (sizing via `T[g*3]` at 0x5DE5BD/0x5DE623, lookup via `T[g*3 + zone]` at 0x5DE5FF).
- Verdict: REFUTED - gr reproduces the original (zone-0 count, current-zone lookup). Class: harmless. gr's `numPeds == 0` check is
  `<= 0` in the original (int16 count, never negative).
