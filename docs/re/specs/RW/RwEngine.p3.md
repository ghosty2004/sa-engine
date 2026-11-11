# RwEngine: engine control, resources arena, plugin registry, SList, strings (RE-SPEC-RW-3 writer B)

All `__cdecl`. Pass-1 files `RwEngine.md`, `RwPluginRegistry.md` (read-only) cover init/open/start and plugin registration.

## Layout
- `RwEngineInstance` [0xc97b24]; static engine block 0xc979c8 (0x56 dwords = 0x158 bytes copied back at close, 0x7f2f27-0x7f2f3a).
  RwGlobals +0x10 RwDevice (passed to `_rwDeviceSystemRequest` 0x7f2ab0 as `engine+0x10`), +0x134 malloc, +0x138 free, +0x144/+0x148
  freelist alloc/free, +0x14c metrics, +0x150 engineStatus (0 none, 1 initialised, 2 opened, 3 started), +0x154 resArenaInitSize.
  `[0xc97b20]` engine instance count (0x7f2f51, 0x7f3130). `[0xc97b00]` = static block +0x138 (free function of the static copy).
- Device requests used: 1 CLOSE, 3 STOP?, 8 FOCUS, 0xc GETTEXMEMSIZE, 0x12 FINALIZESTART-pair (called before stop), 0x13 GETMAXTEXTURESIZE
  (numbers from the pushes at 0x7f2f0f, 0x7f2e4e, 0x7f2df2, 0x7f2d93, 0x7f2e2f, 0x7f2dc3; RW names per d3d9.md section 1).
- Matrix module data engine + `[0xc979bc]`: +0xc RwMatrixTolerance {normal, orthogonal, identity} (3 f32, 0x7f178b).
- Resources module `m = engine + [0xc9bc58]`: +0x0 arena max size (0x8080fb), +0x4 bytes in use (decremented 0x80825f), +0x8 extra
  usage counter (0x8081cc, zeroed 0x8082d2), +0xc arena memory (0x80812f), +0x10/+0x14 used list head (next/prev), +0x18/+0x1c second
  list head, +0x24 pointer to the arena block list (0x8081d0). `[0xc9bc5c]` module open count (0x8080c0). RwResEntry: +0x0 next,
  +0x4 prev, +0x8 size, +0x10 owner reference (`*ref = NULL` on free), +0x14 destroy-notify callback (0x808223-0x808237).
- Plugin registry: registry +0x0 sizeOfStruct, +0x4 origSizeOfStruct, +0x8 maxSizeOfStruct, +0xc staticAlloc, +0x10 first entry, +0x14
  last entry; entry +0x30 next, +0x38 parent registry (0x808371, 0x80836e). `[0xc9a6e4]` registry-entry freelist, `[0xc9a6e8]` array of
  registries, `[0xc9a6ec]` its count (plugins.md).

## 0x7f1780 RwEngineGetMatrixTolerances  (catalogue: specified/REA, port level 0)
- `RwBool (RwMatrixTolerance* out)`: copies the 3 dwords at matrix module +0xc to out; returns 1.
- librw: absent.

## 0x7f17b0 RwEngineSetMatrixTolerances  (catalogue: specified/REA, port level 0)
- `RwBool (const RwMatrixTolerance* in)`: copies 3 dwords into matrix module +0xc (no validation); returns 1.
- librw: absent.

## 0x7f2d80 RwEngineGetTextureMemorySize  (catalogue: specified/REA, port level 2)
- `RwInt32 (void)`: `_rwDeviceSystemRequest(engine+0x10, 0xc, &out, 0, 0)`; returns out if the request returned non-zero, else -1.
- librw: absent.

## 0x7f2db0 RwEngineGetMaxTextureSize  (catalogue: specified/REA, port level 2)
- Same with request 0x13 (D3D9: max(caps MaxTextureWidth, Height)); -1 on failure.
- librw: absent.

## 0x7f2de0 RwEngineSetFocus  (catalogue: specified/REA, port level 2)
- `RwBool (RwBool enable)`: `_rwDeviceSystemRequest(engine+0x10, 8, NULL, NULL, enable)`; returns `result != 0`.
- librw: absent.

## 0x7f2e10 RwEngineGetMetrics  (catalogue: specified/REA, port level 0)
- returns RwGlobals+0x14c.
- librw: absent.

## 0x7f2e20 RwEngineStop  (catalogue: specified/REA, port level 2)
- `RwBool (void)`: `_rwDeviceSystemRequest(engine+0x10, 0x12, 0, 0, 0)` (result ignored, 0x7f2e32); `_rwPluginRegistryDeInitObject(
  &[0x8e2298] engine registry, engine)`; `r = _rwDeviceSystemRequest(engine+0x10, 3, 0, 0, 0)`; if r != 0 -> engineStatus (+0x150) = 2
  (0x7f2e64). Returns r (the raw request result, not normalised).
