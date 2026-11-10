# CRunningScript::ProcessCommands2200To2299 (SCRIPT) - script switch spec

Notation: `SPn` = `ScriptParams[n]` (0xA43C78 + 4*n). `veh(h)` / `ped(h)` / `obj(h)` = pool GetAtRef 0x4048E0 / 0x404910 /
0x465040 (ECX = CPools::ms_pVehiclePool 0xB74494 / ms_pPedPool 0xB74490 / ms_pObjectPool 0xB7449C), null when the slot flag
byte != h & 0xFF; "no null check" = result dereferenced directly (true for every case below unless stated). `this` = ESI.
"DUMMY" = the string at 0x859EE0. Used-object model for negative ids: `*(int32*)(0xA44B88 + (-m)*0x1C)`.
Vehicle flag bytes (gr CVehicle bitfields): +0x428 bit7 bFreebies; +0x42A bit5 bCanBeDamaged; +0x42D bit4 bWinchCanPickMeUp,
bit6 bVehicleCanBeTargettedByHS. All cases return AL = 0 via 0x476122 unless stated.

## 0x474900 CRunningScript::ProcessCommands2200To2299  (catalogue: specified/REA, port level 27)
- Dispatcher: `__thiscall int8 (CRunningScript* this = ECX, int32 command [esp+4])`, `ret 4`. SEH frame (handler 0x838269),
  `sub esp,0x168`, push ebx/ebp/esi/edi; `esi = this` (0x47491E); `ecx = command` (0x474920, still live in the cases: 08E3/08E4
  push it). `eax = command - 0x898` (0x474927), `cmp eax,0x63; ja 0x476120` (0x47492D/0x474931): all 2200..2299 dispatched;
  single table `jmp [eax*4+0x476140]` (0x474937).
- Default 0x476120: `or al,0xFF`; 0x476122: `nop; nop; jmp 0x401ADD` (.text stub `mov ecx,[esp+0x178]; jmp 0x476129`), pop
  edi/esi/ebp, `mov fs:[0],ecx`, pop ebx, `add esp,0x174; ret 4` -> -1.
- Opcodes: 100 with a case: 18 gta-reversed `handler`, 10 `nop` and 2 `unimplemented` (08A1 GET_CHAR_MAP_ATTRACTOR_STATUS,
  08CE) all sharing 0x47611C (`xor al,al`: no operand read, return 0; same as gr nop), 70 `unregistered` specified below.
  No opcode of this range reaches the default.

### 0x475244 08B4..08B9 COMMAND_IS_{GLOBAL,LOCAL}_VAR_BIT_SET_{CONST,VAR,LVAR}  (gr: unregistered, used: 08B4 77/3/135, 08B5 3/0/1, 08B6 8/0/2, 08B7 0/35/80, 08B8 0/0/0, 08B9 0/3/0)
- CollectParameters(2) (0x475248; the variable is read as a value). UpdateCompareFlag(`(SP0 & (1 << (SP1 & 31))) != 0`)
  (`shl edx,cl` masks the count to 5 bits, 0x475259..0x475268). Return 0.

### 0x475274 08BA..08BC COMMAND_SET_GLOBAL_VAR_BIT_{CONST,VAR,LVAR}  (gr: unregistered, used: 08BA 110/5/29, 08BB 1/0/1, 08BC 3/0/2)
- `p = GetPointerToScriptVariable(2)`, CollectParameters(1); `*p |= 1 << (SP0 & 31)`. Return 0.
### 0x4752A2 08BD..08BF COMMAND_SET_LOCAL_VAR_BIT_{CONST,VAR,LVAR}  (used: 0/15/82, 0/0/0, 0/5/0)
- Same with `GetPointerToScriptVariable(1)`.
### 0x4752CC 08C0..08C2 COMMAND_CLEAR_GLOBAL_VAR_BIT_{CONST,VAR,LVAR}  (used: 28/0/26, 9/0/1, 1/0/1)
- `p = GetPointerToScriptVariable(2)`, CollectParameters(1); `*p &= ~(1 << (SP0 & 31))`. Return 0.
### 0x4752FC 08C3..08C5 COMMAND_CLEAR_LOCAL_VAR_BIT_{CONST,VAR,LVAR}  (used: 0/3/39, 0/0/0, 0/0/0)
- Same with `GetPointerToScriptVariable(1)`.

