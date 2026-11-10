# CCarCtrl (POP) - full specs

Common conventions: entity position = `m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &placement.pos (+0x04)`. ThePaths 0x96F050:
m_pPathNodes[] 0x96F854 (CPathNode stride 0x1C), m_pNaviNodes[] 0x96F974 (CCarPathLink stride 0xE). CVehicle autopilot at +0x390
(gr CAutoPilot field = offset - 0x390). `_rand` 0x821B1E, `_ftol` 0x821B40 (truncation). gta-reversed source:
re/gta-reversed/source/game_sa/CarCtrl.cpp; gta-reversed deviation notes for reversed functions are in CCarCtrl.deviations.md.

## 0x421440 CCarCtrl::GetNewVehicleDependingOnCarModel  (catalogue: specified/REA, port level 17)
- Convention / signature: `__cdecl CVehicle* (int32 modelId [esp+4], uint8 createdBy [esp+8])`, returns EAX, plain `ret`.
  Entry `nop; jmp 0x401438` (0x421440) is a .text detour stub that loads `fs:[0]` and jumps back to 0x421446: same behaviour.
  MSVC SEH frame (handler 0x83741E, state slot [esp+0xC] = 0..9 per branch) only to `delete` the pool slot if a constructor
  throws; no observable behaviour for the port.
- Callers: CreatePoliceChase 0x42C2B0, CreateConvoy 0x42C740, GenerateOneRandomCar 0x430050, CreateCarForScript 0x431F80.
- Callees: `CVehicle::operator new` 0x6E2D50 (cdecl (size), vehicle-pool `New()`; size argument ignored by the pool), and the
  thiscall constructors (this = new block, args `(modelId, createdBy)`): CBoat 0x6F2940, CTrain 0x6F6030, CBike 0x6BF430,
  CBmx 0x6BF820, CMonsterTruck 0x6C8D60, CQuadBike 0x6CE370, CHeli 0x6C4190, CPlane 0x6C8E20, CTrailer 0x6D03A0,
  CAutomobile 0x6B0A90 (three args: `(modelId, createdBy, 1 = setupSuspensionLines)`, 0x4216DF).
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8 (read). Field: CVehicleModelInfo +0x3C `m_nVehicleType` (i32).
- Behaviour: `t = ms_modelInfoPtrs[modelId]->m_nVehicleType` (0x42145A-0x421461); jump table 0x42170C indexed by `t-1`, any
  `(uint32)(t-1) > 10` (i.e. AUTOMOBILE 0 and anything out of range) goes to the automobile branch (0x421468):

  | type | size (0x6E2D50 arg) | constructor | case addr |
  |---|---|---|---|
  | 0 AUTOMOBILE / default, 7 FHELI, 8 FPLANE | 0x988 | CAutomobile(model, createdBy, 1) | 0x4216BE |
  | 1 MTRUCK | 0x99C | CMonsterTruck | 0x42158C |
  | 2 QUAD | 0x9BC | CQuadBike | 0x4215CA |
  | 3 HELI | 0xA18 | CHeli | 0x421608 |
  | 4 PLANE | 0xA04 | CPlane | 0x421646 |
  | 5 BOAT | 0x7E8 | CBoat | 0x421475 |
  | 6 TRAIN | 0x6AC | CTrain | 0x4214B3 |
  | 9 BIKE | 0x814 | CBike, then `byte [v+0x614] |= 0x10` | 0x4214F1 / 0x42151B |
  | 10 BMX | 0x838 | CBmx, then `byte [v+0x614] |= 0x10` | 0x421532 / 0x42155C |
  | 11 TRAILER | 0x9F4 | CTrailer | 0x421684 |

  Return value: the constructor's EAX (= this). If `new` returns null, all non-bike branches return 0 (0x4216FA).
  Byte +0x614 bit 0x10 of CBike: gr `bikeFlags` bit 4 = `bOnSideStand` (Bike.h:40; offset taken from the bytes).
- Edge cases / original bug: for BIKE and BMX a null `new` jumps to 0x421573 `xor eax,eax; or byte [eax+0x614],0x10` - a write
  to address 0x614 (access violation) instead of returning null. Port: when the pool is full on a bike/BMX, the original crashes;
  document/assert rather than invent behaviour.
- gta-reversed: `.Reversed = false` hook, declaration only (CarCtrl.h:89, `uint8 createdBy`); no body to compare.
- Open questions: none.

## 0x4217C0 CCarCtrl::SetUpDriverAndPassengersForVehicle  (catalogue: specified/REA, port level 29)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], int32 pedGroup [+8], int32 minPassengers [+0xC],
  bool createAsMale [+0x10] (passed as dword), bool createAsCriminal [+0x14] (dword), int32 maxPassengers [+0x18])`, plain `ret`.
  (gr: `(CVehicle*, int32 arg2, int32 arg3, bool arg4, bool arg5, int32 passengersNum)`; arg names from CVehicle::SetUpDriver
  `(int32 gangPedType, bool createAsMale, bool createAsCriminal)` in gr Vehicle.cpp:1528.)
- Callers: CreatePoliceChase 0x42C2B0, CreateConvoy 0x42C740, GenerateOneRandomCar 0x430050 (3 sites), CGangWars::CreateAttackWave
  0x444810, CCarGenerator::DoInternalProcessing 0x6F34D0.
- Callees: CVehicle::SetUpDriver 0x6D1A50 (thiscall (pedGroup, a4, a5) -> CPed*, result ignored), `_rand` 0x821B1E,
  CPed::GiveObjectToPedToHold 0x5E4390 (thiscall (int32 modelIndex, int32 1)), CModelInfo::IsCarModel 0x4C5AA0 (cdecl (int32) -> AL),
  CAnimManager::GetAnimationBlockIndex(const char*) 0x4D3990 (cdecl -> EAX), virtual `CBaseModelInfo::GetAnimFileIndex`
  (vtable +0x38, thiscall on `ms_modelInfoPtrs[model]`), CVehicle::SetupPassenger 0x6D1AA0 (thiscall (seat, pedGroup, a4, a5) -> CPed*),
  CPed::UpdateStatEnteringVehicle 0x5E01A0 (thiscall).
- Globals: MI_GANG_SMOKE 0x8CD72C (u16, zero-extended), ms_modelInfoPtrs 0xA9B0C8, string 0x858EE0 = "van",
  0x858C7C = 3.0518509447574615e-05 (f32, 1/32767), 0x858C48 = 0.125 (f32).
- Fields: veh +0x460 m_pDriver, +0x488 m_nMaxPassengers (u8), +0x22 m_nModelIndex (i16).
- Behaviour:
  1. `veh->SetUpDriver(pedGroup, a4, a5)` (0x4217D9).
  2. If `14 <= pedGroup <= 23` (signed, 0x4217DE/0x4217E3) and `_rand() < 0x3FFF` (signed, 0x4217ED; rand only called when the range
     test passes): `veh->m_pDriver->GiveObjectToPedToHold(MI_GANG_SMOKE, 1)` (0x4217F4-0x421804).
  3. `maxP = min((int)veh->m_nMaxPassengers, maxPassengers)` (signed `cmp; jge`, 0x421809-0x421818).
     `n = minPassengers`. If `n < maxP` (signed, 0x42181E): repeat `maxP - minPassengers` times (counter esi decremented to 0,
     0x421853): `n += ((float)_rand() * (1/32767) < 0.125f) ? 1 : 0` (x87 `fild; fmul [0x858C7C]; fcomp [0x858C48]`, `test ah,5; jp`,
     0x421828-0x421851); then if `n >= maxP` set `n = maxP` (0x421856). If initially `n >= maxP` (incl. minPassengers > maxP):
     `n = maxP` (0x42185E).
  4. `m = veh->m_nModelIndex`; if `IsCarModel(m)` (0x421865) and `GetAnimationBlockIndex("van") == ms_modelInfoPtrs[m]->GetAnimFileIndex()`
     (0x421871-0x42188C; GetAnimationBlockIndex is called before the virtual) and `n >= 1`: `n = 1` (0x421895).
  5. `for (seat = 0; seat < n; seat++)` (0x42189A-0x4218EC): `p = veh->SetupPassenger(seat, pedGroup, a4, a5)`; if `p`:
     `p->UpdateStatEnteringVehicle()`, then if `14 <= pedGroup <= 23` and `_rand() < 0x3FFF`: `p->GiveObjectToPedToHold(MI_GANG_SMOKE, 1)`.
- Edge cases / original bugs: step 2 uses `m_pDriver` without a null check (0x4217FB); SetUpDriver returns null and leaves no driver when
  the vehicle is not a random vehicle (gr Vehicle.cpp:1533), so a non-random vehicle with pedGroup 14..23 would call
  GiveObjectToPedToHold with this = null when rand() < 0x3FFF. RNG consumption: exactly `maxP - minPassengers` rand() calls in step 3
  when `minPassengers < maxP`, plus the gang rolls.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:862); nothing to compare.
- Open questions: meaning of the 14..23 range (0x4217DE) - with gr's ePedType it is GANG8..SPECIAL, which is suspicious; the value is
  forwarded to SetUpDriver/AddPedInCar as `gangPedType`, so it may be a different enumeration (e.g. a popcycle group). The port only
  needs the numeric range.

## 0x421FE0 CCarCtrl::ThisVehicleShouldTryNotToTurn  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl bool (CVehicle* veh [esp+4])`, returns AL (upper EAX bits garbage), plain `ret`. Caller: PickNextNodeRandomly 0x42DE80.
- Behaviour: `m = (int16)veh->m_nModelIndex (+0x22)` (0x421FE4); switch on `m - 0x193` with bound 0x79 (0x421FED, unsigned `ja` -> false),
  byte index table 0x422010, jump table 0x422008 = {0x422000 true, 0x422003 false}. Returns 1 exactly for models
  0x193 (403 LINERUN), 0x196 (406 DUMPER), 0x1AF (431 BUS), 0x1B5 (437 COACH), 0x1BB (443 PACKER), 0x1C7 (455 FLATBED),
  0x202 (514 PETRO), 0x203 (515 RDTRAIN), 0x20C (524 CEMENT) (byte table read from the image); 0 otherwise.
- gta-reversed: `.Reversed = false`, plugin call only. Open questions: none.

## 0x422090 CCarCtrl::FindPathDirection  (catalogue: specified/REA, port level 1)
- Convention / signature: `__cdecl int8 (CNodeAddress a [esp+4], CNodeAddress b [+8], CNodeAddress c [+0xC], bool* pSharpTurn [+0x10])`,
  CNodeAddress = {u16 area (+0), u16 node (+2)} passed by value in one dword; returns AL; plain `ret`. Uses argument slots as scratch.
- Callers: PickNextNodeRandomly 0x42DE80. Callees: CPathNode::GetPosition 0x420A10 (thiscall (CVector* out) -> EAX = out; decodes the
  compressed i16/8 position), called 8 times (results reused only for one component each, order below).
- Globals: `ThePaths.m_pPathNodes[area]` = dword table 0x96F854 (ThePaths 0x96F050 + 0x804); CPathNode stride 0x1C.
  Constants: 0x858B50 = 0.0, 0x858624 = 1.0, 0x858EE8 = 0.4000000059604645, 0x858EE4 = -0.30000001192092896 (all f32).
- Behaviour:
  1. `*pSharpTurn = 0` (0x4220A0, before any test). If `a.area == 0xFFFF`, `b.area == 0xFFFF` or `c.area == 0xFFFF` (tested in that order,
     0x422097/0x4220B1/0x4220BC) return 0. If `m_pPathNodes[a.area]`, `[b.area]` or `[c.area]` is null (in that order) return 0 (0x42221B).
  2. A = node a, B = node b, C = node c (each `m_pPathNodes[area] + node*0x1C`). Position reads (each a full GetPosition call):
     `dx1 = B.x - A.x` (0x422131, f32 store), `dy1 = B.y - A.y` (0x422158), `dx2 = C.x - B.x` (0x42218C), `dy2 = C.y - B.y` (0x4221B5).
  3. `len1 = sqrt(dy1*dy1 + dx1*dx1)` (0x4221C1-0x4221D3). If `len1 == 0.0f` (fcom, `test ah,0x44; jnp`, 0x4221E0; NaN does not exit)
     return 0. `i1 = 1.0f / len1`; `dx1n = dx1 * i1` stored f32 (0x4221EE); `dy1n = i1 * dy1` kept in x87 (not rounded through memory).
  4. `len2 = sqrt(dy2*dy2 + dx2*dx2)` (0x4221F6-0x422208). If `len2 == 0.0f` (0x422215) return 0. `i2 = 1.0f / len2`;
     `dx2n = dx2 * i2`, `dy2n = i2 * dy2` (x87 only, 0x42222A-0x422232).
  5. `cross = (dx1n * dy2n) - (dx2n * dy1n)` stored f32 (0x422236-0x422242). `dot = (dy1n * dy2n) + (dx2n * dx1n)` (x87, 0x422246-0x42224C).
  6. If `dot > 0.4f` (`test ah,0x41; jne`, 0x422256; NaN/equal fail) return 1 (straight on).
     Else: if `dot < -0.3f` (0x422266-0x422271) `*pSharpTurn = 1`. Return `cross > 0.0f ? 4 : 2` (0x42227B-0x42228A; equal/NaN give 2).
- Return values: 0 invalid/degenerate, 1 straight, 2 turn with cross <= 0 (right, since the map is x east / y north), 4 turn with cross > 0 (left).
- gta-reversed: `.Reversed = false`, plugin call only (`int8 FindPathDirection(CNodeAddress, CNodeAddress, CNodeAddress, bool*)`).
- Open questions: none.

## 0x4222A0 CCarCtrl::SwitchBetweenPhysicsAndGhost  (catalogue: specified/REA, port level 9)
- Convention: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret` (two tail-jumps keep the caller's stack; the tail-called thiscall
  PlaceOnRoadProperly gets ECX = veh and returns to our caller). Caller: SteerAICarWithPhysics 0x437C20.
- Callees: CColStore::HasCollisionLoaded 0x410CE0 (cdecl (const CVector& pos, int32 area = 0) -> AL), CDamageManager::SetWheelStatus
  0x6C21A0 (thiscall on veh+0x5A0, (int32 wheel, int32 status)), CAutomobile::PlaceOnRoadProperly 0x6AF420, CBike::PlaceOnRoadProperly 0x6BEEB0.
- Fields: +0x40 CPhysical flags dword (bit 0x4000 = gr `bDontLoadCollision`, bit 14), +0x4A4 m_nCreatedBy (byte; 2 = MISSION_VEHICLE),
  +0x590 m_nVehicleType, +0x594 m_nVehicleSubType (gr names; i32), +0x36 status in bits 3-7 (3 = STATUS_PHYSICS, 12 = STATUS_GHOST),
  +0x14 m_matrix / +0x04 placement (pos = matrix ? matrix+0x30 : +4), +0x5A0 CAutomobile::m_damageManager.
- Behaviour:
  1. Return unless `(flags & 0x4000)` (0x4222A8) and `createdBy == 2` (0x4222AD) and `subType` (+0x594) is not 5, 4 or 3 (tested in
     that order, 0x4222BC-0x4222C9).
  2. `s = status >> 3` (0x4222CE). If `s == 3`: if `!HasCollisionLoaded(pos, 0)` (0x4222E7): `status byte = (byte & 7) | 0x60` (= STATUS_GHOST,
     0x4222F3-0x4222FA); then if `subType (+0x594) == 0`: `for (w = 0; w <= 3; w++) damageManager.SetWheelStatus(w, 0)` (0x422308-0x42231E).
  3. Else if `s == 12`: if `HasCollisionLoaded(pos, 0)` (0x422339): `status byte = (byte & 7) | 0x18` (= STATUS_PHYSICS, 0x422345-0x422356); then
     `type (+0x590) == 0` -> tail `CAutomobile::PlaceOnRoadProperly(veh)` (0x42236B), `type == 9` -> tail `CBike::PlaceOnRoadProperly(veh)` (0x422363),
     else return.
- Oddity: the ghost->physics branch tests m_nVehicleType (+0x590), while the physics->ghost branch and step 1 test m_nVehicleSubType (+0x594).
  Keep the two offsets exactly. Position is re-read for each HasCollisionLoaded call (no matrix allocation).
- gta-reversed: plugin call only. Open questions: none.

## 0x422370 CCarCtrl::FindGhostRoadHeight  (catalogue: specified/REA, port level 1)
- Convention: `__cdecl float (CVehicle* veh [esp+4])`, returns ST0, plain `ret`. Caller: CPhysical::ProcessCollision 0x54DFB0 (2 sites).
- Callees: CPathNode::GetPosition 0x420A10 (thiscall (CVector* out) -> EAX), 4 calls. Globals: m_pPathNodes table 0x96F854 (ThePaths+0x804),
  node stride 0x1C, 0x858B50 = 0.0 (f32).
- Fields: veh +0x390 autopilot m_currentAddress (CNodeAddress: u16 area +0, u16 node +2), +0x394 m_startingRouteNode (gr name; used here
  as the next node), +0x14/+0x04 position as usual.
- Behaviour:
  1. If `cur.area == 0xFFFF` or `next.area == 0xFFFF` (0x42237D/0x422386) return 0.0f. If `m_pPathNodes[cur.area] == null` (0x4223AB) or
     `m_pPathNodes[next.area] == null` (0x4223CD) return 0.0f.
  2. `z1 = GetPosition(curNode).z` (0x4223F3), `z2 = GetPosition(nextNode).z` (0x42241C) (both kept as f32 in memory).
  3. `V = veh pos`; `P = GetPosition(curNode)`; `dx = P.x - V.x`, `dy = P.y - V.y` (x87); `d1 = sqrt(dy*dy + dx*dx)` stored f32 (0x422450-0x422472).
  4. `Q = GetPosition(nextNode)`; same formula -> `d2` kept in x87 (0x42249F-0x4224BF).
  5. Return `((d1 * z2) + (d2 * z1)) / (d2 + d1)` (x87 order: `fld d1; fmul z2; fld d2; fmul z1; faddp; fxch; fadd d1; fdivp`, 0x4224C1-0x4224D7).
     (Linear interpolation of the node heights by horizontal distance.)
- Edge cases: `d1 + d2 == 0` (vehicle exactly on both nodes) gives NaN/inf. Only the area halves of the addresses are validated; the node index is
  not range-checked.
- gta-reversed: plugin call only. Open questions: none.

## 0x422590 CCarCtrl::StopCarIfNodesAreInvalid  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl bool (CVehicle* veh [esp+4])`, returns AL, plain `ret`. Entry 0x422590 is `jmp 0x1567AD0` (.HOODLUM body, same function).
- Callers: PickNextNodeToChaseCar 0x426EF0, SteerAICarWithPhysicsFollowPath 0x434900, ..._Racing 0x435830, UpdateCarOnRails 0x436540.
- Behaviour: in this order, "invalid" if any test fails (0x1567AD4-0x1567B48):
  `link = veh->+0x3A4` (u16, autopilot m_nCurrentPathNodeInfo) `!= 0xFFFF` and `m_pPathNodes[link >> 10] != 0` (table 0x96F854 - note the PATH NODE
  table, not the navi table, is tested for a link address); same for `+0x3A6` (m_nNextPathNodeInfo); `+0x390` (m_currentAddress.area, u16)
  `!= 0xFFFF` and `m_pPathNodes[area] != 0`; same for `+0x394`. All valid: return 0 (0x1567B4A). Invalid: `byte veh+0x3DC |= 1` (autopilot
  m_nMovementFlags.bIsStopped) and return 1 (0x1567B4E).
- gta-reversed: plugin call only. Open questions: none.

