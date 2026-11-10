# CRunningScript::ProcessCommands1900To1999 (SCRIPT) - script switch spec

Notation: `SPn` = `ScriptParams[n]` (0xA43C78 + 4*n). `veh(h)` / `ped(h)` / `obj(h)` = pool GetAtRef 0x4048E0 / 0x404910 /
0x465040 (ECX = CPools::ms_pVehiclePool 0xB74494 / ms_pPedPool 0xB74490 / ms_pObjectPool 0xB7449C), null when the slot flag
byte != h & 0xFF; "no null check" = result dereferenced directly. GetRef: vehicle 0x424160, ped 0x4442D0, object 0x465070.
In this switch `this` = ESI and **EBP = command id** (0x46B47D). `GivePedScriptedTask(h, task, EBP)` = 0x465C20 (thiscall
ECX = this). Task allocation: `CTask::operator new(size)` 0x61A5A0; null -> task = null (ctor skipped); SEH state [esp+0x154]
set around the ctor, reset to -1 before GivePedScriptedTask. Return path: cases set AL = 0 and jump to 0x46CBD1.

## 0x46B460 CRunningScript::ProcessCommands1900To1999  (catalogue: specified/REA, port level 15)
- Dispatcher: `__thiscall int8 (CRunningScript* this = ECX, int32 command [esp+4])`, `ret 4`. SEH frame (handler 0x837F4D),
  `sub esp,0x13C`, push ebx/ebp/esi/edi. `ebp = command`, `esi = this`. A local CQuaternion at [esp+0x28..0x34] is zeroed at entry
  (0x46B491..0x46B4A9). `eax = command - 0x76C` (0x46B485), `cmp eax,0x63; ja 0x46CBCF` (0x46B48B/0x46B4B1): all 1900..1999
  dispatched; single table `jmp [eax*4+0x46CBEC]` (0x46B4B7).
- Default 0x46CBCF: `or al,0xFF`; 0x46CBD1: `nop; nop; jmp 0x406836` (.text stub `mov ecx,[esp+0x14C]; jmp 0x46CBD8`), pop
  edi/esi/ebp, `mov fs:[0],ecx`, pop ebx, `add esp,0x148; ret 4` -> -1.
