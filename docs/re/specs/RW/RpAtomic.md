# RpAtomic (RenderWare 3.6 RpWorld plugin, statically linked) - full specs, writer B (RE-SPEC-RW)

All functions are plain `__cdecl` C (caller pops), unless stated. `Engine` = `[RwEngineInstance 0xc97b24]`
(RwGlobals*). Memory: `Engine+0x144` = freelist alloc `(freelist, hint)`, `Engine+0x148` = freelist free
`(freelist, ptr)`.

## Layouts used (verified against the bytes)

RpAtomic (registry 0x8d624c, base 0x70; matches docs/re/rw/objects.md):

| off | field | proving instruction |
|---|---|---|
| +0x00 u8 | object.type (1 = rpATOMIC) | 0x749c7e |
| +0x01 u8 | object.subType | 0x749c81 |
| +0x02 u8 | object.flags (5 = COLLISIONTEST\|RENDER at create) | 0x749c91 |
| +0x03 u8 | object.privateFlags, bit0 = world bounding sphere dirty | 0x749c95, 0x749324, 0x749453 |
| +0x04 ptr | object.parent (RwFrame*) | 0x749c84, 0x74933a |
| +0x08 RwLLLink | lFrame (in frame->objectList) | `_rwObjectHasFrameSetFrame` 0x804efb..0x804f2f |
| +0x10 fn | sync callback | 0x749c87 |
| +0x14 ptr | repEntry | 0x749c8e, 0x749dd1 |
| +0x18 ptr | geometry | 0x749ca6, 0x749d4a |
| +0x1c RwSphere | boundingSphere (center +0x1c/+0x20/+0x24, radius +0x28) | 0x74925b..0x7492b6 |
| +0x2c RwSphere | worldBoundingSphere (radius +0x38) | 0x74936d, 0x74945b |
| +0x3c ptr | clump | 0x749d01 |
| +0x40 RwLLLink | inClumpLink | 0x749cfb/0x749cfe |
| +0x48 fn | renderCallBack | 0x749ccd |
| +0x4c i32 | interpolator.flags (bit1 = rpINTERPOLATORDIRTYSPHERE) | 0x749cf4, 0x749d25, 0x74931a |
| +0x50 i16 | interpolator.startMorphTarget | 0x749208 |
| +0x52 i16 | interpolator.endMorphTarget | 0x749202 |
| +0x54 f32 | interpolator.time | 0x749cdb |
| +0x58 f32 | interpolator.recipTime | 0x749cde, 0x74923d |
| +0x5c f32 | interpolator.position | 0x749cf1, 0x749238 |
| +0x60 u16 | renderFrame (NOT initialised by RpAtomicCreate) | - |
| +0x64 RwLinkList | llWorldSectorsInAtomic | 0x749ce1, 0x749d07/0x749d09 |
| +0x6c ptr | pipeline (RxPipeline*) | 0x7491c5, 0x749d04 |

Plugin data used: CLUMPMODULE engine data at `Engine + [0xc924ac]` (+0 = atomic freelist, 0x749c5d / 0x749e2a);
WORLDOBJMODULE atomic extension at `atomic + [0xc925a8]` (+0 = RpWorld* the atomic is in, 0x751054);
PIPEMODULE engine data `Engine + [0xc9bc60]`: +0x3c default atomic pipeline (0x7491d7), +0x54 generic
atomic pipeline (0x75e2b4). RpGeometry: +0x18 numMorphTargets, +0x5c morphTarget array, stride 0x1c;
RpMorphTarget +0x4 boundingSphere (0x749d81), +0x10 = sphere radius (0x74924c). RwMatrix +0xc flags
(0x749377). RwFrame +0xa0 root (RwFrameDirty 0x7f0344).

Rights-to-render globals (written by 0x749120): 0xc924a8 u32 = atomic rights pluginID, 0xc92480 u32 = rights
extra data (names proposed; consumers are the `_rwPluginRegistryInvokeRights` path in the clump reader).

## 0x749120 cb_RIGHTTORENDER_RpAtomic_StreamRead  (catalogue: specified/REA, port level 3)
- Convention: cdecl plugin stream-read callback `RwStream* (RwStream* stream, i32 len, void* object, i32 offset, i32 size)`;
  only `stream` ([esp+4]) and `len` ([esp+8]) are used. Returns EAX = stream or NULL.
- Callees: RwStreamReadInt32 0x7ed540 `(stream, i32* dst, u32 numBytes=4)`.
- Globals written: 0xc924a8 (first int), 0xc92480 (second int).
- Behaviour: `if (!RwStreamReadInt32(stream, &g_rightsPluginId 0xc924a8, 4)) return NULL;` (0x74912d);
  `if (len == 8)` (0x74913b, exact equality) `{ if (!RwStreamReadInt32(stream, &g_rightsExtraData 0xc92480, 4)) return NULL; }`;
  return stream. The atomic object is not touched.
