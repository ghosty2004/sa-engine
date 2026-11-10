# CCarCtrl (POP) - full specs, part 2 (group P2)

Continuation of CCarCtrl.md (group P1); same conventions. Entity position = `m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &placement.pos (+0x04)`
("P" / "H" / "T" below). ThePaths 0x96F050: m_pPathNodes[] 0x96F854 (CPathNode stride 0x1C: +0x08/+0x0A/+0x0C pos i16 * 0.125f, +0x10 m_wBaseLinkId
i16, +0x18 flags: bits 0-3 m_nNumLinks, bit 4 m_onDeadEnd, bit 5 m_isSwitchedOff, bit 7 m_bWaterNode, bit 10 m_bDontWander, bits 20-23
m_nBehaviourType), m_pNaviNodes[] 0x96F974 (CCarPathLink stride 0xE: +0x00/+0x02 pos i16/8, +0x04/+0x06 attachedTo node, +0x08/+0x09 dir i8 *0.01,
+0x0B bits 0-2 / 3-5 lane counts), m_pNodeLinks[] 0x96FA94 (CNodeAddress), m_pNaviLinks[] 0x96FDF4 (u16 link address: area = v >> 10,
index = v & 0x3FF), m_anNumVehicleNodes[] 0x970114. Vehicle CAutoPilot at +0x390 (gr names; +0x390 m_currentAddress = current node, +0x394
m_startingRouteNode = NEXT node, +0x398 m_endingRouteNode = PREVIOUS node). Status = `+0x36 >> 3` (2 SIMPLE, 3 PHYSICS), type = `+0x36 & 7`.
x87 compare idioms: `fcomp; test ah,5; jp X` jumps when NOT (st0 < src) (NaN jumps); `test ah,0x41; jne X` jumps when st0 <= src or NaN;
`test ah,0x41; je X` jumps when st0 > src; `test ah,0x44; jnp X` jumps when equal. `_rand` 0x821B1E, `_ftol` 0x821B40 (truncation),
`floor` 0x8219F0, `srand` 0x821B11. gta-reversed has only `plugin::Call` stubs for all functions in this file (CarCtrl.cpp, `.Reversed = false`).

## 0x429780 CCarCtrl::GetAIPlaneToAttackPlayer  (catalogue: gr-unreversed, port level 18)
- Convention / signature: `__cdecl void (CPlane* plane [esp+4])`, plain `ret`. Frame `sub esp,0x18` + esi; S = esp after `push esi`:
  tz [S+0xC], speed*50 temps [S+4..0xC], FindPlayerCoors buffer [S+0x10].
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1).
- Callees: CWorld::FindPlayerSpeed 0x56E090 (cdecl (-1) -> CVector*), CWorld::FindPlayerCoors 0x56E010 (cdecl (out, -1) -> out),
  CGeneral::GetATanOfXY 0x53CC70 (cdecl (x, y) -> ST0), CCarCtrl::FlyAIPlaneInCertainDirection 0x423000 (cdecl (plane)),
  CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (-1, false), x5), CVehicle::GetVehicleAppearance 0x6D1080 (thiscall -> EAX),
  CCarCtrl::TriggerDogFightMoves 0x429300, CCarCtrl::TestWhetherToFirePlaneGuns 0x429520, CCarCtrl::PossiblyFireHSMissile 0x429600
  (all cdecl (CVehicle* self, CEntity* target)).
- Constants: 0x858B40 = 50.0f.
- Fields: CPlane +0x9B4 m_planeHeading (f32), +0x9A8 m_maxAltitude (f32; used as the target altitude by FlyAIPlaneInCertainDirection).
- Behaviour:
  1. `v = FindPlayerSpeed(-1)`; `s = (v.x*50.0f, v.y*50.0f, v.z*50.0f)` (each stored f32); `c = FindPlayerCoors(-1)`;
     `tx = s.x + c.x`, `ty = s.y + c.y` (kept in x87, not rounded through memory), `tz = s.z + c.z` (stored f32) (0x429784-0x4297DD).
  2. `plane->+0x9B4 = GetATanOfXY(tx - P.x, ty - P.y)` (P = plane position; `ty - P.y` evaluated first; the differences are rounded to f32
     when pushed) (0x4297F0-0x429808); `plane->+0x9A8 = tz` (raw copy, 0x42980F); `FlyAIPlaneInCertainDirection(plane)`.
  3. If `FindPlayerVehicle(-1, false)` is non-NULL and its `GetVehicleAppearance() == 5` (PLANE): `TriggerDogFightMoves(plane,
     FindPlayerVehicle(-1, false))` (0x42981A-0x42984D).
  4. `TestWhetherToFirePlaneGuns(plane, FindPlayerVehicle(-1, false))`; `PossiblyFireHSMissile(plane, FindPlayerVehicle(-1, false))`
     (each with a fresh FindPlayerVehicle call; the target may be NULL when the player is on foot) (0x429855-0x429870).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:523-525). Open questions: none.

## 0x429890 CCarCtrl::GetAIPlaneToDoDogFight  (catalogue: gr-unreversed, port level 18)
- Convention / signature: `__cdecl void (CPlane* plane [esp+4])`, plain `ret`. Frame `sub esp,0x18` + esi/edi; E = esp after `push edi`:
  tx [E+8], ty [E+0xC], tz [E+0x10], z*50 temp [E+0x1C]; the plane arg slot [E+0x24] is reused for rand() values.
- Callers: CCarCtrl::GetAIPlaneToDoDogFightAgainstPlayer 0x42F370 (2 tail jumps), CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90.
- Callees: CGeneral::GetATanOfXY 0x53CC70, `rand` 0x821B1E (x3), CCarCtrl::FlyAIPlaneInCertainDirection 0x423000, TestWhetherToFirePlaneGuns
  0x429520, PossiblyFireHSMissile 0x429600.
- Constants: 0x858B40 = 50.0f, 0x858C7C = 3.0518509e-05f (1/32767), 0x858EDC = 600.0f, 0x858FD8 = 300.0f.
- Fields: plane +0x41C m_autoPilot.m_TargetEntity (read without NULL check), target +0x44/+0x48/+0x4C m_vecMoveSpeed; plane +0x425 autopilot
  m_bPlaneDogfightSomething (i8 dogfight mode), +0x3EC/+0x3F0/+0x3F4 m_vecDestinationCoors, +0x9B4 m_planeHeading, +0x9A8 m_maxAltitude.
- Behaviour:
  1. `T = target`; `tz = T.mv.z*50.0f + T.pos.z` (product stored f32 first), `tx = T.mv.x*50.0f + T.pos.x`, `ty = T.mv.y*50.0f + T.pos.y`
     (products kept in x87; sums stored f32) (0x42989F-0x4298F1).
  2. If `(int8)plane->+0x425 != 1` (0x4298E3-0x4298F5):
     `plane->+0x9B4 = GetATanOfXY(tx - P.x, ty - P.y)` (`ty - P.y` first), `plane->+0x9A8 = tz`. Then if `(rand() & 0x3FF) == 500`:
     `+0x425 = 1`; `+0x3EC = (((float)rand() * 3.0518509e-05f) * 600.0f + tx) - 300.0f`; `+0x3F0 = ((rand()*k) * 600.0f + ty) - 300.0f`;
     `+0x3F4 = tz + 50.0f` (0x429938-0x4299B0).
  3. Else (mode 1, 0x4299C3): `plane->+0x9B4 = GetATanOfXY(dest.x - P.x, dest.y - P.y)` (`dest.y - P.y` first); `+0x9A8 = dest.z` (raw);
     `d = sqrt(dy*dy + dx*dx)` (dx = dest.x - P.x, dy = dest.y - P.y, x87); if `d < 50.0f` `+0x425 = 0` (0x429A16-0x429A44).
  4. `FlyAIPlaneInCertainDirection(plane)`; `TestWhetherToFirePlaneGuns(plane, T)`; `PossiblyFireHSMissile(plane, T)` (0x429A4B-0x429A5A).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:528-530). Open questions: none.

## 0x429A70 CCarCtrl::FlyAIHeliInCertainDirection  (catalogue: gr-unreversed, port level 8)
- Convention / signature: `__cdecl void (CHeli* heli [esp+4], float heading [+8], float distance [+0xC], bool approach [+0x10])`, plain `ret`.
  (gr declares `(CHeli*, float, float, bool)`; the third value is the distance to the goal, the bool enables the distance-based pitch.)
  Frame `sub esp,0x90` + ebx/esi/edi; E = esp after `push edi` (0x429A89): heading arg [E+0xA4] (modified in place by step 1), distance
  [E+0xA8], bool [E+0xAC]. Locals: rightDist [E+0xC], vec [E+0x10..0x18], cos/curHeading [E+0x1C], sin/1000 [E+0x20], temps [E+0x24..0x2C],
  hit entity [E+0x30], A [E+0x34..0x3C], R [E+0x40..0x48], D [E+0x4C..0x54], B [E+0x58..0x60], CColPoint [E+0x70] (pos.z [E+0x78]).
  ebx bit `slow` (bl), edi = 0.
- Callers (7): SteerAIHeliTowardsTargetCoors 0x42A630, GetAIHeliToFlyInDirection 0x42A730, SteerAIHeliToFollowEntity 0x42A750,
  SteerAIHeliAsPoliceHeli 0x42AAD0, SteerAIHeliFlyingAwayFromPlayer 0x42ACB0, SteerAIHeliToLand 0x42AD30, GetAIHeliToAttackPlayer 0x42F3C0.
- Callees: CVector::Normalise 0x59C910 (thiscall, x3), CWorld::ProcessLineOfSight 0x56BA00 (cdecl, 12 args `(const CVector* from, const CVector* to,
  CColPoint*, CEntity** , buildings, vehicles, peds, objects, dummies, seeThrough, ignoreCamera, shootThrough)`; always called with
  (.., 1, 0, 0, 0, 0, 0, 0, 1), x6), CGeneral::GetATanOfXY 0x53CC70, `rand` 0x821B1E.
- Globals: CTimer::m_snPreviousTimeInMilliseconds 0xB7CB78, m_snTimeInMilliseconds 0xB7CB84, ms_fTimeStep 0xB7CB5C.
  Constants (f32): 0x858C58 = 0.01, 0x858FE4 = 1.5707964, 0x858B40 = 50.0, 0x858B34 = 60.0, 0x858B8C = 0.5, 0x858628 = 100.0, 0x858C80 = 5.0,
  0x858CA4 = 30.0, 0x858B50 = 0.0, 0x85862C = 10.0, 0x858B1C = 0.1, 0x858CC4 = 0.2, 0x858F48 = 7.0, 0x858F44 = 0.002, 0x858624 = 1.0,
  0x858EE4 = -0.3, 0x858CB8 = 3.1415927, 0x858CBC = 6.2831855, 0x858CC0 = -3.1415927, 0x858B18 = -2.0, 0x858C1C = -1.0, 0x858C9C = -0.8,
  0x858FE0 = 2.1220658, 0x858FDC = -0.2, 0x858CDC = 0.001; immediates -1.0f, 1000.0f, 0.5f, -0.5f, -0.8f (0xBF4CCCCD), 0.3f (0x3E99999A).
- Fields: +0x14 matrix (right +0x00.., fwd +0x10.., pos +0x30), +0x20 m_nRandomSeed (u16), +0x44/+0x48/+0x4C m_vecMoveSpeed, +0x3BA mission,
  +0x3D0 m_nCruiseSpeed (u8), +0x3D9 m_ucHeliSpeedMult (i8 here), +0x3DF m_ucHeliTargetDist2 (u8), +0x3EC/+0x3F0 dest x/y, +0x42C bit 7
  (vehicleFlags bit 39 bIsRCVehicle); CHeli +0x98C m_fLeftRightSkid, +0x990 m_fSteeringUpDown, +0x994 m_fSteeringLeftRight,
  +0x998 m_fAccelerationBreakStatus, +0x9A8 m_fMaxAltitude, +0x9AC field_9AC (target altitude), +0x9B0 m_fMinAltitude, +0x9B8 (u8, "forward
  probe hit" flag; gr has no name - first byte after field_9B4).
- Behaviour:
  1. (0x429A7F-0x429ADB) `slow = 0`; if mission == 0x29 (HELI_NEWS_BEHAVIOUR) and `(float)m_ucHeliTargetDist2 > distance` and
     `sqrt(mv.x*mv.x + mv.y*mv.y) < 0.01f`: `slow = 1`, `heading += 1.5707964f` (stored into the arg slot).
  2. Probe block, only when `(now + seed) % 500 < (prev + seed) % 500` (unsigned, 0x429AE2-0x429B0C); otherwise go to step 3 with nothing written:
     a. `+0x9AC = +0x9A8` (raw). `A = (mv.x*50 + P.x, mv.y*50 + P.y, mv.z*50 + P.z)` (products of x/y stored f32 first). `c = cos(heading)`,
        `s = sin(heading)` (x87 fcos/fsin of the f32 arg, stored f32). `D = Normalise(c, s, -1.0f)`; `B = A + D*60.0f` (component order z, x, y).
        If `ProcessLineOfSight(A, B, ...)` hits: `h = slow ? (+0x9B0 * 0.5f + cp.z) : (cp.z + +0x9B0)`; `+0x9AC = (+0x9AC > h) ? +0x9AC : h`
        (`test ah,0x41; jne`: equal/NaN take h) (0x429BA1-0x429C53).
     b. `len = sqrt(mv.y*mv.y + mv.x*mv.x) * 100.0f + ((+0x42C & 0x80) ? 5.0f : 30.0f)` stored f32 (0x429C6D-0x429CCC). `D = (c, s, 0)`.
        `+0x9B8 = 1`. `B = A + (c*len, s*len, len*0.0f)` (note `len * 0.0f`, so B.z = A.z unless len is inf/NaN). If ProcessLineOfSight(A, B)
        misses: `+0x9B8 = 0`, `+0x994 = 0` (0x42A070) and go to step 3.
     c. Hit (0x429D65): `R = Normalise(m.right.x, m.right.y, 0)`; `rightDist = leftDist = 1000.0f`.
        Right side: `A = P + R*10.0f` (stored), `B = A + D*len`. If `ProcessLineOfSight(&heli->pos, A)` hits -> `rightDist = 0`; else if
        `ProcessLineOfSight(A, B)` hits -> `rightDist = sqrt((dz*dz + dy*dy) + dx*dx)` of `cp.pos - A` (0x429E5C-0x429EEF).
        Left side: `A = P - R*10.0f` (each `P.c - R.c*10`, z: `P.z - (R.z*10 stored)`), `B = A + D*len`. If LOS(&heli->pos, A) hits ->
        `left = 0.0f`; else if LOS(A, B) hits -> `left = |cp.pos - A|`; else `left = 1000.0f` (0x429EF3-0x42A049).
        `+0x994 = (left > rightDist) ? 0.5f : -0.5f` (`test ah,0x41; jne`, equal/NaN -> -0.5f) (0x42A04D-0x42A064).
  3. (0x42A081) If mission == 0x30 (HELI_LAND_TOUCHING_DOWN): `+0x9B8 = 0`, `+0x994 = 0`.
  4. `cur = GetATanOfXY(m.fwd.x, m.fwd.y)` stored f32. Lift: `+0x998 = 0`; `x = +0x9AC - (mv.z*100.0f + P.z)`; `x *= (x > 0) ? 0.1f : 0.2f`
     (stored to +0x998); `v = ((float)(rand() & 0xF) - 7.0f) * 0.002f + +0x998` (stored to +0x998); `q = v - 0.5f` (stored f32 local);
     `+0x998 = clamp(v, -0.3f, 1.0f)` (v >= 1 or NaN -> 1.0f) (0x42A097-0x42A15C).
  5. Yaw: `d = heading - cur`; `while (d > pi) d -= 2pi`; `while (d < -pi) d += 2pi`; `+0x98C = d * -2.0f` (fst), then `+0x98C = clamp(.., -1, 1)`
     (NaN -> 1.0f) (0x42A162-0x42A209).
  6. Pitch target: if `distance > 60.0f` or `!approach`: `+0x990 = -0.8f`. Else `fx = mv.x*50 + P.x`, `fy = mv.y*50 + P.y`, `m = (float)
     m_ucHeliTargetDist2`; `e = sqrt((fy - dest.y)^2 + (fx - dest.x)^2) - m`; if `e < 0` leave +0x990 unchanged; else `+0x990 = (e * -0.8f) /
     (30.0f - m)` (0x42A20F-0x42A2C2).
  7. If `+0x990 < 0` (0x42A2D2): `d = cur - heading` wrapped to [-pi, pi] (first `< -pi` loop, then `> pi` loop), `a = |d|`;
     `f = 1.0f - 2.1220658f * a`, `f = (0 > f) ? 0 : f`; `+0x990 = f * +0x990`. If `a > 1.5707964f` and `(mv.z*fwd.z + mv.y*fwd.y) + fwd.x*mv.x > 0`:
     `+0x990 = 0.3f` (0x42A2E9-0x42A3A9).
  8. If `q > 0`: `g = (q < 1.0f) ? q : 1.0f`; `+0x990 = (1.0f - g) * +0x990` (0x42A3B3-0x42A3EF).
  9. If `+0x990 < 0` (0x42A3F7): `k = ((float)cruise - ((mv.z*fwd.z + mv.y*fwd.y) + mv.x*fwd.x) * 60.0f) * 0.1f`, `k = clamp(k, 0, 1)` (NaN kept);
     `+0x990 = (-k < +0x990) ? +0x990 : -k` (stored). Then if `m_ucHeliSpeedMult != 0` and `+0x990 < -0.2f`: if `+0x9B8 != 0` go to step 10;
     else `b = ((float)(int8)m_ucHeliSpeedMult * ms_fTimeStep) * 0.001f`; `mv.x += fwd.x * b`, `mv.y += fwd.y * b`, `mv.z += b * 0.0f`
     (0x42A4D1-0x42A54A).
  10. If `+0x9B8 != 0` (0x42A551): `F = Normalise(fwd.x, fwd.y, 0)`; `+0x990 = ((F.y*mv.y + F.z*mv.z) + F.x*mv.x) * 2` (fadd st0,st0).
  11. `+0x990 = clamp(+0x990, -1.0f, 1.0f)` (NaN -> 1.0f) (0x42A5A7-0x42A60F).
- Edge cases: the probe block runs only on 500 ms boundary frames; on other frames +0x994 / +0x9B8 keep their values unless step 3 applies.
  Step 1 modifies the caller's heading for the rest of the function. The heli matrix is dereferenced without NULL checks (fwd/right reads).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:440-442). Open questions: none.

## 0x42A630 CCarCtrl::SteerAIHeliTowardsTargetCoors  (catalogue: gr-unreversed, port level 9)
- Convention / signature: `__cdecl void (CHeli* heli [esp+4])`, plain `ret`. The heli arg slot is reused for the heading.
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1). Callees: CCarCtrl::FlyAIHeliToTarget_FixedOrientation 0x423940 (cdecl),
  CGeneral::GetATanOfXY 0x53CC70, CCarCtrl::FlyAIHeliInCertainDirection 0x429A70 (cdecl (heli, float heading, float distance, bool)).
