# RpGeometry (RenderWare 3.6 RpWorld plugin, statically linked) - full specs, writer B (RE-SPEC-RW)

All functions are `__cdecl` (caller pops). `Engine` = `[RwEngineInstance 0xc97b24]`; `Engine+0x134` malloc
`(size, hint)`, `+0x138` free `(ptr)`, `+0x13c` realloc `(ptr, size, hint)`. Geometries are NOT freelist objects:
they are malloc'd (0x74cb3d) and freed with `Engine+0x138` (0x74cd49). Errors: `RwErrorSet({pluginID 2, code})`
as in RpClump.md.

## Layouts used (verified against the bytes)

RpGeometry (registry 0x8d628c "geometryTKList", base 0x60; final size `[0x8d628c]` read at run time, 0x74cabe):

| off | field | proving instruction |
|---|---|---|
| +0x00 u8 | object.type = 8 | 0x74cb83 |
| +0x01..+0x03 u8 | subType, flags, privateFlags = 0 | 0x74cb86..0x74cb8e |
| +0x04 ptr | object.parent = NULL | 0x74cb6d |
| +0x08 u32 | flags (format): 0x1 TRISTRIP, 0x4 TEXTURED, 0x8 PRELIT, 0x10 NORMALS, 0x80 TEXTURED2, 0x1000000 NATIVE | 0x74cbc0, 0x74c31e, 0x74ca1d |
| +0x0c u16 | lockedSinceLastInst | 0x74cb73, 0x74c7d9 |
| +0x0e i16 | refCount (1 at create) | 0x74cb92, 0x74ccb4 |
| +0x10 i32 | numTriangles | 0x74cbbd |
| +0x14 i32 | numVertices | 0x74cbc3 |
| +0x18 i32 | numMorphTargets | 0x74cb6a, 0x74c3e1 |
| +0x1c i32 | numTexCoordSets | 0x74cb98 |
| +0x20 RpMaterialList | matList {materials +0x20, numMaterials +0x24, space +0x28} | 0x74cb50, 0x74c79a |
| +0x2c ptr | triangles (RpTriangle, 8 bytes: u16 vertIndex[3] +0/+2/+4, u16 matIndex +6) | 0x74cc05, 0x74c6fa |
| +0x30 ptr | preLitLum (RwRGBA[numVertices]) | 0x74cbdc |
| +0x34 ptr[8] | texCoords[8] (RwTexCoords = 8 bytes) | 0x74cb64, 0x74cb9b |
| +0x54 ptr | mesh (RpMeshHeader*) | 0x74cb77, 0x74c7e1 |
| +0x58 ptr | repEntry | 0x74cb70, 0x74ccd5 |
| +0x5c ptr | morphTarget array | 0x74cb67, 0x74c3e4 |

RpMorphTarget (stride 0x1c): +0x0 parentGeom (0x74c471), +0x4 RwSphere boundingSphere (0x74c45e..0x74c46e),
+0x14 verts (0x74c3fc), +0x18 normals (0x74c402). All match objects.md.

Memory layout of a non-native geometry block (one malloc, 0x74cb12..0x74cc19): `[struct incl. plugins ([0x8d628c])]
[preLitLum numVertices*4 if PRELIT][numTexCoordSets * numVertices*8][triangles numTriangles*8]`. Morph targets live in a
separate block: `[numMorphTargets * 0x1c headers][per target: verts numVertices*12 (+ normals numVertices*12 if NORMALS)]`.
GEOMETRYMODULE: offset global 0xc924e0, open counter 0xc924e4. Default surface properties (used by materials):
0x8d62bc ambient, 0x8d62c0 specular, 0x8d62c4 diffuse (f32; see `_rpMaterialSetDefaultSurfaceProperties` 0x74d870).

## 0x74bfa0 cb_GEOMETRYMODULE_RwEngine_Open  (catalogue: specified/REA, port level 0)
- `void* (void* instance, i32 offset, i32 size)`: `[0xc924e0] = offset; [0xc924e4]++;` return instance.

