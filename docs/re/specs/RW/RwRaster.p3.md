# RwRaster: device-dispatching wrappers, context stack, file read (RE-SPEC-RW-3 writer B)

All `__cdecl`. Pass-1 file `RwRaster.md` (read-only) covers create/lock/push/pop context.

## Layout
- RwRaster (base 0x34, registry `[0x8e2518]`): `+0x0` parent raster (0x7fb21b), `+0xc` width, `+0x10` height (0x7fb1ec, 0x7fb1f2),
  `+0x1c` i16 nOffsetX, `+0x1e` i16 nOffsetY (0x7faea8, 0x7fb1f5-0x7fb208), `+0x20` u8 cType, `+0x21` u8 cFlags (bit 0x80
  rwRASTERDONTALLOCATE, 0x7fb1d6), `+0x22` u8 privateFlags (bit 0x1 gamma corrected 0x8044c2; bits 0x8|0x10 palette lock,
  cleared with `and 0xe7` 0x7fb00b), `+0x23` u8 cFormat (bit 0x80 rwRASTERFORMATMIPMAP, 0x7fb164).
- Raster module data at engine + `[0xc980d8]`: +0x0 context stack `RwRaster*[]` indexed by the depth (0x7fae6f), +0x28 RwInt32
  stack depth (0x7fae6b, 0x7faef1), +0x60 raster freelist (0x7fb049, 0x7fb31b). `[0xc980dc]` module open count (0x7fb344).
- Device standard functions `RwEngineInstance + 0x48 + 4*slot` (signature `RwBool (void* a, void* b, RwInt32 n)`; slots from
  RwD3D9 docs): +0x5c RASTERDESTROY (5), +0x64 RASTERSETIMAGE (7), +0x6c IMAGEFINDRASTERFORMAT (9), +0x78 RASTERSUBRASTER (12),
  +0x7c RASTERCLEARRECT (13), +0x80 RASTERCLEAR (14), +0x8c RASTERRENDER (17), +0x90 RASTERRENDERSCALED (18), +0xa4
  RASTERLOCKPALETTE (23), +0xa8 RASTERUNLOCKPALETTE (24), +0xb8 RASTERGETMIPLEVELS (28).
- RwImage fields used: +0x0 flags (bit 2 rwIMAGEGAMMACORRECTED, 0x8044bd), +0x4 width, +0x8 height (0x80445e, 0x8044fd).

## 0x7fae60 RwRasterGetCurrentContext  (catalogue: specified/REA, port level 0)
- `RwRaster* (void)`: `m = engine + [0xc980d8]`; returns `m->stack[m->depth]` (`[eax + edx*4]`, 0x7fae6b-0x7fae72). No depth check.
- librw: raster.cpp `Raster::getCurrentContext` differs: librw bounds-checks the depth (returns nil out of range); the exe does not.

## 0x7fae80 RwRasterRenderScaled  (catalogue: specified/REA, port level 0)
- `RwRaster* (RwRaster* r, RwRect* rect)`: `stdFunc[18](r, rect, 0)` (+0x90, 0x7fae93), result ignored; returns r.
- librw: absent (no scaled render).

## 0x7faea0 RwRasterGetOffset  (catalogue: specified/REA, port level 0)
- `RwRaster* (RwRaster* r, RwInt16* x, RwInt16* y)`: `*x = r+0x1c`, `*y = r+0x1e` (16-bit). Return value: EAX = r (never
  overwritten after 0x7faea0).
- librw: absent.

## 0x7faee0 RwRasterClear  (catalogue: specified/REA, port level 1)
- `RwBool (RwInt32 pixelValue)`: `ok = (stack depth > 0)` (signed `setg`, 0x7faef7). ok -> `stdFunc[14](NULL, NULL, pixelValue)`
  (+0x80; result ignored), return 1. Else `RwErrorSet({1, _rwerror(0x15)})` (0x7faf19-0x7faf31) and return 0. Error 0x15 =
  no current raster context (RW core error index; name UNCLEAR).
- librw: absent.

## 0x7faf90 RwRasterClearRect  (catalogue: specified/REA, port level 0)
- `RwBool (RwRect* rect, RwInt32 pixelValue)`: `stdFunc[13](NULL, rect, pixelValue)` (+0x7c); returns 1 always. No context check.
- librw: absent.

## 0x7fafb0 RwRasterRender  (catalogue: specified/REA, port level 0)
- `RwRaster* (RwRaster* r, RwInt32 x, RwInt32 y)`: local `RwV2d`-shaped pair `{x, y}` (ints copied, 0x7fafc4/0x7fafd1);
  `stdFunc[17](r, &pair, 0)` (+0x8c). Returns r.
- librw: absent.

## 0x7faff0 RwRasterUnlockPalette  (catalogue: specified/REA, port level 0)
- `RwRaster* (RwRaster* r)`: `stdFunc[24](NULL, r, 0)` (+0xa8); then `r+0x22 &= 0xe7` (0x7fb005-0x7fb00d). Returns r.
- librw: absent.

