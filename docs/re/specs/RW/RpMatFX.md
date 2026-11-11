# RenderWare 3.6 RpMatFX (material effects plugin, id 0x120) - full specs, writer C (RE-SPEC-RW-2)

Statically linked RW 3.6 plugin code in gta_sa.exe 1.0 US. gta-reversed has only thunks (RenderWare/rw/rpmatfx.cpp),
so everything below comes from the bytes. All functions are `__cdecl`. `G` = `RwEngineInstance` (0xc97b24);
`G+0x144 freeListAlloc(fl, hint)`, `G+0x148 freeListFree(fl, p)`.

## Layouts used (offsets proven by the instruction addresses given)

Globals (.data):
| global | meaning | proof |
|---|---|---|
| `[0xc9ab74] i32 MatFXMaterialDataOffset` | material extension offset (ext = `MatFXData*`) | 0x810ade |
| `[0xc9ab84] i32` | atomic extension offset (ext = `RwBool effectsEnabled`) | 0x810b2b |
| `[0xc9ab88] i32` | world-sector extension offset (ext = RwBool) | 0x810b78 |
| `[0xc9ab7c] i32` | engine-open counter | 0x810bf0, 0x810c36 |
| `[0xc9ab80] RwFreeList*` | MatFXData freelist (0x34-byte entries) | 0x810c1c |
| `[0x8e29b8] = 0x80`, `[0x8e29bc] = 0x1` | freelist entries per block / blocks to preallocate | 0x810bfe, 0x810bf9 |
| 0xc9ab50 | static RwFreeList storage | 0x810c09 |
| `[0xc9ab8c] RxPipeline*` | MatFX atomic pipeline | 0x815eaa |
| `[0xc9ab90] RxPipeline*` | MatFX world-sector pipeline | 0x815f31 |
| `[0xc9abc4]`, `[0xc9abc8]` | saved default instance callbacks (atomic / sector) | 0x815ed4, 0x815f5a |
| `[0xc9abbc]` MaxTextureBlendStages, `[0xc9abac]` MaxSimultaneousTextures >= 2, `[0xc9abb0]` TextureOpCaps & 0x1000000 (MULTIPLYADD), `[0xc9abb4]` TextureOpCaps & 0x10 (MODULATE2X), `[0xc9abb8]`, `[0x8e29c0]`, `[0x8e29c4]` (only zeroed) | caps-derived flags | 0x815f80..0x815fd9 |
| `[0xc9abc0]` pixel shaders available; `[0xc9ab94]..[0xc9aba8]` six `IDirect3DPixelShader9*` | | 0x815ffd..0x816052 |
| `[0x858624] = 1.0f` (.rdata) | constant 1.0 | 0x8118ba |

`MatFXData` (material extension, 0x34 bytes): two effect slots of 0x18 bytes at +0x00 and +0x18, `u32 flags`
(rpMATFXEFFECT*: 0 NULL, 1 BUMPMAP, 2 ENVMAP, 3 BUMPENVMAP, 4 DUAL, 5 UVTRANSFORM, 6 DUALUVTRANSFORM) at +0x30
(0x811ce7, 0x812152). Slot type at slot+0x14 (0x811177). Slot payloads (from the stream reader and the destroyer):
- BUMPMAP (1): +0x0 frame?, +0x4 `RwTexture* bumpedTexture` (0x8118af), +0x8 `RwTexture* bumpTexture` (0x8118b2),
  +0xc `f32 coefficient` (stored negated, 0x811924), +0x10 `f32 invBumpWidth` (0x8118c0).
- ENVMAP (2): +0x0 `RwFrame* frame` (0x812607), +0x4 `RwTexture* texture` (0x8116c5), +0x8 `f32 coefficient` (0x812607,
  0x811709), +0xc `i32 fbAlpha` (0x81173d).
- DUAL (4): +0x0 `RwTexture* texture` (0x8115a3), +0x4 srcBlend (0x811dc6), +0x8 dstBlend (0x811dbe).
- UVTRANSFORM (5): +0x0, +0x4 matrix pointers (cleared, not freed, 0x810cec).

