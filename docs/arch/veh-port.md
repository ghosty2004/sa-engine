# PORT-VEH: subsystem VEH, first pass (crate `sa-veh`)

First pass over the catalogue subsystem VEH (714 rows: `CVehicle` 205, `CAutomobile` 112, `CBike` 55, `CTrain`
50, `CCarEnterExit` 32, `CVehicleRecording` 29, `CDamageManager` 27, `CPlane` 25, `CHeli` 25, `CBoat` 24, ...).
This pass ports leaf and near-leaf functions, chosen from `catalog/samp-priority.csv` (VEH rows executed under
SA:MP, 304) and `catalog/port-order.csv` level 0/1, smallest first. Follows ADR-001 and the PORT-POP pattern.
The main session sets `catalog/state.csv`; the per-function verdicts are in `docs/arch/veh-port-verdicts.tsv`.

Ported: **214 functions**: 213 `#[hook]` registrations plus `CBoat::PruneWakeTrail` (ported, unhookable).
Not hooked because they are shorter than the 5-byte patch (`TooShort`): `CVehicle::GetRideAnimData`,
`IsOpenTopCar`, `BreakTowLink` (Rust functions kept), and the `ret`-only virtual defaults (not ported).

## Crate

`crates/sa-veh` (PLAN §2 names the crate `sa-vehicle`; `sa-veh` matches the subsystem key). Same rules as
sa-pop: `#![no_std]`, `#![deny(unsafe_code)]`, game state through `globals!`/`GPtr`, every other catalogued
function through `orig!` (so its toggle governs calls from Rust), every port registered with `#[hook]` and
listed in its module's `HOOKS` (`sa_veh::hooks::ALL`, appended by sa-hook after `sa_pop::hooks::ALL`).
Builds for host, i686 (`--features in-process`) and wasm32. Depends on sa-core, sa-math, sa-stream,
sa-world (field helpers), sa-pop (only for the cross-crate globals test).

