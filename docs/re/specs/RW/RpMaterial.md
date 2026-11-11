# RpMaterial and RpMaterialList (RenderWare 3.6 RpWorld plugin, statically linked) - full specs, writer B (RE-SPEC-RW)

All functions are `__cdecl` (caller pops). `Engine` = `[RwEngineInstance 0xc97b24]`; `+0x134` malloc `(size, hint)`,
`+0x138` free, `+0x13c` realloc `(ptr, size, hint)`, `+0x144`/`+0x148` freelist alloc/free. Errors: `RwErrorSet({2, code})`
as in RpClump.md.

## Layouts used (verified against the bytes)

RpMaterial (registry 0x8d62a4, base 0x1c; freelist object; matches objects.md):

| off | field | proving instruction |
|---|---|---|
| +0x00 ptr | texture (RwTexture*) | 0x74d9d4, 0x74dbd1 |
| +0x04 RwRGBA | color (create: 0xffffffff) | 0x74d9da |
| +0x08 ptr | pipeline (RxPipeline*) | 0x74d9dd, used by RpGeometryUnlock 0x74c955 |
| +0x0c f32 | surfaceProps.ambient | 0x74d9f3 |
| +0x10 f32 | surfaceProps.specular | 0x74d9fa |
| +0x14 f32 | surfaceProps.diffuse | 0x74da03 |
| +0x18 i16 | refCount (1 at create) | 0x74d9ba, 0x74da25 |
| +0x1a | pad (not initialised) | - |

RpMaterialList (embedded in RpGeometry at +0x20 / RpWorld at +0x10): +0x0 `RpMaterial** materials`, +0x4 `i32 numMaterials`,
+0x8 `i32 space` (0x74e1b6..0x74e1bb, 0x74e359). RwTexture +0x54 refCount (0x74dbca), +0x0 raster.

MATERIALMODULE: offset global 0xc92514 (engine data +0 = material freelist, 0x74d91d), open counter 0xc92518, static
freelist 0xc924ec, params 0x8d62c8 = 0x100 per block, 0x8d62cc = 1 block. Default surface properties: 0x8d62bc ambient,
0x8d62c0 specular, 0x8d62c4 diffuse (initial `.data` value 1.0f each). Material rights globals (written by the
RIGHTTORENDER material reader 0x74d7d0): 0xc92510 pluginID, 0xc924e8 extra data.

## 0x74d8e0 cb_MATERIALMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- `void* (void* instance, i32 offset, i32 size)`: `[0xc92514] = offset`; freelist =
  `RwFreeListCreateAndPreallocateSpace([0x8d62a4] (material registry size), [0x8d62c8], 4, [0x8d62cc], 0xc924ec, 0x40007)`
  stored at `Engine+offset`; NULL -> return NULL. `[0xc92518]++`; return instance.

## 0x74d990 RpMaterialCreate  (catalogue: specified/REA, port level 1)
- cdecl `RpMaterial* (void)`. `m = Engine->memoryAlloc(*(Engine+[0xc92514]), 0x30007)`; NULL -> NULL. `refCount = 1`;
  `color = {255,255,255,255}`; `texture = NULL`; `pipeline = NULL`; `surfaceProps = {[0x8d62bc], [0x8d62c0], [0x8d62c4]}`
  (raw dword copies); `_rwPluginRegistryInitObject(0x8d62a4, m)`; return m.
- librw: geometry.cpp:Material::create matches (librw fixed surface props instead of the default globals).

## 0x74da20 RpMaterialDestroy  (catalogue: specified/REA, port level 3)
- cdecl `RwBool (RpMaterial* m)`; returns 1.
- If `refCount == 1` (16-bit compare): `DeInitObject(0x8d62a4, m)`; if texture: `RwTextureDestroy(texture)` (0x7f3820);
  `texture = NULL`; free to the material freelist. Else `refCount--` (also when it is 0 or negative).
- librw: geometry.cpp:Material::destroy differs (`--refCount <= 0` test).

## 0x74dbc0 RpMaterialSetTexture  (catalogue: specified/REA, port level 3)
- cdecl `RpMaterial* (RpMaterial* m, RwTexture* tex)`: if tex: `tex->refCount++` (+0x54, 32-bit) FIRST; if old texture:
  `RwTextureDestroy(old)`; `m->texture = tex`; return m.

## 0x74dbf0 RpMaterialRegisterPlugin  (catalogue: specified/REA, port level 2)
- cdecl `i32 (size, pluginID, ctor, dtor, copy)` -> `_rwPluginRegistryAddPlugin(0x8d62a4, ...)` 0x8084a0.

## 0x74dc20 RpMaterialRegisterPluginStream  (catalogue: specified/REA, port level 1)
- cdecl `i32 (pluginID, readCB, writeCB, getSizeCB)` -> `_rwPluginRegistryAddPluginStream(0x8d62a4, ...)` 0x8088e0.

## 0x74dd30 RpMaterialStreamRead  (catalogue: specified/REA, port level 6)
- Convention: cdecl `RpMaterial* (RwStream* stream)`; called after rwID_MATERIAL (7) was found.
- Callees: RwStreamFindChunk, RwStreamRead, RwTextureStreamRead 0x8046e0, RwTextureDestroy, plugin InitObject / DeInitObject /
  ReadDataChunks / InvokeRights 0x808ab0.
