# CRunningScript::ProcessCommands1200To1299 (SCRIPT) - script switch spec

Shared switch conventions (caller 0x469EB0/0x469F00, AL meaning, CRunningScript offsets, gr `unimplemented`
semantics): see ProcessCommands0To99.md, top section. Pool helpers (`__thiscall` on the pool, `ret 4`, null for stale
handles): 0x4048E0 CPool<CVehicle>::GetAt on [0xB74494], 0x465040 CPool<CObject>::GetAt on [0xB7449C].
"-100 ground rule": `z <= -100.0` ([0x859014] = -100.0, `test ah,0x41; jp`) -> `z = CWorld::FindGroundZForCoord(x, y)`
0x569660 (NaN and `z > -100` keep z). `ThePaths` = 0x96F050 (CPathFind, `__thiscall`).

## 0x48B590 CRunningScript::ProcessCommands1200To1299  (catalogue: specified/REA, port level 19)
- Dispatcher: `__thiscall` (this = ECX -> ESI at 0x48B5C4; command at [esp+4], copied to EDI at 0x48B5AF), `ret 4`.
  SEH frame: `push -1; push 0x8384BA; push fs:[0]; mov fs:[0],esp` (0x48B590..0x48B59E), `sub esp,0xB8`, then
  `push ebx; push ebp; push esi; push edi`. Prologue also sets `ebx = 0xFFFF` and initialises two u16 locals
  `[esp+0x4C] = [esp+0x50] = 0xFFFF` (0x48B5B6..0x48B5CB; CNodeAddress areaIds used by 04B9).
  `eax = command - 0x4B0` (0x48B5BB); `cmp eax,0x63; ja 0x48CC3D` (0x48B5C1/0x48B5D0); byte index table
  `movzx eax, byte [eax + 0x48CD64]` (0x48B5D6); jump table `jmp [eax*4 + 0x48CC5C]` (0x48B5DD, 66 entries).
- Common exit 0x48CC3F (every case jumps there with AL set): `nop; nop; jmp 0x408115` -> 0x408115
  `mov ecx,[esp+0xC8]` (saved SEH link) `jmp 0x48CC46` -> `pop edi; pop esi; pop ebp; mov fs:[0],ecx; pop ebx;
  add esp,0xC4; ret 4`. (The detour through 0x408115 is a no-CD-build patch site; same behaviour.)
- Default 0x48CC3D: `or al,0xFF` then the common exit -> 0xFF. Shared return-0: 0x48CC39 / 0x48CB0F (`xor al,al; jmp
  0x48CC3F`). Shared compare tails: 0x48B8ED = `UpdateCompareFlag([esp+0x10]); return 0`; 0x48C48A = same with
  [esp+0x10]. No case returns 1.
- Opcodes (100): 73 with a case; gta-reversed handler: 29, nop: 4 (not re-specified). Specified below: 40 (37
  unregistered + 3 unimplemented in the shared 0x48CC39 case).
  Default-reaching (27): 04C2..04C3, 04C6, 04C9..04CD, 04CF, 04D4, 04DC, 04DE, 04E8, 04EC, 04F2..04F3, 04F6, 04FB, 04FD,
  04FF, 0502, 0504..0505, 050C..050D, 0510..0511. All gr unimplemented (same -1, no params) EXCEPT
  **04DE COMMAND_SET_CHAR_ARMOUR: gr registers a real handler** (CharacterCommands.cpp:4200, `SetCharArmour(CPed&, float)`
  sets `ped.m_fArmour`). The original has no case for 04DE: it returns 0xFF without consuming the two parameters and
  without touching the ped. DEVIATION (keep original). No shipped script uses 04DE (absent from script-usage.csv).

### 0x48CC39 shared "return 0": 04BC, 04BE, 04BF, 04C7, 04F5, 0507, 050B
- No parameter read; `xor al,al; jmp 0x48CC3F` -> 0. 04BC/04BE/04BF/050B are gr nops (same).
  04C7 SWITCH_SECURITY_CAMERA, 04F5 SET_CHAR_AS_PLAYER_FRIEND, 0507 SWITCH_LIFT_CAMERA (gr: unimplemented,
  CameraCommands.cpp:330/332, CharacterCommands.cpp:4428; used 0/0/0): gr returns -1. DEVIATION in the return value only.

