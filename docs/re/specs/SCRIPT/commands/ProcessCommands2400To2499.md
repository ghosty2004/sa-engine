# CRunningScript::ProcessCommands2400To2499 (SCRIPT) - script switch spec

Shared switch conventions (caller 0x469EB0/0x469F00, AL meaning, CRunningScript offsets, gr `unimplemented`
semantics): see ProcessCommands0To99.md, top section. Extra CRunningScript fields used here (gr RunningScript.h:241..243):
+0xC6 m_UsesMissionCleanup (bool), +0xC8 m_IsTextBlockOverride (bool; written by 09BD).
Helpers used below (all `__thiscall`, `ret 4`, ECX = pool object loaded from the pool global):
- 0x4048E0 vehicle handle lookup on `CPools::ms_pVehiclePool` [0xB74494] (gr `GetAtRef`): `i = h >> 8`; returns
  `objects + i*0xA18` if `flags[i] == (u8)h`, else null (0x4048E0..0x404904). 0x465040 = same for `CPools::ms_pObjectPool`
  [0xB7449C]; 0x404910 `CPedPool::GetAtRef` on `CPools::ms_pPedPool` [0xB74490]. "No null check" below means the case
  dereferences the result unconditionally (crash on a stale handle, as in the original).
- 0x41CC10 (jmp stub -> .HOODLUM 0x15638F0) vehicle pool slot accessor (gr `GetAt(index)`): `flags[i] < 0` (signed byte,
  slot free) -> null, else `objects + i*0xA18` (0x15638F7..0x156390A).
- 0x424160 (jmp stub -> 0x15667F0) vehicle pool `GetRef(veh)` (gr `GetRef`): `i = (veh - objects) / 0xA18` (magic
  0x6572EC3, `sar edx,6`, 0x15667FB..0x156680A); returns `(i << 8) + flags[i]`.
- "Entity position" = `m_matrix (+0x14) ? m_matrix + 0x30 : this + 4` (e.g. 0x4798D0..0x4798DC).
- "SetFlag(x)" for a byte set from a script int: `setne`/`sete` on the full dword ScriptParams value (`!= 0` / `== 0`).
  Where a bool is pushed as a full dword after `setne dl`/`setne cl`, the upper 24 bits are whatever the register held
  (e.g. 0x4782ED, 0x4794B0, 0x479577); callees only read the low byte, so the port passes a plain bool.

## 0x478000 CRunningScript::ProcessCommands2400To2499  (catalogue: specified/REA, port level 1)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x47802F; command at [esp+4]), `ret 4`. SEH frame
  `push -1; mov eax,fs:[0]; push 0x83828E; push eax; mov fs:[0],esp` (0x478000..0x47800E), `sub esp,0xC4` (0x478015),
  `push ebx; push ebp; push esi; push edi`. EDI = command ([esp+0xE4], 0x47801F) and stays live in the cases that do not
  overwrite it (used as the opcode argument by 099F at 0x47903F and 09A0 at 0x47905C). `eax = command - 0x960`
  (0x478026); `cmp eax,0x63; ja 0x479ADC` (0x47802C/0x478031); direct jump table `jmp [eax*4 + 0x479AFC]` (0x478037,
  100 entries, no byte index table).
- EH state dword is [esp+0xDC] (the `-1` pushed at 0x478000); only 099F changes it (0 at 0x47901E, -1 at 0x479045). The
  SEH handler 0x83828E unwinds to `CTask::operator delete` 0x61A5B0 (bundle callee list).
- Common exit 0x479ADE: `nop; nop; jmp 0x4074B7` -> 0x4074B7 `mov ecx,[esp+0xD4]` (saved SEH link) `; jmp 0x479AE5` ->
  `pop edi; pop esi; pop ebp; mov fs:[0],ecx; pop ebx; add esp,0xD0; ret 4` (0x479AE5..0x479AF6).
- Default 0x479ADC: `or al,0xFF` + common exit -> 0xFF. Shared "return 0" labels: 0x479AD8 (`xor al,al; jmp 0x479ADE`) and
  0x479606 (`xor al,al; jmp 0x479ADE`, inside the gr-handler case 09B6). No case in this range returns 1.
- Opcodes (100 = 0x960..0x9C3): all 100 jump-table entries point to a case (table dump 0x479AFC), so NO opcode reaches the
  default and defaults.csv has no row for 0x478000. gta-reversed `handler`: 23 (0961, 0965..0968, 0972, 0978, 0982, 0985,
  099C, 09A1, 09A4, 09A7, 09A8, 09AA, 09AD, 09AE, 09B5..09B7, 09BA, 09BC, 09C1; not re-specified); gr `nop`: 9 (0962, 0963,
  097E, 097F, 098F, 0990, 0999, 09A5, 09B1; they share 0x479AD8 = return 0 with no parameters, same as gr). Specified below:
  68 (66 unregistered + 0979, 0993 unimplemented). No gr `handler` opcode reaches the default.

### 0x479AD8 shared "return 0": 0962, 0963, 0979, 097E, 097F, 098F, 0990, 0993, 0999, 09A5, 09B1
- No parameter read, AL = 0 (0x479AD8). Also the target of 098C's overflow branch (0x478CDB) and the fall-through end of 09C3.
- 0979 COMMAND_LOAD_SHARED_CHAR_DECISION_MAKER and 0993 COMMAND_SET_CHAR_AIR_RESISTANCE_MULTIPLIER (gr: unimplemented,
  CharacterCommands.cpp:4478/4479, used 0/0/0 each): gr's NotImplemented returns OR_INTERRUPT (-1). DEVIATION in the return
  value only (original continues with the next command in the same frame); keep original. The other 9 are gr nops (same).

