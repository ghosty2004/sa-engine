# CCam (CAM) - full specs (group A)

Layout (gr `Cam.h`, size 0x238; offsets computed from the header and checked against the bytes): +0x02 m_bCamLookingAtVector,
+0x03 m_bCollisionChecksOn, +0x07 m_bLookingBehind, +0x08 m_bLookingLeft, +0x09 m_bLookingRight, +0x0A m_bResetStatics,
+0x0B m_bRotating, +0x0C m_nMode (i16), +0x10 m_nFinishTime, +0x14 m_nDoCollisionChecksOnFrameNum, +0x18 m_nDoCollisionCheckEveryNumOfFrames,
+0x1C m_nFrameNumWereAt, +0x28 m_nDirectionWasLooking, +0x38 m_fSyphonModeTargetZOffSet, +0x70 m_fDistanceBeforeChanges, +0x90 m_fTimeElapsedFloat,
+0x9C m_fTransitionBeta, +0xA0 m_fTrueBeta, +0xA4 m_fTrueAlpha, +0xA8 m_fInitialPlayerOrientation, +0xAC alpha (m_fVerticalAngle),
+0xB0 m_fAlphaSpeed, +0xB4 m_fFOV, +0xB8 m_fFOVSpeed, +0xBC beta (m_fHorizontalAngle), +0xC0 m_fBetaSpeed, +0xC4 m_fDistance,
+0xCC m_fCaMinDistance, +0xD0 m_fCaMaxDistance, +0xD4 m_fSpeedVar, +0xE8..+0xF4 target zoom Z extras, +0x108 m_fX_Targetting,
+0x10C m_fY_Targetting, +0x110/+0x114 (see Process_Cam_TwoPlayer_Separate_Cars), +0x118/+0x11C cam bump, +0x120 m_nCamBumpedTime,
+0x148 m_vecTargetCoorsForFudgeInter ("fudge"), +0x154 m_vecCamFixedModeVector, +0x160 m_vecCamFixedModeSource, +0x16C m_vecCamFixedModeUpOffSet,
+0x184 m_vecBufferedPlayerBodyOffset, +0x190 m_vecFront, +0x19C m_vecSource, +0x1A8 m_vecSourceBeforeLookBehind, +0x1B4 m_vecUp,
+0x1D8 m_avecTargetHistoryPos[4] ("hist"), +0x208 m_anTargetHistoryTime[4], +0x218 m_nCurrentHistoryPoints, +0x21C m_pCamTargetEntity.
TheCamera = 0xB6F028 (CCamera, size 0xD78). Common notation: `pos(e)` = `e->m_matrix ? &m_matrix->pos (+0x30) : &e->placement.pos (+4)`;
`M(e)` = "if `e->m_matrix` is null: `e->AllocateMatrix(); e->placement.UpdateMatrix(e->m_matrix)`" then the matrix (right +0, fwd +0x10, up +0x20,
pos +0x30); `ts` = CTimer::ms_fTimeStep 0xB7CB5C; `now` = CTimer::m_snTimeInMilliseconds 0xB7CB84; PI 0x858CB8, 2PI 0x858CBC, -PI 0x858CC0,
PI/2 0x858FE4, 0.0f 0x858B50, 1.0f 0x858624, 0.5f 0x858B8C, -1.0f 0x858C1C, 0.012500000f 0x85A5C4, 0.017453292f 0x8595EC.
x87 compare idioms: `fcomp X; test ah,0x41; jne` skips when st0 <= X or unordered; `test ah,5; jp` skips when st0 >= X or unordered;
`test ah,0x44; jp` skips when not equal or unordered. All `fdivp st(1)` / `fsubp st(1)` are encoded DE F9 / DE E9: st1 = st1 op st0.

## 0x509C50 CCam::ClipBeta  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall void (CCam* this = ECX)`, plain `ret`. Leaf, no stack frame; EAX clobbered (fnstsw).
- Callers (6): Process_1rstPersonPedOnPC 0x50EB70, Process_FollowPedWithMouse 0x50F970, Process_M16_1stPerson 0x5105C0,
  Process_Rocket 0x511B50, Process_AimWeapon 0x521500, Process_FollowPed_SA 0x522D40. Callees: none.
- Fields: +0xBC `m_fHorizontalAngle` (beta) f32, read and written.
- Constants: 0x858CB8 = 3.1415927f, 0x858CBC = 6.2831855f, 0x858CC0 = -3.1415927f.
- Behaviour:
  1. `if (beta > 3.1415927f)` (fcomp at 0x509C56, `test ah,0x41; jne` - not taken only when strictly greater and
     ordered): `beta = beta - 6.2831855f` (0x509C69), return.
  2. else `if (beta < -3.1415927f)` (0x509C76, `test ah,5; jp` - taken for >=, equal and NaN): `beta = beta + 6.2831855f`
     (0x509C89).
  Only one wrap step is applied (no loop); values outside [-3pi, 3pi] stay out of range. NaN is left unchanged.
- gta-reversed: C++ at Cam.cpp:509 (no hook installed for 0x509C50). gr LOGIC ERROR: gr does `if (beta < -180deg) beta += 360deg; else beta -= 360deg;`, i.e. it subtracts 2pi from every
  value >= -pi, including in-range values. The original only subtracts when beta > pi. Port the bytes.

## 0x509DC0 CCam::GetWeaponFirstPersonOn  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall bool (CCam* this = ECX)`, returns AL, plain `ret`. Only AL is defined: on the
  true path the upper 24 bits of EAX are those of the entity pointer (0x509DDF `mov al, ...`), on the false path
  `xor al, al` keeps the upper bits of EAX (entity pointer or 0). Callers must only use AL.
- Callers: CEventHandler::ComputeDeadPedResponse 0x4B9470, CCamera::CamControl 0x527FA0 (2 sites). Callees: none.
- Fields: this+0x21C `m_pCamTargetEntity` (CEntity*). Entity +0x36 type byte (`& 7` = entity type, 3 = ped).
  CPed +0x718 `m_nActiveWeaponSlot` (read with `movsx`, i.e. signed i8), CPed +0x5A0 `m_aWeapons[13]` (CWeapon, stride
  0x1C); CWeapon +0x14 `m_IsFirstPersonWeaponModeSelected` (bool).
- Behaviour: `e = this->m_pCamTargetEntity; if (e && (e->type & 7) == 3) return ped->m_aWeapons[(int8)slot].m_IsFirstPersonWeaponModeSelected
  /* byte at e + slot*0x1C + 0x5B4 */; return false;`. The slot index is sign-extended (0x509DD5), so a slot byte >= 0x80
  would index backwards (never happens in practice).
- gta-reversed: Cam.cpp:495, equivalent (gr uses `uint8` slot; only differs for slot >= 0x80).

## 0x50EB70 CCam::Process_1rstPersonPedOnPC  (catalogue: specified/REA, gr-unreversed, port level 7)
- Convention / signature: `__thiscall void (CCam* this = ECX, const CVector& target, float orientation, float speedVar,
  float speedVarWanted)`, `ret 0x10`. Only `orientation` is read (0x50EC3F); its stack slot is later re-used as scratch
  (`h`, `v`, `a` below). Frame `sub esp,0x50` + ebx(=0 throughout), esi(=this), then ebp, edi. B = ESP after all four
  pushes: `pin` (bone point) B+0x24..0x2C, `fw` B+0x18..0x20.
- Callers: CCam::Process 0x526FC0.
- Callees: RwHelper::GetAnimHierarchyFromSkinClump 0x734A40, ConvertPedNode2BoneTag 0x4D58A0, RpHAnimIDGetIndex 0x7C51A0,
  RpHAnimHierarchyGetMatrixArray 0x7C5120, RwV3dTransformPoints 0x7EDD90, RwMatrixScale 0x7F22C0, CPlaceable::AllocateMatrix
  0x54F560, CSimpleTransform::UpdateMatrix 0x54F1B0, CMatrix::operator*(CVector) 0x59C890 (cdecl(out, mat, vec)),
  CVector::Normalise 0x59C910, CPed::GetBonePosition 0x5E4280, CPad::GetPad 0x53FB70, unnamed CPad methods 0x540E80 and
  0x540F80 (thiscall, no stack args, -> i16 in AX), CCam::ClipBeta 0x509C50, CPed::IsPlayer 0x5DF8F0, CPlaceable::GetHeading
  0x441DB0, CCam::GetVectorsReadyForRW 0x509CE0, CMatrix::SetRotateZOnly 0x59B020, CMatrix::UpdateRwMatrix 0x59AD70,
  CSimpleTransform::UpdateRwMatrix 0x54EF40, CPad::SniperZoomOut 0x540B80 / SniperZoomIn 0x540B30 (AL),
  RwCameraSetNearClipPlane 0x7EE1D0.
- Globals: guard byte 0xB6FFE0 (bit 0 set on first call; nothing else is initialised under it), v3d_8CCC54 (CVector
  {0.06, 0.05, 0.0}, .data), `sStartPin` CVector 0xB6FFC4, CVector 0xB6FFD0 (zeroed on reset), byte 0xB6FFDC (cleared
  on reset), TheCamera.m_b1rstPersonRunCloseToAWall 0xB6F066 (W 0), m_bHeadBob 0xB6F067, m_fGaitSwayBuffer 0xB6FC48,
  m_fAlphaForPlayerAnim1rstPerson 0xB6F174 (W), m_pTargetEntity 0xB6F980, blur fields 0xB6F0AC/B0/B4/B8, m_nMotionBlur
  0xB6F0C8, NewMouseControllerState.m_AmountMoved x 0xB73424 / y 0xB73428, CCamera::m_fMouseAccelHorzntl 0xB6EC1C,
  m_fMouseAccelVertical 0xB6EC18, CTimer::ms_fTimeStep 0xB7CB5C, Scene.m_pRwCamera 0xC1703C.
- Fields: this +0x03, +0x0A m_bResetStatics, +0x0C m_nMode (i16), +0xA8 m_fInitialPlayerOrientation, +0xAC alpha,
  +0xB4 FOV, +0xBC beta, +0x184 m_vecBufferedPlayerBodyOffset, +0x190 front, +0x19C source, +0x21C target entity.
  Ped: +0x18 m_pRwObject, +0x36 type, +0xFC m_pAttachedTo, +0x558 m_fCurrentRotation, +0x55C m_fAimingRotation,
  +0x778 u16 turret position (gr `m_fTurretAngleA`, a u16), +0x77C f32 turret angle limit (gr `m_fTurretAngleB`).
- Behaviour:
  1. Static guard (no effect). If `m_nMode != 39` (MODE_SNIPER_RUNABOUT): `FOV = 70.0f`.
     `TheCamera.m_b1rstPersonRunCloseToAWall = 0`.
  2. `e = m_pCamTargetEntity`; if `!e->m_pRwObject`: return (m_bResetStatics untouched, no near-clip call).
     If `(e->type & 7) != 3`: go to step 14.
  3. `pin = v3d_8CCC54`; `hier = GetAnimHierarchyFromSkinClump(e->m_pRwObject)`; `idx = RpHAnimIDGetIndex(hier,
     ConvertPedNode2BoneTag(2))`; `M = RpHAnimHierarchyGetMatrixArray(hier) + idx` (64-byte RwMatrix);
     `RwV3dTransformPoints(&pin, &pin, 1, M)`; `RwMatrixScale(M, &(0,0,0), rwCOMBINEPRECONCAT)` (zeroes the bone matrix).
  4. If `m_bResetStatics`: `beta = orientation; alpha = 0; m_fInitialPlayerOrientation = orientation`; (the entity is a
     ped, so always:) `alpha = 0; beta = ped->m_fCurrentRotation + PI/2; byte 0xB6FFDC = 0; [0xB6FFD8] = [0xB6FFD4] = 0;
     m_bCollisionChecksOn = 1; m_fInitialPlayerOrientation = ped->m_fCurrentRotation + PI/2` (re-loaded);
     `m_vecBufferedPlayerBodyOffset = pin; [0xB6FFD0] = 0; sStartPin = pin`.
  5. `bufOff.y = pin.y`.
  6. If `TheCamera.m_bHeadBob`: `g = m_fGaitSwayBuffer`; `bufOff.x = (1.0f - g)*pin.x + g*bufOff.x`; `bufOff.z = (1.0f - g)*pin.z
     + g*bufOff.z`; ensure matrix (AllocateMatrix + UpdateMatrix if null); `pin = (*e->m_matrix) * bufOff` (full transform).
     Else: `dx = pin.x - sStartPin.x; dy = pin.y - sStartPin.y`; ensure matrix; `fw = (fwd.x, fwd.y, 0)` (matrix +0x10),
     normalise; `mag = sqrt(dx*dx + dy*dy)`; `pin.x = ((fw.x*mag) * 1.23f [0x863108]) + pos.x; pin.y = ((fw.y*mag)*1.23f) + pos.y;
     pin.z = (((fw.z*mag)*1.23f) + pos.z) + 0.59f [0x863104]` (pos = matrix ? &mat.pos : &placement.pos).
     `source = pin`.
  7. `ped->GetBonePosition(&tmp, 3, true)` (result unused). `GetPad(0)` (result unused).
  8. Look input: `mx = m_AmountMoved.x, my = m_AmountMoved.y`.
     - If `mx != 0 || my != 0` (mouse): `f = FOV * 0.0125f [0x85A5C4]`; `dB = (mx * -3.0f [0x859024]) * (f * m_fMouseAccelHorzntl)`;
       `dA = (f * m_fMouseAccelVertical) * (my * 4.0f [0x858B90])`.
     - Else (pad): `h = (float)(-(int)GetPad(0)->sub_540E80())`, `v = (float)(int)GetPad(0)->sub_540F80()`; `sH = (h < 0) ? -1.0f : 1.0f`,
       `sV = (v < 0) ? -1.0f : 1.0f`; `f = FOV * 0.0125f`; `dB = (((h*h) * 0.0001f [0x858FC4]) * (f * 0.057142857f [0x863100])) * sH * ts`;
       `dA = (((v*v) * 4.4444445e-05f [0x8630FC]) * (f * 0.071428575f [0x859B50])) * sV * ts`.
     `beta = dB + beta; alpha = dA + alpha; ClipBeta()`.
  9. Alpha clamp: `if (alpha > 1.0471976f [0x8630F8]) alpha = 1.0471976f; else if (alpha < -1.5620698f [0x8630F4]) alpha = -1.5620698f`.
  10. Turret (0x50F06F): if `ped->IsPlayer() && ped->m_pAttachedTo`: `H` by u16 `ped+0x778` (jump table 0x50F3DC):
      0 -> `GetHeading(att) + PI/2`, 1 -> `GetHeading(att) + PI`, 2 -> `GetHeading(att) - PI/2`, 3 -> `GetHeading(att)`,
      >3 -> the float currently in the orientation slot (B+0x68: `v` from the pad path or `my*4` from the mouse path;
      original bug, keep). `d = beta - H`; `if (d > PI) d -= 2PI; else if (d < -PI) d += 2PI`; `lim = ped+0x77C`;
      `if (d > lim) d = lim; else if (d < -lim) d = -lim`; `beta = d + H`.
  11. Front/source (0x50F15F): `ca = cos(alpha)`; `p = (cos(beta)*ca*3.0f, sin(beta)*ca*3.0f, sin(alpha)*3.0f)` (each product
      times 3.0f [0x858B3C]); `q = p + source` (x: `p.x + src.x` etc.); `front = q - source`; `front.Normalise()`;
      `source.c = front.c*0.4f [0x858EE8] + source.c`.
  12. `TheCamera.m_fAlphaForPlayerAnim1rstPerson = alpha`; `GetVectorsReadyForRW()`.
  13. `h = atan2(-front.x, front.y)`; `t = TheCamera.m_pTargetEntity` (re-read each time): `t->m_fCurrentRotation = h;
      t->m_fAimingRotation = h`; `t->m_matrix ? t->m_matrix->SetRotateZOnly(h) : (t->placement.heading = h)`; if
      `t->m_pRwObject`: update the frame's RwMatrix (`rwobj->parent + 0x10`) via `m_matrix->UpdateRwMatrix` or
      `placement.UpdateRwMatrix`. If `m_nMode == 39`: if `SniperZoomOut() || SniperZoomIn()` (GetPad(0) each call):
      `k = ts*255.0f [0x859AAC] + 10000.0f [0x859AA4]`; ZoomOut: `FOV = (k * FOV) * 0.0001f`; else ZoomIn:
      `FOV = FOV / (k * 0.0001f)`. Then always (in mode 39): `m_nBlurRed = 180, m_nBlurBlue = 180, m_nBlurGreen = 255,
      m_nBlurType = 1, m_nMotionBlur = 120`; `if (FOV > 70.0f) FOV = 70.0f; else if (FOV < 15.0f) FOV = 15.0f`.
  14. `m_bResetStatics = 0; RwCameraSetNearClipPlane(Scene.m_pRwCamera, 0.05f)`.
- gta-reversed: unhooked (`.Reversed = false`) partial C++ at Cam.cpp:533-640. Differences from the bytes (LOGIC ERRORS
  in gr, port the bytes): (a) gr calls `CPad::LookAroundLeftRight/UpDown` (0x540BD0/0x540CC0) - the original calls the
  different functions 0x540E80/0x540F80; (b) gr's head-bob blend uses `m_fScriptPercentageInterToCatchUp` (0xB6FC50) - the
  original uses `m_fGaitSwayBuffer` (0xB6FC48); (c) gr's ClipAlpha clamps to [-85.5deg, 60deg]; the original clamps to
  [-1.5620698 (= -89.5deg), 1.0471976]; (d) gr divides by 10000/22500/17.5/14 where the original multiplies by f32 reciprocals
  (rounding), and computes the mouse terms in a different order; (e) gr's turret code subtracts `heading + X` from beta
  instead of the wrap/clamp in step 10, has no default case and omits steps 11-14 ("// ...").
- Open questions: none.

## 0x50F970 CCam::Process_FollowPedWithMouse  (catalogue: specified/REA, gr-unreversed, port level 8)
- Convention / signature: `__thiscall void (CCam* this = ECX, const CVector& target [B+0xC8], float orientation [B+0xCC],
  float speedVar, float speedVarWanted)`, `ret 0x10`. Frame `sub esp,0xB4` + ebx, ebp, esi, edi(=this); B = ESP after the
  pushes. Locals: `isTrain` u8 B+0x1B, `T` B+0x28..0x30, `dA` B+0x10, `dB` B+0x14, colpoint B+0x68 (point at +0), hit
  entity B+0x40, `inv` B+0x1C.
- Callers: CCam::Process 0x526FC0.
- Callees: CPad::GetPad 0x53FB70, CPad::ClearMouseHistory 0x541BD0, CWorld::FindPlayerVehicle 0x56E0D0 (cdecl(int=-1, bool=0)),
  CVector::Normalise 0x59C910, unnamed CPad 0x540E80 / 0x540F80 (thiscall, -> i16), CCam::ClipBeta 0x509C50, CWorld::ProcessLineOfSight
  0x56BA00 (cdecl(origin*, target*, CColPoint*, CEntity**, 8 bools)), CVector::Magnitude 0x4082C0 (ST0), RwCameraSetNearClipPlane
  0x7EE1D0, CWorld::TestSphereAgainstWorld 0x569E20 (cdecl(CVector center by value, float r, CEntity* ignore, bool buildings,
  vehicles, peds, objects, dummies, camIgnore) -> CEntity*), _CIpow 0x822130, CCam::GetVectorsReadyForRW 0x509CE0,
  CMatrix::SetRotateZOnly 0x59B020, CMatrix::UpdateRwMatrix 0x59AD70 / CSimpleTransform::UpdateRwMatrix 0x54EF40.
- Globals: NewMouseControllerState m_AmountMoved 0xB73424/28, CCamera::m_fMouseAccelHorzntl 0xB6EC1C / Vertical 0xB6EC18,
  TheCamera.m_bFading 0xB6F079, m_nFadeInOutFlag i16 0xB6FC58, CDraw::FadeValue u8 0xC3EFAB, u16 0x8CC7D4 (= 45), TheCamera
  m_bUseTransitionBeta 0xB6F05F, m_bCamDirectlyBehind 0xB6F042, m_bCamDirectlyInFront 0xB6F043, m_fPedOrientForBehindOrInFront
  0xB6F148, m_pTargetEntity 0xB6F980, CWorld::pIgnoreEntity 0xB7CD68, Scene.m_pRwCamera 0xC1703C (near plane f32 at +0x80),
  CDraw::ms_fAspectRatio 0xC3EFA4, gaTempSphereColPoints 0xB9B250 (point of entry 0), byte 0xBA6793 (read, unused).
  Data constants: 0x8CC7D0 0.8f, 0x8CC7D8 -0.22f, 0x8CC7C0 1.7f, 0x8CC7C4 2.0f, 0x8CC7C8 3.0f, 0x8CC7CC 0.01f, 0x8CC38C 0.5f.
- Behaviour:
  1. `FOV = 70.0f; isTrain = 0`; if the target entity is not a ped: return.
  2. If `m_bResetStatics`: `m_bRotating = 0; m_bCollisionChecksOn = 1; GetPad(0)->ClearMouseHistory(); m_bResetStatics = 0`.
  3. If `FindPlayerVehicle(-1, false)` and its base type (+0x590) == 6 (train): `isTrain = 1`.
  4. `T = *target; T.z = 0.8f + T.z`.
  5. Input: if `GetPad(0)->DisablePlayerControls` low byte `& 0x20` (bPlayerSafe): `v = source - T`, normalise; `d = (v.z < -0.9f
     [0x858CAC]) ? (orientation + PI) : atan2(v.y, v.x)`; `dB = d - beta; dA = 0`.
     Else if `(mx != 0 || my != 0) && GetPad(0)->DisablePlayerControls == 0` (u16): `a = mx * -2.5f [0x8631B0]; b = my * 4.0f;
     f = FOV * 0.0125f; dB = (m_fMouseAccelHorzntl * f) * a; dA = (f * b) * m_fMouseAccelVertical`.
     Else: `h = (float)(-(int)GetPad(0)->sub_540E80()); v = (float)(int)GetPad(0)->sub_540F80(); f = FOV*0.0125f;
     dB = (((0.071428575f * f) * ts) * 0.01f) * h; dA = (((f * 0.042857144f) * ts) * v) * 0.01f`.
  6. Auto pitch override: if `(m_bFading && m_nFadeInOutFlag == 1 && FadeValue > 45) || FadeValue > 200 ||
     (GetPad(0)->DisablePlayerControls & 0x20)`: with `P = -0.22f`: `dA = (P - 0.05f > alpha) ? 0.05f : (alpha < P) ? P - alpha :
     (P + 0.05f < alpha) ? -0.05f : (alpha > P) ? P - alpha : 0.0f` (each a separate x87 compare, NaN falls to the last
     alternative except as encoded by the jp/jne tests at 0x50FBF5-0x50FC79).
  7. `beta = dB + beta; alpha = dA + alpha; ClipBeta()`; `if (alpha > 0.7853982f [0x859AB0]) alpha = 0.7853982f; else if
     (alpha < -1.5620698f) alpha = -1.5620698f`.
  8. `dist = (alpha > 0 ? cos((PI/2 < 3.0f*alpha) ? PI/2 : 3.0f*alpha) : cos(alpha)) * 2.0f + 1.7f`.
  9. Beta overrides in order: `m_bUseTransitionBeta == 1` -> `beta = m_fTransitionBeta (+0x9C)`; `m_bCamDirectlyBehind == 1` ->
     `beta = m_fPedOrientForBehindOrInFront + PI`; `m_bCamDirectlyInFront == 1` -> `beta = m_fPedOrientForBehindOrInFront`;
     isTrain -> `beta = orientation`.
  10. `ca = cos(alpha); front = (-(cos(beta)*ca), -(sin(beta)*ca), sin(alpha))`; `source = T - front*dist` (`T.c - dist*front.c`);
      `m_vecTargetCoorsForFudgeInter = T`; `hitEnt = 0`.
  11. Line of sight (0x50FE67): `pIgnoreEntity = target`; if `ProcessLineOfSight(&T, &source, &cp, &hitEnt, true, true, true,
      true, false, false, true, false)`: `L = sqrt((dz*dz + dy*dy) + dx*dx)` with `d = T - cp.point`; `dist2 = dist - L`.
      - If hitEnt is a ped and `dist2 > 0.4f`: second test `ProcessLineOfSight(&cp.point, &source, &cp, &hitEnt, same flags)`
        (origin and output are the same CColPoint buffer). If it hits: `m = Magnitude(T - cp.point)`; `source = cp.point`; if
        `m < 0.6f`: `n = m - 0.3f; RwCameraSetNearClipPlane(cam, (0.05f > n) ? 0.05f : n)`. If it misses: `n = dist2 - 0.35f`;
        `RwCameraSetNearClipPlane(cam, (0.9f < n) ? 0.9f : n)`.
      - Else: `source = cp.point`; if `L < 0.6f`: `n = L - 0.3f; RwCameraSetNearClipPlane(cam, (0.05f > n) ? 0.05f : n)`.
      `pIgnoreEntity = nullptr`.
  12. Near-plane sphere test (0x510086): `t = tan((FOV * 0.017453292f) * 0.5f)` (fptan); `w = (aspect * t) * 1.1f [0x862F10]`;
      `nc = rwcam->nearPlane`; `c = front*nc + source` (`nc*front.c + src.c`); `r = nc * w`; `n = 0`;
      if `TestSphereAgainstWorld(c, r, nullptr, true, true, false, true, false, false)`: `inv = 1.0f / w`; loop:
      - `d = gaTempSphereColPoints[0].point - source`; `pr = (d.z*front.z + d.y*front.y) + d.x*front.x`;
        `e = (d.x - pr*front.x, d.y - pr*front.y, d.z - pr*front.z)`; `s = sqrt((e.z*e.z + e.y*e.y) + e.x*e.x) * inv`;
        `m = (nc < s) ? nc : s; nn = (0.1f > m) ? 0.1f : ((nc < s) ? nc : s)`; if `nn < nc`: `RwCameraSetNearClipPlane(cam, nn)`.
        If `nn == 0.1f` exactly: `source = (T - source)*0.3f + source`.
      - `nc = rwcam->nearPlane; r = ((tan(...) * aspect) * nc) * 1.1f` (note: product order differs from the first radius);
        `c = front*nc + source`; `res = TestSphereAgainstWorld(c, r, ...)`; `++n`; repeat while `n <= 5 && res`.
      (`inv` is not recomputed inside the loop.)
  13. Distance smoothing (0x510378): `D = sqrt((dz*dz + dy*dy) + dx*dx)` with `d = T - source`. If `D < m_fDistance (+0xC4)`:
      `m_fDistance = D`. Else: `p = pow(0.92f [0x862F14], ts); nd = (1.0f - p)*D + p*m_fDistance; m_fDistance = nd`; if `D > 0.05f`:
      `source = T + ((source - T) * nd) * (1.0f / D)` (per component `((u.c*nd) * (1/D)) + T.c`). Then `x = nd - 0.5f`; if
      `x < rwcam->nearPlane`: `RwCameraSetNearClipPlane(cam, (x > 0.1f) ? x : 0.1f)`.
  14. `m_bCamDirectlyBehind = m_bCamDirectlyInFront = 0; GetVectorsReadyForRW()`.
  15. If `m_bFading && m_nFadeInOutFlag == 1 && FadeValue > 0x80`: `h = atan2(-front.x, front.y)`; m_pTargetEntity
      `m_fCurrentRotation = h; m_fAimingRotation = h`; matrix ? `SetRotateZOnly(h)` : `placement.heading = h`; update the
      RwMatrix of its frame if it has an RwObject.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:1000); nothing to compare.
- Open questions: none affecting behaviour.

