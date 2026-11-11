# RenderWare 3.6 RpHAnim (hierarchical animation plugin, id 0x11e) - full specs, writer C (RE-SPEC-RW-2)

Statically linked RW 3.6 plugin code in gta_sa.exe 1.0 US. gta-reversed has only `plugin::Call*` thunks
(RenderWare/rw/rphanim.cpp), so everything below comes from the bytes. All functions are `__cdecl` (caller pops).
`G` = `RwEngineInstance` (dword at 0xc97b24). Allocation hooks: `G+0x134 rwmalloc(size, hint)`, `G+0x138 rwfree(p)`,
`G+0x144 freeListAlloc(fl, hint)`, `G+0x148 freeListFree(fl, p)` (see RwMemory.md / RwFrame.md).

## Layouts used (offsets proven by the instruction addresses given)

Globals:
- `[0xc9b8c8] i32 hAnimFrameOffset`: frame extension offset, written by RpHAnimPluginAttach 0x7c464c.
- `[0xc9b8cc] RwFreeList* hAnimHierarchyFreeList` (blocks of 0x24 bytes), written 0x7c4697.
- `[0x8decd8] i32` freelist entries per block, .data init `[0x8decd8] = 0x80` (read 0x7c4675); `[0x8decdc] i32` blocks to
  preallocate, `[0x8decdc] = 0x1` (read 0x7c4670). Both set by RpHAnimHierarchySetFreeListCreateParams 0x7c45e0.
- 0xc97054: static in-place RwFreeList storage passed to the freelist create (0x7c4684).
- Constant `[0x858624] = 1.0f` (used by the inlined key-frame apply, 0x7c5470).

Frame extension `RpHAnimFrameExt` (8 bytes, at `frame + [0xc9b8c8]`):
| off | field | proof |
|---|---|---|
| +0x0 | `i32 id` (node id; -1 = none) | ctor 0x7c4763, stream read 0x7c4a4f |
| +0x4 | `RpHAnimHierarchy* hierarchy` | ctor 0x7c475c, dtor 0x7c4781, get 0x7c516a |

`RpHAnimHierarchy` (0x24 bytes, freelist [0xc9b8cc]):
| off | field | proof |
|---|---|---|
| +0x00 | `i32 flags` (0x1 SUBHIERARCHY, 0x2 NOMATRICES, 0x1000 UPDATEMODELLINGMATRICES, 0x2000 UPDATELTMS, 0x4000 LOCALSPACEMATRICES) | 0x7c4c69, 0x7c51e1 |
| +0x04 | `i32 numNodes` | 0x7c4c6b, 0x7c55ec |
| +0x08 | `RwMatrix* pMatrixArray` (16-byte aligned) | 0x7c4c99, 0x7c57de |
| +0x0c | `void* pMatrixArrayUnaligned` | 0x7c4c96, freed 0x7c47bb |
| +0x10 | `RpHAnimNodeInfo* pNodeInfo` | 0x7c4cc2, 0x7c52d7 |
| +0x14 | `RwFrame* parentFrame` | 0x7c4c6e, 0x7c51f1 |
| +0x18 | `RpHAnimHierarchy* parentHierarchy` | 0x7c4d1e, 0x7c5634 |
| +0x1c | `i32 rootParentOffset` (-1 = none) | 0x7c562c |
| +0x20 | `RtAnimInterpolator* currentAnim` | 0x7c4c5b, 0x7c52b1 |

`RpHAnimNodeInfo` (0x10 bytes): +0x0 `nodeID` (0x7c4cf7), +0x4 `nodeIndex` (0x7c4cfd), +0x8 `flags` (0x1 POPPARENTMATRIX,
0x2 PUSHPARENTMATRIX; 0x7c4d0e, 0x7c55a0), +0xc `RwFrame* pFrame` (0x7c4ce1, 0x7c5548).

