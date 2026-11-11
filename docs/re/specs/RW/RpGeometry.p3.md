# RenderWare 3.6 RpGeometry - pass-3 specs, writer A (RE-SPEC-RW-3)

Statically linked RW 3.6 world-plugin code (geometry.c). gta-reversed only has `plugin::Call*` thunks. All `__cdecl`.
Extends docs/re/specs/RW/RpGeometry.md (pass 1/2, read-only).

## Layouts used (offsets proven by the instruction addresses given)

`RpGeometry`: +0x8 u32 flags (0x8 PRELIT, 0x10 NORMALS, 0x1000000 NATIVE, 0x2000000 "instancing in progress";
0x74bf54, 0x74ce6e, 0x74bf71), +0xc u16 lockedSinceLastInst (byte OR 6 at 0x74bfec, `|= 0xfff` at 0x74c0b4), +0xe i16
refCount (0x74c091, 0x74d6d8), +0x10 numTriangles (0x74ce8c), +0x14 numVertices (0x74ce76), +0x18 numMorphTargets
(0x74ce97), +0x1c numTexCoordSets (0x74ce89), +0x20 RpMaterialList (0x74cee3), +0x2c RpTriangle* (8 bytes: u16 v[3], u16
mat; 0x74d01f), +0x30 RwRGBA* preLitLum (0x74cfb7), +0x34 RwTexCoords* texCoords[8] (0x74cfef), +0x54 RpMeshHeader*
(0x74ca65), +0x58 RwResEntry* repEntry (0x74bf5d), +0x5c RpMorphTarget* (0x74c002).
`RpMorphTarget` (0x1c): +0x0 parent, +0x4 RwSphere boundingSphere (16), +0x14 verts, +0x18 normals (0x74c010/0x74c014).
Geometry plugin registry `geometryTKList` 0x8d628c (0x74ce3b). `RpAtomic` +0x18 geometry (0x74bf46), +0x48 renderCallBack
(0x74bf7a). G = `[0xc97b24]`: +0x134 rwmalloc, +0x138 rwfree.

Inline "RpGeometryDestroy" sequence (used by 0x74bfe0 and 0x74c130 on an unlock failure): if `refCount - 1 > 0` ->
`refCount--`, return; else free repEntry, `lockedSinceLastInst |= 0xfff`, destroy mesh (`_rpMeshDestroy`, NULL it),
`_rwPluginRegistryDeInitObject(0x8d628c, g)`, `rwfree(morphTarget)` (NULL it), `_rpMaterialListDeinitialize(&matList)`,
`refCount--`, `rwfree(g)` (0x74c091..0x74c109).

## 0x74bf40 RpAtomicInstance  (catalogue: specified/REA, port level 2)
- `RwBool (RpAtomic* a)`: `g = a->geometry`; `g->numMorphTargets != 1` -> 0 (0x74bf49). If not NATIVE: free `g->repEntry`
  if set (0x74bf65); `g->flags |= 0x2000000` (0x74bf77); `a->renderCallBack(a)` (0x74bf7a; instancing happens inside);
  `g->flags = (flags & ~0x2000000) | 0x1000000` (0x74bf83..0x74bf8d). Return 1 (also when already native).
- librw: absent (librw has no RpAtomicInstance; instancing is implicit).

## 0x74bfe0 RpGeometryTransform  (catalogue: specified/REA, port level 8)
- `RpGeometry* (RpGeometry* g, const RwMatrix* m)`: `lockedSinceLastInst |= 6` (byte, 0x74bfec). For each morph target
  (signed count): `RwV3dTransformPoints(verts, verts, numVertices, m)` (0x74c01e); if `normals`: `RwV3dTransformVectors(
  normals, normals, numVertices, m)` (0x74c031) and, if `numVertices`, `_rwV3dNormalize(n, n)` for each normal (result
  popped, 0x74c042..0x74c052); `RpMorphTargetCalcBoundingSphere(mt, &mt->boundingSphere)` (0x74c059).
  `RpGeometryUnlock(g)` (0x74c081); NULL -> inline destroy (above), return NULL; else return g.
- librw: geometry.cpp has no transform (absent).

## 0x74c130 RpGeometryCreateSpace  (catalogue: specified/REA, port level 0)
- `RpGeometry* (f32 radius)`: `g = RpGeometryCreate(0, 0, 0)`; if `g`: `mt->boundingSphere = {0, 0, 0, radius}`
  (0x74c14c..0x74c155). `RpGeometryUnlock(g)` is called even when `g == NULL` (0x74c159); NULL result -> inline destroy,
  NULL; else g.
- librw: absent.

