# CIplStore: gta-reversed deviation notes (STREAM)

Source: `re/gta-reversed/source/game_sa/IplStore.cpp` (pin 20d8d2a). The evidence is the 1.0 US bytes
(addresses below). IplDef layout (gr `IplDef.h`, size 0x34): +0x00 CRect bb (left, top?, right, bottom? as
used: flipped when [+0] > [+8] or [+0xC] > [+4]), +0x10 name[16], +0x22 firstBuilding i16, +0x24 lastBuilding
i16, +0x26 firstDummy i16, +0x28 lastDummy i16, +0x2A staticIdx i16, +0x2C isInterior, +0x2D loaded,
+0x2E loadRequested, +0x2F disableDynamicStreaming, +0x30 ignoreWhenDeleted, +0x31 isLarge.
Pool slot test in all loops below: `flags[i] & 0x80` (sign bit) set = free slot; the object pointer is
`objects + i*0x34`, and the original also skips the slot when that pointer is 0.

## 0x404AC0 CIplStore::FindIplSlot  (gr_deviations = 1)
Row IplStore.cpp:197 (pool-iteration, `GetAllValid()`).
- Original bytes: 0x404AC0-0x404B17 walk slots `i = 0 .. pool->size-1` upwards (ESI counter, EDI byte
  offset += 0x34), skip free slots, `_stricmp(name, def->name)` (0x404AF4); first match returns `i`, end
  returns -1. `pool->size` ([pool+8]) and `ms_pPool` are re-read every iteration (0x404B00/0x404B06).
- Verdict: REFUTED (same ascending order, same result).
- Class: harmless.

## 0x404C90 CIplStore::IncludeEntity  (gr_deviations = 1)
Row IplStore.cpp:281 (`NOTSA_UNREACHABLE("Incorrect entity type")`).
- Original bytes (.HOODLUM body 0x1563730): `type = entity[+0x36] & 7`; type 1 (building) and type 5
  (dummy) are handled; any other type jumps to 0x15637DA and returns without doing anything.
- Verdict: CONFIRMED (gr aborts / UB, original is a no-op).
- Class: keep-original (default case = return).

Found in passing (CONFIRMED, keep-original): gr writes
`lastBuilding = max(firstBuilding, id)` (IplStore.cpp:272). The original compares the index with
`lastBuilding` itself: 0x1563786 `movsx eax, word [ecx+0x24]; cmp edx, eax; jle skip; mov [ecx+0x24], dx`.
So the port must do `if (id < first) first = id; if (id > last) last = id;` for both buildings
(+0x22/+0x24) and dummies (+0x26/+0x28). The index is the full int32 pool index
(`(entity - pool->objects) / 0x38` via the 0x92492493 magic-multiply, signed), compared with the
sign-extended i16 field and then stored as 16 bits; gr truncates to int16 before comparing (same result
for pool sizes < 32768). The dummy pool pointer is read through an obfuscated pointer at 0x1563799-0x15637A8
(it resolves to CPools::ms_pDummyPool).

## 0x404B20 CIplStore::RemoveIpl  (gr_deviations = 1)
Row IplStore.cpp:608 ("In same order as originally").
- Original bytes: buildings (0x404B71-0x404BB2), then the whole object pool (0x404BB4-0x404C0D), then
  dummies (0x404C1B-0x404C58), then a tail jump to CTheCarGenerators::RemoveCarGenerators (0x404C61) with
  the caller's int32 argument still on the stack. `def->loaded (+0x2D) = 0` is written first (0x404B6B).
- Verdict: REFUTED (the order matches).
- Class: harmless.

Found in passing (CONFIRMED, keep-original): gr's `ProcessPool(pool, first, last)` loops
`for (i = first; i < last; i++)` (exclusive). The original loops are inclusive: building loop
`edi = firstBuilding; ...; inc edi; cmp edi, [ebx+0x24]; jle` (0x404BA8-0x404BB2), and the same for dummies
(0x404C4E-0x404C58). The upper bound is re-read from the IplDef every iteration, the loop is skipped
entirely when `first > last` (0x404B6F / 0x404C19). Per entity: skip free slots (building: `test byte
[flags+i], 0x80`; dummy/object: `js`), skip null pointers, compare `entity[+0x2E]` (u8 IPL index) with the
int32 argument, then `CWorld::Remove(e)` and the virtual deleting destructor `vtbl[0](e, 1)`. In the object
loop, if `object[+0x170]` (m_pDummyObject) is non-null, `CWorld::Add(dummy)` is called first. The object
loop runs `0 .. objectPool->size-1` with the size re-read every iteration. The three pool pointers are read
once at entry (0x404B44-0x404B54).

