# RW D3D9 driver: device system, state caches and engine options (RE-SPEC-RW-2)

All `__cdecl` unless stated. COM: IDirect3D9 vtbl+0x18 GetAdapterModeCount, +0x1c EnumAdapterModes, +0x20 GetAdapterDisplayMode,
+0x2c CheckDeviceMultiSampleType; IDirect3DDevice9 vtbl+0x88 StretchRect, +0x94 SetRenderTarget, +0x9c SetDepthStencilSurface,
+0x104 SetTexture, +0x148 DrawIndexedPrimitive, +0x15c SetVertexDeclaration, +0x170 SetVertexShader, +0x190 SetStreamSource,
+0x1a0 SetIndices, +0x1ac SetPixelShader.

## Layout: driver globals used here
- `[0x8e2440]` `_rwD3D9LastFVFUsed`, `[0x8e2444]` LastVertexDeclarationUsed, `[0x8e2448]` LastVertexShaderUsed, `[0x8e244c]`
  LastPixelShaderUsed, `[0x8e2450]` LastIndexBufferUsed: .data, initial value 0xffffffff ("unknown"), reset to -1 on device
  (re)acquire (0x7f7f77-0x7f7f8f, 0x7f9d72-0x7f9d8a).
- Stream-source cache `[0xc97bd8 + 0x10*s]`, s = 0..3: +0x0 vertex buffer, +0x4 offset, +0x8 stride (0x7fa044-0x7fa06e); reset
  to `{-1, 0, 0}` per stream (loop `cmp eax, 0x40`, 0x7f7f97-0x7f7faf).
- Light cache: freelist `[0xc98080]` (entries 0x40 bytes, 0xf per block, created at 0x7f6b9d), slot table `[0xc97c70]` 0x104 dwords
  (loop bound 0x410 bytes), `[0xc97c68]` count, `[0xc98070]` current entry initialised from the identity matrix at .rdata 0x8847a0
  (16 floats, `[0x8847a0] = 1.0f`, `[0x8847a4] = 0.0f`, `[0x8847b4] = 1.0f`, `[0x8847c8] = 1.0f`, `[0x8847dc] = 1.0f` = 4x4 identity, copied
  with `rep movsd` ecx = 0x10).
- Multisampling: `[0x8e242c]` max levels, `[0x8e2430]` `SelectedMultisamplingLevels`, `[0x8e2434]` max quality+1,
  `[0x8e2438]` `SelectedMultisamplingLevelsNonMask` (all .data initial value 1). `[0xc9bef8]` display-mode windowed flag (bit 0).
- `[0x8e243c]` `FullScreenRefreshRateInHz` (.data initial 0xffffffff). Display-mode formats scanned: .rdata 0x884788 =
  {0x17 R5G6B5, 0x16 X8R8G8B8, 0x23 A2R10G10B10} (loop end 0x884794).
- Display modes: `[0xc97c48]` array of 0x14-byte records `{D3DDISPLAYMODE Width, Height, RefreshRate, Format; u32 flags}`,
  `[0xc97c40]` count, `[0xc9bee0]` adapter mode count.
- Render-target cache `[0xc98090 + 4*i]` (i = 0..3), depth-stencil cache `[0xc9808c]`; `[0xc97c30]` `_RwD3D9RenderSurface`,
  `[0xc97c2c]` `_RwD3D9DepthStencilSurface`, `[0xc97c34]` `NeedToCopyFromBackBuffer`, `[0xc9bcc0]` camera being updated
  (catalogue name "dgGGlobals"; RwCamera*, see 0x7f98d0).

## 0x7f6b40 _rwD3D9DeviceLightCacheOpen  (catalogue: unknown/REA, port level 2; name from rw-names)
- `void (void)`. `[0xc97c68] = 0`. If `[0xc98080]` exists: free every non-NULL slot of `[0xc97c70..+0x410)` back to it (`+0x148`)
  and zero it, then `RwFreeListPurge([0xc98080])` 0x801e00. Else `[0xc98080] = _rwFreeListCreate(0x40, 0xf, 0x10, 0x30411)`
  (alignment 0x10) and zero the 0x104-dword slot table. Then `[0xc98070] = RwFreeListAlloc([0xc98080], 0x30411)` (no NULL check)
  and copy the 64-byte identity matrix from .rdata 0x8847a0 into it (0x7f6bd5-0x7f6be4).
