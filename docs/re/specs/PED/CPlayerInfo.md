# CPlayerInfo (PED) - full specs

Written by RE-SPEC-2 group F from the 1.0 US disassembly (instruction addresses cited). Layout names follow gta-reversed / plugin-sdk headers where they exist; x87 notes assume the in-game 24-bit precision control.

## 0x45B110 CPlayerInfo::~CPlayerInfo  (catalogue: specified/REA, port level 11)
- Convention: `__thiscall void()`. Body: `add ecx, 4; jmp CPlayerPedData::Destructor 0x571840` - destroys the embedded
  `m_PlayerData` (+0x4); nothing else. Callers: `__init_CWorld__Players` 0x84E980 (as the eh-vector element dtor),
  `__dtor_CReplay__PlayerInfo` 0x856160, `__dtor_CWorld__Players` 0x856500.

## 0x56DBD0 CPlayerInfo::FindObjectToSteal  (catalogue: specified/REA, gr `.Reversed = false`, port level 1)
- **Convention**: `__thiscall CEntity*(CPed* ped [esp+4])`, `ret 4`; ECX (= the CPlayerInfo, set by the caller at 0x5704E2) is never read.
  gr declares it `static` and calls it with `plugin::CallAndReturn<CEntity*, 0x56DBD0, CPed*>` (cdecl, caller cleans) - that is a
  convention mismatch (the callee also pops 4 bytes); a port must treat it as a callee-cleaned function with one argument (ECX unused).
  Caller: CPlayerInfo::Process 0x56F8D0.
- Callees: CRT `floor` 0x8219F0 (double -> ST0; x87 fallback control word 0x173F = round down), `_ftol2` 0x821B40, CWorld::ClearScanCodes 0x563470.
- Behaviour:
  1. `m = ped->m_matrix` (read and dereferenced before any null test; the null test only selects the position source).
     `a = fwd.x * 0.5f; b = fwd.y * 0.5f; c = fwd.z * 0.5f` (c stored f32); `Q = pedPos + (a*3.0f, b*3.0f, c*3.0f)` (Q.x = a*3 + pos.x, ...;
     pedPos = matrix pos or placement; Q stored f32) (0x56DBD7-0x56DC46). Constants 0.5f 0x858B8C, 3.0f 0x858B3C.
  2. `minX = Q.x - 3`, `maxX = Q.x + 3`, `minY = Q.y - 3`, `maxY = Q.y + 3` (f32 locals).
  3. Sector bounds with `S(v) = v * 0.02f + 60.0f` (stored f32 to a local, then `_ftol2(floor((double)S))`):
     `x0 = ftol(floor(S(minX))) > 0 ? ftol(floor(local)) : 0`; `y0` likewise from minY; `x1 = ftol(floor(S(maxX))) < 119 ? ftol(floor(local)) : 119`;
     `y1` likewise from maxY (the second floor is recomputed from the f32-stored value).
  4. `CWorld::ms_nCurrentScanCode` (u16, 0xB7CD78): if `>= 0xFFFF` -> `ClearScanCodes()`, code = 1; else code+1; store.
  5. `result = null`. If `y0 > y1` (signed) return null. For y = y0..y1, x = x0..x1 (inclusive, x restarts at x0 each row):
     list = `CWorld::ms_aRepeatSectors[(y & 15)*16 + (x & 15)]` object list (dword at 0xB992C0 + idx*0xC); for each node (item +0, next +4,
     next read before processing): `e = item`; `d = e.pos - Q` (three f32 stores); require `e->m_nObjectFlags (+0x140) & 0x2000` (bIsLiftable)
     and `e->m_nScanCode (+0x2C, u16) != code` (**the scan code is never written**, so the test never filters anything after the bump);
     require `(dz*dz + dy*dy) + dx*dx < 4.5f` (0x863214; NaN fails);
     `side = fabs((dz*m.right.z + dy*m.right.y) + dx*m.right.x) * 3.0f`; `fw = (dz*m.fwd.z + dy*m.fwd.y) + dx*m.fwd.x`;
     `if (fw < 0) fw = 10.0f - fw` (0x85862C; NaN keeps fw); if `side + fw < 1000.0f`: `result = e` (**no best-score tracking: the last
     qualifying object in scan order wins**) (0x56DE86-0x56DF01).
  6. Return result (EAX).

## 0x56F4E0 CPlayerInfo::FindClosestCarSectorList  (catalogue: specified/REA, gr `.Reversed = false`, port level 10)
- Convention: `__thiscall void(CPtrListDoubleLink* list [esp+4], CPed* ped [esp+8], float minX, float minY, float maxX, float maxY
  [esp+0xC..0x18] (**all four unused**), float* outVehDist [esp+0x1C], CVehicle** outVehicle [esp+0x20])`, `ret 0x20`.
  The list and ped argument slots are reused as scratch (current distance / next node). Caller: CPlayerInfo::Process 0x56F8D0.
