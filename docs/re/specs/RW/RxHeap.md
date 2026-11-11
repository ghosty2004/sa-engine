# RW PowerPipe execution heap RxHeap (RE-SPEC-RW-3 lead)

All `__cdecl`. Engine memory functions as in RxPipeline.md (Engine+0x134 malloc, +0x138 free, +0x13c realloc, +0xf0
stringFuncs.vecSprintf). Global heap pointer [0xc9a6b0] (created by _rxPipelineOpen with size [0x8e2698] = 4096).

Layouts (rwcore.h, proven by the instructions cited):
- RxHeap (0x1c bytes, malloc hint 0x40409 at 0x809faa): +0x0 superBlockSize, +0x4 head superblock descriptor, +0x8 headBlock,
  +0xc freeBlocks (array of 8-byte rxHeapFreeBlock), +0x10 entriesAlloced, +0x14 entriesUsed, +0x18 dirty (0x80a00e-0x80a020).
- Superblock descriptor = start of its own allocation of `size + 0x8b` bytes (hint 0x1040409): +0x0 start =
  `(desc + 0x8b) & ~0x7f` (128-byte aligned, `and al, 0x80` at 0x80a007 / 0x809b0f), +0x4 size, +0x8 next descriptor.
- Block header (0x20 bytes, user data follows it): +0x0 prev, +0x4 next, +0x8 size (user bytes, multiple of 32), +0xc freeEntry
  (rxHeapFreeBlock* or NULL = in use / sentinel), +0x10..+0x1f padding. rxHeapFreeBlock: +0x0 size, +0x4 header.
- Each superblock holds three headers after (re)initialisation: sentinel H0 at start, one free block H1 at start+0x20 with size
  `size - 0x60`, sentinel HL at start+size-0x20 (0x809bf0). HL of a superblock links to H0 of the next one processed.

librw: absent (no PowerPipe heap).

## 0x8098b0 RxHeapFree  (catalogue: specified/REA, port level 2)
- `void (RxHeap* heap [esp+4], void* p [esp+8])`. `H = p - 0x20`. P = H.prev, N = H.next; prevFree = P && P.freeEntry; nextFree = N && N.freeEntry.
- Both free (0x8098f3): remove N's free entry (copy the LAST used entry into it unless it is the last, fix that block's freeEntry back
  pointer, `entriesUsed -= 1`); `P.size += H.size + N.size + 0x40`; `P.freeEntry->size = P.size`; `P.next = N.next`; if N.next: `N.next->prev = P`.
- Only P free (0x809962): `P.size += H.size + 0x20`; `P.freeEntry->size = P.size`; `P.next = N`; if N: `N.prev = P`.
- Only N free (0x809994): `H.size += N.size + 0x20`; `H.freeEntry = N.freeEntry`; `freeEntry->header = H`; `freeEntry->size = H.size`;
  `H.next = N.next`; if non-NULL its prev = H.
- Neither (0x8099d0): `fe = 0x8099f0(heap)`; if fe: `fe->header = H`, `fe->size = H.size`, `H.freeEntry = fe` (on failure the block is lost).
- The dirty flag is not touched.

## 0x8099f0 sub_8099f0 (proposed: _rxHeapFreeBlockSlot, rw-names hint)  (catalogue: unknown/REA, port level 1)
- `rxHeapFreeBlock* (RxHeap* heap)`. If `entriesAlloced <= entriesUsed` (unsigned, 0x809a03): `entriesAlloced += 32`;
  `nf = Engine+0x13c(freeBlocks, entriesAlloced*8, 0x1030409)`; NULL -> `_rwerror(0x80000013, entriesAlloced*8)` (already increased),
  RwErrorSet, `entriesAlloced -= 32`, return NULL. If nf moved and entriesUsed > 0: for each used entry i: `entry_i.header->freeEntry = &nf[i]`
  (0x809a73); `freeBlocks = nf`.
- Returns `&freeBlocks[entriesUsed++]` (contents not initialised).

## 0x809aa0 RxHeapAlloc  (catalogue: specified/REA, port level 3)
- `void* (RxHeap* heap [esp+4], RwUInt32 size [esp+8])`. `need = (size + 31) & ~31`.
- Search the free list from the LAST entry down to entry 0 for the first `fe->size >= need` (unsigned, 0x809ac8). ORIGINAL BUG: with
  entriesUsed == 0 the loop starts at freeBlocks[-1] and, the counter wrapping, keeps walking backwards until some dword >= need.
- Found: `dirty = 1`; return `0x809c70(heap, fe, need)`.
- Not found: `newSize = max(need + 0x60, superBlockSize)`; `d = Engine+0x134(newSize + 0x8b, 0x1040409)`; NULL -> `_rwerror(0x80000013, need)`,
  RwErrorSet, return NULL. d = {start aligned as above, newSize, next 0}; `0x809bf0(d, heap->head, heap)` (links the old head's last
  sentinel to d); 0 -> free d, same error, NULL. `d->next = heap->head`; `heap->head = d`. Debug text: total = sum of sizes over the chain;
  `vecSprintf(stackbuf, 0x8e26b4, total - newSize, total)` (result unused). `dirty = 1`; return `0x809c70(heap, (d->start + 0x20)->freeEntry, need)`.

