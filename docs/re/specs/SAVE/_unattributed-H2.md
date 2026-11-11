# SAVE unattributed functions - RE-SPEC-REST group B

## 0x5D13FD sub_5d13fd (proposed: CGenericGameStorage::GenericSave, split body chunk)  (catalogue: unknown/REA, port level 7)
- Name proposal: this is not a function but the main body of CGenericGameStorage::GenericSave 0x5D13E0 (gr-reversed, GenericGameStorage.cpp:422),
  reached by `jne 0x5D13FD` at 0x5D13F4 after `sub esp,0x138; ms_bFailed = 0; OpenFileForWriting()`. It shares GenericSave's frame
  (`add esp,0x138` in every epilogue, plain `ret`, returns AL). Port it as part of GenericSave; the differences to gr are listed here.
- Loop `block = 0 .. 27` (EBX; `ms_CheckSum = 0` first, 0x5D1403):
  - If ms_bFailed (0xC16EFC) -> FAIL. Header = 5 bytes of ms_BlockTagName (0x8D2BD8, "BLOCK"), written with an INLINED
    SaveDataToWorkBuffer: if `ms_WorkBufferPos + 5 > 0xC800` (literal constant, unsigned): `SaveDataToWorkBuffer(tag, 0xC800 - pos)`
    0x5D1270, then `SaveWorkBuffer(false)` 0x5D0F80 (either failing -> FAIL), and the remaining bytes are copied from tag + (0xC800-pos);
    then `memcpy(ms_WorkBuffer + pos, src, len)` (rep movsd/movsb) and `pos += len`. gr calls SaveDataToWorkBuffer(tag, strlen(tag)) which
    compares against ms_WorkBufferSize instead of 0xC800 (differs only if ms_WorkBufferSize != 0xC800, e.g. after a load left it smaller;
    OpenFileForWriting does not reset it) - CONFIRMED difference, keep the bytes.
  - `switch (block)` via jump table 0x5D1734 (`block > 0x1B` -> default). Case 0 (0x5D14AA): local CSimpleVariablesSaveStructure::Construct
    0x5D1B80, `ms_bFailed = !SaveDataToWorkBuffer(&vars, 0x138)`. Cases 7, 13, 14 go straight to the check (unused blocks, as in gr).
    The other cases (0x5D14D5-0x5D15A6) are outside this chunk's extent and call the per-system Save functions in the gr order.
  - After each case (stub 0x4012C6 -> 0x5D15B0): if ms_bFailed -> FAIL2 (same as FAIL).
- Padding (0x5D15C2): `SIZE = 0x317FC`. If `pos + ms_FilePos < SIZE`: loop { `rem = SIZE - pos - ms_FilePos`; if `pos + rem < 0xC800`
  { `pos += rem`; break } ; `pos = 0xC800`; if `!SaveWorkBuffer(false)` -> FAIL; if `pos + ms_FilePos >= SIZE` break }. The padding bytes
  are whatever the work buffer already contains (stale data, not zeros).
- Final: `ok = SaveWorkBuffer(true)`. If !ok: free ms_WorkBuffer (if non-null; set 0), `CFileMgr::CloseFile(ms_FileHandle)`, return 0.
  Else free ms_WorkBuffer (set 0); `r = CFileMgr::CloseFile(ms_FileHandle)`; `strncpy(ms_SaveFileNameJustSaved 0xC16030, ms_SaveFileName
  0xC16DB8, 0x103)` 0x821F40 (after the close; gr does it before - no behavioural difference); if r != 0 return 0; else
  `CPad::UpdatePads()` (thunk 0x541E50 -> 0x541DD0); return 1.
- FAIL / FAIL2 (0x5D1628 / 0x5D1660): free ms_WorkBuffer (if non-null; set 0); `CFileMgr::CloseFile(ms_FileHandle)`; return 0 (gr calls
  CloseFile() which does the same).
- Open questions: the case bodies 0x5D14D5-0x5D15A6 are not in this bundle (not re-verified against gr's call list).

## 0x69F8C0 sub_69f8c0 (proposed: UNCLEAR - text sanitiser used by C_PcSave::PopulateSlotInfo)  (catalogue: unknown/REA, port level 0)
- Name proposal: no gr reference; only caller C_PcSave::PopulateSlotInfo 0x619140; converts a byte string into a static display buffer.
- `__cdecl char* (const char* src [esp+4])`, plain `ret`; returns `buf = 0xC1BED0` (static .data buffer, >= 0x101 bytes used).
- `n = 0`; if src != 0: while `n <= 0xFF` and `src[n] != 0`: `c = src[n]`; if `c >= 0x7F` (unsigned) or c == '/' (0x2F) or c == '*' (0x2A)
  or c == '?' (0x3F) `buf[n] = ' '` else `buf[n] = c`; n++. Then `buf[n] = 0` (also when src == 0: buf[0] = 0). Return buf.
