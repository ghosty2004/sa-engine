# CStreaming: gta-reversed deviation notes (STREAM)

Source: `re/gta-reversed/source/game_sa/Streaming.cpp` (pin 20d8d2a). CStreamingInfo stride 0x14 at
ms_aInfoForModel (0x8E4CC0): +0 next i16, +2 prev i16, +4 nextOnCd i16, +6 flags u8, +7 imgId u8,
+8 cdPosn u32, +0xC cdSize u32, +0x10 loadState u8 (1 loaded, 3 reading, 4 finishing).

The RNG helper used by the original is CGeneral::GetRandomNumberInRange(int) at 0x407180:
`min + _ftol((float)(rand() & 0xFFFF) * (1/32768.f) [0x858B14] * (int)(max - min) [fimul])`
(x87, truncation). gr's helper is `(T)lerp(min, max - 1, rand() / 32767.f)` and gives different results
(docs/re/cat14-gr-deviations.md finding 1). Several sites below inline the same formula with an f32
constant instead of `fimul`; the value is the same.

## 0x407D50 CStreaming::GetDefaultCabDriverModel  (gr_deviations = 1)
Row Streaming.cpp:2729 (rng-helper `GetRandomNumberInRange(0, 2)`).
- Original bytes: if `loadState(ms_aDefaultCabDriverModel[s_LastRandomIndex])` (byte 0x8E4CD0 + model*0x14)
  is non-zero, return that model. Else if `CTheZones::m_CurrLevel (0xBA6718) != 0`:
  `r = _ftol((rand() & 0xFFFF) * (1/32768.f) [0x858B14] * -2.0f [0x858B18])` (0x407D79-0x407D97; r is 0 or
  -1), `s_LastRandomIndex (0x965524) = 2*level - r - 2` (0x407D9C-0x407DA4). Return
  `ms_aDefaultCabDriverModel[s_LastRandomIndex]` (table 0x8A5AF4). So the index offset is 1 exactly when
  `rand() >= 16384`.
- Verdict: CONFIRMED (gr: offset 1 only when rand() == 32767).
- Class: keep-original.

## 0x4084B0 CStreaming::Shutdown  (gr_deviations = 2)
Rows Streaming.cpp:3061 (original-comment) and :3069 (FIX_BUGS `ms_pExtraObjectsDir = nullptr`).
- Original bytes (.HOODLUM 0x1563AB0): `CMemoryMgr::FreeAlign(ms_pStreamingBuffer[0])` only (0x1563AB7),
  pointer not cleared; `ms_streamingBufferSize = 0`; if `ms_pExtraObjectsDir` non-null: CDirectory
  destructor + `free` (0x1563AD5-0x1563ADB). `ms_pExtraObjectsDir` (0x8E48D0) is **not** set to 0.
- Verdict :3061: REFUTED (gr frees only buffer 0 too; the alternatives are commented out).
- Verdict :3069: CONFIRMED (gr nulls the pointer, the original leaves it dangling).
- Class: keep-original for both (leave both pointers unchanged).

## 0x408E20 CStreaming::GetNextFileOnCd  (gr_deviations = 1)
Row Streaming.cpp:386 (`IsFixBugs() ? pos.Offset : pos.ToInt()`, CD_STREAM_READ_POS_FIX).
- Original bytes: `offset = ms_files[info.imgId].StreamHandle + info.cdPosn` (0x408FC5-0x408FE0: dword at
  0x8E4904 + imgId*0x30, i.e. tStreamingFileDesc +0x2C at ms_files 0x8E48D8, plus [info+8]); the handle is
  `FileID << 24`, so this is the full position, not the 24-bit offset. Compares are unsigned:
  `offset < firstOffset` -> first (0x408FE3), `offset < nextOffset && offset >= streamLastPosn` -> next
  (0x408FED-0x408FF7).
- Verdict: CONFIRMED (gr uses the 24-bit offset).
- Class: keep-original.

Found in passing (CONFIRMED, keep-original): the end of the function differs. Original (0x40900F-0x409043):
`if next != -1 return next; if first != -1 return first; if ms_numPriorityRequests == 0 return -1;`
otherwise `ms_numPriorityRequests = 0` and a **tail call to GetNextFileOnCd(streamLastPosn, false)**
(`mov [esp+8], 0; jmp 0x408E20`). gr returns MODEL_INVALID instead of retrying, so non-priority requests
are not considered in that call.

## 0x5BCCD0 CStreaming::ReadIniFile  (gr_deviations = 1)
Row Streaming.cpp:2031 ("atoi, atof are replaced by safe analogues").
- Original bytes: integers via `atol` (0x82258E) and floats via `atof` (0x823CEE) followed by
  `fstp dword` (double result rounded to f32, e.g. 0x5BCDDC); tokens via the CRT tokenizer at 0x82244B
  (two calls per line, delimiters string 0x86A8C8).
- Verdict: CONFIRMED (gr uses strtol / strtof: strtof rounds decimal -> f32 directly, atof rounds
  decimal -> f64 -> f32, which can differ by one ulp in rare cases; atol overflow behaviour of the
  2002 CRT is not the C99 clamp).
