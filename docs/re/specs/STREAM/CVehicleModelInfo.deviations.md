# CVehicleModelInfo: gta-reversed deviation notes (STREAM)

Source: `re/gta-reversed/source/game_sa/Models/VehicleModelInfo.cpp` (pin 20d8d2a). Fields: +0x54
m_extraComps (dword: comps A bits 0..11, rule A bits 12..15, comps B bits 16..27, rule B bits 28..31),
+0x5C m_pVehicleStruct (numExtras i8 at +0x30C). ms_compsToUse: two int8 at 0x8A6458/0x8A6459 (-2 = none).
Original RNG helper 0x407180: `min + _ftol((rand() & 0xFFFF) * (1/32768.f) * (int)(max - min))`;
gr's lerp helper differs (cat14 finding 1).

## 0x4C8040 CVehicleModelInfo::ChooseComponent  (gr_deviations = 5)
Rows VehicleModelInfo.cpp:344, :348 (this function) and :1627, :1632, :1639 (rng-helper). The last three
are in gr's free function `::ChooseComponent(rule, comps)` = original 0x4C7FB0 (separate catalogue entry
CVehicleModelInfo::ChooseComponent-Global), which 0x4C8040 calls; CAT-14 attributed them here.
- Original 0x4C8040: if `ms_compsToUse[0] != -2` return it (sign-extended) and reset to -2. If rule A != 0
  and valid (rule != 2, or OldWeatherType/NewWeatherType (i16 at 0xC81320/0xC8131C) equals 16 or 8;
  inlined, 0x4C8071-0x4C809C): return 0x4C7FB0(ruleA, compsA). Else
  `r = _ftol((rand() & 0xFFFF) * (1/32768.f) * 3.0f [0x858B3C])` (inline, 0x4C80B7-0x4C80D5); if `r < 2`:
  `n = GetListOfComponentsNotUsedByRules(extraComps, numExtras, list)`; if n != 0 return
  `list[0x407180(0, n)]`. Return -1.
  gr: `GetRandomNumberInRange(0, 3) < 2` is true except for rand() == 32767 (original: about 2/3), and
  the list index uses the lerp helper.
- 0x4C7FB0 (checked in passing): `switch (rule)` via table 0x4C802C: rules 1 and 2:
  `i = 0x407180(0, CountCompsInRule(comps) [0x4C7F80])`, return `(comps >> 4*i) & 0xF` (arithmetic shift);
  rule 3: `i = 0x407180(-1, count)`, -1 -> return -1, else as above; rule 4: tail call
  `0x407180(0, 5)` (0..4); other rules -1.
- Verdict: CONFIRMED for all five rows (helper formula; for :344 also the probability).
- Class: keep-original.

## 0x4C8120 CVehicleModelInfo::ChooseSecondComponent  (gr_deviations = 2)
Rows :363 and :367 (rng-helper).
- Original bytes: `ms_compsToUse[1]` as above; rule B valid -> 0x4C7FB0(ruleB, compsB); else if rule A != 0
  and IsValidCompRule(ruleA) (0x4C7E10) and `0x407180(0, 3) < 2` (0x4C81B1): list as above, return
  `list[0x407180(0, n)]`; else -1.
- Verdict: CONFIRMED for both rows (helper formula).
- Class: keep-original.

## 0x4C95C0 CVehicleModelInfo::SetClump  (gr_deviations = 1)
Row VehicleModelInfo.cpp:242 (NOTSA `FindDirtMaterials()`).
- Original bytes: CVehicleStructure from m_pInfoPool, CClumpModelInfo::SetClump, SetAtomicRenderCallbacks,
  SetFrameIds(ms_vehicleDescs[type]), SetRenderPipelines, PreprocessHierarchy, ReduceMaterialsInVehicle,
  the four current colours = 0xFF (+0x2D2..+0x2D5), SetCarCustomPlate, return (0x4C95D9-0x4C9674).
  There is no dirt-material pass and m_apDirtMaterials is not written.
- Verdict: CONFIRMED (gr's FindDirtMaterials always clears m_apDirtMaterials and, with the reversible fix
  `CVehicleModelInfo_SetClump_DirtMaterialsNeverFound` on by default, fills it).
- Class: keep-original (no call, field untouched).

## 0x5B6890 CVehicleModelInfo::LoadVehicleColours  (gr_deviations = 1)
Rows VehicleModelInfo.cpp:1328 (`#ifdef FIX_BUGS` retry with "%d.%d %d") and :1330 (comment).
- Original bytes: each line is first sanitised: ',' and '\r' become ' ' (0x5B6922-0x5B6932), the line
  ends at '\n'. In the "col" section, `sscanf(line, "%d %d %d", &r, &g, &b)` (format 0x869718,
  0x5B6A3E) with no retry; r/g/b are stack locals that are **not** reset per line. The colour is stored as
  bytes r, g, b, 0xFF (0x5B6A4F-0x5B6A62) whatever sscanf returned. For the shipped typo line
  `77.93,96` the sanitised text is `77.93 96`: r = 77, the parse stops at '.', and g/b keep the values of
  the previous "col" line.
- Verdict :1328: CONFIRMED (gr reparses and gets 77, 93, 96). Class keep-original.
- Verdict :1330: REFUTED (comment; the behaviour is covered by :1328). Class harmless.
- Open question (0x5B6A22): whether any code between two "col" lines writes the g/b stack slots
  (frame offsets entry-relative +0x1C/+0x20 of the sscanf frame); the car/car4 parsers in the same frame
  were not audited, but in carcols.dat the col section precedes them.
