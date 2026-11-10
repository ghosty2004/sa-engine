# CCustomCarPlateMgr (RENDER) - full specs

## 0x6FDC00 CCustomCarPlateMgr::LoadPlatecharsetDat  (catalogue: specified/REA, port level 2)
- Convention: `__cdecl bool (const char* filename [esp+4], u32* out [esp+8])`, returns AL, plain `ret`. gr declares
  `int8 (const char*, uint8*)`; the function writes one u32 per accepted line through `out` (ebp += 4, 0x6FDD25/0x6FDD28).
- Callers: none found (catalogue callers 0). Callees: CFileMgr::SetDir 0x5387D0, CFileMgr::OpenFile 0x538900 (filename, "r"),
  CFileLoader::LoadLine(FILE) 0x536F80, CFileMgr::CloseFile 0x5389D0, CRT `strtok` 0x82244B (excluded:crt; strtok signature,
  called as (line, delim) then (NULL, delim)), CRT `atol` 0x82258E.
- Data: 0x86A784 "DATA", 0x85A53C "r", 0x858B54 "" (empty), 0x86A958 ";the end", 0x85A510 bytes 20 09 00 = delimiter " \t"
  (copied into the stack slot of the `out` argument at 0x6FDC6F-0x6FDC89 after `out` was loaded into EBP).
- Behaviour:
  1. `SetDir("DATA")`; `fd = OpenFile(filename, "r")`; `SetDir("")` (0x6FDC07-0x6FDC2B). No null check of fd.
  2. `line = LoadLine(fd)`; while line != null (0x6FDC3B, 0x6FDD3A):
     a. If the 9 bytes at line equal ";the end\0" (`repe cmpsb`, ecx = 9, 0x6FDC5E) -> stop (exact match incl. terminator).
     b. If `line[0] == ';'` -> skip line (0x6FDC66).
     c. `tok = strtok(line, " \t")`; n = 0. Loop (0x6FDC95): switch n: 0 -> c0 = clamp(atol(tok)); 1 -> c1 = clamp(atol(tok));
        2 -> c2 = clamp(atol(tok)); >= 3 -> ignored. Then `tok = strtok(NULL, " \t")`, n++, repeat while tok != NULL (0x6FDD02).
        The FIRST token is processed without a NULL check. clamp(v): `cmp ax, 0xFF; jbe` on the low 16 bits, unsigned:
        `(u16)v > 255 ? 255 : (u8)v` (e.g. -1 -> 255, 256 -> 255, 65536 -> 0).
     d. If n < 3 (0x6FDD04 `jb`) -> return 0 immediately (0x6FDD57) WITHOUT closing the file.
     e. `*out++ = 0x80000000 | c2 << 16 | c1 << 8 | c0` (0x6FDD09-0x6FDD25: `or eax, 0xFFFF8000` then two `shl 8`), i.e. bytes
        c0, c1, c2, 0x80 in memory (RGBA order with alpha 128 when the file lists R G B).
     f. `line = LoadLine(fd)`.
  3. `CloseFile(fd)`; return 1 (0x6FDD40-0x6FDD56).
- Edge cases / original bugs: (1) a line with fewer than three tokens returns 0 and leaks the open file handle (0x6FDD07).
  (2) an empty line (LoadLine returns "" after trimming, or a line of only blanks) makes strtok return NULL and `atol(NULL)`
  dereferences NULL (0x82258E -> 0x822506 reads [esi]) -> crash. (3) c0 is held in BL, seeded from an uninitialised stack byte
  (0x6FDC45), but every stored entry needs n >= 3 so all three are always written first. No bound on the number of entries.
- gta-reversed: `plugin::CallAndReturn` only (CustomCarPlateMgr.cpp:97), hook `.Reversed = false`. Open questions: none.
