# RenderWare 3.6 RpSkin (skin plugin, id 0x116) and its D3D9 pipeline - full specs, writer C (RE-SPEC-RW-2)

Statically linked RW 3.6 plugin code in gta_sa.exe 1.0 US. gta-reversed has only `plugin::Call*` thunks
(RenderWare/rw/rpskin.cpp), so everything below comes from the bytes. All functions are `__cdecl` (caller pops).
`G` = `RwEngineInstance` (dword at 0xc97b24); `G+0x134 rwmalloc(size, hint)`, `G+0x138 rwfree(p)`, `G+0x144
freeListAlloc(fl, hint)`, `G+0x148 freeListFree(fl, p)`. Names of callees that are not in this file are catalogue
names (some are descriptive, marked "cat."); their behaviour is only described where this file relies on it.

## Layouts used (offsets proven by the instruction addresses given)

Globals (.data unless noted):
| global | meaning | proof |
|---|---|---|
| `[0xc978a0] i32` | engine plugin offset (RwEngineRegisterPlugin result, never tested) | 0x7c684c |
| `[0xc978a4] i32` | atomic extension offset; ext = `RpHAnimHierarchy*` (4 bytes) | 0x7c686a, 0x7c7524 |
| `[0xc978a8] i32` | geometry extension offset; ext = `RpSkin*` (4 bytes) | 0x7c68c4, 0x7c7554 |
| `[0xc978ac] u8*` | bone-matrix cache, 16-byte aligned (0x4000 usable bytes, 0x30 per bone) | 0x7c6983, 0x7c8107 |
| `[0xc978b0] void*` | the unaligned 0x400f-byte block behind 0xc978ac | 0x7c696a |
| `[0xc978b8] RwFreeList*` | RpSkin freelist (0x40-byte entries) | 0x7c6956 |
| `[0xc978c0] i32` | engine-open counter (init runs only when 0) | 0x7c68e0, 0x7c698f |
| `[0xc978c4] RxPipeline*` | the skin pipeline | 0x7c8999, 0x7c89b4 |
| `[0xc978d4]`, `[0xc978d8]`, `[0xc978dc]` | matrix-cache keys: hierarchy, `~renderFrame`, atomic frame | 0x7c78f6, 0x7c7909, 0x7c7913 |
| `[0xc978e0] i32` | caps: hardware T&L (DevCaps bit 16) | 0x7cb2bb |
| `[0xc978e4] i32` | caps: VS 1.1 and PS 1.1 present | 0x7cb2eb |
| `[0xc978e8] u32` | max hardware bones = (MaxVertexShaderConst - 12) / 3 | 0x7cb30c |
| `[0xc978ec] u32` | MaxVertexShaderConst | 0x7cb2fb |
| `[0x8dece0] = 0x14`, `[0x8dece4] = 0x1` | freelist entries per block / blocks to preallocate (RpSkinSetFreeListCreateParams 0x7c67f0) | 0x7c692b, 0x7c6926 |
| 0xc97078 | static RwFreeList storage | 0x7c6936 |
| `[0x858624] = 1.0f` (.rdata) | constant 1.0 | 0x7c92bb, 0x7c82a9 |

`RpSkin` (0x40 bytes, freelist [0xc978b8]; zeroed with 16 dwords after allocation, 0x7c75e4):
| off | field | proof |
|---|---|---|
| +0x00 | `u32 numBones` | 0x7c76b7, RpSkinGetNumBones 0x7c77e4 |
| +0x04 | `u32 numUsedBones` | 0x7c76bd |
| +0x08 | `u8* usedBones` (= start of the data block) | 0x7c76dc |
| +0x0c | `RwMatrix* skinToBoneMatrices` (16-byte aligned) | 0x7c76c9, 0x7c7814 |
| +0x10 | `u32 maxNumWeightsForVertex` (1..4) | 0x7c75f2, 0x7c704c |
| +0x14 | `u32* vertexBoneIndices` (4 x u8 per vertex) | 0x7c76ce, 0x7c7804 |
| +0x18 | `RwMatrixWeights* vertexBoneWeights` (4 x f32 per vertex) | 0x7c76d3, 0x7c77f4 |
| +0x1c | `u32` bones needed by hardware skinning (boneLimit if split, else numUsedBones) | 0x7c8ff9, 0x7c9001 |
| +0x20 | `u32` use-hardware (vertex shader) flag | 0x7c900e |
| +0x24 | split: `u32 boneLimit` | 0x7c8cdd |
| +0x28 | split: `u32 numMeshes` | 0x7c8cdf |
| +0x2c | split: `u32 numRLE` | 0x7c8cf1 |
| +0x30 | split: `u8* meshBoneRemapIndices` (own rwmalloc block) | 0x7c8cb9 |
| +0x34 | split: `u8* meshBoneRLECount` (2 bytes per mesh: start, count) | 0x7c8ce9 |
| +0x38 | split: `u8* meshBoneRLE` (2 bytes per run: first, count) | 0x7c8cf4 |
| +0x3c | `void* data` (one rwmalloc block: usedBones, matrices, indices, weights) | 0x7c7689 |

Data block layout (RpSkinCreate / _rpSkinAllocateData): size = `numBones*64 + numVertices*20 + numUsedBones + 0xf`
(0x7c7665..0x7c7673); `usedBones = data`; `matrices = (data + numUsedBones + 0xf) & ~0xf` (0x7c76b9..0x7c76c9);
`indices = matrices + numBones*64` (0x7c76cc); `weights = indices + numVertices*4` (0x7c76d1).

Other structures (see objects.md / d3d9.md): RpAtomic +0x4 frame, +0x14 repEntry, +0x18 geometry, +0x4c interpolator
flags, +0x6c pipeline; RpGeometry +0x8 flags (0x8 PRELIT, 0x10 NORMALS, 0x40 MODULATEMATERIALCOLOR, 0x1000000 NATIVE),
+0xc lockedSinceLastInst (u16), +0x14 numVertices, +0x18 numMorphTargets, +0x1c numTexCoordSets, +0x30 preLitLum,
+0x34 texCoords[8], +0x54 mesh header, +0x58 repEntry, +0x5c morphTarget (+0x14 verts, +0x18 normals);
`RwResEntry` (0x18) + `RxD3D9ResEntryHeader` (0x40: +0x0 serialNumber, +0x4 numMeshes, +0x8 indexBuffer, +0xc primType,
+0x10 vertexStream[2] (0x10 each: +0 vb, +4 offset, +8 stride, +0xc u16 geometryFlags, +0xe u8 managed, +0xf u8
dynamicLock), +0x30 useOffsets, +0x34 vertexDeclaration, +0x38 totalNumIndex, +0x3c totalNumVertex) + numMeshes x
`RxD3D9InstanceData` (0x24: +0 numIndex, +4 minVert, +8 material, +0xc vertexAlpha, +0x10 vertexShader, +0x14 baseIndex,
+0x18 numVertices, +0x1c startIndex, +0x20 numPrimitives). D3DCAPS9: +0x1e (byte 2 of DevCaps), +0xc4
VertexShaderVersion, +0xc8 MaxVertexShaderConst, +0xcc PixelShaderVersion, +0xd4 DevCaps2 (bit0 STREAMOFFSET), +0xec
DeclTypes. `D3DVERTEXELEMENT9` (8 bytes): +0 Stream (u16), +2 Offset (u16), +4 Type, +5 Method, +6 Usage, +7 UsageIndex.

