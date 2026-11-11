# RwRaster (RW core, RASTERMODULE) - RE-SPEC-RW writer C

Device-independent raster API of RW 3.6 in gta_sa.exe 1.0 US. All `__cdecl`. Every device operation goes through
`RwGlobals.stdFunc[i]` at `RwEngineInstance [0xc97b24] + 0x48 + 4*i`, signature `RwBool (*)(void* out, void* inOut,
RwInt32 n)`. Indices used here (RW 3.6 `RwStandardFunc` order, confirmed by the D3D9 table filled in
`_rwD3D9SetStandardFunctions` 0x7f72e0): 4 RASTERCREATE (+0x58), 11 SETRASTERCONTEXT (+0x74), 15 RASTERLOCK (+0x84),
16 RASTERUNLOCK (+0x88), 19 RASTERRENDERFAST (+0x94), 20 RASTERSHOWRASTER (+0x98). The D3D9 implementations are in
RwD3D9Raster.md.

## Layouts

`RwRaster` (base 0x34, registry 0x8e2518; RW 3.6 header layout, offsets used here): +0x00 parent (0x7fb28a, set to
self), +0x04 cpPixels (0x7fb28c), +0x08 palette (0x7fb28f), +0x0c width (0x7fb264), +0x10 height (0x7fb26e), +0x14 depth
(0x7fb275), +0x18 stride, +0x1c i16 nOffsetX (0x7fb282), +0x1e i16 nOffsetY (0x7fb286), +0x20 u8 cType, +0x21 u8 cFlags
(0x7fb27f), +0x22 u8 privateFlags (0x7fb26b), +0x23 u8 cFormat, +0x24 originalPixels, +0x28 originalWidth,
+0x2c originalHeight, +0x30 originalStride. The D3D9 extension (plugin 0x40c, 0x24 bytes) follows at offset
[0xb4e9e0] (derived 0x34).

RASTERMODULE globals: base = `RwEngineInstance + [0xc980d8]` (derived offset 0x3bc, size 0x64):

| off | meaning | evidence |
|---|---|---|
| +0x00..+0x24 | context stack `RwRaster* stack[10]` | 0x7fb09f, 0x7fb426 (stack[0] = &dummy) |
| +0x28 | i32 stack top index | 0x7fb084, 0x7fb413 |
| +0x2c..+0x5f | dummy RwRaster (0x34 bytes, zeroed; width/height/depth 0, cFlags 0x80 = rwRASTERDONTALLOCATE) | 0x7fb384-0x7fb404 |
| +0x60 | RwFreeList* raster freelist | 0x7fb241, 0x7fb453 |

0xc980dc: module open count (0x7fb476). 0xc980b4: static freelist struct (0x7fb3a2).

## 0x7f5f60 _rwDeviceRegisterPlugin  (catalogue: specified/REA, port level 4)
- 5-byte `jmp 0x4c9ab0` (`_rwD3D9RasterPluginAttach`, RwD3D9Raster.md): tail call, same arguments and return.

## 0x7faec0 RwRasterUnlock  (catalogue: specified/REA, port level 0)
- `RwRaster* (RwRaster* r)`: `stdFunc[16](NULL, r, 0)` (0x7faecf); the result is ignored; returns r.

## 0x7faf50 RwRasterRenderFast  (catalogue: specified/REA, port level 0)
- `RwRaster* (RwRaster* r, RwInt32 x, RwInt32 y)`: builds a local RwRect with only `x` (+0) and `y` (+4) written (w/h
  left uninitialised, 0x7faf64/0x7faf71) and calls `stdFunc[19](r, &rect, 0)` (0x7faf75). Result ignored; returns r.

## 0x7fb060 RwRasterPushContext  (catalogue: specified/REA, port level 0)
- `RwRaster* (RwRaster* r)`: `if (!stdFunc[11](NULL, r, 0)) return NULL;` then `top = ++module.top`
  (0x7fb089), `module.stack[top] = r` (0x7fb09f); return r. No overflow check against the 10 entries.

## 0x7fb0b0 RwRasterRegisterPlugin  (catalogue: specified/REA, port level 2)
- `RwInt32 (size, pluginID, ctor, dtor, copy)` -> `_rwPluginRegistryAddPlugin(0x8e2518, ...)` 0x8084a0.

## 0x7fb110 RwRasterPopContext  (catalogue: specified/REA, port level 0)
- `RwRaster* ()`: `old = module.top; module.top = old - 1` (0x7fb127); `popped = stack[old]`;
  `ok = stdFunc[11](NULL, stack[old - 1], 0)` (0x7fb14c); returns `ok ? popped : NULL`. The top is decremented even
  when the device call fails. No underflow check.

## 0x7fb1a0 RwRasterShowRaster  (catalogue: specified/REA, port level 1)
- `RwRaster* (RwRaster* r, void* dev, RwUInt32 flags)`: loads `fn = stdFunc[20]` first, then
  `_rwResourcesPurge()` 0x807e50, then `fn(r, dev, flags)` (0x7fb1c1); returns `ok ? r : NULL`.

## 0x7fb230 RwRasterCreate  (catalogue: specified/REA, port level 1)
- `RwRaster* (RwInt32 width, RwInt32 height, RwInt32 depth, RwInt32 flags)`.
- Behaviour: `r = memoryAlloc(module+0x60, 0x30407)` (RwGlobals+0x144, 0x7fb246); NULL -> return NULL.
  Then (0x7fb264-0x7fb28f): width, privateFlags = 0, height, depth, cFlags = 0, nOffsetX = nOffsetY = 0,
  parent = r, cpPixels = NULL, palette = NULL. (cType, cFormat, stride and the original* fields are NOT written here;
  the driver sets them.) `stdFunc[4](NULL, r, flags)` (0x7fb292). Failure -> `memoryFree(module+0x60, r)`
  (0x7fb2ac), return NULL (no plugin init/deinit). Success -> `_rwPluginRegistryInitObject(0x8e2518, r)`, return r.
- librw: `Raster::create` additionally stores type/format from `flags` in the core; in the exe the D3D9 create
  (0x4cce60) does it.

## 0x7fb2d0 RwRasterLock  (catalogue: specified/REA, port level 0)
- `RwUInt8* (RwRaster* r, RwUInt8 level, RwInt32 lockMode)`: `stdFunc[15](&pixels, r, ((level & 0xff) << 8) + lockMode)`
  (0x7fb2f2), where `pixels` is the stack slot of the `level` argument (reused as the out value, 0x7fb2e6). Returns
  `ok ? pixels : NULL`.

## 0x7fb370 cb_RASTERMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- Open callback `(instance, offset, size)`; returns instance or NULL.
- Behaviour: `[0xc980d8] = offset`; zero 13 dwords at module+0x2c (dummy raster, `rep stosd`); then dummy fields:
  +0x38 width = 0, +0x3c height = 0, +0x40 depth = 0, +0x4d cFlags = 0x80, +0x30 cpPixels = 0, +0x34 palette = 0,
  +0x4c cType = 0; module.top (+0x28) = 0; stack[0] = &dummy (module+0x2c, 0x7fb426).
  Freelist: `RwFreeListCreateAndPreallocateSpace([0x8e2518] (registry size), [0x8e2530] (0x80), 4, [0x8e2534] (1),
  0xc980b4, 0x40407)` -> module+0x60; NULL -> return NULL; else `[0xc980dc]++`, return instance.
