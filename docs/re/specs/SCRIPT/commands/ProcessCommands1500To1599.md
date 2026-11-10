# CRunningScript::ProcessCommands1500To1599 (SCRIPT) - opcode specs

Notation (shared by all ProcessCommands* spec files):
- `this` = CRunningScript* (EDI in the case blocks unless stated). `SP[i]` = ScriptParams[i] at 0xA43C78 + 4*i (i32 or f32 as used).
- `Collect(n)` = CRunningScript::CollectParameters 0x464080 (thiscall, n). `Store(n)` = StoreParameters 0x464370. `Cmp(b)` =
  UpdateCompareFlag 0x4859D0 (thiscall, bool pushed as a dword whose low byte is the flag). `ReadLabel(buf,8)` =
  ReadTextLabelFromScript 0x463D50. `VarPtr()` = GetPointerToScriptVariable 0x464790. `CollectNoPC()` =
  CollectNextParameterWithoutIncreasingPC 0x464250.
- `PED(h)` = CPedPool::GetAtRef 0x404910 on [0xB74490] CPools::ms_pPedPool (null when the slot flag byte != (h & 0xFF));
  `VEH(h)` = 0x4048E0 (catalogue FUN_004048e0 = CVehiclePool::GetAtRef, stride 0xA18, continues through the .text stub
  0x40210A, same semantics) on [0xB74494]; `OBJ(h)` = 0x465040 (FUN_00465040 = CObjectPool::GetAtRef, stride 0x19C) on
  [0xB7449C]. `PEDREF(p)` = 0x4442D0 (jmp stub to .HOODLUM 0x156CD40: ((p - base) / 0x7C4) << 8 | flags[idx], i.e.
  CPool<CPed>::GetRef).
- `ftol` = 0x821B40 (MSVC _ftol2: truncates ST0 toward zero, result EDX:EAX). `CRGBA(r,g,b,a)` = 0x7170C0 (thiscall ctor, ret 0x10).
- AL return 0 = OR_CONTINUE unless the case says otherwise (gr OpcodeResult: 0 continue, 1 wait, -1 interrupt).
- Memory stores are written `[0xADDR] <- value`. SEH: in switches that have an SEH frame (see the dispatcher), task-constructing cases set the frame state slot around `new` + ctor and back to -1; this only matters for exception unwinding.
- Vtable slots (index = offset/4): CEntity 0 dtor, 1 Add(rect), 2 Add, 3 Remove, 4 SetIsStatic, 5 SetModelIndex, 6
  SetModelIndexNoCreate, 7 CreateRwObject, 8 DeleteRwObject, 9 GetBoundRect, 10 ProcessControl, 11 ProcessCollision, 12
  ProcessShift, 13 TestCollision, 14 Teleport, 15 SpecialEntityPreCollisionStuff, 16 SpecialEntityCalcCollisionSteps, 17 PreRender,
  18 Render, 19 SetupLighting, 20 RemoveLighting, 21 FlagToDestroyWhenNextProcessed, 22 ProcessEntityCollision (CPhysical);
  CTask vtable: 0 dtor, 1 Clone. CPed +0x46C flags (byte 1 bit 0x01 bInVehicle), +0x474 flags dword 3, +0x47C m_pIntelligence (event group at intelligence+0x68), +0x58C m_pVehicle. CVehicle +0x3BA m_autoPilot.m_nCarMission, +0x41C m_autoPilot.m_pTargetCar, +0x464 m_apPassengers[8], +0x488 m_nMaxPassengers.

- CRunningScript fields: +0x10 m_BaseIP, +0x14 m_IP, +0x3C m_LocalVars[34], +0xC5 m_CondResult, +0xC6 m_UsesMissionCleanup,
  +0xC7 m_IsExternal, +0xC9 m_ExternalType. `MissionCleanUp` = CTheScripts::MissionCleanUp 0xA90850 (CMissionCleanup;
  AddEntityToList 0x4637E0 / RemoveEntityFromList 0x4654B0, thiscall (handle, type: 1 car, 2 char, 3 object)).