`RtAnimInterpolator` fields used here (full layout in RtAnim.md): +0x20 maxInterpKeyFrameSize (0x7c485a), +0x24
currentInterpKeyFrameSize (0x7c52bc), +0x3c keyFrameApplyCB (0x7c52bf), +0x4c first interpolated key frame (0x7c52c6).
HAnim interpolated key frame (`RpHAnimInterpFrame`, 0x24 bytes): +0x0 keyFrame1, +0x4 keyFrame2, +0x8 quat imag.x,
+0xc imag.y, +0x10 imag.z, +0x14 real w, +0x18/+0x1c/+0x20 translation (0x7c53fb..0x7c54f6).

`RwFrame` fields: +0x3 privateFlags, +0x4 parent, +0x8/+0xc inDirtyListLink, +0x10 modelling, +0x50 ltm, +0xa0 root
(see RwFrame.md). `G+0xbc` = dirty frame list head.

## 0x7C4600 RpHAnimPluginAttach  (catalogue: specified/REA, port level 3)
- Convention: `RwBool (void)`; caller App::PluginAttach 0x53d870.
- Behaviour: 1. `RwEngineRegisterPlugin(0, 0x11e, Open=0x7c4670, Close=0x7c4720)` (0x7c4611); `< 0` -> return 0 (0x7c461b).
  2. `[0xc9b8c8] = RwFrameRegisterPlugin(8, 0x11e, Ctor=0x7c4750, Dtor=0x7c4770, Copy=0x7c4830)` (0x7c4633, stored 0x7c464c
  after the next pushes; not tested yet). 3. `r = RwFrameRegisterPluginStream(0x11e, Read=0x7c4a00, Write=0x7c48d0,
  GetSize=0x7c4bf0)` (0x7c4651). 4. return 0 if `r < 0` (0x7c465b) or `[0xc9b8c8] < 0` (0x7c4664); else 1.
- librw: hanim.cpp:registerHAnimPlugin matches in substance (librw stores the offset first; same 3 registrations).

## 0x7C4670 cb_HANIMPLUGIN_RwEngine_Open  (catalogue: specified/REA, port level 2)
- Convention: `void* (void* instance, i32 offset, i32 size)`; local `RtAnimInterpolatorInfo info` (0x30 bytes at esp+4).
- Behaviour: 1. `[0xc9b8cc] = RwFreeListCreateAndPreallocateSpace(0x24, [0x8decd8], 4, [0x8decdc], 0xc97054, 0x4011e)`
  (args pushed 0x7c467f..0x7c468d, call 0x7c468f). 2. result NULL -> the return value becomes NULL (0x7c46a0) but the
  function CONTINUES and still registers the scheme; else return value = instance (0x7c46a4).
  3. Fill info: typeID = 1 (0x7c46ac), interpKeyFrameSize = 0x24 (0x7c46b5), animKeyFrameSize = 0x24 (0x7c46bd),
  apply = 0x7c5b80, blend = 0x7c60c0, interpolate = 0x7c5ca0, add = 0x7c6720, mulRecip = 0x7c65c0, streamRead = 0x7c64c0,
  streamWrite = 0x7c6540, streamGetSize = 0x7c65b0, customDataSize = 0 (0x7c4705).
  4. `RtAnimRegisterInterpolationScheme(&info)` (0x7c470d); its result is ignored. 5. return (NULL or instance).
- librw: hanim.cpp:hanimOpen differs: librw has no freelist; scheme registration equivalent (librw uses its own sizes).

## 0x7C4750 cb_HANIMPLUGIN_RwFrame_Ctor  (catalogue: specified/REA, port level 0)
- `void* (RwFrame* f, i32 offset, i32 size)`; uses the global offset. `ext = f + [0xc9b8c8]`; `ext->hierarchy = NULL`
  (0x7c475c); `ext->id = -1` (0x7c4763); return f.
- librw: hanim.cpp:createHAnim matches (id -1, hierarchy nil).

