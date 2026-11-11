# CCamera (CAM) - full specs (group B)

Layout source: gr `Camera.h` (`VALIDATE_SIZE(CCamera, 0xD78)`), offsets recomputed from the member list (anchors checked
against the bytes: `m_nActiveCam` +0x59, `m_aCams[3]` +0x174 (CCam stride 0x238, `m_nMode` at cam+0xC => this+0x180),
`m_fTrackLinearEndTime` +0xC60, `m_fMoveLinearEndTime` +0xCC0). `TheCamera` = 0xB6F028. CPlaceable part: +0x04 placement
pos, +0x14 `m_matrix` (CMatrix*; position = `m_matrix ? m_matrix->pos (+0x30) : placement.pos`). CMatrix rows: right +0x00,
up (the camera "forward") +0x10, at +0x20, pos +0x30. CCam fields: see `CCam.md` (group A) / gr `Cam.h`
(m_vecFront +0x190, m_vecSource +0x19C, m_vecUp +0x1B4, m_fFOV +0xB4, m_fVerticalAngle +0xAC, m_pCamTargetEntity +0x21C).
x87 notes: compares written as `fcom/fcomp ...; test ah,0x41; je X` jump when ST0 > operand; `test ah,5; jp X` jump when
ST0 >= operand or NaN; `test ah,0x44; jp X` jump when not equal or NaN. `fsin`/`fcos`/`fpatan`/`fptan` results are full
precision; arithmetic is at the in-game 24-bit precision control.

## 0x420C40 CCamera::IsSphereVisible (with matrix)  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall bool(const CVector& center [esp+4], float radius [esp+8], RwMatrix* m [esp+0xC])`, result AL,
  `ret 0xC`. Catalogue name "IsSphereVisible-" = this overload.
- Callers (13): CCamera::IsSphereVisible 0x420D40 (2 sites), CEntity::GetIsOnScreen 0x534540, CPickups::PictureTaken,
  CPlantMgr::Render, CWaterLevel::RenderHighDetailWater*, C3dMarkers::Render, CRope::Render, ...
- Callees: `RwV3dTransformPoints(out, in, 1, m)` 0x7EDD90 (cdecl, in-place on a local copy).
- Globals: `CDraw::ms_fNearClipZ` 0xC3EFA0, `CDraw::ms_fFarClipZ` 0xC3EF9C (f32, read).
- Fields: `m_avecFrustumNormals[4]` (CVector, +0xB24): [0] +0xB24, [1] +0xB30, [2] +0xB3C, [3] +0xB48.
- Behaviour:
  1. `p = *center` (local copy), `RwV3dTransformPoints(&p, &p, 1, m)` -> camera space (y = depth).
  2. `if (p.y + radius < ms_fNearClipZ)` return false (0x420C74-0x420C8A, `test ah,5; jnp`; NaN continues).
  3. `if (p.y - radius > ms_fFarClipZ)` return false (0x420C90-0x420CA3, `test ah,0x41; je`; NaN continues).
  4. `if (p.x*N0.x + p.y*N0.y > radius)` return false (N0 = +0xB24/+0xB28; x product first).
  5. `if (p.x*N1.x + p.y*N1.y > radius)` return false (+0xB30/+0xB34).
  6. `if (p.z*N2.z + p.y*N2.y > radius)` return false (+0xB44/+0xB40; z product computed first, then y, then added).
  7. `if (p.z*N3.z + p.y*N3.y > radius)` return false (+0xB50/+0xB4C); else return true (0x420D2B `jne` -> AL = 1).
  Each plane test: `fcomp radius; test ah,0x41; je false` -> "sum > radius" fails; equal, less and NaN pass.
  The sums are x87 values (not stored), rounded to f32 precision by the 24-bit precision control.
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethodAndReturn` only. No code to compare.
- Open questions: none.

## 0x420D40 CCamera::IsSphereVisible  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall bool(const CVector& center [esp+4], float radius [esp+8])`, AL, `ret 8`.
- Callers (21): CCarCtrl::GenerateCarCreationCoors2, CCarCtrl::CreatePoliceChase, CGangWars::CreateAttackWave,
  CPopulation::AddToPopulation, CPedGroupPlacer::Place*, script commands, ...
- Callees: CCamera::IsSphereVisible(center, radius, m) 0x420C40.
- Behaviour: `if (IsSphereVisible(center, radius, (RwMatrix*)&m_mMatInverse /*+0xA4C*/)) return true;`
  `if (m_bMirrorActive /*+0x970*/) return IsSphereVisible(center, radius, (RwMatrix*)&m_mMatMirrorInverse /*+0xA94*/);`
  `return false;` (0x420D4D-0x420D89).
- gta-reversed: `Camera.cpp:1233` (no hook), matches.
- Open questions: none.

## 0x474870 CCamera::VectorTrackRunning  (catalogue: specified/REA, port level 0)
- `__thiscall bool()`, result EAX = 1/0, plain `ret`. Caller: CRunningScript::ProcessCommands2300To2399 0x4762D0.
- `t = (float)CTimer::m_snTimeInMilliseconds` (0xB7CB84, `fild` of the int32; if negative add 4294967296.0f [0x858C54], i.e.
  unsigned conversion); `return t <= m_fTrackLinearEndTime (+0xC60)` (`fcomp; test ah,0x41; jp` -> false when greater or NaN).
- gta-reversed: `Camera.cpp:1558` `GetTimeInMS() <= m_fTrackLinearEndTime`, matches.

## 0x4748A0 CCamera::VectorMoveRunning  (catalogue: specified/REA, port level 0)
- Same as 0x474870 with `m_fMoveLinearEndTime (+0xCC0)`: `return (float)(uint32)time <= m_fMoveLinearEndTime`.
- gta-reversed: `Camera.cpp:1553`, matches.

## 0x50AFA0 CCamera::CopyCameraMatrixToRWCam  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void(bool skipOldMatrix [esp+4])` (gr name `bUpdateMatrix`; the mirror helpers pass true,
  Process passes false), `ret 4`.
- Callers: CCamera::SetCameraUpForMirror 0x51A560, CCamera::RestoreCameraAfterMirror 0x51A5A0, CCamera::Process 0x52B730.
- Callees: `CMatrix::UpdateMatrix(RwMatrix*)` 0x59AD20 (thiscall, CMatrix <- RwMatrix), `RwMatrixUpdate` 0x7F18A0,
  `RwFrameUpdateObjects` 0x7F0910, `RwFrameOrthoNormalize` 0x7F1170 (cdecl).
- Globals: function statics (catalogue names them around "s_PreviousSource"): `prevRight` 0xB6FF90, `prevUp` 0xB6FF9C,
  `prevAt` 0xB6FFA8, `prevPos` 0xB6FFB4 (CVector each), init-guard bits 0..3 of dword 0xB6FFC0 (bit0 prevPos, bit1 prevAt,
  bit2 prevUp, bit3 prevRight; each initialised to (-99999.0f) x3, bits 0xC7C34F80); thresholds f32 .data
  0x8CCC7C = 1e-5 (pos) and 0x8CCC78 = 1e-5 (directions) (catalogue "v3d_8CCC54+0x28/+0x24").
- Fields: `m_pRwCamera` +0x954 (frame = `RwCamera->object.object.parent` at +4), `m_mCameraMatrix` +0x974,
  `m_mCameraMatrixOld` +0x9BC, `m_bResetOldMatrix` +0x971.
- Behaviour (`frame = m_pRwCamera->parent`, `M = &frame->modelling` (frame+0x10), RwMatrix rows right +0, up +0x10, at +0x20, pos +0x30):
  1. If `!skipOldMatrix`: `m_mCameraMatrixOld.UpdateMatrix(M)` (old = the previous RW matrix).
  2. Copy (dword moves): `M.pos = camMat.pos (+0x9A4)`, `M.at = camMat.up (+0x984)` (forward), `M.up = camMat.at (+0x994)`,
     `M.right = camMat.right (+0x974)`.
  3. Initialise the 4 statics once (guard bits) to (-99999, -99999, -99999).
  4. Snap tiny changes back (in this order):
     - pos: `d = prevPos - M.pos` (component-wise, x87); `if (k*k > (d.z*d.z + d.y*d.y) + d.x*d.x)` with k = [0x8CCC7C]
       (`fcompp`, ST0 = k*k, `test ah,0x41; jne` -> snap only when strictly smaller; NaN no snap): `M.pos = prevPos`.
     - `kk = k2*k2` (k2 = [0x8CCC78], stored f32). at: `if ((dz*dz + dy*dy) + dx*dx < kk)` (d = prevAt - M.at;
       `test ah,5; jp`) `M.at = prevAt`. Same for up (prevUp) and right (prevRight).
  5. `prevPos = M.pos; prevAt = M.at; prevUp = M.up; prevRight = M.right` (after snapping).
  6. `RwMatrixUpdate(M); RwFrameUpdateObjects(frame); RwFrameOrthoNormalize(frame)`.
  7. If `m_bResetOldMatrix && !skipOldMatrix`: `m_mCameraMatrixOld.UpdateMatrix(M); m_bResetOldMatrix = false`.
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod`.
- Open questions: none.

## 0x50C910 CCamera::TakeControlAttachToEntity  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall void(CEntity* target [esp+4], CEntity* attached [esp+8], CVector* camOffset [esp+0xC],
  CVector* camLookAt [esp+0x10], float camAngle [esp+0x14], int16/eSwitchType switchType [esp+0x18], int32 who [esp+0x1C])`,
  `ret 0x1C`. The `attached` slot is overwritten with the default entity.
- Callers: CRunningScript::ProcessCommands1600To1699 0x493FE0 (5 sites).
- Callees: CWorld::FindPlayerVehicle(-1, false) 0x56E0D0 (twice), CWorld::FindPlayerPed(-1) 0x56E210 (result unused),
  CEntity::CleanUpOldReference(CEntity** ) 0x571A00 / CEntity::RegisterReference(CEntity**) 0x571B70 (thiscall on the
  entity), 0x509760 = `bool operator!=(const CVector* a, const CVector* b)` (cdecl; true if any component differs,
  `fcomp` + `test ah,0x44; jp`, NaN counts as different).
- Globals: `CWorld::PlayerInFocus` 0xB7CD74 (u8), `CWorld::Players` 0xB7CD98 (stride 0x190, +0 m_pPed).
- Fields: `m_nWhoIsInControlOfTheCamera` +0xB0 (u32), `m_bLookingAtPlayer` +0x2B, `m_bLookingAtVector` +0x2C,
  `m_bStartInterScript` +0x32, `m_vecAttachedCamOffset` +0x938, `m_vecAttachedCamLookAt` +0x944, `m_fAttachedCamAngle` +0x950,
  `m_pTargetEntity` +0x958, `m_pAttachedEntity` +0x95C, `m_nModeToGoTo` +0xC38 (u16), `m_nTypeOfSwitch` +0xC3C (u16).
- Behaviour:
  1. `if (who == 2 && m_nWhoIsInControlOfTheCamera == 1) return;` then `m_nWhoIsInControlOfTheCamera = who`.
  2. `if (!attached) attached = FindPlayerVehicle(-1, false) ? FindPlayerVehicle(-1, false) : Players[PlayerInFocus].m_pPed;`
  3. If `target`: `if (m_pTargetEntity) m_pTargetEntity->CleanUpOldReference(&m_pTargetEntity); m_pTargetEntity = target;
     target->RegisterReference(&m_pTargetEntity); m_bLookingAtVector = false;`
     else: `m_bLookingAtVector = true;` `m_vecAttachedCamLookAt = (*camLookAt != *camOffset) ? *camLookAt : (0,0,0)`
     (operator!= called as `(camLookAt, camOffset)`; copies as dwords). m_pTargetEntity is left unchanged.
  4. `m_vecAttachedCamOffset = (camOffset->x != 0 || camOffset->y != 0 || camOffset->z != 0) ? *camOffset : (0, 0, 2.0f)`
     (each test `fcomp 0.0f [0x858B50]; test ah,0x44; jp` -> NaN counts as non-zero).
  5. `m_fAttachedCamAngle = camAngle` (dword copy).
  6. `if (m_pAttachedEntity) m_pAttachedEntity->CleanUpOldReference(&m_pAttachedEntity); m_pAttachedEntity = attached;
     attached->RegisterReference(&m_pAttachedEntity);` (no null check on `attached`).
  7. `m_nModeToGoTo = 47` (0x2F, MODE_ATTACHCAM); `m_nTypeOfSwitch = (u16)switchType`; `m_bLookingAtPlayer = false`;
     `m_bStartInterScript = true`; `FindPlayerPed(-1)` (discarded).
- Oddities: in step 3 a look-at vector equal to the offset is replaced by zero; `attached` is dereferenced even when it is
  null (no player ped and no player vehicle).
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod`.
- Open questions: none.

## 0x50CB90 CCamera::SetNearClipBasedOnPedCollision  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall void(float distSq [esp+4])`, `ret 4`; `this` (ECX) is not used. No static callers in the
  catalogue (reached through code not yet resolved, or dead).
- Callees: `RwCameraSetNearClipPlane(Scene.m_pRwCamera (0xC1703C), clip)` 0x7EE1D0 (cdecl).
- Globals: `gpCamColVars` 0xB6FE88 (float*; element [4] at +0x10 = minimum near clip of the current col-var set),
  f32 0xB6EC68 (.data, unnamed, labelled "s_ExitCam+0xC" by the catalogue - divisor, value set at run time),
  f32 0x8CCC84 (.data, initial 0.25), 0.3f [0x858C24].
- Behaviour: `A = gpCamColVars[4]`; `n = ((0.3f - A) * ((sqrt(distSq) / g_B6EC68) * g_8CCC84)) + A`
  (exact x87 order: `sqrt`, `/ g_B6EC68`, `* g_8CCC84`, then `(0.3 - A) *` that, then `+ A`); stored f32;
  `clip = (n < A) ? A : n` (`fcomp st(1); test ah,5; jp`: NaN keeps n); `RwCameraSetNearClipPlane(Scene.camera, clip)`.
- gta-reversed: hooked `.Reversed = false`, `plugin::Call`.
- Open questions: writer/meaning of 0xB6EC68 (0x50CB9E); not determinable from this function.

## 0x50CFA0 CCamera::HandleCameraMotionForDucking  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall void(CPed* ped [esp+4], CVector* source [esp+8], CVector* target [esp+0xC], bool noUpdate [esp+0x10])`,
  `ret 0x10`. The `ped` slot is reused for the goal value.
- Callers: CCam::LookBehind 0x520690, CCam::Process_FollowPed_SA 0x522D40.
- Callees: `CPedIntelligence::GetTaskDuck(bool)` 0x6010A0 (thiscall on `ped->m_pIntelligence` (+0x47C), arg 1) -
  **called twice**, the first result is discarded (0x50CFB0).
- Globals: f32 0x8CCB94 (.data, initial 0.3; catalogue "gCurDistForCam+0x10"), f32 0x8CCB98 (.data, initial 0.3),
  `CTimer::ms_fTimeStep` 0xB7CB5C; constants 1e-6f [0x8630EC], 1.0f [0x858624], 0.5f [0x858B8C], 0.1f [0x858B1C].
- Fields: `this->m_fDuckCamMotionFactor` (+0xC54); ped: `m_pIntelligence` +0x47C, `m_nPedFlags` dword +0x46C bit 26
  (0x4000000, bIsDucking), `m_vecMoveSpeed` +0x44/+0x48/+0x4C; duck task +0x19 (`m_bIsAborting`, int8).
- Behaviour:
  1. `goal = 0.0f`. `duck = GetTaskDuck(true)` (second call).
  2. If `duck && (ped->m_nPedFlags & 0x4000000) && duck->m_bIsAborting == 0`:
     `sq = (v.x*v.x + v.y*v.y) + v.z*v.z` (v = move speed; x first); if `sq <= 1e-6f` (or NaN; `test ah,0x41; jne`)
     `goal = g_8CCB94 - 1.0f` else `goal = g_8CCB98 - 0.5f` (both kept on the x87 stack, not stored).
  3. If `!noUpdate`: `f = m_fDuckCamMotionFactor; f = (goal - f) * (ms_fTimeStep * 0.1f) + f` (order: `goal - f`,
     `ts * 0.1`, multiply, `+ f`), stored f32.
  4. If `source`: `source->z = f + source->z`; if `target`: `target->z = f + target->z` (f re-read from the field).
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod`.
- Open questions: none.

## 0x50D090 CCamera::HandleCameraMotionForDuckingDuringAim  (catalogue: specified/REA, port level 2)
- Convention and arguments as 0x50CFA0, `ret 0x10`. Caller: CCam::Process_AimWeapon 0x521500.
- Same structure as 0x50CFA0 with these differences:
  - goal when the duck condition holds: `-0.35f` [0x8630F0] (0x50D0F6). The move-speed magnitude is still computed and
    compared with 1e-6f, but the comparison result is never used (dead `fcomp`/`fnstsw`, 0x50D0EA-0x50D0FC).
  - field `m_fDuckAimCamMotionFactor` (+0xC58); rate `ms_fTimeStep * 0.13f` [0x859020].
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod` (comment "arg5 always used as false").
- Open questions: none.

## 0x514030 CCamera::AvoidTheGeometry  (catalogue: specified/REA, port level 8)
- Convention: `__thiscall void(const CVector* from [esp+4], const CVector* target [esp+8], CVector* out [esp+0xC], float fovDeg [esp+0x10])`,
  `ret 0x10` (gr names: arg2, arg3, arg4, FOV).
- Callers: CCam::Process_SpecialFixedForSyphon 0x517500, CCam::ProcessArrestCamOne 0x518500 (3), CCam::ProcessPedsDeadBaby
  0x519250, CCam::Process_Cam_TwoPlayer_InCarAndShooting 0x519810.
- Callees: CGeneral::GetATanOfXY(float x, float y) 0x53CC70 (cdecl, ST0), CVector::Normalise 0x59C910,
  CWorld::ProcessLineOfSight 0x56BA00 (12 args), CWorld::FindPlayerPed(-1) 0x56E210, RwCameraSetNearClipPlane 0x7EE1D0,
  CWorld::TestSphereAgainstWorld(CVector c /*by value*/, float r, CEntity* ignore, bool b, bool v, bool p, bool o, bool d,
  bool camIgnore) 0x569E20 (cdecl, returns CEntity* in EAX), CPlaceable::GetMatrix 0x411990 (thiscall),
  VectorSub(CVector* out, const CVector* a, const CVector* b) 0x40FE60 (out = a - b), CrossProduct 0x59C730,
  CRT `_CIpow` 0x822130 (ST0 = pow(ST1, ST0)), CCam::WellBufferMe(float target, float* cur, float* speed, float maxSpeed,
  float accel, bool isAngle) 0x509AE0 (cdecl).