## 0x5105C0 CCam::Process_M16_1stPerson  (catalogue: specified/REA, gr-unreversed, port level 7)
- Convention / signature: `__thiscall void (CCam* this = ECX, const CVector& target [arg1], float orientation, float speedVar,
  float speedVarWanted)`, `ret 0x10`. `target` is copied (0x510624) but the copy is never used (it is overwritten at
  0x511828); the arg1 slot is re-used as scratch. Frame `sub esp,0x4c`, esi(=this), then ebx, ebp, edi. B = ESP after
  all four pushes. Locals: `aMax` B+0x24 = 1.0471976f (60deg), `aMin` B+0x28 = 1.4922565f (85.5deg), `upOff` B+0x14 = 0.1f,
  `fwdOff` B+0x18 = 0.3f, `attached` u8 B+0x13, `pitch` B+0x1C, `H` / `dB` B+0x20, `dA` B+0x2C.
- Callers: CCam::Process 0x526FC0.
- Callees: CPed::IsPlayer 0x5DF8F0, CControllerConfigManager::GetMouseButtonAssociatedWithAction 0x52F580 (thiscall on
  ControlsManager 0xB70198, (int action) -> EAX), CPad::GetPad 0x53FB70 (13 calls; most results unused), CPad::SniperZoomOut
  0x540B80 / SniperZoomIn 0x540B30, CCam::WellBufferMe 0x509AE0 (cdecl(float target, float* val, float* speed, float maxSpeed,
  float accel, bool angle)), CPad::LookAroundLeftRight 0x540BD0 / LookAroundUpDown 0x540CC0 (thiscall(CPed*), ret 4 -> i16),
  CCam::ClipBeta 0x509C50, _CIpow 0x822130, _CIasin 0x821E70, CPlaceable::GetMatrix 0x411990, CPlaceable::GetHeading
  0x441DB0, CPlaceable::AllocateMatrix 0x54F560 + CSimpleTransform::UpdateMatrix 0x54F1B0 (the usual "ensure matrix" pair,
  written below as `M(e)`), CPed::PositionAttachedPed 0x5DFDF0, CMatrix::UpdateRwMatrix 0x59AD70 /
  CSimpleTransform::UpdateRwMatrix 0x54EF40, CEntity::UpdateRwFrame 0x532B00, CEntity::UpdateRpHAnim 0x532B20,
  CPed::GetBonePosition 0x5E4280 (thiscall(RwV3d* out, int bone, bool)), CAutomobile::SetHeliOrientation 0x6A2450
  (thiscall(float)), CWorld::GetIsLineOfSightClear 0x56A490, RwCameraSetNearClipPlane 0x7EE1D0, CCam::GetVectorsReadyForRW 0x509CE0.
- Globals: CCamera::m_bUseMouse3rdPerson 0xB6EC2E, NewMouseControllerState m_fWheelMoved 0xB73420, m_AmountMoved 0xB73424/28,
  CCamera::m_fMouseAccelHorzntl 0xB6EC1C / m_fMouseAccelVertical 0xB6EC18, CTheScripts::fCameraHeadingWhenPlayerIsAttached
  0xA4449C, fCameraHeadingStepWhenPlayerIsAttached 0xA44498 (R/W), statics `sTargetFov` f32 0xB6FFE8, `sLosBlocked` u8 0xB6FFF4,
  f32 0xB6FFEC / 0xB6FFF0 (zeroed on reset), blur fields TheCamera 0xB6F0AC/B0/B4/B8/C8, TheCamera.m_pTargetEntity 0xB6F980,
  Scene.m_pRwCamera 0xC1703C, CTimer::ms_fTimeStep 0xB7CB5C, m_snTimeInMilliseconds 0xB7CB84. Data constants: 0x8CC474 i32 800,
  0x8CC478 u32 600, 0x8CC47C 0.95f, 0x8CC480 0.1f, 0x8CCC90 1.2f, 0x8CCC94 0.5f, 0x8CCC98 0.8f, 0x8CC7B8 0.18f, 0x8CC7BC 0.5f.
- Fields: this +0x03 m_bCollisionChecksOn, +0x0A m_bResetStatics, +0x0C m_nMode, +0xA8, +0xAC alpha, +0xB0 alphaSpeed, +0xB4 FOV,
  +0xB8 m_fFOVSpeed, +0xBC beta, +0xC0 betaSpeed, +0x118 m_fCamBumpedHorz, +0x11C m_fCamBumpedVert, +0x120 m_nCamBumpedTime (i32),
  +0x190 front, +0x19C source. Ped: +0x46C flags (bit 0x04000000 = gr bIsDucking), +0x558 m_fCurrentRotation, +0x55C m_fAimingRotation,
  +0x71C m_pTargetedObject, +0xFC m_pAttachedTo, +0x778 u16 turret position, +0x77C f32 turret limit, +0x780 f32 (gr
  m_nTurretPosnMode, used here as an offset). Attached vehicle +0x590 base type, +0x594 sub-type, CHeli +0x954 f32 orientation.
- Behaviour:
  1. If the target entity (+0x21C) is not a ped: return (0x511B2D).
  2. `attached = ped->IsPlayer() && ped->m_pAttachedTo`.
  3. Reset (if m_bResetStatics): if `m_bUseMouse3rdPerson && ped->m_pTargetedObject == 0`: if attached `{ beta = fCameraHeadingWhenPlayerIsAttached;
     alpha = 0; }` (not attached: beta/alpha untouched). Else: `beta = attached ? fCameraHeadingWhenPlayerIsAttached :
     ped->m_fCurrentRotation - PI/2; alpha = 0`. Then always: `this+0xA8 = ped->m_fCurrentRotation - PI/2; m_bResetStatics = 0;
     sLosBlocked = 0; [0xB6FFF0] = [0xB6FFEC] = 0; m_bCollisionChecksOn = 1; m_fFOVSpeed = 0; sTargetFov = FOV; alphaSpeed = 0;
     betaSpeed = 0`.
  4. Zoom: if `m_nMode != 7 && m_nMode != 46`: `FOV = 70.0f`. Else (sniper/camera):
     - `zi = GetMouseButtonAssociatedWithAction(8)`, `zo = GetMouseButtonAssociatedWithAction(9)`; `w = m_fWheelMoved`; `wheel = 0`.
       If `(w > 0 && zo == 4) || (w < 0 && zo == 5)`: `sTargetFov = ((fabs(w) * 7.0f [0x862F50] + 10000.0f) * sTargetFov) * 0.0001f`, wheel = 1.
       Else if `(w > 0 && zi == 4) || (w < 0 && zi == 5)`: `sTargetFov = sTargetFov / ((fabs(w)*7.0f + 10000.0f) * 0.0001f)`, wheel = 1.
     - If `SniperZoomOut() && !wheel`: `FOV = ((ts*255.0f + 10000.0f) * FOV) * 0.0001f; sTargetFov = FOV; m_fFOVSpeed = 0`.
       Else if `SniperZoomIn() && !wheel`: `FOV = FOV / ((ts*255.0f + 10000.0f) * 0.0001f); sTargetFov = FOV; m_fFOVSpeed = 0`.
       Else if `fabs(sTargetFov - FOV) > 0.5f`: `WellBufferMe(sTargetFov, &FOV, &m_fFOVSpeed, 0.5f, 0.25f, false)`; else `m_fFOVSpeed = 0`.
     - `if (FOV > 70) FOV = 70`; if `sTargetFov > 70`: `sTargetFov = 70` and skip the minimum clamps; else minimum = (mode 46 ? 3.0f
       : 15.0f): `if (FOV < min) FOV = min; if (sTargetFov < min) sTargetFov = min`.
     - Blur: `m_nBlurRed = 180, m_nBlurGreen = 255, m_nBlurBlue = 180, m_nBlurType = 1, m_nMotionBlur = 120`.
  5. Script heading step (0x510A04): if attached and `fCameraHeadingStep > 0.0` (compared to a double 0.0): `d = beta -
     fCameraHeadingWhenPlayerIsAttached; if (d < 0) d += 2PI; r = 2PI - d`; if `d < step || r < step`: `beta = fCameraHeading...;
     step = 0`; else if `d > r`: `beta = step + beta`; else `beta = beta - step`.
  6. Look input: mouse `mx`, `my`.
     - If `mx == 0 && my == 0`: `h = (float)(-(int)GetPad(0)->LookAroundLeftRight(ped))`, `v = (float)(int)GetPad(0)->LookAroundUpDown(ped)`.
       If attached: `a = h * 0.0078125f; f = FOV*0.0125f; dB = ((fabs(a) * (f * 0.04f [0x858CEC])) * ts) * a;
       b = v * 0.0078125f; dA = ((fabs(b) * (f * 0.034285713f [0x8631C0])) * ts) * b`.
       Else: `sH = (h < 0) ? -1 : 1; sV = (v < 0) ? -1 : 1; f = FOV*0.0125f; dB = (((h*h)*0.0001f) * (f*0.057142857f)) * ts * sH;
       dA = (((v*v)*4.4444445e-05f) * (f*0.071428575f)) * ts * sV`.
       `lag = (fabs(h) < 2.0f && fabs(v) < 2.0f) ? 0.5f [0x8CCC94] : 0.8f [0x8CCC98]`; `p = pow(lag, ts); q = 1.0f - p`;
       `betaSpeed = q*dB + p*betaSpeed; alphaSpeed = p*alphaSpeed + q*dA`; `beta = betaSpeed + beta; alpha = alphaSpeed + alpha`.
     - Else (mouse): `a = mx * -3.0f; b = my * 3.0f`; if `GetPad(0)->DisablePlayerControls != 0 || GetPad(0)->JustOutOfFrontEnd (u8
       +0x117) != 0 || !(ts > 0)` then `a = 0, b = 0`; `f = FOV*0.0125f; dB = a * (f*m_fMouseAccelHorzntl); dA = (f*m_fMouseAccelVertical)*b;
       alphaSpeed = betaSpeed = 0; beta = dB + beta; alpha = dA + alpha`.
     `ClipBeta()`.
  7. Cam bump (0x510D83): if `m_nCamBumpedTime > 0` (signed): `x = cos(((float)(uint32)(now - t0) / 800 [fidiv i32 0x8CC474]) * 2PI)`;
     `beta = (0.1f*x) * m_fCamBumpedHorz + beta; alpha = (x*m_fCamBumpedVert) * 0.1f + alpha`; `m_fCamBumpedHorz = pow(0.95f, ts) * horz;
     m_fCamBumpedVert = pow(0.95f, ts) * vert`; if `now > t0 + 600` (unsigned): `m_nCamBumpedTime = 0`.
  8. If `ped->flags46C & 0x04000000`: `fwdOff = 0.8f`.
  9. Attached (0x510E58): switch u16 `ped+0x778` (table 0x511B34; value > 3 uses case 0):
     case 0 (and default): `pitch = asin(clamp(M(att)->fwd.z, -1, 1))`, `H = (att->m_matrix ? atan2(-fwd.x, fwd.y) : placement.heading) - PI/2`;
     case 1: `pitch = asin(clamp(-right.z))`, `H = GetHeading(att)`; case 2: `pitch = asin(clamp(-fwd.z))`, `H = GetHeading(att) + PI/2`;
     case 3: `pitch = asin(clamp(right.z))`, `H = GetHeading(att) - PI`. (clamp: `x > 1 -> 1, else x < -1 -> -1`, NaN passes; matrices via
     GetMatrix 0x411990 in cases 1-3.) Then `ped->PositionAttachedPed()`; update the ped frame RwMatrix (if m_pRwObject); `UpdateRwFrame();
     UpdateRpHAnim()`. `bone = 0`; if att is a vehicle with base type 9: `GetBonePosition(&bone, 5, true); upOff = fwdOff = ped->f780`; else
     `GetBonePosition(&bone, 5, true)`. `source = bone; source += M(ped)->up * upOff` (`upOff*up.c + src.c`); `source -= M(ped)->fwd * fwdOff`.
     Heading follow: `k = ts * 0.05f [0x862F48]; l75 = 0.75f [0x862F44] * lim; l25 = (1.0f - 0.75f) * lim` (lim = ped+0x77C);
     wrap H around beta (`H - beta > PI -> H -= 2PI; else < -PI -> H += 2PI`); `d = H - beta`; if `d > l75`: `e = d - l75`; else if
     `d < -l75`: `e = d + l75`; else skip to the pitch part. `s = k * fabs(e); if (fabs(e) > l25 + s) s = fabs(e) - l25`.
     If the attach entity is a vehicle with sub-type 3 (heli) and `heli->f954 > 0`: `heli->SetHeliOrientation(heli->f954 - (ts*e)*0.1f [0x862F4C])`.
     If `e != 0`: `e < 0 ? beta = beta - s : beta = s + beta` (NaN e adds).
     Pitch follow (60deg side): `u75 = 0.75f*aMax; u25 = 0.25f*aMax` (0.25 = 1.0f - 0.75f kept on the x87 stack); if `u75 + pitch < alpha`:
     `e = pitch - alpha`; if `e < -u75`: `a = fabs(e + u75); s = ts*0.05f*a; if (u25 + s < a) s = a - u25; alpha = alpha - s`.
     (85.5deg side) `l75b = 0.75f*aMin; l25b = 0.25f*aMin`; if `pitch - l75b > alpha`: `e = pitch - alpha`; if `e > l75b`:
     `a = fabs(e - l75b); s = ts*0.05f*a; if (l25b + s < a) s = a - l25b; alpha = s + alpha`.
  10. Not attached (0x511560): `if (alpha > 1.0471976f) alpha = 1.0471976f; else if (alpha < -1.4922565f [0x8631BC]) alpha = -1.4922565f`;
      update ped RwMatrix (if m_pRwObject); `UpdateRwFrame(); UpdateRpHAnim()`; `bone = 0; GetBonePosition(&bone, 5, true); source = bone;
      source.z = source.z + 0.1f [0x858B1C]`. If ducking (step 8 bit): `src.x = src.x - 0.5f*fwd.x; src.y = src.y - 0.5f*fwd.y;
      src.x = src.x - 0.18f*right.x; src.y = src.y - 0.18f*right.y` (M(ped) before each); else `src.x -= fwdOff*fwd.x; src.y -= fwdOff*fwd.y`.
  11. (0x51176E) `if (-1.2f > alpha) alpha = -1.2f; else if (alpha > 1.2f) alpha = 1.2f`.
      `ca = cos(alpha); front = (-(cos(beta)*ca), -(sin(beta)*ca), sin(alpha))`. `P = front*3.0f + source` (B+0x38).
      `source = front*0.4f + source` (`0.4f*front.c + src.c`).
  12. Near clip / LOS (0x511883): if `m_bCollisionChecksOn == 1`: check `GetIsLineOfSightClear(P, source, true, true, false, true, false,
      true, true)`; if clear, check again with `Q = (cos(b2)*cos(a2)*3 + src.x, sin(b2)*cos(a2)*3 + src.y, sin(a2)*3 + src.z)` for
      `b2 = beta + 0.61086524f [0x8631B8], a2 = alpha - 0.34906587f [0x8631B4]`, then for `b2 = beta - 0.61086524f` (same a2). The first
      blocked test: `RwCameraSetNearClipPlane(Scene.m_pRwCamera, 0.3f); sLosBlocked = 1`, skip the rest of this step. All clear:
      `sLosBlocked = 0` and continue with the mode-46 rule. If `m_bCollisionChecksOn != 1`: if `sLosBlocked` skip, else mode-46 rule.
      Mode-46 rule: if `m_nMode == 46`: `m = (15.0f < FOV) ? 15.0f : FOV; RwCameraSetNearClipPlane(cam, ((15.0f - m) * 0.15f [0x862F54]
      + 1.0f) * 0.3f [0x858C24])`.
  13. `source -= front*0.4f` (`src.c - 0.4f*front.c`); `GetVectorsReadyForRW()`; `h = atan2(-front.x, front.y)`;
      `TheCamera.m_pTargetEntity->m_fCurrentRotation = h; ->m_fAimingRotation = h`.
- Edge cases: step 3 leaves beta/alpha untouched for a non-attached mouse player without a target (original behaviour). The second and
  third LOS probes point along +cos/+sin (the opposite of `front`), i.e. behind the camera.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:1010); nothing to compare.
- Open questions: none affecting behaviour; the meaning of ped+0x780 and heli+0x954 is only known through gr's naming.

## 0x512110 CCam::Process_WheelCam  (catalogue: specified/REA, gr-unreversed, port level 8)
- Convention / signature: `__thiscall bool (CCam* this = ECX, const CVector& target, float orientation, float speedVar,
  float speedVarWanted)`, `ret 0x10`, result in AL (0 when the line of sight from the source to the entity is blocked,
  1 otherwise; upper EAX bits undefined). No argument is read. Frame `sub esp,0x80` + ebx, ebp, esi(=this), edi; B = ESP
  after the pushes. Locals: `off` B+0x40..0x48, `R` B+0x34..0x3C, `U` B+0x28..0x30, `P` B+0x1C..0x24, `wl` B+0x10.
  A .HOODLUM stub at 0x40A453 (`mov edx,[esi+0x1A4]; jmp 0x5128DB`) is part of the body (loads source.z for the water call).
- Callers: CCam::Process_1stPerson 0x517EA0, CCam::Process 0x526FC0.
- Callees: CPlaceable::AllocateMatrix 0x54F560 + CSimpleTransform::UpdateMatrix 0x54F1B0 ("ensure matrix", `M(e)`),
  Multiply3x3 0x59C790, CMatrix::operator*(CVector) 0x59C890, CEntity::GetColModel 0x535300, CVehicle::GetVehicleAppearance
  0x6D1080, CVector::CrossProduct-out 0x59C730 (out = a x b), CVector::Normalise 0x59C910, CPed::GetBonePosition 0x5E4280,
  CPlaceable::GetMatrix 0x411990, unnamed vector scalers 0x40FEC0 (cdecl(CVector* out, const CVector* v, float s): out = s*v,
  computed z, y, x) and 0x40FE90 (cdecl(CVector* out, float s, const CVector* v): same), CWaterLevel::GetWaterLevel 0x6EB690,
  `_ftol2` 0x821B40, CWorld::ProcessLineOfSight 0x56BA00.
- Globals: CTimeCycle water colour 0xB7C508/0C/10, TheCamera blur fields 0xB6F0AC/B0/B4/B8/C8, u32 0x8CC7A4 (20), f32 0x8CC7A8 (10.0),
  CTimer::m_snTimeInMilliseconds 0xB7CB84, CTimer::ms_fTimeStep 0xB7CB5C, CWorld::pIgnoreEntity 0xB7CD68. Data constants:
  0x8CC7DC 0.33f, 0x8CC7E0 0.2f, 0x8CCC60 -0.5f, 0x8CCC64 -0.8f, 0x8CCC68 0.3f, 0x8CCC6C 0.2f, 0x8CCC70 -0.2f, 0x8CCC74 -0.3f,
  0x8CCCA0 1.0f, 0x8CCCA4 -1.0f.
- Behaviour (`e` = m_pCamTargetEntity, re-read from this+0x21C at each use):
  1. `FOV = 70.0f`.
  2. Ped (type 3): `off = (-0.3f, -0.5f, 0.1f)`; `source = Multiply3x3(M(e), &off)`; `p = pos(e)`; `source.x = source.x + p.x;
     source.y = p.y + source.y; source.z = p.z + source.z`; `front = (1, 0, 0)`.
     Not a ped: `cm = e->GetColModel()`; `off = (cm.bbox.min.x - 0.33f, -2.3f, 0.3f)`; `source = (*M(e)) * off`; `front = M(e)->fwd`.
  3. If e is a vehicle and appearance is 3 (heli) or 5 (plane): `R = mat.right` (copy), `U = mat.up` (copy), `off.x = -1.55f`;
     `source = (*M(e)) * off` (off = (-1.55, -2.3, 0.3)); go to step 7 (R/U are NOT recomputed).
  4. Vehicle with base type 5 (boat): `R = cross(front, (0,0,1))`, normalise; `U = cross(R, front)`, normalise; `P = 0`.
     If `boat->driver (+0x460)`: `driver->GetBonePosition(&P, 5, true)`; `P.x = ts*mv.x + P.x; Py = ts*mv.y + P.y; Pz = ts*mv.z + P.z`
     (mv = boat +0x44); `P.x = (R.x*-0.5f) + P.x; P.y = (R.y*-0.5f) + Py; P.z = (R.z*-0.5f) + Pz`; with `f = GetMatrix(boat)->fwd`:
     `P.x = f.x*-0.8f + P.x; P.y = f.y*-0.8f + P.y; P.z = (f.z*-0.8f + P.z) + 0.3f`. If model == 0x1AE: `P += R*0.2f` (`R.c*0.2f + P.c`);
     `t = 0x40FEC0(GetMatrix(boat)->fwd, -1.0f)`, `t2 = 0x40FE90(-0.2f, t)`; `P.x = P.x + t2.x; P.y = P.y + t2.y;
     P.z = (P.z + t2.z) + (1.0f * -0.3f)`. `source = P`. Without driver: `source.z = (0.3f + 0.3f) + source.z`. Go to step 7.
  5. Vehicle with base type 9 (bike): `R = mat.right` (copy); `front = cross(this->m_vecUp, R)` (the camera's previous up vector),
     normalise; `off.x = (0.33f - 0.2f) + off.x`; `source = pos(e)`; `source += right*off.x` (`off.x*right.c + src.c`, matrix right);
     `source += front*off.y`; `source += this->m_vecUp*off.z`. Go to step 7 (U keeps whatever B+0x28 holds: (0,0,1) written at
     0x5126A6-0x5126C9).
  6. Otherwise (ped, ordinary vehicle, train): train (base type 6) only: if `(front.z*mv.z + front.y*mv.y) + mv.x*front.x < 0`:
     `front = -front`. Then `R = cross(front, (0,0,1))`, normalise; `U = cross(R, front)`, normalise.
  7. Water blur (0x5128DB): `wl = 0`; if `GetWaterLevel(source.x, source.y, source.z, &wl, true, nullptr)` and `(wl - 0.3f) > source.z`:
     identical to Process_AttachedCam step 5 (`m = sqrt((B*B + G*G) + R*R)`; scale `10/m` when `m > 10`; `_ftol2` each;
     m_nMotionBlur = [0x8CC7A4]; m_nBlurType = 2).
  8. Roll wobble: `x = (float)(uint32)(m_snTimeInMilliseconds & 0x1FFFF) * 4.7936901e-05f [0x8631C4]`; `a = cos(x) * 0.4f`;
     `up.c = U.c*cos(a) + R.c*sin(a)` (x, y, z); `up.Normalise(); front.Normalise()`.
  9. `pIgnoreEntity = e`; `hit = ProcessLineOfSight(&source, pos(e), &cp, &ent, true, false, false, true, false, false, true, false)`;
     `pIgnoreEntity = nullptr`; return `!hit`.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:1152) returning false; nothing to compare.
- Open questions: none.

## 0x512B10 CCam::Process_AttachedCam  (catalogue: specified/REA, gr-unreversed, port level 3)
- Convention / signature: `__thiscall void (CCam* this = ECX)`, plain `ret` (0x512EE0). Frame `sub esp,0x38` + ebx, esi,
  edi (ebp pushed temporarily 0x512BC9-0x512C16). B = ESP after the three pushes; locals: `ang` B+0x10, `wl` B+0x0C,
  `U` B+0x14..0x1C, `R` B+0x20..0x28, scratch B+0x38.
- Callers: CCam::Process 0x526FC0. Callees: CPlaceable::AllocateMatrix 0x54F560, CSimpleTransform::UpdateMatrix 0x54F1B0
  (thiscall(&entity->placement, CMatrix*)), Multiply3x3 0x59C790 (cdecl(CVector* out, const CMatrix*, const CVector*),
  returns out; rotation part only), CVector::CrossProduct-out 0x59C730 (out = a x b), CVector::Normalise 0x59C910,
  CWaterLevel::GetWaterLevel 0x6EB690 (cdecl(float x, float y, float z, float* out, bool waves = 1, CVector* = 0) -> AL),
  `_ftol2` 0x821B40.
- Globals: TheCamera.m_vecAttachedCamOffset 0xB6F960, m_vecAttachedCamLookAt 0xB6F96C, m_fAttachedCamAngle 0xB6F978 (degrees),
  m_pTargetEntity 0xB6F980, m_pAttachedEntity 0xB6F984, m_bLookingAtVector u8 0xB6F054; writes m_nBlurBlue 0xB6F0AC,
  m_nBlurGreen 0xB6F0B0, m_nBlurRed 0xB6F0B4, m_nBlurType 0xB6F0B8, m_nMotionBlur 0xB6F0C8, CWorld::pIgnoreEntity 0xB7CD68.
  CTimeCycle::m_CurrentColours water colour f32 at 0xB7C508 (red), 0xB7C50C (green), 0xB7C510 (blue).
  Constants: 0.017453292f 0x8595EC, 0.3f 0x858C24, 10.0f 0x8CC7A8 (.data), u32 20 at 0x8CC7A4 (.data).
- Behaviour:
  1. `m_fFOV = 70.0f`. `ang = m_fAttachedCamAngle * 0.017453292f`.
  2. `e = m_pAttachedEntity`; if `!e->m_matrix`: `e->AllocateMatrix(); e->placement.UpdateMatrix(e->m_matrix)`.
     `source = Multiply3x3(e->m_matrix, &m_vecAttachedCamOffset)`; `p = pos(e)` (matrix ? &mat->pos : &placement.pos,
     re-read); `source.x = source.x + p.x; source.y = p.y + source.y; source.z = p.z + source.z`.
  3. If `TheCamera.m_bLookingAtVector`: same matrix check on m_pAttachedEntity; `front = Multiply3x3(mat, &m_vecAttachedCamLookAt)`;
     `front.x = front.x + p.x; front.y = p.y + front.y; front.z = p.z + front.z`; then `front -= source` (x, y, z).
     Else: `front = pos(m_pTargetEntity) - source` (no null check of m_pTargetEntity).
     `front.Normalise()`.
  4. `R = cross(front, (0,0,1))`, normalise; `U = cross(R, front)`, normalise.
  5. Underwater blur: `wl = 0`; if `GetWaterLevel(source.x, source.y, source.z, &wl, true, nullptr)` and
     `(wl - 0.3f) > source.z`: `m = sqrt((B*B + G*G) + R*R)` (water blue, green, red in that order);
     if `m > 10.0f`: `s = 10.0f / m; m_nBlurRed = _ftol2(R*s); m_nBlurGreen = _ftol2(G*s); m_nBlurBlue = _ftol2(B*s)`;
     else (also NaN) `m_nBlurRed = _ftol2(R); m_nBlurGreen = _ftol2(G); m_nBlurBlue = _ftol2(B)`;
     then `m_nMotionBlur = *(u32*)0x8CC7A4; m_nBlurType = 2`.
  6. `sA = sin(ang); cA = cos(ang)`; `up.c = U.c*cA + R.c*sA` for x, y, z (products stored first, then added).
  7. `CWorld::pIgnoreEntity = nullptr`.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:783). gr's helper `ApplyUnderwaterMotionBlur` (Cam.cpp:1158, "based on
  0x51847C") computes `sqrt(R*R + G*G + B*B)` (red first; the original sums blue first here) and writes through
  `SetMotionBlur`; values can differ in the last bit. Port the bytes.
- Open questions: none.

## 0x513510 CCam::Process_Cam_TwoPlayer_Separate_Cars  (catalogue: specified/REA, gr-unreversed, port level 1)
- Convention / signature: `__thiscall void (CCam* this = ECX)`, plain `ret` (0x513BB4 / 0x513BDF). Frame `sub esp,0x94`,
  pushes ebx, ebp, esi (edi pushed temporarily at 0x5137A1-0x513AE4). Locals below are named by offset from B0 = ESP
  after the three pushes (when EDI is pushed, `[esp+X]` = B0+X-4).
- Callers: CCam::Process 0x526FC0. Callees: CVector::Normalise 0x59C910 (10 sites), CVector::CrossProduct-out 0x59C730
  (cdecl(out, a, b) -> out = a x b; returns out in EAX).