## 0x74c200 RpMorphTargetCalcBoundingSphere  (catalogue: specified/REA, port level 1)
- Convention: cdecl `const RpMorphTarget* (const RpMorphTarget* mt, RwSphere* out)`; returns mt.
- Callees: RwBBoxCalculate 0x808f60 `(RwBBox* out {sup +0, inf +0xc}, const RwV3d* verts, i32 n)`, _rwSqrt 0x7edb30.
- Behaviour:
  1. `maxD2 = 0` (local int store 0); `n = mt->parentGeom->numVertices`; `RwBBoxCalculate(&bb, mt->verts, n)`.
  2. x87: `sx = bb.sup.x + bb.inf.x`, `sy = sup.y + inf.y`, `sz = sup.z + inf.z` (three register values); `cz = sz` is
     stored f32 first; then `cx = sx * [0x858b8c] = 0.5f` (stored f32), `cy = sy * 0.5f` (stored), `cz = (f32)cz * 0.5f`.
  3. If `n != 0` (0x74c243): for each vertex v (count down `n` times, unsigned `dec/jne`): `dx = v.x - cx`, `dy = v.y - cy`,
     `dz = v.z - cz`, `d2 = (dx*dx + dy*dy) + dz*dz` (register); `if (d2 > maxD2)` (fcom vs f32 maxD2; C0|C3 clear, so NaN
     or equal is not taken) `maxD2 = (f32)d2`. Then `if (maxD2 > [0x858b50] = 0.0f)` (same test style) `maxD2 = _rwSqrt(maxD2)`.
     If n == 0 the sqrt is skipped too.
  4. `out->center = {cx, cy, cz}`; `out->radius = maxD2 * [0x872490] = 1.001f` (stored f32). Note the radius uses the
     variable that holds the sqrt result; when maxD2 <= 0 the (non-negative) squared value 0 is used.
- librw: geometry.cpp:MorphTarget::calculateBoundingSphere differs: own min/max loop seeded with +-1000000, radius = half
  the bbox diagonal (not the max vertex distance), no 1.001f factor.

## 0x74c310 RpGeometryAddMorphTargets  (catalogue: specified/REA, port level 1)
- Convention: cdecl `i32 (RpGeometry* g, i32 n)`; returns index of the first new morph target, or -1.
- Behaviour:
  1. `per = 0x1c` if NATIVE; else `per = numVertices*12 + 0x1c`, plus `numVertices*12` if `flags & 0x10`.
     `total = (numMorphTargets + n) * per` (imul, 32-bit).
  2. If `g->morphTarget != NULL`: `p = Engine->realloc(old, total, 0x3000f)`; NULL -> error `{2, _rwerror(0x80000013, total)}`,
     return -1 (old block kept). Then move the old vertex data up by `n*0x1c` bytes: byte-by-byte copy BACKWARDS of
     `(per - 0x1c) * oldNum` bytes from `p + oldNum*per - 1` to `p + oldNum*per - 1 + n*0x1c` (0x74c3c9).
     Else: `p = Engine->malloc(total, 0x3000f)`; NULL -> same error, -1.
  3. `numMorphTargets += n; morphTarget = p`. `cur = p + numMorphTargets*0x1c`. For EVERY target i (0..numMorphTargets,
     skipped if <= 0): `verts = normals = NULL`; if not NATIVE and `numVertices != 0`: `verts = cur; cur += numVertices*12;`
     if NORMALS (`flags` byte & 0x10): `normals = cur; cur += numVertices*12`.
  4. For the new targets i in [numMorphTargets - n, numMorphTargets): `boundingSphere = {0,0,0,0}`, `parentGeom = g`.
  5. Return `numMorphTargets - n`.
- librw: geometry.cpp:Geometry::addMorphTargets differs (separate allocations per target, no backward move).

## 0x74c690 RpGeometryTriangleSetVertexIndices  (catalogue: specified/REA, port level 0)
- cdecl `const RpGeometry* (const RpGeometry* g, RpTriangle* tri, u16 v1, u16 v2, u16 v3)`: `tri->vertIndex[0..2] = v1, v2, v3`
  (u16 stores at +0/+2/+4); return g.

