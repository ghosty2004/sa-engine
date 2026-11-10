# CRunningScript::ProcessCommands2300To2399 (SCRIPT) - opcode specs

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
  CEntity slot 4 (+0x10) SetIsStatic. CPhysical +0x40 flags, +0x50 m_vecTurnSpeed, +0x8C m_fMass, +0x90 m_fTurnMass. CObject +0x140 object flags dword, +0x148 m_nLastWeaponDamage (i8). CVehicle +0x428 flag byte 0 (0x10 engine on, 0x40 lights on), +0x42E flag byte 6. CAutomobile +0x5A0 m_damageManager, +0x5B8 m_doors[6] (CDoor, stride 0x18), +0x70C an extra CDoor (byte +0x717). `SetEngineOn` = 0x41BDD0 (see ProcessCommands700To799.md).
- CRunningScript fields: +0x10 m_BaseIP, +0x14 m_IP, +0x3C m_LocalVars[34], +0xC5 m_CondResult, +0xC6 m_UsesMissionCleanup,
  +0xC7 m_IsExternal, +0xC9 m_ExternalType. `MissionCleanUp` = CTheScripts::MissionCleanUp 0xA90850 (CMissionCleanup;
  AddEntityToList 0x4637E0 / RemoveEntityFromList 0x4654B0, thiscall (handle, type: 1 car, 2 char, 3 object)).
- gr `unimplemented` = REGISTER_COMMAND_UNIMPLEMENTED -> notsa::script::detail::NotImplemented (CommandParser/Parser.hpp:68): reads no
  parameters and returns OR_INTERRUPT (-1). That equals the exe only for opcodes that reach the switch default; for an
  `unimplemented` opcode that HAS a case here, gr deviates (it skips the case and leaves the IP on the parameters).
- "Protected store" stub 0x403A72 (used through 0x477BDF/0x477BE0): `pushfd; push ecx; clc; ecx = [0x1560794] (.HOODLUM, 0x0094B8DD)
  - [0x4F9E97] (.text, 0xFFF07C65) - CF(0) = 0xA43C78; [ecx] <- edx; pop ecx; popfd; jmp 0x477BE5` -> an obfuscated `SP0 <- EDX`
  (no-CD/anti-tamper protection code), followed at 0x477BE5 by `Store(1)`. Port as `SP0 = edx; Store(1)`.
- Model id rule ("UsedObjectArray"): `if (m < 0) m = *(i32*)(0xA44B88 - m*0x1C)` (CTheScripts::UsedObjectArray[-m].index).

## 0x4762D0 CRunningScript::ProcessCommands2300To2399  (catalogue: specified/REA, port level 18)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x4762E9), command id at [esp+4] kept in EDI (`mov edi,[esp+0x94]` at 0x4762D9),
  `ret 4`. NO SEH frame (`sub esp,0x84`, push EBX/ESI/EDI). Range: `eax = id - 0x8FC` (0x4762E0), `cmp eax,0x63; ja 0x477BFC`,
  `jmp [eax*4 + 0x477C0C]` (100 dwords). Default 0x477BFC: `pop edi; pop esi; or al,0xFF; pop ebx; add esp,0x84; ret 4` (AL = -1).
  Cases return through inlined `pop edi/esi/ebx; add esp,0x84; ret 4` with AL = 0 (0x477BEE is the shared `xor al,al` + epilogue).
- Table: 100 entries, none to the default; 18 to the empty case 0x477BEE (AL = 0, no parameters: 08FC 0902 0903 090A 090B 0911 091B
  0921 0927 092A 092C 092D 0932 0935 0938 093C 093F 0943; gr nop except 0903 0921 0932 0935 0943), 82 real cases. gta-reversed
  handler 30, nop 14 (not re-specified). Specified below: 51 unregistered + 5 unimplemented. No opcode of this range reaches the default.
- gta-reversed deviation: 0903 SET_CHAR_BEEN_PHOTOGRAPHED_FLAG, 0921 CAMERA_SET_SHAKE_SIMULATION, 0932 CAMERA_IS_FOV_RUNNING,
  0935 CAMERA_IS_SHAKE_RUNNING, 0943 SET_CAMERA_BEHIND_CHAR are gr `unimplemented` (-1) but run the empty case here (AL = 0, no
  parameters read, compare flag untouched).

