# CTaskSimpleCarDrive (AI) - RE-SPEC-3 group I2

Vtable 0x86E904, type 0x2C5, size 0x60. Layout (gr TaskSimpleCarDrive.h, offsets confirmed by the bytes): +8 m_pVehicle (registered
ref), +0xC m_pAnimCloseDoorRolling, +0x10 m_pTaskUtilityLineUpPedWithCar (heap object 0x1C), +0x14 CTaskTimer {+0x14 start, +0x18
interval, +0x1C started, +0x1D stopped} (gr names +0x14 "m_TaskTimer*" - it is an inline timer), +0x20 m_nBoppingStartTime (-1 = none),
+0x28 m_nBoppingEndTime (beat interval ms), +0x2C m_fBoppingProgress, +0x30 m_nBoppingCompletedTimes, +0x34 m_nHeadBoppingStartTime,
+0x38 m_nHeadBoppingDirection, +0x3C m_fHeadBoppingOrientation, +0x40 m_fRandomHeadBoppingMultiplier, +0x44 m_fHeadBoppingFactor,
+0x48 m_nArmBoppingStartTime, +0x4C m_nTimePassedSinceCarUpSideDown, +0x50 m_copCarStolenTimer (CTaskTimer), +0x5C flags (bit0 head
bopping active, bit1 arm bopping active, bit2 m_bUpdateCurrentVehicle, bit3 "abort requested", bit4 ride anim blended, bit5 closing
door anim running). Ped: +0x18 clump, +0x20 random seed (u16), +0x47C intelligence, +0x484 created-by, +0x49C/+0x4A0/+0x490 bone frame
data (AnimBlendFrameData*, quaternion pointer at +0x10), +0x46C bit 8 bInVehicle, +0x470 bit 14 bUpdateMatricesRequired, +0x478 bit 20,
+0x58C m_pVehicle, +0x5A0+0x1C*4 weapon slot 4 (+0x610 type, +0x61C ammo).

## 0x63C670 CTaskSimpleCarDrive::MakeAbortable  (catalogue: specified/REA, port level 25)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [+8], const CEvent* event [+0xC])`, `ret 0xC`, SEH (stack CTaskSimpleCarSetPedOut).
- priority == 2: if ped bInVehicle and `ped->m_pVehicle`: `door = CCarEnterExit::ComputeTargetDoorToExit(ped->m_pVehicle, ped)` 0x64F110;
  stack `CTaskSimpleCarSetPedOut so(m_pVehicle (+8), door, false)` 0x6478B0; if `ped->m_pVehicle->m_nVehicleType == 9` (bike) and
  event != 0: `so.+0x12 = 1`; `so.ProcessPed(ped)` 0x647D10; `~so` 0x647950. Then if `IsLooking(ped)` `AbortLookAt(ped, 250)`. Return 1.
- other priorities: `flags |= 8`; return `!(flags & 0x20)` (0x63C73F-0x63C75D).
- gr: `.Reversed = false`.

## 0x63C770 CTaskSimpleCarDrive::SetPedPosition  (catalogue: specified/REA, port level 10)
- `__thiscall bool (this, CPed* ped)`, `ret 4`, always returns 1.
- If `flags & 8`: `t = ped->intel->m_TaskMgr.FindTaskByType(4, 0x2C5)` 0x6817D0; if t and t != this and `t->+0x10`: move the line-up
  object (`this->+0x10 = t->+0x10; t->+0x10 = 0`); then `flags &= ~8`.
- If `+0x10` (line-up): `a` = first non-null of `RpAnimBlendClumpGetAssociation(ped->clump, id)` for id in the table at 0x8D2E9C
  (12 ids: 359, 360, 361, 362, 363, 367, 368, 369, 370, 371, 372, 386; null if none); `lineUp->ProcessPed(ped, m_pVehicle, a)` 0x6513A0;
  destroy it (0x64FC00 + `free`), `+0x10 = 0`; return 1.