## 0x422760 CCarCtrl::ClitargetOrientationToLink  (catalogue: gr-unreversed, port level 0)
Name: gr's spelling (probably "ClipTargetOrientationToLink"); kept as catalogued. Catalogue extent is 1 byte because the entry is a detour:
`0x422760 jmp 0x401E69` -> `movzx eax, word [esp+8]; jmp 0x422765`; the body is 0x422765-0x422B18 (read with `dis.py 0x422765-0x422B20`).
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], CCarPathLinkAddress link [+8] (low u16 used), int8 laneSide [+0xC]
  (sign-extended), float* heading [+0x10], float targetX [+0x14], float targetY [+0x18])`, plain `ret`. Argument slots +8 are reused as scratch.
- Callers: SteerAICarWithPhysicsFollowPath_Racing 0x435830 (4 sites). Callees: CGeneral::GetATanOfXY 0x53CC70 (cdecl (float x, float y) -> ST0), 3 calls.
- Globals: m_pPathNodes 0x96F854, m_pNaviNodes 0x96F974 (ThePaths+0x924), CCarPathLink stride 0xE: +8 i8 dirX, +9 i8 dirY (scale 1/100),
  +0xA u8 width (read with movzx; gr declares FixedFloat<int8,16>), +0xB bits 0-2 lo (gr m_numOppositeDirLanes), bits 3-5 hi (gr m_numSameDirLanes).
  Constants (f32): 0x858C58 = 0.009999999776482582, 0x858B8C = 0.5, 0x858EEC = 0.011574073694646358 (= 1/86.4 = 1/(16*5.4)),
  0x858C24 = 0.30000001192092896, 0x858C50 = 5.400000095367432, 0x858CB8 = 3.1415927410125732, 0x858CBC = 6.2831854820251465,
  0x858CC0 = -3.1415927410125732, 0x858B50 = 0.0.
- Behaviour:
  1. `a = link & 0xFFFF; area = a >> 10` (0x422765-0x42276A). If `m_pPathNodes[area] == null` return (0x422778; the navi table is not tested).
     `L = m_pNaviNodes[area] + (a & 0x3FF) * 0xE` (0x42277E-0x42278C).
  2. `s = (float)(int)laneSide`; `fx = ((float)L.dirX * 0.01f) * s` -> f32 (0x4227BA-0x4227CA); `fy = ((float)L.dirY * 0.01f) * s` -> f32 (0x4227CE-0x4227DA).
  3. Dead call: `GetATanOfXY(targetX - pos.x, targetY - pos.y)` (y difference computed first), result discarded (0x4227E8-0x422807).
  4. `lo = L[+0xB] & 7`, `hi = (L[+0xB] >> 3) & 7`. Two widths A and B:
     - `lo == 0`: `A = B = (float)hi * 0.5f` (0x42281F-0x422829);
     - else `hi == 0`: `A = B = (float)lo * 0.5f` (0x42283F-0x422849);
     - else if `laneSide == 0`: `A = (float)width * 0.011574074f + (float)lo` (f32 store), `B = (float)hi` (0x422853-0x42289F);
     - else: `A = (float)width * 0.011574074f + (float)hi`, `B = (float)lo` (0x42286F-0x42289F).
     (In the first two cases A is stored f32 and B is the same x87 value.)
  5. `A' = A - 0.3f` (x87), `B' = B - 0.3f` stored f32 (0x4228A3-0x4228BD). Points:
     `P1x = ((fy * A') * 5.4f) + targetX` (f32), `P1y = targetY - ((A' * fx) * 5.4f)` (x87), `P2x = targetX - ((B' * fy) * 5.4f)` (f32),
     `P2y = ((B' * fx) * 5.4f) + targetY` (f32) (0x4228C1-0x42290B).
  6. `ang1 = GetATanOfXY(P1x - pos.x, P1y - pos.y)` stored f32 (0x42292E-0x422936); `ang2 = GetATanOfXY(P2x - pos.x, P2y - pos.y)` (0x422965).
     Position is re-read (matrix ? matrix+0x30 : +4) before each call.
  7. `h = *heading` (saved, 0x422974). `d1 = ang1 - h` stored f32 (0x42297A); `d2 = ang2 - h` (x87 only).
  8. Wrap `d1`: `if (d1 > pi) { do d1 -= 2pi; while (d1 > pi); }` then `if (d1 < -pi) { do d1 += 2pi; while (d1 < -pi); }` (0x42298A-0x4229E3; stored f32
     after each loop). Wrap `d2` the same way in x87 (0x4229E7-0x422A37, not stored).
  9. Choose (0x422A39-0x422AAD):
     - `d1 < 0 && d2 < 0`: `*heading = h + (d1 > d2 ? d1 : d2)` (equal or NaN -> d2) (the one closer to zero);
     - otherwise if `d1 > 0 && d2 > 0`: `*heading = h + (d1 < d2 ? d1 : d2)` (equal -> d2);
     - otherwise (signs differ, a zero, or NaN): no change (both popped, 0x422AAF).
     The stored value is `d + h` with d1 taken from its f32 copy and d2 from x87 (0x422A60/0x422AA7).
  10. Normalise `*heading`: `if (*heading < 0) { do x += 2pi; while (x < 0); store }` (0x422AB3-0x422AE3), then
      `if (*heading > 2pi) { do x -= 2pi; while (x > 2pi); store }` (0x422AE5-0x422B13; bound is 6.2831855 = the step itself, so 2pi exactly stays).
- Edge cases: heading is only clipped when both edge directions are on the same side of the current heading (i.e. the current heading points
  outside the lane corridor); the result range is [0, 2pi].
- gta-reversed: `plugin::Call` (CarCtrl.cpp:252), parameter names arg2..arg6. Open questions: none.

## 0x422B20 CCarCtrl::SteerAICarBlockingPlayerForwardAndBack  (catalogue: specified/REA, port level 2)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], float* outSteer [+8], float* outGas [+0xC],
  float* outBrake [+0x10], bool* outHandbrake [+0x14])`, plain `ret`. (gr names arg2..arg5; meaning from caller
  SteerAICarWithPhysics_OnlyMission 0x436A90 and the writes.) Frame: `sub esp,0x30` + push ebx/esi/edi; offsets below use
  S = esp after the three pushes (args: veh S+0x40, outSteer S+0x44, outGas S+0x48, outBrake S+0x4C, outHandbrake S+0x50).
  The outSteer slot is reused for f32 `t` (0x422C7C) and the outHandbrake slot for f32 `a` (0x422CEF) after their pointers
  were consumed (outHandbrake pointer kept in EBX).
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90. Callees: CWorld::FindPlayerSpeed 0x56E090 (cdecl
  `CVector* (int32 = -1)`), CWorld::FindPlayerEntity 0x56E120 (cdecl `CEntity* (int32 = -1)`), CPlaceable::AllocateMatrix
  0x54F560 + CSimpleTransform::UpdateMatrix 0x54F1B0 (ensure-matrix of the player entity), CVector::Normalise 0x59C910 (x2),
  CWorld::FindPlayerCoors 0x56E010 (cdecl `CVector* (CVector* out, int32 = -1)`, x2).
- Globals: none. Constants: 0x858B1C = 0.1 (f32), 0x858B50 = 0.0 (f32), 0x858C58 = 0.01 (f32, 0.009999999776482582),
  0x858624 = 1.0 (f32), 0x858EF4 = -0.1 (f32), 0x858EF0 = 0.95 (f32, 0.949999988079071), 0x858C1C = -1.0 (f32).
- Fields: entity +0x14 m_matrix (right row +0x00..+0x08, forward row +0x10..+0x18, pos +0x30), +0x04 placement pos;
  veh +0x44/+0x48/+0x4C CPhysical::m_vecMoveSpeed.
- Behaviour:
  1. `*outSteer = 0` (0x422B2E), `*outHandbrake = false` (0x422B36). The steer output is never written again.
  2. `W = *FindPlayerSpeed(-1)` copied to local S+0x24 (0x422B39-0x422B50). `pl = FindPlayerEntity(-1)`; ensure its matrix
     (0x422B5B-0x422B73). `W.x = W.x + pl.fwd.x * 0.1f` (0x422B7B/0x422B9F, stored 0x422BAD), `W.y = W.y + pl.fwd.y * 0.1f`
     (0x422B8B/0x422BB5-0x422BC7); W.z keeps the player's speed z (never used afterwards except multiplied by 0, see below).
  3. `R = veh->m_matrix->right` (no null check, 0x422B8E), `R.z = 0` (0x422BBF), `R.Normalise()` (0x422BCD).
     `F = veh->m_matrix->forward` (0x422BD2), `F.z = 0` (0x422BF0), `F.Normalise()` (0x422BF8).
  4. `D = FindPlayerCoors(-1) - veh->GetPosition()` (x, y, z in that order, 0x422C16-0x422C26).
     `p = (R.y*D.y + R.x*D.x) + R.z*D.z` (0x422C29-0x422C3D; R.z is 0).
     `q = (R.y*W.y + R.x*W.x) + R.z*0.0f` (0x422C43-0x422C5F; note: the third term multiplies by the 0.0 constant
     0x858B50 instead of W.z).
     If `q == 0.0f` exactly (0x422C61, `test ah,0x44; jp` not taken) `q = 0.01f` (0x422C70).
     `t = -(p / q)` (0x422C78/0x422C7A), stored f32 at S+0x44.
  5. If `t < 0.0f` (0x422C82, strict; NaN goes on): `*outGas = 0`, `*outBrake = 0` (0x422C99/0x422C9F), return.
  6. `D` recomputed via a second FindPlayerCoors (0x422CBE-0x422CD3).
     `a = (F.y*mv.y + F.z*mv.z) + F.x*mv.x` (0x422CD6-0x422CED; vehicle forward speed; stored f32 at S+0x50).
     `Y = ((F.y*W.y + F.x*W.x) + F.z*0.0f) * t + ((F.y*D.y + F.x*D.x) + F.z*D.z) - a * t`
     exact x87 order: `b = (F.y*W.y + F.x*W.x) + F.z*0.0f` (0x422CF3-0x422D0F), `bt = b * t` (0x422D11);
     `s = (F.y*D.y + D.x*F.x) + D.z*F.z` (0x422D15-0x422D29); `X = s + bt` (0x422D2D); `Y = X - a*t` (0x422D2F-0x422D37).
  7. If `Y > 0.0f` (0x422D3D, `test ah,0x41; jne` -> <=/NaN go to 8): `g = Y * 0.1f`; if `1.0f < g` then `g = 1.0f`
     (0x422D50-0x422D61; NaN keeps g); `*outGas = g` (0x422D6F), `*outBrake = 0` (0x422D73), return.
  8. Else if `a > 0.0f` (0x422D82; <=/NaN go to 9): `*outGas = 0` (0x422D9F); `b = Y * -0.1f` (0x422D8F), if `1.0f < b`
     `b = 1.0f` (0x422DA5-0x422DB0); `*outBrake = b` (0x422DBC); if `b > 0.95f` (0x422DBE, strict, NaN no)
     `*outHandbrake = true` (0x422DCD). Return.
  9. Else: `g = Y * 0.1f` (0x422DD5); if `-1.0f > g` then `g = -1.0f` (0x422DDB-0x422DEC: `fcomp` of -1.0 vs g, `jne` keeps g
     when -1 < g, equal or NaN); `*outGas = g` (0x422DFA), `*outBrake = 0` (0x422DFC). Return.
- Edge cases: the "W.z" terms are multiplied by the constant 0.0 (0x422C59, 0x422D09), so a NaN/inf only matters through
  R.z/F.z (finite after Normalise). If the vehicle has no matrix, 0x422B8E dereferences null (crash); the GetPosition calls
  later handle null but are effectively dead.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:943). Open questions: none.

## 0x423000 CCarCtrl::FlyAIPlaneInCertainDirection  (catalogue: specified/REA, port level 10)
- Convention / signature: `__cdecl void (CPlane* plane [esp+4])`, plain `ret` (0x423784). Entry is `jmp 0x401961`, a HOODLUM
  stub that loads EAX = `*(u32*)([0x1560698] + [0x1569C94])` = `*(u32*)(0x9863CAAD + 0x685400CB mod 2^32 = 0xB7CB78)` =
  `CTimer::m_snPreviousTimeInMilliseconds` (pushfd/clc/adc/popfd keep flags), then `jmp 0x423005`. Port: read
  m_snPreviousTimeInMilliseconds. Frame: `sub esp,0x18` + push ebx/esi/edi; S = esp after the pushes; the arg slot S+0x28
  is reused as f32 scratch (offset, target speed, bank angle).
- Callers: SteerAIPlaneTowardsTargetCoors 0x423790, SteerAIPlaneToFollowEntity 0x4237F0, GetAIPlaneToAttackPlayer 0x429780,
  GetAIPlaneToDoDogFight 0x429890, SteerAICarWithPhysics_OnlyMission 0x436A90.
- Callees: CGeneral::GetATanOfXY 0x53CC70 (cdecl (x, y) -> ST0, x2); FUN_00422F80 (cdecl `float (CVehicle* plane, float heading)`
  -> ST0; terrain probe: for each pitch in the .data table 0x8A5B2C..0x8A5B40 = {0.10471976, 0.05235988, 0.0, -0.34906587,
  -0.69813174, -1.0471976} calls FUN_00422E10(plane, pitch, heading, &h); if the first probe returns true it returns
  0x858EF8 = 100000.0, otherwise the max `h` of the true probes, 0.0 if none - not specified here); CPlane::SetGearUp 0x6CAC20
  (thiscall); CRT `_CIasin` 0x821E70 (ST0 -> ST0; identified by the core 0x821E8D: `fpatan(x, sqrt((1+x)(1-x)))`).
- Globals: CTimer::m_snPreviousTimeInMilliseconds 0xB7CB78 (via stub), CTimer::m_snTimeInMilliseconds 0xB7CB84 (u32),
  CTimer::ms_fTimeStep 0xB7CB5C (f32). Tunables in .data (read only here): 0x8A5B44 = 0.23 (f32), 0x8A5B48 = -1.0 (f32),
  0x8A5B4C = 30.0 (f32), 0x8A5B50 = 0.5 (f32), 0x8A5B54 = 7.0 (f32).
  Constants: 0x858CA0 = 2.0, 0x858CB8 = 3.1415927, 0x858CBC = 6.2831855, 0x858CC0 = -3.1415927, 0x858B50 = 0.0,
  0x858F20 = 0.5235988 (pi/6), 0x858CE8 = 1.5, 0x858F1C = 0.2617994 (pi/12), 0x858F18 = -0.2617994, 0x858A28 = 150.0,
  0x858F14 = 1.1, 0x858B34 = 60.0, 0x858D00 = 1.0, 0x858628 = 100.0, 0x858F10 = 0.033333335, 0x858EE8 = 0.4,
  0x858F0C = -0.4, 0x858C84 = 0.25, 0x85862C = 10.0, 0x858624 = 1.0, 0x858C1C = -1.0, 0x858C20 = 0.9, 0x858CAC = -0.9,
  0x858CB0 = 0.7, 0x858CD0 = -0.7, 0x858B1C = 0.1, 0x858B90 = 4.0, 0x858F08 = 1.2, 0x858B8C = 0.5 (all f32);
  0x858F00 = 20.0 (f64, compared against an f32 local). Immediates: 500.0f (0x43FA0000), 32.0f (0x42000000), 1.0f.
- Fields: entity +0x14 m_matrix (right +0x00/+0x04/+0x08, forward +0x10/+0x14/+0x18, up.z +0x28, pos +0x30), +0x04 placement
  pos, +0x20 u16 m_nRandomSeed, +0x44/+0x48/+0x4C m_vecMoveSpeed; +0x3BB autopilot m_nTempAction (i8); +0x42C bit 7 =
  vehicleFlags.bIsRCVehicle; CAutomobile +0x7D4..+0x7E0 m_fWheelsSuspensionCompression[4]; CPlane (gr Plane.h, base 0x988):
  +0x988 m_fLeftRightSkid (yaw/rudder out), +0x98C m_fSteeringUpDown (pitch out), +0x990 m_fSteeringLeftRight (roll out),
  +0x994 m_fAccelerationBreakStatus, +0x998 m_fAccelerationBreakStatusPrev, +0x99C m_fSteeringFactor (gr name; used here as
  previous bank angle), +0x9A4 m_planeCreationHeading, +0x9A8 m_maxAltitude, +0x9AC m_altitude (target), +0x9B0 m_minAltitude,
  +0x9B4 m_planeHeading (desired), +0x9B8 m_planeHeadingPrev (gr name; used as the chosen steering heading), +0x9BC m_forwardZ
  (previous pitch), +0x9C0 m_nStartedFlyingTime, +0x9CC m_LandingGearAngle.
- Behaviour:
  A. Route re-plan (once per 1000 ms of the seed-shifted clock), 0x42300E-0x42322C:
     `if ((u32)(seed + now) % 1000 < (u32)(prev + seed) % 1000)` (unsigned div 0x42301C/0x423030, `jae` skips A) with
     seed = (u16)+0x20, prev = m_snPreviousTimeInMilliseconds, now = m_snTimeInMilliseconds:
     1. `h = GetATanOfXY(fwd.x, fwd.y)` (matrix unchecked, 0x42303A-0x423045), kept in ST0.
        `desired = m_planeHeading` (0x42305D); by m_nTempAction: 16 -> `desired = h` (0x423085); 17 -> `desired = h - 2.0f`
        (`fsubr` 0x42307D); 18 -> `desired = 2.0f + h` (0x42306F). (stored f32 at S+0xC)
     2. `m_altitude = 500.0f` (0x42308F); `m_planeHeadingPrev = m_planeHeading + pi` (0x423089-0x42309F).
     3. `diff = h - desired` (0x4230A5); `while (diff > pi) diff -= 2pi` (0x4230A9-0x4230D1); `while (diff < -pi) diff += 2pi`
        (0x4230D3-0x4230F1). `maxDev = (|diff| < pi/6) ? 0.0f : diff * 1.5f` (abs via `fchs` 0x423102 when diff < 0;
        `fcomp` 0x423104, `jp` -> >= or NaN uses diff*1.5f 0x42311D; stored S+0x10).
     4. Two passes `pass = 0, 1` (edi, 0x423127/0x423229); inner `i = 1..19` (ebx, `cmp ebx,0x14; jl` 0x4231E6):
        `k = i / 2` (signed, `cdq; sub; sar` 0x423137); `off = (float)k * (i odd ? 0.2617994f : -0.2617994f)` (fild + fmul,
        stored f32 0x42315F) -> sequence 0, -pi/12, +pi/12, -2pi/12, +2pi/12, ..., -9pi/12, +9pi/12. `cand = off + desired`
        (0x423163-0x42316B, f32).
        `inside = (off < 0 && off > maxDev) || (off > 0 && off < maxDev)` (0x42316F-0x4231B3; strict compares; off = 0 -> false).
        Pass 0 probes only `inside` candidates, pass 1 only `!inside` ones (0x4231B5-0x4231C6).
        `r = FUN_00422F80(plane, cand)` (0x4231CE); if `r < 150.0f` (0x4231D3, `test ah,5; jnp` -> strictly less):
        `m_planeHeadingPrev = off * 1.1f + m_planeHeading` (0x4231F1-0x423201; note m_planeHeading, not `desired`);
        `alt = r + m_minAltitude` (0x423207); `m_altitude = (alt > m_maxAltitude) ? alt : m_maxAltitude` (0x42320D, `je` keeps
        alt only when strictly greater; NaN -> m_maxAltitude) (0x423222); then the inner loop ends (jmp 0x423228) and the NEXT
        PASS still runs.
  B. Steering (always), 0x423232-0x423784:
     5. `tgtSpeed = bIsRCVehicle ? [0x8A5B54] (7.0) : 32.0f` (0x42323A-0x423247, S+0x28).
        `h2 = GetATanOfXY(fwd.x, fwd.y)` (0x42325A, f32 S+0x10). `fs = (fwd.y*mv.y + fwd.x*mv.x) * 60.0f` (0x423266-0x42327D, f32 S+0xC).
     6. If any of m_fWheelsSuspensionCompression[0], [1], [2], [3] `< 1.0f` (checked in that order, 0x423281-0x4232CB; NaN = not less)
        `m_nStartedFlyingTime = now` (0x4232D2).
     7. Ground mode when `(u32)(now - m_nStartedFlyingTime) <= 4000` (0x4232E4 `jbe`): `gnd = 1` (0x423378);
        `m_fSteeringUpDown = (fs < tgtSpeed) ? 0.0f : 0.4f` (0x423374-0x423391; `jp` -> >= or NaN gives 0.4f).
        Air mode otherwise (0x4232F0): `gnd = 0`; if `m_LandingGearAngle` bits != 0x3F800000 call `SetGearUp()` (0x4232F8-0x423301).
        `alt = mv.z * 100.0f + pos.z` (pos = matrix ? +0x30 : +4; 0x423306-0x42331F, f32 S+0x20).
        `a = asin(fwd.z)` (0x423323/0x423326, matrix unchecked); `pp = (a - m_forwardZ) * (100.0f / ms_fTimeStep) + a`
        (0x42332B-0x423341, f32 S+0x14); `m_forwardZ = a` (0x423347).
        `up = (m_altitude - alt) * 0.033333335f` (0x42334D-0x423357); if `!(up < 0.4f)` up = 0.4f (0x42335D-0x42336C, NaN -> 0.4);
        else if `!(up > -0.4f)` up = -0.4f (0x42339D-0x4233AC).
        If `fs < tgtSpeed` (0x4233B2) and `!(up < 0.25f)` then up = 0.25f (0x4233C1-0x4233D0).
        `m_fSteeringUpDown = (up - pp) * [0x8A5B50] (0.5)` (0x4233D6-0x4233E0).
     8. `d = m_planeHeadingPrev - h2` (0x4233E6); `while (d < -pi) d += 2pi`; `while (d > pi) d -= 2pi` (0x4233F0-0x423431).
     9. Ground (`gnd`, 0x423433): `s = -(m_planeCreationHeading - h2) * 10.0f` (no wrap; 0x423439-0x423445); if `1.0f < s` s = 1.0f
        (0x42344B-0x42345C) else if `-1.0f > s` s = -1.0f (0x423464-0x423475); `m_fLeftRightSkid = s` (0x42347B),
        `m_fSteeringLeftRight = 0` (0x423481); go to step 12.
     10. Air: `r = -d * 1.5f` (0x423490/0x423492); if `0.9f < r` r = 0.9f else if `-0.9f > r` r = -0.9f (0x423498-0x4234C2).
         If bIsRCVehicle (0x4234C8): if `0.7f < r` r = 0.7f, else if `-0.7f > r` r = -0.7f (0x4234D2-0x42351D).
         `ar = |r|` (0x423529-0x42353A, also after the RC clamps). If `ar < 0.1f` (0x4234EB): `m_fLeftRightSkid = r * 4.0f`
         (0x4234F8), `roll = 0.0f` (0x423504); else `m_fLeftRightSkid = r` (0x42353C), `roll = -r` (0x423542).
         `v = tgtSpeed * 1.2f` (0x423544); if `fs < v` (0x42354E): `f = 1.0f - (v - fs) / (tgtSpeed * 0.5f)` (0x42355B-0x42356B);
         if `0.0f > f` f = 0.0f (0x423571-0x423582); `roll *= f` (0x423588).
         Bank: if matrix (0x42358E): `n = sqrt(rx*rx + ry*ry)` (right.x, right.y, 0x423592-0x4235A1); if `up.z < 0.0f` (+0x28,
         0x4235A7) `n *= -1.0f`; `bank = atan2(right.z, n)` (`fpatan` 0x4235C2), stored f32 S+0x28; no matrix -> bank = 0 (0x4235CA).
         `tsc = (1.0f > ms_fTimeStep) ? 1.0f : ms_fTimeStep` (0x4235D2-0x4235ED; NaN -> timestep).
         `pb = (bank - m_fSteeringFactor) * ([0x8A5B4C] 30.0 / tsc) + bank` (0x4235F3-0x423607); `lr = roll - pb` (`fsubp` 0x42360B).
         `while (lr > pi) lr -= 2pi`; `while (lr < -pi) lr += 2pi` (0x42360F-0x423651). `lr *= [0x8A5B48] (-1.0)` (0x423653).
         `m_fSteeringFactor = bank` (0x42365D). `m_fSteeringLeftRight = clamp(lr)`: `lr < 1.0f ? lr : 1.0f`, then `> -1.0f ? : -1.0f`
         (0x423663-0x423699; the field is also written with intermediates at 0x423669/0x423684, final store 0x423699).
         `u = m_fSteeringUpDown`; `t = 0.23f * bank` ([0x8A5B44], 0x4236A5); `m_fSteeringUpDown = |t| + u` (abs via `fchs` when t < 0,
         0x4236AF-0x4236C8). If `u < 0.0f` (0x4236CE): `h = u * 0.5f` (0x4236DB); `m_fSteeringUpDown = (h > (|t| + u)) ? (|t| + u) : h`
         (0x4236E9-0x4236FA).
     11. (both modes continue here, 0x423704)
     12. Clamp `m_fSteeringUpDown`: `(x < 1.0f) ? x : 1.0f` (NaN -> 1.0f), then `(x > -1.0f) ? x : -1.0f` (0x423704-0x423746).
         `m_fAccelerationBreakStatus = m_fAccelerationBreakStatusPrev` (raw dword copy 0x42374E/0x423754).
         If `m_nTempAction == 15` (TEMPACT_PLANE_FLY_UP, 0x42374C): `m_fSteeringUpDown = 1.0f` (0x423760); if `fs < 20.0` (f64
         0x858F00, 0x42376A) `m_nTempAction = 0` (0x423777).
- Edge cases / oddities: (a) step 4 does not stop after a hit in pass 0: pass 1 runs and can overwrite m_planeHeadingPrev /
  m_altitude (likely an unintended missing double break; keep). (b) The accepted heading is `m_planeHeading + 1.1*off`, while
  the probe used `desired + off` (differs for temp actions 16-18). (c) Matrix is dereferenced without null checks at 0x42303A,
  0x42324F, 0x423263, 0x423323; only the pos read and the bank block test for null. (d) ms_fTimeStep = 0 in step 7 gives inf.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:450). Open questions: semantics of FUN_00422E10's bool/out value
  (called from FUN_00422F80 at 0x422FAA) are outside this spec; this function only uses the returned float (0x4231D3).

## 0x423790 CCarCtrl::SteerAIPlaneTowardsTargetCoors  (catalogue: specified/REA, port level 11)
- Convention: `__cdecl void (CPlane* plane [esp+4])` (gr types it CAutomobile*), plain `ret`. Caller: SteerAICarWithPhysics_OnlyMission 0x436A90.
- Callees: CGeneral::GetATanOfXY 0x53CC70 (cdecl (x, y) -> ST0), CCarCtrl::FlyAIPlaneInCertainDirection 0x423000 (cdecl (CPlane*)).
- Fields: +0x3EC/+0x3F0 autopilot m_vecDestinationCoors.x/.y (0x390 + 0x5C), +0x9B4 CPlane m_planeHeading (gr name, f32), pos = m_matrix ? +0x30 : +4.
- Behaviour: `plane->+0x9B4 = GetATanOfXY(dest.x - pos.x, dest.y - pos.y)` (the y difference is computed first, 0x4237A5-0x4237C6; stored f32);
  then `FlyAIPlaneInCertainDirection(plane)` (0x4237CC). Altitude fields are not touched.
- gta-reversed: plugin call only. Open questions: none.

## 0x4237F0 CCarCtrl::SteerAIPlaneToFollowEntity  (catalogue: specified/REA, port level 11)
- Convention: `__cdecl void (CPlane* plane [esp+4])`, plain `ret`. Caller: SteerAICarWithPhysics_OnlyMission 0x436A90.
- Callees: GetATanOfXY 0x53CC70, FlyAIPlaneInCertainDirection 0x423000.
- Fields: +0x41C autopilot m_TargetEntity (CVehicle*, 0x390 + 0x8C; dereferenced without a null check), +0x9B4 m_planeHeading, +0x9A8 m_maxAltitude
  (gr names).
- Behaviour: `T = target pos`, `V = plane pos` (matrix-or-placement each, 0x4237F6-0x42382D);
  `plane->+0x9B4 = GetATanOfXY(T.x - V.x, T.y - V.y)` (y difference computed first, 0x423830-0x42384F); then target pos is re-read and
  `plane->+0x9A8 = T.z` (raw dword copy, 0x423867-0x42386B); then `FlyAIPlaneInCertainDirection(plane)` (0x423871).
- gta-reversed: plugin call only. Open questions: none.

## 0x423880 CCarCtrl::SteerAIPlaneToCrashAndBurn  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (CPlane* plane [esp+4])`, plain `ret`. Entry `jmp 0x156A3A0` (.HOODLUM body). Caller: SteerAICarWithPhysics_OnlyMission 0x436A90.
- Behaviour: `odd = entity->m_nRandomSeed (+0x20, u16) & 1` (0x156A3A4). Stores (CPlane gr names): `+0x98C m_fSteeringUpDown = -0.3f` (imm 0xBE99999A,
  0x156A3AC); `+0x988 m_fLeftRightSkid = odd ? 1.0f : -1.0f` (0x156A3B8/0x156A3C4); `+0x994 m_fAccelerationBreakStatus = 0` (0x156A3D3);
  `+0x990 m_fSteeringLeftRight = odd ? 1.0f : -1.0f` (0x156A3DB/0x156A3E6).
- gta-reversed: plugin call only. Open questions: none.

## 0x4238E0 CCarCtrl::SteerAIHeliToCrashAndBurn  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (CHeli* heli [esp+4])`, plain `ret`. Caller: SteerAICarWithPhysics_OnlyMission 0x436A90.
- Behaviour: `odd = m_nRandomSeed (+0x20, u16) & 1`. Stores (CHeli gr names): `+0x990 m_fSteeringUpDown = -0.3f` (imm 0xBE99999A, 0x4238EC);
  `+0x98C m_fLeftRightSkid = odd ? +0xA14 (raw dword copy) : -(+0xA14)` (`fld; fchs; fstp`, 0x4238F8/0x423906; +0xA14 is gr `field_A14`, f32);
  `+0x998 m_fAccelerationBreakStatus = -0.5f` (imm 0xBF000000, 0x423917); `+0x994 m_fSteeringLeftRight = odd ? 1.0f : -1.0f` (0x423923/0x42392E).
- Note the CHeli field block starts 4 bytes later than CPlane's (CHeli +0x988 is the u8 m_nHeliFlags), so the two crash functions write
  analogous controls at different offsets.
- gta-reversed: plugin call only. Open questions: none.

## 0x423940 CCarCtrl::FlyAIHeliToTarget_FixedOrientation  (catalogue: specified/REA, port level 8)
- Convention / signature: `__cdecl void (CHeli* heli [esp+4], float orientation [+8], CVector target (by value) [+0xC/+0x10/+0x14])`,
  plain `ret`. Frame `sub esp,0x60` + push esi/edi; S = esp after the pushes (heli S+0x6C, orientation S+0x70, target.x S+0x74,
  target.y S+0x78, target.z S+0x7C). Arg slots are reused: S+0x6C receives ProcessLineOfSight's `outEntity` (0x423A30) and
  later `rand() & 0xF` and the lateral speed; S+0x70 the lateral error; S+0x74 the forward speed.
- Callers: CCarCtrl::SteerAIHeliTowardsTargetCoors 0x42A630, CCarCtrl::SteerAIHeliToFollowEntity 0x42A750.
  Callees: CVector::Normalise 0x59C910, CWorld::ProcessLineOfSight 0x56BA00 (cdecl, 12 args, `add esp,0x30`),
  CGeneral::GetATanOfXY 0x53CC70 (cdecl (x, y) -> ST0), CRT `rand` 0x821B1E.
- Globals: CTimer::m_snPreviousTimeInMilliseconds 0xB7CB78, CTimer::m_snTimeInMilliseconds 0xB7CB84 (u32).
  Constants (f32 unless noted): 0x858B40 = 50.0, 0x858B34 = 60.0, 0x858628 = 100.0, 0x858B50 = 0.0, 0x858B1C = 0.1,
  0x858CC4 = 0.2, 0x858F48 = 7.0, 0x858F44 = 0.002, 0x858624 = 1.0, 0x858EE4 = -0.3, 0x858CB8 = 3.1415927, 0x858CBC = 6.2831855,
  0x858CC0 = -3.1415927, 0x858F40 = -0.5, 0x858C1C = -1.0, 0x858A14 = 80.0, 0x858C80 = 5.0, 0x858F34 = 0.75, 0x858F30 = -0.75,
  0x858B8C = 0.5; 0x858F38 = -0.02 (f64), 0x858F28 = -0.015 (f64) (both multiply f32 values in x87). Immediates -1.0f, +-0.5f.
- Fields: entity +0x14 m_matrix (right +0x00..+0x08, forward +0x10..+0x18, pos +0x30), +0x04 placement pos, +0x20 u16
  m_nRandomSeed, +0x44/+0x48/+0x4C m_vecMoveSpeed. CHeli (gr Heli.h, base 0x988): +0x98C m_fLeftRightSkid (yaw out),
  +0x990 m_fSteeringUpDown (pitch out), +0x994 m_fSteeringLeftRight (roll out), +0x998 m_fAccelerationBreakStatus (lift out),
  +0x9A8 m_fMaxAltitude, +0x9AC field_9AC (target altitude), +0x9B0 m_fMinAltitude.
- Behaviour:
  1. Every 500 ms of the seed-shifted clock: `if ((u32)(seed + now) % 500 < (u32)(prev + seed) % 500)` (0x423940-0x423973;
     prev read first at 0x423940, seed = (u16)+0x20):
     a. `field_9AC = m_fMaxAltitude` (raw copy 0x423979/0x423984).
     b. Start `P = (mv.x*50.0f + pos.x, mv.y*50.0f + pos.y, mv.z*50.0f + pos.z)` (products stored f32 first for x/y,
        0x42398A-0x4239DB; pos = GetPosition()).
     c. Direction `D = (cos(orientation), sin(orientation), -1.0f)` (`fcos`/`fsin` on the f32 arg, 0x4239DF-0x4239EF;
        z immediate 0x4239CB); `D.Normalise()` (0x4239F3).
     d. End `E = (D.x*60.0f + P.x, D.y*60.0f + P.y, D.z*60.0f + P.z)` (0x4239F8-0x423A5B).
        `hit = ProcessLineOfSight(P, E, colPoint, outEntity (into heli's arg slot), buildings 1, vehicles 0, peds 0, objects 0,
        dummies 0, seeThrough 0, cameraIgnore 0, shootThrough 1)` (0x423A5F).
     e. If hit: `z = colPoint.m_vecPoint.z + m_fMinAltitude` (0x423A6B-0x423A6F); `field_9AC = (z < field_9AC) ? field_9AC : z`
        (0x423A75-0x423A8A; `jp` keeps z when >= or NaN).
  2. `h = GetATanOfXY(fwd.x, fwd.y)` (matrix unchecked, 0x423A90-0x423A9B), f32 S+8.
  3. Lift: `m_fAccelerationBreakStatus = 0` (0x423AC2, overwritten); `e = field_9AC - (mv.z*100.0f + pos.z)` (0x423AB6-0x423ACC);
     `a = (e > 0.0f) ? e*0.1f : e*0.2f` (0x423AD2-0x423AE7; `jne` -> <=/NaN use 0.2), stored (0x423AED).
     `v = ((float)(rand() & 0xF) - 7.0f) * 0.002f + a` (0x423AF3-0x423B0F; fild of the masked int).
     Clamp: if `v < 1.0f` (0x423B15): if `-0.3f > v` v = -0.3f (0x423B28-0x423B3D); then if `!(v < 1.0f)` v = 1.0f (0x423B45-0x423B54).
     `m_fAccelerationBreakStatus = v` (0x423B5A; also intermediate store 0x423B1B).
  4. Yaw: `y = orientation - h` (0x423B60/0x423B64); `while (y > pi) y -= 2pi` (0x423B68-0x423B91); `while (y < -pi) y += 2pi`
     (0x423B93-0x423BB1); `y *= -0.5f` (0x423BB3); clamp to [-1, 1] in the same pattern (if < 1: if -1 > y -> -1; then
     `!(y < 1)` -> 1; 0x423BB9-0x423BF8); `m_fLeftRightSkid = y` (0x423C01).
  5. `M = m_matrix` (eax, 0x423BFE; dereferenced below without null check, crash if null); `pos = GetPosition()`.
     `dx = target.x - pos.x`, `dy = target.y - pos.y` (0x423C11-0x423C1B).
     `sR = (mv.z*R.z + mv.y*R.y) + R.x*mv.x` (0x423C1E-0x423C31) -> S+0x6C; `sF = (mv.z*F.z + mv.y*F.y) + mv.x*F.x`
     (0x423C37-0x423C4B) -> S+0x74.
     `latErr = ((dy*R.y + dx*R.x) + R.z*0.0f) + sR*80.0f` (0x423C51-0x423C73) -> S+0x70.
     `fwdErr = ((dy*F.y + dx*F.x) + F.z*0.0f) + sF*80.0f` (0x423C77-0x423C96), kept in ST0.
  6. Roll: if `|latErr| < 5.0f` (0x423C98-0x423CBA; NaN -> else) `m_fSteeringLeftRight = sR` (raw copy of the lateral speed,
     0x423CBC/0x423CC0) else `m_fSteeringLeftRight = latErr * -0.02` (f64 0x858F38, 0x423CCC).
     Clamp to [-0.75, 0.75]: if `x < 0.75f`: if `-0.75f > x` x = -0.75f (0x423CD8-0x423D02); else-path 0x423D0A: `x < 0.75f ? x : 0.75f`;
     stored 0x423D2B.
  7. Pitch: if `|fwdErr| < 5.0f` (0x423D31-0x423D4D) `m_fSteeringUpDown = sF` (raw copy 0x423D4F/0x423D55) else
     `m_fSteeringUpDown = fwdErr * -0.015` (f64 0x858F28, 0x423D5D). Clamp to [-0.5, 0.5]: if `x < 0.5f` and `-0.5f > x` ->
     -0.5f (0x423D94); else `x < 0.5f ? x : 0.5f` (0x423DA3-0x423DC9). Return.
- Edge cases / oddities: (a) inside the dead zone (|err| < 5) roll/pitch are set to the raw lateral/forward SPEED (not 0 and not
  scaled) - original behaviour, keep. (b) The LOS start is the position predicted 50 frames ahead, direction is the requested
  orientation tilted 45 degrees down (z = -1 before normalising), length 60. (c) `rand()` is consumed every call.
  (d) The ProcessLineOfSight outEntity is written into the caller's argument slot (cdecl, harmless).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:445). Open questions: none.

## 0x423DE0 CCarCtrl::RegisterVehicleOfInterest  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret`. Caller: CPlayerPed::ProcessControl 0x60EA90.
- Globals: `apCarsToKeep[2]` 0x969084 (CVehicle*), `aCarsToKeepTime[2]` 0x96907C (u32), `CTimer::m_snTimeInMilliseconds` 0xB7CB84.
- Behaviour (indices are i16 loop counters, all loops ascending 0..1):
  1. If `apCarsToKeep[i] == veh` for some i (first match, 0x423DF0-0x423E01): `aCarsToKeepTime[i] = now` (0x423E6E) and return.
  2. Else the first `i` with `apCarsToKeep[i] == null` (0x423E10-0x423E23): `apCarsToKeep[i] = veh; aCarsToKeepTime[i] = now` (0x423E7F).
  3. Else (both slots used): `best = 0; bestT = 0xFFFFFFFF`; for i in 0..1: if `apCarsToKeep[i] != null` and `aCarsToKeepTime[i] < bestT` (unsigned,
     `jae` skip, 0x423E42): `best = i; bestT = time[i]` (0x423E2C-0x423E50). Replace slot `best` with `(veh, now)` (0x423E60/0x423E66).
     Ties keep the lower index.
- gta-reversed: plugin call only. Open questions: none.

## 0x423F00 CCarCtrl::ClearInterestingVehicleList  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void ()`. Entry `jmp 0x15649A0` (.HOODLUM body). Callers: CGameLogic::RestorePlayerStuffDuringResurrection 0x442060,
  ResetStuffUponResurrection 0x442980, Update 0x442AD0 (3 sites).
- Behaviour: `apCarsToKeep[0] = apCarsToKeep[1] = null` (0x969084 / 0x969088, 0x15649A2/0x15649A7). `aCarsToKeepTime` (0x96907C) is not cleared.
- gta-reversed: plugin call only. Open questions: none.

## 0x423FC0 CCarCtrl::SwitchVehicleToRealPhysics  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret`. Entry `jmp 0x1569420` (.HOODLUM body). 12 callers (CCarAI::UpdateCarAI 0x41DA30,
  PickNextNodeToFollowPath 0x427740, SlowCarDownForOtherCar 0x42D0E0, ProcessCommands900To999 0x483BD0, CPhysical collision 0x54BA60/0x54CFF0/0x54DFB0,
  TriggerExplosionSectorList 0x567750, PreparePedVehicleForPedDeath 0x62FD00, BurstTyre 0x6A32B0/0x6BEB20, SetTowLink 0x6B4410).
