# RpMatFX (material effects, plugin 0x120): accessors, streaming, bump textures, D3D9 render (RE-SPEC-RW-3 writer B)

All `__cdecl`. Pass-1 file `RpMatFX.md` (read-only) has the full layout; summary of what this file uses:

## Layout
- `[0xc9ab74]` material extension offset (ext = `MatFXData*`, 0x8123e4), `[0xc9ab84]` atomic extension offset (RwBool enabled,
  0x811a64), `[0xc9ab88]` world-sector extension offset (RwBool, 0x811b24), `[0xc9ab7c]` engine-open counter (0x810bb5),
  `[0xc9ab80]` MatFXData freelist (0x810bc2), `[0xc9ab8c]` atomic pipeline, `[0xc9ab90]` world-sector pipeline (0x8162fd, 0x816303).
- MatFXData: two 0x18-byte effect slots at +0x00 and +0x18 (index `i*3*8`, `lea eax,[eax+eax*2]` + `*8`, 0x8123ff/0x812402), slot type
  at slot+0x14 (0x812402); +0x30 flags.
  - BUMPMAP (1): +0x0 RwFrame* (0x81235d), +0x4 bumpedTexture (0x812417), +0x8 bumpTexture (0x812467), +0xc coefficient stored
    NEGATED (0x8123bd-0x8123bf), +0x10 invBumpWidth (0x812307).
  - ENVMAP (2): +0x0 frame (0x812777), +0x4 texture (0x812727), +0x8 coefficient f32 (0x812817), +0xc fbAlpha (0x8127c7).
  - DUAL (4): +0x0 texture (0x812967), +0x4 srcBlend, +0x8 dstBlend (0x8129bc, 0x8129c5).
  - UVTRANSFORM (5): +0x0 base RwMatrix*, +0x4 dual RwMatrix* (0x812a1d, 0x812a23).
- RwTexture: +0x0 raster, +0x10 name, +0x50 filterAddressing, +0x54 refCount (0x8121dd). RwRaster: +0xc width, +0x10 height, +0x23
  cFormat (bit 0x80 MIPMAP).