- Behaviour: if `list->first == null` return. For each node (item +0, next +4; next saved before processing), `e = item`:
  1. Skip if `e->m_nScanCode (+0x2C) == CWorld::ms_nCurrentScanCode` (u16, read once per node), if `!(e->m_nFlags (+0x1C) & 1)` (uses collision),
     or if e is not a vehicle (type bits 2). Then `e->m_nScanCode = code`. Skip if status (`byte[+0x36] >> 3`) is 5 or 6.
  2. Ensure `e->m_matrix` (AllocateMatrix + UpdateMatrix from placement if null). Unless `m.up.z (+0x28) > 0.3f` (0x858C24), require
     `m_nVehicleClass (+0x590) == 9` (bike) else skip.
  3. `e->GetBoundCentre(&c)` (0x534250) is called but `c` is then overwritten with e's position; `c.z = (c.z - e->GetHeightAboveRoad()) + 1.0f`
     (virtual slot +0xD4). `zd = (dy + dx) * 0.0f + dz` with `dk = ped.k - c.k` (x87; stored f32) (0x56F5FF-0x56F61F).
  4. Height gate: if `e->m_nModelIndex == 577` (AT400) and `CCarEnterExit::GetNearestCarDoor(ped, e, &door, &doorId=0)` (0x6528F0, cdecl):
     `zd = |ped.z - door.z|` (stored); if `zd < 1.0f` -> accept. Then (and for every other model) if `|zd| < 2.0f` -> accept. Else only boats
     (`+0x590 == 5`) continue: accept if `c.z < ped.z` and `ped.z - 4.0f < c.z`; else accept if `ped->m_pContactEntity (+0x584) == e`; else skip.
  5. Accepted: skip unless `e` vehicle flags byte +0x42E bit 5 (bConsideredByPlayer); skip unless `GetNearestCarDoor(ped, e, &door, &doorId=0)`.
     `d = CVector(ped.x - e.x, ped.y - e.y, 0).Magnitude()` (0x4082C0; components stored f32), stored. If `d > 10.0f` and
     `CPedGeometryAnalyser::LiesInsideBoundingBox(ped, pedPos, e)` (0x5F3880): `d = 10.0f`; else if NOT `d <= 10.0f` (>, NaN) skip.
  6. Skip if `CCranes::IsThisCarBeingCarriedByAnyCrane(e)`. Else `this->EvaluateCarPosition(e, ped, d, outVehDist, outVehicle)` (0x56DAD0).

## 0x56F8D0 CPlayerInfo::Process  (catalogue: specified/REA, gr plugin wrapper, port level 25)
Instruction-level spec (2086 instructions; extents 0x56F8D0-0x5717C0 plus obfuscation chunks 0x401876 `al = this->m_bAfterRemoteVehicleExplosion;
jmp 0x570AAB` and 0x40A447 `ecx = CWorld::PlayerInFocus; jmp 0x571317`). Locals `F+x` = `[esp+x]` after `push edi` (0x56F8F2).
- Convention: `__thiscall void(uint32 playerIndex [F+0x68])`, `ret 4`; ESI = this (CPlayerInfo*). SEH frame (handler 0x83CDD5, state F+0x60 for
  the `new`ed tasks). Caller: CWorld::Process 0x5684A0. EBP = 0 and EBX = 0x100 (bInVehicle mask) in the first half; from 0x570C0B on
  EBX = 0, EBP = 0x100.
- CPlayerInfo fields (gr layout, size 0x190): +0x0 m_pPed, +0xB0 m_pRemoteVehicle, +0xB8 m_nMoney, +0xBC m_nDisplayMoney, +0xCC m_nTaxiTimer,
  +0xD4 m_bTaxiTimerScore, +0xD5 m_bTryingToExitCar, +0xDC m_nPlayerState (low byte tested), +0xDD m_bAfterRemoteVehicleExplosion,
  +0xDF m_bFadeAfterRemoteVehicleExplosion, +0xE0 m_nTimeOfRemoteVehicleExplosion, +0xF0 m_nTimesUpsideDownInARow, +0xF8 m_nCarTwoWheelCounter,
  +0xFC m_fCarTwoWheelDist, +0x100 m_nCarLess3WheelCounter, +0x104 m_nBikeRearWheelCounter, +0x108 m_fBikeRearWheelDist, +0x10C m_nBikeFrontWheelCounter,
  +0x110 m_fBikeFrontWheelDist, +0x114 m_nTempBufferCounter, +0x118/+0x11C best two-wheels time/dist, +0x120/+0x124 best wheelie, +0x128/+0x12C best
  stoppie, +0x134 m_fRoadDensityAroundPlayer, +0x148 m_fCurrentChaseValue, +0x14F m_nMaxHealth, +0x158 crosshair struct (u8 active, CVector2D),
  +0x188 m_bParachuteReferenced, +0x18C m_nRequireParachuteTimer.
