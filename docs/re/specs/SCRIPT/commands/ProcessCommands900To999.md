# CRunningScript::ProcessCommands900To999 (SCRIPT) - script switch spec

Shared switch conventions (caller 0x469EB0/0x469F00, AL meaning, CRunningScript offsets, gr `unimplemented`
semantics): see ProcessCommands0To99.md, top section. Pool helpers (`__thiscall`, `ret 4`, null for stale handles):
0x4048E0 CPool<CVehicle>::GetAt on [0xB74494], 0x465040 CPool<CObject>::GetAt on [0xB7449C]. "-100 ground rule"
(fcom form): `fld z; fcom [0x859014] = -100.0; test ah,0x41; jp keep`; when `z <= -100.0`: `fstp st(0)` and
`z = CWorld::FindGroundZForCoord(x, y)` 0x569660 (ST0); then z is stored f32. NaN and `z > -100` keep z.
"Entity position" = `m_matrix (+0x14) ? m_matrix+0x30 : this+4`.

## 0x483BD0 CRunningScript::ProcessCommands900To999  (catalogue: specified/REA, port level 21)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x483BFB; command at [esp+4]), `ret 4`. SEH frame
  `push -1; push 0x838430; push fs:[0]; mov fs:[0],esp` (0x483BD0..0x483BE2), `sub esp,0x14C`, `push ebx; push ebp;
  push esi; push edi`. `eax = command + 0xFFFFFC7C` (= command - 0x384, 0x483BF2); `cmp eax,0x63; ja 0x485699`
  (0x483BF7/0x483BFD); byte index `movzx eax, byte [eax + 0x4857C4]` (0x483C03); jump table `jmp [eax*4 + 0x4856B8]`
  (0x483C0A, 67 entries).
- Common exit 0x48569B (every case jumps there with AL set): `nop; nop; jmp 0x404CFD` -> `mov ecx,[esp+0x15C]` (saved
  SEH link) -> 0x4856A2 `pop edi; pop esi; pop ebp; mov fs:[0],ecx; pop ebx; add esp,0x158; ret 4`.
- Default 0x485699: `or al,0xFF` + common exit -> 0xFF. Shared return-0: 0x485194 / 0x485695 (`xor al,al`).
  Shared compare tail 0x48517F: `if (AL) [esp+0x10] = 1; UpdateCompareFlag([esp+0x10]); return 0` (0x485188 =
  UpdateCompareFlag part). No case returns 1.
- Opcodes (100 = 0x384..0x3E7): 71 with a case; gta-reversed handler: 29, nop: 8 (not re-specified). Specified below: 34
  (32 unregistered + 0384 + 03DF).
  Default-reaching (29, all gr unimplemented -> -1, no params, same as the original): 0385..0389, 0398..039B, 039D, 03A0,
  03A5..03A6, 03AC, 03AE, 03B2..03B5, 03B8..03B9, 03BB, 03BE, 03C1..03C2, 03C6, 03DB, 03DD, 03E2. No gr `handler` opcode
  reaches the default.

### 0x485695 shared "return 0": 03A9, 03D4, 03DA, 03DF, 03E1
- No parameter read, AL = 0. 03A9/03D4/03DA/03E1 are gr nops (same). 03DF COMMAND_FORCE_RANDOM_PED_TYPE (gr: unimplemented,
  CharacterCommands.cpp:4481, used 0/0/0): gr returns -1. DEVIATION in the return value only.

### 0x4842EC 0395 COMMAND_CLEAR_AREA  (gr: unregistered, used: 53/934/26)
- `CollectParameters(5)`: [0] x, [1] y, [2] z, [3] radius, [4] bool. -100 ground rule (0x4842FB..0x484325).
  `CWorld::ClearExcitingStuffFromArea(CVector(x,y,z) (local, by ref), P3, (u8)P4)` 0x56A0D0 (cdecl, 0x484361). Return 0.

### 0x4851C7 03D0 COMMAND_HAS_MISSION_AUDIO_LOADED  (gr: unregistered, used: 12/752/39)
- `CollectParameters(1)`; `s = AudioEngine (0xB6BC90).GetMissionAudioLoadingStatus((u8)(P0 - 1))` 0x5072A0 (the slot is
  computed in AL with the upper EAX bytes zero, 0x4851D0..0x4851DE); `flag = (s == 1)` (`dec al; neg al; sbb al,al; inc al`,
  0x4851E4..0x4851EA); UpdateCompareFlag(flag). Return 0.

