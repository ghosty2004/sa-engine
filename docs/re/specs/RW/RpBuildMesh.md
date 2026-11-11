# RpBuildMesh: tri-strip and tri-list mesh builders (RE-SPEC-RW-2 writer B)

Build-mesh to mesh-header conversion of RW 3.6 (world plugin, statically linked), used by `RpGeometryUnlock` 0x74c800
and `RpWorldUnlock` 0x74f210 through `_rpMeshOptimise`. All functions `__cdecl`. Engine = [0xc97b24]; engine+0x134
malloc(size, hint), +0x138 free, +0x144/+0x148 freelist alloc/free. Error reporting `RwErrorSet({2, _rwerror(code,
arg)})` (0x808820 / 0x8088d0).

## Layouts (offsets proven at the cited instructions)

- RpBuildMesh (RpMesh.md): +0x4 numTriangles (0x7591ec), +0x8 RpBuildMeshTriangle* (0x759215).
- RpBuildMeshTriangle (0x14): +0x0/+0x2/+0x4 u16 vertIndex[3] (0x759932-0x759948, 0x75a129-0x75a16d), +0x8 RpMaterial*
  (0x759654), +0xc u16 matIndex, +0xe u16 textureIndex, +0x10 u16 rasterIndex, +0x12 u16 pipelineIndex
  (0x7596f1-0x759712).
- MESHMODULE engine data at `engine + [0xc9b8c0]`: +0x0 u16 nextSerialNum (0x759563, 0x75957f), +0x4 RwFreeList* of
  strip nodes (created 0x75937c, destroyed 0x7594f6).
- Strip node (0x10, from module+0x4): +0x0 RwUInt16* indices, +0x4 numIndices, +0x8 capacity, +0xc next
  (0x75990a-0x759914, 0x75a694-0x75a6af).
- TriStripTri (0x24, own freelist): +0x0 triangle index, +0x4/+0x8/+0xc edge[3], +0x10 next / +0x14 prev in the
  neighbour-count bucket list, +0x18 used, +0x1c used-in-trial, +0x20 u8 numNeighbours (0x75a107-0x75a18b, 0x75a273).
- TriStripEdge (0x10, own freelist): +0x0 u16 v0, +0x2 u16 v1, +0x4 tri0, +0x8 tri1 (0 if open), +0xc next
  (0x75a1fd-0x75a216).
- Data: `[0x8d6388]` current tri-strip generator (initial 0x7591b0), `[0x8d638c]` its user data; 0x8d6378 static
  empty RpMeshHeader returned for zero triangles (0x75d98a).

## 0x7591b0 RpBuildMeshGenerateDefaultTriStrip  (catalogue: specified/REA, port level 5)
- `RpMeshHeader* (RpBuildMesh* bm, void* data)`: return `_rpBuildMeshGenerateTriStripCommon(bm, 0, 1)` (0x7591b4-0x7591b9;
  data ignored). Siblings (not in this assignment): IgnoreWinding 0x75a8e0 = (bm, 0, 0), Preprocess 0x75a900 = (bm, 1, 1),
  PreprocessIgnoreWinding 0x75a920 = (bm, 1, 0).
- librw: tristrip.cpp (different algorithm, see 0x759790).

## 0x7591d0 _rpBuildMeshGenerateTriStripCommon  (catalogue: unknown/REA, port level 4)
- `RpMeshHeader* (RpBuildMesh* bm, RwBool preprocess, RwBool keepWinding)`. n = bm->numTriangles.
- `tris = malloc(n*4, 0x10502)`; NULL -> return 0 (0x759206). `tris[i] = &bm->tri[i]` (stride 0x14, 0x759215-0x759227).
  `qsort(tris, n, 4, _rpBuildMeshMaterialCompare 0x759640)` (0x759237).
- numMat = 1 + number of material changes along the sorted array (only when n >= 2, 0x759242-0x759269).
- `meshes = malloc(numMat*4, 0x1010502)`, `runs = malloc(numMat*0xc, 0x10502)` (no NULL checks, 0x75927e/0x75929a).
  Run i: +0x4 triangle count, +0x8 material (+0x0 zeroed, unused). The loop 0x7592d2-0x759336 stores the run start in
  +0x4 and converts it to a count at the next change; the last run gets `n - start` (0x75933c-0x759356).
