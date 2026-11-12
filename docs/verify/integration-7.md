# INTEGRATE-7: the native path (STD-10..14), VERIFIER-9 and the writers

Integration branch `worktree-agent-aa1e6f32d27702b8b`, from master `efe5120`. INTEGRATE-6
(`worktree-agent-aa4c0e8384f020c37`) had not finished when this started (no `docs/verify/integration-6.md`), so it
is skipped, and PORT-AI-7 comes in through the STD-14 chain instead.

Every source branch contributed only its own commits, as cherry-picks. None of their master snapshots were
merged. For conflicts in generated files (`catalog/verify-queue.csv`, `docs/verify/QUEUE.md`, `crates/*/generated*.rs`,
`crates/sa-ai/src/gen_*.rs`, `tools/gen/exclude_inflight.txt`, `docs/arch/platform-mvp-gaps.csv`), the integrated
side was kept and the files were then regenerated with `tools/gen/ai_patterns.py`, `tools/gen/inflight.py`,
`tools/gen/patterns.py` (`--check` clean on both generators) and `python3 -I tools/verify/queue.py`.

## What came from where

| step | source (branch, commits) | taken as | notes |
|---|---|---|---|
| 1 | INTEGRATE-6 | skipped | not finished at 09:01 |
| 2a | STD-13 `worktree-agent-adab932ed2ad46a44` `ca917d6..9e704a6` (53 commits: STD-10, STD-11, STD-12, STD-13 and its WRITER-O/P/Q/INTRO picks) | 49 picks | skipped because master already has them: `d0bd681` (= `0ba8327`), `ab960d0` (= `abe5e42`), `08075c2` (= `c97b6d1`), `2077fc0` (= `399478d`, WRITER-P); `bf33506d` is regeneration only (empty after resolution). Conflicts in `crates/sa-game` were resolved in favour of STD-13; master's `--samp` host skeleton was then put back onto STD-13's `Session` loop (`fe187de`) |
| 2b | STD-14 `worktree-agent-a98204e188a13ff2e` = WRITER-NATIVE `worktree-agent-a6d319b53fd3185ec`, `9e704a6..3ab9516` (36 commits) | picks | these carry RENDER-SHADOW `7e080de1` / `86d6e5fd` / `39842281`, PORT-AI-7 `f9328ce` / `462c48d` / `5e4797c`, PORT-AI-8 `60ec250` / `e030fcd`, WRITER-ATTR attr-1 and attr-3..attr-7, WRITER-NATIVE native-3..native-13 and STD-14's std14-* commits |
| 2c | WRITER-ATTR `worktree-agent-a1954952637efd64a` | `dcbe3d2`, `53c1f7e`, `5ee67d7`, `2c9d51a` | attr-2, attr-7 rest, attr-8; `84ed7ba` / `b3bd508` are queue regeneration only |
| 2d | WRITER-NATIVE | `092fa86` (0x421A50) | `f19bf238` (AI-GEN regeneration) gives the same output as our own regeneration |
| 3 | VERIFIER-9 `worktree-agent-a6555c420bac1a0c9` | `dbcac4a`, `44d4157`, `1f4dbaf`, `fa78175` | 0x5F1FA0 `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached` keeps `unverified = "intro-ai-1"` (main-session decision); `verdicts-native.tsv` marks it `open` |
| 4a | PORT-VEH-3 `worktree-agent-a5414716572d221d7` | `1e2f939`, `be17e4f` | |
| 4b | WRITER-WORLD `worktree-agent-aeecd424d0eaab708` | `cf6ff377`, `6bcc8891` | |
| 4c | WRITER-SAMP `worktree-agent-ac88fcf86cc63a973` | `39986d4c` | |
| 4d | PORT-AI-8 `worktree-agent-a70c1c8af9cc7a9c7` | `d879c4d2` | `60ec250` / `e030fcd` came in through 2b; generator and exclude list taken, AI-GEN modules regenerated |
| 4e | PORT-VEH-4 `worktree-agent-a2678b899a4c57db1` | `08c723d`, `49cc1ac3`, `df2eefba` | resolved by hand (see below) |
| 4f | SAMP-UI `worktree-agent-a76d04bde0aeeaa67` | `3323627..90afa23` (10 commits) | `--samp-ui-demo` and `samp_input` put back onto STD-13's `Session` (the UI filters are applied in `Session::iterate`, so window mode and offscreen mode both go through them) |
| 4g | STD-14 after `3ab9516` | `7db1048`, `df65922`, `963962e`, `6f54784`, `03ef542`, `35ef39b`; `d6d8ecd`'s settlement reproduced | `df65922` adds the AI-GEN exclusion of 0x5F2B80 / 0x5F3730 / 0x5F3B40: their generated bodies called 0x5F1FA0 as a one-argument thiscall, but it takes three cdecl arguments |
| fix | this integration | `b34c610` | `CPedAttractor` `orig!` declarations of ComputeDeltaPos / ComputeDeltaHeading returned `X87`, and the phase A DLL did not build (an `orig!` return must fit in a register pair). They now return f64 (ST0) through `_st0` wrappers |

