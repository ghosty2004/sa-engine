# CClothesBuilder (player clothes geometry/texture builder) - full specs, writer A (RE-SPEC-RW-3)

Game code (CClothesBuilder, 0x5a42b0..0x5a6860) that builds the CJ clothes clump from PLAYER.IMG parts. gta-reversed
(source/game_sa/ClothesBuilder.cpp) installs most of these with `.Reversed = false` or as `plugin::Call`; where it has
C++ it is only a reading aid and every difference to the bytes is listed. All `__cdecl` unless stated.

## Layouts used (offsets proven by the instruction addresses given)

- `CPedClothesDesc` (0x78, gr VALIDATE_SIZE): +0x00 `u32 m_anModelKeys[10]` (0x5a465b `[esi + edi*4]`), +0x28 `u32
  m_anTextureKeys[18]` (0x5a46d5 `[esi + eax*4 + 0x28]`), +0x70 `f32 m_fFatStat` (0x5a42b3).
- `RpGeometry`: +0x10 numTriangles (0x5a6571), +0x14 numVertices (0x5a656e, 0x5a4a2d), +0x2c triangles (`RpTriangle`,
  8 bytes: u16 v[3], u16 mat, 0x5a659d/0x5a66a2), +0x34 texCoords[0] (0x5a49f4), +0x5c morphTarget (0x5a49d7);
  `RpMorphTarget`: +0x4 boundingSphere (16 bytes, 0x5a6726..0x5a673e), +0x14 verts (0x5a49e4), +0x18 normals (0x5a49e7).
  `RpAtomic` +0x18 geometry (0x5a49d4); `RpClump` +0x4 frame (0x5a43a9).
- `CBaseModelInfo` pointer table `CModelInfo::ms_modelInfoPtrs` at 0xa9b0c8; model `384 + i` (MODEL_CLOTHES01) entry
  = `[0xa9b6c8 + 4*i]` (base constant 0xa9b6c4 + 4 at 0x5a55b2/0x5a55bd); `mi+0x13 |= 2` (gr bHasComplexHierarchy,
  0x5a55d0); `mi+0x1c` = RpClump* (0x5a5647).
- `CClothes::ms_numRuleTags` `[0xbc12fc]` (u32), `CClothes::ms_clothesRules` `[0xbc1300]` (u32[600]); `playerImg`
  CDirectory at 0xbc12c0; `CClothes::ms_clothesImageId` `[0xbc12f8]`; `CTxdStore::ms_pTxdPool` `[0xc8800c]` (CPool:
  +0 objects (0xc bytes each, +0 RwTexDictionary*), +4 flags byte array, negative = free; 0x5a60ea..0x5a6108);
  `Scene.m_pRwCamera` `[0xc1703c]`; `CStreaming::ms_pStreamingBuffer` `[0x8e4cac]`, `ms_streamingBufferSize`
  `[0x8e4ca8]`.
- Texture-name pointer table (.data, `char*`): `[0x8d0a4c]` "player_torso", `[0x8d0a50]` "player_legs", `[0x8d0a54]`
  "player_face", `[0x8d0a58]` "player_feet", `[0x8d0a5c]` "torso", `[0x8d0a60]` "torso_fat", `[0x8d0a64]`
  "torso_ripped", `[0x8d0a68]` "legs", `[0x8d0a6c]` "legs_fat", `[0x8d0a70]` "legs_ripped", `[0x8d0a74]` "face",
  `[0x8d0a78]` "face_fat", `[0x8d0a7c]` "torso", `[0x8d0a80]` "head", `[0x8d0a88]` "legs", `[0x8d0a8c]` "feet",
  `[0x8d0a90]` "necklace", `[0x8d0a94]` "watch", `[0x8d0a98]` "glasses", `[0x8d0a9c]` "hat", `[0x8d0aa0]` "extra1".
  Strings at 0x867150 ".DFF", 0x867160 "normal", 0x85a508 "fat", 0x867168 "ripped".
