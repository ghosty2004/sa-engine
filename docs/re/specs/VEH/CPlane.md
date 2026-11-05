# CPlane (VEH) - full specs

Group D, RE-SPEC-2. Authority: disassembly. CPlane derives from CAutomobile (layout in CHeli.md); CVehicle layout and compare idioms in
CBike.md. CPlane fields (size 0xA04): +0x988 m_fLeftRightSkid, +0x98C m_fSteeringUpDown, +0x990 m_fSteeringLeftRight,
+0x994 m_fAccelerationBreakStatus, +0x998 m_fAccelerationBreakStatusPrev, +0x99C m_fSteeringFactor, +0x9A0 m_StallCounter,
+0x9A4 m_planeCreationHeading, +0x9A8 m_maxAltitude, +0x9AC m_altitude, +0x9B0 m_minAltitude, +0x9B4 m_planeHeading,
+0x9B8 m_planeHeadingPrev, +0x9BC m_forwardZ, +0x9C0 m_nStartedFlyingTime, +0x9C4 m_fPropSpeed, +0x9C8 field_9C8,
+0x9CC m_LandingGearAngle, +0x9D0 m_planeDamageWave, +0x9D4 m_pGunParticles, +0x9D8 m_nFiringMultiplier, +0x9DC m_FireMissilePressedTime,
+0x9E0 m_LastMissileTarget, +0x9E4 m_LastHSMissileStruct, +0x9E8 m_apJettrusParticles[4], +0x9F8 m_pSmokeParticle, +0x9FC m_nSmokeTimer,
+0xA00 m_bSmokeEjectorEnabled. CPlane vtable 0x871948. Plane nodes (ePlaneNodes): 12 STATIC_PROP .. 16 RUDDER, 17/18 ELEVATOR_L/R,
19/20 AILERON_L/R, 21/22 GEAR_L/R.

## 0x6C9160 CPlane::~CPlane  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall void ()`, plain `ret`; SEH frame. Caller: CPlane scalar deleting dtor 0x6CADB0.
- Callees: as CHeli::~CHeli (GetPlaneNumGuns 0x6D3F30, FxSystem_c::Kill 0x4AA3F0, FxManager_c::DestroyFxSystem 0x4A9810, `free` 0x8213AE,
  CAEVehicleAudioEntity::Terminate 0x4FB8C0, CAutomobile::~CAutomobile 0x6A61E0).
- Behaviour: `vtable = 0x871948`; if `m_pGunParticles` (+0x9D4) != null: same loop as CHeli (Kill + DestroyFxSystem per non-null gun
  particle, bound re-evaluated each iteration), `free(m_pGunParticles)`, `m_pGunParticles = nullptr`.
  Then for `i = 0..3`: if `m_apJettrusParticles[i]` (+0x9E8 + 4i): `Kill()` and set it to null (no DestroyFxSystem).
  If `m_pSmokeParticle` (+0x9F8): `Kill()`, set null. `m_vehicleAudio.Terminate()`; `CAutomobile::~CAutomobile()`.
- gta-reversed: not reversed separately (header only); behaviour from the bytes.

## 0x6C9260 CPlane::ProcessControl  (catalogue: gr-unreversed/REA, port level 26)
- Convention: virtual (vtable +0x28) `__thiscall void ()`, plain `ret`. No static callers (vtable only).
- Callees: CPad::GetPad 0x53FB70 (cdecl(int)), CDamageManager::SetWheelStatus 0x6C21A0 (thiscall(int, int)), CAutomobile::ProcessControl
  0x6B1880, CVehicle::ProcessWeapons 0x6E3950, `_ftol` 0x821B40, FxSystem_c::GetCompositeMatrix 0x4AA8C0 (thiscall(RwMatrix* out)),
  FxPrtMult_c ctor 0x4AB290, FxSystem_c::AddParticle(CVector*, CVector*, float, FxPrtMult_c*, float, float, float, uint8) 0x4AA440
  (on g_fx smoke system ptr 0xA9AE20), FxSystem_c::Kill 0x4AA3F0.
- Constants: 0x858B38 0.02f, 0x859948 -1000.0f, 0x858C80 5.0f; immediates 0.1f, 1.0f, 0.2f, 0.05f, -1.0f, 1.2f, 0.6f.
- Behaviour:
  1. If status == PLAYER and model is 512 or 513: `pad = CPad::GetPad((m_pDriver && m_pDriver->m_nPedType == 1) ? 1 : 0)`;
     if `pad->NewState.ShockButtonR (+0x26) != 0 && pad->OldState.ShockButtonR (+0x56) == 0`: `m_bSmokeEjectorEnabled = !m_bSmokeEjectorEnabled`.
  2. If m_bSmokeEjectorEnabled and (`!bEngineOn` or bIsDrowning or `!m_pDriver`): `m_bSmokeEjectorEnabled = false`.
  3. Model 460 (Skimmer): `m_damageManager.SetWheelStatus(i, 2)` for i = 0..3.
  4. `CAutomobile::ProcessControl()`.
  5. `u16 [+0x1B4]` (inside m_vehicleAudio, entity +0x7C) = `(uint16)m_StallCounter`; then `m_StallCounter = 0` (both paths write the
     same low word).
  6. `ProcessWeapons()`. Model 539 (Vortex): `m_WheelStates[0..3]` (+0x968..+0x974) = 0.
  7. If `m_pSmokeParticle` (+0x9F8): `m_nSmokeTimer (+0x9FC) += _ftol(ms_fTimeStep * 0.02f * -1000.0f)`;
     `m = GetCompositeMatrix()`; `vel = (-ms.x*5.0f, -ms.y*5.0f, -ms.z*5.0f)`; `mult = FxPrtMult_c(0, 0, 0, 0.2f, 1.0f, 1.0f, 0.1f)`;
     `smoke->AddParticle(&m.pos, &vel, 0.0f, &mult, -1.0f, 1.2f, 0.6f, 0)`; again with time 0.05f.
     If `m_nSmokeTimer <= 0` (signed) or bIsDrowning: `Kill()`, `m_pSmokeParticle = nullptr`.
- gta-reversed: plugin::Call (no C++).

## 0x6C94A0 CPlane::PreRender  (catalogue: gr-unreversed/REA, port level 12)
- Convention: virtual (CEntity slot 17, +0x44) `__thiscall void (CPlane* this)`, esi = this, plain `ret` (0x6CAADD). SEH frame
  (`push -1; push 0x8485AB`), `sub esp, 0x128` + 4 pushes; `L+n` = esp after the prologue (0x6C94C1); EH state at L+0x140. Code
  extent: 0x6C94A0-0x6CAADE plus the obfuscation stub 0x407EBB (`lea edi, [esi+0x6AC]; jmp 0x6CA63E`, entered from `nop; jmp` at
  0x6CA638). Jump tables 0x6CAAE0 (6 dwords) / byte table 0x6CAAF8 (0x75 bytes) live in the gap after the code.
- Locals: CMatrix `mat` at L+0x70 (ctor inlined: L+0xB0 m_pAttachMatrix = 0, L+0xB4 owns = 0 at 0x6C94C3; EH 0; dtor at 0x6CAAC0),
  CQuaternion `q` L+0xB8, temp CMatrix L+0xD4 (Hydra jets, EH 1..4), FxPrtMult_c L+0x11C, `colModel` L+0x10 (later reused as an int
  temp), `mi` L+0x48 (also ebp in the first half), axis CVector L+0x20, scale `s` L+0x18, offset `o` L+0x14, atomic out L+0x38,
  vector temps L+0x2C, L+0x3C, L+0x4C, L+0x58, L+0x64. ebx = 0 for most of the function (pushed as 0 / false).
- Callers: none direct (CPlane vtable 0x871948 slot 17). gta-reversed: `plugin::CallMethod<0x6C94A0>` only (Plane.cpp:367, VMT hook
  `.Reversed = false`); no C++.
