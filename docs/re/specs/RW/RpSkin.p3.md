# RenderWare 3.6 RpSkin (plugin 0x116) - pass-3 specs (software skinning, stream, split data), writer A (RE-SPEC-RW-3)

Statically linked RW 3.6 plugin code in gta_sa.exe 1.0 US; gta-reversed has no code for any of these (only
`plugin::Call*` thunks for the public API), so everything comes from the bytes. All functions are `__cdecl` (caller
pops) unless stated. This file extends docs/re/specs/RW/RpSkin.md (pass 1/2, read-only); its layout tables are valid
here and only the facts used below are repeated with their proof.

## Layouts used (offsets proven by the instruction addresses given)

Globals:
| global | type / meaning | proof |
|---|---|---|
| `[0xc978ac]` | `u8*` 16-byte aligned bone-matrix cache, 0x40 bytes per bone | read 0x7ca8a5, 0x7caa80, 0x7cb3b2 |
| `[0xc978d4]` | `RpHAnimHierarchy*` cache key 1 (last hierarchy) | read 0x7ca850, written 0x7ca9fd, 0x7caaa8 |
| `[0xc978d8]` | `u32` cache key 2 = `u16 G+0x8` zero-extended (G = `[0xc97b24]` RwEngineInstance) | 0x7ca86f/0x7ca879, written 0x7caa08 |
| `[0xc978dc]` | `RwFrame*` cache key 3 = atomic frame (`atomic+0x4`) | 0x7ca884, written 0x7caa11 |
| `[0xc978e0]` | `i32` hardware T&L caps flag | 0x7c85fa |
| `[0xc980a4]` | `i32` = `_rwIntelSSEsupported()` (written 0x7f647c) - selects the SSE helper set in the caller 0x7c7e37 | 0x7f647c |
| `[0x882de4] = 0x3f800000` (.rdata) | 1.0f bit pattern, compared as an unsigned dword | 0x7cac7d, 0x7caf6d |
| `[0x882de8] = 0.0f` (.rdata) | 0.0f, `ucomiss` operand | 0x7cade6, 0x7cae3a |

`RpSkin` (see RpSkin.md): +0x4 numUsedBones (0x7c7eb9), +0x8 usedBones (0x7c7ece), +0x10 maxNumWeightsForVertex
(0x7c7e54), +0x14 vertexBoneIndices (u32 per vertex, byte k = bone of weight k, 0x7c7e77), +0x18 vertexBoneWeights
(4 x f32 per vertex, 0x7c7e6f).

`RpHAnimHierarchy` (see RpHAnim.md): +0x0 flags (0x2 NOMATRICES tested 0x7ca8ba; 0x4000 LOCALSPACEMATRICES tested
0x7ca93a as `test ah, 0x40`), +0x4 numNodes (0x7ca8d9), +0x8 pMatrixArray (0x40-byte RwMatrix per node, 0x7ca95e),
+0x10 pNodeInfo (0x10 bytes per node, +0xc RwFrame* frame, 0x7ca8f5/0x7ca8f8).

Bone-matrix cache formats (two different formats share `[0xc978ac]`):
- x87 format (built by 0x7ca850 via 0x7ca6c0): per bone a 3x4 row-major matrix `T` in the first 0x30 bytes of the
  0x40 slot; `T[4r+c]` for c<3 is column c's row r ("transposed RwMatrix": `T[0..2] = (right.x, up.x, at.x)`,
  `T[3] = pos.x`, `T[4..7]` the y row, `T[8..11]` the z row); bytes 0x30..0x3f are never written by 0x7ca6c0.
- SSE format (built by 0x7caa30 via 0x7cb390): a plain RwMatrix (right +0, up +0x10, at +0x20, pos +0x30) whose four
  dwords +0xc, +0x1c, +0x2c, +0x3c are then zeroed (0x7caa89..0x7caa91).

Notation for float order: `a + b + c` means `((a + b) + c)`; products are written `M[k]*s.x` with the operand order
of the bytes (multiplication is commutative, so only the association of the sums matters). x87 code runs at the
game's 24-bit precision control; SSE code is plain f32 (`mulps`/`addps`, MXCSR rounding) and does not round like
the x87 path - a port must keep the two paths separate.

Software-skin helper argument order (from the caller _rpD3D9SkinAllInOneNode, pushes 0x7c7f2c..0x7c7f48):
`(i32 numVertices, [RwMatrixWeights* weights], u32* indices, u8* mats, RwV3d* dstPos, RwV3d* srcPos, RwV3d* dstNrm,
RwV3d* srcNrm, u32 stride)`; the 1-weight variants drop `weights`, the single-matrix variants (0x7ca330, 0x7caad0)
take `mats + usedBones[0]*64` instead of `(indices, mats)`. `dstPos`/`dstNrm` advance by `stride`, `srcPos`/`srcNrm`
by 0xc. `dstNrm == NULL` exactly when `srcNrm == NULL` (caller 0x7c7e1c..0x7c7e35).

## 0x7c85b0 sub_7c85b0 (proposed: _rpD3D9SkinRenderSoftware)  (catalogue: unknown/REA, port level 4)
- Convention: `void (RxD3D9AllInOneCallbacks* cb, RwResEntry* res, RpAtomic* atomic, u32 geometryFlags)`; one caller
  `_rpD3D9SkinAllInOneNode` 0x7c8049 passes the node private data as `cb` (+0x0 render callback, +0x4 lighting
  callback). `ret` (cdecl).