- Other fields: ped +0x46C bit 8 bInVehicle, +0x474 0x4000000 (gr: bCanExitCar), +0x478 0x2 (gr count: bStuckUnderCar, unverified), +0x470 |= 0x80
  (bTryingToReachDryLand), +0x47C intelligence (task manager +4: primary slots +0x10 PRIMARY, +0x14 DEFAULT), +0x480 m_pPlayerData (first dword
  CWanted*, wanted level at +0x2C), +0x544 m_fMaxHealth, +0x58C m_pVehicle, +0xD4 m_fMovingSpeed, +0xFC m_pAttachedTo. Vehicle: +0x22 model,
  +0x36 type/status byte, +0x40 physical flags (0x100 bSubmergedInWater), +0x42A bit 5 (gr count: bCanBeDamaged), +0x460 driver, +0x484 u8 passengers,
  +0x48C entity under the vehicle (model passed to the bridge test), +0x4C0 m_fHealth, +0x4C4 trailer's tractor, +0x4F8 door lock, +0x590 class,
  +0x594 subtype, +0x5A0 CAutomobile damage manager, +0x7E4/7E8/7EC/7F0 CAutomobile wheel compression [FL, RL, FR, RR] (1.0f = wheel in the air),
  +0x928 CAutomobile dword, +0x960 u8 CAutomobile wheels on ground, CBike +0x720/724/728/72C wheel compression (0x720/724 = front pair, 0x728/72C = rear pair, inferred from the wheelie/stoppie stat ids).
- `dt` below = `_ftol2(ms_fTimeStep * 0.02f * 1000.0f)` (0x858B38 0.02f, 0x858C4C 1000.0f; `_ftol2` 0x821B40 truncates). `FPV()` = the inlined
  `Players[PlayerInFocus].m_pPed && (ped.0x46C & 0x100) ? ped.m_pVehicle : null` (re-evaluated at every use; several sites dereference it
  without testing for null, see oddities). Players = 0xB7CD98, stride 0x190.

### Block 0 - entry and taxi fare (0x56F8D0-0x56F9A2)
If `CReplay::Mode (0xA43088) == 1` return. `pad = CPad::GetPad(playerIndex)` (F+0x24). If `m_bTaxiTimerScore && ped.bInVehicle &&
veh.model in {420, 438} && veh.driver == ped && veh.+0x484 != 0`: `e = time - m_nTaxiTimer` (unsigned); if `e >= 1000`: `s = e / 1000;
m_nMoney += s; m_nTaxiTimer += s * 1000`. Otherwise (gate false): `m_nTaxiTimer = time`.

### Block 1 - stunt counters (0x56F9A2-0x5700E8)
If `!ped.bInVehicle || !veh` -> Z0. Comparisons "== 1.0f" use `test ah,0x44; jp` (NaN counts as not-equal); "< 1.0f" use `test ah,5; jp/jnp`.
**Automobile** (`veh+0x590 == 0`): `+0x100 = (veh+0x960 < 3) ? +0x100 + dt : 0`.
- Right side up (`w[FR] == 1 && w[RR] == 1`): if `w[FL] < 1 && w[RL] < 1 && veh.m_fDamageIntensity(+0xD8) == 0` -> ACC_CAR, else END_CAR.
- Else left side up (`w[FL] == 1 && w[RL] == 1`): if `w[FR] < 1 && w[RR] < 1 && dmgInt == 0` -> ACC_CAR, else END_CAR.
- Else (no side up, 0x56FCA1): if `+0xF8 == 0` -> Z2; if `+0xF8 < 2000` -> R_CAR; else REC_CAR.
- ACC_CAR: `+0xF8 += dt; +0xFC = veh.m_fMovingSpeed + +0xFC; tb = (float)(uint32)+0x114 - (float)(uint32)dt * 0.5f; if (0.0f > tb) tb = 0;
  +0x114 = _ftol2(tb)` (dt computed a second time for this) -> Z2.
- END_CAR: if `+0xF8 > 0 && +0x114 < 500`: `+0x114 = +0x114 - _ftol2(ts * 0.02f * -1000.0f)` (0x859948 -1000.0f) -> Z2. Else if `+0xF8 < 2000` -> R_CAR,
  else REC_CAR.
- REC_CAR: `+0x11C = +0xFC; +0x118 = +0xF8; SetNewRecordStat(19, (float)(+0xF8 / 1000)); SetNewRecordStat(12, +0xFC)` (cdecl (u16 stat, float)).
  No comparison with the old best - the "best" fields take the latest qualifying run. R_CAR: `+0xFC = 0` -> Z1.
**Bike** (`+0x590 == 9`; other classes -> Z0):
- Wheelie (`w720 == 1 && w724 == 1 && +0x10C == 0`): accumulate if `w728 < 1 || (w72C < 1 && dmgInt == 0)` (no damage test for the first
  operand) -> ACC_REAR; else END_REAR.
