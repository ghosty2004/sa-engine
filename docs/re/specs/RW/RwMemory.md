# RenderWare 3.6 freelists, memory and resource arena (RW core) - full specs, writer A (RE-SPEC-RW)

RenderWare 3.6 code linked into gta_sa.exe 1.0 US; gta-reversed/plugin-sdk have thunks only. All `__cdecl`.
`G` = RwGlobals (`[0xC97B24]`): G+0x134 rwmalloc(size, hint), +0x138 rwfree(p), +0x13C rwrealloc, +0x140 rwcalloc,
+0x144 memoryAlloc(freelist, hint), +0x148 memoryFree(freelist, p), +0x154 resArenaInitSize.

## Layouts
`RwFreeList` (0x24): +0x0 entrySize (aligned), +0x4 entriesPerBlock, +0x8 heapSize (bitmap bytes = (entriesPerBlock
+ 7) >> 3), +0xC alignment, +0x10 blockList head (next +0x10, prev +0x14), +0x18 flags (1 rwFREELISTFLAG_STATIC, 2
rwFREELISTFLAG_FREEBLOCKS), +0x1C link.next, +0x20 link.prev (in the global freelist list) (0x801A4D..0x801AEC).
Freelist block (one malloc): +0x0 next, +0x4 prev (in blockList), +0x8 bitmap[heapSize] (bit `0x80 >> (i & 7)` of
byte `i >> 3` = entry i in use), entries start at `align_up(block + 8 + heapSize, alignment)` computed as
`(block + alignment + heapSize + 7) & ~(alignment - 1)` (0x801CAC..0x801CB6); block size
`entriesPerBlock * entrySize + alignment + heapSize + 7` (0x801A67..0x801A6C).

Globals: 0xC9A604 global freelist list head (next 0xC9A604, prev 0xC9A608); 0xC9A60C static RwFreeList used for the
freelist-of-freelists; 0xC9A630 memory-open flag; 0xC9A634 freelist of RwFreeList structs; 0x8E266C freelist
preallocation flag (.data init 1).

`RwResEntry` (0x18 header before the data): +0x0 link.next, +0x4 link.prev, +0x8 size, +0xC owner, +0x10 ownerRef,
+0x14 destroyNotify (0x808047..0x808054).
RESOURCESMODULE (engine plugin 0x40B, 0x28 bytes), offset global 0xC9BC58, module `R = G + [0xC9BC58]`:
+0x0 maxSize (= resArenaInitSize), +0x4 currentSize, +0x8 reusageSize, +0xC arena memory, +0x10 list A head,
+0x18 list B head, +0x20 `RwLinkList* older` (initially &B), +0x24 `RwLinkList* current` (initially &A)
(0x807D45..0x807D64). Instance counter 0xC9BC5C.
Arena: header at the arena memory start: +0x0 first block, +0x4 free hint. Arena block header (0x20 bytes, data at
+0x20): +0x0 arena, +0x4 next, +0x8 prev, +0xC data size, +0x10 inUse (bit 0) (0x80FFFF..0x810016).

## 0x801970 _rwFreeListSetFlags  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 flag)`: `[0x8E266C] = flag`. Caller RwEngineInit (0 when freelists are disabled, else 1).
  When 0, `_rwFreeListCreateInternal` preallocates no blocks.

## 0x801980 _rwFreeListCreate  (catalogue: specified/REA, port level 1)
- `RwFreeList* (RwInt32 entrySize, RwInt32 entriesPerBlock, RwInt32 alignment, RwUInt32 hint)` ->
  `_rwFreeListCreateInternal(entrySize, entriesPerBlock, alignment, 1, NULL, hint)`.

