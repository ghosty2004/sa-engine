# RenderWare 3.6 RpUVAnim (UV animation plugin, id 0x135) - full specs, writer C (RE-SPEC-RW-2)

Statically linked RW 3.6 plugin code in gta_sa.exe 1.0 US. gta-reversed has only thunks (RenderWare/rw/rpuvanim.cpp).
All functions are `__cdecl`. `G` = `RwEngineInstance` (0xc97b24); `G+0x148 freeListFree(fl, p)`.

## Layouts used (offsets proven by the instruction addresses given)

Globals (.data):
| global | meaning | proof |
|---|---|---|
| `[0xc9b8d0] i32` | material extension offset (0x28 bytes) | 0x7cb98c |
| `[0xc978f4] RwFreeList*` | freelist of 0x44-byte UV-anim custom data blocks | 0x7cb9d6 |
| `[0xc978fc] i32` | engine-open counter | 0x7cba0e |
| `[0x8ded48] = 0x80`, `[0x8ded4c] = 0x1` | freelist entries per block / blocks to preallocate | 0x7cb9bb, 0x7cb9b5 |
| 0x8ded50 | `RtDictSchema RpUVAnimDictSchema` (.data: name 0x8dede4, dictChunkType 0x2b, entryChunkType 0x1b, compatibility 0x36000, dictionaries +0x10, current +0x14, addRef 0x7cc100, destroy 0x7cc0c0, getName 0x7cc3a0, getSize 0x7cd4d0, read 0x7cd220, compat NULL, write 0x7cd410) | data dump of 0x8ded50 |
| 0x8ded84, 0x8dedb4 | two `RtAnimInterpolatorInfo` (typeID 0x1c0 and 0x1c1, key frame sizes 0x20/0x20, customDataSize 0x44) | data dumps |

Material extension `RpUVAnimMaterialExt` (0x28 bytes at `material + [0xc9b8d0]`):
| off | field | proof |
|---|---|---|
| +0x00, +0x04 | `RwMatrix* uvTransform[2]` | dtor 0x7cbab2..0x7cbac2 |
| +0x08 .. +0x24 | `RtAnimInterpolator* interp[8]` | dtor 0x7cbacf..0x7cbb17 |

`RtAnimInterpolator` +0x0 pCurrentAnim; `RtAnimAnimation` +0x14 customData; UV custom data +0x40 `i32 refCount` (0x7cbae2).

## 0x7CB940 RpUVAnimPluginAttach  (catalogue: specified/REA, port level 3)
- `RwBool (void)`: `RwEngineRegisterPlugin(0, 0x135, Open=0x7cb9b0, Close=0x7cba20)` (0x7cb951); `< 0` -> 0 (0x7cb95b).
  `[0xc9b8d0] = RpMaterialRegisterPlugin(0x28, 0x135, Ctor=0x7cba80, Dtor=0x7cbaa0, Copy=0x7cbb30)` (0x7cb973, stored 0x7cb98c);
  `r = RpMaterialRegisterPluginStream(0x135, Read=0x7cbc20, Write=0x7cbb70, GetSize=0x7cbf20)` (0x7cb991); return 0 if `r < 0`
  or `[0xc9b8d0] < 0` (0x7cb99b/0x7cb9a4), else 1.
- librw: uvanim.cpp:registerUVAnimPlugin matches in substance (librw registers no engine part; ext layout differs).

## 0x7CB9B0 cb_UVANIMPLUGIN_RwEngine_Open  (catalogue: specified/REA, port level 3)
- `void* (void* instance, i32, i32)`.
- Behaviour: `[0xc978f4] = RwFreeListCreateAndPreallocateSpace(0x44, [0x8ded48], 4, [0x8ded4c], (void*)[0xc978f4], 0x40135)`
  (0x7cb9ce): note the in-place storage argument is the CURRENT VALUE of the global itself (NULL on first open, 0x7cb9b0/0x7cb9c7).
  Result NULL -> return value NULL (0x7cb9df) but execution continues; else return value = instance.
  `RtDictSchemaInit(0x8ded50)` (0x7cb9ec), `RtAnimRegisterInterpolationScheme(0x8ded84)` (0x7cb9f6),
  `RtAnimRegisterInterpolationScheme(0x8dedb4)` (0x7cba00) (results ignored); `[0xc978fc] += 1` (0x7cba0e); return.
- librw: uvanim.cpp:uvanimOpen differs: no freelist; registers its own schemes.

## 0x7CBA80 cb_UVANIMPLUGIN_RpMaterial_Ctor  (catalogue: specified/REA, port level 0)
- `memset(material + [0xc9b8d0], 0, 0x28)` (rep stosd 10 dwords, 0x7cba96); returns material.
- librw: uvanim.cpp:createUVAnim matches.

## 0x7CBAA0 cb_UVANIMPLUGIN_RpMaterial_Dtor  (catalogue: specified/REA, port level 1)
- `void* (RpMaterial* m, i32, i32)`; returns m ([esp+0x14], 0x7cbb19).
- Behaviour: `ext = m + [0xc9b8d0]`. For `i = 0, 1`: if `ext->uvTransform[i]` -> `RwMatrixDestroy` (0x7cbaba); store NULL
  (0x7cbac2). For each of the 8 interpolator slots (0x7cbad7..0x7cbb17): `it = slot`; if non-NULL: `anim = it->pCurrentAnim`
  (dereferenced without a test); `cd = anim->customData`; `--cd->refCount (+0x40)` (0x7cbae5); if it reached 0:
  `G->freeListFree([0xc978f4], cd)` (0x7cbafb) and `RtAnimAnimationDestroy(anim)` (0x7cbb02). Then
  `RtAnimInterpolatorDestroy(it)` (0x7cbb0b). The interpolator slots are NOT cleared.
- librw: uvanim.cpp:destroyUVAnim differs (librw uses its own refcounted UVAnimCustomData and clears slots).

## Open questions
- None.