## 0x7C4770 cb_HANIMPLUGIN_RwFrame_Dtor  (catalogue: specified/REA, port level 1)
- `void* (RwFrame* f, i32 offset, i32 size)`; returns f.
- Behaviour: `ext = f + [0xc9b8c8]` (0x7c477b); `h = ext->hierarchy` (0x7c4781).
  1. `h == NULL`: `ext->id = -1` (0x7c481f), return f.
  2. For `i = 0 .. h->numNodes-1` (numNodes re-read every iteration, 0x7c47a0): `h->pNodeInfo[i].pFrame = NULL` (0x7c479c).
  3. If `h->parentFrame != f` (0x7c47ad): only `ext->hierarchy = NULL` (0x7c481b), `ext->id = -1`; return f
     (the hierarchy stays alive, owned by its parent frame).
  4. Else (this frame owns it): if `!(h->flags & 1)` (not a sub-hierarchy, 0x7c47b3): if `!(h->flags & 2)`:
     `G->rwfree(h->pMatrixArrayUnaligned)` (0x7c47c5); then `G->rwfree(h->pNodeInfo)` (0x7c47d8). In every case:
     `h->pMatrixArrayUnaligned = NULL; h->pMatrixArray = NULL; h->pNodeInfo = NULL` (0x7c47e4..0x7c47eb);
     `RtAnimInterpolatorDestroy(h->currentAnim)` (0x7c47ee); `G->freeListFree([0xc9b8cc], h)` (0x7c4800);
     `*(RpHAnimHierarchy**)(h_parentFrame + [0xc9b8c8] + 4) = NULL` (0x7c4813, the parentFrame read before the free,
     always == f here); then `ext->hierarchy = NULL`, `ext->id = -1`; return f.
- Edge: a sub-hierarchy (flags & 1) shares its arrays with the parent and only the struct and interpolator are freed.
- librw: hanim.cpp:destroyHAnim differs: librw's HAnimHierarchy::destroy frees matrices/nodeInfo unconditionally (no
  SUB/NOMATRICES tests) and uses rwFree instead of the freelist; node pFrame clearing and id = -1 match.

## 0x7C4830 cb_HANIMPLUGIN_RwFrame_Copy  (catalogue: specified/REA, port level 2)
- `void* (RwFrame* dst, RwFrame* src, i32 offset, i32 size)`; returns dst.
- Behaviour: `d = dst + [0xc9b8c8]`, `s = src + [0xc9b8c8]` (0x7c4840/0x7c4843). `d->id = s->id` (0x7c4848).
  `hs = s->hierarchy`; if `hs == NULL` or `hs->flags & 1` (0x7c4853) -> return dst (d->hierarchy untouched).
  Else `hn = RpHAnimHierarchyCreate(hs->numNodes, NULL, NULL, hs->flags, hs->currentAnim->maxInterpKeyFrameSize)`
  (args 0x7c4857..0x7c4866; NULL result is NOT checked: 0x7c486c dereferences it). For `i < hn->numNodes` (re-read):
  `hn->pNodeInfo[i].pFrame = NULL` (0x7c487f), `.flags = hs.flags` (0x7c4891), `.nodeIndex = hs.nodeIndex` (0x7c489f),
  `.nodeID = hs.nodeID` (0x7c48ac). Then `d->hierarchy = hn` (0x7c48be), `hn->parentFrame = dst` (0x7c48c1).
- librw: hanim.cpp:copyHAnim differs: librw sets `dst->hierarchy = nil` first; the exe leaves the destination pointer
  untouched when the source has no (or a sub-) hierarchy. Field copies match.

## 0x7C4A00 cb_HANIMPLUGIN_RwFrame_StreamRead  (catalogue: specified/REA, port level 3)
- Convention: `RwStream* (RwStream* s, i32 length, RwFrame* f, i32 offset, i32 size)`; returns s or NULL. Locals:
  version (re-uses the `s` argument slot, 0x7c4a14), numNodes (re-uses the `length` slot, 0x7c4a63), flags [esp+0x14],
  keyFrameSize [esp+0x10], ext pointer [esp+0x18].
