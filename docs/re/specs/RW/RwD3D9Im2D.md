# RW D3D9 driver: Im2D (RE-SPEC-RW writer C)

D3D9 back end of the RW immediate-mode 2D API (RwIm2DRenderLine/Primitive/IndexedPrimitive reach these through the
RwDevice Im2D slots, README §1.4). All `__cdecl`. Observable D3D behaviour is specified for the wgpu port.

## Globals

0xc980e0 "own Im2D vertex buffer" flag; 0xc980e4 current vertex offset (in vertices) in the Im2D VB; 0xc980e8
`IDirect3DVertexBuffer9*` used as stream 0 (own buffer, or the dynamic buffer returned by the last
RwD3D9DynamicVertexBufferLock); 0xc980ec current index offset; 0xc980f0 `IDirect3DIndexBuffer9*`; 0xc980f4 vertex
declaration. `RwIm2DVertex` = 0x1c bytes: x +0, y +4, z +8, rhw +0xc, D3DCOLOR +0x10, u +0x14, v +0x18 (copy loops in
0x7fbb00). RW -> D3D primitive table at [0x884888]: {0, 2, 3, 4, 5, 6, 1} (index 6 POINTLIST -> D3DPT_POINTLIST 1).
The current camera pointer is read from [0xc9bcc0] (its +0x60 is the frame-buffer raster; matches RwCamera.frameBuffer);
the raster's `nOffsetX/nOffsetY` (+0x1c/+0x1e, i16) are added to every x/y (sub-raster rendering).

## 0x7fb480 _rwD3D9Im2DRenderOpen  (catalogue: specified/REA, port level 1)
- `RwBool ()`. `[0xc980e0] = 0`.
- If DevCaps [0xc9bf1c] lacks 0x10000 (no hardware T&L) AND `device->GetAvailableTextureMem()` (vtbl+0x10) > 0x800000
  (unsigned): `device->CreateVertexBuffer(0x40000, 0x228 (WRITEONLY|DONOTCLIP|DYNAMIC), FVF 0, D3DPOOL_DEFAULT,
  &[0xc980e8], NULL)` (+0x68); success -> [0xc980e0] = 1, [0xc980e4] = 0.
- `[0xc980ec] = 0`; `device->CreateIndexBuffer(0x4e20, 0x228, 0x65 INDEX16, D3DPOOL_DEFAULT, &[0xc980f0], NULL)` (+0x6c).
  Failure -> delete the declaration if any, release the own VB if [0xc980e0] (vtbl+8), release the IB if any;
  return 0.
- Declaration -> [0xc980f4] via `RwD3D9CreateVertexDeclaration` 0x7faa30: (0, 0, FLOAT4 3, POSITIONT 9), (0, 0x10,
  D3DCOLOR 4, COLOR 0xa), (0, 0x14, FLOAT2 1, TEXCOORD 5), END. Return 1 (declaration result ignored).

## 0x7fb5f0 _rwD3D9Im2DRenderClose  (catalogue: specified/REA, port level 1)
- `void ()`: delete declaration [0xc980f4] (0x7fac10) and clear; if own VB flag: release [0xc980e8], clear; release IB
  [0xc980f0], clear.

## 0x7fb650 _rwD3D9Im2DRenderLine  (catalogue: specified/REA, port level 2)
- `RwBool (RwIm2DVertex* verts, RwInt32 numVerts /*unused*/, RwInt32 v1, RwInt32 v2)`.
- `r = camera->frameBuffer` (no NULL check). Get space for 2 vertices: own VB -> if `[0xc980e4] + 2 < 0x2493` (unsigned):
  `VB->Lock([0xc980e4]*0x1c, 0x38, &p, 0x1800 NOOVERWRITE|NOSYSLOCK)`, else `[0xc980e4] = 0`,
  `VB->Lock(0, 0x38, &p, 0x2800 DISCARD|NOSYSLOCK)` (+0x2c); no own VB -> `RwD3D9DynamicVertexBufferLock(0x1c, 2,
  &[0xc980e8], &p, &[0xc980e4])` 0x7f5b10. p == NULL -> return 0.
