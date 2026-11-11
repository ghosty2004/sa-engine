# RpMesh / build meshes / bin-mesh plugin (RenderWare 3.6 RpWorld plugin, statically linked) - full specs, writer B (RE-SPEC-RW)

All functions are `__cdecl` (caller pops). `Engine` = `[RwEngineInstance 0xc97b24]`; `+0x134` malloc `(size, hint)`,
`+0x138` free, `+0x13c` realloc `(ptr, size, hint)`, `+0x144`/`+0x148` freelist alloc/free. Errors: `RwErrorSet({2, code})`.

## Layouts used (verified against the bytes)

RpMeshHeader (0x10, followed by `numMeshes` RpMesh and then the u16 index data; one malloc, hint 0x30502):
+0x0 u32 flags (0x758f53), +0x4 u16 numMeshes (0x758f5d), +0x6 u16 serialNum (0x758f79), +0x8 u32 totalIndicesInMesh
(0x758f81), +0xc u32 firstMeshOffset (meshes start at `header + 0x10 + firstMeshOffset`, 0x758d43). Matches objects.md.

RpMesh (0xc): +0x0 indices (u16*) (0x758fed), +0x4 numIndices (0x758fdb), +0x8 material (0x758fea). Matches objects.md.

RpBuildMesh (0xc, freelist at global 0xc926f0): +0x0 u32 triangleBufferSize (0x758b2b), +0x4 u32 numTriangles (0x758abe),
+0x8 RpBuildMeshTriangle* meshTriangles (0x758ae4).
RpBuildMeshTriangle (0x14): +0x0/+0x2/+0x4 u16 vertIndex[3], +0x6 (unwritten pad), +0x8 RpMaterial* material, +0xc u16 matIndex,
+0xe u16 textureIndex, +0x10 u16 rasterIndex, +0x12 u16 pipelineIndex (0x758c8d..0x758d10).

MESHMODULE engine data (size 0x30, offset stored in 0xc9b8c0 by `_rpMeshOpen`; plugins.md lists no global for it - this is
it): +0x0 u16 nextSerialNum (0x7589b0, 0x758f75/0x758f96); +0x8 u8 meshFlagsToPrimType[0x20] (indexed by header flags:
[0]=3 TRILIST, [1]=4 TRISTRIP, [2]=5 TRIFAN, [4]=1 LINELIST, [8]=2 POLYLINE, [0x10]=6 POINTLIST); +0x28 u8
primTypeToMeshFlags[] indexed by `primType - 1` ({4, 8, 0, 1, 2, 0x10}). Open counter 0xc9b8c4.

## 0x750db0 cb_BINMESHPLUGIN_RpGeometry_StreamRead  (catalogue: specified/REA, port level 4)
- Plugin stream-read callback `RwStream* (RwStream* stream, i32 len, void* geometry, i32 offset, i32 size)`.
- `geometry->mesh (+0x54) = _rpMeshRead(stream, geometry, &geometry->matList (+0x20))` (any previous mesh pointer is overwritten
  without being freed); return `mesh ? stream : NULL` (neg/sbb/and).

## 0x758920 _rpMeshHeaderCreate  (catalogue: specified/REA, port level 0)
- cdecl `RpMeshHeader* (u32 size)`: return `Engine->malloc(size, 0x30502)` (no initialisation).

## 0x758970 _rpMeshOpen  (catalogue: specified/REA, port level 2)
- Engine-module open callback `void* (void* instance, i32 offset, i32 size)`.
- `[0xc9b8c0] = offset`. If `[0xc9b8c4] == 0` (first open): `[0xc926f0] = _rwFreeListCreate(0xc, 0x32, 4, 0x40502)` (0x801980:
  entrySize, entriesPerBlock, alignment, hint); NULL -> return NULL (counter not incremented).
- Then: `*(u16*)(Engine+offset+0) = 1`; `[0xc9b8c4]++`; bytes at `Engine+offset`: +0x8 = 3, +0x9 = 4, +0xa = 5, +0xc = 1,
  +0x10 = 2, +0x18 = 6, +0x28 = 4, +0x29 = 8, +0x2a = 0, +0x2b = 1, +0x2c = 2, +0x2d = 0x10 (other bytes not written). Return instance.
- Edge: the freelist is shared across engine instances (created only when the counter is 0).

## 0x758a90 _rpBuildMeshCreate  (catalogue: specified/REA, port level 1)
- cdecl `RpBuildMesh* (u32 bufferSize)`.
- `bm = Engine->memoryAlloc([0xc926f0], 0x30502)`; NULL -> error `{2, _rwerror(0x80000013, 0xc)}`, NULL. `numTriangles = 0`.
  If `bufferSize != 0`: `meshTriangles = Engine->malloc(bufferSize*20, 0x1030502)`; NULL -> free bm to the freelist, error
  `{2, _rwerror(0x80000013, bufferSize*20)}`, NULL; else `triangleBufferSize = bufferSize`. If 0: `meshTriangles = NULL;
  triangleBufferSize = 0`. Return bm.