### 0x48C3D5 04EF COMMAND_REMOVE_ANIMATION  (gr: unregistered, used: 11/338/69)
- `ReadTextLabelFromScript(buf = esp+0x78, 16)` 0x463D50 (0x48C3DE). `idx = CAnimManager::GetAnimationBlockIndex(buf)`
  0x4D3990 (cdecl, 0x48C3E8). If `CTheScripts::ScriptResourceManager (0xA485A8).RemoveFromResourceManager(idx, 1, this)`
  0x470510 (0x48C3FB) returns true: `CStreaming::SetMissionDoesntRequireAnim(idx)` 0x48B570 (0x48C409). Return 0.

### 0x48C2F8 04EB COMMAND_TASK_TOGGLE_DUCK  (gr: unregistered, used: 0/301/0)
- `CollectParameters(2)` (0x48C2FC): [0] ped handle, [1] toggle.
- `p = CTask::operator new(0xC)` 0x61A5A0 (0x48C303); EH state [esp+0xD0] = 0 (0x48C311); if p != 0
  `CTaskSimpleDuckToggle::Constructor(p, P1)` 0x62F690 (thiscall, 0x48C327) else task = 0. EH state = -1 (0x48C33A).
  `this->GivePedScriptedTask(P0, task, command)` 0x465C20 (thiscall, pushes command (EDI), task, P0; 0x48C345). Return 0.

### 0x48B98D 04BB COMMAND_SET_AREA_VISIBLE  (gr: unregistered, used: 3/248/8)
- `CollectParameters(1)`; `CGame::currArea [0xB72914] = P0` (0x48B99C); `CStreaming::RemoveBuildingsNotInArea(P0)`
  0x4094B0 (0x48B9A1). Return 0.

### 0x48C351 04ED COMMAND_REQUEST_ANIMATION  (gr: unregistered, used: 15/222/71)
- `ReadTextLabelFromScript(buf, 16)` (0x48C35A); `idx = GetAnimationBlockIndex(buf)` (0x48C364);
  `CStreaming::RequestModel(idx + 0x63E7, 4)` 0x4087E0 (0x48C374; 0x63E7 = 25575 = first IFP streaming id);
  `ScriptResourceManager.AddToResourceManager(idx, 1, this)` 0x4704B0 (0x48C385). Return 0. idx is not checked for -1.

### 0x48C391 04EE COMMAND_HAS_ANIMATION_LOADED  (gr: unregistered, used: 1/184/70)
- `ReadTextLabelFromScript(buf, 16)`; `blk = CAnimManager::GetAnimationBlock(buf)` 0x4D3940 (0x48C3A4, no null check);
  `flag = (blk->+0x10 (bLoaded, u8) == 1)` (0x48C3A9..0x48C3BD); UpdateCompareFlag(flag). Return 0.

### 0x48B900 04BA COMMAND_SET_CAR_FORWARD_SPEED  (gr: unregistered, used: 0/187/0)
- `CollectParameters(2)` (0x48B904): [0] vehicle, [1] speed (f32, units per second).
- `veh = VehiclePool.GetAt(P0)` (no null check). `s = P1 * [0x859044] = 0.016666668` (x87, stored f32 as argument,
  0x48B91B..0x48B930). `v = veh->m_matrix->GetForward() (m_matrix (+0x14) + 0x10) * s` via 0x40FEC0 (CVector*float, sret into
  a local, 0x48B93C; m_matrix used without null check); `veh->m_vecMoveSpeed (+0x44) = v` (0x48B941..0x48B951).
- If `veh->m_pHandlingData (+0x384)->m_nModelFlags (+0xCC) & 0x2000000` (MODEL_IS_HELI, 0x48B954..0x48B963) and
  `veh->m_nVehicleType (+0x590) == 0` (0x48B96E..0x48B976): `((CAutomobile*)veh)->m_fHeliRotorSpeed (+0x84C) = 0.22f`
  (imm 0x3E6147AE, 0x48B97C). Return 0.