## 0x7C6820 RpSkinPluginAttach  (catalogue: specified/REA, port level 3)
- Convention: `RwBool (void)`; ALWAYS returns 1 (0x7c68d1): no registration result is tested. Caller App::PluginAttach.
- Behaviour, in order: `[0xc978a0] = RwEngineRegisterPlugin(0, 0x116, Open=0x7c68e0, Close=0x7c69a0)` (0x7c6831);
  `[0xc978a4] = RpAtomicRegisterPlugin(4, 0x116, Ctor=0x7c6a80, Dtor=0x7c6aa0, Copy=0x7c6ac0)` (0x7c6851);
  `RpAtomicRegisterPluginStream(0x116, Read=0x7c72a0, Write=0x7c7500, GetSize=0x7c7510)` (0x7c686f);
  `RpAtomicSetStreamAlwaysCallBack(0x116, 0x7c6ae0)` (0x7c687e); `RpAtomicSetStreamRightsCallBack(0x116, 0x7c6b70)` (0x7c688d);
  `[0xc978a8] = RpGeometryRegisterPlugin(4, 0x116, Ctor=0x7c69f0, Dtor=0x7c6a10, Copy=0x7c6a70)` (0x7c68ab);
  `RpGeometryRegisterPluginStream(0x116, Read=0x7c6d20, Write=0x7c6c30, GetSize=0x7c6be0)` (0x7c68c9).
- librw: skin.cpp:registerSkinPlugin matches the registrations (librw registers no engine-offset global).

## 0x7C68E0 cb_SKINPLUGIN_RwEngine_Open  (catalogue: specified/REA, port level 7)
- Convention: `void* (void* instance, i32 offset, i32 size)`; returns instance always (0x7c6994).
- Behaviour: if `[0xc978c0] != 0` skip to 4 (0x7c68e7). 1. `type = 1`; if `RwEngineGetPluginOffset(0x120)` (MatFX)
  `!= -1` -> `type = 3` (0x7c6906); if `RwEngineGetPluginOffset(0x12e) != -1` -> `type |= 4` (0x7c691d).
  2. `_rpSkinPipelinesCreate(type)` (0x7c6921; result ignored). 3. `[0xc978b8] = RwFreeListCreateAndPreallocateSpace(0x40,
  [0x8dece0], 4, [0x8dece4], 0xc97078, 0x40116)` (0x7c6941; not tested); `p = G->rwmalloc(0x400f, 0x40116)` (0x7c695b);
  `[0xc978b0] = p`; `memset(p, 0, 0x400f)` (rep stosd 0x1003 + stosw + stosb, 0x7c6970..0x7c6974; no NULL test);
  `[0xc978ac] = (p + 0xf) & ~0xf` (0x7c697d..0x7c6983). 4. `[0xc978c0] += 1` (0x7c698e); return instance.
- librw: skin.cpp:skinOpen differs: librw only creates pipelines; no freelist, no matrix cache, no counter.

## 0x7C69F0 cb_SKINPLUGIN_RpGeometry_Ctor  (catalogue: specified/REA, port level 0)
- `*(RpSkin**)(geom + [0xc978a8]) = NULL` (0x7c69fa); returns EAX = geom.
- librw: skin.cpp:createSkin matches.

## 0x7C6A10 cb_SKINPLUGIN_RpGeometry_Dtor  (catalogue: specified/REA, port level 1)
- `void* (RpGeometry* g, i32, i32)`, returns g. `skin = ext`; NULL -> return. Else `_rpSkinDeinitialize(g)` (0x7c6a23);
  if `skin->data` (+0x3c) `G->rwfree(data)` (0x7c6a39); `_rpSkinSplitDataDestroy(skin)` (0x7c6a43);
  `G->freeListFree([0xc978b8], skin)` (0x7c6a55); `ext = NULL` (0x7c6a64).
- librw: skin.cpp:destroySkin matches in substance (rwFree instead of freelist; librw frees platformData too).

## 0x7C6A80 cb_SKINPLUGIN_RpAtomic_Ctor  (catalogue: specified/REA, port level 0)
- `*(atomic + [0xc978a4]) = NULL` (0x7c6a8c); returns EAX = atomic. librw: skin.cpp:createSkinAtm matches.

## 0x7C6AA0 cb_SKINPLUGIN_RpAtomic_Dtor  (catalogue: specified/REA, port level 0)
- If `*(atomic + [0xc978a4]) != 0` store 0 (0x7c6ab0). Returns EAX = atomic ([esp+4], 0x7c6aa6). The hierarchy is not
  destroyed. librw: skin.cpp:destroySkinAtm matches (librw stores nil unconditionally).

## 0x7C6AC0 cb_SKINPLUGIN_RpAtomic_Copy  (catalogue: specified/REA, port level 0)
- `*(dst + [0xc978a4]) = *(src + [0xc978a4])` (0x7c6aca/0x7c6ad1, shallow pointer copy); returns EAX = dst.
- librw: skin.cpp:copySkinAtm matches.

## 0x7C6AE0 _rpSkinAtomicSetupPipelineCB  (catalogue: unknown/REA, port level 2)
- Registered as the atomic STREAM-ALWAYS callback (0x7c6874). `RwBool (RpAtomic* a, i32 offset, i32 size)`; returns 1.
- Behaviour: `type = 1`; if `RwEngineGetPluginOffset(0x120) != -1` and the BYTE at `a + RpAtomicGetPluginOffset(0x120)`
  (MatFX atomic extension, 0x7c6b07) is non-zero -> `type = 2` (0x7c6b11). If `a->geometry` (+0x18) is NULL or its
  skin extension is NULL -> return 1 (0x7c6b1b, 0x7c6b27). Else the type fix-up (identical in 0x7c6b70 and
  RpSkinAtomicSetType): if `RwEngineGetPluginOffset(0x120) == -1 && type == 2` -> `type = 1`; else if
  `RwEngineGetPluginOffset(0x12e) == -1 && type == 3` -> `type = 1` (0x7c6b29..0x7c6b57). `_rpSkinAtomicSetPipeline(a,
  type)` (0x7c6b5e). Return 1.
- librw: skin.cpp:skinAlways differs: no MatFX type selection (always type 1).

## 0x7C6B70 _rpSkinAtomicAlwaysCB  (catalogue: unknown/REA, port level 2)
- Registered as the atomic STREAM-RIGHTS callback (0x7c6883) despite the catalogue name. `RwBool (RpAtomic* a, i32 offset,
  i32 size, u32 extraData)`; `type = extraData` ([esp+0x10], read 0x7c6b89). Returns 1.
- Behaviour: geometry NULL or skin NULL -> return 1 (0x7c6b7a/0x7c6b86). Else the type fix-up described above
  (0x7c6b8d..0x7c6bbb) and `_rpSkinAtomicSetPipeline(a, type)` (0x7c6bc2); return 1.
- librw: skin.cpp:skinRights differs: librw ignores the rights data and does not check geometry/skin.

## 0x7C6D20 cb_SKINPLUGIN_RpGeometry_StreamRead  (catalogue: specified/REA, port level 5)
- Convention: `RwStream* (RwStream* s, i32 length, RpGeometry* g, i32 offset, i32 size)`; frame `sub esp, 0x214`; local
  used-bones buffer of 0x100 bytes ([esp+0x120] in the new-format path, [esp+0x24] in the old one).