- Edge: with len other than 4/8 the remaining bytes are NOT skipped here (the registry reader handles chunk sizes).
- librw: clump.cpp:readAtomicRights differs: always reads 8 bytes into a static array, no len test.

## 0x7491c0 AtomicDefaultRenderCallBack  (catalogue: specified/REA, port level 5)
- Convention: cdecl `RpAtomic* (RpAtomic* atomic)`. Callers: 28 (CVisibilityPlugins render CBs, shadows).
- Callees: RxPipelineExecute 0x805710 `(pipeline, data, RwBool heapReset)`.
- Behaviour: `pipe = atomic->pipeline (+0x6c)`; if NULL `pipe = *(Engine + [0xc9bc60] + 0x3c)` (default atomic
  pipeline). `r = RxPipelineExecute(pipe, atomic, 1)`; return `r != NULL ? atomic : NULL` (neg/sbb/and, 0x7491e7).
- librw: clump.cpp:Atomic::defaultRenderCB differs: returns nothing; pipeline lookup via getPipeline.

## 0x7491f0 _rpAtomicResyncInterpolatedSphere  (catalogue: specified/REA, port level 0)
- Convention: cdecl `void (RpAtomic* atomic)` (EAX on return = atomic or garbage; callers ignore). Callers: 8
  (RpAtomicGetWorldBoundingSphere, frame sync 0x749d20, CVisibilityPlugins::IsAtomicVisible, ...).
- Behaviour:
  1. `geom = atomic->geometry (+0x18)`; if NULL return (0x7491fa) - flags are NOT cleared in that case.
  2. `s = (i16)interp.start (+0x50)`, `e = (i16)interp.end (+0x52)`, `n = geom->numMorphTargets (+0x18)`.
  3. If `s != e` (0x74920c, 16-bit compare) and `s < n` and `e < n` (signed, 0x74921b / 0x749226):
     `S = &geom->morphTarget[s]`, `E = &geom->morphTarget[e]` (stride 0x1c). x87 sequence:
     `f = interp.position(+0x5c) * interp.recipTime(+0x58)` (kept in ST);
     `bs.radius(+0x28) = (E.r - S.r) * f + S.r` (r at morphTarget+0x10; single expression, stored f32);
     `bs.x = E.x - S.x; bs.y = E.y - S.y; bs.z = E.z - S.z` (each stored f32 to +0x1c/+0x20/+0x24);
     then reload each: `tx = bs.x * f` (stored to +0x1c), `ty = bs.y * f` (stored f32 to a local and +0x20),
     `tz = bs.z * f` (local and +0x24);
     `bs.x = S.x + tx` where tx is the x87 register value (not reloaded from memory, 0x74929c);
     `bs.y = S.y + ty_f32`; `bs.z = S.z + tz_f32` (reloaded locals). Centre coordinates of a morph target
     are at morphTarget+0x4/+0x8/+0xc.
  4. Else (s == e, or an index out of range): `n` re-read (0x7492bb); if `s < n && e < n`: copy the 4 dwords of
     `morphTarget[s].boundingSphere` to atomic+0x1c; else copy `morphTarget[0].boundingSphere`.
  5. `interp.flags &= ~2` (0x74931a); `privateFlags |= 1` (0x749324).
- Edge: negative indices pass the signed `< n` test and index before the array (original bug kept).
- librw: absent (no interpolator; atomicSync has "TODO: interpolate").

## 0x749330 RpAtomicGetWorldBoundingSphere  (catalogue: specified/REA, port level 4)
- Convention: cdecl `const RwSphere* (RpAtomic* atomic)`, returns EAX = atomic+0x2c always.
- Callees: 0x7491f0, RwFrameDirty 0x7f0340 (returns `frame->root->privateFlags & 3`), RwFrameGetLTM 0x7f0990,
  RwV3dTransformPoint 0x7edd60 `(out, in, matrix)`, _rwSqrt 0x7edb30 (f32 arg on stack, result ST0).
- Behaviour:
  1. `frame = atomic->parent (+0x4)` read first (0x74933a). If `interp.flags` low byte & 2: call 0x7491f0.
  2. If `RwFrameDirty(frame) == 0` and `(privateFlags & 1) == 0`: return &ws (no work).
  3. `ltm = RwFrameGetLTM(frame)`; `RwV3dTransformPoint(&ws.center (+0x2c), &bs.center (+0x1c), ltm)`.
  4. If `(ltm->flags (+0xc) & 3) == 3` (rwMATRIXTYPEORTHONORMAL, 0x749380): `ws.radius = bs.radius` (raw dword
     copy), clear privateFlags bit0, return.
  5. Else: `r2 = rx*rx + ry*ry + rz*rz` from ltm->right (+0,+4,+8), computed as `(x*x + y*y) + z*z` and kept in an
     x87 register (not rounded to f32 memory); `u2` same from ltm->up (+0x10..+0x18), stored f32; `a2` from ltm->at
     (+0x20..+0x28), stored f32. `m = (u2 < a2) ? a2 : u2` (fcomp, C0; unordered also picks a2).
     If `!(r2 < m)` (C0 clear; 0x74940a): `v = (f32)r2`; else `v = (u2 < a2) ? a2 : u2` (compare redone).
     `ws.radius (+0x38) = _rwSqrt(v) * bs.radius (+0x28)` (ST0 * f32 memory, stored f32); clear privateFlags
     bit0; return &ws.
