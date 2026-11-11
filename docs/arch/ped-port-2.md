# PORT-PED-2: the PED rows pass 1 deferred (crate `sa-ped`)

Second PED pass after PORT-WEAPON-PED (`weapon-ped-port.md`, which lists why each row was deferred). Order: SA:MP
priority (`catalog/samp-priority.csv`, player/ped sync) first, then rows the corpus executes. Agent + 2 helpers
(disjoint new files), all game runs by the agent on lane 5. Verdicts per function: `ped-port-2-verdicts.tsv`.
Ports follow the 1.0 US bytes, never gta-reversed's fixes (specs `docs/re/specs/PED/*.md`, `catalog/gr-deviations.csv`).

## Files

| module | contents | hooks |
|---|---|---|
| `ped_life.rs` (agent) | `CPed` constructor 0x5E8030 / destructor 0x5E8620 / scalar deleting destructor 0x5E8C80; `CCivilianPed` ctor/dtors (0x5DDB70, 0x5DDBE0, 0x5DDC30); `CCopPed` ctor/dtors (0x5DDC60, 0x5DE0D0, 0x5DE2C0); `CEmergencyPed` dtors (0x5DE3B0, 0x5DE3E0); `CPlayerPed` dtors (0x6093B0, 0x60A9A0); `CPed::SetModelIndex` 0x5E4880; `CPed::operator new-poolIndexed` 0x5E4730 and the pool's handle-indexed `New` (FUN_005e0540); `CPedIntelligence::operator new` 0x6074A0 | 17 |
| `ped_render.rs` (agent) | `CPed::PreRender` 0x5E8A20, `SetupLighting` 0x553F00, `RemoveWeaponModel` 0x5E3990, `AddWeaponModel` 0x5E5ED0, `GiveWeapon-` 0x5E6080, `RequestDelayedWeapon` 0x5E8910, `GiveDelayedWeapon` 0x5E89B0, `RemoveWeaponAnims` 0x5F0250 (RW/FX calls stay `orig!`) | 8 |
| `damage.rs` (agent, appended) | `CPedDamageResponseCalculator::ComputeWillKillPed` 0x4B3210, `ComputeDamageResponse` 0x4B5AC0 | 2 |
| `ped_physics.rs` (helper A) | `CPed::SetPedPositionInCar` 0x5DF910, `UpdatePosition` 0x5E1B10, `ProcessBuoyancy` 0x5E1FA0, `SpecialEntityPreCollisionStuff` 0x5E3C30, `SpecialEntityCalcCollisionSteps` 0x5E3E90, `Teleport` 0x5E4110, `SetMoveAnim` 0x5E4A00 | 7 |
| `player_ctrl.rs` (helper B) | `CPlayerInfo::MakePlayerSafe` 0x56E870, `Clear` 0x56F330, `WorkOutEnergyFromHunger` 0x56E610; `CPlayerPed::UpdateCameraWeaponModes` 0x609830, `ReApplyMoveAnims` 0x609650, `SetupPlayerPed` 0x60D790, `SetInitialState` 0x60CD20, `MakeChangesForNewWeapon-` 0x60B460, `ControlButtonSprint` 0x60A610, `HandlePlayerBreath` 0x60A8D0, `SetPlayerMoveBlendRatio` 0x60C520, `ProcessAnimGroups` 0x6098F0, `ProcessWeaponSwitch` 0x60D850; plain (unhookable) `ProcessPlayerWeapon` 0x6097F0, `SetMoveAnim` 0x609490 | 13 |

47 new `#[hook]` registrations (sa-ped total 217 + 47 = 264), 2 more ported but unhookable, 7 bare-`ret` rows
recorded as unhookable (nothing to port): `CWanted::ResetPolicePursuit` 0x561FD0, `CPlayerInfo::CancelPlayerEnteringCars`
0x56E860, `CPed::Update` 0x5DEBE0, `CPed::UpdateStatEnteringVehicle` 0x5E01A0, `CPed::UpdateStatLeavingVehicle`
0x5E01B0, `CPed::SpawnFlyingComponent` 0x5F0190, `~CPedDamageResponseCalculator` 0x4AD420. No `preserve` was needed
(sa-hook `callers` test and `hookscan.py` section 2 clean for every new entry); no protection read of a new entry
(`patch_refs`, `data_reads`, hookscan section 1).

