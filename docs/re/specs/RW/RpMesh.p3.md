# RenderWare 3.6 RpMesh / RpBuildMesh / RpTriStrip - pass-3 specs, writer A (RE-SPEC-RW-3)

Statically linked RW 3.6 world-plugin code (mesh.c, bamesh.c, tristrip.c, geometry sort). gta-reversed only has
thunks. All `__cdecl`. Extends docs/re/specs/RW/RpMesh.md and RpBuildMesh.md (pass 1/2, read-only).

## Layouts used (offsets proven by the instruction addresses given)

- `RpMeshHeader` (0x10): +0x0 flags (1 = TRISTRIP, 0x759154), +0x4 u16 numMeshes (0x758d85), +0x6 u16 serialNum (0x75915c), +0x8
  totalIndicesInMesh (0x758d89), +0xc firstMeshOffset (0x759160); meshes `RpMesh` (0xc: +0 u16* indices, +4 numIndices, +8
  material) follow at header+0x10 (0x758dc2 assumes firstMeshOffset 0).
- `RpBuildMesh`: +0x4 numTriangles, +0x8 triangles (0x14 each: u16 v[3] +0/+2/+4, +8 material) (0x759107..0x759185).
- MESHMODULE engine data `G + [0xc9b8c0]`: +0x0 u16 nextSerialNum (0x7590ed); open counter `[0xc9b8c4]` (0x758940); build-mesh
  freelist `[0xc926f0]` (0x75894d). Tri-strip generator `[0x8d6388]` (default 0x7591b0) and user data `[0x8d638c]`
  (0x75d4c8/0x75d4d1). World-sector device extension offset `[0xc926ec]` (0x7588e4). World-module data `G + [0xc9bc60]`: +0x44
  current default material pipeline, +0x5c the built-in default (0x75e2e4/0x75e2e8).
- RW SDK tri-strip structs (rpworld.h): `RpTriStripPolygon` +0x18 numEdges, +0x1c `RpTriStripEdge* edges[]`; `RpTriStripEdge`
  +0x10 poly1, +0x14 poly2, +0x18/+0x1a u16 vert1/vert2, +0x1c RwBool strip (0x75c338..0x75c374, 0x75bb35).
- Object type byte (+0): 7 world, 8 geometry; flags +8 bit 0x1000000 native (0x759099..0x7590ae).

## 0x758870 cb_RXWORLDDEVICEMODULE_RpGeometry_RpWorldSector_Copy  (catalogue: specified/REA, port level 0)
- `void* (void* dst, const void* src, i32 off, i32)`: copies the 4-byte extension (0x758878/0x75887f); EAX = dst.
- librw: absent.

## 0x7588e0 RpD3D9WorldSectorSetUsageFlags  (catalogue: specified/REA, port level 0)
- `*(u32*)(sector + [0xc926ec]) = flags` (0x7588ee); no return value set (EAX = flags).
- librw: absent.

## 0x758910 _rpMeshHeaderDestroy  (catalogue: specified/REA, port level 0)
- Tail-jumps to `G->rwfree` (G+0x138) with the header argument unchanged (0x758916).
- librw: absent (librw frees inside Geometry).

## 0x758940 _rpMeshClose  (catalogue: specified/REA, port level 1)
- `[0xc9b8c4] -= 1`; at 0 and if `[0xc926f0]`: `RwFreeListDestroy`, `[0xc926f0] <- 0` (0x758945..0x75895f); returns instance.
- librw: absent.

## 0x758d70 _rpMeshWrite  (catalogue: specified/REA, port level 3)
- `RwStream* (const RpMeshHeader* h, RwStream* s, const void* object, const RpMaterialList* ml)`: `RwStreamWriteInt32(s, {flags,
  numMeshes, totalIndicesInMesh}, 0xc)` (0x758da7). For each mesh: `mi = _rpMaterialListFindMaterialIndex(ml, mesh->material)`, `< 0`
  -> 0 (0x758de9..0x758df9); `RwStreamWriteInt32({numIndices, mi}, 8)`. Indices are written only for a non-native geometry (type 8)
  or non-native world (type 7) (0x758e1c..0x758e3d): converted u16 -> u32 into a 0x100-entry local buffer and written in chunks
  with `RwStreamWriteInt32(buf, n*4)` (0x758e49..0x758e96). Failures -> NULL; else s.
