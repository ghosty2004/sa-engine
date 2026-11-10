# CRunningScript::ProcessCommands2500To2599 (SCRIPT) - opcode specs

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
- Memory stores are written `[0xADDR] <- value`. SEH: every switch has a frame; a case that constructs a task sets the state slot to 0 around the ctor and back to -1.
- Vtable slots (index = offset/4): CEntity 0 dtor, 1 Add(rect), 2 Add, 3 Remove, 4 SetIsStatic, 5 SetModelIndex, 6
  SetModelIndexNoCreate, 7 CreateRwObject, 8 DeleteRwObject, 9 GetBoundRect, 10 ProcessControl, 11 ProcessCollision, 12
  ProcessShift, 13 TestCollision, 14 Teleport, 15 SpecialEntityPreCollisionStuff, 16 SpecialEntityCalcCollisionSteps, 17 PreRender,
  18 Render, 19 SetupLighting, 20 RemoveLighting, 21 FlagToDestroyWhenNextProcessed, 22 ProcessEntityCollision (CPhysical);
  CPed slot 8 (+0x20) DeleteRwObject, slot 5 (+0x14) SetModelIndex; CObject slot 13 (+0x34) TestCollision. CVehicle +0x428..+0x42F vehicle-flag bytes, +0x434..+0x437 colours 1-4 (u8), +0x590 m_nVehicleType (0 automobile, 9 bike). CPed +0x47C m_pIntelligence (task manager at +4), +0x480 m_pPlayerData.

- CRunningScript fields: +0x10 m_BaseIP, +0x14 m_IP, +0x3C m_LocalVars[34], +0xC5 m_CondResult, +0xC6 m_UsesMissionCleanup,
  +0xC7 m_IsExternal, +0xC9 m_ExternalType. `MissionCleanUp` = CTheScripts::MissionCleanUp 0xA90850 (CMissionCleanup;
  AddEntityToList 0x4637E0 / RemoveEntityFromList 0x4654B0, thiscall (handle, type: 1 car, 2 char, 3 object)).
- gr `unimplemented` = REGISTER_COMMAND_UNIMPLEMENTED -> notsa::script::detail::NotImplemented (CommandParser/Parser.hpp:68): reads no
  parameters and returns OR_INTERRUPT (-1). That equals the exe only for opcodes that reach the switch default; for an
  `unimplemented` opcode that HAS a case here, gr deviates (it skips the case and leaves the IP on the parameters).
- Bit-insert idiom used by several cases: `new = old ^ ((old ^ (v << k)) & (1 << k))`, i.e. bit k of the field becomes bit 0 of the
  script value (not `v != 0`).

## 0x47A760 CRunningScript::ProcessCommands2500To2599  (catalogue: specified/REA, port level 10)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x47A77E), command id at [esp+4] (`mov ecx,[esp+0x184]` at 0x47A780; ECX still
  holds the id when a case starts), `ret 4`. SEH frame `push -1; push 0x8382E7; push fs:[0]`, `sub esp,0x168`, push EBX/EBP/ESI/EDI.
  Before the jump it zeroes EBP and EBX (0x47A78D/0x47A78F); many cases use EBP as the constant 0 and 09DA uses it as its loop
  counter starting at 0. In cases: command id at `[esp+0x188]`, SEH state slot `[esp+0x180]`.
  Range: `eax = id - 0x9C4`, `cmp eax,0x63; ja 0x47BF2F`, `jmp [eax*4 + 0x47BF4C]` (100 dwords).
- Default 0x47BF2F: `or al,0xFF` (AL = -1). Tail 0x47BF31: `nop; nop; jmp 0x403A66` -> .text stub `mov ecx,[esp+0x178]; jmp 0x47BF38`
  -> `pop edi/esi/ebp; fs:[0] <- ecx; pop ebx; add esp,0x174; ret 4`. Cases end `xor al,al; jmp 0x47BF31`. Shared compare tail
  0x47BB9A (`if al: scratch [esp+0x10] = 1`) / 0x47BBA3 (`Cmp(scratch)`), 0x47B8B7 (`Cmp(al)`), 0x47BBAF / 0x47B8C7 (`xor al,al`).
