# CCarCtrl (POP) - full specs, part 4 (group P4)

Same conventions as CCarCtrl.md / CCarCtrl.p2.md. Vehicle CAutoPilot at +0x390 (gr names): +0x390 m_currentAddress, +0x394
m_startingRouteNode, +0x398 m_endingRouteNode, +0x3AC m_nTimeToStartMission (+0x1C), +0x3B0 m_nTimeSwitchedToRealPhysics (+0x20),
+0x3BA m_nCarMission (+0x2A, i8), +0x3BB m_nTempAction (+0x2B, i8), +0x3BC m_nTempActionTime (+0x2C, u32), +0x3DC autopilot flags byte
(+0x4C), +0x41C m_TargetEntity (+0x8C). Vehicle: +0x22 model (i16), +0x44 m_vecMoveSpeed, +0x420 dword, +0x424 recording id (i8), +0x428
flags byte (bit 5 = bIsHandbrakeOn), +0x494 m_fSteerAngle, +0x49C m_fGasPedal, +0x4A0 m_fBreakPedal. Timer: m_snTimeInMilliseconds
0xB7CB84, ms_fTimeStep 0xB7CB5C. Vector helpers: 0x40FDB0 `DotProduct(const CVector*, const CVector*)` -> ST0, 0x40FEC0 `out = v * f`
(cdecl (out, const CVector* v, float f) -> out), 0x40FE30 `out = a + b` (cdecl (out, a, b) -> out).

