# CHeli (VEH) - full specs

Group D, RE-SPEC-2. Authority: disassembly. CHeli derives from CAutomobile (size 0x988; +0x5A0 m_damageManager, +0x5B8 m_doors[6],
+0x648 m_aCarNodes[25], +0x6AC m_panels[3] (0x20 each), +0x70C m_swingingChassis, +0x724 m_wheelColPoint[4], +0x7D4 suspension
compression[4], +0x828 m_wheelRotation[4], +0x838 m_wheelPosition[4], +0x848 m_wheelSpeed[4] (+0x84C = m_fHeliRotorSpeed), +0x868 autoFlags,
+0x878 m_aSuspensionSpringLength[4], +0x888 m_aSuspensionLineLength[4], +0x954 m_fForcedOrientation, +0x958 m_fPropRotate, +0x960
m_nNumContactWheels, +0x968 m_WheelStates[4], +0x984 m_heliDustFxTimeConst). CVehicle layout and compare idioms: see CBike.md.
CHeli fields (size 0xA18): +0x988 m_nHeliFlags (u8), +0x98C m_fLeftRightSkid, +0x990 m_fSteeringUpDown, +0x994 m_fSteeringLeftRight,
+0x998 m_fAccelerationBreakStatus, +0x9A0 m_fRotorZ, +0x9A4 m_fSecondRotorZ, +0x9A8 m_fMaxAltitude, +0x9B0 m_fMinAltitude,
+0x9B9 m_nNumSwatOccupants, +0x9BA m_aSwatState[4], +0x9C0 field_9C0, +0x9D4 m_pParticlesList, +0x9D8 m_aSearchLightHistoryX[3],
+0x9E4 m_aSearchLightHistoryY[3], +0x9F0 m_nSearchLightTimer, +0x9F4 m_vecSearchLightTarget, +0xA00 m_fSearchLightIntensity,
+0xA04 m_nShootTimer, +0xA08 m_nPoliceShoutTimer, +0xA0C m_ppGunflashFx, +0xA10 m_nFiringMultiplier, +0xA11 m_bSearchLightEnabled.
CHeli vtable 0x871680. Statics: pHelis[2] 0xC1C964, TestForNewRandomHelisTimer 0xC1C960, NumberOfSearchLights 0xC1C96C,
bPoliceHelisAllowed 0x8D338C.

## 0x6C4340 CHeli::~CHeli  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall void ()`, plain `ret`; SEH frame. Caller: CHeli scalar deleting dtor 0x6C4810.
- Callees: CVehicle::GetPlaneNumGuns 0x6D3F30 (thiscall -> int32), FxSystem_c::Kill 0x4AA3F0, FxManager_c::DestroyFxSystem 0x4A9810
  (thiscall on g_fxMan 0xA9AE80, (FxSystem_c*)), `free` 0x8213AE, CAEVehicleAudioEntity::Terminate 0x4FB8C0, CAutomobile::~CAutomobile 0x6A61E0.
- Behaviour: `vtable = 0x871680`; if `m_ppGunflashFx` (+0xA0C) != null: for `i = 0; i < GetPlaneNumGuns(); i++` (GetPlaneNumGuns is
  called before the loop and again after each iteration): if `m_ppGunflashFx[i]`: `Kill()` it, then `g_fxMan.DestroyFxSystem(m_ppGunflashFx[i])`;
  then `free(m_ppGunflashFx)`; `m_ppGunflashFx = nullptr`. `m_vehicleAudio.Terminate()`; `CAutomobile::~CAutomobile()`.
- gta-reversed: `~CHeli()` hooked at 0x6C4340/0x6C4810 per the header comment; behaviour above is from the bytes.

## 0x6C4550 CHeli::SetupDamageAfterLoad  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall void ()`, plain `ret`.
- Behaviour: `and byte [ecx+0x42A], 0xFE` (0x6C4550): clears bit 0 of byte +0x42A, i.e. bit 16 of `m_nVehicleUpperFlags`
  (+0x428) = `vehicleFlags.bIsDamaged`. Nothing else.
- gta-reversed: `vehicleFlags.bIsDamaged = false`, matches.

## 0x6C4750 CHeli::SpecialHeliPreRender  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void ()` (static), single `ret` (0x6C4750). Caller: CRenderer::PreRender 0x553910.
- Behaviour: empty.
- gta-reversed: empty "dummy function", matches.

## 0x6C4760 CHeli::FindSwatPositionRelativeToHeli  (catalogue: specified/REA, port level 0)
- Convention: `__thiscall CVector* (CVector* out [esp+4], int32 swatNumber [esp+8])` with hidden return pointer, `ret 8`;
  EAX = `out` on return (each path does `mov eax,[esp+4]`). `this` (ECX) is not used.
- Callers: CHeli::SendDownSwat 0x6C69C0, CHeli::ProcessControl 0x6C7050 (2 sites).
- Behaviour: `if ((uint32)swatNumber > 3)` (unsigned `cmp eax,3; ja`, 0x6C4764) -> out = (0,0,0) (integer zero stores, 0x6C47DC).
  Else jump table at 0x6C47F0 = {0x6C47C1, 0x6C47A6, 0x6C478B, 0x6C4770}:
  - 0: (-1.2f, -1.0f, -0.5f) (bits 0xBF99999A, 0xBF800000, 0xBF000000; 0x6C47C1)
  - 1: ( 1.2f, -1.0f, -0.5f) (0x3F99999A, 0xBF800000, 0xBF000000; 0x6C47A6)
  - 2: (-1.2f,  1.0f, -0.5f) (0xBF99999A, 0x3F800000, 0xBF000000; 0x6C478B)
  - 3: ( 1.2f,  1.0f, -0.5f) (0x3F99999A, 0x3F800000, 0xBF000000; 0x6C4770)
  Stores are x, y, z in that order; negative indices fall into the default (unsigned compare).
- gta-reversed: plugin::Call to 0x6C4760 followed by unreachable C++ that matches the table above.

## 0x6C4810 CHeli::vfunc_0 (proposed name: CHeli scalar deleting destructor)  (catalogue: specified/REA, port level 17)
- Convention: vtable slot 0, `__thiscall CHeli* (uint8 flags [esp+4])`, `ret 4`, EAX = this. Callers (virtual): CCarCtrl::CreatePoliceChase,
  CreateConvoy, GenerateOneRandomCar, CSetPiece::Update.
- Behaviour: `CHeli::~CHeli(0x6C4340)(this)`; if `flags & 1`: `CVehicle::operator delete(this)` (0x6E2D90, cdecl, pool free); return this.
- Name: the catalogue name `vfunc_0` is the compiler's scalar deleting destructor (vtable slot 0, test of bit 0 then delete).
- gta-reversed: not hooked (the C++ `delete` expression produces it).

