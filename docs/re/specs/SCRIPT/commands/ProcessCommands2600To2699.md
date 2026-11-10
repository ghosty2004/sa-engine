# CRunningScript::ProcessCommands2600To2699 (SCRIPT) - opcode specs

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
- Memory stores are written `[0xADDR] <- value`.
- Vtable slots (index = offset/4): CEntity 0 dtor, 1 Add(rect), 2 Add, 3 Remove, 4 SetIsStatic, 5 SetModelIndex, 6
  SetModelIndexNoCreate, 7 CreateRwObject, 8 DeleteRwObject, 9 GetBoundRect, 10 ProcessControl, 11 ProcessCollision, 12
  ProcessShift, 13 TestCollision, 14 Teleport, 15 SpecialEntityPreCollisionStuff, 16 SpecialEntityCalcCollisionSteps, 17 PreRender,
  18 Render, 19 SetupLighting, 20 RemoveLighting, 21 FlagToDestroyWhenNextProcessed, 22 ProcessEntityCollision (CPhysical);
  CVehicle slot 50 (+0xC8) = Fix (CAutomobile vtable 0x871120 slot 50 = 0x6A3440 CAutomobile::Fix).

## 0x479DA0 CRunningScript::ProcessCommands2600To2699  (catalogue: specified/REA, port level 10)
- Dispatcher: `__thiscall` (this = ECX CRunningScript*, copied to EDI at 0x479DCB), command id at [esp+4], `ret 4` (0x47A6C0).
  SEH frame: `push -1; push 0x8382AE; push fs:[0]` (0x479DA0-0x479DAD), `fs:[0] <- esp` at 0x479DB2; locals `sub esp,0x1C4`
  (0x479DB9), then pushes EBX/EBP/ESI/EDI. Inside cases the command id is `[esp+0x1E4]` and the SEH state slot `[esp+0x1DC]`.
  Range check: `eax = id - 0xA28` (0x479DC2), `cmp eax,0x26; ja 0x47A6A6` (0x479DC7/0x479DCD), `jmp [eax*4 + 0x47A6C4]`
  (0x479DD3; 39-entry table at 0x47A6C4).
- Default 0x47A6A6: `or al,0xFF` (AL = -1). Common tail 0x47A6A8: `nop; nop; jmp 0x409B6C` -> .text stub
  `mov ecx,[esp+0x1D4]; jmp 0x47A6AF` (saved SEH link) -> `pop edi/esi/ebp; fs:[0] <- ecx; pop ebx; add esp,0x1D0; ret 4`
  (0x47A6AF-0x47A6C0). Every case below ends with `xor al,al; jmp 0x47A6A8` (AL = 0) unless stated.
- Shared tiny targets: 0x47A6A2 (`xor al,al`, empty case: table indices 0x0C 0A34, 0x10 0A38, 0x1A 0A42, all gr nop);
  0x47A68F (`Cmp(0)`, indices 0x21 0A49 IS_XBOX_VERSION, 0x25 0A4D IS_JAPANESE_VERSION, gr handlers);
  0x47A69C (`inc [0xA95190] CTheScripts::UnknownDebugStuff; AL = 0`, index 0x26 0A4E, gr handler DoDebugStuff).
- Opcodes: 39 with a case (table entries 0A28..0A4E, 36 distinct targets). gta-reversed handler: 22 (+ 0A4E listed under two names), nop: 3 (not re-specified).
  Specified below (gr unregistered): 0A2A, 0A2E, 0A30, 0A31, 0A35, 0A36, 0A37, 0A3A, 0A3B, 0A3F, 0A40, 0A41, 0A46, 0A48 (14).
- Default-reaching: 0A4F COMMAND_FINISHED_WITH_XBOX_PLAYER2 (id - 0xA28 = 0x27 > 0x26). The catalogue marks it gr `handler`,
  but in the pinned gr source its REGISTER_COMMAND_HANDLER is inside `#if BUILD_XBOX` (Scripts/Commands/GameCommands.cpp:220-223),
  so a PC build of gr also forwards it to this switch and gets -1 (OR_INTERRUPT). Port: 0A4F returns -1 here.

