# RwIm3D (RW core IMMEDIATEMODULE + D3D9 Im3D driver) - RE-SPEC-RW writer C

Immediate-mode 3D API of RW 3.6 in gta_sa.exe 1.0 US and its D3D9 back end. All `__cdecl`. The observable effect of
the D3D9 part (render-state calls, buffer locks and draw calls, in order) is what phase B must reproduce on wgpu.

## Layouts

IMMEDIATEMODULE globals: base = `RwEngineInstance [0xc97b24] + [0xc9c078]` (derived offset 0x4b8, size 0x74); a copy of
the base pointer is cached in [0xc97968] at open (0x7efe45). Open count [0xc9c07c].

| off | meaning | evidence |
|---|---|---|
| +0x00 | current transform pipeline | 0x7efad4, 0x7ef4f8 |
| +0x04 / +0x08 / +0x0c / +0x10 / +0x14 / +0x18 | current render pipeline for TRILIST / TRIFAN / TRISTRIP / LINELIST / POLYLINE / POINTLIST | jump table 0x7efd94 |
| +0x1c | default transform pipeline | 0x7efe58, 0x7efaf1 |
| +0x20 / +0x24 / +0x28 / +0x2c / +0x30 / +0x34 | default render pipeline, same type order as +0x04.. | jump table 0x7efdac |
| +0x38 | `rwIm3DPool`: u16 numElements (+0x38), elements ptr (+0x3c), i32 stride (+0x40) | 0x7ef498-0x7ef4b7 |
| +0x44 | `_rwIm3DPoolStash` (0x30 bytes): flags +0x44, ltm +0x48, ..., pipeline +0x64, primType +0x68, indices +0x6c, numIndices +0x70 | 0x7ef4cf/0x7ef4e5, 0x7ef577-0x7ef58e |

RW primitive types: 1 LINELIST, 2 POLYLINE, 3 TRILIST, 4 TRISTRIP, 5 TRIFAN, 6 POINTLIST. Im3D vertex
(`RwIm3DVertex`, stride 0x24): pos V3d +0x00, normal +0x0c, RGBA color +0x18, u +0x1c, v +0x20 (copy loops
0x80e4c0-0x80e4dd). Transform flags: 1 VERTEXUV, 2 ALLOPAQUE, 4 NOCLIP (uses at 0x80e28a, 0x80e2b8, 0x80e2ed).

D3D9 Im3D globals: 0xc9ab30 pool pointer handed to the render node; 0xc9ab34 base vertex index of the last dynamic VB
lock; 0xc9ab38 dynamic vertex buffer of the last lock; 0xc9ab3c running offset (in indices) into the Im3D index buffer;
0xc9ab40 `IDirect3DIndexBuffer9*` (created at 0x80e058); 0xc9ab44 / 0xc9ab48 / 0xc9ab4c vertex declarations (full / pos+color+uv / pos+color). Device
`_RwD3DDevice` [0xc97c28]. Driver caches: `_rwD3D9LastFVFUsed` 0x8e2440, `...LastVertexDeclarationUsed` 0x8e2444,
`...LastVertexShaderUsed` 0x8e2448, `...LastPixelShaderUsed` 0x8e244c, `...LastIndexBufferUsed` 0x8e2450.

RW -> D3D primitive table at [0x88513c] (u32[6]): {0, 2 LINELIST, 3 LINESTRIP, 4 TRIANGLELIST, 5 TRIANGLESTRIP,
6 TRIANGLEFAN}, indexed by RW primType (index 6 = POINTLIST reads past the table: the dword 0x23284040).

## 0x7ef450 RwIm3DTransform  (catalogue: specified/REA, port level 5)
- `void* (RwIm3DVertex* verts, RwUInt32 numVerts, RwMatrix* ltm, RwUInt32 flags)`.
- If `numVerts > 0x10000` (unsigned, 0x7ef457): `_rwerror(0x32)` + RwErrorSet({1, code}), return NULL. (Exactly
  0x10000 is accepted and is stored as u16 0.)
- `pool.numElements = (u16)numVerts`; `pool.elements = verts`; `pool.stride = 0x24`; `stash.ltm = ltm`;
  `stash.flags = flags | 0x18` (VERTEXXYZ | VERTEXRGBA forced); `RxPipelineExecute(module+0 (transform pipe),
  &pool (module+0x38), 1)` 0x805710; NULL -> return NULL; else return verts.

## 0x7ef520 RwIm3DEnd  (catalogue: specified/REA, port level 0)
- `RwBool ()`: if `pool.elements == NULL` return 0; else zero the 15 dwords module+0x38..+0x73 (pool and stash,
  `rep stosd`) and return 1.

