# RW D3D9 driver: device, video mode, render targets, draw wrappers (RE-SPEC-RW-3 writer B)

All `__cdecl` unless stated. Pass-1 file `RwD3D9Device.md` (read-only) has the driver-global layout; this file only adds.
COM vtable offsets used here: IDirect3D9 +0x2c CheckDeviceMultiSampleType. IDirect3DDevice9 +0xc TestCooperativeLevel,
+0x40 Reset, +0x94 SetRenderTarget, +0x98 GetRenderTarget, +0x9c SetDepthStencilSurface, +0xa0 GetDepthStencilSurface,
+0xb4 GetTransform, +0xcc SetLight, +0xd0 GetLight, +0xd4 LightEnable, +0x144 DrawPrimitive, +0x14c DrawPrimitiveUP,
+0x150 DrawIndexedPrimitiveUP, +0x164 SetFVF, +0x178 SetVertexShaderConstantF, +0x1b0 SetPixelShaderConstantF;
IUnknown +0x8 Release.

## Layout: globals used in this file
- `[0xc97c28]` `_RwD3DDevice` (IDirect3DDevice9*), `[0xc97c20]` `_RwDirect3DObject` (IDirect3D9*), `[0xc97c24]` adapter index,
  `[0x8e2428]` D3DDEVTYPE (.data initial 1 = HAL), `[0xc97c1c]` HWND.
- `[0xc97c38]` "device started" flag: every public entry here returns early when it is 0 (0x7f8648, 0x7f8a98). UNCLEAR name.
- `[0xc97c18]` current video-mode index (compared 0x7f866c, written 0x7f88b2).
- `[0xc97c48]` display-mode array, 0x14-byte records `{Width +0, Height +4, RefreshRate +8, Format +0xc, flags +0x10}` (index
  scaled `lea ebx,[ebp+ebp*4]; shl ebx,2` 0x7f86a4-0x7f86a9), `[0xc97c40]` count (0x7f8660).
- `[0xc9bee4..0xc9bef8]` current mode copy in `_RwD3D9AdapterInformation`: +0x204 width [0xc9bee4], +0x208 height [0xc9bee8],
  +0x20c refresh [0xc9beec], +0x210 format [0xc9bef0], +0x214 colour depth bits [0xc9bef4], +0x218 mode flags [0xc9bef8]
  (stores 0x7f88be-0x7f8912).
- `[0xc9c040]` D3DPRESENT_PARAMETERS `Present` (0xe dwords, copied with `rep movsd` ecx = 0xe at 0x7f8678-0x7f8686);
  +0x0 BackBufferWidth [0xc9c040], +0x4 BackBufferHeight [0xc9c044], +0x20 Windowed [0xc9c060].
- `[0xc97bd4]` argument passed through to `D3D9SetPresentParameters` 0x7f6cb0 (third arg; UNCLEAR meaning, 0x7f869e).
- `[0xc98084]` MaxNumLights, `[0xc98088]` LightsCache (records 0x6c bytes: D3DLIGHT9 at +0, enabled flag at +0x68; 0x7f8759,
  0x7f876f), `[0xc980b0]` D3D9RestoreDeviceCallback (`void (*)(void)`, may be NULL; 0x7f8797).
- `[0x8e242c]` max multisampling levels, `[0x8e2430]` SelectedMultisamplingLevels, `[0x8e2434]` max quality+1,
  `[0x8e2438]` SelectedMultisamplingLevelsNonMask (pass-1 layout).

### Shared block "restore device after Reset" (R)
Used twice in each of 0x7f8640 and 0x7f8a90 (identical sequences; addresses below from 0x7f86d7-0x7f8793):
1. `device->GetRenderTarget(0, &[0xc97c30])` (+0x98, 0x7f86e5), then `[0xc97c30]->Release()` (+0x8, 0x7f86f3) (the cached
   pointer stays, the extra reference is dropped).
