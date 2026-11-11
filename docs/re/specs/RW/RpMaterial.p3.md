# RenderWare 3.6 RpMaterial / RpMaterialList - pass-3 specs, writer A (RE-SPEC-RW-3)

Statically linked RW 3.6 world-plugin code (material.c, matlist.c). gta-reversed only has `plugin::Call*` thunks.
All `__cdecl`. Extends docs/re/specs/RW/RpMaterial.md (pass 1/2, read-only).

## Layouts used (offsets proven by the instruction addresses given)

`RpMaterial`: +0x0 RwTexture* (0x74db15), +0x4 RwRGBA color (0x74daca), +0x8 RxPipeline* (0x74d81c), +0xc
RwSurfaceProperties {ambient, specular, diffuse} (0x74daad, 0x74e0cc), +0x18 i16 refCount (0x74dac4, 0x74e282).
Material freelist `*(G + [0xc92514])` (0x74da86); material plugin registry 0x8d62a4 (0x74dae3). Default surface
properties (.data f32): `[0x8d62bc]` ambient, `[0x8d62c0]` specular, `[0x8d62c4]` diffuse, initial value 1.0f each:
`[0x8d62bc] = 1.0f`, `[0x8d62c0] = 1.0f`, `[0x8d62c4] = 1.0f`. `RwTexture` +0x54 refCount (0x74db0e). `RxPipeline`
+0x2c pluginId, +0x30 pluginData.
`RpMaterialList` {+0x0 RpMaterial** materials, +0x4 numMaterials, +0x8 space} (0x74e1fd..0x74e209).
G = `[0xc97b24]`: +0x134 rwmalloc, +0x13c rwrealloc, +0x144 freeListAlloc, +0x148 freeListFree.

## 0x74d810 cb_RIGHTTORENDER_RpMaterial_StreamWrite  (catalogue: specified/REA, port level 0)
- `RwStreamWriteInt32(s, &m->pipeline->pluginId, 4)`, then `(&pipeline->pluginData, 4)` (0x74d824/0x74d83d; no NULL test);
  returns ok ? s : NULL.
- librw: absent (no right-to-render on materials in librw).

## 0x74d850 cb_RIGHTTORENDER_RpMaterial_StreamGetSize  (catalogue: specified/REA, port level 0)
- `m->pipeline && pipeline->pluginId` -> 8, else 0 (0x74d854..0x74d862). librw: absent.

## 0x74d870 _rpMaterialSetDefaultSurfaceProperties  (catalogue: specified/REA, port level 0)
- `void (const RwSurfaceProperties* sp)`: `sp == NULL` -> `[0x8d62bc] <- 1.0f`, `[0x8d62c4] <- 1.0f`, `[0x8d62c0] <- 1.0f`
  (0x74d878..0x74d88c); else copy ambient, specular, diffuse (0x74d897..0x74d8ab).
- librw: absent (librw uses fixed defaults).

## 0x74da80 RpMaterialClone  (catalogue: specified/REA, port level 3)
- `RpMaterial* (const RpMaterial* src)`: `m = freeListAlloc(materialFL, 0x30007)`; NULL -> NULL. Inline create: `refCount =
  1`, `color = 0xffffffff`, `texture = NULL`, `pipeline = NULL`, `surfaceProps = defaults`, `_rwPluginRegistryInitObject(
  0x8d62a4, m)` (0x74daab..0x74dafb). Inline SetTexture: if `src->texture`: `tex->refCount++`, destroy `m->texture` if any,
  `m->texture = tex` (0x74db08..0x74db24). `m->color = src->color`, `m->pipeline = src->pipeline`, `m->surfaceProps =
  src->surfaceProps` (0x74db26..0x74db4e); `_rwPluginRegistryCopyObject(0x8d62a4, m, src)` (0x74db51, result ignored);
  return m. The destroy path at 0x74db60 is unreachable (`&src->surfaceProps` is never NULL).
- librw: material.cpp:Material::clone matches in content.