Not taken: SAMP-4 (main-session decision). WRITER-SAMP-2 `a089a405` / `855a5ba0` was offered late and is not taken,
because it would have needed a second full verify-all (open item).

## Duplicates settled (one registration per address)

- STD-14's head is the reference.
- `hooks.rs` / `lib.rs` / `mod.rs` / sa-ports `NATIVES` lists are unions; the lines that the merges doubled
  were removed (`visibility_vehicle`, `visibility_weapon`, `float_pow`, `vehicle_collision_particles`).
- Hand ports beat generated hooks through the generator, not through hand edits: `ai_patterns.py` skips every
  address that is already ported, plus `tools/gen/ai_patterns_exclude.txt` (PORT-AI-8's list plus STD-14's
  three 0x5F1FA0 callers). STD-14's hand edits of `gen_*.rs` (`3ab9516`) are reproduced by the generator
  (`--check` clean).
- WRITER-SAMP's copies were dropped in favour of WRITER-NATIVE / WRITER-ATTR / STD-13:
  - hooks 0x46F690, 0x640330, 0x6478B0, 0x647950 (`car_task_ctors_dtors.rs`)
  - hooks 0x4AD960, 0x4AFE70, 0x4B1170 (`event_ref_dtors.rs`)
  - hook 0x4A62B0 (`fx_info_get_value_samp.rs`)
  - hook 0x734E50 (`quad_marker_wind_index_buffer.rs`; WRITER-ATTR's `rw_tex_dict_find_hash.rs` stays)
  - natives 0x4AD420 and 0x5E01B0 (master's `std7_natives` has them)
- WRITER-WORLD's 0x59EE80 `CHandObject::Render` gave way to WRITER-ATTR's `hand_object.rs`.
- 0x6B7080 `CBike::PlayCarHorn` and 0x64F240 `CCarEnterExit::IsPlayerToQuitCarEnter` follow STD-14 (`d6d8ecd`):
  PORT-VEH-3's ports stay, and STD-13/14's `bike_car_horn.rs` / `player_quit_car_enter.rs` are removed.
- PORT-VEH-4 against WRITER-WORLD: WRITER-WORLD's copies stay for these:
  - BreakObject_c ×7 (`break_object.rs`)
  - GivePlayerGoodiesWithPickUpMI 0x4564F0 (`pickup_goodies.rs`)
  - Remove2ndPlayerIfPresent 0x4413C0 (the WORLD hook stays; PORT-VEH-4's native is not registered)
  - InteriorGroup_c::ArePathsLoaded (`interior_paths_loaded.rs`; PORT-VEH-4's `interior_group_paths.rs` dropped)
  - WaterCreature_c (`water_creature.rs`)

  PORT-VEH-4's `entry_exit_transition.rs` (TransitionStarted / Finished) is the only copy and is kept.
- Crane native 0x6F3FF0: one registration (`short_entry_natives.rs`).
- keycells `READERS`: the union by `at` gives **43** readers (STD-14's 41 plus WRITER-SAMP's 0x1561116 and
  0x403968). There is no duplicate `at`.
- `PRESERVE_SPECS_PENDING`: union (PORT-AI-8's 0x6289F0, WRITER-SAMP's 0x4ADE90 / 0x4B13C0).
- `tools/e2e/l2-extras.txt`: union. `catalog/verify-status.csv`: union with one row per address (0x4D2E10 keeps
  VERIFIER-9's newer row).
- Still registered twice (`sa-game` reports "duplicate #[hook] ... (used)"; the first one is used):
  - 0x5B0440 and 0x618E70: both registrations were already on master.
  - 0x84A540, 0x84E300, 0x84F320: master's `static_inits` natives and PORT-VEH-3's static-initialiser hooks.
    STD-14 has the same pair.

## Tests

- `cargo test -p sa-hook --lib`: 58 passed. This includes `every_hook_installs_on_the_exe`, the keycells reader
  test and `unverified_selection`.
- `cargo test -p sa-script --lib`: 9 passed. `-p sa-core --lib`: 72 passed. `-p sa-rw-wgpu`: 58 passed.
- `cargo test -p sa-oracle --test l1 --release`: 17 passed, 1 ignored, and `preserved_registers_match_the_hook_registrations`
  failed. That test flagged STD-12's skid-value hooks 0x4F5F30 / 0x4F6000, which use `preserve = edx` but have no
  spec. They are now in `PRESERVE_SPECS_PENDING` (`a46da0b`), and the test passes on a rerun. The full run took
  1422 s and did not hang.
- sa-oracle and satool were rebuilt in this worktree (release).
- `i686-pc-windows-gnu` release build of sa-launcher and sa-hook: clean. This needed the f64 fix above.
- sa-game `i686-unknown-linux-gnu` release build plus a native smoke run. Command: `--start-in-world --fixed-step 40
  --frames 300`, offscreen, `SA_GAME_SCRIPT_SKIP=1`, with no exploration stubs (STD-14's play package has none
  left). Result: **316 ok, 0 blocked, 0 original-only, 0 crashed**, 788,937 platform calls served natively,
  13,291 Rust ports in the `orig!` registry.

## verify-all

`SA_ROOT=~/sa-engine tools/e2e/verify-all.sh int7 6,7` at `b34c610`, DLL sha256 `c813154e…`. **IDENTICAL on all
13 recordings**, 17 subsystems each:

| recording | frames |
|---|---|
| grove walk (inf4) | 5200 |
| air-water | 5800 |
| anim-input | 3300 |
| car-ai | 5000 |
| c-bug | 3220 |
| driving | 4338 |
| menus-hud | 5700 |
| menus-map-controls | 4000 |
| pause-restart | 3700 |
| police-gang | 5200 |
| save-load | 3900 (save files identical except 750 stack-residue bytes, ledger SAVE-1, as on master) |
| weapon-combat | 3500 |
| world-cheats | 3276 |

The disk guard skipped no recording, and no step had to be dropped.

The first attempt failed for two reasons: the phase A DLL did not build (fixed in `b34c610`), and satool had not
been built in the worktree yet. Neither is a divergence.

Changes after `b34c610`:
- WRITER-SAMP's two duplicate natives were removed. This matters for phase B only, but it changes the DLL bytes
  (sha256 `281422d7…`).
- `PRESERVE_SPECS_PENDING`, the verdict note, the MVP gap list and this document changed.

The final head `a46da0b` (DLL `281422d7…`) was rechecked as `verify-all.sh int7b 6,7` on the grove walk (inf4,
5200 frames) and on driving (4338 frames): IDENTICAL on both.

## VERIFIER-9 promotions for the catalogue (162)

VERIFIER-9 promoted 163 hooks; 0x5F1FA0 is held back, so 162 are listed here. The 18 L1 promotions have 10^6 native
L1 cases each. The 144 L2 promotions have a split (or an in-process L2) run with 0 mismatches and 0 undeclared
writes. All 162 also passed the L3 batch union on 13 recordings. Per-hook evidence is in
`docs/verify/verdicts-native.tsv`. None of the 162 is left in the regenerated `catalog/verify-queue.csv`.
`catalog/state.csv` is not touched; the catalogue update belongs to the main session.

| address | level | name | batch |
|---|---|---|---|
| `0x40EE20` | L1 | CBox::Recalc | std9-coll-1 |
| `0x40EE70` | L1 | CColBox::Set | std6-world-1 |
| `0x40FF20` | L1 | CColSphere::IntersectRay | std8-world-1 |
| `0x4100E0` | L1 | CColSphere::IntersectEdge | std8-world-1 |
| `0x44E480` | L1 | CVector2D::Normalise | std8-math-1 |
| `0x59A840` | L1 | CMatrix::ConvertToEulerAngles | std9-math-1 |
| `0x59AA40` | L1 | CMatrix::ConvertFromEulerAngles | std9-math-1 |
| `0x59ADF0` | L1 | CMatrix::operator+= | std6-math-2 |
| `0x59AEA0` | L1 | CMatrix::ResetOrientation | std6-math-2 |
| `0x59AF80` | L1 | CMatrix::SetTranslateOnly | std6-math-2 |
| `0x59B460` | L1 | CMatrix::Rotate | std9-math-1 |
| `0x59B6A0` | L1 | CMatrix::Reorthogonalise | std8-math-1 |
| `0x59B7E0` | L1 | CMatrix::ForceUpVector | std8-math-1 |
| `0x59B920` | L1 | CMatrix::Invert-1 | std6-math-2 |
| `0x59BDD0` | L1 | CMatrix::Invert-2 | std6-math-2 |
| `0x59BE30` | L1 | CMatrix::operator*-Mat | std6-math-2 |
| `0x59BFA0` | L1 | CMatrix::operator+- | std6-math-2 |
| `0x6FE260` | L1 | RoadsignGetLineAndRow | mvpn-render-1 |
| `0x4042D0` | L2 | CRect::IsPointInside-Tolerance | std7-math-1 |
| `0x40FC80` | L2 | thunk_FUN_015637e0 | std8-world-1 |
| `0x411A80` | L2 | CMatrix::operator*= | std7-math-1 |
| `0x41B350` | L2 | CCollisionPlugin::SetModelInfo | std6-render-1 |
| `0x4442D0` | L2 | thunk_FUN_0156cd40 | std6-core-1 |
| `0x445C30` | L2 | CGangWars::ReleasePedsInAttackWave | std8-pop-1 |
| `0x452090` | L2 | CPathFind::Find2NodesForCarCreation | std8-pop-1 |
| `0x45B160` | L2 | CReplay::EnableReplays | std7-world-1 |
| `0x45C440` | L2 | CReplay::ShouldStandardCameraBeProcessed | std8-core-1 |
| `0x464BD0` | L2 | CRunningScript::RemoveScriptFromList | std7-script-1 |
| `0x464D70` | L2 | CRunningScript::IsPedDead | std7-script-1 |
| `0x465070` | L2 | thunk_FUN_01561010 | std7-core-1 |
| `0x470150` | L2 | CRunningScript::PlayAnimScriptCommand | mvpn-script-1 |
| `0x474800` | L2 | CTheScripts::ScriptAttachAnimGroupToCharModel | std7-script-1 |
| `0x479DA0` | L2 | CRunningScript::ProcessCommands2600To2699 | mvpn-script-2 |
| `0x485A50` | L2 | CRunningScript::DoDeathArrestCheck | std7-script-1 |
| `0x486B00` | L2 | CTheScripts::ClearSpaceForMissionEntity | std6-script-1 |
| `0x4A8E30` | L2 | FUN_004a8e30 | std8-render-1 |
| `0x4ABA50` | L2 | RwHelper::GetEventGlobalGroup | std7-ai-1 |
| `0x4B92B0` | L2 | CEventHandler::ComputeObjectCollisionResponse | intro-ai-2 |
| `0x4B9340` | L2 | CEventHandler::RegisterKill | intro-ai-2 |
| `0x4B9400` | L2 | CEventHandler::ComputeDeathResponse | intro-ai-2 |
| `0x4B9470` | L2 | CEventHandler::ComputeDeadPedResponse | intro-ai-3 |
| `0x4B9B50` | L2 | CEventHandler::ComputePedToFleeResponse | intro-ai-2 |
| `0x4B9BE0` | L2 | CEventHandler::ComputeAttractorResponse | intro-ai-2 |
| `0x4BB2E0` | L2 | CEventHandler::ComputeVehicleOnFireResponse | intro-ai-3 |
| `0x4BBB90` | L2 | CEventHandler::ComputeObjectCollisionPassiveResponse | intro-ai-2 |
| `0x4BC3E0` | L2 | CEventHandler::IsKillTaskAppropriate | intro-ai-2 |
| `0x4BC710` | L2 | CEventHandler::ComputeShotFiredResponse | intro-ai-4 |
| `0x4BCC30` | L2 | CEventHandler::ComputeDraggedOutCarResponse | intro-ai-5 |
| `0x4BD6A0` | L2 | CEventHandler::ComputeVehicleCollisionResponse | intro-ai-4 |
| `0x4BDB80` | L2 | CEventHandler::ComputePedCollisionWithPedResponse | intro-ai-1 |
| `0x4BF9B0` | L2 | CEventHandler::ComputePersonalityResponseToDamage | intro-ai-4 |
| `0x4C0BD0` | L2 | CEventHandler::ComputeVehiclePotentialCollisionResponse | intro-ai-5 |
| `0x4C19A0` | L2 | CEventHandler::ComputePedThreatResponse | intro-ai-1 |
| `0x4C2610` | L2 | CEventHandler::ComputePotentialPedCollideResponse | intro-ai-1 |
| `0x4C2FC0` | L2 | CEventHandler::ComputeVehicleDamageResponse | intro-ai-4 |
| `0x4C3430` | L2 | CEventHandler::ComputeGotKnockedOverByCarResponse | intro-ai-3 |
| `0x4C35F0` | L2 | CEventHandler::ComputeSeenPanickedPedResponse | intro-ai-3 |
| `0x4D1610` | L2 | CQuaternion::Normalise | std7-anim-1 |
| `0x4D34F0` | L2 | RpAnimBlend::RpAnimBlendClumpUpdateAnimations | std7-anim-1 |
| `0x4D5F50` | L2 | RpAnimBlend::RpAnimBlendAllocateData | std6-anim-1 |
| `0x4D60E0` | L2 | RpAnimBlend::RpAnimBlendCreateAnimationForHierarchy | std6-anim-1 |
| `0x4D64A0` | L2 | RpAnimBlend::RpAnimBlendClumpFillFrameArray | std6-anim-1 |
| `0x4D6720` | L2 | RpAnimBlend::RpAnimBlendClumpInit | std6-anim-1 |
| `0x4DF210` | L2 | CAEGlobalWeaponAudioEntity::ServiceAmbientGunFire | std7-audio-1 |
| `0x52CD90` | L2 | CClock::Initialise | std6-render-1 |
| `0x5404A0` | L2 | FUN_005404a0 | std6-input-1 |
| `0x556B40` | L2 | CRopes::RegisterRope | std8-render-1 |
| `0x557530` | L2 | CRope::Update | std9-render-2 |
| `0x59B0A0` | L2 | CMatrix::SetRotateY | std6-math-2 |
| `0x59B120` | L2 | CMatrix::SetRotate-xyz | std6-math-1 |
| `0x59B1E0` | L2 | CMatrix::RotateX | std6-math-1 |
| `0x59B390` | L2 | CMatrix::RotateZ | std6-math-1 |
| `0x59BAD0` | L2 | CCompressedMatrixNotAligned::CompressFromFullMatrix | std7-math-1 |
| `0x5A41C0` | L2 | CClothesBuilder::RequestGeometry | clump-1 |
| `0x5A4220` | L2 | CClothesBuilder::RequestTexture | clump-1 |
| `0x5A42B0` | L2 | CClothesBuilder::nullptr | clump-1 |
| `0x5A44A0` | L2 | CClothesBuilder::nullptr | clump-1 |
| `0x5A44C0` | L2 | CClothesBuilder::PreprocessClothesDesc | clump-1 |
| `0x5A4840` | L2 | CClothesBuilder::AddWeightToBoneVertex | clump-1 |
| `0x5A48B0` | L2 | CClothesBuilder::StoreBoneArray | clump-1 |
| `0x5A4940` | L2 | CClothesBuilder::BlendGeometry-3 | clump-1 |
| `0x5A55A0` | L2 | CClothesBuilder::ConstructGeometryArray | clump-1 |
| `0x5A56E0` | L2 | CClothesBuilder::BuildBoneIndexConversionTable | clump-1 |
| `0x5A57B0` | L2 | CClothesBuilder::PlaceTextureOnTopOfTexture | clump-1 |
| `0x5A59C0` | L2 | CClothesBuilder::BlendTextures-Dst-Src1-Src2 | clump-1 |
| `0x5A6040` | L2 | CClothesBuilder::ConstructTextures | clump-1 |
| `0x5A69D0` | L2 | CClothesBuilder::CreateSkinnedClump | clump-1 |
| `0x5B05A0` | L2 | CCutsceneMgr::LoadCutsceneData_preload | mvpn-anim-2 |
| `0x5D5B40` | L2 | CCarFXRenderer::CustomCarPipeClumpSetup | std6-render-1 |
| `0x5D5B80` | L2 | CCarFXRenderer::IsCCPCPipelineAttached | std6-render-1 |
| `0x5D6830` | L2 | FUN_005d6830 | std6-render-1 |
| `0x5D6850` | L2 | CCustomBuildingDNPipeline::NVCPipelineProcess | std6-render-1 |
| `0x5D6E90` | L2 | CCustomBuildingDNPipeline::GetExtraVertColourPtr | std6-render-1 |
| `0x5D7120` | L2 | CCustomBuildingDNPipeline::CustomPipeMaterialSetup | std6-render-1 |
| `0x5D71C0` | L2 | CCustomBuildingDNPipeline::CustomPipeAtomicSetup | std6-render-1 |
| `0x5D7200` | L2 | CCustomBuildingDNPipeline::PreRenderUpdate-Atomic | std6-render-1 |
| `0x5D7F00` | L2 | CCustomBuildingRenderer::AtomicSetup | std6-render-1 |
| `0x5D7F40` | L2 | CCustomBuildingRenderer::IsCBPCPipelineAttached | std6-render-1 |
| `0x5D7F80` | L2 | CCustomBuildingRenderer::UpdateDayNightBalanceParam | std6-render-1 |
| `0x5D8050` | L2 | CCustomBuildingRenderer::Update | std6-render-1 |
| `0x5D8870` | L2 | CCustomCarEnvMapPipeline::PreRenderUpdate | std9-render-3 |
| `0x5DA230` | L2 | CCustomCarEnvMapPipeline::SetFxEnvTexture | std6-render-1 |
| `0x5DA560` | L2 | CCustomCarEnvMapPipeline::CustomPipeMaterialSetup | std6-render-1 |
| `0x5DA610` | L2 | CCustomCarEnvMapPipeline::CustomPipeAtomicSetup | std6-render-1 |
| `0x5DA6A0` | L2 | RxD3D9InstanceDataRenderLighting | std9-render-4 |
| `0x5DABC0` | L2 | CGrassRenderer::SetGlobalCameraPos | std8-render-1 |
| `0x5DAC00` | L2 | CGrassRenderer::SetGlobalWindBending | std8-render-1 |
| `0x5FFA20` | L2 | CEntityScanner::ScanForEntitiesInRange | std7-ai-1 |
| `0x64F860` | L2 | CCarEnterExit::SetAnimOffsetForEnterOrExitVehicle | std6-veh-1 |
| `0x681FE0` | L2 | (missed by ghidra: cat7-call) | std8-ai-1 |
| `0x685170` | L2 | CTaskComplexWalkAlongsidePed::ControlSubTask | mvpn-ai-1 |
| `0x698F30` | L2 | CCover::AddCoverPoint | std7-ai-1 |
| `0x6997E0` | L2 | CCover::Update | std7-ai-1 |
| `0x6FEDA0` | L2 | CCustomRoadsignMgr::CreateRoadsignAtomicA | mvpn-render-1 |
| `0x708290` | L2 | RenderBuffer::StartStoring | std9-render-8 |
| `0x722220` | L2 | MarkerAtomicCB | std8-render-1 |
| `0x72FC40` | L2 | PipelinePlugin::GetPipelineID | std6-render-1 |
| `0x72FC50` | L2 | PipelinePlugin::SetPipelineID | std6-render-1 |
| `0x7316D0` | L2 | FUN_007316d0 | std8-ped-1 |
| `0x732230` | L2 | CVisibilityPlugins::SetModelInfoIndex | std6-render-1 |
| `0x732260` | L2 | CVisibilityPlugins::GetModelInfo | std6-render-1 |
| `0x7322B0` | L2 | CVisibilityPlugins::SetAtomicFlag | std6-render-1 |
| `0x732310` | L2 | CVisibilityPlugins::ClearAtomicFlag | std6-render-1 |
| `0x732330` | L2 | CVisibilityPlugins::clearClumpForAllAtomicsFlagCB | std6-render-1 |
| `0x732350` | L2 | CVisibilityPlugins::ClearClumpForAllAtomicsFlag | std6-render-1 |
| `0x732370` | L2 | CVisibilityPlugins::GetAtomicId | std6-render-1 |
| `0x732380` | L2 | CVisibilityPlugins::SetUserValue | std6-render-1 |
| `0x7323A0` | L2 | CVisibilityPlugins::GetUserValue | std6-render-1 |
| `0x732480` | L2 | CVisibilityPlugins::RenderAtomicWithAlpha | std9-render-5 |
| `0x732500` | L2 | CVisibilityPlugins::CalculateFadingAtomicAlpha | std9-render-7 |
| `0x732610` | L2 | CVisibilityPlugins::RenderFadingAtomic | std9-render-7 |
| `0x7328A0` | L2 | CVisibilityPlugins::SetAtomicRenderCallback | std6-render-1 |
| `0x732A00` | L2 | CVisibilityPlugins::SetFrameHierarchyId | std6-render-1 |
| `0x732B40` | L2 | CVisibilityPlugins::RenderEntity | std9-render-7 |
| `0x733160` | L2 | CVisibilityPlugins::SetupVehicleVariables | std9-render-8 |
| `0x7335B0` | L2 | CVisibilityPlugins::RenderPedCB | std9-render-5 |
| `0x7337D0` | L2 | CVisibilityPlugins::RenderFadingUnderwaterEntities | std9-render-6 |
| `0x733800` | L2 | CVisibilityPlugins::RenderReallyDrawLastObjects | std9-render-6 |
| `0x733E90` | L2 | CVisibilityPlugins::RenderAlphaAtomics | std9-render-6 |
| `0x733EC0` | L2 | CVisibilityPlugins::RenderBoatAlphaAtomics | std9-render-6 |
| `0x733F10` | L2 | CVisibilityPlugins::RenderFadingEntities | std9-render-6 |
| `0x734570` | L2 | CVisibilityPlugins::InsertEntityIntoSortedList | std9-render-1 |
| `0x734880` | L2 | RwHelper::Get2DEffectAtomic | std6-rw-1 |
| `0x7348C0` | L2 | RwHelper::GetFirstObject | std6-rw-1 |
| `0x734900` | L2 | RwHelper::GetFirstChild | std6-rw-1 |
| `0x734930` | L2 | GetFirstTextureCallback | std8-render-1 |
| `0x734A40` | L2 | RwHelper::GetAnimHierarchyFromSkinClump | std6-rw-1 |
| `0x734B10` | L2 | RwHelper::GetAnimHierarchyFromClump | std6-rw-1 |
| `0x734B90` | L2 | RwHelper::AtomicRemoveAnimFromSkinCB | std6-rw-1 |
| `0x7567A0` | L2 | (missed by ghidra: data-code-immediate) | std9-rw-2 |
| `0x756830` | L2 | FUN_00756830 | std9-rw-2 |
| `0x756960` | L2 | (missed by ghidra: cat7-call) | std9-rw-2 |
| `0x75D530` | L2 | _rpTriListMeshGenerate | std8-rw-1 |
| `0x75D730` | L2 | (missed by ghidra: data-code-immediate) | std8-rw-1 |
| `0x7C4C30` | L2 | RpHAnimHierarchyCreate | std6-rw-1 |
| `0x7C4ED0` | L2 | RpHAnimHierarchyCreateFromHierarchy | std6-rw-1 |
| `0x7C6A10` | L2 | cb_SKINPLUGIN_RpGeometry_Dtor | std8-rw-1 |
| `0x7CCE40` | L2 | RtAnimAnimationCreate | std6-anim-1 |
| `0x7CCF10` | L2 | RtAnimAnimationDestroy | std6-rw-1 |
| `0x7CD520` | L2 | RtAnimInterpolatorCreate | std6-rw-1 |
| `0x7CD590` | L2 | RtAnimInterpolatorDestroy | std6-rw-1 |
| `0x805710` | L2 | RxPipelineExecute | std9-rw-1 |

## Open items

- WRITER-SAMP-2 (`worktree-agent-aa6be1d1b3cc0f218`: `a089a405`, `855a5ba0`) is not taken. It came too late for a
  second verify-all. When it is taken:
  - FireInstantHit 0x73FB10 needs detwin to skip its scrub when that port is selected.
  - Its `Fx_c::AddSparks` file duplicates WRITER-NATIVE's `fx_add_sparks.rs`.
- SAMP-4 is not taken (main-session decision).
- These addresses are still registered twice in phase B (the first registration wins):
  - 0x5B0440 and 0x618E70, both already doubled on master.
  - 0x84A540, 0x84E300 and 0x84F320: master's `static_inits` natives and PORT-VEH-3's hooks.
- 0x5F1FA0 stays unverified, and the three AI-GEN bodies that call it (0x5F2B80, 0x5F3730, 0x5F3B40) stay excluded
  from the generator.
- STD-12's 0x4F5F30 / 0x4F6000 need preserve-edx specs. They are listed in `PRESERVE_SPECS_PENDING`.
- `tools/gen/exclude_inflight.txt` lists the other agents' branches as they stood at about 09:36. Rerun it after the
  next integration.