- Else (0x56FEAC): if `+0x104 > 0`: record if `+0x104 >= 5000` (REC_REAR), then `+0x104 = +0x108 = 0` -> Z3. If `+0x104 == 0`:
  stoppie (`w728 == 1 && w72C == 1`): accumulate if `w720 < 1 || (w724 < 1 && dmgInt == 0)` -> ACC_FRONT; else END_FRONT.
  No stoppie: if `+0x10C >= 2000` REC_FRONT; -> R_FRONT.
- ACC_REAR: `+0x104 += dt; +0x108 = veh.0xD4 + +0x108`; ACC_FRONT: `+0x10C += dt; +0x110 = veh.0xD4 + +0x110`; both then
  `tb = (float)(uint32)+0x114 - (float)(uint32)dt * 0.2f` (0x858CC4), clamp at 0, `+0x114 = _ftol2(tb)`, `+0xF8 = +0x100 = 0` -> Z4.
- END_REAR: if `+0x104 > 0 && +0x114 < 500` -> DEC; else `+0x104 >= 5000` ? REC_REAR : nothing; then `+0x104 = +0x108 = 0` -> Z3.
  END_FRONT: if `+0x10C > 0 && +0x114 < 500` -> DEC; else `+0x10C >= 2000` ? REC_FRONT; -> R_FRONT.
  DEC (0x56FF92): `+0x114 = +0x114 - _ftol2(ts * 0.02f * -1000.0f); +0xF8 = +0x100 = 0` -> Z4.
- REC_REAR: `+0x124 = +0x108; +0x120 = +0x104; SetNewRecordStat(17, (float)(+0x104/1000)); SetNewRecordStat(10, +0x108)`.
  REC_FRONT: `+0x12C = +0x110; +0x128 = +0x10C; SetNewRecordStat(18, ...); SetNewRecordStat(11, +0x110)`.
- Exits: R_FRONT 0x5700B0 `+0x10C = +0x110 = 0`, then Z3 0x5700BC `+0x114 = 0; +0xF8 = 0; +0x100 = 0` -> Z4. Z0 0x5700D0 `+0x100 = 0`, Z1 0x5700D6
  `+0x114 = 0; +0xF8 = 0`, Z2 0x5700E2 `+0x104 = 0; +0x10C = 0`, Z4 = 0x5700EE.

### Block 2 - hunger, money display, health, parachute (0x5700EE-0x5701DA)
`WorkOutEnergyFromHunger()`. If `m_nDisplayMoney != m_nMoney`: `diff = money - display; a = |diff|` (signed); `step = a > 100000 ? 12345 :
a > 10000 ? 1234 : a > 1000 ? 123 : a > 50 ? 42 : 1`; `display += diff >= 0 ? step : -step`.
`ped.m_fMaxHealth = CStats::GetFatAndMuscleModifier(9)` (cdecl, ST0); `m_nMaxHealth = (uint8)_ftol2(ped.m_fMaxHealth)`.
If ped.bInVehicle and `!(CStats::GetStatValue(223) < 400.0f)` (NaN passes): `StreamParachuteWeapon(true)`. Else if `m_bParachuteReferenced`:
`CGameLogic::IsCoopGameGoingOn()` (result ignored); if still referenced: `CStreaming::SetModelIsDeletable(371)`, `+0x188 = 0`, `+0x18C = 0`.

### Block 3 - road density (0x5701E0-0x5702AE)
Every 16th frame (`CTimer::m_FrameCounter & 0xF == 0`): pos = vehicle pos if ped.bInVehicle (m_pVehicle not null-checked) else ped pos;
`+0x134 = ThePaths.CalcRoadDensity(pos.x, pos.y)`. **Every frame**: `d = (+0x134 - 1.0f) * 0.6f + 1.0f; if (0.5f > d) d = 0.5f;
if (1.45f < d) d = 1.45f; +0x134 = d` - the remap is re-applied to the stored value on the 15 frames without a recompute, so the value decays
towards 1.0 between samples (keep).

### Block 4 - leave vehicle / steal object (0x5702AA-0x5705F0)
Gate: `!pad->GetTarget() && ped.0x474 & 0x4000000 && !(ped.0x478 & 2)` and (`pad->ExitVehicleJustDown()` or (`m_bTryingToExitCar &&
pad->GetExitVehicle() && ped.bInVehicle`)); if the gate fails -> 0x570A9E (`m_bTryingToExitCar = 0`). Otherwise `m_bTryingToExitCar = 0`, then:
- In vehicle: return-to-tail (0x570AA5) if `m_pRemoteVehicle`, or `veh+0x48C && CBridge::ThisIsABridgeObjectMovingUp(model)` (always false),
  or status (`+0x36 >> 3`) is 5 or 6, or `veh+0x4F8 == 4`. `t = GetActiveTask()`; if t and `!t->MakeAbortable(ped, 1, null)` (vfunc +0x18):
  `m_bTryingToExitCar = 1` -> tail. If boat (`+0x590 == 5`): `SetTask(new CTaskComplexLeaveCar(veh, 0, 0, true, false) (0x34 bytes), 3, false)`;
  `ped.0x470 |= 0x80` -> tail. Else if `veh->CanPedStepOutCar(false)` (vfunc +0xE4, 0x6D1F30) or `veh->CanPedJumpOutCar(ped)` (+0xE8, 0x6D2030):
  same LeaveCar task + SetTask, `m_bTryingToExitCar = 1`, `GivePlayerParachute()` -> tail. Else if `(veh+0x36 & 0xF8) == 0` (STATUS_PLAYER):
  `m_bTryingToExitCar = 1`. -> tail.
