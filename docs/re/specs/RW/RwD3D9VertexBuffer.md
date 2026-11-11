# RW D3D9 driver: vertex/index buffer managers (RE-SPEC-RW-2)

All `__cdecl`. Allocations go through `RwEngineInstance` `[0xc97b24]` freelist slots: `+0x144` = `RwFreeListAlloc(list,
hint)`, `+0x148` = `RwFreeListFree(list, entry)` (used at 0x7f5530, 0x7f55d6). `_rwFreeListCreate` 0x801980 is
`(entrySize, entriesPerBlock, alignment, hint)` (argument order from the pushes at 0x7f5cb5-0x7f5cbe).
COM offsets: IDirect3DDevice9 vtbl+0x68 CreateVertexBuffer, IDirect3DVertexBuffer9 vtbl+0x2c Lock, +0x30 Unlock, +8 Release.

## Layout: static vertex-buffer manager
- `[0xc97b7c]` list of stride nodes, entries from freelist `[0xc97b80]` (0x10 bytes): +0x0 u32 stride (0x7f5514), +0x4 free-block
  list head (0x7f5552), +0x8 created-VB record list (0x7f5621), +0xc next (0x7f5518).
- Free block, freelist `[0xc97b84]` (0x14 bytes): +0x0 u32 byte offset, +0x4 u32 byte size (0x7f555d), +0x8
  `IDirect3DVertexBuffer9*` (0x7f55b2), +0xc next, +0x10 prev (0x7f55e9-0x7f55fa).
- VB record, freelist `[0xc97b88]` (8 bytes): +0x0 vertex buffer, +0x4 next (0x7f561c-0x7f5624).
- `[0x8e2414]` = 0x20000 (.data u32): default static VB size in bytes.

