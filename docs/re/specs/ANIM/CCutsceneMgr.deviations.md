# CCutsceneMgr: gta-reversed deviation notes (ANIM)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-2 group F.

## 0x4D5AF0 CCutsceneMgr::RemoveEverythingBecauseCutsceneDoesntFitInMemory  (gr_deviations = 1)
Row CutsceneMgr.cpp:873 (`NOTSA_UNREACHABLE(); // Unused function`).
- Reachability: no static callers (catalogue), but the original has a body (`__cdecl void()`):
  1. if `restoreEverythingAfterCutscene` (0xB5F85C) != 0: `CGame::DrasticTidyUpMemory(1)` (0x53C810); return.
  2. `CWorld::ClearExcitingStuffFromArea(*FindPlayerCoors(&tmp, -1), 120.0f, 1)` (0x56E010, 0x56A0D0);
     `CColStore::RemoveAllCollision()`; `CWorld::bProcessCutsceneOnly (0xB7CD6D) = 1`; `CStreaming::RemoveCurrentZonesModels()`;
     `while (CStreaming::RemoveLoadedVehicle()) ;`; `CRadar::RemoveRadarSections()`;
     `SetModelIsDeletable` + `SetModelTxdIsDeletable` for models 7, 346, 334, 345 (in that order, each pair) (0x4D5B48-0x4D5B8D).
  3. `p = FindPlayerPed(-1)`; `p->RemoveWeaponAnims(0, -1000.0f)` (0x5F0250); `numPlayerWeaponsToRestore = 0`; for the 13 weapon
     slots (p+0x5A0, stride 0x1C) with `type != 0`: `playerWeaponsToRestore_Type[n] = type` (0xB5EB90), `_Ammo[n] = totalAmmo (+0xC)`
     (0xB5EB5C), n++; store n (0xB5EB58); `p->ClearWeapons()` (0x5E6320); `restoreEverythingAfterCutscene = 1`;
     `CGame::DrasticTidyUpMemory(1)`.
- Verdict: CONFIRMED (gr has no body; it is dead code in practice).
- Class: keep-original (port the body above).

## 0x5AFBC0 CCutsceneMgr::LoadCutsceneData_postload  (gr_deviations = 5)
Original bytes (`__cdecl void()`): `CMessages::ClearThisPrintBigNow(0)`; `PedDensityMultiplier (0x8D2530) = 0`; `CarDensityMultiplier
(0x8A5B20) = 0`; `ms_disableStreaming (0x9654B0) = 0`; `s = RwStreamOpen(2, 1, "ANIM\\CUTS.IMG")`; **`sprintf(gString (0xB71670), "%s.IFP",
ms_cutsceneName)`** (CRT 0x821BB5); `if (ms_pCutsceneDir->FindItem(gString, &pos, &size))` { `MakeSpaceFor(size << 10)`;
`ImGonnaUseStreamingMemory()`; `RwStreamSkip(s, pos << 11)`; `CAnimManager::LoadAnimFile(s, 1, ms_aUncompressedCutsceneAnims)`;
`ms_cutsceneAssociations.CreateAssociations(ms_cutsceneName, ms_cLoadAnimName, ms_cLoadObjectName, 32)` (0x4CE3B0);
`IHaveUsedStreamingMemory()`; `ms_animLoaded = 1` } else `ms_animLoaded = 0`; `RwStreamClose(s, 0)`.
Then `f = CFileMgr::OpenFile("ANIM\\CUTS.IMG", "rb")`; **`sprintf(gString, "%s.DAT", ms_cutsceneName)`**; if FindItem(gString, ...):
`ImGonnaUseStreamingMemory(); CFileMgr::Seek(f, pos << 11, 0); TheCamera.LoadPathSplines(f); IHaveUsedStreamingMemory();
dataFileLoaded (0xBC4006) = 1` else 0; `CFileMgr::CloseFile(f)`; `ms_cutsceneLoadStatus (0xB5F84C) = 2`; `ms_cutsceneTimerS = 0`;
`pd = FindPlayerPed(-1)->m_pPlayerData`; `CWanted::ClearQdCrimes(pd ? pd->m_Wanted : null)` (called with ECX = 0 when pd is null).
Row CutsceneMgr.cpp:454 (ScopeGuard closing the stream): Verdict REFUTED (the original closes the stream once after the block,
0x5AFC8E). Class: harmless.
Rows :471, :472 (asserts on the name arrays): Verdict REFUTED (debug only; the original passes 32 = array length). Class: harmless.
Row :489 (ScopeGuard closing the IMG file): Verdict REFUTED (CloseFile at 0x5AFD12). Class: harmless.
Row :492 (`format_to_sz(csSplinesFile, "{}.DAT", ...)` into a local):
- Verdict: CONFIRMED - the original formats both names into the global scratch buffer `gString` (0xB71670), leaving "<name>.DAT"
  in it afterwards; gr uses local buffers (also for the .IFP name at gr :457).
