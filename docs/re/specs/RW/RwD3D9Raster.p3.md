# RW D3D9 driver: rasters - pass-3 specs (render/copy helpers, DDS create, remaining small functions), writer A (RE-SPEC-RW-3)

Part of the D3D9 driver linked into the game half of `.text` (0x4c9970-0x4cd820). Extends
docs/re/specs/RW/RwD3D9Raster.md (pass 1/2, read-only). All `__cdecl` unless stated; several helpers here take
register arguments (stated per function). gta-reversed has no code for any of them.

## Layouts used (offsets proven by the instruction addresses given)

`RwRaster` (RW 3.6): +0x0 parent (root raster has `parent == self`, walk 0x4ca5b3..0x4ca5b9), +0xc width
(0x4cabdc), +0x10 height (0x4cabdf), +0x14 depth (0x4cd19b), +0x1c i16 nOffsetX (0x4ca747, `movsx`), +0x1e i16
nOffsetY (0x4ca75c), +0x20 u8 cType (0=NORMAL, 1=ZBUFFER, 2=CAMERA, 4=TEXTURE, 5=CAMERATEXTURE; 0x4ca5cb), +0x21 u8
cFlags (0x80 DONTALLOCATE cleared 0x4cd151), +0x23 u8 cFormat (format >> 8; 0x4cd14e).

D3D9 raster extension `ext = raster + [0xb4e9e0]` (`_RwD3D9RasterExtOffset`, i32): +0x0 `IDirect3DTexture9*`
(or `IDirect3DSurface9*` for a camera raster with a swap chain, 0x4cab4b), +0x8 u8 alpha (0x4ca8db, 0x4cd18c),
+0x9 u8 cube:4 (low) / face:4 (high) (0x4cab0a..0x4cab22), +0xa u8 autoMipmapGen:4 (low) / compressed:4 (high)
(0x4cd0b8, 0x4cd178), +0x18 D3DFORMAT (0x4cd163), +0x1c `IDirect3DSwapChain9*` (0x4cab43).

Globals: `[0xb4e9e4]` current raster context (`RwRaster*`, the render destination; 0x4ca5c6), `[0xc97c28]`
`_RwD3DDevice` (IDirect3DDevice9*), `[0xc97c2c]` `_RwD3D9DepthStencilSurface`, `[0xc97c30]` `_RwD3D9RenderSurface`
(back buffer surface), `[0xc9bff4]` u32 = `_RwD3D9DeviceCaps + 0xf4` (D3DCAPS9.StretchRectFilterCaps; masks 0x2000200
= MAGF|MINF LINEAR, 0x1000100 = MAGF|MINF POINT, 0x4caa68/0x4caa77), `[0xb4e7e0]` D3DFORMAT info table (4 bytes per
format < 0x80: +0 alpha, +1 depth, +2 u16 rwRasterFormat whose high byte +3 is ORed into cFormat; see RwD3D9Raster.md),
`[0xb4e9e8]` u32 count / `[0xb4e9ec]` u32 capacity / `[0xb4e9f4]` u16* array of the palette-index free stack.
Im2D quad vertex buffer: 4 x `RwIm2DVertex` (0x1c bytes: x, y, z, rhw, u32 color, u, v) at .data 0xb4e770
(vertex k at 0xb4e770 + k*0x1c).

IDirect3DDevice9 vtable slots used: +0x78 UpdateSurface, +0x80 GetRenderTargetData, +0x88 StretchRect, +0xbc
SetViewport, +0x5c CreateTexture. IDirect3DTexture9/IDirect3DCubeTexture9 +0x48 GetSurfaceLevel/GetCubeMapSurface,
IUnknown +0x8 Release.

## 0x4ca5a0 sub_4ca5a0 (proposed: _rwD3D9RasterRenderQuad)  (catalogue: unknown/REA, port level 4)
- Convention (register): ESI = source raster, EBX = "scaled" flag (read at 0x4ca652/0x4ca911 without being set
  here), stack `(RwRect* rect, i32 blend)`; caller 0x4cad90 (`_rwD3D9RasterRenderGeneric`) pops. No return value is
  used (the caller returns 1). Frame 0x68 bytes.
