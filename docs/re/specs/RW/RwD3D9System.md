# RW D3D9 driver: device system entry point D3D9System (RE-SPEC-RW-3 lead)

Globals (names from catalog/rw-names.csv where present): WindowHandle [0xc97c1c], _RwDirect3DObject [0xc97c20] (IDirect3D9*),
_RwD3DAdapterIndex [0xc97c24], _RwD3DAdapterType [0x8e2428] (image default 1 = HAL), _RwD3DDevice [0xc97c28],
_RwD3D9DepthStencilSurface [0xc97c2c], _RwD3D9RenderSurface [0xc97c30], device-started flag [0xc97c38], NumDisplayModes
[0xc97c40], DisplayModes [0xc97c48] (array of 0x14-byte {width, height, refreshRate, D3DFORMAT, RW mode flags}), current mode index
[0xc97c18], current mode {width [0xc9bee4], height [0xc9bee8], refresh [0xc9beec], format [0xc9bef0], depth [0xc9bef4], flags
[0xc9bef8]}, adapter mode count [0xc9bee0], desktop mode copy 0xc97bc8..0xc97bd4, D3DCAPS9 _RwD3D9DeviceCaps at 0xc9bf00
(+0x1c DevCaps [0xc9bf1c], +0x58 MaxTextureWidth [0xc9bf58], +0x5c MaxTextureHeight [0xc9bf5c]), D3DPRESENT_PARAMETERS Present
at 0xc9c040 (+0x1c hDeviceWindow [0xc9c05c], +0x20 Windowed [0xc9c060], +0x2c Flags [0xc9c06c]), multithread flag [0xc97c4c],
software-VP flag [0xc97c50], multisampling: max level [0x8e242c], SelectedMultisamplingLevels [0x8e2430], max non-maskable
level [0x8e2434], SelectedMultisamplingLevelsNonMask [0x8e2438] (all image defaults 1), CPU features MMX [0xc980a0], SSE [0xc980a4],
SSE2 [0xc980a8], 3DNow [0xc980ac]. IDirect3D9 vtbl: +0x8 Release, +0x10 GetAdapterCount, +0x14 GetAdapterIdentifier, +0x18
GetAdapterModeCount, +0x20 GetAdapterDisplayMode, +0x2c CheckDeviceMultiSampleType, +0x38 GetDeviceCaps, +0x40 CreateDevice.

D3DFORMAT -> depth (byte table [0x7f6a84], same at 0x7f6aa0 / 0x7f6ae4, index fmt-0x15, values outside 0..14 -> 0): 0x15, 0x16, 0x23 -> 32;
0x17, 0x18, 0x19 -> 16; 0x1a..0x22 -> 0. D3DFORMAT -> RW raster format (byte table [0x7f6ac8] into 0x7f6ab0): 0x15 -> 0x500, 0x16 -> 0x600,
0x17 -> 0x200, 0x18 -> 0xa00, 0x19 -> 0x100, 0x23 -> 0x500 (A2R10G10B10 reported as 8888), others 0.

## 0x7f5f70 D3D9System  (catalogue: specified/REA, port level 4)
- `RwBool (RwInt32 request [esp+4], void* out [esp+8], void* inOut [esp+0xc], RwInt32 param [esp+0x10])`, cdecl; RwDevice.fpSystem
  (stored in the RwDevice template at 0x8e2498+4). 0x470-byte stack frame. `request > 0x16` -> return 0; jump table [0x7f6a1c]
  (23 entries). Requests 9, 18, 20, 21 go to the default (return 0, 0x7f6a10).
- 0 Open (0x7f62fa): `WindowHandle = *(HWND*)inOut`; `d3d = Direct3DCreate9(0x1f)` (import thunk 0x807c2b), stored; NULL -> 0.
  `Engine+0xf8 (stringFuncs.vecStrcpy)(0xc9bce0 _RwD3D9AdapterInformation, 0x8e2510)`. `n = d3d->GetAdapterCount()`; adapter = 0,
  type = 1. For adapter 0..n-1 try `GetDeviceCaps(adapter, type, 0xc9bf00)`; first success (>= 0) wins (0x7f6384). None (or n == 0):
  `d3d->Release()`, [0xc97c20] = 0, return 0. Then `[0xc9bee0] = sum GetAdapterModeCount(adapter, f)` for f in the .rdata list
  0x884788 = {0x17, 0x16, 0x23}; `GetAdapterDisplayMode(adapter, 0xc9bee4)`; depth from format; [0xc9bef8] = 0; desktop copy
  0xc97bc8..0xc97bd4 = width, height, refresh, format; [0xc97c18] = 0; call 0x7f6bf0; store the four CPU feature probes
  (0x80dbe0, 0x80dc40, 0x80dca0, 0x80dd00 -> 0xc980a0..0xc980ac); return 1.