- Behaviour, in order: status byte `+0x36 = (+0x36 & 7) | 0x18` (STATUS_PHYSICS = 3 in bits 3-7, 0x1569424-0x156942D); `+0x3BB = 0` (autopilot
  +0x2B m_nTempAction = 0); `+0x3AC = now + 2000` (autopilot m_nTimeToStartMission, 0x1569438-0x1569444); `+0x3B0 = now` (m_nTimeSwitchedToRealPhysics,
  `now` re-read, 0x156944A); `byte +0xB8 = 0` (CPhysical, gr m_nNumEntitiesCollided, 0x1569456). `now` = 0xB7CB84.
- gta-reversed: plugin call only. Open questions: none.

## 0x424000 CCarCtrl::UpdateCarCount  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (CVehicle* veh [esp+4], bool decrease [esp+8])`, plain `ret`. Contains a detour: `0x42400E jmp 0x4076A5`
  (`movzx eax, byte [ecx+0x4A4]; jmp 0x424013`); the `cmp al, dl` flags from 0x42400A survive the detour (movzx/jmp do not change flags) and
  drive `jne` at 0x424013. Callers: CPools::LoadVehiclePool 0x5D2A20, CVehicle::SetVehicleCreatedBy 0x6D5D70 (2), CVehicle ctor 0x6D5F10, dtor 0x6E2B40.
- Fields: +0x4A4 m_nCreatedBy (u8: 1 RANDOM, 2 MISSION, 3 PARKED, 4 PERMANENT), +0x428 byte bit 0 (gr vehicleFlags.bIsLawEnforcer).
- Globals (i32): NumRandomCars 0x969094, NumLawEnforcerCars 0x969098, NumMissionCars 0x96909C, NumParkedCars 0x9690A0, NumPermanentVehicles 0x9690A4.
- Behaviour, `c = createdBy`; values outside 1..4 do nothing (unsigned `ja`, 0x424019/0x42406D). Jump tables 0x4240CC (increase) / 0x4240DC (decrease).
  - Increase (`decrease == 0`): c=1: `if (flags & 1) NumLawEnforcerCars++; NumRandomCars++` (0x424026). c=2: `if (flags & 1) { flags &= ~1 (writes the
    vehicle!); NumLawEnforcerCars--; } NumMissionCars++` (0x42403C; no clamp on this decrement). c=3: NumParkedCars++ (0x42405B). c=4: NumPermanentVehicles++ (0x424062).
  - Decrease: c=1: `if (flags & 1) { if (--NumLawEnforcerCars < 0) NumLawEnforcerCars = 0; } if (--NumRandomCars < 0) NumRandomCars = 0` (0x424076-0x424095).
    c=2: `if (--NumMissionCars < 0) = 0` (0x42409C). c=3: same for NumParkedCars (0x4240AB). c=4: same for NumPermanentVehicles (0x4240BA).
    (`dec mem; jns` - the clamp tests the sign of the decremented value.)
- Oddities: increasing a mission vehicle clears its law-enforcer flag; decreasing a mission vehicle never touches NumLawEnforcerCars.
- gta-reversed: plugin call only. Open questions: none.