- librw: clump.cpp:Atomic::getWorldBoundingSphere differs: no interpolator resync, no scaling (radius copied
  unconditionally).

## 0x749c50 RpAtomicCreate  (catalogue: specified/REA, port level 2)
- Convention: cdecl `RpAtomic* (void)`. Callers: 6 (RpClumpCreateSpace, 0x72e270 GTA atomic reader, ...).
- Callees: Engine+0x144 alloc, _rwObjectHasFrameSetFrame 0x804ef0, _rwPluginRegistryInitObject 0x8086e0.
- Behaviour:
  1. `a = Engine->memoryAlloc(*(Engine + [0xc924ac]) /*atomic freelist*/, 0x30014)`; NULL -> return NULL
     (no error code set).
  2. `type=1, subType=0, parent=NULL, sync=0x749d20, repEntry=0, flags=5, privateFlags=1` (in that order).
  3. `_rwObjectHasFrameSetFrame(a, NULL)`; `privateFlags |= 1`.
  4. `geometry=0`; bs = {0,0,0,0} (+0x1c..+0x28), ws = {0,0,0,0} (+0x2c..+0x38); `renderCallBack = 0x7491c0`
     (a dead "if NULL store again" test follows at 0x749cb6).
  5. Interpolator: `time (+0x54) = recipTime (+0x58) = [imm 0x3f800000] 1.0f`; `start = end = 0`;
     `position (+0x5c) = 0`; `flags (+0x4c) = 3`.
  6. `inClumpLink = {0,0}`, `clump = 0`, `pipeline = 0`, `llWorldSectorsInAtomic` = empty (next = prev = &list).
  7. `_rwPluginRegistryInitObject(0x8d624c, a)` (result ignored); return a.
- Edge: renderFrame (+0x60) is left uninitialised (freelist memory).
- librw: clump.cpp:Atomic::create differs: no interpolator, ERR_ALLOC error, world ext set inline (here via plugin ctor 0x7507b0).

## 0x749d20 _rpAtomicDefaultFrameSyncCallBack  (catalogue: unknown/REA, port level 1)
- Name proposed: stored as atomic->sync by RpAtomicCreate (0x749c87); body = librw `atomicSync`.
- Convention: cdecl `RpAtomic* (RpAtomic* atomic)`; returns atomic.
- Behaviour: if `interp.flags & 2` (byte test 0x749d25) call 0x7491f0(atomic); `privateFlags |= 1`; return atomic.
- librw: clump.cpp:atomicSync differs (no interpolation).

## 0x749d40 RpAtomicSetGeometry  (catalogue: specified/REA, port level 6)
- Convention: cdecl `RpAtomic* (RpAtomic* atomic, RpGeometry* geom, u32 flags)`; returns atomic.
- Callees: _rpGeometryAddRef 0x74ccb0, RpGeometryDestroy 0x74ccc0, RpAtomicGetWorld 0x751050, RwFrameUpdateObjects 0x7f0910.
- Behaviour: if `geom == atomic->geometry` return atomic (no work at all, 0x749d4d). If geom: AddRef(geom)
  (before the old one is released). If old geometry: RpGeometryDestroy(old). `atomic->geometry = geom`.
  If `flags & 1` (rpATOMICSAMEBOUNDINGSPHERE, low byte test 0x749d73) return. If geom: copy
  `geom->morphTarget[0].boundingSphere` (4 dwords) to +0x1c. If `frame = atomic->parent` non-NULL and
  `RpAtomicGetWorld(atomic) != NULL`: RwFrameUpdateObjects(frame). Return atomic.
- librw: clump.cpp:Atomic::setGeometry differs: destroys old before addRef, no same-pointer early-out, updates frame without world test.

## 0x749dc0 RpAtomicDestroy  (catalogue: specified/REA, port level 6)
- Convention: cdecl `RwBool (RpAtomic* atomic)`; returns 1 always.
- Callees: _rwPluginRegistryDeInitObject 0x808740, RwResourcesFreeResEntry 0x807de0, RpGeometryDestroy, RpAtomicGetWorld,
  RwFrameUpdateObjects, _rwObjectHasFrameReleaseFrame 0x804f40, Engine+0x148 free.