### 0x47500D 08A9 COMMAND_STREAM_SCRIPT / 0x475049 08AB COMMAND_HAS_STREAMED_SCRIPT_LOADED  (gr: unregistered, used: 50/19/28, 57/8/0)
- CollectParameters(1). `i = (int16)CTheScripts::StreamedScripts (0xA47B60).GetProperIndexFromIndexUsedByScript((u16)SP0)` (0x470810);
  `SP0 = i` (written back). 08A9: `CStreaming::RequestModel(i + 0x6676, 4)` (0x4087E0; 0x6676 = 26230, first SCM model index).
  08AB: UpdateCompareFlag(`byte [0x964E08 + i*20] == 1`) (= CStreaming::ms_aInfoForModel[i + 26230].m_nLoadState == LOADED;
  tail 0x4760E5). Return 0.

### 0x475500 08D4 COMMAND_CREATE_MENU  (gr: unregistered, used: 2/13/110)
- ReadTextLabelFromScript(label, 8); `title[0] = 0`; if `_stricmp(label, "DUMMY") != 0` copy label into title (byte loop
  0x475530..0x47553B). CollectParameters(7): SP0 = x, SP1 = y, SP2 = width (f32), SP3 = columns, SP4 = interactive,
  SP5 = background, SP6 = alignment.
- `sx = (float)RsGlobal.maximumWidth [0xC17044] (fild) * [0x859520]=0.0015625f`; `x = SP0 * sx` (stored f32); `w = sx * SP2`
  (stored f32); `y = ((float)RsGlobal.maximumHeight [0xC17048] * [0x859524]=0.002232143f) * SP1` (0x475546..0x4755A2).
- `SP0 = (u8)CMenuSystem::CreateNewMenu(0, title, x, y, w, SP3, SP4, SP5, SP6)` (0x582300, cdecl 9 dwords, movzx), StoreParameters(1).

### 0x4756B4 08DB COMMAND_SET_MENU_COLUMN  (gr: unregistered, used: 3/24/48)
- CollectParameters(2): SP0 = menu, SP1 = column. Then 13 text labels (8 chars each): label A, label B, and 11 rows. For each,
  ReadTextLabelFromScript(tmp, 8) and copy it into its buffer only when `_stricmp(tmp, "DUMMY") != 0`; buffers are pre-cleared
  (`buf[0] = 0`; rows are 11 entries of 10 bytes at [esp+0xF0], 0x47574D..0x475767).
- `CMenuSystem::InsertMenu(SP0, SP1, A, B, row0, ..., row10)` (0x581E00, cdecl 15 dwords, `add esp,0x3C`). Return 0.

### 0x475DE4 08EE COMMAND_SET_MENU_ITEM_WITH_NUMBER / 08EF COMMAND_SET_MENU_ITEM_WITH_2_NUMBERS  (gr: unregistered, used: 1/18/17)
- CollectParameters(3): menu, column, row; ReadTextLabelFromScript(label, 8) (copied to a second buffer). If command == 0x8EE:
  CollectParameters(1), `n2 = -1`; if 0x8EF: CollectParameters(2), `n2 = SP1`. `CMenuSystem::InsertOneMenuItemWithNumber(menu,
  column, row, label, SP0, n2)` (0x581D70). Return 0.

