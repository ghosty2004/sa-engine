# RpWorld pass 3: world/sector management, world-object callbacks, world streaming (RE-SPEC-RW-3 lead)

All `__cdecl`. Layouts (RpWorld, RpWorldSector, RpPlaneSector, RpTie, RpLightTie, camera WORLDOBJ extension, module data) are the
ones proven in docs/re/specs/RW/RpWorld.md (pass 1); only additions/corrections are listed here.
Engine = [0xc97b24]. WORLDOBJMODULE data `W = Engine + [0xc9259c]`: +0 RpTie freelist, +4 RpLightTie freelist. WORLDMODULE data
`M = Engine + [0xc9254c]`: +0 registration-node freelist, +4 RwLinkList of registered worlds. RpWorldSector registry 0x8d736c.

Corrections to pass 1 / catalog/rw-names.csv (CAT-17 proposals):
- RpWorldSector +0x38 is the list of RpTie (collision atomics) and +0x40 the list of RpLightTie: 0x74ef60 walks +0x40 and treats
  node+8 as the RpLight; 0x74eca0 frees +0x38 nodes with 0x74fff0 (which returns them to W+0 = RpTie freelist) and +0x40 nodes with
  0x74ffa0 (returns them to W+4 = RpLightTie freelist).
- Therefore the names are swapped: 0x74ffa0 (named _rpWorldSectorTieDestroy) destroys a LIGHT tie, 0x74fff0 (named
  _rpWorldSectorLightTieDestroy) destroys an atomic RpTie. The specs below keep the catalogue names and state the real behaviour.
- Sector type values: < 0 = atomic sector (-1); -2 is treated as "no sector / skip" by the destroy and de-instance walkers
  (0x74eca7, 0x74ed58): such a node is only freed (0x74eea9) or ignored.

Tree walks: several functions walk the BSP tree depth-first with a fixed 64-entry stack of right subtrees on the native stack
(0x100 bytes, e.g. 0x74efa4, 0x74f1a0); a tree deeper than 64 plane levels overflows it (original limitation).

## 0x74eb40 _rpDestroyD3D9WorldSectorPipe  (catalogue: specified/REA, port level 1)
- `void (void)`. `RpWorldSetDefaultSectorPipeline 0x75e270(NULL)`; `p = PIPE+0x58` (`Engine + [0xc9bc60] + 0x58`, D3D9 sector pipeline); if p:
  `_rxPipelineDestroy 0x805820(p)`, `PIPE+0x58 = 0`. Callers _rpWorldPipeOpen/_rpWorldPipeClose.

## 0x74ec00 sub_74ec00 (proposed: _rpDestroyD3D9AtomicPipeAndDynamicGeometry)  (catalogue: unknown/REA, port level 1)
- `void (void)`, called by _rpWorldPipeOpen/_rpWorldPipeClose. Walk the dynamic-geometry list head [0xc92520] (nodes `{RpGeometry* geom; next}`):
  for each node: `node->geom = 0`, return the node to the freelist [0xc92524] via Engine+0x148, follow `next` (read after the free, 0x74ec24);
  then [0xc92520] = 0 (only when the list was non-empty). If [0xc92524]: `RwFreeListDestroy 0x801b80`, [0xc92524] = 0.
- `_rpD3D9VertexShaderCachePurge 0x75ee60()`, `_rwD3D9LightsClose 0x755fe0()`, `RpAtomicSetDefaultPipeline 0x75e2a0(NULL)`; if `PIPE+0x54`
  (D3D9 atomic all-in-one pipeline): `_rxPipelineDestroy`, `PIPE+0x54 = 0`.
- Note: reads `next` from a node already handed back to the freelist (works because freelist free does not clobber +4).

## 0x74eca0 _rpWorldSectorDeinstanceAll  (catalogue: specified/REA, port level 2)
- `void (RpSector* s)`, recursive. type == -2: return. Plane sectors (type >= 0): recurse into left (+8) if non-NULL, then loop on right (+0xc)
  iteratively (tail loop 0x74ecc5); a NULL or -2 right ends it.
- Atomic sector (type == -1): if `repEntry (+0x34)`: `RwResourcesFreeResEntry 0x807de0(repEntry)`; destroy every RpTie of +0x38 with 0x74fff0 and
  every RpLightTie of +0x40 with 0x74ffa0 (next read before each destroy); `_rwPluginRegistryDeInitObject 0x808740(0x8d736c, s)`; if `mesh (+0x78)`:
  `_rpMeshDestroy 0x758bc0`, +0x78 = 0. Sector memory and geometry arrays are kept.

## 0x74ed50 _rpWorldSectorDestroyRecurse  (catalogue: specified/REA, port level 2)
- `void (RpSector* s)`, recursive. type == -2: free s (Engine+0x138), return. Plane: recurse left, `+8 = 0`, recurse right, `+0xc = 0`, free s.
- Atomic sector: as 0x74eca0 (res entry, ties, plugin de-init), then free and zero, in this order: +0x8 vertices, +0xc normals, +0x30 preLitLum,
  +0x4 triangles, texCoords +0x10..+0x2c (8 slots); then `_rpMeshDestroy(+0x78)` if set; free s.

## 0x74eec0 _rpSectorDefaultRenderCallBack  (catalogue: specified/REA, port level 1)
- `RpWorldSector* (RpWorldSector* s)`. `numTriangles (u16 +0x84) == 0` -> return s. `pipe = s+0x7c`, else `curWorld (Engine+4)->pipeline (+0x6c)`,
  else `PIPE+0x40` (default sector pipeline). `r = RxPipelineExecute 0x805710(pipe, s, 1)`; return `r ? s : NULL`.

## 0x74ef10 _rpWorldForAllGlobalLights  (catalogue: specified/REA, port level 0)
- `RwBool (RpLightCallBack cb [esp+4], void* data [esp+8])`. Walk `curWorld (Engine+4)->directionalLightList (+0x3c)`: for each link,
  `next` read first, `light = link - 0x34`; if light != NULL call `cb(light, data)`; 0 stops. Returns 1.

## 0x74ef60 _rpWorldSectorForAllLocalLights  (catalogue: specified/REA, port level 0)
- `RpWorldSector* (RpWorldSector* s, RpLightCallBack cb, void* data)`: walk the RpLightTie list at s+0x40; `light = tie+8`; non-NULL ->
  `cb(light, data)`; 0 stops. Returns s.

