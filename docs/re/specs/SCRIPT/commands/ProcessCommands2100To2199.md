# CRunningScript::ProcessCommands2100To2199 (SCRIPT) - script switch spec

Notation: `SPn` = `ScriptParams[n]` (0xA43C78 + 4*n). `veh(h)` / `ped(h)` / `obj(h)` = pool GetAtRef 0x4048E0 / 0x404910 /
0x465040 (ECX = CPools::ms_pVehiclePool 0xB74494 / ms_pPedPool 0xB74490 / ms_pObjectPool 0xB7449C); null when the slot flag
byte != h & 0xFF; "no null check" = result dereferenced directly. **In this switch `this` is EBP and EDI holds the command id.**
`GivePedScriptedTask(h, task, cmd)` = CRunningScript::GivePedScriptedTask 0x465C20 (thiscall, ECX = this; args ped handle, CTask*,
command id = EDI). Task allocation pattern: `CTask::operator new(size)` 0x61A5A0, null -> task = null (ctor skipped), SEH state
[esp+0x170] set around the constructor and reset to -1 before the call.
Compare-flag tails: 0x470F5B (`flag = 1` then) 0x470F60 UpdateCompareFlag([esp+0x10]); 0x47213F `test al,al; je 0x472148`,
0x472143 `flag = 1`, 0x472148 UpdateCompareFlag(flag). All cases return 0 through 0x47215A.

## 0x470A90 CRunningScript::ProcessCommands2100To2199  (catalogue: specified/REA, port level 18)
- Dispatcher: `__thiscall int8 (CRunningScript* this = ECX, int32 command [esp+4])`, `ret 4`. SEH frame (handler 0x83817E,
  0x470A90..0x470A9E), `sub esp,0x158`, push ebx/ebp/esi/edi; `edi = command` (0x470AAF), `ebp = this` (0x470ABF).
  `eax = command - 0x834` (0x470AB6), `cmp eax,0x63; ja 0x472158` (0x470ABC/0x470AC1): all of 2100..2199 dispatched;
  single table `jmp [eax*4+0x472178]` (0x470AC7).
- Default 0x472158: `or al,0xFF`, `jmp 0x4067C0` (.text stub: `mov ecx,[esp+0x168]; jmp 0x472161`), pop edi/esi/ebp,
  `mov fs:[0],ecx`, pop ebx, `add esp,0x164; ret 4` -> -1. Cases set AL (0) and jump to 0x47215A (same stub path).