## 0x424210 CCarCtrl::GenerateCarCreationCoors2  (catalogue: specified/REA, port level 6)
- Convention / signature: `__cdecl bool (CVector pos [esp+4..+0xC], float dirX [+0x10], float dirY [+0x14], float dotLimit [+0x18],
  bool wantAhead [+0x1C], float range1 [+0x20], float range2 [+0x24], CVector* outCoors [+0x28], CNodeAddress* outNode1 [+0x2C],
  CNodeAddress* outNode2 [+0x30], float* outFrac [+0x34], bool restrict [+0x38], bool noRandomStart [+0x3C])`, plain `ret`,
  result in AL (0 at 0x424B34 / 0x424C07, 1 at 0x424CCE). Frame `sub esp,0x178` + push ebx/ebp/esi/edi, so arg N is at
  `[esp+0x18C + N]` in the listing. gr's prototype (CarCtrl.h:79) names these `posn, radius, arg3, arg4, arg5, arg6, arg7,
  origin, nodeAddress1, nodeAddress2, arg11, arg12, arg13`; the bytes show `radius`/`arg3` are a 2D direction (dot product
  at 0x424BDB-0x424BF1), not a radius (name correction). The function writes `range1` back into its own argument slot
  (`range1 *= 1.5f` at 0x424459-0x424468).
- Callers: CCarCtrl::GenerateOneEmergencyServicesCar 0x42B7D0, CCarCtrl::GenerateOneRandomCar 0x430050, CGangWars::CreateAttackWave
  0x444810.
- Callees (via `orig!`): CPathFind::Find2NodesForCarCreation 0x452090 (thiscall ThePaths 0x96F050, `(CVector pos, CNodeAddress* a,
  CNodeAddress* b, bool)`, `ret 0x18`), CPathFind::FindNodeClosestToCoors 0x44F460 (thiscall, hidden return pointer first,
  `ret 0x2C`), CCarPathLink::GetNodeCoors-like 0x420A60 (thiscall, `(CVector* out)` -> out = (posn.x*0.125, posn.y*0.125, 0),
  returns out, `ret 4`), CPathNode::GetPosition 0x420A10 (thiscall `(CVector* out)` -> (x,y,z)*0.125, `ret 4`),
  CCamera::IsSphereVisible 0x420D40 (thiscall TheCamera 0xB6F028, `(const CVector*, float)`, `ret 8`),
  CPathFind::ThisNodeWillLeadIntoADeadEnd 0x44D310 (thiscall, `(CPathNode* a, CPathNode* b)`, `ret 8`), `_rand` 0x821B1E (5 sites),
  `eh vector constructor iterator` 0x403D60 (stdcall `(ptr, 4, 30, 0x4241A0)`; 0x4241A0 = CNodeAddress ctor: `area = 0xFFFF`
  only), `sprintf` 0x821BB5 (cdecl).
- Globals: function-local statics with guard dword 0x969108 (bits 1,2,4,8,0x10): CNodeAddress s_a 0x969104 (area u16 0x969104,
  node u16 0x969106), s_b 0x969100, s_c 0x9690FC, s_d 0x9690F8 (guard bits 1/2/4/8 set `area = 0xFFFF`, node word untouched,
  0x424230-0x424271; bit 0x10 only sets the guard, 0x42427C), u32 s_nextRefreshTime 0x9690E8, CVector s_lastPos 0x9690EC/F0/F4.
  CCarCtrl::SequenceElements 0x969078 (i32, W), SequenceRandomOffset 0x969074 (i32, W), bSequenceOtherWay 0x969070 (u8, W),
  CPopulation::m_bMoreCarsAndFewerPeds 0xC0FCB5 (u8, W), CTimer::m_snTimeInMilliseconds 0xB7CB84, ThePaths 0x96F050:
  m_pPathNodes 0x96F854 (+0x804), m_pNaviNodes 0x96F974 (+0x924), m_pNodeLinks 0x96FA94 (+0xA44), m_pNaviLinks 0x96FDF4
  (+0xDA4); gString 0xB71670 (W by sprintf). Format string 0x858F58 = "tell Obbe it happened again %d/%d %d/%d".
- Constants: 0x85862C = 10.0 (f32), imm 0x1388 = 5000, 0x858A10 = 40.0 (f32), imm 0x42480000 = 50.0f, 0x858CE8 = 1.5 (f32),
  0x858F80 = 230.0 (f32), 0x858C48 = 0.125 (f32, node coordinate scale), 0x858B50 = 0.0 (f32), imm 0x40A00000 = 5.0f
  (sphere radius), 0x858624 = 1.0 (f32), 0x858B8C = 0.5 (f32), imm 0x1E = 30 (visited capacity).
- Structures: CPathNode (0x1C): +8/+0xA/+0xC i16 pos (x8), +0x10 i16 m_wBaseLinkId, +0x12 u16 m_wAreaId, +0x18 byte0 (bits 0-3
  m_nNumLinks, 0x10 m_onDeadEnd, 0x20 m_isSwitchedOff), +0x19 byte1 (0x10 m_bNotHighway, 0x20 m_bHighway). CCarPathLink (0xE):
  +0 i16 x/y, +0xB byte (bits 0-2 m_numOppositeDirLanes, bits 3-5 m_numSameDirLanes). Node address = dword (area lo word,
  node hi word). "node(A)" = `m_pPathNodes[A.area] + A.node*0x1C`; "pos2(n)" = `((float)(i16)n.x * 0.125, (float)(i16)n.y * 0.125)`.
- Behaviour:
  1. Statics init (0x424216-0x42427F) as above. `accDist = 0.0` (int 0 stored to `[esp+0x30]`, 0x424221).
  2. Refresh test (0x424284-0x4242DB): `d = sqrt((dx*dx) + (dy*dy))` with `dx = pos.x - s_lastPos.x`, `dy = pos.y - s_lastPos.y`
     (x87: dx^2 + dy^2). If `d > 10.0f` (`test ah,0x41; je`) or `now > s_nextRefreshTime` (unsigned, `jbe` skips) refresh:
     `Find2NodesForCarCreation(pos, &s_a, &s_b, true)` (0x4242FF), `Find2NodesForCarCreation(pos, &s_c, &s_d, false)` (0x424322),
     `s_nextRefreshTime = now + 5000`, `s_lastPos = pos` (raw dwords, 0x42433F-0x42434B). Then (still inside the refresh branch)
     the m_bMoreCarsAndFewerPeds update, all exits go to step 3:
     a. Skip (flag unchanged) if `s_a.area == 0xFFFF` (0x424351) or `m_pPathNodes[s_a.area] == null` (0x424363).
     b. `n = node(s_a)`; `nl = m_pNaviLinks[n.m_wAreaId][n.m_wBaseLinkId]` (u16, 0x424369-0x424385); skip (unchanged) if
        `m_pPathNodes[nl >> 10] == null` (0x424397; note: tests the path-node area table, not the navi table);
        `link = m_pNaviNodes[nl >> 10] + (nl & 0x3FF)*0xE`.
     c. If `(link[+0xB] & 7) < 2 && (link[+0xB] & 0x38) < 0x10` (fewer than 2 lanes each way, 0x4243AB-0x4243BE): flag = 0.
     d. Else `c = link->GetNodeCoors()` (0x4243CA); `e = sqrt((cy-pos.y)^2 + (cx-pos.x)^2)` (x87 sum order: dy^2 + dx^2,
        0x4243CF-0x4243EC); if `!(e < 40.0f)` (`test ah,5; jp`, NaN too) flag = 0; else flag = 1 (0x424406) and then
        if `(n.byte1 & 0x30) < 0x20` (not m_bHighway, 0x4243FF-0x42440D) flag = 0.
  3. Start node (0x424416): `if ((rand() & 0xF) == 4 && !noRandomStart)`: `start = FindNodeClosestToCoors(pos, 0, 50.0f, 0, 0, 0,
     1, 1)` (gr parameter names: nodeType = 0 PATH_TYPE_VEH, maxDistance, unk2, unk3, unk4, bBoatsOnly = 1, unk6 = 1 - raw
     values from the pushes at 0x42442C-0x42443B) and `range1 = range1 * 1.5f` (stored f32 into the argument slot).
     Else `r = rand()`: `start = (r & 3) ? (restrict ? s_a : s_c) : (restrict ? s_b : s_d)` (0x424471-0x42449F).
  4. `cur = start` (`[esp+0x20]`, 0x4244AA). Return 0 if `cur.area == 0xFFFF` (0x4244A5) or `m_pPathNodes[cur.area] == null`
     (0x4244C0). `visited[30]` constructed (area = 0xFFFF), `visited[0] = start`, `count = 1` (0x4244DC-0x4244E3).
  5. Outer loop (0x4244F0): return 0 if `!(accDist < 230.0f)` (`test ah,5; jp`). Return 0 if `m_pPathNodes[cur.area] == null`
     (0x424517). `cn = node(cur)`; `numLinks = cn.byte0 & 0xF`; `SequenceElements = numLinks`;
     `SequenceRandomOffset = rand() % SequenceElements` (signed `idiv`, 0x424541); `bSequenceOtherWay = (rand() >> 4) & 1`
     (low byte, 0x42455E-0x42456A). `i = 0`, `j = SequenceRandomOffset + SequenceElements`.
  6. Inner loop over links (0x424578): if `i >= numLinks` return 0 (0x42457A; no link of this node was usable).
     `k = bSequenceOtherWay ? (i + SequenceRandomOffset) % SequenceElements : j % SequenceElements` (signed idiv 0x424593; the
     globals are re-read on every iteration after the first, 0x424613/0x42462B). `idx = cn.m_wBaseLinkId + k`;
     `next = m_pNodeLinks[cur.area][idx]` (dword). Skip the link (`i++, j--`, 0x424619) if `m_pPathNodes[next.area] == null`
     (0x4245B5) or `m_pPathNodes[m_pNaviLinks[cur.area][idx] >> 10] == null` (0x4245CD) or `next` is in `visited[0..count-1]`
     (area and node words compared, loop does not break early, 0x4245F0-0x42460D).
  7. Candidate (0x42463E), `nn = node(next)`: `C = pos2(cn)`, `N = pos2(nn)`;
     `dCur = sqrt((pos.y - C.y)^2 + (pos.x - C.x)^2)` (0x42464E-0x4246AE, x87 order y-term first, stored f32),
     `dNext = sqrt((pos.y - N.y)^2 + (pos.x - N.x)^2)` (0x4246B2-0x4246FC, same order).
     If `(nn.byte0 & 0x20 || cn.byte0 & 0x20)` (either switched off; flags of 0x424663 consumed at 0x424700) `&& restrict`:
     advance (step 10).
  8. Range test A (0x424717) with R = range1: `a = dNext - R`, `b = dCur - R`; if `!(b * a < 0.0f)` (`test ah,5; jp`) go to
     test B. Else `B = |b|`, `A = |a|` (fchs when `< 0.0f`, NaN unchanged); `w = 1.0f / (A + B)` (0x42480D);
     `outCoors->x = (N.x*B + C.x*A) * w`, `->y = (N.y*B + C.y*A) * w`, `->z = (Nz*B + Cz*A) * w` with `Cz/Nz = (i16)z * 0.125`
     (products stored/used as f32; with the in-game 24-bit precision control the stores do not change the values).
     If `IsSphereVisible(outCoors, 5.0f)` (0x42484E) go to test B; else `*outFrac = w * B` (0x424857-0x424866) and go to the slope check in step 9.
  9. Range test B (0x424878) with R = range2: same `a`, `b`, sign test (`jp 0x424A8E` -> advance), `B = |b|`, `A = |a|`;
     `P = cn.GetPosition()`, `Q = nn.GetPosition()` (0x4248D3/0x424902); `outCoors = (Q*B + P*A) * w`, `w = 1/(A + B)` (same
     formula, 0x4248D8-0x424984). If `IsSphereVisible(outCoors, 5.0f)` (0x42498E) advance (step 10); else `*outFrac = w * B`.
     Slope check (0x4249AC): `DX = C.x - N.x`, `DY = C.y - N.y`, `dz = |Cz - Nz|` (`fabs`), `L = sqrt(DX*DX + DY*DY)`; if
     `L * 0.5f < dz` (`fcompp; test ah,5; jnp`, 0x424A5D-0x424A64) advance (too steep). Otherwise accept:
     `if (rand() & 8)`: `*outNode1 = cur`, `*outNode2 = next` (0x424A81-0x424A87); else `*outNode1 = next`, `*outNode2 = cur`,
     `*outFrac = 1.0f - *outFrac` (0x424B41-0x424B58).
     Final checks (0x424B5A): `n2 = node(*outNode2)`, `n1 = node(*outNode1)`; if `n2.byte0 & 0x10` (m_onDeadEnd) and
     `ThisNodeWillLeadIntoADeadEnd(n2, n1)` and `restrict`: return 0 (0x424BAB).
     Direction: `inv = 1.0f / sqrt(dy*dy + dx*dx)` with `dx = outCoors->x - pos.x`, `dy = outCoors->y - pos.y`;
     `dot = (inv*dy)*dirY + (inv*dx)*dirX` (x87 0x424BAD-0x424BF1; the y term is the first addend). The `je` at 0x424C00
     tests the flags of `test al,al` (wantAhead) at 0x424BBD (x87 ops leave EFLAGS alone):
     - wantAhead != 0: accept only if `dot > dotLimit` (`test ah,0x41; je`), else return 0 (equal/NaN -> 0).
     - wantAhead == 0: accept only if `dot <= dotLimit` (`test ah,0x41; jp` -> 0 when greater or NaN).
     Accepted: a debug loop scans `m_pNodeLinks[n1area][n1.m_wBaseLinkId + e]` for `e < (n1.byte0 & 0xF)` looking for
     `*outNode2` (0x424C1D-0x424CA5); its result is unused and `sprintf(gString, "tell Obbe it happened again %d/%d %d/%d",
     outNode1->area, outNode1->node, outNode2->area, outNode2->node)` (u16 zero-extended, 0x424CA7-0x424CC3) runs whether or
     not the link was found. Return 1.
  10. Advance (0x424A92): `cur = next` (`[esp+0x20]`), `visited[count] = next`, `count++`; `accDist += sqrt(DX'^2 + DY'^2)` with
     `DX' = N.x - C.x`, `DY' = N.y - C.y` (0x424AA6-0x424B2A, stored f32). Loop to step 5 while `count < 30` (`jl` on the
     `cmp eax,0x1E` flags of 0x424AD2), else return 0.
- Edge cases / oddities:
  - A node with 0 links makes `rand() % 0` (0x424541) raise an integer divide fault (original bug; real data presumably never
    reaches it). Port: reproduce or assert, do not silently return.
  - The "tell Obbe" sprintf is unconditional on success (the found/not-found debug test was compiled out except the loop); it
    overwrites gString 0xB71670, which is a visible side effect.
  - The visited list holds at most 30 entries; the loop stops at 30 visited nodes or 230 accumulated metres.
  - m_bMoreCarsAndFewerPeds is only recomputed on refresh (every 5 s or after moving > 10 m).
  - Test A and B use different "ranges" (range1, possibly x1.5, then range2); B is only tried when A's crossing point is
    visible or there is no crossing.
- gta-reversed: `plugin::CallAndReturn` only (CarCtrl.cpp:455); parameter names misleading (see above).
- Open questions: none for behaviour. The meaning of FindNodeClosestToCoors' trailing args (1, 1) per gr names (bBoatsOnly,
  unk6) at 0x42442C-0x42442E is not verified here; the port passes the raw values.

## 0x424CE0 CCarCtrl::ChooseModel  (catalogue: specified/REA, port level 3)
- Convention / signature: `__cdecl int32 (int32* outType [esp+4])`, plain `ret`. Returns the model id in EAX or -1 (`or eax,-1`
  at 0x424D9C / 0x424F76). `outType` receives a category code (0 normal, 13 cop, 14+gang, 25 dealer) that only the caller
  interprets. `sub esp,8` frame: `[esp]` = car count (int, later re-stored as f32), `[esp+4]` = random value scratch.
- Callers: CCarCtrl::GenerateOneRandomCar 0x430050 (1 site).
- Callees: `_rand` 0x821B1E (4 sites), `_ftol` 0x821B40 (4 sites, truncation toward zero), CGangWars::GangWarFightingGoingOn
  0x443AC0, CLoadedCarGroup::PickRandomCar 0x611C50 (thiscall), CPopulation::PickGangCar 0x614490 (cdecl, int gang),
  CGeneral::GetRandomNumberInRange(int,int) 0x407180 (cdecl), tail-jump to CCarCtrl::ChoosePoliceCarModel 0x421980.
- Globals: CPopCycle::m_NumOther_Cars 0xC0BC30, m_NumCops_Cars 0xC0BC34, m_NumGangs_Cars 0xC0BC38, m_NumDealers_Cars 0xC0BC3C
  (all f32); CPopCycle::m_pCurrZoneInfo 0xC0BC68 (CZoneInfo*; bytes +0..+9 = GangStrength[10]); CCheat::m_aCheatsActive
  0x969130: +0x4A = 0x96917A (cheat 74 CHEAT_REDUCED_TRAFFIC), +0x2B = 0x96915B (cheat 43 CHEAT_GANGS_CONTROLS_THE_STREETS);
  CTheScripts::ForceRandomCarModel 0xA4448C (i32); CPopulation::m_bDontCreateRandomGangMembers 0xC0FCB2 (u8),
  m_bDontCreateRandomCops 0xC0FCB4 (u8); CPopulation::m_AppropriateLoadedCars 0xC0E9F8; CPopulation::m_CarGroups 0xC0ED38
  (int16[34][23], stride 0x2E): 0xC0F074 = group 18 (POPCYCLE_CARGROUP_BALLAS, first gang group), 0xC0F240 = group 28
  (POPCYCLE_CARGROUP_DEALERS) element 0; CStreaming::ms_aInfoForModel 0x8E4CC0 (stride 0x14, +0x10 load state, 1 = LOADED;
  accessed as `[model*20 + 0x8E4CD0]`).
- Constants: 0x858B14 = 3.0517578125e-05 (f32, 1/32768), 0x858628 = 100.0 (f32), 0x858C7C = 3.0518509447574615e-05
  (f32, 1/32767), 0x858F84 = 23.0 (f32), imm 0x7D0 = 2000 (empty car-group slot marker), imm 0x19, 0xD, 0xE.
- Behaviour:
  1. `total = ftol(((Other + Cops) + Gangs) + Dealers)` (x87 sum in that order, 0x424CE3-0x424CF5; ftol 0x424CFB); stored
     to `[esp]`. If `total <= 0` (signed, 0x424D06) return -1.
  2. If cheat 74 (0x96917A) active: `k = ftol(((rand() & 0xFFFF) * 1/32768) * 100.0)` (0x424D15-0x424D33); if `k != 0`
     return -1 (0x424D3A). (So with reduced traffic only about 1 in 100 attempts continues.)
  3. `r = (float)rand() * (1/32767)` (no mask, 0x424D3C-0x424D4F), stored f32 at `[esp+4]`; `cl = cheat43` (0x424D49);
     `total` is converted to f32 and stored over `[esp]` (fild/fstp 0x424D59/0x424D5D).
  4. Dealer test (0x424D61-0x424D78): `if (Dealers / totalF > r && !cheat43)` (fcomp + `test ah,0x41; jne`: equal or NaN
     go on to step 5): `*outType = 25` (0x424D7E); `m = m_CarGroups[28][0]` (u16 0xC0F240, 0x424D84);
     return `m` if `ms_aInfoForModel[m].loadState == 1` (0x424D8E, `je 0x424F7C` with EAX = m) else -1.
  5. Gang test (0x424DA3-0x424DC7): `if ((Gangs + Dealers) / totalF > r || cheat43)` go to step 8 (`je 0x424E52` when
     strictly greater; `jne` on cl).
  6. Cop test (0x424DCD-0x424DEC): `if ((Cops + Gangs) + Dealers) / totalF > r`: if `GangWarFightingGoingOn()` (0x424DEE)
     or `m_bDontCreateRandomCops` (0x424DFB) return -1; else `*outType = 13` (0x424E0E), the frame is popped, the
     CALLER'S argument slot `[esp+4]` (which held `outType`) is overwritten with 0 (0x424E18) and the function tail-jumps
     to `ChoosePoliceCarModel(0)` (0x424E20); its return value is returned.
  7. Normal (0x424E25): `*outType = 0`; if `ForceRandomCarModel != -1` return it (0x424E34, EAX = the forced id);
     else return `m_AppropriateLoadedCars.PickRandomCar(true, false)` (push 0; push 1; ecx = 0xC0E9F8, 0x424E3D-0x424E46).
  8. Gang (0x424E52): if `m_bDontCreateRandomGangMembers` return -1. `sum = GS[8]+GS[9]+GS[7]+GS[6]+...+GS[1]+GS[0]`
     (u8 GangStrength of m_pCurrZoneInfo, integer). `gang = 0`; if `sum <= 0` return -1 (0x424EAE).
     `v = ftol(((rand() & 0xFFFF) * 1/32768) * sum)` (`fimul` with the int, 0x424EB4-0x424ED0). The zone pointer is
     reloaded (0x424ED5). `while (GS[gang] < v) { v -= GS[gang]; gang++; }` (signed compares 0x424EDE `jge`, 0x424EEF `jl`).
     If cheat43: `gang = GetRandomNumberInRange(0, 9)` (0x424EFA-0x424F06, replaces the weighted pick).
     `*outType = gang + 14` (0x424F0C-0x424F10). If `PickGangCar(gang) < 0` (0x424F12-0x424F1C) return -1 (the
     returned model itself is discarded).
     Retry loop (tries = 0, 0x424F1E): `idx = ftol(((rand() & 0xFFFF) * 1/32768) * 23.0)` (0x424F28-0x424F46);
     `m = m_CarGroups[18 + gang][idx]` (word at `0xC0F074 + 2*(gang*23 + idx)`); if `m == 2000` retry WITHOUT counting
     (0x424F56); else if `ms_aInfoForModel[m].loadState == 1` return `m` (0x424F6E); else `++tries`, retry while
     `tries < 10` (0x424F74); then return -1.
- Edge cases / oddities:
  - The random test values differ: steps 2/8 use `1/32768` with `& 0xFFFF`, step 3 uses `1/32767` without mask, so r can
    reach 1.0 exactly (then all tests fail and a normal car is chosen).
  - Weighted gang pick: a gang with strength 0 is chosen if `v == 0` and all earlier strengths are 0 (`<` not `<=`).
  - The 2000-marker retry loop is unbounded; it only terminates because PickGangCar succeeded (group has non-2000 entries,
    idx 0..22 random). The car group's actual count (`m_nNumCarsInGroup`) is ignored; any of the 23 slots can be drawn.
  - Step 6 clobbers the caller's stack argument slot (allowed in cdecl; the port should simply return
    `ChoosePoliceCarModel(0)`); whether GenerateOneRandomCar re-reads that slot after the call was not checked.
- gta-reversed: `plugin::CallAndReturn` only (CarCtrl.cpp:223). Open questions: none.

## 0x424F80 CCarCtrl::PossiblyRemoveVehicle  (catalogue: specified/REA, port level 4)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4])`, plain `ret`. Frame `sub esp,0x2C; push esi` (+`push edi` from
  0x42523F on). The function reuses its own argument slot `[esp+0x34]` (veh, already copied to ESI) as the f32 local
  "removal range" R (0x4250DE / 0x4250F4 / 0x42510B); harmless for a port.
- Callers: CCarCtrl::RemoveDistantCars 0x42CD10, CPlayerPed::KeepAreaAroundPlayerClear 0x60C1E0.
- Callees: CVehicle::CanBeDeleted 0x6D1180 (thiscall, x2), CCranes::IsThisCarBeingTargettedByAnyCrane 0x6F4000 (cdecl),
  CVisibilityPlugins::GetClumpAlpha 0x732B20 (cdecl RpClump*), CWorld::FindPlayerCentreOfWorld 0x56E250 (cdecl int32 player
  -> CVector*), CEntity::GetIsOnScreen 0x534540 (thiscall, x4), CCamera::GetLookDirection 0x50AE90 (thiscall on TheCamera
  0xB6F028), 0x420800 = `float max(float a, float b)` helper (cdecl; `a > b ? a : b`, returns ST0; equal/NaN -> b),
  CGarages::IsPointWithinHideOutGarage 0x448900 (cdecl const CVector&, x3), CCarCtrl::IsThisVehicleInteresting 0x423EA0
  (cdecl, x2), VectorSub 0x40FE60 (cdecl `(CVector* out, const CVector* a, const CVector* b)` -> out = a - b, returns out),
  0x406DA0 = CVector::SquaredMagnitude (thiscall, `(x*x + y*y) + z*z` -> ST0), CTrafficLights::ShouldCarStopForLight 0x49D610
  (cdecl (veh, bool)), CTrafficLights::ShouldCarStopForBridge 0x49D420 (cdecl), CWorld::Remove 0x563280 (cdecl, x3),
  and 3 indirect `call [vtbl+0]` with `push 1` = scalar deleting destructor (`delete veh`).
- Globals: CWorld::PlayerInFocus 0xB7CD74 (u8), CTimer::m_snTimeInMilliseconds 0xB7CB84 (u32), TheCamera 0xB6F028:
  +0x59 = 0xB6F081 ActiveCam (u8), Cams[3] at +0x174 stride 0x238, of which +7/+8/+9 = 0xB6F1A3/0xB6F1A4/0xB6F1A5 (gr
  CCam m_bLookingBehind / m_bLookingLeft / m_bLookingRight); +0xF4 = 0xB6F11C (f32; gr Camera.h:180-181 puts
  m_fLODDistMultiplier / m_fGenerationDistMultiplier around there, offset not validated by gr, see open question);
  0xB6F9B4 = TheCamera.m_mCameraMatrix.forward.z (f32).
- Constants: 0x858F98 = 170.0 (f32), imm 0x42340000 = 45.0f, imm 0x428C0000 = 70.0f, imm 0x432A0000 = 170.0f,
  0x858CAC = -0.8999999761581421 (f32 -0.9), 0x858F94 = 0.0058823530562222 (f32, 1/170), 0x858B50 = 0.0 (f32),
  0x858F90 = 0.7400000095367432 (f32 0.74), 0x858F8C = 22.0 (f32), 0x858F88 = 42.25 (f32, = 6.5^2), imm 0x1388 = 5000,
  imm 0xEA60 = 60000.
- Fields (veh; offsets computed from gr Vehicle.h / AutoPilot.h order): +0x04 placement pos, +0x14 m_matrix (pos +0x30,
  up.z +0x28), +0x18 m_pRwClump, +0x1C entity flags (bit 4 = m_bIsStuck), +0x22 m_nModelIndex (i16), +0x36 type/status
  (status = byte >> 3), +0xB8 CPhysical m_nFakePhysics (u8), autopilot (CVehicle+0x390): +0x3B0 m_nTimeSwitchedToRealPhysics
  (u32), +0x3B9 m_nCarDrivingStyle (u8), +0x3BA m_nCarMission (u8), +0x3BB m_nTempAction (u8), +0x41C m_TargetEntity;
  vehicle flags u32 at +0x428: +0x428 bit0 bIsLawEnforcer, bit3 bIsLocked; +0x42A bit2 bFadeOut; +0x42B bit1
  bIsCarParkVehicle, bit3 bPartOfConvoy; +0x42C bit6 bNeverUseSmallerRemovalRange; +0x485 m_nNumGettingIn (u8),
  +0x4A4 m_nCreatedBy (u8), +0x4A6 m_nExtendedRemovalRange (i16), +0x4D0 m_nTimeTillWeNeedThisCar (u32),
  +0x4D8 m_nTimeWhenBlowedUp (u32), +0x594 m_nVehicleSubType (u32; 3 HELI, 4 PLANE).
- "pos" below = `m_matrix ? &m_matrix->pos : &placement.pos` (no allocation anywhere). "Kill" = `CWorld::Remove(veh);
  veh->vtbl[0](1)` (delete) then return (0x4251E5, 0x425220, 0x42541D).
- Behaviour:
  1. If `m_nNumGettingIn != 0` return (0x424F90). If `bPartOfConvoy && m_TargetEntity != null` return (0x424F96-0x424FA7).
  2. If `bIsLocked` (0x424FAD) or `!CanBeDeleted()` (0x424FC3) or `IsThisCarBeingTargettedByAnyCrane(veh)` (0x424FD4):
     go to step 7.
  3. If `bFadeOut && GetClumpAlpha(m_pRwClump) == 0` (0x424FDA-0x424FF1): Kill.
  4. `C = *FindPlayerCentreOfWorld(PlayerInFocus)` (copied to locals, 0x424FF7-0x42501C).
     `dist = sqrt((dy*dy) + (dx*dx))` with `dx = pos.x - C.x`, `dy = pos.y - C.y` (x87 0x42502A-0x42504D: sum order
     dy^2 + dx^2), stored f32 (0x425053).
     Small range `R = 45.0f` (0x4250DE) only if ALL hold (checked in this order): `!GetIsOnScreen()` (0x42505E),
     `!cam.m_bLookingLeft`, `!cam.m_bLookingRight`, `!cam.m_bLookingBehind` (cam = Cams[ActiveCam], 0x425064-0x42508D),
     `TheCamera.GetLookDirection() != 0` (0x42509B), `m_nCreatedBy != 3` (PARKED_VEHICLE, 0x42509D), model != 0x1A0 (416
     ambulance) and != 0x197 (407 firetruck) (0x4250AA/0x4250B0), `!bIsLawEnforcer` (0x4250B6), `!bIsCarParkVehicle`
     (0x4250BF), `now >= m_nTimeTillWeNeedThisCar` (unsigned, `jb` 0x4250D3), `!bNeverUseSmallerRemovalRange` (0x4250D5).
     Otherwise `R = TheCamera[+0xF4] * 170.0f` (0x4250E8-0x4250F4, stored f32).
     Then if `forward.z(0xB6F9B4) < -0.9f` (`test ah,5; jp` 0x425109: NaN keeps R) `R = 70.0f` (overrides both cases).
  5. `e = max((float)(i16)m_nExtendedRemovalRange, 170.0f)` (fild then call 0x420800; 0x425113-0x42512B);
     `thr = (e * R) * (1/170)` (x87 0x425130-0x425137, unrounded). If `thr < dist` (`fcomp [dist]; test ah,5; jp`
     0x42513D-0x425146; NaN goes to step 6):
     a. If `m_nCarMission == 59` (MISSION_PLANE_ATTACK_PLAYER_POLICE, 0x425148) go to step 6.
     b. If `IsPointWithinHideOutGarage(copy of pos)` (0x425179) go to step 6.
     c. If `IsThisVehicleInteresting(veh)` (0x425186): `m_nFakePhysics(+0xB8) = 10` (0x425192), return.
     d. If `GetIsOnScreen()` (0x4251A0): `bFadeOut = 1` (`or byte [esi+0x42A],4`), return.
     e. Kill (0x425220).
  6. (0x4251B5) `s = status`. If `s == 2` (STATUS_SIMPLE): `uz = m_matrix->up.z` (matrix pointer NOT null-checked,
     0x4251C0); `a = uz < 0.0f ? -uz : uz` (0x4251C6-0x4251D6, NaN not negated); if `a < 0.74f` (0x4251D8-0x4251E3)
     Kill (0x4251E5). Then (0x4251FB) if `s == 3 || s == 5` (PHYSICS / WRECKED) and `m_nVehicleSubType` is 4 or 3 and
     entity flag bit 4 (`m_bIsStuck`, `[+0x1C] >> 4 & 1`) is set: Kill (0x425220).
  7. (0x425236) Stuck/abandoned removal. Skipped (-> step 8) if `m_nVehicleSubType == 3 || == 4` (0x42523C/0x425246), or
     `m_nTempAction == 12` (TEMPACT_STUCKINTRAFFIC, 0x42524F). Status must be 2, or 3 with `m_nCarDrivingStyle` 0 or 4
     (0x42525C-0x42527A), else step 8. Then all of: `now - m_nTimeSwitchedToRealPhysics > 5000` (unsigned, `jbe`
     0x425295), `m_nTimeTillWeNeedThisCar == 0` (0x4252A3), `now != 0` (`test eax,eax; jbe` 0x4252AB),
     `!GetIsOnScreen()` (0x4252BA), `sqrt(d.x*d.x + d.y*d.y) > 22.0f` with `d = VectorSub(pos, C)` (sum order x^2 + y^2,
     0x4252DF-0x4252F1; `test ah,0x41; jne` 0x425302: equal/NaN fail), `!IsThisVehicleInteresting` (0x42530F),
     `!bIsLocked` (0x425318), `CanBeDeleted()` (0x425323), `!ShouldCarStopForLight(veh, true)` (0x425332),
     `!ShouldCarStopForBridge(veh)` (0x42533F), `!IsPointWithinHideOutGarage(pos)` (0x425373 `je` -> Kill at 0x42541D).
     If any fails: step 8.
  8. (0x425379) Wreck removal: if status == 5 (`(byte & 0xF8) == 0x28`), `m_nTimeWhenBlowedUp != 0` (0x42538E),
     `now > m_nTimeWhenBlowedUp + 60000` (unsigned `jbe` 0x4253A1), `now > m_nTimeTillWeNeedThisCar` (unsigned 0x4253AD),
     `!GetIsOnScreen()` (0x4253B8), `SquaredMagnitude(pos - C) > 42.25f` (3D, `test ah,0x41; jne` 0x4253EC) and
     `!IsPointWithinHideOutGarage(pos)` (0x42541B): Kill (0x42541D). Return.
- Edge cases / oddities:
  - Step 4 compares the 2D distance against the range but step 8 uses the 3D squared distance (6.5 m), step 7 2D 22 m.
  - Step 6 dereferences `m_matrix` without a null check (0x4251C0) for STATUS_SIMPLE vehicles.
  - Step 7 requires `m_nTimeTillWeNeedThisCar == 0` while steps 4/8 compare it with the time (different semantics kept).
  - The vehicle is deleted through vtable slot 0 with flag 1; the caller must not touch it afterwards.
  - `now + ...` additions wrap as u32 (0x425394).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:728); no C++ logic to compare.
- Open questions: exact identity of TheCamera+0xF4 (0x4250E8) - gr names m_fLODDistMultiplier and m_fGenerationDistMultiplier
  adjacently (Camera.h:180/181) but has no VALIDATE_OFFSET; the port only needs "f32 at TheCamera+0xF4". Meaning of the
  nonzero GetLookDirection result (0x42509B) is not needed for the port.

## 0x425440 CCarCtrl::SlowCarDownForPedsSectorList<CPtrListDoubleLink<CPhysical*>>  (catalogue: specified/REA, port level 9)
- Convention / signature: `__cdecl void (CPtrListDoubleLink* list [esp+4], CVehicle* veh [+8], float minX [+0xC], float minY [+0x10],
  float maxX [+0x14], float maxY [+0x18], float* pMaxSpeed [+0x1C], float cruiseSpeed [+0x20])`, plain `ret`. SEH frame
  (handler 0x837440, state slot [esp0-4]) because two stack `CEventPotentialGetRunOver` temporaries are built.
  Stack names below: S = esp after `push esi` (0x42545B) = esp0-0xA0; inside the loop 3 more regs are pushed (T = S-0xC).
- Callers: CCarCtrl::ScanForPedDanger 0x42CE40, CCarCtrl::FindMaximumSpeedForThisCarInTraffic 0x434400.
- Callees: CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (int -1, bool 0), x2), CWorld::FindPlayerPed 0x56E210 (cdecl (-1)),
  CCollision::DistAlongLine2D 0x412A80 (cdecl 6 floats -> ST0 = (px-lx)*dx + (py-ly)*dy), CPlaceable::GetTopDirection 0x41CCB0
  (thiscall, ret-buffer -> EAX = CVector*), VectorSub 0x40FE60 (cdecl (out, a, b) out = a - b), 0x406DA0 (thiscall CVector*
  -> ST0 = x*x + y*y + z*z, i.e. SquaredMagnitude; sum order (x*x + y*y) + z*z), 0x406D10 (cdecl float -> ST0 = fabs, i.e. `Abs`),
  0x4AE1A0 CEventPotentialGetRunOver ctor (thiscall (CVehicle*), ret 4; vtable 0x85AE88, GetEventType slot 0x4AE210 returns 0xC =
  EVENT_POTENTIAL_GET_RUN_OVER), 0x4AE270 its dtor, CEventGroup::Add 0x4AB420 (thiscall on ped->m_pIntelligence+0x68
  (m_eventGroup), args (CEvent*, bool 0)), CPed::IsPlayer 0x5DF8F0, 0x421050 "CMentalState::IncrementAnger" (thiscall, arg 2).
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8, `CWorld::ms_nCurrentScanCode` 0xB7CD78 (u16), `CWorld::PlayerInFocus` 0xB7CD74 (u8),
  `CWorld::Players[i].m_pRemoteVehicle` 0xB7CE70 + 0x190*i, `CTimer::m_snTimeInMilliseconds` 0xB7CB84.
  Constants (f32): 0x858A48 = 200.0, 0x858B40 = 50.0, 0x858B50 = 0.0, 0x858B44 = 6.0, 0x858B3C = 3.0, 0x858FA8 = 1.6,
  0x858B8C = 0.5, 0x858CE4 = 13.0, 0x858624 = 1.0, 0x858FA4 = 0.07692308 (1/13), 0x858B90 = 4.0, 0x858FA0 = 2.5,
  0x858D04 = 49.0, 0x86C8CC = 0.05, 0x858F9C = 0.35.