- Table: 100 entries; 3 go to the default (09CD 09CE 09DF point at 0x47BF2F inside the table), 10 to the empty case 0x47BF2B
  (`xor al,al`, AL = 0: 09C6 09D3 09DC 09EA 09F3 09F9 0A00 0A04 0A05 0A0D; gr nop except 09DC), 87 real cases. gta-reversed handler
  41, nop 10 (not re-specified). Specified below: 45 unregistered + 1 unimplemented with a case (09DC).
- Default-reaching: 09CD IS_ITALIAN_GAME, 09CE IS_SPANISH_GAME, 09DF RESTORE_PLAYER_AFTER_2P_GAME (gr unimplemented -> -1, same).
- gta-reversed deviation: 09DC SET_CHAR_CAN_CLIMB_OUT_WATER is gr `unimplemented` (CharacterCommands.cpp:4480, -1) but the exe runs
  the empty case (AL = 0, parameters not read).

### 0x47A921 09CA COMMAND_SET_OBJECT_PROOFS  (gr: unregistered, used: 7 / 33 / 0)
- `Collect(6)`; `o = OBJ(SP0)` (unchecked); CPhysical flags `o+0x40`, bit-inserted in this order (each stored back): bit 18 (0x40000,
  bBulletProof) <- SP1, bit 19 (0x80000, bFireProof) <- SP2, bit 23 (0x800000, bExplosionProof) <- SP3, bit 20 (0x100000,
  bCollisionProof) <- SP4, bit 21 (0x200000, bMeleeProof) <- SP5 (0x47A942-0x47A9AC; bit 0 of each SP).

### 0x47A7A2 09C4 COMMAND_SET_PETROL_TANK_WEAKPOINT  (gr: unregistered, used: 0 / 49 / 0)
- `Collect(2)`; `v = VEH(SP0)` (unchecked); `v[+0x42E]` bit 6 (0x40) <- bit 0 of SP1 (0x47A7BC-0x47A7D4).

### 0x47AD5A 09DD COMMAND_MAKE_ROOM_IN_PLAYER_GANG_FOR_MISSION_PEDS  (gr: unregistered, used: 0 / 44 / 0)
- `Collect(1)`. `pd = CWorld::FindPlayerPed(-1)->m_pPlayerData`; `m = &ms_groups[pd->m_nPlayerGroup (+0x38)].m_groupMembership`
  (0xC09928 + g*0x2D4); FindPlayerPed(-1) is called a second time for `pd2` (0x47AD75).
  `n = m->CountMembersExcludingLeader() - (u8) pd2[+0x43] (m_nScriptLimitToGangSize) - SP0` (signed); if `n > 0`:
  `m->RemoveNFollowers(n)` 0x5FB1D0.

### 0x47AECF 09E2 COMMAND_CREATE_CAR_GENERATOR_WITH_PLATE  (gr: unregistered, used: 1 / 11 / 0)
- `Collect(12)`: x SP0, y SP1, z SP2, angle SP3, model SP4, colour1 SP5, colour2 SP6, forceSpawn SP7, alarm SP8, doorLock SP9,
  minDelay SP10, maxDelay SP11.
- `if !(z <= -100.0f)`... precisely: `fld z; fcomp [0x859014] (-100.0f); test ah,0x41; jne skip` -> only when `z > -100.0f` (not NaN):
  `z = z + [0x859F00] (0.015f)` (stored f32). (No ground lookup here, unlike the other create commands.)
- `gen = CTheCarGenerators::CreateCarGenerator(x, y, z, SP3, SP4, (u16)SP5, (u16)SP6, (u8)SP7, (u8)SP8, (u8)SP9, (u16)SP10,
  (u16)SP11, iplId 0, ignorePopulationLimit 1)` 0x6F31A0 (cdecl 14 dwords).
- `ReadLabel(plate, 9)` (buffer length 9, 0x47AF7C). For i = 0..7: if `plate[i] == '_'` or `plate[i] == 0` -> `' '`; `plate[8] <- 0`.
  `CSpecialPlateHandler::Add(&CTheCarGenerators::m_SpecialPlateHandler 0xC279D8, gen, plate)` 0x6F2D90.
