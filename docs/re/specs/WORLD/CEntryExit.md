# CEntryExit (WORLD) - full specs

Layout (gr `EntryExit.h`, size 0x3C, matches every offset used below): +0x00 `m_szName[8]`,
+0x08 `m_recEntrance` (CRect), +0x18 `m_fEntranceZ`, +0x1C `m_fEntranceAngleRad`, +0x20 `m_vecExitPos`
(CVector), +0x2C `m_fExitAngle` (degrees, f32), +0x30 `m_nFlags` (u16: 0x1 bUnknownInterior, 0x2
bUnknownPairing, 0x8 bRewardInterior, 0x10 bUsedRewardEntrance, 0x100 bAcceptNpcGroup, 0x200 bFoodDateFlag,
0x2000 bEnteredWithoutExit, 0x8000 bDeleteEnex), +0x32 `m_nArea` (u8), +0x33 `m_nSkyColor` (u8),
+0x38 `m_pLink` (CEntryExit*).

`IsTeleportPointValid` (0x43EA90) is a free function used by `CEntryExit::WarpGangWithPlayer`; its spec is
in `CEntryExitManager.md` (assignment note).

## 0x4404A0 CEntryExit::TransitionFinished  (catalogue: specified/REA, port level 35)
- Convention: `__thiscall bool (CEntryExit* this = ECX, CPed* ped = [esp+4])`, returns AL (0/1; upper EAX
  bytes are garbage), `ret 4`. SEH frame (handler 0x837806), locals 0x38 bytes. Registers: ebp = this,
  esi = ped, edi = 0x100 (the ped `bInVehicle` mask) on every path that reaches the common tail.
- Callers: CEntryExitManager::Update 0x440D10 (1).
- Callees (all via `orig!`): CPed::GetEntityThatThisPedIsHolding 0x5E02E0; CColStore::AddCollisionNeededAtPosn
  0x4103A0 (cdecl(const CVector*)); CIplStore::AddIplsNeededAtPosn 0x4045B0 (cdecl(const CVector*));
  CStreaming::AddModelsToRequestList 0x40D3F0 (cdecl(const CVector*, int32 flags)); CEntryExit::RequestObjectsInFrustum
  0x43E690 (thiscall); CPathFind::SetPathsNeededAtPosition 0x44DCD0 (thiscall on ThePaths 0x96F050, (const CVector*));
  CCamera::GetFading 0x50ADE0, SetFadeColour 0x50BF00 (three args pushed as dwords), Fade 0x50AC20 (float, direction pushed as dword 0/1),
  RestoreWithJumpCut 0x50BAB0 (thiscall on TheCamera 0xB6F028); CEntryExitManager::SetAreaCodeForVisibleObjects
  0x43ECF0, ResetAreaCodeForVisibleObjects 0x43ED80, AddEntryExitToStack 0x43E410 (cdecl(CEntryExit*)),
  DeleteOne 0x43FD50 (cdecl(int32)); CPedGroups::GetPedsGroup 0x5F7E80 (cdecl(CPed*)); CEvent::operator new
  0x4B5620 (cdecl(size)); CEventLeaderEntryExit ctor 0x43E1C0 (thiscall(CPed*)); CEventAreaCodes ctor 0x4B2190;
  CEventGroupEvent ctor 0x4ADFD0 (thiscall(CPed*, CEvent*)) / dtor 0x4AE070; CPedGroupIntelligence::AddEvent 0x5F7470
  (thiscall(CEvent*)); CEntryExit::GetEntryExitToDisplayNameOf 0x43E650; CText::Get 0x6A0050 (thiscall on TheText
  0xC1B340); CHud::SetZoneName 0x588BB0 (cdecl(const char*, bool)); CPad::GetPad 0x53FB70; CClothes::RebuildPlayerIfNeeded
  0x5A8390 (cdecl(CPlayerPed*)); CAudioZones::Update 0x5083C0 (cdecl(bool, CVector by value)); CWaterLevel::FindNearestWaterAndItsFlow
  0x6E9D70; CGarages::CloseHideOutGaragesBeforeSave 0x44A170; InteriorManager_c::SetEntryExitPtr 0x598180 (thiscall on
  g_interiorMan 0xBAF670, (CEntryExit*)) and ::Update 0x598F50 (returns bool); CPopulation::RemoveAllRandomPeds 0x6122C0;
  CEntryExit::RequestAmbientPeds 0x43E6D0; CStreaming::LoadAllRequestedModels 0x40EA10; CTimer::Suspend 0x5619D0 /
  Resume 0x561A00; CTimeCycle::FindFarClipForCoors 0x5616E0 (cdecl(CVector by value) -> ST0 float); RwCameraSetFarClipPlane
  0x7EE2A0; CStreaming::LoadScene 0x40EB70 (cdecl(const CVector*)); CEntryExit::GenerateAmbientPeds 0x43E8B0 (thiscall(const CVector*));
  CStreaming::SetLoadVehiclesInLoadScene 0x407A30 (cdecl(bool)); CStreaming::ClearFlagForAll 0x407A40 (cdecl(int32));
  CTimeCycle::StartExtraColour 0x55FEC0 (cdecl(int32, bool)) / StopExtraColour 0x55FF20 (cdecl(bool)); CRubbish::SetVisibility
  0x7204C0 (cdecl(bool)); CEntryExit::FindValidTeleportPoint 0x43EAF0 (thiscall(CVector* inout)); CMatrix::SetRotateZOnly
  0x59B020 (thiscall(float)); CTheScripts::ClearSpaceForMissionEntity 0x486B00 (cdecl(const CVector*, CEntity*));
  CShopping::RemoveLoadedShop 0x49AE30, LoadShop 0x49BBE0 (cdecl(const char*)); CPopulation::ManageAllPopulation 0x6160A0;
  CTheScripts::Process 0x46A000; CEntryExit::WarpGangWithPlayer 0x43F1F0 (thiscall(CPed*)); CEntryExit::ProcessStealableObjects
  0x43E990 (thiscall(CPed*)); CTaskManager::GetTaskSecondary 0x681810 (thiscall(int32)); CTaskComplexFacial::StopAll 0x691250;
  CGame::TidyUpMemory 0x53C500 (cdecl(bool, bool)). Virtual calls: task vtable +0x10 (GetTaskType), vehicle +0xD4
  (GetHeightAboveRoad, returns ST0), entity +0x38 (Teleport(CVector by value, bool)).