- Constants: `[0x858a48] = 200.0f`, `[0x858b50] = 0.0f`, `[0x865354] = 0.00125f` (0x3aa3d70a), `[0x858cdc] = 0.001f`
  (0x3a83126f), `[0x858624] = 1.0f`.

## 0x5a42b0 CClothesBuilder::nullptr (proposed: CClothesBuilder::CalcBlendRatios)  (catalogue: specified/REA, port level 1)
- Convention (register, no stack args): EAX = `CPedClothesDesc*`, ESI = `f32* rFat`, EDI = `f32* rMuscle`, EBX = `f32*
  rNormal`; `ret` (callers rely on ESI/EDI/EBX being preserved; the function does not touch them except as pointers).
  gr inlines it in CreateSkinnedClump (ClothesBuilder.cpp:540 "Calculate blend ratios").
- Behaviour (x87): `fat = desc->m_fFatStat` (copied as a dword, 0x5a42b3); `m = f32(CStats::GetStatValue(0x17))`
  (STAT_MUSCLE, 0x5a42bc/0x5a42c1). `t = fat - [0x858a48] 200.0` (0x5a42cc, register). If `m < 0.0` (0x5a42d6..0x5a42e3;
  NaN keeps m) `m = 0`. If `t < 0.0` (0x5a42eb..0x5a42fa) `t = 0.0`. `*rFat = f32(t * [0x865354] 0.00125)` (0x5a4300),
  `*rMuscle = f32(m * [0x858cdc] 0.001)` (0x5a430c). If `*rFat > 1.0` -> 1.0 (0x5a4316..0x5a4323; NaN not clamped); same
  for `*rMuscle` (0x5a432b..0x5a4338). `*rNormal = 1.0 - (*rFat + *rMuscle)` (0x5a433e..0x5a4348). `s = *rFat + *rMuscle`;
  if `s > 1.0` (0x5a434e..0x5a4359): `inv = 1.0 / s`; `*rFat *= inv`, `*rMuscle *= inv`, `*rNormal = 0` (0x5a435b..0x5a436b).
- gta-reversed differs: uses `/ 1000.f` and `/ 800.f` (exe multiplies by the f32 constants 0.001 and 0.00125, rounding
  differently), `std::clamp` (NaN behaviour differs), and tests `rNormal <= 0` instead of `s > 1.0` (at `s == 1.0` the
  exe does not renormalise; the result is the same apart from rounding). Port keeps the bytes.
- librw: not RW code.

## 0x5a4380 CClothesBuilder::nullptr (proposed: CClothesBuilder::AtomicInstanceCB)  (catalogue: specified/REA, port level 3)
- `RpAtomic* (RpAtomic* a, void* data)`: `RpAtomicInstance(a)` (0x5a4386), return `a`. Used by 0x5a43a0 (pushed at 0x5a43e4).
- gta-reversed: commented install only ("AtomicInstanceCB"). librw: not RW code.

## 0x5a43a0 CClothesBuilder::nullptr (proposed: CClothesBuilder::RebuildClumpThroughStream)  (catalogue: specified/REA, port level 15)
- Convention: `RpClump* (RpClump* clump)`; returns the re-read clump or NULL. Frame: `RwMemory mem` at [esp+0x8]
  (start, length), `RwV3d zero` at [esp+0x14].