- Constants: 0x858B50 = 0.0f.
- Fields: CAutomobile +0x954 m_fForcedOrientation (f32, -1 = none), autopilot +0x3EC/+0x3F0/+0x3F4 m_vecDestinationCoors.
- Behaviour:
  1. If `!(heli->+0x954 < 0.0f)` (`test ah,1; jne`: negative or NaN take step 2): `FlyAIHeliToTarget_FixedOrientation(heli, +0x954, dest (CVector
     by value), 1)` - six dwords are pushed (`push 1` first, `add esp,0x18`); the callee (CCarCtrl.md) uses five, the trailing 1 is ignored
     (0x42A649-0x42A673). Return.
  2. `h = GetATanOfXY(dest.x - P.x, dest.y - P.y)` (`dest.y - P.y` first, stored f32 in the arg slot); `d = sqrt((dest.y - P.y)^2 + (dest.x - P.x)^2)`
     (each difference recomputed from memory; y product first) stored f32; `FlyAIHeliInCertainDirection(heli, h, d, true)` (0x42A68D-0x42A707).
- gta-reversed: `plugin::Call` only. Open questions: none.

## 0x42A730 CCarCtrl::GetAIHeliToFlyInDirection  (catalogue: gr-unreversed, port level 9)
- Convention / signature: `__cdecl void (CHeli* heli [esp+4])`, plain `ret`. Entry `jmp 0x156DD90` (.HOODLUM body, same behaviour).
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1). Callees: CCarCtrl::FlyAIHeliInCertainDirection 0x429A70.
- Behaviour: `FlyAIHeliInCertainDirection(heli, heli->+0x9B4 (CHeli field_9B4, heading, f32 passed as raw dword), 1000.0f (imm 0x447A0000),
  false)` (0x156DD90-0x156DDA3).
- gta-reversed: `plugin::Call` only. Open questions: none.

## 0x42A750 CCarCtrl::SteerAIHeliToFollowEntity  (catalogue: gr-unreversed, port level 9)
- Convention / signature: `__cdecl void (CHeli* heli [esp+4])`, plain `ret`. Frame `sub esp,0x1C` + 4 pushes; E = esp after `push edi`:
  heading [E+0x1C]; the heli arg slot [E+0x30] is reused (target position pointer, then the 2D distance).
- Callers: CCarCtrl::SteerAIHeliToKeepEntityInView 0x42AEB0, CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90.
- Callees: AllocateMatrix 0x54F560 + UpdateMatrix 0x54F1B0 (ensure the target's matrix), CGeneral::GetATanOfXY 0x53CC70,
  CCarCtrl::FlyAIHeliToTarget_FixedOrientation 0x423940, CCarCtrl::FlyAIHeliInCertainDirection 0x429A70, CTaskManager::GetActiveTask 0x681720
  (thiscall), task vtable +0x10 GetTaskType.
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84. Constants: 0x858B50 = 0.0f, 0x858B44 = 6.0f, 0x858B40 = 50.0f, 0x858FE8 = 25.0f, 0x858FD8 = 300.0f.
- Fields: heli +0x41C m_TargetEntity (T, no NULL check), +0x3EC dest, +0x3DA autopilot field_4A (i8 forward offset), +0x3DB m_nCarCtrlFlags
  (0x20 bDoTargetCatchupCheck, 0x80 bHeliFollowTarget), +0x3BA mission, +0x430 m_nCreationTime, +0x4C0 m_fHealth, +0x954 m_fForcedOrientation,
  +0x9A8 m_fMaxAltitude; target vehicle +0x36 type/status, +0x3BA, +0x3B9, +0x3D0; target ped +0x47C m_pIntelligence (task manager at +4).
- Behaviour:
  1. `dest = T.pos` (0x42A770-0x42A785). If `(int8)heli->+0x3DA != 0`: ensure T's matrix; `o = (float)(int8)+0x3DA`;
     `dest.x += T.fwd.x * o`, `dest.y += T.fwd.y * o`, `dest.z += o * 0.0f` (T.fwd.z is copied but unused) (0x42A792-0x42A806).
  2. `heading = GetATanOfXY(dest.x - P.x, dest.y - P.y)` (`dest.y - P.y` first) stored f32. `d = sqrt((T.y - P.y)^2 + (T.x - P.x)^2)` (from T's
     position, not dest; y product first) stored f32 (0x42A80C-0x42A8F7).
  3. `alt = (T.pos.z > 6.0f) ? T.pos.z : 6.0f` (`test ah,0x41; je`), `+0x9A8 = alt`; if `d > 50.0f`: `+0x9A8 = (alt > 25.0f) ? alt : 25.0f` (0x42A905-0x42A949).
  4. If `!(+0x954 < 0.0f)`: `FlyAIHeliToTarget_FixedOrientation(heli, +0x954, T.pos (CVector by value), 1)` (6 dwords, trailing 1 unused);
     else `FlyAIHeliInCertainDirection(heli, heading, d, true)` (0x42A953-0x42A9B0).
  5. If `+0x3DB & 0x20`: `d2 = sqrt((P.y - T.y)^2 + (P.x - T.x)^2)`; if `d2 < 25.0f` (0x42A9C0-0x42AA07):
     - T is a vehicle (type 2) with status 2 or 3: `T.status = 3` (`(+0x36 & 7) | 0x18`), `T.mission = 1` (CRUISE), `T.+0x3D0 = 100`, `T.+0x3B9 = 2`;
     - T is a ped (type 3): `t = T->+0x47C->taskManager.GetActiveTask()`; if `t` and `t->GetTaskType() == 0x390` (912 TASK_COMPLEX_WANDER):
       `t->+0xC = 7` (gr CTaskComplexWander m_nMoveState);
     then `+0x3DB &= ~0x20` (0x42AA6F). (No clear when d2 >= 25.)
  6. If `+0x3DB & 0x80` and `now > +0x430 + 50000` (unsigned): `+0x3DB &= 0x7F`, `mission = 0x2A` (HELI_FLY_AWAY_FROM_PLAYER) (0x42AA76-0x42AAA0).
  7. If `mission == 0x29` (HELI_NEWS_BEHAVIOUR) and `+0x4C0 < 300.0f`: `mission = 0x2A` (0x42AAA6-0x42AAC2).
- gta-reversed: `plugin::Call` only. Open questions: none.

## 0x42AAD0 CCarCtrl::SteerAIHeliAsPoliceHeli  (catalogue: gr-unreversed, port level 9)
- Convention / signature: `__cdecl void (CHeli* heli [esp+4])`, plain `ret`. Frame `sub esp,0x10` + 4 pushes; the heli arg slot is reused for
  the 2D distance; heading in a local.
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1). Callees: CGeneral::GetATanOfXY 0x53CC70, CCarCtrl::FlyAIHeliInCertainDirection 0x429A70.
- Constants: 0x858B44 = 6.0f, 0x858B40 = 50.0f, 0x858FE8 = 25.0f, 0x858F80 = 230.0f.
- Fields: +0x41C m_TargetEntity (T, no NULL check), +0x3EC dest, +0x9A8 m_fMaxAltitude, +0x4C0 m_fHealth, +0x3BA mission.
- Behaviour: `heading = GetATanOfXY(T.x - H.x, T.y - H.y)` (`T.y - H.y` first, stored f32); `d = sqrt((T.y - H.y)^2 + (T.x - H.x)^2)` (y product
  first) stored f32; `alt = (T.z > 6.0f) ? T.z : 6.0f`, `+0x9A8 = alt`; if `d > 50.0f` `+0x9A8 = (alt > 25.0f) ? alt : 25.0f` (same code as
  SteerAIHeliToFollowEntity step 3); `dest (+0x3EC) = T.pos`; `FlyAIHeliInCertainDirection(heli, heading, d, true)`; if `+0x4C0 < 230.0f`
  `mission = 0x2A` (HELI_FLY_AWAY_FROM_PLAYER) (0x42AC86-0x42AC9C).
- gta-reversed: `plugin::Call` only. Open questions: none.

## 0x42ACB0 CCarCtrl::SteerAIHeliFlyingAwayFromPlayer  (catalogue: gr-unreversed, port level 9)
- Convention / signature: `__cdecl void (CHeli* heli [esp+4])`, plain `ret`. Frame `sub esp,0x18` + 4 pushes; two FindPlayerCoors buffers;
  the heli arg slot is reused for the heading.
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1). Callees: CWorld::FindPlayerCoors 0x56E010 (x2), CGeneral::GetATanOfXY
  0x53CC70, CCarCtrl::FlyAIHeliInCertainDirection 0x429A70.
- Constants: 0x858CB8 = 3.1415927f; immediate 1000.0f (0x447A0000).
- Behaviour: `P = heli pos`; `c1 = FindPlayerCoors(-1)`, `c2 = FindPlayerCoors(-1)`; `h = GetATanOfXY(c1.x - P.x, c2.y - P.y)` (`c2.y - P.y`
  computed first; both stored f32 as the call args) `+ 3.1415927f` (stored f32, 0x42ACD2-0x42AD0B); `FlyAIHeliInCertainDirection(heli, h,
  1000.0f, false)` (0x42AD15). The heading points from the player to the heli (+pi) and is not wrapped into [-pi, pi].
- gta-reversed: `plugin::Call` only. Open questions: none.

## 0x42AD30 CCarCtrl::SteerAIHeliToLand  (catalogue: gr-unreversed, port level 9)
- Convention / signature: `__cdecl void (CHeli* heli [esp+4])`, plain `ret`. Frame `push ecx` (heading local) + ebx (= 0) / esi / edi; the heli
  arg slot is reused for the distance.
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1). Callees: CGeneral::GetATanOfXY 0x53CC70, CCarCtrl::FlyAIHeliInCertainDirection 0x429A70.
- Constants: 0x85862C = 10.0f, 0x858C28 = 0.05f, 0x858D00 = 1.0f.
- Fields: +0x3EC/+0x3F0 dest, +0x44/+0x48 move speed, +0x7D4..+0x7E0 CAutomobile m_fWheelsSuspensionCompression[4], CHeli +0x98C/+0x990/+0x994/
  +0x998 control outputs, +0x9A8 m_fMaxAltitude, +0x9B0 m_fMinAltitude.
- Behaviour:
  1. `heading = GetATanOfXY(dest.x - P.x, dest.y - P.y)` (`dest.y - P.y` first); `d = sqrt((dest.y - P.y)^2 + (dest.x - P.x)^2)` (stored f32);
     `FlyAIHeliInCertainDirection(heli, heading, d, true)` (0x42AD4D-0x42ADD5).
  2. If `d < 10.0f` and `sqrt(mv.x*mv.x + mv.y*mv.y) < 0.05f` (0x42ADDA-0x42AE14): `+0x9B0 = 0`, `+0x9A8 = 0`; then if any of the four
     suspension compressions (tested in index order 0..3) is `< 1.0f` (a wheel touches ground): `+0x998 = +0x98C = +0x990 = +0x994 = 0`
     (0x42AE16-0x42AE80).
- gta-reversed: `plugin::Call` only. Open questions: none.

## 0x42AEB0 CCarCtrl::SteerAIHeliToKeepEntityInView  (catalogue: gr-unreversed, port level 10)
- Convention / signature: `__cdecl void (CHeli* heli [esp+4])`, plain `ret`. Frame `sub esp,8` + ebx/esi (+ transient ebp/edi); the heli arg
  slot [esp0+4] receives the 2D distance `d`; [esp0-4] holds `(float)` scratch / the heading.
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1). Callees: CGeneral::GetATanOfXY 0x53CC70 (x2), CCarCtrl::SteerAIHeliToFollowEntity
  0x42A750, `rand` 0x821B1E.
- Constants (f32): 0x858FE4 = 1.5707964, 0x858CB8 = 3.1415927, 0x858CBC = 6.2831855, 0x858CC0 = -3.1415927, 0x858C1C = -1.0, 0x858624 = 1.0,
  0x858B48 = 15.0, 0x858628 = 100.0, 0x858B50 = 0.0, 0x858B1C = 0.1, 0x858CC4 = 0.2, 0x858C24 = 0.3, 0x858F48 = 7.0, 0x858F44 = 0.002,
  0x858B8C = 0.5, 0x858CE8 = 1.5; immediates 0.3f (0x3E99999A), +-0.5f.
- Fields: +0x41C m_TargetEntity (T, no NULL check), +0x3D8 m_ucHeliTargetDist (u8), +0x3EC dest, +0x44/+0x48/+0x4C move speed, +0x14 matrix (read
  raw for fwd), CHeli +0x98C/+0x990/+0x994/+0x998 outputs, +0x9A8 m_fMaxAltitude, +0x9AC field_9AC.
- Behaviour:
  1. `h1 = GetATanOfXY(T.x - H.x, T.y - H.y)` (`T.y - H.y` first; kept in ST0); `d = sqrt((T.y - H.y)^2 + (T.x - H.x)^2)` (stored f32 in the arg
     slot); if `d > (float)m_ucHeliTargetDist * 2` (`fadd st0,st0`; `test ah,0x41; jne`): discard h1, `SteerAIHeliToFollowEntity(heli)`, return
     (0x42AF08-0x42AFEC).
  2. Yaw (0x42AFED-0x42B0A4): `h = h1 + 1.5707964f` (stored f32); `e = h - GetATanOfXY(m.fwd.x, m.fwd.y)`; `while (e > pi) e -= 2pi`;
     `while (e < -pi) e += 2pi`; `+0x98C = e * -1.0f` (fst) then clamped to [-1, 1] (NaN -> 1.0f).
  3. Altitude: `+0x9A8 = T.z + 15.0f`; `dest (+0x3EC) = T.pos`; `+0x9AC = +0x9A8` (from the f32 just stored) (0x42B0B9-0x42B0F0).
  4. Lift: `+0x998 = 0.3f` (temporarily); `x = +0x9AC - (mv.z*100.0f + P.z)`; `x = x * ((x > 0) ? 0.1f : 0.2f) + 0.3f` -> `+0x998`;
     `v = ((float)(rand() & 0xF) - 7.0f) * 0.002f + +0x998` (fst to +0x998), then `+0x998 = clamp(v, 0.0f, 1.0f)` (NaN -> 1.0f)
     (0x42B108-0x42B1B5).
  5. Roll: `t = (float)m_ucHeliTargetDist`; if `t * 0.5f > d` (`test ah,0x41; jne`): `+0x994 = 0.5f`; else if `d > t` : `+0x994 = -0.5f`;
     else `+0x994 = (mv.z*m.right.z + mv.y*m.right.y) + m.right.x*mv.x` (matrix right row, 0x42B1FB-0x42B213).
  6. Pitch: `+0x990 = 0`; if `d < t * 1.5f` (fcompp, strict, NaN false): `+0x990 = (mv.z*fwd.z + mv.y*fwd.y) + mv.x*fwd.x` (0x42B219-0x42B255).
- gta-reversed: `plugin::Call` only. Open questions: none.

## 0x42B270 CCarCtrl::FireHeliRocketsAtTarget  (catalogue: gr-unreversed, port level 17)
- Convention / signature: `__cdecl void (CAutomobile* heli [esp+4], CEntity* target [+8])`, plain `ret`. Frame `sub esp,0x30` + ebx/esi/edi.
- Callers: CCarCtrl::GetAIHeliToAttackPlayer 0x42F3C0 (1).
- Callees: CCollision::DistToMathematicalLine 0x412970 (cdecl (const CVector* lineStart, const CVector* lineEnd, const CVector* point) -> ST0),
  CProjectileInfo::AddProjectile 0x737C80 (cdecl (CEntity* creator, eWeaponType, CVector pos (by value), float force, CVector* dir, CEntity* victim)).