- Globals: `CWorld::pIgnoreEntity` 0xB7CD68 (W), byte 0xB6EC65 (.data, unnamed flag enabling the second LOS test),
  `Scene.m_pRwCamera` 0xC1703C (near plane f32 at RwCamera+0x80 = "near"), `CDraw::ms_fAspectRatio` 0xC3EFA4,
  `CTimer::ms_fTimeStep` 0xB7CB5C, `gaTempSphereColPoints[0]` 0xB9B250 (point +0, normal +0x10), byte 0xBA6793 (loaded at
  0x5143C0 and never used), .data tunables (catalogue "gCameraMode+4/+8"): 0x8CC38C = 0.5, 0x8CC390 = 0.15 (minimum near
  clip), 0x8CC81C = 0.9 (timer decay base), 0x8CC820 = 1.15 (sphere radius factor); WellBuffer state floats
  0xB6EC38 (value) / 0xB6EC3C (speed). Constants 0.017453292f [0x8595EC], 0.5f [0x858B8C], 0.9f [0x858C20], 0.0f [0x858B50],
  immediates 0.2f (0x3E4CCCCD), 0.05f (0x3D4CCCCD).
- Fields: `m_vecClearGeometryVec` +0x8FC, `m_fAvoidTheGeometryProbsTimer` +0x164, `m_nAvoidTheGeometryProbsDirn` +0x168 (i16),
  `m_bMoveCamToAvoidGeom` +0x2D, `m_pTargetEntity` +0x958, `m_mCameraMatrix.up` x/y (+0x984/+0x988).
- Behaviour (near = Scene.m_pRwCamera->nearPlane, re-read at every use):
  1. `D = *target - *from` (x kept on the x87 stack, all stored f32); `m_vecClearGeometryVec = (0,0,0)`; `wbTarget = 0.0f`.
     `len3 = sqrt((D.z*D.z + D.x*D.x) + D.y*D.y)`; `len2 = sqrt(D.y*D.y + D.x*D.x)` (both f32).
  2. `beta = (D.x == 0 && D.y == 0) ? GetATanOfXY(up.x, up.y) : GetATanOfXY(D.x, D.y)` (exact compares; NaN = not zero;
     stored f32). `alpha = (len2 == 0 && D.z == 0) ? 0.0f : GetATanOfXY(len2, D.z)` (kept on x87).
  3. `F = (cos(beta)*cos(alpha), sin(beta)*cos(alpha), sin(alpha))` (f32 stores; `fcos`/`fsin` of the f32 beta and of
     the x87 alpha). `*out = (target.x - F.x*len3, target.y - F.y*len3, target.z - F.z*len3)` (F*len3 products first).
     `F.Normalise()`.
  4. `pIgnoreEntity = m_pTargetEntity; ent = nullptr;` `if (ProcessLineOfSight(*target, *out, cp, ent, true, false, false,
     true, false, false, true, false))`: `*out = cp.point; H = cp.point;` and if `byte 0xB6EC65 != 0` and
     `ProcessLineOfSight(*out, *target, cp, ent, false, true, true, true, false, false, true, false)`:
     `d1 = |*out - cp.point|` (sqrt((dx*dx + dz*dz) + dy*dy)); if `d1 < near` -> `*out = cp.point`; else
     `d2 = |*out - H|` (same order); if `d2 < near` -> `*out = H` (no-op in practice).
     Then `pIgnoreEntity = nullptr` (also on the no-hit path).
  5. `V = *target - *out`. If `FindPlayerPed(-1)`: `dist = sqrt((V.x*V.x + V.z*V.z) + V.y*V.y) - 0.5-global`; if
     `dist < near`: `RwCameraSetNearClipPlane(cam, dist > 0.15-global ? dist : 0.15-global)` (`<=` -> 0.15).
  6. `R = tan(fovDeg * 0.017453292f * 0.5f) * ms_fAspectRatio * 1.15-global` (x87 `fptan`); `radius = near * R` (f32);
     `C = (F.x*near + out.x, F.y*near + out.y, F.z*near + out.z)` (products stored f32 for x/y, z kept on x87).
     `if (!TestSphereAgainstWorld(C, radius, nullptr, true, false, false, true, false, true))` -> step 9.
  7. Sphere hit: `P1 = gaTempSphereColPoints[0].point`, `N = ...[0].normal`. `W = P1 - C`.
     `d = ((P1.x - out.x)*F.x + (P1.z - out.z)*F.z) + (P1.y - out.y)*F.y` (f32).
     If `0.15-global < d && d < 0.9f`: if `d < near` -> `RwCameraSetNearClipPlane(cam, d)`.
     Else if `d < 0.15-global` -> `RwCameraSetNearClipPlane(cam, 0.15-global)`. (NaN: neither.)
     `pen = radius - sqrt((W.x*W.x + W.z*W.z) + W.y*W.y)` (f32); `W.Normalise()`; `N.Normalise()` (copy).
     `Wp = W * pen` (x kept on x87, y/z stored f32). If `((-W.x)*N.x + N.z*(-W.z)) + N.y*(-W.y) < 0.0f` -> `N = -N`.
     `wbTarget = 1.0f`. `proj = ((-Wp.x)*N.x + (-Wp.z)*N.z) + (-Wp.y)*N.y`; `m_vecClearGeometryVec = (N.x*proj, N.y*proj, N.z*proj)`.
  8. If `m_pTargetEntity && (type & 7) == 3 && near < 0.15-global + 0.15-global` (x87 `fadd st0,st0`; equal/NaN fail):
     `m = m_pTargetEntity->GetMatrix()`; `s = (N.z*m.up.z + N.y*m.up.y) + N.x*m.up.x` (up = matrix +0x10, the forward row).
     If `s < 0`: `if (timer < 0) timer = 0; timer = ms_fTimeStep + timer`.
     Else (GetMatrix called again, same dot) if `s > 0.5f`: `if (timer > 0) timer = 0; timer = timer - ms_fTimeStep`.
     Then if `m_nAvoidTheGeometryProbsDirn == 0`: `X = CrossProduct(VectorSub(targetPos, *out), N)` (targetPos via
     m_matrix/placement); `m_nAvoidTheGeometryProbsDirn = (X.z <= 0.0f) ? 1 : -1` (NaN -> 1).
  9. `timer = pow(0.9-global, ms_fTimeStep) * timer`; `WellBufferMe(wbTarget, &g_B6EC38, &g_B6EC3C, 0.2f, 0.05f, false)`;
     `m_vecClearGeometryVec *= g_B6EC38` (each `g * c`); `m_bMoveCamToAvoidGeom = true`.
- Oddities (keep): the camera position is re-derived from angles (step 3) instead of using `from`; the second LOS
  fallback (`d2`) compares against the point that `*out` already equals; byte 0xBA6793 is loaded for nothing.
- gta-reversed: not hooked; `Camera.h:444` only declares it (no body).
- Open questions: (1) writers/meaning of byte 0xB6EC65 (0x51421D); (2) exact semantics of `TestSphereAgainstWorld`'s
  return and of `gaTempSphereColPoints` ordering are those of 0x569E20 (not specified here).

## 0x514D60 CCamera::CalculateFrustumPlanes  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void(bool forMirror [esp+4])`, `ret 4`.
- Caller: CCamera::CalculateDerivedValues 0x5150E0.
- Callees: `TransformVectors(RwV3d* out, int32 n, const CMatrix& m, const RwV3d* in)` 0x54EEA0 (cdecl; rotation only).
- Globals: `CDraw::ms_fFOV` 0x8D5038, `RsGlobal.maximumWidth` 0xC17044 (int32), `RsGlobal.maximumHeight` 0xC17048 (int32);
  constant 0.008726389f [0x8631D8] (= pi/360).
- Fields: `m_avecFrustumNormals[4]` +0xB24, `m_avecFrustumWorldNormals[4]` +0xB54, `m_avecFrustumWorldNormals_Mirror[4]` +0xB84,
  `m_fFrustumPlaneOffsets[4]` +0xBB4, `m_fFrustumPlaneOffsets_Mirror[4]` +0xBC4, `m_mCameraMatrix` +0x974, `m_matrix` +0x14.
- Behaviour:
  1. `a = ms_fFOV * 0.008726389f` (x87); `c = cos(a)` (f32 store), `s = sin(a)` (f32 store; `fcos` on a copy, `fsin` on a).
  2. `N0 = (c, -s, 0)`, `N1 = (-c, -s, 0)` (negations of the stored f32 values).
  3. `r = (float)maximumHeight / maximumWidth` (`fild` + `fidiv` with the int32; kept on the x87 stack, also stored f32);
     `N2 = (0, -(r*s), -(r*c))`, `N3 = (0, -(r*s), r*c)` (products of the x87 r with the f32 c/s; stored f32).
  4. Not mirror: `TransformVectors(m_avecFrustumWorldNormals, 4, m_mCameraMatrix, m_avecFrustumNormals)`; then for i = 0..3:
     `m_fFrustumPlaneOffsets[i] = (W[i].y*P.y + W[i].z*P.z) + W[i].x*P.x` with `P = m_matrix ? m_matrix->pos : placement.pos`
     (re-evaluated per plane; operand order: y product, z product, add, x product, add).
     Mirror: the same with `m_avecFrustumWorldNormals_Mirror` / `m_fFrustumPlaneOffsets_Mirror`, **still using
     m_mCameraMatrix** (not a mirror matrix) and the same camera position.
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod`.
- Open questions: none.

## 0x5150E0 CCamera::CalculateDerivedValues  (catalogue: specified/REA, port level 4)
- Convention: `__thiscall void(bool forMirror [esp+4], bool computeOrientation [esp+8])`, `ret 8`. SEH frame for the
  temporary CMatrix (destructor 0x59ACD0).
- Callers: CReplay::ProcessReplayCamera 0x45D060, CReplay::ProcessLookAroundCam 0x45D760, CCamera::SetCameraUpForMirror
  0x51A560, CCamera::RestoreCameraAfterMirror 0x51A5A0, CCamera::Process 0x52B730.
- Callees: `Invert(CMatrix* out, const CMatrix* in)` 0x59BDD0 (cdecl, returns out; catalogue "CMatrix::Invert-2"),
  `CMatrix::operator=` 0x59BBC0, `CMatrix::~CMatrix` 0x59ACD0, CCamera::CalculateFrustumPlanes 0x514D60.
- Fields: `m_mCameraMatrix` +0x974 (its "up"/forward row at +0x984/+0x988 = matrix+0x10/+0x14), `m_mMatInverse` +0xA4C,
  `m_fOrientation` +0x150, `m_fCamFrontXNorm` +0xDC, `m_fCamFrontYNorm` +0xE0.
- Behaviour:
  1. `CMatrix tmp; Invert(&tmp, &m_mCameraMatrix); m_mMatInverse = tmp; ~tmp`.
  2. `CalculateFrustumPlanes(forMirror)`.
  3. `f = m_mCameraMatrix.up` (x = +0x984, y = +0x988). If `f.x == 0.0f && f.y == 0.0f` (exact; NaN = not equal):
     `m_mCameraMatrix.up.x = 0.0001f` (0x38D1B717, written into the matrix) and the orientation is NOT computed.
     Else if `computeOrientation`: `m_fOrientation = atan2(f.x, f.y)` (`fpatan` with ST1 = up.x, ST0 = up.y).
  4. `x = up.x` (re-read, after the possible fix-up), `y = up.y`; `m_fCamFrontXNorm = x; m_fCamFrontYNorm = y`;
     `len = sqrt(y*y + x*x)` (y term first, x87). If `len == 0.0f`: `m_fCamFrontXNorm = 1.0f` (YNorm keeps y).
     Else `inv = 1.0f / len; m_fCamFrontXNorm = inv * x; m_fCamFrontYNorm = inv * y` (x87 products, f32 stores).
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod`.
- Open questions: none.

## 0x515BD0 CCamera::UpdateSoundDistances  (catalogue: specified/REA, port level 8)
- Convention: `__thiscall void()`, plain `ret`. Caller: CCamera::Process 0x52B730.
- Callees: `CWorld::ProcessVerticalLine(const CVector& origin, float zEnd, CColPoint& cp, CEntity*& ent, bool buildings,
  bool vehicles, bool peds, bool objects, bool dummies, bool seeThrough, CStoredCollPoly* poly)` 0x5674E0 (cdecl, 11 args).
- Globals: `CTimer::m_FrameCounter` 0xB7CB4C (u32). Constants 0.5f [0x858B8C], 5.0f [0x858C80], 20.0f [0x858BA4],
  1.0f [0x858624], 0.16666667f [0x85F0A0].
- Fields: `m_nActiveCam` +0x59, active cam mode (this+0x180 + active*0x238, u16), `m_pTargetEntity` +0x958,
  `m_mCameraMatrix.up` (+0x984/+0x988/+0x98C), `m_fSoundDistUp` +0x158, `m_fSoundDistUpAsRead` +0x15C,
  `m_fSoundDistUpAsReadOld` +0x160.
- Behaviour:
  1. `k = 5.0f`; if the active cam mode is one of 16, 7, 39, 40, 52, 42, 43, 41, 45, 46, 34, 8, 51 (first-person / aiming
     modes; test order as listed) **and** `(m_pTargetEntity->byte[0x36] & 7) == 3` (ped): `k = 0.5f`.
  2. `P = camPos + up * k` with camPos = `m_matrix ? m_matrix->pos : placement.pos`; per component `up.c * k` then
     `+ camPos.c` (z product stored f32 first, then added).
  3. `n = m_FrameCounter % 12` (unsigned). If `n == 0`:
     `m_fSoundDistUpAsReadOld = m_fSoundDistUpAsRead`;
     `if (ProcessVerticalLine(P, P.z + 20.0f, cp, ent, true, false, false, false, true, false, nullptr))
        m_fSoundDistUpAsRead = cp.m_vecPoint.z - P.z; else m_fSoundDistUpAsRead = 20.0f;`
  4. `f = (float)(n + 1) * 0.16666667f` (fild of the int); `m_fSoundDistUp = (1.0f - f) * m_fSoundDistUpAsReadOld + f * m_fSoundDistUpAsRead`
     (x87 order: `1 - f`, `* Old`, then `f * AsRead`, add).
- Oddities (keep): the blend factor runs over 12 frames but divides by 6, so for n >= 6 f > 1 and the result
  extrapolates beyond AsRead (up to 2x at n = 11).
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod`.
- Open questions: none.

## 0x516560 CCamera::ProcessShake (float)  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall CVector*(float unused [esp+4])`, `ret 4`. The argument is never read. EAX on return is the
  pointer returned by the last CrossProduct call = the address of a stack local of this function (dangling); callers
  must not use it.
- Callers: CCamera::ProcessShake() 0x51A6F0, CCamera::Process 0x52B730.
- Callees: CHandShaker::Process(float) 0x50D930 (thiscall), 0x59C810 = `Multiply3x3(CVector* out, const CVector* v, const CMatrix* m)`
  (cdecl; `out = (dot(v, m.right), dot(v, m.up), dot(v, m.at))`, i.e. transpose-multiply; unnamed in the catalogue),
  CVector::CrossProduct(out, a, b) 0x59C730 (cdecl, out = a x b, returns out), CVector::Normalise 0x59C910.
- Globals: static init flag byte 0xB70048 (catalogue "lastCalcCamPos+0x14"), `gHandShaker[6]` 0xB6ECA0 (stride 0x94;
  CHandShaker: +0x00 m_ang, +0x0C m_lim, +0x18 m_motion, +0x24 m_vel, +0x30 m_slow, +0x3C m_resultMat (CMatrix),
  +0x84 m_scaleReactionMin, +0x88 m_scaleReactionMax, +0x8C m_twitchFreq (int32), +0x90 m_twitchVel).
- Fields: `m_nActiveCam` +0x59, `m_aCams[active]` (cam = this + 0x174 + active*0x238; `m_vecFront` cam+0x190,
  `m_vecUp` cam+0x1B4), `m_fShakeIntensity` +0xC90, `m_nShakeType` +0xCA0 (int32, index into gHandShaker, unchecked).
- Behaviour:
  1. One-time init (if byte 0xB70048 == 0; then set to 1 at 0x516729): writes, for shakers 1..5 (shaker 0 untouched):
     `m_lim = (0.02, 0.02, Lz)`, `m_motion = (0.0002 [0x3951B717], 0.0002, 0.0001 [0x38D1B717])`, `m_slow = (1.3, 1.3, 1.4)`,
     `m_scaleReactionMin = 0.3`, `m_scaleReactionMax = 1.0`, and per shaker:
     [1] Lz 0.01, twitchFreq 15, twitchVel 0.001; [2] Lz 0.04, 20, 0.001; [3] Lz 0.01, 10, 0.0005; [4] Lz 0.01, 20, 0.002;
     [5] Lz 0.01, 2, 0.003 (all exact f32 bit patterns: 0.02 = 0x3CA3D70A, 0.01 = 0x3C23D70A, 0.04 = 0x3D23D70A,
     1.3 = 0x3FA66666, 1.4 = 0x3FB33333, 0.3 = 0x3E99999A, 0.001 = 0x3A83126F, 0.0005 = 0x3A03126F, 0.002 = 0x3B03126F,
     0.003 = 0x3B449BA6). Store order is irrelevant (no reads in between).
  2. `hs = &gHandShaker[m_nShakeType]`; `hs->Process(m_fShakeIntensity)`.
  3. `roll = hs->m_ang.z * m_fShakeIntensity` (f32 local).
  4. `tmp = Multiply3x3(cam.front, hs->m_resultMat)`; `cam.front = tmp`; `cam.front.Normalise()`.
  5. `cam.up = (sin(roll), 0, cos(roll))` (`fsin`/`fcos` of the f32 local, stored f32).
  6. `r = CrossProduct(cam.front, cam.up)`; `r.Normalise()`; `cam.up = CrossProduct(r, cam.front)`.
  7. If `cam.front.x == 0.0f && cam.front.y == 0.0f` (exact; NaN = not equal): `cam.front.x = cam.front.y = 0.0001f`
     (bits 0x38D1B717).
  8. `r = CrossProduct(cam.front, cam.up)`; `r.Normalise()`; `cam.up = CrossProduct(r, cam.front)`.
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethodAndReturn` (comment "shakeIntensity not used" - correct).
- Open questions: none.

## 0x51A6F0 CCamera::ProcessShake ()  (catalogue: specified/REA, port level 3)
- Convention: `__thiscall void()`, plain `ret`. No static callers (gr: "unused").
- Behaviour: `t = (float)(uint32)CTimer::m_snTimeInMilliseconds` (fild + 2^32 fix-up [0x858C54] when negative);
  if `t <= m_fEndShakeTime (+0xC98)` (`fcom; test ah,0x41; jp` -> NaN/greater skip):
  `ProcessShake((t - m_fStartShakeTime (+0xC94)) / (m_fEndShakeTime - m_fStartShakeTime))` (the float argument is unused
  by the callee). Otherwise nothing.
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod`.
- Open questions: none.