## 0x405C00 CIplStore::LoadIplBoundingBox  (gr_deviations = 1)
Rows IplStore.cpp:416 (`uchar*` vs `char*` note) and :431 (`assert(pIPLLODEntities)`).
- Original bytes: the data pointer is only used for `strncmp(data, "bnry", 4)` (0x405C51), the binary
  header reads and `CFileLoader::LoadLine`; line tests are equality compares against 0, '#', 'i','n','s','t'
  and 'e','n','d' (0x405DA0-0x405DF7), so signedness of `char` does not matter. There is no null check of
  the LOD array: `lod = array[lodIndex]` at 0x405CB6 / 0x405E27.
- Verdict: REFUTED for both rows (no behavioural difference; the assert is debug-only).
- Class: harmless.

Found in passing (keep-original): in the binary path the original relocates the instance offset in place,
`[data+0x1C] += data` (0x405C75-0x405C7D), i.e. it writes into the streaming buffer; gr computes the
pointer without writing. Also `ppCurrIplInstance` (0x8E3EFC) is only advanced when the instance has a LOD
(0x405CBF / 0x405E30), as in gr. The binary path restricts the bounding box inline (0x405D1D-0x405D62:
`left = min`, `right = max` on +0/+8, `+0xC = min`, `+4 = max`, each assigned only on strict `<` / `>`;
NaN never assigns), the text path calls CRect::Restrict (0x405E8F).

## 0x406080 CIplStore::LoadIpl  (gr_deviations = 1)
Rows IplStore.cpp:319 (`uchar*` note) and :360 (`assert(pIPLLODEntities)`).
- Original bytes: same parsing pattern as 0x405C00 (0x4061AF strncmp "bnry", 0x4062A6-0x4062EF char
  equality tests). LOD lookup without null check at 0x406205 / 0x40630B; `++lod->numLodChildren`
  (`inc byte [eax+0x34]`) is inlined.
- Verdict: REFUTED for both rows.
- Class: harmless.

Found in passing (keep-original): `def->loaded = 1` is written at the end of every path (0x406179,
0x406281, 0x40634C), including after a failed LoadIplBoundingBox (the return value is still 0); gr sets it
at the top, which is equivalent because nothing in between reads it. The binary path relocates both
offsets in place: `[data+0x1C] += data` (0x4061CF-0x4061D9) and `[data+0x3C] += data` (0x40624C-0x406250,
car generators, stride 0x30). Counts are i16 and loops end when `count <= 0` (signed test), with unsigned
loop compares.

## 0x405110 CIplStore::RemoveRelatedIpls  (gr_deviations = 1)
Row IplStore.cpp:653 (pool-iteration).
- Original bytes: 0x405130-0x405165 ascending `i = 0 .. size-1` (size and pool pointer re-read after each
  call), `if def->staticIdx (+0x2A, sign-extended) == arg: CStreaming::RemoveModel(i + 0x62A7)`.
- Verdict: REFUTED.
- Class: harmless.

## 0x405FA0 CIplStore::Shutdown  (gr_deviations = 1)
Row IplStore.cpp:45 (pool-iteration).
- Original bytes (.HOODLUM 0x15610B0): after RemoveAllIpls, a fixed loop over 256 slots
  (`edi += 0x34` until 0x3400, 0x15610E5-0x15610EF), ascending, calling RemoveIplSlot(i) for every used
  non-null slot. The bound is the constant 256, not `pool->size` (the pool is created with 256 entries,
  gr `TOTAL_IPL_MODEL_IDS`).
- Verdict: REFUTED (ascending, same slots).
- Class: harmless.

## 0x404DE0 CIplStore::SetupRelatedIpls  (gr_deviations = 3)
Rows IplStore.cpp:767 ("NOTSA: Inverted conditional"), :768 and :779 (pool-iteration), :785 (inlined).
- Original bytes: `if (CColAccel::isCacheLoading())` (0x404F8E) takes the cache branch first, the else
  branch at 0x40504A; both loops are ascending over `0 .. pool->size-1` with size re-read each iteration
  (0x40502E-0x40503F, 0x4050D0-0x4050D9). In the request branch the original re-fetches the slot through
  an inlined GetAt and writes `+0x2F = 0` (0x4050A0-0x4050B6), then `RequestModel(i + 0x62A7, 8)` and,
  after the loop, `LoadAllRequestedModels(false)`. In the cache branch it copies the 0x34-byte IplDef
  returned by `CColAccel::getIplDef(slot)`, then writes staticIdx, isInterior, `loaded = 0` and adds to
  the quad tree.
- Verdict: REFUTED for all four rows.
- Class: harmless.

Found in passing (note): the name buffer is 32 bytes on the stack (entry-0x54..entry-0x34); the name is
copied with a manual loop from after the last '\\' up to the '.', then `"_stream"` (8 bytes incl. NUL) is
appended with two dword stores (0x404F72-0x404F86). A name longer than 24 chars overflows into the
getIplDef temporary (original bug; gr uses a 1024-byte buffer). The return value is
`(ppCurrIplInstance - array) >> 2` (0x4050F0-0x4050F3).
