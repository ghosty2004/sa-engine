# _unattributed-REST (CALLED-ONLY) — RE-SPEC-REST

## 0x424100 sub_424100 (proposed: `ReturnFalse` — a shared, COMDAT-folded `bool f() { return false; }`)  (catalogue: unknown/REA, port level 0)
- `__cdecl` (no args read; also safe as thiscall), body `xor al, al; ret` (0x424100): returns AL = 0. EAX upper bytes are whatever the
  caller had (only AL is defined).
- Callers: CAutomobile::ProcessControl 0x6B1880, CBike::ProcessControl 0x6B9250, CTrain::ProcessControl 0x6F86A0 (gr Train.cpp:913
  keeps it as a commented-out `!plugin::Call<0x424100>()` term next to `m_nFakePhysics > 10`). Port: constant false.

## 0x4A4880 FindClosestPtOnLine(CVector* out, const CVector* a, const CVector* b, const CVector* p)  (catalogue: specified/REA, port level 0)
- `__cdecl`: out [esp+4], a [esp+8] (ECX), b [esp+0xC] (EDX), p [esp+0x10] (EAX); plain `ret`; leaves EAX = out. Caller:
  FxInfoAttractLine_c::GetValue 0x4A5850. gr: `plugin::Call<0x4A4880>` (FxInfoAttractLine.cpp:16).
- Behaviour (x87 order exact):
  1. `dx = b.x - a.x`, kept on the stack AND stored as f32 into the `b` argument slot [esp+0xC] (`fst`, 0x4A4890); `dy = b.y - a.y`,
     `dz = b.z - a.z` kept in registers (0x4A4894-0x4A489D).
  2. `num = (((p.z - a.z) * (b.z - a.z)) + ((p.y - a.y) * (b.y - a.y))) + ((p.x - a.x) * dx_f32)` — the z and y deltas of the line are
     recomputed from memory, x uses the stored f32 (0x4A48A0-0x4A48C6).
  3. `den = ((dz*dz) + (dy*dy)) + (dx*dx)` (register values, 0x4A48C8-0x4A48D6); `t = num / den` (0x4A48D8). a == b gives 0/0 = NaN.
  4. Clamp: `fcom [0x858B50]=0.0; test ah, 5; jp` — t < 0 → t = 0.0f (0x4A48ED); else `fcom [0x858624]=1.0f; test ah, 0x41; jne` —
     t > 1 → t = 1.0f (0x4A4904). NaN passes both tests unchanged.
  5. `out.x = (dx_f32 * t) + a.x`; `out.y = ((b.y - a.y) * t) + a.y`; `out.z = ((b.z - a.z) * t) + a.z` (0x4A490C-0x4A4933), t not rounded.

## 0x5DD100 AtomicCreatePrelitIfNeeded(RpAtomic* atomic)  (catalogue: specified/REA, port level 8)
- `__cdecl`, atomic [esp+4]; returns EAX = atomic in both paths; plain `ret`. Caller: CPlantMgr::LoadModels 0x5DD220. gr:
  `plugin::Call<0x5DD100>` (PlantMgr.cpp:11). Callees: RpGeometryCreate 0x74CA90, RpGeometryTriangleGetMaterial 0x74C760,
  RpGeometryTriangleSetMaterial 0x74C6C0, RpGeometryUnlock 0x74C800, RpAtomicSetGeometry 0x749D40.
- RW offsets used: atomic+0x18 geometry; geometry +0x08 flags, +0x10 numTriangles, +0x14 numVertices, +0x2C triangles (8 bytes each),
  +0x34 texCoords[0] (8 bytes per vertex), +0x5C morphTarget array (morphTarget +0x14 = vertices, 12 bytes each).
