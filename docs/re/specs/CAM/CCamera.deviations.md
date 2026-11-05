# CCamera: gta-reversed deviation notes (CAM, group B)

Source: `re/gta-reversed/source/game_sa/Camera.cpp` (pin 20d8d2a). Layout: see CCamera.md.

## 0x50C1B0 CCamera::SetZoomValueCamStringScript  (gr_deviations = 1)
Row Camera.cpp:896 (`NOTSA_UNREACHABLE("Unexpected zoom mode")` in the switch on `zoomMode`).
- Original bytes (`__thiscall(int16 zoomMode [esp+4])`, `ret 4`): `e = m_aCams[0].m_pCamTargetEntity` (+0x390);
  `if ((e->byte[0x36] & 7) != 2)` -> tail `SetZoomValueFollowPedScript(zoomMode)` (0x50C160) and return.
  Else: `arrPos = 0`; `GetArrPosForVehicleType(e->GetVehicleAppearance() /*0x6D1080*/, &arrPos)` (0x50AF00; result
  ignored, arrPos stays 0 on failure); compare the low 16 bits of zoomMode: 0 -> `m_fCarZoomValueScript (+0xC4) =
  table0[arrPos]` (0x8CC3E0); 1 -> table 0x8CC3F4; 2 -> table 0x8CC408; any other value -> m_fCarZoomValueScript unchanged
  (0x50C231 `jne 0x50C248`). In every vehicle case `m_bUseScriptZoomValueCar (+0x39) = 1`.
  Tables (f32): 0x8CC3E0 = {-1.0, -0.2, -3.2, 0.05, -2.41}, 0x8CC3F4 = {1.0, 1.4, 0.65, 1.9, 6.49},
  0x8CC408 = {6.0, 6.0, 15.9, 15.9, 15.0}.
- Verdict: CONFIRMED (gr aborts / UB for other zoom modes; the original keeps the old value and still sets the flag).
- Class: keep-original.
- Found in passing (CONFIRMED, keep-original): the vehicle test. The original tests `(byte +0x36 & 7) == 2`, i.e.
  `m_nType == ENTITY_TYPE_VEHICLE` (gr `Entity.h:47`: type = bits 0-2, status = bits 3-7). gr tests
  `entity->GetStatus() == STATUS_SIMPLE` (Camera.cpp:884), a different bitfield: gr takes the vehicle path for any entity
  whose status is 2 and the ped path for real vehicles that are not STATUS_SIMPLE. Port: use the type test.

## 0x514B80 CCamera::CalculateGroundHeight  (gr_deviations = 1)
Row Camera.cpp:1641 (`default: NOTSA_UNREACHABLE();` in the switch on `type`).
- Original bytes (`__thiscall float(int32 type [esp+4])`, result in ST0, `ret 4`): after the cached update, 0x514D23:
  `type == 2` -> `fld bbTopZ (0xB7002C)`; `type == 1` -> `fld exactGroundHeight (0xB70030)`; any other value (0 and
  everything else) -> `fld bbBottomZ (0xB70028)`.
- Verdict: CONFIRMED (gr aborts / UB for other values; the original returns bbBottomZ).
- Class: keep-original.
- Rest of the function checked against gr (Camera.cpp:1605-1643): same structure. Details for the port:
  first-call init of `lastCalcCamPos` (0xB70034) to (0,0,0) guarded by bit 0 of 0xB70040 (0x514B80-0x514BAA);
  recalc if any `fabs(lastCalcCamPos.c - camPos.c) > 20.0f` [0x858BA4] (x, then y, then z; NaN does not trigger);
  `CWorld::ProcessVerticalLine({camPos.x, camPos.y, 1000.0f}, -1000.0f, cp, hitEntity, true, false, false, false, true,
  false, nullptr)` (11 cdecl args, 0x514C26-0x514C50); on hit: `exactGroundHeight = cp.point.z`;
  `col = ms_modelInfoPtrs[hit->m_nModelIndex]->m_pColModel` (bounding box min +0x0, max +0xC);
  `bbTopZ = col.max.z + hitPos.z`; `bbBottomZ = (col.max.x - col.min.x > 120.0f || col.max.y - col.min.y > 120.0f) ?
  exactGroundHeight : hitPos.z + col.min.z`; then `if (bbBottomZ < 0.0f) bbBottomZ = 0` (NaN kept - gr's `std::max(0.f, x)`
  would return 0 for NaN, harmless). `lastCalcCamPos = camPos` whether or not the line hit.

## 0x5BC520 CCamera::Init  (gr_deviations = 1)
Row Camera.cpp:220 (`m_fMouseAccelVertical = IsFixBugs() ? m_fMouseAccelHorzntl * 0.6f : 0.0015f`).
- Original bytes: inside `if (FrontEndMenuManager byte 0xBA67A5 == 0)` (gr `m_bStartGameLoading`), 0x5BC7B4 stores the
  constant `0x3AC49BA6` (0.0015f) into `CCamera::m_fMouseAccelVertical` 0xB6EC18.