- Else if m_pVehicle and `ped->m_pVehicle == m_pVehicle` and ped bInVehicle: `ped->SetPedPositionInCar()` 0x5DF910.

## 0x6426E0 CTaskSimpleCarDrive::`scalar deleting destructor'  (catalogue: specified/REA, port level 2)
- call 0x63C460 (complete dtor); `if (flags & 1) CTask::operator delete(this)`; return this; `ret 4`.

## 0x642760 CTaskSimpleCarDrive::StartBopping  (catalogue: specified/REA, port level 1)
- `__thiscall void (this, CPed* ped)`, `ret 4`. A local L lives in the slot of the entry `push ecx` (so it starts as the value of `this`).
- Looks for another occupant already bopping: `start = -1`. Driver `d = ped->m_pVehicle->m_pDriver`: if d and d != ped and
  `t = d TaskMgr.FindActiveTaskByType(0x2C5)`: `start = t->+0x20`, `interval = t->+0x28`, L = interval; if start != -1 -> ADOPT.
  If there is no such task, `interval` takes the value of L (ORIGINAL BUG: initially the `this` pointer value) (0x642873/0x642877).
  Then passengers +0x464, +0x468, +0x46C in order: for p != 0 and p != ped with an active 0x2C5 task: `interval = t->+0x28; start =
  t->+0x20`; if start != -1 -> L = interval, ADOPT.
- NEW (nobody bopping): `+0x20 = now [0xB7CB84]`; `bpm = 60 - _ftol((float)(_rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * -60.0f
  [0x86EBC4])` (60..119); `+0x28 = _ftol(1.0f [0x858624] / ((float)bpm * 0.016666668f [0x859044]) * 1000.0f [0x858C4C])` (fild; fmul;
  fdivr; fmul; _ftol); `+0x2C = 0`.
- ADOPT (0x642884): `+0x20 = start`, `+0x28 = interval`; `q = (now - start) / interval`, `r = (now - start) % interval` (signed idiv;
  interval 0 would fault); `+0x2C = (float)r / (float)L` (fild r; fidiv L); `+0x30 = q % 2` (C signed remainder: -1, 0 or 1).
- gr: `.Reversed = false`.

## 0x6428C0 CTaskSimpleCarDrive::ProcessHeadBopping  (catalogue: specified/REA, port level 3)
- `__thiscall void (this, CPed* ped [esp+4], bool bop [esp+8], float x [esp+0xC])`, `ret 0xC`. Local `s = 0.0f`.
- A. If bop: if `!(flags & 1)`: if `factor (+0x44) != 0.0f` (NaN counts as !=) -> bop = 0; else `r = GetRandomNumberInRange(0, 1000)`
  0x407180; if `r <= 995` -> bop = 0; else: if `+0x20 == -1` { StartBopping(ped); `+0x34 = +0x20` } else `+0x34 = now`; `+0x38 =
  GetRandomNumberInRange(1, 3)`; `+0x40 = CGeneral::GetRandomNumberInRange(3.0f, 8.0f)` 0x41BD90 (float version, ST0); `+0x44 = 0`;
  `+0x3C = 0`. If `flags & 1`: `s = sin(+0x2C * 3.1415927f [0x858CB8])` (`fsin`, stored f32); if `(u32)(now - +0x34) > 5000` and
  `GetRandomNumberInRange(0, 1000) > 995` -> bop = 0.
- B. `flags bit0 = bop`. If bop: if `factor < 1.0f` factor += 0.05f [0x858C28]; if `factor > 1.0f` factor = 1.0f; `v = (s - +0x3C) *
  factor`. Else: if `factor > 0.0f` factor -= 0.05f; if `factor < 0.0f` factor = 0; `v = (s - +0x3C) * (1.0f - factor)`.
  `+0x3C = v + +0x3C` (0x642A34).