### 0x478805 097A COMMAND_REPORT_MISSION_AUDIO_EVENT_AT_POSITION  (gr: unregistered, used: 6/292/60)
- `CollectParameters(4)` (0x478809): [0] x, [1] y, [2] z (floats, raw dwords), [3] event id.
- Local `CVector pos = (P0, P1, P2)` (stores 0x47881F/0x47882C/0x47883A). `AudioEngine (0xB6BC90).ReportMissionAudioEvent(
  (u16)P3 (zero-extended, `mov dx,[0xA43C84]` 0x478825), &pos)` 0x507340 (0x47883F). Return 0.

### 0x47884B 097B COMMAND_REPORT_MISSION_AUDIO_EVENT_AT_OBJECT  (gr: unregistered, used: 8/81/59)
- `CollectParameters(2)`: [0] object handle, [1] event id. `obj = ObjectPool lookup(P0)` 0x465040 (no null check needed here;
  the pointer is only passed). `AudioEngine.ReportMissionAudioEvent((u16)P1, obj)` 0x507350 (CObject* overload, 0x478875).
  Return 0.

### 0x47949F 09B2 COMMAND_GET_RANDOM_CAR_MODEL_IN_MEMORY  (gr: unregistered, used: 0/77/0)
- `CollectParameters(1)`: [0] only-normal-cars flag. `m = CPopulation::m_AppropriateLoadedCars (0xC0E9F8).PickRandomCar(0,
  P0 != 0)` 0x611C50 (args pushed `edx` (setne, 0x4794B0) then `0`; gr signature `(bNotTooManyInTheWorld, bOnlyPickNormalCars)`).
- `ScriptParams[0] = m` (0x4794C3). If `m == -1` (0x4794C0): `ScriptParams[1] = -1` (0x4794CE). Else `ScriptParams[1] =
  (s32)(s8) ms_modelInfoPtrs[m]->+0x4D` (CVehicleModelInfo m_nVehicleClass, `movsx` 0x4794E6). `StoreParameters(2)` 0x464370.
  Return 0.

### 0x478ECB 0998 COMMAND_AWARD_PLAYER_MISSION_RESPECT  (gr: unregistered, used: 0/65/0)
- `CollectParameters(1)`; `CStats::IncrementStat(0xE0, (float)(s32)P0)` 0x55C180 (cdecl; `fild` 0x478ED4 -> `fstp` f32 on the
  stack 0x478EDB; stat 224 pushed 0x478EDE). Return 0.

### 0x478EA4 0997 COMMAND_SET_MISSION_RESPECT_TOTAL  (gr: unregistered, used: 1/0/0)
- `CollectParameters(1)`; `CStats::SetStatValue(0xE4, (float)(s32)P0)` 0x55A070 (cdecl, `fild` 0x478EAD, stat 228 0x478EB7).
  Return 0.

### 0x479536 09B4 COMMAND_SET_CLOSEST_ENTRY_EXIT_FLAG  (gr: unregistered, used: 1/60/0)
- `CollectParameters(5)`: [0] x, [1] y, [2] radius, [3] flag mask, [4] enable.
- Local `CVector2D p = (P0, P1)` (0x479552, 0x479560). `idx = CEntryExitManager::FindNearestEntryExit(&p, P2, -1)` 0x43F4B0
  (cdecl, pushes -1, P2, &p at 0x479550..0x47955B). Then `SetEntryExitFlagWithIndex(idx, P3, P4 != 0)` via 0x43EF90 (cdecl,
  0x47957D; `add esp,0x18` pops both calls at 0x479582). Return 0.
- 0x43EF90 is an uncatalogued `.HOODLUM` stub, specified at the end of this file. When no entry/exit is within range,
  `idx = -1` and the callee reads `flags[-1]` of the pool and may write through a null/garbage entry: original bug, keep.
- Dead store: [esp+0x48] = P2 (0x47955C) is never read.

### 0x478E13 0994 COMMAND_MARK_ROAD_NODE_AS_DONT_WANDER  (gr: unregistered, used: 0/49/0)
- `CollectParameters(3)`: x, y, z (raw floats). `ThePaths (0x96F050).MarkRoadNodeAsDontWander(P0, P1, P2)` 0x450560 (0x478E35).
  Return 0.

### 0x4797B6 09BD COMMAND_SET_MINIGAME_IN_PROGRESS  (gr: unregistered, used: 0/0/45)
- `CollectParameters(1)`. If P0 != 0 (0x4797C4): `this->+0xC8 = 1` (0x4797C8), `CTheScripts::bMiniGameInProgress [0xA444A8] = 1`,
  `CTheScripts::bDisplayNonMiniGameHelpMessages [0xA444A7] = 0`. Else (0x4797E4): `+0xC8 = 0`, `[0xA444A8] = 0`, `[0xA444A7] = 1`.
  Return 0.

### 0x479800 09BE COMMAND_IS_MINIGAME_IN_PROGRESS  (gr: unregistered, used: 20/14/3)
- No parameters. `UpdateCompareFlag(byte [0xA444A8])` 0x4859D0 (zero-extended, 0x479800..0x47980B). Return 0.

### 0x478D67 098E COMMAND_SET_NAMED_ENTRY_EXIT_FLAG  (gr: unregistered, used: 0/0/36)
- `ReadTextLabelFromScript(buf = [esp+0x4C], 8)` 0x463D50 (0x478D70); `CollectParameters(2)`: [0] flag mask, [1] enable.
- `CEntryExitManager::SetEntryExitFlag(buf, P0, P1 != 0)` 0x43EF20 (cdecl, 3 args, `add esp,0xC` 0x478D9B). Return 0.

