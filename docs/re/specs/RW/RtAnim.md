# RenderWare 3.6 RtAnim (animation toolkit, engine plugin id 0x1b7) - full specs, writer C (RE-SPEC-RW-2)

Statically linked RW 3.6 toolkit code in gta_sa.exe 1.0 US. gta-reversed has thunks for these (RenderWare/rw/rtanim.cpp)
and hooked code only for RtAnimInterpolatorSetCurrentAnim (see RtAnim.deviations.md). All functions are `__cdecl`.
`G` = `RwEngineInstance` (0xc97b24); `G+0x134 rwmalloc(size, hint)`, `G+0x138 rwfree(p)`.

## Layouts used (offsets proven by the instruction addresses given)

Globals (.data):
| global | meaning | proof |
|---|---|---|
| `[0xc97900] i32 numSchemes` | number of registered interpolation schemes (max 16) | 0x7ccd48, 0x7ccd81 |
| 0xc9b920 | `RtAnimInterpolatorInfo schemes[16]` (0x30 bytes each) | 0x7ccd5b, 0x7ccd77 |
| `[0xc9bc20] RwFreeList*` | toolkit freelist (0x18-byte entries) | 0x7cccf3 |
| `[0x8dedec] = 0x80`, `[0x8dedf0] = 0x1` | freelist entries per block / blocks to preallocate | 0x7cccd5, 0x7cccd0 |
| 0xc9b8e0 | static RwFreeList storage | 0x7ccce0 |

`RtAnimInterpolatorInfo` (0x30): +0x0 typeID, +0x4 interpKeyFrameSize, +0x8 animKeyFrameSize, +0xc apply, +0x10 blend,
+0x14 interpolate, +0x18 add, +0x1c mulRecip, +0x20 streamRead, +0x24 streamWrite, +0x28 streamGetSize, +0x2c
customDataSize (0x7ccea6, 0x7cceb0; RtAnim.deviations.md 0x7cd5b5..0x7cd5d6).

`RtAnimAnimation` (0x18 header + key frames + custom data): +0x0 interpInfo (0x7ccedd), +0x4 numFrames (0x7cced7),
+0x8 flags (0x7cced1), +0xc duration f32 (0x7cceda), +0x10 pFrames = anim+0x18 (0x7ccedf), +0x14 customData (0x7ccef5).

`RtAnimInterpolator` (0x4c header + numNodes * maxInterpKeyFrameSize): +0x0 pCurrentAnim, +0x4 currentTime f32, +0x8
pNextFrame, +0xc pAnimCallBack, +0x10 pAnimCallBackData, +0x14 animCallBackTime f32, +0x18 pAnimLoopCallBack, +0x1c
pAnimLoopCallBackData, +0x20 maxInterpKeyFrameSize, +0x24 currentInterpKeyFrameSize, +0x28 currentAnimKeyFrameSize,
+0x2c numNodes, +0x30 isSubInterpolator, +0x34 offsetInParent, +0x38 parentAnimation, +0x3c keyFrameApplyCB, +0x40
keyFrameBlendCB, +0x44 keyFrameInterpolateCB, +0x48 keyFrameAddCB, +0x4c interpolated frames (all stores in 0x7cd547..0x7cd587).

## 0x7CCCA0 RtAnimInitialize  (catalogue: specified/REA, port level 3)
- `RwBool (void)`: `r = RwEngineRegisterPlugin(0, 0x1b7, Open=0x7cccd0, Close=0x7ccd10)` (0x7cccb1); return `r > 0`
  (setg, 0x7cccbd: an offset of exactly 0 also returns 0). Caller App::PluginAttach.
- librw: absent (librw registers interpolators statically).

## 0x7CCCD0 cb_ANIMTOOLKIT_RwEngine_Open  (catalogue: specified/REA, port level 2)
- `void* (void* instance, i32, i32)`: `[0xc9bc20] = RwFreeListCreateAndPreallocateSpace(0x18, [0x8dedec], 4, [0x8dedf0],
  0xc9b8e0, 0x4001b)` (0x7ccceb); NULL -> return NULL (0x7cccfc); else return instance. No counter, no other state.
- librw: absent.