### Menu helpers (simple)
| case | opcode | name (used) | behaviour |
|---|---|---|---|
| 0x4755D0 | 08D6 | SET_MENU_COLUMN_ORIENTATION (0/4/40) | CollectParameters(3); `CMenuSystem::SetColumnOrientation((u8)SP0, (u8)SP1, (u8)SP2)` (0x582080). |
| 0x475602 | 08D7 | GET_MENU_ITEM_SELECTED (2/0/12) | CollectParameters(1); `SP0 = (int8)CMenuSystem::CheckForSelected(SP0)` (0x5807E0, movsx); StoreParameters(1). |
| 0x475632 | 08D8 | GET_MENU_ITEM_ACCEPTED (0/0/8) | as 08D7 with `CheckForAccept` (0x5807C0). |
| 0x475661 | 08D9 | ACTIVATE_MENU_ITEM (0/5/7) | CollectParameters(3); `CMenuSystem::ActivateOneItem((u8)SP0, (u8)SP1, (u8)SP2)` (0x581B30). |
| 0x475693 | 08DA | DELETE_MENU (3/14/179) | CollectParameters(1); `CMenuSystem::SwitchOffMenu((u8)SP0)` (0x580750). |

### 0x474C0A 08A0 COMMAND_TASK_USE_CLOSEST_MAP_ATTRACTOR  (gr: unregistered, used: 0/0/7)
- Parameters: CollectParameters(6): SP0 = ped or -1, SP1 = radius, SP2 = model (0 = any, < 0 = used object), SP3..SP5 = x/y/z;
  then ReadTextLabelFromScript(name, 8).
- Behaviour: if `SP0 == -1`: `p = null`, `pos = (SP3, SP4, SP5)`; else `p = ped(SP0)`, `pos = p`'s position.
  `inUse = p ? CPedIntelligence::GetEffectInUse(p->m_pIntelligence [+0x47C]) : null` (0x6018D0). `model = SP2 == 0 ? -1 :
  (SP2 < 0 ? used-object model : SP2)`. `effect = entity = null`; call 0x6002F0 (unnamed, cdecl 9 args) `(&pos, SP1, model, 5,
  name, true, inUse, &effect, &entity)` (0x474CE6..0x474D10; proposed: find closest 2D-effect attractor of type 5 = script).
  If `effect == null`: UpdateCompareFlag(false), return 0 (0x474E2B).
  - If `this->m_ExternalType` (byte [esi+0xC9]) != 0xFF (script brain): `RemoveScriptFromList(&CTheScripts::pActiveScripts
    0xA8B42C)` (0x464BD0), `AddScriptToList(&pIdleScripts 0xA8B428)` (0x464C00), `ShutdownThisScript()` (0x465AA0); stack
    `CEventAttractor ev(effect, entity, true)` (0x4AF350), `word [ev+0xE] = 0xE9` (0x474D82), `p->m_pIntelligence->m_eventGroup
    (+0x68).Add(&ev, true)` (0x4AB420), `SetEffectInUse(p->m_pIntelligence, effect)` (0x6018E0), `~ev` (0x4AF400).
    **Returns AL = 1** (0x474DBA). p is not null-checked on this path (SP0 == -1 crashes).
  - Else: `task = new(0x24) CTaskComplexUseEffect(effect, entity)` (0x6321F0); `GivePedScriptedTask(SP0, task, command)`;
    UpdateCompareFlag(true). Return 0.
- Open question: identity/signature of 0x6002F0 (unnamed in the catalogue) and meaning of event word +0xE = 0xE9.

### 0x474F22 08A6 COMMAND_OPEN_CAR_DOOR_A_BIT  (gr: unregistered, used: 0/9/0)
- CollectParameters(3): SP0 = vehicle, SP1 = door, SP2 = ratio (f32). `v = veh(SP0)`; `n = CDamageManager::GetCarNodeIndexFromDoor(SP1)`
  (0x6C26F0, cdecl). If `v->vtable[0x98/4](SP1)` (door-missing query) is true or `CAutomobile::m_aCarNodes[n] ([v + 0x648 + n*4])`
  is null -> return 0. Else `v->vtable[0x6C/4](nullptr, n, SP1, SP2, true)` (CVehicle::OpenDoor). Return 0.
### 0x474F8C 08A7 COMMAND_IS_CAR_DOOR_FULLY_OPEN  (gr: unregistered, used: 0/1/0)
- CollectParameters(2). `n = GetCarNodeIndexFromDoor(SP1)`; UpdateCompareFlag(`veh(SP0)->vtable[0x84/4](n)`) - the virtual receives
  the car NODE index, not the door id (consistent with gr's `...U32` vtable variants; slot identity not cross-checked, open
  question at 0x474FBC).