- gr `unimplemented` = REGISTER_COMMAND_UNIMPLEMENTED -> notsa::script::detail::NotImplemented (CommandParser/Parser.hpp:68): reads no
  parameters and returns OR_INTERRUPT (-1). That equals the exe only for opcodes that reach the switch default; for an
  `unimplemented` opcode that HAS a case here, gr deviates (it skips the case and leaves the IP on the parameters).
- Groups: CPedGroups::ms_groups 0xC09920 (CPedGroup, stride 0x2D4; +0x08 m_groupMembership (0xC09928), +0x30 m_groupIntelligence
  (0xC09950), +0x2D0 byte (0xC09BF0) set by CREATE_GROUP), ms_activeGroups 0xC098E0 (8 bytes). Task sequences: CTaskSequences::
  ms_taskSequence 0xC178F0 (CTaskComplexSequence, stride 0x40), ms_bIsOpened 0xC17898, ms_iActiveSequence 0x8D2E98 (i32),
  CTheScripts::ScriptSequenceTaskArray 0xA43F68 (stride 4, byte 0 = in-use flag). Script thing types for
  GetActualScriptThingIndex 0x4839A0 / GetNewUniqueScriptThingIndex 0x483720 (cdecl (value, type)): 4 sequence, 6 attractor
  (2D effect), 8 group.

## 0x490DB0 CRunningScript::ProcessCommands1500To1599  (catalogue: specified/REA, port level 36)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x490DDF), command id at [esp+4] kept in EBX for the whole function
  (`mov ebx,[esp+0x22C]` at 0x490DCC), `ret 4`. SEH frame `push -1; push 0x838A54; push fs:[0]` (0x490DB0-0x490DBE), `sub esp,0x218`,
  push EBX/EBP/ESI/EDI. In the cases: command id also at `[esp+0x238]`, SEH state slot `[esp+0x230]` (each task-constructing case
  stores its own unwind state number there, 0x00..0x21, around `new` + ctor, then -1).
  Range: `eax = id - 0x5DC` (0x490DD5), `cmp eax,0x5D; ja 0x492AA9` (0x490DDB/0x490DE1), `jmp [eax*4 + 0x492AC8]` (94 dwords).
- Default 0x492AA9: `or al,0xFF` (AL = -1). Common epilogue 0x492AAB: `mov ecx,[esp+0x228]; pop edi/esi/ebp; fs:[0] <- ecx;
  pop ebx; add esp,0x224; ret 4`. Cases end `xor al,al; jmp 0x492AAB`; 0x492203 and 0x492AA5 are shared `xor al,al` tails.
- Shared task tails: GIVE-A 0x492A8E (`task = null`) / 0x492A90: `push ebx (command id); push eax (task); push ebp (ped handle);
  mov ecx,esi; nop x4; nop; nop; jmp 0x4015FB` -> .text stub `mov [esp+0x23C], -1` (SEH state -1) `; jmp 0x492AA0` ->
  `this->GivePedScriptedTask(ped, task, commandId)` 0x465C20, AL = 0. GIVE-B 0x490ED2 (`task = null`) / 0x490ED4: same call with
  ped handle in EBX and the command id re-read from `[esp+0x238]` (used where EBX was overwritten).
- Opcodes with a case: 94 table entries (id 0x5DC..0x639). Empty `xor al,al` cases (0x492AA5, AL = 0, no parameters; all gr nop):
  05DF 05E0 05E1 05E3-05E8 05EA 05EF 05F0 0608 0609 060C 0610 0617 061C 061F 0620 062B 062C 062D (23). gta-reversed handler 32, nop 25
  (not re-specified). Specified below: 32 unregistered (05DC 05DD 05DE 05EB 05EC 05ED 05EE 05F1 05F2 05F3 05F4 05F5 0603 0604 0605 0606
  0607 060E 0615 0616 061B 061D 061E 0622 062F 0630 0631 0632 0633 0635 0637 0638) + 5 unimplemented with a case (05F7 05F9 05FA
  05FF 0600).
- Default-reaching: 063A-063F (gr unimplemented -> -1, same as the exe).
- gta-reversed deviation: 05F7, 05F9, 05FA, 05FF, 0600 are gr `unimplemented` (-1, no parameters read) but share the real case
  0x491289 with the gr-handled 05F6 05F8 05FB-05FE 0601 (see below).

