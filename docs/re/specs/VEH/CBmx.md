# CBmx (VEH) - full specs

Group D, RE-SPEC-2. Authority: disassembly. CBmx derives from CBike (see CBike.md for the CBike/CVehicle layout and compare idioms).
CBmx fields (size 0x838): +0x814 m_fControlJump, +0x818 m_fControlPedaling (written by ProcessAI; gr's "always 0" comment is wrong),
+0x81C m_fSprintLeanAngle, +0x820 m_fCrankAngle, +0x824 m_fPedalAngleL, +0x828 m_fPedalAngleR, +0x82C m_fMidWheelDistY,
+0x830 m_fMidWheelFracY, +0x834 m_bIsFreewheeling. CBmx vtable 0x871528.

## 0x6BF9D0 CBmx::~CBmx  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall void ()`, plain `ret`; SEH frame (unwind calls CBike::~CBike). Caller: CBmx scalar deleting dtor 0x6C0570.
- Behaviour: `this->vtable = 0x871528` (CBmx vtable); `m_vehicleAudio.Terminate()` (CAEVehicleAudioEntity::Terminate 0x4FB8C0 on this+0x138);
  `CBike::~CBike()` (0x6B57A0).
- gta-reversed: `~CBmx() override` (Bmx.cpp) - the explicit audio Terminate comes from the original destructor body (the CBike/CVehicle
  destructors run afterwards). Matches.

## 0x6BFB50 CBmx::ProcessDrivingAnims  (catalogue: gr-unreversed/REA, port level 10)
- Convention / signature: virtual override, `__thiscall void (CPed* driver [esp+4], bool blend [esp+8])`, `ret 8`. ECX = CBmx*.
  `blend` is never read. After its last use as a pointer (0x6BFC5B) the `driver` stack slot ([esp+0x24] in the 0x6BFBEB..0x6C024A
  frame) is reused as a temp `S` (first the drive-by association pointer, then on the pedalling path a float); `driver` itself lives in EBP.
  Frame locals (after `push ebx; push edi`): [esp+0x10] fwdSpeed, [esp+0x14] assoc LEFT, [esp+0x18] assoc RIGHT, [esp+0x1C] assoc FWD.
- Callers: 1, CTaskSimpleCarDrive::ProcessPed 0x644470 (virtual). Callees: CPed::IsPlayer 0x5DF8F0 (x2);
  RpAnimBlendClumpGetAssociation(clump, animId) 0x4D68B0 (cdecl, x9); CPed::SetMoveState 0x5DEC00 (thiscall(eMoveState));
  CAnimManager::BlendAnimation(clump, AssocGroupId, AnimationId, float blendDelta) 0x4D4610 (cdecl, x3, always delta 4.0f = push 0x40800000);
  CAnimBlendAssociation::SetCurrentTime 0x4CEA80 (thiscall(float)); CBike::ProcessRiderAnims 0x6B7280 (cdecl(CPed*, CVehicle*, CRideAnimData*,
  tBikeHandlingData*, int16) - 5 dwords popped by caller); `_CIpow` 0x822130 (ST1 = base, ST0 = exponent -> ST0 = base^exponent).
- Anim ids: 0xC4 BIKE_LEFT(196), 0xC5 BIKE_RIGHT(197), 0xC7 BIKE_FWD(199), 0xCA/0xCB/0xCC BIKE_DRIVEBYLHS/RHS/FT(202/203/204),
  0xCE BIKE_BUNNYHOP(206), 0xCF BIKE_PEDAL(207), 0xD0 BIKE_SPRINT(208).
- CAnimBlendAssociation fields: +0x14 hierarchy (hier+0x10 m_fTotalTime), +0x18 blendAmount, +0x1C blendDelta, +0x20 currentTime, +0x24 speed,
  +0x2E flags (bit 0 = ANIMATION_IS_PLAYING; touched as word at 0x6BFDD5/0x6BFDDD/0x6C00E2/0x6C00ED and as byte at 0x6BFE78/0x6BFF23).
- Globals: CTimer::ms_fTimeStep 0xB7CB5C (R). Constants: 0x858624 = 1.0f, 0x87150C = pi/2 (1.5707964f), 0x871504 = pi (3.1415927f),
  0x871508 = 0.0f, 0x871500 = 0.0f, 0x858B50 = 0.0f, 0x859EF8 = 0.0 (f64), 0x858C80 = 5.0f, 0x8714E0 = 0.4f, 0x8714E4 = 0.7f, 0x8714EC = 0.01f,
  0x8714F0 = 3.0f, 0x8714F4 = 2.5f, 0x8714F8 = 5.0f, 0x8714FC = 2.0f, 0x858C84 = 0.25f, 0x858B8C = 0.5f, 0x859948 = -1000.0f,
  0x8594F0 = 1/(2pi) (0.15915494f), 0x858CBC = 2pi (6.2831855f), 0x871510 = 0.97f, 0x858C28 = 0.05f, 0x8714E8 = 0.95f, 0x858B1C = 0.1f,
  0x858CC8 = 0.6f; raw store 0xC1000000 = -8.0f.
- Fields of `this`: +0x14 matrix (forward at +0x10/+0x14/+0x18), +0x1C entity flags (0x20000 = bOffscreen), +0x22 m_nModelIndex (u16),
  +0x36 status byte (status = byte >> 3), +0x44 m_vecMoveSpeed, +0x384 m_pHandlingData (+0x88 = transmission+0x5C, gr name m_MaxFlatVelocity),
  +0x428 byte (0x20 bIsHandbrakeOn), +0x49C m_GasPedal, +0x4B4 m_nCurrentGear (u8), +0x614 nBikeFlags (0x80 bWheelieForCamera),
  +0x63C m_BikeHandling (+0x2C m_fWheelieAng, +0x30 m_fStoppieAng), +0x640 m_RideAnimData (+0x640 AnimGroup passed to BlendAnimation,
  +0x648 LeanAngle, +0x650 LeanFwd, +0x654 AnimLeanLeft, +0x658 AnimLeanFwd), +0x720 m_aRatioHistory[4], +0x730 m_WheelCounts[4],
  +0x7B4 m_nFixLeftHand, +0x7B5 m_nFixRightHand, +0x818 m_fControlPedaling, +0x820 m_fCrankAngle, +0x824 m_fPedalAngleL,
  +0x828 m_fPedalAngleR, +0x834 m_bIsFreewheeling. Driver: +0x18 RpClump, +0x480 m_pPlayerData.
- Behaviour:
  1. Gate (0x6BFB5B): if `(m_nFlags & 0x20000)` (bOffscreen): return if `driver == NULL` (0x6BFB66) or `!driver->IsPlayer()` (0x6BFB75).
     When not offscreen, `driver` is NOT null-checked (dereferenced at 0x6BFB89).
  2. 0x6BFB7B: `m_nFixLeftHand = 1; m_nFixRightHand = 1`.
  3. Bunny hop (0x6BFB89): `hop = GetAssoc(driver->clump, 206)`. If `hop`: `crank = (1.0f - hop.blend) * crank + pi/2[0x87150C] * hop.blend`
     (x87: `fld 1.0; fsub blend; fmul crank; fld pi/2; fmul blend; faddp`), `fst crank` and jump to step 9's tail at 0x6C0371:
     `PedalAngleL = -st0` (register value), `PedalAngleR = -crank` (reloaded). Return. (Skips everything else incl. step 8.)
  4. 0x6BFBC3: if `driver->m_pPlayerData != NULL`: `driver->SetMoveState(0 /*PEDMOVE_NONE*/)`.
     `fwdSpeed = (moveSpeed.z * fwd.z + moveSpeed.y * fwd.y) + fwd.x * moveSpeed.x` (exact x87 order 0x6BFBD9-0x6BFBFA; stored f32).
     Look-ups in this order: `pedal = 207` (EDI), `sprint = 208` (EBX), `left = 196`, `right = 197`, `fwd = 199`, `S = 202`; if S==NULL `S = 203`;
     if still NULL `S = 204` (short-circuit, 0x6BFC5E-0x6BFC90).
  5. Pedalling test (0x6BFC94): `thr = (ControlPedaling > 5.0f) ? 0.7f : 0.4f` (NaN/<= -> 0.4f). Pedalling path (0x6BFCFC) is taken iff
     all of: `|LeanAngle| < thr` (0x6BFCBD, NaN fails), `LeanFwd < thr` (0x6BFCCA: `fcomp` thr vs LeanFwd, needs thr > LeanFwd),
     `fwdSpeed > 0.01f` (0x6BFCDF), `S == NULL` (0x6BFCF6). Otherwise -> step 7 (0x6C008C).
  6. Pedalling path:
     a. Pedal speed S and sprint threshold K (0x6BFCFC): if `m_nModelIndex == 510` (MODEL_MTBIKE):
        - `m_nCurrentGear < 1` (0): `K = 2.0f`, `S = 0` (int 0 = 0.0f).
        - else: `S = (5.0f[0x8714F8] * fwdSpeed) / ((float)gear * handling[+0x88] - 0.25f)` (x87: `fld 5; fmul fwd; fild gear; fmul h88;
          fsub 0.25; fdivp`), `K = 2.0f`.
        Other models: `S = 3.0f * fwdSpeed`, `K = 2.5f`. K stays in ST0 (extended), S is stored f32.
     b. 0x6BFD67: `newAnim = false` (CL). If `ControlPedaling > 5.0f` AND `S < K` (0x6BFD7C; NaN fails) -> SPRINT:
        - if `sprint == NULL || (sprint.blend < 1.0f && sprint.blendDelta <= 0.0f)` (0x6BFD8B-0x6BFDAD; NaN skips):
          `sprint = BlendAnimation(clump, RideAnimData.AnimGroup, 208, 4.0f); newAnim = true`.
        - `sprint.flags |= 1` (0x6BFDD5). If `pedal != NULL`: `pedal.flags |= 1; pedal.speed = S; anim = pedal` (0x6BFDDD, 0x6BFF27);
          else `sprint.speed = S; anim = sprint` (0x6BFDE6). -> c.
        Else (0x6BFDF6): FREEWHEEL iff `m_GasPedal == 0.0f` (0x6BFE04, NaN/!= -> PEDAL) AND `!(ControlPedaling > 0.0f)` (0x6BFE1E) AND
        `status != 2 (STATUS_SIMPLE)` (0x6BFE2A: `(byte[+0x36] & 0xF8) != 0x10`); otherwise PEDAL.
        - FREEWHEEL (0x6BFE33): if `pedal == NULL || (pedal.blend < 1.0f && pedal.blendDelta <= 0.0f)`: `pedal = Blend(..., 207, 4.0f); newAnim = true`.
          `pedal.flags &= ~1` (byte, 0x6BFE78; anim paused). If `!(byte[+0x428] & 0x20)` (handbrake off) and any of
          `m_aRatioHistory[0..3] < 1.0f` (checked 0,1,2,3 in order, 0x6BFE89-0x6BFED3): `m_bIsFreewheeling = 1`. `anim = pedal`; speed untouched.
        - PEDAL (0x6BFEDE): same re-blend test/blend of 207 as FREEWHEEL; `pedal.flags |= 1` (byte); `pedal.speed = S`; `anim = pedal`.
     c. Crank (0x6BFF2E): if `anim == NULL` (only if BlendAnimation returned NULL): `crank = _CIpow(0.97f, ms_fTimeStep) * crank` (0x6C0016).
        Else if `newAnim`: start angle `a` = pi if `left && left.blend > 0.5f`, else 0.0f if `right && right.blend > 0.5f`, else pi/2 if
        `fwd && fwd.blend > 0.5f` (each `> 0.5f` NaN fails); if none -> the normal formula below. With `a`: if `a <= -1000.0f` (0x6BFF9C; dead)
        -> normal formula; else `t = (0.0f - a) * 0.15915494f; if (t < 0.0f) t += 1.0f` (stored to S between steps);
        `anim->SetCurrentTime(anim.hier.totalTime * t)`; then `crank = t` (0x6BFFF5 copies the FRACTION, not an angle - original oddity,
        overwritten next frame by the normal formula). Normal formula (0x6BFFFF): `crank = 0.0f - (anim.currentTime / anim.hier.totalTime) * 2pi`.
     d. 0x6C0033: if `|AnimLeanLeft| > 0.05f || |AnimLeanFwd| > 0.05f`: `AnimLeanLeft = 0.95f * AnimLeanLeft; AnimLeanFwd = 0.95f * AnimLeanFwd`.
        -> step 8.
  7. Not pedalling (0x6C008C):
     a. "Cycling anims active" = (`pedal && !(pedal.blendDelta < 0.0f) && pedal.blend > 0.0f`) || (same for `sprint`) (0x6C008C-0x6C00D2;
        `test ah,1` so NaN delta fails). If so: for each non-null of pedal, sprint: `flags &= 0xFFFE` (word), `blendDelta = -8.0f`; then
        `AnimLeanLeft *= 0.95f; AnimLeanFwd *= 0.95f` (0.95f loaded first). Else: `CBike::ProcessRiderAnims(driver, this, &m_RideAnimData,
        m_BikeHandling, 0)` (0x6C012C).
     b. 0x6C0134: `a = pedal ? pedal : sprint`; if none `crank = 0` (int store of EBX=0); else `crank = 0.0f - (a.currentTime / a.hier.totalTime) * 2pi`.
     c. 0x6C0161: first match: `left && left.blend > 0.1f` -> `crank = (1 - b) * crank + pi * b; m_bIsFreewheeling = 1`;
        `right && right.blend > 0.1f` -> `crank = (1 - b) * crank + 0.0f * b; m_bIsFreewheeling = 1`;
        `fwd && fwd.blend > 0.1f` -> `crank = (1 - b) * crank + pi/2 * b` (freewheel NOT set); else `crank = _CIpow(0.97f, ms_fTimeStep) * crank`.
        x87 order for the blends: `fld 1.0; fsub b; fmul crank; fld K; fmul b; faddp`.
     d. 0x6C0225: store crank; if `S` (drive-by assoc) != NULL: `m_nFixRightHand = 0; m_bIsFreewheeling = 1`.
  8. Wheelie flag (0x6C0241): if `driver->IsPlayer()`: `nBikeFlags &= 0x7F` (stored immediately). Then:
     - If `WheelCounts[0] <= 0 && WheelCounts[1] <= 0` (0x6C0258/0x6C0275; `> 0` or NaN goes to the stoppie test): require `fwd.z > 0.0f`
       (else done) and (`WheelCounts[2] > 0.0f` or `WheelCounts[3] > 0.0` compared with the f64 constant 0x859EF8) (else done); set
       bit 0x80 iff `0.5f * WheelieAng > (WheelieAng - fwd.z)` (0x6C02C1: `fld W; fsub fz; fld W; fmul 0.5; fcompp`; NaN -> not set).
     - Stoppie (0x6C02E5): require `WheelCounts[2] <= 0` and `WheelCounts[3] <= 0` (NaN fails), `fwd.z < 0.0f`, and (`WheelCounts[0] > 0` or
       `WheelCounts[1] > 0`); set 0x80 iff `0.6f * StoppieAng < (StoppieAng - fwd.z)` (0x6C0344).
  9. 0x6C036B: `PedalAngleL = -crank; PedalAngleR = -crank` (each reloaded from +0x820). Return.
- Oddities: `blend` unused; no null check of `driver` when on-screen; `crank = t` fraction on a fresh blend with a lean anim (0x6BFFF5);
  the `<= -1000` test is dead; in SPRINT with an existing pedal anim, the pedal (not the sprint) anim gets speed S and drives the crank; the
  bunny-hop path does not update the wheelie flag. f64 compare at 0x6C02B4 vs f32 elsewhere (same result: 0.0).
- gta-reversed: `plugin::CallMethod<0x6BFB50>` only (Bmx.cpp:108). gr's CBike::ProcessDrivingAnims (0x6BF400) gates on
  `bOffscreen && status == PLAYER`; CBmx's gate is different (`bOffscreen && (!driver || !driver->IsPlayer())`).
- Open questions: none affecting behaviour. Naming only: handling+0x88 is gr `cTransmission::m_MaxFlatVelocity` (transmission+0x5C); CBmx.md's
  ProcessAI section calls it "max velocity".

## 0x6C0560 CBmx::BlowUpCar  (catalogue: specified/REA, port level 0)
- Convention: virtual `__thiscall void (CEntity* damager [esp+4], bool hideExplosion [esp+8])`, `ret 8` (0x6C0560, the only instruction).
- Behaviour: does nothing (BMX cannot explode).
- Callers (virtual): CPlayerInfo::BlowUpRCBuggy, CBike::ProcessControl, CVehicle::ProcessDelayedExplosion, CVehicle::InflictDamage.
- gta-reversed: empty override, matches.

## 0x6C0570 CBmx::vfunc_0 (proposed name: CBmx scalar deleting destructor)  (catalogue: specified/REA, port level 17)
- Convention: vtable slot 0, `__thiscall CBmx* (uint8 flags [esp+4])`, `ret 4`, EAX = this. Callers (virtual): CCarCtrl::CreatePoliceChase,
  CreateConvoy, GenerateOneRandomCar, CSetPiece::Update.
- Behaviour: `CBmx::~CBmx(0x6BF9D0)(this)`; if `flags & 1`: `CVehicle::operator delete(this)` (0x6E2D90, cdecl, pool free); return this.
- Name: the catalogue name `vfunc_0` is the compiler's scalar deleting destructor (vtable slot 0, test of bit 0 then delete).
- gta-reversed: not hooked (the C++ `delete` expression produces it).

## 0x6C0810 CBmx::PreRender  (catalogue: gr-unreversed/REA, port level 12)
- Convention: virtual (CEntity slot 17, +0x44) `__thiscall void (CBmx* this)`, esi = this, plain `ret` (0x6C146B). SEH frame
  (`push -1; push 0x848401`), `sub esp, 0x174` + 4 pushes. Below, `L+n` = esp after the prologue (0x6C082F); EH state at L+0x18C.
  Locals: CMatrix `mat` at L+0x24 (0x48 bytes; ctor inlined at 0x6C0B28/0x6C0B2C: L+0x64 m_pAttachMatrix = 0, L+0x68 owns = 0,
  dtor 0x59ACD0 at 0x6C144E), CMatrix `rot` at L+0xB4 attached to the RwMatrix buffer at L+0x144 (0x6C0DD5..0x6C0DE3, only
  inside block F), temporary CMatrix at L+0xFC, CQuaternion at L+0xA4, `savedPos` CVector at L+0x10, colModel at L+0x88,
  colData at L+0x20 (also reused as f32 scratch).
- Callers: none direct (vtable 0x871528 slot 17). gta-reversed: `plugin::CallMethod<0x6C0810>` only (Bmx.cpp:213, VMT hook
  `.Reversed = false`), no C++ - everything below is from the bytes.
- Callees: CVehicle::PreRender 0x6D6480, CEntity::GetColModel 0x535300 (x2), virtual +0xD0 DoBurstAndSoftGroundRatios (CBike
  0x6B6950), CVehicle::AddSingleWheelParticles 0x6DE880 (thiscall, 11 args, callee cleans), CBike::CalculateLeanMatrix 0x6B7150,
  CShadows::StoreShadowForVehicle 0x70BDA0 (cdecl), Multiply3x3 0x59C790 (cdecl, `(CVector* out, CMatrix* m, CVector* v)`),
  CPhysical::GetSpeed 0x542CE0 (thiscall `(CVector* out, CVector point by value)`, returns out), CVehicle::ProcessWheelRotation
  0x6D1230 (thiscall `(int wheelState, CVector* fwd, CVector* speed, float radius)`, result in ST0), CMatrix::Attach 0x59BD10
  (`(RwMatrix*, bool owns)`), SetUnity 0x59AE70, UpdateRW 0x59BBB0, Update 0x59BB60, operator= 0x59BBC0, operator*= 0x411A80,
  operator*(CMatrix out, a, b) 0x59BE30 (cdecl, hidden out), ~CMatrix 0x59ACD0, SetTranslate 0x59AF40 (reset orientation + pos),
  SetRotateX 0x59B060 and SetRotate(x,y,z) 0x59B120 (both zero the position), RotateY 0x59B2C0 (one float arg),
  CVector::Normalise 0x59C910, CQuaternion::Set(axis*, angle) 0x59C600, CQuaternion::Get(RwMatrix*) 0x59C080, `_CIasin` 0x821E70.
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8 (indexed by model id, i16 at +0x22), `CTimer::ms_fTimeStep` 0xB7CB5C, u8
  0xC1C83C (unnamed bool; only reader in catalog/rwsets.csv is this function, no writer found -> .bss 0 in practice, but the port
  must keep both branches; proposed name "bike wheel-position debug toggle", unverified). Constants: 0x858624 1.0f, 0x8714BC 1.0f,
  0x858B50 0.0f, 0x858F34 0.75f, 0x858C98 0.8f, 0x858B8C 0.5f, 0x8595EC 0.017453292f, 0x858C1C -1.0f, 0x858B38 0.02f, 0x858CEC 0.04f,
  0x858C20 0.9f, 0x8714C0 -0.05f.
- Fields (CBike/CBmx layout of the file headers): +0x14 m_matrix, +0x22 model id, +0x36 status byte (>>3), +0x42B bit0
  bVehicleColProcessed, +0x44 m_vecMoveSpeed, +0x494 m_fSteerAngle, +0x5A4.. m_aBikeNodes[1..9] (BMX ids: 1 CHASSIS, 2 FORKS_FRONT,
  3 FORKS_REAR, 4 WHEEL_FRONT, 5 WHEEL_REAR, 6 HANDLEBARS, 7 CHAINSET, 8 PEDAL_R, 9 PEDAL_L), +0x5C8 m_bLeanMatrixCalculated, +0x644
  RideAnim.BarSteerAngle, +0x648 RideAnim.LeanAngle, +0x65C/+0x65D m_nWheelStatus[2], +0x660 m_aWheelColPoints[4], +0x710 wheel ratios[4],
  +0x720 ratio history[4], +0x730 m_WheelCounts[4], +0x744 skidmark type[2], +0x74C bloody[2], +0x74E more-skidmarks[2], +0x750
  pitch (rotation) [2], +0x758 angular velocity [2], +0x760 suspension heights[2] `h`, +0x768 orig heights[2] `oh`, +0x770 suspension
  length[4] `spring`, +0x780 line length[4], +0x798 swing arm length, +0x79C fork Y offset, +0x7A0 fork Z offset, +0x7A4 steer angle
  tan, +0x80C m_WheelStates[2], +0x81C m_fSprintLeanAngle, +0x820 m_fCrankAngle, +0x824 / +0x828 pedal angles, +0x82C
  m_fMidWheelDistY, +0x830 m_fMidWheelFracY. Model info (CVehicleModelInfo*, `mi`): +0x40 m_fWheelSizeFront, +0x44 m_fWheelSizeRear,
  +0x58 m_fBikeSteerAngle (degrees). Col data `cd = GetColModel()->m_pColData` (+0x2C); `cd->m_pLines` (+0x10), CColLine stride
  0x20: line[i].start.y at +0x20*i+4, start.z at +0x20*i+8. Col model bbox.min.z at colModel+8.
- Compare idioms as in CBike.md header.

### A. 0x6C0810-0x6C095C: base pre-render + suspension heights
1. `CVehicle::PreRender()` (0x6C0831); `colModel = GetColModel()` (0x6C0838) -> L+0x88; `cd = colModel->m_pColData` -> ebp and L+0x20.
2. If byte +0x42B bit 0 is clear: skip to B (0x6C0852). Else virtual +0xD0 `DoBurstAndSoftGroundRatios()` (0x6C085C), then front
   (wheel 0) then rear (wheel 1), identical code:
   - front: `r = 1.0f - spring[0]/lineLen[0]` (fld +0x770, fdiv +0x780, fsubr 1.0 [0x858624]); `m = (ratio[1] < ratio[0]) ? ratio[1]
     : ratio[0]` (0x6C0874: fld +0x714, fcomp +0x710, `test ah,5; jp` -> ratio[0]; NaN -> ratio[0]); `c = f32((m - r) / (1.0f - r))`
     (fsub st1; fld 1.0; fsub st2; fdivp; fstp L+0x1C); `p = line[0].start.z`; if `c > 0.0f` (0x6C08B9 `test ah,0x41; jne` skips on
     <= / NaN): `p = p - c*spring[0]`; `h[0] = (p - h[0]) * 0.75f + h[0]` (0x858F34; stored f32 at +0x760).
   - rear (0x6C08E2): same with spring[2] +0x778, lineLen[2] +0x788, `m = (ratio[3] < ratio[2]) ? ratio[3] : ratio[2]` (+0x71C vs
     +0x718), `p = line[2].start.z` (lines+0x48), result `h[1]` (+0x764).
   - Unlike CAutomobile::PreRender there is no "compressed and p > h" snap branch: always the 0.75 blend.

### B. 0x6C0962-0x6C0B0C: wheel particles (status 0..3 only)
1. `status = byte[+0x36] >> 3`; if status not in {3, 0, 1, 2} (0x6C0968..0x6C0976) skip to C.
2. `speed = sqrt((vx*vx + vy*vy) + vz*vz)` of +0x44/+0x48/+0x4C (0x6C097C..0x6C099D) -> f32 L+0x6C.
3. Loop `i = 0, 1` (edi, 0x6C09A3..0x6C0B06, ascending):
   - Contact column `c`: i == 0: `c = (hist[0] < 1.0f || NaN) ? 0 : ((hist[1] < 1.0f) ? 1 : 0)` (0x6C09A7..0x6C09CF; first compare
     `test ah,1; jne` takes < and NaN, second `test ah,5; jp`); i == 1: `c = (hist[3] < 1.0f || NaN) ? 3 : ((hist[2] < 1.0f) ? 2 : 3)`
     (0x6C09D6..0x6C0A01). hist = +0x720, constant 1.0f [0x8714BC].
   - `flags = 4` if i == 0 or `m_WheelStates[1]` (+0x810) == 3, else 0 (0x6C0A06..0x6C0A15).
   - `k = sin(LeanAngle) * GetColModel()->bbox.min.z * 0.8f` (0x6C0A1C..0x6C0A31; fsin of +0x648, fmul [eax+8], fmul 0x858C98).
   - `pos = cp[c].point + right*k` with right = m_matrix+0..8, computed as x: `k*right.x + p.x`, y: `p.y + k*right.y`, z: `p.z +
     k*right.z` (0x6C0A40..0x6C0A8A; each stored f32 to L+0x10..0x18).
   - `flags += 1` if bloody[i] (+0x74C+i) != 0; `flags += 2` if moreSkid[i] (+0x74E+i) != 0.
   - `side = (LeanAngle > 0.0f) ? -1.0f : 1.0f` (0x6C0AA1: `test ah,0x41; je` keeps -1.0 when lean > 0; NaN -> 1.0).
   - `AddSingleWheelParticles(m_WheelStates[i] (+0x80C+4i), wheelStatus[i] (u8 +0x65C+i zero-extended), hist[c] (+0x720+4c),
     speed, &cp[c] (+0x660+0x2C*c), &pos, side, i, skidType[i] (+0x744+4i), &bloody[i], flags)` (0x6C0AFD; arguments in C order).
4. ebp = colData again (0x6C0B0C).

### C. 0x6C0B10-0x6C0BB5: lean matrix, shadow, steered wheel direction
1. `byte +0x5C8 = 0`; `CalculateLeanMatrix()`; `CShadows::StoreShadowForVehicle(this, 1)` (cdecl, args cleaned at 0x6C0BB2).
2. `mat` ctor (see header); `mi = ms_modelInfoPtrs[i16 +0x22]` (edi); EH state = 0.
3. `v = (-sin(steer), cos(steer), 0.0f)` with steer = +0x494, fsin then fcos of the f32 field (0x6C0B30..0x6C0B75), stored at L+0x70.
   `wheelFwd (L+0x98) = Multiply3x3(m_matrix, &v)`. `fwdCopy (L+0x8C..0x94) = m_matrix->forward` (+0x10..+0x18, raw dword copy).

### D. 0x6C0B84-0x6C0C9B: front wheel spin
- Condition: `counts[0] (+0x730) > 0.0f || counts[1] (+0x734) > 0.0f` (first `test ah,0x41; je` -> do it, second `jne` -> skip to E).
- `point = (0.0f, f32((line[1].start.y + line[0].start.y) * 0.5f), f32(((line[0].start.z - m*spring[0]) - mi->wheelSizeFront*0.5f)))`
  with `m = (hist[0] < hist[1]) ? hist[0] : hist[1]` (0x6C0BEC: fld +0x720, fcomp +0x724, jp -> hist[1]); order: fld m, fmul +0x770,
  fsubr line0.z, fld [mi+0x40], fmul 0.5, fsubp.
- `spd (L+0x7C) = GetSpeed(point)`. `w = ProcessWheelRotation(0 /*constant: ebx = 0, NOT m_WheelStates[0]*/, &wheelFwd, &spd,
  f32(mi->wheelSizeFront * 0.5f))` (0x6C0C84). `angVel[0] (+0x758) = w` (fst), then `pitch[0] (+0x750) = w * ms_fTimeStep +
  pitch[0]` (fmul then fadd, f32 store).

### E. 0x6C0CA1-0x6C0D9B: rear wheel spin
- Condition: `counts[2] (+0x738) > 0.0f || counts[3] (+0x73C) > 0.0f`.
- `point = (0.0f, (line[3].start.y + line[2].start.y) * 0.5f, ((line[2].start.z - m*spring[2] (+0x778)) - mi->wheelSizeFront*0.5f))`
  with `m = (hist[2] < hist[3]) ? hist[2] : hist[3]` (+0x728 vs +0x72C). NOTE: the z uses `mi+0x40` (front wheel size) at 0x6C0D1D,
  while the radius below uses `mi+0x44` (rear size). Keep.
- `spd = GetSpeed(point)`; `w = ProcessWheelRotation(m_WheelStates[1] (+0x810), &fwdCopy (L+0x8C, the vehicle forward, not the
  steered vector), &spd, f32(mi->wheelSizeRear (+0x44) * 0.5f))`; `angVel[1] (+0x75C) = w`; `pitch[1] (+0x754) = w*timeStep + pitch[1]`.

Frame updates below all follow the pattern `mat.Attach(&node->modelling (node+0x10), false)`; `savedPos = mat.pos` (L+0x54..0x5C
-> L+0x10..0x18); <rotate, which zeroes mat.pos>; `mat.pos.x/y/z = mat.pos.c + savedPos.c` (three fld/fadd/fstp, x then y then z);
`mat.UpdateRW()`. "Rotate" lists only the middle step.

### F. 0x6C0DA1-0x6C0EF6: front forks (node 2, +0x5A8), skipped if null
1. Attach + savedPos. `rot` ctor on RwMatrix buffer L+0x144 (owns = 0), EH state 1; `rot.SetUnity(); rot.UpdateRW()`.
2. `a = mi->m_fBikeSteerAngle` (degrees); `axis = (0.0f, sin(a*0.017453292f), -cos(a*0.017453292f))` (0x6C0E0A..0x6C0E3C: fld a,
   fmul 0x8595EC, fsin; fld a, fmul, fcos, fchs) at L+0x70; `axis.Normalise()`.
3. `q (L+0xA4).Set(&axis, -BarSteerAngle)` (fld +0x644, fchs); `q.Get(&rwBuf)`; `rot.Update()` (reads the RwMatrix back).
4. `mat.SetUnity()`; `tmp (L+0xFC) = operator*(mat, rot)`; `mat = tmp` (EH 2); `~tmp` (EH 1). Then pos += savedPos, UpdateRW.
5. Handlebars (node 6, +0x5B8), skipped if null (0x6C0F03): Attach + savedPos; status = byte +0x36 >> 3; if status != 4 && != 5:
   `mat.SetTranslate(savedPos)` (0x6C0F44; orientation reset to identity, pos = savedPos, no add) and UpdateRW; else (ABANDONED /
   WRECKED): `mat.SetUnity(); mat *= rot` (0x411A80), pos += savedPos, UpdateRW. (So the bars get the steering quaternion only for
   statuses 4/5 - presumably because the bars frame is a child of the forks; keep literally.)
6. `~rot` (EH 0) at 0x6C0FA1. Note: `rot` exists only when node 2 exists, so the handlebars are not updated at all without forks.

### G. 0x6C0FA6-0x6C1037: rear forks / swing arm (node 3, +0x5AC), skipped if null
- `a = (h[1] - oh[1]) / swingArmLength` (+0x764 - +0x76C, fdiv +0x798), `fst` L+0x20, then `_CIasin(a)` on the unrounded ST0,
  `fmul -1.0f` [0x858C1C], stored f32 L+0x20. Attach + savedPos; `mat.SetRotate(-asin(a), 0.0f, 0.0f)`; pos += savedPos; UpdateRW.

### H. 0x6C103C-0x6C1114: front wheel (node 4, +0x5B0) - NOT null-checked (Attach on node+0x10 even if node is null)
1. Attach + savedPos. If u8 [0xC1C83C] != 0: `savedPos.z = h[0] - forkZOffset` (+0x760 - +0x7A0); `savedPos.y = (((line[1].start.y +
   line[0].start.y) * 0.5f) - forkYOffset (+0x79C)) - (h[0] - oh[0]) * steerTan (+0x7A4)` (0x6C1084..0x6C10AA, final fsubp).
2. If wheelStatus[0] (u8 +0x65C) == 1 (burst): `mat.SetRotate(pitch[0], 0.0f, f32(sin(pitch[0]) * 0.02f))` (0x858B38), else
   `mat.SetRotateX(pitch[0])`. pos += savedPos; UpdateRW.

### I. 0x6C1119-0x6C11C6: rear wheel (node 5, +0x5B4) - NOT null-checked
1. Attach + savedPos. If [0xC1C83C] != 0 && node 3 (+0x5AC) == null: `savedPos.z = h[1]` (raw copy of +0x764).
2. If wheelStatus[1] (+0x65D) == 1: `SetRotate(pitch[1], 0.0f, f32(sin(pitch[1]) * 0.04f))` (0x858CEC), else `SetRotateX(pitch[1])`.
   pos += savedPos; UpdateRW.

### J. 0x6C11CB-0x6C12ED: chassis (node 1, +0x5A4), skipped if null
1. `zOff (L+0x1C) = 0.0f`, `pitch (L+0x6C) = 0.0f`. If [0xC1C83C] == 0 (0x6C11F1): `d0 = h[0] - oh[0]`, `d1 = h[1] - oh[1]` (both
   kept in x87 registers, unrounded); `zOff = f32((1.0f - fracY) * d0 + d1 * fracY)` (fracY = +0x830; fld 1.0, fsub, fmul st2, then
   `fld d1; fmul fracY; faddp`); `pitch = f32(atan2(d0 - d1, distY))` (fpatan with st1 = d0 - d1, st0 = +0x82C).
2. Attach + savedPos (x, y, z). Then `savedPos.z = f32((1.0f - cos(Lean)) * colModel->bbox.min.z * 0.9f + zOff)` (0x6C124B..0x6C128C;
   fcos of +0x648, fld 1.0, fsub st1, fmul [colModel+8] (colModel from L+0x88), fmul 0.9f [0x858C20], fadd zOff) - this REPLACES the
   frame's own z, it is not added to it.
3. `mat.SetRotateX(f32(fabs(Lean) * -0.05f + pitch))` (0x8714C0); `mat.RotateY(f32(sprintLean (+0x81C) + Lean (+0x648)))`. pos +=
   savedPos; UpdateRW.

### K. 0x6C12F2-0x6C143A: chainset and pedals, each skipped if null
- Node 7 (+0x5BC): `SetRotate(m_fCrankAngle (+0x820), 0, 0)`; node 8 (+0x5C0, BMX_PEDAL_R): `SetRotate([+0x824], 0, 0)`; node 9
  (+0x5C4, BMX_PEDAL_L): `SetRotate([+0x828], 0, 0)`; each with Attach, savedPos, pos += savedPos, UpdateRW.

### L. 0x6C143F-0x6C146B: EH state -1, `~mat` (0x59ACD0), restore fs:[0], return.

- Oddities / original behaviour to keep: front ProcessWheelRotation always gets wheel state 0 (0x6C0C81 `push ebx`, ebx = 0) while the
  rear passes m_WheelStates[1]; rear contact point uses the FRONT wheel size (0x6C0D1D); nodes 4 and 5 are not null-checked; the
  chassis z is overwritten (J.2); handlebars rotate only for status 4/5 and only when node 2 exists; the global 0xC1C83C has no writer.
- Naming: the BMX node 8 is PEDAL_R but reads +0x824 (named m_fPedalAngleL in the CBmx header) and node 9 PEDAL_L reads +0x828
  (m_fPedalAngleR). Either the field names or gr's node names are swapped; behaviour is as written (node 8 <- +0x824, node 9 <- +0x828).
- Open questions: 0x6C1060/0x6C113D/0x6C11D9 - meaning/writer of u8 0xC1C83C (treated as a plain global bool).

## 0x6C1470 CBmx::ProcessAI  (catalogue: gr-unreversed/REA, port level 2)
- Convention: virtual (vtable +0x108) `__thiscall bool (uint32& extraHandlingFlags [esp+4])`, `ret 4`, always returns AL = 1.
  The argument slot ([esp+0x58] in-frame) is overwritten with float/int temps on some paths after its last use as a pointer.
  Caller: CBike::ProcessControl 0x6B9250 (virtual).
- Extent: the catalogue extent is incomplete. The real body is 0x6C1470-0x6C200A plus a 2-instruction chunk at 0x401797
  (`mov eax,[esi+0x460]; jmp 0x6C1A04`). The switch cases at 0x6C14B2-0x6C19FE and 0x6C1B65-0x6C1ED7 are part of this function
  (disassembled with `dis.py 0x6c14b2-0x6c19fe` / `0x6c1b65-0x6c1ed7`).
- Callees: CPed::IsPlayer 0x5DF8F0; virtual ProcessControlInputs (vtable +0x60); CPlayerPed::GetPadFromPlayer 0x609560;
  CVector::Magnitude 0x4082C0 (thiscall, ST0); `CVector operator+` 0x40FE30 (cdecl(out, a, b)); `operator*(float, CVector)` 0x40FE90
  (cdecl(out, float s, const CVector* v)); CPhysical::ApplyTurnForce 0x542A50 (thiscall(CVector force, CVector point), callee pops 0x18);
  CPhysical::ApplyMoveForce 0x5429F0; CEntity::PruneReferences 0x571A90; CVehicle::DoDriveByShootings 0x741FD0;
  CBike::DoSoftGroundResistance 0x6B6D40; DotProduct 0x40FDB0 (cdecl(const CVector*, const CVector*) -> ST0);
  CPad::GetSteeringLeftRight 0x53FB80 / GetAccelerate 0x5403F0 / GetAccelerateJustDown 0x540440 / CarGunJustDown 0x53FFE0;
  CBmx::ProcessBunnyHop 0x6C0590; RpAnimBlendClumpGetAssociation(clump, animId) 0x4D68B0; CPlayerPed::ControlButtonSprint 0x60A610,
  GetButtonSprintResults 0x60A820 (thiscall(eSprintType) -> ST0), HandleSprintEnergy 0x60A550 (thiscall(bool, float));
  `max(float a, float b)` 0x420800 (cdecl, returns `a > b ? a : b`, NaN -> b); CStats::UpdateStatsWhenCycling 0x55C780 (cdecl(bool, CBmx*));
  CVehicle::ActivateBomb 0x6D24F0; CCarAI::UpdateCarAI 0x41DA30, CCarCtrl::UpdateCarOnRails 0x436540, CCarCtrl::SteerAICarWithPhysics
  0x437C20 (cdecl(CVehicle*)); CPhysical::ProcessControl 0x5485E0 (non-virtual call); cTransmission::CalculateGearForSimpleCar 0x6D0530
  (thiscall on handling+0x2C, (float speed, uint8& gear)); CVehicle::ProcessWheelRotation 0x6D1230 (thiscall(tWheelState, const CVector& fwd,
  const CVector& moveSpeed, float radius) -> ST0); CBike::PlayHornIfNecessary 0x6B7130; CBike::ReduceHornCounter 0x6B5F90;
  CStats::GetFatAndMuscleModifier 0x559AF0 (cdecl(int) -> ST0).
- Constants: 0x858B50 = 0.0f, 0x858B1C = 0.1f, 0x858CB0 = 0.7f, 0x858C24 = 0.3f, 0x858624 = 1.0f, 0x858B8C = 0.5f, 0x8714C8 = 0.04f,
  0x858B88 = 0.0078125f, 0x8714C4 = 0.002f, 0x8714D4 = 1.2f, 0x859A3C = 1/255 (0.003921569f), 0x8714D0 = 0.4f, 0x858B38 = 0.02f,
  0x8714D8 = 0.02f, 0x8714DC = 0.01f, 0x858C58 = 0.01f, 0x858C80 = 5.0f, 0x858CE8 = 1.5f, 0x858FB0 = 2.4f, 0x858CA0 = 2.0f,
  0x8595F0 = 1.25f, 0x8714CC = 0.3f, GAME_GRAVITY 0x863984 = 0.008f; raw stores 0x409CCCCD = 4.9f, 0x3F800000 = 1.0f, 0x3D4CCCCD = 0.05f.
- Fields: +0x36 status (byte >> 3), +0x3C8 m_autoPilot.m_speed, +0x3DB m_autoPilot.m_nCarCtrlFlags, +0x428 byte0 of upper vehicle flags
  (0x20 bIsHandbrakeOn), +0x42A (0x08 bIsBeingCarJacked), +0x42B, +0x42F (0x02 = lower-flags bit 25, gr bUseCarCheats), +0x460 m_pDriver,
  +0x464 m_apPassengers[0], +0x494 m_fSteerAngle, +0x49C m_GasPedal, +0x4A0 m_BrakePedal, +0x4B4 m_nCurrentGear, +0x514 m_HornCounter (u32;
  ReduceHornCounter 0x6B5F90 confirms the offset), +0x614 nBikeFlags (0x08 bGettingPickedUp, 0x10 bOnSideStand, 0x20 bPlayerBoost,
  0x80 bWheelieForCamera), +0x63C m_BikeHandling (+4 LeanFwdCOM, +8 LeanFwdForce, +0xC LeanBakCOM, +0x10 LeanBakForce), +0x650
  m_RideAnimData.LeanFwd, +0x654 AnimLeanLeft, +0x658 AnimLeanFwd, +0x710 ratios[4], +0x750/+0x754 wheel rotation, +0x804..+0x806
  contact/drive wheel counts, +0x818 m_fControlPedaling (gr comment "always 0.0f" is wrong: written here), +0x834 m_bIsFreewheeling;
  CPhysical +0x50 turn speed, +0x8C mass, +0x90 turn mass, +0xA4 m_vecCentreOfMass (y at +0xA8); handling +0x18 COM.y, +0x88
  (transmission +0x5C, the max velocity). Driver: +0x598 m_nPedType, +0x480 m_pPlayerData (+0x18 m_fTimeCanRun, +0x1C m_fMoveSpeed).
- Behaviour:
  0. `m_autoPilot.m_nCarCtrlFlags &= 0xFC` (clear bHonkAtCar|bHonkAtPed); `m_bIsFreewheeling = 0`; `switch (status)` via table 0x6C200C
     (status > 9 and 6, 7, 8 -> default). Every case ends in the common tail T (step 7).
  1. STATUS_PLAYER (0, 0x6C14B2): `*flags += 2`; `nBikeFlags &= ~0x08`. If no driver -> T. If `!driver->IsPlayer()` -> T.
     `ProcessControlInputs((uint8)driver->m_nPedType)`; `pad = driver->GetPadFromPlayer()`.
     a. Lean (0x6C14FC): if `LeanFwd < 0.0f`:
        `COM.y = bh->LeanBakCOM * LeanFwd + handling->COM.y`; apply only if `(m_BrakePedal == 0.0f && !bIsHandbrakeOn) ||
        m_nNoOfContactWheels == 0` (brake `!=`/NaN counts as pressed; 0x6C153B-0x6C155F);
        `m = |moveSpeed|; if (m > 0.1f) m = 0.1f` (Magnitude is called a second time on the else path);
        `g = (model == 468 /*0x1D4*/) ? (m_GasPedal * 0.7f + 0.3f) : ((m_GasPedal + 1.0f) * 0.5f)`;
        `k = (((g * bh->LeanBakForce) * m_fTurnMass) * LeanFwd) * m` stored f32.
        else (`LeanFwd >= 0` or NaN, 0x6C161F): `COM.y = bh->LeanFwdCOM * LeanFwd + handling->COM.y`; apply only if `m_BrakePedal < 0.0f ||
        m_nNoOfContactWheels == 0`; `m` as above; `k = ((bh->LeanFwdForce * m_fTurnMass) * m) * LeanFwd`.
        Apply: `p = m_vecCentreOfMass + fwd`; `F = -(ms_fTimeStep * k) * up`; `ApplyTurnForce(F, p)`.
     b. `PruneReferences()`; if status == 0: `DoDriveByShootings()`. `DoSoftGroundResistance(flags)`.
     c. Air control (0x6C1724): only if all four `ratio[i]` are exactly 1.0f (raw dword compare with 0x3F800000):
        `d = DotProduct(turnSpeed, up)`; apply if `(d < 0.04f && (float)GetSteeringLeftRight() < 0.0f)` or (else) `(-0.04f < d &&
        (float)GetSteeringLeftRight() > 0.0f)`. Apply: `s = (float)GetSteeringLeftRight() /*3rd call*/ * 0.0078125f * m_fTurnMass *
        ms_fTimeStep * 0.002f` (left to right); `ApplyTurnForce(s * right, fwd)` (right = matrix +0x00, point = matrix forward).
     d. `ProcessBunnyHop()`; `sa = GetAssociation(driver clump, 208 ANIM_ID_BIKE_SPRINT)`.
        If `driver->ControlButtonSprint(SPRINT_BMX) > 1.2f` or (`GetButtonSprintResults(SPRINT_BMX) > 1.0f` and `sa` and `sa->blend > 0.5f`)
        (0x6C1872-0x6C18B0): `nBikeFlags |= 0x20`; `m_fControlPedaling = driver->m_pPlayerData->m_fMoveSpeed` (raw copy).
        Else: `HandleSprintEnergy(false, max(0.5f, 1.0f - (float)GetAccelerate() * (1/255) * 0.5f))`;
        if `GetButtonSprintResults(SPRINT_BMX) > 0.0f`: `m_fControlPedaling = 4.9f`; if `m_GasPedal == 0.0f && m_BrakePedal == 0.0f`
        then `m_GasPedal = 1.0f`.
        else if `m_pPlayerData->m_fTimeCanRun < 0.0f`: `m_fControlPedaling = GetAccelerateJustDown() ? 4.9f :
        max(0.0f, m_fControlPedaling - 0.4f)`; else `m_fControlPedaling = 0`.
     e. `UpdateStatsWhenCycling((nBikeFlags >> 5) & 1, this)`; if `pad->CarGunJustDown()` (AX != 0) `ActivateBomb()`. -> T.
  2. STATUS_PLAYER_PLAYBACK_FROM_BUFFER (1): `*flags += 2` -> T.
  3. STATUS_SIMPLE (2, 0x6C1B71): `UpdateCarAI(this); CPhysical::ProcessControl(); UpdateCarOnRails(this)`;
     `m_NumDriveWheelsOnGroundLastFrame = m_NumDriveWheelsOnGround` (old value), `m_nNoOfContactWheels = 2`, `m_NumDriveWheelsOnGround = 2`;
     `handling->m_transmissionData.CalculateGearForSimpleCar(m_autoPilot.m_speed * 0.02f, m_nCurrentGear)`;
     `rot[0] = ProcessWheelRotation(0, fwd, moveSpeed, wsF * 0.5f) + rot[0]`; `rot[1] = ProcessWheelRotation(0, fwd, moveSpeed, wsR * 0.5f) + rot[1]`;
     `PlayHornIfNecessary(); ReduceHornCounter()`; byte +0x42B &= 0xDE (clears upper-flag bits 24 bVehicleColProcessed and
     29 bAudioChangingGear); `nBikeFlags &= 0x7F`; `m_fControlPedaling = 0` -> T. (No `*flags` change.)
  4. STATUS_PHYSICS (3, 0x6C1C5A): `UpdateCarAI(this); SteerAICarWithPhysics(this); PlayHornIfNecessary(); *flags += 2;
     nBikeFlags &= 0x7F`; if bIsBeingCarJacked: `bIsHandbrakeOn = 1, m_GasPedal = 0, m_BrakePedal = 1.0f`; else `nBikeFlags &= ~0x08`.
     If `m_fControlPedaling > 0.0f`: `t = (m_fControlPedaling > 5.0f) ? ms_fTimeStep * 0.02f : ms_fTimeStep * 0.01f` and in the second case
     `t = t + t` if byte+0x42F & 2; `m_fControlPedaling -= t`; `if (< 0.0f) = 0` -> T.
  5. STATUS_ABANDONED (4, 0x6C1D33): `m_BrakePedal = 0`; `v2 = (ms.x^2 + ms.y^2) + ms.z^2`; bIsHandbrakeOn = `(v2 < 0.01f) ||
     (nBikeFlags & 0x10)`; `m_GasPedal = 0; m_HornCounter = 0`; if `m_pDriver || m_apPassengers[0] || bIsBeingCarJacked`:
     `if (!(nBikeFlags & 0x10)) *flags += 2`; then `nBikeFlags &= 0x7F; AnimLeanLeft = AnimLeanFwd = 0; m_fControlPedaling = 0`;
     if bIsBeingCarJacked: `bIsHandbrakeOn = 1, m_GasPedal = 0, m_BrakePedal = 1.0f` -> T.
  6. STATUS_WRECKED (5, 0x6C1E87): `bIsHandbrakeOn = 1; nBikeFlags &= 0x7F; m_BrakePedal = 0.05f; m_fSteerAngle = 0; m_GasPedal = 0;
     m_HornCounter = 0; m_fControlPedaling = 0; AnimLeanLeft = AnimLeanFwd = 0` -> T (no flags change).
     STATUS_FORCED_STOP (9, 0x6C1E0E): `if (v2 < 0.01f) { m_BrakePedal = 1.0f; handbrake = 1 } else { m_BrakePedal = 0; handbrake = 0 }`;
     `m_fSteerAngle = 0; m_GasPedal = 0; m_HornCounter = 0; *flags += 2; nBikeFlags &= 0x7F; m_fControlPedaling = 0` -> T.
  7. Tail T (0x6C19FE -> 0x401797 -> 0x6C1A04): if `m_pDriver`:
     `a = GetAssoc(199 BIKE_FWD)`; if `!a || a->m_fBlendAmount (+0x18) < 0.5f` then `a = GetAssoc(198 BIKE_BACK)`;
     `b = GetAssoc(202)`; if `!b || b.blend < 0.5f` `b = GetAssoc(203)`; if `!b || b.blend < 0.5f` `b = GetAssoc(204)`.
     If `(a && a.blend > 0.5f) || (b && b.blend > 0.5f)` (strict, 0x6C1ABF-0x6C1AE9): `m_GasPedal = 0`; if bIsHandbrakeOn return true;
     if any `ratio[0..3] < 1.0f` set `m_bIsFreewheeling = 1`; return true.
     Otherwise (no driver or neither anim active) (0x6C1ED7): if `m_fControlPedaling > 5.0f` and (`ratio[2] < 1.0f` or `ratio[3] < 1.0f`):
     `s = 2.4f - (((ms.z*fwd.z + ms.y*fwd.y) + ms.x*fwd.x) / handling[+0x88]) * 1.5f`; clamp: `if (s > 2.0f) s = 2.0f; else if (s < 0.0f) s = 0.0f`;
     if status == PLAYER `s = GetFatAndMuscleModifier(5) * s`, else if byte+0x42F & 2 `s = s * 1.25f`;
     `f = ((ms_fTimeStep * m_fMass) * 0.3f) * s * 0.008f`; `ApplyMoveForce(f * fwd)`. Return true.
- Edge cases: `*flags` is incremented with `add`, not OR. The forward-push in T happens even for a riderless BMX if
  m_fControlPedaling > 5 (it decays only in the PHYSICS case or is zeroed by the other cases).
- gta-reversed: plugin::Call (no C++).