- Behaviour (order): `DeInitObject(0x8d624c, atomic)` first (plugin dtors run before anything else, so the
  WORLDOBJ dtor 0x7509b0 removes it from the world); if repEntry: RwResourcesFreeResEntry(repEntry); if geometry:
  RpGeometryDestroy(geometry), `geometry = 0`, then if parent frame and RpAtomicGetWorld(atomic) != NULL:
  RwFrameUpdateObjects(frame); `_rwObjectHasFrameReleaseFrame(atomic)`; `Engine->memoryFree(*(Engine+[0xc924ac]), atomic)`.
- Edge: does not remove the atomic from its clump (caller's job).
- librw: clump.cpp:Atomic::destroy differs (no resEntry, no frame update).

## 0x749e60 RpAtomicClone  (catalogue: specified/REA, port level 6)
- Convention: cdecl `RpAtomic* (RpAtomic* src)`; returns clone or NULL (if RpAtomicCreate fails).
- Behaviour: `c = RpAtomicCreate()`; copy the four object header bytes (type, subType, flags, privateFlags) from
  src; `c->parent = NULL`; copy flags byte again; `_rwObjectHasFrameSetFrame(c, NULL)`; `privateFlags |= 1`.
  Then the RpAtomicSetGeometry(c, src->geometry, 0) logic inlined (same-pointer test against c->geometry, AddRef new,
  Destroy old, copy morphTarget[0] sphere, frame update test - frame is NULL so never taken). Then copy
  `src->boundingSphere` (4 dwords, overwrites the morph-target copy), `renderCallBack (+0x48)`, `pipeline (+0x6c)`,
  and the 5 interpolator dwords (+0x4c..+0x5c, rep movsd). Finally `_rwPluginRegistryCopyObject(0x8d624c, c, src)`.
- Not copied: worldBoundingSphere (stays zero), clump, repEntry, renderFrame.
- librw: clump.cpp:Atomic::clone differs (no bounding sphere / interpolator copy).

## 0x74bda0 RpAtomicRegisterPlugin  (catalogue: specified/REA, port level 2)
- cdecl `i32 (i32 size, u32 pluginID, ctor, dtor, copy)` -> `_rwPluginRegistryAddPlugin(0x8d624c, size, id, ctor, dtor, copy)`
  0x8084a0; returns its offset (-1 on failure).

## 0x74be00 RpAtomicRegisterPluginStream  (catalogue: specified/REA, port level 1)
- cdecl `i32 (u32 pluginID, readCB, writeCB, getSizeCB)` -> `_rwPluginRegistryAddPluginStream(0x8d624c, ...)` 0x8088e0.

## 0x74be30 RpAtomicSetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- cdecl `i32 (u32 pluginID, alwaysCB)` -> `_rwPluginRegistryAddPlgnStrmlwysCB(0x8d624c, id, cb)` 0x808920.

## 0x74be50 RpAtomicSetStreamRightsCallBack  (catalogue: specified/REA, port level 1)
- cdecl `i32 (u32 pluginID, rightsCB)` -> `_rwPluginRegistryAddPlgnStrmRightsCB(0x8d624c, id, cb)` 0x808950.

## 0x74bec0 RpAtomicGetPluginOffset  (catalogue: specified/REA, port level 1)
- cdecl `i32 (u32 pluginID)` -> `_rwPluginRegistryGetPluginOffset(0x8d624c, id)` 0x808470.

## 0x74bf20 RpAtomicSetFrame  (catalogue: specified/REA, port level 2)
- cdecl `RpAtomic* (RpAtomic* atomic, RwFrame* frame)`: `_rwObjectHasFrameSetFrame(atomic, frame)` (unlinks from
  the old frame's objectList, links at the head of `frame->objectList` (+0x90), calls RwFrameUpdateObjects(frame) if
  frame non-NULL, see 0x804ef0); then `privateFlags |= 1`; return atomic.

## 0x751050 RpAtomicGetWorld  (catalogue: specified/REA, port level 0)
- cdecl `RpWorld* (const RpAtomic* atomic)`: return `*(RpWorld**)(atomic + [0xc925a8])` (WORLDOBJMODULE atomic ext +0).

## 0x75e2a0 RpAtomicSetDefaultPipeline  (catalogue: specified/REA, port level 0)
- cdecl `RxPipeline* (RxPipeline* pipeline)`. `P = Engine + [0xc9bc60]` (PIPEMODULE data).
  If pipeline != NULL: `P+0x3c = pipeline`, return pipeline. Else `P+0x3c = P+0x54` (the generic/platform atomic
  pipeline) and return that value (EAX = the restored pipeline).
- Callers: _rpCreateD3D9AtomicAllInOnePipe 0x74eb80, 0x74ec00 (destroy).