- Behaviour:
  1. `g->flags & 0x1000000` (native, 0x7c6d31): `r = _rpSkinGeometryNativeRead(s, g)` 0x7c88b0 (0x7c7091); return
     `r ? s : NULL`.
  2. `RwStreamReadInt32(s, &hdr, 4)` (0x7c6d4d); fail -> NULL. `numBones = hdr & 0xff` (0x7c6d77), `numUsed = (hdr >> 8)
     & 0xff` (0x7c6d6f), `maxWeights = (hdr >> 16) & 0xff` (0x7c6d73); `numVertices = g->numVertices` (0x7c6d68).
  3. OLD format, `maxWeights == 0` (0x7c6d8e): `skin = G->freeListAlloc([0xc978b8], 0x30116)` (0x7c6da4), memset 0x40
     (0x7c6dbc; before any NULL test). `cnt = numBones` (0x7c6da0); if `numBones == 0`: `_rpSkinFindUsedBones(skin, NULL,
     NULL, buf, &cnt, numVertices)` (0x7c6dd6; with maxNumWeights still 0 it only sets cnt = 0). `_rpSkinAllocateData(skin,
     numBones, cnt, numVertices, buf, NULL, NULL, NULL)` (0x7c6df1; copies cnt bytes of the UNINITIALISED stack buffer,
     overwritten later); 0 -> freeListFree(skin), return NULL (0x7c6e0a).
  4. NEW format (0x7c6e35): same allocation (0x7c6e46) and memset; if `numUsed == 0` a no-op FindUsedBones into a
     different local (0x7c6e7b); `_rpSkinAllocateData(skin, numBones, numUsed, numVertices, buf, NULL, NULL, NULL)` (0x7c6e9d);
     0 -> free, NULL. `RwStreamRead(s, skin->usedBones, numUsed) != numUsed` -> NULL (0x7c6eea/0x7c6ef2; skin leaks).
  5. Both: `RwStreamReadInt32(s, skin->indices, numVertices*4)` (0x7c6f1d) and `RwStreamReadReal(s, skin->weights,
     numVertices*16)` (0x7c6f3f); any NULL -> return NULL (skin leaks).
  6. New format (0x7c704a): `skin->maxNumWeights = maxWeights`; `RwStreamReadReal(s, skin->matrices, numBones*64)`
     (0x7c7058); `_rpSkinSplitDataStreamRead(s, skin)` (0x7c7071); either NULL -> return NULL.
     Old format (0x7c6f62): for `b < skin->numBones`: `RwStreamSkip(s, 4)` (0x7c6f76) then `RwStreamReadReal(s, matrices+b*64,
     0x40)` (0x7c6f8f); NULL -> return NULL. Then maxNumWeights scan (same loop as RpSkinCreate, 0x7c6fb4..0x7c6fe6,
     dword `!= 0` tests) and `_rpSkinFindUsedBones(skin, indices, weights, skin->usedBones, &skin->numUsedBones,
     numVertices)` (0x7c6ff7).
  7. Attach (0x7c7006): `old = ext(g)`; if `old == skin` return s; if `old` `_rpSkinDeinitialize(g)` (0x7c7023);
     `ext(g) = skin` (0x7c7032); `_rpSkinInitialize(g)` (0x7c7035, result ignored); return s.
- librw: skin.cpp:readSkin differs: librw detects the old format by `header[1] == 0` (exe: `header[2] == 0`), allocates
  `numBones` used-bone bytes in the old format (exe: numBones, or 0 when numBones is 0), has no usedBones read-length check,
  sorts used bones via a 256-entry table (exe keeps first-seen order then qsorts in _rpSkinInitialize), never sorts
  weights, and does not call an initialise step.

## 0x7C70C0 _rpSkinFindUsedBones  (catalogue: unknown/REA, port level 0)
- Convention: `void (RpSkin* skin, u32* indices, RwMatrixWeights* weights, u8* usedOut, u32* countOut, u32 numVertices)`
  (args proven at 0x7c70c1, 0x7c70c6, 0x7c70e1, 0x7c70e6; the arg slots are reused as loop variables).
- Behaviour: `*countOut = 0` (0x7c70d4); `numVertices == 0` -> return. For `v = 0 .. numVertices-1` (unsigned), for
  `j = 0 .. skin->maxNumWeightsForVertex-1` (re-read, unsigned, 0x7c70fa): if the weight DWORD `weights[v].w[j] != 0`
  (integer test, so -0.0f counts as used, 0x7c710b): `b = (indices[v] >> (8*j)) & 0xff` (0x7c711b..0x7c7124); linear
  search `usedOut[0..*countOut-1]` (0x7c712d); not found -> `usedOut[*countOut] = b; *countOut += 1` (0x7c713b..0x7c7142).
  Output is in first-seen order (not sorted). No bound on 256 entries other than the u8 index range.
- librw: skin.cpp:Skin::findUsedBones differs (float compare, sorted table output).

## 0x7C7190 _rpSkinAllocateData  (catalogue: unknown/REA, port level 0)
- Convention: `RwBool (RpSkin* skin, u32 numBones, u32 numUsedBones, u32 numVertices, const u8* usedBones,
  const RwMatrixWeights* weights, const u32* indices, const RwMatrix* matrices)` (arg proofs 0x7c7195, 0x7c719b, 0x7c7190,
  0x7c7205, 0x7c7273, 0x7c7258, 0x7c722c); returns 1, or 0 if the allocation failed (EAX = 0 from 0x7c71c8).
- Behaviour: `size = numBones*64 + numVertices*20 + numUsedBones + 0xf` (0x7c71a5..0x7c71b4); `skin->data =
  G->rwmalloc(size, 0x30116)` (0x7c71b9/0x7c71cc); NULL -> return 0. `memset(data, 0, size)` (0x7c71da/0x7c71e1).
  `skin->numBones = numBones` (0x7c71ee), `numUsedBones` (0x7c71f4), matrices/indices/weights/usedBones pointers as in the
  data block layout (0x7c7200..0x7c7213). Copies, in this order: usedBones (numUsedBones bytes, only if src != NULL and
  count != 0, 0x7c7223); matrices: if src != NULL and `skin->numBones != 0`, 0x40 bytes per matrix from the LAST to the
  first (0x7c723f..0x7c7256); indices (`numVertices*4` bytes, 0x7c726a); weights (`numVertices*16` bytes, 0x7c728a).
- librw: skin.cpp:Skin::init differs: librw leaves pointers NULL for zero counts and copies nothing.

## 0x7C7520 RpSkinAtomicSetHAnimHierarchy  (catalogue: specified/REA, port level 0)
- `RpAtomic* (RpAtomic* a, RpHAnimHierarchy* h)`: `*(a + [0xc978a4]) = h` (0x7c752e); returns a. librw: field store, matches.

## 0x7C7540 RpSkinAtomicGetHAnimHierarchy  (catalogue: specified/REA, port level 0)
- `RpHAnimHierarchy* (RpAtomic* a)`: return `*(a + [0xc978a4])` (0x7c754a). librw: matches.

## 0x7C7550 RpSkinGeometryGetSkin  (catalogue: specified/REA, port level 0)
- `RpSkin* (RpGeometry* g)`: return `*(g + [0xc978a8])` (0x7c755a). librw: Skin::get matches.

## 0x7C7560 RpSkinGeometrySetSkin  (catalogue: specified/REA, port level 1)
- `RpGeometry* (RpGeometry* g, RpSkin* skin)`: `old = ext`; if `skin == old` return g (0x7c7574). If `old`
  `_rpSkinDeinitialize(g)` (0x7c757b; the old skin is NOT destroyed). `ext = skin` (0x7c758b). If `skin != NULL` and
  `_rpSkinInitialize(g) == 0` -> return 0 (EAX from the call, 0x7c759b); else return g.
- librw: absent (librw writes the pointer directly).

## 0x7C75B0 RpSkinCreate  (catalogue: specified/REA, port level 1)
- Convention: `RpSkin* (u32 numVertices, u32 numBones, RwMatrixWeights* weights, u32* indices, RwMatrix* skinToBone)`
  (args at [esp+0x118..0x128] after the prologue, proofs 0x7c75e6, 0x7c76a6, 0x7c7605, 0x7c7644, 0x7c76f9).
  Frame: `sub esp, 0x104` = count (4) + used-bones buffer (0x100). Caller: CClothesBuilder::CreateSkinnedClump 0x5a69d0.
- Behaviour: 1. `skin = G->freeListAlloc([0xc978b8], 0x30116)` (0x7c75d3); memset 16 dwords (0x7c75e4; no NULL test).
  2. `skin->maxNumWeightsForVertex = 1` (0x7c75f2). For `v < numVertices` (unsigned, 0x7c762f): `m = skin->maxNumWeights`;
     if `m < 4`: for `j = m .. 3`: if the DWORD `weights[v].w[j] == 0` break (0x7c7612); `skin->maxNumWeights += 1`; if it
     became 4, stop the whole scan (0x7c7623). (weights == NULL with numVertices > 0 crashes here.)
  3. `_rpSkinFindUsedBones(skin, indices, weights, localBuf, &cnt, numVertices)` (0x7c764f).
  4. `size = numBones*64 + numVertices*20 + cnt + 0xf` (0x7c7665..0x7c7673); `data = G->rwmalloc(size, 0x30116)` (0x7c767c);
     `skin->data = data`; NULL -> `G->freeListFree([0xc978b8], skin)` (0x7c7788), return NULL.
  5. memset(data, 0, size) (0x7c769d); numBones (0x7c76b7), numUsedBones = cnt (0x7c76bd), pointers (0x7c76c9..0x7c76dc);
     copy cnt used bones from the local buffer (0x7c76f0); if `skinToBone != NULL && numBones != 0` copy matrices last to
     first (0x7c770f..0x7c7729); if `indices` copy `numVertices*4` bytes (0x7c773e); if `weights` copy `numVertices*16`
     bytes (0x7c7764). Return skin. Split data stays zero; used bones are NOT sorted here.