- Strip freelist: `module+4 = _rwFreeListCreate(0x10, n/10 + 5, 4, 0x10502)` (n/10 via 0xcccccccd mul, 0x759351-0x75937c).
- For each run (sorted triangle pointer advancing): `local = malloc(count*0x14)`, copy the 5 dwords of each triangle
  (0x7593c1-0x7593e4); `_rpBuildMeshStripify(local, count, &list, preprocess)` (0x7593f5); `_rpTriStripMeshCreate(&list,
  keepWinding)` (0x759404, merges the run's strips into one node). For each node in list (one): `mesh = malloc(2*num +
  0xc, 0x10502)`: +8 material = run material, +4 numIndices, +0 indices = mesh+0xc, indices copied (0x759418-0x75945d);
  `meshes[meshCount++ & 0xffff] = mesh` (0x759474). Then every list node: free(indices), freelist free (0x759486-0x7594c2);
  free(local).
- `RwFreeListDestroy(module+4)`, module+4 = 0 (0x7594f6-0x759519).
- `size = 0x10 + sum(2*numIndices + 0xc)`, `total = sum(numIndices)` over meshCount (u16) meshes (0x759528-0x75953b);
  `mh = _rpMeshHeaderCreate(size)` (no NULL check); flags = 1 (TRISTRIP), numMeshes = meshCount, serialNum =
  module.nextSerialNum, firstMeshOffset = 0, totalIndicesInMesh = total; nextSerialNum++ (0x75954c-0x75957f).
- RpMesh array at mh+0x10, index data after `meshCount*0xc`: each mesh {indices, numIndices, material} with a memcpy
  of the indices; the temporary mesh is freed (0x759594-0x7595f9). Free tris, meshes, runs; return mh.
- librw: tristrip.cpp differs (librw builds one header with its own stripifier; no per-material run arrays).

## 0x759640 _rpBuildMeshMaterialCompare  (catalogue: unknown/REA, port level 0)
- qsort comparator `int (RpBuildMeshTriangle** pa, RpBuildMeshTriangle** pb)`. a/b = the triangles.
- Same material pointer -> 0 (0x75965a-0x75966e).
- Alpha key per material m (0 or 0x10): 0x10 if m->texture && m->texture->raster && `((raster->cFormat (+0x23) & 0xf) << 8)`
  is 0x100 (C1555), 0x300 (C4444) or 0x500 (C8888), or if m->color.alpha (+7) != 0xff (0x759670-0x7596ec). NULL
  material -> 0.
- `lt = (a.raster < b.raster)*8 | (a.pipeline < b.pipeline)*4 | (a.texture < b.texture)*2 | (a.matIndex < b.matIndex)`
  (unsigned u16 compares via sbb, 0x75971a-0x759743); `gt` = same with a and b swapped (0x75974b-0x759782).
  Return `(gt | keyA) - (lt | keyB)` (0x759784-0x759787). Effect: opaque before alpha, then raster, pipeline, texture,
  material index.
- librw: absent.

## 0x759790 _rpBuildMeshStripify  (catalogue: unknown/REA, port level 3)
- `RwBool (RpBuildMeshTriangle* tris, RwUInt32 n, StripNode** list, RwBool preprocess)`; returns 1.
- Setup (0x759790-0x7598a2): `nodes = _rpTriStripAdjacencyCreate(n, &ctx, &edgeList, tris)` (ctx = {tri freelist,
  edge freelist}); four bucket heads B[0..3] = 0; every node i (ascending) is pushed to the front of B[numNeighbours]
  (0x7597e9-0x75981b). Two scratch strips A and Bk from the strip freelist (module+4) with capacity `2n+2` and
  `indices = malloc(cap*4)` (0x75981d-0x7598a2).
- Main loop while `done < n` (0x7598bb .. 0x759f92):
  - `mode0 = preprocess ? 0 : 3` (neg/sbb/and/add, 0x7598bf-0x7598cc), best = 0.
  - B[0] non-empty (isolated triangle t, 0x7598d5): new strip node {3 indices = t's vertIndex[0..2]}, capacity 3,
    pushed to the front of *list; t.used = t.trial = 1; pop B[0]; done++ (0x7598db-0x759979).
  - Else t = head of the first non-empty bucket B[k], k >= 1 (0x759982-0x7599a0). With `av(x) = x && !x.used` and
    `S(e) = av(e.tri0) + av(e.tri1)` (t itself counts, it is unused) for t's edges e0, e1, e2, the start index s is
    (0x7599a4-0x759c43): `S(e2) >= 2 && S(e1) >= 2` -> 1; else `S(e0) >= 2 && S(e2) >= 2` -> 2; else
    `S(e1) >= 2 && S(e0) >= 2` -> 0; else if `S(e0) > S(e1)`: `1 + (S(e0) > S(e2))`; else `(S(e1) <= S(e2)) ? 1 : 0`.
    bestStart = s, snapshot = done.
  - Pass loop (0x759c5c): for every node trial = used (0x759c5e-0x759c73). `start` = s, (s+1)%3, (s+2)%3 for mode 0, 1, 2,
    and bestStart for mode >= 3 (switch 0x759c75-0x759c9e); `mode += 1` before use. Forward edge = t.edge[(start+1)%3],
    backward edge = t.edge[start] (0x759ca0-0x759cd9). A = {v[start%3], v[(start+1)%3], v[(start+2)%3]}, A.n = 3
    (0x759cdd-0x759d2e). `_rpTriStripMarkUsed(t, B, mode)`; `cnt = snapshot + 1 + _rpTriStripWalk(A, fwd, B, tris, mode)`
    (0x759d35-0x759d5a); done = cnt.
    If the backward edge has an available triangle (trial flag when mode < 4, used flag otherwise, 0x759d57-0x759dc5):
    Bk = {A[1], A[0]}, n = 2; `cnt += _rpTriStripWalk(Bk, back, B, tris, mode)`; if Bk.n is odd append Bk[n-2]
    (0x759dcc-0x759e28). If `cnt > best` (unsigned): best = cnt, bestStart = start (0x759e2b / 0x759e47).
    mode < 4 -> next pass. mode >= 4 -> commit:
    - with Bk: node cap = A.n + Bk.n - 2; output = Bk[Bk.n-1], Bk[Bk.n-2], ..., Bk[2] (Bk.n decremented to 2) followed by
      a copy of `A.n*4` BYTES of A (twice the index bytes - copies garbage after A's valid indices but stays inside both
      allocations); n = cap (0x759e67-0x759f16).
    - without: node cap = n = A.n, copy `A.n*4` bytes (0x759f1b-0x759f82). Node pushed to the front of *list.
- Cleanup (0x759f98-0x75a07e): free both scratch strips (indices + freelist entries), free every edge in edgeList
  and destroy the edge freelist, free every TriStripTri and destroy that freelist, free `nodes`. Return 1.
- Confidence medium: the start heuristic and the trial/commit protocol are decoded from the bytes above; port
  0x759982-0x759c46 instruction by instruction.
- librw: tristrip.cpp differs (librw's stripifier walks with a different start heuristic and no trial passes).

## 0x75a090 _rpTriStripAdjacencyCreate  (catalogue: unknown/REA, port level 2)
- `TriStripTri** (RwUInt32 n, RwFreeList* ctx[2], TriStripEdge** edgeList, const RpBuildMeshTriangle* tris)`.
- `nodes = malloc(n*4, 0x30502)`; `ctx[0] = _rwFreeListCreate(0x24, n, 4, 0x30502)`, `ctx[1] = _rwFreeListCreate(0x10,
  n/2 + 1, 4, 0x30502)` (0x75a0a5-0x75a0e3).
- For i < n: node from ctx[0]: numNeighbours = 0, index = i, next = prev = used = 0 (+0x1c not initialised,
  0x75a107-0x75a126); edge[0] = EdgeAdd(ctx[1], edgeList, v0, v1, i, nodes), edge[1] = (v1, v2), edge[2] = (v2, v0)
  (0x75a13c, 0x75a15c, 0x75a17c). Return nodes. No NULL checks.
- librw: tristrip.cpp builds adjacency differently (per-vertex lists).

## 0x75a1b0 _rpTriStripEdgeAdd  (catalogue: unknown/REA, port level 0)
- `TriStripEdge* (RwFreeList* fl, TriStripEdge** head, RwUInt16 a, RwUInt16 b, RwUInt32 i, TriStripTri** nodes)`.
- Scan *head for an edge with `e.v1 == a && e.v0 == b && e.tri1 == NULL` (reverse direction, still open,
  0x75a1c7-0x75a1de). Found: `e.tri0->numNeighbours++`, `nodes[i]->numNeighbours++` (u8 wrap), `e.tri1 = nodes[i]`,
  return e (0x75a21f-0x75a242).
- Else new edge from fl: v0 = a, v1 = b, tri0 = nodes[i], tri1 = 0, pushed to the front of *head; return it.

## 0x75a250 _rpTriStripMarkUsed  (catalogue: unknown/REA, port level 0)
- `void (TriStripTri* t, TriStripTri** buckets, RwInt32 mode)`.
- mode < 4 (signed): `t.trial (+0x1c) = 1` and return (0x75a257-0x75a262).
- Else t.used = 1; unlink t from buckets[t.numNeighbours] (head case resets the new head's prev, 0x75a27d-0x75a2b4).
  For each of t's 3 edges: neighbour = e.tri0 if non-NULL, != t and unused, else e.tri1 if non-NULL and unused
  (0x75a2c2-0x75a2e3); if any: unlink it from buckets[k], k -= 1 (u8), push to the front of buckets[k]
  (0x75a2e5-0x75a347).

## 0x75a370 _rpTriStripWalk  (catalogue: unknown/REA, port level 1)
- `RwUInt32 (StripNode* s, TriStripEdge* e, TriStripTri** buckets, void* /*unused*/, RwInt32 mode)`; returns the
  number of triangles added. "free(x)" = x && !(mode < 4 ? x.trial : x.used).
- Loop (0x75a38d): t = e.tri0 if free, else e.tri1 if free, else return (also e == NULL). count++,
  `_rpTriStripMarkUsed(t, buckets, mode)` (0x75a3f6).
  k = index of e in t.edge (0..2; -1 if absent); `f = t.edge[(k+1)%3]` (signed idiv); new vertex
  `v = (f.tri0 == t) ? f.v1 : f.v0` (0x75a41f-0x75a441).
  last = s[n-1]; exit = first of t.edge[0..2] equal to the unordered pair (last, v), else NULL (0x75a446-0x75a4c1).
  If exit has a free triangle on either side: append v and continue with e = exit (0x75a615-0x75a62b).
  Else choose `alt` = first of t.edge[0..2] that is neither the entering edge nor exit (else the previous alt, initial
  NULL) (0x75a538-0x75a572). If alt has a free triangle and n is even: swap: s[n] = s[n-2]; s[n+1] = v; n += 2;
  continue with e = alt (0x75a5ed-0x75a62b). Otherwise append v and return (0x75a63d).
- librw: tristrip.cpp walk differs (no swap insertion rule identical to this).

## 0x75a660 _rpTriStripMeshCreate  (catalogue: unknown/REA, port level 0)
- `RwBool (StripNode** list, RwBool keepWinding)`: joins all strips of *list into one node.
- *list NULL -> return 0. out = freelist alloc (module+4, 0x30502); out.n = 0; out.cap = sum(numIndices + 6);
  `out.indices = malloc(cap*4, 0x30502)` (4 bytes per u16 slot) (0x75a672-0x75a6cd).
- Append the first node's indices; free its indices and node (0x75a6dc-0x75a722).
- While nodes remain (0x75a733): with L = out[n-1], P = out[n-2]:
  1. node starting with L (first in list order): if n odd and keepWinding: out[n++] = node[0] (0x75a776-0x75a796); else
     nothing extra (0x75a778).
  2. else node starting with P: if n odd or !keepWinding: out[n++] = node[0]; else out[n] = out[n+1] = node[0], n += 2
     (0x75a79b-0x75a7cd).
  3. else the head node: n odd and keepWinding: out[n] = L, out[n+1] = out[n+2] = node[0], n += 3; else out[n] = L,
     out[n+1] = node[0], n += 2 (0x75a7d0-0x75a833).
  Then append all node indices, free its indices, unlink it (head or predecessor search), free the node
  (0x75a839-0x75a8b0).
- `*list = out`, out.next = 0, return 1.
- librw: absent (librw concatenates differently).

## 0x75d530 _rpTriListMeshGenerate  (catalogue: specified/REA, port level 1)
- `RpMeshHeader* (RpBuildMesh* bm, void* data /*unused*/)`. n = bm->numTriangles (n > 0 assumed: the copy loop is a
  do-while, 0x75d667).
- `tris = malloc(n*4, 0x10502)`; NULL -> `RwErrorSet({2, _rwerror(0x80000013, n*4)})`, return 0 (0x75d6ed-0x75d717).
  Pointers + qsort by 0x759640, material count m (0x75d560-0x75d5ba).
- `size = 6n + 12m + 0x10` (0x75d5bc-0x75d5c2); `mh = _rpMeshHeaderCreate(size)`; NULL -> free tris, same error with size.
- flags = 0 (TRILIST), numMeshes = 1, serialNum = module.nextSerialNum, firstMeshOffset = 0, totalIndicesInMesh = 3n;
  nextSerialNum++ (0x75d61a-0x75d64c). Index data starts at mh + 0x10 + 12m.
- For each sorted triangle: a material change opens the next RpMesh (numIndices 0, indices = write pointer, material),
  numMeshes++; write v0, v1, v2 and numIndices += 3 (0x75d667-0x75d6c4).
- `_rpMeshHeaderForAllMeshes(mh, 0x75d730, NULL)` (0x75d6ce), free tris, return mh.
- librw: absent as such (librw keeps the build order).

## 0x75d730 _rpTriListMeshGenerateCallback  (catalogue: unknown/REA, port level 0)
- `RpMesh* (RpMesh* mesh, RpMeshHeader*, void*)`: reorders the triangles of one tri-list mesh; returns mesh.
- `nt = numIndices/3` (unsigned); `or = OR of all indices`; `bytes = (or + 0x1f) >> 3`; `bitmap = malloc(bytes, 0x10502)`;
  `copy = malloc(2*numIndices, 0x10502)` filled from mesh.indices; any NULL -> free what exists, return mesh
  (0x75d73b-0x75d7ca).
- Passes while emitted < nt (0x75d7df-0x75d930): clear the bitmap; for each triangle (a, b, c) of copy: skip if
  `(a & b & c) == 0xffff` (already emitted) or if bit a, b or c is set (bit x = `bitmap[x>>5] >> (x&31)`); else set the
  three bits, write a, b, c to the next output slot of mesh.indices, overwrite the copy triangle with 0xffff x3,
  emitted++ (0x75d805-0x75d910).
- Edge case: the bitmap is `(max+31)>>3` bytes but is accessed as dwords `[x>>5]`, so up to 3 bytes past the
  allocation can be read/written (e.g. or = 0x20 allocates 7 bytes, writes byte 4..7) - original behaviour, keep the
  same sizes or over-allocate in the port without changing results.
- free copy and bitmap.

## 0x75d970 _rpMeshOptimise  (catalogue: specified/REA, port level 6)
- `RpMeshHeader* (RpBuildMesh* bm, RwUInt32 flags)`. bm NULL -> 0.
- `bm->numTriangles == 0` -> `_rpBuildMeshDestroy(bm)`, return 0x8d6378 (0x75d981-0x75d98a).
- `flags & 1` (TRISTRIP): `mh = [0x8d6388](bm, [0x8d638c])`; else `mh = _rpTriListMeshGenerate(bm, NULL)`
  (0x75d992-0x75d9af). mh != NULL -> destroy bm, return mh; else return 0 and bm is NOT destroyed (0x75d9c8).
- librw: geometry.cpp buildMeshes differs (no build-mesh object, no strip method pointer).

## Open questions
- UNCLEAR: 0x759790 relies on TriStripTri+0x1c being initialised by the first pass reset (0x759c5e) before MarkUsed /
  Walk read it in trial mode; the isolated-triangle path (0x7598db) writes it explicitly. No uninitialised read was
  found, but the reset loop runs only on the non-isolated path.