## 0x8019B0 _rwFreeListCreateInternal  (catalogue: specified/REA, port level 0)
- `RwFreeList* (entrySize, entriesPerBlock, alignment, numBlocksToPreallocate, void* inPlaceSpace, RwUInt32 hint)`.
- Behaviour:
  1. If `[0x8E266C] == 0`: numBlocks = 0. `align = alignment ? alignment : 4`.
  2. If inPlaceSpace == NULL: `fl = [0xC9A634] ? G->memoryAlloc([0xC9A634], hint & 0xFF0000) : G->rwmalloc(0x24, hint &
     0xFF0000)`; NULL -> return NULL; `fl->flags = 2`. Else `fl = inPlaceSpace; fl->flags = 3`.
  3. `fl->entrySize = (entrySize + align - 1) & ~(align - 1); entriesPerBlock; heapSize = (entriesPerBlock + 7) >> 3;
     alignment = align`; blockList empty.
  4. Repeat numBlocks times (unsigned count): `b = G->rwmalloc(blockSize, hint)` (full hint); NULL -> cleanup (5).
     `b->next = b->prev = 0`, then insert b at the head of blockList; zero the bitmap.
  5. Cleanup on failure: free every block in blockList (unlink head.next, `G->rwfree`); if `!(flags & 1)`: if
     `[0xC9A634] != fl && [0xC9A634] != NULL` -> `G->memoryFree([0xC9A634], fl)` else `G->rwfree(fl)`; return NULL.
  6. Insert `&fl->link` at the head of the global list 0xC9A604; return fl.
- librw: absent.

## 0x801B70 RwFreeListCreateAndPreallocateSpace  (catalogue: specified/REA, port level 1)
- `jmp 0x8019B0`: same arguments as `_rwFreeListCreateInternal`. Callers (20): every module Open.

## 0x801B80 RwFreeListDestroy  (catalogue: specified/REA, port level 0)
- `RwBool (RwFreeList* fl)`: unlink `fl->link` from the global list; free every block (`G->rwfree`); if `!(flags & 1)`:
  if `[0xC9A634] != fl && [0xC9A634] != NULL` -> `G->memoryFree([0xC9A634], fl)` else `G->rwfree(fl)`. Returns 1.
  Entries still in use are not checked.

## 0x801C30 _rwFreeListAllocReal  (catalogue: specified/REA, port level 0)
- `void* (RwFreeList* fl, RwUInt32 hint)`; installed as G+0x144 by RwEngineInit.
- Behaviour: for each block b (from blockList.next): `rem = entriesPerBlock`; for byte i = 0..heapSize-1: if
  `bitmap[i] == 0xFF` -> `rem -= 8`, next byte; else for bit j = 0..7 while `rem > 0`: if bit `0x80 >> j` clear ->
  set it, return `entries(b) + (i*8 + j) * entrySize`; else `rem--`. (rem reaching 0 ends that byte.)
  No free slot: `b = G->rwmalloc(blockSize, hint)`; NULL -> return NULL; zero bitmap; insert b at the HEAD of
  blockList; `bitmap[0] = 0x80`; return `entries(b)`.
- Note: blocks are searched from the list head, and new blocks go to the head.
- librw: absent.

## 0x801D50 _rwFreeListFreeReal  (catalogue: specified/REA, port level 0)
- `RwFreeList* (RwFreeList* fl, void* p)`; returns fl, or NULL if p is in no block (no error).
- Behaviour: find the first block with `start <= p <= start + entriesPerBlock*entrySize` where `start = b + heapSize + 8`
  (the UNALIGNED start; upper bound inclusive, unsigned compares 0x801D6D/0x801D7B); `idx = (p - start) / entrySize`
  (unsigned div, padding < alignment <= entrySize keeps it exact); clear bit `0x80 >> (idx & 7)` of byte `idx >> 3`.
  If `fl->flags & 2`: sum the bitmap bytes; if 0 -> unlink the block and `G->rwfree(b)`. Return fl.