## 0x6C4830 CHeli::ProcessControlInputs  (catalogue: gr-unreversed/REA, port level 10)
- Convention: virtual (vtable +0x38) `__thiscall void (uint8 playerNum [esp+4])`, `ret 4` (arg slot reused as int temp).
  Caller: CAutomobile::ProcessAI 0x6B4800 (virtual).
- Callees: CPad::GetPad 0x53FB70 (8 calls, cdecl(int)), CPad::GetAccelerate 0x5403F0 / GetBrake 0x540080 / GetSteeringLeftRight 0x53FB80 /
  GetSteeringUpDown 0x53FBD0 (thiscall -> int16), GetLookLeft 0x53FDD0 / GetLookRight 0x53FE10 / GetHorn 0x53FEE0 (-> AL), CVector::CrossProduct
  0x59C730, CVector::Normalise 0x59C910, CWorld::FindPlayerPed 0x56E210, CPlayerPed::KeepAreaAroundPlayerClear 0x60C1E0, CRT `_CIpow` 0x822130
  (ST0 = pow(ST1, ST0)).
- Globals: CCamera::m_bUseMouse3rdPerson 0xB6EC2E, CVehicle::m_bEnableMouseFlying 0xC1CC03, CVehicle::m_nLastControlInput 0xC1CC04 (RW,
  0 = pad, 2 = mouse), CPad::NewMouseControllerState X 0xB73424 / Y 0xB73428 (f32), CTimer::ms_fTimeStep. Constants: 0x859A3C 1/255,
  0x858B50 0.0f, 0x858B88 0.0078125f (1/128), 0x871674 0.0025f, 0x871678 0.5f, 0x87167C 0.98f, 0x858624 1.0f, 0x858C1C -1.0f,
  0x858CA0 2.0f, 0x858B18 -2.0f, 0x871254 0.28f, 0x859F80 250.0f, 0x858B8C 0.5f; immediates -1.0f, 1.0f, -0.1f.
- Fields: CHeli +0x98C m_fLeftRightSkid, +0x990 m_fSteeringUpDown, +0x994 m_fSteeringLeftRight, +0x998 m_fAccelerationBreakStatus;
  +0x388 m_pFlyingHandlingData (+0x1C m_fRollStab, +0x24 m_fPitchStab); CPad +0x2C NewState.m_bVehicleMouseLook (i16), +0x10E
  DisablePlayerControls (u16).
- Behaviour:
  1. `pad = GetPad(playerNum)`; `accel(+0x998) = (float)(GetAccelerate() - GetBrake()) * (1/255)` (int subtraction, fild).
  2. Steering:
     - If `m_bUseMouse3rdPerson && m_bEnableMouseFlying` (0x6C4873): mouse path M is taken if `mouseX != 0` or `mouseY != 0`
       (GetPad is called before each read, results unused); otherwise if (`|LR| > 0` or `|UD| > 0`) and `m_nLastControlInput == 2` and
       both GetSteeringLeftRight and GetSteeringUpDown (of GetPad(playerNum)) are 0 -> M. Otherwise: if both pad steering values are 0
       and `m_nLastControlInput == 2` -> keep UD/LR (go to 3); else `m_nLastControlInput = 0`, `UD = (float)GetSteeringUpDown() / 128`
       (`* 0.0078125f`), `LR = (float)(-GetSteeringLeftRight()) * 0.0078125f`.
       M (0x6C4993): `m_nLastControlInput = 2`; if `pad(playerNum)->NewState.m_bVehicleMouseLook == 0`: `LR = LR - mouseX * 0.0025f`;
       `UD = mouseY * 0.0025f + UD`. Then if `|LR| < 0.5f`: `LR *= pow(0.98f, ms_fTimeStep)`; if `|UD| < 0.5f`: `UD *= pow(0.98f, ms_fTimeStep)`.
     - Else (0x6C4A4E): `m_nLastControlInput = 0`; `UD = GetSteeringUpDown() * (1/128)`; `LR = -GetSteeringLeftRight() * (1/128)`.
  3. Clamp UD then LR to [-1, 1]: `if (1.0f < v) v = 1.0f; else if (-1.0f > v) v = -1.0f` (NaN kept).
  4. `skid(+0x98C) = (float)GetLookRight()` (0 or 1); if `GetLookLeft()` `skid = -1.0f`.
  5. Horn hover-assist (0x6C4B4F): if `GetHorn() && m_matrix->up.z > 0.0f`: `skid = 0`; `n1 = normalise(CrossProduct((0,0,1), right))`;
     `v = ((n1.y*ms.y + n1.z*ms.z) + n1.x*ms.x) * fh->m_fPitchStab`; `UD = clamp(v, -2.0f, 2.0f)` (`v < -2 -> -2`, `v > 2 -> 2`, the
     dot product is recomputed for every test); `n2 = normalise(CrossProduct(fwd, (0,0,1)))`; `LR = clamp(dot(n2, ms) * fh->m_fRollStab,
     -2.0f, 2.0f)` (same evaluation order).
  6. `bIsHandbrakeOn = 0`; `m_fSteerAngle = 0`; `m_BrakePedal = 1.0f`; `m_GasPedal = 0`.
  7. If `pad->DisablePlayerControls != 0`: `FindPlayerPed(-1)->KeepAreaAroundPlayerClear()`; `s = sqrt((ms.x^2 + ms.y^2) + ms.z^2)`;
     if `s > 0.28f`: `k = 0.28f / s`; `ms.x *= k; ms.y *= k; ms.z *= k` (`k*ms.c` stored f32).
  8. If `m_fHealth < 250.0f`: `accel = -0.1f`; `skid += 0.5f`.
- gta-reversed: plugin::Call (no C++).

## 0x6C4E60 CHeli::ProcessFlyingCarStuff  (catalogue: gr-unreversed/REA, port level 11)
- Convention: virtual (vtable +0xE8) `__thiscall void ()`, plain `ret`; SEH frame (two CMatrix locals). Caller: CAutomobile::ProcessControl.
- Callees: CVehicle::FlyingControl 0x6D85F0 (thiscall(eFlightModel, skid, upDown, leftRight, accel)), CMatrix::Attach 0x59BD10, ~CMatrix
  0x59ACD0, RwFrameForAllObjects 0x7F1200 + GetCurrentAtomicObjectCB 0x6A0750, _rpAtomicResyncInterpolatedSphere 0x7491F0,
  CVehicle::DoBladeCollision 0x6E2E50 (thiscall(CVector pos, CMatrix& m, int16 rotorType, float radius, float forceMult)),
  CWindModifiers::RegisterOne 0x72C8B0 (cdecl(CVector pos, int32 type, float power)), Multiply3x3 0x59C790,
  CAEVehicleAudioEntity::AddAudioEvent(int, float) 0x4F6420.
