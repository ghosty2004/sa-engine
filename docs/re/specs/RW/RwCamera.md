# RwCamera (RW core, CAMERAMODULE) - RE-SPEC-RW writer C

RW 3.6 camera object as linked into gta_sa.exe 1.0 US. All `__cdecl`. gta-reversed hooks only `RwCameraBeginUpdate`
0x7ee190, `RwCameraEndUpdate` 0x7ee180, `RwCameraClear` 0x7ee340 and `RwCameraShowRaster` 0x7ee370 (not in this
file); the rest is thunks there. x87 runs at 24-bit precision (README §5), so register intermediates round like f32.

## Layouts

`RwCamera` (base 0x184, registry 0x8e222c):

| off | type | field | evidence |
|---|---|---|---|
| +0x00 | RwObject | type 4 (rwCAMERA), subType/flags/privateFlags 0, +0x04 parent = RwFrame* | 0x7ee520-0x7ee52c, 0x7ee1e5 |
| +0x08 | RwLLLink | lFrame (not initialised by create) | - |
| +0x10 | fn | sync callback = `_rwCameraFrameSyncCB` 0x7ee5a0 | 0x7ee52f |
| +0x14 | i32 | projectionType (1 perspective, 2 parallel) | 0x7ee57a, 0x7ee3b5, 0x7ee5a8 |
| +0x18 | fn | beginUpdate (default 0x7ef370) | 0x7ee536 |
| +0x1c | fn | endUpdate (default 0x7ef340) | 0x7ee53d |
| +0x20 | RwMatrix | viewMatrix (right +0x20, up +0x30, at +0x40, pos +0x50, flags +0x2c) | 0x7ee5b8, 0x7ee58c |
| +0x60 | RwRaster* | frameBuffer | 0x7ee574 |
| +0x64 | RwRaster* | zBuffer | 0x7ee577 |
| +0x68 | RwV2d | viewWindow | 0x7ee421/0x7ee42a |
| +0x70 | RwV2d | recipViewWindow | 0x7ee432/0x7ee43e |
| +0x78 | RwV2d | viewOffset | 0x7ee550/0x7ee553 |
| +0x80 | f32 | nearPlane | 0x7ee1da |
| +0x84 | f32 | farPlane | 0x7ee2aa |
| +0x88 | f32 | fogPlane | 0x7ee56a |
| +0x8c | f32 | zScale | 0x7ee26a |
| +0x90 | f32 | zShift | 0x7ee286 |
| +0x94 | RwFrustumPlane[6] | 0x14 each: normal V3d +0, distance +0xc, closestX/Y/Z u8 +0x10/+0x11/+0x12 | 0x7ee9b2-0x7ee9f2 |
| +0x10c | RwBBox | frustumBoundBox | 0x7ee71a |
| +0x124 | RwV3d[8] | frustumCorners | 0x7ee756, 0x7ee712 |

`RwFrame` LTM at frame+0x50 (right +0x50, up +0x60, at +0x70, pos +0x80). `RwGlobals`: +0x00 curCamera (0x7ef37a),
+0x18 / +0x1c `dOpenDevice.zBufferNear` / `zBufferFar` (RwDevice copy at +0x10; 0x7ee208/0x7ee20f), stdFunc[1]
CAMERABEGINUPDATE +0x4c (0x7ef38c), stdFunc[10] CAMERAENDUPDATE +0x70 (0x7ef34c).

CAMERAMODULE globals: base `RwEngineInstance + [0xc97960]`; +0x00 RwFreeList* camera freelist (0x7ee14d).
0xc97964 = open count; 0xc9793c = static freelist struct.

"closest" bytes: for each normal component c, the byte is `(bits(c) >> 31) + 1`, i.e. 0 if the sign bit is set
(including -0.0), else 1; computed from the f32 value stored in the plane (`sar; inc`, e.g. 0x7ee9cd-0x7ee9f0).