- Behaviour (every read is `RwStreamReadInt32(s, p, 4)`; a NULL result returns 0):
  1. read version (0x7c4a23); `!= 0x100` -> return 0 (0x7c4a37).
  2. read `ext->id` directly into the extension (0x7c4a4f). 3. read numNodes (0x7c4a6b).
  4. `numNodes <= 0` -> return s (0x7c4a85; ext->hierarchy untouched).
  5. read flags (0x7c4a93), read maxKeyFrameSize (0x7c4aaf).
  6. `h = G->freeListAlloc([0xc9b8cc], 0x3011e)` (0x7c4ad6); `memset(h, 0, 0x24)` (rep stosd, 9 dwords, 0x7c4ae7) BEFORE
     the NULL test (NULL -> access violation in the original); `h == NULL` -> `ext->hierarchy = NULL`, return s (0x7c4bd1).
  7. `h->currentAnim = RtAnimInterpolatorCreate(numNodes, maxKeyFrameSize)` (0x7c4b00/0x7c4b05), `h->flags = flags`
     (0x7c4b0c), `h->parentFrame = f` (0x7c4b0e), `h->numNodes = numNodes` (0x7c4b1a), `h->parentHierarchy = h` (0x7c4b1d).
     rootParentOffset stays 0 from the memset.
  8. `flags & 2`: matrices NULL (0x7c4b22/0x7c4b25). Else `p = G->rwmalloc((numNodes << 6) + 0xf, 0x3011e)` (0x7c4b3f),
     `h->pMatrixArrayUnaligned = p`, `h->pMatrixArray = (p + 0xf) & ~0xf` (0x7c4b4b..0x7c4b51); no NULL check.
  9. `h->pNodeInfo = G->rwmalloc(numNodes << 4, 0x3011e)` (0x7c4b66); no NULL check.
  10. For `i = 0 .. h->numNodes-1`: read nodeID (+0, 0x7c4b81), nodeIndex (+4, 0x7c4b98), flags (+8, 0x7c4baf); set pFrame
      = NULL (0x7c4bbf). Any failed read -> return 0 (0x7c4abb) leaking h (ext->hierarchy not set).
  11. `ext->hierarchy = h` (0x7c4bd5); return s.
- librw: hanim.cpp:readHAnim differs: librw only asserts the version (exe returns NULL), treats numNodes != 0 (exe: > 0),
  ignores the stored nodeIndex (exe stores it), builds via HAnimHierarchy::create with temp arrays.

## 0x7C4C30 RpHAnimHierarchyCreate  (catalogue: specified/REA, port level 1)
- Convention: `RpHAnimHierarchy* (i32 numNodes, u32* nodeFlags, i32* nodeIDs, i32 flags, i32 maxKeyFrameSize)`
  (stack args +4..+0x14; offsets proven at 0x7c4c4a, 0x7c4c4e, 0x7c4c5e, 0x7c4cc7, 0x7c4ccb). Callers (2): Copy 0x7c4830,
  CreateFromHierarchy 0x7c4ed0 (plus game code via thunks).
- Behaviour: 1. `h = G->freeListAlloc([0xc9b8cc], 0x3011e)` (0x7c4c44); NOT NULL-checked and NOT zeroed.
  2. `h->currentAnim = RtAnimInterpolatorCreate(numNodes, maxKeyFrameSize)` (0x7c4c56/0x7c4c5b). 3. `h->flags = flags`,
  `h->numNodes = numNodes`, `h->parentFrame = NULL` (0x7c4c69..0x7c4c6e).
  4. `!(flags & 2)`: `p = G->rwmalloc((numNodes << 6) + 0xf, 0x3011e)` (0x7c4c87), unaligned = p, aligned = (p+0xf)&~0xf
  (0x7c4c93..0x7c4c99); else both NULL (0x7c4c9e).
  5. `h->pNodeInfo = G->rwmalloc(numNodes << 4, 0x3011e)` (0x7c4cb5).
  6. For `i = 0 .. numNodes-1` (numNodes = the argument, 0x7c4d19): `info[i].pFrame = NULL` (0x7c4ce1); if `nodeIDs != NULL`
     `info[i].nodeID = nodeIDs[i]` (0x7c4cf7); `info[i].nodeIndex = i` (0x7c4cfd); if `nodeFlags != NULL`
     `info[i].flags = nodeFlags[i]` (0x7c4d0e). A NULL array leaves the field UNINITIALISED (rwmalloc memory).
  7. `h->parentHierarchy = h` (0x7c4d1e); `h->rootParentOffset` (+0x1c) is NEVER written (freelist garbage). Return h.
- librw: hanim.cpp:HAnimHierarchy::create differs: librw uses rwMalloc with NULL check, writes 0 for missing ids/flags;
  the exe leaves them and rootParentOffset uninitialised.

