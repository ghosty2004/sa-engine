# CGangWars (POP) - full specs (group P4)

Globals (gr names, GangWars.h, all `.data`): pDriveByCar 0x96AB28 (CAutomobile*), GangRatingStrength[3] 0x96AB2C, GangRatings[3] 0x96AB38,
FightTimer 0x96AB44 (i32), TimeTillNextAttack 0x96AB48 (f32), State2 0x96AB4C (0 NO_ATTACK, 1 WAR_NOTIFIED, 2 PLAYER_CAME_TO_WAR), Gang2
0x96AB50, LastTimeInArea 0x96AB54 (u32), WarFerocity 0x96AB58 (i32), Provocation 0x96AB5C (f32), TimeStarted 0x96AB60 (u32), State 0x96AB64
(0 NOT_IN_WAR .. 6 THIRD_WAVE), aSpecificZones[] 0x96AB68 (i32), Difficulty 0x96AB80 (f32), Gang1 0x96AB84, pZoneToFightOver 0x96AB88 (CZone*),
pZoneInfoToFightOver 0x96AB8C, bGangWarsActive 0x96AB90, bTrainingMission 0x96AB91, bPlayerIsCloseby 0x96AB92,
bCanTriggerGangWarWhenOnAMission 0x96AB93, NumSpecificZones 0x96AB94, RadarBlip 0x96AB98, TerritoryUnderControlPercentage 0x96AB9C (f32),
bIsPlayerOnAMission 0x96ABA0, CoorsOfPlayerAtStartOfWar 0x96ABBC, PointOfAttack 0x96ABC8 (CVector).
Timer: m_snTimeInMilliseconds 0xB7CB84, m_snPreviousTimeInMilliseconds 0xB7CB78, ms_fTimeStep 0xB7CB5C, m_FrameCounter 0xB7CB4C.
Zones: NavigationZoneArray 0xBA3798 (CZone stride 0x20; +0x10/+0x12 x1/y1 i16, +0x16/+0x18 x2/y2 i16, +0x1C m_ZoneInfoIndex u16),
ZoneInfoArray 0xBA1DF0 (stride 0x11, +0..+9 GangStrength u8). Gang colour tables 0x8D1344 / 0x8D1350 / 0x8D135C (u8 x 12, see
CGangWars.deviations.md): colour(g) = `T1344[g] << 24 | T1350[g] << 16 | T135C[g] << 8 | 0xFF`.
Helpers: `_rand` 0x821B1E, `_ftol` 0x821B40 (truncation), CStreaming::ms_aInfoForModel 0x8E4CD0 (stride 0x14, +0 load state u8, 1 = LOADED),
CPools::GetPedRef 0x4442D0 (stub to 0x156CD40, `__thiscall` on the pool, `ret 4`: `((ped - base) / 0x7C4) << 8 | flags[idx]`).
k = 0x858C7C = 3.0518509e-05 (approx 1/32767). gta-reversed: `plugin::Call` for all three functions below.

## 0x443950 CGangWars::AddKillToProvocation  (catalogue: specified/REA, port level 1)
- Convention / signature: `__cdecl void (int32 pedType [esp+4])`, plain `ret`. Callers: CDarkel::RegisterKillByPlayer 0x43DCD0.
- Callees: CTheScripts::IsPlayerOnAMission 0x464D50.
- Behaviour:
  1. If `!bCanTriggerGangWarWhenOnAMission` (0x443950) and IsPlayerOnAMission() returns non-zero: return (0x443962).
  2. If pedType is not 7 and not 9 (PED_TYPE_GANG1 / GANG3 = Ballas / Vagos): return (0x443968-0x443970).
  3. If NumSpecificZones == 0: `Provocation += 1.0f [0x858624]` (f32 load/add/store, 0x4439A9-0x4439B5), return.
  4. Else for i = 0 .. NumSpecificZones-1 (signed, `jle` when <= 0): `zi = ZoneInfoArray + 0x11 * NavigationZoneArray[aSpecificZones[i]].m_ZoneInfoIndex`
     (u16 at 0xBA37B4 + 32*zone); if `zi[pedType - 7]` (byte at 0xBA1DE9 + pedType + 0x11*idx) != 0: Provocation += 1.0f and return.