- Globals: CWorld::Players 0xB7CD98 (CPlayerInfo stride 0x190, +0 m_pPed), Players[1].m_pPed at 0xB7CF28,
  CModelInfo::ms_modelInfoPtrs 0xA9B0C8, CTimer::ms_fTimeStep 0xB7CB5C. Constants: 1.0 0x858624, 6.0 0x858B44,
  0.0 0x858B50, 0.5 0x858B8C, PI 0x858CB8, 0.75 0x858F34, -0.13 0x8631CC, 0.04 0x8CCCB4 (in .data).
- Fields of `this`: +0xB4 m_fFOV, +0x110 used as an int32 player index 0/1 (gr names it `m_pCarWeAreFocussingOn`,
  CVehicle*; this mode stores 0/1 there), +0x114 used as an f32 blend 0..1 (gr `m_pCarWeAreFocussingOnI`), +0x148
  m_vecTargetCoorsForFudgeInter, +0x190 m_vecFront, +0x19C m_vecSource, +0x1B4 m_vecUp.
  Ped +0x58C m_pVehicle; entity +0x14 matrix (pos at +0x30), +4 placement pos, +0x22 model; CBaseModelInfo +0x14
  m_pColModel; CColModel +0x14 bbox.max.z; physical +0x44 m_vecMoveSpeed.
- Behaviour (`pos(e)` = `e->matrix ? &matrix->pos : &placement.pos`, re-evaluated at each use):
  1. `FOV = 80.0f` (0x513519). `V1 = Players[0].ped->m_pVehicle` (ESI), `V2 = Players[1].ped->m_pVehicle` (EBP); no null
     checks.
  2. `D = pos(V2) - pos(V1)` (computed z, y, x; stored x, y, z), `D.Normalise()`.
  3. `A = pos(V1); A.z = (A.z + col(V1).max.z) + 1.0f`; with `X6 = D.x*6, Y6 = D.y*6, Z6 = D.z*6` (each `D.c * 6.0f`):
     `A.x -= X6; A.y -= Y6; A.z -= Z6`. `Bp = pos(V2); Bp.z = (Bp.z + col(V2).max.z) + 1.0f; Bp.x = X6 + Bp.x;
     Bp.y = Y6 + Bp.y; Bp.z = Z6 + Bp.z`.
  4. `E1 = pos(V2) - A`, normalise (B0+0x78). `E2 = pos(V1) - Bp`, normalise (B0+0x84).
  5. `N = (A.y - Bp.y, Bp.x - A.x, 0)`, normalise; then `N.z = -0.1f`, normalise again (B0+0x60).
  6. `ang = this->f114 * PI; sA = sin(ang); w = (cos(ang) + 1.0f) * 0.5f` (fsin/fcos of the same x87 value).
  7. `dist = sqrt((dx*dx + dz*dz) + dy*dy)` with `d = pos(V1) - pos(V2)`.
     `sx = sA*N.x, sy = sA*N.y, sz = sA*N.z`; `O.c = ((s_c) * dist) * 0.75f` for c = x, y, z.
     `w2 = 1.0f - w`; `C.x = w*A.x + Bp.x*w2; C.y = w*A.y + Bp.y*w2; C.z = w*A.z + Bp.z*w2` (products formed first,
     then added in this order). `source = C - O` (x, y, z).
  8. `F.c = E1.c*w + E2.c*w2` (x, y, z); `k = 1.0f - sA`; `front = (F.x*k + sx, F.y*k + sy, F.z*k + sz)`;
     `front.Normalise()`.
  9. `m_vecTargetCoorsForFudgeInter = (this->i110 != 0) ? *pos(V1) : *pos(V2)`.
  10. `up = (0, 0, 1)`, Normalise; `R = cross(up, front)` (CrossProduct-out(out, &up, &front)), normalise;
      `up = cross(front, R)`, normalise.
  11. Player switch: `Va = Players[i110].ped->m_pVehicle`, `Vb = Players[(i110 - 1) & 1].ped->m_pVehicle`;
      `H = (front.x, front.y, 0)`, normalise. `d1 = (va.x*H.x + va.z*H.z) + H.y*va.y` (va = Va->m_vecMoveSpeed).
      If `d1 < -0.13f` (`fcom; test ah,5; jp` - NaN/>= skip): `d2 = (vb.z*(-H.z) + vb.y*(-H.y)) + (-H.x)*vb.x`;
      if `d1 < d2`: `i110 = (i110 - 1) & 1`.
  12. Blend (0x513B74): `inc = ts * 0.04f`. If `i110 != 0`: `t = inc + f114; f114 = (t < 1.0f) ? t : 1.0f` (NaN -> 1.0).
      Else: `t = f114 - inc; f114 = (t > 0.0f) ? t : 0.0f` (NaN -> 0).
- Edge cases: no null checks on the players' peds/vehicles (the mode is only used when both are in cars). The index
  i110 is read from the CCam field before step 11 updates it; step 9 uses the old value.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:798), `.Reversed = false`; row Cam.cpp:799 is not a behavioural
  difference (stub never executes). Name note: gr's field names for +0x110/+0x114 do not match their use here.
- Open questions: none.

## 0x517500 CCam::Process_SpecialFixedForSyphon  (catalogue: specified/REA, gr-unreversed, port level 9)
- Convention / signature: `__thiscall void (CCam* this = ECX, const CVector& target [B+0x2C], float, float, float)`, `ret 0x10`;
  only `target` is read. Frame `sub esp,0x18` + ebx, ebp, esi(=this), edi; B = ESP after the pushes. Locals `L` B+0x10..0x18,
  `R` B+0x1C..0x24.
- Callers: CCam::Process 0x526FC0.
- Callees: CCamera::AvoidTheGeometry 0x514030 (thiscall on TheCamera, 4 args), CCam::GetVectorsReadyForRW 0x509CE0,
  CVector::Normalise 0x59C910, CVector::CrossProduct-out 0x59C730 (out = a x b), CPed::GetWeaponSkill 0x5E6580,
  CWeaponInfo::GetWeaponInfo 0x743C60 (cdecl(eWeaponType, eWeaponSkill) -> CWeaponInfo*), VectorSub 0x40FE60
  (cdecl(out, a, b): out = a - b), CPlaceable::SetHeading 0x43E0C0, CEntity::UpdateRwMatrix 0x446F90.
- Fields: this +0x38 m_fSyphonModeTargetZOffSet, +0xB4 FOV, +0x148 m_vecTargetCoorsForFudgeInter, +0x160 m_vecCamFixedModeSource,
  +0x16C m_vecCamFixedModeUpOffSet, +0x190 front, +0x19C source, +0x1B4 up, +0x21C target. Ped: +0x46C flags, +0x558, +0x55C,
  +0x5A0 weapons (stride 0x1C, +0 type), +0x718 active slot (i8), +0x71C m_pTargetedObject. CWeaponInfo +0x00 m_nWeaponFire,
  +0x18 flags (bit 1 tested).
- Behaviour:
  1. `zo = m_fSyphonModeTargetZOffSet` (loaded first). `source = m_vecCamFixedModeSource`. `fudge = *target; fudge.z = zo + fudge.z`.
     `front = *target - source` (the unmodified target).
  2. `L = m_vecCamFixedModeSource`; `TheCamera.AvoidTheGeometry(&L, &fudge, &source, FOV)`.
  3. `front.z = front.z + m_fSyphonModeTargetZOffSet`; `GetVectorsReadyForRW()`.
  4. `up.x = up.x + upOff.x; up.y = upOff.y + up.y; up.z = upOff.z + up.z` (upOff = +0x16C); `up.Normalise()`;
     `R = cross(up, front)`, normalise; `front = cross(R, up)`, normalise.
  5. `FOV = 70.0f`.
  6. Ped aim: `p = m_pCamTargetEntity`; if `p` is a ped with `m_pTargetedObject`: `wi = GetWeaponInfo(p->m_aWeapons[(int8)slot].type,
     p->GetWeaponSkill())`; if `wi` and not (`(wi->flags >> 1) & 1` and the ped is not ducking (`flags46C & 0x04000000` == 0)) and
     `wi->m_nWeaponFire != 0`: `d = pos(target) - pos(p)`; `h = atan2(-d.x, d.y)`; `p->m_fAimingRotation = h;
     p->m_fCurrentRotation = h; p->SetHeading(h); p->UpdateRwMatrix()`.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:1147); nothing to compare.
- Open questions: none.

## 0x518500 CCam::ProcessArrestCamOne  (catalogue: specified/REA, gr-unreversed, port level 10)
- Convention / signature: `__thiscall bool (CCam* this = ECX)`, plain `ret`, AL result (gr declares `void`; the bytes return a bool
  in AL: 0 only when the target is neither ped nor vehicle, or from the mode-1 helper). Frame `sub esp,0x68` + ebx(=cop), ebp(=this),
  esi, edi(=player ped); B = ESP after the pushes. Locals: `modes` i32[6] B+0x60..0x74 (initialised to -1 with `rep stosd`), `found`
  u8 B+0x13, `cop` B+0x14, `ped` B+0x18, `i` B+0x1C, `out` CVector B+0x24, `P` CVector B+0x30 (look-at point), `R` B+0x3C, `L` B+0x48/B+0x54.
- Callers: CCam::Process 0x526FC0.
- Callees: CPed::GetBonePosition 0x5E4280, CWorld::FindPlayerPed 0x56E210 (cdecl(int -1)), `rand` 0x821B1E, CPed::IsPlayer 0x5DF8F0,
  CText::Get 0x6A0050 (on TheText 0xC1B340, key "BUSTED" at 0x85987C), CMessages::AddBigMessage 0x69F2B0 (cdecl(text, 5000, 2)),
  unnamed CCam helpers 0x512EF0 (thiscall(CPed* cop, bool init) -> AL), 0x515D80 and 0x516010 (thiscall(CEntity* target, CPed* cop,
  CVector* P, CVector* out) -> AL), CCam::GetLookFromLampPostPos 0x5161A0 (same argument shape), CTaskManager::FindActiveTaskByType
  0x681740 (type 1100), CEntity::RegisterReference 0x571B70, CCamera::AvoidTheGeometry 0x514030, CVector::Normalise 0x59C910,
  CVector::CrossProduct-out 0x59C730 (out = a x b), CWorld::TestSphereAgainstWorld 0x569E20, CWorld::FindGroundZFor3DCoord 0x5696C0
  (cdecl(x, y, z, bool* found, CEntity** = 0) -> ST0).
- Globals: TheCamera.m_pTargetEntity 0xB6F980, `sArrestMode` i32 0xB6EC58, `s_ExitCam` CEntity* 0xB6EC5C, f32 0xB6EC60 (start time),
  u8 0xBAADC0 (when non-zero the BUSTED message is suppressed), CTimer::m_snTimeInMilliseconds 0xB7CB84, ms_fTimeStep 0xB7CB5C.
  Data: 0x8CC7E4 0.1f, 0x8CC7E8 0.05f, 0x8CC7EC 0.1f, 0x8CC7F8 0.7f, 0x8CC7FC 10.0f, 0x8CC800 0.1f; 0x858C7C 1/32767, 0x858F50 0.65f.
  Player data: `FindPlayerPed(-1)->+0x480` (CPlayerData*) `->+8` = arresting cop (gr m_ArrestingOfficer).
- `GETCOP`: `cop = FindPlayerPed(-1) && FindPlayerPed(-1)->data->cop ? FindPlayerPed(-1)->data->cop : <unchanged>` (FindPlayerPed called up
  to 3 times).
- Behaviour:
  1. `modes[0..5] = -1; found = 0; cop = 0; FOV = 45.0f`.
  2. Reset path (m_bResetStatics):
     - `t = TheCamera.m_pTargetEntity; sArrestMode = 0`.
     - Ped: `ped = t; t->GetBonePosition(&P, 3, true)`; GETCOP; if a cop was found and `(float)rand()*(1/32767) > 0.5f`: `modes =
       {1, 2, 3, 2, 8}`; else `modes = {1, 3, 2, 8}`.
       Vehicle: if `t->driver && driver->IsPlayer()`: `ped = driver; driver->GetBonePosition(&P, 3, true)`; else `ped = 0; P = pos(t)`;
       GETCOP; if cop and `rand()*(1/32767) > 0.65f`: `modes = {2, 8, 3, 2}`; else `modes = {8, 3, 2}`.
       Other entity types: return 0.
     - If `!byte[0xBAADC0]`: `CMessages::AddBigMessage(TheText.Get("BUSTED"), 5000, 2)`.
     - For `i = 0; i < 6 && modes[i] > 0 && sArrestMode == 0; ++i`: `s_ExitCam = nullptr`; by `modes[i]`:
       1: `[0xB6EC60] = (float)(uint32)now`; if `F512EF0(cop, true)`: `t->flags1C &= ~0x80; sArrestMode = 1; m_bResetStatics = 0`;
          return 1. (failure: next i)
       2: if cop: `found = F515D80(t, cop, &P, &out); s_ExitCam = cop; cop = 0`. Else if `ped`: scan the 16 pointers at
          `ped->m_pIntelligence (+0x47C) + 0x130`: for each non-null `p` whose active task of type 1100 has `task+8 == FindPlayerPed(-1)`:
          `found = F515D80(t, p, &P, &out)`; on success `s_ExitCam = p` and stop.
       3: if cop: `found = F516010(t, cop, &P, &out); s_ExitCam = cop; cop = 0`. Else the same ped scan as mode 2 (calling 0x515D80,
          not 0x516010 - as in the bytes at 0x518A19).
       4..7: nothing.  8: `found = GetLookFromLampPostPos(t, cop, &P, &out)`.
       If `found`: if `s_ExitCam`: `s_ExitCam->RegisterReference(&s_ExitCam)`; `sArrestMode = modes[i]`; if it is 3: `r = rand()*(1/32767)*5.0f`;
       `sArrestMode = r < 1 ? 3 : r < 2 ? 4 : r < 3 ? 5 : r < 4 ? 6 : 7`.
     - Finish (0x518737): `source = out; L = out; TheCamera.AvoidTheGeometry(&L, &P, &source, FOV); front = P - source`, normalise;
       `up = (0,0,1); R = cross(front, up)`, normalise; `up = cross(R, front)`. If `sArrestMode == 0`: return 1 WITHOUT clearing
       m_bResetStatics (the search is retried next frame). Else `m_bResetStatics = 0`; return 1.
  3. Running path:
     - `sArrestMode == 1`: `t->flags1C &= ~0x80`; `cop = 0`, GETCOP; return `F512EF0(cop, false) != 0`.
     - Otherwise compute `P` as in step 2 (ped: bone 3; vehicle with player driver: driver bone 3; vehicle otherwise: pos; other: return 0).
     - Mode 2: if `s_ExitCam`: `ok = F515D80(t, s_ExitCam, &P, &out)`; `zl = ts*0.1f [0x8CC7EC] + source.z`; `if (out.z > zl) out.z = zl`;
       if `ok` go to APPLY, else KEEP. If `!s_ExitCam`: KEEP.
     - Modes 4..7: `out = source; front = P - source`, normalise; `R = cross(front, (0,0,1))`, normalise; modes 6/7: `R = -R` (each times -1.0f).
       If `TestSphereAgainstWorld(R*0.5f + source, 0.4f, t, true, true, false, true, false, true)`: KEEP. Else `out.c = (R.c*0.1f)*ts + out.c`;
       modes 5/7: `out.z = ts*0.05f + out.z`; modes 4/6: `gz = FindGroundZFor3DCoord(out.x, out.y, out.z, &f, nullptr)` (f pre-cleared);
       if `f`: `out.z = gz + 0.7f`. APPLY.
     - Mode 8: `out = source; front = (P - out); front.z = 0`, normalise; `R = cross(front, (0,0,1))`, normalise; `front = (P - out) + R*10.0f`
       (`D.x + R.x*10`, etc.); `front.z = 0`, normalise. If `TestSphereAgainstWorld(front*0.5f + out, 0.4f, t, true, true, false, true, false,
       true)`: KEEP. Else `out.c = (0.1f*front.c)*ts + out.c`; APPLY.
     - Other modes: KEEP.
     KEEP: `L = source; TheCamera.AvoidTheGeometry(&L, &P, &source, FOV)`; return 1.
     APPLY: `source = out; L = out; AvoidTheGeometry(&L, &P, &source, FOV); front = P - source`, normalise; `R = cross(front, (0,0,1))`,
     normalise; `up = cross(R, front)`; return 1.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:523) with `void` return; the original returns a bool in AL (CCam::Process may use it).
- Open questions: the meaning of entity flag bit 0x80 at +0x1C and byte 0xBAADC0 (behaviour is by address).

## 0x519250 CCam::ProcessPedsDeadBaby  (catalogue: specified/REA, gr-unreversed, port level 9)
- Convention / signature: `__thiscall void (CCam* this = ECX)`, plain `ret`. Frame `sub esp,0x5C` + ebx, ebp, esi, edi(=this);
  B = ESP after the pushes. Locals: `TP` B+0x30..0x38 (target point), `S` = (EBP, EBX, [B+0x20]) (new source), `wl` B+0x14,
  `R` B+0x24..0x2C, `dt` B+0x10.
- Callers: CCam::Process 0x526FC0.
- Callees: CPed::GetBonePosition 0x5E4280, CEntity::GetColModel 0x535300, CWaterLevel::GetWaterLevelNoWaves 0x6E8580
  (cdecl(float x, float y, float z, float* out, ...two zero args) -> AL), CPlaceable::AllocateMatrix 0x54F560 +
  CSimpleTransform::UpdateMatrix 0x54F1B0, CVector::CrossProduct-out 0x59C730 (out = a x b), CVector::Normalise 0x59C910,
  CWorld::TestSphereAgainstWorld 0x569E20, CCamera::AvoidTheGeometry 0x514030.
- Globals: TheCamera.m_pTargetEntity 0xB6F980, m_nTimeLastChange u32 0xB6F094, m_bMoveCamToAvoidGeom u8 0xB6F055 (W 0),
  CTimer::m_snTimeInMilliseconds 0xB7CB84, ms_fTimeStep 0xB7CB5C, statics f32 0xB70054 and u32 0xB70050 (written on reset only).
  Constants: 0x8CC804 2.0f, 0x8CC808 0.04f, 0x8CC80C 2.0f, 0x8CC810 600.0f, 0x8CC814 -0.35f (.data); 0x858CE8 1.5f, 0x858CC4 0.2f,
  0x858C4C 1000.0f, 0x858CDC 0.001f, 0x8631F0 2000.0f, 0x859978 0.0005f.
- Behaviour:
  1. `t = TheCamera.m_pTargetEntity`. Ped: `t->GetBonePosition(&TP, 3, true)`. Vehicle: `TP = pos(t); TP.z = TP.z + t->GetColModel()->bbox.max.z`.
     Any other type: return.
  2. Reset (m_bResetStatics): `[0xB70054] = (float)(uint32)m_snTimeInMilliseconds; TheCamera.m_nTimeLastChange = m_snTimeInMilliseconds;
     [0xB70050] = 0`; `S = (TP.x, TP.y, 2.0f + TP.z)`; `wl = 0`; if `GetWaterLevelNoWaves(TP.x, TP.y, S.z, &wl, 0, 0)`:
     `w = wl + 1.5f; if (w > S.z) S.z = w`. `R = cross(M(t)->fwd, (0,0,1)); R.z = 0; R.Normalise()`. `front = TP - S`, normalise;
     `up = cross(R, front)`, normalise; `m_bResetStatics = 0`. Go to step 4.
  3. Not reset: `S = this->source`. If `!TestSphereAgainstWorld((S.x, S.y, S.z + 0.2f), 0.3f, t, true, true, false, true, false, true)`:
     `S.z = ts*0.04f + S.z`. `R` as in step 2. `dt = (float)(uint32)(m_snTimeInMilliseconds - m_nTimeLastChange)`;
     `m = (1000.0f < dt) ? 1000.0f : dt; k = sin(dt / 600.0f) * (m * 0.001f)`.
     `Q.x = (R.x*2.0f)*k + TP.x; Q.y = (R.y*2.0f)*k + TP.y`; `D = (Q.x - S.x, Q.y - S.y, 0)`, normalise.
     If `!TestSphereAgainstWorld((D.x*0.2f + S.x, D.y*0.2f + S.y, D.z*0.2f + S.z), 0.3f, t, true, true, false, true, false, true)`:
     `S.x = Q.x; S.y = Q.y` (S.z unchanged).
     `front = (0, 0, -1)`; `m2 = (2000.0f < dt) ? 2000.0f : dt; w = m2 * 0.0005f; c = cos(dt / 600.0f)`;
     `front.c = ((R.c * -0.35f) * w) * c + front.c` (R.z term as `-0.35f * R.z`); `front.Normalise()`; `up = cross(R, front)`, normalise.
  4. `source = S`; `L = S`; `TheCamera.AvoidTheGeometry(&L, &TP, &source, FOV)`; `TheCamera.m_bMoveCamToAvoidGeom = 0`.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:528); nothing to compare.
- Open questions: none.

## 0x519810 CCam::Process_Cam_TwoPlayer_InCarAndShooting  (catalogue: specified/REA, gr-unreversed, port level 22)
- Convention / signature: `__thiscall void (CCam* this = ECX)`, plain `ret`. MSVC SEH frame (handler 0x83C798; the try-level local covers
  a stack CTaskSimpleGangDriveBy), `sub esp,0xC4`, pushes ebx(=vehicle V), ebp(=shooter), esi(=this), edi; B = ESP after the pushes.
  Locals: `P` B+0x1C..0x24 (vehicle pos, later the aim point), `pad` B+0x2C, `H` B+0x30 (vehicle heading - PI/2), `wi` B+0x38, `target` B+0x48,
  `outX/outY` B+0x4C/0x50, `excess` B+0x28, `base` B+0x34, `len` B+0x14, `maxz`/`maxD` B+0x18, `minD` B+0x10.
- Callers: CCam::Process 0x526FC0.
- Callees: CPad::GetPad, CPad::AimWeaponLeftRight/UpDown (thiscall(CPed*)), _CIpow, CPed::GetWeaponSkill 0x5E6580, CWeaponInfo::GetWeaponInfo 0x743C60,
  CWeapon::FindNearestTargetEntityWithScreenCoors 0x73E240 (cdecl(float x, float y, float range, CVector point, float* outX, float* outY) -> CEntity*),
  CEntity::GetColModel, CVehicle::GetVehicleAppearance, CCam::RotCamIfInFrontCar 0x50A4F0 (thiscall(const CVector&, float)), CCamera::AvoidTheGeometry,
  CCam::GetVectorsReadyForRW, CVehicle::CanPedLeanOut 0x6D5CF0 (thiscall(CPed*) -> AL), CWeapon::Update 0x73DB40 (thiscall(CPed*)), CPad::GetCarGunFired
  0x53FF90 (-> AX), CWeapon::IsTypeMelee, CVector::CrossProduct-out, vector helpers 0x40FEC0/0x40FE30/0x40FEF0/0x411A00, VectorSub 0x40FE60,
  CPlaceable::GetHeading, `_ftol2`, CTaskSimpleGangDriveBy ctor 0x6217D0, unnamed 0x627CC0 (thiscall on the task, (CPed*)), its destructor 0x6218C0,
  CCamera::CamShakeNoPos 0x50A970 (cdecl(CCamera*, float)).
- Fields: this +0x70 m_fDistanceBeforeChanges (W), +0xAC alpha, +0xB4 FOV, +0xBC beta, +0xCC/+0xD0 min/max distance, +0x108 m_fX_Targetting,
  +0x10C m_fY_Targetting, +0x148 fudge, +0x190 front, +0x19C source, +0x1B4 up. Vehicle: +0x460 driver, +0x468 passenger[1], +0x44 move speed,
  +0x594 sub-type. Ped weapon state at +0x5A4 (`m_aWeapons[slot].m_State`).
- Data: 0x8CC53C 0.6f, 0x8CC540 0.4f, 0x8CC544 0.98f, 0x8CC600 1.3f, 0x8CC608 0.4f, 0x8CCD18 0.03f, 0x8CCD1C 0.3f; 0x862F60 0.5f, 0x862F64 2.0f,
  0x862F68 -0.34906587f, 0x862F6C 0.5f, 0x862F70 -1.0471976f, 0x862F74 0.5235988f, 0x862F78 0.96f, 0x862F7C 5.0f, 0x862F80 -1.0471976f,
  0x862F84 0.17453294f, 0x862F88 3.0f, 0x862F8C 0.004f, 0x862F90 0.85f, 0x862F94 0.01f, 0x8631D4 0.008726646f, 0x8631F8 (double) 0.002.
- Behaviour:
  1. If the target entity is not a vehicle: return. `V = Players[0].ped->m_pVehicle`; `P = pos(V)`; `H = heading(V) - PI/2`.
     If `V->driver == Players[0].ped`: `pad = GetPad(1); shooter = Players[1].ped`; else `pad = GetPad(0); shooter = Players[0].ped`.
  2. FOV: identical to Process_FollowCar_SA step 10 without the reset branch (speed bonus for sub-type 0/9 with 0.4f, decay `pow(0.98f, ts)` above 70,
     clamp [70, 100]).
  3. Aim cursor: `LR = (int)AimWeaponLeftRight(shooter)`, `UD = -(int)AimWeaponUpDown(shooter)`; `f = FOV*0.0125f`;
     `X += ((((f*0.071428575f)*|LR|)*ts)*0.004f*0.004f)*LR; Y += ((((f*0.042857144f)*|UD|)*ts)*0.004f*0.004f)*UD`.
     `wi = GetWeaponInfo(shooter weapon type, shooter->GetWeaponSkill())`; `target = FindNearestTargetEntityWithScreenCoors(X, Y, wi->m_fTargetRange + same,
     pos(shooter), &outX, &outY)`. If `target && |LR| < 120 && |UD| < 120`: `k = 1 - pow(0.85f, ts); m = ts*0.01f`; `X += clamp((outX - X)*k, -m, m);
     Y += clamp((outY - Y)*k, -m, m)`.
     `excess = 0`; `X > 0.9f -> excess = X - 0.9f, X = 0.9f`; `X < -0.9f -> excess = X + 0.9f, X = -0.9f`; `Y > 0.9f -> alpha = alpha - ((Y - 0.9f)*ts)*0.5f,
     Y = 0.9f`; `Y < -0.9f -> alpha = alpha - ((Y + 0.9f)*ts)*0.5f, Y = -0.9f`.
     Recentre: if `|LR| < 1 && |UD| < 1 && !target`: `dy = Y + 0.4f` (`Y - (-0.4f)`); `d = sqrt(dy*dy + X*X); s = ts*0.002f`; if `d < s`: `X = 0, Y = -0.4f`;
     else `X = X - (s*(1/d))*X; Y = Y - ((1/d)*s)*dy`.
  4. Distances: `maxz = col(V).bbox.max.z; len = 0.5f + (|col.min.y| + same); base = -0.34906587f`. Heli appearance with status != 8:
     `P.c = ((0.6f*up.c)*maxz) + P.c`; else `d = maxz*1.3f - 0.4f`; if `d > 0`: `P.z += d; len += d; base = (0.3f/len)*d + base`.
     `minD = len*0.9f = m_fCaMinDistance; maxD = 2.0f + len = m_fCaMaxDistance`.
     `dx = src.x - P.x, dy = src.y - P.y; d = sqrt(dx*dx + dy*dy); m_fDistanceBeforeChanges = d`; if `d < 0.002`: `d = 0.002f`;
     `beta = atan2(-(P.x - src.x), P.y - src.y) - PI/2`; if `d > maxD`: `k = maxD/d`; else if `d < minD`: `k = minD/d`; else no change; with k:
     `src.x = dx*k + P.x; src.y = dy*k + P.y`.
  5. Pitch: if `|mv|^2 > 0.0001f` (V): `pv = atan2(mv.z, sqrt(mv.x*mv.x + mv.y*mv.y)); k = (sub-type 3) ? 3.0f : 5.0f; s = min((|mv| - 0.01f)*k, 1.0f);
     base = pv*s + base; p = pow(0.96f, ts); alpha = (1-p)*base + p*alpha`. Clamp: `lo/hi = (-1.0471976f, 0.5235988f)`, heli `(-1.0471976f, 0.17453294f)`;
     `if (lo > alpha) alpha = lo; else if (alpha > hi) alpha = hi`. `src.z = P.z - sin(alpha)*maxD`.
  6. `RotCamIfInFrontCar(P, H); fudge = P; L = source; TheCamera.AvoidTheGeometry(&L, &fudge, &source, FOV)`.
     Rotate by the X excess: `dx = src.x - P.x, dy = src.y - P.y, s = sin(excess), c = cos(excess)`; `src.x = (s*dy + c*dx) + P.x; src.y = (c*dy - s*dx) + P.y`.
     `front = P - source`; `m_bResetStatics = 0`; `GetVectorsReadyForRW()`.
  7. Shooting: if `!V->CanPedLeanOut(shooter)`: `shooter weapon.Update(shooter)`. If `pad->GetCarGunFired() && !CanPedLeanOut(shooter) && !IsTypeMelee &&
     weapon.m_State == 0`: aim point `A = target ? pos(target) : source + ((front + (X*R)*tanH) - (Y*tanV)*up) * (wi->m_fTargetRange*3.0f)` where
     `R = cross(front, up)`, `tanH = tan(FOV*0.008726646f)`, `tanV = tanH/aspect` (both fptan of the same value). `left = (shooter != V->passenger[1])`.
     `ang = atan2(-(A - source).x, (A - source).y) - GetHeading(V)` wrapped +-PI, `+ PI/4`, `+2PI if < 0`; `dir = _ftol2(ang * 0.63661975f)`.
     Build `CTaskSimpleGangDriveBy task(nullptr, nullptr, 100.0f, 100, 8, left)` on the stack; `task.weaponInfo (+0x34) = GetWeaponInfo(type, skill)`;
     `task.byte18 = dir`; `task.F627CC0(shooter)`; `CCamera::CamShakeNoPos(&TheCamera, 0.03f)`; destroy the task (0x6218C0).
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:793); nothing to compare.
- Open questions: the exact role of 0x627CC0 (fires the drive-by from the stack task) belongs to the task's own spec.

