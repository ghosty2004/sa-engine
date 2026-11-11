# RW D3D9 driver: instancing, vertex declarations, atomic all-in-one node (RE-SPEC-RW-2 writer B)

The D3D9 instance pipeline of RW 3.6 in gta_sa.exe 1.0 US: `RpAtomicRender` -> pipeline -> node body
`_rwD3D9AtomicAllInOneNode` 0x7575f0, which (re)instances the geometry into a `RwResEntry` holding an
`RxD3D9ResEntryHeader` plus one `RxD3D9InstanceData` per mesh, then calls the node's lighting and render
callbacks. All functions are `__cdecl` unless stated otherwise. Device = `_RwD3DDevice` [0xc97c28]
(IDirect3DDevice9*), engine = `RwEngineInstance` [0xc97b24] (RwGlobals*).

## Layouts and globals used in this file (each offset proven at the cited instruction)

RwResEntry (0x18) + RxD3D9ResEntryHeader at resEntry+0x18 (size 0x40) + numMeshes x RxD3D9InstanceData (0x24):

| struct | off | field | evidence |
|---|---|---|---|
| RwResEntry | +0x0/+0x4 | link next/prev | 0x7569ad/0x7569af (zeroed), 0x7576b8-0x757718 (unlink/relink) |
| | +0x8 | size (bytes after the 0x18 header) | 0x7569b2 |
| | +0xc / +0x10 | owner / ownerRef | 0x7569b5, 0x7569b8 |
| | +0x14 | destroyNotify (0x4c9990) | 0x7569bb |
| header (res+0x18) | +0x0 | serialNumber (u32 copy of meshHeader->serialNum) | 0x756a06; compared at 0x75764c (`[resEntry+0x18]`) |
| | +0x4 | numMeshes | 0x756a0c |
| | +0x8 | indexBuffer (IDirect3DIndexBuffer9*) | 0x756a13, 0x756aaa |
| | +0xc | primType (D3DPRIMITIVETYPE) | 0x756ad4, 0x756b02 |
| | +0x10 + 0x10*s | vertexStream[2]: +0 vertexBuffer, +4 offset, +8 stride, +0xc u16 geometryFlags, +0xe u8 managed, +0xf u8 dynamicLock | 0x756b0d-0x756b1d, 0x757918-0x757969 |
| | +0x30 | useOffsets | 0x757c98, 0x757cbe |
| | +0x34 | vertexDeclaration (IDirect3DVertexDeclaration9*) | 0x757bf4, 0x757ce0 |
| | +0x38 | totalNumIndex | 0x756a15, 0x756a3a |
| | +0x3c | totalNumVertex | 0x757905 |
| instance (header+0x40+0x24*i) | +0x0 | numIndex | 0x756c46, 0x756c8f |
| | +0x4 | minVert | 0x756b6d, 0x756bb1 |
| | +0x8 | material (RpMaterial*) | 0x756bc4 |
| | +0xc | vertexAlpha (RwBool) | 0x756bc7, 0x7580b3 |
| | +0x10 | vertexShader | 0x756bc1, 0x756fef |
| | +0x14 | baseIndex | 0x756cdc, 0x757ca8 |
| | +0x18 | numVertices | 0x756b6a, 0x756bae |
| | +0x1c | startIndex | 0x756bd8, 0x756c8c |
| | +0x20 | numPrimitives | 0x756cac.. 0x756ccd |

Node private data (`RxPipelineNode+0x14` -> 4 callbacks, 0x757394): +0x0 instance, +0x4 reinstance, +0x8 lighting,
+0xc render (init 0x757897-0x7578ab).

Other fields: RpMeshHeader +0x0 flags (0x100 = UNINDEXED, low byte = primitive kind), +0x4 u16 numMeshes, +0x6 u16
serialNum, meshes at +0x10 (RpMesh 0xc: +0 indices, +4 numIndices, +8 material) (0x756a1f, 0x756a32, 0x756b3e);
RpGeometry +0x8 flags, +0xc u16 lockedSinceLastInst, +0x10 numTriangles, +0x14 numVertices, +0x18 numMorphTargets,
+0x1c numTexCoordSets, +0x30 preLitLum, +0x34 texCoords[8], +0x54 mesh, +0x58 repEntry, +0x5c morphTargets (stride
0x1c: +0x14 verts, +0x18 normals) (0x7578da, 0x757675, 0x756a91, 0x757607, 0x757632, 0x7578e6, 0x758055, 0x758106,
0x757612, 0x75763f, 0x757e75, 0x757f2a); RpAtomic +0x4 frame, +0x14 repEntry, +0x18 geometry, +0x4c interpolator
flags, +0x50 i16 startMorphTarget, +0x52 i16 endMorphTarget, +0x58 recipTime, +0x5c position, +0x64
llWorldSectorsInAtomic, +0x6c pipeline (0x757772, 0x75763a, 0x757604, 0x7576aa, 0x757e17, 0x757e1b, 0x757e33,
0x7574ee, 0x75799c); RxPipeline +0x2c pluginId (0x7579a3); RpWorldSector +0x84 u16 numTriangles (0x756a7c), sector
bounding box at +0x60 used for the clip test (0x756e41).

Globals: [0xc92520] dynamic-geometry list head (singly linked, node {geometry, next}); [0xc92524] its RwFreeList*;
[0xc9b8c0] offset of the MESHMODULE data in the engine (byte table `meshFlagsToPrimType` at +0x8, RpMesh.md);
[0xc9bc58] offset of the RESOURCES module data in the engine (+0x24 = pointer to the used-resource list head,
0x7576cf); [0xc926e8] RpGeometry plugin offset of the D3D9 usage flags (u32, 0x7588b4); D3DCAPS9 copy at 0xc9bf00:
[0xc9bf1c] DevCaps (0x10000 HWTRANSFORMANDLIGHT), [0xc9bfd4] DevCaps2 (1 STREAMOFFSET), [0xc9bfec] DeclTypes
(4 SHORT2N, 0x10 USHORT2N, 0x80 DEC3N). Driver caches [0x8e2440] last FVF, [0x8e2444] last vertex declaration,
[0x8e2448] last vertex shader, [0x8e244c] last pixel shader, [0x8e2450] last index buffer.

Data tables: `[0x874ef8]` u32[17] D3DDECLTYPE size: `[0x874ef8] = 0x4` FLOAT1, 8, 0xc, 0x10, 4 (D3DCOLOR), 4, 4, 8, 4,
4, 8, 4, 8, 4, 4, 4, 8 (types 0..0x10); index 0x11 reads the next constant (`[0x874f3c] = 511.0f`) - see InstV3d.
`[0x874fec]` u32[7] RW primitive type -> D3DPRIMITIVETYPE: {0, 2 LINELIST, 3 LINESTRIP, 4 TRIANGLELIST, 5 TRIANGLESTRIP,
6 TRIANGLEFAN, 1 POINTLIST} (0x756afb).