## 0x7ee110 cb_CAMERAMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- Open callback `(instance, offset, size)`: `[0xc97960] = offset`; module+0 = `RwFreeListCreateAndPreallocateSpace(
  [0x8e222c] (registry size), [0x8e2244] (4), 4, [0x8e2248] (1), 0xc9793c, 0x40005)`; NULL -> return NULL; else
  `[0xc97964]++`, return instance.

## 0x7ee1d0 RwCameraSetNearClipPlane  (catalogue: specified/REA, port level 1)
- `RwCamera* (RwCamera* c, RwReal nearPlane)`: `c->nearPlane = nearPlane` (0x7ee1da); `_rwCameraUpdateZShiftScale(c)`
  0x7ee200; if `c->frame` (+4) != NULL: `RwFrameUpdateObjects(frame)` 0x7f0910; return c. No validation.

## 0x7ee200 _rwCameraUpdateZShiftScale  (catalogue: specified/REA, port level 0)
- `void (RwCamera* c)` (cdecl; returns with the x87 stack empty).
- Let `zN = RwGlobals.zBufferNear` (+0x18), `zF = zBufferFar` (+0x1c) (both copied to f32 locals).
- If `c->projectionType != 2` (perspective; `sub ecx,2; je` 0x7ee223): `n = 1.0f [0x858624] / near`,
  `f = 1.0f [0x858624] / far` (f computed first, 0x7ee228; then n, 0x7ee234). If parallel: `n = near`, `f = far`.
- Then (0x7ee242-0x7ee286), with 24-bit x87:
  1. `d = (zF - zN) * 1e-4f [0x858fc4]`.
  2. `zF2 = zF - d` (stored to the f32 local); `zN2 = d + zN` (kept in a register).
  3. `scale = (zF2 - zN2) / (f - n)`; `c->zScale = scale` (+0x8c).
  4. `c->zShift = ((zF2 + zN2) - (n + f) * scale) * 0.5f [0x858b8c]` (+0x90). (Exact op order: `t = zF2 + zN2`;
     `u = n + f`; `u *= scale`; `t -= u`; `t *= 0.5`.)
- librw: `Camera::updateZShiftScale` (camera.cpp) computes the same formula; the exe's 1e-4 inset of the z range is the
  same; librw differs only in using float temporaries throughout.

## 0x7ee2a0 RwCameraSetFarClipPlane  (catalogue: specified/REA, port level 1)
- As 0x7ee1d0 with `c->farPlane` (+0x84, 0x7ee2aa).

## 0x7ee3a0 RwCameraSetProjection  (catalogue: specified/REA, port level 1)
- `RwCamera* (RwCamera* c, RwCameraProjection p)`: if `p <= 0 || p > 2` (signed, 0x7ee3a9/0x7ee3ae):
  `_rwerror(0x80000003, "Invalid projection type ..." [string at 0x8e224c])` + RwErrorSet({1, code}), return NULL.
  Else `c->projectionType = p`; `RwFrameUpdateObjects(frame)` if frame != NULL; THEN `_rwCameraUpdateZShiftScale(c)`
  (order differs from Set*ClipPlane, which update z first); return c.

## 0x7ee410 RwCameraSetViewWindow  (catalogue: specified/REA, port level 1)
- `RwCamera* (RwCamera* c, const RwV2d* vw)`: `c->viewWindow = *vw` (dword copies); `c->recipViewWindow.x =
  1.0f [0x858624] / c->viewWindow.x`; `.y = 1.0f / c->viewWindow.y` (division by the stored value, 0x7ee427/0x7ee43b);
  `RwFrameUpdateObjects(frame)` if frame; return c. No zero check (1/0 = inf).

## 0x7ee450 RwCameraRegisterPlugin  (catalogue: specified/REA, port level 2)
- `_rwPluginRegistryAddPlugin(0x8e222c, size, id, ctor, dtor, copy)` 0x8084a0; returns its result.

## 0x7ee4f0 RwCameraCreate  (catalogue: specified/REA, port level 1)
- `RwCamera* ()`. `c = memoryAlloc(module+0, 0x30005)` (RwGlobals+0x144); NULL -> return NULL.
- Init (0x7ee520-0x7ee58c): type 4, subType/flags/privateFlags 0, parent NULL; sync = 0x7ee5a0; beginUpdate = 0x7ef370;
  endUpdate = 0x7ef340; viewWindow = (1.0, 1.0); recipViewWindow = (1.0, 1.0); viewOffset = (0, 0);
  nearPlane = 0.05f (immediate 0x3d4ccccd); farPlane = 10.0f (0x41200000); fogPlane = 5.0f (0x40a00000);
  frameBuffer = zBuffer = NULL; projectionType = 1; `_rwCameraUpdateZShiftScale(c)`; viewMatrix.flags (+0x2c) = 0;
  `_rwPluginRegistryInitObject(0x8e222c, c)`; return c. The lFrame link, view matrix body, frustum planes, bbox and
  corners stay uninitialised until the first frame sync.
- librw: `Camera::create` uses the same defaults (near 0.05, far 10, fog 5).

## 0x7ee5a0 _rwCameraFrameSyncCB  (catalogue: specified/REA, port level 3)
- `RwObjectHasFrame* (RwCamera* c)` (sync callback, cdecl; returns c). Requires `c->frame != NULL` (no check).
- Notation: `L = frame LTM` with `r = L.right`, `u = L.up`, `a = L.at`, `p = L.pos`; `V = c->viewMatrix`.
- If projectionType == 1 (0x7ee5a8), build V inline (0x7ee5b2-0x7ee6f0):
  1. `s = recipViewWindow.x * -0.5f [0x858f40]`; `t0 = s*r.x` (register), `t1 = r.y*s`, `t2 = r.z*s` (f32 locals);
     `o = 0.5f [0x858b8c] - viewOffset.x*s` (f32 local). `X = a.x*o + t0`, `Y = a.y*o + t1`, `Z = a.z*o + t2`.
     `V.right.x = X; V.up.x = Y; V.at.x = Z`; `V.pos.x = 0.5f - (((p.y*Y + p.x*X) + p.z*Z) + o)`.
  2. `s' = recipViewWindow.y * -0.5f`; `t0' = u.x*s'`, `t1' = u.y*s'`, `t2' = u.z*s'`;
     `o' = viewOffset.y*s' + 0.5f` (note: `+`, while step 1 uses `0.5 - ...`). `X' = a.x*o' + t0'`, `Y' = a.y*o' + t1'`,
     `Z' = a.z*o' + t2'`; `V.right.y = X'; V.up.y = Y'; V.at.y = Z'`;
     `V.pos.y = 0.5f - (((p.y*Y' + p.x*X') + p.z*Z') + o')`.
  3. `V.right.z = a.x; V.up.z = a.y; V.at.z = a.z` (dword copies);
     `V.pos.z = -((a.y*p.y + p.x*a.x) + a.z*p.z)` (`fchs`, 0x7ee6ee).
  4. `RwMatrixOptimize(&V, NULL)` 0x7f17e0; `_rwCameraBuildPerspClipPlanes(c)` 0x7ee740.
