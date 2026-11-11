# RpWorld, world sectors, world-object sync, world pipelines (RenderWare 3.6 RpWorld plugin, statically linked) - full specs, writer B (RE-SPEC-RW)

All functions are `__cdecl` (caller pops). `Engine` = `[RwEngineInstance 0xc97b24]`; `+0x134` malloc `(size, hint)`,
`+0x138` free, `+0x13c` realloc `(ptr, size, hint)`, `+0x144`/`+0x148` freelist alloc/free. Errors: `RwErrorSet({2, code})`
(see RpClump.md). Engine-module open callbacks have the signature `void* (void* instance, i32 offset, i32 size)` and return
instance or NULL.

## Layouts used (verified against the bytes)

RpWorld (registry 0x8d62d0, base 0x70; malloc'd, hint 0x3000b; matches objects.md):
+0x00 u8 type = 7 (0x74f7c6); +0x01..+0x03 = 0; +0x04 parent = 0 (0x74f7d3); +0x08 u32 flags (0x1 = TRISTRIP for
meshes 0x74f474, 0x1000000 native) (0x74f7e0); +0x0c renderOrder (2 at create = BACK2FRONT, 0x74f7e3; 1 = FRONT2BACK
0x7506ab); +0x10 RpMaterialList matList (materials +0x10, numMaterials +0x14; 0x74f248, 0x74f288); +0x1c rootSector
(0x74f8e6); +0x20 numTexCoordSets (0x74f8e9); +0x24 numClumpsInWorld (0x74f91b); +0x28 currentClumpLink (0x74f927);
+0x2c clumpList (0x74f921); +0x34 lightList; +0x3c directionalLightList (0x74f92c / 0x74f934); +0x44 worldOrigin
(0x74f8f2..0x74f8ec); +0x50 boundingBox {sup +0x50, inf +0x5c} (0x74f8f8..0x74f91e); +0x68 renderCallBack (default
0x74eec0 `_rpSectorDefaultRenderCallBack`, 0x74f93c); +0x6c pipeline (0x74f943).

RpWorldSector (registry 0x8d736c, base 0x88): +0x00 i32 type (< 0 = atomic sector, -1 at create, 0x74f858); +0x04 triangles
(0x74f87a, 0x74f2ff); +0x08 vertices; +0x0c normals; +0x10 texCoords[8]; +0x30 preLitLum; +0x34 repEntry; +0x38
collAtomicsInWorldSector (RpTie list, 0x750910); +0x40 lightsInWorldSector (RpLightTie list, 0x750c0e); +0x48 boundingBox
{sup +0x48, inf +0x54} (0x74f8a5 / 0x74f880, 0x75055b); +0x60 tightBoundingBox {sup +0x60, inf +0x6c}; +0x78 mesh (0x74f257);
+0x7c pipeline; +0x80 u16 matListWindowBase (0x74f24d; NOT set by RpWorldCreate); +0x82 u16 numVertices; +0x84 u16
numTriangles (0x74f268). Matches objects.md.

RpPlaneSector (BSP node, type >= 0): +0x00 i32 type = byte offset of the axis inside an RwV3d (0, 4, 8; used as
`[bbox + type]`, 0x750951); +0x04 f32 value (0x75064a); +0x08 leftSubTree (0x750963); +0x0c rightSubTree (0x750976);
+0x10 f32 leftValue (0x750955); +0x14 f32 rightValue (0x75096a).

RpTie (0x18): +0x0 RwLLLink lAtomicInWorldSector, +0x8 apAtom, +0xc RwLLLink lWorldSectorInAtomic, +0x14 worldSector
(0x750910..0x750939). RpLightTie (0x18): +0x0 link in sector, +0x8 light, +0xc link in light, +0x14 sector (0x750c0e..0x750c37).

RwCamera fields: +0x04 parent, +0x10 sync, +0x18 beginUpdate, +0x1c endUpdate (0x7503f7..0x750417); +0x94 frustumPlanes[6]
(stride 0x14: RwPlane {normal +0, distance +0xc}, u8 closestX +0x10, closestY +0x11, closestZ +0x12; 0x75054d..0x75058b);
+0x10c frustumBoundBox {sup +0x10c, inf +0x118} (0x750504, 0x7504f4).

Plugin extensions (offset globals written by `_rpWorldObjRegisterPlugins`):
- RwCamera WORLDOBJ ext (0x1c, `[0xc925a4]`): +0x0 RpWorldSector** visible-sector array, +0x4 i32 capacity, +0x8 i32 count,
  +0xc RpWorld* world, +0x10 saved beginUpdate, +0x14 saved endUpdate, +0x18 saved sync (0x7503ef..0x75041e, 0x7505f5..0x7506a2).
- RpAtomic ext (8, `[0xc925a8]`): +0x0 RpWorld*, +0x4 saved sync (0x7507bc, 0x7507d5).
- RpClump ext (8, `[0xc925ac]`): +0x0 RpWorld*, +0x4 copy of WORLDOBJ engine data +8 (0x750a3d, 0x750a54).
- RpLight ext (8, `[0xc925b0]`): +0x0 RpWorld*, +0x4 saved sync (0x750aac, 0x750ab5).
Module engine data: WORLDOBJMODULE `[0xc9259c]` (0xc): +0 RpTie freelist, +4 RpLightTie freelist, +8 dword (0 at open);
open counter 0xc925a0; static freelists 0xc92578 / 0xc92554; params 0x8d62e8 = 0x100, 0x8d62ec = 1, 0x8d62f0 = 0x20,
0x8d62f4 = 1. WORLDMODULE `[0xc9254c]` (0xc): +0 world-registration freelist (0x10-byte nodes `{RpWorld* world; i32 size;
RwLLLink link}`), +4 RwLinkList of registered worlds; counter 0xc92550; static freelist 0xc92528. PIPEMODULE `[0xc9bc60]`:
+0x3c default atomic pipeline, +0x40 default sector pipeline, +0x54 D3D9 atomic all-in-one pipeline, +0x58 D3D9 sector
pipeline (0x75e2b8, 0x75e288, 0x74ebcc, 0x74eb1c). RwGlobals +0x4 curWorld, +0x8 u16 renderFrame (0x750449, 0x750453),
+0xa0 stdFunc[22] = rwSTANDARDHINTRENDERF2B (0x7504c8). Other counters: SECTORMODULE 0xc94c0c, BINWORLDMODULE 0xc94c24,
RXWORLDDEVICEMODULE 0xc926e4. D3D9: previous device restore callback saved at 0xc9251c; dynamic-geometry list head 0xc92520
(nodes `{RpGeometry* geom; node* next}`, maintained by `_rpD3D9AddDynamicGeometry` 0x74e970 / `RemoveDynamicGeometry` 0x74e9e0).

## 0x74ea40 _rpWorldDeviceRegisterPlugins  (catalogue: unknown/REA, port level 4)
- cdecl `RwBool (void)`. Callers: RpWorldPluginAttach.
- `ok = _rpD3D9LightRegisterPlugin() (0x755b90) != 0 && _rpWorldDeviceRegisterPlugins2() (0x7587d0) != 0` (short-circuit).
  Then ALWAYS: `[0xc9251c] = _rwD3D9DeviceGetRestoreCallback()` (0x7fae30); `_rwD3D9DeviceSetRestoreCallback(0x74ea80)`
  (0x7fae20). Return ok (1/0).

## 0x74ea80 _rpD3D9WorldRestoreCallback  (catalogue: specified/REA, port level 0)
- cdecl `void (void)`. If `[0xc9251c]` call it (no args). Then for each node of the list at 0xc92520 (`node = node->next`
  at +4): `node->geom->lockedSinceLastInst (+0xc) = 0xfff` (forces re-instancing of dynamic geometry after device reset).

## 0x74eab0 _rpWorldPipeDummyTrue  (catalogue: unknown/REA, port level 0)
- `RwBool (void)`: return 1. (Called first by `_rpWorldPipeOpen` 0x75e39d.)

## 0x74ead0 _rpCreateD3D9WorldSectorAllInOnePipe  (catalogue: specified/REA, port level 5)
- cdecl `RwBool (void)`. `p = RxPipelineCreate()` (0x8057b0); NULL -> 0. `p->pluginId (+0x2c) = 2`. `l = RxPipelineLock(p)`
  (0x806990); NULL -> `_rxPipelineDestroy(p)` (0x805820), 0. `l = RxLockedPipeUnlock(RxLockedPipeAddFragment(l, NULL,
  RxNodeDefinitionGetD3D9WorldSectorAllInOne() /*0x75e9f0*/, NULL))` (0x806be0, 0x805f40; Unlock gets AddFragment's result
  even if NULL); NULL -> destroy p, 0. `PIPE+0x58 = p`; `RpWorldSetDefaultSectorPipeline(p)`; return 1.

## 0x74eb80 _rpCreateD3D9AtomicAllInOnePipe  (catalogue: specified/REA, port level 5)
- Same as 0x74ead0 with `RxNodeDefinitionGetD3D9AtomicAllInOne()` (0x7582e0): `PIPE+0x54 = p`; `RpAtomicSetDefaultPipeline(p)`
  (0x75e2a0). Then `_rwD3D9LightsOpen()` (0x755d80); 0 -> `_rxPipelineDestroy(p)` and return 0 (PIPE+0x54 / +0x3c keep the
  dangling pointer; original). Then `0x760cf0()` (proposed `_rpD3D9VertexShaderCacheReset`, result ignored); return 1.

## 0x74f210 RpWorldUnlock  (catalogue: specified/REA, port level 7)
- Convention: cdecl `RpWorld* (RpWorld* world)`; returns world or NULL. Callers: RpWorldCreate, world stream read.
- Behaviour: `s = world->rootSector`; NULL -> return NULL. Depth-first walk with a local stack (array at frame+0x40, no
  overflow check; index starts at 0, pushes pre-increment):
  - Plane sector (`type >= 0`): push `right` (+0xc), continue with `left` (+0x8).
  - Atomic sector with `mesh != NULL`: pop.
  - Atomic sector without mesh: `matBase = world->matList.materials + sector->matListWindowBase*4`;
    `bm = _rpBuildMeshCreate(sector->numTriangles (u16))`; NULL -> return NULL. If `world->matList.numMaterials != 0`:
    three arrays of `numMaterials*4` bytes (malloc hints 0x10006, 0x10507, 0x10507; unchecked); for each triangle i
    (`i < numTriangles`, re-read): `mat = matBase[tri.matIndex (+6)]`; find-or-append `mat->texture`, `texture->raster`
    (NULL if no texture), `mat->pipeline (+8)` exactly as RpGeometryUnlock (16-bit indices); `_rpBuildMeshAddTriangle(bm, mat,
    v0, v1, v2, tri.matIndex, ti, ri, pi)`; free the three arrays. `m = _rpMeshOptimise(bm, (world->flags & 1) ? 1 : 0)`;
    NULL -> `_rpBuildMeshDestroy(bm)`, return NULL (earlier sectors keep their meshes). Else `sector->mesh = m`; pop.
  - Pop: `s = stack[idx]; idx--`; stop when idx < 0. Return world.
- librw: absent (librw has no RpWorldUnlock; world.cpp differs).

## 0x74f760 RpWorldCreate  (catalogue: specified/REA, port level 8)
- Convention: cdecl `RpWorld* (const RwBBox* bbox)` (bbox = {sup +0, inf +0xc}).
- Behaviour:
  1. `w = Engine->malloc([0x8d62d0], 0x3000b)`; NULL -> error `{2, _rwerror(0x80000013, [0x8d62d0])}`, NULL.
  2. `type 7`, sub/flags/private 0, `parent 0`; `_rpMaterialListInitialize(&w->matList)`; `flags = 0`; `renderOrder = 2`.
  3. `s = Engine->malloc([0x8d736c], 0x3000b)`; NULL -> error `{2, _rwerror(0x80000013, 4)}` (constant 4 as size; original),
     `Engine->free(w)`, NULL.
  4. Sector: both tie lists empty; `type = -1`; repEntry, mesh = 0; numVertices = numTriangles = 0; vertices, triangles,
     normals = 0; texCoords[0..7] = 0; preLitLum = 0; `boundingBox = tightBoundingBox = *bbox`; pipeline = 0.
     matListWindowBase and pad are not written.
  5. World: `rootSector = s; numTexCoordSets = 0; worldOrigin = {0,0,0}; boundingBox = *bbox; numClumpsInWorld = 0;
     clumpList` empty, `currentClumpLink = &clumpList`; lightList and directionalLightList empty; `renderCallBack = 0x74eec0`;
     `pipeline = 0`.
  6. Register: `n = Engine->memoryAlloc(WORLDMODULE+0, 0x40507)`; if non-NULL: `n->world = w; n->size = [0x8d62d0]`; insert
     `n+8` at the head of the WORLDMODULE+4 list. (Allocation failure is ignored.)
  7. `_rwPluginRegistryInitObject(0x8d62d0, w)`, then `_rwPluginRegistryInitObject(0x8d736c, s)`.
  8. `RpWorldUnlock(w)` non-NULL -> return w.
  9. Failure: find the registration node whose `world == w` (walk list, node = link - 8), unlink, free it to WORLDMODULE+0.
     Walk the BSP (same stack scheme) and `_rpMeshDestroy` + NULL every atomic sector mesh. `_rpMaterialListDeinitialize`.
     If `w->privateFlags & 1`: `_rpWorldSectorDeinstanceAll(root)` (0x74eca0) if root; else `_rpWorldSectorDestroyRecurse(root)`
     (0x74ed50) if root. `DeInitObject(0x8d62d0, w)`; `Engine->free(w)`; return NULL.
- librw: world.cpp differs (no RpWorldCreate equivalent with a bbox; separate structures).

## 0x74fcd0 RpWorldRegisterPlugin  (catalogue: specified/REA, port level 2)
- cdecl `i32 (size, pluginID, ctor, dtor, copy)` -> `_rwPluginRegistryAddPlugin(0x8d62d0, ...)` 0x8084a0.

## 0x74fd00 RpWorldRegisterPluginStream  (catalogue: specified/REA, port level 1)
- cdecl `i32 (pluginID, readCB, writeCB, getSizeCB)` -> `_rwPluginRegistryAddPluginStream(0x8d62d0, ...)` 0x8088e0.

## 0x74fda0 RpWorldPluginAttach  (catalogue: specified/REA, port level 5)
- cdecl `RwBool (void)`. Calls `RwEngineRegisterPlugin(size, id, open, close)` (0x7f2bb0) in this order: (4, 0x501 MATERIAL,
  0x74d8e0, 0x74d950), (0x30, 0x502 MESH, 0x758970, 0x758940), (4, 0x503 GEOMETRY, 0x74bfa0, 0x74bfc0), (8, 0x504 CLUMP,
  0x749760, 0x749480), (4, 0x505 LIGHT, 0x752270, 0x7521b0), (0, 0x50a SECTOR, 0x761c30, 0x761c40), (0xc, 0x507 WORLD,
  0x74fee0, 0x74fe90), (0, 0x50b BINWORLD, 0x7637e0, 0x7637d0). All are attempted; the results are OR-ed and if the OR is
  negative (any -1) return 0. Then `_rpWorldObjRegisterPlugins()` 0 -> 0; `_rpClumpRegisterExtensions()` 0 -> 0;
  return `_rpWorldDeviceRegisterPlugins() != 0`.

## 0x74fee0 cb_WORLDMODULE_RwEngine_Open  (catalogue: specified/REA, port level 7)
- `[0xc9254c] = offset`. `_rpWorldPipeOpen()` 0 -> NULL. Freelist `RwFreeListCreateAndPreallocateSpace(0x10, 8, 4, 1, 0xc92528,
  0x4000b)` at `Engine+offset+0`; NULL -> NULL (pipelines are not closed). `Engine+offset+4` = empty list (next = prev = self).
  `[0xc92550]++`; return instance.

## 0x750050 _rpWorldObjRegisterPlugins  (catalogue: specified/REA, port level 3)
- cdecl `RwBool (void)`. Registrations in order (all attempted, results OR-ed):
  RwEngineRegisterPlugin(0xc, 0x509, 0x750280, 0x750370); `[0xc925a4] = RwCameraRegisterPlugin(0x1c, 0x509, 0x7503e0, 0x750760, 0x750710)`;
  `[0xc925a8] = RpAtomicRegisterPlugin(8, 0x509, 0x7507b0, 0x7509b0, 0x7509a0)`; `[0xc925ac] = RpClumpRegisterPlugin(8, 0x509,
  0x750a30, 0x750a90, 0x750a60)`; `[0xc925b0] = RpLightRegisterPlugin(8, 0x509, 0x750aa0, 0x750d30, 0x750cd0)`;
  RpGeometryRegisterPlugin(0, 0x50e, 0,0,0); RpWorldSectorRegisterPlugin(0, 0x50e, 0,0,0); RpGeometryRegisterPluginStream(0x50e,
  0x750db0, 0x750d90, 0x750de0); RpWorldSectorRegisterPluginStream(0x50e, 0x750ed0, 0x750ea0, 0x750f00); RpGeometryRegisterPlugin(0,
  0x510, ...); RpWorldSectorRegisterPlugin(0, 0x510, ...); RpGeometryRegisterPluginStream(0x510, 0x750e20, 0x750e00, 0x750e40);
  RpWorldSectorRegisterPluginStream(0x510, 0x750e70, 0x750e50, 0x750e90); RpAtomicRegisterPlugin(0, 0x1f, ...);
  RpAtomicRegisterPluginStream(0x1f, 0x749120, 0x749160, 0x7491a0); RpWorldRegisterPlugin(0, 0x1f, ...);
  RpWorldRegisterPluginStream(0x1f, 0x761d60, 0x761da0, 0x761de0); RpWorldSectorRegisterPlugin(0, 0x1f, ...);
  RpWorldSectorRegisterPluginStream(0x1f, 0x761e00, 0x761e40, 0x761e80); RpMaterialRegisterPlugin(0, 0x1f, ...);
  RpMaterialRegisterPluginStream(0x1f, 0x74d7d0, 0x74d810, 0x74d850).
- If the OR is negative return 0; else return `_rpWorldObjDummyTrue() != 0` (= 1).

## 0x750280 cb_WORLDOBJMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- `[0xc9259c] = offset`. Tie freelist `RwFreeListCreateAndPreallocateSpace(0x18, [0x8d62e8], 4, [0x8d62ec], 0xc92578, 0x40507)` at
  `+0`; NULL -> NULL. Light-tie freelist `(0x18, [0x8d62f0], 4, [0x8d62f4], 0xc92554, 0x40507)` at `+4`; NULL -> `RwFreeListDestroy(+0)`,
  `+0 = 0`, NULL. Then `Engine->renderFrame (+0x8, u16) = 1` (0x750338); `+8 = 0`; `[0xc925a0]++`; return instance.

## 0x7503e0 cb_WORLDOBJMODULE_RwCamera_Ctor  (catalogue: specified/REA, port level 0)
- `void* (RwCamera* cam, i32, i32)`: `ext = cam + [0xc925a4]`; `ext+0 = ext+4 = ext+8 = 0`; `ext+0x10 = cam->beginUpdate`;
  `ext+0x14 = cam->endUpdate`; `ext+0x18 = cam->sync`; `cam->sync = 0x750480; cam->beginUpdate = 0x750430;
  cam->endUpdate = 0x750460`; `ext+0xc (world) = 0`. Returns cam.

## 0x750430 _rpWorldObjCameraBeginUpdate  (catalogue: specified/REA, port level 0)
- cdecl `RwCamera* (RwCamera* cam)`: `Engine->curWorld (+4) = ext->world`; `Engine->renderFrame (+8, u16)++`; tail-jump to
  `ext->savedBeginUpdate(cam)` (its return value is returned).

## 0x750460 _rpWorldObjCameraEndUpdate  (catalogue: specified/REA, port level 0)
- cdecl `RwCamera* (RwCamera* cam)`: `Engine->curWorld = NULL`; tail-jump to `ext->savedEndUpdate(cam)`.

## 0x750480 _rpWorldObjCameraSync  (catalogue: specified/REA, port level 4)
- Convention: cdecl `RwCamera* (RwCamera* cam)` (camera frame-sync callback). Returns cam, or NULL if the saved sync failed.
- Behaviour:
  1. `if (!ext->savedSync(cam)) return NULL`. `world = ext->world`; NULL -> return cam.
  2. `count = 0`; `hint = Engine->stdFunc[22] (+0xa0)`; `pos = RwFrameGetLTM(cam->parent)->pos (+0x30)` (copied);
     `inf = cam->frustumBoundBox.inf (+0x118)`, `sup = cam->frustumBoundBox.sup (+0x10c)` (copied);
     `b2f = (world->renderOrder == 2)`.
  3. Walk from `world->rootSector` with a local stack (frame+0x54, pre-increment push, no overflow check):
     - Atomic sector (type < 0): for each of the 6 frustum planes p (in order 0..5): `x = bbox[closestX].x`, `y = bbox[closestY].y`,
       `z = bbox[closestZ].z` where `bbox[0] = sector->boundingBox.sup (+0x48)`, `bbox[1] = .inf (+0x54)`;
       `d = (p.ny*y + p.nx*x) + p.nz*z - p.distance` (x87 order: ny*y, then nx*x added, then nz*z added, then distance
       subtracted; stored f32). If the stored f32 bits as a SIGNED int are > 0 (i.e. d > +0, or a positive NaN) the sector is
       outside: skip it. If all six pass: append the sector to the ext array: if `count >= capacity` (signed):
       `size = capacity*4 + 0xc8`; `p = ext->array ? Engine->realloc(array, size, 0x1030507) : Engine->malloc(size, 0x1030507)`;
       NULL -> error `{2, _rwerror(0x80000013, size)}`, `ext->count = count`, go to step 4; else `array = p; capacity += 0x32`.
       `array[count++] = sector`. Then pop.
     - Plane sector: `a = inf[type] - leftValue` and `b = rightValue - sup[type]` (both stored f32). If bits(a) < 0 and bits(b) < 0
       (signed int tests, so -0.0 counts as negative): the box straddles: `gt = (pos[type] > sector->value)` (fcomp; NaN/equal ->
       false). Then: b2f && gt -> push right, go left; b2f && !gt -> push left, go right; !b2f && gt -> push
       left, go right; !b2f && !gt -> push right, go left. Else if bits(a) >= 0 -> go right; else (a < 0, b >= 0) -> go left.
     - Pop: `s = stack[idx]; idx--`; loop while idx >= 0.
  4. `ext->count = count`; `hint(NULL, NULL, world->renderOrder == 1)`; return cam.
- librw: world.cpp/camera.cpp differ (librw keeps no visible-sector list).

## 0x7507b0 cb_WORLDOBJMODULE_RpAtomic_Ctor  (catalogue: specified/REA, port level 0)
- `ext = atomic + [0xc925a8]`: `ext->world = 0`; `atomic->renderFrame (+0x60) = Engine->renderFrame - 1` (u16);
  `ext->savedSync = atomic->sync`; `atomic->sync = 0x7507e0`. Returns atomic.

## 0x7507e0 _rpWorldObjAtomicSync  (catalogue: specified/REA, port level 5)
- Convention: cdecl `RpAtomic* (RpAtomic* atomic)`. Returns atomic, or NULL if the saved sync returned NULL.
- Behaviour:
  1. `if (!ext->savedSync(atomic)) return NULL`. `world = ext->world`; NULL -> return atomic.
  2. Remove all ties: for each link L in `atomic->llWorldSectorsInAtomic (+0x64)` (next read first): `tie = L - 0xc`; if
     `tie->apAtom != NULL && tie->worldSector != NULL`: unlink `tie+0xc` and `tie+0`, free tie to WORLDOBJ +0. (Ties failing the
     test stay linked.)
  3. `sph = RpAtomicGetWorldBoundingSphere(atomic)`. Box: `inf.x = c.x - r`, `inf.y = c.y - r`, `inf.z = c.z - r`, then
     `sup.x = c.x + r`, `sup.y`, `sup.z` (each from f32 copies, stored f32).
  4. Walk from `world->rootSector` (local stack at frame+0x2c):
     - Atomic sector: `tie = Engine->memoryAlloc(WORLDOBJ+0, 0x30507)` (result NOT checked); `tie->worldSector = sector;
       tie->apAtom = atomic`; insert `tie+0` at the head of `sector->collAtomicsInWorldSector (+0x38)`; insert `tie+0xc` at the head
       of `atomic->llWorldSectorsInAtomic`. Pop.
     - Plane sector: if `inf[type] < leftValue` (strict, ordered; `test ah,5; jp`): go left, and additionally push right if
       `sup[type] > rightValue` (strict; C0|C3 clear). Else if `sup[type] > rightValue`: go right. Else pop.
  5. Return atomic.
- librw: clump.cpp worldAtomicSync differs (only calls the original sync; no sector ties).

## 0x7509a0 cb_WORLDOBJMODULE_RpAtomic_Copy  (catalogue: specified/REA, port level 0)
- `void* (void* dst, const void* src, i32, i32)`: return dst (the world pointer is not copied).

## 0x7509b0 cb_WORLDOBJMODULE_RpAtomic_Dtor  (catalogue: specified/REA, port level 0)
- `void* (RpAtomic* atomic, i32, i32)`: remove ties exactly as step 2 of 0x7507e0; then `atomic->sync = ext->savedSync`; return
  atomic. (ext->world is not cleared.)

## 0x750a30 cb_WORLDOBJMODULE_RpClump_Ctor  (catalogue: specified/REA, port level 0)
- `ext = clump + [0xc925ac]`: `ext+0 (world) = 0`; `ext+4 = *(Engine + [0xc9259c] + 8)`. Returns clump.
- Open question: meaning of WORLDOBJ engine data +8 / clump ext +4 (only zero is ever seen here; 0x750a4f).

## 0x750a60 cb_WORLDOBJMODULE_RpClump_Copy  (catalogue: specified/REA, port level 3)
- `void* (RpClump* dst, const RpClump* src, i32, i32)`: if `src` ext world != NULL: `RpWorldAddClump(world, dst)` (0x751300);
  return dst.

## 0x750a90 cb_WORLDOBJMODULE_RpClump_Dtor  (catalogue: specified/REA, port level 0)
- Returns clump; nothing else.

## 0x750aa0 cb_WORLDOBJMODULE_RpLight_Ctor  (catalogue: specified/REA, port level 0)
- `ext = light + [0xc925b0]`: `ext->world = 0; ext->savedSync = light->sync; light->sync = 0x750ac0`. Returns light.

## 0x750ac0 _rpWorldObjLightSync  (catalogue: specified/REA, port level 4)
- Convention: cdecl `RpLight* (RpLight* light)`; always returns light.
- Behaviour:
  1. If `ext->savedSync(light) == NULL`: error `{2, _rwerror(0x80000016)}`, return light (differs from the atomic sync).
  2. If `subType < 0x80` (global light) return light. `world = ext->world`, `frame = light->parent`; either NULL -> return light.
  3. `r = light->radius`. Remove all light ties unconditionally (unlink both links, free to WORLDOBJ +4).
  4. `pos = RwFrameGetLTM(frame)->pos`. `sup = pos + r` (x, y, z), then `inf = pos - r` (each stored f32).
  5. Same BSP walk as 0x7507e0 (stack at frame+0x38); leaves: `tie = Engine->memoryAlloc(WORLDOBJ+4, 0x30507)` (unchecked);
     `tie->sect = sector; tie->light = light`; insert `tie+0` at the head of `sector->lightsInWorldSector (+0x40)` and `tie+0xc` at
     the head of `light->WorldSectorsInLight (+0x2c)`.
  6. Return light.

## 0x750d30 cb_WORLDOBJMODULE_RpLight_Dtor  (catalogue: specified/REA, port level 0)
- `void* (RpLight* light, i32, i32)`: unlink and free (to WORLDOBJ +4) every light tie of `light->WorldSectorsInLight`; return light.
  The saved sync is not restored and ext->world is not cleared.

## 0x750f20 RpWorldAddCamera  (catalogue: specified/REA, port level 1)
- cdecl `RpWorld* (RpWorld* world, RwCamera* cam)`: if `cam->parent`: `RwFrameUpdateObjects(parent)` FIRST; then
  `ext->world (+0xc) = world`; return world.

## 0x7587d0 _rpWorldDeviceRegisterPlugins2  (catalogue: specified/REA, port level 3)
- cdecl `RwBool (void)`: `RwEngineRegisterPlugin(0, 0x50f, 0x758890, 0x7588a0)` < 0 -> 0. `[0xc926e8] = RpGeometryRegisterPlugin(4,
  0x50f, 0x758850, 0x758860, 0x758870)`; < 0 -> 0. `[0xc926ec] = RpWorldSectorRegisterPlugin(4, 0x50f, same three)`; return `>= 0`.

## 0x758850 cb_RXWORLDDEVICEMODULE_RpGeometry_RpWorldSector_Ctor  (catalogue: specified/REA, port level 0)
- `void* (void* obj, i32 offset, i32 size)`: `*(u32*)(obj + offset) = 0` (uses the offset argument); return obj.

## 0x758860 cb_RXWORLDDEVICEMODULE_RpGeometry_RpWorldSector_Dtor  (catalogue: specified/REA, port level 0)
- Identical body to 0x758850: `*(obj + offset) = 0`; return obj.

## 0x758890 cb_RXWORLDDEVICEMODULE_RwEngine_Open  (catalogue: specified/REA, port level 0)
- `[0xc926e4]++`; return instance.

## 0x75e270 RpWorldSetDefaultSectorPipeline  (catalogue: specified/REA, port level 0)
- cdecl `RxPipeline* (RxPipeline* p)`: if p: `PIPE+0x40 = p`, return p; else `PIPE+0x40 = PIPE+0x58` and return that value.

## 0x75e310 _rpWorldPipeOpen  (catalogue: specified/REA, port level 6)
- cdecl `RwBool (void)`. Zero PIPEMODULE data +0x3c, +0x40, +0x44, +0x48, +0x4c, +0x50, +0x54, +0x58, +0x5c (in that order).
  Then `0x74eab0()` (always 1) && `_rpCreateD3D9AtomicAllInOnePipe()` && `_rpCreateD3D9WorldSectorAllInOnePipe()` -> return 1.
  On the first failure: `_rpDestroyD3D9WorldSectorPipe()` 0x74eb40, `0x74ec00()` (atomic pipe destroy, see RpAtomic.md callers),
  `0x74eac0()`; return 0.

## 0x75e3e0 _rpWorldObjDummyTrue  (catalogue: unknown/REA, port level 0)
- `RwBool (void)`: return 1. Called at the end of `_rpWorldObjRegisterPlugins` (0x750269).

## 0x761c30 cb_SECTORMODULE_RwEngine_Open  (catalogue: specified/REA, port level 0)
- `[0xc94c0c]++`; return instance.

## 0x761c90 RpWorldSectorRegisterPlugin  (catalogue: specified/REA, port level 2)
- cdecl `i32 (size, pluginID, ctor, dtor, copy)` -> `_rwPluginRegistryAddPlugin(0x8d736c, ...)` 0x8084a0.

## 0x761cc0 RpWorldSectorRegisterPluginStream  (catalogue: specified/REA, port level 1)
- cdecl `i32 (pluginID, readCB, writeCB, getSizeCB)` -> `_rwPluginRegistryAddPluginStream(0x8d736c, ...)` 0x8088e0.

## 0x7637e0 cb_BINWORLDMODULE_RwEngine_Open  (catalogue: specified/REA, port level 0)
- `[0xc94c24]++`; return instance.