## 0x74efa0 _rpWorldFindBBox  (catalogue: specified/REA, port level 2)
- `RwBool (RpWorld* w, RwBBox* box)`. DFS from `w->rootSector (+0x1c)` (plane: push right +0xc, descend left +8). For each atomic sector in
  left-first order: the first one `RwBBoxInitialize 0x809020(box, &s->tightBoundingBox.inf (+0x6c))`, later ones `RwBBoxAddPoint 0x809060(box, s+0x6c)`;
  then always `RwBBoxAddPoint(box, s+0x60 sup)`. Returns 1. (No NULL checks; an empty tree is not possible here.)

## 0x74f020 _rpWorldSetupSectorBoundingBoxes  (catalogue: specified/REA, port level 2)
- `RpWorld* (RpWorld* w)`. Current box = copy of `w->boundingBox (+0x50, 6 dwords: sup then inf)`. DFS from root with a parallel stack of boxes
  (0x18 each). Plane node (type = axis byte offset): push the right child with a copy of the current box whose `inf[axis] = rightValue (+0x14)`
  (fld/fstp f32, 0x74f081/0x74f099); then the current box gets `sup[axis] = leftValue (+0x10)` (0x74f09c/0x74f0a2) and the walk descends left.
  Atomic sector: copy the current box to `s->boundingBox (+0x48)`, then pop the next (sector, box). Returns w.

## 0x74f0c0 _rpWorldRegisterWorld  (catalogue: specified/REA, port level 1)
- `void (RpWorld* w, RwInt32 size)`. `n = Engine+0x144(M+0, 0x40507)`; NULL -> return. `n = {w, size}`; insert `n+8` at the HEAD of the list
  M+4 (`link.next = first`, `link.prev = &list`, `first.prev = link`, `list.first = link`). EAX on return is not meaningful.

## 0x74f140 _rpWorldUnregisterWorld  (catalogue: specified/REA, port level 1)
- `void (RpWorld* w)`. Find the first node of M+4 with `node.world == w` (node = link - 8); unlink it and return it to the M+0 freelist
  (Engine+0x148). Not found: nothing.

## 0x74f1a0 RpWorldLock  (catalogue: specified/REA, port level 0)
- `RpWorld* (RpWorld* w)`. `root == NULL` -> return NULL. DFS over all atomic sectors: if `mesh (+0x78)`: `_rpMeshDestroy`, +0x78 = 0. Returns w.
  No lock flag is set (RW 3.6 semantics: locking only drops the meshes; RpWorldUnlock rebuilds them).

## 0x74f4e0 RpWorldSectorGetWorld  (catalogue: specified/REA, port level 0)
- `RpWorld* (RpWorldSector* s)`. Walk the registered-world list M+4 (node = link - 8: +0 world, +4 size). For each world w: if
  `privateFlags (byte w+3) & 1` (world, sectors and arrays in one allocation, 0x74f509): return w when `w <= s < w + node.size` (unsigned);
  otherwise skip. Else DFS w's tree (64-entry stack) and return w if an atomic sector pointer equals s. Not found -> NULL.

## 0x74f570 RpWorldRender  (catalogue: specified/REA, port level 0)
- `RpWorld* (RpWorld* w)`: `RwCameraForAllSectorsInFrustum 0x751660(curCamera (Engine+0), _rpWorldRenderSectorCB 0x74f590, w)`; returns w.

## 0x74f590 _rpWorldRenderSectorCB  (catalogue: specified/REA, port level 0)
- `RpWorldSector* (RpWorldSector* s, void* data)`. If `RpWorldSectorRender 0x761c50(s) != 0`: for each RpTie of s+0x38 (atomic = tie+8):
  if `atomic.flags (byte +2) & 4` (rpATOMICRENDER) and `atomic+0x60 (u16 renderFrame) != Engine+8 (u16)`: if
  `RwCameraFrustumTestSphere 0x7ee2d0(curCamera, RpAtomicGetWorldBoundingSphere 0x749330(atomic)) != 0` call `atomic->renderCallBack (+0x48)(atomic)`;
  then (culled or not) `atomic+0x60 = Engine+8` (0x74f5f5). Returns s. An atomic spanning several sectors renders once per frame.