- librw: geometry.cpp:writeMesh matches in layout.

## 0x759090 _rpMeshSize  (catalogue: specified/REA, port level 0)
- `(h, object)`: native (as above) -> `numMeshes*8 + 0xc`; else `(totalIndicesInMesh + numMeshes*2)*4 + 0xc` (0x7590b3..0x7590d5).
- librw: matches.

## 0x7590e0 _rpMeshGetNextSerialNumber  (catalogue: specified/REA, port level 0)
- Returns the old u16 serial (AX) and stores it + 1 (0x7590ed..0x7590f4).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x759100 RpBuildMeshGenerateTrivialTriStrip  (catalogue: specified/REA, port level 1)
- `RpMeshHeader* (RpBuildMesh* bm, void*)`: `n = bm->numTriangles`; `h = _rpMeshHeaderCreate(18*n + 0x10)` (0x75911c, no NULL check);
  fill the whole block with 0xcd bytes (0x759127..0x75913f); `numMeshes = n`, `totalIndicesInMesh = 3n`, `flags = 1` (TRISTRIP),
  `serialNum = 0`, `firstMeshOffset = 0`; each triangle becomes one mesh of 3 indices `{v0, v1, v2}` with its material, indices after
  the n mesh records (0x759169..0x759199). Returns h.
- librw: absent.

## 0x75a8e0 RpBuildMeshGenerateDefaultIgnoreWindingTriStrip  (catalogue: specified/REA, port level 0)
- `0x7591d0(bm, 0, 0)` (0x75a8e9).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75a900 RpBuildMeshGeneratePreprocessTriStrip  (catalogue: specified/REA, port level 0)
- `0x7591d0(bm, 1, 1)` (0x75a909).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75a920 RpBuildMeshGeneratePreprocessIgnoreWindingTriStrip  (catalogue: specified/REA, port level 0)
- `0x7591d0(bm, 1, 0)` (pushes 0 then 1, 0x75a924..0x75a929).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75a940 RpBuildMeshGenerateExhaustiveTriStrip  (catalogue: specified/REA, port level 0)
- `0x75a960(bm, data, 1)` (0x75a94c).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75b4e0 RpBuildMeshGenerateExhaustiveIgnoreWindingTriStrip  (catalogue: specified/REA, port level 0)
- `0x75a960(bm, data, 0)` (0x75b4ec).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75bb20 sub_75bb20 (proposed: _rpTriStripPathInvert)  (catalogue: unknown/REA, port level 0)
- `void (path*)` with `+0 u32 length`, `+8 RpTriStripEdge** edges`: for each edge `e->strip = (e->strip == 0)` (0x75bb2d..0x75bb45).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75bb50 sub_75bb50 (proposed: _rpTriStripPolygonNumStripEdges)  (catalogue: unknown/REA, port level 0)
- `u32 (const RpTriStripPolygon* p)`: count of `edges[j]` with `strip != 0`, `j < numEdges` (0x75bb56..0x75bb70).
- librw: tristrip.cpp:numStripEdges matches in purpose.

## 0x75c330 RpTriStripPolygonFollowStrip  (catalogue: specified/REA, port level 0)
- `RpTriStripPolygon* (RpTriStripPolygon* p, RpTriStripPolygon* prev)`: first edge `e` (j ascending) with `e->strip && e->poly1 != prev
  && e->poly2 != prev` (0x75c34a..0x75c35a); returns `e->poly1 != p ? e->poly1 : e->poly2` (0x75c369..0x75c374); none -> NULL.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75d4c0 RpMeshSetTriStripMethod  (catalogue: specified/REA, port level 0)
- `(cb, data)`: `cb != NULL` -> `[0x8d6388] = cb`, `[0x8d638c] = data`; else default `[0x8d6388] <- 0x7591b0`, `[0x8d638c] <- 0`
  (0x75d4dc/0x75d4e6). Returns 1.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75d500 RpMeshGetTriStripMethod  (catalogue: specified/REA, port level 0)