## 0x7C4ED0 RpHAnimHierarchyCreateFromHierarchy  (catalogue: specified/REA, port level 2)
- Convention: `RpHAnimHierarchy* (RpHAnimHierarchy* src, i32 flags, i32 maxKeyFrameSize)`.
- Behaviour: `hn = RpHAnimHierarchyCreate(src->numNodes, NULL, NULL, flags, maxKeyFrameSize)` (0x7c4ee7); no NULL check.
  For `i < hn->numNodes` (re-read each iteration, 0x7c4f30): `hn.info[i].pFrame = NULL` (0x7c4f00), flags (+8, 0x7c4f12),
  nodeIndex (+4, 0x7c4f20), nodeID (+0, 0x7c4f2d) copied from `src->pNodeInfo[i]`. Returns hn (parentFrame stays NULL).
- librw: absent.

## 0x7C5120 RpHAnimHierarchyGetMatrixArray  (catalogue: specified/REA, port level 0)
- `RwMatrix* (RpHAnimHierarchy* h)`: return `h->pMatrixArray` (+0x8, 0x7c5124). 39 callers (peds, cutscenes).
- librw: field access, matches.

## 0x7C5130 RpHAnimFrameSetHierarchy  (catalogue: specified/REA, port level 0)
- `RwBool (RwFrame* f, RpHAnimHierarchy* h)`; returns 1 always.
- Behaviour: `slot = f + [0xc9b8c8] + 4` (0x7c513d); `old = *slot`; if `old != NULL` `old->parentFrame = NULL`
  (0x7c5145); `*slot = h` (0x7c5150); if `h != NULL` `h->parentFrame = f` (0x7c515b). return 1.
- librw: absent (librw sets HAnimData fields directly).

## 0x7C5160 RpHAnimFrameGetHierarchy  (catalogue: specified/REA, port level 0)
- `RpHAnimHierarchy* (RwFrame* f)`: return `*(f + [0xc9b8c8] + 4)` (0x7c516a).
- librw: hanim.cpp:HAnimHierarchy::get matches.

## 0x7C51A0 RpHAnimIDGetIndex  (catalogue: specified/REA, port level 0)
- `i32 (RpHAnimHierarchy* h, i32 id)`. Linear search `i = 0 .. h->numNodes-1` (numNodes read once, 0x7c51ab) for
  `h->pNodeInfo[i].nodeID == id` (0x7c51b9); returns the first i, or -1 (0x7c51a5) if not found / numNodes <= 0.
- librw: hanim.cpp:HAnimHierarchy::getIndex(int32) matches.

## 0x7C51D0 RpHAnimHierarchyUpdateMatrices  (catalogue: specified/REA, port level 4)
- Convention: `RwBool (RpHAnimHierarchy* h)`; always returns 1 (0x7c5608, 0x7c5b53). Frame: `sub esp, 0xa14` + 4 pushes;
  offsets below are relative to esp after the pushes. Caller: CEntity::UpdateRpHAnim 0x532b20.
- Callees: RwFrameDirty 0x7f0340, RwFrameGetLTM 0x7f0990, RwMatrixMultiply 0x7f18b0 (`dst = a * b`, args (dst, a, b)),
  RwFrameUpdateObjects 0x7f0910, indirect `anim->keyFrameApplyCB`.
- Locals: `flags` [0x1c] (copy of h->flags, 0x7c51e7); `animMat` RwMatrix at [0x54]; quaternion products at
  [0x2c..0x40], [0x98].

### Shared pieces
- **ComputeRootLTM(frame, dst)**: `p = frame->parent` (+4). If `p == NULL`: `dst` = identity built field by field:
  right/up/at diagonal `1.0f` (stores of 0x3f800000), other components and pos 0, `dst.flags = dst.flags | 0x20003`
  (OR into the uninitialised stack value, 0x7c535d / 0x7c5895). Else if `RwFrameDirty(p) == 0`: `dst = *RwFrameGetLTM(p)`
  (0x40-byte copy, 0x7c5227 / 0x7c5753). Else (dirty, avoids a sync): `dst = p->modelling` (copy from p+0x10, 0x7c5301);
  then for `q = p->parent; q != NULL; q = q->parent`: `tmp = dst` (copy to [0x1a4]); `RwMatrixMultiply(&dst, &tmp,
  &q->modelling)` (0x7c5337 / 0x7c586f).
