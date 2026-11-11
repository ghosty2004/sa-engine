# CTaskComplexLeaveCar (AI) - RE-SPEC-3 group I1

Vtable 0x86E828 (CTaskComplexLeaveCarAndDie 0x86DD00 shares MakeAbortable / CreateNextSubTask / ControlSubTask), task type 0x2C0
(704). Layout (gr TaskComplexLeaveCar.h, size 0x34): +0xC m_pTargetVehicle CVehicle* (registered ref), +0x10 m_nTargetDoor i32
(8..11), +0x14 m_nDelayTime i32, +0x18 m_bSensibleLeaveCar u8, +0x19 m_bForceGetOut u8, +0x1A m_bDie u8,
+0x1C m_pTaskUtilityLineUpPedWithCar (plain heap object, size 0x1C, `operator new` 0x82119A / `free` 0x8214BD),
+0x20 m_nDoorFlagsSet u8, +0x21 m_nNumGettingInSet u8, +0x24 m_nDieAnimID i32, +0x28 m_fDieAnimBlendDelta f32,
+0x2C m_fDieAnimSpeed f32, +0x30 m_bIsInAir u8.
Vehicle fields: +0x384 m_pHandlingData (handling +0xCC = model flags: bit 9 and bit 15 tested), +0x460 m_pDriver,
+0x464/+0x468/+0x46C m_apPassengers[0..2], +0x485 m_nNumGettingIn u8, +0x486 m_nGettingInFlags, +0x487 m_nGettingOutFlags,
+0x4F8 door lock (eCarLock; 1 = UNLOCKED, 5 = COP_CAR), +0x590 m_nVehicleType (0 automobile, 5 boat, 9 bike), +0x594 sub type
(2 = QUAD, 4 = PLANE), +0x44 m_vecMoveSpeed, +0x5A0 CDamageManager (automobile), +0x648 CAutomobile::m_aCarNodes[] (RwFrame*).
Ped: +0x46C bit 8 bInVehicle, +0x470 bit 19 (0x80000) bIsDrowning, +0x58C m_pVehicle, +0x530 m_nPedState, +0x718 active weapon
slot (i8), +0x5A0 + 0x1C*slot weapon type, +0x4C m_vecMoveSpeed.z, +0x18 m_pRwClump.
Helpers used (not in this group; catalogue unknown, proposed names):
- 0x63BAB0 `CTaskComplexLeaveCar::ComputeTargetDoor(CPed*)` (thiscall `ret 4`): if m_nTargetDoor == 0 then
  `m_nTargetDoor = CCarEnterExit::ComputeTargetDoorToExit(veh, ped)` 0x64F110.
- 0x63BA00 `CTaskComplexLeaveCar::SetVehicleFlags(CPed*)` (thiscall `ret 4`): m_nDoorFlagsSet = ComputeDoorFlag(veh, door, true) 0x64E550;
  veh->SetGettingOutFlags(it) 0x6D3020; m_nNumGettingInSet = 1; veh->m_nNumGettingIn++; then autopilot tweaks for a non-player
  driver leaving sensibly, and player-specific handling (not specified here; see its own spec when assigned).
- 0x63BAE0 `CTaskComplexLeaveCar::CreateLineUpTask(CPed*)` (thiscall `ret 4`): `m_pTaskUtilityLineUpPedWithCar = new(0x1C)
  CTaskUtilityLineUpPedWithCar(&zeroVector, 0, 0, m_nTargetDoor)` 0x64FBB0 (or 0); the previous pointer is overwritten, not freed.
- 0x646CF0 (cdecl `(CVehicle*, int door, bool* gettingIn, bool* gettingOut)`): door 8/9/10/11 -> bit 4/8/1/2 (jump table 0x646D58);
  sets `*gettingIn = 1` if `veh->+0x486 & bit`, `*gettingOut = 1` if `veh->+0x487 & bit` (never clears them). Proposed name
  `CCarEnterExit::IsCarDoorInUse`.
- 0x4ABCA0 `CEventDamage::HasKilledPed()` (thiscall): `m_damageResponse.m_bHealthZero (+0x40) && m_bAddToEventGroup (+0xC)` -> 1/0
  (gr EventDamage.h:69 inline; catalogue unknown FUN_004abca0 - NAME PROPOSAL).

## 0x63B960 CTaskComplexLeaveCar::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: returns 0x2C0.