- The same block is inlined in `D3D9DeviceReleaseVideoMemory` (0x7f7fb1-0x7f8052) and `RwD3D9GetCurrentD3DDevice` (0x7f9daa-0x7f9e4c).
- librw: absent.

## 0x7f6bf0 D3D9CalculateMaxMultisamplingLevels  (catalogue: unknown/REA, port level 0; name from rw-names)
- `void (void)`. Sets `[0x8e242c] = [0x8e2430] = [0x8e2434] = [0x8e2438] = 1`. `windowed = ~[0xc9bef8] & 1` (0x7f6bfe/0x7f6c1d).
  For type = 2..16 (`jbe 0x10`, 0x7f6c56): if `IDirect3D9::CheckDeviceMultiSampleType(adapter [0xc97c24], devType [0x8e2428],
  fmt [0xc9bef0], windowed, type, NULL) >= 0` then `[0x8e242c] = type` (last success wins). Then
  `CheckDeviceMultiSampleType(..., D3DMULTISAMPLE_NONMASKABLE 1, &quality)`; if >= 0: `[0x8e2434] = quality + 1` and, if that is
  greater than `[0x8e242c]` (unsigned), `[0x8e242c] = quality + 1` (0x7f6c89-0x7f6c9d).
- librw: absent.

## 0x7f72e0 _rwD3D9SetStandardFunctions  (catalogue: specified/REA, port level 0)
- `RwBool (RwStandardFunc *table [esp+4], RwInt32 count [esp+8])`. Builds 27 (slot, function) pairs on the stack (0x7f72fb-
  0x7f74e6; the full list is in docs/re/rw/d3d9.md §2). If `count > 0` fill `table[0..count)` with
  `_rwD3D9StandardFuncNotImplemented` 0x7f7530 (`rep stosd`); then for the pairs in reverse order (index 26..0): if `count > 1`
  (`cmp esi, 1; jle` at 0x7f7501 - NOT a per-slot bound check) `table[slot] = fn`. Slot 1 = 0x7f8f20 and slot 21 = 0x7f7730
  (gta-reversed WindowedModeHooks). Returns 1.
- Oddity: slot indices up to 28 are written whenever `count > 1`, so a table shorter than 29 entries would overflow (the engine
  passes `rwSTANDARDNUMOFSTANDARD`).
- librw: absent (librw dispatches through its own Driver struct).

## 0x7f7530 _rwD3D9StandardFuncNotImplemented  (catalogue: specified/REA, port level 0)
- `RwBool (void*, void*, RwInt32)`: returns 0.

## 0x7f7540 D3D9DeviceSystemEnumModes  (catalogue: unknown/REA, port level 0; name from rw-names)
- `void (void)`. (Re)allocates `[0xc97c48]` to `(adapterModeCount [0xc9bee0] + 1) * 0x14` bytes via `RwEngineInstance+0x13c`
  (realloc, hint 0x1040411) or `+0x134` (malloc) (0x7f755d-0x7f7584). `IDirect3D9::GetAdapterDisplayMode(adapter, &modes[0])`
  (vtbl+0x20). Switch on `modes[0].Format - 0x15` through the byte map .rdata 0x7f7714 = {0,0,0,0,0,1,1,1,1,1,1,1,1,1,0}
  (15 entries) and targets 0x7f770c = {0x7f75c5 keep, 0x7f75d4 drop}; values outside 0..14 (`ja`, 0x7f75b4) drop. So desktop
  formats 0x15..0x19 and 0x23 keep entry 0 (`modes[0].flags = 0`, count = 1); everything else gives count = 0.
- For each format f in {0x17, 0x16, 0x23} (.rdata 0x884788): `n = GetAdapterModeCount(adapter, f)`; for i in 0..n-1:
  `EnumAdapterModes(adapter, f, i, &modes[count])`; search entries 1..count-1 for one with the same Width, Height and Format
  (0x7f7652-0x7f7664): none -> `modes[count].flags = 1`, `count++`; found -> keep the higher RefreshRate in the found entry
  (`if found.Refresh < new.Refresh` found.Refresh = new, 0x7f769a).
- Finally, if `count < [0xc9bee0] + 1`, shrink the array to `count * 0x14` (realloc).
- librw: d3d/d3ddevice.cpp: findVideoModes-like code differs (no dedupe by refresh, other format list).
- Open questions: UNCLEAR whether entry 0's flags = 0 means "windowed/desktop" (0x7f75c5) - consumers not checked here.

