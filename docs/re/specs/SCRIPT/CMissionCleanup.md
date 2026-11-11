# CMissionCleanup (SCRIPT) — RE-SPEC-REST

## 0x4637C0 CMissionCleanup::FindFree  (catalogue: specified/REA, port level 0)
- `__thiscall`, no args, plain `ret`; returns EAX = pointer to the first entry with `type (+0) == 0` among the 75 (stride 8, u16 counter),
  or NULL. Callers: none in game code.

## 0x4637E0 CMissionCleanup::AddEntityToList(int32 handle, MissionCleanUpEntityType type)  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = CMissionCleanup*; layout gr MissionCleanup.h: 75 entries {u8 type +0, i32 handle +4} stride 8, i8 m_Count at +0x258),
  handle [esp+4], u8 type [esp+8], `ret 8`. Stub 0x4637E0 → .HOODLUM body 0x1565D40. 16 callers (CCarCtrl::CreateCarForScript 0x431F80,
  script command switches 100-199, 200-299, 1800-1899, 2000-2099, 2200-2299, ...).
- Behaviour: first entry i (u16 counter, i < 0x4B) with `type == 0`; none → return. Else `entry.handle = handle; entry.type = type;
  m_Count++` (byte increment, wraps). Duplicates are not checked.
- gta-reversed: catalogue REA (gr C++ exists in MissionCleanup.cpp; not hooked); same logic.

## 0x4652D0 CMissionCleanup::CheckIfCollisionHasLoadedForMissionObjects  (catalogue: specified/REA, port level 9)
- `__thiscall` (ECX = this), no args, plain `ret`. Stub → .HOODLUM body 0x1569D40. Caller: CTheScripts::Process 0x46A000.
  Callees: CColStore::HasCollisionLoaded 0x410CE0 and CIplStore::HaveIplsLoaded 0x405600 (both cdecl (const CVector* pos, i32 area)),
  CEntity::GetIsStatic 0x4633E0, CPhysical::AddToMovingList 0x542800, CAutomobile::PlaceOnRoadProperly 0x6AF420,
  CBike::PlaceOnRoadProperly 0x6BEEB0.
- Behaviour: for each of the 75 entries (in order):
  - type 3 (object pool [0xB7449C], 0x19C) or type 2 (ped pool [0xB74490], 0x7C4): GetAt(handle) as in RemoveEntityFromList; if found and
    `m_nFlags (+0x1C) & 0x40000`: `pos = m_matrix (+0x14) ? &matrix.pos (+0x30) : &m_placement.pos (+0x4)`, `area = (u8)m_nAreaCode (+0x2F)`;
    if `HasCollisionLoaded(pos, area)` and then (pos re-derived) `HaveIplsLoaded(pos, area)`: clear 0x40000 and, if `!GetIsStatic()`,
    `AddToMovingList()`.
  - type 1 (vehicle pool [0xB74494], 0xA18): same checks and flag clear/AddToMovingList, then by `m_nVehicleSubType (+0x590)`: 0 or 11 →
    `CAutomobile::PlaceOnRoadProperly`; 9 → `CBike::PlaceOnRoadProperly`; others nothing (0x1569ED3-0x1569EE6, `sub 0; je`, `sub 9; je`,
    `sub 2; jne`).
  - other types: nothing.

## 0x4654B0 CMissionCleanup::RemoveEntityFromList(int32 handle, MissionCleanUpEntityType type)  (catalogue: specified/REA, port level 3)
- `__thiscall` (ECX = this → EBP), handle [esp+4], u8 type [esp+8], `ret 8`. Stub → .HOODLUM body 0x156E810. 12 callers
  (CRemote::TakeRemoteControlledCarFromPlayer 0x45AE80, CMissionCleanup::Process 0x468560, several command switches).
  Callees: CEntity::GetIsStatic 0x4633E0, CPhysical::AddToMovingList 0x542800.
- Behaviour: for ALL 75 entries (no early exit): if `entry.type == type && entry.handle == handle`:
  1. Entity lookup by type: 1 → vehicle pool [0xB74494] (size 0xA18), 2 → ped pool [0xB74490] (0x7C4), 3 → object pool [0xB7449C]
     (0x19C); pool GetAt: `idx = handle >> 8` (sar), valid when `flags[idx] == (u8)handle`, `ptr = objects + idx*size`. Other types: no
     lookup. If ptr != NULL and `ptr.m_nFlags (+0x1C) & 0x40000` (bIsStaticWaitingForCollision): clear that bit; then if
     `!ptr->GetIsStatic()` → `ptr->AddToMovingList()`.
  2. `entry.handle = 0; entry.type = 0; m_Count--` (byte).