- librw: absent (librw has no RpSkinCreate equivalent besides Skin::init).

## 0x7C77E0 RpSkinGetNumBones  (catalogue: specified/REA, port level 0)
- `u32 (RpSkin* s)`: return `s->numBones` (+0x0, 0x7c77e4). librw: field.

## 0x7C77F0 RpSkinGetVertexBoneWeights  (catalogue: specified/REA, port level 0)
- `RwMatrixWeights* (RpSkin* s)`: return `s+0x18` pointer (0x7c77f4). librw: field.

## 0x7C7800 RpSkinGetVertexBoneIndices  (catalogue: specified/REA, port level 0)
- `u32* (RpSkin* s)`: return `s+0x14` pointer (0x7c7804). librw: field.

## 0x7C7810 RpSkinGetSkinToBoneMatrices  (catalogue: specified/REA, port level 0)
- `RwMatrix* (RpSkin* s)`: return `s+0xc` pointer (0x7c7814). librw: field (inverseMatrices).

## 0x7C7830 RpSkinAtomicSetType  (catalogue: specified/REA, port level 2)
- `RpAtomic* (RpAtomic* a, i32 type)`: type fix-up (MatFX missing and type 2 -> 1, 0x7c7847; else normal-map 0x12e
  missing and type 3 -> 1, 0x7c785e) then return `_rpSkinAtomicSetPipeline(a, type)` (0x7c786e; returns a).
- librw: absent.

## 0x7C78A0 _rpD3D9SkinVertexShaderMatrixUpdate  (catalogue: specified/REA, port level 4)
- Convention: `void (u8* cache, RpAtomic* a, RpSkin* skin)`; frame `sub esp, 0xc4`. Caller: _rpD3D9SkinRenderVS 0x7c8060.
  Locals (after the 4 pushes): `R` matrix [0x14], `T` matrix [0x54], `IW` inverse world [0x94].
- Behaviour: `h = *(a + [0xc978a4])` (0x7c78b4); NULL -> return. Cache test: if `[0xc978d4] == h` and `[0xc978d8] ==
  ~(u32)(u16)G->renderFrame` (G+8, 0x7c78d9..0x7c78df) and `[0xc978dc] == a->frame` (+4) -> return (already done this
  frame). Else store the three keys (0x7c78f6..0x7c7913). `stb = RpSkinGetSkinToBoneMatrices(skin)` (0x7c7918).
  Output per bone i: 0x30 bytes at `cache + 0x30*i`, the transposed upper 3x4 of R: row0 = (R.right.x, R.up.x, R.at.x,
  R.pos.x), row1 = (.y components), row2 = (.z components) (stores 0x7c7998..0x7c79e3). Loops run `i = 0 .. h->numNodes-1`
  (re-read, signed).
  1. `h->flags & 2` (NOMATRICES, 0x7c7924): `IW = _rwD3D9VSGetInverseWorldMatrix(&IW)` (0x7c793b); per node:
     `T = stb[i] * LTM(h->pNodeInfo[i].pFrame)` (RwFrameGetLTM 0x7c7964, RwMatrixMultiply(&T, stb+i, ltm) 0x7c7970);
     `R = T * IW` (0x7c7987).
  2. else `m = h->pMatrixArray`; if `h->flags & 0x4000` (LOCALSPACE, 0x7c7a0d): `R = stb[i] * m[i]` (0x7c7a45), no
     inverse world.
  3. else `IW` as in 1 (0x7c7ac1); `T = stb[i] * m[i]` (0x7c7af8); `R = T * IW` (0x7c7b0f).
- librw: d3d/d3d9skin.cpp:uploadSkinMatrices differs: no per-frame cache keys, librw always multiplies by the inverse
  atomic LTM it computes itself and handles no NOMATRICES case.

## 0x7C7B90 _rpD3D9SkinAllInOneNode  (catalogue: specified/REA, port level 6)
- Convention: RxNodeBody `RwBool (RxPipelineNode* self, const RxPipelineNodeParam* params)`; `a = params->dataParam`
  (0x7c7b9d); `pd = self->privateData` (+0x14, 0x7c7b9f); `g = a->geometry`. Returns 1, or 0 when instancing failed.
- Behaviour:
  1. `g->numVertices <= 0` (0x7c7bb4) or `g->mesh->numMeshes == 0` (0x7c7bbd) -> return 1.
  2. `res = (g->numMorphTargets != 1) ? a->repEntry : g->repEntry` (0x7c7bc8..0x7c7be4). If `res` and
     `header.serialNumber (res+0x18) != g->mesh->serialNum` (u16 +6) -> `RwResourcesFreeResEntry(res)`, res = NULL (0x7c7bf8).
  3. `res == NULL`: owner/ownerRef = (a, &a->repEntry) or (g, &g->repEntry) by the same morph test;
     `res = _rpD3D9SkinInstanceMeshes(a, owner, ownerRef, g->mesh)` (0x7c7c31); NULL -> return 0; `g->lockedSinceLastInst = 0`
     (0x7c7c4d). Else if `res->link.next != NULL` (0x7c7c57): unlink res and insert it at the head of the list whose head
     pointer is `*(G + [0xc9bc58] + 0x24)` (resources module "used" list, inlined RwResourcesUseResEntry, 0x7c7c5b..0x7c7cb1).
  4. `skin = ext(g)`, `h = ext(a)` (0x7c7cbe/0x7c7cc1). SOFTWARE block if `skin && skin->useHW (+0x20) == 0 && h`
     (0x7c7cca..0x7c7cdd):
     a. `hdr = res + 0x18` (res re-read by the morph test); `srcPos = g->morphTarget->verts`; `srcNrm = (g->flags & 0x10) ?
        morphTarget->normals : NULL` (0x7c7cfd..0x7c7d0f); `locked = NULL`.
     b. If `0x7c89c0(a, hdr) != 0` (0x7c7d21; returns 1 when `[0xc978e0] == 0`, or when VS present, skin present and
        `skin+0x1c <= [0xc978e8]`): if `hdr->stream[0].managed == 0` -> `_rpD3D9SkinGeometryInstanceVS(a, hdr, 1)` and
        `g->lockedSinceLastInst = 0xfff` (0x7c7d38/0x7c7d40); `vb = stream[0].vb`, `stride = stream[0].stride`;
        `vb->Lock(stream[0].offset, g->numVertices*stride, &locked, 0x800)` (IDirect3DVertexBuffer9 vtbl+0x2c, 0x7c7d68);
        go to d (no geometryFlags merge).
     c. Else: if `stream[0].managed != 0` -> InstanceVS(a, hdr, 1) and lockedSinceLastInst = 0xfff (0x7c7d78/0x7c7d80).
        If `vb != NULL && offset == 0 && dynamicLock == 0`: `vb->Lock(0, 0, &locked, 0x2800)` (0x7c7db3). Else
        `RwD3D9DynamicVertexBufferLock(stride, g->numVertices, &vb, &locked, &base)` (0x7c7dcc); `stream[0].vb = vb`,
        `stream[0].offset = base*stride` (0x7c7ddc/0x7c7de5); if `hdr->useOffsets == 0`: for every instance (do-while on
        numMeshes) `inst.baseIndex = inst.minVert + offset/stride` (unsigned div, 0x7c7df5..0x7c7e0e).
        Then `g->lockedSinceLastInst |= stream[0].geometryFlags` (0x7c7e14).
     d. If `locked != NULL` (0x7c7e1c): `dstPos = locked`, `dstNrm = srcNrm ? locked + 0xc : NULL`. If `[0xc980a4] != 0`:
        `mats = 0x7caa30(a, skin, h)` (cat. _rpSkinPrepareMatricesVS) and dispatch on `skin->maxNumWeights`: `> 2` ->
        0x7cac60, `== 2` -> 0x7caf50 (both `(numVertices, weights, indices, mats, dstPos, srcPos, dstNrm, srcNrm, stride)`),
        `<= 1` and `numUsedBones == 1` -> 0x7caad0 `(numVertices, mats + usedBones[0]*64, dstPos, srcPos, dstNrm, srcNrm,
        stride)`, else 0x7cab80 `(numVertices, indices, mats, dstPos, srcPos, dstNrm, srcNrm, stride)` (0x7c7e4f..0x7c7f06).
        Else (0x7c7f10): `mats = 0x7ca850(a, skin, h)` and the same dispatch with 0x7c9da0 / 0x7ca410 / 0x7ca330 / 0x7ca240.
        Then `vb->Unlock()` (vtbl+0x30, 0x7c7fca).
     e. `g->lockedSinceLastInst &= ~6` (byte AND 0xf9, 0x7c7fcd).
  5. If `g->lockedSinceLastInst != 0` (0x7c7fe1): `_rpD3D9SkinGeometryInstance(a, res+0x18, h, lockedSinceLastInst)`
     (0x7c7ff2); `lockedSinceLastInst = 0` (0x7c7ffe).
  6. `a->interpolator.flags (+0x4c) &= ~1` (0x7c8007).
  7. If `skin && skin->useHW && h`: `_rpD3D9SkinRenderVS(pd + 8, a, g, skin, h)` (0x7c8028). Else
     `0x7c85b0(pd, res, a, g->flags)` (cat. _rpD3D9SkinRenderSoftware, 0x7c8049). Return 1.
