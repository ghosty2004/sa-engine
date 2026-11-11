# RtAnim - gta-reversed deviation notes, writer C (RE-SPEC-RW-2)

gta-reversed hooks RtAnimInterpolatorSetCurrentAnim with its own C++ (source/game_sa/RenderWare/rw/rtanim.cpp). The
original bytes, for reference: `RwBool (RtAnimInterpolator* it, RtAnimAnimation* anim)` (it = [esp+0x14] after the
pushes, 0x7cd5a7; anim = ebx, 0x7cd5a1), always returns 1 (0x7cd651):
1. `it->pCurrentAnim = anim` (0x7cd5ae), `it->currentTime = 0` (0x7cd5b0).
2. From `info = anim->interpInfo`: currentInterpKeyFrameSize (+0x24) = info+0x4, currentAnimKeyFrameSize (+0x28) = info+0x8,
   apply (+0x3c) = info+0xc, blend (+0x40) = info+0x10, interpolate (+0x44) = info+0x14, add (+0x48) = info+0x18
   (0x7cd5b5..0x7cd5db). No check against maxInterpKeyFrameSize.
3. For `i = 0 .. it->numNodes-1` (re-read, signed, 0x7cd5d3/0x7cd60f): `it->keyFrameInterpolateCB(it + 0x4c +
   i*currentInterpKeyFrameSize, anim->pFrames + i*animKFS, anim->pFrames + (i + numNodes)*animKFS, 0.0f (dword 0),
   anim->customData)` (0x7cd5e0..0x7cd605).
4. UNCONDITIONALLY, for `i < numNodes`: interp frame header `keyFrame1 (+0) = pFrames + i*animKFS`, `keyFrame2 (+4) =
   pFrames + (numNodes + i)*animKFS` (0x7cd62a..0x7cd641).
5. `it->pNextFrame = pFrames + 2*numNodes*animKFS` (0x7cd643..0x7cd656); return 1.

## 0x7CD5A0 RtAnim::RtAnimInterpolatorSetCurrentAnim  (gr_deviations = 4)
- Row 1: `source/game_sa/RenderWare/rw/rtanim.cpp:123` (fix-bugs-runtime)
  `if (!notsa::IsFixBugs() || anim->interpInfo->typeID != rwID_RPANIMBLENDPLUGIN) {`
  - Original bytes: there is no typeID test; the keyFrame1/keyFrame2 header loop at 0x7cd613..0x7cd641 always runs
    (only guarded by `numNodes > 0`, 0x7cd626). With FIX_BUGS enabled gr skips it for RpAnimBlend interpolators.
  - Verdict: CONFIRMED (gr differs only when its FIX_BUGS switch is on; with it off gr matches the bytes).
  - Class: keep-original (the port must always write the headers, corrupting RpHAnimBlendInterpFrame data exactly as the
    original does).
- Row 2: `rtanim.cpp:129` (original-comment) "But R* hasn't really used RW's animation system since III."
  - Original bytes: comment text only; no code corresponds to it.
  - Verdict: REFUTED (not a behavioural difference). Class: harmless.
- Row 3: `rtanim.cpp:130` (original-comment) "In III and VC R* wrote their own interpolated animation data into a dummy"
  - Original bytes: comment only. Verdict: REFUTED. Class: harmless.
- Row 4: `rtanim.cpp:132` (original-comment) "In SA they went a step further and now write into the interpolation frames of"
  - Original bytes: comment only. Verdict: REFUTED. Class: harmless.
- Other gr vs bytes checks (no catalogued row): gr's interpolate call passes `0.0f` and `anim->customData` in the same
  order as 0x7cd5f1..0x7cd604 and sets the same fields in the same order; no further difference found.
- librw: anim.cpp:AnimInterpolator::setCurrentAnim differs: it fails when the frame is larger than the maximum, writes the
  headers before calling interpolate and skips a NULL interpolate callback.