## 0x74dcc0 _rpMaterialChunkInfoRead  (catalogue: specified/REA, port level 0)
- `info* (RwStream* s, info* out /*0x1c*/, i32* size)`: `RwStreamFindChunk(s, 1, &len, NULL)` fail -> NULL; zero 7 dwords;
  `RwStreamRead(s, out, len)` (len unchecked against 0x1c: overflow possible) `!= len` -> NULL; `*size = len + 0xc`; if
  `len <= 0x10` (old files without surface properties): `out+0x10/+0x14/+0x18 = 1.0f` (0x74dd13..0x74dd25); return out.
- librw: material.cpp reads the struct the same way (librw checks version instead of length).

## 0x74e010 RpMaterialStreamGetSize  (catalogue: specified/REA, port level 0)
- `(texture ? RwTextureStreamGetSize(tex) + 0x34 : 0x28) + _rwPluginRegistryGetSize(0x8d62a4, m) + 0xc` (0x74e016..0x74e03d).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74e050 RpMaterialStreamWrite  (catalogue: specified/REA, port level 1)
- `const RpMaterial* (const RpMaterial* m, RwStream* s)`: `Header(s, 7 MATERIAL, size as 0x74e010, 0x36003, 0xffff)`;
  `Header(s, 1, 0x1c)`; raw `RwStreamWrite` of 0x1c bytes `{flags = 0, color, unused, textured = texture != NULL, ambient,
  specular, diffuse}` (0x74e0c6..0x74e101) - the `unused` dword is never initialised (stack garbage is written; kept);
  if texture `RwTextureStreamWrite(tex, s)`; `_rwPluginRegistryWriteDataChunks(0x8d62a4, s, m)`; failures -> NULL; else m.
- librw: material.cpp:Material::streamWrite matches in layout (librw writes 0 for unused).

## 0x74e1c0 _rpMaterialListAlloc  (catalogue: specified/REA, port level 0)
- `void* (i32 n)`: `G->rwmalloc(n*4, 0x1030008)` (0x74e1d7).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74e1f0 _rpMaterialListCopy  (catalogue: specified/REA, port level 1)
- `list* (list* dst, const list* src)`: `dst->num = src->num`, `dst->space = src->space`, `dst->materials = NULL`; if
  `src->materials`: `rwmalloc(space*4, 0x1030008)`; NULL -> `RwErrorSet({2, _rwerror(0x80000013, space*4)})`, NULL. Copy
  `num` pointers, `refCount++` each (0x74e273..0x74e28c); zero the rest up to `space`; return dst.
- librw: geometry.cpp MaterialList copy matches in content.

## 0x74e2c0 _rpMaterialListSetSize  (catalogue: specified/REA, port level 0)
- `list* (list* l, i32 n)`: if `l->space < n` (signed): `p = materials ? rwrealloc(materials, n*4, 0x1030008) :
  rwmalloc(n*4, 0x1030008)`; NULL -> error (as above), NULL; `materials = p`, `space = n`. Return l. New slots are not zeroed.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74e450 _rpMaterialListStreamGetSize  (catalogue: specified/REA, port level 0)
- `s = numMaterials*4 + 0x10`; for each `i` whose material does not occur earlier in the list (backward search
  0x74e476..0x74e483): `s += RpMaterialStreamGetSize(mat) + 0xc`; returns s.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74e4b0 _rpMaterialListStreamWrite  (catalogue: specified/REA, port level 1)
- `list* (const list* l, RwStream* s)`: `Header(s, 8 MATLIST, size as 0x74e450)`, `Header(s, 1, num*4 + 4)`,
  `RwStreamWriteInt32(&num, 4)`; for each `i`: write the index of the NEAREST earlier identical material (backward search
  from `i-1`) or -1 (0x74e570..0x74e5a8); then `RpMaterialStreamWrite` for every material not seen earlier
  (0x74e5b3..0x74e5f2). Failures -> NULL; else l.
- librw: geometry.cpp:MaterialList::streamWrite matches (also the nearest earlier index).