- Write `verts[v1]` then `verts[v2]` with `x = (float)offX + x`, `y = (float)offY + y` (always, even for 0 offsets),
  other 5 dwords copied. `VB->Unlock()` (+0x30); `_rwD3D9Im2DSetRenderStates()` 0x7fb810;
  `DrawPrimitive(D3DPT_LINELIST 2, [0xc980e4], 1)` (+0x144); `[0xc980e4] += 2`; return `hr >= 0`.

## 0x7fb810 _rwD3D9Im2DSetRenderStates  (catalogue: unknown/REA, port level 1)
- `void ()`: `_rwD3D9SetVertexDeclaration([0xc980f4])` 0x7f9f70; `_rwD3D9SetVertexShader(NULL)` 0x7f9fb0;
  `_rwD3D9SetPixelShader(NULL)` 0x7f9ff0; `RwD3D9SetRenderState(0x88 CLIPPING, 0)`, `(0x89 LIGHTING, 0)`; stage-0 TSS
  as in the Im3D node (no texture: COLOROP SELECTARG2 / COLORARG2 DIFFUSE / ALPHAOP SELECTARG2 / ALPHAARG2 DIFFUSE;
  texture: MODULATE TEXTURE*DIFFUSE for color and alpha), chosen by `_rwD3D9GetCurrentTexture(0)`;
  `RwD3D9SetStreamSource(0, [0xc980e8], 0, 0x1c)` 0x7fa030; tail-jump `_rwD3D9RenderStateFlushCache()`.

## 0x7fbb00 _rwD3D9Im2DRenderPrimitive  (catalogue: specified/REA, port level 2)
- `RwBool (RwPrimitiveType type, RwIm2DVertex* verts, RwInt32 n)`.
- Raster: camera [0xc9bcc0] != NULL ? camera->frameBuffer : `RwRasterGetCurrentContext()` 0x7fae60.
- primCount: 1 LINELIST `n / 2` (signed), 2 POLYLINE `n - 1`, 3 TRILIST `n / 3` (signed), 4/5 `n - 2`, else 0.
- Space for n vertices as in 0x7fb650 (`[0xc980e4] + n < 0x2493` test, lock size n*0x1c); p == NULL -> return 0.
- If offX == 0 and offY == 0: copy n*0x1c bytes verbatim; else per vertex add the offsets to x/y as above.
- `VB->Unlock()`; `_rwD3D9Im2DSetRenderStates()`; `DrawPrimitive([0x884888][type], [0xc980e4], primCount)`;
  `[0xc980e4] += n`; return `hr >= 0`.
- Confidence medium (structure from the decompile; offsets and lock flags checked against the disassembly).

## 0x7fbd30 _rwD3D9Im2DRenderIndexedPrimitive  (catalogue: specified/REA, port level 2)
- `RwBool (RwPrimitiveType type, RwIm2DVertex* verts, RwInt32 numVerts, RwImVertexIndex* indices, RwInt32 numIndices)`.
- Raster = camera->frameBuffer (no NULL fallback). primCount from numIndices as in 0x7fbb00.
- If `numVerts < numIndices` (signed): upload numVerts vertices (same lock/copy/offset rules); then if [0xc980f0] != NULL
  and numIndices < 0x2711: lock the IB (`[0xc980ec] + numIndices < 0x2711` -> offset lock with 0x1800, else offset = 0 and
  0x2800; size numIndices*2), copy the indices, unlock, `_rwD3D9Im2DSetRenderStates()`, `_rwD3D9SetIndices(IB)` 0x7fa1c0,
  `DrawIndexedPrimitive([0x884888][type], [0xc980e4], 0, numVerts, [0xc980ec], primCount)` (+0x148),
  `[0xc980ec] += numIndices`; then `[0xc980e4] += numVerts`; return `hr >= 0`.
- Else: lock space for numIndices vertices and de-index (`verts[indices[i]]`, 7 dwords each, offsets applied), unlock,
  set states, `DrawPrimitive(type, [0xc980e4], primCount)`, `[0xc980e4] += numIndices`.
- Confidence low: Ghidra loses the stack in this function (0x7fbd30-0x7fc1f0); the order above is from the decompile
  and must be re-read from the bytes when porting. Open question: what is returned when the IB lock fails
  (0x7fbd30.., the decompile shows the VB offset still advanced).