- `SP0 <- gen`; `Store(1)`. (gen = -1 when no slot: still passed to Add and stored - original.)

### 0x47B109 09E7 COMMAND_IS_PLAYER_CONTROL_ON  (gr: unregistered, used: 1 / 5 / 2)
- `Collect(1)`; `p = CPad::GetPad(SP0)`; `Cmp(!((p[+0x10E] >> 5) & 1))` (bit 5 of byte +0x10E = player-controls-disabled flag).

### 0x47B84C 0A0C COMMAND_IS_PLAYER_USING_JETPACK  (gr: unregistered, used: 3 / 6 / 8)
- `Collect(1)`; `Cmp(CWorld::Players[SP0].m_pPed->m_pIntelligence->GetTaskJetPack() 0x601110 != null)`.

### 0x47AD10 09DB COMMAND_SET_MENU_COLUMN_WIDTH  (gr: unregistered, used: 0 / 6 / 30)
- `Collect(3)`: SP0 menu, SP1 column, SP2 width (u16 used). `w = ftol(((float)(i32)[0xC17044] RsGlobal.maximumWidth *
  [0x859520] (0.0015625f)) * (i32)(u16)SP2)` (fild, fmul, fimul, _ftol). `CMenuSystem::SetColumnWidth((u8)SP0, (u8)SP1, w)` 0x582050
  (cdecl, full EAX pushed).

### 0x47B8AD 0A0F COMMAND_HAS_LANGUAGE_CHANGED  (gr: unregistered, used: 0 / 2 / 7)
- `Cmp(CMenuManager::HasLanguageChanged(&FrontEndMenuManager 0xBA6748))` 0x573CD0.

### 0x47AE2C 09E0 COMMAND_SET_UP_SKIP_FOR_SPECIFIC_VEHICLE  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(5)`; `CGameLogic::SetUpSkip(CVector(SP0,SP1,SP2), SP3, afterMission 0, VEH(SP4), finishedByScript 0)` 0x4423C0 (cdecl
  7 dwords; compare 0A35 in the 2600 switch which passes finishedByScript 1).

### 0x47B02B 09E5 COMMAND_DRAW_LIGHT_WITH_RANGE  (gr: unregistered, used: 0 / 2 / 2)
- `Collect(7)`: x,y,z SP0..SP2, r,g,b SP3..SP5 (i32 0-255), range SP6 (f32).
- Colour floats computed in this order: `b = (float)SP5 * [0x859A3C] (0.003921569f = 1/255)`, then `g = (float)SP4 * same`, then
  `r = (float)SP3 * same` (fild i32, fmul f32, stored f32).
- `CPointLights::AddLight(type 0, CVector(x,y,z), direction CVector(0,0,0), range SP6, r, g, b, fogType 0, generateExtraShadows 1,
  entityAffected null)` 0x7000E0 (cdecl 14 dwords).

### 0x47B543 09FD COMMAND_GET_STRING_WIDTH / 0x47B6EB 0A08 COMMAND_GET_STRING_WIDTH_WITH_NUMBER  (gr: unregistered, used: 09FD 0/3/0, 0A08 0/1/0)
- 09FD: `ReadLabel(l, 8)`; `t = CText::Get(&TheText, l)`; `SP0 <- ftol(CFont::GetStringWidth(t, 1, 1))` 0x71A0E0 (cdecl, ST0);
  `Store(1)`.
- 0A08: `ReadLabel(l, 8)`; `t = CText::Get(l)`; `Collect(1)`; `CMessages::InsertNumberInString(t, SP0, -1, -1, -1, -1, -1, buf)`
  0x69DE90 (cdecl 8 args, wide local buffer); `SP0 <- ftol(CFont::GetStringWidth(buf, 1, 1))`; `Store(1)`.

### 0x47B1A4 09EB COMMAND_PLAYER_TAKE_OFF_GOGGLES  (gr: unregistered, used: 0 / 4 / 0)
- `Collect(2)`: SP0 player, SP1 animate flag. `ped = CWorld::Players[SP0].m_pPed`.
- If `SP1 == 0`: `ped->TakeOffGoggles()` 0x5E6010.
- Else: if `ped[+0x4FC]` (goggles object pointer) == 0 -> done; if `ped->m_pIntelligence[+0x10]` (task manager primary slot 3) != 0
  -> done. `t = new(0xC) CTaskComplexUseGoggles` 0x634EF0 (SEH state 1); `TaskManager(intel+4).SetTask(t, 3, 0)` 0x681AF0;
  `pd = ped->m_pPlayerData (+0x480)`; if non-null `pd[+0x85] <- 1`.

