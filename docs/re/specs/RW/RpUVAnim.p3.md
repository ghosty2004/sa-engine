# RenderWare 3.6 RpUVAnim (plugin 0x135) - pass-3 specs, writer A (RE-SPEC-RW-3)

Statically linked RW 3.6 toolkit code; gta-reversed only has `plugin::Call*` thunks for the public API, so everything
comes from the bytes. All `__cdecl`. Extends docs/re/specs/RW/RpUVAnim.md (pass 1, read-only).

## Layouts used (offsets proven by the instruction addresses given)

Globals: `[0xc9b8d0]` i32 material extension offset (0x7cbb30); `[0xc978f4]` RwFreeList* of 0x44-byte custom-data
blocks (0x7cbe28, freed 0x7cba44); `[0xc978fc]` i32 engine-open counter (decremented 0x7cba2c); `RpUVAnimDictSchema`
at 0x8ded50 (0x7cba26); interpolator infos at 0x8ded84 (`[0x8ded84] = 0x1c0` linear) and 0x8dedb4 (`[0x8dedb4] =
0x1c1` parameterised) (0x7cc031/0x7cc039). `G` = `[0xc97b24]`: G+0x144 freeListAlloc(fl, hint), G+0x148 freeListFree(fl, p).

Material extension (0x28 bytes, `mat + [0xc9b8d0]`): +0x0/+0x4 `RwMatrix* uvTransform[2]`, +0x8..+0x24
`RtAnimInterpolator* interp[8]` (0x7cbb4c, 0x7cc16d).

UV custom data (0x44 bytes, `anim->customData` at `RtAnimAnimation+0x14`): +0x0 `char name[32]` (byte +0x1f forced 0,
0x7cbe5d), +0x20 `u32 nodeToUVChannelMap[8]` (0x7cbda0), +0x40 `i32 refCount` (0x7cbe65).
`RtAnimAnimation`: +0x0 interpInfo (+0x0 typeID, +0x4 keyFrameSize used for interpolator creation, +0x8 keyFrameSize
used for frame stepping; 0x7cbcec, 0x7cbe78), +0x4 numFrames (0x7cc8ba), +0x10 pFrames (0x7cc8b4), +0x14 customData.
`RtAnimInterpolator`: +0x0 pCurrentAnim, +0x24 currentInterpKeyFrameSize (0x7cc18a), +0x2c numNodes (0x7cbda6), +0x3c
keyFrameApplyCB (0x7cc18d), +0x4c interpolated key frames (0x7cc19a).
Key frames (0x20 bytes): +0x0 prev, +0x4 time; linear (0x1c0) data +0x8 a, +0xc b, +0x10 c, +0x14 d (2x2 UV matrix),
+0x18 tx, +0x1c ty; parameterised (0x1c1) data +0x8 angle (radians), +0xc scale.x, +0x10 scale.y, +0x14 skew, +0x18 pos.x,
+0x1c pos.y (0x7cc581..0x7cc5b4).
Constants: `[0x859878] = 57.29578f` (rad->deg), `[0x882f2c] = -57.29578f`, `[0x858cc0] = -3.1415927f`, `[0x858cb8] =
3.1415927f`, `[0x858cbc] = 6.2831855f`, `[0x858b50] = 0.0f`, `[0x858624] = 1.0f`; V3d constants at 0x882ed4 (-0.5f,
-0.5f, 0) and 0x882ee0 (0.5f, 0.5f, 0): `[0x882ed4] = -0.5f`, `[0x882ee0] = 0.5f`.

## 0x7cba20 cb_UVANIMPLUGIN_RwEngine_Close  (catalogue: specified/REA, port level 0)
- `[0xc978fc] -= 1` (0x7cba2c); `RtDictSchemaDestruct(0x8ded50)` (0x7cba32); if `[0xc978f4]` -> `RwFreeListDestroy`, set
  NULL (0x7cba44/0x7cba4c); return instance.
- librw: uvanim.cpp:uvanimClose differs (does nothing).