- Behaviour:
  1. `FindChunk(stream, 1, &len, &ver)`; fail -> NULL (no error). `ver` outside [0x34000, 0x36003] -> error 0x80000004, NULL.
  2. Local struct (0x1c bytes, zeroed): `{i32 flags; RwRGBA color; i32 unused; i32 textured; f32 ambient; f32 specular;
     f32 diffuse}`. `RwStreamRead(stream, &s, len) != len` -> NULL (no error; `len` is not clamped to 0x1c).
  3. Inline RpMaterialCreate (alloc hint 0x30007; NULL -> return NULL), then `color = s.color`,
     `surfaceProps = {s.ambient, s.specular, s.diffuse}` (taken from the struct unconditionally), `texture = NULL`.
  4. If `s.textured`: `FindChunk(stream, 6 /*TEXTURE*/, NULL, &ver)`; fail -> inline destroy (see D) and return NULL with no
     error; `ver` out of range -> D + error 0x80000004, NULL. `texture = RwTextureStreamRead(stream)` - a NULL result is
     accepted (material without texture; no failure).
  5. `[0xc92510] = 0; [0xc924e8] = 0`; `_rwPluginRegistryReadDataChunks(0x8d62a4, stream, m)`; 0 -> D, NULL (no error).
  6. If `[0xc92510] != 0`: `_rwPluginRegistryInvokeRights(0x8d62a4, [0xc92510], m, [0xc924e8])`. Return m.
- D (inline RpMaterialDestroy): if `refCount == 1`: DeInitObject, RwTextureDestroy(texture) if any, `texture = 0`, free;
  else `refCount--`.
- librw: geometry.cpp:Material::streamRead differs: reads surface props only for version > 0x30400, returns failure when the
  texture read fails, no rights handling.

## 0x74e150 _rpMaterialListDeinitialize  (catalogue: specified/REA, port level 4)
- cdecl `RpMaterialList* (RpMaterialList* l)`. If `materials`: for i in 0..numMaterials (read once): `RpMaterialDestroy(materials[i])`,
  `materials[i] = NULL`; `Engine->free(materials)`; `materials = NULL`. Then `numMaterials = 0; space = 0`; return l.

## 0x74e1b0 _rpMaterialListInitialize  (catalogue: specified/REA, port level 0)
- cdecl `RpMaterialList* (RpMaterialList* l)`: `space = materials = numMaterials = 0`; return l.

## 0x74e2b0 _rpMaterialListGetMaterial  (catalogue: specified/REA, port level 0)
- cdecl `RpMaterial* (const RpMaterialList* l, i32 index)`: return `l->materials[index]` (no bounds check).

## 0x74e350 _rpMaterialListAppendMaterial  (catalogue: specified/REA, port level 1)
- cdecl `i32 (RpMaterialList* l, RpMaterial* m)`; returns the new index or -1.
- If `space > numMaterials` (signed): `materials[num] = m; m->refCount++; num++`; return `num - 1`.
- Else grow by 20 entries: `size = space*4 + 0x50`; `p = materials ? Engine->realloc(materials, size, 0x1030008) :
  Engine->malloc(size, 0x1030008)`; NULL -> error `{2, _rwerror(0x80000013, size)}`, return -1 (list unchanged). Else
  `materials = p; space += 0x14;` then append as above.

## 0x74e420 _rpMaterialListFindMaterialIndex  (catalogue: specified/REA, port level 0)
- cdecl `i32 (const RpMaterialList* l, const RpMaterial* m)`: search BACKWARDS from `num-1` to 0, return the first (highest)
  index with `materials[i] == m`; not found -> -1. (If `numMaterials < 0` it returns `numMaterials - 1`.)

## 0x74e600 _rpMaterialListStreamRead  (catalogue: specified/REA, port level 7)
- Convention: cdecl `RpMaterialList* (RwStream* stream, RpMaterialList* l)`; called after rwID_MATLIST (8) was found.
- Behaviour:
  1. `FindChunk(stream, 1, &len, &ver)`; fail -> NULL (no error). `ver` outside [0x34000, 0x36003] -> error 0x80000004, NULL.
  2. `RwStreamReadInt32(stream, &n, 4)` 0 -> NULL. `l = {0, 0, 0}`. `n == 0` -> return l.
  3. If `n > 0`: `materials = Engine->malloc(n*4, 0x1030008)`; NULL -> error `{2, _rwerror(0x80000013, n*4)}`, return NULL.
     `space = n`.
  4. `idx = Engine->malloc(n*4, 0x10501)` (NOT checked). `RwStreamReadInt32(stream, idx, n*4)` 0 -> free idx, cleanup (see C), NULL.
  5. For i in 0..n (n re-read each iteration; loop skipped if n <= 0):
     - If `idx[i] >= 0` (signed): `m = materials[idx[i]]; m->refCount++` (shared material reference).
     - Else: `FindChunk(stream, 7 /*MATERIAL*/, NULL, &ver)`; fail -> free idx, C, NULL (no error). `ver` out of range -> error
       0x80000004, free idx, C, NULL. `m = RpMaterialStreamRead(stream)`; NULL -> free idx, C, NULL.
     - `_rpMaterialListAppendMaterial(l, m)` (result ignored), then `RpMaterialDestroy(m)` (drops the extra reference).
  6. `Engine->free(idx)`; return l.
- C: RpMaterialDestroy each of the `numMaterials` entries (and NULL them), free `materials`, `materials = NULL`,
  `numMaterials = space = 0`.
- librw: geometry.cpp:MaterialList::streamRead matches in outline; differs: no version-range rejection.