## 0x74f610 RpWorldDestroy  (catalogue: specified/REA, port level 2)
- `RwBool (RpWorld* w)`. Unregister (inlined 0x74f140 logic). DFS: destroy every atomic sector's mesh (+0x78 -> 0). `_rpMaterialListDeinitialize
  0x74e150(w+0x10)`. If `privateFlags & 1`: `_rpWorldSectorDeinstanceAll(root)` (if root), else `_rpWorldSectorDestroyRecurse(root)` (if root).
  `_rwPluginRegistryDeInitObject(0x8d62d0, w)`; free w (Engine+0x138); return 1. Clumps/lights still in the world are not removed.

## 0x74f730 RpWorldSetSectorRenderCallBack  (catalogue: specified/REA, port level 0)
- `RpWorld* (RpWorld* w, RpWorldSectorCallBackRender cb)`: `w+0x68 = cb ? cb : 0x74eec0`; returns w (EAX = w).

## 0x74f750 RpWorldGetSectorRenderCallBack  (catalogue: specified/REA, port level 0)
- `RpWorldSectorCallBackRender (RpWorld* w)`: returns `w+0x68`.

## 0x74fb80 RpWorldForAllClumps  (catalogue: specified/REA, port level 0)
- `RpWorld* (RpWorld* w, RpClumpCallBack cb, void* data)`: list at w+0x2c; `clump = link - 0x20`; next read before the call; `cb(clump, data)`
  returning NULL stops. Returns w.

## 0x74fbc0 RpWorldForAllMaterials  (catalogue: specified/REA, port level 0)
- `RpWorld* (RpWorld* w, RpMaterialCallBack cb, void* data)`: for i = 0 .. `w+0x14` numMaterials - 1 (count read once, signed): `cb(materials (+0x10)[i], data)`;
  NULL stops. Returns w.

## 0x74fc00 RpWorldForAllLights  (catalogue: specified/REA, port level 0)
- `RpWorld* (RpWorld* w, RpLightCallBack cb, void* data)`: first the directional list w+0x3c, then the local list w+0x34; `light = link - 0x34`;
  next read before the call; a NULL return from cb ends the WHOLE function (the second list is skipped, 0x74fc2c). Returns w.

## 0x74fc70 RpWorldForAllWorldSectors  (catalogue: specified/REA, port level 1)
- `RpWorld* (RpWorld* w, RpWorldSectorCallBack cb, void* data)`: DFS from root (left first, 64-entry right stack): every atomic sector s:
  `cb(s, data)`; NULL stops. Returns w. No NULL-root check.

## 0x74fe90 cb_WORLDMODULE_RwEngine_Close  (catalogue: specified/REA, port level 1)
- Engine close callback: if `M+0` freelist: `RwFreeListDestroy`, `M+0 = 0`; `_rpWorldPipeClose 0x75e300()`; `[0xc92550] -= 1`; returns instance.

## 0x74ffa0 _rpWorldSectorTieDestroy  (catalogue: specified/REA, port level 0)
- `RwBool (RpLightTie* t)` - despite the name this destroys a LIGHT tie (CAT-17 swap, see the header): unlinks `t+0xc` (link in the light) and `t+0`
  (link in the sector) unconditionally and returns t to the RpLightTie freelist `W+4` (Engine+0x148, 0x74ffd5). Returns 1.

## 0x74fff0 _rpWorldSectorLightTieDestroy  (catalogue: specified/REA, port level 0)
- `RwBool (RpTie* t)` - destroys an ATOMIC tie: only if `t+8` (atomic) and `t+0x14` (sector) are both non-NULL: unlink `t+0xc` and `t+0`, return t to the RpTie
  freelist `W+0` (0x75002e). Returns 1 in all cases.

## 0x750370 cb_WORLDOBJMODULE_RwEngine_Close  (catalogue: specified/REA, port level 1)
- Destroys `W+4` (RpLightTie freelist) then `W+0` (RpTie freelist), zeroing each slot; `[0xc925a0] -= 1`; returns instance.

## 0x750710 cb_WORLDOBJMODULE_RwCamera_Copy  (catalogue: specified/REA, port level 2)
- `RwCamera* (RwCamera* dst [esp+4], RwCamera* src [esp+8], RwInt32 offset, RwInt32 size)`. ext = camera + [0xc925a4]. `dst ext+0, +4, +8 = 0`
  (sector array, capacity, count). If `src ext+0xc` (world) != NULL: if `dst->parent (+4)` frame: `RwFrameUpdateObjects 0x7f0910(frame)`; then
  `dst ext+0xc = world`. Returns dst. The saved update/sync hooks (+0x10..+0x18) are not copied.

## 0x750760 cb_WORLDOBJMODULE_RwCamera_Dtor  (catalogue: specified/REA, port level 0)
- `RwCamera* (RwCamera* cam, ...)`. ext = cam + [0xc925a4]: free `ext+0` (Engine+0x138) if set; `ext+0, +4, +8 = 0`; restore the camera hooks:
  `cam+0x18 beginUpdate = ext+0x10`, `cam+0x1c endUpdate = ext+0x14`, `cam+0x10 sync = ext+0x18`. Returns cam. ext+0xc (world) is left as is.

## 0x750cd0 cb_WORLDOBJMODULE_RpLight_Copy  (catalogue: specified/REA, port level 1)
- `RpLight* (RpLight* dst, RpLight* src, ...)`. ext = light + [0xc925b0]; `w = src ext+0`; NULL -> return dst. `dst ext+0 = w`. If `dst subType
  (byte +1) < 0x80` (unsigned, directional/ambient): list = w+0x3c; else: `RwFrameUpdateObjects(dst->parent)` when the frame is set, list = w+0x34.
  Insert `dst+0x34` (link) at the head of list (`dst+0x38 = list`, `dst+0x34 = first`, `first->prev = link`, `list.first = link`). Returns dst.
  The saved sync (ext+4) is not set and no RpLightTies are created.

Plugin stream callbacks below have the RW signature `(RwStream* stream [esp+4], RwInt32 length [esp+8], void* object [esp+0xc], RwInt32 offset,
RwInt32 size)` (write/read return the stream or NULL, get-size returns RwInt32).

## 0x750d90 cb_BINMESHPLUGIN_RpGeometry_StreamWrite  (catalogue: specified/REA, port level 2)
- Returns `_rpMeshWrite 0x758d70(geom+0x54 meshHeader, geom, stream, geom+0x20 matList)` (push order 0x750d9b-0x750da1).

## 0x750de0 cb_BINMESHPLUGIN_RpGeometry_StreamGetSize  (catalogue: specified/REA, port level 1)
- `object` is taken from [esp+4] here (the get-size signature is `(object, offset, size)`): returns `_rpMeshSize 0x759090(geom+0x54, geom)`.

## 0x750e00 cb_NATIVEDATAPLUGIN_RpGeometry_StreamWrite  (catalogue: specified/REA, port level 2)
- Returns `_rpD3D9NativeDataGeometryStreamWrite 0x7637f0(stream, geom)`.

## 0x750e20 cb_NATIVEDATAPLUGIN_RpGeometry_StreamRead  (catalogue: specified/REA, port level 2)
- `r = _rpD3D9NativeDataGeometryStreamRead 0x763a80(stream, geom)`; returns `r ? stream : NULL`.

## 0x750e50 cb_NATIVEDATAPLUGIN_RpWorldSector_StreamWrite  (catalogue: specified/REA, port level 2)
- Returns `_rpD3D9NativeDataWorldSectorStreamWrite 0x763f20(stream, sector)`.

## 0x750e70 cb_NATIVEDATAPLUGIN_RpWorldSector_StreamRead  (catalogue: specified/REA, port level 2)
- `r = _rpD3D9NativeDataWorldSectorStreamRead 0x7641c0(stream, sector)`; returns `r ? stream : NULL`.

## 0x750ea0 cb_BINMESHPLUGIN_RpWorldSector_StreamWrite  (catalogue: specified/REA, port level 2)
- `w = RpWorldSectorGetWorld 0x74f4e0(sector)`; returns `_rpMeshWrite(sector+0x78, w, stream, w+0x10 matList)` (the WORLD is passed in the object
  slot; w is not NULL-checked).

## 0x750ed0 cb_BINMESHPLUGIN_RpWorldSector_StreamRead  (catalogue: specified/REA, port level 2)
- `w = RpWorldSectorGetWorld(sector)`; `mesh = _rpMeshRead 0x758ec0(stream, w, w+0x10)`; `sector+0x78 = mesh` (also when NULL); returns `mesh ? stream : NULL`.

## 0x750f00 cb_BINMESHPLUGIN_RpWorldSector_StreamGetSize  (catalogue: specified/REA, port level 1)
- `(sector [esp+4], offset, size)`: returns `_rpMeshSize(sector+0x78, RpWorldSectorGetWorld(sector))`.

## 0x750f50 RpWorldRemoveCamera  (catalogue: specified/REA, port level 0)
- `RpWorld* (RpWorld* w, RwCamera* cam)`. ext = cam + [0xc925a4]. If `ext+0xc` (world) != 0: `ext+0xc = 0`, `ext+8` (visible-sector count) = 0,
  return w; else NULL. Does not check that cam belongs to w and does not restore the camera hooks or free the sector array.
- Caller: CGame::ShutdownRenderWare 0x53bb80.

## 0x750f80 RwCameraGetWorld  (catalogue: specified/REA, port level 0)
- `RpWorld* (RwCamera* cam)`: returns `*(cam + [0xc925a4] + 0xc)`.

## 0x750f90 RpWorldAddAtomic  (catalogue: specified/REA, port level 0)
- `RpWorld* (RpWorld* w, RpAtomic* a)`: if `a->parent (+4)`: `RwFrameUpdateObjects 0x7f0910(frame)`; then `*(a + [0xc925a8]) = w` (atomic WORLDOBJ ext +0);
  returns w. Sector ties are created later by the atomic's sync hook, not here.

## 0x750fc0 RpWorldRemoveAtomic  (catalogue: specified/REA, port level 1)
- `RpWorld* (RpWorld* w, RpAtomic* a)`. If `a->repEntry (+0x14)`: `RwResourcesFreeResEntry 0x807de0`. Walk the atomic's tie list a+0x64 (links are
  RpTie+0xc; tie = link - 0xc; next read first): only when `tie->apAtom (+8)` and `tie->worldSector (+0x14)` are both non-NULL, unlink the tie from both
  lists and return it to the RpTie freelist W+0 (Engine+0x148). Then `*(a + [0xc925a8]) = 0`. Returns w. The world argument is not compared with the
  atomic's world.

## 0x751060 RpAtomicForAllWorldSectors  (catalogue: specified/REA, port level 0)
- `RpAtomic* (RpAtomic* a, RpWorldSectorCallBackRender cb, void* data)`: walk a+0x64 (next read first); `cb(link+8 = tie->worldSector, data)`; NULL stops.
  Returns a.

Removal-safe iteration used by the next three functions: before each callback a marker link L on the stack is inserted directly after the
current node (`L.next = cur.next`, `L.prev = cur`, `cur.next.prev = &L`, `cur.next = &L`); after the callback L is unlinked
(`L.prev.next = L.next`, `L.next.prev = L.prev`) and the walk continues with `L.next` until it is the list head. The callback may therefore
remove the current element.

## 0x7510a0 RpWorldSectorForAllAtomics  (catalogue: specified/REA, port level 0)
- `RpWorldSector* (RpWorldSector* s, RpAtomicCallBack cb, void* data)`: RpTie list s+0x38 with the marker scheme; `atomic = tie+8`; if non-NULL
  `cb(atomic, data)`; NULL return unlinks the marker and stops. Returns s.

## 0x751140 RpWorldSectorForAllCollisionAtomics  (catalogue: specified/REA, port level 0)
- Byte-for-byte the same code shape as 0x7510a0 over the same list s+0x38 (there is no separate collision list in this build). Returns s.

## 0x7511e0 RpWorldSectorForAllLights  (catalogue: specified/REA, port level 0)
- `RpWorldSector* (RpWorldSector* s, RpLightCallBack cb, void* data)`: first the local RpLightTie list s+0x40 (`light = tie+8`, NULL skipped) with the marker
  scheme; a NULL return stops and returns s. Then `w = RpWorldSectorGetWorld 0x74f4e0(s)`; if w: the world's directional list w+0x3c (`light = link - 0x34`,
  NULL skipped), same scheme. Returns s.

## 0x751300 RpWorldAddClump  (catalogue: specified/REA, port level 2)
- `RpWorld* (RpWorld* w, RpClump* c)`. ext = c + [0xc925ac]; `frame = c->parent (+4)` read first. Insert the clump link c+0x20 at the head of w's
  clump list w+0x2c (`c+0x24 = &list`, `c+0x20 = first`, `first.prev = link`, `list.first = link`); `w+0x24 numClumps += 1`; `ext+0 = w`.
- `RpClumpForAllAtomics 0x749b70(c, 0x751390, w)`, `RpClumpForAllLights 0x749c00(c, 0x7513d0, w)`, `RpClumpForAllCameras 0x749bb0(c, 0x751420, w)`.
- If frame: `RwMatrixOptimize 0x7f17e0(frame+0x10, NULL)`, `RwFrameUpdateObjects 0x7f0910(frame)`. `ext+4 = W+8` (module dword). Returns w.

## 0x751390 _rpWorldAddAtomicSetWorldCB  (catalogue: specified/REA, port level 1)
- `RpAtomic* (RpAtomic* a, RpWorld* w)`: if `a->parent`: RwFrameUpdateObjects(frame); `*(a + [0xc925a8]) = w`; returns a.

## 0x7513d0 _rpWorldAddLightCB  (catalogue: specified/REA, port level 1)
- `RpLight* (RpLight* l, RpWorld* w)`: `*(l + [0xc925b0]) = w`; list = `subType (byte +1) < 0x80` ? w+0x3c : (RwFrameUpdateObjects(frame) if set, w+0x34);
  head-insert `l+0x34` as in 0x750cd0. Returns l.

## 0x751420 _rpWorldAddCameraCB  (catalogue: specified/REA, port level 1)
- `RwCamera* (RwCamera* cam, RpWorld* w)`: if `cam->parent`: RwFrameUpdateObjects; `*(cam + [0xc925a4] + 0xc) = w`; returns cam.

## 0x751460 RpWorldRemoveClump  (catalogue: specified/REA, port level 2)
- `RpWorld* (RpWorld* w, RpClump* c)`. ext = c + [0xc925ac]; `ext+0 == NULL` -> `RwErrorSet({2, _rwerror(4)})` (plugin id 2, code 4, 0x7514d8), NULL.
- `world = ext+0`: `world+0x24 -= 1`; if `world+0x28` (current clump link of an in-progress iteration) == c+0x20 it advances to `link.next`;
  unlink c+0x20. Then `RpClumpForAllAtomics(c, 0x751500, w)`, `ForAllLights(c, 0x7515a0, w)`, `ForAllCameras(c, 0x751630, w)` (the ARGUMENT w is
  passed, not ext's world); `ext+0 = 0`; returns w.

## 0x751500 _rpWorldRemoveAtomicCB  (catalogue: specified/REA, port level 1)
- `RpAtomic* (RpAtomic* a, RpWorld* w)`: ext = a + [0xc925a8]; only if `ext+0 == w`: free the res entry (+0x14), release its RpTies exactly as
  RpWorldRemoveAtomic 0x750fc0, `ext+0 = 0`. Returns a.

## 0x7515a0 _rpWorldRemoveLightCB  (catalogue: specified/REA, port level 1)
- `RpLight* (RpLight* l, RpWorld* w)`: ext = l + [0xc925b0]; only if `ext+0 == w`: `ext+0 = 0`; for every RpLightTie of the light's list l+0x2c
  (link = tie+0xc; next read first): unlink both links and return the tie to W+4 (RpLightTie freelist); then unlink the light's world link
  (`l+0x38 prev ->next = l+0x34`, `l+0x34 next ->prev = l+0x38`). Returns l.

## 0x751630 _rpWorldRemoveCameraCB  (catalogue: specified/REA, port level 0)
- `RwCamera* (RwCamera* cam, RpWorld* w)`: ext = cam + [0xc925a4]; if `ext+0xc == w` and non-zero: `ext+0xc = 0`, `ext+8 = 0`. Returns cam (EAX
  is never changed after loading cam).

## 0x751660 RwCameraForAllSectorsInFrustum  (catalogue: specified/REA, port level 1)
- `RwCamera* (RwCamera* cam, RpWorldSectorCallBack cb, void* data)`: over the camera's visible-sector array (ext = cam + [0xc925a4]: array ext+0, count
  ext+8, filled by the camera sync hook): `cb(sector, data)` in array order; NULL stops. Returns cam in both cases.

## 0x7516c0 RwCameraForAllClumpsInFrustum  (catalogue: specified/REA, port level 1)
- `RwCamera* (RwCamera* cam, void* data)` (RW 3.6 signature: the second argument is the data passed to each clump's own callback).
  `stamp = ++(W+8)` (module dword used as an iteration stamp, 0x7516da). For each visible sector (array order) and each RpTie of its list +0x38
  (marker scheme): `atomic = tie+8`, `clump = atomic+0x3c`; if clump and `*(clump + [0xc925ac] + 4) != stamp` and
  `RwCameraFrustumTestSphere(cam, RpAtomicGetWorldBoundingSphere(atomic)) != 0`: set that stamp, call `clump->callBack (+0x28)(clump, data)` (0x75177b);
  NULL -> unlink the marker, return NULL (0x7517ca). Returns cam.

## 0x7517f0 RwCameraForAllAtomicsInFrustum  (catalogue: specified/REA, port level 1)
- `RwCamera* (RwCamera* cam, RpAtomicCallBack cb, void* data)`. `stamp = ++(W+8)` (32-bit). For each visible sector and RpTie (marker scheme): atomic =
  tie+8; if atomic and `(u32)(u16)atomic+0x60 != stamp` (0x751871: the 16-bit field is zero-extended and compared with the 32-bit stamp, so once the stamp
  exceeds 0xffff the test never matches and an atomic in several sectors is reported once per sector) and the frustum sphere test passes:
  `atomic+0x60 = (u16)stamp`, `cb(atomic, data)`; NULL -> unlink marker, return NULL. Returns cam.
- Note: atomic+0x60 is the same u16 that _rpWorldRenderSectorCB compares with the engine render frame (Engine+8), so the two mechanisms share the field.

## 0x7519f0 RpLightForAllWorldSectors  (catalogue: specified/REA, port level 1)
- `RpLight* (RpLight* l, RpWorldSectorCallBack cb, void* data)`. Directional/ambient (`subType (byte +1) < 0x80`): `w = *(l + [0xc925b0])`; no world ->
  return l; `RpWorldForAllWorldSectors 0x74fc70(w, cb, data)` returning NULL -> return NULL, else l. Local lights: RpLightTie list l+0x2c (link = tie+0xc,
  sector = link+8; next read first): `cb(sector, data)`; NULL stops; returns l.

## 0x761c50 RpWorldSectorRender  (catalogue: specified/REA, port level 0)
- `RpWorldSector* (RpWorldSector* s)`: tail-jump to `curWorld (Engine+4)->renderCallBack (+0x68)` with the caller's stack (s as the only argument).

## 0x761c60 RpWorldSectorForAllMeshes  (catalogue: specified/REA, port level 1)
- `RpWorldSector* (RpWorldSector* s, RpMeshCallBack cb, void* data)`: mesh = s+0x78; NULL -> NULL; `r = _rpMeshHeaderForAllMeshes 0x758d30(mesh, cb, data)`;
  returns `r ? s : NULL`.

## 0x761da0 cb_RIGHTTORENDER_RpWorld_StreamWrite  (catalogue: specified/REA, port level 3)
- `(stream, len, RpWorld* w, ...)`: `pipe = w+0x6c` (not NULL-checked); `RwStreamWriteInt32 0x7ed460(stream, pipe+0x2c pluginId, 4)`; 0 -> return 0; then
  `RwStreamWriteInt32(stream, pipe+0x30 pluginData, 4)`; returns `r ? stream : NULL`.

## 0x761de0 cb_RIGHTTORENDER_RpWorld_StreamGetSize  (catalogue: specified/REA, port level 1)
- `(RpWorld* w, ...)`: returns 8 when `w+0x6c` pipeline is set and its `pluginId (+0x2c) != 0`, else 0 (so the write callback is only invoked with a pipeline).

## 0x761e40 cb_RIGHTTORENDER_RpWorldSector_StreamWrite  (catalogue: specified/REA, port level 3)
- As 0x761da0 with the sector pipeline `s+0x7c`.

## 0x761e80 cb_RIGHTTORENDER_RpWorldSector_StreamGetSize  (catalogue: specified/REA, port level 1)
- As 0x761de0 with `s+0x7c`.

Stream-size helper used below, for an atomic sector s of world w with world flags F (`w+8`): `A(s) = _rwPluginRegistryGetSize 0x808b00(0x8d736c, s) + D(s)`
where `D(s) = 0x2c` if `F & 0x1000000` (native data, geometry sized by the native plugin), else, with `n = s+0x82` (u16 numVertices) and `t = s+0x84`
(u16 numTriangles): `D = 0x2c + 12n + (F & 0x10 ? 4n : 0) + (F & 8 ? 4n : 0) + 8*(w+0x20 numTexCoordSets * n + t)` (0x761edd-0x761f0f; the flag
tests use the low byte of F).

## 0x761ea0 RpWorldStreamGetSize  (catalogue: specified/REA, port level 3)
- `RwUInt32 (RpWorld* w)`. `m = _rpMaterialListStreamGetSize 0x74e450(w+0x10) + 0x58`. Root r = w+0x1c:
  - r atomic: `body = m + A(r) + 0x24`.
  - r plane: left L: atomic -> `x = A(L) + 0x48`, plane -> `x = 0x762040(L, w, F) + 0x30`; right R: atomic -> `x += A(R) + 0x24`, plane ->
    `x += 0x762040(R, w, F) + 0xc`; `body = m + x + 0xc`.
- Returns `_rwPluginRegistryGetSize(0x8d62d0, w) + body + 0xc`. (The asymmetric 0x48/0x24 and 0x30/0xc constants put the plane's own
  chunk overhead on the left branch; reproduce exactly.)

## 0x762040 _rpWorldPlaneSectorStreamGetSize  (catalogue: specified/REA, port level 2)
- `RwUInt32 (RpPlaneSector* p, RpWorld* w, RwUInt32 F)` (recursive; F passed by the caller = w+8). Left: atomic -> `x = A(L) + 0x48`, plane -> `x = self(L) + 0x30`.
  Right: atomic -> return `x + A(R) + 0x24`; plane -> return `x + self(R) + 0xc`. The native test uses `w+8` directly, the vertex-format tests use the low
  byte of the F argument.

## 0x762440 _rpWorldAtomicSectorStructGetSize  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RpWorldSector* s, RpWorld* w, RwUInt32 F)`: returns `D(s)` as defined above (0x2c when `w+8 & 0x1000000`).