### 0x479E75 0A2A COMMAND_IS_THIS_HELP_MESSAGE_BEING_DISPLAYED  (gr: unregistered, used: main 1 / missions 2 / streamed 63)
- Parameters: `ReadLabel(local8, 8)` (0x479E75-0x479E7E).
- Behaviour: `text = CText::Get(&TheText 0xC1B340, local8)` (0x479E8D); `result = false` (0x479E94).
  If `CHud::HelpMessageDisplayed()` 0x588B50 returns true (0x479E99/0x479EA0): `CMessages::StringCopy(buf400, text, 0x190)`
  0x69DB70 (wide string copy into a local, 400 chars), `CMessages::InsertPlayerControlKeysInString(buf400)` 0x69E160,
  `len = CMessages::GetStringLength(buf400)` 0x69DB50, `result = CMessages::StringCompare(buf400, &CHud::m_pHelpMessageToPrint
  0xBAA480, len)` 0x69DBD0 (all cdecl, cleaned together `add esp,0x20` at 0x479ED6). `Cmp(result)` (0x479EE4).

### 0x479F3D 0A2E COMMAND_TASK_FOLLOW_PATH_NODES_TO_COORD_WITH_RADIUS  (gr: unregistered, used: 0 / 1 / 0)
- Parameters: `Collect(7)`: SP0 ped handle, SP1..SP3 target x,y,z (f32, copied to a local CVector), SP4 move state (i32), SP5 time
  (i32), SP6 radius (f32).
- Behaviour: time: if `SP5 == -1` -> `[0x86FCAC]` (i32 50000 = 0xC350) (0x479F52/0x479F84); else if `SP5 == -2` -> -1 (0x479F8C/0x479F91).
  `task = CTask::operator new(0x60)` 0x61A5A0 (SEH state 0 during construction, 0x479FA4); if non-null:
  `CTaskComplexFollowNodeRoute::CTaskComplexFollowNodeRoute(task, moveState SP4, &target, tolerance SP6, slowDownDist
  [0x86FCA4] (3.0f), followNodeThresholdHeightChange [0x86FCA8] (2.0f), bKeepNodesHeadingAwayFromTarget 1, time, bUseBlending 1)`
  0x66EA30 (0x479FB1-0x479FD1); null -> task = 0 (0x479FD8). SEH state -1 (0x479FE6), then
  `this->GivePedScriptedTask(SP0, task, commandId [esp+0x1E4])` 0x465C20 (0x479FF1).

### 0x47A020 0A30 COMMAND_FIX_CAR  (gr: unregistered, used: 0 / 1 / 3)
- Parameters: `Collect(1)`: SP0 vehicle handle.
- Behaviour: `veh = VEH(SP0)` (0x47A035); virtual slot 50 `veh->Fix()` (`call [vtbl+0xC8]`, 0x47A040);
  `veh->m_fHealth (+0x4C0) <- 1000.0f` (imm 0x447A0000, 0x47A046).
- Edge: no null check: an invalid handle dereferences null at 0x47A03C (original crash).

### 0x47A057 0A31 COMMAND_SET_PLAYER_GROUP_TO_FOLLOW_NEVER  (gr: unregistered, used: 0 / 7 / 0)
- Parameters: `Collect(2)`: SP0 player index, SP1 flag.
- Behaviour: `CWorld::FindPlayerPed(SP0)` 0x56E210 (cdecl), `->ForceGroupToNeverFollow(SP1 != 0)` 0x60C800 (setne at 0x47A06E).
  No null check on the returned ped.

### 0x47A129 0A35 COMMAND_SET_UP_SKIP_FOR_VEHICLE_FINISHED_BY_SCRIPT  (gr: unregistered, used: 0 / 1 / 0)
- Parameters: `Collect(5)`: SP0..SP2 coords (f32), SP3 angle (f32), SP4 vehicle handle.
- Behaviour: `veh = VEH(SP4)` (0x47A15E, may be null; not checked); `CGameLogic::SetUpSkip(CVector(SP0,SP1,SP2), SP3,
  afterMission false, veh, finishedByScript true)` 0x4423C0 (cdecl, 7 dwords, `add esp,0x1C` at 0x47A18B; the `push 1` at 0x47A14D is
  the last argument).

