# CRunningScript::ProcessCommands1700To1799 (SCRIPT) - opcode specs

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
  CVehicle +0x22 m_nModelIndex (i16), +0x36 status bits 3-7, +0x390 m_autoPilot (+0x3AC timer, +0x3BA m_nCarMission, +0x3D0 m_nCruiseSpeed). `SetEngineOn` = 0x41BDD0 (see ProcessCommands700To799.md).

- CRunningScript fields: +0x10 m_BaseIP, +0x14 m_IP, +0x3C m_LocalVars[34], +0xC5 m_CondResult, +0xC6 m_UsesMissionCleanup,
  +0xC7 m_IsExternal, +0xC9 m_ExternalType. `MissionCleanUp` = CTheScripts::MissionCleanUp 0xA90850 (CMissionCleanup;
  AddEntityToList 0x4637E0 / RemoveEntityFromList 0x4654B0, thiscall (handle, type: 1 car, 2 char, 3 object)).
- gr `unimplemented` = REGISTER_COMMAND_UNIMPLEMENTED -> notsa::script::detail::NotImplemented (CommandParser/Parser.hpp:68): reads no
  parameters and returns OR_INTERRUPT (-1). That equals the exe only for opcodes that reach the switch default; for an
  `unimplemented` opcode that HAS a case here, gr deviates (it skips the case and leaves the IP on the parameters).
- "Protected store" stub 0x4021A4 (reached by `jmp 0x4021A4` at 0x498C8F): `pushfd; push esi; stc; esi = [0x1560728] (.HOODLUM,
  0x00C12FC2) + [0x5D720D] (.text, 0xFFE30CB5) + CF(1) = 0xA43C78; [esi] <- eax; pop esi; popfd; jmp 0x498C94` -> obfuscated
  `SP0 <- EAX` (anti-tamper code), then 0x498C94 `Store(1)`. Port as `SP0 = eax; Store(1)`. Cases that use it: "-> PSTORE" below.
- `VEHREF(v)` = 0x424160 (jmp stub to .HOODLUM 0x15667F0): CPool<CVehicle>::GetRef(v) = ((v - base) / 0xA18) << 8 | flags[idx]
  (magic 0x6572EC3, sar 6); no null handling.
- Model id rule ("UsedObjectArray"): `if (m < 0) m = *(i32*)(0xA44B88 - m*0x1C)`.

## 0x496E00 CRunningScript::ProcessCommands1700To1799  (catalogue: specified/REA, port level 36)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x496E2F), command id at [esp+4] kept in EBP (`mov ebp,[esp+0x1D4]` at 0x496E1D),
  `ret 4`. SEH frame `push -1; push 0x838D7D; push fs:[0]`, `sub esp,0x1BC`, push EBX/EBP/ESI/EDI; in cases the command id is also at
  `[esp+0x1DC]` and the SEH state slot at `[esp+0x1D4]`. Before the range check it initialises a function-scope CRect local
  (inlined default ctor): `[esp+0x74] <- 1000000.0f, [esp+0x80] <- 1000000.0f, [esp+0x7C] <- -1000000.0f, [esp+0x78] <- -1000000.0f`
  (imms 0x49742400 / 0xC9742400, 0x496E31-0x496E4C); only 0702/0703 use that local and they overwrite all four fields.
  Range: `eax = id - 0x6A4` (0x496E25), `cmp eax,0x63; ja 0x498EF1`, `jmp [eax*4 + 0x498F10]` (100 dwords).
- Default 0x498EF1: `or al,0xFF` (AL = -1). Epilogue 0x498EF3: `mov ecx,[esp+0x1CC]; pop edi/esi/ebp; fs:[0] <- ecx; pop ebx;
  add esp,0x1C8; ret 4`. Cases end `xor al,al; jmp 0x498EF3`. Task tail GIVE (0x496E9D `task = null` / 0x496E9F): `push ebp (command
  id); push eax (task); push ebx (ped handle); mov ecx,esi; nop x6; jmp 0x409B0D` -> .text stub `mov [esp+0x1E0], -1` (SEH state)
  `; jmp 0x496EAF` -> `GivePedScriptedTask(ped, task, commandId)` 0x465C20. Shared compare tail 0x4976B0 (`scratch = 1`) / 0x4976B5
  (`Cmp(scratch)`), 0x497580/0x497585 (same for 06B3/06EA).