## 0x809bf0 sub_809bf0 (proposed: _rxHeapInitSuperBlock; rw-names hint _rxHeapAddBlock)  (catalogue: unknown/REA, port level 2)
- `RwBool (desc* d, desc* prev, RxHeap* heap)`. `fe = 0x8099f0(heap)`; NULL -> 0.
- `H0 = d->start`, `HL = d->start + d->size - 0x20`: H0 prev/next/size/freeEntry = 0; copy all 8 dwords of H0 to HL (padding included);
  `H1 = H0 + 0x20`; `H0.next = H1`; `H1.prev = H0`; `H1.next = HL`; `HL.prev = H1`; `H1.size = HL - H1 - 0x20`; `H1.freeEntry = fe`;
  `fe->header = H1`; `fe->size = H1.size`.
- If prev: `prevHL = prev->start + prev->size - 0x20`; `prevHL.next = H0`; `H0.prev = prevHL` (0x809c5b-0x809c63). Return 1.

## 0x809c70 sub_809c70 (proposed: _rxHeapSplitBlock, rw-names hint)  (catalogue: unknown/REA, port level 0)
- `void* (RxHeap* heap, rxHeapFreeBlock* fe, RwUInt32 need)`. `H = fe->header`; `rem = fe->size - need`.
- rem < 0x100 (unsigned): whole block used: remove fe (copy the last used entry into it unless it is the last, fix back pointer),
  `entriesUsed -= 1`; `H.freeEntry = 0`; return H + 0x20.
- Else split, the allocation takes the FRONT: `N = H + 0x20 + need`; `N.size = rem - 0x20`; `N.prev = H`; `N.next = H.next`;
  `N.freeEntry = fe`; `H.size = need`; `H.next = N`; if N.next: `N.next->prev = N`; `fe->header = N`; `fe->size = N.size`; `H.freeEntry = 0`;
  return H + 0x20.

## 0x809d10 RxHeapRealloc  (catalogue: specified/REA, port level 4)
- `void* (RxHeap* heap, void* p, RwUInt32 size)`. `need = (size + 31) & ~31`; `H = p - 0x20`; `delta = need - H.size` (signed test, 0x809d2f).
- delta > 0: if `N = H.next` is free and `N.size + 0x20 >= delta` (unsigned): `total = N.size + H.size + 0x20`; if `total - need < 0x100`:
  absorb N completely (remove its free entry as in 0x809c70, `H.size = total`, `H.next = N.next`, fix prev) and return p; else move N's
  header to `H + 0x20 + need` with size `N.size - delta`, same freeEntry (updated header/size), relink, `H.size = need`, return p.
  Otherwise `q = RxHeapAlloc(heap, need)` (the rounded size); NULL -> `_rwerror(0x80000013, need)`, RwErrorSet, NULL. Copy `H.size` bytes p -> q,
  `RxHeapFree(heap, p)`, return q.
- delta <= 0: `shrink = -delta`; if `shrink >= 0x100` (signed) and `fe = 0x8099f0(heap)` succeeds: new free header at `H + 0x20 + need` with size
  `shrink - 0x20`, inserted after H, `H.size = need`. The new free block is NOT merged with a following free block. Return p.

## 0x809ec0 _rxHeapReset  (catalogue: specified/REA, port level 3)
- `RwBool (RxHeap* heap)`. `entriesUsed = 0`. For d = head->next, head->next->next, ... (prev starts NULL): `0x809bf0(d, prev, heap)`; 0 -> return 0;
  for the first one `headBlock = d->start`; prev = d. Finally `0x809bf0(head, prev, heap)` (head goes LAST in the block chain); 0 -> return 0;
  if it was the only one `headBlock = head->start`. `dirty = 0`; return 1. All previous allocations become invalid.

## 0x809f30 RxHeapDestroy  (catalogue: specified/REA, port level 0)
- `void (RxHeap* heap)`: NULL -> nothing. Free freeBlocks (if set, then 0); free every superblock descriptor following +8 from head; free heap.

## 0x809f90 RxHeapCreate  (catalogue: specified/REA, port level 3)
- `RxHeap* (RwUInt32 size [esp+4])`. `size = max(size, 0x400)`; `heap = Engine+0x134(0x1c, 0x40409)`; NULL -> NULL.
  `size = (size + 0x1f) & ~0x1f`, then max(size, 0x80) (dead after the 0x400 floor). `d = Engine+0x134(size + 0x8b, 0x1040409)`; NULL -> free heap, NULL.
  d = {aligned start, size, next 0}; heap = {superBlockSize = size, head = d, freeBlocks 0, entriesAlloced 0, dirty 1, entriesUsed 0}.
- Same initialisation loop as _rxHeapReset (no successors here): `0x809bf0(d, NULL, heap)`; 0 -> free d and heap, NULL. `headBlock = d->start`;
  `dirty = 0`; return heap.