## Layout: dynamic vertex buffers
- Descriptor list `[0xc97b8c]`, entries from freelist `[0xc97b90]` (0x14 bytes): +0x0 u32 size, +0x4 u32 inUse, +0x8
  `IDirect3DVertexBuffer9*`, +0xc `IDirect3DVertexBuffer9**` owner (the caller's variable, rewritten on restore), +0x10 next
  (0x7f5a12-0x7f5a15, 0x7f5ac2-0x7f5ad0).
- Four rotating "Im" buffers: `[0xc97b94]` current index (0..3); `[0xc97b98 + 4*i]` bytes used; `[0xc97ba8 + 4*i]` capacity;
  `[0xc97bb8 + 4*i]` vertex buffer (0x7f5958-0x7f596a).
- `[0xc9bf1c]` = D3DCAPS9.DevCaps (caps base 0xc9bf00 + 0x1c); bit 0x10000 = D3DDEVCAPS_HWTRANSFORMANDLIGHT selects 4 rotating
  buffers in D3DPOOL_DEFAULT; without it a single buffer in D3DPOOL_SYSTEMMEM.

## 0x4c9970 RwD3D9IndexBufferCreate  (catalogue: specified/REA, port level 2)
- `RwBool (RwUInt32 numIndices [esp+4], void **indexBuffer [esp+8])`: `hr = D3DResourceSystem::CreateIndexBuffer(numIndices,
  0x65 (D3DFMT_INDEX16), indexBuffer)` 0x7306a0 (cdecl, 0x4c997c); returns `hr >= 0` (0x4c9988). The game routes index buffers
  through its own pool (gta-reversed D3DResourceSystem).
- librw: d3d/d3d.cpp:createIndexBuffer differs (direct `CreateIndexBuffer` with D3DUSAGE_WRITEONLY, managed pool).

## 0x4c9990 _rxD3D9DefaultDestroyInstanceCallback  (catalogue: specified/REA, port level 3)
- `void (RwResEntry *entry [esp+4])`; the `RxD3D9ResEntryHeader` (docs/re/rw/objects.md) starts at entry+0x18. Fields used
  (entry-relative): +0xc owner (RwResEntry.owner, 0x4c9998), +0x20 = header.indexBuffer (0x4c99d8), +0x28 + 0x10*k =
  header.vertexStream[k] (k = 0, 1) = `{+0x0 vertexBuffer, +0x4 offset, +0x8 stride, +0xc u16 geometryFlags, +0xe u8 managed,
  +0xf u8 dynamicLock}` (0x4c9a06-0x4c9a4c), +0x4c = header.vertexDeclaration (0x4c99eb), +0x54 = header.totalNumVertex (0x4c9a19).
- Behaviour: if `owner != NULL` and `owner->type (u8 +0) != 0xff` (0xff = world sector): if some stream k has `managed == 0`
  (scan +0x36, +0x46, 0x4c99b0): owner type 8 (geometry) -> `_rpD3D9RemoveDynamicGeometry(owner)` 0x74e9e0; type 1 (atomic) ->
  `_rpD3D9RemoveDynamicGeometry(atomic->geometry (+0x18))` (0x4c99cc); other types nothing.
  Then if index buffer: `D3DResourceSystem::DestroyIndexBuffer(ib)` 0x730d30, field = 0. If vertex declaration:
  `RwD3D9DeleteVertexDeclaration(decl)` 0x7fac10, field = 0. For both streams: if `vb`: if `managed`:
  `RwD3D9DestroyVertexBuffer(stride, stride * totalNumVertex, vb, offset)` 0x7f56a0; else if `offset == 0 && dynamicLock == 0`:
  `RwD3D9DynamicVertexBufferDestroy(vb)` 0x7f5ae0 (a non-managed buffer with offset != 0 or dynamicLock set is dropped without
  release). The stream's vb, offset, stride, flags, managed, dynamicLock are then zeroed (also when vb was NULL; 0x4c9a3d-0x4c9a4c).
- librw: d3d/d3d9.cpp:destroyNativeData differs (no dynamic-geometry list, no GTA resource pool, releases buffers directly).

## 0x7f5500 RwD3D9CreateVertexBuffer  (catalogue: specified/REA, port level 0)
- `RwBool (RwUInt32 stride [esp+4], RwUInt32 size [esp+8], void **vertexBuffer [esp+0xc], RwUInt32 *offset [esp+0x10])`.
- 1. Find the stride node (`node->stride == stride`, 0x7f5514); if absent allocate one from `[0xc97b80]` (hint 0x30411, no NULL
  check) = `{stride, 0, 0, next = [0xc97b7c]}` and push it front (0x7f551f-0x7f554c).
- 2. First free block with `block->size >= size` (unsigned `jae`, 0x7f5560) -> step 4.
- 3. None: allocate a block from `[0xc97b84]`; `offset = 0`; `bytes = ((0x20000 [0x8e2414] + stride - 1) / stride) * stride`
  (unsigned div, 0x7f5595-0x7f559d); `block->size = size >= bytes ? size : bytes`; `hr = device->CreateVertexBuffer(block->size,
  D3DUSAGE_WRITEONLY 8, FVF 0, D3DPOOL_MANAGED 1, &block->vb, NULL)` (0x7f55c2); `hr != 0` -> free the block, return 0. Link the
  block at the front of the node's free list (prev = 0) and push a VB record `{vb, node->vbList}` from `[0xc97b88]` (no NULL check).
- 4. `*vertexBuffer = block->vb; *offset = block->offset; block->offset += size; block->size -= size` (0x7f5627-0x7f5647). If the
  block is now empty (`size == 0`): unlink it (prev/next, list head) and free it (0x7f564e-0x7f5681). Return 1.
- librw: d3d/d3d.cpp:createVertexBuffer differs (one D3D buffer per call, no sub-allocation).

## 0x7f56a0 RwD3D9DestroyVertexBuffer  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 stride, RwUInt32 size, void *vertexBuffer, RwUInt32 offset)` (args [esp+4..0x10]); EAX on return is not a
  result (callers ignore it).
- Find the stride node; none -> return. Search the free list for a block with `vb == vertexBuffer` that is adjacent:
  `block->offset + block->size == offset` or `offset + size == block->offset` (0x7f56e4-0x7f5702). None: allocate a block
  `{offset, size, vb, next = head, prev = 0}` and push it front (0x7f570a-0x7f5747); return.
- Found: merge (0x7f5750): if `block->offset + block->size == offset` -> `block->size += size`; else if `offset + size ==
  block->offset` -> `block->offset = offset; block->size += size`. Then rescan the whole free list from the head for another block
  of the same vb adjacent to the merged range (0x7f5771-0x7f579c); if one is found, unlink and free the PREVIOUS block (0x7f57a4-
  0x7f57db) and merge the range into the found one (loop to 0x7f5750). Stops when no adjacent block remains. D3D buffers are only
  released in `_rwD3D9VertexBufferManagerClose`.
- librw: d3d/d3d.cpp:destroyVertexBuffer differs (releases the buffer).

## 0x7f5800 _rwD3D9DynamicVertexBufferManagerFrame  (catalogue: unknown/REA, port level 0; name from rw-names)
- `void (void)`, called once per present from `_rwD3D9RasterShowRaster` 0x7f99b0. Zeroes `[0xc97b98..0xc97ba4]` (bytes used of
  all four buffers); `[0xc97b94] = [0xc97b94] + 1`, reset to 0 when `>= 4` (unsigned, 0x7f5824).
- librw: absent.

## 0x7f5840 _rwD3D9DynamicVertexBufferRelease  (catalogue: specified/REA, port level 0)
- `void (void)`; from `D3D9DeviceReleaseVideoMemory` 0x7f7f70. `[0xc97b94] = 0`; for i = 0..3: used[i] = 0, capacity[i] = 0;
  if `vb[i]`: find the descriptor with `desc->vb == vb[i]` -> `inUse = 0`, `owner = NULL`; `vb[i] = NULL` (the D3D object is not
  released here). Then, only if DevCaps & 0x10000: for every descriptor with a vb: `vb->Release()`, `desc->vb = NULL`, and
  `*desc->owner = NULL` if owner != NULL (owner field itself kept) (0x7f58a7-0x7f58c1). (Without HW T&L the system-memory buffers
  survive a reset.)
- librw: absent (librw keeps one dynamic buffer list without owner pointers).

## 0x7f58d0 _rwD3D9DynamicVertexBufferRestore  (catalogue: specified/REA, port level 2)
- `RwBool (void)`. Only if DevCaps & 0x10000: for each descriptor with `vb == NULL`: `hr = CreateVertexBuffer(desc->size, 0x208
  (DYNAMIC|WRITEONLY), 0, D3DPOOL_DEFAULT 0, &desc->vb, NULL)` (0x7f5911); `*desc->owner = desc->vb` if owner != NULL (written even
  when hr < 0); hr < 0 -> return 0 (0x7f5921). Then tail-jumps to `_rwD3D9DynamicVertexBufferManagerCreate` 0x7f5940 and returns
  its result.
- librw: absent.

## 0x7f5940 _rwD3D9DynamicVertexBufferManagerCreate  (catalogue: unknown/REA, port level 1; name from rw-names)
- `RwBool (void)`. `[0xc97b94] = 0`. With DevCaps & 0x10000: for i = 0..3: `used[i] = 0`, `capacity[i] = 0x40000`,
  `RwD3D9DynamicVertexBufferCreate(0x40000, &vb[i])` - 0 on any failure (0x7f597e). Without: only buffer 0 (`used = 0`, capacity
  0x40000, create; failure -> 0), and entries 1..3 get used = 0, vb = NULL, capacity = 0 (0x7f59bb-0x7f59ef). Returns 1.
- librw: absent.

## 0x7f5a00 RwD3D9DynamicVertexBufferCreate  (catalogue: specified/REA, port level 0)
- `RwBool (RwUInt32 size [esp+4], void **vertexBuffer [esp+8])`.
- Scan the descriptor list (0x7f5a12-0x7f5a30): a descriptor with `inUse == 0 && size == size` is reused at once: `inUse = 1`,
  `*vertexBuffer = desc->vb`, `desc->owner = vertexBuffer`, return 1 (0x7f5a48). Otherwise remember the free descriptor with the
  smallest size (`best == NULL || best->size >= d->size` -> best = d, so ties pick the later one). If a best exists:
  `best->vb->Release()`, `best->vb = NULL` (0x7f5a36) and reuse it; else allocate a descriptor from `[0xc97b90]` (hint 0x40411, no
  NULL check) and push it front (0x7f5a64-0x7f5a89).
- `hr = CreateVertexBuffer(size, 0x208, 0, pool, vertexBuffer, NULL)` with `pool = (~DevCaps >> 15) & 2` (DEFAULT with HW T&L,
  SYSTEMMEM 2 without; 0x7f5aa0-0x7f5aa8); `hr != 0` -> return 0 (descriptor keeps its old size, `vb == NULL`). Else `desc = {size,
  inUse 1, vb = *vertexBuffer, owner = vertexBuffer}`; return 1.
- librw: d3d/d3d.cpp:createDynamicVertexBuffer differs (always D3DPOOL_DEFAULT, no descriptor reuse).

## 0x7f5b10 RwD3D9DynamicVertexBufferLock  (catalogue: specified/REA, port level 1)
- `RwBool (RwUInt32 vertexSize [esp+4], RwUInt32 numVertex [esp+8], void **vertexBufferOut [esp+0xc], void **vertexData
  [esp+0x10], RwUInt32 *baseIndex [esp+0x14])`. `bytes = vertexSize * numVertex` (32-bit `imul`, 0x7f5b25); `cur = [0xc97b94]`.
- With DevCaps & 0x10000: `off = ceil(used[cur] / vertexSize) * vertexSize`; if `off + bytes <= capacity[cur]` use cur. Else try
  i = 0..3 skipping cur, first i with `ceil(used[i]/vertexSize)*vertexSize + bytes <= capacity[i]` -> `[0xc97b94] = i` (0x7f5b87).
  If none: `[0xc97b94] = cur + 1` (0 when it reaches 4) and `off = 0` (0x7f5b94-0x7f5ba8). Without the cap: `off = 0` on `cur`.
- If `bytes > capacity[idx]` (0x7f5bac): mark the descriptor of `vb[idx]` free (`inUse = 0, owner = NULL`), `capacity[idx] =
  bytes`, `RwD3D9DynamicVertexBufferCreate(bytes, &vb[idx])` (result ignored).
- Lock: `off == 0` -> `hr = vb->Lock(0, bytes, vertexData, D3DLOCK_DISCARD|NOSYSLOCK 0x2800)`, `used[idx] = bytes`, `*baseIndex =
  0`; else `hr = vb->Lock(off, bytes, vertexData, D3DLOCK_NOOVERWRITE|NOSYSLOCK 0x1800)`, `used[idx] = off + bytes`, `*baseIndex =
  off / vertexSize` (unsigned). `*vertexBufferOut = vb[idx]`; return `hr >= 0`.
- librw: absent (librw im2d/im3d lock their own buffers with DISCARD every time).

## 0x7f5c90 RwD3D9DynamicVertexBufferUnlock  (catalogue: specified/REA, port level 0)
- `RwBool (void *vertexBuffer)`: `return vb->Unlock() >= 0` (vtbl+0x30).

## 0x7f5cb0 _rwD3D9VertexBufferManagerOpen  (catalogue: specified/REA, port level 2)
- `RwBool (void)`: `_rwD3D9VertexBufferManagerClose()`; `[0xc97b80] = _rwFreeListCreate(0x10, 0x10, 4, 0x40411)`; `[0xc97b84] =
  (0x14, 0x64, 4, 0x40411)`; `[0xc97b88] = (8, 0x64, 4, 0x40411)`; `[0xc97b90] = (0x14, 0x2a, 4, 0x40411)`; call 0x7f5940 (result
  ignored); return 1. No NULL checks.

## 0x7f5d20 _rwD3D9VertexBufferManagerClose  (catalogue: specified/REA, port level 1)
- `void (void)`. Same four-buffer reset as 0x7f5840 (`[0xc97b94] = 0`, used/capacity 0, descriptors of vb[i] freed, vb[i] = NULL).
  Then pop every descriptor from `[0xc97b8c]`: `vb->Release()` if set, `vb = NULL`, `*owner = NULL` if owner, free the descriptor
  (`[0xc97b90]`). `RwFreeListDestroy([0xc97b90])` 0x801b80 if non-NULL, set 0. Then for every stride node: free all free blocks
  (`[0xc97b84]`), release every recorded VB (`Release`, record freed to `[0xc97b88]`), free the node (`[0xc97b80]`). Finally destroy
  the freelists `[0xc97b88]`, `[0xc97b84]`, `[0xc97b80]` (in that order) and zero them.
- librw: absent.