2. `device->GetDepthStencilSurface(&[0xc97c2c])` (+0xa0, 0x7f8703), `Release()` (0x7f8711).
3. `_rxD3D9VideoMemoryRasterListRestore` 0x4cc970; 0 -> fail. `_rwD3D9DynamicVertexBufferRestore` 0x7f58d0; 0 -> fail.
4. `_rwD3D9RenderStateReset` 0x7fd100.
5. For i = 0 while i < [0xc98084] (re-read each pass, 0x7f8768): `SetLight(i, LightsCache + i*0x6c)` (+0xcc),
   `hr = LightEnable(i, LightsCache[i].+0x68)` (+0xd4). After the loop, only the LAST LightEnable hr is tested: `hr < 0` ->
   fail (0x7f8778). If MaxNumLights <= 0 the test is skipped.
6. `_rwD3D9Im2DRenderOpen` 0x7fb480; 0 -> fail. `_rwD3D9Im3DRenderOpen` 0x80e020; 0 -> fail. Else ok.
7. If `[0xc980b0]` != NULL it is called (both on ok and fail; 0x7f8797-0x7f87a0).

## 0x7f5ae0 RwD3D9DynamicVertexBufferDestroy  (catalogue: specified/REA, port level 0)
- `void (IDirect3DVertexBuffer9* vb)`: walk the descriptor list from `[0xc97b8c]` via `+0x10 next` (0x7f5af4); first node with
  `node+0x8 == vb` (0x7f5aef): `node+0x4 inUse = 0`, `node+0xc owner = NULL` (0x7f5afc, 0x7f5aff). The D3D buffer is NOT
  released and the node stays in the list (it is reused by a later Create). No match / empty list: nothing.
- librw: d3d/d3ddevice.cpp `removeDynamicVB` differs: librw releases the buffer and unlinks/frees the node.

## 0x7f8640 RwD3D9ChangeVideoMode  (catalogue: specified/REA, port level 3)
- `RwBool (RwInt32 modeIndex)` (arg at [esp+0x5c] after `sub esp,0x48` + 4 pushes). Returns 1 except when the recovery Reset
  fails (0).
- 0x7f8640-0x7f8672: return 1 without doing anything if `[0xc97c38] == 0`, `modeIndex < 0` (signed, 0x7f865a),
  `modeIndex >= [0xc97c40]` (signed `jge`, 0x7f8666) or `modeIndex == [0xc97c18]` (0x7f866c).
- 0x7f8678-0x7f86ca: save `Present` (0xe dwords) to a local copy; `D3D9DeviceReleaseVideoMemory` 0x7f7f70;
  `device->TestCooperativeLevel()` (+0xc, result ignored, 0x7f8695); `D3D9SetPresentParameters(&modes[i], modes[i].flags (+0x10),
  [0xc97bd4])` 0x7f6cb0 (0x7f86b5); `hr = device->Reset(&Present)` (+0x40, 0x7f86ca).
- `hr < 0` (0x7f86d1) -> recovery (0x7f87af). Else run block R (0x7f86d7-0x7f8793); R ok -> success path 0x7f88ad; R fail ->
  `D3D9DeviceReleaseVideoMemory` again (0x7f87aa) then recovery.
- Recovery 0x7f87af-0x7f88ac: copy the saved Present back to [0xc9c040]; `device->Reset(&Present)`; `hr < 0` -> return 0
  (0x7f87d1). Else block R again (0x7f87dd-0x7f888c; its result is ignored, the restore callback still runs) and return 1.
  `[0xc97c18]` and the mode copy are NOT changed on this path.
- Success 0x7f88ad-0x7f8912: `[0xc97c18] = modeIndex`; copy mode: [0xc9bee4] = Width, [0xc9bee8] = Height, [0xc9beec] =
  RefreshRate, [0xc9bef0] = Format, [0xc9bef8] = flags. Depth [0xc9bef4] from Format via the jump table at 0x7f8a6c (byte index
  table 0x7f8a78, range check `Format-0x15 > 0xe` -> 0, 0x7f88ee): 0x15 A8R8G8B8, 0x16 X8R8G8B8, 0x23 A2R10G10B10 -> 0x20
  (0x7f8902); 0x17 R5G6B5, 0x18 X1R5G5B5, 0x19 A1R5G5B5 -> 0x10 (0x7f8909); anything else -> 0 (0x7f8910).
