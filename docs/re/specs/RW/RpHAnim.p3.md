# RenderWare 3.6 RpHAnim (plugin 0x11e) - pass-3 specs, writer A (RE-SPEC-RW-3)

Statically linked RW 3.6 plugin code; gta-reversed has only `plugin::Call*` thunks. All `__cdecl`. Extends
docs/re/specs/RW/RpHAnim.md (pass 1, read-only); its layout tables apply. Facts used here, with proof:

## Layouts used (offsets proven by the instruction addresses given)

- `[0xc9b8c8]` i32 frame extension offset (0x7c48ff); ext +0x0 `i32 id` (0x7c4bfa), +0x4 `RpHAnimHierarchy*` (0x7c491e).
- `[0xc9b8cc]` RwFreeList* hierarchy freelist (0x7c4720, 0x7c4dfb). `G = [0xc97b24]`: +0x138 rwfree (0x7c4d4e), +0x144
  freeListAlloc (0x7c4e0c), +0x148 freeListFree (0x7c4d8a).
- `RpHAnimHierarchy`: +0x0 flags (1 SUB, 2 NOMATRICES; 0x7c4d3c/0x7c4d40), +0x4 numNodes, +0x8 pMatrixArray (0x7c4ea7),
  +0xc pMatrixArrayUnaligned (0x7c4d44), +0x10 pNodeInfo (0x7c4d57), +0x14 parentFrame (0x7c4d39), +0x18 parentHierarchy
  (0x7c4eaf), +0x1c rootParentOffset (0x7c4e40), +0x20 currentAnim (0x7c4d69).
- `RpHAnimNodeInfo` (0x10): +0 nodeID (0x7c4f63), +4 nodeIndex (0x7c4999), +8 flags (2 PUSH, 1 POP; 0x7c4de7/0x7c4ded), +0xc
  frame (0x7c4f67).
- `RtAnimInterpolator` +0x20 (written to the stream as "max key frame size", 0x7c4963).
- `RpHAnimKeyFrame` (0x24 bytes): +0x0 prev, +0x4 time, +0x8 quat (x, y, z, w at +0x8/+0xc/+0x10/+0x14), +0x18 translation
  (+0x18/+0x1c/+0x20) (0x7c5b87..0x7c5c8f). `RtAnimAnimation` +0x4 numFrames (0x7c64c7), +0x10 pFrames (0x7c64ca).
- Constants `[0x858624] = 1.0f`, `[0x858b50] = 0.0f`.

## 0x7c4720 cb_HANIMPLUGIN_RwEngine_Close  (catalogue: specified/REA, port level 0)
- If `[0xc9b8cc]`: `RwFreeListDestroy`, set NULL (0x7c472a/0x7c4732); return instance.
- librw: hanim.cpp has no engine close (no freelist).

## 0x7c48d0 cb_HANIMPLUGIN_RwFrame_StreamWrite  (catalogue: specified/REA, port level 1)
- `RwStream* (RwStream* s, i32 len, const RwFrame* f, i32 off, i32 size)`: `RwStreamWriteInt32(s, &0x100, 4)` (version,
  0x7c48e1); `WriteInt32(&ext->id)`. `h = ext->hierarchy`; if `h && !(h->flags & 1)` (0x7c4923/0x7c492b): `WriteInt32(
  &h->numNodes)`, `(&h->flags)`, `(&h->currentAnim->+0x20)` (0x7c493b..0x7c496d); then for each node `n < numNodes`:
  `nodeID`, `nodeIndex`, `flags` (0x7c4989..0x7c49c8). Else `WriteInt32(&0)` (numNodes = 0, 0x7c49d2..0x7c49de). Any
  failure -> NULL; else s.
- librw: hanim.cpp:writeHAnim matches.

## 0x7c4bf0 cb_HANIMPLUGIN_RwFrame_StreamGetSize  (catalogue: specified/REA, port level 0)
- `ext->id == -1 && ext->hierarchy == NULL` -> 0 (0x7c4bfa..0x7c4c06); hierarchy NULL or sub (`flags & 1`) -> 0xc;
  else `numNodes*12 + 0x14` (0x7c4c1d..0x7c4c20).
- librw: hanim.cpp:getSizeHAnim matches.

