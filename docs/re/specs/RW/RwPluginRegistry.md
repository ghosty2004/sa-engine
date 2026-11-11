# RenderWare 3.6 plugin registry (RW core) - full specs, writer A (RE-SPEC-RW)

RenderWare 3.6 code linked into gta_sa.exe 1.0 US; gta-reversed/plugin-sdk have thunks only. All `__cdecl`.
Errors are reported as `RwErrorSet(&{1, _rwerror(code)})` (RwEngine.md).

## Layouts
`RwPluginRegistry` (0x18, in .data; table of all 13 registries in docs/re/rw/plugins.md):
+0x0 sizeOfStruct (0x8085B2), +0x4 origSizeOfStruct, +0x8 maxSizeOfStruct (0x8085B4), +0xC staticAlloc,
+0x10 firstRegEntry (0x808474), +0x14 lastRegEntry (0x808746).

`RwPluginRegEntry` (0x3C, from the freelist at 0xC9A6E4; field stores 0x8085EF..0x808643):
| off | field | off | field |
|---|---|---|---|
| +0x0 | offset | +0x20 | constructCB `void* (obj, offset, size)` |
| +0x4 | size (unrounded) | +0x24 | destructCB `void* (obj, offset, size)` |
| +0x8 | pluginID | +0x28 | copyCB `void* (dst, src, offset, size)` |
| +0xC | readCB `RwStream* (stream, len, obj, offset, size)` | +0x2C | errStrCB (0) |
| +0x10 | writeCB | +0x30 | next |
| +0x14 | getSizeCB | +0x34 | prev |
| +0x18 | alwaysCB `RwBool/ptr (obj, offset, size)` | +0x38 | parentRegistry |
| +0x1C | rightsCB `(obj, offset, size, extraData)` | | |

Globals: 0xC9A6E4 registry-entry freelist (created by 0x8082E0, see RwMemory.md); 0xC9A6E8 `RwPluginRegistry**`
array of registries and 0xC9A6EC its count (only maintained when freelists are disabled, below). Engine registry
0x8E2298.

Entry lookup (inlined in 0x808470, 0x8084A0, 0x8088E0, 0x808920, 0x808950, 0x808980, 0x808AB0): walk from
`firstRegEntry` via `next`, first entry with `pluginID == id`.

## 0x7F2BB0 RwEngineRegisterPlugin  (catalogue: specified/REA, port level 2)
- `RwInt32 (RwInt32 size, RwUInt32 pluginID, RwPluginObjectConstructor initCB, RwPluginObjectDestructor termCB)`
  -> `_rwPluginRegistryAddPlugin(0x8E2298, size, pluginID, initCB, termCB, NULL)`. Callers (9): RpWorldPluginAttach,
  RpAnisotPluginAttach, ...

## 0x7F2BE0 RwEngineGetPluginOffset  (catalogue: specified/REA, port level 1)
- `RwInt32 (RwUInt32 pluginID)` -> `_rwPluginRegistryGetPluginOffset(0x8E2298, pluginID)`. Callers: skin plugin
  (0x7C68E0 queries 0x120 and 0x12E), RpSkinAtomicSetType.

## 0x808470 _rwPluginRegistryGetPluginOffset  (catalogue: specified/REA, port level 0)
- `RwInt32 (const RwPluginRegistry* reg, RwUInt32 id)`: entry lookup; found -> `entry->offset`, else -1.

## 0x8084A0 _rwPluginRegistryAddPlugin  (catalogue: specified/REA, port level 1)
- `RwInt32 (RwPluginRegistry* reg, RwInt32 size, RwUInt32 id, ctor, dtor, copy)`; returns the plugin offset or -1.
  Callers (14): every `*RegisterPlugin` and RwEngineInit.
- Behaviour, in order:
  1. If the entry freelist `[0xC9A6E4] == NULL` -> return -1 (no error).
  2. If `_rwGetNumEngineInstances()` (0x7F2B90) != 0 -> error 0x80000017 E_RW_PLUGININIT, return -1.
  3. If `G->memoryAlloc (G+0x144) != 0x801C30` (`_rwFreeListAllocReal`, i.e. the engine was initialised with
     rwENGINEINITNOFREELISTS): if reg is not yet in the array `[0xC9A6E8][0..count)`: `n = G->rwmalloc(count*4 + 4,
     0x40000)` (result not NULL-checked); if the old array is non-NULL copy `count` entries and `G->rwfree(old)`;
     `n[count] = reg` (index 0 when the old array was NULL); `[0xC9A6E8] = n; [0xC9A6EC] = count + 1`.
     (docs/re/rw/plugins.md states the opposite condition; the bytes `cmp [ecx+0x144], 0x801C30; je skip` at 0x8084F1
     show the array is kept only when the default freelist allocator is NOT installed.)
  4. Duplicate: if an entry with this id exists -> error 0x80000017, return that entry's offset.
  5. `newSize = reg->sizeOfStruct + ((size + 3) & ~3)`; if `reg->maxSizeOfStruct != 0 && newSize > max` (signed) ->
     return -1 (no error).
  6. `e = G->memoryAlloc([0xC9A6E4], 0x40000)`; NULL -> return -1.
  7. `e->offset = reg->sizeOfStruct; reg->sizeOfStruct = newSize; e->size = size; e->pluginID = id;` callbacks
     +0xC..+0x1C = 0; `ctor/dtor/copy` or the defaults 0x8086B0 / 0x8086C0 / 0x8086D0 when NULL; errStrCB = 0;
     next = prev = NULL; parentRegistry = reg.
  8. Append: if `reg->firstRegEntry == NULL` -> first = last = e; else `last->next = e; e->prev = last; last = e`.
  9. Return `e->offset`.