## 0x762150 RpWorldStreamWrite  (catalogue: specified/REA, port level 4)
- `RpWorld* (RpWorld* w [esp+4], RwStream* stream [esp+8])`. `F = w+8`.
- Size: identical arithmetic to RpWorldStreamGetSize 0x761ea0 (left atomic child computed as `0x762440 + 0xc + plugin size + 0x3c`, right atomic as
  `... + 0xc + 0x18`, which equal the 0x48 / 0x24 constants there). `_rwStreamWriteVersionedChunkHeader 0x7ed270(stream, 0xb, size, 0x36003, 0xffff)`; 0 ->
  return 0. Struct header `(stream, 1, 0x40, 0x36003, 0xffff)`; 0 -> 0.
- Counts by a DFS over the tree: numWorldSectors (atomic sectors), numPlaneSectors, numTriangles = sum u16 +0x84, numVertices = sum u16 +0x82.
- 0x40-byte struct S on the stack (0x762327-0x76239e): S+0x0 rootIsWorldSector = `root->type < 0`; S+0x4..+0xc = `w+0x44..+0x4c worldOrigin * [0x858c1c] = -1.0f`
  (x87 fld/fmul/fstp each component); S+0x10 numTriangles; S+0x14 numVertices; S+0x18 numPlaneSectors; S+0x1c numWorldSectors; S+0x20 = 0;
  S+0x24 format = `F | (((w+0x20 numTexCoordSets & 0xff) | 0x4000) << 16)`; S+0x28..+0x3c = w+0x50 boundingBox (6 dwords). `RwStreamWrite 0x7ecb30(stream, S, 0x40)`;
  0 -> 0.
