# CFileLoader: gta-reversed deviation notes (STREAM)

Source: `re/gta-reversed/source/game_sa/FileLoader.cpp` (pin 20d8d2a). CCollisionData layout (gr
CollisionData.h, size 0x30): +0 numSpheres u16, +2 numBoxes u16, +4 numTriangles u16, +6 numLines u8,
+7 flags (bit0 usesDisks, bit1 hasFaceGroups, bit2 hasShadowInfo), +8 spheres, +0xC boxes, +0x10 lines,
+0x14 vertices, +0x18 triangles, +0x1C trianglePlanes, +0x20 numShadowTriangles, +0x24 numShadowVertices,
+0x28 shadowVertices, +0x2C shadowTriangles. CColModel: +0x28 colSlot, +0x29 flags (bit0 hasCollisionVolumes,
bit1 isSingleColDataAlloc), +0x2C colData.

## 0x536FE0 CFileLoader::LoadLine(char*&, int&)  (gr_deviations = 1)
Rows FileLoader.cpp:250 (bugfix-comment) and :251 ("LoadCutSceneFile relies on it filtering `\r`").
- Original bytes 0x537000-0x537022: for each char until '\n', NUL or the size runs out: chars `< 0x20`
  (unsigned, includes '\r') and ',' are written as ' ', others copied; `size` is decremented per char and
  the loop continues while `size > 0` (signed). '\r' is **copied as a space**, not skipped. Then
  `bufferIt` is advanced one past the terminator, the copy is NUL-terminated and the result skips leading
  bytes `<= 0x20` (0x537040-0x53704B).
- Verdict: CONFIRMED for both rows (gr drops '\r', so a CRLF line has no trailing space in gr).
- Class: keep-original. Consumers that compare whole lines (gr mentions LoadCutSceneFile) must be ported
  against this behaviour (trailing ' ' on CRLF files).

## 0x537580 CFileLoader::LoadCollisionModel (V1)  (gr_deviations = 1)
Rows FileLoader.cpp:637 (`assert(cd)`) and :672 (vertex loop unrolled by 4).
- Original bytes: vertices 0x537735-0x5378C9, a 4-per-iteration block while the block's last index < n,
  then a remainder loop; every component is `(int16)_ftol(f * 128.0f [0x858BF4])` (truncation), in x, y, z
  order. Same values as gr's 1-per-iteration loop.
- Verdict :637 REFUTED (assert), :672 REFUTED (unroll only).
- Class: harmless.
- Found in passing (note): the triangle count is read as a 16-bit word, stored to +4 and tested signed
  (`mov dx, word [esi]; ... jle`, 0x5378D2-0x5378E2) while the input pointer advances 4 bytes; gr reads a
  uint32. Same for counts < 32768.

## 0x537EE0 CFileLoader::LoadCollisionModelVer2  (gr_deviations = 1)
Rows :719 (bug-note on `assert(h.IsEmpty())`), :729 (`assert(p)`), :730 ("R* used a cast here"),
:776 (assert).
- :719/:729/:776 Verdict REFUTED (debug-only asserts, the original has no checks). Class harmless.
- :730 Original: `p = CMemoryMgr::Malloc(size - 0x4C + 0x30)` (0x537F5A), `colModel->colData = p`
  without constructing CCollisionData, copies the data to p+0x30, then writes +0, +2, +6, +4, flags
  bits 0/2 cleared and bit1 = face-group flag, pointers +8..+0x18 (null when the file offset is 0, else
  `offset + p - 0x38`), +0x1C = 0, +0x28 = 0, +0x2C = 0, +0x24 = 0 (0x537F80-0x538075). **+0x20
  (numShadowTriangles) is never written** and keeps the Malloc contents; flag bits 3..7 also keep them.
  gr's placement-new plus explicit `m_nNumShadowTriangles = 0` zeroes +0x20.
- Verdict :730: CONFIRMED (one field differs).
- Class: harmless if every reader of +0x20 is gated by bit2 (hasShadowInfo, always 0 here).
  Open question: readers of CCollisionData+0x20 were not audited (first address: 0x537F80 region).

## 0x537CE0 CFileLoader::LoadCollisionModelVer3  (gr_deviations = 1)
Rows :792 (bug-note assert), :802 (assert), :803 ("R* used a cast"), :848 (assert).
- Original bytes: Malloc + raw cast as in V2, but every CCollisionData field is written: +0x20 from the
  header (0x537DC9), shadow pointers +0x28/+0x2C, flag bit2 set when
  `offShdwFaces != 0 && offShdwVerts != 0 && nShdwFaces > 0` (signed `jle`, 0x537E87-0x537E99), +0x24 =
  GetNoOfShdwVerts (0x537510) or 0, +0x1C = 0 (0x537ECB).
- Verdict: REFUTED for all four rows (placement new changes nothing observable; asserts debug-only).
- Class: harmless. Note: gr tests `nShdwFaces` for non-zero, the original for > 0 (same for valid data).