- Behaviour:
  1. If `cb->lighting (+0x4) != NULL` call it with `(atomic)` (0x7c85ba..0x7c85c6).
  2. `ltm = RwFrameGetLTM(atomic->frame (+0x4))` (0x7c85cf); `RwD3D9SetTransformWorld(ltm)` (0x7c85d7).
  3. `RwD3D9GetRenderState(0x89 D3DRS_LIGHTING, &lit)` (0x7c85e6). `normalize = 0` if `lit == 0` (0x7c85f4).
     Else `normalize = 1` if `[0xc978e0] != 0` (0x7c85fa..0x7c8605). Else if `ltm->flags (+0xc) & 0x20001`
     (0x7c860a) -> `normalize = 0`. Else for each of right (+0), up (+0x10), at (+0x20):
     `len2 = x*x + y*y + z*z` (x87, 0x7c8617..0x7c8641: `x*x` and `y*y` are summed first, then `z*z`), stored as f32
     (0x7c8643) and compared as a signed dword: `bits > 0x3f8ccccd` (1.1f, 0x7c8627/0x7c8653 `jg`) or
     `bits < 0x3f666666` (0.9f, 0x7c862f/0x7c865b `jl`) -> `normalize = 1` (first failing axis stops). If all three are
     inside -> 0. (A NaN length has bits > 1.1f -> 1; a negative cannot occur.)
  4. `RwD3D9SetRenderState(0x8f D3DRS_NORMALIZENORMALS, normalize)` (0x7c86c6).
  5. If `cb->render (+0x0) != NULL`: `render(res, atomic, 1 /*rpATOMIC*/, geometryFlags)` (0x7c86d4..0x7c86e1).