### 0x49186F 0615 COMMAND_OPEN_SEQUENCE_TASK  (gr: unregistered, used: 28 / 1369 / 65)
- Parameters: output only. `GetActualScriptThingIndex(CollectNoPC(), 4)` is evaluated and its result discarded (0x491879).
- `slot = CTaskSequences::GetAvailableSlot((u8) this->m_UsesMissionCleanup)` 0x632E00 (cdecl; `add esp,0xC` at 0x49188E pops the
  three pushes of both calls).
- If `0 <= slot < 0x40`: `ms_bIsOpened[slot] <- 1`; `ms_taskSequence[slot].Flush()` 0x632C10; `[0x8D2E98] <- slot`;
  `SP0 <- GetNewUniqueScriptThingIndex(slot, 4)`; if `m_UsesMissionCleanup`: `MissionCleanUp.AddEntityToList(SP0, 8)` 0x4637E0.
  Else `SP0 <- -1` (0x4918EE). Both: `Store(1)`.

### 0x491908 0616 COMMAND_CLOSE_SEQUENCE_TASK  (gr: unregistered, used: 28 / 1369 / 65)
- `Collect(1)`; `i = GetActualScriptThingIndex(SP0, 4)`; if `0 <= i < 0x40`: `ms_bIsOpened[i] <- 0`; `[0x8D2E98] <- -1`.

### 0x491A4A 061B COMMAND_CLEAR_SEQUENCE_TASK  (gr: unregistered, used: 27 / 1252 / 64)
- `Collect(1)`; `i = GetActualScriptThingIndex(SP0, 4)`. If `0 <= i < 0x40`: `ms_bIsOpened[i] <- 0`; `s = &ms_taskSequence[i]`;
  if `s[+0x3C]` (dword, reference count) == 0: `s[+0x38] <- 0` (byte), `s->Flush()`; else `s[+0x38] <- 1` (flush deferred);
  `ScriptSequenceTaskArray[i] byte <- 0` (0xA43F68 + i*4).
- If `m_UsesMissionCleanup`: `MissionCleanUp.RemoveEntityFromList(SP0, 8)` (also for an invalid handle).

### 0x4913F5 0605 COMMAND_TASK_PLAY_ANIM  (gr: unregistered, used: 0 / 922 / 218)
- `this->PlayAnimScriptCommand(commandId)` 0x470150 (thiscall, EBX pushed; separate function, reads its own parameters). AL = 0.

### 0x492975 0638 COMMAND_TASK_STAY_IN_SAME_PLACE  (gr: unregistered, used: 0 / 507 / 7)
- `Collect(2)`: SP0 ped handle or -1, SP1 flag.
- If `SP0 != -1`: `ped = PED(SP0)` (unchecked); local `CTaskSimpleSetStayInSamePlace t(SP1 != 0)` 0x62F590; `t.ProcessPed(ped)`
  0x62F5E0 (applied immediately, no task is given); `t` destroyed 0x62F5D0. AL = 0.
- If `SP0 == -1`: `t = new(0xC) CTaskSimpleSetStayInSamePlace(SP1 != 0)` (null -> 0); `ms_taskSequence[[0x8D2E98]].AddTask(t)`
  0x632D10 (index read unchecked: with no open sequence (-1) it addresses 0xC178F0 - 0x40 - original bug, scripts always open
  one first).

### 0x4911EF 05F5 COMMAND_TASK_FOLLOW_PATH_NODES_TO_COORD  (gr: unregistered, used: 0 / 224 / 8)
- `Collect(6)`: SP0 ped (EBX), SP1..SP3 target, SP4 move state (EBP), SP5 time. time: `-1 -> [0x86FCAC]` (i32 50000), `-2 -> -1`.
- `t = new(0x60) CTaskComplexFollowNodeRoute(SP4, &target, [0x86FCA0] (0.5f), [0x86FCA4] (3.0f), [0x86FCA8] (2.0f), 1, time, 1)`
  0x66EA30 (args 3-5: tolerance, slowDownDist, followNodeThresholdHeightChange; bKeepNodesHeadingAwayFromTarget 1, bUseBlending 1).
  GIVE-B.