- Callees: GetColModel 0x535300 (x2), CVehicle::PreRender 0x6D6480, virtual +0x114 (DoHoverSuspensionRatios, CAutomobile 0x6A45C0),
  virtual +0xD0 (DoBurstAndSoftGroundRatios, CAutomobile 0x6A47F0), CVehicleModelInfo::GetWheelPosn 0x4C7D20 (thiscall `(int wheel,
  CVector& out, bool local)`), GetOriginalCompPosition 0x4C7DD0 (thiscall `(CVector& out, int node)` -> bool in AL), `_CIasin` 0x821E70,
  CAutomobile::UpdateWheelMatrix 0x6AA290 (thiscall `(int node, int flags)`), CMatrix Attach/UpdateRW/Update/SetTranslateOnly 0x59AF80/
  operator*(CMatrix,CMatrix) 0x59BE30/operator*(CMatrix,CVector) 0x59C890 (cdecl, hidden out)/~CMatrix, CVector::Normalise, CQuaternion
  Set 0x59C600 / Get(RwMatrix*) 0x59C080 (writes right/up/at only), CVehicle::SetComponentRotation 0x6DBA30 (thiscall `(RwFrame*,
  int axis, float angle, bool resetPos)`, returns at once for a null frame), RwFrameForAllObjects 0x7F1200 with GetCurrentAtomicObjectCB
  0x6A0750, CVehicle::SetComponentAtomicAlpha 0x6D2960 (cdecl `(RpAtomic*, int)`), CAutomobile::DoHeliDustEffect 0x6B0690 (thiscall
  `(float, float)`), FxSystem_c Kill 0x4AA3F0 / Play 0x4AA2F0 / SetLocalParticles 0x4AA910 / CopyParentMatrix 0x4AA890 / SetMatrix
  0x4AA630 / SetOffsetPos 0x4AA660 / SetConstTime(bool, float) 0x4AA6C0 / AddParticle(CVector*, CVector*, float, FxPrtMult_c*, float,
  float, float, bool) 0x4AA440, FxManager_c::CreateFxSystem(name, CVector*, RwMatrix*, bool) 0x4A9BE0 on g_fxMan 0xA9AE80, FxPrtMult_c
  ctor 0x4AB290 (r, g, b, a, size, rot, life), CBouncingPanel::ProcessPanel 0x6F49A0, CShadows::StoreShadowForVehicle 0x70BDA0,
  CMotionBlurStreaks::RegisterStreak 0x721DC0 (cdecl `(u32 id, u8 r, u8 g, u8 b, u8 a, CVector left, CVector right)`), `_ftol` 0x821B40,
  `_CIpow` 0x822130, CVehicle::DoBoatSplashes 0x6DD130 (thiscall `(float)`).
- Globals: ms_modelInfoPtrs 0xA9B0C8, ms_fTimeStep 0xB7CB5C, g_fx+0x20 0xA9AE20 (smoke FxSystem_c*), CPlane::ANDROM_COL_ANGLE_MULT
  0x8D33C0 (0.00015f), HARRIER_NOZZLE_ROTATE_LIMIT 0x8D33C4 (i16 5000), HARRIER_NOZZLE_SWITCH_LIMIT 0x8D33C8 (i16 3000), unnamed .data
  f32: 0x8D33FC 0.03f, 0x8D3400 0.98f, 0x8D3404 1.3f, 0x8D3408 0.1f, 0x8D340C 0.1f, 0x8D3410 0.9f, 0x8D3414 0.3f, 0x8D3418 2.0f; string
  0x871A64 "jetthrust". Constants: 0x858624 1.0f, 0x858B50 0.0f, 0x858F34 0.75f, 0x858C1C -1.0f, 0x858EF0 0.95f, 0x858CBC 6.2831855f,
  0x871914 40.0f, 0x8595EC 0.017453292f, 0x871920 0.5f, 0x871924 0.3f, 0x87191C -30.0f, 0x871928 0.03f, 0x871918 25.0f, 0x87192C
  1.4835299f, 0x871930 -1.5707964f, 0x871934 -1.3962635f, 0x871938 / 0x87193C 2.268928f, 0x871940 1.3089969f, 0x859EF8 0.0 (f64),
  0x858FE4 1.5707964f, 0x858B8C 0.5f, 0x858C84 0.25f, 0x859AEC 6400.0f, 0x85950C 32.0f, 0x858B1C 0.1f, 0x85862C 10.0f, 0x858B3C 3.0f.