## 0x7cbb30 cb_UVANIMPLUGIN_RpMaterial_Copy  (catalogue: specified/REA, port level 0)
- `void* (void* dst, const void* src, i32 off, i32 size)`: copies all 0x28 extension bytes (`rep movsd` 10, 0x7cbb4f) -
  the two uvTransform matrix POINTERS are shared, not cloned; then for each of the 8 slots of dst, if non-NULL:
  `slot->pCurrentAnim->customData->refCount++` (0x7cbb61). Returns dst.
- Edge: shared matrices are destroyed twice when both materials die (pass-1 dtor destroys them). Kept.
- librw: uvanim.cpp:copyUVAnim differs (copies interpolator pointers similarly, no matrices in librw).

## 0x7cbb70 cb_UVANIMPLUGIN_RpMaterial_StreamWrite  (catalogue: specified/REA, port level 1)
- `RwStream* (RwStream* s, i32 len, const RpMaterial* m, i32 off, i32 size)`: `mask` = bit i for every non-NULL slot,
  `size = 4 + 0x20*count` (0x7cbb86..0x7cbbb3); `_rwStreamWriteVersionedChunkHeader(s, 1, size, 0x36003, 0xffff)`
  (0x7cbbc7) fail -> NULL; `RwStreamWriteInt32(s, &mask, 4)` fail -> NULL; for each non-NULL slot (in order)
  `RwStreamWrite(s, slot->pCurrentAnim->customData, 0x20)` (the name, result ignored, 0x7cbc05); return s.
- librw: uvanim.cpp:writeUVAnim matches.

## 0x7cbc20 cb_UVANIMPLUGIN_RpMaterial_StreamRead  (catalogue: specified/REA, port level 1)
- `RwStream* (RwStream* s, i32 len, RpMaterial* m, i32 off, i32 size)`: `ext = m + [0xc9b8d0]`;
  `RwStreamFindChunk(s, 1, NULL, NULL)` fail -> NULL; `RwStreamReadInt32(s, &mask, 4)` fail -> NULL (mask stored in the
  `m` argument slot). For `i = 0..7` with `mask & (1 << i)`: read the 32-byte name (0x7cbc8b, fail -> NULL); `dict =
  RtDictSchemaGetCurrentDict(0x8ded50)`; if dict: `anim = RtDictFindNamedEntry(dict, name)`, found -> `refCount++`
  (0x7cbcc3); not found or no dict -> `anim = 0x7cbe00(name)` (dummy). `anim == NULL` -> NULL. `it =
  RtAnimInterpolatorCreate(RtAnimAnimationGetNumNodes(anim), anim->interpInfo->+4)` (0x7cbcfa); `ext->interp[i] = it`;
  if `it == NULL` or `RtAnimInterpolatorSetCurrentAnim(it, anim) == 0`: release anim (`--refCount`; at 0
  `freeListFree(cd)` + `RtAnimAnimationDestroy`, 0x7cbd3e..0x7cbd61), return NULL. After the loop return `0x7cbd80(ext)
  ? s : NULL` (0x7cbd28..0x7cbd35).
- librw: uvanim.cpp:readUVAnim matches in structure (librw makes the dummy with its own helper, no matrix setup).

## 0x7cbd80 sub_7cbd80 (proposed: _rpUVAnimMaterialExtUpdateTransforms)  (catalogue: unknown/REA, port level 1)
- `ext* (RpUVAnimMaterialExt* ext)`; returns ext, or NULL when a matrix allocation fails (0x7cbdf5).
- For `ch = 0, 1`: search all 8 slots (non-NULL) and their nodes `n < it->numNodes` for `customData->nodeToUVChannelMap[n]
  == ch` (0x7cbd95..0x7cbdb7). Found: if `uvTransform[ch] == NULL` -> `RwMatrixCreate()` (0x7cbde5), NULL -> return NULL.
  Not found: if present `RwMatrixDestroy(uvTransform[ch])`, `uvTransform[ch] = NULL` (0x7cbdc0..0x7cbdc8).
