# CRunningScript::ProcessCommands700To799 (SCRIPT) - opcode specs

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
  CVehicle slot 50 (+0xC8) = Fix.

- CRunningScript fields: +0x10 m_BaseIP, +0x14 m_IP, +0x3C m_LocalVars[34], +0xC5 m_CondResult, +0xC6 m_UsesMissionCleanup,
  +0xC7 m_IsExternal, +0xC9 m_ExternalType. `MissionCleanUp` = CTheScripts::MissionCleanUp 0xA90850 (CMissionCleanup;
  AddEntityToList 0x4637E0 / RemoveEntityFromList 0x4654B0, thiscall (handle, type: 1 car, 2 char, 3 object)).
- gr `unimplemented` = REGISTER_COMMAND_UNIMPLEMENTED -> notsa::script::detail::NotImplemented (CommandParser/Parser.hpp:68): reads no
  parameters and returns OR_INTERRUPT (-1). That equals the exe only for opcodes that reach the switch default; for an
  `unimplemented` opcode that HAS a case here, gr deviates (it skips the case and leaves the IP on the parameters).
- CVehicle fields used here: +0x14 m_matrix (CMatrix*: forward at +0x10/+0x14/+0x18), +0x22 m_nModelIndex (i16), +0x36 type
  (bits 0-2) / status (bits 3-7; `(b & 7) | 0x18` = STATUS_PHYSICS 3), +0x40 CPhysical flags, +0x390 m_autoPilot (CAutoPilot:
  +0x3AC m_nTimeToStartMission, +0x3BA m_nCarMission (u8), +0x3D0 m_nCruiseSpeed (u8), +0x3EC/+0x3F0/+0x3F4
  m_vecDestinationCoors), +0x428 vehicle flags byte (bit 0x10 bEngineOn), +0x508 m_nLastWeaponDamageType (i8). Car missions 0x39 = 57
  MISSION_PLANE_CRASH_AND_BURN and 0x3A = 58 MISSION_HELI_CRASH_AND_BURN are never overwritten by these commands.
- `SetEngineOn(v, b)` = 0x41BDD0 (jmp stub to .HOODLUM 0x1567620, thiscall, ret 4): if `v[+0x42D] & 2` (engine broken)
  `v[+0x428] &= ~0x10` else `v[+0x428] bit 0x10 = b & 1`.
- Ground-Z rule used by several cases: `fld z; fcomp [0x859014] (-100.0f); test ah,0x41; jp skip` -> the lookup happens only when
  `z <= -100.0f` (NaN skips it).

## 0x47FA30 CRunningScript::ProcessCommands700To799  (catalogue: specified/REA, port level 35)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x47FA49), command id at [esp+4], `ret 4`. No SEH frame. `sub esp,0xD8`
  (0x47FA30), push EBX/EBP/ESI/EDI. Before the range check it zeroes the local `[esp+0x54]` (0x47FA4B): this is the
  water-level output of 02D3 (see there). Range: `eax = id - 0x2BF` (0x47FA40), `cmp eax,0x5F; ja 0x480F00` (0x47FA45/0x47FA53);
  `movzx eax, byte [eax + 0x480FD8]` (96-byte index table), `jmp [eax*4 + 0x480F10]` (50 dwords; index 0x31 = default 0x480F00,
  index 0x16 = 0x480EF1 empty case).
- Epilogues: each case returns through an inlined `pop edi; pop esi; pop ebp; pop ebx; add esp,0xD8; ret 4` with AL = 0
  (shared copies e.g. 0x480CED, 0x480E7B, 0x480EF1). Default 0x480F00: same pops with `or al,0xFF` (AL = -1, 0x480F03).
- Opcodes with a case: 52. gta-reversed handler: 23, nop: 1 (not re-specified). Specified below: 24 unregistered (02BF 02C1 02C2 02CA
  02CC 02D0 02D1 02D3 02D4 02DB 02E1 02E4 02E7 02E8 02E9 02EA 02ED 02EE 02F8 02F9 02FA 030D 0318 031E) + 4 unimplemented with a
  case (02E5 02E6 02F3 0319).
