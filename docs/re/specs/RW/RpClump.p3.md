# RenderWare 3.6 RpClump / RpAtomic module and small texture/atomic plugins - pass-3 specs, writer A (RE-SPEC-RW-3)

Statically linked RW 3.6 world-plugin code (clump.c) plus the anisotropy and right-to-render plugin callbacks that
the catalogue files here. gta-reversed has only `plugin::Call*` thunks. All `__cdecl`. Extends
docs/re/specs/RW/RpClump.md (pass 1/2, read-only); its layout table applies.

## Layouts used (offsets proven by the instruction addresses given)

- `RpClump`: +0x4 frame (0x749665, 0x749b16), +0x8 atomic list head (links = atomic+0x40, 0x749533..0x749548), +0x10
  light list (0x7495c9), +0x18 camera list (0x749617), +0x28 callback (0x74a24c).
- `RpAtomic`: +0x2 u8 flags (0x749aec), +0x3 u8 privateFlags (0x749a79), +0x4 frame (0x7496d1), +0x14 repEntry (0x7496b1),
  +0x18 geometry (0x7496c4), +0x1c boundingSphere (0x749aae), +0x3c clump (0x749b13), +0x40 inClumpLink (next +0x40, prev
  +0x44; 0x749b01..0x749b11), +0x6c pipeline (0x74916c).
- Clump plugin extension on lights `[0xc924dc]` and cameras `[0xc924d8]` (i32 offsets): ext +0 clump, +4 link next, +8
  link prev (0x74a1cd..0x74a1e6); object = `link - offset - 4` (0x749c25/0x749c27).
- CLUMPMODULE engine data `G + [0xc924ac]`: +0 atomic freelist, +4 clump freelist (0x749492, 0x7494ae); open counter
  `[0xc924b0]` (0x749508). Registries: atomic 0x8d624c (0x7496a7), clump 0x8d6264 (0x749529).
- Anisotropy texture extension: u8 at `texture + [0xc92478]` (0x74904a). `RxPipeline` +0x2c pluginId, +0x30 pluginData
  (0x74916f/0x749188).
- Stream-write context (built by RpClumpStreamWrite / RpAtomicStreamWrite): +0x0 RwStream*, +0x4 rwFrameList {+0x4
  frames, +0x8 numFrames}, +0xc RpGeometry** geoms, +0x10 numGeoms, +0x14 success flag (0x74a880..0x74a9f6, 0x74a85f..0x74a873).
- G = `[0xc97b24]`: +0x134 rwmalloc, +0x138 rwfree, +0x148 freeListFree.

## 0x749040 cb_ANISOTPLUGIN_RwTexture_Copy  (catalogue: specified/REA, port level 0)
- `*(u8*)(dst + [0xc92478]) = *(u8*)(src + [0xc92478])` (0x74904a/0x749051); EAX = dst.
- librw: absent (librw has no anisotropy plugin).

## 0x749060 cb_ANISOTPLUGIN_RwTexture_StreamRead  (catalogue: specified/REA, port level 0)
- `(s, len, tex, off, size)`: if `len != 4` or `tex == NULL` -> NULL (0x749065..0x749071; a wrong length fails the whole
  texture read). `RwStreamReadInt32(s, &v, 4)` fail -> NULL; `ext = (u8)v`; return s.
- librw: absent.

## 0x7490b0 cb_ANISOTPLUGIN_RwTexture_StreamWrite  (catalogue: specified/REA, port level 0)
- If `tex == NULL` or `tex->raster (+0) == NULL` -> NULL; else `RwStreamWriteInt32(s, &(u32)ext, 4)`; return ok ? s : NULL.
- librw: absent.

## 0x7490f0 cb_ANISOTPLUGIN_RwTexture_StreamGetSize  (catalogue: specified/REA, port level 0)
- `tex && tex->raster && ext != 1` -> 4, else 0 (0x7490f4..0x749109).
- librw: absent.

## 0x749160 cb_RIGHTTORENDER_RpAtomic_StreamWrite  (catalogue: specified/REA, port level 0)
- `RwStreamWriteInt32(s, &atomic->pipeline->pluginId, 4)` then `(&pipeline->pluginData, 4)` (0x749174/0x74918d; no NULL
  test of the pipeline here); return ok ? s : NULL.