- On foot (0x5704D5): tail if `ped.m_pAttachedTo`. `obj = FindObjectToSteal(ped)`; `s = GetSimplestActiveTask()` (dereferenced without a null
  check); `hold = GetTaskHold(false)`; tail if `s->GetTaskType() == 254` (TASK_SIMPLE_CLIMB), or `GetTaskFighting()`, or `hold && hold+8`.
  If obj: `t = GetActiveTask()`; tail if t and !MakeAbortable(ped, 1, null); `task = new CTaskComplexGoPickUpEntity(obj, 0x51)` (0x34 bytes, ctor
  0x6919C0, see below); `CEventScriptCommand ev(3, task, false)` (0x4B0A00, on the stack at F+0x34); `Players[PlayerInFocus].m_pPed->intel.eventGroup
  .Add(&ev, false)` (note: PlayerInFocus's ped, not this->m_pPed); `~CEventScriptCommand` (0x4B0A50) -> tail. Else -> block 5.

### Block 5 - nearest car (0x5705F5-0x57074F)
`F+0x10 veh = null; F+0x2C dist = 0.0f`; `minX = pos.x - 10.0f` (F+0x20), `maxX = pos.x + 10.0f` (F+0x18), `minY = pos.y - 10.0f` (F+0x1C),
`maxY = pos.y + 10.0f` (F+0x14) (ped pos). Sector indices `_ftol2(floor((double)(v * 0.02f + 60.0f)))` (0x406CE0 = cdecl floor((double)f) -> ST0,
CRT floor 0x8219F0): x0 (minX), y0 (minY), x1 (maxX), y1 (maxY). `SetNextScanCode()`. `for (y = y0; y <= y1; y++) for (x = x0; x <= x1; x++)
FindClosestCarSectorList(*GetRepeatSector(x, y) /*vehicle list*/, ped, minX, minY, maxX, maxY, &dist, &veh)` (GetRepeatSector wraps with `& 15`,
stride 0xC, base 0xB992B8; no index clamping).

### Block 6 - enter car (0x570751-0x570A9E)
Tail if !veh. If `veh+0x594 == 11 && veh+0x4C4`: veh = tractor. Tail if `!veh->CanBeDriven()`. If `GetTaskJetPack()`: `GetTaskJetPack()->DropJetPack(ped)`.
- Boat (`+0x590 == 5`, 0x570A43): tail if `veh.driver`; t = GetActiveTask(); tail if t && !MakeAbortable(ped, 1, null);
  `SetTask(new CTaskComplexEnterCarAsDriver(veh) (0x50 bytes), 3, false)`.
- Else `t = GetActiveTask()`; if t: if `t->GetTaskType() == 268` (TASK_COMPLEX_IN_WATER) and `GetTaskSwim()` and model in {460, 539, 447, 417}
  and `CCarEnterExit::GetNearestCarDoor(ped, veh, &F+0x34, &F+0x30 /*=0*/)`: `swim+0x14 = F+0x34 (CVector); swim+0x58 = 5000` (no abort).
  Otherwise tail if `!t->MakeAbortable(ped, 1, null)`.
- Co-op (`Players[1].m_pPed (0xB7CF28) && !CGameLogic::bPlayersCanBeInSeparateCars (0x96A8B3)`): `o = Players[(playerIndex + 1) % 2]` (signed mod);
  `tp = o.ped.intel taskMgr primary[3]` (+0x10), `td = primary[4]` (+0x14); `typeD = td ? td->GetTaskType() : 0`; `typeP = tp ? tp->GetTaskType() : 0`;
  `drv = (typeP in {701, 702, 703, 715}) ? tp+0xC : null`; if `typeD == 1 || typeP == 1 || typeD == 709 || typeP == 709`: `v = o.ped.m_pVehicle`;
  if `v.driver == o.ped` then `drv = v, pas = null` else `pas = v`; else `pas = null`; if `typeP == 700`: `pas = tp+0xC`.
  If `drv == veh`: `SetTask(new CTaskComplexEnterCarAsPassenger(veh, 0, false), 3, false)`; else if `pas == veh || (drv == null && pas == null)`:
  `SetTask(new CTaskComplexEnterCarAsDriver(veh), 3, false)`; else nothing.
- Single player: `SetTask(new CTaskComplexEnterCarAsDriver(veh), 3, false)`.
`new` returning null passes a null task to SetTask (as compiled).

### Block 7 - remote-vehicle explosion fade (0x570AAB-0x570C07)
If `m_bAfterRemoteVehicleExplosion`: `ep = prevTime - +0xE0; en = time - +0xE0` (unsigned). If `ep < 1000 && en >= 1000 && state == 0 && +0xDF`:
`TheCamera.SetFadeColour(0,0,0); TheCamera.Fade(1.0f, 0)`. If `en > 2000`: if `state == 0 && +0xDF`: `RestoreWithJumpCut(); SetFadeColour(0,0,0);
Fade(1.0f, 1); CCamera::Process(); CTimer::Stop(); CRenderer::RequestObjectsInFrustum(0, 0); CStreaming::LoadAllRequestedModels(false);
CTimer::Update()`; then `+0xDD = 0`; `r = Players[PIF].m_pRemoteVehicle; if r: r.m_nFlags |= 0x800; Players[PIF].m_pRemoteVehicle = 0`;
if FPV(): `FPV().+0x36 &= 7` (status = STATUS_PLAYER; the null branch 0x570C01 would write to address 0x36 but is unreachable).

### Block 8 - upside-down counter (0x570C10-0x570F13), every 32nd frame (`m_FrameCounter & 0x1F == 0`)
If `FPV() && this.ped.bInVehicle && FPV().up.z (+0x28 of matrix) < 0.0f && |FPV().moveSpeed| < 0.05f && FPV().class in {0, 5} &&
!(FPV().+0x40 & 0x100)`: `+0xF0 += (up.z < -0.5f) ? 2 : 1`; else `+0xF0 = 0`. If `+0xF0 > 6` and `FPV().byte+0x42A & 0x20`:
`FPV().health = (health > 249.0f) ? 249.0f : health`; if `FPV().class == 0`: `damageManager(+0x5A0).SetEngineStatus(225)`; `FPV().+0x928 = 0`.
(These FPV() uses after the counter test are not null-checked.)

### Block 9 - driving stats (0x570F19-0x57149A)
If FPV() (else block 10): if `model == 457` (caddy): `IncrementStat(7, FPV().m_fMovingSpeed)`; else if `subtype == 10` (BMX): `IncrementStat(27, speed)`;
else by `a = GetVehicleAppearance()` (re-called each time): a==3 -> stat 8, a==5 -> 9, a==1 -> 4, a==2 -> 5, a==4 -> 6 (value = moving speed); then
if a is 5 or 3 and `|moveSpeed| > 0.2f`: `IncrementStat(169, ts * 16.0f)`. Then (all three cases) if `FindPlayerTrain(-1) == null` and
`(v.x*v.x + v.y*v.y) + v.z*v.z > 0.0f` (v = FPV().moveSpeed): switch `a - 2` (table 0x5717C0): bike (2) -> `UpdateStatsWhenOnMotorBike(FPV())`
unless subtype 10; heli (3) / plane (5) -> `UpdateStatsWhenFlying(FPV())`; boat (4) -> nothing; other -> `UpdateStatsWhenDriving(FPV())`.

### Block 10 - on-foot stats (0x57149F-0x571564)
`p = Players[PIF].m_pPed` (no null check); if `p.m_fMovingSpeed > 0.0f`: if this ped `GetTaskSwim()`: if `GetPad(0)->GetPedWalkLeftRight() != 0 ||
GetPad(0)->GetAccelerate() != 0`: `IncrementStat(26, p.movingSpeed)`; else if `GetTaskJetPack()`: `IncrementStat(173, ts * 16.0f)`; else
`IncrementStat(3, p.movingSpeed)`. (Pad 0 is hard-coded.)

### Block 11 - chase value (0x571567-0x571760)
`w = ped.m_pPlayerData ? *(CWanted**)m_pPlayerData : null` (w deref'd unchecked). If `w.level(+0x2C) == 0 || IsPlayerOnAMission()`: `+0x148 = 0`
-> block 12. Function statics: init byte 0xB9B9A0 bit 0, saved coords 0xB9B994/98/9C, flags 0x8CDF21 (moved) and 0x8CDF20 (near node).
If `prevTime / 10000 != time / 10000`: `p = FindPlayerCoors(-1)`; `0x8CDF21 = 0`; `d = sqrt((dz*dz + dy*dy) + dx*dx)` (d? = saved - p);
if `!(d < 10.0f)`: `0x8CDF21 = 1`; saved = FindPlayerCoors(-1); `ThePaths.FindNodeClosestToCoors(&node, FindPlayerCoors(-1), 0, 60.0f, 1, 0, 0, 0, 0)`;
`0x8CDF20 = (node.word0 != 0xFFFF)`. `target = {31, 62, 125, 250, 500, 1000}[level - 1]` (table 0x5717D0; level outside 1..6 -> 0.0f).
`delta = (target - +0x148) * ts * 0.0001f`; if `delta < 0.0f`, or (`0x8CDF21 && 0x8CDF20 && !CCullZones::NoPolice() && !CCullZones::PoliceAbandonCars()
&& CGame::currArea == 0`): `+0x148 = delta + +0x148`.

### Block 12 - crosshair and money clamp (0x571766-0x5717BD)
`CPlayerCrossHair::Process(this+0x158, playerIndex, pad)` (0x56EC80, see below). `m_nMoney = min(m_nMoney, 999999999)`, same for m_nDisplayMoney
(signed `jl`, so negative values pass).

### Helper 0x56EC80 (proposed CPlayerCrossHair::Process; thiscall(this = CPlayerInfo+0x158, int playerIndex, CPad* pad), `ret 8`)
If the active byte (+0) is set: `x += steerLR * ts * 0.00033333333f` (0x865030); y: `+= steerUD * ts * k` when byte 0xB73402 is set, else `-=`;
clamp x and y to [-0.9f, 0.9f]. If `pad->GetCarGunFired()`: `CamShakeNoPos(&TheCamera, 0.2f)`; builds a ray from TheCamera's matrix and FOV
(`tan(FindCamFOV() * 0.0087266462f)` scaled by the crosshair, `/ CDraw::ms_fAspectRatio`, length 200.0f) and fires an instant hit with a
stack `CWeapon(31, 5000)` from `Players[playerIndex].m_pPed` (temporarily clearing ped flag 0x470 bit 25 around `CWeapon::FireInstantHit`).
Only its interface is specified here; it needs its own spec before porting.

### Helper 0x6919C0 = CTaskComplexGoPickUpEntity::CTaskComplexGoPickUpEntity(CEntity* entity, int32 arg2), thiscall, `ret 8`
Calls CTaskComplex ctor 0x61A3B0; vtable 0x870B98 (GetTaskType 0x691A40 returns 310 = TASK_COMPLEX_GO_PICKUP_ENTITY); `+0xC = entity`
(RegisterReference if non-null); `+0x10 = +0x1C = -999.0f`; `+0x14 = +0x18 = +0x20 = +0x24 = 0`; `+0x28 = 0`; `+0x2C = arg2` (0x51 here);
`+0x30 (u8) = 0`. Size allocated by the caller: 0x34.

- Oddities to keep: road-density remap applied every frame (block 3); "best" stunt fields overwritten by the latest qualifying run; bike
  accumulation clears the car counters; unchecked dereferences of `GetSimplestActiveTask()`, `m_pPlayerData`, and FPV() after its first test;
  the steal-object event goes to PlayerInFocus's ped.
- gta-reversed: plugin wrapper only (PlayerInfo.cpp:132, `.Reversed = false`); no C++ to compare.
- Open questions: semantics of byte 0xB73402 (inverted look flag?) in 0x56EC80; meaning of CAutomobile +0x928 (zeroed at 0x570F13); stat ids
  are numeric (names not verified against gr eStats).

## 0x5D24B0 CPlayerInfoSaveStructure::Construct (proposed; catalogue: unnamed, unknown/REA, port level 0)
- Name evidence: only caller CPlayerInfo::Save 0x5D3AC0; the copy matches gr `CPlayerInfoSaveStructure::Construct(CPlayerInfo*)`
  (`PlayerInfo.h`, size 0x28) field for field.
- Convention: `__thiscall void(CPlayerInfo* info [esp+4])`, ECX = save structure, `ret 4`. Clobbers EDX, EAX (=info, then low word).
- Behaviour (exact order, save-struct offset <- CPlayerInfo offset):
  +0x00 d <- +0xB8 (m_nMoney); +0x04 w <- +0x130 (m_nCarDensityForCurrentZone); +0x06 b <- +0xDC (m_nPlayerState, u8);
  +0x08 d <- +0x134 (m_fRoadDensityAroundPlayer, raw copy); +0x0C d <- +0xBC (m_nDisplayMoney); +0x10 b <- low byte of +0x144
  (m_nNumHoursDidntEat u16 truncated to u8); +0x14 d <- +0xC0 (m_nCollectablesPickedUp); +0x18 d <- +0xC4 (m_nTotalNumCollectables);
  +0x1C..+0x24 b <- +0x14C..+0x154 (9 bytes, one by one: DoesNotGetTired, FastReload, FireProof, MaxHealth, MaxArmour,
  GetOutOfJailFree, FreeHealthCare, CanDoDriveBy, BustedAudioStatus); +0x26 w <- +0x156 (m_nLastBustMessageNumber).
  (0x5D24B4-0x5D2555.) Padding bytes +0x07, +0x11..+0x13, +0x25 are not written (stack garbage is saved as-is by the caller).
- gta-reversed: same fields and order. Note the padding garbage: the saved block contains uninitialised bytes in the original.

## 0x5D2560 CPlayerInfoSaveStructure::Extract (proposed; catalogue: unnamed, unknown/REA, port level 0)
- Name evidence: only caller CPlayerInfo::Load 0x5D3B00; matches gr `CPlayerInfoSaveStructure::Extract(CPlayerInfo*)`.
- Convention: `__thiscall void(CPlayerInfo* info [esp+4])`, ECX = save structure, `ret 4`.
- Behaviour (order of stores into info): +0xB8 d <- +0x00; +0x130 w <- +0x04; +0xDC b <- +0x06; +0x130 w <- +0x04 again
  (0x5D2580, redundant second store, as gr's comment notes); +0x134 d <- +0x08; +0xBC d <- +0x0C;
  +0x144 w <- zero-extended byte +0x10 (`movzx dx, byte`, 0x5D259D); +0xC0 d <- +0x14; +0xC4 d <- +0x18;
  +0x14C..+0x154 b <- +0x1C..+0x24 (9 bytes); +0x156 w <- +0x26.
- gta-reversed: same values (the redundant store is commented out; no observable difference).

## 0x56EC80 CPlayerInfo::sub_56EC80 (proposed: CPlayerCrossHair::Process(int32 playerIndex, CPad* pad))  (catalogue: unknown/REA, port level 20)
(RE-SPEC-FINAL. SA:MP-executed.) The free-aim "crosshair gun" of the player info (gr fields m_nCrosshairActivated / m_vecCrosshairTarget).
- Convention: `__thiscall void (crosshair* this = ECX, int32 playerIndex [esp+4], CPad* pad [esp+8])`, `ret 8`. Only call: CPlayerInfo::Process
  0x571776 with `this = &playerInfo + 0x158` (+0 activated, read as a BYTE; +4 x f32; +8 y f32, both in [-0.9, 0.9]).
- Callees: CPad::GetSteeringLeftRight 0x53FB80, CPad::GetSteeringUpDown 0x53FBD0, CPad::GetCarGunFired 0x53FF90 (thiscall on pad, i16),
  CCamera::CamShakeNoPos 0x50A970 (cdecl (CCamera*, float)), CCamera::FindCamFOV 0x50AD20 (thiscall, ST0), CWeapon::CWeapon(type, ammo)
  0x73B430, CWeapon::FireInstantHit 0x73FB10.
- Globals: CTimer::ms_fTimeStep 0xB7CB5C, CPad::bInvertLook4Pad 0xB73402, CDraw::ms_fAspectRatio 0xC3EFA4, TheCamera 0xB6F028 with its
  camera CMatrix at TheCamera+0x974 (right 0xB6F99C, forward 0xB6F9AC, up 0xB6F9BC, pos 0xB6F9CC), TheCamera.m_matrix (0xB6F03C) /
  placement pos 0xB6F02C, CWorld::Players 0xB7CD98 (stride 0x190). Constants 0.00033333333f [0x865030], 0.9f [0x858C20], -0.9f [0x858CAC],
  0.008726646f [0x8631D4] (PI/360), 200.0f [0x858A48].
- Behaviour:
  1. If `this->byte[0] == 0` return.
  2. `x += ((float)pad->GetSteeringLeftRight() * ts) * 0.00033333333f` (ts = ms_fTimeStep copied before the call; i16 -> fild).
     `y`: if bInvertLook4Pad `y = ((float)ud * ts) * k + y` else `y = y - ((float)ud * ts) * k` (ud = GetSteeringUpDown, ts re-read).
  3. Clamp each of x then y: `if (v > 0.9f) v = 0.9f` (bits 0x3F666666); `if (v < -0.9f) v = -0.9f` (strict).
  4. If `pad->GetCarGunFired() == 0` return.
  5. `CamShakeNoPos(&TheCamera, 0.2f)`.
  6. `tv = tan(FindCamFOV() * PI/360) / ms_fAspectRatio * y` (fptan); `A = camUp * tv` (per component, stored f32).
     `th = tan(FindCamFOV() * PI/360)` (second call); `Rx = (right.x * x) * th`, `Ry = (right.y * x) * th` (inner products stored f32),
     `Rz = (x * right.z) * th` (x87). `D = (forward - R) - A` per component (`forward.c - R.c` stored f32 for y/z, x kept in x87; each D.c
     stored f32).
  7. `end = camPos + D * 200.0f` (x: `D.x*200 + pos.x`, y: `pos.y + D.y*200`, z: `pos.z + D.z*200`).
  8. `w = CWeapon(31 /*0x1F*/, 5000)` (local; no destructor call). `start = TheCamera.m_matrix ? m_matrix->pos : TheCamera placement pos`.
  9. `ped = CWorld::Players[playerIndex].m_pPed`; `saved = (ped->dword[+0x470] >> 25) & 1`; clear bit 25 (`&= 0xFDFFFFFF`);
     `w.FireInstantHit(ped, &start, &start, null, &end, null, true, true)`; then (ped re-read) restore bit 25 of `+0x470` to `saved`.
- gta-reversed: absent (CPlayerInfo has the fields, no function). Open questions: name of ped flag +0x470 bit 25 (the code temporarily
  clears it so the shot is not blocked/attributed; semantics unverified).