## 0x7fb020 RwRasterDestroy  (catalogue: specified/REA, port level 1)
- `RwBool (RwRaster* r)`: `_rwPluginRegistryDeInitObject(&[0x8e2518], r)` 0x808740; `stdFunc[5](NULL, r, 0)` (+0x5c, result
  ignored); `memoryFree(+0x148)(module+0x60 freelist, r)` (0x7fb049-0x7fb04e). Returns 1.
- Order: plugins first, then the device destroy, then the free.
- librw: raster.cpp `Raster::destroy` matches (destruct plugins, device destroy, free).

## 0x7fb0e0 RwRasterLockPalette  (catalogue: specified/REA, port level 0)
- `RwUInt8* (RwRaster* r, RwInt32 lockMode)`: `stdFunc[23](&out, r, lockMode)` (+0xa4; `out` is the first argument's stack slot,
  0x7fb0ee). Returns `out` if the call returned non-zero, else NULL (neg/sbb/and 0x7fb101-0x7fb105). No privateFlags update here.
- librw: absent.

## 0x7fb160 RwRasterGetNumLevels  (catalogue: specified/REA, port level 0)
- `RwInt32 (RwRaster* r)`: if `r+0x23` bit 7 is clear (`js`, 0x7fb169) -> 1. Else `stdFunc[28](&out, r, 0)` (+0xb8); non-zero ->
  `out`, else -1 (0x7fb192).
- librw: raster.cpp `Raster::getNumLevels` differs: librw calls the device for every raster (non-mipmapped rasters give 1 through
  the device).

## 0x7fb1d0 RwRasterSubRaster  (catalogue: specified/REA, port level 0)
- `RwRaster* (RwRaster* r, RwRaster* in, RwRect* rect /*x +0, y +4, w +8, h +0xc*/)`: if `r+0x21` bit 7 (DONTALLOCATE) clear ->
  NULL (0x7fb1db). Else `r.width = rect.w`, `r.height = rect.h`, `r.nOffsetX = in.nOffsetX + (i16)rect.x`, `r.nOffsetY =
  in.nOffsetY + (i16)rect.y` (16-bit adds, wrap, 0x7fb1f5-0x7fb208); `stdFunc[12](r, in, 0)` (+0x78); non-zero -> `r.parent =
  in.parent` (0x7fb21b) and return r; else NULL (the fields already written stay).
- librw: raster.cpp `Raster::subRaster` differs: librw sets the fields and parent without calling the device and returns void.

## 0x7fb310 cb_RASTERMODULE_RwEngine_Close  (catalogue: specified/REA, port level 1)
- `void* (void* instance, RwInt32 offset, RwInt32 size)`: if module+0x60 freelist != NULL: `RwFreeListDestroy` 0x801b80, set it
  to NULL (0x7fb337). `[0xc980dc]--` (0x7fb344). Returns instance.
- librw: absent (librw engine close differs structurally).

## 0x8043f0 RwRasterRead  (catalogue: specified/REA, port level 4)
- `RwRaster* (const RwChar* filename)`. `img = RwImageRead(filename)` 0x802fd0; NULL -> NULL.
- `stdFunc[9](&tmpRaster, img, 0)` (+0x6c, 0x80441e; tmpRaster is a local RwRaster-shaped struct). Success: `width =
  tmp+0xc`, `height = tmp+0x10`, `depth = tmp+0x14`, `flags = (tmp.cFormat (+0x23) << 8) | tmp.cType (+0x20)`
  (0x804428-0x80444e). FAILURE (0x804452): width, height and flags are all loaded from the filename argument's stack slot and depth
  stays an uninitialised local (original bug: uninitialised variables; the port must reproduce "garbage" by failing the same way or
  record a divergence).
- If `width != img.width` or `height != img.height` (0x80445e-0x804466): `img2 = RwImageCreateResample(img, width, height)`
  0x80cd10, `RwImageDestroy(img)`, `img = img2` (no NULL check of img2).
- `r = RwRasterCreate(width, height, depth, flags)` 0x7fb230; NULL -> `RwImageDestroy(img)`, NULL.
- `stdFunc[7](r, img, 0)` (+0x64); if it returned non-zero and `img.flags & 2` -> `r+0x22 |= 1` (0x8044c2). `RwImageDestroy(img)`;
  return r (also when set-image failed).
- librw: absent.

## 0x8044e0 RwRasterReadMaskedRaster  (catalogue: specified/REA, port level 5)
- `RwRaster* (const RwChar* name, const RwChar* maskName)`: `img = RwImageReadMaskedImage(name, maskName)` 0x8035c0; NULL -> NULL.
  `r = RwRasterCreate(img.width, img.height, 0, 0)` (0x804509); NULL -> destroy img, NULL. `stdFunc[7](r, img, 0)`; if non-zero
  and `img.flags & 2` -> `r+0x22 |= 1`. Destroy img, return r.
- librw: absent.
