# RW D3D9 driver: rasters (RE-SPEC-RW pass 1 writer C + pass 2)

Part of the D3D9 driver linked into the game half of `.text` (0x4c9970-0x4cd820, README §6). All `__cdecl` unless
stated. Pass 1 covered the small functions; pass 2 (RE-SPEC-RW-2) added plugin attach, create/format-select/lock/unlock/destroy,
the video-memory raster list, the generic render dispatcher and the native texture reader.

## Layout: D3D9 raster extension (plugin 0x40c, 0x24 bytes at `raster + [0xb4e9e0]` `_RwD3D9RasterExtOffset`)
RW 3.6 D3D9 `_rwD3D9RasterExt`: +0x00 `IDirect3DTexture9*` texture (used by SetTexture, 0x7fde09), +0x04 palette
(`+0x400` = palette index, 0x7fde21), +0x08 u8 alpha (0x7fdcfc), +0x09 u8 cube:4/face:4, +0x0a u8
automipmapgen:4/compressed:4, +0x0b u8 lockedMipLevel, +0x0c lockedSurface, +0x10 D3DLOCKED_RECT, +0x18 D3DFORMAT,
+0x1c `IDirect3DSwapChain9*` swapChain (0x4c9a6a, 0x4c9a8b), +0x20 HWND window. Fields not proven by a cited
instruction come from the SDK header and must be checked when the big functions are specified.

Driver globals used here: 0xc97c20 `_RwDirect3DObject` (IDirect3D9*), 0xc97c24 `_RwD3DAdapterIndex`, 0x8e2428
`_RwD3DAdapterType` (.data initial value 1 = D3DDEVTYPE_HAL), 0xc9bef0 current display-mode format, 0xc9c068 depth
buffer format, 0xb4e9e4 current raster context.

Pass-2 proofs of the extension fields: +0x00 texture (0x4ca002, 0x4cbba4), +0x04 palette (0x4cb83d; palette index at
palette+0x400, 0x4cb86e), +0x08 alpha (0x4cb88f, 0x4cd9a9), +0x09 cube:4 (low) / face:4 (high) (0x4ca018, 0x4cdd0d), +0x0a
autoMipmapGen:4 (low) / compressed:4 (high) (0x4cc786, 0x4cd9b5), +0x0b lockedMipLevel (0x4ca25a, 0x4ccea9 init 0xff), +0x0c
lockedSurface (0x4ca008), +0x10 D3DLOCKED_RECT {Pitch +0x10, pBits +0x14} (0x4ca24b, 0x4ca214), +0x18 D3DFORMAT (0x4cb7e8),
+0x1c swapChain (0x4ca173), +0x20 window (SDK header only, not used in these functions).

## 0x4c9a60 cb_DEVICEMODULE_RwRaster_Ctor  (catalogue: specified/REA, port level 0)
- Plugin ctor `void* (void* raster, RwInt32 offset, RwInt32 size)`: `ext->swapChain (+0x1c) = NULL` (0x4c9a6a); returns
  raster. Nothing else of the extension is initialised here.

## 0x4c9a80 cb_DEVICEMODULE_RwRaster_Dtor  (catalogue: specified/REA, port level 0)
- Plugin dtor: if `ext->swapChain != NULL`: `swapChain->Release()` (vtbl+8), set NULL. Returns raster.

## 0x4c9ab0 _rwD3D9RasterPluginAttach  (catalogue: specified/REA, port level 3)
- Convention: `RwBool __cdecl (void)`; tail-called from `_rwDeviceRegisterPlugin` 0x7f5f60. Callees: `RwRasterRegisterPlugin`
  0x7fb0b0.
- Behaviour: `off = RwRasterRegisterPlugin(size 0x24, id 0x40c, ctor 0x4c9a60, dtor 0x4c9a80, copy NULL)` (pushes 0x4c9ab3..0x4c9ac3);
  `[0xb4e9e0] _RwD3D9RasterExtOffset = off` (0x4c9acf, stored before the test); if `off < 0` return 0 (0x4c9ad4, signed `jge`).
- Then builds the D3DFORMAT info table `_rwD3D9PixelFormatInfo`† at .bss 0xb4e7e0: 128 entries x 4 bytes indexed by D3DFORMAT
  value, entry = `{u8 alpha; u8 depth; u16 rwRasterFormat}`. First the whole table is zeroed (`rep stosd`, ecx = 0x80 dwords,
  0x4c9ae0-0x4c9aea), then these entries are written (byte/word stores 0x4c9af3-0x4c9e6f; register values bl=0, al=1, cl=0x20,
  dl=0x10, si=0x900, di=0x700). All other entries (incl. every FOURCC/DXT format, which is > 127) stay 0.

| D3DFMT (index) | entry addr | alpha | depth | rwFormat |
|---|---|---|---|---|
| R8G8B8 (20) | 0xb4e830 | 0 | 24 | 0x0000 |
| A8R8G8B8 (21) | 0xb4e834 | 1 | 32 | 0x0500 (8888) |
| X8R8G8B8 (22) | 0xb4e838 | 0 | 32 | 0x0600 (888) |
| R5G6B5 (23) | 0xb4e83c | 0 | 16 | 0x0200 (565) |
| X1R5G5B5 (24) | 0xb4e840 | 0 | 16 | 0x0a00 (555) |
| A1R5G5B5 (25) | 0xb4e844 | 1 | 16 | 0x0100 (1555) |
| A4R4G4B4 (26) | 0xb4e848 | 1 | 16 | 0x0300 (4444) |
| R3G3B2 (27) | 0xb4e84c | 0 | 8 | 0 |
| A8 (28) | 0xb4e850 | 1 | 8 | 0 |
| A8R3G3B2 (29) | 0xb4e854 | 1 | 16 | 0 |
| X4R4G4B4 (30) | 0xb4e858 | 0 | 16 | 0 |
| A2B10G10R10 (31) | 0xb4e85c | 1 | 32 | 0 |
| A8B8G8R8 (32) | 0xb4e860 | 1 | 32 | 0 |
| X8B8G8R8 (33) | 0xb4e864 | 0 | 32 | 0 |
| G16R16 (34) | 0xb4e868 | 0 | 32 | 0 |
| A2R10G10B10 (35) | 0xb4e86c | 1 | 32 | 0 |
| A16B16G16R16 (36) | 0xb4e870 | 1 | 64 | 0 |
| A8P8 (40) | 0xb4e880 | 1 | 16 | 0 |
| P8 (41) | 0xb4e884 | 0 | 8 | 0x2000 (PAL8) |
| L8 (50) | 0xb4e8a8 | 0 | 8 | 0x0400 (LUM8) |
| A8L8 (51) | 0xb4e8ac | 1 | 16 | 0 |
| A4L4 (52) | 0xb4e8b0 | 1 | 8 | 0 |
| V8U8 (60), L6V5U5 (61) | 0xb4e8d0, 0xb4e8d4 | 0 | 16 | 0 |
| X8L8V8U8 (62), Q8W8V8U8 (63), V16U16 (64) | 0xb4e8d8..0xb4e8e0 | 0 | 32 | 0 |
| A2W10V10U10 (67) | 0xb4e8ec | 1 | 32 | 0 |
| D16_LOCKABLE (70) | 0xb4e8f8 | 0 | 16 | 0x0700 (16) |
| D32 (71) | 0xb4e8fc | 0 | 32 | 0x0900 (32) |
| D15S1 (73) | 0xb4e904 | 0 | 16 | 0x0700 |
| D24S8 (75), D24X8 (77), D24X4S4 (79) | 0xb4e90c, 0xb4e914, 0xb4e91c | 0 | 32 | 0x0900 |
| D16 (80) | 0xb4e920 | 0 | 16 | 0x0700 |
| L16 (81) | 0xb4e924 | 0 | 16 | 0 |
| D32F_LOCKABLE (82), D24FS8 (83) | 0xb4e928, 0xb4e92c | 0 | 32 | 0x0900 |
| Q16W16V16U16 (110) | 0xb4e998 | 0 | 64 | 0 |
| R16F (111) | 0xb4e99c | 0 | 16 | 0 |
| G16R16F (112) | 0xb4e9a0 | 0 | 32 | 0 |
| A16B16G16R16F (113) | 0xb4e9a4 | 1 | 64 | 0 |
| R32F (114) | 0xb4e9a8 | 0 | 32 | 0 |
| G32R32F (115) | 0xb4e9ac | 0 | 64 | 0 |
| A32B32G32R32F (116) | 0xb4e9b0 | 1 | 128 | 0 |
| CxV8U8 (117) | 0xb4e9b4 | 0 | 16 | 0 |

  Returns 1 (`mov eax, 1` at 0x4c9aec survives to `ret` 0x4c9e76).
