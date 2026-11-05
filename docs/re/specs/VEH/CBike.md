# CBike (VEH) - full specs

Group D, RE-SPEC-2. Authority: the disassembly of gta_sa.exe 1.0 US (instruction addresses cited). gta-reversed (pin in
re/gta-reversed) has only `plugin::Call` stubs for almost all of these functions; where it has C++ it is noted per function.

Layout used (derived from gr `Bike.h`/`Vehicle.h`, confirmed by the accesses below; CVehicle size 0x5A0, CBike size 0x814):
CVehicle: +0x384 m_pHandlingData, +0x388 m_pFlyingHandlingData, +0x38C handling flags, +0x390 m_autoPilot (0x98 bytes; +0x3BA
m_nCarMission, +0x3C8 m_speed, +0x3D0 m_nCruiseSpeed, +0x3DB m_nCarCtrlFlags, +0x41C m_TargetEntity), +0x428/+0x42C vehicle flags
(upper/lower dwords; bit numbers as in gr), +0x45C m_nAlarmState, +0x460 m_pDriver, +0x464.. m_apPassengers[8], +0x48C m_pEntityWeAreOn,
+0x494 m_fSteerAngle, +0x49C m_GasPedal, +0x4A0 m_BrakePedal, +0x4A6 m_nExtendedRemovalRange, +0x4A8 bomb/lights bitfield, +0x4B4
m_nCurrentGear, +0x4C0 m_fHealth, +0x4D8 m_nTimeWhenBlowedUp, +0x4DE m_wBombTimer, +0x508 m_nLastWeaponDamageType, +0x50C
m_pLastDamageEntity, +0x514 m_HornCounter (u32; confirmed by CBike::ReduceHornCounter 0x6B5F90 - gr's eCarWeapon at +0x513 is 1 byte),
+0x519 m_nCarHornTimer (u8), +0x51C m_FrontCollPoly, +0x548 m_RearCollPoly (CStoredCollPoly 0x2C, lighting at +0x28), +0x574
m_anCollisionLighting[4], +0x590 m_nVehicleType, +0x594 m_nVehicleSubType.
CBike: +0x5A0 m_aBikeNodes[10], +0x5C8 m_bLeanMatrixCalculated, +0x5CC m_mLeanMatrix, +0x614 nBikeFlags (0x01 bShouldNotChangeColour,
0x02 bPanelsAreThougher, 0x04 bWaterTight, 0x08 bGettingPickedUp, 0x10 bOnSideStand, 0x20 bPlayerBoost, 0x40 bEngineOnFire,
0x80 bWheelieForCamera), +0x618 m_vecAveGroundNormal, +0x624 m_vecGroundRight, +0x630 m_vecOldSpeedForPlayback, +0x63C m_BikeHandling,
+0x640 m_RideAnimData (+0 AnimGroup, +4 BarSteerAngle, +8 LeanAngle, +0xC DesiredLeanAngle, +0x10 LeanFwd, +0x14 AnimLeanLeft,
+0x18 AnimLeanFwd), +0x65C m_nWheelStatus[2], +0x660 m_aWheelColPoints[4] (CColPoint 0x2C: +0x23 surfaceB, +0x25 lightingB),
+0x710 m_aWheelRatios[4], +0x720 m_aRatioHistory[4], +0x730 m_WheelCounts[4], +0x740 m_fBrakeCount, +0x744 m_aWheelSkidmarkType[2],
+0x74C m_bWheelBloody[2], +0x74E m_bMoreSkidMarks[2], +0x750 m_aWheelPitchAngles[2] (wheel rotation), +0x758 m_aWheelAngularVelocity[2],
+0x760 m_aWheelSuspensionHeights[2], +0x768 m_aWheelOrigHeights[2], +0x770 m_fSuspensionLength[4], +0x780 m_fLineLength[4],
+0x790 m_fHeightAboveRoad, +0x794 m_fExtraTractionMult, +0x798 m_fSwingArmLength, +0x79C m_fForkYOffset, +0x7A0 m_fForkZOffset,
+0x7A4 m_fSteerAngleTan, +0x7A8 m_nBrakesOn, +0x7AC m_fTyreTemp, +0x7B0 m_fBrakingSlide, +0x7B4 m_nFixLeftHand, +0x7B5 m_nFixRightHand,
+0x7B6 m_nTestPedCollision, +0x7B8 m_PrevSpeed, +0x7BC m_BlowUpTimer, +0x7C0 m_aGroundPhysicalPtrs[4], +0x7D0 m_aGroundOffsets[4],
+0x800 m_Damager, +0x804 m_nNoOfContactWheels, +0x805 m_NumDriveWheelsOnGround, +0x806 ..LastFrame, +0x808 m_GasPedalAudioRevs,
+0x80C m_WheelStates[2].
Shared CRT helpers: `_ftol` 0x821B40 (truncation), `rand` 0x821B1E, `floor` 0x8219F0 (double in, ST0 out), `_CIpow` 0x822130.
x87 compare idioms used below: `fcomp; test ah,5; jp X` jumps when NOT (st0 < src) (NaN jumps); `test ah,0x41; je X` jumps when st0 > src;
`test ah,0x41; jne X` jumps when st0 <= src or NaN; `test ah,0x44; jp X` jumps when st0 != src or NaN.

## 0x6B58A0 CBike::GetNumContactWheels  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall int32 ()` (ECX = CBike*), plain `ret`. Callers: CTaskComplexLeaveCar::MakeAbortable (virtual).
- Behaviour: `return (uint8)this->m_nNoOfContactWheels` (+0x804), zero-extended into EAX (`movzx` 0x6B58A0).
- gta-reversed: inline in Bike.h, matches.

## 0x6B58B0 CBike::GetHeightAboveRoad  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall float ()`, result in ST0 (`fld dword [ecx+0x790]`, 0x6B58B0), plain `ret`.
- Behaviour: `return this->m_fHeightAboveRoad` (+0x790, f32).
- Callers: CCarCtrl::CreatePoliceChase/CreateConvoy/GenerateOneRandomCar, CPhysical::ProcessCollision, CBike::PlaceOnRoadProperly.
- gta-reversed: inline, matches.

## 0x6B58C0 CBike::GetRideAnimData  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall CRideAnimData* ()`, EAX, plain `ret`.
- Behaviour: `return &this->m_RideAnimData` (`lea eax,[ecx+0x640]`, 0x6B58C0).
- gta-reversed: inline, matches.

## 0x6B58E0 CBike::IsDoorReady  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall bool (eDoors door [esp+4])`, `ret 4`; result in AL only (`mov al,1` at 0x6B58E0; the upper EAX bytes are not defined by this function).
- Behaviour: ignores the argument and returns `true`.
- gta-reversed: inline in Bike.h, matches.

## 0x6B58F0 CBike::IsDoorFullyOpen  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall bool (eDoors door [esp+4])`, `ret 4`; result in AL only (`xor al,al` at 0x6B58F0; the upper EAX bytes are not defined by this function).
- Behaviour: ignores the argument and returns `false`.
- gta-reversed: inline in Bike.h, matches.

## 0x6B5900 CBike::IsDoorClosed  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall bool (eDoors door [esp+4])`, `ret 4`; result in AL only (`xor al,al` at 0x6B5900; the upper EAX bytes are not defined by this function).
- Behaviour: ignores the argument and returns `false`.
- gta-reversed: inline in Bike.h, matches.

## 0x6B5910 CBike::IsDoorMissing  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall bool (eDoors door [esp+4])`, `ret 4`; result in AL only (`mov al,1` at 0x6B5910; the upper EAX bytes are not defined by this function).
- Behaviour: ignores the argument and returns `true`.
- gta-reversed: inline in Bike.h, matches.

## 0x6B5920 CBike::IsDoorReadyU32  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall bool (uint32 door [esp+4])`, `ret 4`; result in AL only (`mov al,1` at 0x6B5920; the upper EAX bytes are not defined by this function).
- Behaviour: ignores the argument and returns `true`.
- gta-reversed: inline in Bike.h, matches.

## 0x6B5930 CBike::IsDoorFullyOpenU32  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall bool (uint32 door [esp+4])`, `ret 4`; result in AL only (`xor al,al` at 0x6B5930; the upper EAX bytes are not defined by this function).
- Behaviour: ignores the argument and returns `false`.
- gta-reversed: inline in Bike.h, matches.

## 0x6B5940 CBike::IsDoorClosedU32  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall bool (uint32 door [esp+4])`, `ret 4`; result in AL only (`xor al,al` at 0x6B5940; the upper EAX bytes are not defined by this function).
- Behaviour: ignores the argument and returns `false`.
- gta-reversed: inline in Bike.h, matches.

## 0x6B5950 CBike::IsDoorMissingU32  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall bool (uint32 door [esp+4])`, `ret 4`; result in AL only (`mov al,1` at 0x6B5950; the upper EAX bytes are not defined by this function).
- Behaviour: ignores the argument and returns `true`.
- gta-reversed: inline in Bike.h, matches.

## 0x6B59E0 CBike::IsComponentPresent  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall bool (int32 component [esp+4])`, `ret 4`, result AL (`setne al`, 0x6B59ED; upper EAX bytes hold the low bits of `component`).
- Behaviour: `return this->m_aBikeNodes[component] != nullptr` (dword at `this + 0x5A0 + component*4`, 0x6B59E4). No bounds check: any
  index is read as-is (out-of-range reads continue into the rest of the object).
- gta-reversed: no separate implementation hooked at this address in Bike.cpp (absent); behaviour above is from the bytes.

## 0x6B5F50 CBike::SetRemoveAnimFlags  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (CPed* ped [esp+4])`, `ret 4`. `this` (ECX) is not used (ECX is clobbered by the callees).
  No callers found statically (catalogue lists 0).
- Callees: RpAnimBlendClumpGetFirstAssociation(clump, flags) 0x4D6A70 and RpAnimBlendGetNextAssociation(assoc, flags) 0x4D6AD0
  (both cdecl, caller pops 8).
- Behaviour: if `(ped->m_nType & 7) != 3` (byte +0x36 low 3 bits; 3 = ENTITY_TYPE_PED; 0x6B5F54-0x6B5F5D) return.
  `a = GetFirstAssociation(ped->m_pRwClump (+0x18), 0x1000)`; while `a`: `byte [a+0x2E] |= 4` (u16 `m_nFlags` at +0x2E,
  bit 0x0004 = ANIMATION_IS_BLEND_AUTO_REMOVE), the association list is walked with flag filter 0x1000
  (ANIMATION_SECONDARY_TASK_ANIM), `a = GetNextAssociation(a, 0x1000)` (0x6B5F74-0x6B5F88).
- gta-reversed: plugin::Call (no C++).

## 0x6B6740 CBike::ResetSuspension  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall void ()`, plain `ret`. Caller: CBike::Teleport 0x6BCFC0.
- Behaviour (two loops, in this order):
  1. for i = 0..1: `m_aWheelPitchAngles[i]` (+0x750 + 4i, gr name; it is the wheel rotation angle) = 0 (int store of 0, 0x6B6750);
     `m_WheelStates[i]` (+0x80C + 4i) = 0 (0x6B675A).
  2. for i = 0..3: `m_aWheelRatios[i]` (+0x710 + 4i) = 1.0f (bits 0x3F800000, 0x6B6780); `m_WheelCounts[i]` (+0x730 + 4i) = 0 (0x6B6783).
- gta-reversed: plugin::Call to 0x6B6740 (no C++).

## 0x6B67E0 CBike::SetUpWheelColModel  (catalogue: specified/REA, port level 2)
- Convention: virtual `__thiscall bool (CColModel* wheelCol [esp+4])`, `ret 4`, returns AL = 1 always (0x6B693D). The arg slot
  is reused to hold the model-info pointer.
- Callees: CEntity::GetColModel 0x535300, RwMatrixTransform 0x7F25A0 (cdecl(RwMatrix* m, const RwMatrix* t, 2 = rwCOMBINEPOSTCONCAT)),
  CColSphere::Set 0x40FD10 (thiscall(float radius, const CVector& center, uint8 surface, uint8 piece, uint8 lighting), callee pops 0x14).
- Globals: CModelInfo::ms_modelInfoPtrs 0xA9B0C8. Constant 0x858B8C = 0.5f.
- Fields: `m_aBikeNodes[1]` (+0x5A4, BIKE_CHASSIS frame), `[4]` (+0x5B0, WHEEL_FRONT), `[5]` (+0x5B4, WHEEL_REAR);
  CVehicleModelInfo +0x40/+0x44 wheel sizes; CColModel: bbox +0x00 (0x18 bytes), sphere +0x18 (0x10 bytes), m_pColData +0x2C;
  CCollisionData: u16 m_nNumSpheres +0x00, m_pSpheres +0x08 (CColSphere stride 0x14); RwFrame: parent +0x04, modelling matrix +0x10.
- Behaviour:
  1. `cd = wheelCol->m_pColData` (read before anything else, 0x6B67FE); `cm = GetColModel()`;
     copy `cm->m_boundSphere` (16 bytes, +0x18) then `cm->m_boundBox` (24 bytes, +0x00, `rep movsd`) into `wheelCol`.
  2. Front: `M = wheelFrontFrame->modelling` (64-byte copy); `p = wheelFrontFrame->parent`; loop (0x6B6850-0x6B6870):
     `while (p) { RwMatrixTransform(&M, &p->modelling, 2); p = p->parent; if (p == chassisFrame) break; }`.
     The first parent is always applied even if it is the chassis frame itself (the chassis test happens after the transform).
     `cd->m_pSpheres[0].Set(mi->m_fWheelSizeFront * 0.5f, M.pos, 0x43 /*SURFACE_RUBBER*/, 0x0D, 0xFF)`.
  3. Rear: same walk starting at `m_aBikeNodes[5]`; `cd->m_pSpheres[1].Set(mi->m_fWheelSizeRear * 0.5f, M.pos, 0x43, 0x0F, 0xFF)`.
  4. `cd->m_nNumSpheres = 2` (word store, 0x6B6936); return true.
- Edge cases: no null checks on the frames, `cd` or `m_pSpheres`.
- gta-reversed: plugin::Call (no C++). (CAutomobile's version uses `/ 2.0f`; here it is `* 0.5f` - same value.)

## 0x6B6950 CBike::DoBurstAndSoftGroundRatios  (catalogue: specified/REA, port level 1)
- Convention: virtual `__thiscall void ()`, plain `ret` (aligned frame `and esp,-8`). Callers: CBike::PreRender 0x6BD090,
  CBmx::PreRender 0x6C0810.
- Callees: `rand` 0x821B1E, `_ftol` 0x821B40 (x2, truncation), `floor` 0x8219F0 (x2, double in / ST0 out),
  SurfaceInfos_c::GetAdhesionGroup 0x55E5C0 (thiscall on g_surfaceInfos 0xB79538).
- Globals: CModelInfo::ms_modelInfoPtrs 0xA9B0C8, CTimer::ms_fTimeStep 0xB7CB5C, CWeather::WetRoads 0xC81308.
  Constants: 0x871284 = 1.0f, 0x858624 = 1.0f, 0x858B14 = 1/32768 (3.0517578e-05f), 0x858A10 = 40.0f, 0x871310 = 0.2f,
  0x858B8C = 0.5f, 0x871328 = 1.5f, 0x87132C = 0.3f, 0x858B50 = 0.0f, 0x871324 = 0.3f, 0x858CC4 = 0.2f, 0x871314 = 0.25f,
  0x87131C = 0.1f, 0x871318 = 0.15f, 0x871320 = 0.3f, 0x858CB0 = 0.7f, 0x858EE8 = 0.4f.
- Fields: `m_nWheelStatus[2]` +0x65C (u8: 0 OK, 1 BURST, 2 MISSING), `m_aWheelRatios[4]` +0x710, surfaceB of colpoint i at +0x683 + i*0x2C,
  `m_aWheelPitchAngles[2]` +0x750 (wheel rotation), `m_aWheelAngularVelocity[2]` +0x758, `m_fSuspensionLength[4]` +0x770,
  `m_fLineLength[4]` +0x780, `m_nHandlingFlagsIntValue` +0x38C, `m_nModelIndex` +0x22, move speed +0x44, matrix forward +0x10.
  CVehicleModelInfo `m_fWheelSizeFront` +0x40, `m_fWheelSizeRear` +0x44.
- Behaviour:
  0. Locals: `soft[4] = {1,1,1,1}` (bytes); `spd = fabs((ms.z*fwd.z + ms.y*fwd.y) + ms.x*fwd.x)` stored f32 (0x6B6962-0x6B69A8);
     `mi = ms_modelInfoPtrs[m_nModelIndex]`.
  1. For wheel `i` = 0, 1 (lines `a,b` = 0,1 for i=0 and 2,3 for i=1; 0x6B69B2):
     - status 2 (MISSING): `ratio[a] = 1.0f; ratio[b] = 1.0f`; then `soft[a] = soft[b] = 0`.
     - status 1 (BURST) (0x6B69F3-0x6B6A99): `r = (float)(rand() & 0xFFFF) * (1/32768)` (fild, fmul f32);
       `n = _ftol(spd * 40.0f)`; `m = (uint16)n + 98` (zero-extend low 16 bits of the truncated int, 0x6B6A1A);
       `q = _ftol(r * (float)m)` (`fimul` with the int32); if `q < 100` (signed, 0x6B6A2D):
       `t = (lineLen[a] - suspLen[a]) / lineLen[a] * 0.2f`;
       `ratio[a] = min1(t + ratio[a])`, `ratio[b] = min1(t + ratio[b])` where `min1(v) = (v < 1.0f) ? v : 1.0f` (NaN -> 1.0f;
       `fcom; test ah,5; jnp`). Both use `t` computed from line `a`. In both outcomes then `soft[a] = soft[b] = 0`.
     - otherwise (status 0 or any other value) (0x6B6A9E): rail bump test. Condition: `(ratio[a] < 1.0f && surfaceB[a] == 0xB2)`
       OR `(ratio[b] < 1.0f && surfaceB[b] == 0xB2)` (178 SURFACE_RAILTRACK; NaN ratio = false). If false: go to next wheel WITHOUT
       clearing `soft[a]/soft[b]`. If true (0x6B6AEC):
       - `w = (i == 0) ? mi->m_fWheelSizeFront : mi->m_fWheelSizeRear`; `v = 1.5f / (w * 0.5f)` (`fmul 0.5; fdivr 1.5`);
         if `spd > 0.3f` (0x87132C; `<=`/NaN skip): `v = v * (spd / 0.3f)`.
       - `per = 1.0f / v` stored f32; `x1 = per * rot[i]` stored f32; `f1 = x1 - floor(x1)` stored f32;
         `x2 = (ms_fTimeStep * angVel[i] + rot[i]) * per` stored f32; `f2 = x2 - floor(x2)` (kept in ST0).
       - bump if `(angVel[i] > 0.0f && f2 < f1)` or `(angVel[i] < 0.0f && f2 > f1)` (the second test is only reached when the first
         fails; both compares are ordered, NaN = no bump; 0x6B6B79-0x6B6BB8).
       - bump (0x6B6BBE): `t = (lineLen[a] - suspLen[a]) / lineLen[a] * 0.3f` (0x871324); `ratio[a] = max02(ratio[a] - t)`,
         `ratio[b] = max02(ratio[b] - t)` with `max02(v) = (v > 0.2f) ? v : 0.2f` (NaN -> 0.2f; `test ah,0x41; je`).
       - bump or no bump: `soft[a] = soft[b] = 0` (0x6B6C25).
  2. For line `j` = 0..3 (0x6B6C50-0x6B6D2D): only if `soft[j] != 0` AND `ratio[j] < 1.0f` AND
     `GetAdhesionGroup(surfaceB[j]) == 4` (SAND) AND `m_nModelIndex != 0x1B0` (432, compared as a 16-bit word):
     - `k = 0.25f`; if `handlingFlags & 0x200000` (bOffroadAbility2) `k = 0.1f`, else if `& 0x100000` (bOffroadAbility) `k = 0.15f`.
     - `s = (1.0f - (spd / 0.3f) * 0.7f) - WetRoads * 0.7f`; `if (0.4f > s) s = 0.4f` (NaN keeps s; 0x6B6CE2-0x6B6CF3).
     - `u = (lineLen[j] - suspLen[j]) / lineLen[j]`; `ratio[j] = ratio[j] + k * (u * s)` (u*s first, then times k, then add; 0x6B6D02-0x6B6D08),
       stored f32; if the stored value `> 1.0f` (ordered) it is replaced by 1.0f (integer copy of 0x3F800000).
     The surface/adhesion lookup is only done after the ratio test passes (short-circuit order: soft, ratio, adhesion, model).
- Edge cases: so the sand softening only applies to lines whose wheel was "OK" and not on a rail track bump test path (a wheel that
  passed the rail-track condition clears soft for both its lines whether or not it bumps). Model 432 (Rhino) is excluded even though
  this is a bike function (inherited check; keep).
- gta-reversed: plugin::Call (no C++).

## 0x6B6D40 CBike::DoSoftGroundResistance  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall void (uint32& extraHandlingFlags [esp+4])`, `ret 4`. Caller: CBike::ProcessAI 0x6BC930.
  The argument stack slot is reused as a float temp after the pointer was consumed (irrelevant for a port).
- Callees: SurfaceInfos_c::GetAdhesionGroup 0x55E5C0 (thiscall on g_surfaceInfos 0xB79538, (uint32 surface) -> int32),
  CPhysical::ApplyMoveForce(CVector) 0x5429F0 (thiscall, vector by value, 12 bytes pushed).
- Globals: CTimer::ms_fTimeStep 0xB7CB5C, CVehicle::ms_fRailTrackResistance 0x8D34AC (f32, initial 0.003f).
  Constants: 0x871284 = 1.0f, 0x858C24 = 0.3f, 0x871320 = 0.3f, 0x871330 = 0.02f.
- Fields: `m_aWheelRatios[4]` +0x710 (f32), `m_aWheelColPoints[i].m_nSurfaceTypeB` +0x683 + i*0x2C (bytes at 0x683/0x6AF/0x6DB/0x707),
  `m_vecMoveSpeed` +0x44, `m_fMass` +0x8C, `m_GasPedal` +0x49C, matrix (+0x14): forward +0x10, up +0x20.
- Behaviour. "Line i touches" below means `ratio[i] < 1.0f` (ordered less; NaN = not touching; `fcomp; test ah,5; jp`).
  1. Sand test, wheels i = 0,1,2,3 in order, short-circuit (0x6B6D46-0x6B6DF4): if any line touches AND
     `GetAdhesionGroup(surfaceB[i]) == 4` (ADHESION_GROUP_SAND) -> sand block (step 2). GetAdhesionGroup is only called for touching
     lines. If none -> step 3.
  2. Sand block (0x6B6DFA-0x6B6F29):
     - `d = (ms.z*up.z + ms.y*up.y) + ms.x*up.x` (x87, z-term first); `t.x = d*up.x`, `t.y = d*up.y`, `t.z = d*up.z` each stored f32;
       `h.x = ms.x - t.x`, `h.y = ms.y - t.y` (kept on the x87 stack), `h.z = ms.z - t.z` (stored f32).
     - If `m_GasPedal > 0.3f` (0x6B6E45; `<=`/NaN skip):
       - `s = (h.z*h.z + h.y*h.y) + h.x*h.x`; if `0.3f*0.3f > s` (0.09 computed as `fld 0.3; fmul 0.3`, then `fcompp` with s; 0x6B6E6C-0x6B6E7F)
         then `extraHandlingFlags += 4` (an ADD, `add dword [eax],4` 0x6B6E85, not an OR).
       - `e = (h.z*fwd.z + h.y*fwd.y) + h.x*fwd.x`; `h.x -= f32(e*fwd.x)`; `h.y -= f32(e*fwd.y)`; `h.z = h.z - e*fwd.z` (this last
         product is not stored to memory first, 0x6B6EC5) and stored f32. Removes the forward component.
     - `k = -(ms_fTimeStep * m_fMass * 0.02f)` (in that multiply order, then `fchs`; 0x6B6ED1-0x6B6EE8), stored f32.
     - `ApplyMoveForce(CVector(k*h.x, h.y*k, h.z*k))` (each product stored f32; 0x6B6EEE-0x6B6F20). Return.
  3. Rail test (0x6B6F2C-0x6B6F9C), wheels 0..3 short-circuit: line touches AND `surfaceB[i] == 0xB2` (178, SURFACE_RAILTRACK).
     None -> return.
  4. Rail block (0x6B6FA2-0x6B7038): `d` as in 2; `t.y = f32(d*up.y)`, `t.z = f32(d*up.z)`, `h.x = ms.x - d*up.x` (product not
     stored first), `h.y = ms.y - t.y`, `h.z = ms.z - t.z` (stored f32);
     `k = -(ms_fTimeStep * m_fMass * ms_fRailTrackResistance)` stored f32; `ApplyMoveForce(CVector(h.x*k, h.y*k, h.z*k))`. Return.
     The rail block does not touch `extraHandlingFlags` and has no gas-pedal branch.
- Edge cases: sand has priority: the rail test only runs if no touching line is on sand. Lines are tested per index (front lines 0,1,
  rear lines 2,3).
- gta-reversed: plugin::Call (no C++). The flags update is an addition in the original: if called twice in a frame with the
  condition true it adds 8 (a port must use `+= 4`, not `|= 4`).

## 0x6B7080 CBike::PlayCarHorn  (catalogue: specified/REA, port level 6)
- Convention: virtual (vtable +0xD8) `__thiscall void ()`, plain `ret`. Callers: CBike::ProcessAI 0x6BC930 (2 sites).
- Callees: `rand` 0x821B1E; CPed::Say 0x5EFFE0 (thiscall(uint16 phrase, uint32 startTimeDelay, float volume, bool, bool, bool); pushes
  `0x16, 0, 1.0f, 0, 0, 0`).
- Fields: +0x45C m_nAlarmState (u16), +0x36 status, +0x514 m_HornCounter (u32), +0x519 m_nCarHornTimer (u8; gr declares
  `m_HornPattern` at +0x518 and `m_nCarHornTimer` at +0x519 once `m_nVehicleWeaponInUse` is taken as 1 byte), +0x460 m_pDriver,
  +0x3DB m_autoPilot.m_nCarCtrlFlags (bit0 bHonkAtCar).
- Behaviour:
  1. If `alarm != 0 && alarm != 0xFFFF && status != 5 (WRECKED)` return (0x6B7083-0x6B709C).
  2. If `m_HornCounter != 0` return.
  3. If `m_nCarHornTimer != 0` (unsigned): `m_nCarHornTimer--`; return.
  4. `r = rand()`; `t = (uint8)((r & 0x7F) - 0x6A)` (8-bit wrap); `m_nCarHornTimer = t`; `c = t & 7` (0x6B70C0-0x6B70CF):
     - `c < 2`: `m_HornCounter = 45`.
     - `2 <= c < 4`: if `m_pDriver && (m_nCarCtrlFlags & 1)`: `m_pDriver->Say(22, 0, 1.0f, 0, 0, 0)`; then `m_HornCounter = 45`.
     - `c >= 4`: if `m_pDriver`: `m_pDriver->Say(22, 0, 1.0f, 0, 0, 0)`; `m_HornCounter` unchanged.
- gta-reversed: plugin::Call (no C++).

## 0x6B7270 CBike::IsRoomForPedToLeaveCar  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall bool (uint32 door [esp+4], CVector* jackOffset [esp+8])`, `ret 8`, AL = 1 (0x6B7270).
- Behaviour: always `true`; arguments ignored.
- gta-reversed: inline, matches.

## 0x6B7280 CBike::ProcessRiderAnims  (catalogue: gr-unreversed/REA, port level 9)
- Convention: static `__cdecl void (CPed* rider [esp+4], CVehicle* veh [esp+8], CRideAnimData* ride [esp+0xC], tBikeHandlingData*
  bh [esp+0x10], int16 unused [esp+0x14])`, plain `ret` (0x6B7F83), caller pops 5 dwords. The 5th argument is never read (no access
  to L+0x140). SEH frame (`push -1; push 0x848299`), `sub esp, 0x110` + 4 pushes; below `L+n` = esp after the prologue, args at
  L+0x130/0x134/0x138/0x13C, EH state at L+0x128. Registers: edi = rider, esi = veh (until 0x6B7D8C), ebp = &veh->m_vecMoveSpeed
  (+0x44) in the knock-off code, ebx = ride from 0x6B7553.
- Callers (3): CBike::ProcessDrivingAnims 0x6BF400, CBmx::ProcessDrivingAnims 0x6BFB50, CQuadBike::ProcessDrivingAnims 0x6CE280.
- Callees: CPed::IsPlayer 0x5DF8F0; CPed::GetBikeRidingSkill 0x5DF510 (x2, ST0 result); RpAnimBlendClumpGetAssociation(clump, id)
  0x4D68B0 (cdecl, x9); CAnimManager::BlendAnimation(clump, group, id, delta) 0x4D4610 (x2); CAnimManager::AddAnimation(clump, group,
  id) 0x4D3AA0 (x4); CAnimBlendAssociation::SetCurrentTime(float) 0x4CEA80 (x4); CEventKnockOffBike ctor 0x4AFCF0 (x4) / dtor
  0x4AFE70; CEventGroup::Add(CEvent*, bool) 0x4AB420 (x4); `_CIpow` 0x822130 (ST1 base, ST0 exponent); `rand` 0x821B1E (x2);
  RtQuatRotate(RtQuat*, RwV3d* axis, float angleDeg, int combineOp) 0x7EB7C0 (cdecl, x2).
- Globals: CTimer::ms_fTimeStep 0xB7CB5C, ms_fTimeStepNonClipped 0xB7CB58, m_snTimeInMilliseconds 0xB7CB84; CPedIK::XaxisIK 0x8D232C
  (1,0,0) and YaxisIK 0x8D2338 (0,1,0) passed by address. Constants: 0x858624 1.0f, 0x871348 0.3f, 0x871334 0.02f, 0x858B50 0.0f,
  0x859028 3.5f, 0x858F40 -0.5f, 0x86C680 1.5 (f64), 0x858B38 0.02f, 0x858C1C -1.0f, 0x871350 0.86f, 0x87134C 0.89f, 0x859EF8 0.0 (f64),
  0x858B8C 0.5f, 0x858CC8 0.6f, 0x858FCC 0.15f, 0x871344 0.25f, 0x858C58 0.01f, 0x87133C 0.1f, 0x858C24 0.3f, 0x871340 -0.3f,
  0x871358 0.3f, 0x871354 0.56f, 0x87135C 0.01f, 0x871338 6.0f, 0x858C7C 3.0518509e-05f (1/32767).
- Fields. Vehicle: +0x14 matrix (forward +0x10..0x18, up.z +0x28), +0x3C CPhysical m_nLastCollisionTime, +0x44 move speed, +0x50
  turn speed, +0x384 m_pHandlingData (+0x88 transmission.m_MaxFlatVelocity, +0x8C m_MaxReverseVelocity), +0x464 m_apPassengers[0],
  +0x49C gas, +0x4A0 brake, +0x590 m_nVehicleType (0 AUTOMOBILE, 9 BIKE), +0x594 subtype (2 QUAD, 10 BMX). CBike: +0x614 nBikeFlags
  (0x80 bWheelieForCamera), +0x730 m_WheelCounts[4], +0x7B5 m_nFixRightHand, +0x804 m_nNoOfContactWheels (u8). CAutomobile: +0x7F4
  m_WheelCounts[4], +0x960 m_nNumContactWheels (u8). Ped: +0x18 m_pRwClump, +0x470 flags2 (0x4000 = bit 46 bUpdateMatricesRequired),
  +0x47C m_pIntelligence (+0x68 event group), +0x490 m_apBones[PED_NODE_HEAD] (offset confirmed by CPed::DoesLOSBulletHitPed
  0x5F01C9), frame data +0x10 KeyFrame (quaternion first). CRideAnimData (ride): +0 AnimGroup, +8 LeanAngle, +0x10 LeanFwd, +0x14
  AnimLeanLeft, +0x18 AnimLeanFwd. tBikeHandlingData (bh): +0x18 m_fFullAnimLean, +0x2C m_fWheelieAng, +0x30 m_fStoppieAng.
  CAnimBlendAssociation: +0x14 hierarchy (+0x10 total time), +0x18 blendAmount, +0x1C blendDelta, +0x2E flags (byte; bit 0 playing).
- Anim ids: 0xC3 STILL, 0xC4 LEFT, 0xC5 RIGHT, 0xC6 BACK, 0xC7 FWD, 0xC8 PUSHES, 0xCA/0xCB/0xCC DRIVEBY LHS/RHS/FT.
- Compare idioms as in the file header. Note `fcomp; test ah,0x41; jp X` jumps when st0 > src or NaN.

### A. 0x6B729F-0x6B73CC: setup and association lookup
1. `isPlayer = rider->IsPlayer()` (L+0x13). `bike (L+0x38) = 0; car (L+0x1C) = 0; nWheels (i16 L+0x3C) = 0`. If `veh->+0x590 == 9`:
   `bike = veh; nWheels = (u8)+0x804`; else if `+0x590 == 0`: `car = veh; nWheels = (u8)+0x960`.
2. `w (L+0x20) = 1.0f`, `tgt (L+0x18) = 0.0f`. Associations from `rider->m_pRwClump`: aLeft (L+0x30) = 0xC4, aRight (L+0x28) = 0xC5,
   aStill (L+0x14) = 0xC3; only if isPlayer: aFwd (L+0x50) = 0xC7, aBack (L+0x4C) = 0xC6 (otherwise those slots are uninitialised and
   never read); aPush (L+0x2C, also ebx) = 0xC8; aDriveBy (L+0x34) = first non-null of 0xCA, 0xCB, 0xCC (lookups stop at the first hit).

### B. 0x6B73D0-0x6B7403: forward speed, BMX hand fix
- `fwd (L+0x24) = f32((v.z*f.z + v.y*f.y) + v.x*f.x)` (move speed . matrix forward; fld +0x4C, fmul fwd.z first).
- If subtype (+0x594) == 10 and (aPush != 0 || aDriveBy != 0): byte `veh+0x7B5 = 0`.

### C. 0x6B740A-0x6B7548: knock-off by spin (all riders)
- `t = (GetBikeRidingSkill() + 1.0f) * 0.3f`; `S = (tx*tx + ty*ty) + tz*tz` of the turn speed (+0x50/+0x54/+0x58), all in x87
  registers; if `t*t < S` (fcompp, `test ah,5; jp` skips; NaN skips):
  1. `ev (L+0xA8) = CEventKnockOffBike(veh, &veh->m_vecMoveSpeed, &(0,0,1.0f) (L+0x60), 0.0f, 0.0f, 0x36, 2, 0, nullptr, true, false)`
     (gr parameter order: vehicle, moveSpeed, collisionImpactVelocity, damageIntensity, a6, knockOffType, knockOffDirection, time, ped,
     isVictimDriver, forceKnockOff); EH 0; `rider->m_pIntelligence->eventGroup.Add(&ev, false)`.
  2. If `p = veh->m_apPassengers[0]` is non-null and != rider: a second event (L+0x6C, vector L+0x40 = (0,0,1)) with the same
     arguments except isVictimDriver = false; added to `p`'s event group; destroyed (EH 0).
  3. `~ev` (EH -1).

### D. 0x6B754D-0x6B75BC: standing still
- If `aDriveBy == 0 && fabs(fwd) < 0.02f` (NaN -> E): if `aStill == 0 || (aStill->blendAmount < 1.0f && !(aStill->blendDelta > 0.0f))`
  (blendDelta NaN counts as > 0): `aStill = BlendAnimation(clump, ride->AnimGroup, 0xC3, 2.0f)`. In all sub-cases go to G.

### E. 0x6B75C1-0x6B77DE: moving (or drive-by active)
1. If `fwd < 0.0f` (else go to F):
   - `x = (GetBikeRidingSkill() + 1.0f) * 3.5f`; if subtype == 2 (QUAD) `x = x + x`; `x = x * handling->m_MaxReverseVelocity (+0x8C)`.
   - If `x > fwd` (0x6B7608 `test ah,0x41; jne` -> step 2 on <= / NaN): knock off if `nWheels > 0` (signed word, `jg`) OR
     (`matrix.up.z < -0.5f` AND `(u32)(m_snTimeInMilliseconds - veh->+0x3C) < 100`) (0x6B761C..0x6B763C). Knock-off = the same two
     events as C (rider: L+0xE4 / vector L+0x40; passenger: L+0x6C / vector L+0x54; EH 2/3), then go to G (0x6B773D). Otherwise step 2.
   - Step 2 (0x6B7742): if `gas (+0x49C) < 0.0f` and `handling->+0x8C * 1.5 < fwd` (f64 constant 0x86C680, compared in x87):
     `if (aPush == 0 || (aPush->blendAmount < 1.0f && !(aPush->blendDelta > 0.0f))) aPush = BlendAnimation(clump, group, 0xC8, 4.0f)`;
     go to G either way.
   - Step 3 (0x6B77BD): if isPlayer and `handling->+0x8C * 1.5 > fwd` (`test ah,0x41; jne` skips on <= / NaN): `tgt = -1.0f`.
2. F (0x6B77E6, also when fwd >= 0): if aStill and `!(aStill->blendDelta < 0.0f)` (NaN -> unchanged): `aStill->blendDelta = -4.0f`;
   same for aPush.

### G. 0x6B7821-0x6B78D0: remaining blend weight
- If aStill: `b = (tsNonClipped * 0.02f) * aStill->blendDelta + aStill->blendAmount` (x87, not stored); `if (1.0f < b) b = 1.0f`;
  `w = f32(1.0f - b)`.
- If aDriveBy: `b` likewise from aDriveBy; `if (w < b) b = w`; `w = f32(w - b)`. Then the same for aPush.

### H. 0x6B78D2-0x6B795C: sideways lean
- `lr (L+0x14) = f32(ride->LeanAngle / bh->m_fFullAnimLean)`. If `tgt == -1.0f` (`test ah,0x44; jp`, exact equality): `lr = 0.0f`;
  else `if (lr > 1.0f) lr = 1.0f; else if (lr < -1.0f) lr = -1.0f` (NaN unchanged).
- `k = pow(0.86f, ms_fTimeStep)`; `ride->AnimLeanLeft = (1.0f - k) * lr + k * ride->AnimLeanLeft` (x87 order: `1-k`, `*lr`, then
  `k*AnimLeanLeft`, `faddp`).

### I. 0x6B795F-0x6B7C7D: forward lean target (`tgt`)
- If `!isPlayer || veh->m_apPassengers[0] != 0` (0x6B7C64): `tgt = 0.0f`; if bike: `nBikeFlags &= 0x7F`; go to J.
- If `!(tgt > -1.0f)` (i.e. reversing flag -1 set; NaN impossible): go to J with tgt = -1.0f.
- `tgt = ride->LeanFwd` (raw copy).
- Bike branch (bike != 0): `nBikeFlags &= 0x7F` (stored at once). Let `c = bike->m_WheelCounts`, `fz = matrix.forward.z`.
  - Wheelie (0x6B79A1): if `!(c[0] > 0) && !(c[1] > 0) && fz > 0.0f && (c[2] > 0.0f || c[3] > 0.0 (f64 compare))`:
    `d = bh->m_fWheelieAng - fz`; if `d < bh->m_fWheelieAng * 0.5f`: `nBikeFlags |= 0x80`; then if `d < 0.15f`: `tgt = (tgt > 0.25f) ?
    tgt : 0.25f` and go to the lean-left damping (skipping the brake/gas rules); else continue at "Brake".
  - Otherwise stoppie (0x6B7A3D): if `!(c[2] > 0) && !(c[3] > 0) && fz < 0.0f && (c[0] > 0 || c[1] > 0)` and
    `bh->m_fStoppieAng * 0.6f < bh->m_fStoppieAng - fz`: `nBikeFlags |= 0x80` (no tgt change). Continue at "Brake".
- Car branch (bike == 0, car != 0; car == 0 -> "Brake"): same wheelie test on `car->m_WheelCounts` (+0x7F4..+0x800) with all
  compares against f32 0.0, then `d = bh->m_fWheelieAng - fz`; if `d < 0.15f`: `tgt = max-like 0.25f` as above and skip to damping.
  (No flag write for cars, no 0.5 test.)
- Brake (0x6B7AC4): if `brake (+0x4A0) > 0.5f && fwd > 0.01f`: `if (!(tgt > 0.1f)) tgt = 0.1f`. Else if `gas (+0x49C) > 0.5f &&
  !(tgt > 0.0f) && handling->m_MaxFlatVelocity (+0x88) * 0.3f > fwd`: `if (!(tgt < -0.3f)) tgt = -0.3f` (0x6B7C0A `jnp` keeps it).
- Damping (0x6B7C16): `a = fabs(lr)`; if `a > 0.3f`: `f = 1.0f - (a - 0.3f) / (0.56f - 0.3f)` (both subtractions done in x87: `a -
  0.3f`, `0.56f - 0.3f`, fdivp, fsubr 1.0); `if (0.0f > f) f = 0.0f`; `tgt = f32(f * tgt)`.

### J. 0x6B7C7F-0x6B7CAE: forward lean filter
- isPlayer: `k = pow(0.89f, ms_fTimeStep)`; `ride->AnimLeanFwd = (1.0f - k) * tgt + k * ride->AnimLeanFwd`. Else `AnimLeanFwd = 0.0f`.

### K. 0x6B7CB7-0x6B7D64: split of `w` between left/right (`wLR`, L+0x14) and fwd/back (`wFB`, L+0x1C)
- If `fabs(AnimLeanLeft) > 0.56f` or !isPlayer: `(A, B) = (1, 0)`. Else if `fabs(AnimLeanFwd) > 0.56f`: `(A, B) = (0, 1)`. Else
  `len = sqrt(ALL*ALL + ALF*ALF)` (ALL squared first); if `len > 0.01f`: `inv = f32(1.0f / len)`, `B = fabs(ALF * inv)`, `A =
  fabs(inv * ALL)`; else `A = fabs(ALL)`, `B = fabs(ALF)`.
- `wFB = f32(B * w)`, `wLR = f32(A * w)`.

### L. 0x6B7D68-0x6B7EA3: drive the lean animations
- isPlayer only: `if (!aFwd) aFwd = AddAnimation(clump, group, 0xC7)`; `if (!aBack) aBack = AddAnimation(clump, group, 0xC6)`. If
  `!(ALF < 0.0f)` (NaN -> back branch): `aFwd->blendAmount = wFB; aFwd->SetCurrentTime(aFwd->hier->totalTime * ALF); aFwd->flags &=
  0xFE; aBack->blendAmount = 0.0f`. Else: `aBack->blendAmount = wFB; aBack->SetCurrentTime(-(aBack->hier->totalTime * ALF));
  aBack->flags &= 0xFE; aFwd->blendAmount = 0.0f`.
- Always: `if (!aLeft) aLeft = AddAnimation(.., 0xC4)`; `if (!aRight) aRight = AddAnimation(.., 0xC5)`. If `!(ALL < 0.0f)`:
  `aRight->blendAmount = wLR; aRight->SetCurrentTime(aRight->hier->totalTime * ALL); aRight->flags &= 0xFE; aLeft->blendAmount = 0`;
  else `aLeft->blendAmount = wLR; aLeft->SetCurrentTime(-(aLeft->hier->totalTime * ALL)); aLeft->flags &= 0xFE; aRight->blendAmount = 0`.
  (Positive lean -> RIGHT anim.)

### M. 0x6B7EA6-0x6B7F65: head shake at speed
- If `fwd > 0.3f`: `q = &rider->m_apBones[HEAD]->KeyFrame->q` (no null checks). Twice (X axis, then Y axis): `s = f32(6.0f * fwd)`,
  `m = f32(-s)`; `r = rand()`; angle = `f32((s - m) * (float(r) * 3.0518509e-05f) + m)` (fild r, fmul, then `s - m`, fmul, fadd m);
  `RtQuatRotate(q, &XaxisIK / &YaxisIK, angle, 2 /*rwCOMBINEPOSTCONCAT*/)`. Then `rider->+0x470 |= 0x4000` (bUpdateMatricesRequired).
- Exit: restore fs:[0], return.

- Oddities to keep: knock-off by spin applies to every rider (players and peds), uses `t*t < |turn|^2`; the 5th parameter is unused;
  for a non-player rider the lean weights are `wLR = w, wFB = 0` and AnimLeanFwd is forced to 0; the stoppie only raises the camera
  flag; the car (quad) path has no 0.5 test; the f64 compare at 0x6B79FD is equivalent to f32 0.0.
- gta-reversed: `plugin::Call<0x6B7280>` only (Bike.cpp:386), no C++.
- Open questions: none affecting behaviour. (The meaning of event args 0x36 / 2 follows gr's parameter names only.)

## 0x6B7F90 CBike::FixHandsToBars  (catalogue: gr-unreversed/REA, port level 4)
- Convention: `__thiscall void (CPed* rider [esp+4])`, `ret 4`. SEH frame (two CMatrix locals). Extent includes the 2-instruction
  chunk 0x403C8D (`mov al,[ebx+0x7B5]; jmp 0x6B8200`). Caller: CPedIntelligence::ProcessAfterPreRender 0x6019B0.
- Callees: CMatrix::Attach 0x59BD10 (thiscall(RwMatrix*, bool owns)), CMatrix::operator= 0x59BBC0, CMatrix::operator*= 0x411A80,
  CMatrix(RwMatrix*, bool) ctor 0x59C050, ~CMatrix 0x59ACD0, Multiply3x3(out, const CMatrix&, const CVector&) 0x59C790 (rotation * v),
  0x59C810 Multiply3x3(out, const CVector&, const CMatrix&) (transposed rotation: `out = (v.right, v.fwd, v.up)` dot products),
  CMatrix*CVector 0x59C890 (cdecl(out, const CMatrix&, const CVector&)), RwHelper GetAnimHierarchyFromSkinClump 0x734A40,
  RpHAnimIDGetIndex 0x7C51A0 (cdecl(hier, boneId)), RpHAnimHierarchyGetMatrixArray 0x7C5120 (cdecl(hier) -> RwMatrix[], stride 0x40,
  position at +0x30), CPed::IsPlayer 0x5DF8F0.
- Globals: CModelInfo::ms_modelInfoPtrs 0xA9B0C8; default handlebar positions (.data f32): 0x8D3244/48/4C = (0.25, 0.29, 0.525)
  (generic), 0x8D3250/54/58 = (0.25, 0.29, 0.525) (model 510, MTBIKE), 0x8D325C/60/64 = (0.25, 0.21, 0.69) (model 509, BIKE);
  hand offset 0x8D3274/78/7C = (-0.075, -0.03, 0.0). Constants: 0x858B50 0.0f, 0x858C1C -1.0f, 0x871488 0.667f, 0x864E30 0.333f,
  0x858F34 0.75f, 0x858EE8 0.4f.
- Fields: +0x5A4 m_aBikeNodes[1] (chassis frame; modelling matrix at frame+0x10), +0x14 m_matrix, +0x22 model, +0x7B4 m_nFixLeftHand,
  +0x7B5 m_nFixRightHand; model info +0x5C m_pVehicleStruc, dummy at struc+0x78 (m_avDummyPos[10], the ped-arm/handlebar dummy);
  rider +0x18 m_pRwClump.
- Behaviour:
  1. If `!m_nFixRightHand && !m_nFixLeftHand` return (no matrices built).
  2. `matA`, `matB` = CMatrix (attach ptr / owns zeroed). If `m_aBikeNodes[1] == null`: destroy both and return WITHOUT clearing the
     fix flags (0x6B88F1).
  3. `matB.Attach(&chassis->modelling, false)`; `matA = *m_matrix`; `matA *= matB`; `hier = GetAnimHierarchyFromSkinClump(rider->m_pRwClump)`.
  4. `T = mi->m_pVehicleStruc->m_avDummyPos[10]`. If `T.x == 0 && T.y == 0 && T.z == 0` (ordered equal, 0x6B806A-0x6B80A9): `T` = defaults by
     model (510 -> 0x8D3250.., 509 -> 0x8D325C.., else 0x8D3244..). Else: `C = CMatrix(&mats[GetIndex(hier, 24)], false)` (right hand);
     `r = Multiply3x3(C, (-0.075, -0.03, 0.0))`; `r = Multiply3x3(r, matA)` (transpose); `T.x += r.x; T.y += r.y; T.z += r.z`; destroy C.
     (This adjustment always uses the right-hand bone 24, also for the left hand.)
  5. Right hand (if `m_nFixRightHand`, 0x6B8200): `W = matA * T`; `P = mats[i(24)].pos`; `D = W - P` (z, y, x order, each f32);
     bone 24 pos `+= D`; bone 25 `+= D`; bone 26 `+= D`; bone 23 `+= 0.667f * D`; bone 22 `+= 0.333f * D`; if `rider->IsPlayer()`:
     bone 301 `+= 0.333f * D`. Each "+=" is `pos.c = P.c + k*D.c` stored f32 per component (the scaled `k*D` is computed once, z, y, x).
  6. Left hand (if `m_nFixLeftHand`, 0x6B8551): `T.x = T.x * -1.0f` (T is modified in place; the right-hand step already used it);
     `W = matA * T`; `D = W - mats[i(34)].pos`; bone 34 `+= D`; 35 `+= D`; 36 `+= D`; 33 `+= 0.75f * D`; 32 `+= 0.4f * D`;
     if `rider->IsPlayer()`: bone 302 `+= 0.4f * D`.
     Bone indices are looked up with RpHAnimIDGetIndex for every bone (no caching); positions are written into the hierarchy matrix array.
  7. `m_nFixLeftHand = 0; m_nFixRightHand = 0`; destroy matB then matA.
- Edge cases: the right/left chains use different falloffs (0.667/0.333 vs 0.75/0.4) - keep. The left side mirrors only T.x (after the
  optional bone-24 offset). Bone matrices other than the translation are untouched.
- gta-reversed: plugin::Call (no C++).

## 0x6B8940 CBike::FindWheelWidth  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall float (bool rear [esp+4])`, `ret 4`, result ST0.
- Behaviour: `return 0.15f` (`fld dword [0x858FCC]` = 0x3E19999A, 0x6B8940); argument ignored.
- Callers: CVehicle::AddSingleWheelParticles 0x6DE880 (virtual).
- gta-reversed: inline, matches.

## 0x6B8950 CBike::vfunc_0 (proposed name: CBike scalar deleting destructor)  (catalogue: specified/REA, port level 16)
- Convention: vtable slot 0, `__thiscall CBike* (uint8 flags [esp+4])`, `ret 4`, EAX = this. Callers (virtual): CCarCtrl::CreatePoliceChase,
  CreateConvoy, GenerateOneRandomCar, CSetPiece::Update.
- Behaviour: `CBike::~CBike(0x6B57A0)(this)`; if `flags & 1`: `CVehicle::operator delete(this)` (0x6E2D90, cdecl, pool free); return this.
- Name: the catalogue name `vfunc_0` is the compiler's scalar deleting destructor (vtable slot 0, test of bit 0 then delete).
- gta-reversed: not hooked (the C++ `delete` expression produces it).

## 0x6B89B0 CBike::SetupSuspensionLines  (catalogue: specified/REA, port level 2)
- Convention: virtual `__thiscall void ()`, plain `ret`. Callers: CBike::Constructor 0x6BF430, CVehicle::ClearVehicleUpgradeFlags
  0x6D3210 (virtual, 2 sites).
- Callees: RwMatrixTransform 0x7F25A0 (cdecl(RwMatrix*, const RwMatrix*, 2 = POSTCONCAT)), 3 frame walks.
- Globals: CModelInfo::ms_modelInfoPtrs 0xA9B0C8. Constants: 0x858C84 = 0.25f, 0x85C654 = -0.25f, 0x858B8C = 0.5f, 0x858B90 = 4.0f,
  0x858624 = 1.0f, 0x858EE8 = 0.4f; marker 0x47C34FFF (= 99999.99f) compared as raw dword.
- Data: `mi` = CVehicleModelInfo (+0x40 wheel size front, +0x44 rear, +0x14 m_pColModel); `cm` = mi->m_pColModel (bbox min +0x00,
  max +0x0C, sphere centre +0x18, radius +0x24, m_pColData +0x2C); `cd` = cm->m_pColData: u16 numSpheres +0x00 (signed compares),
  m_pSpheres +0x08 (stride 0x14: centre +0, radius +0xC), m_pLines +0x10 (CColLine stride 0x20: start +0x00, end +0x10).
  Handling `h` = this->m_pHandlingData (+0x384): +0xAC m_fSuspensionForceLevel, +0xB8 m_fSuspensionUpperLimit,
  +0xBC m_fSuspensionLowerLimit, +0xCC m_nModelFlags (0x40000000 = bForceGroundClearance).
  Bike fields: +0x5A8/+0x5AC/+0x5B0/+0x5B4 = m_aBikeNodes[2..5] (FORKS_FRONT, FORKS_REAR, WHEEL_FRONT, WHEEL_REAR),
  +0x18 m_pRwClump (its frame = clump->object.parent, +4), +0x760 m_aWheelSuspensionHeights[2], +0x768 m_aWheelOrigHeights[2],
  +0x770 m_fSuspensionLength[4], +0x780 m_fLineLength[4], +0x790 m_fHeightAboveRoad, +0x798 m_fSwingArmLength,
  +0x79C m_fForkYOffset, +0x7A0 m_fForkZOffset.
- Frame walk W(frame, stop): `M = frame->modelling; p = frame->parent; while (p) { RwMatrixTransform(&M, &p->modelling, 2);
  p = p->parent; if (p == stop) break; }` - the first parent is always applied (the stop test follows the transform).
- Behaviour:
  1. `linesSet = (cd->m_pLines[0].start.z bits != 0x47C34FFF)` (0x6B89F9); `line1Marker = (cd->m_pLines[1].start.z bits == 0x47C34FFF)`
     (0x6B89DB). `off = 0.0f`.
  2. For `i = 0..3` (16-bit counter, 0x6B8A20-0x6B8C98):
     a. If `linesSet` (0x6B8A2B): `P = lines[i].start`; `P.z = (i < 2) ? m_aWheelOrigHeights[0] : m_aWheelOrigHeights[1]`; go to (c).
        (`off` is not used on this path.)
     b. Else: jump table 0x6B8EB0: i=0: `fr = WHEEL_FRONT, off = wsF * 0.25f`; i=1: `fr = WHEEL_FRONT, off = wsF * -0.25f`;
        i=2: `fr = WHEEL_REAR, off = wsR * 0.25f`; i=3: `fr = WHEEL_REAR, off = wsR * -0.25f`.
        `W(fr, clumpFrame)`; `P = M.pos`.
        - i == 0: `m_aWheelOrigHeights[0] = P.z`.
        - i == 2: `m_aWheelOrigHeights[1] = P.z`; if `m_aBikeNodes[3]` (FORKS_REAR) != null: `W(forksRear, clumpFrame)`;
          `dy = P.y - M.pos.y; dz = P.z - M.pos.z; m_fSwingArmLength = sqrt(dz*dz + dy*dy)` (dz-square first, 0x6B8B8A-0x6B8BAA);
          else `m_fSwingArmLength = 0`.
        - all i: `P.y = P.y + off` (0x6B8BBC).
     c. Tail (0x6B8BC8): `P.z = P.z + h->upper`; `lines[i].start = P`; `ws = (i==0||i==1) ? wsF : wsR`;
        `lines[i].end = (P.x, P.y, ((h->lower - h->upper) - ws*0.5f) + P.z)`;
        `m_fSuspensionLength[i] = h->upper - h->lower`; `m_fLineLength[i] = lines[i].start.z - lines[i].end.z` (re-read from memory).
  3. If `!linesSet` (0x6B8CA6): `W(forksFront, forksFront)` - the stop frame is the forks frame itself, so this walk runs up to the
     root (null parent) and includes the clump frame's matrix (unlike the walks in step 2, which stop below the clump frame);
     `m_fForkYOffset = M.pos.y; m_fForkZOffset = M.pos.z`. Keep this asymmetry.
  4. `m_fHeightAboveRoad = (1.0f - 1.0f / (h->forceLevel * 4.0f)) * m_fSuspensionLength[0] + (wsF * 0.5f - lines[0].start.z)`
     (0x6B8CF5-0x6B8D34; the left product is computed first, then the bracket, then added).
  5. For `j = 0, 1`: `m_aWheelSuspensionHeights[j] = (j==0 ? wsF : wsR) * 0.5f - m_fHeightAboveRoad` (0x6B8D40-0x6B8D68).
  6. Bounds (0x6B8D6A-0x6B8E02): if `lines[0].end.z < cm->bbox.min.z` then `min.z = lines[0].end.z` (raw copy);
     `r1 = sqrt((min.x^2 + min.y^2) + min.z^2)`, `r2 = sqrt((max.x^2 + max.y^2) + max.z^2)`; `r = (r1 > r2) ? r1 : r2` (r2 recomputed
     on the else path); `if (r > cm->sphere.radius) cm->sphere.radius = r` (ordered `>`).
  7. Ground clearance (0x6B8E04): only if `(h->m_nModelFlags & 0x40000000) && line1Marker`: `g = 0.25f - m_fHeightAboveRoad`;
     for `k = 0 .. (int16)cd->numSpheres-1`: `s = spheres[k]`; if `(s.centre.z - s.radius) < g`: { if `s.radius > 0.4f`:
     `s.radius = max(s.centre.z - g, 0.4f)` (written as `if (0.4f > v) v = 0.4f`, NaN keeps v); then `s.centre.z = g + s.radius` }.
- Edge cases: `line1Marker` tests line 1 while `linesSet` tests line 0 (two different markers). In step 2a the line x/y come from
  the stored lines (already offset by `off` on the first run). No null checks on the wheel frames.
- gta-reversed: plugin::Call (no C++).

## 0x6B8EC0 CBike::VehicleDamage  (catalogue: gr-unreversed/REA, port level 14)
- Convention: virtual (vtable +0xE0) `__thiscall void (float damageIntensity [esp+4], int component, CEntity* damager, CVector* coors,
  CVector* dir, eWeaponType weapon)`, `ret 0x18`. Only `damageIntensity` is read (its slot is then reused as a float temp).
  Caller: CBike::ProcessControl 0x6B9250.
- Callees: CStats::GetPercentageProgress 0x5591E0 (ST0), CBike::DamageKnockOffRider 0x6B5A10 (cdecl(CVehicle*, float, uint16, CEntity*,
  const CVector&, const CVector&)), CEntity::RegisterReference 0x571B70, CVector::DotProduct 0x40FDB0, CWorld::FindPlayerVehicle 0x56E0D0
  (cdecl(-1, false), 7 calls), CVector::Magnitude 0x4082C0, CWorld::FindPlayerPed 0x56E210, CPlayerPed::SetWantedLevelNoDrop 0x609F30,
  CPed::Say 0x5EFFE0.
- Constants: 0x859EF8 0.0 (f64), 0x858624 1.0f, 0x858628 100.0f, 0x858B8C 0.5f, 0x858BA4 20.0f, 0x858CC8 0.6f, 0x858FE8 25.0f,
  0x858B1C 0.1f, 0x858B50 0.0f, 0x858C80 5.0f, 0x85F0A0 0.16666667f, 0x862D68 0.6666667f, 0x858C84 0.25f, 0x85F0FC 0.083333336f,
  0x859F80 250.0f.
- Fields: CPhysical +0xD8 m_fDamageIntensity, +0xDC m_pDamageEntity, +0xE0 m_vecLastCollisionImpactVelocity, +0xEC m_vecLastCollisionPosn,
  +0xF8 m_nPieceType (u16), +0x40 physical flags (0x100000 bCollisionProof); vehicle +0x4C0 m_fHealth, +0x508 m_nLastWeaponDamageType,
  +0x50C m_pLastDamageEntity, +0x428 (bit0 bIsLawEnforcer), +0x429 (bit7 bTakeLessDamage); bike +0x614 (0x10 bOnSideStand, 0x40
  bEngineOnFire), +0x7BC m_BlowUpTimer, +0x800 m_Damager; handling +0xC8 m_fCollisionDamageMultiplier.
- Behaviour:
  1. Return if `damageIntensity > 0.0` (ordered, f64 compare; only collision damage, called with 0, is handled), if
     `m_fDamageIntensity < 1.0f`, or if `!bCanBeDamaged`.
  2. `d = m_fDamageIntensity`; if status == PLAYER and `!(GetPercentageProgress() < 100.0f)`: `d *= 0.5f`.
     If bOnSideStand and `d > 20.0f`: clear bOnSideStand.
  3. `DamageKnockOffRider(this, m_fDamageIntensity /*raw, not d*/, m_nPieceType, m_pDamageEntity, m_vecLastCollisionPosn,
     m_vecLastCollisionImpactVelocity)`.
  4. If `m_pDamageEntity` is a vehicle: `m_nLastWeaponDamageType = 49; m_pLastDamageEntity = it; RegisterReference(&m_pLastDamageEntity)`
     (old reference not cleaned up).
  5. Return if bCollisionProof. If `m_pDamageEntity` is a building and `DotProduct(impactVelocity, up) > 0.6f`: return.
  6. If `d > 25.0f && status != WRECKED`:
     a. If bIsLawEnforcer && `FindPlayerVehicle()` && `m_pDamageEntity == FindPlayerVehicle()` && status != ABANDONED &&
        `!(|playerVeh.moveSpeed| < |this.moveSpeed|)` && `|playerVeh.moveSpeed| > 0.1f`: `FindPlayerPed()->SetWantedLevelNoDrop(1)`.
     b. `d2 = (d - 25.0f) * handling->m_fCollisionDamageMultiplier`; if `d2 > 0.0f`:
        - if `d2 > 5.0f && m_pDriver && m_pDamageEntity is a vehicle && !(FindPlayerVehicle() == this && damager->m_nCreatedBy == 2)
          && damager->m_pDriver`: `m_pDriver->Say(66, 0, 1.0f, 0, 0, 0)`.
        - `old = m_fHealth`; `k = (FindPlayerVehicle() == this) ? (bTakeLessDamage ? d2*0.16666667f : d2*0.5f) :
          (bTakeLessDamage ? d2*0.083333336f : (m_pDamageEntity && m_pDamageEntity == FindPlayerVehicle()) ? d2*0.6666667f : d2*0.25f)`;
          `m_fHealth = m_fHealth - k`; if `m_fHealth <= 1.0f && old > 1.0f`: `m_fHealth = 1.0f` (one collision cannot take the bike
          from above 1 to below 1).
  7. (0x6B91FC, also reached when step 6 is skipped) If `m_fHealth < 250.0f` and not bEngineOnFire: set bEngineOnFire;
     `m_BlowUpTimer = 0`; `m_Damager = m_pDamageEntity`; if non-null `RegisterReference(&m_Damager)`.
- gta-reversed: plugin::Call (no C++).

## 0x6B9250 CBike::ProcessControl  (catalogue: gr-unreversed/REA, port level 24)
- Convention / signature: virtual override, `__thiscall void ()`, plain `ret`. ESI = this throughout. SEH frame (handler 0x8482EB; trylevel
  [esp+0x194] set to 0 only around the CEventVehicleOnFire temp in B30), `sub esp,0x17C` + 4 saved regs. Locals below are named `Lxx` =
  `[esp+0xxx]` in the frame with no pushes outstanding.
- Extent: 0x6B9250-0x6BC92D (catalogue chunks 0x6B9250-0x6B9FB7, 0x6B9FC0-0x6BA118, 0x6BA120-0x6BA4AD, 0x6BA4B0-0x6BC92D; the gaps are
  alignment padding before loop heads, no obfuscation stubs). All of it is covered below in address order, B1..B37.
- Callers: CBmx::ProcessControl 0x6BFA30 (direct call); CBike vtable slot (virtual).
- Callees: CEntity::GetColModel 0x535300; CAEVehicleAudioEntity::Service 0x502280; CVehicle::ProcessCarAlarm 0x6D21F0,
  ActivateBombWhenEntered 0x6D2570, UpdateClumpAlpha 0x6D2980, ProcessDelayedExplosion 0x6D1340, ProcessSirenAndHorn 0x6E0950,
  ProcessBikeWheel 0x6D73B0 (x3); CPed::IsPlayer 0x5DF8F0; CPlayerPed::ResetPlayerBreath 0x60A8A0; virtuals ProcessAI (vtable+0x108),
  ProcessControlCollisionCheck (+0x5C), VehicleDamage (+0xE0), BlowUpCar (+0xA4); CPhysical::ProcessControl 0x5485E0, SkipPhysics 0x5433B0,
  ApplyMoveForce 0x5429F0 (thiscall(CVector)), ApplyTurnForce 0x542A50 (thiscall(CVector force, CVector point); force at the lower stack
  address), GetSpeed 0x542CE0 (thiscall(CVector* out, CVector point)), ApplySpringCollision 0x543C90, ApplySpringCollisionAlt 0x543D60,
  ApplySpringDampening 0x543E90; CBike::ProcessBuoyancy 0x6B5FB0; cTransmission::CalculateDriveAcceleration 0x6D05E0; CStats::GetFatAndMuscleModifier
  0x559AF0 (cdecl(int) -> ST0); SurfaceInfos_c (this = g_surfaceInfos 0xB79538) GetAdhesionGroup 0x55E5C0, GetWetMultiplier 0x55E600,
  GetSkidmarkType 0x55E630, GetRoughness 0x55E810, GetAdhesiveLimit 0x55EB50 (thiscall(CColPoint&) -> ST0); RpAnimBlendClumpGetAssociation 0x4D68B0;
  CCullZones::DoExtraAirResistanceForPlayer 0x72DDD0; CCarCtrl::ScanForPedDanger 0x42CE40; CWorld::FindPlayerPed 0x56E210; CPad::GetPad 0x53FB70,
  StartShake 0x53F920; FxManager_c::CreateFxSystem 0x4A9BE0, FxSystem_c::Play 0x4AA2F0 / Kill 0x4AA3F0; CEventVehicleOnFire ctor 0x4B10C0 / dtor
  0x4B1170; RwHelper::GetEventGlobalGroup 0x4ABA50; CEventGroup::Add 0x4AB420. Math helpers (cdecl unless noted): DotProduct 0x40FDB0 (-> ST0),
  `operator+` 0x40FE30 (out, a*, b*), `operator*(float, const CVector*)` 0x40FE90 (out, s, v*), `operator/` 0x4119D0 (out, v*, s: r = 1/s, v*r),
  CVector::SquaredMagnitude 0x406DA0 (thiscall, x*x + y*y + z*z), Multiply3x3 0x59C790 (out, const CMatrix*, const CVector*), transposed
  Multiply3x3 0x59C810 (out, const CVector*, const CMatrix*), CrossProduct 0x59C730 (out, a*, b*) = a x b, CVector::Normalise 0x59C910 (thiscall),
  0x424100 (cdecl(float, float), body `xor al,al; ret` - always false), `_ftol` 0x821B40, `_CIasin` 0x821E70 (ST0 -> ST0; fpatan(x, sqrt((1+x)(1-x)))),
  `_CIpow` 0x822130 (ST1 base, ST0 exponent).
- Globals: CReplay::Mode 0xA43088 (u8, R); CTimer::ms_fTimeStep 0xB7CB5C (`ts` below); CTimer::m_snTimeInMilliseconds 0xB7CB84;
  CWorld::bForceProcessControl 0xB7CD6E; CCheat::m_aCheatsActive[28] 0x96914C; CModelInfo::ms_modelInfoPtrs 0xA9B0C8; g_fxMan 0xA9AE80;
  GAME_GRAVITY 0x863984 (0.008f); tweakable .data floats 0x8D3238 (0.9995f), 0x8D323C (0.9f), 0x8D3240 (0.95f), 0x8D3280 (0.05f), 0x8D3284 (-0.1f),
  0x8D3288 (0.01f) (image values; unnamed, near gDuckAnimBlendData); 0xC1C818 (computed float, see _static-init-E.md); wheel-state temps
  0xC1C26C / 0xC1C270 (tWheelState front/rear, R/W) and thrust temp 0xC1C27C (float, R/W) - static file-scope variables shared with the next
  call, NOT per-vehicle (gr calls these "inappropriate statics" in ProcessWheel).
- Fields (beyond the CBike.md header layout): CPhysical +0x40 m_nPhysicalFlags (0x4 bDisableCollisionForce, 0x8 bCollidable, 0x100
  bSubmergedInWater), +0x50 turn, +0x5C/+0x68 friction move/turn, +0x74 m_vecForce, +0x80 m_vecTorque, +0x8C m_fMass, +0x90 m_fTurnMass, +0x94
  m_fVelocityFrequency, +0x98 m_fAirResistance, +0xA4 m_vecCentreOfMass, +0xB8 m_nFakePhysics (u8), +0xD4 m_fMovingSpeed, +0xD8
  m_fDamageIntensity, +0xDC m_pDamageEntity, +0xE0 m_vecLastCollisionImpactVelocity, +0xFC m_pAttachedTo; CEntity +0x1C flags (0x10 bIsStuck,
  0x20 bIsInSafePosition, 0x40 bWasPostponed); +0x4B8 m_fGearChangeCount; +0x488 m_nNumPassengers; +0x57C m_pFireParticle; +0x630
  m_vecOldSpeedForPlayback. tHandlingData: +0x14 COM, +0x28 traction mult, +0x2C transmission, +0x94 brake decel, +0x98 brake bias, +0xA0
  steering lock, +0xA8 traction bias, +0xAC susp force, +0xB0 susp damping, +0xC0 susp front/rear bias, +0xD0 handling flags (bit 3
  bNpcNeutralHandl, bit 23 bProcRearwheelFirst). tBikeHandlingData (bh = +0x63C): +0x14 MaxLean, +0x1C DesLean, +0x20 SpeedSteer, +0x24 SlipSteer,
  +0x28 NoPlayerCOMz, +0x2C WheelieAng, +0x30 StoppieAng, +0x34 WheelieSteer, +0x38 WheelieStabMult, +0x3C StoppieStabMult.
  CVehicleModelInfo (mi): +0x40 m_fWheelSizeFront, +0x44 m_fWheelSizeRear, +0x5C m_pVehicleStruct. colData: +0x10 m_pLines (CColLine 0x20: start
  +0, end +0x10). CColPoint 0x2C: +0 point, +0x10 normal, +0x20 surfaceTypeA, +0x23 surfaceTypeB.
- extraHandlingFlags (L7C, filled by ProcessAI): bit 2 = rider/controlled ("+= 2" in ProcessAI), bit 4 tested only for the wheel-state reset.
- Behaviour (address order; `ratio[i]` = m_aWheelRatios[i] +0x710, `steer` = m_fSteerAngle +0x494, `M` = *matrix):
  B1. Prologue/entry (0x6B9250-0x6B92DF): SEH frame (handler 0x8482EB), `sub esp,0x17C`. `colData = GetColModel()->m_pColData` (+0x2C) -> L74;
      `mi = CModelInfo::ms_modelInfoPtrs[m_nModelIndex]` -> L60; `extraHandlingFlags` L7C = 0. `m_vehicleAudio.Service()` (this+0x138, 0x502280).
      Clears (stored in this order): byte+0x429 &= 0xDF (bWarnedPeds), +0x5C8 m_bLeanMatrixCalculated = 0, word +0x7A8 m_nBrakesOn = 0,
      nBikeFlags &= 0xDF (bPlayerBoost), byte+0x42C &= 0xFD (bRestingOnPhysical). If `CReplay::Mode` (0xA43088, u8) == 1 -> exit (0x6BC914).
  B2. 0x6B92E5-0x6B935D: ProcessCarAlarm 0x6D21F0, ActivateBombWhenEntered 0x6D2570, UpdateClumpAlpha 0x6D2980 (all thiscall, this).
      m_nTestPedCollision (+0x7B6, u8): "player aboard" = `driver && (driver->IsPlayer() || (passenger0 && passenger0->IsPlayer()))` - NOTE:
      a NULL driver goes straight to 0 (0x6B9302), passenger0 is only checked when a non-player driver exists. If player aboard:
      `1 -> 2`, `0 -> 1`, `>= 2` unchanged (0x6B9326-0x6B9335); else `= 0`.
      `this->ProcessAI(&extraHandlingFlags)` (virtual vtable+0x108; CBike 0x6BC930 / CBmx 0x6C1470), return value IGNORED.
      If `status == 2` (STATUS_SIMPLE; `byte[+0x36] >> 3`) -> exit.
  B3. Side stand / COM (0x6B9363-0x6B93D5): if `nBikeFlags & 0x10` (bOnSideStand) and (`|matrix.right.z| > 0.35f`[0x858F9C] or
      `|matrix.forward.z| > 0.5f`): clear 0x10 (stored). If `!(extraHandlingFlags & 2)` and `!(nBikeFlags & 0x18)`:
      `m_vecCentreOfMass = (handling.COM.x [+0x14], handling.COM.y [+0x18], m_BikeHandling->m_fNoPlayerCOMz [+0x28])` -> B5 (0x6B97C9).
  B4. Rider stabilisation (0x6B93DA-0x6B97C6), entered when extraHandlingFlags&2 or bOnSideStand|bGettingPickedUp:
      Locals from tweakable .data floats: L40 = [0x8D323C] (init 0.9f), L3C = [0x8D3238] (init 0.9995f), L44 = [0x8D3240] (init 0.95f, unused
      below), L34 = 1.0f [0x8712D8]. `lt = Multiply3x3T(m_vecTurnSpeed, *matrix)` via 0x59C810 (cdecl(out, const CVector* v, const CMatrix* m):
      out.x = (m.right.y*v.y + m.right.z*v.z) + v.x*m.right.x; out.y = (m.fwd.y*v.y + m.fwd.x*v.x) + m.fwd.z*v.z; out.z likewise with up)
      -> L68/L6C/L70.
      Base damping D (ST0) by status/wheel state:
      - status != PLAYER (byte[+0x36] & 0xF8 != 0, 0x6B9419): D = L3C.
      - PLAYER, both front ratios `m_aWheelRatios[0] >= 1 && [1] >= 1` (0x6B9421/0x6B9438, `test ah,1`: `< 1` or NaN -> stoppie branch):
        L34 = 100.0f; L10 = GetFatAndMuscleModifier(13) * 0.2f[0x858CC4]. If (`ratio[2] < 1 || ratio[3] < 1`) and `fwd.z > 0`:
        `v = |bh.WheelieAng[+0x2C] - fwd.z| * L10`; D = `L3C - 0.05f` if `v > 0.05f` else `L3C - v` (recomputed, same order: fld W; fsub fz; fabs;
        fmul L10; fsubr L3C). Otherwise D = 0.98f [0x8712E0].
      - PLAYER, a front ratio < 1: if `WheelCounts[2] > 0` or `WheelCounts[3] > 0` (or NaN) -> D = L3C. Else L34 = 100.0f;
        L10 = GFM(13) * 0.075f[0x8714A8]; L4C = GFM(13) * 0.25f (two separate calls). If `fwd.z < 0`: `v = |bh.StoppieAng[+0x30] - fwd.z| * L4C`;
        D = `(L10 + 0.9f[0x858C20]) * L3C` if `v > L10` else `(v + 0.9f) * L3C`; else D = L3C.
      Then (0x6B95AD): `L3C = D / (lt.x * lt.x * L34 + 1.0f)` (fld lx; fmul lx; fmul L34; fadd 1; fdivr); `L40 = L40 / (1000.0f * ly * ly + 1.0f)`
      (fld 1000; fmul ly; fmul ly; fadd 1; fdivr L40); `L3C = _CIpow(L3C, ms_fTimeStep)`; `P40 = _CIpow(L40, ms_fTimeStep)` (kept in ST0).
      `dX = lx * L3C - lx` -> L80; `dY = P40 * ly - ly`.
      Torque 1: `F1 = (-1.0f*up.x * dY * TM, -1.0f*up.y * dY * TM, -1.0f*up.z * dY * TM)` (TM = m_fTurnMass +0x90; each up component is multiplied
      by -1.0f [0x858C1C] first, then by dY, rounded to f32, then by TM); `P1 = Multiply3x3(*matrix, m_vecCentreOfMass)` (0x59C790) `+ matrix.right`
      (components added z, y, x); `ApplyTurnForce(F1, P1)`.
      Torque 2: `F2 = (dX*up.x*TM, dX*up.y*TM, dX*up.z*TM)`, `P2 = Multiply3x3(*matrix, COM) + matrix.forward`; `ApplyTurnForce(F2, P2)`.
      If `status != PLAYER` (0x6B97AC): `m_vecCentreOfMass = handling.COM` (+0x14..+0x1C).
  B5. Fake-physics / skip test (0x6B97C9-0x6B9A3D): `skip` L5F = 0. Block entered only if `!(m_nFlags & 0x10)` (bIsStuck) and status is
      4 (ABANDONED) or 5 (WRECKED) and `!(nBikeFlags & 0x08)`.
      `idle` L27 = `!(byte[+0x42B] & 1)` (bVehicleColProcessed) `&& moveSpeed.x == 0 && .y == 0 && .z == 0` (`test ah,0x44`, NaN = non-zero)
      `&& dword[+0x72C] != 0x3F800000` - the original compares m_aRatioHistory[3] (raw bits vs 1.0f) FOUR times (0x6B983A-0x6B984E); only
      index 3 is ever tested.
      Limits: WRECKED: L10 = 0.006f, L4C = 0.0015f, L78 = 0.015f; ABANDONED: 0.003f, 0.0009f, 0.005f (raw stores).
      `m_vecForce(+0x74) = (m_vecForce + m_vecMoveSpeed) / 2.0f` (0x40FE30 operator+(out, a, b), 0x4119D0 operator/(out, v, s): r = 1.0f/s;
      out = (v.x*r, v.y*r, v.z*r)); `m_vecTorque(+0x80) = (m_vecTorque + m_vecTurnSpeed) / 2.0f`.
      `reset` = `force.SqMag() <= (L10*ts)^2` (0x6B9926: fld t; fmul t; fcompp; `< sqmag` or NaN fails) `&& torque.SqMag() <= (L4C*ts)^2`
      `&& m_fMovingSpeed[+0xD4] < L78`. SqMag = 0x406DA0 (x*x + y*y + z*z).
      If `!reset && !idle`: `m_nFakePhysics(+0xB8) = 0`. Else: `m_nFakePhysics++` (u8); if `> 10 || idle`: call 0x424100(pos.x, pos.y) (pos =
      matrix ? matrix.pos : placement.pos +4) - it always returns 0 (`xor al,al; ret`), so the `jne` at 0x6B99B7 is never taken; then if
      `!idle || m_nFakePhysics > 10`: `m_nFakePhysics = 10`; `moveSpeed = turnSpeed = 0`; skip = 1.
  B6. Ground physicals (0x6B9A44-0x6B9A93): for i = 0..3 ascending over m_aGroundPhysicalPtrs (+0x7C0): if non-null: byte+0x42C |= 2
      (bRestingOnPhysical); if `CWorld::bForceProcessControl` (0xB7CD6E) is NON-zero and `ptr->m_nFlags & 0x20` (bit 5, gr m_bIsInSafePosition):
      `m_nFlags |= 0x40` (bWasPostponed) and return (0x6B9BD6). (gr's CAutomobile tests `!bForceProcessControl`; the bike tests the flag set.)
      After the loop: if bRestingOnPhysical: skip = 0, m_nFakePhysics = 0.
  B7. 0x6B9A95: `this->VehicleDamage(0.0f, 0, NULL, NULL, NULL, 49 /*WEAPON_RAMMEDBYCAR*/)` (virtual vtable+0xE0).
      `knock` L27 = (`m_fDamageIntensity(+0xD8) > 0.0` (f64 const) `&& |dot(m_vecLastCollisionImpactVelocity(+0xE0), matrix.right)| > 0.5f`
      (dot order: (iz*rz + iy*ry) + ix*rx) `&& moveSpeed.SqMag() < 0.1f` (inline x*x+y*y+z*z)) `|| (nBikeFlags & 0x08)`.
  B8. Skip path (0x6B9B39, skip != 0): `CPhysical::SkipPhysics()` (0x5433B0); byte+0x42B &= 0xDE (bVehicleColProcessed, bAudioChangingGear).
      If bOnSideStand: if `RideAnimData.BarSteerAngle(+0x644) < 0.34906587f` [0x8631B4]: `+0x644 = ts * 0.017453292f[0x8595EC] + +0x644`.
      Then `P = _CIpow(0.97 [f64 0x866FD0], ts)` -> L10; `c = clamp(matrix.right.z, -1, 1)` (> 1.0 -> 1.0; else if -1.0 > rz -> -1.0; else rz;
      NaN -> rz); `a = _CIasin(c)` (0x821E70: fpatan(x, sqrt((1+x)(1-x))) for |x| < 1); `lean = P * DesiredLean(+0x64C) - (1.0f - P) * (a + 0.2617994f
      [0x858F1C])`; store to +0x64C (fst) and +0x648. -> B_TAIL (0x6BBC18). (Not on stand: -> 0x6BBC18 directly.)
  B9. Physics path (0x6B9C3A): if `!bVehicleColProcessed`: `ProcessControlCollisionCheck(true)` (vtable+0x5C).
      If `!(extraHandlingFlags & 2) && !(nBikeFlags & 0x18)` (no rider control): steer towards the low side: if `matrix.right.z < 0`:
      if `m_fSteerAngle(+0x494) > -0.43633232f` [0x8714A4]: `steer = steer - ts * 0.0087266462f` [0x8631D4]; else (rz >= 0 or NaN): if
      `steer < 0.43633232f` [0x8714A0]: `steer = ts * 0.0087266462f + steer`.
      `savedAir` L64 = m_fAirResistance (+0x98). If `status == PLAYER && driver`: `a = GetAssoc(driver->clump, 199 BIKE_FWD)`; if `a &&
      a.blend > 0.5f && a.currentTime > 0.06f [0x871294] && a.currentTime < 0.14f [0x871298]`: `m_fAirResistance *=
      (CCullZones::DoExtraAirResistanceForPlayer() ? 0.85f [0x862E1C] : 0.6f [0x858CC8])`; then if `m_GasPedal > 0.5f` and
      `DotProduct(&moveSpeed, &matrix.forward) > 0.25f` (0x40FDB0, cdecl -> ST0): `ApplyMoveForce((ts * m_fMass * 0.0016f[0x87149C]) * matrix.forward)`
      (0x40FE90 operator*(out, float, const CVector*)); `nBikeFlags |= 0x20` (bPlayerBoost).
      `wasSubmerged` EDI = (m_nPhysicalFlags(+0x40) >> 8) & 1 (bSubmergedInWater). `CPhysical::ProcessControl()` (0x5485E0, non-virtual);
      `m_fAirResistance = savedAir`; `CBike::ProcessBuoyancy()` (0x6B5FB0).
  B10. Breath (0x6B9DEF): if `!wasSubmerged && (m_nPhysicalFlags & 0x100)`: if `driver && driver->IsPlayer()`: `driver->ResetPlayerBreath()`
      (0x60A8A0); else (incl. no driver) for i = 0 .. m_nNumPassengers(+0x488, u8, re-read each iteration) - 1 over m_apPassengers (+0x464):
      if `p && p->IsPlayer()`: `p->ResetPlayerBreath()`.
  B11. Ratio rescale (0x6B9E55): for i = 0..3: `wr = 1.0f - m_fSuspensionLength[i](+0x770) / m_fLineLength[i](+0x780)`;
      `ratio[i] = (ratio[i] - wr) / (1.0f - wr)` (wr kept in ST; f32 store).
  B12. Contact points / spring dirs (0x6B9E86): for i = 0..3 (col line stride 0x20, colpoint stride 0x2C): if `ratio[i] < 1.0f`
      [0x871284]: `cp[i]` (LF0 + 0xC*i) = `m_aWheelColPoints[i].m_vecPoint - pos` (pos = matrix ? matrix.pos : placement.pos; z, y, x computed);
      `dir[i]` (LC0 + 0xC*i) = `Normalise(Multiply3x3(*matrix, colData->m_pLines[i].end - colData->m_pLines[i].start))` (line at
      colData+0x10; CVector::Normalise 0x59C910). Ratio >= 1: nothing here (cp set in B13).
  B13. Springs (0x6B9F78): `m_aWheelSkidmarkType[1] = m_aWheelSkidmarkType[0] = 0`, `m_bMoreSkidMarks[1] = [0] = 0`. For i = 0..3:
      - ratio < 1: `bias` L48 = handling->m_fSuspensionBiasBetweenFrontAndRear (+0xC0); for i = 2, 3: `bias = 1.0f - bias`.
        If `colpoint[i].normal.z > 0.35f`: `ApplySpringCollisionAlt(handling->m_fSuspensionForceLevel(+0xAC), dir[i], cp[i], ratio[i], bias,
        colpoint[i].normal, impulse[i])` (0x543D60; impulse L150 + 4*i); else `ApplySpringCollision(forceLevel, dir[i], cp[i], ratio[i], bias,
        impulse[i])` (0x543C90). Then `r = (i >= 2)`; `m_aWheelSkidmarkType[r] = g_surfaceInfos.GetSkidmarkType(colpoint[i].surfaceB(+0x23))`
        (0x55E630, thiscall on 0xB79538); if it == 2: `m_bMoreSkidMarks[r] = 1`. (Wheel 1 overwrites wheel 0's value, 3 overwrites 2.)
      - ratio >= 1: `cp[i] = Multiply3x3(*matrix, colData->m_pLines[i].end)`.
  B14. Contact speeds (0x6BA0FC): for i = 0..3: `spd[i]` (L120 + 0xC*i) = `this->GetSpeed(cp[i])` (0x542CE0, thiscall(CVector* out, CVector p));
      if `m_aGroundPhysicalPtrs[i]`: `spd[i] -= ground->GetSpeed(m_aGroundOffsets[i](+0x7D0 + 0xC*i))` (x, y, z).
  B15. Ground-normal spring dirs (0x6BA1E0): if `ratio[0] < 1 || ratio[1] < 1`: `N = ratio[0] < 1 ? colpoint[0].normal : colpoint[1].normal`;
      if `N.z > 0.35f`: `dir[0] = -N`; `N = ratio[1] < 1 ? colpoint[1].normal : colpoint[0].normal`; if `N.z > 0.35f`: `dir[1] = -N`.
      Same for the rear pair (0x6BA33E): `ratio[2] < 1 || ratio[3] < 1`; dir[2] from (2 else 3), dir[3] from (3 else 2). Negation via `fchs`.
  B16. Dampening (0x6BA49A): for i = 0..3: if `ratio[i] < 1`: `ApplySpringDampening(handling->m_fSuspensionDampingLevel(+0xB0), impulse[i],
      dir[i], cp[i], spd[i])` (0x543E90).
  B17. (0x6BA503) Recompute `spd[i]` exactly as B14.
  B18. Drive / brake (0x6BA5E1): `fwdSpeed` L80 = `(move.z*fwd.z + move.y*fwd.y) + move.x*fwd.x`;
      `acc = handling->m_transmissionData.CalculateDriveAcceleration(&m_GasPedal, &m_nCurrentGear(+0x4B4), &m_fGearChangeCount(+0x4B8),
      &fwdSpeed, NULL, NULL, m_NumDriveWheelsOnGround(+0x805), CCheat::m_aCheatsActive[28] (0x96914C, CHEAT_PERFECT_HANDLING))` (0x6D05E0,
      thiscall on handling+0x2C, `ret 0x20`); `L84 = acc / m_fVelocityFrequency(+0x94)`.
      `brake` L48 = `handling->m_fBrakeDeceleration(+0x94) * m_BrakePedal(+0x4A0) * ts`.
      If `status != PLAYER && status != 8 && (handling->m_nHandlingFlags(+0xD0) & 8)` (bNpcNeutralHandl): L10 = L64 = L34 = L8C = 1.0f.
      Else: `L10 = brakeBias(+0x98) + brakeBias` (fadd st0,st0); `L64 = (1.0f - brakeBias) * 2` (as `x + x`); `L34 = tractionBias(+0xA8) * 2`;
      `L8C = 2.0f - L34`. (L10/L64 = front/rear brake bias, L34/L8C = front/rear traction bias.)
  B19. Contact counts (0x6BA6E2): `m_NumDriveWheelsOnGroundLastFrame(+0x806) = +0x805`; `m_nNoOfContactWheels(+0x804) = 0`; `+0x805 = 0`.
      For i = 0..3 (16-bit counter): if `ratio[i] < 1`: `WheelCounts[i] = 4.0f`; else `w = WheelCounts[i] - ts; if (!(w > 0)) w = 0.0f;
      WheelCounts[i] = w`, and continue only if `w > 0`. Counted wheels: `++m_nNoOfContactWheels`; if i is 2 or 3: `+0x805 = 1`;
      if the new count == 1: `m_vecAveGroundNormal(+0x618) = colpoint[i].normal`, else `+= colpoint[i].normal` (a stale normal is used for an
      airborne wheel whose counter is still > 0).
  B20. (0x6BA7D0) If count > 0: `r = 1.0f / (float)count`; `avg = (avg.x*r, avg.y*r, avg.z*r)`; if `dot(matrix.up, avg) < -0.5f` [0x858F40]
      (order (uz*az + uy*ay) + ux*ax): `avg *= -1.0f` (each component `fmul -1.0`). Count == 0: `avg = (0, 0, 1)`.
  B21. Wheel contact positions (0x6BA8AE): `f` L4C = `ratio[0] < ratio[1] ? 0 : 1` (NaN -> 1); `r` EBX = `ratio[2] < ratio[3] ? 2 : 3`.
      `wpF` L90 = `Multiply3x3(*matrix, (0, lines[0].start.y, (lines[0].start.z - ratio[f] * m_fSuspensionLength[0]) - mi->m_fWheelSizeFront(+0x40) * 0.5f))`.
      `wpR` L9C = `Multiply3x3(*matrix, (0, lines[3].start.y (+0x64), (lines[2].start.z (+0x48) - ratio[r] * m_fSuspensionLength[2](+0x778)) -
      mi->m_fWheelSizeRear(+0x44) * 0.5f))` - NOTE the y comes from col line 3 and the z from col line 2 (literal).
      `trac` L38 = `handling->m_fTractionMultiplier(+0x28) * m_fExtraTractionMult(+0x794) * [0xC1C818] * 0.25f` (0xC1C818 = unnamed global,
      `[0xC1C814] * 10.0f`, see _static-init-E.md 0x853600).
  B22. Bar steer angle RideAnimData.BarSteerAngle (+0x644) (0x6BA9F7):
      - `status != PLAYER && bOnSideStand && !bGettingPickedUp`: if `+0x644 < 0.34906587f`: `+0x644 = ts * 0.02617994f[0x8631A8] + +0x644`
        (else unchanged).
      - else if `|move.x| < 0.01f && |move.y| < 0.01f && m_fSteerAngle == 0.0f`: `+0x644 = _CIpow(0.96 [f64 0x86F0E0], ts) * +0x644`.
      - else `k = 1.0f`, except when `fwdSpeed > 0.01f && (WheelCounts[0] > 0 || WheelCounts[1] > 0) && status == PLAYER`:
        build a stack CColPoint L160 with surfaceTypeA (+0x20) = 0x3C and surfaceTypeB (+0x23) = 1 (other fields uninitialised);
        `s = g_surfaceInfos.GetAdhesiveLimit(L160) * bh->m_fSpeedSteer(+0x20) * trac * 4.0f[0x858B90]`; if
        `GetAdhesionGroup(colpoint[r].surfaceB)` is 3 or 4 (two calls): `s *= bh->m_fSlipSteer(+0x24)`; `v = s / (fwdSpeed * fwdSpeed)`;
        `v = (1.0f < v) ? 1.0f : v`; `k = _CIasin(v) / (handling->m_fSteeringLock(+0xA0) * 0.017453292f)`; if `(m_fSteerAngle < 0.0 (f64) &&
        LeanAngle(+0x648) < 0) || (m_fSteerAngle > 0 && LeanAngle > 0)`: `k = k + k`; `k = (1.0f < k) ? 1.0f : k`.
        Finally if `status != PLAYER`: `k = 1.0f` (0x6BAC08). `+0x644 = k * m_fSteerAngle`.
  B23. Front wheel friction (0x6BAC22): `oldMove` L50 = m_vecMoveSpeed (copy). `rearFirst` L8B = `(handling->m_nHandlingFlags >> 23) & 1`
      (bProcRearwheelFirst). If rearFirst -> rear block (0x6BAF62) first [order/continuation below].
      If `!(WheelCounts[0] > 0 || WheelCounts[1] > 0)`: `m_aWheelAngularVelocity[0](+0x758) *= 0.95f[0x858EF0]` (fst) and
      `m_aWheelPitchAngles[0](+0x750) += it`; -> 0x6BAF62.
      Else: `wf` = `Multiply3x3(*matrix, (-sin(bar), cos(bar), 0))` (`fsin`/`fcos` on +0x644) -> L3C; `d = (wf.y*n.y + wf.z*n.z) + wf.x*n.x` with
      `n = colpoint[f].normal`; `wf -= d*n` (x, y, z; products formed as d*nz, d*ny, d*nx first); `Normalise(wf)`;
      `wr` L28 = `Normalise(CrossProduct(wf, n))` (0x59C730 cdecl(out, a, b) = a x b); if `knock` (L27): `wr.z = 0` (not renormalised).
      `[0xC1C27C] = 0` (dword); `colpoint[f].surfaceTypeA(+0x20) = 0x3C` (written into the member colpoint); `adh` L14 =
      `GetAdhesiveLimit(colpoint[f]) * trac`; `L74 = 1.0f`. If `m_fBrakingSlide(+0x7B0) > 0`: `g = GetAdhesionGroup(colpoint[f].surfaceB)`:
      g == 1 or 3 -> `L74 = 0.9f[0x8712F8]`; g == 2 -> jump 0x6BAFA5 [see B25]; else nothing.
      If status == PLAYER: `adh = GetWetMultiplier(colpoint[f].surfaceB) * adh` (0x55E600). If `m_nWheelStatus[0](+0x65C) == 1`: `adh *= 0.4f`.
      `[0xC1C26C] = m_WheelStates[0](+0x80C)` (dword copy). `cs` L68 = `GetSpeed(wpF)`; if `m_aGroundPhysicalPtrs[f]`: ...
      `cs -= ground->GetSpeed(m_aGroundOffsets[f])`. Then (0x6BAEE1):
      `ProcessBikeWheel(wf, wr, cs, wpF, 2, thrust = *(float*)0xC1C27C (= 0 here), brake = L10 * brake(L48), adhesion = adh * L34,
      destab = L74, wheelId 0, &m_aWheelAngularVelocity[0](+0x758), (tWheelState*)0xC1C26C, special 0, (u16)m_nWheelStatus[0])`
      (0x6D73B0, thiscall, 14 args; brake product formed as `fld L10; fmul L48`, adhesion `fld adh; fmul L34`).
      If `extraHandlingFlags & 4` and `[0xC1C26C]` is 1 or 2: `[0xC1C26C] = 0`.
      (g == 2 path 0x6BAFA5: `L74 = 0.7f[0x8712F4]`, then continues at 0x6BADFF.)
  B24. Rear wheel (0x6BAF62):
      - Rear airborne (`!(WheelCounts[2] > 0) && !(WheelCounts[3] > 0)`): if bIsHandbrakeOn (byte+0x428 & 0x20): `m_aWheelAngularVelocity[1]
        (+0x75C) = 0`. Else if `acc`(L84) `!= 0.0f` (equality via `test ah,0x44; jnp`; NaN counts as non-zero): if `acc > 0`: if `angVel1 < 1.0f`:
        `angVel1 = angVel1 - 0.1f`; else (`acc <= 0`/NaN): if `angVel1 > -1.0f`: `angVel1 = angVel1 + 0.05f`.
        Then `m_aWheelPitchAngles[1](+0x754) = ts * angVel1 + pitch1`. -> B25 (0x6BB4E6).
      - Rear touching (0x6BB038): `rbrake` L78 = brake(L48); `rtrac` L74 = trac(L38); `wr` L28 = matrix.right; `wf` L3C = matrix.forward;
        project wf on `colpoint[r].normal` exactly as the front (B23), Normalise; `wr = Normalise(CrossProduct(wf, n))`; if knock: `wr.z = 0`.
        * bIsHandbrakeOn: `rbrake = 20000.0f`; `m_fTyreTemp(+0x7AC) = 1.0f`.
        * else if `m_nBrakesOn` (word +0x7A8) != 0: `rbrake = 0; rtrac = 0`; `t = steer * m_fTurnMass * -0.0007f[0x8712F0]`;
          `v = (t*right.x*ts, t*right.y*ts, t*right.z*ts)` (products t*r rounded to f32 first, then *ts); `ApplyTurnForce(cp[2], v)` - the
          stack layout puts the contact point cp[2] (L108, fixed index 2) in the FORCE slot and v in the POINT slot (0x6BB157-0x6BB1ED); keep.
        * else if `m_fTyreTemp < 1.0f && m_GasPedal > 0.75f[0x858F34]`: `rtrac = trac * m_fTyreTemp`;
          `t = (1.0f - m_fTyreTemp) * steer * m_fTurnMass * -0.0007f`; same v and same `ApplyTurnForce(cp[2], v)`.
        0x6BB2D9: if `*(float*)0xC1C27C > 0 && brake(L48) > 0`: `brake(L48) = 0` (L48 only; rbrake already copied - affects a later front pass
        in rear-first order; the global holds 0 after a front pass, else the value left by the previous ProcessBikeWheel caller).
        `*(float*)0xC1C27C = acc`; `colpoint[r].surfaceTypeA = 0x3C`; `adh = GetAdhesiveLimit(colpoint[r]) * rtrac`; `destab` L84 = 1.0f
        (overwrites acc); if `m_fBrakingSlide > 0`: group 1/3 -> 0.9f, group 2 -> 0.7f (0x6BB52A); if PLAYER: `adh = wet * adh`;
        if `m_nWheelStatus[1](+0x65D) == 1`: `adh *= 0.4f`. `[0xC1C270] = m_WheelStates[1](+0x810)`; `cs = GetSpeed(wpR)`, minus ground[r] speed.
        `ProcessBikeWheel(wf, wr, cs, wpR, 2, thrust = acc (global), brake = rbrake * L64, adhesion = adh * L8C, destab, wheelId 1,
        &m_aWheelAngularVelocity[1], (tWheelState*)0xC1C270, special 1, (u16)m_nWheelStatus[1])`; extraHandlingFlags&4: state 1/2 -> 0.
  B25. Tyre temperature (0x6BB4E6): if `m_nBrakesOn != 0 && m_WheelStates[1](+0x810, member, not the global) == 1`:
      `m_fTyreTemp = m_fTyreTemp - ts * 0.002f[0x858F44]`; if `< 0`: `= 0`. Else if `m_fTyreTemp < 1.0f`: `m_fTyreTemp = ts * 0.005f[0x858B4C] + m_fTyreTemp`.
  B26. Front wheel, rear-first order (0x6BB566, only if rearFirst): same as B23 with these byte-level differences: airborne branch does
      `angVel0 *= 0.95f` (fst) then `pitch0 = angVel0 * ts + pitch0` (normal order adds WITHOUT ts); `wr` goes to L18 and the knock `wr.z = 0`
      step is ABSENT; destab lives in L4C (initial 1.0f; group 1/3 -> 0.9f, group 2 -> 0.7f via 0x6BB9BF); brake = `L10 * L48` (L48 possibly
      zeroed in B24). Same `[0xC1C27C] = 0`, surfaceTypeA write, wet/burst factors, `[0xC1C26C] = m_WheelStates[0]`, state reset on flag 4.
  B27. 0x6BB87A: `m_aGroundPhysicalPtrs[0..3] = NULL`. `standLean` L10 = 0; if driver: `a = GetAssoc(driver->clump, 195 BIKE_STILL)`;
      if a: `L10 = a.blend * 0.17453294f[0x871498]`.
      If knock: `m_vecAveGroundNormal = (0, 0, 1)`; `t = Normalise(CrossProduct(matrix.forward, avg))`; `avg = Normalise(CrossProduct(t, matrix.forward))`.
  B28. Lean (0x6BB955):
      - `!(extraHandlingFlags & 2) && !bGettingPickedUp`:
        * bOnSideStand: `P = _CIpow(0.97 f64, ts)` -> L48; `a = _CIasin(matrix.right.z)` (NOT clamped here, unlike B8);
          `D = P * DesiredLean - (1.0f - P) * ((a + standLean) + 0.2617994f)`.
        * else: `D = _CIpow(0.95 [f64 0x86C3C8], ts) * DesiredLean`.
      - else (rider): `m_vecGroundRight(+0x624) = Normalise(CrossProduct(matrix.forward, m_vecAveGroundNormal))`; side acceleration `s`:
        * `m_pAttachedTo(+0xFC) != NULL`: `s = 0.0f`;
        * `m_nNoOfContactWheels == 0`: `s = steer / (steeringLock * 0.017453292f) * ts * -0.004f[0x871494]`;
        * else `dv` = if `m_nPhysicalFlags & 4` (gr bDisableCollisionForce): `oldMove - m_vecOldSpeedForPlayback(+0x630)` and then
          `m_vecOldSpeedForPlayback = oldMove`; otherwise `moveSpeed - oldMove`; `s = (dv.y*gr.y + dv.z*gr.z) + dv.x*gr.x`.
        `dt = (0.01f > ts) ? 0.01f : ts`; `L14 = s / (dt * GAME_GRAVITY 0.008f)`; clamp to `[-lim, lim]` with `lim = bh->m_fMaxLean(+0x14)`, or
        `0.4f * MaxLean` if `m_nWheelStatus[0] == 1` (upper bound tested first; NaN passes unclamped).
        `P = _CIpow(bh->m_fDesLean(+0x1C), ts)`; `D = (_CIasin(L14) - standLean) * (1.0f - P) + P * DesiredLean`.
      Store `DesiredLean(+0x64C) = D`; `LeanAngle(+0x648) = DesiredLean` (dword copy).
  B29. 0x6BBBB3: `m_WheelStates[0], [1] (+0x80C/+0x810) = [0xC1C26C], [0xC1C270]`. If `m_GasPedal < 0` and `m_WheelStates[1] == 1`:
      `m_WheelStates[1] = 0`. If status == PLAYER: `ProcessSirenAndHorn(true)` (0x6E0950); else if `m_HornCounter(+0x514, u32) > 0`: `--`.
  B30. Fire / blow-up (0x6BBC18, also reached from the skip path B8): if `m_fHealth(+0x4C0) < 250.0f` and status != 5 (WRECKED):
      if `m_nVehicleSubType(+0x594) != 10` (BMX) and `m_pFireParticle(+0x57C) == NULL` and the RwFrame matrix exists (`m_pRwObject` (+0x18) ?
      `((RwFrame*)rwObject->parent)+0x10` : NULL): `pos` = `mi->m_pVehicleStruct(+0x5C)->m_avDummyPos[7]` (+0x54);
      `m_pFireParticle = g_fxMan.CreateFxSystem("fire_bike" [0x85AA5C], &pos, frameMatrix, false)` (0x4A9BE0); if created: `Play()` (0x4AA2F0);
      `CEventVehicleOnFire ev(this)` (0x4B10C0, stack L90, SEH trylevel 0); `GetEventGlobalGroup()->Add(&ev, false)` (0x4ABA50, 0x4AB420);
      trylevel -1; `ev.~()` (0x4B1170).
      Then `m_BlowUpTimer(+0x7BC) = (float)(uint32)_ftol(ts * 0.02f * 1000.0f) + m_BlowUpTimer` (fild + 2^32 if negative); if `> 5000.0f`:
      `BlowUpCar(m_Damager(+0x800), false)` (vtable+0xA4).
      Else: `m_BlowUpTimer = 0`; if `m_pFireParticle`: `Kill()` (0x4AA3F0), `= NULL`.
      `ProcessDelayedExplosion()` (0x6D1340).
  B31. Suspension history (0x6BBD7F): `sp2` L60 = x*x + y*y + z*z of moveSpeed; `maxComp` L14 = 0; `maxRough` L38 = 0. For i = 0..3:
      `diff = m_aRatioHistory[i] - ratio[i]` (L10); if `diff > 0.3f[0x858C24] && (i == 0 || i == 2) && sp2 > 0.04f[0x863244] &&
      (status == 0 || status == 3) && diff > maxComp`: `maxComp = diff`. If `ratio[i] < 1 && status == PLAYER`:
      `rough = (float)GetRoughness(colpoint[i].surfaceB) * 0.1f` (0x55E810); `if (!(maxRough > rough)) maxRough = rough`.
      `m_aRatioHistory[i] = ratio[i]; ratio[i] = 1.0f`.
  B32. 0x6BBE89: if `(CTimer::m_snTimeInMilliseconds & 0x7FF) > 800`: if `maxRough >= 0.29f[0x871268]` (`test ah,1`): `maxComp = 0`; `maxRough = 0`.
  B33. Pad shake (0x6BBEBE): if `(maxComp > 0 || maxRough > 0) && status == PLAYER`: `sp2` recomputed (x*x + y*y + z*z); if `sp2 > 0.0100000007f`
      [0x86CD7C] (`fcom`, value kept): `v = sqrt(sp2) / m_fMass` (fsqrt, then fdiv after the next compare);
      - `maxComp > 0`: `f = v * maxComp * 400000.0f[0x871264] + 100.0f[0x858628]`; `if (250.0f < f) f = 250.0f`; `freq = (u8)_ftol(f)`;
        `dur = _ftol(ts * 20000.0f[0x871260] / (int)freq)` (`fidiv` with the zero-extended byte); `CPad::GetPad(0)->StartShake(dur, freq, 0)`
        (0x53FB70, 0x53F920).
      - else: `f = v * maxRough * 400000.0f + 40.0f[0x858A10]`; clamp 150.0f[0x858A28]; `dur = _ftol(ts * 5000.0f / freq)`; same StartShake.
  B34. 0x6BC006: byte+0x42B &= 0xDE; if `!bWarnedPeds` (+0x429 & 0x20): `CCarCtrl::ScanForPedDanger(this)` (0x42CE40, cdecl).
  B35. 0x6BC028: if `(m_nPhysicalFlags & 4) && (m_nPhysicalFlags & 8)`: zero m_vecMoveSpeed, m_vecTurnSpeed, m_vecFrictionMoveSpeed(+0x5C),
      m_vecFrictionTurnSpeed(+0x68). Else if `!skip` and (`m_GasPedal == 0.0f` or status == WRECKED) and `|move.x| < 0.005f && |move.y| < 0.005f
      && |move.z| < 0.005f` [0x858B4C] and `!(m_fDamageIntensity > 0 && m_pDamageEntity(+0xDC) == CWorld::FindPlayerPed(-1))` (FindPlayerPed only
      called when intensity > 0): `move.z = 0` then `moveSpeed = (0,0,0)`.
  B36. Self-righting (0x6BC192): exit unless `(extraHandlingFlags & 2) || (nBikeFlags & 0x18)`.
      `d = (avg.z*right.z + avg.y*right.y) + right.x*avg.x` (fst L38); clamp: `> 1.0f -> 1.0f`, else `< -1.0f -> -1.0f`.
      `pt = Multiply3x3(*matrix, m_vecCentreOfMass) + matrix.right` (x, y, z); `t = d * m_fTurnMass`;
      - `extraHandlingFlags & 2`: `t *= -0.07f[0x8712E8]`; `ApplyTurnForce((t*up.x*ts, t*up.y*ts, t*up.z*ts), pt)` (t*u rounded to f32 first);
        `nBikeFlags &= ~0x10` (bOnSideStand).
      - else: `t *= -0.1f[0x8712EC]`; same ApplyTurnForce (flags untouched).
  B37. Player wheelie/stoppie assist (0x6BC34D): exit if status != PLAYER.
      Wheelie: if `!(WheelCounts[0] > 0) && !(WheelCounts[1] > 0) && fwd.z > 0 && (WheelCounts[2] > 0 || WheelCounts[3] > 0)`:
      `x = bh->m_fWheelieAng - fwd.z`; if `x > 0.15f[0x858FCC]`: `x = 0.3f - x; if (0.0f > x) x = 0`; else if `x < -0.08f[0x871490]`:
      `x = -0.14f[0x87148C] - x; if (0.0f < x) x = 0`; (else x unchanged). Then `spd = sqrt(moveSpeed.SqMag)` ...
      `m = (spd > 0.1f) ? 0.1f : sqrt(SqMag)` (recomputed; NaN -> recomputed); `L10 = x * bh->m_fWheelieStabMult(+0x38) * m`;
      `k = GetFatAndMuscleModifier(12) * L10` (the cdecl arg slot is left on the stack and reused as part of the next call's args).
      `pt = M*COM (L50 from B36) + matrix.forward`; `T = k * m_fTurnMass * ts * 0.5f`; `ApplyTurnForce(T * up, pt)` (T*u per component).
      `T2 = bh->m_fWheelieSteer(+0x34) * BarSteer(+0x644) * m_fTurnMass * ts * 0.5f`; `ApplyTurnForce(T2 * right, M*COM + forward)`.
      `q = sqrt(SqMag) * m_fMass * WheelieSteer * BarSteer * ts * [0x8D3288] (.data, init 0.01f)`; `F = (q*right.x, q*right.y, q*right.z) * sqrt(SqMag)`
      (speed recomputed, each component rounded then multiplied); `ApplyMoveForce(F)`; `LeanAngle = ts * BarSteer * [0x8D3284] (.data, init -0.1f)
      + LeanAngle`. Exit.
      Stoppie (0x6BC68C, else): if `!(WheelCounts[2] > 0) && !(WheelCounts[3] > 0) && fwd.z < 0 && (WheelCounts[0] > 0 || WheelCounts[1] > 0)`:
      `x = bh->m_fStoppieAng - fwd.z`; if `x > 0.15f`: `x = 0.3f - x; if (0 > x) x = 0`; else if `x < -0.15f[0x86C330]`: `x = -0.3f[0x858EE4] - x;
      if (0 < x) x = 0`. `m = (0.1f < spd) ? 0.1f : spd` (not recomputed); `k = GFM(12) * (x * bh->m_fStoppieStabMult(+0x3C) * m)`;
      `ApplyTurnForce(k * TM * ts * 0.5f * up, M*COM + forward)`. Then `q = ((move.z*right.z + move.y*right.y) + right.x*move.x) * TM * ts *
      [0x8D3280] (.data, init 0.05f)`; `a = Normalise(CrossProduct((0,0,1), matrix.right))`; `ApplyTurnForce(-q * right, -a)`.
  Exit (0x6BC914): restore SEH frame, `ret` (no args).
- Control-flow summary: B1-B7 always; skip path B8 -> B30; physics path B9-B29 -> B30; then B30-B37. B23/B24/B26 order: normal
  front (B23) then rear (B24); with bProcRearwheelFirst rear (B24) then front (B26).
- Oddities / original bugs to keep: passenger0 not checked when there is no driver (B2); ProcessAI result ignored (B2); m_aRatioHistory[3]
  compared four times (B5); 0x424100 always false (B5); bForceProcessControl sense inverted vs gr CAutomobile (B6); rear wheel position mixes
  col line 3 (y) and line 2 (z) (B21); surfaceTypeA of the member colpoints is overwritten with 0x3C (B23/B24/B26); ApplyTurnForce gets the
  contact point as force and the torque vector as point (B24); file-static wheel state / thrust temps leak between calls and vehicles
  (B23/B24/B26/B29); airborne front wheel integrates pitch without ts in normal order but with ts in rear-first order (B23 vs B26); rear-first
  front pass lacks the knock `wr.z = 0` (B26); m_WheelStates[1] (member) is read for the tyre-temperature test before the globals are copied
  back (B25 vs B29); side-stand asin argument is clamped in B8 but not in B28; `fidiv` by a zero-extended byte in B33 (cannot be 0 since
  f >= 40).
- gta-reversed: `plugin::CallMethod<0x6B9250>` only (Bike.cpp:522), no C++. Its CAutomobile::ProcessControl (Automobile.cpp:394) has the same
  skeleton (fake physics, ground physicals, springs, wheel processing) and was used only as a naming aid; the bike differs in the points above.
- Open questions: (1) resolved: ApplyTurnForce 0x542A50 takes force first ([esp+4]) and point second (0x542AC3-0x542AFC computes
  `(point - M*COM) x force`), so in B24 cp[2] really is passed as the force and the scaled right vector as the point.
  (2) Names only: L27 "knock", the meaning of extraHandlingFlags bit 4, and the unnamed .data tweakables 0x8D3238..0x8D3288 / 0xC1C818.
  (3) 0x6BA9F7: the stack CColPoint L160 has only surfaceTypeA/B initialised; GetAdhesiveLimit reads only those (per gr), so the rest is
  irrelevant - verify when porting 0x55EB50.

## 0x6BC930 CBike::ProcessAI  (catalogue: gr-unreversed/REA, port level 23)
- Convention: virtual (vtable +0x108) `__thiscall bool (uint32& extraHandlingFlags [esp+4])`, `ret 4`, AL. Caller: CBike::ProcessControl.
- Callees: CPad::GetPad 0x53FB70, CPad::CarGunJustDown 0x53FFE0, CVehicle::ActivateBomb 0x6D24F0, CWorld::FindPlayerPed 0x56E210, virtual
  ProcessControlInputs (+0x60) and PlayCarHorn (+0xD8), CVector::Magnitude 0x4082C0, `min(a, b)` 0x404330 (cdecl, `a < b ? a : b`, NaN -> b),
  `max(a, b)` 0x420800 (`a > b ? a : b`, NaN -> b), CStats::GetFatAndMuscleModifier 0x559AF0, `operator+` 0x40FE30, `operator*(float, CVector)`
  0x40FE90, CPhysical::ApplyTurnForce 0x542A50, CEntity::PruneReferences 0x571A90, CVehicle::DoDriveByShootings 0x741FD0,
  CBike::DoSoftGroundResistance 0x6B6D40, CCarAI::UpdateCarAI 0x41DA30, CPhysical::ProcessControl 0x5485E0, CCarCtrl::UpdateCarOnRails 0x436540,
  CCarCtrl::SteerAICarWithPhysics 0x437C20, cTransmission::CalculateGearForSimpleCar 0x6D0530, CVehicle::ProcessWheelRotation 0x6D1230,
  CVector::MagnitudeSqr 0x406DA0.
- Globals: CVehicleRecording::bUseCarAI[] 0x97D6C0, CWorld::Players[0].m_pPed 0xB7CD98, Players[1].m_pPed 0xB7CF28, ms_fTimeStep.
  Constants: 0x858B50 0.0f, 0x8712FC 0.1f, 0x858B8C 0.5f, 0x858B38 0.02f, 0x858C58 0.01f; raw 1.0f, 0.05f.
- Behaviour:
  0. `m_nCarCtrlFlags &= 0xFC`. If `(int8)m_autoPilot.m_vehicleRecordingId (+0x424) >= 0` and `!bUseCarAI[id]`: `*flags += 2`; return
     false. `switch (status)` (table 0x6BCF80, 0..12; >12 -> return true).
  1. PLAYER (0): `*flags += 2`; `nBikeFlags &= ~0x08`. If `FindPlayerPed()->m_nPedState (+0x530)` is 61 or 58: skip to e.
     a. If `m_pDriver`: `== Players[0].m_pPed` -> `ProcessControlInputs(0)`; else `== Players[1].m_pPed` -> `ProcessControlInputs(1)`.
     b. Lean (bh = m_BikeHandling, h = m_pHandlingData): if `LeanFwd < 0.0f`: `COM.y = bh->LeanBakCOM * LeanFwd + h->COM.y`; apply when
        `(m_BrakePedal == 0.0f && !bIsHandbrakeOn) || m_nNoOfContactWheels == 0`: `m = min(0.1f, |ms|)`;
        `g = max(m / 0.1f, m_GasPedal) + m_GasPedal`; `k = ((((bh->LeanBakForce * m_fTurnMass) * LeanFwd) * m) * g) * 0.5f`.
        Else: `COM.y = bh->LeanFwdCOM * LeanFwd + h->COM.y`; apply when `m_BrakePedal < 0.0f || m_nNoOfContactWheels == 0`; `m` as above;
        `g = max(m / 0.1f, m_BrakePedal) + m_BrakePedal`; `k = ((((bh->LeanFwdForce * m_fTurnMass) * LeanFwd) * m) * g) * 0.5f`.
        Apply: `k = GetFatAndMuscleModifier(11) * k`; `ApplyTurnForce(-(ms_fTimeStep * k) * up, m_vecCentreOfMass + fwd)`.
     c. `PruneReferences()`; if status PLAYER `DoDriveByShootings()`; `DoSoftGroundResistance(flags)`.
     e. If `CPad::GetPad(0)->CarGunJustDown()` (always pad 0): `ActivateBomb()`. Return true.
  2. PLAYBACK (1): `*flags += 2`; return true.
  3. SIMPLE (2): `UpdateCarAI; CPhysical::ProcessControl; UpdateCarOnRails`; `+0x806 = old +0x805; +0x804 = 2; +0x805 = 2`;
     `CalculateGearForSimpleCar(m_autoPilot.m_speed * 0.02f, m_nCurrentGear)`; `rot[0] = ProcessWheelRotation(0, fwd, ms, wsF*0.5f) * ts + rot[0]`;
     `rot[1] = ProcessWheelRotation(0, fwd, ms, wsR*0.5f) * ts + rot[1]` (multiplied by the time step, unlike CBmx::ProcessAI);
     if `m_nCarCtrlFlags & 3` `PlayCarHorn()`; `if (m_HornCounter) m_HornCounter--`; `+0x42B &= 0xDE`; `nBikeFlags &= 0x7F`; return true.
  4. PHYSICS (3) and GHOST (12): `UpdateCarAI; SteerAICarWithPhysics`; if `m_nCarCtrlFlags & 3` `PlayCarHorn()`; `*flags += 2`;
     `nBikeFlags &= 0x7F`; if bIsBeingCarJacked: `bIsHandbrakeOn = 1, m_GasPedal = 0, m_BrakePedal = 1.0f`; else `nBikeFlags &= ~0x08`. Return true.
  5. ABANDONED (4): `m_BrakePedal = 0`; bIsHandbrakeOn = `(MagnitudeSqr(ms) < 0.01f) || bOnSideStand`; `m_GasPedal = 0; m_HornCounter = 0`;
     if `(m_pDriver || m_apPassengers[0] || bIsBeingCarJacked) && !bOnSideStand`: `*flags += 2`; `AnimLeanLeft = AnimLeanFwd = 0`;
     `nBikeFlags &= 0x7F`; if bIsBeingCarJacked: handbrake on, gas 0, brake 1.0f. Return true.
  6. WRECKED (5): handbrake on; `nBikeFlags &= 0x7F`; `m_BrakePedal = 0.05f`; steer, gas, horn counter = 0; AnimLeanLeft/Fwd = 0. Return true.
  7. FORCED_STOP (9): `if (MagnitudeSqr(ms) < 0.01f) { handbrake on; brake = 1.0f } else { handbrake off; brake = 0 }`; steer, gas,
     horn counter = 0; `*flags += 2`; `nBikeFlags &= 0x7F`. Return true. Statuses 6, 7, 8, 10, 11: return true.
- gta-reversed: plugin::Call (no C++).

## 0x6BD090 CBike::PreRender  (catalogue: gr-unreversed/REA, port level 14)
- Convention: virtual (CEntity slot 17, +0x44) `__thiscall void (CBike* this)`, esi = this, plain `ret` (0x6BDE18). SEH frame
  (`push -1; push 0x848321`), `sub esp, 0x19C` + 4 pushes; `L+n` = esp after the prologue (0x6BD0AF); EH state at L+0x1B4. Locals:
  colData at L+0x7C (and ebx), colModel at L+0xA4, CMatrix `mat` at L+0x28 (ctor inlined at 0x6BD66C/0x6BD670: +0x40 = L+0x68 = 0,
  +0x44 = L+0x6C = 0; dtor 0x6BDDFB), CMatrix `rot` at L+0xDC attached to the RwMatrix buffer L+0x16C (block H only), temp CMatrix
  L+0x124, CQuaternion L+0xCC, `savedPos` L+0x14..0x1C, wheel-particle speed L+0x20, steering vector L+0x70, steered forward L+0xC0,
  forward copy L+0xA8, wheel ground speed L+0x80.
- Callers: none direct (CBike vtable slot 17; CBmx/CQuadBike have their own PreRender). gta-reversed: `plugin::CallMethod<0x6BD090>`
  only (Bike.cpp:596, VMT hook `.Reversed = false`), no C++.
- Callees: as CBmx::PreRender 0x6C0810 (CVehicle::PreRender 0x6D6480, GetColModel 0x535300 x2, virtual +0xD0
  DoBurstAndSoftGroundRatios, AddSingleWheelParticles 0x6DE880, CalculateLeanMatrix 0x6B7150, StoreShadowForVehicle 0x70BDA0,
  Multiply3x3 0x59C790, GetSpeed 0x542CE0, ProcessWheelRotation 0x6D1230, CMatrix Attach/SetUnity/UpdateRW/Update/operator=/
  operator*=/operator*/~CMatrix/SetTranslate/SetRotateX/SetRotate(xyz)/RotateY, CVector::Normalise, CQuaternion::Set/Get, `_CIasin`
  0x821E70) plus: CVehicle::AddExhaustParticles 0x6DE240, CVehicle::AddDamagedVehicleParticles 0x6D2A80, CVehicle::DoVehicleLights
  0x6E1A60 (thiscall `(CMatrix&, uint32 flags)`), CCoronas::RegisterCorona 0x6FC580 (cdecl, 21 args, texture overload),
  CPointLights::AddLight 0x7000E0 (cdecl, 14 dwords), `_ftol` 0x821B40 (x2).
- Globals: ms_modelInfoPtrs 0xA9B0C8, ms_fTimeStep 0xB7CB5C, m_snTimeInMilliseconds 0xB7CB84, f32 0xC1C81C (= 0.277778f/50.0f, set by
  `__init_g_c1c81c` 0x853620), f32 0xB7C4E4 (CTimeCycle::m_CurrentColours+0x44). Constants: 0x858624 1.0f, 0x871284 1.0f, 0x858B50
  0.0f, 0x858F34 0.75f, 0x858C98 0.8f, 0x871250 130.0f, 0x8714AC 25.5f, 0x858B8C 0.5f, 0x85C6D4 0.0009765625f (1/1024), 0x8595EC
  0.017453292f, 0x858C1C -1.0f, 0x858C28 0.05f, 0x858CA8 0.07f, 0x858C20 0.9f, 0x871290 -0.05f; immediates 0.28f/0.6f/0.3f, 0.4f, 40.0f,
  1.5f, 30.0f, 10.0f.
- Fields: as CBmx::PreRender (same CBike offsets), plus +0x40 physical flags (0x20000000 = bit 29, bRenderScorched in gr's order),
  +0x384 handling (+0xCC model flags, 0x1000 = NO_EXHAUST), +0x428 bit 4 bEngineOn, +0x42B bit 6 bIsDrowning, +0x42D bit 7
  bSirenOrAlarm, +0x5CC m_mLeanMatrix. Bike nodes (gr eBikeNodes): +0x5A4 CHASSIS, +0x5A8 FORKS_FRONT, +0x5AC FORKS_REAR, +0x5B0
  WHEEL_FRONT, +0x5B4 WHEEL_REAR, +0x5B8 MUDGUARD, +0x5BC HANDLEBARS.

### A. 0x6BD0B1-0x6BD1DC: base pre-render + suspension heights
- Byte-identical logic to CBmx::PreRender block A (same offsets, constants 0x858624 / 0x858B50 / 0x858F34; temp at L+0x24): if
  +0x42B bit 0: virtual +0xD0, then `h[0]` from spring[0]/lineLen[0]/ratio[0..1]/line[0].start.z and `h[1]` from
  spring[2]/lineLen[2]/ratio[2..3]/line[2].start.z, each `h = (p - h) * 0.75f + h`.

### B. 0x6BD1E2-0x6BD39B: wheel particles (status 0..3)
- `spin (u8 L+0x13) = (m_WheelStates[1] (+0x810) == 2)`. `speed (L+0x20) = sqrt((vx*vx + vy*vy) + vz*vz)`.
- Loop i = 0, 1: column `c` selection identical to CBmx block B (constant 1.0f from 0x871284). `flags = (i == 0 && !spin) ? 4 : 0`
  (0x6BD299..0x6BD2A7; differs from CBmx, which uses `i == 0 || state[1] == 3`). Then `k = sin(Lean) * colModel.bbox.min.z * 0.8f`,
  `pos = cp[c].point + right*k`, `+1` bloody, `+2` more-skid, `side = (Lean > 0) ? -1.0f : 1.0f` and the AddSingleWheelParticles call
  exactly as CBmx (same argument list, `hist[c]`, &pos at L+0x14).

### C. 0x6BD39F-0x6BD40A: lean matrix, exhaust, damage particles
1. `byte +0x5C8 = 0; CalculateLeanMatrix()`.
2. `fwdKmh = ((f.z*v.z + f.y*v.y) + f.x*v.x) / [0xC1C81C]` (forward = matrix +0x10..; kept in ST0).
3. If bEngineOn (+0x428 & 0x10) and `!(handling->+0xCC & 0x1000)` and `fwdKmh < 130.0f` (NaN -> no) and !bIsDrowning (+0x42B & 0x40):
   `AddExhaustParticles()`. (ST0 popped on every path.) Then always `AddDamagedVehicleParticles()`.

### D. 0x6BD40F-0x6BD650: police-bike siren lights
- Condition: model id (i16 +0x22) == 0x20B (523, COPBIKE) && byte +0x42D bit 7 && +0x428 bit 4 && !(dword +0x40 & 0x20000000).
  Otherwise skip (ebp = 0) to E.
- `t = m_snTimeInMilliseconds & 0x1FF`; `c = _ftol([0xB7C4E4] * 25.5f)` (fld/fmul before the branch, ftol in each branch; colour byte =
  low 8 bits). `L = (0.28f, 0.6f, 0.3f)` at L+0x70, `R = (-0.28f, 0.6f, 0.3f)` at L+0xB4 (entity-relative).
- If `t < 0x100`: `a = (u8)t`; `RegisterCorona(id = (u32)this + 0x15, attachTo = this, r = c, g = 0, b = 0, alpha = 0xFF, &R, radius
  0.4f, farClip 40.0f, texture nullptr, flare 0, reflection false, checkObstacles false, unused 0, angle 0.0f, longDistance false,
  nearClip 1.5f, fadeState 0, fadeSpeed 30.0f, onlyFromBelow false, reflectionDelay true)`.
  Else: `a = (u8)(-(u8)t)` (`neg al`); `RegisterCorona(id = this + 0x16, this, r = 0, g = 0, b = c, 0xFF, &L, same remaining args)`.
  In both cases `bl = (u8)(0xFF - a)` (byte subtraction) and `a` is stored in L+0x13.
- Light position (0x6BD53B..0x6BD5D9): `P = &m_matrix->pos` (or `&m_placement.pos` (+4) if m_matrix == null); with f = forward, u =
  up of m_matrix: `X = (P.x + f.x) + u.x*0.5f`, `Y = (f.y + P.y) + u.y*0.5f`, `Z = (f.z + P.z) + u.z*0.5f` (`u.x*0.5f`, `u.y*0.5f`, `P.x + f.x`
  and `f.y + P.y` are stored f32 first; `f.z + P.z` and `u.z*0.5f` stay in x87 registers).
- `CPointLights::AddLight(0, (X, Y, Z), (0, 0, 0), 10.0f, red = float(a) * (1/1024), green = 0.0f, blue = float(bl) * (1/1024),
  fogType 0, castShadows true, entity nullptr)` (fild of the zero-extended byte, fmul 0x85C6D4).

### E. 0x6BD654-0x6BD6FA: vehicle lights, shadow, steering vector
- `DoVehicleLights(m_mLeanMatrix (+0x5CC), 4)`; `StoreShadowForVehicle(this, 1)`; `mat` ctor; EH 0; `mi = ms_modelInfoPtrs[model]`;
  `v = (-sin(steer), cos(steer), 0)`; `steeredFwd = Multiply3x3(m_matrix, &v)`; `fwdCopy = m_matrix->forward` (as CBmx block C).

### F. 0x6BD6CC-0x6BD7E6: front wheel spin; G. 0x6BD7EC-0x6BD8E9: rear wheel spin
- Byte-identical to CBmx blocks D and E: front only if `counts[0] > 0 || counts[1] > 0`, point `(0, (line1.y + line0.y)*0.5f,
  (line0.z - min(hist0,hist1)*spring[0]) - mi->wheelSizeFront*0.5f)`, `ProcessWheelRotation(0 /*ebp = 0 constant*/, &steeredFwd,
  &spd, mi->wheelSizeFront*0.5f)`, `angVel[0]`, `pitch[0] += w*timeStep`; rear with counts[2..3], lines 2/3, spring[2], z offset again
  from `mi+0x40` (front size, 0x6BD868), `ProcessWheelRotation(m_WheelStates[1], &fwdCopy, &spd, mi->wheelSizeRear*0.5f)`.

### H. 0x6BD8EF-0x6BDAFE: front forks (+0x5A8, null-checked) and handlebars (+0x5BC, null-checked inside)
- Identical to CBmx block F (axis from `mi->m_fBikeSteerAngle`, quaternion angle `-BarSteerAngle`, `mat = I * rot`, pos += savedPos),
  but the second node is HANDLEBARS +0x5BC: status 4/5 -> `SetUnity; mat *= rot; pos += savedPos`, otherwise `SetTranslate(savedPos)`;
  UpdateRW; `~rot`.

### I. 0x6BDB03-0x6BDB94: rear forks (+0x5AC, null-checked)
- Identical to CBmx block G: `SetRotate(-asin((h[1] - oh[1]) / swingArmLength), 0, 0)`, pos += savedPos.

### J. 0x6BDB99-0x6BDC69: front wheel (+0x5B0, NOT null-checked)
- Attach + savedPos, then unconditionally (no 0xC1C83C test, unlike CBmx): `savedPos.z = f32(h[0] - forkZOffset)` (+0x760 - +0x7A0);
  `savedPos.y = f32((((line[1].start.y + line[0].start.y) * 0.5f) - forkYOffset) - (h[0] - oh[0]) * steerTan)`.
- If wheelStatus[0] (+0x65C) == 1: `SetRotate(pitch[0], 0, f32(sin(pitch[0]) * 0.05f))` (0x858C28), else `SetRotateX(pitch[0])`.
  pos += savedPos; UpdateRW.

### K. 0x6BDC6E-0x6BDC9E: mudguard (+0x5B8, NOT null-checked)
- `mat.Attach(&node->modelling, false)`; `mat.pos = savedPos` (the front wheel's adjusted (x, y', z'), plain copy, orientation of
  the mudguard frame kept); UpdateRW.

### L. 0x6BDCA3-0x6BDD35: rear wheel (+0x5B4, NOT null-checked)
- Attach + savedPos (no height override). If wheelStatus[1] (+0x65D) == 1: `SetRotate(pitch[1], 0, f32(sin(pitch[1]) * 0.07f))`
  (0x858CA8), else `SetRotateX(pitch[1])`. pos += savedPos; UpdateRW.

### M. 0x6BDD3A-0x6BDDE7: chassis (+0x5A4, null-checked)
- Attach + savedPos; `savedPos.z = f32((1.0f - cos(Lean)) * colModel->bbox.min.z * 0.9f)` (replaces the frame z, no suspension
  pitch/offset term unlike CBmx); `SetRotateX(f32(fabs(Lean) * -0.05f))` (0x871290); `RotateY(Lean)` (raw +0x648, no sprint lean);
  pos += savedPos; UpdateRW.

### N. 0x6BDDEC-0x6BDE18: EH -1, `~mat`, restore fs:[0], return.

- Oddities to keep: constant wheel state 0 for the front ProcessWheelRotation; rear contact z uses the front wheel size; nodes
  +0x5B0/+0x5B4/+0x5B8 not null-checked; corona ids are `this + 0x15` / `this + 0x16` (pointer arithmetic); the light's red/blue use
  the byte `a`/`0xFF - a`, so at t = 0x100 the red channel is 0 and at t = 0x101 it jumps to 255/1024; burst-wheel wobble factors
  0.05 front / 0.07 rear (CBmx: 0.02 / 0.04).
- Open questions: none affecting behaviour (gr name of physical flag bit 29 taken from gr's bitfield order).

## 0x6BE310 CBike::ProcessControlInputs  (catalogue: gr-unreversed/REA, port level 10)
- Convention: virtual (vtable +0x60) `__thiscall void (uint8 playerNum [esp+4])`, `ret 4` (arg slot reused as an int temp).
  Callers: CBike::ProcessAI 0x6BC930 (2 sites).
- Callees: CPad::GetPad 0x53FB70 (24 calls; `GetPad(playerNum)` except the last block, which uses pad 0), GetExitVehicle 0x5400D0,
  GetHandBrake 0x540040, GetSteeringLeftRight 0x53FB80, GetSteeringUpDown 0x53FBD0, GetAccelerate 0x5403F0, GetBrake 0x540080,
  `_CIpow` 0x822130, CWorld::FindPlayerPed 0x56E210, CPlayerPed::KeepAreaAroundPlayerClear 0x60C1E0.
- Globals: CCamera::m_bUseMouse3rdPerson 0xB6EC2E, CVehicle::m_bEnableMouseSteering 0xC1CC02, CVehicle::m_nLastControlInput 0xC1CC04 (RW),
  mouse X/Y 0xB73424/0xB73428, CTimer::ms_fTimeStep, CTimer::m_snTimeInMilliseconds, float 0xC1C804 (W then R: the signed squared steer),
  CVehicleRecording::bUseCarAI 0x97D6C0, CGameLogic::SkipState 0x96A8A0. Constants: 0x858B50 0.0f, 0x858B88 1/128, 0x871300 0.2f,
  0x858CC4 0.2f, 0x871304 -0.0035f, 0x871308 0.35f, 0x87130C 0.98f, 0x858C1C -1.0f, 0x858624 1.0f, 0x859A3C 1/255, 0x858C58 0.01f,
  0x858A28 150.0f, 0x8595EC pi/180, 0x859018 0.08f, 0x858B10 0.03f, 0x871254 0.28f.
- Fields: +0x58C m_fRawSteerAngle, +0x650 m_RideAnimData.LeanFwd, +0x49C gas, +0x4A0 brake, +0x494 m_fSteerAngle, +0x7A8 m_nBrakesOn
  (written as dword 1), +0x594 sub-type (10 = BMX), +0x424 m_autoPilot.m_vehicleRecordingId (int8), handling +0xA0 m_fSteeringLock.
- Behaviour:
  1. `fs = (ms.z*fwd.z + ms.y*fwd.y) + ms.x*fwd.x` (stored f32).
  2. `bIsHandbrakeOn = GetExitVehicle() ? 1 : (GetHandBrake() != 0)`.
  3. Steering/lean. Pad path P: `t = ts`; `raw = raw + ((float)(-LR) * (1/128) - raw) * t * 0.2f`; `lf = lf + ((float)(-UD) * (1/128) - lf) * t * 0.2f`
     (UD is also negated). With mouse steering enabled (`m_bUseMouse3rdPerson && m_bEnableMouseSteering`): mouse path M if `mouseX != 0`
     or `mouseY != 0`, or if `|raw| > 0`, `m_nLastControlInput == 2` and both pad axes are 0. Otherwise, if both pad axes are 0 and
     `m_nLastControlInput == 2` keep the values; else `m_nLastControlInput = 0` and P. Without mouse steering: P (m_nLastControlInput
     untouched). M: `m_nLastControlInput = 2`; if `NewState.m_bVehicleMouseLook == 0`: `raw = mouseX * -0.0035f + raw`,
     `lf = mouseY * -0.0035f + lf`; then `raw *= pow(0.98f, ts)` if `|raw| < 0.35f` or mouse-look is on; same rule for `lf`.
  4. Clamp raw then lf: `if (!(v > -1.0f)) v = -1.0f; else if (!(v < 1.0f)) v = 1.0f` (NaN becomes -1.0f - differs from the heli clamp).
  5. Pedals: `v = (float)(GetAccelerate() - GetBrake()) * (1/255)`.
     - If `|fs| < 0.01f`: if `GetAccelerate() > 150 && GetBrake() > 150 && sub-type != 10`: `gas = acc/255, brake = brk/255, m_nBrakesOn = 1`
       (burnout); else `gas = v, brake = 0`.
     - Else if `fs < 0.0f` (rolling back): `v < 0 -> gas = v, brake = 0`; else `brake = v, gas = 0`.
     - Else: `v < 0 -> brake = -v, gas = 0`; else `gas = v, brake = 0`.
  6. `s2 = raw * raw` with the sign of raw, stored to 0xC1C804; unless (`recordingId >= 0 && !bUseCarAI[recordingId]`):
     `m_fSteerAngle = handling->m_fSteeringLock * (pi/180) * s2`.
  7. bComedyControls (+0x429 & 0x10): if `(now & 0x3C00) < 0x3000` `gas = 1.0f`; if `(((now >> 10) + 6) & 0xF) < 12` `brake = 0`;
     handbrake off; `m_fSteerAngle += (now & 0x800) ? 0.08f : -0.03f`.
  8. If `GetPad(0)->DisablePlayerControls != 0 && SkipState != 2`: `brake = 1.0f`, handbrake on, `gas = 0`;
     `FindPlayerPed()->KeepAreaAroundPlayerClear()`; if `|ms| > 0.28f`: `ms *= 0.28f / |ms|` (per component, stored f32).
- gta-reversed: plugin::Call (no C++).

## 0x6BEA10 CBike::BlowUpCar  (catalogue: gr-unreversed/REA, port level 17)
- Convention: virtual (vtable +0xA4) `__thiscall void (CEntity* damager [esp+4], bool hideExplosion [esp+8])`, `ret 8`.
  Callers: CPlayerInfo::BlowUpRCBuggy, CBike::ProcessControl, CVehicle::ProcessDelayedExplosion, CVehicle::InflictDamage.
- Callees: CVisibilityPlugins::SetClumpForAllAtomicsFlag 0x7322F0 (cdecl(RpClump*, int flag)), CCamera::CamShake 0x50A9F0 (thiscall on
  TheCamera 0xB6F028, (float strength, float x, float y, float z)), CVehicle::KillPedsInVehicle 0x6D1C80, CVehicle::ChangeLawEnforcerState
  0x6D2330 (thiscall(bool)), CExplosion::AddExplosion 0x736A50 (cdecl, 10 dwords), CDarkel::RegisterCarBlownUpByPlayer 0x43DF20 (cdecl(CVehicle*, int)).
- Constant 0x859020 = 0.13f.
- Behaviour:
  1. If `!(byte +0x42A & 0x20)` (bCanBeDamaged, upper-flags bit 21) return.
  2. `m_vecMoveSpeed.z += 0.13f`; status := 5 (WRECKED) (byte +0x36 = (byte & 7) | 0x28); physical flags |= 0x20000000 (bRenderScorched);
     `SetClumpForAllAtomicsFlag(m_pRwClump, 0x4000)`; `m_fHealth = 0` (int store); `m_wBombTimer` (+0x4DE, u16) = 0.
  3. `TheCamera.CamShake(0.4f, pos.x, pos.y, pos.z)` (pos = matrix pos or placement pos, read per component).
  4. `KillPedsInVehicle()`; byte +0x4A8 &= 0xE7 (m_nOverrideLights = 0); byte +0x428 &= 0xAF (bEngineOn = 0, bLightsOn = 0);
     `ChangeLawEnforcerState(false)`.
  5. `AddExplosion(this, damager, 4 /*EXPLOSION_TYPE 4*/, pos, 0 /*lifetime*/, 1 /*sound*/, -1.0f /*cam shake*/, hideExplosion)` -
     the 8th argument receives `hideExplosion` unchanged.
  6. `CDarkel::RegisterCarBlownUpByPlayer(this, 0)` (called unconditionally, whoever the damager is).
- gta-reversed: plugin::Call (no C++).

## 0x6BEB20 CBike::BurstTyre  (catalogue: gr-unreversed/REA, port level 9)
- Convention: virtual (vtable +0xB0) `__thiscall bool (uint8 tyreComponentId [esp+4], bool physicalEffect [esp+8])`, `ret 8`, AL; SEH frame
  (two CEventKnockOffBike locals). Caller: CVehicle::InflictDamage 0x6D7C90.
- Callees: CAEVehicleAudioEntity::AddAudioEvent(int, float) 0x4F6420 (on this+0x138), CCarCtrl::SwitchVehicleToRealPhysics 0x423FC0,
  CGeneral::GetRandomNumberInRange(float, float) 0x41BD90 (cdecl -> ST0; the ORIGINAL helper, see cat14 finding 1), CPhysical::ApplyMoveForce
  0x5429F0, CPhysical::ApplyTurnForce 0x542A50, CEventKnockOffBike ctor 0x4AFCF0 (11 args) / dtor 0x4AFE70, CEventGroup::Add 0x4AB420
  (thiscall(CEvent*, bool)), `operator*(float, CVector)` 0x40FE90.
- Globals (.data f32): 0x8D322C = 0.02, 0x8D3230 = 0.3, 0x8D3234 = 0.55. Constant 0x871284 = 1.0f.
- Behaviour:
  1. If byte +0x42B bit 7 (bTyresDontBurst) or physical flag 0x20000000 return false (0x6BEB40-0x6BEB56).
  2. `w = id`; if `id == 13` `w = 0`; else if `id == 15` `w = 1`; any other id is used unchanged as the index (no bounds check; e.g. 14
     reads byte +0x66A). `result = false`.
  3. If `m_nWheelStatus[w] == 0`: `m_nWheelStatus[w] = 1`; `m_vehicleAudio.AddAudioEvent(90, 0.0f)`; if status == SIMPLE (2)
     `SwitchVehicleToRealPhysics(this)`. If `physicalEffect`:
     `f = GetRandomNumberInRange(-0.02f, 0.02f) * m_fMass`; `ApplyMoveForce(f * right)` (right = matrix +0x00);
     `g = GetRandomNumberInRange(-0.02f, 0.02f) * m_fTurnMass`; `ApplyTurnForce(g * right, fwd)`.
     `result = true` (also when physicalEffect is false).
  4. Knock-off (0x6BECA5), only if `m_pDriver`. NOTE: it tests the remapped `w` (BL) against 13 and 14:
     - `w == 13` (unreachable: 13 was remapped to 0): continue if `m_aRatioHistory[0] < 1.0f || m_aRatioHistory[1] < 1.0f`.
     - `w == 14` (only for id 14): continue if `m_aRatioHistory[2] < 1.0f || m_aRatioHistory[3] < 1.0f` (+0x728/+0x72C).
     - otherwise return `result`.
     Then `s = sqrt((ms.x^2 + ms.y^2) + ms.z^2)`; require `s > 0.3f` and, if status == PLAYER, `s > 0.55f`; else return `result`.
     - `w == 13`: `CEventKnockOffBike ev(this, &m_vecMoveSpeed, &this->+0xE0 (last collision impact velocity), 0.0f, 0.0f, 0x31, 0, 0,
       nullptr, true, false)`; `m_pDriver->m_pIntelligence->m_eventGroup.Add(&ev, false)`; if `m_apPassengers[0]`: same event with
       `isVictimDriver = false` added to the passenger's group; destroy the events.
     - `w == 14`: `ApplyTurnForce((0.02f * m_fTurnMass + same) * right, fwd)` (`fadd st0,st0`, i.e. `2 * (0.02f*turnMass)`).
     Return `result`.
- Edge cases (keep): because of the remap the rider is never knocked off by a front (13) or rear (15) burst; only id 14 reaches the
  rear branch, which applies a turn force. Ids other than 13/15 index m_nWheelStatus out of range.
- gta-reversed: plugin::Call (no C++).

## 0x6BEEB0 CBike::PlaceOnRoadProperly  (catalogue: gr-unreversed/REA, port level 8)
- Convention: `__thiscall void ()`, plain `ret`. Callers: CCarCtrl::SwitchBetweenPhysicsAndGhost (tail call), CreatePoliceChase,
  CreateConvoy, CreateCarForScript, SetCoordsOfScriptCar, CCheat::VehicleCheat, CGameLogic::UpdateSkip, 0x4485C0,
  CMissionCleanup::CheckIfCollisionHasLoadedForMissionObjects.
- Callees: CEntity::GetColModel 0x535300 (x2), CWorld::ProcessVerticalLine 0x5674E0 (cdecl(const CVector& origin, float zEnd,
  CColPoint& out, CEntity*& outEntity, bool buildings, vehicles, peds, objects, dummies, seeThrough, CStoredCollPoly* poly), x2),
  CVector::CrossProduct(out, a, b) 0x59C730, virtual GetHeightAboveRoad (vtable +0xD4, ST0).
- Constants: 0x858C80 = 5.0f, 0x858B8C = 0.5f.
- Behaviour:
  1. `frontY = GetColModel()->bbox.max.y` (+0x10); `rearY = -GetColModel()->bbox.min.y` (+0x04).
     `fwd = m_matrix->forward` (read with NO null check of m_matrix, 0x6BEECC-0x6BEEEA); `p = m_matrix ? m_matrix->pos : placement.pos`.
     `rear.x = p.x - fwd.x*rearY`, `rear.y = p.y - fwd.y*rearY`; `front.x = fwd.x*frontY + p.x`, `front.y = fwd.y*frontY + p.y`.
  2. Front probe: `ProcessVerticalLine((front.x, front.y, p.z + 5.0f), p.z - 5.0f, cp, ent, true, false x5, nullptr)`.
     Hit: `frontZ = cp.m_vecPoint.z`; dword `+0x544` (m_FrontCollPoly.ligthing, written as a zero-extended 32-bit store) =
     `cp.m_nLightingB` (+0x25); `m_pEntityWeAreOn = ent`; copy bits 30 (bTunnel) and 31 (bTunnelTransition) of `ent->m_nFlags` (+0x1C)
     into this->m_nFlags (0x6BEFC9-0x6BF003). Miss: `frontZ = p.z`.
  3. Rear probe: same from `(rear.x, rear.y, p.z + 5.0f)`; hit writes `rearZ`, dword `+0x570` (m_RearCollPoly.ligthing), entity, tunnel bits;
     miss: `rearZ = p.z`. (`ent` is not reset between probes.)
  4. Orientation (0x6BF0F1-0x6BF1A7): `L = rearY + frontY`; `pitch = atan((frontZ - rearZ) / L)` (`fld1; fpatan`); `c = cos(pitch)`;
     `right.x = (front.y - rear.y) / L`; `right.y = -((front.x - rear.x) / L)`; `right.z = 0` (int store);
     `fwd.x = -(c * right.y)`; `fwd.y = c * right.x` (right components re-read as stored f32); `fwd.z = sin(pitch)`;
     `up = CrossProduct(right, fwd)` written to +0x20..+0x28.
  5. Position: `x = (front.x + rear.x) * 0.5f`, `y = (front.y + rear.y) * 0.5f`, `z = GetHeightAboveRoad() + (rearZ + frontZ) * 0.5f`;
     written to `m_matrix->pos` if m_matrix, else to placement pos (+4/+8/+0xC) (that branch is unreachable in practice because
     steps 1 and 4 already dereference m_matrix).
- gta-reversed: plugin::Call (no C++).

## 0x6BF230 CBike::GetCorrectedWorldDoorPosition  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall CVector* (CVector& out [esp+4], CVector a [esp+8..+0x10], CVector b [esp+0x14..+0x1C])`, `ret 0x1C`;
  EAX = &out on return (not used by gr's `void` signature). The `out` and `b` argument slots are reused as temps.
- Callers: CCarEnterExit::GetPositionToOpenCarDoor 0x64E740, CTaskUtilityLineUpPedWithCar::GetLocalPositionToOpenCarDoor 0x64FC10.
- Callees: CVector::CrossProduct(out, a, b) 0x59C730 (cdecl, `out = a x b`; both calls' args popped together by `add esp,0x18`),
  CEntity::GetColModel 0x535300 (thiscall -> CColModel*).
- Constant 0x858B50 = 0.0f. Fields: matrix (+0x14): forward +0x10, up +0x20, pos +0x30; placement pos this+4.
- Behaviour (exact order):
  1. `R = CrossProduct(fwd, (0,0,1))`; `U = CrossProduct(R, fwd)` (0x6BF236-0x6BF275).
  2. `dRU = (R.y*up.y + R.z*up.z) + R.x*up.x` (y-term first!) stored f32 (0x6BF27A-0x6BF29D).
  3. `cm = GetColModel()`; `g = 0.0f; if (cm->bbox.max.z (+0x14) > cm->bbox.max.x (+0x0C)) g = max.z - max.x` (ordered `>`;
     0x6BF2A6-0x6BF2C0). Note it really subtracts max.x (offset +0x0C), not min.z.
  4. `dy = b.y - a.y`; out = (0,0,0); `out = (dy*fwd.x + 0, dy*fwd.y + 0, dy*fwd.z + 0)` (each product + the zeroed out component,
     so a -0.0 becomes +0.0; 0x6BF2C7-0x6BF33B).
  5. `s = (b.x - a.x) + g*dRU`; `out.x += R.x*s; out.y += R.y*s; out.z += R.z*s` (0x6BF33E-0x6BF38B).
  6. `dz = b.z - a.z`; `out.x += U.x*dz; out.y += U.y*dz; out.z += U.z*dz` (0x6BF38E-0x6BF3CF).
  7. `p = m_matrix ? &m_matrix->pos : &m_placement.pos`; `out += p` (x, y, z) (0x6BF3D2-0x6BF3F5).
  All products and sums are f32-stored between steps (intermediate x87 values have 24-bit significands).
  So the result is `pos + dy*fwd + (dx + g*dot(R,up))*R + dz*U` with `R = fwd x Z`, `U = R x fwd` (an upright frame built from the
  forward vector only, plus a lean correction along R).
- gta-reversed: plugin::Call (no C++).