- Class: keep-original (cheap: write gString; it is a shared scratch buffer).

## 0x5B05A0 CCutsceneMgr::LoadCutsceneData_preload  (gr_deviations = 4)
Row CutsceneMgr.cpp:513 (gr-only helper `LoadCutSceneFile`, "Code from at 0x5B0794"):
- Original bytes (0x5B0785-0x5B0826): `sprintf(gString (0xB71670), "%s.CUT" (0x868A44), ms_cutsceneName)`; `if (!ms_pCutsceneDir->FindItem(gString,
  &pos, &size))` -> `LoadCutsceneData_postload()` and return (0x5B07B8 -> 0x5B108F). Else `pos <<= 11; size <<= 11` (stored);
  `buf = operator new(size)` (0x821195, **size = sectors * 2048**); `s = RwStreamOpen(2, 1, "ANIM\\CUTS.IMG")`; `RwStreamSkip(s, pos)`;
  `RwStreamRead(s, buf, size)`; `RwStreamClose(s, 0)`; then the line parser continues in a separate chunk (jump to 0x401ED9 at 0x5B0826).
- Verdict: CONFIRMED - (1) gr allocates `csFileSzBytes * STREAMING_SECTOR_SIZE` after already multiplying by the sector size
  (`CutsceneMgr.cpp:524-527`), i.e. 2048x too much (reads are the same); (2) the original formats the name into the global `gString`,
  gr into a local.
- Class: keep-original (allocate size<<11; write gString).
- Open question: the section parser chunk (from 0x401ED9) was not compared statement by statement with gr's helper.
Row :836 (TODO/BUG: missing bound in the "next free CUTOBJ" loop):
- Original bytes (0x5B1114-0x5B114E): special model: `id = 300 + n` stored to `ms_iModelIndex[i]` (0xBC31C0), `RequestSpecialModel(id, name,
  0x1C)`; `n++`; then `while (ms_aInfoForModel[300 + n].m_nLoadState (byte 0x8E6440 + (300+n)*0x14... table base 0x8E6440 = info+0x10 at
  model 0) == 1) n++` with no upper bound.
- Verdict: REFUTED (gr keeps the same unbounded loop). Class: keep-original (bug kept by both).
Rows :839 and :844 (asserts): Verdict REFUTED (debug only; the original has no checks; MODEL_USE_PREV (3) copies `ms_iModelIndex[i-1]`
  via table 0xBC31BC even for i = 0, 0x5B1155). Class: harmless.

## 0x5B11C0 CCutsceneMgr::LoadCutsceneData_loading  (gr_deviations = 1)
Row CutsceneMgr.cpp:391 (`notsa::try_ston<uint32>({ csfx.m_szObjectPart })`, dereferenced with `*nodeIdx`).
- Original bytes (FX loop, per tCutsceneParticleEffect, stride 0x6C, 0x5B1272-0x5B136F): `g_fx.CreateMatFromVec(&m, &pos, &dir)`;
  `objMat = null`; `id = m_nObjectId`; if `id > 0 && id < ms_numCutsceneObjs + 1` (0xBC3FE4): `obj = ms_pCutsceneObjects[id-1]`
  (table 0xBC3F18), `clump = obj->m_pRwClump`; if `GetFirstAtomic(clump)` and its geometry has a skin: **`n = atol(m_szObjectPart)`**
  (CRT `_atol` 0x82258E: no validation, non-numeric -> 0, signed); `h = GetAnimHierarchyFromSkinClump(clump)`;
  `objMat = RpHAnimHierarchyGetMatrixArray(h) + RpHAnimIDGetIndex(h, n) * 64`; else if `f = CClumpModelInfo::GetFrameFromName(clump,
  part)`: **`objMat = RwFrameGetLTM(f)`** (0x7F0990); then `m_pFxSystem = g_fxMan.CreateFxSystem(name, &m, objMat, 1)` (0x4A9BB0).