- Globals: R `CEntryExit::ms_spawnPoint` 0x96A7C0 (CEntryExit*, re-read at every use - keep re-reading, callees
  may change it), R `CEntryExit::ms_bWarping` 0x96A7B8 (u8), R/W `CEntryExitManager::ms_exitEnterState` 0x96A7CC
  (int32), W `CEntryExitManager::ms_numVisibleEntities` 0x96A7DC, R `CEntryExitManager::mp_poolEntryExits` 0x96A7D8,
  R/W `CGame::currArea` 0xB72914 (written as int32, read as int32 or low byte), W `CRenderer::m_loadingPriority`
  0xB76850 (u8), TheCamera 0xB6F028 (W pos +4/+8/+0xC when no matrix; R `m_matrix` 0xB6F03C = TheCamera+0x14;
  R `m_pRwCamera` 0xB6F97C), constants 0x858624 = 1.0f, 0x8595EC = 0.017453292f (f32 deg->rad).
- Ped fields: +0x14 m_matrix, +0x10 placement heading, +0x4 placement pos, +0x2F m_nAreaCode (u8), +0x46C flag dword
  (0x100 = bInVehicle), +0x474 flag dword (bit 0x04000000 set here; gr's commented code calls it bCanExitCar - name
  not verified, value is), +0x47C m_pIntelligence (CPedIntelligence: +4 task manager, +0x10 = primary task slot 3),
  +0x558 m_fCurrentRotation, +0x55C m_fAimingRotation, +0x58C m_pVehicle, +0x78C m_pEnex (CEntryExit*).
  Vehicle: +0x14 m_matrix, +0x10 placement heading, +0x2F m_nAreaCode. Held entity: +0x2F m_nAreaCode.
  CPedGroup +0x30 = m_groupIntelligence. CPad +0x10E bit 0x08 (gr: bDisablePlayerEnterCar).