## 0x810AA0 RpMatFXPluginAttach  (catalogue: specified/REA, port level 3)
- Convention: `RwBool (void)`. Each registration result `< 0` returns 0 immediately.
- Behaviour, in order: `RwEngineRegisterPlugin(0, 0x120, Open=0x810bf0, Close=0x810bb0)` (0x810ab1; offset not stored);
  `[0xc9ab74] = RpMaterialRegisterPlugin(4, 0x120, 0x810c40, 0x810c60, 0x810d20)` (0x810ad6);
  `RpMaterialRegisterPluginStream(0x120, Read=0x811410, Write=0x8111f0, GetSize=0x811970)` (0x810afe);
  `[0xc9ab84] = RpAtomicRegisterPlugin(4, 0x120, 0x811a00, 0x811a20, 0x811a40)` (0x810b23);
  `RpAtomicRegisterPluginStream(0x120, 0x811a90, 0x811a60, 0x811ae0)` (0x810b4b);
  `[0xc9ab88] = RpWorldSectorRegisterPlugin(4, 0x120, 0x811b00, 0x811b20, 0x811b40)` (0x810b70);
  `return RpWorldSectorRegisterPluginStream(0x120, 0x811b90, 0x811b60, 0x811be0) >= 0` (setge, 0x810ba4).
- librw: matfx.cpp:registerMatFXPlugin matches (same three object types).

## 0x810BF0 cb_MATERIALEFFECTSPLUGIN_RwEngine_Open  (catalogue: specified/REA, port level 6)
- `void* (void* instance, i32, i32)`. If `[0xc9ab7c] == 0` (0x810bf7): `[0xc9ab80] = RwFreeListCreateAndPreallocateSpace(0x34,
  [0x8e29b8], 4, [0x8e29bc], 0xc9ab50, 0x40120)` (0x810c14); NULL -> return NULL (counter unchanged, 0x810c25).
  `_rpMatFXPipelinesCreate() == 0` -> return 0 (0x810c2f). Then `[0xc9ab7c] += 1` and return instance (0x810c30..0x810c3b).
- librw: matfx.cpp:matfxOpen differs (no freelist, no counter).

## 0x810C40 cb_MATERIALEFFECTSPLUGIN_RpMaterial_Ctor  (catalogue: specified/REA, port level 0)
- `*(mat + [0xc9ab74]) = NULL` (0x810c4a); returns EAX = mat. librw: createMaterialMatFX matches.

## 0x810C60 cb_MATERIALEFFECTSPLUGIN_RpMaterial_Dtor  (catalogue: specified/REA, port level 4)
- `void* (RpMaterial* m, i32, i32)`, returns m. `d = ext`; if `d`: `_rpMatFXMaterialDataDestroyEffects(d)` (0x810c73),
  `G->freeListFree([0xc9ab80], d)` (0x810c86), `ext = NULL` (0x810c94).
- librw: matfx.cpp:destroyMaterialMatFX matches in substance (rwFree instead of freelist).

## 0x810CA0 _rpMatFXMaterialDataDestroyEffects  (catalogue: specified/REA, port level 3)
- `MatFXData* (MatFXData* d)`, returns d. For both slots (`slot = d + 0x18*k`, k = 0,1; loop 0x810cb2..0x810cf5), switch
  `slot.type - 1` (jump table 0x810d0c, 5 entries):
  1 BUMPMAP: `RwTextureDestroy(slot+0x4)` if non-NULL (0x810cc9), then `RwTextureDestroy(slot+0x8)` if non-NULL (0x810ce2);
  2 ENVMAP: `RwTextureDestroy(slot+0x4)` if non-NULL; 3: nothing; 4 DUAL: `RwTextureDestroy(slot+0x0)` if non-NULL;
  5 UVTRANSFORM: `slot+0x0 = slot+0x4 = NULL` (0x810cec); other types: nothing. Then `memset(d, 0, 0x34)` (13 dwords,
  0x810d00); return d. (Pointers are not cleared individually; the memset does it.)
- librw: matfx.cpp:clearMatFX matches in substance (librw also handles type 6 only via slots).

## 0x811150 _rpMatFXMaterialFindEffect  (catalogue: specified/REA, port level 0)
- `void* (RpMaterial* m, u32 type)`: `d = ext(m)` (NOT NULL-checked); for `i = 0, 1` (u8 counter): if `d->slot[i].type
  == type` return `&d->slot[i]` (0x811177/0x811198); else return NULL (0x811189).
