# GTA RenderWare stream readers (STREAM) - full specs, writer A (RE-SPEC-1)

Free functions (no class) that GTA added on top of RenderWare to split texture-dictionary and clump loading
into two halves ("Read1" at stream start, "Read2" when the rest of the file has arrived). gta-reversed only
has `plugin::CallAndReturn` wrappers for them (RenderWare/rw/rwtexdict.cpp) - no code to port; everything
below comes from the bytes.

RW structures used (gta-reversed RenderWare headers):
- `RwStream`: +0x0 type, +0x4 accessType, +0x8 position, +0xC `Type` union (memory stream: u32 memory.position
  at +0xC; file stream: FILE* at +0xC), +0x18 rwOwned. The functions below save/compare the raw dword at
  +0xC. GTA streams these through memory streams, so it is the byte position inside the memory block.
- `RwTexture`: +0x50 u32 filterAddressing (low byte = filter mode: 1 rwFILTERNEAREST, 2 rwFILTERLINEAR,
  3 rwFILTERMIPNEAREST, 4 rwFILTERMIPLINEAR).
- `RwGlobals` (`RwEngineInstance` at 0xC97B24): stdFunc[] starts at +0x48 (after curCamera, curWorld, 4 u16,
  and the 0x38-byte RwDevice), so +0xB0 = `stdFunc[26]` = rwSTANDARDNATIVETEXTUREREAD,
  signature `RwBool (*)(void* pOut, void* pInOut, RwInt32 nI)`; called as `(stream, &texture, length)`.
- Chunk ids: 1 rwID_STRUCT, 0xE rwID_FRAMELIST, 0xF rwID_GEOMETRY, 0x14 rwID_ATOMIC, 0x15 rwID_TEXTURENATIVE,
  0x1A rwID_GEOMETRYLIST.
- `rpGeometryList` { RpGeometry** geometries +0x0; i32 numGeoms +0x4 }; `rwFrameList` { RwFrame** frames +0x0;
  i32 numFrames +0x4 }.

Globals of the split readers (unnamed in gr; names below are proposals, from re3's ClumpRead.cpp/TexRead.cpp
which have the same scheme for GTA III/VC):
- 0x8D607C i32 `numberGeometrys` (initial value 0xFFFFFFFF in .data)
- 0xC87AE8 `gClumpInfo` (12 bytes: i32 numAtomics +0, numLights +4, numCameras +8)
- 0xC87AF4 `gFrameList` (rwFrameList, 8 bytes)
- 0xC87AFC i32 `streamPosition` (clump)
- 0xC87B00 `gGeomList` (rpGeometryList, 8 bytes)
- 0x8D6088 i32 `numberTextures` (initial value 0xFFFFFFFF in .data)
- 0xC87FE8 i32 `streamPosition` (texture dictionary)
- 0xC87FEC f32 texture-load average time in ms (proposed `gTexLoadAvgMs`), 0xC87FF0 i32 its sample count.

Callee addresses (all via `orig!`): RwStreamFindChunk 0x7ED2D0 `(stream, type, u32* len, u32* ver)` returns
nonzero on success; RwStreamRead 0x7EC9D0 `(stream, buf, len)` returns bytes read; RwStreamSkip 0x7ECD00
`(stream, n)`; RpGeometryStreamRead 0x74D190; CMemoryMgr::Malloc 0x72F420 `(size, hint)` (forwards size to
malloc 0x824257); RwTexDictionaryCreate 0x7F3600; RwTexDictionaryDestroy 0x7F36A0;
RwTexDictionaryForAllTextures 0x7F3730; RwTexDictionaryAddTexture 0x7F3980 `(dict, tex)`; RwTextureDestroy
0x7F3820; 0x730E50 = local callback `RwTexture* cb(RwTexture* t, void*) { RwTextureDestroy(t); return NULL; }`
(returning NULL makes ForAllTextures stop after the first texture); RpClumpCreate 0x74A290; RpClumpDestroy
0x74A310; RpClumpAddAtomic 0x74A490 `(clump, atomic)`; _rwFrameListStreamRead 0x807970 `(stream, frameList)`;
_rwFrameListDeinitialize 0x807720 `(frameList)`; CTimer::GetCurrentTimeInCycles 0x561A80;
CTimer::GetCyclesPerMillisecond 0x561A40; RpAnisotTextureGetMaxAnisotropy 0x748F50 `(tex)` returns i8 in AL;
RpAnisotGetMaxSupportedMaxAnisotropy 0x748F20; RpAnisotTextureSetMaxAnisotropy 0x748F30 `(tex, val)`;
Fx_c::GetFxQuality 0x49EA50 (thiscall, ECX = &g_fx 0xA9AE00). Non-standard helpers: 0x72E270 (atomic read:
EAX = stream, EBX = frameList, [esp+4] = geomList, cdecl cleanup by caller) and 0x72E360 (geometry list
deinitialise: ESI = geomList).