- Globals: CTimer::m_snPreviousTimeInMilliseconds 0xB7CB78, m_snTimeInMilliseconds 0xB7CB84. Constants: 0x858A14 = 80.0f, 0x858F48 = 7.0f,
  0x858B90 = 4.0f, 0x858CE8 = 1.5f; immediate 1.0f.
- Fields: heli +0x513 m_nVehicleWeaponInUse (u8), +0x14 matrix (right +0x00, fwd +0x10, pos +0x30; read raw at 0x42B315/0x42B36F), positions.
- Behaviour:
  1. Return unless `heli->+0x513` is 4 (CAR_WEAPON_DOUBLE_ROCKET) or 0 (0x42B27A-0x42B286).
  2. Return if `now / 250 == prev / 250` (unsigned; magic 0x10624DD3, `shr 4`) (0x42B28C-0x42B2AC).
  3. `d = sqrt((dz*dz + dy*dy) + dx*dx)` with `d? = H.? - T.?`; return unless `d < 80.0f` (0x42B2D0-0x42B303).
  4. `Q = (H.x + fwd.x, H.y + fwd.y, H.z + fwd.z)` (stored f32); return unless `DistToMathematicalLine(&H.pos, &Q, &T.pos) < 7.0f` (0x42B313-0x42B35E).
  5. `side = (now / 250) & 1`. `p = (fwd.x*4 + H.x, fwd.y*4 + H.y, fwd.z*4 + H.z)` (x/y kept in x87, z stored f32); `r = right * 1.5f`
     (x, z stored f32; y in x87); if `side`: `r = -r` (each negated via fchs and stored) (0x42B364-0x42B413).
     `AddProjectile(heli, 0x13 (WEAPON_ROCKET), (r.x + p.x, r.y + p.y, r.z + p.z), 1.0f, &heli->m_matrix->fwd, NULL)` (0x42B417-0x42B45B).
- Edge cases: alternates the launch side every 250 ms; fires at most once per 250 ms boundary.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:435-437). Open questions: none.

## 0x42B470 CCarCtrl::FindLinksToGoWithTheseNodes  (catalogue: gr-unreversed, port level 1)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret`. Frame `sub esp,0x1C` + esi/ebx/ebp/edi; E = esp
  after the 4 pushes (0x42B498): bestDist [E+0x10], B (link node pos) [E+0x14..0x1C], A (current node pos) [E+0x20..0x28];
  the arg slot [E+0x30] is reused as a dword scratch.
- Callers: CCarCtrl::JoinCarWithRoadSystem 0x42F5A0, CCarCtrl::ReconsiderRoute 0x42FC40. Callees: `srand` 0x821B11 (cdecl),
  CCollision::DistToLine 0x417610 (cdecl (const CVector* lineStart, const CVector* lineEnd, const CVector* point) -> ST0).
- Globals: ThePaths.m_pPathNodes 0x96F854, m_pNodeLinks 0x96FA94 (CNodeAddress[], 4 bytes), m_pNaviLinks 0x96FDF4 (u16 link
  addresses), all indexed by area. Constant 0x858C48 = 0.125f; immediate 999999.875f (0x497423FE).
- Fields: veh +0x45E m_nForcedRandomRouteSeed (u16), +0x390 m_autoPilot.m_currentAddress (area u16 +0x390, node u16 +0x392),
  +0x394/+0x396 m_startingRouteNode (area/node), +0x3A4 m_nCurrentPathNodeInfo (u16), +0x3A6 m_nNextPathNodeInfo (u16),
  +0x3B5 `_smthCurr` (i8, direction of the current link), +0x3B6 `_smthNext` (i8). CPathNode (stride 0x1C): +0x08/+0x0A/+0x0C
  pos x/y/z (i16, *1/8), +0x10 m_wBaseLinkId (i16), +0x18 flags (bits 0-3 m_nNumLinks).
  Notation: `cur = veh->+0x390` (dword), `N = m_pPathNodes[cur.area] + cur.node*0x1C`, `base = (int16)N->m_wBaseLinkId`.
- Behaviour:
  1. If `veh->+0x45E != 0`: `srand(zero-extended seed)` (0x42B478-0x42B488).
  2. Next link (0x42B490-0x42B514): `for (i = 0; i < 12; i++)` (i16, 0x42B4E8) stop at the first `m_pNodeLinks[cur.area][base + i]`
     whose area == `veh->+0x394` and node == `veh->+0x396`. `veh->+0x3A6 = m_pNaviLinks[cur.area][base + i]` (0x42B50A-0x42B514).
     The bound is the constant 12, not the node's link count; if nothing matches, `i = 12` and the entry `base + 12` (beyond this
     node's links) is used (original behaviour, keep).
  3. `veh->+0x3B6 = (cur.area < start.area || (cur.area == start.area && cur.node < start.node)) ? -1 : 1` (unsigned word compares,
     0x42B51B-0x42B54D; start = +0x394/+0x396).
  4. `n = N->flags & 0xF` (0x42B566). If `n == 1`: `idx = 0` (0x42B57F) and go to step 6.
     Else `best = -1`, `bestDist = 999999.875f`; if `n == 0` -> `idx = 0` (0x42B5BF `jbe` = ZF after `test al,0xF`).
     Loop `i = 0 .. n-1` (n re-read from `veh->+0x390`'s node every iteration, 0x42B6FD-0x42B727; `cur` re-read at 0x42B5C5):
     `L = m_pNodeLinks[cur.area][base + i]`; skip if `L == (veh->+0x394, veh->+0x396)` (0x42B5F1-0x42B60E) or if
     `m_pPathNodes[L.area] == NULL` (area not loaded, 0x42B617-0x42B620). Otherwise
     `A = ((float)N.x * 0.125f, (float)N.y * 0.125f, (float)N.z * 0.125f)` (fild of the sign-extended i16 then `fmul [0x858C48]`, each
     stored f32; computed z, y, x and stored x, y, z, 0x42B63A-0x42B680), `B` = same for node `L` (0x42B684-0x42B6C7),
     `P = veh->GetPosition()` (matrix +0x30 or placement +4), `d = DistToLine(&A, &B, &P)` (0x42B6E0).
     If `d < bestDist` (strict, `test ah,5; jp`; NaN false): `bestDist = d` (stored f32), `best = i` (0x42B6F3-0x42B6F7).
     After the loop `if (best < 0) best = 0` (0x42B72D-0x42B731); `idx = best`.
  5. (non-single path) `cur` re-read (0x42B733), `base` recomputed.
  6. `veh->+0x3A4 = m_pNaviLinks[cur.area][base + idx]` (0x42B75C-0x42B766). `L = m_pNodeLinks[cur.area][base + idx]`;
     `veh->+0x3B5 = (L.area < cur.area || (L.area == cur.area && L.node < cur.node)) ? -1 : 1` (unsigned, 0x42B79C-0x42B7C2).
- Edge cases: the srand reseed makes the following rand() calls deterministic for vehicles with a forced seed. The step-2 bound of 12
  can read one entry past the node's links. The current node's area must be loaded (no NULL check for `m_pPathNodes[cur.area]`).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:378-380). Open questions: none.

## 0x42B7D0 CCarCtrl::GenerateOneEmergencyServicesCar  (catalogue: gr-unreversed, port level 16)
- Convention / signature: `__cdecl CAutomobile* (uint32 modelId [esp+4], CVector target (by value) [+8..+0x10])`, returns EAX (NULL on failure),
  plain `ret`. Entry `push -1; jmp 0x4018AB`: the stub computes `[0x156068C] - [0x71E135] - 1` (`stc; sbb`) = 0x0083745B (the SEH handler) and
  pushes it via `xchg [esp], esi`, then continues at 0x42B7D7 (MSVC SEH frame for the `new`/constructor; state slot [E+0x84]). E = esp after
  `push edi` (0x42B801): count (i16) [E+0x14], node1 [E+0x18] (later groundZ), node2 [E+0x1C], frac [E+0x24], entity [E+0x28], outPos
  [E+0x2C..0x34], dir [E+0x38..0x40] (copy [E+0x44..0x4C]), CColPoint [E+0x50] (pos.z [E+0x58]); found flag byte [E+0x13].
- Callers: CCarCtrl::GenerateEmergencyServicesCar 0x42F9C0 (2), ScriptGenerateOneEmergencyServicesCar 0x42FBC0, CGangWars::CreateAttackWave 0x444810.
- Callees: CWorld::FindPlayerCentreOfWorld 0x56E250, CCarCtrl::GenerateCarCreationCoors2 0x424210 (cdecl, 15 dwords; see CCarCtrl.md),
  CWorld::FindObjectsKindaColliding 0x568B80, `CVehicle::operator new` 0x6E2D50 (size 0x988), CAutomobile::CAutomobile 0x6B0A90 (thiscall
  (model, createdBy 1, setupSuspensionLines 1)), CWorld::ProcessVerticalLine 0x5674E0 (x2), virtual scalar deleting destructor (vtable +0,
  arg 1), CEntity::GetDistanceFromCentreOfMassToBaseOfModel 0x536BE0 (thiscall -> ST0), CAutomobile::PlaceOnRoadProperly 0x6AF420,
  CWorld::Add 0x563220, CCarAI::AddAmbulanceOccupants 0x41C4A0, CCarAI::AddFiretruckOccupants 0x41C600, CCarAI::AddPoliceCarOccupants 0x41C070
  (cdecl (veh, bool 0)).
- Globals: CWorld::PlayerInFocus 0xB7CD74, ThePaths.m_pPathNodes 0x96F854, NumAmbulancesOnDuty 0x9690A8, NumFireTrucksOnDuty 0x9690AC (RW).
  Constants: 0x858B50 = 0.0f, 0x858624 = 1.0f, 0x858C48 = 0.125f, 0x858FEC = 1.0e9f; immediates 160.0f, 0.707f (0x3F34FDF4), -1.0f, 10.0f,
  1000.0f, -1000.0f, 1.0e9f (0x4E6E6B28).
- Behaviour:
  1. `node1.area = node2.area = 0xFFFF`; `C = FindPlayerCentreOfWorld(PlayerInFocus)`; `found = 0`; `i = 0` (i16 in ebx).
  2. Search loop (0x42B820-0x42B8CD): `while (i < 5)`: `ok = GenerateCarCreationCoors2(C, 0.707f, 0.707f, -1.0f, true, 160.0f, 160.0f, &outPos,
     &node1, &node2, &frac, false, false)`; if `ok` and node1 is not a water node (`flags byte +0x18` bit 7 clear):
     `FindObjectsKindaColliding(outPos, 10.0f, true, &count, 2, NULL, false, true, true, false, false)` and `found = (count == 0)`; `i++`;
     stop when found. Then `if (i >= 5) return NULL` (0x42B8D3) - so a candidate found on the 5th attempt is discarded (original quirk,
     only four attempts can succeed).
  3. `car = new (0x988) CAutomobile(modelId, 1, 1)` (NULL if the pool is full -> the code continues with esi = 0 and crashes at
     `[esi+0x14]`; keep). Position = outPos (matrix +0x30 or placement +4).
  4. Orientation (0x42B94C-0x42BA25): `d = target - outPos` (stored f32 each); `len = sqrt(d.y*d.y + d.x*d.x)`; if `len == 0` (`test ah,0x44; jp`;
     NaN normalises): `(X, Y) = (1.0f, d.y)`; else `inv = 1.0f / len` (stored f32), `X = inv * d.x`, `Y = inv * d.y`. Matrix:
     `fwd = (X, Y, 0)`, `right = (Y, -X, 0)`, `up = (0, 0, 1.0f)` (stored in the order fwd.x, fwd.y, fwd.z, right.x, right.y, right.z, up.x, up.y, up.z).
  5. Ground (0x42BA12-0x42BB24): `outPos.z = (1.0f - frac) * (node1.z * 0.125f) + frac * (node2.z * 0.125f)` (stored); `gz = 1.0e9f`;
     `if (ProcessVerticalLine(outPos, 1000.0f, cp, ent, 1, 0, 0, 0, 1, 0, NULL)) gz = cp.z`; `if (ProcessVerticalLine(outPos, -1000.0f, ...same...)
     && |cp.z - outPos.z| < |gz - outPos.z|) gz = cp.z` (fabs via fchs on negative).
  6. If `gz == 1.0e9f`: `car->vtbl[0](1)` (delete) and return NULL (0x42BB28-0x42BB41).
  7. `z = car->GetDistanceFromCentreOfMassToBaseOfModel() + gz`; set position `(outPos.x, outPos.y, z)`; `m_vecMoveSpeed = 0`;
     `PlaceOnRoadProperly()`; `CWorld::Add(car)` (0x42BB56-0x42BBC3).
  8. By model (jump table 0x42BC4C, byte table 0x42BC5C indexed `model - 407`, `ja` for > 0xC0): 407 FIRETRUK -> `+0x428 |= 4`
     (bIsFireTruckOnDuty), `NumFireTrucksOnDuty++`, `AddFiretruckOccupants(car)`; 416 AMBULAN -> `+0x428 |= 2` (bIsAmbulanceOnDuty),
     `NumAmbulancesOnDuty++`, `AddAmbulanceOccupants(car)`; 427, 523, 596, 597, 598, 599 -> `AddPoliceCarOccupants(car, false)`; for these
     three cases `+0x42D |= 0x80` (vehicleFlags bit 47 bSirenOrAlarm). Other models: nothing. Return `car`.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:465-467). Open questions: none.

## 0x42BD20 CCarCtrl::FindNodesThisCarIsNearestTo  (catalogue: gr-unreversed, port level 1)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], CNodeAddress* outNode [+8], CNodeAddress* outLinkNode [+0xC])`, plain `ret`.
  Frame `sub esp,0x9C` + ebx/ebp/esi/edi; E = esp after the pushes (0x42BD44): best node [E+0x20] (dword), best link node [E+0x24],
  bestScore [E+0x38], current area index [E+0x14], node index [E+0x2C], node byte offset [E+0x28], dir vector [E+0x40..0x48],
  line distance [E+0x4C], VP [E+0x54..0x5C], A [E+0x64..0x6C], B [E+0x70..0x78].
- Callers: CCarCtrl::ReconsiderRoute 0x42FC40 (1).
- Callees (all thiscall ECX = ThePaths 0x96F050 unless noted): CPathFind::FindXRegionForCoors 0x44D890 (float) / FindYRegionForCoors 0x44D8C0
  -> EAX int; FindXCoorsForRegion 0x44D8F0 / FindYCoorsForRegion 0x44D910 (int) -> ST0; CCollision::DistToLine 0x417610 (cdecl (A, B, P) -> ST0);
  CVector::Normalise 0x59C910 (thiscall ECX = vector).
- Globals: ThePaths m_pPathNodes 0x96F854, m_pNodeLinks 0x96FA94, m_anNumVehicleNodes 0x970114 (ThePaths + 0x10C4, u32 per area).
  Constants: 0x858A48 = 200.0f, 0x858FF0 = 550.0f, 0x858C48 = 0.125f, 0x858A28 = 150.0f, 0x858B3C = 3.0f, 0x858624 = 1.0f, 0x858C80 = 5.0f;
  immediate 99999.898f (0x47C34FF3).
- Behaviour:
  1. `best.area = bestLink.area = 0xFFFF` (only the low words are written; the node words stay uninitialised stack), `bestScore = 99999.898f`.
  2. `P = veh pos`; `xr = FindXRegionForCoors(P.x)`, `yr = FindYRegionForCoors(P.y)`; `fx = P.x - FindXCoorsForRegion(xr)` (stored f32),
     `fy = P.y - FindYCoorsForRegion(yr)` (x87). `x0 = (fx < 200) ? xr-1 : xr`, `y0 = (fy < 200) ? yr-1 : yr`, `x1 = (fx > 550) ? xr+1 : xr`,
     `y1 = (fy > 550) ? yr+1 : yr` (strict; NaN keeps the region); clamp `x0, y0 = max(.., 0)`, `x1, y1 = min(.., 7)` (signed)
     (0x42BDBF-0x42BE48). If `x0 > x1` write the outputs immediately (0x42C226).
  3. `for (x = x0; x <= x1; x++) for (y = y0; y <= y1; y++)` with `area = x + 8*y` (0x42BE60-0x42C1FB; y loop runs `y1 - y0 + 1` times,
     skipped if `y0 > y1`): skip the area if `m_pPathNodes[area] == NULL`; for node i = 0 .. m_anNumVehicleNodes[area]-1 (`N = nodes + i*0x1C`):
     a. `d = sqrt((dz*dz + dy*dy) + dx*dx)` with `d? = N.? * 0.125f - P.?` (dx stored f32 first, 0x42BEC4-0x42BF33). Skip if `!(d < 150.0f)`.
     b. For each link k = 0 .. (N.flags & 0xF)-1: `L = m_pNodeLinks[area][N.base + k]`; skip if `m_pPathNodes[L.area] == NULL`.
        `A = (Nx, Ny, Nz * 3.0f)`, `B = (Lx, Ly, Lz * 3.0f)`, `VP = (P.x, P.y, P.z * 3.0f)` (node coords i16 * 0.125f stored f32, then the z values
        multiplied by 3.0f and stored again; 0x42BF86-0x42C06C). `dl = DistToLine(&A, &B, &VP)` stored f32.
        `dir = (Lx - Nx, Ly - Ny, Lz - Nz)` (unscaled, recomputed from the i16 values; stored f32), `dir.Normalise()`.
        `score = (1.0f - ((dir.z * F.z + dir.y * F.y) + dir.x * F.x)) * 5.0f + dl` (F = `veh->m_matrix` forward row, read without a NULL
        check, 0x42C12F-0x42C15E). If `score < bestScore` (strict, NaN false): `bestScore = score`, `best = {area (u16), i (u16)}`,
        `bestLink = L` (0x42C16D-0x42C185).
  4. `*outNode = best`, `*outLinkNode = bestLink` (whole dwords, 0x42C201-0x42C21C).