- Opcodes: 98 with a case: 23 gta-reversed `handler`, 34 `nop`, 41 `unregistered` specified below. Of the gr-`nop` ones,
  30 use 0x472154 (`xor al,al`, no operand read) but **0868, 0869, 086B, 086C use 0x47175E: `GetIndexOfGlobalVariable()`
  (0x464700) is called, consuming one global-variable operand, then return 0** - a port must still advance m_IP over that
  operand (gr deviation if gr's nop does not).
- Default-reaching (2, gr `unimplemented`): 0854 BLOCK_NODES_IN_AREA, 0865 FREEZE_STATE_OF_INTERIORS.

### 0x47176C 0871 COMMAND_SWITCH_START, 0872 COMMAND_SWITCH_CONTINUED  (gr: unregistered, used: 0871 48/540/222, 0872 28/150/68)
- 0871 (EDI == 0x871, 0x47176C): CollectParameters(4) (0x471778); `CTheScripts::ValueToCheckInSwitchStatement [0xA43F5C] = SP0`;
  `NumberOfEntriesStillToReadForSwitch [0xA43F60] (u16) = (u16)(SP1 * 2)` (0x471792/0x4717A0); `SwitchDefaultExists [0xA43F58] =
  (SP2 != 0)`; `SwitchDefaultAddress [0xA43F54] = SP3`; `n = 14` (0x4717B2). 0872: `n = 18` (0x4717BC) (the `[esp+0x10]`
  fallback at 0x4717C0 is unreachable).
- `CollectParameters(n)` (0x4717D0; the 0871 values above are taken before this second collect, so SP0.. now hold pairs).
- If `N = NumberOfEntriesStillToRead > n` (u16 compare, `jbe` at 0x4717DC): for `i = 0; i < n; i += 2`:
  `CTheScripts::AddToSwitchJumpTable(SP[i], SP[i+1])` (0x470390, cdecl (value, address)); `N -= n` (0x47180F..0x47181D);
  if `N != 0` return 0 (0x472154); otherwise fall into the jump below.
  Else (`N <= n`): for `i = 0; i < N; i += 2`: AddToSwitchJumpTable(SP[i], SP[i+1]) (0x471849..0x47187D); `N = 0` (0x47187F).
- Jump (0x471829): `CTheScripts::UseSwitchJumpTable(&addr)` (0x4703C0, cdecl, writes the target into the local), then
  `this->UpdatePC(addr)` (0x464DA0). Return 0.
- gta-reversed: absent (these core control-flow opcodes are unregistered in gr and run the original switch). Open questions: none.

### 0x471D2B 0888 COMMAND_ADD_BLIP_FOR_DEAD_CHAR  (gr: unregistered, used: 0/1/0)
- CollectParameters(1). Dead `CRadar::GetActualBlipArrayIndex(CollectNextParameterWithoutIncreasingPC())` (0x582870, result
  unused). `b = CRadar::SetEntityBlip(2 /*BLIP_CHAR*/, SP0, 1, 3 /*display*/, this->m_szName (EBP+8))` (0x5839A0);
  `CRadar::ChangeBlipScale(b, 3)` (0x583CC0); `add esp,0x20`; SP0 = b, StoreParameters(1). Return 0.

### 0x4713B5 085B COMMAND_TASK_KINDA_STAY_IN_SAME_PLACE  (gr: unregistered, used: 0/54/0)
- CollectParameters(2): SP0 = ped handle or -1 (sequence), SP1 = flag.
- If `SP0 != -1`: `p = ped(SP0)`; stack `CTaskSimpleSetKindaStayInSamePlace t(SP1 != 0)` (0x62F610); `t.ProcessPed(p)` (0x62F660,
  p not null-checked); `~t` (0x62F650). Else: `task = new(0xC) CTaskSimpleSetKindaStayInSamePlace(SP1 != 0)`;
  `CTaskSequences::ms_taskSequence[CTaskSequences::ms_iActiveSequence].AddTask(task)` (ECX = 0xC178F0 + [0x8D2E98]*0x40,
  0x632D10; active sequence index not range-checked). Return 0.

### 0x4715B8 0866 COMMAND_GET_CLOSEST_STEALABLE_OBJECT  (gr: unregistered, used: 0/2/0)
- CollectParameters(4): SP0..SP2 = x/y/z, SP3 = radius. Output SP0 = object handle or -1.
- `best = SP3 + SP3` (fadd st0,st0). `CWorld::FindObjectsInRange(pos, SP3, true /*2D*/, &count, 16, list, false, false, false,
  true /*objects*/, false)` (0x564A20, 11 dwords). For i = 0 .. count-1 (unsigned): `e = list[i]`; require
  `([e+0x36] & 7) == 4` (object) and `[e+0x140] & 0x2000` (CObject flag bit 13); `d = CVector::Magnitude(pos(e) - pos)` (3D,
  components fsub-ed and stored f32 first, 0x47168A..0x4716D2); if `d < best` (ordered): best = d, chosen = e.
  `SP0 = chosen ? CPool<CObject>::GetRef(chosen) (0x465070) : -1`, StoreParameters(1). Return 0.
- Open question: gr name of CObject flag 0x2000 at +0x140 (0x47166A).

### 0x471EE8 088D COMMAND_SET_USES_COLLISION_OF_CLOSEST_OBJECT_OF_TYPE  (gr: unregistered, used: 0/11/4)
- CollectParameters(6): SP0..SP2 = x/y/z, SP3 = radius, SP4 = model (negative -> `*(int32*)(0xA44B88 + (-SP4)*0x1C)`), SP5 = flag.
- If `z <= -100.0f` ([0x859014], `test ah,0x41; jp`): `z = FindGroundZForCoord(x, y)` (no offset). `best = SP3 + SP3`.
  `CWorld::FindObjectsOfTypeInRange(model, pos, SP3, true, &count, 16, list, true /*buildings*/, false, false, true, true)`
  (0x564C70, 12 dwords). Nearest by 3D `CVector::Magnitude` (`d < best`, ordered; u16 loop counter); the chosen pointer is kept in
  EBP (overwriting `this`, harmless since only the epilogue follows). If found: `[e+0x1C] |= 1` when SP5 != 0 else `&= ~1`
  (bUsesCollision, 0x472063/0x472070). No LOD fallback (unlike 0363). Return 0.

### 0x47134B 085A COMMAND_CREATE_EMERGENCY_SERVICES_CAR  (gr: unregistered, used: 0/5/0)
- CollectParameters(4). UpdateCompareFlag(`CCarCtrl::ScriptGenerateOneEmergencyServicesCar(SP0, CVector(SP1,SP2,SP3))`) (0x42FBC0,
  cdecl 4 dwords; tails 0x470F5B/0x470F60). Return 0.

### 0x470FF5 0850 COMMAND_TASK_FOLLOW_FOOTSTEPS  (gr: unregistered, used: 0/7/1)
- CollectParameters(2). `t = new(0x20) CTaskComplexFollowPedFootsteps(ped(SP1))` (0x694E20); GivePedScriptedTask(SP0, t, 0x850).
### 0x4712DF 0859 COMMAND_TASK_WALK_ALONGSIDE_CHAR  (gr: unregistered, used: 0/0/1)
- CollectParameters(2). `t = new(0x20) CTaskComplexWalkAlongsidePed(ped(SP1), 10.0f /*imm 0x41200000*/)` (0x683240);
  GivePedScriptedTask(SP0, t, 0x859).
### 0x471E0C 088A COMMAND_TASK_PLAY_ANIM_WITH_FLAGS  (gr: unregistered, used: 0/1/0)
- `this->PlayAnimScriptCommand(0x88A)` (0x470150, thiscall, arg = EDI); all operand reading is inside that function. Return 0.

### 0x471470 085E COMMAND_START_PLAYBACK_RECORDED_CAR_LOOPED  (gr: unregistered, used: 0/3/0)
- CollectParameters(2). `CVehicleRecording::StartPlaybackRecordedCar(veh(SP0), SP1, false /*useCarAI*/, true /*looped*/)` (0x45A980).
### 0x47188A 0873 COMMAND_REMOVE_CAR_RECORDING  (gr: unregistered, used: 0/138/1)
- CollectParameters(1). `CVehicleRecording::RemoveRecordingFile(SP0)` (0x45A0A0).

### 0x47152E 0864 COMMAND_ENABLE_ENTRY_EXIT_PLAYER_GROUP_WARPING  (gr: unregistered, used: 0/8/0)
- CollectParameters(4): SP0/SP1 = x/y, SP2 = radius, SP3 = enable.
- `ee = CEntryExitManager::GetEntryExit(CEntryExitManager::FindNearestEntryExit(CVector2D(SP0,SP1), SP2, -1))` (0x43F4B0, 0x43EF00;
  ee not null-checked). If SP3: `ee->m_nFlags (u16 +0x30) |= 0x100` (bAcceptNpcGroup), and the same on `ee->m_pLink (+0x38)` when
  non-null; else `&= 0xFEFF` on both. Return 0.

### Simple cases
| case | opcode | name (used) | behaviour |
|---|---|---|---|
| 0x470B03 | 0835 | GET_CURRENT_DATE (0/0/8) | `SP0 = (u8)CClock::ms_nGameClockDays [0xB70154]`, `SP1 = (u8)ms_nGameClockMonth [0xB70155]`, StoreParameters(2). |
| 0x470E15 | 083F | GET_CAR_UPRIGHT_VALUE (0/4/0) | CollectParameters(1); `SP0 = veh(SP0)->m_matrix->up.z` (raw dword [m+0x28], no null checks), StoreParameters(1). |
| 0x470E5B | 0840 | SET_VEHICLE_AREA_VISIBLE (0/38/4) | CollectParameters(2); `[veh(SP0)+0x2F] = (u8)SP1` (m_nAreaCode). |
| 0x470E86 | 0841 | SELECT_WEAPONS_FOR_VEHICLE (0/2/0) | CollectParameters(2); `[veh(SP0)+0x513] = (u8)SP1`. |
| 0x470EB4 | 0842 | GET_CITY_PLAYER_IS_IN (0/4/1) | CollectParameters(1) (value ignored); `SP0 = CTheZones::m_CurrLevel [0xBA6718]`, StoreParameters(1). |
| 0x470F77 | 084D | ACTIVATE_INTERIOR_PEDS (0/2/0) | CollectParameters(1); `g_interiorMan (0xBAF670).ActivatePeds(SP0 != 0)` (0x598080). |
| 0x470FAA | 084E | SET_VEHICLE_CAN_BE_TARGETTED (0/1/0) | CollectParameters(2); `[veh(SP0)+0x42D]` bit 0x04 (bVehicleCanBeTargetted) set if SP1 != 0 else cleared. |
| 0x471161 | 0852 | SET_CAR_CAN_BE_VISIBLY_DAMAGED (0/19/0) | CollectParameters(2); `[veh(SP0)+0x868]` bit 0x10 set if SP1 != 0 else cleared (CAutomobile-only field; no type check). |
| 0x4711AB | 0853 | SET_HELI_REACHED_TARGET_DISTANCE (0/3/0) | CollectParameters(2); `[veh(SP0)+0x3DF] = (u8)SP1` (m_autoPilot + 0x4F). |
| 0x4711D9 | 0855 | GET_SOUND_LEVEL_AT_COORDS (0/2/0) | CollectParameters(4); `p = (SP0 == -1) ? null : ped(SP0)`; `SP0 = GetEventGlobalGroup()->GetSoundLevel(p, CVector(SP1,SP2,SP3))` (0x4ABA50 then 0x4AB900, ST0 stored f32), StoreParameters(1). |
| 0x4712AB | 0858 | SET_HEADING_FOR_ATTACHED_PLAYER (0/21/0) | CollectParameters(3) (SP0 unused); `CTheScripts::fCameraHeadingWhenPlayerIsAttached [0xA4449C] = SP1 * [0x8595EC]=0.017453292f`; `fCameraHeadingStepWhenPlayerIsAttached [0xA44498] = SP2 * 0.017453292f`. |
| 0x471504 | 0861 | IS_ATTACHED_PLAYER_HEADING_ACHIEVED (0/1/0) | CollectParameters(1) (unused); UpdateCompareFlag(`[0xA44498] == [0x858B50]=0.0f`) (`test ah,0x44; jp`, ordered equality). |
| 0x47173A | 0867 | IS_PROCEDURAL_INTERIOR_ACTIVE (0/1/0) | CollectParameters(1); UpdateCompareFlag(`g_interiorMan.IsGroupActive(SP0)` != 0) (0x598280). |
| 0x471916 | 0875 | SET_OBJECT_ONLY_DAMAGED_BY_PLAYER (0/46/0) | CollectParameters(2); `[obj(SP0)+0x40]` (CPhysical flags) bit 0x400000 set if SP1 != 0 else cleared. |
| 0x47195E | 0876 | CREATE_BIRDS (0/2/0) | CollectParameters(8); `CBirds::CreateNumberOfBirds(CVector((float)SP0,(float)SP1,(float)SP2), CVector((float)SP3,(float)SP4,(float)SP5), SP6, SP7, false)` (0x711EF0, 9 dwords). The six coordinates are converted with `fild` (read as INTEGERS): a float operand would be misread (original behaviour). |
| 0x471A00 | 0878 | SET_VEHICLE_DIRT_LEVEL (0/1/4) | CollectParameters(2); `[veh(SP0)+0x4B0] = SP1` (raw dword copy into m_fDirtLevel). |
| 0x471A2D | 0879 | SET_GANG_WARS_ACTIVE (0/10/0) | CollectParameters(1); `CGangWars::SetGangWarsActive(SP0 != 0)` (0x446570). |
| 0x471A61 | 087A | IS_GANG_WAR_GOING_ON (0/6/0) | No params; UpdateCompareFlag(`CGangWars::GangWarGoingOn()` != 0) (0x443AA0). |
| 0x471A70 | 087B | GIVE_PLAYER_CLOTHES_OUTSIDE_SHOP (0/8/0) | CollectParameters(1) (player); ReadTextLabelFromScript(tex, 16); ReadTextLabelFromScript(model, 16); CollectParameters(1) (body part); `CWorld::Players[player].m_PlayerData.m_pPedClothesDesc ([0xB7CD98 + player*0x190 + 8])->SetTextureAndModel(tex, model, SP0)` (0x5A8080); `CShopping::SetPlayerHasBought(CKeyGen::GetUppercaseKey(tex))` (0x53CF30, 0x49B610). |
| 0x471AF2 | 087C | CLEAR_LOADED_SHOP (0/1/2) | `CShopping::RemoveLoadedShop()` (0x49AE30). |
| 0x471AFE | 087D | SET_GROUP_SEQUENCE (0/1/0) | CollectParameters(2); `g = CTheScripts::GetActualScriptThingIndex(SP0, 8)` (0x4839A0); if `0 <= g < 8`: `s = (SP1 == -1) ? -1 : GetActualScriptThingIndex(SP1, 4)`; `*(int32*)(0xC09BEC + g*0x2D4) = s` (CPedGroups::ms_groups[g] (0xC09920) + 0x2CC). |
| 0x471C8F | 0884 | REGISTER_ATTRACTOR_SCRIPT_BRAIN_FOR_CODE_USE (0/0/0) | CollectParameters(1); `SP0 = (int16)CTheScripts::StreamedScripts (0xA47B60).GetProperIndexFromIndexUsedByScript((u16)SP0)` (0x470810); ReadTextLabelFromScript(name, 8); `CTheScripts::ScriptsForBrains (0xA90CF0).AddNewStreamedScriptBrainForCodeUse((u16)SP0, name, 5)` (0x46A9C0). |
| 0x471E1B | 088B | SET_VEHICLE_AIR_RESISTANCE_MULTIPLIER (0/6/0) | CollectParameters(2); `v = veh(SP0)` (no null check); `h = v->m_pHandlingData [+0x384]`; if h: `[v+0x98] = (h->m_fDragMult [+0x10] > [0x858C58]=0.01f) ? h->m_fDragMult / [0x859E80]=1000.0f * [0x858B8C]=0.5f : h->m_fDragMult` (stored f32), then `[v+0x98] = SP1 * [v+0x98]` (0x471E48..0x471E91). No handling -> nothing. |
| 0x471EA5 | 088C | SET_CAR_COORDINATES_NO_OFFSET (0/2/0) | CollectParameters(4); `CCarCtrl::SetCoordsOfScriptCar(veh(SP0), SP1, SP2, SP3, false, false)` (0x4342A0). |
| 0x47207D | 088E | SET_TIME_ONE_DAY_FORWARD (0/0/1) | `CClock::OffsetClockByADay(1)` (0x52D0B0). |
| 0x472090 | 0890 | SET_TIMER_BEEP_COUNTDOWN_TIME (0/3/0) | `i = GetIndexOfGlobalVariable()` (0x464700); CollectParameters(1); `CUserDisplay::OnscnTimer (0xBA1788).SetClockBeepCountdownSecs((u16)i, SP0)` (0x44CEE0). |
| 0x4720BB | 0893 | ATTACH_TRAILER_TO_CAB (0/5/0) | CollectParameters(2); `t = veh(SP0)`, `c = veh(SP1)`; if both non-null: `t->vtable[0xF4/4 = 61](c, true)` (CVehicle::SetTowLink). |
| 0x472100 | 0897 | IS_VEHICLE_TOUCHING_OBJECT (0/12/0) | CollectParameters(2); `v = veh(SP0)`, `o = obj(SP1)`; flag = 0; if v: flag = `CPhysical::GetHasCollidedWith(v, o)` (0x543540; o may be null); UpdateCompareFlag(flag). |
