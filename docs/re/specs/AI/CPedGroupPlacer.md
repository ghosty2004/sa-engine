# CPedGroupPlacer (AI) - RE-SPEC-AI-2 (lead)

## 0x5FD330 CPedGroupPlacer::PlaceRandomGroup  (catalogue: specified/REA, port level 35) - RE-SPEC-AI-2 (lead)
- Convention: `__thiscall bool (this ignored, ePedType pedType [esp+4], int32 numPeds [esp+8], const CVector* origin [esp+0xC],
  u32 allocType [esp+0x10] unused)`, `ret 0x10`, SEH frame (state slot set to -1/1 around the two task allocations). Locals below are
  relative to B = esp after `push esi` at 0x5FD382 (entry - 0x9C): B+0x12 blocked u8, B+0x13 groundFound u8, B+0x14 i, B+0x18 angleStep,
  B+0x1C radius, B+0x20 numPlaced (copy of EBX), B+0x24/B+0x28 scratch, B+0x2C groupId, B+0x34 pos CVector, B+0x40 firstPos CVector,
  B+0x4C placed CPed*[..], B+0x6C CEntity* objects[9].
- Callers: PlaceGroup 0x5FD810 (tail jump, allocType = 5). Callees: CPedGroups::AddGroup 0x5FB800, CCamera::IsSphereVisible 0x420D40
  (ECX = TheCamera 0xB6F028), CWorld::FindPlayerPed 0x56E210, CPopulation::PedCreationDistMultiplier 0x6116C0 (ST0),
  CPedPlacement::IsPositionClearForPed 0x616860 (cdecl 7 args), `rand` 0x821B1E, CWorld::FindGroundZFor3DCoord 0x5696C0,
  CPopulation::ChooseGangOccupation 0x611550, CWorld::GetIsLineOfSightClear 0x56A490, CPopulation::AddPed 0x612710,
  CGeneral::GetRadianAngleBetweenPoints 0x53CBE0, CVisibilityPlugins::SetClumpAlpha 0x732B00, CPopulation::RemovePed 0x610F20,
  CPedGroupIntelligence::SetDefaultTaskAllocator 0x5FB280, CPedGroupMembership::SetLeader 0x5FB9C0 / AddFollower 0x5F8020 /
  Process 0x5FBA60, CPedGroupIntelligence::Process 0x5FC4A0, CTask::operator new 0x61A5A0, CTaskComplexWanderGang ctor 0x66F5C0,
  `_ftol` 0x821B40 (truncation), CTaskManager::SetTask 0x681AF0, CTaskComplexBeInGroup ctor 0x632E50.
- Globals: CModelInfo::ms_modelInfoPtrs 0xA9B0C8, CPedGroups::ms_groups 0xC09920 (stride 0x2D4; +8 membership, +0x30 intelligence),
  CPedGroupDefaultTaskAllocators::ms_RandomAlloc 0xC09918.