- `_rpMaterialListStreamWrite 0x74e4b0(w+0x10, stream)`; 0 -> 0. Root: atomic -> `_rpWorldAtomicSectorStreamWrite 0x7624a0(root, stream, w, F)`, plane ->
  `_rpWorldPlaneSectorStreamWrite 0x762740(root, stream, w, F)`; 0 -> 0. `_rwPluginRegistryWriteDataChunks 0x808b40(0x8d62d0, stream, w)`; returns
  `r ? w : NULL`.

## 0x7624a0 _rpWorldAtomicSectorStreamWrite  (catalogue: specified/REA, port level 3)
- `RpWorldSector* (RpWorldSector* s [esp+4], RwStream* stream, RpWorld* w, RwUInt32 F)`.
- Chunk header `(stream, 9, plugin size(s) + D(s) + 0x18, 0x36003, 0xffff)`; struct header `(stream, 1, D(s), 0x36003, 0xffff)`; failure -> 0.
- 0x2c-byte struct on the stack: +0x0 matListWindowBase (u16 s+0x80, zero-extended), +0x4 numTriangles, +0x8 numVertices, +0xc..+0x14
  tightBoundingBox.inf (s+0x6c..), +0x18..+0x20 tightBoundingBox.sup (s+0x60..), +0x24 NOT written by the code (uninitialised stack dword written
  to the file - ORIGINAL BUG; the slot at pre-push [esp+0x34] is never stored, 0x76259e-0x7625fb), +0x28 = 0. `RwStreamWrite(stream, &S, 0x2c)`.
