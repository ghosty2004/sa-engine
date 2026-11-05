# CStreaming (STREAM) - full specs, writer A (RE-SPEC-1)

Layout notes used below (gta-reversed `StreamingInfo.h`, `Streaming.h`, validated by the bytes):
- `CStreaming::ms_aInfoForModel` = `CStreamingInfo[26316]` at 0x8E4CC0, stride 0x14.
  `CStreamingInfo`: +0x0 i16 m_NextIndex, +0x2 i16 m_PrevIndex, +0x4 i16 m_NextIndexOnCd, +0x6 u8 m_Flags,
  +0x7 u8 m_ImgID, +0x8 u32 m_CDOffset, +0xC u32 m_CDSize, +0x10 u8 m_LoadState (1 = LOADSTATE_LOADED).
- `CModelInfo::ms_modelInfoPtrs` = `CBaseModelInfo*[20000]` at 0xA9B0C8. `CBaseModelInfo` +0xA = i16 m_nTxdIndex.
- Resource id bases: TXD = 20000 (0x4E20), IFP = 25575 (0x63E7), special characters = model 290 (0x122).

## 0x407800 CStreaming::IsObjectInCdImage  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl bool IsObjectInCdImage(int32 modelId)`; arg at [esp+4]; return AL; plain `ret`.
  Body lives in `.HOODLUM` at 0x1565170 (0x407800 is a 5-byte `jmp` stub).
- Callers: 1 (CRunningScript::ProcessCommands2000To2099 0x472310). Callees: none.
- Globals: R `ms_aInfoForModel[modelId].m_CDSize` (0x8E4CCC + modelId*0x14, u32).
- Behaviour: `eax = modelId*5; ecx = *(u32*)(0x8E4CCC + eax*4); al = (ecx != 0)`.
- Edge cases: only AL is defined as the result; bits 8..31 of EAX hold `(modelId*5) >> 8` (callers only
  test AL). No bounds check on modelId.
- gta-reversed: `IsObjectInCdImage` (Streaming.cpp:2990) returns `GetInfo(modelId).GetCdSize() != 0`;
  matches the bytes.
- Open questions: none.

## 0x407A10 CStreaming::ForceLayerToRead  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void ForceLayerToRead(int32 arg)`; arg at [esp+4]; plain `ret`. Body in `.HOODLUM`
  at 0x156FEE0.
- Callers: 1 (CCutsceneMgr::DeleteCutsceneData_overlay 0x5AFD60). Callees: none.
- Globals: W u8 0x9654C4 (`CStreaming::ms_bReadLayerForceFully`, gr also aliases it as
  `m_bModelStreamNotLoaded`); W i32 0x9654C8 (unnamed in gr, "unused").
- Behaviour: `*(u8*)0x9654C4 = 1; *(i32*)0x9654C8 = arg;` (in that order). EAX = arg on return (no
  caller uses it).
- gta-reversed: Streaming.cpp:2659, same semantics.
- Open questions: none.

## 0x407A30 CStreaming::SetLoadVehiclesInLoadScene  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void SetLoadVehiclesInLoadScene(bool enable)`; reads only the low byte of [esp+4];
  plain `ret`. Body in `.HOODLUM` at 0x1562C80.
- Callers: 2 (CEntryExit::TransitionFinished 0x4404A0, CCutsceneMgr::LoadCutsceneData_preload 0x5B05A0).
- Globals: W u8 0x8A5A88 `CStreaming::ms_bLoadVehiclesInLoadScene`.
- Behaviour: `*(u8*)0x8A5A88 = (u8)arg;` (the raw low byte is stored, not normalised to 0/1).
- gta-reversed: Streaming.cpp:2995, same.
- Open questions: none.

## 0x407AC0 CStreaming::RemoveUnusedModelsInLoadedList  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (void)`; body is a single `ret`.
- Callers: 2 (CGameLogic::SortOutStreamingAndMemory 0x441440, CCutsceneMgr::LoadCutsceneData_overlay 0x5B13F0).
- Behaviour: no-op. Registers and memory unchanged.
- gta-reversed: Streaming.cpp:2477, `// NOP`. Matches.

## 0x407BE0 CStreaming::ImGonnaUseStreamingMemory  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (void)`; body is a single `ret`.
- Callers: 6 (CMenuManager::LoadAllTextures, CMenuManager::SwapTexturesRound, CCutsceneMgr::LoadCutsceneData_postload,
  CCutsceneMgr::CreateCutsceneObject, CCutsceneMgr::SetCutsceneAnim, CPlantMgr::Initialise).