- Verdict: CONFIRMED (vanilla constant; gr's FixBugs path differs).
- Class: keep-original.

Found in passing (CONFIRMED, keep-original): gr's `CCamera::Init` (Camera.cpp:169-235, marked reversed) is much shorter
than the original. Complete original sequence (`__thiscall void()`, plain `ret`; callers CGame::ReInitGameObjectVariables
0x53BCF0, CGame::InitialiseRenderWare 0x5BD600):
1. `memset(this + 0x18, 0, 0x358 * 4)` (`rep stosd`, reached through an obfuscation jump to 0x406E9A that loads ECX = 0x358,
   0x5BC529-0x5BC535): zeroes every CCamera field after CPlaceable (+0x18..+0xD77). **gr has no such clear.**
2. `m_pRwCamera (+0x954) = 0`; `InitialiseScriptableComponents()` (0x50D2D0) called with **ECX = &TheCamera (0xB6F028),
   not `this`** (0x5BC539); identical for the only instance, but a port must call it on the global.
3. `m_bPlayerWasOnBike (+0x27) = 0; m_b1rstPersonRunCloseToAWall (+0x3E) = 0; m_fPositionAlongSpline (+0x140) = 0;
   m_bCameraJustRestored (+0x1C) = 0;` then `CCam::Init` (0x50E490) on m_aCams[0], [1], [2] (+0x174, +0x3AC, +0x5E4).
4. `m_aCams[0].m_nMode = m_aCams[1].m_nMode = 4` (+0x180, +0x3B8); `m_aCams[0].m_fTargetCloseInDist (+0x274) =
   2.08378f [0x86A46C] - 1.85f [0x86A468]` (x87, f32 store); `m_bEnable1rstPersonCamCntrlsScript (+0x41) = 0;
   m_bAllow1rstPersonWeaponsCamera (+0x42) = 0; m_bCooperativeCamMode (+0x43) = 0; m_bAllowShootingWith2PlayersInCar (+0x44) = 1;
   m_bVehicleSuspenHigh (+0x40) = 0;` cam0: `m_fMinRealGroundDist (+0x270) = 1.85f`, +0x250 = -0.55f (0xBF0CCCCD),
   +0x254 = 1.5f, +0x258 = 3.6f (0x40666666), +0x25C = 0.06f (0x3D75C28F), +0x260 = -0.1f (0xBDCCCCCD), +0x264 = 0,
   +0x268 = -0.07f (0xBD8F5C29), +0x26C = 0.900407f (0x3F668113); `m_bMoveCamToAvoidGeom (+0x2D) = 0`.
5. `ClearPlayerWeaponMode()` (0x50AB10).
6. `m_bInATunnelAndABigVehicle (+0x22) = 0`; for cams 0/1/2: `m_pCamTargetEntity = 0` (+0x390, +0x5C8, +0x800),
   `m_bCamLookingAtVector = 0`, `m_fPlayerVelocity = 0`; cams 0/1: `m_fCamBufferedHeight = m_fCamBufferedHeightSpeed = 0`.
7. `m_bHeadBob (+0x3F) = 0; m_fFractionInterToStopMoving (+0xC10) = 0.25f; m_fFractionInterToStopCatchUp (+0xC14) = 0.75f;
   m_fGaitSwayBuffer (+0xC20) = 0.85f (0x3F59999A); m_bScriptParametersSetForInterp (+0x50) = 0; m_nCamShakeStart (+0x5C) = 0;
   m_fCamShakeForce (+0x12C) = 0; m_nModeObbeCamIsInForCar (+0xC34) = 30; m_bIgnoreFadingStuffForMusic (+0x25) = 0;
   m_bWaitForInterpolToFinish (+0x3A) = 0; m_pToGarageWeAreIn (+0x81C) = 0; m_pToGarageWeAreInForHackAvoidFirstPerson (+0x820) = 0;
   m_bPlayerIsInGarage (+0x26) = 0; m_bJustCameOutOfGarage (+0x28) = 0; m_fNearClipScript (+0x138) = 0.9f (0x3F666666);
   m_bUseNearClipScript (+0x31) = 0; m_bDoingSpecialInterp (+0x4F) = 0; m_bAboveGroundTrainNodesLoaded (+0x18) = 0;
   m_bBelowGroundTrainNodesLoaded (+0x19) = 0;` two-player modes: +0x46 = 50, +0x48 = 49, +0x4A = 3, +0x4C = 48 (u16);
   `m_bWideScreenOn (+0x3D) = 0; m_fFOV_Wide_Screen (+0x134) = 0; m_bRestoreByJumpCut (+0x30) = 0; m_nCarZoom (+0xB4) = 2;
   m_nPedZoom (+0xC8) = 2;` Car/Ped zoom base/total/smoothed (+0xB8..+0xC0, +0xCC..+0xD4) = 0.
8. `m_pTargetEntity (+0x958) = 0`, then `= FindPlayerVehicle(-1, false) ? FindPlayerVehicle(-1, false) :
   Players[PlayerInFocus].m_pPed`; if non-null `RegisterReference(&m_pTargetEntity)` (no CleanUpOldReference: the memset
   already dropped the old pointer, so an old registration is leaked - original behaviour).
9. `m_bInitialNodeFound (+0x23) = 0; m_fScreenReductionPercentage (+0x144) = 0; m_fScreenReductionSpeed (+0x148) = 0;
   m_bWideScreenOn = 0; m_bWantsToSwitchWidescreenOff (+0x3C) = 0; m_bWorldViewerBeingUsed (+0x56) = 0;
   m_fPlayerExhaustion (+0x154) = 1.0f; m_fPedOrientForBehindOrInFront (+0x120) = 0`.
10. If byte 0xBA67A5 == 0: `m_bFading (+0x51) = 0; CDraw::FadeValue (0xC3EFAB) = 0; m_fFadeAlpha (+0xBFC) = 0;
    m_bMusicFading (+0x52) = 0; m_fTimeToFadeMusic (+0xC08) = 0; m_fEffectsFaderScalingFactor (+0xC00) = 0;
    m_fMouseAccelVertical = 0.0015f`. Then if byte 0xBA67A5 == 1: `m_fTimeToFadeMusic = 0`.
11. `m_bStartingSpline (+0x33) = 0; m_nTypeOfSwitch (+0xC3C) = 1; m_bUseScriptZoomValuePed (+0x38) = 0;
    m_bUseScriptZoomValueCar (+0x39) = 0; m_fPedZoomValueScript (+0xD8) = 0; m_fCarZoomValueScript (+0xC4) = 0;
    m_bUseSpecialFovTrain (+0x36) = 0; m_fFovForTrain (+0x130) = 70.0f; m_nModeToGoTo (+0xC38) = 4; m_bJust_Switched (+0x2A) = 0;
    m_bUseTransitionBeta (+0x37) = 0;` `m_mCameraMatrix (+0x974).SetScale(1.0f)` (0x59AED0);
    `m_bTargetJustBeenOnTrain (+0x34) = 0; m_bInitialNoNodeStaticsSet (+0x24) = 0; m_nLongestTimeInMill (+0x64) = 5000;
    m_nTimeLastChange (+0x6C) = 0; m_bIdleOn (+0x21) = 0; m_nTimeWeLeftIdle_StillNoInput (+0x70) = 0; m_nTimeWeEnteredIdle (+0x74) = 0;
    m_fLODDistMultiplier (+0xF0) = 1.0f; m_bCamDirectlyBehind (+0x1A) = 0; m_bCamDirectlyInFront (+0x1B) = 0; m_nMotionBlur (+0xA0) = 0;
    m_bGarageFixedCamPositionSet (+0x4E) = 0;` `SetMotionBlur(255, 255, 255, 0, 0)` (0x50BF40);
    `m_bCullZoneChecksOn (+0x1E) = 0; m_bFailedCullZoneTestPreviously (+0x54) = 0; m_nCheckCullZoneThisNumFrames (+0xA8) = 6;
    m_nZoneCullFrameNumWereAt (+0xAC) = 0; m_fCameraAverageSpeed (+0x124) = 0; m_fCameraSpeedSoFar (+0x128) = 0;
    m_vecPreviousCameraPosition (+0x83C) = (0,0,0); m_nWorkOutSpeedThisNumFrames (+0x94) = 4; m_nNumFramesSoFar (+0x98) = 0;
    m_bJustInitialized (+0x29) = 1; m_bTransitionState (+0x58) = 0; m_nTimeTransitionStart (+0x78) = 0; m_bLookingAtPlayer (+0x2B) = 1;
    m_bLookingAtVector (+0x2C) = 0; m_f3rdPersonCHairMultX (0xB6EC14) = 0.53f (0x3F07AE14); m_f3rdPersonCHairMultY (0xB6EC10) = 0.4f;
    m_fAvoidTheGeometryProbsTimer (+0x164) = 0; m_nAvoidTheGeometryProbsDirn (+0x168) = 0 (u16); gPlayerPedVisible (0x8CC380) = 1;
    m_bResetOldMatrix (+0x971) = 1`.
gr only performs steps 2-6 partially, 8, 10 (FadeValue and mouse accel only), SetMotionBlur, the crosshair multipliers,
gPlayerPedVisible and m_bResetOldMatrix; everything else (including the memset) is missing, so a re-initialisation in gr
keeps stale state.