| file | contents |
|---|---|
| `common.rs` | field helpers (sa-world's), `rand_at`, `rand_frac`, x87 compare helpers, `model_info`, `b` |
| `damage.rs` | `CDamageManager`: all 25 hookable functions (status getters/setters, Progress*, GetComponentGroup, FuckCarCompletely, ApplyDamage) |
| `vehicle.rs` | `CVehicle`: base-class virtual defaults, getting-in/out flags, winch/rope wrappers, driver checks, lighting bytes, Shutdown (41) |
| `automobile.rs` | `CAutomobile` getters, door queries (incl. the `...U32` jump tables), taxi lights, ResetSuspension, FindWheelWidth, FixTyre, Teleport; `CQuadBike`, `CMonsterTruck` constants (30) |
| `bike.rs` | `CBike` / `CBmx` overrides, horn counter, Fix, RemoveRefsToVehicle (23) |
| `door.rs` | `CDoor::Open/GetAngleOpenRatio/IsClosed/IsFullyOpen`, `CBouncingPanel::ResetPanel/SetPanel` (6) |
| `enterexit.rs` | `CCarEnterExit` leaf helpers (18) |
| `handling.rs` | `cHandlingDataMgr` (9) |
| `train.rs`, `heli.rs`, `boat.rs` | `CTrain` (11), `CHeli`/`CPlane` (17), `CBoat` (4) |
| `checks.rs` | `CVehicleRecording`, `CStuckCarCheck`, `CUpsideDownCarCheck`, `CCurrentVehicle`, `CVehicleAnimGroup` (17) |
| `statics.rs` | 12 VEH static initialisers (`__init_*`) |

Shared-crate changes: sa-hook `table.rs` (sa-veh's hooks appended), `callers.rs` test and sa-oracle
`hook_preserve` chain sa-veh's registrations; sa-hook `keycells.rs` gains the reader `adc ebp, [0x6F4924]`
at 0x401635 (the protection reads `CBouncingPanel::SetPanel`'s entry bytes). No L1 specs in this pass.

## Corpus: `driving.sarec`

`tools/e2e/programs/driving.txt` (generated from a small timeline script; pad controls): the grove-walk
start, the BMX that waits next to CJ after the intro (ridden ~150 frames), HESOYAM/ASNAEB between blocks,
JQNTDMH (a car: CJ ends up driving the nearest car, a taxi, into the Grove Street walls; damage visible),
CQZIJMB (Bloodring Banger, driven), CPKTNWT (every car blows up and burns; a freight train passes), AKJJYGLC
(quad, driven into the overpass), OHDUDE (Hunter: spawned, CJ takes a nearby sedan instead), KGGGDKP
(Vortex, spawned). No boat (no water within reach of Grove Street). 4338 frames. Recorded on lane 7 with
`record-program.sh --replay` (replay == record IDENTICAL over 4338 frames); screenshots checked at the BMX
ride, the crashed taxi, the explosions and the quad crash. Stored as
`$SA_ROOT/target/e2e/corpus/driving.sarec`.

## Conventions and findings worth knowing

- Ported from the 1.0 US bytes (objdump incl. the `.HOODLUM` bodies); gta-reversed and
  `docs/re/specs/VEH/*.md` as reading aids. `expect` strings are whole instructions covering 5 bytes.
- `CDamageManager::FuckCarCompletely` writes the random wheel byte at `this + 5 - n` with
  n = ftol(frac * -3.0) = -5..0 (the original subtracts a non-positive index, so doors 0/1 can be hit;
  the door store right after overwrites them), consumes the panel rand calls, then clears lights and
  panels anyway. Kept. (The first build multiplied by 6.0: the split run caught it, 12 mismatches.)
- `CDamageManager::Get/SetDoorStatus_Component`: components 8/9/10/11 map to door bytes +0xC/+0xE/+0xB/
  +0xD (jump tables 0x6C2288 / 0x6C221C); `GetComponentGroup` gives group 5 for components 11..15 and
  0 for 16.. (`setge; dec; and 5`). Both were wrong in the first build; L2 caught them.
- `CDamageManager::ApplyDamage`: GetComponentGroup's out-parameters are the function's own argument
  slots (`car`'s slot receives the sub-index byte, so the dword passed to ProgressPanelDamage /
  ProgressDoorDamage / ProgressWheelDamage keeps the pointer's upper 3 bytes; reproduced). Group 0 skips
  SetLightStatus, group 5 sets the light before the panel.
- `CDamageManager::ProgressDoorDamage`: statuses above 4 are rewritten unchanged and return true.
- `CVehicle::QueryPickedUpEntityWithWinch` etc. call `CRopes::FindRope(this + 0x1D)` and index
  `CRopes::aRopes` without checking -1, as the original.
- `CVehicle::SetHasslePosId` with `set == false` toggles (`xor`) the bit instead of clearing it. Kept.
- `CAutomobile::...U32` door queries: component 8/9/10/11 -> door 3/5/2/4, 18 -> constant, others -> false.
- `CCarEnterExit::IsCarQuickJackPossible` always returns false (the final `xor al, al`); kept.
- `CHeli::InitHelis` clears only 2 of the 4 search lights; kept.
- `CVehicleRecording::IsPlaybackPausedForCar` always returns 0; kept.
- `CUpsideDownCarCheck::IsCarUpsideDown(vehicle)` does not negate the vtable +0xE4 result (gta-reversed
  does); the bytes win.
- `cHandlingDataMgr::HasFrontWheelDrive/HasRearWheelDrive` leave `(id * 0xE0)`'s upper bits in EAX;
  reproduced (returns u32).
- Return registers: most bool ports return `u8` (AL); originals that set only AL leave the caller's EAX upper
  bytes (base-class defaults `xor al, al`, `CTrain::FindSideStationIsOn`, ...). Callers test AL.
- `preserve`: `CCarEnterExit::ComputePassengerIndexFromCarDoor` (EDX), `CPlane::SetGearUp` (ECX).

## Verification (commands, results)

All game runs on lane 7 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=7 L2_PER_FRAME=1000`, under
`systemd-run --user --scope -p MemoryMax=8G`), outputs in the worktree's `target/e2e/lane-7/port-l2-veh-*`
(spools deleted after evaluation). Lists: `tools/e2e/lists/veh-all.txt` (213), `veh-l2.txt` (184, plain l2)
and `veh-split.txt` (29: rand consumers, world add/remove, frees, RW callbacks, ropes, HUD, static
initialisers, the anim-association writers; split mode).

- **L1**: none in this pass (no pure helper with a non-trivial input space was ported by itself; the
  damage-bit helpers are covered by L2).
- Tests: `cargo test -p sa-veh` (3 pass: list complete/unique, globals declared once across crates,
  initial values match the exe), `cargo test --release -p sa-hook --lib` 45 pass (incl.
  `every_hook_installs_on_the_exe`, `matches_catalogue_and_exe`, the callers test, the keycells and INF-20
  `data_reads` tests); clippy `-D warnings`: sa-veh host (all targets) and i686 (`--features in-process`),
  sa-oracle host; wasm32 build of sa-veh.

### Run results (lane 7, final build)

| run | tag | result |
|---|---|---|
| plain + l2 + rust, driving (4338 frames), veh-l2 list | `veh-d-l2` | (b) l2 == plain IDENTICAL, (c) rust == plain IDENTICAL, 17 subsystems; 71 hooks PASS (0 mismatches, 0 undeclared), 113 NOCALLS |
| plain + split + rust, driving, veh-split list | `veh-d-split` | splito == plain, splitr == plain IDENTICAL; split-report 20 PASS, 0 FAIL / DIVERGED / COUNT (the rest NOCASES); **(c) FAIL: the `rust` run (only the 29 split-list hooks on Rust) diverges at frame 2614 (vehicles; rand at 2628)** |
| all engine ports on Rust (`ports`, 1479 hooks incl. the 213 VEH ones) | `veh-{d,g,c,b}-ports` | IDENTICAL over driving 4338 / grove 5200 / world-cheats 3276 / c-bug 3220 frames, 17 subsystems, no crash / panic / install failure (the first world-cheats `rust` run faulted in Wine after reaching its end frame, before writing its trace; rerun IDENTICAL) |

The first build had three bugs that these runs caught: `GetDoorStatus_Component`/`SetDoorStatus_Component`
(jump-table order, 3 mismatches), `GetComponentGroup` (group 5/0 swapped, 1 mismatch), and
`FuckCarCompletely` (constant -3.0 read as 6.0, 12 split mismatches); the results above are after the fixes.

Verdict counts (213 hooks + 1 unhookable): **L2 90** (71 plain l2 + 19 split; 1 more split row below),
**L2-split with undeclared writes to review 1** (`CCarEnterExit::RemoveCarSitAnim`: 6 cases write the blend
associations, 0 mismatches), **implemented-needs-L2 122** (no call in the driving recording: mostly the
`CVehicle`/`CBike` virtual defaults that the class overrides hide, train/plane/heli code, the `...U32`
door queries, init functions run only at boot before the samples, ...), **ported-unhookable 1**
(`CBoat::PruneWakeTrail`). No L2 mismatch remains. L2 was run on the driving recording only; grove, cheats
and c-bug were run with every port on Rust (L3 evidence for the subsystem, not per-hook L2).

The split-list `rust` divergence (driving, frame 2614, right after CPKTNWT's explosions) is unexplained:
every split hook passes split-run comparison (splitr, where the Rust results drive the game, is identical),
and with *all* ports on Rust the run is identical. The difference is the mix: only those 29 VEH hooks on Rust
and every other port on the original. Candidates: `FuckCarCompletely` / `ApplyDamage` (called at frame
2612, caller 0x6B385D) leaving a register a caller uses (the callers test only checks ECX/EDX after direct
calls), or a Rust-only-with-original-neighbour interaction. Needs a bisection (per-hook rust runs); no row
claims L3.

## Divergence notes (ledger candidates)

- `cHandlingDataMgr::FindExactWord` (0x6F4F30): the original copies each table entry into a 20-byte stack
  buffer with `strncpy` and then compares; an entry longer than 20 characters overflows the buffer. The port
  compares such an entry in place (no stack overflow). The handling table's names are all shorter, so it is
  unreachable with the shipped data.
- Bool ports set only AL where the original leaves other EAX bits (listed above). Callers test AL.

## Open items

- `CBoat::PruneWakeTrail` (0x6F0E20): unhookable (its loop branch at 0x6F0E5F lands inside the patch
  range); the Rust function is kept.
- 3-byte functions cannot be hooked (installer `TooShort`): the `CVehicle` defaults above, `CTrain::UpdateTrains`,
  `ProcessTrainAnnouncements`, `PlayAnnouncement`, `OpenTrainDoor`, `AddPassenger`, `RemovePassenger`,
  `CHeli::SpecialHeliPreRender`, `CBoat::ProcessOpenDoor`, `CVehicleRecording::Render`, the `ret N`
  `CVehicle` virtual defaults. They need an exclusion decision (catalogue `excluded: trivial stub`?) or a
  hook at their callers.
- Skipped: `CDoor::ProcessImpact`/`Process`, `CBouncingPanel::ProcessPanel`, `cHandlingDataMgr::
  ConvertBikeDataToWorldUnits` (CRT x87 `asin` with its argument in ST0), the constructor/atexit initialisers
  (`__init_s_TestBladeCol*`, `__init_CVehicle__m_aSpecialColModel`, `__init_gHandlingDataMgr`), the
  `CTrain::GetDooorAngleOpenRatio*` pair, every ProcessControl / PreRender / Render / damage-effect driver,
  the constructors and destructors, the recording playback.
- `RemoveGetInAnims` / `RemoveCarSitAnim` write the blend associations they find (heap chain, not
  expressible as an extras line); they run in split mode.
- Hook-off (per-hook L3) runs.

## Functions

| address | name | state | evidence |
|---|---|---|---|
| 0x6A2460 | CAutomobile::ClearHeliOrientation | implemented-needs-L2 | no call in the recordings run |
| 0x6A2450 | CAutomobile::SetHeliOrientation | implemented-needs-L2 | no call in the recordings run |
| 0x6A3850 | CAutomobile::IsRoomForPedToLeaveCar | implemented-needs-L2 | no call in the recordings run |
| 0x6A44C0 | CAutomobile::KnockPedOutCar | implemented-needs-L2 | no call in the recordings run |
| 0x6A62A0 | CAutomobile::GetNumContactWheels | implemented-needs-L2 | no call in the recordings run |
| 0x6A2F70 | CAutomobile::GetAllWheelsOffGround | implemented-needs-L2 | no call in the recordings run |
| 0x6A62B0 | CAutomobile::GetHeightAboveRoad | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6A3760 | CAutomobile::SetAllTaxiLights | implemented-needs-L2 | no call in the recordings run |
| 0x6A3740 | CAutomobile::SetTaxiLight | implemented-needs-L2 | no call in the recordings run |
| 0x6A2250 | CAutomobile::IsComponentPresent | implemented-needs-L2 | no call in the recordings run |
| 0x6A29A0 | CAutomobile::ReduceHornCounter | implemented-needs-L2 | no call in the recordings run |
| 0x6A3030 | CAutomobile::RemoveRefsToVehicle | implemented-needs-L2 | no call in the recordings run |
| 0x6A2AE0 | CAutomobile::ResetSuspension | implemented-needs-L2 | no call in the recordings run |
| 0x6A2270 | CAutomobile::GetDooorAngleOpenRatio | L2 | L2 drv:39/39 PASS (0 mismatches, 0 undeclared) |
| 0x6A2310 | CAutomobile::IsDoorClosed | implemented-needs-L2 | no call in the recordings run |
| 0x6A2330 | CAutomobile::IsDoorMissing | L2 | L2 drv:396/396 PASS (0 mismatches, 0 undeclared) |
| 0x6A2290 | CAutomobile::IsDoorReady | implemented-needs-L2 | no call in the recordings run |
| 0x6A22D0 | CAutomobile::IsDoorFullyOpen | implemented-needs-L2 | no call in the recordings run |
| 0x6A6350 | CAutomobile::IsDoorReadyU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6A63E0 | CAutomobile::IsDoorFullyOpenU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6A6470 | CAutomobile::IsDoorClosedU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6A6500 | CAutomobile::IsDoorMissingU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6A62C0 | CAutomobile::GetDooorAngleOpenRatioU32 | L2 | L2 drv:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x6A2350 | CAutomobile::IsOpenTopCar | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6A6090 | CAutomobile::FindWheelWidth | L2 | L2 drv:770/770 PASS (0 mismatches, 0 undeclared) |
| 0x6A3580 | CAutomobile::FixTyre | implemented-needs-L2 | no call in the recordings run |
| 0x6A9CA0 | CAutomobile::Teleport | implemented-needs-L2 | no call in the recordings run |
| 0x6CDC90 | CQuadBike::GetRideAnimData | L2 | L2 drv:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x6C7D20 | CMonsterTruck::SetUpWheelColModel | implemented-needs-L2 | no call in the recordings run |
| 0x6C7D30 | CMonsterTruck::BurstTyre | implemented-needs-L2 | no call in the recordings run |
| 0x6B58A0 | CBike::GetNumContactWheels | implemented-needs-L2 | no call in the recordings run |
| 0x6B6790 | CBike::GetAllWheelsOffGround | implemented-needs-L2 | no call in the recordings run |
| 0x6B58B0 | CBike::GetHeightAboveRoad | L2 | L2 drv:33/33 PASS (0 mismatches, 0 undeclared) |
| 0x6B58C0 | CBike::GetRideAnimData | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6B58E0 | CBike::IsDoorReady | implemented-needs-L2 | no call in the recordings run |
| 0x6B58F0 | CBike::IsDoorFullyOpen | implemented-needs-L2 | no call in the recordings run |
| 0x6B5900 | CBike::IsDoorClosed | implemented-needs-L2 | no call in the recordings run |
| 0x6B5910 | CBike::IsDoorMissing | implemented-needs-L2 | no call in the recordings run |
| 0x6B5920 | CBike::IsDoorReadyU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6B5930 | CBike::IsDoorFullyOpenU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6B5940 | CBike::IsDoorClosedU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6B5950 | CBike::IsDoorMissingU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6B7270 | CBike::IsRoomForPedToLeaveCar | implemented-needs-L2 | no call in the recordings run |
| 0x6B5F40 | CBike::KnockOffRider | implemented-needs-L2 | no call in the recordings run |
| 0x6B8940 | CBike::FindWheelWidth | implemented-needs-L2 | no call in the recordings run |
| 0x6B59E0 | CBike::IsComponentPresent | implemented-needs-L2 | no call in the recordings run |
| 0x6B7130 | CBike::PlayHornIfNecessary | implemented-needs-L2 | no call in the recordings run |
| 0x6B5F90 | CBike::ReduceHornCounter | implemented-needs-L2 | no call in the recordings run |
| 0x6B7050 | CBike::Fix | implemented-needs-L2 | no call in the recordings run |
| 0x6B67B0 | CBike::RemoveRefsToVehicle | implemented-needs-L2 | no call in the recordings run |
| 0x6BF9B0 | CBmx::SetUpWheelColModel | implemented-needs-L2 | no call in the recordings run |
| 0x6BF9C0 | CBmx::BurstTyre | implemented-needs-L2 | no call in the recordings run |
| 0x6C0550 | CBmx::FindWheelWidth | L2 | L2 drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x6F0200 | CBoat::IsComponentPresent | implemented-needs-L2 | no call in the recordings run |
| 0x6F00D0 | CBoat::GetBoatAtomicObjectCB | implemented-needs-L2 | no call in the recordings run |
| 0x6F0DE0 | CBoat::ModifyHandlingValue | implemented-needs-L2 | no call in the recordings run |
| 0x6F10C0 | CBoat::CheckForSkippingCalculations | implemented-needs-L2 | no call in the recordings run |
| 0x459390 | CVehicleRecording::Init | L2 | L2 drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x45A1B0 | CVehicleRecording::InitAtStartOfGame | L2 | L2 drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x459FF0 | CVehicleRecording::FindIndexWithFileNameNumber | implemented-needs-L2 | no call in the recordings run |
| 0x45A060 | CVehicleRecording::HasRecordingFileBeenLoaded | implemented-needs-L2 | no call in the recordings run |
| 0x4595A0 | CVehicleRecording::IsPlaybackPausedForCar | implemented-needs-L2 | no call in the recordings run |
| 0x4639E0 | CStuckCarCheck::Init | L2 | L2 drv:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x463970 | CStuckCarCheck::ResetArrayElement | implemented-needs-L2 | no call in the recordings run |
| 0x463C40 | CStuckCarCheck::ClearStuckFlagForCar | implemented-needs-L2 | no call in the recordings run |
| 0x463C00 | CStuckCarCheck::HasCarBeenStuckForAWhile | implemented-needs-L2 | no call in the recordings run |
| 0x463C70 | CStuckCarCheck::IsCarInStuckCarArray | implemented-needs-L2 | no call in the recordings run |
| 0x463830 | CUpsideDownCarCheck::IsCarUpsideDown-vehicle | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x4655E0 | CUpsideDownCarCheck::UpdateTimers | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x4638D0 | CUpsideDownCarCheck::AddCarToCheck | implemented-needs-L2 | no call in the recordings run |
| 0x463910 | CUpsideDownCarCheck::RemoveCarFromCheck | implemented-needs-L2 | no call in the recordings run |
| 0x463940 | CUpsideDownCarCheck::HasCarBeenUpsideDownForAWhile | implemented-needs-L2 | no call in the recordings run |
| 0x572040 | CCurrentVehicle::Process | L2 | split drv:1000/1000 PASS (0 mismatches, 0 undeclared) |
| 0x5BD420 | CVehicleAnimGroup::CVehicleAnimGroup | L2 | L2 drv:60/60 PASS (0 mismatches, 0 undeclared) |
| 0x6A0520 | CDamageManager::Init | implemented-needs-L2 | no call in the recordings run |
| 0x6C20E0 | CDamageManager::ResetDamageStatus | L2 | L2 drv:83/83 PASS (0 mismatches, 0 undeclared) |
| 0x6A04E0 | CDamageManager::ResetDamageStatusAndWheelDamage | implemented-needs-L2 | no call in the recordings run |
| 0x6C22C0 | CDamageManager::GetEngineStatus | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6C22A0 | CDamageManager::SetEngineStatus | L2 | L2 drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x6C23B0 | CDamageManager::ProgressEngineDamage | implemented-needs-L2 | no call in the recordings run |
| 0x6C21B0 | CDamageManager::GetWheelStatus | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6C21A0 | CDamageManager::SetWheelStatus | L2 | L2 drv:4/4 PASS (0 mismatches, 0 undeclared) |
| 0x6C2440 | CDamageManager::ProgressWheelDamage | implemented-needs-L2 | no call in the recordings run |
| 0x6C2230 | CDamageManager::GetDoorStatus | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6C21C0 | CDamageManager::SetDoorStatus- | L2 | L2 drv:26/26 PASS (0 mismatches, 0 undeclared) |
| 0x6C2250 | CDamageManager::GetDoorStatus_Component | L2 | L2 drv:47/47 PASS (0 mismatches, 0 undeclared) |
| 0x6C21E0 | CDamageManager::SetDoorStatus_Component | L2 | L2 drv:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x6C2130 | CDamageManager::GetLightStatus | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6C2100 | CDamageManager::SetLightStatus | implemented-needs-L2 | no call in the recordings run |
| 0x6C2180 | CDamageManager::GetPanelStatus | L2 | L2 drv:236/236 PASS (0 mismatches, 0 undeclared) |
| 0x6C2150 | CDamageManager::SetPanelStatus | L2 | L2 drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x6C2300 | CDamageManager::GetAeroplaneCompStatus | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6C22D0 | CDamageManager::SetAeroplaneCompStatus | implemented-needs-L2 | no call in the recordings run |
| 0x6C2460 | CDamageManager::ProgressAeroplaneDamage | implemented-needs-L2 | no call in the recordings run |
| 0x6C23C0 | CDamageManager::ProgressPanelDamage | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x6C2320 | CDamageManager::ProgressDoorDamage | L2 | split drv:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x6C2040 | CDamageManager::GetComponentGroup | L2 | L2 drv:4/4 PASS (0 mismatches, 0 undeclared) |
| 0x6C25D0 | CDamageManager::FuckCarCompletely | L2 | split drv:13/13 PASS (0 mismatches, 0 undeclared) |
| 0x6C24B0 | CDamageManager::ApplyDamage | L2 | split drv:4/4 PASS (0 mismatches, 0 undeclared) |
| 0x6F4790 | CDoor::Open | L2 | L2 drv:245/245 PASS (0 mismatches, 0 undeclared) |
| 0x6F47E0 | CDoor::GetAngleOpenRatio | implemented-needs-L2 | no call in the recordings run |
| 0x6F4800 | CDoor::IsClosed | L2 | L2 drv:242/242 PASS (0 mismatches, 0 undeclared) |
| 0x6F4820 | CDoor::IsFullyOpen | implemented-needs-L2 | no call in the recordings run |
| 0x6F4910 | CBouncingPanel::ResetPanel | implemented-needs-L2 | no call in the recordings run |
| 0x6F4920 | CBouncingPanel::SetPanel | L2 | L2 drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x64EEC0 | CCarEnterExit::IsVehicleHealthy | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x64EEE0 | CCarEnterExit::IsPedHealthy | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x64ED90 | CCarEnterExit::IsCarDoorReady | L2 | L2 drv:6/6 PASS (0 mismatches, 0 undeclared) |
| 0x64EE10 | CCarEnterExit::CarHasDoorToClose | L2 | L2 drv:45/45 PASS (0 mismatches, 0 undeclared) |
| 0x64EDD0 | CCarEnterExit::CarHasDoorToOpen | L2 | L2 drv:9/9 PASS (0 mismatches, 0 undeclared) |
| 0x64EE70 | CCarEnterExit::CarHasPartiallyOpenDoor | L2 | L2 drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x64EE50 | CCarEnterExit::CarHasOpenableDoor | L2 | L2 drv:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x64EF00 | CCarEnterExit::IsCarQuickJackPossible | L2 | L2 drv:4/4 PASS (0 mismatches, 0 undeclared) |
| 0x64F190 | CCarEnterExit::ComputeTargetDoorToEnterAsPassenger | L2 | L2 drv:44/44 PASS (0 mismatches, 0 undeclared) |
| 0x64F1E0 | CCarEnterExit::ComputePassengerIndexFromCarDoor | L2 | L2 drv:31/31 PASS (0 mismatches, 0 undeclared) |
| 0x64E550 | CCarEnterExit::ComputeDoorFlag | L2 | L2 drv:11/11 PASS (0 mismatches, 0 undeclared) |
| 0x64E610 | CCarEnterExit::ComputeOppositeDoorFlag | implemented-needs-L2 | no call in the recordings run |
| 0x64F110 | CCarEnterExit::ComputeTargetDoorToExit | L2 | L2 drv:59/59 PASS (0 mismatches, 0 undeclared) |
| 0x64EF70 | CCarEnterExit::IsCarSlowJackRequired | L2 | L2 drv:5/5 PASS (0 mismatches, 0 undeclared) |
| 0x64F070 | CCarEnterExit::ComputeSlowJackedPed | L2 | L2 drv:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x64EC90 | CCarEnterExit::IsCarDoorInUse | L2 | L2 drv:6/6 PASS (0 mismatches, 0 undeclared) |
| 0x64F6E0 | CCarEnterExit::RemoveGetInAnims | L2 | split drv:48/48 PASS (0 mismatches, 0 undeclared) |
| 0x64F680 | CCarEnterExit::RemoveCarSitAnim | L2-split (undeclared writes to review) | split drv:10/10 PASS (0 mismatches, 6 undeclared) |
| 0x6A0480 | cHandlingDataMgr::HasFrontWheelDrive | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6A04B0 | cHandlingDataMgr::HasRearWheelDrive | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6F5300 | cHandlingDataMgr::GetBoatPointer | L2 | L2 drv:12/12 PASS (0 mismatches, 0 undeclared) |
| 0x6F52D0 | cHandlingDataMgr::GetFlyingPointer | L2 | L2 drv:90/90 PASS (0 mismatches, 0 undeclared) |
| 0x6F5290 | cHandlingDataMgr::ConvertBikeDataToGameUnits | L2 | L2 drv:13/13 PASS (0 mismatches, 0 undeclared) |
| 0x6F4FD0 | cHandlingDataMgr::GetHandlingId | L2 | L2 drv:212/212 PASS (0 mismatches, 0 undeclared) |
| 0x6F4F30 | cHandlingDataMgr::FindExactWord | L2 | L2 drv:259/259 PASS (0 mismatches, 0 undeclared) |
| 0x6F5010 | cHandlingDataMgr::ConvertDataToWorldUnits | implemented-needs-L2 | no call in the recordings run |
| 0x5BF3D0 | cHandlingDataMgr::Initialise | implemented-needs-L2 | no call in the recordings run |
| 0x6C4560 | CHeli::InitHelis | L2 | L2 drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x6C45B0 | CHeli::AddHeliSearchLight | implemented-needs-L2 | no call in the recordings run |
| 0x6C4650 | CHeli::Pre_SearchLightCone | implemented-needs-L2 | no call in the recordings run |
| 0x6C46E0 | CHeli::Post_SearchLightCone | implemented-needs-L2 | no call in the recordings run |
| 0x6C4800 | CHeli::SwitchPoliceHelis | implemented-needs-L2 | no call in the recordings run |
| 0x6C4320 | CHeli::SetUpWheelColModel | implemented-needs-L2 | no call in the recordings run |
| 0x6C4330 | CHeli::BurstTyre | implemented-needs-L2 | no call in the recordings run |
| 0x6C4550 | CHeli::SetupDamageAfterLoad | implemented-needs-L2 | no call in the recordings run |
| 0x6C4530 | CHeli::Fix | implemented-needs-L2 | no call in the recordings run |
| 0x6C9140 | CPlane::SetUpWheelColModel | implemented-needs-L2 | no call in the recordings run |
| 0x6C9150 | CPlane::BurstTyre | implemented-needs-L2 | no call in the recordings run |
| 0x6CAC10 | CPlane::SetupDamageAfterLoad | implemented-needs-L2 | no call in the recordings run |
| 0x6CAB90 | CPlane::IsAlreadyFlying | implemented-needs-L2 | no call in the recordings run |
| 0x6CAD90 | CPlane::InitPlaneGenerationAndRemoval | implemented-needs-L2 | no call in the recordings run |
| 0x6CAC20 | CPlane::SetGearUp | implemented-needs-L2 | no call in the recordings run |
| 0x6CAC70 | CPlane::SetGearDown | implemented-needs-L2 | no call in the recordings run |
| 0x6CCA50 | CPlane::CountPlanesAndHelis | L2 | L2 drv:127/127 PASS (0 mismatches, 0 undeclared) |
| 0x84A300 | __init_g_97f634 | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x84A340 | __init_g_97f63c | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x853500 | __init_g_c1bfec | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x853600 | __init_g_c1c818 | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x853620 | __init_g_c1c81c | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x853C20 | __init_g_c1cb28 | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x853CE0 | __init_g_c1cb54 | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x8540B0 | __init_fTimeMult | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x854390 | __init_g_c2b9bc | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x854410 | __init_g_c3804c | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x854350 | __init_g_c2b9b4 | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x853540 | __init_g_c1c208 | L2 | split drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x6F5DB0 | CTrain::DisableRandomTrains | L2 | L2 drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x6F5CD0 | CTrain::FindNumCarriagesPulled | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6F5DC0 | CTrain::RemoveOneMissionTrain | implemented-needs-L2 | no call in the recordings run |
| 0x6F5DF0 | CTrain::ReleaseOneMissionTrain | implemented-needs-L2 | no call in the recordings run |
| 0x6F5E20 | CTrain::SetTrainSpeed | L2 | L2 drv:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x6F5E50 | CTrain::SetTrainCruiseSpeed | implemented-needs-L2 | no call in the recordings run |
| 0x6F5E70 | CTrain::FindCaboose | implemented-needs-L2 | no call in the recordings run |
| 0x6F5E90 | CTrain::FindEngine | implemented-needs-L2 | no call in the recordings run |
| 0x6F5EB0 | CTrain::FindCarriage | implemented-needs-L2 | no call in the recordings run |
| 0x6F5EF0 | CTrain::FindSideStationIsOn | implemented-needs-L2 | no call in the recordings run |
| 0x6F5EE0 | CTrain::IsRoomForPedToLeaveCar | implemented-needs-L2 | no call in the recordings run |
| 0x6B5830 | CVehicle::GetDooorAngleOpenRatioU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6D56B0 | CVehicle::GetDooorAngleOpenRatio | implemented-needs-L2 | no call in the recordings run |
| 0x6D3D50 | CVehicle::FindWheelWidth | implemented-needs-L2 | no call in the recordings run |
| 0x6B5880 | CVehicle::SetTowLink | implemented-needs-L2 | no call in the recordings run |
| 0x6D62A0 | CVehicle::IsComponentPresent | implemented-needs-L2 | no call in the recordings run |
| 0x6D62B0 | CVehicle::IsDoorReady | implemented-needs-L2 | no call in the recordings run |
| 0x6D62C0 | CVehicle::IsDoorFullyOpen | implemented-needs-L2 | no call in the recordings run |
| 0x6D62D0 | CVehicle::IsDoorClosed | implemented-needs-L2 | no call in the recordings run |
| 0x6D62E0 | CVehicle::IsDoorMissing | implemented-needs-L2 | no call in the recordings run |
| 0x6D62F0 | CVehicle::IsDoorReadyU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6D6300 | CVehicle::IsDoorFullyOpenU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6D6310 | CVehicle::IsDoorClosedU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6D6320 | CVehicle::IsDoorMissingU32 | implemented-needs-L2 | no call in the recordings run |
| 0x6D6350 | CVehicle::SetUpWheelColModel | implemented-needs-L2 | no call in the recordings run |
| 0x6D6360 | CVehicle::BurstTyre | implemented-needs-L2 | no call in the recordings run |
| 0x6D6370 | CVehicle::IsRoomForPedToLeaveCar | implemented-needs-L2 | no call in the recordings run |
| 0x6D63D0 | CVehicle::GetNumContactWheels | implemented-needs-L2 | no call in the recordings run |
| 0x6D63F0 | CVehicle::GetHeightAboveRoad | L2 | L2 drv:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x6B5860 | CVehicle::AddMovingCollisionSpeed | implemented-needs-L2 | no call in the recordings run |
| 0x6D3000 | CVehicle::SetGettingInFlags | L2 | L2 drv:7/7 PASS (0 mismatches, 0 undeclared) |
| 0x6D3020 | CVehicle::SetGettingOutFlags | L2 | L2 drv:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x6D3040 | CVehicle::ClearGettingInFlags | L2 | L2 drv:11/11 PASS (0 mismatches, 0 undeclared) |
| 0x6D3060 | CVehicle::ClearGettingOutFlags | L2 | L2 drv:4/4 PASS (0 mismatches, 0 undeclared) |
| 0x6D3B60 | CVehicle::InitWinch | implemented-needs-L2 | no call in the recordings run |
| 0x6D3B30 | CVehicle::SetHasslePosId | implemented-needs-L2 | no call in the recordings run |
| 0x6D2950 | CVehicle::SetCompAlphaCB | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6D2960 | CVehicle::SetComponentAtomicAlpha | L2 | split drv:1000/1000 PASS (0 mismatches, 0 undeclared) |
| 0x6D1C40 | CVehicle::IsDriver-Ped | L2 | L2 drv:385/385 PASS (0 mismatches, 0 undeclared) |
| 0x6D1C60 | CVehicle::IsDriver-ModelID | implemented-needs-L2 | no call in the recordings run |
| 0x6D2370 | CVehicle::IsLawEnforcementVehicle | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6D1E60 | CVehicle::CanDoorsBeDamaged | L2 | L2 drv:79/79 PASS (0 mismatches, 0 undeclared) |
| 0x6D0CA0 | CVehicle::SetCollisionLighting | L2 | L2 drv:19/19 PASS (0 mismatches, 0 undeclared) |
| 0x6D0CC0 | CVehicle::UpdateLightingFromStoredPolys | L2 | L2 drv:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6D1150 | CVehicle::CustomCarPlate_TextureDestroy | implemented-needs-L2 | no call in the recordings run |
| 0x6D3CF0 | CVehicle::QueryPickedUpEntityWithWinch | implemented-needs-L2 | no call in the recordings run |
| 0x6D3D10 | CVehicle::GetRopeHeightForHeli | implemented-needs-L2 | no call in the recordings run |
| 0x6D3D30 | CVehicle::SetRopeHeightForHeli | implemented-needs-L2 | no call in the recordings run |
| 0x6D3CB0 | CVehicle::ReleasePickedUpEntityWithWinch | implemented-needs-L2 | no call in the recordings run |
| 0x6D3CD0 | CVehicle::PickUpEntityWithWinch | implemented-needs-L2 | no call in the recordings run |
| 0x6D3630 | CVehicle::RemoveUpgrade | implemented-needs-L2 | no call in the recordings run |
| 0x6D0B40 | CVehicle::Shutdown | implemented-needs-L2 | no call in the recordings run |
| 0x6F0E20 | CBoat::PruneWakeTrail | ported-unhookable | its loop branch at 0x6F0E5F lands inside the patch range; `boat::prune_wake_trail` |