## 0x516B20 CCamera::ImproveNearClip  (catalogue: specified/REA, port level 10)
- Convention: `__thiscall void(CVehicle* veh [esp+4], CPed* ped [esp+8], CVector* source [esp+0xC], CVector* target [esp+0x10])`,
  `ret 0x10`. The `source`-slot neighbour [esp+0x28] (= arg `ped` slot after pushes) is reused as a temp.
- Callers: CCam::LookBehind 0x520690 (2), CCam::Process_AimWeapon 0x521500, CCam::Process_FollowPed_SA 0x522D40.
- Callees: RwCameraSetNearClipPlane 0x7EE1D0, CCamera::CalculateGroundHeight 0x514B80 (ST0),
  CCamera::GetRoughDistanceToGround 0x516B00 (ST0), CPedIntelligence::GetUsingParachute 0x6011B0 (AL), ::GetTaskSwim 0x601070,
  ::GetTaskJetPack 0x601110, `CWaterLevel::GetWaterLevel(x, y, z, float* lvl, bool waves, CVector* normal)` 0x6EB690 (AL),
  VectorSub(out, a, b) 0x40FE60, CVector::Magnitude 0x4082C0 (ST0), 0x404330 = `float min(float a, float b)` (cdecl:
  returns a if a < b else b), CPedModelInfo::AnimatePedColModelSkinnedWorld(RpClump*) 0x4C7170 (thiscall),
  `_ftol2` 0x821B40, `CCollision::CheckPeds(const CVector& pos, const CVector& dir, float* dist)` 0x4154A0 (cdecl).
- Globals: `gCurDistForCam` 0x8CCB84, `gLastRadiusUsedInCollisionPreventionOfCamera` 0xB6EC6C, `Scene.m_pRwCamera` 0xC1703C
  (near = +0x80), `TheCamera.m_nPedZoom` 0xB6F0F0, active cam (`TheCamera.m_aCams[m_nActiveCam]`, absolute 0xB6F19C +
  i*0x238: m_vecFront +0x190, m_vecSource +0x19C, m_fFOV via 0xB6F250). .data tunables: 0x8CCD08 = 10.0, 0x8CCD04 = 1.0,
  0x8CCD00 = 0.3, 0x8CCCFC = 10.0, 0x8CCCF8 = 5.0, 0x8CCCF4 = 0.1, 0x8CCCF0 = 0.1, 0x8CCCEC = 0.3, 0x8CCCE8 = 0.1,
  0x8CCCE4 = 0.1, 0x8CCCE0 = 2.0, 0x8CCCDC = 0.3, 0x8CCCD8 = 1.0, 0x8CCCD4 = 0.02, 0x8CCCD0 = 0.3. Constants 0.5f [0x858B8C],
  90.0f [0x85991C], 0.017453292f [0x8595EC], 100.0f [0x858628], 0.01f [0x858C58], 999999.0f (0x497423F0).
- Behaviour (`near` = current near plane, re-read at each use; d = *source - *target):
  1. `if (sqrt((d.z*d.z + d.y*d.y) + d.x*d.x) > 10-global) { L = 1.0-global * gCurDistForCam; if (L > near) SetNear(L); }`
  2. If `veh`: if `veh->dword[0x594]` is 4 (plane) or 3 (heli):
     - if `gCurDistForCam > 0.3-global (0x8CCD00)`: `g = CalculateGroundHeight(0)`; if `activeCam.m_vecSource.z (this+0x318+i*0x238)
       - g > 10-global (0x8CCCFC)`: `L = 5-global * gCurDistForCam`; `m = |d| * 0.1-global` (same sqrt order); `if (m < L) L = m`;
       `if (L > near) SetNear(L)`.
     - else (gCurDist <= 0.3) if heli (3): `SetNear(0.1-global 0x8CCCF0)`.
     Other vehicles: nothing. Then step 4.
  3. Else if `ped`:
     a. Not standing (`(ped->byte[0x46C] & 1) == 0`, bIsStanding clear): `para = GetUsingParachute()`; `sw = GetTaskSwim()`;
        - if `sw`: `deep = (sw->int16[+0xA] == 4)` (swim state); `found = GetWaterLevel(src.x, src.y, src.z, &lvl /*init 0*/, false, nullptr)`;
          if `found && fabs(lvl - src.z) < 0.3-global (0x8CCCEC)` -> `SetNear(0.1-global 0x8CCCE8)`; else if `deep && m_nPedZoom (0xB6F0F0) == 1`
          -> `SetNear(0.1-global 0x8CCCE4)`.
        - else if `para || GetTaskJetPack()`: if `GetRoughDistanceToGround() > 10-global (0x8CCCFC)`:
          `L = min(Magnitude(*source - *target) * 0.3-global, 2.0-global * gCurDistForCam)`; `if (L > near) SetNear(L)`.
     b. Standing: `a = sin((90.0f - activeCam.m_fFOV * 0.5f) * 0.017453292f) * gLastRadius...` (f32);
        `mi = ms_modelInfoPtrs[ped->m_nModelIndex]`; `mi->AnimatePedColModelSkinnedWorld(ped->m_pRwClump)`;
        `sp = mi->m_pColModel (+0x34)->m_pColData (+0x2C)->m_pSpheres (+8)` (12 spheres used, stride 0x14: center +0, radius +0xC,
        piece byte +0x11); `d0 = (F.z*S.z + F.y*S.y) + S.x*F.x` (F = activeCam front, S = activeCam source);
        `mn = 999999.0f`; for i = 0..11: `v = ((c.x*F.x + c.z*F.z) + c.y*F.y) - d0 - r` (for i > 0 the code forms `c.z*F.z + c.x*F.x`,
        same value); if `piece == 9`: `v = v - 1.0-global * r`; `if (v < mn) mn = v` (strict).
        Then `if (mn > a) mn = a; else if (mn < 0.02-global) mn = 0.02-global;` `if (mn > 0.3-global) mn = 0.3-global;`
        `SetNear((float)_ftol2(mn * 100.0f) * 0.01f)` (truncation to centimetres).
  4. Always (all paths, also when veh and ped are null): `CCollision::CheckPeds(*source, activeCam.m_vecFront, &tmp /*999999.0f*/)`;
     the result is discarded.
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod`.
- Open questions: (1) gr name of `veh->dword[0x594]` (gr CVehicle m_nVehicleSubType?, values 3 heli, 4 plane per eVehicleType);
  (2) the swim-state value 4 meaning (0x516CE6).

## 0x51D770 CCamera::IsItTimeForNewcam  (catalogue: specified/REA, port level 11)
- Convention: `__thiscall bool(int32 seq [esp+4], uint32 startTimeMs [esp+8])`, AL (1 = switch to a new camera), `ret 8`.
  Entry is split by obfuscation: 0x51D770 `nop; jmp 0x409B61` (`sub esp,0x158`) -> 0x51D776.
- Callers: the five ProcessObbeCinemaCamera* (0x5267C0, 0x526950, 0x526AE0, 0x526C80, 0x526E20).
- Callees: CPad::GetPad(0) 0x53FB70, CWorld::FindPlayerVehicle(-1, false) 0x56E0D0, CWorld::FindPlayerCoors(CVector* out, -1)
  0x56E010, CWorld::FindPlayerSpeed(-1) 0x56E090 (returns CVector*), CWorld::GetIsLineOfSightClear(from, to, b, v, p, o, d,
  seeThrough, camIgnore) 0x56A490, VectorSub(out, a, b) 0x40FE60 (out = a - b), CVector::Magnitude 0x4082C0 (ST0),
  CEntity::GetIsOnScreen 0x534540, RwCameraSetNearClipPlane 0x7EE1D0, CCam::Process_DW_HeliChaseCam 0x51A740,
  ::Process_DW_CamManCam 0x51B120, ::Process_DW_BirdyCam 0x51B850, ::Process_DW_PlaneSpotterCam 0x51C250,
  ::Process_DW_PlaneCam1 0x51C760, ::Process_DW_PlaneCam2 0x51CC30, ::Process_DW_PlaneCam3 0x51D100 (each `thiscall(bool)` on
  `m_aCams[m_nActiveCam]`, called with `true`, AL).
- Globals: `CTimer::m_snTimeInMilliseconds`; .data f32 0x8CCDF0 = 15000.0 (max shot time), 0x8CCDF8 = 20000.0 (absolute max);
  byte 0x8CCDF4 (stick-released latch, R/W); int8 0x8CC471 (sequence step, written +1/-1); `TheCamera.m_bUseNearClipScript`
  0xB6F059 (written via the absolute address in cases 24/25). Constants: 2^32 [0x858C54], 32.0f [0x85950C], 96.0f [0x863218],
  0.0f [0x858B50], 40.0f [0x858A10], 4.5f [0x863214], 2.0f [0x858CA0], 0.5f [0x858B8C], 0.05f [0x858C28], 29.0f [0x863210],
  48.0f [0x862D38], 38.0f [0x86320C], 44.0f [0x858FF8], 50.0f [0x858B40], 3.0f [0x858B3C], 57.0f [0x863208], 1.0f [0x858624],
  36.0f [0x863204]; immediates 0.15f (0x3E19999A), 0.6f (0x3F19999A).
- Fields: `m_vecFixedModeSource` +0x86C, `m_pTargetEntity` +0x958, `m_bUseNearClipScript` +0x31, `m_fNearClipScript` +0x138,
  active cam source = this + 0x310 + m_nActiveCam*0x238.
- Common prologue:
  1. `if (seq < 0) return true`.
  2. `el = (float)(uint32)now - (int32)startTimeMs` (fild + 2^32 fix-up, `fisub`, stored f32). `if (el > 20000-global) return true`.
  3. `s = (float)CPad::GetPad(0)->NewState.RightStickX` (int16); if `fabs(s) < 32.0f`: `latch = 1`. Else if `fabs(s) > 96.0f &&
     latch`: `step = (s > 0.0f) ? 1 : -1; latch = 0; return true`. (32 <= |s| <= 96, or no latch: fall through.)
  4. `switch (seq)` (table 0x51E4E8, 30 entries); seq > 29 and seq 9..14 -> return false.
- Notation: `P = FindPlayerCoors(-1)` (called afresh for each use), `F = m_vecFixedModeSource`, `V = P - F` with `V.z = 0`
  unless stated, `LOS(a,b)` = `GetIsLineOfSightClear(a, b, true, false, false, false, false, false, false)`,
  `boatSkip` = "if FindPlayerVehicle() exists and its `dword[0x590] == 5` (boat) and `m_pTargetEntity->m_nModelIndex != 460`
  (0x1CC, skimmer) -> return true", `moving` = `dot > 0` with `sp = FindPlayerSpeed(-1)` and dot = V.x*sp.x + V.y*sp.y + sp.z*0.0f
  (cases 1/2/5 add as `(V.y*sp.y + sp.z*0) + V.x*sp.x`, cases 3/15/16/17 as `(V.y*sp.y + V.x*sp.x) + sp.z*0`; equal for finite
  values), `near015` = `RwCameraSetNearClipPlane(Scene.camera, 0.15f)`. "long" = `el > 15000-global` -> return true.
  Time tests `now > start + N` are unsigned 32-bit.
- Cases:
  - 0: if a player vehicle exists: boatSkip-style test (boat and target model != 460 -> true); if the vehicle's model is 432
    (0x1B0, Rhino) -> true; `if (!GetIsLineOfSightClear(targetPos, activeCam.source, true, false x6)) return true`. Then
    `if (now > start + 5000) return true; near015; return false`.
  - 1: long; boatSkip; `if (!LOS(P, F)) return true`; if `|V| > 40` and moving -> true; `return |V| < 4.5f`.
  - 2: long; boatSkip; `!LOS -> true`; if `|V| < 2.0f`: `m_fNearClipScript = max(|V| * 0.5f, 0.05f)` (`< 0.05` -> 0.05),
    `m_bUseNearClipScript = true`; if `|V| > 29` and moving -> true; if `|V| < 2.0f` -> true; else `near015; return false`.
  - 3: long; (no boatSkip) `!LOS -> true`; if `|V| > 48` and moving -> true; else `near015; return false`.
  - 4, 6: `return now > start + 3000`.
  - 5: long; boatSkip; `!LOS -> true`; `return |V| > 38 && moving`.
  - 7: `if (now <= start + 2000) return false; return !FindPlayerVehicle(-1,false)->GetIsOnScreen()` (no null check).
  - 8: long; boatSkip; `if (!GetIsLineOfSightClear(targetPos, activeCam.source, true, false x6)) return true`;
    `if (now > start + 1000) return true`; `m_fNearClipScript = 0.6f; m_bUseNearClipScript = true; return false`.
  - 15: long; no player vehicle -> false; `!LOS -> true`; if `|V| > 44` and moving -> true; `return |V| < 3.0f`.
  - 16: long; no vehicle -> false; `!LOS -> true`; if `|V| > 50` -> true; `return |V| < 3.0f`.
  - 17: long; no vehicle -> false; `!LOS -> true`; if `|V| > 50` and moving -> true; `return |V| < 2.0f`.
  - 18: long; `!LOS -> true`; V is 3-D here (z kept); `if (|V| > 57) return true; return |V| < 1.0f`.
  - 19: long; `!LOS -> true`; `if (|V| > 36) return true; return |V| < 2.0f`.
  - 20..23, 26..28: `return !activeCam.Process_DW_X(true)` with X = HeliChaseCam (20), CamManCam (21), BirdyCam (22),
    PlaneSpotterCam (23), PlaneCam1 (26), PlaneCam2 (27), PlaneCam3 (28).
  - 24, 25: `TheCamera.m_bUseNearClipScript (0xB6F059) = false; return true`.
  - 29: `return now > start + 5000`.
  All "|V| > k" use `fcomp; test ah,0x41` (strict, NaN false); "|V| < k" use `test ah,5` (strict, NaN false). Magnitude is
  recomputed by a fresh call each time it is used.
- gta-reversed: not hooked, no body.
- Open questions: none.

## 0x51E560 CCamera::TryToStartNewCamMode  (catalogue: specified/REA, port level 11)
- Convention: `__thiscall bool(int32 seq [esp+4])`, AL, `ret 4`. Obfuscated entry/exit: 0x51E560 `nop; jmp 0x4014E2`
  (`sub esp,0x290`) -> 0x51E566; the DW-camera common tail returns through 0x40188B (`add esp,0x290; jmp 0x51FFD4 ret 4`).
  `this` kept in EDI and in a local ([esp+0x5C]).
- Callers: the five ProcessObbeCinemaCamera* (3 sites each).
- Callees (main): CCamera::TakeControl(CEntity* target, int16 mode, int16 switchType, int32 who) 0x50C7C0 (always
  `switchType = 2, who = 2` here), CCamera::SetCamPositionForFixedMode(const CVector* pos, const CVector* up) 0x50BEC0,
  0x50B830 (active cam under water, see ProcessObbeCinemaCameraCar), CWorld::FindPlayerVehicle(-1,false),
  FindPlayerCoors(out,-1), FindPlayerSpeed(-1), FindPlayerEntity(-1) 0x56E120, FindPlayerPed(-1) 0x56E210,
  CPlayerPed::GetWantedLevel 0x41BE60, `CPool<CVehicle>::GetAt(i)` 0x41CC10 (on `CPools::ms_pVehiclePool` 0xB74494, size at +8),
  Multiply3x3(CVector* out, const CMatrix* m, const CVector* v) 0x59C790, `CVector::operator+=` 0x411A00 (thiscall),
  CVector::Normalise, CVector::Magnitude, VectorSub(out, a, b), CGeneral::GetATanOfXY(x, y) 0x53CC70,
  CWorld::FindGroundZFor3DCoord(x, y, z, bool* found, CEntity** e) 0x5696C0 (ST0), CWorld::FindRoofZFor3DCoord(x, y, z, bool* found)
  0x569750 (ST0), CWaterLevel::GetWaterLevelNoWaves(x, y, z, float* lvl, float* a, float* b) 0x6E8580 (AL),
  GetIsLineOfSightClear 0x56A490, RwCameraSetNearClipPlane, CCam::Process_DW_* (as in IsItTimeForNewcam).
- Globals: .data 0x8CC8C0 = 1.0 (water offset), 0x8CC8C4 = -2.0 (water offset for boats); `TheCamera.m_bUseNearClipScript`
  0xB6F059. Constants listed per case.
- Notation: `P` = FindPlayerCoors(-1) (fresh call per use); `S` = FindPlayerSpeed(-1) copied, `S.z = 0`, normalised;
  `LOS(a,b)` = `GetIsLineOfSightClear(a, b, true, false, false, false, false, false, false)`; `boat` = player vehicle exists and
  its `dword[0x590] == 5`; `boatSkipF` = "boat && m_pTargetEntity->m_nModelIndex != 460 -> return false";
  `fixed(C)` = `SetCamPositionForFixedMode(&C, &(0,0,0)); TakeControl(FindPlayerEntity(-1), 15 /*MODE_FIXED*/, 2, 2)`;
  `uw()` = 0x50B830 (camera under water); `moving` / `|V|` as in IsItTimeForNewcam (V = P - C, `V.z = 0` unless stated).
  "x87 position pattern" (exact, from case 1, 0x51E6E2-0x51E746): with distance k and side factor q:
  `C.x = (P.x + S.x*k) + S.y*q` (first sum stored f32), `C.y = (S.y*k + P.y) + S.x*(-q)` (the -q is a separate f32 constant
  multiplied, not a subtraction), `C.z = P.z + S.z*k`; all products `S.c * k` are formed first.
- Cases (`switch (seq)`, table 0x520110; seq > 29, 4 and 9..14 -> return false):
  - 0 (wheel cam on the player car): `v = FindPlayerVehicle`; null -> false; boat && target model != 460 -> false;
    `v->m_nModelIndex == 432` (Rhino) -> false; `W = Multiply3x3(v->m_matrix, (-1.4f, -2.3f, 0.3f))` (bits 0xBFB33333,
    0xC0133333, 0x3E99999A), `W += vPos`; `if (!GetIsLineOfSightClear(vPos, W, true, false x6)) return false`;
    `TakeControl(v, 14 /*WHEELCAM*/, 2, 2); return true`.
  - 1: boatSkipF; position pattern k = 20.0 [0x858BA4], q = +3.0 [0x858B3C] (S.y*q, fmul at 0x51E722), -q = -3.0 [0x859024] (S.x*(-q), fmul at 0x51E72C; re-checked: [0x858B3C] holds +3.0f, not -3.0); ground fix:
    `g = FindGroundZFor3DCoord(C.x, C.y, C.z + 5.0f, &f, nullptr)`; if f `C.z = g + 1.5f` [0x858CE8]; else
    `r = FindRoofZFor3DCoord(C.x, C.y, C.z - 5.0f, &f)`; if f `C.z = r + 1.5f`. `!LOS(P, C) -> false`;
    if `|V| > 40` and moving -> false; if `|V| < 4.5` -> false; `fixed(C); return !uw()`.
  - 2: boatSkipF; k = 16.0 [0x8599D0], q = 2.5 [0x858FA0] / -2.5 [0x8631B0]; ground/roof fix with +0.5f; `!LOS -> false`;
    `|V| > 29 && moving -> false`; `|V| < 2 -> false`; `fixed(C); RwCameraSetNearClipPlane(cam, 0.15f); return !uw()`.
  - 3: k = 30.0 [0x858CA4], q = 8.0 [0x859000] / -8.0 [0x85B320], then `C.z = (P.z + S.z*30) + 16.0f` [0x8599D0];
    no ground fix, no distance tests; `!LOS -> false`; `fixed(C)`; `if (uw()) return false; SetNear(0.15f); return true`.
  - 5: k = 30.0, side reversed: `C.x = (P.x + S.x*30) + S.y*(-6.0f)` [0x85A5C0], `C.y = (S.y*30 + P.y) + S.x*6.0f` [0x858B44];
    ground/roof fix with +3.5f [0x859028]; `!LOS -> false`; `fixed(C); return !uw()`.
  - 6: `TakeControl(FindPlayerEntity(-1), 16 /*1STPERSON*/, 2, 2); return true`.
  - 7 (police car string cam): `if (FindPlayerPed(-1)->GetWantedLevel() < 1) return false`; no player vehicle -> false;
    boatSkipF; for `i = pool.size - 1 .. 0` (`GetAt(i)`): candidate `c` must satisfy: non-null, `c->dword[0x590] == 0`
    (automobile), `c != FindPlayerVehicle()`, `c->byte[0x428] & 1`, `(c->byte[0x36] & 0xF8) == 0x18` (status 3);
    `dx = cPos.x - P.x`, `dy = cPos.y - P.y` (f32); `if (|cPos - P| >= 30.0f) continue` (full 3-D VectorSub(P?, see
    0x51F012-0x51F035) magnitude, `test ah,5`); `if (dy*pv.up.y + dx*pv.up.x >= 0) continue` (pv = player vehicle matrix,
    up row +0x10; the car must be behind); `if (c.up.y*pv.up.y + c.up.x*pv.up.x <= 0.8f) continue` [0x858C98];
    `TakeControl(c, 18 /*CAM_ON_A_STRING*/, 2, 2)`; `if (!uw()) return true;` else continue scanning. Loop end -> false.
  - 8 (police wheel cam): wanted level >= 1, player vehicle, boatSkipF as in 7; same scan **without the status test**;
    for the first matching car: `W = Multiply3x3(c->m_matrix, (-1.4, -2.3, 0.3)) + cPos`; `if (!GetIsLineOfSightClear(cPos, W,
    true, false x6)) return false` (aborts the whole scan); `TakeControl(c, 14, 2, 2)`; `if (!uw()) return true;` else continue.
  - 15 [x87 checked 0x51F3F2-0x51F473]: player vehicle required; with k = 34.0 [0x86322C]: `C.x = S.x*34 + P.x`,
    `C.y = P.y + S.y*34`, `C.z = S.z*34 + P.z` (no side term; the three products are formed first, S.z*34 stored f32), then
    `C.z = FindPlayerCoors().z + 0.5f` (fresh call, overwrites); if boat `C.z += 1.0f` (0x51F469, slot of C.z confirmed);
    `!LOS -> false`; V = P - C **3-D** (V.z not cleared, 0x51F4C4-0x51F512); `|V| > 44.0 [0x858FF8] && moving -> false` with
    the dot formed as `(V.y*sp.y + sp.z*0.0f) + V.x*sp.x` (0x51F52B-0x51F546); `|V| < 3.0 -> false`; `fixed(C); return !uw()`.
  - 16 / 17 (angled): player vehicle required; `a = GetATanOfXY(S.x, S.y) + k_ang` (16: 1.0471976f [0x8630F8];
    17: 3.3161256f [0x863228]); `S.x = cos(a) + S.x; S.y = sin(a) + S.y; S.Normalise()`; `C = P + S*d` (16: d = 30;
    17: d = 25.0 [0x858FE8]; x: `S.x*d + P.x`, y: `P.y + S.y*d`, z: `S.z*d + P.z`; x87 checked for 17 at 0x51F86B-0x51F8B5:
    the a = atan + k_ang is kept in a register, `fcos`/`fsin` of it, `sin` stored f32 first, then `S.x = cos + S.x`,
    `S.y = sin + S.y`); then **`C.z = P.z - h` is stored into C first** (P re-read; 16: h = 5.5 [0x859B4C] at 0x51F676;
    17: h = 1.0 at 0x51F8C5) and `r = FindRoofZFor3DCoord(C.x, C.y, C.z, &f)`; if `f == 1`: `C.z = r + 0.5f`; else if
    `GetWaterLevelNoWaves(C.x, C.y, C.z /* = P.z - h */, &lvl, 0, 0)`: `w = (boat ? -2.0-global : 1.0-global) + lvl`;
    `if (C.z < w) C.z = w` (`test ah,5; jp` keeps C.z on >= or NaN). So the `S.z*d + P.z` height is always replaced.
    `!LOS -> false`; V = P - C **3-D**; 16: `|V| > 50 -> false`, `|V| < 3 -> false`;
    17: `|V| > 50.0 [0x858B40] && moving -> false` (dot order as case 15, 0x51FA4D-0x51FA68), `|V| < 2.0 -> false`;
    `fixed(C); return !uw()`.
  - 18 / 19 (high/low angled): `Pz' = P.z + (boat ? 23.0 : -23.0)` [0x858F84] (19: `boat ? P.z + 4.0` [0x858B90] `: P.z - 1.0`);
    `sp = FindPlayerSpeed()` (not normalised first); `a = GetATanOfXY(sp.x, sp.y) + k_ang` (18: 2.5307274f [0x863224];
    19: 0.4886922f [0x863220]); `S = (sp.x + cos(a), sp.y + sin(a), 0)`, normalised; `C = (P.x + S.x*d, P.y + S.y*d, S.z*d + Pz')`
    (18: d = 15.0 [0x858B48]; 19: d = 12.5 [0x86321C]); `g = FindGroundZFor3DCoord(C.x, C.y, C.z, &f, nullptr)`;
    **if `g == 1.0f` exactly** (`fcomp 1.0; test ah,0x44; jp`, the `found` flag is ignored - original bug): if `C.z < g`
    `C.z = g + 0.5f`; else the water rule of 16/17. `!LOS -> false`; V 3-D; 18: `|V| > 57 -> false`, `|V| < 1 -> false`;
    19: `|V| > 36 -> false`, `|V| < 2 -> false`; `fixed(C); return !uw()`.
  - 20..23, 26..28: `if (!activeCam.Process_DW_X(true)) return false; TakeControl(FindPlayerEntity(-1), mode, 2, 2);
    return !uw();` with (X, mode) = (HeliChaseCam, 56), (CamManCam, 57), (BirdyCam, 58), (PlaneSpotterCam, 59),
    (PlaneCam1, 62), (PlaneCam2, 63), (PlaneCam3, 64).
  - 24, 25: `TheCamera.m_bUseNearClipScript (0xB6F059) = false; return false`.
  - 29: `TakeControl(FindPlayerEntity(-1), 18, 2, 2); return true`.