## 0x72E140 GeometryListStreamRead1 (proposed name)  (catalogue: unknown/REA, port level 9)
Evidence: only caller RpClumpGtaStreamRead1 passes `EBX = &gGeomList`; body matches re3
`GeometryListStreamRead1(RwStream*, rpGeometryList*)` (reads the count, allocates, reads only the first half
of the geometries).
- Convention (non-standard): stream at [esp+4] (cdecl, caller pops 4); `geomList` in **EBX** (preserved);
  returns EAX = geomList or NULL. Preserves EBP, ESI, EDI.
- Globals: W/R 0x8D607C numberGeometrys.
- Behaviour:
  1. `numberGeometrys = 0;` then `RwStreamFindChunk(stream, 1, &len, &ver)`; 0 -> return NULL.
  2. `RwStreamRead(stream, &n, 4)` into a local i32; result != 4 -> return NULL.
  3. `numberGeometrys = n / 2` (signed division truncating toward zero: `cdq; sub eax,edx; sar eax,1`);
     `geomList->numGeoms = n`.
  4. If `n > 0` (signed): `p = Malloc(n*4, 0); geomList->geometries = p;` if p == NULL -> return NULL;
     zero n dwords at p. Else `geomList->geometries = NULL`.
  5. `for (i = 0; i < numberGeometrys; i++)` (bound re-read from the global each iteration, signed):
     `RwStreamFindChunk(stream, 0xF, NULL, &ver)` == 0 -> return NULL;
     `geomList->geometries[i] = RpGeometryStreamRead(stream)`; re-read it, NULL -> return NULL.
  6. Return geomList.
- Edge cases: on failure the allocated array and already-read geometries are not freed (leak, original
  behaviour). Chunk length is not checked; exactly 4 bytes are read.
- Open questions: none.

## 0x72E220 GeometryListStreamRead2 (proposed name)  (catalogue: unknown/REA, port level 9)
Evidence: only caller RpClumpGtaStreamRead2 passes `EBX = stream, EDI = &gGeomList`; continues the loop of
0x72E140 from `numberGeometrys` to `numGeoms` (re3 `GeometryListStreamRead2`).
- Convention (non-standard): stream in **EBX**, geomList in **EDI** (both preserved); no stack args; plain
  `ret`; returns EAX = geomList or NULL. Preserves ESI.
- Behaviour: `for (i = numberGeometrys; i < geomList->numGeoms; i++)` (signed; numGeoms re-read each
  iteration, numberGeometrys read once): `RwStreamFindChunk(stream, 0xF, NULL, &ver)` == 0 -> return NULL;
  `geomList->geometries[i] = RpGeometryStreamRead(stream)`; NULL -> return NULL. Return geomList.
- Open questions: none.

## 0x72E570 RpClumpGtaStreamRead1  (catalogue: specified/REA, port level 10)
- Convention: `__cdecl bool (RwStream* stream)`; return AL (0/1); plain `ret`.
- Callers: 1 (CFileLoader::StartLoadClumpFile 0x5373F0).
- Behaviour:
  1. `RwStreamFindChunk(stream, 1, &len, &ver)` == 0 -> return false.
  2. `RwStreamRead(stream, &gClumpInfo /*0xC87AE8*/, 12) != 12` -> return false.
  3. `RwStreamFindChunk(stream, 0xE, NULL, &ver)` == 0 -> return false.
  4. `_rwFrameListStreamRead(stream, &gFrameList /*0xC87AF4*/)` == NULL -> return false.
  5. If `RwStreamFindChunk(stream, 0x1A, NULL, &ver)` != 0 and `GeometryListStreamRead1(stream)` with
     EBX = &gGeomList (0xC87B00) returns non-NULL: `streamPosition (0xC87AFC) = *(u32*)(stream+0xC)`;
     return true.
  6. Otherwise: `_rwFrameListDeinitialize(&gFrameList)`; return false.
- Edge cases: failures in steps 1-4 do not deinitialise anything; a failure inside GeometryListStreamRead1
  leaves gGeomList partially filled (not freed). Chunk lengths are ignored (12 bytes are always read).
- Open questions: none.