## 0x7ef550 RwIm3DRenderIndexedPrimitive  (catalogue: specified/REA, port level 5)
- `RwBool (RwPrimitiveType primType, RwImVertexIndex* indices, RwInt32 numIndices)`.
- If `pool.elements == NULL`: `_rwerror(0x23)` + RwErrorSet, return 0.
- `stash.indices = indices; stash.pipeline = NULL; stash.primType = primType; stash.numIndices = numIndices`.
- By `primType - 1` (jump table 0x7ef690): LINELIST -> pipeline = module+0x10, `numIndices -= numIndices % 2`
  (signed remainder); POLYLINE -> +0x14; TRILIST -> +0x04, `numIndices -= numIndices % 3` (signed `idiv`);
  TRISTRIP -> +0x0c; TRIFAN -> +0x08. Any other value (incl. POINTLIST): `_rwerror(0x25, primType)` + RwErrorSet and
  execution continues with pipeline NULL.
- `RxPipelineExecute(stash.pipeline, &stash, 0)`; returns 1 if non-NULL else 0.

## 0x7ef6b0 RwIm3DRenderPrimitive  (catalogue: specified/REA, port level 5)
- `RwBool (RwPrimitiveType primType)`. `RxHeapGetGlobalHeap()` 0x8052f0 is called first (result unused).
- If `pool.elements == NULL`: error 0x23, return 0. Else `stash.pipeline = NULL; stash.primType = primType;
  stash.indices = NULL; stash.numIndices = pool.numElements` (u16 zero-extended), then the pipeline by type as above
  (table 0x7ef7f0; no count truncation), invalid type -> error 0x25 and continue with NULL;
  `RxPipelineExecute(pipeline, &stash, 0)`; return result != NULL.

## 0x7efac0 RwIm3DSetTransformPipeline  (catalogue: specified/REA, port level 0)
- `RxPipeline* (RxPipeline* p)`: if p != NULL: current = p; else current = default (+0x1c). Returns current.

## 0x7efb10 RwIm3DSetRenderPipeline  (catalogue: specified/REA, port level 1)
- `RxPipeline* (RxPipeline* p, RwPrimitiveType primType)`.
- p != NULL: by type (table 0x7efd94) store p in the current slot (+0x10 LINELIST, +0x14 POLYLINE, +0x04 TRILIST,
  +0x0c TRISTRIP, +0x08 TRIFAN, +0x18 POINTLIST); return p.
- p == NULL: restore the slot from its default (+0x20.. same order; table 0x7efdac) and return the new current value.
- Invalid type: `_rwerror(0x25, primType)` + RwErrorSet, return NULL.

## 0x7efe20 cb_IMMEDIATEMODULE_RwEngine_Open  (catalogue: specified/REA, port level 6)
- Open callback `(instance, offset, size)`: `[0xc9c078] = offset`; `[0xc9c07c]++`; `[0xc97968] = instance + offset`;
  zero 0x1d dwords at the module (0x74 bytes); `_rwIm3DCreateTransformPipeline(module+0x1c)` 0x80a0a0, then
  `_rwIm3DCreateRenderPipelines(module+0x20)` 0x80a1a0. Both succeed -> return instance. Either fails ->
  `_rwIm3DDestroyRenderPipelines(module+0x20)` 0x80a140, `_rwIm3DDestroyTransformPipeline(module+0x1c)` 0x80a110,
  `[0xc9c07c]--`, return NULL.

## 0x80a0a0 _rwIm3DCreateTransformPipeline  (catalogue: specified/REA, port level 5)
- `RwBool (RxPipeline** out)`: `p = RxPipelineCreate()` 0x8057b0; NULL -> return 0. `p->pluginId (+0x2c) = 1`;
  `lp = RxPipelineLock(p)` 0x806990; NULL -> `_rxPipelineDestroy(p)` 0x805820, return 0.
  `p2 = RxLockedPipeUnlock(RxLockedPipeAddFragment(lp, NULL, RxNodeDefinitionGetD3D9ImmInstance(), NULL))`
  (0x806be0 with a NULL-terminated node list; 0x805f40). p2 == NULL -> `_rxPipelineDestroy(NULL)` (the
  register holding p was overwritten; original bug: the pipeline leaks), return 0. Else `*out = p2`,
  `RwIm3DSetTransformPipeline(p2)`, return 1.