## 0x7CCD40 RtAnimRegisterInterpolationScheme  (catalogue: specified/REA, port level 1)
- Convention: `RwBool (const RtAnimInterpolatorInfo* info)`; local `RwError e` {pluginID, errorCode} at [esp+8].
- Behaviour: `n = [0xc97900]`; if `n >= 16` (0x7ccd4d): `e = {0x1b7, _rwerror(1)}`, `RwErrorSet(&e)`, return 0 (0x7ccdb8..0x7ccdd8).
  Search `schemes[0..n-1]` for `typeID == info->typeID` (0x7ccd60); found -> `e = {0x1b7, _rwerror(0)}`, RwErrorSet, return 0
  (0x7ccd90..0x7ccdb0). Else copy 12 dwords of `*info` into `schemes[n]` (rep movsd, 0x7ccd7e), `[0xc97900] = n + 1`, return 1.
- librw: anim.cpp:AnimInterpolatorInfo::registerInterp differs: stores a pointer (no copy), no duplicate check, asserts on overflow.

## 0x7CCE40 RtAnimAnimationCreate  (catalogue: specified/REA, port level 1)
- Convention: `RtAnimAnimation* (i32 typeID, i32 numFrames, i32 flags, f32 duration)` (args proven at 0x7cce51, 0x7ccea9,
  0x7ccec9, 0x7ccecd).
- Behaviour: linear search of `schemes[0..[0xc97900]-1]` for typeID (0x7cce5a); not found -> `RwErrorSet({0x1b7,
  _rwerror(2)})`, return NULL (0x7cce66..0x7cce86). `info = &schemes[i]` (a redundant NULL test follows, 0x7cce9a).
  `a = G->rwmalloc(info->animKeyFrameSize*numFrames + info->customDataSize + 0x18, 0x3001b)` (0x7ccec3; NOT NULL-checked).
  `a->flags = flags`, `a->numFrames = numFrames`, `a->duration = duration` (dword copy), `a->interpInfo = info`,
  `a->pFrames = a + 0x18` (0x7cced1..0x7ccedf). `a->customData = (info->customDataSize > 0) ? pFrames +
  animKeyFrameSize*numFrames : NULL` (signed test, 0x7ccee8..0x7ccefe). Return a.
- librw: anim.cpp:Animation::create differs: takes the info pointer, NULL-checks the allocation, always sets customData.

## 0x7CCF10 RtAnimAnimationDestroy  (catalogue: specified/REA, port level 0)
- `RwBool (RtAnimAnimation* a)`: `G->rwfree(a)` (0x7ccf1b); return 1. librw: anim.cpp:Animation::destroy matches.

## 0x7CD520 RtAnimInterpolatorCreate  (catalogue: specified/REA, port level 0)
- Convention: `RtAnimInterpolator* (i32 numNodes, i32 maxKeyFrameSize)`.
- Behaviour: `it = G->rwmalloc(numNodes*maxKeyFrameSize + 0x4c, 0x3001b)` (0x7cd53e; NOT NULL-checked). Then in this
  order: numNodes (+0x2c), currentInterpKeyFrameSize (+0x24) = maxKeyFrameSize, maxInterpKeyFrameSize (+0x20) =
  maxKeyFrameSize, pCurrentAnim = 0, pNextFrame = 0, currentTime = 0, pAnimCallBack = 0, animCallBackTime (+0x14) = -1.0f
  (0xbf800000, 0x7cd55e), pAnimCallBackData = 0, loop callback and data = 0, currentAnimKeyFrameSize (+0x28) = -1
  (0x7cd56e), isSubInterpolator = 0, offsetInParent = 0, parentAnimation (+0x38) = it (0x7cd57b), apply/add/blend/
  interpolate callbacks = 0 (0x7cd57e..0x7cd587). Return it. Interpolated frames are not initialised.
- librw: anim.cpp:AnimInterpolator::create differs: NULL-checked, does not set callback time/parent/sub fields.

## 0x7CD590 RtAnimInterpolatorDestroy  (catalogue: specified/REA, port level 0)
- `void (RtAnimInterpolator* it)`: tail jump `jmp [G+0x138]` (rwfree) with the caller's argument (0x7cd596); returns
  whatever rwfree returns. librw: anim.cpp:AnimInterpolator::destroy matches.

## Open questions
- None.
