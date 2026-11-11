# RW D3D9 driver: rasters (RE-SPEC-RW writer C, partial pass)

Part of the D3D9 driver linked into the game half of `.text` (0x4c9970-0x4cd820, README §6). All `__cdecl` unless
stated. This pass covers the small functions; the large raster create/lock/unlock/destroy/native-read functions are
listed at the end as not yet written.

## Layout: D3D9 raster extension (plugin 0x40c, 0x24 bytes at `raster + [0xb4e9e0]` `_RwD3D9RasterExtOffset`)
RW 3.6 D3D9 `_rwD3D9RasterExt`: +0x00 `IDirect3DTexture9*` texture (used by SetTexture, 0x7fde09), +0x04 palette
(`+0x400` = palette index, 0x7fde21), +0x08 u8 alpha (0x7fdcfc), +0x09 u8 cube:4/face:4, +0x0a u8
automipmapgen:4/compressed:4, +0x0b u8 lockedMipLevel, +0x0c lockedSurface, +0x10 D3DLOCKED_RECT, +0x18 D3DFORMAT,
+0x1c `IDirect3DSwapChain9*` swapChain (0x4c9a6a, 0x4c9a8b), +0x20 HWND window. Fields not proven by a cited
instruction come from the SDK header and must be checked when the big functions are specified.

Driver globals used here: 0xc97c20 `_RwDirect3DObject` (IDirect3D9*), 0xc97c24 `_RwD3DAdapterIndex`, 0x8e2428
`_RwD3DAdapterType` (.data initial value 1 = D3DDEVTYPE_HAL), 0xc9bef0 current display-mode format, 0xc9c068 depth
buffer format, 0xb4e9e4 current raster context.

## 0x4c9a60 cb_DEVICEMODULE_RwRaster_Ctor  (catalogue: specified/REA, port level 0)
- Plugin ctor `void* (void* raster, RwInt32 offset, RwInt32 size)`: `ext->swapChain (+0x1c) = NULL` (0x4c9a6a); returns
  raster. Nothing else of the extension is initialised here.

## 0x4c9a80 cb_DEVICEMODULE_RwRaster_Dtor  (catalogue: specified/REA, port level 0)
- Plugin dtor: if `ext->swapChain != NULL`: `swapChain->Release()` (vtbl+8), set NULL. Returns raster.

## 0x4cae60 _rwD3D9RasterRenderFast  (catalogue: specified/REA, port level 6)
- Standard function 19 (`RwBool (void* raster, void* rect, RwInt32 flags)`): calls the internal blit 0x4cad90 with a
  NON-STANDARD convention: EAX = raster (arg 0), EDX = rect (arg 1), ECX = 0, one stack argument 0; returns its EAX.
  The caller's `flags` argument is ignored.

## 0x4cb520 _rwD3D9SetRasterContext  (catalogue: specified/REA, port level 0)
- Standard function 11 `(void* out, void* raster, RwInt32 n)`: `[0xb4e9e4] = raster`; returns 1.
- Note: the blit routine 0x4cad90 reads this global through a `.HOODLUM` disguised load (0x401ee5: the address is
  computed as `[0x15606f8] - [0x5da61a]` = 0xb4e9e4 in the shipped image); a port must read [0xb4e9e4] directly.

## 0x4cbd80 _rwD3D9CheckValidZBufferFormat  (catalogue: specified/REA, port level 0)
- `RwBool (D3DFORMAT fmt)`: `IDirect3D9::CheckDeviceFormat(adapter [0xc97c24], devType [0x8e2428], adapterFmt [0xc9bef0],
  D3DUSAGE_DEPTHSTENCIL 2, D3DRTYPE_SURFACE 1, fmt)` (vtbl+0x28); if hr >= 0 also `CheckDepthStencilMatch(adapter,
  devType, adapterFmt, adapterFmt, fmt)` (+0x30). Returns `hr >= 0` of the last call made.

## 0x4cbde0 _rwD3D9CheckValidTextureFormat  (catalogue: specified/REA, port level 0)
- `RwBool (D3DFORMAT fmt)`: `CheckDeviceFormat(adapter, devType, adapterFmt, 0, D3DRTYPE_TEXTURE 3, fmt) >= 0`.

## 0x4cbe20 _rwD3D9CheckValidCameraTextureFormat  (catalogue: specified/REA, port level 0)
- `RwBool (D3DFORMAT fmt)`: `CheckDeviceFormat(adapter, devType, adapterFmt, D3DUSAGE_RENDERTARGET 1, D3DRTYPE_TEXTURE 3,
  fmt)`; if hr >= 0 also `CheckDepthStencilMatch(adapter, devType, adapterFmt, fmt, [0xc9c068])`. Returns `hr >= 0`.

## Not written in this pass (removed from this pass's index; next RE-SPEC-RW pass)
0x4c9ab0 _rwD3D9RasterPluginAttach, 0x4c9f90 _rwD3D9RasterLock, 0x4ca290 _rwD3D9RasterUnlock,
0x4cad90 _rwD3D9RasterRenderGeneric (contains a .HOODLUM disguised load, see above), 0x4cb7c0
_rwD3D9RasterCreatePalettized, 0x4cb9c0 _rwD3D9RasterCreateTexture, 0x4cbb00 _rwD3D9RasterDestroy,
0x4cc5c0 _rwD3D9RasterCreateFormatSelect, 0x4ccd70 _rwD3D9RasterCreateZBuffer, 0x4cce60 _rwD3D9RasterCreate,
0x4cd820 _rwD3D9NativeTextureRead.