- Behaviour: no-op in the 1.0 US binary.
- gta-reversed: Streaming.cpp:3786 calls `CMemoryMgr::PushMemId(MEM_STREAMING)` (not hooked). That is
  NOT what the binary does: the port must be an empty function (keep-original).
- Open questions: none.

## 0x407BF0 CStreaming::IHaveUsedStreamingMemory  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (void)`; body is a single `ret`.
- Callers: same 6 as 0x407BE0.
- Behaviour: no-op in the 1.0 US binary.
- gta-reversed: Streaming.cpp:3780 calls `CMemoryMgr::PopMemId(); UpdateMemoryUsed();` (not hooked).
  The binary does neither: port as an empty function (keep-original).
- Open questions: none.

## 0x407F20 CStreaming::LoadInitialVehicles  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (void)`; body is a single `ret`.
- Callers: 2 (CCutsceneMgr::DeleteCutsceneData_overlay 0x5AFD60, CGame::Init2 0x5BA1A0).
- Behaviour: no-op. gta-reversed Streaming.cpp:1241 `// NOP` matches.

## 0x4096C0 CStreaming::StartRenderEntities  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl void (void)`; plain `ret`.
- Callers: 1 (CRenderer::ConstructRenderList 0x5556E0, as a tail call).
- Globals: W ptr 0x8E48A0 `CStreaming::ms_renderEntityLink` (`CLink<CEntity*>*`); the value is the
  address 0x9654F0 = `&CStreaming::ms_rwObjectInstances` (a `CLinkList<CEntity*>`, whose first member is
  `usedListHead`, so this is `&ms_rwObjectInstances.usedListHead`).
- Behaviour: `*(u32*)0x8E48A0 = 0x9654F0;`
- gta-reversed: Streaming.cpp:3073, same.
- Open questions: none.

## 0x409C70 CStreaming::SetModelTxdIsDeletable  (catalogue: specified/REA, port level 8)
- Convention: `__cdecl void SetModelTxdIsDeletable(int32 modelId)`; arg at [esp+4]; ends in a tail `jmp`
  to 0x409C10 `CStreaming::SetModelIsDeletable(int32)` after overwriting its own argument slot.
- Callers: 6 functions (CCheat::VehicleCheat, CCutsceneMgr::RemoveEverythingBecauseCutsceneDoesntFitInMemory x4,
  CPopulation::RemoveSpecificDriverModelsForCar (2 calls + 6 tail calls), CPlane::DoPlaneGenerationAndRemoval,
  CTrain::DoTrainGenerationAndRemoval).
- Callees: `orig!(0x409C10 CStreaming::SetModelIsDeletable)(id)`.
- Globals: R `CModelInfo::ms_modelInfoPtrs[modelId]` (0xA9B0C8).
- Fields: `CBaseModelInfo` +0xA i16 m_nTxdIndex (sign-extended with `movsx`).
- Behaviour: `mi = ms_modelInfoPtrs[modelId]; id = (int32)(int16)mi->m_nTxdIndex + 20000;
  [esp+4] = id; jmp SetModelIsDeletable` - i.e. `SetModelIsDeletable(TXDToModelId(mi->m_nTxdIndex))`.
- Edge cases: mi is not null-checked (null -> access violation, same as the original). The caller's
  argument slot on the stack is overwritten with the new id (callers are cdecl and clean up; none reads
  the slot afterwards in the listed callers - a port doing a normal call is equivalent).
- gta-reversed: Streaming.cpp:3045, same semantics.
- Open questions: none.

## 0x40B490 CStreaming::SetMissionDoesntRequireSpecialChar  (catalogue: specified/REA, port level 10)
- Convention: `__cdecl void (int32 slot)`; tail `jmp` to 0x409C90 `CStreaming::SetMissionDoesntRequireModel`
  with the argument slot rewritten.
- Callers: 2 (CMissionCleanup::Process 0x468560, CRunningScript::ProcessCommands600To699 0x47F370).
- Callees: `orig!(0x409C90 CStreaming::SetMissionDoesntRequireModel)(slot + 290)`.
- Behaviour: `SetMissionDoesntRequireModel(slot + 0x122)` (290 = SPECIAL_MODELS_RESOURCE_ID). Plain int add,
  wraps on overflow.
- gta-reversed: Streaming.cpp:3014, same.
- Open questions: none.

## 0x40C450 CStreaming::ProcessEntitiesInSectorList  (catalogue: specified/REA, port level 2)
The overload without radius checks: `template<class PtrListType> void ProcessEntitiesInSectorList(PtrListType& list, int32 streamingFlags)`.
- Convention: `__cdecl void (CPtrList* list, int32 streamingFlags)`; list at [esp+4], flags at [esp+8];
  plain `ret`. Saves/restores EBP, EBX, ESI, EDI.
