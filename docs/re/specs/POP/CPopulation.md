# CPopulation (POP) - full specs (group P4)

Entity position = `m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &placement.pos (+0x04)`. "delete e" = `CWorld::Remove(e)` (0x563280) then
`e->vtable[0](1)` (scalar deleting destructor). x87 compare idioms: `test ah,5; jp X` jumps when NOT (st0 < src) (NaN jumps);
`test ah,0x41; jne X` jumps when st0 <= src or NaN; `test ah,0x41; je X` jumps when st0 > src. `_rand` 0x821B1E, `_ftol` 0x821B40.

## 0x614720 CPopulation::AddToPopulation  (catalogue: specified/REA, port level 38)
- Convention / signature: `__cdecl bool (float r1 [esp+4], float r2 [esp+8], float r3 [esp+0xC], float r4 [esp+0x10])`, returns AL,
  plain `ret` (epilogue reached through the stub 0x401F53 `mov ecx, [esp+0x130]` = saved SEH link). SEH frame, `sub esp,0x120`. Callers:
  CPopulation::Update 0x616650 (r1 = PDM*cam[0xB6F11C]*42.5, r2 = PDM*cam*50.5, r3 = PDM*25 - 10, r4 = PDM*25), GeneratePedsAtStartOfGame
  0x615C90. PDM = PedCreationDistMultiplier 0x6116C0 (-> ST0).
  Locals: husband = -1, wife = -1 (EDI keeps husband), addr1 = addr2 = 0xFFFF (area words), leader = NULL, createCops (BL) = 0,
  room = 0 (the RETURN VALUE: every exit except step 1 returns `room`), pedType, model (cop type for cops), count, coords C (3 f32),
  orientation, i.