## 0x51A740 CCam::Process_DW_HeliChaseCam  (catalogue: specified/REA, gr-unreversed, port level 9)
- Convention / signature: `__thiscall bool (CCam* this = ECX, bool arg [esp+4])`, `ret 4`, result in AL (1 = camera produced,
  0 = mode failed / not applicable). The bool argument is never read. Entry `nop; jmp 0x409B4B` (.HOODLUM: `sub esp,0xEC;
  jmp 0x51A746`), then pushes ebx(=this), ebp, esi, edi. B = ESP after the pushes.
- Callers: CCamera::IsItTimeForNewcam 0x51D770, CCamera::TryToStartNewCamMode 0x51E560, CCam::Process 0x526FC0.
- Callees: `rand` 0x821B1E, `_ftol2` 0x821B40, CCam::GetCoreDataForDWCineyCamMode 0x517130 (thiscall, 12 reference args,
  see gr Cam.cpp:244), unnamed 0x50E180 and 0x50E090 (thiscall on the settings block, no args; specified elsewhere),
  CHandShaker::Reset 0x50D860 (on gHandShaker[0] 0xB6ECA0), CWorld::TestSphereAgainstWorld 0x569E20, CWorld::ProcessLineOfSight
  0x56BA00, CCam::IsTimeToExitThisDWCineyCamMode 0x517400 (thiscall(int camId, const CVector* src, const CVector* dst, float t,
  bool los)), CCam::Finalise_DW_CineyCams 0x50DD70 (thiscall(const CVector* src, const CVector* dst, float roll, float fov,
  float nearClip, float shake)).
- Locals (outputs of GetCoreData): `ent` B+0x6C, `veh` B+0x10, `dest` B+0x1C, `src` B+0x38, `up` B+0x8C, `right` B+0x80,
  `fwd` B+0x28, `vel` B+0x54, `speed` B+0x14, `angVel` B+0x70, `angSpeed` B+0x44, colsphere B+0xBC. Several of these slots are
  re-used as temporaries later (noted where relevant).
- Globals: TheCamera.m_bUseNearClipScript u8 0xB6F059 (W 0), `sFailed` u8 0xB6EC70, `gDWCamMode` i32 0x8CC488 (init -1),
  gLastFrameProcessedDWCineyCam 0x8CCB9C, `sSceneStart` u32 0x8CCBA0, s_DWCineyCamSceneEndTime 0x8CCBA4, u32 0x8CCBA8
  (20000), CTimer::m_FrameCounter 0xB7CB4C, m_snTimeInMilliseconds 0xB7CB84, CWorld::pIgnoreEntity 0xB7CD68, statics
  `sLockPos` CVector 0xB70058, guard 0xB70064 (bit 0, no other effect), `sLockCount` i32 0x8CCD24 (init 100), f32 0x8CCD20 (0.5),
  f32 0x8CCD28 (15.0).
- Settings block `S` (unnamed, 0x9C bytes): `S = 0xB6FEC0 - k*0x9C` with `k = _ftol2(((float)(rand() & 0xFFFF) * (1/32768)) *
  0.0f)` - always 0, so `S = 0xB6FEC0` (the rand() call still consumes one random number). Fields by offset: +0x00 CVector A
  (end position), +0x0C CVector Bs (start position), +0x18, +0x1C, +0x20, +0x24, +0x28, +0x2C, +0x30, +0x34, +0x38, +0x3C f32,
  +0x40 u8 locked, +0x44 i32 lockFrames, +0x48 i32, +0x4C CVector lockDest, +0x58 i32 tries, +0x5C u8, +0x60 i32, +0x64 i32,
  +0x68, +0x6C, +0x70, +0x74 f32, +0x78 f32 (sphere radius, passed as raw dword), +0x7C f32, +0x80 u8, +0x81 u8, +0x84 f32,
  +0x88 u8, +0x8C i32, +0x90 i32, +0x94 f32, +0x98 i32.
- Common sub-formula `E(x) = (sin((270.0f [0x859070] - x*180.0f [0x85A994]) * 0.017453292f) + 1.0f) * 0.5f` (ease in/out).
- Behaviour:
  1. `TheCamera.m_bUseNearClipScript = 0`; draw `k` (above). If `m_pCamTargetEntity` is null or not a vehicle: return 0.
  2. `GetCoreDataForDWCineyCamMode(...)`. `now = m_snTimeInMilliseconds`.
  3. Start (0x51A81F) unless `gDWCamMode == 56 && gLastFrameProcessedDWCineyCam >= m_FrameCounter - 1` (unsigned):
     `sFailed = 0; gDWCamMode = 56; sSceneStart = now; s_DWCineyCamSceneEndTime = now + [0x8CCBA8]`; `S->F50E180()`;
     `S.7C = S.2C - S.30; S.44 = S.48; S.40 = 0; S.5C = 0; S.80 = 0; S.81 = 0; S.64 = S.60; S.88 = 0`; `S->F50E090()`;
     `gHandShaker[0].Reset()`. Then up to `S.58` tries (`n = 0..`, signed compare):
     - `Bs = dest - fwd*S.1C` (`dest.c - fwd.c*S.1C`); `A = fwd*S.18 + dest`; `Bs.z = Bs.z + S.20; A.z = S.20 + A.z`.
     - `sx = (rand() < 0x3FFF) ? -1 : 1; sy = (rand() < 0x3FFF) ? -1 : 1` (two rand() calls in this order).
     - `Bs += (right.x*S.24, right.y*S.24, S.24*0.0f) * sx`; `A += (right.x*S.24, right.y*S.24, S.24*0.0f) * sy` (z term is a
       multiplication of S.24 by the constant 0.0, not right.z).
     - If `!TestSphereAgainstWorld(Bs, S.78, nullptr, true, true, false, false, false, false)`: `pIgnoreEntity = ent`;
       `clear = !ProcessLineOfSight(&dest, &Bs, &cp, &e, true, true, false, false, false, false, false, false)`;
       `pIgnoreEntity = nullptr`; if clear: `S.81 = rand() < 0x3FFF; S.88 = rand() < 0x3FFF`; continue at step 4.
     All tries failed: repeat the field reset (S.7C, S.40, S.5C, S.80, S.81, S.64 = S.60, S.44 = S.48, S.88), `sFailed = 1`,
     return 0.
  4. Per frame (0x51AB85): if `sFailed`: return 0. `f = (float)(int)(now - sSceneStart) / (int)(sceneEnd - sSceneStart)` (fild/fidiv).
     `P = (A - Bs)*f + Bs` (per component `(A.c - Bs.c)*f + Bs.c`); `Q = P` (src slot B+0x38).
     `dest.c = (fwd.c*speed*S.34 + fwd.c) + dest.c` (`((fwd.c*speed)*S.34) + fwd.c`, then added to dest).
     `v = (dest.x - P.x, dest.y - P.y)`; `d = sqrt(v.y*v.y + v.x*v.x)`; `v *= 1/d`; if `d < S.74`: `Q.x = dest.x - v.x*S.74;
     Q.y = dest.y - v.y*S.74`.
  5. FOV: `fov = S.30`; if `f < S.28 && !S.81`: `fov = (S.30 - S.2C) * E((1.0f/S.28)*f) + S.2C`.
     `D = sqrt((dz*dz + dy*dy) + dx*dx)` with `d = Q - dest`; `y = 0`; if `D > S.68`: `x = (D - S.68)/(S.6C - S.68)` clamped
     (`x < 0 -> 0, else x > 1 -> 1`), `y = E(x) * S.70`. `fov2 = fov - y`. `roll = f * S.38`.
  6. Sphere lock: if `!S.5C` and `!TestSphereAgainstWorld(Q, 15.0f, nullptr, true, true, false, false, false, false)`: skip to 7.
     Else: (guard) if `!S.5C`: `sLockPos = Q; S.5C = 1; c = 100` else `c = sLockCount`; `sLockCount = c - 1`; if `c < 0`:
     `sFailed = 1`, return 0. `Q = (Q - sLockPos)*0.5f + sLockPos`.
  7. If `!S.40`: `pIgnoreEntity = ent; clear = !ProcessLineOfSight(&dest, &Q, &cp, &e, true, true, false x6)`;
     `pIgnoreEntity = nullptr`. Clear: `S.80 = 1`; if `!S.88 && S.64 < S.60/4` (C signed division): `S.84 = fov2; S.88 = 1;
     S.8C = now; S.90 = now + S.98`; then `c = S.64--`; if `c == 0`: `S.4C = dest; S.40 = 1`. Blocked: `S.64++`; if
     `S.64 > S.60`: `S.64 = S.60`.
     Else (locked): `dest = S.4C`; `c = S.44--`; if `c == 0`: `sFailed = 1`, return 0.
  8. If `!S.88` and `!(f < S.94)`: `S.84 = fov2; S.88 = 1; S.8C = now; S.90 = now + S.98`. If `S.88`:
     `u = ((float)now - (float)S.8C) / ((float)S.90 - (float)S.8C)` (fild of each int), clamped to [0,1];
     `fov2 = (S.2C - S.84) * E(u) + S.84`.
  9. If `IsTimeToExitThisDWCineyCamMode(20, &Q, &dest, f, false)`: `sFailed = 1`, return 0.
  10. `Finalise_DW_CineyCams(&Q, &dest, roll, fov2, S.3C, 1.0f)`; return 1.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:844); nothing to compare.
- Open questions: semantics/names of the settings block fields and of 0x50E180 / 0x50E090 (behaviour fully given by offsets).

## 0x51B120 CCam::Process_DW_CamManCam  (catalogue: specified/REA, gr-unreversed, port level 9)
- Convention / signature: `__thiscall bool (CCam* this = ECX, bool arg)`, `ret 4`, AL result (1 = produced). The argument is not
  read. Entry `nop; jmp 0x401279` (.HOODLUM `sub esp,0xFC; jmp 0x51B126`), pushes ebx, ebp, esi(=this, also saved at B+0x48),
  edi; B = ESP after the pushes.
- Callers: CCamera::IsItTimeForNewcam 0x51D770, CCamera::TryToStartNewCamMode 0x51E560, CCam::Process 0x526FC0.
- Callees: CCam::GetCoreDataForDWCineyCamMode 0x517130, CHandShaker::Reset 0x50D860, CWorld::FindObjectsInRange 0x564A20
  (cdecl(const CVector&, float r, bool 2D, int16* count, int16 max, CEntity** out, bool buildings, vehicles, peds, objects, dummies)),
  CPlaceable::AllocateMatrix 0x54F560 / CSimpleTransform::UpdateMatrix 0x54F1B0, IsLampPost 0x509A30 (cdecl(int32 model) -> AL),
  CEntity::GetColModel 0x535300, CPlaceable::GetMatrix 0x411990, CMatrix::operator*(CVector) 0x59C890, CVector::Normalise 0x59C910,
  CWorld::GetIsLineOfSightClear 0x56A490, CCam::IsTimeToExitThisDWCineyCamMode 0x517400, CWorld::ProcessLineOfSight 0x56BA00,
  helper 0x420800 (`a > b ? a : b`), CCam::Finalise_DW_CineyCams 0x50DD70.
- Locals (GetCoreData outputs): `ent` B+0x78, `veh` B+0x14, `dest` B+0x18, `src` B+0x24, `up` B+0x3C, `right` B+0x30, `fwd` B+0x50,
  `vel` B+0x60, `speed` B+0x4C, `angVel` B+0x6C, `angSpeed` B+0x5C (then `now`), colsphere B+0x8C; object list B+0xA0 (15 entries),
  count i16 B+0x10.
- Globals: TheCamera.m_bUseNearClipScript 0xB6F059 (W 0), `sFailed` u8 0xB6EC71, gDWCamMode 0x8CC488, gLastFrameProcessedDWCineyCam
  0x8CCB9C, sSceneStart 0x8CCBA0, s_DWCineyCamSceneEndTime 0x8CCBA4, u32 0x8CCBAC (10000), guard u32 0xB70078 (bit 0: nothing;
  bit 1: first call sets `sLosCount (0xB70074) = [0x8CCD58]` = 100), `sLampPos` CVector 0xB70068, CWorld::pIgnoreEntity 0xB7CD68.
  Data constants: 0x8CCD2C 0.2f, 0x8CCD30 8.0f, 0x8CCD34 70.0f, 0x8CCD38 0.1f, 0x8CCD3C 70.0f, 0x8CCD40 15.0f, 0x8CCD44 30.0f,
  0x8CCD48 1.0f, 0x8CCD4C 6.0f, 0x8CCD50 5.0f, 0x8CCD54 50.0f, 0x8CCD58 i32 100. `E(x)` as in Process_DW_HeliChaseCam.
- Behaviour:
  1. `m_bUseNearClipScript = 0`; target must be a non-null vehicle, else return 0 (without touching sFailed).
  2. GetCoreData; `now = m_snTimeInMilliseconds`; static guards (above).
  3. Start unless `gDWCamMode == 57 && gLastFrameProcessedDWCineyCam >= m_FrameCounter - 1`: `gDWCamMode = 57; sLosCount = 100
     ([0x8CCD58]); gHandShaker[0].Reset(); sFailed = 0; sSceneStart = now; sceneEnd = now + 10000`.
     `c = (fwd.x*50 + dest.x, fwd.y*50 + dest.y, dest.z + fwd.z*50)`; `FindObjectsInRange(c, 50.0f, true, &count, 15, list, false,
     false, false, true, true)`. `best = 10000.0f; lamp = nullptr`. For `i = 0 .. count-1` (i16 compare):
     - `e = list[i]`; require `(e->flags1C & 0x4) || (e->flags1C & 0x40000)`; `M(e)`; require `mat.up.z (+0x28) > 0.9f [0x858C20]`;
       require `IsLampPost(e->m_nModelIndex)`.
     - `d = sqrt(dy*dy + dx*dx)` with `d = pos(e) - dest` (2D); require `d < best && d > 5.0f`.
     - `L = e->GetColModel()->bbox.max`; `T = (*GetMatrix(e)) * L`; `T.z = T.z - colmodel.bbox.max.z`; `T.z = colmodel.bbox.min.z*0.5f + T.z`.
     - `q = T - dest`, normalise; `q.x = q.x + dest.x; q.y = q.y + dest.y; q.z = dest.z + q.z`.
     - Dead test: `fabs(0.0 (double 0x859EF8)) < 6.0f` is always true.
     - If `GetIsLineOfSightClear(T, q, true, false, false, false, false, true, true)`: `best = d; src = T; sLampPos = T; lamp = e`.
     If `!lamp`: `sFailed = 1`, return 0.
  4. Per frame (0x51B4E9): `f = (float)(int)(now - sSceneStart) / (int)(sceneEnd - sSceneStart)`. If `!sFailed`: `src = sLampPos;
     q = dest - sLampPos`, normalise; `src = q*1.0f + src` (`q.c*1.0f + src.c`). (If sFailed is set here, `src` keeps the GetCoreData
     value and the function continues.)
  5. `D = sqrt((dx*dx + dz*dz) + dy*dy) / 30.0f` with `d = dest - src`, clamped [0,1]; `fov = (15.0f - 70.0f) * E(D) + 70.0f`.
     If `f < 0.1f`: `x = clamp(f / 0.1f, 0, 1); fov = (fov - 70.0f) * E(x) + 70.0f`.
  6. If `IsTimeToExitThisDWCineyCamMode(21, &src, &dest, f, false)`: `sFailed = 1`, return 0.
  7. `pIgnoreEntity = ent; clear = !ProcessLineOfSight(&dest, &src, &cp, &e, true, true, false x6); pIgnoreEntity = nullptr`.
     Blocked: `c = sLosCount--`; if `c == 0`: `sFailed = 1`, return 0. Clear: `c = sLosCount++`; if `c > 100`: `sLosCount = 100`.
  8. Shake: `s = sqrt((vel.x*vel.x + vel.z*vel.z) + vel.y*vel.y) * 8.0f`; `a = (s > 0.2f) ? s : 0.2f`; if `a < 0`: `shake = 0`; else if
     `((s > 0.2f) ? s : 0.2f) > 1.0f`: `shake = 1.0f`; else `shake = helper(s, 0.2f)`.
  9. `near = 10.0f - (fov * 0.014285714f [0x85B328]) * 9.7f [0x863200]`; `Finalise_DW_CineyCams(&src, &dest, 0.0f, fov, near, shake)`;
     return 1.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:839); nothing to compare.
- Open questions: entity flag bits 0x4 / 0x40000 at +0x1C (gr names not verified here; behaviour is by bit).

## 0x51B850 CCam::Process_DW_BirdyCam  (catalogue: specified/REA, gr-unreversed, port level 9)
- Convention / signature: `__thiscall bool (CCam* this = ECX, bool arg)`, `ret 4`, AL result. The argument is not read. MSVC SEH
  frame (`push -1; push 0x83C87E; push fs:[0]`), `sub esp,0x8F0`, pushes ebx(=0 mostly), ebp, esi, edi(=this, also at B+0xA8);
  B = ESP after the pushes. The epilogue (0x51C1F1) restores fs:[0]. The catalogue's last instruction `jmp 0x51C1F1` (0x51C244) is
  the shared epilogue jump of the success path.
- Callers: CCamera::IsItTimeForNewcam 0x51D770, CCamera::TryToStartNewCamMode 0x51E560, CCam::Process 0x526FC0.
- Callees: CCam::GetCoreDataForDWCineyCamMode 0x517130, eh-vector-constructor helper 0x403D60 (cdecl(void* arr, size 0xC, count 2,
  ctor 0x404350); static CVector[2] construction), CHandShaker::Reset 0x50D860, CWorld::FindObjectsInRange 0x564A20 (2 calls),
  CPlaceable::AllocateMatrix 0x54F560 / CSimpleTransform::UpdateMatrix 0x54F1B0 (`M(e)`), IsLampPost 0x509A30, CEntity::GetColModel
  0x535300, CMatrix::operator*(CVector) 0x59C890, `rand` 0x821B1E, CVector::Normalise 0x59C910, CWorld::GetIsLineOfSightClear 0x56A490,
  CCam::IsTimeToExitThisDWCineyCamMode 0x517400, CWorld::ProcessLineOfSight 0x56BA00, CCam::Finalise_DW_CineyCams 0x50DD70.
- Locals: GetCoreData outputs `ent` B+0xAC, `veh` B+0x50, `dest` B+0x44, `src` B+0x74, `up` B+0x24, `right` B+0x90, `fwd` B+0x30,
  `vel` B+0x68, `speed` B+0x80, `angVel` B+0x84, `angSpeed` B+0x1C (then `now`), colsphere B+0xC0. Object lists: list1 B+0x100 (count
  i16 B+0x20), list2 B+0x300 (count i16 B+0x40); lamp lists lamps1 B+0x500 (count n1 in BP), lamps2 B+0x700 (count n2 at B+0x54).
- Globals: TheCamera.m_bUseNearClipScript 0xB6F059 (W 0), `sFailed` u8 0xB6EC72, gDWCamMode 0x8CC488, gLastFrameProcessedDWCineyCam,
  sSceneStart 0x8CCBA0, sceneEnd 0x8CCBA4, u32 0x8CCBB0 (5000), guard u32 0xB70098 (bit 0: construct `sLamps` CVector[2] at
  0xB70080 (0xB70080 and 0xB7008C) via 0x403D60; bit 1: `sLosCount (0xB7007C) = [0x8CCD74]` = 30), CWorld::pIgnoreEntity 0xB7CD68.
  Data constants: 0x8CCD5C 1.0f, 0x8CCD60 6.0f, 0x8CCD64 1.0f, 0x8CCD68 50.0f, 0x8CCD6C 2.0f, 0x8CCD70 50.0f, 0x8CCD74 i32 30.
- Lamp filter `ok(e)`: `(e->flags1C & 0x4) || (e->flags1C & 0x40000)`, then `M(e)->up.z > 0.9f [0x858C20]`, then `IsLampPost(model)`.
- Lamp top `top(e)`: `T = (*M(e)) * colmodel.bbox.max`; `h = colmodel.bbox.max.z - colmodel.bbox.min.z*0.5f`; `r = (float)rand() *
  (1/32767) [0x858C7C]` (unmasked); `T.z = T.z - ((h - 1.0f)*r + 1.0f)`.
- Behaviour:
  1. `m_bUseNearClipScript = 0`; target must be a non-null vehicle, else return 0. GetCoreData; `now`; static guards.
  2. Start unless `gDWCamMode == 58 && gLastFrameProcessedDWCineyCam >= m_FrameCounter - 1`: `gDWCamMode = 58; sceneEnd = now + 5000;
     gHandShaker[0].Reset(); sFailed = 0; sSceneStart = now; sLosCount = 30`.
     - `c1 = ((fwd.x*50)*2 + dest.x, dest.y + (fwd.y*50)*2, dest.z + (fwd.z*50)*2)`; `FindObjectsInRange(c1, 50.0f, true, &count1, 127,
       list1, false, false, false, true, true)`.
     - `c2 = (dest.x - (fwd.x*50)*1.0f, dest.y - (fwd.y*50)*1.0f, dest.z - (fwd.z*50)*1.0f)`; `FindObjectsInRange(c2, 50.0f, true, &count2,
       127, list2, ...)`.
     - lamps1 = entries of list1 passing `ok`, lamps2 = entries of list2 passing `ok` (in list order).
     - If `n1 <= 0`: fail. Pair search: `found = 0`; for `i = 0; i < n1 && !found; ++i`: `e1 = lamps1[i]`; if `ok(e1)`: `T1 = top(e1)`
       (one rand()); `q1 = T1 - dest`, normalise, `q1.x += dest.x; q1.y += dest.y; q1.z = dest.z + q1.z`; dead test `fabs(0.0) < 6.0f`
       (always true); for `j = i; j < n2 && !found; ++j` (NOTE: the inner index starts at the outer index i - original quirk):
       `e2 = lamps2[j]`; if `ok(e2)`: `T2 = top(e2)` (one rand()); if `GetIsLineOfSightClear(T1, T2, true, false, false, false, false, true,
       true)` and `GetIsLineOfSightClear(T2, q1, true, false, false, false, false, true, true)`: `src = T1; sLamps[0] = T1; sLamps[1] = T2;
       found = 1`. If not found: fail.
     Fail = `sFailed = 1`, return 0.
  3. Per frame (0x51BF9C): `f = (float)(int)(now - sSceneStart) / (int)(sceneEnd - sSceneStart)`. `P0 = sLamps[0]; P1 = sLamps[1];
     d = sLamps[1] - sLamps[0]`, normalise, `d *= 1.0f`; `P1 -= d`; `P0.c = (d.c + d.c) + P0.c`.
     If `!sFailed`: `s = sin((270 - f*180) * 0.017453292f); w = (1.0f + s)*0.5f`; `src.x = (P0.x - P1.x)*w + P1.x; src.y = (P0.y - P1.y)*w + P1.y;
     src.z = (P0.z - P1.z)*((s + 1.0f)*0.5f) + P1.z`. (If sFailed is set, src keeps the GetCoreData value.)
  4. If `IsTimeToExitThisDWCineyCamMode(22, &src, &dest, f, false)`: fail.
  5. `pIgnoreEntity = ent; clear = !ProcessLineOfSight(&dest, &src, &cp, &e, true, true, false x6); pIgnoreEntity = nullptr`. Blocked:
     `c = sLosCount--`; if `c == 0`: fail. Clear: `c = sLosCount++`; if `c > 30`: `sLosCount = 30`.
  6. `Finalise_DW_CineyCams(&src, &dest, 0.0f, 70.0f, 0.3f, 0.0f)`; return 1.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:834); nothing to compare.
- Open questions: none affecting behaviour (the j = i start is in the bytes at 0x51BDA2).

## 0x51C250 CCam::Process_DW_PlaneSpotterCam  (catalogue: specified/REA, gr-unreversed, port level 9)
- Convention / signature: `__thiscall bool (CCam* this = ECX, bool arg)`, `ret 4`, AL result; the argument is not read. Entry
  `nop; jmp 0x401DC0` (.HOODLUM `sub esp,0xDC; jmp 0x51C256`), pushes ebx(=0), ebp, esi(=now), edi(=this, also at B+0x38).
- Callers: CCamera::IsItTimeForNewcam 0x51D770, CCamera::TryToStartNewCamMode 0x51E560, CCam::Process 0x526FC0.
- Callees: CCam::GetCoreDataForDWCineyCamMode 0x517130, `rand` 0x821B1E, CWorld::ProcessLineOfSight 0x56BA00,
  CCam::IsTimeToExitThisDWCineyCamMode 0x517400, CCam::Finalise_DW_CineyCams 0x50DD70.
- Locals: GetCoreData outputs `ent` B+0x10 (kept in EBP), `veh` B+0x14, `dest` B+0x28, `src` B+0x1C, `up` B+0x7C, `right` B+0xA0,
  `fwd` B+0x88, `vel` B+0x70, `speed` B+0x34, `angVel` B+0x94, `angSpeed` B+0x18, colsphere B+0xAC; colpoint B+0x44 / B+0xC0,
  `f` B+0x10, `fov` B+0x14 (slots re-used).
- Globals: m_bUseNearClipScript 0xB6F059 (W 0), `sFailed` u8 0xB6EC73, gDWCamMode 0x8CC488, gLastFrameProcessedDWCineyCam, sSceneStart
  0x8CCBA0, sceneEnd 0x8CCBA4, u32 0x8CCBB4 (10000), guard u32 0xB700B4 (bit 0 no effect; bit 1: `sFovStart` f32 0xB700A0 =
  [0x8CCD80]; bit 2: first LOS-count use), `sPos` CVector 0xB700A8, `sZoom` u8 0xB700A4, `sLosCount` i32 0xB7009C,
  CWorld::pIgnoreEntity. Data constants: 0x8CCD78 i32 100, 0x8CCD7C 0.1f, 0x8CCD80 70.0f, 0x8CCD84 12.0f, 0x8CCD88 100.0f,
  0x8CCD8C 100.0f, 0x8CCD90 100.0f, 0x8CCD94 i32 8. `E(x)` as in Process_DW_HeliChaseCam.