- Oddities (keep): cases 18/19 test the returned ground height against 1.0f instead of the found flag; case 8 aborts the
  scan on the first blocked LOS; boatSkip returns false here (true in IsItTimeForNewcam).
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethodAndReturn`.
- Open questions (confidence medium): exact x87 operand order of the position formulas is checked for cases 1, 2, 3, 5,
  15, 16, 17, 18 and 19 (15 and 17 re-verified from the full listing by the follow-up writer). Case 7 distance:
  VectorSub(cPos?, P) operand order at 0x51F012-0x51F02B not re-derived (only its magnitude is used, so order is irrelevant).
  The `moving` dot product in IsItTimeForNewcam is written there as `V.x*sp.x + V.y*sp.y + sp.z*0`; in this function
  (cases 15, 17) the bytes add `(V.y*sp.y + sp.z*0) + V.x*sp.x`.

## 0x520190 CCamera::CameraColDetAndReact  (catalogue: specified/REA, port level 9)
- Convention: `bool (CVector* source [esp+4], const CVector* target [esp+8])`, AL, `ret 8`. ECX is never read (gr declares
  it a CCamera member; the original behaves as a static/stdcall function). The `target` slot is reused for the radius.
- Callers: CCam::LookBehind 0x520690 (2), CCam::LookRight 0x520E40, CCam::Process_AimWeapon 0x521500,
  CCam::Process_FollowPed_SA 0x522D40.
- Callees: CEntity::GetColModel 0x535300, CPlaceable::AllocateMatrix 0x54F560, CSimpleTransform::UpdateMatrix 0x54F1B0,
  **CCamera::ConeCastCollisionResolve(src, tgt, CVector* outPos, float radius, float angle, float* outDist) 0x51A5D0 called
  with ECX = &TheCamera** (0x5204BF), RwCameraSetNearClipPlane 0x7EE1D0.
- Globals: `gpCamColVars` 0xB6FE88 (float*, cv[0..5] used), `gCurCamColVars` 0x8CCB80 (u8), `gCurDistForCam` 0x8CCB84 (f32, R/W),
  `CWorld::pIgnoreEntity` 0xB7CD68, `gLastRadiusUsedInCollisionPreventionOfCamera` 0xB6EC6C (W), `CTimer::ms_fTimeStep`,
  `Scene.m_pRwCamera` 0xC1703C, `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8. Statics: min sphere bottom 0xB700EC (f32) cached
  for model id 0xB700F0 (int32); prevSource 0xB700DC..0xB700E4 with init bit 0 of 0xB700E8.
  .data tunables: 0x8CCB90 = 0.2939, 0x8CCE18 = 0.65, 0x8CCE14 = 0.3, 0x8CCE10 = 0.18, 0x8CCE0C = 0.05, 0x8CCE08 = 0.01,
  0x8CCE04 = 0.05, 0x8CCE00 = 0.5, 0x8CCDFC = 0.05. Constants 0.5f [0x858B8C], 0.2f [0x858CC4], 1.0f [0x858624].
- Behaviour (`cv = gpCamColVars`, `ign = pIgnoreEntity`, `ccv = gCurCamColVars`):
  1. `D = *source - *target` (f32 x, y, z). `R = sqrt((D.z*D.z + D.y*D.y) + D.x*D.x) * cv[0] * 0.2939-global` (f32).
  2. If `ccv >= 10` (unsigned) and `ign != nullptr`:
     a. Vehicle automobile (`(ign->byte[0x36] & 7) == 2 && ign->dword[0x594] == 0`): if `ign->m_nModelIndex (+0x22, i16) !=
        cachedModel`: `minBottom = 100.0f`; `cd = ign->GetColModel()->m_pColData (+0x2C)`; if cd and `n = cd->numSpheres (i16 +0) > 0`:
        for each sphere s (array `cd+8`, stride 0x14): `v = s.center.z - s.radius`; `if (v < minBottom) minBottom = v`;
        `cachedModel = model id`. Ensure a matrix (`AllocateMatrix(); placement.UpdateMatrix(m_matrix)` when null).
        `h = ((dz*m.at.z + dy*m.at.y) + dx*m.at.x) - minBottom` with `d = *target - vehPos` (at = matrix +0x20); `h = max(h, 0.2f)`
        (`0.2 > h` -> 0.2; NaN keeps h).
     b. Otherwise: `bb = modelInfo->m_pColModel` bounding box; `hx = (max.x-min.x)*0.5f`, `hy = (max.y-min.y)*0.5f`,
        `hz = (max.z-min.z)*0.5f` (the z extent stored f32 before the multiply); `h = (hx < hy) ? (hx < hz ? hx : hz) : (hy < hz ? hy : hz)`.
     c. With h from a or b: `if (h > cv[1]) R = min(R, cv[1]) else R = min(R, h)` (min = replace when strictly smaller).
  3. `if (R > cv[1]) R = cv[1]; if (R < 0.65-global) R = 0.65-global`.
  4. Cone angle `A = cv[2]`; if `ccv <= 9`: `A = (ccv > 3 ? 0.3-global : 0.18-global) / |*source - *target|`
     (`sqrt((dz*dz + dy*dy) + dx*dx)`, recomputed from the vectors). `bikeFlag = false`; if `ccv >= 10 && ign &&
     type == 2 && ign->dword[0x590] == 9` (bike): `bikeFlag = true; A = 0.05-global (0x8CCE0C)`.
  5. `gLastRadiusUsedInCollisionPreventionOfCamera = R`;
     `hit = TheCamera.ConeCastCollisionResolve(source, target, &tmpPos, R, A, &dist)`; if hit: `ret = 1`; if `dist <= cv[3]`
     (NaN no): `RwCameraSetNearClipPlane(Scene.camera, cv[4])`. Else `ret = 0`.
  6. If `dist < gCurDistForCam`: `gCurDistForCam = dist`. Else: init prevSource to (0,0,0) once; `m = |source - prevSource|^2`
     (`(dz*dz + dy*dy) + dx*dx`); if `0.01*0.01 < m` (`fcompp`; NaN no): `inc = (dist - gCurDistForCam) * (ms_fTimeStep * cv[5])`;
     `if (inc > 0.05-global) inc = 0.05-global`; `gCurDistForCam = inc + gCurDistForCam`. Then `prevSource = *source`.
  7. `if (gCurDistForCam > 1.0f) gCurDistForCam = 1.0f`.
  8. `*source = (D.x*g + target.x, D.y*g + target.y, D.z*g + target.z)` with g = gCurDistForCam (D from step 1).
  9. If `bikeFlag && gCurDistForCam < 0.5-global`: `RwCameraSetNearClipPlane(Scene.camera, 0.05-global (0x8CCDFC))`.
  10. Return ret.
- Oddities (keep): 0x51A5D0 returns false WITHOUT writing `*outDist` when `*source == *target` exactly (its first test,
  0x51A5DD-0x51A5FB; gr Camera.cpp:1679 agrees); then step 6 reads the uninitialised stack local `dist` ([esp+0x10]).
  On other misses it writes 1.0. A port must reproduce "previous stack contents" - in practice use a persistent value and
  record this in the divergence ledger. `tmpPos` is never used.
- gta-reversed: hooked `.Reversed = false`, `plugin::Call`.
- Open questions: value of the uninitialised `dist` at 0x5204FC when source == target (stack garbage).

## 0x5267C0 CCamera::ProcessObbeCinemaCameraCar  (catalogue: specified/REA, port level 12)
The five "Obbe cinema camera" selectors 0x5267C0 (car), 0x526950 (train), 0x526AE0 (heli), 0x526C80 (plane), 0x526E20
(boat) share one body, described here once with a parameter table; each has its own section below.
- Convention: `__thiscall void()`, plain `ret`. Caller: CCamera::CamControl 0x527FA0 (one site each).
- Callees: CCamera::IsItTimeForNewcam(int32 seq, int32 startTimeMs) 0x51D770 (AL), CCamera::TryToStartNewCamMode(int32 seq)
  0x51E560 (AL), `CText::Get(TheText 0xC1B340, "CINCAM" (0x863248))` 0x6A0050, `CHud::SetHelpMessage(text, true, false, false)`
  0x588BE0 (cdecl, 4 args); heli and plane also 0x50B830 = "active camera under water" (cdecl, AL: `GetWaterLevel(
  src.x, src.y, src.z, &lvl, true, nullptr)` on `m_aCams[m_nActiveCam].m_vecSource` and `lvl >= src.z` (NaN true); not in this
  work list).
- Globals: `CCamera::bDidWeProcessAnyCinemaCam` 0xB6EC2D (u8, R/W), byte 0xB6EC34 (written 1), `gbCineyCamMessageDisplayed`
  0x8CC381 (int8, R/W, initial 2), step int8 0x8CC471 (initial 1), `CTimer::m_snTimeInMilliseconds`.
  Per vehicle (idx = int32 static, last = int32 static start time, table = int32 sequence ids, N = count):

  | function | idx | last | table | N | table contents [0..N-1] | table[N] |
  |---|---|---|---|---|---|---|
  | car 0x5267C0 | 0x8CCEEC | 0xB70120 | 0x8CC828 | 11 | 20,22,7,3,22,1,21,8,2,22,5 | 6 (0x8CC854) |
  | train 0x526950 | 0x8CCEF0 | 0xB70124 | 0x8CC858 | 6 | 20,22,2,21,3,0 | 0 (0x8CC870) |
  | heli 0x526AE0 | 0x8CCEF4 | 0xB70128 | 0x8CC874 | 7 | 26,27,23,20,22,28,23 | (unused) |
  | plane 0x526C80 | 0x8CCEF8 | 0xB7012C | 0x8CC894 | 6 | 26,27,20,22,28,23 | (unused) |
  | boat 0x526E20 | 0x8CCEFC | 0xB70130 | 0x8CC8B0 | 3 | 20,3,18 | (unused) |
- Fields: `m_bCinemaCamera` +0xCF0, `m_nModeObbeCamIsInForCar` +0xC34 (int32, shared by all five), active cam mode
  (this+0x180 + m_nActiveCam*0x238, u16).