- librw: world.cpp/pipeline right-to-render writer matches in content.

## 0x7491a0 cb_RIGHTTORENDER_RpAtomic_StreamGetSize  (catalogue: specified/REA, port level 0)
- `pipeline && pipeline->pluginId` -> 8, else 0 (0x7491a4..0x7491b2).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x749480 cb_CLUMPMODULE_RwEngine_Close  (catalogue: specified/REA, port level 0)
- `RwFreeListForAllUsed(clumpFL, _rpClumpDestructor 0x749520, NULL)` (0x749497), `RwFreeListForAllUsed(atomicFL,
  _rpAtomicDestructor 0x7496a0, NULL)` (0x7494b2), `RwFreeListDestroy(atomicFL)`, `RwFreeListDestroy(clumpFL)`, both slots
  = 0 (0x7494ee/0x749500), `[0xc924b0] -= 1`; return instance.
- librw: absent (no freelists).

## 0x749520 _rpClumpDestructor  (catalogue: specified/REA, port level 6)
- `void (RpClump* c, void* data)`: `_rwPluginRegistryDeInitObject(0x8d6264, c)` (0x74952e). For every atomic link (next
  read first, 0x749546), inline atomic destruction (identical to 0x7496a0, atomics are NOT unlinked): deinit plugins
  (0x8d624c), free repEntry, `RpGeometryDestroy(geometry)` + `geometry = NULL` + `RwFrameUpdateObjects(frame)` if the atomic
  has a frame and `RpAtomicGetWorld(a)`, `_rwObjectHasFrameReleaseFrame(a)`, `freeListFree(atomicFL, a)`
  (0x74954b..0x7495af). Lights (0x7495c9..0x749611): unlink the ext link, clear ext (3 dwords), `RpLightDestroy(l)`; the walk
  stops after a NULL object. Cameras likewise with `RwCameraDestroy` (0x749621..0x74965f). If `c->frame`:
  `RwFrameDestroyHierarchy(frame)` (0x74966d). `freeListFree(clumpFL, c)` (0x749686).
- librw: clump.cpp:Clump::destroy differs (librw destroys atomics through Atomic::destroy and does not touch the world).

## 0x7496a0 _rpAtomicDestructor  (catalogue: specified/REA, port level 6)
- `void (RpAtomic* a, void* data)`: `_rwPluginRegistryDeInitObject(0x8d624c, a)`; if `repEntry` ->
  `RwResourcesFreeResEntry` (0x7496bc); if `geometry`: `RpGeometryDestroy(geometry)`, `geometry = NULL`; if `a->frame &&
  RpAtomicGetWorld(a)` -> `RwFrameUpdateObjects(frame)` (0x7496e2..0x7496f0); `_rwObjectHasFrameReleaseFrame(a)`;
  `freeListFree(atomicFL, a)` (0x74970e).
- librw: clump.cpp:Atomic::destroy differs in the world/frame update.

## 0x749910 RpClumpGetNumLights  (catalogue: specified/REA, port level 0)
- Counts list links from `c+0x10`: per link `n++`, then stop if the light pointer (`link - [0xc924dc] - 4`) is 0
  (0x749928..0x749936); returns n.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x749940 RpClumpGetNumCameras  (catalogue: specified/REA, port level 0)
- Same over `c+0x18` with `[0xc924d8]` (0x749958..0x749966).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x749970 RpClumpCreateSpace  (catalogue: specified/REA, port level 9)
- `RpClump* (const RwV3d* pos, f32 radius)`: `g = RpGeometryCreateSpace(radius)` NULL -> NULL; `f = RwFrameCreate()` NULL
  -> destroy g, NULL; `f->modelling.pos = *pos` (frame+0x40, no dirty marking, 0x7499ab..0x7499bb); `a =
  RpAtomicCreate()` NULL -> destroy f, g, NULL; `c = RpClumpCreate()` NULL -> inline atomic destruction (as 0x7496a0),
  `RwFrameDestroy(f)`, `RpGeometryDestroy(g)`, NULL (0x7499ee..0x749a63).