- Fields: entity +0x04 placement pos, +0x14 m_matrix (right +0x00, forward +0x10, pos +0x30), +0x1C bit0 bUsesCollision, +0x22 model
  index, +0x2C m_nScanCode (u16), +0x36 type (bits 0-2) / status (bits 3-7); CPhysical +0x44 m_vecMoveSpeed; vehicle +0x390 CAutoPilot:
  +0x3B9 m_nCarDrivingStyle, +0x3BB m_nTempAction, +0x3BC m_nTempActionTime, +0x3DB m_nCarCtrlFlags (bit 1 bHonkAtPed);
  +0x460 m_pDriver, +0x514 m_HornCounter, +0x590 m_nVehicleType; ped +0x47C m_pIntelligence. ColModel bbox +0x0C max.x, +0x10 max.y.
- Behaviour:
  1. Prologue (0x42545C-0x4254AF): `halfW = colModel(veh).max.x` (local S+0x1C, 0x42547E); `fwdSpeed = (fwd.z*mv.z + fwd.y*mv.y) +
     fwd.x*mv.x` (matrix read without null check, 0x42547B-0x425495, stored f32 S+0x14); `lookAhead = fwdSpeed*200.0` (S+0x38, signed);
     `lookAhead2 = |fwdSpeed|*50.0` (S+0x3C). `halfL = colModel.max.y` (S+0x20, 0x425513).
  2. Flag `warnPeds` (S+7, 0x4254BB-0x4254F2) = 1 if `veh == FindPlayerVehicle(-1,false)`, or vehicle type is 6 (train), 4 (plane)
     or 3 (heli), or (driving style != 0 and != 6); otherwise `warnPeds = (status == 3 /*(+0x36 & 0xF8) == 0x18*/)`.
  3. If `list->head == null` return (0x425517). Iterate nodes (`node->item` = ped, next read first, 0x425530-0x42553F; loop tail 0x425B04).
     Per ped (all "skip" = go to next node):
     a. skip if `ped->m_nScanCode == ms_nCurrentScanCode` (0x425538) or `!(ped+0x1C & 1)` (0x425549); else set the scan code (0x425553).
        (So a non-colliding ped never gets its scan code set.)
     b. P = ped GetPosition (copied as f32 x/y/z). Skip unless `P.x > minX` (0x425576, NaN skips), `P.x < maxX` (0x425594),
        `P.y > minY` (0x4255AA), `P.y < maxY` (0x4255C0).
     c. `|P.z - V.z| < 6.0` (V = veh GetPosition; abs via sign test 0x4255E6, compare 0x425616; >= or NaN skips).
     d. `along = DistAlongLine2D(V.x, V.y, fwd.x, fwd.y, P.x, P.y)` (0x425649; matrix fwd read unconditionally - null deref if no matrix);
        `top = veh->GetTopDirection()`; `h = P.z - (along*top.z + V.z)` (0x425671-0x42567B), skip unless `|h| < 3.0` (0x42568E).
     e. `rz = P.z - V.z`, `ry = P.y - V.y`, `rx = P.x - V.x` (stored f32, 0x4256AC-0x4256C8);
        `fwdDist = (ry*fwd.y + rz*fwd.z) + rx*fwd.x` (stored S+0xC, 0x4256CC-0x4256ED).
     f. Brake/horn block, only if driving style (+0x3B9) is 0, 4, 1 or 6 (0x4256E3-0x4256FD); otherwise go to step g:
        - if `ped == FindPlayerPed(-1)` and `Players[PlayerInFocus].m_pRemoteVehicle == veh` -> go to g (0x425703-0x425724).
        - require `fwdDist > halfL` (0x42572E); `dist = fwdDist - halfL` (stored S+0x18); require `dist < lookAhead` (0x425749).
        - `side = |(ry*right.y + rz*right.z) + rx*right.x|` (x87, 0x425758-0x42577C). If vehicle type == 9 (bike):
          `halfW = halfW * 1.6` STORED BACK into the local (0x425780-0x42578A).
        - require `side <= halfW + 0.5` (inclusive; `fcompp`+`test ah,0x41; jp`, 0x42578E-0x4257A1; NaN skips).
        - require `dist < 13.0` (0x4257AB). `t = dist - 1.0` (stored f32 S+8, 0x4257C6).
        - `v = max(1.0, (0.07692308 * (t < 0 ? 0 : t)) * cruiseSpeed)` (0x4257CA-0x42583A; `0 > t` picks 0.0, else t; then if `1.0 > v`
          the result is 1.0, else v, recomputed from memory). `*pMaxSpeed = (v < *pMaxSpeed || equal) ? v : *pMaxSpeed` (0x425841-0x4258C4;
          the value is recomputed a third time for the store; i.e. `*pMaxSpeed = min(*pMaxSpeed, v)`, NaN v is stored).
        - `m_nCarCtrlFlags |= 2` (bHonkAtPed, 0x4258D0-0x4258D8).
        - if `dist < 4.0`: `m_nTempAction = 1` (WAIT), `m_nTempActionTime = now + 4000` (0x4258E5-0x4258F6).
        - if `dist < 2.5`: `m_nTempAction = 0x18` (BRAKE), `m_nTempActionTime = now + 4000` (0x42590D-0x425920).
     g. Ped warnings (0x425926): only if `(ped+0x36 & 7) == 3` (ENTITY_TYPE_PED).
        - Horn: if `veh == FindPlayerVehicle(-1,false)` and `veh->m_HornCounter != 0` and `SquaredMagnitude(P - V) < 49.0` (0x42598A):
          build `CEventPotentialGetRunOver(veh)` on the stack, `ped->m_pIntelligence->m_eventGroup.Add(&ev, false)`, destroy it.
        - If `warnPeds` (0x4259D1) and `fwdSpeed != 0` (0x4259E9: `test ah,0x44; jnp` - only exact equality skips, NaN continues)
          and `sign(fwdDist) == sign(fwdSpeed)` (each sign = `< 0 ? -1 : 1`, NaN -> 1, 0x4259F2-0x425A24):
          `a = |fwdDist|` (stored back to S+0xC); require `a > halfL` (0x425A34), `Abs(fwdSpeed) > 0.05` (0x425A4D),
          `a - halfL < lookAhead2` (0x425A69), `|(ry*right.y + rz*right.z) + rx*right.x| <= halfW + 0.35` (inclusive, 0x425A78-0x425AA8;
          halfW includes any 1.6 multiplications done so far). Then build `CEventPotentialGetRunOver(veh)`, add it to the ped's event
          group (bool 0); if `veh->m_pDriver && m_pDriver->IsPlayer()`: call 0x421050 with ECX = `ped->m_pIntelligence` (not
          `&m_mentalState`) and arg 2; destroy the event.
  4. Restore SEH, return.
- Edge cases / original bugs:
  - `halfW *= 1.6` for bikes is written to the loop-invariant local (0x42578A), so it compounds once per ped that reaches that point
    in the same call (bike widths grow 1.6^n). Keep.
  - `lookAhead` is signed (fwdSpeed*200): a reversing vehicle never brakes for peds in block f (dist < negative is false).
  - The max(1.0, ...) clamp means the speed limit written to *pMaxSpeed is never below 1.0.
  - 0x421050 is called with ECX = CPedIntelligence*; its body (0x156FC20) reads [ecx+0x180]/[ecx+0x181]/[ecx+0x178]/[ecx+0x17C],
    which are the CMentalState fields at their CPedIntelligence offsets (m_mentalState sits before field_188). So 0x421050 is in
    effect a CPedIntelligence method; gr's `CMentalState::IncrementAnger(this = CMentalState*)` binding would pass the wrong this.
- gta-reversed: `plugin::Call` stub only (CarCtrl.cpp:899-903); signature (list, vehicle, 4 floats, float*, float) matches.
- Open questions: none affecting behaviour.

## 0x425B30 CCarCtrl::TestCollisionBetween2MovingRects  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl float (CVehicle* car1 [esp+4], CVehicle* car2 [+8], float velX [+0xC], float velY [+0x10],
  CVector* fwd2 [+0x14], CVector* fwd1 [+0x18])`, plain `ret`; returns ST0 (the caller 0x42D0E0 does `fstp` right after the
  call at 0x42D2D7). Ghidra shows `void` (wrong); gr declares `float` (right). Only `.x/.y` of the vectors are read.
  Arg names from the bytes: `fwd2` (+0x14) is the axis frame of car2's box, `fwd1` (+0x18) orients car1's corners.
- Callers: 1, CCarCtrl::SlowCarDownForOtherCar 0x42D0E0 (site 0x42D2D7; passes negated floats as velX/velY, 0x42D2BD/0x42D2D0).
  Callees: none.
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8 (read). Constants: 0x858624 = 1.0 (f32), 0x858B50 = 0.0 (f32).
- Fields: entity +0x14 m_matrix (pos at +0x30/+0x34), +0x04 placement pos (GetPosition = matrix ? matrix+0x30 : this+4,
  no allocation), +0x22 m_nModelIndex (i16). ModelInfo +0x14 m_pColModel; colmodel bounding box +0x04 min.y, +0x0C max.x,
  +0x10 max.y (CBox m_vecMin/m_vecMax).
- Behaviour (locals are f32 stack slots unless noted; best = running result kept in x87 ST0 from 0x425BA7):
  1. `dx = P1.x - P2.x` (0x425B5A-0x425B61), `dy = P1.y - P2.y` (0x425B74-0x425B8F); P1 = car1 position, P2 = car2 position.
  2. car2 box: `hx2 = max.x` (0x425B93), `maxY2 = max.y` (0x425B8C), `nminY2 = -min.y` (0x425B96-0x425BA3).
     car1 box: `hx1 = max.x` (0x425BBB), `hy1 = max.y` (0x425BC2).
  3. `best = 1.0` (0x425BA7, ST0). `A = velX*fwd2.y - velY*fwd2.x` (0x425BB4-0x425BE3), `B = velX*fwd2.x + velY*fwd2.y`
     (0x425BE7-0x425BF6). Both stored f32.
  4. Loop corner i = 0, 1 (edx counts up, edi = 2 counts down, 0x425BD1/0x425BD6/0x425F5A-0x425F5C):
     - i = 0 (0x425C36): `cx = (hy1*fwd1.x + hx1*fwd1.y) + dx`; `cy = (hy1*fwd1.y + dy) - hx1*fwd1.x`.
     - i = 1 (0x425C0A): `cx = (hy1*fwd1.x + dx) - hx1*fwd1.y`; `cy = (hx1*fwd1.x + hy1*fwd1.y) + dy`.
     (each stored f32 at 0x425C49/0x425C1D and 0x425C60.)
     X slab (car2 half width hx2, symmetric): `entryX = 0` (0x425C79), exitX = 1.0 in x87 (0x425C64);
     `lat = cx*fwd2.y - cy*fwd2.x` (0x425C6E-0x425C89, stored f32).
     a. `lat > hx2` (0x425C91, fails on <=/NaN): if `!(A < 0)` (0x425CA0-0x425CAB, A >= 0 or NaN) -> entryX = 1.0, exitX = 1.0
        (0x425CE6). Else `inv = 1.0/A` (x87, not stored), `e = -((lat - hx2)*inv)` (0x425CAD-0x425CC1); if `!(e < 1.0)`
        (0x425CC3, incl. NaN) -> entryX = 1.0, exitX = 1.0 (0x425D6B). Else entryX = e (0x425CD4),
        `x = e - (hx2 + hx2)*inv` (0x425CD8-0x425CE2) and exitX = x if `x < 1.0` (0x425D50-0x425D65), else stays 1.0.
     b. else if `lat < -hx2` (0x425CFB-0x425D16): if `!(A > 0)` (0x425D18, A <= 0 or NaN) -> entryX = exitX = 1.0. Else
        `inv = 1.0/A`, `e = -((lat + hx2)*inv)` (0x425D1D-0x425D31); `!(e < 1.0)` -> both 1.0; else entryX = e,
        `x = (hx2 + hx2)*inv + e` (0x425D4E), exitX = x if `x < 1.0`.
     c. else (inside, or lat NaN) entryX stays 0: if `A > 0` (0x425D79): `exitX = (hx2 - lat)/A` (direct fdiv, 0x425D80-0x425D88);
        else if `A < 0` (0x425D8E-0x425D9D): `exitX = -((lat + hx2)/A)` (0x425DA1-0x425DAD); A == 0 / NaN: exitX = 1.0.
     Y slab (car2 box from min.y to max.y): `entryY = 0` (x87 0.0, 0x425DAF), `exitY = 1.0` (0x425DC4);
     `lon = cy*fwd2.y + cx*fwd2.x` (0x425DB9-0x425DD4, stored f32).
     d. `lon > maxY2` (0x425DDC): if `!(B < 0)` (0x425DE9-0x425DFE) -> entryY = 1.0, exitY = 1.0 (0x425E41). Else
        `inv = 1.0/B` STORED f32 (0x425E04), `e = -((lon - maxY2)*inv)` (0x425E08-0x425E14, stored f32 0x425E16);
        `!(e < 1.0)` (0x425E1A) -> entryY = 1.0 (0x425EC6), exitY 1.0. Else entryY = e (reloaded f32),
        `x = e - (nminY2 + maxY2)*inv` (0x425E2B-0x425E3B), exitY = x if `x < 1.0` (0x425EB3-0x425EC0).
     e. else if `lon < -nminY2` (= min.y; 0x425E4E-0x425E5D): if `!(B > 0)` (0x425E61-0x425E76) -> entryY = exitY = 1.0.
        Else `inv = 1.0/B` stored f32 (0x425E7C), `e = -((lon + nminY2)*inv)` (0x425E80-0x425E8C); `!(e < 1.0)` -> entryY 1.0;
        else entryY = e, `x = (nminY2 + maxY2)*inv + e` (0x425E9F-0x425EAF), exitY = x if `x < 1.0`.
     f. else entryY = 0: if `B > 0` (0x425ECE): `exitY = (maxY2 - lon)/B` (0x425EDF); else if `B < 0`: `exitY = -((lon + nminY2)/B)`
        (0x425F02-0x425F10); B == 0 / NaN: 1.0.
     Combine (0x425F18-0x425F56): `entry = entryX > entryY ? entryX : entryY` (fails/NaN keep entryY); if
     `entry < exitX && entry < exitY && !(best < entry)` then `best = entry` (entry reloaded from its f32 store 0x425F2B).
  5. Return best (ST0) (0x425F68).
- Edge cases: the X test is symmetric around 0 using only max.x of car2 (min.x ignored); the Y test uses min.y and max.y.
  Only car1's two FRONT corners (+max.y) are tested; car1's min.x/min.y are not read. X-axis reciprocal stays in x87,
  Y-axis reciprocal is rounded to f32 (0x425E04/0x425E7C) - keep this. Result is in [.., 1.0]; 1.0 means no hit.
- gta-reversed: `plugin::CallAndReturn` stub only (CarCtrl.cpp:1068); no logic to compare.
- Open questions: none.

## 0x425F70 CCarCtrl::TestCollisionBetween2MovingRects_OnlyFrontBumper  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl float (CVehicle* car1 [esp+4], CVehicle* car2 [+8], float moveX [+0xC], float moveY [+0x10],
  CVector* fwd2 [+0x14], CVector* fwd1 [+0x18])`, plain `ret`; returns ST0 (0x426208; caller does `fstp` at 0x42D2AE).
  The function reuses its own argument slots car1/car2 ([esp0+4]/[esp0+8]) as scratch (0x425FCB, 0x425FE0) - harmless.
- Callers: 1, CCarCtrl::SlowCarDownForOtherCar 0x42D0E0 (site 0x42D2A9). Callees: none.
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8. Constants: 0x858B50 = 0.0 (f32); immediate 1.0f (0x426014).
- Fields: as 0x425B30 (entity +0x14 matrix, +0x04 placement, +0x22 model index; colmodel +0x04 min.y, +0x0C max.x, +0x10 max.y).
- Behaviour (esp-relative offsets below are after `push esi`, i.e. esp0-0x44):
  1. P1 = car1 position (x 0x425F88, y 0x425FA1), P2 = car2 position (x 0x425FF4, y loaded x87 0x426002). car2 box
     `hx2 = max.x`, `hy2 = max.y`; car1 box `hx1 = max.x`, `hy1 = max.y`, `nminY1 = -min.y` (0x425FD2-0x425FDC). best = 1.0 (0x426014).
  2. car2 front-bumper segment (0x426009-0x42606C), with a = hy2*fwd2.x, b = hx2*fwd2.y, c = hy2*fwd2.y, d = hx2*fwd2.x
     (each product stored f32 where noted: c 0x42603C, d 0x426046):
     `C0x = (a + b) + P2.x` (0x426029-0x426031), `C0y = (c + P2.y) - d` (0x42604A-0x426054),
     `C1x = (a + P2.x) - b` (0x426058-0x42605E), `C1y = P2.y + (d + c)` (0x426064-0x42606C, kept in x87).
     Then x87 holds fwd2.x (0x42606E) and two placeholder values (fwd2.y and an UNINITIALISED stack dword [esp+0x2C],
     0x426073/0x42607B) that the first switch case pops unused (dead; no behavioural effect).
  3. Loop i = 0..3 (edx up, esi = 4 down, 0x426020/0x426024/0x4261F7-0x4261F9), jump table 0x426210 =
     {0x426090, 0x4260BF, 0x4260EB, 0x426117}; corner (X, Y) of car1 (fwd1 = f):
     - 0: `X = (hx1*f.y + hy1*f.x) + P1.x`, `Y = (hy1*f.y + P1.y) - hx1*f.x` (front, 0x426094-0x4260B8)
     - 1: `X = (hy1*f.x + P1.x) - hx1*f.y`, `Y = (hy1*f.y + hx1*f.x) + P1.y` (front, 0x4260C3-0x4260E5)
     - 2: `X = (P1.x - nminY1*f.x) + hx1*f.y`, `Y = (P1.y - nminY1*f.y) - hx1*f.x` (rear, 0x4260EF-0x426113)
     - 3: `X = (P1.x - nminY1*f.x) - hx1*f.y`, `Y = (P1.y - nminY1*f.y) + hx1*f.x` (rear, 0x42611B-0x42613F)
     (X, Y stay in x87 at full 24-bit-significand precision, not stored.)
  4. Per corner (0x426141-0x4261F5): `X' = X + moveX`, `Y' = Y + moveY`;
     `d0 = (Y - C0y)*fwd2.y + (X - C0x)*fwd2.x` (stored f32 0x426161); `d1 = (Y' - C0y)*fwd2.y + (X' - C0x)*fwd2.x` (stored f32 0x426177).
     Skip unless `d0 > 0` (0x42617F, fails on <=/NaN) and `d1 < 0` (0x426190).
     `cr1 = (C1x - X)*moveY - (C1y - Y)*moveX` (0x42619D-0x4261AF); `cr0 = (C0x - X)*moveY - (C0y - Y)*moveX` (0x4261B1-0x4261C5);
     skip unless `cr0*cr1 < 0` (0x4261C7-0x4261D4, x87 product).
     `t = d0 / (d0 - d1)` (0x4261D6-0x4261DE, from the f32 stores); if `!(best < t)` (0x4261E2-0x4261ED) then `best = t` (f32 store).
  5. Pop the 4 x87 leftovers (0x4261FF-0x426206), return `best` reloaded (0x426208).
- Edge cases: only car2's front edge (y = max.y) is tested as a line between its two corners; car1 contributes all four corners.
  An exactly-equal t replaces best (no effect). d0 - d1 > 0 always when the checks pass, so no division by zero.
- gta-reversed: `plugin::CallAndReturn` stub only (CarCtrl.cpp:1073).
- Open questions: none (the uninitialised x87 load at 0x426073 is dead).

## 0x426350 CCarCtrl::WeaveForOtherCar  (catalogue: specified/REA, port level 2)
- Convention / signature: `__cdecl void (CEntity* other [esp+4], CVehicle* veh [+8], float* pAngleLeft [+0xC], float* pAngleRight [+0x10])`,
  plain `ret`. `other` is treated as a CVehicle in the body (autopilot/model/matrix reads) but is only typed CEntity by the caller.
  Stack name below: C = esp after `push ebx` (0x42641D) = esp0-0x94; all temporaries are C-relative.
- Callers: CCarCtrl::WeaveThroughCarsSectorList 0x42D680 (1 site).
- Callees: CWorld::FindPlayerVehicle 0x56E0D0 (cdecl (-1, false)); 0x59C890 `CMatrix::operator*(const CMatrix&, const CVector&)`
  (cdecl (CVector* out, CMatrix* m, CVector* v): full transform incl. translation; x4); Multiply3x3 0x59C790 (cdecl (out, m, v):
  rotation only; x4); 0x421A50 (unnamed, cdecl, 13 args, returns AL; see step 6).