## 0x80a1a0 _rwIm3DCreateRenderPipelines  (catalogue: specified/REA, port level 5)
- `RwBool (RxPipeline** defaults)`: as 0x80a0a0 with node `RxNodeDefinitionGetD3D9SubmitNoLight()` 0x80e9a0 (same
  failure paths). On success `defaults[0..4] = p` (TRILIST, TRIFAN, TRISTRIP, LINELIST, POLYLINE; the POINTLIST
  default +0x34 stays NULL), then `RwIm3DSetRenderPipeline(p, 3)`, `(p, 5)`, `(p, 4)`, `(p, 1)`, `(p, 2)` in that order,
  then TAIL-JUMPS to `_rwD3D9Im3DRenderOpen` 0x80e020 and returns its result.

## 0x80dfb0 _rwD3D9Im3DRenderClose  (catalogue: specified/REA, port level 1)
- `void ()`: if [0xc9ab40] != NULL: `Release()` (vtbl+8), clear. Then for 0xc9ab4c, 0xc9ab48, 0xc9ab44 in that order:
  if non-NULL `RwD3D9DeleteVertexDeclaration(decl)` 0x7fac10, clear.

## 0x80e020 _rwD3D9Im3DRenderOpen  (catalogue: specified/REA, port level 1)
- `RwBool ()`. `[0xc9ab3c] = 0`.
- `device->CreateIndexBuffer(0x1fffe, 0x208 (WRITEONLY|DYNAMIC), 0x65 (D3DFMT_INDEX16), pool, &[0xc9ab40], NULL)`
  (vtbl+0x6c, 0x80e058); `pool = ((~[0xc9bf1c]) >> 15) & 2`, i.e. D3DPOOL_DEFAULT (0) when bit 0x10000 of [0xc9bf1c]
  (the D3D caps DevCaps word, D3DDEVCAPS_HWTRANSFORMANDLIGHT) is set, else D3DPOOL_SYSTEMMEM (2).
  hr != 0 -> run the close sequence of 0x80dfb0 inline and return 0.
- Vertex declarations via `RwD3D9CreateVertexDeclaration(elements, &out)` 0x7faa30 (results ignored), each element
  `{stream u16, offset u16, type u8, method u8, usage u8, usageIndex u8}` ending with `{0xff, 0, 0x11 UNUSED, 0, 0, 0}`:
  - decl [0xc9ab44]: (0, 0, FLOAT3, POSITION), (0, 0xc, FLOAT3, NORMAL), (0, 0x18, D3DCOLOR, COLOR), (0, 0x1c, FLOAT2, TEXCOORD).
  - decl [0xc9ab48]: (0, 0, FLOAT3, POSITION), (0, 0xc, D3DCOLOR, COLOR), (0, 0x10, FLOAT2, TEXCOORD).
  - decl [0xc9ab4c]: (0, 0, FLOAT3, POSITION), (0, 0xc, D3DCOLOR, COLOR).
  Return 1.
- Note: the method at vtbl+0x6c of IDirect3DDevice9 is CreateIndexBuffer (slot 27); the rw-names entry says nothing
  about it; the buffer is used through `IDirect3DIndexBuffer9::Lock/Unlock` (+0x2c/+0x30) in 0x80e270.

## 0x80e270 _rwD3D9ImmRenderPrimitiveNode  (catalogue: specified/REA, port level 2)
- Node body `RwBool (RxPipelineNode* self, const RxPipelineNodeParam* params)`; params unused; always returns 1.
  Reads `pool = [0xc9ab30]`, `verts = pool->elements`, `numVerts = pool->numElements`, `stash` (pool+0xc).