### 0x47A195 0A36 COMMAND_IS_SKIP_WAITING_FOR_SCRIPT_TO_FADE_IN  (gr: unregistered, used: 0 / 1 / 0)
- No parameters. `Cmp(CGameLogic::IsSkipWaitingForScriptToFadeIn())` 0x4416C0 (0x47A195-0x47A1A5).

### 0x47A1B1 0A37 COMMAND_FORCE_ALL_VEHICLE_LIGHTS_OFF  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(1)`; `[0xC1CC18] CVehicle::ms_forceVehicleLightsOff <- low byte of SP0` (byte copy, not `!= 0`; 0x47A1BA/0x47A1BF).

### 0x47A1E7 0A3A COMMAND_IS_LAST_BUILDING_MODEL_SHOT_BY_PLAYER  (gr: unregistered, used: 0 / 3 / 0)
- Parameters: `Collect(2)`: SP0 player index, SP1 model id.
- Behaviour: `info = &CWorld::Players[SP0]` (0xB7CD98 + SP0*0x190, 0x47A1FB). `model = SP1`; if `SP1 < 0` (signed, 0x47A208):
  `model = *(i32*)(0xA44B88 - SP1*0x1C)` = CTheScripts::UsedObjectArray[-SP1] field +0x18 (model index; array base 0xA44B70, stride
  0x1C). `Cmp(info+0xA0 == model)` (gr name m_PlayerData.m_nModelIndexOfLastBuildingShot, 0x47A216-0x47A22F).

### 0x47A23B 0A3B COMMAND_CLEAR_LAST_BUILDING_MODEL_SHOT_BY_PLAYER  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(1)`; `[0xB7CE38 + SP0*0x190] <- -1` (CWorld::Players[SP0]+0xA0, 0x47A250).

### 0x47A498 0A3F COMMAND_SET_SCRIPT_COOP_GAME  (gr: unregistered, used: 0 / 3 / 0)
- `Collect(1)`; `[0x96A8A8] CGameLogic::bScriptCoopGameGoingOn <- (SP0 != 0)` (0x47A4A6-0x47A4AB).

### 0x47A4B8 0A40 COMMAND_CREATE_USER_3D_MARKER  (gr: unregistered, used: 5 / 0 / 0)
- Parameters: `Collect(4)`: SP0..SP2 x,y,z (f32), SP3 colour (i32).
- Behaviour: `r = C3dMarkers::User3dMarkerSet(SP0, SP1, SP2, SP3)` 0x720FD0 (cdecl, `add esp,0x10`); `SP0 <- r` (0x47A4FD); `Store(1)`.

### 0x47A50E 0A41 COMMAND_REMOVE_USER_3D_MARKER  (gr: unregistered, used: 8 / 0 / 0)
- `Collect(1)`; `C3dMarkers::User3dMarkerDelete(SP0)` 0x721090 (cdecl).

### 0x47A5A0 0A46 COMMAND_SWITCH_OBJECT_BRAINS  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(2)`; `CScriptsForBrains::SwitchAllObjectBrainsWithThisID(&CTheScripts::ScriptsForBrains 0xA90CF0, (u8)SP0, SP1 != 0)`
  0x46A900 (two copies of the call for SP1 true/false, 0x47A5C2 / 0x47A5D8; the ID is the zero-extended low byte of SP0).

### 0x47A5F5 0A48 COMMAND_ALLOW_PAUSE_IN_WIDESCREEN  (gr: unregistered, used: 0 / 0 / 2)
- `Collect(1)`; `[0xBA677C] <- (SP0 != 0)` (FrontEndMenuManager+0x34 = gr m_bMenuAccessWidescreen, 0x47A603-0x47A608).

- gta-reversed: none of the 14 opcodes above is registered (gr calls this original switch for them). Open questions: none.