## Findings (original kept)

- `CPed::~CPed` 0x5E8620 starts `push -1; jmp 0x40136C`: a protection chunk pushes the SEH handler computed as
  `[0x1560624] - [0x6FF415] - CF` and jumps back to 0x5E8627. Only the SEH handler value depends on it; the hook
  steals the 7 entry bytes (relocated jmp) and the port skips the SEH frame. Install, patch_refs and data_reads tests
  pass; the destructor ran on Rust 99 times in the split run with 0 mismatches.
- `CCopPed::CCopPed` jump table 0x5DDE5C: cop types 6 and > 7 use the type value itself as the model index
  (`push eax` after `ja`/case 6), type 0 asks `CStreaming::GetDefaultCopModel`.
- `CPed::CPed` sets bit 17 of +0x46C from `(rand() & 3) == 0` (return address 0x5E82DE) and writes the `CTask*` it
  allocates into its own argument slot; with the switch byte 0x96913F it builds a `CEventAcquaintancePedHate`
  (0x18 bytes, Rust stack buffer, original ctor/dtor) with word +0xE = 1000.
- `CPed::RemoveWeaponAnims` looks up the same association id (0xE0) 0x22 times (the loop counter is not the id).
- `CPedDamageResponseCalculator::ComputeDamageResponse` pushes the `CEventDamage` in-vehicle flag as
  `(flags >> 8) & 0xFFFFFF01` (upper bits kept, as the original) and calls `CPlayerPed::AnnoyPlayerPed(false)` on a
  hit player (the `push 0` before `FindPlayerPed(-1)` stays on the stack as its argument).
- `ComputeWillKillPed` writes the old and new health into its first argument slot (stack-only, not reproduced).
- Helper A: `SetPedPositionInCar` zeroes its `CMatrix` locals (the original leaves them uninitialised apart from the
  fields it sets) and takes the heading through `X87::fpatan` (f64 model, same FPATAN root as the PHYS
  `ApplySpeed` ledger item); `SpecialEntityCalcCollisionSteps` reproduces AL only (the caller reads `movzx edx, al`);
  `ProcessBuoyancy` leaves bytes 9..11 of its `CEventInWater` local at 0 (uninitialised in the original);
  `Teleport` zeroes +0xDC after `CleanUpOldReference` (gta-reversed leaves a todo).
- Helper B: `CPlayerInfo::Clear` zeroes word +0x144 of `Players[PlayerInFocus]`, not of `this` (kept);
  `SetInitialState` writes +0xE of a `CEvent::operator new` result even when it is null (kept); stack events
  `CEventPlayerCommandToGroup` / `CEventGroupEvent` in Rust buffers; `ControlButtonSprint` keeps the computed
  value on the x87 stack against f32 bounds (fcomp st(1) order).

## Verification

Lane 5, `SA_ROOT=/home/ghosty2004/sa-engine`, outputs in the worktree's `target/e2e/lane-5/port-l2-<tag>/`.

- Build/tests: clippy `-D warnings` sa-ped host (all targets), sa-ped + sa-hook i686 (`--features sa-ped/in-process`),
  sa-ped wasm32; `cargo test -p sa-ped` 3/3; `cargo test --release -p sa-hook --lib` 51/51 (install, catalogue,
  callers, patch_refs, data_reads, keycells).
