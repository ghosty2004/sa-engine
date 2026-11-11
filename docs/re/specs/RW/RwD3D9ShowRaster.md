# RW D3D9 driver: _rwD3D9RasterShowRaster (RE-SPEC-RW-3 lead)

Globals: InsideScene [0xc97c54], _RwD3DDevice [0xc97c28], D3DPRESENT_PARAMETERS Present at 0xc9c040 (+0x34 PresentationInterval
[0xc9c074]), _RwD3D9RenderSurface [0xc97c30], _RwD3D9DepthStencilSurface [0xc97c2c], MaxNumLights [0xc98084], LightsCache
[0xc98088] (stride 0x6c: D3DLIGHT9 0x68 bytes + enabled flag at +0x68), D3D9RestoreDeviceCallback [0xc980b0],
_RwD3D9RasterExtOffset [0xb4e9e0] (raster extension: +0x1c IDirect3DSwapChain9* for camera rasters with their own swap chain).

## 0x7f99b0 _rwD3D9RasterShowRaster  (catalogue: specified/REA, port level 3)
- `RwBool (RwRaster* raster [esp+4], void* inOut [esp+8], RwInt32 flags [esp+0xc])`, cdecl (standard function rwSTANDARDRASTERSHOWRASTER).
- If InsideScene: `device->EndScene()` (vtbl +0xa8, 0x7f99c1), InsideScene = 0.
- V-sync switch (0x7f99dc): if `flags & 1` (rwRASTERFLIPWAITVSYNC): only when PresentationInterval == 0x80000000 (IMMEDIATE) set it to 1 and
  reset; else only when PresentationInterval == 1 set it to 0x80000000 and reset. Otherwise go straight to Present.
- Reset sequence R (0x7f9a0c, duplicated at 0x7f9b39): `D3D9DeviceReleaseVideoMemory 0x7f7f70()`; `device->Reset(0xc9c040)` (+0x40, result
  ignored); `GetRenderTarget(0, 0xc97c30)` (+0x98) + Release (+8); `GetDepthStencilSurface(0xc97c2c)` (+0xa0) + Release;
  `_rxD3D9VideoMemoryRasterListRestore 0x4cc970()` == 0 -> skip to callback; `_rwD3D9DynamicVertexBufferRestore 0x7f58d0()` == 0 -> skip to
  callback; `_rwD3D9RenderStateReset 0x7fd100()`; for i = 0 .. MaxNumLights-1 (signed): `SetLight(i, LightsCache + i*0x6c)` (+0xcc),
  `LightEnable(i, *(u32*)(LightsCache + i*0x6c + 0x68))` (+0xd4); if the loop ran and the LAST LightEnable returned < 0 skip to callback;
  `_rwD3D9Im2DRenderOpen 0x7fb480()`; if non-zero `_rwD3D9Im3DRenderOpen 0x80e020()`. Callback: if [0xc980b0] call it (no args).
- Present (0x7f9ae0): `p = raster->parent` (RwRaster+0, loaded at 0x7f9aee); `sc = *(IDirect3DSwapChain9**)(p + [0xb4e9e0] + 0x1c)`.
  If sc: `hr = sc->Present(0, 0, 0, 0, 0)` (vtbl +0xc); else `hr = device->Present(0, 0, 0, 0)` (+0x44).
- If `hr == 0x88760868` (D3DERR_DEVICELOST): `hr = device->TestCooperativeLevel()` (+0xc); if `hr == 0x88760869` (D3DERR_DEVICENOTRESET)
  run reset sequence R (without changing the interval); hr keeps 0x88760869.
- Always `0x7f5800()` (0x7f9c0d), then return `hr >= 0` (`setge`). After a device reset the function returns FALSE; the vsync-switch
  reset path does not influence the return value.
- librw: d3d/d3ddevice.cpp showRaster differs: no vsync interval switching, no device-lost recovery inside showRaster, no per-camera swap chain.
- Open questions: UNCLEAR what 0x7f5800 does (called after every present; not specified in this pass).

