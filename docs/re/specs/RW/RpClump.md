# RpClump (RenderWare 3.6 RpWorld plugin, statically linked) - full specs, writer B (RE-SPEC-RW)

All functions are `__cdecl` (caller pops). `Engine` = `[RwEngineInstance 0xc97b24]`. Memory: `Engine+0x144`
freelist alloc `(freelist, hint)`, `Engine+0x148` freelist free `(freelist, ptr)`, `Engine+0x134` malloc
`(size, hint)`, `Engine+0x138` free `(ptr)`. Error reporting: a local `RwError {i32 pluginID; i32 errorCode}`
is filled with `pluginID = 2` and `errorCode = _rwerror(code, ...)` (0x8088d0 just returns its first argument)
and passed to `RwErrorSet` 0x808820. Codes seen: 0x8000001a (chunk/read failure), 0x80000004 (bad version),
0x80000013 (allocation failure, extra arg = size), 5 (clone: atomic without a frame).

## Layouts used (verified against the bytes)

RpClump (registry 0x8d6264, base 0x2c; matches objects.md):

| off | field | proving instruction |
|---|---|---|
| +0x00 u8 | object.type = 2 | 0x74a2cf |
| +0x01/+0x02/+0x03 u8 | subType / flags / privateFlags = 0 | 0x74a2d7..0x74a2dd |
| +0x04 ptr | object.parent = root RwFrame | 0x74a2e0, 0x74b65e |
| +0x08 RwLinkList | atomicList (links are atomic+0x40) | 0x74a2bb, 0x7498e7, 0x7498f1 |
| +0x10 RwLinkList | lightList | 0x74a2c9, 0x749c07 |
| +0x18 RwLinkList | cameraList | 0x74a2d4, 0x74a3b9.. |
| +0x20 RwLLLink | inWorldLink (zeroed) | 0x74a2eb/0x74a2e8 |
| +0x28 fn | callback (default 0x74a260 `_rpClumpDefaultCallBack` = return arg) | 0x74a2ee |

RpAtomic fields used: +0x3c clump (0x74a4af), +0x40/+0x44 inClumpLink next/prev (0x74a4a3), +0x02 flags bit 4 =
rpATOMICRENDER (0x749b34 reads link-0x3e), +0x48 renderCallBack.

CLUMP plugin extension on RwCamera (offset global 0xc924d8) and RpLight (0xc924dc), 0xc bytes each
(registered by 0x749830): `+0x0 RpClump* clump; +0x4 RwLLLink inClumpLink (next +0x4, prev +0x8)`. Proof: list
links are converted with `link - offset - 4` (0x749c25/0x749c27) and the unlink code writes ext+8/+4/+0
(0x74a1cd..0x74a1e6); stream read sets `ext+0 = clump` (0x74ba05).

CLUMPMODULE engine data at `Engine + [0xc924ac]` (size 8): +0 atomic freelist, +4 clump freelist (0x749c5d,
0x74a2a1). Open-instance counter 0xc924b0. Freelist parameter globals: 0x8d627c = 0x80 atomics per block,
0x8d6280 = 1 block preallocated; 0x8d6284 = 0x80 clumps per block, 0x8d6288 = 1 (values from .data).
Static freelist structs: 0xc92484 (atomics), 0xc924b4 (clumps).

## 0x749760 cb_CLUMPMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- Convention: engine-module open callback `void* (void* instance, i32 offset, i32 size)`; returns instance or NULL.
- Callees: RwFreeListCreateAndPreallocateSpace 0x801b70 `(entrySize, entriesPerBlock, alignment, numBlocks, RwFreeList* inFreeList, hint)`, RwFreeListDestroy 0x801b80.
- Behaviour: `[0xc924ac] = offset` (written before anything else, 0x74977a). Atomic freelist =
  `Create(entrySize = [0x8d624c] (atomic registry sizeOfStruct), [0x8d627c], 4, [0x8d6280], 0xc92484, 0x40010)` stored at
  `Engine+offset+0`; NULL -> return NULL. Clump freelist = `Create([0x8d6264], [0x8d6284], 4, [0x8d6288], 0xc924b4, 0x40014)` stored at
  `+4`; if NULL: `RwFreeListDestroy(atomic freelist)`, `+0 = 0`, return NULL. Success: `[0xc924b0]++`, return instance.