## 0x72E620 RpClumpGtaStreamRead2  (catalogue: specified/REA, port level 10)
- Convention: `__cdecl RpClump* (RwStream* stream)`; return EAX; plain `ret`.
- Callers: 1 (CFileLoader::FinishLoadClumpFile 0x537450).
- Behaviour:
  1. `clump = RpClumpCreate()`; NULL -> return NULL (globals untouched, frame/geometry lists not freed).
  2. `RwStreamSkip(stream, streamPosition - *(u32*)(stream+0xC))` (result ignored).
  3. `GeometryListStreamRead2` (EBX = stream, EDI = &gGeomList). NULL -> goto fail.
  4. `clump->object.parent (+0x4) = gFrameList.frames[0]` (i.e. `RpClumpSetFrame(clump, frames[0])` as a raw
     store).
  5. `for (i = 0; i < gClumpInfo.numAtomics; i++)` (bound re-read each iteration, signed):
     `RwStreamFindChunk(stream, 0x14, NULL, &ver)` == 0 -> goto fail;
     `atomic = 0x72E270(EAX = stream, EBX = &gFrameList, push &gGeomList)`; NULL -> goto fail;
     `RpClumpAddAtomic(clump, atomic)`.
  6. Success: `0x72E360(ESI = &gGeomList)` (destroy geometry refs, free array); `_rwFrameListDeinitialize(&gFrameList)`;
     return clump.
  fail: `0x72E360(ESI = &gGeomList)`; `_rwFrameListDeinitialize(&gFrameList)`; `RpClumpDestroy(clump)`;
  return NULL.
- Edge cases: lights and cameras counted in gClumpInfo are not read (the rest of the clump chunk is left in
  the stream). Frame 0 is taken without checking numFrames > 0.
- Open questions: none.

## 0x730E60 RwTextureGtaStreamRead  (catalogue: specified/REA, port level 4)
- Convention: `__cdecl RwTexture* (RwStream* stream)`; return EAX; plain `ret`. Preserves ESI.
- Callers: 3 (RwTexDictionaryGtaStreamRead, ...Read1, ...Read2).
- Globals: R RwEngineInstance 0xC97B24; R i32 gGameState 0xC8D4C0 (compared as a dword); R/W f32 0xC87FEC,
  i32 0xC87FF0; R u8 anisotropySupportedByGFX 0xC87FFC; constant f32 0x858C54 = 4294967296.0.
- Locals: `tex` (RwTexture*, initially uninitialised), `len`, `ver`, f32 `t`, f32 `start`.
- Behaviour:
  1. `RwStreamFindChunk(stream, 0x15, &len, &ver)` == 0 -> return NULL.
  2. Start time (x87, result stored as f32): `c = GetCurrentTimeInCycles()` (u32); `fild` it as int32; if it was
     negative as int32, `fadd f32 4294967296.0`; `fstp` f32 -> `t`. `k = GetCyclesPerMillisecond()` (u32),
     same unsigned conversion in ST0; `ST0 = t / ST0` (`fdivr m32`); `fstp` f32 -> `start`.
  3. `ok = RwEngineInstance->stdFunc[26](stream, &tex, len)` (call through [RwEngineInstance+0xB0], cdecl,
     3 args). `ok == 0` -> return NULL.
  4. If `gGameState (u32 at 0xC8D4C0) == 8` (GAME_STATE_LOADING_STARTED):
     `c2 = GetCurrentTimeInCycles()` -> unsigned-to-x87 as above -> `fstp` f32 `t2`; `k2 =
     GetCyclesPerMillisecond()` -> unsigned-to-x87; `ST0 = t2 / ST0`; `ST0 = ST0 - start` (f32 operand) = `d`;
     `cnt = *(i32*)0xC87FF0; new = cnt + 1;` `fild cnt` (old value); `ST0 = cnt * avg` (avg = f32 at 0xC87FEC);
     `ST0 = ST0 + d`; `ST0 = ST0 / new` (`fidiv` int32); `fstp` f32 -> 0xC87FEC; `*(i32*)0xC87FF0 = new`
     (written before the fmul); pop `d`.
     Every x87 operation rounds to the in-game precision control (24-bit).
  5. If `tex == NULL` -> return NULL.
  6. `f = tex->filterAddressing & 0xFF; if (f == 1) low byte = 2; else if (f == 3) low byte = 4;` (only the
     low byte is written).
  7. If `anisotropySupportedByGFX != 0` and `(i8)RpAnisotTextureGetMaxAnisotropy(tex) >= 1` (signed byte
     compare) and `g_fx.GetFxQuality() >= 2` (signed): `RpAnisotTextureSetMaxAnisotropy(tex,
     RpAnisotGetMaxSupportedMaxAnisotropy())`.
  8. Return tex.