- Constants: 0x8717B0 0.00055f, 0x8717AC 0.22f, 0x858CDC 0.001f, 0x859CD8 0.003f, 0x858FCC 0.15f, 0x859EF8 0.0 (f64), 0x858B38 0.02f,
  0x8717A8 0.015f, 0x858B1C 0.1f, 0x8717A4 0.0075f, 0x866FBC 6.6666665f, 0x858624 1.0f, 0x8717A0 0.154f, 0x87179C 0.0044f,
  0x85A700 400.0f, 0x858F20 0.5235988f (pi/6), 0x858C58 0.01f, 0x858EF0 0.95f; immediates 1.0f, 0.9f, 0.8f, 0.5f.
- Fields: +0x84C rotor speed (m_fHeliRotorSpeed = CAutomobile m_wheelSpeed[1]), +0x82C m_wheelRotation[1], +0x958 m_fPropRotate,
  +0x678 m_aCarNodes[12] (moving rotor), +0x960 m_nNumContactWheels, +0x98C..+0x998 skid/UD/LR/accel, +0xB8 m_nFakePhysics,
  handling +0xCC (0x02000000 m_bIsHeli), +0x42C bit 7 (bIsRCVehicle), physical flags 0x100 (bSubmergedInWater), 0x08000000 (bTouchingWater).
- Behaviour:
  1. If status not in {PLAYER 0, REMOTE_CONTROLLED 8, PHYSICS 3}: return if the handling is not a heli; `bEngineOn = 0`;
     `t = ms_fTimeStep * 0.00055f`; if `t < rotor`: `rotor -= t; m_nFakePhysics = 0` else `rotor = 0`; go to 4.
  2. Spin-up: if `rotor < 0.22f` and not submerged: `rotor += (model 465 or 501) ? 0.003f : 0.001f` (per call, not time-scaled).
     If `rotor > 0.15f`: `water = touchingWater && (model 447 || model 417)`;
     if bIsRCVehicle: `FlyingControl(2, skid, UD, LR, accel)`; else if `(m_nNumContactWheels < 4 && !water)` or `accel > 0.0` or any
     `|moveSpeed.c| > 0.02f` (x, y, z in that order): `FlyingControl(6, skid, UD, LR, accel)`.
  3. Blade collision / wind, if `rotor > 0.015f` and rotor node present: `A.Attach(&node->modelling, false)`; `a` = its current atomic;
     if `a`: resync its interpolated sphere if `a->interpolator.flags (+0x4C) & 2`; `r = a->boundingSphere.radius (+0x28)`; if `r > 0.1f`:
     `k = 1.0f` (465/501: 0.9f; 469/447: 0.8f; 425: 0.5f); if status PLAYER or REMOTE: `DoBladeCollision(A.pos, *m_matrix, -3, r, k)`.
     Then if status is PLAYER or PHYSICS and `rotor > 0.0075f`: `RegisterOne(pos, 1, min(rotor * 6.6666665f, 1.0f))` (a status-SIMPLE
     branch registering power 1.0f exists but is unreachable here). Destroy A.
  4. Rotor whoosh (all paths except the non-heli return): if model not 465/501 and `0.0044f < rotor < 0.154f` and rotor node present:
     `d = camPos - pos`; `sq = (d.x*d.x + d.y*d.y) + d.z*d.z`; if `sq < 400.0f` and `|m_fPropRotate - m_wheelRotation[1]| > pi/6`:
     `B.Attach(&node->modelling, false)`; `v = Multiply3x3(*m_matrix, B.right)`; `dist = max(sqrt(sq), 0.01f)`; `n = d * (1/dist)`;
     if `|(v.z*n.z + v.y*n.y) + v.x*n.x| > 0.95f`: `m_vehicleAudio.AddAudioEvent(94, 0.0f)`; `m_fPropRotate = m_wheelRotation[1]`.
- gta-reversed: plugin::Call (no C++).

## 0x6C5420 CHeli::PreRender  (catalogue: gr-unreversed/REA, port level 12)
- Convention: virtual (vtable +0x1C in the CHeli table) `__thiscall void ()`, plain `ret`; SEH frame (one CMatrix `M`, attach ptr/owns zeroed).
- Callees: CVehicle::PreRender 0x6D6480, CClock::GetIsTimeInRange 0x52CEE0 (cdecl(uint8 from, uint8 to)), CMatrix*CVector 0x59C890,
  CHeli::AddHeliSearchLight 0x6C45B0 (cdecl(const CVector& origin, const CVector& target, float radius, float power, uint32 corona,
  uint8, uint8)), CEntity::GetColModel 0x535300 (result unused), virtual DoBurstAndSoftGroundRatios (vtable +0xD0),
  CVehicleModelInfo::GetWheelPosn 0x4C7D20 (thiscall(int, CVector&, bool)), CAutomobile::UpdateWheelMatrix 0x6AA290 (thiscall(int node, int flags)),
  CAutomobile::DoHeliDustEffect 0x6B0690 (thiscall(float, float)), CMatrix::Attach 0x59BD10, SetRotateZ 0x59B0E0, SetRotateX 0x59B060,
  UpdateRW 0x59BBB0, CShadows::StoreShadowForVehicle 0x70BDA0 (cdecl(CVehicle*, int)).
- Globals/constants: 0x8D33A0 = 1.66f (.data), 0x858F54 2.3f, 0x863234 -6.2831855f, 0x858CBC 6.2831855f, 0x858624 1.0f, 0x858B50 0.0f,
  0x858F34 0.75f; immediates (0, 3.5f, -0.3f), 20.0f.
