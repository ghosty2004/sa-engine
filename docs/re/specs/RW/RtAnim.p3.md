# RtAnim (animation toolkit) and the BMP row decoder (RE-SPEC-RW-3 writer B)

All `__cdecl`. Pass-1 file `RtAnim.md` (read-only) has create/destroy/register and the full struct layouts; summary:

## Layout
- `[0xc97900]` i32 number of registered schemes; 0xc9b920 `RtAnimInterpolatorInfo schemes[16]`, 0x30 bytes each
  (`lea eax,[eax+eax*2]; shl eax,4`, 0x7cce2c); `[0xc9bc20]` toolkit freelist (0x7ccd10).
- RtAnimInterpolatorInfo: +0x0 typeID (0x7ccdf9), +0x4 interpKeyFrameSize (0x7cdb0e), +0x8 animKeyFrameSize (0x7cd3a4),
  +0xc apply, +0x10 blend, +0x14 interpolate, +0x18 add (copied 0x7cd6c4-0x7cd6da), +0x1c mulRecip (0x7cdb83), +0x20 streamRead
  (0x7cd3fc), +0x24 streamWrite (0x7cd4b6), +0x28 streamGetSize (0x7cd4d7), +0x2c customDataSize (0x7cd3a1).
- RtAnimAnimation: +0x0 interpInfo, +0x4 numFrames, +0x8 flags, +0xc duration f32, +0x10 pFrames (= anim+0x18), +0x14 customData
  (stores 0x7cd3c6-0x7cd3eb).
- Key frame header (all schemes): +0x0 prevFrame (pointer), +0x4 time f32 (0x7cd9db-0x7cd9dd, 0x7cd836).
- RtAnimInterpolator: +0x0 pCurrentAnim, +0x4 currentTime f32, +0x8 pNextFrame, +0xc animCallBack, +0x10 its data, +0x14
  animCallBackTime f32, +0x18 loopCallBack, +0x1c its data, +0x20 maxInterpKeyFrameSize, +0x24 currentInterpKeyFrameSize (node
  stride), +0x28 currentAnimKeyFrameSize, +0x2c numNodes, +0x30 isSubInterpolator, +0x34 offsetInParent, +0x38 parent, +0x3c..+0x48
  apply/blend/interpolate/add callbacks, +0x4c node array. Interpolated node: +0x0 keyFrame1, +0x4 keyFrame2 (0x7cda01-0x7cda0e).
- Error reports in this file use `RwErrorSet({pluginID, _rwerror(code)})`: RtAnim uses pluginID 0x1b7 with code 2 (0x7cce05);
  the BMP decoder pluginID 0x19d with code 0x80000008 (0x7ce8a2).
- `[0x858b50] = 0.0f` (compared in 0x7cd8d4, 0x7cdabf).

## 0x7ccd10 cb_ANIMTOOLKIT_RwEngine_Close  (catalogue: specified/REA, port level 1)
- `void* (void* instance, RwInt32, RwInt32)`: `[0xc97900] = 0` (forgets all schemes, 0x7ccd15); if `[0xc9bc20]` != NULL
  `RwFreeListDestroy` it and store NULL. Returns instance.
- librw: absent.

## 0x7ccde0 RtAnimGetInterpolatorInfo  (catalogue: specified/REA, port level 1)
- `RtAnimInterpolatorInfo* (RwInt32 typeID)`: linear search i = 0 while i < `[0xc97900]` (signed `jle`/`jl`) for `schemes[i].typeID ==
  typeID`; found -> `&schemes[i]`. Not found -> `RwErrorSet({0x1b7, _rwerror(2)})`, NULL.
- librw: anim.cpp `findAnimInterpolatorInfo` matches (librw sets no error).

## 0x7ccf30 RtAnimAnimationRead  (catalogue: specified/REA, port level 4)
- `RtAnimAnimation* (const RwChar* filename)`. `s = RwStreamOpen(2 rwSTREAMFILENAME, 1 rwSTREAMREAD, filename)` 0x7ecef0; NULL ->
  NULL. `RwStreamFindChunk(s, 0x1b rwID_ANIMANIMATION, NULL, NULL)`; 0 -> return NULL WITHOUT closing the stream (0x7ccf65; file
  handle leak, original bug).
- Then the body of `RtAnimAnimationStreamRead` 0x7cd220 inlined (same order, same checks, same allocation; 0x7ccf6c-0x7cd126); every
  path afterwards ends at `RwStreamClose(s, NULL)` 0x7ece20 (0x7cd133) and returns the animation or NULL.
- librw: anim.cpp absent as file reader (librw has only `streamRead`).