### 0x476742 0913 COMMAND_START_NEW_STREAMED_SCRIPT  (gr: unregistered, used: 39 / 22 / 38)
- `Collect(1)`; `i = (i32)(i16) CStreamedScripts::GetProperIndexFromIndexUsedByScript(&CTheScripts::StreamedScripts 0xA47B60, (u16)SP0)`
  0x470810; `SP0 <- i`; `s = StreamedScripts.StartNewStreamedScript(i)` 0x470890; `this->ReadParametersForNewlyStartedScript(s)` 0x464500
  (reads the variable-length argument list). AL = 0.

### 0x47668A 090F COMMAND_MARK_STREAMED_SCRIPT_AS_NO_LONGER_NEEDED  (gr: unregistered, used: 33 / 5 / 1)
- `Collect(1)`; `i` as in 0913, `SP0 <- i`; `FUN_004700E0(i)` = `CStreaming::SetMissionDoesntRequireModel(i + 0x6676)` (tail jump to
  0x409C90; 0x6676 = 26230 = first streamed-script resource id; proposed name CStreaming::SetMissionDoesntRequireStreamedScript).

### 0x476D10 0926 COMMAND_GET_NUMBER_OF_INSTANCES_OF_STREAMED_SCRIPT  (gr: unregistered, used: 53 / 5 / 0)
- `Collect(1)`; `i` as in 0913 (SP0 not written back); `SP0 <- (u8) [0xA47B64 + i*0x20]` (StreamedScripts.m_aScripts[i] byte +4,
  instance count) via the protected store; `Store(1)`.

### 0x476D99 0929 COMMAND_ALLOCATE_STREAMED_SCRIPT_TO_OBJECT  (gr: unregistered, used: 35 / 0 / 0)
- `Collect(5)`; `i` as in 0913, `SP0 <- i`; `m = SP1` (UsedObjectArray rule); `CScriptsForBrains::AddNewScriptBrain(&ScriptsForBrains
  0xA90CF0, i, m, (u16)SP2 priority, attachType 1 (object), (u8)SP4 type, SP3 radius (f32))` 0x46A930.

### 0x476323 08FE COMMAND_IS_HELP_MESSAGE_BEING_DISPLAYED  (gr: unregistered, used: 20 / 70 / 15)
- `Cmp(CHud::HelpMessageDisplayed())` 0x588B50.

### 0x476659 090E COMMAND_SET_ACTIVE_MENU_ITEM  (gr: unregistered, used: 2 / 2 / 4)
- `Collect(2)`; `CMenuSystem::SetActiveMenuItem((u8)SP0, (u8)SP1)` 0x5820C0 (cdecl).

### 0x476D3F 0928 COMMAND_ALLOCATE_STREAMED_SCRIPT_TO_RANDOM_PED  (gr: unregistered, used: 5 / 0 / 0)
- `Collect(3)`; `i` as in 0913, `SP0 <- i`; `AddNewScriptBrain(&ScriptsForBrains, i, (u16)SP1 model, (u16)SP2 priority, attachType 0
  (ped), type -1, radius -1.0f)` (imm 0xBF800000).

### 0x47644C 0904 COMMAND_GET_HUD_COLOUR  (gr: unregistered, used: 1 / 80 / 1)
- `Collect(1)`; `c = CHudColours::GetRGB(&HudColour 0xBAB22C, (u8)SP0)` 0x58FEA0 (CRGBA via hidden result pointer);
  `SP0 <- c.r, SP1 <- c.g, SP2 <- c.b, SP3 <- c.a` (zero-extended bytes); `Store(4)`.