## 0x801E00 RwFreeListPurge  (catalogue: specified/REA, port level 0)
- `RwInt32 (RwFreeList* fl)`: walk the blocks (next pointer taken after unlinking, i.e. the original next): unlink b;
  if its bitmap is all zero -> `G->rwfree(b)`, `count++`; else re-insert b at the head of blockList (kept blocks end up
  in reverse order). Returns `fl->entrySize * count` (0x801E7F..0x801E82; not the number of bytes freed).
- librw: absent.

## 0x801F90 RwFreeListPurgeAllFreeLists  (catalogue: specified/REA, port level 1)
- `RwInt32 (void)`: for each freelist in the global list (link - 0x1C): `r = RwFreeListPurge(fl)`; `if (r > 0) total += r`.
  Returns total. Caller D3D9DeviceReleaseVideoMemory.

## 0x801FD0 _rwMemoryOpen  (catalogue: specified/REA, port level 1)
- `RwBool (const RwMemoryFunctions* funcs)`; caller RwEngineInit.
- Behaviour: global list 0xC9A604 = empty; `[0xC9A630] = 1`; `fl = _rwFreeListCreateInternal(0x24, 0x10, 4, 0,
  (void*)0xC9A60C, 0x40000)`; `[0xC9A634] = fl`; NULL -> `[0xC9A630] = 0`, return 0. Unlink fl's own link from the global
  list (the freelist-of-freelists is not listed). If funcs != NULL: copy its 4 dwords to G+0x134..G+0x140; else
  defaults G+0x134 = 0x8020A0 `_rwMemoryDefaultMalloc`, +0x138 = 0x82413F (CRT free), +0x13C = 0x8020B0 realloc,
  +0x140 = 0x8020D0 calloc. Return 1.

## 0x807C90 cb_RESOURCESMODULE_RwEngine_Open  (catalogue: specified/REA, port level 1)
- `void* Open(instance, offset, size)`.
- Behaviour: `[0xC9BC58] = offset`; `size0 = G->resArenaInitSize`; if `size0 > 0`: `R+0xC = G->rwmalloc(size0, 0x4040B)`;
  NULL -> error {1, _rwerror(0x80000013, size0)}, return NULL; `_rwResourcesArenaInit(mem, size0) == 0` -> `G->rwfree(mem)`,
  error {1, _rwerror(0xC E_RW_INSUFFICIENTRESOURCES, 0)}, return NULL. If size0 == 0: `R+0xC = NULL` (a later
  allocation would dereference NULL in `_rwResourcesArenaFindFree`). Lists A and B empty; `R+0x24 = &A; R+0x20 = &B;
  R+0 = size0; R+4 = R+8 = 0`; `[0xC9BC5C]++`; return instance.

## 0x807DE0 RwResourcesFreeResEntry  (catalogue: specified/REA, port level 1)
- `RwBool (RwResEntry* e)`: if `e->destroyNotify` call it(e); if `e->ownerRef` `*e->ownerRef = NULL`; if
  `e->link.next != NULL`: unlink, `R->currentSize -= e->size`, `_rwResourcesArenaFreeBlock(e)`; else `G->rwfree(e)`.
  Returns 1. Callers (21): _rpAtomicDestructor, _rpClumpDestructor, ...

## 0x807E50 _rwResourcesPurge  (catalogue: specified/REA, port level 0)
- `void (void)`; caller RwRasterShowRaster 0x7FB1A0 (once per frame). Name from the catalogue; behaviour = per-frame
  list rotation:
  `cur = R+0x24, old = R+0x20`; if old is non-empty: append all of old's entries to the END of cur (if cur is empty,
  cur takes them in order), old becomes empty. Then swap: `R+0x24 = old` (now empty), `R+0x20 = cur`. `R+8 = 0`.
  Entries are not freed.

## 0x807ED0 RwResourcesAllocateResEntry  (catalogue: specified/REA, port level 2)
- `RwResEntry* (void* owner, RwResEntry** ownerRef, RwInt32 size, RwResEntryDestroyNotify destroyNotify)`;
  callers: `_rpD3D9InstanceMeshes` 0x756960, `_rpD3D9SkinInstanceMeshes` 0x7C8D60.