- Edge cases: if the native reader returns success without writing `tex`, the uninitialised local is used
  (original bug, unreachable with the D3D9 driver). Steps 2 and 4 have no effect on the result except the
  timing globals; they also call the timer functions (side-effect-free).
- Open questions: none.

## 0x730FC0 RwTexDictionaryGtaStreamRead  (catalogue: specified/REA, port level 5)
- Convention: `__cdecl RwTexDictionary* (RwStream* stream)`; return EAX; plain `ret`. Preserves ESI, EDI.
- Callers: 2 (CFileLoader::LoadTexDictionary 0x5B3860, CTxdStore::LoadTxd(stream) 0x731DD0).
- Behaviour:
  1. `RwStreamFindChunk(stream, 1, &len, &ver)` == 0 -> return NULL.
  2. `RwStreamRead(stream, &hdr, len)` where `hdr` is a 4-byte local {u16 numTextures, u16 deviceId};
     if the return value != `len` (re-read from its stack slot after the call) -> return NULL.
  3. `dict = RwTexDictionaryCreate()`; NULL -> return NULL.
  4. `while (numTextures != 0) { numTextures--; tex = RwTextureGtaStreamRead(stream); if (!tex) {
     RwTexDictionaryForAllTextures(dict, cb_0x730E50, NULL); RwTexDictionaryDestroy(dict); return NULL; }
     RwTexDictionaryAddTexture(dict, tex); }` (16-bit counter).
  5. Return dict.
- Edge cases: `len` bytes are read into a 4-byte stack buffer: a struct chunk longer than 4 bytes overwrites
  `len`/`ver` and further stack (original bug; real files have len 4). The failure-path callback destroys
  only the first texture of the dictionary (it returns NULL), then the dictionary destroy handles the rest.
- Open questions: none.

## 0x731070 RwTexDictionaryGtaStreamRead1  (catalogue: specified/REA, port level 5)
- Convention: `__cdecl RwTexDictionary* (RwStream* stream)`; return EAX; plain `ret`.
- Callers: 1 (CTxdStore::StartLoadTxd 0x731930).
- Globals: W/R 0x8D6088 numberTextures; W 0xC87FE8 streamPosition.
- Behaviour:
  1. `numberTextures = 0;` (before FindChunk) `RwStreamFindChunk(stream, 1, &len, &ver)` == 0 -> return NULL.
  2. `RwStreamRead(stream, &hdr, len) != len` -> return NULL (same 4-byte-buffer caveat as 0x730FC0).
  3. `dict = RwTexDictionaryCreate()`; NULL -> return NULL.
  4. `n = (u16)hdr.numTextures; numberTextures = n >> 1;` If `n > (n >> 1)` (signed int compare; false only
     for n == 0): loop { `tex = RwTextureGtaStreamRead(stream)`; NULL -> ForAllTextures(dict, cb, NULL),
     RwTexDictionaryDestroy(dict), return NULL (numberTextures stays n>>1); `AddTexture(dict, tex)`;
     `n = (u16)(n - 1)` (stored back to the local); continue while `(u32)n > numberTextures` (signed
     compare, numberTextures re-read) }.
     This reads `n - n/2` textures (the first, larger half).
  5. `streamPosition (0xC87FE8) = *(u32*)(stream+0xC); numberTextures = (u16)n` (the remaining count, equal to
     the original n >> 1); return dict.
- Open questions: none.

## 0x731150 RwTexDictionaryGtaStreamRead2  (catalogue: specified/REA, port level 5)
- Convention: `__cdecl RwTexDictionary* (RwStream* stream, RwTexDictionary* dict)`; return EAX; plain `ret`.
- Callers: 1 (CTxdStore::FinishLoadTxd 0x731E40).
- Behaviour:
  1. `RwStreamSkip(stream, streamPosition(0xC87FE8) - *(u32*)(stream+0xC))` (result ignored).
  2. `while (numberTextures != 0) { numberTextures--; tex = RwTextureGtaStreamRead(stream); if (!tex) {
     ForAllTextures(dict, cb_0x730E50, NULL); RwTexDictionaryDestroy(dict); return NULL; }
     AddTexture(dict, tex); }` (numberTextures re-read from memory each time; decremented before the read).
  3. `numberTextures--` (so it ends at -1, its initial value); return dict.
- Edge cases: on failure numberTextures is left at the already-decremented value (not reset to -1).
- Open questions: none.
