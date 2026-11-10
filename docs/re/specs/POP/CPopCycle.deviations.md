# CPopCycle (POP) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/PopCycle.cpp`. Layout: m_nPercTypeGroup 0xC0BC78 `uint8[12][2][20][18]`; the six
`uint8[12][2][20]` tables m_nMaxNumPeds 0xC0E798, m_nMaxNumCars 0xC0E5B8, m_nPercDealers 0xC0E3D8, m_nPercGang 0xC0E1F8,
m_nPercCops 0xC0E018, m_nPercOther 0xC0DE38. `_rand` 0x821B1E, `_ftol` 0x821B40 (truncation). RNG background:
docs/re/cat14-gr-deviations.md finding 1.

## 0x5BC090 CPopCycle::Initialise  (gr_deviations = 7)
Original (`__cdecl void ()`, frame `sub esp,0x84`): `CFileMgr::SetDir("DATA")` (0x5BC09F, string 0x86A784), `f = CFileMgr::OpenFile
("POPCYCLE.DAT" 0x86A7D8, "rb" 0x859C6C)` (0x5BC0AE), `SetDir("")` (0x858B54, 0x5BC0BE). Loops: zone 0..19 (esi += 0x12 while
`< 0x168`, 0x5BC4CD-0x5BC4DF) > week 0..1 ([esp+0x1C] = 2 down-counter, 0x5BC4B2) > time 0..11 ([esp+0x30] = 12 down-counter,
0x5BC48E); u8-table index `zone + week*20 + time*40`, perc index `zone*18 + week*360 + time*720`. Per data line:
1. `l = LoadLine(f)` 0x536F80; while `l != NULL && (l[0] == '/' || l[0] == 0)` read the next line (0x5BC114-0x5BC137).
2. `sscanf(l, "%d %d ... %d" (24 x %d, 0x86A790), &i[0..23])` into 24 int locals (0x5BC1E4); the return value is ignored. Then the
   LOW BYTE of each int is stored: i0 MaxNumPeds, i1 MaxNumCars, i2 PercDealers, i3 PercGang, i4 PercCops, i5 PercOther,
   i6..i23 -> percs[0..17] (0x5BC1E9-0x5BC30B; argument/slot mapping verified push by push).
3. Normalisation, ALWAYS executed (no "sum < 100" condition): `sum = percs[0] + ... + percs[17]` (int, 0x5BC320-0x5BC381, two
   blocks of 9); `k = 100.0f / (float)sum` (fild, `fdivr [0x858628]` = 100.0f, kept in ST1, not rounded to f32 memory);
   for j = 0..17: `percs[j] = (uint8)_ftol((float)percs[j] * k)` (0x5BC3A0-0x5BC3C2).
4. `newSum` and `best` = index of the largest new value, ties to the LAST index (`cmp ebx,eax; jl skip`, start max 0 / index 0,
   0x5BC3C4-0x5BC463); `percs[best] += (uint8)(100 - newSum)` (u8 wrap-around arithmetic, 0x5BC469-0x5BC47A). So every row
   sums to exactly 100 (mod 256) after loading.
After the loops: m_nCurrentTimeIndex 0xC0BC74 = 0, m_nCurrentTimeOfWeek 0xC0BC70 = 0, m_pCurrZoneInfo 0xC0BC68 = NULL,
m_nCurrentZoneType 0xC0BC6C = -1 (0x5BC4EC-0x5BC4FB), then `CloseFile(f)` (0x5BC505).

Rows:
- PopCycle.cpp:39 `notsa::ScopeGuard` closing the file. Original: CloseFile at 0x5BC505 after the loops. REFUTED. harmless.
- PopCycle.cpp:50 `NOTSA_UNREACHABLE("Expected more data, got EOF!")`. Original: when LoadLine returns NULL (0x5BC11F/0x5BC137 ->
  0x5BC139) it still calls `sscanf(NULL, ...)` (CRT behaviour: invalid-parameter / crash). CONFIRMED (both abort on a short file;
  only the failure mode differs). harmless (a malformed data file is not a supported state).
- PopCycle.cpp:81 `NOTSA_UNREACHABLE("Failed reading all data!")` on `nread != 24`. Original ignores the sscanf result and stores
  whatever is in the int locals (stale values from the previous line, or uninitialised stack for the first line). CONFIRMED.
  keep-original only matters for malformed files; harmless for the shipped POPCYCLE.DAT.
