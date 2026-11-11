# RwFrame pass 3: hierarchy edits, frame lists (RE-SPEC-RW-3 lead, taking over writer B's unwritten file)

All `__cdecl`. RwFrame layout as in docs/re/specs/RW/RwFrame.md (pass 1): +0x3 privateFlags (u8), +0x4 parent, +0x8/+0xc dirty-list link
(next/prev), +0x10 modelling RwMatrix (right +0x10, flags +0x1c, up +0x20, at +0x30, pos +0x40), +0x98 child, +0x9c next sibling, +0xa0 root.
Frame plugin registry 0x8e2270. Engine dirty-frame list head at `Engine+0xbc` (RwGlobals.dirtyFrameList).

"Mark dirty" (an inlined block that appears in several functions below, e.g. 0x7f0ef3-0x7f0f5b): `r = f->root (+0xa0)`; if `(r.privateFlags & 3) == 0`
insert `r+8` at the HEAD of the engine dirty list (`r+8 = first`, `r+0xc = Engine+0xbc`, `first.prev = r+8`, `Engine+0xbc = r+8`); then `r.privateFlags |= 3`
and `f.privateFlags |= 0xc` (byte ORs; the value ORed into r is the byte read before the insert).

"Detach" (inlined in 0x7f06f0 and 0x7f09c0): if `f->parent` p: unlink f from p's child chain (`p->child == f` ? `p->child = f->next` : walk +0x9c to the
predecessor and bypass f); `f->parent = 0`, `f->next = 0`, `f->root = f`; `_rwSetHierarchyRoot 0x7f0210(c, f)` for each child c (via +0x9c); then mark dirty f.

## 0x7f0250 RwFrameCloneHierarchy  (catalogue: specified/REA, port level 2)
- `RwFrame* (RwFrame* src)`. `c = _rwFrameCloneRecurse 0x7f0050(src, NULL)`; if c: `c.privateFlags &= ~3` (0x7f0270), then mark dirty c (root of c = c+0xa0).
- Then the SOURCE hierarchy roots are repaired (the clone recursion uses the root field as scratch): `src->root = src->parent ? src->parent->root : src`;
  `_rwSetHierarchyRoot(child, src->root)` for each direct child of src (0x7f02ff / 0x7f0322). Returns c (NULL if the clone failed; the repair still runs).
- librw: frame.cpp `Frame::cloneHierarchy` differs (librw repairs via a separate map, no dirty-flag clearing).

## 0x7f0450 _rwFrameInit  (catalogue: specified/REA, port level 1)
- `RwBool (RwFrame* f)`: `_rwFrameInternalInit 0x7f0360(f)`; `f.privateFlags |= 0x10`; returns 1.

## 0x7f06f0 _rwFrameDeInit  (catalogue: specified/REA, port level 1)
- `RwBool (RwFrame* f)`: `_rwPluginRegistryDeInitObject 0x808740(0x8e2270, f)`; Detach (above, including mark dirty of f) when it has a parent; then if
  `f.privateFlags & 3` unlink `f+8` from the dirty list (0x7f07f2-0x7f0800; the flags are not cleared); every child gets `parent = 0` (0x7f080d; children
  are not destroyed and keep their root). Returns 1.

## 0x7f09b0 RwFrameGetRoot  (catalogue: specified/REA, port level 0)
- `RwFrame* (RwFrame* f)`: returns `f+0xa0`.

## 0x7f09c0 RwFrameAddChildNoUpdate  (catalogue: specified/REA, port level 1)
- `RwFrame* (RwFrame* parent [esp+4], RwFrame* child [esp+8])`. If `child->parent`: Detach child (including mark dirty of child, which puts it on the
  dirty list as a root). Then `child->next = parent->child`; `parent->child = child`; `child->parent = parent`; `child->root = parent->root`;
  `_rwSetHierarchyRoot(gc, root)` for each child gc of child. Returns parent.
- Quirk: a child that was detached here stays linked in the engine dirty list (privateFlags 3) although it is no longer a root; the parent hierarchy is
  not marked dirty ("NoUpdate").

## 0x7f0e00 RwFrameCount  (catalogue: specified/REA, port level 0)
- `RwInt32 (RwFrame* f)`: recursive `1 + sum RwFrameCount(c)` over the children (+0x98, then +0x9c).

## 0x7f0ed0 RwFrameScale  (catalogue: specified/REA, port level 1)
- `RwFrame* (RwFrame* f, const RwV3d* v, RwOpCombineType op)`: `RwMatrixScale 0x7f22c0(f+0x10, v, op)`; mark dirty f; returns f. Caller e.g. 0x5a43a0
  (CClothesBuilder).

RwFrameList: +0x0 RwFrame** frames, +0x4 RwInt32 numFrames.

## 0x8075d0 _rwFrameListInitialize  (catalogue: specified/REA, port level 2)
- `RwFrameList* (RwFrameList* list, RwFrame* root)`. `n = RwFrameCount(root)`; `a = Engine+0x134(4n, 0x4000e)`; NULL -> `RwErrorSet({1, _rwerror(0x80000013, 4n)})`,
  NULL. Order: `a[0] = root`; if `root->child` c: `a[1] = c`, then `_rwFrameListFlattenRecurse(&cursor, c->next)`, then `(&cursor, c->child)`. Since
  0x8076a0 itself emits a frame, then its `next` chain, then its children, the order is a "siblings-before-children" pre-order. `list->frames = a`,
  `list->numFrames = n`; returns list. (root == NULL would crash in RwFrameCount; the NULL test at 0x807632 only skips the fill.)

## 0x8076a0 _rwFrameListFlattenRecurse  (catalogue: specified/REA, port level 0)
- `RwBool (RwFrame*** cursor, RwFrame* f)`: if f: `**cursor = f`, `*cursor += 4`; recurse `(cursor, f->next)`; recurse `(cursor, f->child)`. Returns 1.

## 0x8076e0 _rwFrameListFindFrame  (catalogue: specified/REA, port level 0)
- `RwBool (RwFrameList* list, RwFrame* f, RwInt32* index)`: first i < numFrames (signed) with `frames[i] == f`: `*index = i`, return 1; else 0.

## 0x807750 _rwFrameListStreamGetSize  (catalogue: specified/REA, port level 1)
- `RwUInt32 (RwFrameList* list)`: `56n + 0x10 + sum_i (_rwPluginRegistryGetSize 0x808b00(0x8e2270, frames[i]) + 0xc)`.

## 0x8077a0 _rwFrameListStreamWrite  (catalogue: specified/REA, port level 2)
- `RwFrameList* (RwFrameList* list, RwStream* stream)`. Header `(stream, 0xe FRAMELIST, size as 0x807750, 0x36003, 0xffff)`; struct header
  `(stream, 1, 56n + 4, ...)`; `RwStreamWrite(stream, &n, 4)`. Any failure -> NULL.
- Per frame (0x38-byte record): right (f+0x10, 3 dwords), up (f+0x20), at (f+0x30), pos (f+0x40), parent index = index of `f->parent` in the list (linear
  search from 0; -1 when no parent or not found), then `f+0x1c` (modelling matrix flags) as the last dword (0x8078ff-0x80790a). `RwStreamWrite(stream, rec, 0x38)`.
- Then for each frame `_rwPluginRegistryWriteDataChunks 0x808b40(0x8e2270, stream, frame)`. Returns list.
- librw: frame.cpp FrameList_::streamWrite: same record layout (the last dword was not compared with librw).
