# RenderWare 3.6 streams (RW core) - full specs, writer A (RE-SPEC-RW)

RenderWare 3.6 code linked into gta_sa.exe 1.0 US; gta-reversed/plugin-sdk have thunks only. All `__cdecl`.
Every RW error below is reported as `RwErrorSet(&{pluginID = 1, errorCode = _rwerror(code, ...)})` (RwEngine.md);
`_rwerror` returns its first argument, extra arguments are ignored.

## Layouts
`RwStream` (0x24 bytes, freelist entry size 0x24 at 0x7EC79E; SDK rwplcore.h:3098):
| off | field | proof |
|---|---|---|
| +0x0 | type: 1 rwSTREAMFILE, 2 rwSTREAMFILENAME, 3 rwSTREAMMEMORY, 4 rwSTREAMCUSTOM | 0x7EC833, 0x7EC9DA |
| +0x4 | accessType: 1 READ, 2 WRITE, 3 APPEND | 0x7EC83A |
| +0x8 | position (never touched by the functions here) | - |
| +0xC | file: `void* fp`; memory: `u32 position`; custom: `sfnclose` | 0x7EC86B, 0x7ECA79, 0x7ECE79 |
| +0x10 | memory: `u32 nSize`; custom: `sfnread(data, buf, len)` | 0x7ECA76, 0x7ECAE4 |
| +0x14 | memory: `u8* memBlock`; custom: `sfnwrite` | 0x7ECAA8 |
| +0x18 | custom: `sfnskip(data, offset)` | 0x7ECDCF |
| +0x1C | custom: `void* data` | 0x7ECADE |
| +0x20 | `RwBool rwOwned` (stream freed by RwStreamClose) | 0x7EC83D, 0x7ECE8E |
Note: docs/re/specs/STREAM/RwGtaStream.md says rwOwned is at +0x18; the bytes put it at +0x20 (+0x18 is sfnskip).

`RwMemory` {u8* start +0x0, u32 length +0x4} (0x7EC96A/0x7EC970). `RwChunkHeaderInfo` {type +0x0, length +0x4,
version +0x8, buildNum +0xC, isComplex +0x10} (0x7ED5CB..0x7ED5E6).

File functions (RwGlobals `G` = [0xC97B24], `RwFileFunctions` at G+0xC4): +0xC8 fopen(name, mode), +0xCC fclose(fp),
+0xD0 fread(buf, 1, n, fp), +0xE0 feof(fp), +0xE4 fseek(fp, off, whence), +0xEC ftell(fp) (SDK order fexist,
fopen, fclose, fread, fwrite, fgets, fputs, feof, fseek, fflush, ftell). Mode strings: 0x8E2220 "rb", 0x8E2224 "wb",
0x8E2228 "ab".

STREAMMODULE (engine plugin 0x404, 4 bytes): offset global 0xC9792C; `G + [0xC9792C]` +0 = stream freelist.
Instance counter 0xC97930.

Error codes used: 0x80000002 E_RW_BADOPEN, 0x80000004 E_RW_BADVERSION, 0x80000013 E_RW_NOMEM, 0x8000001A E_RW_READ
(common codes, errcom.def index + 1 | 0x80000000); 5 E_RW_ENDOFSTREAM, 0xD E_RW_INVSTREAMACCESSTYPE, 0xE
E_RW_INVSTREAMTYPE (core codes, errcore.def index).