- Unless `w+8 & 0x1000000`: if numVertices != 0: `RwStreamWriteReal 0x7ed3d0(stream, s+8 vertices, 12n)`; if `F & 0x10`: `RwStreamWrite(stream, s+0xc normals, 4n)`;
  if `F & 8`: `RwStreamWrite(stream, s+0x30 preLit, 4n)`; for i < `w+0x20` (signed): `RwStreamWriteReal(stream, s+0x10+4i texCoords[i], 8n)`. (numVertices == 0
  skips all of these.) If numTriangles != 0: `RwStreamWriteInt16 0x7ed480(stream, s+4 triangles, 8t)`. Any failure -> 0.
- `_rwPluginRegistryWriteDataChunks(0x8d736c, stream, s)`; returns `r ? s : NULL`.

## 0x762740 _rpWorldPlaneSectorStreamWrite  (catalogue: specified/REA, port level 3)
- `RpPlaneSector* (RpPlaneSector* p, RwStream* stream, RpWorld* w, RwUInt32 F)`, recursive. Size as in 0x762040 for p (left atomic `+0x48`, left plane `+0x30`,
  right atomic `+0x24`, right plane `+0xc`). Chunk header `(stream, 0xa, size, 0x36003, 0xffff)`; struct header `(stream, 1, 0x18, ...)`.