- gta-reversed: provided but `.Reversed = false` (GangWars.cpp:93-111); same logic. Open questions: none.

## 0x444810 CGangWars::CreateAttackWave  (catalogue: specified/REA, port level 30)
- Convention / signature: `__cdecl bool (int32 warFerocity [esp+4], int32 waveId [esp+8])`, returns AL (1 = at least one ped created), plain
  `ret`. Entry `push -1; jmp 0x406AD9` (obfuscated `push` of the SEH handler 0x8378C7 = [0x1560840] - [0x5E5E6B]), SEH frame, `sub esp,0xC0`.
  F = esp after the 4 register pushes (0x44482E): arg1 = F+0xE0, arg2 = F+0xE4.
  Frame: F+0x13 anyCreated (u8), F+0x14 weaponA, F+0x18 weaponB, F+0x1C temp z / rand, F+0x20 group k, F+0x28 temp, F+0x2C..0x34 pickup
  pos Q, F+0x38..0x40 spacing vector S, F+0x44 player ped, F+0x48 gang of this group, F+0x4C ped offset m, F+0x50..0x58 player pos Pp,
  F+0x5C..0x64 candidate C (GenerateCarCreationCoors2 out), F+0x68..0x70 chosen point A, F+0x80..0x88 ped position, F+0x8C..0x94 previous
  point B, F+0x98/F+0xA0 node outs (u16), F+0xA4 ped model out, F+0xA8 float out, F+0xAC temp.
- Callers: CGangWars::Update 0x446610 (3 sites).
- Callees: CWorld::FindPlayerCoors 0x56E010, CWorld::FindPlayerPed 0x56E210, CPathFind::AreNodesLoadedForArea 0x44DD10 (thiscall ThePaths
  0x96F050), CGangWars::PedStreamedInForThisGang 0x4439D0, CStreaming::RequestModel 0x4087E0, sub_406E80 (stub to 0x1560E20: cdecl
  `(a, b, c) -> m_PedGroups[ 0x8D2540[3a + c] * 21 + b ]` (u16 0xC0F358), i.e. model b of the ped group of popcycle slot a / variant c),
  CCarCtrl::GenerateCarCreationCoors2 0x424210, CCamera::IsSphereVisible 0x420D40 (thiscall TheCamera 0xB6F028), CVector::Normalise 0x59C910,
  CGangWars::PickStreamedInPedForThisGang 0x443A20, CWorld::FindGroundZFor3DCoord 0x5696C0 (-> ST0), CPed::operator new 0x5E4720 (0x79C),
  CCivilianPed ctor 0x5DDB70, CPlaceable::SetPosn 0x4241C0, CPed::SetCharCreatedBy 0x5E47E0, CWorld::Add 0x563220, CTask::operator new
  0x61A5A0, CTaskComplexKillPedOnFoot ctor 0x620E30, CEventScriptCommand ctor 0x4B0A00 / dtor 0x4B0A50, CEventGroup::Add 0x4AB420,
  CGeneral::GetRandomNumberInRange<float> 0x41BD90 (cdecl (min, max) -> ST0), CPed::GiveWeapon 0x5E6080, CPed::SetCurrentWeapon 0x5E6280,
  CPools::GetPedRef 0x4442D0, CRadar::SetEntityBlip 0x5839A0, ChangeBlipScale 0x583CC0, ChangeBlipColour 0x583AB0, CPickups::GenerateNewOne
  0x456F20, CPopulation::PickGangCar 0x614490, CCarCtrl::GenerateOneEmergencyServicesCar 0x42B7D0, CEntity::RegisterReference 0x571B70,
  CCarCtrl::JoinCarWithRoadSystemGotoCoors 0x42F870, CCarCtrl::SetUpDriverAndPassengersForVehicle 0x4217C0,
  CPedIntelligence::SetPedDecisionMakerType 0x600B50, CPed::GiveDelayedWeapon 0x5E89B0, CTaskSimpleGangDriveBy ctor 0x6217D0.