- Edge: with type == 0 every empty slot whose handle equals the argument (normally 0) matches, so `RemoveEntityFromList(0, 0)` decrements
  m_Count once per empty slot (original behaviour).

## 0x468560 CMissionCleanup::Process  (catalogue: specified/REA, port level 32)
(RE-SPEC-FINAL.) Called when a mission ends (and from script command FAIL/cleanup).
- Convention: `__thiscall void (CMissionCleanup* this = ECX)`, plain `ret` (this kept at [esp+0xC], later [esp+0x10]). EBX = 0, EBP = -1 in
  the body. Caller: CRunningScript::ProcessCommands200To299 0x469390. Layout: 75 entries {u8 type +0, i32 handle +4}, stride 8.
- Behaviour, in this exact order:
  1. If `CTheScripts::bScriptHasFadedOut` (0xA4448A): clear it; then if `CWorld::Players[0].m_nPlayerState` (byte 0xB7CE74) is neither 2
     nor 1: `TheCamera.Fade(0.5f, 1)` and `CPad::GetPad(0)->byte[+0x10E] &= 0xDF`.
  2. `CTrain::DisableRandomTrains(false)`; `CTrain::ReleaseMissionTrains()`; `CPlane::SwitchAmbientPlanes(true)`;
     `CPopulation::PedDensityMultiplier` (0x8D2530) = 1.0f; `CCarCtrl::CarDensityMultiplier` (0x8A5B20) = 1.0f.
  3. If `CGangWars::bTrainingMission` (0x96AB91): `arg = (CGangWars::bGangWarsActive (0x96AB90) == 0)`; `bTrainingMission = 0`;
     `CTheZones::FillZonesWithGangColours(arg)`.
  4. `CPopulation::m_AllRandomPedsThisType` (0x8D2534) = -1; `CGangWars::bCanTriggerGangWarWhenOnAMission` (0x96AB93) = 0;
     `CGangWars::ClearSpecificZonesToTriggerGangWar()`; bytes 0xC0FCB2, 0xC0FCB3, 0xC0FCB4 (CPopulation m_bDontCreateRandomGangMembers,
     m_bOnlyCreateRandomGangMembers, m_bDontCreateRandomCops) = 0; `CGarages::NoResprays` (0x96C008) = 0;
     `CGarages::AllRespraysCloseOrOpen(true)`; `CCullZones::bMilitaryZonesDisabled` (0xC87ACD) = 0; `FindPlayerWanted(-1)->dword[+0x14] = 1.0f`;
     `CPickups::RemoveMissionPickUps()`; `CRoadBlocks::ClearScriptRoadBlocks()`; `CStreaming::DisableCopBikes(false)`.
  5. byte 0xB72994 (g_LoadMonitor+0x1C) = 1; `CObject::bArea51SamSiteDisabled` (0xBB4A72) = 0; `CObject::bAircraftCarrierSamSiteDisabled`
     (0x8D0A24) = 1; `ThePaths.ReleaseRequestedNodes()`; `ThePaths.UnMarkAllRoadNodesAsDontWander()`; `ThePaths.TidyUpNodeSwitchesAfterMission()`;
     `CVehicleRecording::RemoveAllRecordingsThatArentUsed()`.
  6. `TheCamera.SetWideScreenOff()`; TheCamera words: +0x46 (0xB6F06E) = 50, +0x48 (0xB6F070) = 49, +0x4A (0xB6F072) = 3, +0x4C (0xB6F074)
     = 48; byte +0x45 (0xB6F06D) = 0; byte 0xB6FD18 = 0; `TheCamera.InitialiseScriptableComponents()`; `TheCamera.ResetDuckingSystem(0)`
     (thiscall, arg 0); `gCurDistForCam` (0x8CCB84) = 1.0f; byte 0xB6EC2C = 0; `CGameLogic::bScriptCoopGameGoingOn` (0x96A8A8) = 0;
     `CTheScripts::bDrawCrossHair` (dword 0xA44490) = 0; `CSpecialFX::bVideoCam` (0xC7C70C) = 0, `bLiftCam` (0xC7C70D) = 0;
     `CPostEffects::ScriptResetForEffects()`; `CEntryExitManager::ms_bDisabled` (0x96A7C8) = 0; if `CGame::currArea == 0`:
     `CTimeCycle::StopExtraColour(0)`.
  7. For `i = 0..3` (u16 counter): `AudioEngine.ClearMissionAudio(i)` (0xB6BC90). `CWeather::ReleaseWeather()`; dword 0xB728E0 = 99999;
     for `i = 0..9`: `CStreaming::SetMissionDoesntRequireSpecialChar(i)`; `CTheScripts::ClearAllSuppressedCarModels()`.
  8. `CTheScripts::ForceRandomCarModel` (0xA4448C) = -1; `CStreaming::ms_disableStreaming` (0x9654B0) = 0; `CHud::m_ItemToFlash` (word 0xBAB1DC)
     = -1; CHud bytes 0xBAA3FB, 0xBAA3F9, 0xBAA3F8, 0xBAA3FA = 0; byte 0xBA677C (FrontEndMenuManager+0x34) = 0; CTheScripts::RadarZoomValue
     (0xA444A3), RadarShowBlipOnAllLevels (0xA444A2), HideAllFrontEndMapBlips (0xA444A1) = 0; `C3dMarkers::ForceRender(0)`;
     `CTheScripts::bDisplayHud` (0xA444A0) = 1; `fCameraHeadingWhenPlayerIsAttached` (0xA4449C) = 0, `fCameraHeadingStepWhenPlayerIsAttached`
     (0xA44498) = 0; `bEnableCraneRaise/Lower/Release` (0xA44496/5/4) = 1; byte 0xBA18D9 = 0; `bUseMessageFormatting` (0xA44B66) = 0;
     `MessageCentre` (word 0xA44B64) = 0; `MessageWidth` (word 0xA44B60) = 0; `bDrawOddJobTitleBeforeFade` (0xA44489) = 1;
     `bDrawSubtitlesBeforeFade` (0xA44488) = 1.
  9. Player group (`P = CWorld::Players[0].m_pPed` re-read before each use, `pd = P->m_pPlayerData (+0x480)`): `g = pd->dword[+0x38]`;
     `gi = 0xC09950 + 0x2D4*g` (CPedGroups::ms_groups[g] + 0x30, the group intelligence); `gi->SetDefaultTaskAllocatorType(1)`;
     `gi->SetGroupDecisionMakerType(-1)`; dword `0xC0994C + 0x2D4*g` = dword [0x86C6C0] (ms_fPlayerGroupMaxSeparation = 120.0f);
     `pd->dword[+0x34] &= ~0x100`; `P->ForceGroupToAlwaysFollow(false)`; `P->ForceGroupToNeverFollow(false)`; `P->MakePlayerGroupReappear()`;
     `pd->byte[+0x43] = 99`; `P->dword[+0x734] = 1.0f`; bytes 0xB7CDDD (Players+0x45) = 1, 0xB7CDDE (Players+0x46) = 0; `P->EnablePedSpeech()`;
     `P->EnablePedSpeechForScriptSpeech()`; `GetPad(0)->SetDrunkInputDelay(0)`; `GetPad(0)->byte[+0x118..+0x11F] = 0` (8 separate calls, in
     ascending order); byte 0xB7CEEB (Players+0x153) = 1.
  10. `t = P->m_pIntelligence->taskManager(+4).FindTaskByType(3, 1600)`; if `t && t->GetTaskType() (vtable +0x10) == 1600`: `0x634A40(t, P)`.
  11. `CVehicle::bDisableRemoteDetonation` (0xC1CC00) = 0, `bDisableRemoteDetonationOnContact` (0xC1CC01) = 0; `CGameLogic::ClearSkip(true)`.
      If `CGameLogic::GameState` (0x96A8B0) is neither 2 nor 1, and `FindPlayerPed(-1)->m_nPedState (+0x530)` is neither 55 nor 54
      (FindPlayerPed called twice): `CRestart::ClearRespawnPointForDurationOfMission()`.
  12. `CTheScripts::RiotIntensity` (0xA444A5) = 0; `gFireManager.ClearAllScriptFireFlags()`; `StoreVehicleIndex` (0xA447EC) = -1;
      `StoreVehicleWasRandom` (0xA447F0) = 1; `CTheScripts::UpsideDownCars[0..5]` (0xA4892C, 8 bytes each) = {handle -1, timer 0};
      `CTheScripts::StuckCars.Init()` (0xA90AB0); `CStats::bShowUpdateStats` (0x8CDE56) = 1; `CEventGunShot::ms_fGunShotSenseRangeForRiot2`
      (0x8A625C) = -1.0f; `CHud::m_fHelpMessageBoxWidth` (0x8D0934) = 200.0f; `CVehicle::ms_forceVehicleLightsOff` (0xC1CC18) = 0.
  13. If `CTheScripts::bMiniGameInProgress` (0xA444A8) == 0: if `CWorld::Players[PlayerInFocus].dword[+0xB0]` (0xB7CE48 + 0x190*idx) == 0:
      `TheCamera.Restore()`; `w = pd ? pd->dword[0] : 0` (the CWanted*; a null pd would write to address 0x1E); `w->byte[+0x1E] &= 0xFE`;
      (re-read) `w->byte[+0x1E] &= 0xFB`; `CWorld::Players[0].MakePlayerSafe(false, 10000.0f)`; `CHud::SetHelpMessage(null, true, false, false)`.
  14. Entries (i = 0..74): skip `type == 0`; by type (jump table 0x468D10, types 1..13):
      1 vehicle / 2 ped / 3 object: pool lookup by handle (`idx = handle >> 8` arithmetic, valid if `pool.byteMap[idx] == (u8)handle`, no range
        check; object = `pool.objects + idx*size`, sizes 0xA18 / 0x7C4 / 0x19C; pools 0xB74494 / 0xB74490 / 0xB7449C); if found
        `CTheScripts::CleanUpThisVehicle / CleanUpThisPed / CleanUpThisObject(obj)`;
      4: `k = GetActualScriptThingIndex(handle, 1)`; if `k >= 0` and `fx = dword[0xA44114 + 8*k]` != 0: `fx->Kill()`, then
        `CTheScripts::RemoveScriptEffectSystem(handle)` (only when fx != 0);
      5: `k = GetActualScriptThingIndex(handle, 8)`; `k >= 0` -> `CPedGroups::RemoveGroup(k)`;
      7: `k = GetActualScriptThingIndex(handle, 6)`; `k >= 0` -> `CScripted2dEffects::ms_activated[k]` (byte 0xC3A1A0 + k) = 0;
      8: `k = GetActualScriptThingIndex(handle, 4)`; `k >= 0` -> `s = 0xC178F0 + 0x40*k` (CTaskSequences::ms_taskSequence[k]); if
        `s->dword[+0x3C] == 0`: `s->byte[+0x38] = 0; s->Flush()`; else `s->byte[+0x38] = 1`; then `ms_bIsOpened[k]` (0xC17898 + k) = 0 and byte
        `0xA43F68 + 4*k` (ScriptSequenceTaskArray) = 0;
      9: `k = GetActualScriptThingIndex(handle, 7)`; `k >= 0` -> `CDecisionMakerTypesFileLoader::UnloadDecisionMaker(k)`;
      11: `GetActualScriptThingIndex(handle, 2) >= 0` -> `CTheScripts::RemoveScriptSearchLight(handle)`;
      12: `GetActualScriptThingIndex(handle, 3) >= 0` -> `CTheScripts::RemoveScriptCheckpoint(handle)`;
      13: `CTheScripts::RemoveScriptTextureDictionary()`;
      6, 10 and > 13: nothing.
      Then, for every non-empty entry (also when the lookup failed): `this->RemoveEntityFromList(entry.handle, entry.type)` (re-read).
  15. Acquaintances: for `a = 0..23`, for `b = 24..31`, for `k = 0..4`: `CPedType::ClearPedTypeAsAcquaintance(k, a, CPedType::GetPedFlag(b))`;
      then for `b = 24..31`, for `k = 0..4`: `CPedType::ClearPedTypeAcquaintances(k, b)`.
- gta-reversed: raw call only (MissionCleanup.cpp:26). gr's `MissionCleanUpEntityType` names value 7 `PED_QUEUE`, but the original treats 7
  as a scripted 2D effect (script thing type 6, `CScripted2dEffects::ms_activated`; 0x468C3C) - name error in gr's enum.
- Open questions: semantics of TheCamera +0x45..+0x4C, pd +0x34/+0x38/+0x43, P +0x734, pad +0x10E/+0x118..+0x11F (stores are exact).

## 0x634A40 CMissionCleanup::sub_634A40 (proposed: CTaskComplexUseMobilePhone::Stop(CPed*))  (catalogue: specified/REA, port level 0)
- `__thiscall void (CTask* this = ECX, CPed* ped [esp+4])`, `ret 4`. Callers: CMissionCleanup::Process (call at 0x468961, on the task found with
  type 1600 = TASK_COMPLEX_USE_MOBILE_PHONE) and the script switch at 0x46D050; it is a method of that task class, not of CMissionCleanup.
- Behaviour: if `this->byte[+0x1D] == 0`: `this->byte[+0x1D] = 1`; `this->MakeAbortable(ped, 0, null)` (virtual slot +0x18). Else nothing.
- gta-reversed: absent. Open questions: none.