- Readers of the table in this file: `[0xb4e7e2 + 4*fmt]` (u16 rwFormat, 0x4cc86b), `[0xb4e7e3 + 4*fmt]` (high byte, 0x4cbad0).
- librw: d3d/d3d.cpp keeps a static `formatInfo[]` table instead; the exe builds it at plugin attach. librw registers no
  copy callback either (matches).

## 0x4c9f90 _rwD3D9RasterLock  (catalogue: specified/REA, port level 0)
- Convention: standard function 15, `RwBool __cdecl (void **pixelsOut, RwRaster *raster, RwInt32 flags)`; args [esp+4],
  [esp+8], [esp+0xc] at entry. Also called from `_rwD3D9RasterClearGeneric` 0x4caea0. Callees: COM only (13 indirect calls).
  The arg slots [esp+8] (raster) is reused as scratch (cType byte at 0x4c9fe7, cube byte 0x4ca00c, surface pointer 0x4ca0c4).
- Decode: `level = (s8)(flags >> 8)` (`sar ecx, 8`, low byte kept, 0x4c9fc2/0x4c9fc8); `write = flags & 1` (0x4c9fc5);
  D3D lock flags = 0x800 (D3DLOCK_NOSYSLOCK) if write, else 0x8810 (NOSYSLOCK|READONLY|NO_DIRTY_UPDATE) (0x4c9fcc-0x4c9fda).
> QA (2026-11-11): `level` is unsigned, not s8: the byte stored at 0x4c9fc8 is only ever zero-extended (`movzx ecx, cl` at 0x4ca005 and 0x4ca0c1 for the GetSurfaceLevel / GetCubeMapSurface Level argument, `movzx ecx, byte [esp+0x13]` at 0x4ca217 for the `sar` shift count at 0x4ca228/0x4ca22d, byte store to ext+0xb at 0x4ca25a). Read it as `level = (u8)(flags >> 8)`. Differs only for levels >= 0x80.
- If `raster->cpPixels (+0x4) != NULL` return 0 (already locked, 0x4c9f98).
- `ext = raster + [0xb4e9e0]` (0x4c9fb0); `top` = walk `raster->parent (+0x0)` until `top->parent == top` (0x4c9fb3-0x4c9fba);
  `topExt = top + [0xb4e9e0]`.
- Switch on `raster->cType (+0x20) & 7` (0x4c9fe4-0x4c9ff7, table 0x4ca278): 0 NORMAL and 4 TEXTURE -> texture path
  0x4c9ffe; 2 CAMERA and 5 CAMERATEXTURE -> camera path 0x4ca0aa; 1, 3 and > 5 -> return 0.
- Texture path: `tex = topExt->texture (+0x0)`; if `topExt+0x9` low nibble (cube) != 0:
  `IDirect3DCubeTexture9::GetCubeMapSurface(tex, face = topExt+0x9 >> 4, level, &ext->lockedSurface (+0xc))` (vtbl+0x48,
  0x4ca022) else `IDirect3DTexture9::GetSurfaceLevel(tex, level, &ext->lockedSurface)` (vtbl+0x48, 0x4ca028). hr < 0 -> return 0.
  Then `IDirect3DSurface9::LockRect(surf, &ext->lockedRect (+0x10), rect, d3dFlags)` (vtbl+0x34) with rect = NULL if
  `top == raster` (0x4ca040) else a stack RECT `{nOffsetX (+0x1c, s16), nOffsetY (+0x1e, s16), nOffsetX + width (+0xc),
  nOffsetY + height (+0x10)}` (0x4ca04c-0x4ca078). hr < 0 -> return 0 WITHOUT releasing the surface (leak, 0x4ca084).
  On success: if `flags & 2` (LOCKREAD) `privateFlags (+0x22) |= 2` (0x4ca08a); if write `privateFlags |= 4` (0x4ca0a1).
- Camera path: if `cType & 7 == 5`: source surface = `GetCubeMapSurface`/`GetSurfaceLevel` of `topExt->texture` as above but
  into the scratch slot (0x4ca0ba-0x4ca0e3, 0x4ca16a); hr < 0 -> return 0. Else (CAMERA): source = `topExt->texture` if
  `topExt->swapChain (+0x1c) != NULL` (0x4ca173) else `[0xc97c30] _RwD3D9RenderSurface` (0x4ca187); no AddRef for either.
  `src->GetDesc(&desc)` (vtbl+0x30, 0x4ca0fa); `hr = device[0xc97c28]->CreateOffscreenPlainSurface(desc.Width, desc.Height,
  desc.Format, D3DPOOL_SYSTEMMEM 2, &ext->lockedSurface, NULL)` (vtbl+0x90, 0x4ca11c). If hr >= 0: `rd = flags & 2`; if rd:
  `device->GetRenderTargetData(src, ext->lockedSurface)` (vtbl+0x80, 0x4ca14b, result ignored); then `LockRect` as in the
  texture path (same rect rule, 0x4ca151-0x4ca1ca); if that hr < 0: `lockedSurface->Release()` (vtbl+8, 0x4ca1f4; pointer not
  cleared); else set private flags 2 (if rd) / 4 (if write). Finally, if `cType & 7 == 5`, `src->Release()` (0x4ca209), and
  return 0 if the last hr < 0 (0x4ca212).