- Behaviour: `CStreaming::FlushChannels()` (0x5a43ac); `RwFrameScale(clump->frame, &{0,0,0}, 0 /*REPLACE*/)` (0x5a43b9..0x5a43d1);
  `RwCameraBeginUpdate([0xc1703c])` (0x5a43dd); `RpClumpForAllAtomics(clump, 0x5a4380, NULL)` (instances every atomic,
  0x5a43ea); `RwCameraEndUpdate([0xc1703c])` (0x5a43f6). `mem = {[0x8e4cac], [0x8e4ca8] << 12}` (0x5a43fb..0x5a4416);
  `s = RwStreamOpen(3 /*MEMORY*/, 3, &mem)` (0x5a441a); if `s`: `s->memory.position (+0xc) = 0` (0x5a442a),
  `RpClumpStreamWrite(clump, s)` (0x5a4431), `RwStreamClose(s, &mem)` (0x5a443c). `RpClumpDestroy(clump)` (0x5a4445,
  always). `s = RwStreamOpen(3, 1 /*READ*/, &mem)` (0x5a4455); if `s`: if `RwStreamFindChunk(s, 0x10 /*rwID_CLUMP*/, NULL,
  NULL)` (0x5a4468) -> `result = RpClumpStreamRead(s)` (0x5a4475); `RwStreamClose(s, NULL)` (0x5a4482). Return result.
- Edge: the clump frame is scaled to zero before instancing (the instanced copy is invisible) - the written stream holds
  the scaled frame. The streaming buffer is used as scratch (size in bytes = buffer size << 12).
- gta-reversed: absent (commented install). librw: not RW code.

## 0x5a44a0 CClothesBuilder::nullptr (proposed: CClothesBuilder::DestroyTextureCB)  (catalogue: specified/REA, port level 3)
- `RwTexture* (RwTexture* t, void* data)`: `RwTexDictionaryRemoveTexture(t)` (0x5a44a6), `RwTextureDestroy(t)` (0x5a44ac),
  return `t` (dangling). gta-reversed: commented install. librw: not RW code.

## 0x5a44c0 CClothesBuilder::PreprocessClothesDesc  (catalogue: specified/REA, port level 2)
- Convention: `void (CPedClothesDesc* desc, bool applyCuts)` (desc [esp+0x50], flag byte [esp+0x54] after the 0x3c frame
  and 4 pushes). Locals: `u32 keys[8]` at [esp+0x2c] (all = -1 via `rep stosd` of 8 dwords, 0x5a44c5..0x5a44d1), `u8
  flag[8]` at [esp+0x24] (NOT initialised; written only together with a key), rule args a0 [0x10], a1 [0x14], a2 [0x18],
  a3 [0x1c], tag [0x20].
- Interpreter over `CClothes::ms_clothesRules` (tags = gr eClothRule: 0 CUTS, 1 SETC, 2 TEX, 3 HIDE, 4 END_IGNORE,
  5 IGNORE, 6 END_EXCLUSIVE, 7 EXCLUSIVE). `p = 0`; if `ms_numRuleTags == 0` return (0x5a44dc). Loop (do-while `p <
  ms_numRuleTags`, unsigned, 0x5a4764):
  1. `tag = rules[p++]`, `a1 = -1` (0x5a44f0..0x5a4502). `tag > 7` -> step 3 with no args. Jump table 0x5a4778:
     - CUTS (0x5a45c1): `a0 = rules[p]`, `a2 = rules[p+1]`, `p += 2`.
     - SETC (0x5a45da): `a0 = rules[p]`, `a1 = rules[p+1]` (model part), `a2 = rules[p+2]`, `a3 = rules[p+3]`, `p += 4`.
     - TEX (0x5a4607): `a0 = rules[p]`, `a3 = rules[p+1]`, `p += 2`. HIDE (0x5a4620): `a0 = rules[p]`, `a1 = rules[p+1]`,
       `p += 2`.
     - IGNORE (0x5a4513) / EXCLUSIVE (0x5a456f): first `i` in 0..7 with `keys[i] == -1`: `keys[i] = rules[p]`, `flag[i] =
       1` (IGNORE) / 0 (EXCLUSIVE), `p += 1`. If all 8 are used: nothing and `p` is NOT advanced (the argument is
       then read as the next tag; original bug, 0x5a4521/0x5a457d).
     - END_IGNORE (0x5a453b) / END_EXCLUSIVE (0x5a4597): `k = rules[p++]`; first `i` with `keys[i] == k`: `keys[i] = -1`,
       `flag[i] = 1` / 0.
  2. (all tags fall into 3, also 4..7.)
  3. For `part = 0 .. 9` (0x5a4638..0x5a475e): `chk = (a1 != -1) ? a1 : part` (0x5a4640..0x5a464b). Blocked if some `i`
     has `keys[i] != -1` and `(desc->models[chk] == keys[i]) == flag[i]` (0x5a464d..0x5a4673). If not blocked and `tag <= 3`
     (table 0x5a4798):
     - CUTS (0x5a468d): if `applyCuts` and `desc->models[part] == a0`: `desc->SetModel(a2, part)` (0x5a7910, thiscall).
     - SETC (0x5a46b1): if `desc->models[part] == a0`: if `a1 == 2`: `desc->SetModel(a2, 2)` (0x5a4714); else: if `a3 ==
       0` -> `a3 = desc->textures[GetDependentTexture(a1)]` (0x5a46cf..0x5a46dc); if `a2 == 0` -> `a2 = desc->models[a1]`;
       `desc->SetTextureAndModel(a3, a2, GetDependentTexture(a1))` (0x5a470d). The filled a2/a3 persist for later parts.
     - TEX (0x5a471d): if `desc->models[part] == a0`: `desc->SetTextureAndModel(a3, a0, GetDependentTexture(part))` (0x5a473c).
     - HIDE (0x5a4743): if `desc->models[part] == a0`: `desc->SetModel(0, a1)` (0x5a4755).