- Table: 100 entries; 5 to the default (06B8 SET_GROUP_DEFAULT_TASK, 06CC, 06CD, 06D3 VAR_TEXT_LABEL16, 06D4 LVAR_TEXT_LABEL16; all gr
  unimplemented -> -1, same), 12 to the empty case 0x498EED (`xor al,al`, AL = 0, nothing read: 06A4 06A6 06C6 06CB 06CE 06F4 06F6
  06F7 06F9 06FB 06FE 0700; gr nop, except 06FE GET_CAR_VALUE which is gr-unregistered and so also reaches this empty case in gr),
  83 real cases. gta-reversed handler 20, nop 13 (not re-specified). Specified below: 62 unregistered (06FE included).
- No gta-reversed deviation in this range (no `handler`/`unimplemented` opcode with a mismatching exe path).

### 0x497E54 06D5 COMMAND_CREATE_CHECKPOINT / 0x497EF9 06D6 COMMAND_DELETE_CHECKPOINT  (gr: unregistered, used: 0 / 97 / 0 each)
- 06D5: `Collect(8)`: SP0 type, SP1..SP3 position, SP4..SP6 point-to, SP7 radius. `GetActualScriptThingIndex(CollectNoPC(), 3)` evaluated
  and discarded (its pushes are cleaned by the later `add esp,0x28`). `SP0 <- CTheScripts::AddScriptCheckpoint(SP1, SP2, SP3, SP4, SP5,
  SP6, SP7, SP0)` 0x4935A0 (cdecl 8 dwords: CVector at, CVector pointTo, radius, type). If `m_UsesMissionCleanup`:
  `MissionCleanUp.AddEntityToList(SP0, 0xC)` (via 0x4974EA). `Store(1)`.
- 06D6: `Collect(1)`; if `GetActualScriptThingIndex(SP0, 3) < 0` -> done; `CTheScripts::RemoveScriptCheckpoint(SP0)` 0x4936C0; if
  `m_UsesMissionCleanup`: `MissionCleanUp.RemoveEntityFromList(SP0, 0xC)`.

### 0x498A86 06FD COMMAND_SET_PLAYBACK_SPEED  (gr: unregistered, used: 0 / 95 / 0)
- `Collect(2)`; `CVehicleRecording::SetPlaybackSpeed(VEH(SP0), SP1 (f32))` 0x459660 (cdecl; null unchecked).

### 0x498BCF 0701 COMMAND_SKIP_CUTSCENE_END / 0x498ED8 0707 COMMAND_SKIP_CUTSCENE_START_INTERNAL  (gr: unregistered, used: 0 / 138 / 0 each)
- 0701: `this->m_SceneSkipIP (+0xD8) <- 0`. 0707: `Collect(1)`; `this->m_SceneSkipIP <- SP0` (raw).

### 0x497C50 06C7 COMMAND_TASK_CAR_TEMP_ACTION  (gr: unregistered, used: 0 / 76 / 7)
- `Collect(4)`: SP0 ped, SP1 vehicle (-1 -> null else `VEH(SP1)`), SP2 action, SP3 time. `t = new(0x68)
  CTaskSimpleCarSetTempAction(veh, SP2, SP3)` 0x63D6F0 (SEH 0xA); `GivePedScriptedTask(SP0, t, commandId [esp+0x1DC])`.

