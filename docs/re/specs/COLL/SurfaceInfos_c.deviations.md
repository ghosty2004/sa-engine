# SurfaceInfos_c: gta-reversed deviation notes (COLL)

Rows from `catalog/gr-deviations.csv`, checked against the original bytes (RE-SPEC-1, writer B). The two REA
loaders (0x55eb90, 0x55f2b0) have full specs in `SurfaceInfos_c.md`; their rows are resolved here too.

## 0x55d0e0 SurfaceInfos_c::LoadAdhesiveLimits  (gr_deviations = 1)
- SurfaceInfos_c.cpp:258 `#if FIX_BUGS` block (return early when `surface.dat` cannot be opened)
  - Original bytes: 0x55d100 `call CFileMgr::OpenFile` -> 0x55d105 `mov ebx, eax` -> 0x55d109 `push ebx` ->
    0x55d112 `call CFileLoader::LoadLine` with no null test in between.
  - Verdict: CONFIRMED (the original has no check; a missing file goes straight into LoadLine). Port keeps the
    original (no early return).
  - Class: keep-original.

## 0x55eb90 SurfaceInfos_c::LoadSurfaceInfos  (gr_deviations = 2)
- SurfaceInfos_c.cpp:304 `#if FIX_BUGS` block (null-file check)
  - Original bytes: 0x55eba4 OpenFile -> 0x55ebab `push esi` -> 0x55ebb0 LoadLine; no test.
  - Verdict: CONFIRMED. Class: keep-original.
- SurfaceInfos_c.cpp:329 "BUG:? Data has negative values, with *original* conversion we loose sign"
  (`surface.wetGrip = uint32(si.WetGrip * 100.0f)`)
  - Original bytes: 0x55ee2d `fld [WetGrip]`, `fmul [0x858628] = 100.0f`, `call _ftol 0x821b40` (truncation
    toward zero to a signed 32-bit int), 0x55ee56 stores the low byte AL into entry+1 (`int8 wetGrip`).
    -25.0 becomes byte 0xE7, i.e. -25 when read back as int8: the sign is NOT lost.
  - Verdict: CONFIRMED. gr's `uint32(negative float)` is undefined behaviour in C++ and is a different operation
    from `_ftol`; the port must do `(trunc(x) as i32) as u8` (low byte of the truncated signed value).
  - Class: keep-original.
- Other differences (not rows): skid-mark values and 1-bit flag conversion, see the full spec
  (`SANDY` stores 2 and `MUDDY` stores 1, gr stores 1 and 2; original keeps only bit 0 of each flag column, gr uses
  `(bool)value`).

## 0x55f2b0 SurfaceInfos_c::LoadSurfaceAudioInfos  (gr_deviations = 1)
- SurfaceInfos_c.cpp:381 `#if FIX_BUGS` block (null-file check)
  - Original bytes: 0x55f2c1 OpenFile -> 0x55f2c6 `mov edi, eax` -> 0x55f2c9 LoadLine; no test.
  - Verdict: CONFIRMED. Class: keep-original.