### 0x478FD2 099F COMMAND_TASK_SET_IGNORE_WEAPON_RANGE_FLAG  (gr: unregistered, used: 0/33/0)
- `CollectParameters(2)`: [0] ped handle or -1, [1] flag.
- P0 != -1 (0x478FE0): `ped = CPedPool::GetAtRef(P0)` 0x404910 (no null check); direct flag write without a task:
  `d = ped->+0x478` (0x478FF8); `edx = (((u32)(u8)P1 << 9) ^ d) & 0x200` (0x478FF1..0x479003); then shared 0x479600
  `ped->+0x478 ^= edx` -> bit 9 of the dword at +0x478 becomes bit 0 of P1's low byte (P1 = 2 clears it). Returns 0 via
  0x479606. (gr names this bit bIgnoreWeaponRange through CTaskSimpleSetCharIgnoreWeaponRangeFlag; gr's Ped.h bitfield
  count is 2 bits off at this dword, so use the raw mask.)
- P0 == -1 (sequence building): `t = CTask::operator new(0xC)` 0x61A5A0; EH state [esp+0xDC] = 0 (0x47901E); if t != 0:
  `CTaskSimpleSetCharIgnoreWeaponRangeFlag::Constructor(t, (u8)P1)` 0x474620 (0x479036) else t = 0; EH state = -1 (0x479045);
  `this->GivePedScriptedTask(-1, t, command = 0x99F (EDI))` 0x465C20 (0x479050). Return 0.

### 0x478DC7 0992 COMMAND_SET_PLAYER_CYCLE_WEAPON_BUTTON  (gr: unregistered, used: 0/14/5)
- `CollectParameters(2)`: [0] pad/player index, [1] enable. `pad = CPad::GetPad(P0)` 0x53FB70 (cdecl). Byte `pad->+0x11D`
  (bDisablePlayerCycleWeapon) `= (P1 != 0) ? 0 : 1` (0x478DE5 / 0x478E02). Return 0.

### 0x47803E 0960 COMMAND_SET_PLAYER_DISPLAY_VITAL_STATS_BUTTON  (gr: unregistered, used: 0/3/0)
- Same shape as 0992 with byte `pad->+0x11F` (bDisablePlayerDisplayVitalStats) `= (P1 != 0) ? 0 : 1` (0x47805B / 0x478078).
  Return 0.

### 0x478C53 0989 COMMAND_SET_HELP_MESSAGE_BOX_SIZE  (gr: unregistered, used: 0/5/14)
- `CollectParameters(1)`; `CHud::m_fHelpMessageBoxWidth [0x8D0934] = (float)(s32)P0` (`fild` 0x478C5C, `fstp` f32 0x478C64).
  Return 0.

### 0x4786D6 0977 COMMAND_IS_OBJECT_WITHIN_BRAIN_ACTIVATION_RANGE  (gr: unregistered, used: 0/0/12)
- `CollectParameters(1)`; `obj = ObjectPool lookup(P0)` (0x4786EC). `flag = 0`. `p = CWorld::PlayerInFocus` byte [0xB7CD74];
  if `CWorld::Players[p].m_pPed` ([0xB7CD98 + p*0x190]) != 0 and obj != 0 (0x478708, 0x478711): `v = *CWorld::FindPlayerCentreOfWorld(p)`
  0x56E250 (cdecl; CVector copied to a local, 0x47871B..0x478739); `flag = CTheScripts::ScriptsForBrains (0xA90CF0)
  .IsObjectWithinBrainActivationRange(obj, &v)` 0x46B3D0 `!= 0`. `UpdateCompareFlag(flag)`. Return 0.

### 0x47858D 0971 COMMAND_SYNC_WATER  (gr: unregistered, used: 0/11/0)
- No parameters. Calls 0x6E76E0 (uncatalogued; gr `CWaterLevel::SyncWater`, WaterLevel.cpp:811):
  `CWaterLevel::m_nWaterTimeOffset [0xC228A4] = CTimer::m_snTimeInMilliseconds [0xB7CB84]` (0x6E76E0..0x6E76EA). Return 0.

### 0x478EF2 099A COMMAND_SET_CAR_COLLISION  (gr: unregistered, used: 0/11/0)
- `CollectParameters(2)`; `veh = vehicle lookup(P0)` (no null check). Reads `e = veh->+0x1C` (entity flags) and `f = veh->+0x40`
  (physical flags) first (0x478F13, 0x478F18). P1 != 0: `+0x1C = e | 1` (bUsesCollision), `+0x40 = f | 2`; else `+0x1C = e & ~1`,
  `+0x40 = f & ~2` (0x478F1D..0x478F39; +0x1C written before +0x40). gr names bit 1 of +0x40 `bApplyGravity` (Physical.h:71);
  name unverified, port the mask. Return 0.

### 0x478881 097C COMMAND_ATTACH_MISSION_AUDIO_TO_OBJECT  (gr: unregistered, used: 0/10/2)
- `CollectParameters(2)`: [0] slot (1-based), [1] object. `obj = ObjectPool lookup(P1)` (0x478897).
  `AudioEngine.AttachMissionAudioToObject((u8)((u8)P0 - 1), obj)` 0x507320 (`mov al,[P0]; dec al` with EAX upper bytes zero,
  0x47889D..0x4788A4). Return 0.

### 0x47936D 09AC COMMAND_HIDE_ALL_FRONTEND_BLIPS  (gr: unregistered, used: 0/10/0)
- `CollectParameters(1)`; `CTheScripts::HideAllFrontEndMapBlips [0xA444A1] = (P0 != 0)`. Return 0.

### 0x4785C6 0973 COMMAND_DOES_SCRIPT_FIRE_EXIST  (gr: unregistered, used: 0/9/0)
- `CollectParameters(1)`; `i = CTheScripts::GetActualScriptThingIndex(P0, 5)` 0x4839A0 (cdecl); `flag = (i >= 0 && i < 0x3C)`
  (signed, 0x4785DF..0x4785ED). `UpdateCompareFlag(flag)`. Return 0.

### 0x4782D9 096A COMMAND_SWITCH_POLICE_HELIS  (gr: unregistered, used: 0/7/0)
- `CollectParameters(1)`; `CHeli::SwitchPoliceHelis(P0 != 0)` 0x6C4800 (cdecl, 0x4782EE). Return 0.

