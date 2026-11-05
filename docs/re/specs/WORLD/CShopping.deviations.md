# CShopping: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x49aad0 CShopping::GetPriceSectionFromName  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Shopping.cpp:380` (notsa-unreachable): `NOTSA_UNREACHABLE();` after the loop over ms_sectionNames.
  - Original bytes: loop `esi = 0..10` (`cmp esi,0xb; jl`, 0x49aaf6) over `ms_sectionNames[esi]` (0x8a61d8), `_stricmp(name, sectionName)` (0x49aae9, CRT 0x8229b6; arg order: name first). Match returns esi (0x49ab02). No match: returns -1 (`or eax,0xffffffff`, 0x49aafc).
  - Verdict: CONFIRMED. The original returns -1 for an unknown section name; gr has undefined behaviour.
  - Class: keep-original.

## 0x49ab10 CShopping::GetItemIndex  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/Shopping.cpp:306` (notsa-unreachable): `NOTSA_UNREACHABLE();` after the linear search of ms_keys.
  - Original bytes: body 0x156e330. `eax = 0..0x22f` (`cmp eax,0x230; jl`, 0x156e340), `cmp [eax*4+0xa97d90 ms_keys], key` (0x156e336), match returns eax. No match: returns -1 (0x156e347).
  - Verdict: CONFIRMED. The original returns -1 when the key is not in the 560-entry table. Callers such as HasPlayerBought (0x49b5e0) / SetPlayerHasBought (0x49b610) then index `ms_bHasBought[-1]` (one byte before the array); a port must reproduce that read/write address or document it.
  - Class: keep-original.

## 0x49ad20 CShopping::FindItem  (gr_deviations = 1)
Rows checked: 2 (CONFIRMED 1, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Shopping.cpp:228` (inlined): "@addr 0x49AD20 (inlined)" (doc comment). REFUTED: comment only; 0x49ad20 exists as a real function. Class: harmless. (Not counted in gr_deviations.)
- gr `Shopping.cpp:241` (notsa-unreachable): `NOTSA_UNREACHABLE();` when the key is not found.
  - Original bytes: `edx = ms_numPrices (0xa9a7cc)`, `eax = 0`; if `edx <= 0` (signed, `jle`, 0x49ad2b) -> return -1; else scan `ms_prices[eax].key` (0xa986f0, stride 0x18, key at +0) for `eax < edx` (0x49ad36-0x49ad40); match returns eax; no match returns -1 (0x49ad42).
  - Verdict: CONFIRMED. Original returns -1 when not found (and when ms_numPrices <= 0).
  - Class: keep-original.

## 0x5d3de0 CShopping::Save  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Shopping.cpp:815` (bug-note): `SaveDataToWorkBuffer(&ms_bHasBought, ms_numBuyableItems); // TODO: This looks funky`.
  - Original bytes: `SaveDataToWorkBuffer(&ms_numPriceModifiers 0xa9a7d0, 4)`; `for (u32 i = 0; i < ms_numPriceModifiers (re-read); i++) Save(&ms_priceModifiers[i] 0xa98650 + i*8, 8)`; `Save(&ms_numBuyableItems 0xa9a310, 4)`; `Save(&ms_bHasBought 0xa972a0, ms_numBuyableItems)` (byte count = item count, 0x5d3e26-0x5d3e31). Writer 0x5d1270.
  - Verdict: REFUTED (gr does exactly what the original does; the "funky" part is original behaviour).
  - Class: harmless.
- Found in passing: the original returns `AL = 1` (0x5d3e39); gr's `Save()` is `void`. Port should return true if any caller tests it.

## 0x5d3e40 CShopping::Load  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 0, REFUTED 1, UNCLEAR 0).
- gr `source/game_sa/Shopping.cpp:804` (bug-note): `LoadDataFromWorkBuffer(&ms_bHasBought, ms_numBuyableItems); // TODO: This looks funky`.
  - Original bytes: mirror of Save with LoadDataFromWorkBuffer (0x5d1300): count (4 bytes), `ms_numPriceModifiers` entries of 8 bytes (unsigned loop, count re-read), `ms_numBuyableItems` (4), then `ms_numBuyableItems` bytes into `ms_bHasBought` (0xa972a0) (0x5d3e86-0x5d3e91). No bounds check on the loaded counts.
  - Verdict: REFUTED (gr matches).
  - Class: harmless.
- Found in passing: returns `AL = 1` (0x5d3e99); gr's `Load()` is `void`.