- Behaviour:
  1. `m_bUseNearClipScript = 0`; non-null vehicle target required, else return 0. GetCoreData; `now`.
  2. Start unless `gDWCamMode == 59 && gLastFrameProcessedDWCineyCam >= m_FrameCounter - 1`: `gDWCamMode = 59; sFailed = 0;
     sSceneStart = now; sceneEnd = now + 10000`. For `i = 0; i < 8 ([0x8CCD94])`:
     - `src = (dest.x, dest.y, dest.z - 100.0f)`; `h = 100.0f * 0.5f`; `src.x = ((100.0f - h) * r1 + h) + dest.x`;
       `src.y = ((100.0f - h) * r2 + h) + dest.y` with `r = (float)rand() * (1/32767)` (r1 drawn first).
     - `pIgnoreEntity = ent; hit = ProcessLineOfSight(&dest, &src, &cp, &e, true, true, false x6); pIgnoreEntity = nullptr`.
     - If `hit`: `sPos = cp.point; sPos.z = cp.point.z + 2.0f`; stop (success). Else next try.
     If no try hit: `sFailed = 1`, return 0. (If the try count is <= 0 the loop is skipped and the decision uses the low byte of the
     count; with the data value 8 this cannot happen.) On success: `sZoom = (rand() < 0x3FFF)`.
  3. Per frame (0x51C496): `f = (float)(int)(now - sSceneStart) / (int)(sceneEnd - sSceneStart)`; `src = sPos; fov = 70.0f`.
     If `sqrt(dy*dy + dx*dx) < 5.0f` (d = sPos - dest, 2D): fail.
  4. If `sZoom`: `D = clamp(sqrt((dz*dz + dy*dy) + dx*dx) / 100.0f, 0, 1)` (d = dest - sPos); `fov = (12.0f - 70.0f)*E(D) + 70.0f`;
     if `f < 0.1f`: (guard bit 1: `sFovStart = 70.0f` once) `x = clamp(f/0.1f, 0, 1); fov = (fov - sFovStart)*E(x) + sFovStart`.
  5. `pIgnoreEntity = ent; clear = !ProcessLineOfSight(&dest, &src, &cp, &e, true, true, false x6); pIgnoreEntity = nullptr`.
     `c = (guard bit 2 not yet set) ? (set bit 2, 100) : sLosCount`. Blocked: `sLosCount = c - 1`; if `c == 0`: fail. Clear:
     `sLosCount = c + 1`; if `c > 100`: `sLosCount = 100`.
  6. If `IsTimeToExitThisDWCineyCamMode(23, &src, &dest, f, false)`: fail.
  7. `Finalise_DW_CineyCams(&src, &dest, 0.0f, fov, 10.0f - (fov*0.014285714f)*9.7f, 1.0f)`; return 1.
  (fail = `sFailed = 1`, return 0.)
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:864); nothing to compare.
- Open questions: none.

## 0x51C760 CCam::Process_DW_PlaneCam1  (catalogue: specified/REA, gr-unreversed, port level 10)
- Convention / signature: `__thiscall bool (CCam* this = ECX, bool arg)`, `ret 4`, AL result; the argument is not read. Frame
  `sub esp,0xEC` + ebx(=0), ebp(=this), esi(=now), edi(=ent); B = ESP after the pushes.
- Callers: CCamera::IsItTimeForNewcam 0x51D770, CCamera::TryToStartNewCamMode 0x51E560, CCam::Process 0x526FC0.
- Callees: GetCoreDataForDWCineyCamMode 0x517130, CCamera::GetRoughDistanceToGround 0x516B00 (thiscall on TheCamera -> ST0),
  CWorld::ProcessLineOfSight 0x56BA00, `rand` 0x821B1E, CVector::Normalise 0x59C910, IsTimeToExitThisDWCineyCamMode 0x517400,
  Finalise_DW_CineyCams 0x50DD70.
- Locals: GetCoreData outputs `ent` B+0x10 (later `f`), `veh` B+0x70, `dest` B+0x54, `src` B+0x14, `up` B+0x48, `right` B+0x20,
  `fwd` B+0x2C, `vel` B+0x84, `speed` B+0x38, `angVel` B+0x78, `angSpeed` B+0x60 (then `now`), colsphere B+0x90.
- Globals: m_bUseNearClipScript 0xB6F059 (W 0), `sFailed` u8 0xB6EC76, gDWCamMode, gLastFrameProcessedDWCineyCam, sSceneStart, sceneEnd,
  u32 0x8CCBC0 (12000), `sSignA` f32 0xB700C0, `sLosCount` i32 0xB700B8 with guard bit 0 of 0xB700BC, CWorld::pIgnoreEntity.
  Data: 0x8CCD98 i32 100, 0x8CCD9C 0.5f, 0x8CCDA0 i32 4, 0x8CCDA4 -150.0f, 0x8CCDA8 15.0f, 0x8CCDAC 30.0f, 0x8CCDB0 10.0f,
  `sSignC` f32 0x8CCDB4 (initial 1.0f, writable, only ever set to -1.0f), 0x8CCDB8 30.0f, 0x8CCDBC 80.0f.
- Behaviour:
  1. `m_bUseNearClipScript = 0`; non-null vehicle target required, else return 0 (sFailed untouched). GetCoreData; `now`.
  2. If `dest.z < 80.0f`: fail.
  3. If `gDWCamMode == 62 && gLastFrameProcessedDWCineyCam >= m_FrameCounter - 1` (continuing): if
     `TheCamera.GetRoughDistanceToGround() < 30.0f`: fail; else go to step 5.
  4. Start: `gDWCamMode = 62; sceneEnd = now + 12000; sFailed = 0; sSceneStart = now; pIgnoreEntity = ent;
     hit = ProcessLineOfSight(&dest, &src, &cp, &e, true, true, false x6); pIgnoreEntity = nullptr`; if `hit`: fail.
     `sSignA = 1.0f; if (rand() < 0x3FFF) sSignA *= -1.0f; if (rand() < 0x3FFF) sSignC = -1.0f`.
  5. Per frame (0x51C8EA): `f = (float)(int)(now - sSceneStart) / (int)(sceneEnd - sSceneStart)`. `V1 = fwd, V2 = right` (copies),
     normalise both; `V2 *= sSignC`.
     `P = (V1.x*10.0f + dest.x, dest.y + V1.y*10.0f, V1.z*10.0f + dest.z)`; `src = P`.
     `src.x = V2.x*30.0f + P.x; sy = V2.y*30.0f + src.y; sz = src.z + V2.z*30.0f`;
     `src.x = up.x*15.0f + src.x; src.y = up.y*15.0f + sy; z1 = up.z*15.0f + sz`;
     `t = f - 0.5f`; `src.x = ((up.x*-150.0f)*t)*sSignA + src.x; src.y = ((up.y*-150.0f)*t)*sSignA + src.y; z2 = ((up.z*-150.0f)*t)*sSignA + z1`;
     `s = sin(((((float)now - (float)sSceneStart) / ((float)sceneEnd - (float)sSceneStart)) * (float)4) * 360.0f * 0.017453292f)`;
     `src.x = (up.x*0.5f)*s + src.x; src.y = (up.y*0.5f)*s + src.y; src.z = (up.z*0.5f)*s + z2`.
  6. `pIgnoreEntity = ent; clear = !ProcessLineOfSight(&dest, &src, &cp, &e, true, true, false x6); pIgnoreEntity = nullptr`;
     `c = first use ? 100 : sLosCount`; blocked: `sLosCount = c - 1`, fail if `c == 0`; clear: `sLosCount = c + 1`, if `c > 100` set 100.
  7. If `IsTimeToExitThisDWCineyCamMode(26, &src, &dest, f, false)`: fail.
  8. `Finalise_DW_CineyCams(&src, &dest, 0.0f, 70.0f, 5.0f, 1.0f)`; return 1. (fail = `sFailed = 1`, return 0.)
- Oddity: the ground-distance test only runs on continuing frames, not on the start frame; `sSignC` is never reset to +1.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:849); nothing to compare.
- Open questions: none.

## 0x51CC30 CCam::Process_DW_PlaneCam2  (catalogue: specified/REA, gr-unreversed, port level 9)
- Convention / signature: `__thiscall bool (CCam* this = ECX, bool arg)`, `ret 4`, AL result; the argument is not read. Entry
  `nop; jmp 0x40801D` (.HOODLUM `sub esp,0xEC; jmp 0x51CC36`), pushes ebx(=0), ebp(=this), esi, edi(=ent).
- Callers: CCamera::IsItTimeForNewcam 0x51D770, CCamera::TryToStartNewCamMode 0x51E560, CCam::Process 0x526FC0.
- Callees: GetCoreDataForDWCineyCamMode 0x517130, CWorld::ProcessLineOfSight 0x56BA00, `rand` 0x821B1E, CVector::Normalise 0x59C910,
  IsTimeToExitThisDWCineyCamMode 0x517400, Finalise_DW_CineyCams 0x50DD70.
- Locals: GetCoreData outputs `ent` B+0x10, `veh` B+0x6C, `dest` B+0x50, `src` B+0x14, `up` B+0x60, `right` B+0x38, `fwd` B+0x44,
  `vel` B+0x78, `speed` B+0x70, `angVel` B+0x84, `angSpeed` B+0x5C (then `now`), colsphere B+0x90.
- Globals: m_bUseNearClipScript 0xB6F059 (W 0), `sFailed` u8 0xB6EC77, gDWCamMode 0x8CC488, gLastFrameProcessedDWCineyCam, sSceneStart
  0x8CCBA0, sceneEnd 0x8CCBA4, u32 0x8CCBC4 (7000), statics `sSignA` f32 0xB700D0 (written, never read here), `sSignB` f32 0xB700CC,
  `sLosCount` i32 0xB700C4 with guard bit 0 of 0xB700C8, CWorld::pIgnoreEntity. Data: 0x8CCDC0 i32 100, 0x8CCDC4 0.5f, 0x8CCDC8 i32 4,
  0x8CCDCC 5.0f, 0x8CCDD0 30.0f, 0x8CCDD4 30.0f, `sSignC` f32 0x8CCDD8 (initial 1.0f, writable), 0x8CCDDC 80.0f.