### 0x498101 06E1 COMMAND_TASK_CAR_MISSION  (gr: unregistered, used: 0 / 77 / 5)
- `Collect(6)`: SP0 ped, SP1 vehicle (`>= 0` -> VEH else null), SP2 target vehicle (`>= 0` -> VEH else null), SP3 mission, SP4 cruise
  speed (f32), SP5 driving style. `t = new(0x2C) CTaskComplexCarDriveMission(veh, target, SP3, SP5, SP4)` 0x63CC30 (SEH 0xB);
  `GivePedScriptedTask(SP0, t, commandId)`.

### 0x49707D 06A9 COMMAND_TASK_LOOK_AT_COORD  (gr: unregistered, used: 0 / 33 / 1)
- `Collect(5)`: SP0 ped or -1, SP1..SP3 position, SP4 time (`-1 -> 0x4E20` (20000), `-2 -> 0xF423F` (999999), else as is).
- If `SP0 != -1` (no task is created): `p = PED(SP0)` (unchecked); `IKChainManager_c::LookAt(&g_ikChainMan 0xC15448,
  "COMMAND_TASK_LOOK_AT_COO..." [0x85A3F0], p, target null, time, bone -1, &CVector(SP1, SP2, SP3), useTorso 0, speed 0.25f, blendTime
  500, priority 6, forceLooking 1)` 0x618970. AL = 0.
- If `SP0 == -1`: `t = new(0x30)`; ctor `FUN_00634440(t, entity null, time, bone -1, CVector(SP1, SP2, SP3), useTorso 1, 0.25f, 1000,
  priority 3)` (SEH 4; proposed name CTaskSimpleTriggerLookAt::CTaskSimpleTriggerLookAt, by argument shape); `ms_taskSequence
  [[0x8D2E98] CTaskSequences::ms_iActiveSequence].AddTask(t)` 0x632D10 (index unchecked; 0xC178F0 + idx*0x40).
  Note the two paths use different torso/blend/priority values (original).

### 0x498D79 0704 COMMAND_CAR_GOTO_COORDINATES_RACING  (gr: unregistered, used: 0 / 0 / 1)
- Same as 02C2 CAR_GOTO_COORDINATES_ACCURATE (ProcessCommands700To799.md) - ground z if `z <= -100.0f`, `z += GetDistanceFrom
  CentreOfMassToBaseOfModel`, `JoinCarWithRoadSystemGotoCoors(v, &pos, 0, 0)` - except the missions: joined -> `9`, not joined ->
  `0x21` (33) (both skipped when the mission is 0x39/0x3A). Then status `(b & 7) | 0x18`, `SetEngineOn(v, 1)`, cruise speed
  `max(cs, 1)`, `+0x3AC <- [0xB7CB84]`.

### 0x497F85 06D8 COMMAND_CREATE_MISSION_TRAIN  (gr: unregistered, used: 0 / 22 / 0)
- `Collect(5)`: SP0 train type, SP1..SP3 position, SP4 direction. `CTrain::CreateMissionTrain(CVector(SP1, SP2, SP3), SP4 != 0, SP0,
  &first, null, nodeIndex -1, trackId -1, isMissionTrain 1)` 0x6F7550 (cdecl 10 dwords). `SP0 <- VEHREF(first)` -> PSTORE.

