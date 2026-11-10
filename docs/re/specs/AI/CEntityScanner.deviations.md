# CEntityScanner: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x5FFA20 CEntityScanner::ScanForEntitiesInRange  (gr_deviations = 2)
Original sector range (0x5FFA67-0x5FFC31): `range = max(intel->m_fSeeingRange (+0xBC), intel->m_fHearingRange (+0xC0))` (the +0xBC value is
taken when `[+0xBC] > [+0xC0]`; equal/NaN take +0xC0); `p = ped position`; for each bound the index is
`ftol(floor((double)((p.c -/+ range) * 0.02f [0x858B38] + 60.0f [0x858B34])))` (`floor` 0x8219F0 on a double, then `_ftol` 0x821B40), each
expression evaluated TWICE (once for the test, once for the value). Low bounds: `x0 = (idx > 0) ? idx : 0`; high bounds: `x1 = (idx < 0x77)
? idx : 0x77` (119). Then scan code: `if (ms_nCurrentScanCode >= 0xFFFF) { ClearScanCodes(); code = 1 } else code++`; the scanning ped's
own scan code (+0x2C) is set so it is skipped. Loops y = y0..y1 (outer), x = x0..x1 (inner), sector = ms_aRepeatSectors[(y & 0xF)*16 +
(x & 0xF)] (0xB992B8, stride 0xC).
Row EntityScanner.cpp:99 (gr iterates the rect and skips sectors outside 0..119):
- Original: clamps the index range to [0, 119] before looping (0x5FFAEE, 0x5FFB4B, 0x5FFBA2, 0x5FFC00), so the same set of in-map
  sectors is visited and nothing outside is scanned.
- Verdict: REFUTED for the set of sectors, PROVIDED gr's `CRect{pos, range}` / IterateSectorsOverlappedByRect produce the same
  indices as `ftol(floor((c +- range)*0.02f + 60.0f))` - the port must use the formula above. Iteration order (y outer, x inner) affects
  the order of equal-distance insertions; not verified for gr's helper. Class: harmless (if indices/order match).
Row EntityScanner.cpp:106 (`default: NOTSA_UNREACHABLE_CASE(sectorList)`):
- Original: list selector [esp+0x78] (arg 1): 0 -> sector+0 (Vehicles), 1 -> +4 (Peds), 2 -> +8 (Objects); any other value leaves ECX = 0
  (0x5FFC9E) and reads the list head from address 0 (0x5FFCBD) - an access violation.
- Verdict: REFUTED (both are fatal / undefined; no defined behaviour to preserve). Class: harmless.