## 0x63B970 CTaskComplexLeaveCar::~CTaskComplexLeaveCar  (catalogue: specified/REA, port level 2)
- Complete dtor (SEH): vtbl = 0x86E828; if veh: `veh->ClearGettingOutFlags((u8)m_nDoorFlagsSet)` 0x6D3060; `veh->m_nNumGettingIn -=
  m_nNumGettingInSet` (u8 sub); `CleanUpOldReference(veh, &m_pTargetVehicle)` 0x571A00; if lineUp: `lineUp->~CTaskUtilityLineUpPedWithCar()`
  0x64FC00 and `free(lineUp)` 0x8214BD; `~CTaskComplex` 0x61A3D0 (0x63B98E-0x63B9E5). gr cpp:35 matches (gr `delete`).

## 0x6410E0 CTaskComplexLeaveCar::`scalar deleting destructor'  (catalogue: specified/REA, port level 3)
- Slot 0 of 0x86E828: call 0x63B970; `if (flags & 1) CTask::operator delete(this)` 0x61A5B0; return this; `ret 4`.

## 0x641100 CTaskComplexLeaveCar::MakeAbortable  (catalogue: specified/REA, port level 25)
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority [+8], const CEvent* event [+0xC])`, `ret 0xC`, SEH frame
  (`sub esp,0x60`; two task objects live on the stack). Base B = esp after `push ebx; push edi`: ped [B+0x7C], priority [B+0x80],
  event [B+0x84]. "ClearFlags" below = `veh->ClearGettingOutFlags(m_nDoorFlagsSet)`; `m_nDoorFlagsSet = 0`;
  `veh->m_nNumGettingIn -= m_nNumGettingInSet`; `m_nNumGettingInSet = 0` (exact order 0x6411E6-0x64120E).
- Behaviour:
  1. If m_pTargetVehicle == 0 return 1 (0x64111B-0x641133).
  2. priority == 2 (IMMEDIATE) (0x641145-0x641223): `m_pSubTask->MakeAbortable(ped, 2, event)` (result ignored, no null check).
     If ped bInVehicle: `ComputeTargetDoor(ped)` only if m_nTargetDoor == 0 (0x641167-0x641171); construct on the stack
     `CTaskSimpleCarCloseDoorFromOutside cd(veh, m_nTargetDoor, nullptr)` 0x6464F0 and call `cd.MakeAbortable(ped, 2, event)` 0x64B020;
     construct `CTaskSimpleCarSetPedOut so(veh, m_nTargetDoor, (u8)m_bSensibleLeaveCar)` 0x6478B0; `so.ProcessPed(ped)` 0x647D10;
     destroy `so` (0x647950) then `cd` (0x646570). Then always ClearFlags and return 1.
  3. priority == 1 (URGENT) (0x64122F-0x64152A); `t()` = `m_pSubTask->GetTaskType()` - re-called at each test:
     a. t() == 0xD4 (DIE): (dead re-test t() == 0x2CF -> return 0); `r = sub->MakeAbortable(ped, 1, event)`; if r ClearFlags; return r.
     b. t() == 0x329 (WAIT_TO_SLOW_DOWN) and event and `event->GetEventType() == 9` (DAMAGE) and `HasKilledPed(event)` 0x4ABCA0:
        `r = sub->MakeAbortable(ped, 1, event)`; if r ClearFlags; return r.
     c. t() == 0xCE (GET_UP_AND_STAND_STILL) with the same event test: same as b.
     d. t() == 0x32E (JUMP_OUT): if event == 0 return 0. If event type is 0x21 (IN_AIR) or 0x42 (IN_WATER) (type re-queried):
        if type == 0x21 `m_bIsInAir = 1`; `if (!sub->MakeAbortable(ped, 1, event)) return 0`; then:
        - if ped's weapon type == 0x2E (parachute) and type == 0x21 and `veh->GetNumContactWheels()` (vtbl+0xDC) == 0:
          if `veh->+0x594 == 4` (plane) and `|veh->m_vecMoveSpeed| > 0.2f [0x858CC4]` (CVector::Magnitude 0x4082C0; `test ah,0x41; je`
          strict) -> blend 8.0f; else if `!ped->IsPlayer()` return 0; else if `CWorld::ProcessVerticalLine(pedPos, -10.0f, &colPoint,
          &entity, true, false, false, false, true, false, nullptr)` 0x5674E0 hits -> return 0; else blend 8.0f.
        - else if type == 0x42 and `ped->m_vecMoveSpeed.z < -0.3f [0x858EE4]` (strict): blend 16.0f.
        - "blend d" = `CAnimManager::BlendAnimation(ped->m_pRwClump, 0, 0x7D, d)` 0x4D4610 (d passed as raw f32 dword stored in the ped
          arg slot). In ALL these sub-cases the function returns 0 (AL cleared at 0x6414E2) even though the subtask was aborted, and
          the getting-out flags are NOT cleared - original behaviour.
     e. otherwise (incl. JUMP_OUT with any other event type): if event == 0 return 0; if t() == 0x2CF (DRIVE_TIMED) return
        `sub->MakeAbortable(ped, 1, event)` (no ClearFlags); else return 0.
  4. Any other priority (0 LEISURE): return 0.
- gta-reversed: plugin::Call (cpp:51), not reversed. Open questions: none.

## 0x641530 CTaskComplexLeaveCar::CreateSubTask  (catalogue: specified/REA, port level 4)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`, SEH. Entry is obfuscated (`nop; jmp 0x40181E`, which
  reads fs:[0] and jumps back) - no effect. Returns 0 for unknown types and when `CTask::operator new` fails.