- librw: d3d/d3d9skin.cpp differs: librw has no software-skin path, no resource-list reuse, no 0x7c89c0 test; instancing
  and render are separate callbacks.

## 0x7C8060 _rpD3D9SkinRenderVS  (catalogue: unknown/REA, port level 5)
- Convention: `void (RxD3D9VSCallbacks* cb, RpAtomic* a, RpGeometry* g, RpSkin* skin, RpHAnimHierarchy* h)`; `cb` = node
  private data + 8: +0 begin (descriptor), +4 lighting, +8 getMaterialShader, +0xc meshRender, +0x10 end (see 0x7cb240).
  Locals: `desc` (_rpD3D9VertexShaderDescriptor, 4 bytes) [0x14], `disp` dispatch descriptor [0x44] (byte +0 material
  shader id, +1 fog id, +4 bone constant base), caches: `lastBones` u8 [0x11] = 0xff, `lastMat` u8 [0x12] = 0xff, `lastFog`
  u8 [0x13] = 0xff (0x7c8087..0x7c8091), `lastAmbient` f32 [0x18] = 1.0f, `lastDiffuse` f32 [0x24] = 1.0f (0x7c8077/0x7c807f),
  `lastColor` [0x28].
- Behaviour:
  1. `cb->begin(a, 1, &desc)` (0x7c8096). `desc.numWeights (byte2 bits 1..3) = skin->maxNumWeights` ((m*2 ^ b) & 0xe ^ b,
     0x7c80a7..0x7c80b3). `RwD3D9SetRenderState(0x88 D3DRS_CLIPPING, 1)` (0x7c80b7).
  2. `_rwD3D9VSSetActiveWorldMatrix(RwFrameGetLTM(a->frame))` (0x7c80c0/0x7c80c6); `_rwD3D9VSGetComposedTransformMatrix(
     0xc970a0)` (0x7c80d0); `end = cb->lighting(a, 1, 0xc970e0, &desc)` (0x7c80e2); `dev->SetVertexShaderConstantF(0,
     0xc970a0, (end - 0xc970a0) >> 4)` (IDirect3DDevice9 vtbl+0x178, device 0xc97c28, 0x7c8101).
  3. `_rpD3D9SkinVertexShaderMatrixUpdate([0xc978ac], a, skin)` (0x7c810f).
  4. `hdr = ((g->numMorphTargets == 1) ? g->repEntry : a->repEntry) + 0x18` (0x7c811b..0x7c8129). If `hdr->indexBuffer` and
     `!= [0x8e2450]` (last index buffer): store and `dev->SetIndices(ib)` (vtbl+0x1a0, 0x7c8150).
     `_rwD3D9SetStreams(&hdr->stream[0], hdr->useOffsets)` (0x7c815e). If `hdr->vertexDeclaration != [0x8e2444]`: store,
     `[0x8e2440] = -1` (last FVF), `dev->SetVertexDeclaration(decl)` (vtbl+0x15c, 0x7c818e).
  5. `rle = skin->meshBoneRLE` (+0x38), `rleCount = skin->meshBoneRLECount` (+0x34), `lastColor = inst[0].material->color`
     (0x7c8194..0x7c81b4). For each instance `inst` (numMeshes, do-while from 0x7c81c2; skipped if numMeshes == 0):
     a. `mat = inst->material`; `_rwD3D9RenderStateVertexAlphaEnable(mat->color.alpha != 0xff || inst->vertexAlpha)`
        (0x7c81c5..0x7c81e1).
     b. `inst->vertexShader = cb->getMaterialShader(mat, &desc, &disp)` (0x7c81f8). While
        `_rpD3D9GetNumConstantsUsed(&desc) + 3*skin->+0x1c > [0xc978ec]` (0x7c8203..0x7c8219, re-test 0x7c827d..0x7c8295):
        reduce in this priority: numSpotLights (byte1 low nibble) -1; else numPointLights (byte0 high nibble) -1; else
        numDirectionalLights (byte0 low nibble) -1; else fogMode (byte3 bits 0..1) = 0; else stop. If any reduction loop ran,
        re-query the shader (0x7c82a6) and set `lastAmbient = 1.0[0x858624] - mat->surfaceProps.ambient` (x87 fsub, 0x7c82a9..
        0x7c82b8) to force the light update.
     c. If `lastAmbient != mat->ambient` (+0xc) or `lastDiffuse != mat->diffuse` (+0x14) (fcomp; unordered counts as different,
        0x7c82c0..0x7c82d6): `_rpD3D9VertexShaderUpdateLightsColors(0xc970e0, &desc, mat->ambient / lastAmbient,
        mat->diffuse / lastDiffuse)` (x87 f32 divisions, 0x7c82d8..0x7c82fa); then copy both values into the caches.
     d. If `disp.byte0 != lastMat` or `lastColor != mat->color`: update both caches and
        `_rpD3D9VertexShaderUpdateMaterialColor(&mat->color, &disp)` (0x7c833a). If `disp.byte1 != lastFog`: update and
        `_rpD3D9VertexShaderUpdateFogData(&desc, &disp)` (0x7c835c).
     e. Bone constants (register base `B = disp.byte4`):
        - split skin (`rleCount != NULL`, 0x7c8368): `lastBones = B`; `start = rleCount[0]`, `n = rleCount[1]`, rleCount += 2;
          for each of the n runs `(first, count) = rle[(start + k)*2 .. +1]`: for `bone = first .. first+count-1`:
          `SetVertexShaderConstantF(B + 3*remap[bone], cache + bone*0x30, 3)` (remap = skin+0x30, 0x7c83cc..0x7c83ec).
        - else if `B != lastBones` (0x7c8422): `lastBones = B`; if `h->numNodes > [0xc978e8]`: runs of consecutive entries of
          `usedBones` (start s, end e exclusive): `SetVertexShaderConstantF(B + 3*s, cache + used[s]*0x30, 3*(e-s))`
          (0x7c84e0..0x7c854a). Else if `skin->numUsedBones >= h->numNodes`: one call `(B, cache, 3*numNodes)` (0x7c84d8).
          Else runs as above but register `B + 3*used[s]` (0x7c8456..0x7c84bd).
     f. `cb->meshRender(hdr, inst, &desc, &disp)` (0x7c8568); `inst->vertexShader = 0` (0x7c856f).
  6. `cb->end(a, 1, &desc)` (0x7c859b). No defined return value.