- 0x18-byte struct: +0x0 type, +0x4 value (p+4), +0x8 leftIsWorldSector = `left->type < 0`, +0xc rightIsWorldSector, +0x10 leftValue (p+0x10), +0x14 rightValue
  (p+0x14); `RwStreamWrite(stream, S, 0x18)`.
- Then the left child, then the right child, each with 0x7624a0 or itself (args `(child, stream, w, F)`); any 0 -> 0. Returns p. (No plugin data for plane
  sectors.)

## 0x7635b0 _rpWorldSectorChunkInfoRead  (catalogue: specified/REA, port level 1)
- `void* (RwStream* stream, void* info, RwInt32* bytesRead)`. `RwStreamFindChunk 0x7ed2d0(stream, 1, &len, NULL)` (len is stored over the stream argument
  slot, 0x7635b6); 0 -> 0. Zero 0x2c bytes of info; `RwStreamRead 0x7ec9d0(stream, info, 0x2c)` must return 0x2c, else 0. `*bytesRead = len + 0xc`;
  `RwStreamSkip 0x7ecd00(stream, len - 0x2c)` (unsigned wrap when len < 0x2c). Returns info.

## 0x763620 _rpPlaneSectorChunkInfoRead  (catalogue: specified/REA, port level 1)
- Same as 0x7635b0 with a 0x18-byte struct (zero 6 dwords, read 0x18, skip `len - 0x18`).

## 0x763690 _rpWorldChunkInfoRead  (catalogue: specified/REA, port level 1)
- `void* (RwStream* stream, void* info, RwInt32* bytesRead)`. `RwStreamFindChunk(stream, 1, &len, &version)`; 0 -> 0. Version outside [0x34000, 0x36003]
  (`jb`/`ja`) -> `RwErrorSet({2, _rwerror(0x80000004)})`, 0. Zero 0x40 bytes of info.
- version < 0x34001 (old layout): read `len` bytes into a zeroed 0x34-byte stack buffer B (NOT clamped: len > 0x34 overflows the stack, 0x763704); result
  != len -> 0. Convert: info+0x0..+0xc = B+0x0..+0xc; info+0x10..+0x24 = B+0x1c..+0x30 (B+0x10..+0x1b dropped); info+0x28..+0x3f stay 0 (no bounding box in
  the old format). 
- Else read `len` bytes straight into info (0x40 bytes, also unclamped, 0x763778); != len -> 0.
- `*bytesRead = len + 0xc`; returns info.

## 0x762f30 _rpWorldAtomicSectorStreamRead  (catalogue: specified/REA, port level 3)
- `RpWorldSector* (RwStream* stream [esp+4], RwUInt8** cursor [esp+8], RpWorld* w [esp+0xc], RwUInt32 F [esp+0x10])`. All memory comes from the bump
  cursor `*cursor` (block pre-allocated by RpWorldStreamRead); every allocation is `p = *cursor; *cursor += n;` and the NULL test is made on p AFTER the
  bump.
- `RwStreamFindChunk(stream, 1, NULL, &version)`; 0 -> 0. Version outside [0x34000, 0x36003] -> `RwErrorSet({2, _rwerror(0x80000004)})`, 0.
  `RwStreamRead(stream, S, 0x2c) != 0x2c` -> 0 (no error set).
- `s = alloc([0x8d736c])` (sector registry size); NULL -> `RwErrorSet({2, _rwerror(0x80000013, size)})`, 0. Initialise: `type = -1`; tightBoundingBox
  inf (+0x6c) = S+0xc..+0x14, sup (+0x60) = S+0x18..+0x20; u16 +0x80 = S+0, +0x84 = S+4, +0x82 = S+8; +4, +8, +0x30, +0xc = 0; texCoords[8] = 0; empty
  lists at +0x38 and +0x40; +0x7c, +0x34, +0x78 = 0. (boundingBox +0x48 is not set here.)
- Unless `w+8 & 0x1000000`: if numVertices (S+8) != 0: vertices = alloc(12n) + `RwStreamReadReal 0x7ed4f0(stream, v, 12n)`; if `F & 0x10` normals = alloc(4n) +
  `RwStreamRead` that must return 4n; if `F & 8` preLit (+0x30) = alloc(4n) + `RwStreamRead` that must be non-zero (only a zero test, 0x763151); for i < w+0x20:
  texCoords[i] = alloc(8n) + ReadReal(8n). Then if numTriangles (S+4) != 0: triangles = alloc(8t) + `RwStreamReadInt16 0x7ed4a0(stream, t, 8t)`; if
  `version < 0x36002` each triangle's four u16 are rotated: new = {old[1], old[2], old[3], old[0]} (0x763228-0x76325d: the material index moves from the
  first to the last slot). (numVertices == 0 skips straight to the triangles.)
- Failure after s exists (0x7630ed): `privateFlags (byte w+3) & 1` ? `_rpWorldSectorDeinstanceAll(s)` : `_rpWorldSectorDestroyRecurse(s)` (the latter frees
  cursor memory with Engine+0x138 - original behaviour), return 0. Out-of-memory sets `{2, _rwerror(0x80000013)}` first.