### 0x485228 03D2 COMMAND_HAS_MISSION_AUDIO_FINISHED  (gr: unregistered, used: 4/538/11)
- `CollectParameters(1)`; `flag = AudioEngine.IsMissionAudioSampleFinished((u8)(P0 - 1))` 0x5072C0 `!= 0`; UpdateCompareFlag. Return 0.

### 0x4841E3 0392 COMMAND_SET_OBJECT_DYNAMIC  (gr: unregistered, used: 4/96/9)
- `CollectParameters(2)`; `obj = ObjectPool.GetAt(P0)` (no null check); `st = obj->+0x1C & 4` (entity flag bit 2, bIsStatic).
  P1 != 0: if st set -> `obj->vtable[4](0)` (+0x10, SetIsStatic(false); 0x48421A) and `CPhysical::AddToMovingList` 0x542800;
  else nothing. P1 == 0: if st clear -> `obj->vtable[4](1)` (0x484239) and `CPhysical::RemoveFromMovingList` 0x542860; else
  nothing. Return 0.

### 0x4842AF 0394 COMMAND_PLAY_MISSION_PASSED_TUNE  (gr: unregistered, used: 8/130/3)
- `CollectParameters(1)`; if P0 is 1 or 2 (0x4842BD..0x4842C5): `AudioEngine.PreloadBeatTrack(P0 + 10)` 0x507F40 (0x4842D4),
  `AudioEngine.PlayPreloadedBeatTrack(1)` 0x507180 (0x4842E0). Other values: nothing. Return 0.

### 0x485003 03C7 COMMAND_SET_WANTED_MULTIPLIER  (gr: unregistered, used: 0/97/1)
- `CollectParameters(1)`; `CWorld::FindPlayerWanted(-1)` 0x56E230 `->m_fMultiplier (+0x14) = P0` (raw dword). Return 0.

### 0x484370 0396 COMMAND_FREEZE_ONSCREEN_TIMER  (gr: unregistered, used: 0/43/0)
- `CollectParameters(1)`; byte `[0xBA18D9] = (P0 != 0)` (CUserDisplay::OnscnTimer 0xBA1788 + 0x151, paused flag). Return 0.

### 0x48438F 0397 COMMAND_SWITCH_CAR_SIREN  (gr: unregistered, used: 0/61/0)
- `CollectParameters(2)`; `veh = GetAt(P0)` (no null check); byte +0x42D: `|= 0x80` if P1 != 0 else `&= 0x7F`
  (vehicleFlags bit 47, gr bSirenOrAlarm). Return 0.

### 0x484607 03A2 COMMAND_SET_CAR_STATUS  (gr: unregistered, used: 0/46/0)
- `CollectParameters(2)`; `veh = GetAt(P0)` (no null check). If `(veh->+0x36 & 0xF8) == 0x10` (status 2, STATUS_SIMPLE) and
  `P1 != 2`: `CCarCtrl::SwitchVehicleToRealPhysics(veh)` 0x423FC0 (0x484639). Then `veh->+0x36 = ((u8)P1 << 3) | (veh->+0x36 & 7)`
  (byte, 0x484646..0x484651). Return 0.

### 0x484B77 03BD COMMAND_REMOVE_SPHERE / 0x484AB4 03BC COMMAND_ADD_SPHERE  (gr: unregistered, used: 0/7/0, 0/3/0)
- 03BD: `CollectParameters(1)`; `CTheScripts::RemoveScriptSphere(P0)` 0x483BA0. Return 0.
- 03BC: `CollectParameters(4)`: [0] x, [1] y, [2] z, [3] radius. -100 ground rule (0x484AC3..0x484AEA).
  `CTheScripts::GetActualScriptThingIndex(CollectNextParameterWithoutIncreasingPC(), 0)` 0x4839A0 (0x484B2E) - result discarded.
  `h = CTheScripts::AddScriptSphere(id = (u32)this->m_IP + (u32)this, CVector(x,y,z), P3)` 0x483B30 (cdecl, 0x484B5A);
  `ScriptParams[0] = h`; StoreParameters(1). Return 0.