### 0x48C1B0 04E0 COMMAND_SET_CAR_STRAIGHT_LINE_DISTANCE  (gr: unregistered, used: 0/40/3)
- `CollectParameters(2)`; `veh = GetAt(P0)` (no null check); `veh->m_autoPilot.m_nStraightLineDistance (+0x390+0x4D = +0x3DD)
  = (u8)P1` (0x48C1D0). Return 0.

### 0x48C244 04E4 COMMAND_REQUEST_COLLISION  (gr: unregistered, used: 3/70/10)
- `CollectParameters(2)`: [0] x, [1] y. Local `CVector v(P0, P1, 0)` (0x48C25E..0x48C275);
  `CColStore::RequestCollision(v, CGame::currArea [0xB72914])` 0x410C00 (cdecl, 0x48C280). Return 0.

### 0x48C28F 04E5 COMMAND_LOCATE_OBJECT_2D / 04E6 COMMAND_LOCATE_OBJECT_3D  (gr: unregistered, used: 0/4/2, 2/47/2)
- `this->LocateObjectCommand(command)` 0x487D10 (thiscall, `push edi` = full opcode; 0x48C292); the callee reads all
  parameters (6 resp. 8). Return 0.

### 0x48C52F 04F7 COMMAND_DISPLAY_NTH_ONSCREEN_COUNTER_WITH_STRING  (gr: unregistered, used: 4/34/13)
- `idx = GetIndexOfGlobalVariable()` 0x464700 (0x48C531); `CollectParameters(2)` (0x48C53C): [0] counter type, [1] slot
  (1-based); `ReadTextLabelFromScript(buf = esp+0x54, 8)` (0x48C54A).
- `TheText.Get(buf)` 0x6A0050 (0x48C559): result DISCARDED. `CUserDisplay::OnscnTimer (0xBA1788).AddCounter((u16)idx,
  (u16)P0, buf, P1 - 1)` 0x44CDA0 (0x48C57D; the label buffer is passed, not the GXT string). Return 0.

### 0x48BC0A 04CE COMMAND_ADD_SHORT_RANGE_SPRITE_BLIP_FOR_COORD  (gr: unregistered, used: 11/87/30)
- `CollectParameters(4)`: [0] x, [1] y, [2] z, [3] sprite. -100 ground rule (0x48BC22..0x48BC43). Discarded
  `CRadar::GetActualBlipArrayIndex(CollectNextParameterWithoutIncreasingPC())` (0x48BC51/0x48BC57).
  `h = CRadar::SetShortRangeCoordBlip(4 /*COORD*/, CVector(x,y,z), 5 /*colour*/, 3 /*BOTH*/, this->m_szName)` 0x583920
  (0x48BC9A); `CRadar::SetBlipSprite(h, P3)` 0x583D70 (0x48BCA8); `add esp,0x24`; `ScriptParams[0] = h`;
  StoreParameters(1). Return 0.

### 0x48C1DD 04E1 COMMAND_POP_CAR_BOOT  (gr: unregistered, used: 0/7/0)
- `CollectParameters(1)`; `CAutomobile::PopBoot(GetAt(P0))` 0x6AF910 (no null/type check). Return 0.

### 0x48C2E9 04E9 COMMAND_IS_OBJECT_IN_AREA_2D / 04EA COMMAND_IS_OBJECT_IN_AREA_3D  (gr: unregistered, used: 0/0/0, 0/1/3)
- `this->ObjectInAreaCheckCommand(command)` 0x489150 (thiscall, `push edi`; 0x48C2EC). Return 0.

### 0x48C768 04FE COMMAND_BURST_CAR_TYRE  (gr: unregistered, used: 0/21/0)
- `CollectParameters(2)`: [0] vehicle, [1] tyre. `veh = GetAt(P0)` (no null check).
- If `veh->m_nVehicleType (+0x590) == 9` (BIKE, 0x48C785): `w = P1`; if `w == 2` then `w = 0`, else if `w == 3` then `w = 1`,
  and in those two cases `ScriptParams[1] = w` is written back (0x48C793..0x48C7A6); `veh->vtable[0xB0/4](w, 1)` (slot 44,
  CVehicle::BurstTyre(component, bPhysicalEffect); full dword pushed). Else: `veh->vtable[44]((u8)P1, 1)` (0x48C7BD..0x48C7CA).
  Return 0.