- librw: d3d/d3d9skin.cpp:skinRenderCB differs: one fixed shader, no light-count reduction, no RLE/split bone upload, no
  material/fog caches.

## 0x7C8720 _rpSkinBoneIndexCompare  (catalogue: unknown/REA, port level 0)
- `int (const u8* a, const u8* b)`: return `(int)*a - (int)*b` (zero-extended bytes, 0x7c8728..0x7c8730). qsort comparator.
- librw: absent.

## 0x7C8740 _rpSkinInitialize  (catalogue: specified/REA, port level 0)
- Convention: `RpGeometry* (RpGeometry* g)`; returns g.
- Behaviour: `skin = ext(g)`. If NULL, execution still reaches 0x7c87f4 and reads `skin->usedBones` -> NULL dereference
  (original bug; callers never pass a skinless geometry). For `v = 0 .. g->numVertices-1` (signed, 0x7c8764): w = &weights[v],
  idx bytes = &indices[v]. If the DWORD `w0 >= 0x3f800000` (signed integer compare against 1.0f, 0x7c876e) skip the vertex.
  Otherwise bubble sort descending using SIGNED INTEGER compares of the float bit patterns: pass: `sw = 0`; if `w0 < w1`
  swap (w0,w1) and idx bytes 0/1, `sw = 1` (0x7c877a..0x7c8795); if `w1 < w2` swap, `sw = 1` (0x7c879d..0x7c87b8); if
  `w2 < w3` swap and restart immediately (0x7c87c0..0x7c87db); else restart if `sw` (0x7c87df). The 1.0 test is not
  repeated on restart. The moved float goes through x87 fld/fstp (value preserved).
  Then if `skin->usedBones != NULL`: `qsort(usedBones, numUsedBones, 1, 0x7c8720)` (0x8247e0, 0x7c8807). Return g.
- librw: absent (librw never sorts).

## 0x7C8820 _rpSkinDeinitialize  (catalogue: specified/REA, port level 0)
- `RpGeometry* (RpGeometry* g)`: returns g (0x7c8820), no other effect. librw: absent.

## 0x7C8980 _rpSkinPipelinesCreate  (catalogue: specified/REA, port level 6)
- `RwBool (u32 type)`: only bit 0 is tested (`test al, 1`, 0x7c8985): if set, `p = _rpD3D9SkinPipelineCreate()` (0x7c898e),
  `[0xc978c4] = p` (0x7c8999), return `p != NULL`; if bit 0 is clear return 1 without creating anything. The MatFX/normal-map
  bits (2, 4) never create separate pipelines.
- librw: skin.cpp skinOpen differs (librw creates per-platform pipelines unconditionally).

## 0x7C89B0 _rpSkinAtomicSetPipeline  (catalogue: specified/REA, port level 0)
- `RpAtomic* (RpAtomic* a, i32 type)`: `a->pipeline (+0x6c) = [0xc978c4]` (0x7c89ba); `type` ignored; returns a.
- librw: skin.cpp:Skin::setPipeline matches (type ignored there too).

## 0x7C8A00 _rpD3D9SkinUseHardware  (catalogue: unknown/REA, port level 0)
- `RwBool (RpAtomic* a)`: return 1 iff `[0xc978e0] != 0 && [0xc978e4] != 0 && skin = ext(a->geometry) != NULL &&
  skin->+0x1c <= [0xc978e8]` (unsigned, 0x7c8a2f); else 0. Called by _rpD3D9SkinInstanceMeshes to set skin+0x20.
- librw: absent.

## 0x7C8B10 _rpSkinSplitDataDestroy  (catalogue: specified/REA, port level 0)
- `RwBool (RpSkin* s)`: if `s->meshBoneRemapIndices` (+0x30) `G->rwfree(it)` (0x7c8b25); zero +0x24, +0x28, +0x2c, +0x30, +0x34,
  +0x38 (0x7c8b2e..0x7c8b3d); return 1. librw: absent (destroySkin frees the block).

## 0x7C8BE0 _rpSkinSplitDataStreamRead  (catalogue: unknown/REA, port level 3)
- Convention: `RwStream* (RwStream* s, RpSkin* skin)`; locals boneLimit, numMeshes, numRLE.
- Behaviour: three `RwStreamReadInt32(s, &x, 4)`: boneLimit (0x7c8bfa), numMeshes (0x7c8c16), numRLE (0x7c8c32); any NULL ->
  return NULL. `numMeshes <= 0` (signed) -> return s (split fields untouched, 0x7c8c4e). Else: free an existing remap block
  (0x7c8c78); zero +0x24..+0x38 (0x7c8c88..0x7c8c96); `size = numBones + 2*(numMeshes + numRLE)` (0x7c8c85/0x7c8c9f);
  `p = G->rwmalloc(size, 0x30116)` (0x7c8cac); `skin+0x30 = p`; NULL -> return 0. memset(p, 0, size). `+0x24 = boneLimit`,
  `+0x28 = numMeshes`, `+0x34 = p + numBones`, `+0x38 = p + numBones + 2*numMeshes`, `+0x2c = numRLE` (0x7c8cdd..0x7c8cf1).
  `RwStreamRead(s, p, numBones + 2*(numRLE + numMeshes))` (0x7c8d0a); a ZERO result -> `G->rwfree(skin + 0x24)` (frees the
  address of the split struct inside the freelist block, NOT p: original bug, 0x7c8d1b) and return NULL; a short non-zero
  read is accepted. Return s.
- librw: skin.cpp:readSkinSplitData differs: librw tests `numMeshes` non-zero, reads three separate arrays, no free bug.

## 0x7C8D60 _rpD3D9SkinInstanceMeshes  (catalogue: unknown/REA, port level 4)
- Convention: `RwResEntry* (RpAtomic* a, void* owner, RwResEntry** ownerRef, RpMeshHeader* mh)` (arg proofs 0x7c8d66,
  0x7c8d6c, 0x7c8d74, 0x7c8fce). Returns the resEntry (never NULL-checked).
- Behaviour:
  1. `res = RwResourcesAllocateResEntry(owner, ownerRef, mh->numMeshes*0x24 + 0x40, 0x4c9990)` (0x7c8d8b). `hdr = res+0x18`;
     memset(hdr, 0, size) (0x7c8da2). `hdr->serialNumber = mh->serialNum` (0x7c8dbb), `numMeshes` (0x7c8dc1),
     `indexBuffer = 0`, `totalNumIndex = sum(mesh[i].numIndices)` (meshes at mh+0x10, stride 0xc, 0x7c8de3..0x7c8dec).
  2. If `totalNumIndex != 0` and `RwD3D9IndexBufferCreate(totalNumIndex, &hdr->indexBuffer)` (0x7c8df7) succeeds:
     `ib->Lock(0, 0, &dst, 0)` (IDirect3DIndexBuffer9 vtbl+0x2c, 0x7c8e12).
  3. `hdr->primType = [0x882d74 + 4*k]` with `k = byte at G + [0xc9b8c0] + 8 + (mh->flags & 0xff)` (0x7c8e15..0x7c8e3f;
     table 0x882d74 = {0, 2, 3, 4, 5, 6, 1}); both vertex streams cleared (0x7c8e42..0x7c8e55); `vertexDeclaration = 0`.
  4. Per mesh (i, `inst = hdr + 0x40 + 0x24*i`): `_rpD3D9IndexRangeMinMax(indices, n, &inst->numVertices, &inst->minVert)`
     (0x756710: min and max-min+1, zeros when n == 0) (0x7c8e9a). `inst->numPrimitives` by primType (jump table 0x7c90bc):
     2 -> n>>1, 3 -> n-1, 4 -> n/3, 5 -> n-2, 6 -> n-2, else 0 (0x7c8eb8..0x7c8ee7). `material = mesh.material`, vertexShader,
     baseIndex, vertexAlpha = 0. If `dst != NULL`: `numIndex = n`, `startIndex = running` (0x7c8f0f/0x7c8f12); copy indices
     as u16, subtracting `minVert` when it is non-zero (0x7c8f29..0x7c8f42) else memcpy (0x7c8f54); if primType == 4
     `_rpD3D9SortTriangles(dst, n)` 0x756770 (0x7c8f78); `dst += n`, `running += n`. Else numIndex = startIndex = 0.
  5. If `dst != NULL` `ib->Unlock()` (vtbl+0x30, 0x7c8fcb).
  6. `skin = ext(g = a->geometry)`, `h = ext(a)`: if skin: `skin+0x1c = (skin->meshBoneRLECount != NULL) ? skin->boneLimit :
     skin->numUsedBones` (0x7c8fef..0x7c9001); `skin+0x20 = h ? _rpD3D9SkinUseHardware(a) : 0` (0x7c9009/0x7c901a).
  7. `hdr->totalNumVertex = g->numVertices` (0x7c902e); `_rpD3D9SkinGeometryInstanceVS(a, hdr, h != NULL)` (0x7c9031).
  8. `flags = 0xfff` if `skin->useHW` (skin dereferenced without NULL test, 0x7c9036); else `flags = 2`, `|4` (6) if
     g->flags & 0x10, `|8` if g->flags & 8, `|0x10 << t` for every texcoord set t; if `hdr->stream[0].managed == 0`
     `flags &= ~stream[0].geometryFlags` (0x7c9040..0x7c9094). `_rpD3D9SkinGeometryInstance(a, hdr, h, flags)` (0x7c90a5).
  9. Return res.