## 0x7c4d30 RpHAnimHierarchyDestroy  (catalogue: specified/REA, port level 0)
- `RpHAnimHierarchy* (RpHAnimHierarchy* h)`; returns NULL. `pf = h->parentFrame`. If `!(flags & 1)`: if `!(flags & 2)`
  `rwfree(h->pMatrixArrayUnaligned)` (0x7c4d4e); `rwfree(h->pNodeInfo)` (0x7c4d60). Then `+0xc = +0x8 = +0x10 = 0`;
  `RtAnimInterpolatorDestroy(h->currentAnim)` (0x7c4d78); `freeListFree([0xc9b8cc], h)` (0x7c4d8a); if `pf`:
  `*(pf + [0xc9b8c8] + 4) = NULL` (0x7c4d9f).
- librw: hanim.cpp:HAnimHierarchy::destroy differs (no parent-frame clear, frees unconditionally).

## 0x7c4db0 RpHAnimHierarchyCreateSubHierarchy  (catalogue: specified/REA, port level 1)
- `RpHAnimHierarchy* (RpHAnimHierarchy* h, i32 start, u32 flags, i32 maxKeyFrameSize)`.
- Node count: `n = 1`; if `info[start].flags & 1 (POP)` == 0: depth `d = 1`, walk `i = start+1, ...`: PUSH -> `d++`,
  POP -> `d--`, `n++`, until `d == 0` (0x7c4dd6..0x7c4df9).
- `s = freeListAlloc([0xc9b8cc], 0x3011e)` (no NULL check); `s->flags = flags | 1`, `s->numNodes = n`; `s->currentAnim =
  RtAnimInterpolatorCreateSubInterpolator(h->currentAnim, start, n, maxKeyFrameSize)` (0x7c4e31).
- `rootParentOffset`: `start == 0` -> -1 (0x7c4e40); else `p = -1`, local stack (on the frame, no bound check); for
  `i < start`: PUSH -> push `p`; then POP -> `p = pop` else `p = i` (0x7c4e5c..0x7c4e7e); result `p` (0x7c4e87).
- `s->parentFrame = info[start].frame` (0x7c4e98), `s->pMatrixArray = h->pMatrixArray + start*0x40` (0x7c4ea7),
  `s->pMatrixArrayUnaligned = 0`, `s->parentHierarchy = h`, `s->pNodeInfo = h->pNodeInfo + start*0x10`; return s.
- librw: hanim.cpp has no sub-hierarchies (absent).

## 0x7c4f40 RpHAnimHierarchyAttach  (catalogue: specified/REA, port level 0)
- `h* (h)`: `f = h->parentFrame`; for each node with `info.nodeID == ext(f)->id`: `info.frame = f` (0x7c4f55..0x7c4f73);
  `RwFrameUpdateObjects(f)` (0x7c4f77); `RwFrameForAllChildren(f, 0x7c4f90, h)` (0x7c4f83); return h.
- librw: hanim.cpp:HAnimHierarchy::attach differs (librw walks by index order).

## 0x7c4f90 _rpHAnimHierarchyAttachFrameCB  (catalogue: specified/REA, port level 0)
- `RwFrame* (RwFrame* f, h)`: same node match/assign for `f` (0x7c4fa6..0x7c4fc4), `RwFrameUpdateObjects(f)`, recurse
  `RwFrameForAllChildren(f, 0x7c4f90, h)`; return f. Every node whose id matches gets the frame (no early break).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c4ff0 RpHAnimHierarchyDetach  (catalogue: specified/REA, port level 0)
- For `i < numNodes`: `info[i].frame = NULL` (0x7c5004); EAX = h.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c5020 RpHAnimHierarchyAttachFrameIndex  (catalogue: specified/REA, port level 1)
- `h* (h, i32 index)`: `id = info[index].nodeID`; `f = h->parentFrame`; if `ext(f)->id == id`: every node with `nodeID == id`
  gets `frame = f` (0x7c5048..0x7c5073). `RwFrameUpdateObjects(f)`; `RwFrameForAllChildren(f, 0x7c50a0, &{h, id})`
  (0x7c5086); return h.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c50a0 _rpHAnimHierarchyAttachFrameIndexCB  (catalogue: specified/REA, port level 0)
- `RwFrame* (RwFrame* f, {h, id}*)`: if `ext(f)->id == id`: nodes with that id get `frame = f` (0x7c50bc..0x7c50e0);
  `RwFrameUpdateObjects(f)`; recurse; return f.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c5100 RpHAnimHierarchyDetachFrameIndex  (catalogue: specified/REA, port level 0)