- Success: `_rwObjectHasFrameSetFrame(a, f)` (0x749a74); `a->privateFlags |= 1`; inline RpAtomicSetGeometry(a, g):
  if `g != a->geometry`: AddRef g, destroy old, `a->geometry = g`, `a->boundingSphere = g->morphTarget->boundingSphere`
  (0x749aa8..0x749ac4), frame update if in a world. `a->flags &= ~4` (0x749af0); `RpGeometryDestroy(g)` (drops the
  creation reference). Inline RpClumpAddAtomic: link `a` at the head of `c->atomics`, `a->clump = c` (0x749afb..0x749b13);
  `c->frame = f` (0x749b16, direct store); return c.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x749bb0 RpClumpForAllCameras  (catalogue: specified/REA, port level 0)
- `RpClump* (c, cb, data)`: for each link (next read first): `cb(link - [0xc924d8] - 4, data)`; NULL result stops
  (0x749bca..0x749be8); returns c.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x749c00 RpClumpForAllLights  (catalogue: specified/REA, port level 0)
- As 0x749bb0 over the light list with `[0xc924dc]` (0x749c1a..0x749c38).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a1c0 _rpClumpRemoveAndDestroyLight  (catalogue: specified/REA, port level 0)
- Unlink `light + [0xc924dc]` ext link, clear ext (+8, +4, +0), `RpLightDestroy(light)` (0x74a1e8); returns light.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a200 _rpClumpRemoveAndDestroyCamera  (catalogue: specified/REA, port level 0)
- Same with `[0xc924d8]` and `RwCameraDestroy` (0x74a228); returns camera.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a240 RpClumpSetCallBack  (catalogue: specified/REA, port level 0)
- `c->callback = cb ? cb : 0x74a260` (_rpClumpDefaultCallBack) (0x74a24c/0x74a250); EAX = c.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a270 RpClumpGetCallBack  (catalogue: specified/REA, port level 0)
- Returns `c->callback`, or NULL when it equals 0x74a260 (0x74a279..0x74a282).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a4f0 RpClumpAddLight  (catalogue: specified/REA, port level 0)
- Insert `light + [0xc924dc] + 4` at the head of `c->lights` (+0x10), `ext->clump = c` (0x74a4fc..0x74a519); returns c.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a520 RpClumpRemoveLight  (catalogue: specified/REA, port level 0)
- Unlink and clear ext (3 dwords) (0x74a52c..0x74a545); returns c.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a550 RpClumpAddCamera  (catalogue: specified/REA, port level 0)
- As 0x74a4f0 with `[0xc924d8]` and list +0x18 (0x74a55c..0x74a579).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a580 RpClumpRemoveCamera  (catalogue: specified/REA, port level 0)
- As 0x74a520 with `[0xc924d8]` (0x74a58c..0x74a5a5).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a5b0 RpAtomicStreamGetSize  (catalogue: specified/REA, port level 0)
- `RpGeometryStreamGetSize(a->geometry) + 0x28 + _rwPluginRegistryGetSize(0x8d624c, a) + 0xc` (0x74a5ba..0x74a5d2).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a5e0 RpClumpStreamGetSize  (catalogue: specified/REA, port level 6)
- `_rwFrameListInitialize(&fl, c->frame)` fail -> 0; `fs = _rwFrameListStreamGetSize(&fl) + 0x24`; deinit (0x74a5f3..0x74a61a).
  `0x74a770(&gl, c)` fail -> 0. `s = 0x10 + sum(RpGeometryStreamGetSize(g) + 0xc)` (0x74a645..0x74a667); `size = s + fs +
  0xc`; destroy every listed geometry (drops the refs) and `rwfree(gl.geoms)` (0x74a66d..0x74a6a4). Per atomic `size +=
  0x28 + pluginSize(a) + 0xc` (stops after a NULL atomic, 0x74a6b6..0x74a6d8); per light `+= 0x10 +
  RpLightStreamGetSize(l) + 0xc`; per camera `+= 0x10 + RwCameraStreamGetSize(cam) + 0xc`; return `size +
  _rwPluginRegistryGetSize(0x8d6264, c) + 0xc` (0x74a752..0x74a75a).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a770 sub_74a770 (proposed: _rpClumpGeometryListInitialize)  (catalogue: unknown/REA, port level 1)