### 0x4981BE 06E2 COMMAND_TASK_GO_TO_OBJECT  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(4)`: SP0 ped, SP1 object, SP2 time, SP3 radius (f32). `o = OBJ(SP1)`; `t = new(0x4C) CTaskComplexSeekEntity<Standard>(o, SP2,
  1000, SP3, [0x859E30] (2.0f), [0x859E34] (2.0f), 1, 1)` 0x46AC10 (SEH 0xC); `GivePedScriptedTask(SP0, t, commandId)`.

### 0x498259 06E3 COMMAND_TASK_WEAPON_ROLL  (gr: unregistered, used: 0 / 12 / 0)
- `Collect(2)`; `t = new(0x20) CTaskSimpleRunAnim(group 0, anim SP1 != 0 ? 0x3B : 0x39, blendDelta 4.0f, 0)` 0x61A8B0 (imm 0x40800000;
  SEH 0xD / 0xE); `GivePedScriptedTask(SP0, t, commandId)` (EBP).

### 0x4977E3 06BC COMMAND_FIRE_SINGLE_BULLET  (gr: unregistered, used: 0 / 4 / 0)
- `Collect(7)`; `CWeapon::FireOneInstantHitRound(&CVector(SP0, SP1, SP2), &CVector(SP3, SP4, SP5), SP6)` 0x73AF00 (cdecl).

### 0x497852 06BD COMMAND_IS_LINE_OF_SIGHT_CLEAR  (gr: unregistered, used: 0 / 10 / 2)
- `Collect(11)`; `Cmp(CWorld::GetIsLineOfSightClear(&CVector(SP0, SP1, SP2), &CVector(SP3, SP4, SP5), SP6 != 0, SP7 != 0, SP8 != 0,
  SP9 != 0, SP10 != 0, 0, 0))` 0x56A490 (buildings, vehicles, peds, objects, dummies, seeThrough 0, ignoreSomeObjects 0).

### 0x497281 06AD COMMAND_SET_GROUP_DECISION_MAKER  (gr: unregistered, used: 0 / 20 / 1)
- `Collect(2)`; `g = GetActualScriptThingIndex(SP0, 8)`; if `0 <= g < 8`: `dm = SP1 == -1 ? -1 : GetActualScriptThingIndex(SP1, 7)`;
  `ms_groups[g].m_groupIntelligence (0xC09950 + g*0x2D4).SetGroupDecisionMakerType(dm)` 0x5F7340.

### 0x4972E2 06AE COMMAND_LOAD_GROUP_DECISION_MAKER  (gr: unregistered, used: 0 / 11 / 0)
- `Collect(1)`; `CDecisionMakerTypesFileLoader::GetGrpDMName(SP0, buf)` 0x600880; `GetActualScriptThingIndex(CollectNoPC(), 7)`
  evaluated and discarded; `i = CDecisionMakerTypesFileLoader::LoadDecisionMaker(buf, 1 (group), (u8) m_UsesMissionCleanup)` 0x607D30;
  `SP0 <- GetNewUniqueScriptThingIndex(i, 7)`; `Store(1)`; if `m_UsesMissionCleanup`: `MissionCleanUp.AddEntityToList(SP0, 9)`.

### 0x497444 06B1 COMMAND_CREATE_SEARCHLIGHT / 0x4979B4 06C1 COMMAND_CREATE_SEARCHLIGHT_ON_VEHICLE  (gr: unregistered, used: 06B1 0/19/0, 06C1 0/7/0)
- 06B1: `Collect(8)`; `GetActualScriptThingIndex(CollectNoPC(), 2)` discarded; `h = CTheScripts::AddScriptSearchLight(SP0, SP1, SP2,
  entity null, SP3, SP4, SP5, SP6, SP7)` 0x493000 (cdecl: CVector start, CEntity*, CVector target, targetRadius, baseRadius).
- 06C1: `Collect(9)`; same discarded peek; `v = VEH(SP0)`; `h = AddScriptSearchLight(SP1, SP2, SP3, v, SP4, SP5, SP6, SP7, SP8)`.
- Both (tail 0x4974D5): `SP0 <- h`; if `m_UsesMissionCleanup`: `MissionCleanUp.AddEntityToList(SP0, 0xB)`; `Store(1)`.

### 0x497505 06B2 COMMAND_DELETE_SEARCHLIGHT  (gr: unregistered, used: 0 / 16 / 0)
- `Collect(1)`; if `GetActualScriptThingIndex(SP0, 2) < 0` -> done; `CTheScripts::RemoveScriptSearchLight(SP0)` 0x493160; if cleanup:
  `RemoveEntityFromList(SP0, 0xB)`.

### 0x49755D 06B3 COMMAND_DOES_SEARCHLIGHT_EXIST  (gr: unregistered, used: 0 / 17 / 0)
- `Collect(1)`; `Cmp(GetActualScriptThingIndex(SP0, 2) >= 0)`.

### 0x497598 06B4 COMMAND_MOVE_SEARCHLIGHT_BETWEEN_COORDS  (gr: unregistered, used: 0 / 7 / 0)
- `Collect(8)`; `CTheScripts::MoveSearchLightBetweenTwoPoints(SP0, SP1, ..., SP7)` 0x493360 (cdecl 8 dwords in order).

### 0x4975E5 06B5 / 0x49761E 06B6 / 0x497940 06BF COMMAND_POINT_SEARCHLIGHT_AT_COORD / _AT_CHAR / _AT_VEHICLE  (gr: unregistered, used: 06B5 0, 06B6 0/10/0, 06BF 0)
- 06B5: `Collect(5)`; `CTheScripts::MoveSearchLightToPointAndStop(SP0, SP1, SP2, SP3, SP4)` 0x493480.
- 06B6: `Collect(3)`; `CTheScripts::MoveSearchLightToEntity(SP0, PED(SP1), SP2)` 0x493420. 06BF: same with `VEH(SP1)`.

### 0x497978 06C0 COMMAND_IS_VEHICLE_IN_SEARCHLIGHT  (gr: unregistered, used: 0)
- `Collect(2)`; scratch = 0; `Cmp(CTheScripts::IsEntityWithinSearchLight(SP0, VEH(SP1)) 0x493900 != 0)`.

### 0x497BC7 06C4 COMMAND_ADD_BLIP_FOR_SEARCHLIGHT  (gr: unregistered, used: 0 / 5 / 0)
- `Collect(1)`; `GetActualScriptThingIndex(SP0, 2)` and `CRadar::GetActualBlipArrayIndex(CollectNoPC())` 0x582870 are both evaluated
  and discarded. `b = CRadar::SetEntityBlip(6, SP0, 0, 3, this->m_szName (this+8))` 0x5839A0; `CRadar::ChangeBlipScale(b, 3)` 0x583CC0;
  `SP0 <- b`; `Store(1)`.

### 0x497D54 06CA COMMAND_ATTACH_SEARCHLIGHT_TO_SEARCHLIGHT_OBJECT  (gr: unregistered, used: 0 / 18 / 0)
- `Collect(7)`; `CTheScripts::AttachSearchlightToSearchlightObject(SP0, OBJ(SP1), OBJ(SP2), OBJ(SP3), CVector(SP4, SP5, SP6))` 0x4934F0
  (lookups in order SP3, SP2, SP1; unchecked).

### 0x4976A2 06B9 COMMAND_HAS_CUTSCENE_LOADED  (gr: unregistered, used: 0 / 134 / 2)
- `Cmp([0xB5F84C] CCutsceneMgr::ms_cutsceneLoadStatus == 2)`.

### 0x49776F 06BB COMMAND_TASK_DRIVE_POINT_ROUTE  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(3)`: SP0 ped, SP1 vehicle (`< 0` -> null), SP2 cruise speed. `t = new(0x2C) CTaskComplexDrivePointRoute(veh, &route 0xC18D50,
  SP2, mode 2, model -1, -1.0f, drivingStyle 0)` 0x6433E0 (SEH 8); GIVE.