- Edge: a constraint tests `models[a1]` (for SETC/HIDE) rather than `models[part]`. Uninitialised `flag[i]` is never read
  for a key still at -1.
- gta-reversed: `plugin::Call` only. librw: not RW code.

## 0x5a47b0 CClothesBuilder::ReleaseGeometry  (catalogue: specified/REA, port level 9)
- `void (i32 n)`: if `n == 0` return (0x5a47b5); do { `n--`; `CStreaming::SetModelIsDeletable(n + 0x180)` } while `n != 0`
  (0x5a47c0..0x5a47d2) - models 384+n-1 down to 384.
- gta-reversed matches (`for (i = n; i; SetModelIsDeletable(384 + i)) --i;`). librw: not RW code.

## 0x5a4940 CClothesBuilder::BlendGeometry-3  (catalogue: specified/REA, port level 8)
- Convention: `RpGeometry* (RpClump* clump, const char* n0, const char* n1, const char* n2, f32 r0, f32 r1, f32 r2)`;
  frame 0x80 + 4 pushes (`r0..r2` at [esp+0xa4], [0xa8], [0xac] in the base frame). Returns the geometry of atomic
  `n0`, modified in place and AddRef'd.
- Setup: for k = 0..2 `atom[k]` = result of `RpClumpForAllAtomics(clump, 0x5a47e0, &{name_k, NULL})` (name-search callback,
  0x5a496d/0x5a4994/0x5a49bb; atom slots [0x18], [0x1c], [0x20]). For each k (0x5a49d0..0x5a4a1c): `g_k = atom->geometry`,
  `V_k = g_k->morphTarget->verts`, `N_k = ...->normals`, `U_k = g_k->texCoords[0]`, `skin = RpSkinGeometryGetSkin(g_k)`,
  `I_k = RpSkinGetVertexBoneIndices(skin)` (u32 per vertex), `W_k = RpSkinGetVertexBoneWeights(skin)` (4 f32 per vertex).
  No NULL checks (a missing atomic crashes).