- **MarkRootDirtyForLTM** (only if `flags & 0x2000`, 0x7c522f / 0x7c5759): `r = h->parentFrame->root` (+0xa0); if
  `(r->privateFlags & 3) == 0` (0x7c5244): insert `&r->inDirtyListLink` at the head of `G->dirtyFrameList` (G+0xbc):
  `r.link.next = head.next` (0x7c5256), `r.link.prev = &head` (0x7c526e), `head.next->prev = &r.link` (0x7c5289),
  `head.next = &r.link` (0x7c529e); then `r->privateFlags |= 2` (only bit 1, 0x7c52ad / 0x7c57d7; unlike RwFrameUpdateObjects
  which sets 3). h->parentFrame is dereferenced without a NULL test.
- **InlineApply** (when `anim->keyFrameApplyCB == 0x7c5b80` RpHAnimKeyFrameApply, compare 0x7c53ef / 0x7c5920), x87 with
  every product stored to an f32 stack slot (so rounded to f32), `k = interp frame`, x=[k+8], y=[k+0xc], z=[k+0x10],
  w=[k+0x14]: `xx=x*x, yy=y*y, zz=z*z, zy=z*y, zx=z*x, yx=y*x, wx=w*x` (stored), `wy=w*y, wz=w*z` (kept in x87 registers).
  Then, in this order:
  `right.x = 1.0[0x858624] - ((zz + yy) + (zz + yy))` (fadd st0,st0 doubling, then fsubr),
  `right.y = (yx + wz)*2`, `right.z = (zx - wy)*2`, `up.x = (yx - wz)*2`, `up.y = 1.0 - (zz + xx)*2`,
  `up.z = (wx + zy)*2`, `at.x = (wy + zx)*2`, `at.y = (zy - wx)*2`, `at.z = 1.0 - (yy + xx)*2`;
  `animMat.flags = 3` (0x7c54f9); `animMat.pos = (k+0x18, k+0x1c, k+0x20)` (copied as dwords). The pad words
  (+0x1c, +0x2c, +0x3c of animMat) are not written. Each "*2" is `t + t` in x87 (0x7c546e etc.).
  Otherwise: `anim->keyFrameApplyCB(&animMat, k)` (0x7c5522 / 0x7c5a53).

### Path A: `flags & 2` (NOMATRICES) - 0x7c51eb..0x7c5620
Matrices are kept BY VALUE in a stack array at [0x264] (room for 31 matrices up to 0xa24; no bound check).
1. `ComputeRootLTM(h->parentFrame, parentMat=[0xa4])` (h->parentFrame dereferenced at 0x7c51f4 without test; there is NO
   LOCALSPACEMATRICES test in this path). 2. `ltmFlag = flags & 0x2000` ([0x28], 0x7c5235); MarkRootDirtyForLTM.
3. `anim = h->currentAnim`; `stride = anim->currentInterpKeyFrameSize` ([0x44], 0x7c52c2); `k = anim + 0x4c`;
   `sp = &stack[0]`; `node = h->pNodeInfo`. If `h->numNodes <= 0` return 1 (0x7c52e0).
4. For `i = 0; i < h->numNodes (re-read, 0x7c55ec); i++, k += stride, node++`:
   a. InlineApply -> animMat. b. `RwMatrixMultiply(&cur=[0x164], &animMat, &parentMat)` (0x7c553c).
   c. `fr = node->pFrame`; if `fr != NULL`: if `flags & 0x1000` (0x7c5553): `fr->modelling = animMat` (copy to fr+0x10,
      0x7c5564); if `ltmFlag == 0` -> `RwFrameUpdateObjects(fr)` (0x7c556f) and skip the LTM store. If `ltmFlag != 0` (either
      branch): `fr->ltm = cur` (copy to fr+0x50, 0x7c558c); `fr->privateFlags = (fr->privateFlags & 0xfb) | 8` (0x7c5591).
   d. switch `node->flags & 3` (0x7c55a0): 0 -> `parentMat = cur`; 1 (POP) -> `sp -= 0x40; parentMat = *sp` (0x7c5615);
      2 (PUSH) -> `*sp = parentMat; sp += 0x40; parentMat = cur` (0x7c55ae..0x7c55de); 3 -> parentMat unchanged.