### 0x47B9AF 0A15 COMMAND_HAS_CAR_BEEN_RESPRAYED  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(1)`; `v = VEH(SP0)` (unchecked); scratch = 0; if `v[+0x42F] & 1`: clear that bit and scratch = 1 (read-and-reset);
  `Cmp(scratch)`.

### 0x47AA8D 09D0 COMMAND_IS_VEHICLE_ON_ALL_WHEELS  (gr: unregistered, used: 0 / 22 / 0)
- `Collect(1)`; `v = VEH(SP0)` (unchecked); scratch = 0; `t = v->m_nVehicleType (+0x590)`.
  If `t == 9` (bike) and `(u8) v[+0x804] == 4` -> 1; if `t == 0` (automobile) and `(u8) v[+0x960]` (m_nNumContactWheels) `== 4` -> 1.
  Any other type (quad, monster truck, plane...) is always false. `Cmp(scratch)`.

### 0x47AB20 09D2 COMMAND_ENABLE_AMBIENT_CRIME  (gr: unregistered, used: 0 / 20 / 0)
- `Collect(1)`; `[0xB72994] <- (SP0 != 0)` (byte; catalogue: g_LoadMonitor+0x1C neighbourhood, gr name unknown).

### 0x47AAEB 09D1 COMMAND_DOES_PICKUP_EXIST  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(1)`; `Cmp(CPickups::GetActualPickupIndex(SP0) 0x4552A0 != -1)`.

### 0x47AA49 09CF COMMAND_SET_TRAIN_FORCED_TO_SLOW_DOWN  (gr: unregistered, used: 0 / 3 / 0)
- `Collect(2)`; `v = VEH(SP0)` (unchecked, assumed CTrain); `v[+0x5B9]` bit 1 (0x02) <- `(SP1 != 0)`.

### 0x47AFD2 09E3 COMMAND_FIND_TRAIN_DIRECTION  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(1)`; `Cmp((VEH(SP0)[+0x5B8] >> 6) & 1)` (CTrain flag byte, bit 6).

### 0x47B00C 09E4 COMMAND_SET_AIRCRAFT_CARRIER_SAM_SITE  (gr: unregistered, used: 0 / 5 / 0)
- `Collect(1)`; `[0x8D0A24] CObject::bAircraftCarrierSamSiteDisabled <- (SP0 == 0)`.

### 0x47AC06 09D7 COMMAND_FORCE_INTERIOR_LIGHTING_FOR_PLAYER  (gr: unregistered, used: 0 / 4 / 0)
- `Collect(2)`; `CWorld::Players[SP0].m_pPed->m_pPlayerData[+0x86] <- (SP1 != 0)` (m_bForceInteriorLighting; no null checks).

### 0x47B0E7 09E6 COMMAND_ENABLE_BURGLARY_HOUSES  (gr: unregistered, used: 0 / 3 / 0)
- `Collect(1)`; `CEntryExitManager::EnableBurglaryHouses(SP0 != 0)` 0x43F180 (cdecl).

### 0x47B2BC 09EE COMMAND_FORCE_BIG_MESSAGE_AND_COUNTER  (gr: unregistered, used: 1 / 5 / 2)
- `Collect(1)`; `[0xBAA3FA] CHud::bScriptForceDisplayWithCounters <- (u8) SP0` (byte copy).

### 0x47B509 09FC COMMAND_IS_OBJECT_INTERSECTING_WORLD  (gr: unregistered, used: 0 / 3 / 0)
- `Collect(1)`; `o = OBJ(SP0)` (unchecked); `Cmp(o->TestCollision(false))` (virtual slot 13, +0x34; EBP = 0 pushed).