- Common success (0x4ca214): `cpPixels = ext->lockedRect.pBits (+0x14)`; `originalWidth (+0x28) = width`;
  `originalHeight (+0x2c) = height`; `width >>= level; height >>= level` (arithmetic `sar`, 0x4ca228/0x4ca22d), each forced to
  1 if it became 0; `stride (+0x18) = lockedRect.Pitch (+0x10)`; `ext->lockedMipLevel (+0xb) = level`; `*pixelsOut =
  cpPixels`; return 1.
- librw: d3d/d3d.cpp:rasterLock differs: checks privateFlags instead of cpPixels; READ selects READONLY (exe: absence of WRITE
  does, so READ|WRITE is not read-only in the exe); librw ignores sub-raster rects, cube faces and the swap-chain surface, always
  calls GetRenderTargetData, does not save originalWidth/Height here, and asserts instead of failing.

## 0x4ca290 _rwD3D9RasterUnlock  (catalogue: specified/REA, port level 0)
- Convention: standard function 16, `RwBool __cdecl (void *unused, RwRaster *raster, RwInt32 unused)`; raster = [esp+8].
  Callees: `RwTextureRasterGenerateMipmaps` 0x7f3cd0 (cdecl `(RwRaster*, RwImage* = NULL)`), COM.
- If `raster->cpPixels == NULL` return 0 (0x4ca2a0). `ext = raster + [0xb4e9e0]`. Switch on `cType & 7` (table 0x4ca4c0):
  0/4 -> 0x4ca2cc, 2/5 -> 0x4ca342, else return 0 (0x4ca4b5).
- Texture path: `ext->lockedSurface->UnlockRect()` (vtbl+0x38), `->Release()` (vtbl+8) (pointer not cleared); `width =
  originalWidth; height = originalHeight; stride = 0; cpPixels = NULL` (0x4ca2de-0x4ca2f2). If `privateFlags & 4` AND
  `cFormat (+0x23) & 0x10` (AUTOMIPMAP) AND `ext->lockedMipLevel == 0` AND `(ext+0xa & 0xf) == 0` (no HW autogen):
  `privateFlags &= 0xf9`; `ext->lockedMipLevel = 0xff`; walk to the top parent starting at `raster->parent`;
  `RwTextureRasterGenerateMipmaps(top, NULL)` (0x4ca32d); return 1. Otherwise `privateFlags &= 0xf9` and return 1
  (lockedMipLevel unchanged).
- Camera path: restore width/height, `stride = 0`, `cpPixels = NULL`, `lockedSurface->UnlockRect()` (0x4ca35a). If
  `!(privateFlags & 4)`: `lockedSurface->Release()`, `privateFlags &= 0xf9`, return 1 (0x4ca497). Else (write-back): top =
  top parent; destination = (cType & 7 == 5) ? `GetCubeMapSurface(topExt->texture, face, ext->lockedMipLevel)` /
  `GetSurfaceLevel(topExt->texture, ext->lockedMipLevel)` (vtbl+0x48; on failure skip to the release step 0x4ca45b) :
  (`topExt->swapChain != NULL` ? `topExt->texture` : `[0xc97c30]`). If `top == raster`:
  `device->UpdateSurface(ext->lockedSurface, NULL, dest, NULL)` (vtbl+0x78, 0x4ca3cd); else
  `UpdateSurface(lockedSurface, &rect{offX, offY, offX + width, offY + height}, dest, &point{offX, offY})` (0x4ca3fc-0x4ca443;
  width/height already restored). Result ignored. If type 5 release dest (0x4ca458). Then `lockedSurface->Release()`,
  `privateFlags &= 0xf9` (0x4ca464); if type 5 AND `cFormat & 0x10` AND `lockedMipLevel == 0`: `lockedMipLevel = 0xff`,
  `RwTextureRasterGenerateMipmaps(top, NULL)` (0x4ca482; NO ext+0xa autogen test here, unlike the texture path). Return 1.
- librw: d3d/d3d.cpp:rasterUnlock differs: no write-back of camera rasters, no mipmap regeneration, clears lockedSurf, restores
  stride/pixels from original* (exe sets 0/NULL).

## 0x4cad90 _rwD3D9RasterRenderGeneric  (catalogue: unknown/REA, port level 5; subsystem UNK-game, name from rw-names, low)
- Convention (non-standard, register): EAX = source raster, EDX = rect (RwRect*), ECX = "scaled" flag (0 Render 0x4cae40 /
  Fast 0x4cae60, 1 RenderScaled 0x4cae80), one stack arg [esp+4] = blend flag (1 Render/Scaled, 0 Fast); caller cleans; returns
  EAX. Callees: 0x4caae0 (unnamed, proposed `_rwD3D9RasterCopy`†: EAX = src, stack `(rect, scaled)`), 0x4ca5a0 (unnamed,
  proposed `_rwD3D9RasterRenderQuad`†: ESI = src, EBX = scaled (read at 0x4ca652 without a save), stack `(rect, blend)`).
- Behaviour: if EAX == NULL return 0 (0x4cad90/0x4cad95). `ebx = ECX` (scaled). The destination is the current raster context
  `[0xb4e9e4]`, read through a `.HOODLUM` disguised load: 0x4cad98 jumps to 0x401ee5 which computes `ecx = [[0x15606f8] -
  [0x5da61a]]` (= `[0xb4e9e4]` in the shipped image: 0xe329d7cd - 0xe274ede9 = 0xb4e9e4) under pushfd/popfd and jumps back to
  0x4cad9d. A port reads `[0xb4e9e4]` directly.
- `switch (dest->cType)` on the FULL byte +0x20 (not masked with 7; `movzx`, 0x4cad9d) with `> 5 -> return 0`, table 0x4cadf8:
  dest 0/4 (NORMAL/TEXTURE) -> `switch (src->cType)` table 0x4cae10; dest 2/5 (CAMERA/CAMERATEXTURE) -> table 0x4cae28;
  dest 1/3 -> return 0.
  - table 0x4cae10 (dest texture): src 0, 2, 4, 5 -> copy; src 1 -> return 0; src 3 -> return 1 (no-op).
  - table 0x4cae28 (dest camera): src 0, 4, 5 -> quad; src 2 -> copy; src 1 -> return 0; src 3 -> return 1.
  - copy (0x4cadcd): `r = 0x4caae0(rect, scaled)` with EAX = src; return r != 0 ? 1 : 0.
  - quad (0x4caddf): `0x4ca5a0(rect, blend)` with ESI = src, EBX = scaled, ECX = blend (also pushed); returns 1 regardless.