- Fields (CAutomobile layout in CHeli.md, CPlane fields in this file's header): +0x14 m_matrix (right +0, forward +0x10, up +0x20),
  +0x18 m_pRwObject, +0x22 model, +0x36 status, +0x40 physical flags, +0x44 move speed, +0x50 turn speed, +0x5C/+0x68 friction
  move/turn speed, +0x384 handling (+0xB8 suspension upper limit), +0x388 flying handling (+0x2C m_fAttackLift, +0x34 m_fGearDownL),
  +0x38C handling flags, +0x428/+0x42B vehicle flags, +0x580 m_pDustParticle, +0x648 m_aCarNodes[25], +0x6AC m_panels[3],
  +0x724 m_wheelColPoint[4] (normal at +0x10), +0x7D4 compression[4], +0x828 m_wheelRotation[4], +0x838 m_wheelPosition[4], +0x86C
  m_wMiscComponentAngle (u16), +0x878 spring[4], +0x888 line length[4], +0x8B4 m_moveForce, +0x8C0 m_turnForce, +0x950 (gr
  m_fDoomHorizontalRotation), +0x984 m_heliDustFxTimeConst, CPlane +0x988 m_fLeftRightSkid, +0x98C m_fSteeringUpDown, +0x990
  m_fSteeringLeftRight, +0x994 m_fAccelerationBreakStatus, +0x9C4 m_fPropSpeed, +0x9C8 field_9C8 (propeller angle), +0x9CC
  m_LandingGearAngle, +0x9E8 m_apJettrusParticles[4], +0xA00 m_bSmokeEjectorEnabled. Model info: +0x40 wheelSizeFront, +0x5C
  m_pVehicleStruct (dummies m_avDummyPos[i] at +0xC*i: [9] +0x6C, [10] +0x78, [11] +0x84, [12] +0x90).
- Plane nodes used (m_aCarNodes index = (offset - 0x648) / 4): +0x654 [3] WHEEL_RM, +0x660 [6] WHEEL_LM (Hydra nozzles), +0x678..+0x684
  [12..15] STATIC_PROP / MOVING_PROP / STATIC_PROP2 / MOVING_PROP2, +0x688 [16] RUDDER, +0x68C [17] ELEVATOR_L, +0x690 [18] ELEVATOR_R,
  +0x694 [19] AILERON_L, +0x698 [20] AILERON_R, +0x69C [21] GEAR_L, +0x6A0 [22] GEAR_R, +0x6A4 [23] MISC_A, +0x6A8 [24] MISC_B.

### A. 0x6C94C3-0x6C950D: setup
- `mat` ctor, `mi = ms_modelInfoPtrs[model]` (L+0x48), EH 0, L+0x38 = 0, `colModel = GetColModel()` (L+0x10; called BEFORE
  CVehicle::PreRender), `CVehicle::PreRender()`. If model == 0x21B (539 VORTEX): virtual +0x114 (DoHoverSuspensionRatios).

### B. 0x6C950D-0x6C9669: suspension (only if byte +0x42B bit 0)
- Virtual +0xD0 (DoBurstAndSoftGroundRatios), then for i = 0..3 ascending (edi = &wheelPos[i], ebx = &cp[i].normal.y):
  1. `r = 1.0f - spring[i]/line[i]`; `c = f32((comp[i] - r) / (1.0f - r))` (L+0x1C).
  2. `mi->GetWheelPosn(i, out (L+0x4C), true)`; `z = out.z + handling->+0xB8`; if `c > 0.0f`: `z = z - c*spring[i]`.
  3. If `z > wheelPos[i]` (fcom, `test ah,0x41; je`): `wheelPos[i] = z`; else if `(physFlags +0x40 & 0x04000000) && (+0x38C & 0x20000)`:
     `wheelPos[i] = z`; else `wheelPos[i] = (z - wheelPos[i]) * 0.75f + wheelPos[i]`. (Same as CHeli::PreRender step 4.)
  4. Model 0x250 (592 ANDROM) only: if `comp[i] < 1.0f`: `d = (f.z*n.z + n.x*f.x) + f.y*n.y` (forward of m_matrix, n = cp[i] normal);
     `wheelRot[i] (+0x828) = -asin(dd)` with `dd = -1.0f` if `-1.0f > d`, else `1.0f` if `1.0f < d`, else `f32(d)` (NaN -> asin(NaN));
     else (`comp >= 1` or NaN): `wheelRot[i] = wheelRot[i] * 0.95f`.

### C. 0x6C966B-0x6C96A0: wheel matrices
- `UpdateWheelMatrix(4, 1)`, `(7, 1)`, `(2, 1)`, `(5, (model == 0x208 /*520 HYDRA*/) ? 3 : 1)`.

### D. 0x6C96A5-0x6C96E3: propeller angle
- `a = ms_fTimeStep * m_fPropSpeed + field_9C8`; `field_9C8 = f32(a)` (fst); if `a > 6.2831855f`: reload the stored f32, then
  `do a -= 6.2831855f while (a > 6.2831855f)` in x87, store f32.

### E. 0x6C96E9-0x6C9C8F: control surfaces (only status 0..3: `(byte+0x36 & 0xF8) <= 0x18`, else go to F)
Common sub-step "axis from dummy k": `axis (L+0x20) = mi->m_pVehicleStruct->m_avDummyPos[k]`; if all three components == 0.0f (each
`test ah,0x44; jp` exact): set one component to 1.0f (given below) and skip normalising; else `axis = axis - mat.pos` (three f32
subtractions) and `axis.Normalise()`. "Attach" = `mat.Attach(&node->modelling, false); mat.UpdateRW()`. "Apply" =
`q.Set(&axis, angle); q.Get(mat.m_pAttachMatrix)` (writes the rotation straight into the frame's RwMatrix, position untouched).
1. Rudder (node 16 +0x688, if non-null): Attach; axis from dummy 11 (default z = 1.0f); angle = `f32((40.0f * 0.017453292f) *
   m_fLeftRightSkid)`; Apply. If model == 0x21B and node 24 (+0x6A8) non-null: Attach(node 24) and `q.Get(attach)` again (same q).
2. Ailerons (only if node 20 AILERON_R +0x698 non-null; else go to 3):
   - `s = 1.0f`, `o = 0.0f`; if `flyingHandling->+0x34 > 1.0f` and `fabs(m_LandingGearAngle) < 1.0f` and (node 21 +0x69C or node 22
     +0x6A0 non-null): `s = f32((1.0f - fabs(gear)) * 0.5f + 1.0f)`, `o = f32((1.0f - fabs(gear)) * 0.3f)`.
   - Right: Attach(node 20); axis from dummy 9 (default x = 1.0f); angle = `f32(((-30.0f * 0.017453292f) * m_fSteeringLeftRight) + o)`;
     Apply. If `s > 1.0f`: `mat.Update()`; if `axis.x == 1.0f`: `mat.forward.y *= s` (L+0x84); else if
     `mi->GetOriginalCompPosition(out (L+0x4C), 20)`: `mat.SetTranslateOnly(out.x, f32(((1.0f - s) * colModel->bbox.max.y (+0x10)) *
     0.03f + out.y), out.z)`; then `mat.UpdateRW()` (also when GetOriginalCompPosition fails).
   - Left (node 19 +0x694, if non-null): Attach; `axis.x = -axis.x` (fmul -1.0f; the right aileron's axis is reused); angle =
     `f32(((-30.0f * 0.017453292f) * m_fSteeringLeftRight) - o)`; Apply; if `s > 1.0f`: Update; if `axis.x == -1.0f`: forward.y *= s;
     else GetOriginalCompPosition(out, 19) -> same translate formula; UpdateRW.
3. Elevators (only if node 18 ELEVATOR_R +0x690 non-null): Attach; axis from dummy 10 (default x = 1.0f); angle `o' = f32(-((25.0f *
   0.017453292f) * m_fSteeringUpDown))` (stored in L+0x14); Apply. Node 17 (+0x68C, if non-null): Attach; angle `f32((25.0f *
   0.017453292f) * m_fSteeringUpDown)`; `axis.x = -axis.x` (written before Set is called); Apply.

### F. 0x6C9C94-0x6C9D79: propellers (always)
- Two passes: (static, moving, sign) = (node 12 +0x678, node 13 +0x67C, +1.0f) then (node 14 +0x680, node 15 +0x684, -1.0f).
  - Static node non-null: `SetComponentRotation(node, 1 /*Y*/, f32(t + t) with t = sign * field_9C8` (fadd st0,st0), true)`; `atomic = 0;
    RwFrameForAllObjects(node, GetCurrentAtomicObjectCB, &atomic)`; if atomic: `SetComponentAtomicAlpha(atomic, 0xFF)`.
  - Moving node non-null: `SetComponentRotation(node, 1, f32(-(sign * field_9C8)), true)`; same atomic lookup; alpha 0.

### G. 0x6C9D7E-0x6C9EDE: landing gear (`g = fabs(m_LandingGearAngle)`, each angle `f32(g * k)`)
- `switch (model - 476)` via byte table (cases outside 0..0x74 and unlisted models: skip to H):
  | model | axis | GEAR_L (+0x69C) k | GEAR_R (+0x6A0) k | MISC_A (+0x6A4, axis X) k | MISC_B (+0x6A8, axis X) k |
  | 476 RUSTLER | Y | -1.4835299 | 1.4835299 | - | - |
  | 519 SHAMAL, 577 AT400 | Y | -1.4835299 | 1.4835299 | 2.268928 | - |
  | 520 HYDRA | X | -1.5707964 | -1.5707964 | -1.3962635 | 2.268928 |
  | 553 NEVADA | X | 1.3089969 | 1.3089969 | - | - |
  | 592 ANDROM | X | 2.268928 | 2.268928 | -2.268928 | - |
  All calls `SetComponentRotation(node, axis, angle, true)`; MISC_A only when ebp > -1, MISC_B only when its k (L+0x1C) > 0.0 (f64
  compare). Gear L multiplies the k kept in ST0, the others read the f32 locals.

### H. 0x6C9EE3-0x6CA633: model specials
1. 592 ANDROM: `SetComponentRotation(node 24, 0 /*X*/, f32(float(u16 +0x86C) * ANDROM_COL_ANGLE_MULT), true)`; go to I.
2. 520 HYDRA:
   - `a = f32(float(u16 +0x86C) * 1.5707964f / (int)HARRIER_NOZZLE_ROTATE_LIMIT)` (fild, fmul, fidiv of the i16 sign-extended);
     `SetComponentRotation(node 6 (+0x660), 0, a, true)`, `SetComponentRotation(node 3 (+0x654), 0, a, true)`.
   - If !bEngineOn: Kill and null all four m_apJettrusParticles; if m_pDustParticle: Kill it, `+0x580 = 0`, `+0x984 = 0`; go to I.
   - Else: `t = f32((m_fAccelerationBreakStatus + 1.0f) * 0.5f)`. If `(int)u16 +0x86C >= (int)HARRIER_NOZZLE_SWITCH_LIMIT`:
     `DoHeliDustEffect(t, 2.0f)`; else if m_pDustParticle: Kill, `+0x580 = 0`, `+0x984 = 0`.
   - For each jet slot j = 0..3 (ascending): if `m_pRwObject != null` (the derived `&parentFrame->modelling` null test is effectively
     always true) and `jet[j] == null`: `jet[j] = g_fxMan.CreateFxSystem("jetthrust", &(0,0,0) (L+0x2C), &((RwFrame*)m_pRwObject->parent)
     ->modelling, false)`; if non-null: `Play(); SetLocalParticles(1); CopyParentMatrix()`.
   - For j with jet[j] non-null: `tmp = (*m_matrix) * frameMatrix` (the RwMatrix of node 6 for j = 0, 1 and of node 3 for j = 2, 3,
     read as a CMatrix); `jet[j]->SetMatrix(&tmp); jet[j]->SetOffsetPos(&off_j); jet[j]->SetConstTime(true, t)`; `~tmp`. Offsets:
     j0 (0.7, -0.45, 0.05), j1 (-0.82, -0.45, 0.05), j2 (0.63, -0.45, 0.07), j3 (-0.75, -0.45, 0.07). Go to I.
3. 539 VORTEX, only if node 23 (+0x6A4) non-null:
   - `cd = GetColModel()->m_pColData`; `mi->GetWheelPosn(0, out (L+0x2C), false)`; `mat.Attach(&node23->modelling, false)`.
   - `mat.up.y = clamp(((f.z*v.z + f.y*v.y) + f.x*v.x) * 2.0f, -0.3f, 0.3f)` and `mat.up.x = clamp(((v.z*r.z + v.y*r.y) + r.x*v.x) *
     2.0f, -0.3f, 0.3f)` (f/r = m_matrix forward/right, v = move speed; 0x8D3418 / 0x8D3414; the dot product is recomputed for each
     test, upper test first, `> 0.3` -> 0.3, then `-0.3 > x` -> -0.3; NaN stores NaN).
   - `k = pow(0.9f [0x8D3410], ms_fTimeStep)`.
   - `a = (((w2 - w3) + (w0 - w1)) * 0.5f) / (line0.start.y - line1.start.y)` (w = wheelPos +0x838.., lines from cd+0x10; x87 order:
     `w2-w3`, `w0-w1`, faddp); clamp: `> 0.1f` -> 0.1f [0x8D340C], else `< -0.1f` -> -0.1f; `mat.forward.z = mat.forward.z*k +
     f32(1.0f - k)*a`.
   - `b = (((w3 - w1) + (w2 - w0)) * 0.5f) / (line3.start.x - line1.start.x)` (line +0x60 / +0x20); clamp +-0.1f [0x8D3408];
     `mat.right.z = mat.right.z*k + (1-k)*b`.
   - `c = 1.0f - ((((w3 + w1) + w2) + w0) * 0.25f - out.z) / (mi->wheelSizeFront * 0.5f)`; if `c > 1.3f` -> 1.3f (no lower clamp);
     `mat.up.z = mat.up.z*k + (1-k)*c`. `mat.UpdateRW()`.

### I. 0x6CA638 -> 0x407EBB -> 0x6CA63E-0x6CA73A: panels, forces, shadow
- For the 3 panels (edi = +0x6AC, stride 0x20): if `(i16)panel.frameId > -1`: `panel.ProcessPanel(this, m_aCarNodes[frameId],
  m_moveForce (+0x8B4, by value), m_turnForce (+0x8C0, by value), 0.03f [0x8D33FC], 0.98f [0x8D3400])`.
- `m_moveForce = moveSpeed + frictionMoveSpeed` and `m_turnForce = turnSpeed + frictionTurnSpeed` (component-wise, f32).
- `StoreShadowForVehicle(this, model == 539 ? 0 : (model == 577 || model == 592) ? 5 : 3)`.

### J. 0x6CA73F-0x6CA934: wing-tip motion blur streaks
- Skip if bIsDrowning (+0x42B & 0x40) or model 539 or 464 (RCBARON).
- `alpha = (u8)_ftol(max(fabs(((v.z*u.z + v.y*u.y) + u.x*v.x) * flyingHandling->m_fAttackLift * 6400.0f) - 32.0f, 0.0f))` (u = m_matrix
  up; `0.0f > x` -> 0).
- `B = m_avDummyPos[12]` (L+0x58), `A = (B.x - 0.1f, B.y, B.z)` (L+0x3C). `RegisterStreak((u32)this, 0xFF, 0xFF, 0xFF, alpha,
  (*m_matrix) * B, (*m_matrix) * A)`. Then `B.x = -B.x; A.x = -A.x` and `RegisterStreak((u32)this + 1, 0xFF, 0xFF, 0xFF, alpha,
  (*m_matrix) * B, (*m_matrix) * A)`.

### K. 0x6CA937-0x6CAA8E: smoke ejector (if m_bSmokeEjectorEnabled)
- 512 CROPDUST: `mult = FxPrtMult_c(1, 1, 1, 0.4f, 1, 1, 0.2f)`; `pos = (*m_matrix) * (0, -0.5f, -0.5f)`; `vel = (vx*10.0f, vy*10.0f,
  vz*10.0f - 3.0f)`.
- 513 STUNT: `mult = FxPrtMult_c(1, 0, 0, 0.4f, 1, 1, 0.3f)`; `pos = (*m_matrix) * (0, -5.0f, 0)`; `vel = (vx*10, vy*10, vz*10)`.
- Other models: nothing. Both: `(*(FxSystem_c**)0xA9AE20)->AddParticle(&pos, &vel, 0.0f, &mult, -1.0f, 1.2f, 0.6f, false)`.

### L. 0x6CAA93-0x6CAADD: skimmer splashes, exit
- If model == 460 (SKIMMER) and physical flags bit 8 (bSubmergedInWater): `DoBoatSplashes(+0x950)`. EH -1, `~mat`, return.

- Oddities to keep: GetColModel before CVehicle::PreRender; rudder/aileron/elevator rotations written only into the RwMatrix (CMatrix
  mat is stale unless Update is called); left aileron reuses (negated) the right aileron's axis and is skipped when the right one is
  missing; the aileron "flap" condition reads flying handling +0x34 (gr name m_fGearDownL); the Hydra jet matrices multiply by a
  RwMatrix read as CMatrix; Vortex nozzle matrix is written without re-orthonormalising; the c-clamp has no lower bound.
- Open questions: none affecting behaviour. Names: +0x9C8 (propeller angle) is gr `field_9C8`; flying handling +0x34 semantics.

## 0x6CADB0 CPlane::vfunc_0 (proposed name: CPlane scalar deleting destructor)  (catalogue: specified/REA, port level 17)
- Convention: vtable slot 0, `__thiscall CPlane* (uint8 flags [esp+4])`, `ret 4`, EAX = this. Callers (virtual): CCarCtrl::CreatePoliceChase,
  CreateConvoy, GenerateOneRandomCar, CSetPiece::Update.
- Behaviour: `CPlane::~CPlane(0x6C9160)(this)`; if `flags & 1`: `CVehicle::operator delete(this)` (0x6E2D90, cdecl, pool free); return this.
- Name: the catalogue name `vfunc_0` is the compiler's scalar deleting destructor (vtable slot 0, test of bit 0 then delete).
- gta-reversed: not hooked (the C++ `delete` expression produces it).

## 0x6CADD0 CPlane::ProcessControlInputs  (catalogue: gr-unreversed/REA, port level 10)
- Convention: virtual (vtable +0x60) `__thiscall void (uint8 playerNum [esp+4])`, `ret 4`. Extent includes the chunk 0x406AF1
  (`cmp word [esi+0x22],0x208; jmp 0x6CB4F9`). Caller: CAutomobile::ProcessAI 0x6B4800.
- Callees: CPad::GetPad (always `GetPad(playerNum)`), GetSteeringLeftRight/UpDown, GetLookLeft/Right, GetHandBrake 0x540040, GetBrake,
  GetAccelerate, GetCarGunUpDown 0x53FC10, `_CIpow` 0x822130, `_ftol` 0x821B40, CPlane::SetGearUp 0x6CAC20, CWorld::FindPlayerPed,
  CPlayerPed::KeepAreaAroundPlayerClear.
- Globals: m_bUseMouse3rdPerson 0xB6EC2E, CVehicle::m_bEnableMouseFlying 0xC1CC03, m_nLastControlInput 0xC1CC04 (RW), mouse X/Y,
  ms_fTimeStep, HARRIER_NOZZLE_ROTATE_LIMIT 0x8D33C4 (i16), HARRIER_NOZZLE_ROTATERATE 0x8D33DC (f32 25.0). Constants: 0x858B50 0.0f,
  0x858B88 1/128, 0x858CC4 0.2f, 0x871908 0.0015f, 0x87190C 0.15f, 0x871910 0.99f, 0x858C1C -1.0f, 0x858624 1.0f, 0x858B1C 0.1f,
  0x8595EC pi/180, 0x858CE8 1.5f, 0x859A3C 1/255, 0x871904 0.02f, 0x871900 1000.0f, 0x858B8C 0.5f, 0x85862C 10.0f, 0x858F9C 0.35f,
  0x871A70 1/510 (0.0019607844f), 0x871254 0.28f.
- Fields: +0x988 skid, +0x98C up/down, +0x990 left/right, +0x994 accel, +0x9CC m_LandingGearAngle, +0x7D4..+0x7E0 suspension compression,
  +0x5A5..+0x5A8 wheel status bytes (CDamageManager +5..+8), +0x86C/+0x86E m_wMiscComponentAngle / Prev, +0x98 m_fAirResistance,
  flying handling +0x30 m_fGearUpR, handling +0x10 m_fDragMult, +0xA0 m_fSteeringLock.
- Behaviour:
  1. `fs = (ms.z*fwd.z + ms.y*fwd.y) + ms.x*fwd.x`.
  2. LR/UD steering: pad path P: `LR = LR + ((float)GetSteeringLeftRight() * (1/128) - LR) * ts * 0.2f`, `UD = UD + ((float)GetSteeringUpDown()
     * (1/128) - UD) * ts * 0.2f` (not negated). With mouse flying (`m_bUseMouse3rdPerson && m_bEnableMouseFlying`): mouse path M if
     `mouseX != 0 || mouseY != 0`, or if (`|LR| > 0 || |UD| > 0`) and last input is 2 and both pad axes are 0; else if both pad axes
     are 0 and last input is 2 keep values; else `m_nLastControlInput = 0` + P. Without mouse flying: `m_nLastControlInput = 0` + P.
     M: `m_nLastControlInput = 2`; if `m_bVehicleMouseLook == 0`: `LR = mouseX * 0.0015f + LR`, `UD = mouseY * 0.0015f + UD`; then
     `if (|LR| < 0.15f) LR *= pow(0.99f, ts)`; `if (|UD| < 0.15f) UD *= pow(0.99f, ts)`.
  3. Clamp LR then UD: `(-1, 1)` kept; `>= 1 -> 1.0f`; `<= -1` or NaN -> `-1.0f`.
  4. Skid: model 539 (Vortex): `skid = skid + ((float)GetSteeringLeftRight() * (1/128) - skid) * ts * 0.2f`; else
     `target = GetLookLeft() ? -1.0f : (float)GetLookRight()` and `skid = skid + (target - skid) * ts * 0.2f`; clamp as in 3.
  5. `f = 1.0f - fs; if (0.1f > f) f = 0.1f`; `m_fSteerAngle = -(((lock * pi/180) * skid) * f)`; Vortex with `GetHandBrake()`:
     `m_fSteerAngle *= 1.5f; skid *= 1.5f`.
  6. `accel = (float)(GetAccelerate() - GetBrake()) * (1/255)`.
  7. Landing gear: toggle requested when `NewState.ShockButtonR != 0 && OldState.ShockButtonR == 0` and all four compression values are
     exactly 1.0f (raw compare) and `m_fGearUpR < 1.0f` and model != 464:
     `gear == 0.0f` -> the four wheel status bytes (+0x5A5..+0x5A8 = m_damageManager wheel status[0..3]) = 2 and `gear = ts*0.02f + gear`; else `gear == 1.0f` (raw) -> `gear = ts*0.02f - 1.0f`;
     else `gear = gear * -1.0f`.
     No toggle: if `gear < 0`: `gear = ts*0.02f + gear`; when it is no longer `< 0`: `gear = 0`, wheel status bytes = 0,
     `m_fAirResistance = handling->m_fDragMult / 1000.0f * 0.5f`. Else if `0 < gear < 1`: `gear = ts*0.02f + gear`; when no longer `< 1`:
     `SetGearUp()`.
  8. Hydra (model 520): if `|(float)GetCarGunUpDown()| > 10.0f`: `prev = angle`; `d = (int16)_ftol((float)GetCarGunUpDown() * ts * 25.0f *
     (1/128))`; `n = d + (uint16)angle`; if `n < 0` -> 0 else n is recomputed with a second pad read and `_ftol`; stored as u16; if
     `(uint16)n > (int)LIMIT` -> `angle = LIMIT`.
  9. `bIsHandbrakeOn = 0; gas = 0`. If `fs > 0.0f && GetBrake() != 0`: `brake = GetBrake() * ((fs > 0.35f) ? 1/510 : 1/255)`. Else:
     `brake = (|ms| < 0.1f && GetBrake() < 10 && GetAccelerate() < 10) ? 0.5f : 0.0f`.
  10. If `GetPad(playerNum)->DisablePlayerControls`: `brake = 1.0f`, handbrake on, `gas = 0`, `KeepAreaAroundPlayerClear()`,
      `|ms|` clamped to 0.28f (as CHeli::ProcessControlInputs step 7).
- gta-reversed: plugin::Call (no C++).

## 0x6CB7C0 CPlane::ProcessFlyingCarStuff  (catalogue: gr-unreversed/REA, port level 3)
- Convention: virtual (vtable +0x110) `__thiscall void ()`, plain `ret`. Caller: CAutomobile::ProcessControl 0x6B1880 (2 sites).
- Callees: `rand` 0x821B1E (16 sites), `_ftol` 0x821B40, CDamageManager::SetAeroplaneCompStatus 0x6C22D0 (thiscall(int comp, int status))
  / GetAeroplaneCompStatus 0x6C2300 (thiscall(int comp) -> int), CPlaceable::GetRoll 0x420B30 (ST0), CMatrix operator*(CVector)
  0x59C890 (cdecl(out, const CMatrix&, const CVector&)), FxPrtMult_c::FxPrtMult_c 0x4AB290 (thiscall(r,g,b,a,size,rot,life)),
  FxSystem_c::AddParticle(CVector*...) 0x4AA440, CVehicle::HeightAboveCeiling 0x6D2600 (thiscall(float z, int32 flightModel) -> ST0),
  CVehicle::FlyingControl 0x6D85F0 (thiscall(eFlightModel, float skid, float upDown, float leftRight, float accel)),
  cHandlingDataMgr::GetFlyingPointer 0x6F52D0 (thiscall on gHandlingDataMgr 0xC2B9C8, (uint8 id) -> tFlyingHandlingData*).
- Globals: CTimer::ms_fTimeStep 0xB7CB5C; PLANE_DAMAGE_WAVE_COUNTER_VAR 0x8D33E0 (f32 0.75); tuning floats (all `.data`, writable):
  0x8D341C = 0.25, 0x8D3420 = 10.0, 0x8D3424 = 0.002, 0x8D3428 = 0.05, 0x8D342C = 10.0, 0x8D3430 = 0.002, 0x8D3434 = 0.05,
  0x8D3438 = 10.0, 0x8D343C = 0.002, 0x8D3440 = 0.05, 0x8D3444 = 0.2, 0x8D3448 = int32 2500, 0x8D344C = 0.8;
  CPlane::HARRIER_NOZZLE_SWITCH_LIMIT 0x8D33C8 (int16 3000), PLANE_MIN/STD/MAX/ROC_PROP_SPEED 0x8D33CC/0x8D33D0/0x8D33D4/0x8D33D8
  (0.05/0.18/0.34/0.01); TheCamera matrix ptr 0xB6F03C / placement pos 0xB6F02C; g_fx smoke system ptr 0xA9AE20;
  CModelInfo::ms_modelInfoPtrs[425] (dword 0xA9B76C, MODEL_HUNTER).
  Constants: 0x858B50 0.0f, 0x85A310 1.0 (f64), 0x858624 1.0f, 0x858C7C 3.0518509e-05f (1/32767), 0x858B38 0.02f, 0x858C4C 1000.0f,
  0x858C54 4294967296.0f (unsigned fix-up), 0x858B3C 3.0f, 0x859F80 250.0f, 0x858B8C 0.5f, 0x86DAF4 2.3561945f, 0x858F34 0.75f,
  0x858FE4 1.5707964f, 0x858CBC 6.2831855f, 0x858CC4 0.2f, 0x858FCC 0.15f, 0x858C84 0.25f, 0x858C24 0.3f, 0x859AEC 6400.0f,
  0x858B40 50.0f, 0x871A74 0.20000005f, 0x858C20 0.9f, 0x858CDC 0.001f, 0x859CD8 0.003f, 0x858CEC 0.04f, 0x858C1C -1.0f,
  0x859068 0.05 (f64), 0x859EF8 0.0 (f64).
- Fields: CAutomobile +0x5A0 m_damageManager, +0x648 m_aCarNodes[25], +0x6AC m_panels[3] (CBouncingPanel stride 0x20: +0 u16 frameId,
  +0x08 rotation vec (y at panel+0x0C = this+0x6B8+0x20i), +0x14 pos vec (y at this+0x6C4+0x20i)), +0x86C m_wMiscComponentAngle (u16),
  +0x960 m_nNumContactWheels. CPlane +0x988 m_fLeftRightSkid, +0x98C m_fSteeringUpDown, +0x990 m_fSteeringLeftRight,
  +0x994 m_fAccelerationBreakStatus, +0x9C4 m_fPropSpeed, +0x9D0 m_planeDamageWave (int32). Vehicle: +0x36 status, +0x40
  physical flags (0x08000000 bTouchingWater), +0xB8 m_nFakePhysics (u8), +0x388 m_pFlyingHandlingData, +0x428 (0x10 bEngineOn),
  +0x42B (0x40 bIsDrowning), +0x4C0 m_fHealth.
- Behaviour (locals: `cSkid`, `cUD`, `cLR` = copies of +0x988/+0x98C/+0x990 taken at 0x6CB8D0-0x6CB8F7; `SF` speed factor):
  1. `if (ms_fTimeStep <= 0.0f) return` (ordered; NaN continues; 0x6CB7C0-0x6CB7DA).
  2. Damage wave (0x6CB7E0-0x6CB85B): `hi = f32((double)WAVE_VAR + 1.0)`, `lo = 1.0f - WAVE_VAR`; `r = (float)rand() * (1/32767)`;
     `v = (hi - lo) * r + lo`; `n = _ftol(ms_fTimeStep * 0.02f * 1000.0f)`; `m_planeDamageWave += _ftol(v * (float)(uint32)n)`.
  3. `SF = |moveSpeed| * 3.0f` (sqrt of `(x^2 + y^2) + z^2`); `if (SF > 1.0f) SF = 1.0f` (the else path recomputes the same value).
  4. If `m_fHealth < 250.0f && status == PLAYER` (0x6CB8CA-0x6CB901): SetAeroplaneCompStatus(12, 2), (13, 2); `cSkid += 0.5f`;
     `if (|GetRoll()| < 2.3561945f) cLR += 0.75f`; `if (|GetRoll()| > 1.5707964f) cUD += 0.5f` (GetRoll called twice).
  5. If model != 464 (0x1D0, RC Baron): for `comp = 12 .. 24` (0x6CB990-0x6CC0E1):
     - `st = GetAeroplaneCompStatus(comp)`; skip unless `m_aCarNodes[comp] && st > 0` (signed).
     - `pi` = first index 0..3 with `(int16)m_panels[pi].frameId == comp`, else -1. The scan reads 4 entries although the array has 3
       (entry 3 overlaps `m_swingingChassis` at +0x70C) - keep the 4-entry scan.
     - `switch (comp - 12)` (table 0x6CC48C; comps 21..24 skip). `R(h) = (h - (-h)) * rand01 + (-h)` (fresh `rand()` each time).
       * 12..15 (props, 0x6CB9F1): if `accel(+0x994) > 0.0f`: `A = (0.8+1.0 - (1.0-0.8)) * r + (1.0-0.8)` using 0x8D344C;
         `accel += A * (sin((float)(uint32)wave * 6.2831855f / (float)(uint32)2500) - 1.0f) * st * st * ms_fTimeStep * 0.2f[0x8D3444]`
         (exact op order: `((A * (sin-1)) * st) * st`, then *ts, *0.2, + accel). `smoke = (float)st * 0.5f`.
       * 16 rudder (0x6CBABA): `skid(+0x988) *= (1.0f - st*0.2f)`; `skid += R(0.05[0x8D3440]) * (float)(st*st) * ts * SF`; `cSkid = skid`.
         If `pi >= 0`: `panel[pi].pos.y += R(0.002[0x8D343C]) * st^2 * ts * SF`; `cSkid -= 10.0f[0x8D3438] * panel[pi].rot.y`.
         `smoke = st * 0.2f`.
       * 17, 18 elevators (0x6CBBD7): same with `+0x98C`, 0x8D3434/0x8D3430/0x8D342C, result into `cUD` (`cUD -= 10*rot.y`);
         `smoke = st * 0.15f`.
       * 19, 20 ailerons (0x6CBCFC): same with `+0x990`, 0x8D3428/0x8D3424/0x8D3420, result into `cLR`; panel term:
         `cLR += 10*rot.y` for comp 19, `cLR -= 10*rot.y` for comp 20 (the `cmp ebx,0x13` test; in the rudder/elevator copies the
         `== 19` arm is dead). `smoke = st * 0.25f`.
       (`R` = `((hi - lo) * r) + lo` with `lo = -hi`; the panel R uses its own `rand()`.)
     - Smoke particle (0x6CBE0B): skip if `smoke <= 0.0f` or bIsDrowning; if `!(SF > 0.3f)` skip unless `(rand() & 7) == 0`;
       `d = vehPos - camPos` (cam = TheCamera matrix pos or placement pos), skip if `!((d.z^2 + d.y^2) + d.x^2 < 6400.0f) && status != PLAYER`;
       skip if node null. `p = (*m_matrix) * node->modelling.pos` (frame +0x40); `mult = FxPrtMult_c(0,0,0, 0.2f, 1.0f, 1.0f, smoke)`;
       `vel = (ms.x*0.25f*50.0f, ms.y*0.25f*50.0f, ms.z*0.25f*50.0f)`; `vel.x *= r*0.20000005f + 0.9f`, then vel.y, vel.z (one rand each);
       then `mult.m_fLife = rand01` (overwrites the ctor's `smoke` life, 0x6CC003); `p.x += R(smoke)`, `p.y += R(smoke)`, `p.z += R(smoke)`;
       `g_fx.m_fxSysSmoke->AddParticle(&p, &vel, 0.0f, &mult, -1.0f, 1.2f, 0.6f, 0)`.
  6. Engine/prop (0x6CC0EA), status `s`:
     - If `s` not in {0 PLAYER, 3 PHYSICS, 8 REMOTE_CONTROLLED}: `thr = ts * 0.001f`; if `prop > thr`: `prop -= (prop > 0.18f[STD]) ?
       ts*0.003f : thr`; `m_nFakePhysics = 0`; return. Else `prop = 0; bEngineOn = 0`; return.
     - `target = STD`; if `accel > 0` `target = (MAX - STD) * accel + STD`; else if `accel < 0` `target = (STD - MIN) * accel + STD`.
     - If `s` is PLAYER or REMOTE_CONTROLLED: `fm = (model == 464) ? 1 : 3`; `z` = matrix pos z or placement z;
       if `HeightAboveCeiling(z, fm) > 0`: `target *= max(HeightAboveCeiling(z, fm) * 0.02f, 0.0f)` (written `if (0 > h) h = 0`);
       `accel = accel - HeightAboveCeiling(z, fm) * 0.04f; if (-1.0f > accel) accel = -1.0f` (stored).
     - `prop += (target - prop) * ts * ROC` (0x6CC2CE). If bIsDrowning: `prop = 0, accel = 0, bEngineOn = 0`.
  7. Flight (0x6CC318):
     - model 464: if `!drowning && bEngineOn`: `FlyingControl(1, cSkid, cUD, cLR, accel)`. Return.
     - else: if `!bEngineOn` return; if `!(prop > MIN)` and `!((ms.x^2+ms.y^2)+ms.z^2 > 0.05)` return.
       If model 520 (0x208, Hydra) && status PLAYER && `(int)(uint16)m_wMiscComponentAngle >= (int)NOZZLE_SWITCH_LIMIT` (signed):
       save `m_pFlyingHandlingData`; set it to `GetFlyingPointer((uint8)ms_modelInfoPtrs[425]->m_nHandlingId)` (low byte of +0x4A);
       if `accel > 0.0` or (`m_nNumContactWheels < 4` and not bTouchingWater): `FlyingControl(6, cSkid, cUD, -cUD, 0.25f[0x8D341C] * accel)`
       (the 4th argument really is `-cUD`, 0x6CC43F); restore the pointer; return.
       Else `FlyingControl(3, cSkid, cUD, cLR, accel)`.
- Edge cases: `cSkid/cUD/cLR` adjustments of step 4 are discarded when step 5 rewrites them from the damaged component.
  The model index for the hover handling is Hunter (425), read via its model-info pointer, not the plane's own.
- gta-reversed: plugin::Call (no C++).

## 0x6CC4B0 CPlane::VehicleDamage  (catalogue: gr-unreversed/REA, port level 19)
- Convention: virtual (vtable +0xE0) `__thiscall void (float damageIntensity [+4], int component [+8], CEntity* damager [+0xC],
  CVector* coors [+0x10], CVector* dir [+0x14], eWeaponType weapon [+0x18])`, `ret 0x18`. Several argument slots are reused as temps
  (intensity, damager -> d2, coors -> collision-position pointer, dir -> bool / old health). Caller: CAutomobile::ProcessControl 0x6B1880.
- Callees: CAutomobile::VehicleDamage 0x6A7650, CWorld::FindPlayerPed/FindPlayerVehicle, CVehicle::CanVehicleBeDamaged 0x6D1280
  (thiscall(CEntity*, eWeaponType, bool*) -> AL), VectorSub 0x40FE60 (cdecl(out, a, b)), 0x59C810 (transposed 3x3 multiply),
  CVector::MagnitudeSqr 0x406DA0 (thiscall -> ST0), `rand`, CDamageManager::ProgressAeroplaneDamage 0x6C2460 (thiscall(int) -> AL) /
  GetAeroplaneCompStatus 0x6C2300, CBouncingPanel::SetPanel 0x6F4920 (thiscall(int16 frame, int16 axis, float limit)),
  CVehicle::SetComponentVisibility 0x6D2700 (thiscall(RwFrame*, uint32)), CAutomobile::dmgDrawCarCollidingParticles 0x6A6DC0
  (thiscall(const CVector&, float, eWeaponType)), FxSystem_c::Kill/Play/SetVelAdd/SetLocalParticles, CEntity::GetModellingMatrix 0x46A2D0,
  FxManager_c::CreateFxSystem(const char*, CVector*, RwMatrix*, bool) 0x4A9BE0 on g_fxMan 0xA9AE80, CGeneral::GetRandomNumberInRange(int, int)
  0x407180 (original helper), virtual BlowUpCar (vtable +0xA4).
- Globals: PLANE_DAMAGE_THRESHHOLD 0x8D33E4 (500.0f), PLANE_DAMAGE_SCALE_MASS 0x8D33E8 (10000.0f), PLANE_DAMAGE_DESTROY_THRESHHOLD
  0x8D33EC (5000.0f); string 0x85AA80 = "fire_med". Constants: 0x858B50 0.0f, 0x858624 1.0f, 0x858C4C 1000.0f, 0x858C80 5.0f and the
  health factors 0.5f / 0.16666667f / 0.083333336f / 0.6666667f / 0.25f (as CBike::VehicleDamage); immediates 0.333f (0x3EAA7EFA),
  1000.0f, -0.02f (0xBCA3D70A).
- Behaviour:
  1. Model 539 (Vortex): `CAutomobile::VehicleDamage(all six args)`; return.
  2. Return if `!bCanBeDamaged`. If status != PLAYER and physical flag 0x400000 (bInvulnerable): return unless `m_pDamageEntity` is
     `FindPlayerPed(-1)` or `FindPlayerVehicle(-1, false)`.
  3. Locals: `destroyT = DESTROY_THRESHHOLD`, `thr = THRESHHOLD`, `pk = 0.333f`, `isWeapon = false`.
     - `damageIntensity == 0.0f` (collision; 0x6CC554): `m = m_fMass / SCALE_MASS`; `I = m_fDamageIntensity`; `pk = 1.0f`;
       `thr = max(m, 1.0f) * thr` (`if (1.0f > m)` uses 1.0f); `destroyT = m * destroyT`; `collPos = &m_vecLastCollisionPosn`;
       return if `I <= 0.0f`, if bCollisionProof, or if `m_pDamageEntity` is a ped.
     - else (weapon; 0x6CC5F7): `b = false; if (!CanVehicleBeDamaged(damager, weapon, &b)) return`; return unless `weapon` is
       19, 20, 21 or 51; `isWeapon = true`; `I = 0`; `collPos = coors` (the argument pointer).
  4. Health: `d2 = I * handling->m_fCollisionDamageMultiplier`; `old = m_fHealth`; `m_fHealth -= k(d2)` with the same factor selection
     as CBike::VehicleDamage step 6b; `if (m_fHealth <= 0.0f && old > 0.0f) m_fHealth = 1.0f`.
  5. If `I > destroyT` -> `BlowUpCar(this, false)` (the plane passes ITSELF as damager) and return.
     If `m_fHealth <= 1.0f && old > 0.0f` -> `BlowUpCar(this, false)`; return.
  6. Component damage only if (`I > thr` or isWeapon) and status != WRECKED and `m_fMass > 1000.0f`:
     - `L = transpose(rot(m_matrix)) * (*collPos - pos)`; nearest node `n` in 12..24 (non-null `m_aCarNodes[n]`, distance of
       `node->modelling.pos - L`, strict `<` against the running best which starts at 1000.0f); none -> return.
     - If `n == 16` and `rand() & 1`: if `m_aCarNodes[17]` and `rand() & 1` -> `n = 17`; else if `m_aCarNodes[18]` -> `n = 18`.
     - If `m_damageManager.ProgressAeroplaneDamage(n)`: if `GetAeroplaneCompStatus(n) == 1 && 16 <= n <= 22`: scan panels 0..3 (4 entries,
       one past the array): stop at the first panel whose frameId == n; at the first free panel (frameId == 0xFFFF) call
       `SetPanel(n, 0, -0.02f)`. Else if `GetAeroplaneCompStatus(n) == 2`: `SetComponentVisibility(m_aCarNodes[n], 2)`.
     - `dmgDrawCarCollidingParticles(*collPos, pk * I, weapon)` (I is 0 for weapons).
     - Smoke: `p = m_aCarNodes[n]->modelling.pos`; if `m_pSmokeParticle` -> Kill, null; `M = GetModellingMatrix()`; if null return;
       `m_pSmokeParticle = g_fxMan.CreateFxSystem("fire_med", &p, M, false)`; if null return; `Play()`;
       `SetVelAdd(&(-ms.x*5.0f, -ms.y*5.0f, -ms.z*5.0f))`; `SetLocalParticles(true)`; `m_nSmokeTimer = GetRandomNumberInRange(2000, 4000)`.
- gta-reversed: plugin::Call (no C++).

## 0x6CCC50 CPlane::SwitchAmbientPlanes  (catalogue: gr-unreversed/REA, port level 3)
- Convention: `__cdecl void (bool enable [esp+4])` (static), plain `ret`. Callers: CMissionCleanup::Process 0x468560,
  CRunningScript::ProcessCommands2300To2399 0x4762D0.
- Callees: CWorld::Remove 0x563280 (cdecl(CEntity*)); virtual slot 0 of the vehicle (scalar deleting destructor, `this`, arg 1).
- Globals: CPlane::GenPlane_Active 0x8D33BC (bool, R/W), CPools::ms_pVehiclePool 0xB74494 (CPool: +0 objects, +4 byte map, +8 size;
  stride 0xA18).
- Behaviour:
  1. If `GenPlane_Active == 0`: `GenPlane_Active = enable`; return (0x6CCCD5).
  2. If `enable != 0`: `GenPlane_Active = enable` (1); return.
  3. Else (active and disabling): for slot `i = size-1 .. 0` (descending): skip free slots (byte map bit 7) and a null object address;
     if `v->m_nVehicleSubType` (+0x594) is 3 (HELI) or 4 (PLANE) and `v->m_nCreatedBy` (+0x4A4, byte) == 1 (RANDOM_VEHICLE):
     `CWorld::Remove(v); delete v` (vtable[0](1)). The pool header (objects base and byte map) is re-read every iteration.
     Finally `GenPlane_Active = enable` (= 0; the byte is re-read from the argument after each delete).
- Edge cases: heli/plane sub-types are tested on +0x594 (sub-type), so e.g. the Vortex / special vehicles follow their sub-type.
- gta-reversed: plugin::Call (no C++).

## 0x6CCCF0 CPlane::BlowUpCar  (catalogue: gr-unreversed/REA, port level 17)
- Convention: virtual (vtable +0xA4) `__thiscall void (CEntity* damager [esp+4], bool hideExplosion [esp+8])`, `ret 8`; `hideExplosion` unused.
- Callees/globals: as CHeli::BlowUpCar 0x6C6D30 (same helpers, same Players/havoc/stat/fire/explosion globals).
- Behaviour (differences from CHeli::BlowUpCar are marked *):
  1. If `!bCanBeDamaged` return.
  2. If status != PLAYER && `m_nCarMission` (+0x3BA) != 57* (MISSION_PLANE_CRASH_AND_BURN) && model != 464* (RC Baron):
     `m_nCarMission = 57; m_fHealth = 0`; return.
  3. Player-damager bookkeeping identical (havoc += 20, chase value += 10.0f, IncrementStat(125, rand()%6000 + 4000)). *No news-heli flag.
  4. If status == PLAYER*: for the driver (+0x460) and the 8 passenger slots (+0x464..+0x480) in that order, if non-null:
     `ped dword +0x474 |= 2` (CPed flag word 3, bit 1); then `m_nFlags &= ~0x81`; move and turn speed zeroed (z, y, x).
  5. status := WRECKED; physical flags |= 0x20000000; `m_nTimeWhenBlowedUp = now`; `SetClumpForAllAtomicsFlag(clump, 0x4000)`;
     `m_damageManager.FuckCarCompletely(false)`.
  6. If model != 464: bumpers 5, 6 and doors 0..5 damaged (`false`), `SpawnFlyingComponent(5, 1)`; if `m_aCarNodes[5]` != null* (null
     check absent in the heli version): hide its current atomic (`flags = 0`).
  7. Same resets as the heli (bomb, health, bomb timer, CamShake 0.4f, KillPedsInVehicle, lights override, engine/lights, siren, taxi
     light, ambulance/fire-truck counters) plus `m_bSmokeEjectorEnabled` (+0xA00) = 0*; `ChangeLawEnforcerState(false)`.
  8. `StartFire(this, damager, 0.8f, 1, 7000, 0)`; `RegisterCarBlownUpByPlayer(this, 0)`; `AddExplosion(this, damager, model == 464 ? 12 : 7,
     pos, 0, 1, -1.0f, 0)`.
- gta-reversed: plugin::Call (no C++).

## 0x6CD090 CPlane::FindPlaneCreationCoors  (catalogue: gr-unreversed/REA, port level 6)
- Convention: `__cdecl void (CVector* outCoors [esp+4], CVector* outTarget [esp+8], float* outOrientation [esp+0xC],
  float* outFlightHeight [esp+0x10], bool isBigPlane [esp+0x14])`, plain `ret`. Caller: CPlane::DoPlaneGenerationAndRemoval (2 sites).
- Callees: `rand` 0x821B1E (x3/iteration), CWorld::FindPlayerCoors 0x56E010 (cdecl(CVector& out, int32 player = -1) -> CVector*),
  CGeneral::GetATanOfXY 0x53CC70 (cdecl(float x, float y) -> ST0), CWorld::ClearScanCodes 0x563470, CColSphere::Set 0x40FD10,
  CWorld::GetIsLineOfSightClear 0x56A490 (cdecl(const CVector&, const CVector&, bool buildings, vehicles, peds, objects, dummies,
  seeThrough, ignoreSomeObjects) -> AL), `_ftol` 0x821B40, CCollision::CheckCameraCollisionBuildings 0x41A820
  (cdecl(int32 X, int32 Y, const CColBox&, const CColSphere&, const CColSphere&, const CColSphere&) -> AL).
- Globals: TheCamera.m_mCameraMatrix.forward 0xB6F9AC/0xB6F9B0/0xB6F9B4 (f32 x,y,z), CWorld::ms_nCurrentScanCode 0xB7CD78 (u16, RW).
  Constants: 0x8595EC = 0.017453292f (pi/180), 0x858BA4 = 20.0f; immediates 340.0f/200.0f (big) or 140.0f/25.0f, 15.0f.
- Behaviour:
  1. `n = 0` (int); `R, H = isBigPlane ? (340.0f, 200.0f) : (140.0f, 25.0f)`.
  2. Loop while `n < 500` (signed, 0x6CD0F0):
     a. `a = (float)(rand() % 360) * (pi/180)` (signed `idiv`), stored f32.
     b. `*outCoors = FindPlayerCoors(-1)`; `*outFlightHeight = ((float)(rand() & 0xF) + (float)n) + H` (fild, fiadd n, fadd H).
        `outCoors.x += cos(a) * R`; `outCoors.y += sin(a) * R` (x87 `fcos`/`fsin` of the f32 angle); `outCoors.z += *outFlightHeight`.
     c. `*outTarget = FindPlayerCoors(-1)`; `s = (float)(rand() & 0x1F) + 20.0f`;
        `outTarget.x += fwd.x*s; outTarget.y += fwd.y*s; outTarget.z = (outTarget.z + fwd.z*s) + *outFlightHeight` (fwd = camera
        matrix forward; products for y and z are stored f32 before the add, x is not).
     d. `*outOrientation = GetATanOfXY(outTarget.x - outCoors.x, outTarget.y - outCoors.y)`.
     e. Scan code: `if (ms_nCurrentScanCode == 0xFFFF) { ClearScanCodes(); ms_nCurrentScanCode = 1; } else ms_nCurrentScanCode++`.
        `E = outTarget + (outTarget - outCoors)` (component-wise: `(t - c) + t`).
     f. `sphere.Set(15.0f, *outCoors, 0, 0, 0xFF)` (a local CColSphere).
     g. If `GetIsLineOfSightClear(*outCoors, E, true, false, false, false, false, false, false)` AND
        `!CheckCameraCollisionBuildings(_ftol(outCoors.x), _ftol(outCoors.y), box, sphere, sphere, sphere)` -> leave the loop
        (success). Otherwise `n += 10` and retry.
  3. After the loop (success or `n >= 500`): `*outFlightHeight += 20.0f; outTarget.z += 20.0f`.
- Edge cases (keep): the X/Y passed to CheckCameraCollisionBuildings are truncated WORLD coordinates, not repeat-sector indices;
  the CColBox argument is an uninitialised stack local (never written in this function); the same sphere is passed three times.
  When all 50 attempts fail, the last candidate is returned. `_ftol` of y is computed before x (argument order).
- gta-reversed: plugin::Call (no C++).

## 0x6CD2F0 CPlane::DoPlaneGenerationAndRemoval  (catalogue: gr-unreversed/REA, port level 17)
- Convention: `__cdecl void ()` (static), plain `ret`; SEH frame (CEventDanger local). Caller: CCarCtrl::GenerateRandomCars 0x4341C0.
- Callees (all from the bytes): CPlane::CountPlanesAndHelis 0x6CCA50, CStreaming::RequestModel 0x4087E0 (cdecl(model, 8)),
  CPlane::FindPlaneCreationCoors 0x6CD090, CVehicle::operator new 0x6E2D50 (0xA18 heli / 0xA04 plane), CHeli::CHeli 0x6C4190 /
  CPlane::CPlane 0x6C8E20 (createdBy 1), CPlaceable::SetPosn 0x4241C0, CPlaceable heading setter 0x6C4130 (thiscall(float); builds the
  rotation from sin/cos of the angle), CCarCtrl::JoinCarWithRoadSystem 0x42F5A0, CWorld::Add 0x563220, CPhysical::SetVelocity 0x441130
  (thunk 0x156C740), CVehicle::SetEngineOn 0x41BDD0 (thunk 0x1567620; ignored when bEngineBroken), `rand`, CWorld::FindPlayerCoors 0x56E010,
  CWorld::FindUnsuspectingTargetCar 0x566C90 / FindUnsuspectingTargetPed 0x566DA0 (cdecl(CVector from, CVector playerPos)),
  CEntity::RegisterReference 0x571B70, CPedGroups::GetPedsGroup 0x5F7E80, CPed::IsPlayer 0x5DF8F0, CEventDanger ctor 0x4B2600
  (thiscall(CEntity*, float 200.0f)) / dtor 0x4B26A0, CEventGroup::Add 0x4AB420, CPedGroupIntelligence::AddEvent 0x5F7470,
  CPlane::SetGearUp 0x6CAC20, CStreaming::SetModelIsDeletable 0x409C10 / SetModelTxdIsDeletable 0x409C70, CPlane::AreWeInNoPlaneZone
  0x6CCAA0 / AreWeInNoBigPlaneZone 0x6CCBB0, CWorld::FindPlayerPed/FindPlayerVehicle, CPlayerPed::GetWantedLevel 0x41BE60 (thunk 0x156D300),
  CPool::GetAt 0x41CC10, CVehicle::GetVehicleAppearance 0x6D1080, CTheScripts::IsPlayerOnAMission 0x464D50, CGameLogic::LaRiotsActiveHere 0x441C10.
- Globals: CTimer::m_FrameCounter 0xB7CB4C, CTimer::m_snTimeInMilliseconds 0xB7CB84, CPlane::GenPlane_Active 0x8D33BC,
  GenPlane_ModelIndex 0xC1CAD8, GenPlane_Status 0xC1CADC, GenPlane_LastTimeGenerated 0xC1CAE0 (all RW), load states
  CStreaming::ms_aInfoForModel[m].m_nLoadState (0x8E4CD0 + m*0x14), CGame::currArea 0xB72914, CCutsceneMgr::ms_cutsceneProcessing 0xB5F852,
  CWeather::WeatherRegion 0xC81314 / OldWeatherType 0xC81320 / NewWeatherType 0xC8131C (i16), CPopCycle::m_bCurrentZoneIsGangArea 0xC0BC5C,
  vehicle pool 0xB74494. Constant 0x862D68 = 0.6666667f; immediates 200.0f, 0.22f (0x3E6147AE).
- Behaviour ("now" = m_snTimeInMilliseconds, "last" = GenPlane_LastTimeGenerated; "done" = `last = now; status = 0; return`):
  1. Return unless `(m_FrameCounter & 0x1F) == 0x1E` and `GenPlane_Active`. `count = CountPlanesAndHelis()`.
     `GenPlane_Status == 1` -> step 2; `== 0` -> step 3; any other value -> return.
  2. Creation (model `m = GenPlane_ModelIndex`): if `loadState[m] != 1`: `RequestModel(m, 8)`; return.
     - `m == 497` (police Maverick): `FindPlaneCreationCoors(&c, &t, &h, &fh, false)`; `v = new CHeli(497, 1)`; `v->SetPosn(c)`; heading(h);
       `JoinCarWithRoadSystem(v)`; rotor speed (+0x84C) = 0.22f; status PHYSICS; `CWorld::Add(v)`; `m_fMinAltitude = fh`; mission = 38;
       `+0x9B4 = h`; `m_fMaxAltitude = fh`; `SetVelocity(cos(h)*0.6667f, sin(h)*0.6667f, 0)`; bNeverUseSmallerRemovalRange;
       `m_nExtendedRemovalRange = 255`; cruise speed 50; `SetEngineOn(true)`.
       `rand() & 1 == 0`: `e = FindUnsuspectingTargetCar(heliPos, playerCoors)`; if `e`: `m_TargetEntity = e`, mission 39, RegisterReference,
       `m_nHeliFlags |= 2`, `m_nCarCtrlFlags |= 0xA0`.
       `== 1`: `p = FindUnsuspectingTargetPed(heliPos, playerCoors)`; if `p`: same assignments; `g = GetPedsGroup(p)`; if `!g || g[+0x2D0] == 1`:
       if `p->IsPlayer()` or `p[+0x484] == 2` or `p->m_nPedType == 6` -> done (no event); else `CEventDanger ev(v, 200.0f)` added to
       `p->m_pIntelligence->m_eventGroup` (Add(&ev, false)); else `CEventDanger ev(v, 200.0f)` -> `g->m_groupIntelligence.AddEvent(&ev)`. Done.
     - other models: `big = (m == 577 || m == 592)`; `FindPlaneCreationCoors(&c, &t, &h, &fh, big)`; `v = new CPlane(m, 1)`; `SetPosn(c)`;
       `m_planeCreationHeading = h`; heading(h); `JoinCarWithRoadSystem(v)`; status PHYSICS; `CWorld::Add`; `m_planeHeading = h`;
       `m_minAltitude = fh`; mission 36; `m_planeHeadingPrev = h`; `m_maxAltitude = fh`; `SetVelocity(cos(h)*0.6667f, sin(h)*0.6667f, 0)`;
       bNeverUseSmallerRemovalRange; `m_nExtendedRemovalRange = (!big && m != 520) ? 150 : 360`; if big `m_nFlags &= ~1`;
       if `m == 520`: mission 59, `m_wMiscComponentAngle = 0`; `SetGearUp()`; `SetEngineOn(true)`; `SetModelIsDeletable(m)`;
       `SetModelTxdIsDeletable(m)`. Done.
  3. Selection (status 0):
     a. If `currArea != 0` or `count > 4` or `AreWeInNoPlaneZone()` or a cutscene is processing: `last = now`; return.
     b. Hydra attack: if `FindPlayerPed()->GetWantedLevel() >= 4` and the player vehicle exists with sub-type 4 or 3, is not model 577/592,
        and `now > last + 60000`: count pool vehicles whose `m_nCarMission == 59` (descending scan); if that count is 0, or it is 1 and
        the wanted level is > 4: `GenPlane_ModelIndex = 520; GenPlane_Status = 1`.
     c. If `count > 0` or `GenPlane_Status != 0`: return.
     d. Small planes: if WeatherRegion is 0 or 4 and neither weather type is 19: if there is no player vehicle, or its appearance is not
        3 (heli) and not 5 (plane): if `now > last + 120000`: `r = rand() & 7`: 0,1,2,7 -> model 476; 3,4 -> 512; 5,6 -> 511; status 1;
        return. If status != 0 return.
     e. Gang area: if `m_bCurrentZoneIsGangArea`: if `!IsPlayerOnAMission()` and `now > last + 350000`: model 497, status 1, return.
        If status != 0 return.
     f. If `LaRiotsActiveHere()`: status 1, model 497, return.
     g. Big planes: if status == 0 and WeatherRegion not 0 and not 4 and not gang area and neither weather type is 19 and
        `now > last + 200000` and `!AreWeInNoBigPlaneZone()`, and (no player vehicle or appearance not 3/5): `model = (rand() & 1) ? 592 : 577`;
        status 1.
- gta-reversed: plugin::Call (no C++).
- Open questions (confidence low; derived from the decompile and spot-checked compares, not every instruction was transcribed):
  the meaning of `g[+0x2D0]` and `p[+0x484]` (0x6CD545, 0x6CD58D); the two 16-bit stores into the CEventDanger local (0x38F / 0x5E1 in
  the decompile) before the Add calls; the exact signed/unsigned nature of each time comparison (expected unsigned, as elsewhere);
  the order in which step 3b reads the wanted level relative to the vehicle checks. Verify against 0x6CD81D-0x6CDBF6 before porting.