- Edge cases: the "node address" stored for the best node is `(area index, node index)` built from the loop counters. If nothing qualifies
  the outputs carry area 0xFFFF and uninitialised node words (callers only test the area). The vehicle's matrix pointer is read once per
  area (0x42BEA5) for the position and again per candidate for the forward row.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:388-390). Open questions: none.

## 0x42C2B0 CCarCtrl::CreatePoliceChase  (catalogue: gr-unreversed, port level 32)
- Convention / signature: `__cdecl bool (CVehicle* veh [esp+4], int32 arg2 [+8], CNodeAddress node [+0xC] (by value))`, returns AL, plain `ret`.
  MSVC SEH frame (handler 0x837486, state slot [E+0xA4]; only unwinds the task `new`). E = esp after `push edi` (0x42C2E6): gz [E+0x10],
  P [E+0x14..0x1C], count (i16) [E+0x20], entity [E+0x24], event object [E+0x28..], CColPoint [E+0x40] (pos.z [E+0x48]), temps [E+0x78..0x90].
  `veh` is the chased (criminal) vehicle; `arg2` is passed on unchanged to SetUpDriverAndPassengersForVehicle (gr name `arg2`).
- Callers: CCarCtrl::GenerateOneRandomCar 0x430050 (1).
- Callees: `rand` 0x821B1E, `_ftol` 0x821B40 (x2), CCarCtrl::ChoosePoliceCarModel 0x421980 (cdecl (1)), GetNewVehicleDependingOnCarModel 0x421440
  (cdecl (model, 1)), CPathNode::GetPosition 0x420A10, CWorld::ProcessVerticalLine 0x5674E0 (x2), vehicle vtable +0 (delete, arg 1) and +0xD4
  (GetHeightAboveRoad), CEntity::GetIsOnScreen 0x534540, CCamera::IsSphereVisible 0x420D40 (thiscall ECX = TheCamera 0xB6F028, (const CVector&,
  float radius)), CWorld::FindObjectsKindaColliding 0x568B80, CPlaceable::GetTopDirection 0x41CCB0, 0x40FEC0 `CVector operator*(const CVector&, float)`
  (cdecl (out, v, s): `out = (v.x*s, v.y*s, v.z*s)`, z product computed first; x2), CCarCtrl::SetUpDriverAndPassengersForVehicle 0x4217C0,
  CMatrix::operator= 0x59BBC0, CPlaceable::SetPosn 0x4241C0, CVehicle::ChangeLawEnforcerState 0x6D2330 (thiscall (1)), CWorld::Add 0x563220,
  CBike::PlaceOnRoadProperly 0x6BEEB0 / CAutomobile::PlaceOnRoadProperly 0x6AF420, CCarAI::AddPoliceCarOccupants 0x41C070 (cdecl (car, true)),
  CTask::operator new 0x61A5A0 (0x24), CTaskComplexCarDriveWander ctor 0x63CB10 (thiscall (car, 0, 10.0f)), CTaskManager::SetTask 0x681AF0
  (thiscall (task, 4, false)), CEventAcquaintancePed ctor 0x4AF820 (thiscall (CPed*)), CEventGroup::Add 0x4AB420 (thiscall (CEvent*, false)),
  event destructor 0x4AF890.
- Globals: CPopCycle::m_NumGangs_Cars 0xC0BC38 (f32), CStreaming::ms_aInfoForModel load state (byte 0x8E4CD0 + model*0x14), ms_modelInfoPtrs 0xA9B0C8,
  ThePaths.m_pPathNodes 0x96F854, TheCamera 0xB6F028. Constants: 0x858B50 = 0.0f, 0x858B14 = 3.0517578e-05f, 0x858B90 = 4.0f, 0x858FEC = 1.0e9f,
  0x85862C = 10.0f; immediates 1000.0f, -1000.0f, 1.0e9f, 0.02f (0x3CA3D70A), 10.0f; vtable 0x858E68 (CEventAcquaintancePedHate).
- Behaviour:
  1. Chance gate (0x42C2B6-0x42C318): if `m_NumGangs_Cars != 0` (`test ah,0x44; jnp`; NaN counts as non-zero) and `arg2` is not in [14, 23]
     (signed): `if (_ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f * 4.0f) != 0) return 0` (1 in 4 passes).
  2. `model = ChoosePoliceCarModel(1)`; return 0 if `model <= -1` or its load state != 1. `car = GetNewVehicleDependingOnCarModel(model, 1)`;
     return 0 if NULL (0x42C31E-0x42C351).
  3. `P = node.GetPosition()` (ThePaths node of `node`); `gz = 1.0e9f`; two ProcessVerticalLine probes from P (+1000 / -1000, flags (1,0,0,0,1,0,NULL))
     exactly as in GenerateOneEmergencyServicesCar step 5 (second hit kept if `|cp.z - P.z| < |gz - P.z|`) (0x42C357-0x42C43A).
  4. If `gz > 1.0e9f` (`test ah,0x41; jne` continues on `<`, equal or NaN): delete `car`, return 0. Original quirk: the "no ground" value 1.0e9f
     itself passes, so a car without ground is NOT discarded here (unlike 0x42BB36, which tests equality) (0x42C43E-0x42C457).
  5. `P.z = car->GetHeightAboveRoad() + gz` (stored); `r = colModel(car)->boundSphere radius (+0x24)` (0x42C472-0x42C49A).
  6. If `!veh->GetIsOnScreen()` and `TheCamera.IsSphereVisible(P, r)`: delete car, return 0 (0x42C49E-0x42C4D1).
  7. `count = FindObjectsKindaColliding(P, r, true, &count, 2, NULL, false, true, true, false, false)`; if `count != 0` delete car, return 0.
  8. Chased vehicle (0x42C50B-0x42C5A1): `veh.status = 3`; `veh->+0x3B9 = 2` (AVOID_CARS); `c = (uint8)_ftol((float)veh->+0x3D0 + 10.0f)`;
     `veh->+0x42E |= 8` (vehicleFlags bit 51 bMadDriver); `veh->+0x3D0 = c`; `veh->m_vecMoveSpeed = (veh->GetTopDirection() * (float)c) * 0.02f`
     (two 0x40FEC0 calls, each result stored f32). `SetUpDriverAndPassengersForVehicle(veh, arg2, 2, true, true, 99)`.
  9. Police car (0x42C5A9-0x42C620): `*car->m_matrix = *veh->m_matrix`; `car->SetPosn(P)`; `car->m_vecMoveSpeed = veh->m_vecMoveSpeed`;
     `car.status = 3`; `car->+0x42C |= 1` (bCreatedAsPoliceVehicle); `car->ChangeLawEnforcerState(1)`; `CWorld::Add(car)`; if `car->+0x590 == 9`
     (VEHICLE_TYPE_BIKE) `CBike::PlaceOnRoadProperly` else `CAutomobile::PlaceOnRoadProperly`; `AddPoliceCarOccupants(car, true)`.
  10. Wanted flags (0x42C625-0x42C66C): `veh->m_pDriver->+0x478 |= 0x800` (ped flag 107 bWantedByPolice; driver not NULL-checked); for
     `i < veh->+0x488` (u8, re-read each iteration) `if (veh->+0x464[i]) passenger->+0x478 |= 0x800`.
  11. `task = new CTaskComplexCarDriveWander(car, 0, 10.0f)` (NULL if the task pool is full); `car->m_pDriver->m_pIntelligence (+0x47C)->taskManager
     (+4).SetTask(task, 4, false)` (driver not NULL-checked) (0x42C66E-0x42C6BC).
  12. Event (0x42C6C1-0x42C72D): `CEventAcquaintancePed ev(veh->m_pDriver)` on the stack; then its vtable is set to 0x858E68
     (CEventAcquaintancePedHate) and the int16 at ev+0xE = 0x451; `car->m_pDriver->m_pIntelligence->+0x68 (event group).Add(&ev, false)`;
     `car->+0x42C |= 0x40` (bNeverUseSmallerRemovalRange); `car->+0x4A6 (i16 m_nExtendedRemovalRange) = 0xFF`; destroy `ev` (vtable reset to
     0x858E68, dtor 0x4AF890). Return 1.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:348-350). Open questions: none.

## 0x42C740 CCarCtrl::CreateConvoy  (catalogue: gr-unreversed, port level 30)
- Convention / signature: `__cdecl bool (CVehicle* leader [esp+4], int32 arg2 [+8])`, returns AL (1 if at least one convoy car was created),
  plain `ret`. MSVC SEH frame (handler 0x83749E; state slot [E+0xBC] only around the task `new`). E = esp inside the loop (after `push esi`,
  0x42C7CB): gz [E+0x18], radius [E+0x1C], i [E+0x20], Q [E+0x24..0x2C], count (i16) [E+0x30], previous car [E+0x34], n [E+0x38], entity [E+0x48],
  CColPoint [E+0x7C] (pos.z [E+0x84]). `arg2` is forwarded to SetUpDriverAndPassengersForVehicle (gr name).
- Callers: CCarCtrl::GenerateOneRandomCar 0x430050 (1).
- Callees: `rand` 0x821B1E, `_ftol` 0x821B40, GetNewVehicleDependingOnCarModel 0x421440 (cdecl (leader model, 1)), CWorld::ProcessVerticalLine 0x5674E0 (x2),
  vtable +0 delete / +0xD4 GetHeightAboveRoad, CWorld::FindObjectsKindaColliding 0x568B80, CPlaceable::GetTopDirection 0x41CCB0, CMatrix::operator= 0x59BBC0,
  CWorld::Add 0x563220, CBike::PlaceOnRoadProperly 0x6BEEB0, CAutomobile::PlaceOnRoadProperly 0x6AF420, SetUpDriverAndPassengersForVehicle 0x4217C0,
  CTask::operator new 0x61A5A0, CTaskComplexCarDriveWander ctor 0x63CB10, CTaskManager::SetTask 0x681AF0, CEntity::RegisterReference 0x571B70.
- Constants: 0x858B14 = 3.0517578e-05f, 0x858628 = 100.0f, 0x858FA0 = 2.5f, 0x858B50 = 0.0f, 0x858FEC = 1.0e9f, 0x858B38 = 0.02f; immediates 1000.0f,
  -1000.0f, 1.0e9f, 10.0f.
- Behaviour:
  1. `created = 0`; `n = (_ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f * 100.0f) > 50) + 2` (2 or 3); `radius = colModel(leader)->boundSphere radius (+0x24)`;
     `leader->+0x45E = leader->+0x20` (forced route seed = entity random seed); `prev = leader` (0x42C75E-0x42C7C5).
  2. For `i = 0 .. n-1` (0x42C7D0-0x42CBAF):
     a. `car = GetNewVehicleDependingOnCarModel(leader->model, 1)`; if NULL continue.
     b. `k = (float)(i + 1)`, `s = radius * 2 + 2.5f` (`fadd st0,st0`, stored f32); `Q = (L.x - (k*F.x)*s, L.y - (k*F.y)*s, L.z - (k*F.z)*s)` (F = leader
        matrix forward, read raw; products stored f32 before the subtraction) (0x42C7E9-0x42C887).
     c. Ground: `gz = 1.0e9f` and the two ProcessVerticalLine probes (+1000 / -1000, flags (1,0,0,0,1,0,NULL), closest hit) as in CreatePoliceChase;
        if `gz > 1.0e9f` delete car and continue (the equality case passes, same quirk as 0x42C44C) (0x42C89F-0x42C94C).
     d. `Q.z = car->GetHeightAboveRoad() + gz`; `FindObjectsKindaColliding(Q, radius, true, &count, 2, NULL, false, true, true, false, false)`;
        if `count != 0` delete car and continue (0x42C951-0x42C989).
     e. First successful car only (`created == 0`): `leader.status = 3`, `leader->+0x3B9 = 2`, `c = (float)leader->+0x3D0`, `D = leader->GetTopDirection()`;
        `leader->m_vecMoveSpeed = ((c*D.x)*0.02f, (c*D.y)*0.02f, (c*D.z)*0.02f)` (0x42C997-0x42CA22).
     f. `*car->m_matrix = *leader->m_matrix`; position = Q (matrix or placement); `car->m_vecMoveSpeed = leader->m_vecMoveSpeed`; `car` status bits
        3-7 = leader's; `car->+0x42B |= 8` (bPartOfConvoy); `CWorld::Add(car)`; `+0x590 == 9` -> CBike::PlaceOnRoadProperly, `== 0` -> CAutomobile::
        PlaceOnRoadProperly, other types: none (0x42CA25-0x42CABA).
     g. `SetUpDriverAndPassengersForVehicle(car, arg2, 0, true, false, 99)`; `task = new CTaskComplexCarDriveWander(car, 0, 10.0f)`;
        `car->m_pDriver->m_pIntelligence->taskManager.SetTask(task, 4, false)` (driver not NULL-checked) (0x42CABF-0x42CB1C).
     h. `car->+0x3D0 = leader->+0x3D0`; `car->+0x3C8 = leader->+0x3C8 - (float)i`, then 0 if negative (NaN kept); `car->+0x3B9 = leader->+0x3B9`;
        `car->+0x3BA = leader->+0x3BA`; `car->+0x20 = leader->+0x20`; `car->+0x45E = leader->+0x45E`; `car->+0x41C = prev`;
        `prev->RegisterReference(&car->+0x41C)`; `prev = car`; `created = 1` (0x42CB21-0x42CB94).
  3. Return `created`.
- Edge cases: each convoy car follows the previously created one (chain), the first follows the leader. A failed slot still advances `i`, so
  the spacing multiplier `k` keeps growing.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:343-345). Open questions: none.

## 0x42CE40 CCarCtrl::ScanForPedDanger  (catalogue: gr-unreversed, port level 10)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret`. Frame `sub esp,0x24` + 4 pushes; E = esp after the
  pushes (0x42CE47): minX [E+0x24], maxX [E+0x1C], minY [E+0x20], maxY [E+0x18], sector temp / endY [E+0x14], startX copy [E+0x10],
  y counter [E+0x28], curSpeed (f32, passed by pointer) [E+0x30]; the arg slot [E+0x38] is overwritten with a saved flag bit.
- Callers: CAutomobile::ProcessControl 0x6B1880, CBike::ProcessControl 0x6B9250, CTrain::ProcessControl 0x6F86A0.
- Callees: CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (-1, 0)), `floor` 0x8219F0 (double in, ST0 out, x8), `_ftol` 0x821B40 (x8),
  CWorld::ClearScanCodes 0x563470, CCarCtrl::SlowCarDownForPedsSectorList 0x425440 (cdecl, 8 args, see step 5).
- Globals: CWorld::ms_nCurrentScanCode 0xB7CD78 (u16, RW), CWorld::ms_aRepeatSectors 0xB992B8 (16x16 CRepeatSector, 12 bytes each =
  3 list heads; +4 = the peds list, hence the table address 0xB992BC). Constants: 0x858FF8 = 44.0f, 0x858FBC = 11.0f, 0x858B38 = 0.02f,
  0x858B34 = 60.0f.
- Fields: veh +0x14/+0x04 position, +0x3D0 m_autoPilot.m_nCruiseSpeed (u8), +0x3DB m_autoPilot.m_nCarCtrlFlags (bit 1, gr name
  bHonkAtPed; III name m_bSlowedDownBecauseOfPeds), +0x429 bit 5 (vehicleFlags bit 13 bWarnedPeds).
- Behaviour:
  1. `r = (veh == FindPlayerVehicle(-1, false)) ? 44.0f : 11.0f` (kept in ST0, 0x42CE49-0x42CE64).
     `saved = (veh->+0x3DB >> 1) & 1` stored in the arg slot (0x42CE6A-0x42CE74).
  2. With P = veh position: `minX = P.x - r`, `maxX = r + P.x`, `minY = P.y - r`, `maxY = r + P.y` (each stored f32, 0x42CE87-0x42CEC4).
  3. Sector bounds, each with the `GetSector` macro shape (`t = v * 0.02f + 60.0f` computed in x87 and stored f32 (fst), the x87 value
     converted to double and floored, `_ftol`; if the clamp test passes, `floor((double)t)` and `_ftol` are evaluated again from the
     stored f32):
     `startX = (k = ftol(floor(minX*0.02+60))) > 0 ? k : 0` (0x42CEC8-0x42CF13, esi and [E+0x10]);
     `startY = ... minY ... > 0 ? k : 0` (0x42CF15-0x42CF57, ebp);
     `endX = (k = ...maxX...) < 119 ? k : 119` (0x42CF59-0x42CFA0, ebx); `endY = (k = ...maxY...) < 119 ? k : 119` (0x42CFA5-0x42CFEE, [E+0x14]).
     Compares are signed int (`jle 0` / `cmp eax,0x77; jge`).
  4. Scan code: `if (ms_nCurrentScanCode >= 0xFFFF) { ClearScanCodes(); ms_nCurrentScanCode = 1; } else ms_nCurrentScanCode++`
     (unsigned word compare, 0x42CFF6-0x42D00F).
  5. `curSpeed = (float)veh->m_nCruiseSpeed` (fild of the zero-extended byte, stored f32 [E+0x30], 0x42D018-0x42D02F).
     `for (y = startY; y <= endY; y++) for (x = startX; x <= endX; x++)` (signed, 0x42D023-0x42D09F; outer test `jg` before the first
     pass, inner test `cmp esi,ebx; jg`):
     `SlowCarDownForPedsSectorList(&ms_aRepeatSectors[(y & 15)*16 + (x & 15)].lists[1] (= 0xB992BC + idx*12), veh, minX, minY, maxX,
     maxY, &curSpeed, (float)veh->m_nCruiseSpeed)` (the last arg re-read and converted for every call, 0x42D040-0x42D07F).
  6. `veh->+0x429 |= 0x20` (bWarnedPeds = true, 0x42D0A1-0x42D0AF). Then bit 1 of `veh->+0x3DB` is set back to `saved`
     (0x42D0B5-0x42D0C3: `cl = bl ^ (((saved << 1) ^ bl) & 2)`), i.e. any change the sector scans made to that bit is undone.