- C. If `x < 64.0f [0x859A44]` and `+0x3C > 0.0f` (both strict): `a = +0x40 * +0x3C` (f32); `q = (ped->+0x490)->+0x10`;
  if `+0x38 >= 1`: `RtQuatRotate(q, &(1,0,0) [0x86EBD4], +0x30 == 0 ? a : -a, 1)` 0x7EB7C0; then (always)
  `RtQuatRotate(q, &(0,0,1) [0x86EBC8], +0x38 == 2 ? a : -a, 1)`; `ped->+0x470 |= 0x4000`.
- gr: `.Reversed = false`.

## 0x642AE0 CTaskSimpleCarDrive::ProcessArmBopping  (catalogue: specified/REA, port level 3)
- `__thiscall void (this, CPed* ped [esp+4], bool bop [esp+8], float x [esp+0xC])`, `ret 0xC`.
- 1. If `ped->IsPlayer()` return.
- 2. If `CWeather::Rain [0xC81324] > 0.0f`: `ped->m_pVehicle->ClearWindowOpenFlag(10)`, then (8), (11), (9) (0x6D30B0) and bop = 0.
  Else set the window of the ped's seat: driver -> 10, passenger[0] -> 8, passenger[1] -> 11, passenger[2] -> 9 (none -> nothing)
  `SetWindowOpenFlag` 0x6D3080.
- 3. `anim = 0xA2`; if ped is passenger[0] or passenger[2] `anim = 0xA3`; `assoc = RpAnimBlendClumpGetAssociation(ped->clump, anim)`.
- 4. If !bop: if assoc `assoc->m_fBlendDelta (+0x1C) = -4.0f`; `flags &= ~2`; return.
- 5. If `!(flags & 2)`: if `GetRandomNumberInRange(0, 1000) <= 995` return. `ok = 1`. If `m_pVehicle->+0x594 == 0`:
  `door = ComputeTargetDoorToExit(m_pVehicle, ped)`; `ok = 0` if `damageManager.GetDoorStatus_Component(door) != 0` 0x6C2250, or
  `CarHasDoorToClose(m_pVehicle, door)` 0x64EE10, or `!m_pVehicle->CanPedLeanOut(ped)` 0x6D5CF0 (tested in that order, stopping at the
  first failure). `V = vehModelInfo->+0x5C->+0x78` (CVector), `W = ...->+0x30`; if V == (0,0,0) (all three `== 0.0f`) return;
  `d = V.z - W.z`; if `veh->+0x429 & 8` return; if the handling's anim-group record (table 0xC1CDC8, stride 0x94, index handling+0xDE)
  has bit 3 of its first dword: require `0.4f [0x858EE8] <= d <= 0.44f [0x86EBF4]`, else `0.39f [0x86EBF0] <= d <= 0.46f [0x86EBEC]`
  (`d < low` returns, `d > high` returns; NaN passes both); if !ok return. (A non-zero +0x594 skips all of this.)
  Then `CAnimManager::BlendAnimation(ped->clump, 0, anim, 4.0f)` 0x4D4610; if `+0x20 == -1` { StartBopping(ped); `+0x48 = +0x20` } else
  `+0x48 = now`; `flags |= 2`.
- 6. If `flags & 2`: if !assoc { flags &= ~2; return }. If `x < 64.0f`: `a = (+0x2C < 0.75f) ? 0.0f : (+0x2C - 0.75f [0x858F34]) * 45.0f
  [0x858CB4]`; `q = (anim == 0xA2 ? ped->+0x49C : ped->+0x4A0)->+0x10`; `RtQuatRotate(q, &(0,0,1) [0x86EBE0], -a, 1)`; `ped->+0x470 |= 0x4000`.
  If `(u32)(now - +0x48) > 5000` and `GetRandomNumberInRange(0, 1000) > 995`: `assoc->+0x1C = -4.0f`; `flags &= ~2`.
- gr: `.Reversed = false`.