- Behaviour:
  1. `root = src`; while `root->parent != root` follow (0x4ca5b3..0x4ca5b9); `rext = root + [0xb4e9e0]` (0x4ca5bb).
     `rtOk = 0` (0x4ca5ab).
  2. `dest = [0xb4e9e4]`. If `dest->cType != 2` (0x4ca5cb): `rtOk = RwD3D9SetRenderTarget(0, dest)` (0x4ca5d4); if
     `rtOk != 0` go to 3. Otherwise (camera dest or failure): `_rwD3D9SetRenderTarget(0, [0xc97c30])` (0x4ca5ed).
  3. `_rwD3D9SetDepthStencilSurface(NULL)` (0x4ca5f7). Viewport `{X = dest->nOffsetX, Y = dest->nOffsetY, Width =
     dest->width, Height = dest->height, MinZ = 0, MaxZ = 1.0f}` (0x4ca601..0x4ca634) -> `device->SetViewport`
     (vtbl+0xbc, 0x4ca63f).
  4. If `_rwD3D9TestState() == 0` (0x4ca645) skip to 8.
  5. Quad vertices (int -> f32 with `fild`): v0 `x = rect->x` (0xb4e770), `y = rect->y` (0xb4e774); v1 `x = rect->x`
     (0xb4e78c); if scaled (0x4ca652): `w = rect->w` (+8), `h = rect->h` (+0xc), else `w = src->width`, `h =
     src->height` (0x4ca6cc..0x4ca717); v1 `y = rect->y + h` (0xb4e790); v2 `x = rect->x + w` (0xb4e7a8), `y = rect->y`
     (0xb4e7ac); v3 `x = rect->x + w` (0xb4e7c4), `y = rect->y + h` (0xb4e7c8). For every vertex `z = 0`, `rhw = 1.0f`,
     `color = 0xffffffff` (0x4ca72b..0x4ca742, 0x4ca76b..0x4ca77a, 0x4ca7b1..0x4ca7c0, 0x4ca7fe..0x4ca80d).
     UVs (x87 `fild` i16 offset then `fidiv` i32 root size, stored f32): v0 `u = src->nOffsetX / root->width`
     (0xb4e784), `v = src->nOffsetY / root->height` (0xb4e788); v1 `u = nOffsetX / root->width` (0xb4e7a0), `v =
     (nOffsetY + src->height) / root->height` (0xb4e7a4); v2 `u = (nOffsetX + src->width) / root->width` (0xb4e7bc),
     `v = nOffsetY / root->height` (0xb4e7c0); v3 `u = (nOffsetX + src->width)/root->width` (0xb4e7d8), `v = (nOffsetY +
     src->height)/root->height` (0xb4e7dc).
  6. Save RW states with `_rwD3D9RWGetRenderState`: 0xa SRCBLEND, 0xb DESTBLEND, 0xc VERTEXALPHAENABLE, 0xe
     FOGENABLE, 9 TEXTUREFILTER, 3 TEXTUREADDRESSU, 4 TEXTUREADDRESSV, 1 TEXTURERASTER (0x4ca7f9..0x4ca8a1, in this
     order); D3D states 7 ZENABLE and 0x16 CULLMODE with `RwD3D9GetRenderState` (0x4ca8b0, 0x4ca8bc).
     `0x7fdcd0(root, 0)` (set root as stage-0 texture, 0x4ca8c4).
  7. If `blend != 0 && rext->alpha != 0` (0x4ca8d7..0x4ca8e0): `_rwD3D9RenderStateVertexAlphaEnable(1)`,
     `SrcBlend(5)`, `DestBlend(6)` (0x4ca8e4..0x4ca8f2); else `VertexAlphaEnable(0)` (0x4ca8fe). Then
     `_rwD3D9RWSetRenderState(0xe, 0)`, `(9, scaled ? 2 : 1)` (0x4ca90f..0x4ca91a), `(2 TEXTUREADDRESS, 3 CLAMP)`,
     `RwD3D9SetRenderState(7, 0)`, `(0x16, 1)` (0x4ca91f..0x4ca935); `_rwD3D9Im2DRenderPrimitive(4 TRISTRIP, 0xb4e770,
     4)` (0x4ca943). Restore 7, 0x16 (D3D), then RW 0xa, 0xb, 0xc, 0xe, 9, `0x7fdcd0(savedRaster, 0)` (0x4ca9a6),
     RW 3, 4 (0x4ca948..0x4ca9be). The saved raster is restored through 0x7fdcd0, not through state 1.
  8. If `dest->cType == 2` return (0x4ca9d2). If `rtOk != 0`: `_rwD3D9SetRenderTarget(0, [0xc97c30])`,
     `_rwD3D9SetDepthStencilSurface([0xc97c2c])` (0x4caac2/0x4caacd), return.
  9. Else (render target could not be set; the quad went to the back buffer) copy it into the destination
     texture: `surf = rext->texture->GetCubeMapSurface(face = ext+9 >> 4, 0, &surf)` if `rext+9 & 0xf` else
     `GetSurfaceLevel(0, &surf)` (both vtbl+0x48, 0x4caa03/0x4caa10) - NOTE: the root of the SOURCE, `edi` = rext
     (0x4ca9c8/0x4ca8d0), whose surface receives the copy. `srcRect = {dest.offX, dest.offY, dest.offX + dest.width,
     dest.offY + dest.height}` (0x4caa13..0x4caa3d), `dstRect = {src.offX, src.offY, src.offX + src.width, src.offY +
     src.height}` (0x4caa41..0x4caa62); filter = scaled ? (`[0xc9bff4] & 0x2000200` ? 2 : 0) : (`& 0x1000100` ? 1 : 0);
     `device->StretchRect([0xc97c30], &srcRect, surf, &dstRect, filter)` (vtbl+0x88, 0x4caaa3); `surf->Release()`
     (0x4caab0). The depth-stencil surface stays NULL on this path and on the camera path of step 8.