Float constants used by the converters: `[0x859aac] = 255.0f`, `[0x86b2c0] = 32767.0f`, `[0x8729e8] = 65535.0f`,
`[0x874f3c] = 511.0f`, `[0x874f40] = 0.0019569471f` (1/511), `[0x858624] = 1.0f`, `[0x858b50] = 0.0f`,
`[0x858c1c] = -1.0f`.

Conversion convention of all `Inst*` converters below: "rnd(v)" = the product is computed in x87, stored to an f32
temporary (rounding to f32), reloaded and converted with `fistp` to an int32 using the CURRENT rounding mode
(round-to-nearest-even in game), then truncated to the destination width (low byte / low word). "fist(v)" = the
source f32 is converted the same way without scaling. No clamping anywhere. Every converter returns
`[0x874ef8 + 4*type]` (the element size), also when `n <= 0` or the type is out of range (no write in that case).

## 0x74e970 _rpD3D9AddDynamicGeometry  (catalogue: specified/REA, port level 2)
- `void (RpGeometry* g)` ([esp+4] -> esi 0x74e976). Callers: instance callback 0x7579d1, skin 0x7c9680.
- Walk the list [0xc92520] (node +0 geometry, +4 next, 0x74e97e/0x74e982): if a node holds `g` -> return (no dup).
- If the freelist [0xc92524] is NULL: create it `_rwFreeListCreate(8, 0x80, 4, 0x3050d)` (0x74e992-0x74e9a8; args
  entrySize 8, entriesPerBlock 0x80, alignment 4, hint 0x3050d).
- `node = engine->memoryAlloc(freelist, 0x3050d)` (engine+0x144, 0x74e9b8); `node->geometry = g`,
  `node->next = [0xc92520]`, `[0xc92520] = node` (push front, 0x74e9be-0x74e9cc).
- librw: absent (no dynamic-geometry list).

## 0x74e9e0 _rpD3D9RemoveDynamicGeometry  (catalogue: specified/REA, port level 0)
- `void (RpGeometry* g)`; caller `_rxD3D9DefaultDestroyInstanceCallback` 0x4c9990.
- Linear search with predecessor (0x74e9f3-0x74e9fe); not found -> return. Found: if it is the head,
  `[0xc92520] = node->next` (0x74ea09) else `prev->next = node->next` (0x74ea14); `node->geometry = 0` (0x74ea17);
  `engine->memoryFree([0xc92524], node)` (engine+0x148, 0x74ea2a). The freelist is never destroyed here.
- librw: absent.

## 0x7522e0 _rpD3D9VertexDeclarationGetSize  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RwUInt32 d3dDeclType)`: return `[0x874ef8 + 4*type]` (0x7522e4); no range check.
- librw: absent.

## 0x752320 _rpD3D9VertexDeclarationInstWeights  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RwUInt32 type, RwUInt8* dst, const RwMatrixWeights* src /*4 f32, stride 0x10*/, RwInt32 n,
  RwUInt32 stride)`; args at [esp+0x24..0x34] after the 0x10-byte frame and 4 pushes (0x752320-0x75232d).
  Caller: skin instancing 0x7c90d0. `type > 0xe` -> return size only (0x75232e). Jump table 0x752a94.
- Per vertex (w0..w3 = src[0..3]), dst advanced by `stride`, src by 0x10:
  - 0 FLOAT1: dst f32[0] = w0 (0x752353). 1 FLOAT2: w0, w1 (0x75238a).
  - 2 FLOAT3: copies FOUR dwords w0..w3 (0x7523d0-0x7523e4), i.e. writes 16 bytes into a 12-byte element
    (original bug; the 4th dword overwrites the next 4 bytes of the vertex). 3 FLOAT4: w0..w3 (0x752417-0x75242d).
  - 4 D3DCOLOR (0x752448): if `w0 < 1.0f` is false (`fcomp [0x858624]`, `and 0x100` = C0; NaN takes the convert
    path, 0x752469-0x752476): bytes {0, 0, 0xff, 0} (dst[2] = 0xff, 0x752478). Else dst[1] = rnd(w1*255),
    dst[0] = rnd(w2*255), dst[3] = rnd(w3*255) (`[0x859aac] = 255.0f`), dst[2] = u8(0xff - dst[3] - dst[1] - dst[0])
    (0x7524d5-0x7524e5, 8-bit wraparound). So R carries w0 = remainder.
  - 5 UBYTE4: dst[i] = u8(fist(wi)) for i=0..3 (0x752524-0x75257a).
  - 6 SHORT2: u16 fist(w0), fist(w1) (0x7525ad). 7 SHORT4: u16 fist(w0..w3) (0x752611).
  - 8 UBYTE4N (0x75268a): same test on w0: `w0 >= 1.0f` -> {0xff, 0, 0, 0} (0x7526bc); else dst[1] = rnd(w1*255),
    dst[2] = rnd(w2*255), dst[3] = rnd(w3*255), dst[0] = u8(0xff - dst[2] - dst[3] - dst[1]) (0x752719-0x752727).
  - 9 SHORT2N: `a = rnd(w0*32767)` (`[0x86b2c0] = 32767.0f`); u16[0] = a, u16[1] = 0x7fff - a (0x752777-0x752785).
  - 0xa SHORT4N: a = rnd(w1*32767), b = rnd(w2*32767), c = rnd(w3*32767); u16[1] = a, u16[2] = b, u16[3] = c,
    u16[0] = 0x7fff - c - b - a (0x752808-0x752822).
  - 0xb USHORT2N / 0xc USHORT4N: as 9 / 0xa with `[0x8729e8] = 65535.0f` and 0xffff (0x752859, 0x7528b4).
  - 0xd UDEC3: `(fist(w2)&0x3ff)<<20 | (fist(w1)&0x3ff)<<10 | fist(w0)&0x3ff` (0x75297b-0x7529a4).
  - 0xe DEC3N (0x7529dc): if `w2 < [0x874f40] = 0.0019569471f` (C0 alone; equal or NaN take the other path,
    `test ah,5; jp`): `a = rnd(w0*511)&0x3ff` (`[0x874f3c] = 511.0f`), value = `(0x7fc00 - (a<<10)) | a`
    (0x752a04-0x752a15). Else `b = rnd(w1*511)&0x3ff`, `c = rnd(w2*511)&0x3ff`,
    value = `((c<<10 | b) << 10) | (0x1ff - c - b)` with the low term NOT masked (a negative remainder sets high bits,
    0x752a50-0x752a69).
- librw: absent (librw's d3d9skin.cpp writes weights as FLOAT4 only).

## 0x752ad0 _rpD3D9VertexDeclarationInstV3d  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RwUInt32 type, RwUInt8* dst, const RwV3d* src, RwInt32 n, RwUInt32 stride)` (args [esp+0x18..0x28]
  after `push ecx` + 4 pushes, 0x752ad0-0x752adb). Callers: instance callback (2), sector instance 0x75e5e0, skin
  0x7c90d0, `_rpD3D9VertexDeclarationInstTangent` 0x754e20. `type > 0xe` -> size only; jump table 0x753174.