### 0x47945B 09B0 COMMAND_SET_VEHICLE_IS_CONSIDERED_BY_PLAYER  (gr: unregistered, used: 0/6/0)
- `CollectParameters(2)`; `veh = vehicle lookup(P0)` (no null check); byte +0x42E bit 5 (vehicleFlags bit 53, bConsideredByPlayer)
  `= (P1 != 0)` (xor-merge 0x479483..0x479492). Return 0.

### 0x4780D1 0964 COMMAND_CREATE_MENU_GRID  (gr: unregistered, used: 0/0/5)
- Parameters: `ReadTextLabelFromScript(label = [esp+0x4C], 8)` (0x4780DA); `CollectParameters(7)` (0x478111): [0] x, [1] y,
  [2] width (floats, 640x448 units), [3] columns, [4] interactive, [5] background, [6] alignment. Output: StoreParameters(1).
- Title buffer `title = [esp+0x2C]`: `title[0] = 0` (0x4780E9); if `_stricmp(label, "DUMMY" [0x859EE0]) != 0` 0x8229B6 (cdecl,
  0x4780EE) the label is copied byte by byte up to and including its NUL (0x478100..0x47810B). If the 8-byte label has no NUL
  the copy runs on into the following stack bytes (original behaviour; ReadTextLabelFromScript decides termination).
- Floats (x87, PC24): `w = (float)RsGlobal.maximumWidth [0xC17044] * [0x859520] = 0.0015625` (0x478116/0x478128; kept in ST);
  `x = P0 * w` (0x478140) stored f32 (0x478145); `wd = w * P2` (0x47814D) stored f32 (0x478153);
  `y = (float)RsGlobal.maximumHeight [0xC17048] * [0x859524] = 0.0022321430 * P1` (0x47815B..0x47816C, multiply order
  height*k first, then *P1) stored f32 (0x478172).
- `id = CMenuSystem::CreateNewMenu(1, title, x, y, wd, P3, P4, P5, P6)` 0x582300 (cdecl, 9 dwords, `add esp,0x24` 0x478187;
  P3..P6 pushed as raw dwords). `ScriptParams[0] = (u8)id` (`movzx` 0x478184); `StoreParameters(1)`. Return 0.

### 0x478315 096D COMMAND_GET_CURRENT_CAR_MOD  (gr: unregistered, used: 1/0/1)
- `CollectParameters(2)`: [0] vehicle, [1] mod slot. `veh = vehicle lookup(P0)` (EDI; no null check). `eax = -1` (0x478337);
  `if ((u32)P1 > 16) -> store -1` (0x47833A); else `jmp [P1*4 + 0x479C8C]` (17 entries). Result r goes to ScriptParams[0],
  `StoreParameters(1)`, return 0. `GetUpgrade` = `CVehicle::GetUpgrade(id)` 0x6D3650, `GetRepl` = `CVehicle::GetReplacementUpgrade(node)` 0x6D3A50:

| P1 | target | r |
|----|--------|---|
| 0 | 0x47834A | GetUpgrade(0) |
| 1 | 0x478368 | a = GetUpgrade(1); a != -1 ? a : GetUpgrade(2) |
| 2 | 0x478398 | GetUpgrade(6) |
| 3 | 0x4783B6 | a = GetUpgrade(8); a != -1 ? a : GetUpgrade(9) |
| 4..10 | 0x4783E6, 0x478404, 0x478422, 0x478440, 0x47845E, 0x47847C, 0x47849A | GetUpgrade(10, 11, 12, 14, 15, 16, 17) |
| 11 | 0x4784D1 | -1 |
| 12..16 | 0x4784B8, 0x4784BC, 0x4784C0, 0x4784C4, 0x4784C8 | GetRepl(2, 0x13, 0xC, 0xD, 0x14) |

### 0x4784E6 096E COMMAND_IS_CAR_LOW_RIDER / 0x478534 096F COMMAND_IS_CAR_STREET_RACER  (gr: unregistered, used: 1/1/3, 0/0/3)
- `CollectParameters(1)`; `veh = vehicle lookup(P0)` (no null check); `mi = CModelInfo::ms_modelInfoPtrs [0xA9B0C8][veh->m_nModelIndex
  (+0x22, s16)]`; `h = (u8)mi->+0x4A` (low byte of m_nHandlingId, `movzx byte`); `fl = dword [h*0xE0 + 0xC2BAAC]`
  (gHandlingDataMgr 0xC2B9C8 .m_aVehicleHandling (+0x14)[h].m_nHandlingFlags (+0xD0)). 096E: `flag = (fl >> 25) & 1`
  (VEHICLE_HANDLING_LOW_RIDER, 0x47851C); 096F: `flag = (fl >> 26) & 1` (STREET_RACER, 0x478569). The model's handling
  data is used, not the vehicle's own handlingFlags copy. `UpdateCompareFlag(flag)`. Return 0.

### 0x47866F 0976 COMMAND_KILL_FX_SYSTEM_NOW  (gr: unregistered, used: 0/2/0)
- `CollectParameters(1)`; `i = CTheScripts::GetActualScriptThingIndex(P0, 1)`. If `i < 0` -> return 0 (0x47868B -> 0x479606).
  `fx = CTheScripts::ScriptEffectSystemArray[i].m_pFxSystem` (dword [i*8 + 0xA44114]); if null -> return 0.
  `g_fxMan (0xA9AE80).DestroyFxSystem(fx)` 0x4A9810; `CTheScripts::RemoveScriptEffectSystem(P0)` 0x492FD0 (cdecl).
  If `this->m_UsesMissionCleanup (+0xC6)`: `CTheScripts::MissionCleanUp (0xA90850).RemoveEntityFromList(P0, 4)` 0x4654B0.
  Return 0.