- librw: d3d/d3d9skin.cpp:skinInstanceCB differs (librw builds one static VB with fixed declaration, no splitting,
  no software path).

## 0x7C90D0 _rpD3D9SkinGeometryInstance  (catalogue: unknown/REA, port level 2)
- Convention: `void (RpAtomic* a, RxD3D9ResEntryHeader* hdr, RpHAnimHierarchy* h, u32 lockFlags)` (`ebp` frame, args
  [ebp+8..0x14]). Writes vertex data for the components selected by lockFlags (0xfff = all).
- Behaviour:
  1. `usage = RpD3D9GeometryGetUsageFlags(g)` (0x7c90ea). `hdr->vertexDeclaration->GetDeclaration(elems, &n)`
     (IDirect3DVertexDeclaration9 vtbl+0x10, 0x7c911d). `numStreams = 1 + max(elem.Stream)` over elements before the
     D3DDECL_END type 0x11 (0x7c912e..0x7c914a). `locked[0] = locked[1] = 0`.
  2. Lock: for `s < numStreams` with `lockFlags == 0xfff || (lockFlags & stream[s].geometryFlags)`: if `!managed &&
     offset == 0`: `lockFlags |= geometryFlags` and `vb->Lock(0, 0, &locked[s], 0x2800)` (0x7c91bb); else
     `vb->Lock(offset, stride*totalNumVertex, &locked[s], 0x800)` (0x7c91d4). Element lookups below search the array for
     (Usage, UsageIndex) with no end test.
  3. `lockFlags & 2`: POSITION -> `_rpD3D9VertexDeclarationInstV3d(type, locked[s] + off, g->morphTarget->verts,
     totalNumVertex, stride)` (0x7c923d).
  4. If `skin && skin->useHW && lockFlags == 0xfff` (0x7c924c..0x7c925f):
     - if `maxNumWeights > 1`: first clamp the skin's own weights: for every vertex with `w0 > 1.0f[0x858624]` (x87 fcomp,
       C0|C3 clear, 0x7c92b8..0x7c92cb) store `w = (1.0, 0, 0, 0)`; then BLENDWEIGHT ->
       `_rpD3D9VertexDeclarationInstWeights(type, dst, weights, totalNumVertex, stride)` (0x7c932e).
     - BLENDINDICES (0x7c934e): remap table `tab[0xff]` zeroed. Split skin (`+0x34 != NULL`): `remap = skin+0x30`; if
       `remap[0] >= h->numNodes` (signed) `remap[0] = 0` (writes skin data, 0x7c938a); for `i < h->numNodes`: if `remap[i] <
       0xff` `tab[i] = (u8)(remap[i]*3)` (0x7c93a5..0x7c93b4). Else if `h->numNodes <= [0xc978e8]`: `tab[i] = (u8)(i*3)` for
       i < numNodes (0x7c93e0). Else `tab[0] = 0` and `tab[usedBones[i]] = (u8)(i*3)` for i < numUsedBones (0x7c940f..0x7c9421).
       `_rpD3D9VertexDeclarationInstIndicesRemap(type, dst, skin->indices, tab, totalNumVertex, stride)` (0x7c9464).
  5. `g->flags & 0x10 && lockFlags & 4`: NORMAL -> InstV3d(type, dst, morphTarget->normals, numV, stride) (0x7c94c6).
  6. `usage & 0x10000 && lockFlags & 0xff2`: TANGENT -> `_rpD3D9VertexDeclarationInstTangent(type, dst, verts,
     g->texCoords[numTexCoordSets-1], hdr, stride)` (0x7c952f).
  7. `numTexCoordSets && lockFlags & 0xff0`: for each set t with `lockFlags & (0x10 << t)`: TEXCOORD t ->
     `_rpD3D9VertexDeclarationInstV2d(type, dst, g->texCoords[t], numV, stride)` (0x7c95a5).
  8. `g->flags & 8 && lockFlags & 8`: COLOR; per instance (do-while numMeshes): `inst->vertexAlpha =
     _rpD3D9VertexDeclarationInstColor(locked[s] + inst->minVert*stride + off, g->preLitLum + inst->minVert,
     inst->numVertices, stride)` (0x7c962a/0x7c962f).
  9. Unlock every stream with `locked[s] != 0` (vtbl+0x30, 0x7c9666). No defined return value.
- librw: d3d/d3d9skin.cpp:skinInstanceCB (reinstance part) differs: no weight clamping, no remap table, no tangents.

## 0x7C9680 _rpD3D9SkinGeometryInstanceVS  (catalogue: unknown/REA, port level 3)
- Convention: `RwBool (RpAtomic* a, RxD3D9ResEntryHeader* hdr, RwBool hasHierarchy)`; returns 1, or 0 when a vertex buffer
  creation failed. Builds the vertex declaration and the vertex buffers. Element array at [esp+0x28] (after pushes).
