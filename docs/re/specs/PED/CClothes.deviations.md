# CClothes: gta-reversed deviation notes (PED)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x5A7B30 CClothes::LoadClothesFile  (gr_deviations = 1)
Row Clothes.cpp:103 (`NOTSA_UNREACHABLE("Invalid rule tag: {}", strTag)` in the tag lookup lambda).
- Original bytes: locals: `tag` ([esp+0x10] after `push ebx`, **never initialised**), delimiter string held in a dword local = 0x002C0920
  = `" \t,"` (0x5A7B46), `inRule` = 0. `ms_numRuleTags = 0`; `f = CFileMgr::OpenFile("DATA\\CLOTHES.DAT", "r")`; per `CFileLoader::LoadLine(f)`
  line: skip if empty or '#'; if !inRule: `inRule = strncmp("rule", line, 4) == 0`; else if `strncmp("end", line, 3) == 0`: inRule = 0;
  else `t = strtok(line, " \t,")` (CRT 0x82244B, global strtok state); skip if null; then **case-insensitive** `_stricmp` against
  "cuts" 0, "setc" 1, "tex" 2, "hide" 3, "ignore" 5, "endignore" 4, "exclusive" 7, "endexclusive" 6 (in that order, 0x5A7BE5-0x5A7CC0);
  **no match leaves `tag` unchanged** (the previous rule line's tag, or stack garbage on the first one); `ms_clothesRules[ms_numRuleTags++] = tag`;
  `a1 = strtok(0, delims)`, `a2 = strtok(0, delims)`; `if ((uint32)tag <= 7)` jump table 0x5A7E7C handles the arguments, else nothing more.
- Verdict: CONFIRMED - gr aborts/UB on an unknown tag; the original reuses the stale tag value. Also: gr compares with case-sensitive
  `strcmp` against UPPERCASE names, the original uses `_stricmp` with lowercase literals (equivalent only for files in upper case).
- Class: keep-original.