- Edge cases: the computed curSpeed is not used after the loops (only the callees read/write it through the pointer). The sector
  indices wrap with `& 15` (repeat sectors).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:841-843). Note: gr's P1-documented SlowCarDownForPedsSectorList sets bit 1 of
  +0x3DB (0x4258D0); the restore in step 6 means that, after ScanForPedDanger, the bit always has its pre-call value.
- Open questions: none.

## 0x42D0E0 CCarCtrl::SlowCarDownForOtherCar  (catalogue: gr-unreversed, port level 3)
- Convention / signature: `__cdecl void (CEntity* other [esp+4], CVehicle* veh [+8], float* pSpeed [+0xC], float cruise [+0x10])`,
  plain `ret`. `other` is used as a CVehicle (+0x3B0 read) only after its type was checked to be a vehicle in step 9; earlier it
  is read as CPhysical (+0x44/+0x48). Frame `sub esp,0x30` + 4 pushes; E = esp after the pushes (0x42D0E7): D (veh unit forward)
  [E+0x1C]/[E+0x20], OD (other unit forward) [E+0x34]/[E+0x38] (with [E+0x30] = [E+0x3C] = 0 written but never read),
  relX [E+0x10], arg slots reused: [E+0x44] (other) = D.x*cruise then relY, [E+0x48] (veh) = pointer to other's position,
  then other.mv.x*60, then t, then the matrix pointer and the forward dot.
- Callers: CCarCtrl::SlowCarDownForCarsSectorList 0x432420 (1). Callees: AllocateMatrix 0x54F560 + UpdateMatrix 0x54F1B0
  (ensure `other`'s matrix, 2 sites), TestCollisionBetween2MovingRects_OnlyFrontBumper 0x425F70 and TestCollisionBetween2MovingRects
  0x425B30 (both cdecl, 6 args, ST0 result; see CCarCtrl.md), CEntity::RegisterReference 0x571B70 (thiscall ECX = other, arg
  CEntity**), CPlaceable::GetMatrix 0x411990 (thiscall, ensures the matrix, returns it), CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (-1, 0)),
  CCarCtrl::SwitchVehicleToRealPhysics 0x423FC0 (cdecl (veh)).
- Globals: CTimer::m_snTimeInMilliseconds 0xB7CB84. Constants (f32): 0x858B50 = 0.0, 0x858624 = 1.0, 0x858B34 = 60.0, 0x858CE8 = 1.5,
  0x858B3C = 3.0, 0x858CC4 = 0.2, 0x858FFC = 0.7692308 (1/1.3), 0x858B8C = 0.5, 0x858F40 = -0.5.
- Fields: entity +0x14 m_matrix (fwd.x +0x10, fwd.y +0x14, pos +0x30), +0x04 placement pos, +0x36 type bits 0-2 / status bits 3-7;
  physical +0x44/+0x48 m_vecMoveSpeed.x/.y; vehicle +0x3B0 m_autoPilot.m_nTimeSwitchedToRealPhysics (u32), +0x3B9
  m_nCarDrivingStyle (u8), +0x3BC m_nTempActionTime (u32), +0x3DB m_nCarCtrlFlags (bit 0, gr bHonkAtCar; III
  m_bSlowedDownBecauseOfCars), +0x420 m_ObstructingEntity.
- Behaviour:
  1. `vm = veh->m_matrix` (no null check, 0x42D0EB). `len = sqrt(vm.fwd.y*vm.fwd.y + vm.fwd.x*vm.fwd.x)`; if `len == 0` (`test ah,0x44;
     jnp`, NaN normalises) `D = (1.0, vm.fwd.y)`, else `inv = 1.0f / len`, `D.x = inv * vm.fwd.x`, `D.y = inv * vm.fwd.y` (stored f32;
     0x42D0EE-0x42D12F).
  2. `O = other pos`, `V = veh pos`: if `((O.y - V.y) * D.y) + ((O.x - V.x) * D.x) < 0` return (0x42D172-0x42D196; `test ah,5; jnp`,
     equal/NaN continue).
  3. `omx = other.mv.x * 60.0f` [E+0x48], `omy = other.mv.y * 60.0f` [E+0x14], `cx = D.x * cruise` [E+0x44], `cy = D.y * cruise` [E+0x18]
     (each stored f32, 0x42D19E-0x42D1D4).
  4. Ensure `other`'s matrix (twice, 0x42D1D8-0x42D202); `om = other->m_matrix`. `OD` = unit 2D forward of `om` exactly as in step 1
     (`len2 = sqrt(om.fwd.y^2 + om.fwd.x^2)`, zero -> `OD = (1.0, om.fwd.y)`, 0x42D207-0x42D273).
  5. `relX = omx - cx`, `relY = omy - cy` (stored f32). `t1 = TestCollisionBetween2MovingRects_OnlyFrontBumper(other, veh, relX, relY,
     &D, &OD)` (0x42D2A9, stored f32); `t2 = TestCollisionBetween2MovingRects(veh, other, -relX, -relY, &OD, &D)` (0x42D2D7, negations
     with fchs of the f32 values). `t = (t1 < t2) ? t1 : t2` (0x42D2DC-0x42D2F2; equal or NaN -> t2), stored f32.
  6. If `t < 0` return (`test ah,1`, NaN returns too, 0x42D2F4-0x42D303).
  7. If `t < 1.5f` (0x42D309-0x42D318):
     `veh->+0x3DB |= 1`; `veh->+0x420 = other`; `other->RegisterReference(&veh->+0x420)` (0x42D31E-0x42D338).
     `q = 1.0f / cruise` (x87):
     a. if `t < q`: `*pSpeed = 0` (0x42D347-0x42D35A).
     b. else if `t < q * 3.0f` (fcompp, 0x42D362-0x42D377): `*pSpeed = (*pSpeed < 1.0f) ? *pSpeed : 1.0f` (equal/NaN -> 1.0f, 0x42D379-0x42D3DC).
     c. else: `k = (t - 0.2f) * 0.7692308f`; `t = (0.0f > k) ? 0.0f : k` (stored back to t! 0x42D394-0x42D3BF; NaN/equal keep k);
        `v = t * cruise`; `*pSpeed = (v > *pSpeed) ? *pSpeed : v` (`test ah,0x41; jne` -> v stored when v < *pSpeed, equal or NaN;
        0x42D3C3-0x42D3DC).
  8. (always, using the possibly modified t) if `t < 0` return (`test ah,1`); if `!(t < 0.5f)` return (0x42D3DE-0x42D402).
  9. If `other` type (`+0x36 & 7`) != 2 return. If `now - veh->+0x3B0 <= 15000` or `now - other->+0x3B0 <= 15000` (unsigned) return
     (0x42D408-0x42D43D).
  10. `vm = veh->m_matrix` (raw), `om = other->GetMatrix()` (twice; 0x42D443-0x42D458); `dotF = om.fwd.y * vm.fwd.y + om.fwd.x * vm.fwd.x`
     (stored f32). If `other == FindPlayerVehicle(-1, false)` return. If `!(dotF < -0.5f)` return. If `veh >= other` (unsigned pointer
     compare, 0x42D494) return - only the vehicle with the lower address acts.
  11. `v = cruise * 0.2f`; `*pSpeed = (v < *pSpeed) ? *pSpeed : v` (i.e. raised to at least 20% of cruise; equal/NaN -> v;
     0x42D498-0x42D4B3). If `(veh->+0x36 & 0xF8) == 0x10` (status 2 STATUS_SIMPLE) `SwitchVehicleToRealPhysics(veh)`.
     `veh->+0x3B9 = 2` (DRIVING_STYLE_AVOID_CARS); `veh->+0x3BC = now + 1000` (0x42D4C9-0x42D4DA).
- Edge cases / original quirks: branch 7c overwrites `t`, so step 8's `t < 0.5` test uses the remapped value; `cruise == 0` gives
  `q = inf` (then 7a always zeroes the speed). `veh->m_matrix` is dereferenced without allocation (0x42D0EB, 0x42D443, 0x42D44F).
  The pointer compare in step 10 makes the deadlock breaker depend on pool slot addresses (keep).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:895-898). Open questions: none.

## 0x42D4F0 CCarCtrl::SlowCarDownForObjectsSectorList<CPtrListDoubleLink<CPhysical*>>  (catalogue: gr-unreversed, port level 3)
- Convention / signature: `__cdecl void (CPtrListDoubleLink* list [esp+4], CVehicle* veh [+8], float minX [+0xC], float minY [+0x10],
  float maxX [+0x14], float maxY [+0x18], float* pSpeed [+0x1C], float cruise [+0x20])`, plain `ret`. Frame `sub esp,0x18` + 4 pushes;
  E = esp after the pushes: bound centre C [E+0x10..0x18], direction temp [E+0x1C..0x24]; the list arg slot [E+0x2C] is reused for `d`.
- Callers: CCarCtrl::FindMaximumSpeedForThisCarInTraffic 0x434400 (1). Callees: CEntity::GetBoundCentre 0x534250 (thiscall, out
  CVector*), CCollision::DistAlongLine2D 0x412A80 (cdecl (px, py, dirX, dirY, qx, qy) -> ST0), CPlaceable::GetTopDirection 0x41CCB0
  (thiscall (CVector* out) `ret 4`, returns `out` in EAX; copies matrix +0x10 (the forward row) or, without a matrix, `(-sin h, cos h, 0)`),
  CCarCtrl::SlowCarDownForObject 0x426220 (cdecl (CEntity*, CVehicle*, float*, float)).
- Globals: CWorld::ms_nCurrentScanCode 0xB7CD78 (u16), MI_ROADWORKBARRIER1 0x8CD57C, MI_ROADBLOCKFUCKEDCAR1 0x8CD580,
  MI_ROADBLOCKFUCKEDCAR2 0x8CD584 (u16). Constants: 0x85862C = 10.0f, 0x858B50 = 0.0f, 0x858B3C = 3.0f.
- Fields: list node +0 item, +4 next; entity +0x2C m_nScanCode (u16), +0x22 model (i16); veh +0x14 m_matrix (fwd +0x10/+0x14, pos +0x30), +0x04.
- Behaviour: `for (node = list->head; node; node = next)` (next read before processing, 0x42D51D):
  1. `e = node->item`; if `e->m_nScanCode == ms_nCurrentScanCode` skip; else store the current code (0x42D513-0x42D526).
  2. `m = (int32)(int16)e->model`; continue only if `m` equals (zero-extended) MI_ROADWORKBARRIER1, MI_ROADBLOCKFUCKEDCAR1 or
     MI_ROADBLOCKFUCKEDCAR2 (0x42D52A-0x42D54D).
  3. `C = e->GetBoundCentre()`. Require `C.x > minX` (`test ah,0x41; jne` -> equal/NaN fail), `C.x < maxX`, `C.y > minY`, `C.y < maxY`
     (0x42D55F-0x42D5A5).
  4. `V = veh pos`. Require `fabs(C.z - V.z) < 10.0f` (0x42D5B8-0x42D5CC).
  5. `d = DistAlongLine2D(V.x, V.y, vm.fwd.x, vm.fwd.y, C.x, C.y)` (vm = veh->m_matrix read raw: if it is NULL the code reads
     `[0x10]`/`[0x14]` -> crash; 0x42D5D2-0x42D5FC), stored f32.
  6. `P = veh pos`, `F = veh->GetTopDirection()` (matrix forward row); `h = C.z - (d * F.z + P.z)` (fmul, fadd, fsubr; 0x42D621-0x42D62B);
     require `fabs(h) < 3.0f` (0x42D62F-0x42D649; strict, NaN fails).
  7. `SlowCarDownForObject(e, veh, pSpeed, cruise)` (0x42D657).
- Edge cases: the bounds are strict; NaN centre coordinates are rejected. Scan codes are written before the model filter, so a
  non-matching entity is still marked as visited.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:890-892). Open questions: none.

## 0x42D680 CCarCtrl::WeaveThroughCarsSectorList<CPtrListDoubleLink<CPhysical*>>  (catalogue: gr-unreversed, port level 3)
- Convention / signature: `__cdecl void (CPtrListDoubleLink* list [esp+4], CVehicle* veh [+8], CPhysical* exclude [+0xC], float minX [+0x10],
  float minY [+0x14], float maxX [+0x18], float maxY [+0x1C], float* angleA [+0x20], float* angleB [+0x24])`, plain `ret`.
  Frame `sub esp,0xC` (bound centre C at [E+0x10..0x18], E = esp after the 4 pushes) + ebx/ebp/esi/edi.
- Callers: CCarCtrl::FindAngleToWeaveThroughTraffic 0x4325C0 (2 sites). Callees: CEntity::GetBoundCentre 0x534250 (thiscall, out
  CVector*), CCarCtrl::WeaveForOtherCar 0x426350 (cdecl (CEntity*, CVehicle*, float*, float*)).
- Globals: CWorld::ms_nCurrentScanCode 0xB7CD78 (u16). Constants: 0x858B50 = 0.0f, 0x859000 = 8.0f.
- Fields: entity +0x1C bit 0 m_bUsesCollision, +0x2C m_nScanCode (u16), +0x14/+0x04 position; vehicle +0x42B bit 3 (vehicleFlags bit 27
  bPartOfConvoy).
- Behaviour: `for (node = list->head; node; node = next)` (next read first, 0x42D6AC); `e = node->item`:
  1. Skip if `e->m_nScanCode == ms_nCurrentScanCode`, if `!(e->+0x1C & 1)`, or if `e == exclude` (0x42D6A8-0x42D6C3); only then
     `e->m_nScanCode = ms_nCurrentScanCode` (0x42D6D0).
  2. `C = e->GetBoundCentre()`; require `C.x > minX`, `C.x < maxX`, `C.y > minY`, `C.y < maxY` (strict; `test ah,0x41; jne` for the
     `>` tests and `test ah,5; jp` for the `<` tests, NaN fails all; 0x42D6D9-0x42D71F).
  3. `dz = E.z - V.z` (entity position z minus veh position z); `adz = (dz < 0) ? -(E.z - V.z) : dz` (recomputed from memory before the
     fchs; NaN takes the non-negated path); require `adz < 8.0f` (0x42D73F-0x42D799).
  4. Skip if `e == veh` (0x42D79B), or if `(veh->+0x42B & 8) && (e->+0x42B & 8)` (both part of a convoy, 0x42D79F-0x42D7AF).
  5. `WeaveForOtherCar(e, veh, angleA, angleB)` (0x42D7BD).
- Edge cases: the `e == veh` test comes after the geometric tests (so `veh` itself gets its scan code set). `e` is assumed to be a
  vehicle for the +0x42B read (vehicle sector lists only).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp, `WeaveThroughCarsSectorList`). Open questions: none.

## 0x42D7E0 CCarCtrl::WeaveThroughPedsSectorList<CPtrListDoubleLink<CPhysical*>>  (catalogue: gr-unreversed, port level 2)
- Convention / signature: `__cdecl void (CPtrListDoubleLink* list [esp+4], CVehicle* veh [+8], CPhysical* exclude [+0xC], float minX [+0x10],
  float minY [+0x14], float maxX [+0x18], float maxY [+0x1C], float* angleA [+0x20], float* angleB [+0x24])`, plain `ret`; no locals
  (ebx/ebp/esi/edi pushed).
- Callers: CCarCtrl::FindAngleToWeaveThroughTraffic 0x4325C0 (1). Callees: 0x426970 CCarCtrl::WeaveForPed (proposed name; spec in
  `_unattributed-P2.md`), cdecl (CPed*, CVehicle*, float*, float*).
- Globals: CWorld::ms_nCurrentScanCode 0xB7CD78. Constants: 0x858B50 = 0.0f, 0x858B90 = 4.0f.
- Fields: entity +0x1C bit 0 m_bUsesCollision, +0x2C m_nScanCode, position; ped +0x584 m_pContactEntity, +0xFC m_pAttachedTo (CPhysical).
- Behaviour: list walk as in WeaveThroughCarsSectorList (next read first); `p = node->item`:
  1. Skip if scan code equal, `!(p->+0x1C & 1)`, or `p == exclude` (0x42D802-0x42D823); then set the scan code (0x42D82E).
  2. Uses the entity POSITION (no bound centre): require `P.x > minX`, `P.x < maxX`, `P.y > minY`, `P.y < maxY` (strict, NaN fails;
     0x42D83A-0x42D898).
  3. `|P.z - V.z| < 4.0f` with the same sign/recompute pattern as the cars version (0x42D8B5-0x42D90F).
  4. Skip if `p->+0x584 == veh` (standing on the vehicle) or `p->+0xFC == veh` (attached to it) (0x42D911-0x42D91F).
  5. `WeaveForPed(p, veh, angleA, angleB)` (0x42D92D).