- `info[index].frame = NULL` (0x7c510e); EAX = h.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c5170 RpHAnimFrameSetID  (catalogue: specified/REA, port level 0)
- `ext(f)->id = id` (0x7c517e); returns 1.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c5b80 RpHAnimKeyFrameApply  (catalogue: specified/REA, port level 0)
- `void (RwMatrix* m, const RpHAnimKeyFrame* k)`, x87 with f32 temps: `xx, yy, zz, zy, zx, yx, wx` stored as f32
  (0x7c5b8d..0x7c5bdd), `wy`, `wz` kept in registers. Results (`2*t` computed as `t + t`):
  `m.right.x = 1.0 - 2*(zz + yy)` (0x7c5bf3..0x7c5c03), `right.y = 2*(yx + wz)`, `right.z = 2*(zx - wy)`, `up.x = 2*(yx -
  wz)`, `up.y = 1.0 - 2*(zz + xx)`, `up.z = 2*(wx + zy)`, `at.x = 2*(wy + zx)`, `at.y = 2*(zy - wx)`, `at.z = 1.0 - 2*(yy +
  xx)` (0x7c5c05..0x7c5c7d); `flags = 3` (0x7c5c70); `pos = k->trans` (0x7c5c83..0x7c5c92). Pads untouched.
- librw: hanim.cpp:hanimApplyCB matches in purpose (quaternion via its own Matrix code; sum order differs; exe wins).

## 0x7c64c0 RpHAnimKeyFrameStreamRead  (catalogue: specified/REA, port level 0)
- `RtAnimAnimation* (RwStream* s, RtAnimAnimation* a)`: for `i < numFrames`: `RwStreamReadReal(s, &k->time, 0x20)`
  (time, quat, trans), `RwStreamReadInt32(s, &v, 4)`; `k->prev = pFrames + (v / 36)*36` (unsigned magic division
  0x38e38e39 >> 3, 0x7c6505..0x7c6518): the stored value is a BYTE offset, rounded down to a frame boundary. Failures ->
  NULL; return a.
- librw: hanim.cpp:hAnimFrameRead matches (`prev = readI32()/0x24`).

## 0x7c6540 RpHAnimKeyFrameStreamWrite  (catalogue: specified/REA, port level 0)
- Per frame `RwStreamWriteReal(s, &k->time, 0x20)`, `RwStreamWriteInt32(&(k->prev - pFrames) /*bytes*/, 4)` (0x7c657b);
  failure -> 0; return 1.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c65b0 RpHAnimKeyFrameStreamGetSize  (catalogue: specified/REA, port level 0)
- `numFrames * 36` (0x7c65b7..0x7c65ba).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7c65c0 RpHAnimKeyFrameMulRecip  (catalogue: specified/REA, port level 0)
- `void (RpHAnimKeyFrame* f, const RpHAnimKeyFrame* s)`: `n = s.x*s.x + s.y*s.y + s.z*s.z + s.w*s.w` (0x7c65d3..0x7c65e7).
  If `n > 0.0` (0x7c65f1..0x7c65fe; `<= 0` and NaN skip): `r = 1/n`; inverse `iw = f32(r*s.w)`, `ix = f32(s.x*(-r))`, `iy =
  f32(s.y*(-r))`, `iz = (-r)*s.z` (register) (0x7c6600..0x7c6623). Else the locals stay UNINITIALISED (only `iz` is loaded
  from an uninitialised slot, 0x7c662a) - original bug, garbage result.
- With `f` copied to locals: `f.w = fw*iw - ((fy*iy + fx*ix) + fz*iz)` (0x7c6654..0x7c6678); `f.x = ((fz*iy - fy*iz) +
  fw*ix) + fx*iw` (0x7c667b..0x7c669f); `f.y = ((fx*iz - fz*ix) + fw*iy) + fy*iw` (0x7c66a1..0x7c66c5); `f.z = ((fy*ix -
  fx*iy) + fw*iz) + fz*iw` (0x7c66c8..0x7c66ee); `f.t -= s.t` (0x7c66f3..0x7c670b).
- librw: absent (librw has no mulRecip callback).