- `[0x858624] = 1.0f`.
- `0x816280` is a 1-instruction stub `return 1` (the D3D9 build's no-op "set up dual render state", called with (slot, 1|2|0xa|0xb)).

### Common pattern "FIND(type)"
Every accessor below inlines `_rpMatFXMaterialFindEffect` 0x811150: `d = *(MatFXData**)(material + [0xc9ab74])`; for i = 0, 1 (u8
counter, `cmp bl, 2; jb`): if `d->slot[i].type (+0x14) == type` -> slot = `d + i*0x18`. Not found -> slot = NULL and the accessor still
reads/writes `NULL + field` (access violation in the original; asserts compiled out). The port should treat "effect absent" as a
caller error (crash) or record a divergence.

## 0x810830 sub_810830 (proposed: _rpMatFXD3D9CreateBumpedTexture)  (catalogue: unknown/REA, port level 3)
- Evidence: only caller 0x816290 (from `RpMatFXMaterialSetBumpMapTexture`); builds a new texture from (base, bump) via image mask.
- `RwTexture* (RwTexture* base /*may be NULL*/, RwTexture* bump)`. No NULL checks on any allocation.
- 0x810830-: `br = bump->raster`; `bumpImg = RwImageCreate(br.width, br.height, 32)` 0x8026e0, `RwImageAllocatePixels`,
  `RwImageSetFromRaster(bumpImg, br)` 0x804250.
- base == NULL: `baseImg` = 32-bit image of the BUMP raster size, every pixel 0xffffffff (row loop y < h, x < w, signed). Else
  `baseImg` = 32-bit image of base->raster size, `RwImageSetFromRaster(baseImg, base->raster)`.
- If the base size differs from the bump size: `tmp = RwImageCreate(baseW, baseH, 32)` + allocate; `RwImageResample(tmp, bumpImg)`
  0x80c600; destroy bumpImg; bumpImg = tmp.
- `RwImageMakeMask(bumpImg)` 0x802a20; `RwImageApplyMask(baseImg, bumpImg)` 0x802af0.
- `RwImageFindRasterFormat(baseImg, 4 rwRASTERTYPETEXTURE, &w, &h, &depth, &format)` 0x8042c0; if the source raster (base's, or bump's
  when base == NULL) has cFormat bit 0x80 -> `format |= 0x9000` (MIPMAP|AUTOMIPMAP). `r = RwRasterCreate(w, h, depth, format)`,
  `RwRasterSetFromImage(r, baseImg)` 0x804290, `tex = RwTextureCreate(r)` 0x7f37c0.
- Filter/addressing from the source texture `s` (base, or bump when base == NULL), `f = s+0x50`: `aV = (f >> 12) & 0xf`, `aU =
  (f >> 8) & 0xf`; `a = (aU == aV) ? aV : 0`; `tex+0x50 = (tex+0x50 & 0xffff0000) | (a << 12) | (a << 8) | (f & 0xff)`.
- Destroy baseImg and bumpImg. Name: interleave the two names char by char (base name first, then bump name; when base == NULL both
  sources are the bump name) into a zeroed 32-byte buffer, skipping exhausted strings, until both are exhausted or the count reaches
  >= 30 after a pair (max 31 chars); `RwTextureSetName(tex, buf)` 0x7f38a0. Returns tex.
- librw: absent (librw matfx.cpp has no bumped-texture builder).

## 0x810bb0 cb_MATERIALEFFECTSPLUGIN_RwEngine_Close  (catalogue: specified/REA, port level 3)
- `void* (void* instance, RwInt32, RwInt32)`: `if (--[0xc9ab7c] == 0) _rpMatFXPipelinesDestroy()` 0x816190 (0x810bbb); if `[0xc9ab80]` !=
  NULL: `RwFreeListDestroy`, store NULL. Returns instance.
- librw: absent.

## 0x8111a0 sub_8111a0 (proposed: _rpMatFXMaterialFindEffectConst)  (catalogue: unknown/REA, port level 0)
- Evidence: byte shape identical to `_rpMatFXMaterialFindEffect` 0x811150; called only from the material copy callback 0x810d20.
- `MatFXEffectData* (const RpMaterial* m, RwUInt32 type)`: FIND(type); returns the slot or NULL (0x8111d9).
- librw: matfx.cpp `MatFX::getEffectIndex` differs (returns an index, -1 if absent).

## 0x811970 cb_MATERIALEFFECTSPLUGIN_RpMaterial_StreamGetSize  (catalogue: specified/REA, port level 2)
- `RwInt32 (const RpMaterial* m, RwInt32, RwInt32)`: `d = ext`; `d == NULL` or `d+0x30 flags == 0` -> 0 (0x81197f, 0x811986).
- `size = 4` (flags dword); for both slots (counter 2, slot pointer `edi = slot+4`, stride 0x18): `size += 4` (type dword, 0x81199b), then
  by `slot+0x14` type: 1 BUMPMAP: `size += 4` (coefficient) `+ T(slot+0x4) + T(slot+0x8)`; 2 ENVMAP: `size += 8 + T(slot+0x4)`;
  4 DUAL: `size += 8 + T(slot+0x0)`; any other type (0, 3, 5, 6) adds nothing more (0x8119a7).
  `T(t) = t ? RwTextureStreamGetSize(t) + 0x10 : 4` (0x8119c7-0x8119ec; bool dword + chunk header).
- librw: matfx.cpp `getSizeMaterialMatFX` differs: librw handles the effect types via a switch including UVTRANSFORM (0 bytes) and
  counts the env-map coefficient/fbAlpha the same; BUMPENVMAP is not a slot type in either.

## 0x811a60 cb_MATERIALEFFECTSPLUGIN_RpAtomic_StreamWrite  (catalogue: specified/REA, port level 3)
- `RwStream* (RwStream* s, RwInt32 len, const RpAtomic* a, RwInt32 off, RwInt32 size)`: `v = *(u32*)(a + [0xc9ab84])`;
  returns `RwStreamWriteInt32(s, &v, 4)` 0x7ed460.
- librw: matfx.cpp `writeAtomicMatFX` matches.

## 0x811ae0 cb_MATERIALEFFECTSPLUGIN_RpAtomic_StreamGetSize  (catalogue: specified/REA, port level 0)
- returns `ext != 0 ? 4 : 0` (neg/sbb/and 4, 0x811aed).
- librw: matfx.cpp `getSizeAtomicMatFX` matches.

## 0x811b20 cb_MATERIALEFFECTSPLUGIN_RpWorldSector_Dtor  (catalogue: specified/REA, port level 0)
- `*(u32*)(sector + [0xc9ab88]) = 0`; returns sector (EAX = arg).
- librw: absent.

## 0x811b40 cb_MATERIALEFFECTSPLUGIN_RpWorldSector_Copy  (catalogue: specified/REA, port level 0)
- `(dst, src, off, size)`: if src ext != 0 -> dst ext = 1 (0x811b55); otherwise dst unchanged. Returns dst.
- librw: absent.

## 0x811b60 cb_MATERIALEFFECTSPLUGIN_RpWorldSector_StreamWrite  (catalogue: specified/REA, port level 3)
- Same shape as 0x811a60 with `[0xc9ab88]`: writes the sector flag as int32, returns the RwStreamWriteInt32 result.
- librw: absent.

## 0x811b90 cb_MATERIALEFFECTSPLUGIN_RpWorldSector_StreamRead  (catalogue: specified/REA, port level 3)
- `RwStream* (RwStream* s, RwInt32 len, RpWorldSector* sec, RwInt32, RwInt32)`: `RwStreamReadInt32(s, &v, 4)`; 0 -> NULL. If `v != 0`
  and the extension is still 0: `_rpMatFXWorldSectorSetPipeline(sec)` 0x816270 and, if it returned non-NULL (always), ext = 1
  (0x811bd5). Returns s.
- librw: absent.

## 0x811be0 cb_MATERIALEFFECTSPLUGIN_RpWorldSector_StreamGetSize  (catalogue: specified/REA, port level 0)
- returns `ext != 0 ? 4 : 0`.
- librw: absent.

## 0x811c00 RpMatFXAtomicEnableEffects  (catalogue: specified/REA, port level 1)
- `RpAtomic* (RpAtomic* a)`: if ext == 0: `r = _rpMatFXAtomicSetPipeline(a)` 0x816260; r == NULL -> return NULL (0x811c22, ext stays 0);
  ext = 1. Returns a (also when already enabled).
- librw: matfx.cpp `MatFX::enableEffects(Atomic*)` matches (librw sets the pipeline unconditionally).

## 0x811c40 RpMatFXWorldSectorEnableEffects  (catalogue: specified/REA, port level 1)
- Same as 0x811c00 with `[0xc9ab88]` and `_rpMatFXWorldSectorSetPipeline` 0x816270.
- librw: absent.

## 0x812160 RpMatFXMaterialSetBumpMapTexture  (catalogue: specified/REA, port level 5)
- `RpMaterial* (RpMaterial* m, RwTexture* bump)`. slot = FIND(1) (NULL path continues with slot = NULL, 0x8121a1).
- Release old: if slot+0x8 bumpTexture != NULL -> `RwTextureDestroy`, slot+0x8 = NULL (0x8121ad-0x8121b5); if slot+0x4 bumpedTexture
  != NULL -> destroy, slot+0x4 = NULL AND slot+0x10 = 0 (0x8121c8-0x8121cb).
- bump == NULL (0x8122f6): `t = m->texture (+0)`; then the invBumpWidth step below with t (crashes if the material has no texture).
- bump != NULL: slot+0x8 = bump; `bump+0x54 refCount++`. `base = m+0` texture. If `bump->raster->+0xc` width == 0 (`sete`) -> return m
  (0x8121fd, no bumped texture built). If base != NULL and `base->raster->+0xc == 0` -> return m.
  Build the name: interleave base name (if base) and bump name (0x81225f-0x812291, same rule as 0x810830; when base == NULL both
  pointers are the bump name). `d = RwTexDictionaryGetCurrent()` 0x7f3a90; slot+0x4 = NULL; if d: slot+0x4 =
  `RwTexDictionaryFindNamedTexture(d, name)` 0x7f39f0. Found -> `refCount++` (0x8122ee). Not found -> slot+0x4 = `0x816290(base,
  bump)`; NULL -> return... the function returns with EAX = 0 here (0x8122d3; the material is NOT returned, so the caller sees
  failure); else if d != NULL `RwTexDictionaryAddTexture(d, new)` 0x7f3980.
- invBumpWidth (0x8122fc): `slot+0x10 = 1.0f [0x858624] / (f32)(int)t->raster->width` (x87 `fild`, `fdivr`, fstp f32), with t = the
  bumped texture (or m->texture when bump == NULL). Returns m.
- librw: matfx.cpp `MatFX::setBumpTexture` differs: librw only stores the texture (no bumped-texture build, no dictionary lookup).

## 0x812320 RpMatFXMaterialSetBumpMapFrame  (catalogue: specified/REA, port level 0)
- `RpMaterial* (m, RwFrame* f)`: FIND(1); slot+0x0 = f (0x812372). Returns m.
- librw: absent (librw has no bump frame setter).

## 0x812380 RpMatFXMaterialSetBumpMapCoefficient  (catalogue: specified/REA, port level 0)
- `RpMaterial* (m, RwReal c)`: FIND(1); `slot+0xc = -c` (x87 `fld; fchs; fstp`, 0x8123c4-0x8123d3). Returns m.
- librw: matfx.cpp `MatFX::setBumpCoefficient` differs: librw stores c unnegated.

## 0x8123e0 RpMatFXMaterialGetBumpMapBumpedTexture  (catalogue: specified/REA, port level 0)
- FIND(1); returns slot+0x4 (0x812428).
- librw: absent.

## 0x812430 RpMatFXMaterialGetBumpMapTexture  (catalogue: specified/REA, port level 0)
- FIND(1); returns slot+0x8 (0x812478).
- librw: absent.

## 0x812480 RpMatFXMaterialGetBumpMapFrame  (catalogue: specified/REA, port level 0)
- FIND(1); returns slot+0x0 (0x8124c7).
- librw: absent.

## 0x8124d0 RpMatFXMaterialGetBumpMapCoefficient  (catalogue: specified/REA, port level 0)
- FIND(1); returns `-slot+0xc` in ST0 (`fld; fchs`, 0x812517-0x81251e).
- librw: absent.

## 0x812530 RpMatFXMaterialSetEnvMapTexture  (catalogue: specified/REA, port level 3)
- `RpMaterial* (m, RwTexture* t)`: FIND(2); `t+0x54 refCount++` (no NULL check of t, 0x81256c); if slot+0x4 != NULL -> `RwTextureDestroy`
  (0x81257b); slot+0x4 = t. Returns m. (Reference taken before releasing the old one, so setting the same texture is safe.)
- librw: matfx.cpp `MatFX::setEnvTexture` matches in effect (librw: `if(tex) tex->addRef()`).

## 0x812610 RpMatFXMaterialSetEnvMapFrameBufferAlpha  (catalogue: specified/REA, port level 0)
- `RpMaterial* (m, RwBool a)`: FIND(2); slot+0xc = a (0x812668). Returns m.
- librw: matfx.cpp `MatFX::setEnvFBAlpha` matches.

## 0x8126f0 RpMatFXMaterialGetEnvMapTexture  (catalogue: specified/REA, port level 0)
- FIND(2); returns slot+0x4.
- librw: absent.

## 0x812740 RpMatFXMaterialGetEnvMapFrame  (catalogue: specified/REA, port level 0)
- FIND(2); returns slot+0x0.
- librw: absent.

## 0x812790 RpMatFXMaterialGetEnvMapFrameBufferAlpha  (catalogue: specified/REA, port level 0)
- FIND(2); returns slot+0xc.
- librw: absent.

## 0x8127e0 RpMatFXMaterialGetEnvMapCoefficient  (catalogue: specified/REA, port level 0)
- FIND(2); returns slot+0x8 in ST0 (`fld`, 0x812825), not negated.
- librw: absent.

## 0x812830 RpMatFXMaterialSetDualTexture  (catalogue: specified/REA, port level 3)
- `RpMaterial* (m, RwTexture* t)`: FIND(4); `t+0x54++` (no NULL check); old slot+0x0 destroyed if set; slot+0x0 = t; then
  `0x816280(slot, 2)` and `0x816280(slot, 1)` (no-ops). Returns m.
- librw: matfx.cpp `MatFX::setDualTexture` matches in effect.

## 0x8128c0 RpMatFXMaterialSetDualBlendModes  (catalogue: specified/REA, port level 1)
- `RpMaterial* (m, RwBlendFunction src, RwBlendFunction dst)`: FIND(4); slot+0x4 = src, slot+0x8 = dst (0x812906, 0x812909);
  `0x816280(slot, 0xa)`, `0x816280(slot, 0xb)`. Returns m.
- librw: matfx.cpp `MatFX::setDualBlend` matches.

## 0x812930 RpMatFXMaterialGetDualTexture  (catalogue: specified/REA, port level 0)
- FIND(4); returns slot+0x0.
- librw: absent.

## 0x812980 RpMatFXMaterialGetDualBlendModes  (catalogue: specified/REA, port level 0)
- `RpMaterial* (m, RwBlendFunction* src, RwBlendFunction* dst)`: FIND(4); `*src = slot+0x4`, `*dst = slot+0x8` (no NULL checks).
  Returns m.
- librw: absent.

## 0x8129e0 RpMatFXMaterialSetUVTransformMatrices  (catalogue: specified/REA, port level 0)
- `RpMaterial* (m, RwMatrix* base, RwMatrix* dual)`: FIND(5); slot+0x0 = base, slot+0x4 = dual (pointers stored, not copied). Returns m.
- librw: absent (no UV transform effect in librw).

## 0x812a50 RpMatFXMaterialGetUVTransformMatrices  (catalogue: specified/REA, port level 0)
- `RpMaterial* (m, RwMatrix** base, RwMatrix** dual)`: FIND(5); if base != NULL `*base = slot+0x0`; if dual != NULL `*dual =
  slot+0x4` (0x812a8c-0x812aa1). Returns m (EAX = arg, never overwritten).
- librw: absent.

## 0x8160a0 _rpMatFXAtomicInstanceCallback  (catalogue: specified/REA, port level 1)
- `RwBool (void* atomic, RxD3D9ResEntryHeader* h, RwBool reinstance)`. If `reinstance == 0` (0x8160ab): for n = h+0x4 meshes (do-while
  `dec ebp; jne`: n == 0 would wrap), mesh records of 0x24 bytes starting at h+0x2c, material at record+0x1c (`edi = h + 0x48`,
  0x8160b7): if the material's MatFXData exists and its flags (+0x30) are 1 (BUMPMAP) or 3 (BUMPENVMAP) and `[0xc9abc0]` (pixel
  shaders available) != 0: `RpD3D9GeometrySetUsageFlags(geo, RpD3D9GeometryGetUsageFlags(geo) | 0x10000)` with `geo = atomic+0x18`
  (0x8160dd-0x8160ed).