- librw: matfx.cpp:MatFX::getEffectIndex matches (index instead of pointer).

## 0x811410 cb_MATERIALEFFECTSPLUGIN_RpMaterial_StreamRead  (catalogue: specified/REA, port level 6)
- Convention: `RwStream* (RwStream* s, i32 length, RpMaterial* m, i32, i32)`; frame `sub esp, 0x60`. Returns s or NULL.
- Shared sub-step **ReadTex** (used 4 times, e.g. 0x811505..0x811568): `RwStreamFindChunk(s, 6 /*rwID_TEXTURE*/, NULL,
  NULL)`, 0 -> return NULL; `RwErrorGet(&e)` (clears the pending error); `t = RwTextureStreamRead(s)`; if `t == NULL`:
  `RwErrorGet(&e)` (same buffer); if `e.errorCode` is 0x80000000 or 0x16 continue with `t = NULL`; otherwise
  `RwErrorSet(&e)` and return NULL.
- Behaviour:
  1. `d = ext(m)`; NULL -> `d = G->freeListAlloc([0xc9ab80], 0x30120)` (0x81143d); NULL -> return NULL; memset 0x34; ext = d.
  2. `RwStreamReadInt32(s, &flags, 4)` (0x811473), fail -> NULL; `RpMatFXMaterialSetEffects(m, flags)` (0x81148d).
  3. For `i = 0, 1` (u8 [esp+0x10], 0x811495/0x81192f): read slot type (Int32, 0x8114a2; fail -> NULL) and switch:
     - 4 DUAL (0x8114cd): `RwStreamReadInt32(s, blend, 8)` (src, dst; 0x8114d5); read hasTexture (0x8114ed); if set ReadTex.
       If `t`: `slot = _rpMatFXMaterialFindEffect(m, 4)` (0x81157a); `t->refCount (+0x54) += 1`; destroy and clear an old
       `slot->texture`; `slot->texture = t`; `0x816280(slot, 2)`, `0x816280(slot, 1)` (stubs returning 1); `RwTextureDestroy(t)`
       (drops the extra reference). Then `RpMatFXMaterialSetDualBlendModes(m, src, dst)` (0x8115c6).
     - 2 ENVMAP (0x8115d3): `RwStreamReadReal(s, &coef, 4)`, `RwStreamReadInt32(s, &fbAlpha, 4)`, read hasTexture, ReadTex.
       If `t`: `slot = FindEffect(m, 2)`, refCount += 1, destroy old `slot+0x4`, `slot+0x4 = t`, `RwTextureDestroy(t)`
       (0x81169c..0x8116c8). Then (inline find of type 2, NULL if absent and then written through) `slot+0x8 = coef`
       (0x811709) and `slot+0xc = fbAlpha` (0x81173d / 0x811763).
     - 1 BUMPMAP (0x81176b): `slot = d + 0x18*i` (the CURRENT index, not a type search, 0x811787); `RwStreamReadReal(&coef)`,
       read hasBumped, ReadTex -> `tb`; read hasBump (0x81182c; fail -> destroy tb, NULL), ReadTex -> `t` (failure ->
       destroy tb, NULL, 0x811892). If `tb`: `slot+0x4 = tb`, `slot+0x8 = t`, `slot+0x10 = 1.0f[0x858624] / (f32)(i32)
       tb->raster->width` (x87 fild of raster+0xc, fdivr, 0x8118b5..0x8118c0); no refcount change. Else if `t`:
       `RpMatFXMaterialSetBumpMapTexture(m, t)` 0x812160 and `RwTextureDestroy(t)` (0x8118cf/0x8118d5). Else `slot+0x4 =
       slot+0x8 = NULL`. Finally (inline find of type 1) `slot1->coef (+0xc) = -coef` (fld, fchs, fstp, 0x81191e..0x811924).
     - any other type (0, 3, 5, 6, ...): nothing is read for this slot (0x8114c7).
  4. Return s (0x811942).
- librw: matfx.cpp:readMaterialMatFX differs: librw stores the bump coefficient unnegated, has no invBumpWidth, ignores
  the tolerated error codes (any NULL texture is accepted), finds slots by type for bump maps, and keeps texture refs as read.