### 0x484C57 03C3 COMMAND_DISPLAY_ONSCREEN_TIMER_WITH_STRING  (gr: unregistered, used: 1/63/1)
- `idx = GetIndexOfGlobalVariable()` 0x464700; `CollectParameters(1)`: [0] direction; `ReadTextLabelFromScript(buf, 8)` (0x484C7F).
  `TheText.Get(buf)` 0x6A0050 (0x484C8E) - result discarded. `OnscnTimer (0xBA1788).AddClock((u16)idx, buf, P0 != 0)` 0x44CD50
  (0x484CA7; the label buffer is passed). Return 0.

### 0x484CB3 03C4 COMMAND_DISPLAY_ONSCREEN_COUNTER_WITH_STRING  (gr: unregistered, used: 0/51/3)
- `idx = GetIndexOfGlobalVariable()`; `CollectParameters(1)`: [0] counter type; `ReadTextLabelFromScript(buf, 8)`;
  `TheText.Get(buf)` discarded; `OnscnTimer.AddCounter((u16)idx, (u16)P0, buf, 0)` 0x44CDA0 (0x484CFC). Return 0.

### 0x484D08 03C5 COMMAND_CREATE_RANDOM_CAR_FOR_CAR_PARK  (gr: unregistered, used: 0/0/57)
- `CollectParameters(4)`: [0] x, [1] y, [2] z, [3] heading (degrees). No output.
- If `CCarCtrl::NumRandomCars [0x969094] >= 45` (0x484D11) return 0. `m = CPopulation::m_AppropriateLoadedCars (0xC0E9F8)
  .PickRandomCar(0, 1)` 0x611C50 (0x484D27); if `m == -1` return 0.
- Construct by `t = ms_modelInfoPtrs[m]->m_nVehicleType (+0x3C)`; `t - 1` indexes table 0x485828 (11 entries; `ja` for t == 0
  or t > 11 -> automobile): t=1 `new(0x99C)` CMonsterTruck::Constructor(m, 1) 0x6C8D60; t=2 `new(0x9BC)` CQuadBike 0x6CE370;
  t=3 `new(0xA18)` CHeli 0x6C4190; t=4 `new(0xA04)` CPlane 0x6C8E20; t=5..8 and default `new(0x988)` CAutomobile::Constructor(m, 1, 1)
  0x6B0A90; t=9 `new(0x814)` CBike 0x6BF430; t=10 `new(0x838)` CBmx 0x6BF820; t=11 `new(0x9F4)` CTrailer 0x6D03A0
  (`operator new` 0x6E2D50; EH state [esp+0x164] = 0..7 per kind, -1 after at 0x484F0D). For bike and bmx, after the ctor
  `veh->m_nBikeFlags (+0x614) |= 0x10` (0x484E4A / 0x484E80). If `new` returns 0: non-bike kinds continue with veh = 0
  (0x484EEE); bike/bmx go to 0x484E89 which writes `[0 + 0x614]` (null-pointer write, original bug, only on OOM).
- Placement: `x = P0`, `y = P1`; `z = veh->vtable[0xD4/4]() (ST0) + P2` (0x484F1C/0x484F22, stored f32); `veh->SetPosn(CVector&)`
  0x4241C0 (0x484F49); `veh->SetHeading(P3 * [0x8595EC] = 0.017453292)` 0x43E0C0 (0x484F60);
  `CTheScripts::ClearSpaceForMissionEntity(pos, veh)` 0x486B00.
- State: `+0x36 = (+0x36 & 7) | 0x20` (status 4, abandoned); byte +0x428 `&= 0xF7` (bit 3 bIsLocked cleared); byte +0x42B `|= 2`
  (bit 25 bIsCarParkVehicle) (0x484F70..0x484F94); `CCarCtrl::JoinCarWithRoadSystem(veh)` 0x42F5A0 (0x484F9A). If
  `m_autoPilot.m_nCarMission (+0x3BA)` is not 0x39/0x3A it becomes 0 (0x484FA8..0x484FB0). Byte +0x428 `&= 0xEF` (bit 4 bEngineOn
  cleared, stored at 0x484FEE); autopilot: +0x3BB m_nTempAction = 0, +0x3B9 m_nCarDrivingStyle = 0, +0x3C8 (autopilot+0x38,
  f32) = 9.0f (imm 0x41100000), +0x3D0 m_nCruiseSpeed = 9, +0x3B7 m_nCurrentLane = 0, +0x3B8 m_nNextLane = 0
  (0x484FC1..0x484FE7). `CWorld::Add(veh)` 0x563220. Return 0 (no handle stored, no mission cleanup).