- Stores `[0x8d6388]` / `[0x8d638c]` into the non-NULL out pointers (0x75d508..0x75d51e); returns 1.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75e230 _rpGeometrySortByMaterialCompare  (catalogue: specified/REA, port level 0)
- qsort comparator on 0x?-byte sort records: compares i16 `+4` signed (`jle`/`jge`, 0x75e241..0x75e24f) then u16 `+6` unsigned
  (0x75e25c..0x75e268); returns 1 / -1 / 0.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75e2d0 RpMaterialSetDefaultPipeline  (catalogue: specified/REA, port level 0)
- `(RxPipeline* p)`: `p == NULL` -> module+0x44 = module+0x5c (built-in default); else module+0x44 = p (0x75e2d4..0x75e2f9).
  Returns the argument / pipeline in EAX.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75b500 RpTriStripDefaultCost  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RpTriStripPolygon* start, RwUInt32 testFrame, void* data)`. Locals: `cost = 40000` (0x9c40, 0x75b518), `run = 1`
  (0x75b520), `swaps = 0`.
- `start->testFrame = testFrame` (0x75b530). `p1` = the polygon across the first strip edge of `start` whose `poly1` and `poly2` are
  both non-NULL (0x75b535..0x75b561); `p2 = FollowStrip(p1, start)`, `p3 = FollowStrip(p2, p1)` (inlined scans, each new polygon gets
  `testFrame`); any of them NULL -> return `cost + swaps` (0x75b76c).
- Loop over windows `(a, b, c, d)` of consecutive strip polygons (0x75b606..0x75b74c): `d->testFrame = testFrame`; if some vertex of
  `a` (vertIndex at +0x28, `j < a->numEdges`) also occurs in `b`, in `c` and in `d` (index found below `c->numEdges`, 0x75b63a..0x75b6a1)
  -> `swaps = 1`, `cost += 10000` (0x2710, 0x75b6d6..0x75b6e7); otherwise `swaps ? swaps++ : run++` (0x75b6c3..0x75b6ed). Shift the
  window and `d = FollowStrip(d, c)` (inlined) until NULL.
- Return `cost + (swaps > run ? run : swaps)` (unsigned compare, 0x75b752..0x75b777).
- librw: absent (librw has its own tri-striper).

## 0x75bb80 sub_75bb80 (proposed: _rpTriStripPathHasNoLoop)  (catalogue: unknown/REA, port level 0)
- `RwBool (path*)` with `+0 count`, `+0xc RpTriStripPolygon** polys` read as pairs `(A, B)` (i += 2, pointer += 8; 0x75bc45..0x75bc61).
  For each pair: walk the strip from `B` away from `A` (inlined FollowStrip, 0x75bba8..0x75bc3d); reaching `A` again -> return 0
  (0x75bc41..0x75bc77). All pairs pass -> 1.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75bc80 sub_75bc80 (proposed: _rpTriStripMeshTotalCost)  (catalogue: unknown/REA, port level 0)
- `RwUInt32 (ctx*)` with `+0 n`, `+8 costCB`, `+0xc RpTriStripPolygon** polys`. For `i = 0 .. n` INCLUSIVE (`jbe`, 0x75bd5d; reads one
  entry past `n` unless `n` is the last index): `p = polys[i]`; skip if `p->testFrame == [0x8d6390]` (global test frame, initial 1);
  else walk to the end of its strip (first strip edge with both polygons set, then FollowStrip, 0x75bcaa..0x75bd2e) and `sum +=
  costCB(end, [0x8d6390], ctx->polys)` (0x75bd44). After the loop `[0x8d6390] += 1` (0x75bd6b); return sum.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75c290 sub_75c290 (proposed: _rpTriStripPolygonFindNextEdge)  (catalogue: unknown/REA, port level 0)
- `RpTriStripEdge* (RpTriStripEdge* from, RpTriStripPolygon* p, u16 v, RwUInt32 testFrame)`: over `p->edges[0..2]` (always 3):
  first `e != from` that does not contain `v` (`vert1`/`vert2`), whose other polygon `q` is non-NULL, `q->testFrame != testFrame` and
  `q->+0x10 != 0` (in the free list) -> return e (0x75c2a4..0x75c2d9). None: a second pass without the vertex test (0x75c2f7..0x75c31d);
  none -> NULL.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75c380 RpBuildMeshGenerateTriStrip  (catalogue: specified/REA, port level 0)
- `RpMeshHeader* (RpBuildMesh* bm, RpTriStripData* d)` (d: +4 stripCB, +8 data).
- 1. `tris = rwmalloc(numTriangles*4, 0x10502)` (no NULL check); collect pointers to the non-degenerate triangles (`v0 != v1 && v1 != v2
  && v2 != v0`, 0x75c3bd..0x75c3e1) -> `n`. `qsort(tris, n, 4, 0x759640)` (CRT qsort 0x8247e0).
- 2. `runs = 1 + number of material changes between neighbours` (0x75c420..0x75c439); `sm = rwmalloc(runs*4, 0x10502)`.
- 3. For each run `[start, i)` of equal material (0x75c470..0x75c4b1) and finally the last run (0x75c4b3..0x75c4cf; executed even when
  `n == 0`, then with one uninitialised entry - kept): `sm[k] = 0x75c660(&tris[start], len)`; `d->stripCB(sm[k], d->data)`.
- 4. `h = rwmalloc(12*(n + runs) + 0x10, 0x30502)` (no NULL check); `numMeshes = runs`, `flags = 1` (TRISTRIP), `serialNum = 0`,
  `totalIndicesInMesh = 0`, `firstMeshOffset = 0` (0x75c501..0x75c525). Indices start after the `runs` mesh records. For each run: `mesh.indices
  = cursor`, `mesh.material = first triangle's material`, `0x75c9d0(sm[k], &mesh, d)` fills indices/numIndices, `totalIndicesInMesh +=
  numIndices`, `cursor += numIndices*2` (0x75c542..0x75c5d7).
- 5. `h = rwrealloc(h, used, 0x30502)` (G+0x13c, no NULL check) and the mesh index pointers are re-based on the new block (0x75c5f7..0x75c60f);
  `rwfree` each `sm[k]`, `sm`, `tris`; return h.
- librw: absent.

## 0x75d9d0 RpGeometryIsCorrectlySorted  (catalogue: specified/REA, port level 0)
- `RpGeometry* (const RpGeometry* g, RwBool* result)`. NATIVE -> `sum = 0`. Else `r = rwmalloc(numMaterials*4, 0x1050d)`; NULL -> return
  NULL (result untouched). Each entry `{u16 min = 0xffff, u16 max = 0}`; for every triangle (`matIndex` as i16 at +6) and each of its 3
  vertex indices update `max` (unsigned `>`), `min` (unsigned `<`) (0x75da33..0x75da71). `sum = Σ (max - min + 1)` over all materials with
  zero-extended u16 (an unused material contributes `0 - 65535 + 1`, lowering the sum - kept) (0x75da7e..0x75da9c); `rwfree(r)`.
- `*result = (sum <= g->numVertices)` (signed `jle`, 0x75dab7..0x75dacd); returns g.
- librw: absent.

## 0x75c660 sub_75c660 (proposed: _rpTriStripMeshCreate)  (catalogue: unknown/REA, port level 0)
- `RpTriStripMesh* (RpBuildMeshTriangle** tris, u32 n)`: `maxV` = largest vertex index of all `n` triangles (u16, 0x75c673..0x75c693).
  `buckets = rwmalloc(maxV*12, 0x10502)`, each with an empty circular RwLinkList at entry+4 (0x75c6b8..0x75c6c4; indexed by the smaller
  edge vertex, which is always < maxV for non-degenerate input). `links = rwmalloc(n*0x24, 0x10502)` (3 x 12-byte bucket nodes per
  triangle). `m = rwmalloc(148n + 0x2c, 0x30502)` (header 0x2c + n polygons x 0x34 + 3n edges x 0x20; none of the three allocations is
  checked). `m->numPolygons = n`; the five lists at +4, +0xc, +0x14, +0x1c, +0x24 are initialised empty (0x75c713..0x75c742).
- For each triangle i (polygon `p = m + 0x2c + i*0x34`): `p->vertIndex[0..2] = tri->v[0..2]` (0x75c766..0x75c794); `p+8` linked at the head
  of the list at m+0xc and `p+0` at the head of m+4 (0x75c798..0x75c7bf); `p->numEdges = 3`, `p->testFrame = 0`.
  For k = 0..2 with `(lo, hi) = sorted(v[k], v[(k+1)%3])` (0x75c7db..0x75c80b): search bucket[lo] for an edge `e` with `e->vert2 == hi &&
  e->poly2 == NULL` whose `poly1` shares exactly 2 of `p`'s vertices (0x75c834..0x75c8c1); found -> `p->edges[k] = e`, `e->poly2 = p`
  (0x75c9ab..0x75c9bf). Otherwise a new edge `e` (next free 0x20 slot): linked at the head of m+0x1c, `p->edges[k] = e`, `poly1 = p`,
  `poly2 = NULL`, `vert1 = lo`, `vert2 = hi`, `strip = 0`, plus a bucket node `{e, link}` inserted at the head of bucket[lo]
  (0x75c8d9..0x75c92d).
- `rwfree(links)`, `rwfree(buckets)` (0x75c988/0x75c998); return m.
- librw: tristrip.cpp:collectFaces/connectNodes is a different design (absent here).

## 0x75d240 sub_75d240 (proposed: _rpTriStripGetStartTriangle)  (catalogue: unknown/REA, port level 0)
- `void (RpTriStripPolygon* p, RpTriStripPolygon** o1, RpTriStripPolygon** o2, RpTriStripPolygon** o3, u16* va, u16* vb, u16* vc)`;
  only caller 0x75c9d0 (6 sites). `*o1 = p`; `q` = polygon across the first strip edge of `p` with both polygons set (0x75d25d..0x75d289);
  `*o2 = q`.
- `q == NULL`: `*o3 = NULL`, `(*va, *vb, *vc) = (p.v[0], p.v[1], p.v[2])` (0x75d298..0x75d2ca).
- Else `r = FollowStrip(q, p)` (inlined), `*o3 = r`. `*vc` = (r == NULL) ? the first vertex of `p` not in `q` (0x75d323..0x75d375) : the
  first vertex of `p` that is in `q` and in `r` (0x75d37d..0x75d411). If no such vertex exists the index equals `p->numEdges` and the
  u16 after the vertex array is read (kept). `*vb` = first vertex of `p` that is `!= *vc` and occurs in `q` (0x75d414..0x75d479); `*va` =
  first vertex of `p` equal to neither (0x75d47c..0x75d4b1).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75b780 RpTriStripMeshTunnel  (catalogue: specified/REA, port level 1)
- `RpTriStripMesh* (RpTriStripMesh* m, RpTriStripTunnelData* d)` (d: +0 f32 quality, +4 u32 lengthLimit); returns m.
- `d->lengthLimit == 0` -> return m (0x75b788..0x75b78d). `path = rwmalloc(lengthLimit*8 + 0x14, 0x10502)`: +0 length = 0, +4 maxLength = 1,
  +8 edge array (at path+0x10), +0xc polygon array (after the edges) (0x75b7b1..0x75b7cf). `work = rwmalloc(lengthLimit*36, 0x10502)` (search
  stack of 12-byte entries {edge, polygon, depth}). `limit = f32(pow(0.001, quality))` (CRT `_pow` 0x8220f0 with the double constant
  0x3f50624de0000000 = 0.001f widened, 0x75b801..0x75b813). Neither allocation is checked.
- Outer loop (0x75b81a..0x75ba50) over the polygons of the list at `m+4`: depth-first search from each polygon for an alternating path
  (non-strip edge, then strip edge, ...) of length <= maxLength (stack pushes at 0x75b86f..0x75b87b and 0x75b9a0..0x75b9a5; the parity of the
  path length selects the expected edge kind, 0x75b8b4..0x75b8b9, 0x75b961..0x75b96a). For a candidate path: invert it (0x75bb20 toggles every
  `edge->strip`), reject it if 0x75bb80 finds a loop, else compare total costs before/after with 0x75bc80 using `d` (0x75b8f9..0x75b91f) and keep
  the inversion only if it lowers the cost (otherwise invert back). An accepted path updates the end-polygon list (polygons with < 2 strip edges
  are linked into / removed from `m+4`, using 0x75bb50, 0x75baa5..0x75baf9), resets the path and restarts.
- After each full pass: `maxLength += 1`; `progress = progress*0.5 + (improvements / m->numPolygons)` (x87 `fild` u64, `fidiv`, `fmul 0.5`,
  0x75ba0f..0x75ba36); continue while `progress >= limit` (exactly-C0 test 0x75ba44) and `maxLength < lengthLimit` (0x75ba4d). Free `work`
  and `path`; return m.
- Confidence: low - block structure and constants verified, the search bookkeeping not traced instruction by instruction.
- librw: absent (librw's tunnel is a different algorithm).

## 0x75dae0 RpGeometrySortByMaterial  (catalogue: specified/REA, port level 0)
- `RpGeometry* (const RpGeometry* src, RpGeometrySortByMaterialCallBack cb)`; returns the new geometry or NULL.
- 1. Allocations (all `rwmalloc(.., 0x1050d)`): four per-morph-target pointer arrays (`numMorphTargets*4` each: src/dst vertices,
  src/dst normals) and `rec = 3*numTriangles` 8-byte sort records (0x75db01..0x75db83). With NORMALS (`flags & 0x10`) the normal arrays must
  be non-NULL; any NULL -> free what was allocated, return NULL (0x75dba0..0x75dbc8, 0x75e1b2..0x75e215).
- 2. Records: for triangle t, corner k: `{u32 key = (k << 30) | t, i16 mat = tri.matIndex, u16 vertex = tri.v[k]}` (0x75dbe4..0x75dc29);
  `qsort(rec, 3*numTriangles, 8, _rpGeometrySortByMaterialCompare)` (0x75dc3e).
- 3. `nv` = number of distinct consecutive `(mat, vertex)` pairs (0x75dc49..0x75dc80). `remap = rwmalloc(nv*2, 0x1050d)`; `g =
  RpGeometryCreate(nv, numTriangles, src->flags | (numTexCoordSets & 0xff) << 16)` (0x75dcc2), `RpGeometryLock(g, 0xfff)`; either NULL ->
  cleanup, NULL (0x75e152..0x75e1a0).
- 4. Walk the sorted records: a new pair -> `idx++`; `g->triangles[key & 0x3fffffff].v[key >> 30] = idx`, its `matIndex = src` triangle's,
  `remap[idx] = vertex` (0x75dd08..0x75dd86). Free `rec`.
- 5. `RpGeometryAddMorphTarget(g)` `numMorphTargets - 1` times; a count mismatch -> destroy g, free the arrays, NULL (0x75dda6..0x75de06).
- 6. For every new vertex i: prelit colour (if PRELIT), each morph target's vertex (and normal with NORMALS) and each texture-coordinate set
  are copied from `remap[i]` (0x75de07..0x75e013). `RpMorphTargetCalcBoundingSphere` for each target and the sphere stored (0x75e022..0x75e068).
  Free the four arrays.
- 7. `_rpMaterialListCopy(&g->matList, &src->matList)`; failure -> free remap, destroy g, NULL (0x75e0b4..0x75e0e3). Plugin data is copied
  RAW: for each entry of the list returned by 0x74bfd0 (+0 offset, +4 size, +0x30 next) `memcpy(g + off, src + off, size)` (0x75e0e4..0x75e10f;
  a shallow copy of every plugin's bytes). `cb(src, g, remap, nv)` if set (0x75e125); free remap; `RpGeometryUnlock(g)` fails -> destroy,
  NULL; else g.
- Confidence: low - the per-vertex copy loops (0x75de07..0x75e013) are summarised, not traced per array.
- librw: absent.

## 0x75bd80 RpTriStripMeshQuick  (catalogue: specified/REA, port level 0)
- `RpTriStripMesh* (RpTriStripMesh* m, void* data)`; returns m; no allocations. Locals: four circular list heads (buckets by the number of
  free neighbours 0..3, at [esp+0x28 + 8k], 0x75bd83..0x75bdce) and a running test-frame counter (starts 0; incremented per trial and forced
  to 1 when it wraps to 0, 0x75be8d..0x75bea4).
- 1. For each polygon on the list at `m+0xc` (polygon = link - 8): `k` = number of edges whose other polygon exists and differs from it
  (0x75bdd4..0x75bdfd); link `p+0x10` at the head of bucket[k] (0x75be04..0x75be19).
- 2. Repeat (0x75be28..0x75c271): take the first polygon `p` of the lowest non-empty bucket. Try the 6 starts (edge `j % 3`, start vertex
  `vert1` for j < 3 else `vert2`): walk with 0x75c290 stamping `testFrame`, count the polygons reached (0x75be51..0x75bf10); keep the longest.
  Unlink `p` from its bucket; walk the chosen start again setting `edge->strip = 1`, clearing each reached polygon's testFrame and unlinking
  it from its bucket (0x75bf16..0x75bfee). Then follow the new strip from `p` to its end (0x75bff4..0x75c091) and for the neighbours of every
  strip polygon recompute their free-neighbour count and move them to the matching bucket (0x75c093..0x75c261). Restart from bucket 0; stop
  when all four buckets are empty (0x75c26d..0x75c271).
- Confidence: low - control flow and data structures verified, the per-step bookkeeping summarised.
- librw: absent (librw's buildStrips is a different heuristic).

## 0x75c9d0 sub_75c9d0 (proposed: _rpTriStripMeshWriteIndices)  (catalogue: unknown/REA, port level 0)
- `void (RpTriStripMesh* m, RpMesh* mesh, RpTriStripData* d)`; caller RpBuildMeshGenerateTriStrip (2 sites). `keepWinding = (d->ignoreWinding
  == 0)` (0x75c9db..0x75c9ea; later toggled with `xor 3` to track strip parity, values 1/2); `out = mesh->indices` (u16*).
- Strips are emitted from the polygons of the end-polygon list at `m+4` (0x75c9fd..0x75ca9e, 0x75cad2): for each strip start
  `_rpTriStripGetStartTriangle` 0x75d240 gives the first polygons and the vertex order (va, vb, vc). With `keepWinding` the start vertex
  position is checked against the parity (0x75ca46..0x75caab) and the start may be rotated. Every strip after the first is stitched with
  degenerate triangles: the previous last index is repeated and the new first index written twice (`out[0] = out[-1]`, `out[1] = va`,
  0x75cab7..0x75cac7 / 0x75cb0f..0x75cb2a). Then `va, vb, vc` are written (0x75cb2d..0x75cb4f) and the strip is followed polygon by polygon
  (inlined FollowStrip), appending the vertex not on the shared edge; where the next vertex would break the alternation an extra duplicate
  index is inserted and the parity flipped (`xor 3`, 0x75cb52..0x75d223; six 0x75d240 call sites in total).
- End: `mesh->numIndices = (out - mesh->indices) >> 1` (`sar`, 0x75d228..0x75d231).
- Confidence: low - strip emission and stitching verified, the swap/parity cases summarised.
- librw: tristrip.cpp:makeMesh plays the same role with different stitching rules.

## 0x75a960 sub_75a960 (proposed: _rpBuildMeshGenerateExhaustiveTriStripCommon)  (catalogue: unknown/REA, port level 3)
- `RpMeshHeader* (RpBuildMesh* bm, void* data, RwBool keepWinding)`; callers RpBuildMeshGenerateExhaustiveTriStrip (keepWinding 1) and
  ...IgnoreWinding (0). Structure only (877 instructions):
  - 0x75a960..0x75a9c6: triangle pointer array (`rwmalloc(.., 0x30502)`), `qsort(.., 0x759640)` by material.
  - 0x75aa05..0x75ab4a: per-material work arrays (hints 0x1010502 / 0x10502) and a node freelist `_rwFreeListCreate(0x10, ..)` (0x75aafa).
  - 0x75abd3: 0x75a090 builds the adjacency; 0x75ac31..0x75ad0e: strip node allocation from freelists (G+0x144) and result buffers.
  - 0x75ae82..0x75b01b: exhaustive search per start triangle with 0x75a250 / 0x75a370 (strip extension and scoring helpers).
  - 0x75b051..0x75b14b: growing the output (freelist allocations and rwmalloc 0x30502); 0x75b192..0x75b378: freeing nodes
    (G+0x148), freelists (`RwFreeListDestroy` x3) and temporaries; 0x75b275: 0x75a660 merges the strips.
  - 0x75b3c3: `_rpMeshHeaderCreate`, fill the meshes and indices; 0x75b474..0x75b4c0: free the remaining arrays; return the header.
- Confidence: low (block map only; the helper functions 0x75a090/0x75a250/0x75a370/0x75a660 are not specified in this pass).
- librw: absent.