- librw: d3d/d3d9skin.cpp:skinRenderCB differs: librw has no software-skin render path and no normalize test
  (the logic matches RW's `_rwD3D9AtomicDefaultRenderCallback` normalize heuristic instead).

## 0x7c9da0 sub_7c9da0 (proposed: _rpSkinBlendPositionsNormals4Weights, catalogue hint _rpSkinBlendNormalsGeneric)  (catalogue: unknown/REA, port level 0)
- Convention: `void (i32 n, RwMatrixWeights* w, u32* idx, u8* mats /*x87 format*/, RwV3d* dstPos, RwV3d* srcPos,
  RwV3d* dstNrm, RwV3d* srcNrm, u32 stride)`; caller 0x7c7f49 when `skin->maxNumWeights > 2` and SSE is off.
- Loop: `for (v = 0; v < n; v++)` with an entry test `n <= 0 -> return` (0x7c9da9/0x7c9daf, `jle`), counter compare
  0x7ca220 `jl`. Per vertex `i = idx[v]` (0x7c9dd4), `M0 = mats + (i & 0xff)*64` (0x7c9ddd..0x7c9de6), weights
  pointer advances 0x10 per vertex (0x7ca21d).
- One-bone fast path when the dword of `w0` compares signed `>= 0x3f800000` (0x7c9de8, `jl` to the blend path):
  `dst.x = M0[0]*s.x + s.z*M0[2] + s.y*M0[1] + M0[3]` (0x7c9def..0x7c9e03), `dst.y = M0[4]*s.x + s.z*M0[6] +
  s.y*M0[5] + M0[7]` (0x7c9e19..0x7c9e2f), `dst.z = M0[9]*s.y + s.x*M0[8] + s.z*M0[10] + M0[11]` (0x7c9e35..0x7c9e4b);
  if `dstNrm`: `n'.x = n.y*M0[1] + M0[0]*n.x + n.z*M0[2]`, `n'.y = n.z*M0[6] + n.y*M0[5] + M0[4]*n.x`,
  `n'.z = n.y*M0[9] + n.x*M0[8] + n.z*M0[10]` (0x7c9e5f..0x7c9ea3, no renormalisation).
- Blend path (w0 < 1.0 or negative): with `P_x(M) = M[0]*s.x + s.z*M[2] + s.y*M[1] + M[3]`, `P_y(M) = M[4]*s.x +
  s.z*M[6] + s.y*M[5] + M[7]`, `P_z(M) = M[9]*s.y + s.x*M[8] + s.z*M[10] + M[11]` and the normal rows
  `N_x(M) = n.y*M[1] + M[0]*n.x + n.z*M[2]`, `N_y(M) = n.z*M[6] + n.y*M[5] + M[4]*n.x`, `N_z(M) = n.y*M[9] +
  n.x*M[8] + n.z*M[10]`:
  1. bone 0: `ax = P_x(M0)*w0` (kept on the x87 stack), `ay = P_y(M0)*w0` (stack), `az = f32(P_z(M0)*w0)`
     (0x7c9ead..0x7c9ee8); if `srcNrm`: `nx = f32(N_x(M0)*w0)`, `ny`, `nz` likewise (0x7c9eee..0x7c9f3d).
  2. bone 1 `M1 = mats + ((i >> 8) & 0xff)*64` (0x7c9f41..0x7c9f48), unconditional: `px = f32(P_x(M1)*w1 + ax)`,
     `py = f32(P_y(M1)*w1 + ay)`, `pz = f32(P_z(M1)*w1 + az)` (0x7c9f4c..0x7c9fb0); normals `nk = f32(N_k(M1)*w1 + nk)`
     if `srcNrm` (0x7c9fb6..0x7ca011).
  3. bone 2 only if the dword of `w2` compares signed `> 0` (0x7ca015 `cmp [w+8], 0; jle`): `M2 = mats + byte2(i)*64`
     (0x7ca020 reads the byte at the saved index +2), `pk = f32(P_k(M2)*w2 + pk)`, normals likewise (0x7ca02b..0x7ca0ea).
  4. bone 3 only after bone 2 and if dword `w3 > 0` signed (0x7ca0ee..0x7ca0f3): `M3 = mats + (i >> 24)*64`
     (0x7ca0f9..0x7ca0ff), same accumulation (0x7ca105..0x7ca1ca).
  5. Store `px, py, pz` to `dstPos` (dword copies 0x7ca1db..0x7ca1e8), `dstPos += stride`; if `dstNrm` store the three
     normal temps and `dstNrm += stride` (0x7ca1f9..0x7ca20e); `srcNrm += 0xc` (0x7ca211), `srcPos += 0xc` (0x7ca1d8).
- Edge cases: weights are tested as integers (a weight of -0.0 = 0x80000000 is `<= 0` -> skipped; NaN > 0 bits ->
  used). No normalisation of the blended normal. When `srcNrm == NULL` the normal temps are not written but the
  pointer handling still skips (0x7c9e53 / 0x7ca1f1).
- librw: skin.cpp:skinVertices / d3d9skin software path absent; librw skins with a loop over 4 weights using `> 0.0f`
  float tests and RwMatrix layout (differs in layout, tests and float order).

## 0x7ca240 sub_7ca240 (proposed: _rpSkinBlendPositions1Weight)  (catalogue: unknown/REA, port level 0)
- Convention: `void (i32 n, u32* idx, u8* mats /*x87 format*/, RwV3d* dstPos, RwV3d* srcPos, RwV3d* dstNrm,
  RwV3d* srcNrm, u32 stride)`; caller 0x7c7fbb (`maxNumWeights <= 1` and `numUsedBones != 1`, SSE off).
- Loop: do-while on `n` (0x7ca315..0x7ca31e: `dec; jne`), NO entry test: `n == 0` would run 2^32 times (the caller
  only reaches here with `numVertices > 0`, checked in the node at 0x7c7bb4).
- Per vertex: `M = mats + (idx[v] & 0xff)*64` (0x7ca258..0x7ca26c), idx advances 4 (0x7ca266).
  `dst.x = s.z*M[2] + s.y*M[1] + s.x*M[0] + M[3]` (0x7ca25e..0x7ca28a), `dst.y = s.y*M[5] + s.x*M[4] + s.z*M[6] +
  M[7]` (0x7ca28c..0x7ca2a5), `dst.z = s.z*M[10] + M[9]*s.y + s.x*M[8] + M[11]` (0x7ca2a8..0x7ca2c1); `dstPos += stride`.
  If `dstNrm != NULL` (0x7ca2c6): `n'.x = n.y*M[1] + M[0]*n.x + M[2]*n.z`, `n'.y = n.y*M[5] + n.x*M[4] + n.z*M[6]`,
  `n'.z = M[8]*n.x + M[10]*n.z + M[9]*n.y` (0x7ca2ca..0x7ca30d); `dstNrm += stride`, `srcNrm += 0xc`.
- librw: absent (no software skin path with this matrix format).

## 0x7ca330 sub_7ca330 (proposed: _rpSkinBlendPositionsRigid)  (catalogue: unknown/REA, port level 0)
- Convention: `void (i32 n, u8* M /*one x87-format matrix*/, RwV3d* dstPos, RwV3d* srcPos, RwV3d* dstNrm,
  RwV3d* srcNrm, u32 stride)`; caller 0x7c7fa8 with `M = mats + usedBones[0]*64` (`maxNumWeights <= 1` and
  `numUsedBones == 1`).
- Loop: do-while on `n` (0x7ca3f5 `dec ebp; jne`), no entry test (same caveat as 0x7ca240).
- `dst.x = M[0]*s.x + s.z*M[2] + s.y*M[1] + M[3]` (0x7ca350..0x7ca367), `dst.y = M[4]*s.x + M[6]*s.z + s.y*M[5] +
  M[7]` (0x7ca36c..0x7ca382), `dst.z = M[9]*s.y + M[8]*s.x + s.z*M[10] + M[11]` (0x7ca388..0x7ca39e).
  If `dstNrm` (0x7ca3a6): `n'.x = n.y*M[1] + M[0]*n.x + M[2]*n.z`, `n'.y = M[6]*n.z + n.y*M[5] + M[4]*n.x`,
  `n'.z = M[9]*n.y + M[8]*n.x + M[10]*n.z` (0x7ca3aa..0x7ca3ed).
- librw: absent.

## 0x7ca410 sub_7ca410 (proposed: _rpSkinBlendPositionsNormals2Weights)  (catalogue: unknown/REA, port level 0)
- Convention: as 0x7c9da0 (9 args, with weights); caller 0x7c7f75 (`maxNumWeights == 2`, SSE off).
- Loop: `for (v = 0; v < n; v++)`, entry test `n <= 0 -> return` (0x7ca419/0x7ca41f), compare 0x7ca69d `jl`;
  weights advance 0x10 (0x7ca69a). `M0 = mats + (i & 0xff)*64`, `M1 = mats + ((i >> 8) & 0xff)*64` (0x7ca441..0x7ca46d).
- `t = M0[0]*s.x + s.z*M0[2] + s.y*M0[1] + M0[3]` is computed first (0x7ca458..0x7ca47a), then the dword of `w0` is
  compared signed with 0x3f800000 (0x7ca472, `jl` -> blend path).
- One-bone path: `dst.x = t`; `dst.y = M0[4]*s.x + s.z*M0[6] + s.y*M0[5] + M0[7]`; `dst.z = M0[9]*s.y + s.x*M0[8] +
  s.z*M0[10] + M0[11]` (0x7ca483..0x7ca4c4). If `dstNrm` (0x7ca4c9): `n'.x = M0[2]*n.z + M0[0]*n.x + n.y*M0[1]`,
  `n'.y = n.z*M0[6] + M0[4]*n.x + n.y*M0[5]`, `n'.z = M0[10]*n.z + M0[9]*n.y + n.x*M0[8]` (0x7ca4d1..0x7ca514).
- Blend path: `ax = t*w0` (stack); `ay = (M0[4]*s.x + s.z*M0[6] + s.y*M0[5] + M0[7])*w0` (stack);
  `az = f32((M0[9]*s.y + s.x*M0[8] + s.z*M0[10] + M0[11])*w0)` (0x7ca522..0x7ca55d). If `srcNrm` (0x7ca530 tests the
  source pointer): `nx0 = f32((M0[2]*n.z + M0[0]*n.x + n.y*M0[1])*w0)`, `ny0 = f32((n.z*M0[6] + M0[4]*n.x +
  n.y*M0[5])*w0)`, `nz0 = f32((M0[10]*n.z + M0[9]*n.y + n.x*M0[8])*w0)` (0x7ca563..0x7ca5b2).
  `dst.x = (M1[1]*s.y + M1[0]*s.x + M1[2]*s.z + M1[3])*w1 + ax` (0x7ca5b6..0x7ca5d9), `dst.y = (M1[5]*s.y +
  M1[6]*s.z + s.x*M1[4] + M1[7])*w1 + ay` (0x7ca5db..0x7ca5f9), `dst.z = (M1[8]*s.x + s.z*M1[10] + s.y*M1[9] +
  M1[11])*w1 + az` (0x7ca600..0x7ca620). If `srcNrm` (0x7ca625): `n'.x = (M1[1]*n.y + M1[0]*n.x + M1[2]*n.z)*w1 + nx0`,
  `n'.y = (M1[5]*n.y + M1[4]*n.x + M1[6]*n.z)*w1 + ny0`, `n'.z = (n.z*M1[10] + M1[8]*n.x + n.y*M1[9])*w1 + nz0`
  (0x7ca629..0x7ca685).
- Edge: `w1` is used even when 0; no weight sum check. Signed compare: negative `w0` takes the blend path (contrast
  the SSE twin 0x7caf50, which compares unsigned).
- librw: absent.

## 0x7ca6c0 sub_7ca6c0 (proposed: _rpSkinMatrixMultiplyTransposed)  (catalogue: unknown/REA, port level 0)
- Convention: `void (f32* T /*out, 12 floats*/, const RwMatrix* a, const RwMatrix* b)`; callers only 0x7ca850
  (0x7ca91f, 0x7ca968, 0x7ca9d8). Leaf, x87.
- Computes `c = a * b` (RW row-vector convention) and writes it transposed as 3x4 rows; with `a[k]`, `b[k]` the
  floats of the matrices (`[0..2]` right, `[4..6]` up, `[8..10]` at, `[12..14]` pos):
  `T[0] = a[0]*b[0] + a[2]*b[8] + b[4]*a[1]` (0x7ca6cc..0x7ca6e0), `T[4] = b[9]*a[2] + b[5]*a[1] + a[0]*b[1]`
  (0x7ca6e2..0x7ca6f7), `T[8] = a[2]*b[10] + b[6]*a[1] + b[2]*a[0]` (0x7ca6fa..0x7ca70f),
  `T[1] = a[6]*b[8] + a[4]*b[0] + b[4]*a[5]` (0x7ca712..0x7ca727), `T[5] = b[5]*a[5] + a[4]*b[1] + a[6]*b[9]`
  (0x7ca72a..0x7ca740), `T[9] = a[6]*b[10] + b[2]*a[4] + b[6]*a[5]` (0x7ca743..0x7ca759),
  `T[2] = a[10]*b[8] + a[8]*b[0] + a[9]*b[4]` (0x7ca75c..0x7ca771), `T[6] = a[9]*b[5] + a[8]*b[1] + a[10]*b[9]`
  (0x7ca774..0x7ca78a), `T[10] = a[10]*b[10] + a[9]*b[6] + b[2]*a[8]` (0x7ca78d..0x7ca7a3),
  `T[3] = a[14]*b[8] + b[0]*a[12] + a[13]*b[4] + b[12]` (0x7ca7a6..0x7ca7be), `T[7] = a[14]*b[9] + a[13]*b[5] +
  a[12]*b[1] + b[13]` (0x7ca7c1..0x7ca7da), `T[11] = b[2]*a[12] + a[14]*b[10] + a[13]*b[6] + b[14]` (0x7ca7dd..0x7ca7f6).
  `T[12..15]` untouched. `a`/`b` flags are ignored.
- librw: absent.

## 0x7ca850 sub_7ca850 (proposed: _rpSkinPrepareAtomicMatrix, x87 format)  (catalogue: unknown/REA, port level 4)
- Convention: `u8* (RpAtomic* atomic, RpSkin* skin, RpHAnimHierarchy* h)`; returns `[0xc978ac]` always (0x7caa17).
  Caller 0x7c7f1b. Locals: `inv` RwMatrix at [esp+0x18..], `tmp` RwMatrix at [esp+0x58..] (0x84-byte frame).
- Cache test (0x7ca850..0x7ca88d): if `[0xc978d4] == h` and `[0xc978d8] == (u32)(u16)G+0x8` and `[0xc978dc] ==
  atomic->frame` -> return cache without work.
- If `h == NULL` (0x7ca895) skip the build. Else `out = [0xc978ac]`, `stb = RpSkinGetSkinToBoneMatrices(skin)`
  (0x7ca8ac):
  - `h->flags & 2` (NOMATRICES, 0x7ca8ba): `RwMatrixInvert(&inv, RwFrameGetLTM(atomic->frame))` (0x7ca8c9/0x7ca8d4);
    for `i = 0 .. h->numNodes-1` (signed, `jle` entry test 0x7ca8e3, `jl` 0x7ca933): `RwMatrixMultiply(&tmp,
    &stb[i], RwFrameGetLTM(h->pNodeInfo[i].frame))` (0x7ca8fd/0x7ca90f), `0x7ca6c0(out + i*0x40, &tmp, &inv)` (0x7ca91f).
  - else `h->flags & 0x4000` (0x7ca93a): for each node `0x7ca6c0(out + i*0x40, &stb[i], &h->pMatrixArray[i])` (0x7ca968).
  - else: `RwMatrixInvert(&inv, LTM(atomic->frame))` (0x7ca996); per node `RwMatrixMultiply(&tmp, &stb[i],
    &h->pMatrixArray[i])` (0x7ca9c8), `0x7ca6c0(out + i*0x40, &tmp, &inv)` (0x7ca9d8).
- Then (also when `h == NULL`) store the keys: `[0xc978d4] = h` (0x7ca9fd), `[0xc978d8] = u16 G+0x8` (0x7caa08),
  `[0xc978dc] = atomic->frame` (0x7caa11).
- Edge: the numNodes loop does not check against `skin->numBones`; the cache holds `numNodes` matrices. With `h ==
  NULL` the cache contents are stale but the keys are updated.
- librw: skin.cpp:skinMatrices? differs: librw builds RwMatrix-format palettes (not transposed) and has no cache keys.

## 0x7caa30 sub_7caa30 (proposed: _rpSkinPrepareAtomicMatrixSSE)  (catalogue: unknown/REA, port level 5)
- Convention: `u8* (RpAtomic* atomic, RpSkin* skin, RpHAnimHierarchy* h)`; returns `[0xc978ac]` (0x7caac0). Caller
  0x7c7e4f (SSE path).
- Same cache test as 0x7ca850 (0x7caa30..0x7caa64). On a miss: `0x7cb390(atomic, skin, h)` (0x7caa6d); then for
  `i = 0 .. h->numNodes-1` (signed, entry test 0x7caa7e `jle`) zero the dwords at `cache + i*0x40 + 0xc, +0x1c, +0x2c,
  +0x3c` (0x7caa86..0x7caa91); then store the three keys (0x7caaa8..0x7caabb).
- Bug kept: `h->numNodes` is read (0x7caa72) after 0x7cb390 even when `h == NULL` -> NULL dereference; the caller
  only enters with `h != NULL` (node test 0x7c7cdd).
- librw: absent.

## 0x7caad0 sub_7caad0 (proposed: _rpSkinBlendRigidSSE)  (catalogue: unknown/REA, port level 0)
- Convention: `void (i32 n, const RwMatrix* M /*16-byte aligned, SSE format*/, RwV3d* dstPos, RwV3d* srcPos,
  RwV3d* dstNrm, RwV3d* srcNrm, u32 stride)` (ebp frame: n +8, M +0xc, dstPos +0x10, srcPos +0x14, dstNrm +0x18,
  srcNrm +0x1c, stride +0x20; 0x7caad6..0x7caaf7). Caller 0x7c7ee0.
- `movaps` loads right/up/at/pos once (0x7caae5..0x7caaf0, faults if `M` is not 16-byte aligned). Do-while on `n`
  (0x7cab73), no entry test.
- Per vertex (SSE f32 lanes): `p = (s.z*at + pos) + (s.y*up + s.x*right)` (0x7cab14..0x7cab26: `xmm1 = y*up + x*right`,
  `xmm2 = z*at + pos`, `xmm2 += xmm1`), lanes 0,1 stored with `movlps` and lane 2 with `movss` (0x7cab2c/0x7cab2f).
  If `srcNrm != NULL` (0x7cab36): `n' = n.z*at + (n.y*up + n.x*right)` (0x7cab54..0x7cab63), stored the same way.
- librw: absent.

## 0x7cab80 sub_7cab80 (proposed: _rpSkinBlend1WeightSSE)  (catalogue: unknown/REA, port level 0)
- Convention: `void (i32 n, u32* idx, u8* mats /*SSE format*/, RwV3d* dstPos, RwV3d* srcPos, RwV3d* dstNrm,
  RwV3d* srcNrm, u32 stride)` (ebp +8 .. +0x24; 0x7cab86..0x7cab8f). Caller 0x7c7f06.
- Do-while on `n` (0x7cac50). Per vertex `M = mats + (idx[v] & 0xff)*64` (byte load 0x7cabb1, 0x7cabb9..0x7cabbf), the
  same position formula as 0x7caad0 (0x7cabd1..0x7cabe3), stored lane by lane with `movss` (0x7cabe9..0x7cabf9). If
  `srcNrm` (0x7cac03) the normal formula of 0x7caad0 (0x7cac21..0x7cac30).
- librw: absent.

## 0x7cac60 sub_7cac60 (proposed: _rpSkinBlend4WeightsSSE)  (catalogue: unknown/REA, port level 0)
- Convention: `void (i32 n, RwMatrixWeights* w, u32* idx, u8* mats, RwV3d* dstPos, RwV3d* srcPos, RwV3d* dstNrm,
  RwV3d* srcNrm, u32 stride)` (`ebx` = entry esp: n +8, w +0xc, idx +0x10, mats +0x14, dstPos +0x18, srcPos +0x1c,
  dstNrm +0x20, srcNrm +0x24, stride +0x28). Stack realigned to 16 (0x7cac64) for the `[ebp-0x10]` broadcast temp.
  Caller 0x7c7e7d (`maxNumWeights > 2`, SSE on).
- Do-while on `n` (0x7caf32 `dec; jne`), no entry test; `w += 0x10`, `idx += 4` per vertex (0x7caf2c/0x7caf2f).
- If the dword of `w0` compares UNSIGNED `>= [0x882de4] = 0x3f800000` (0x7cac7d `jb` to the blend path): rigid
  transform with `M0 = mats + (i & 0xff)*64` exactly as 0x7cab80 (0x7cac89..0x7cad4b). Note: any negative `w0`
  (sign bit set) also takes this path.
- Blend: `A = M0*w0` (each of the 4 rows, 0x7cad60..0x7cad96); `A = M1*w1 + A` with `M1 = mats + ((i & 0xff00) >> 2)`
  (0x7cad9a..0x7cadde, `xmm4 = M1*w1; xmm4 += xmm0`); if `w2 > [0x882de8] = 0.0` (`ucomiss`, `jbe` skips; NaN skips,
  0x7cade6): `A = A + M2*w2`, `M2 = mats + ((i & 0xff0000) >> 10)` (0x7cadf3..0x7cae32); then (only inside the w2
  branch) if `w3 > 0.0` (0x7cae3a): `A = A + M3*w3`, `M3 = mats + ((i & 0xff000000) >> 18)` (0x7cae43..0x7cae82).
  Position `p = (s.z*A.at + A.pos) + (s.y*A.up + s.x*A.right)` (0x7cae85..0x7caeb7), lane stores; normal if `srcNrm`
  (0x7caedb): `n' = n.z*A.at + (n.y*A.up + n.x*A.right)` (0x7caef9..0x7caf0b).
- Differences to the x87 twin 0x7c9da0: blends matrices then transforms once (x87 transforms per bone), float `> 0.0`
  tests instead of signed-integer tests, unsigned `w0 >= 1.0` test.
- librw: absent.

## 0x7caf50 sub_7caf50 (proposed: _rpSkinBlend2WeightsSSE)  (catalogue: unknown/REA, port level 0)
- Convention and frame identical to 0x7cac60; caller 0x7c7eac (`maxNumWeights == 2`, SSE on).
- Identical to 0x7cac60 except the blend uses only bones 0 and 1 (0x7cb050..0x7cb0cb: `A = M1*w1 + M0*w0`), no w2/w3
  terms; the `w0 >= 1.0` test is the same unsigned compare against `[0x882de4]` (0x7caf6d). Note this blend path
  loads `M1` with `eax = ((i & 0xff00) >> 2) + mats` (0x7cb095..0x7cb0a1).
- librw: absent.

## 0x7cb390 sub_7cb390 (proposed: _rpSkinBuildMatricesSSE)  (catalogue: unknown/REA, port level 4)
- Convention: `u8* (RpAtomic* atomic, RpSkin* skin, RpHAnimHierarchy* h)`; returns 0 when `h == NULL` (0x7cb396/0x7cb3a5),
  else `[0xc978ac]` (0x7cb44c, 0x7cb4a3, 0x7cb524, 0x7cb533). Caller 0x7caa6d.
- `out = [0xc978ac]` (0x7cb3b2); `stb = RpSkinGetSkinToBoneMatrices(skin)` (0x7cb3bd). Branches as in 0x7ca850 but with
  RwMatrix-format output:
  - `h->flags & 2` (0x7cb3cb): `RwMatrixInvert(&inv, LTM(atomic->frame))` (0x7cb3e9); per node (signed loop, entry
    `jle` 0x7cb3f8): `RwMatrixMultiply(&tmp, &stb[i], LTM(pNodeInfo[i].frame))` (0x7cb426),
    `RwMatrixMultiply(out + i*0x40, &tmp, &inv)` (0x7cb436).
  - `h->flags & 0x4000` (0x7cb45b): `RwMatrixMultiply(out + i*0x40, &stb[i], &h->pMatrixArray[i])` (0x7cb48d).
  - else `RwMatrixInvert(&inv, LTM(atomic->frame))` (0x7cb4c8); `RwMatrixMultiply(&tmp, &stb[i], &pMatrixArray[i])`
    (0x7cb4fe), `RwMatrixMultiply(out + i*0x40, &tmp, &inv)` (0x7cb50e).
- librw: skin.cpp (skinMatrices-style palette build) matches in structure (skinToBone * bone * inverse(atomic LTM))
  but has no LOCALSPACE 0x4000 shortcut variant in the same form; exe wins.
## 0x7c69a0 cb_SKINPLUGIN_RwEngine_Close  (catalogue: specified/REA, port level 0)
- `[0xc978c0] -= 1`; if it reached 0: `_rpSkinPipelinesDestroy()`, `RwFreeListDestroy([0xc978b8])`, `[0xc978b8] <- 0`,
  `rwfree([0xc978b0])` (G+0x138), `[0xc978b0] <- 0` (0x7c69a5..0x7c69dd). Returns instance.
- librw: skin.cpp has no engine close (absent).

## 0x7c6be0 cb_SKINPLUGIN_RpGeometry_StreamGetSize  (catalogue: specified/REA, port level 0)
- `skin = *(g + [0xc978a8])`; NULL -> 0. NATIVE -> `0x7c8830(g)` = 0 (0x7c6c22). Else `numBones*64 + numVertices*20 + numUsedBones + 4
  + 0x7c8d40(skin)` (0x7c6bfc..0x7c6c1b).
- librw: skin.cpp:getSizeSkin matches for the non-native layout.

## 0x7c6c30 cb_SKINPLUGIN_RpGeometry_StreamWrite  (catalogue: specified/REA, port level 1)
- `(s, len, g, off, size)`: skin NULL -> s. NATIVE -> `0x7c8840(s, g)` ? s : NULL. Else `RwStreamWriteInt32` of `numBones & 0xff |
  numUsedBones(u8) << 8 | maxNumWeights(u8) << 16` (0x7c6c59..0x7c6c7c), raw `usedBones` (numUsedBones bytes), `RwStreamWriteInt32(
  indices, numVertices*4)`, `RwStreamWriteReal(weights, numVertices*16)`, `RwStreamWriteReal(skinToBone, numBones*64)`, then
  `0x7c8b50(s, skin)` (split data). Failures -> NULL; else s.
- librw: skin.cpp:writeSkin matches in layout.

## 0x7c72a0 cb_SKINPLUGIN_RpAtomic_StreamRead  (catalogue: specified/REA, port level 1)
- Legacy (atomic-stored) skin reader `(s, len, a, off, size)`. `g = a->geometry`; if g already has a skin:
  `RwStreamSkip(s, size)` - the `sizeInObject` argument (4), NOT the chunk length (0x7c74b4..0x7c74c4; kept) - return s or NULL.
- Else `numBones = RwStreamReadInt32` (fail -> NULL); `nv = g->numVertices`; `skin = freeListAlloc([0xc978b8], 0x30116)` (no NULL
  check), zeroed. If `numBones == 0`: `_rpSkinFindUsedBones(skin, NULL, NULL, buf, &cnt, nv)` and `used = cnt`; else `used =
  numBones` (0x7c732a..0x7c7341). `_rpSkinAllocateData(skin, numBones, used, nv, buf, NULL, NULL, NULL)` (0x7c7357); failure ->
  free the skin and continue with `skin = NULL` (crashes below; kept). `RwStreamSkip(s, 4)`; `RwStreamReadInt32(s, indices,
  nv*4)`; `RwStreamReadReal(s, weights, nv*16)`; per bone `RwStreamSkip(s, 0xc)` + `RwStreamReadReal(s, &matrices[i], 0x40)`.
  `maxNumWeights`: start 1; for each vertex raise it while the next weight dword is non-zero (cap 4, 0x7c742e..0x7c745f).
  `_rpSkinFindUsedBones(skin, indices, weights, skin->usedBones, &skin->numUsedBones, nv)` (0x7c7470); if the geometry's skin
  differs: `_rpSkinDeinitialize(g)` when one exists, set the extension, `_rpSkinInitialize(g)` (0x7c748d..0x7c749f). Return s.
- librw: absent (librw ignores the old atomic skin chunk).

## 0x7c7510 cb_SKINPLUGIN_RpAtomic_StreamGetSize  (catalogue: specified/REA, port level 0)
- Returns 0 (0x7c7510).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c77a0 RpSkinDestroy  (catalogue: specified/REA, port level 1)
- `rwfree(skin->data)` if set (0x7c77b2), `_rpSkinSplitDataDestroy(skin)`, `freeListFree([0xc978b8], skin)`; returns 0.
- librw: skin.cpp Skin destroy matches in effect.

## 0x7c7820 RpSkinIsSplit  (catalogue: specified/REA, port level 0)
- Returns `skin->numMeshes (+0x28) != 0` (0x7c7826..0x7c782b).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c7880 RpSkinAtomicGetType  (catalogue: specified/REA, port level 0)
- `p = a->pipeline (+0x6c)`; `p->pluginId == 0x116` ? `p->pluginData` : 0 (0x7c7884..0x7c7894; no NULL check of p).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c86f0 _rpSkinPipelinesDestroy  (catalogue: specified/REA, port level 0)
- For the four pipeline slots 0xc978c4..0xc978d0: `_rxPipelineDestroy` and NULL if set (0x7c86f6..0x7c8714); returns 1.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c8830 sub_7c8830 (proposed: _rpSkinGeometryNativeSize)  (catalogue: unknown/REA, port level 0)
- Returns 0.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c8840 sub_7c8840 (proposed: _rpSkinGeometryNativeWrite)  (catalogue: unknown/REA, port level 1)
- `RwStream* (RwStream* s, const RpGeometry* g)`: `Header(s, 1, -0xc /*size field 0xfffffff4*/, 0x36003, 0xffff)` (0x7c8850; kept),
  `RwStreamWriteInt32(&9 /*platform*/, 4)`, `RwStreamWriteInt32(skin, 4)` (writes `numBones`, 0x7c8894); returns ok ? s : NULL.
- librw: absent (librw writes native skin through its own platform code).

## 0x7c88b0 sub_7c88b0 (proposed: _rpSkinGeometryNativeRead)  (catalogue: unknown/REA, port level 1)
- `RwStream* (RwStream* s, RpGeometry* g)`: `RwStreamFindChunk(s, 1, &len, &ver)` fail -> NULL; `ver` outside [0x34000, 0x36003] ->
  `RwErrorSet({0x116, _rwerror(0x80000004)})`, NULL (0x7c88da..0x7c896a); `platform = ReadInt32` must be 9 else NULL; `skin =
  freeListAlloc(0x30116)` zeroed; `RwStreamReadInt32(s, &skin->numBones, 4)` fail -> NULL (skin leaked);
  `RpSkinGeometrySetSkin(g, skin)` (0x7c893f); return s.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c89c0 sub_7c89c0 (proposed: _rpD3D9SkinAtomicCanUseVS)  (catalogue: unknown/REA, port level 0)
- `RwBool (const RpAtomic* a)`: `[0xc978e0] == 0` -> 1; `[0xc978e4] == 0` -> 0; `skin = ext(a->geometry)`; NULL -> 0; return
  `skin->+0x1c <= [0xc978e8]` (unsigned, 0x7c89e6..0x7c89f1).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c8a40 _rpSkinGetMeshBoneRemapIndices  (catalogue: specified/REA, port level 0)
- Returns `skin->+0x30` (0x7c8a44).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c8a50 _rpSkinGetMeshBoneRLECount  (catalogue: specified/REA, port level 0)
- Returns `skin->+0x34` (0x7c8a54).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c8a60 _rpSkinGetMeshBoneRLE  (catalogue: specified/REA, port level 0)
- Returns `skin->+0x38` (0x7c8a64).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c8a70 _rpSkinSplitDataCreate  (catalogue: specified/REA, port level 0)
- `RpSkin* (RpSkin* skin, u32 boneLimit, u32 numRemap, u32 numMeshes, u32 numRLE)`: free an old `+0x30` block; zero +0x24..+0x38;
  `p = rwmalloc(numRemap + (numMeshes + numRLE)*2, 0x30116)` (0x7c8ac0); `skin+0x30 = p`; NULL -> return NULL. Zero it; `+0x24 =
  boneLimit`, `+0x28 = numMeshes`, `+0x2c = numRLE`, `+0x34 = p + numRemap`, `+0x38 = p + numRemap + numMeshes*2`; return skin.
- librw: skin.cpp split data matches in layout.

## 0x7c8b50 sub_7c8b50 (proposed: _rpSkinSplitDataStreamWrite)  (catalogue: unknown/REA, port level 3)
- `RwStream* (RwStream* s, const RpSkin* skin)`: `RwStreamWriteInt32(&boneLimit)`, `(&numMeshes)`, `(&numRLE)`; if `numMeshes`:
  raw `RwStreamWrite(s, skin->+0x30, numBones + (numMeshes + numRLE)*2)` (0x7c8ba4..0x7c8bc1); failures -> NULL; else s.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c8d40 sub_7c8d40 (proposed: _rpSkinSplitDataStreamGetSize)  (catalogue: unknown/REA, port level 0)
- `numMeshes == 0` -> 0xc; else `numBones + (numRLE + numMeshes)*2 + 0xc` (0x7c8d44..0x7c8d57).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cb6a0 sub_7cb6a0 (proposed: _rpSkinBlendGenericReverse, unreferenced)  (catalogue: unknown/REA, port level 1)
- No static callers. ebp frame `(i32 n, RwMatrixWeights* w, u32* idx, u8* mats /*RwMatrix format*/, RwV3d* dstPos, RwV3d* srcPos,
  RwV3d* dstNrm, RwV3d* srcNrm, u32 stride)`. Walks the vertices from the LAST to the first (do-while on n, 0x7cb81c). Per vertex:
  bone k = byte k of `idx[v]`; for k = 0..3 while the weight dword `w[v].k` compares signed `> 0` (0x7cb6e7 etc.): `0x7cb828`
  (position) with ESI = -1 for k = 0 (store) and 0 for k > 0 (accumulate); if `dstNrm != NULL` also `0x7cb8b0` (normal). A vertex
  whose w0 <= 0 keeps its old destination.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cb828 sub_7cb828 (proposed: _rpSkinBlendPointHelper)  (catalogue: unknown/REA, port level 0)
- Register convention: EDX = src RwV3d*, EBX = RwMatrix*, EDI = f32* weight, EAX = dst, ESI = 0 accumulate / non-zero store.
  `x = (((r.x*s.x + u.x*s.y) + a.x*s.z) + p.x) * w`, same for y (`r.y, u.y, a.y, p.y`) and z (0x7cb828..0x7cb87d). Accumulate:
  `dst.k = x_k + dst.k`; store: `dst.k = x_k` (0x7cb87f..0x7cb8ac).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cb8b0 sub_7cb8b0 (proposed: _rpSkinBlendNormalHelper)  (catalogue: unknown/REA, port level 0)
- As 0x7cb828 without the translation term (0x7cb8b0..0x7cb928).
- librw: no direct equivalent identified in librw (not compared line by line).