- Globals: `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8. Constants (f32): 0x858B50 = 0.0, 0x858CC4 = 0.2, 0x858FAC = 0.10471976 (6 deg),
  0x858CBC = 6.2831855 (2*pi).
- Fields: vehicle +0x3BA m_autoPilot.m_nCarMission (u8), +0x41C m_autoPilot.m_TargetEntity, +0x42B bit 3 (vehicle flags dword 0x428
  bit 27 = bPartOfConvoy), +0x14 m_matrix (right +0x00, forward +0x10), +0x22 model index, +0x36 type bits 0-2, +0x44/+0x48
  m_vecMoveSpeed.x/.y; ped +0x46C bit 8 bInVehicle, +0x58C m_pVehicle. ColModel bbox +0x00 min.x, +0x04 min.y, +0x0C max.x, +0x10 max.y.
- Behaviour:
  1. Early-outs (return without touching the angles), in this order:
     a. `veh.mission == 3` (RAMPLAYER_CLOSE) and `other == FindPlayerVehicle(-1,false)` (0x426367-0x42637E).
     b. `veh.mission == 0x10` (RAMCAR_CLOSE) and `other == veh.target` (0x42638A-0x426394).
     c. `veh.mission == 0x35` (FOLLOWCAR_CLOSE): `other == veh.target`, or (`other` type == 2 and `other+0x42B & 8`) (0x42639A-0x4263BC).
     d. `veh.mission == 0x37` (KILLPED_CLOSE), `other` type == 3 (ped), `other+0x46C & 0x100` and `other+0x58C == veh.target` (0x4263C2-0x4263E8).
     e. `other+0x3BA` (read for any entity type) in {0x1D, 0x1E, 0x20, 0x1F, 0x1B, 0x1C} (ESCORT_*/PROTECTION_*) and `other+0x41C == veh`
        (0x4263EE-0x426412).
  2. `dx = O.x - V.x` (stored C+0x14), `dy = O.y - V.y` (C+0x3C) (GetPosition of each, 0x426418-0x426455). With vm = veh matrix (read
     without null check): `fwdDot = dy*vm.fwd.y + dx*vm.fwd.x` (0x426459-0x426463). If `fwdDot < 0` return (0x426465-0x426470; NaN continues).
     `rightDot = dy*vm.right.y + dx*vm.right.x` (C+0x10, 0x426476-0x426489).
  3. Own vehicle shape (vb = veh col box), all vectors with z = 0:
     - `P0 = vm * (vb.min.x - 0.2, fwdDot > 0 ? vb.max.y + 0.2 : vb.min.y - 0.2, 0)` (0x42648D-0x4264DB; `> 0` strict, so only
       fwdDot == 0 or NaN takes min.y) -> C+0x58.
     - `Wv = Multiply3x3(vm, (((vb.max.x + 0.2) - vb.min.x) - 0.2, 0, 0))` (0x426509-0x426538) -> C+0x4C.
     - `P1 = vm * (rightDot > 0 ? vb.max.x + 0.2 : vb.min.x - 0.2, vb.min.y - 0.2, 0)` (0x42653F-0x4265AB; rightDot <= 0 or NaN -> min.x) -> C+0x70.
     - `Lv = Multiply3x3(vm, (0, ((vb.max.y + 0.2) - vb.min.y) - 0.2, 0))` (0x4265DF-0x42660F) -> C+0x18.
  4. Other vehicle (om = other matrix, ob = other col box): `oRight = dy*om.right.y + dx*om.right.x` (C+0x10, 0x426616-0x426641),
     `oFwd = dy*om.fwd.y + dx*om.fwd.x` (x87, 0x42664B-0x426659).
     - `Q0 = om * (ob.min.x - 0.2, oFwd < 0 ? ob.max.y + 0.2 : ob.min.y - 0.2, 0)` (0x42665B-0x4266A1; `test ah,5; jp` -> >= 0 or NaN uses min.y) -> C+0x64.
     - `Wc = Multiply3x3(om, (((ob.max.x + 0.2) - ob.min.x) - 0.2, 0, 0))` (0x4266CF-0x4266FE) -> C+0x30.
     - `Q1 = om * (oRight < 0 ? ob.max.x + 0.2 : ob.min.x - 0.2, ob.min.y - 0.2, 0)` (0x426705-0x426771; >= 0 or NaN -> min.x) -> C+0x7C.
     - `Lc = Multiply3x3(om, (0, ((ob.max.y + 0.2) - ob.min.y) - 0.2, 0))` (0x4267A8-0x4267DB) -> C+0x24.
  5. `narrower = (vb.max.x < ob.max.x)` (byte at C+0x10, 0x426816-0x426825). `speed = sqrt(mv.x*mv.x + mv.y*mv.y)` of veh
     (x*x first, 0x42682A-0x426849, stored f32).
  6. Left sweep (0x426851-0x4268C9): up to 8 times: `if (!sub_421A50(*pAngleLeft, &P0, &Wv, &P1, &Lv, &Q0, &Wc, &Q1, &Lc, other.mv.x,
     other.mv.y, speed, narrower)) break;` then `a = *pAngleLeft - 0.10471976`, stored; if `a < 0` store `a + 6.2831855` (0x4268A2-0x4268C0).
     The bool arg is pushed as the whole dword at C+0x10 (upper 3 bytes are leftovers of oRight); the callee reads only the byte.
  7. Right sweep (0x4268D4-0x42694C): same call with `*pAngleRight`; on true `a = *pAngleRight + 0.10471976`, stored; if `a > 6.2831855`
     store `a - 6.2831855` (`test ah,0x41; jne` -> equal/NaN not wrapped). Up to 8 iterations, stops at the first false.
- Edge cases / original bugs:
  - The "extent" vectors are `((max + 0.2) - min) - 0.2` = plain box size: the 0.2 margin cancels (0x42650D/0x42651E, 0x4265E3/0x4265F5,
    0x4266D3/0x4266E4, 0x4267AC/0x4267C1), while the corner points do include the 0.2 margin. Reproduce exactly (op order matters for rounding).
  - Asymmetric side choice: own corner P1 takes max.x when `rightDot > 0`, the other's Q1 takes max.x when `oRight < 0`.
  - Matrices of both entities are dereferenced without null checks (0x426459, 0x426624).
  - sub_421A50 (unnamed, 0x421A50-0x421FDA; caller-derived proposed name `CCarCtrl::WouldCollideAtThisAngle`): computes
    `(mvX - cos(angle)*speed) * k[0x858628]` and `(mvY - sin(angle)*speed) * k` (0x421A53-0x421A8B) and, when the bool is set,
    negates and swaps the two rectangle sets (0x421A91-0x421AF1). Not part of this spec.
- gta-reversed: `plugin::Call` stub only (CarCtrl.cpp:1108-1110).
- Open questions: none affecting this function (sub_421A50 needs its own spec).

## 0x426BC0 CCarCtrl::WeaveForObject  (catalogue: specified/REA, port level 2)
- Convention / signature: `__cdecl void (CEntity* obj [esp+4], CVehicle* veh [+8], float* angleA [+0xC], float* angleB [+0x10])`,
  plain `ret` (0x426EDB / 0x426EE2). Frame: `sub esp,0x1C; push esi` (+ a transient `push edi` 0x426CB1..0x426D2E).
  The caller (WeaveThroughObjectsSectorList 0x42DA92-0x42DA9A) passes its own `float*` args 7/8 through unchanged; angleA/angleB
  are the two edges of the blocked heading interval (gr names them `arg3`/`arg4`). The obj arg slot [esp+0x24] is reused as a
  scratch float (dy, then the half-arc).
- Callers: 1 (CCarCtrl::WeaveThroughObjectsSectorList 0x42D950, site 0x42DA9A). Callees: CBaseModelInfo::SwaysInWind 0x4212C0
  (thiscall, returns EAX 0/1: `(flags+0x12 & 0x7800) == 0x800 || == 0x1000`), CPlaceable::AllocateMatrix 0x54F560 +
  CSimpleTransform::UpdateMatrix 0x54F1B0 ("ensure matrix", 4 sites, all on `obj`), CGeneral::GetATanOfXY 0x53CC70 (cdecl (x,y) -> ST0).
- Globals read: MI_TRAFFICLIGHTS 0x8CD4F4, MI_SINGLESTREETLIGHTS1 0x8CD518, MI_SINGLESTREETLIGHTS2 0x8CD51C,
  MI_SINGLESTREETLIGHTS3 0x8CD520, MI_DOUBLESTREETLIGHTS 0x8CD524 (all u16, compared zero-extended against the sign-extended
  model index), CModelInfo::ms_modelInfoPtrs 0xA9B0C8.
  Constants: 0x858FB0 = 2.4f, 0x858C24 = 0.3f, 0x858B8C = 0.5f, 0x858B50 = 0.0f, 0x858CC0 = -3.1415927f, 0x858CB8 = 3.1415927f,
  0x858CBC = 6.2831855f; immediates 2.957f (0x403D3F7D), 0.147f (0x3E16872B), 0.744f (0x3F3E76C9), 0.043f (0x3D3020C5),
  1.143f (0x3F924DD3), 0.145f (0x3E147AE1), -0.048f (0xBD449BA6).
- Fields: entity +0x04 placement pos, +0x14 m_matrix (right +0x00, forward +0x10, pos +0x30), +0x22 m_nModelIndex (i16);
  modelinfo +0x14 m_pColModel; colmodel +0x0C = m_boundBox.m_vecMax.x.
- Behaviour:
  1. Pick a local offset (offX [esp+8], offY [esp+0xC]) by obj model (0x426BCF-0x426C6C), first match wins:
     MI_TRAFFICLIGHTS -> (2.957, 0.147); MI_SINGLESTREETLIGHTS1 -> (0.744, 0); MI_SINGLESTREETLIGHTS2 -> (0.043, 0);
     MI_SINGLESTREETLIGHTS3 -> (1.143, 0.145); MI_DOUBLESTREETLIGHTS -> (0, -0.048); otherwise if
     `SwaysInWind(ms_modelInfoPtrs[model])` (0x426C57) -> (0, 0); otherwise return (0x426C5E -> 0x426EDE) with nothing written.
  2. `P = obj->GetPosition()` copied to locals BEFORE ensuring the matrix (0x426C74-0x426C93: matrix pos if m_matrix else
     placement pos). Then ensure obj's matrix (0x426C97; repeated at 0x426CB4, 0x426CE5, 0x426CFF, only the first can allocate).
     M = obj matrix.
  3. `P.x = (offY * M.forward.x + offX * M.right.x) + P.x`, stored f32 (0x426CC9-0x426CE1).
     `py = (offX * M.right.y + offY * M.forward.y) + P.y` kept in ST0 (0x426D17-0x426D34; note the opposite term order vs x).
  4. `V = veh->GetPosition()` (no allocation). `dx = P.x - V.x` stored f32 [esp+4] (0x426D43); `dy = py - V.y` stored f32
     (0x426D53/0x426D5A). `ang = GetATanOfXY(dx, dy)` (0x426D64), kept in ST0 until the end.
  5. `r = veh colmodel boundBox.max.x * 2.4f + 0.3f` (0x426D7A-0x426D87); `half = r / sqrt(dy*dy + dx*dx)` (sum order dy^2 first,
     0x426D8D-0x426DA5) stored f32; then `half = half * 0.5f` stored f32 (0x426E02-0x426E0C).
  6. A side (ecx = angleA): `d = ang - *angleA` (0x426DAD); `while (d < -pi) d += 2pi` (0x426DAF-0x426DD1; the test is `C0` only,
     NaN exits); `while (d > pi) d -= 2pi` (0x426DD3-0x426DF1; `test ah,0x41` so equal/NaN exit); `if (d < 0) d = -d` (0x426DF3-0x426E00).
     The half-arc product (step 5 second store) is computed here, between the normalisation and the compare.
     If `d < half` (0x426E10, strict, NaN false): `*angleA = ang - half` (fsubr, stored 0x426E21); if that value `< -pi`
     (0x426E23), reload it and `do x += 2pi while (x < -pi)`, store (0x426E30-0x426E45).
  7. B side (ecx = angleB, 0x426E47): same normalisation of `d = ang - *angleB` (0x426E4B-0x426EA0); if `d < half` (0x426EA2):
     `*angleB = ang + half` stored (0x426EAD-0x426EB1); if that value `> pi` (0x426EB3, `test ah,0x41`), reload and
     `do x -= 2pi while (x > pi)`, store (0x426EC0-0x426ED5). Otherwise pop ang (0x426EDC).
  The two sides are independent: both can be widened in one call.
- Edge cases / oddities: if `dx = dy = 0` the division gives `half = +inf` (no guard); then `d < inf` is true and
  `*angleA = ang - inf = -inf`, and the loop `x += 2pi while x < -pi` never terminates (0x426E32-0x426E43): an original hang
  (practically unreachable: needs the vehicle origin exactly at the offset point). Each adjusted angle is checked against the
  limit only once before the loop, so a single `ang - half` already in [-pi, pi] is stored unnormalised from the other side.
  Precision: `py` and `ang` are never spilled (extended exponent, 24-bit significand).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:1103-1105), no logic to compare. Open questions: none.

## 0x426EF0 CCarCtrl::PickNextNodeToChaseCar  (catalogue: specified/REA, port level 5)
- Convention / signature: `__cdecl bool (CVehicle* veh [esp+4], float destX [+8], float destY [+0xC], float destZ [+0x10])`,
  return AL, plain `ret` (0x427403 / 0x427737). Frame `sub esp,0x40` + push ebx/ebp/esi/edi (E = esp after the pushes; args at
  E+0x54..E+0x60). Argument slots are reused as scratch: E+0x54 := old `veh+0x394` (0x426F4F), E+0x58/E+0x5C/E+0x60 are
  overwritten in some paths (described below). AL = 1 means "no new node taken / car stopped" (0x4273FA), AL = 0 = advanced.
- Callers: CCarCtrl::PickNextNodeAccordingStrategy 0x432B10 (2 sites).
- Callees: srand 0x821B11 (CRT, cdecl (u32)), CPathFind::DoPathSearch 0x4515D0 (thiscall ECX = ThePaths 0x96F050, 17 dwords of
  args, callee-cleaned), CGeneral::GetATanOfXY 0x53CC70 (cdecl (x,y) -> ST0), CCarCtrl::StopCarIfNodesAreInvalid 0x422590
  (cdecl (veh) -> AL), _rand 0x821B1E (CRT MS LCG, returns 0..0x7FFF), CCarPathLink::OneWayLaneOffset 0x44DB00 (thiscall,
  pure, returns ST0; both results are discarded).
- Globals: CWeather::WeatherRegion 0xC81314 (i16), ThePaths.m_pPathNodes[] 0x96F854, m_pNaviNodes[] 0x96F974,
  m_pNodeLinks[] 0x96FA94, m_pNaviLinks[] 0x96FDF4 (all indexed by area*4), 0x8A5F44 = CNodeAddress constant
  {area 0xFFFF, node 0} (bytes `ffff0000`, an "empty node" passed as forbiddenNodeAddr).
  Constants: 0x858C48 = 0.125f, 0x858B3C = 3.0f, 0x858CB8 = 3.1415927f, 0x858CC0 = -3.1415927f, 0x858CBC = 6.2831855f,
  0x858B50 = 0.0f, 0x858FB4 = 256.0f; immediates 999999.875f (0x497423FE), 50.0f (0x42480000), 10.0f (0x41200000).
- Fields (vehicle; CAutoPilot at +0x390, gr names): +0x22 model (i16), +0x14 matrix / +0x04 placement (GetPosition),
  +0x390 m_currentAddress, +0x394 m_startingRouteNode, +0x398 m_endingRouteNode (CNodeAddress = u16 area, u16 node),
  +0x39C field_C, +0x3A0 m_nSpeedScaleFactor (u32), +0x3A4/+0x3A6/+0x3A8 m_nCurrent/Next/PreviousPathNodeInfo
  (CCarPathLinkAddress u16: area = v >> 10, index = v & 0x3FF), +0x3B4 _smthPrev, +0x3B5 _smthCurr, +0x3B6 _smthNext
  (i8 direction), +0x3B7 m_nCurrentLane, +0x3B8 m_nNextLane (i8), +0x3BA m_nCarMission (u8), +0x3DB m_nCarCtrlFlags
  (0x08 bStayInFastLane, 0x10 bStayInSlowLane), +0x3E0 autopilot field_50 (u8 lane-change countdown), +0x45E
  m_nForcedRandomRouteSeed (u16 read), +0x594 vehicle type (gr m_nVehicleSubType; 10 = VEHICLE_TYPE_BMX).
  CPathNode (0x1C): +0x08/+0x0A/+0x0C pos (i16, /8), +0x10 m_wBaseLinkId (i16), +0x18 low nibble m_nNumLinks.
  CCarPathLink (0xE): +0x00/+0x02 pos (i16, /8), +0x0B bits 0-2 m_numOppositeDirLanes, bits 3-5 m_numSameDirLanes.
- Behaviour (S = node `veh+0x394` (area As, node Ns), N_S = &m_pPathNodes[As][Ns]):
  1. Locals: `chosen.area = 0xFFFF` (E+0x14, word only, 0x426EF7); `res[0].area = res[1].area = 0xFFFF` (E+0x30/E+0x34,
     0x426F07); `maxDist = 999999.875f` (E+0x18). If `veh+0x45E != 0`: `srand((u16)seed)` (0x426F2D).
     `prev = veh+0x390` saved (E+0x1C); if `WeatherRegion == 0 || WeatherRegion == 4`: `maxDist = 50.0f` (0x426F53-0x426F5B).
  2. `ThePaths.DoPathSearch(pathType 0, originPos = N_S.pos * 0.125f (each i16 -> fild -> *0.125f -> f32), originAddr = S,
     targetPos = (destX, destY, destZ), outNodes = res, &count (i16 E+0x10), maxNodes 2, &pathDist (f32 E+0x24),
     maxSearchDistance = maxDist, targetNodeAddr = nullptr, maxUnkLimit = 999999.875f, oneSideOnly = 0,
     forbidden = *(CNodeAddress*)0x8A5F44, includeNodesWithoutLinks = (veh model == 0x21B) (sete al 0x426FB6; the pushed dword
     carries a pointer in bits 8-31), waterPath = 0)` (pushes 0x426FB4-0x42704B, call 0x427055).
  3. Countryside check (0x42705A): if `WeatherRegion == 0 || == 4`: if `count == 0` -> return 1 (0x427073); `V = veh pos`;
     `d = sqrt((destY - V.y)^2 + (destX - V.x)^2)` (0x427088-0x42709F; y term first); if `d * 3.0f < pathDist` -> return 1
     (0x4270A5-0x4270B4; NaN does not return). Returns 1 here touch nothing.
  4. Path result (0x4270BA), only if `count == 1 || count == 2`:
     a. If `res[0] != S` (area then node compared): `chosen = res[0]` (dword) and search S's links:
        `for (k = 0;; k++) if (m_pNodeLinks[As][N_S.baseLinkId + (i16)k] == res[0]) break;` (0x4270F4-0x42711C) -> commit (5).
     b. Else if `count == 2` and `res[1] != S`: same with `res[1]` (0x427136-0x427178) -> commit.
     c. Otherwise fall to 4'.
     The link search has NO bound on k (original): it relies on DoPathSearch returning a direct neighbour of S; if not, it
     reads past the link list until a match (or faults). E+0x5C (destY slot) holds baseLinkId during this search.
  4'. Heading fallback (0x42717A; count 0 or other, or the results equal S): `V = veh pos`;
     `target = GetATanOfXY(destX - V.x, destY - V.y)` (each difference stored f32 as the arg), stored f32 E+0x2C.
     `n = N_S.flags & 0xF` stored as a dword whose upper 16 bits are stale (only the word is compared later). `chosen.area = 0xFFFF`
     (word), `best = 10.0f`, `bestK = 0` (ebx). If `n <= 0` -> commit with k = 0 (0x4271DE).
     For `k = 0 .. n-1` (i16 compare, 0x42733F): `L = m_pNodeLinks[As][N_S.baseLinkId + k]`;
     skip if `L == prev` (both words) and `n > 1` (signed word, 0x427225); skip if `m_pPathNodes[L.area] == null`;
     `a = GetATanOfXY(L.x/8 - S.x/8, L.y/8 - S.y/8)` (positions i16 * 0.125f; x difference from two f32 locals, y difference in
     x87), `t = a - target` (0x4272B8); `while (t > pi) t -= 2pi` (0x4272BF-0x4272E1, `test ah,0x41`: equal/NaN exit);
     `while (t < -pi) t += 2pi` (0x4272E3-0x427301); `if (t < 0) t = -t`; if `t <= best` (0x427312: `test ah,0x41; jp` means
     less OR EQUAL takes, greater/NaN skip): `bestK = k`, `best = t` (f32), `chosen = L` (dword).
     After the loop, ebp is reloaded with N_S's offset (0x427352). With no acceptable link `chosen` keeps area 0xFFFF and
     whatever node word was at E+0x16 (uninitialised stack; only the area word was set).
  5. Commit (0x427356), k = link index (ebx):
     `veh+0x398 = old veh+0x390; veh+0x390 = old veh+0x394; veh+0x394 = chosen;`
     `field_C (+0x39C) += m_nSpeedScaleFactor (+0x3A0);`
     `+0x3A8 = old +0x3A4; +0x3A4 = old +0x3A6; +0x3A6 = m_pNaviLinks[As][N_S.baseLinkId + (i16)k]` (u16, 0x4273CC-0x4273E7);
     `+0x3B4 = old +0x3B5; +0x3B5 = old +0x3B6; +0x3B7 = old +0x3B8` (bytes).
  6. If `StopCarIfNodesAreInvalid(veh)` -> return 1 (0x4273EE-0x4273F8).
  7. Direction (0x427404): compare old S (E+0x54) with new `veh+0x394` as (area, node) unsigned words: if `S < new`
     (area less, or area equal and node less): `_smthNext = -1` (0xFF) and `lanes = navi(+0x3A6).byte0B & 7` (opposite lanes);
     else `_smthNext = 1` and `lanes = (navi(+0x3A6).byte0B >> 3) & 7` (same-dir lanes) (0x427424-0x42747F).
     `lanes` is u8 zero-extended, so the `jl 0x42748C` branch (keeps m_nCurrentLane as next lane) is dead code.
  8. Lane change (0x427497): with C = navi(+0x3A4), X = navi(+0x3A6) positions (i16 * 0.125f each):
     `q = (X.x/8 - C.x/8) * (X.x/8 - C.x/8) + (X.y/8 - C.y/8) * (X.y/8 - C.y/8)` (each difference computed twice from fresh
     filds, x87). If `q > 256.0f` (0x42761C, `test ah,0x41`: equal/NaN skip) and mission (+0x3BA) not in
     {2, 4, 0x0F, 0x34, 0x11, 0x36, 0x2B, 0x3D}: `if (--field_50 == 0) { field_50 = (rand() & 3) + 4;
     if (rand() < 0x3FFF) nextLane++; else nextLane--; }` (0x42764F-0x42767E).
  9. Clamp (0x427684): `nextLane = min((i8)nextLane, lanes - 1)`; `if (nextLane <= 0) nextLane = 0` (signed byte).
     Then (0x4276B0) if `flags & 0x08` (stay in fast lane): `nextLane = 0`; else if `flags & 0x10` (slow lane) or
     `veh+0x594 == 10`: `nextLane = max(lanes - 1, 0)` (0x4276D2-0x4276E2).
  10. Calls `OneWayLaneOffset` on navi(+0x3A4) and navi(+0x3A6), discarding both ST0 results (0x427704, 0x427727; no side
      effects). Return 0.
- Edge cases / oddities: unbounded link search (step 4a/4b); uninitialised `chosen.node` when the fallback finds nothing
  (step 4'); the tie rule `t <= best` picks the LAST of equal-angle links; `includeNodesWithoutLinks` dword has garbage upper
  bits (callee must only use the low byte); the countryside path-length rejection uses straight-line distance from the vehicle,
  not from S. srand is called with the seed every time (deterministic wander for that vehicle).
- gta-reversed: `plugin::CallAndReturn` only (CarCtrl.cpp:689-691), signature `(CVehicle*, float, float, float) -> bool` matches.
- Open questions: return-value meaning is taken from the bytes only (1 at 0x4273FA for both "path rejected" and "nodes invalid");
  vehicle +0x594 is named type in one existing spec and subtype in another (value compared: 10 at 0x4276C9).

## 0x427740 CCarCtrl::PickNextNodeToFollowPath  (catalogue: specified/REA, port level 5)
- Convention / signature: `__cdecl bool (CVehicle* veh [esp+4])`, return AL, plain `ret` (0x42786D / 0x427FD0). Frame
  `sub esp,0x38` + push ebx/ebp/esi/edi (E = esp after the four pushes; veh at E+0x4C, that slot is reused: first the saved
  old `veh+0x394` (0x427777), later the lane count / scratch). AL = 1: no path (0x427864); AL = 0: advanced (0x427FCA).
- Callers: CCarCtrl::PickNextNodeAccordingStrategy 0x432B10 (1 site).
- Callees: srand 0x821B11, CPathFind::DoPathSearch 0x4515D0 (thiscall ECX = ThePaths 0x96F050), CAutoPilot::RemoveOnePathNode
  0x41B950 (thiscall ECX = veh+0x390: `--count(+0x88); for (i = 0; i < count; i++) nodes[i] = nodes[i+1]` on +0x68 list),
  _rand 0x821B1E, CCarPathLink::OneWayLaneOffset 0x44DB00 (thiscall, pure -> ST0), CCurves::CalcSpeedScaleFactor 0x43C710
  (cdecl (const CVector& start, const CVector& end, float sdx, float sdy, float edx, float edy) -> ST0, caller pops 0x18),
  _ftol 0x821B40 (CRT, truncation), CCarCtrl::SwitchVehicleToRealPhysics 0x423FC0 (cdecl (veh)).
- Globals: ThePaths tables m_pPathNodes 0x96F854, m_pNaviNodes 0x96F974, m_pNodeLinks 0x96FA94, m_pNaviLinks 0x96FDF4;
  0x8A5F44 CNodeAddress {0xFFFF, 0} (forbidden node). Constants: 0x858C58 = 0.01f, 0x858C48 = 0.125f, 0x858FB4 = 256.0f,
  0x858C50 = 5.4f, 0x858C4C = 1000.0f; immediate 999999.875f (0x497423FE).
- Fields (veh; autopilot at +0x390, gr names, see PickNextNodeToChaseCar 0x426EF0 for the common ones): +0x390
  m_currentAddress, +0x394 m_startingRouteNode, +0x398 m_endingRouteNode, +0x39C field_C, +0x3A0 m_nSpeedScaleFactor,
  +0x3A4/+0x3A6/+0x3A8 current/next/previous CCarPathLinkAddress, +0x3B4..+0x3B8 _smthPrev/_smthCurr/_smthNext/
  m_nCurrentLane/m_nNextLane (i8), +0x3BA m_nCarMission, +0x3C8 m_speed (f32), +0x3DB m_nCarCtrlFlags (0x08 fast lane,
  0x10 slow lane, 0x40 bCantGoAgainstTraffic), +0x3EC m_vecDestinationCoors, +0x3F8 m_aPathFindNodesInfo[8],
  +0x418 m_nPathFindNodesCount (i16), +0x45E m_nForcedRandomRouteSeed. CCarPathLink +0x08/+0x09 m_dir (i8, *0.01).
  CPathNode +0x1A high nibble = m_nBehaviourType.
- Behaviour:
  1. If `seed (+0x45E) != 0`: `srand((u16)seed)` (0x42775B). `S0 = veh+0x394` saved (E+0x4C, 0x427777).
  2. If `count (+0x418) <= 0` (signed, 0x42777B): `DoPathSearch(pathType 0, originPos = veh->GetPosition() (matrix pos or
     placement, no allocation), originAddr = veh+0x394, targetPos = m_vecDestinationCoors, outNodes = veh+0x3F8,
     &count = veh+0x418, maxNodes 8, outDistance = nullptr, maxSearchDistance 999999.875f, targetNodeAddr nullptr,
     maxUnkLimit 999999.875f, oneSideOnly = (flags >> 6) & 1 (0x4277AB-0x4277BC), forbidden = *0x8A5F44,
     includeNodesWithoutLinks = (model == 0x21B) (`sete dl`; the dword's upper bits are the caller's EDX), waterPath 0)`.
     Then: if `count < 2` -> return 1 (0x427814; a one-node result also fails). If `nodes[0] == veh+0x394` ->
     RemoveOnePathNode (0x427835); the same test again with the shifted list -> RemoveOnePathNode (0x427859); if `count < 1`
     -> return 1 (0x42785E).
  3. Advance (0x42786E): `veh+0x398 = old +0x390; +0x390 = old +0x394; +0x394 = nodes[0]`; RemoveOnePathNode (0x427896);
     `+0x3A8 = old +0x3A4; field_C += m_nSpeedScaleFactor; +0x3B5 = old +0x3B6; +0x3B4 = old +0x3B5; +0x3A4 = old +0x3A6;
     +0x3B7 = old +0x3B8`.
  4. Link index (0x42790F): with S = new +0x390 (area As, node Ns), `for (k = 0;; k++) if (m_pNodeLinks[As][S.baseLinkId +
     (i16)k] == veh+0x394) break;` (unbounded, as in 0x426EF0). `+0x3A6 = m_pNaviLinks[As][S.baseLinkId + (i16)k]` (0x427964).
  5. Direction / lanes (0x42796B): if `S0 < new +0x394` (area, then node, unsigned words): `_smthNext = -1`,
     `lanes = (navi(+0x3A6).byte0B >> 3) & 7` (same-dir lanes); else `_smthNext = 1`, `lanes = byte0B & 7` (opposite lanes)
     (0x42798F-0x4279F9). NOTE: the mapping is the reverse of PickNextNodeToChaseCar (0x427424 uses `& 7` for the "<" case);
     both are original, port each as is. `lanes` is zero-extended, so `jl 0x427CC8` (0x427AE3) is dead.
  6. Direction vectors (0x4279EF-0x427AE1), C = navi(+0x3A4), X = navi(+0x3A6):
     `cd = ((i8)C.dir.x * 0.01f * curDir, (i8)C.dir.y * 0.01f * curDir)` with `curDir = (float)(i8)_smthCurr` (stored f32
     E+0x28/E+0x2C); `nd = (X.dir.x * 0.01f * nextDir, X.dir.y * 0.01f * nextDir)`, `nextDir = (float)(i8)_smthNext`
     (E+0x20/E+0x24). Each product is `(dir * 0.01f) * dirSign`.
  7. Lane change (0x427AE9-0x427C96): `q = (X.y/8 - C.y/8)^2 + (X.x/8 - C.x/8)^2` (each difference recomputed from fresh
     filds of the i16 positions * 0.125f). If `q > 256.0f` (0x427C67; equal/NaN skip) and `(rand() & 0x600) == 0`
     (`test ah,6`, 0x427C79): `if (rand() < 0x3FFF) nextLane++; else nextLane--;`. Unlike 0x426EF0 there is no mission filter
     and no countdown byte.
  8. Clamp (0x427C9C): `nextLane = min((i8)nextLane, lanes - 1)`; `if (nextLane <= 0) nextLane = 0`. Then if `flags & 0x08`:
     `nextLane = 0`; else if `flags & 0x10`: `nextLane = max(lanes - 1, 0)` (0x427CDF-0x427CED; no BMX test here).
  9. Curve timing (0x427CF3-0x427F7B):
     `latC = (OneWayLaneOffset(C) + (i8)m_nCurrentLane) * 5.4f` (fiadd of the i32, stored f32 E+0x4C);
     `latX = (OneWayLaneOffset(X) + (i8)m_nNextLane) * 5.4f` (x87);
     `end = (X.x/8 + latX * nd.y, X.y/8 - latX * nd.x, 0)` (E+0x30; y computed first, 0x427DB6-0x427DFB);
     `start = (C.x/8 + latC * cd.y, C.y/8 - latC * cd.x, 0)` (E+0x3C);
     `f = CalcSpeedScaleFactor(start, end, C.dir.x*0.01f*curDir, C.dir.y*0.01f*curDir, X.dir.x*0.01f*nextDir,
     X.dir.y*0.01f*nextDir)` (directions recomputed from the i8 fields, 0x427E80-0x427F47);
     `m_nSpeedScaleFactor = _ftol(f * (1000.0f / m_speed))` (0x427F55-0x427F66: 1000/speed first, then times f; speed 0 gives inf
     and _ftol's 0x80000000) and then `if (v <= 10) v = 10` (signed, 0x427F6B-0x427F7B; the unclamped value is stored first).
  10. Parking (0x427F81): if `m_pPathNodes[new.area] != null` and `(node.byte1A & 0xF0) == 0x20` (behaviour type 2, parking
      node): `SwitchVehicleToRealPhysics(veh)`; `m_nCarMission = 0x2D` (45 MISSION_PARK_PERPENDICULAR). Return 0.
- Edge cases / oddities: the unbounded link search (step 4) relies on consecutive path nodes being linked; step 2 rejects a
  one-node search result; a negative `m_nSpeedScaleFactor` (overflowed _ftol) is clamped to 10 by the signed compare.
- gta-reversed: `plugin::CallAndReturn` only (CarCtrl.cpp:694-696). Open questions: none affecting behaviour.

## 0x428040 CCarCtrl::DealWithBend_Racing  (catalogue: specified/REA, port level 1)
- Convention / signature: `__cdecl bool (CVehicle* veh [esp+4], CCarPathLinkAddress L1 [+8], L2 [+0xC], L3 [+0x10],
  L4 [+0x14] (each a u16 in a dword slot), int8 s1 [+0x18], int8 s2 [+0x1C], int8 s3 [+0x20], int8 s4 [+0x24],
  float speed [+0x28], float* outAngle [+0x2C], float* outSpeedMult [+0x30], float* outAngleDiff [+0x34],
  float* outDist [+0x38], CVector2D* outPos [+0x3C] (only .x/.y written))`, returns AL (0/1), plain `ret`.
  s1..s4 are the per-link travel direction signs (+1/-1 in practice; used only as multipliers of the link direction).
  Frame: `sub esp,0x40` then push ebx/ebp/esi/edi; below `[sp+X]` means esp after the four pushes (arg k at entry offset o
  is `[sp+0x50+o]`). The L1 arg slot `[sp+0x58]` is reused as scratch (node-1 y, later the chosen corner y).
- Callers: CCarCtrl::SteerAICarWithPhysicsFollowPath_Racing 0x435830 (1 site). Callees: CCarPathLink::OneWayLaneOffset
  0x44DB00 (thiscall ECX = link -> ST0, 8 sites), CGeneral::GetATanOfXY 0x53CC70 (cdecl (x, y) -> ST0, 4 sites),
  CCarCtrl::FindIntersection2Lines 0x4226F0 (cdecl, 10 dword args, `add esp,0x28`).
- Globals: ThePaths (0x96F050) `m_pPathNodes[area]` at 0x96F854 (only null-tested), `m_pNaviNodes[area]` at 0x96F974
  (CCarPathLink*, stride 0xE). Constants: 0x858C48 = 0.125, 0x858B34 = 60.0, 0x858628 = 100.0, 0x858C58 = 0.01,
  0x858CB8 = 3.1415927, 0x858CBC = 6.2831855, 0x858CC0 = -3.1415927, 0x858B50 = 0.0, 0x858624 = 1.0, 0x858C50 = 5.4,
  0x858FBC = 11.0, 0x858B1C = 0.1, 0x858FB8 = 0.63661975, 0x858CC8 = 0.6, 0x858F10 = 0.033333335, 0x858CC4 = 0.2,
  0x858BA4 = 20.0; immediates 20deg 0.34906587 (0x3EB2B8C3), 40deg 0.69813174 (0x3F32B8C3), 30deg 0.52359879
  (0x3F060A92), 60deg 1.0471976 (0x3F860A92), 50.0f, 40.0f, 1.0f.
- Fields: veh +0x594 dword (gr `m_nVehicleSubType`, eVehicleType; 10 = VEHICLE_TYPE_BMX), +0x14 m_matrix (fwd +0x10/+0x14,
  pos +0x30/+0x34), +0x04 placement pos. CCarPathLink: +0 i16 posn.x, +2 i16 posn.y (x0.125), +8 i8 dir.x, +9 i8 dir.y
  (x0.01), +0xB byte: bits 0-2 m_numOppositeDirLanes ("opp"), bits 3-5 m_numSameDirLanes ("same").
  Link address: area = addr >> 10, index = addr & 0x3FF (0x4280B2-0x428103).
- Behaviour:
  1. If `L1 == 0xFFFF` return 0 (0x428047; before any output write). If `L2 == 0xFFFF` return 0 (0x42805A).
  2. Thresholds (0x42806A): if `veh+0x594 == 10` (BMX): `thrA = 20deg 0.34906587`, `thrB = 40deg 0.69813174`,
     `distLim = 50.0f`; else `thrA = 30deg 0.52359879`, `thrB = 60deg 1.0471976`, `distLim = 40.0f`.
  3. `*outSpeedMult = 1.0f` (0x4280AC). If `ThePaths.m_pPathNodes[L1.area] == null` (0x4280C3) or
     `m_pPathNodes[L2.area] == null` (0x4280DC) return 0. `K1 = &m_pNaviNodes[L1.area][L1.idx]`,
     `A = &m_pNaviNodes[L2.area][L2.idx]`.
  4. `n1x = (float)K1.posn.x * 0.125f` (0x428110-0x42812A, stored f32), `n1y = (float)K1.posn.y * 0.125f` (stored f32).
     `P = veh->GetPosition()` (no allocation). `d = sqrt((n1y - P.y)*(n1y - P.y) + (n1x - P.x)*(n1x - P.x))`
     (0x428159-0x428181; y product first); `*outDist = d` (0x428185, f32). If `d > 60.0f` (0x428187, `test ah,0x41; je`;
     NaN continues) return 0.
  5. Choose the exit link (esi) and its sign (dl), 0x428198-0x4282D4:
     - If `L3 == 0xFFFF`: link = A, sign = s2 (0x4282D4).
     - Else `K3 = navi(L3)`; `ey = A.y*0.125 - K3.y*0.125`, `ex = A.x*0.125 - K3.x*0.125` (each int16 -> fild -> *0.125,
       0x4281BF-0x428207); if `ex*ex + ey*ey < 100.0f` (0x428213, `test ah,5`): link = K3, sign = s3 (0x428224);
       else link = A, sign = s2 (0x42823C).
     - Then (only if L3 was valid) if `L4 != 0xFFFF` (0x428244): `K4 = navi(L4)`; the same squared-distance test between
       the CURRENT link (A or K3) and K4 (0x428267-0x4282BB); if `< 100.0f`: link = K4, sign = s4 (0x4282CC). Otherwise
       keep. (If K3 was rejected, K4 is still compared against A.)
     Call the chosen link `B` and its sign `sB`.
  6. 0x4282D8-0x42838C: `Bx = B.x*0.125`, `By = B.y*0.125` (stored f32 [sp+0x28]/[sp+0x2C]);
     `D1x = (K1.dir.x*0.01f)*(float)s1` [sp+0x3C], `D1y = (K1.dir.y*0.01f)*(float)s1` [sp+0x38];
     `DBx = (B.dir.x*0.01f)*(float)sB` [sp+0x30], `DBy = (B.dir.y*0.01f)*(float)sB` [sp+0x34] (each: fild int8,
     fmul 0x858C58, fmul by the fild'ed sign, stored f32).
  7. `h = GetATanOfXY(veh->m_matrix->fwd.x, fwd.y)` (0x42838E; raw forward, not normalised, matrix not null-tested),
     stored f32; `diff = h - GetATanOfXY(DBx, DBy)` (0x4283A1, fsubr). Wrap, storing `diff` f32 into [sp+0x44] after
     each step: first `while (diff > PI) diff -= 2PI` (0x4283AD-0x4283D3, `test ah,0x41`), then
     `while (diff < -PI) diff += 2PI` (0x4283D7-0x428403, `test ah,5`). x87 keeps the unrounded value in st0.
  8. If `|diff| < thrA` (0x428407-0x428421) return 0. If both B and K1 have `opp + same >= 4` (0x42842F-0x428459) and
     `|diff| < thrB` (0x42845B-0x428475) return 0. (abs via `fcom 0; fchs if less`.)
  9. `*outAngleDiff = diff` (0x428480, the x87 value).
  10. Edge points of K1 (0x428482-0x428545), with `o_k = K1.OneWayLaneOffset()` (a fresh call before each line):
      `P1x = (o + ((float)opp1 - 1.0f)) * D1y * 5.4f + n1x` [sp+0x14];
      `P1y = n1y - (o + ((float)opp1 - 1.0f)) * D1x * 5.4f` [sp+0x18];
      `Q1x = n1x - (o + ((float)same1 - 1.0f)) * D1y * 5.4f` [sp+0x1C];
      `Q1y = n1y + (o + ((float)same1 - 1.0f)) * D1x * 5.4f` (x87). Each: fild count, fsub 1.0, faddp (o + ..),
      fmul D, fmul 0x858C50, then fadd/fsubr of n1.
      `dP = (P1x-Bx)^2 + (P1y-By)^2`, `dQ = (Q1x-Bx)^2 + (Q1y-By)^2` (0x428549-0x428579; x term first in each);
      if `dQ > dP` (0x42857B fcompp, `test ah,0x41; jne`): `N = P1`, `F = Q1`; else (<=, NaN): `N = Q1`, `F = P1`
      (N at [sp+0x24]/[sp+0x20], F at [sp+0x4C]/[sp+0x48]).
  11. `ex = F.x - N.x`, `ey = F.y - N.y`, `w = sqrt(ey*ey + ex*ex)` (0x4285BE-0x4285DC). If `w > 11.0f` (0x4285DE,
      `test ah,0x41; jne`): `k = 11.0f / w`; `F.x = k*ex + N.x`, `F.y = k*ey + N.y` (0x4285EB-0x428603).
  12. Edge points of B (0x428611-0x4286D3), same formulas with `o = B.OneWayLaneOffset()` per line, DB and (Bx,By):
      `P2x = (o + (oppB - 1)) * DBy * 5.4f + Bx`, `P2y = By - (o + (oppB - 1)) * DBx * 5.4f`,
      `Q2x = Bx - (o + (sameB - 1)) * DBy * 5.4f`, `Q2y = By + (o + (sameB - 1)) * DBx * 5.4f`.
      Distances to (n1x, n1y) (0x4286D7-0x428707); if `dQ2 > dP2`: `M = P2` else `M = Q2` (M at [sp+0x2C]/[sp+0x58]).
  13. Corner (0x42873A-0x4287CB): if `|D1x - DBx| < 0.1f` (0x428751, `test ah,5`; NaN fails) AND `|D1y - DBy| != 0.0f`
      (0x428775 fcomp 0.0, `test ah,0x44; jnp` = equal goes to the intersection; NaN counts as "not equal"):
      `outPos->x = N.x`, `outPos->y = N.y` (0x428789-0x428796). Otherwise
      `FindIntersection2Lines(N.x, N.y, D1x, D1y, M.x, M.y, DBx, DBy, &outPos->x, &outPos->y)` (0x4287AC-0x4287C6; arg
      order exactly this; FindIntersection2Lines returns point1 when the cross product is 0).
  14. If `*outDist < distLim` (0x4287D5, `test ah,5`): `c = min(|diff_f32 * 0.63661975f|, 1.0f)` (diff re-read as the f32
      at [sp+0x44]; 0x4287E2-0x428825: `1.0 < |t|` -> 1.0 else |t|, NaN keeps |t|);
      `*outSpeedMult = 1.0f - ((distLim - *outDist) / distLim) * c * 0.6f` (0x428827-0x428846, x87 in this order).
  15. `c` recomputed identically (0x42884A-0x42888D). `s = speed * 0.033333335f` (0x42888F); clamp: if `s > 1.0f` -> 1.0;
      else if `s < 0.2f` -> 0.2 (0x428899-0x4288D6; NaN stays NaN). If `*outDist < s * c * 20.0f` (0x4288DC-0x4288EF,
      product order (s*c)*20; `test ah,5`):
      `*outAngle = GetATanOfXY(outPos->x - P.x, outPos->y - P.y)` (0x428905-0x428927; P = veh->GetPosition());
      else `*outAngle = GetATanOfXY(F.x - P.x, F.y - P.y)` (0x42894C-0x428972). Both return 1.
- Edge cases / oddities:
  - Probable original bug (step 13): the "lines parallel" shortcut requires `|D1y - DBy| != 0` instead of a small
    tolerance like the x test; when the y components are exactly equal and x nearly equal the code calls
    FindIntersection2Lines on (near-)parallel lines. Port as is.
  - Step 5: when K3 is too far, the K4 test still runs against A (not skipped).
  - `*outSpeedMult = 1.0` and `*outDist` are written even on several `return 0` paths (after step 3 / step 4).
  - Register reliance: after FindIntersection2Lines (0x4287C6) the code re-uses EDX as `outPos` (0x428911 `fld [edx]`)
    without reloading it; this is valid only because 0x4226F0 does not touch EDX (it uses only EAX/ECX). A port just uses
    outPos.
  - Area pointers are tested in `m_pPathNodes` but the data comes from `m_pNaviNodes` (same area index).
- gta-reversed: `plugin::CallAndReturn` only (CarCtrl.cpp:353-355); signature (vehicle, 4 link addresses, 4 int8,
  float, 4 float*, CVector*) matches the bytes. gr's arg6..arg9 = s1..s4, arg10 = speed, arg11..arg14 = outAngle,
  outSpeedMult, outAngleDiff, outDist.
- Open questions: none affecting behaviour (names of outputs are inferred from use).

## 0x428990 CCarCtrl::SteerAICarWithPhysicsTryingToBlockTarget_Stop  (catalogue: specified/REA, port level 4)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], float targetX [+8], float targetY [+0xC],
  float targetMoveX [+0x10], float targetMoveY [+0x14], float* outSteer [+0x18], float* outGas [+0x1C],
  float* outBrake [+0x20], bool* outHandbrake [+0x24])`, plain `ret` (0x428A1F / 0x428BD6). Preserves EBX/ESI/EDI
  (pushed); EBX = 0 throughout. The `targetX` arg slot is overwritten with the squared distance (0x4289F3); the `targetMoveX/Y` names are proposed (they are only squared and compared, 0x428A8A).
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (2 sites). Callees: CTaskManager::GetActiveTask
  0x681720 (thiscall), task vtable slot +0x10 (GetTaskType, thiscall -> EAX), CWorld::FindPlayerVehicle 0x56E0D0
  (cdecl (int playerId = -1, bool = false) -> CVehicle*, called up to 5 times), CVehicle::IsUpsideDown 0x6D1D90
  (thiscall -> AL), CRT `_ftol2` 0x821B40 (ST0 -> EAX, truncation toward zero), CCarAI::TellOccupantsToLeaveCar
  0x41C760 (cdecl (CVehicle*)).