### 0x47B587 09FE COMMAND_RESET_VEHICLE_HYDRAULICS  (gr: unregistered, used: 0 / 6 / 1)
- `Collect(1)`; `v = VEH(SP0)` (unchecked); if `v[+0x38C] & 0x20000` (handling flags, hydraulics installed) and
  `v->m_nVehicleType == 0`: `CAutomobile::m_wMiscComponentAngle (+0x86C, u16) <- 0`.

### 0x47AB3F 09D4 COMMAND_CLEAR_WANTED_LEVEL_IN_GARAGE  (gr: unregistered, used: 0 / 0 / 3)
- No parameters: `CWanted::ClearWantedLevelAndGoOnParole(CWorld::FindPlayerWanted(-1) 0x56E230)` 0x5625A0.

### 0x47AC4E 09D9 COMMAND_USE_DETONATOR  (gr: unregistered, used: 0 / 1 / 0)
- No parameters. `CWorld::UseDetonator(CWorld::FindPlayerPed(-1))` 0x5660B0 (cdecl).

### 0x47AC65 09DA COMMAND_IS_MONEY_PICKUP_AT_COORDS  (gr: unregistered, used: 0 / 0 / 3)
- `Collect(3)`: pos = (SP0, SP1, SP2). `found = 0`. Loop `off = EBP (0 from the dispatcher); off < 0x4D80; off += 0x20` over
  `CPickups::aPickUps` 0x9788C0 (stride 0x20, 620 pickups), stopping as soon as `found` (checked at the top, 0x47AC96):
  if `pickup[+0x1C] == 8` (PICKUP_MONEY): `d = (pickup.GetPosn() 0x4549A0 - pos)` (VectorSub 0x40FE60), `if d.Magnitude() 0x4082C0
  < [0x858B8C] (0.5f)` (fcomp + `test ah,5; jp`: NaN/>= fail) `found = 1`. `Cmp(found)`.