## 0x7f7f70 D3D9DeviceReleaseVideoMemory  (catalogue: specified/REA, port level 2)
- `RwBool (void)`. Resets the five last-used caches to -1 and the stream cache to `{-1, 0, 0}` x4; light cache reopen (inline
  copy of 0x7f6b40). `SetTexture(stage, NULL)` for stage 0..7 (vtbl+0x104); `SetIndices(NULL)`; `SetStreamSource(s, NULL, 0, 0)`
  for s = 0..3; `SetPixelShader(NULL)`; `SetVertexDeclaration(NULL)`; `SetVertexShader(NULL)`. Render target 0: if `[0xc98090] !=
  [0xc97c30]` -> `[0xc98090] = [0xc97c30]`, `SetRenderTarget(0, [0xc97c30])`. For i = 1..3: if `[0xc98090 + 4i] != 0` -> clear it,
  `SetRenderTarget(i, NULL)`. Depth: if `[0xc9808c] != [0xc97c2c]` -> update and `SetDepthStencilSurface([0xc97c2c])`.
  Then `_rwD3D9Im2DRenderClose` 0x7fb5f0, `_rwD3D9Im3DRenderClose` 0x80dfb0, `_rwD3D9DynamicVertexBufferRelease` 0x7f5840,
  `_rxD3D9VideoMemoryRasterListRelease` 0x4cb640, `RwFreeListPurgeAllFreeLists` 0x801f90; return 1.
- librw: d3d/d3ddevice.cpp:releaseVideoMemory matches in outline; differs: no light cache, fewer state resets.

## 0x7f84f0 RwD3D9EngineSetMultiSamplingLevels  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 levels)`. `levels = min(levels, [0x8e242c])` (unsigned); `[0x8e2430] = levels`. If `levels < 2`:
  `[0x8e2430] = [0x8e2438] = 1`, return. Else loop: if `CheckDeviceMultiSampleType(adapter, devType, [0xc9bef0], windowed,
  [0x8e2430], NULL) >= 0` stop, else `[0x8e2430]--` while it stays `>= 2` (0x7f8554-0x7f855d; may end at 1 without a final check).
  Finally `[0x8e2438] = ([0x8e2434] * levels) / [0x8e242c]` (unsigned, with the clamped requested `levels`, not the found one).
- librw: absent.

## 0x7f8580 RwD3D9EngineSetRefreshRate  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 refreshRate)`. `[0x8e243c] = refreshRate`; if 0 return. `best = 0`; for each format in {0x17, 0x16, 0x23}: for
  i = n-1 down to 0 (`dec esi` before use): `EnumAdapterModes(adapter, f, i, &mode)`; if `mode.RefreshRate == [0x8e243c]` ->
  `best = it` and stop scanning this format (0x7f85ed); else if `RefreshRate < [0x8e243c]` and `> best` -> `best = RefreshRate`.
  After all formats, if `[0x8e243c] != best` -> `[0x8e243c] = best` (the nearest lower rate over all modes, or the exact one).
  Note the comparisons use the requested rate read at the start of each format (`ecx` reloaded at 0x7f85d5/0x7f85f1).
- librw: absent.

## 0x7f98d0 _rwD3D9CameraEndUpdate  (catalogue: specified/REA, port level 0)
- Standard function 10, `RwBool (void*, void*, RwInt32)`. `fb = *([0xc9bcc0] + 0x60)` (0x7f98df; `[0xc9bcc0]` is the camera
  stored by BeginUpdate, +0x60 = RwCamera.frameBuffer), `top = fb->parent (+0x0)` (0x7f98e5). If `topExt->swapChain (+0x1c) == 0`
  AND `top->cType == 5` AND `[0xc97c34] != 0`: get surface level 0 of `topExt->texture` (`GetCubeMapSurface(tex, face, 0, &s)` if
  the cube nibble is set, else `GetSurfaceLevel(tex, 0, &s)`, vtbl+0x48; hr not checked); `rect = {fb.nOffsetX, fb.nOffsetY,
  fb.nOffsetX + fb.width, fb.nOffsetY + fb.height}` (0x7f9937-0x7f9964); `StretchRect([0xc97c30], &rect, s, &rect, filter)`
  (vtbl+0x88) with filter = 1 (D3DTEXF_POINT) if `[0xc9bff4] & 0x1000100` (D3DCAPS9 + 0xf4 StretchRectFilterCaps) else 0;
  `s->Release()`; `[0xc97c34] = 0`. Always `[0xc9bcc0] = 0`; return 1.