- gta-reversed: `plugin::Call` only. Open questions: none.

## 0x42D950 CCarCtrl::WeaveThroughObjectsSectorList<CPtrListDoubleLink<CPhysical*>>  (catalogue: gr-unreversed, port level 3)
- Convention / signature: `__cdecl void (CPtrListDoubleLink* list [esp+4], CVehicle* veh [+8], float minX [+0xC], float minY [+0x10],
  float maxX [+0x14], float maxY [+0x18], float* angleA [+0x1C], float* angleB [+0x20])`, plain `ret` (no exclude argument).
- Callers: CCarCtrl::FindAngleToWeaveThroughTraffic 0x4325C0 (1). Callees: CPlaceable::GetMatrix 0x411990 (thiscall, allocates the
  matrix if needed, returns it), CCarCtrl::WeaveForObject 0x426BC0 (cdecl (CEntity*, CVehicle*, float*, float*)).
- Globals: CWorld::ms_nCurrentScanCode 0xB7CD78. Constants: 0x858B50 = 0.0f, 0x859000 = 8.0f, 0x858C20 = 0.9f.
- Behaviour: list walk (next read first); `o = node->item`:
  1. Skip if scan code equal or `!(o->+0x1C & 1)` (0x42D972-0x42D989); then set the scan code (0x42D994).
  2. Position bounds on `o`'s position, strict as above (0x42D9A0-0x42D9FE).
  3. `|O.z - V.z| < 8.0f` (same pattern, 0x42DA1B-0x42DA75).
  4. Require `o->GetMatrix()->up.z (+0x28) > 0.9f` (`test ah,0x41; jne` -> equal/NaN skip; 0x42DA77-0x42DA8C): only upright objects.
  5. `WeaveForObject(o, veh, angleA, angleB)` (0x42DA9A).
- gta-reversed: `plugin::Call` only. Open questions: none.

## 0x42DAB0 CCarCtrl::IsThisAnAppropriateNode  (catalogue: gr-unreversed, port level 5)
- Convention / signature: `__cdecl bool (CVehicle* veh [esp+4], CNodeAddress n1 [+8], CNodeAddress n2 [+0xC], CNodeAddress n3 [+0x10],
  bool wrongWay [+0x14])`, returns AL (0/1), plain `ret`. The node addresses are passed by value (dword: area u16 low, node u16 high).
  Semantics (from the caller 0x42E18B): n1 = previous node, n2 = current node, n3 = candidate next node, `wrongWay` = the candidate
  link has no lanes in the travel direction (gr `arg5`). The caller pushes a 6th dword that is ignored. Frame `sub esp,0x88` + 4 pushes.
- Callers: CCarCtrl::PickNextNodeRandomly 0x42DE80 (1).
- Callees: CPathNode::GetPosition 0x420A10 (thiscall (CVector* out), `ret 4`, returns out; `(x,y,z) = (i16 +8, +0xA, +0xC) * 0.125f`),
  VectorSub 0x40FE60 (cdecl (out, a, b): `out = a - b` per component), CVector::CrossProduct 0x59C730 (cdecl (out, a, b): `a x b`; only
  `z = a.x*b.y - b.x*a.y` (0x59C761-0x59C777) is used here),
  CPlaceable::GetRightDirection 0x41CC70 (thiscall (CVector* out), returns out), CVector::DotProduct 0x40FDB0 (cdecl (a, b) -> ST0:
  `(a.z*b.z + a.y*b.y) + a.x*b.x`), CVehicle::IsLawEnforcementVehicle 0x6D2370, CCarCtrl::IsAnyoneParking 0x42C250,
  CWorld::FindObjectsKindaColliding 0x568B80 (cdecl, 11 args).
- Globals: ThePaths.m_pPathNodes 0x96F854, CModelInfo::ms_modelInfoPtrs 0xA9B0C8. Constants: 0x858B50 = 0.0f, 0x858CE8 = 1.5f,
  0x858CA0 = 2.0f, 0x858B90 = 4.0f, immediate 5.0f (0x40A00000).
- Fields: veh +0x22 model (i16), +0x4A4 m_nCreatedBy (u8, 2 = MISSION_VEHICLE), +0x460 m_pDriver, +0x3BA mission (u8); ped +0x484
  m_nCreatedBy (u8, 2 = mission), +0x598 m_nPedType (i32; 0x14 = PED_TYPE_CRIMINAL); CVehicleModelInfo +0x4D m_nVehicleClass (i8,
  5 = VEHICLE_CLASS_BIG), +0x14 colmodel (bbox max x/y/z at +0x0C/+0x10/+0x14). CPathNode flags dword +0x18: byte +0x18 bit 4
  m_onDeadEnd, bit 5 m_isSwitchedOff, bit 7 m_bWaterNode; byte +0x19 bit 2 (dword bit 10) m_bDontWander; byte +0x1A high nibble
  m_nBehaviourType.
- Behaviour (N2 = node n2, N3 = node n3, f2/f3 = their flag byte +0x18):
  1. If `m_pPathNodes[n3.area] == NULL` or `n1 == n3` (both words) return 0 (0x42DAB0-0x42DAEC).
  2. If `(f2 ^ f3) & 0x80` (water/non-water transition) and `veh model != 0x21B` (539, Vortex) return 0 (0x42DB1E-0x42DB34).
  3. Switch on `N3 behaviourType` (byte +0x1A >> 4): `t-1` > 9 -> default; byte table 0x42DE68 {0,1,4,4,2,4,4,3,3,2}, jump table
     0x42DE54 {A 0x42DB59, B 0x42DCA1, C 0x42DDDE, D 0x42DD88, default 0x42DE07}: t=1 -> A, t=2 -> B, t=5,10 -> C, t=8,9 -> D,
     t=0,3,4,6,7,>10 -> default.
     Case A (0x42DB59): return 0 if `veh->m_nCreatedBy == 2`, if the driver exists and `driver->+0x484 == 2`, or if vehicle class == 5.
       If `!wrongWay`: `a = N3pos - N2pos`, `b = N3pos - Vpos` (VectorSub); `c = Cross(a, b)` (0x42DBFB); return 0 if `c.z < 0`
       (`test ah,5; jnp`, NaN passes). Then (shared tail) return 0 if IsLawEnforcementVehicle(veh), if class == 5 (again), if the driver
       exists and `driver->m_nPedType == 0x14`, or if `IsAnyoneParking()`. Finally `count = 0 (i16)`;
       `FindObjectsKindaColliding(N3pos, 5.0f, true, &count, 2, NULL, false, true, false, false, false)` (vehicles only, 0x42DC85)
       and return `count == 0` (0x42DC8D-0x42DC96).
     Case B (0x42DCA1): return 0 if mission-created vehicle or mission-created driver (no class test here). If `!wrongWay`:
       `d = N3pos - Vpos`, `r = veh->GetRightDirection()`, return 0 if `DotProduct(r, d) < 0` (0x42DD08-0x42DD1B). Then the same tail as A
       (law enforcement, class 5, criminal driver, IsAnyoneParking, FindObjectsKindaColliding around N3 with radius 5) -> `count == 0`.
     Case C (0x42DDDE): if `wrongWay` return 0; return 0 if `colmodel.bbox.max.y < 2.0f` (`test ah,5; jnp`); else default.
     Case D (0x42DD88): if `wrongWay` return 0; return 0 if `max.y > 1.5f`, if `max.x > 2.0f`, or if `max.z > 4.0f` (`test ah,0x41`,
       NaN passes each); else default.
     Default (0x42DE07): if `veh->m_nCreatedBy == 2` and `veh->mission == 1` (CRUISE) and `N3 dontWander` and `!N2 dontWander` return 0;
       if `(f3 & 0x10) && !(f2 & 0x10)` return 0 (entering a dead end); if `(f3 & 0x20) && !(f2 & 0x20)` return 0 (entering switched-off
       nodes); return `!wrongWay` (`sete`, 0x42DE3B-0x42DE49).
- Edge cases: with `wrongWay = true` the default path and cases C/D return 0; only parking-type nodes (behaviour 1/2, cases A/B) can
  still be accepted, and then without the geometric side test. `m_pPathNodes[n2.area]` is not NULL-checked (0x42DB05). The parking check scans the whole vehicle pool.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:582-584). Open questions: none.

## 0x42DE80 CCarCtrl::PickNextNodeRandomly  (catalogue: gr-unreversed, port level 6)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret`. Frame `sub esp,0x44` + esi/ebx/ebp/edi; E = esp after the
  4 pushes (0x42DEC8). Locals: [E+0x11] byte curOneWay, [E+0x12] byte sharpTurn (FindPathDirection out), [E+0x13] byte dirCode,
  [E+0x14] loop counter, [E+0x18] oldCur (dword CNodeAddress), [E+0x1C] oldNext (dword; later reused as a float), [E+0x20] numLinks
  (later a float), [E+0x24] offset `oldNext.node*0x1C`, [E+0x28]/[E+0x2C] candidate lane flags (later floats), [E+0x30..0x38] P2,
  [E+0x3C..0x44] P1. The veh arg slot [E+0x58] is reused: turn mask byte (loop 1), then the candidate link (u16), then scratch.
- Callers: CCarCtrl::GenerateOneRandomCar 0x430050, CCarCtrl::PickNextNodeAccordingStrategy 0x432B10.
- Callees: `srand` 0x821B11, CCarCtrl::ThisVehicleShouldTryNotToTurn 0x421FE0 (cdecl (veh) -> AL), CCarCtrl::InitSequence 0x421740 (cdecl
  (int n): `SequenceElements = n; SequenceRandomOffset = rand() % n; bSequenceOtherWay = (rand() / 4) % 2`), CCarCtrl::FindPathDirection
  0x422090 (cdecl (a, b, c, bool*) -> AL: 0 invalid, 1 straight, 2 / 4 turn; see CCarCtrl.md), CCarCtrl::IsThisAnAppropriateNode 0x42DAB0,
  CAutoPilot::ModifySpeed 0x41B980 (thiscall ECX = &veh->m_autoPilot (+0x390), float arg; pushed as `push 0` = 0.0f), CCarPathLink::
  OneWayLaneOffset 0x44DB00 (thiscall ECX = navi link, -> ST0), CCurves::CalcSpeedScaleFactor 0x43C710 (cdecl (CVector* p1, CVector* p2,
  float d1x, float d1y, float d2x, float d2y) -> ST0), `rand` 0x821B1E, `_ftol` 0x821B40.
- Globals: ThePaths m_pPathNodes 0x96F854, m_pNaviNodes 0x96F974, m_pNodeLinks 0x96FA94, m_pNaviLinks 0x96FDF4; CCarCtrl::bSequenceOtherWay
  0x969070 (u8), SequenceRandomOffset 0x969074, SequenceElements 0x969078 (i32); CTimer::m_snTimeInMilliseconds 0xB7CB84.
  Constants: 0x858C58 = 0.01f, 0x858C48 = 0.125f, 0x858FB4 = 256.0f, 0x858C50 = 5.4f, 0x858C4C = 1000.0f.
- Fields (vehicle, gr CAutoPilot names; note the real roles): +0x390 m_currentAddress (current node), +0x394 m_startingRouteNode (actually
  the NEXT node), +0x398 m_endingRouteNode (actually the PREVIOUS node), +0x39C field_C (time the current curve was entered), +0x3A0
  m_nSpeedScaleFactor (curve duration, u32), +0x3A4/+0x3A6/+0x3A8 current/next/previous link (CCarPathLinkAddress u16: area = v >> 10,
  index = v & 0x3FF), +0x3B4/+0x3B5/+0x3B6 _smthPrev/_smthCurr/_smthNext (i8 link direction +-1), +0x3B7 m_nCurrentLane, +0x3B8
  m_nNextLane (i8), +0x3B9 m_nCarDrivingStyle, +0x3BA m_nCarMission, +0x3BB m_nTempAction, +0x3BC m_nTempActionTime, +0x3AC
  m_nTimeToStartMission, +0x3B0 m_nTimeSwitchedToRealPhysics, +0x3C8 m_speed (f32), +0x3DB m_nCarCtrlFlags (0x08 bStayInFastLane,
  0x10 bStayInSlowLane), +0x3E0 field_50 (u8 lane-change countdown), +0x45E m_nForcedRandomRouteSeed, +0x4A4 m_nCreatedBy, +0x36 status
  bits 3-7, +0xB8 m_nFakePhysics (u8). CPathNode / CCarPathLink layouts as in FindLinksToGoWithTheseNodes; CCarPathLink +0x08/+0x09 dir
  x/y (i8, *0.01), +0x0B bits 0-2 (call it LA) and bits 3-5 (LB) lane counts.
  "Lanes of link L seen from node X": if `L.attachedTo == X` (words +4/+6) our lanes = LA and the other side = LB, else ours = LB, other = LA.
- Behaviour:
  1. Setup (0x42DE80-0x42DF16): if `veh->+0x45E != 0` `srand(seed)`; `[E+0x11] = 0`. `oldCur = +0x390`, `oldNext = +0x394`,
     `lk = +0x3A6`. Return immediately if `m_pPathNodes[oldCur.area]`, `m_pPathNodes[oldNext.area]` or `m_pPathNodes[lk >> 10]` is NULL
     (0x42DEB7, 0x42DED2, 0x42DEEE). `N = node oldNext`, `numLinks = N.flags & 0xF`.
  2. Current link lanes (0x42DF1A-0x42DFCE): `L = navi[lk]`; if `L.attachedTo == oldNext`: `nLanes = LA`, `curOneWay = (LB == 0)`; else
     `nLanes = LB`, `curOneWay = (LA == 0)`.
  3. Turn mask (0x42DFD3-0x42E006): `lane = (i8)veh->+0x3B8`; `mask = (lane == 0) ? 4 : 0`; `if (lane == nLanes - 1) mask |= 2`;
     `if (nLanes < 3 || mask == 0) mask |= 1`.
  4. Advance (0x42E00A-0x42E01D): `+0x398 = oldCur`; `+0x390 = oldNext`. If `ThisVehicleShouldTryNotToTurn(veh)` `mask = 1` (0x42E023-0x42E02F).
  5. `InitSequence(numLinks)` (0x42E039; called before the `numLinks > 0` test, so `numLinks == 0` divides by zero inside InitSequence -
     original crash, keep). Sequence index for loop counter i: `idx = bSequenceOtherWay ? (SequenceRandomOffset + i) % SequenceElements
     : (SequenceElements - i + SequenceRandomOffset) % SequenceElements` (signed idiv remainder; recomputed the same way in all 3 loops).
     For each i the candidate is `cand = m_pNodeLinks[N.area][N.base + idx]`, `candLink = m_pNaviLinks[N.area][N.base + idx]`, and
     `+0x394 = cand` is written immediately (so after a failed search +0x394 holds the last candidate tried until step 9 overwrites it).
  6. Loop 1 (0x42E043-0x42E1C8), i = 0..numLinks-1: skip if `m_pPathNodes[cand.area] == NULL`. `dirCode = FindPathDirection(oldCur,
     +0x390 (= oldNext), cand, &sharpTurn)`. Skip if `m_pPathNodes[candLink >> 10] == NULL`. `wrongWay = 0, otherNone = 0`; skip if status
     == 2 (STATUS_SIMPLE, `(+0x36 & 0xF8) == 0x10`) and `sharpTurn`. Lanes of navi[candLink] seen from oldNext: `wrongWay = (ours == 0)`,
     `otherNone = (other == 0)` (0x42E12A-0x42E174). Then `IsThisAnAppropriateNode(veh, oldCur, oldNext, cand, wrongWay)` (6 dwords pushed,
     the 6th = otherNone is ignored by the callee; the bool args are pushed as whole dwords whose upper bytes are stale) (0x42E18B).
     Accept (goto 10 with `newLink = candLink`) if it returned 1, `(mask & dirCode) != 0`, and (`!curOneWay` or `!otherNone`) (0x42E193-0x42E1B3).
  7. Loop 2 (0x42E1D2-0x42E330), i = 0..numLinks-1: skip NULL areas as above (`candLink` saved in [E+0x58]); `wrongWay` as in loop 1;
     skip if `cand == oldCur` or `wrongWay`; accept (goto 10 with `newLink = [E+0x58]`) if `!(candNode.flags & 0x20)` (not switched off) or
     `(oldCurNode.flags & 0x20)` (coming from a switched-off node) (0x42E2DB-0x42E317).
  8. Loop 3 (0x42E336-0x42E447): as loop 2 without the switched-off test: accept the first candidate with `cand != oldCur` and `!wrongWay`.
  9. Nothing found (0x42E44D): `+0x394 = oldCur` (U-turn), `newLink = +0x3A6` (the current next link, unchanged).
  10. U-turn handling (0x42E468): if `+0x394 == oldCur` and status != 3: status = 3 (STATUS_PHYSICS, `+0x36 = (+0x36 & 7) | 0x18`),
     `+0x3BB = 0`, `+0x3AC = now + 2000`, `+0x3B0 = now`, `+0xB8 = 0` (0x42E484-0x42E4BC).
  11. Behaviour of the new next node (`t = +0x394 node byte +0x1A >> 4`, 0x42E4C3-0x42E4F0):
     t == 1: status = 3, `+0x3BB = 0`, `+0x3AC = now + 2000`, mission `+0x3BA = 0x2E` (PARK_PARALLEL), `+0x3B0 = now`, `+0xB8 = 0`,
       `+0x3B9 = 0`; t == 2: same with mission 0x2D (PARK_PERPENDICULAR) (0x42E52D-0x42E59C);
     t == 10: `+0x3BB = 1` (TEMPACT_WAIT), `+0x3BC = now + 10000`; if status == 2 `ModifySpeed(0.0f)` (0x42E4F6-0x42E526).
  12. If the (new) current node `+0x390` has `t == 9` (`(byte +0x1A & 0xF0) == 0x90`) and `veh->+0x4A4 != 2`: `+0x3BB = 1`, `+0x3BC = now + 4500`;
     if status == 2 `ModifySpeed(0.0f)` (0x42E5A3-0x42E600).
  13. Shift links (0x42E605-0x42E65D): `+0x3A8 = +0x3A4`; `+0x3A4 = +0x3A6`; `+0x39C += +0x3A0`; `+0x3B4 = +0x3B5`; `+0x3B5 = +0x3B6`;
     `+0x3B7 = +0x3B8`; `+0x3A6 = newLink`. `+0x3B6 = (oldNext < +0x394) ? -1 : 1` (lexicographic area, node; unsigned, 0x42E664-0x42E6C6).
     `lanes = m_pPathNodes[newLink >> 10] ? (dir == -1 ? LB : LA) of navi[newLink] : 1` (0x42E680-0x42E6F3).
  14. Direction floats (0x42E6F8-0x42E7C3): `cd = (dirX, dirY) of navi[+0x3A4] * 0.01f * (float)+0x3B5` -> [E+0x20], [E+0x1C];
     `nd = dir of navi[newLink] * 0.01f * (float)+0x3B6` -> [E+0x28], [E+0x24] (each `fild i8; fmul 0.01; fmul dirSign`, stored f32).
     m_pNaviNodes[newLink >> 10] is read here without the NULL check of step 13.
  15. Lane change (0x42E7CB-0x42E9B2; guarded by `lanes < 0` which can never be true): `d2 = (Ax - Cx)*(Ax - Cx) + (Ay - Cy)*(Ay - Cy)`
     with A = node +0x394, C = node +0x390 positions (i16 * 0.125f, x87, x product first); if `d2 > 256.0f` (`test ah,0x41; jne`):
     `if (--veh->+0x3E0 == 0) { +0x3E0 = (rand() & 3) + 4; if (rand() < 0x3FFF) +0x3B8++ else +0x3B8--; }` (u8 / i8 wrap).
     Then clamp: `l = (i8)+0x3B8; if (l >= lanes - 1) l = lanes - 1; +0x3B8 = (int8)l; if ((int8)l <= 0) +0x3B8 = 0`.
  16. `if (+0x3DB & 8) +0x3B8 = 0; else if (+0x3DB & 0x10) +0x3B8 = max(lanes - 1, 0)` (0x42E9B8-0x42E9DC).
  17. Only if status == 2 (else return, 0x42E9EB): `o1 = (OneWayLaneOffset(navi[+0x3A4]) + (float)(i8)+0x3B7) * 5.4f` stored f32;
     `o2 = (OneWayLaneOffset(navi[+0x3A6]) + (float)(i8)+0x3B8) * 5.4f` (x87); with NL = navi[+0x3A6], CL = navi[+0x3A4] positions (i16 * 0.125f):
     `P2 = (NL.x + o2 * nd.y, NL.y - o2 * nd.x, 0)`, `P1 = (CL.x + o1 * cd.y, CL.y - o1 * cd.x, 0)` (0x42EAB4-0x42EB7A);
     `s = CalcSpeedScaleFactor(&P1, &P2, cd'x, cd'y, nd'x, nd'y)` where the four directions are recomputed from the link bytes
     (`dir * 0.01f * sign`, same values as step 14) (0x42EC4E); `v = _ftol(s * (1000.0f / veh->+0x3C8))` (0x42EC53-0x42EC64);
     `+0x3A0 = v`, then `+0x3A0 = max(v, 10)` (signed).
