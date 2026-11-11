# RwFreeList / memory module helpers (RE-SPEC-RW-3 writer B)

All `__cdecl`. Pass-1 file `RwMemory.md` (read-only) covers freelist create/alloc/free/purge.

## Layout
- RwFreeList: +0x0 entrySize, +0x4 entriesPerBlock, +0x8 heapSize (bytes of the per-block usage bitmap), +0xc alignment, +0x10
  block list head (RwLinkList: next +0x10, prev +0x14), +0x18 flags (bit 0 = static storage, not freed), +0x1c/+0x20 link in the global
  freelist list (decompile of 0x801e90 / 0x8020f0; instruction addresses below).
- Block: +0x0/+0x4 list link, +0x8 bitmap (heapSize bytes, bit 0x80 of byte i = entry 8*i), entries from
  `(block + heapSize + alignment + 7) & ~(alignment - 1)`.
- `[0xc9a604]` head of the global list of freelists (link at freelist+0x1c); `[0xc9a634]` the freelist that holds RwFreeList
  structs; `[0xc9a630]` cleared at close.
- RwGlobals +0x134 memoryFuncs {malloc +0x134, free +0x138, realloc +0x13c, calloc +0x140}; +0x148 freelist free.

## 0x801c10 RwFreeListSetFlags  (catalogue: specified/REA, port level 0)
- `void (RwFreeList* fl, RwUInt32 flags)`: `fl+0x18 |= flags & 0xfffffffe` (bit 0 cannot be set; flags are only ever added).
- librw: absent.

## 0x801c20 RwFreeListGetFlags  (catalogue: specified/REA, port level 0)
- returns `fl+0x18`.
- librw: absent.

## 0x801e90 RwFreeListForAllUsed  (catalogue: specified/REA, port level 0)
- `RwFreeList* (RwFreeList* fl, RwFreeListCallBack cb, void* data)`. Empty block list -> return fl.
- For each block (next pointer read BEFORE the callbacks, so callbacks may free entries): `tmp = malloc(heapSize, 0x10000)` (+0x134);
  NULL -> return NULL. Copy the bitmap into tmp (snapshot). For byte i = 0 while i < heapSize (unsigned), if the byte is non-zero, for bit
  j = 0..7: if `byte & (0x80 >> j)` -> `cb(entries + (i*8 + j) * entrySize, data)` (callback result ignored). `free(tmp)` (+0x138).
  Returns fl.
- librw: absent.

## 0x8020b0 _rwMemoryDefaultRealloc  (catalogue: specified/REA, port level 0)
- `void* (void* p, size_t n, RwUInt32 hint)`: returns CRT `realloc(p, n)` 0x824269 (hint dropped).
- librw: absent.

## 0x8020d0 _rwMemoryDefaultCalloc  (catalogue: specified/REA, port level 0)
- `void* (size_t num, size_t size, RwUInt32 hint)`: returns CRT `calloc(num, size)` 0x824416.
- librw: absent.

## 0x8020f0 _rwMemoryClose  (catalogue: specified/REA, port level 0)
- `void (void)`. While the global list `[0xc9a604]` is not empty: take the first link L (freelist `fl = L - 0x1c`), unlink it; unlink
  and `free` (+0x138) every block of fl (head fl+0x10). If `fl+0x18 & 1 == 0` (dynamic): if `[0xc9a634] == fl` or `[0xc9a634] == NULL`
  -> `free(fl)` (+0x138) else `memoryFree (+0x148)([0xc9a634], fl)`.
- Then the freelist-of-freelists `F = [0xc9a634]` (dereferenced without a NULL check): unlink F+0x1c (it is no longer on the list,
  the unlink rewrites its stale neighbours), free all its blocks; if `F+0x18 & 1 == 0`: `free(F)`. Finally `[0xc9a634] = 0`,
  `[0xc9a630] = 0`.
- librw: absent.

## 0x802230 RwOsGetMemoryInterface  (catalogue: specified/REA, port level 0)
- returns `RwEngineInstance + 0x134` (pointer to RwMemoryFunctions).
- librw: absent.