## 0x758b80 _rpBuildMeshDestroy  (catalogue: specified/REA, port level 0)
- cdecl `RwBool (RpBuildMesh* bm)`: if `meshTriangles`: `Engine->free`, `= NULL`; free bm to `[0xc926f0]`; return 1.

## 0x758bc0 _rpMeshDestroy  (catalogue: specified/REA, port level 0)
- cdecl `RwBool (RpMeshHeader* h)`: `Engine->free(h)` unless the 16-byte header is entirely zero (flags, numMeshes, serialNum,
  totalIndices, firstMeshOffset all 0 -> nothing freed). Returns 1.
- Open question: which caller passes an all-zero header (static empty mesh?) - not seen in this work list (0x758bc4).

## 0x758c00 _rpBuildMeshAddTriangle  (catalogue: specified/REA, port level 1)
- cdecl `RpBuildMesh* (RpBuildMesh* bm, RpMaterial* mat, i32 v0, i32 v1, i32 v2, u16 matIndex, u16 texIdx, u16 rasIdx, u16 pipeIdx)`
  (9 dword args; only the low 16 bits of the index args are stored).
- If `numTriangles >= triangleBufferSize` (unsigned): `size = (numTriangles + 1) * 20`; if `numTriangles != 0` (tests the
  count, not the pointer): `p = Engine->realloc(meshTriangles, size, 0x1030502)` else `p = Engine->malloc(size, 0x1030502)`;
  NULL -> error `{2, _rwerror(0x80000013, size)}`, return NULL. `meshTriangles = p; triangleBufferSize = numTriangles + 1`
  (grows one entry at a time).
- `t = &meshTriangles[numTriangles]`: `t->material = mat; vertIndex = {v0, v1, v2}; matIndex, textureIndex, rasterIndex,
  pipelineIndex` as given; `numTriangles++`; return bm.

## 0x758d30 _rpMeshHeaderForAllMeshes  (catalogue: specified/REA, port level 0)
- cdecl `RpMeshHeader* (RpMeshHeader* h, RpMeshCallBack cb, void* data)`: `n = h->numMeshes` (u16); `m = h + 0x10 + firstMeshOffset`;
  while `n != 0`: `n--; if (!cb(m, h, data)) break; m += 0xc`. Return h.

## 0x758ec0 _rpMeshRead  (catalogue: specified/REA, port level 3)
- Convention: cdecl `RpMeshHeader* (RwStream* stream, const void* object, const RpMaterialList* matList)`; `object` is an
  RpGeometry (type byte 8) or an RpWorld (type byte 7, for sectors); `object+8` = its flags. 0x420-byte stack frame
  (0x400-byte index buffer).
- Behaviour:
  1. `RwStreamReadInt32(stream, &hdr, 12)` -> `{u32 flags; u32 numMeshes; u32 totalIndices}`; 0 -> NULL.
  2. `size = (numMeshes + 1) * 16`; `nonNative = (type == 8 || type == 7) && !(object->flags & 0x1000000)`; if nonNative
     `size += totalIndices * 2`. `h = Engine->malloc(size, 0x30502)`; NULL -> NULL (no error).
  3. `h->flags = flags; h->numMeshes = (u16)numMeshes; h->serialNum = nextSerialNum (MESHMODULE +0); nextSerialNum++;
     h->totalIndicesInMesh = totalIndices; h->firstMeshOffset = 0`. `idx = (u16*)(h + 0x10 + numMeshes*12)`.
  4. For each mesh (counter = `h->numMeshes` (u16), loop while non-zero): `RwStreamReadInt32(stream, &mh, 8)` -> `{u32 numIndices;
     i32 matIndex}`; 0 -> return NULL (header leaked, serial number consumed). `mesh->numIndices = numIndices;
     mesh->material = _rpMaterialListGetMaterial(matList, matIndex)` (no reference added, no bounds check); `mesh->indices = idx`.
     If nonNative: read the indices in chunks of `min(remaining, 0x100)` with `RwStreamReadInt32(stream, buf, chunk*4)` (0 ->
     return NULL, leak) and store each as `(u16)value` at `idx++`. Native objects store no index data (all meshes point at the
     same address after the mesh array).
  5. Return h.
- librw: geometry.cpp / mesh read (readMesh) differs: reads with explicit 16/32-bit index handling, no serial numbers,
  allocates separately.