- Cases (allocation size, constructor and arguments in order):
  | type | size | constructor |
  |---|---|---|
  | 0xCA PAUSE | 0x18 | CTaskSimplePause(-1) 0x48E750 |
  | 0xCE GET_UP_AND_STAND_STILL | 0x0C | CTaskComplexGetUpAndStandStill() 0x678130 |
  | 0xD4 DIE | 0x28 | CTaskSimpleDie(0 /*group*/, m_nDieAnimID, m_fDieAnimBlendDelta, m_fDieAnimSpeed) 0x62FA00 |
  | 0x2CC LEAVE_BOAT | 0x14 | CTaskComplexLeaveBoat(veh, 0) 0x63BB60 |
  | 0x2CF DRIVE_TIMED | 0x70 | CTaskSimpleCarDriveTimed(veh, m_nDelayTime) 0x5FF940 |
  | 0x326 CLOSE_DOOR_FROM_OUTSIDE | 0x1C | CTaskSimpleCarCloseDoorFromOutside(veh, m_nTargetDoor, m_pTaskUtilityLineUpPedWithCar) 0x6464F0 |
  | 0x329 WAIT_TO_SLOW_DOWN | 0x14 | if m_bDie: (veh, 2); else if m_bSensibleLeaveCar: ped->IsPlayer() ? (veh, 0) : (veh, 1); else (veh, 2) - CTaskSimpleCarWaitToSlowDown 0x646990 |
  | 0x32A WAIT_FOR_DOOR_NOT_TO_BE_IN_USE | 0x14 | (veh, m_nTargetDoor, 0) 0x646B70 |
  | 0x32D GET_OUT | 0x20 | CTaskSimpleCarGetOut(veh, m_nTargetDoor, lineUp) 0x6471D0 |
  | 0x32E JUMP_OUT | 0x24 | CTaskSimpleCarJumpOut(veh, m_nTargetDoor, lineUp) 0x64BE40 |
  | 0x32F FORCE_PED_OUT | 0x10 | 0x647710 (catalogue unknown; proposed CTaskSimpleCarForcePedOut ctor)(veh, m_nTargetDoor) |
  | 0x330 SET_PED_OUT | 0x18 | CTaskSimpleCarSetPedOut(veh, m_nTargetDoor, (u8)m_bSensibleLeaveCar) 0x6478B0 |
  | 0x337 SLOW_BE_DRAGGED_OUT | 0x1C | CTaskComplexCarSlowBeDraggedOut(veh, m_nTargetDoor, true) 0x648490 |
  Compare tree (for exactness of which types reach which case): `>0x32A` / `==0x32A` / `>0x2CC` / `==0x2CC`, then subtractions
  (0x641542-0x641690, 0x64186A-0x64199B). `ped` is only used by the 0x329 case (IsPlayer 0x5DF8F0).
- gta-reversed: `.Reversed = false` (plugin::Call).

## 0x6419F0 CTaskComplexLeaveCar::CreateNextSubTask  (catalogue: specified/REA, port level 9)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`. `t = m_pSubTask->GetTaskType()` (once, 0x6419FF). Below, `C(x)` =
  `CreateSubTask(x, ped)`; C(0x516) returns 0 (task finished).