## 0x74c6c0 RpGeometryTriangleSetMaterial  (catalogue: specified/REA, port level 2)
- cdecl `RpGeometry* (RpGeometry* g, RpTriangle* tri, RpMaterial* mat)`.
- If `mat == NULL`: `tri->matIndex = 0xffff`, return g. Else `i = _rpMaterialListFindMaterialIndex(&g->matList, mat)`
  (0x74e420); if `i < 0`: `i = _rpMaterialListAppendMaterial(&g->matList, mat)` (0x74e350); if still `< 0` return NULL
  (tri unchanged). `tri->matIndex = (u16)i`; return g.

## 0x74c760 RpGeometryTriangleGetMaterial  (catalogue: specified/REA, port level 0)
- cdecl `RpMaterial* (const RpGeometry* g, const RpTriangle* tri)`: if `tri->matIndex == 0xffff` return NULL; else
  `g->matList.materials[matIndex]` (u16 index, no bounds check).

## 0x74c790 RpGeometryForAllMaterials  (catalogue: specified/REA, port level 0)
- cdecl `RpGeometry* (RpGeometry* g, RpMaterialCallBack cb, void* data)`: `n = matList.numMaterials` read once; for
  i in 0..n (signed): `if (!cb(matList.materials[i], data)) break` (materials pointer re-read each iteration). Return g.

## 0x74c7d0 RpGeometryLock  (catalogue: specified/REA, port level 1)
- cdecl `RpGeometry* (RpGeometry* g, i32 lockMode)`: `lockedSinceLastInst |= (u16)lockMode`; if `lockMode & 1`
  (rpGEOMETRYLOCKPOLYGONS) and `g->mesh`: `_rpMeshDestroy(mesh)` (0x758bc0), `mesh = NULL`. Return g.

## 0x74c800 RpGeometryUnlock  (catalogue: specified/REA, port level 7)
- Convention: cdecl `RpGeometry* (RpGeometry* g)`; returns g or NULL. Does not clear lockedSinceLastInst and does not
  recompute bounding spheres.
- Callees: _rpBuildMeshCreate 0x758a90, _rpMaterialListGetMaterial 0x74e2b0, _rpBuildMeshAddTriangle 0x758c00,
  _rpMeshOptimise 0x75d970, _rpBuildMeshDestroy 0x758b80.
- Behaviour:
  1. If `g->mesh != NULL` return g (nothing to rebuild).
  2. `bm = _rpBuildMeshCreate(numTriangles)`; NULL -> return NULL.
  3. If `numMaterials > 0`: allocate three arrays of `numMaterials*4` bytes: `texs = malloc(.., 0x10006)`,
     `rasters = malloc(.., 0x10503)`, `pipes = malloc(.., 0x10503)` (results unchecked). Counters nTex = nRas = nPipe = 0 (u16).
     For each triangle t (i = 0..numTriangles, numTriangles re-read each iteration):
     - `mat = _rpMaterialListGetMaterial(&g->matList, t.matIndex)`; `tex = mat->texture` (+0x0, mat assumed non-NULL).
     - `ti` = first index in texs[0..nTex) equal to tex (16-bit index), else append (`texs[nTex++] = tex`).
     - `ras = tex ? tex->raster (+0x0) : NULL`; `ri` likewise in rasters.
     - `pp = mat->pipeline (+0x8)`; `pi` likewise in pipes.
     - `_rpBuildMeshAddTriangle(bm, mat, t.v[0], t.v[1], t.v[2], t.matIndex, ti, ri, pi)` (9 args, 0x74c9c9).
     Free texs, rasters, pipes (Engine+0x138). If `numMaterials <= 0` no triangle is added at all.
  4. `m = _rpMeshOptimise(bm, (g->flags & 1) ? 1 : 0)`; NULL -> `_rpBuildMeshDestroy(bm)`, return NULL. Else `g->mesh = m`;
     return g (bm is not destroyed here; see _rpMeshOptimise).
- librw: geometry.cpp:Geometry::unlock differs (builds meshes by material only, recalculates spheres).