- Behaviour:
  1. `Pp = FindPlayerCoors(-1)`, `F44 = FindPlayerPed(-1)`. Return 0 if `CGame::currArea` (0xB72914) != 0 (0x444850) or `Pp.z > 950.0f
     [0x858F4C]` (0x444862 `test ah,0x41; je`: NaN continues).
  2. Return 0 unless `ThePaths.AreNodesLoadedForArea(Pp.x - 20, Pp.x + 20, Pp.y - 20, Pp.y + 20)` (20.0f [0x858BA4], each stored f32).
  3. If `!PedStreamedInForThisGang(Gang1)`: `RequestModel(m_PedGroups[0x8D2618[Gang1*3] * 21 + 0], 8)` (inline form of sub_406E80 with
     a = Gang1 + 18, 0x4448CA). If `Gang2 != Gang1` and `!PedStreamedInForThisGang(Gang2)`: `RequestModel(sub_406E80(Gang2 + 18, 0, 0), 8)`.
     (Continues either way.)
  4. Weapon/model choice by `warFerocity` (unsigned `ja` > 5, table 0x4453B4) giving (modelA, weaponA, modelB, weaponB):
     0: (336, 5, 346, 22); 1: (346, 22, 352, 28); 2: (352, 28, 353, 29); 3: (353, 29, 355, 30); 4: (355, 30, 355, 30); 5: (348, 24, 355, 30).
     > 5 (0x4449DE): modelA = modelB = the dword at F+0x44 (the player ped POINTER) and weaponA/weaponB are left uninitialised: original bug
     (reachable only if Update passes WarFerocity + 1/+2 > 5, i.e. WarFerocity >= 4).
     If bTrainingMission: by warFerocity 0 -> (336, 5) for both; 1 -> (346, 22) both; other -> (372, 32) both (0x4449E6-0x444A2C).
  5. If `ms_aInfoForModel[modelA].loadState != 1` or same for modelB: `RequestModel(modelA, 0); RequestModel(modelB, 0)`; return 0 (0x445383).
     Return 0 if `!PedStreamedInForThisGang(Gang1)` or `!PedStreamedInForThisGang(Gang2)`.
  6. `B = (999999.875f, 999999.875f, 999999.875f)` (imm 0x497423FE), anyCreated = 0. For k = 0, 1 (0x444AB0, `cmp 2; jl`):
     a. gang = (k == 0) ? Gang1 : Gang2 (F48; Gang1 is re-read from the global at 0x444AA9). found (bl) = 0.
     b. Up to 20 attempts (esi counts 1..20, `cmp esi,0x14; jl`): node outs F98 = FA0 = 0xFFFF, then
        `GenerateCarCreationCoors2(Pp (by value), 1.0f, 0.0f, -1.0f, true, 50.0f, 50.0f, &C, &F98, &FA0, &FA8, false, true)`. If it returns 0:
        next attempt. Else if `!TheCamera.IsSphereVisible(&C, 7.0f)`: `A = C`, found = 1, and if
        `sqrt((dx*dx + dy*dy) + dz*dz) > 15.0f [0x858B48]` with `d = B - C` (dz, dy, dx computed first as f32-precision x87 values, 0x444B5F-0x444BB2)
        stop searching (use A); otherwise (<= 15 or NaN) continue attempts. If the sphere IS visible: if found == 0 { A = C; found = 1 }; continue.
     c. If found == 0 after the attempts: skip to the next k (no pickup, 0x444BEA).
     d. `B = A`. `S = (Pp.y - A.y, A.x - Pp.x, 0)` (note: perpendicular to the player->A direction; x87 at 0x444BF0-0x444C31), `S.Normalise()`,
        then `S.x *= 1.2f`, `S.y *= 1.2f`, `S.z *= 1.2f` [0x858F08] (each stored f32).
     e. `count = ftol((Difficulty * 0.3f [0x858C24] + 0.7f [0x858CB0]) * (int)(warFerocity + 3))` (`fimul`, 0x444C84). If bTrainingMission:
        count = 2. If warFerocity == 5: count = 10 (overrides training). Else if count <= 0: skip to step g.
     f. `m = -2 * (count / 2)` (signed division truncating toward 0, 0x444CB5-0x444CBE). Repeat count times (m += 2 each time):
        - `z0 = A.z + S.z*m` (F1C), `y0 = S.y*m + A.y`, `x0 = S.x*m + A.x`, each f32 (`fild m` once, products in x87, 0x444CD0-0x444D20).
        - `x = (float)rand() * k * 3.0f [0x858B3C] + x0 - 1.5f [0x858CE8]`; then `y = (float)rand() * k * 3.0f + y0 - 1.5f` (two rand calls,
          always consumed, 0x444D27-0x444D83).
        - If `!PickStreamedInPedForThisGang(gang, &FA4)`: next iteration.
        - `z = FindGroundZFor3DCoord(x, y, z0 + 2.0f [0x858CA0], 0, 0) + 1.3f [0x859918]` (f32).
        - `ped = new CCivilianPed(Gang1 + 7, FA4)` (ped type uses the GLOBAL Gang1 even for the Gang2 group; NULL from new -> esi = 0 and the
          following calls run on NULL); `SetPosn(&(x, y, z))`, `SetCharCreatedBy(2)`, `CWorld::Add(ped)`.
        - `task = new(0x38) CTaskComplexKillPedOnFoot(F44, -1, 0, 0, 0, 2)`; `CEventScriptCommand ev(3, task, 0)`; `intel(+0x47C)+0x68 ->
          CEventGroup::Add(&ev, 0)`.
        - `r = GetRandomNumberInRange<float>(-0.3f, 1.3f)` (0x444EA7); weapon = (r > Difficulty) ? weaponA : weaponB (0x444EC4 `test ah,0x41;
          je`: <= or NaN gives weaponB). `GiveWeapon(weapon, 5000, 0)`, `SetCurrentWeapon(weapon)`.
        - `ped+0x71A (u8 accuracy) = ftol(Difficulty * 25.0f [0x858FE8] + 70.0f [0x858CE0])`; `+0x474 |= 0x3000`; `+0x544 = +0x540 = 120.0f`
          (health / max health).
        - `blip = SetEntityBlip(2, GetPedRef(ped), colour(gang), 2, "CODEGW2" [0x859910])`; `ChangeBlipScale(blip, 2)`; `ChangeBlipColour(blip,
          colour(gang))`; anyCreated = 1; event dtor.
     g. Pickup (always when A was found): `Q = A`; `Q.x = (float)rand() * k * 4.0f [0x858B90] + A.x - 2.0f`; `Q.y = (float)rand() * k * 4.0f + A.y - 2.0f`;
        `Q.z = FindGroundZFor3DCoord(Q.x, Q.y, A.z + 1.0f, 0, 0) + 0.75f [0x858F34]`. Model by waveId (0x445034): 0 -> MI_PICKUP_HEALTH
        (0x8CD5C8) if k == 0 else none; 1 -> MI_PICKUP_BODYARMOUR (0x8CD5C0) if k == 0 else none; 2 -> k == 0 HEALTH, k == 1 BODYARMOUR;
        other -> none. If a model was chosen: `GenerateNewOne(Q, model, 5, 0, 0, 0, 0)`.
  7. If anyCreated == 0: return 0. If bTrainingMission or pDriveByCar != NULL: return 1.
  8. `mi = PickGangCar(Gang1)`; return 1 if `mi < 0` or not loaded. `pDriveByCar = GenerateOneEmergencyServicesCar(mi, FindPlayerCoors(-1))`;
     return 1 if NULL. `pDriveByCar->RegisterReference(&pDriveByCar)`; `JoinCarWithRoadSystemGotoCoors(car, FindPlayerCoors(-1), 0, 0)`;
     `+0x42D |= 8` (bPartOfAttackWave); autopilot: `+0x3BA` (m_nCarMission) = 0x3D, `+0x3D0` (m_nCruiseSpeed) = 10, `+0x41C`
     (m_TargetEntity) = FindPlayerPed(-1), `+0x3B9` (m_nCarDrivingStyle) = 2, `+0x3DD` (m_nStraightLineDistance) = 30; `+0x36 = (b & 7) | 0x18`
     (status 3 = PHYSICS); `+0x42C |= 0x40`; `SetUpDriverAndPassengersForVehicle(car, Gang1 + 14, 1, 0, 0, 1)`.
     If driver (+0x460): `driver->intel->SetPedDecisionMakerType(-1)`.
     For slot i = 0..7 (passengers +0x464 + 4i, 0x445230-0x4452FD): if non-NULL: `GiveDelayedWeapon(28, 1500)`, `SetCurrentWeapon(28)`,
     `task = new(0x44) CTaskSimpleGangDriveBy(FindPlayerPed(-1), NULL, 100.0f, 50, 8, (i & 1) == 0)`, posted as above.
     `blip = SetEntityBlip(2, GetPedRef(driver), colour(F48), 2, "CODEGW3" [0x859908])` (F48 = the gang of the LAST group, i.e. Gang2; the
     driver is not NULL-checked here, 0x445339), `ChangeBlipScale(blip, 3)`, `ChangeBlipColour(blip, colour)`, `driver+0x474 |= 0x2000`.
     Return 1.