- librw: d3d/d3ddevice.cpp:endUpdate differs (EndScene only; no back-buffer copy).
- Open questions: UNCLEAR - GetSurfaceLevel's hr is ignored (0x7f9928/0x7f9934); on failure `s` is an uninitialised stack slot.

## 0x7f9c20 _rwDeviceGetHandle  (catalogue: specified/REA, port level 0)
- `RwDevice *(void)`: returns 0x8e2498 (the static RwDevice, docs/re/rw/d3d9.md §1).

## 0x7f9c30 RwD3D9DeviceSupportsDXTTexture  (catalogue: specified/REA, port level 0)
- `RwBool (void)`: returns 1 as soon as `CheckDeviceFormat(adapter, devType, [0xc9bef0], 0, D3DRTYPE_TEXTURE 3, fmt) >= 0` for fmt
  in order DXT1 0x31545844, DXT2 0x32545844, DXT3 0x33545844, DXT4 0x34545844, DXT5 0x35545844; else 0.
- librw: absent.

## 0x7f9d30 RwD3D9SetStencilClear  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 v)`: `[0xc97c44] StencilClearValue = v`.

## 0x7f9d50 RwD3D9GetCurrentD3DDevice  (catalogue: specified/REA, port level 2)
- `void *(void)`: returns `[0xc97c28]`, but FIRST resets the state caches (the RW 3.6 driver does this because the application may
  change device state behind RW): `[0xc98090..0xc9809c] = 0` (render targets), `[0xc9808c] = 0`, the five last-used caches = -1, the
  stream cache = `{-1, 0, 0}` x4, the light cache reopen (inline 0x7f6b40), then `[0xc98084] MaxNumLights = 0` and, if
  `[0xc98088] LightsCache`, free it via `RwEngineInstance+0x138` and set NULL (0x7f9e4e-0x7f9e6f).
- librw: absent.

## 0x7f9f70 _rwD3D9SetVertexDeclaration  (catalogue: specified/REA, port level 0)
- `void (void *decl)`: if `decl != [0x8e2444]`: `hr = SetVertexDeclaration(decl)`; `[0x8e2444] = hr >= 0 ? decl : -1`;
  `[0x8e2440] (LastFVF) = -1` in both cases.

## 0x7f9fb0 _rwD3D9SetVertexShader  (catalogue: specified/REA, port level 0)
- `void (void *vs)`: if `vs != [0x8e2448]`: `hr = SetVertexShader(vs)`; `[0x8e2448] = hr >= 0 ? vs : -1`.

## 0x7f9ff0 _rwD3D9SetPixelShader  (catalogue: specified/REA, port level 0)
- `void (void *ps)`: same with `[0x8e244c]` and SetPixelShader (vtbl+0x1ac).

## 0x7fa030 RwD3D9SetStreamSource  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 stream, void *vb, RwUInt32 offset, RwUInt32 stride)`: if the cache entry `[0xc97bd8 + 0x10*stream]` differs in
  vb, offset or stride: store all three and call `SetStreamSource(stream, vb, offset, stride)` (result ignored). No bound check on
  `stream`.

## 0x7fa090 _rwD3D9SetStreams  (catalogue: specified/REA, port level 0)
- `void (RxD3D9VertexStream *streams [esp+4], RwBool useOffsets [esp+8])`; streams are 0x10 bytes `{vb, offset, stride, ...}`, two
  of them. For s = 0, 1: if `streams[s].vb`: without useOffsets compare/store `(vb, 0, stride)` and call
  `SetStreamSource(s, vb, 0, stride)` on change; with useOffsets the same with `streams[s].offset`. If `vb == NULL` and the cache
  entry's vb != 0: zero the entry and `SetStreamSource(s, NULL, 0, 0)`.

## 0x7fa1c0 _rwD3D9SetIndices  (catalogue: specified/REA, port level 0)
- `void (void *ib)`: if `ib != [0x8e2450]`: `[0x8e2450] = ib` (before the call), `SetIndices(ib)` (result ignored).

