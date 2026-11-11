# RW D3D9 driver: render-state getters, blend states, Im2D triangle (RE-SPEC-RW-3 writer B)

All `__cdecl`. Pass-1 file `RwD3D9RenderState.md` (read-only) has the cache layout: wanted render state `rs[s] = {value, queued}`
at 0xc991d0 (pairs, stride 8), render-state dirty list `[0xc98b40 + 4*i]` with count `[0xc9a5ec]`, texture-stage state at 0xc980f8
(index `stage*33 + type`, stride 8), sampler state `u32 [0xc98938 + 4*(stage*14 + type)]`, RW blend mirrors `[0xc9a5c8]` SRCBLEND /
`[0xc9a5cc]` DESTBLEND, blend map table .rdata 0x88490c (identity 0..11).

## Layout added here (Im2D vertex buffer)
- `[0xc980e0]` Im2D uses a private vertex buffer (non-zero) or the shared dynamic buffers (0) (0x7fb8f9); `[0xc980e8]`
  IDirect3DVertexBuffer9* (0x7fb90d); `[0xc980e4]` next free vertex index (0x7fb8fd, updated 0x7fbade). Capacity limit 0x2492 vertices
  (0x7fb905). Vertex = RwIm2DVertex 0x1c bytes {x, y, z, rhw, color, u, v} (stride via `eax*8 - eax` then `*4`, 0x7fb9a2).
- `[0xc9bcc0]` camera being rendered (pass-1 "dgGGlobals"); its +0x60 frame buffer raster (0x7fb8f1).
- IDirect3DVertexBuffer9 vtbl +0x2c Lock, +0x30 Unlock; IDirect3DDevice9 +0x144 DrawPrimitive, +0xdc SetClipPlane.

## 0x7fb8e0 _rwD3D9Im2DRenderTriangle  (catalogue: specified/REA, port level 2)
- `RwBool (RwIm2DVertex* v, RwInt32 numVerts, RwInt32 i1, RwInt32 i2, RwInt32 i3)` (device slot fpIm2DRenderTriangle). `r =
  camera->frameBuffer`; `ptr = NULL`.
- Lock (0x7fb8f9-0x7fb973): private buffer: if `[0xc980e4] + 3 > 0x2492` (unsigned): `[0xc980e4] = 0`, `vb->Lock(0, 0x54, &ptr, 0x2800)`
  (DISCARD|NOSYSLOCK); else `vb->Lock([0xc980e4] * 0x1c, 0x54, &ptr, 0x1800)` (NOOVERWRITE|NOSYSLOCK). Shared: `RwD3D9DynamicVertexBufferLock(
  0x1c, 3, &[0xc980e8], &ptr, &[0xc980e4])` 0x7f5b10. Lock results are not checked; `ptr == NULL` -> return 0 (0x7fb97c).
- Copy (x87): `ox = (f)(i16)r+0x1c`, `oy = (f)(i16)r+0x1e` (`movsx` + `fild`). For k in (i1, i2, i3) in that order: out.x = ox + v[k].x,
  out.y = oy + v[k].y (f32 stores), z, rhw, color, u, v copied as dwords (0x7fb982-0x7fbaa2).
- `vb->Unlock()` (+0x30); `0x7fb810()` (Im2D state set-up, not in this file); `hr = device->DrawPrimitive(4 TRIANGLELIST, [0xc980e4], 1)`
  (+0x144, 0x7fbac8); `[0xc980e4] += 3`; returns `hr >= 0`.
- librw: d3d/d3dimmed.cpp `im2DRenderTriangle` differs: librw uses DrawIndexedPrimitiveUP style buffers and does not add the raster
  offset.

## 0x7fc3a0 RwD3D9GetTextureStageState  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 stage, RwUInt32 type, void* out)`: `*(u32*)out = [0xc980f8 + 8*(stage*33 + type)]` (`shl 5` + add stage,
  0x7fc3aa-0x7fc3b5): the wanted (queued) value, not the device value.
- librw: d3d/d3ddevice.cpp `getTextureStageState` matches in effect.

## 0x7fc400 RwD3D9GetSamplerState  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 stage, RwUInt32 type, void* out)`: `*(u32*)out = [0xc98938 + 4*(stage*14 + type)]` (`eax*8 - eax` then `*2`,
  0x7fc408-0x7fc418).
- librw: d3d/d3ddevice.cpp `getSamplerState` matches in effect.

## 0x7fc4a0 RwD3D9SetClipPlane  (catalogue: specified/REA, port level 0)
- `RwBool (RwUInt32 index, const RwV4d* plane)`: `device->SetClipPlane(index, plane)` (+0xdc, immediate, no cache); returns `hr >= 0`.
- librw: absent.

## 0x7fdc30 _rwD3D9RenderStateSrcBlend  (catalogue: specified/REA, port level 0)
- `RwBool (RwBlendFunction b)`: if `b != [0xc9a5c8]`: `rs[0x13 D3DRS_SRCBLEND].value = [0x88490c + 4*b]` (0xc99268); if not queued
  (`[0xc9926c] == 0`): queued = 1, `rsList[[0xc9a5ec]++] = 0x13` (0x7fdc61); `[0xc9a5c8] = b`. Returns 1. No range check of b.
- librw: d3d/d3ddevice.cpp `setRwRenderState(SRCBLEND)` matches in effect (librw compares the cached RW value too).

## 0x7fdc80 _rwD3D9RenderStateDestBlend  (catalogue: specified/REA, port level 0)
- Same with `[0xc9a5cc]`, `rs[0x14 D3DRS_DESTBLEND]` (value 0xc99270, queued 0xc99274), list entry 0x14 (0x7fdcb1). Returns 1.
- librw: d3d/d3ddevice.cpp `setRwRenderState(DESTBLEND)` matches in effect.
