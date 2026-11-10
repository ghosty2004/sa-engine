# CRunningScript::ProcessCommands1800To1899 (SCRIPT) - script switch spec

Shared switch conventions (caller 0x469EB0/0x469F00, AL meaning, CRunningScript offsets, gr `unimplemented`
semantics): see ProcessCommands0To99.md, top section. Pool helpers (`__thiscall`, `ret 4`, null for stale handles):
0x4048E0 CPool<CVehicle>::GetAt [0xB74494], 0x404910 CPedPool::GetAtRef [0xB74490], 0x465040 CPool<CObject>::GetAt
[0xB7449C], 0x424160 CPool<CVehicle>::GetRef(veh), 0x41CC10 (jmp to .HOODLUM 0x15638F0) CPool<CVehicle>::GetAt by slot
index (null when `flags[i] & 0x80`). "Entity position" = `m_matrix (+0x14) ? m_matrix+0x30 : this+4`. "-100 ground
rule" (fcom form): `z <= -100.0` ([0x859014] = -100.0) -> `z = CWorld::FindGroundZForCoord(x, y)` 0x569660.
"Mission guard": `m = veh->m_autoPilot.m_nCarMission (+0x3BA)`; the new mission is written only if `m` is neither 0x39
(PLANE_CRASH_AND_BURN) nor 0x3A (HELI_CRASH_AND_BURN). "Task give": `CTask::operator new(size)` 0x61A5A0, EH state
[esp+0xD0] = k during the constructor, task = 0 when new fails, EH state -1, then
`this->GivePedScriptedTask(pedHandle, task, command)` 0x465C20 (thiscall, pushes command, task, handle).

## 0x46D050 CRunningScript::ProcessCommands1800To1899  (catalogue: specified/REA, port level 21)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x46D07F; command at [esp+4] copied to EDI at 0x46D06F), `ret 4`.
  SEH frame `push -1; push 0x837FD0; push fs:[0]; mov fs:[0],esp` (0x46D050..0x46D05E), `sub esp,0xB8`, `push ebx; push ebp;
  push esi; push edi`. `eax = command - 0x708` (0x46D076); `cmp eax,0x63; ja 0x46ED96` (0x46D07C/0x46D081); direct jump
  table `jmp [eax*4 + 0x46EDB4]` (0x46D087, 100 entries).
- Common exit 0x46ED98: `nop; nop; jmp 0x4082A5` -> `mov ecx,[esp+0xC8]; jmp 0x46ED9F` -> `pop edi; pop esi; pop ebp;
  mov fs:[0],ecx; pop ebx; add esp,0xC4; ret 4`. Default 0x46ED96: `or al,0xFF` -> 0xFF. Shared return-0: 0x46ED92,
  0x46EBD4. Shared store-1 tail 0x46EBC6: `ScriptParams[0] = EAX; StoreParameters(1); return 0`. No case returns 1.
- Opcodes (100): 95 with a case; gta-reversed handler: 20, nop: 22 (not re-specified). Specified below: 53 (49
  unregistered + 4 unimplemented in the shared 0x46ED92 case).
  Default-reaching (5, gr unimplemented, same -1 behaviour): 0753, 0756, 0757, 0768, 0769. No gr `handler` opcode
  reaches the default.

### 0x46ED92 shared "return 0": 0711, 0712, 0718, 071B, 071C, 071D, 0720, 0721, 0722, 0725, 0728, 0738, 0739, 073A, 073D, 0740, 0744, 0748, 0758, 0759, 0764, 0765, 0766
- No parameter read, AL = 0. The 19 gr nops are `REGISTER_COMMAND_NOP(cmd)` without arguments (same). 071C GET_CHAR_LIGHTING,
  0738 SET_ZONE_RADAR_COLOURS, 0744 GET_STAT_CHANGE_AMOUNT, 0758 SET_CHAR_SPECIAL_EVENT (gr: unimplemented, used 0/0/0): gr
  returns -1. DEVIATION in the return value only.

### 0x46EB03 075C COMMAND_DOES_BLIP_EXIST  (gr: unregistered, used: 7/441/8)
- `CollectParameters(1)`; `flag = CRadar::GetActualBlipArrayIndex(P0)` 0x582870 `!= -1`; UpdateCompareFlag. Return 0.