- Behaviour:
  1. `old = atomic->geometry`; if `old.flags & 8` (rpGEOMETRYPRELIT) → return atomic (0x5DD10B).
  2. `g = RpGeometryCreate(numVertices, numTriangles, 0xD)` (TRISTRIP|TEXTURED|PRELIT; arg order verts, tris, flags; 0x5DD121-0x5DD12D).
     No NULL check.
  3. Copy `numVertices*12` bytes of morphTarget[0].vertices (old → g), then `numVertices*2` dwords of texCoords[0], then
     `numTriangles*2` dwords of triangles (0x5DD141-0x5DD181; rep movsd/movsb).
  4. For each triangle i (0x5DD191 loop, only if numTriangles > 0): `RpGeometryTriangleSetMaterial(g, &g.triangles[i],
     RpGeometryTriangleGetMaterial(old, &old.triangles[i]))`.
  5. `RpGeometryUnlock(g)`; `g.flags |= 2` (rpGEOMETRYPOSITIONS, 0x5DD1C4); `RpAtomicSetGeometry(atomic, g, 1)`
     (rpATOMICSAMEBOUNDINGSPHERE). Normals, prelight colours (left as RpGeometryCreate leaves them) and further UV sets are not copied;
     the old geometry is only released through RpAtomicSetGeometry's reference counting.

## 0x734DE0 RpGeometryReplaceOldMaterialWithNewMaterial(RpGeometry* g, RpMaterial* oldMat, RpMaterial* newMat)  (catalogue: specified/REA, port level 4)
- `__cdecl`, args [esp+4]/[esp+8]/[esp+0xC]; returns AL (bool: any replacement); `ret`. Caller: CCustomCarPlateMgr::MaterialUpgradeSetCarplate
  0x6FDF50. Callees: _rpMaterialListFindMaterialIndex 0x74E420 (cdecl (matList*, mat)), RpMaterialDestroy 0x74DA20.
- Behaviour: `hdr = g->mesh` (+0x54, RpMeshHeader); `n = (u16)hdr.numMeshes` (+4); result = 0; if n <= 0 return 0. For each mesh
  (RpMesh array right after the 0x10-byte header, stride 0xC, material at +8; pointer starts at hdr+0x18): if `mesh.material == oldMat`:
  `idx = _rpMaterialListFindMaterialIndex(&g->matList (+0x20), mesh.material)`; `RpMaterialDestroy(mesh.material)`;
  `g->matList.materials[idx] = newMat` (+0x20 → materials array); `mesh.material = newMat`; `newMat->refCount (+0x18, i16)++`; result = 1.
- Edge: if oldMat appears in two meshes, the second lookup no longer finds it in the material list (idx = -1 from RW) and the original
  writes materials[-1]; RpMaterialDestroy is called once per matching mesh. Keep.

## 0x734E50 RwTexDictionaryFindHashNamedTexture(RwTexDictionary* dict, uint32 hash)  (catalogue: specified/REA, port level 1)
- `__cdecl`, args [esp+4], [esp+8]; returns EAX = RwTexture* or NULL; `ret`. Callers: CHud::DrawWeaponIcon 0x58D7D0,
  CHud::DrawCrossHairs 0x58E020, 0x59EEB0. Callee: CKeyGen::GetUppercaseKey 0x53CF30 (cdecl, const char*).
- Behaviour: walk the dictionary's texture list (head link at dict+8; `node = head.next` until node == head, 0x734E5A-0x734E80);
  `tex = node - 8` (link at RwTexture+8); `name = tex + 0x10`; if name != NULL (only false for a node at address 8; dead check,
  0x734E6B) and `GetUppercaseKey(name) == hash` → return tex. End → return NULL. First match in list order wins.

## 0x734FC0 RpClumpGetBoundingSphere(RpClump* clump, RwSphere* sphere, bool useLTM)  (catalogue: specified/REA, port level 4)
- `__cdecl`, args [esp+4]/[esp+8]/[esp+0xC] (u8); returns EAX = clump, or NULL; `ret`. Caller: CRealTimeShadow::SetupForThisEntity
  0x706520. gr: raw call wrapper (RwHelper.cpp:263). Global: u8 [0x8D60BC] (`useLTM` flag shared with the two callbacks; .data initial 1).