### 0x478611 0975 COMMAND_IS_EMERGENCY_SERVICES_VEHICLE  (gr: unregistered, used: 0/0/4)
- `CollectParameters(1)`; `veh = vehicle lookup(P0)` (no null check). `flag = veh->IsLawEnforcementVehicle()` 0x6D2370 `||
  m == 0x1A0 || m == 0x197 || m == 0x220` with `m = (s16)veh->+0x22` (416 ambulance, 407 and 544 fire trucks; 0x47863E..0x478650,
  short-circuit in that order). `UpdateCompareFlag(flag)`. Return 0.

### 0x4789DE 0984 COMMAND_GET_OBJECT_MODEL  (gr: unregistered, used: 1/0/4)
- `CollectParameters(1)`; `obj = ObjectPool lookup(P0)` (no null check); `ScriptParams[0] = (s32)(s16)obj->+0x22` (m_nModelIndex);
  `StoreParameters(1)`. Return 0.

### 0x4792AF 09A9 COMMAND_GET_HASH_KEY  (gr: unregistered, used: 0/0/4)
- `ReadTextLabelFromScript(buf = [esp+0x84], 0xF)` (0x4792BB; buffer length 15). `ScriptParams[0] = CKeyGen::GetUppercaseKey(buf)`
  0x53CF30 (cdecl). `StoreParameters(1)`. Return 0.

### 0x47917C 09A3 COMMAND_DRAW_CROSSHAIR  (gr: unregistered, used: 0/4/0)
- `CollectParameters(1)`; dword `CTheScripts::bDrawCrossHair [0xA44490] = (P0 != 0) ? 1 : 0` (full dword write, 0x479193). Return 0.

### 0x479203 09A6 COMMAND_SHOW_BLIPS_ON_ALL_LEVELS  (gr: unregistered, used: 0/4/0)
- `CollectParameters(1)`; byte `CTheScripts::RadarShowBlipOnAllLevels [0xA444A2] = (P0 != 0)`. Return 0.

### 0x47966F 09B8 COMMAND_ADD_BLOOD  (gr: unregistered, used: 0/4/0)
- `CollectParameters(8)`: [0..2] position, [3..5] direction, [6] amount (int), [7] ped handle.
- Locals `pos = (P0, P1, P2)` (orig [esp+0x58..0x60]), `dir = (P3, P4, P5)` (orig [esp+0x6C..0x74]) (0x479689..0x4796BA).
  `ped = CPedPool::GetAtRef(P7)` (0x4796BE; no null check). `g_fx (0xA9AE00).AddBlood(&pos, &dir, P6, ped->+0x12C)` 0x49EB00
  (last arg = raw f32 CPhysical::m_fContactSurfaceBrightness). Return 0.

### 0x478D2F 098D COMMAND_GET_CAR_MOVING_COMPONENT_OFFSET  (gr: unregistered, used: 0/4/0)
- `CollectParameters(1)`; `veh = vehicle lookup(P0)` (no null check, no vehicle-type check); `ScriptParams[0] =
  (f32) CAutomobile::GetMovingCollisionOffset(veh)` 0x6A2150 (ST0 return, `fstp` 0x478D51); `StoreParameters(1)`. Return 0.

### 0x479401 09AF COMMAND_SET_UP_SKIP_AFTER_MISSION  (gr: unregistered, used: 0/3/0)
- `CollectParameters(4)`: x, y, z, heading. `CGameLogic::SetUpSkip(CVector(P0, P1, P2) by value, P3, true, nullptr, false)`
  0x4423C0 (cdecl, 7 dwords: pushes 0, 0, 1, P3 then a 12-byte vector, `add esp,0x1C` 0x479451). Return 0.

### 0x4788FB 0980 COMMAND_EXTINGUISH_FIRE_AT_POINT  (gr: unregistered, used: 0/1/0)
- `CollectParameters(4)`: x, y, z, radius. `gFireManager (0xB71F80).ExtinguishPoint(CVector(P0, P1, P2) by value, P3)` 0x539450
  (thiscall, P3 pushed first, then the 12-byte vector, 0x47891F..0x47893E). Return 0.

### 0x478BC8 0987 COMMAND_GET_CAR_BLOCKING_CAR  (gr: unregistered, used: 0/1/2)
- `CollectParameters(1)`; `r = -1` (0x478BDF); `veh = vehicle lookup(P0)` (no null check); `e = veh->+0x420`
  (m_autoPilot (+0x390) .m_ObstructingEntity (+0x90)). If `e != 0 && (e->+0x36 & 7) == 2` (entity type vehicle, 0x478BF1..0x478BFA):
  `r = vehicle pool GetRef(e)` 0x424160. `ScriptParams[0] = r`; `StoreParameters(1)`. Return 0.

### 0x47972B 09BB COMMAND_IS_CAR_DOOR_DAMAGED  (gr: unregistered, used: 0/2/0)
- `CollectParameters(2)`; `veh = vehicle lookup(P0)` (no null check, no type check); `flag = (veh+0x5A0 (CAutomobile
  m_damageManager)).GetDoorStatus(P1)` 0x6C2230 `!= 0`. `UpdateCompareFlag(flag)`. Return 0.

### 0x478C1C 0988 COMMAND_GET_CURRENT_VEHICLE_PAINTJOB  (gr: unregistered, used: 0/0/2)
- `CollectParameters(1)`; `ScriptParams[0] = vehicle lookup(P0)->GetRemapIndex()` 0x6D0B70 (no null check); `StoreParameters(1)`.
  Return 0.

### 0x478DA5 0991 COMMAND_PAUSE_CURRENT_BEAT_TRACK  (gr: unregistered, used: 0/0/2)
- `CollectParameters(1)`; `AudioEngine.PauseBeatTrack((u8)P0)` 0x507200 (low byte of P0, zero-extended, 0x478DAE). Return 0.