### 0x497A65 06C2 COMMAND_TASK_GO_TO_COORD_WHILE_AIMING  (gr: unregistered, used: 0 / 2 / 1)
- `Collect(11)`: SP0 ped, SP1..SP3 target, SP4 move state, SP5 target radius, SP6 slow-down distance, SP7 aim ped (`< 0` -> null else
  PED), SP8..SP10 aim position. `t = new(0x38) CTaskComplexGoToPointAiming(SP4, &target, aim, CVector(SP8, SP9, SP10), SP5, SP6)`
  0x668790 (SEH 9); GIVE.

### 0x497B75 06C3 COMMAND_GET_NUMBER_OF_FIRES_IN_RANGE  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(4)`; `SP0 <- CFireManager::GetNumFiresInRange(&gFireManager 0xB71F80, &CVector(SP0, SP1, SP2), SP3)` 0x5397F0 -> PSTORE.

### 0x497C26 06C5 COMMAND_SKIP_TO_END_AND_STOP_PLAYBACK_RECORDED_CAR  (gr: unregistered, used: 0 / 12 / 0)
- `Collect(1)`; `CVehicleRecording::SkipToEndAndStopPlaybackRecordedCar(VEH(SP0))` 0x45A4A0.

### 0x498E78 0705 COMMAND_START_PLAYBACK_RECORDED_CAR_USING_AI  (gr: unregistered, used: 0 / 6 / 0)
- `Collect(2)`; `CVehicleRecording::StartPlaybackRecordedCar(VEH(SP0), SP1, useCarAI 1, 0)` 0x45A980.