- Else (parallel): `0x7ef1f0(c)` (parallel view matrix) then `0x7eede0(c)` (parallel frustum).
- Both paths: `RwBBoxCalculate(&c->frustumBoundBox, c->frustumCorners, 8)` 0x808f60; return c.
- librw: `cameraSync` builds the same matrix (inverse LTM scaled by the view window and offset); the exe keeps the
  odd sign asymmetry of the offsets (x: `0.5 - vo.x*s`, y: `vo.y*s' + 0.5` with s, s' both negative).

## 0x7ee740 _rwCameraBuildPerspClipPlanes  (catalogue: specified/REA, port level 1)
- `void (RwCamera* c)`; builds the 8 frustum corners (world space) and 6 planes for a perspective camera.
- Corners (0x7ee74a-0x7ee9a3); all products/sums in the order written, `*_s` = value stored to f32 memory:
  1. `ox = -viewOffset.x`; `O.x = viewOffset.y*u.x + ox*r.x`; `O.y = viewOffset.y*u.y + (r.y*ox)_s`;
     `O.z_s = viewOffset.y*u.z + (r.z*ox)_s`.
  2. `R = viewWindow.x * r` (`R.z` stored), `U = viewWindow.y * u` (all three stored).
  3. `D0 = ((a.x + R.x) + U.x, (a.y + R.y) + U.y, (a.z + R.z)_s + U.z)` written to corners[0];
     `D1 = D0 - 2R` -> corners[1]; `D2 = D1 - 2U` -> corners[2]; `D3 = D2 + 2R` -> corners[3]
     (2R, 2U computed as `x + x`; each D from the stored f32 of the previous one).
  4. For i = 0..3 (loop counter 4, 0x7ee8f6): `d = Di - O` (`d.z` stored); then
     `corners[i] = (p + O)` (stored) then `corners[i] = near*d + corners[i]` (per component, x, y, z);
     `corners[i+4] = (p + O)` then `corners[i+4] = far*d + corners[i+4]`.
- Planes (`P[k]` at c+0x94+0x14*k), each normal stored then closest bytes set:
  - P0 (far): normal = L.at (copy); `dist = (a.x*c4.x + c4.y*a.y) + c4.z*a.z`.
  - P1 (near): normal = -P0.normal (`fchs` of the stored values); `dist = (c0.y*n.y + n.x*c0.x) + c0.z*n.z`.
  - P2: `n = (c1 - c5) x (c6 - c5)`, i.e. with `P = c1 - c5` (stored), `E = c6 - c5`:
    `n = (P.y*E.z - E.y*P.z, E.x*P.z - E.z*P.x, E.y*P.x - E.x*P.y)`; `k = _rwInvSqrt((n.x*n.x + n.y*n.y) + n.z*n.z)` (12-bit table lookup, NOT an exact 1/sqrt; see RwMatrix.md)
    0x7edb90 (argument passed as f32); `n *= k` (x, y, z stored); `dist = (c1.y*n.y + c1.x*n.x) + c1.z*n.z`.
  - P3: `n = (c4 - c5) x (c1 - c5)` (same component pattern with Q = c4 - c5 first: `(Q.y*P.z - Q.z*P.y,
    Q.z*P.x - Q.x*P.z, Q.x*P.y - Q.y*P.x)`), normalised as P2; `dist = (c1.x*n.x + c1.y*n.y) + c1.z*n.z`.
  - P4: `A = c3 - c7` (stored), `E = c4 - c7`: `n = (A.y*E.z - E.y*A.z, E.x*A.z - E.z*A.x, E.y*A.x - E.x*A.y)`,
    normalised; `dist = (c3.y*n.y + c3.x*n.x) + c3.z*n.z`.
  - P5: `G = c6 - c7`: `n = (G.y*A.z - G.z*A.y, G.z*A.x - G.x*A.z, G.x*A.y - G.y*A.x)`, normalised;
    `dist = (c3.y*n.y + c3.x*n.x) + c3.z*n.z`.
- librw: `buildPlanes`/`buildClipPersp` produce equivalent planes; librw computes P4/P5 the same way but its corner
  construction and cross-product operand order differ (do not port from librw).
- Name: rw-names calls this `_rwCameraBuildPerspClipPlanes` (fits); it also builds the corners.

## 0x7eede0 _rwCameraBuildFrustumBBox  (catalogue: unknown/REA, port level 1)
- Proposed name: `_rwCameraBuildParallelClipPlanes` (evidence: only called from the parallel branch of 0x7ee5a0 at
  0x7ee709, builds corners in camera space then transforms them; the bbox itself is computed by the caller with
  RwBBoxCalculate 0x7ee722). rw-names' `_rwCameraBuildFrustumBBox` does not match the body.
- `void (RwCamera* c)`. Corners in camera space (0x7eedea-0x7eeecf), `A = viewWindow.x`, `B = viewWindow.y`,
  `N = near`, `F = far`: `k1 = 1.0f [0x858624] - N`, `k2 = 1.0f - F` (stored); `m = k1*(-viewOffset.x)`,
  `n = k1*viewOffset.y`, `m2 = k2*(-viewOffset.x)`, `n2 = k2*viewOffset.y`:
  c0 = (m + A, B + n, N), c1 = (m - A, B + n, N), c2 = (m - A, n - B, N), c3 = (m + A, n - B, N),
  c4 = (m2 + A, B + n2, F), c5 = (m2 - A, B + n2, F), c6 = (m2 - A, n2 - B, F), c7 = (m2 + A, n2 - B, F).
  Then `RwV3dTransformPoints(corners, corners, 8, LTM)` 0x7edd90 (the installed kernel, README §5).
- Planes: P0 = (L.at, `dist = (n.y*c4.y + c4.x*n.x) + c4.z*n.z`), P1 = (-L.at, `dist = (c0.y*n.y + n.x*c0.x) + c0.z*n.z`),
  P2 and P3 exactly as in 0x7ee740 (same edges c1-c5, c6-c5, c4-c5 and the same through-point c1; P3 dist order
  `(c1.y*n.y + c1.x*n.x) + c1.z*n.z`). Different from the perspective case: P4 = -P2 normal (`fchs` of the stored
  values) with `dist = (c3.x*n.x + c3.y*n.y) + c3.z*n.z`, and P5 = -P3 normal with the same dist form through c3
  (0x7ef13e-0x7ef1e0). Closest bytes as above.

## 0x7ef1f0 _rwCameraBuildParallelClipPlanes  (catalogue: unknown/REA, port level 2)
- Proposed name: `_rwCameraBuildParallelViewMatrix` (evidence: body builds `c->viewMatrix` like the perspective
  inline block of 0x7ee5a0 and ends with RwMatrixOptimize; no planes are touched).
- `RwCamera* (RwCamera* c)`; returns c. Same as 0x7ee5a0 steps 1-4 (without the clip-plane call) except:
  `o = -(viewOffset.x*s)` (`fchs`, no 0.5); `o' = viewOffset.y*s'` (no 0.5); the position sums are written
  `((p.x*X + p.y*Y) + p.z*Z) + o` (same value), still finished with `0.5f [0x858b8c] - (...)`;
  `V.pos.z = -((p.x*a.x + a.y*p.y) + a.z*p.z)`. Then `RwMatrixOptimize(&V, NULL)`.

## 0x7ef340 _rwCameraDefaultEndUpdate  (catalogue: specified/REA, port level 0)
- `RwCamera* (RwCamera* c)`: `stdFunc[10](NULL, c, 0)` (RwGlobals+0x70); on success `RwGlobals.curCamera = NULL`, return
  c; else return NULL (curCamera unchanged).

## 0x7ef370 _rwCameraDefaultBeginUpdate  (catalogue: specified/REA, port level 3)
- `RwCamera* (RwCamera* c)`: `RwGlobals.curCamera = c` (first); `_rwFrameSyncDirty()` 0x809550; `stdFunc[1](NULL, c, 0)`
  (RwGlobals+0x4c = `_rwD3D9CameraBeginUpdate` 0x7f8f20 in this build); failure -> return NULL (curCamera stays c);
  success -> `_rwCameraBeginUpdateHook(c)` 0x807c80 (empty), return c.

## 0x807c80 _rwCameraBeginUpdateHook  (catalogue: unknown/REA, port level 0)
- A single `ret`: empty cdecl function (debug/profiling hook compiled out).