- Default-reaching (48): 33 gr `unimplemented` and 8 gr `unsupported` (030E 030F 0311-0316; REGISTER_UNSUPPORTED_COMMAND_HANDLER
  becomes NotImplemented unless IMPLEMENT_UNSUPPORTED_OPCODES is defined, CommandParser/Parser.hpp:135-142) behave like the exe
  (-1). gta-reversed DEVIATIONS: 7 opcodes have a real gr handler but reach the default (AL = -1, no parameters read) in the exe:
  02FC PRINT_WITH_2_NUMBERS, 0300 PRINT_WITH_3_NUMBERS_NOW, 0305 PRINT_WITH_5_NUMBERS, 0306 PRINT_WITH_5_NUMBERS_NOW,
  0309 PRINT_WITH_6_NUMBERS_NOW (TextCommands.cpp:331-333 ...), 031B IS_FIRST_CAR_COLOUR, 031C IS_SECOND_CAR_COLOUR
  (VehicleCommands.cpp:149 ...). Full default list: 02BC 02BD 02BE 02C3-02C9 02CD 02D2 02D5 02D7 02D9 02DA 02DC 02DE 02DF 02EF 02F0
  02F1 02F4 02F5 02FB 02FC 02FE 0300 0301 0304 0305 0306 0307 0309 030A 030B 030E-0316 031B 031C 031F.
- gta-reversed deviation (other direction): 02E5, 02E6, 02F3, 0319 are gr `unimplemented` (-1) but the exe runs the empty case (AL = 0).

### 0x480D75 0318 COMMAND_REGISTER_MISSION_PASSED  (gr: unregistered, used: 0 / 127 / 0)
- `ReadLabel(label, 8)`. `CText::Get(&TheText 0xC1B340, label)` 0x6A0050 is called and its result discarded (0x480D90).
  `strncpy(CStats::LastMissionPassedName 0xB78A00, label, 8)` 0x821F40 (cdecl).
  `CStats::IncrementStat(0x93 STAT_MISSIONS_PASSED, 1.0f)` 0x55C180 (imm 0x3F800000).
  `v = CStats::GetStatValue(0xB0 STAT_KILLS_SINCE_LAST_CHECKPOINT)` 0x558E40 (ST0, stored f32 into a reused argument slot at
  0x480DC2); `CStats::IncrementStat(0xB1 STAT_TOTAL_LEGITIMATE_KILLS, v)` (0x480DCE); `CStats::SetStatValue(0xB0, 0.0f)` 0x55A070.
  `[0xA476A8] CTheScripts::LastMissionPassedTime <- [0xB7CB84] CTimer::m_snTimeInMilliseconds`.

### 0x4806FB 02E4 COMMAND_LOAD_CUTSCENE  (gr: unregistered, used: 0 / 135 / 22)
- `ReadLabel(name, 8)`; `CCutsceneMgr::LoadCutsceneData(name)` 0x4D5E80 (cdecl).

### 0x48078A 02EA COMMAND_CLEAR_CUTSCENE  (used: 0 / 136 / 1) - `CCutsceneMgr::DeleteCutsceneData()` 0x4D5ED0.
### 0x48072B 02E7 COMMAND_START_CUTSCENE  (used: 0 / 134 / 1) - `CCutsceneMgr::StartCutscene()` 0x5B1460.
### 0x480761 02E9 COMMAND_HAS_CUTSCENE_FINISHED  (used: 0 / 134 / 1) - `Cmp(CCutsceneMgr::HasCutsceneFinished() != 0)` 0x5B0570.
### 0x48073F 02E8 COMMAND_GET_CUTSCENE_TIME  (used: 0 / 1 / 0) - `SP0 <- CCutsceneMgr::GetCutsceneTimeInMilleseconds()` 0x5B0550; `Store(1)`.
(All four: gr unregistered, no parameters collected, AL = 0.)

### 0x47FFC3 02D1 COMMAND_REMOVE_SCRIPT_FIRE  (gr: unregistered, used: 0 / 76 / 0)
- `Collect(1)`; `i = CTheScripts::GetActualScriptThingIndex(SP0, 5)` 0x4839A0 (cdecl, type 5 = fire); if `0 <= i < 0x3C` (signed,
  0x47FFDD/0x47FFE5) `CFireManager::RemoveScriptFire(&gFireManager 0xB71F80, i)` 0x539700. AL = 0 either way.