- Edge: 9 reads `rext` as the copy target although `dest` was the requested destination; for this to make sense
  `src` must be a sub-raster of `dest` (render-to-self). Kept as in the bytes. UNCLEAR (0x4ca9e8): intended pairing.
- librw: absent (librw has no D3D9 raster render).

## 0x4caae0 sub_4caae0 (proposed: _rwD3D9RasterCopy)  (catalogue: unknown/REA, port level 0)
- Convention (register): EAX = source raster, stack `(RwRect* rect, i32 scaled)`; returns `hr >= 0 ? 1 : 0`
  (0x4cad74..0x4cad79). Caller 0x4cad90 (copy cases). Frame 0x34 bytes.
- Behaviour:
  1. `rootSrc` = parent walk of src (0x4caae7..0x4caafd). Source surface: if `rootSrc->cType == 2` (0x4caaff): `ext->
     swapChain != NULL` ? `ext+0x0` (surface pointer, no AddRef) : `[0xc97c30]` (0x4cab43..0x4cab5a); else
     cube (`ext+9 & 0xf`) ? `tex->GetCubeMapSurface(ext+9 >> 4, 0, &s)` : `tex->GetSurfaceLevel(0, &s)` (vtbl+0x48,
     0x4cab27/0x4cab39).
  2. `dest = [0xb4e9e4]`, `rootDst` = parent walk (0x4cab5f..0x4cab77); destination surface the same way
     (0x4cab79..0x4cabd0).
  3. `srcRect = {src.offX, src.offY, src.offX + src.width, src.offY + src.height}` (0x4cabd4..0x4cabf9, from `src`, not
     the root). `dstRect`: if scaled (0x4cabfd): `{x = dest.offX + rect.x, y = dest.offY + rect.y, x + rect.w, y +
     rect.h}` (0x4cac09..0x4cac2c); else `{x, y, x + src.width, y + src.height}` (0x4cac32..0x4cac57).
     `point = {src.offX, src.offY}` (stack slots 0x1c/0x20 written at 0x4cac03, 0x4cac8c/0x4cac96).
  4. Dispatch on cTypes (0x4cac5b..0x4cac77): dest camera (2 or 5) and src camera (2 or 5) -> filter as in 0x4ca5a0
     step 9 (from `scaled` and `[0xc9bff4]`), `hr = StretchRect(srcSurf, &srcRect, dstSurf, &dstRect, filter)`
     (vtbl+0x88, 0x4cad46); dest camera, src not camera -> `hr = UpdateSurface(srcSurf, &srcRect, dstSurf, &point)`
     (vtbl+0x78, 0x4cacf7); dest not camera, src camera -> `hr = GetRenderTargetData(srcSurf, dstSurf)` (vtbl+0x80,
     0x4cacb7, whole surface, rects ignored); neither -> `hr = UpdateSurface(srcSurf, &srcRect, dstSurf, &point)`
     (0x4cac9d).
  5. Release `dstSurf` if `rootDst->cType != 2` (0x4cad4e..0x4cad5d) and `srcSurf` if `rootSrc->cType != 2`
     (0x4cad60..0x4cad71).
- Edge: for UpdateSurface the destination point is the SOURCE offset, so `rect` is ignored in both UpdateSurface cases
  (bytes 0x4cac79..0x4cac96). Kept.
- librw: absent.

## 0x4cb730 sub_4cb730 (proposed: _rwD3D9PaletteIndexFreePush)  (catalogue: unknown/REA, port level 0)
- Convention (register): SI = u16 palette index to push; no stack args; returns nothing useful. Callers
  0x4cb7c0 (create, failure path) and `_rwD3D9RasterDestroy` 0x4cbb00 (2 sites).
- Entry is a `.HOODLUM` disguised load: 0x4cb731 jumps to 0x40a288 which computes `ecx = [~[0x156091c] ^
  [0x156d562]]` = `[0xb4e9e8]` (0x3e839dc7 inverted = 0xc17c6238, xor 0xc1c88bd0 = 0xb4e9e8) under pushfd/popfd and
  returns to 0x4cb736. A port reads `[0xb4e9e8]` directly.
- If `count < [0xb4e9ec]` (unsigned `jb`, 0x4cb73b): `[0xb4e9f4][count] = si`, `[0xb4e9e8] = count + 1` (0x4cb7ab..0x4cb7b6).
- Else grow: `cap = [0xb4e9ec] + 0x200`, stored first (0x4cb74c); array NULL ? `G->rwmalloc(cap*2, 0x1030411)` (G+0x134,
  0x4cb761) : `G->rwrealloc(array, cap*2, 0x1030411)` (G+0x13c, 0x4cb78b); `[0xb4e9f4] = result` (no NULL check: a
  failed allocation writes through NULL, 0x4cb770); store `si`, increment count.