- Class: keep-original (parse as f64 then round to f32).

Found in passing (CONFIRMED, keep-original): `pe_leftx`, `pe_rightx`, `pe_topy`, `pe_bottomy` store the
**int32** atol result (`mov dword [0x8D5150/54/58/5C], eax`, 0x5BCE45-0x5BCEA8; the .data initial values
are the integer 8). gr declares these four globals as float (PostEffects.h:94, `StaticRef<float>`) and
stores `(float)strtol(...)`. The port must keep them int32. The memory/devkit_memory branches store
`atol(value) << 10` (0x5BCD4D, 0x5BCD75).

## 0x5B6170 CStreaming::LoadCdDirectory(const char*, int)  (gr_deviations = 3)
Rows Streaming.cpp:1111 (NOTSA assert "VER2"), :1150 (FIX_BUGS "remember which cdimage"), :1162
(`IsFixBugs() ? FindColSlot(name) : FindColSlot()`), :1197 (assert).
- :1111 Original: the 4 version bytes are read (0x5B61AB) and never checked. Verdict REFUTED (debug-only
  assert). Class harmless.
- :1150 Original (0x5B625E-0x5B627A): for an unknown DFF, `entry.pos |= img << 24` (OR into the whole
  32-bit position), then `ms_pExtraObjectsDir->AddItem(entry)`. gr's `AddItem(entry, img)` sets FileID =
  img, which is the same for positions below 2^24. Verdict REFUTED. Class harmless (the original also
  remembers the image). Found in passing: gr's `CDirectory::AddItem` has a FIX_BUGS duplicate check
  (Directory.cpp:52); the original 0x532310 appends unconditionally (that belongs to 0x532310's own notes).
- :1162 Original: calls 0x410390, which is `or eax, -1; ret` (always -1), so every .col entry calls
  `CColStore::AddColSlot(name)` (0x5B630B) and gets a new slot. Verdict CONFIRMED (gr looks the name up).
  Class keep-original.
- :1197 Original stores the model id as a word without a check (0x5B64A2). Verdict REFUTED (assert).
  Class harmless.

Found in passing (notes): the entry loop is `do { --count; ... } while (count != 0)` after a `count == 0`
test (0x5B61C4, 0x5B64B5), so a negative count would loop ~2^32 times (gr: `for i < count`). The buffer
size max uses a signed compare of the u16 size (0x5B61F4). The extension test is `ext - name > 20` ->
skip (0x5B6222). SetCdPosnAndSize gets the raw 32-bit position dword (0x5B647A), imgId is written before it
(0x5B6468), flags cleared after it (0x5B6495).

## 0x40A150 CStreaming::StreamCopModels  (gr_deviations = 1)
Rows Streaming.cpp:3120 (rng-helper `GetRandomNumberInRange(30000, 50000)`) and :3142 (assert).
- Original bytes (.HOODLUM, continuation at 0x156C146 after an obfuscated call to FindPlayerWanted):
  if `wanted->level (+0x2C) < 3 && level != 0 && !m_bDisableCopBikes (0x9654BF)`: if
  `ms_nTimePassedSinceLastCopBikeStreamedIn (0x9654C0) < CTimer::m_snTimeInMilliseconds (0xB7CB84)`
  (unsigned), toggle `m_bCopBikeLoaded (0x9654BE)` and set the time to
  `now + GetRandomNumberInRange(30000, 50000)` via 0x407180 (0x156C183-0x156C19C), i.e.
  `now + 30000 + _ftol((rand() & 0xFFFF) * (1/32768.f) * 20000)`.
- Verdict :3120: CONFIRMED (helper formula). Class keep-original.
- Verdict :3142: REFUTED (assert only). Class harmless.

## 0x40BBB0 CStreaming::StreamPedsForInterior  (gr_deviations = 3)
Rows Streaming.cpp:3271 (rng-helper, wife), :3306 (rng-helper, office), :3317 (NOTSA_UNREACHABLE).
Group tables: group id per zone at 0x8D26B4 (husbands), 0x8D26C0 (wives), 0x8D269C (shopkeepers), 0x8D26A8
(office workers), indexed by CPopulation::CurrentWorldZone (0xC0FCBC); counts at
CPopulation::m_nNumPedsInGroup (0xC0ECC0, i16, indexed by the zone-0 group id); models at
CPopulation::m_PedGroups (0xC0F358, u16, group*21 + index).
- Case 0 (0x40BCBE): `husband = rand() % nHusbands` (signed idiv, rand not masked);
  `r = _ftol((rand() & 0xFFFF) * (1/32768.f) * 6.0f [0x858B44])` (0..5);
  `wife = husband - r + 3`; `if wife < 0: wife = 0` (sets/dec/and at 0x40BD10-0x40BD19);
  `if nWives - 1 < wife: wife = nWives - 1`; ClearSlots(2); request husband (flag 8), ms_pedsLoaded[0],
  ++ms_numPedsLoaded; request wife, ms_pedsLoaded[1], ++ms_numPedsLoaded.
  This equals `husband - GetRandomNumberInRange(-3, 3)` with the original helper. gr uses
  `husband - GetRandomNumberInRange(3, 9)`, a different range **and** the lerp formula.
  Verdict :3271: CONFIRMED. Class keep-original.
- Case 2 (0x40BBCF): ClearSlots(8); the office group id is read through an obfuscated pointer at
  0x4084B5 (it decodes to 0x8D26A8); `n = m_nNumPedsInGroup[group0]`;
  `r = _ftol((rand() & 0xFFFF) * (1/32768.f) * n)` (fimul, 0x40BBEA-0x40BC06); for i = 0..7:
  `model = m_PedGroups[groupId(zone)*21 + (i + r) % n]` (n re-read each iteration, signed idiv),
  RequestModel(model, 8), ms_pedsLoaded[i] = model, ++ms_numPedsLoaded.
  Verdict :3306: CONFIRMED (helper formula). Class keep-original.
- Default (interiorType not 0/1/2): `jne 0x40BD8E` returns without doing anything.
  Verdict :3317: CONFIRMED. Class keep-original (no-op).

## 0x40D7C0 CStreaming::DeleteRwObjectsBehindCamera  (gr_deviations = 1)
Row Streaming.cpp:892 (bug-note "possibly missing AdvanceCurrentScanCode").
- Original bytes: five inlined scan-code advances (0x40D909, 0x40DA91, 0x40DBB0, 0x40DD56, 0x40DEDD);
  the final sector pass after RemoveReferencedTxds (0x40E001) runs without one (0x40E069-0x40E0CA).
- Verdict: REFUTED (gr also has no advance there; the comment only flags it).
- Class: keep-original (no advance before the last pass).

## 0x40C6B0 CStreaming::ConvertBufferToObject  (gr_deviations = 2)
Rows Streaming.cpp:436 (TODO naming), :437 and :478 ("stream seemingly never closed"), :447 ("todo: add
FIX_BUGS"), :618 (NOTSA `assert(0)` default).
- :436 Verdict REFUTED (comment only). Class harmless.
- :437/:478 Original: `_rwStreamInitialize(&gRwStream, 0, 3, 1, &initData)` at 0x40C6E8; atomic path:
  RwStreamReadChunkHeaderInfo, optional UV-anim dict, `RwStreamClose` (0x40C7CA), a second
  `_rwStreamInitialize` (0x40C7DF) and LoadAtomicFile on it. gr does the same. Verdict REFUTED for both.
  Class harmless.
- :447 Original (0x40C70D-0x40C733): `txdDef = GetAt(mi->txdIndex)` (0 for a free slot), then
  `cmp dword [txdDef], 0; je fail` without a null check, so a free TXD slot dereferences address 0. gr
  tests `txdDef && !txdDef->m_pRwDictionary`, i.e. it skips the failure path for a free slot. Then
  `if animIndex != -1 && !animBlocks[animIndex].loaded (byte 0xB5D4B0 + idx*0x20)` -> fail. Fail path
  (0x40C999): RemoveModel(id), `RequestModel(id, info.flags)` (flags read after RemoveModel), RwStreamClose,
  return 0. Verdict CONFIRMED. Class keep-original (no null check; unreachable with consistent data).
- :618 Original: every model id maps to a handled type; there is no default action. Verdict REFUTED
  (debug-only assert). Class harmless.

Found in passing (note): in the clump path the original calls `mi->vtbl[0x10]` (GetModelType) and
discards the result before LoadClumpFile (0x40C805-0x40C80A); harmless. The TXD branch reads the parent
index from `GetAt(txdSlot)+6` also without a null check (0x40C8A8-0x40C8CE).

## 0x4076C0 CStreaming::RetryLoadFile  (gr_deviations = 1)
Row Streaming.cpp:2574 (NOTSA `NOTSA_LOG_DEBUG(...)`).
- Original bytes: the first instruction after `push esi` is `call CPad::StopPadsShaking` (0x4076C1). gr
  replaced it with a debug log and never stops the pad shaking.
- Verdict: CONFIRMED.
- Class: keep-original (call CPad::StopPadsShaking() first).

Found in passing (CONFIRMED, keep-original): the ERR state (3) logic is inverted in gr. Original
(0x40770B-0x407736): `++totalTries; s = CdStreamGetStatus(ch)`; if `s == 0xFF (READING)`: `s2 =
CdStreamGetStatus(ch)`; if `s2 == 0xFF` -> do nothing this round, else -> re-read. If `s == 0xFA
(WAITING)` -> do nothing. Any other s -> re-read (`CdStreamRead(ch, ms_pStreamingBuffer[ch], pos,
sectorCount)`, state = 1, loadingLevel (+4) = -600). gr's condition skips when the second status is
**not** READING and re-reads when it is. The number of CdStreamGetStatus calls (which reset a failure
status) is the same. State values: 0 idle -> read, 1 reading -> ProcessLoadingChannel, 2 -> nothing,
3 error as above; after each pass the loop exits through CLoadingScreen::Continue when ms_channelError == -1.