## 0x644470 CTaskSimpleCarDrive::ProcessPed  (catalogue: specified/REA, port level 7)
- `__thiscall bool (this, CPed* ped)`, `ret 4`; returns 0 while driving, 1 when the task ends. SEH (`sub esp,0x34`, stack events).
- Block 1 (0x64448E-0x6444DE): `retune = AudioEngine.HasRadioRetuneJustStarted()` 0x506FE0 (ECX 0xB6BC90). If `flags & 4` and ped
  bInVehicle and `ped->m_pVehicle` and it differs from m_pVehicle: re-register m_pVehicle to `ped->m_pVehicle` (CleanUpOldReference /
  RegisterReference).
- If `m_pVehicle == 0` or !ped bInVehicle -> END (0x644DD6): clear ped bInVehicle (`+0x46C &= ~0x100`), `AbortLookAt(ped, 250)` if looking,
  return 1.
- Block 2, upside-down (0x6444FB-0x6445EE): `g = CPedGroups::GetPedsGroup(ped)`. If `CTheScripts::UpsideDownCars.IsCarUpsideDown(m_pVehicle)`
  0x463830: if ped != FindPlayerPed(-1) and the player is bInVehicle and `player->m_pVehicle == m_pVehicle`: `ped->Say(0x22, 0, 1.0f, 0,0,0)`;
  then if `!g || g->membership.IsLeader(ped)`: `+0x4C += _ftol(CTimer::ms_fTimeStep [0xB7CB5C] * 0.02f [0x858B38] * 1000.0f)`; if `+0x4C > 2000`
  (unsigned): `CEventCarUpsideDown ev(m_pVehicle)` 0x4B1CC0 added to `ped->intel->m_eventGroup` (Add 0x4AB420), destroyed (0x4B1D50).
  Not upside down: `+0x4C = 0`.
- Block 3, cop car (0x6445EE-0x64465D): if `ped->IsPlayer()` and `m_pVehicle->IsLawEnforcementVehicle()` 0x6D2370 and (timer +0x50 not
  started or `IsOutOfTime(&+0x50)`): restart it (start = now, interval 2000, started = 1); `CEventCopCarBeingStolen ev(ped, m_pVehicle)`
  0x4B1740 added to `GetEventGlobalGroup()` 0x4ABA50; destroyed (0x4B17F0).
- If vehicle type == 9 (bike) or sub type == 2 or model == 0x213: `ped->+0x478 |= 0x100000`.
- Block 4, player driver (ped->IsPlayer() and `m_pVehicle->m_pDriver == ped`) (0x6446AD-0x6449A6):
  - `pad = CPlayerPed::GetPadFromPlayer(ped)` 0x609560. If byte [0x969179] and weapon slot 4 type != 0 and ammo > 0 and
    `CanPedLeanOut(ped)`: `p = new(0x44)`; `p ? CTaskSimpleGangDriveBy(p, nullptr, nullptr, 100.0f, 100, 8, false)` 0x6217D0; push it as
    `CEventScriptCommand(3, p, false)` 0x4B0A00 into the ped's event group, then destroy the event (0x4B0A50).
  - Door handling: `v = ped->m_pVehicle`. If v.model != 0x23A and v type == 6 and `!CDoor::IsClosed(v+0x608)` 0x6F4800: if pad and any of
    `GetAccelerate()` / `GetSteeringLeftRight()` / `GetBrake()` is non-zero (each converted to float and compared `!= 0.0f`, in that
    order) -> skip to Block 6; else `v->ProcessOpenDoor(ped, 10, CVehicleAnimGroupData::GetGroupForAnim(handling+0xDE, 0x182), 0x182, 1.0f)`
    (vtbl+0x70) -> Block 6.
  - Else if v type == 0 and `damageManager.GetDoorStatus(2)` 0x6C2230 == 1 and `!v->IsDoorMissing(2)` (vtbl+0x98): `a =
    GetAssociation(ped->clump, 0x182)`; if v gettingOut (+0x487) bit0 or gettingIn (+0x486) bit0: if a: `ProcessOpenDoor(ped, 10,
    GetGroupForAnim(.., a->+0x2C), 0x182, a->+0x20)`; then `ProcessBopping(ped, false)` 0x642E70. Else if a: same ProcessOpenDoor then
    ProcessBopping(false). Else if pad and any pad input non-zero: `ProcessBopping(ped, false)`. Else: `v->SetGettingOutFlags(1)` 0x6D3020;
    `flags |= 0x20`; `0x642700(ped)` (start the close-door anim: AddAnimation(clump, group, 0x182), finish callback 0x63C840 with this,
    stored in +0xC); return 0.
  - Else (other vehicles): `m_pVehicle->ProcessDrivingAnims(ped, retune)` (vtbl+0xB8); `ProcessBopping(ped, true)`.