## 0x436A90 CCarCtrl::SteerAICarWithPhysics_OnlyMission  (catalogue: gr-unreversed, port level 21)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], float* steer [esp+8], float* gas [esp+0xC], float* brake [esp+0x10],
  bool* handbrake [esp+0x14])`, plain `ret`. Frame `sub esp,0xE4` + ebp, esi (+ ebx, edi inside the switch). Many cases are plain tail-like
  calls; the outputs are written only where stated (a case that does not mention them leaves `*steer/*gas/*brake/*handbrake` untouched).
- Callers: SteerAICarWithPhysics 0x437C20 (2 sites).
- Callees (all cdecl): see the cases; plus CWorld::FindPlayerCoors 0x56E010 (out, -1), FindPlayerVehicle 0x56E0D0 (-1, 0), FindPlayerSpeed
  0x56E090 (-1) -> CVector*, FindPlayerEntity 0x56E120 (-1), VectorSub 0x40FE60 (cdecl (out, a, b): out = a - b), CVector::Magnitude
  0x4082C0 (thiscall -> ST0), CMatrix::operator* 0x59C890 (cdecl (out, const CMatrix*, const CVector*) -> transformed point),
  CVector::Normalise 0x59C910, CVehicle::PickRandomPassenger 0x6D2A10, CPed::Say 0x5EFFE0. "HFT(e, x, y)" below =
  `SteerAICarWithPhysicsHeadingForTarget(veh, e, x, y, steer, gas, brake, handbrake)` 0x433280; "OUT5(f)" = `f(veh, steer, gas, brake,
  handbrake)`. T = veh->+0x41C (m_TargetEntity), Tp = its position; dest = (+0x3EC, +0x3F0) (autopilot m_vecDestinationCoors.x/y).
  Colmodel extents: `ms_modelInfoPtrs[model]->+0x14` (CColModel*), `+0x0C` bbox max.x, `+0x10` bbox max.y.
- Behaviour: `m = (int8)veh->+0x3BA`; if `m > 0x44` (unsigned): return. Byte table 0x437BD4 -> jump table 0x437B24:
  - 0 (NONE): `*steer = 0, *gas = 0, *handbrake = 1, *brake = 0.5f`. 10, 11 (EMERGENCYVEHICLE_STOP, STOP_FOREVER): the same.
  - 1, 2, 4, 8, 12, 15, 17, 43, 52, 54, 61, 65-68: OUT5(SteerAICarWithPhysicsFollowPath 0x434900).
  - 7, 27, 28: nothing (0x437B18).
  - 3 (RAMPLAYER_CLOSE), 0x436ABF: `P = FindPlayerCoors(-1)`; `pv = FindPlayerVehicle(-1, 0)`; if pv == NULL go to the final call with
    target NULL. `d = DotProduct(&pv.matrix->forward, &veh.matrix->forward)` (f32 local; both matrices dereferenced without NULL checks).
    a. If `(veh->+0x20 & 1)` (random seed) and `d > 0.5f [0x858B8C]`: `w = (colmax.x(self) + colmax.x(pv)) - 0.2f [0x858CC4]` (f32);
       if `seed & 2`: `P.x = w * pv.right.x + P.x`, `P.y = w * pv.right.y + P.y`; else `P.x = P.x - w * pv.right.x`, `P.y = P.y - w * pv.right.y`
       (each stored f32). `r = |pv.+0x44 - veh.+0x44| * 12.0f [0x858CCC] + 2.0f [0x858CA0]`; `dist = sqrt((V.y - P.y)^2 + (V.x - P.x)^2)` (V =
       own position, y term first); if `dist < r` and tempAction (+0x3BB) == 0: tempAction = (seed & 2) ? 7 : 8, tempActionTime = now + 250.
    b. Else: `k = (float)((uint8)seed - 128)`; `P.x = k * pv.right.x * 0.00625f [0x859050] + P.x`, `P.y = k * pv.right.y * 0.00625f + P.y`.
    c. If `d < 0.0f`: `q = d * -0.02f [0x85904C]` (f32); `P.x = q * FindPlayerSpeed(-1).x + P.x`, `P.y = q * FindPlayerSpeed(-1).y + P.y`.
    d. If `veh->+0x428 & 1` (law enforcer) and `sqrt(vx*vx + vy*vy) > 0.4f [0x858EE8]` for pv's +0x44/+0x48: `p = pv->PickRandomPassenger()`;
       if p: `p->Say(0x26, 0, 1.0f, 0, 0, 0)`.
    e. HFT(FindPlayerVehicle(-1, 0), P.x, P.y).
  - 5 (BLOCKPLAYER_CLOSE): `SteerAICarWithPhysicsTryingToBlockTarget(veh, FindPlayerEntity(-1), P.x, P.y, S.x, S.y, steer, gas, brake,
    handbrake)` (0x4335E0; P = FindPlayerCoors, S = FindPlayerSpeed; x components from the first calls, y from second calls).
    6 (BLOCKPLAYER_HANDBRAKESTOP): `SteerAICarWithPhysicsTryingToBlockTarget_Stop(veh, P.x, P.y, S.x, S.y, steer, gas, brake, handbrake)`
    (0x428990). 18 (BLOCKCAR_CLOSE): TryingToBlockTarget(veh, T, Tp.x, Tp.y, T.+0x44, T.+0x48, ...). 19 (BLOCKCAR_HANDBRAKESTOP):
    TryingToBlockTarget_Stop(veh, Tp.x, Tp.y, T.+0x44, T.+0x48, ...). (T not NULL-checked.)
  - 9, 13 (GOTOCOORDINATES_STRAIGHTLINE(_ACCURATE)): HFT(NULL, dest.x, dest.y). 24 (SLOWLY_DRIVE_TOWARDS_PLAYER_1): HFT(NULL, dest.x, dest.y).
    25 (SLOWLY_DRIVE_TOWARDS_PLAYER_2): HFT(NULL, FindPlayerCoors.x, FindPlayerCoors.y) (two calls; x from the first).
  - 14 (GOTOCOORDINATES_ASTHECROWSWIMS): `SteerAIBoatWithPhysicsHeadingForTarget(veh, dest.x, dest.y, steer, gas, brake)` (0x428BE0), then
    `*handbrake = 0`.
  - 16, 55 (RAMCAR_CLOSE, KILLPED_CLOSE): HFT(T, Tp.x, Tp.y).
  - 53 (FOLLOWCAR_CLOSE), 0x436E73: HFT(T, Tp.x, Tp.y); then `dist = |ownPos - Tp|` (f32); `f = (float)(uint8)veh->+0x3DE` (f32); return
    unless `dist < f + 10.0f [0x85862C]`. `ts = sqrt(T.vx^2 + T.vy^2) * 60.0f [0x858B34]` (x87), `os = sqrt(vx^2 + vy^2) * 60.0f` (own, f32);
    `e = dist - f`; `e = (e < 0) ? e * 5.0f [0x858C80] : e + e`; `v = (ts + e) - os`. If `v < 0`: `v *= -0.1f [0x858EF4]`, `*gas = 0`,
    `*brake = min(v, 1.0f)` (1.0f [0x858624]; `1.0 < v` -> 1.0). Else `v *= 0.05f [0x858C28]`, `*gas = min(v, 1.0f)`, `*brake = 0`.
  - 44 (APPROACHPLAYER_CLOSE), 0x436FFD: if `|FindPlayerCoors - ownPos| < 10.0f`: `*steer = 0, *gas = 0, *brake = 1.0f, *handbrake = 0`;
    return. Else `P = FindPlayerCoors(-1)`; if pv = FindPlayerVehicle(-1, 0): `R = ownPos - FindPlayerCoors` (VectorSub); if
    `(pv.matrix.forward.y + pv.matrix.forward.x) * R.x > 0.0f` (0x4370DD-0x4370F5; a sum, not a dot product - original quirk):
    `A = pv.matrix * (4, 0, 0)`, `B = pv.matrix * (-4, 0, 0)`; `P = (|own - B| > |own - A|) ? A : B` (ties/NaN pick B). HFT(FindPlayerVehicle,
    P.x, P.y).
  - 20 HELI_FLYTOCOORS: SteerAIHeliTowardsTargetCoors 0x42A630 (veh). 22 PLANE_FLYTOCOORS: SteerAIPlaneTowardsTargetCoors 0x423790.
    21: OUT5(SteerAIBoatWithPhysicsAttackingPlayer 0x428DE0). 64: OUT5(SteerAIBoatWithPhysicsCirclingPlayer 0x429090). 26: OUT5(
    SteerAICarBlockingPlayerForwardAndBack 0x422B20). 33: OUT5(SteerAICarWithPhysicsFollowPath_Racing 0x435830). 34: OUT5(
    SteerAICarWithPhysicsFollowPreRecordedPath 0x432DD0). 45, 49: OUT5(SteerAICarParkPerpendicular 0x433EA0). 46, 50: OUT5(SteerAICarParkParallel
    0x433BA0).
  - Escorts: `SteerAICarTowardsPointInEscort(veh, T, offX, offY, steer, gas, brake, handbrake)` (0x4336D0) with
    29 ESCORT_LEFT: `offX = -((colmax.x(T) + colmax.x(self)) + 2.0f)`, offY = 0; 30 ESCORT_RIGHT: offX = +(same), offY = 0; 31 ESCORT_REAR:
    offX = 0, `offY = -((colmax.y(T) + colmax.y(self)) + 7.0f [0x858F48])`; 32 ESCORT_FRONT: offY = +(same) (T's extent loaded first).
  - Planes/helis (veh only): 35, 59 GetAIPlaneToAttackPlayer 0x429780; 36 FlyAIPlaneInCertainDirection 0x423000; 37
    SteerAIPlaneToFollowEntity 0x4237F0; 57 SteerAIPlaneToCrashAndBurn 0x423880; 58 SteerAIHeliToCrashAndBurn 0x4238E0; 23, 56
    GetAIHeliToAttackPlayer 0x42F3C0; 38 GetAIHeliToFlyInDirection 0x42A730; 39, 41 SteerAIHeliToFollowEntity 0x42A750; 40
    SteerAIHeliAsPoliceHeli 0x42AAD0; 42 SteerAIHeliFlyingAwayFromPlayer 0x42ACB0; 47, 48 SteerAIHeliToLand 0x42AD30; 51
    SteerAIHeliToKeepEntityInView 0x42AEB0; 62 GetAIPlaneToDoDogFight 0x429890; 63 GetAIPlaneToDoDogFightAgainstPlayer 0x42F370.
  - 60 (DO_DRIVEBY_CLOSE), 0x4379C9: return if T == NULL. `v = (Tp.y - P.y, P.x - Tp.x, 0)` (P = own position; perpendicular), Normalise,
    `v *= 10.0f` (each f32); if `veh->+0x20 & 1` v = -v. HFT(NULL, v.x + Tp.x, v.y + Tp.y).
- Edge cases: T is dereferenced without a NULL check in missions 16, 18, 19, 29-32, 53, 55 (mission 60 checks); mission 3 with a NULL
  player vehicle still calls HFT (target NULL, player coordinates).
- gta-reversed: `plugin::Call` only. Open questions: none.

## 0x437C20 CCarCtrl::SteerAICarWithPhysics  (catalogue: gr-unreversed, port level 22)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret`. Entry `sub esp,0x34; push esi; esi = veh; nop; jmp 0x4013E0`
  (stub `mov al, [esi+0x424]; jmp 0x437C2E`). Locals (frame after `push ebx; push edi`): steer, gas, brake (f32), hb (u8) - all 0 at entry;
  BL = handbrake value written at the end.
- Callers: CAutomobile::ProcessAI 0x6B4800, CBike::ProcessAI 0x6BC930, CBmx::ProcessAI 0x6C1470, CBoat::ProcessControl 0x6F1770.
- Callees: SwitchBetweenPhysicsAndGhost 0x4222A0, CPathFind::AreNodesLoadedForArea 0x44DD10 (ThePaths), JoinCarWithRoadAccordingToMission
  0x432CB0, SteerAICarWithPhysics_OnlyMission 0x436A90 (cdecl (veh, float* steer, float* gas, float* brake, bool* handbrake)),
  FindMaxSteerAngle 0x427FE0 (cdecl (veh) -> ST0), CPlaceable::GetTopDirection 0x41CCB0 (thiscall (CVector* out) -> out), 0x40FDB0,
  0x40FEC0, 0x40FE30.
- Globals: CVehicleRecording::bUseCarAI[] 0x97D6C0 (u8 per recording).
- Behaviour:
  1. `r = (int8)veh->+0x424`; if `r >= 0` and `bUseCarAI[r] == 0`: return without writing anything (0x437C5A).
  2. `SwitchBetweenPhysicsAndGhost(veh)`; `veh->+0x420 = 0` (dword).
  3. Mission fix-up: `k = (int8)mission - 15`; if `k <= 0x28` (unsigned), byte table 0x4382E4 / jump table 0x4382D8: missions 15-19, 27-32,
     37, 52-55: if `+0x41C == NULL` mission = 0; mission 39: if `+0x41C == NULL` mission = 42; others unchanged.
  4. If `+0x3DC & 1` (needs joining the road): steer = gas = 0, brake = 0.2f, BL = 0; with P = position, if
     `AreNodesLoadedForArea(P.x - 270.0f, P.x + 270.0f, P.y - 270.0f, P.y + 270.0f)` (270.0f [0x859070], each f32): clear bit 0 and
     `JoinCarWithRoadAccordingToMission(veh)`. Go to OUT (brake read from the local).
  5. Else if `+0x3DC & 2` and mission != 11 and mission != 0: tempAction = 22, tempActionTime = now + 2000, clear bit 1, and swap nodes:
     `new.cur = old.start`, `new.start = old.cur`, `new.end = old.cur` (0x437D7B-0x437DB4).
  6. `switch (tempAction)` (`k = ta - 1`, unsigned `ja` > 0x17 -> default; table 0x438310). "Expire" = `if (now > tempActionTime)` (unsigned).
     - 1 / 24: brake = 0.2f [0x858CC4] / 1.0f [0x858624]; steer = gas = 0; BL = 0; expire: ta = 0, +0x3AC = now, +0x3B0 = now.
     - 3: `OnlyMission(veh, &steer, &gas, &brake, &hb)`; `steer = -steer`; BL = 0; `d = DotProduct(&+0x44, &matrix->forward)` (matrix +0x14 not
       NULL-checked); if `d > 0.04f [0x858CEC]`: brake = 0.5f, gas = 0 else brake = 0.0f, gas = -0.5f; expire: ta = 0, +0x3AC = now.
     - 4 / 5 / 6: BL = 1, steer = 1.0f / -1.0f / 0, gas = 0, brake = 0; expire: ta = 0.
     - 7 / 8: BL = 0, steer = 1.0f / -1.0f, gas = 1.0f, brake = 0; expire: ta = 0.
     - 9: BL = 0, steer = 0, gas = 0.5f, brake = 0; expire: ta = 0.
     - 10, 11, 20, 21: BL = 0; steer = (ta == 11 || ta == 21) ? 0.25f : -0.25f; brake = 0.001f [0x858CDC]; gas = 0; if
       `now > tempActionTime - 1250` (unsigned) steer = -steer; if `now > tempActionTime`: if ta is 20 or 21 { ta = 1; tempActionTime = now + 4000 }
       else ta = 0.
     - 13 / 14: gas = -0.75f, brake = 0, BL = 0, steer = FindMaxSteerAngle(veh) / -FindMaxSteerAngle(veh); expire: ta = 0, +0x3AC = now.
     - 19: BL = 0, gas = 0, brake = (now > tempActionTime) ? 1.0f : 0.0f; `s = veh->+0x494`; if `s > 0.0f`: `v = ts * 0.05 [f64 0x859068] + s`,
       `v = (0.5 < v) ? 0.5 : v` (f64 0x859060); else `v = s - ts * 0.05`, `v = (-0.5 > v) ? -0.5 : v` (f64 0x859058); steer = (f32)v;
       expire (same time comparison): ta = 0.
     - 22: BL = 0, steer = 0; `d = DotProduct(&+0x44, &matrix->forward)`; if `d > 0.1f [0x858B1C]`: brake = 0.5f, gas = 0 else brake = 0.0f,
       gas = -0.5f; expire: ta = 0.
     - 23: `v = ts * 0.012f [0x859054]`; steer = old `+0x494` (unchanged), gas = 1.0f, brake = 0, BL = 0;
       `veh->m_vecMoveSpeed = m_vecMoveSpeed + GetTopDirection() * v` (0x40FEC0 then 0x40FE30); expire: ta = 0.
     - 15..18: expire: ta = 0; then the default.
     - 2, 12, 0 and others (default, 0x43825D): `OnlyMission(veh, &steer, &gas, &brake, &hb)`; BL = hb.
  7. OUT: `+0x4A0 = brake` (f32), `+0x494 = steer`, `+0x428 bit 5 = BL & 1`, `+0x49C = gas`; if model == 539 (0x21B, Vortex): `+0x994 = gas`,
     `+0x988 = steer`.
- Edge cases: in case 3 the handbrake returned by OnlyMission is discarded (BL = 0). The double constants in case 19 are used with the f32
  steer angle in x87.
- gta-reversed: `plugin::Call` only. Open questions: none.