- Open question 0x484F1C: name of vehicle virtual +0xD4 (slot 53, returns float in ST0); the port calls that slot.

### 0x4843DA 039C COMMAND_SET_CAR_WATERTIGHT  (gr: unregistered, used: 0/10/0)
- `CollectParameters(2)`; `veh = GetAt(P0)` (no null check). If `veh->m_nVehicleType (+0x590) == 0`: byte +0x868 (CAutomobile
  autoFlags) `|= 4` if P1 != 0 else `&= 0xFB` (bit 2 bWaterTight). Else if type == 9 (bike): byte +0x614 (CBike m_nBikeFlags)
  `|= 4` / `&= 0xFB`. Other types: nothing. Return 0.

### 0x484562 03A1 COMMAND_DRAW_SPHERE  (gr: unregistered, used: 0/10/0)
- `CollectParameters(4)`: [0] x, [1] y, [2] z, [3] radius. -100 ground rule (0x484571..0x48459B).
  `C3dMarkers::PlaceMarkerSet(id = (u32)this->m_IP + (u32)this, 1, pos, P3, 0xFF, 0, 0, 0xE4, 0x800, 0.1f, 0)` 0x725BA0
  (cdecl, 11 dwords, `add esp,0x2C` at 0x4845FD; immediates at 0x4845AF..0x4845D4). Return 0.

### 0x4844B8 039F COMMAND_TURN_CAR_TO_FACE_COORD  (gr: unregistered, used: 0/21/2)
- `CollectParameters(3)`: [0] vehicle, [1] x, [2] y. `veh = GetAt(P0)` (no null check); p = entity position.
- `dx = p.x - P1` (stored f32, 0x484504); `dy = p.y - P2` (0x484516); `a = CGeneral::GetATanOfXY(dx, dy)` 0x53CC70 (cdecl, ST0)
  `+ [0x858FE4] = 1.5707964` (0x484527), stored f32; if `a > [0x858CBC] = 6.2831855` (`test ah,0x41; jne` skips for <=, NaN):
  `a -= 6.2831855` (stored f32). `veh->SetHeading(a)` 0x43E0C0 (0x484556). No UpdateRwMatrix. Return 0.

### 0x483C8C 038A COMMAND_IS_POINT_OBSCURED_BY_A_MISSION_ENTITY  (gr: unregistered, used: 0/27/1)
- `CollectParameters(6)`: [0] x, [1] y, [2] z, [3] rx, [4] ry, [5] rz.
- x87 (0x483C95..0x483CFA): `minX = x - rx` (f32), `maxX = rx + x`, `minY = y - ry`, `maxY = ry + y` (each stored f32);
  `zlo = z - rz`, `zhi = rz + z` (kept in ST0/ST1; zhi also stored f32). For each axis, if `min > max` (`test ah,0x41; jne`
  skips for <, == and NaN) the two are swapped (0x483D0D..0x483D19, 0x483D2C..0x483D38, 0x483D47..0x483D51). Corner A =
  (minX, minY, lower z) at esp+0x88, corner B = (maxX, maxY, higher z) at esp+0xAC.
- `CWorld::FindMissionEntitiesIntersectingCube(A, B, &n (i16), 2, nullptr, 1, 1, 1)` 0x569240 (cdecl, 8 dwords, 0x483DB3).
  `flag = (n > 0)` (signed i16, 0x483DBB); UpdateCompareFlag(flag). Return 0.

### 0x485261 03D3 COMMAND_GET_CLOSEST_CAR_NODE_WITH_HEADING  (gr: unregistered, used: 0/6/4)
- `CollectParameters(3)`: [0] x, [1] y, [2] z. -100 ground rule (0x485270..0x48529A). Output StoreParameters(4) + flag.
- `node = ThePaths (0x96F050).FindNodeClosestToCoors(CVector(x,y,z), 0, 999999.875f (imm 0x497423FE), 0, 1, 0, 0, 0)` 0x44F460
  (sret, 0x4852E5). `c = ThePaths.FindNodeCoorsForScript(node, &found)` 0x4505E0 (0x4852FC). If found: `ScriptParams[0..2] = c`,
  `ScriptParams[3] = ThePaths.FindNodeOrientationForCarPlacement(node)` 0x450320 (ST0 stored f32 directly into 0xA43C84,
  0x485331), flag 1. Else ScriptParams[0..3] = 0, flag 0. StoreParameters(4), UpdateCompareFlag(flag). Return 0.