- Behaviour:
  1. `m_bUseNearClipScript = 0`; non-null vehicle target required, else return 0. GetCoreData; `now`.
  2. If `dest.z < 80.0f`: fail.
  3. Start unless `gDWCamMode == 63 && gLastFrameProcessedDWCineyCam >= m_FrameCounter - 1`: `gDWCamMode = 63; sceneEnd = now + 7000;
     sFailed = 0; sSceneStart = now; pIgnoreEntity = ent`; `hit = ProcessLineOfSight(&dest, &src, &cp, &e, true, true, false x6)`
     (src = GetCoreData's last-position output); `pIgnoreEntity = nullptr`; if `hit`: fail. `sSignA = 1.0f; if (rand() < 0x3FFF) sSignA *= -1.0f;
     sSignB = 1.0f; if (rand() < 0x3FFF) sSignB *= -1.0f; if (rand() < 0x3FFF) sSignC = -1.0f` (sSignC is never set back to +1: once
     flipped it stays -1 for the rest of the session - original behaviour).
  4. Per frame (0x51CDBA): `f = (float)(int)(now - sSceneStart) / (int)(sceneEnd - sSceneStart)`. `A = fwd`, `Bv = right` (copies),
     normalise both. `g = 1.0f - f; Bv *= g`; `h = 1.0f - (f + f); A *= h; A *= sSignB; Bv *= sSignC`.
     `P1.x = A.x*30.0f [0x8CCDD4] + dest.x; P1.y = dest.y + A.y*30.0f; P1.z = A.z*30.0f + dest.z`; `src = P1`.
     `src.x = Bv.x*30.0f [0x8CCDD0] + P1.x; sy = Bv.y*30.0f + src.y; sz = src.z + Bv.z*30.0f`;
     `src.x = up.x*5.0f + src.x; src.y = up.y*5.0f + sy; z1 = up.z*5.0f + sz` (z1 kept on the x87 stack).
     `s = sin(((((float)now - (float)sSceneStart) / ((float)sceneEnd - (float)sSceneStart)) * (float)4 [fild 0x8CCDC8]) * 360.0f *
     0.017453292f)` (each int via fild). `src.x = (up.x*0.5f)*s + src.x; src.y = (up.y*0.5f)*s + src.y; src.z = (up.z*0.5f)*s + z1`.
  5. `pIgnoreEntity = ent; clear = !ProcessLineOfSight(&dest, &src, &cp, &e, true, true, false x6); pIgnoreEntity = nullptr`;
     `c = first use ? 100 : sLosCount`; blocked: `sLosCount = c - 1`, fail if `c == 0`; clear: `sLosCount = c + 1`, if `c > 100` set 100.
  6. If `IsTimeToExitThisDWCineyCamMode(27, &src, &dest, f, false)`: fail.
  7. `Finalise_DW_CineyCams(&src, &dest, 0.0f, 70.0f, 5.0f, 1.0f)`; return 1. (fail = `sFailed = 1`, return 0.)
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:854); nothing to compare.
- Open questions: none.

## 0x51D100 CCam::Process_DW_PlaneCam3  (catalogue: specified/REA, gr-unreversed, port level 9)
- Convention / signature: `__thiscall bool (CCam* this = ECX, bool arg)`, `ret 4`, AL result; the argument is not read. Frame
  `sub esp,0xD4` + ebx(=0), ebp(=this), esi(=now), edi(=ent); B = ESP after the pushes.
- Callers: CCamera::IsItTimeForNewcam 0x51D770, CCamera::TryToStartNewCamMode 0x51E560, CCam::Process 0x526FC0.
- Callees: GetCoreDataForDWCineyCamMode 0x517130, CWorld::ProcessLineOfSight 0x56BA00, IsTimeToExitThisDWCineyCamMode 0x517400,
  Finalise_DW_CineyCams 0x50DD70.
- Locals: GetCoreData outputs `ent` B+0x10 (later `f`), `veh` B+0x38, `dest` B+0x2C, `src` B+0x14, `up` B+0x48, `right` B+0x6C,
  `fwd` B+0x20, `vel` B+0x60, `speed` B+0x58, `angVel` B+0x3C, `angSpeed` B+0x54, colsphere B+0x78.
- Globals: m_bUseNearClipScript 0xB6F059 (W 0), `sFailed` u8 0xB6EC78, gDWCamMode, gLastFrameProcessedDWCineyCam, sSceneStart, sceneEnd,
  u32 0x8CCBC8 (5000), CModelInfo::ms_modelInfoPtrs 0xA9B0C8, `sLosCount` i32 0xB700D4 with guard bit 0 of 0xB700D8,
  CWorld::pIgnoreEntity. Data: 0x8CCDE0 i32 100, 0x8CCDE4 5.0f, `sLen` f32 0x8CCDE8 (initial 15.0f, OVERWRITTEN every frame),
  0x8CCDEC 80.0f.
- Behaviour:
  1. `m_bUseNearClipScript = 0`; non-null vehicle target required, else return 0. GetCoreData; `now`.
  2. If `dest.z < 80.0f`: fail.
  3. Start unless `gDWCamMode == 64 && gLastFrameProcessedDWCineyCam >= m_FrameCounter - 1`: `gDWCamMode = 64; sceneEnd = now + 5000;
     sFailed = 0; sSceneStart = now; pIgnoreEntity = ent; hit = ProcessLineOfSight(&dest, &src, &cp, &e, true, true, false x6);
     pIgnoreEntity = nullptr`; if `hit`: fail.
  4. Per frame: `f = (float)(int)(now - sSceneStart) / (int)(sceneEnd - sSceneStart)`. `cm = ms_modelInfoPtrs[ent->m_nModelIndex]->m_pColModel`;
     `sLen = ((cm.bbox.max.y - cm.bbox.min.y) * 0.5f) + same` (`fadd st0,st0`). `P = (fwd.x*sLen + dest.x, dest.y + fwd.y*sLen,
     fwd.z*sLen + dest.z)`; `src = (up.x*5.0f + P.x, P.y + up.y*5.0f, P.z + up.z*5.0f)`.
  5. `pIgnoreEntity = ent; clear = !ProcessLineOfSight(&dest, &src, &cp, &e, true, true, false x6); pIgnoreEntity = nullptr`;
     `c = first use ? 100 : sLosCount`; blocked: `sLosCount = c - 1`, fail if `c == 0`; clear: `sLosCount = c + 1`, if `c > 100` set 100.
  6. If `IsTimeToExitThisDWCineyCamMode(28, &src, &dest, f, false)`: fail.
  7. `Finalise_DW_CineyCams(&src, &dest, 0.0f, 70.0f, 5.0f, 1.0f)`; return 1. (fail = `sFailed = 1`, return 0.)
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:859); nothing to compare.
- Open questions: none.

## 0x520690 CCam::LookBehind  (catalogue: specified/REA, gr-unreversed, port level 11)
- Convention / signature: `__thiscall bool (CCam* this = ECX)`, plain `ret`. Returns AL: 0 when the mode/target is not handled (0x520705),
  else 1 (0x520E32). gr declares `void`. Frame `sub esp,0x3C` + ebx, edi(=this); then ebp(=&front), esi(=&source). B1 = ESP after the
  first two pushes (`carMode` u8 B1+9, `fp` u8 B1+0xA, `isPed` u8 B1+0xB, `pos` B1+0x14..0x1C, `dist`/tables B1+0x0C/0x10, `L2` B1+0x2C).
- Callers: CCam::Process 0x526FC0.
- Callees: `M(e)` pair (AllocateMatrix/UpdateMatrix), CCamera::CameraVehicleModeSpecialCases 0x50CDE0, CCamera::CameraColDetAndReact 0x520190,
  CCam::GetVectorsReadyForRW 0x509CE0 (4), CCamera::ImproveNearClip 0x516B20 (thiscall(CVehicle*, CPed*, CVector* src, CVector* tgt)),
  RwCameraSetNearClipPlane 0x7EE1D0, CVector::Normalise 0x59C910, CVehicle::GetVehicleAppearance 0x6D1080, CPedIntelligence::GetTaskSwim
  0x601070, CPedIntelligence::GetTaskHold 0x600FF0 (thiscall(bool)), CCamera::HandleCameraMotionForDucking 0x50CFA0
  (thiscall(CPed*, CVector* src, CVector* tgt, bool)).
- Globals: CVector 0xB6F018 (target stored by Process_FollowCar_SA), TheCamera.m_fPedZoomSmoothed 0xB6F0FC, m_nPedZoom i32 0xB6F0F0,
  m_nExtraEntitiesCount 0xB6FC70 and the array at 0xB6FC74, CCollision::bCamCollideWithVehicles 0x8A5B14 / Objects 0x8A5B15 / Peds 0x8A5B17
  (all set to 1), CWorld::pIgnoreEntity, Scene.m_pRwCamera 0xC1703C. Tables (f32[3], indexed by m_nPedZoom-1): 0x8CCE1C {0,1,1},
  0x8CCE28 {-1,-1,-1}, 0x8CCE34 {0.6,0.6,0.6}, 0x8CCE40 {0.6,0,0}; 0x8CCE4C 2.0f.
- Behaviour (`t` = m_pCamTargetEntity):
  1. `carMode = (m_nMode == 18 || 22 || 3) && t is a vehicle`; `isPed = t is a ped`; `fp = m_nMode == 16 && t is a vehicle`. If none: return 0.
  2. `pos = pos(t)`; `front = pos(t) - source`.
  3. carMode (0x52078E): `L = [0xB6F018]`; `m_bLookingBehind (+7) = 1`; `dist = (m_nMode == 18) ? m_fCaMaxDistance : 15.5f`;
     `source = M(t)->fwd; source.z = source.z + 0.2f; source = L + source*dist` (`L.c + dist*source.c`); `L2 = L`;
     `pIgnoreEntity = t; m_nExtraEntitiesCount = 0; CameraVehicleModeSpecialCases(t); CameraColDetAndReact(&source, &L2)`;
     `front = pos(t) - source; GetVectorsReadyForRW(); ImproveNearClip(t, nullptr, &source, &L2); pIgnoreEntity = nullptr`.
  4. fp (0x52091D): `m_bLookingBehind = 1; RwCameraSetNearClipPlane(cam, 0.05f); front = M(t)->fwd`, normalise; if base type == 5:
     `source.z = source.z - 0.5f`. Appearance 2 (bike): `source += front*2.3f [0x858F54]; front = -front; GetVectorsReadyForRW()`.
     Appearance 3 (heli): `front = M(t)->up * -1.0f; up = M(t)->fwd; source += front*0.25f [0x858C84]`. Other: `source += front*0.25f;
     front = -front`.
  5. isPed (0x520B6C): `dir = (-cos(beta), -sin(beta), 0)` (stored in `source`); `source.z = (0 - ((V.z*dir.z + V.y*dir.y) + V.x*dir.x)) + 0.3f`
     with V = ped field CVector +0x578 (gr `field_578`); `dir.Normalise()`; `dist = 2.0f + m_fPedZoomSmoothed; if (0.6f > dist) dist = 0.6f`;
     `source = dir*dist + pos` (`dist*dir.c + pos.c`). `z = m_nPedZoom - 1`; `A = tbl40[z], Bz = tbl34[z]`; if `GetTaskSwim()`:
     `source = (pos - source)*tbl28[z] + pos` (per component, products first) and `source.z = tbl1C[z] + source.z`. `source.z = A + source.z;
     pos.z = pos.z + Bz`. `TheCamera.HandleCameraMotionForDucking(ped, &source, &pos, false)`. `m_nExtraEntitiesCount = 0`; if
     `ped->m_pIntelligence->GetTaskHold(false)` has a held entity (+8): append it to the extra-entities list. Set the three
     CCollision::bCamCollideWith* flags to 1; `CameraColDetAndReact(&source, &pos)`; `front = pos - source`; `GetVectorsReadyForRW()`;
     `ImproveNearClip(nullptr, ped, &source, &pos)`; if `m_nPedZoom == 1 && rwcam->nearPlane > 0.05f`: `RwCameraSetNearClipPlane(cam, 0.05f)`.
  6. `GetVectorsReadyForRW()`; return 1.
- gta-reversed: `void LookBehind()` with `NOTSA_UNREACHABLE()` body (Cam.cpp:419). The original returns a bool in AL.
- Open questions: none (the ped field +0x578 is unnamed in gr).

## 0x520E40 CCam::LookRight  (catalogue: specified/REA, gr-unreversed, port level 10)
- Convention / signature: `__thiscall bool (CCam* this = ECX, bool bLookRight [esp+4])`, `ret 4`. Returns AL: 1 on every path that
  does something, 0 when the mode/target combination is not handled (0x520ED4, AL is 0 there because it was just tested). gr declares
  `void`. A .HOODLUM stub at 0x4012B0 (`mov eax,[esi+0x21C]; jmp 0x520E65`) is part of the body. Frame `sub esp,0x34` + ebx, ebp, esi(=this),
  edi; B = ESP after the pushes; `pos` B+0x20..0x28, `sign` B+0x1C, `dist`/colmodel B+0x14, `L` B+0x38.
- Callers: CCam::Process 0x526FC0 (2 sites).
- Callees: CPlaceable::AllocateMatrix/CSimpleTransform::UpdateMatrix (`M(e)`), unnamed 0x509CA0 (thiscall(float* out) -> AL: if the target
  has boat handling (`cHandlingDataMgr::GetBoatPointer(modelInfo->handlingId (+0x4A))`), `*out = boatHandling+0x20`, return 1),
  CCullZones::Cam1stPersonForPlayer 0x72DD40, CVector::Normalise 0x59C910, CGeneral::GetATanOfXY 0x53CC70, CEntity::GetColModel 0x535300,
  CCamera::CameraVehicleModeSpecialCases 0x50CDE0, CCamera::CameraColDetAndReact 0x520190, RwCameraSetNearClipPlane 0x7EE1D0,
  CPed::SetPedPositionInCar 0x5DF910, CEntity::UpdateRwMatrix 0x446F90, UpdateRwFrame 0x532B00, UpdateRpHAnim 0x532B20,
  CPed::GetBonePosition 0x5E4280, CPlaceable::GetMatrix 0x411990, vector scaler 0x40FE90 (out = s*v), CVector::operator+= 0x411A00
  (thiscall(const CVector*): `this.c = other.c + this.c`), CVector::CrossProduct-out 0x59C730, CVehicle::GetVehicleAppearance 0x6D1080,
  CCam::GetVectorsReadyForRW 0x509CE0.
- Globals: CWorld::pIgnoreEntity 0xB7CD68, TheCamera.m_nExtraEntitiesCount 0xB6FC70, Scene.m_pRwCamera 0xC1703C. Data: 0x8CCE50 0.05f,
  0x8CC494 0.2f, 0x8CC498 0.7f, 0x8CC49C 0.3f; 0x858F14 1.1f, 0x858F08 1.2f, 0x858F9C 0.35f, 0x863230 1.45f, 0x858B1C 0.1f.
- Behaviour (`t` = m_pCamTargetEntity):
  1. `carMode = (m_nMode == 18 || m_nMode == 22 || m_nMode == 3) && t is a vehicle`; `isPed = t is a ped`; `fp = (m_nMode == 16 && t is
     a vehicle)`. `sign = 1.0f`; if `!bLookRight`: `m_bLookingLeft (+8) = 1; sign = -1.0f`; else `m_bLookingRight (+9) = 1`.
  2. If `!carMode`: `fp` -> step 4; else `isPed` -> return 1; else return 0.
  3. Car (0x520EDE): `pos = pos(t)`; `dist = (m_nMode == 18) ? m_fCaMaxDistance : 9.0f`; if `m_nMode == 22`: `h = 0`; if `F509CA0(&h) &&
     !CCullZones::Cam1stPersonForPlayer()`: `source.z = pos.z + h`. `fw = M(t)->fwd`, normalise; `a = GetATanOfXY(fw.x, fw.y) + sign*(PI/2)`;
     `source.x = cos(a)*dist + pos.x; source.y = sin(a)*dist + pos.y`. `cm = t->GetColModel()`; `L = pos; sz = source.z` (saved);
     `pIgnoreEntity = t; m_nExtraEntitiesCount = 0; TheCamera.CameraVehicleModeSpecialCases(t); TheCamera.CameraColDetAndReact(&source, &L);
     pIgnoreEntity = nullptr`.
     `p2 = pos(t)`; `p2.z = (bLookRight ? M(t)->right.z * cm.bbox.min.x : cm.bbox.max.x * M(t)->right.z) + p2.z`;
     `h = cm.bbox.max.z * M(t)->up.z + p2.z`; `m = (h < fudge.z) ? fudge.z : h` (fudge = m_vecTargetCoorsForFudgeInter, +0x150 = z);
     `v = (sz < m + 0.1f) ? sz : m + 0.1f`; `source.z = (v > source.z) ? v : source.z`.
     `front = pos(t) - source; front.z = front.z + 1.1f`; if `m_nMode == 22`: `front.z = front.z + 1.2f`. `GetVectorsReadyForRW()`; return 1.
  4. First person in vehicle (0x5211F1): set the look flag again; `RwCameraSetNearClipPlane(cam, 0.05f)`. If base type == 5 (boat):
     if `t->driver`: `b = 0; driver->SetPedPositionInCar(); driver->UpdateRwMatrix(); ->UpdateRwFrame(); ->UpdateRpHAnim();
     driver->GetBonePosition(&b, 4, true)`; `b += 0x40FE90(bLookRight ? 0.7f : 0.3f, &GetMatrix(t)->right)` (via operator+=);
     `source = (0.2f*up.x + b.x, 0.2f*up.y + b.y, 0.2f*up.z + b.z)` (up = GetMatrix(t) +0x20). Without driver: `source.z = source.z - 0.5f`.
     If base type != 9: `source -= M(t)->right * 0.35f` (`src.c - 0.35f*right.c`).
     `up = M(t)->up`, normalise; `front = M(t)->fwd`, normalise; `front = bLookRight ? cross(front, up) : cross(up, front)`, normalise.
     If appearance == 2 (bike): `source -= front * 1.45f`. Return 1.
- gta-reversed: `void LookRight(bool)` with `NOTSA_UNREACHABLE()` body (Cam.cpp:424). The original returns a bool in AL.
- Open questions: none.

## 0x521500 CCam::Process_AimWeapon  (catalogue: specified/REA, gr-unreversed, port level 11)
- Convention / signature: `__thiscall void (CCam* this = ECX, const CVector& target [B+0x6C], float, float, float)`, `ret 0x10`.
  Frame `sub esp,0x58` + esi(=this), ebp, edi(=ped), ebx(=0); B = ESP after the four pushes (ebx is popped at 0x522B75, after which
  `[esp+X]` = B+X-4). The target-argument slot B+0x6C is re-used as scratch (`sideOff`, `maxStep2`, pitch, `p`).
  Locals: `wi` B+0x1C (CWeaponInfo*), `aimType` i32 B+0x18 (row index into table `AT`), `fovT` B+0x14, `yawOff` B+0x10, `pitchOff` B+0x20,
  `T` B+0x38..0x40, `dz` B+0x24.
- Callers: CCam::Process 0x526FC0.
- Callees (main): CPed::IsPlayer 0x5DF8F0, CPedIntelligence::GetTaskUseGun 0x600F70 / GetTaskJetPack 0x601110 / GetTaskFighting 0x600F30,
  CWeaponInfo::GetWeaponInfo 0x743C60 (cdecl(type, skill=1)), CWeapon::IsTypeMelee 0x73B1C0, vector add 0x40FE30 (cdecl(out, a, b): out = a + b),
  VectorSub 0x40FE60, vector scale 0x40FE90 (out = s*v), CVector::operator*=(float) 0x40FEF0, CVector::CrossProduct-out 0x59C730,
  CVector::Magnitude 0x4082C0, helper 0x420800 (`a > b ? a : b`), CWorld::GetIsLineOfSightClear 0x56A490, _CIpow 0x822130, _CIasin 0x821E70,
  _CIacos 0x822380 (verified: `fpatan` of sqrt((1+x)(1-x)) over x), CCamera::ResetDuckingSystem 0x50CEF0, CVector::DotProduct 0x40FDB0,
  CEntity::UpdateRpHAnim 0x532B20, CPedClothesDesc::HasVisibleNewHairCut 0x5A7970, CPed::GetBonePosition 0x5E4280, CPad::GetPad 0x53FB70,
  CPad::AimWeaponLeftRight 0x541040 / AimWeaponUpDown 0x5410C0 (thiscall(CPed*)), CPad::GetEnterTargeting 0x5406B0, CPad::GetWeapon 0x540180,
  `_ftol2`, CCam::ClipBeta 0x509C50, CCamera::HandleCameraMotionForDuckingDuringAim 0x50D090, CCamera::SetColVarsAimWeapon 0x50CBF0 (cdecl(int)),
  CCamera::CameraGenericModeSpecialCases 0x50CD30, CameraPedAimModeSpecialCases 0x50CDA0, CameraColDetAndReact 0x520190, ImproveNearClip 0x516B20,
  CCam::GetVectorsReadyForRW 0x509CE0, CPlaceable::SetHeading 0x43E0C0, CEntity::UpdateRwMatrix 0x446F90, CCamera::Find3rdPersonQuickAimPitch 0x50AD40.
- Table `AT[aimType]` = 7 f32 at 0x8CC4C0 + aimType*0x1C (writable .data): +0 baseDist, +4 distScale, +8 alphaDistMul, +0xC alphaDefault,
  +0x10 zOffset, +0x14 alphaMax, +0x18 alphaMin (used negated). Values: 0 {1, 1.6, 1, -0.12, 0, 0.785398, 1.55334}; 1 {3.5, 0.7, 1, -0.16, 0.2,
  0.610865, 1.22173}; 2 {6, 0.7, 1, -0.16, 0.4, 0.610865, 1.22173}; 3 {2.5, 0.7, 1, -0.12, 0.15, 0.785398, 0.794125}.
- Statics: guard 0xB70110 (bit0: `sLock` CVector 0xB70104 = 0; bit1: `sA` f32 0xB70100 = 3.0f [0x862F20]; bit2: `sB` f32 0xB700FC = 20.0f [0x862F24]),
  `sC` f32 0xB700F4, `sLosTimer` f32 0xB700F8, `sAutoAim` u8 0xB6EC44, `sIdleMs` i32 0xB6EC48, `sTargetMs` u32 0xB6EC4C, `sHeading` f32 0x8CC530
  (initial -1001.0), i32 0x8CC534 (5000), i32 0x8CC538 (2000), u8 0x8CCE64 (initial 1). Other data: 0x8CC39C 0.9f, 0x8CC4A0 0.007f, 0x8CC4A4 0.1f,
  0x8CC4A8 0.1f, 0x8CC4AC 0.25f, 0x8CC4B0 0.17453294f, 0x8CC4B4 50.0f, 0x8CC4B8 35.0f, 0x8CCE54 500.0f, 0x8CCE58 0.5f, 0x8CCE5C 0.8f, 0x8CCE60 0.75f.
- Behaviour (block structure; formulas give the exact x87 order):
  1. Guard bit0 init. If the target entity is not a ped or `!IsPlayer()`: return (without clearing m_bResetStatics).
  2. `wi = GetTaskUseGun() ? task->+0x30 : GetWeaponInfo(weapons[slot].type, 1)`. `aimType`: in vehicle (`flags46C & 0x100`): `m_pVehicle &&
     (base type 9 || sub-type 2) ? 1 : 2`; else jetpack task -> 1; else melee weapon -> 3; else 0. `fovT = 70`; weapon type 30/31 -> 50.0f,
     33 -> 35.0f.
  3. FOV: if `!TheCamera.m_bTransitionState (0xB6F080)`: if `m_bResetStatics && type != 33`: `FOV = fovT`; else `s = ts*1.0f [0x862F1C];
     v = s + FOV`; `if (fovT > v) FOV = v; else { v2 = FOV - s; FOV = (fovT < v2) ? v2 : fovT; }`.
  4. Offsets. Melee fire type (`wi->m_nWeaponFire == 0`): guards bit1/bit2; `a = 3.0f, b = 20.0f, c = 0`; if `GetTaskFighting() && ped+0x534 < 4 &&
     ped->m_pTargetedObject (o)`: timer logic on `sLosTimer`: if `> ts`: `t = timer - ts` (stored), if `t >= 0`: `a = 0.0f, b = 70.0f, c = 1.0f`.
     Else if `-ts > timer`: `t = ts + timer` (stored) and the same `t >= 0` test. Else (|timer| <= ts): `P0 = pos(ped) + (0,0,0.75)`;
     `R = cross(pos(o) - pos(ped), (0,0,1)); R *= 2.0f / max(0.7f, |R|)` (helper(0.7f, |R|)); `P1 = R + P0`; if `GetIsLineOfSightClear(P0, P1,
     true, true, false, true, false, true, true)`: `sLosTimer = 100.0f` and set a/b/c as above; else `sLosTimer = -100.0f`.
     If m_bResetStatics: `sA = a; sB = b; sC = 0`. Else if !transition: `p = pow(0.96f [0x862F30], ts); q = 1 - p; sA = a*q + sA*p; sB = b*q + sB*p;
     sC = q*c + sC*p`. `yawOff = sB*0.017453292f; pitchOff = sA*0.017453292f`.
     Other fire types: `sC = 0; t = tan(FOV*0.5f*0.017453292f)`; `yawOff = atan(((CHairMultX [0xB6EC14] - 0.5f)*2) * t)`; `pitchOff =
     atan((((0.5f - CHairMultY [0xB6EC10])*2) * (1.0f/ms_fAspectRatio)) * t)` (each via `fld1; fpatan`).
  5. Reset (m_bResetStatics): `TheCamera.ResetDuckingSystem(ped); m_bRotating = 0; m_bCollisionChecksOn = 1; alphaSpeed = betaSpeed = 0;
     sAutoAim = 1; sIdleMs = 60000; sHeading = -1001.0f; sTargetMs = 0`. Unless `m_bUseMouse3rdPerson && ped->m_pTargetedObject == 0`... precisely:
     if `!m_bUseMouse3rdPerson || ped->m_pTargetedObject != 0`: `alpha = AT.alphaDefault`; in vehicle with m_pVehicle: `beta = (rot - PI/2) - yawOff;
     alpha += asin(clamp(veh->mat.fwd.z))`; else if `!m_pTargetedObject`: `beta = (rot - PI/2) + yawOff`; if `flags46C & 1`: `alpha = alpha -
     asin(clamp(DotProduct(&ped->field_578, &mat.fwd)))`; if weapon type 42: `alpha += CWeapon::ms_fExtinguisherAimAngle (0x8D610C)`.
     (rot = ped->m_fCurrentRotation +0x558.)
  6. Script heading step: identical to Process_M16_1stPerson step 5.
  7. `T = *target; ped->UpdateRpHAnim()`; `z0 = (pos(ped).z + 0.5f) + AT.zOffset`; if `FOV < 70`: `z0 = min((70 - FOV)/(70 - 50), 1.0f)*0.1f + z0`;
     `T.z = z0; dz = z0 - target.z`.
     `sideOff = 0.2f`; if `!((wi->flags18 >> 1) & 1) && ped->clothes->HasVisibleNewHairCut(1)`: `sideOff = 0.3f`; otherwise if `FOV < 70`:
     `sideOff = min((70 - FOV)/(70 - 35), 1.0f)*0.1f + 0.2f`.
     If `byte[0x8CCE64]`: `R = cross(front, up)`; `d = (R.y*right.y + R.z*right.z) + R.x*right.x` (ped matrix right); `v = d > 1 ? 1 : (d >= 0 ? d : 0)`;
     `k = (1.0f - acos(v)*0.63661975f [0x858FB8]) * sideOff`; `T += R*k`. Else `T += pedRight*sideOff`.
  8. Lock-on point: if `o = ped->m_pTargetedObject`: `L = (o is ped && wi->fire != 0) ? o->bone 3 : pos(o)`; melee: `L.z = dz*0.75f + L.z`;
     if `!m_bResetStatics && GetTaskFighting()`: `p = pow(0.9f, ts); sLock = p*sLock + (1-p)*L` (via the helpers, per component `a.c + b.c`);
     else `sLock = L`.
  9. Orientation: (a) lock-on (`o`): `D = sLock - T; yaw = atan2(-D.x, D.y) - PI/2; r2 = D.x*D.x + D.y*D.y; pitch = atan2(D.z, sqrt(r2))`;
     melee: `pitch = cos(yawOff) * pitch`; else `len = (|T - source| < AT.baseDist) ? Magnitude(T - source) : AT.baseDist`; `h = sqrt(D.z*D.z + r2)`;
     `ratio = (len + h)/h; pitchOff *= ratio; yawOff = ratio*yawOff`. `tB = yaw + yawOff; tA = pitch - pitchOff` (wrap tA by 2PI if < -PI / > PI);
     `maxStep = m_bResetStatics ? 1000.0f : ts*0.1f`; alpha and beta (beta after wrapping tB around beta) each move toward the target by at most
     maxStep (`|d| < maxStep -> target; d < 0 -> x - maxStep; else x + maxStep`); `alphaSpeed = betaSpeed = 0`.
     (b) mouse (`m_bUseMouse3rdPerson && DisablePlayerControls == 0 && (mx != 0 || my != 0)`): `f = FOV*0.0125f; dA = ((my*4.0f)*f)*m_fMouseAccelVertical;
     dB = ((mx*-2.5f)*f)*m_fMouseAccelHorzntl; betaSpeed = alphaSpeed = 0; beta += dB; alpha += dA`.
     (c) pad: `h = -(int)AimWeaponLeftRight(ped), v = (int)AimWeaponUpDown(ped)` (floats); `f = FOV*0.0125f`;
     `dB = ((((0.071428575f*f)*|h|)*h)*ts)*0.007f*0.007f; dA = ((((f*0.042857144f)*|v|)*v)*ts)*0.007f*0.007f`; `lag = (|h| < 2 && |v| < 2) ? 0.5f : 0.8f`;
     `p = pow(lag, ts); q = 1 - p; betaSpeed = p*betaSpeed + q*dB; alphaSpeed = q*dA + p*alphaSpeed` (the new speeds are the increments `bs`, `as`).
     Passenger in a vehicle (driver != ped): if `GetEnterTargeting()`: if `(float)(uint32)(now - sTargetMs) < 500`: `bs = PI; as = 0`; else `sTargetMs = now`.
     Driver: if `h == 0 && v == 0`: `if (!GetWeapon(ped)) sIdleMs += _ftol2(ts*0.02f*1000.0f)`; else `sIdleMs = 0`. If `sIdleMs > 5000`: `sAutoAim = 0;
     sHeading = (rot - PI/2) + yawOff` -> AUTO. Else if `> 2000`: `d = wrap2PI(((rot - PI/2) - yawOff) - beta)` (wrap only beyond +-2PI); if `d < 0.5235988f`
     (signed): `sHeading = (rot - PI/2) + yawOff; sIdleMs = 5001; sAutoAim = 0` -> AUTO; else `sAutoAim = 1` -> APPLY. Else `sAutoAim = 1`; if
     `GetWeapon(ped)`: `sIdleMs = 0`. Then: if `sAutoAim` -> APPLY else AUTO.
     APPLY: `sAutoAim = 1; sHeading = -1001.0f; beta = bs + beta; alpha = as + alpha`.
     AUTO (0x522641): `hd = rot - PI/2`; if `sHeading < -1000.0f`: `sHeading = hd` else `hd = sHeading`; if `!(wi flag bit1) && wi->fire != 0`:
     `ped->m_fCurrentRotation = ped->m_fAimingRotation = sHeading + PI/2; SetHeading(that); UpdateRwMatrix()`. `t = hd - yawOff`;
     `maxStep = ts*0.1f; dead = 0`; driver: `maxStep *= 0.25f; dead = 0.17453294f`. Wrap t around beta; `d = t - beta`; dead zone (if dead > 0:
     `d > dead -> d - dead; d < -dead -> d + dead; else 0`); `|d| < maxStep -> beta = d + beta (sAutoAim = 1); d < 0 -> beta - maxStep; else
     maxStep + beta`. Driver: same for alpha toward `asin(clamp(veh->fwd.z)) + AT.alphaDefault` (wrapped around alpha, dead zone 10deg); otherwise
     `alpha = as + alpha`.
  10. `ClipBeta()`; `if (alpha > AT.alphaMax) alpha = AT.alphaMax; else if (-AT.alphaMin > alpha) alpha = -AT.alphaMin`.
  11. `c = alpha > 0 ? cos(min(AT.alphaDistMul*alpha, PI/2)) : cos(alpha)`; `dist = c*AT.distScale + AT.baseDist`; `front = (-(cos b * cos a),
      -(sin b * cos a), sin a)`; `source = T - front*dist`; `HandleCameraMotionForDuckingDuringAim(ped, &source, &T, false)`; `fudge = T`;
      `SetColVarsAimWeapon(aimType)`; if `gCameraDirection == 3`: `CameraGenericModeSpecialCases(ped); CameraPedAimModeSpecialCases(ped);
      CameraColDetAndReact(&source, &T); ImproveNearClip(nullptr, ped, &source, &T)`. `m_bCamDirectlyBehind = m_bCamDirectlyInFront = 0`.
      If `sC > 0 && ped->m_pTargetedObject`: `k = sC*0.5f; P = T*(1-k) + sLock*k` (products then sums); `front = P - source`, normalise.
      `GetVectorsReadyForRW()`.
  12. Ped heading: skip if `(wi flag bit1) && !ducking`, if `wi->fire == 0`, or if in vehicle. `h` = weapon type 41 ? `atan2(-front.x, front.y) - yawOff`
      : `o` ? `atan2(-(pos(o)-pos(ped)).x, (pos(o)-pos(ped)).y)` : `sAutoAim` ? `atan2(-front.x, front.y) - yawOff` : (no heading change). If `h > -100.0f`:
      `ped->m_fCurrentRotation = ped->m_fAimingRotation = h + -0.05f [0x862F18]; ped->SetHeading(h)` (unbiased h); `UpdateRwMatrix()`.
      Then (also when no heading change but not skipped) `TheCamera.m_pTargetEntity->m_pPlayerData(+0x480)->f54 = TheCamera.Find3rdPersonQuickAimPitch()`.
  13. `m_bResetStatics = 0`.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:778); nothing to compare.
- Open questions: ped+0x534 (compared < 4) and CPlayerData+0x54 names are not verified (behaviour by offset).

## 0x522D40 CCam::Process_FollowPed_SA  (catalogue: specified/REA, gr-unreversed, port level 11)
- Convention / signature: `__thiscall void (CCam* this = ECX, const CVector& target [B+0x90], float, float, float, bool arg5 [B+0xA0])`,
  `ret 0x14`. Entry `sub esp,0x7C; push edi(=this); nop; jmp 0x406AFC` (.HOODLUM: `mov ecx,[edi+0x21C]; jmp 0x522D4C`), later pushes ebp(=ped),
  ebx, esi. B = ESP after all four pushes. Locals: `pad` B+0x2C, `T` B+0x5C..0x64, `interior` i32 B+0x54, `row` (interior*0x3C) B+0x48,
  `z` (distance) B+0x1C, `zMin` B+0x34, `Lmin` B+0x40, `pitchBase` B+0x28, `w` B+0x38, `w2` B+0x44, `up13` B+0x4C, `lo14` B+0x50,
  `a` B+0x20, `H` B+0x34 (re-used), `Lasin` B+0x18, `Lap` B+0x3C, `betaRate` B+0x58, `LR` B+0x30, `UD` B+0x24.
- Callers: CRunningScript::ProcessCommands2500To2599 0x47A760, CCam::Process_Cam_TwoPlayer 0x525E50 (2), CCam::Process 0x526FC0.
- Callees: see the bundle list; notably CPad::GetPad, CPedIntelligence::GetUsingParachute/GetTaskSwim/GetTaskJetPack/GetTaskUseGun/GetTaskClimb,
  CVector::Magnitude 0x4082C0, helper 0x420800, vector helpers 0x40FE90/0x40FEC0/0x40FE30/0x411A00 (operator+=), VectorSub 0x40FE60,
  CCamera::ResetDuckingSystem, CVector::DotProduct, _CIasin, _CIpow, CPad::GetForceCameraBehindPlayer 0x540AE0, AimWeaponLeftRight/UpDown
  (thiscall(CPed*)), GetPedWalkLeftRight 0x53FC90 / GetPedWalkUpDown 0x53FD30, CWorld::FindPlayerPed, CPlaceable::GetTopDirection 0x41CCB0
  (on TheCamera), IKChainManager_c::LookAt 0x618970 (on g_ikChainMan 0xC15448), CTaskSimpleClimb::GetCameraStickModifier 0x67A5D0, unnamed
  0x63A380 (thiscall on a TASK_COMPLEX_ENTER_CAR_AS_DRIVER (701) task), CPad::ClearMouseHistory, CCam::ClipBeta, CCamera::GetFadingDirection
  0x50ADF0, rounding helper 0x50A0A0 (cdecl(float, int) -> ST0), unnamed 0x50A120 (cdecl(CVector*, int)), CCam::GetVectorsReadyForRW,
  CCamera::HandleCameraMotionForDucking 0x50CFA0, SetColVarsPed 0x50CC50 (cdecl(int interior, int zoom)), CameraGenericModeSpecialCases,
  CameraColDetAndReact, ImproveNearClip, CGameLogic::IsCoopGameGoingOn 0x441390, squared-length helper 0x406DA0 (thiscall CVector* ->
  ST0 = (x*x + y*y) + z*z), CIdleCam::Process 0x522C80 (on gIdleCam 0xB6FDA0).
- Ped camera table `C[k]` = f32 at `0x8CC548 + interior*0x3C + 4k` (.data): row 0 {0.6, 2, 0.15, 2, 4, 0.8, 0.1, 0.5, 0.8, 0.1, 0.1, 0.02, 1,
  0.785398, 1.48353}; row 1 {0.6, 2, 0.15, 2, 3, 0.9, 0.1, 1, 0.8, 0.1, 0.3, 0.05, 1, 0.785398, 0.785398} (C7 and C12 unused here).
- Statics: `sPrevZ` f32 0xB6EC50, `sForceBehind` u8 0xB6EC54, CVector 0x8CCC3C and CVector 0xB6EC7C (written when arg5), `sLastAlpha` f32 0x8CCE74 /
  `sLastBeta` f32 0x8CCE6C (init -9999.0), u32 0xB6FE34. Data: 0x8CCE68/0x8CCE70/0x8CCE78/0x8CCE7C 0.0001f, 0x8CCE80 4.0f, 0x8CCE84 1.0f,
  0x8CCE88 0.9f, 0x8CCE8C 0.34906587f, 0x8CCE90 PI/2, 0x8CCE94 2.0f, 0x8CCE98 0.5f, 0x8CCE9C 2.9670599f, 0x8CCEA0 0.01f; 0x862F34 0.5f,
  0x862F38 -0.2617994f, 0x862F3C 3.0f, 0x862F40 -0.34906587f.
- Behaviour:
  1. If the target is not a ped or `!IsPlayer()`: return (m_bResetStatics unchanged). `pad = GetPad(0)`; ped type 1 -> `GetPad(1)`. `T = *target`.
     `interior = (CGame::currArea != 0)`.
  2. `d = C1`; if not standing (`flags46C & 1` clear) and `m_nPedZoom == 3` and parachuting: `d = d + d`. `z = m_fPedZoomSmoothed + d`;
     `zMin = C3`; `C4` -> B+0x3C; `up13 = C13; lo14 = C14`; if `z > sPrevZ`: `zMin = z`; `sPrevZ = z`. `pitchBase = C2 +` (zoom 1: m_fTargetZoomOneZExtra
     +0xE8; zoom 2: interior ? +0xF0 : +0xEC; zoom 3: +0xF4).
  3. `w = 0, w2 = 0`; swim task: `w = 1.0f`, if swim state (u16 task+0xA) != 4: `w2 = 0.5f`. Else jetpack: `w = 0.5f`; not standing: `w2 = 3.0f`.
  4. FOV (unless transition): reset -> 70; else step 1.0f*ts toward 70 (same rule as Process_AimWeapon step 3 with fovT = 70).
  5. `T.z = T.z + C0`. `Lmin = (z > C4) ? z : C4`.
  6. Train: if `!reset && !CamDirectlyBehind && !CamDirectlyInFront && !arg5` and `e = ped->m_standingOnEntity (+0x568)` is a train (sub-type 6) or
     attached to one: `sp = |e->moveSpeed|; k = max(0, sp - 0.01f) / max(0.01f, sp)` (helper 0x420800 twice); `hist[0] += ts*(k*e->mv);
     hist[1] += ts*(k*e->mv)` (each built with 0x40FE90 then 0x40FEC0, added with operator+=).
  7. Reset block (reset || behind || front || arg5): if arg5: `[0x8CCC3C] = pos(ped); [0xB6EC7C] = 0; T = pos(ped); T.z += C0`.
     `ResetDuckingSystem(ped); m_bRotating = 0; sForceBehind = 0; m_bCollisionChecksOn = 1`; if `!JustCameOutOfGarage && !arg5`:
     `beta = heading(ped) - PI/2` (+PI if DirectlyInFront). `betaSpeed = alphaSpeed = 0; m_fDistance = 1000.0f`. If `z == m_fPedZoomBase (0xB6F0F4)`:
     `m_fPedZoomSmoothed = z = m_fPedZoomTotal (0xB6F0F8)`. If `!garage && !arg5`: `alpha = 0`; standing: `alpha = -asin(clamp(DotProduct(&ped->field_578,
     &mat.fwd)))`. `front` from alpha/beta (`(-(cos b*cos a), -(sin b*cos a), sin a)`); `hist[0] = T - front*Lmin; histTime[0] = now; hist[1] = T - front*z;
     histPoints = 0`; if `!garage && !arg5`: `alpha = -pitchBase`; swim (state != 4): `alpha += -0.2617994f`; else jetpack: `alpha += -0.34906587f`.
     `GetPad(0)->ClearMouseHistory()`.
  8. Main (0x523426): `front = T - hist[0]`, normalise; `Ldist = sqrt((dx*dx + dz*dz) + dy*dy)` (T - hist[1]); if `Ldist < z && z > C3`:
     `z = (zMin > Ldist) ? zMin : Ldist`. `a = atan2(-front.x, front.y) - PI/2` (+2PI if < -PI). `H = heading(ped) - PI/2`, wrapped around `a` (+-PI).
  9. Force-behind: `pad->GetForceCameraBehindPlayer()` sets `sForceBehind = 1`; else if set it is cleared when `|ped mv|^2 > 0.001f`, or `|H - a| < 0.01f`,
     or AimWeaponLeftRight(ped) != 0, or AimWeaponUpDown(ped) != 0.
  10. Turn: `d = H - a`; if `|d| < 2.9670599f || sForceBehind || w != 0`: `r = ts*C10; lim = ts*C11`; if `!sForceBehind && w == 0`: `m = |ped.mv -
      standOn.mv|` (or `|ped.mv|` when not standing on an entity); `r = (m*r > 1.0f) ? 1.0f : m*r` (magnitude recomputed); else `r = min(r*0.5f, 1.0f);
      lim = 2.0f*lim`; if `w != 0`: `r *= w; lim *= w`. `turn = clamp(d*r, -lim, lim)`; else `turn = 0`. `a = a + turn`; wrap a around beta; `dt = max(1, ts)`;
      `betaRate = (a - beta)/dt`.
  11. Pitch: `Lasin = asin(clamp(front.z))`; `e = |H - a|`; if `e > PI/2 && |ped mv|^2 > 0.002f`: `k = min(((e - PI/2)*1.2f)/(PI - PI/2), 1.0f);
      lim2 = PI/2 - (PI/2 - 0.34906587f)*k; p = pow(0.9f, ts)`; `if (Lasin > lim2) Lasin = (1-p)*lim2 + p*Lasin; else if (-lim2 > Lasin) Lasin = Lasin*p - (1-p)*lim2`.
      If `w2 != 0` or (`sForceBehind` and standing): `tgt = jetpack ? -0.34906587f : swim ? -0.2617994f : standing ? -asin(clamp(DotProduct(&field_578, &fwd))) : 0`;
      `r2 = min(1.0f*r, 1.0f); lim3 = 4.0f*lim`; if `w2 != 0`: `r2 *= w2; lim3 *= w2`; `turnA = clamp((tgt - Lasin)*r2, -lim3, lim3)`; else `turnA = 0`.
      `Lasin = (Lasin + turnA) - pitchBase`; `Lasin = (Lasin > C13) ? C13 : (Lasin < -C14 ? -C14 : Lasin)`.
      `lim4 = ts*C6; Lap = clamp((Lasin - alpha)*(1.0f - pow(C5, ts)), -lim4, lim4)`.
  12. Input: `LR = -(int)AimWeaponLeftRight(ped)`, `UD = (int)AimWeaponUpDown(ped)` (floats). With a gun task in state 2 or 3 whose weapon info has flag bit1 clear:
      if `|LR| < |GetPedWalkLeftRight()|`: `LR = -(float)GetPedWalkLeftRight()`. If `UD == 0 && LR == 0 && walkLR == 0 && walkUD == 0`: `p0 = FindPlayerPed(0)`;
      `top = TheCamera.GetTopDirection()`; if `(top.x*fwd.x + top.z*fwd.z) + top.y*fwd.y > 0.3f` (fwd = p0 matrix +0x10):
      `g_ikChainMan.LookAt("FollowPedSA", p0, nullptr, 1500, -1, &(pos(p0) + top*5.0f), false, 0.25f, 500, 3, false)`.
      `f = FOV*0.0125f`; `LR = ((|LR|*(f*0.071428575f))*0.007f*0.007f)*LR`; `UD = ((|UD|*(f*0.042857144f))*0.007f*0.007f)*UD`.
      Climb task: `climb->GetCameraStickModifier(ped, &alpha, &beta, &UD, &LR)`; else if the active task type is 701: `task->F63A380(ped, z, &alpha, &beta, &UD, &LR)`.
  13. Beta: `p = pow(C8, ts); lim = C9; x = clamp(LR + betaRate, -lim, lim); q = 1 - p; betaSpeed = p*betaSpeed + q*x`; `|betaSpeed| < 0.0001 -> 0`.
      Mouse (m_bUseMouse3rdPerson && DisablePlayerControls == 0): `LR = ((mx*-2.5f)*(FOV*0.0125f))*m_fMouseAccelHorzntl; betaSpeed = 0; beta = LR + beta`;
      else `beta = ts*betaSpeed + beta`. `ClipBeta()`.
  14. Alpha: `alphaSpeed = q*alphaSpeed + UD*p` (NOTE: p and q are swapped relative to beta - original behaviour); clamp to [-C9, C9]; `< 0.0001 -> 0`;
      `Lasin = ts*alphaSpeed + Lasin`. Mouse: `dA = ((my*2.5f [0x858FA0])*(FOV*0.0125f))*m_fMouseAccelHorzntl` (horizontal accel); if `(m_bFading &&
      GetFadingDirection() == 1 && FadeValue > 45) || FadeValue > 200`: auto pitch toward `P = -pitchBase` (same 5-way rule as Process_FollowPedWithMouse
      step 6); `alphaSpeed = 0; alpha = dA + alpha`. Not mouse: `alpha = Lap + alpha`. Clamp: `alpha > C13 -> C13, alphaSpeed = 0; else -C14 > alpha ->
      -C14, alphaSpeed = 0`. Jitter filter with sLastAlpha/sLastBeta (as Process_FollowCar_SA step 24).
  15. Output: `front` from alpha/beta; `GetVectorsReadyForRW()`; `m_bCamDirectlyBehind = m_bCamDirectlyInFront = 0`; `source = T - front*z`; round source
      (x, y, z via 0x50A0A0(v, 4)). `Lasin = Lasin + pitchBase`; `f' = (-(cos b*cos Lasin), -(sin b*cos Lasin), sin Lasin)`; `hist[0] = T - f'*Lmin;
      hist[1] = T - f'*z`.
  16. If `GetForceCameraBehindPlayer() && AimWeaponLeftRight(0) != 0` and `ts*0.1f > |wrapPI(beta - (rot - PI/2))|`: `ped->m_fAimingRotation = beta + PI/2`.
      `HandleCameraMotionForDucking(ped, &source, &T, false); fudge = T`; round source; `SetColVarsPed(interior, m_nPedZoom)`. If `gCameraDirection == 3`:
      `CameraGenericModeSpecialCases(ped)`; CCollision::bCamCollideWithVehicles/Objects/Peds = 1; `CameraColDetAndReact(&source, &T); ImproveNearClip(nullptr,
      ped, &source, &T); F50A120(&source, 4)`. `m_bCamDirectlyBehind = m_bCamDirectlyInFront = 0`; round source; `GetVectorsReadyForRW()`.
  17. Idle cam: if `!interior && m_nWhoIsInControlOfTheCamera (0xB6F0D8) != 1 && standing && !IsCoopGameGoingOn() && !m_bFOVLerpProcessed (0xB6FCDD) &&
      !m_bVecMoveLinearProcessed (0xB6FD14) && !m_bVecTrackLinearProcessed (0xB6FCB4) && !(|ped mv|^2 > 0.0001f)`: `gIdleCam.Process()`. Else
      `[0xB6FE34] = 0`. `m_bResetStatics = 0`.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:1005); nothing to compare.