- Always tail: `return [0xc9abc4](atomic, h, reinstance)` (saved default instance callback, 0x816109).
- librw: absent (librw d3d9matfx.cpp instances without usage flags).

## 0x816120 _rpMatFXWorldSectorInstanceCallback  (catalogue: specified/REA, port level 1)
- Same as 0x8160a0 but WITHOUT the reinstance test (the mesh loop always runs), using `RpD3D9WorldSectorGetUsageFlags` 0x758900 /
  `SetUsageFlags` 0x7588e0 on the sector itself, and the saved default `[0xc9abc8]`.
- librw: absent.

## 0x816190 _rpMatFXPipelinesDestroy  (catalogue: specified/REA, port level 2)
- `RwBool (void)`, returns 1 (0x816253). If `[0xc9ab8c]` -> `_rxPipelineDestroy` 0x805820, store 0; same for `[0xc9ab90]`. Then for the six pixel shaders
  `[0xc9ab94]`, `[0xc9ab98]`, `[0xc9ab9c]`, `[0xc9aba0]`, `[0xc9aba4]`, `[0xc9aba8]` in this order: if non-NULL
  `RwD3D9DeletePixelShader` 0x7facf0, store 0.
- librw: absent.

## 0x816270 _rpMatFXWorldSectorSetPipeline  (catalogue: specified/REA, port level 0)
- `RpWorldSector* (RpWorldSector* s)`: `s+0x7c pipeline = [0xc9ab90]`; returns s (EAX = arg).
- librw: absent.

## 0x816290 sub_816290 (proposed: _rpMatFXSetupBumpMapTextures)  (catalogue: unknown/REA, port level 4)
- Evidence: called only from `RpMatFXMaterialSetBumpMapTexture` 0x8122c4 with (material texture, bump texture); returns the bumped
  texture.
- `RwTexture* (RwTexture* base, RwTexture* bump)`: bump == NULL -> return base (0x8162e6). If base != NULL: if
  `RwD3D9RasterIsCompressed(base->raster)` 0x820c90 or `RwD3D9RasterIsCompressed(bump->raster)` -> NULL. If base == NULL: bump raster
  compressed -> NULL. Else `return 0x810830(base, bump)`.
- librw: absent.

## 0x8162f0 RpMatFXGetD3D9Pipeline  (catalogue: specified/REA, port level 0)
- `RxPipeline* (RpMatFXD3D9Pipeline type)`: 1 -> `[0xc9ab8c]` (atomic), 2 -> `[0xc9ab90]` (world sector), else NULL.
- librw: absent.