### 0x498EAA 0706 COMMAND_SKIP_IN_PLAYBACK_RECORDED_CAR  (gr: unregistered, used: 0 / 22 / 2)
- `Collect(2)`; `CVehicleRecording::SkipForwardInRecording(VEH(SP0), SP1 (f32))` 0x459D10.

### 0x497CDB 06C8 COMMAND_SET_LA_RIOTS  (gr: unregistered, used: 0 / 4 / 0) - `Collect(1)`; `[0xB72958] gbLARiots <- (SP0 != 0)`.
### 0x497DC8 06D0 COMMAND_SWITCH_EMERGENCY_SERVICES  (gr: unregistered, used: 0 / 34 / 4) - `Collect(1)`; `[0x8A5B28] CCarCtrl::bAllowEmergencyServicesToBeCreated <- (SP0 != 0)`.

### 0x497F51 06D7 COMMAND_SWITCH_RANDOM_TRAINS  (gr: unregistered, used: 0 / 33 / 0)
- `Collect(1)`; `CTrain::DisableRandomTrains(SP0 == 0)` 0x6F5DB0 (SP0 != 0 pushes 0, else 1).

### 0x498001 06D9 / 0x49800D 06DA / 0x498019 06DB COMMAND_DELETE_MISSION_TRAINS / MARK_MISSION_TRAINS_AS_NO_LONGER_NEEDED / DELETE_ALL_TRAINS  (gr: unregistered, used: 0/5/0, 0/4/0, 0/7/0)
- `CTrain::RemoveMissionTrains()` 0x6F6A20 / `CTrain::ReleaseMissionTrains()` 0x6F6B60 / `CTrain::RemoveAllTrains()` 0x6F6AA0.

### 0x498025 06DC / 0x49805A 06DD COMMAND_SET_TRAIN_SPEED / SET_TRAIN_CRUISE_SPEED  (gr: unregistered, used: 0/29/0, 0/46/0)
- `Collect(2)`; `CTrain::SetTrainSpeed(VEH(SP0), SP1)` 0x6F5E20 / `CTrain::SetTrainCruiseSpeed(VEH(SP0), SP1)` 0x6F5E50 (cdecl; f32).

### 0x498090 06DE COMMAND_GET_TRAIN_CABOOSE  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(1)`; `SP0 <- VEHREF(CTrain::FindCaboose(VEH(SP0)) 0x6F5E70)` -> PSTORE.

### 0x497908 06BE COMMAND_GET_CAR_ROLL  (gr: unregistered, used: 0 / 6 / 0)
- `Collect(1)`; `SP0 <- CAutomobile::GetCarRoll(VEH(SP0))` 0x6A6010 (ST0 stored f32); `Store(1)`.

### 0x498340 06E5 COMMAND_GET_AVAILABLE_VEHICLE_MOD  (gr: unregistered, used: 0 / 0 / 1)
- `Collect(2)`; `mi = ms_modelInfoPtrs[VEH(SP0)->m_nModelIndex]`; `SP0 <- (i32)(i16) mi[+0x2D6 + SP1*2]` (no bounds check); `Store(1)`.