- librw: absent (librw has no per-channel matrices).

## 0x7cbe00 sub_7cbe00 (proposed: _rpUVAnimCreateDummy)  (catalogue: unknown/REA, port level 5)
- `RtAnimAnimation* (const char* name)`: `a = RtAnimAnimationCreate([0x8ded84] = 0x1c0, 2, 0, 1.0f)` (0x7cbe16); NULL ->
  NULL. `cd = freeListAlloc([0xc978f4], 0x30135)`; NULL -> NULL (a leaked). `strncpy(cd, name, 0x20)`, `cd[0x1f] = 0`,
  `cd->map[0] = 0` (only the first entry, 0x7cbe63), `refCount = 1`, `a->customData = cd`. Local RwMatrix `M` = identity
  with `flags = (uninitialised flags) | 0x20003` (0x7cbe71..0x7cbee9). `0x7cc810(a, frames[0], NULL, 0, &M)` (0x7cbef1),
  `0x7cc810(a, frames[1], frames[0], 1.0f, &M)` (frame 1 = `pFrames + interpInfo->+8`, 0x7cbf03). Returns a.
- librw: uvanim.cpp:makeDummyAnimation matches in purpose (identity frames).

## 0x7cbf20 cb_UVANIMPLUGIN_RpMaterial_StreamGetSize  (catalogue: specified/REA, port level 0)
- No non-NULL slot -> 0 (0x7cbf3f); else `0x10 + 0x20 * count` (0x7cbf47..0x7cbf5d).
- librw: uvanim.cpp:getSizeUVAnim matches.

## 0x7cbf70 _rpUVAnimCustomDataStreamRead  (catalogue: specified/REA, port level 0)
- `cd* (RwStream* s)`: `cd = freeListAlloc([0xc978f4], 0x30135)`; NULL -> NULL. `RwStreamRead(s, cd, 0x20) == 0` -> NULL
  (cd leaked; any non-zero count accepted, 0x7cbfa5); `RwStreamReadInt32(s, cd+0x20, 0x20)` fail -> NULL; `refCount = 1`;
  return cd.
- librw: uvanim.cpp:uvAnimStreamRead matches in content.

## 0x7cbfd0 _rpUVAnimCustomDataStreamWrite  (catalogue: specified/REA, port level 0)
- `cd* (const cd* c, RwStream* s)`: `RwStreamWrite(s, c, 0x20)` fail -> NULL; `RwStreamWriteInt32(s, c+0x20, 0x20)`;
  return success ? c : NULL (0x7cbffc..0x7cc001).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc010 _rpUVAnimCustomDataStreamGetSize  (catalogue: specified/REA, port level 0)
- Returns 0x40.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc020 RpUVAnimCreate  (catalogue: specified/REA, port level 2)
- `RtAnimAnimation* (const char* name, u32 numNodes, i32 numFrames, f32 duration, const u32* nodeIndexToUVChannelMap,
  i32 type)`: `type` 0 -> info 0x8ded84, 1 -> 0x8dedb4, else NULL (0x7cc025..0x7cc02b). `a = RtAnimAnimationCreate(typeID,
  numFrames, 0, duration)`; NULL -> NULL. `cd = freeListAlloc(..., 0x30135)`; NULL -> NULL (a leaked). `strncpy(cd, name,
  0x20)`; `cd[0x1f] = 0`; copy `numNodes` dwords from the map into `cd+0x20` (`rep movsd`, no bound: numNodes > 8 overflows
  into refCount and beyond, 0x7cc0a1); `refCount = 1`; `a->customData = cd`; return a.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc0c0 RpUVAnimDestroy  (catalogue: specified/REA, port level 1)