## 0x811A00 cb_MATERIALEFFECTSPLUGIN_RpAtomic_Ctor  (catalogue: specified/REA, port level 0)
- `*(atomic + [0xc9ab84]) = 0` (0x811a0a); returns EAX = atomic. librw: createAtomicMatFX matches.

## 0x811A20 cb_MATERIALEFFECTSPLUGIN_RpAtomic_Dtor  (catalogue: specified/REA, port level 0)
- `*(atomic + [0xc9ab84]) = 0` (0x811a2a); returns atomic. librw: absent.

## 0x811A40 cb_MATERIALEFFECTSPLUGIN_RpAtomic_Copy  (catalogue: specified/REA, port level 0)
- If `*(src + off) != 0` then `*(dst + off) = 1` (0x811a55); a zero source leaves dst unchanged. Returns EAX = dst.
- librw: matfx.cpp:copyAtomicMatFX differs (copies the value, and through enableEffects sets the pipeline).

## 0x811A90 cb_MATERIALEFFECTSPLUGIN_RpAtomic_StreamRead  (catalogue: specified/REA, port level 3)
- `RwStream* (RwStream* s, i32 length, RpAtomic* a, i32, i32)`: `RwStreamReadInt32(s, &v, 4)` into the length slot (0x811a9d);
  fail -> NULL. If `v != 0` and `ext(a) == 0`: `if (_rpMatFXAtomicSetPipeline(a) != NULL) ext(a) = 1` (0x811ac9..0x811ad5).
  Return s.
- librw: matfx.cpp:readAtomicMatFX matches in substance (enableEffects).

## 0x811B00 cb_MATERIALEFFECTSPLUGIN_RpWorldSector_Ctor  (catalogue: specified/REA, port level 0)
- `*(sector + [0xc9ab88]) = 0` (0x811b0a); returns sector. librw: absent (no sector support).

## 0x811C30 RpMatFXAtomicQueryEffects  (catalogue: specified/REA, port level 0)
- `RwBool (RpAtomic* a)`: return `*(a + [0xc9ab84])` (0x811c3a). Caller CEntity::PreRender. librw: getEffects(Atomic) matches.

## 0x811C80 RpMatFXMaterialSetEffects  (catalogue: specified/REA, port level 4)
- Convention: `RpMaterial* (RpMaterial* m, u32 flags)`; returns m, or NULL when the data allocation failed.
- Behaviour: `d = ext`; NULL -> allocate from [0xc9ab80] (hint 0x30120, 0x811ca5), NULL -> return NULL (EAX 0 at 0x811cb0),
  memset 0x34, ext = d. If `flags == 0`, or `d->flags != 0 && d->flags != flags` (0x811ccf..0x811cdc):
  `_rpMatFXMaterialDataDestroyEffects(d)` (0x811cdf). `d->flags = flags` (0x811ce7). Switch `flags - 1` (table 0x811de4, 6
  entries; others -> return m):
  1 -> slot0.type = 1; 2 -> slot0.type = 2; 3 -> slot0.type = 1, slot1.type = 2 (0x811d1d/0x811d24);
  4 -> slot0.type = 4, then the DUAL slot (search by type, 0x811d45..0x811d65) gets `+0x8 = 6`, `+0x4 = 5` (0x811dbe/0x811dc6)
  and `0x816280(slot, 0xa)`, `0x816280(slot, 0xb)` (stubs); 5 -> slot0.type = 5; 6 -> slot0.type = 5, slot1.type = 4 and the
  same DUAL defaults on slot1. If the search fails the stores go to NULL+4/+8 (not reachable).
- librw: matfx.cpp:MatFX::setEffects matches the slot assignment; librw's dual defaults are set elsewhere.

## 0x812140 RpMatFXMaterialGetEffects  (catalogue: specified/REA, port level 0)
- `u32 (RpMaterial* m)`: `d = ext`; return `d ? d->flags (+0x30) : 0` (0x81214d..0x812152). librw: matches.