- Behaviour (original order):
  1. `pos = ms_spawnPoint->m_vecExitPos` copied to a local (x as raw dword; y, z via fld/fstp f32 - bit-exact
     except that a signalling NaN would be quieted; ignore).
  2. `if (e = ped->GetEntityThatThisPedIsHolding()) e->m_nAreaCode = ms_spawnPoint->m_nArea`.
  3. Path selection: if `(this->m_nFlags & 0x202) != 0` OR `(ped[+0x46C] & 0x100) != 0` -> PATH B (step 5),
     else PATH A (step 4).
  4. PATH A (on foot, normal enex):
     a. `AddCollisionNeededAtPosn(&pos)`; `AddIplsNeededAtPosn(&pos)`.
     b. if `ms_bWarping`: `CRenderer::m_loadingPriority = 0`; `AddModelsToRequestList(&pos, 0x20)`;
        `ms_spawnPoint->RequestObjectsInFrustum()`; `ThePaths.SetPathsNeededAtPosition(&pos)`.
     c. switch `ms_exitEnterState`:
        - 0: `SetAreaCodeForVisibleObjects()`; state = 1; `currArea = (int32)ms_spawnPoint->m_nArea` (zero-ext);
          `g = GetPedsGroup(ped)`; if g: `ev = CEvent::operator new(0x10)`; `ev = ev ? CEventLeaderEntryExit::ctor(ev, ped) : nullptr`;
          local `CEventGroupEvent ge(ped, ev)`; `g->m_groupIntelligence.AddEvent(&ge)`; `ge.~()`. Return false
          (no zone name in this case).
        - 1: `t = ped->m_pIntelligence->taskMgr.primary[3]`; if `t == nullptr || t->GetTaskType() != 932`
          (0x3A4 TASK_COMPLEX_GOTO_DOOR_AND_OPEN): `SetFadeColour(0,0,0)`; `Fade(1.0f, 0)`; state = 2.
          Then DisplayName (step 7) and return false.
        - 2: if `TheCamera.GetFading()` return false; else state = 3 and fall to the common tail (step 6).
        - any other value (3, negative, >3): go to the common tail.
  5. PATH B (vehicle, or flags 0x2/0x200):
        - state 0: `SetFadeColour(0,0,0)`; `Fade(0.5f, 0)`; state = 2; `g = GetPedsGroup(ped)`; if g:
          `ev = CEvent::operator new(0x10)`; if ev: `CEventAreaCodes::ctor(ev, ped)` then `*(void**)ev = 0x859540`
          (this is an inlined CEventLeaderEntryExit ctor, identical to 0x43E1C0); `CEventGroupEvent ge(ped, ev)`;
          AddEvent; `~ge`. Then DisplayName (step 7), return false.
        - state 2: if `!GetFading()` state = 3. Return false (does NOT continue to the tail in the same call).
        - any other value (1, 3, ...): `currArea = (int32)ms_spawnPoint->m_nArea`; `ms_numVisibleEntities = 0`;
          fall to the common tail.
  6. Common tail:
     a. `ped->m_nAreaCode = (u8)currArea`; if `ped->m_pVehicle && (ped[+0x46C] & 0x100)`: `veh->m_nAreaCode = (u8)currArea`.
     b. `ped->m_pEnex = (currArea != 0) ? this : nullptr` (32-bit currArea compare).
     c. `AddEntryExitToStack(this)`; `ped[+0x474] |= 0x04000000`; `CPad::GetPad(0)->[+0x10E] &= 0xF7`;
        `CClothes::RebuildPlayerIfNeeded(ped)`.
     d. if `this->m_nFlags & 0x200`: `ms_spawnPoint->m_nFlags &= ~0x2000` (on ms_spawnPoint, not this); return true.
     e. TheCamera position := pos (inlined CPlaceable::SetPosn: if `TheCamera.m_matrix` set matrix->pos
        (+0x30/+0x34/+0x38), else TheCamera+4/+8/+0xC); `TheCamera.RestoreWithJumpCut()`.
     f. `CAudioZones::Update(true, pos)`; `CWaterLevel::FindNearestWaterAndItsFlow()`;
        `CGarages::CloseHideOutGaragesBeforeSave()`; `ResetAreaCodeForVisibleObjects()`;
        `g_interiorMan.SetEntryExitPtr(this)`; `CPopulation::RemoveAllRandomPeds()`; `this->RequestAmbientPeds()`;
        `LoadAllRequestedModels(false)`; `CTimer::Suspend()`.
     g. if `currArea == 0` (int32): `f = FindFarClipForCoors(pos)` (stored to f32); `RwCameraSetFarClipPlane(TheCamera.m_pRwCamera, f)`.
     h. `ms_spawnPoint->RequestObjectsInFrustum()`; `LoadScene(&pos)`; `LoadAllRequestedModels(false)`;
        `this->GenerateAmbientPeds(&pos)`.
     i. if `g_interiorMan.Update()`: `SetLoadVehiclesInLoadScene(false)`; `LoadScene(&pos)`; `SetLoadVehiclesInLoadScene(true)`.
     j. `CTimer::Resume()`; `ClearFlagForAll(0x20)`.
     k. `c = ms_spawnPoint->m_nSkyColor`; if `c != 0`: `StartExtraColour(c - 1, false)` else `StopExtraColour(false)`.
     l. `CRubbish::SetVisibility(ms_spawnPoint->m_nFlags & 1)` (low byte & 1, passed as int 0/1).
     m. if `ped[+0x46C] & 0x100` (in vehicle):
        `v = ped->m_pVehicle`; `h = v->GetHeightAboveRoad()` (virtual +0xD4, ST0); `z' = (pos.z - 1.0f) + h`
        (x87: `fld pos.z; fsub [0x858624]; faddp` -> rounded to f32 on store); `v->Teleport({pos.x, pos.y, z'}, false)`;
        `a = ms_spawnPoint->m_fExitAngle * 0.017453292f` (f32 fmul, stored f32); if `v->m_matrix`
        `v->m_matrix->SetRotateZOnly(a)` else `v->placement.heading = a`.
        else (on foot):
        `ms_spawnPoint->FindValidTeleportPoint(&pos)` (ECX = ms_spawnPoint, pos updated in place);
        `ped->Teleport(pos, false)` (virtual +0x38); `a = ms_spawnPoint->m_fExitAngle * 0.017453292f`;
        `ped->m_fCurrentRotation = a` (fst f32); `ped->m_fAimingRotation = a` (fstp of the same register);
        if `ped->m_matrix` `SetRotateZOnly(m_fCurrentRotation)` else `ped->placement.heading = m_fCurrentRotation`.
     n. (both branches) `TheCamera.Fade(1.0f, 1)`.
     o. `CTheScripts::ClearSpaceForMissionEntity(ped->m_matrix ? &ped->m_matrix->pos : &ped->placement.pos, ped)`.
     p. `f = ms_spawnPoint->m_nFlags`; if `f & 0x10`: `CShopping::RemoveLoadedShop()`; else if `f & 0x08`:
        `CShopping::LoadShop(this->m_szName)`; if `this->m_pLink` then `this->m_nFlags |= 0x10` (byte +0x30).
     q. `CPopulation::ManageAllPopulation()`; `CTheScripts::Process()`.
     r. if `ms_spawnPoint->m_nFlags & 0x100`: `this->WarpGangWithPlayer(ped)`. Then `this->ProcessStealableObjects(ped)`.
     s. `ms_spawnPoint->m_nFlags &= ~0x2000`; if `ms_spawnPoint->m_nFlags & 0x8000`:
        `CEntryExitManager::DeleteOne((ms_spawnPoint - pool->m_pObjects) / 0x3C)` (signed byte-difference
        division by 60 via magic multiply 0x88888889, truncating toward zero).
     t. `ped->m_pIntelligence->taskMgr.GetTaskSecondary(3)->StopAll()` (no null check).
     u. `CGame::TidyUpMemory(true, true)`; return true.
  7. DisplayName: `d = this->GetEntryExitToDisplayNameOf()`; if d: `CHud::SetZoneName(TheText.Get(d->m_szName), true)`.