### 0x475400 08CB COMMAND_EXPLODE_CAR_IN_CUTSCENE_SHAKE_AND_BITS  (gr: unregistered, used: 0/3/0)
- CollectParameters(4). `v = veh(SP0)`; `[v+0x42A] |= 0x20` (bCanBeDamaged); `v->vtable[0xA8/4 = 42](SP1 == 0, SP2 == 0, false,
  SP3 != 0)` (CVehicle::BlowUpCarCutSceneNoExtras(bNoCamShake, bNoSpawnFlyingComps, bDetachWheels, bExplosionSound)). Return 0.

### 0x475CB0 08EB COMMAND_ADD_SPARKS  (gr: unregistered, used: 0/17/0)
- CollectParameters(7): SP0..SP2 = position, SP3..SP5 = direction, SP6 = amount. `d = (SP3,SP4,SP5)`; `force =
  CVector::NormaliseAndMag(&d)` (0x59C970, ST0 -> f32). `g_fx (0xA9AE00).AddSparks(&pos, &d, force, SP6, CVector(0,0,0), 1,
  0.4f /*imm 0x3ECCCCCD*/, 1.0f /*imm 0x3F800000*/)` (0x49F040). Return 0.

### 0x475FA0 08F4 COMMAND_SET_SCRIPT_LIMIT_TO_GANG_SIZE  (gr: unregistered, used: 0/51/0)
- CollectParameters(1). `pd = CWorld::FindPlayerPed(-1)->m_pPlayerData (+0x480)`; `byte [pd+0x43] = (u8)SP0` (m_nScriptLimitToGangSize);
  `g = [pd+0x38]` (player group index; FindPlayerPed called again for each read); `m = &CPedGroups::ms_groups[g].m_groupMembership`
  (0xC09928 + g*0x2D4); `n = m->CountMembersExcludingLeader() - byte [pd+0x43]` (0x5F6AA0); if `n > 0`: `m->RemoveNFollowers(n)` (0x5FB1D0).

### 0x47603F 08F7 COMMAND_GET_CLOTHES_ITEM  (gr: unregistered, used: 0/0/38)
- CollectParameters(2): SP0 = player, SP1 = body part. `d = CClothes::GetTextureDependency(SP1)` (0x5A7EA0); `desc =
  CWorld::Players[SP0].m_PlayerData.m_pPedClothesDesc ([0xB7CD98 + SP0*0x190 + 8])`; `SP0 = [desc + 0x28 + SP1*4]` (texture key);
  `SP1 = [desc + d*4]` (model key); StoreParameters(2).

### 0x475838 08DC COMMAND_SET_BLIP_ENTRY_EXIT  (gr: unregistered, used: 6/94/0)
- CollectParameters(4). `CRadar::SetBlipEntryExit(SP0, CEntryExitManager::GetEntryExit(CEntryExitManager::FindNearestEntryExit(
  CVector2D(SP1, SP2), SP3, 0)))` (0x583F00, 0x43EF00, 0x43F4B0; ignore-area argument 0 here, -1 in 0864 of 2100To2199).