### 0x46D3A7 0713 COMMAND_TASK_DRIVE_BY  (gr: unregistered, used: 0/169/3)
- `CollectParameters(10)`: [0] ped, [1] target ped (or < 0), [2] target car (or < 0), [3..5] target coords, [6] abort range
  (f32), [7] driveby style, [8] seat RHS flag, [9] frequency percentage.
- Target: `t = 0`; if `P1 >= 0` t = PedPool.GetAtRef(P1) (also kept as tPed at [esp+0x18], else tPed = 0); if `P2 >= 0`
  t = VehiclePool.GetAt(P2) (0x46D41E..0x46D456).
- If `P0 != -1` (0x46D458): `cur = PedPool.GetAtRef(P0)->m_pIntelligence (+0x47C)->+0x10` (task-manager slot); if cur != 0,
  `cur->vtable[4]()` (GetTaskType) == 0x3FE (TASK_SIMPLE_GANG_DRIVEBY) and `cur->+0x34 == tPed` -> return 0 without a new task
  (0x46D48B). (No null check on the GetAtRef result.)
- Task give (0x44 bytes, EH 0): `CTaskSimpleGangDriveBy(t, &CVector(P3,P4,P5), P6, P9, (u8)P7, P8 != 0)` 0x6217D0; then
  `task->+0xE = 1` (0x46D4EA; executed even when new returned 0 -> null write on OOM, original bug); GivePedScriptedTask(P0, ...).
  Return 0.

### 0x46D754 0723 COMMAND_BREAK_OBJECT  (gr: unregistered, used: 0/46/0)
- `CollectParameters(2)`: [0] object, [1] flag. `obj = ObjectPool.GetAt(P0)` (no null check).
- If `CGlass::IsObjectGlass(obj)` 0x46A760: `CGlass::WindowRespondsToCollision(obj, 99999.9f (imm 0x47C34FF3), CVector(0,0,0),
  CVector(entity position), 0)` 0x71BC40 (cdecl, 9 dwords, 0x46D7DA); `obj->m_nObjectFlags (+0x140) |= 0x400`. Return 0.
- Else: `info = obj->m_pObjectInfo (+0x160)`; local `v = info->+0x38..+0x40`; `g_breakMan (0xBB4240).Add(obj, &v, info->+0x44,
  P1 != 0)` 0x59E9B0 (0x46D83E). `obj->+0x1C &= 0xFFFFFF7E` (clear entity bits 0, 7); if `!CEntity::GetIsStatic(obj)` 0x4633E0:
  CPhysical::RemoveFromMovingList 0x542860. Then `+0x40 |= 0x800000`, `+0x140 |= 0x400`, `+0x1C |= 4`, `+0x44..+0x58 = 0`
  (move and turn speed), `obj->vtable[8]()` (+0x20, 0x46D89D). Return 0.

### 0x46D56F 0716 COMMAND_IS_CLOSEST_OBJECT_OF_TYPE_SMASHED_OR_DAMAGED  (gr: unregistered, used: 0/45/0)
- `CollectParameters(7)`: [0..2] pos, [3] radius, [4] model, [5] check smashed, [6] check damaged. -100 ground rule
  (0x46D57E..0x46D5A5). Model < 0 -> UsedObjectArray[-m].nModelIndex (0xA44B88 - m*0x1C).
- `o = CWorld::FindNearestObjectOfType(model, pos, P3, 0, 0, 0, 0, 1, 1)` 0x5693F0 (0x46D5FD). `flag = 0`. If o and
  `(o->+0x36 & 7) == 4` (OBJECT): if `o->+0x140 & 0x400` (`test dl,ch` with dl = 4, 0x46D61F) and P5 -> flag 1; if
  `0x46A2F0(o)` (thiscall, jmp to .HOODLUM 0x1569D20; damaged-state query) != 0 and P6 -> flag 1. UpdateCompareFlag. Return 0.