- Block 5, non-player-driver occupants (0x6449AB-0x644AF2): a non-player driver of m_pVehicle goes to the ProcessDrivingAnims path above.
  Else if `m_pVehicle->IsPassenger(ped)` 0x6D1BD0: if `!(flags & 0x18)`: if `m_pVehicle->GetRideAnimData()` (vtbl+0xBC) non-null:
  `BlendAnimation(ped->clump, *rideData, 0xCD, 8.0f)`; `flags |= 0x10`. `ProcessBopping(ped, true)`. Then for a player passenger in a co-op
  game (`CGameLogic::IsCoopGameGoingOn()` 0x441390) with slot-4 weapon and ammo, `CanPedLeanOut`, and `TheCamera.Cams[ActiveCam].Mode`
  (word at 0xB6F1A8 + 0x238 * u8[0xB6F081]) == 0x31: drive-by task as above with last arg `(ped != passenger[1])`.
- Block 6, holding (0x644AF7-0x644BB6): if `held = ped->GetEntityThatThisPedIsHolding()` and `held->model == MI_GANG_SMOKE` (u16
  [0x8CD72C], read through an obfuscated jump at 0x407F90) and !IsLooking(ped) and `GetRandomNumberInRange(0, 60) == 15` and
  `h = FindActiveTaskByType(0x133)`: `h->PlayAnim(IsPassenger(ped) ? 0xAA : 0xA9, 0)` 0x691720. Then if `GetAssociation(ped->clump, 0xA9)`:
  `AbortLookAt(ped, 250)` if looking and `+0x20 = -1`; else `TriggerIK(ped)` 0x63C500.
- Block 7, mission passenger left alone (0x644BB9-0x644CFC): if `ped->+0x484 == 1` and no group and `m_pVehicle` has no driver and
  `passenger[0] == ped`: start timer +0x14 (now, 4000) if not started; when `IsOutOfTime(&+0x14)`: sequence `new(0x40)
  CTaskComplexSequence` + `AddTask(new(0x34) CTaskComplexLeaveCar(m_pVehicle, 0, 0, true, false))` + `AddTask(new(0x24)
  CTaskComplexCarDriveWander(m_pVehicle, 0, 10.0f))` 0x63CB10 (AddTask on a null sequence when allocation fails - ORIGINAL BUG),
  pushed as `CEventScriptCommand(3, seq, false)`. If the conditions fail at the driver/passenger test: `+0x1C = 0` (timer reset).
- Block 8, every 16384 frames per ped (0x644D00-0x644DCD): if `((u16)ped->+0x20 + CTimer::m_FrameCounter [0xB7CB4C]) & 0x3FFF == 0`
  and the driver is another ped who is the player: `speed = sqrt(vx*vx + vy*vy)` of `m_pVehicle->m_vecMoveSpeed` (x87: x*x computed
  first, then + y*y, `fsqrt`); if `speed > 0.7f [0x858CB0]` `ped->Say(0x20, ...)`;
  recomputed speed `< 0.1f [0x858B1C]` -> `ped->Say(0x27, ...)`; then always `ped->Say(0x29, 0, 1.0f, 0, 0, 0)`.
- Return 0.
- gr: `.Reversed = false`. Open questions: name of the byte flag 0x969179 (drive-by enable, likely CPad/CHud setting) and of
  `veh+0x429 bit 3`.