## 0x7cd160 RtAnimAnimationWrite  (catalogue: specified/REA, port level 3)
- `RwBool (const RtAnimAnimation* a, const RwChar* filename)`: `s = RwStreamOpen(2, 2 rwSTREAMWRITE, filename)`; NULL -> 0. Then the
  body of 0x7cd410 inlined (header result ignored; the stop-at-first-failure chain), `RwStreamClose(s, NULL)`, return 1 (even if a
  write failed).
- librw: absent.

## 0x7cd220 RtAnimAnimationStreamRead  (catalogue: specified/REA, port level 3)
- `RtAnimAnimation* (RwStream* s)` (no chunk header read here). `RwStreamReadInt32(s, &ver, 4)` 0x7ed540; 0 -> NULL; `ver != 0x100`
  -> NULL (0x7cd247, no error). Read typeID (int32). Look up the scheme (inline search, not found -> error {0x1b7, 2}, NULL).
  Read numFrames (int32), flags (int32), duration (`RwStreamReadReal` 0x7ed4f0); any 0 -> NULL.
- Second inline lookup of the same scheme (0x7cd34b); then `a = memoryFuncs.rwmalloc (+0x134)(info.animKeyFrameSize * numFrames +
  0x18 + info.customDataSize, 0x3001b)` (0x7cd3a4-0x7cd3b7); NO NULL check. `a+0xc = duration`, `a+4 = numFrames`, `a+8 = flags`,
  `a+0 = info`, `a+0x10 = a+0x18`; `a+0x14 = a+0x18 + animKeyFrameSize*numFrames` if `customDataSize > 0` (signed) else NULL
  (0x7cd3d7-0x7cd3eb). `info.streamRead (+0x20)(s, a)` (result ignored, 0x7cd3fc). Returns a.
- librw: anim.cpp `Animation::streamRead` differs: librw uses `Animation::create`, does not check read results and does not
  report an unknown scheme.

## 0x7cd410 RtAnimAnimationStreamWrite  (catalogue: specified/REA, port level 3)
- `RwBool (const RtAnimAnimation* a, RwStream* s)`. `size = info.streamGetSize(+0x28)(a) + 0x14`;
  `_rwStreamWriteVersionedChunkHeader(s, 0x1b, size, 0x36003, 0xffff)` (result IGNORED, 0x7cd43a).
- `RwStreamWriteInt32` 0x7ed460 of: 0x100 (version), `*a->interpInfo` (= typeID, written through the info pointer, 0x7cd457), a+4
  numFrames, a+8 flags; `RwStreamWriteReal` 0x7ed3d0 of a+0xc duration. The first failing write returns 0.
- `info.streamWrite (+0x24)(a, s)` (result ignored); return 1.
- librw: anim.cpp `Animation::streamWrite` matches the layout.

## 0x7cd4d0 RtAnimAnimationStreamGetSize  (catalogue: specified/REA, port level 0)
- `RwInt32 (const RtAnimAnimation* a)`: `info.streamGetSize(+0x28)(a) + 0x14` (0x7cd4dd; 0x14 = 4 dwords + real, no chunk header).
- librw: anim.cpp `Animation::streamGetSize` matches.

## 0x7cd4f0 RtAnimAnimationGetNumNodes  (catalogue: specified/REA, port level 0)
- `RwInt32 (const RtAnimAnimation* a)`: `f0 = a+0x10`, stride = info+0x8. n = 0; `p = f0`; while `p->prevFrame (+0) != f0`:
  `p += stride; n++` (0x7cd502-0x7cd50e). Returns the index of the first key frame whose prevFrame is the first key frame (= the
  number of initial frames, one per node). No bound by numFrames.
- librw: anim.cpp `Animation::getNumNodes` matches.

## 0x7cd660 RtAnimInterpolatorSetKeyFrameCallBacks  (catalogue: specified/REA, port level 1)
- `RwBool (RtAnimInterpolator* it, RwInt32 typeID)`: inline scheme lookup (error {0x1b7, 2} and return 0 if absent). Copies info
  +0xc..+0x18 (apply, blend, interpolate, add) to it+0x3c..+0x48 (0x7cd6c4-0x7cd6da). Returns 1.