- Globals: CTimer::ms_fTimeStep 0xB7CB5C (f32, R). Constants: 0x858628 = 100.0, 0x858FC4 = 0.0001, 0x858FC0 = 0.0004,
  0x858C28 = 0.05, 0x858CF0 = 16.666666 (f32 = 16.66666603); immediate 1.0f (0x3F800000) and literal 0x9C4 = 2500.
- Fields: veh +0x14 m_matrix (pos +0x30/+0x34), +0x04 placement pos; +0x44/+0x48/+0x4C m_vecMoveSpeed;
  +0x3BA u8 m_autoPilot.m_nCarMission; +0x3D0 u8 m_autoPilot.m_nCruiseSpeed; +0x428 bit 0 bIsLawEnforcer
  (first vehicle flag dword); +0x460 m_pDriver; +0x4DC u16 m_nCopsInCarTimer (gr declares int16; the original reads
  it with `movzx` and compares unsigned, 0x428B09 / 0x428B99). Ped +0x47C m_pIntelligence, intelligence +4 CTaskManager.
- Behaviour:
  1. Defaults, in this store order: `*outSteer = 0` (0x42899F), `*outGas = 0` (0x4289A5), `*outBrake = 1.0f` (0x4289AC),
     `*outHandbrake = 1` (0x4289B2).
  2. `P = veh->GetPosition()` (matrix pos if m_matrix else placement pos, 0x4289B5-0x4289D0; no allocation).
     `distSq = (P.y - targetY)*(P.y - targetY) + (P.x - targetX)*(P.x - targetX)` (0x4289D3-0x4289F1, y product first,
     each difference computed twice from memory, x87), stored f32 into the targetX slot (0x4289F3).
  3. If `distSq > 100.0f` (0x4289F7 fcomp 0x858628, `test ah,0x41; jne` - equal/NaN go to step 4):
     `veh->m_nCarMission = (m_nCarMission == 19 /*BLOCKCAR_HANDBRAKESTOP*/) ? 18 /*BLOCKCAR_CLOSE*/ : 5
     /*BLOCKPLAYER_CLOSE*/` (0x428A04-0x428A17, branchless setne/dec/and 0xD/add 5). Return.
  4. Close. If `m_nCarMission == 19` (0x428A2A):
     a. If `driver = veh->m_pDriver` (0x428A37) is non-null and `task = driver->m_pIntelligence->m_TaskMgr.GetActiveTask()`
        (0x428A4A) is non-null and `task->GetTaskType() == 0x451` (0x428A5A; 1105 = TASK_COMPLEX_KILL_CRIMINAL by gr's
        eTaskType numbering) -> return.
     b. `mx*mx + my*my` (own move speed, 0x428A65-0x428A73; x product first) must be `< 0.0001f` (0x428A75, `test ah,5;
        jp` -> return on >= / NaN).
     c. `targetMoveX*targetMoveX + targetMoveY*targetMoveY < 0.0004f` (0x428A8A-0x428AA7), else return.
     d. If `veh->bIsLawEnforcer` (0x428AAD `test byte [+0x428],1`) -> step 6; else return.
  5. Close and mission != 19 (0x428ABF):
     a. If `FindPlayerVehicle(-1,0)` (0x428AC2) is non-null: `pv = FindPlayerVehicle(-1,0)` (0x428AD1);
        `s = sqrt((x*x + y*y) + z*z)` of pv->m_vecMoveSpeed (0x428AD6-0x428AF4). If `s < 0.05f` (0x428AFC, `test ah,5`):
        `m_nCopsInCarTimer = (u16)_ftol2(ms_fTimeStep * 16.666666f + (float)(int)(u16)m_nCopsInCarTimer)` (0x428B09-0x428B29:
        fld timestep, fmul 0x858CF0, fiadd of the zero-extended timer, truncation, low 16 bits stored); else (>= / NaN)
        `m_nCopsInCarTimer = 0` (0x428B32). If there is no player vehicle: `m_nCopsInCarTimer = 0` (0x428B32).
     b. If `FindPlayerVehicle(-1,0)` (0x428B3C) is non-null and `!FindPlayerVehicle(-1,0)->IsUpsideDown()` (0x428B55):
        recompute `s` exactly as in 5a (0x428B66-0x428B84); return unless `s < 0.05f` (0x428B8C) AND
        `m_nCopsInCarTimer > 2500` (unsigned, 0x428B99 `cmp word,0x9C4; jbe` returns). Null player vehicle or upside-down
        skip these tests.
     c. 0x428BA4: return unless `bIsLawEnforcer`; return unless `distSq < 100.0f` (re-read stored f32, 0x428BAD;
        distSq == 100 or NaN returns here).
  6. 0x428BBE: `CCarAI::TellOccupantsToLeaveCar(veh)`; `m_nCruiseSpeed = 0` (0x428BC7); `m_nCarMission = 0` (MISSION_NONE,
     0x428BCD).
- Edge cases / oddities: the far branch only demotes the mission (to *_CLOSE), it does not leave the brake/handbrake
  defaults (they stay set). The timer is accumulated in milliseconds through float and wraps at 16 bits (no clamp). With
  `distSq` exactly 100.0 the function goes to the close path (step 4/5) but can never reach step 6 via step 5c.
  Step 4 path does not test the distance again (any distSq <= 100 or NaN qualifies).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:993-996), signature (vehicle, x, y, arg4, arg5, float*, float*, float*,
  bool*) matches the bytes; gr's arg6..arg9 are steer/gas/brake/handbrake.
- Open questions: meaning of args +0x10/+0x14 (treated as the target's move speed x/y; only squared at 0x428A8A-0x428A96,
  behaviour is exact regardless of name).

