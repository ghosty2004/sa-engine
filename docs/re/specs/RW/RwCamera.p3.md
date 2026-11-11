# RwCamera: destroy, clone, frustum sphere test, streaming (RE-SPEC-RW-3 writer B)

All `__cdecl`. Pass-1 file `RwCamera.md` (read-only) covers create/update/projection.

## Layout (RwCamera, base 0x184, registry `[0x8e222c]`)
- `+0x4` parent frame (RwObjectHasFrame; 0x7ef3c3), `+0x14` projectionType (0x7ef42e, 0x808d92), `+0x18` beginUpdate,
  `+0x1c` endUpdate (0x7ef3cd-0x7ef3d7), `+0x60` frameBuffer, `+0x64` zBuffer (0x7ef422-0x7ef42b), `+0x68/+0x6c` viewWindow,
  `+0x70/+0x74` recipViewWindow (0x7ef3e6-0x7ef3ef), `+0x78/+0x7c` viewOffset (0x7ee1ab, 0x7ee1b4), `+0x80` nearPlane,
  `+0x84` farPlane, `+0x88` fogPlane (0x808d5c-0x808d7d), `+0x94` frustum planes, 6 x 0x14 bytes `{RwV3d normal +0, RwReal
  distance +0xc, u8 closest[3] +0x10}` (0x7ee2df, stride 0x7ee325).
- Camera module data: engine + `[0xc97960]`; its +0x0 is the camera freelist (0x7ee4d2).
- Stream struct (chunk STRUCT, 0x20 bytes): `{RwV2d viewWindow +0, RwV2d viewOffset +8, RwReal near +0x10, far +0x14, fog +0x18,
  RwInt32 projection +0x1c}` (0x808d5c-0x808da1).
- Error codes: 0x80000004 = E_RW_BADVERSION (0x808ebc).

## 0x7ee1a0 RwCameraSetViewOffset  (catalogue: specified/REA, port level 1)
- `RwCamera* (RwCamera* cam, const RwV2d* off)`: cam+0x78 = off->x, cam+0x7c = off->y (integer moves, 0x7ee1ab/0x7ee1b4); if
  cam+0x4 frame != NULL `RwFrameUpdateObjects(frame)` 0x7f0910 (0x7ee1bc). Returns cam.
- librw: camera.cpp `Camera::setViewOffset` matches.

## 0x7ee2d0 RwCameraFrustumTestSphere  (catalogue: specified/REA, port level 0)
- `RwFrustumTestResult (const RwCamera* cam, const RwSphere* s /*center +0, radius +0xc*/)`. result = 2 (INSIDE, 0x7ee2da).
- For the 6 planes at cam+0x94 (counter esi = 6, `dec` at 0x7ee2f0, stride 0x14): x87, f32 loads:
  `d = n.y*c.y; d = n.x*c.x + d; d = d + n.z*c.z; d = d - plane.distance` (order 0x7ee2ea-0x7ee2ff).
  `fcom radius` (0x7ee302): if `d > radius` (C0 = C3 = 0) -> return 0 OUTSIDE immediately (0x7ee331). NaN d: unordered sets
  C0/C3, not outside.
  Else compare `d` with `-radius` (`fcompp` st0 = d, 0x7ee315): if `d > -radius` -> result = 1 BOUNDARY (0x7ee320); equal or
  less (or NaN) keeps the result. All 6 planes are always visited unless OUTSIDE.
- librw: camera.cpp `Camera::frustumTestSphere` matches (librw computes x+y+z dot order; the exe sums y*y, then +x*x, then +z*z).

## 0x7ee4b0 RwCameraDestroy  (catalogue: specified/REA, port level 1)
- `RwBool (RwCamera* cam)`: `_rwPluginRegistryDeInitObject(&[0x8e222c], cam)` 0x808740; `_rwObjectHasFrameReleaseFrame(cam)`
  0x804f40; `RwEngineInstance->memoryFree (+0x148)(*(engine + [0xc97960]), cam)` (0x7ee4d6). Returns 1. No check that the camera is
  still in a clump/world, no NULL check.
- librw: camera.cpp `Camera::destroy` matches (asserts aside).

## 0x7ef3b0 RwCameraClone  (catalogue: specified/REA, port level 2)
- `RwCamera* (RwCamera* src)`: `dst = RwCameraCreate()` 0x7ee4f0; NULL -> return NULL (0x7ef3ba).
- `_rwObjectHasFrameSetFrame(dst, src+0x4)` 0x804ef0 (the clone shares the source frame). Copy dwords in this order:
  +0x18, +0x1c, +0x68, +0x6c, +0x70, +0x74, +0x78, +0x7c, +0x80, +0x84, +0x88, +0x60, +0x64, +0x14 (0x7ef3cd-0x7ef431).
  `_rwCameraUpdateZShiftScale(dst)` 0x7ee200; `_rwPluginRegistryCopyObject(&[0x8e222c], dst, src)` 0x808770. Returns dst.