- t == 0xCA PAUSE: C(0x516). t == 0xD4 DIE: C(0x516). t == 0xCE: ped bInVehicle ? C(0x330) : C(0x516).
- t == 0x2CC LEAVE_BOAT: m_bDie ? C(0xD4) : C(0x516).
- t == 0x326 CLOSE_DOOR: `m_nDieAnimID = 0xF`; C(0x330) (0x641A97).
- t == 0x2CF DRIVE_TIMED (0x641AB8): if !ped bInVehicle -> C(0x516). If `!ped->m_pVehicle->IsPassenger(ped)` 0x6D1BD0 and `!IsDriver(ped)`
  0x6D1C40: clear ped bInVehicle (`+0x46C &= ~0x100`), C(0x516). Else `ComputeTargetDoor(ped)`; `in = out = false`;
  `0x646CF0(veh, m_nTargetDoor, &in, &out)` (the `out` flag lives in the low byte of the ped arg slot, `in` in a local byte);
  if !m_bForceGetOut: out ? C(0x32A) : (in ? C(0x516) : C(0x329)); if m_bForceGetOut: C(0x329).
- t == 0x329 WAIT_TO_SLOW_DOWN (0x641B68):
  - if `!veh->CanPedStepOutCar(false)` (vtbl+0xE4): SetVehicleFlags(ped); `ped->SetPedState(0)` 0x5E4500; C(0x32E JUMP_OUT).
  - else if `CCarEnterExit::IsRoomForPedToLeaveCar(veh, m_nTargetDoor, nullptr)` 0x6504C0: goto OUT.
  - else choose an alternative door `alt` from m_nTargetDoor (jump table 0x641F84):
    door 8: if driver (+0x460) or (gettingIn & 1) -> FORCE; else alt = 10.
    door 9: if passenger[1] (+0x468) or (gettingIn & 2) -> FORCE; else alt = 11.
    door 10: if vehType == 9 or handling bit 9 -> alt = 8; else if passenger[0] (+0x464) or (gettingIn & 4) -> FORCE; else alt = 8.
    door 11: if vehType == 9 or handling bit 9 -> alt = 9; else if passenger[2] (+0x46C) or (gettingIn & 8) -> FORCE; else alt = 9.
    other: alt = 0.
    Then: if `!IsRoomForPedToLeaveCar(veh, alt, nullptr)` -> FORCE; if handling bit 15 and vehType == 0 and
    `automobile->m_aCarNodes[alt]` (+0x648 + 4*alt) == 0 -> FORCE; else `m_nTargetDoor = alt`, goto OUT.
  - OUT (0x641CC6): SetVehicleFlags(ped); SetPedState(0); m_bDie ? C(0x337) : (CreateLineUpTask(ped); C(0x32D GET_OUT)).
  - FORCE (0x641C23): SetVehicleFlags(ped); SetPedState(0); CreateLineUpTask(ped); C(0x32F FORCE_PED_OUT).
- t in 0x32A..0x337 (byte table 0x641FB0, jump table 0x641F94):
  - 0x32A WAIT_FOR_DOOR: C(0x329).
  - 0x32D GET_OUT (0x641D70): `lineUp->+0x14 = 2` (no null check); if (!m_bForceGetOut or veh->+0x4F8 != 1) and
    `CCarEnterExit::CarHasDoorToClose(veh, door)` 0x64EE10 -> C(0x326). Else: if vehType == 0 and CarHasDoorToClose(veh, door):
    `s = damageManager.GetDoorStatus(door)` 0x6C2250; if s == 0 or s == 2: `SetDoorStatus(door, GetDoorStatus(door) + 1)` 0x6C21E0
    (GetDoorStatus is called again). Then C(0x330).
  - 0x32E JUMP_OUT (0x641E42): if m_bDie: `m_nDieAnimID = 0xF`, blend = 1000.0f, speed = 0.5f (raw immediates), C(0xD4). Else if
    veh == 0 or vehType == 9 or subtype == 2: C(0x516); else if m_bIsInAir: C(0x330); else if ped bIsDrowning (+0x470 & 0x80000):
    C(0x330); else ClearFlags (as MakeAbortable) and C(0xCE).
  - 0x32F FORCE_PED_OUT: C(0x330).
  - 0x330 SET_PED_OUT: m_bDie ? C(0xD4) : C(0x516).
  - 0x337 SLOW_BE_DRAGGED_OUT (0x641F00): `m_nDieAnimID = 0x24`, blend 1000.0f, speed 0.5f; if veh and `veh->+0x4F8 == 5` set it to 1;
    C(0xD4).
  - 0x32B, 0x32C, 0x331..0x336: return 0 (no new subtask).