- Behaviour:
  1. `CVehicle::PreRender()`.
  2. Searchlight: if `m_bSearchLightEnabled (+0xA11)` and `m_fSearchLightIntensity (+0xA00) > 0.0f` and `CClock::GetIsTimeInRange(19, 6)`:
     `AddHeliSearchLight((*m_matrix) * (0, 3.5f, -0.3f), m_vecSearchLightTarget (+0x9F4), 20.0f, m_fSearchLightIntensity,
     (uint32)this + 0xB, 1, 1)`.
  3. `GetColModel()` (value discarded).
  4. If byte +0x42B & 1 (bVehicleColProcessed): `DoBurstAndSoftGroundRatios()`; for wheel i = 0..3:
     `r = 1.0f - spring[i] / line[i]` (m_aSuspensionSpringLength +0x878, m_aSuspensionLineLength +0x888);
     `c = (m_fWheelsSuspensionCompression[i] (+0x7D4) - r) / (1.0f - r)`; `p = mi->GetWheelPosn(i, true)`; `z = p.z + handling->m_fSuspensionUpperLimit`;
     `if (c > 0.0f) z = z - c * spring[i]`; then `w = m_wheelPosition[i]` (+0x838): if `z > w` -> `w = z`; else if physical flag 0x04000000
     and handling flag 0x20000 (bHydraulicInst) -> `w = z`; else `w = (z - w) * 0.75f + w`.
  5. `UpdateWheelMatrix(4, 1)`, `(7, 1)`, `(2, 1)`, `(5, 1)`; if model not 465/501: `DoHeliDustEffect(1.0f, 1.0f)`.
  6. Rotors: `s = m_fHeliRotorSpeed (+0x84C)`; `m_fRotorZ (+0x9A0) -= ms_fTimeStep * s * ((model in {469, 447, 487, 488, 497}) ? 1.66f : 1)`
     (the factor multiply only exists for those models); while `m_fRotorZ < -2pi` add 2pi.
     `m_fSecondRotorZ (+0x9A4) -= (model == 417) ? (ts*s + ts*s) : ts*s*2.3f`; while `> 2pi` subtract 2pi (it only decreases, so this
     wrap never triggers in practice - keep).
  7. For nodes 12 and 13 (+0x678, +0x67C) if present: `M.Attach(&node->modelling, false)`; `p = M.pos`; `M.SetRotateZ(m_fRotorZ)`;
     `M.pos = M.pos + p` (component-wise fadd); `M.UpdateRW()`. For nodes 14 and 15 (+0x680, +0x684): same with `SetRotateX(m_fSecondRotorZ)`.
  8. `CShadows::StoreShadowForVehicle(this, 2)`; destroy M.
- gta-reversed: plugin::Call (no C++).

## 0x6C58E0 CHeli::SearchLightCone  (catalogue: specified/REA, port level 8)
- Convention: `__cdecl void (int32 coronaIndex [+4], CVector origin [+8], CVector target [+0x14], float targetRadius [+0x20],
  float power [+0x24], uint8 clipIfColliding [+0x28], uint8 drawShadow [+0x2C], CVector* out0 [+0x30], CVector* out1 [+0x34],
  CVector* out2 [+0x38], bool a11 [+0x3C], float baseRadius [+0x40], float a13 [+0x44], float a14 [+0x48], float a15 [+0x4C])`
  (offsets from the entry ESP), plain `ret`. Arguments `clipIfColliding`, `a11`, `a13..a15` are never read. The `target`
  argument slots are modified in place. Callers: CTheScripts::RenderAllSearchLights 0x493E30, CHeli::RenderAllHeliSearchLights 0x6C7C50.
- Callees: CVector::Normalise 0x59C910 (x6), CVector::CrossProduct(out,a,b) 0x59C730 (x2), CWorld::ProcessLineOfSight 0x56BA00,
  `_ftol` 0x821B40 (x14), CCoronas::RegisterCorona (texture overload) 0x6FC580, RwIm3DTransform 0x7EF450,
  RwIm3DRenderIndexedPrimitive 0x7EF550, RwIm3DEnd 0x7EF520, CShadows::StoreShadowToBeRendered (texture overload) 0x707390.
- Globals: TheCamera matrix ptr 0xB6F03C / placement 0xB6F02C, CCustomBuildingDNPipeline::m_fDNBalanceParam 0x8D12C0,
  uiTempBufferVerticesStored 0xC4B950, uiTempBufferIndicesStored 0xC4B954 (both RW), aTempBufferIndices 0xC4B958 (u16),
  TempVertexBuffer 0xC4D958 (RwIm3DVertex stride 0x24: pos +0, colour +0x18), gpShadowExplosionTex 0xC403F4.
  Constants: 0x858B3C 3.0f, 0x858628 100.0f, 0x858B50 0.0f, 0x858BA4 20.0f, 0x859AAC 255.0f, 0x8717B4 0.15707964f (pi/20),
  0x858FCC 0.15f, 0x858B1C 0.1f, 0x858624 1.0f, 0x858A48 200.0f, 0x858C80 5.0f, 0x858F08 1.2f, 0x858FE8 25.0f, 0x858CEC 0.04f,
  0x858B8C 0.5f, 0x858BF4 128.0f.
- Behaviour (block level):
  1. Beam clip (0x6C58E6-0x6C59F0): `d = normalise(target - origin)`; `target += d * 3.0f` (written back into the argument);
     if `ProcessLineOfSight(origin, target, cp, ent, true, false, false, false, false, false, false, false)` then `target = cp.point`.
  2. Corona (0x6C59F7-0x6C5B24): `c = normalise(camPos - origin)` (camPos = TheCamera matrix pos or placement pos);
     `f = dot(c, d)` (`c.x*d.x` first); `if (f < 0) f = 0`; `f6 = f^6` (five chained multiplies); `RegisterCorona(coronaIndex, nullptr,
     200, 200, 255, _ftol(f6 * 255.0f), origin, 20.0f * f6, 100.0f, nullptr, 0, true, false, 0, 0.0f, false, 1.5f, 0, 15.0f, false, false)`.
     (`origin + d*100` is also computed into a local at 0x6C5A02; see open questions.)
  3. Cone mesh (0x6C5B29-0x6C60CF): `uiTempBufferVerticesStored = uiTempBufferIndicesStored = 0`; `maxI = 0`;
     `A = normalise(CrossProduct(d, (0,0,1)))`, `B = normalise(CrossProduct(A, d))` (recomputed every iteration).
     For `i = 0..40` inclusive: `s = sin(i * pi/20)`, `k = cos(i * pi/20)` (fild of i, `fmul`, `fsin`/`fcos` of the same value);
     `P0 = origin + baseRadius*(k*B + s*A)`; `Pf = target + targetRadius*(k*B + s*A)`; `t = (P0.z - target.z) / (P0.z - Pf.z)`;
     `P1 = P0 + (Pf - P0) * t`; capture `P1` when `i == 20` (`R20`) and `i == 30` (`R30`) (captured before the following clamp);
     if `|P1 - P0| > 100.0f`: `P1 = P0 + normalise(P1 - P0) * 100.0f`.
     Vertex `v` = P0, vertex `v+1` = P1; `w = (P0 - origin)` normalised; `q = dot(w, c)`; `if (q < 0) q = -q`; `q = q*q`;
     `if (q > maxI) maxI = q`; per-vertex factors `I[v] = m_fDNBalanceParam * 0.15f + 0.1f`, `I[v+1] = 0`, `Q[v] = Q[v+1] = q`.
     If `i != 40`: indices `(v, v+3, v+1)`; and if `baseRadius > 0.0f` also `(v, v+2, v+3)`. `v += 2`.
  4. Colours: `inv = 1.0f / maxI`; for every vertex `j`: `x = Q[j] * I[j] * inv`; colour dword = `(b200 << 16) | (b200 << 8) | b255` with
     `b200 = _ftol(200.0f * x) & 0xFF`, `b255 = _ftol(x * 255.0f) & 0xFF` (alpha byte 0) (loop unrolled by 4, same result).
  5. If index count > 0 and `RwIm3DTransform(TempVertexBuffer, count, nullptr, 0x18)` != null:
     `RwIm3DRenderIndexedPrimitive(3 /*TRILIST*/, aTempBufferIndices, indexCount)`, `RwIm3DEnd()`.
  6. Outputs (0x6C62B5-0x6C6398): `*out0 = target`; `*out1 = R20 - target`; `*out2 = R30 - target` (x, y, z).
  7. Shadow (if `drawShadow`): `pos = (target.x, target.y, target.z + 5.0f)`; `f1 = (out1.x*1.2f, out1.y*1.2f)`, `f2 = (out2.x*1.2f,
     out2.y*1.2f)`; require `sqrt(f1.x^2 + f1.y^2) < 100` and `sqrt(f2.x^2 + f2.y^2) < 100` (NaN aborts) and
     `dist = sqrt((target.y-cam.y)^2 + (target.x-cam.x)^2) <= 25.0f` (`> 25` aborts, NaN continues);
     `k = (1.0f - dist*0.04f) * power * 0.5f`; `StoreShadowToBeRendered(2, gpShadowExplosionTex, pos, f1.x, f1.y, f2.x, f2.y,
     (int16)_ftol(k*128.0f), (uint8)_ftol(200.0f*k), (uint8)_ftol(200.0f*k), (uint8)_ftol(255.0f*k), 15.0f, true, 1.0f, nullptr, false)`.