- Callers: 1 function (CStreaming::AddModelsToRequestList 0x40D3F0, 3 call sites: Buildings, repeat-sector
  Peds, Dummies lists).
- Callees: `orig!(0x4087E0 CStreaming::RequestModel)(int32 modelId, int32 flags)` (cdecl);
  `orig!(0x52CEE0 CClock::GetIsTimeInRange)(uint8 from, uint8 to)` (cdecl, returns AL);
  virtual `CBaseModelInfo::GetTimeInfo` (vtable +0x14, slot 5, thiscall, returns `CTimeInfo*` in EAX);
  virtual `CEntity::CreateRwObject` (vtable +0x1C, slot 7, thiscall, no args).
- Globals read: u16 0xB7CD78 `CWorld::ms_nCurrentScanCode`; i32 0xB72914 `CGame::currArea`;
  u8 `ms_aInfoForModel[id].m_LoadState` (0x8E4CD0 + id*0x14); `CModelInfo::ms_modelInfoPtrs[id]` (0xA9B0C8).
- Fields used:
  - list: `CPtrList` +0x0 = first node. Node (`CPtrNode`, single link): +0x0 item (CEntity*), +0x4 next.
  - CEntity: +0x18 m_pRwObject, +0x1C u32 flags (bit 7 0x80 m_bIsVisible, bit 10 0x400 m_bStreamingDontDelete,
    bit 19 0x80000 m_bDontStream), +0x22 i16 model index (read with `movsx`), +0x2C u16 scan code,
    +0x2F u8 area code.
  - CBaseModelInfo: +0x1C m_pRwObject.
  - CTimeInfo: +0x0 u8 timeOn, +0x1 u8 timeOff.
- Behaviour:
  1. `node = list->first; if (node == NULL) return;`
  2. Loop (do-while, list order): `e = node->item; node = node->next;` (next is fetched before any work, so
     the loop is safe against nothing in particular - it simply reads it early).
  3. `sc = ms_nCurrentScanCode; if (e->scanCode == sc) continue; e->scanCode = sc;`
  4. `id = (int32)(int16)e->modelIndex; if (ms_aInfoForModel[id].m_LoadState == 1) continue;`
  5. `f = e->flags` (read once); `if (f & 0x400) continue;` (m_bStreamingDontDelete)
  6. Area: `if ((u32)e->areaCode != CGame::currArea && e->areaCode != 13) continue;` (zero-extended byte
     compared as a 32-bit int with currArea; then byte compare with 13).
  7. `if (f & 0x80000) continue;` (m_bDontStream) `if (!(f & 0x80)) continue;` (m_bIsVisible clear).
  8. `mi = ms_modelInfoPtrs[id]; ti = mi->GetTimeInfo();` if `ti != NULL` and
     `!CClock::GetIsTimeInRange(ti->timeOn, ti->timeOff)` (AL == 0) -> continue.
  9. `if (mi->m_pRwObject != NULL && e->m_pRwObject == NULL) e->CreateRwObject();`
  10. `RequestModel((int32)(int16)e->modelIndex, streamingFlags);` (model index re-read after step 9).
  11. Loop while `node != NULL`.
- Edge cases: the model index is sign-extended (gr declares it u16; ids >= 0x8000 would index negatively
  in the original - not reachable with valid data). `mi` is not null-checked. Bits 8..31 of the
  GetIsTimeInRange result are ignored (only AL tested).
- gta-reversed: Streaming.cpp:2538-2560 (`ProcessEntitiesInSectorList(PtrListType&, int32)`), same order of
  checks (`IsScanCodeCurrent`, `IsLoaded`, `m_bStreamingDontDelete`, `IsInCurrentArea`, `m_bDontStream ||
  !m_bIsVisible`, time info, CreateRwObject, RequestModel). No behavioural difference found.
- Open questions: none.

## 0x48B570 CStreaming::SetMissionDoesntRequireAnim  (catalogue: specified/REA, port level 10)
- Convention: `__cdecl void (int32 slot)`; tail `jmp` to 0x409C90 with the argument slot rewritten.
- Callers: 1 (CRunningScript::ProcessCommands1200To1299 0x48B590).
- Callees: `orig!(0x409C90 CStreaming::SetMissionDoesntRequireModel)(slot + 25575)`.
- Behaviour: `SetMissionDoesntRequireModel(slot + 0x63E7)` (IFPToModelId).
- gta-reversed: Streaming.cpp:3000, same.
- Open questions: none.