### 0x479500 09B3 COMMAND_GET_CAR_DOOR_LOCK_STATUS  (gr: unregistered, used: 0/0/3)
- `CollectParameters(1)`; `ScriptParams[0] = vehicle lookup(P0)->m_nDoorLock (+0x4F8, dword)` (no null check); `StoreParameters(1)`.
  Return 0.

### 0x47829F 0969 COMMAND_IS_BIG_VEHICLE  (gr: unregistered, used: 0/0/3)
- `CollectParameters(1)`; `flag = (byte veh->+0x429 >> 2) & 1` (vehicleFlags bit 10, bIsBig; 0x4782BB..0x4782C4; no null check).
  `UpdateCompareFlag(flag)`. Return 0.

### 0x478952 0981 COMMAND_HAS_TRAIN_DERAILED  (gr: unregistered, used: 1/1/0)
- `CollectParameters(1)`; `flag = byte veh->+0x5B9 & 1` (CTrain m_nTrainFlags (+0x5B8) bit 8, bNotOnARailRoad; 0x47896D). No
  null check and no vehicle-type check (reads the byte on any vehicle). `UpdateCompareFlag(flag)`. Return 0.

### 0x47931F 09AB COMMAND_RANDOM_PASSENGER_SAY  (gr: unregistered, used: 1/0/0)
- `CollectParameters(2)`: [0] vehicle, [1] phrase id. `p = vehicle lookup(P0)->PickRandomPassenger()` 0x6D2A10 (no null check
  on the vehicle). If p == 0 -> return 0 (0x479342 -> 0x479606). `p->Say((u16)P1, 0, 1.0f (imm 0x3F800000), 0, 0, 0)` 0x5EFFE0
  (0x479348..0x479361). Return 0.

### 0x479107 09A2 COMMAND_REMOVE_OBJECT_ELEGANTLY  (gr: unregistered, used: 0/0/1)
- `CollectParameters(1)`; `obj = ObjectPool lookup(P0)`; `CTheScripts::CleanUpThisObject(obj)` 0x4866C0 (cdecl, called even when
  obj is null, 0x479124). If obj != 0: if `obj->m_pRwObject (+0x18) != 0`: `RpClumpForAllAtomics(rwobj, CClumpModelInfo::
  SetAtomicRendererCB 0x4C5280, CVisibilityPlugins::RenderFadingClumpCB 0x733630)` 0x749B70 (cdecl, 0x479142; the RwObject is
  treated as a clump without a type check); then `obj->+0x140 |= 0x400000` (CObject flags bit 22, bFadingIn; 0x47914A).
  Then if `this->m_UsesMissionCleanup (+0xC6)`: `MissionCleanUp.RemoveEntityFromList(P0, 3)` (0x479170). Return 0.

### 0x47905C 09A0 COMMAND_TASK_PICK_UP_SECOND_OBJECT  (gr: unregistered, used: 0/0/1)
- `this->ScriptTaskPickUpObject(command = 0x9A0 (EDI))` 0x46AF50 (gr-unreversed helper that reads all parameters itself:
  script-params.csv C7 T T C1). Return 0.

### 0x479831 09C0 COMMAND_GET_RANDOM_CAR_OF_TYPE_IN_ANGLED_AREA_NO_SAVE  (gr: unregistered, used: 0/1/0)
- `CollectParameters(6)`: [0] x1, [1] y1, [2] x2, [3] y2, [4] width (floats), [5] model (-1 or any negative = any).
  Locals: [esp+0x14] = P0, [esp+0x1C] = P1, [esp+0x38] = P2, [esp+0x2C] = P3, [esp+0x40] = P4.
- `r = -1` (EBP, 0x47986B); `i = pool->m_nSize` ([0xB74494]+8, 0x479854); if i == 0 skip. Loop (0x479880): `--i`; if `r != -1`
  break (0x479881, so the scan stops at the first match); `veh = slot accessor(i)` 0x41CC10; skip if null; skip unless
  `veh->GetVehicleAppearance()` 0x6D1080 is 1 (automobile) or 2 (bike) (called a second time when the first result is not 1,
  0x4798A8); `m = (s16)veh->+0x22`: continue only if `m == P5` or `P5 < 0` (0x4798BD..0x4798C3); skip unless
  `veh->CanBeDeleted()` 0x6D1180; `pos = entity position`; if `IsPositionWithinAngledArea2D(pos, P0, P1, P2, P3, P4)` 0x477DA0
  (cdecl, `add esp,0x18`, spec below) then `r = vehicle pool GetRef(veh)` (0x47990C). Loop while `i != 0` (0x479913).
  Slots are scanned from m_nSize-1 down to 0.
- `ScriptParams[0] = r`; `StoreParameters(1)`. No mission-cleanup registration ("NO_SAVE"). Return 0.

### 0x479957 09C3 COMMAND_IS_COP_VEHICLE_IN_AREA_3D_NO_SAVE  (gr: unregistered, used: 0/0/5)
- `CollectParameters(6)`: [0..2] corner A (x, y, z), [3..5] corner B. Locals: minX [esp+0x1C] = P0, maxX [esp+0x14] = P3,
  minY [esp+0x18] = P1, maxY [esp+0x10] = P4, minZ [esp+0x38] = P2, maxZ [esp+0x2C] = P5.
- Ordering (each `fld a; fcomp b; test ah,0x41; jne keep`, so the swap happens only when `a > b`; NaN keeps): X 0x47997B..0x4799B1,
  Y 0x4799B5..0x4799D0, Z 0x4799D4..0x4799EF. A swap writes the old min with `fld/fstp` f32 into max and the old max dword into min.