- `geomList* (geomList* gl, RpClump* c)`: count atomic links (count includes the first NULL atomic and stops there,
  0x74a786..0x74a792); `count > 0`: `gl->geoms = rwmalloc(count*4, 0x3000f)` (0x74a7aa); NULL -> `RwErrorSet({2,
  _rwerror(0x80000013 /*E_RW_NOMEM*/, count*4)})`, return NULL (0x74a7bd..0x74a7d9). Else `geoms = NULL`. `gl->num = 0`;
  for each atomic: `g = a->geometry`; if not already in `geoms[0..num)`: `_rpGeometryAddRef(g)`, append (0x74a809..0x74a83d).
  Returns gl.
- librw: clump.cpp writes geometries via its own list (equivalent; no AddRef).

## 0x74a850 RpAtomicStreamWrite  (catalogue: specified/REA, port level 9)
- `RpAtomic* (RpAtomic* a, RwStream* s)`: context `{stream = s, numFrames = 0, numGeoms = 0, success = 1}` (frames /
  geoms pointers uninitialised) -> `_rpClumpAtomicStreamWrite(a, &ctx)` (0x74a877); returns its result.
- Edge: with no frame list and no geometry list the struct's frameIndex/geomIndex are written UNINITIALISED (see 0x74a880).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74a880 _rpClumpAtomicStreamWrite  (catalogue: specified/REA, port level 9)
- `RpAtomic* (RpAtomic* a, ctx*)`: chunk size = `numGeoms == 0 ? RpGeometryStreamGetSize(g) + 0x28 + plug + 0xc : plug +
  0x28` (plug = `_rwPluginRegistryGetSize(0x8d624c, a)`); `_rwStreamWriteVersionedChunkHeader(s, 0x14 ATOMIC, size, 0x36003,
  0xffff)` fail -> `success = 0`, NULL (0x74a8c3/0x74a8f8). `Header(s, 1, 0x10)` fail -> NULL (success untouched, 0x74a92a).
  Struct `{frameIndex, geomIndex, flags = (u8)a->flags, 0}` (0x74a934..0x74a942): if `ctx->numFrames != 0`:
  `_rwFrameListFindFrame(&ctx->frameList, a->frame, &frameIndex)` fail -> success 0, NULL. If `numGeoms != 0`: linear search
  for `a->geometry`; not found -> success 0, NULL. `RwStreamWrite(s, &struct, 0x10)`; if `numGeoms == 0`
  `RpGeometryStreamWrite(g, s)`; `_rwPluginRegistryWriteDataChunks(0x8d624c, s, a)`; any failure -> success 0, NULL;
  return a.
- librw: clump.cpp:writeAtomic matches in layout (librw always fills indices).

## 0x74aa10 RpClumpStreamWrite  (catalogue: specified/REA, port level 6)
- `RpClump* (RpClump* c, RwStream* s)`; frame 0x38 + 4 pushes. Blocks:
  - 0x74aa10..0x74aafe size: `0x18 + (frameListSize + 0xc)` from a temporary frame list (`_rwFrameListInitialize(c->frame)`
    fail -> NULL at 0x74ab02), geometry-list size `0x10 + sum(geomSize + 0xc)` + 0xc (via 0x74a770 and
    `_rpClumpGeometryListDestroy`), then `RpClumpForAllAtomics/Lights/Cameras` with the size CBs 0x74b000/0x74afa0/0x74afd0,
    plus `clumpPluginSize + 0xc`.
  - 0x74ab02..0x74abc8: `Header(s, 0x10 CLUMP, size)`, `Header(s, 1, 0xc)`; `numAtomics` = inline link count (stops after a
    NULL atomic, 0x74ab57..0x74ab63), `numLights` via `RpClumpForAllLights(c, 0x74ae40 count, &n)`, `numCameras` inline
    count (0x74ab9e..0x74abac); `RwStreamWrite(s, {numAtomics, numLights, numCameras}, 0xc)`. Failures -> NULL.
  - 0x74abd2..0x74ac06: `_rwFrameListInitialize(&fl, c->frame)` fail -> NULL; `_rwFrameListStreamWrite(s, &fl)` fail ->
    deinit, NULL.
  - 0x74ac18..0x74acf3: `0x74a770(&gl, c)` (fail -> deinit frames, NULL); `Header(s, 0x1a GEOMETRYLIST, 0x10 +
    sum(geom+0xc))`, `Header(s, 1, 4)`, `RwStreamWrite(&numGeoms, 4)`, `RpGeometryStreamWrite` for each; any failure ->
    0x74add1: destroy listed geometries, free the array, return NULL (the frame list is NOT deinitialised: leak).
  - 0x74acf5..0x74ada6: `ctx.success = 1`; for each atomic `_rpClumpAtomicStreamWrite(a, &ctx)` until NULL; if
    `numLights > 0` and success: `_rpClumpLightStreamWrite` per light until NULL; same for cameras with
    `_rpClumpCameraStreamWrite`.
  - 0x74ada8..0x74ae2a: `_rpClumpGeometryListDestroy(&gl)`, `_rwFrameListDeinitialize(&fl)`; `success == 0` -> NULL; else
    `_rwPluginRegistryWriteDataChunks(0x8d6264, s, c)` ? c : NULL.