- Edge cases: PATH A state 2 -> 3 continues to the tail in the same call, PATH B state 2 -> 3 does not. PATH B
  treats state 1 like 3. Step 6.t crashes if the ped has no secondary facial task (original behaviour).
  The FindFarClipForCoors temp is spilled into the stack slot of the `ped` argument (irrelevant to a port).
- gta-reversed: hooked with `.Reversed = false`; the body is a `plugin::Call` and the commented-out code is wrong
  in several places, do NOT port it: (1) state 1 fade condition inverted (original fades when the primary task is
  absent or NOT GOTO_DOOR_AND_OPEN); (2) PATH B default case covers every state except 0/2, not only 3;
  (3) `m_pEnex` condition inverted relative to gr's `CanSeeOutSideFromCurrArea` wording - original: `currArea != 0 ? this : null`;
  (4) step 6.d clears bEnteredWithoutExit on ms_spawnPoint, not this; (5) far clip is set when `currArea == 0`;
  (6) g_interiorMan calls and CRubbish::SetVisibility are present (gr TODO); (7) vehicle Z adds GetHeightAboveRoad();
  angles use ms_spawnPoint->m_fExitAngle; (8) FindValidTeleportPoint is called on ms_spawnPoint with the exit pos
  as in/out; (9) Fade(1,1), ClearSpaceForMissionEntity, shopping, population, scripts, gang warp, stealables,
  DeleteOne, StopAll and TidyUpMemory run for the vehicle branch too; (10) shopping logic: RemoveLoadedShop if
  bUsedRewardEntrance, LoadShop only if bRewardInterior, and the 0x10 flag is set on `this`; (11) returns true.
- Open questions: none affecting behaviour.
