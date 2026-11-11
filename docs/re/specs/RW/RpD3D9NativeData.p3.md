# RW D3D9 native-data plugin (pre-instanced geometry / world-sector streaming) - pass-3 specs, writer A (RE-SPEC-RW-3)

RW 3.6 D3D9 world-plugin code (nativedata). gta-reversed only has thunks. All `__cdecl`.

## Layouts used (offsets proven by the instruction addresses given)

- `RpGeometry` +0x8 flags (0x1000000 NATIVE, 0x7637fa), +0x20 material list (0x763875), +0x58 repEntry (0x76380d). `RpWorldSector`
  +0x34 repEntry (0x7645d1); `RpWorld` +0x8 flags (0x7645de).
- `RwResEntry` +0x8 size of the data after the 0x18-byte entry header (0x763819); `RxD3D9ResEntryHeader` at res+0x18: +0x4
  numMeshes (0x763861), +0x8 indexBuffer (0x763834), +0x10 vertexStream[2] (0x10 each: +0 vb, +4 offset, +8 stride; 0x7639ba..0x7639f3),
  +0x34 vertexDeclaration (0x763823), +0x38 totalNumIndex (0x76383b), +0x3c totalNumVertex (0x763853), +0x40 instances (0x24 each,
  material at +8 = hdr+0x48, 0x763868).
- `IDirect3DVertexDeclaration9` vtbl+0x10 GetDeclaration(elements, &count) (0x763829); index/vertex buffer vtbl+0x2c Lock,
  +0x30 Unlock (0x76397f/0x7639a2).
- Platform id `[0x8d7384] = 0x9` (dword, written first, 0x7638b7).

## 0x7637f0 _rpD3D9NativeDataGeometryStreamWrite  (catalogue: specified/REA, port level 0)
- `RwStream* (RwStream* s, const RpGeometry* g)` (g = [esp+8] at 0x7637f0; called from cb_NATIVEDATAPLUGIN_RpGeometry_StreamWrite
  0x750e00): not NATIVE -> return s (0x763807/0x763a5d).
- `res = g->repEntry`, `hdr = res + 0x18`; `GetDeclaration(elems, &ne)` into a local buffer (0x763829). Size = `res->size + 8 +
  ne*8 + 4 + (indexBuffer ? totalNumIndex*2 : 0) + 2*0x10 + sum over streams with vb of stride*totalNumVertex` (0x76382c..0x76385f).
- Every instance's material pointer is replaced in place by `_rpMaterialListFindMaterialIndex(&g->matList, mat)` (0x763868..0x763887).
- `Header(s, 1, size, 0x36003, 0xffff)`; raw writes: platform (4 bytes from 0x8d7384), `res->size` (4), the `res->size` bytes at
  `res+0x18` (header + instances), `ne` (4), `ne*8` element bytes (0x7638b5..0x763950).
- Index buffer (if any): `Lock(0, 0, &p, 0)`; `p != NULL` -> write `totalNumIndex*2` bytes, `Unlock`; a NULL pointer or failed write ->
  return NULL (0x763967..0x7639a9).
- For both streams: write the 0x10-byte stream record; if `vb`: `vb->Lock(offset, stride*totalNumVertex, &p, 0x800)` (non-zero HRESULT
  -> NULL), write `stride*totalNumVertex` bytes, `Unlock`, failed write -> NULL (0x7639bd..0x763a26).
- Finally the material indices are converted back with `_rpMaterialListGetMaterial` (0x763a3c..0x763a4e); return s.
- Bug kept: every early NULL return after the conversion leaves the instances holding material INDICES instead of pointers.
- librw: d3d/d3d9.cpp native geometry write differs (librw serialises its own instance layout).

## 0x763a80 _rpD3D9NativeDataGeometryStreamRead  (catalogue: specified/REA, port level 0)
- `RpGeometry* (RwStream* s, RpGeometry* g)` (g at entry+8, 0x763b35; returns g or NULL). `rem` = chunk length bookkeeping only.
- `RwStreamFindChunk(s, 1, &len, &ver)` fail -> NULL; `ver` outside [0x34000, 0x36003] -> `RwErrorSet({2, _rwerror(0x80000004)})`,
  NULL (0x763aae..0x763e8d). Read platform (4) - must be 9 (0x763aee); read `size` (4); `res = rwmalloc(size + 0x18, 0x3050d)` (no
  NULL check, 0x763b2f); `g->repEntry = res`; `res->link = {0, 0}`, `owner = g`, `size`, `ownerRef = &g->repEntry`, `destroyNotify =
  0x4c9990` (0x763b43..0x763b69); read `size` bytes to `res+0x18` (short read -> NULL, entry stays attached).