### 0x4854B9 03DC COMMAND_ADD_BLIP_FOR_PICKUP  (gr: unregistered, used: 0/36/0)
- `CollectParameters(1)`: [0] pickup handle. `CPickups::GetActualPickupIndex(P0)` 0x4552A0 (0x4854C9) - result discarded;
  `CRadar::GetActualBlipArrayIndex(CollectNextParameterWithoutIncreasingPC())` - discarded.
  `h = CRadar::SetEntityBlip(7 /*PICKUP*/, P0, 6, 3, this->m_szName)` 0x5839A0 (0x4854EE); `ChangeBlipScale(h, 3)` 0x583CC0;
  `ScriptParams[0] = h`; StoreParameters(1). Return 0.

### 0x48470F 03AB COMMAND_SET_CAR_STRONG  (gr: unregistered, used: 0/15/0)
- `CollectParameters(2)`; `veh = GetAt(P0)`; byte +0x429 `|= 0x80` if P1 != 0 else `&= 0x7F` (vehicleFlags bit 15, gr
  bTakeLessDamage). Return 0.

### 0x4849EB 03BA COMMAND_CLEAR_AREA_OF_CARS  (gr: unregistered, used: 0/14/10)
- `CollectParameters(6)`: corners (P0,P1,P2) and (P3,P4,P5). Per axis, swap when `first > second` (0x484A17..0x484A83; f32
  compares, NaN/equal keep). `CWorld::ClearCarsFromArea(minX, minY, minZ, maxX, maxY, maxZ)` 0x566610 (cdecl, 0x484AA5). Return 0.

### 0x4847AD 03B0 COMMAND_IS_GARAGE_OPEN / 0x4847E4 03B1 COMMAND_IS_GARAGE_CLOSED  (gr: unregistered, used: 5/13/0, 0/19/0)
- `ReadTextLabelFromScript(buf, 8)`; `n = CGarages::GetGarageNumberByName(buf)` 0x447680; `flag = 0`; if `(i16)n < 0` (`test ax,ax;
  jl 0x485188`) -> UpdateCompareFlag(0). Else `CGarages::IsGarageOpen(n)` 0x447D00 resp. `IsGarageClosed(n)` 0x447D30 (the whole
  EAX is pushed; the callee takes a 16-bit index), AL != 0 -> flag 1 (0x48517F). UpdateCompareFlag(flag). Return 0.

### 0x485037 03C9 COMMAND_IS_CAR_VISIBLY_DAMAGED  (gr: unregistered, used: 0/6/0)
- `CollectParameters(1)`; `flag = GetAt(P0)->+0x42A & 1` (vehicleFlags bit 16, gr bIsDamaged; no null check; 0x485057 -> 0x485181);
  UpdateCompareFlag. Return 0.

### 0x4853D9 03D7 COMMAND_SET_MISSION_AUDIO_POSITION  (gr: unregistered, used: 0/11/9)
- `CollectParameters(4)`: [0] slot, [1..3] position. `AudioEngine.SetMissionAudioPosition((u8)(P0 - 1), &CVector(P1,P2,P3))`
  0x507300 (0x485420). Return 0.

### 0x483DF6 038C COMMAND_ADD_TO_OBJECT_VELOCITY  (gr: unregistered, used: 0/9/1)
- `CollectParameters(4)`: [0] object, [1..3] velocity delta (per second). `obj = ObjectPool.GetAt(P0)` (no null check).
  `v = obj->m_vecMoveSpeed (+0x44)` copied; `v.x = P1 * [0x858B38] = 0.02 + v.x`, then y, then z (each `fld P; fmul; fadd v;
  fstp f32`, 0x483E10..0x483E6E); written back to +0x44/+0x48/+0x4C. Return 0.