- `RpGeometryLock(g_0, 0xfff)` (0x5a4a28). For `v = 0 .. g_0->numVertices-1` (signed, re-read each iteration 0x5a4edf;
  entry `jle` 0x5a4a39), x87:
  1. `V0.x = r1*V1.x + r0*V0.x + r2*V2.x` (0x5a4a54..0x5a4a79); `V0.y = r1*V1.y + r2*V2.y + r0*V0.y` (0x5a4a7b..0x5a4a9d);
     `V0.z = r1*V1.z + r2*V2.z + r0*V0.z` (0x5a4aa0..0x5a4aca).
  2. `N0.x = r2*N2.x + r1*N1.x + r0*N0.x` (0x5a4acd..0x5a4aec); `N0.y = r1*N1.y + r2*N2.y + r0*N0.y`; `N0.z = r1*N1.z +
     r2*N2.z + r0*N0.z` (to 0x5a4b35); `RwV3dNormalize(&N0, &N0)` (0x5a4b38, return popped).
  3. `U0.u = r2*U2.u + r1*U1.u + r0*U0.u` (0x5a4b43..0x5a4b6d); `U0.v = r1*U1.v + r2*U2.v + r0*U0.v` (0x5a4b6f..0x5a4bbe).
  4. `f32 w[5] = 0` ([0x6c..0x7c], 0x5a4b8f..0x5a4bb1), `u8 b[4] = 0` (reusing the atom slot [0x18..0x1b], 0x5a4bb9..0x5a4be7).
     Twelve calls `AddWeightToBoneVertex(w, b, f32(r_k * W_k[v].w_j), I_k[v].byte_j)` (0x5a4840) in the order k = 0, 1, 2
     and j = 0..3 inside (0x5a4bec..0x5a4dbf).
  5. `I0[v].byte_j = b[j]` and `W0[v].w_j = w[j]` for j = 0..3 (0x5a4ddd..0x5a4e16). If `w[4] != 0.0` (or NaN;
     `test ah,0x44; jnp` 0x5a4e13/0x5a4e19): `s = w[3] + w[2] + w[1] + w[0]`, `inv = 1.0 / s`, `W0[v].w_j *= inv`
     (0x5a4e1b..0x5a4e4a).
  6. All pointers advance (verts/normals 0xc, uv 8, indices 4, weights 0x10; 0x5a4e4d..0x5a4ecd).
- `RpGeometryUnlock(g_0)` (0x5a4ef2), `_rpGeometryAddRef(g_0)` (0x5a4ef8), return `g_0`.
- Edge: `w` has 5 slots; AddWeightToBoneVertex may write `w[i+1]` up to index 8 (gr 0x5a4840 loops to 8), i.e. into
  the following stack slots, when more than 5 distinct bones occur. Kept.
- gta-reversed (generic `BlendGeometry<N>`, ClothesBuilder.cpp:158) differs: indexes bone indices/weights with
  `vtxIdx = i + wi` (exe: vertex `v`, byte/weight `j`), different sum order (`multiply_weighted`), and uses
  `RpGeometryGetVertexTexCoords(g, 1)` (= texCoords[0], same). Port follows the bytes.
- librw: absent (game code, not part of RenderWare).

## 0x5a4f10 CClothesBuilder::BlendGeometry-2  (catalogue: specified/REA, port level 8)
- Convention: `RpGeometry* (RpClump* clump, const char* n0, const char* n1, f32 r0, f32 r1)`; frame 0x60 + 4 pushes; the
  `n0` argument slot [0x78] is reused as the vertex counter (0x5a4fd5).
- As 0x5a4940 with two models (atom slots [0x40], [0x44]; per-model arrays geoms [0x38], verts [0x28], normals [0x30],
  uvs [0x20], indices [0x10], weights [0x18]; 0x5a4f70..0x5a4fbc). Per vertex:
  `V0.k = r1*V1.k + r0*V0.k` for x, y, z (0x5a4ff4..0x5a5041); `N0.k = r1*N1.k + r0*N0.k` (0x5a5044..0x5a5089),
  `RwV3dNormalize(&N0, &N0)` (0x5a508c); `U0.u = r0*U0.u + r1*U1.u` (0x5a5097..0x5a50da), `U0.v = r1*U1.v + r0*U0.v`
  (0x5a50dc..0x5a50f2); `w[5] = 0` ([0x4c..0x5c], 0x5a50ae..0x5a50d2); eight `AddWeightToBoneVertex` calls (model 0 then 1,
  0x5a5110..0x5a5231); copy back and the same `w[4] != 0.0` renormalisation `s = w3 + w2 + w1 + w0` (0x5a5236..0x5a52af).