- `--cd->refCount`; at 0: `freeListFree([0xc978f4], cd)`, `RtAnimAnimationDestroy(a)` (0x7cc0e2/0x7cc0e9); return 1.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc100 RpUVAnimAddRef  (catalogue: specified/REA, port level 0)
- `a->customData->refCount++` (0x7cc107); EAX = `a` (left from the load).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc110 RpMaterialUVAnimApplyUpdate  (catalogue: specified/REA, port level 2)
- `RpMaterial* (RpMaterial* m)`: `ext = m + [0xc9b8d0]`.
  1. For `ch = 0, 1` with `uvTransform[ch]`: set it to identity (`+0, +0x14, +0x28 = 1.0f`, the other 9 elements 0) and
     `flags |= 0x20003` (0x7cc135..0x7cc164).
  2. For each of the 8 slots (counter 0x7cc170) with `it != NULL`: `apply = it->+0x3c`, `stride = it->+0x24`, `map =
     cd->nodeToUVChannelMap`, `f = it + 0x4c`. For `n < it->numNodes` (signed): `ch = map[n]`; if `ch < 2` (unsigned) and
     `uvTransform[ch]`: build local `M`: `apply == 0x7cc9f0` -> inlined RpUVAnimLinearKeyFrameApply (0x7cc1de..0x7cc23c);
     `apply == 0x7cc560` -> inlined RpUVAnimParamKeyFrameApply incl. the translate/rotate/translate sequence
     (0x7cc254..0x7cc313); else `apply(&M, f)` (0x7cc322). `RwMatrixTransform(uvTransform[ch], &M, 1 /*PRECONCAT*/)`
     (0x7cc32f) and only then `f += stride` (0x7cc349).
  3. `RpMatFXMaterialSetUVTransformMatrices(m, uvTransform[0], uvTransform[1])` (0x7cc387); return m.
- Bug kept: for a node with `ch >= 2` or no matrix the frame pointer is NOT advanced (0x7cc1c2 / 0x7cc1cd jump past
  0x7cc349), so following nodes read the wrong interpolated frame.
- librw: uvanim.cpp:UVAnim::applyUpdate differs (librw advances per node, applies to its own matrices).

## 0x7cc3a0 RpUVAnimGetName  (catalogue: specified/REA, port level 0)
- Returns `a->customData` (the name is at +0, 0x7cc3a4).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc3b0 RpMaterialSetUVAnim  (catalogue: specified/REA, port level 2)
- `RpMaterial* (RpMaterial* m, RtAnimAnimation* a, u32 slot)`: `ext->interp[slot] = RtAnimInterpolatorCreate(
  GetNumNodes(a), a->interpInfo->+4)` (old slot value overwritten without destroy; 0x7cc3e0); NULL -> return NULL.
  `RtAnimInterpolatorSetCurrentAnim(it, a)` (result ignored); if `0x7cbd80(ext) == NULL`: `RtAnimInterpolatorDestroy(
  slot)` (pointer left in the slot), return NULL. Else `cd->refCount++`, return m.
- librw: absent as a function (librw sets interpolators directly).

## 0x7cc430 RpMaterialUVAnimGetInterpolator  (catalogue: specified/REA, port level 0)
- `RtAnimInterpolator* (RpMaterial* m, u32 slot)`: `*(m + [0xc9b8d0] + 8 + slot*4)` (0x7cc43a..0x7cc442).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc450 RpMaterialUVAnimSetInterpolator  (catalogue: specified/REA, port level 0)
- `(m, slot, it)`: stores `it` at the same address (0x7cc466); EAX = m.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc530 RpMaterialUVAnimExists  (catalogue: specified/REA, port level 0)
- Returns 1 if any of the 8 slots is non-NULL (0x7cc540..0x7cc551), else 0.
- librw: uvanim.cpp:UVAnim::exists matches.

## 0x7cc560 RpUVAnimParamKeyFrameApply  (catalogue: specified/REA, port level 0)
- `void (RwMatrix* M, const kf* f)`: `M.right = (sx, skew, 0)`, `M.up = (0, sy, 0)`, `M.at = (0, 0, 0)`, `M.pos = (px, py,
  0)` (flags untouched; 0x7cc581..0x7cc5b7); `RwMatrixUpdate(M)`; `RwMatrixTranslate(M, (-0.5,-0.5,0), 2)` (0x7cc5c7);
  `RwMatrixRotate(M, (0,0,1), f32(angle * 57.29578), 2)` (0x7cc5e4); `RwMatrixTranslate(M, (0.5,0.5,0), 2)` (0x7cc5f1).