- librw: absent (librw has no palette index stack).

## 0x4cd050 _rwD3D9RasterCreateFromDDSFormat  (catalogue: unknown/REA, port level 2)
- Convention: `RwRaster* (i32 width, i32 height, D3DFORMAT format, u32 rasterFlags)`; returns the raster or NULL.
  Callers `_rwD3D9NativeTextureRead` 0x4cd820, `RwD3D9RasterStreamReadDDS` 0x81f360 (3 sites). The stack slots of
  `format` and `rasterFlags` are reused as `pool` / `usage` locals.
- Behaviour:
  1. If `(flags & 7)` is 0 or 1 (0x4cd058..0x4cd060): `flags &= 0xffff6fff` (clear MIPMAP 0x8000 and AUTOMIPMAP 0x1000).
  2. `r = RwRasterCreate(width, height, 0, flags | 0x80)` (0x4cd07c); NULL -> return NULL.
  3. `ext = r + [0xb4e9e0]`. If `(flags & 0x9000) == 0x9000` and `0x4cbf80(format) != 0` (autogen-mipmap caps check,
     0x4cd0ac): `ext+0xa = (ext+0xa & 0xf1) | 1` (0x4cd0b8..0x4cd0c1).
     `usage = (ext+0xa & 0xf) ? 0x400 (D3DUSAGE_AUTOGENMIPMAP) : 0` (0x4cd0c4..0x4cd0da).
  4. If `(flags & 5) == 5` (0x4cd0d4): `pool = 0`; `_rwD3D9CheckValidCameraTextureFormat(format)` == 0 -> fail;
     `usage |= 1` (RENDERTARGET) (0x4cd0e4..0x4cd101). Else if `0x46 <= format <= 0x50` (signed, 0x4cd1a6..0x4cd1ae):
     `pool = 0`; `_rwD3D9CheckValidZBufferTextureFormat(format)` == 0 -> fail; `usage |= 2` (DEPTHSTENCIL). Else
     `pool = 1` (MANAGED), `_rwD3D9CheckValidTextureFormat(format)` == 0 -> fail (0x4cd1d1..0x4cd1e4).
  5. `hr = device->CreateTexture(r->width, r->height, levels = (flags & 0x8000) ? 0 : 1, usage, format, pool,
     &ext->texture, NULL)` (vtbl+0x5c, 0x4cd12f; levels = `~(flags >> 15) & 1`, 0x4cd120..0x4cd125); `hr < 0` -> fail.
  6. If `pool == 0`: `_rxD3D9VideoMemoryRasterListAdd(r)` (0x4cd143). `r->cFlags &= 0x7f` (0x4cd151), `r->cFormat &=
     0xf0` (0x4cd154), `ext->format = format` (0x4cd163).
  7. If `'DXT1' 0x31545844 <= format <= 'DXT5' 0x35545844` (signed compares 0x4cd157/0x4cd16c): `ext+0xa = (ext+0xa &
     0xf) | 0x10` (compressed); DXT1 -> `ext->alpha = 0`, `cFormat |= 2` (0x4cd18c..0x4cd197); others -> `alpha = 1`,
     `cFormat |= 3` (0x4cd1fa..0x4cd205); `r->depth = 0x10`; return r.
     Else `ext+0xa &= 0xf` (0x4cd214..0x4cd220); if `format < 0x80` (signed): `ext->alpha = [0xb4e7e0 + format*4]`,
     `r->depth = [0xb4e7e1 + format*4]` (zero-extended), `r->cFormat |= [0xb4e7e3 + format*4]` (0x4cd225..0x4cd246).
     Return r (a FOURCC format other than DXTn leaves alpha/depth/cFormat as set by RwRasterCreate).
  8. fail: `RwRasterDestroy(r)` (0x4cd1eb), return NULL.
- librw: d3d/d3d.cpp:rasterCreate-like paths differ (librw has no DDS create helper, no video-memory list, sets
  formats from its own tables).
## 0x4c9ea0 _rwD3D9RasterHasAlpha  (catalogue: specified/REA, port level 0)
- `RwBool (const RwTexture* t)` (first dword = raster): `raster ? ext(raster)->alpha (+8, zero-extended) : 0` (0x4c9ea4..0x4c9eb0).
- librw: d3d/d3d.cpp:rasterHasAlpha? reads its own flag; matches in meaning.

## 0x4c9f70 sub_4c9f70 (proposed: _rwD3D9RasterIsCubeMap)  (catalogue: unknown/REA, port level 0)
- `u32 (const RwRaster* r)`: `ext(r)->+9 & 0xf` (0x4c9f7a..0x4c9f7f).
- librw: absent.