### 0x4777E6 0952 / 0x477811 0953 / 0x477823 0954 / 0x47783D 0955 COMMAND_PRELOAD_BEAT_TRACK / GET_BEAT_TRACK_STATUS / PLAY_BEAT_TRACK / STOP_BEAT_TRACK  (gr: unregistered, used: 0952 0/5/1, 0953 4/6/3, 0954 0/4/1, 0955 1/8/2)
- 0952: `Collect(1)`; `CAudioEngine::PreloadBeatTrack(&AudioEngine 0xB6BC90, (u16)SP0)` 0x507F40.
- 0953: `SP0 <- (i32)(i8) AudioEngine.GetBeatTrackStatus()` 0x507170 (protected store); `Store(1)`.
- 0954: `AudioEngine.PlayPreloadedBeatTrack(0)` 0x507180. 0955: `AudioEngine.StopBeatTrack()` 0x5071A0.

### 0x4778AA 0958 COMMAND_CREATE_SNAPSHOT_PICKUP, 0959 CREATE_HORSESHOE_PICKUP, 095A CREATE_OYSTER_PICKUP  (gr: unregistered, used: 0958 0/50/0, 0959 0, 095A 0)
- Shared case, selected by the command id in EDI: `Collect(3)`: x, y, z.
  - 0958: model `(u16) MI_PICKUP_CAMERA [0x8CD5D8]`, pickup type 0x14 (20 PICKUP_SNAPSHOT), stat 0xE8 (232).
  - 0959: model `(u16) MI_HORSESHOE [0x8CD754]`, type 3, stat 0xF2 (242).
  - 095A: model `(u16) MI_OYSTER [0x8CD750]`, type 3, stat 0xF4 (244).
- `CStats::IncrementStat(stat, 1.0f)` 0x55C180 (one per pickup CREATED; imm 0x3F800000). If `z <= -100.0f` ([0x859014]):
  `z = FindGroundZForCoord(x, y) + [0x858B8C] (0.5f)`. `h = CPickups::GenerateNewOne(CVector(x, y, z), model, type, 0, 0, 0, 0)` 0x456F20;
  `SP0 <- h`; `Store(1)` (0x47798B).

### 0x4774DA 0948 COMMAND_ADD_EXPLOSION_VARIABLE_SHAKE  (gr: unregistered, used: 0 / 33 / 0)
- `Collect(5)`: x, y, z, type SP3, shake SP4 (f32). `CExplosion::AddExplosion(null, null, SP3, CVector(x, y, z), lifetime 0, makeSound 1,
  camShake SP4, last argument 0)` 0x736A50 (cdecl, 10 dwords).

### 0x477323 0940 COMMAND_SET_GROUP_FOLLOW_STATUS  (gr: unregistered, used: 0 / 27 / 0)
- `Collect(2)`; `g = GetActualScriptThingIndex(SP0, 8)`; if `0 <= g < 8`: `byte [0xC09924 + g*0x2D4] <- (SP1 != 0)` (CPedGroups::
  ms_groups[g] + 4).

### 0x476810 0917 COMMAND_SWITCH_AUDIO_ZONE  (gr: unregistered, used: 0 / 16 / 0)
- `ReadLabel(name, 8)`; `Collect(1)`; `CAudioZones::SwitchAudioZone(name, SP0 != 0)` 0x508320 (cdecl).

### 0x476346 08FF COMMAND_HAS_OBJECT_BEEN_DAMAGED_BY_WEAPON  (gr: unregistered, used: 0 / 18 / 0)
- `Collect(2)`; `o = OBJ(SP0)`; scratch = 0; null -> false. If `SP1 == 0x38` or `0x39`: scratch = `CDarkel::CheckDamagedWeaponType((i8)
  o[+0x148], SP1)` 0x43D9E0; else scratch = `((i8) o[+0x148] == SP1)`. `Cmp(scratch)`.

### 0x4762F8 08FD COMMAND_SET_HEATHAZE_EFFECT  (gr: unregistered, used: 0 / 19 / 0)
- `Collect(1)`; `CPostEffects::ScriptHeatHazeFXSwitch(SP0 != 0)` 0x701160 (cdecl).

### 0x476CC6 0924 COMMAND_SET_DARKNESS_EFFECT  (gr: unregistered, used: 0 / 27 / 0)
- `Collect(2)`; `CPostEffects::ScriptDarknessFilterSwitch(SP0 != 0, SP1)` 0x701170 (cdecl).