## 0x537AE0 CFileLoader::LoadCollisionModelVer4  (gr_deviations = 1)
Rows :864 (assert), :874 (assert), :875 ("R* used a cast"), :920 (assert).
- Original bytes: same structure as V3 (0x537B55 Malloc, +0x20 at 0x537BC9, flag/shadow logic
  0x537C87-0x537CCB).
- Verdict: REFUTED for all four rows.
- Class: harmless.

## 0x5B4C80 CFileLoader::LoadOcclusionVolume  (gr_deviations = 1)
Row FileLoader.cpp:1424 ("BUGFIX: Using %u instead of %d").
- Original bytes: format string 0x868D74 is `"%f %f %f %f %f %f %f %f %f %d "`; rotY, rotX and flags are
  zeroed before the sscanf (0x5B4CC0-0x5B4CD0). `z = height * 0.5f [0x858B8C] + bottomZ` (x87, 0x5B4CDD-
  0x5B4CFC).
- Verdict: CONFIRMED (format differs) but the parsed bits are the same for every value in the data files.
- Class: harmless (use `%d` semantics if exactness matters).
- Found in passing (keep-original): the interior test only checks `filename[len-7..len-5] == "int"`
  (0x5B4D09-0x5B4D1D); gr checks `ends_with("int.ipl")`. Equal for `*.ipl` names; the original reads before
  the string for names shorter than 7 chars.

## 0x538440 CFileLoader::LoadCollisionFile(uint8*, uint32, uint8)  (gr_deviations = 1)
Rows :493 (assert on buffer size) and :511 (NOTSA assert `fileTotalSize <= remaining`).
- Verdict: REFUTED for both (debug-only).
- Class: harmless.
- Found in passing (notes): the loop runs while `remaining > 8` (unsigned, 0x538461 and 0x538650) and stops
  at an unknown fourcc (0x53849D/0x5384C6); gr stops when `remaining < 8`, so with exactly 8 bytes left gr
  reads one more fourcc. The model-info register (EBX) is not cleared per entry: for a model id >= 20000
  the previous entry's mi is reused for the key test (0x5384F4-0x53851C); the name lookup then gives the
  same result, so this is harmless. Versions dispatch through a 4-entry jump table (COLL, COL2, COL3,
  COL4).

## 0x5B4E60 CFileLoader::LoadCollisionFile(const char*, uint8)  (gr_deviations = 1)
Row FileLoader.cpp:553 (assert: data fits the 32 KB buffer).
- Original bytes: no size check; reads `size - 0x18` bytes into 0xBC40D8 (0x5B4EF4).
- Verdict: REFUTED (debug-only).
- Class: keep-original (unchecked read).
- Found in passing (CONFIRMED, keep-original): the version is chosen only for COLL (0), COL2 (1), COL3 (2)
  (0x5B4EB0-0x5B4ED4); any other fourcc, including COL4, leaves the version register EBX unchanged, i.e.
  the previous entry's version (first entry: an uninitialised stack value loaded at 0x5B4EA3), and the
  data is parsed with that loader. gr's LoadCollisionModelAnyVersion parses COL4 with the V4 loader and
  ignores unknown versions. There is no V4 call in this function.

## 0x5B3DE0 CFileLoader::LoadTimeObject  (gr_deviations = 1)
Row FileLoader.cpp:1777 (`NOTSA_UNREACHABLE()` for numObjs not 1..3).
- Original bytes: `switch (numObjs)` at 0x5B3E62; any value other than 1, 2, 3 jumps to 0x5B3F2C and
  continues with the values from the first sscanf (no extra parse).
- Verdict: CONFIRMED.
- Class: keep-original.
- Note: the original does not initialise modelId / drawDistance / flags / times (gr does); they keep stack
  contents when a sscanf does not write them.