- librw: engine.cpp `Engine::stop` differs (no device pre-request, returns bool).

## 0x7f2f00 RwEngineClose  (catalogue: specified/REA, port level 2)
- `RwBool (void)`: `r = _rwDeviceSystemRequest(engine+0x10, 1 CLOSE, 0, 0, 0)`; r == 0 -> return 0. Else: `old = RwEngineInstance`;
  `RwEngineInstance = 0xc979c8`; copy 0x56 dwords from old to 0xc979c8 (0x7f2f3a); `[0xc97b00](old)` (free of the heap engine block via
  the static copy's free function, 0x7f2f3c); `[0xc97b20]--`; static engine +0x150 = 1. Returns r.
- librw: engine.cpp `Engine::close` differs (librw frees its engine struct without copying back).

## 0x7f3130 RwEngineTerm  (catalogue: specified/REA, port level 2)
- `RwBool (void)`: `ok = ([0xc97b20] == 0)` (`sete`, 0x7f313a); if ok: `0x808320()` (plugin registry close, below), `0x804240()`,
  `_rwMemoryClose()` 0x8020f0, engineStatus = 0. Returns ok. (Calling Term with an engine still open returns 0 and does nothing.)
- librw: engine.cpp `Engine::term` differs (no open-engine check).

## 0x807d80 cb_RESOURCESMODULE_RwEngine_Close  (catalogue: specified/REA, port level 2)
- `void* (void* instance, RwInt32, RwInt32)`: `RwResourcesEmptyArena()` 0x8081f0; `0x810030(m+0xc)` (arena shutdown); `free(m+0xc)`
  (+0x138); `m+0xc = 0`; `[0xc9bc5c]--`. Returns instance.
- librw: absent.

## 0x8080c0 RwResourcesSetArenaSize  (catalogue: specified/REA, port level 2)
- `RwBool (RwUInt32 size)`. Module not open (`[0xc9bc5c] == 0`): engine+0x154 = size, return 1.
- Open: `m+0 = size`; `RwResourcesEmptyArena()`; `0x810030(m+0xc)`; `free(m+0xc)`; `m+0xc = malloc(size, 0x3040b)` (+0x134).
  NULL -> `m+0 = 0`, error `{1, _rwerror(0x80000013, size)}`, return 0. `_rwResourcesArenaInit(mem, size)` 0x80ffe0 == 0 -> `free(mem)`
  (m+0xc is left pointing to the freed block), error `{1, _rwerror(0xc, 0)}`, return 0. Else 1.
- librw: absent.

## 0x8081b0 RwResourcesGetArenaSize  (catalogue: specified/REA, port level 0)
- returns m+0x0.
- librw: absent.

## 0x8081c0 RwResourcesGetArenaUsage  (catalogue: specified/REA, port level 0)
- `RwInt32 (void)`: `sum = m+0x8`; walk the list whose head pointer is m+0x24 (`next` +0, until back at the head) adding each node's
  +0x8 (0x8081db-0x8081e4). Returns sum.
- librw: absent.

## 0x8081f0 RwResourcesEmptyArena  (catalogue: specified/REA, port level 1)
- `RwBool (void)`. Splice: `*(m+0x14 tail of list A) = m+0x18 first of list B` (0x8081fd-0x808205), then walk from `m+0x10` until the
  node equals the address m+0x18 (the second head acts as the terminator), next pointer read before processing (0x808221). Per entry e:
  if `e+0x14` destroy callback -> `cb(e)`; if `e+0x10` owner ref -> `*ref = NULL`; if `e+0x0` (next) == 0 -> `free(e)` (+0x138) else
  unlink e (`prev->next = next; next->prev = prev`), `m+4 -= e+8`, `_rwResourcesArenaFreeBlock(e)` 0x810040.
- Then reset both heads to empty (`m+0x10 = m+0x14 = &m+0x10`, `m+0x18 = m+0x1c = &m+0x18`) and `m+0x8 = 0`. Returns 1.
- Oddity: an entry with next == 0 would also make the walk continue from NULL (0x808278); presumably never happens.
- librw: absent.

## 0x808320 sub_808320 (proposed: _rwPluginRegistryClose)  (catalogue: unknown/REA, port level 1)
- Evidence: called from `RwEngineTerm` 0x7f3144; frees the registry-entry freelist `[0xc9a6e4]` and the registry array `[0xc9a6e8]`.
- `RwBool (void)`: if `[0xc9a6e4]` == NULL -> 1. `RwFreeListForAllUsed([0xc9a6e4], 0x8083f0, [0xc9a6e4])` 0x801e90.
  If `RwGlobals+0x144 != 0x801c30` (`_rwFreeListAllocReal`, 0x808344): for i < `[0xc9a6ec]` (unsigned): `reg = array[i]`; `e = reg+0x10`;
  if e: `p = e+0x38` (parent), then free every entry along `+0x30` with `memoryFree(NULL, e)` (+0x148, freelist argument 0); if p and
  `p+0x10 != 0`: `p+0x10 = 0`, `p+0x0 = p+0x4`, `p+0x14 = 0`. Then `free([0xc9a6e8])` (+0x138), `[0xc9a6e8] = 0`.
- `RwFreeListDestroy([0xc9a6e4])`, `[0xc9a6e4] = 0`. Returns 1.
- librw: absent.

## 0x8083f0 sub_8083f0 (proposed: _rwPluginRegistryEntryFreeCB)  (catalogue: unknown/REA, port level 0)
- Evidence: passed as callback to `RwFreeListForAllUsed` by 0x808320 (0x808331).
- `void (RwPluginRegEntry* e, RwFreeList* fl)`: `reg = e+0x38`; if `reg+0x10 != 0`: `reg+0x0 = reg+0x4` (size back to base), `reg+0x10 = 0`,
  `reg+0x14 = 0`. `memoryFree (+0x148)(fl, e)`. Return value: garbage (EAX from the free call).
- librw: absent.

## 0x808430 _rwPluginRegistrySetStaticPluginsSize  (catalogue: specified/REA, port level 0)
- `RwBool (RwPluginRegistry* r, RwInt32 n)`: if `r+0 != r+4` (plugins already registered) -> 0. Else if `r+8 <= 0` (signed) or `n < r+8`:
  `r+8 maxSize = r+0 + n` (0x808457); (n >= current max leaves it). `r+0xc staticAlloc = 1`. Returns 1.
- librw: absent.

## 0x808b00 _rwPluginRegistryGetSize  (catalogue: specified/REA, port level 0)
- `RwInt32 (const RwPluginRegistry* r, const void* obj)`: `total = 0`; for each entry from r+0x10 along +0x30: if `e+0x14 getSizeCB`:
  `s = cb(obj, e+0x0 offset, e+0x4 size)`; if `s > 0` (signed) -> `total += s + 0xc` (0x808b2c). Returns total (extension chunk header
  not included).
- librw: plg.cpp `PluginList::streamGetSize` matches.

## 0x808b40 _rwPluginRegistryWriteDataChunks  (catalogue: specified/REA, port level 3)
- `const RwPluginRegistry* (const RwPluginRegistry* r, RwStream* s, const void* obj)`. Total as in 0x808b00 (recomputed inline,
  0x808b55-0x808b76). `_rwStreamWriteVersionedChunkHeader(s, 3 rwID_EXTENSION, total, 0x36003, 0xffff)`; 0 -> NULL (an empty extension
  chunk of size 0 is always written).
- For each entry with BOTH getSizeCB (+0x14) and writeCB (+0x10): `n = getSize(obj, off, size)`; if `n > 0`: header `(s, e+0x8 pluginID,
  n, 0x36003, 0xffff)` (0 -> NULL), then `writeCB(s, n, obj, off, size)` (0x808bef); NULL -> return NULL. Returns r (the registry
  pointer, 0x808c00), i.e. non-NULL on success.
- Entries with a size callback but no write callback are counted in the header total but not written (header/body mismatch; none in
  the game presumably).
- librw: plg.cpp `PluginList::streamWrite` matches (librw returns bool).

## 0x809240 _rwSListGetNewEntry  (catalogue: specified/REA, port level 1)
- `void* (RwSList* l /*entries +0, numElements +4, capacity +8, entrySize +0xc*/, RwUInt32 hint)`: if `num >= cap` (signed):
  `newCap = cap + cap/4` (signed division truncating toward zero: `cdq; and edx,3; add; sar 2`, 0x809257-0x809268); `p = realloc(entries,
  newCap * entrySize, hint)` (+0x13c); NULL -> `RwErrorSet({1, _rwerror(0x80000013, newCap*entrySize)})`, NULL. Else entries = p,
  cap = newCap. Returns `entries + num*entrySize` and `num++`.
- Edge: capacity 0..3 does not grow (`cap/4 == 0`), so the returned entry lies past the buffer (original bug; RW creates SLists with
  a non-trivial initial capacity).
- librw: absent.

## 0x809400 _rwSListDestroyEndEntries  (catalogue: specified/REA, port level 0)
- `RwSList* (RwSList* l, RwInt32 n)`: `l+4 -= n`; EAX = l.
- librw: absent.

## 0x809440 _rwSListDestroy  (catalogue: specified/REA, port level 0)
- `RwBool (RwSList* l)`: if `entries != NULL && capacity != 0`: `free(entries)`, entries = 0, capacity = 0; `free(l)` (+0x138). Returns 1.
- librw: absent.

## 0x8094b0 _rwSListGetNumEntries  (catalogue: specified/REA, port level 0)
- returns l+4.
- librw: absent.

## 0x809510 _rwSListGetEntry  (catalogue: specified/REA, port level 0)
- returns `l+0 + i * l+0xc` (no bounds check).
- librw: absent.

## 0x809530 _rwSListGetBegin  (catalogue: specified/REA, port level 0)
- returns l+0.
- librw: absent.

## 0x809540 _rwSListGetEnd  (catalogue: specified/REA, port level 0)
- returns `l+0 + l+4 * l+0xc`.
- librw: absent.

## 0x80a350 _rwStringCompareNoCase  (catalogue: specified/REA, port level 0)
- `RwInt32 (const RwChar* a, const RwChar* b)`: either NULL -> 0 (equal). Loop: lower-case each char if in 'A'..'Z' (signed compares
  0x41/0x5a, `add 0x20`); differ -> `(i8)la - (i8)lb` (0x80a38e-0x80a394); equal: stop with 0 when either char is 0.
- librw: absent.

## 0x80a3a0 _rwStringToUpper  (catalogue: specified/REA, port level 0)
- `RwChar* (RwChar* s)`: in place, 'a'..'z' -= 0x20; NULL or empty -> unchanged. Returns s.
- librw: absent.

## 0x80a3d0 _rwStringToLower  (catalogue: specified/REA, port level 0)
- Same with 'A'..'Z' += 0x20. Returns s.
- librw: absent.

## 0x80a400 _rwStringFindChar  (catalogue: specified/REA, port level 0)
- `RwChar* (const RwChar* s, RwChar c)`: first position with `*p == c` (compared before the terminator test, so c == 0 returns the
  terminator); none -> NULL.
- librw: absent.

## 0x80a420 _rwStringFindCharReverse  (catalogue: specified/REA, port level 0)
- `RwChar* (const RwChar* s, RwChar c)`: last position with `*p == c` scanning to and including the terminator; none -> NULL.
- librw: absent.

## 0x80a450 _rwStringDestroy  (catalogue: specified/REA, port level 0)
- `RwBool (RwChar* s)`: `free(s)` (+0x138); returns 1.
- librw: absent.

## 0x80a470 _rwStringStreamGetSize  (catalogue: specified/REA, port level 0)
- `RwUInt32 (const RwChar* s)`: `s = s ? s : ""` (.rdata 0x884f6a, empty string); returns `(rwstrlen(s) (+0x120) + 4) & ~3`.
- librw: base.cpp/plg.cpp string size helper matches (`strlen+4 & ~3`).

## 0x80a4a0 _rwStringStreamWrite  (catalogue: specified/REA, port level 3)
- `const RwChar* (const RwChar* s, RwStream* st)`: `s = s ? s : ""`; `n = (strlen(s) + 4) & ~3`; `_rwStreamWriteVersionedChunkHeader(st,
  2 rwID_STRING, n, 0x36003, 0xffff)`; 0 -> NULL. `RwStreamWrite(st, s, n)`: writes n bytes straight from the string, i.e. the
  terminator plus up to 3 bytes that FOLLOW the string in memory (not zero padding). Returns s if written, else NULL.
- librw: absent as such (librw `writeString` zero-pads into a buffer: padding bytes differ).

## 0x80a7e0 cb_CHUNKGROUPMODULE_RwEngine_Close  (catalogue: specified/REA, port level 1)
- `void* (void* instance, RwInt32, RwInt32)`: if module (engine + `[0xc9a6f8]`) +0 freelist != NULL -> `RwFreeListDestroy` (pointer not
  cleared); `[0xc9a6fc]--`. Returns instance.
- librw: absent.

## 0x80e9b0 _rwFileNameIsAbsolute  (catalogue: specified/REA, port level 0)
- `RwBool (const RwChar* name)`: 1 if `name[0] == '\\'` or (`name[0]` is an ASCII letter and `name[1] == ':'`); else 0. ('/' is not
  absolute.)
- librw: absent.