### 0x47684E 0918 COMMAND_SET_CAR_ENGINE_ON  (gr: unregistered, used: 0 / 11 / 0)
- `Collect(2)`; `SetEngineOn(VEH(SP0), SP1 != 0)` 0x41BDD0 (unchecked).

### 0x47776C 0950 COMMAND_SET_UP_SKIP  (gr: unregistered, used: 0 / 12 / 0)
- `Collect(4)`; `CGameLogic::SetUpSkip(CVector(SP0, SP1, SP2), SP3, afterMission 0, vehicle null, finishedByScript 0)` 0x4423C0.

### 0x4777CE 0951 COMMAND_CLEAR_SKIP  (gr: unregistered, used: 0 / 20 / 0) - `CGameLogic::ClearSkip(0)` 0x441560 (cdecl).

### 0x476E6E 092E COMMAND_GET_WATER_HEIGHT_AT_COORDS  (gr: unregistered, used: 0 / 5 / 0)
- `Collect(3)`: x, y, waves flag SP2. If `SP2 != 0`: `ok = CWaterLevel::GetWaterLevel(x, y, 0.0f, &h, 1, null)` 0x6EB690; else
  `ok = CWaterLevel::GetWaterLevelNoWaves(x, y, 0.0f, &h, null, null)` 0x6E8580 (cdecl, 6 dwords each). If `!ok`: `h = -1000.0f`
  (imm 0xC47A0000). `SP0 <- h`; `Store(1)`.

### 0x477372 0941 COMMAND_SET_SEARCHLIGHT_CLIP_IF_COLLIDING  (gr: unregistered, used: 0 / 6 / 0)
- `Collect(2)`; `i = GetActualScriptThingIndex(SP0, 2)`; if `i < 0` -> done (no upper bound). `byte [0xA94D69 + i*0x7C] <- SP1 ? 1 : 0`
  (ScriptSearchLightArray[i].m_bClipIfColliding).

### 0x477584 094A COMMAND_UPDATE_PICKUP_MONEY_PER_DAY  (gr: unregistered, used: 1 / 0 / 0)
- `Collect(2)`; `CPickups::UpdateMoneyPerDay(SP0, (u16)SP1)` 0x455680 (cdecl).

### 0x477545 0949 COMMAND_ATTACH_MISSION_AUDIO_TO_CHAR  (gr: unregistered, used: 3 / 17 / 3)
- `Collect(2)`; `CAudioEngine::AttachMissionAudioToPed(&AudioEngine, (u8)SP0 - 1, PED(SP1))` 0x507310 (slot byte minus 1, 8-bit wrap).

### 0x4768EC 091C COMMAND_GET_USER_OF_CLOSEST_MAP_ATTRACTOR  (gr: unregistered, used: 0 / 0 / 11)
- `Collect(5)`: pos SP0..SP2, radius SP3, model SP4; `ReadLabel(name, 8)`. `m = SP4 == 0 ? -1 : (SP4 < 0 ? UsedObjectArray rule : SP4)`.
- `effect = 0; entity = 0`; `FUN_006002F0(&pos, SP3, m, 5, name, 0, 0, &entity, &effect)` (cdecl 9 args; a large search over 2D
  effects of type 5 near pos, proposed name "find nearest 2D effect"; outputs the owning entity and the effect).
- `SP0 <- -1`. If `entity != 0` and `ped = CPedAttractorManager::GetPedUsingEffect(GetPedAttractorManager() 0x5EE190, entity,
  effect)` 0x5EBE50 is non-null: `SP0 <- PEDREF(ped)` (0x4442D0). `Store(1)`.

### 0x4767B2 0916 COMMAND_WINCH_CAN_PICK_OBJECT_UP  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(2)`; `o = OBJ(SP0)` (unchecked); `o[+0x140] |= 0x40000` if `SP1 != 0` else `&= ~0x40000`.

### 0x47688A 0919 COMMAND_SET_CAR_LIGHTS_ON  (gr: unregistered, used: 0 / 3 / 0)
- `Collect(2)`; `v = VEH(SP0)` (unchecked); `v[+0x428]` bit 6 (0x40) <- `(SP1 != 0)` (bit-insert).