- librw: absent (librw reads the callbacks from the animation's interpInfo).

## 0x7cd6f0 RtAnimInterpolatorSetAnimLoopCallBack  (catalogue: specified/REA, port level 0)
- `(it, cb, data)`: it+0x18 = cb, it+0x1c = data. EAX = it (never overwritten).
- librw: absent.

## 0x7cd710 RtAnimInterpolatorSetAnimCallBack  (catalogue: specified/REA, port level 0)
- `(it, cb, RwReal time, data)`: it+0xc = cb, it+0x14 = time (raw dword move), it+0x10 = data. EAX = it.
- librw: absent.

## 0x7cd730 RtAnimInterpolatorCopy  (catalogue: specified/REA, port level 0)
- `RwBool (RtAnimInterpolator* out, RtAnimInterpolator* in)`: `memcpy(out+0x4c, in+0x4c, out.numNodes(+0x2c) *
  out.currentInterpKeyFrameSize(+0x24))` (`rep movsd` + `rep movsb`, 0x7cd73a-0x7cd758). Returns 1. Sizes taken from `out` only.
- librw: absent.

## 0x7cd760 RtAnimInterpolatorSubAnimTime  (catalogue: specified/REA, port level 2)
- `RwBool (RtAnimInterpolator* it, RwReal dt)`. Returns 1. x87, f32 operands. `looped = 0`.
- Callback trigger `cbHit` (0x7cd76e-0x7cd795): `cur > cbTime` (it+4 vs it+0x14; equal/less/NaN -> 0) AND `(cur - dt) <= cbTime`
  (`test ah,0x41; jp`: exactly one of C0/C3 set -> 1; NaN -> 0).
- `t = cur - dt` (0x7cd797), `it+4 = (f32)t` (`fst`, 0x7cd7a3). If `t < 0.0f [0x858b50]` (only C0 set, 0x7cd7b1) -> WRAP:
  `t = (f32)it+4`; repeat `t += anim.duration` (x87, not rounded between iterations) while `t < 0` (0x7cd7bd-0x7cd7cc); store t to a
  local and to it+4; `RtAnimInterpolatorSetCurrentAnim(it, anim)` 0x7cd5a0; `RtAnimInterpolatorAddAnimTime(it, t)` 0x7cd8d0 (with the
  local f32 t); `looped = 1`; `cbHit |= (it+4 < cbTime)` (0x7cd7e7-0x7cd802; NaN -> 0).
- NO WRAP (t >= 0 or NaN, 0x7cd812): for each node (count it+0x2c, `je` on 0 only): while `cur < kf1.time` (`kf1 = node+0`; exits
  when cur >= time or NaN): `kf2 = kf1; kf1 = kf1->prevFrame` (0x7cd82f-0x7cd838). Then `it.interpolate(+0x44)(node, kf1, kf2,
  (f32)cur, anim.customData)` (0x7cd85c). After all nodes `it+8 pNextFrame = NULL` (0x7cd86b).
- Then (0x7cd873): if cbHit and it+0xc != NULL: `r = it+0xc(it, it+0x10)`; r == NULL -> it+0xc = NULL. If looped and it+0x18 != NULL:
  `r = it+0x18(it, it+0x1c)`; r == NULL -> it+0x18 = NULL.
- librw: absent (librw anim.cpp has only `addTime` / `setCurrentAnim`).

## 0x7cd8d0 RtAnimInterpolatorAddAnimTime  (catalogue: specified/REA, port level 1)
- `RwBool (RtAnimInterpolator* it, RwReal dt)`. Returns 1 always.
- `dt <= 0.0f` or NaN (C0|C3 set, 0x7cd8e0) -> return 1 immediately.
- Locals: anim = it+0, dur = anim+0xc, numNodes = it+0x2c, looped = 0, cbT = it+0x14.
- cbHit (0x7cd8f7-0x7cd942): `cur < cbT` (C0 only; cur >= cbT or NaN -> 0) AND NOT `(dt + cur) < cbT` (sum computed `fld dt; fadd
  cur`, then `fcomp cbT`, C0 test: NaN -> hit).
- `t = dt + cur` (0x7cd942), `it+4 = (f32)t` (`fst`). If `t > dur` (no C0/C3, 0x7cd954): looped = 1; loop 0x7cd967: `t -= dur`
  (x87 extended, not rounded per iteration); `cbHit |= !(t < cbT)`; repeat while `t > dur`. Then `it+4 = (f32)t`;
  `RtAnimInterpolatorSetCurrentAnim(it, anim)`; `it+4 = (f32)t` again (keeps the time across the reset, 0x7cd9a2).
- If it+8 pNextFrame == NULL (0x7cd9af): `SetCurrentAnim(it, anim)` with it+4 saved/restored around the call.
- Frame advance (0x7cd9c3-0x7cda20): `end = anim.pFrames + anim.numFrames * it+0x28`; node search index c = 0 (persists across
  frames). While `it+8 < end` (unsigned): `nf = it+8; prev = nf->prevFrame`; if `prev.time > cur` (strict, flags clear) stop.
  Else find the node whose keyFrame2 == prev, scanning cyclically from c (`c++`, wrap to 0 at numNodes; no termination if none
  matches, 0x7cd9ef-0x7cda04); `node.kf1 = node.kf2; node.kf2 = nf`; `it+8 += it+0x28`.
- Interpolate every node i < numNodes (signed): `it.interpolate(+0x44)(node, node.kf1, node.kf2, (f32)it+4, anim.customData)`
  (0x7cda42).
- Callbacks: if cbHit and it+0xc: call (it, it+0x10), NULL result clears it+0xc. If looped and it+0x18: call (it, it+0x1c), NULL
  clears it+0x18.
- librw: anim.cpp `AnimInterpolator::addTime` differs: no callbacks; on `t > duration` librw only calls `setCurrentAnim` (restarts at
  time 0, no wrap-around subtraction); searches the node from index 0 each time; no NULL-nextFrame re-init.

## 0x7cdab0 RtAnimInterpolatorSetCurrentTime  (catalogue: specified/REA, port level 3)
- `RwBool (RtAnimInterpolator* it, RwReal time)`: `d = (f32)(time - it+4)` (stored back into the argument, 0x7cdabb); compare with
  0.0f [0x858b50]: `d < 0` -> `RtAnimInterpolatorSubAnimTime(it, -d)` (`fchs`, 0x7cdad7); else (including NaN) ->
  `RtAnimInterpolatorAddAnimTime(it, d)`. Returns 1.
- librw: absent.

## 0x7cdb00 RtAnimAnimationMakeDelta  (catalogue: specified/REA, port level 3)
- `RwBool (RtAnimAnimation* a, RwInt32 numNodes, RwReal time)`. `it = RtAnimInterpolatorCreate(numNodes, info.interpKeyFrameSize)`
  0x7cd520; NULL -> 0. `SetCurrentAnim(it, a)`; `d = (f32)(time - it+4)`; `d < 0` -> SubAnimTime(it, -d) else AddAnimTime(it, d)
  (0x7cdb3a-0x7cdb5f).
- For node i = 0 while i < numNodes (signed; frame pointer f = a.pFrames + i * it+0x28, node = it+0x4c + i * it+0x24):
  loop: `info.mulRecip(+0x1c)(f, node)` (0x7cdb83); if `f.time == a.duration` (C3 only, `test ah,0x44; jnp`) stop this node; else
  find the next frame of the chain: `g = f`; if `f.prevFrame != f`: scan `g += it+0x28` until `g.prevFrame == f` (unbounded);
  `f = g` and repeat. (NaN times never stop.)
- `memoryFuncs.free (+0x138)(it)` (0x7cdbd2). Returns 1.
- librw: absent.

## 0x7cdbf0 RtAnimInterpolatorBlend  (catalogue: specified/REA, port level 0)
- `RwBool (out, in1, in2, RwReal alpha)`: for i = 0 while i < out.numNodes (re-read each pass): `out.blend(+0x40)(out.node[i],
  in1.node[i], in2.node[i], alpha)` with each interpolator's own stride (+0x24) (0x7cdc09-0x7cdc2f). Returns 1.
- librw: absent.

## 0x7cdc50 RtAnimInterpolatorAddTogether  (catalogue: specified/REA, port level 0)
- `RwBool (out, in1, in2)`: same loop with `out.add(+0x48)(out.node[i], in1.node[i], in2.node[i])` (0x7cdc8a). Returns 1.
- librw: absent.

## 0x7cdcb0 RtAnimInterpolatorCreateSubInterpolator  (catalogue: specified/REA, port level 1)
- `RtAnimInterpolator* (RtAnimInterpolator* parent, RwInt32 startNode, RwInt32 numNodes, RwInt32 maxKeySize)`: if `maxKeySize == -1`
  use parent+0x20. `sub = RtAnimInterpolatorCreate(numNodes, maxKeySize)`; NO NULL check: `sub+0x38 = parent`, `sub+0x34 = startNode`,
  `sub+0x30 = 1` (0x7cdcd3-0x7cdcd9). Returns sub.
- librw: absent.

## 0x7cdcf0 RtAnimInterpolatorBlendSubInterpolator  (catalogue: specified/REA, port level 1)
- `RwBool (out, in1, in2, RwReal alpha)`; returns 1. Strides: each interpolator's +0x24; blend = out+0x40.
- Case A `out+0x30 == 1` (sub output): o = out+0x34; for k = 0 while o+k < o + out.numNodes: `blend(out[k], in1[o + k - in1+0x34],
  in2[o + k - in2+0x34], alpha)` (call 0x7cdd6f).
- Case B `in1+0x30 == 1`: `memcpy(out[0], in2[0], in1.off * out.stride)` (0x7cddae); for j = in1.off while j < in1.off + in1.numNodes:
  `blend(out[j], in1[j - in1.off], in2[j], alpha)` (0x7cdded); then `e = in1.off + in1.numNodes`; `memcpy(out + e*out.stride,
  in2 + e*in2.stride, (out.numNodes - in1.numNodes - in1.off) * out.stride)` (0x7cde30).
- Case C (otherwise; in2 is the sub): `memcpy(out[0], in1[0], in2.off * out.stride)` (0x7cde56); `blend(out[j], in1[j], in2[j - in2.off],
  alpha)` for j in [in2.off, in2.off + in2.numNodes) (0x7cde95); tail `memcpy` from in1 at `e = in2.off + in2.numNodes`,
  `(out.numNodes - in2.numNodes - in2.off) * out.stride` bytes (0x7cded8).
- All copy byte counts use out's stride; a negative count would be a huge unsigned copy (no check).
- librw: absent.

## 0x7cdef0 RtAnimInterpolatorAddSubInterpolator  (catalogue: specified/REA, port level 0)
- `RwBool (out, main, sub)`: `oo = (out+0x30 == 0) ? sub+0x34 : 0` (0x7cdf02-0x7cdf0a). For i = 0 while i < out.numNodes:
  `out.add(+0x48)(out[i + oo], main[sub+0x34 + i], sub[i])` (0x7cdf41). Returns 1. The loop bound is out's node count even when out
  is the full interpolator (reads past sub's nodes).
- librw: absent.

## 0x7ce5d0 sub_7ce5d0 (proposed: _rtBMPImageReadRow)  (catalogue: unknown/REA, port level 1)
- Evidence: only caller `RtBMPImageRead` 0x7cdf60; error pluginID 0x19d (BMP toolkit) 0x7ce8a7; converts one BMP scan line into an
  RwImage row.
- `RwBool (RwImage* img, RwInt32 y, const RwUInt8* src, RwInt32 bmpBits, RwBool noSwap)`. Image fields: +0x4 width (pixel count),
  +0xc depth, +0x10 stride, +0x14 cpPixels, +0x18 palette (0x7ce5db-0x7ce5f0). `dst = cpPixels + y*stride` (0x7ce5e2-0x7ce5f8).
- Dispatch on `key = depth | (bmpBits << 8)` (0x7ce5f3-0x7ce5f6):
  - 0x808: copy `width` bytes (dword copy then remainder). Return 1.
  - 0x104 / 0x108 (1-bit into 4/8-bit image): one byte per pixel = bit (MSB first, shift 7..0, next source byte when the shift goes
    negative).
  - 0x120 (1-bit into 32-bit): `*(u32*)dst = palette[bit]` (0x7ce668), MSB first.
  - 0x404 / 0x408 (4-bit into 4/8-bit): one byte per pixel = nibble, high nibble first (shift 4 then 0).
  - 0x420: `palette[nibble]`, high nibble first. 0x820: `palette[byte]`.
  - 0x1020 (16-bit 565 into 32-bit): `v = u16 src`: R = (v >> 8) & 0xf8, G = (v >> 3) & 0xfc, B = (u8)(v << 3), A = 0xff.
  - 0xf20 (15-bit): R = (v >> 7) & 0xf8, G = (v >> 2) & 0xf8, B = (u8)(v << 3), A = 0xff.
  - 0x1820 (24-bit): noSwap != 0 (0x7ce913): dst = {s0, s1, s2, 0xff}; noSwap == 0: dst = {s2, s1, s0, 0xff} (BGR -> RGB,
    0x7ce963-0x7ce977). Source stride 3.
  - 0x2020 (32-bit): dst = {s0, s1, s2, 0xff}, source stride 4 (source alpha dropped, NO swap regardless of noSwap, 0x7ce8e5-0x7ce8fc).
  - anything else: `RwErrorSet({0x19d, _rwerror(0x80000008)})`, return 0 (0x7ce8a2).
- Width 0 returns 1 for every format. The 1/4-bit loops pre-load the next source byte after the last pixel when the shift wraps
  (reads up to 1 byte past the packed row; harmless for padded BMP rows).
- librw: bmp.cpp `readBMP` differs: librw decodes inline into 8/16/24/32-bit images, swaps BGR for 24 and 32 bit and keeps the
  32-bit alpha unless `noalpha`; no 1/4/15/16-bit palette expansion as here.