## 0x7EC780 cb_STREAMMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- `void* Open(instance, offset, size)`: `[0xC9792C] = offset`; `G+offset+0 = RwFreeListCreateAndPreallocateSpace(0x24,
  [0x8E2218] (init 0x10), 4, [0x8E221C] (init 1), (void*)0xC97908, 0x40404)`; NULL -> return NULL; `[0xC97930]++`;
  return instance.

## 0x7EC810 _rwStreamInitialize  (catalogue: specified/REA, port level 1)
- Convention: `RwStream* (RwStream* s, RwBool rwOwned, RwStreamType type, RwStreamAccessType access, const void* pData)`;
  returns s or NULL. Callers: CStreaming::FinishLoadingLargeFile 0x408CB0, CStreaming::ConvertBufferToObject 0x40C6B0
  (GTA initialises stack streams with it). Jump table 0x7EC9C0 (type - 1, 4 entries).
- Behaviour: s == NULL -> return NULL. `s->type = type; s->accessType = access; s->rwOwned = rwOwned` (before the type
  check). Switch type:
  - 1 FILE: if `G->ftell(pData) == -1` return NULL (no error); else `s->fp = pData`; return s.
  - 2 FILENAME: mode = "rb"/"wb"/"ab" for access 1/2/3; other access: RwErrorSet(0xD) and then also RwErrorSet(BADOPEN,
    pData) (the second call is ignored by RwErrorSet's sticky logic), return NULL. `fp = G->fopen(pData, mode)`;
    NULL -> error 0x80000002 E_RW_BADOPEN (with pData), return NULL; else `s->fp = fp`; return s.
  - 3 MEMORY: access 1 READ: `pos = 0; nSize = mem->length; memBlock = mem->start`; 2 WRITE: `pos = nSize = 0,
    memBlock = NULL` (pData unused); 3 APPEND: `pos = nSize = mem->length; memBlock = mem->start`; other: error 0xD,
    return NULL.
  - 4 CUSTOM: copy 5 dwords from pData to s+0xC..+0x1F; return s.
  - other: error 0xE E_RW_INVSTREAMTYPE; return NULL.
- librw: absent (librw uses C++ stream classes).

## 0x7EC9D0 RwStreamRead  (catalogue: specified/REA, port level 1)
- `RwUInt32 (RwStream* s, void* buf, RwUInt32 len)`; returns the number of bytes read. Callers (57). Table 0x7ECB1C.
- FILE / FILENAME: `n = G->fread(buf, 1, len, s->fp)`; if `n != len`: if `G->feof(fp)` -> error 5 E_RW_ENDOFSTREAM,
  else error 0x8000001A E_RW_READ. Return n.
- MEMORY: `avail = nSize - pos` (u32); if `len > avail` (unsigned): error 5, `len = avail`. Copy len bytes from
  `memBlock + pos` (dwords then bytes); `pos += len`; return len.
- CUSTOM: return `s->sfnread(s->data, buf, len)`.
- other: error 0xE; return 0.
- librw: base.cpp:StreamMemory::read8 differs (librw sets an EOF state on short read and returns 0 afterwards).

## 0x7ECD00 RwStreamSkip  (catalogue: specified/REA, port level 1)
- `RwStream* (RwStream* s, RwUInt32 offset)`; returns s or NULL. offset == 0 -> return s immediately (type not checked).
- FILE / FILENAME: `if (G->fseek(fp, offset, 1 /*SEEK_CUR*/) != 0) { if (G->feof(fp)) error 5; return NULL; }` return s.
- MEMORY: `np = pos + offset` (u32 wrap); if `np > nSize`: `pos = nSize`; error 5; return NULL. Else `pos = np`; return s.
- CUSTOM: return `s->sfnskip(s->data, offset) ? s : NULL`.
- other: error 0xE; return NULL.
- librw: base.cpp:StreamMemory::seek differs (no error, EOF marker).

## 0x7ECE20 RwStreamClose  (catalogue: specified/REA, port level 1)
- `RwBool (RwStream* s, void* pData)`. Table 0x7ECEE0.
- FILE: nothing (the caller owns the FILE*); result 1. FILENAME: `result = (G->fclose(fp) == 0)`.
  MEMORY: result 1; if `access != READ && pData != NULL`: `((RwMemory*)pData)->start = memBlock; ->length = pos`
  (position, not nSize). CUSTOM: if `sfnclose` non-NULL call `sfnclose(data)`; result 1.
- Then, for those four types: if `s->rwOwned` -> `G->memoryFree(streamFreeList, s)`. Return result.
- other type: error 0xE; return 0 (stream not freed).

## 0x7ECEF0 RwStreamOpen  (catalogue: specified/REA, port level 1)
- `RwStream* (RwStreamType type, RwStreamAccessType access, const void* pData)`; callers (17) e.g. CAnimManager::LoadAnimFiles.
- Behaviour: `s = G->memoryAlloc(streamFreeList, 0x30404)`; NULL -> return NULL (no error). `s->type, s->accessType
  = args; s->rwOwned = 1`. Then exactly the per-type initialisation of `_rwStreamInitialize` (same errors; table
  0x7ED0D4). On any failure (including an unknown type, error 0xE) the stream is returned to the freelist and NULL
  is returned; on success s.
- librw: absent.

## 0x7ED1E0 _rwChunkIsComplex  (catalogue: unknown/REA, port level 0)
- `RwBool (const RwChunkHeaderInfo* info)`: returns 1 for `info->type` in {5, 6, 7, 8, 9, 0xA, 0xB, 0xE, 0xF, 0x10,
  0x12, 0x14, 0x1A} (jump table 0x7ED1FC indexed by type - 1, 26 entries), else 0 (also for type 0 or > 0x1A).
  So rwID_STRUCT(1)/STRING(2)/EXTENSION(3), 0xC, 0xD, 0x11, 0x13, 0x15..0x19 are "not complex". Callers:
  `_rwStreamReadChunkHeader` 0x7ED0F0 (result discarded), RwStreamReadChunkHeaderInfo.

## 0x7ED2D0 RwStreamFindChunk  (catalogue: specified/REA, port level 3)
- `RwBool (RwStream* s, RwUInt32 type, RwUInt32* lengthOut, RwUInt32* versionOut)`; callers (52).
- Callee `_rwStreamReadChunkHeader` 0x7ED0F0 `(s, &type, &length, &version, &build)`: reads 12 bytes (fails with
  E_RW_READ if fewer); version decode: `id` = third dword; if `id & 0xFFFF0000`: `version = (((id >> 14) & 0x3FF00) +
  0x30000) | ((id >> 16) & 0x3F)`, `build = id & 0xFFFF`; else `version = id << 8`, `build = 0`.
- Behaviour: read a header (NULL build pointer); fail -> return 0. Loop: if `hdr.type == type` -> version check;
  else `RwStreamSkip(s, hdr.length)` (NULL -> return 0) and read the next header (fail -> return 0).
  Version check on the matching chunk only: `version < 0x34000` or `version > 0x36003` (unsigned) -> error
  0x80000004 E_RW_BADVERSION, return 0. Else `*lengthOut = length` and `*versionOut = version` when the pointers are
  non-NULL; return 1.
- librw: base.cpp:findChunk differs (librw stops at ID_NAOBJECT, has no version check).

## 0x7ED4F0 RwStreamReadReal  (catalogue: specified/REA, port level 2)
- `RwStream* (RwStream* s, RwReal* p, RwUInt32 numBytes)`: `if (RwStreamRead(s, p, numBytes) == 0) { error
  0x8000001A; return NULL; } return s`. No byte swapping (x86), only a zero result is treated as failure
  (a short read returns s).

## 0x7ED540 RwStreamReadInt32  (catalogue: specified/REA, port level 2)
- Identical to RwStreamReadReal for `RwInt32*`.

## 0x7ED590 RwStreamReadChunkHeaderInfo  (catalogue: specified/REA, port level 3)
- `RwStream* (RwStream* s, RwChunkHeaderInfo* info)`; caller CStreaming::ConvertBufferToObject 0x40C6B0.
- Behaviour: `_rwStreamReadChunkHeader(s, &type, &length, &version, &build)`; 0 -> return NULL. Fill `info->type,
  length, version, buildNum` in that order, then `info->isComplex = _rwChunkIsComplex(info)`; return s.
- librw: base.cpp:readChunkHeaderInfo matches (librw has no isComplex).

## 0x80A510 _rwStringStreamFindAndRead  (catalogue: specified/REA, port level 3)
- `RwChar* (RwChar* string, RwStream* s)`; returns the string buffer or NULL. Caller: RwTextureStreamRead 0x8046E0.
  Local buffer of 0x80 bytes at [esp+0x2C].
- Behaviour: read chunk headers (no build pointer) in a loop; for EVERY header first check the version: outside
  [0x34000, 0x36003] -> error 0x80000004, return NULL. Type 2 (rwID_STRING) -> ASCII; type 0x13 (rwID_UNICODESTRING)
  -> Unicode; else `RwStreamSkip(s, length)` (NULL -> return NULL) and read the next header (fail -> return NULL).
  - ASCII: `buf = string`; if NULL: `buf = G->rwmalloc(length, 0x30002)`; NULL -> error 0x80000013 (length), return NULL.
    While `remaining > 0`: `n = min(remaining, 0x40)`; `RwStreamRead(s, local, n) != n` -> return NULL (an allocated
    buffer leaks); copy n bytes to `buf + done`. Return buf. No terminator is added (the chunk is expected to
    contain it) and a caller buffer is not size-checked.
  - Unicode: same allocation (`length` bytes, flag "allocated" kept); chunks of `min(remaining, 0x80)` bytes; for
    each chunk copy `n >> 1` bytes: `dst[i] = local[2*i]` (low byte of each UTF-16 unit); on a short read: if allocated,
    `G->rwfree(buf)`; return NULL.
- librw: absent (librw readString handles only rwID_STRING).

## Open questions
- None.