### 0x4915F5 060E COMMAND_IS_PLAYBACK_GOING_ON_FOR_CAR  (gr: unregistered, used: 0 / 257 / 2)
- `Collect(1)`; `Cmp(CVehicleRecording::IsPlaybackGoingOnForCar(VEH(SP0)) != 0)` 0x4594C0 (cdecl; null passed unchecked; scratch
  `[esp+0x14]` via 0x4912DC/0x4912E1).

### 0x490FB0 05EB COMMAND_START_PLAYBACK_RECORDED_CAR  (gr: unregistered, used: 0 / 286 / 1)
- `Collect(2)`; `CVehicleRecording::StartPlaybackRecordedCar(VEH(SP0), SP1, 0, 0)` 0x45A980 (cdecl 4 args; null unchecked).

### 0x490FE6 05EC COMMAND_STOP_PLAYBACK_RECORDED_CAR  (gr: unregistered, used: 0 / 321 / 0)
- `Collect(1)`; `v = VEH(SP0)`; if non-null `CVehicleRecording::StopPlaybackRecordedCar(v)` 0x45A280.

### 0x491018 05ED COMMAND_PAUSE_PLAYBACK_RECORDED_CAR / 0x491043 05EE COMMAND_UNPAUSE_PLAYBACK_RECORDED_CAR  (gr: unregistered, used: 05ED 0/33/0, 05EE 0/37/0)
- `Collect(1)`; `CVehicleRecording::PausePlaybackRecordedCar(VEH(SP0))` 0x459740 / `UnpausePlaybackRecordedCar` 0x459850 (cdecl,
  null passed unchecked).

### 0x49277A 0633 COMMAND_TASK_LEAVE_ANY_CAR  (gr: unregistered, used: 0 / 161 / 7)
- `Collect(1)`; `t = new(0x14) CTaskComplexLeaveAnyCar(delayTime 0, sensibleLeaveCar 1, forceGetOut 0)` 0x421150;
  `GivePedScriptedTask(SP0, t, commandId)` (inline copy of GIVE-A with EDI = SP0, 0x4927B7).

### 0x490E63 05DD COMMAND_TASK_SMART_FLEE_CHAR  (gr: unregistered, used: 0 / 73 / 0)
- `Collect(4)`: SP0 ped, SP1 flee-from ped, SP2 safe distance as an INTEGER, SP3 time. `d = (float)(i32)SP2` (fild, stored f32,
  0x490E6C). `e = PED(SP1)` (may be null, passed through).
- `t = new(0x40) CTaskComplexSmartFleeEntity(e, scream 1, d, SP3, posCheckPeriod [0x86F678] (i32 1000), posChangeTolerance
  [0xC18CF0] (f32 .bss global, runtime-initialised, unnamed in gr))` 0x65C430; GIVE-B (EBX = SP0).

