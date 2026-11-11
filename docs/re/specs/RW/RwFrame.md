# RenderWare 3.6 frames (RW core) - full specs, writer A (RE-SPEC-RW)

All functions are RenderWare 3.6 code statically linked into gta_sa.exe 1.0 US. gta-reversed and plugin-sdk
only have call thunks (`plugin::Call*`) for them, so everything below comes from the bytes. All functions are
`__cdecl` (caller pops) unless stated. "free(x)" / "alloc" below always name the exact RwGlobals hook used.

## Layouts used (offsets proven by the instruction addresses given)

`RwGlobals` (`RwEngineInstance` = dword at 0xC97B24; written `G` below):
- G+0xBC `RwLinkList dirtyFrameList` (head: +0xBC next, +0xC0 prev): initialised by FRAMEMODULE Open 0x7EFF44..0x7EFF56,
  walked by `_rwFrameSyncDirty` 0x809559.
- G+0x134 `memoryFuncs.rwmalloc(size, hint)` (0x8079E9), G+0x138 `rwfree(p)` (0x807735).
- G+0x144 `memoryAlloc(freelist, hint)` (0x7F0067), G+0x148 `memoryFree(freelist, p)` (0x7F0193). With the default
  engine init these are `_rwFreeListAllocReal` 0x801C30 / `_rwFreeListFreeReal` 0x801D50 (see RwMemory.md).

FRAMEMODULE (engine plugin 0x403, 4 bytes): offset global 0xC9796C (written by 0x7EFF0A); module data
`G + [0xC9796C]` +0 = frame freelist pointer (0x7EFF2D). Open-instance counter 0xC97970 (0x7EFF62).