## 0x7fa320 _rwD3D9DrawIndexedPrimitive  (catalogue: specified/REA, port level 1)
- `void (RwUInt32 primType, RwInt32 baseVertexIndex, RwUInt32 minIndex, RwUInt32 numVertices, RwUInt32 startIndex, RwUInt32
  primCount)`: `_rwD3D9RenderStateFlushCache()` 0x7fc200, then `DrawIndexedPrimitive(primType, baseVertexIndex, minIndex,
  numVertices, startIndex, primCount)` (vtbl+0x148; args forwarded in order from [esp+4..0x18]); result ignored.

## 0x7fa390 RwD3D9SetTransform  (catalogue: specified/REA, port level 0)
- `RwBool (RwUInt32 state [ebp+8], const void *matrix [ebp+0xc])` (D3DTRANSFORMSTATETYPE, D3DMATRIX* or NULL). Cache:
  `[0xc97c70 + 4*state]` -> 64-byte matrix from freelist `[0xc98080]` (the world slot is `[0xc98070]` = index 0x100); flag
  `[0xc97c68]` = "world is identity".
- matrix != NULL: if the slot exists and all 16 dwords equal (integer compare, 0x7fa3bc-0x7fa3cd) return 1. Else allocate the slot
  if missing (`+0x144`, hint 0x30411, no NULL check), copy the 16 dwords, `hr = device->SetTransform(state, matrix)` (vtbl+0xb0);
  if `state == 0x100` `[0xc97c68] = 0`; return `hr >= 0`.
- matrix == NULL: same with the identity at .rdata 0x8847a0 (`[0x8847a0] = 1.0f`): equal -> return 1; else allocate/copy, call
  `SetTransform(state, 0x8847a0)`, and if `state == 0x100` `[0xc97c68] = 1`; return `hr >= 0`.
- librw: d3d/d3ddevice.cpp:setTransform? differs (no cache; librw keeps matrices in its own state block).

## 0x7fa520 RwD3D9SetTransformWorld  (catalogue: specified/REA, port level 0)
- `RwBool (const RwMatrix *m [ebp+8])`. If `m == NULL` or `m->flags (+0xc) & 0x20000` (rwMATRIXINTERNALIDENTITY): if `[0xc97c68]`
  already set return 1; else copy the identity into `[0xc98070]`, `SetTransform(0x100, 0x8847a0)`, `[0xc97c68] = 1`, return
  `hr >= 0` (0x7fa616-0x7fa651).
- Else build the D3DMATRIX in the static .data buffer 0x8e24d0: rows 0..3 = m->right (+0x0..+0x8), up (+0x10..), at (+0x20..),
  pos (+0x30..), three dwords each, to 0x8e24d0/0x8e24e0/0x8e24f0/0x8e2500; the fourth column is never written and keeps its .data
  values `[0x8e24dc] = 0.0f`, `[0x8e24ec] = 0.0f`, `[0x8e24fc] = 0.0f`, `[0x8e250c] = 1.0f`. If `[0xc97c68] == 0` and the 16 dwords
  equal the cached world `[0xc98070]` return 1. Else copy into the cache, `hr = SetTransform(0x100, 0x8e24d0)`, `[0xc97c68] = 0`,
  return `hr >= 0`.
- librw: absent as such (d3d9render.cpp builds the world matrix with `convMatrix` and always sets it).

## 0x7fa660 RwD3D9SetLight  (catalogue: specified/REA, port level 1)
- `RwBool (RwInt32 index [esp+4], const D3DLIGHT9 *light [esp+8])`. Light cache `[0xc98088]`, entries 0x6c bytes (D3DLIGHT9 0x68 +
  enabled flag +0x68), count `[0xc98084]` MaxNumLights.
- `index >= count` (signed): `count = index + 1`; `[0xc98088] = realloc(cache, count*0x6c)` (`+0x13c`, hint 0x1030411) or `malloc`
  (`+0x134`) when NULL (no NULL check); entries old count .. count-1 get `enabled = 0` (0x7fa6d7-0x7fa6ed); then copy and set
  (below).
- `index < count`: if `_rwD3D9LightCompare(light, &cache[index])` 0x7fa8b0 returns non-zero (equal) return 1.
- Copy 0x1a dwords of `light` into `cache[index]` and `return device->SetLight(index, light) >= 0` (vtbl+0xcc).
- librw: absent (librw calls SetLight directly).