## 0x4ca4e0 _rwD3D9RasterLockPalette  (catalogue: specified/REA, port level 0)
- Standard fn `RwBool (void** out, RwRaster* r, i32 flags)`: `t = cType & 7`; allowed `t == 0` or `t == 4/5` (0x4ca4e9..0x4ca4f6),
  else return 0. `r->palette (+8) = ext->palette (+4)` (0x4ca50f); `privateFlags (+0x22) |= (flags & 1) ? 0x18 : 8`
  (0x4ca514/0x4ca526); `*out = palette`; return 1.
- librw: absent (librw palettes are plain arrays).

## 0x4ca540 _rwD3D9RasterUnlockPalette  (catalogue: specified/REA, port level 0)
- Same type test; if `privateFlags & 0x10` (palette written): `device->SetPaletteEntries(palette->+0x400 index, palette)`
  (IDirect3DDevice9 vtbl+0x11c, 0x4ca57d; result ignored). `privateFlags &= 0xe7`, `r->palette = NULL`; return 1.
- librw: absent.

## 0x4cae40 _rwD3D9RasterRender  (catalogue: specified/REA, port level 0)
- Standard fn `(raster, rect, flags)`: calls 0x4cad90 with EAX = raster, EDX = rect, ECX = 0, stack blend = 1 (0x4cae40..0x4cae4c);
  returns its EAX. `flags` ignored. (See RwD3D9Raster.md 0x4cad90.)
- librw: absent.

## 0x4cae80 _rwD3D9RasterRenderScaled  (catalogue: specified/REA, port level 0)
- As 0x4cae40 with ECX = 1 (scaled) (0x4cae8a).
- librw: absent.

## 0x4cb4c0 _rwD3D9RasterClearRect  (catalogue: specified/REA, port level 3)
- Standard fn `(void*, RwRect* rect, u32 color)`: `_rwD3D9RasterClearGeneric(color)` with ESI = rect (register arg,
  0x4cb4c5..0x4cb4ca); returns its EAX.
- librw: absent.

## 0x4cb4e0 _rwD3D9RasterClear  (catalogue: specified/REA, port level 3)
- `(void*, void*, u32 color)`: local rect `{0, 0, ctx->width, ctx->height}` from `[0xb4e9e4]` (0x4cb4e3..0x4cb506); ClearGeneric
  with ESI = &rect (0x4cb50a).
- librw: absent.

## 0x4cb5e0 _rxD3D9VideoMemoryRasterListDestroy  (catalogue: specified/REA, port level 0)
- Walk the list `[0xb4e9fc]` (node +4 = next): `freeListFree([0xb4ea00], node)`, `[0xb4e9fc] = next` (0x4cb5f0..0x4cb612); then
  `RwFreeListDestroy([0xb4ea00])` and NULL it if set (0x4cb61f/0x4cb627).
- librw: absent.

## 0x4cbcb0 _rwD3D9RasterGetMipLevels  (catalogue: specified/REA, port level 0)
- `RwBool (i32* out, RwRaster* r)`: root walk (0x4cbcb4..0x4cbcc6); if root `cType` (full byte) is not 0, 4 or 5 -> `*out = 1`.
  Else if `ext->texture`: `*out = texture->GetLevelCount()` (vtbl+0x34, 0x4cbcf9); else `n = max(width, height)` (signed
  `jg`), count `n` right shifts until 0 (unsigned `shr`, 0x4cbd20..0x4cbd25) -> `*out`. Returns 1.
- librw: d3d/d3d.cpp:getNumLevels similar (librw computes from size only).

## 0x4cbd40 _rwD3D9TextureSetRaster  (catalogue: specified/REA, port level 0)
- `(RwTexture* t, RwRaster* r)`: `t->raster = r` (0x4cbd48); returns 1.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x4cbd50 _rwD3D9RasterSubRaster  (catalogue: specified/REA, port level 0)
- `(RwRaster* sub, const RwRaster* parent)`: copy stride (+0x18), depth (+0x14), cType (+0x20), cFormat (+0x23); `sub->cpPixels
  (+4) = NULL` (0x4cbd54..0x4cbd70); returns 1.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x4cbe80 _rwD3D9CheckValidZBufferTextureFormat  (catalogue: specified/REA, port level 0)