### 0x46DD76 0732 COMMAND_SUPPRESS_CAR_MODEL / 0x46DD95 0733 COMMAND_DONT_SUPPRESS_CAR_MODEL / 0x46DDB4 0734 COMMAND_DONT_SUPPRESS_ANY_CAR_MODELS  (gr: unregistered, used: 0/89/0, 0/10/0, 0/2/0)
- 0732: `CollectParameters(1)`; `0x46B1A0(P0)` (cdecl; adds the model to CTheScripts::SuppressedVehicleModels). 0733:
  `0x46A7E0(P0)` (jmp to .HOODLUM 0x15624A0; removes it). 0734: `CTheScripts::SuppressedVehicleModels` (0xA44940, 40 dwords)
  filled with -1 (`rep stosd`, ecx = 0x28, 0x46DDB4..0x46DDC1). Return 0.

### 0x46E9FC 075A COMMAND_PLAY_OBJECT_ANIM  (gr: unregistered, used: 0/21/65)
- `CollectParameters(1)` [0] object; `ReadTextLabelFromScript(anim, 24)`; `ReadTextLabelFromScript(block, 16)`;
  `CollectParameters(3)`: [0] blend delta (f32), [1] loop, [2] keep last frame (inverted).
- `a = CAnimManager::GetAnimation(anim, CAnimManager::GetAnimationBlock(block))` 0x4D42F0 / 0x4D3940. `f = (P1 ? 2 : 0) |
  (P2 == 0 ? 8 : 0)` (0x46EA6D..0x46EA81). If `obj->m_pRwClump (+0x18)`: init the clump with RpAnimBlendClumpInit 0x4D6720
  unless RpAnimBlendClumpIsInitialized 0x4D6760; `CAnimManager::BlendAnimation(clump, a, f, P0)` 0x4D4410; flag 1. Else flag 0.
  UpdateCompareFlag(flag). Return 0. (No null check on obj or a.)

### 0x46E450 0746 COMMAND_SET_RELATIONSHIP / 0x46E482 0747 COMMAND_CLEAR_RELATIONSHIP  (gr: unregistered, used: 22/126/0, 0/2/0)
- `CollectParameters(3)`; `CPedType::SetPedTypeAsAcquaintance(P0, P1, CPedType::GetPedFlag(P2))` 0x608E20 resp.
  `ClearPedTypeAsAcquaintance(...)` 0x6089F0 (GetPedFlag 0x608830; `add esp,0x10` cleans both). Return 0.

### 0x46E93B 0755 COMMAND_EXTEND_PATROL_ROUTE / 0x46E92A 0754 COMMAND_FLUSH_PATROL_ROUTE  (gr: unregistered, used: 0/125/0, 0/46/0)
- 0754: `[0xC18DB8] = 0` (patrol-route point count). Return 0.
- 0755: `CollectParameters(3)` pos (local at esp+0x30); `ReadTextLabelFromScript(a = esp+0x88, 24)`; `ReadTextLabelFromScript(b =
  esp+0x74, 16)`. If the first 5 bytes of `a` equal "NONE\0" ([0x859E38], `repe cmpsb`, 0x46E993): local anim record (esp+0xA0)
  gets `byte[0] = 0` and `byte[0x18] = 0`; else `0x46A340(&rec, a, b)` (thiscall ctor, jmp to .HOODLUM 0x156E1D0) builds it.
  `0x46AE80(this = 0xC18DB8, &pos, &rec)` (jmp to .HOODLUM 0x15622E0; appends a point). Return 0.

### 0x46DA35 0729 COMMAND_TASK_USE_MOBILE_PHONE  (gr: unregistered, used: 2/54/7)
- `CollectParameters(2)`: [0] ped, [1] start. If `P1 > 0` (signed): task give (0x20, EH 1) `CTaskComplexUseMobilePhone(-1)` 0x6348A0.
  Else: `ped = PedPool.GetAtRef(P0)` (no null check); `t = ped->m_pIntelligence->m_TaskMgr (+0x47C, +4).FindTaskByType(3, 0x640)`
  0x6817D0; if t and `t->vtable[4]() == 0x640`: `0x634A40(t, ped)` (thiscall; ends the phone task). Return 0.

### 0x46D71E 071F COMMAND_SET_OBJECT_HEALTH / 0x46D6F4 071E COMMAND_GET_OBJECT_HEALTH  (gr: unregistered, used: 0/65/0, 0/19/0)
- 071F: `CollectParameters(2)`; `obj->m_fHealth (+0x154) = (float)P1` (`fild`, f32). 071E: `CollectParameters(1)`;
  `ScriptParams[0] = _ftol(obj->m_fHealth)` (0x821B40, truncation) via 0x46EBC6. No null checks. Return 0.