## 0x7fa860 RwD3D9EnableLight  (catalogue: specified/REA, port level 0)
- `RwBool (RwInt32 index, RwBool enable)`. If `index >= [0xc98084] MaxNumLights` (signed `jge`) return 1 without doing anything.
  Light cache `[0xc98088]` = array of 0x6c-byte entries (`index*3*9*4`, 0x7fa874-0x7fa877) with the enabled flag at +0x68
  (0x7fa87a). If `entry.enabled != enable`: store it and `return IDirect3DDevice9::LightEnable(index, enable) >= 0` (vtbl+0xd4);
  else return 1.
- librw: absent (librw calls LightEnable directly).

## 0x7fa8b0 _rwD3D9LightCompare  (catalogue: unknown/REA, port level 0; name from rw-names)
- `RwBool (const D3DLIGHT9 *a, const D3DLIGHT9 *b)`; all comparisons are 32-bit integer compares of the float bits (so -0.0 !=
  0.0 and NaN == NaN by bits). Returns 0 on the first difference, else 1.
- Always: Type (+0x0), Diffuse.r/g/b (+0x4/+0x8/+0xc); Diffuse.a, Specular and Ambient are NOT compared. Then by Type:
  1 POINT: Position (+0x34/+0x38/+0x3c), Range (+0x4c); 2 SPOT: Position, Direction (+0x40/+0x44/+0x48), Range, Theta (+0x60), Phi
  (+0x64); 3 DIRECTIONAL: Direction; any other Type: equal. Falloff (+0x50) and Attenuation0..2 (+0x54..+0x5c) are never compared,
  so changing only those does not reach `IDirect3DDevice9::SetLight`.
- librw: absent.

## 0x7faa30 RwD3D9CreateVertexDeclaration  (catalogue: specified/REA, port level 0)
- `RwBool (const D3DVERTEXELEMENT9 *elements [esp+4], void **declaration [esp+8])`. Cache `[0xc97c60]` of 0xc-byte entries
  `{+0x0 IDirect3DVertexDeclaration9*, +0x4 u32 numElements, +0x8 element copy}`, count `[0xc97c58]`, capacity `[0xc97c5c]`.
- `n` = index of the first 8-byte element whose Type byte (+4) is 0x11 (D3DDECLTYPE_UNUSED = D3DDECL_END), plus 1 (the end marker
  counts; 0x7faa3a-0x7faa57).
- Search entries in order for `numElements == n` and the first `n*8` bytes equal (dword compare, 0x7faa66-0x7faa88). Found: if
  `entry.decl` -> `decl->AddRef()` (vtbl+4); else `device->CreateVertexDeclaration(entry.elements, &entry.decl)` (vtbl+0x158, hr
  ignored). `*declaration = entry.decl`; return `*declaration != NULL` (0x7fabc1-0x7fac0c).
- Not found: if `count >= capacity`: `capacity += 0x10` and realloc (`+0x13c`) / malloc (`+0x134`) the array to `capacity * 0xc`
  bytes (hint 0x1040411; no NULL check). `hr = CreateVertexDeclaration(elements, declaration)`; hr < 0 -> `*declaration = NULL`,
  return 0. Else `entry[count] = {*declaration, n, malloc(n*8, hint 0x40411)}`, copy the elements, `count++`, return
  `*declaration != NULL`. The cache keeps the creation reference; `RwD3D9DeleteVertexDeclaration` 0x7fac10 releases one reference
  and clears the slot only when the count reaches 0.
- librw: d3d/d3d.cpp:createVertexDeclaration differs (no cache).

## 0x7fac10 RwD3D9DeleteVertexDeclaration  (catalogue: specified/REA, port level 0)
- `void (void *decl)`. Vertex-declaration cache: `[0xc97c60]` array of 0xc-byte entries (+0 IDirect3DVertexDeclaration9*), count
  `[0xc97c58]`. Find the first entry with `ptr == decl` (linear, 0x7fac2a); not found -> return. `refs = decl->Release()`
  (vtbl+8); if `refs == 0` (`ja` not taken on unsigned > 0, 0x7fac4a) the entry's pointer is set to 0 (entry and count kept).
- librw: d3d/d3d.cpp:destroyVertexDeclaration? differs (no cache).