- `pp2-life-wc` (`ped2-life.txt`, the agent's 27 hooks, weapon-combat 3500 frames, `L2_MAX=150 SPLIT_PER_FRAME=2
  SPLIT_STACKFILL=16384`, runs `plain rust split`): rust trace IDENTICAL (17 subsystems), splito and splitr traces
  IDENTICAL, split report **20 PASS, 0 PASS-FP, 0 FAIL, 0 DIVERGED, 0 mismatches in every compared pair** (e.g.
  `CPed::Constructor` 127/127, `CPed::Destructor` 99/99, `CPed::SetModelIndex` 128/128, `CCivilianPed::Constructor`
  120/120, `GiveWeapon-` 139/139, `CCopPed::Constructor` 7/7, `ComputeDamageResponse` 13/13). Check (e) failed only
  on undeclared writes: static regions written by the original callees (anim blocks, model-info stores,
  CReferences, conversations, groups, replay buffers, gMatrixList, speech phrase memory, population counters) and
  the `CPedDamageResponse` argument. A `PORT-PED-2` block of `tools/e2e/l2-extras.txt` now declares them (generated
  from that report); the rerun with those regions (`pp2-all-wc`) lost its `splitr` run: /home fell to 17 GB free
  (other agents' runs) and port-l2.sh skipped it. 7 of the 27 had no call (pool `New`, `operator new-poolIndexed`,
  `CPed` scalar deleting destructor, `CEmergencyPed` and `CPlayerPed` destructors).
- `pp2-all-wc` (`ped2-all.txt`, all 47 hooks on Rust): rust trace **IDENTICAL on all 3500 frames, 17 subsystems**;
  splito trace IDENTICAL; splitr skipped (disk).
- All-ports L3 (`tools/e2e/verify-all.sh pp2-va 5`, every engine-crate port on Rust): **IDENTICAL on the grove walk (5200 frames) and air-water (5800)**; the other 9 recordings were not run: port-l2.sh refused them with /home at 17 GB free (other agents' disk use). Rerun `verify-all.sh TAG 5 RECS...` for anim-input, car-ai, c-bug, driving, menus-hud, pause-restart, police-gang, weapon-combat, world-cheats (weapon-combat is already IDENTICAL with the 47 PED-2 hooks on Rust, `pp2-all-wc`).

States (agent proposal; the main session sets `catalog/state.csv`): **L2 4** (`CPedIntelligence::operator new`
127/127, `CPed::PreRender` 150/150, `CPed::SetupLighting` 150/150, `CPed::RemoveWeaponAnims` 4/4: split PASS, 0
mismatches, 0 undeclared), **implemented 43** (split PASS with undeclared writes only, NOCALLS, or no split pair yet),
ported-unhookable 2, unhookable 7.

## Not ported (reason)

- Time box: `CPed::CalculateNewVelocity` 0x5E4C50, `DoFootLanded` 0x5E5380, `PlayFootSteps` 0x5E57F0,
  `PositionPedOutOfCollision` 0x5E0820, `PositionAnyPedOutOfCollision` 0x5E13C0, `ProcessEntityCollision` 0x5E2530,
  `ProcessControl` 0x5E8CD0, `PreRenderAfterTest` 0x5E65A0, `Render` 0x5E7680; `CPlayerPed::ProcessControl` 0x60EA90,
  `SetRealMoveAnim` 0x60A9C0, `Compute3rdPersonMouseTarget` 0x60B650; `CPlayerInfo::Process` 0x56F8D0,
  `FindObjectToSteal` 0x56DBD0, `FindClosestCarSectorList` 0x56F4E0; the CCivilianPed/CCopPed/CEmergencyPed
  `ProcessControl`s, `CPed::Say`, bone/gun-flash helpers, `CAEPedWeaponAudioEntity`, loaders/save (need a
  load/save recording).
- Protection-computed entries still need a runtime measurement (PLAN 4b): `PedIsReadyForConversation` 0x43ABA0,
  `CreateDeadPedMoney` 0x4590F0, `GetIsWearingBalaclava` 0x5A7950, `GetDefaultPlayerMotionGroup` 0x5A81B0,
  `IsPointerValid` 0x5E4220; `RemoveGogglesModel` 0x5DF170 is a protection-read entry (unhookable).

## Ledger proposals (pending)

- Stack-only: SEH frames of every constructor/destructor ported here; `CPed::CPed` writing its task pointer into its
  argument slot; `ComputeWillKillPed` health in its argument slot; `SetPedPositionInCar` / `ProcessBuoyancy` locals
  the original leaves uninitialised (zeroed in Rust; no caller reads them).
- `SetPedPositionInCar` heading via the f64 FPATAN model (same root as the PHYS/CAM FPATAN entries).
- `SpecialEntityCalcCollisionSteps`: EAX upper bytes not reproduced (caller reads AL).