- librw: clump.cpp:Clump::streamWrite matches in chunk layout; differs in error handling (exe leaks the frame list on a
  geometry-list failure).

## 0x74ae40 _rpClumpCountObjectsCB  (catalogue: specified/REA, port level 0)
- `(obj, i32* n)`: `(*n)++` (0x74ae46); returns obj.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74ae50 _rpClumpGeometryListDestroy  (catalogue: specified/REA, port level 0)
- For `i < gl->num` (signed): `RpGeometryDestroy(geoms[i])`; if `geoms` -> `rwfree`, `geoms = NULL` (0x74ae75..0x74ae8b);
  returns gl (num not reset).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74aea0 _rpClumpLightStreamWrite  (catalogue: specified/REA, port level 0)
- `RpLight* (RpLight* l, ctx*)`: `_rwFrameListFindFrame(&ctx->frameList, l->frame, &idx)` (0x74aeb7), `Header(s, 1, 4)`,
  `RwStreamWriteInt32(&idx)`, `RpLightStreamWrite(l, s)`; any failure -> `ctx->success = 0`, NULL; else l.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74af20 _rpClumpCameraStreamWrite  (catalogue: specified/REA, port level 0)
- Same as 0x74aea0 with `RwCameraStreamWrite` (0x74af7a).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74afa0 _rpClumpLightStreamGetSizeCB  (catalogue: specified/REA, port level 0)
- `*size += 0x10; *size += RpLightStreamGetSize(l) + 0xc` (0x74afa8..0x74afc3); returns l.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74afd0 _rpClumpCameraStreamGetSizeCB  (catalogue: specified/REA, port level 0)
- Same with `RwCameraStreamGetSize` (0x74afe2).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74b000 _rpClumpAtomicStreamGetSizeCB  (catalogue: specified/REA, port level 0)
- `*size += 0x28; *size += _rwPluginRegistryGetSize(0x8d624c, a) + 0xc` (0x74b00d..0x74b028); returns a.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74b030 RpAtomicStreamRead  (catalogue: specified/REA, port level 10)
- `RpAtomic* (RwStream* s)`: empty frame list and geometry list locals (num fields 0, 0x74b039/0x74b03d) ->
  `_rpClumpAtomicStreamRead(s, &geomList, &frameList)` (0x74b04c); returns its result.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74bcc0 sub_74bcc0 (proposed: _rpClumpAtomicDestroyCB, used by RpClumpStreamRead)  (catalogue: unknown/REA, port level 6)
- Body identical to 0x7496a0 (0x74bcc6..0x74bd2e) but returns the atomic in EAX (0x74bd37). Referenced twice from
  RpClumpStreamRead 0x74b420 as a ForAll callback.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74bd40 _rpClumpChunkInfoRead  (catalogue: specified/REA, port level 0)
- `info* (RwStream* s, info* out, i32* size)`: `RwStreamFindChunk(s, 1, &len, &ver)` fail -> NULL; `RwStreamRead(s, out, 0xc)
  != 0xc` -> NULL; `*size = len + 0xc` (0x74bd87); returns out.
- librw: no direct equivalent identified in librw (not compared line by line).