### 0x476701 0912 COMMAND_SET_MESSAGE_FORMATTING  (gr: unregistered, used: 0 / 6 / 0)
- `Collect(3)`; `[0xA44B64] CTheScripts::MessageCentre <- (u16)SP1`; `[0xA44B66] CTheScripts::bUseMessageFormatting <- (SP0 != 0)`;
  `[0xA44B60] CTheScripts::MessageWidth <- (u16)SP2`.

### 0x477AE5 095E COMMAND_CONTROL_CAR_DOOR  (gr: unregistered, used: 0 / 10 / 0)
- `Collect(4)`: SP0 vehicle, SP1 door, SP2 state, SP3 open ratio as an INTEGER. `v = VEH(SP0)` (unchecked, assumed CAutomobile);
  `a = (float)(i32)SP3` (fild, stored f32).
- If `SP1 < 6` (signed): if `!(a < 0.0f)` [0x858B50]: `CDoor::Open(&v->m_doors[SP1] (v + 0x5B8 + SP1*0x18), a)` 0x6F4790;
  `v->m_damageManager (+0x5A0).SetDoorStatus(SP1, SP2)` 0x6C21C0; `CAutomobile::SetDoorDamage(v, SP1, 0)` 0x6B1600.
- Else: if `!(a < 0.0f)`: `CDoor::Open(v + 0x70C, a)`; `byte v[+0x717] <- (u8)SP2`. (Negative door ids take the first branch.)

### 0x477239 093A COMMAND_SET_GARAGE_RESPRAY_FREE  (gr: unregistered, used: 2 / 0 / 0)
- `ReadLabel(name, 8)`; `Collect(1)`; `g = (i16) CGarages::GetGarageNumberByName(name)` 0x447680; if `g < 0` -> done.
  `byte [0x96C096 + g*0xD8]` bit 7 <- `(SP0 != 0)` (CGarages::aGarages 0x96C048 [g] + 0x4E, bits 0-6 kept).

### 0x476C9B 0923 COMMAND_SWITCH_AMBIENT_PLANES  (gr: unregistered, used: 0 / 10 / 0)
- `Collect(1)`; `CPlane::SwitchAmbientPlanes(SP0 != 0)` 0x6CCC50.

### 0x477175 0939 COMMAND_ATTACH_CAR_TO_OBJECT  (gr: unregistered, used: 0 / 4 / 0)
- `Collect(8)`: SP0 vehicle, SP1 object, SP2..SP4 offset, SP5..SP7 rotation in degrees. `v = VEH(SP0)`, `o = OBJ(SP1)` (unchecked).
  `rot = (SP5 * k, SP6 * k, SP7 * k)` with `k = [0x8595EC]` (0.017453292f), each stored f32.
  `CPhysical::AttachEntityToEntity(v, o, CVector(SP2, SP3, SP4), rot)` 0x54D570 (thiscall, entity + two CVectors by value).

### 0x4764AF 0905 COMMAND_LOCK_DOOR  (gr: unregistered, used: 0 / 1 / 2)
- `Collect(2)`; `o = OBJ(SP0)` (unchecked). If `SP1 != 0`: `CObject::LockDoor(o)` 0x59F5C0. Else: `o[+0x40] &= ~0x0C` (CPhysical flag bits 2
  and 3); `o->m_vecTurnSpeed (+0x50) <- (0, 0, 0)`; `o->SetIsStatic(true)` (slot 4).

### 0x4763BB 0900 COMMAND_CLEAR_OBJECT_LAST_WEAPON_DAMAGE  (gr: unregistered, used: 0 / 3 / 0)
- `Collect(1)`; `o = OBJ(SP0)`; if non-null `o[+0x148] <- 0xFF` (-1).

### 0x476BA1 091F COMMAND_GET_PLANE_UNDERCARRIAGE_POSITION  (gr: unregistered, used: 0 / 3 / 0)
- `Collect(1)`; `SP0 <- VEH(SP0)[+0x9CC]` (raw f32, CPlane landing-gear position; unchecked, no type check); `Store(1)`.