- `RwBool (D3DFORMAT f)`: `d3d->CheckDeviceFormat([0xc97c24], [0x8e2428], [0xc9bef0], 2 DEPTHSTENCIL, 3 TEXTURE, f) >= 0`
  (IDirect3D9 vtbl+0x28, 0x4cbea8) and `d3d->CheckDepthStencilMatch(idx, type, [0xc9bef0], [0xc9bef0], f) >= 0` (vtbl+0x30,
  0x4cbec8). (`[0xc97c20]` IDirect3D9*, `[0xc97c24]` adapter, `[0x8e2428]` device type, `[0xc9bef0]` display format.)
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x4cbee0 _rwD3D9CheckValidCubeTextureFormat  (catalogue: specified/REA, port level 0)
- `CheckDeviceFormat(..., usage 0, 5 CUBETEXTURE, f) >= 0` (0x4cbf05).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x4cbf20 _rwD3D9CheckValidCameraCubeTextureFormat  (catalogue: specified/REA, port level 0)
- `CheckDeviceFormat(..., 1 RENDERTARGET, 5, f) >= 0` and `CheckDepthStencilMatch(..., [0xc9bef0], f, [0xc9c068] depth
  format) >= 0` (0x4cbf48/0x4cbf6e).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x4cbf80 sub_4cbf80 (proposed: _rwD3D9CheckAutoMipmapGenTextureFormat, thunk)  (catalogue: unknown/REA, port level 0)
- Entry jumps to a `.HOODLUM` stub 0x4067e3 that does `test [0xc9bf0c], 0x40000000` (D3DCAPS9.Caps2 CANAUTOGENMIPMAP) and returns
  to 0x4cbf8a. Not set -> 0. Else `CheckDeviceFormat(..., 0x400 AUTOGENMIPMAP, 3 TEXTURE, f) == 0` (exactly D3D_OK; the
  positive D3DOK_NOAUTOGEN gives 0) (0x4cbfb4..0x4cbfbb).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x4cbfd0 _rwD3D9CheckAutoMipmapGenTextureFormat  (catalogue: specified/REA, port level 0)
- Same with usage 0x401 (AUTOGENMIPMAP | RENDERTARGET) (0x4cbff1); used for camera textures.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x4cc020 _rwD3D9CheckAutoMipmapGenCubeTextureFormat  (catalogue: specified/REA, port level 0)
- Same with usage 0x400 and resource type 5 (0x4cc03f/0x4cc041).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x4cc0c0 sub_4cc0c0 (proposed: _rwD3D9CheckValidFormatForRasterType)  (catalogue: unknown/REA, port level 1)
- Convention (register): EAX = cube flag, ECX = raster type, EDX = D3DFORMAT (pushed). `type & 7`: 0 or 4 -> `CheckDeviceFormat(...,
  0, cube ? 5 : 3, f) >= 0` (0x4cc0ed..0x4cc13a); 5 -> `cube ? CheckValidCameraCubeTextureFormat(f) : CheckValidCameraTextureFormat(f)`
  (0x4cc0d4..0x4cc0e3); other types -> 0.
- librw: absent.

## 0x4cc170 _rwD3D9RasterClose  (catalogue: specified/REA, port level 2)
- `_rxD3D9VideoMemoryRasterListDestroy()`; `[0xb4e9e8] <- 0`, `[0xb4e9ec] <- 0`, `[0xb4e9f0] <- 0`; `rwfree([0xb4e9f4])` + NULL if set;
  `RwFreeListDestroy([0xb4e9f8])` + NULL if set (0x4cc171..0x4cc1ba).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x4cc1d0 sub_4cc1d0 (proposed: _rwD3D9ImageFindFormat)  (catalogue: unknown/REA, port level 1)
- `u32 (i32 type, u32 flags)`; returns `f` = `flags & 0xff00` with a pixel format chosen. Only caller 0x4cc580. Switch on `type`
  (`> 5` -> f; table 0x4cc55c): 0/4 texture, 1 zbuffer, 2 camera, 3 -> f, 5 camera texture.
  - Texture, no format (`f & 0xf00 == 0`): PAL8 (0x2000): `CheckValidTextureFormat(0x29 P8)` ? `f |= ([0xc9bf3c] & 0x80) ? 0x500 :
    0x600` : `f &= ~0x2000`; still PAL8 -> return f. Otherwise `A8R8G8B8 (0x15)` valid -> `f | 0x500`; else `A4R4G4B4 (0x1a)`
    valid -> `f | 0x300`; else `f | 0x100` (0x4cc1ef..0x4cc250, 0x4cc320..0x4cc386).
  - Texture with format: PAL4 (0x4000) is turned into PAL8 (`f = f & ~0x4000 | 0x2000`, 0x4cc25c). PAL8: P8 valid and (format !=
    0x500 or ALPHAPALETTE cap) -> f; else drop PAL8 and: format 0x600 -> keep if `[0xc9bef4] > 16` and X8R8G8B8 (0x16) valid, else
    becomes 0x200; other formats -> 0x500 if depth > 16 and 0x15 valid, else the 4444/1555 fallback (0x4cc2a1..0x4cc31a).
    Not palettised: 0x500 needs 0x15 else 4444/1555 fallback; 0x600 needs 0x16 else 0x200; 0x300 needs 0x1a else 0x100;
    0x400 needs L8 (0x32) else `f &= ~0x400` and `0x16` valid -> `f |= 0x16` (BUG kept: ORs the D3DFORMAT value 0x16 instead of
    0x600, 0x4cc3bf) else `f |= 0x200`; other formats unchanged (0x4cc2fa..0x4cc3c5).
  - Camera texture (0x4cc3c6): no format: `[0xc9befc] == 16` -> `f | 0x200`; else `CheckValidCameraTextureFormat(0x16)` ? `f |
    0x600` : `f | 0x200`. With format: `d = depth table [0x85c674 + nibble*8]`; ZBufferDepth 16: 32-bit formats are mapped
    (0x500 -> 0x300, 0x600 -> 0x200, other -> f = 0x200 replacing all flags), then 0x300 falls back to 0x100 if 0x1a is not a valid
    camera format, then 0x100 falls back to 0x200 if 0x19 is not (0x4cc409..0x4cc4a6). ZBufferDepth != 16: 16-bit formats map to
    0x500 (from 0x300/0x100) or 0x600 (from 0x200), other -> f = 0x600; then 0x500 falls back to 0x600 if 0x15 is not valid
    (0x4cc4aa..0x4cc514).
  - Camera (2): with format -> f; else `f = u16 [0xb4e7e2 + 4*[0xc9c048]]` (rwFormat of the back buffer format, 0x4cc520..0x4cc525).
  - ZBuffer (1): with format -> f; ZBufferDepth 16 -> 0x700, 32 -> 0x900, else f (0x4cc539..0x4cc551).