- Edge cases: warFerocity > 5 bug (step 4); ped type always Gang1 + 7; drive-by blip uses Gang2's colour and dereferences the driver without a check.
- gta-reversed: `plugin::CallAndReturn` only (GangWars.cpp:207). Open questions: none.

## 0x446610 CGangWars::Update  (catalogue: specified/REA, port level 31)
- Convention / signature: `__cdecl void ()`, frame `sub esp,0x34` + ebx (ebx = 0 throughout). Tail at 0x446E4B `jmp 0x40642E`, an
  obfuscated `mov eax, [RadarBlip 0x96AB98]` (pointer = rol([0x1560814], [0x72CAF5] & 31)) that jumps back to 0x446E50.
- Callers: CGame::Process 0x53BEE0.
- Callees: IsPlayerOnAMission 0x464D50, EndGangWar 0x4464C0, UpdateTerritoryUnderControlPercentage 0x443DE0, CGameLogic::IsCoopGameGoingOn
  0x441390, StartOffensiveGangWar 0x446050, CreateAttackWave 0x444810, AttackWaveOvercome 0x445B30, ReleasePedsInAttackWave 0x445C30,
  ReleaseCarsInAttackWave 0x445E20, MakePlayerGainInfluenceInZone 0x445E80 (cdecl (float) -> AL), DoStuffWhenPlayerVictorious 0x446400,
  FindPlayerCoors 0x56E010, CTheZones::FillZonesWithGangColours 0x572440, CText::Get 0x6A0050 (TheText 0xC1B340), CMessages::AddMessageQ
  0x69F0B0, CMessages::AddToPreviousBriefArray 0x69DD50, CalculateTimeTillNextAttack 0x443DB0, StrengthenPlayerInfluenceInZone 0x445F50,
  MakeEnemyGainInfluenceInZone 0x445FD0 (cdecl (gang, amount)), CStats::DecrementStat 0x559FA0, VectorSub 0x40FE60, CreateDefendingGroup
  0x4453D0, FindPlayerVehicle 0x56E0D0, StartDefensiveGangWar 0x444300, CRadar::ChangeBlipDisplay 0x583D20, CRadar::ClearBlip 0x587CE0,
  CStreaming::StreamZoneModels_Gangs 0x40AA10.
  "Message(L)" below = `AddMessageQ(TheText.Get(L), 4500, 1, 1)` then `AddToPreviousBriefArray(TheText.Get(L), -1, -1, -1, -1, -1, -1, 0)`.
  Labels: GW_CLR1 0x85996C, GW_CLR2 0x859964, GW_FLEE 0x85995C, GW_WARN 0x859954, GW_WON 0x85994C, GW_SLOW 0x859940, GW_NOSH 0x859938.
  "NewAttackTime" = `TimeTillNextAttack = (float)(uint32)CalculateTimeTillNextAttack()` (`fild` + 4294967296.0f [0x858C54] if negative).
  "dt" = `ftol(ms_fTimeStep * 0.02f [0x858B38] * K)` with K = -1000.0f [0x859948] or 1000.0f [0x858C4C].