- Behaviour:
  1. `[0x8D60BC] = useLTM` (stored first, even on failure, 0x734FCF). If clump == NULL or sphere == NULL → return NULL.
  2. `*sphere = {0,0,0, radius 0}` (0x734FE6-0x735019).
  3. `nf = (float)RpClumpGetNumAtomics(clump)` (0x7498E0; fild, stored f32). If `nf < [0x858624]=1.0f` (C0 only; `jnp`, 0x73503B) → return NULL
     (sphere stays zeroed).
  4. `c = {0,0,0}`; `RpClumpForAllAtomics(clump, cbSumCentres 0x734970, &c)`.
  5. `inv = 1.0f / nf`; `c.x *= inv; c.y *= inv; c.z *= inv` (each stored f32). `s = {c.x, c.y, c.z, r = 0}`;
     `RpClumpForAllAtomics(clump, cbMaxRadius 0x734ED0, &s)`.
  6. `m = invert(useLTM ? RwFrameGetLTM(clump->frame) : clump->frame->modelling)` (frame at clump+4, modelling at frame+0x10;
     RwMatrixInvert 0x7F2070, RwFrameGetLTM 0x7F0990); `RwV3dTransformPoints(&s.center, &s.center, 1, &m)` (0x7EDD90).
  7. `*sphere = {s.center, s.radius}`; return clump.
- Callback 0x734970 (cbSumCentres(atomic, data), returns atomic; not separately catalogued): if atomic interpolator flags (+0x4C) & 2 →
  `_rpAtomicResyncInterpolatedSphere(atomic)` 0x7491F0. `p = transform(atomic->boundingSphere.center (+0x1C))` by the CLUMP's frame
  (`atomic+0x3C` = clump, clump+4 = frame) LTM if [0x8D60BC] else its modelling matrix. Then **ORIGINAL BUG**: `data.x += p.x;
  data.y += p.x; data.z += p.y` (0x7349DF-0x7349F9 load [esp+8], [esp+8], [esp+0xC]) — y accumulates x and z accumulates y.
- Callback 0x734ED0 (cbMaxRadius(atomic, data)): resync as above (the flag test runs twice when set, 0x734ED9 and 0x734EFB); `p` as above;
  `d = fsqrt(((dz*dz) + (dy*dy)) + (dx*dx))` with `dx = p.x - data.x` etc.; `r = d + atomic->boundingSphere.radius (+0x28)`; if
  `r > data.radius` (`test ah, 0x41; jne` skips on <, =, NaN) → `data.radius = r`.
- Ports must keep the centre bug; the resulting sphere is used for real-time shadows.

## 0x735140 SkinGetBonePositions(RpClump* clump)  (catalogue: specified/REA, port level 2)
- `__cdecl`, clump [esp+4]; void; `ret`. Callers: none in game code (gr raw wrapper RwHelper.cpp:268). Globals: once-flag u8 [0xC88658];
  table `g_BonePositions[]` at 0xC88258, stride 0x10: +0 i32 parent index, +4 RwV3d position (so pos[i] at 0xC8825C + 16*i).
- Behaviour:
  1. If `[0xC88658] != 0` return; set it to 1 (0x73516B) — the function runs once per process.
  2. `atomic` = first atomic (RpClumpForAllAtomics with RwHelper::GetFirstAtomicCallback 0x734810, init NULL); `skin =
     RpSkinGeometryGetSkin(atomic->geometry)` 0x7C7550 (no NULL check on atomic); `hier` via RwHelper::SkinAtomicGetHAnimHierarchCB
     0x734A20 (init NULL).
  3. `pos[0] = {0,0,0}`, `parent[0] = -1` (0x7351A5-0x7351B7). `n = RpSkinGetNumBones(skin)` 0x7C77E0; if n <= 1 return.
  4. `parent = 0`; pointer stack `sp` = local area start (first push writes one slot above it); for i = 1 .. n-1 (0x735200 loop):
     `inv = RwMatrixInvert(copy of RpSkinGetSkinToBoneMatrices(skin)[i])`; `RwV3dTransformPoints(&pos[i], &inv.pos, 1,
     &skinToBone[parent])` (matrices fetched again with 0x7C7810 and copied to the stack); `g.parent[i] = parent`;
     `flags = hier->pNodeInfo[i].flags` (+0x10 nodeInfo array, stride 0x10, flags at +8);
     if flags & 2 (PUSH): `*++sp = parent`; if flags & 1 (POP): `parent = *sp--` else `parent = i`.
     No bounds check on the stack (about 29 slots of local space) or on the global table size.