## 0x7c6720 RpHAnimKeyFrameAdd  (catalogue: specified/REA, port level 0)
- `void (RpHAnimKeyFrame* out, const RpHAnimKeyFrame* a, const RpHAnimKeyFrame* b)`: `out.w = b.w*a.w - ((b.y*a.y +
  b.x*a.x) + b.z*a.z)` (0x7c672c..0x7c674a); `out.x = a.y*b.z - b.y*a.z`, `out.y = b.x*a.z - a.x*b.z`, `out.z = b.y*a.x -
  b.x*a.y` (0x7c674d..0x7c677d); then in place `out.k += a.w*b.k` (k = x, y, z) and `out.k += b.w*a.k` (each stored as f32,
  0x7c6780..0x7c67c5); `out.t = a.t + b.t` (0x7c67c8..0x7c67e0). If `out` aliases `a` or `b` the in-place steps read
  modified values (kept).
- librw: absent (librw has no add callback).

## 0x7c5ca0 RpHAnimKeyFrameInterpolate  (catalogue: specified/REA, port level 0)
- `void (RpHAnimKeyFrame* out, const RpHAnimKeyFrame* a, RpHAnimKeyFrame* b, f32 t)` (out ebx, a esi, b edi; 0x7c5ca2..0x7c5cab).
- 1. `dot = b.x*a.x + a.y*b.y + b.z*a.z + a.w*b.w` (x87 register, 0x7c5caf..0x7c5ccb). `u = f32((t - a.time) / (b.time - a.time))`
  (stored in a dead argument slot, 0x7c5ccd..0x7c5cdc). `out.t = (b.t - a.t)*u + a.t` for x, y, z (0x7c5ce0..0x7c5d0d).
- 2. If `dot < 0.0` (exactly C0): `dot = -dot` and `b`'s quaternion is negated IN PLACE (x, y, z, w; 0x7c5d1d..0x7c5d3c) - the input key
  frame is modified (kept).
- 3. `sa = f32(1.0 - u)`. If `!(dot < 0.999 [0x863e2c])` (also NaN, 0x7c5d4d..0x7c5d5a): weights `wa = sa`, `wb = u` (plain lerp).
  Else: `theta = acosf(dot)` with the inlined fdlibm acos described in RpLight.p3.md 0x751ae0 (same constants, 0x7c5d60..0x7c5f8f);
  `sin(x)` is the inlined polynomial `x + (x*x*x) * ((((((S6*z - S5)*z + S4)*z - S3)*z + S2)*z) - S1)` with `z = x*x` (0x7c5f91..0x7c5fc9;
  `S6 = [0x86d2b0] = 1.589691e-10`, `S5 = [0x86d2ac] = 2.505076e-08`, `S4 = [0x86d2a8] = 2.7557314e-06`, `S3 = [0x86d2a4] = 0.0001984127`,
  `S2 = [0x864e0c] = 0.008333334`, `S1 = [0x85f0a0] = 0.16666667`; no range reduction); `inv = 1.0 / sin(theta)` (0x7c5fcb);
  `wa = f32(sin(theta*sa) * inv)` (0x7c5fd1..0x7c6013), `wb = f32(sin(theta*u) * inv)` (0x7c6019..0x7c605b).
- 4. `out.x = wb*b.x + wa*a.x`, `out.y = wa*a.y + wb*b.y`, `out.z = wb*b.z + wa*a.z`, `out.w = wa*a.w + wb*b.w` (0x7c6063..0x7c60ae);
  no normalisation; `out.time`/`out.prev` not written.
- librw: hanim.cpp:hanimInterpCB differs (librw uses its own slerp with acos/sin from libm, does not modify the input frame).

## 0x7c60c0 RpHAnimKeyFrameBlend  (catalogue: specified/REA, port level 0)
- `void (RpHAnimKeyFrame* out, const RpHAnimKeyFrame* a, RpHAnimKeyFrame* b, f32 alpha)`: as 0x7c5ca0 with `u = alpha` directly:
  `dot = b.y*a.y + b.x*a.x + b.z*a.z + b.w*a.w` (0x7c60cf..0x7c60eb), `out.t = (b.t - a.t)*alpha + a.t` (0x7c60ed..0x7c611a), in-place
  negation of `b` when `dot < 0` (0x7c612a..0x7c6149), `sa = 1.0 - alpha` (0x7c614c), the same 0.999 test / acos / sin weights
  (0x7c615a..0x7c646e), and `out.k = alpha'*b.k + sa'*a.k` for y, z, w (0x7c6483..0x7c64bb).
- Confidence: the acos/sin block was compared by structure with 0x7c5ca0 (same constants), not re-traced line by line.
- librw: absent.