- gta-reversed: signature only (plugin call); gr calls the out vectors "useless", but the original writes all three.
- Open questions (confidence low): the exact x87 addition order of the ring-point sums `P0`/`Pf` differs between components in the
  bytes (0x6C5BCD-0x6C5DB1) and was not fully transcribed; the role of the local `origin + d*100` (0x6C5A02-0x6C5A49) and of the
  locals at [esp+0xA8..0xB0] used in the Pf sum (0x6C5D33) must be confirmed against the bytes before porting.

## 0x6C6520 CHeli::GenerateHeli  (catalogue: specified/REA, port level 29)
- Convention: `__cdecl CHeli* (CPed* target [esp+4], bool newsHeli [esp+8])`, plain `ret`, EAX = new heli; SEH frame. The `target`
  argument slot is reused for the spawn angle. Caller: CHeli::UpdateHelis 0x6C79A0.
- Callees: CVehicle::operator new 0x6E2D50 (size 0xA18), CHeli::CHeli 0x6C4190 (thiscall(int32 model, int createdBy)), `rand`,
  CWorld::FindGroundZForCoord 0x569660 (cdecl(x, y) -> ST0), CMatrix::SetTranslate 0x59AF40 (thiscall(float x, y, z)), CWorld::FindPlayerPed
  0x56E210, CWorld::Add 0x563220, CVehicle::SetUpDriver 0x6D1A50 (thiscall(int pedType, bool, bool)).
- Globals: CHeli::pHelis 0xC1C964 (read only in a dead loop). Constants: 0x859C44 0.02453125f (~2pi/256), 0x859F80 250.0f,
  0x859A90 -3000.0f, 0x859A94 3000.0f, 0x858CB8 pi (3.1415927f), 0x858B40 50.0f, 0x8717C0 -2322.0f, 0x8717BC -1653.0f, 0x858A4C 350.0f,
  0x858A48 200.0f, 0x8717B8 560.0f, 0x858BA4 20.0f; immediates 20.0f, 12.0f, 30.0f, 27.0f, 0.165f, 1.0f.
- Behaviour:
  1. `heli = new CHeli(newsHeli ? 488 : 497, 4)` (no null check afterwards).
  2. `T = target->GetPosition()`; `a = (float)(rand() & 0xFF) * 0.02453125f` (stored f32); `P.x = cos(a)*250.0f + T.x`,
     `P.y = sin(a)*250.0f + T.y`. If `P.x < -3000 || P.x > 3000 || P.y < -3000 || P.y > 3000`: `a = a + pi` (stored) and P recomputed
     from T with the new angle. `P.z = T.z + 50.0f`.
  3. If `sqrt((P.y + 1653)^2 + (P.x + 2322)^2) < 350.0f` (i.e. near (-2322, -1653)): `x = T.x, y = T.y, z = T.z + 200.0f` and
     `if (!(z > 560.0f)) z = 560.0f`; else `x = P.x, y = P.y, z = P.z`.
  4. `g = FindGroundZForCoord(x, y) + 20.0f`; `z = (z > g) ? z : g`; `heli->m_matrix->SetTranslate(x, y, z)`.
  5. status := PHYSICS (3); bIsLocked (+0x428 |= 8). (A loop scanning `pHelis[0..1]` for a free slot runs here but its result is
     discarded - dead code, keep or drop.)
  6. If `newsHeli`: `m_nCarMission = 41`, `m_TargetEntity = FindPlayerPed(-1)`, `m_nCruiseSpeed = 35`; else `m_nCarMission = 40`,
     `m_nCruiseSpeed = 70`. Then unconditionally `m_TargetEntity = target` (overwrites the news-heli assignment; no RegisterReference).
  7. `m_fMaxAltitude (+0x9A8) = 20.0f`, `m_fMinAltitude (+0x9B0) = 12.0f`; news heli: 30.0f / 27.0f. `m_fHeliRotorSpeed (+0x84C) = 0.165f`.
  8. Orientation with `b = a + pi` (f32): right = (sin b, -cos b, 0), forward = (cos b, sin b, 0), up = (0, 0, 1) (zeros as int stores).
  9. `CWorld::Add(heli)`; `heli->SetUpDriver(-1, false, false)`; return heli.
- gta-reversed: plugin::Call (no C++).

## 0x6C69C0 CHeli::SendDownSwat  (catalogue: gr-unreversed/REA, port level 18)
- Convention: `__thiscall bool ()`, plain `ret`, AL; SEH frame (one CMatrix local). Caller: CHeli::ProcessControl 0x6C7050.
- Callees: `rand`; CVector::Magnitude 0x4082C0; CMatrix copy ctor 0x59BCF0 / dtor 0x59ACD0; CHeli::FindSwatPositionRelativeToHeli 0x6C4760;
  Multiply3x3(out, const CMatrix&, const CVector&) 0x59C790; CVector::operator+= 0x411A00; CWorld::FindGroundZFor3DCoord 0x5696C0
  (cdecl(x, y, z, bool*, CEntity**) -> ST0); CWaterLevel::GetWaterLevelNoWaves 0x6E8580 (cdecl(x, y, z, float* out, float*, float*) -> AL);
  CRopes::RegisterRope 0x556B40 (cdecl(uint32 id, uint32 type, CVector pos, bool expires, uint8 segs, uint8 flags, CPhysical* holder,
  uint32 time) -> int32); CPopulation::AddPed 0x612710 (cdecl(ePedType, uint32 model/copType, const CVector&, bool) -> CPed*);
  CTask::operator new 0x61A5A0; CTaskComplexSequence ctor 0x632BD0 / AddTask 0x632D10; CTaskComplexUseSwatRope ctor 0x659470
  (thiscall(uint32 ropeId, CHeli*)); CGeneral::GetRandomNumberInRange(int, int) 0x407180 (original helper); CTaskComplexWanderCop ctor
  0x460C80 (thiscall(int moveState, uint8 dir)); CTaskManager::SetTask 0x681AF0 (thiscall(CTask*, int slot, bool)); CAnimManager::BlendAnimation
  0x4D4610 (cdecl(RpClump*, int group, int animId, float delta)).
