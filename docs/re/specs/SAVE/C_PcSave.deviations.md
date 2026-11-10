# C_PcSave (SAVE) - gta-reversed deviation notes (group H1)

Source: `re/gta-reversed/source/game_sa/C_PcSave.cpp`. Globals: `ms_Slots` 0xC16EBC (int32[8]; 0 = filled, 1 = free),
`ms_SlotFileName` 0xC16368 (stride 0x104), `ms_SlotSaveDate` 0xC16138 (stride 0x46), PcSaveHelper.error 0xC17034.

## 0x619140 C_PcSave::PopulateSlotInfo  (gr_deviations = 3)
Original (`__thiscall void ()`, plain `ret`), bytes 0x619140-0x6193A1:
1. `error = 0`; `ms_Slots[0..7] = 1`; for each slot `ms_SlotFileName[i][0] = 0`, `ms_SlotSaveDate[i][0] = 0`.
   `monthText = NULL` (local, set ONCE before the slot loop, 0x61915B).
2. For i = 0..7: `sprintf(path, "%s%i%s", DefaultPCSaveFileName, i + 1, ".b")`; `f = OpenFile(path, "rb")`; if f:
   `Seek(f, 5, 0)` (constant 5, not strlen of the block tag), `Read(f, vars, 0x138)`; if `strncmp(vars.m_szSaveName (vars+4),
   "THIS FILE IS NOT VALID YET" (0x8D2BE4), 26) != 0`: copy 48 bytes (12 dwords) of the name into ms_SlotFileName[i],
   `ms_Slots[i] = 0`, `ms_SlotFileName[i][24] = 0`; `CloseFile(f)`.
3. If `ms_Slots[i] == 0`: if `!CGenericGameStorage::CheckDataNotCorrupt(i, path)` 0x5D1170:
   `InsertNumberInString(TheText.Get("FEC_SLC"), i + 1, -1, -1, -1, -1, -1, ms_SlotFileName[i])` (0x619268-0x619284);
   the slot state is NOT changed. Else: `switch (wMonth - 1)` 0..11 (table 0x6193A4) -> `monthText =
   GxtToAscii(TheText.Get("MONTH1".."MONTH12"))` (0x69F8C0); out of range keeps the previous monthText (NULL for the
   first); `sprintf(date, "%02d %s %04d %02d:%02d:%02d" (0x86D2FC), wDay, monthText, wYear, wHour, wMinute, wSecond)`;
   `AsciiToGxtChar(date, ms_SlotSaveDate[i])`.
Rows C_PcSave.cpp:74 and :78 (`assert(time.wMonth - 1 < 12); // NOTSA`):
- Original bytes 0x6192B1-0x6192C5: no assert; out-of-range months reuse the previous slot's month text (or NULL).
- Verdict: REFUTED as asserts (debug only). Class: harmless. Found in passing: gr's release behaviour for a bad month is
  `TheText.Get("MONTH<n>")` instead of the stale pointer (CONFIRMED, keep-original).
Row C_PcSave.cpp:85 (`ms_Slots[i] = SLOT_CORRUPTED; // NOTSA: Missing corrupted slot`):
- Original bytes 0x619268-0x61928C: no store to ms_Slots; the corrupt slot stays "filled" (0) with the FEC_SLC text as
  its name.
- Verdict: CONFIRMED. Class: keep-original.
Found in passing (CONFIRMED, keep-original): gr passes `i` as the number for FEC_SLC; the original passes `i + 1`
(esi, 0x619282).