### 0x498388 06E6 COMMAND_GET_VEHICLE_MOD_TYPE  (gr: unregistered, used: 0 / 0 / 1)
- `Collect(1)`; `m` = SP0 with the UsedObjectArray rule; `f = (u16) ms_modelInfoPtrs[m][+0x12]`; `t = (f >> 10) & 0x1F`.
- If `f & 0x100`: inner byte table 0x4990BC on `t - 1` (`ja` if > 0x15): t = 1 -> 11, 2 -> 12, 19 -> 13, 12 -> 14, 13 -> 15, 20/21/22
  -> 16 (jump table 0x4990A0).
- Else (`t > 0x11` -> default): jump table 0x4990D4: t = 0 -> 0, 1/2 -> 1, 6 -> 2, 8/9 -> 3, 10 -> 4, 11 -> 5, 12 -> 6, 14 -> 7, 15 -> 8,
  16 -> 9, 17 -> 10.
- A hit writes `SP0 <- value` then `Store(1)`. Every other t goes to 0x498C94 = `Store(1)` WITHOUT writing SP0, so the script receives
  its own input value back (original quirk).

### 0x4985A6 06E7 / 0x4985E4 06E8 COMMAND_ADD_VEHICLE_MOD / REMOVE_VEHICLE_MOD  (gr: unregistered, used: 06E7 0/4/1, 06E8 0)
- `Collect(2)`; `v = VEH(SP0)`; `m = SP1` (UsedObjectArray rule). 06E7: `SP0 <- CVehicle::AddVehicleUpgrade(v, m)` 0x6E3290 -> PSTORE.
  06E8: `CVehicle::RemoveVehicleUpgrade(v, m)` 0x6DF930.

### 0x498624 06E9 / 0x498654 06EA / 0x49868D 06EB COMMAND_REQUEST_VEHICLE_MOD / HAS_VEHICLE_MOD_LOADED / MARK_VEHICLE_MOD_AS_NO_LONGER_NEEDED  (gr: unregistered, used: 0/4/1, 0/2/1, 0/3/1)
- `Collect(1)`; `m` = SP0 (UsedObjectArray rule).
- 06E9: `CStreaming::RequestVehicleUpgrade(m, 0xC)` 0x408C70. 06EA: `Cmp(CStreaming::HasVehicleUpgradeLoaded(m) 0x407820 != 0)`.
- 06EB: `CStreaming::SetMissionDoesntRequireModel(m)` 0x409C90; `o = (i16) CLinkedUpgradeList::FindOtherUpgrade(&CVehicleModelInfo::
  ms_linkedUpgrades 0xB4E6D8, (i16)m)` 0x4C74D0; if `o != -1`: `SetMissionDoesntRequireModel(o)`.

### 0x4986DC 06EC COMMAND_GET_NUM_AVAILABLE_PAINTJOBS / 0x49870C 06ED COMMAND_GIVE_VEHICLE_PAINTJOB  (gr: unregistered, used: 06EC 0/0/1, 06ED 0/4/4)
- 06EC: `Collect(1)`; `SP0 <- CVehicleModelInfo::GetNumRemaps(ms_modelInfoPtrs[VEH(SP0)->m_nModelIndex])` 0x4C86B0 -> PSTORE.
- 06ED: `Collect(2)`; `CVehicle::SetRemap(VEH(SP0), SP1)` 0x6D0C00.

### 0x498801 06F0 COMMAND_SET_GROUP_SEPARATION_RANGE  (gr: unregistered, used: 0 / 12 / 0)
- `Collect(2)`; `g = GetActualScriptThingIndex(SP0, 8)`; if `0 <= g < 8`: `[0xC0994C + g*0x2D4] <- SP1` (raw f32; ms_groups[g] + 0x2C,
  separation range).