- librw: absent (plain malloc).

## 0x749830 _rpClumpRegisterExtensions  (catalogue: specified/REA, port level 3)
- Convention: cdecl `RwBool (void)`.
- Behaviour: `[0xc924d8] = RwCameraRegisterPlugin(0xc, 0x10 /*rwID_CLUMP*/, 0x749880, 0x7498a0, NULL)` (0x7ee450);
  if `< 0` return 0. `[0xc924dc] = RpLightRegisterPlugin(0xc, 0x10, 0x7498b0, 0x7498d0, NULL)` (0x751d60) - stored
  unconditionally - return `offset >= 0`.

## 0x749880 cb_CLUMP_RwCamera_Ctor  (catalogue: specified/REA, port level 0)
- `void* (void* camera, i32 offset, i32 size)`: zero the 3 dwords at `camera + [0xc924d8]` (writes +8, +4, +0); return camera.
  (Uses the global, not the `offset` argument.)

## 0x7498b0 cb_CLUMP_RpLight_Ctor  (catalogue: specified/REA, port level 0)
- Same as 0x749880 with `light + [0xc924dc]`; returns light.

## 0x7498d0 cb_CLUMP_RpLight_Dtor  (catalogue: specified/REA, port level 0)
- `void* (void* light, i32, i32)`: returns light; nothing else.

## 0x7498e0 RpClumpGetNumAtomics  (catalogue: specified/REA, port level 0)
- cdecl `i32 (RpClump* clump)`. Walk `atomicList` from `head.next` until the link equals `&clump->atomicList`; per
  link: `count++`, then stop early if `link - 0x40 == NULL` (only possible for link 0x40; count already incremented).
  Return count.

## 0x749b20 RpClumpRender  (catalogue: specified/REA, port level 4)
- cdecl `RpClump* (RpClump* clump)`; returns clump, or NULL if any atomic render callback returned NULL.
- Behaviour: for each link of atomicList (head to tail): `a = link - 0x40`; if `a->object.flags & 4` (byte +2):
  `RwFrameGetLTM(a->parent)` (0x7f0990; result discarded, forces the LTM sync), then `r = a->renderCallBack(a)` (the
  frame pointer is still on the stack as a 2nd argument; both popped together, 0x749b4b); `if (!r) result = NULL`.
  The next link is read AFTER the callback (0x749b54): a callback must not unlink its atomic. All atomics are rendered
  even after a failure.
- librw: clump.cpp:Clump::render differs (no NULL propagation).

## 0x749b70 RpClumpForAllAtomics  (catalogue: specified/REA, port level 0)
- cdecl `RpClump* (RpClump* clump, RpAtomicCallBack cb, void* data)`. For each link of atomicList: next is read
  BEFORE the call (0x749b8a, deletion-safe); `if (!cb(link - 0x40, data)) break`. Returns clump always.

## 0x749f70 RpClumpClone  (catalogue: specified/REA, port level 7)
- Convention: cdecl `RpClump* (RpClump* src)`; returns new clump or NULL. Only caller: CClumpModelInfo::CreateInstance 0x4c5140.
- Callees: RpClumpCreate, _rwFrameCloneAndLinkClones 0x7effb0 (clones the hierarchy and leaves each source frame's
  `root` (+0xa0) pointing at its clone), _rwFramePurgeClone 0x7f01a0 (restores the source roots), RpAtomicClone,
  _rwObjectHasFrameSetFrame, RpClumpAddAtomic, RpAtomicDestroy, RpClumpRemoveLight 0x74a520, RpLightDestroy 0x7520d0,
  _rpClumpRemoveAndDestroyLight 0x74a1c0, RpClumpForAllCameras 0x749bb0 with _rpClumpRemoveAndDestroyCamera 0x74a200,
  RwFrameDestroyHierarchy 0x7f08a0, plugin DeInit/Copy, RwErrorSet.