- Behaviour:
  1. If numPeds < 2 (signed, 0x5FD353) return 0. groupId = AddGroup(); if groupId < 0 return 0 (0x5FD35F-0x5FD36A).
  2. angleStep = 6.2831855f [0x858CBC] / (float)numPeds (fild; fdivr; fst f32 to B+0x18, 0x5FD36C-0x5FD388). radius =
     sqrt(0.5f [0x858B8C] / (1.0f [0x858624] - cos(angleStep))) computed from the unrounded x87 step (fcos on ST0 after the fst),
     stored f32 (0x5FD38C-0x5FD39C) - i.e. neighbours on the circle are 1 m apart.
  3. If TheCamera.IsSphereVisible(origin, radius) (0x5FD3A6): p = player ped (matrix ? &matrix->pos : &placement.pos, 0x5FD3B6-0x5FD3C5);
     d = sqrt((o.x-p.x)^2 + (o.y-p.y)^2) (x87: dy = o.y-p.y, dx = o.x-p.x, dx*dx + dy*dy, fsqrt, stored f32, 0x5FD3C8-0x5FD3E3);
     if PedCreationDistMultiplier() * 42.5f [0x86C850] > d (`fcomp; test ah,0x41; je`, 0x5FD3EC-0x5FD3FB) return 0.
  4. If !IsPositionClearForPed(origin, radius, -1, nullptr, 1, 1, 1) return 0 (0x5FD401-0x5FD417).
     NOTE: the early returns of steps 3/4 (and of step 7) do not remove the group created by AddGroup (original behaviour).
  5. numPlaced (EBX) = 0. For i = 0 .. numPeds-1 (0x5FD431-0x5FD693):
     a. aj = ((float)rand() * 3.0518509e-05f [0x858C7C] * 0.4f [0x858EE8] - 0.2f [0x858CC4]) * angleStep -> f32 B+0x24 (0x5FD431-0x5FD454).
     b. rj = ((float)rand() * 3.0518509e-05f [0x858C7C] * 0.4f [0x858EE8] - 0.2f [0x858CC4]) * radius -> f32 B+0x28 (0x5FD458-0x5FD485).
     c. a = (float)i * angleStep + aj; R = rj + radius (both x87, unrounded); offX = cos(a) * R -> f32; offY = sin(a) * R -> f32
        (0x5FD489-0x5FD4B1).
     d. gz = FindGroundZFor3DCoord(offX + o.x, offY + o.y, o.z + 1.0f [0x858624], &groundFound, nullptr) (args stored f32,
        0x5FD4B3-0x5FD4D5); z = gz + 1.0f [0x858624] (x87). If !groundFound -> next i (0x5FD4E9).
     e. If z < o.z (`fcom; test ah,5; jp` falls through only for "less") z = o.z (0x5FD4EF-0x5FD4FB): z = max(gz+1, o.z), NaN keeps gz+1.
     f. pos = (offX + o.x, offY + o.y, z) (f32 stores, 0x5FD500-0x5FD516). If i == 0: firstPos = pos (0x5FD51C-0x5FD530).
     g. model = ChooseGangOccupation(pedType - 7) (0x5FD534-0x5FD53F). If ms_modelInfoPtrs[model]->+0x1C (RwObject) == 0 -> next i.
     h. objects[9] = 0; IsPositionClearForPed(&pos, mi->m_pColModel(+0x14)->boundSphere.radius(+0x24), 9, objects, 1, 1, 1) - the
        RETURN VALUE IS IGNORED (0x5FD587-0x5FD58C). blocked = 1 if any non-null objects[k] (k = 0..8) is not one of placed[0..numPlaced-1]
        (0x5FD58F-0x5FD5C8).
     i. If i == 0: losOk = zOk = 1. Else losOk = GetIsLineOfSightClear(&pos, &firstPos, 1, 0, 0, 0, 0, 0, 0) (0x5FD5CA-0x5FD5E2) and
        zOk = |pos.z - firstPos.z| < 1.0f [0x858624] (fabs; fcomp; `test ah,5; jp` -> NaN/>= gives 0, 0x5FD5E7-0x5FD607).
     j. If blocked or !losOk or !zOk -> next i. ped = AddPed(pedType, model, &pos, false) (0x5FD619-0x5FD629).
        If ped == 0: `RemovePed(nullptr)` (0x5FD673; passes NULL to CWorld::Remove - original bug, keep the call shape).
        Else placed[numPlaced++] = ped; h = GetRadianAngleBetweenPoints(o.x, o.y, pos.x, pos.y) (0x5FD637-0x5FD652); ped->+0x558
        m_fCurrentRotation = h; ped->+0x55C m_fAimingRotation = h (fst/fstp f32); SetClumpAlpha(ped->m_pRwClump (+0x18), 0).
  6. If numPlaced < 1 (0x5FD699): RemovePed for each placed (none) and return 0.
  7. g = &ms_groups[groupId]; g->intelligence.SetDefaultTaskAllocator(&ms_RandomAlloc 0xC09918) (0x5FD6BD-0x5FD6D7).
     For j = 0 .. numPlaced-1: j == 0 ? membership.SetLeader(placed[0]) : membership.AddFollower(placed[j]); membership.Process();
     intelligence.Process() (0x5FD6F0-0x5FD717);
     t = new(0x38): if t: dir = _ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * 8.0f [0x859000]);
     CTaskComplexWanderGang(t, 4 /*PEDMOVE_WALK*/, (u8)dir, 5000, 1, 0.5f [0x86FCB0]) (0x5FD739-0x5FD76F);
     placed[j]->m_pIntelligence(+0x47C)->m_TaskMgr(+4).SetTask(t, 4 /*PRIMARY_DEFAULT*/, 0) (0x5FD774-0x5FD791);
     t2 = new(0x28): if t2: CTaskComplexBeInGroup(t2, groupId, 0) (0x5FD7B3-0x5FD7BC); SetTask(t2, 3 /*PRIMARY_PRIMARY*/, 0).
  8. Return 1 (0x5FD7F0).