- Edge cases / original bugs: division by zero in InitSequence when the next node has no links (step 5); +0x394 temporarily holds rejected
  candidates; the dead `lanes < 0` guard; the IsThisAnAppropriateNode call passes 6 args to a 5-arg function (harmless, cdecl);
  `m_speed == 0` gives inf -> `_ftol` = 0x80000000 (negative) -> `+0x3A0 = 10`.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:684-686). Open questions: none.

## 0x42EC90 CCarCtrl::DragCarToPoint  (catalogue: gr-unreversed, port level 8)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], CVector* target [+8])`, plain `ret`. Only `target->x/y` are read.
  Frame `sub esp,0x110` + esi, then ebx (= 0 throughout) / edi; T = esp after `push edi` (0x42ED18) = esp0 - 0x11C. Locals (T-relative):
  tx [T+0xC], ty [T+0x10], frontGroundZ fz [T+0x14], rear anchor rx [T+0x18] / ry [T+0x1C], rearGroundZ rz [T+0x20], scratch z [T+0x24],
  line-test vector V [T+0x28..0x30], hit entity [T+0x34], fwd copy [T+0x44..0x4C] and [T+0x78..0x80], L [T+0x50], zHigh [T+0x54],
  k1 [T+0x58], k2 [T+0x5C], zLow [T+0x60], halfDz [T+0x64], depth [T+0x68], CColPoint [T+0x84] (pos.z at [T+0x8C]), CColSphere
  [T+0xB0..0xBC], CColLine [T+0xC4..0xE0], CColPoint [T+0xF0].
- Callers: CCarCtrl::UpdateCarOnRails 0x436540 (1).
- Callees: CCollision::ProcessLineSphere 0x412AA0 (cdecl (const CColLine&, const CColSphere&, CColPoint&, float& depth) -> AL),
  CCollision::IsStoredPolyStillValidVerticalLine 0x414D70 (cdecl (const CVector& origin, float zEnd, CColPoint&, CStoredCollPoly*) -> AL),
  CWorld::ProcessVerticalLine 0x5674E0 (cdecl, 11 args (origin, zEnd, colPoint, CEntity*& outEntity, buildings, vehicles, peds, objects,
  dummies, seeThrough, CStoredCollPoly*) -> AL; always called with (.., 1, 0, 0, 0, 0, 0, poly)), CVector::CrossProduct 0x59C730 (cdecl
  (out, a, b), `a x b`), virtual `CVehicle::GetHeightAboveRoad` (vtable +0xD4, slot 53; thiscall -> ST0; CAutomobile 0x6A62B0,
  CBike 0x6B58B0), CVehicle::UpdateLightingFromStoredPolys 0x6D0CC0 (thiscall).
- Globals: CModelInfo::ms_modelInfoPtrs 0xA9B0C8. Constants (f32): 0x858EF0 = 0.95, 0x858B8C = 0.5, 0x858CE8 = 1.5, 0x858CA0 = 2.0,
  0x858624 = 1.0, 0x858C1C = -1.0, 0x859004 = -10.0, 0x858F40 = -0.5; immediates 3.0f (0x40400000), 100.0f (0x42C80000), 1.0f.
- Fields: veh +0x14 m_matrix (right +0x00, fwd +0x10, up +0x20, pos +0x30; dereferenced without NULL check at 0x42ECE6), +0x04 placement,
  +0x1C entity flags (bit 30 m_bTunnel, bit 31 m_bTunnelTransition), +0x22 model, +0x3E1 autopilot field_51 (u8/i8 frames-without-ground
  counter), +0x48C m_pEntityWeAreOn, +0x494 m_fSteerAngle (f32), +0x4E4 m_fVehicleFrontGroundZ, +0x4E8 m_fVehicleRearGroundZ,
  +0x51C m_FrontCollPoly, +0x548 m_RearCollPoly. Colmodel (modelinfo +0x14) bbox min.y +0x04, max.y +0x10.
- Behaviour:
  1. `L = (cm.max.y - cm.min.y) * 0.95f` (stored f32, kept in ST0). `k1 = k2 = 3.0f`; if `(int8)veh->+0x3E1 > 16` (signed) `k1 = k2 = 100.0f`
     (0x42ECAC-0x42ECDE). `F = m.fwd` copied (0x42ECE6-0x42ED19).
  2. Rear anchor: `rx = P.x - (F.x * L) * 0.5f`, `ry = P.y - (F.y * L) * 0.5f` (fsubr, stored f32; P = veh position) (0x42ED27-0x42ED6E).
     `tx = target->x`, `ty = target->y`.
  3. Line/sphere (0x42ED76-0x42EE55): line start `(rx, ry, 0)`, end `(F.x*1.5f + rx, F.y*1.5f + ry, 0)` (1.5*F computed first, then `+ r`);
     sphere centre `(tx, ty, 0)`, radius `L` (the remaining ST0); `depth = 1.0f`. If `ProcessLineSphere(line, sphere, colPoint, depth)`:
     `rx = colPoint.pos.x`, `ry = colPoint.pos.y` (0x42EE61-0x42EE73). Else (0x42EE79-0x42EEC1): `d = sqrt(dx*dx + dy*dy)` with
     `dy = ty - ry`, `dx = tx - rx` (x87; sum order dx^2 + dy^2), `k = k1 / d`, `rx = (rx - tx) * k + tx`, `ry = (ry - ty) * k + ty` (stored f32).
  4. Front ground (0x42EEC3-0x42F01F): `halfDz = (F.z * L) * 0.5f` (stored); `zf = P.z + halfDz` (stored [T+0x24]); `veh->+0x3E1++`;
     `zLow = zf - k1`, `zHigh = zf + k2` (stored). If `IsStoredPolyStillValidVerticalLine((tx, ty, zHigh), zLow, colPoint, &veh->+0x51C)`:
     use it (no entity update). Else `ProcessVerticalLine((tx, ty, zf + 1.5f), zf - 2.0f, colPoint, &ent, 1,0,0,0,0,0, &veh->+0x51C)`; if that
     misses, `ProcessVerticalLine((tx, ty, zHigh), zLow, ...)` (same flags); if both miss: `fz = veh->+0x4E4` (old value) and skip the
     entity update. On a ProcessVerticalLine hit: `veh->+0x48C = ent`; veh flags (+0x1C) bits 30 and 31 are copied from `ent->+0x1C`
     (0x42EFD4-0x42F000: `v ^= (v ^ e) & 0x40000000`, then `v = ((e ^ v) & 0x7FFFFFFF) ^ e`); `veh->+0x3E1 = 0`.
     With a hit (any kind): `fz = colPoint.pos.z`. `veh->+0x4E4 = fz` (0x42F009-0x42F019).
  5. Rear ground (0x42F031-0x42F17D): `zr = P.z - halfDz` (stored [T+0x24]); `zLow = zr - k1`, `zHigh = zr + k2`; same three-step test at
     `(rx, ry, ...)` with `&veh->+0x548` (stored poly; then `(rx, ry, zr + 1.5f)` to `zr - 2.0f`; then `(rx, ry, zHigh)` to `zLow`); on a
     ProcessVerticalLine hit the same entity / flag / counter update; on a total miss `rz = veh->+0x4E8`. `veh->+0x4E8 = rz` (0x42F192).
  6. Orientation (0x42F181-0x42F1F6): `invL = 1.0f / L`; `pitch = atan2((fz - rz) * invL, 1.0)` (fpatan); `c = cos(pitch)`;
     `m.right.x = (ty - ry) * invL`; `m.right.y = (tx - rx) * (-1.0f / L)` (the -1/L quotient computed first); `m.right.z = 0`;
     `m.fwd.x = -(c * m.right.y)`; `m.fwd.y = c * m.right.x` (both read back from the stored f32 right); `m.fwd.z = sin(pitch)`;
     `m.up = Cross(m.right, m.fwd)` (0x42F202) stored component-wise.
  7. Position (0x42F207-0x42F2EB): `x = (tx + rx) * 0.5f`, `y = (ty + ry) * 0.5f`, `z = (rz + fz) * 0.5f` (each sum stored/reloaded f32);
     `z = veh->GetHeightAboveRoad() + z` (vtable +0xD4, 0x42F29B); `UpdateLightingFromStoredPolys()`; write `(x, y, z)` to `m.pos` if the
     matrix exists, else to the placement position.
  8. Steering (0x42F2EB-0x42F356): `c = Cross(newFwd (m+0x10), F (the copy from step 1))`; `s = c.z * -10.0f`; `veh->+0x494 = (s < 0.5f) ?
     ((s > -0.5f) ? s : -0.5f) : 0.5f` (NaN -> 0.5f via the first test failing).
- Edge cases: `d == 0` in step 3 (target exactly at the rear anchor) gives inf/NaN anchors. The +0x3E1 counter grows each call until a
  ProcessVerticalLine hit resets it (stored-poly hits do not reset it); above 16 the search ranges widen to 100 m.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:358-360). Open questions: none.

## 0x42F370 CCarCtrl::GetAIPlaneToDoDogFightAgainstPlayer  (catalogue: gr-unreversed, port level 19)
- Convention / signature: `__cdecl void (CPlane* plane [esp+4])`; ends with a tail `jmp` to GetAIPlaneToDoDogFight 0x429890 with the same
  stack argument (rewritten in place, 0x42F396 / 0x42F3B3).
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1). Callees: CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (-1, false), x2),
  CWorld::FindPlayerPed 0x56E210 (cdecl (-1)), CCarCtrl::GetAIPlaneToDoDogFight 0x429890.
- Behaviour: if `FindPlayerVehicle(-1, false) != NULL`: `plane->+0x41C (m_TargetEntity) = FindPlayerVehicle(-1, false)` (second call);
  else `plane->+0x41C = FindPlayerPed(-1)`. Then `GetAIPlaneToDoDogFight(plane)`.
- Edge cases: the target pointer is stored raw - no RegisterReference / CleanUpOldReference (unlike the script/AI paths that set targets).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:533-535). Open questions: none.

## 0x42F3C0 CCarCtrl::GetAIHeliToAttackPlayer  (catalogue: gr-unreversed, port level 18)
- Convention / signature: `__cdecl void (CAutomobile* heli [esp+4])`, plain `ret`. Frame `sub esp,0x40` + 4 pushes; E = esp after `push edi`:
  heading [E+0x10]; the heli arg slot [E+0x54] holds a position pointer, then the 2D distance `d`.
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1).
- Callees: CWorld::FindPlayerCoors 0x56E010 (x8, each with its own buffer), CGeneral::GetATanOfXY 0x53CC70, CCarCtrl::FlyAIHeliInCertainDirection
  0x429A70, CWorld::FindPlayerEntity 0x56E120 (cdecl (-1), x2), CCarCtrl::TestWhetherToFirePlaneGuns 0x429520, CCarCtrl::FireHeliRocketsAtTarget 0x42B270.
- Constants: 0x859008 = 18.0f, 0x858CB8 = 3.1415927f, 0x858B48 = 15.0f, 0x858B40 = 50.0f.
- Fields: +0x3BA mission, +0x3EC dest, +0x9A8 m_fMaxAltitude.
- Behaviour:
  1. `heading = GetATanOfXY(c.x - H.x, c.y - H.y)` (c = FindPlayerCoors(-1); `c.y - H.y` first) stored f32 (0x42F3DB-0x42F411).
  2. `d = sqrt((c.y - H.y)*(c.y - H.y) + (c.x - H.x)*(c.x - H.x))` (each factor from a separate FindPlayerCoors call; y product first) stored f32
     (0x42F463-0x42F4CC). `+0x9A8 = c.z` (raw); `dest = c` (0x42F4D0-0x42F4FD).
  3. If mission == 0x17 (HELI_ATTACK_PLAYER): if `d < 15.0f` `mission = 0x38` (HELI_ATTACK_PLAYER_FLY_AWAY); `d = d + 50.0f` (stored).
     Else if mission == 0x38: if `d > 18.0f` `mission = 0x17`; `heading += 3.1415927f` (stored) (0x42F500-0x42F55B).
  4. `FlyAIHeliInCertainDirection(heli, heading, d, false)`; `TestWhetherToFirePlaneGuns(heli, FindPlayerEntity(-1))`;
     `FireHeliRocketsAtTarget(heli, FindPlayerEntity(-1))` (0x42F55F-0x42F588).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:513-515). Open questions: none.

## 0x42F5A0 CCarCtrl::JoinCarWithRoadSystem  (catalogue: gr-unreversed, port level 2)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret`. Entry: `sub esp,0x54; push ebx; push ebp; jmp 0x405881`
  -> `mov ebx,0xFFFF; jmp 0x42F5AA` (a .text detour fragment; ebx = 0xFFFF is the "empty area" constant used below). E = esp after
  `push edi` (0x42F5B4) = esp0 - 0x64: best link [E+0x10] (dword; low word tested), loop counter [E+0x14], scratch floats [E+0x18..0x5C],
  [E+0x68] = node / bestDist / fwd.y.
- Callers: 18 sites (CCarAI 0x41BFA0, 0x41C8A0, 0x41C900, 0x41C960, 0x41DA30 x5; CCarCtrl::CreateCarForScript 0x431F80 x2,
  JoinCarWithRoadAccordingToMission 0x432CB0; CRemote 0x45AB10; CRoadBlocks 0x4619C0; script 0x466DE0, 0x483BD0; 0x486300; ...).