- DIFFERENCE to the 3-way version: the bone byte array `b[4]` (slot [0x40..0x43], which first holds `atom[0]`) is NEVER
  zeroed (no byte stores to it except the copy-out; verified: only reads at 0x5a523a..0x5a5262). Slots whose weight stays
  0 therefore keep stale bytes (the atom pointer on the first vertex, the previous vertex's bones later) and those are
  written into `I0[v]`. Kept as original behaviour.
- `RpGeometryUnlock`, `_rpGeometryAddRef`, return `g_0` (0x5a5319/0x5a531f).
- gta-reversed: generic template (see 0x5a4940), zero-initialises the bone array (differs).
- librw: absent (game code, not part of RenderWare).

## 0x5a5340 CClothesBuilder::CopyGeometry  (catalogue: specified/REA, port level 8)
- Convention: `RpGeometry* (RpClump* clump, const char* dstName, const char* srcName)`; frame 0x38 + 4 pushes; the
  `dstName` slot [0x50] is reused as the vertex counter (0x5a5405).
- Finds both atomics (0x5a5364, 0x5a5388) and collects per model (k = 0 dst, 1 src): geoms [0x28+4k], verts [0x20+4k],
  normals [0x10+4k], uvs [0x18+4k], indices [0x30+4k], weights [0x38+4k] (0x5a53a0..0x5a53ec).
- `RpGeometryLock(g0, 0xfff)` (0x5a53f8); for `v < g0->numVertices` (signed): copy vertex (3 dwords), normal (3 dwords),
  uv (2 dwords), the 4 index bytes and the 4 weight dwords from model 1 to model 0, interleaved as in the bytes
  (0x5a5423..0x5a5497). `RpGeometryUnlock(g0)` (0x5a54f7).
- Then the collection loop runs again (0x5a54ff..0x5a554d) followed by a loop that only advances local pointers
  `g0->numVertices` times (0x5a5570..0x5a5580): no observable effect besides the getter calls. `_rpGeometryAddRef(g0)`
  (0x5a5587); return g0.
- gta-reversed: `plugin::CallAndReturn` only. librw: not RW code.

## 0x5a55a0 CClothesBuilder::ConstructGeometryArray  (catalogue: specified/REA, port level 16)
- Convention: `void (RpGeometry** out, u32* modelKeys, f32 normal, f32 fat, f32 strength)`.
- For `i = 0 .. 9` (model `m = 0x180 + i`, esi; loop `jl` on `i < 10` 0x5a56a2):
  - `modelKeys[i] == 0` -> `out[i] = NULL` (0x5a5687..0x5a568b).
  - else: `mi = [0xa9b6c8 + 4*i]`; `mi+0x13 |= 2` (0x5a55d0). Inlined RequestGeometry: `h = CKeyGen::AppendStringToKey(key,
    ".DFF")` (0x5a55da); `playerImg.FindItem(h, &pos, &size)` (thiscall 0x5324d0, result ignored); `CStreaming::RequestFile(m,
    pos, size, [0xbc12f8], 0x12)` (0x5a560a). `CStreaming::LoadAllRequestedModels(1)` (0x5a5611). If `i + 1 < 10` and
    `modelKeys[i+1] != 0`: `CClothesBuilder::RequestGeometry(m + 1, modelKeys[i+1])` (0x5a562f) and
    `CStreaming::LoadRequestedModels()` (0x5a5637). `out[i] = BlendGeometry(mi->clump (+0x1c), "normal", "fat", "ripped",
    normal, fat, strength)` (0x5a5661); `StoreBoneArray(mi->clump, i)` (0x5a5677); `CStreaming::RemoveModel(m)` (0x5a567d).
- gta-reversed differs: writes `*out` without the index (all results to out[0]; bug in gr), calls RequestGeometry for
  model i (which additionally sets the flag and VERIFYs FindItem). Port follows the bytes.
- librw: absent (game code, not part of RenderWare).

## 0x5a6040 CClothesBuilder::ConstructTextures  (catalogue: specified/REA, port level 17)
- Convention: `void (RwTexDictionary* dict, u32* tex /*CPedClothesDesc::m_anTextureKeys*/, f32 fA, f32 fB, f32 fC)`
  (base frame [0x24] dict, [0x28] tex, [0x2c] fA, [0x30] fB, [0x34] fC; the `tex` slot is later reused as a txd id).
  GTFT = `GetTextureFromTxdAndLoadNextTxd(RwTexture* dst, i32 txd, u32 nextKey, i32* nextTxd)` 0x5a5f70; GetTxd(id) is
  inlined as `flags[id] < 0 ? NULL : objects[id]` followed by an unconditional `->m_pRwDictionary` load (a free slot
  crashes: 0x5a60fc/0x5a6108, 0x5a61fc, 0x5a6305, 0x5a64d4).
- Block A tattoos (0x5a6040..0x5a60a4): `txd = RequestTexture(tex[4])`, `LoadRequestedModels()`; `tat = NULL`; for `k = 4 ..
  12`: `tat = GTFT(tat, txd, k < 12 ? tex[k+1] : GetUppercaseKey("player_torso"), k < 12 ? &n : &torsoTxd)`, `txd = n`
  (0x5a606b..0x5a60a4).
- Block B (0x5a60a6..0x5a60c7): if `fB < 0.0` (exactly C0 set; NaN no): `fA = fA + fB`, `fB = 0`.
- Block C torso (0x5a60cb..0x5a61c5): `LoadAllRequestedModels(1)`; `t0 = RequestTexture(tex[0])`; `LoadRequestedModels()`;
  `d = GetTxd(torsoTxd)`; `a = Find(d, "torso")`, `b = Find(d, "torso_fat")`, `c = Find(d, "torso_ripped")`; `dst =
  CopyTexture(a)` (0x5a613d); if `tat`: `BlendTextures(dst, b, c, fA, fB, fC, 0x6c, tat)` (0x5a5bc0) then
  `RwTextureDestroy(tat)` (0x5a6166); else `BlendTextures(dst, b, c, fA, fB, fC, 0x6c)` (0x5a59c0). `RemoveModel(torsoTxd +
  20000)` (0x5a618b). `GTFT(dst, t0, GetUppercaseKey("player_legs"), &legsTxd)` (0x5a61ad; places tex[0]'s texture on
  top); `RwTextureSetName(dst, "torso")`, `RwTexDictionaryAddTexture(dict, dst)` (0x5a61ba/0x5a61c5).
- Block D legs (0x5a61ca..0x5a62b4): `LoadAllRequestedModels(1)`; `t2 = RequestTexture(tex[2])`; `LoadRequestedModels()`;
  `d = GetTxd(legsTxd)`; `a = Find("legs")`, `b = Find("legs_fat")`, `c = Find("legs_ripped")`; `dst = CopyTexture(a)`;
  `BlendTextures(dst, b, c, fA, fB, fC, 0x6c)` (0x5a6258); `RemoveModel(legsTxd + 20000)`; `GTFT(dst, t2, tex[1] != 0 ?
  tex[1] : GetUppercaseKey("player_face"), &faceTxd)` (0x5a6299); name "legs", add to dict.
- Block E head (0x5a62b9..0x5a63b0): `LoadAllRequestedModels(1)`; `feetTxd = RequestTexture(tex[3] != 0 ? tex[3] :
  GetUppercaseKey("player_feet"))` (0x5a62cb/0x5a62e1); `LoadRequestedModels()`; `d = GetTxd(faceTxd)`; `a = Find("face")`,
  `b = Find("face_fat")`. If `a == NULL`: `dst = CopyTexture(GetFirstTexture(d))` (0x5a637d). Else `dst = CopyTexture(a)`;
  if `b`: `s = fA + fB + fC` (x87 `fld fA; fadd fB; fadd fC`, 0x5a6346..0x5a6353); `BlendTextures(dst, b, f32((fA + fC) /
  s), f32(fB / s), 0x6c)` (0x5a5820; 0x5a6357..0x5a6372). `RemoveModel(faceTxd + 20000)`; name "head", add.
- Block F accessories (0x5a63b5..0x5a64af): `x = GTFT(NULL, feetTxd, tex[13], &n1)` -> name "feet", add (no NULL test);
  `x = GTFT(NULL, n1, tex[14], &n2)` -> if x: "necklace"; `GTFT(NULL, n2, tex[15], &n1)` -> "watch"; `GTFT(NULL, n1,
  tex[16], &n2)` -> "glasses"; `GTFT(NULL, n2, tex[17], &n1)` -> "hat" (each name/add only when non-NULL).
- Block G (0x5a64b2..0x5a6516): if `n1 != -1`: `LoadAllRequestedModels(1)`; `x = CopyTexture(GetFirstTexture(GetTxd(n1)))`;
  `RemoveModel(n1 + 20000)`; if x: name "extra1", add.
- gta-reversed: `plugin::Call` only. librw: not RW code.

## 0x5a6530 CClothesBuilder::ConstructGeometryAndSkinArrays  (catalogue: specified/REA, port level 8)
- Convention: `void (RpHAnimHierarchy* h, RpGeometry** outGeom, RwMatrixWeights** outWeights, u32** outIndices, u32 n,
  RpGeometry** geoms, RpMaterial** mats)` (base frame [0x98] .. [0xb0]).
- 1. `nv = sum geoms[i]->numVertices`, `nt = sum geoms[i]->numTriangles` over non-NULL `geoms[i]`, `i < n` unsigned
  (0x5a6560..0x5a6577). `G = RpGeometryCreate(nv, nt, 0x35)` (TRISTRIP|TEXTURED|NORMALS|LIGHT, 0x5a6581; no NULL check);
  `*outGeom = G`.
- 2. `dv = G->morphTarget->verts`, `dn = ...->normals`, `du = G->texCoords[0]`, `dt = G->triangles`, `base = 0`. For each
  non-NULL `g = geoms[i]` (do-while over `n`, 0x5a65e6..0x5a6711): copy `g->numVertices` vertices, normals (3 dwords each)
  and uvs (2 dwords) (0x5a6611..0x5a667f); then for each of `g->numTriangles` triangles (signed `jle` 0x5a6696):
  `RpGeometryTriangleSetVertexIndices(G, dt, u16(v0 + base), u16(v1 + base), u16(v2 + base))` (0x5a66bf),
  `RpGeometryTriangleSetMaterial(G, dt, mats[i])` (0x5a66d2); `dt += 8`; finally `base += g->numVertices` (0x5a66f7).
- 3. `RpMorphTargetCalcBoundingSphere(mt, &sph)` (0x5a671d), `mt->boundingSphere = sph` (0x5a6726..0x5a673e);
  `RpGeometryUnlock(G)` (0x5a6741).
- 4. `*outWeights = operator new(nv*16)` (0x5a6750), `*outIndices = operator new(nv*4)` (0x5a6766) (no NULL checks). For
  each non-NULL `geoms[i]` (`i < n` unsigned): `skin = RpSkinGeometryGetSkin(g)`; `BuildBoneIndexConversionTable(tbl, h, i)`
  (0x5a67c3, `u8 tbl[]` at [esp+0x58]); for `v < g->numVertices` (signed): `idx = RpSkinGetVertexBoneIndices(skin)`
  (called per vertex); out index bytes `= tbl[idx[v].byte_j]` (j = 0..3, 0x5a67dd..0x5a6809); out weights = the 16 bytes
  of `RpSkinGetVertexBoneWeights(skin)[v]` (0x5a680c..0x5a682a).
- gta-reversed: `plugin::Call` only. librw: not RW code.