- PopCycle.cpp:84 comment "original code rescales ... but fails sometimes". Original: the rescale is followed by the
  "add the deficit to the largest entry" fix-up (0x5BC469-0x5BC47A), so the row always sums to 100 (mod 256). REFUTED (the comment
  is wrong about the original).
- PopCycle.cpp:86 `#ifndef FIX_BUGS` block (rescale only when `sum < 100`, integer `p * 100 / sum`, no fix-up). CONFIRMED: the
  original always rescales, uses float `_ftol((float)p * (100.0f / sum))` and then adds `100 - newSum` to the largest entry
  (last one on ties). gr without FIX_BUGS gives different percentages (rows summing above 100 are left untouched, rows below are
  not fixed up); gr with FIX_BUGS keeps the raw values. keep-original. Note: a row with `sum == 0` gives `k = +inf`, `0 * inf = NaN`,
  `_ftol(NaN)` = 0x80000000 -> low byte 0 for all, then `percs[17] += 100` (index 17 because ties go to the last index).
- PopCycle.cpp:88 `notsa::accumulate` helper. REFUTED as a marker (same int sum as 0x5BC320). harmless.
- PopCycle.cpp:94 commented-out assert. REFUTED (no code). harmless.
Found in passing (CONFIRMED, keep-original): (a) the format is 24 x `%d` into ints with low-byte stores, gr uses `%hhu` (same
result for 0..255 values); (b) open mode "rb" (gr "r"; LoadLine 0x536F80 turns control characters incl. '\r' into spaces, so no
behavioural difference); (c) gr does not reset m_nCurrentTimeIndex / m_nCurrentTimeOfWeek / m_pCurrZoneInfo / m_nCurrentZoneType
(0, 0, NULL, -1) at the end; the port must (Initialise is also called from CGame::ReInitGameObjectVariables 0x53BCF0).

## 0x60FBD0 CPopCycle::FindNewPedType  (gr_deviations = 7)
Original: `__cdecl bool (ePedType* outType [esp+4], int32* outMI [+8], bool noGangs [+0xC], bool noCops [+0x10])`, returns AL,
plain `ret`. Locals (before the 4 pushes at 0x60FDE0): gang [esp], dealers [+4], cops [+8], civ [+0xC].
1. `if (m_pCurrZoneInfo == NULL) return 0` (0x60FBD0-0x60FBE1); nothing is written.
2. If `CPopulation::bInPoliceStation` 0xC0FCB6: `r = _ftol(((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14]) * 100.0f [0x858628])`;
   if `r < 70` (0x60FC0E, signed): `*outType = 6` (COP), `*outMI = CPopulation::ChoosePolicePedOccupation()` 0x610F40, return 1.
3. `dealers = m_NumDealers_Peds 0xC0E978 - (float)ms_nNumDealers 0xC0EC38` (0x60FC2E-0x60FC41).
   `gang = m_NumGangs_Peds 0xC0BC48 - (float)(ms_nNumGang[0] + ... + [9])` (fild + 9 x fiadd 0xC0EC3C..0xC0EC60, 0x60FC45-0x60FC89);
   if m_bOnlyCreateRandomGangMembers 0xC0FCB3: `gang = 50.0f`; if m_bDontCreateRandomGangMembers 0xC0FCB2 or `noGangs`: `gang = -10.0f`.
   `cops = m_NumCops_Peds 0xC0BC44 - (float)ms_nNumCop 0xC0EC68`; `if CGangWars::GangWarFightingGoingOn() 0x443AC0` (always called)
   `cops = -10`; if m_bDontCreateRandomCops 0xC0FCB4 or `noCops`: `cops = -10`; if `m_pCurrZoneInfo && (zi->+0x0F & 0x80)` (IsNoCops):
   `cops = -10` (0x60FCB2-0x60FCFC).
   `civ = m_NumOther_Peds 0xC0BC40 - (float)(ms_nNumCivFemale 0xC0EC6C + ms_nNumCivMale 0xC0EC70)` (int add first, 0x60FD04-0x60FD1F).
4. In the order civ, cops, dealers, gang: `if (x < 2.0f [0x858CA0])` (NaN false) `x = ((float)rand() * 3.0518509e-05f [0x858C7C]) * x`
   (full rand, no mask; 0x60FD23-0x60FDCB). Then `if (CGame::currArea 0xB72914 != 0) dealers = -10.0f` (0x60FDCF).