- `_rwPluginRegistryInitObject 0x8086e0(0x8d736c, s)`; [0xc94c1c] = 0, [0xc94c10] = 0 (right-to-render id/data for sectors); `_rwPluginRegistryReadDataChunks
  0x808980(0x8d736c, stream, s)`; 0 -> cleanup. If [0xc94c1c] != 0: `_rwPluginRegistryInvokeRights 0x808ab0(0x8d736c, [0xc94c1c], s, [0xc94c10])`. Returns s.

## 0x763310 _rpWorldPlaneSectorStreamRead  (catalogue: specified/REA, port level 3)
- `RpPlaneSector* (RwStream* stream, RwUInt8** cursor, RpWorld* w, RwUInt32 F)`, recursive. `RwStreamFindChunk(stream, 1, &len, &version)` (version stored
  over the stream argument slot); 0 -> 0; version outside [0x34000, 0x36003] -> `{2, 0x80000004}`, 0. Read `len` bytes into a zeroed 0x18-byte stack struct
  S (NOT clamped, 0x76336a); result != len -> 0.
- `p = alloc(0x18)` from the cursor (NULL test after the bump) -> `{2, _rwerror(0x80000013, 0x18)}`, 0. `p+0 type = S+0`, `p+4 value = S+4`; if
  `F & 0x40000000` (new format) `leftValue (+0x10) = S+0x10`, `rightValue (+0x14) = S+0x14`; else both = S+4.
- Left (S+8 leftIsWorldSector): non-zero -> `FindChunk(stream, 9, NULL, &version)`, version check, `p+8 = 0x762f30(stream, cursor, w, F)`; zero ->
  `FindChunk(stream, 0xa, ...)` (failure here returns 0 at once, 0x76345b), version check, `p+8 = self(...)`. Right (S+0xc) the same into `p+0xc`.
  Version errors set `{2, 0x80000004}`.
- Any failure after p exists: if `p+8` (left) is set, `privateFlags(w) & 1 ? _rpWorldSectorDeinstanceAll : _rpWorldSectorDestroyRecurse` on it and
  `p+8 = 0`; return 0 (the right subtree and p itself are not released). Returns p.

## 0x762960 RpWorldStreamRead  (catalogue: specified/REA, port level 4)
- `RpWorld* (RwStream* stream)`. Called after the caller found the WORLD chunk. Errors are `RwErrorSet({2, _rwerror(code[, arg])})`.
- `RwStreamFindChunk(stream, 1, &len, &version)`; 0 -> NULL. Version outside [0x34000, 0x36003] -> 0x80000004, NULL.
- World struct S (0x40, zeroed; layout as written by RpWorldStreamWrite): version < 0x34001: read len bytes into a zeroed 0x34-byte buffer B (unclamped)
  and convert exactly as 0x763690 (S+0..0xc = B+0..0xc, S+0x10..0x24 = B+0x1c..0x30, no bounding box); else read len bytes into S (unclamped). Result != len ->
  NULL. `S+0x20 colSectorSize > 0` (signed) -> 0x80000004, NULL.
- numTexCoordSets T (0x762aba-0x762ae0): `format & 0xff0000` ? byte (format >> 16) : (`format & 0x80` ? 2 : (format >> 2) & 1).
- size = `[0x8d62d0] (world size) + [0x8d736c] (sector size) * numWorldSectors (S+0x1c) + 24 * numPlaneSectors (S+0x18)`; unless `format & 0x1000000`:
  `+ 12V + (format & 0x10 ? 4V : 0) + (format & 8 ? 4V : 0) + 8*T*V (when T > 0) + 8*numTriangles`, V = S+0x14 numVertices.
- `w = Engine+0x134(size, 0x3000b)`; NULL -> `0x80000013` with size, NULL. Zero all size bytes. `cursor = w + [0x8d62d0]`. Header: type 7, subType 0, flags 0,
  parent 0, privateFlags = 1 (single allocation). `_rpWorldRegisterWorld(w, size)`. `renderOrder (+0xc) = 2`; `flags (+8) = format`; `worldOrigin (+0x44..+0x4c) =
  S+4..+0xc * [0x858c1c] = -1.0f` (x87, one component at a time); `+0x20 = T`; empty lists +0x2c clumps, +0x34 lights, +0x3c directional lights; `+0x1c = 0`,
  `+0x6c = 0`.
- `FindChunk(stream, 8, NULL, &version)` (material list); 0 -> RpWorldDestroy(w), NULL; version check (error 0x80000004 + destroy);
  `_rpMaterialListStreamRead 0x74e600(stream, w+0x10)`; 0 -> destroy, NULL.
- Root: `S+0 != 0` -> `FindChunk(9)`, version check, `w+0x1c = _rpWorldAtomicSectorStreamRead(stream, &cursor, w, format)`; else `FindChunk(0xa)`, version
  check, `w+0x1c = _rpWorldPlaneSectorStreamRead(...)`. NULL -> destroy, NULL.
- `w+0x24 = 0`, `w+0x28 = &w->clumpList (w+0x2c)`. If the struct-chunk version < 0x34003: `_rpWorldFindBBox(w, w+0x50)`; else copy S+0x28 (6 dwords) to w+0x50.
  `_rpWorldSetupSectorBoundingBoxes(w)`; `RpWorldSetSectorRenderCallBack(w, NULL)` (default 0x74eec0). For each material i < numMaterials (+0x14): if
  `material->texture (+0)` is set call `RpMaterialSetTexture 0x74dbc0(material, texture)` (re-sets the same texture).
- `_rwPluginRegistryInitObject(0x8d62d0, w)`; [0xc94c18] = [0xc94c14] = 0; `_rwPluginRegistryReadDataChunks(0x8d62d0, stream, w)`; 0 ->
  `_rpWorldUnregisterWorld(w)` and a plain free of w (materials, sectors and plugins are NOT released), NULL. If [0xc94c18]: `_rwPluginRegistryInvokeRights(0x8d62d0,
  [0xc94c18], w, [0xc94c14])`. `RpWorldUnlock 0x74f210(w)` (builds the meshes); 0 -> RpWorldDestroy, NULL. Returns w.
- librw: world.cpp is a stub in librw (no BSP streaming): absent.