- Behaviour (`tries` = 0):
  1. If `bDidWeProcessAnyCinemaCam == 0` (first frame of a cinema sequence):
     `idx = -1`; if `(int8)gbCineyCamMessageDisplayed > 0 && !m_bCinemaCamera`: `gbCineyCamMessageDisplayed -= 1;
     CHud::SetHelpMessage(TheText.Get("CINCAM"), true, false, false)`; `byte 0xB6EC34 = 1`; go to 3.
  2. Else (already running): heli/plane only: if the camera is under water (0x50B830) go to 3.
     `if (!IsItTimeForNewcam(table[idx], last))` -> go to 5 (keep the current camera; `last` unchanged).
  3. `idx = (idx + step) % N` (signed `idiv`); `if (idx < 0) idx = N - 1; else if (idx > N - 1) idx = 0` (dead).
     `if (TryToStartNewCamMode(table[idx]))` -> success.
     Loop: `while (tries <= N) { idx = next(idx) (same formula); tries++; if (TryToStartNewCamMode(table[idx])) break; }`
     (`cmp edi, N; jg` before each try; the counter is incremented before the call).
  4. Outcome (car/train): `last = now` (always, 0x5268E0), then if `tries < N` -> success, else failure:
     `idx = N; TryToStartNewCamMode(table[N])` (reads one element past the N-entry table: car 6, train 0).
     Outcome (heli/plane/boat): first try success -> `last = now`, success. After the loop: `tries < N` -> `last = now`,
     success; else failure: `idx = N`; if the active cam mode != 18 (MODE_CAM_ON_A_STRING): `TryToStartNewCamMode(29)`
     and `last = now` (mode 18: neither).
  5. `m_nModeObbeCamIsInForCar = idx`; `bDidWeProcessAnyCinemaCam = 1`.
- Oddities (keep): a sequence that only succeeds on the N-th or (N+1)-th retry is treated as a failure (the success test is
  `tries < N` after the loop); car/train failure reads `table[N]` past the end; on failure `m_nModeObbeCamIsInForCar` is set
  to N, so the next frame's `IsItTimeForNewcam(table[N], ...)` uses that out-of-range entry too.
- gta-reversed: not hooked; `Camera.cpp:1467` is `assert(0)` (car; same for train/heli/plane/boat).
- Open questions: none.

## 0x526950 CCamera::ProcessObbeCinemaCameraTrain  (catalogue: specified/REA, port level 12)
- As 0x5267C0 with the train row of the table (car/train outcome variant, failure uses `table[6]` = 0x8CC870 = 0).
- gta-reversed: not hooked, `assert(0)` (Camera.cpp:1423).

## 0x526AE0 CCamera::ProcessObbeCinemaCameraHeli  (catalogue: specified/REA, port level 12)
- As 0x5267C0 with the heli row; includes the under-water check in step 2 (0x526B48) and the heli/plane/boat outcome
  (failure: `idx = 7`, `TryToStartNewCamMode(29)` unless mode 18).
- gta-reversed: not hooked, `assert(0)` (Camera.cpp:1472).

## 0x526C80 CCamera::ProcessObbeCinemaCameraPlane  (catalogue: specified/REA, port level 12)
- As 0x5267C0 with the plane row; under-water check (0x526CEA); heli/plane/boat outcome (failure `idx = 6`, 29 unless
  mode 18). In this copy byte 0xB6EC34 is set before the message test (0x526CA6), same effect.
- gta-reversed: not hooked, `assert(0)` (Camera.cpp:1418).

## 0x526E20 CCamera::ProcessObbeCinemaCameraBoat  (catalogue: specified/REA, port level 12)
- As 0x5267C0 with the boat row; no under-water check; heli/plane/boat outcome (failure `idx = 3`, 29 unless mode 18).
- gta-reversed: not hooked, `assert(0)` (Camera.cpp:1462).

## 0x527FA0 CCamera::CamControl  (catalogue: specified/REA, port level 13)
Structured by block (3912 insns, complete coverage 0x527FA0-0x52B725 plus the four .text obfuscation stubs, all read from
the disassembly; the Ghidra decompile stops at the stubs). Confidence: medium (all branches and constants read from the
bytes; see Open questions for the three uninitialised-stack reads).
- Convention: `__thiscall void()`, `this` in ESI for the whole body, frame `sub esp,0xB4` + push ebx/ebp/esi/edi, plain `ret`.
  Caller: CCamera::Process 0x52B730 (0x52B840).
- Obfuscation stubs (part of the extent, plain jumps, no behaviour of their own):
  0x528697 -> 0x401B3B `eax = m_pToGarageWeAreIn (+0x81C)` -> 0x52869C;
  0x529138 -> 0x4038EB `push ebp; ebp = [0x1560774] + [0x6FBA21] = 0x3CCCCCCD; xchg [esp], ebp` (= `push 0x3CCCCCCD` (0.025f),
  EBP preserved) -> 0x52913D;
  0x52A00D -> 0x403817 `eax = *(~[0x156075C] ^ [0x463561]) = *(0xB7CD98)` (= `CWorld::Players[0].m_pPed`) -> 0x52A012;
  0x52B500 -> 0x403CBA `edx *= 0x238` -> 0x52B505.