- Edge cases / original bugs: firstPos is uninitialised when the ground probe for i = 0 fails (step 5d skips before 5f), so later
  LOS / dz checks compare with stack garbage; a single placed ped still forms a "group" of one (numPlaced >= 1); `rand()` is not masked
  in 5a/5b (MSVC rand is 0..0x7FFF anyway).
- gta-reversed: plugin::CallMethodAndReturn (PedGroupPlacer.cpp:24), not reversed; declares numOfPeds uint32 but the bytes compare signed.
- Open questions: none.

## 0x5FC9B0 CPedGroupPlacer::PlaceFormationGroup  (catalogue: specified/REA, port level 35)
- Convention: `__thiscall bool (this ignored, ePedType pedType [esp+4], int32 numPeds [esp+8], const CVector* origin [esp+0xC],
  u32 allocType [esp+0x10] unused)`, `ret 0x10`, SEH frame. Caller: PlaceGroup 0x5FD810 (tail jump). Result in AL.
- Callees (same as PlaceRandomGroup 0x5FD330 above, plus): CVector add 0x40FE30 (cdecl `(out, a, b)`: out = a + b, computed z, y, x,
  stored x, y, z), CPlaceable::SetPosn(const CVector&) 0x4241C0, CPlaceable::SetPosn(x, y, z) 0x420B80.
- Globals: CTaskComplexFollowLeaderInFormation::ms_offsets 0xC196E8 (CVector2D[], stride 8), CPedGroups::ms_groups 0xC09920 (stride
  0x2D4; +8 membership, +0x30 intelligence), CPedGroupDefaultTaskAllocators::ms_FollowAnyMeansAllocator 0xC09908, TheCamera 0xB6F028.
- 1. `gid = CPedGroups::AddGroup()` 0x5FB800 (first instruction after the SEH setup; no numPeds check). gid < 0 -> return 0.
- 2. If `TheCamera.IsSphereVisible(origin, 3.0f)` 0x420D40: p = player ped position (FindPlayerPed(-1); matrix pos or +4);
  `d = sqrt((o.x-p.x)^2 + (o.y-p.y)^2)` (dy computed first, then dx; dx*dx + dy*dy; stored f32); if `PedCreationDistMultiplier() *
  [0x86C850]=42.5f > d` (strict; `test ah,0x41; je`) return 0.
- 3. If `!CPedPlacement::IsPositionClearForPed(origin, 3.0f, -1, nullptr, true, true, true)` 0x616860 return 0.
- 4. `gz = FindGroundZFor3DCoord(o.x, o.y, o.z + 1.0f [0x858624], &found, nullptr)`; `z = gz + 1.0f`; if !found return 0; if `z < o.z`
  (`fcom; test ah,5; jp` keeps z for >= and NaN) `z = o.z`. (Returns in steps 2-5 do not remove the group created in step 1.)
- 5. `m = CPopulation::ChooseGangOccupation(pedType - 7)` 0x611550; if `ms_modelInfoPtrs[m]->+0x1C == 0` return 0. `L =
  CPopulation::AddPed(pedType, m, &(o.x, o.y, z), false)` 0x612710; L == 0 -> return 0. `placed[0] = L`, `n = 1`.
- 6. For `i = 1 .. numPeds-1` (signed compare, so numPeds <= 1 skips the loop):
  - `m = ChooseGangOccupation(pedType - 7)`; if its RwObject (+0x1C) is 0 -> next i. `P = AddPed(pedType, m, &(o.x, o.y, z), false)`;
    0 -> next i.
  - `off = (ms_offsets[i].x, ms_offsets[i].y, 0)`; `P->SetPosn(Lpos + off)` (Lpos = leader matrix pos or +4; vector add 0x40FE30).
  - `q = P` position (re-read). `gz = FindGroundZFor3DCoord(q.x, q.y, q.z + 1.0f, &found, nullptr) + 1.0f`; `zz = (q.z > gz) ? q.z : gz`
    (`fcomp; test ah,0x41; jne`: equal and NaN keep gz). If !found: `CPopulation::RemovePed(P)` 0x610F20, next i.
  - If `fabs(zz - Lpos.z) > 1.0f` (strict; NaN passes): RemovePed(P), next i.
  - If `!CWorld::GetIsLineOfSightClear(&(q.x, q.y, zz), &Lpos, true, false, false, false, false, false, false)` 0x56A490: RemovePed(P),
    next i.
  - `P->SetPosn(q.x, q.y, zz)` 0x420B80; `placed[n++] = P`; `CVisibilityPlugins::SetClumpAlpha(P->+0x18, 0)` 0x732B00. (The leader is
    never made transparent.)