### 0x46D166 070A COMMAND_TASK_PICK_UP_OBJECT  (gr: unregistered, used: 0/51/22)
- `this->ScriptTaskPickUpObject(command)` 0x46AF50 (reads its own parameters). Return 0.

### 0x46E802 0750 COMMAND_SET_OBJECT_VISIBLE  (gr: unregistered, used: 2/44/11)
- `CollectParameters(2)`; obj +0x1C `|= 0x80` if P1 != 0 else `&= 0xFFFFFF7F` (bIsVisible). Return 0.

### 0x46EAE7 075B COMMAND_SET_RADAR_ZOOM  (gr: unregistered, used: 0/44/0)
- `CollectParameters(1)`; `CTheScripts::RadarZoomValue [0xA444A3] = (u8)P0`. Return 0.

### 0x46DFC5 073C COMMAND_DAMAGE_CAR_DOOR / 0x46DCAD 0730 COMMAND_DAMAGE_CAR_PANEL  (gr: unregistered, used: 0/37/0, 0/24/0)
- `CollectParameters(2)`; `veh = GetAt(P0)`; `veh->m_damageManager (+0x5A0).ApplyDamage(veh, P1 + 5 (door) / P1 + 11 (panel),
  [0x871640] = 150.0 (raw dword copy), 1.0f)` 0x6C24B0 (0x46DFFD / 0x46DCE4). No null/type check. Return 0.

### 0x46E4B4 0749 COMMAND_CLEAR_GROUP_DECISION_MAKER_EVENT_RESPONSE / 0x46E4CB 074A COMMAND_ADD_GROUP_DECISION_MAKER_EVENT_RESPONSE  (gr: unregistered, used: 0/36/1, 0/14/0)
- 0749: `CollectParameters(2)`; jumps into the 0708 handler tail at 0x46D09F: `i = CTheScripts::GetActualScriptThingIndex(P0, 7)`
  0x4839A0; if `0 <= i < 20`: `CDecisionMakerTypes::GetInstance()` 0x4684F0 `->FlushDecisionMakerEventResponse(i, P1)` 0x604490.
- 074A: `CollectParameters(9)`: [0] dm, [1] event, [2] task, [3..6] f32 chances, [7], [8] ints. `i` as above (0x46E4DD), same
  range check. Local float[4] at esp+0x30 = {P6, P5, P3, P4}; local int[] at esp+0x24 = {P8, P7} (store order 0x46E4F4..0x46E52B).
  `GetInstance()->AddEventResponse(i, P1, P2, floats, ints)` 0x6044C0 (0x46E552). Return 0.

### 0x46D4FA 0714 COMMAND_SET_CAR_STAY_IN_SLOW_LANE / 0x46DF7A 073B COMMAND_SET_CAR_CAN_GO_AGAINST_TRAFFIC  (gr: unregistered, used: 0/6/0, 0/15/4)
- `CollectParameters(2)`; byte `veh+0x3DB` (m_autoPilot.m_nCarCtrlFlags): 0714 `|= 0x10` if P1 != 0 else `&= 0xEF`;
  073B `&= 0xBF` if P1 != 0 else `|= 0x40` (inverted sense). Return 0.

### 0x46D994 0727 COMMAND_POLICE_HELI_CHASE_ENTITY / 0x46D8F3 0726 COMMAND_HELI_FOLLOW_ENTITY / 0x46D304 0710 COMMAND_PLANE_FOLLOW_ENTITY  (gr: unregistered, used: 0/1/0, 0/6/0, 0/3/0)
- `CollectParameters(4)`: [0] vehicle, [1] ped (or < 0), [2] car (or < 0), [3] f32. `veh = GetAt(P0)`; `t = 0`; `P1 >= 0` -> t =
  PedPool.GetAtRef(P1); `P2 >= 0` -> t = VehiclePool.GetAt(P2) (overrides).
- Mission guard -> 0x28 (0727) / 0x27 (0726) / 0x25 (0710). Target ref `veh->m_autoPilot.m_pTargetCar (+0x41C)`: if old != 0
  `old->CleanUpOldReference(&field)` 0x571A00; `field = t`; `t->RegisterReference(&field)` 0x571B70 (thiscall on t, NOT
  null-checked: both handles < 0 call it with this = 0, original bug).
