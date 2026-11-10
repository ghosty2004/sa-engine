# CAEMP3TrackLoader (AUDIO) - gta-reversed deviation notes (RE-SPEC-3, group U1)

Source: `re/gta-reversed/source/game_sa/Audio/Loaders/AEMP3TrackLoader.h/.cpp` (pin 20d8d2a). Layout: +0 u8 m_bInitialised,
+4 u32 m_nTrackCount, +8 u32 m_nStreamPackCount, +0xC tTrackLookup* (12 bytes each), +0x10 StreamPack* (16 bytes each, name
first), +0x14 u8 m_bStreamingFromDVD, +0x18 char* m_pszDvdDrivePath. Strings: 0x85F184 "AUDIO\\CONFIG\\STRMPAKS.DAT",
0x85F1A0 "AUDIO\\CONFIG\\TRAKLKUP.DAT", 0x85F1BC "AUDIO\\STREAMS\\", 0x85A53C "r".

## 0x4E0930 CAEMP3TrackLoader::Constructor  (gr_deviations = 1)
Original: `__thiscall CAEMP3TrackLoader* (this)`: `+0 = 0` (u8), `+0x18 = 0`, return this (0x4E0932-0x4E0935).
Row AEMP3TrackLoader.h:20 (NOTSA wrapper). Verdict: REFUTED (gr ctor sets the same two fields). Class: harmless.

## 0x4E0970 CAEMP3TrackLoader::LoadStreamPackTable  (gr_deviations = 3)
Original: `__thiscall bool (this)`, plain `ret`.
- Original bytes: `h = CreateFileA("AUDIO\\CONFIG\\STRMPAKS.DAT", GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL)`;
  if `h == -1` return 0 (fields untouched). `size = GetFileSize(h, NULL)`; `+0x10 = CMemoryMgr::Malloc(size, 0)` 0x72F420;
  `ReadFile(h, +0x10, size, &read, NULL)`; `CloseHandle(h)`; `+8 = read >> 4` (count from BYTES READ, 0x4E09D2-0x4E09D9);
  return 1.
Row :62 (notsa::File instead of Win32): CONFIRMED, harmless for an existing file (same bytes).
Row :65 (`notsa::File tmp(..., "w")` on failure): CONFIRMED - gr CREATES an empty STRMPAKS.DAT when it is missing and nulls
+0x10 / +8; the original only returns 0. Class: keep-original (no file creation, no field writes).
Row :66 (comment "Originally CMemoryMgr::Malloc(0)"): the original does not allocate on the failure path at all
(0x4E0994-0x4E099B). Verdict: CONFIRMED (gr comment inaccurate). Class: keep-original. Also: gr computes the count from the
file size, the original from the read byte count (differs only on short reads).

## 0x4E09F0 CAEMP3TrackLoader::LoadTrackLookupTable  (gr_deviations = 1)
- Original bytes: same Win32 pattern with "AUDIO\\CONFIG\\TRAKLKUP.DAT"; `+0xC = Malloc(size, 0)`; ReadFile; `+4 = size / 12`
  (from the FILE SIZE: `mul 0xAAAAAAAB; shr edx,3`, 0x4E0A46-0x4E0A51); CloseHandle; return 1. On open failure return 0 without
  touching fields.
Row :84 (notsa::File): Verdict: CONFIRMED (gr also nulls +0xC/+4 on failure; the original does not). Class: harmless/keep-original.

## 0x4E0BD0 CAEMP3TrackLoader::IsCurrentAudioStreamAvailable  (gr_deviations = 1)
Original: `__thiscall bool (this)`, stack buffer 0x104.
- Original bytes: `+0x18 = getDvdGamePath()` 0x747300; if null -> `+0x14 = 0`, return 0. Else `buf = lstrcpyA(dvd)`,
  `lstrcatA(buf, "AUDIO\\STREAMS\\")`, `lstrcatA(buf, StreamPacks[0].name (+0x10 ptr))` (no separator added); `f = CFileMgr::
  OpenFile(buf, "r")` 0x538900; if `f > 0` (signed, 0x4E0C24): `+0x14 = 1`, CloseFile(f), return 1; else `+0x14 = 0`, return 0.
  m_bInitialised (+0) is NOT written.
Row :146 (notsa::File with `std::format("{}\\AUDIO\\STREAMS\\{}")`): CONFIRMED - gr inserts an extra backslash, opens with
fopen instead of CFileMgr::OpenFile (which resolves relative to CFileMgr's current directory), calls getDvdGamePath twice and
also sets `m_bInitialised = true`. Class: keep-original.

## 0x4E0C50 CAEMP3TrackLoader::Initialise  (gr_deviations = 2)
- Original bytes: if `!LoadStreamPackTable() || !LoadTrackLookupTable()` return 0. `+0x14 = 0`; `buf = operator new(lstrlenA(
  StreamPacks[0].name) + 0x2F)` 0x82119A; `lstrcpyA(buf, "AUDIO\\STREAMS\\")`; `lstrcatA(buf, name)`; `f = CFileMgr::OpenFile(buf,
  "r")`; if `f > 0`: `+0x14 = 0`, go to success. Else `+0x18 = getDvdGamePath()`; if null -> `_free(buf)`, return 0; else
  `lstrcpyA(buf, dvd); lstrcatA(buf, "AUDIO\\STREAMS\\"); lstrcatA(buf, name)`; `f = OpenFile(buf, "r")`; if `f <= 0` ->
  `_free(buf)`, return 0; else `+0x14 = 1`. Success: `CloseFile(f)`, `_free(buf)`, `+0 = 1`, return 1.
Row :46 (NOTSA: heap string replaced by std::format): CONFIRMED. ORIGINAL BUG: the heap buffer is sized for
"AUDIO\\STREAMS\\" + name + 32 spare bytes (`len + 0x2F`), but the DVD branch writes `dvdPath + "AUDIO\\STREAMS\\" + name`
into it - a DVD path longer than 32 characters overflows the heap block (0x4E0CC9-0x4E0CDA). Class: keep-original semantics
(the port should keep the path composition; overflow itself cannot be reproduced meaningfully - note in the ledger).
Row :49 (notsa::File local check): CONFIRMED - original uses CFileMgr::OpenFile (current-dir relative) and handles the DVD
path inline (it does not call IsCurrentAudioStreamAvailable). Class: keep-original.