5. Loop (0x60FDF1): `m1 = dealers > gang ? dealers : gang` (ties/NaN -> gang); `m2 = m1 > cops ? m1 : cops`; `hi = m2 > civ ? m2 : civ`
   (each compare `test ah,0x41; jne`). `if (hi <= 0.0f) return 0` (0x60FE88-0x60FE93: `test ah,0x41; jnp` - NaN continues).
   - `hi == dealers` (0x60FE99, `test ah,0x44; jp`): `*outType = 0x11` (DEALER), `*outMI = -1`; `grp = m_TranslationArray[28].ids[0]`
     (dword 0x8D2690); `for (i = 0; i < (int16)m_nNumPedsInGroup[grp]; i++) { mi = m_PedGroups[grp*0x15 + i]; if (ms_aInfoForModel[mi]
     .m_nLoadState (byte 0x8E4CD0 + mi*0x14) == 1) *outMI = mi; }` (no break: the LAST loaded model wins). If `*outMI >= 0` return 1,
     else `dealers = 0` and loop.
   - `hi == gang` (0x60FF0A): `t = PickGangToCreateMembersOf()` 0x60F8D0; `*outType = t`; if `t == 0` (returned by 0x60F99A when the
     zone's gang density sum <= 0): `*outMI = -1`; else `*outMI = ChooseGangOccupation(t - 7)` 0x611550 and return 1 if `>= 0`.
     On failure: if m_bOnlyCreateRandomGangMembers return 0, else `gang = 0` and loop.
   - `hi == cops` (0x60FF62): `*outType = 6`, `*outMI = ChoosePolicePedOccupation()`, return 1.
   - `hi == civ` (0x60FF86): `mi = ChooseCivilianOccupation(0, 0, -1, -1, -1, 0, 1, 0, 0)` 0x612F90 (9 cdecl args); `*outMI = mi`;
     if `mi < 0 || mi == 7` return 0; `*outType = ms_modelInfoPtrs[mi]->+0x28` (m_nPedType); return 1.
   - none equal (only with NaN chances): return 1 (`mov al,bl`, 0x60FF8F -> 0x60FFC1) with nothing written in this pass.

Rows:
- PopCycle.cpp:106 `outPedMI = MODEL_INVALID` at entry ("NOTSA: Bug prevention"). Original writes nothing on the early-out paths
  (zone info NULL, `hi <= 0`, gang failure with m_bOnlyCreateRandomGangMembers keeps the previous `*outMI`). CONFIRMED. keep-original.
- PopCycle.cpp:138 `*chance *= GetRandomNumberInRange(0.f, 1.f)`. gr float version = `lerp(0, 1, t)` = `1*t + 0*(1-t)` = `t` with
  `t = (float)rand() * (1/32767.0f)` = same constant 0x38000100 as 0x858C7C; the product `t * chance` equals the original's
  `(rand * k) * chance` (multiplication order irrelevant for one rounding, 24-bit precision). REFUTED (numerically identical, given
  gr's `lerp` evaluates to exactly `t`). harmless.
- PopCycle.cpp:156 comment "originally ... always returned at the last viable ID"; gr iterates reversed and returns the first.
  REFUTED for the selected model (both give the last loaded model of the dealer group). But CONFIRMED side effect: the original sets
  `*outType = 0x11` and `*outMI = -1` BEFORE the search (0x60FEA4/0x60FEAC), so if no dealer model is loaded and the function later
  returns 0, the caller sees `*outType == DEALER`, `*outMI == -1`; gr leaves them unchanged. keep-original.
- PopCycle.cpp:162, 171, 173, 193 asserts. REFUTED (no code in the original). harmless. (Note for 171: the original's
  PickGangToCreateMembersOf can return 0, handled at 0x60FF20; gr has no such branch - see below.)
- PopCycle.cpp:197 `NOTSA_UNREACHABLE()` when `hi` equals none of the four. Original returns 1 (0x60FF8F `jp 0x60FFC1`, AL = BL = 1)
  without writing outputs. Reachable only with NaN chances (`hi` NaN passes the `<= 0` test). CONFIRMED. keep-original.
Found in passing (CONFIRMED, keep-original): (a) police station roll: original `_ftol(rand&0xFFFF / 32768 * 100) < 70`, i.e.
`rand <= 22937`; gr `RandomBool(70)` = `lerp(0,100,rand/32767) <= 70`, i.e. `rand <= 22936`. (b) gang branch: when
PickGangToCreateMembersOf returns 0 the original does not call ChooseGangOccupation (stores -1); gr calls
`ChooseGangOccupation(-7)` (out of bounds). (c) The dealers `-10` rule is `CGame::currArea != 0` (gr
`!CanSeeOutSideFromCurrArea()`, same test per Game.cpp:183).

## 0x610420 CPopCycle::PickARandomGroupOfOtherPeds  (gr_deviations = 4)
Original: `__cdecl int32 ()` returns EAX. `r = _ftol(((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14]) * 100.0f [0x858628])`
(0..99, 0x610423-0x610441). `p = &m_nPercTypeGroup[timeIndex 0xC0BC74][timeOfWeek 0xC0BC70][zi->+0x0F & 0x1F]` (0x610446-0x610470);
`g = 0; while (p[g] < r) { r -= p[g]; g++; }` (signed compare of the zero-extended byte with r, 0x610477-0x61048A); return g.
No bound on g.
Rows:
- PopCycle.cpp:280 `GetRandomNumberInRange(0, 100)` (gr int version = `(int)lerp(0, 99, rand/32767.0f)`). Original:
  `_ftol((rand & 0xFFFF) * (1/32768) * 100)`. Both 0..99 but the mapping from rand() differs. CONFIRMED. keep-original.
- PopCycle.cpp:282 `#ifdef FIX_BUGS` (range = sum of percs). Original uses 100 (the #else branch). REFUTED for the non-FIX_BUGS
  build; keep-original (range 100).
- PopCycle.cpp:283 `notsa::accumulate`. Only in the FIX_BUGS branch. REFUTED. harmless.
- PopCycle.cpp:294 `NOTSA_UNREACHABLE()` after 18 groups. Original has no bound and keeps reading bytes past the row (into the
  next zone's row) until `p[g] >= r`. Unreachable in practice because Initialise forces each row to sum to 100 > r. CONFIRMED
  (structural), harmless.
Stop test `p[g] >= r` (0x61047C `jge`) matches gr's `grpPerc >= rndPerc`.

## 0x610BF0 CPopCycle::Update  (gr_deviations = 1)
Original: `__cdecl void ()`. `day = CClock::CurrentDay 0xB7014E (u8)`, `h = ms_nGameClockHours 0xB70153 (u8)`. If `day > 7`
(unsigned, 0x610C03) m_nCurrentTimeOfWeek is NOT written; else jump table 0x610D0C = {0x610C0C, 0x610C18, 0x610C26 x4, 0x610C32,
0x610C0C}: day 0,7 -> 1; day 1 -> `h < 20 ? 1 : 0` (`cmp cl,0x14; sbb; neg`); days 2..5 -> 0; day 6 -> `h < 20 ? 0 : 1`.
`m_nCurrentTimeIndex = h >> 1` (0x610C48). `pos = FindPlayerCentreOfWorld(PlayerInFocus)`; if `!(pos.z < 950.0f [0x858F4C])` (NaN
included) and `m_pCurrZoneInfo != NULL` keep the zone; otherwise `m_pCurrZoneInfo = CTheZones::GetZoneInfo(&pos, &m_pCurrZone 0xC0BC64)`
0x572400 and `m_nCurrentZoneType = zi->+0x0F & 0x1F`. Then UpdatePercentages 0x610770, UpdateDealerStrengths 0x6104B0,
UpdateAreaDodgyness 0x610560, and the inlined UpdateIsGangArea: `m_bCurrentZoneIsGangArea 0xC0BC5C = (sum of zi bytes +0..+9) > 20`
(signed, 0x610CF9).
Row:
- PopCycle.cpp:341 `NOTSA_UNREACHABLE()` for a day outside 0..7. Original leaves m_nCurrentTimeOfWeek unchanged (`ja 0x610C3E`).
  CONFIRMED (structural; CurrentDay is 1..7 in practice). harmless.
Found in passing: gr's day mapping and the `< 950` condition match the bytes (NaN z: gr `pos.z < 950.f` false -> keeps the zone if
non-NULL, same as the original).