## 0x8125B0 RpMatFXMaterialSetEnvMapFrame  (catalogue: specified/REA, port level 0)
- `RpMaterial* (RpMaterial* m, RwFrame* f)`: inline search for the slot with type 2 (0x8125ca..0x8125e7); found ->
  `slot+0x0 = f` (0x812607); not found -> stores to address 0 (`mov [ecx], edx` with ecx = 0, 0x8125f3: crash in the
  original). Returns m.
- librw: matfx.cpp:MatFX::setEnvFrame differs (librw ignores a missing env slot).

## 0x812680 RpMatFXMaterialSetEnvMapCoefficient  (catalogue: specified/REA, port level 0)
- `RpMaterial* (RpMaterial* m, f32 coef)`: same search; `slot+0x8 = coef` (dword copy, 0x8126d8); not found -> store to
  address 8 (0x8126bf). Returns m.
- librw: matfx.cpp:MatFX::setEnvCoefficient differs (librw ignores a missing slot).

## 0x815E60 _rpMatFXPipelinesCreate  (catalogue: specified/REA, port level 5)
- Convention: `RwBool (void)`; always returns 1 (0x81605a / 0x81608f), even when a pipeline could not be created.
- Behaviour:
  1. Atomic pipeline: `p = RxPipelineCreate()`; if p: `p->pluginId = 0x120` (0x815e71); `lp = RxPipelineLock(p)`; lp ->
     `RxLockedPipeAddFragment(lp, NULL, RxNodeDefinitionGetD3D9AtomicAllInOne(), NULL)` + `RxLockedPipeUnlock` (results
     ignored); lock failure -> `_rxPipelineDestroy(p)`, p = NULL. `[0xc9ab8c] = p` (0x815eaa).
     `node = RxPipelineFindNodeByName([0xc9ab8c], nodeDef->name, NULL, NULL)` (0x815ec1; p not NULL-checked);
     `[0xc9abc4] = RxD3D9AllInOneGetInstanceCallBack(node)`; `RxD3D9AllInOneSetInstanceCallBack(node, 0x8160a0)`;
     `RxD3D9AllInOneSetRenderCallBack(node, 0x815c20)` (0x815ec9..0x815ee4).
  2. Sector pipeline: same with RxNodeDefinitionGetD3D9WorldSectorAllInOne -> `[0xc9ab90]`, saved instance callback
     `[0xc9abc8]`, instance 0x816120, render 0x815c20 (0x815eec..0x815f6a).
  3. `caps = RwD3D9GetCaps()`; `[0xc9abbc] = caps->MaxTextureBlendStages (+0x94)`; if `>= 2`: `[0xc9abac] =
     (caps->MaxSimultaneousTextures (+0x98) >= 2)` (cmp/sbb/inc, 0x815f88..0x815f92), `[0xc9abb0] = TextureOpCaps (+0x90) &
     0x1000000`, `[0xc9abb4] = TextureOpCaps & 0x10`. Else `[0xc9abac] = [0xc9abb0] = [0xc9abb4] = [0xc9abb8] = 0` and
     `[0x8e29c0] = [0x8e29c4] = 0` (.data defaults 1, 0x815fd3/0x815fd9).
  4. If `(caps->PixelShaderVersion & 0xffff) >= 0x101` (0x815feb): `[0xc9abc0] = 1`; `RwD3D9CreatePixelShader(blob, &out)`
     for blobs 0x8852f0, 0x885350, 0x8853a0, 0x8853e0, 0x885468, 0x885518 into 0xc9ab94, 0xc9ab98, 0xc9ab9c, 0xc9aba0,
     0xc9aba4, 0xc9aba8 (results ignored). Else all seven globals = 0 (0x816064..0x816088).
- librw: d3d/d3d9matfx.cpp differs (librw uses its own shaders, no caps globals).

## 0x816260 _rpMatFXAtomicSetPipeline  (catalogue: specified/REA, port level 0)
- `RpAtomic* (RpAtomic* a)`: `a->pipeline (+0x6c) = [0xc9ab8c]` (0x81626a); returns a.
- librw: matfx.cpp:MatFX::enableEffects matches (pipeline assignment).

## Open questions
- UNCLEAR: what 0x816280 was meant to do (called with render-state ids 1, 2, 0xa, 0xb at 0x8115a5, 0x8115ad, 0x811dcd,
  0x811dd5); in this build it is `mov eax, 1; ret`, so it has no effect.