- librw: absent (librw has no raster render functions for D3D9).
- Open questions: none for this body; 0x4caae0 and 0x4ca5a0 are not specified yet (not in this pass's list).

## 0x4cae60 _rwD3D9RasterRenderFast  (catalogue: specified/REA, port level 6)
- Standard function 19 (`RwBool (void* raster, void* rect, RwInt32 flags)`): calls the internal blit 0x4cad90 with a
  NON-STANDARD convention: EAX = raster (arg 0), EDX = rect (arg 1), ECX = 0, one stack argument 0; returns its EAX.
  The caller's `flags` argument is ignored.

## 0x4cb520 _rwD3D9SetRasterContext  (catalogue: specified/REA, port level 0)
- Standard function 11 `(void* out, void* raster, RwInt32 n)`: `[0xb4e9e4] = raster`; returns 1.
- Note: the blit routine 0x4cad90 reads this global through a `.HOODLUM` disguised load (0x401ee5: the address is
  computed as `[0x15606f8] - [0x5da61a]` = 0xb4e9e4 in the shipped image); a port must read [0xb4e9e4] directly.

## 0x4cb530 _rxD3D9VideoMemoryRasterListAdd  (catalogue: specified/REA, port level 0)
- `void __cdecl (RwRaster *raster)`. `node = RwEngineInstance[0xc97b24]->freeListAlloc (+0x144)([0xb4ea00], 0x30411)`
  (0x4cb541; no NULL check); `node->raster (+0x0) = raster; node->next (+0x4) = [0xb4e9fc]; [0xb4e9fc] = node` (push front).
- Globals: `[0xb4e9fc]` video-memory raster list head (node = `{RwRaster *raster; node *next}`, 8 bytes), `[0xb4ea00]` its
  RwFreeList (created by 0x4cc150).
- librw: d3d/d3d.cpp:addVidmemRaster matches in purpose (librw uses an intrusive link in the raster extension).

## 0x4cb560 _rxD3D9VideoMemoryRasterListRemove  (catalogue: specified/REA, port level 0)
- Register convention: ESI = raster (not on the stack; callers 0x4cbb00 keep the raster in ESI); clobbers EAX/ECX/EDX; returns
  nothing meaningful.
- Walks the list from `[0xb4e9fc]` keeping `prev` (ECX, starts 0) until `node->raster == ESI` (0x4cb570); not found -> return
  (0x4cb57d). Unlink: if `prev` `prev->next = node->next` (0x4cb585) else if `[0xb4e9fc] == node` `[0xb4e9fc] = node->next`;
  then `freeListFree (+0x148)([0xb4ea00], node)` (0x4cb5a4). Only the first match is removed.
- librw: d3d/d3d.cpp:removeVidmemRaster matches in purpose.

## 0x4cb640 _rxD3D9VideoMemoryRasterListRelease  (catalogue: specified/REA, port level 0)
- `void __cdecl (void)`; called on device loss (D3D9DeviceReleaseVideoMemory). For each node of `[0xb4e9fc]` (next read before
  the body, 0x4cb652): `r = node->raster`; skip if `r->parent != r` (sub-raster, 0x4cb655); `ext = r + [0xb4e9e0]`; switch on
  the full byte `r->cType` (table 0x4cb6e0, > 5 skip):
  - 0 NORMAL, 4 TEXTURE: if `ext->texture != NULL` and `ext->d3dFormat (+0x18)` in [0x46, 0x50] (D16_LOCKABLE..D16, signed
    compares 0x4cb67b/0x4cb680): `Release()`, `ext->texture = NULL`.
  - 5 CAMERATEXTURE: if texture: `Release()`, `ext->texture = NULL` (0x4cb693).
  - 1 ZBUFFER: if texture and `texture != [0xc97c2c] _RwD3D9DepthStencilSurface`: `Release()`, NULL (0x4cb6a7).
  - 2 CAMERA: if `ext->swapChain (+0x1c)`: `swapChain->Release()`, pointer NOT cleared (0x4cb6c3).
  - 3: nothing.
- librw: d3d/d3d.cpp:releaseVidmemRasters differs (librw releases every camera texture/zbuffer and nulls the swap chain).

## 0x4cb7c0 _rwD3D9RasterCreatePalettized  (catalogue: unknown/REA, port level 2; subsystem UNK-game; proposed: `_rwD3D9RasterCreateTextureRaster`†)
- Convention (register + stack): EDI = ext of the raster, [esp+4] = raster; `__cdecl` return EAX. Called only from
  `_rwD3D9RasterCreate` 0x4cce60 (types 0/4 when not DONTALLOCATE). The low-confidence rw-names name is misleading: it creates
  every NORMAL/TEXTURE raster, palettized or not.
- Globals: format table `_rwD3D9RasterFormatInfo`† .rdata 0x85c670, 16 x 8 bytes indexed by `(format >> 8) & 0xf`:
  `{u32 d3dFormat; u8 depth; u8 alpha; u16 pad}`; valid rows 0..10: 0 = {0,0,0}, 1 1555 = {25, 16, 1}, 2 565 = {23, 16, 0},
  3 4444 = {26, 16, 1}, 4 LUM8 = {50, 8, 0}, 5 8888 = {21, 32, 1}, 6 888 = {22, 32, 0}, 7 16 = {80, 16, 0}, 8 24 = {77, 32, 0},
  9 32 = {71, 32, 0}, 10 555 = {24, 16, 0}; rows 11..15 overlap unrelated .rdata (format nibbles 0xb..0xf would read
  garbage). Palette freelist `[0xb4e9f8]`, palette-index free stack `[0xb4e9f4]` (u16 array) with count `[0xb4e9e8]`, next fresh
  index `[0xb4e9f0]`, palettized-raster list head `[0xb4e9fc]` (shared with the video-memory list) / freelist `[0xb4ea00]`.
- Behaviour: `fmt = cFormat (+0x23) << 8` (0x4cb7c6).
  1. If `fmt & 0x6000 == 0` (not PAL4/PAL8): `ext->d3dFormat (+0x18) = [0x85c670 + 8*nibble]` (0x4cb7e1).
     Else only `fmt & 0xf00` == 0x600 (888) or 0x500 (8888) is accepted (else return 0, 0x4cb7fe); create the palette
     freelist on first use `[0xb4e9f8] = _rwFreeListCreate(0x404, 0x40, 4, 0x30411)` (0x4cb819); `ext->palette (+0x4) =
     RwEngineInstance->freeListAlloc (+0x144)([0xb4e9f8], 0x30411)` (0x4cb832); if non-NULL its palette index
     `palette+0x400` = (`[0xb4e9e8]` != 0 ? `[0xb4e9f4][--[0xb4e9e8]]` (u16) : `[0xb4e9f0]++`) (0x4cb842-0x4cb86e); a NULL
     palette is NOT an error. `ext->d3dFormat = 0x29` (D3DFMT_P8, 0x4cb878).
  2. `ext->alpha (+0x8) = [0x85c675 + 8*nibble]` (0x4cb887); `_rwD3D9ClampRasterSize`† 0x7fed70(&width (+0xc), &height (+0x10),
     cType | fmt)` (0x4cb8a2).
  3. Create: if cube (`ext+0x9 & 0xf`): `device->CreateCubeTexture(width, levels, usage, ext->d3dFormat, D3DPOOL_MANAGED 1,
     &ext->texture, NULL)` (vtbl+0x64, 0x4cb8df) with `levels = (fmt & 0x8000) ? 0 : 1` (`shr esi, 0xf; not; and 1`) and
     `usage = (ext+0xa & 0xf) ? 0x400 (AUTOGENMIPMAP) : 0`. Else if d3dFormat in [0x46, 0x50] (depth formats, signed):
     `device->CreateTexture(width, height, 1, D3DUSAGE_DEPTHSTENCIL 2, fmt, D3DPOOL_DEFAULT 0, &ext->texture, NULL)` (vtbl+0x5c,
     0x4cb90b) and on success `_rxD3D9VideoMemoryRasterListAdd(raster)` (0x4cb919). Else
     `D3DResourceSystem::CreateTexture(width, height, levels, d3dFormat, ext)` (0x730510, cdecl, 5 args, 0x4cb935).
  4. hr < 0: if `d3dFormat == 0x29` and palette: `ESI = palette+0x400; call 0x4cb730` (palette-index release, register arg
     ESI; its body is reached through a second `.HOODLUM` disguised load at 0x40a288 that yields `[0xb4e9e8]`), free the palette
     (`+0x148`), `ext->palette = NULL`; return 0.
  5. Success: if palette: push a node `{raster, [0xb4e9fc]}` from `[0xb4ea00]` (no NULL check) onto `[0xb4e9fc]` (0x4cb988-0x4cb9b2).
     Return 1.
- librw: d3d/d3d.cpp:rasterCreateTexture differs: allocates a 1 KiB palette with rwNew (no index pool), computes the level count
  itself, no depth-texture special case, no GTA texture pool (D3DResourceSystem), no palette list.
- Open questions: 0x4cb730 (palette-index release) is not specified in this pass.

## 0x4cb9c0 _rwD3D9RasterCreateTexture  (catalogue: unknown/REA, port level 1; subsystem UNK-game; proposed: `_rwD3D9RasterCreateCameraTexture`†)
- Convention: ESI = ext, [esp+4] = raster; returns EAX. Called for CAMERATEXTURE (type 5) rasters only (0x4ccf0a).
- Behaviour: `fmt = cFormat << 8`; if `fmt & 0x6000` return 0 (0x4cb9d4). `ext->d3dFormat = [0x85c670 + 8*nibble]`,
  `ext->alpha = [0x85c675 + 8*nibble]` (0x4cb9e3/0x4cb9ec); `0x7fed70(&width, &height, cType | fmt)`;
  `device->EvictManagedResources()` (vtbl+0x14, 0x4cba14).
  `levels = (fmt & 0x8000) ? 0 : 1`; `usage = ((ext+0xa & 0xf) ? 0x400 : 0) | 1` (RENDERTARGET); cube (`ext+0x9 & 0xf`) ->
  `CreateCubeTexture(width, levels, usage, d3dFormat, D3DPOOL_DEFAULT 0, &ext->texture, NULL)` (vtbl+0x64) else
  `CreateTexture(width, height, levels, usage, d3dFormat, 0, &ext->texture, NULL)` (vtbl+0x5c).
  If hr == 0x8876017c (D3DERR_OUTOFVIDEOMEMORY) return 0 (0x4cba6f). If hr >= 0 return 1. Otherwise retry once with format
  `[0xc9c048]` (present-parameters BackBufferFormat) and usage WITHOUT the RENDERTARGET bit (0x4cba9a-0x4cbac4: only `& 0x400`,
  no `or 1`); on failure return 0; on success `cFormat = [0xb4e7e3 + 4*[0xc9c048]]` (rwFormat >> 8 from the 0x4c9ab0 table) and
  `depth (+0x14) = [0xc9bef4]` (display depth) (0x4cbad0-0x4cbae4) but `ext->d3dFormat` and `ext->alpha` are NOT updated.
- librw: d3d/d3d.cpp:rasterCreateCameraTexture differs: no EvictManagedResources, no fallback, no cube maps, adds to the
  video-memory list here (the exe does it in 0x4cce60).

## 0x4cbb00 _rwD3D9RasterDestroy  (catalogue: specified/REA, port level 0)
- Convention: standard function 5, `RwBool __cdecl (void *unused, RwRaster *raster, RwInt32 unused)`; raster [esp+8]; ESI holds
  the raster for the register-arg callee 0x4cb560.
- Behaviour: for stage = 0..7 (`ebx`, `jb 8`, 0x4cbb29): if `_rwD3D9GetCurrentTexture(stage)` 0x7fde50 == raster:
  `0x7fdcd0(NULL, stage)` (`_rwD3D9RWSetRasterStage`, CAT-17 proposal). If `raster->parent != raster` (sub-raster): if
  `cType == 2` (full byte) remove from the list (0x4cbc84); return 1. Else switch on the full byte cType (table 0x4cbc94):
  - 0 NORMAL (0x4cbb4b) and 4 TEXTURE (0x4cbbbd): if `cFlags (+0x21) & 0x80` (DONTALLOCATE, `js`) return 1. If
    `ext->palette`: list-remove (0x4cb560), palette-index release (0x4cb730, ESI = palette+0x400), freelist free palette, NULL;
    else if `d3dFormat` in [0x46, 0x50]: list-remove. Then if `ext->texture`: type 0 -> `texture->Release()` (vtbl+8);
    type 4 -> `D3DResourceSystem::DestroyTexture(texture)` 0x730b70. The pointer is not cleared. Return 1.
  - 5 CAMERATEXTURE (0x4cbc2e): unless DONTALLOCATE: list-remove; `Release()` if non-NULL.
  - 1 ZBUFFER (0x4cbc4f): unless DONTALLOCATE: list-remove; `Release()` unless NULL or `== [0xc97c2c]`.
  - 2 CAMERA: list-remove; return 1. 3 or > 5: return 0 (0x4cbc78).
- librw: d3d/d3d.cpp:rasterDestroy (destroyNativeRaster) differs: no stage unbinding, every type releases through the COM object,
  no GTA texture pool, no DONTALLOCATE test.

## 0x4cbd80 _rwD3D9CheckValidZBufferFormat  (catalogue: specified/REA, port level 0)
- `RwBool (D3DFORMAT fmt)`: `IDirect3D9::CheckDeviceFormat(adapter [0xc97c24], devType [0x8e2428], adapterFmt [0xc9bef0],
  D3DUSAGE_DEPTHSTENCIL 2, D3DRTYPE_SURFACE 1, fmt)` (vtbl+0x28); if hr >= 0 also `CheckDepthStencilMatch(adapter,
  devType, adapterFmt, adapterFmt, fmt)` (+0x30). Returns `hr >= 0` of the last call made.

## 0x4cbde0 _rwD3D9CheckValidTextureFormat  (catalogue: specified/REA, port level 0)
- `RwBool (D3DFORMAT fmt)`: `CheckDeviceFormat(adapter, devType, adapterFmt, 0, D3DRTYPE_TEXTURE 3, fmt) >= 0`.

## 0x4cbe20 _rwD3D9CheckValidCameraTextureFormat  (catalogue: specified/REA, port level 0)
- `RwBool (D3DFORMAT fmt)`: `CheckDeviceFormat(adapter, devType, adapterFmt, D3DUSAGE_RENDERTARGET 1, D3DRTYPE_TEXTURE 3,
  fmt)`; if hr >= 0 also `CheckDepthStencilMatch(adapter, devType, adapterFmt, fmt, [0xc9c068])`. Returns `hr >= 0`.

## 0x4cc150 _rxD3D9VideoMemoryRasterListCreate  (catalogue: specified/REA, port level 0)
- `RwBool __cdecl (void)`: `[0xb4ea00] = _rwFreeListCreate(8, 0x7f, 4, 0x30411)` (0x4cc15b); returns the freelist pointer
  (non-zero = success).

## 0x4cc5c0 _rwD3D9RasterCreateFormatSelect  (catalogue: unknown/REA, port level 1; subsystem UNK-game)
- Convention (register): EAX = flags (type | format, the `RwRasterCreate` flags), EDI = raster; no stack args; returns EAX
  (1 ok / 0 fail). Callees: `_rwD3D9CheckValidTextureFormat` 0x4cbde0, `_rwD3D9CheckValidCameraTextureFormat` 0x4cbe20,
  `_rwD3D9CheckValidZBufferTextureFormat` 0x4cbe80, `_rwD3D9CheckAutoMipmapGenTextureFormat` (thunk 0x4cbf80 whose first
  instruction `test [0xc9bf0c], 0x40000000` sits in a `.HOODLUM` stub at 0x4067e3), `_rwD3D9CheckAutoMipmapGenCameraTextureFormat`
  0x4cbfd0 (rw-names calls it AutoMipmapGenTextureFormat).
- Globals: `[0xc9bef4]` display depth, `[0xc9c048]` BackBufferFormat, `[0xc9befc]` `_RwD3D9ZBufferDepth`, `[0xc9bf3c]` byte =
  low byte of D3DCAPS9.TextureCaps (caps base 0xc9bf00 + 0x3c; sign bit = D3DPTEXTURECAPS_ALPHAPALETTE 0x80).
- Behaviour: `cType = flags & 7` stored to +0x20, `cFlags = flags & 0xf8` stored to +0x21 (0x4cc5c9/0x4cc5df);
  `f = flags & 0xff00`. Switch on `flags & 7` (table 0x4cc94c): 0/4 -> 0x4cc5ef, 1 -> 0x4cc8bb, 2 -> 0x4cc85e, 5 -> 0x4cc7a1,
  3/6/7 -> return 0. Every success path ends with `cFormat (+0x23) = f >> 8` and returns 1.
  - Texture (0/4), no pixel format (`f & 0xf00 == 0`): if PAL8 (0x2000): require CheckValidTextureFormat(P8 0x29) else fail;
    `f |= (TextureCaps & 0x80) ? 0x500 : 0x600`, depth (+0x14) = 8. Otherwise (incl. PAL4 without format, 0x4cc5fd only tests
    0x2000): if `[0xc9bef4] > 16` and CheckValid(A8R8G8B8 0x15): `f |= 0x500`, depth 32; else if CheckValid(A4R4G4B4 0x1a):
    `f |= 0x300`, depth 16; else `f |= 0x100`, depth 16 (no check).
  - Texture with a format: PAL4 (0x4000) -> fail. PAL8: require P8 valid and the format be 0x600, or 0x500 with the
    ALPHAPALETTE cap; depth 8. Else `d3d = [0x85c670 + 8*nibble]`; depth formats [0x46, 0x50] use CheckValidZBufferTextureFormat,
    others CheckValidTextureFormat; fail -> 0; `depth = [0x85c674 + 8*nibble]`.
  - Then (0x4cc73f) if not PAL8 and `(flags & 0x9000) == 0x9000` (MIPMAP|AUTOMIPMAP) and AutoMipmapGen check of `d3d` passes:
    `ext+0xa = (ext+0xa & 0xf1) | 1` (0x4cc786).
  - CAMERATEXTURE (5): no format: if `[0xc9bef4] == 16` or CheckValidCameraTextureFormat(X8R8G8B8 0x16) fails: `f |= 0x200`,
    depth 16; else `f |= 0x600`, depth 32. With a format: CheckValidCameraTextureFormat(table d3d) else fail; depth = table.
    Then the same MIPMAP|AUTOMIPMAP test with 0x4cbfd0 (0x4cc82a) sets the ext+0xa nibble.
  - CAMERA (2): no format: `f = [0xb4e7e2 + 4*[0xc9c048]]` (u16 rwFormat of the back buffer, REPLACES f, dropping mip/pal
    bits, 0x4cc86b); depth = `[0xc9bef4]`. With a format: must equal that rwFormat exactly else fail.
  - ZBUFFER (1): no format: `[0xc9befc]` 16 -> f = 0x700, depth 16; 32 -> f = 0x900, depth 32; any other value -> success with
    cFormat = f >> 8 and depth untouched (0x4cc8ee). With a format: 0x700 needs ZBufferDepth 16, 0x900 needs 32, else fail.
- librw: d3d/d3d.cpp:rasterSetFormat differs: picks formats from `defaultFormat` tables, no caps queries, never fails.

## 0x4cc970 _rxD3D9VideoMemoryRasterListRestore  (catalogue: specified/REA, port level 0)
- Convention: `RwBool __cdecl (void)`; callers: `_rwD3D9CameraClear` 0x7f7730 (x2), `_rwD3D9TestState` 0x7f8300,
  `RwD3D9ChangeVideoMode` 0x7f8640 (x2) and others (6 call sites), i.e. after a device Reset. Returns the AND of all steps
  (local result starts 1, cleared when a step's hr < 0, 0x4ccd27); returns 1 at once when the list is empty (0x4ccd47).
- The list head `[0xb4e9fc]` is read through a `.HOODLUM` disguised load: 0x4cc973 jumps to 0x4059b5 which computes
  `eax = [rol([0x15607dc], [0x6c19ff] & 31)]` (rotate count 9; 0xfe005a74 rol 9 = 0xb4e9fc) under pushfd and returns to
  0x4cc978. A port reads `[0xb4e9fc]` directly.
- Loop over nodes (`next` saved first, 0x4cc9a0); skip sub-rasters (`r->parent != r`); `ext = r + [0xb4e9e0]`; `hr` (EBX)
  starts 0 and keeps the value of the previous node when a branch makes no call. Switch on the full byte cType (table
  0x4ccd50, > 5 skip):
  - 0/4 (0x4cc9be): if `cFormat & 0x60` (PAL4|PAL8): if `ext->palette`: `hr = device->SetPaletteEntries(palette+0x400 index,
    palette)` (vtbl+0x11c, 0x4cca1c). Else if `ext->d3dFormat` in [0x46, 0x50]: `hr = device->CreateTexture(width, height, 1,
    D3DUSAGE_DEPTHSTENCIL 2, d3dFormat, D3DPOOL_DEFAULT 0, &ext->texture, NULL)` (vtbl+0x5c, 0x4cc9f7) - no test whether
    `ext->texture` is already set. Other textures are managed and are not touched.
  - 5 CAMERATEXTURE (0x4cca29): only if `ext->texture == NULL`: `levels = (cFormat & 0x80) ? 0 : 1`; `usage = (ext+0xa & 0xf ?
    0x400 : 0) | 1`; cube (`ext+9 & 0xf`) -> `CreateCubeTexture(width, levels, usage, d3dFormat, 0, &ext->texture, NULL)` else
    `CreateTexture(width, height, levels, usage, d3dFormat, 0, ...)`. hr >= 0 -> next node. hr == 0x8876017c -> result 0, next.
    Other failure: retry with `[0xc9c048]` BackBufferFormat and usage without the RENDERTARGET bit (0x4ccaa9-0x4ccaf4), then
    (regardless of the retry's hr) `cFormat = [0xb4e7e3 + 4*BB]`, `depth = [0xb4e7e1 + 4*BB]` (byte), `ext->d3dFormat = BB`,
    `ext->alpha = [0xb4e7e0 + 4*BB]` (0x4ccaf9-0x4ccd20). (The create path 0x4cb9c0 does not update d3dFormat/alpha.)
  - 1 ZBUFFER (0x4ccb1e): if `ext->texture == NULL`: format pick identical to 0x4ccd70 (D15S1/D16 for `[0xc9befc]` 16;
    D24S8/D24X4S4/D32/D24X8 for 32; else unchanged), then `hr = CreateDepthStencilSurface(max(width, 1), max(height, 1),
    d3dFormat, [0xc9c050], [0xc9c054], FALSE, &ext->texture, NULL)` (vtbl+0x74, 0x4ccbd1; NO client-rect sharing test here). Else
    (the shared surface): `GetClientRect([0xc97c1c], &rc)`; `width = rc.right; height = rc.bottom; ext->texture = [0xc97c2c];
    ext->d3dFormat = [0xc9c068]` (AutoDepthStencilFormat) (0x4ccbdb-0x4ccc09).
  - 2 CAMERA with `ext->swapChain` (0x4ccc11): copy the 14-dword present parameters `[0xc9c040]` to the stack; `GetClientRect(
    ext->window (+0x20), &rc)`; copy.BackBufferWidth = `rc.right == 0`, BackBufferHeight = `rc.bottom == 0` (so 0 = use the
    window size, 1 for an empty window), hDeviceWindow = ext->window, EnableAutoDepthStencil = 0; `ext->swapChain = NULL`;
    `hr = device->CreateAdditionalSwapChain(&copy, &ext->swapChain)` (vtbl+0x34); if hr != 0 next node (EBX keeps the old hr).
    Else `hr = swapChain->GetBackBuffer(0, D3DBACKBUFFER_TYPE_MONO 0, &ext->texture)` (IDirect3DSwapChain9 vtbl+0x14);
    `ext->texture->Release()` (pointer kept, 0x4ccc9c); `ext->d3dFormat = copy.BackBufferFormat`.
  - 2 CAMERA without swap chain (0x4ccca8): if `[0xc9bee4] != 0` and `width != [0xc9c040]` (BackBufferWidth): `width =
    (u32)(width * [0xc9c040]) / [0xc9bee4]` (32-bit `imul`, unsigned `div`); same for height with `[0xc9bee8]` and `[0xc9c044]`
    (0x4cccc9-0x4ccce7). `[0xc9bee4]/[0xc9bee8]` = previous back-buffer size (written around 0x7f7c3b, see
    docs/re/cat16-fallthrough.md). Then `depth`, `cFormat`, `d3dFormat`, `alpha` from the back-buffer format as above.
  - 3: nothing.
- librw: d3d/d3d.cpp:restoreVidmemRasters differs: recreates camera textures and z-buffers only (no palettes, no swap chains,
  no camera rescale, no format fallback).

## 0x4ccd70 _rwD3D9RasterCreateZBuffer  (catalogue: unknown/REA, port level 1; subsystem UNK-game)
- Convention: ESI = ext, EDI = raster; returns EAX. Called for ZBUFFER rasters (0x4ccf2c).
- Format pick into `ext->d3dFormat (+0x18)` (`neg/sbb` selects, 0x4ccd87/0x4ccdd0): `[0xc9befc] == 16`: D15S1 (0x49) if
  `_rwD3D9CheckValidZBufferFormat(0x49)` 0x4cbd80 else D16 (0x50). `== 32`: D24S8 (0x4b) if valid, else D24X4S4 (0x4f) if
  valid, else D32 (0x47) if valid, else D24X8 (0x4d). Other depths: unchanged (0 from 0x4cce60).
- `ext->alpha = 0` (0x4ccde1). `GetClientRect([0xc97c1c] hwnd, &rc)` (import [0x858304]); if `rc.right == width && rc.bottom ==
  height`: `ext->texture = [0xc97c2c]` (shared default depth surface, no AddRef), return 1. Else `device->EvictManagedResources()`
  (vtbl+0x14) then `CreateDepthStencilSurface(width, height, ext->d3dFormat, [0xc9c050] MultiSampleType, [0xc9c054]
  MultiSampleQuality, FALSE, &ext->texture, NULL)` (vtbl+0x74, 0x4cce48); hr == 0x8876017c or hr < 0 -> 0, else 1.
- librw: d3d/d3d.cpp:rasterCreateZbuffer differs: always uses the AutoDepthStencilFormat (no format probing), no
  EvictManagedResources, adds to the list itself.

## 0x4cce60 _rwD3D9RasterCreate  (catalogue: specified/REA, port level 0)
- Convention: standard function 4, `RwBool __cdecl (void *unused, RwRaster *raster, RwInt32 flags)`; raster [esp+8], flags
  [esp+0xc]. Callees: 0x4cc5c0, 0x4cb7c0, 0x4cb9c0, 0x4ccd70, `_rxD3D9VideoMemoryRasterListAdd` 0x4cb530, GetClientRect.
- Init (0x4cce78-0x4ccea3): `cpPixels = palette = 0`; `cType = flags & 7`; `cFlags = flags & 0xf8`; ext: `texture = palette = 0`,
  `alpha (+8) = 0`, `+0xa = 0`, `+9 = 0`, `lockedMipLevel (+0xb) = 0xff`, `lockedSurface (+0xc) = 0`, `d3dFormat (+0x18) = 0`.
  (swapChain +0x1c is set by the plugin ctor, not here.)
- `if (!0x4cc5c0(EAX = flags, EDI = raster)) return 0`.
- If `width == 0 || height == 0` (0x4cceb3/0x4ccebc): `cFlags = 0x80` (overwrites all flags), `stride = 0`, `depth = 16` if it was
  0; if `cType == 2` push a list node (inline copy of 0x4cb530); return 1.
- Switch on the full byte cType (table 0x4cd02c): 0/4: unless `cFlags & 0x80`, `0x4cb7c0(raster)` with EDI = ext -> 0 on failure
  (no list add). 5: unless DONTALLOCATE, `0x4cb9c0(raster)` with ESI = ext, then list add. 1: unless DONTALLOCATE, `0x4ccd70`
  (ESI = ext, EDI = raster), then list add. 2 (CAMERA, 0x4ccf42): `GetClientRect(hwnd, &rc)`; unless DONTALLOCATE require
  `rc.right >= width && rc.bottom >= height` (signed) else return 0; depth from `[0xc9c048]`: 0x15/0x16/0x23 -> 32, 0x17/0x18/
  0x19 -> 16, else 0; `cFlags = 0x80`; `stride = 0`; `cpPixels = 0`; `originalWidth = width`; `originalHeight = height`;
  `ext->d3dFormat = [0xc9c048]`; list add; return 1. 3: return 1. Rasters with DONTALLOCATE return 1 without a list add.
- librw: d3d/d3d.cpp:rasterCreate differs: sets original* for every raster, never sets depth 16 for empty rasters, camera rasters
  get no client-rect check.

## 0x4cd820 _rwD3D9NativeTextureRead  (catalogue: specified/REA, port level 4)
- Convention: standard function 26, `RwBool __cdecl (RwStream *stream, RwTexture **out, RwInt32 unused)`; stream [esp+4], out
  [esp+8]. Locals: version (esp+0x1c), header 0x48 bytes at esp+0x38 = `{u32 platformId; u32 filterAddressing; char name[32];
  char mask[32]}`, raster header 0x10 bytes at esp+0x28 = `{u32 rasterFormat; u32 d3dFormat; u16 width; u16 height; u8 depth;
  u8 numLevels; u8 rasterType; u8 flags}` (flags bit0 alpha, bit1 cube, bit2 auto-mipmap, bit3 compressed).
- Header: `RwStreamFindChunk(stream, 1 (STRUCT), &len, &version)` 0x7ed2d0; fail -> 0. Version must be in [0x34000, 0x36003]
  (unsigned, 0x4cd84c/0x4cd857). `RwStreamRead(stream, hdr, 0x48) == 0x48` and `platformId == 9` (0x4cd87b); `RwStreamRead(...,
  0x10) == 0x10`. All failures before a raster exists return 0.
- Raster creation (three branches; `rf = rasterFormat`, `t = rasterType`):
  - Compressed (flags & 8, 0x4cd8a4): `CheckValidTextureFormat(d3dFormat)` else 0. `r = RwRasterCreate(width, height, depth,
    t | 0x80 | rf)` 0x7fb230 (DONTALLOCATE). `levels = (r->cFormat & 0x80) ? numLevels : 1` (0x4cd90b). If cube (flags & 2): if
    `!([0xc9bf44] & 0x30000)` (CubeTextureFilterCaps MIPF bits) and `levels > 1` -> destroy, 0; set ext+9 cube nibble
    (`& 0xf1 | 1`); if `(rf & 0x9000) == 0x9000` and `_rwD3D9CheckAutoMipmapGenCubeTextureFormat(d3dFormat)` 0x4cc020: ext+0xa
    autogen nibble (`& 0xf1 | 1`); `CreateCubeTexture(width, levels, autogen ? 0x400 : 0, d3dFormat, D3DPOOL_MANAGED 1,
    &ext->texture, NULL)` (vtbl+0x64). Not cube: same autogen test with 0x4cbf80; `D3DResourceSystem::CreateTexture(width,
    height, levels, d3dFormat, ext)`. Failure -> destroy, 0. Then `cFlags &= 0x7f`, `ext->alpha = flags & 1`, (cube: nibble set
    again), `ext+0xa = (ext+0xa & 0xf) | 0x10` (compressed), `ext->d3dFormat = d3dFormat` (0x4cd9af-0x4cd9c2).
  - Cube, not compressed (flags & 2, 0x4cda1e): `RwRasterCreate(width, height, depth, t | 0x80 | rf)`; same levels / caps / cube
    nibble / autogen (0x4cc020) logic; `CreateCubeTexture(width, levels, usage, d3dFormat, 1, &ext->texture, NULL)`; `cFlags &=
    0x7f`; `alpha = flags & 1`. `ext->d3dFormat` is NOT written in this branch (stays 0 from 0x4cce60), so the format check below
    fails whenever the file's d3dFormat is non-zero: the exe cannot load uncompressed cube textures (bytes 0x4cda1e-0x4cdb02).
  - Plain: if `rf & 0xffff6fff` (any bit besides MIPMAP/AUTOMIPMAP, 0x4cdb08): `r = RwRasterCreate(width, height, depth, t | rf)`
    (allocates through 0x4cce60) and `ext+0xa &= 0xf`. Else `r = 0x4cd050(width, height, d3dFormat, (rf & 0x9000) | t)`
    (unnamed, create-from-D3D-format, not specified here). NULL -> 0.
- Checks (0x4cdb81): if `flags & 4` and `(ext+0xa & 0xf) == 0` -> destroy, 0. Then `rf == cFormat << 8`, `d3dFormat ==
  ext->d3dFormat`, `width == r->width`, `height == r->height`, each else destroy, 0.
- Palette: PAL4 (`rf & 0x4000`): `RwRasterLockPalette(r, 1)` 0x7fb0e0, read 0x80 bytes; PAL8 (`rf & 0x2000`): read 0x400 bytes;
  short read -> `RwRasterUnlockPalette`, `RwRasterDestroy`, 0; else `RwRasterUnlockPalette`.
- Pixels (0x4cdc4c): `autoMip = cFormat & 0x10`; `cFormat &= ~0x10` (so the unlocks below do not regenerate mipmaps);
  `faces = cube ? 6 : 1`; `nLev = (ext+0xa & 0xf) ? 1 : numLevels` (file value, not the clamped `levels`); `ext+9 = cube
  nibble` (face 0). For each face (do-while): for `lv = 0 .. nLev-1`: `p = RwRasterLock(r, lv, 1 (WRITE))` 0x7fb2d0; read u32
  `size` (4 bytes, else `RwRasterUnlock` + destroy + 0); `RwStreamRead(stream, p, size) == size` else unlock + destroy + 0 (no
  check of `size` against the locked surface: overflow possible with a crafted file); `RwRasterUnlock(r)` 0x7faec0. After each
  face, if cube: face nibble += 1 (`(b & 0xf0) + 0x10`, 0x4cdd0d). Then `cFormat |= autoMip`.
- Texture: `t = RwTextureCreate(r)` 0x7f37c0 (NULL -> destroy raster, 0). `t->filterAddressing (+0x50)`: low byte = header
  byte, bits 8..11 and 12..15 replaced by the header's bits 8..11 / 12..15, other bits kept (0x4cdd3d-0x4cdd7c).
  `RwTextureSetName(t, name)`, `RwTextureSetMaskName(t, mask)`; `*out = t`; return 1.
- librw: d3d/d3d9.cpp:readNativeTexture differs: no version window, no caps/format validation, asserts on cube maps (exe: cube
  creates but uncompressed cube fails the format check), no format/size consistency checks, skips extra levels with seek instead
  of failing, never clears/restores the AUTOMIPMAP bit, reads directly into Texture fields.
- Open questions: 0x4cd050 is unspecified (outside this list); whether any shipped TXD triggers the cube path is unknown.