- Behaviour:
  1. If IsPlayerOnAMission() && !bIsPlayerOnAMission && NumSpecificZones == 0: EndGangWar(1). Then `bIsPlayerOnAMission = IsPlayerOnAMission() != 0`
     (second call).
  2. Return if `CCutsceneMgr::ms_cutsceneProcessing` (0xB5F852). If the LOW BYTE of m_FrameCounter == 0x38 (`cmp byte`, 0x446654):
     UpdateTerritoryUnderControlPercentage() (every 256 frames). Return if !bGangWarsActive or IsCoopGameGoingOn().
  3. `p = Provocation - ms_fTimeStep * 0.0005f [0x859978]`; `Provocation = (0.0f > p) ? 0.0f : p` (0x44668D `fcomp st1; test ah,0x41; jne` keeps p
     when 0 <= p or NaN).
  4. `switch (State)` (unsigned `ja` > 6 -> step 5; table 0x446F48):
     0: if `Provocation > 2.1f [0x859974]` StartOffensiveGangWar(). -> step 5.
     1/3/5 (PRE waves, wave w = 0/1/2): if `now > TimeStarted + 10000` (unsigned) and `CreateAttackWave(max(WarFerocity + w, 0), w)`:
        State = 2/4/6, TimeStarted = now, -> step 6 (skips the State==0 test). Otherwise -> step 5.
     2/4 (FIRST/SECOND wave): if AttackWaveOvercome(): ReleasePedsInAttackWave(0, 0); ReleaseCarsInAttackWave();
        if MakePlayerGainInfluenceInZone(0.3f) -> DoStuffWhenPlayerVictorious(), step 5; else Message(GW_CLR1 / GW_CLR2), State = 3 / 5,
        TimeStarted = now, -> step 6. Not overcome -> step 5.
     6: if AttackWaveOvercome(): MakePlayerGainInfluenceInZone(1.0f) (result ignored), DoStuffWhenPlayerVictorious(). -> step 5.
  5. If State == 0 -> step 8.
  6. Area test (0x44691C-0x4469E6), 4 separate FindPlayerCoors calls, z = pZoneToFightOver: inside = `(float)z.x1 - 50.0f <= P.x` &&
     `(float)z.x2 + 50.0f >= P.x` && `(float)z.y1 - 50.0f <= P.y` && `(float)z.y2 + 50.0f >= P.y` (50.0f [0x858B40]; NaN = outside).
     If `CGame::currArea != 0` or inside: LastTimeInArea = now. Else with `d = now - LastTimeInArea`, `dp = prevTime - LastTimeInArea` (u32):
     if `d > 30000`: State = 0, Provocation = 0, ReleasePedsInAttackWave(1, 0), ReleaseCarsInAttackWave(), FillZonesWithGangColours(0),
     AddMessageQ(GW_FLEE, 4500, 1, 1) and AddToPreviousBriefArray(GW_WARN, ...) (the brief gets GW_WARN, shared tail 0x446A72); else if
     `d > 10000 && dp <= 10000`: Message(GW_WARN).
  7. If State != 0 -> clamp (step 8b).
  8. a. (State == 0) if IsPlayerOnAMission() -> clamp; else if `FindPlayerCoors(-1).z > 950.0f` -> clamp; else skip the clamp.
     b. Clamp: `if (!(TimeTillNextAttack > 30000.0f [0x859928])) TimeTillNextAttack = 30000.0f` (0x446AE8 `test ah,0x41; je` skips when greater):
        i.e. a MAX with 30000, NaN becomes 30000.
  9. `switch (State2)`:
     0 (0x446D7A): skip if IsPlayerOnAMission() or bTrainingMission or (FindPlayerVehicle(-1, 0) && its `+0x594` m_nVehicleType is 3 or 4 (three
        FindPlayerVehicle calls)). If State == 0: `t = dt(+1000)`; if `CWeather::WeatherRegion` (i16 0xC81314) == 1: `TimeTillNextAttack -= (float)(uint32)t`
        else `TimeTillNextAttack -= (float)(uint32)t * 0.06f [0x859934]`. Then (any State) if `TimeTillNextAttack < 0.0f` StartDefensiveGangWar().
     1 (0x446C5C): `d = VectorSub(FindPlayerCoors(-1), PointOfAttack)`, `dist = sqrt(d.x*d.x + d.y*d.y)`. If `dist < 70.0f [0x858CE0]`: if
        CreateDefendingGroup(0): `FightTimer += 30000`, State2 = 2. Else (>= 70 or NaN): `FightTimer += dt(-1000)`; if the result is negative
        (`jns`): Message(GW_NOSH); State2 = 0; MakeEnemyGainInfluenceInZone(Gang1, 30); FillZonesWithGangColours(0); NewAttackTime;
        DecrementStat(0x44, 30.0f).
     2 (0x446B12): if AttackWaveOvercome(): State2 = 0; NewAttackTime; Message(GW_WON); StrengthenPlayerInfluenceInZone(10);
        FillZonesWithGangColours(0); ReleasePedsInAttackWave(1, 0). Else `FightTimer += dt(-1000)`; if negative: State2 = 0; NewAttackTime;
        Message(GW_SLOW); `n = ReleasePedsInAttackWave(1, 0)`; MakeEnemyGainInfluenceInZone(Gang1, 3*n); FillZonesWithGangColours(0);
        DecrementStat(0x44, 30.0f).
     other: nothing.
  10. (0x446E50) If RadarBlip != 0: if State2 is 0 or 2: ClearBlip(RadarBlip), RadarBlip = 0. Else `s = FightTimer > 120000 ? 10 : > 60000 ? 9 :
      > 30000 ? 8 : 7` (signed); `ChangeBlipDisplay(RadarBlip, ((now >> s) & 1) ? 0 : 2)`.
  11. If State2 != 0: `d = FindPlayerCoors(-1) - PointOfAttack` (x then y), if `sqrt(dx*dx + dy*dy) < 150.0f [0x858A28]`: if !bPlayerIsCloseby:
      StreamZoneModels_Gangs(FindPlayerCoors(-1)) (pointer to the fresh coords), bPlayerIsCloseby = 1. Else bPlayerIsCloseby = 0.
- Edge cases: the GW_FLEE path puts GW_WARN in the brief; the frame-counter test is on the low byte; the attack-time clamp is a maximum.
- gta-reversed: `plugin::Call` followed by unreachable C++ (GangWars.cpp:725-958) that differs from the bytes: `GetFrameCounter() == 56`
  (full counter), `ts / 2000.0f` (original `ts * 0.0005f`), `CreateAttackWave(..., 0)` for all waves (original passes 0/1/2),
  `std::min(TimeTillNextAttack, 30000)` (original: max), `* 0.6f` (original 0.06f [0x859934]), NewAttackTime/message order in State2 2,
  and `StreamZoneModels_Gangs(CVector{})` (original passes the player coordinates). Port from the bytes. Open questions: none.