- Globals: byte 0x8E6314 = CStreaming::ms_aInfoForModel[285].m_nLoadState (0x8E4CC0 + 285*0x14 + 0x10; model 285 = SWAT).
  Constants: 0x858B40 50.0f, 0x858B1C 0.1f, 0x858B50 0.0f, 0x858FA0 2.5f, 4.0f (0x40800000).
- Fields: +0x41C m_autoPilot.m_TargetEntity, +0x9B9 m_nNumSwatOccupants (u8), +0x9BA m_aSwatState[4], +0x40 physical flags
  (0x100 bSubmergedInWater).
- Behaviour:
  1. `tp` = target's position (matrix pos or placement) - read BEFORE any check, with no null check of the target (0x6C69E0).
  2. Return false if `m_nNumSwatOccupants == 0`, if bSubmergedInWater, if the SWAT model load state != 1, if `(rand() & 0x7F) != 0`,
     if `|heliPos - tp| > 50.0f` (sqrt((dx^2 + dy^2) + dz^2), x-difference squared first), or if `|m_vecMoveSpeed| > 0.1f`.
  3. `M = *m_matrix`; `P = pos + Multiply3x3(M, FindSwatPositionRelativeToHeli(m_nNumSwatOccupants - 1))`.
  4. `g = FindGroundZFor3DCoord(P.x, P.y, P.z, nullptr, nullptr)`; fail unless `|tp.z - g| < 2.5f`.
     If `GetWaterLevelNoWaves(P.x, P.y, P.z, &w, nullptr, nullptr)` and `!(w < g)`: fail.
  5. `id = (uint32)this + m_nNumSwatOccupants - 1` (byte address used as rope id); if `RegisterRope(id, 8, P, false, 0, 0, nullptr, 20000) < 0`: fail.
  6. `ped = CPopulation::AddPed(6 /*PED_TYPE_COP*/, 3, P, true)`; `seq = new CTaskComplexSequence`; `seq->AddTask(new
     CTaskComplexUseSwatRope(id, this))`; `seq->AddTask(new CTaskComplexWanderCop(4, GetRandomNumberInRange(0, 8)))`;
     `ped->m_pIntelligence->m_TaskMgr.SetTask(seq, 3, false)`; `ped->m_nFlags &= ~1` (bUsesCollision off).
  7. `m_nNumSwatOccupants--`; `m_aSwatState[m_nNumSwatOccupants] = 0xAA`; `BlendAnimation(ped->m_pRwClump, 0, 152, 4.0f)`; return true.
  "fail" destroys M and returns false. No null checks on `ped` or on the new tasks other than passing null to AddTask.
- gta-reversed: plugin::Call (no C++).

## 0x6C6D30 CHeli::BlowUpCar  (catalogue: gr-unreversed/REA, port level 17)
- Convention: virtual (vtable +0x7C) `__thiscall void (CEntity* damager [esp+4], bool hideExplosion [esp+8])`, `ret 8`.
  `hideExplosion` is never read.
- Callees: CWorld::FindPlayerPed 0x56E210 (cdecl(int=-1)), CWorld::FindPlayerVehicle 0x56E0D0 (cdecl(int=-1, bool=false)), `rand`,
  CStats::IncrementStat 0x55C180 (cdecl(int stat, float)), CVisibilityPlugins::SetClumpForAllAtomicsFlag 0x7322F0, CDamageManager::
  FuckCarCompletely 0x6C25D0 (thiscall(bool)), CAutomobile::SetBumperDamage 0x6B1350 / SetDoorDamage 0x6B1600 (thiscall(int, bool)),
  CAutomobile::SpawnFlyingComponent 0x6A8580 (thiscall(int node, uint32 colType)), RwFrameForAllObjects 0x7F1200 with
  GetCurrentAtomicObjectCB 0x6A0750, CCamera::CamShake 0x50A9F0, CVehicle::KillPedsInVehicle 0x6D1C80, CVehicle::ChangeLawEnforcerState
  0x6D2330, CFireManager::StartFire(CEntity*, CEntity*, float, uint8, uint32, int8) 0x53A050 (on gFireManager 0xB71F80),
  CDarkel::RegisterCarBlownUpByPlayer 0x43DF20, CExplosion::AddExplosion 0x736A50.
- Globals: CWorld::PlayerInFocus 0xB7CD74 (u8), CWorld::Players[].m_nHavocCaused 0xB7CED8 + 0x190*i (u32) and .m_fCurrentChaseValue
  0xB7CEE0 + 0x190*i (f32), CWanted::UseNewsHeliInAdditionToPolice 0xB7CB8C, CTimer::m_snTimeInMilliseconds 0xB7CB84,
  CCarCtrl::NumAmbulancesOnDuty 0x9690A8, NumFireTrucksOnDuty 0x9690AC. Constant 0x85862C = 10.0f.