- librw: uvanim.cpp:uvAnimParamApplyCB differs (builds the matrix arithmetically with cos/sin; exe uses RW matrix ops).

## 0x7cc600 RpUVAnimParamKeyFrameInterpolate  (catalogue: specified/REA, port level 0)
- `void (kf* out, const kf* a, const kf* b, f32 t)`: `u = (t - a.time) / (b.time - a.time)` (register); `d = b.angle -
  a.angle`; if `d < -pi [0x858cc0]` -> `d += 2pi [0x858cbc]`; else if `d > pi [0x858cb8]` -> `d -= 2pi` (0x7cc622..0x7cc646);
  `out.angle = d*u + a.angle`; the five other fields `out.k = (b.k - a.k)*u + a.k` (0x7cc654..0x7cc697). `out.time` and
  `out.prev` not written.
- librw: uvanim.cpp:uvAnimParamInterpCB matches the formula (check its angle wrap).

## 0x7cc6a0 RpUVAnimParamKeyFrameBlend  (catalogue: specified/REA, port level 0)
- `void (kf* out, const kf* a, const kf* b, f32 alpha)`: the same angle wrap and lerps with `alpha` (0x7cc6ad..0x7cc735).
- librw: absent.

## 0x7cc760 sub_7cc760 (proposed: _rpUVAnimParamFromMatrix)  (catalogue: unknown/REA, port level 3)
- `f32* (f32 out[6], const RwMatrix* m)`: copy m (16 dwords) to a local; `out[0] = atan2(-m.up.x, m.right.x)` (`fpatan`,
  0x7cc776..0x7cc7a8); local: translate (-0.5,-0.5,0) postconcat, rotate about (0,0,1) by `out[0] * -57.29578 [0x882f2c]`
  (0x7cc7ca), translate (0.5,0.5,0); `out[1] = L.right.x`, `out[2] = L.up.y`, `out[3] = L.right.y`, `out[4] = L.pos.x`,
  `out[5] = L.pos.y` (0x7cc7ef..0x7cc803); returns out.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc810 sub_7cc810 (proposed: _rpUVAnimKeyFrameSet)  (catalogue: unknown/REA, port level 4)
- `i32 (RtAnimAnimation* a, kf* f, kf* prev, f32 time, const RwMatrix* m)`: `f->prev = prev`, `f->time = time`
  (0x7cc827/0x7cc829); typeID `== [0x8ded84]` -> `0x7ccc50(f+8, m)` (0x7cc83f); else `== [0x8dedb4]` -> `0x7cc760(f+8, m)`
  (0x7cc85e). Returns 0 always (0x7cc868).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc870 RpUVAnimKeyFrameStreamRead  (catalogue: specified/REA, port level 1)
- `RtAnimAnimation* (RwStream* s, RtAnimAnimation* a)`: `RwStreamReadInt32(s, &unused, 4)` fail -> NULL (value
  discarded); `cd = _rpUVAnimCustomDataStreamRead(s)` NULL -> NULL; `a->customData = cd`. For `i < a->numFrames`:
  `RwStreamReadReal(s, &f->time, 0x1c)` (time + 6 data floats), `RwStreamReadInt32(s, &k, 4)`, `f->prev = pFrames + k*0x20`
  (0x7cc8c3..0x7cc8ff); failures -> NULL. Returns a.
- librw: uvanim.cpp:uvAnimStreamRead matches.

## 0x7cc920 RpUVAnimKeyFrameStreamWrite  (catalogue: specified/REA, port level 1)
- `RwBool (RwStream* s, const RtAnimAnimation* a)`: `RwStreamWriteInt32(s, &zero, 4)`; `_rpUVAnimCustomDataStreamWrite(cd,
  s)`; per frame `RwStreamWriteReal(s, &f->time, 0x1c)`, `RwStreamWriteInt32((f->prev - pFrames) >> 5)` (`sar`, 0x7cc99c);
  any failure -> 0; return 1.