## 0x7fac60 RwD3D9CreateVertexShader  (catalogue: specified/REA, port level 0)
- `RwBool (const RwUInt32 *function, void **shader)`: `hr = CreateVertexShader(function, shader)` (vtbl+0x16c); if hr >= 0
  `[0x8e2448] = -1` (invalidate the last-shader cache); return `hr >= 0`.

## 0x7facc0 RwD3D9CreatePixelShader  (catalogue: specified/REA, port level 0)
- `RwBool (const RwUInt32 *function, void **shader)`: `return CreatePixelShader(function, shader) >= 0` (vtbl+0x1a8); no cache
  invalidation (unlike 0x7fac60).

## 0x7fad20 RwD3D9GetCaps  (catalogue: specified/REA, port level 0)
- `const void *(void)`: returns 0xc9bf00 (`_RwD3D9DeviceCaps`, a D3DCAPS9 copy).

## 0x7fad30 RwD3D9CameraIsSphereFullyInsideFrustum  (catalogue: specified/REA, port level 0)
- `RwBool (const RwCamera *camera [esp+4], const RwSphere *sphere [esp+8])`. `negR = -sphere->radius (+0xc)` (x87 `fchs`, stored as
  f32 into the arg slot [esp+8], 0x7fad3e-0x7fad43). For the 6 frustum planes at `camera + 0x94`, stride 0x14 (RwFrustumPlane:
  normal +0/+4/+8, distance +0xc): `d = ((plane.y * s.y) + (plane.x * s.x)) + (plane.z * s.z) - plane.distance` (x87 order: fld
  [+4] * s[+4]; fld [+0] * s[+0]; faddp; fld [+8] * s[+8]; faddp; fsub [+0xc]; 0x7fad4d-0x7fad61); `fcomp negR` and `and ax,
  0x4100` (C0|C3): if `d < negR` or `d == negR` (or unordered, which sets C0|C3) the loop continues; if `d > negR` return 0
  (0x7fad6f). After all 6 planes return 1. So "fully inside" is reported when every plane distance is `<= -radius` (the RW plane
  normals point outwards).
- librw: absent.

## 0x7fae20 _rwD3D9DeviceSetRestoreCallback  (catalogue: specified/REA, port level 0)
- `void (void (*cb)(void))`: `[0xc980b0] D3D9RestoreDeviceCallback = cb`.

## 0x7fae30 _rwD3D9DeviceGetRestoreCallback  (catalogue: specified/REA, port level 0)
- returns `[0xc980b0]`.

## 0x7fe0a0 _rwD3D9RenderStateVertexAlphaEnable  (catalogue: specified/REA, port level 0)
- `void (RwBool enable)`; render-state cache layout in RwD3D9RenderState.md (`[0xc991d0 + 8*rs]` value, `+4` queued flag, queue
  `[0xc98b40]` with count `[0xc9a5ec]`). `[0xc9a4e8]` = vertex-alpha flag, `[0xc9a4ec]` = stage-0 texture has alpha, `[0xc9a5d4]` =
  alpha test wanted.
- enable != 0: if `[0xc9a4e8]` already set -> return. `[0xc9a4e8] = 1`; if `[0xc9a4ec]` -> return (blend already on). Set RS 0x1b
  ALPHABLENDENABLE value `[0xc992a8] = 1` and queue it if not queued (`[0xc992ac]`, queue entry 0x1b, 0x7fe0ea); then if
  `[0xc99248]` (RS 0xf ALPHATESTENABLE value) != `[0xc9a5d4]`: store it and queue RS 0xf if not queued (`[0xc9924c]`,
  0x7fe165-0x7fe184).
- enable == 0: if flag already 0 return; `[0xc9a4e8] = 0`; if `[0xc9a4ec]` return; RS 0x1b value = 0, queue; if RS 0xf value != 0:
  value = 0, queue.
- librw: d3d/d3drender.cpp setAlphaBlend? differs (librw sets the states immediately, no stage-0 alpha coupling).

## 0x7fe190 _rwD3D9RenderStateIsVertexAlphaEnable  (catalogue: specified/REA, port level 0)
- returns `[0xc9a4e8]`.

## Not written in RE-SPEC-RW-2 (next pass)
D3D9System (0x7f5f70, 802 insns, request switch summarised in docs/re/rw/d3d9.md §1) and _rwD3D9RasterShowRaster (0x7f99b0).