- Callees: CEntity::RegisterReference 0x571B70 / CleanUpOldReference 0x571A00, CCullZones::CamCloseInForPlayer 0x72DD20,
  ::CamStairsForPlayer 0x72DD30, ::FindZoneWithStairsAttributeForPlayer 0x72DAD0 (returns zone* or null),
  0x72D6D0 (thiscall on a cull zone, `ret 4`, writes a CVector: `x = (w8 + w4)*0.5 + w0`, `y = (wA + w6)*0.5 + w2`,
  `z = (wE + wC)*0.5` from the zone's int16 words at +0..+0xE, each `fild`/`fiadd`), CPad::GetPad 0x53FB70,
  CPad 0x5404A0 (`bool`: cycle-camera button "just down", mode-dependent), CPad::sub_5404F0, CPad::sub_540530,
  CPad 0x540E80 / 0x540F80 (results unused, see 4c), CPad::GetPedWalkLeftRight 0x53FC90, ::GetPedWalkUpDown 0x53FD30,
  CPad::GetTarget 0x540670, ::GetEnterTargeting 0x5406B0, CGameLogic::IsCoopGameGoingOn 0x441390, CCamera::Restore 0x50B930,
  RestoreWithJumpCut 0x50BAB0, SetCameraDirectlyBehindForFollowPed_CamOnAString 0x50BD40, SetCamPositionForFixedMode 0x50BEC0,
  ClearPlayerWeaponMode 0x50AB10, GetArrPosForVehicleType 0x50AF00, StartTransition(int16 mode) 0x515200 (6 sites),
  ProcessObbeCinemaCamera{Car 0x5267C0, Train 0x526950, Heli 0x526AE0, Plane 0x526C80, Boat 0x526E20},
  CGarages::IsPointInAGarageCameraZone(CVector by value) 0x449BA0, CGarages::CameraShouldBeOutside 0x448650,
  CGarage::FindDoorsWithGarage(CObject** first, CObject** second) 0x449FF0 (thiscall, `ret 8`; zeroes both, scans the object
  pool for `m_nGarageDoorGarageIndex (+0x147)` == garage index; `second` is only ever set when `first` is set),
  CPlaceable::GetMatrix 0x411990, CPlaceable::GetHeading 0x441DB0, CGeneral::GetATanOfXY 0x53CC70, VectorSub(out,a,b)=a-b
  0x40FE60, 0x40FE30 (out = a + b, cdecl), 0x40FE90 (out = s * v; args (out, float s, const CVector* v)), 0x40FEC0
  (out = v * s; args (out, const CVector* v, float s)), CVector::Normalise 0x59C910 (zero length -> x = 1.0, y/z kept),
  CVector::Magnitude 0x4082C0, CWorld::GetIsLineOfSightClear 0x56A490, CWorld::ProcessLineOfSight 0x56BA00,
  CWorld::FindGroundZFor3DCoord 0x5696C0, FindPlayerPed 0x56E210, FindPlayerVehicle 0x56E0D0, CVehicle::GetVehicleAppearance
  0x6D1080, CCam::WellBufferMe 0x509AE0, CCam::GetWeaponFirstPersonOn 0x509DC0, CCam::Using3rdPersonMouseCam 0x50A850,
  CTaskManager::GetTaskSecondary 0x681810, ::FindActiveTaskByType 0x681740, CPedIntelligence::GetTaskJetPack 0x601110,
  ::GetTaskHold 0x600FF0, CPed::IsPedInControl 0x5E3960, CPed::CanWeRunAndFireWithWeapon 0x5E88E0,
  CAudioEngine::ReportFrontendAudioEvent 0x506EA0 (on AudioEngine 0xB6BC90).
- Globals: `REQ` = static int16 0xB70140 (the mode the camera should go to; persists across frames); int32 0x8CC824 (forced
  car mode, consumed when > 0, reset to -1); byte 0xB6EC34 (cleared at entry, tested at the end); byte 0xB7013C (ped-was-
  arrested latch), int32 0xB70138 (previous player ped state), int32 0xB70134 (stored arrest mode); byte 0xB7013D (syphon
  fixed-cam latch); CTimer::m_UserPause 0xB7CB49, m_CodePause 0xB7CB48, ms_fTimeStep 0xB7CB5C, m_snTimeInMilliseconds 0xB7CB84;
  CReplay::Mode 0xA43088; CWorld::PlayerInFocus 0xB7CD74, Players 0xB7CD98 (stride 0x190, m_pPed at +0); gbModelViewer
  0xBA6728; CCamera::m_bUseMouse3rdPerson 0xB6EC2E, CCamera::bDidWeProcessAnyCinemaCam 0xB6EC2D (W);
  CPostEffects::m_bSpeedFXUserFlagCurrentFrame 0x8D5109 (W); gPlayerPedVisible 0x8CC380; `TheCamera.m_fPedZoomSmoothed`
  0xB6F0FC (read through the global, not `this`, at 0x52A91C).
  .data tunables: 0x8CCF04 4.0, 0x8CCF08 3.0, 0x8CCF0C 2.0, 0x8CCF10 -10.0 (ped garage cam), 0x8CCF14 0.5 (ped zoom when the
  cull test failed), 0x8CCF18 2.0, 0x8CCF1C -10.0 (car garage cam), 0x8CC46C 30.0 (degrees). Tables 0x8CC3E0 / 0x8CC3F4 /
  0x8CC408 (f32 car zoom per arr-pos, zoom 1/2/3).
  .rdata: 0.12 [0x85A6F0], -0.65 [0x863274], 15.0 [0x858B48], 0.5 [0x858B8C], 0.0 [0x858B50], 1.0 [0x858624], 2^32 [0x858C54],
  2850.0 [0x863270], 0.2 [0x858CC4], 0.7 [0x858CB0], 3.75 [0x86326C], 2.1 [0x859974], 3.1 [0x863268], 1.15 [0x863264],
  4.0 [0x858B90], pi/2 [0x858FE4], pi [0x858CB8], 2pi [0x858CBC], 0.017453292 [0x8595EC], 2.0 [0x858CA0], 1.5 [0x858CE8],
  17.5 [0x863260]; immediates 0.65f (0x3F266666), 0.7f (0x3F333333), 5.6f (0x40B33333), 0.1f, 0.25f, 13.0f, 3.75f, 0.001f.
- Notation: `T` = `m_pTargetEntity (+0x958)`; `pos(e)` = `e->m_matrix ? m_matrix->pos : e->placement.pos` (evaluated where
  used); `type(e)` = `e->byte[0x36] & 7` (2 vehicle, 3 ped); `cam` = `m_aCams[m_nActiveCam]` (index re-read at every use);
  `other` = `m_aCams[(m_nActiveCam + 1) signed-mod 2]`; `pad` = `CPad::GetPad(0)` (fresh call per use); `ped0` =
  `FindPlayerPed(-1)` (fresh call); `Pf` = `Players[PlayerInFocus].m_pPed`; `LOS9(a,b)` =
  `GetIsLineOfSightClear(a, b, true, false, false, false, false, false, true)`. `Lxx` = stack slot `[esp+0xXX]` of the final
  frame (after the four pushes). Ped flags dword +0x46C: bit 0 bIsStanding, bit 8 bInVehicle, bit 26 bIsDucking. Ped +0x47C
  m_pIntelligence, +0x480 m_pPlayerData, +0x530 m_nPedState, +0x558 m_fCurrentRotation, +0x55C m_fAimingRotation, +0x58C
  m_pVehicle, +0x71C m_pTargetedObject. Vehicle +0x590 m_nVehicleType, +0x594 m_nVehicleSubType, entity +0x22 model.
  Garage (CGarage) +0x28 left, +0x2C right, +0x30 front, +0x34 back, +0x4C type. CPad words: +0x1A Select, +0x1C Square,
  +0x1E Triangle, +0x20 Cross, +0x22 Circle, +0x10E DisablePlayerControls.

### 1. Prologue (0x527FA0-0x5280CF)
`L94 = (int32)cam.m_nMode` (movsx, initial mode); clear +0x2E m_bObbeCinematicPedCamOn, +0x2F m_bObbeCinematicCarCamOn, +0x37
m_bUseTransitionBeta, +0x36 m_bUseSpecialFovTrain, +0x28 m_bJustCameOutOfGarage, +0x35 m_bTargetJustCameOffTrain, +0x22
m_bInATunnelAndABigVehicle, +0x20 m_bJustJumpedOutOf1stPersonBecauseOfTarget (in this order); `L10 = 0` (switch-now flag),
`L11 = 0` (on-stairs flag); `byte 0xB6EC34 = 0`. If `cam.m_pCamTargetEntity == null && T == null`: `T = Pf; RegisterReference(&T)`.
Cull zones: `m_nZoneCullFrameNumWereAt (+0xAC) += 1`; if `> m_nCheckCullZoneThisNumFrames (+0xA8)` (signed) it becomes 1;
`m_bCullZoneChecksOn (+0x1E) = (+0xAC == +0xA8)`; if that is 1: `m_bFailedCullZoneTestPreviously (+0x54) = CamCloseInForPlayer() != 0`.
If `m_bLookingAtPlayer (+0x2B) == 1`: `pad->byte[0x10E] &= 0xFE`; `ped0->dword[0x1C] |= 0x80` (bIsVisible).
Registers set here and relied on later: EBP = 18, EBX = 2, EDI = 6.
If `m_UserPause || m_CodePause || m_bIdleOn (+0x21)` -> block 6 entry 0x529FFB (`di = REQ`).
Else `L20 = 0` (car WellBuffer target), `L3C = 0` (ped WellBuffer target);
if `m_bTargetJustBeenOnTrain (+0x34) == 1` and `!(type(T) == 2 && T->dword[0x590] == 6)`: `Restore(); m_bTargetJustCameOffTrain = 1;
m_bTargetJustBeenOnTrain = 0; m_bWantsToSwitchWidescreenOff (+0x3C) = (m_bWideScreenOn (+0x3D) == 1)`.
`type(T) == 2` -> block 2; `== 3` -> block 4; otherwise -> 0x529FFB.

### 2. Vehicle target: mode, zoom cycling, garages (0x52812F-0x528AC1)
a. (0x52812F) `if (int32 0x8CC824 > 0) { REQ = (int16)that; 0x8CC824 = -1; }`. If `T->m_nVehicleType == 6` (train):
   `di = 3; REQ = 3` -> 0x52A002 (no zoom/garage code).
b. Zoom cycling (0x528160): if `(pad->0x5404A0() || pad->sub_5404F0()) && CReplay::Mode != 1 && !m_bWideScreenOn`:
   if `m_bFailedCullZoneTestPreviously` -> skip to c; else if `(m_bLookingAtPlayer == 1 || m_nWhoIsInControlOfTheCamera (+0xB0) == 2)
   && !IsCoopGameGoingOn()`: `m_nCarZoom (+0xB4) += pad->0x5404A0() ? -1 : +1`; `if (> 5) = 0; else if (< 0) = 5;` then
   `if (zoom == 4) zoom = pad->0x5404A0() ? 3 : 5; else if (zoom == 0 && m_bDisableFirstPersonInCar (+0x45)) zoom = pad->0x5404A0() ? 5 : 1;`
   (0x5404A0 is called again for each test).
c. (0x528266) `if (m_bFailedCullZoneTestPreviously && m_nCarZoom != 4 && m_nCarZoom != 0) REQ = 18`.
d. (0x528283) `vt = T->m_nVehicleType`: boat (5) with model != 460 -> `REQ = 22` (BEHINDBOAT), go to e; boat with model 460
   (Skimmer) is treated as vt 0; vt not in {0, 9} -> go to e (REQ untouched). For vt 0 / 9 / Skimmer:
   - `stairs (L4C, EDI) = 0`; only if vt == 9 (bike; the Skimmer path has vt forced to 0): `if (CamStairsForPlayer())
     { stairs = FindZoneWithStairsAttributeForPlayer(); if (stairs) L11 = 1; }`.
   - If `!IsPointInAGarageCameraZone(pos(T)) && !L11`: `if (m_bPlayerIsInGarage (+0x26)) { m_bJustCameOutOfGarage = 1;
     m_bPlayerIsInGarage = 0; }`, `m_bGarageFixedCamPositionSet (+0x4E) = 0`, `REQ = 18` -> e.
   - If not `((!m_bGarageFixedCamPositionSet && m_bLookingAtPlayer == 1) || who == 2)` -> d-tail. If `m_pToGarageWeAreIn == null
     && stairs == null` -> d-tail.
   - Garage (`g = m_pToGarageWeAreIn` non-null, 0x528380): `g->FindDoorsWithGarage(&L74 /*first*/, &L78 /*second*/)`;
     first: `sel (EDI) = 1; L30 = (first.pos.x, first.pos.y, 0)`; else second: `sel = 2` (unreachable, see the callee);
     else `sel = 1; L30.xy = pos(T).xy; L48 = 0` (L38 not written here).
   - Stairs, no garage (0x528445): `L30 = cam.m_vecSource (+0x19C)`; `L50 = pos(T)`; `sel = 1`;
     `if (!(sqrt((L50.x - L30.x)^2 + (L50.y - L30.y)^2) > 15.0f))` -> stub (x87: `dy*dy` formed first, then `dx*dx`, sum `dx2 + dy2`).
     Else `L40 = Normalise(VectorSub(pos(T), &L50) with z = 0)`: **both operands are pos(T)**, so L40 = (1, 0, 0) always
     (original bug; the ped variant 3d uses the zone centre). Extent: with the zone words a=+4, b=+6, c=+8, d=+0xA,
     `n = (|a| + |c| > |b| + |d|) ? |a| + |c| : |b| + |d|`; `k = (float)n + (float)n` (fild, `fadd st0,st0`, stored f32).
     `W = pos(T) + k*L40` (0x40FE90 then 0x40FE30); `if (LOS9(pos(T), W)) L30 = W; else { W = pos(T) - k*L40; if (LOS9(pos(T), W)) L30 = W; }`.
   - Stub 0x401B3B -> 0x52869C: if g: `L50 = ((g.right + g.left)*0.5f, (g.back + g.front)*0.5f, 0)` (fld right, fadd left, fmul);
     else `L38 = 0`; and if `stairs == null`: `L50 = (pos(T).x, pos(T).y, 0)` (stairs keeps L50 = pos(T) incl. z).
   - `D = (sel == 1 ? L30 : L40) - L50` (per component, stored f32) into L7C..L84; `FindGroundZFor3DCoord(pos(T), &L13, nullptr)`,
     result popped unused; `L88 = pos(T)`; `L84 = 0; Normalise(L7C)` (D is never used afterwards: dead).
   - Doors (0x528800): `L30 = L40 = 0`; `if (L74) { L30 = pos(L74->m_pDummyObject (+0x170)); L24 = dummy->GetMatrix()->right; }`
     `if (L78) { L40 = pos(L78->m_pDummyObject); L24 = that dummy's GetMatrix()->right; }`.
     Base L64: both doors: `L64 = L30 + (L40 - L30)*0.5f` (x: product stored f32 then `+ L30.x`; y in register; z: difference stored
     f32, then `*0.5 + L38`); only first: `L64 = L30`; only second: `L64 = L40`; none: `L64 = L88` (= pos(T)) and
     `L24 = Normalise((L88.x - L50.x, L8C - L50.y, 0))`.
     `C = L64 + L24 * -10.0f [.data 0x8CCF1C]; C.z = C.z + 2.0f [.data 0x8CCF18]`;
     `SetCamPositionForFixedMode(&C (L30), &(0,0,0) (L14))`; `m_bGarageFixedCamPositionSet = 1`.
     NOTE: when no garage (stairs path), L74/L78 are never written in this function (uninitialised stack, see Open questions).
   - d-tail (0x528A6E): `if ((CameraShouldBeOutside() || L11) && m_bGarageFixedCamPositionSet == 1 && (m_bLookingAtPlayer == 1 ||
     who == 2) && (m_pToGarageWeAreIn || stairs)) { REQ = 15; m_bPlayerIsInGarage = 1; }` else `{ if (m_bPlayerIsInGarage)
     { m_bJustCameOutOfGarage = 1; m_bPlayerIsInGarage = 0; } REQ = 18; }`.
e. Car zoom (0x528AC6): `L5C = 0; arr: GetArrPosForVehicleType(T->GetVehicleAppearance(), &L5C)`; `z = m_nCarZoom`:
   `z == 0 && !m_bPlayerIsInGarage`: `m_fCarZoomBase (+0xB8) = 0; REQ = di = 16`; z 1/2/3: `base = table{0x8CC3E0,0x8CC3F4,0x8CC408}[L5C]`;
   then `di = REQ` (except the z == 0 case); `if (z == 4 && !m_bPlayerIsInGarage) base = 1.0f`.
   `if (m_fCarZoomTotal (+0xBC) == 0.0f) m_fCarZoomTotal = base` (`test ah,0x44; jp` = not equal or NaN skips).
   Smoothing of `s = m_fCarZoomSmoothed (+0xC0)` with `step = ms_fTimeStep * 0.12f`:
   - `m_bUseScriptZoomValueCar (+0x39) == 1`, target `v = m_fCarZoomValueScript (+0xC4)`: if `s < v`: `n = step + s; s = (n > v) ? v : n`;
     else (`s >= v` or NaN): `n = s - step; s = (n >= v || NaN) ? n : v`.
   - else if `m_bFailedCullZoneTestPreviously`: `L20 = 0.65f`; same rule toward -0.65f [0x863274] (`s < -0.65`: `n = step + s`,
     `s = (-0.65 >= n || NaN) ? n : -0.65`; else `n = s - step`, `s = (-0.65 <= n || NaN) ? n : -0.65`).
   - else toward `base` (same rule as the script case), and then `if (z == 3 && base == 0.0f) s = base`.
   `WellBufferMe(L20, &cam.m_fCloseInCarHeightOffset (cam+0x64), &cam.m_fCloseInCarHeightOffsetSpeed (cam+0x68), 0.1f, 0.25f, false)`
   -> 0x52A002 with `di` as above.

### 3. Ped target (0x528D19-0x529FF4)
a. Zoom cycling: if `(pad->0x5404A0() || pad->sub_5404F0()) && CReplay::Mode != 1 && !m_bWideScreenOn && !m_bFailedCullZoneTestPreviously
   && !m_bFirstPersonBeingUsed (+0x1F) && (m_bLookingAtPlayer == 1 || who == 2) && !IsCoopGameGoingOn()`:
   `m_nPedZoom (+0xC8) += pad->0x5404A0() ? -1 : +1; if (> 3) = 1; else if (< 1) = 3;`. Then always `REQ = 4`.
b. First person (0x528DBD): if `(m_bLookingAtPlayer == 1 || m_bEnable1rstPersonCamCntrlsScript (+0x41)) && type(T) == 3 &&
   (!m_bWideScreenOn || +0x41) && !m_aCams[0].Using3rdPersonMouseCam()` (**cam 0**, not the active cam):
   `if (ped0->m_pIntelligence->m_TaskMgr.GetTaskSecondary(0) == null) { if (pad->0x540E80() == 0) pad->0x540F80(); }`
   (results unused); then if `m_bFirstPersonBeingUsed`: clear it (0x528F2D) if any of `pad->GetPedWalkLeftRight()`,
   `GetPedWalkUpDown()`, words +0x1C, +0x1E, +0x20, +0x22, +0x1A is non-zero, or if `(float)(uint32)(now - m_nFirstPersonCamLastInputTime (+0x60))
   > 2850.0f` (fild + 2^32 fix-up; `je` when greater); otherwise `if (pad->GetEnterTargeting()) { m_bJustJumpedOutOf1stPersonBecauseOfTarget = 1; clear; }`.
   If the outer condition fails: clear.
   Then (0x528F31) clear also `if (!ped0->IsPedInControl() || ped0->m_pPlayerData->m_fMoveBlendRatio (+0x14) > 0.0f)`.
   If still set: `REQ = 16; pad->byte[0x10E] |= 1`.
c. Zoom smoothing: `m_fPedZoomBase (+0xCC) = cam.m_fTargetZoomGround{One (+0xDC) if zoom 1, Three (+0xE4) if zoom 3, Two (+0xE0) otherwise}`;
   smoothing of `m_fPedZoomSmoothed (+0xD4)` exactly as 2e with: script `m_bUseScriptZoomValuePed (+0x38) == 1` toward
   `m_fPedZoomValueScript (+0xD8)`; else cull failed: `L3C = 0.7f` and toward 0.5f [.data 0x8CCF14] (same comparison forms as the
   -0.65 case); else toward base and `if (zoom == 3 && base == 0) s = base`.
   `WellBufferMe(L3C, &cam.m_fCloseInPedHeightOffset (cam+0x5C), &cam.m_fCloseInPedHeightOffsetSpeed (cam+0x60), 0.1f, 0.025f, false)`
   (0.025f pushed by stub 0x4038EB).
d. Garage / stairs (0x52915A): `stairs (L70, EBP) = 0; if (CamStairsForPlayer()) { stairs = FindZone...(); if (stairs) L11 = 1; }` (any ped).
   - `!IsPointInAGarageCameraZone(pos(T)) && !L11`: garage-exit flags as 2d, `m_bGarageFixedCamPositionSet = 0` -> e.
   - If `m_bGarageFixedCamPositionSet || m_bLookingAtPlayer != 1` -> d-tail. If `!g && !L11` -> d-doors.
   - g: `L20 = T` then `g->FindDoorsWithGarage(&L20 /*first*/, &L3C /*second*/)`; first: `sel (EBX) = 1, L30 = (first.x, first.y, 0)`;
     second only: `sel = 2` (EBX still holds the constant 2; unreachable); none: `sel = 1, L30.xy = pos(T).xy, L48 = 0`.
   - Stairs, no g (0x5292D5): `L30 = cam.m_vecSource; sel = 1`; `L50 = 0x72D6D0(stairs)` (zone centre);
     `L40 = Normalise(VectorSub(pos(T), L50) with z = 0)`; `k` as in 2d; `W = pos(T) + k*L40`, LOS9 / `pos(T) - k*L40` LOS9 fallback
     exactly as 2d (no 15-unit distance gate here).
   - (0x529507) `L50 = g ? ((right + left)*0.5, (back + front)*0.5, 0) : (stairs ? L50 : (pos(T).x, pos(T).y, 0))`, `L38 = 0` when no g.
     `D (L7C) = (sel == 1 ? L30 : L40) - L50`; `gz = FindGroundZFor3DCoord(pos(T), &L13, nullptr)` stored to L3C; `L88 = pos(T)`;
     `if (!L13) L3C = pos(T).z - 0.2f`; `D.z = 0`.
     sel == 1: `if (!g && L11) { Normalise(D); if (stairs) { n as in 2d; k2 = (float)n * 0.7f + 3.75f; L30 = L50 + D*k2; }
     else L30 = L30 + D*3.75f; } else { Normalise(D); L30 = L30 + D*13.0f; }`; sel == 2 (dead): `Normalise(D); L30 = L40 + D*13.0f`.
     `if (m_nPedZoom == 4 && !L11) { L30.xy = L50.xy; L30.z = L50.z + ped0.pos.z + 2.1f; if (g && L30.z > g->+0x2C) L30.z = g->+0x2C; }`
     else `L30.z = L3C + 3.1f`. `SetCamPositionForFixedMode(&L30, &(0,0,0)); m_bGarageFixedCamPositionSet = 1`; falls into d-doors.
   - d-doors (0x52990C): `L64 = L24 = 0`; if `!g` -> d-tail. `g->FindDoorsWithGarage(&L3C /*first*/, &L20 /*second*/)`;
     first: `L64 = pos(first->m_pDummyObject)`, `L14 = dummy->GetMatrix()->right`; second: `L24 = pos(second dummy)`, `L14 = its right`;
     base `L40`: both `L64 + (L24 - L64)*0.5f` (same x87 shape as 2d), first only `L64`, second only `L24`, none `L88` with
     `L14 = Normalise((L88.x - L50.x, L8C - L50.y, 0))`. `C = L40 + L14 * -10.0f [.data 0x8CCF10]; C.z += 2.0f [.data 0x8CCF0C]`;
     `SetCamPositionForFixedMode(&C, &(0,0,0)); m_bGarageFixedCamPositionSet = 1`. So with a garage the fixed position is set
     twice in the same frame; the door-based one wins.
   - d-tail (0x529B9E): `if ((CameraShouldBeOutside() || L11) && m_bLookingAtPlayer == 1 && m_bGarageFixedCamPositionSet == 1 &&
     (g || L11)) { REQ = 15; m_bPlayerIsInGarage = 1; }` else `{ garage-exit flags; REQ = 4; }`.
e. Weapon modes (0x529BF4): `wm = m_PlayerWeaponMode.m_nMode (+0x830)`; `if (!pad->GetTarget() && wm != 0 && wm != 45 && wm != 55 &&
   wm != 65 && !(wm == 46 && ped0->m_pAttachedTo (+0xFC))) ClearPlayerWeaponMode()`.
   `di = m_PlayerMode.m_nMode (+0x824); if (di) REQ = di; else di = REQ`. `bp = m_PlayerWeaponMode.m_nMode` (re-read);
   `if (bp == 0 || L11)` -> 0x52A002. If `bp in {7, 8, 51, 34, 45, 46}` or `cam.GetWeaponFirstPersonOn()` -> f.
   `if (di == 37)` -> 0x52A002. `L12 = 0`; `L24 = pos(T)`; `L14 = m_vecAimingTargetCoors (+0x854) - L24`; `L40 = cam.m_vecSource - L24`;
   `L20 = 0`. `if (!ped0->m_pTargetedObject && !(ped0->m_pPlayerData->byte[0x34] & 8 /*m_bFreeAiming*/))` -> 0x529FFB.
   If the targeted object is a ped with `m_nPedState` 55 or 54: `L20 = 3.0f [.data 0x8CCF08]; L12 = 1`.
   `L78 = sqrt(L18*L18 + L14*L14)` (y term first); `L3C = GetATanOfXY(L40.x, L40.y)`; `GetATanOfXY(L14.x, L14.y)` (result popped, dead).
   `di = wm; REQ = wm; L74 = 0`.
   - `wm == 53` (AIMWEAPON): `if (!L12)` -> 0x52A07A. If `ped0->m_pTargetedObject && (!m_bTransitionState || cam.m_nMode == 35)`:
     `thr = (cam.m_nMode == 35 && type(targeted) == 3) ? 4.0f [.data 0x8CCF04] : L20`; `if (L78 < thr) { L74 = 5.6f; REQ = di = 35; }`
     (`test ah,5; jp` skips on >= or NaN). Otherwise `di = REQ`.
   - `di != 35` -> 0x52A07A (`byte 0xB7013D = 0` -> 0x52A002).
   - `di == 35` (0x529EFA; also reached with `wm == 35` directly, then L74 = 0 and C = pos(T) + (0, 0, 1.15)): `if (byte 0xB7013D)` -> 0x52A002. `L20 = 0`; `C.x = cos(L3C)*L74 + pos(T).x; C.y = sin(L3C)*L74 + pos(T).y;
     C.z = pos(T).z + 1.15f` (fld angle, fcos/fsin, fmul, fadd, each stored f32). `if (ProcessLineOfSight(pos(T), C, &L98 colpoint,
     &L20 entity, true, false, false, true, false, true, true, false)) SetCamPositionForFixedMode(&L98.point, &(0,0,0)); else
     SetCamPositionForFixedMode(&C, &(0,0,0));` `byte 0xB7013D = 1` -> 0x529FFB.
f. First-person weapon (0x52A086): `if (Pf->m_nPedState == 24 && di != 37 && !cam.GetWeaponFirstPersonOn()) REQ = di = 4; else REQ = di = bp;`
   -> 0x52A002.

### 4. Common part (0x529FFB-0x52A4AA)
a. 0x529FFB: `di = REQ`.
b. Co-op (0x52A002): if `m_bCooperativeCamMode (+0x43)`: `p0 = Players[0].m_pPed` (stub 0x403817), `p1 = *(0xB7CF28)` (Players[1]);
   if both exist: if both have bInVehicle and both `m_pVehicle` non-null: `T = p0->m_pVehicle` (raw store, no reference
   bookkeeping); `di = (p0.veh == p1.veh) ? (m_bAllowShootingWith2PlayersInCar (+0x44) ? +0x48 : +0x4A) : +0x46`; else `di = +0x4C`
   (m_nModeForTwoPlayers{SameCarShootingAllowed, SameCarShootingNotAllowed, SeparateCars, NotBothInCar}); `REQ = di`.
c. Arrest latch (0x52A0F8): `st = Pf->m_nPedState; L13 = 0; if (st == 63) byte 0xB7013C = 1; else if (byte 0xB7013C) { L13 = 1; byte 0xB7013C = 0; }`.
   `go = (dword 0xB70138 != 63 && st == 63) ? !(m_nCarZoom == 0 && type(T) == 2) : false; dword 0xB70138 = st;`
   `if (go) { di = 32; dword 0xB70134 = 32; cam.m_bResetStatics (+0xA) = 1; REQ = 32; } else if (st == 63) { di = (int16)dword 0xB70134; REQ = di; }`.
d. Dead player (0x52A1A8): if `Pf->m_nPedState == 55`: `m_bObbeCinematicCarCamOn = 0`; `cm = cam.m_nMode`; `cm == 29` -> di = 29;
   `cm == 32` -> di = 32; else if `type(T) == 3`: for `i = 0..15` over `T->m_pIntelligence + 0x130` (16 entity pointers, gr
   m_pedScanner entities): `e` non-null, `t = e->m_pIntelligence->m_TaskMgr.FindActiveTaskByType(1100 /*TASK_SIMPLE_ARREST_PED*/)`,
   `t && t->dword[8] == ped0 && |pos(e) - pos(T)| < 4.0f` -> `cam.m_bResetStatics = 1; di = 32` (0x52A4E3); loop end (or T not a ped):
   `cam.m_bResetStatics = 1; di = 29`. `REQ = di`.
e. Restore by jump cut (0x52A2C7): if `m_bRestoreByJumpCut (+0x30) == 1`: if `di not in {4, 34, 7, 8, 51, 46, 11, 28, 35, 18, 3}` and
   `!m_bUseMouse3rdPerson`: `SetCameraDirectlyBehindForFollowPed_CamOnAString()`. Then `cam.m_nMode = REQ = m_nModeToGoTo (+0xC38)`
   (DI is not updated); `m_bJust_Switched (+0x2A) = 1; cam.m_bResetStatics = 1; cam.m_vecCamFixedModeVector (+0x154) =
   m_vecFixedModeVector (+0x860)`; `if (cam.m_pCamTargetEntity) CleanUpOldReference(&it); it = T; RegisterReference(&it)`;
   `cam.m_vecCamFixedModeSource (+0x160) = m_vecFixedModeSource (+0x86C); cam.m_vecCamFixedModeUpOffSet (+0x16C) = +0x878;
   cam.m_bCamLookingAtVector (+2) = 0; cam.m_vecLastAboveWaterCamPosition (+0x178) = other.same; m_bRestoreByJumpCut = 0;
   cam.m_bResetStatics = 1; m_fCarZoomSmoothed = m_fCarZoomBase; m_fPedZoomSmoothed = m_fPedZoomBase; m_bTransitionState (+0x58) = 0;
   m_bDoingSpecialInterp (+0x4F) = 0`.
f. `if (gbModelViewer) REQ = 9`.

### 5. Cinema cameras (0x52A4BC-0x52A750)
`L12 = 1`. If T: vehicle -> `if (m_nCarZoom == 5) m_bObbeCinematicCarCamOn = 1`; else `if (m_nPedZoom == 5) m_bObbeCinematicPedCamOn = 1`.
`if (FindPlayerVehicle(-1,false) && it->m_nVehicleType == 6) m_bObbeCinematicCarCamOn = 1`.
`dx = REQ`, except: if `T && type(T) == 2 && ped0 && ped0->m_nPedState in {63, 55}`: `m_bObbeCinematicPedCamOn = 0; L12 = 0;
REQ = dx = (state == 63) ? 32 : 29`.
`if (m_bTargetJustBeenOnTrain == 1 || dx in {29, 23, 28, 11, 7, 35, 8, 51, 32, 33, 34, 36, 39, 40, 52, 42, 43, 41, 45, 46} || who == 1 ||
m_bJustCameOutOfGarage || m_bPlayerIsInGarage || cam.m_nMode == 29) L12 = 0`. EBP = 1 from here on.
`if (m_bCinemaCamera (+0xCF0)) { m_bObbeCinematicCarCamOn = 1; L12 = 1; }`.
- `m_bObbeCinematicPedCamOn && L12 == 1` -> block 6 directly (nothing called).
- else `m_bObbeCinematicCarCamOn && L12 == 1`: `CPostEffects::m_bSpeedFXUserFlagCurrentFrame = 0`; if `type(T) == 2`: `sub = T->m_nVehicleSubType`;
  `sub == 4` -> ProcessObbeCinemaCameraPlane; else `T->GetVehicleAppearance() == 3` -> Heli; else `sub == 5` -> Boat; `sub == 6` -> Train;
  else Car. `dx = REQ` after the call.
- else: `if (m_bPlayerIsInGarage && m_bObbeCinematicCarCamOn) L10 = 1; L12 = 0; bDidWeProcessAnyCinemaCam = 0`.

### 6. Mode decision, looking at the player (0x52A757-0x52B210)
If `m_bLookingAtPlayer != 1` -> block 7. `ax = cam.m_nMode` (re-read where noted), `dx = REQ`.
a. `dx in {1, 16, 37}` -> L10 = 1; `dx in {18, 22}` and `ax in {1, 16, 37}` -> L10 = 1; `dx == 15`: `if (ax == 1) L10 = 1`, go to c.
b. If `dx in {53, 55, 65}` and `T && type(T) == 3`: `jet = T->m_pIntelligence->GetTaskJetPack() != null`.
   If `REQ == 53 && cam.m_nMode == 4 && !jet`: `a = (T->m_pTargetedObject ? atan2(-(pos(tgo).x - pos(T).x), pos(tgo).y - pos(T).y)
   (fchs, fpatan) : T->GetHeading()) - pi/2`; `h = cam.m_fHorizontalAngle (+0xBC)`; `if (a > h + pi) a -= 2pi; else if (a < h - pi)
   a += 2pi;` (second test `test ah,5`: NaN/>= skip); `if (30.0f[.data] * 0.017453292f < fabs(a - h)) L10 = 1; else if
   ((TheCamera.m_fPedZoomSmoothed + 2.0f) * 1.5f < |pos(T) - m_mCameraMatrix.pos (+0x9A4)|) L10 = 1;`
   `if (m_bUseMouse3rdPerson) L10 = 0;` go to c. Otherwise (`REQ != 53 || mode != 4 || jet`): `dx = REQ; L10 = 1`.
   After b (and when b does not apply, 0x52A958): `dx == 48 && cam.m_nMode != 49` or `dx == 49 && cam.m_nMode != 48` -> L10 = 1
   (a subset of c).
c. (0x52A994) `ax = cam.m_nMode; dx = REQ`; `if ((ax == 48 && dx != 49) || (ax == 49 && dx != 48)) L10 = 1`.
d. `dx == 1`: `if (ax == 37 || ax == 29) { L10 = 0; goto g-test }` else go to h. `dx == 37`: same with `ax == 1 || ax == 29`.
e. `dx in {16, 7, 34, 8, 51, 39, 40, 52, 42, 43, 41, 45, 32, 33, 46}` and `type(T) == 3` -> L10 = 1, go to h.
   `dx == 15`: `if (m_bPlayerIsInGarage != 1)` -> g-test; else `if ((ax in {7, 45, 8, 51, 34, 37} || L11 || ax in {16, 39, 40, 52, 42, 43,
   41, 46}) && T && type(T) == 2) L10 = 1`; go to h.
f. `dx == 4` (0x52AAFD): `bl = 0`; if `ax == 53 && T->CanWeRunAndFireWithWeapon() && !(T flags & bIsDucking)`: `a = T->GetHeading() - pi/2`,
   wrapped against `h = cam.m_fHorizontalAngle` as in b; `bl = (30deg < fabs(a - h)) || !(T flags & bIsStanding)`; `if (m_bUseMouse3rdPerson)
   { bl = 0; m_bJustCameOutOfGarage = 1; }`.
   If `cam.m_nMode in {16, 7, 34, 8, 51, 29, 32, 33, 30, 39, 40, 52, 42, 43, 41, 45, 1, 37, 46} || bl || L13`: if `!m_bJustCameOutOfGarage`:
   `if (mode in {7, 8, 51, 34, 16, 39, 40, 52, 42, 43, 41, 45, 46}) { b = GetATanOfXY(cam.front.x, cam.front.y) - pi/2; T->m_fCurrentRotation =
   T->m_fAimingRotation = b; }` `m_bUseTransitionBeta = 1; L10 = 1;` `cam.m_fTransitionBeta (+0x9C) = (mode == 37) ?
   GetATanOfXY(0.001f, 1.0f) : GetATanOfXY(cam.front.x, cam.front.y) + pi` (for 37 the code first computes
   `Normalise((cam.source - ped0.pos) with z = 0)` into L14 and then overwrites L14.xy with (0.001, 1.0): dead). Go to h.
   dx not 4 (0x52ADB3): `dx in {38, 32, 33, 29}` -> L10 = 1, go to h; else g-test. (Any dx not handled by d/e/f also ends here.)
g. g-test (0x52ADCB): `if (ax == 29) L10 = 1`.
h. (0x52ADD6) `bp = cam.m_nMode`; `if (dx != bp && !cam.m_pCamTargetEntity) L10 = 1`. If `m_bPlayerIsInGarage` (bl):
   `if (g && g->m_nType in {2, 3, 4} && type(T) == 2 && T->model == 423 && dx != bp) L10 = 1`; if `cam.m_pCamTargetEntity` (e):
   `a = pos(e) - m_vecFixedModeSource; b = pos(e) - cam.m_vecSource; if ((b.x*a.x + b.z*a.z) + b.y*a.y < 0.0f) L10 = 1`.
i. Switch (0x52AEC8):
   - `dx != bp && !m_bTransitionState`: `L10 == 1` -> jump cut (j); else `if (!m_bWaitForInterpolToFinish (+0x3A)) StartTransition(dx)`.
   - `dx != bp && m_bTransitionState && L10 == 1` -> jump cut (j).
   - else if `m_bTransitionState && dx != bp`: if `m_bWaitForInterpolToFinish || !m_bLookingAtPlayer` -> k; else
     `d = ped0.pos - m_mCameraMatrix.pos` (three FindPlayerPed calls, each component stored f32); if `T && type(T) == 3 && |d| > 17.5f`:
     `dx = REQ; if (dx == 11 || dx == 28) m_bWaitForInterpolToFinish = 1`. k: `if (m_bWaitForInterpolToFinish != 1)
     { m_bDoingSpecialInterp = 1; StartTransition(dx); }`.
   - else (`!m_bTransitionState || dx == bp`): `if (dx == 15 && T != cam.m_pCamTargetEntity && m_bPlayerIsInGarage) { if (m_bTransitionState)
     m_bDoingSpecialInterp = 1; StartTransition(15); }`.
j. Jump cut (0x52AF2B): `if (!(m_bPlayerIsInGarage && !m_bJustCameOutOfGarage) && dx not in {4, 34, 7, 8, 51, 46, 11, 16, 28, 35} &&
   !m_bUseMouse3rdPerson) { SetCameraDirectlyBehindForFollowPed_CamOnAString(); dx = REQ; }`; `cam.m_nMode = dx; m_bJust_Switched = 1`;
   fixed-mode vector, target reference, fixed source and up offset copied as 4e; `cam.m_bCamLookingAtVector = m_bLookingAtVector (+0x2C)`;
   last-above-water from `other`; zoom smoothed = bases; `m_bTransitionState = m_bDoingSpecialInterp = m_bStartInterScript (+0x32) = 0;
   cam.m_bResetStatics = 1`.

### 7. Not looking at the player (0x52B215-0x52B4F5)
`bl = 0; L13 = 0`. If `m_bEnable1rstPersonCamCntrlsScript == 1 || m_bAllow1rstPersonWeaponsCamera (+0x42) == 1`:
`dx == 16`: `bl = (cam.m_nMode != 16)`; else if `m_PlayerWeaponMode.m_nMode in {7, 16, 8, 51} && pad->GetTarget() && +0x42`: `bl = L13 = 1`;
else `if (cam.m_nMode != m_nModeToGoTo) { m_bStartInterScript = 1; m_nTypeOfSwitch (+0xC3C) = 2; pad->byte[0x10E] &= 0xFE; }`; `dx = REQ`.
Then: `m_bStartInterScript == 1 && m_nTypeOfSwitch == 1` -> `REQ = m_nModeToGoTo; if (m_bTransitionState) m_bDoingSpecialInterp = 1;
StartTransition(m_nModeToGoTo)`; else if `(m_bStartInterScript == 1 && m_nTypeOfSwitch == 2) || bl`: jump cut: `m_bTransitionState =
m_bDoingSpecialInterp = 0; cam.m_nMode = (+0x41 == 1 && dx == 16) ? 16 : (L13 ? m_PlayerWeaponMode.m_nMode : m_nModeToGoTo)`;
`m_bJust_Switched = 1; cam.m_bResetStatics = 1`; vectors / target / looking-at-vector / last-above-water as 6j; zoom smoothed = bases.

### 8. Tail (0x52B4FB-0x52B725)
`m_bStartInterScript = 0`; `if (!cam.m_pCamTargetEntity) { it = T; RegisterReference(&it); }`.
Player visibility: if `cam.m_nMode == 17` or (`type(T) == 3` and mode in {16, 7, 34, 46, 45, 8, 51}): if `ped0` is visible (flags
byte bit 7): clear it and `h = ped0->m_pIntelligence->GetTaskHold(0); if (h && h->dword[8]) h->dword[8]->flags &= ~0x80`;
else `ped0->flags |= 0x80`. If `cam.m_nMode == 15`: `ped0` bit 7 := bit 0 of `gPlayerPedVisible`.
`bl = 0; if (L12 == 0 && who == 2) { RestoreWithJumpCut(); bl = 1; m_bCamDirectlyBehind (+0x1A) = 1; if (ped0)
m_fPedOrientForBehindOrInFront (+0x120) = GetATanOfXY(ped0->m_matrix->up.x, ped0->m_matrix->up.y); }` (m_matrix not null-checked).
Sound: if `(L94 != cam.m_nMode || bl || mode == 4 || mode == 18) && pad->sub_540530() && CReplay::Mode != 1 &&
(m_bLookingAtPlayer == 1 || who == 2) && !m_bWideScreenOn && (who != 2 || (byte 0xB6EC34 == 1 && pad->word[0x10E] == 0))`:
`AudioEngine.ReportFrontendAudioEvent(32, 0.0f, 1.0f)`.

- Oddities (keep): 2d stairs push-out direction is always +X (VectorSub of pos(T) with its own copy); 2d/3d fixed camera
  computed twice per frame for ped garages; the "second door only" branches are unreachable; 3d clamps the camera Z with
  garage field +0x2C (gr `m_fRightCoord`); 3b calls 0x540E80/0x540F80 and 3e/6f compute values that are discarded; 6b reads
  `TheCamera.m_fPedZoomSmoothed` through the global; 6f sets `m_bJustCameOutOfGarage` when the mouse camera is used; the
  co-op block writes `T` without reference counting; byte 0xB6EC34 is cleared at entry and never set here, so with who == 2 the
  sound needs a callee to set it; 4e leaves DI stale (later code reloads REQ into DX).
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod` (no C++ body).
- Open questions: (1) 0x528800 / 0x52887A: on the vehicle stairs path without a garage, L74/L78 (door pointers) are read
  without being written in this function (stack garbage from earlier calls); a port must decide (treating them as null gives
  base = pos(T)). (2) 0x52876E / 0x5295D5: `sel == 2` reads L40 uninitialised but is unreachable (FindDoorsWithGarage sets
  `second` only after `first`). (3) 0x52871B (vehicle path, garage without doors): L38 is uninitialised but only feeds D.z,
  which is overwritten. (4) Exact semantics of CPad 0x5404A0 / sub_5404F0 / sub_540530 / 0x540E80 / 0x540F80 belong to
  INPUT/CPad.md. (5) 0x52A20B: the +0x130 entity array is gr's ped-scanner list by layout only (not re-derived).

## 0x52B730 CCamera::Process  (catalogue: specified/REA, port level 24)
Structured by block (1495 insns). Derived from the bytes with the Ghidra decompile as the reading aid; x87 order is spelled
out where it was checked against the disassembly (marked "[x87 checked]"); elsewhere the stated order is the decompile's
and must be re-checked with `tools/specs/dis.py 0x52B730` when porting (confidence medium).
- Convention: `__thiscall void()`, plain `ret`. Callers: CGame::Initialise 0x53BC80, CGame::ReInitGameObjectVariables
  0x53BCF0, CGame::Process 0x53BEE0, CPlayerInfo::Process 0x56F8D0.
- Callees (in order of first use): CCollision::ResetMadeInvisibleObjects 0x415540 (stub into .HOODLUM 0x156D280: makes the
  entities in the 0x9655A0 list visible again, clears the list), CCamera::UpdateTargetEntity 0x50C360, CWorld::FindPlayerPed,
  CEntity::RegisterReference, CCamera::CamControl 0x527FA0, ProcessVectorMoveLinear() 0x5164A0, ProcessVectorTrackLinear()
  0x516440, ProcessFOVLerp() 0x516500, ProcessFade 0x50B5D0, ProcessMusicFade 0x50B6D0, ProcessWideScreenOn 0x50B890,
  RwCameraSetNearClipPlane, CGeneral::GetATanOfXY, CCam::Process 0x526FC0, CCamera::ProcessShake(float) 0x516560,
  CCamera::StoreValuesDuringInterPol 0x50C290, CVector::Normalise/CrossProduct, VectorSub 0x40FE60, 0x40FEC0 / 0x40FE30
  (CVector scale / add helpers used for lerps), CCam::KeepTrackOfTheSpeed 0x509DF0, CCullZones::CamStairsForPlayer 0x72DD30,
  CWorld::ProcessLineOfSight, `rand`, `_ftol2`, CWorld::FindPlayerVehicle, CDraw::SetFOV 0x6FF410,
  CCamera::CalculateDerivedValues 0x5150E0, CCamera::CopyCameraMatrixToRWCam 0x50AFA0, CCamera::UpdateSoundDistances 0x515BD0,
  RwCameraSetFarClipPlane 0x7EE2A0, CWaterLevel::GetWaterLevel 0x6EB690.
- Globals: `CMBlur::Drunkness` 0xC73C58, drunk phase f32 0xB6EC30, bytes 0xB70142 / 0xB70143, `CCutsceneMgr::ms_running`
  0xB5F851, `ms_useLodMultiplier` 0xB5F853, `CDraw::ms_fFOV` 0x8D5038, `CRenderer::ms_lodDistScale` 0x8CD800,
  `CDraw::ms_fNearClipZ` 0xC3EFA0 / `ms_fFarClipZ` 0xC3EF9C (W), `CWeather::WaterDepth` 0xC81330 (W),
  `CWeather::UnderWaterness` 0xC8132C (W), `Scene.m_pRwCamera` 0xC1703C, `CTimer::m_snTimeInMilliseconds`.
  Constants: pi 3.1415927 [0x858CB8], 2pi [0x858CBC], -pi [0x858CC0], 0.3 [0x858C24], 1.3 [0x8CCF20, .data], -0.02 [0x85904C],
  0.05 [0x858C28], -0.1 [0x858EF4], 5.0 [0x858C80], 0.00028 [0x863280], 2.0 [0x858CA0], 0.1 [0x858B1C], -255.0 [0x86327C],
  0.2 [0x858CC4], 70.0 [0x858CE0], 100.0 [0x858628], 0.01 [0x858C58], 0.4 [0x858EE8], 0.6 [0x858CC8], 0.8333333 [0x863278],
  0.5 [0x858B8C], 1.0, 0.0, 2^32.
- Fields: cam = `m_aCams[m_nActiveCam]` (this + 0x174 + i*0x238; CCam +0x190 m_vecFront, +0x19C m_vecSource,
  +0x1A8 m_vecSourceBeforeLookBehind, +0x1B4 m_vecUp, +0xB4 m_fFOV, +0x28 m_nDirectionWasLooking, +0x148
  m_vecTargetCoorsForFudgeInter, +0xC m_nMode, +0x21C m_pCamTargetEntity); CCamera fields are named in each step.
- Behaviour:
  1. (0x52B73C) `ResetMadeInvisibleObjects()`; `fov = 0`; `m_bJust_Switched (+0x2A) = 0`;
     `m_vecRealPreviousCameraPosition (+0x848) = GetPosition()`.
  2. If `m_bLookingAtPlayer (+0x2B) == 1 || m_bTargetJustBeenOnTrain (+0x34) == 1 || m_nWhoIsInControlOfTheCamera (+0xB0) == 2`:
     `UpdateTargetEntity()`.
  3. If `m_pTargetEntity (+0x958) == null`: `= FindPlayerPed(-1)`, RegisterReference. Then for cam i = active and cam
     j = `(active + 1) & 0x80000001` (signed-mod-2 idiom; gives 0/1): if `m_aCams[k].m_pCamTargetEntity (+0x390+k*0x238)` is null,
     set it to m_pTargetEntity and RegisterReference.
  4. `CamControl()` (0x52B840); `ProcessVectorMoveLinear(); ProcessVectorTrackLinear(); ProcessFOVLerp();`
     `if (m_bFading (+0x51)) ProcessFade(); if (m_bMusicFading (+0x52)) ProcessMusicFade(); if (m_bWideScreenOn (+0x3D)) ProcessWideScreenOn();`
  5. `RwCameraSetNearClipPlane(Scene.camera, 0.3f)` (immediate 0x3E99999A, 0x52B899).
  6. `beta0 = (cam.front.x == 0 && cam.front.y == 0) ? 0 : GetATanOfXY(cam.front.x, cam.front.y)`; `cam.Process()` (0x52B90A,
     CCam::Process, re-reading the active index); `beta1` likewise after it.
  7. If `m_bTransitionState (+0x58) && m_nTimeTransitionStart (+0x78) + m_nTransitionDuration (+0x7C) < now` (unsigned):
     `m_bTransitionState = 0; m_bDoingSpecialInterp (+0x4F) = 0; m_bWaitForInterpolToFinish (+0x3A) = 0`.
  8. `if (m_bUseNearClipScript (+0x31)) RwCameraSetNearClipPlane(Scene.camera, m_fNearClipScript (+0x138))`.
  9. `d = beta1 - beta0`; `while (d >= pi) d -= 2pi; while (d < -pi) d += 2pi;` `if (fabs(d) > 0.3f) m_bJust_Switched = 1`.
  10. `vehicleTarget = (cam.dword[+0x28] (this+0x19C+i*0x238) != 3) && (m_pTargetEntity type == 2)` (0x52B9xx; the dword at
      cam+0x28 is gr `m_nDirectionWasLooking`; 3 = LOOKING_DIRECTION_FORWARD).
  11. Shake: `t = (float)(uint32)now; if (t <= m_fEndShakeTime (+0xC98)) ProcessShake((t - m_fStartShakeTime) / (m_fEndShakeTime - m_fStartShakeTime))` (0x52BA68).
  12. Camera vectors (src, front, up, fov):
      a. Not in transition, or vehicleTarget: `src = cam.source (+0x19C)`, `up = cam.up (+0x1B4)`; if `!m_bMoveCamToAvoidGeom (+0x2D)`:
         `front = cam.front`; else `src += m_vecClearGeometryVec (+0x8FC)`; `front = Normalise(cam.m_vecTargetCoorsForFudgeInter (cam+0x148) - src)`; then
         `r = Normalise(CrossProduct(front, up)); up = Normalise(CrossProduct(r, front))`. `byte 0xB70143 = 0`;
         `fov = cam.m_fFOV`.
      b. In transition (0x52BA84-0x52C3D4) [x87 checked by follow-up writer]:
         `dur = m_nTransitionDuration (+0x7C)`; `el = now - m_nTimeTransitionStart (+0x78)`, `if (el >= dur) el = dur` (unsigned `jb`);
         `fS = (float)el / (float)dur` (each `fild` + 2^32 fix-up when the sign bit is set; `fdivr`), stored f32 (L1C);
         `fT = (float)el / (float)m_nTransitionDurationTargetCoors (+0x80)` (same fix-up), kept in ST0, then
         `if (fT < 0) fT = 0` (`test ah,5; jp` keeps on >= / NaN) and `if (fT > 1) fT = 1` (`test ah,0x41; jne` keeps on <= / NaN).
         Target (C18 = m_fFractionInterToStopMovingTarget, C1C = ...CatchUpTarget):
         - `fT <= C18` (0x52BB16): `u = (C18 == 0) ? 0 : (C18 - fT) / C18` (fsubr, fdiv); `w = 0.5f - cos(u * pi) * 0.5f` (fmul pi,
           fcos, fmul 0.5, fsubr 0.5); `tgt = w * m_vecTargetSpeedAtStartInter (+0x8C0) + m_vecStartingTargetForInterPol (+0x89C)`
           (w*s.y and w*s.z stored f32 before the adds; w*s.x stays in ST0) and the result is also written to
           m_vecTargetWhenInterPol (+0x8E4).
         - `fT > C18` (0x52BBD7): `u = (C1C == 0) ? 1 : (fT - C18) / C1C`; `w` as above, stored f32; `if (C18 == 0) +0x8E4 =
           +0x89C`; `tgt = (cam.m_vecTargetCoorsForFudgeInter (cam+0x148) - +0x8E4) * w + +0x8E4` per component (the z difference
           and the x product are stored f32 first); +0x8E4 is NOT updated in this branch (tgt is local only).
         - fT NaN (only reachable through the compare at 0x52BBC6): ST0 popped, tgt left unwritten.
         Source/up/FOV (C10 = m_fFractionInterToStopMoving, C14 = ...CatchUp), compare `fS` vs C10 at 0x52BCDD:
         - `fS <= C10` (0x52BCEE): `u = (C10 == 0) ? 0 : (C10 - fS) / C10`; `w = 0.5f - cos(u*pi)*0.5f` (stored f32, `fst` L1C);
           `m_vecSourceWhenInterPol (+0x8D8) = w * m_vecSourceSpeedAtStartInter (+0x8B4) + m_vecStartingSourceForInterPol (+0x890)`
           (z product stored f32 first); if `m_bLookingAtPlayer != 0` (byte test, not `== 1`): `dx = SWI.x - tgt.x`, `dy = SWI.y -
           tgt.y` (stored f32), `if (sqrt(dx*dx + dy*dy) < 1.3f [.data 0x8CCF20]) { a = GetATanOfXY(dx, dy); SWI.x = cos(a)*1.3f +
           tgt.x; SWI.y = sin(a)*1.3f + tgt.y; }`; `m_vecUpWhenInterPol (+0x8F0) = w * m_vecUpSpeedAtStartInter (+0x8CC) +
           m_vecStartingUpForInterPol (+0x8A8)`; `m_fFOVWhenInterPol (+0x110) = w * m_fFOVSpeedAtStartInter (+0x114) +
           m_fStartingFOVForInterPol (+0x170)`; `src = SWI` (copy); `front = tgt - src` (each component stored f32);
           `StoreValuesDuringInterPol(&src, &m_vecTargetWhenInterPol, &m_vecUpWhenInterPol, &m_fFOVWhenInterPol)`.
         - `C10 < fS <= 1` (0x52C040): `u = (C14 == 0) ? 1 : (fS - C10) / C14`; `w` as above (stored f32);
           `src = SWI + (cam.m_vecSource - SWI) * w` (VectorSub, 0x40FEC0 scale, 0x40FE30 add; each step stored f32); same 1.3 push-out
           of `src` (when `m_bLookingAtPlayer != 0`; here `dy*dy` is formed before `dx*dx`); `fovL = (cam.m_fFOV - m_fFOVWhenInterPol) * w +
           m_fFOVWhenInterPol`; `up = UWI + (cam.m_vecUp - UWI) * w`; `front = tgt - src`;
           `StoreValuesDuringInterPol(&src, &tgt /*local*/, &up, &fovL)` (it copies them to +0x914/+0x920/+0x92C/+0xEC only).
         - `fS > 1` or NaN (e.g. dur == 0): jumps straight to the KeepTrackOfTheSpeed call (0x52C340) with src/front/up/fov
           unwritten this frame (uninitialised stack; open question).
         Both lerp branches then: `Normalise(front)`; `up = (m_bLookingAtPlayer == 1) ? (0, 0, 1) : (first branch: m_vecUpWhenInterPol;
         second branch: the lerped up)` - the (0,0,1) constants at 0x52BEEE / 0x52C228 are written to a scratch vector (L10) and copied
         into **up**, not front; first branch `Normalise(up)` here; if `cam.m_nMode == 1 || == 37`: `Normalise(front); up =
         Normalise(CrossProduct(front, (-1, 0, 0)))` (out = a x b, a = front, b = the fixed vector); else `Normalise(front);
         Normalise(up); r = Normalise(CrossProduct(front, up)); up = Normalise(CrossProduct(r, front))`.
         `fov = m_fFOVWhenInterPol` in both branches (0x52C00D / 0x52C336): in the second branch the lerped `fovL` only reaches
         StoreValuesDuringInterPol, so the frame FOV stays frozen at the end-of-first-phase value (original behaviour, keep).
         Finally (0x52C340) `d = src - tgt` (stored f32), `alpha = GetATanOfXY(sqrt(d.y*d.y + d.x*d.x), d.z)`,
         `beta = GetATanOfXY(d.x, d.y)`, `cam.KeepTrackOfTheSpeed(&src, &tgt, &up, &alpha, &beta, &fov)`.
  13. (0x52C52B-0x52C5E2) If `m_bTransitionState && !m_bLookingAtVector (+0x2C) && m_bLookingAtPlayer == 1 &&
      !CamStairsForPlayer() && !m_bPlayerIsInGarage (+0x26)`: `ent = 0`; if `ProcessLineOfSight(targetPos, src, cp, ent,
      true, false, false, true, false, true, true, false)`: `src = cp.point; RwCameraSetNearClipPlane(Scene.camera, 0.05f)`.
  14. Drunk (0x52C5EA-0x52C72F) if `CMBlur::Drunkness > 0`: `c = cos(phase * 0.017453292)`, `s = sin(phase * 0.017453292)`
      (phase = f32 0xB6EC30); `src.x += c * Drunkness * -0.02f; src.z += s * Drunkness * -0.02f` (as `c*D*k + src`);
      normalise; `up.x += c*D*0.05f; up.y += s*D*0.05f`; normalise; `front.x += c*D*(-0.1f); front.y += s*D*(-0.1f)`; normalise;
      `r = Normalise(CrossProduct(front, up)); up = Normalise(CrossProduct(r, front))`; `phase += 5.0f`.
  15. Matrix (0x52C747): `right = CrossProduct(up, front)`; `m_mCameraMatrix`: right row +0x974..+0x97C = right, up row
      +0x984.. = front, at row +0x994.. = up, pos +0x9A4.. = src (stores in the order listed by the decompile).
  16. Shake (0x52C7F4-0x52C8E5) [x87 checked]: `el = (float)(uint32)(now - m_nCamShakeStart (+0x5C))`;
      `f = m_fCamShakeForce (+0x12C) - el * 0.00028f`; `if (!(f < 2.0f)) f = 2.0f; else if (f <= 0.0f) f = 0.0f;`
      `r = rand() & 0xFFFF; k = f * 0.1f`; `pos.x = (float)((r & 0xF) - 7) * k + pos.x`, `pos.y = (float)(((r >> 4) & 0xF) - 7) * k + pos.y`,
      `pos.z = (float)(((r >> 8) & 0xF) - 7) * k + pos.z` (fild of the int); if `k > 0 && m_nBlurType (+0x90) != 1`:
      `m_nMotionBlurAddAlpha (+0xA4) = min(25 - _ftol2(f * -255.0f), 150)` (signed).
  17. (0x52C8EB-0x52C96A) If cam mode == 16 (1STPERSON) and a player vehicle exists and its `m_matrix->at.z (+0x28) < 0.2f`:
      `m_nBlurRed = m_nBlurGreen = m_nBlurBlue = 255 (+0x8C/+0x88/+0x84); m_nBlurType = 1; m_nMotionBlur (+0xA0) = 240;
      byte 0xB70142 = 1`. Else if byte 0xB70142: clear it.
  18. `CDraw::SetFOV(fov)`; `CalculateDerivedValues(false, true)`; `CopyCameraMatrixToRWCam(false)`;
      `m_vecGameCamPos (+0x908) = matrix pos`; `UpdateSoundDistances()`.
  19. LOD [x87 checked]: `m_fLODDistMultiplier (+0xF0) = (ms_running && !ms_useLodMultiplier) ? 1.0f : 70.0f / CDraw::ms_fFOV`;
      `m_fGenerationDistMultiplier (+0xF4) = m_fLODDistMultiplier`; `m_fLODDistMultiplier = ms_lodDistScale * m_fLODDistMultiplier`.
  20. Far clip [x87 checked]: `RwCameraSetFarClipPlane(cam, (float)_ftol2(cam->farPlane (+0x84) * 100.0f) * 0.01f)`;
      `CDraw::ms_fNearClipZ = m_pRwCamera->nearPlane (+0x80)`; `ms_fFarClipZ = m_pRwCamera->farPlane (+0x84)`.
  21. Speed tracking: if `m_bJustInitialized (+0x29) == 1 || m_bJust_Switched == 1`: `m_vecPreviousCameraPosition (+0x83C) = pos;
      m_bJustInitialized = 0`. `m_nNumFramesSoFar (+0x98) += 1`; `m_fCameraSpeedSoFar (+0x128) += |pos - prev|`;
      if `(float)m_nNumFramesSoFar == (float)m_nWorkOutSpeedThisNumFrames (+0x94)`: `m_fCameraAverageSpeed (+0x124) =
      speedSoFar / (float)(int)N; speedSoFar = 0; numFrames = 0`. `prev = pos`.
  22. Look-behind restore: `o = m_fOrientation (+0x150) + pi`; if active cam `dword[+0x28] != 3` and mode != 37:
      `cam.source = cam.sourceBeforeLookBehind (+0x1A8); m_fOrientation = o`. If `m_bTransitionState`: for the other cam j
      (as in step 3), if its target entity exists, `m_pTargetEntity` is a ped, the other cam's target is not a vehicle, mode
      != 37 and its `dword[+0x28] != 3`: `camJ.source = m_aCams[active & 0x80000001].sourceBeforeLookBehind; m_fOrientation = o`.
  23. `m_bCameraJustRestored (+0x1C) = 0; m_bMoveCamToAvoidGeom (+0x2D) = 0`.
  24. Under-water (0x52CC80-0x52CD73): probe point `q = camPos + front*0.4f` with front = `m_matrix ? m_matrix->up : (-sin(heading),
      cos(heading), 0)` (heading +0x10); `if (GetWaterLevel(q.x, q.y, q.z, &lvl, true, nullptr) && q.z - 0.6f <= lvl)`:
      `WaterDepth = max(lvl - q.z, 0)`; `UnderWaterness = (lvl > q.z + 0.6f) ? 1.0f : 1.0f - (q.z - (lvl - 0.6f)) * 0.8333333f`;
      else `UnderWaterness = 0`.
- gta-reversed: hooked `.Reversed = false`, `plugin::CallMethod` (gr also lists `ProcessScriptedCommands` 0x516AE0 as
  inlined at 0x52B845, consistent with step 4).
- Open questions (confidence medium): (1) resolved: the (0,0,1) at 0x52BEE8-0x52BF2D / 0x52C222-0x52C254 is the up vector.
  (2) 0x52C016 / 0x52C03A: with fS NaN (dur == 0) or fS > 1 the transition path reaches KeepTrackOfTheSpeed / SetFOV with
  src, front, up and fov not written this frame (stack garbage); also 0x52BBD1 for fT NaN leaves tgt unwritten. (3) 12b x87
  order is now checked; steps outside 12b marked without "[x87 checked]" still follow the decompile order. (4) step 10's
  vehicleTarget only gates 12a (0x52BA7C).

## 0x5B24D0 CCamera::LoadPathSplines  (catalogue: specified/REA, port level 2)
- Convention: `__thiscall void(FILESTREAM file [esp+4])`, `ret 4`.
- Caller: CCutsceneMgr::LoadCutsceneData_postload 0x5AFBC0.
- Callees: CCamera::DeleteCutSceneCamDataMemory 0x5B24A0, `CFileLoader::LoadLine(FILESTREAM)` 0x536F80 (cdecl, returns
  char* or null), `sscanf` 0x8220AD with "%d" (0x859508), `operator new` 0x82119A, `strtok` 0x82244B (delimiters ", \t",
  0x868B10), `atof` 0x823CEE (double result stored with `fstp dword` = rounded to f32).
- Fields: `m_aPathArray[4]` (+0x960, float* each; CCamPathSplines).
- Behaviour:
  1. `DeleteCutSceneCamDataMemory()`. `left = 0; idx = -1; wantHeader = true; dst = (float*)file` (garbage until the first
     allocation; never used before it).
  2. For each `line = LoadLine(file)` until null:
     - if `line[0] == '#'` or `line[0] == 0`: skip.
     - else if `left == 0`:
       - if `wantHeader`: `idx++`; if `idx > 3` -> return (0x5B2536, the remaining lines are not read).
         `sscanf(line, "%d", &left)` (result ignored; on failure left stays 0). Allocation size:
         `idx == 0 || idx == 1` -> `left*16 + 4`, else `left*40 + 4` (bytes; 32-bit arithmetic).
         `p = operator new(size)`; `m_aPathArray[idx] = p`; `p[0] = (float)left` (fild); `dst = p + 1`; `wantHeader = false`.
       - else if `line[0] == ';'`: `wantHeader = true`. Other lines are ignored.
     - else (`left != 0`): `left--`; `for (tok = strtok(line, ", \t"); tok; tok = strtok(nullptr, ", \t")) *dst++ = (float)atof(tok);`
  Tracks 0 and 1 hold 4 floats per node (time, value, in, out), tracks 2 and 3 hold 10 floats per node (time, xyz, in xyz,
  out xyz); see 0x5B2330 / 0x5B2090 in `_unattributed-B.md`.
- Oddities (keep): no bounds check on the number of tokens per line (heap overflow if a line has more values than the
  node stride); the count line is parsed with sscanf and an unparsable count gives a 4-byte array with count 0.
- gta-reversed: hooked `.Reversed = false`; the gr stub even drops the `file` argument (`plugin::CallMethod<0x5B24D0, CCamera*>(this)`),
  i.e. gr calls the original with an unset stack argument - a gr bug, irrelevant for the port.
- Open questions: none.


## 0x50A870 CCamera::~CCamera  (catalogue: specified/REA, port level 2)
- `__thiscall` (ECX = this → ESI), plain `ret`; SEH frame with unwind states 8..0. Callers: CCamera::`scalar deleting destructor' 0x514010,
  __dtor_TheCamera 0x856410 (tail call).
- Behaviour (exact order; destructors run in reverse member order):
  1. `this->vtable = 0x8630E8` (CCamera vtable); `this+0x14 = 0` (the CPlaceable matrix pointer, 0x50A8AB).
  2. `eh vector destructor iterator` 0x821E02 (stdcall: arr, elemSize, count, dtor; descending) on `this+0xCF4`, 0x10, 5, dtor 0x509800.
  3. `CMatrix::~CMatrix` 0x59ACD0 (thiscall) on this+0xADC, +0xA94, +0xA4C, +0xA04, +0x9BC, +0x974, in that order.
  4. 0x821E02 on `this+0x960`, 4, 4, dtor 0x5097C0; then on `this+0x174` (Cams[3]), 0x238, 3, dtor 0x5097D0 (CCam dtor).
  5. Base destructor 0x54F490 (CPlaceable dtor, uncatalogued name: sets vtable 0x863C40; because +0x14 was zeroed in step 1 its
     `CPlaceable::RemoveMatrix` 0x54F3B0 / `[0xB74238]--` branch is skipped; then `+0x14 = 0xB74240`).
- gta-reversed: catalogue REA (named by CAT-17 correction, high).