### 0x48C65A 04F9 COMMAND_SET_EXTRA_COLOURS / 0x48C69D 04FA COMMAND_CLEAR_EXTRA_COLOURS  (gr: unregistered, used: 1/4/12, 1/11/2)
- 04F9: `CollectParameters(2)`; `CTimeCycle::StartExtraColour(P0 - 1, P1 != 0 ? 1 : 0)` 0x55FEC0 (cdecl, 0x48C676 / 0x48C68E).
- 04FA: `CollectParameters(1)`; `CTimeCycle::StopExtraColour(P0 != 0 ? 1 : 0)` 0x55FF20 (0x48C6B1 / 0x48C6C2). Return 0.

### 0x48CB16 050E COMMAND_SORT_OUT_OBJECT_COLLISION_WITH_CAR  (gr: unregistered, used: 0/18/1)
- `CollectParameters(2)`: [0] object, [1] vehicle. `veh = VehiclePool.GetAt(P1)` (0x48CB2C, first); `obj = ObjectPool.GetAt(P0)`
  (0x48CB3F); `obj->+0x128 = veh` (0x48CB44; CObject field after CPhysical, gr name not validated - the "damaged by /
  collision-ignore entity"; plain store, no reference registration). No null checks. ESI is clobbered (not used after). Return 0.

### 0x48B9B0 04BD COMMAND_MARK_CAR_AS_CONVOY_CAR  (gr: unregistered, used: 0/15/0)
- `CollectParameters(2)`; `veh = GetAt(P0)`; byte +0x42B: P1 != 0 -> `|= 8` else `&= 0xF7` (vehicleFlags bit 27,
  gr bPartOfConvoy; 0x48B9DB / 0x48B9EB). Return 0.

### 0x48CA1C 0508 COMMAND_CLOSE_ALL_CAR_DOORS  (gr: unregistered, used: 0/19/0)
- `CollectParameters(1)`; `CAutomobile::CloseAllDoors(GetAt(P0))` 0x6A4520 (no null/type check). Return 0.

### 0x48BDA3 04D2 COMMAND_PLANE_GOTO_COORDS  (gr: unregistered, used: 0/16/0)
- `CollectParameters(6)`; `CAutomobile::TellPlaneToGoToCoors(GetAt(P0), P1, P2, P3, P4, P5)` 0x6A2470 (the five values are
  pushed before the GetAt call, 0x48BDBD..0x48BDD9). Return 0.

### 0x48C9F4 0506 COMMAND_SET_CAR_MODEL_COMPONENTS  (gr: unregistered, used: 0/7/1)
- `CollectParameters(3)`; `CVehicleModelInfo::ms_compsToUse[0] (0x8A6458) = (u8)P1`, `[1] (0x8A6459) = (u8)P2` (0x48CA09/
  0x48CA0F). P0 (model) is ignored. Return 0.

### 0x48C07B 04D9 COMMAND_SET_OBJECT_RECORDS_COLLISIONS  (gr: unregistered, used: 1/9/1)
- `CollectParameters(2)`; `obj = ObjectPool.GetAt(P0)` (no null check); +0x40 `|= 0x10000000` if P1 != 0 else
  `&= 0xEFFFFFFF` (CPhysical flags bit 28, gr bCanBeCollidedWith; 0x48C0A3 / 0x48C0B3). Return 0.

### 0x48C0C3 04DA COMMAND_HAS_OBJECT_COLLIDED_WITH_ANYTHING  (gr: unregistered, used: 1/6/1)
- `CollectParameters(1)`; `flag = 0`; `obj = GetAt(P0)` (no null check); if `obj->m_nNumEntitiesCollided (+0xB9, u8) != 0`
  (0x48C0E3..0x48C0EB) flag = 1; UpdateCompareFlag(flag). Return 0.

### 0x48BDED 04D3 COMMAND_GET_NTH_CLOSEST_CAR_NODE  (gr: unregistered, used: 0/6/1)
- `CollectParameters(4)`: [0] x, [1] y, [2] z, [3] n (1-based). Output StoreParameters(3) + compare flag.
- -100 ground rule, fcom/`fstp st(0)` form (0x48BDFC..0x48BE23); z stored at 0x48BE31.
  `a = ThePaths.FindNthNodeClosestToCoors(CVector(x,y,z), 0 /*nodeType*/, 999999.875f (imm 0x497423FE), 0, 1, P3 - 1, 0, 0,
  nullptr)` 0x44F8C0 (sret local at esp+0xCC; 0x48BE7E). `c = ThePaths.FindNodeCoorsForScript(a, &found)` 0x4505E0 (0x48BE95).
  If found: `ScriptParams[0..2] = c`, flag 1 (0x48BEA2..0x48BEBF); else `ScriptParams[0..2] = 0`, flag 0 (0x48BECB..0x48BEE6).
  Then (0x48B8E6) StoreParameters(3), UpdateCompareFlag(flag). Return 0.

### 0x48C45C 04F1 COMMAND_IS_CAR_WAITING_FOR_WORLD_COLLISION  (gr: unregistered, used: 0/6/0)
- `CollectParameters(1)`; flag = `GetAt(P0)->+0x1C & 0x40000` != 0 (entity flag bit 18, gr m_bIsStaticWaitingForCollision;
  0x48C47C; no null check); UpdateCompareFlag(flag). Return 0.

### 0x48C29E 04E7 COMMAND_IS_OBJECT_IN_WATER  (gr: unregistered, used: 0/5/0)
- `CollectParameters(1)`; `flag = 0`; `obj = GetAt(P0)`; if obj != 0 and `obj->+0x40 & 0x100` (bit 8, bSubmergedInWater;
  0x48C2C5/0x48C2C8) flag = 1. UpdateCompareFlag(flag). Return 0.

### 0x48B9FB 04C0 COMMAND_CREATE_SCRIPT_ROADBLOCK  (gr: unregistered, used: 0/5/0)
- `CollectParameters(7)`; `CRoadBlocks::RegisterScriptRoadBlock(CVector(P0,P1,P2), CVector(P3,P4,P5), P6 != 0)` 0x460DF0
  (cdecl, 7 dwords, 0x48BA7C). Return 0.

### 0x48C165 04DF COMMAND_SET_HELI_STABILISER  (gr: unregistered, used: 0/4/0)
- `CollectParameters(2)`; `veh = GetAt(P0)`; byte +0x42B `|= 0x10` if P1 != 0 else `&= 0xEF` (bit 28, gr bHeliMinimumTilt). Return 0.

### 0x48B7B5 04B9 COMMAND_GET_CLOSEST_STRAIGHT_ROAD  (gr: unregistered, used: 0/3/0)
- `CollectParameters(5)`: [0] x, [1] y, [2] z, [3] minDist, [4] maxDist. Output StoreParameters(7) + compare flag.
- `ThePaths.FindNodePairClosestToCoors(CVector(x,y,z), 0, &n1 (esp+0x4C), &n2 (esp+0x50), &d (esp+0x48), P3, P4, 1, 1, 0)`
  0x44FEE0 (0x48B81D). If `n1.areaId == 0xFFFF` (0x48B822, compares BX) -> fail. Else `c1 = FindNodeCoorsForScript(n1, nullptr)`
  0x4505E0 (0x48B83E) -> ScriptParams[0..2]; `c2 = FindNodeCoorsForScript(n2, &found)` (0x48B877); if found:
  ScriptParams[3..5] = c2, ScriptParams[6] = d, flag 1 (0x48B894..0x48B8DF). Fail paths: ScriptParams[0..6] = 0, flag 0
  (0x48B8B0..0x48B8DF; this also zeroes c1 already written). StoreParameters(7) (0x48B8E8), UpdateCompareFlag(flag). Return 0.

### 0x48BFC3 04D6 COMMAND_SET_ENABLE_RC_DETONATE_ON_CONTACT  (gr: unregistered, used: 0/3/0)
- `CollectParameters(1)`; `CVehicle::bDisableRemoteDetonationOnContact [0xC1CC01] = (P0 == 0)` (0x48BFD3). Return 0.

### 0x48BA8B 04C1 COMMAND_CLEAR_ALL_SCRIPT_ROADBLOCKS  (gr: unregistered, used: 0/3/0)
- `CRoadBlocks::ClearScriptRoadBlocks()` 0x460EC0. Return 0.

### 0x48CB51 050F COMMAND_GET_MAX_WANTED_LEVEL  (gr: unregistered, used: 0/7/0)
- `ScriptParams[0] = CWanted::MaximumWantedLevel [0x8CDEE4]`; StoreParameters(1). Return 0.

### 0x48BCC6 04D0 COMMAND_SET_HELI_ORIENTATION  (gr: unregistered, used: 0/1/0)
- `CollectParameters(2)`: [0] vehicle, [1] angle (degrees). `veh = GetAt(P0)` (ECX = veh from 0x48BCED).
- x87: `a = (P1 + [0x85991C] = 90.0) * [0x8595EC] = 0.017453292` (0x48BCE1..0x48BCEF); stored f32 [esp+0x48] (0x48BCFB).
  If `a < 0` (`test ah,5; jp` not taken): loop `a += [0x858CBC] = 6.2831855` while `a < 0` (0x48BD10..0x48BD21, in ST0), then
  store f32. Then `fcom 6.2831855; test ah,0x41; jne 0x48BD68`: if `a <= 2pi` or NaN, the stored f32 is passed; else loop
  `a -= 6.2831855` while `a > 2pi` (0x48BD40..0x48BD51), `fstp` f32. `CAutomobile::SetHeliOrientation(veh, a)` 0x6A2450
  (0x48BD5C / 0x48BD6F). Return 0.

### 0x48BEF2 04D5 COMMAND_DRAW_WEAPONSHOP_CORONA  (gr: unregistered, used: 0/1/0)
- `CollectParameters(9)`: [0] x, [1] y, [2] z, [3] size (f32), [4] corona type, [5] flare type, [6] r, [7] g, [8] b.
- -100 ground rule (fcom form, 0x48BF01..0x48BF29); pos local (x, y, z) at esp+0x94.
- `CCoronas::RegisterCorona(id = (u32)this->m_IP + (u32)this, nullptr, (u8)P6, (u8)P7, (u8)P8, 0xFF, &pos, P3, 150.0f,
  (u8)P4, (u8)P5, 1, 0, 0, 0.0f, 0, 0.2f, 0, 15.0f, 0, 0)` 0x6FC580 (21 dwords, `add esp,0x54` at 0x48BFB9; id computed at
  0x48BFAA..0x48BFAE; immediates 0x43160000, 0x3E4CCCCD, 0x41700000). Argument order is the gr type-overload order
  (id, attachTo, r, g, b, intensity, posn, radius, farClip, coronaType, flareType, enableReflection, checkObstacles,
  notUsed, angle, longDistance, nearClip, fadeState, fadeSpeed, onlyFromBelow, reflectionDelay). Return 0.

### 0x48C6D1 04FC COMMAND_GET_WHEELIE_STATS  (gr: unregistered, used: 1/0/0)
- `CollectParameters(1)`: [0] player index. `pi = &CWorld::Players[P0]` (0xB7CD98 + P0*0x190, 0x48C6E0).
  `ScriptParams[0..5] = pi->+0x118..+0x12C` (m_nBestCarTwoWheelsTimeMs, m_fBestCarTwoWheelsDistM, m_nBestBikeWheelieTimeMs,
  m_fBestBikeWheelieDistM, m_nBestBikeStoppieTimeMs, m_fBestBikeStoppieDistM; raw dwords); StoreParameters(6) (0x48C736);
  then the six fields are set to 0 (0x48C73D..0x48C755). Return 0.

Open questions: none (the CObject +0x128 field name in 050E is not validated by gr/plugin-sdk; the behaviour is a plain store).