- Open questions: semantics of 0x50A120 and 0x63A380 (other specs); behaviour here is fully specified by the calls.

## 0x5245B0 CCam::Process_FollowCar_SA  (catalogue: specified/REA, gr-unreversed, port level 0)
- CATALOGUE EXTENT IS WRONG (truncated). The catalogue gives 0x407E47-0x407E60;0x5245B0-0x5245BD (14 insns). The
  .HOODLUM stub at 0x407E47 loads `edi = this->m_pCamTargetEntity` and "returns" to `[0x15608A4] + [0x531F2C]` =
  0x0065536B + 0xFFECF252 = 0x5245BD (mod 2^32), i.e. straight back into the function. The real body is
  0x5245BD-0x525E4A (~1700 instructions, ends with `ret 0x14` at 0x525E48; 0x525E4B-0x525E4F are nop padding before
  Process_Cam_TwoPlayer 0x525E50). The spec below covers the full body (disassembled with tools/specs/dis.py
  0x5245BD-0x525E4B). The catalogue/extents.csv row should be corrected by the owner of the catalogue.
- Convention / signature: `__thiscall void (CCam* this = ECX, const CVector& target [esp+4], float orientation [+8],
  float speedVar [+0xC], float speedVarWanted [+0x10], bool arg5 [+0x14])`, `ret 0x14`. Prologue `sub esp,0x68; push esi;
  push edi; push ebx; push ebp`. orientation/speedVar/speedVarWanted are never read. The `target` argument slot is
  re-used as a float local after the vector has been copied (irrelevant for a port). arg5 is read once (0x524C02,
  "do not reset alpha/beta"), then its slot is used as a scratch float.
- Callers: CRunningScript::ProcessCommands2500To2599 0x47A760, Process_Cam_TwoPlayer 0x525E50 (2), CCam::Process 0x526FC0.
- Callees (via `orig!`): CPad::GetPad 0x53FB70, CCamera::ApplyVehicleCameraTweaks 0x50A480, CVehicle::GetVehicleAppearance
  0x6D1080, CCamera::GetArrPosForVehicleType 0x50AF00 (thiscall(appearance, int* out), ret 8), CEntity::GetColModel
  0x535300, CVector::Normalise 0x59C910, CVector::CrossProduct-out 0x59C730 (cdecl(out, a, b)), CVector::DotProduct
  0x40FDB0 (cdecl(a*, b*) -> ST0), CPlaceable::GetHeading 0x441DB0 (ST0), vehicle vtable slot +0xD4 = GetHeightAboveRoad
  (ST0), helper 0x420800 (cdecl(float a, float b) -> ST0 = `a > b ? a : b`, NaN -> b), CRT _CIasin 0x821E70 (ST0 -> ST0),
  CRT _CIpow 0x822130 (ST1 ^ ST0), CPad::AimWeaponLeftRight 0x541040 / AimWeaponUpDown 0x5410C0 (thiscall(CPed* = 0) ->
  i16 AX), CPad::GetSteeringUpDown 0x53FBD0, CTaskManager::GetActiveTask 0x681720, task vtable +0x10 = GetTaskType,
  and the tail calls listed in the later blocks.
- Notation: `V` = vehicle (EDI = this->m_pCamTargetEntity, +0x21C). Locals are named by their ESP offset after the
  four pushes (frame base B = ESP at 0x524624): `L14` = [B+0x14] etc. `T` = target copy at B+0x48..0x50.
  `C[k]` = f32 at `0x8CC600 + vt*0x3C + 4*k` (per-vehicle-type camera table, 7 rows of 15 floats, .data):
  ```
  vt   C0   C1   C2   C3   C4   C5    C6    C7   C8    C9    C10    C11   C12   C13       C14
  0   1.3  1.0  0.4  10   15   0.5   1.0   1.0  0.85  0.2   0.075  0.05  0.8   0.785398  1.55334
  1   1.1  1.0  0.1  10   11   0.5   1.0   1.0  0.85  0.2   0.075  0.05  0.75  0.785398  1.55334
  2   1.1  1.0  0.2  10   15   0.05  0.05  0    0.9   0.05  0.01   0.05  1.0   0.174533  1.22173
  3   1.1  3.5  0.2  10   25   0.5   1.0   1.0  0.75  0.1   0.005  0.2   1.0   1.55334   1.55334
  4   1.3  1.0  0.4  10   15   0.5   1.0   0    0.9   0.05  0.005  0.05  1.0   0.349066  1.22173
  5   1.1  1.0  0.2  10   5    0.5   1.0   1.0  0.75  0.1   0.005  0.2   1.0   0.785398  1.55334
  6   1.1  1.0  0.2  10   5    0.5   1.0   1.0  0.75  0.1   0.005  0.2   1.0   0.349066  1.22173
  ```
  (values read from the image; it is writable data, the port must use the live global). `ts` = CTimer::ms_fTimeStep
  0xB7CB5C. `sBlend` = static f32 0xB7011C (trailer/passenger blend), `sMouseTimer` = static f32 0xB70118,
  `sPitchDone` = static u8 0xB70114. Constants: PI 0x858CB8, 2PI 0x858CBC, -PI 0x858CC0, PI/2 0x858FE4 = 1.5707964f,
  1.0 0x858624, 0.0 0x858B50, 0.5 0x858B8C.
- Fields of `this` (CCam): +0x03 m_bCollisionChecksOn, +0x0A m_bResetStatics, +0x0B m_bRotating, +0xAC alpha
  (m_fVerticalAngle), +0xB0 m_fAlphaSpeed, +0xB4 m_fFOV, +0xBC beta (m_fHorizontalAngle), +0xC0 m_fBetaSpeed,
  +0xC4 m_fDistance, +0xCC m_fCaMinDistance, +0xD0 m_fCaMaxDistance, +0x190 m_vecFront, +0x19C m_vecSource,
  +0x1B4 m_vecUp, +0x1D8 m_avecTargetHistoryPos[0..1], +0x208 m_anTargetHistoryTime[0], +0x218 m_nCurrentHistoryPoints.
  Vehicle: +0x14 matrix ptr, +0x22 model i16, +0x36 type/status byte, +0x44 m_vecMoveSpeed, +0x50 m_vecTurnSpeed,
  +0x38C u32 flags (bit 0x20000 tested), +0x460 driver, +0x464 passenger[0], +0x4C8 trailer, +0x590 base vehicle type,
  +0x594 vehicle sub-type (eVehicleType: 0 automobile, 2 quad, 3 heli, 4 plane, 5 boat, 9 bike), +0x804 (CBike)
  m_nNoOfContactWheels u8, +0x960 (CAutomobile) m_nNumContactWheels u8, +0x86C (CPlane, Hydra) nozzle angle u16.
- Behaviour:
  1. (0x5245BD) If `(V->type & 7) != 2` (not a vehicle): return immediately (no state change).
  2. Copy `T = *target` (B+0x48). `pad = CPad::GetPad(0)`; if `V->driver && driver->m_nPedType (+0x598) == 1`:
     `pad = GetPad(1)`. `TheCamera.ApplyVehicleCameraTweaks(V)`.
  3. Vehicle camera type `vt` (0x524624-0x5246C6, stored at L24): model 0x1B9, 0x1D0, 0x234, 0x252 -> 5; model 0x1D1,
     0x1F5 -> 6; else base type == 9 -> 1; sub-type 2 -> 1; sub-type 3 -> 2; sub-type 4: if model 0x208 and nozzle
     (u16 +0x86C, zero-extended) >= CPlane::HARRIER_NOZZLE_SWITCH_LIMIT (i16 0x8D33C8, sign-extended; 3000) -> 2, else
     `vt = (model == 0x21B) ? 0 : 3` (computed as `((model==0x21B) - 1) & 3`); sub-type 5 -> 4; otherwise 0.
  4. `zoom (in the target-argument slot, "Lz") = TheCamera.m_fCarZoomSmoothed (0xB6F0E8) + C1`.
     `arr = 0; TheCamera.GetArrPosForVehicleType(V->GetVehicleAppearance(), &arr)`.
     `Lh (L1C) = 0`; if `(V->byte36 & 0xF8) == 0x40` (status 8 = REMOTE_CONTROLLED): `Lh = tblB[arr]`; else by
     `TheCamera.m_nCarZoom` (0xB6F0DC): 1 -> `tblA[arr]`, 2 -> `tblB[arr]`, 3 -> `tblC[arr]`, other -> stays 0.
     tblA = f32[5] at 0x8CC41C {0.08,0.08,0.15,0.08,0.08}, tblB at 0x8CC430 {0.07,0.08,0.3,0.08,0.08}, tblC at 0x8CC444
     {0.055,0.05,0.15,0.06,0.08}.
  5. `Lmaxz (L18) = V->GetColModel()->bbox.max.z (+0x14)`; `Llen (L38) = fabs(colmodel.bbox.min.y (+4)) + same`
     (`fadd st0,st0`).
  6. Trailer (0x524776): if `V->trailer (+0x4C8)`:
     - if `sBlend < 1.0f`: `t = ts * 0.02f [0x8CCEE8] + sBlend; sBlend = (1.0f < t) ? 1.0f : t`.
     - `cv = V col model, ct = trailer col model`; `dx = ct.max.x - cv.min.x, dy = ct.max.y - cv.min.y, dz = ct.max.z - cv.min.z`;
       `Llen = sqrt((dz*dz + dy*dy) + dx*dx) * 0.5f [0x8CCEE4] * sBlend + Llen`.
     - `m = (Lmaxz > ct.max.z) ? Lmaxz : ct.max.z`; `Lmaxz = (m - Lmaxz) * sBlend + Lmaxz`.
     - `P = trailer->matrix ? &matrix->pos : &trailer->placement.pos`; `k = sBlend * 0.5f`; `w = 1.0f - k`;
       `T.x = T.x*w + k*P.x; T.y = T.y*w + k*P.y; T.z = T.z*w + k*P.z`.
     Else (no trailer, 0x5248F2): if sub-type is not 9 and not 2: `sBlend = 0`. Otherwise (bike/quad): if
     `V->passenger[0]` (+0x464): if `sBlend < 1.0f`: `t = ts*0.02f + sBlend; sBlend = (1.0f < t) ? 1.0f : t`;
     else (no passenger): if `sBlend > 0`: `t = sBlend - ts*0.02f; sBlend = (0 > t) ? 0 : t` (t kept when NaN);
     then `Lmaxz = 0.4f [0x8CCEE0] * sBlend + Lmaxz`.
  7. (0x5249B3) `a = TheCamera.m_fCurrentTweakDistance (0xB6FD70) * Llen; Lz = Lz + a; Lmin2 (L28) = a * C3`.
  8. Height: if `V->GetVehicleAppearance() == 3` (heli) and status != 8: with `u = V->matrix->up (+0x20)`:
     `T.x += (0.6f [0x8CC53C]*u.x)*Lmaxz; T.y += (0.6f*u.y)*Lmaxz; Tz' = (0.6f*u.z)*Lmaxz + T.z`.
     Else: `d = Lmaxz * C0 - C2`; `Tz' = T.z`; if `d > 0`: `Tz' = T.z + d; Lz = d + Lz; Lh = (0.3f [0x8CCEDC] / Lz) * d + Lh`.
     Then (0x524A8E) `T.z = TheCamera.m_fCurrentTweakAltitude (0xB6FD74) * Tz'` (multiplicative, as in the bytes);
     `Lh = TheCamera.m_fCurrentTweakAngle (0xB6FD78) + Lh`.
  9. `lim = C4`; if `m_nCarZoom == 1 && (vt == 0 || vt == 1)`: `lim *= 0.65f [0x858F50]`.
     `Lmin (L2C) = (Lz > lim) ? Lz : lim`. Then `this->m_fCaMaxDistance = Lz; this->m_fCaMinDistance = 3.5f`.
  10. FOV (0x524AEA): if `m_bResetStatics`: `FOV = 70.0f`. Else: if sub-type is 0 or 9: `dot = (fwd.z*mv.z + fwd.y*mv.y) +
      mv.x*fwd.x` (fwd = matrix +0x10, mv = +0x44); if `dot > 0.4f [0x8CC540]`: `FOV = (dot - 0.4f) * ts + FOV` (dot
      recomputed with the same products). Then if `FOV > 70.0f [0x858CE0]`: `FOV = (FOV - 70.0f) * pow(0.98f [0x8CC544], ts)
      + 70.0f`. Then `if (FOV > 100.0f) FOV = 100.0f; else if (FOV < 70.0f) FOV = 70.0f`.
  11. Reset block (0x524BE8): if `m_bResetStatics || TheCamera.m_bCamDirectlyBehind (0xB6F042) ||
      TheCamera.m_bCamDirectlyInFront (0xB6F043)`:
      `m_bResetStatics = 0; m_bRotating = 0; m_bCollisionChecksOn = 1; TheCamera.m_bResetOldMatrix (0xB6F999) = 1`.
      If `!TheCamera.m_bJustCameOutOfGarage (0xB6F050) && !arg5`: `alpha = 0`;
      `beta = (V->matrix ? atan2(-fwd.x, fwd.y) : V->placement.heading (+0x10)) - PI/2`; if `m_bCamDirectlyInFront`:
      `beta += PI` (the stored beta is the value after the add in that case).
      Then always: `m_fBetaSpeed = 0; m_fAlphaSpeed = 0; m_fDistance = 1000.0f`;
      `ca = cos(alpha)`; `front = (-(cos(beta)*ca), -(sin(beta)*ca), sin(alpha))`;
      `hist[0] = T - front*Lmin` (each component `T.c - Lmin*front.c`); `histTime[0] = CTimer::m_snTimeInMilliseconds`;
      `hist[1] = T - front*Lz`; `m_nCurrentHistoryPoints = 0`.
      If `!m_bJustCameOutOfGarage && !arg5`: `alpha = -Lh`.
  12. Direction (0x524DA9): `front = T - hist[0]; front.Normalise()`.
      `Ldist (L38) = sqrt((dx*dx + dz*dz) + dy*dy)` with `d = T - hist[1]`.
      `a = atan2(-front.x, front.y) - PI/2; if (a < -PI) a += 2PI`.
      `sp = sqrt(mv.x*mv.x + mv.y*mv.y)`; `b = (sp > 0.02f [0x858B38]) ? atan2(-mv.x, mv.y) - PI/2 : a`.
      Wrap b around a: `if (b > a + PI) b -= 2PI; else if (b < a - PI) b += 2PI`.
      `r = ts * C10; Lclamp (L18) = ts * C11`.
      `dv = (mv.z*front.z + mv.y*front.y) + mv.x*front.x`; `lat = mv - front*dv`;
      `ll = sqrt((lat.x*lat.x + lat.z*lat.z) + lat.y*lat.y)`; `f = r * ll; if (1.0f < f) f = 1.0f`.
      `d = (b - a) * f`; `if (d > Lclamp) d = Lclamp; else if (d < -Lclamp) d = -Lclamp`; `nb = a + d`.
      Wrap nb around beta: `if (nb > beta + PI) nb -= 2PI; else if (nb < beta - PI) nb += 2PI`.
      `dt = (1.0f > ts) ? 1.0f : ts`; `LbetaSpd (L44) = (nb - beta) / dt`.
      `v = clamp(front.z, -1, 1)` (if `1 < fz` -> 1; else if `-1 > fz` -> -1; NaN passes through); `Lasin (L14) = asin(v)`.
  13. Distance (0x52507C): if `Ldist < Lz`: if `Lz > Lmin2`: `Lz = (Lmin2 > Ldist) ? Lmin2 : Ldist`.
      `Lup (L28) = C13`.
  14. Slope pitch (0x5250BB, only when `(mv.x*mv.x + mv.y*mv.y) + mv.z*mv.z < 0.04f [0x863244]`): skipped when base type
      is 9 and `V->m_nNoOfContactWheels (+0x804) < 4`, when sub-type is 3, or when sub-type is 4 and
      `m_nNumContactWheels (+0x960) == 0`. Otherwise:
      `R = cross(fwd, (0,0,1))`, normalise; `U = cross(R, fwd)`, normalise (CrossProduct-out(out, a, b) = a x b).
      If `(U.y*front.y + U.z*front.z) + U.x*front.x > 0`:
      - `h = V->GetHeightAboveRoad()`; `P = V->matrix ? &matrix->pos : &placement.pos`; `Lrel (L40) = (T.z - P.z) + h`.
      - `s = fabs(sin(beta - (GetHeading(V) - PI/2)))`; `A = asin(s)`; `Bc = atan2(cm.max.x, -cm.min.y)` (V col model).
      - if `A > Bc`: `Rr = (1.2f [0x8CCED8] + cm.max.x) / cos(max(0.0f, PI/2 - A))` (helper 0x420800 with a = 0.0f,
        b = PI/2 - A); else `Rr = (1.5f [0x8CCED4] - cm.min.y) / cos(A)`. (L30)
      - The matrix pointer is dereferenced without a null check from here (0x525263). `c = cos(beta - (GetHeading(V) - PI/2))`;
        `pitch = atan2(fwd.z, sqrt(fwd.x*fwd.x + fwd.y*fwd.y))`;
        `Lup = c*pitch + atan2(Lrel, Rr * 1.2f [0x8CCED0])`.
      - if base type == 0 and `m_nNumContactWheels > 1` and `fabs(DotProduct(&V->turnSpeed, &fwd)) < 0.05f [0x858C28]`:
        `c2 = cos((beta - (GetHeading(V) - PI/2)) + PI/2)`; `roll = atan2(right.z, sqrt(right.x*right.x + right.y*right.y))`;
        `Lup = c2*roll + Lup`.
  15. Pitch target (0x525368): `Lasin = Lasin - Lh`; `if (Lasin > Lup) Lasin = Lup; else if (Lasin < -C14) Lasin = -C14`.
      `Lclamp = ts * C6`; `Lap (L30) = (Lasin - alpha) * (1.0f - pow(C5, ts))`; clamp Lap to [-Lclamp, Lclamp]
      (`> Lclamp` -> Lclamp, else `< -Lclamp` -> -Lclamp).
  16. Stick input (0x52540D): `LR = -(int)pad->AimWeaponLeftRight(0)`, `UD = (int)pad->AimWeaponUpDown(0)` (both i16
      sign-extended, then fild). If `CCamera::m_bUseMouse3rdPerson` (0xB6EC2E): `UD = 0.0f`. `f = FOV * 0.0125f [0x85A5C4]`.
      `Lh2 (L20) = ((fabs(LR) * (f * 0.071428575f [0x859B50])) * LR) * 0.007f * 0.007f` (0x8CC4A0 twice);
      `Lv (L10) = ((fabs(UD) * (f * 0.042857144f [0x8631AC])) * UD) * 0.007f * 0.007f`. `allowAuto (byte at B+0x8C) = 1`.
      Models 0x1BB, 0x1E6, 0x196, 0x20C, 0x250, 0x208, 0x20D, 0x212, 0x213: `Lv = 0`. Model 0x234, or (base type 0 and
      `V->flags38C & 0x20000`): `Lh2 = 0; Lv = 0`. Any other model: `allowAuto = 0` (both kept).
      If `gCameraDirection (0x8CC384) != 3`: `Lv = 0; Lh2 = 0`.
  17. If `vt == 0` (0x52553F): `s = GetSteeringUpDown(pad)`; if `fabs(s) > 120.0f [0x858BB0]` and V->driver and the
      driver's active task (driver+0x47C -> +4 task manager) exists and its type != 704 (TASK_COMPLEX_LEAVE_CAR):
      `s` re-read; `Lv = ((fabs(s) * ((FOV*0.0125f) * 0.042857144f)) * s) * 0.007f * 0.007f * 0.5f + Lv`.
      Then if `Lv > 0`: `Lv *= 0.5f`.
  18. Mouse (0x525615, `mouseUsed` = BL = 0): if `m_bUseMouse3rdPerson && pad->DisablePlayerControls (u16 +0x10E) == 0`:
      `my2 = NewMouseControllerState.m_AmountMoved.y (0xB73428) * 2` (`fadd st0,st0`), `mx2 = m_AmountMoved.x (0xB73424) *
      -2.0f [0x858B18]`; `en = (sub-type 4 or 3) ? CVehicle::m_bEnableMouseFlying (0xC1CC03) : m_bEnableMouseSteering (0xC1CC02)`.
      - If `(mx2 != 0 || my2 != 0) && !(pad->NewState.m_bVehicleMouseLook (+0x2C) == 0 && en)` (mouse look, 0x5256F1):
        `g = FOV * 0.0125f`; `Lv = (my2 * g) * CCamera::m_fMouseAccelHorzntl (0xB6EC1C)`; `Lh2 = (mx2 * g) *
        m_fMouseAccelHorzntl` (the horizontal accel is used for both axes); `m_fBetaSpeed = 0; m_fAlphaSpeed = 0;
        Lasin = alpha; sMouseTimer = 1.0f [0x8CCECC] * 50.0f [0x858B40]`; `mouseUsed = 1`.
      - Else, if `sMouseTimer > 0`: `m_fBetaSpeed = 0; m_fAlphaSpeed = 0; Lv = 0; Lh2 = 0; Lasin = alpha;
        sMouseTimer = max(0.0f, sMouseTimer - ts)` (helper 0x420800(0.0f, x)); `mouseUsed = 1`.
  19. Passenger solicit (0x525749): if `V->passenger[0]` has an active task of type 1309 (TASK_COMPLEX_PROSTITUTE_SOLICIT)
      with `byte(task+0x2D) & 1`: `Lv = ((Lup - 0.1f [0x8CCEC8]) > alpha) ? ts * 0.0035f [0x8CCEC4] : 0`.
  20. Auto pitch (0x5257D2): if `allowAuto`: if `gCameraMode (0x8CC388) != 18`: `sPitchDone = 0` and test; else if
      `sPitchDone` skip the test (and set `sPitchDone = 1` again). Test: if `fabs(Lh + alpha) > 0.05f [0x858C28]`:
      `Lv = (-Lh - alpha) * 0.05f [0x8CCEC0]`; else `sPitchDone = 1`.
      Then `Lv *= C12`.
  21. Beta (0x525842): `Lh2 = Lh2 * C12`; `pw (L3C) = pow(C8, ts)`; `x = Lh2 + LbetaSpd`; clamp x to [-C9, C9]
      (`x > C9` -> C9, else `x < -C9` -> -C9). `q = 1.0f - pw`; `m_fBetaSpeed = x*q + pw*m_fBetaSpeed`; if
      `fabs(m_fBetaSpeed) < 0.0001f [0x8CCEBC]`: `m_fBetaSpeed = 0`. `beta = (mouseUsed ? Lh2 : ts*m_fBetaSpeed) + beta`.
      If `m_bJustCameOutOfGarage`: `beta = CGeneral::GetATanOfXY(front.x, front.y) + PI`. Then `ClipBeta()` (0x509C50).
  22. Extra pitch-down (0x52593E): if `vt <= 1` (signed) and `Lasin < alpha` and NOT `Ldist < Lz` (i.e. Ldist >= Lz;
      `test ah,1` - NaN skips) and the contact-wheel count (base type 0: +0x960, base type 9: +0x804, other: skip) > 1:
      `Lv = (Lasin - alpha) * 0.075f [0x8CCEB8] + Lv`.
  23. Alpha (0x5259A1): `as = q*Lv + pw*m_fAlphaSpeed`; `m_fAlphaSpeed = as`; `lim = C9; if (Lv > 0) lim *= 0.5f`;
      `if (as > lim) m_fAlphaSpeed = lim; else if (as < -lim) m_fAlphaSpeed = -lim`; if `fabs(m_fAlphaSpeed) < 0.0001f
      [0x8CCEB4]`: 0. If mouseUsed: `Lasin = Lv + Lasin; alpha = Lv + alpha`; else `Lasin = ts*m_fAlphaSpeed + Lasin;
      alpha = Lap + alpha`. Clamp: `if (alpha > Lup) { alpha = Lup; m_fAlphaSpeed = 0; } else if (-C14 > alpha)
      { alpha = -C14; m_fAlphaSpeed = 0; }`.
  24. Jitter filter: statics `sLastAlpha` f32 0x8CCEB0 and `sLastBeta` f32 0x8CCEA8 (both initialised -9999.0f):
      `if (fabs(sLastAlpha - alpha) < 0.0001f [0x8CCEAC]) alpha = sLastAlpha; sLastAlpha = alpha;`
      `if (fabs(sLastBeta - beta) < 0.0001f [0x8CCEA4]) beta = sLastBeta; sLastBeta = beta;`
  25. Output (0x525B0E): `ca = cos(alpha)`; `front = (-(cos(beta)*ca), -(sin(beta)*ca), sin(alpha))`.
      `source.x/y/z = R(source.c, 4)` for x, y, z in that order, where R = unnamed helper 0x50A0A0 (cdecl(float, int32)
      -> ST0, rounds to the given number of decimals; owned by another spec) applied to the PREVIOUS source values.
      `GetVectorsReadyForRW()` (0x509CE0). `m_bCamDirectlyBehind = m_bCamDirectlyInFront = 0`.
      `source = T - front*Lz` (`T.c - Lz*front.c`). `Lasin = Lasin + Lh`. `m_vecTargetCoorsForFudgeInter (+0x148) = T`.
      `hist[2] (+0x1F0) = hist[0]` (dword copy). With `ca2 = cos(Lasin)`, `f' = (-(cos(beta)*ca2), -(sin(beta)*ca2),
      sin(Lasin))`: `hist[0] = T - f'*Lmin`, `hist[1] = T - f'*Lz` (each `T.c - f'.c*L`).
  26. `CCamera::SetColVarsVehicle(vt, TheCamera.m_nCarZoom)` (0x50CCA0, cdecl(int32, int32)).
  27. If `gCameraDirection == 3`: `CWorld::pIgnoreEntity (0xB7CD68) = V; TheCamera.m_nExtraEntitiesCount (0xB6FC70) = 0;
      TheCamera.CameraVehicleModeSpecialCases(V)` (0x50CDE0); if `byte(V+0x429) & 4`: `byte 0x9655E5 = 1`;
      `TheCamera.CameraColDetAndReact(&source, &T)` (0x520190); `TheCamera.ImproveNearClip(V, nullptr, &source, &T)`
      (0x516B20); `pIgnoreEntity = nullptr`; `source.c = R(source.c, 4)` (x, y, z).
  28. Always: `m_bCamDirectlyBehind = m_bCamDirectlyInFront = 0`; `source.c = R(source.c, 4)` (x, y, z) again;
      `GetVectorsReadyForRW()`; global CVector at 0xB6F018 (unnamed, just before TheCamera) `= T`. Return.