### 0x47AE95 09E1 COMMAND_GET_CAR_MODEL_VALUE  (gr: unregistered, used: 4 / 0 / 0)
- `Collect(1)`; `mi = CModelInfo::ms_modelInfoPtrs[SP0]` (0xA9B0C8, no bounds or null check);
  `SP0 <- [0xC2BAB4 + (u8) mi[+0x4A] * 0xE0]` (handling entry of the model's handling id: tHandlingData m_nMonetaryValue); `Store(1)`.

### 0x47A820 09C7 COMMAND_SET_PLAYER_MODEL  (gr: unregistered, used: 0)
- `Collect(2)`; `ped = CWorld::Players[SP0].m_pPed`; `saved = ped[+0x4D4]`; `ped->DeleteRwObject()` (slot 8, +0x20);
  `ped->m_nModelIndex (+0x22) <- -1` (0xFFFF); `ped->SetModelIndex(SP1)` (slot 5, +0x14); `ped[+0x4D4] <- saved`.

### 0x47A869 09C8 COMMAND_ARE_SUBTITLES_SWITCHED_ON  (gr: unregistered, used: 0 / 1 / 0)
- `Cmp((u8) [0xBA678C])` (FrontEndMenuManager+0x44 m_bShowSubtitles).

### 0x47A9B6 09CB COMMAND_IS_CAR_TOUCHING_CAR  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(2)`; `Cmp(CPhysical::GetHasCollidedWith(VEH(SP0), VEH(SP1)))` 0x543540 (thiscall on the first, unchecked).

### 0x47B179 09E9 COMMAND_GIVE_NON_PLAYER_CAR_NITRO  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(1)`; `CAutomobile::NitrousControl(VEH(SP0), 1)` 0x6A3EA0 (no type or null check).

### 0x47B37A 09F2 COMMAND_DOES_DECISION_MAKER_EXIST  (gr: unregistered, used: 2 / 0 / 0)
- `Collect(1)`; scratch = 0; `i = GetActualScriptThingIndex(SP0, 7)`; if `0 <= i < 0x14`: `CDecisionMakerTypes::GetInstance()`
  0x4684F0 (result unused) and scratch = `CDecisionMakerTypes::m_IsActive[i]` (0xC0B01C) != 0 (via 0x47BB9A); `Cmp(scratch)`.

### 0x47B619 0A01 COMMAND_IS_THIS_MODEL_A_CAR  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(1)`; scratch = 0; `Cmp(CModelInfo::IsCarModel(SP0) 0x4C5AA0 != 0)` (via 0x47BB9A).

### 0x47B63A 0A02 COMMAND_SWITCH_ON_GROUND_SEARCHLIGHT  (gr: unregistered, used: 0 / 2 / 0)
- `Collect(2)`; `i = GetActualScriptThingIndex(SP0, 2)`; if `i < 0` -> done (no upper bound check). `byte [0xA94D6A + i*0x7C] <- (u8)SP1`
  (CTheScripts::ScriptSearchLightArray 0xA94D68 [i] + 2 = m_bEnableShadow per gr layout).

### 0x47B672 0A03 COMMAND_IS_GANG_WAR_FIGHTING_GOING_ON  (gr: unregistered, used: 4 / 0 / 1)
- `Cmp(CGangWars::GangWarFightingGoingOn() 0x443AC0 != 0)`.

### 0x47B698 0A06 COMMAND_IS_NEXT_STATION_ALLOWED  (gr: unregistered, used: 0 / 0 / 1)
- `Collect(1)`; `Cmp(CTrain::IsNextStationAllowed(VEH(SP0)))` 0x6F7260 (cdecl; via 0x47B8B7).

### 0x47B8FA 0A11 COMMAND_SET_EXTRA_CAR_COLOURS  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(3)`; `v = VEH(SP0)` (unchecked); `v[+0x436] <- (u8)SP1`; `v[+0x437] <- (u8)SP2`.

### 0x47B977 0A13 COMMAND_MANAGE_ALL_POPULATION  (gr: unregistered, used: 0 / 0 / 1) - `CPopulation::ManageAllPopulation()` 0x6160A0.

### 0x47BB08 0A1A COMMAND_TASK_PLAY_ANIM_SECONDARY  (gr: unregistered, used: 0 / 3 / 3)
- `this->PlayAnimScriptCommand(commandId)` 0x470150 (ECX = command id pushed at 0x47BB08). AL = 0.

### 0x47BBF7 0A1D COMMAND_TASK_HAND_GESTURE  (gr: unregistered, used: 0 / 0 / 1)
- `Collect(2)`; `target = PED(SP1)`; `t = new(0x18) CTaskComplexSignalAtPed(target, -1, 0)` 0x660A30 (SEH state 2);
  `GivePedScriptedTask(SP0, t, commandId [esp+0x188])`.

### 0x47BCE8 0A21 COMMAND_IMPROVE_CAR_BY_CHEATING  (gr: unregistered, used: 0 / 6 / 0)
- `Collect(2)`; `v = VEH(SP0)` (unchecked); `v[+0x42F]` bit 1 (0x02) <- bit 0 of SP1.

### 0x47BD27 0A22 COMMAND_CHANGE_CAR_COLOUR_FROM_MENU  (gr: unregistered, used: 0 / 0 / 4)
- `Collect(4)`: SP0 menu, SP1 vehicle, SP2 which colour, SP3 grid item. `v = VEH(SP1)` (unchecked);
  `c = CMenuSystem::GetCarColourFromGrid((u8)SP0, (u8)SP3)` 0x5822B0; if `SP2 == 1`: `v[+0x434] <- c` else `v[+0x435] <- c`.

### 0x47BD82 0A23 COMMAND_HIGHLIGHT_MENU_ITEM  (gr: unregistered, used: 0 / 0 / 3)
- `Collect(3)`; `CMenuSystem::HighlightOneItem((u8)SP0, (u8)SP1, (u8)SP2)` 0x581C10 (cdecl).

### 0x47BF2B 09DC COMMAND_SET_CHAR_CAN_CLIMB_OUT_WATER  (gr: unimplemented, used: 0)
- Empty shared case (with gr-nop 09C6 09D3 09EA 09F3 09F9 0A00 0A04 0A05 0A0D): AL = 0, no parameters read.

- gta-reversed: none of the unregistered opcodes has gr code. Open questions: 09D2 the name of byte 0xB72994; 09E7 the gr name of
  CPad byte +0x10E bit 5; 09EB the meaning of ped +0x4FC / player-data +0x85 (offsets exact).
