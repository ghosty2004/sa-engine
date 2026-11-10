# CPopCycle (POP) - full specs

Globals (gr PopCycle.h): m_nPercTypeGroup 0xC0BC78 `uint8[12 time][2 week][20 zoneType][18 group]` (stride time 0x2D0, week
0x168, zone 0x12), m_pCurrZoneInfo 0xC0BC68 (CZoneInfo*: +0x0F bits 0-4 PopType, bit 7 IsNoCops; +0x10 bits 0-3 PopRaces),
m_nCurrentTimeOfWeek 0xC0BC70, m_nCurrentTimeIndex 0xC0BC74. `_rand` 0x821B1E, `_ftol` 0x821B40 (truncation, pops ST0).
gta-reversed deviation notes for the reversed CPopCycle functions are in CPopCycle.deviations.md.

## 0x60FFD0 CPopCycle::PickPedMIToStreamInForCurrentZone  (catalogue: gr-unreversed, port level 0)
- Convention / signature: `__cdecl int32 ()`, returns the model id in EAX (-1 = none), plain `ret` (0x610131 / 0x61014E).
  Frame: `sub esp,8` + 4 pushes; [esp+0x14] (after the pushes) = try counter, [esp+0x10] = rand temp, then the inner counter.
- Callers: CStreaming::StreamZoneModels 0x40A560 (2 sites). Callees: `_rand` 0x821B1E, `_ftol` 0x821B40.
- Globals read: m_nCurrentTimeOfWeek 0xC0BC70, m_nCurrentTimeIndex 0xC0BC74, m_pCurrZoneInfo 0xC0BC68, m_nPercTypeGroup 0xC0BC78,
  CPopulation::CurrentWorldZone 0xC0FCBC (i32), CPopulation::m_TranslationArray 0x8D2540 (3 x i32 per popcycle group, stride 0xC),
  CPopulation::m_nNumPedsInGroup 0xC0ECC0 (i16[]), CPopulation::m_PedGroups 0xC0F358 (u16[group][0x15]),
  CStreaming::ms_pedsLoaded 0x8E4C00 (i32[8], all 8 entries compared), CModelInfo::ms_modelInfoPtrs 0xA9B0C8 (ped model info +0x3A = race, u8).
  Read/written: CStreaming::ms_NextPedToLoadFromGroup 0x8E4BB8 (i32[18], indexed by the POPCYCLE group, not the ped group).
  Constants: 0x858B14 = 3.0517578e-05f (1/32768), 0x858628 = 100.0f.
- Behaviour: `for (try = 0; try < 10; try++)` (0x60FFD6, 0x610132-0x61013E):
  1. `r = _ftol(((float)(rand() & 0xFFFF) * 3.0517578e-05f) * 100.0f)` (0x60FFE0-0x60FFFE; x87, 0..99).
  2. Inlined group pick (0x610003-0x61004A): `p = &m_nPercTypeGroup[timeIndex][timeOfWeek][zoneInfo->PopType & 0x1F][0]`
     (index `((PopType + (week + timeIndex*2)*20)*18)`), `g = 0; while (r >= p[g]) { r -= p[g]; g++; }` (signed int compare of
     `r` against the zero-extended byte; the loop has no upper bound: it reads past the 18 entries if the remaining `r` never drops
     below a percentage). Note the strict `r < p[g]` stop test (0x610039 `jl`, 0x61004A `jge` loops) - this is NOT the same test as
     CPopCycle::PickARandomGroupOfOtherPeds 0x610420 (which stops at `p[g] >= r`).
  3. `pedGroup = m_TranslationArray[g].pedGroupIds[CurrentWorldZone]` (dword at `0x8D2540 + (g*3 + zone)*4`, 0x61004C-0x610058);
     `n = (int16)m_nNumPedsInGroup[pedGroup]` (movsx, 0x61005F).
  4. `for (k = 0; k < n; k++)` (signed, 0x610070-0x610124):
     `slot = (ms_NextPedToLoadFromGroup[g] + 1) % n` (cdq/idiv signed, 0x61007A-0x610083), stored back to ms_NextPedToLoadFromGroup[g]
     (0x61009C); `mi = m_PedGroups[pedGroup*0x15 + slot]` (u16 zero-extended, 0x610092);
     `ok = mi differs from all 8 ms_pedsLoaded[i]` (0x6100A3-0x6100ED; all 8 compared, no use of ms_numPedsLoaded);
     race test (always executed): `race = ms_modelInfoPtrs[mi]->+0x3A`; if `race != 0` and `((zoneInfo->+0x10 & 0xF) & (uint8)(1 << (race-1))) == 0`
     then `ok = false` (0x6100EF-0x610117; the test is on the low byte only, so race >= 9 always fails).
     If `ok`: return `mi` (0x61012A).
  5. After 10 tries return -1 (`or eax,-1`, 0x610147).
- Edge cases: `n <= 0` skips the inner loop (no division). The group pick can index past the 18 percentages (unbounded loop) if
  the current row sums to less than 100 (cannot happen after Initialise's normalisation, which forces the sum to 100 mod 256).
- gta-reversed: present but unhooked (`.Reversed = false`, PopCycle.cpp:298-313). Differences vs bytes: (1) gr calls
  `PickARandomGroupOfOtherPeds()` whose stop test is `p[g] >= r` (and with gr's RNG formula), while the original inlines a pick
  with `r < p[g]`: when `r` equals a partial remainder (e.g. `r = 0` and `p[0] = 0`) the original moves on to the next group;
  (2) gr's RNG is `GetRandomNumberInRange(0,100)` (cat14 finding 1) instead of `_ftol(rand&0xFFFF * (1/32768) * 100)`.
  The rest (slot rotation, loaded check over all 8 entries, race test) matches.
- Open questions: none.