### 0x476E09 092B COMMAND_GET_GROUP_MEMBER  (gr: unregistered, used: 0 / 0 / 4)
- `Collect(2)`; `g = GetActualScriptThingIndex(SP0, 8)` (NOT range-checked: an invalid handle (-1) addresses before ms_groups -
  original bug); `p = CPedGroupMembership::GetMember(0xC09928 + g*0x2D4, SP1)` 0x5F69B0; `SP0 <- p ? PEDREF(p) : -1`; `Store(1)`.

### 0x47652F 0906 / 0x476563 0907 / 0x4765A0 0908 / 0x4765D5 0909 COMMAND_SET_OBJECT_MASS / GET_OBJECT_MASS / SET_OBJECT_TURN_MASS / GET_OBJECT_TURN_MASS  (gr: unregistered, used: 0906 0/1/0, 0907 0/1/0, 0908 0/1/0, 0909 0)
- Set: `Collect(2)`; `OBJ(SP0)[+0x8C (mass) / +0x90 (turn mass)] <- SP1` (raw dword). Get: `Collect(1)`; `SP0 <- OBJ(SP0)[+0x8C / +0x90]`;
  `Store(1)`. All unchecked.

### 0x4779A7 095B COMMAND_HAS_OBJECT_BEEN_UPROOTED  (gr: unregistered, used: 0 / 0 / 1)
- `Collect(1)`; `Cmp(!CEntity::GetIsStatic(OBJ(SP0)))` 0x4633E0 (neg/sbb/inc; unchecked).

### 0x477855 0956 COMMAND_FIND_MAX_NUMBER_OF_GROUP_MEMBERS  (gr: unregistered, used: 2 / 4 / 0)
- `SP0 <- CStats::FindMaxNumberOfGroupMembers()` 0x559A50; `Store(1)` (0x47798B).

### 0x47785F 0957 COMMAND_VEHICLE_DOES_PROVIDE_COVER  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(2)`; `v = VEH(SP0)` (unchecked); `v[+0x42E]` bit 2 (0x04) <- `(SP1 != 0)`.

### 0x4779EC 095C COMMAND_ADD_SMOKE_PARTICLE  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(12)`: pos SP0..SP2, velocity SP3..SP5, r,g,b,a SP6..SP9 (f32), size SP10, life SP11.
  Local `FxPrtMult_c m(SP6, SP7, SP8, SP9, SP10, 1.0f, SP11)` 0x4AB290.
  `FxSystem_c::AddParticle([0xA9AE20] (FxSystem_c*, member of g_fx 0xA9AE00), &pos, &vel, 0.0f, &m, -1.0f, 1.2f, 0.6f, 0)` 0x4AA440
  (imms 0xBF800000, 0x3F99999A, 0x3F19999A).

### 0x477B8C 095F COMMAND_GET_DOOR_ANGLE_RATIO  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(2)`; `v = VEH(SP0)` (unchecked). If `SP1 < 6` (signed): `SP0 <- v[+0x5C4 + SP1*0x18]` (m_doors[SP1] f32 at +0xC, fld/fstp),
  `Store(1)`. Else: `SP0 <- v[+0x718]` (the extra door at +0x70C, field +0xC) via the protected store, `Store(1)`.

### 0x4766C5 0910 COMMAND_REMOVE_STREAMED_SCRIPT  (gr: unregistered, used: 0)
- `Collect(1)`; `i` as in 0913, `SP0 <- i`; `FUN_0040C1C0(i)` = `CStreaming::RemoveModel(i + 0x6676)` (tail jump to 0x4089A0).

### 0x47679F 0915 COMMAND_SET_WEATHER_TO_APPROPRIATE_TYPE_NOW  (gr: unregistered, used: 0 / 3 / 0)
- `CWeather::SetWeatherToAppropriateTypeNow()` 0x72A790.

### 0x477BEE 0903, 0921, 0932, 0935, 0943  (gr: unimplemented, used: 0)
- Empty shared case: AL = 0, nothing read (deviation vs gr, see dispatcher).

- gta-reversed: none of the unregistered opcodes has gr code. Open questions: FUN_006002F0 (091C) is unnamed (the 9-argument contract
  above is exact from the pushes at 0x47696D-0x476993); the last AddExplosion argument (0x4774F4) is 0 - its gr meaning is unclear.