## 0x5B6F30 CFileLoader::LoadVehicleObject  (gr_deviations = 1)
Rows :1811 (wheel sizes default 0.7f), :1845/:1846 and :1899/:1900 (`notsa::find_value` mappings),
:1854 (f_heli).
- :1811 Original: only two locals are initialised to -1 before the sscanf (0x5B6F41, 0x5B6F45: misc and
  wheelUpgradeClass); the two wheel-size floats are not. For car/mtruck/quad/heli/plane/trailer the
  original stores `+0x40 = [frame+0x14]`, `+0x44 = [frame+0x10]` (fld order at 0x5B7086/0x5B708F, stores
  0x5B7267/0x5B726A), bike/bmx likewise (0x5B71F7/0x5B7223), so a line without wheel sizes uses whatever
  was on the stack. gr uses 0.7f.
  Verdict CONFIRMED. Class keep-original, but the value is not reproducible from the bytes alone.
  Open question (0x5B7086): what those stack slots hold for such lines in the shipped data (likely the
  previous LoadVehicleObject call's values, since the frame is reused from LoadObjectTypes); needs a
  runtime measurement.
- :1854 Original: `"f_heli"` maps to type 3 (HELI) (0x5B71A3). gr maps it to FHELI.
  Verdict CONFIRMED. Class keep-original.
- :1845/:1846 (type mapping) Original: inline `repe cmpsb` chain (exact, case-sensitive, includes the NUL)
  over car, mtruck, quad, heli, plane, boat, train, f_heli, f_plane, bike, bmx, trailer. No match: the
  type field (+0x3C) is not written and no wheel fields are written; execution continues at 0x5B726D.
  gr aborts (`NOTSA_UNREACHABLE` inside find_value).
  Verdict CONFIRMED (miss case only; matched keys are equivalent). Class keep-original.
- :1899/:1900 (class mapping) Original: chain normal..bicycle sets the class byte (+0x4D) and then
  `frq (+0x52) = frq`; "ignore" sets -1 and returns without frq (0x5B7402); no match leaves +0x4D
  unchanged and still writes frq (0x5B73FA -> 0x5B729E). gr aborts on a miss.
  Verdict CONFIRMED (miss case only). Class keep-original.
- Note: boat, train and f_heli write no wheel fields; f_plane writes 1.0f to both and the wheel model
  index; bike/bmx write `m_fBikeSteerAngle (+0x58) = (float)misc` (fild) and no wheel model index; the
  function returns the parsed modelId.

## 0x5B8400 CFileLoader::LoadObjectTypes  (gr_deviations = 4)
Rows :2183 ("add original min/max check"), :2192 (NOTSA section NONE), :2253 ("R* does something weird
with the object IDs"), :2256 (path section leftover).
- Original bytes: min/max of the returned object ids are tracked in two locals (init -1 and 0x7FFFFFF,
  0x5B841C/0x5B8420; updated at 0x5B85EA-0x5B85FA) and never used. Section 0 = none (EDI = 0). The path
  section matches gr (LoadPathHeader / LoadPedPathNode / LoadCarPathNode with false/true, counter reset
  after 12, 0x5B865B-0x5B86B1). The filename is copied into an unused local at entry (0x5B843A).
- Verdict: REFUTED for all four rows.
- Class: harmless.

## 0x5371F0 CFileLoader::LoadAtomicFile(RwStream*, uint32)  (gr_deviations = 1)
Row FileLoader.cpp:172 (FIX_BUGS V1004: `mi && !mi->GetRwObject()`).
- Original bytes: `mi = ms_modelInfoPtrs[id]->vtbl[1]()` (AsAtomicModelInfoPtr); after the clump read the
  original tests `mi->rwObject (+0x1C)` with no null check (0x53725A); a null mi dereferences address 0x1C.
  On rwObject == 0 it returns 0 **without** StopUsingCommonVehicleTexDicationary (0x53725F -> 0x53727A).
- Verdict: CONFIRMED (gr returns true for a null mi).
- Class: keep-original (unreachable with consistent data).

## 0x5B8700 CFileLoader::LoadScene  (gr_deviations = 1)
Row FileLoader.cpp:2034 (NOTSA section NONE placeholder).
- Original bytes: the section register ESI starts at 0 (0x5B870E) and 0 means "no section".
- Verdict: REFUTED.
- Class: harmless.

## 0x5B9030 CFileLoader::LoadLevel  (gr_deviations = 5)
Rows :1304 / :1312 (path copied per id), :1358 ("probably inlined" ConvertAnimFileIndex loop), :1368
(IPL path copy), :1373 (dispatch table instead of else-if).
- Original bytes: TEXDICTION and IPL copy `line + 11` / `line + 4` into a stack buffer (0x5B90B5,
  0x5B9234); the other ids pass `line + strlen(id) + 1` directly; their callees open the file before any
  LoadLine, and LoadObjectTypes copies its argument at entry, so the missing copy has no effect.
  The model-info loop calls `vtbl[0x34]` for every non-null entry of ms_modelInfoPtrs up to 0xAAE948
  (0x5B9299-0x5B92B4). The else-if order is EXIT, TEXDICTION, IMG, COLFILE, MODELFILE, HIERFILE, IDE,
  IPL, SPLASH (strncmp lengths 4, 10, 3, 7, 9, 8, 3, 3, 6), with no overlapping prefixes.
- Verdict: REFUTED for all five rows.
- Class: harmless.
- Found in passing (CONFIRMED, keep-original): (1) the IMG branch (0x5B913F-0x5B9163) does **not** call
  LoadingScreenLoadingFile; gr calls it for every table entry including IMG. It compares the path with
  "MODELS\\GTA_INT.IMG" (19 bytes incl. NUL) and otherwise calls `AddImageToList(path, true)`.
  (2) COLFILE uses `line + 10` (skips "COLFILE 0 ") for both LoadingScreenLoadingFile and
  LoadCollisionFile(path, 0) (0x5B917C-0x5B9188); gr passes "0 <path>" to LoadingScreenLoadingFile (same
  displayed basename when the path contains a backslash). (3) SPLASH is compared and ignored. (4)
  TEXDICTION inlines LoadingScreenLoadingFile (0x5B90CA-0x5B9100, same as 0x5B3680).