- librw: uvanim.cpp:uvAnimStreamWrite matches.

## 0x7cc9d0 RpUVAnimKeyFrameStreamGetSize  (catalogue: specified/REA, port level 0)
- `_rpUVAnimCustomDataStreamGetSize(cd) + 4 + numFrames*0x20` (0x7cc9d9..0x7cc9ec).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7cc9f0 RpUVAnimLinearKeyFrameApply  (catalogue: specified/REA, port level 0)
- `void (RwMatrix* M, const kf* f)`: `M.right = (a, b, 0)`, `M.up = (c, d, 0)`, `M.at = 0`, `M.pos = (tx, ty, 0)`
  (0x7cc9fa..0x7cca2e); `RwMatrixUpdate(M)` (0x7cca31).
- librw: uvanim.cpp:uvAnimLinearApplyCB matches.

## 0x7cca40 RpUVAnimLinearKeyFrameInterpolate  (catalogue: specified/REA, port level 0)
- `u = (t - a.time) / (b.time - a.time)`; for the six data floats `out.k = (b.k - a.k)*u + a.k` (0x7cca5b..0x7ccaac).
- librw: uvanim.cpp:uvAnimLinearInterpCB matches.

## 0x7ccac0 RpUVAnimLinearKeyFrameBlend  (catalogue: specified/REA, port level 0)
- `out.k = (b.k - a.k)*alpha + a.k` for the six data floats (0x7ccacc..0x7ccb29). librw: absent.

## 0x7ccb30 RpUVAnimLinearKeyFrameMulRecip  (catalogue: specified/REA, port level 0)
- `void (kf* f, const kf* s)`: `det = s.a*s.d - s.b*s.c` (0x7ccb3b..0x7ccb47). If `det != 0.0` (or NaN; `jnp` skips only
  on equal, 0x7ccb54): `r = 1.0 / det`; `t = f32(-(r*s.c))` (0x7ccb7e..0x7ccb83); `f.a = (-(r*s.b))*f.c + (r*s.d)*f.a`
  (0x7ccb89..0x7ccb93), `f.b = (-(r*s.b))*f.d + (r*s.d)*f.b` (0x7ccb96..0x7ccba4, using the saved old f.b/f.d), `f.c = (f.c
  + f.a_old) * t` (0x7ccba9..0x7ccbaf), `f.d = (f.d_old + f.b_old) * t` (0x7ccbb4..0x7ccbc0). Always `f.tx -= s.tx`, `f.ty
  -= s.ty` (0x7ccbc7..0x7ccbd6).
- NOTE: the c/d rows are NOT the mathematical inverse product (the `r*s.a` term is missing: `c' = (c + a)*(-r*s.c)`);
  original behaviour, kept. librw: absent.

## 0x7ccbe0 RpUVAnimLinearKeyFrameAdd  (catalogue: specified/REA, port level 0)
- `void (kf* out, const kf* x, const kf* y)`: `out.a = y.a*x.a + y.c*x.b`, `out.b = y.b*x.a + y.d*x.b`, `out.c = y.c*x.d +
  y.a*x.c`, `out.d = y.d*x.d + y.b*x.c`, `out.tx = y.tx + x.tx`, `out.ty = y.ty + x.ty` (0x7ccbec..0x7ccc3f; translation
  is not transformed). librw: absent.

## 0x7ccc50 sub_7ccc50 (proposed: _rpUVAnimLinearFromMatrix)  (catalogue: unknown/REA, port level 0)
- `f32* (f32 out[6], const RwMatrix* m)`: `out = (m.right.x, m.right.y, m.up.x, m.up.y, m.pos.x, m.pos.y)` (dword copies
  0x7ccc58..0x7ccc77); returns out.
- librw: no direct equivalent identified in librw (not compared line by line).