### 0x49888D 06F5 COMMAND_GET_SCRIPT_FIRE_COORDS  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(1)`; `i = GetActualScriptThingIndex(SP0, 5)`; if `0 <= i < 0x3C`: `p = CFireManager::GetScriptFireCoords(&gFireManager, i)`
  0x5397E0; `SP0..SP2 <- p`; else `SP0..SP2 <- 0`. `Store(3)`.

### 0x498900 06F8 COMMAND_GET_NTH_CLOSEST_CAR_NODE_WITH_HEADING  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(4)`: x, y, z (ground if `z <= -100.0f`, NaN skips), n SP3. `node = CPathFind::FindNthNodeClosestToCoors(&ThePaths, &ret,
  CVector(x, y, z), nodeType 0, maxDist 999999.875f, 0, 1, SP3 - 1, 0, 0, 0)` 0x44F8C0 (pushes 0x498951-0x498962).
  `FindNodeCoorsForScript(&ThePaths, &out, node, &found)` 0x4505E0.
- found: `SP0..SP2 <- out`, `SP3 <- ThePaths.FindNodeOrientationForCarPlacement(node)` 0x450320 (ST0, stored f32), flag = 1. Else
  `SP0..SP3 <- 0`, flag = 0. `Store(4)`; `Cmp(flag)`.

### 0x498A4B 06FC COMMAND_DOES_CAR_HAVE_STUCK_CAR_CHECK  (gr: unregistered, used: 0 / 5 / 7)
- `Collect(1)`; `Cmp(CStuckCarCheck::IsCarInStuckCarArray(&CTheScripts::StuckCars 0xA90AB0, SP0) 0x463C70 != 0)`.

### 0x498BE0 0702 COMMAND_GET_PERCENTAGE_TAGGED_IN_AREA / 0x498CA4 0703 COMMAND_SET_TAG_STATUS_IN_AREA  (gr: unregistered, used: 0702 0/13/0, 0703 0/14/0)
- `Collect(4)` (0702) / `Collect(5)` (0703): x1 SP0, y1 SP1, x2 SP2, y2 SP3. Swap x so x1 <= x2, then y so y1 <= y2 (`test ah,0x41;
  jne`, NaN not swapped; y1 is kept in ST0 throughout). Rect local at `[esp+0x74]`: `+0 = minX, +4 = maxY, +8 = maxX, +0xC = minY`.
- 0702: `SP0 <- CTagManager::GetPercentageTaggedInArea(&rect)` 0x49D0B0 -> PSTORE.
- 0703: `CTagManager::SetAlphaInArea(&rect, SP4 != 0 ? 0xFF : 0)` 0x49CFE0.

### 0x496E61 06A5 COMMAND_TASK_DIVE_FROM_ATTACHMENT_AND_GET_UP  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(2)`; `t = new(0x24) CTaskComplexDiveFromAttachedEntityAndGetUp(SP1)` 0x492E20 (SEH 0); GIVE (EBX = SP0).

### 0x4973B9 06B0 COMMAND_TASK_SIT_DOWN  (gr: unregistered, used: 0 / 14 / 0)
- `Collect(2)`; `t = new(0x14)` (always allocated first); ctor `CTaskComplexSitDownThenIdleThenStandUp(t, SP1 == -1 ? [0x86DB28]
  (i32 20000) : SP1, 0, 0)` 0x631460 (SEH 5 / 6); `GivePedScriptedTask(SP0, t, commandId [esp+0x1DC])`.

### 0x498EED 06FE COMMAND_GET_CAR_VALUE  (gr: unregistered, used: 0)
- Empty shared case: AL = 0, no parameters read, nothing stored (a PC no-op; gr also ends here).

- gta-reversed: none of these opcodes has gr code. Open questions: FUN_00634440 (06A9) proposed name; the gr names of the
  CTaskComplexSeekEntity ctor arguments (06E2) and of vehicle-model-info offsets +0x12 / +0x2D6 (06E5/06E6) are not validated (values
  and offsets are exact).