- Opcodes: 99 with a case: 23 gta-reversed `handler`, 26 `nop` and 4 `unimplemented` (0787 SET_CHAR_TYRES_CAN_BE_BURST, 078E
  TASK_DRAG_CHAR_FROM_CAR, 079B ATTACH_CHAR_TO_ROPE_FOR_OBJECT, 07B2 SET_BEAT_ZONE_SIZE) all sharing 0x46CBCB = `xor al,al` (no
  operand read, return 0; matches gr's nop), 46 `unregistered` specified below.
- Default-reaching (1): 07B9 TASK_KILL_CHAR_ON_FOOT_PATROL (gr `unimplemented`).

### 0x46C6A1 07C0 COMMAND_REQUEST_CAR_RECORDING / 0x46C6C0 07C1 COMMAND_HAS_CAR_RECORDING_BEEN_LOADED  (gr: unregistered, used: 0/323/2, 0/319/2)
- 07C0: CollectParameters(1); `CVehicleRecording::RequestRecordingFile(SP0)` (0x45A020).
- 07C1: CollectParameters(1); flag = 0; UpdateCompareFlag(`CVehicleRecording::HasRecordingFileBeenLoaded(SP0)` != 0) (0x45A060, shared
  tail 0x46B57D). Return 0.

### 0x46B571 076F COMMAND_IS_MESSAGE_BEING_DISPLAYED  (gr: unregistered, used: 13/131/12)
- No params. UpdateCompareFlag(`CMessages::BriefMessages[0].m_pText` (dword [0xC1A7F0]) != 0). Return 0.

### 0x46C67A 07BF COMMAND_SET_BLIP_ALWAYS_DISPLAY_ON_ZOOMED_RADAR  (gr: unregistered, used: 0/81/0)
- CollectParameters(2). `CRadar::SetBlipAlwaysDisplayInZoom(SP0, (u8)SP1)` (0x583DB0, cdecl).

### 0x46B630 0772 COMMAND_TASK_GOTO_CAR  (gr: unregistered, used: 0/6/4)
- CollectParameters(4): SP0 = ped, SP1 = vehicle, SP2 = time (ms), SP3 = radius (f32).
- `v = veh(SP1)`; `t = SP2 < 0 ? 50000 (0xC350) : SP2` (0x46B660..0x46B666). `task = new(0x4C)
  CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>(v, t, 1000, SP3, [0x859E30]=2.0f, [0x859E34]=2.0f, true, true)`
  (0x46AC10). GivePedScriptedTask(SP0, task, EBP).

### 0x46BC65 078F COMMAND_TASK_CLIMB  (gr: unregistered, used: 0/33/0)
- CollectParameters(2). `task = new(0x14) CTaskComplexClimb()` (0x46A630); after the SEH state reset, `byte [task+0x10] =
  (SP1 != 0)` (0x46BCB7; written even if the allocation failed - original oddity). GivePedScriptedTask(SP0, task, EBP).

### 0x46C149 07A7 COMMAND_TASK_JETPACK  (gr: unregistered, used: 0/2/0)
- CollectParameters(1). `task = new(0x70) CTaskSimpleJetPack(nullptr, 10.0f /*imm 0x41200000*/, 0, nullptr)` (0x67B4E0);
  GivePedScriptedTask(SP0, task, EBP).

### 0x46C988 07C9 COMMAND_TASK_COMPLEX_PICKUP_OBJECT  (gr: unregistered, used: 0/3/0)
- CollectParameters(2). `o = obj(SP1)`; `task = new(0x34) <ctor 0x6919C0>(o, 0x51)` (unnamed constructor, sets vtable 0x870B98;
  proposed CTaskComplexGoPickUpEntity::CTaskComplexGoPickUpEntity(CEntity*, int32)); GivePedScriptedTask(SP0, task, EBP).
- Open question: identity of the task class built at 0x6919C0 (vtable 0x870B98) and meaning of its second argument 0x51.

### 0x46B6D4 0776 COMMAND_REQUEST_IPL / 0x46B6FC 0777 COMMAND_REMOVE_IPL / 0x46B724 0778 COMMAND_REMOVE_IPL_DISCREETLY  (gr: unregistered, used: 1/7/0, 0/6/0, 0/2/0)
- ReadTextLabelFromScript(buf, 0x12) (18-byte buffer); `s = CIplStore::FindIplSlot(buf)` (0x404AC0); then
  `CIplStore::RequestIplAndIgnore(s)` 0x405850 / `RemoveIplAndIgnore(s)` 0x405890 / `RemoveIplWhenFarAway(s)` 0x4058D0 (cdecl). Return 0.

### 0x46B80C 077D COMMAND_GET_CAR_PITCH  (gr: unregistered, used: 0/3/0)
- CollectParameters(1). `p = CAutomobile::GetCarPitch(veh(SP0)) * [0x859878]=57.295776f` (0x6A6050, ST0; stored f32, no checks);
  if `p < 0` (ordered): `p += [0x859E2C]=360.0f`; then if `p > 360.0f` (`test ah,0x41; jne` skips on <=, =, NaN): `p -= 360.0f`.
  SP0 = p, StoreParameters(1).

### 0x46B88C 077E COMMAND_GET_AREA_VISIBLE  (gr: unregistered, used: 9/52/9)
- `SP0 = CGame::currArea [0xB72914]`, StoreParameters(1).

### 0x46B8A6 0780 COMMAND_HELI_KEEP_ENTITY_IN_VIEW  (gr: unregistered, used: 0/2/0)
- CollectParameters(5): SP0 = heli, SP1 = ped or <0, SP2 = vehicle or <0, SP3 = min distance (f32), SP4 = f32.
- `h = veh(SP0)` (no null check). `e = null; if (SP1 >= 0) e = ped(SP1); if (SP2 >= 0) e = veh(SP2)` (vehicle wins).
  Mission byte [h+0x3BA]: keep 0x39/0x3A else `= 0x33` (51 HELI_KEEP_ENTITY_IN_VIEW). Target `[h+0x41C]`: if old non-null
  `CEntity::CleanUpOldReference(old, &slot)` (0x571A00); `slot = e`; `e->RegisterReference(&slot)` (0x571B70, ECX = e: **called with
  ECX = null when both SP1 and SP2 are negative** - original bug). `[h+0x9B0] = SP4` (raw dword); cruise `[h+0x3D0] = 100`;
  `[h+0x3D8] = (u8)__ftol2(SP3)` (0x821B40). Return 0.

### 0x46B960 0781 COMMAND_GET_WEAPONTYPE_MODEL / 0x46B992 0782 COMMAND_GET_WEAPONTYPE_SLOT  (gr: unregistered, used: 0/0/1 each)
- CollectParameters(1). `wi = CWeaponInfo::GetWeaponInfo(SP0, 1)` (0x743C60, cdecl, skill 1 = STD); `SP0 = wi->m_nModelId1 [+0xC]`
  (0781) / `wi->m_nSlot [+0x14]` (0782). StoreParameters(1).

### 0x46B9C5 0783 COMMAND_GET_SHOPPING_EXTRA_INFO  (gr: unregistered, used: 0/0/12)
- CollectParameters(2). `SP0 = CShopping::GetExtraInfo(SP0, SP1)` (0x49ADE0), StoreParameters(1).

### 0x46BA33 0786 COMMAND_GET_NUMBER_OF_FIRES_IN_AREA  (gr: unregistered, used: 0/2/0)
- CollectParameters(6). Per axis swap (SP0,SP3), (SP1,SP4), (SP2,SP5) only when the first is strictly greater
  (`fcomp; test ah,0x41; jne`). `SP0 = gFireManager (0xB71F80).GetNumFiresInArea(xlo, ylo, zlo, xhi, yhi, zhi)` (0x539860), StoreParameters(1).

### Winch / rope / train / shop (simple)
| case | opcode | name (used) | behaviour |
|---|---|---|---|
| 0x46BB10 | 0788 | ATTACH_WINCH_TO_HELI (0/1/0) | CollectParameters(2); `CVehicle::InitWinch(veh(SP0), SP1)` (0x6D3B60; SP1 left on the stack from the GetAtRef push sequence). |
| 0x46BB3F | 0789 | RELEASE_ENTITY_FROM_WINCH (0/1/0) | CollectParameters(1); `CVehicle::ReleasePickedUpEntityWithWinch(veh(SP0))` (0x6D3CB0). |
| 0x46BB68 | 078A | GET_TRAIN_CARRIAGE (0/12/0) | CollectParameters(2); `c = CTrain::FindCarriage(veh(SP0), (u8)SP1)` (0x6F5EB0, cdecl); `SP0 = c ? GetRef(c) : -1`; StoreParameters(1). |
| 0x46BBD3 | 078B | GRAB_ENTITY_ON_WINCH (0/2/0) | CollectParameters(1); SP0 = SP1 = SP2 = -1; `e = CVehicle::QueryPickedUpEntityWithWinch(veh(SP0))` (0x6D3CF0); shared tail 0x46BDCB: if e: by `[e+0x36] & 7`: 2 -> SP0 = vehicle GetRef, 3 -> SP1 = ped GetRef, 4 -> SP2 = object GetRef, other -> nothing; StoreParameters(3). |
| 0x46BD94 | 0798 | GRAB_ENTITY_ON_ROPE_FOR_OBJECT (4/11/0) | As 078B with `e = CObject::GetObjectCarriedWithRope(obj(SP0))` (0x59F3C0). |
| 0x46BE30 | 0799 | RELEASE_ENTITY_FROM_ROPE_FOR_OBJECT (1/5/1) | CollectParameters(1); `CObject::ReleaseObjectCarriedWithRope(obj(SP0))` (0x59F3E0). |
| 0x46BD2D | 0796 | GET_ROPE_HEIGHT_FOR_OBJECT (0/1/5) | CollectParameters(1); `SP0 = CObject::GetRopeHeight(obj(SP0))` (0x59F380, ST0 -> f32); StoreParameters(1). |
| 0x46BD65 | 0797 | SET_ROPE_HEIGHT_FOR_OBJECT (1/7/0) | CollectParameters(2); `CObject::SetRopeHeight(obj(SP0), SP1)` (0x59F3A0). |
| 0x46C615 | 07BD | DELETE_MISSION_TRAIN (0/3/0) | CollectParameters(1); if `veh(SP0)`: `CTrain::RemoveOneMissionTrain(v)` (0x6F5DC0). |
| 0x46C647 | 07BE | MARK_MISSION_TRAIN_AS_NO_LONGER_NEEDED (0/5/0) | CollectParameters(1); if `veh(SP0)`: `CTrain::ReleaseOneMissionTrain(v)` (0x6F5DF0). |
| 0x46C928 | 07C7 | SET_MISSION_TRAIN_COORDINATES (0/2/0) | CollectParameters(4); `CTrain::SetNewTrainPosition(veh(SP0), CVector(SP1,SP2,SP3))` (0x6F7140, cdecl 4 dwords). |
| 0x46BC30 | 078C | GET_NAME_OF_ITEM (0/0/33) | CollectParameters(1); `n = CShopping::GetNameTag(SP0)` (0x49ADA0); `p = GetPointerToScriptVariable(2)`; `_strncpy(p, n, 8)` (0x821F40). |
| 0x46BCC6 | 0790 | BUY_ITEM (0/0/8) | CollectParameters(1); `CShopping::Buy(SP0, 0)` (0x49BF70). |
| 0x46BD15 | 0793 | STORE_CLOTHES_STATE (1/4/17) | `CShopping::StoreClothesState()` (0x49B200). |
| 0x46BD21 | 0794 | RESTORE_CLOTHES_STATE (0/4/3) | `CShopping::RestoreClothesState()` (0x49B240). |
| 0x46C364 | 07B0 | GET_LOADED_SHOP (2/0/77) | `_strncpy(buf, CShopping::ms_shopLoaded (0xA9A7D8), 8)`; `MakeUpperCase(buf)` (0x718710); `p = GetPointerToScriptVariable(2)`; `_strncpy(p, buf, 8)`. |
| 0x46C1A7 | 07A8 | SET_AREA51_SAM_SITE (0/8/0) | CollectParameters(1); `CObject::bArea51SamSiteDisabled [0xBB4A72] = (SP0 == 0)`. |
| 0x46C552 | 07BB | ACTIVATE_HELI_SPEED_CHEAT (0/2/0) | CollectParameters(2); `[veh(SP0)+0x3D9] = (u8)SP1` (autopilot + 0x49). |
| 0x46C4B7 | 07B3 | SET_GROUP_DEFAULT_TASK_ALLOCATOR (0/15/0) | CollectParameters(2); `g = CTheScripts::GetActualScriptThingIndex(SP0, 8)`; if `0 <= g < 8`: `CPedGroupIntelligence::SetDefaultTaskAllocatorType(0xC09950 + g*0x2D4, SP1)` (0x5FBB70; = CPedGroups::ms_groups[g].m_groupIntelligence). |
| 0x46B5E8 | 0771 | CUSTOM_PLATE_DESIGN_FOR_NEXT_CAR (0/1/0) | CollectParameters(2); `mi = CModelInfo::ms_modelInfoPtrs[SP0]` (0xA9B0C8, index unchecked); if mi && `mi->vtable[0x10/4]()` (GetModelType) == 6 (vehicle) && `[mi+0x24] != 0`: `byte [mi+0x31] = (u8)SP1`. |
| 0x46C0AF | 07A6 | GET_NEAREST_TAG_POSITION (0/1/0) | CollectParameters(3); `t = CTagManager::GetNearestTag(CVector(SP0,SP1,SP2))` (0x49D160, by pointer); SP0..SP2 = t ? position of t : -4000.0f each (imm 0xC57A0000); StoreParameters(3). |

### 0x46C22E 07AB COMMAND_IS_TRAILER_ATTACHED_TO_CAB  (gr: unregistered, used: 0/13/0)
- CollectParameters(2). `t = SP0 == -1 ? null : veh(SP0)`; `c = SP1 == -1 ? null : veh(SP1)`; flag = 0.
  If t: flag = c ? (`t->m_pTractor [+0x4C4] == c`) : (`t->m_pTractor != 0`). Else: flag = (`c->m_pTrailer [+0x4C8] != 0`) (c not
  null-checked; both -1 crashes). UpdateCompareFlag(flag) (0x46C2A3). Return 0.

### 0x46C2AF 07AC COMMAND_DETACH_TRAILER_FROM_CAB  (gr: unregistered, used: 0/1/0)
- CollectParameters(2); t / c as in 07AB. If t: `t->vtable[0xF8/4 = 62]()` (CVehicle::BreakTowLink; both sub-branches identical,
  0x46C2F7 / 0x46C306). Else `x = c->m_pTrailer` (c not null-checked); if x: `x->BreakTowLink()`. Return 0.

### 0x46C3A2 07B1 COMMAND_GET_BEAT_PROXIMITY  (gr: unregistered, used: 5/0/3)
- CollectParameters(1): n. `info = AudioEngine (0xB6BC90).GetBeatInfo()` (0x5071B0). If `info[+0xA0] == 0` -> SP0 = SP1 = SP2 = -1.
- n < 0: `i = n + 10`; n > 0: `i = n + 9`: SP0 = `info[i*8]`, SP1 = `info[i*8+4]`, SP2 = `info[+0xA8]`.
- n == 0: `a = info[+0x48]`, `b = info[+0x50]`; if `b != 0 && info[+0xA8] != 0 && |b| > |a|` (signed abs via cdq/xor/sub):
  SP0 = a, SP1 = `info[+0x4C]`, SP2 = `info[+0xA8] - 1`; else SP0 = b, SP1 = `info[+0x54]`, SP2 = `info[+0xA8]`.
- StoreParameters(3). Return 0. (EBP is clobbered at 0x46C42C; not used afterwards.)

### 0x46C6E4 07C3 COMMAND_GET_OBJECT_QUATERNION / 0x46C7F7 07C5 COMMAND_GET_VEHICLE_QUATERNION  (gr: unregistered, used: 0/1/0, 0/0/0)
- CollectParameters(1). `m = CEntity::GetModellingMatrix(e)` (0x46A2D0; e = obj(SP0) / veh(SP0), unchecked); `q.Set(*m)`
  (CQuaternion::Set 0x59C3E0 on the zero-initialised local at [esp+0x28]); SP0..SP3 = q dwords +0, +4, +8, +0xC; StoreParameters(4).

### 0x46C747 07C4 COMMAND_SET_OBJECT_QUATERNION / 0x46C85A 07C6 COMMAND_SET_VEHICLE_QUATERNION  (gr: unregistered, used: 0/1/0 each)
- CollectParameters(5): SP0 = entity, SP1..SP4 = quaternion. `e = obj(SP0)` / `veh(SP0)` (unchecked). `q = (SP1, SP2, SP3, SP4)`;
  `q.Get(&rwm)` (0x59C080, RwMatrix local); `pos` = copy of e's position; `CMatrix tmp(&rwm, false)` (0x59C050; SEH state 7 / 8);
  `CPlaceable::SetMatrix(e, tmp)` (0x54F610); `~tmp` (0x59ACD0); `CPlaceable::SetPosn(e, pos)` (0x4241C0) restores the old position.
  Return 0.