- 1 Close (0x7f64a2): free DisplayModes (Engine+0x138) if set, NumDisplayModes = 0; release d3d if set and null it; return 1.
- 2 Start (0x7f669a): `GetAdapterDisplayMode(adapter, &mode)` (stack); zero Present (0xe dwords); `0x7f6cb0(0xc9bee4, [0xc9bef8], mode.Format)`
  fills it; Present.hDeviceWindow = WindowHandle; Present.Flags = 0; `GetDeviceCaps(adapter, type, 0xc9bf00)`.
  Behaviour flags: `[0xc97c4c] ? 4 : 0`; if [0xc97c50]: `|= 0x20`, DevCaps &= ~0x10000, [0xc9bfec] = 0 (0x7f672b-0x7f6739); else if
  `DevCaps & 0x10000`: `|= (DevCaps & 0x100000) ? 0x50 : 0x40`; else `|= 0x20`.
  `CreateDevice(adapter, type, WindowHandle, flags, 0xc9c040, 0xc97c28)` (0x7f6781); < 0 -> [0xc97c38] = 0, return 0.
  If WindowHandle: `GetClientRect` -> [0xc9bee4] = rect.right, [0xc9bee8] = rect.bottom. `device->GetRenderTarget(0, 0xc97c30)` (+0x98) then
  Release it; `device->GetDepthStencilSurface(0xc97c2c)` (+0xa0) then Release (pointers kept, references dropped). [0xc9808c] =
  [0xc98090] = [0xc98094] = [0xc98098] = [0xc9809c] = 0; 0x7f6b00 (render-state cache invalidate); 0x7f6b40; MaxNumLights [0xc98084] = 0,
  free LightsCache [0xc98088] if set; then _rxD3D9VideoMemoryRasterListCreate 0x4cc150, _rwD3D9Im2DRenderOpen 0x7fb480,
  _rwD3D9RenderStateOpen 0x7fcac0, _rwD3D9VertexBufferManagerOpen 0x7f5cb0 (return values ignored); [0xc97c38] = 1; return 1.
- 3 Stop (0x7f6871): returns `0x7f7100(out, inOut, param)`. 11 Standards (0x7f689b): returns `_rwD3D9SetStandardFunctions 0x7f72e0(out, inOut, param)`.
- 4 Register (0x7f62cb): copy 0xe dwords from the RwDevice template 0x8e2498 to out; [0xc9bcc4] = inOut; return 1.
- 5 GetNumModes (0x7f60d9): if DisplayModes == NULL call 0x7f7540; `*(RwInt32*)out = NumDisplayModes`; return 1.
- 6 GetModeInfo (0x7f6105): build list if needed; param < 0 or >= NumDisplayModes -> 0. out RwVideoMode: +0 width, +4 height, +8 depth
  (format table), +0xc flags (entry +0x10), +0x10 refRate (entry +8), +0x14 RW raster format (table above); return 1.
- 7 UseMode (0x7f5f8d): if [0xc97c38] != 0 -> 0 (cannot change while started); param range check -> 0. [0xc97c18] = param; copy the entry to
  the current-mode globals; depth from format. Multisampling probe (shared code with 16): windowed = `!(flags & 1)`; all four multisample
  globals = 1; for t = 2..16 (`jbe 0x10`): if `CheckDeviceMultiSampleType(adapter, type, format, windowed, t, NULL) >= 0` then [0x8e242c] = t;
  then `CheckDeviceMultiSampleType(adapter, type, format, windowed, 1 (NONMASKABLE), &q)`: if >= 0: [0x8e2434] = q+1 and, if q+1 > [0x8e242c],
  [0x8e242c] = q+1. Returns 1 (also when the last probe fails).
- 8 Focus (0x7f626d): device NULL -> 0. If Present.Windowed [0xc9c060] != 0 -> 1. Else `ShowWindow(WindowHandle, param ? 9 : 0)`; return 1.
- 10 GetMode (0x7f624f): `*out = [0xc97c18]`; 1. 12 GetTexMemSize (0x7f68c5): `*out = device->GetAvailableTextureMem()` (+0x10); 1.
- 13 GetNumSubSystems (0x7f64ed): `*out = d3d->GetAdapterCount()`; 1. 14 GetSubSystemInfo (0x7f6510): `GetAdapterIdentifier(param, 0, &id)`
  (stack 0x30), `vecStrcpy(out, id.Description)` (id+0x200 at [esp+0x230]); 1. 15 GetCurrentSubSystem: `*out = adapter`; 1.
- 16 SetSubSystem (0x7f6576): adapter = param; recount [0xc9bee0] (same three formats); `GetAdapterDisplayMode(adapter, 0xc9bee4)`; 0x7f7540
  (rebuild list); multisampling probe with the current flags [0xc9bef8] (depth [0xc9bef4] NOT recomputed); return 1.
- 17 FinalizeStart (0x7f68e8): `_rwProcessorInitialize 0x857404`; if SSE: MXCSR |= 0x8000 (flush-to-zero, stmxcsr/ldmxcsr on the stack).
  `_rwMatrixSetMultFn(0x8572a0 _rwX87MatrixMultiply)` always. Vector transforms via `_rwVectorSetMultFn(point, points, vector, vectors)`:
  SSE && SSE2 && !3DNow -> SSE2 set (0x80d570, 0x80d440, 0x80d600, 0x80d4e0); SSE otherwise -> SSE set (0x80d280, 0x80d070, 0x80d370,
  0x80d180); no SSE -> x87 set (0x8570ef, 0x857000, 0x857160, 0x85707f). Return 1. Note a CPU reporting 3DNow gets the SSE (not SSE2) set.
- 19 GetMaxTextureSize (0x7f69b6): `*out = max(MaxTextureWidth, MaxTextureHeight)` (unsigned; width when equal); 1.
- 22 (0x7f69f5): `*(RwUInt16*)out = 2`; return 1 (device id).
- librw: d3d/d3ddevice.cpp deviceSystemD3d9 differs: different request numbering; no adapter search loop (adapter 0), no 3-format mode
  count, no multisample probing, no CPU-specific transform selection.
- gta-reversed: the present-parameters helper 0x7f6cb0 is a gta-reversed hook (WindowedModeHooks); D3D9System itself has no hook.
- Open questions: UNCLEAR what 0x7f6bf0 and 0x7f6b40 do (not in this pass's scope); [0xc9bfec] is D3DCAPS9+0xec (zeroed only for software
  vertex processing; field identity not checked).