- 0710 only: `CVehicle::SetEngineOn(veh, 1)` 0x41BDD0 (jmp to .HOODLUM 0x1567620; sets vehicleFlags bit 4 unless +0x42D & 2).
- `veh->+0x9B0 = P3` (raw). 0726/0727: `veh->m_autoPilot.m_nCruiseSpeed (+0x3D0) = 100`. Return 0.

### 0x46D8A7 0724 COMMAND_HELI_ATTACK_PLAYER / 0x46D24B 070E COMMAND_PLANE_ATTACK_PLAYER  (gr: unregistered, used: 0/2/0, 0/1/0)
- `CollectParameters(3)`; veh = GetAt(P0); P1 unused. Mission guard -> 0x17 (0724) / 0x23 (070E); `veh->+0x9B0 = P2` (fld/fstp f32);
  0724 also `+0x3D0 = 100`. Return 0.

### 0x46D290 070F COMMAND_PLANE_FLY_IN_DIRECTION  (gr: unregistered, used: 0/2/0)
- `CollectParameters(4)`; `h = P1 * [0x8595EC] = 0.017453292`; mission guard -> 0x24; `+0x9B4 = h`, `+0x9A8 = P3` (raw),
  `+0x9B8 = h`, `+0x9B0 = P2` (x87 order 0x46D2E1..0x46D2F7). Return 0.

### 0x46E3A8 0743 COMMAND_HELI_LAND_AT_COORDS  (gr: unregistered, used: 0/1/0)
- `CollectParameters(6)`; `CAutomobile::TellHeliToGoToCoors(GetAt(P0), P1, P2, P3, P4, P5)` 0x6A2390; mission guard -> 0x2F. Return 0.

### 0x46E37A 0742 COMMAND_SET_PLANE_THROTTLE  (gr: unregistered, used: 0/11/0)
- `CollectParameters(2)`; `GetAt(P0)->+0x998 = P1` (raw). Return 0.

### 0x46E411 0745 COMMAND_PLANE_STARTS_IN_AIR  (gr: unregistered, used: 0/9/0)
- `CollectParameters(1)`; `CPlane::IsAlreadyFlying(veh)` 0x6CAB90 (thiscall, result ignored); if `(i16)veh->+0x22 == 0x208`
  (model 520) `(u16)veh->+0x86C = 0`. Return 0.

### 0x46DCF0 0731 COMMAND_SET_CAR_ROLL  (gr: unregistered, used: 0/1/0)
- `CollectParameters(2)`; `veh = GetAt(P0)`; `p = entity position` (copied); `veh->m_matrix->SetRotateZ(veh->GetHeading())`
  0x59B0E0 / 0x441DB0; `m_matrix->RotateY(P1 * 0.017453292)` 0x59B2C0; `0x46AE00(m_matrix, &p)` (CMatrix pos += p, adds x,y,z
  to +0x30..+0x38). Return 0.

### 0x46E009 073E COMMAND_GET_RANDOM_CAR_IN_SPHERE_NO_SAVE  (gr: unregistered, used: 1/1/0)
- `CollectParameters(5)`: [0..2] centre, [3] radius, [4] model (< 0 any). `best = 9999.9f` (imm 0x461C3F9A), `res = -1`.
  For i = size-1 down to 0 (all slots, no early exit): `veh = 0x41CC10(i)`; require appearance 1 or 2, `!(+0x428 & 1)`
  (not bIsLawEnforcer), model match, CanBeDeleted 0x6D1180; `d = |pos - centre|` (components f32, CVector::Magnitude 0x4082C0,
  stored f32); if `d < P3` and `d < best` (strict, NaN fails): `res = GetRef(veh)`, `best = d`. `ScriptParams[0] = res`;
  StoreParameters(1). Return 0.

### 0x46EC43 0763 COMMAND_SET_CAR_AS_MISSION_CAR  (gr: unregistered, used: 0/3/0)
- `CollectParameters(1)`; `veh = GetAt(P0)`; `h = GetRef(veh)`. If `this->m_UsesMissionCleanup` and `veh->m_nCreatedBy (+0x4A4)`
  is 1 or 3: `veh->SetVehicleCreatedBy(2)` 0x6D5D70 and `MissionCleanUp (0xA90850).AddEntityToList(h, 1)`. Return 0.