- `flag = 0` (byte [esp+0x40], 0x479A00); `n = pool->m_nSize`; for `i = 0; i < n; ++i` (signed, `jle` 0x479A05, `jl` 0x479AC6):
  `veh = slot accessor(i)`; skip if null, if `!veh->IsLawEnforcementVehicle()`, or if `word veh->+0x22 == 0x1AE` (430, Predator
  boat; 0x479A35). `p = entity position` (copied as dwords). Inside test, each with its own fcomp (0x479A59..0x479ABC):
  skip if `p.x < minX` (`test ah,1`; NaN skips), skip if `p.x > maxX` (`test ah,0x41; jp`; NaN skips), same for y with
  minY/maxY and z with minZ/maxZ. Passing all sets `flag = 1`; the loop does NOT break (all slots are visited).
- `UpdateCompareFlag(flag)` (0x479AD3), then falls into 0x479AD8: return 0.

### 0x478C89 098B COMMAND_STRING_CAT16 / 098C COMMAND_STRING_CAT8 (shared)  (gr: unregistered, used: 0/0/0 each)
- `a = GetPointerToScriptVariable(2)` 0x464790 (EDI), `b = GetPointerToScriptVariable(2)` (EBP) (0x478C89..0x478C98).
  `la = strlen(a)` (0x478CA1..0x478CAD), `total = la + strlen(b)` (0x478CAF..0x478CC2).
- Opcode test on [esp+0xE4] (0x478CB9/0x478CC4): 098B: if `total >= 16` (signed) return 0 at 0x478CD1; 098C: if `total >= 8`
  return 0 via 0x479AD8. In both overflow paths the third (destination) parameter is NOT consumed: m_IP is left on it and the
  next command decode starts there (original bug; unused by the shipped scripts, keep).
- Otherwise `c = GetPointerToScriptVariable(2)` (0x478CE5); byte loop `strcpy(c, a)` copying forward up to and including NUL
  (0x478CF0..0x478CF8); `n = strlen(b) + 1` (0x478D00..0x478D09); `d = c + strlen(c)` (scan from c, 0x478D0B..0x478D16);
  `memcpy(d, b, n)` as `rep movsd` (n/4) then `rep movsb` (n&3) (0x478D1F..0x478D26). Overlapping a/b/c behave as these exact
  forward byte/dword copies. Return 0.

### Trivial cases

| case | opcode | name (gr: unregistered) | used m/mi/s | behaviour |
|------|--------|-------------------------|-------------|-----------|
| 0x4782FD | 096B | STORE_CAR_MOD_STATE | 0/0/5 | `CShopping::StoreVehicleMods()` 0x49B280; return 0 |
| 0x478309 | 096C | RESTORE_CAR_MOD_STATE | 0/0/2 | `CShopping::RestoreVehicleMods()` 0x49B3C0; return 0 |
| 0x478581 | 0970 | FORCE_DEATH_RESTART | 0/3/0 | `CGameLogic::ForceDeathRestart()` 0x441240; return 0 |
| 0x478605 | 0974 | RESET_STUFF_UPON_RESURRECTION | 0/1/0 | `CGameLogic::ResetStuffUponResurrection()` 0x442980; return 0 |
| 0x478BB7 | 0986 | CLEAR_ALL_SCRIPT_FIRE_FLAGS | 0/3/0 | `gFireManager (0xB71F80).ClearAllScriptFireFlags()` 0x5397A0; return 0 |
| 0x478E41 | 0995 | UNMARK_ALL_ROAD_NODES_AS_DONT_WANDER | 0/4/0 | `ThePaths (0x96F050).UnMarkAllRoadNodesAsDontWander()` 0x44D400; return 0 |
| 0x47994B | 09C2 | FAIL_KILL_FRENZY | 0/0/0 | `CDarkel::FailKillFrenzy()` 0x43DC60; return 0 |
| 0x4788B8 | 097D | GET_NUM_CAR_COLOURS | 0/0/1 | C1; `ScriptParams[0] = (u8) ms_modelInfoPtrs[(s16)veh->+0x22]->+0x2D0` (m_nNumColorVariations, 0x4788DE; no null check); Store(1); return 0 |
| 0x4789BF | 0983 | SET_ONLY_CREATE_GANG_MEMBERS | 0/3/0 | C1; `CPopulation::m_bOnlyCreateRandomGangMembers [0xC0FCB3] = (P0 == 0)` (`sete`, 0x4789CF): INVERTED relative to the name (P0 = 1 clears the flag); original behaviour, keep |
| 0x478C6F | 098A | SET_GUNSHOT_SENSE_RANGE_FOR_RIOT2 | 0/3/0 | C1; dword `CEventGunShot::ms_fGunShotSenseRangeForRiot2 [0x8A625C] = P0` (raw float copy); return 0 |
| 0x478E52 | 0996 | SET_CHECKPOINT_HEADING | 0/1/0 | C2; `i = GetActualScriptThingIndex(P0, 3)`; `i < 0` -> return 0; `cp = ScriptCheckpointArray[i].m_Checkpoint` ([i*8 + 0xA44074]); null -> return 0; `CCheckpoints::SetHeading(cp->+4 (id), P1 raw float)` 0x722970 (cdecl); return 0 |
| 0x478F43 | 099B | CHANGE_PLAYBACK_TO_USE_AI | 0/0/0 | C1; `CVehicleRecording::ChangeCarPlaybackToUseAI(vehicle lookup(P0))` 0x45A360 (cdecl); return 0 |
| 0x478F9C | 099D | IS_NIGHT_VISION_ACTIVE | 0/1/0 | `UpdateCompareFlag(byte CPostEffects::m_bNightVision [0xC402B8])`; return 0 |
| 0x478FB2 | 099E | SET_CREATE_RANDOM_COPS | 0/3/0 | C1; `CPopulation::m_bDontCreateRandomCops [0xC0FCB4] = (P0 == 0)`; return 0 |
| 0x4796EC | 09B9 | DISPLAY_CAR_NAMES | 0/14/0 | C1; `CHud::bScriptDontDisplayVehicleName [0xBAA3F9] = (P0 == 0)`; return 0 |
| 0x479817 | 09BF | SET_FORCE_RANDOM_CAR_MODEL | 0/1/0 | C1; dword `CTheScripts::ForceRandomCarModel [0xA4448C] = P0`; return 0 |