- Behaviour: loop:
  1. `e = _rwResourcesArenaFindFree(R+0xC, size + 0x18)`; non-NULL -> step 4.
  2. If list `R+0x20` (older) is non-empty: take its LAST entry; destroy it (destroyNotify, `*ownerRef = NULL`, if linked:
     unlink, `currentSize -= size`, arena free, else `G->rwfree`); retry.
  3. Else if list `R+0x24` (current) is non-empty: take its last entry, `R->reusageSize += e->size`, destroy it the same
     way; retry. Else: if ownerRef `*ownerRef = NULL`; error {1, _rwerror(0xC, size)}; return NULL.
  4. Insert e at the head of list `R+0x24`; `e->owner = owner; e->size = size; e->ownerRef = ownerRef;
     e->destroyNotify = destroyNotify`; `R->currentSize += size`; if ownerRef `*ownerRef = e`. Return e.
- librw: absent.

## 0x8082E0 _rwResourcesOpen  (catalogue: unknown/REA, port level 2)
- Name mismatch: the bytes create the plugin-registry entry freelist, so this is RW's `_rwPluginRegistryOpen`
  (proposed name); RwEngineInit calls it right after the file system open.
- `RwBool (void)`: `[0xC9A6E4] = RwFreeListCreateAndPreallocateSpace(0x3C, [0x8E26AC] (init 0x40), 4, [0x8E26B0]
  (init 1), (void*)0xC9A6C0, 0x40000)`; NULL -> return 0; `[0xC9A6EC] = 0`; return 1.

## 0x80FFE0 _rwResourcesArenaInit  (catalogue: specified/REA, port level 0)
- `RwBool (void* mem, RwUInt32 size)`: `blk = (mem + 0x27) & ~0x1F`; `avail = ((mem + size) & ~0x1F) - blk - 0x20`;
  if `avail < 0x20` (signed) return 0. `blk = {arena = mem, next = prev = NULL, size = avail, inUse = 0}`;
  `mem[0] = mem[1] = blk`; return 1.

## 0x810040 _rwResourcesArenaFreeBlock  (catalogue: specified/REA, port level 0)
- `void (void* data)`: `b = data - 0x20; arena = b->arena; next = b->next; prev = b->prev; b->inUse = 0`.
  If `arena->hint == NULL || b < arena->hint` -> `arena->hint = b`. If prev and prev free: `prev->next = next`,
  `next->prev = prev` (if next), `prev->size += b->size + 0x20`, cur = prev; else cur = b. If next and next free:
  `cur->next = next->next`, `next->next->prev = cur` (if any), `cur->size += next->size + 0x20`.

## 0x8100C0 _rwResourcesArenaFindFree  (catalogue: unknown/REA, port level 1)
- `void* (Arena* arena, RwUInt32 size)`: `need = (size + 0x1F) & ~0x1F`; first block from `arena->hint` (via next) with
  `!(inUse & 1) && blk->size >= need`; none -> NULL. If `blk->size > need + 0x40`: `_rwResourcesArenaSplitBlock(blk,
  need)`. If `blk == arena->hint`: advance hint to the next block that is not in use (or NULL). `blk->inUse = 1`;
  return `blk + 0x20`.

## 0x810140 _rwResourcesArenaSplitBlock  (catalogue: unknown/REA, port level 0)
- `ArenaBlock* (ArenaBlock* b, RwUInt32 size)`: `n = b + 0x20 + size`; if `b->next` is non-NULL and free: absorb it
  (`n->next = b->next->next`, `n->size = b->next->size - size + b->size`); else `n->next = b->next`,
  `n->size = b->size - size - 0x20`. `b->next = n; n->inUse = 0; n->prev = b`; `n->next->prev = n` (if any);
  `b->size = size; n->arena = b->arena`. Returns n.

## Open questions
- None.