- Callees: CPathFind::FindNodeClosestToCoorsFavourDirection 0x44FCE0 (thiscall ECX = ThePaths 0x96F050; stack (CNodeAddress* ret,
  CVector pos (12 bytes by value), int32 pathType = 0 (cars), float dirX, float dirY); returns EAX = ret), CCarCtrl::FindLinksToGoWithTheseNodes
  0x42B470.
- Globals: ThePaths 0x96F050 (m_pPathNodes 0x96F854, m_pNodeLinks 0x96FA94). Constants: 0x858C48 = 0.125f, 0x858B50 = 0.0f, 0x858624 = 1.0f,
  immediate 999999.875f (0x497423FE).
- Fields: veh +0x14 m_matrix (fwd.x +0x10, fwd.y +0x14), +0x390/+0x394/+0x398 current/next/previous node (gr m_currentAddress /
  m_startingRouteNode / m_endingRouteNode), +0x3A4/+0x3A6/+0x3A8 links, +0x3B7/+0x3B8 current/next lane (i8), +0x418 m_nPathFindNodesCount (u16).
- Behaviour:
  1. Reset (0x42F5B5-0x42F5E4): area words (+0) of +0x398, +0x390, +0x394 = 0xFFFF (node words untouched); +0x3A4, +0x3A8, +0x3A6 = 0xFFFF;
     +0x418 = 0.
  2. `node = FindNodeClosestToCoorsFavourDirection(veh->GetPosition(), 0, m.fwd.x, m.fwd.y)` (0x42F621; the matrix fields are read from
     `veh->m_matrix` without a NULL check even though the position falls back to the placement). Return if `node.area == 0xFFFF` or
     `m_pPathNodes[node.area] == NULL` (0x42F628-0x42F643).
  3. `n = N.flags & 0xF`; `bestDist = 999999.875f`, `best.area = 0xFFFF`; if `n <= 0` return. For each link k = 0..n-1 (pointer walk over
     `m_pNodeLinks[node.area][N.base + k]`, 0x42F690-0x42F766): skip if `m_pPathNodes[L.area] == NULL`; `dx = Lx - Nx`, `dy = Ly - Ny`
     (positions i16 * 0.125f, each also stored f32); `d = sqrt(dx*dx + dy*dy)` (x87, x term first); if `d < bestDist` (strict, NaN false)
     `bestDist = d`, `best = L`.
  4. If `best.area == 0xFFFF` return (0x42F76C-0x42F774).
  5. `dirX = m.fwd.x`, `dirY = m.fwd.y`; if both are exactly 0 (`test ah,0x44; jp`; NaN counts as non-zero) `dirX = 1.0f` (0x42F77A-0x42F7A7).
     `dot = (Nx - Bx) * dirX + (Ny - By) * dirY` (B = node `best`; x87, 0x42F7AD-0x42F817). If `dot < 0` (strict; NaN false):
     current = `node`, next = `best`; else current = `best`, next = `node` (0x42F825-0x42F836).
  6. `+0x398` area = 0xFFFF (again); `+0x390 = current`; `+0x394 = next` (dwords); `FindLinksToGoWithTheseNodes(veh)`; `+0x3B7 = 0`;
     `+0x3B8 = 0` (0x42F83A-0x42F85D).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:662-664). Open questions: none.

## 0x42F870 CCarCtrl::JoinCarWithRoadSystemGotoCoors  (catalogue: gr-unreversed, port level 5)
- Convention / signature: `__cdecl bool (CVehicle* veh [esp+4], const CVector* pos [+8], bool unused [+0xC], bool waterPath [+0x10])`,
  returns AL, plain `ret`. Entry `jmp 0x156A4A0` (.HOODLUM body 0x156A4A0-0x156A5FA; the catalogue extent stops at 0x156A591 because the
  body jumps into the middle of an instruction: `jmp 0x156A590` at 0x156A58F lands on `call eax`). E = esp after the 4 pushes.
- Callers: 10 functions (CCarAI 0x41CE30, 0x41D0E0, 0x41D210, 0x41DA30 x6; CCarCtrl::ScriptGenerateOneEmergencyServicesCar 0x42FBC0,
  JoinCarWithRoadAccordingToMission 0x432CB0 x3; CGangWars::CreateAttackWave 0x444810; script 0x466DE0, 0x47FA30, 0x496E00).
- Callees: CPathFind::DoPathSearch 0x4515D0 (thiscall ECX = ThePaths 0x96F050, 19 dwords, see ReconsiderRoute), CPathFind::RemoveBadStartNode
  0x44E4F0 (gr PathFind.h:235 `void RemoveBadStartNode(CVector pos, CNodeAddress* nodes, int16* count)`, thiscall, address absent in gr;
  ECX comes from the protection indirection below and is expected to be ThePaths: the callee reads `[ecx + area*4 + 0x804]` = m_pPathNodes),
  CAutoPilot::RemoveOnePathNode 0x41B950 (thiscall ECX = veh+0x390), CCarCtrl::FindLinksToGoWithTheseNodes 0x42B470, CCarCtrl::JoinCarWithRoadSystem 0x42F5A0.
  Protection indirection (0x156A57D-0x156A595): `pushfd; push eax; push esp; push 0x155FFEC; push 0xCB73E0; pop eax; call eax; mov ecx, eax;
  pop eax; popfd` - SecuROM routine 0xCB73E0 returns a pointer in EAX used as ECX (same pattern as documented in
  docs/re/specs/WORLD/InteriorGroup_c.md for 0x44DF60). The port passes `&ThePaths`.
- Globals: 0x8A5F44 (CNodeAddress constant {area 0xFFFF, node 0}, read twice), ThePaths 0x96F050. Immediate 999999.875f (0x497423FE).
- Fields: veh +0x22 model, +0x3DB bit 6 bCantGoAgainstTraffic, +0x3EC m_vecDestinationCoors (CVector), +0x3F8 m_aPathFindNodesInfo[8],
  +0x418 m_nPathFindNodesCount (i16), +0x390/+0x394/+0x398 current/next/previous node, +0x3B7/+0x3B8 lanes.
- Behaviour:
  1. `veh->+0x3EC = *pos` (0x156A4AC-0x156A4C1).
  2. `DoPathSearch(0, vehPos, *(CNodeAddress*)0x8A5F44, *pos, &veh->+0x3F8, &veh->+0x418, 8, NULL, 999999.875f, NULL, 999999.875f,
     (+0x3DB >> 6) & 1, *(CNodeAddress*)0x8A5F44, model == 0x21B, waterPath)` (0x156A4D3-0x156A552).
  3. `RemoveBadStartNode(vehPos (re-read), &veh->+0x3F8, &veh->+0x418)` (0x156A557-0x156A596).
  4. If `(int16)veh->+0x418 < 2` (0x156A59B): `JoinCarWithRoadSystem(veh)`; `veh->+0x418 = 0`; return 1 (0x156A5E8-0x156A5FA).
  5. Else: `+0x390 = nodes[0]`; `+0x398` area word = 0xFFFF; `RemoveOnePathNode()`; `+0x394 = nodes[0]` (the former nodes[1]);
     `RemoveOnePathNode()`; `FindLinksToGoWithTheseNodes(veh)`; `+0x3B7 = 0`; `+0x3B8 = 0`; return 0 (0x156A5A1-0x156A5E7).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:667-669; declared `(CVehicle*, const CVector&, bool unused, bool bIsBoat)`).
  Open questions: the pointer returned by 0xCB73E0 at 0x156A590 is not statically known (expected &ThePaths, consistent with the callee's
  m_pPathNodes access).

## 0x42F9C0 CCarCtrl::GenerateEmergencyServicesCar  (catalogue: gr-unreversed, port level 17)
- Convention / signature: `__cdecl void ()`, plain `ret`. Frame `sub esp,0xC` (CVector temp for FindPlayerCoors) + esi.
- Callers: CCarCtrl::GenerateRandomCars 0x4341C0 (tail call).
- Callees: CGangWars::GangWarFightingGoingOn 0x443AC0, CWorld::FindPlayerPed 0x56E210 (cdecl (-1)), CAccidentManager::GetInstance 0x56CF20,
  CAccidentManager::GetNumberOfFreeAccidents 0x56CEE0 (thiscall), CWorld::FindPlayerCoors 0x56E010 (cdecl (CVector* out, -1) -> EAX = out),
  CAccidentManager::GetNearestFreeAccident 0x56D050 (thiscall (const CVector&, bool = 0) -> CAccident*), CStreaming::StreamAmbulanceAndMedic
  0x40A2A0 (cdecl (bool) -> AL), 0x407D30 CStreaming::GetDefaultAmbulanceModel (proposed; `_unattributed-P2.md`), 0x407DC0
  CStreaming::GetDefaultFireEngineModel (proposed), CCarCtrl::GenerateOneEmergencyServicesCar 0x42B7D0 (cdecl (uint32 model, CVector pos by
  value) -> EAX), CFireManager::GetNumOfNonScriptFires 0x538F10 (thiscall ECX = gFireManager 0xB71F80 -> AX), CFireManager::FindNearestFire
  0x538F40 (thiscall (const CVector&, bool 1, bool 1) -> CFire*), CStreaming::StreamFireEngineAndFireman 0x40A400 (cdecl (bool) -> AL).
- Globals read: bAllowEmergencyServicesToBeCreated 0x8A5B28 (u8), CGame::currArea 0xB72914, CTheZones::m_CurrLevel 0xBA6718, NumAmbulancesOnDuty
  0x9690A8, NumFireTrucksOnDuty 0x9690AC, NumParkedCars 0x9690A0, NumMissionCars 0x96909C, NumLawEnforcerCars 0x969098, NumRandomCars
  0x969094, MaxNumberOfCarsInUse 0x8A5B24, CTimer::m_snTimeInMilliseconds 0xB7CB84. RW: LastTimeAmbulanceCreated 0x9690B0,
  LastTimeFireTruckCreated 0x9690B4.
- Fields: player ped +0x480 m_pPlayerData -> +0 m_pWanted -> +0x2C m_nWantedLevel; CAccident +0 m_pPed; CFire +0x04 position.
- Behaviour:
  1. Return if `!bAllowEmergencyServicesToBeCreated`, if `GangWarFightingGoingOn()`, if `wanted->m_nWantedLevel > 3` (signed; the wanted
     pointer is `playerData ? playerData->m_pWanted : NULL` and is dereferenced either way, 0x42F9E5-0x42F9FC), if `currArea != 0`, if
     `m_CurrLevel == 0`, or if `ambul + fire + parked + mission + law + random > MaxNumberOfCarsInUse` (signed sum in that order, 0x42FA1C-0x42FA51).
  2. Ambulance (only if `NumAmbulancesOnDuty == 0`, 0x42FA57): if `GetInstance()->GetNumberOfFreeAccidents() < 2`:
     `StreamAmbulanceAndMedic(false)` (0x42FB00). Else `acc = GetNearestFreeAccident(FindPlayerCoors(-1), false)`; if `acc` and
     `StreamAmbulanceAndMedic(true)` and `now > LastTimeAmbulanceCreated + 30000` (unsigned): `if (GenerateOneEmergencyServicesCar(
     GetDefaultAmbulanceModel(), acc->m_pPed->GetPosition()))` `LastTimeAmbulanceCreated = now` (0x42FA98-0x42FAF8).
  3. Fire engine (0x42FB08): return if `NumFireTrucksOnDuty != 0`. If `GetNumOfNonScriptFires() < 3` (unsigned u16): `StreamFireEngineAndFireman
     (false)` and return. Else `fire = gFireManager.FindNearestFire(FindPlayerCoors(-1), true, true)`; if `fire` and `StreamFireEngineAndFireman
     (true)` and `now > LastTimeFireTruckCreated + 35000`: `if (GenerateOneEmergencyServicesCar(GetDefaultFireEngineModel(), fire->+4 position))`
     `LastTimeFireTruckCreated = now` (0x42FB49-0x42FB9A).
- Edge cases: a player ped without player data would crash at 0x42F9F8 (reads `[0 + 0x2C]`). The ambulance step does not stop the fire step.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:460-462). Open questions: none.

## 0x42FC40 CCarCtrl::ReconsiderRoute  (catalogue: gr-unreversed, port level 5)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret`. Frame `sub esp,0x24` + ebx/ebp/esi/edi; E = esp after the
  pushes (0x42FC64): n2 [E+0x10], n1 [E+0x14] (CNodeAddress dwords), dist [E+0x18] (f32), target x [E+0x1C] (y in ebp, z in ebx),
  FindPlayerCoors buffer [E+0x28], path count [E+0x38] (i16).
- Callers: CAutomobile::ProcessAI 0x6B4800 (1).
- Callees: CCarCtrl::FindNodesThisCarIsNearestTo 0x42BD20 (cdecl (veh, CNodeAddress* out1, CNodeAddress* out2)), 0x420920 -> 0x1565640
  `CNodeAddress::operator==` (thiscall ECX = this, arg `const CNodeAddress*`; returns EAX 1 if both words equal, `ret 4`), CWorld::FindPlayerCoors
  0x56E010 (cdecl (CVector* out, -1) -> EAX = out), CPathFind::DoPathSearch 0x4515D0 (thiscall ECX = ThePaths 0x96F050, 19 stack dwords
  `(pathType, CVector originPos, CNodeAddress originAddr, CVector targetPos, CNodeAddress* outNodes, int16* outCount, int32 maxNodes,
  float* outDistance, float maxSearchDistance, CNodeAddress* targetNodeAddr, float maxUnkLimit, bool oneSideOnly, CNodeAddress forbidden,
  bool includeNodesWithoutLinks, bool waterPath)`, callee-cleaned), CCarCtrl::FindLinksToGoWithTheseNodes 0x42B470, CAutoPilot::RemoveOnePathNode
  0x41B950 (thiscall ECX = veh+0x390).
- Globals: CTimer::m_snPreviousTimeInMilliseconds 0xB7CB78, m_snTimeInMilliseconds 0xB7CB84. Constant 0x85900C = 90000.0f; immediate
  999999.875f (0x497423FE).
- Fields: veh +0x20 CEntity::m_nRandomSeed (u16), +0x22 model, +0x390 current node, +0x394 next node, +0x398 previous node, +0x3BA mission (i8
  via movsx), +0x3C5 m_ucCarMissionModeCounter (u8), +0x3DB bit 6 bCantGoAgainstTraffic, +0x3EC m_vecDestinationCoors, +0x3F8
  m_aPathFindNodesInfo[8], +0x418 m_nPathFindNodesCount (i16), +0x41C m_TargetEntity.
- Behaviour:
  1. Throttle (0x42FC40-0x42FC76): `s = veh->+0x20`; return unless `(now + s) / 2000 != (prev + s) / 2000` (unsigned 32-bit division done
     with the magic multiply 0x10624DD3 and `shr 7`).
  2. Mission filter (jump table 0x42FF98 via byte table 0x42FFA0, index `mission - 2`, `ja` for > 0x3B): continue only for missions
     2 RAMPLAYER_FARAWAY, 4 BLOCKPLAYER_FARAWAY, 8 GOTOCOORDINATES, 15 RAMCAR_FARAWAY, 17 BLOCKCAR_FARAWAY, 43 APPROACHPLAYER_FARAWAY,
     52 FOLLOWCAR_FARAWAY, 54 KILLPED_FARAWAY, 61 DO_DRIVEBY_FARAWAY; every other mission goes to RESET (0x42FF88: `+0x3C5 = 0`, return).
  3. `n1.area = n2.area = 0xFFFF`; `FindNodesThisCarIsNearestTo(veh, &n1, &n2)` (0x42FCB7); if `n1.area == 0xFFFF` return WITHOUT reset.
  4. Already-on-route tests (each `->` RESET): `cur == n1 && next == n2`; `cur == n2 && next == n1`; `prev == n1 && cur == n2`;
     `prev == n2 && cur == n1`; `cur == n2`; `prev == n2` (0x42FCCD-0x42FD9C, in this order; cur/next/prev = +0x390/+0x394/+0x398).
  5. `if (++veh->+0x3C5 <= 4) return` (u8, no reset, 0x42FDA2-0x42FDB4).
  6. Target (jump table 0x42FFDC via byte table 0x42FFEC): missions 2, 4, 43 -> `FindPlayerCoors(-1)`; 8 -> `+0x3EC`; 15, 17, 52, 54, 61 ->
     `m_TargetEntity` position (matrix +0x30 or placement +4). (The table's default entry 0x42FE39, which would use uninitialised locals,
     is unreachable because step 2 already filtered the mission.)
  7. `DoPathSearch(0, vehPos, n2, target, NULL, &count, 0, &dist, 999999.875f, NULL, 999999.875f, (+0x3DB >> 6) & 1, cur (+0x390),
     model == 0x21B, false)` (0x42FEBD; the bool dwords carry stale upper bytes, the callee reads bytes). If `!(dist < 90000.0f)` (NaN too)
     or `count < 2` (i16) -> RESET.
  8. `+0x390 = n1`; `+0x394 = n2`; `FindLinksToGoWithTheseNodes(veh)`; `DoPathSearch(0, vehPos, n2, target, &veh->+0x3F8, &veh->+0x418, 8,
     NULL, 999999.875f, NULL, 999999.875f, cantGoAgainstTraffic, +0x390 (= n1 now), model == 0x21B, false)` (0x42FF7C);
     `RemoveOnePathNode(&veh->m_autoPilot)`; then RESET (`+0x3C5 = 0`).
- Edge cases: step 3's early exit and step 5 keep the counter; all other exits clear it. vehPos for both searches is read via
  GetPosition (matrix or placement).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:748-750). Open questions: none.