- Behaviour:
  1. If `!bCanBeDamaged` (byte +0x42A & 0x20) return.
  2. If status != PLAYER && `m_autoPilot.m_nCarMission` (byte +0x3BA) != 58 (MISSION_HELI_CRASH_AND_BURN) && model != 465 && model != 501
     (RC heli models): `m_nCarMission = 58`; `m_fHealth = 0`; return (the heli crashes first; the explosion happens later).
  3. If `damager == FindPlayerPed(-1) || damager == FindPlayerVehicle(-1, false)`: `Players[PlayerInFocus].m_nHavocCaused += 20`;
     `.m_fCurrentChaseValue += 10.0f`; `CStats::IncrementStat(125, (float)(rand() % 6000 + 4000))`.
  4. If model == 488 (0x1E8): `UseNewsHeliInAdditionToPolice = 0`.
  5. If status == PLAYER: `m_nFlags &= ~0x81` (bits 0 and 7); move speed and turn speed set to 0 (z, y, x order; integer stores).
  6. status := WRECKED; physical flags |= 0x20000000; `m_nTimeWhenBlowedUp = m_snTimeInMilliseconds`;
     `SetClumpForAllAtomicsFlag(m_pRwClump, 0x4000)`; `m_damageManager.FuckCarCompletely(false)`.
  7. If model is not 465/501: `SetBumperDamage(5, false)`, `SetBumperDamage(6, false)`, `SetDoorDamage(d, false)` for d = 0..5,
     `SpawnFlyingComponent(5, 1)`; `a = null; RwFrameForAllObjects(m_aCarNodes[5], GetCurrentAtomicObjectCB, &a)`; if `a` `a->flags (+2) = 0`.
  8. `m_nBombOnBoard = 0` (byte +0x4A8 &= 0xF8); `m_fHealth = 0`; `m_wBombTimer = 0`; `TheCamera.CamShake(0.4f, pos)`; `KillPedsInVehicle()`;
     `m_nOverrideLights = 0` (+0x4A8 &= 0xE7); bEngineOn = bLightsOn = 0 (+0x428 &= 0xAF); bSirenOrAlarm = 0 (+0x42D &= 0x7F);
     autoFlags.bTaxiLight = 0 (+0x868 &= 0xFE). If bIsAmbulanceOnDuty: clear it, `NumAmbulancesOnDuty--`. If bIsFireTruckOnDuty: clear,
     `NumFireTrucksOnDuty--`. `ChangeLawEnforcerState(false)`.
  9. `gFireManager.StartFire(this, damager, 0.8f, 1, 7000, 0)`; `RegisterCarBlownUpByPlayer(this, 0)`;
     `AddExplosion(this, damager, (model is 465 or 501) ? 12 : 7, pos, 0, 1, -1.0f, 0)` (last argument always 0).
- gta-reversed: plugin::Call (no C++).

## 0x6C7050 CHeli::ProcessControl  (catalogue: gr-unreversed/REA, port level 26)
- Convention: virtual (vtable +0x00 slot of the CHeli table at 0x871680 = ProcessControl) `__thiscall void ()`, plain `ret`. No static callers.
- Callees: CAutomobile::ProcessControl 0x6B1880; FxSystem_c::Kill 0x4AA3F0; CPad::GetPad 0x53FB70, CPad::HornJustDown 0x53FF30;
  CCullZones::PlayerNoRain 0x72DDC0 / NoPolice 0x72DD50; CWorld::FindPlayerVehicle 0x56E0D0, FindPlayerEntity 0x56E120, FindPlayerPed
  0x56E210, FindPlayerWanted 0x56E230; CPlaceable::GetAtDirection 0x50E420 (matrix +0x20) / GetTopDirection 0x41CCB0 (matrix +0x10);
  CMatrix*CVector 0x59C890; CWorld::GetIsLineOfSightClear 0x56A490; CVector::Normalise 0x59C910; CAudioEngine::SayPedless 0x5073C0 /
  ReportWeaponEvent 0x506F40 (on AudioEngine 0xB6BC90); CWeapon::FireOneInstantHitRound 0x73AF00 (cdecl(CVector* start, CVector* end,
  int damage)); CGeneral::GetRandomNumberInRange(float, float) 0x41BD90 (original); `rand`; CHeli::SendDownSwat 0x6C69C0;
  CInterestingEvents::Add 0x602590 (thiscall on g_InterestingEvents 0xC0B058, (int type, CEntity*)); CHeli::FindSwatPositionRelativeToHeli
  0x6C4760; CRopes::RegisterRope 0x556B40; Multiply3x3 0x59C790; CRopes::SetSpeedOfTopNode 0x555DF0 (cdecl(uint32 id, CVector));
  CVehicle::UpdateWinch 0x6D3B80; CVehicle::ProcessWeapons 0x6E3950.
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84, m_snPreviousTimeInMilliseconds 0xB7CB78, ms_fTimeStep; byte 0xC0B184
  (g_InterestingEvents +0x12C, bit 0 = enabled); .data 0x8D33A4 = 0.15f. Constants: 0x859CE4 -30.0f, 0x85862C 10.0f, 0x858628 100.0f,
  0x858CDC 0.001f, 0x858624 1.0f, 0x858B34 60.0f, 0x858A10 40.0f, 0x858C28 0.05f, 0x858C20 0.9f, 0x8717C4 49.0f, 0x858B38 0.02f,
  0x858B3C 3.0f, 0x858B1C 0.1f, 0x858C7C 1/32767; immediates (0, 3.5f, -1.0f), 1.0f.
- Fields (CHeli): history arrays are really two `float[6]`: X history +0x9C0..+0x9D4 and Y history +0x9D8..+0x9EC (gr's layout names
  +0x9C0 a CVector, +0x9D4 `m_pParticlesList` and declares 3-element histories at +0x9D8/+0x9E4 - wrong; +0x9D4 is history X[5]).
  +0x9F0 m_nSearchLightTimer, +0x9F4 m_vecSearchLightTarget, +0xA00 intensity, +0xA04 m_nShootTimer, +0xA08 m_nPoliceShoutTimer,
  +0xA11 m_bSearchLightEnabled, +0x988 m_nHeliFlags (bit 1), +0x580 m_pDustParticle, +0x984 m_heliDustFxTimeConst; vehicle +0x504
  m_nTimeForMinigunFiring (used as the next-shot time).