## 0x74c4d0 RpGeometryAddMorphTarget  (catalogue: specified/REA, port level 0)
- `RpGeometryAddMorphTargets(g, 1)` (0x74c4d7); returns its result.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74c4e0 RpGeometryRemoveMorphTarget  (catalogue: specified/REA, port level 1)
- `RpGeometry* (RpGeometry* g, i32 index)`: `sz = 0x1c + numVertices*12 + (flags & 0x10 ? numVertices*12 : 0)`
  (0x74c4ea..0x74c504); `nb = rwmalloc((numMorphTargets - 1) * sz, 0x3000f)` (0x74c51a); NULL -> `RwErrorSet({2,
  _rwerror(0x80000013, size)})`, NULL. Copy the 0x1c-byte headers of all targets except `index` (0x74c56e..0x74c59b); copy
  the data blocks (`sz - 0x1c` bytes each, assumed contiguous after the old headers) except `index` (0x74c5d0..0x74c600);
  `rwfree(old)`; `numMorphTargets--`; `morphTarget = nb`; re-point each target: `verts = normals = NULL`, then if
  `numVertices`: `verts = data`, `data += numVertices*12`, and if NORMALS `normals = data`, `data += numVertices*12`
  (0x74c63b..0x74c66b). Returns g. No index range check.
- librw: geometry.cpp has no removeMorphTarget (absent).

## 0x74c720 RpGeometryTriangleGetVertexIndices  (catalogue: specified/REA, port level 0)
- `(g, tri, u16* v1, u16* v2, u16* v3)`: each non-NULL out gets `tri->v[0..2]` (0x74c72c..0x74c74d); returns g.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74ca60 RpGeometryForAllMeshes  (catalogue: specified/REA, port level 0)
- `g->mesh && _rpMeshHeaderForAllMeshes(mesh, cb, data)` -> g, else NULL (0x74ca65..0x74ca87).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74ce20 RpGeometryStreamGetSize  (catalogue: specified/REA, port level 0)
- `_rpGeometryStructGetSize(g) + 0xc + _rpMaterialListStreamGetSize(&g->matList) + 0xc + _rwPluginRegistryGetSize(0x8d628c,
  g) + 0xc` (0x74ce27..0x74ce4c).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74ce60 _rpGeometryStructGetSize  (catalogue: specified/REA, port level 0)
- NATIVE -> 0x28 (0x74cec8). Else `s = 0x10`; if `numVertices`: PRELIT -> `s = numVertices*4 + 0x10`; `s += (numTexCoordSets
  * numVertices + numTriangles) * 8` (0x74ce7d..0x74ce94). Per morph target (`numMorphTargets > 0`, signed): `s += 0x18`, `+
  numVertices*12` if verts, `+ numVertices*12` if normals (0x74cea4..0x74cec3).
- librw: geometry.cpp:Geometry::streamGetSize matches.

## 0x74ced0 RpGeometryStreamWrite  (catalogue: specified/REA, port level 6)
- `RpGeometry* (const RpGeometry* g, RwStream* s)`:
  1. `Header(s, 0xf GEOMETRY, struct + 0xc + matList + 0xc + plugins + 0xc, 0x36003, 0xffff)`; `Header(s, 1, structSize)`
     (0x74cf14/0x74cf41).
  2. Raw 16-byte struct `{flags | (numTexCoordSets & 0xff) << 16, numTriangles, numVertices, numMorphTargets}`
     (`RwStreamWrite`, 0x74cf55..0x74cf86).
  3. If not NATIVE and `numVertices`: PRELIT -> raw `preLitLum` (numVertices*4); for each tex-coord set
     `RwStreamWriteReal(texCoords[i], numVertices*8)`; for each triangle two raw dwords `(v0 << 16) | v1` and `(v2 << 16) |
     mat` (0x74d022..0x74d057; on-disk u16 order v1, v0, mat, v2).
  4. Per morph target: raw 0x18 bytes `{sphere (4 dwords), pointsPresent, normalsPresent}` (present = NATIVE ? 0 :
     pointer != NULL; 0x74d07a..0x74d0da); then `RwStreamWriteReal(verts, numVertices*12)` / normals if present.
  5. `_rpMaterialListStreamWrite(&matList, s)`; `_rwPluginRegistryWriteDataChunks(0x8d628c, s, g)`; any failure -> NULL;
     else g.
- librw: geometry.cpp:Geometry::streamWrite matches in layout.

## 0x74d6d0 sub_74d6d0 (proposed: _rpGeometryDestroyFinal)  (catalogue: unknown/REA, port level 5)
- `RwBool (RpGeometry* g)`: `refCount++` (0x74d6d8), `lockedSinceLastInst |= 0xfff`; mesh destroy (NULL it);
  `_rwPluginRegistryDeInitObject(0x8d628c, g)`; `rwfree(morphTarget)` (NULL it); `_rpMaterialListDeinitialize(&matList)`;
  `refCount--`; `rwfree(g)`; return 1. (No repEntry free here, unlike the inline destroy.)
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x74d750 _rpGeometryChunkInfoRead  (catalogue: specified/REA, port level 0)
- `info* (RwStream* s, info* out16, i32* size)`: `RwStreamFindChunk(s, 1, &len, &ver)` fail -> NULL; zero 16 bytes; `read
  16` != 16 -> NULL; `RwStreamSkip(s, len - 0x10)` (0x74d7a7, result ignored); `*size = len + 0xc`; return out.
- librw: no direct equivalent identified in librw (not compared line by line).