## 0x74ca90 RpGeometryCreate  (catalogue: specified/REA, port level 5)
- Convention: cdecl `RpGeometry* (i32 numVerts, i32 numTriangles, u32 format)`; returns geometry or NULL.
- Behaviour:
  1. `numVerts < 0` -> NULL (no error). `numVerts >= 0x10000` -> error `{2, _rwerror(6)}`, NULL. `numTriangles < 0` -> NULL.
  2. Texture-coordinate set count `nt`: if `format & 0xff0000` -> `nt = (format >> 16) & 0xff`; else if `format & 0x80`
     -> 2; else `nt = (format >> 2) & 1`. Low flag byte: `fl = (format & 0x7b) | (nt == 1 ? 4 : (nt > 1 ? 0x80 : 0))`
     (unsigned compare, 0x74caf2).
  3. Size: `[0x8d628c]`; if not NATIVE (`format & 0x1000000`): `+ numVerts*4` if `fl & 8`; `+ nt*numVerts*8` if nt > 0;
     `+ numTriangles*8`. `g = Engine->malloc(size, 0x3000f)`; NULL -> return NULL.
  4. `_rpMaterialListInitialize(&g->matList)` (always returns the list; the NULL test at 0x74cb5e is dead).
  5. Fields: morphTarget, numMorphTargets, parent, repEntry, lockedSinceLastInst, mesh = 0; type 8, sub/flags/private 0;
     refCount 1; `numTexCoordSets = nt`; texCoords[0..7] = 0; preLitLum = triangles = 0; `numTriangles`;
     `flags = (format & 0x0f000000) | fl` (bits 8..23 of format are dropped); `numVertices = numVerts`.
  6. If not NATIVE: `p = g + [0x8d628c]`; if `(fl & 8) && numVerts`: `preLitLum = p; p += numVerts*4`. If `nt > 0 && numVerts`:
     for k in 0..nt: `texCoords[k] = p; p += numVerts*8` (no clamp to 8: nt up to 255 overruns the array; original bug).
     If `numTriangles`: `triangles = p`; every `triangles[i].matIndex = 0xffff`.
  7. `RpGeometryAddMorphTargets(g, 1) < 0` -> `_rpMaterialListDeinitialize(&matList)`, `Engine->free(g)`, NULL.
  8. `_rwPluginRegistryInitObject(0x8d628c, g)`; return g.
- librw: geometry.cpp:Geometry::create differs (separate allocations, no 0x10000 limit error).

## 0x74ccb0 _rpGeometryAddRef  (catalogue: specified/REA, port level 0)
- cdecl `void (RpGeometry* g)`: `g->refCount++` (16-bit `inc word`). EAX = g.

## 0x74ccc0 RpGeometryDestroy  (catalogue: specified/REA, port level 5)
- cdecl `RwBool (RpGeometry* g)`; returns 1 always.
- If `(i16)refCount - 1 > 0`: `refCount--`, return 1. Else: if repEntry: RwResourcesFreeResEntry(repEntry);
  `lockedSinceLastInst |= 0xfff`; if mesh: `_rpMeshDestroy(mesh)`, `mesh = NULL`; `_rwPluginRegistryDeInitObject(0x8d628c, g)`;
  if morphTarget: `Engine->free(morphTarget)`, `= NULL`; `_rpMaterialListDeinitialize(&matList)` (0x74e150, releases
  material references); `refCount--` (to 0); `Engine->free(g)`; return 1.
- librw: geometry.cpp:Geometry::destroy matches in outline (no resEntry/lock flag).

## 0x74cd70 RpGeometryRegisterPlugin  (catalogue: specified/REA, port level 2)
- cdecl `i32 (size, pluginID, ctor, dtor, copy)` -> `_rwPluginRegistryAddPlugin(0x8d628c, ...)` 0x8084a0.

## 0x74cda0 RpGeometryRegisterPluginStream  (catalogue: specified/REA, port level 1)
- cdecl `i32 (pluginID, readCB, writeCB, getSizeCB)` -> `_rwPluginRegistryAddPluginStream(0x8d628c, ...)` 0x8088e0.

## 0x74d190 RpGeometryStreamRead  (catalogue: specified/REA, port level 8)
- Convention: cdecl `RpGeometry* (RwStream* stream)`; called after rwID_GEOMETRY (0xf) was found. The argument slot is
  reused as the `version` out-variable. Callers: RpClumpStreamRead, _rpClumpAtomicStreamRead, GTA readers 0x72e080..0x72e270.