- Behaviour:
  1. Return 0 if `CGangWars::DontCreateCivilians()` (0x4439C0) or cheat byte 0x96917A is set.
  2. `ctr = FindPlayerCentreOfWorld(PlayerInFocus)` (copy). P = `CWorld::Players[PlayerInFocus]` (0xB7CD98, stride 0x190), ped = P+0,
     wanted = `*(ped->+0x480)` (NULL if +0x480 is NULL).
     If `wanted->+0x2C > 2` and `ms_nNumCop < wanted->+0x19 (u8)` and `!CGangWars::GangWarFightingGoingOn()` and `!(ped->+0x46C & 0x100)`:
     createCops = 1 if `NumLawEnforcerCars >= wanted->+0x1A (u8)` or `(float)(u16)P->+0x130 * CarDensityMultiplier [0x8A5B20] <=
     (float)NumRandomCars` (0x61484C; NaN does not set) or `NumAmbulancesOnDuty + NumFireTrucksOnDuty + NumParkedCars + NumMissionCars +
     NumRandomCars + NumLawEnforcerCars >= MaxNumberOfCarsInUse [0x8A5B24]`; when set here, `r1 = PDM() * 42.5f [0x86C850]`,
     `r2 = PDM() * 50.5f [0x86D284]` (the arg slots are overwritten).
  3. If `CGameLogic::LaRiotsActiveHere()` and `PedDensityMultiplier [0x8D2530] > 0.1f [0x858B1C]` and `ms_nNumCop < 1`: createCops = 1.
  4. `maxPeds = (float)MaxNumberOfPedsInUse [0x8D2538]`; `CDarkel::FrenzyOnGoing()` (result unused); `t = ((Other + Cops) + Gangs) + Dealers`
     (CPopCycle m_Num*_Peds 0xC0BC40/44/48, 0xC0E978, f32); `maxPeds = (maxPeds < t) ? maxPeds : t`. If `CGame::currArea != 0`:
     `maxPeds = (float)NumberOfPedsInUseInterior [0x8D253C]`. `lim = (FewerPeds() ? 0.6f [0x858CC8] : 1.0f) * PedDensityMultiplier * maxPeds`;
     `room = ((float)ms_nTotalPeds < lim)`. If !room and !createCops: return room (0). If createCops: go to 5b, else 5a.
  5. a. Normal: `z = ctr.z > 950.0f [0x858F4C]`; if `!CPopCycle::FindNewPedType(&pedType, &model, z, z)` (0x60FBD0) return room. If pedType is
        4 or 5 and `GetRandomNumberInRange<float>(0.0f, 1.0f) > 0.9f [0x858C20]` (0x41BD90): `ChooseCivilianCoupleOccupations(&husband, &wife)`;
        return room if either is -1; `pedType = ms_modelInfoPtrs[husband]->+0x28`. If `m_AllRandomPedsThisType [0x8D2534] > 0`: pedType = it.
        `count = (7 <= pedType <= 16) ? sub_610DB0() : 1` (sub_610DB0 = `ms_minGangSize + ftol((rand() & 0xFFFF) * 3.0517578e-05 *
        (ms_maxGangSize - ms_minGangSize))`, globals 0x86C6C4 / 0x86C6C8).
     b. Cops: pedType = 6, model (cop type) = 0, count = 1.
     Then `R1 = r1, R2 = r2`, and for gang types (7..16) `R1 = r1 + 30.0f [0x858CA4]`, `R2 = r2 + 30.0f`. `lowTraffic = (pedType == 6 &&
     FindPlayerWanted(-1)->+0x2C > 0)`.
  6. `if (!ThePaths.GeneratePedCreationCoors(ctr.x, ctr.y, R1, R2, r3, r4, &C, &addr1, &addr2, &orientation, lowTraffic, NULL))` (0x44E790,
     thiscall ThePaths) return room. `p = min(node(addr2).+0x1A & 0xF, node(addr1).+0x1A & 0xF)` (spawn probability); if `(rand() & 0xF) > p`
     return room. `ThePaths.sub_44DA30(addr1, addr2, rand(), &C.x, &C.y)` (0x44DA30, thiscall; adjusts C for the path width).
  7. Gangs (7..16): `PlaceGangMembers(pedType, count, &C)` (0x6117D0); return room.
  8. Couple (husband >= 0 and wife >= 0), 0x614BDC: `Q = C`. Return room if LaRiotsActiveHere(). If `TheCamera.IsSphereVisible(&Q, 1.5f)` and
     `PDM()*42.5f > sqrt(dx*dx + dy*dy)` (d = Q - player ped position, dx*dx first) return room. Return room unless
     `CPedPlacement::IsPositionClearForPed(&Q, ms_modelInfoPtrs[husband]->colModel(+0x14)->+0x24 (radius), -1, NULL, 1, 1, 1)` (0x616860).
     `g = FindGroundZFor3DCoord(Q.x, Q.y, Q.z + 1.0f, &found, NULL) + 1.0f`; if !found return room; `z = (Q.z > g) ? Q.z : g`.
     Return room if `ms_modelInfoPtrs[husband]->+0x1C == 0`; `h = AddPed(4, husband, &(Q.x, Q.y, z), 1)`; return room if NULL;
     `SetClumpAlpha(h->+0x18, 0)`. Return room if `ms_modelInfoPtrs[wife]->+0x1C == 0` (h stays). `w = AddPed(5, wife, &(Q.x, Q.y, z), 1)`;
     return room if NULL. `sw = w->GetWalkAnimSpeed()`, `sh = h->GetWalkAnimSpeed()` (0x5E04B0, f32); return room if `sw < 0.75f [0x858F34]`,
     if `sh < 0.75f`, or if `|sw - sh| > 0.45f [0x8631D0]` (both peds stay). Tasks (CTaskComplexBeInCouple 0x6836F0, size 0x20, ctor
     `(partner, isLeader, 1, 1, 10.0f)`, set with `intel->taskManager.SetTask(task, 3, 0)` 0x681AF0): if `sw > sh`: h gets (w, 0), then w gets
     (h, 1); else w gets (h, 0), then h gets (w, 1). Wife position: `W = h.pos + (ms_offsets entry at 0xC19708 .x, .y, 0)` (0x40FE30);
     `g = FindGroundZFor3DCoord(W.x, W.y, W.z + 1.0f, &found, NULL) + 1.0f`; if !found: `RemovePed(h)`, `RemovePed(w)` (0x610F20), return room.
     `w->SetPosn(W.x, W.y, (W.z > g) ? W.z : g)` (0x420B80). `IsPositionClearForPed(&W, ms_modelInfoPtrs[wife] radius, 3, ents[3] = {0},
     1, 1, 1)` (result ignored); if any `ents[k]` is non-NULL and not h or w: remove both, return room. `SetClumpAlpha(w->+0x18, 0)`; return room.
  9. Single / group peds (0x61504E): i = 0; while i < count (loop re-entered at 0x615060 for i >= 1):
     a. Model checks (each iteration): cops: cop type 4 needs model 286 loaded (byte 0x8E6328 == 1) and weapon 29's model loaded
        (`ms_aInfoForModel[CWeaponInfo::GetWeaponInfo(29, 1)->+0xC].loadState == 1`, 0x743C60); type 2 needs model 285 (0x8E6314) and weapon 28;
        type 5 needs model 287 (0x8E633C), weapon 29 and weapon 16; types 0, 1, 3 no check. Non-cops: `ms_modelInfoPtrs[model]->+0x1C != 0`.
        Failure returns room.
     b. `C.z += 0.7f [0x858CB0]` (every iteration). `rand()` (value unused, consumed every iteration). If leader != NULL: `k = (float)i`;
        `hi = (1.0f + k) * 0.75f`, `lo = k * 0.75f` (f32); `a = Rand<float>(lo, hi)`, `b = Rand<float>(lo, hi)` (0x41BD90, in this order);
        if `rand() & 1` a = -a; if `rand() & 1` b = -b; `C.x = a + leader.x`, `C.y = b + leader.y`.
     c. Return room unless `IsPositionClearForPed(&C, -1.0f, -1, NULL, 1, 1, 1)`.
     d. If `i + 1 < count`: `g = FindGroundZFor3DCoord(C.x, C.y, C.z + 2.0f, &found, NULL) + 0.7f`; if !found return room; `C.z = (C.z > g) ? C.z : g`.
     e. ok = 1; if `TheCamera.IsSphereVisible(&C, 2.0f)` and `PDM()*42.5f > sqrt(dx*dx + dy*dy)` (d = C - ctr, dx*dx first): ok = 0; else if
        `ms_modelInfoPtrs[model]->+0x2C == 40` (skater): `ok = IsSkateable(&C)` (0x6114C0). If `IsSunbather(model)` (0x611760) and the game
        hour (0xB70153) is < 8 or > 19: ok = 0.
     f. Dealers (17): return room if !ok or `FindDistanceToNearestPedOfType(17, C) < 20.0f [0x858BA4]`. Return room if !ok. Gang types
        7..16 except 8: return room if `2400.0f < C.x < 2540.0f` and `-1730.0f < C.y < -1625.0f` (0x86D280/7C/78/74; Grove Street box).
     g. `ped = AddPed(pedType, model, &C, 1)` (not NULL-checked). towel = 0.
     h. Sunbathers (`IsSunbather(model)` and `rand() & 3`): `ProcessVerticalLine(&(C.x, C.y, C.z + 2.0f), C.z - 2.0f, &cp, &ent, 1, 0, 0, 0, 0, 0, 0)`
        (0x5674E0); requires ent != NULL, surface `cp.+0x23` beach (`IsBeach` 0x55E750) or 0x22 or 0xA0 or cheat byte 0x969159, and
        (`CTaskComplexSunbathe::CanSunbathe()` 0x632140 or that cheat). Then ok2 = 1; if `IsPositionClearForPed(&C, 3.0f, 2, ents[2], 1, 1, 1)`
        returns true, ok2 = 0 when ents[0] or ents[1] is non-NULL and not ped. towel = (surface != 0xA0). If !ok2: go to i.
        `ProcessVerticalLine(&(p.x, p.y, p.z + 10.0f), -10.0f, &cp2, &ent2, 1, 0, 0, 0, 1, 0, 0)` with p = ped position; if no hit or
        `(n.x + n.y) * 0.0f + n.z <= 0.95f [0x858EF0]` (cp2 normal): go to j (no task). `hd = Rand<float>(0.0f, 1.0f) * 6.2831855f [0x858CBC]`;
        `ped+0x55C = ped+0x558 = hd`; `ped->SetHeading(hd)`. If towel: `T = CWaterLevel::CreateBeachToy(&(p.x, p.y, cp2.z + 0.04f [0x858CEC]), 11)`
        (0x6EABA0); if T: SetHeading(hd); `up = cp2 normal`; `right = Cross(up, forward)`; `forward = Cross(up, right)` (0x59C730);
        UpdateRwMatrix; UpdateRwFrame; if `(rand() & 3) == 0`: `B = p + T.right * Rand<float>(-0.5f, 0.5f) + T.forward * Rand<float>(-1.0f, 1.0f)`
        (B.z = cp2.z + 0.04f), `CreateBeachToy(&B, 6)`. Task: `new(0x38) CTaskComplexSunbathe(T, (rand() & 3) == 0)` (0x631F80),
        `SetTask(task, 3, 0)`. Go to j.
     i. Others: if LaRiotsActiveHere(): if `rand() & 3`: `SetTask(new(0x38) CTaskComplexWanderCriminal(6, GetRandomNumberInRange(0, 8), 1), 4, 0)`
        (0x48E610, 0x407180); else if pedType != 6: `o = new(0x17C) CObject(MI_TELLY [0x8CD5A8], 1)` (0x5A1D70), `CWorld::Add(o)`,
        `SetTask(CTaskComplexWander::GetWanderTaskByPedType(ped), 4, 0)`, `SetTaskSecondary(new(0x3C) CTaskSimpleHoldEntity(o, &(0, 0.45f,
        0.35f), 1, 1, 0x13E, 0x51, 0), 4)` (0x6913A0, 0x681B60). Without riots: `CTheScripts::ScriptsForBrains.CheckIfNewEntityNeedsScript(ped, 0, 0)`
        (0x46FF20).
     j. If i == 0: leader = ped. `SetClumpAlpha(ped->+0x18, 0)`. Continue with i + 1 only while `i + 1 < count` and count > 1.
  10. Return room.