### Simple cases
| case | opcode | name (used) | behaviour |
|---|---|---|---|
| 0x47493E | 0898 | ENABLE_CRANE_CONTROLS (0/26/0) | CollectParameters(3); `CTheScripts::bEnableCraneRaise [0xA44496] = SP0 != 0`, `bEnableCraneLower [0xA44495] = SP1 != 0`, `bEnableCraneRelease [0xA44494] = SP2 != 0`. |
| 0x474E88 | 08A3 | CAN_TRIGGER_GANG_WAR_WHEN_ON_A_MISSION (0/16/0) | CollectParameters(1); `CGangWars::bCanTriggerGangWarWhenOnAMission [0x96AB93] = SP0 != 0`. |
| 0x474EA8 | 08A4 | CONTROL_MOVABLE_VEHICLE_PART (0/6/0) | CollectParameters(2); `CAutomobile::UpdateMovingCollision(veh(SP0), SP1)` (0x6A1460). |
| 0x474ED7 | 08A5 | WINCH_CAN_PICK_VEHICLE_UP (0/5/0) | CollectParameters(2); `[veh(SP0)+0x42D]` bit 0x10 set if SP1 != 0 else cleared. |
| 0x474FD9 | 08A8 | SET_ALWAYS_DRAW_3D_MARKERS (0/6/0) | CollectParameters(1); `C3dMarkers::ForceRender(SP0 != 0)` (0x722870). |
| 0x475085 | 08AC | SET_GANG_WARS_TRAINING_MISSION (0/1/0) | CollectParameters(1); `CGangWars::bTrainingMission [0x96AB91] = SP0 != 0`. |
| 0x4751A5 | 08B1 | SET_NIGHT_VISION (0/5/0) | CollectParameters(1); `CPostEffects::ScriptNightVisionSwitch(SP0 != 0)` (0x701120). |
| 0x4751D9 | 08B2 | SET_INFRARED_VISION (0/2/0) | CollectParameters(1); `CPostEffects::ScriptInfraredVisionSwitch(SP0 != 0)` (0x701140). |
| 0x4753B0 | 08C8 | ADD_PRICE_MODIFIER (0/0/5) | CollectParameters(2); `0x49BDD0(SP0, SP1)` (unnamed, cdecl; proposed CShopping::AddPriceModifier). |
| 0x4753D6 | 08C9 | REMOVE_PRICE_MODIFIER (0/0/1) | CollectParameters(1); `0x49ACD0(SP0)` (.HOODLUM 0x156AC70; proposed CShopping::RemovePriceModifier). |
| 0x475459 | 08D0 | IS_SKIP_CUTSCENE_BUTTON_PRESSED (0/31/0) | UpdateCompareFlag(`CCutsceneMgr::IsCutsceneSkipButtonBeingPressed() != 0`) (0x4D5D10). |
| 0x47547A | 08D1 | GET_CUTSCENE_OFFSET (0/0/0) | SP0..SP2 = CCutsceneMgr::ms_cutsceneOffset (0xBC4034/38/3C, raw dwords); StoreParameters(3). |
| 0x475891 | 08DD | SWITCH_DEATH_PENALTIES (2/0/0) | CollectParameters(1); `CGameLogic::bPenaltyForDeathApplies [0x8A5E48] = SP0 != 0`. |
| 0x4758B1 | 08DE | SWITCH_ARREST_PENALTIES (2/0/0) | CollectParameters(1); `CGameLogic::bPenaltyForArrestApplies [0x8A5E49] = SP0 != 0`. |
| 0x4758D0 | 08DF | SET_EXTRA_HOSPITAL_RESTART_POINT (2/0/0) | CollectParameters(5); `CRestart::ExtraHospitalRestartCoors (0xA43414/18/1C) = SP0..SP2`, `ExtraHospitalRestartRadius [0xA43258] = SP3`, `ExtraHospitalRestartHeading [0xA43254] = SP4` (raw dwords). |
| 0x475926 | 08E0 | SET_EXTRA_POLICE_STATION_RESTART_POINT (2/0/0) | Same with `ExtraPoliceStationRestartCoors (0xA43420/24/28)`, radius [0xA43250], heading [0xA4324C]. |
| 0x47597A | 08E1 | FIND_NUMBER_TAGS_TAGGED (1/0/0) | `SP0 = CTagManager::ms_numTagged [0xA9AD74]`; StoreParameters(1). |
| 0x475996 | 08E2 | GET_TERRITORY_UNDER_CONTROL_PERCENTAGE (3/0/0) | `SP0 = __ftol2(CGangWars::TerritoryUnderControlPercentage [0x96AB9C] * [0x858628]=100.0f)`; StoreParameters(1). |
| 0x4759BC | 08E3, 08E4 | IS_OBJECT_IN_ANGLED_AREA_2D/3D (0/1/0) | `this->ObjectInAngledAreaCheckCommand(command)` (0x4883F0, arg = ECX still holding the command id); all operand reads inside it. |
| 0x475B96 | 08E6 | SET_PLANE_UNDERCARRIAGE_UP (0/9/3) | CollectParameters(2); `SP1 ? CPlane::SetGearUp(v) (0x6CAC20) : CPlane::SetGearDown(v) (0x6CAC70)`, v = veh(SP0), no type check. |
| 0x475BD4 | 08E7 | DISABLE_ALL_ENTRY_EXITS (0/27/0) | CollectParameters(1); `CEntryExitManager::ms_bDisabled [0x96A7C8] = SP0 != 0`. |
| 0x475BF3 | 08E8 | ATTACH_ANIMS_TO_MODEL (4/0/0) | CollectParameters(1); ReadTextLabelFromScript(name, 8); model = SP0 (< 0 -> used-object model); `CTheScripts::ScriptAttachAnimGroupToCharModel(model, name)` (0x474800); `CTheScripts::AddToListOfSpecialAnimGroupsAttachedToCharModels(model, name)` (0x474750). |
| 0x475C40 | 08E9 | SET_OBJECT_AS_STEALABLE (0/3/0) | CollectParameters(2); `[obj(SP0)+0x140]` bit 0x2000 set if SP1 != 0 else cleared (flag tested by 0866). |
| 0x475C91 | 08EA | SET_CREATE_RANDOM_GANG_MEMBERS (0/10/0) | CollectParameters(1); `CPopulation::m_bDontCreateRandomGangMembers [0xC0FCB2] = (SP0 == 0)`. |
| 0x475D73 | 08EC | GET_VEHICLE_CLASS (1/1/3) | CollectParameters(1); `SP0 = (int8) byte [ms_modelInfoPtrs[(int16)[v+0x22]] + 0x4D]` (CVehicleModelInfo vehicle class, sign-extended); StoreParameters(1). |
| 0x475E76 | 08F0 | APPEND_TO_NEXT_CUTSCENE (0/0/14) | Two ReadTextLabelFromScript(…, 8); `CCutsceneMgr::AppendToNextCutscene(first, second)` (0x4D5DB0). |
| 0x475F0A | 08F2 | VEHICLE_CAN_BE_TARGETTED_BY_HS_MISSILE (0/9/0) | CollectParameters(2); `[veh(SP0)+0x42D]` bit 0x40 set if SP1 != 0 else cleared. |
| 0x475F55 | 08F3 | SET_FREEBIES_IN_VEHICLE (0/2/0) | CollectParameters(2); `[veh(SP0)+0x428]` bit 0x80 set if SP1 != 0 else cleared. |
| 0x47600F | 08F5 | MAKE_PLAYER_GANG_DISAPPEAR (0/79/0) | `CPlayerPed::MakePlayerGroupDisappear(CWorld::FindPlayerPed(-1))` (0x60A440). |
| 0x476027 | 08F6 | MAKE_PLAYER_GANG_REAPPEAR (0/83/0) | `CPlayerPed::MakePlayerGroupReappear(FindPlayerPed(-1))` (0x60A4B0). |
| 0x47608E | 08F8 | SHOW_UPDATE_STATS (4/10/36) | CollectParameters(1); `CStats::bShowUpdateStats [0x8CDE56] = SP0 != 0`. |
| 0x4760FC | 08FB | SET_COORD_BLIP_APPEARANCE (0/90/0) | CollectParameters(2); `CRadar::SetCoordBlipAppearance(SP0, (u8)SP1)` (0x583E50); falls into 0x47611C (return 0). |
| 0x47611C | 08A1, 08CE (+ gr-nop 0899, 089A, 089D, 08AA, 08AE, 08B0, 08CC, 08CD, 08CF, 08D5) | GET_CHAR_MAP_ATTRACTOR_STATUS etc. (0/0/0) | No operands read, return 0 (gr: unimplemented stubs / nop). |