- Behaviour:
  1. `c = RpClumpCreate()`; NULL -> return NULL.
  2. `c->parent = _rwFrameCloneAndLinkClones(src->parent)` (direct store, no SetFrame). If NULL -> failure A.
  3. `ok = 1`. For each src atomic link (next read before the work): `n = RpAtomicClone(a)`; if NULL: `ok = 0`, stop.
     If `a->parent == NULL` (0x74a044): destroy n inline (DeInitObject(atomic registry), free repEntry,
     `RpAtomicSetGeometry(n, NULL, 0)`, ReleaseFrame, free to atomic freelist), set error `{2, 5}` and STOP the loop
     with `ok` still 1 (original quirk: the clone then succeeds with the remaining atomics missing). Otherwise
     `_rwObjectHasFrameSetFrame(n, a->parent->root /*= the cloned frame*/)`, `n->privateFlags |= 1`,
     `RpClumpAddAtomic(c, n)` (head insertion, so the clone's atomic order is reversed).
  4. `_rwFramePurgeClone(src->parent)`.
  5. If ok: `c->callback (+0x28) = src->callback`; `_rwPluginRegistryCopyObject(0x8d6264, c, src)`; return c.
     Lights and cameras of src are NOT cloned.
  6. If !ok (failure B): DeInitObject(clump registry, c); for each atomic of c: RpAtomicDestroy (stops if the computed
     atomic pointer is NULL); for each light: `_rpClumpRemoveAndDestroyLight(light, NULL)` until it returns NULL; then
     the common tail.
  7. Failure A: DeInitObject(c); RpAtomicDestroy on each atomic; for each light: `RpClumpRemoveLight(ext->clump, light)`
     + `RpLightDestroy(light)`; common tail.
  8. Common tail (0x74a15d): `RpClumpForAllCameras(c, 0x74a200, NULL)`; if `c->parent` RwFrameDestroyHierarchy;
     free c to the clump freelist (`Engine+[0xc924ac]+4`); return NULL.
- librw: clump.cpp:Clump::clone differs: no failure handling, adds the clone to the source's world, no callback copy.

## 0x74a290 RpClumpCreate  (catalogue: specified/REA, port level 1)
- cdecl `RpClump* (void)`. `c = Engine->memoryAlloc(*(Engine+[0xc924ac]+4), 0x30010)`; NULL -> return NULL (no error).
  Init: the three lists empty (next = prev = &list), `type = 2`, subType/flags/privateFlags = 0, `parent = NULL`,
  `inWorldLink = {0,0}`, `callback = 0x74a260`. `_rwPluginRegistryInitObject(0x8d6264, c)`; return c.
- librw: clump.cpp:Clump::create matches apart from no callback field / world ext done inline.

## 0x74a310 RpClumpDestroy  (catalogue: specified/REA, port level 6)
- cdecl `RwBool (RpClump* clump)`; returns 1.
- Behaviour (order):
  1. `_rwPluginRegistryDeInitObject(0x8d6264, clump)` (WORLDOBJ clump dtor runs here).
  2. Atomics: for each link (next read first): inline RpAtomicDestroy (DeInitObject(0x8d624c), free repEntry,
     RpGeometryDestroy + `geometry = 0` + RwFrameUpdateObjects(frame) if frame and RpAtomicGetWorld, ReleaseFrame,
     free to atomic freelist). The atomics are NOT unlinked from the list (it is being discarded).
  3. Lights: for each link: `light = link - [0xc924dc] - 4`; unlink ext link from the list; zero ext +8/+4/+0;
     RpLightDestroy(light); stop if light pointer == NULL.
  4. Cameras: same with `[0xc924d8]` and RwCameraDestroy 0x7ee4b0.
  5. If `clump->parent`: RwFrameDestroyHierarchy(parent).
  6. Free clump to `Engine+[0xc924ac]+4`; return 1.
- librw: clump.cpp:Clump::destroy differs (removes atomics from list first, asserts world == nil).

## 0x74a490 RpClumpAddAtomic  (catalogue: specified/REA, port level 0)
- cdecl `RpClump* (RpClump* clump, RpAtomic* atomic)`: insert `atomic+0x40` at the head of `clump->atomicList`
  (`link.prev = &list; link.next = old head; old head->prev = link; list.next = link`), `atomic->clump = clump`; return clump.

## 0x74a4c0 RpClumpRemoveAtomic  (catalogue: specified/REA, port level 0)
- cdecl `RpClump* (RpClump* clump, RpAtomic* atomic)`: `prev->next = next; next->prev = prev` for atomic+0x40 (link
  fields not cleared); `atomic->clump = NULL`; return clump.

## 0x74b060 _rpClumpAtomicStreamRead  (catalogue: specified/REA, port level 9)
- Convention: cdecl `RpAtomic* (RwStream* stream, rwFrameList* frameList, rpGeometryList* geomList)`. frameList =
  `{RwFrame** frames +0; i32 numFrames +4}`, geomList = `{RpGeometry** geometries +0; i32 numGeoms +4}`. Called after the
  caller found the rwID_ATOMIC chunk. Callers: RpAtomicStreamRead 0x74b030, RpClumpStreamRead.
- Behaviour:
  1. `RwStreamFindChunk(stream, 1 /*STRUCT*/, &len, &ver)`; fail -> error 0x8000001a, return NULL.
  2. If `ver < 0x34000 || ver > 0x36003` (unsigned) -> error 0x80000004, return NULL (0x74b08c..0x74b09c).
  3. `buf[4] = {0}`; `RwStreamRead(stream, buf, len)` - reads `len` bytes into the 16-byte local (no clamp; len > 16
     overflows the stack in the original); result != len -> error 0x8000001a, return NULL.
     buf[0] = frame index, buf[1] = geometry index, buf[2] = flags, buf[3] unused.
  4. `a = RpAtomicCreate()`; NULL -> return NULL (no extra error). `a->object.flags = (u8)buf[2]`.
  5. If `frameList->numFrames != 0`: `_rwObjectHasFrameSetFrame(a, frameList->frames[buf[0]])`, `privateFlags |= 1`.
     (No bounds check on any index.)
  6. If `geomList->numGeoms != 0`: inline `RpAtomicSetGeometry(a, geomList->geometries[buf[1]], 0)` (see RpAtomic.md;
     includes the frame update only when the atomic is in a world).
     Else: `RwStreamFindChunk(stream, 0xf /*GEOMETRY*/, NULL, &ver)`; fail -> destroy a (inline RpAtomicDestroy),
     error 0x8000001a, NULL. Version out of [0x34000, 0x36003] -> destroy a, error 0x80000004, NULL.
     `g = RpGeometryStreamRead(stream)`; NULL -> destroy a, error 0x8000001a, NULL. Else inline SetGeometry(a, g, 0)
     then `RpGeometryDestroy(g)` (drops the creation reference).
  7. `[0xc924a8] = 0; [0xc92480] = 0` (rights globals); `_rwPluginRegistryReadDataChunks(0x8d624c, stream, a)`; 0 ->
     error 0x8000001a, return NULL. The atomic is LEAKED on this path (original bug).
  8. If `[0xc924a8] != 0`: `_rwPluginRegistryInvokeRights(0x8d624c, [0xc924a8], a, [0xc92480])` (0x808ab0).
  9. Return a.
- librw: clump.cpp:Atomic::streamReadClump differs: reads 12 or 16 bytes by version, no version check, setGeometry
  always, flags set after geometry, destroys the atomic on plugin failure.

## 0x74b420 RpClumpStreamRead  (catalogue: specified/REA, port level 10)
- Convention: cdecl `RpClump* (RwStream* stream)`; called after the caller found rwID_CLUMP (0x10). Callers:
  CFileLoader::LoadClumpFile / LoadAtomicFile, CClothesBuilder. The argument slot [esp+0x58] is reused as a local i32
  (count / frame index) once the stream is in EDI.
- Locals: `clumpInfo {numAtomics, numLights, numCameras}` (12 bytes), `frameList` (8 bytes), `geomList` (8 bytes).
- Behaviour:
  1. `RwStreamFindChunk(stream, 1, &len, &ver)`; fail -> error 0x8000001a, NULL. `ver` outside [0x34000, 0x36003] ->
     error 0x80000004, NULL. `RwStreamRead(stream, &clumpInfo, 12) != 12` -> error 0x8000001a, NULL. (Always 12 bytes;
     `len` is ignored.)
  2. `c = RpClumpCreate()`; NULL -> return NULL.
  3. `RwStreamFindChunk(stream, 0xe /*FRAMELIST*/, NULL, &ver2)`; fail -> cleanup C1 (see below). `_rwFrameListStreamRead(stream, &frameList)`
     (0x807970) NULL -> C1. `c->parent = frameList.frames[0]` (direct store, 0x74b65e).
  4. `RwStreamFindChunk(stream, 0x1a /*GEOMETRYLIST*/, NULL, &ver2)`; fail -> `_rwFrameListDeinitialize(&frameList)`, C1.
  5. `RwStreamFindChunk(stream, 1, &len2, &ver3)`; fail -> C2. ver3 outside [0x34000, 0x36003] -> error 0x80000004, C2.
     `RwStreamRead(stream, &n, 4) != 4` -> C2. `geomList.numGeoms = 0`. If `n > 0`: `geomList.geometries = Engine->malloc(n*4, 0x3000f)`;
     NULL -> error `_rwerror(0x80000013, n*4)`, C2. If `n <= 0`: `geometries = NULL`.
     For i in 0..n: `FindChunk(stream, 0xf, NULL, &ver3)`; fail -> destroy the `numGeoms` geometries read so far,
     `Engine->free(geometries)`, C2. ver3 out of range -> `_rpClumpGeometryListDestroy(&geomList)` (0x74ae50), C2 (no
     version error is set on this path). `geometries[i] = RpGeometryStreamRead(stream)`; NULL -> `_rpClumpGeometryListDestroy`, C2;
     else `numGeoms++`.
  6. Atomics: for i in 0..numAtomics (signed, skipped if <= 0): `FindChunk(stream, 0x14 /*ATOMIC*/, NULL, &ver)`; fail ->
     C3. `a = _rpClumpAtomicStreamRead(stream, &frameList, &geomList)`; NULL -> C3. Inline RpClumpAddAtomic(c, a).
  7. Lights: for i in 0..numLights: `FindChunk(stream, 1, NULL, NULL)`, `RwStreamReadInt32(stream, &idx, 4)`,
     `FindChunk(stream, 0x12 /*LIGHT*/, NULL, NULL)`, `l = RpLightStreamRead(stream)` (0x751f00); any failure -> C3.
     `_rwObjectHasFrameSetFrame(l, frameList.frames[idx])`; insert `l + [0xc924dc] + 4` at the head of `c->lightList`;
     `ext->clump = c`.
  8. Cameras: same with chunk 5 (rwID_CAMERA), `RwCameraStreamRead` 0x808de0, ext `[0xc924d8]`, list +0x18.
  9. Success tail: RpGeometryDestroy on each of the `numGeoms` list entries (drop list references); if geometries:
     `Engine->free(geometries)`; `_rwFrameListDeinitialize(&frameList)`; `_rwPluginRegistryReadDataChunks(0x8d6264, stream, c)`;
     0 -> `RpClumpDestroy(c)`, error 0x8000001a, NULL. Return c.
- Cleanups (all end with error 0x8000001a and return NULL):
  - C1 (0x74b4f7/0x74b527): DeInitObject(c); RpAtomicDestroy each atomic; `RpClumpForAllLights(c, 0x74a1c0, NULL)`;
    `RpClumpForAllCameras(c, 0x74a200, NULL)`; RwFrameDestroyHierarchy(c->parent) if any; free c.
  - C2 (0x74b7de): `_rwFrameListDeinitialize(&frameList)`; DeInitObject(c); `RpClumpForAllAtomics(c, 0x74bcc0, NULL)`
    (0x74bcc0 = destroy atomic without unlinking, returns the atomic); lights; cameras; frame hierarchy; free c.
  - C3 (0x74ba15 / 0x74ba41 / 0x74bbb2): destroy the `numGeoms` list geometries, free the array, then C2.
- Edge: no bounds check on light/camera frame indices; chunk versions of FRAMELIST/ATOMIC/LIGHT/CAMERA are not checked.
- librw: clump.cpp:Clump::streamRead differs: reads `len` bytes and version-gated light/camera counts, supports
  versions < 0x30400 without a geometry list, no version-range rejection.

## 0x74bdd0 RpClumpRegisterPlugin  (catalogue: specified/REA, port level 2)
- cdecl `i32 (i32 size, u32 pluginID, ctor, dtor, copy)` -> `_rwPluginRegistryAddPlugin(0x8d6264, ...)` 0x8084a0.

## 0x74be70 RpClumpRegisterPluginStream  (catalogue: specified/REA, port level 1)
- cdecl `i32 (u32 pluginID, readCB, writeCB, getSizeCB)` -> `_rwPluginRegistryAddPluginStream(0x8d6264, ...)` 0x8088e0.