## 0x428BE0 CCarCtrl::SteerAIBoatWithPhysicsHeadingForTarget  (catalogue: specified/REA, port level 1)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], float targetX [+8], float targetY [+0xC],
  float* outSteer [+0x10], float* outGas [+0x14], float* outBrake [+0x18])`, plain `ret` (0x428D83 / 0x428DD0).
  Frame: `sub esp,0x10; push esi`. The function reuses its own argument slots as scratch: the `veh` slot receives
  `forward.y` (0x428BF1) and later `(float)cruiseSpeed` (0x428D1E); the `targetY` slot receives the angle to the target
  (0x428C67). Harmless for a cdecl caller that does not re-read its pushed args (sole caller does not).
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1 site). Callees: CGeneral::GetATanOfXY 0x53CC70
  (cdecl (float x, float y) -> ST0) x2.
- Globals: none (only .rdata constants: 0x858B50 = 0.0, 0x858624 = 1.0, 0x858CC0 = -3.1415927, 0x858CBC = 6.2831855,
  0x858CB8 = 3.1415927, 0x858F40 = -0.5, 0x858B8C = 0.5, 0x858B34 = 60.0, 0x858FC8 = -5.0, 0x858C1C = -1.0,
  0x858C84 = 0.25, 0x858B90 = 4.0; immediates 1.0f 0x3F800000, -0.1f 0xBDCCCCCD, -0.2f 0xBE4CCCCD).
- Fields: veh +0x14 m_matrix (forward at +0x10/+0x14, pos at +0x30/+0x34); veh +0x04 placement pos (only if matrix
  null, unreachable, see Edge cases); veh +0x44/+0x48 CPhysical m_vecMoveSpeed x/y; veh +0x3D0 u8
  m_autoPilot.m_nCruiseSpeed (m_autoPilot at +0x390, field +0x40).
- Behaviour:
  1. `m = veh->m_matrix` (0x428BE8, no null check). `fx = m->fwd.x` (0x428BEE), `fy = m->fwd.y` (0x428BEB, kept as f32 in
     the veh arg slot). `len = sqrt(fy*fy + fx*fx)` (0x428BF5-0x428C03, x87, fy*fy first).
     If `len == 0.0f` (0x428C05 `test ah,0x44; jnp`; NaN does NOT take this path): `nx = 1.0f`, `ny = fy` (raw, not
     normalised) (0x428C2C). Else `inv = 1.0f / len` (0x428C12, fdivr 0x858624), `nx = inv*fx` (0x428C1A, stored f32),
     `ny = inv*fy` (0x428C20, stored f32).
  2. `P = m ? &m->pos : &veh->placement.pos` (0x428C34-0x428C44 / 0x428CDA). `dy = targetY - P.y` (0x428C47, stored f32),
     `dx = targetX - P.x` (0x428C55, stored f32). `aTarget = GetATanOfXY(dx, dy)` (0x428C5E), stored f32 (0x428C67).
     `aHead = GetATanOfXY(nx, ny)` (0x428C71). `d = aTarget - aHead` (0x428C76 fsubr, kept in x87).
  3. Wrap: `while (d < -PI) d += 2PI` (0x428C7D-0x428CA1; `test ah,5`, NaN exits); `while (d > PI) d -= 2PI`
     (0x428CA3-0x428CC1; `test ah,0x41`, equal/NaN exit). Constants 0x858CC0 / 0x858CBC / 0x858CB8.
  4. Clamp: if `d < -0.5f` then `d = -0.5f` (0x428CC3-0x428CD2); else if `d > 0.5f` then `d = 0.5f` (0x428CE2-0x428CF1).
     `steer = d` stays in x87 (st1 from here on, not rounded until stored).
  5. `cruise = (float)(int)veh->m_nCruiseSpeed` (u8 zero-extended, fild, stored f32 0x428D1E).
     `spd = sqrt(mv.y*mv.y + mv.x*mv.x)` (0x428D22-0x428D34, y term first, from copies of +0x48/+0x44);
     `diff = cruise - spd * 60.0f` (0x428D36 fmul 0x858B34, 0x428D3C fsubr).
  6. If `diff > 0.0f` OR diff is NaN (0x428D40 `test ah,0x41; jp 0x428D84`) -> accelerate:
     `r = diff / cruise` (0x428D84). If `r > 0.25f` (0x428D88 `test ah,0x41; jne`): `*outGas = 1.0f` (0x428D99);
     else (<=, NaN): `*outGas = 1.0f - (0.25f - r) * 4.0f` (0x428DA1-0x428DB9, exactly this order: fld 0.25, fsub r,
     fmul 4.0, fsubr 1.0). Then `*outBrake = 0.0f` (0x428DC5), `*outSteer = steer` (0x428DCB). Return.
  7. Else (diff <= 0): `*outGas = -0.1f` (0x428D57); if `diff < -5.0f` (0x428D4D fcomp 0x858FC8, `test ah,5; jp`)
     `*outGas = -0.2f` (0x428D64). `*outBrake = 0.0f` (0x428D78), `*outSteer = steer * -1.0f` (0x428D6A fmul 0x858C1C,
     stored 0x428D7E) - steering is mirrored when reversing. Return.
  Store order in both tails: outGas, outBrake, outSteer.
- Edge cases / oddities: matrix pointer is dereferenced at 0x428BEB before the null test at 0x428C34, so the
  placement-pos path is dead in practice. `cruise == 0` with the boat moving gives diff < 0 (reverse branch); with
  `diff > 0` impossible when cruise == 0. cruise == 0 and NaN speed -> r = NaN/0 -> gas NaN. When len == 0 the y heading
  component is the raw (zero) value.
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:938-941), signature matches (vehicle, x, y, float*, float*, float*);
  gr's arg names arg4/arg5/arg6 = steer/gas/brake per the bytes.
- Open questions: none.

## 0x428DE0 CCarCtrl::SteerAIBoatWithPhysicsAttackingPlayer  (catalogue: specified/REA, port level 20)
- Convention / signature: `__cdecl void (CVehicle* veh [esp+4], float* outSteer [+8], float* outGas [+0xC],
  float* outBrake [+0x10], bool* outHandbrake [+0x14])`, plain `ret` (0x429083 / 0x42908B). Frame `sub esp,0x1C;
  push esi; push edi`. The `veh` arg slot is reused as scratch (normalised forward.y at 0x428E5D/0x428E90, later
  `(float)cruiseSpeed` at 0x428FA9).
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90 (1 site). Callees: CWorld::FindPlayerCoors 0x56E010
  (cdecl, returns CVector by hidden pointer: `(CVector* ret, int playerId = -1)`, EAX = ret; called 3 times),
  CWorld::FindPlayerSpeed 0x56E090 (cdecl `(int playerId = -1)` -> CVector*; called 2 times), CGeneral::GetATanOfXY
  0x53CC70 (cdecl (x, y) -> ST0) x2, CVehicle::FireFixedMachineGuns 0x73DF00 (thiscall ECX = veh).
- Globals: none. Constants: 0x858C28 = 0.05, 0x858CA0 = 2.0, 0x858B50 = 0.0, 0x858624 = 1.0, 0x858B34 = 60.0,
  0x858CC0 = -3.1415927, 0x858CBC = 6.2831855, 0x858CB8 = 3.1415927, 0x858FC8 = -5.0, 0x858C84 = 0.25, 0x858B90 = 4.0,
  0x858A10 = 40.0, 0x858FCC = 0.15; immediates 2.0f, 1.0f, -0.1f (0xBDCCCCCD), -0.2f (0xBE4CCCCD).
- Fields: veh +0x14 m_matrix (fwd +0x10/+0x14, pos +0x30..), +0x04 placement pos, +0x22 i16 m_nModelIndex,
  +0x44/+0x48 m_vecMoveSpeed, +0x3D0 u8 m_autoPilot.m_nCruiseSpeed.
- Behaviour:
  1. `P = veh->GetPosition()` (0x428DE8-0x428DF3, no allocation). `pp = FindPlayerCoors(-1)` (0x428DFD).
     `dx = pp.x - P.x`, `dy = pp.y - P.y`, `dz = pp.z - P.z` (x87, 0x428E02-0x428E12);
     `dist = sqrt((dz*dz + dy*dy) + dx*dx)` (0x428E15-0x428E25), stored f32 (0x428E2D, `fst`, value kept).
  2. `lead = dist * 0.05f` (0x428E31); if `lead < 2.0f` (0x428E37, `test ah,5; jp`) store `lead` (f32, 0x428E44),
     else (>= or NaN) `lead = 2.0f` (0x428E4C).
  3. Heading: `m = veh->m_matrix` (0x428E54, dereferenced without null test). `fx = m->fwd.x`, `fy = m->fwd.y`;
     `len = sqrt(fy*fy + fx*fx)` (0x428E61-0x428E6F). If `len == 0` (`test ah,0x44; jnp`, NaN not equal):
     `nx = 1.0f`, `ny = fy` raw (0x428E98). Else `inv = 1.0f/len`; `nx = inv*fx` (0x428E88), `ny = inv*fy` (0x428E90),
     both stored f32.
  4. Predicted target (exact order): `sp = FindPlayerSpeed(-1)` (0x428EA4); `pp = FindPlayerCoors(-1)` (0x428EB2);
     `tx = (lead * sp->x) * 60.0f + pp.x` (0x428EB7-0x428EC5), stored f32 (0x428EC7).
     `sp = FindPlayerSpeed(-1)` (0x428ECB); `t = (lead * sp->y) * 60.0f` (0x428ED0-0x428EDE) stored f32 (0x428EE4, into
     the `lead` slot); `pp = FindPlayerCoors(-1)` (0x428EE8); `ty = t + pp.y` (0x428EF4, x87, not stored).
  5. `P = veh->GetPosition()` again (0x428EF7-0x428F08). `ay = ty - P.y` (0x428F0B, stored f32), `ax = tx - P.x`
     (0x428F15, stored f32); `aTarget = GetATanOfXY(ax, ay)` (0x428F1E), stored f32 (0x428F27);
     `d = aTarget - GetATanOfXY(nx, ny)` (0x428F31, 0x428F36 fsubr). Wrap: `while (d < -PI) d += 2PI`
     (0x428F3D-0x428F61, `test ah,5`); `while (d > PI) d -= 2PI` (0x428F63-0x428F81, `test ah,0x41`).
     NO clamp of the steering angle (contrast with 0x428BE0 which clamps to +-0.5). `steer = d` kept in x87.
  6. `cruise = (float)(int)m_nCruiseSpeed` (0x428F8F-0x428FA9, stored f32). `spd = sqrt(my*my + mx*mx)` (0x428FAD-0x428FBF,
     copies of +0x48/+0x44). `diff = cruise - spd * 60.0f` (0x428FC1, 0x428FC7).
     - If `diff > 0` or NaN (0x428FCB `test ah,0x41; jp 0x428FFF`): `r = diff / cruise` (0x428FFF); if `r > 0.25f`
       (0x429003, `test ah,0x41; jne`) `*outGas = 1.0f` (0x429014) else `*outGas = 1.0f - (0.25f - r) * 4.0f`
       (0x42901C-0x429034). Pop r (0x429036).
     - Else: `*outGas = -0.1f` (0x428FE2); if `diff < -5.0f` (0x428FD8 fcomp, `test ah,5; jp`) `*outGas = -0.2f`
       (0x428FEF). The steering is NOT negated here (contrast 0x428D6A in HeadingForTarget).
  7. 0x429038: `*outBrake = 0.0f` (0x429046), `*outSteer = steer` (0x42904C, stored f32 from a duplicate; steer stays
     in st0), `*outHandbrake = 0` (0x42904E).
  8. If `veh->m_nModelIndex == 430` (MODEL_PREDATOR, 0x429051) and `dist < 40.0f` (0x429059-0x429068, f32 from step 1)
     and `steer < 0.15f` (0x42906A fcomp of the x87 steer value, `test ah,5; jp`): `veh->FireFixedMachineGuns()`
     (0x429079). The x87 stack is cleaned on every path (0x429084 pops when the model/dist test fails).
- Edge cases / oddities: original bug - the fire test compares the signed steering angle `steer < 0.15f`, not its
  absolute value, so the Predator fires whenever the target is anywhere to one side (any negative d, down to -PI),
  not only when it is ahead. FindPlayerSpeed/FindPlayerCoors are re-evaluated per component (same result in
  practice). `cruise == 0` behaves as in 0x428BE0 (diff <= 0 -> reverse gas -0.1/-0.2).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:928-931); signature (vehicle, float*, float*, float*, bool*) matches.
- Open questions: none.

## 0x429090 CCarCtrl::SteerAIBoatWithPhysicsCirclingPlayer  (catalogue: specified/REA, port level 1)
- Convention / signature: `__cdecl void (CVehicle* boat [esp+4], float* outSteer [+8], float* outGas [+0xC],
  float* outBrake [+0x10], bool* outHandbrake [+0x14])`, plain `ret` (0x4292F4). (gr header names them arg2..arg5; meaning
  from the writes at 0x4292DA-0x4292EE and the caller SteerAICarWithPhysics_OnlyMission 0x436A90.) The `boat` argument
  slot is reused as scratch (f32 `H.y` at 0x429158, then f32 `cruise` at 0x42924B).
- Callers: CCarCtrl::SteerAICarWithPhysics_OnlyMission 0x436A90. Callees: CWorld::FindPlayerCoors 0x56E010 (cdecl
  `CVector* (CVector* out, int32 playerId = -1)`, returns `out` in EAX; called twice), CVector::Normalise 0x59C910 (thiscall),
  CGeneral::GetATanOfXY 0x53CC70 (cdecl `(float x, float y)` -> ST0; called twice).
- Globals: none. Constants: 0x858FD4 = -12.0 (f32), 0x858FD0 = 26.0 (f32), 0x858B50 = 0.0 (f32), 0x858624 = 1.0 (f32),
  0x858CC0 = -3.1415927 (f32), 0x858CBC = 6.2831855 (f32), 0x858CB8 = 3.1415927 (f32), 0x858B34 = 60.0 (f32),
  0x858FC8 = -5.0 (f32), 0x858C84 = 0.25 (f32), 0x858B90 = 4.0 (f32); immediates -0.1f (0xBDCCCCCD), -0.2f (0xBE4CCCCD), 1.0f.
- Fields: entity +0x14 m_matrix (forward x/y at +0x10/+0x14, pos +0x30), +0x04 placement pos, +0x20 u8 (low byte of
  CEntity::m_nRandomSeed), +0x44 CPhysical::m_vecMoveSpeed (x, y used), +0x3D0 u8 = autopilot(+0x390)+0x40 `m_nCruiseSpeed`.
- Behaviour:
  1. `V = boat->GetPosition()` (0x429098-0x4290A3). `P = FindPlayerCoors(-1)` (0x4290AD). Local `D = (P.x - V.x, P.y - V.y, 0)`
     (y computed first at 0x4290B2, x at 0x4290BB; z stored 0 at 0x4290C3). `D.Normalise()` (0x4290D3).
  2. Offset: `k = (m_nRandomSeed & 1) ? -12.0f : 26.0f` (byte +0x20 test at 0x4290E5). `O = (D.y * k, -D.x * k, 0 * k)`
     (0x4290F1-0x429129; `-D.x` via `fchs` 0x4290E3; O.z is -0.0 for k = -12, unused).
  3. `P2 = FindPlayerCoors(-1)` again (0x42913A). `Tx = O.x + P2.x` (0x42913F), `Ty = O.y + P2.y` (0x429148), kept in x87.
  4. 2-D heading `H` from the forward row, matrix read without null check (0x429145/0x429152):
     `n = sqrt(fy*fy + fx*fx)` (0x42915C-0x42916A); if `n == 0.0f` (0x42916C, `test ah,0x44; jnp`, NaN does NOT take it)
     `H = (1.0f, fy)`; else `inv = 1.0f / n` (`fdivr` 0x429179), `H.x = inv * fx` (0x429181), `H.y = inv * fy` (0x429187), both f32.
  5. `aT = GetATanOfXY(Tx - V.x, Ty - V.y)` (V = matrix pos, or placement pos when matrix null - dead since step 4 already
     dereferenced it; y operand computed first 0x4291AE, x 0x4291B8), stored f32. `aH = GetATanOfXY(H.x, H.y)` (0x4291D0).
     `steer = aT - aH` (`fsubr` 0x4291D5), kept in x87.
  6. Wrap: `while (steer < -pi) steer += 2pi` (0x4291DC-0x429201, test before first add, NaN exits);
     `while (steer > pi) steer -= 2pi` (0x429203-0x429221, `test ah,0x41`: > continues, <=/NaN exits). No clamping.
  7. `cruise = (float)(u8)m_nCruiseSpeed` (fild of the zero-extended byte, 0x429241, stored f32 0x42924B).
     `spd = sqrt(mv.y*mv.y + mv.x*mv.x)` (0x42924F-0x429261, from a local copy of m_vecMoveSpeed); `ds = cruise - spd * 60.0f`
     (0x429263, `fsubr` 0x429269).
  8. If `ds > 0.0f` or NaN (0x42926D, `test ah,0x41; jp` taken when C0 = C3 (greater or unordered)) -> 0x4292A1:
     `r = ds / cruise` (0x4292A1); if `r > 0.25f` (0x4292A5; `jne` taken for <=/NaN) `*outGas = 1.0f` (0x4292B6), else
     `*outGas = 1.0f - (0.25f - r) * 4.0f` (0x4292BE-0x4292D6: `0.25 - r`, `*4`, `fsubr 1.0`).
     Else (`ds <= 0`): `*outGas = -0.1f` (0x429284); if `ds < -5.0f` (0x42927A, strict) `*outGas = -0.2f` (0x429291).
  9. `*outBrake = 0` (0x4292E6), `*outSteer = steer` (0x4292EC, f32), `*outHandbrake = false` (0x4292EE).
- Edge cases: cruise = 0 with ds > 0 is impossible (ds = -spd*60 <= 0); with NaN ds the accel path is taken and r is NaN,
  so `r > 0.25` fails and outGas = NaN. Steer is not limited to the vehicle's max steer angle here (caller's job).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:933; params named arg2..arg5). Open questions: none.

## 0x429300 CCarCtrl::TriggerDogFightMoves  (catalogue: specified/REA, port level 1)
- Convention / signature: `__cdecl void (CVehicle* plane [esp+4], CVehicle* other [esp+8])`, plain `ret`. Note: the
  argument slot of `plane` is reused as a scratch f32 (`dot`, stored at 0x4293AE `[esp+0x1C]` with edi pushed, read back as
  `[esp+0x18]` after `pop edi`) - harmless for a cdecl caller, but a port must not rely on the arg afterwards.
- Callers: CCarCtrl::GetAIPlaneToAttackPlayer 0x429780 (only). Callees: CVector::Normalise 0x59C910 (thiscall, ECX = &local),
  CRT `rand` 0x821B1E (1 or 2 calls).
- Globals: `CTimer::m_snTimeInMilliseconds` 0xB7CB84 (u32, read). Constants: 0x858CE0 = 70.0 (f32), 0x858B50 = 0.0 (f32),
  0x858B48 = 15.0 (f32), 0x858CB0 = 0.7 (f32, 0.699999988079071). Jump table 0x429510 (4 dwords: 0x42943F, 0x42947E,
  0x429498, 0x4294D3).
- Fields: vehicle +0x390 CAutoPilot; +0x3BB = autopilot+0x2B `m_nTempAction` (i8), +0x3BC = autopilot+0x2C `m_nTempActionTime` (u32);
  entity +0x14 m_matrix (forward +0x10/+0x14/+0x18, pos +0x30), +0x04 placement pos.
- Behaviour:
  1. If `plane->m_autoPilot.m_nTempAction != 0` (0x429308) return.
  2. `P = plane->GetPosition()`, `O = other->GetPosition()` (matrix ? +0x30 : +4, 0x42931B-0x429334).
     `dz = P.z - O.z`, `dy = P.y - O.y`, `dx = P.x - O.x` (x87 in that order 0x429337-0x429345), stored to local d (dx, dy, dz) f32.
     `len = sqrt((dx*dx + dy*dy) + dz*dz)` (dx reloaded from the f32 local, dy/dz unrounded x87; 0x429355-0x429369).
  3. If `!(len < 70.0f)` (0x42936F, `test ah,5; jp`: >= or NaN) return.
  4. `d.Normalise()` (0x429385). `dot = (d.z * Fo.z + d.y * Fo.y) + d.x * Fo.x`, Fo = `other->m_matrix` forward row read WITHOUT a
     null check (0x42938E); stored f32 (0x4293AE). dot > 0 means `plane` is ahead of `other`.
  5. `h = P.z - O.z` recomputed from memory (positions via GetPosition again, 0x4293B2-0x4293C5). If `h < 0.0f` (0x4293C8, C0 set)
     `ah = -(P.z - O.z)` (recomputed, `fchs` 0x4293F1) else (>= 0 or NaN) `ah = P.z - O.z` (0x42940B).
     If `!(ah < 15.0f)` (0x429411, `jp` -> >= or NaN) return.
  6. `r = rand() & 0xFF` (0x429422); `switch (r - 12)`, unsigned `> 3` returns (0x42942F), i.e. only r = 12..15 act
     (4/256 chance per call):
     - r = 12 (0x42943F): if `dot > 0.0f` (`test ah,0x41; jne` -> <=, NaN skip): `m_nTempAction = 16` (TEMPACT_PLANE_FLY_STRAIGHT,
       0x429454), `m_nTempActionTime = (rand() & 0x3FF) + now + 1500` (0x42945B-0x429473, `lea edx,[eax+ecx+0x5DC]`).
     - r = 13 (0x42947E): if `dot > 0.0f`: `m_nTempAction = 17` (TEMPACT_PLANE_SHARP_LEFT, 0x42948F), then the shared tail 0x4294B0:
       `m_nTempActionTime = (rand() & 0x1FF) + now + 700` (`lea edx,[eax+ecx+0x2BC]`).
     - r = 14 (0x429498): if `dot > 0.0f`: `m_nTempAction = 18` (TEMPACT_PLANE_SHARP_RIGHT, 0x4294A9), same tail (+700 + rand&0x1FF).
     - r = 15 (0x4294D3): if `dot > 0.7f` (0x4294D7): `m_nTempAction = 15` (TEMPACT_PLANE_FLY_UP, 0x4294E4),
       `m_nTempActionTime = (rand() & 0x7FF) + now + 3000` (0x4294FB, `lea edx,[eax+ecx+0xBB8]`).
     `now` is read from 0xB7CB84 after the second rand() call; sums are u32 wrapping.
- Edge cases: the first rand() is consumed whenever steps 1-5 pass, even if no move triggers; the second rand() only when a move
  triggers. `other` without a matrix crashes at 0x429391 (planes always have one).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:1088). Open questions: none.

## 0x429520 CCarCtrl::TestWhetherToFirePlaneGuns  (catalogue: specified/REA, port level 1)
- Convention / signature: `__cdecl void (CVehicle* vehicle [esp+4], CEntity* target [esp+8])`, plain `ret` (0x4295F5).
  Entry is a .text detour: `sub esp,0xC; push esi; esi = vehicle` (0x429520-0x429524), `nop; jmp 0x401F1A`, which does
  `dl = [esi+0x42C]; jmp 0x42952E` - same behaviour as a straight body.
- Callers: CCarCtrl::GetAIPlaneToAttackPlayer 0x429780, CCarCtrl::GetAIPlaneToDoDogFight 0x429890,
  CCarCtrl::GetAIHeliToAttackPlayer 0x42F3C0. Callees: CVector::Normalise 0x59C910 (thiscall, ECX = &local vector).
- Globals: none. Constants: 0x858A28 = 150.0 (f32), 0x858C98 = 0.8 (f32, 0.800000011920929).
- Fields: vehicle +0x42C bit 4 (0x10) = `vehicleFlags.bFireGun` (first byte of m_nVehicleLowerFlags, gr union at 0x428);
  vehicle +0x513 u8 `m_nVehicleWeaponInUse` (eCarWeapon: 0 NOT_USED, 1 HEAVY_GUN); entity +0x14 m_matrix
  (pos +0x30, forward row +0x10/+0x14/+0x18), +0x04 placement pos.
- Behaviour:
  1. `vehicle->bFireGun = 0` always (byte `and 0xEF`, stored at 0x429539, before any test).
  2. If `m_nVehicleWeaponInUse` (0x42952E) is neither 0 nor 1 (0x429537/0x429541) return.
  3. If `target == nullptr` (0x42954D) return.
  4. `V = vehicle->GetPosition()` (matrix ? matrix+0x30 : +4, 0x429555-0x42955F), `T = target->GetPosition()` (same,
     0x429562-0x42956E). x87: `dz = T.z - V.z` (0x429571), `dy = T.y - V.y` (0x429577), `dx = T.x - V.x` (0x42957D);
     stored to a local CVector d = (dx [esp+4], dy [esp+8], dz [esp+0xC]) as f32.
     `len = sqrt((dx*dx + dy*dy) + dz*dz)` where dx comes from the stored f32 local but dy and dz are the unrounded x87
     values (0x42958F-0x4295A3).
  5. `fcomp len, 150.0` (0x4295A9) + `test ah,5; jp` : return when `len >= 150.0` or NaN; continue only if `len < 150.0`.
  6. `d.Normalise()` (0x4295BA). `dot = (d.z * F.z + d.y * F.y) + d.x * F.x` with F = vehicle matrix forward (+0x10..+0x18),
     reading `vehicle->m_matrix` WITHOUT a null check (0x4295C3) (sum order z, y, x: 0x4295BF-0x4295DB).
  7. If `dot > 0.8f` (0x4295DD, `test ah,0x41; jne` : equal / less / NaN skip) set `bFireGun = 1` (`or byte [esi+0x42C],0x10`, 0x4295EA).
- Edge cases: if the vehicle has no matrix, step 4 uses placement pos but step 6 dereferences a null matrix (crash);
  in practice planes/helis always have a matrix. The flag is cleared even when the weapon type rejects (step 1 precedes step 2).
- gta-reversed: `plugin::Call` only (CarCtrl.cpp:1078). Open questions: none.