- Any other t: return 0.
- gta-reversed: plugin::Call (cpp:56).

## 0x641FC0 CTaskComplexLeaveCar::CreateFirstSubTask  (catalogue: specified/REA, port level 10)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH (`sub esp,0x14`, local CEventGroupEvent).
- Behaviour:
  1. If (`ped->m_nPedState (+0x530) == 0x3F` or `ped->+0x474 & 0x40`) and `ped->IsPlayer()`: `p = new(0x18)`; return p ?
     `CTaskSimplePause(p, -1)` : 0 (0x641FDE-0x64204C).
  2. If !ped bInVehicle: `printf("CTaskComplexLeaveCar - ped not in car")` (0x821982 on string 0x86EB9C, debug output) then the same
     Pause(-1) task (0x642015-0x64204C).
  3. If `veh->m_pDriver (+0x460) == ped` (0x642064): `g = CPedGroups::GetPedsGroup(ped)` 0x5F7E80; if g and
     `g->m_groupMembership.IsLeader(ped)` 0x5F69C0: `e = CEvent::operator new(0x14)`; `e ? CEventLeaderExitedCarAsDriver(e)`
     0x4B8300 : 0; `CEventGroupEvent ge(ped, e)` on the stack 0x4ADFD0; `g->m_groupIntelligence.AddEvent(&ge)` 0x5F7470 (ECX = g+0x30);
     `~CEventGroupEvent(&ge)` 0x4AE070 (deletes e). Then if !ped->IsPlayer(): `ped->SetRadioStation()` 0x5DFD90; else if
     ped->m_pVehicle: `CAEVehicleAudioEntity::PlayerAboutToExitVehicleAsDriver(&veh->+0x138)` 0x4F5BA0.
  4. If `veh->m_nVehicleType == 5` (boat): `p = new(0x14)`; return p ? `CTaskComplexLeaveBoat(p, veh, 0)` : 0.
  5. Else `p = new(0x70)`; return p ? `CTaskSimpleCarDriveTimed(p, veh, m_nDelayTime)` : 0.
- gta-reversed: plugin::Call (cpp:61).

## 0x6421B0 CTaskComplexLeaveCar::ControlSubTask  (catalogue: specified/REA, port level 5)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`.
- If veh == 0 return 0. If !ped bInVehicle and the subtask type (re-queried per compare) is none of 0x330, 0x32E, 0xCE, 0xD4:
  return `CreateSubTask(0x330, ped)` (0x6421C5-0x642214). Else if `!m_bSensibleLeaveCar` and type == 0x329:
  `m_pSubTask->MakeAbortable(ped, 0, nullptr)` (result ignored). Return m_pSubTask.
- gta-reversed: cpp:66 C++ (not hooked) matches the bytes (gr calls GetTaskType once - harmless).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x62F250 CTaskComplexLeaveCar::~CTaskComplexLeaveCar  (catalogue: specified/REA, port level 4)
- Scalar deleting destructor, `__thiscall void* (this, uint8 flags [esp+4])`, `ret 4`: calls the complete dtor 0x62F270 FUN_0062f270 (unnamed complete dtor: stores vtbl 0x86DD00) (0x62F253); if `flags & 1` (0x62F258) calls 0x61A5B0 CTask::operator delete(this) (cdecl, 0x62F260); returns this (0x62F268).

<!-- RE-SPEC-AI-2 helper A (groups J1/J6) additions -->

## 0x63D9E0 CTaskComplexLeaveCar::Clone  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* ()`, `ret`, SEH frame: `p = CTask::operator new(0x34)` 0x61A5A0 (cdecl, task pool, 0x63D9FB); if p == 0 return 0 (0x63DA11); else `CTaskComplexLeaveCar` 0x63B8C0 on p with args (this+0xC, this+0x10, this+0x14, (u8)this+0x18 (zero-extended) (byte, mov), (u8)this+0x19 (zero-extended) (byte, mov)) (0x63DA2D) and return its EAX (the ctor returns p).