### 0x47FDA1 02CA COMMAND_IS_CAR_ON_SCREEN / 0x47FE34 02CC COMMAND_IS_OBJECT_ON_SCREEN  (gr: unregistered, used: 02CA 1/61/3, 02CC 0/7/1)
- `Collect(1)`; `e = VEH(SP0)` (02CA) / `e = OBJ(SP0)` (02CC; pool [0xB7449C]); no null check (crash on a bad handle).
  `r = CModelInfo::ms_modelInfoPtrs[e->m_nModelIndex (i16 +0x22)] (0xA9B0C8)->m_pColModel (+0x14)->boundSphere radius (+0x24)`
  (raw dword copy). 02CA jumps into 02CC's tail at 0x47FE6A. `scratch = 0`;
  `c = CEntity::GetBoundCentre(e, &tmp)` 0x534250 (thiscall, returns the CVector* tmp); `vis = CCamera::IsSphereVisible(&TheCamera
  0xB6F028, c, r)` 0x420D40; jump to 0x47FAC2 (02BF's tail): if `vis` scratch = 1; `Cmp(scratch)`.

### 0x47FA67 02BF COMMAND_IS_CAR_IN_WATER  (gr: unregistered, used: 3 / 27 / 1)
- `Collect(1)`; `v = VEH(SP0)`; `scratch [esp+0x10] = 0`; if `v == null` -> `Cmp(0)`.
- If `v->+0x40 & 0x100` (CPhysical bIsInWater, `test ah,1` at 0x47FA90) -> true.
  Else if `v->m_nModelIndex == 0x21B` (539, Vortex hovercraft; 0x47FA95) and `v->m_fWheelsSuspensionCompression[0] (+0x7D4) < 1.0f`
  ([0x858624] = 1.0; `test ah,5; jp` -> NaN and >= fail, 0x47FAA3-0x47FAAE) and `SurfaceInfos_c::IsShallowWater(&g_surfaceInfos
  0xB79538, (u8) v[+0x747])` 0x55E730 (CAutomobile m_wheelColPoint[0].m_nSurfaceTypeB, 0x724 + 0x23) -> true. Otherwise false.
- `Cmp(scratch)` (0x47FAD2).

### 0x47FF83 02D0 COMMAND_IS_SCRIPT_FIRE_EXTINGUISHED  (gr: unregistered, used: 0 / 15 / 0)
- `Collect(1)`; `i = GetActualScriptThingIndex(SP0, 5)`; scratch = 0; if `0 <= i < 0x3C`: `b = CFireManager::IsScriptFireExtinguished(
  &gFireManager, i)` 0x5396E0 and scratch = (b != 0) via the shared tail 0x47FAC2; `Cmp(scratch)`. Invalid index -> false.

### 0x480A57 02FA COMMAND_CHANGE_GARAGE_TYPE  (gr: unregistered, used: 2 / 18 / 0)
- `ReadLabel(name, 8)`; `g = CGarages::GetGarageNumberByName(name)` 0x447680 (cdecl, i16 in DI); `Collect(1)` (always, 0x480A7E);
  if `(i16) g < 0` -> AL = 0 (0x480A86). Else `CGarages::ChangeGarageType(g (full EDI pushed), (u8)SP0, 0)` 0x4476D0 (cdecl, 3 args).

### 0x480E8A 031E COMMAND_HAS_CAR_BEEN_DAMAGED_BY_WEAPON  (gr: unregistered, used: 0 / 4 / 0)
- `Collect(2)`: SP0 vehicle, SP1 weapon type. `v = VEH(SP0)`; scratch = 0; null -> false.
- If `SP1 == 0x38` (56) or `SP1 == 0x39` (57) (0x480EB4/0x480EB9, the "any melee / any weapon" pseudo types): scratch =
  `CDarkel::CheckDamagedWeaponType((i8) v->m_nLastWeaponDamageType (+0x508), SP1)` 0x43D9E0 (cdecl). Else scratch =
  `((i8) v[+0x508] == SP1)` (sign-extended compare, 0x480EBE). `Cmp(scratch)`.

### 0x4807FF 02EE COMMAND_IS_PROJECTILE_IN_AREA  (gr: unregistered, used: 0 / 5 / 0)
- `Collect(6)`: x1 SP0, y1 SP1, z1 SP2, x2 SP3, y2 SP4, z2 SP5 (f32).
- Sort each axis (0x480823-0x480897): `if !(x1 < x2 || x1 == x2)` (fcomp + `test ah,0x41; jne skip`: skip when x1 <= x2 or
  NaN) swap x1/x2; same for y, then z. So after it: min/max per axis except NaN inputs, which stay unswapped.
- `scratch = CProjectileInfo::IsProjectileInRange(x1, x2, y1, y2, z1, z2, false)` 0x739860 (cdecl 7 args); `Cmp(scratch)`.
- Debug: if `[0x859CF8] CTheScripts::DbgFlag` (.rdata, value 0) != 0: `FUN_00486AE0(x1, y1, x2, y2, z1, z2)` (debug area draw,
  0x4808FC) - dead in the shipped exe (the flag is a constant 0). AL = 0.

### 0x47FBDE 02C1 COMMAND_GET_CLOSEST_CAR_NODE  (gr: unregistered, used: 0 / 6 / 0)
- `Collect(3)`: x SP0, y SP1, z SP2. `z` stays in ST0; if `z <= -100.0f` ([0x859014], NaN skips): `fstp st0`, `z =
  CWorld::FindGroundZForCoord(x, y)` 0x569660 (cdecl ST0). z stored f32 (0x47FC20).
- `node = CPathFind::FindNodeClosestToCoors(&ThePaths 0x96F050, &retNode /*hidden return*/, CVector(x,y,z), nodeType 0
  (PATH_TYPE_CARS), maxDistance 999999.875f (imm 0x497423FE), unk2 0, unk3 1, unk4 0, bBoatsOnly 0, unk6 0)` 0x44F460 (pushes at
  0x47FC28-0x47FC37 are, in push order, `0, 0, 0, 1, 0, 0x497423FE, 0`; gr default arguments differ (unk2 = 1, unk3 = 0) - the
  script passes these explicitly; returns a CNodeAddress*, `mov eax,[eax]`). `CPathFind::FindNodeCoorsForScript(&ThePaths, &outVec, node, &found)` 0x4505E0 (the linked-node overload).
- If `found` (byte at 0x47FC7B): jump 0x47FB98 (inside 02C0's block): `SP0..SP2 <- outVec.x,y,z`, scratch = 1.
  Else: `SP1 <- 0, SP2 <- 0` (SP0 keeps the input x; 0x47FC87-0x47FC98), scratch = 0. Both: `Store(3)` then `Cmp(scratch)`
  (0x47FBB5-0x47FBCA). Note the not-found path leaves SP0 = input x, not 0 (original behaviour).

### 0x47FCA7 02C2 COMMAND_CAR_GOTO_COORDINATES_ACCURATE  (gr: unregistered, used: 0 / 8 / 0)
- `Collect(4)`: SP0 vehicle, SP1..SP3 x,y,z. `v = VEH(SP0)` (no null check). `z = SP3` (f32 local); if `z <= -100.0f`:
  `z = FindGroundZForCoord(x, y)`. `z = GetDistanceFromCentreOfMassToBaseOfModel(v) 0x536BE0 + z` (fadd at 0x47FD07; stored f32).
- `ok = CCarCtrl::JoinCarWithRoadSystemGotoCoors(v, &CVector(x, y, z), 0, 0)` 0x42F870 (cdecl, 4 args).
- `m = v->m_nCarMission (+0x3BA)`; if m is not 0x39/0x3A: `+0x3BA <- ok ? 0x0D : 0x0C` (0x47FD31-0x47FD54).
- `v[+0x36] = (v[+0x36] & 7) | 0x18` (STATUS_PHYSICS); `SetEngineOn(v, 1)` 0x41BDD0;
  `+0x3D0 m_nCruiseSpeed = max(m_nCruiseSpeed, 1)` (unsigned, 0x47FD6E-0x47FD80); `+0x3AC <- [0xB7CB84]` (timer). AL = 0.

### 0x480008 02D3 COMMAND_BOAT_GOTO_COORDS  (gr: unregistered, used: 0 / 1 / 0)
- `Collect(4)`: SP0 boat, SP1..SP3 x,y,z. `v = VEH(SP0)` (no null check). `zIn = SP3`.
- If `zIn <= -100.0f` (NaN skips): `CWaterLevel::GetWaterLevel(x, y, zIn, &wl, 1, 0)` 0x6EB690 (cdecl 6 args; `wl` = the local
  `[esp+0x54]` that the dispatcher zeroed at 0x47FA4B).
- ORIGINAL BUG: the destination z is always `wl` (`fld [esp+0x54]` at 0x480071 -> `+0x3F4`), never SP3: with a normal z (>
  -100) the boat's target z is 0.0 (from the dispatcher's zero-init), only with z <= -100 it is the water level. Port exactly
  (keep the dispatcher's `[esp+0x54] = 0` semantics: wl starts at 0.0 each call).
- If `m_nCarMission` is not 0x39/0x3A: `<- 0x0E` (0x48007D). `+0x3EC <- x, +0x3F0 <- y, +0x3F4 <- wl`; status `(b & 7) | 0x18`;
  `SetEngineOn(v, 1)`; cruise speed `= max(cruise, 1)`; `+0x3AC <- [0xB7CB84]`. AL = 0.

### 0x4800E5 02D4 COMMAND_BOAT_STOP  (gr: unregistered, used: 0 / 3 / 0)
- `Collect(1)`; `v = VEH(SP0)` (unchecked). If mission not 0x39/0x3A: `+0x3BA <- 0`. `+0x36 = (b & 7) | 0x18`;
  `+0x428 &= 0xEF` (engine off, directly - not via SetEngineOn); `+0x3D0 <- 0`.

### 0x480288 02DB COMMAND_SET_BOAT_CRUISE_SPEED  (gr: unregistered, used: 0 / 6 / 0)
- `Collect(2)`; `s = (u8) ftol(SP1 as f32)` (0x480297; truncation, low byte kept); `VEH(SP0)->m_nCruiseSpeed (+0x3D0) <- s`
  (unchecked). Note ftol is evaluated before the pool lookup.

### 0x48056A 02E1 COMMAND_CREATE_MONEY_PICKUP  (gr: unregistered, used: 0 / 3 / 1)
- `Collect(5)`: x SP0, y SP1, z SP2, amount SP3 (i32), permanent SP4 (i32). `z = SP2`; if `z <= -100.0f`: `z = FindGroundZForCoord(x, y)
  + [0x858B8C] (0.5f)` (fadd then fstp f32, 0x4805A7/0x4805B0).
- `CPickups::GetActualPickupIndex(CollectNoPC())` 0x4552A0 (cdecl) is called on the peeked next parameter (the output variable)
  and its result is discarded (EAX overwritten at 0x4805C1) - dead call, keep it (side-effect free read).
- `h = CPickups::GenerateNewOne(CVector(x, y, z), (u16) MI_MONEY [0x8CD59C], SP4 != 0 ? 0x13 PICKUP_MONEY_DOESNTDISAPPEAR : 8
  PICKUP_MONEY, SP3, 0, 0, 0)` 0x456F20 (cdecl, 9 dwords `add esp,0x24`); `SP0 <- h`; `Store(1)`.

### 0x4809B2 02F8 COMMAND_GET_CAR_FORWARD_X / 0x480A04 02F9 COMMAND_GET_CAR_FORWARD_Y  (gr: unregistered, used: 0 / 7 / 0 each)
- `Collect(1)`; `m = VEH(SP0)->m_matrix (+0x14)` (no null checks on vehicle or matrix). `fx = m[+0x10]`, `fy = m[+0x14]` (f32).
  `d = sqrt(fy*fy + fx*fx)` (x87: fy*fy computed first, then fx*fx, added as `fy² + fx²`, 0x4809DA-0x4809E4 / 0x480A2B-0x480A35).
  02F8: `SP0 <- fx / d`; 02F9: `SP0 <- fy / d` (fdivr, stored f32). `Store(1)`. Division by 0 gives inf/NaN (no guard).

### 0x4807CF 02ED COMMAND_SET_COLLECTABLE1_TOTAL  (gr: unregistered, used: 0 / 0 / 0)
- `Collect(1)`; `CWorld::Players[(u8) [0xB7CD74] CWorld::PlayerInFocus] + 0xC4 <- SP0` (address 0xB7CE5C + idx*0x190; gr
  m_nTotalNumCollectables).

### 0x480D28 030D COMMAND_SET_PROGRESS_TOTAL  (gr: unregistered, used: 1 / 0 / 0)
- `Collect(1)`; `CStats::SetStatValue(1 STAT_TOTAL_PROGRESS, (float) SP0)` 0x55A070 (fild i32 -> f32 store, 0x480D31).

### 0x480EF1 02E5 CREATE_CUTSCENE_OBJECT, 02E6 SET_CUTSCENE_ANIM, 02F3 LOAD_SPECIAL_MODEL, 0319 SET_CHAR_RUNNING  (gr: unimplemented, used: 0)
- Shared empty case: `xor al,al` and return (AL = 0, no parameters read). In the exe these four are PC no-ops that leave the IP
  on their parameters (never used by the shipped scripts). gr returns -1 (deviation; see dispatcher).

- gta-reversed: none of the unregistered opcodes has gr code. Open questions: none.