- Verdict: CONFIRMED - gr dereferences an empty optional (UB) for a non-numeric part and parses unsigned; the original uses `atol`.
- Class: keep-original.
- Additional gr logic errors in the same function (not catalogued rows):
  - gr returns `RwFrameGetMatrix(frame)` (the frame's local matrix) where the original calls `RwFrameGetLTM(frame)` (0x5B1338).
  - Attachment loop (0x5B1375-0x5B13D8): the original does, for each tCutsceneAttachment {pedId +0, attachId +4, bone +8}:
    `obj = ms_pCutsceneObjects[pedId]`, `att = ms_pCutsceneObjects[attachId]`; `att->m_pAttachmentObject (+0x180) = obj`;
    `att->m_nAttachBone (+0x17C) = RpHAnimIDGetIndex(GetAnimHierarchyFromSkinClump(obj->m_pRwClump), bone)`. gr calls
    `AttachObjectToBone(objs[pedId], objs[pedId], bone)` (`CutsceneMgr.cpp:413`), attaching the ped object to itself and never
    touching the attachment object. Keep-original.

## 0x5B1720 CCutsceneMgr::Update_overlay  (gr_deviations = 2)
Original bytes (`__cdecl void()`):
  1. `CIplStore::AddIplsNeededAtPosn(&ms_cutsceneOffset)`. `st = ms_cutsceneLoadStatus` (0xB5F84C): 1 -> `CTimer::Suspend();
     LoadCutsceneData_loading(); CTimer::Resume()` (tail jump) ; 2 -> step 2; **any other value -> return** (0x5B1749 `jne 0x5B1966`).
  2. Play status (0xBC3FE0), jump table 0x5B1968 on `status - 1` (unsigned `ja` -> no change for 0, negative or > 4):
     1 -> SetupCutsceneToStart, HideRequestedObjects, if `stricmp(ms_cutsceneName, "finale") != 0` {DisableEffectsLoading,
     DisableAllPedSpeech, PlayPreloadedCutsceneTrack}, `status++`; 2, 3 -> `status++`; 4 -> `status = 0`.
  3. if `!ms_running` (0xB5F851) return. `t = CTimer::ms_fTimeStepNonClipped * 0.02f [0x858B38] + ms_cutsceneTimerS`; `fst` to
     ms_cutsceneTimerS; `ms = _ftol2(t * 1000.0f)` using the x87 value (0x5B17B8-0x5B17DD).
  4. FX loop (stride 0x6C, base 0xBC1D90): if `playing (+0x48)` and `endTime (+0x8) != -1` and `fx (+0x0) != 0` and
     **`(uint32)ms > (uint32)(start (+0x4) + end)`** (`jbe` skip): `fx->Stop()`, playing = 0, stopped (+0x49) = 1. Then (not `else`)
     if `fx != 0 && !playing && (uint32)ms >= (uint32)start && !stopped`: `fx->Play()`, playing = 1.
  5. Text: if `ms_currTextOutput < ms_numTextOutput` (signed) and **`(uint32)ms > (uint32)ms_iTextStartTime[cur]`** (`jbe` skip):
     `CMessages::AddMessageJump(TheText.Get(ms_cTextOutput[cur]), ms_iTextDuration[cur], 1, 1)`; cur++.
  6. For each cutscene object with model 300..319: `UpdateCutsceneObjectBoundingBox(clump, model)`.
  7. if `dataFileLoaded` and name != "finale" and active cam mode == 17 (FLYBY) and load status == 2: if
     `(uint32)(ms + 1000) > CCamera::GetCutSceneFinishTime()` and `!g_bCutSceneFinishing`: set it, `TheCamera.Fade(1.0f, 0)`;
     if `IsCutsceneSkipButtonBeingPressed()`: `byte [0xBAAD40] = 0` (CHud big message), `ms_wasCutsceneSkipped = 1`, tail-jump FinishCutscene.
Row CutsceneMgr.cpp:1020 (play-status switch `default: NOTSA_UNREACHABLE()`):
- Verdict: CONFIRMED (out-of-range status: the original leaves it unchanged; gr UB). Class: keep-original.
Row :1084 (load-status switch `default: NOTSA_UNREACHABLE()`):
- Verdict: CONFIRMED (any status other than 1/2 returns in the original; gr's only caller filters NOT_LOADED, but other values are UB).
  Class: keep-original.
- Additional differences (not catalogued rows): gr uses signed `>=` for the FX stop test and text start test where the original uses
  unsigned strict `>`; gr computes `ts / 50.0f` where the original multiplies by 0.02f (rounding); gr converts the stored f32 timer.
  Keep-original.