- 7. (`n >= 1` always holds here; the "remove all placed and return 0" block at 0x5FCD99 is dead.) `g = ms_groups + 0x2D4*gid`;
  `(g+0x30)->SetDefaultTaskAllocator(&ms_FollowAnyMeansAllocator)` 0x5FB280; `(g+8)->SetLeader(L)` 0x5FB9C0; `(g+8)->Process()` 0x5FBA60;
  `(g+0x30)->Process()` 0x5FC4A0; `t = new(0x28) ? CTaskComplexBeInGroup(t, gid, true) 0x632E50 : 0`;
  `L->m_pIntelligence(+0x47C)->m_TaskMgr(+4).SetTask(t, 3, 0)` 0x681AF0.
  For `j = 1 .. n-1`: `(g+8)->AddFollower(placed[j])` 0x5F8020; membership Process; intelligence Process; `t = new(0x28) ?
  CTaskComplexBeInGroup(t, gid, false) : 0`; `placed[j]->...SetTask(t, 3, 0)`.
- 8. Return 1.
- Differences to PlaceRandomGroup (same file): no `numPeds < 2` check, fixed 3.0f radius, followers are spawned at the leader's spawn
  point and then moved to the formation offsets, the leader gets BeInGroup(gid, true) and the allocator is FollowAnyMeans.
- gta-reversed: plugin::CallMethodAndReturn (PedGroupPlacer.cpp:14), not reversed. Open questions: none.

## 0x5FCE80 CPedGroupPlacer::PlaceChatGroup  (catalogue: specified/REA, port level 35)
- Convention: identical to PlaceRandomGroup 0x5FD330 (section above): `__thiscall bool (this ignored, ePedType pedType [esp+4],
  int32 numPeds [esp+8], const CVector* origin [esp+0xC], u32 allocType [esp+0x10] unused)`, `ret 0x10`, SEH frame. Caller: PlaceGroup
  0x5FD810.
- Body: steps 1-6 are instruction-for-instruction the same as PlaceRandomGroup steps 1-6 (verified by a normalised diff of the two
  listings: only the register allocation of `origin` / the placed counter (EBX/EBP swapped) and stack offsets differ; the same constants
  [0x858CBC]=6.2831855f, [0x858624]=1.0f, [0x858B8C]=0.5f, [0x86C850]=42.5f, [0x858C7C]=3.0518509e-05f, [0x858EE8]=0.4f, [0x858CC4]=0.2f,
  and the same callees in the same order). So: `numPeds < 2` -> 0; AddGroup; circle radius `sqrt(0.5 / (1 - cos(2PI/numPeds)))`; camera /
  distance / clear checks; per-ped jittered circle placement with ground probe, IsPositionClearForPed object scan, LOS and |dz| < 1
  checks against the first position; AddPed, heading towards the origin (`GetRadianAngleBetweenPoints(o.x, o.y, pos.x, pos.y)` stored to
  +0x558 and +0x55C), SetClumpAlpha(0); `numPlaced < 1` -> RemovePed each and return 0 (same leak of the group on early returns, same
  uninitialised firstPos and RemovePed(nullptr) quirks).
- Step 7 differs (0x5FD209-0x5FD30F): `g = ms_groups + 0x2D4*gid`; `(g+0x30)->SetDefaultTaskAllocator(&CPedGroupDefaultTaskAllocators::
  ms_StandStillAllocator 0xC09910)`; `(g+8)->SetLeader(placed[0])`; `(g+8)->Process()`; `(g+0x30)->Process()`; `t = new(0x28) ?
  CTaskComplexBeInGroup(t, gid, true) 0x632E50 : 0`; `placed[0]->m_pIntelligence->m_TaskMgr.SetTask(t, 3, 0)` 0x681AF0. For `j = 1 ..
  numPlaced-1`: `(g+8)->AddFollower(placed[j])`; membership Process; intelligence Process; `t = new(0x28) ? CTaskComplexBeInGroup(t, gid,
  false) : 0`; `placed[j]->...SetTask(t, 3, 0)`. No CTaskComplexWanderGang default task is created (PlaceRandomGroup creates one per
  ped). Return 1.
- gta-reversed: plugin::CallMethodAndReturn (PedGroupPlacer.cpp:19), not reversed. Open questions: none.