- Steps (observable D3D state, in order):
  1. If `stash.flags & 1` (UV): decl = [0xc9ab48], stride 0x18; else decl = [0xc9ab4c], stride 0x10.
  2. `_rwD3D9RenderStateVertexAlphaEnable(flags & 2 ? 0 : 1)` 0x7fe0a0; `RwD3D9SetTransformWorld(stash.ltm)` 0x7fa520;
     `RwD3D9SetRenderState(0x89 LIGHTING, 0)`, `(0x8f NORMALIZENORMALS, 0)`, `(0x88 CLIPPING, flags & 4 ? 0 : 1)` 0x7fc2d0.
  3. `_rwD3D9GetCurrentTexture(0)` 0x7fde50 == NULL: `RwD3D9SetTextureStageState(0, 1 COLOROP, 3 SELECTARG2)`,
     `(0, 3 COLORARG2, 0 DIFFUSE)`, `(0, 4 ALPHAOP, 3 SELECTARG2)`, `(0, 6 ALPHAARG2, 0 DIFFUSE)`.
     Else: `(0, COLOROP, 4 MODULATE)`, `(0, 2 COLORARG1, 2 TEXTURE)`, `(0, COLORARG2, DIFFUSE)`, `(0, ALPHAOP, MODULATE)`,
     `(0, 5 ALPHAARG1, TEXTURE)`, `(0, ALPHAARG2, DIFFUSE)`.
  4. Caches: if [0x8e2444] != decl: store, [0x8e2440] = -1, `SetVertexDeclaration(decl)` (vtbl+0x15c). If [0x8e2448] != 0:
     store 0, `SetVertexShader(NULL)` (+0x170). If [0x8e244c] != 0: store 0, `SetPixelShader(NULL)` (+0x1ac).
  5. `d3dPrim = [0x88513c][stash.primType]`. If numVerts == 0 -> return 1.
  6. primCount from a count `n` by d3dPrim: LINELIST `n >> 1`; LINESTRIP `n - 1`; TRIANGLELIST `n / 3` (unsigned);
     TRIANGLESTRIP/FAN `n - 2`; other values leave primCount uninitialised (stack garbage).
  7. Indexed (stash.indices != NULL and stash.numIndices != 0; n = numIndices):
     a. If `numIndices <= numVerts && numIndices <= 8`: `RwD3D9DynamicVertexBufferLock(stride, numIndices,
        &[0xc9ab38], &ptr, &[0xc9ab34])` 0x7f5b10; failure -> fallback (8). De-index: for each index i (u16) copy
        `verts[i]` pos (3 dwords), color (+0x18) and, with UV, u/v (+0x1c/+0x20) into the locked buffer.
        `RwD3D9DynamicVertexBufferUnlock([0xc9ab38])` 0x7f5c90; `RwD3D9SetStreamSource(0, [0xc9ab38], 0, stride)` 0x7fa030;
        `_rwD3D9RenderStateFlushCache()` 0x7fc200; `DrawPrimitive(d3dPrim, [0xc9ab34], primCount)` (vtbl+0x144);
        hr < 0 -> fallback, else return 1.
     b. Otherwise: if [0xc9ab40] == NULL or numIndices > 0xffff -> fallback. Lock the dynamic VB for numVerts vertices
        (same call), copy every vertex (pos, color[, uv]), unlock. Index buffer: if `[0xc9ab3c] + numIndices > 0xffff`:
        `[0xc9ab3c] = 0`, `IB->Lock(0, numIndices*2, &p, 0x2800 DISCARD|NOSYSLOCK)`; else
        `IB->Lock([0xc9ab3c]*2, numIndices*2, &p, 0x1800 NOOVERWRITE|NOSYSLOCK)` (+0x2c). hr < 0 -> fallback.
        Copy numIndices*2 bytes; `IB->Unlock()` (+0x30); `RwD3D9SetStreamSource(0, [0xc9ab38], 0, stride)`; if
        [0x8e2450] != IB: store, `SetIndices(IB)` (+0x1a0); flush cache;
        `DrawIndexedPrimitive(d3dPrim, [0xc9ab34], 0, numVerts, [0xc9ab3c], primCount)` (+0x148);
        `[0xc9ab3c] += numIndices` (also on failure); hr >= 0 -> return 1, else fallback.
     Fallback (indexed): decl = [0xc9ab44] through the cache (step 4 form); `RwD3D9SetStreamSource(0, NULL, 0, 0)`; if
     [0x8e2450] != 0: store 0, `SetIndices(NULL)`; flush; `DrawIndexedPrimitiveUP(d3dPrim, 0, numVerts, primCount,
     indices, 0x65 INDEX16, verts, 0x24)` (+0x150); return 1.
  8. Non-indexed (n = numVerts): lock the dynamic VB for numVerts, copy all vertices, unlock, SetStreamSource, flush,
     `DrawPrimitive(d3dPrim, [0xc9ab34], primCount)`; hr < 0 or lock failure -> decl [0xc9ab44], SetStreamSource(0, NULL,
     0, 0), flush (no SetIndices), `DrawPrimitiveUP(d3dPrim, primCount, verts, 0x24)` (+0x14c). Return 1.
- Notes: the normal is never sent except in the UP fallbacks (full declaration); lighting is forced off anyway.
- librw: `d3d9/d3d9render.cpp` im3DRenderIndexedPrimitive uses a simpler path (no de-indexing for <= 8 indices, no UP
  fallback); port from the bytes.

## 0x80e970 _rwD3D9ImmInstanceNode  (catalogue: specified/REA, port level 0)
- Node body `(self, params)`: `[0xc9ab30] = params->dataParam` (first dword of the param block, i.e. the pool pointer
  passed to RxPipelineExecute by RwIm3DTransform); returns 1. No transformation is done on the CPU.