## 0x7352D0 SkinSetBonePositions(RpClump* clump)  (catalogue: specified/REA, port level 1)
- `__cdecl`, clump [esp+4]; void. Callers: none in game code (gr raw wrapper RwHelper.cpp:273). Reads the table filled by
  SkinGetBonePositions 0x735140 (parent at 0xC88258 + 16*i, pos at 0xC8825C + 16*i).
- Behaviour: first atomic (0x734810 callback), `skin = RpSkinGeometryGetSkin(atomic->geometry)`, hierarchy (0x734A20 callback),
  `M = RpHAnimHierarchyGetMatrixArray(hier)` 0x7C5120, `n = RpSkinGetNumBones(skin)`. If n > 1, for i = 1 .. n-1 (ESI from 0xC8826C,
  EDI = &M[1].pos = M + 0x70, step 0x40): `RwV3dTransformPoints(&M[i].pos, &pos[i], 1, &M[parent[i]])` (0x735337-0x735353).
  Only the position rows are written.

## 0x735360 SkinGetBonePositionsToTable(RpClump* clump, RwV3d* table)  (catalogue: specified/REA, port level 2)
- `__cdecl`, clump [esp+4], table [esp+8]; void. Caller: RpAnimBlendClumpInitSkinned 0x4D6510 (gr RpAnimBlend.cpp:124 notes it).
- Behaviour: if table == NULL return (0x735373). Same algorithm as SkinGetBonePositions 0x735140 but without the once-flag and
  writing into the caller's table: first atomic → skin → hierarchy; `table[0] = {0,0,0}`; n = RpSkinGetNumBones; for i = 1..n-1:
  `table[i] = skinToBone[parent] * inverse(skinToBone[i]).pos` (RwMatrixInvert 0x7F2070 into a stack matrix, RwV3dTransformPoints
  0x7EDD90 with the parent matrix copied to the stack), then the PUSH (flags & 2) / POP (flags & 1) parent stack logic on
  `hier->pNodeInfo[i].flags` (index i*0x10 + 8), `parent = i` when not popping. No parent indices are stored.

## 0x821195 sub_821195 = `operator new[](size_t)`  (catalogue: unknown/REA, port level 0)
- A 5-byte `jmp 0x82119A` (operator new). cdecl, size at [esp+4], returns EAX. gr MemoryMgr.cpp:454 / LinkList.h:20 call it as
  `operator new[]`. NAME: operator new[] (MSVC CRT). Port: same allocator as operator new.

## 0x8213AE sub_8213ae = `operator delete[](void*)`  (catalogue: unknown/REA, port level 1)
- A 5-byte `jmp 0x8214BD` (operator delete). cdecl, pointer at [esp+4]. gr MemoryMgr.cpp:462 / LinkList.h:24. NAME: operator delete[].

## 0x8214BD sub_8214bd = `operator delete(void*)`  (catalogue: unknown/REA, port level 0)
- A 5-byte `jmp 0x82413F` (CRT `_free`). cdecl, pointer at [esp+4], NULL allowed (free semantics). gr MemoryMgr.cpp:458 calls it as
  `operator delete`. NAME: operator delete (bundles elsewhere label this address `_free`; the real _free is 0x82413F). 282 callers.