- Depth table bytes `[0x85c674 + 8*k]` k = 0..15: 0, 16, 16, 16, 8, 32, 32, 16, 32, 32, 16, 80, 0, 10, 0, 75 (dumped).
- librw: absent (librw's findFormat uses fixed tables, no caps queries).

## 0x4cc580 sub_4cc580 (proposed: _rwD3D9RasterSetFormatFromImage)  (catalogue: unknown/REA, port level 2)
- `RwBool (RwRaster* r, u32 flags)`: `cType = flags & 7`, `cFlags = flags & 0xf8` (0x4cc58d/0x4cc59a); `f = 0x4cc1d0(cType, flags)`;
  `cFormat = f >> 8` (0x4cc5a5); `depth = [0x85c674 + ((f >> 8) & 0xf)*8]` (0x4cc5ab..0x4cc5b6); returns 1. Caller
  `_rwD3D9ImageFindRasterFormat` 0x7fff00 (3 sites).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x4cd250 _rwD3D9RasterConvertToNonPalettized  (catalogue: specified/REA, port level 2)
- `void (RwRaster* r)`: `img = RwImageCreate(r->width, r->height, 32)`; NULL -> return. `RwImageAllocatePixels(img)`;
  `_rwD3D9ImageGetRaster(img, r->parent (+0), 0)` (0x4cd288). Pixel format: if `ext->alpha`: `[0xc9bef4] > 16` and
  `0x4cc0c0(cube, type, 0x15)` -> 0x500; else `0x4cc0c0(.., 0x1a)` ? 0x300 : 0x100 (0x4cd2a5..0x4cd2e4). No alpha: depth > 16 ->
  `0x4cc0c0(.., 0x16)` ? 0x600 : 0x200; else 0x200. `_rwD3D9RasterDestroy(0, r, 0)`; `_rwD3D9RasterCreate(0, r, (cType & 7) |
  fmt | 0x9000)` (MIPMAP|AUTOMIPMAP forced, result ignored, 0x4cd33e); `_rwD3D9RasterSetImage(r, img, 0)`; `RwImageDestroy(img)`.
- librw: absent.

## 0x4cd360 _rwD3D9NativeTextureGetSize  (catalogue: specified/REA, port level 1)
- `RwBool (i32* size, const RwTexture* t, i32)`: `raster == NULL` -> `*size = 0x54`, return 1. `base = 0x64`; `cFormat & 0x40`
  (PAL4) -> 0xe4; `& 0x20` (PAL8) -> 0x464 (0x4cd380..0x4cd3a1). Levels: `autogen (ext+0xa & 0xf)` ? 1 : `tex->GetLevelCount()`
  (vtbl+0x34). For each level from the last down: `GetLevelDesc` (vtbl+0x44), `LockRect(level, &lr, NULL, 0x8010)` (vtbl+0x4c;
  cube: face 0), `UnlockRect` (vtbl+0x50); `bytes = compressed (ext+0xa & 0xf0) ? (Height >> 2) * Pitch : Pitch * Height`; `sum
  += bytes + 4`. Cube (`ext+9 & 0xf`): `size = base + sum*6` (0x4cd436..0x4cd43d); else `base + sum`. Returns 1.
- librw: d3d/d3d9.cpp native texture size differs (no surface locking).

## 0x4cd4d0 _rwD3D9NativeTextureWrite  (catalogue: specified/REA, port level 1)
- `RwBool (RwStream* s, const RwTexture* t)`: `size` from 0x4cd360; `Header(s, 1, size - 0xc)`; raw 0x48 bytes `{platform 9,
  filterAddressing = (((fa >> 12) & 0xf) << 4 | ((fa >> 8) & 0xf)) << 8 | (fa & 0xff), name[32] (t+0x10), mask[32] (t+0x30)}`
  (0x4cd51d..0x4cd578). Raster header 0x10 bytes: `{rasterFormat = cFormat << 8, d3dFormat = ext+0x18, u16 width, u16 height,
  u8 depth, u8 RwRasterGetNumLevels(r), u8 cType, u8 flags}` with flags bit0 alpha, bit1 cube, bit2 autoMipmap, bit3
  compressed (0x4cd591..0x4cd61a). Palette: PAL4 -> `RwRasterLockPalette(r, 2)`, write 0x80 bytes, unlock; PAL8 -> 0x400 bytes.
- Pixels: `ext+9 &= 0xf` (face reset, 0x4cd6a2); faces = cube ? 6 : 1; per face, per level `i < numLevels` (only level 0 when
  autogen): `RwRasterLock(r, i, 2)` (NULL -> 0), `GetLevelDesc` (vtbl+0x44), `size = compressed ? (Height >> 2)*lockedPitch :
  lockedPitch*Height` (`ext+0x10` pitch), write the u32 size, then the bytes in 0x400 chunks plus the remainder, `RwRasterUnlock`.
  After each cube face the face nibble (`ext+9` high) is incremented (0x4cd7ed..0x4cd7f5). Returns 1.
- librw: d3d/d3d9.cpp:writeNativeTexture matches in layout (librw does not lock rasters to get the pitch).

## 0x4caea0 _rwD3D9RasterClearGeneric  (catalogue: specified/REA, port level 0)
- Convention (register + stack): ESI = `RwRect*` (x +0, y +4, w +8, h +0xc), `[esp+4]` = u32 colour; returns 1/0. Callers 0x4cb4c0,
  0x4cb4e0. `ctx = [0xb4e9e4]`; `switch (ctx->cType)` on the full byte, `> 5` -> 0 (0x4caeb0), table 0x4cb49c: 0/4 texture
  (0x4caf77), 1 zbuffer (0x4cb413), 2 camera (0x4caec3), 3 -> 0, 5 camera texture (0x4cb370).
- Colour conversion (camera and camera-texture paths, and the D3D texture fill): if `ctx->depth != 32`:
  `_rwD3D9PixelToRGB(&rgba, &colour, cFormat << 8)` then `_rwD3D9RGBToPixel(&c, &rgba, 0x500)` (to 8888); else `c = colour`
  (0x4caec3..0x4caef7). The texture fill path converts unconditionally (0x4cafa2..0x4cafcc).
- D3D rectangle: `{x1 = ctx->nOffsetX + rect.x, y1 = ctx->nOffsetY + rect.y, x2 = x1 + rect.w, y2 = y1 + rect.h}` (e.g. 0x4caf1a..0x4caf53).
- Camera (2): `_rwD3D9SetRenderTarget(0, [0xc97c30])`, `_rwD3D9SetDepthStencilSurface([0xc97c2c])`; `device->Clear(1, &r, 1 TARGET, c, 1.0f,
  0)` (vtbl+0xac, 0x4caf65); return 1.
- ZBuffer (1): root walk; `_rwD3D9SetDepthStencilSurface(rootExt->+0)`; `device->Clear(1, &r, 2 ZBUFFER, 0, 1.0f, 0)`; restore
  `_rwD3D9SetDepthStencilSurface([0xc97c2c])` (0x4cb413..0x4cb486); return 1.
- Texture (0/4): `p = ctx->parent` (one level, not a full root walk, 0x4caf77); `tex->GetLevelDesc(0, &desc)` (vtbl+0x44). `desc.Pool == 0`
  (DEFAULT, render-target texture): convert colour, surface via GetCubeMapSurface(face)/GetSurfaceLevel(0) (vtbl+0x48),
  `device->ColorFill(surf, &r, c)` (vtbl+0x8c, 0x4cb03f), `surf->Release()`; return 1.
  Otherwise software fill (0x4cb05b): `_rwD3D9RasterLock(&pix, ctx, 1)`; NULL -> 0. By `ctx->depth`: 32 -> every dword of the rect rows
  (`row = pix + (rect.y + y)*ctx->stride + rect.x*4`) = colour; 16 -> u16 colour; 8 -> nearest palette entry: for the 256 entries of
  `rootExt->palette` (4 bytes each) `d = sum over the 4 bytes of (entry.k - colour.byte_k)^2` (signed ints), keep the first strictly
  smaller (unsigned compare, start 0xffffffff), stop at `d == 0` (0x4cb180..0x4cb301), then fill bytes with the index; other depths
  -> no fill and result 0. `_rwD3D9RasterUnlock(pix, ctx, 1)` (0x4cb357); return 1 if filled.
- Camera texture (5): colour conversion, root walk, surface as above, then the ColorFill + Release tail (jump to 0x4cb01a); return 1.
- librw: absent.