5. return 1.

### Path B: matrices present - 0x7c5622..0x7c5b5f
Here the parent "matrix" is a POINTER (`parent` [0x10]); the stack holds pointers at [0x1a8].
1. Root selection (0x7c5622..0x7c5759), `rootLTM` = matrix [0xe4]:
   - If `flags & 1` and `h->rootParentOffset != -1` (0x7c562f): `parent = h->parentHierarchy->pMatrixArray +
     rootParentOffset*64` (0x7c5637..0x7c5640). If `(flags & 0x2000) && (flags & 0x4000)`: ComputeRootLTM(
     `h->parentHierarchy->parentFrame` (0x7c5653), rootLTM) - with an extra NULL test of the frame (0x7c5718). Otherwise
     rootLTM is not computed.
   - Else if `flags & 0x4000` (LOCALSPACE, 0x7c565b): build identity at [0x124] (same field stores, flags |= 0x20003,
     0x7c5664..0x7c56fb); `parent = &[0x124]`; if `flags & 0x2000` ComputeRootLTM(h->parentFrame, rootLTM) (0x7c5715).
   - Else: `parent = &rootLTM` (0x7c570a) and ComputeRootLTM(h->parentFrame, rootLTM), where a NULL parentFrame also
     gives identity (0x7c5718..0x7c571a -> 0x7c5883).
2. `ltmFlag = flags & 0x2000` ([0x44], 0x7c575f); MarkRootDirtyForLTM (dereferences h->parentFrame without a test).
3. `anim = h->currentAnim`; `m = h->pMatrixArray` (0x7c57de); `stride = anim+0x24`; `k = anim+0x4c`; `sp = &ptrStack[0]`;
   `numNodes <= 0` -> return 1 (0x7c5818).
4. For each node (i < h->numNodes re-read 0x7c5b2e; `k += stride`, `m += 0x40`, `node++`):
   a. InlineApply -> animMat. b. `RwMatrixMultiply(m, &animMat, parent)` (0x7c5a63).
   c. `fr = node->pFrame` (0x7c5a6f); if `fr != NULL`: if `flags & 0x1000`: `fr->modelling = animMat` (0x7c5a8b); if
      `ltmFlag == 0` -> `RwFrameUpdateObjects(fr)` (0x7c5a96) and no LTM store. If `ltmFlag != 0`: if `flags & 0x4000`:
      `RwMatrixMultiply(&fr->ltm, m, &rootLTM)` (0x7c5acc) else `fr->ltm = *m` (copy, 0x7c5ade); then
      `fr->privateFlags = (pf & 0xfb) | 8` (0x7c5ae8..0x7c5af1).
   d. switch `node->flags & 3` (0x7c5af8): 0 -> `parent = m`; 1 (POP) -> `sp -= 4; parent = *sp` (0x7c5b60);
      2 (PUSH) -> `*sp = parent; sp += 4; parent = m` (0x7c5b08..0x7c5b19); 3 -> unchanged.
5. return 1.

- Edge cases: identity flags are OR-ed into uninitialised stack words (only bits 0x20003 are defined); the POP/PUSH stacks
  have no bounds checks; case 3 (push+pop) leaves the parent unchanged (equivalent to librw's push-then-pop).
- librw: hanim.cpp:HAnimHierarchy::updateMatrices differs: librw only implements path B without sub-hierarchies, without
  UPDATEMODELLINGMATRICES/UPDATELTMS/LOCALSPACE handling, takes `parfrm->getLTM()` (syncs dirty frames) instead of
  composing modelling matrices, and always calls applyCB (`Matrix::rotate(q)`, different float order) instead of the
  exe's inlined quaternion-to-matrix.

## Open questions
- None (the identity-flag OR at 0x7c535d/0x7c5895 reads uninitialised stack bits by design of the original; ports must
  keep `flags |= 0x20003` semantics; the other bits are undefined).