- `hdr = res+0x18`: `indexBuffer = NULL`, both stream vbs NULL, `vertexDeclaration = NULL`; read `ne` (4) and `ne*8` element bytes;
  `RwD3D9CreateVertexDeclaration(elems, &hdr->vertexDeclaration)` (result ignored, 0x763c29).
- If `totalNumIndex`: `RwD3D9IndexBufferCreate(totalNumIndex, &hdr->indexBuffer)` fail -> `_rxD3D9DefaultDestroyInstanceCallback(res)`,
  NULL; `Lock(0, 0, &p, 0)`; `p` -> read `totalNumIndex*2` bytes, `Unlock`; read count != `2*totalNumIndex` (also when `p == NULL`) ->
  destroy, NULL (0x763c60..0x763cad).
- Streams (2): read the 0x10-byte record (short -> destroy, NULL). If the record's vb field is non-zero: `n++`; `bytes = stride *
  totalNumVertex`; `g->numMorphTargets > 1` -> `managed = 0`, `RwD3D9DynamicVertexBufferCreate(bytes, &record)`; else `managed = 1`,
  `RwD3D9CreateVertexBuffer(stride, bytes, &record.vb, &record.offset)` (0x763d1a..0x763d3a). `vb->Lock(offset, bytes, &p, managed ?
  0x800 : 0x2800)`, read `bytes`, `Unlock`; `failed = (read != bytes)` (0x763d51..0x763da3). A creation failure is skipped only while
  `failed` is still set from before (0x763d46..0x763d4c); otherwise the NULL buffer is locked (crash; kept). After both streams
  `failed` -> destroy, NULL (0x763dca..0x763dd5). (`failed` starts at 1, so a geometry with no vertex stream fails.)
- `n > 1`: `useOffsets = 1` only if `[0xc9bfd4] & 1` (D3DCAPS9.DevCaps2 STREAMOFFSET), every `inst.baseIndex = inst.minVert`
  (0x763df7..0x763e11); else `useOffsets = 0` and `inst.baseIndex = stream[0].offset / stream[0].stride + inst.minVert` (unsigned
  div, 0x763e15..0x763e36). Instance material indices -> pointers via `_rpMaterialListGetMaterial(&g->matList, idx)`; return g.
- librw: d3d/d3d9.cpp native read differs (librw rebuilds its own instance data).

## 0x763eb0 _rpD3D9NativeDataGeometryStreamGetSize  (catalogue: specified/REA, port level 0)
- Not NATIVE -> 0. Else `GetDeclaration(elems, &ne)`; `size = res->size + 0x14 + 2*totalNumIndex + 8*ne + 4` (index bytes counted
  even without an index buffer, unlike the writer, 0x763ee3..0x763ef0), then for each of the 2 streams `+0x10` and `+ stride *
  totalNumVertex` if `vb` (0x763ef9..0x763f0f).
- librw: absent.

## 0x7645c0 _rpD3D9NativeDataWorldSectorStreamGetSize  (catalogue: specified/REA, port level 0)
- `sector->repEntry != NULL && RpWorldSectorGetWorld(sector)->flags & 0x1000000` -> the same formula as 0x763eb0 on the sector's
  resource entry (0x7645ec..0x764634); else 0.
- librw: absent.

## 0x763f20 _rpD3D9NativeDataWorldSectorStreamWrite  (catalogue: specified/REA, port level 0)
- `RwStream* (RwStream* s, const RpWorldSector* sec)`: `w = RpWorldSectorGetWorld(sec)` (0x763f32); `w->flags & 0x1000000` clear -> return
  s (0x763f3d/0x76419d). Otherwise identical to 0x7637f0 on `sec->repEntry` (+0x34, 0x763f4d), with the material list of the WORLD
  (`w + 0x10`, 0x763fb4 for the pointer->index conversion and 0x764179 for the conversion back).
- Confidence: block structure compared with 0x7637f0 (same call sequence, 0x763f4d..0x76419d); not re-traced instruction by instruction.
- librw: absent (no native world sectors in librw).

## 0x7641c0 _rpD3D9NativeDataWorldSectorStreamRead  (catalogue: specified/REA, port level 0)
- `RpWorldSector* (RwStream* s, RpWorldSector* sec)`: same as 0x763a80 on `sec->repEntry` with these differences: the material list is
  `RpWorldSectorGetWorld(sec) + 0x10` (0x764559); vertex buffers are always created with `RwD3D9CreateVertexBuffer(stride, bytes, &vb,
  &offset)` and `managed = 1` (0x764457/0x76445b; no dynamic-buffer path, no morph-target test). Version check [0x34000, 0x36003]
  (0x764203/0x76420e) and platform 9 (0x76423f) as in the geometry reader.
- Confidence: compared by call list and key compares, not fully re-traced.
- librw: absent.