- librw: plg.cpp:PluginList::registerPlugin differs (no engine-open check, no duplicate check returning the old
  offset; librw keeps a global list of all plugins instead of the registry array).

## 0x8086B0 _rwPluginRegistryDefaultConstructor  (catalogue: specified/REA, port level 0)
- `void* (void* obj, RwInt32 offset, RwInt32 size)`: returns obj.

## 0x8086C0 _rwPluginRegistryDefaultDestructor  (catalogue: specified/REA, port level 0)
- `void* (void* obj, RwInt32 offset, RwInt32 size)`: returns obj.

## 0x8086D0 _rwPluginRegistryDefaultCopyConstructor  (catalogue: specified/REA, port level 0)
- `void* (void* dst, const void* src, RwInt32 offset, RwInt32 size)`: returns dst.

## 0x8086E0 _rwPluginRegistryInitObject  (catalogue: specified/REA, port level 0)
- `const RwPluginRegistry* (const RwPluginRegistry* reg, void* obj)`; returns reg on success (NOT obj), NULL on failure.
  Callers (21): RpAtomicCreate, RpClumpCreate, RwEngineStart (module Open callbacks), _rwFrameInternalInit, ...
- Behaviour: for each entry e from first (via next): `if (e->constructCB(obj, e->offset, e->size) == NULL)`: for each
  entry p from `e->prev` backwards (via prev): `p->destructCB(obj, p->offset, p->size)`; return NULL. Return reg.
- librw: plg.cpp:PluginList::construct differs (ignores constructor failures).

## 0x808740 _rwPluginRegistryDeInitObject  (catalogue: specified/REA, port level 0)
- `const RwPluginRegistry* (reg, void* obj)`: for each entry from **last** backwards (prev): `destructCB(obj, offset,
  size)` (result ignored). Returns reg.
- librw: plg.cpp:PluginList::destruct differs (forward order).

## 0x808770 _rwPluginRegistryCopyObject  (catalogue: specified/REA, port level 0)
- `const RwPluginRegistry* (reg, void* dst, const void* src)`: for each entry from first: `copyCB(dst, src, offset,
  size)` (result ignored). Returns reg.
- librw: plg.cpp:PluginList::copy matches.

## 0x8088E0 _rwPluginRegistryAddPluginStream  (catalogue: specified/REA, port level 0)
- `RwInt32 (reg, RwUInt32 id, readCB, writeCB, getSizeCB)`: entry lookup; not found -> -1. Set +0xC, +0x10, +0x14;
  return `e->offset`.

## 0x808920 _rwPluginRegistryAddPlgnStrmlwysCB  (catalogue: specified/REA, port level 0)
- `RwInt32 (reg, id, alwaysCB)`: lookup; -1 if absent; `e->alwaysCB (+0x18) = cb`; return offset.

## 0x808950 _rwPluginRegistryAddPlgnStrmRightsCB  (catalogue: specified/REA, port level 0)
- `RwInt32 (reg, id, rightsCB)`: lookup; -1 if absent; `e->rightsCB (+0x1C) = cb`; return offset.

## 0x808980 _rwPluginRegistryReadDataChunks  (catalogue: specified/REA, port level 4)
- `const RwPluginRegistry* (reg, RwStream* s, void* obj)`; returns reg or NULL. Callers (11): RpClumpStreamRead,
  RpGeometryStreamRead, RpMaterialStreamRead, _rwFrameListStreamRead, ...
- Behaviour:
  1. `RwStreamFindChunk(s, 3 /*rwID_EXTENSION*/, &length, &version)`; 0 -> return NULL.
  2. Version outside [0x34000, 0x36003] -> error 0x80000004, return NULL (unreachable: FindChunk already rejected it).
  3. While `length != 0` (unsigned; the first test is `jbe`, later `jne` on the result of the subtraction, so an
     inconsistent length underflows and keeps reading): read a header `(s, &type, &chunkLen, NULL, NULL)`; fail ->
     return NULL. Lookup `type`; if found and `readCB != NULL`: `r = readCB(s, chunkLen, obj, e->offset, e->size)`,
     r == NULL -> return NULL. Otherwise `RwStreamSkip(s, chunkLen)`, NULL -> return NULL.
     `length += -12 - chunkLen`.
  4. For each entry from first: if `alwaysCB != NULL` and `alwaysCB(obj, offset, size) == 0` -> return NULL.
  5. Return reg.
- No extension chunk (step 1 fails) means the always callbacks are NOT run.
- librw: plg.cpp:PluginList::streamRead differs (ignores read/always callback results, loop `while (length > 0)` signed).

## 0x808AB0 _rwPluginRegistryInvokeRights  (catalogue: specified/REA, port level 0)
- `const RwPluginRegistry* (reg, RwUInt32 id, void* obj, RwUInt32 extraData)`: lookup id; absent or `rightsCB == NULL`
  -> return NULL; else `return rightsCB(obj, e->offset, e->size, extraData) ? reg : NULL`.
- librw: plg.cpp:PluginList::assertRights differs (void).

## 0x808C10 _rwPluginRegistrySkipDataChunks  (catalogue: specified/REA, port level 4)
- `const RwPluginRegistry* (reg, RwStream* s)`; caller RwTextureStreamRead 0x8046E0.
- Behaviour: `RwStreamFindChunk(s, 3, &length, NULL)`; 0 -> return NULL. While `length != 0`: read header `(s, NULL,
  &chunkLen, NULL, NULL)` (fail -> NULL); `RwStreamSkip(s, chunkLen)` (NULL -> NULL); `length += -12 - chunkLen`.
  Return reg.
- librw: plg.cpp:PluginList::streamSkip matches (void there).

## Open questions
- None.