- Edge cases: the return value is the "pool has room" flag, not "a ped was created"; createCops can force creation when the population is
  full. In group spawning the last ped is not snapped to the ground (step 9d). AddPed's result is used unchecked in step 9.
- gta-reversed: calls the original address (Population.cpp:1580-1584). Open questions: none (callee names sub_44DA30 / sub_610DB0 are
  descriptive; GeneratePedCreationCoors 0x44E790 matches gr's declared signature).

## 0x615DC0 CPopulation::ManageObject  (catalogue: specified/REA, port level 14)
- Convention / signature: `__cdecl void (CObject* obj [esp+4], const CVector* centre [esp+8])`, plain `ret`.
- Callers: ManageAllPopulation 0x6160A0, ManagePopulation 0x616190. Callees: CObject::CanBeDeleted 0x59F120, CEntity::GetIsOnScreen
  0x534540, CWorld::Remove 0x563280, CVisibilityPlugins::GetClumpAlpha 0x732B20, CEntity::IsVisible 0x536BC0, FindDummyDistForModel 0x610ED0
  (cdecl (int model) -> ST0), ConvertToDummyObject 0x614670.
- Globals: ModelIndices MI_ROADWORKBARRIER1 0x8CD57C, MI_ROADBLOCKFUCKEDCAR1 0x8CD580, MI_ROADBLOCKFUCKEDCAR2 0x8CD584, MI_BEACHBALL 0x8CD638,
  MI_BEACHTOWEL01 0x8CD68C, MI_BEACHTOWEL04 0x8CD698, MI_SAMSITE 0x8CD6AC, MI_SAMSITE2 0x8CD6B0 (u16 each); m_snTimeInMilliseconds 0xB7CB84.
- Fields: +0x18 m_pRwObject (first byte = RW object type, 2 = clump), +0x22 model (i16, sign-extended), +0x13C m_nObjectType (u8),
  +0x140 object flags (bit 22 = 0x400000, gr bFadingIn), +0x150 m_nRemovalTime (u32), +0x170 m_pDummyObject (CEntity*).
- Behaviour:
  1. Return if `!obj->CanBeDeleted()`.
  2. `d = sqrt((dz*dz + dy*dy) + dx*dx)`, `d = objPos - *centre` (x87, kept in ST0, not rounded through memory, 0x615DE8-0x615E10).
  3. If `+0x13C == 3` (temporary object):
     a. model == ROADWORKBARRIER1, ROADBLOCKFUCKEDCAR1, ROADBLOCKFUCKEDCAR2 or BEACHBALL (equality on `movsx` model vs `movzx` index):
        delete if `d > 120.0f [0x858BB0]` (0x615F40 then the shared `jne` at 0x615F26), else return.
     b. MI_BEACHTOWEL01 <= model <= MI_BEACHTOWEL04 (UNSIGNED compares of the sign-extended model): if `d > 64.5f [0x86D288]` delete; else if
        `d <= 35.0f [0x85F070]` (or NaN) return; else delete unless `GetIsOnScreen()`.
     c. Other temporary objects: if `d > 54.5f [0x858FF4]` delete. Else if `d > 25.0f [0x858FE8]` and `!GetIsOnScreen()` delete. Otherwise:
        if `now > +0x150` (unsigned) delete; else if `+0x18 != NULL` and its type byte == 2 and `+0x140 & 0x400000`: delete when
        `GetClumpAlpha(clump) == 0` or `!IsVisible()`; else return.
  4. Else (0x615F4D): `dd = +0x170 ? |dummyPos - *centre|` (`sqrt((dz*dz + dy*dy) + dx*dx)`, stored f32) `: 100000.0f` (imm 0x47C35000).
     `thr = (model == MI_SAMSITE || model == MI_SAMSITE2) ? 750.0f [0x859A98] : 80.0f [0x858A14]`. Return if `d <= thr` (or NaN,
     0x615FCC). If `FindDummyDistForModel(model) < dd` (0x615FE0 `test ah,5; jp`): `ConvertToDummyObject(obj)`.
- gta-reversed: calls the original address directly (Population.cpp:1713-1715). Open questions: none.
