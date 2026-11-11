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