- Window fix-up, full screen (`[0xc9c060]` Windowed == 0, 0x7f891e): RECT r = {0, 0, mode.Width, mode.Height} (0x7f8932-0x7f893e);
  `AdjustWindowRectEx(&r, GetWindowLongA(hwnd, GWL_STYLE -0x10), GetMenu(hwnd) != 0, GetWindowLongA(hwnd, GWL_EXSTYLE -0x14))`
  (0x7f896a); `SetWindowPos(hwnd, NULL, r.left, r.bottom, r.right-r.left, r.bottom-r.top, 6 = SWP_NOMOVE|SWP_NOZORDER)`
  (0x7f8993; Y is r.bottom, harmless because of SWP_NOMOVE). Return 1.
- Windowed (0x7f89a6): `SetWindowLongA(hwnd, GWL_STYLE, 0x10cf0000)` (WS_VISIBLE|WS_OVERLAPPEDWINDOW, 0x7f89b4); r = {0, 0,
  Present.BackBufferWidth [0xc9c040], Present.BackBufferHeight [0xc9c044]}; same AdjustWindowRectEx; `SetWindowPos(..., 0x26 =
  SWP_NOMOVE|SWP_NOZORDER|SWP_FRAMECHANGED)` (0x7f8a1b-0x7f8a34); `GetClientRect(hwnd, &r)` (0x7f8a46); [0xc9bee4] = r.right,
  [0xc9bee8] = r.bottom (0x7f8a54, 0x7f8a59) (overrides the mode width/height). Return 1.
- librw: absent (librw has no runtime mode change; `restoreD3d9Device` d3d/d3ddevice.cpp is the closest Reset path and differs:
  no saved-Present fallback, no window resize).
- Open questions: meaning of `[0xc97bd4]` (third arg of 0x7f6cb0, 0x7f869e). `[0xc97c38]` name (0x7f8640).

## 0x7f8a90 RwD3D9ChangeMultiSamplingLevels  (catalogue: specified/REA, port level 3)
- `RwBool (RwUInt32 levels)` (arg [esp+0x4c]). Returns 1 except when the recovery Reset fails (0).
- Early return 1 if `[0xc97c38] == 0` (0x7f8a9e) or `levels == [0x8e2430]` (0x7f8ab0).
- 0x7f8ab6-0x7f8b45: save old Selected (ebx) and NonMask (ebp), save Present (0xe dwords). `n = min(levels, [0x8e242c])`
  (unsigned `jbe`, 0x7f8ad8); `[0x8e2430] = n`. If `n < 2` (unsigned, 0x7f8ae6): `[0x8e2430] = 1`, `[0x8e2438] = 1`. Else loop
  0x7f8aff: `IDirect3D9::CheckDeviceMultiSampleType(adapter [0xc97c24], devType [0x8e2428], fmt [0xc9bef0], windowed = ~[0xc9bef8]
  & 1, [0x8e2430], NULL)` (+0x2c); `hr >= 0` stops; else `[0x8e2430]--` and retry while it is `>= 2` (0x7f8b2a-0x7f8b33; it can
  end at 1 without a final check). Then `[0x8e2438] = ([0x8e2434] * n) / [0x8e242c]` (unsigned `imul`/`div`, 0x7f8b35-0x7f8b45; with
  the clamped requested n, not the found level). Same logic as `RwD3D9EngineSetMultiSamplingLevels` 0x7f84f0.
- 0x7f8b4a-0x7f8b81: `D3D9DeviceReleaseVideoMemory`; `TestCooperativeLevel` (+0xc, ignored); `D3D9SetPresentParameters(&[0xc9bee4]
  (current mode copy), [0xc9bef8], [0xc97bd4])` (0x7f8b6c); `hr = Reset(&Present)` (+0x40).
- `hr >= 0`: block R (0x7f8b8c-0x7f8c56); ok -> return 1 (0x7f8c5a); fail -> `D3D9DeviceReleaseVideoMemory` (0x7f8c60) then recovery.
  `hr < 0` -> recovery directly (0x7f8b86).
- Recovery 0x7f8c65-0x7f8d5c: restore Present from the copy, `[0x8e2430] = old Selected`, `[0x8e2438] = old NonMask`
  (0x7f8c7a, 0x7f8c80); `Reset(&Present)`; `< 0` -> return 0 (0x7f8c93). Else block R (result ignored) and return 1.
- librw: absent.