- Behaviour:
  1. `CAutomobile::ProcessControl()`. If `!bEngineOn && m_pDustParticle`: `Kill()`, `m_pDustParticle = 0`, `m_heliDustFxTimeConst = 0`.
     `if (GetPad(driver && driver->m_nPedType == 1 ? 1 : 0)->HornJustDown()) m_bSearchLightEnabled = !m_bSearchLightEnabled`.
  2. If physical flag 0x20000000 or `PlayerNoRain()`: `intensity = 0` and go to 7.
  3. Light owner (bl = light, sh = may shoot, tgt = target):
     - mission 40 and (`!FindPlayerVehicle()` or its sub-type is not 3/4): `bl = sh = 1`, `tgt = FindPlayerEntity(-1)`;
     - else mission 39 and `m_TargetEntity` and `m_nHeliFlags & 2`: `bl = 1, sh = 0, tgt = m_TargetEntity`;
     - else status PLAYER and model 497 and `m_bSearchLightEnabled`: `bl = 1, sh = 0, tgt = null`;
     - else `bl = sh = 0` (tgt is an uninitialised local, unused).
     If submerged (flag 0x100): `bl = sh = 0`. `m_bSearchLightEnabled = bl`. If `!bl` go to 7.
  4. Light position: if `tgt`: `T = tgt pos`, `V = tgt->m_vecMoveSpeed`; else `T = pos + fwd*10.0f + up*(-30.0f)`, `V = this->m_vecMoveSpeed`.
     `dt = now - m_nSearchLightTimer`; if `dt > 1000`: `k = (dt - 1001)/1000 + 1` (unsigned), repeat k times: shift both histories by one
     (`h[5..1] = h[4..0]`), `timer += 1000`, `X[0] = V.x*100 + T.x`, `Y[0] = T.y + V.y*100`; `dt -= 1000*k`.
     `t = (float)dt * 0.001f`; `target.z = T.z`; `target.x = (1-t)*X[2] + t*X[1]`; `target.y = (1-t)*Y[2] + t*Y[1]` (1-s lag).
     `d = sqrt((target.y-pos.y)^2 + (target.x-pos.x)^2)`: `intensity = d > 60 ? 0 : d < 40 ? 1.0f : 1.0f - (d - 40)*0.05f`.
     If `intensity < 0.9f` or `(T.y-target.y)^2 + (T.x-target.x)^2 > 49.0f`: `m_nShootTimer = now; +0x504 = now`.
     Else if `now > m_nPoliceShoutTimer`: `m_nPoliceShoutTimer = now + 4500 + (rand() & 0xFFF)`.
  5. If `!sh` go to 7. `w = FindPlayerPed()->m_pPlayerData->m_pWanted->m_nWantedLevel` (no null checks): delay = {0,1,2: 999999; 3: 10000;
     4: 5000; 5: 3500; 6: 2000; >6: the stale bits of a float local (0x6C750F) - keep}. If `w != 0`: `AudioEngine.SayPedless(52, 174, this,
     0, 1.0f, 0, 0, 0)` (every frame). If `NoPolice()`: `delay /= 2` (signed). If `tgt != FindPlayerPed(-1)`: `delay = 5000`.
     If `FindPlayerWanted()->flags (+0x1E) & 7`: reset both timers to now; go to 7.
     `G = (*m_matrix) * (0, 3.5f, -1.0f)`. If `now > shootTimer + delay` and `prevNow <= shootTimer + delay` and
     `!GetIsLineOfSightClear(G, T, true, false x6)`: reset both timers to now.
     If `now > shootTimer + delay` (unsigned) and `now > +0x504`: `A = (T.x + ((rand()&0xFF) - 128) * 0.02f, T.y + (same, new rand), T.z)`;
     `n = normalise(T - G)`; `FireOneInstantHitRound(&(G + n*3), &(A + n*3), 20)`; `AudioEngine.ReportWeaponEvent(145, 31, this)`;
     `+0x504 = now + (GetRandomNumberInRange(0.0f, 1.0f) < 0.15f ? 400 : 150)`.
  7. If mission 40 and `m_nNumSwatOccupants > 0`: `SendDownSwat()`; `g_InterestingEvents.Add(26, this)` (regardless of the result).
  8. For i = 0..3 with `m_aSwatState[i] != 0`: `m_aSwatState[i]--`; `RegisterRope((uint32)this + i, 8, (*m_matrix) * FindSwat...(i), false, 0, 0,
     nullptr, 20000)`; if it reached 0: `v = Multiply3x3(*m_matrix, FindSwat...(i) * 0.05f)`; `CRopes::SetSpeedOfTopNode((uint32)this + i,
     CVector(v.x, v.y, 0.0f))`.
  9. `UpdateWinch(); ProcessWeapons()`. If interesting events are enabled: `p = ts * 0.02f * 0.1f` (doubled if `sh`); if
     `rand() * (1/32767) < p`: `g_InterestingEvents.Add(21, this)`.
- gta-reversed: plugin::Call (no C++). Layout note above (history arrays) is a gr naming/layout error.

## 0x6C79A0 CHeli::UpdateHelis  (catalogue: specified/REA, port level 30)
- Convention: `__cdecl void ()` (static), plain `ret`. Caller: CGame::Process 0x53BEE0.
- Callees: CWorld::FindPlayerWanted 0x56E230 (cdecl(-1)), CWanted::NumOfHelisRequired 0x561FA0 (thiscall -> int32), CCullZones::PlayerNoRain
  0x72DDC0 (AL), CWorld::FindPlayerPed 0x56E210, CHeli::GenerateHeli 0x6C6520, CEntity::RegisterReference 0x571B70 (thiscall on the heli,
  (CEntity**)), CWorld::FindPlayerCoors 0x56E010, CWorld::Remove 0x563280, virtual slot 0 (deleting dtor, arg 1).
- Globals: CHeli::NumberOfSearchLights 0xC1C96C (W), CHeli::pHelis[2] 0xC1C964/0xC1C968 (RW), CHeli::TestForNewRandomHelisTimer 0xC1C960
  (RW, u32), CHeli::bPoliceHelisAllowed 0x8D338C, CGame::currArea 0xB72914, CWeather::OldWeatherType 0xC81320 / NewWeatherType 0xC8131C (i16),
  CWanted::UseNewsHeliInAdditionToPolice 0xB7CB8C, CTimer::m_snTimeInMilliseconds 0xB7CB84, load-state bytes of model 497 (0x8E73A4) and
  model 488 (0x8E72F0) in CStreaming::ms_aInfoForModel. Constants: 0x858F98 170.0f, immediate 100.0f.
- Behaviour:
  1. `NumberOfSearchLights = 0`; `req = FindPlayerWanted(-1)->NumOfHelisRequired()`; `n = 0`; `police = false`.
     For each slot i = 0, 1: if `pHelis[i]`: `n++`; if model == 497 and not (physical flag 0x20000000) and not bIsDrowning: `police = true`.
  2. `req = 0` if `PlayerNoRain()` or `currArea != 0`, if `!bPoliceHelisAllowed`, or if Old or New weather type == 19.
  3. `news = police`; `news = false` if either slot holds a model-488 heli. If `!UseNewsHeliInAdditionToPolice` -> `news = false`.
     Spawn gate: if `news` -> require model 488 loaded (state 1), else require model 497 loaded (state 1).
     If the gate passes and `now > TestForNewRandomHelisTimer` (unsigned): `TestForNewRandomHelisTimer = now + 15000`; if `n < req` (signed):
     `h = GenerateHeli(FindPlayerPed(-1), news)`; store in the first null slot (0 then 1) and `h->RegisterReference(&pHelis[slot])`;
     if both slots are taken the heli is not stored (it stays in the world).
  4. For each slot: if heli: if physical flag 0x20000000 or bIsDrowning -> `m_nCarMission = 42` and the slot is cleared (heli not deleted);
     else if `m_nCarMission == 42` and `fabs(|playerCoors - heliPos|) > 170.0f`: `CWorld::Remove(h); delete h;` slot cleared.
  5. `e = req`; if `pHelis[0]` and its mission != 42: if `req > 0` `e = req - 1`, else `mission = 42, m_fMinAltitude (+0x9B0) = 100.0f,
     m_fMaxAltitude (+0x9A8) = 100.0f`. If `pHelis[1]` and its mission != 42 and `e <= 0`: same retire step for slot 1.
- gta-reversed: plugin::Call (no C++).