### 0x490EF7 05DE COMMAND_TASK_WANDER_STANDARD  (gr: unregistered, used: 0 / 28 / 12)
- `Collect(1)`; `t = new(0x38)`; if non-null: `dir = CGeneral::GetRandomNumberInRange(0, 8)` 0x407180 (the ORIGINAL int version;
  gr's differs, see docs/re/cat14-gr-deviations.md finding 1) - called only when the allocation succeeded;
  `CTaskComplexWanderStandard(t, moveState 4, dir, bWanderSensibly 1)` 0x48E4F0. GIVE-A (EBP = SP0).

### 0x4912F4 0603 COMMAND_TASK_GO_TO_COORD_ANY_MEANS  (gr: unregistered, used: 0 / 117 / 0)
- `Collect(6)`: SP0 ped (EBX), SP1..SP3 target, SP4 move state (EBP), SP5 vehicle handle. `v = SP5 >= 0 ? VEH(SP5) : null` (signed,
  0x49132F). `t = new(0x34) CTaskComplexGoToPointAnyMeans(SP4, &target, v, radius [0xC18DB4] (f32 .bss global, runtime-initialised),
  modelId -1)` 0x66B790; GIVE-B.

### 0x490DEE 05DC COMMAND_TASK_SMART_FLEE_POINT  (gr: unregistered, used: 0 / 10 / 0)
- `Collect(6)`: SP0 ped (EBP), SP1..SP3 point, SP4 safe distance (f32), SP5 time. `t = new(0x44) CTaskComplexSmartFleePoint(&point,
  scream 1, SP4, SP5)` 0x65BD20; GIVE-A.

### 0x4928AD 0637 COMMAND_TASK_GO_TO_COORD_WHILE_SHOOTING  (gr: unregistered, used: 0 / 50 / 0)
- `Collect(8)`: SP0 ped (EBP), SP1..SP3 target, SP4 move state, SP5 target radius, SP6 slow-down distance, SP7 aim-at ped.
  `aim = PED(SP7)` (evaluated before `new`; may be null).
- `t = new(0x38) CTaskComplexGoToPointShooting(SP4, &target, aim, CVector(0,0,0) by value, SP5, SP6)` 0x668C70; GIVE-A.

### 0x491FFF 0622 COMMAND_TASK_LEAVE_CAR_IMMEDIATELY  (gr: unregistered, used: 0 / 22 / 1)
- `Collect(2)`; `v = VEH(SP1)`; `t = new(0x34) CTaskComplexLeaveCar(v, door 0, delay 0, sensible 0, forceGetOut 0)` 0x63B8C0; GIVE-A
  (EBP = SP0).

### 0x49283D 0635 COMMAND_TASK_AIM_GUN_AT_CHAR  (gr: unregistered, used: 0 / 22 / 0)
- `Collect(3)`: SP0 ped (EBP), SP1 target ped, SP2 duration. `e = PED(SP1)`; `t = new(0x3C) CTaskSimpleGunControl(e, targetPos null,
  moveTarget null, firingTask 0, burstAmmoCnt 1, leisureDurMs SP2)` 0x61F3F0 (the two CVector references are passed as NULL
  pointers - gr's signature uses references; the port must pass "no vector"); GIVE-A.

### 0x4923BA 0631 COMMAND_SET_GROUP_MEMBER  (gr: unregistered, used: 0 / 138 / 8)
- `Collect(2)`: SP0 group handle, SP1 ped. `g = GetActualScriptThingIndex(SP0, 8)`; `ped = PED(SP1)` (unchecked); if not
  `0 <= g < 8` -> AL = 0.
- `t = new(0x28) CTaskComplexBeInGroup(g, isLeader 0)` 0x632E50 (null -> 0); local `CEventScriptCommand ev(3, t, 0)` 0x4B0A00;
  `ped->m_pIntelligence->m_eventGroup (+0x68).Add(&ev, 0)` 0x4AB420.
- `m = &ms_groups[g].m_groupMembership`. If `m->CountMembersExcludingLeader() >= 7` 0x5F6AA0 and `m->GetLeader()` 0x5F69A0 is
  non-null and `IsPlayer()` 0x5DF8F0: `m->RemoveNFollowers(1)` 0x5FB1D0. `m->AddFollower(ped)` 0x5F8020; `ms_groups[g].Process()`
  0x5FC7E0.
- Leader car follow-up: `L = m->GetLeader()`; if null -> done. `veh = (L[+0x46C] & 0x100 && L->m_pVehicle) ? L->m_pVehicle :
  (task = L->m_pIntelligence->FindTaskByType(0x2BD /*701 ENTER_CAR_AS_DRIVER*/) 0x600EE0) ? task[+0xC] : null`; if null -> done.
  Scan `veh->m_apPassengers[0 .. maxPassengers-1]` for the first null; none -> done.
- With a free seat: local `CEventLeaderEnteredCarAsDriver evL(veh)` 0x48E1C0; `CEventEditableResponse::ComputeResponseTaskType(&evL,
  &ms_groups[g])` 0x4B57A0; `r = (i16) evL+0xE` (response task type):
  - `r == 0x5E8` (1512 TASK_GROUP_ENTER_CAR): local `CTaskComplexEnterCarAsPassenger tp(veh, seat 0, carryOn 1)` 0x640340;
    `gi = &ms_groups[g].m_groupIntelligence`; `gi->SetScriptCommandTask(ped, &tp)` 0x5F8560. If `!(ped[+0x474] & 0x2000000)`:
    set that bit; local `CTaskComplexSequence seq` 0x632BD0; `seq.AddTask(new(0x60) CTaskSimpleCarDrive(veh, 0, 1) 0x63C340)`;
    `d = gi->GetTaskDefault(ped)` 0x5F86C0; if d: `seq.AddTask(d->Clone())` (vtable +4); `gi->SetDefaultTask(ped, &seq)` 0x5F8580;
    `seq` destroyed 0x6389F0. Then `tp` destroyed 0x640380.
  - `r == 0x5E9` (1513 TASK_GROUP_ENTER_CAR_AND_PERFORM_SEQUENCE): `e2 = CEvent::operator new(0x18)` 0x4B5620 +
    `CEventLeaderEnteredCarAsDriver(e2, veh)`; local `CEventGroupEvent ge(L, e2)` 0x4ADFD0; `gi->AddEvent(&ge)` 0x5F7470; `ge`
    destroyed 0x4AE070.
  - other: nothing. Then `evL` destroyed 0x48E220.
- Finally `ev` destroyed 0x4B0A50. AL = 0.

### 0x4922D8 0630 COMMAND_SET_GROUP_LEADER  (gr: unregistered, used: 0 / 15 / 1)
- `Collect(2)`; `g = GetActualScriptThingIndex(SP0, 8)`; `ped = PED(SP1)` (unchecked); if not `0 <= g < 8` -> AL = 0.
- If `!ped->IsPlayer()`: `t = new(0x28) CTaskComplexBeInGroup(g, isLeader 1)`; local `CEventScriptCommand ev(3, t, 0)`;
  `ped->m_pIntelligence->m_eventGroup.Add(&ev, 0)`; `ev` destroyed 0x4B0A50.
- `ms_groups[g].m_groupMembership.SetLeader(ped)` 0x5FB9C0; `ms_groups[g].Process()` 0x5FC7E0.

### 0x492258 062F COMMAND_CREATE_GROUP  (gr: unregistered, used: 0 / 20 / 1)
- `Collect(1)`: SP0 default task allocator type. `GetActualScriptThingIndex(CollectNoPC(), 8)` evaluated and discarded.
- `g = CPedGroups::AddGroup()` 0x5FB800 (returns -1 when all 8 are active - NOT checked: the following writes then hit
  ms_groups[-1]; original bug). `ms_groups[g].m_groupIntelligence.SetDefaultTaskAllocatorType(SP0)` 0x5FBB70;
  `byte [0xC09BF0 + g*0x2D4] <- 1`; `SP0 <- GetNewUniqueScriptThingIndex(g, 8)`; `Store(1)`; if `m_UsesMissionCleanup`:
  `MissionCleanUp.AddEntityToList(SP0, 5)`.

### 0x4926E9 0632 COMMAND_REMOVE_GROUP  (gr: unregistered, used: 1 / 20 / 1)
- `Collect(1)`; `g = GetActualScriptThingIndex(SP0, 8)`.
- If `0 <= g < 8`: if `ms_activeGroups[g]` and the leader exists and `IsPlayer()`: `CPedGroups::RemoveAllFollowersFromGroup(g)`
  0x5FB8A0 and RETURN immediately (AL = 0 at 0x49274C: neither RemoveGroup nor the mission-cleanup removal happen). Otherwise
  `CPedGroups::RemoveGroup(g)` 0x5FB870.
- Then (also for an invalid g) if `m_UsesMissionCleanup`: `MissionCleanUp.RemoveEntityFromList(SP0, 5)`.

### 0x491ABC 061D COMMAND_ADD_ATTRACTOR  (gr: unregistered, used: 0 / 2 / 60)
- `Collect(6)`: SP0..SP2 position, SP3 heading 1 (deg), SP4 heading 2 (deg), SP5 sequence handle.
  `GetActualScriptThingIndex(CollectNoPC(), 6)` evaluated and discarded. `slot = CScripted2dEffects::AddScripted2DEffect(-1.0f)`
  0x6FA7C0 (imm 0xBF800000). `seq = GetActualScriptThingIndex(SP5, 4)`.
- If slot or seq is outside `[0, 0x40)`: `SP0 <- -1; Store(1)` (0x491FE5). The allocated effect slot is not released (leak,
  original).
- `a = SP3 * [0x8595EC] (0.017453292f)`; `v1 = (-sin(a), cos(a), 0)` (x87 fsin/fcos on the same product, each stored f32);
  `v1.Normalise()` 0x59C910. Same for `v2` from SP4.
- `E = 0xC3AB00 + slot*0x40` (CScripted2dEffects::ms_effects): `E+0 <- SP0, SP1, SP2`; `E+0x10 <- v1`; `E+0x1C <- v1`;
  `E+0x28 <- v2`; `byte E+0xC <- 3`; `byte E+0x34 <- 7`; `ms_effectSequenceTaskIDs[slot] (0xC3A020 + slot*4) <- seq`.
- `SP0 <- GetNewUniqueScriptThingIndex(slot, 6)`; if `m_UsesMissionCleanup`: `MissionCleanUp.AddEntityToList(SP0, 7)`; `Store(1)`.

### 0x491C33 061E COMMAND_CLEAR_ATTRACTOR  (gr: unregistered, used: 0 / 4 / 6)
- `Collect(1)`; `i = GetActualScriptThingIndex(SP0, 6)`; if `0 <= i < 0x40`: `CScripted2dEffects::ms_activated[i] (0xC3A1A0) <- 0`.
  If `m_UsesMissionCleanup`: `MissionCleanUp.RemoveEntityFromList(SP0, 7)`.

### 0x49137C 0604 COMMAND_GET_HEADING_FROM_VECTOR_2D  (gr: unregistered, used: 0 / 26 / 12)
- `Collect(2)`: x SP0, y SP1. `h = CGeneral::GetATanOfXY(SP0, SP1)` 0x53CC70 (ST0) `* [0x859878] (57.295776f) - [0x85991C] (90.0f)`.
  `while (h < [0x858B50] (0.0f)) h += [0x859E2C] (360.0f)` (fcom + `test ah,5` - NaN exits; x87 keeps h in ST0 between steps).
  `SP0 <- h` (f32); `Store(1)`.

### 0x491404 0606 COMMAND_LOAD_PATH_NODES_IN_AREA  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(4)`: x1 SP0, y1 SP1, x2 SP2, y2 SP3; swap so x1 <= x2, then y1 <= y2 (`test ah,0x41; jne` - NaN not swapped).
  `CPathFind::MakeRequestForNodesToBeLoaded(&ThePaths 0x96F050, x1, x2, y1, y2)` 0x450D70.

### 0x491494 0607 COMMAND_RELEASE_PATH_NODES  (gr: unregistered, used: 0 / 2 / 0) - `CPathFind::ReleaseRequestedNodes(&ThePaths)` 0x44DD00.

### 0x49106E 05F1 / 0x4910CE 05F2 / 0x49112E 05F3 / 0x49118F 05F4 COMMAND_SET_CAR_ESCORT_CAR_LEFT / RIGHT / REAR / FRONT  (gr: unregistered, used: 05F1 0/1/0, 05F2 0/1/0, 05F3 0, 05F4 0)
- `Collect(2)`; `v = VEH(SP0)`, `t = VEH(SP1)` (both unchecked). `v->m_autoPilot.m_pTargetCar (+0x41C) <- t`;
  `t->RegisterReference(&v[+0x41C])` 0x571B70 (thiscall on t: a bad SP1 handle crashes). If `v->m_nCarMission` is not 0x39/0x3A:
  `<- 0x1D` (05F1) / `0x1E` (05F2) / `0x1F` (05F3) / `0x20` (05F4).

### 0x491289 05F7, 05F9, 05FA, 05FF, 0600 COMMAND_IS_CHAR_[STOPPED_]IN_ANGLED_AREA_*  (gr: unimplemented, used: 0)
- Case shared by 05F6-0601: `this->CharInAngledAreaCheckCommand(commandId)` 0x487F60 (thiscall, EBX pushed; separate function, reads
  7 (2D) / 8 (3D) parameters and sets the compare flag). AL = 0. gr returns -1 for these five (deviation).

- gta-reversed: none of the unregistered opcodes has gr code. Open questions: 0x492A95 (four `nop`s before the stub jump) and the
  `nop; nop; jmp 0x4015FB` detour are no-ops; the stub's `mov [esp+0x23C], -1` is the SEH state reset (no behaviour beyond SEH).