### 0x48478E 03AF COMMAND_SWITCH_STREAMING  (gr: unregistered, used: 0/8/0)
- `CollectParameters(1)`; `CStreaming::ms_disableStreaming [0x9654B0] = (P0 == 0)`. Return 0.

### 0x48481B 03B6 COMMAND_SWAP_NEAREST_BUILDING_MODEL  (gr: unregistered, used: 0/3/0)
- `CollectParameters(6)`: [0] x, [1] y, [2] z, [3] radius, [4] old model, [5] new model.
- `fld z; fcomp -100; jp` (0x484841..0x484850): if `z <= -100.0`, `z = FindGroundZForCoord(x, y)` (stored f32). `best = P3 + P3`
  (`fadd st(0),st(0)`, f32 at esp+0x28); `bestEnt = 0`. Old/new model: a negative value is replaced by
  `CTheScripts::UsedObjectArray[-v].nModelIndex` (0xA44B88 - v*0x1C, 0x484887 / 0x4848A2).
- `CWorld::FindObjectsOfTypeInRange(old, pos, P3, 1, &n (i16), 16, list, 1, 0, 0, 0, 0)` 0x564C70 (12 dwords, 0x4848F8).
  If `n <= 0` return 0. For i = 0 .. n-1 (u16 counter): e = list[i]; `d = entityPos(e) - pos` (each component stored f32);
  `m = CVector::Magnitude(&d)` 0x4082C0; if `m < best` (`test ah,5; jp`, strict, NaN fails): `best = m` (f32), `bestEnt = e`.
- If bestEnt: `CBuilding::ReplaceWithNewModel(bestEnt, new)` 0x403EC0 (thiscall); `CTheScripts::AddToBuildingSwapArray(bestEnt,
  old, new)` 0x481140 (cdecl). Return 0.

### 0x48542C 03D8 COMMAND_ACTIVATE_SAVE_MENU  (gr: unregistered, used: 1/0/0)
- No params. If `CWorld::FindPlayerPed(-1)` 0x56E210 != 0 and its `m_nPedFlags (+0x46C) & 0x100` (bInVehicle) is clear: byte
  `[0xBA67A7] = 1` (FrontEndMenuManager 0xBA6748 + 0x5F, m_bSaveMenuActive); `FindPlayerPed(-1)->SetVelocity(CVector(0,0,0))`
  0x441130; `FindPlayerPed(-1)->SetTurnSpeed(CVector(0,0,0))` 0x45AFB0 (four FindPlayerPed calls). Return 0.

### 0x48548A 03D9 COMMAND_HAS_SAVE_GAME_FINISHED  (gr: unregistered, used: 1/0/0)
- `flag = ([0xBA67A4] == 0 && [0xBA67A7] == 0)` (menu manager +0x5C m_bMenuActive, +0x5F m_bSaveMenuActive; 0x48548A..0x4854A1);
  UpdateCompareFlag. Return 0.

### 0x483C11 0384 COMMAND_PRINT_STRING_IN_STRING_NOW  (gr: unimplemented, TextCommands.cpp:378, used: 0/5/0)
- Parameters: `ReadTextLabelFromScript(buf, 8)`; `t1 = TheText.Get(buf)` 0x6A0050; `ReadTextLabelFromScript(buf, 8)` (same buffer);
  `t2 = TheText.Get(buf)`; `CollectParameters(2)`: [0] time, [1] flags.
- Behaviour: `CMessages::AddMessageJumpQWithString(t1, P0, (u16)P1, t2, CTheScripts::bAddNextMessageToPreviousBriefs [0xA4448B] != 0)`
  0x69E950 (cdecl, 0x483C76); then `bAddNextMessageToPreviousBriefs = 1` (0x483C7E). Return 0.
- gta-reversed: registered as unimplemented -> returns OR_INTERRUPT without reading the 4 operands. DEVIATION (keep original):
  this opcode is used by 5 mission scripts, which would stall/misparse under gr.

### 0x4849CC 03B7 COMMAND_SWITCH_WORLD_PROCESSING  (gr: unregistered, used: 0/0/0)
- `CollectParameters(1)`; `CWorld::bProcessCutsceneOnly [0xB7CD6D] = (P0 == 0)`. Return 0.

Open questions: 0x484F1C (vehicle virtual +0xD4 name, see 03C5).