`RwFrame` (base 0xA4 bytes; allocation size = frame registry `sizeOfStruct` at 0x8E2270, 0xC8 with GTA's plugins):
| off | field | proof |
|---|---|---|
| +0x0 u8 | object.type (0 = rwFRAME) | 0x7F0372 |
| +0x1 u8 | object.subType | 0x7F037A |
| +0x2 u8 | object.flags | 0x7F0384 |
| +0x3 u8 | object.privateFlags (dirty bits, below) | 0x7F0387, 0x7F099B |
| +0x4 | object.parent (RwFrame*) | 0x7F038A, 0x7F0B06 |
| +0x8 | inDirtyListLink.next | 0x7EFFED |
| +0xC | inDirtyListLink.prev | 0x7F0002 |
| +0x10 | RwMatrix modelling (0x40 bytes; flags at +0x1C) | 0x7F03B1, 0x7F0E3F |
| +0x50 | RwMatrix ltm (flags at +0x5C) | 0x7F03E5, 0x7F09AA |
| +0x90 | RwLinkList objectList (next +0x90, prev +0x94) | 0x7F036C..0x7F037D |
| +0x98 | child | 0x7F03EC, 0x7F0B11 |
| +0x9C | next (sibling) | 0x7F03F2, 0x7F0B1B |
| +0xA0 | root | 0x7F03F8, 0x7F0344 |

`privateFlags` bits (RW names from rwcore.h:4177): 0x01 HIERARCHYSYNCLTM, 0x02 HIERARCHYSYNCOBJ (both only on a
hierarchy root; set together = "root is in the dirty list"), 0x04 SUBTREESYNCLTM, 0x08 SUBTREESYNCOBJ.

`RwObjectHasFrame` (atomics, lights, cameras): +0x4 parent frame, +0x8 lFrame.next, +0xC lFrame.prev, +0x10 sync
callback `RwObjectHasFrame* (*)(RwObjectHasFrame*)` (0x804EF4..0x804F28; called at 0x8095B9 with the object
pointer = link - 8).

`RwMatrix`: right +0x0, flags +0xC, up +0x10, at +0x20, pos +0x30 (see RwMatrix.md). Identity flags value
0x20003 (rwMATRIXINTERNALIDENTITY 0x20000 | rwMATRIXTYPEORTHONORMAL 3), 0x7F03B1.

`rwFrameList` (frame list streaming helper): +0x0 `RwFrame** frames`, +0x4 `i32 numFrames` (0x8079DC, 0x8079F2).

Frame plugin registry: `RwPluginRegistry` at 0x8E2270 (see RwPluginRegistry.md).

### Shared inline sequences (expanded in many functions; described once here)
- **MarkDirty(f)** = the inlined body of `RwFrameUpdateObjects` (0x7F0910..0x7F098C): `r = f->root`; read
  `rf = r->privateFlags` once; if `(rf & 3) == 0`, insert `&r->inDirtyListLink` at the **head** of `G->dirtyFrameList`
  in this order: `link.next = head.next` (0x7F0931), `link.prev = &head` (0x7F0946), `head.next->prev = &link`
  (0x7F095E), `head.next = &link` (0x7F0970). Then `r->privateFlags = rf | 3` (0x7F0980) and afterwards
  `f->privateFlags = f->privateFlags | 0x0C` (re-read after the root store, 0x7F0983; when f == r the result is
  `rf | 0x0F`).
- **Unlink(f)**: `f->inDirtyListLink.prev->next = link.next; link.next->prev = link.prev` (0x7F06A2..0x7F06B0);
  the link fields themselves are not cleared.
- **Detach(child)** (0x7F0B11..0x7F0B7E, identical in RwFrameRemoveChild/AddChild/AddChildNoUpdate/Destroy):
  `p = child->parent`; if `p->child == child` then `p->child = child->next`, else walk `s = p->child` while
  `s->next != child` and set `s->next = child->next` (no termination guard: child must be in the list);
  then `child->parent = NULL; child->next = NULL; child->root = child`; for every `c` in child's child list
  (`c = c->next`, next read after the call): `_rwSetHierarchyRoot(c, child)`.

## 0x7EFEF0 cb_FRAMEMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- Convention: `void* Open(void* instance, i32 offset, i32 size)` (engine plugin constructor); returns instance or NULL.
- Callers: `_rwPluginRegistryInitObject` from RwEngineStart 0x7F2E9A (via engine registry). Callees:
  `RwFreeListCreateAndPreallocateSpace` 0x801B70.
- Globals: W 0xC9796C (module offset) = offset; R 0x8E2270 frame `sizeOfStruct`, 0x8E2288 (entriesPerBlock, .data
  init 0x32), 0x8E228C (blocks to preallocate, init 1); RW 0xC97970 instance counter.
- Behaviour: 1. `[0xC9796C] = offset`. 2. `fl = RwFreeListCreateAndPreallocateSpace([0x8E2270], [0x8E2288], 4,
  [0x8E228C], (void*)0xC97974 /*static in-place freelist*/, 0x4000E)`; store at `G+offset+0`. 3. If NULL return NULL.
  4. `G->dirtyFrameList.next = .prev = &G->dirtyFrameList` (0x7EFF49, 0x7EFF56). 5. `[0xC97970]++`; return instance.
- librw: frame.cpp:frameOpen differs: no freelist (librw mallocs frames).

## 0x7EFFB0 _rwFrameCloneAndLinkClones  (catalogue: specified/REA, port level 3)
- Convention: `RwFrame* (RwFrame* root)`; returns the cloned root or NULL.
- Callers (2): CFileLoader::LoadClumpFile 0x5372D0, RpClumpClone 0x749F70. Callees: `_rwFrameCloneRecurse` 0x7F0050.
- Behaviour: `c = _rwFrameCloneRecurse(root, NULL)`; NULL -> return NULL. `c->privateFlags &= 0xFC` (0x7EFFD0;
  the clone copied the original's bits, so the clone must not look as if it were already in the dirty list);
  then MarkDirty(c) (c is its own root, so c is inserted at the dirty-list head and ends with `privateFlags | 0x0F`).
  Return c. Afterwards every original frame's `root` field points to its clone (set by 0x7F00E0) until
  `_rwFramePurgeClone` restores it ("link clones").
- librw: frame.cpp:Frame::cloneAndLink matches.

## 0x7F0050 _rwFrameCloneRecurse  (catalogue: specified/REA, port level 2)
- Convention: `RwFrame* (RwFrame* old, RwFrame* newRoot)`; preserves EBX/EBP/ESI/EDI; returns new frame or NULL.
- Callers: 0x7EFFB0, RwFrameCloneHierarchy 0x7F0250, itself. Callees: G+0x144 alloc, itself, `_rwPluginRegistryInitObject`
  0x8086E0, `_rwPluginRegistryCopyObject` 0x808770, `_rwFrameDestroyRecurse` 0x7F0830, `_rwPluginRegistryDeInitObject`
  0x808740, G+0x148 free.
- Behaviour:
  1. `f = memoryAlloc(frameFreeList, 0x3000E)`; NULL -> return NULL (no error).
  2. If `newRoot == NULL` then `newRoot = f` (0x7F0084).
  3. Copy the 4 object bytes +0..+3 one by one from old (type, subType, flags, privateFlags) (0x7F0090..0x7F00AD);
     `f->objectList` = empty (next = prev = &f->objectList, 0x7F00B0/0x7F00BB); `f->parent = NULL` (0x7F00C1);
     `f->modelling = old->modelling` (16 dwords incl. flags and pads, `rep movsd` 0x7F00C4); `f->child = NULL;
     f->next = NULL; f->root = newRoot`; **`old->root = f`** (0x7F00E0).
     Not written: `f->inDirtyListLink` (+0x8/+0xC), `f->ltm` (+0x50..+0x8F) and the plugin area: they keep the
     freelist slot's previous contents until the next sync.
  4. For each child `c` of old (`c = c->next` read from the original after the recursion, 0x7F0110):
     `cc = _rwFrameCloneRecurse(c, newRoot)`; NULL -> step 6. Else `cc->next = f->child; f->child = cc;
     cc->parent = f` (prepend: the clone's child order is the reverse of the original's).
  5. `_rwPluginRegistryInitObject(&frameRegistry 0x8E2270, f)` (result ignored); `_rwPluginRegistryCopyObject(
     0x8E2270, f, old)`; return f.
  6. Failure: for each already cloned child of f (next read before the call): `_rwFrameDestroyRecurse(cc)`;
     `_rwPluginRegistryDeInitObject(0x8E2270, f)` (plugin destructors run although InitObject never ran for f);
     if `f->privateFlags & 3` (copied from old): Unlink(f) using the uninitialised link fields of step 3;
     `memoryFree(frameFreeList, f)`; return NULL.
- Edge cases: the failure path only happens on freelist exhaustion; it is buggy in the original (deinit of
  unconstructed plugins, unlink through stale pointers) and must be reproduced only if exercised.
- librw: frame.cpp:cloneRecurse differs: librw uses Frame::create (initialised ltm, constructed plugins first),
  copies object via object.copy, has no failure path.

## 0x7F01A0 _rwFramePurgeClone  (catalogue: specified/REA, port level 1)
- Convention: `RwFrame* (RwFrame* frame)`; returns frame.
- Callers: RpClumpClone 0x749F70. Callees: `_rwSetHierarchyRoot` 0x7F0210.
- Behaviour: `r = frame->parent ? frame->parent->root : frame` (0x7F01B4 / 0x7F01E0); `frame->root = r`; for each
  child c of frame (next read after the call): `_rwSetHierarchyRoot(c, r)`. Return frame.
- librw: frame.cpp:Frame::purgeClone matches.

## 0x7F0210 _rwSetHierarchyRoot  (catalogue: specified/REA, port level 0)
- Convention: `void (RwFrame* f, RwFrame* root)`; EAX = f on return (not used).
- Behaviour: `f->root = root` (0x7F0220); for each child c of f (c->next read after the recursive call, 0x7F0231):
  `_rwSetHierarchyRoot(c, root)`. f's own siblings are not visited.
- librw: frame.cpp:Frame::setHierarchyRoot matches.

## 0x7F0340 RwFrameDirty  (catalogue: specified/REA, port level 0)
- `RwBool (const RwFrame* f)`: returns `f->root->privateFlags & 3` (EAX fully defined: `and eax, 3` 0x7F034D).
- librw: absent.

## 0x7F0360 _rwFrameInternalInit  (catalogue: specified/REA, port level 1)
- Convention: `(RwFrame* f)`; returns EAX = result of `_rwPluginRegistryInitObject` (registry pointer or NULL).
- Callers: RwFrameCreate 0x7F0410, _rwFrameInit 0x7F0450. Callees: `_rwPluginRegistryInitObject` 0x8086E0.
- Behaviour (stores in this order is irrelevant; values): object bytes +0..+3 = 0; parent = NULL; objectList empty;
  modelling = identity (+0x10/+0x24/+0x38 = 1.0f `0x3F800000`, other 9 floats 0, flags +0x1C = 0x20003);
  ltm = identity (+0x50/+0x64/+0x78 = 1.0f, flags +0x5C = 0x20003); child = next = NULL; root = f. The matrix pad
  dwords (+0x2C/+0x3C/+0x4C, +0x6C/+0x7C/+0x8C) and `inDirtyListLink` are not written.
  Then `return _rwPluginRegistryInitObject(0x8E2270, f)`.
- librw: frame.cpp:Frame::create matches (allocation separated).

## 0x7F0410 RwFrameCreate  (catalogue: specified/REA, port level 2)
- `RwFrame* (void)`. Callers (22) e.g. CAtomicModelInfo::CreateInstance 0x4C44D0, _rwFrameListStreamRead.
- Behaviour: `f = memoryAlloc(frameFreeList, 0x3000E)`; NULL -> return NULL (no RwError). `_rwFrameInternalInit(f)`
  (its result is ignored: a plugin-constructor failure still returns f, 0x7F043F). Return f.
- librw: frame.cpp:Frame::create differs: sets RWERROR(ERR_ALLOC) on failure.

## 0x7F05A0 RwFrameDestroy  (catalogue: specified/REA, port level 1)
- `RwBool (RwFrame* f)`: always returns 1. Callers (25) e.g. CAtomicModelInfo::DeleteRwObject 0x4C4440.
- Behaviour, in order:
  1. `_rwPluginRegistryDeInitObject(0x8E2270, f)` (0x7F05AB) - plugin destructors run first.
  2. If `f->parent != NULL`: Detach(f) (0x7F05C0..0x7F0625), then MarkDirty(f) (f is now its own root, so f is put
     into the dirty list and gets bits |= 0x0F) (0x7F0627..0x7F0699).
  3. If `f->privateFlags & 3`: Unlink(f) (0x7F069C) (so a frame that step 2 just inserted is removed again).
  4. For each child c of f (`c = c->next`): `c->parent = NULL` (0x7F06BD). Their `root` fields are NOT reset: if f
     had a parent they were set to f in step 2, otherwise they already were f -> children keep a dangling root.
  5. `memoryFree(frameFreeList, f)`; return 1.
- Not done: attached objects in `objectList` are not detached (their `parent` keeps pointing at the freed frame).
- librw: frame.cpp:Frame::destroy differs: librw detaches every attached object first and destructs plugins
  after the object loop; exe order is plugins first, no object loop.

## 0x7F0830 _rwFrameDestroyRecurse  (catalogue: specified/REA, port level 1)
- `void (RwFrame* f)`. Callers: 0x7F0050, 0x7F08A0, itself.
- Behaviour: if f == NULL return. For each child c (next read before the recursive call, 0x7F0844):
  `_rwFrameDestroyRecurse(c)`. Then `_rwPluginRegistryDeInitObject(0x8E2270, f)`; if `f->privateFlags & 3`
  Unlink(f); `memoryFree(frameFreeList, f)`. No detach from parent, no object handling.
- librw: frame.cpp:Frame::destroyHierarchy matches (librw asserts empty objectList).

## 0x7F08A0 RwFrameDestroyHierarchy  (catalogue: specified/REA, port level 2)
- `RwBool (RwFrame* f)`: returns 1 (also for f == NULL, 0x7F0909). Body identical to `_rwFrameDestroyRecurse` (the
  children loop calls 0x7F0830). f is NOT removed from its parent's child list (callers pass roots).
- Callers (5): _rpClumpDestructor, RpClumpClone, RpClumpDestroy, RpClumpStreamRead, _rwFrameListStreamRead.
- librw: frame.cpp:Frame::destroyHierarchy matches.

## 0x7F0910 RwFrameUpdateObjects  (catalogue: specified/REA, port level 0)
- `RwFrame* (RwFrame* f)`: MarkDirty(f) (see header, exact store order there); returns f (EAX = f, 0x7F0910).
- Callers (56) e.g. CReplay::ProcessReplayCamera, CVehicleModelInfo::SetRenderPipelines, `_rwObjectHasFrameSetFrame`.
- librw: frame.cpp:Frame::updateObjects matches (librw LinkList::add also inserts at head).

## 0x7F0990 RwFrameGetLTM  (catalogue: specified/REA, port level 3)
- `RwMatrix* (RwFrame* f)`: if `f->root->privateFlags & 1` (0x7F099B) call `_rwFrameSyncHierarchyLTM(f->root)`;
  return `&f->ltm` (f + 0x50). Callers (62): CPhysical::PositionAttachedEntity, CPed::PositionAttachedPed, ...
- librw: frame.cpp:Frame::getLTM matches.

## 0x7F0B00 RwFrameAddChild  (catalogue: specified/REA, port level 1)
- `RwFrame* (RwFrame* parent, RwFrame* child)`; returns parent. Callers (7): CVehicleModelInfo::PreprocessHierarchy,
  CVehicleModelInfo::CreateInstance, CFileLoader::LoadClumpFile, _rwFrameListStreamRead (when auto-update is on), ...
- Behaviour, in order:
  1. If `child->parent != NULL`: Detach(child) then MarkDirty(child) (0x7F0B0B..0x7F0BF0).
  2. `child->next = parent->child; parent->child = child` (prepend, 0x7F0BF7..0x7F0C03); `child->parent = parent`;
     `child->root = parent->root`; for each c in child's children: `_rwSetHierarchyRoot(c, parent->root)` (the root
     value is read once into EDI at 0x7F0C12).
  3. If `child->privateFlags & 3` (child was a root in the dirty list): Unlink(child); `child->privateFlags &= 0xFC`
     (0x7F0C36..0x7F0C52).
  4. MarkDirty(child) (now marks parent's root; 0x7F0C55..0x7F0CC6). Return parent.
- Related, not in this list: `RwFrameAddChildNoUpdate` 0x7F09C0 = steps 1 and 2 only (returns parent).
- librw: frame.cpp:Frame::addChild matches for append == false (librw optionally appends, exe always prepends).

## 0x7F0CD0 RwFrameRemoveChild  (catalogue: specified/REA, port level 1)
- `RwFrame* (RwFrame* child)`; returns child. No NULL check on `child->parent` (crash if called on a root).
- Behaviour: Detach(child) (0x7F0CD0..0x7F0D3F); MarkDirty(child) (0x7F0D41..0x7F0DB2). Caller: CVehicleModelInfo::PreprocessHierarchy.
- librw: frame.cpp:Frame::removeChild matches.

## 0x7F0DC0 RwFrameForAllChildren  (catalogue: specified/REA, port level 0)
- `RwFrame* (RwFrame* f, RwFrame* (*cb)(RwFrame*, void*), void* data)`; returns f.
- Behaviour: `c = f->child`; while c: `n = c->next` (read BEFORE the callback, 0x7F0DDA); `if (cb(c, data) == NULL) break;
  c = n`. Direct children only.
- librw: frame.cpp:Frame::forAllChildren matches.

## 0x7F0E30 RwFrameTranslate  (catalogue: specified/REA, port level 2)
- `RwFrame* (RwFrame* f, const RwV3d* v, RwOpCombineType op)`: `RwMatrixTranslate(&f->modelling, v, op)` (0x7F0E43,
  result ignored, so an invalid op crashes inside RwMatrixTranslate, see RwMatrix.md); MarkDirty(f); return f.
- librw: frame.cpp:Frame::translate matches.

## 0x7F0F70 RwFrameTransform  (catalogue: specified/REA, port level 2)
- `RwFrame* (RwFrame* f, const RwMatrix* m, RwOpCombineType op)`: `RwMatrixTransform(&f->modelling, m, op)`
  (0x7F0F83, result ignored); MarkDirty(f); return f.
- librw: frame.cpp:Frame::transform matches.

## 0x7F1010 RwFrameRotate  (catalogue: specified/REA, port level 3)
- `RwFrame* (RwFrame* f, const RwV3d* axis, RwReal angleDegrees, RwOpCombineType op)`: `RwMatrixRotate(&f->modelling,
  axis, angle, op)` (0x7F1028; angle passed as the raw dword); MarkDirty(f); return f.
- librw: frame.cpp:Frame::rotate matches.

## 0x7F10B0 RwFrameSetIdentity  (catalogue: specified/REA, port level 0)
- `RwFrame* (RwFrame* f)`: modelling right/up/at/pos = identity (1.0f at +0x10/+0x24/+0x38, other 9 floats 0);
  `modelling.flags |= 0x20003` (OR, other flag bits kept, 0x7F10E8); MarkDirty(f); return f (EAX = f).
- librw: absent (librw has matrix.setIdentity + updateObjects; it assigns the flags instead of OR).

## 0x7F1170 RwFrameOrthoNormalize  (catalogue: specified/REA, port level 3)
- `RwFrame* (RwFrame* f)`: `RwMatrixOrthoNormalize(&f->modelling, &f->modelling)` (in place, 0x7F117A); MarkDirty(f);
  return f. Callers: CReplay::ProcessReplayCamera, CCamera::CopyCameraMatrixToRWCam, CShadowCamera.
- librw: absent.

## 0x7F1200 RwFrameForAllObjects  (catalogue: specified/REA, port level 0)
- `RwFrame* (RwFrame* f, RwObject* (*cb)(RwObject*, void*), void* data)`; returns f.
- Behaviour: `l = f->objectList.next`; while `l != &f->objectList`: `n = l->next` (read before the callback,
  0x7F1220); `if (cb((RwObject*)(l - 8 bytes), data) == NULL) break; l = n`.
- librw: absent as such (FORLIST); matches semantics.

## 0x7F1260 RwFrameRegisterPlugin  (catalogue: specified/REA, port level 2)
- `RwInt32 (RwInt32 size, RwUInt32 pluginID, ctor, dtor, copy)` -> `_rwPluginRegistryAddPlugin(0x8E2270, size,
  pluginID, ctor, dtor, copy)`; returns its result (offset or -1). Callers: NodeNamePluginAttach, CVisibilityPlugins::PluginAttach, RpHAnimPluginAttach.

## 0x804EF0 _rwObjectHasFrameSetFrame  (catalogue: specified/REA, port level 1)
- Convention: `void (RwObjectHasFrame* obj, RwFrame* frame)`; EAX on return is garbage (callers ignore it).
- Callers (15): Light::LightsCreate, CGame::InitialiseRenderWare, CShadowCamera::Destroy, ... Callees: RwFrameUpdateObjects.
- Behaviour: 1. If `obj->parent != NULL`: unlink `obj->lFrame` (prev->next = next; next->prev = prev, 0x804EFB).
  2. `obj->parent = frame`. 3. If frame != NULL: insert `&obj->lFrame` at the head of `frame->objectList`
  (`link.prev = &list` 0x804F25, `link.next = old list.next` 0x804F28, `old.next->prev = &link` 0x804F2C,
  `list.next = &link` 0x804F2F), then `RwFrameUpdateObjects(frame)`.
- librw: frame.cpp:ObjectWithFrame::setFrame matches.

## 0x804F40 _rwObjectHasFrameReleaseFrame  (catalogue: specified/REA, port level 0)
- `void (RwObjectHasFrame* obj)`: if `obj->parent != NULL`, unlink `obj->lFrame` (same order as above). `obj->parent`
  is NOT cleared. Callers (11): _rpClumpDestructor, _rpAtomicDestructor, RpAtomicDestroy, ...
- librw: absent.

## 0x807580 RwFrameRegisterPluginStream  (catalogue: specified/REA, port level 1)
- `RwInt32 (RwUInt32 pluginID, readCB, writeCB, getSizeCB)` -> `_rwPluginRegistryAddPluginStream(0x8E2270, ...)`;
  returns the plugin offset or -1. Callers: NodeNamePluginAttach, RpHAnimPluginAttach.

## 0x807720 _rwFrameListDeinitialize  (catalogue: specified/REA, port level 0)
- `rwFrameList* (rwFrameList* list)`: if `list->numFrames != 0` (+0x4, 0x807725) then `G->rwfree(list->frames)`
  (G+0x138). Returns list. The frames themselves are not destroyed. Callers (7): RpClumpGtaStreamRead1/2,
  RpClumpGtaCancelStream, RpClumpStreamRead, ...
- librw: absent (librw frees inline).

## 0x807970 _rwFrameListStreamRead  (catalogue: specified/REA, port level 5)
- Convention: `rwFrameList* (RwStream* stream, rwFrameList* list)`; returns list or NULL. Frame of 0x4C bytes:
  locals numFrames [esp+0x10], version [+0x14], length [+0x18], RwError [+0x1C], `rwStreamFrame` buffer [+0x24]
  (0x38 bytes: right +0, up +0xC, at +0x18, pos +0x24, i32 parentIndex +0x30 = [esp+0x54], u32 data +0x34 unused).
- Callers: 0x72E3A0, RpClumpGtaStreamRead1 0x72E570, RpClumpStreamRead 0x74B420. Callees: RwStreamFindChunk 0x7ED2D0,
  RwStreamRead 0x7EC9D0, RwFrameCreate 0x7F0410, _rwMatrixNormalError 0x7F1500, _rwMatrixOrthogonalError 0x7F14A0,
  _rwMatrixDeterminant 0x7F1450, RwFrameAddChild 0x7F0B00, RwFrameAddChildNoUpdate 0x7F09C0,
  _rwPluginRegistryReadDataChunks 0x808980, RwFrameDestroyHierarchy 0x7F08A0, _rwerror 0x8088D0, RwErrorSet 0x808820.
- Globals: R 0x8E26A8 `frameListAutoUpdate` (i32, .data init 1; set by RwFrameListSetAutoUpdate 0x807570).
- Behaviour:
  1. `RwStreamFindChunk(stream, 1 /*rwID_STRUCT*/, &length, &version)`; 0 -> return NULL.
  2. If `version < 0x34000 || version > 0x36003` (unsigned, 0x8079A0/0x8079AB): RwErrorSet({1,
     _rwerror(0x80000004 E_RW_BADVERSION)}); return NULL.
  3. `RwStreamRead(stream, &numFrames, 4) != 4` -> return NULL (no error of its own).
  4. `list->numFrames = numFrames`; `list->frames = G->rwmalloc(numFrames * 4, 0x3000E)` (G+0x134); NULL ->
     RwErrorSet({1, _rwerror(0x80000013 E_RW_NOMEM, numFrames*4)}); return NULL.
  5. For `i = 0; i < numFrames; i++` (signed, bound re-read each time):
     a. `RwStreamRead(stream, &buf, 0x38) != 0x38` -> `G->rwfree(list->frames)`; return NULL (frames created so far leak).
     b. `fr = RwFrameCreate()`; NULL -> `rwfree(list->frames)`; return NULL.
     c. Copy the 12 floats into `fr->modelling` right/up/at/pos (flags dword untouched, = 0x20003 from create).
     d. Matrix type: if `_rwMatrixNormalError(&m) <= [0x858C58] = 0.01f` and `_rwMatrixOrthogonalError(&m) <= 0.01f`
        and NOT `_rwMatrixDeterminant(&m) < [0x862CD0] = 0.99f` -> `flags &= 0xFFFDFFFF` (keeps type bits 3: orthonormal);
        otherwise `flags &= 0xFFFDFFFC`. Compare details: `fcomp; test ah,0x41; jp` (0x807AD5/0x807AEB) takes the
        "otherwise" path when error > 0.01 or unordered; the determinant test `and eax,0x100; jne` (0x807B03) takes
        it when det < 0.99 or unordered. The three results are compared at x87 precision (ST0 return values).
     e. `list->frames[i] = fr`. If `buf.parentIndex >= 0` (signed): `p = list->frames[parentIndex]` (no bounds check;
        an index >= i reads an unset slot), then if `frameListAutoUpdate != 0` `RwFrameAddChild(p, fr)` else
        `RwFrameAddChildNoUpdate(p, fr)`.
  6. For `i = 0; i < numFrames; i++`: `_rwPluginRegistryReadDataChunks(0x8E2270, stream, list->frames[i])`; 0 ->
     `RwFrameDestroyHierarchy(list->frames[0])`; `rwfree(list->frames)`; return NULL.
  7. Return list.
- Edge cases: numFrames <= 0 skips both loops and returns list (with whatever rwmalloc(0 or negative*4) returned).
  The struct chunk length is never checked against numFrames.
- librw: frame.cpp:FrameList_::streamRead differs: no version check, uses Matrix::optimize (also identity test) then
  clears IDENTITY; exe uses normal/orthogonal/determinant, never sets 0x20000; librw ignores plugin read failure and
  has no auto-update switch (it has streamAppendFrames instead).

## 0x809550 _rwFrameSyncDirty  (catalogue: specified/REA, port level 2)
- Convention: `RwBool (void)`; returns 1. Caller: `_rwCameraDefaultBeginUpdate` 0x7EF370 (GTA replaces the camera
  begin-update via the world-object plugin, which still ends up here). Callees: RwMatrixMultiply 0x7F18B0, object sync
  callbacks (`obj->sync` at +0x10), `_rwFrameSyncDirtyLTM` 0x809700, `_rwFrameSyncDirtyObjects` 0x809780.
- Behaviour: `head = &G->dirtyFrameList`; `link = head->next`; while `link != head`:
  1. `fr = link - 8`; `flags = fr->privateFlags` (snapshot, 0x80957B).
  2. If `flags & 1` (HIERARCHYSYNCLTM):
     a. If `flags & 4`: `fr->ltm = fr->modelling` (16 dwords incl. flags/pads, 0x8095A0).
     b. SyncObjects(fr): for each link l in `fr->objectList` (next read AFTER the callback, 0x8095B9): `obj = l - 8;
        obj->sync(obj)`.
     c. `inh = flags & 4`; for each child c of fr (c->next read after the recursion, 0x809632):
        `cf = c->privateFlags | inh`; if `cf & 4`: `RwMatrixMultiply(&c->ltm, &c->modelling, &c->parent->ltm)`;
        SyncObjects(c) (always, whatever bit 8 says); `c->privateFlags &= 0xF3`; `_rwFrameSyncDirtyLTM(c->child, cf)`.
  3. Else: SyncObjects(fr); for each child c: SyncObjects(c); `c->privateFlags &= 0xF7`; `_rwFrameSyncDirtyObjects(c->child)`.
  4. `fr->privateFlags = flags & 0xF0` (written from the snapshot, 0x8096BE: any change a sync callback made to this
     byte is overwritten); `link = link->next` (read after processing).
  After the loop: `head->next = head->prev = head` (0x8096DD/0x8096EA); return 1. Frames are not unlinked
  individually (their link fields stay stale).
- librw: frame.cpp:Frame::syncDirty matches (same structure; librw re-reads flags for the final clear).

## 0x809700 _rwFrameSyncDirtyLTM  (catalogue: specified/REA, port level 1)
- `void (RwFrame* f, RwUInt32 parentFlags)`: for `; f; f = f->next` (next read after recursion): `cf = f->privateFlags
  | parentFlags`; if `cf & 4` RwMatrixMultiply(&f->ltm, &f->modelling, &f->parent->ltm); SyncObjects(f);
  `f->privateFlags &= 0xF3`; `_rwFrameSyncDirtyLTM(f->child, cf)`.
- librw: frame.cpp:syncRecurse matches.

## 0x809780 _rwFrameSyncDirtyObjects  (catalogue: specified/REA, port level 0)
- `void (RwFrame* f)`: for `; f; f = f->next`: SyncObjects(f); `f->privateFlags &= 0xF7`; `_rwFrameSyncDirtyObjects(f->child)`.
- librw: frame.cpp:syncObjRecurse matches.

## 0x8097D0 _rwFrameSyncHierarchyLTM  (catalogue: specified/REA, port level 2)
- `void (RwFrame* root)`. Caller: RwFrameGetLTM. Callees: RwMatrixMultiply, 0x809850.
- Behaviour: `flags = root->privateFlags` (snapshot kept in the argument slot, 0x8097DD); if `flags & 4`:
  `root->ltm = root->modelling` (16 dwords). For each child c (next read after recursion): `cf = c->privateFlags | flags`;
  if `cf & 4`: RwMatrixMultiply(&c->ltm, &c->modelling, &c->parent->ltm) and `c->privateFlags &= 0xFB` (only when
  multiplied, 0x809823); `_rwFrameSyncHierarchyLTMRecurse(c->child, cf)`. Finally `root->privateFlags = flags & 0xFA`
  (0x809847). No object sync; the root stays in the dirty list (bit 0x02 still set), so the next `_rwFrameSyncDirty`
  takes the objects-only branch for it.
- librw: frame.cpp:Frame::syncHierarchyLTM matches.

## 0x809850 _rwFrameSyncHierarchyLTMRecurse  (catalogue: specified/REA, port level 1)
- `void (RwFrame* f, RwUInt32 parentFlags)`: for `; f; f = f->next`: `cf = f->privateFlags | parentFlags`; if `cf & 4`:
  RwMatrixMultiply(&f->ltm, &f->modelling, &f->parent->ltm), `f->privateFlags &= 0xFB`; recurse(f->child, cf).
- librw: frame.cpp:syncLTMRecurse matches.

## Open questions
- None affecting behaviour. Note for porters: the stale-state cases (clone failure path, RwFrameDestroy leaving
  children with a dangling root) are original behaviour.