- Per vertex (x, y, z), src stride 0xc:
  - 0 FLOAT1: f32 x (0x752b01). 1 FLOAT2: x, y (0x752b32).
  - 2 FLOAT3: if `stride == 0xc` -> one `rep movsd/movsb` of `n*12` bytes WITHOUT the `n <= 0` test (0x752b55-0x752b78;
    a negative n gives a huge copy); else per-vertex copy of 3 dwords (0x752b9b).
  - 3 FLOAT4: x, y, z, 1.0f (immediate 0x3f800000, 0x752bdc).
  - 4 D3DCOLOR: dst[2] = rnd(x*255), dst[1] = rnd(y*255), dst[0] = rnd(z*255), dst[3] = 0xff (0x752c2d-0x752c7f).
  - 5 UBYTE4: dst[0..2] = fist(x, y, z), dst[3] = 1 (0x752cb7-0x752cf8).
  - 6 SHORT2: u16 fist(x), fist(y). 7 SHORT4: fist(x, y, z), u16[3] = 1 (0x752dcd).
  - 8 UBYTE4N: dst[0..2] = rnd(x*255, y*255, z*255), dst[3] = 0xff (0x752e06-0x752e59).
  - 9 SHORT2N: rnd(x*32767), rnd(y*32767). 0xa SHORT4N: x, y, z scaled, u16[3] = 0x7fff (0x752f4c).
  - 0xb USHORT2N: rnd(x*65535), rnd(y*65535). 0xc USHORT4N: 3 scaled, u16[3] = 0xffff (0x753044).
  - 0xd UDEC3: `(fist(z)&0x3ff)<<20 | (fist(y)&0x3ff)<<10 | (fist(x)&0x3ff)` (0x7530a4-0x7530cd).
  - 0xe DEC3N: same packing of rnd(x*511), rnd(y*511), rnd(z*511) (`[0x874f3c] = 511.0f`, 0x7530f7-0x75315e).
- Return `[0x874ef8 + 4*type]` (0x753167); for `type` 0xf/0x10 (no case, `ja`) this is 4/8, for 0x11 it would read
  0x43ff8000 (the 511.0f constant), not reachable from the instance callback.
- librw: pipeline.cpp:instV3d differs: only FLOAT3 and a librw-specific COMPNORM (11/11/10 bits with 1023/511
  scales, truncating casts); the exe has 15 D3DDECLTYPE cases, x87 rounding, and returns the element size.

## 0x7544e0 _rpD3D9VertexDeclarationInstV2d  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RwUInt32 type, RwUInt8* dst, const RwV2d* src, RwInt32 n, RwUInt32 stride)`, jump table 0x754a9c,
  `type > 0xe` -> size only (0x7544eb). Callers: instance callback 0x758165, sector instance, skin 0x7c90d0.
- Per vertex (u, v), src stride 8:
  - 0 FLOAT1: u. 1 FLOAT2: if `stride == 8` one `rep movsd` of `2n` dwords without an `n` test (0x75452c-0x754540),
    else u, v. 2 FLOAT3: u, v, 0 (0x7545a9). 3 FLOAT4: u, v, 0, 1.0f (0x7545ee-0x7545f4).
  - 4 D3DCOLOR: dst[2] = rnd(u*255), dst[1] = rnd(v*255), dst[0] = 0, dst[3] = 0xff (0x75463d-0x75465f).
  - 5 UBYTE4: dst[0] = fist(u), dst[1] = fist(v), dst[2] = 0, dst[3] = 1 (0x7546a8-0x7546c4).
  - 6 SHORT2: fist(u), fist(v). 7 SHORT4: fist(u), fist(v), 0, 1 (0x754780-0x754789).
  - 8 UBYTE4N: rnd(u*255), rnd(v*255), 0, 0xff (0x7547d6-0x7547f8). 9 SHORT2N: u, v x32767. 0xa SHORT4N: u, v x32767,
    0, 0x7fff (0x7548cc-0x7548d5). 0xb USHORT2N: x65535. 0xc USHORT4N: u, v x65535, 0, 0xffff (0x7549aa-0x7549b3).
  - 0xd UDEC3: `(fist(v)&0x3ff)<<10 | fist(u)&0x3ff` (0x754a00-0x754a1c). 0xe DEC3N: same with x511 (0x754a74).
- librw: pipeline.cpp:instTexCoords differs (FLOAT2 copy only; no other formats, no return value).

## 0x754ae0 _rpD3D9VertexDeclarationInstColor  (catalogue: specified/REA, port level 0)
- `RwBool (RwUInt8* dst, const RwRGBA* src, RwInt32 n, RwUInt32 stride)` (dst [esp+8] -> esi 0x754ae1, src
  [esp+0x10] after 2 pushes, n [esp+0x14] -> edi, stride [esp+0x20] -> ebp after 4 pushes). No type argument: always
  D3DCOLOR.
- `alphaAnd = 0xff` (0x754aea); for each vertex: `*(u32*)dst = a<<24 | r<<16 | g<<8 | b` (0x754b02-0x754b1c);
  `alphaAnd &= a` (0x754b21); `dst += stride`, `src += 4`. Return `alphaAnd != 0xff` (any alpha below 255),
  0 when `n <= 0` (0x754b2a-0x754b31).
- librw: pipeline.cpp:instColor matches in effect (returns hasAlpha), but takes a type argument.