- Behaviour:
  1. `caps = RwD3D9GetCaps()`, `skin = ext(g)`, `usage = RpD3D9GeometryGetUsageFlags(g)`.
  2. Release both streams (0x7c96dd..0x7c9732): if `vb`: managed -> `RwD3D9DestroyVertexBuffer(stride, stride*totalNumVertex,
     vb, offset)` (0x7c96fa); else if `offset == 0 && dynamicLock == 0` -> `RwD3D9DynamicVertexBufferDestroy(vb)` (0x7c9710);
     in all cases vb = 0. Then offset, stride, geometryFlags, managed, dynamicLock = 0.
  3. Stream count: if `(usage & 0xfff) == 0` and NOT (`skin && skin->useHW == 0 && hasHierarchy && 0x7c89c0(a) == 0`):
     `numStreams = 1`, `stream[0].managed = 1` (0x7c9771..0x7c977a). Else `numStreams = 2`, `stream[0].managed = 0`,
     `stream[1].managed = 1`, `_rpD3D9AddDynamicGeometry(g)` (0x7c977f..0x7c9790). "dyn(bit)" below = stream 0 if
     `usage & bit` else stream numStreams-1.
  4. Declaration, hardware path (`skin && skin->useHW`, 0x7c97a0..0x7c9af1); every element: Offset = current stride of its
     stream, Method 0:
     - POSITION: stream dyn(2), FLOAT3 (2); that stream's stride = 0xc (assignment) and geometryFlags = 2 (0x7c97f9/0x7c97fc).
     - NORMAL (g->flags & 0x10): stream dyn(4); DEC3N (0xe) +4 if `caps->DeclTypes & 0x80`, else FLOAT3 +0xc; flags |= 4.
     - TANGENT (usage & 0x10000): stream 0 if `usage & 0xff2` else last; DEC3N/FLOAT3 as above; Usage 6; flags unchanged.
     - COLOR (g->flags & 8): stream dyn(8); D3DCOLOR (4) +4; Usage 0xa; flags |= 8.
     - TEXCOORD t (t < numTexCoordSets): stream dyn(0x10<<t); Type = `_rpD3D9FindFormatV2d(g->texCoords[t],
       hdr->totalNumVertex)` (0x7c9975); stride += `_rpD3D9VertexDeclarationGetSize(type)` (0x7c9992); Usage 5, UsageIndex t;
       flags |= 0x10<<t.
     - BLENDWEIGHT (skin->maxNumWeights > 1, last stream): `>= 4`: USHORT4N 0xc (+8) if DeclTypes & 0x20, else SHORT4N 0xa
       (+8) if & 8, else UBYTE4N 8 (+4) if & 2, else D3DCOLOR 4 (+4); `== 3`: DEC3N 0xe (+4) if & 0x80 else the `>= 4` chain;
       `== 2`: USHORT2N 0xb (+4) if & 0x10, SHORT2N 9 (+4) if & 4, DEC3N 0xe (+4) if & 0x80, else UBYTE4N/D3DCOLOR chain
       (0x7c99f7..0x7c9a85). Usage 1.
     - BLENDINDICES (always, last stream): maxNumWeights >= 3: UBYTE4 5 (+4) if DeclTypes & 1 else SHORT4 7 (+8); else
       SHORT2 6 (+4) (0x7c9aa6..0x7c9ae6). Usage 2.
  5. Declaration, software path (0x7c9af6..0x7c9c13): POSITION FLOAT3 in stream 0 (stride 0xc, flags 2); NORMAL FLOAT3 in
     stream 0 at offset 0xc (stride += 0xc, flags |= 4); COLOR in stream dyn(8) (+4, flags |= 8); TEXCOORD t FLOAT2 (1) in
     stream dyn(0x10<<t) (+8, flags |= 0x10<<t).
  6. Terminator {0xff, 0, 0x11, 0, 0, 0} (0x7c9c1e..0x7c9c3b); `RwD3D9CreateVertexDeclaration(elems, &hdr->vertexDeclaration)`
     (0x7c9c40; result ignored).
  7. `n = numStreams`, then while `n > 1 && stream[n-1].stride == 0` n-- (0x7c9c4c..0x7c9c66).
     - `n == 1 && stream[0].managed`: `RwD3D9CreateVertexBuffer(stride, stride*totalNumVertex, &vb, &offset)` (0x7c9c84);
       0 -> return 0. Then (label 0x7c9d5f) `useOffsets = 0` and for each instance (do-while numMeshes) `baseIndex =
       stream[0].offset / stream[0].stride + minVert` (unsigned div, 0x7c9d6c..0x7c9d80); return 1.
     - else for each stream s < n: if managed and `caps->DevCaps2 & 1`: RwD3D9CreateVertexBuffer as above (0x7c9cd0), 0 ->
       return 0. Else if `skin->useHW == 0` (skin dereferenced) and (`DevCaps2 & 1` or `n == 1`): `dynamicLock = 1` and no
       buffer (0x7c9cfb). Else `RwD3D9DynamicVertexBufferCreate(stride*totalNumVertex, &vb)` (0x7c9d06), 0 -> return 0.
       Non-created branches set `managed = 0`, `offset = 0` (0x7c9d12/0x7c9d16).
       After the loop: `n > 1`: if DevCaps2 & 1 `useOffsets = 1` (0x7c9d37); `baseIndex = minVert` for each instance
       (0x7c9d44); return 1. `n <= 1`: label 0x7c9d5f as above.
- Edge: both do-while instance loops wrap around for numMeshes == 0 (the node rejects that case before).
- librw: d3d/d3d9skin.cpp:skinInstanceCB differs: fixed FLOAT3/D3DCOLOR/FLOAT2/FLOAT4 weights/UBYTE4 layout, one static
  buffer, no caps-dependent compression, no software declaration.

## 0x7CB190 _rpD3D9SkinVertexShaderDescriptor  (catalogue: unknown/REA, port level 0)
- Convention: VS begin callback `void (RpAtomic* a, u32 type, _rpD3D9VertexShaderDescriptor* desc)`; `type` unused; the arg
  slots are reused as scratch for RwRenderStateGet results.
- Behaviour: `*(u32*)desc = 0` (0x7cb199). `g = a->geometry`; byte1 = `(byte1 & 0xf) | (g->numTexCoordSets << 4)` (numTexCoords,
  0x7cb1a8..0x7cb1b8). `g->flags & 8` -> byte2 |= 0x10 (prelit, 0x7cb1bd); `& 0x10` -> byte2 |= 0x60 (normals +
  normalizeNormals, 0x7cb1c6); `& 0x40` -> byte2 |= 0x80 (modulateMaterial, 0x7cb1cf). `RwRenderStateGet(0xe FOGENABLE, &v)`
  (G+0x24, 0x7cb1e0); if v: `RwRenderStateGet(0x10 FOGTYPE, &t)` (0x7cb1fa): t == 1 -> fogMode (byte3 bits 0..1) = 1; 2 -> 2;
  3 -> 3; else unchanged (0x7cb204..0x7cb22e).
- librw: absent.

## 0x7CB240 _rpD3D9SkinPipelineInit  (catalogue: specified/REA, port level 0)
- Node init `RwBool (RxPipelineNode* node)`; `pd = node->privateData` (+0x14): pd+0 render = 0x756df0
  (_rxD3D9DefaultRenderCallback), pd+4 lighting = 0x757400. If `[0xc978e4] != 0` (0x7cb256): pd+8 = 0x7cb190, +0xc = 0x761170,
  +0x10 = 0x761010, +0x14 = 0x761030, +0x18 = 0x761000 (VS callbacks); else those five = 0 (0x7cb287..0x7cb293). Return 1.
- librw: absent.

## 0x7CB2B0 _rpD3D9SkinPipelineCreate  (catalogue: specified/REA, port level 5)
- Convention: `RxPipeline* (void)`.
- Behaviour: `caps = RwD3D9GetCaps()`; `[0xc978e0] = caps byte +0x1e & 1` (DevCaps HWTRANSFORMANDLIGHT, 0x7cb2b5..0x7cb2bb).
  If set and `(caps->VertexShaderVersion & 0xffff) >= 0x101` and `(caps->PixelShaderVersion & 0xffff) >= 0x101`:
  `[0xc978e4] = 1`, `[0xc978ec] = caps->MaxVertexShaderConst`, `[0xc978e8] = (MaxVertexShaderConst - 12) / 3` (unsigned,
  multiply by 0xaaaaaaab and shift, 0x7cb300..0x7cb30c). Else e4, e8, ec = 0 (0x7cb314..0x7cb328).
  `p = RxPipelineCreate()` (0x7cb333); NULL -> return NULL. `p->pluginId (+0x2c) = 0x116`, `p->pluginData (+0x30) = 1`
  (0x7cb33f/0x7cb346). `lp = RxPipelineLock(p)`; NULL -> `_rxPipelineDestroy(p)`, return NULL (0x7cb376). Else
  `RxLockedPipeAddFragment(lp, NULL, 0x8ded08 /*skin node definition*/, NULL)` (0x7cb363) and `RxLockedPipeUnlock(result)`
  (0x7cb369), both results ignored; return p.
- librw: d3d/d3d9skin.cpp:makeSkinPipeline differs (no caps globals, ObjPipeline instead of RxPipeline).

## Open questions
- UNCLEAR: exact behaviour of the software blend helpers 0x7c9da0/0x7ca240/0x7ca330/0x7ca410/0x7caad0/0x7cab80/0x7cac60/
  0x7caf50, the matrix preparers 0x7ca850/0x7caa30 and the software render 0x7c85b0 (outside this work list; only the
  argument lists used at 0x7c7e4f..0x7c7fbb are specified here).
- UNCLEAR: meaning of the global [0xc980a4] tested at 0x7c7e37 (selects the 0x7caa30 + SSE-named helper set; probably the
  driver's SSE-support flag) and of [0xc9b8c0] used at 0x7c8e17 (mesh-module offset into G for the flags-to-primitive table).