### 0x46DBE9 072C COMMAND_SWITCH_COPS_ON_BIKES  (gr: unregistered, used: 0/5/0)
- `CollectParameters(1)`; `CStreaming::DisableCopBikes(P0 == 0)` 0x407D10. Return 0.

### 0x46D1DB 070C COMMAND_EXPLODE_CAR_IN_CUTSCENE  (gr: unregistered, used: 0/9/0)
- `CollectParameters(1)`; `veh = GetAt(P0)`; byte +0x42A `|= 0x20` (vehicleFlags bit 21); `veh->vtable[0xA4/4](nullptr, 1)`
  (BlowUpCar(damager, inCutscene)). Return 0.

### 0x46E675 074C COMMAND_TASK_USE_ATTRACTOR  (gr: unregistered, used: 0/2/5)
- `CollectParameters(2)`; `i = GetActualScriptThingIndex(P1, 6)`; if `0 <= i < 64`: task give (0x24, EH 4)
  `CTaskComplexUseEffect(&CScripted2dEffects::ms_effects[i] (0xC3AB00 + i*0x40), 0)` 0x6321F0, ped P0. Return 0.

### 0x46E6FF 074D COMMAND_TASK_SHOOT_AT_CHAR  (gr: unregistered, used: 0/23/0)
- `CollectParameters(3)`; `t = PedPool.GetAtRef(P1)`; task give (0x3C, EH 5) `CTaskSimpleGunControl(t, nullptr, nullptr, 3, 1, P2)`
  0x61F3F0, ped P0. Return 0.

### 0x46EBDB 0762 COMMAND_TASK_DEAD  (gr: unregistered, used: 1/4/1)
- `CollectParameters(1)`; task give (0x28, EH 7) `CTaskComplexDie(0, 0, 0xF, 4.0f, 0.0f, 0, 0, 0, 0)` 0x630040, ped P0. Return 0.

### 0x46D545 0715 COMMAND_TAKE_REMOTE_CONTROL_OF_CAR  (gr: unregistered, used: 0/2/0)
- `CollectParameters(2)`; `0x45AD40(VehiclePool.GetAt(P1))` (cdecl, jmp to .HOODLUM 0x156E620; take remote control). P0 (player)
  unused. Return 0.

### Shop opcodes 075D, 075E, 075F, 0760, 0761  (gr: unregistered, used streamed 5, 5, 20, 11, 23)
| case | opcode | behaviour |
|---|---|---|
| 0x46EB37 | 075D LOAD_PRICES | `ReadTextLabelFromScript(buf, 16)`; `CShopping::LoadPrices(buf)` 0x49B8D0 |
| 0x46EB59 | 075E LOAD_SHOP | `ReadTextLabelFromScript(buf, 16)`; `CShopping::LoadShop(buf)` 0x49BBE0 |
| 0x46EB7B | 075F GET_NUMBER_OF_ITEMS_IN_SHOP | `ScriptParams[0] = CShopping::ms_numItemsInShop [0xA9A7F0]`; StoreParameters(1) |
| 0x46EB97 | 0760 GET_ITEM_IN_SHOP | `CollectParameters(1)`; `ScriptParams[0] = ms_shopContents[P0]` (0xA9A318, no bounds check); store 1 |
| 0x46EBAF | 0761 GET_PRICE_OF_ITEM | `CollectParameters(1)`; `ScriptParams[0] = CShopping::GetPrice(P0)` 0x49AD50; store 1 |
All return 0.

### 0x46DC1D 072D COMMAND_IS_FLAME_IN_ANGLED_AREA_2D / 072E ..._3D  (gr: unregistered, used: 0/0/1, 0/0/0)
- `this->FlameInAngledAreaCheckCommand(command)` 0x488780. Return 0.

Open questions: names of the unnamed callees 0x46A2F0, 0x46B1A0, 0x46A7E0, 0x634A40, 0x45AD40, 0x46A340, 0x46AE80 (behaviour is
"call it"); veh fields +0x998..+0x9B8 are CPlane/CHeli members not validated in gr.