- Not copied: view matrix, zScale/zShift (recomputed), frustum planes/bbox/corners (recomputed on the next frame sync).
- librw: camera.cpp `Camera::clone` differs: librw also copies object flags and adds the clone to the source's world; it does not
  copy beginUpdate/endUpdate or recipViewWindow and does not recompute z shift/scale.

## 0x808ce0 RwCameraStreamGetSize  (catalogue: specified/REA, port level 1)
- `RwUInt32 (const RwCamera* cam)`: `_rwPluginRegistryGetSize(&[0x8e222c], cam) + 0x38` (0x808cf2; 0x20 struct + two 12-byte
  chunk headers).
- librw: camera.cpp `streamGetSize` matches.

## 0x808d00 RwCameraStreamWrite  (catalogue: specified/REA, port level 4)
- `const RwCamera* (const RwCamera* cam, RwStream* s)`.
- `_rwStreamWriteVersionedChunkHeader(s, 5 rwID_CAMERA, GetSize+0x38, 0x36003, 0xffff)` 0x808d29; 0 -> return NULL. Then
  `(s, 1 rwID_STRUCT, 0x20, 0x36003, 0xffff)` 0x808d4a; 0 -> NULL.
- Build the struct: viewWindow (+0x68/+0x6c) and viewOffset (+0x78/+0x7c) by integer moves; near/far/fog through x87
  `fld/fstp` f32 (0x808d5c-0x808d99; bit-exact except signalling NaNs); projection = +0x14. `RwStreamWrite(s, &buf, 0x20)` 0x808da5;
  0 -> NULL. `_rwPluginRegistryWriteDataChunks(&[0x8e222c], s, cam)` 0x808dbe; returns `cam` if non-zero else NULL (neg/sbb/and).
- No byte-swap / endian conversion of the struct (written raw, little-endian host).
- librw: camera.cpp `Camera::streamWrite` matches in layout (librw uses `writeStruct32`).

## 0x808de0 RwCameraStreamRead  (catalogue: specified/REA, port level 5)
- `RwCamera* (RwStream* s)`. `RwStreamFindChunk(s, 1, &size, &version)` 0x808df6; 0 -> NULL.
- Version check (0x808e0a-0x808e1a): `version < 0x34000` or `> 0x36003` (unsigned) -> `RwErrorSet({1, _rwerror(0x80000004)})`
  (0x808ebc-0x808ed7), return NULL.
- Zero the 0x20-byte local struct (`rep stosd` ecx = 8, 0x808e31), then `RwStreamRead(s, &buf, size)` 0x808e39 with the CHUNK
  size (not 0x20): a chunk larger than 0x20 overflows the stack buffer (original bug; real files have 0x20). Read count != size
  -> NULL.
- `cam = RwCameraCreate()`; NULL -> NULL. `_rwPluginRegistryReadDataChunks(&[0x8e222c], s, cam)` 0x808e64; 0 -> return NULL
  WITHOUT destroying cam (leak, 0x808e6e).
- Apply in this order: `RwCameraSetViewWindow(cam, &buf.viewWindow)` 0x7ee410, `RwCameraSetViewOffset(cam, &buf.viewOffset)`
  0x7ee1a0, `RwCameraSetNearClipPlane(cam, near)` 0x7ee1d0, `RwCameraSetFarClipPlane(cam, far)` 0x7ee2a0, `cam+0x88 = fog`
  (direct store, 0x808ea0), `RwCameraSetProjection(cam, proj)` 0x7ee3a0. Return values of the setters are ignored. Returns cam.
- librw: camera.cpp `Camera::streamRead` differs: no version check, reads sizeof(struct) not the chunk size, stores fields
  directly (no setters, so no recip/zshift update), destroys the camera on plugin read failure, and reads plugins after the fields.

## 0x808ef0 RwCameraChunkInfoRead  (catalogue: specified/REA, port level 4)
- `RwCameraChunkInfo* (RwStream* s, RwCameraChunkInfo* info, RwInt32* bytesRead)`: `RwStreamFindChunk(s, 1, &size, NULL)`; 0 ->
  NULL. Zero 8 dwords of `info` (0x808f21), `RwStreamRead(s, info, 0x20)`; != 0x20 -> NULL (0x808f2b). `*bytesRead = size + 0xc`
  (0x808f3e/0x808f46); `RwStreamSkip(s, size - 0x20)` 0x7ecd00 (result ignored). Returns info. No version check.
- librw: absent.