- Callees: RwStreamFindChunk 0x7ed2d0, RwStreamRead 0x7ec9d0, RwStreamReadReal 0x7ed4f0 `(stream, float*, numBytes)`,
  RpGeometryCreate, RpGeometryAddMorphTargets, _rpMaterialSetDefaultSurfaceProperties 0x74d870, _rpMaterialListStreamRead
  0x74e600, _rwPluginRegistryReadDataChunks, RpGeometryUnlock, RpGeometryLock, 0x74d6d0 (geometry free helper).
- Behaviour:
  1. `FindChunk(stream, 1, NULL, &ver)`; fail -> return NULL (no error set). `ver` outside [0x34000, 0x36003] -> error
     0x80000004, NULL.
  2. Header (16 bytes): `{u32 format; i32 numTriangles; i32 numVertices; i32 numMorphTargets}` read with RwStreamRead
     (`!= 16` -> NULL, no error). If `ver < 0x34001` an extra 12 bytes `{f32 ambient, specular, diffuse}` follow; short read
     -> NULL (no error).
  3. `g = RpGeometryCreate(numVertices, numTriangles, format)`; NULL -> NULL.
  4. If `numMorphTargets > 1`: `RpGeometryAddMorphTargets(g, numMorphTargets - 1) < 0` -> F.
  5. If not NATIVE (`g->flags`) and `g->numVertices != 0`:
     - if `format & 8` (header byte): `RwStreamRead(stream, preLitLum, numVertices*4)` short -> F.
     - for k in 0..numTexCoordSets: `RwStreamReadReal(stream, texCoords[k], numVertices*8)` 0 -> F.
     - if `numTriangles`: `RwStreamRead(stream, triangles, numTriangles*8)` short -> F; then convert each triangle in place:
       file u16 words `[w0 w1 w2 w3]` become `vertIndex = {w1, w0, w3}`, `matIndex = w2` (halves of each dword swapped).
  6. For each morph target i (0..numMorphTargets): read 0x18 bytes `{RwSphere sphere; i32 hasVerts; i32 hasNormals}` (short -> F);
     `mt[i].boundingSphere = sphere`. If hasVerts && hasNormals: ONE `RwStreamReadReal(stream, mt[i].verts, numVertices*24)`
     (fills verts and the normals that follow them in memory). If hasVerts only: `ReadReal(verts, numVertices*12)`. If hasNormals
     only: `ReadReal(normals, numVertices*12)`. Any 0 -> F.
  7. `FindChunk(stream, 8 /*MATLIST*/, NULL, &ver)`; fail -> return NULL WITHOUT freeing g (leak, original bug).
     `ver` out of range -> F + error 0x80000004 (the error is set even when the refcount path only decrements).
  8. If `ver < 0x34001`: `_rpMaterialSetDefaultSurfaceProperties(&headerSurfProps)`. `_rpMaterialListStreamRead(stream,
     &g->matList)` NULL -> F2. If `ver < 0x34001`: `_rpMaterialSetDefaultSurfaceProperties(NULL)` (back to 1.0f).
  9. `_rwPluginRegistryReadDataChunks(0x8d628c, stream, g)` 0 -> F2. `RpGeometryUnlock(g)` NULL -> F2. Return g.
- F (0x74d522 / 0x74d3ac / 0x74d600): if `refCount - 1 > 0` -> `refCount--`; else: free repEntry, `RpGeometryLock(g, 0xfff)`,
  DeInitObject, free morphTarget, MatListDeinitialize, `refCount--`, `Engine->free(g)`. Return NULL.
- F2 (0x74d5b4): if `refCount - 1 > 0` -> `refCount--`; else free repEntry, `refCount--`, `0x74d6d0(g)` (re-increments,
  sets lock 0xfff, destroys mesh, DeInit, frees morph targets, MatListDeinitialize, decrements, frees g). Return NULL.
- Edge: in step 5 the version-0x34000 surface properties are only kept in a local and pushed to the defaults; vertex data
  is read with RwStreamReadReal (endian-converting reals), triangles with RwStreamRead.
- librw: geometry.cpp:Geometry::streamRead differs: reads verts and normals with separate calls, no version-range rejection,
  frees the geometry on every failure.