- Edge cases / oddities: the 3rd-person mouse look scales both axes by `m_fMouseAccelHorzntl` (0x525703 and 0x525713);
  tweak altitude multiplies the world Z of the target (step 8); the slope block dereferences `V->m_matrix` without a null
  check (0x525263); source rounding happens before the new source is computed (step 25) and again twice afterwards.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:995), hook `.Reversed = false`. Row Cam.cpp:996 (notsa-unreachable)
  is not a behavioural difference: gr never runs its own body. Nothing to compare.
- Open questions: none affecting behaviour; names of the statics 0xB7011C/0xB70118/0xB70114/0x8CCEB0/0x8CCEA8, the global
  0xB6F018 and byte 0x9655E5 are unknown (behaviour is fully given by address).

## 0x525E50 CCam::Process_Cam_TwoPlayer  (catalogue: specified/REA, gr-unreversed, port level 12)
- Convention / signature: `__thiscall void (CCam* this = ECX)`, plain `ret`. Frame `sub esp,0x74` + ebx(=0, later loop index), ebp, esi(=this), edi;
  B = ESP after the pushes. Locals: `cand` B+0x10, `t` B+0x14, `lim` B+0x18, `r`/`p` B+0x1C, `newB`/`rate` B+0x20, `v` B+0x28..0x30, `src` B+0x34..0x3C,
  out2 B+0x40.
- Callers: CCam::Process 0x526FC0.
- Callees: CPad::GetPad 0x53FB70, unnamed CPad 0x5404A0 (thiscall, no args -> AL; "switch focus" button), unnamed CCam 0x5132D0
  (thiscall(float beta, CVector* outSource, CVector* out2, CVector* fudge)) and 0x513220 (thiscall(CVector source by value) -> AL, 1 = position OK),
  CVector::Magnitude, _CIpow, CPad::AimWeaponLeftRight 0x541040 (thiscall(CPed*)), CWorld::ProcessLineOfSight, CText::Get, CHud::SetHelpMessage 0x588BE0
  (cdecl(text, 0, 0, 0)), CVector::Normalise, CrossProduct-out, CCam::Process_FollowCar_SA 0x5245B0, CCam::Process_FollowPed_SA 0x522D40.
- Globals: CGameLogic::n2PlayerPedInFocus i32 0x8A5E50, gLastTime2PlayerCameraWasOK u32 0xB6EC24, gLastTime2PlayerCameraCollided u32 0xB6EC28,
  CWorld::Players (0xB7CD98 = player 0 ped, 0xB7CF28 = player 1 ped), gCurCamColVars u8 0x8CCB80 (W 5), CGameLogic::nPrintFocusHelpTimer 0x96A8B4,
  nPrintFocusHelpCounter 0x96A8B8. Data: 0x8CC5E0 0.8f, 0x8CC5E4 0.1f, 0x8CC5E8 0.1f, 0x8CC5EC 0.02f; 0x858FCC 0.15f, 0x862F58 0.2f, 0x862F5C 0.1f,
  0x858C58 0.01f, 0x858BF4 128.0f, 0x85F3C4 -128.0f, 0x8CC4A0 0.007f.
- Behaviour:
  1. Focus: if `GetPad(0)->F5404A0()`: `n2PlayerPedInFocus = (n == 0) ? 2 : 0`; else if `GetPad(1)->F5404A0()`: `n = (n == 1) ? 2 : 1`.
  2. `n == 0`: player-0 ped `P0`: if in a vehicle (`flags46C & 0x100` and m_pVehicle): `m_pCamTargetEntity = veh; Process_FollowCar_SA(pos(veh), 0, 0, 0, false);
     m_pCamTargetEntity = P0`; else `Process_FollowPed_SA(pos(P0), 0, 0, 0, false)` (target entity not changed). `m_bResetStatics = 0`; return.
     `n == 1`: same with player-1 ped `P1` (vehicle: target = veh, FollowCar, target = P0; on foot: target = P1, FollowPed, target = P0). Return.
  3. `n == 2` (shared camera): `alpha = -0.5235988f; alphaSpeed = 0`. Search: for `i = 0..20`: `k = (i+1)/2` (C signed); `cand = (i odd) ? (float)k*0.15f + beta
     : beta - (float)k*0.15f`; `F5132D0(cand, &src, &out2, &fudge)`; if `F513220(src)`: `gLastTime2PlayerCameraWasOK = now`, stop. If none succeeded (i == 21):
     `cand = beta`. If `i > 0` (including 21): `gLastTime2PlayerCameraCollided = now`. If m_bResetStatics: `beta = cand`.
     `d = wrapPI(cand around beta) - beta; d = clamp(d*0.2f, -0.1f, 0.1f); newB = d + beta; t = 0`.
  4. Drift (only if `i == 0` and `now >= collided + 1000`): `v = P0.mv + P1.mv`; `s2 = (v.z*v.z + v.x*v.x) + v.y*v.y`; if `s2 > 0.01f`: `r = 0.1f*ts; lim = 0.02f*ts;
     h = wrapPI(atan2(-v.x, v.y) - PI/2 around newB); k = (sqrt(s2)*r > 1.0f) ? 1.0f : |v|*r; t = clamp((h - newB)*k, -lim, lim)`; then `cand = (t > 0.01f) ?
     cand + 0.15f : (t == 0.01f ? cand : cand - 0.15f)`; if `s2 <= 0.01f`: `cand = cand - 0.15f`. If `|t| > 0.01f`: `F5132D0(cand, ...)`; if `!F513220(src)`: `betaSpeed = 0` unless (`t > 0` and `betaSpeed > 0`) or
     (`t < 0` and `betaSpeed < 0`); then `gLastTime2PlayerCameraCollided = now; t = 0`.
  5. `nb = wrapPI(t + newB around beta)`; `dt = max(1, ts)`; `rate = (nb - beta)/dt`; `p = pow(0.8f, ts); lim = 0.1f`.
  6. Steering (only if `i == 0` and `now >= collided + 1000`): `a0 = -(int)GetPad(0)->AimWeaponLeftRight(P0); a1 = (int)GetPad(1)->AimWeaponLeftRight(P1)`;
     `s = clamp(a0 - a1, -128, 128)` (floats); `t = ((((FOV*0.0125f)*0.071428575f)*|s|)*0.007f*0.007f)*s`; cand adjusted by +-0.15f exactly as in step 4
     (`t > 0.01` add; `t < 0.01` subtract); if `|t| > 0.01f`: same F5132D0/F513220 test and betaSpeed/collided/t reset as step 4; `x = t + rate`.
     Otherwise `x = rate`.
  7. `x = clamp(x, -lim, lim)`; `betaSpeed = x*(1.0f - p) + p*betaSpeed`; `beta = ts*betaSpeed + beta`; `F5132D0(beta, &source, &front, &fudge)` (writes the
     camera vectors directly).
  8. If `i == 21 && now - gLastTime2PlayerCameraWasOK > 500` (unsigned): `gCurCamColVars = 5`; `hitEnt = 0`; if `ProcessLineOfSight(&fudge, &source, &cp, &hitEnt,
     true, false, false, false, false, true, true, false)`: `source = cp.point`. If `now > nPrintFocusHelpTimer && nPrintFocusHelpCounter < 6`:
     `CHud::SetHelpMessage(TheText.Get("WRN2_2P"), 0, 0, 0); nPrintFocusHelpTimer = now + 60000; nPrintFocusHelpCounter++`.
  9. `up = (0,0,1)`, normalise; `R = cross(up, front)`, normalise; `up = cross(front, R)`, normalise; `FOV = 70.0f; m_bResetStatics = 0`.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:788); nothing to compare.
- Open questions: step 4/6 betaSpeed reset condition is transcribed from the compare chain at 0x52621D-0x526265 / 0x526466-0x5264AE: `betaSpeed = 0` is skipped
  only when (`t > 0` and `betaSpeed > 0`) or (`t < 0` and `betaSpeed < 0`) - verify sign-zero cases against the bytes when porting.

## 0x526FC0 CCam::Process  (catalogue: specified/REA, gr-unreversed, port level 23)
- Convention / signature: `__thiscall void (CCam* this = ECX)`, plain `ret`. The first instruction is `fild [0xB6FE34]` (before the frame);
  frame `sub esp,0x7C` + esi(=this), edi, then ebx, ebp; B = ESP after the four pushes. Locals: `speedVarWanted` B+0x10, `orientation` B+0x14,
  `Tgt` B+0x20..0x28 (the target point passed to the mode functions), `P` B+0x2C..0x34. A .HOODLUM stub at 0x401C8E (`gDWCamMode = -1;
  jmp 0x527D0D`) is part of the body.
- Callers: CCamera::Process 0x52B730.
- Callees: every CCam::Process_* mode function (table below), CEntity::RegisterReference 0x571B70, CWorld::FindPlayerPed 0x56E210,
  CWeaponEffects::ClearCrossHairImmediately 0x742CA0, `M(e)` pair, CGeneral::GetATanOfXY 0x53CC70 (cdecl(float x, float y) -> ST0),
  CVector::Normalise, CCam::Using3rdPersonMouseCam 0x50A850, CPedIntelligence::GetTaskClimb/GetTaskFighting, CTaskSimpleClimb::GetCameraTargetPos
  0x67A390, _CIpow, vector helpers 0x40FE90 (s*v), 0x40FEC0 (v*s), 0x40FE30 (a+b), 0x4119D0 (cdecl(out, v, s): `inv = 1.0f/s; out = v*inv`),
  helper 0x420800 (`a > b ? a : b`), CPad::GetPad / GetLookBehindForCar 0x53FE70 / GetLookLeft 0x53FDD0 / GetLookRight 0x53FE10 /
  GetLookBehindForPed 0x53FEC0, CVehicle::GetVehicleAppearance, CCam::KeepTrackOfTheSpeed 0x509DF0, CCam::LookBehind 0x520690,
  CCam::LookRight 0x520E40, CCam::GetVectorsReadyForRW 0x509CE0.
- Globals: u32 0xB6FE34 (idle counter, read as int), f32 0xB6FDC8 (gIdleCam+0x28), u8 0xC0B184 (bit 0 cleared), TheCamera.m_nActiveCam 0xB6F081,
  m_aCams 0xB6F19C (stride 0x238; +0xC mode), m_pTargetEntity 0xB6F980, byte 0xC8A860, statics `sPedPos` CVector 0x8CCC3C and `sPedVel` CVector
  0xB6EC7C, gCameraDirection i32 0x8CC384, gCameraMode i32 0x8CC388, gDWCamMode i32 0x8CC488, gCurDistForCam f32 0x8CCB84, TheCamera
  m_bTransitionState 0xB6F080, m_bDoingSpecialInterp 0xB6F077, m_bWaitForInterpolToFinish 0xB6F062, m_bJust_Switched 0xB6F052,
  m_bResetOldMatrix 0xB6F999, m_bCamDirectlyBehind 0xB6F042, m_bCamDirectlyInFront 0xB6F043, m_bPlayerIsInGarage 0xB6F04E,
  m_bJustCameOutOfGarage 0xB6F050, m_bUseNearClipScript 0xB6F059, m_bFOVLerpProcessed 0xB6FCDD / m_fFOVNew 0xB6FCE0,
  m_bVecMoveLinearProcessed 0xB6FD14 / m_vecMoveLinear 0xB6FD08, m_bVecTrackLinearProcessed 0xB6FCB4 / m_vecTrackLinear 0xB6FCA8,
  CCamera::m_bUseMouse3rdPerson 0xB6EC2E, u8 0x8CCF00 (initial 1), CPostEffects::m_bSpeedFXUserFlagCurrentFrame 0x8D5109,
  gbFirstPersonRunThisFrame 0xB6EC20. Data: 0x86325C 1.1111112f, 0x863258 0.5555556f, 0x863254 0.10500002f, 0x863250 0.895f,
  0x85EED4 9.0f, 0x8CC394 0.6f, 0x8CC398 0.8f, 0x8CC39C 0.9f.
- Behaviour:
  1. If `!((float)(int)[0xB6FE34] > [0xB6FDC8])`: `byte[0xC0B184] &= 0xFE`. If the active cam's mode != 4: `byte[0xC0B184] &= 0xFE; [0xB6FE34] = 0`.
     `speedVarWanted = 0; orientation = 0`. If `m_pCamTargetEntity == nullptr`: `m_pCamTargetEntity = TheCamera.m_pTargetEntity;
     RegisterReference(&m_pCamTargetEntity)`. If `byte[0xC8A860] == 1` and not (player in a vehicle with model 0x208): `ClearCrossHairImmediately(0)`.
  2. `m_nFrameNumWereAt++`; if `> m_nDoCollisionCheckEveryNumOfFrames`: `= 1`. `m_bCollisionChecksOn = (m_nFrameNumWereAt == m_nDoCollisionChecksOnFrameNum)`.
  3. Target point (if `!m_bCamLookingAtVector`, else `Tgt = m_vecCamFixedModeVector (+0x154)` and skip to 4):
     - Vehicle target: `Tgt = pos(t)`; `orientation = (fwd.x == 0 && fwd.y == 0) ? 0 : GetATanOfXY(fwd.x, fwd.y)` (matrix +0x10);
       `f = (fwd.x, fwd.y, 0)`, normalise, then 2D renormalise (`l = sqrt(f.y*f.y + f.x*f.x)`; if `l != 0`: `inv = 1/l; f.x *= inv; f.y = inv*f.y`);
       `a = f.x*mv.x, b = f.y*mv.y`; `sp = sqrt(b*b + a*a)`; if `b + a > 0`: `speedVarWanted = min(sp*1.1111112f, 1.0f)`; else
       `speedVarWanted = -min(sp*0.5555556f, 0.5f)`; `m_fSpeedVar = speedVarWanted*0.10500002f + m_fSpeedVar*0.895f`. If
       `m_nDirectionWasLooking != 3`: unless `(GetLookBehindForCar() && !GetLookLeft() && !GetLookRight())`, set `m_bCamDirectlyBehind = 1`.
     - The player ped (FindPlayerPed(-1)): `P = pos(player)`; climbing: `climbTask->GetCameraTargetPos(player, &P)`. If `|sPedPos - P|^2 <= 9.0f`
       (computed `(dz*dz + dy*dy) + dx*dx`) and `ts >= 0.2f` and `!Using3rdPersonMouseCam() && !CamDirectlyBehind && !CamDirectlyInFront`:
       - fighting task and `m_nMode == 53`: `p = pow(0.9f, ts)`; `N = p*sPedPos + (1-p)*P` (0x40FE30(out, p*sPedPos, (1-p)*P)); `sPedPos = N; sPedVel = 0`.
       - else: `a = 1 - pow(0.6f, ts); b = 1 - pow(0.8f, ts)`; `pred = sPedPos + ts*sPedVel`; `N = a*P + (1-a)*pred` (add order a*P first);
         `D = (N.x - sPedPos.x, N.y - sPedPos.y, P.z - sPedPos.z)`; `sPedVel = (b*D)*(1/max(1.0f, ts)) + (1-b)*sPedVel` with `sPedVel.z = 0`;
         `sPedPos = (N.x, N.y, P.z)`. `Tgt = (N.x, N.y, P.z)`.
       Otherwise `Tgt = sPedPos = P; sPedVel = 0`.
     - Any other target: `Tgt = pos(t)`.
     For non-vehicle targets: `orientation` from the target's matrix as for vehicles; `speedVarWanted = 0; m_fSpeedVar = 0`.
  4. `m_nDirectionWasLooking = gCameraDirection; gCameraDirection = 3`. If `this` is the active cam: with `ok = !m_bTransitionState`:
     - mode 18/16/22/3 with a vehicle target: `ok = (appearance == 3 || appearance == 5)`; if `GetLookBehindForCar()`: clear m_bTransitionState,
       m_bDoingSpecialInterp, m_bWaitForInterpolToFinish; if `m_nDirectionWasLooking != 0`: `m_bJust_Switched = 1`; `gCameraDirection = 0`. Else
       `dir = (GetLookLeft() && !ok) ? 1 : (GetLookRight() && !ok) ? 2 : 3` (the two look cases also clear the three transition flags);
       if `m_nDirectionWasLooking != dir`: `m_bJust_Switched = 1`; `gCameraDirection = dir`.
     - mode 4 with a ped target: if `GetLookBehindForPed()`: if `m_nDirectionWasLooking != 0 && ok`: `m_bJust_Switched = 1`; `gCameraDirection = 0`.
       Else (and for mode 53): if `m_nDirectionWasLooking != 3`: `gCurDistForCam = 1.0f`; `gCameraDirection = 3`.
  5. If `m_bJust_Switched`: `gCurDistForCam = 1.0f; m_bResetOldMatrix = 1`. If mode not in {3, 18, 22, 16, 49}: `m_bSpeedFXUserFlagCurrentFrame = 0`.
     `gbFirstPersonRunThisFrame = 0`.
  6. Dispatch on `m_nMode` (byte table 0x527F4C + dword table 0x527ED4, index mode-3, 63 entries; args `(&Tgt, orientation, m_fSpeedVar,
     speedVarWanted[, extra])`): 3/18/22 FollowCar_SA(.., false); 4 `m_bUseMouse3rdPerson && !byte[0x8CCF00]` ? FollowPedWithMouse : FollowPed_SA(.., false);
     8 Rocket(.., false); 51 Rocket(.., true); 7/34/45/46 M16_1stPerson; 53/55/65 AimWeapon; 14 WheelCam; 15 Fixed; 35 SpecialFixedForSyphon;
     16 1stPerson; 17 FlyBy; 29 ProcessPedsDeadBaby then `m_bPlayerIsInGarage = m_bJustCameOutOfGarage = 0`; 48 Cam_TwoPlayer; 49 InCarAndShooting;
     50 Separate_Cars; 54 Separate_Cars_TopDown; 44 Editor; 39-43/52 1rstPersonPedOnPC; 32 ProcessArrestCamOne (result ignored); 47 AttachedCam;
     56 DW_HeliChase(false); 57 DW_CamMan(false); 58 DW_Birdy(false); 59 DW_PlaneSpotter(false); 60/61 `m_bUseNearClipScript = 0` only;
     62/63/64 DW_PlaneCam1/2/3(false); 33 nothing; all other modes (and modes < 3 or > 65): `source = (0,0,0); front = (0,1,0); up = (0,0,1)`.
  7. If mode < 56 or > 64: `gDWCamMode = -1`. `gCameraMode = m_nMode`. `d = source - fudge`; `m_fTrueBeta (+0xA0) = GetATanOfXY(d.x, d.y)`;
     `m_fTrueAlpha (+0xA4) = GetATanOfXY(sqrt(d.y*d.y + d.x*d.x), d.z)`. If `!m_bTransitionState`: `KeepTrackOfTheSpeed(source, fudge, up,
     m_fTrueAlpha, m_fTrueBeta, m_fFOV)`.
  8. `m_vecSourceBeforeLookBehind = source; m_bLookingRight = m_bLookingLeft = m_bLookingBehind = 0`. If active cam: `gCameraDirection` 0 ->
     `LookBehind()`, 1 -> `LookRight(false)`, 2 -> `LookRight(true)` (results ignored); `m_nDirectionWasLooking = gCameraDirection`.
  9. Script overrides: `m_bFOVLerpProcessed` -> `FOV = m_fFOVNew`, clear; `m_bVecMoveLinearProcessed` -> `source = m_vecMoveLinear`, clear;
     `m_bVecTrackLinearProcessed` -> `front = m_vecTrackLinear - source`, normalise, `GetVectorsReadyForRW()`, clear.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:518); nothing to compare.
- Open questions: none affecting behaviour (names of 0xB6FE34/0xC0B184/0xC8A860/0x8CCF00 unknown).

## 0x5B25F0 CCam::Process_FlyBy  (catalogue: specified/REA, gr-unreversed, port level 1)
- Convention / signature: `__thiscall void (CCam* this = ECX, const CVector& target, float orientation, float speedVar,
  float speedVarWanted)`, `ret 0x10`; none of the four arguments is read. Frame `sub esp,0x24`, push edi (=this);
  later ebx, ebp, esi. Local `roll` f32 (initialised 0 at 0x5B25FD), local `t` f32.
- Callers: CCam::Process 0x526FC0. Callees: CRT `_ftol2` 0x821B40 (7), unnamed spline helpers 0x5B2090 (vector:
  cdecl(CVector* out, const float* spline, float timeMs, int32* index)) and 0x5B2330 (scalar: cdecl(float* out,
  const float* spline, float timeMs, int32* index)) (both specified by another group), CVector::Normalise 0x59C910,
  CVector::CrossProduct-out 0x59C730 (cdecl(out, a, b) -> a x b, returns out).
- Globals: TheCamera.m_bCutsceneFinished u8 0xB6F045 (R), TheCamera.m_bStartingSpline u8 0xB6F05B (R/W),
  TheCamera.m_fPositionAlongSpline f32 0xB6F168 (W), TheCamera.m_aPathArray[4] (each a `float*` spline buffer) at
  0xB6F988 ([0] FOV), 0xB6F98C ([1] roll), 0xB6F990 ([2] source), 0xB6F994 ([3] look-at), CTimer::ms_fTimeStepNonClipped
  0xB7CB58. Statics (unnamed, .data): `sFov` f32 0xBC4074, `sFov2` f32 0xBC4078 (written once, never read here),
  `idxLook` i32 0xBC407C, `idxPos` i32 0xBC4080, `idxFov` i32 0xBC4084, `idxRoll` i32 0xBC4088.
  Constants: 0.02f 0x858B38, 1000.0f 0x858C4C, 4294967296.0f 0x858C54, PI/2 0x858FE4, 0.017453292f 0x8595EC.
- Fields of `this`: +0x10 m_nFinishTime (u32), +0x90 m_fTimeElapsedFloat, +0xB4 m_fFOV, +0x148 m_vecTargetCoorsForFudgeInter,
  +0x190 m_vecFront (used first as the look-at point), +0x19C m_vecSource, +0x1B4 m_vecUp (+0x1BC = up.z).
- Behaviour:
  1. If `TheCamera.m_bCutsceneFinished`: return.
  2. `up = (0, 0, 1)` (stores x, y, z).
  3. If `!m_bStartingSpline`: `m_fTimeElapsedFloat = 0`; `sp = m_aPathArray[2]`; `n = _ftol2(sp[0])`;
     `m_nFinishTime = _ftol2(sp[10*n - 9] * 1000.0f)` (address `sp + n*40 - 0x24`); `idxRoll = idxFov = 5`;
     `m_bStartingSpline = 1`; `idxPos = idxLook = 11`; `sFov2 = sFov = m_aPathArray[0][2]`.
     Else: `m_fTimeElapsedFloat = (ms_fTimeStepNonClipped * 0.02f) * 1000.0f + m_fTimeElapsedFloat`.
  4. `t = (float)(uint32)_ftol2(m_fTimeElapsedFloat)` (fild of the int, +2^32 if negative); `fin = (float)(uint32)m_nFinishTime`
     (same unsigned conversion). If `t < fin` (`test ah,5; jp` - NaN/>= go to step 6):
     - `TheCamera.m_fPositionAlongSpline = t / fin`.
     - Index advance, done for each spline before its evaluation: `while ((sp[idx] - sp[1]) * 1000.0f <= t) idx += step`
       (the first test is made before the loop; `test ah,0x41; jp` leaves when greater or unordered). The new index is
       stored to the static on each step.
     - Source: `sp = path[2]`, idxPos, step 10; `F5B2090(&source, sp, t, &idxPos)`.
     - Look-at: `path[3]`, idxLook, step 10; `F5B2090(&front, path[3], t, &idxLook)`.
     - Roll: `path[1]`, idxRoll, step 4; `F5B2330(&roll, path[1], t, &idxRoll)`; `a = roll * 0.017453292f + PI/2`;
       `up.x = cos(a); up.z = sin(a)` (same x87 value; up.y stays 0).
     - FOV: `path[0]`, idxFov, step 4; `F5B2330(&sFov, path[0], t, &idxFov)`.
  5. (end of path) else: `idxPos = (_ftol2(path[2][0]) - 1)*10 + 1`; `idxLook = (_ftol2(path[3][0]) - 1)*10 + 1`;
     `idxRoll = _ftol2(path[1][0])*4 - 3`; `idxFov = _ftol2(path[0][0])*4 - 3` (computed in this order); then without the
     advance loops: `F5B2090(&source, path[2], t, &idxPos)`, `F5B2090(&front, path[3], t, &idxLook)`,
     `F5B2330(&roll, path[1], t, &idxRoll)`, up from roll as in step 4, `F5B2330(&sFov, path[0], t, &idxFov)`;
     `m_fPositionAlongSpline = 1.0f`; `idxPos = idxLook = idxRoll = idxFov = 0`.
  6. (0x5B29E0) `m_vecTargetCoorsForFudgeInter = front` (the look-at point); `front = front - source` (x, y, z);
     `front.Normalise()`; `L = cross(up, front)`; `up = cross(front, L)`; `up.Normalise()`; `m_fFOV = sFov`.
- Edge cases: `t` is the truncated elapsed time; when the elapsed float exceeds 2^31 the `_ftol2` result is taken as
  unsigned. The index loops have no upper bound (rely on the spline data). The roll output local is 0 if the helper
  does not write it.
- gta-reversed: `NOTSA_UNREACHABLE()` stub (Cam.cpp:990), `.Reversed = false`; row Cam.cpp:991 not a behavioural difference.
- Open questions: none in this function (the spline helpers' internals belong to their own specs).