(C1 = `CollectParameters(1)`; "vehicle lookup" = 0x4048E0, no null check.)

- gta-reversed: all opcodes above except 0979/0993 are unregistered (gr calls this original switch for them); 0979/0993 see
  0x479AD8. No other gr differences.

## Uncatalogued callees used by this switch (not in catalog/procedures.csv; specified here so the port is complete)

### 0x477DA0 IsPositionWithinAngledArea2D (proposed name; caller 0x4798F9)
- `__cdecl bool (const CVector* pos, float x1, float y1, float x2, float y2, float width)`, return AL, plain `ret` (caller pops
  0x18). Frame: `sub esp,0x24` + 4 pushes; args at [esp+0x38..0x4C] (pos [0x38], x1 [0x3C], y1 [0x40], x2 [0x44], y2 [0x48],
  width [0x4C]); arg slots are reused as locals (noted below). pos.z is not used.
- `a = CGeneral::GetRadianAngleBetweenPoints(x1, y1, x2, y2)` 0x53CBE0 (cdecl, ST0) `+ [0x858FE4] = 1.5707964` (0x477DC0).
  `while (a < [0x858B50] = 0.0) a += [0x858CBC] = 6.2831855` (`test ah,5; jp` exits on >= and NaN, 0x477DC9..0x477DF1);
  then `while (a > 6.2831855) a -= 6.2831855` (`test ah,0x41`, 0x477DF3..0x477E11). a stays in ST (not rounded to f32).
- `s = fsin(a) * width` (0x477E19/0x477E1B); `C.x = x2 + s` -> f32 [0x18] (0x477E23); `c = -(fcos(a) * width)` (0x477E2B..0x477E31),
  stored f32 into the width slot [0x4C] (0x477E33); `C.y = c + y2` (unrounded c) -> f32 [0x14]; `D.x = s + x1` -> f32 [0x10];
  `D.y = c_f32 + y1` -> f32 [0x4C] (0x477E47..0x477E4F).
- `dx = x2 - x1` -> f32 [0x2C]; `dy = y2 - y1` kept in ST; `e.x = D.x - x1` -> [0x1C]; `e.y = D.y - y1` -> [0x20]
  (0x477E53..0x477E7B). `sq1 = dy*dy + dx*dx` (0x477E7F..0x477E8B); `len1 = sqrt(sq1)` -> f32 into x2 slot [0x44];
  `len2 = sqrt(e.y*e.y + e.x*e.x)` -> f32 into y2 slot [0x48] (0x477E95..0x477EA9).
- `r.x = pos.x - x1` -> [0x24], `r.y = pos.y - y1` -> [0x28] (0x477EAD..0x477EBE).
- Inline normalise of (dx, dy): if `sq1 > 0.0` or NaN (`fcom 0.0; test ah,0x41; jp`, 0x477EC2): `inv = [0x858624] = 1.0 / sqrt(sq1)`
  (`fdivr`), stored f32 into y1 slot [0x40] (`fst`); `nx = inv(ST, unrounded) * dx`; `ny = dy * inv_f32` (0x477ED9..0x477EED).
  Else: `nx = 1.0`, `ny = dy` (0x477ECF..0x477ED1).
- `dot1 = ny*r.y + r.x*nx` (0x477EEF..0x477EFD; product order exactly so). If `dot1 < 0.0` or NaN -> false (0x477F01..0x477F0C);
  if `dot1 > len1` (or unordered) -> false (0x477F0E..0x477F17).
- `CVector2D::Normalise(&e)` 0x44E480 (thiscall, 0x477F1D). `dot2 = r.y*e.y + r.x*e.x` (0x477F22..0x477F32). If `dot2 < 0.0` or
  NaN -> false; if `dot2 > len2` (or unordered) -> false; else result = 1 (0x477F4C).
- Debug: if byte `CTheScripts::DbgFlag [0x859CF8]` != 0 (0 in the exe): `CTheScripts::DrawDebugAngledSquare(x1, y1, x2, y2,
  C.x, C.y, D.x, D.y)` 0x486990 (cdecl, 8 floats, 0x477F77). Return AL = result.
- Open question 0x477DA0: other callers were not enumerated here (the function is missing from the catalogue).

### 0x43EF90 CEntryExitManager::SetEntryExitFlagWithIndex (jmp stub -> .HOODLUM 0x1564BE0; gr EntryExitManager.h:68 declares it)
- `__cdecl void (int32 index, uint32 flagMask, bool enable)`, plain `ret`. `pool = CEntryExitManager::mp_poolEntryExits [0x96A7D8]`;
  if `(s8)pool->flags (+4)[index] < 0` -> `entry = null` else `entry = pool->objects (+0) + index*0x3C` (0x1564BE0..0x1564BFC).
  If `(u8)enable`: `entry->m_nFlags (+0x30, u16) |= (u16)flagMask`; else `&= ~(u16)flagMask` (0x1564BFE..0x1564C16). A free slot
  gives a write to address 0x30 (crash): original behaviour, keep.

### 0x6E76E0 CWaterLevel::SyncWater (gr WaterLevel.cpp:811)
- `__cdecl void()`: `[0xC228A4] = [0xB7CB84]` (CWaterLevel::m_nWaterTimeOffset = CTimer::m_snTimeInMilliseconds), `ret`.

## Open questions
- 0x4780FC..0x47810B (0964): gr's ReadTextLabelFromScript (RunningScript.cpp:766, a reading aid only) copies a static
  8-byte label without adding a NUL, so an 8-character label would make this copy read past the label buffer into the
  following stack bytes; confirm against 0x463D50's own spec. The port must reproduce the byte-copy-until-NUL loop.
- 0x478F20 (099A): meaning of bit 1 of CPhysical +0x40 (gr name bApplyGravity unverified); the mask is exact.