## 0x754c80 _rpD3D9VertexDeclarationInstIndicesRemap  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RwUInt32 type, RwUInt8* dst, const RwUInt32* boneIndices /*4 u8 each*/, const RwUInt8* remap,
  RwInt32 n, RwUInt32 stride)`. Only types 4..7 (jump table 0x754e04, `type-4 > 3` -> size only, 0x754c8a-0x754c92).
  Caller: skin 0x7c90d0. With s[0..3] the four index bytes and `R(x) = remap[x]`:
  - 4 D3DCOLOR: dst = {R(s2), R(s1), R(s0), R(s3)} (0x754cbd-0x754ce6).
  - 5 UBYTE4: {R(s0), R(s1), R(s2), R(s3)} (0x754d1a-0x754d43).
  - 6 SHORT2: u16 {R(s0), R(s1)} (0x754d71-0x754d88). 7 SHORT4: u16 {R(s0)..R(s3)} (0x754db9-0x754ded).
- librw: absent.

## 0x7558f0 _rpD3D9GetMinMaxValuesV2d  (catalogue: specified/REA, port level 0)
- `void (const RwV2d* uv, RwInt32 n, RwV2d* min, RwV2d* max)` (uv [esp+4], min [esp+0xc], n/max at [esp+0xc]/[esp+0x14]
  after `push esi`).
- `*min = *max = uv[0]` (0x7558fd-0x75590e). If `n > 1` loop `n-1` times with `p` STARTING AT uv[0] (0x75591b,
  `add ecx, 8` at the end, 0x75596a): so elements 0..n-2 are examined and the LAST element uv[n-1] is never
  examined (original bug). Per element, per axis (x then y), else-if structure:
  `if (!(min.c <= p.c))  min.c = p.c` (fcomp; `and 0x4100` = C0|C3, 0x75591e-0x75592d, NaN does not update) `else if
  (max.c < p.c) max.c = p.c` (`test ah,5; jp`, 0x755931-0x75593e). The max test only runs when min did not change.
- librw: absent.

## 0x755aa0 _rpD3D9FindFormatV2d  (catalogue: specified/REA, port level 1)
- `RwUInt32 (const RwV2d* uv, RwInt32 n)` -> D3DDECLTYPE. Caller: skin instancing 0x7c9680.
- `_rpD3D9GetMinMaxValuesV2d(uv, n, &min, &max)` (locals: min at [esp], max at [esp+8], 0x755aa7-0x755ab7).
- Default result 1 FLOAT2 (0x755ac9). If `max.x > 1.0f` or `max.y > 1.0f` (or NaN; `test ah,0x41; jp`,
  `[0x858624] = 1.0f`, 0x755abc-0x755ae8) -> 1.
- If `min.x >= 0` and `min.y >= 0` (`[0x858b50] = 0.0f`, C0 test, 0x755aee-0x755b12): DeclTypes [0xc9bfec] & 0x10
  -> 0xb USHORT2N (0x755b1d); else `(DeclTypes & 0x80) ? 0xe DEC3N : 4 D3DCOLOR` (neg/sbb/and 0xa/add 4, 0x755b28-0x755b31).
- Else (some min < 0 or NaN): if `min.x < -1.0f` or `min.y < -1.0f` (`[0x858c1c] = -1.0f`, NaN counts as less,
  0x755b3a-0x755b5e) -> 1; else DeclTypes & 4 -> 9 SHORT2N (0x755b69); else DeclTypes & 0x80 (sign of the byte) ->
  0xe DEC3N (0x755b78); else 1.
- Note: because of the min/max bug above, the last uv is not part of the range test.
- librw: absent.

## 0x756710 _rpD3D9IndexRangeMinMax  (catalogue: unknown/REA, port level 0)
- Proposed name kept from the assignment; `void (const RwUInt16* idx, RwUInt32 n, RwUInt32* outNumVerts,
  RwUInt32* outMin)` (edi = n [esp+0xc] after `push edi`, esi = idx; outputs written at 0x756745 / 0x75674c).
  Caller: skin instancing 0x7c8d60 only.
- n == 0: both outputs 0 (0x756758/0x75675e). Else unsigned min (init 0xffffffff) and max (init 0) over the u16
  values (0x756723-0x75673b); `*outNumVerts = max - min + 1`, `*outMin = min`.
- librw: d3d9.cpp findMinVertAndNumVertices equivalent (matches).

## 0x756770 _rpD3D9SortTriangles  (catalogue: unknown/REA, port level 0)
- `void (RwUInt16* indices, RwUInt32 numIndices)`: `qsort(indices, numIndices/3 /*unsigned, 0xaaaaaaab mul*/, 6,
  _rpD3D9TriangleSortCompare 0x7567a0)` (CRT qsort 0x8247e0, 0x756770-0x756788). Caller: skin 0x7c8d60.
- librw: absent.

## 0x7567a0 _rpD3D9TriangleSortCompare  (catalogue: unknown/REA, port level 0)
- qsort comparator `int (const RwUInt16 a[3], const RwUInt16 b[3])`. Each triangle's three indices are sorted
  ascending with a 3-element network (swap(0,1) if t0 > t1; if t1 > t2 { swap(1,2); if t0 > t1 swap(0,1) },
  0x7567b7-0x7567d4 for a, 0x7567e8-0x756808 for b). Result: `a0 - b0` if different, else `a1 - b1` if different,
  else `a2 - b2` (int subtraction of zero-extended values, 0x75680a-0x756823).
- Also used by `_rpD3D9InstanceMeshes` (0x756c58) for TRIANGLELIST index buffers. librw: absent.

## 0x756830 _rpD3D9ConvertTriStripToList  (catalogue: unknown/REA, port level 0)
- `RwUInt32 (RwUInt16* dst, const RwUInt16* strip, RwUInt32 numIndices, RwUInt32 baseIndex)`; returns the number of
  indices written. Caller: `_rpD3D9InstanceMeshes` 0x756bf0.
- `count = numIndices - 2` (unsigned; numIndices < 2 wraps to a huge count, 0x75683c); if count == 0 return 0.
- For i = 0 .. count-1 (`v0 = strip[i], v1 = strip[i+1], v2 = strip[i+2]`): skip when any two RAW indices are equal
  (0x75686b-0x75687c). Otherwise, with `w = v - baseIndex` (u16 wrap): i odd -> dst = {w2, w1, w0} (0x756884-0x75689a),
  i even -> {w0, w1, w2} (0x75689f-0x7568b4); dst += 3, ret += 3. Parity is of the strip position i (degenerates count).
  Two copies of the loop: baseIndex != 0 (subtracting, 0x756864) and baseIndex == 0 (plain copy, 0x7568e8).
- librw: absent (librw keeps strips).

## 0x756960 _rpD3D9InstanceMeshes  (catalogue: unknown/REA, port level 3)
- `RwResEntry* (void* object, void* owner, RwUInt8 type /*1 atomic, 0xff sector*/, RwResEntry** repEntry,
  const RpMeshHeader* mh, RxD3D9AllInOneInstanceCallBack instanceCb, RwBool allocateNative)` (args [esp+0x30..0x48]
  after `sub esp,0x1c` + 4 pushes; mh -> ebp 0x756965). Callers: atomic node 0x757736, sector node 0x75e3f0.
- Size `sz = numMeshes*0x24 + 0x40` (0x756977-0x75697a).
- Allocation: `allocateNative` -> `res = engine->malloc(sz + 0x18, 0x3050d)` (engine+0x134, 0x756998), `*repEntry = res`,
  link/owner/ownerRef = 0, size = sz, destroyNotify = 0x4c9990 (0x7569ab-0x7569bb). Else
  `res = RwResourcesAllocateResEntry(owner, repEntry, sz, 0x4c9990)` (0x7569d4).
- `memset(res+0x18, 0, sz)` (0x7569e2-0x7569f9). header.serialNumber = mh.serialNum, numMeshes = mh.numMeshes,
  indexBuffer = 0, totalNumIndex = 0; local `ip = NULL` (index write pointer) (0x7569fb-0x756a15).
- Indices (only if `!(mh.flags & 0x100)`, 0x756a1f): totalNumIndex = sum of mesh.numIndices (0x756a35). If non-zero:
  - `rwPrim = meshModule.meshFlagsToPrimType[mh.flags & 0xff]` = byte at `engine + [0xc9b8c0] + 8 + (flags&0xff)`
    (0x756a4b-0x756a61). If rwPrim == 4 (TRISTRIP): `limit` = sector->numTriangles (u16 +0x84) for type 0xff,
    atomic->geometry->numTriangles (+0x10) for type 1, else 0x55555555; if `totalNumIndex > 3*limit` (unsigned):
    `convert = 1`, `totalNumIndex = 3*limit` (0x756a68-0x756aa3).
  - `RwD3D9IndexBufferCreate(totalNumIndex, &header.indexBuffer)` (0x756aab); on success
    `indexBuffer->Lock(0, 0, &ip, 0)` (IDirect3DIndexBuffer9 vtbl+0x2c, 0x756ac7).
  - If `convert`: header.primType = 4 TRIANGLELIST (0x756ad4) and the table lookup is skipped.
- Otherwise (unindexed, zero indices, or no conversion): header.primType = `[0x874fec + 4*rwPrim]` (0x756afb).
- vertexStream[0..1] all fields 0 (0x756b05-0x756b25), header.vertexDeclaration = 0 (0x756b34).
- Per mesh i (inst = header+0x40+0x24*i, mesh = mh+0x10+0xc*i), with running `vtx` (unindexed) and `startIdx`
  (kept in the repEntry arg slot):
  - Unindexed: numVertices = mesh.numIndices, minVert = vtx, vtx += numIndices (0x756b66-0x756b72).
    Indexed: min/max over the u16 indices -> numVertices = max-min+1, minVert = min; both 0 if numIndices == 0
    (0x756b7a-0x756bbb).
  - vertexShader = 0, material = mesh.material, vertexAlpha = 0 (0x756bbe-0x756bc7).
  - If `ip != NULL`: startIndex = startIdx; n = convert ? `_rpD3D9ConvertTriStripToList(ip, mesh.indices,
    numIndices, minVert)` : numIndices, copying with `- minVert` (u16) when minVert != 0 or a plain memcpy of
    `2*numIndices` bytes (0x756bd4-0x756c3a). numIndex = n (0x756c46). If header.primType == 4:
    `qsort(ip, n/3, 6, 0x7567a0)` (sorts the triangles of this mesh, 0x756c4e-0x756c69). `ip += n`, startIdx += n.
  - Else: startIndex = 0, numIndex = 0 (0x756c88-0x756c8f), n = mesh.numIndices for the next step.
  - numPrimitives from header.primType via jump table 0x756d74: 2 -> n>>1, 3 -> n-1, 4 -> n/3 (unsigned), 5 and 6 ->
    n-2, other -> 0 (0x756c9b-0x756ccd). baseIndex = 0 (0x756cdc).
- After the loop: if `ip` (the advanced pointer) != NULL -> `indexBuffer->Unlock()` (vtbl+0x30, 0x756d10).
- If instanceCb: `if (!instanceCb(object, header, 0 /*not reinstance*/))` (0x756d23) free the entry
  (`engine->free(res)` (+0x138) when allocateNative, else `RwResourcesFreeResEntry(res)`) and return NULL.
  Return res.
- Edge cases: numIndices of a converted strip mesh may exceed the buffer when the 3*limit clamp is hit (the clamp
  only reduces the buffer size, the conversion still writes every non-degenerate triangle) - original behaviour.
- librw: d3d9.cpp instanceMesh differs: always TRISTRIP/TRILIST from the flags, never converts strips, never sorts
  triangles, baseIndex = minVert (exe: 0 here, set later by the instance callback), no resource arena.

## 0x756d90 _rwD3D9EnableClippingIfNeeded  (catalogue: specified/REA, port level 5)
- `void (void* object, RwUInt32 type)`; camera = `engine->curCamera` (engine+0, 0x756d96).
- type == 1: `inside = RwD3D9CameraIsSphereFullyInsideFrustum(camera, RpAtomicGetWorldBoundingSphere(atomic))`
  (0x756da6/0x756dad); else `inside = RwD3D9CameraIsBBoxFullyInsideFrustum(camera, sector+0x60)` (0x756dc0).
- Tail call `RwD3D9SetRenderState(0x88 D3DRS_CLIPPING, inside == 0)` (0x756dcd-0x756ddc).
- Callers: the three game render callbacks (0x5d6480, 0x5d77d0, 0x5d9900) and MatFX 0x815c20. librw: absent.

## 0x756df0 _rxD3D9DefaultRenderCallback  (catalogue: specified/REA, port level 5)
- `void (RwResEntry* res, void* object, RwUInt8 type, RwUInt32 flags /*geometry flags*/)`; res -> ebp (0x756e60).
  Header fields below are `res+0x18+off`.
- Prologue (both paths): if [0x8e244c] != 0: `[0x8e244c] = 0`, `device->SetPixelShader(NULL)` (vtbl+0x1ac, 0x756e11).
  Clipping exactly as 0x756d90 (inlined, 0x756e17-0x756e5b). If header.indexBuffer != 0 and != [0x8e2450]:
  store and `device->SetIndices(ib)` (+0x1a0, 0x756e87). `_rwD3D9SetStreams(&header.vertexStream, header.useOffsets)`
  (0x756e95). If header.vertexDeclaration != [0x8e2444]: store, `[0x8e2440] = -1`, `device->SetVertexDeclaration`
  (+0x15c, 0x756ec5). `RwD3D9GetRenderState(0x89 LIGHTING, &lit)` (0x756ed5).
- Unlit path (lit == 0 and `!(flags & 8 PRELIT)`, 0x756ee1-0x756ef0): save RS 0x1a DITHERENABLE and RS 9 SHADEMODE
  (into the flags and object argument slots, 0x756ef6-0x756f09); `_rwD3D9RenderStateVertexAlphaEnable(0)`; RS 0x3c
  TEXTUREFACTOR = 0xff000000, RS 0x1a = 0, RS 9 = 1 FLAT; TSS stage 0: COLOROP = 3 SELECTARG2, COLORARG1 = 2 TEXTURE,
  COLORARG2 = 3 TFACTOR, ALPHAOP = 3, ALPHAARG1 = 2, ALPHAARG2 = 3; `RwD3D9SetTexture(NULL, 0)` (0x756f0e-0x756f72).
  Per instance (alphaMode `edi` starts 0): if `(flags & 0x84 TEXTURED|TEXTURED2)` and material->texture and
  `_rwD3D9RasterHasAlpha(texture)` (0x4c9ea0 reads `texture->raster` then the raster ext alpha byte, so it is called
  with the texture, 0x756f9f): `RwD3D9SetTexture(texture, 0)`; if alphaMode == 0 { TSS(0, ALPHAOP, 4 MODULATE);
  alphaMode = 1 }. Else if alphaMode == 1 { alphaMode = 0; TSS(0, ALPHAOP, 3); SetTexture(NULL, 0) }
  (0x756f92-0x756fec). Then set the instance vertex shader if != [0x8e2448] (+0x170, 0x75700b) and draw (below).
  After all instances restore RS 0x1a and RS 9 (0x757074-0x757087). TEXTUREFACTOR and vertex alpha are not restored.
- Lit/prelit path (0x757097): `va = _rwD3D9RenderStateIsVertexAlphaEnable()`, `last = 0x80000000`. Per instance
  (material m = inst.material):
  1. Vertex alpha: wanted = !(m->color.alpha == 0xff && inst.vertexAlpha == 0) (0x7570c7-0x7570d6); if wanted != va:
     `va = wanted`, `_rwD3D9RenderStateVertexAlphaEnable(wanted)` (0x7570dc-0x7570f1).
  2. mode = 0. If lit: `RwD3D9SetSurfaceProperties(&m->surfaceProps (+0xc), &m->color (+4), flags)` (0x75710b).
     Else if `(flags & 0x40 MODULATE)` and `*(u32*)&m->color != 0xffffffff`: mode = 2, RS 0x3c TEXTUREFACTOR =
     `a<<24 | r<<16 | g<<8 | b` (0x757121-0x757144).
  3. If m->texture and `(flags & 0x84)`: mode |= 1, `RwD3D9SetTexture(texture, 0)`, else `RwD3D9SetTexture(NULL, 0)`.
  4. If mode != last (0x757171): textured (mode&1): if `!(last & 1)`: stage 0 COLOROP 4 MODULATE, COLORARG1 2 TEXTURE,
     COLORARG2 0 DIFFUSE, ALPHAOP 4, ALPHAARG1 2, ALPHAARG2 0 (0x757186-0x7571c3). Then if mode&2: stage 1 COLOROP 4,
     COLORARG1 1 CURRENT, COLORARG2 3 TFACTOR, ALPHAOP 4, ALPHAARG1 1, ALPHAARG2 3 (0x7571d8-0x757215); else if
     last == 3: stage 1 COLOROP 1 DISABLE, ALPHAOP 1 (0x7572a6-0x7572b7). Untextured: mode&2 -> stage 0 COLOROP 4,
     COLORARG1 0 DIFFUSE, COLORARG2 3 TFACTOR, ALPHAOP 4, ALPHAARG1 0, ALPHAARG2 3 (0x757227-0x757264); else stage 0
     COLOROP 3 SELECTARG2, COLORARG2 0, ALPHAOP 3, ALPHAARG2 0 (0x75726e-0x757295); then if last == 3 disable
     stage 1. `last = mode`.
  5. Vertex shader if != [0x8e2448] (0x7572e1), then draw.
  After the loop: if last == 3 disable stage 1 COLOROP/ALPHAOP (0x757355-0x757366).
- Draw (both paths): `_rwD3D9RenderStateFlushCache()` then, if header.indexBuffer:
  `DrawIndexedPrimitive(header.primType, inst.baseIndex, 0, inst.numVertices, inst.startIndex, inst.numPrimitives)`
  (vtbl+0x148, 0x75703b / 0x757315); else `DrawPrimitive(header.primType, inst.baseIndex, inst.numPrimitives)`
  (vtbl+0x144, 0x75705c / 0x757336).
- librw: d3d9render.cpp defaultRenderCB_Fix differs completely (always uses the material colour through stage 1
  TSS_CONSTANT, sets LIGHTING itself, no unlit TFACTOR path, no mode caching, no clipping test).

## 0x757380 RxD3D9AllInOneSetInstanceCallBack  (catalogue: specified/REA, port level 0)
- `void (RxPipelineNode* node, cb)`: `((void**)node->privateData)[0] = cb` (+0x14, 0x757388).
## 0x757390 RxD3D9AllInOneGetInstanceCallBack  (catalogue: specified/REA, port level 0)
- `cb (RxPipelineNode* node)`: return `privateData[0]` (0x757397).
## 0x7573a0 RxD3D9AllInOneSetReinstanceCallBack  (catalogue: specified/REA, port level 0)
- `privateData[1] = cb` (+4, 0x7573ab). (Set/Get lighting at 0x7573c0/0x7573d0 use +8, Get render 0x7573f0 +0xc.)
## 0x7573b0 RxD3D9AllInOneGetReinstanceCallBack  (catalogue: specified/REA, port level 0)
- return `privateData[1]` (0x7573b7).
## 0x7573e0 RxD3D9AllInOneSetRenderCallBack  (catalogue: specified/REA, port level 0)
- `privateData[3] = cb` (+0xc, 0x7573eb). librw: ObjPipeline fields instanceCB/renderCB (equivalent).

## 0x757400 _rxD3D9AtomicDefaultLightingCallback  (catalogue: specified/REA, port level 5)
- `void (RpAtomic* atomic)` (ebp = [esp+0x14] after `sub esp,8` + 2 pushes). geomFlags = atomic->geometry->flags (ebx).
- If `!(geomFlags & 0x20 LIGHT)` or `engine->curWorld` (engine+4) == NULL: `_rwD3D9LightsEnable(0, 1)` (0x7575d9).
- Else: AmbientSaturated [0x8e2418..0x8e2424] = (0, 0, 0, 1.0f) (0x757428-0x757446); `lit = 0`.
  For each link in `world+0x3c` (directional light list; light = link - 0x34, 0x757467): if light != NULL and
  `light->object.flags (+2) & 1 LIGHTATOMICS`: if subType (+1) == 1 DIRECTIONAL: only if `geomFlags & 0x10 NORMALS`:
  `_rwD3D9LightDirectionalEnable(light)`, lit = 1 (0x757480); else (any other type, i.e. ambient):
  AmbientSaturated.rgb += light->color.rgb (+0x18/+0x1c/+0x20; x87 `fld acc; fadd light; fstp` f32 each,
  0x75748a-0x7574b1), lit = 1.
  `RwD3D9SetRenderState(0x8b AMBIENT, lit ? 0xffffffff : 0)` (0x7574c5-0x7574d8).
  If geomFlags & 0x10: `engine->lightFrame` (u16 +0xa) += 1 (0x7574f5); for each tie link in
  `atomic->llWorldSectorsInAtomic` (+0x64): sector = `[link+8]` (RpTie.worldSector), for each light tie in
  `sector+0x40`: light = `[tie+8]`; if light && `light->lightFrame (u16 +0x3c) != engine.lightFrame` &&
  `light->object.flags & 1`: mark `light->lightFrame = lightFrame` (0x757535); `pos = RwFrameGetLTM(light->frame)+0x30`;
  `s = RpAtomicGetWorldBoundingSphere(atomic)`; x87: `dx = s.x-pos.x; dy = s.y-pos.y; dz = s.z-pos.z;
  r = light->radius (+0x14) + s.radius (+0xc); d2 = (dy*dy + dx*dx) + dz*dz; r2 = r*r` (0x757554-0x757582);
  if `r2 > d2` (fcompp, C0|C3 clear, NaN rejects, 0x757584-0x757595): `lit |= _rwD3D9LightLocalEnable(light)`.
- `_rwD3D9LightsEnable(lit, 1)` (0x7575c9). The second argument is not read by 0x756600.
- librw: d3d/d3d9render.cpp/lightingCB_Fix differs (no light-frame marking, no sphere test against the sector
  lights).

## 0x7575f0 _rwD3D9AtomicAllInOneNode  (catalogue: specified/REA, port level 4)
- Node body `RwBool (RxPipelineNode* self, const RxPipelineNodeParam* params)`; atomic = `*(RpAtomic**)params`
  (0x7575fc), cb = self->privateData (0x757600), g = atomic->geometry.
- Return 1 immediately if `g->numVertices <= 0` (signed, 0x75760c) or `g->mesh->numMeshes == 0` (0x75761a).
- `flags = g->flags` (kept in the `self` arg slot). If `flags & 0x1000000 NATIVE`: res = g->repEntry (0x757754),
  no instancing.
- Else res = (g->numMorphTargets == 1) ? g->repEntry : atomic->repEntry (0x757632-0x75763f).
  - res != NULL and `res.header.serialNumber != mesh.serialNum` -> `RwResourcesFreeResEntry(res)` and instance anew.
  - res valid: if `g->lockedSinceLastInst != 0` or `numMorphTargets != 1` (0x757675-0x75767f): if cb.reinstance:
    `if (!cb.reinstance(atomic, res, cb.instance)) { RwResourcesFreeResEntry(res); return 0; }` (0x75768e-0x7576a9);
    then (also when no reinstance callback) `atomic->interpolator.flags &= ~1` (dword and, 0x7576aa) and
    `g->lockedSinceLastInst = 0`. Then (all valid cases) move res to the front of the used-resource list if it is
    linked (`res.link.next != 0`): unlink, insert after the list head `*(engine + [0xc9bc58] + 0x24)`
    (0x7576b8-0x757718) - an inlined RwResourcesUseResEntry.
  - No/freed res: `res = _rpD3D9InstanceMeshes(atomic, numMorph==1 ? g : atomic, 1, numMorph==1 ? &g->repEntry :
    &atomic->repEntry, g->mesh, cb.instance, (flags >> 25) & 1)` (0x75771c-0x757736); NULL -> return 0;
    `g->lockedSinceLastInst = 0` (0x75774c).
- If `flags & 0x2000000` -> return 1 without lighting/rendering (0x757757).
- `if (cb.lighting) cb.lighting(atomic)` (0x75776d). `ltm = RwFrameGetLTM(atomic->frame)`,
  `RwD3D9SetTransformWorld(ltm)` (0x757776/0x75777e).
- NORMALIZENORMALS: `RwD3D9GetRenderState(0x89, &lit)`; value 0 if `!lit` or `ltm->flags (+0xc) & 0x20001`.
  Otherwise, for right (+0), up (+0x10), at (+0x20): `len2 = (x*x + y*y) + z*z` (x87, stored f32) and compared as a
  SIGNED INT bit pattern with 0x3f8ccccd (1.1f) and 0x3f666666 (0.9f) (immediates at 0x7577be/0x7577c6): value 1 if
  any axis has `bits > 1.1f` or `bits < 0.9f` (NaN bit patterns 0x7fc.. are > 1.1f), else 0 (0x7577ae-0x757850).
  `RwD3D9SetRenderState(0x8f NORMALIZENORMALS, value)` (0x75785d).
- `if (cb.render) cb.render(res, atomic, 1, flags)` (0x757875). Return 1.
- librw: d3d9.cpp instance/render differ (geometry-only instance data, no morph handling, no NORMALIZENORMALS test).

## 0x757890 _rwD3D9AtomicAllInOnePipelineInit  (catalogue: specified/REA, port level 0)
- `RwBool (RxPipelineNode* node)`: privateData = {0x7578c0 instance, 0x758270 reinstance, 0x757400 lighting,
  0x756df0 render} (0x757897-0x7578ab); return 1. Referenced from the node definition 0x8d6338 (+0x10 = 0x757890).

## 0x7578c0 _rxD3D9DefaultInstanceCallback  (catalogue: specified/REA, port level 3)
- `RwBool (void* atomic, RxD3D9ResEntryHeader* h, RwBool reinstance)` ([ebp+8], [ebp+0xc] -> edi, [ebp+0x10]);
  ebp frame with 8-aligned esp, locals: `decl[]` D3DVERTEXELEMENT9 at local 0x40 (8 bytes each: +0 u16 Stream,
  +2 u16 Offset, +4 u8 Type, +5 Method, +6 Usage, +7 UsageIndex), `lockPtr[2]` at local 0x28, `numStreams` local 0x18,
  `lockMask` local 0x1c, `usage` local 0x20, geom flags local 0x24, `numTexCoordSets` local 0x34.
- g = atomic->geometry; `usage = RpD3D9GeometryGetUsageFlags(g)` (0x7578e1).
- A. First instance (`reinstance == 0`, 0x7578fc):
  1. h.totalNumVertex = g->numVertices (0x757905). For s = 0,1: if stream.vb: managed -> `RwD3D9DestroyVertexBuffer(
     stride, stride*totalNumVertex, vb, offset)` (0x757938); else if offset == 0 and !dynamicLock ->
     `RwD3D9DynamicVertexBufferDestroy(vb)` (0x75794f); otherwise no destroy. In all cases vb, offset, stride,
     geometryFlags, managed, dynamicLock = 0 (0x757959-0x757969).
  2. dynamic = (`numMorphTargets > 1` or `usage & 0xfff`) and `[0xc9bf1c] & 0x10000` and !(atomic->pipeline &&
     pipeline->pluginId == 0x12e) (0x75797b-0x7579aa). If dynamic: if numMorph > 1 `usage |= 6`; numStreams = 2,
     `_rpD3D9AddDynamicGeometry(g)` (0x7579d1). Else numStreams = 1.
     Stream choice per attribute: `s = (usage & bit) ? 0 : numStreams-1`.
  3. Elements, in this order, each with Offset = current stride of its stream: POSITION (bit 2): FLOAT3, stride = 0xc
     (SET, not added) and geometryFlags = 2 (set) (0x757a32/0x757a39). NORMAL if `geomFlags & 0x10` (bit 4): DEC3N +4
     if DeclTypes & 0x80 else FLOAT3 +0xc, geometryFlags |= 4 (0x757a55-0x757a96). COLOR 0 if `geomFlags & 8` (bit 8):
     D3DCOLOR +4, |= 8 (0x757aa4-0x757ae5). TEXCOORD i for i < numTexCoordSets (bit 0x10<<i): FLOAT2 +8, |= 0x10<<i
     (0x757af6-0x757b46). TANGENT 0 if `usage & 0x10000` and numTexCoordSets > 0 (dynamic test `usage & 0xff2`):
     DEC3N +4 / FLOAT3 +0xc, geometryFlags NOT updated (0x757b5f-0x757bc1). Method 0 everywhere.
  4. While numStreams > 1 and the last stream's stride is 0: numStreams-- (0x757bc2-0x757bde).
  5. `qsort(decl, count, 8, _rxD3D9IndexCompare 0x758240)` (by Stream then Offset), append D3DDECL_END
     {0xff, 0, 0x11, 0, 0, 0}, `RwD3D9CreateVertexDeclaration(decl, &h.vertexDeclaration)` (0x757be2-0x757c1f).
  6. For s < numStreams: size = stride*totalNumVertex; if `!dynamic || (s != 0 && [0xc9bfd4] & 1)`: managed = 1,
     `RwD3D9CreateVertexBuffer(stride, size, &vb, &offset)` (0x757c61); else managed = 0,
     `RwD3D9DynamicVertexBufferCreate(size, &vb)` (0x757c71). Failure -> return 0 (0x757cb2).
  7. numStreams > 1: `useOffsets = 1` only if [0xc9bfd4] & 1; every inst.baseIndex = inst.minVert (0x757c8f-0x757cae).
     numStreams == 1: useOffsets = 0, inst.baseIndex = stream0.offset / stream0.stride (unsigned div) + minVert
     (0x757cbb-0x757cdc).
- B. Reinstance: `h.vertexDeclaration->GetDeclaration(decl, &n)` (vtbl+0x10, 0x757cf0); numStreams = 1 + max Stream
  over the elements before the Type 0x11 terminator (0 if the first is the terminator) (0x757cf3-0x757d1d).
- C. Lock (0x757d21): lockPtr = {0, 0}. First instance: lockMask = 0xfff, each stream
  `vb->Lock(offset, stride*totalNumVertex, &lockPtr[s], 0x800 NOSYSLOCK)` (IDirect3DVertexBuffer9 vtbl+0x2c,
  0x757d73). Reinstance: lockMask = g->lockedSinceLastInst; a stream is locked only if lockMask == 0xfff or
  `lockMask & stream.geometryFlags`; managed -> flags 0x800; dynamic -> flags 0x2800 (DISCARD|NOSYSLOCK) and
  `lockMask |= stream.geometryFlags` (everything in a discarded stream is rewritten) (0x757d9d-0x757df0).
- D. Positions/normals (element lookup = first element with the Usage/UsageIndex, no bound; dst = lockPtr[Stream] +
  Offset; count = totalNumVertex; stride = the stream stride):
  - numMorph != 1: start = (i16)atomic+0x50, end = (i16)+0x52; if start >= numMorph or end >= numMorph both = 0
    (signed compares, 0x757e1f-0x757e2f). `t = atomic->position (+0x5c) * recipTime (+0x58)` (x87, stored f32
    local 0x10, 0x757e33-0x757e3b). If start != end and `t != 0.0f` (NaN counts as != 0; `[0x858b50] = 0.0f`,
    0x757e41-0x757e50): morph path: `_rpD3D9VertexDeclarationInstV3dMorph(type, dst, mt[start].verts, mt[end].verts,
    t, count, stride)` (0x753b60, 0x757f0f) for POSITION and, if geomFlags & 0x10, for NORMAL with the normals
    (0x757f77); the morph path ignores lockMask. Otherwise fall into the static path with morph index `start`.
  - Static path (numMorph == 1 uses index 0): if lockMask & 2: `InstV3d(type, dst, mt[idx].verts, count, stride)`
    (0x757faf); if geomFlags & 0x10 and lockMask & 4: same with normals (0x75801c).
- E. Prelight (0x758024): geomFlags & 8 and lockMask & 8: COLOR element; per instance
  `inst.vertexAlpha = InstColor(lockPtr[s] + Offset + minVert*stride, g->preLitLum + minVert, inst.numVertices,
  stride)` (0x7580ae-0x7580b3). Not prelit and first instance: every inst.vertexAlpha = 0 (0x7580d0-0x7580e0).
  Prelit but bit 8 not in lockMask: untouched.
- F. Texcoords: if numTexCoordSets and lockMask & 0xff0: for i, if lockMask & (0x10<<i): TEXCOORD i ->
  `InstV2d(type, dst, g->texCoords[i], count, stride)` (0x758165).
- G. Tangents: usage & 0x10000, numTexCoordSets != 0 and lockMask & 0xff2: TANGENT 0 ->
  `_rpD3D9VertexDeclarationInstTangent(type, dst, mt[start].verts, g->texCoords[numTexCoordSets-1], h, stride)`
  (0x754e20, 0x7581f6; the LAST texcoord set is used).
- H. Unlock every stream with lockPtr != NULL (`vb->Unlock()` vtbl+0x30, 0x75821e). Return 1.
- librw: d3d9.cpp defaultInstanceCB differs: one stream always, FLOAT3 normals, fixed element order (prelit, texcoords,
  normals), extra stream-2 fallback elements, no dynamic buffers, no morphing, no tangents, no element sort.

## 0x758240 _rxD3D9IndexCompare  (catalogue: unknown/REA, port level 0)
- qsort comparator on D3DVERTEXELEMENT9: `a.Stream - b.Stream` (zero-extended u16) if non-zero, else
  `a.Offset - b.Offset` (0x758249-0x758264). librw: absent.

## 0x758270 _rxD3D9DefaultReinstanceCallback  (catalogue: specified/REA, port level 4)
- `RwBool (RpAtomic* atomic, RwResEntry* res, RxD3D9AllInOneInstanceCallBack instanceCb)`.
- numMorphTargets != 1 (0x758278): only if `atomic->interpolator.flags & 1` or `g->lockedSinceLastInst != 0`:
  `saved = locked; locked |= 6; _rxD3D9DefaultInstanceCallback(atomic, res+0x18, 1); locked = saved`
  (0x75828b-0x7582b1; calls the DEFAULT callback directly, ignoring instanceCb and its result). Return 1.
- numMorphTargets == 1: if instanceCb: return `instanceCb(atomic, res+0x18, 1) ? 1 : 0` (0x7582d0-0x7582d9); else 1.
- librw: absent (reinstance is folded into instance()).

## 0x7582e0 RxNodeDefinitionGetD3D9AtomicAllInOne  (catalogue: specified/REA, port level 0)
- Returns 0x8d6338 (RxNodeDefinition: +0 name ptr 0x8d6318, +4 body 0x7575f0, +0x10 pipelineNodeInit 0x757890,
  +0x34 privateDataSize 0x10 - dwords read from .data 0x8d6338).

## 0x7588b0 RpD3D9GeometrySetUsageFlags  (catalogue: specified/REA, port level 0)
- `void (RpGeometry* g, RwUInt32 flags)`: `*(u32*)((u8*)g + [0xc926e8]) = flags` (0x7588be).

## 0x7588d0 RpD3D9GeometryGetUsageFlags  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RpGeometry* g)`: return `*(u32*)((u8*)g + [0xc926e8])` (0x7588da). librw: absent.

## 0x75e540 _rwD3D9WorldSectorAllInOnePipelineInit  (catalogue: specified/REA, port level 0)
- privateData = {0x75e5e0 instance, 0 reinstance, 0x75e570 lighting, 0x756df0 render} (0x75e547-0x75e55b); return 1.

## 0x75e9f0 RxNodeDefinitionGetD3D9WorldSectorAllInOne  (catalogue: specified/REA, port level 0)
- Returns 0x8d63b8 (name 0x8d6398, body 0x75e3f0, init 0x75e540, privateDataSize 0x10).

## Open questions
- UNCLEAR: pluginId 0x12e excluded from dynamic buffers at 0x7579a3 - which plugin pipeline carries it (not one of the
  pipelines in d3d9.md section 6); behaviour is fully determined by the compare.
- UNCLEAR: `_rpD3D9VertexDeclarationInstV3dMorph` 0x753b60 and `_rpD3D9VertexDeclarationInstTangent` 0x754e20 are
  not in this assignment; their argument lists above are taken from the call sites (0x757f0f, 0x7581f6).
