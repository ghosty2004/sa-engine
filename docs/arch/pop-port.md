# PORT-POP: subsystem POP, first pass (crate `sa-pop`)

First pass over the catalogue subsystem POP (430 rows: `CCarCtrl` 110, `CPathFind` 72, `CPopulation` 66,
`CGangWars` 35, `CCarAI` 25, `CTagManager` 19, `CDarkel` 19, `CPopCycle` 15, `CTrafficLights` 12, ...).
This pass ports the leaf and near-leaf functions, chosen from `catalog/samp-priority.csv` (POP rows executed
under SA:MP) and the corpus coverage (`target/e2e/lane-3/inf10/cov-pause-restart.trace.cov.csv`), smallest
first. Follows ADR-001 and the PORT-PHYS / PORT-WORLD pattern. The main session sets `catalog/state.csv`;
the per-function verdicts are in `docs/arch/pop-port-verdicts.tsv` and the table at the end.

Ported: **74 functions** (73 `#[hook]` registrations + `CCarAI::CarHasReasonToStop`, ported but
unhookable). `CPathFind` (72 rows) is not touched: it is a separate pass (node streaming, interiors).
`CPedType` is catalogued in PED, not POP, and is not part of this pass.

## Crate

`crates/sa-pop` (PLAN §2 names it). Same rules as sa-world: `#![no_std]`, `#![deny(unsafe_code)]`, game
state through `globals!`/`GPtr`, every other catalogued function through `orig!` (so its toggle governs
calls from Rust), every port registered with `#[hook]` and listed in its module's `HOOKS`
(`sa_pop::hooks::ALL`, appended by sa-hook after `sa_world::hooks::ALL`). Builds for host, i686
(`--features in-process`) and wasm32. Depends on sa-core, sa-math, sa-stream (CLoadedCarGroup, streaming
globals, `ModelIndices`), sa-world (zones, cheats, `FindPlayer*`).

| file | contents |
|---|---|
| `common.rs` | sa-world's field helpers re-exported, `rand_at(site)` (rand with its original return address), `rand_frac`, x87 compare helpers `jp5` / `jne_mask`, `model_info` |
| `popcycle.rs` | `CPopCycle`: globals (`m_nPercTypeGroup`, `m_nPercOther`, the current zone/time, dodgyness, dealer probabilities 0x8D24F0) and 8 ports |
| `population.rs` | `CPopulation`: globals (translation array 0x8D2540 = 33 groups x 3 zones, car/ped groups, loaded car groups incl. the 11 gang groups, counters, flags) and 11 ports |
| `carctrl.rs` | `CCarCtrl` / `CCarAI`: vehicles of interest, sequences, counters, Init/ReInit, ChooseCarModelToLoad (14 hooks + 1 unhookable) |
| `cargen.rs` | `CCarGenerator` / `CTheCarGenerators`: SwitchOn/Off, Setup, CreateCarGenerator, RemoveCarGenerators, Init |
| `gangs.rs` | `CGangs::Initialise`, `ChooseGangPedModel`, 6 `CGangWars` leaves |
| `misc.rs` | `CDarkel` (4), `CTagManager` (6), `CTrafficLights` (7), `CRoadBlocks::ClearScriptRoadBlocks`, `CAccidentManager` (2) |
| `statics.rs` | 6 POP static initialisers (`__init_*`) |

Shared-crate changes: sa-hook `table.rs` (sa-pop's hooks appended), `callers.rs` test and sa-oracle
`hook_preserve` chain sa-pop's registrations; sa-oracle `specs_pop.rs` (3 L1 specs). No harness core
change.

## Conventions and findings worth knowing

- Ported from the 1.0 US bytes (objdump of the exe incl. the `.HOODLUM` bodies); gta-reversed and the
  RE-SPEC-3 specs (`docs/re/specs/POP/*.md`) only as reading aids. Every `expect` string was regenerated
  from the exe (whole instructions covering 5 bytes).
- `rand()` calls name their original return address (`orig::at_site`), including `.HOODLUM` ones
  (`InitSequence` 0x15616EE / 0x1561700).
- `CPopCycle::PickPedMIToStreamInForCurrentZone` stops the group walk at `r < p[g]`;
  `PickARandomGroupOfOtherPeds` at `p[g] >= r` (spec CPopCycle.md; both kept).
- `CPopCycle::UpdateDealerStrengths` runs once per **minute** of `CTimer` (`time / 60000` via the magic
  multiply 0x45E7B273 >> 46), not per 14 s.
- `CPopulation::m_TranslationArray` is 33 x 3 int32 (0x8D2540..0x8D26CC): the 18 popcycle groups then
  the gangs (`CGangs::ChooseGangPedModel` reads from +0xD8).
- `CGangs::ChooseGangPedModel`: with the override byte set (!= 0xFF) it returns member **0** of the gang
  group in the current world zone (the override value itself is not used); the random sequence is sized
  with world zone 0 and looked up in the current zone (gr's NOTE).
- `CCarCtrl::Init` / `ReInit` clear **11** `m_LoadedGangCars` groups (0xC0EA28 stride 0x2E up to 0xC0EC22),
  then appropriate, inappropriate, boats (the "CLoadedCarGroup clear loop 11 entries" deviation).
  `ReInit` does not reset `LastTimeAmbulanceCreated` / `LastTimeFireTruckCreated` (0x9690B0/B4) nor
  `bCarsGeneratedAroundCamera`; `Init` does not reset `LastTimeLawEnforcerCreated` (0x9690B8).
- `CGangs::Initialise` leaves gang 3's weapons untouched (as the bytes).
- `CCarAI::CarHasReasonToStop` (stub 0x41C050) is ported but **unhookable**: protection code reads the stub
  bytes as data (`lea reg, [reg*4 + 0x41C050]` at 0x14B2FB5, 0x14B2FC3, 0x14B5ED1, 0x14B5EDF; sa-hook
  `every_hook_installs_on_the_exe` rejected it).
- `CPopulation::ChoosePolicePedOccupation` (0x610F40, `xor eax,eax; ret`) and `ChooseGangOccupation`
  (0x611550, a lone `jmp CGangs::ChooseGangPedModel`) are not hooked (3 bytes / an alias).
- Return registers: bool ports return `u8` (AL). A few originals leave other bits in EAX (`IsTag`'s
  `xor al,al` path, `LightForCars*_Visual`, `PlaceGangMembers`); callers only test AL (L2 compares AL).
- `this` declared for `CCarGenerator` (0x20) and `CAccidentManager` (0x80) thiscall ports.

## Verification (commands, results)

All game runs on lane 1 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=1 L2_PER_FRAME=1000`, under
`systemd-run --user --scope -p MemoryMax=8G`), outputs in the worktree's `target/e2e/lane-1/port-l2-pop-*`.
Lists: `tools/e2e/lists/pop-all.txt` (73), `pop-l2.txt` (54, plain l2) and `pop-split.txt` (19: rand
consumers, allocation, deletion, references, file loading, HUD, static initialisers; split mode).

- **L1** (`SA_WINEPREFIX=/nonexistent sa-oracle l1 <key> --cases 1000000 --no-fixtures --threads 4`,
  `crates/sa-oracle/src/specs_pop.rs`): `PopFindTrafficLightType` 1,000,058 cases, 0 mismatches
  (INCOMPLETE only by its signalling-NaN cases, unverified with the native backend off, as COLL/PHYS);
  `PopFindSpeedMultiplierWithSpeedFromNodes` 1,002,000 PASS; `PopFindPedRaceFromName` 1,002,000 PASS.
  `cargo test --release -p sa-oracle --test l1`: 16 pass.
- **L2 / split / L3**: see "Run results" below.
- Tests: `cargo test -p sa-pop` (3 pass: list complete/unique, globals declared once across crates, initial
  values match the exe), `cargo test --release -p sa-hook --lib` 45 pass (incl.
  `every_hook_installs_on_the_exe`, `matches_catalogue_and_exe`, `callers_reading_ecx_edx_after_a_hook_are_declared`,
  the keycells tests `readers_match_the_exe` / `every_code_key_read_of_a_hooked_range_is_redirected`, and the
  INF-20 `data_reads` tests); clippy `-D warnings`: sa-pop host (all targets), i686 (`--features in-process`),
  wasm32; sa-oracle host (all targets); sa-hook i686.
- **L2 extra regions** (`tools/e2e/l2-extras.txt`, section `# PORT-POP`), each from the first grove run
  (`port-l2-pop-g-l2`, first build): `UpdatePedCount` (ped +0x474 and the counters: without the ped flag
  the Rust run saw the flag the original had just set, 22 mismatches), `ClearSpecificZonesToTriggerGangWar`
  (zone-info colours via FillZonesWithGangColours), `CTheCarGenerators::{Create,Remove,Init}` (generator
  array, plate handler), `CTagManager::AddTag` (tag array), `CPopCycle::Update` (UpdatePercentages' floats).

### Run results (lane 1, final build)

| run | tag | result |
|---|---|---|
| plain l2 + rust, grove walk (5200 frames) | `pop-g-l2` | (b) l2 == plain, (c) rust == plain IDENTICAL, 17 subsystems; every hook PASS except `CPopCycle::Update` (UpdatePercentages' floats, declared, rerun `pop-g-fix` PASS 1000/1001) |
| plain l2 + rust, pause-restart (3700) | `pop-p-l2` | (b), (c) IDENTICAL; `Update` (m_pCurrZone out-parameter) and `CCarCtrl::ReInit` (loaded car groups via Clear) undeclared, declared, rerun `pop-p-fix`: both PASS |
| plain l2 + rust, world-cheats (3276) | `pop-c-l2` | (b), (c) IDENTICAL; `Update` as above, rerun `pop-c-fix` PASS 1000/1001 |
| split + rust, grove / pr / cheats | `pop-{g,p,c}-split` | splito == plain, splitr == plain, rust == plain on every frame; split-report 17 / 17 / 19 PASS, 0 FAIL / DIVERGED (the rest NOCASES) |
| all engine ports on Rust (`ports`, 1266 hooks incl. the 73 POP ones) | `pop-{g,p,c}-ports` | IDENTICAL over 5200 / 3700 / 3276 frames, 17 subsystems, no crash / panic / install failure |

The first grove run (first build, before the extras) is `pop-g-l2` of the first build (overwritten); its
findings are the extras lines above. L3: every POP hook on Rust together is frame-identical on all three
recordings (the `rust` runs of the three list runs and the `ports` runs). Per-hook hook-off runs were not
made, so no row claims L3.

Verdict counts (73 hooks + 1 unhookable): **L1 3** (also L2 PASS), **L2 58**, **L2-split with undeclared
writes to review 9**, **implemented-needs-L2 3** (no call in any recording: `CCarGenerator::Setup` is only
reached through `CreateCarGenerator`, whose Rust port calls it through `orig!`, so it ran in the `rust`
runs; `CDarkel::ThisVehicleShouldBeKilledForFrenzy`; `CCarCtrl::RegisterVehicleOfInterest`), **ported-
unhookable 1** (`CCarAI::CarHasReasonToStop`). No mismatch anywhere.

## Divergence notes (ledger candidates)

- `CTrafficLights::FindOrientationForTrafficLightType` (0x49D520): when the entity has no matrix twice in a
  row (allocation failed the first time), the original reads `[edi+0x14]` with `edi` = the first, null
  matrix pointer (crash); the port reads the reallocated matrix. Unreachable in practice.
- `CPopulation::PlaceGangMembers` (0x6117D0): the original passes a 4-byte uninitialised local as the
  `CPedGroupPlacer` `this`; the port passes 4 zero bytes (`PlaceGroup` does not read it per the split runs:
  0 mismatches).
- Bool ports set only AL; the original leaves other EAX bits in `IsTag` (type != 1 path),
  `LightForCars*_Visual` (riot path) and `PlaceGangMembers` (callee's EAX). Callers test AL.

## Open items

- The 9 `L2-split (undeclared writes to review)` rows: `RemovePed` (the deleted ped and pool), `PlaceGangMembers`
  (created peds), `CPopulation::Initialise` (the loaded group tables), `PickPedMIToStreamInForCurrentZone`
  (`ms_NextPedToLoadFromGroup[g]`, computed index), `ResetOnPlayerDeath` (HUD / weapons), `CCarCtrl::Init`
  (the grove split run predates its extras line; pr/cheats 0 undeclared), the three node-array static
  initialisers (computed addresses). Declaring the regions is a follow-up; none had a mismatch.
- Not ported in this pass (≈355 POP rows): the `CPathFind` class (72), the large drivers (`CPopulation::Update`,
  `ManagePopulation`, `AddPed*`, `GeneratePedsAtAttractors`, `ConvertTo*Object`, the `CCarCtrl` generation
  and steering code incl. `GenerateRandomCars`, `RemoveDistantCars`, `ScanForPedDanger`, the `CCarAI` mission
  logic, `CGangWars::Update`/`StartOffensiveGangWar`, `CRoadBlocks::GenerateRoadBlocks`, `CTheCarGenerators::Process`,
  `CDarkel::Update`/`DrawMessages`, `CPopCycle::Initialise`/`UpdatePercentages`, the file loaders). The
  RE-SPEC-3 deviation examples for those (renormalised rows, 2D distance + downward pool walk, 4/2 mission
  ids, stuck interval, bTrainingMission test, CanBeDeleted) are not yet exercised by a port.
- `statics.rs` writes its globals through `g()` (direct addresses, as sa-world's foreign globals): the
  CPathFind node arrays and two f32 belong to the CPathFind / CTagManager-area pass, which should own them.
- Hook-off (per-hook L3) runs.

## Functions

| address | name | state | evidence |
|---|---|---|---|
| 0x6F2E30 | CCarGenerator::SwitchOff | L2 | L2 grove:133/133 PASS (0 mismatches, 0 undeclared); L2 pr:266/266 PASS (0 mismatches, 0 undeclared); L2 cheats:133/133 PASS (0 mismatches, 0 undeclared) |
| 0x6F32C0 | CCarGenerator::SwitchOn | L2 | L2 grove:182/182 PASS (0 mismatches, 0 undeclared); L2 pr:317/317 PASS (0 mismatches, 0 undeclared); L2 cheats:179/179 PASS (0 mismatches, 0 undeclared) |
| 0x6F2E50 | CCarGenerator::Setup | implemented-needs-L2 | no call in grove walk / pause-restart / cheats |
| 0x6F31A0 | CTheCarGenerators::CreateCarGenerator | L2 | L2 grove:318/318 PASS (0 mismatches, 0 undeclared); L2 pr:589/589 PASS (0 mismatches, 0 undeclared); L2 cheats:315/315 PASS (0 mismatches, 0 undeclared) |
| 0x6F3240 | CTheCarGenerators::RemoveCarGenerators | L2 | L2 grove:216/216 PASS (0 mismatches, 0 undeclared); L2 pr:226/226 PASS (0 mismatches, 0 undeclared); L2 cheats:210/210 PASS (0 mismatches, 0 undeclared) |
| 0x6F3270 | CTheCarGenerators::Init | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:2/2 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x611B60 | CPopulation::FindCarMultiplierMotorway | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x610ED0 | CPopulation::FindDummyDistForModel | L2 | L2 grove:207/207 PASS (0 mismatches, 0 undeclared); L2 pr:117/117 PASS (0 mismatches, 0 undeclared); L2 cheats:88/88 PASS (0 mismatches, 0 undeclared) |
| 0x611760 | CPopulation::IsSunbather-ModelID | L2 | L2 grove:77/77 PASS (0 mismatches, 0 undeclared); L2 pr:49/49 PASS (0 mismatches, 0 undeclared); L2 cheats:134/134 PASS (0 mismatches, 0 undeclared) |
| 0x406F50 | CPopulation::DoesCarGroupHaveModelId | L2 | L2 grove:466/466 PASS (0 mismatches, 0 undeclared); L2 pr:157/157 PASS (0 mismatches, 0 undeclared); L2 cheats:144/144 PASS (0 mismatches, 0 undeclared) |
| 0x5B6D40 | CPopulation::FindPedRaceFromName | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:276/276 PASS (0 mismatches, 0 undeclared); L2 pr:276/276 PASS (0 mismatches, 0 undeclared); L2 cheats:276/276 PASS (0 mismatches, 0 undeclared) |
| 0x614490 | CPopulation::PickGangCar | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x610F20 | CPopulation::RemovePed | L2-split (undeclared writes to review) | split grove:187/187 PASS (0 mismatches, 187 undeclared); split pr:48/48 PASS (0 mismatches, 48 undeclared); split cheats:95/95 PASS (0 mismatches, 95 undeclared) |
| 0x6117D0 | CPopulation::PlaceGangMembers | L2-split (undeclared writes to review) | split grove:17/17 PASS (0 mismatches, 11 undeclared); split pr:16/16 PASS (0 mismatches, 15 undeclared); split cheats:11/11 PASS (0 mismatches, 10 undeclared) |
| 0x610E10 | CPopulation::Initialise | L2-split (undeclared writes to review) | split grove:1/1 PASS (0 mismatches, 1 undeclared); split pr:2/2 PASS (0 mismatches, 2 undeclared); split cheats:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x611570 | CPopulation::UpdatePedCount | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6116C0 | CPopulation::PedCreationDistMultiplier | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x84B350 | __init_g_a9a8b8 | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x84B370 | __init_g_a9a8bc | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x8540D0 | __init_g_c2b950 | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x84A040 | __init_aNodesToBeCleared | L2-split (undeclared writes to review) | split grove:1/1 PASS (0 mismatches, 1 undeclared); split pr:1/1 PASS (0 mismatches, 1 undeclared); split cheats:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x84A080 | __init_aExteriorNodeLinkedTo | L2-split (undeclared writes to review) | split grove:1/1 PASS (0 mismatches, 1 undeclared); split pr:1/1 PASS (0 mismatches, 1 undeclared); split cheats:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x84A640 | __init_CRoadBlocks__RoadBlockNodes | L2-split (undeclared writes to review) | split grove:1/1 PASS (0 mismatches, 1 undeclared); split pr:1/1 PASS (0 mismatches, 1 undeclared); split cheats:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x610210 | CPopCycle::IsPedInGroup | L2 | L2 grove:395/395 PASS (0 mismatches, 0 undeclared); L2 pr:183/183 PASS (0 mismatches, 0 undeclared); L2 cheats:210/210 PASS (0 mismatches, 0 undeclared) |
| 0x610310 | CPopCycle::GetCurrentPercOther_Peds | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x610420 | CPopCycle::PickARandomGroupOfOtherPeds | L2 | split grove:18/18 PASS (0 mismatches, 0 undeclared); split pr:19/19 PASS (0 mismatches, 0 undeclared); split cheats:10/10 PASS (0 mismatches, 0 undeclared) |
| 0x6104B0 | CPopCycle::UpdateDealerStrengths | L2 | split grove:1000/1000 PASS (0 mismatches, 0 undeclared); split pr:1000/1000 PASS (0 mismatches, 0 undeclared); split cheats:1000/1000 PASS (0 mismatches, 0 undeclared) |
| 0x610560 | CPopCycle::UpdateAreaDodgyness | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x610720 | CPopCycle::PedIsAcceptableInCurrentZone | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x60FFD0 | CPopCycle::PickPedMIToStreamInForCurrentZone | L2-split (undeclared writes to review) | split grove:79/79 PASS (0 mismatches, 78 undeclared); split pr:38/38 PASS (0 mismatches, 37 undeclared); split cheats:20/20 PASS (0 mismatches, 19 undeclared) |
| 0x610BF0 | CPopCycle::Update | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x43CEB0 | CDarkel::Init | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:2/2 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x43D1F0 | CDarkel::FrenzyOnGoing | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x43DC10 | CDarkel::ResetOnPlayerDeath | L2-split (undeclared writes to review) | split cheats:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x43D350 | CDarkel::ThisVehicleShouldBeKilledForFrenzy | implemented-needs-L2 | no call in grove walk / pause-restart / cheats |
| 0x49CC50 | CTagManager::Init | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:1/1 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x49CC90 | CTagManager::AddTag | L2 | L2 grove:100/100 PASS (0 mismatches, 0 undeclared); L2 pr:100/100 PASS (0 mismatches, 0 undeclared); L2 cheats:100/100 PASS (0 mismatches, 0 undeclared) |
| 0x49CC60 | CTagManager::ShutdownForRestart | L2 | L2 pr:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x49CDA0 | CTagManager::GetPercentageTagged | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x49CCE0 | CTagManager::IsTag | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x49CE10 | CTagManager::SetupAtomic | L2 | L2 grove:3/3 PASS (0 mismatches, 0 undeclared); L2 pr:2/2 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x49D2D0 | CTrafficLights::LightForCars1 | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x49D310 | CTrafficLights::LightForCars2 | L2 | L2 grove:876/876 PASS (0 mismatches, 0 undeclared); L2 pr:58/58 PASS (0 mismatches, 0 undeclared); L2 cheats:247/247 PASS (0 mismatches, 0 undeclared) |
| 0x49D350 | CTrafficLights::LightForCars1_Visual | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x49D3A0 | CTrafficLights::LightForCars2_Visual | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x49D4D0 | CTrafficLights::FindTrafficLightTypeFromOrientation | L1 | L1 1,000,058 cases, 0 mismatches (SNaN unverified); L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x49D5A0 | CTrafficLights::IsMITrafficLight | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x49D520 | CTrafficLights::FindOrientationForTrafficLightType | L2 | split grove:1000/1000 PASS (0 mismatches, 0 undeclared); split pr:1000/1000 PASS (0 mismatches, 0 undeclared); split cheats:1000/1000 PASS (0 mismatches, 0 undeclared) |
| 0x460EC0 | CRoadBlocks::ClearScriptRoadBlocks | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:1/1 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x56CEE0 | CAccidentManager::GetNumberOfFreeAccidents | L2 | L2 grove:19/19 PASS (0 mismatches, 0 undeclared); L2 pr:20/20 PASS (0 mismatches, 0 undeclared); L2 cheats:23/23 PASS (0 mismatches, 0 undeclared) |
| 0x56CE80 | CAccidentManager::ReportAccident | L2 | split cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x5DE680 | CGangs::Initialise | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:2/2 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x5DE5A0 | CGangs::ChooseGangPedModel | L2 | split grove:96/96 PASS (0 mismatches, 0 undeclared); split pr:66/66 PASS (0 mismatches, 0 undeclared); split cheats:39/39 PASS (0 mismatches, 0 undeclared) |
| 0x4439C0 | CGangWars::DontCreateCivilians | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x443920 | CGangWars::InitAtStartOfGame | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:2/2 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x443AC0 | CGangWars::GangWarFightingGoingOn | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x443FF0 | CGangWars::ClearSpecificZonesToTriggerGangWar | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:1/1 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x443F80 | CGangWars::CanPlayerStartAGangWarHere | L2 | L2 grove:53/53 PASS (0 mismatches, 0 undeclared); L2 pr:106/106 PASS (0 mismatches, 0 undeclared); L2 cheats:106/106 PASS (0 mismatches, 0 undeclared) |
| 0x443D50 | CGangWars::TellStreamingWhichGangsAreNeeded | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x423F00 | CCarCtrl::ClearInterestingVehicleList | L2 | L2 cheats:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x423EA0 | CCarCtrl::IsThisVehicleInteresting | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:538/538 PASS (0 mismatches, 0 undeclared); L2 cheats:406/406 PASS (0 mismatches, 0 undeclared) |
| 0x423ED0 | CCarCtrl::RemoveFromInterestingVehicleList | L2 | L2 grove:5/5 PASS (0 mismatches, 0 undeclared); L2 pr:8/8 PASS (0 mismatches, 0 undeclared); L2 cheats:10/10 PASS (0 mismatches, 0 undeclared) |
| 0x423DE0 | CCarCtrl::RegisterVehicleOfInterest | implemented-needs-L2 | no call in grove walk / pause-restart / cheats |
| 0x423F10 | CCarCtrl::PruneVehiclesOfInterest | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x421740 | CCarCtrl::InitSequence | L2 | split grove:474/474 PASS (0 mismatches, 0 undeclared); split pr:179/179 PASS (0 mismatches, 0 undeclared); split cheats:221/221 PASS (0 mismatches, 0 undeclared) |
| 0x421770 | CCarCtrl::FindSequenceElement | L2 | L2 grove:110/110 PASS (0 mismatches, 0 undeclared); L2 pr:68/68 PASS (0 mismatches, 0 undeclared); L2 cheats:41/41 PASS (0 mismatches, 0 undeclared) |
| 0x421FE0 | CCarCtrl::ThisVehicleShouldTryNotToTurn | L2 | L2 grove:378/378 PASS (0 mismatches, 0 undeclared); L2 pr:113/113 PASS (0 mismatches, 0 undeclared); L2 cheats:182/182 PASS (0 mismatches, 0 undeclared) |
| 0x424130 | CCarCtrl::FindSpeedMultiplierWithSpeedFromNodes | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x423FC0 | CCarCtrl::SwitchVehicleToRealPhysics | L2 | L2 grove:14/14 PASS (0 mismatches, 0 undeclared); L2 pr:2/2 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x424000 | CCarCtrl::UpdateCarCount | L2 | L2 grove:503/503 PASS (0 mismatches, 0 undeclared); L2 pr:222/222 PASS (0 mismatches, 0 undeclared); L2 cheats:253/253 PASS (0 mismatches, 0 undeclared) |
| 0x4212E0 | CCarCtrl::Init | L2-split (undeclared writes to review) | split grove:1/1 PASS (0 mismatches, 1 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x4213B0 | CCarCtrl::ReInit | L2 | L2 pr:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x421900 | CCarCtrl::ChooseCarModelToLoad | L2 | split grove:18/18 PASS (0 mismatches, 0 undeclared); split pr:19/19 PASS (0 mismatches, 0 undeclared); split cheats:17/17 PASS (0 mismatches, 0 undeclared) |
| 0x41C050 | CCarAI::CarHasReasonToStop | ported-unhookable | stub bytes read as data by protection code (4 `lea` sites); `carctrl::car_has_reason_to_stop` |

# Pass 2 (PORT-POP-2)

Second pass over the rows pass 1 left plus the POP rows of the UNK classification proposal
(`catalog/unk-classification-proposal.csv`). Done by the PORT-POP-2 agent with two helpers (CPathFind,
CPopulation/CPopCycle) on disjoint new files; verification on lane 2 only. Per-hook verdicts:
`docs/arch/pop-port-2-verdicts.tsv`.

Ported: **321 `#[hook]` registrations** and 14 ported but unhookable functions.

| file | contents |
|---|---|
| `statics_gen.rs` | the 106 POP "header constant" `__init_g_*` initialisers, GENERATED by `tools/gen/pop_statics.py` from the exe bytes (`--check` regenerates in memory). Three shapes only (`a/b`, `a*b`, `c/(a*b)`), computed in f64 and rounded once: they run under the CRT's PC=53 (see the generator) |
| `pathfind.rs` | `CPathFind` (70 of 72 rows), `CPathNode::GetPosition`, the `CNodeAddress` operators, `CompressedVector2D`, `CCarPathLink::OneWayLaneOffset`, the node-area helpers and the 0x977AF0/0x977BE8 statics: 83 hooks + 2 unhookable |
| `population2.rs` | `CPopulation` drivers (Update, ManagePopulation/Ped/Dummy/Object, conversions, RemovePedsIfThePoolGetsFull, GeneratePeds*, AddPedInCar, the file loaders, ...) and `CPopCycle` (Initialise, UpdatePercentages, FindNewPedType, ...): 48 hooks + 7 unhookable |
| `carctrl2.rs` | `CCarCtrl` / `CCarAI` / `CAutoPilot`: GenerateRandomCars, GenerateEmergencyServicesCar, RemoveDistantCars, RemoveCarsIfThePoolGetsFull, SwitchBetweenPhysicsAndGhost, SlowCarOnRailsDownForTrafficAndLights, SetUpDriverAndPassengersForVehicle, ChoosePoliceCarModel, the police mission helpers, the "tell car to" helpers, small steering helpers: 22 hooks + 4 unhookable |
| `misc2.rs` | `CGangs` / `CGangWars` (incl. UpdateTerritoryUnderControlPercentage, DoStuffWhenPlayerVictorious, influence, Save/Load), `CDarkel` (Update, kill registration, frenzy fail), `CTagManager` (rest), `CTrafficLights::ShouldCarStopForLight`/`Bridge`, `CCarGenerator::Process`/`CheckIfWithinRangeOfAnyPlayers`, `CTheCarGenerators::Process`, `CRoadBlocks::Init`, `CAccident*`: 62 hooks + 1 unhookable |

Shared changes: none outside sa-pop (the registry already appends `sa_pop::hooks::ALL`); new lists
`tools/e2e/lists/pop2-*.txt`, generator `tools/gen/pop_statics.py`.

## Findings worth knowing (bytes over gta-reversed / specs)

- `CPopCycle::UpdatePercentages`: the per-zone-type cop values of jump table 0x610BC8 are **floors**
  (`fld cops; fcomp c; jne` stores `c` when `cops <= c`), not caps. The first port had them reversed (split
  mismatch at police-gang frame 305); fixed.
- `FUN_0044E790` (GeneratePedCreationCoors): the squared node distance local `[esp+0x20]` is overwritten
  by its square root for every link that reaches the distance test, so a second such link of the same
  node compares the root of the root (original bug, kept). Without it the all-hooks run diverged in rand
  calls at police-gang frame 208.
- `CPathFind::DoPathSearch` keeps the original endless loop when no link matches; `RemoveInterior` keeps
  its write into the `CPathFind` object; `SwitchRoadsOffInArea` keeps the entry copy bug;
  `UpdateStreaming` marks only vehicles created by the mission (+0x4A4 == 2), correcting
  CPathFind.deviations.md. `CountNeighboursToBeSwitchedOff` needs `preserve = "ecx"`.
- `CCarCtrl::FindIntersection2Lines` and `CDarkel::CheckDamagedWeaponType` need `preserve = "edx"`.
- `CTrafficLights::ShouldCarStopForLight`: a red-light test that fails returns false at once (it does
  not go on with the next link); the direction-byte branch uses the flags of a `cmp` made before the x87
  sequence.
- `CTagManager::Load` saves the tag count with `SaveDataToWorkBuffer` (original bug, kept).
- `CDarkel::Update`: after a time-out the kills-needed test still runs and can end the frenzy as a success.
- `CCarAI::FindPoliceCarMissionForWantedLevel`: missions 4 / 2 by `rand() & 3` (the RE-SPEC-3 "mission
  ids 4/2"), ported but unhookable.
- `CGangWars::EndGangWar` reads `State2` through an obfuscated pointer (`[0x451F3F] ^ [0x156063C]` =
  0x96AB4C); `StrengthenPlayerInfluenceInZone` / `FindPoliceBoatMissionForWantedLevel` /
  `AddNodeToNewInterior` decode constants the same way (decoded from the static image).
- Pool walks: the vehicle / ped pool walks of RemoveDistantCars, IsAnyoneParking, ReleaseCarsInAttackWave,
  ClearTheStreets, RemoveCarsIfThePoolGetsFull, RemovePedsIfThePoolGetsFull, ManageAllPopulation go
  downwards as the bytes do (the RE-SPEC-3 "downward pool walk").

## Unhookable (ported as plain `pub fn`) and not ported

Unhookable: `CGangWars::EndGangWar` 0x4464C0 (entry bytes are a code key read at 0x40589D),
`CCarAI::FindSwitchDistanceFar` 0x41BF70 and `FindPoliceCarMissionForWantedLevel` 0x41C9D0 (an
address inside the patched entry is held at 0x40248F / 0x4049D5), `FindPoliceCarSpeedForWantedLevel`
0x41CAA0 and `TellCarToFollowOtherCar` 0x41C960 (entry bytes read/written as data by protection code),
`CNodeAddress` constructors 0x4241A0 / 0x4241B0 (same), `CPopulation::ChooseCivilianOccupation` 0x612F90
(code key at 0x401732), `AddExistingPedInCar` 0x611560 (data reads at 0x1426A2E / 0x1515006), and five
1–3-byte functions (Shutdown 0x610EC0, MoveCarsAndPedsOutOfAbandonedZones 0x6116A0, DealWithZoneChange
0x6116B0, IsSecurityGuard 0x6114B0, CanSolicitPlayerOnFoot 0x611780). Too short and not ported:
`FindPoliceBikeMissionForWantedLevel` 0x41CA40 (`mov al, 0x2B; ret`), `~CGangInfo` 0x5DE540 / `~CAccident`
0x56CDF0 (`ret`), the alias jumps `ChooseGangCarModel` 0x421A40 / `GetAlpha(RpAtomic)` 0x49CD40.

Not ported (time or protection): `CPopulation::AddToPopulation`, `AddPed` (protection stub + EH frame),
`AddPedAtAttractor`, `PedMICanBeCreatedAtThisAttractor`, `AddDeadPedInFrontOfCar`, `CreateWaitingCoppers`,
`PlaceCouple`, `PopulateInterior`, `CWaterLevel::CreateBeachToy`; `CPathFind::Find2NodesForCarCreation` and
`RemoveInteriorLinks` (values decoded at run time by the 0xCB73E0 protection VM); `CRoadBlocks::
RegisterScriptRoadBlock` (calls the VM), `GenerateRoadBlocks`, `CreateRoadBlockBetween2Points`,
`GenerateRoadBlockPedsForCar`, `GetRoadBlockNodeInfo`, `ClearSpaceForRoadBlockObject`; `CGangWars::Update`,
`Start*GangWar`, `CreateAttackWave`, `CreateDefendingGroup`, `CheerVictory`, `TellGangMembersTo`,
`PickZoneToAttack`, `AttackWaveOvercome` (obfuscated pool pointer), `ReleasePedsInAttackWave` (VM);
`CCarCtrl::GenerateOneRandomCar` and the steering / weaving / pick-next-node / flying functions,
`CCarAI::UpdateCarAI` and the GetCarToGoTo* family; `CTheCarGenerators::Save/Load`,
`CCarGenerator::DoInternalProcessing` / `CheckForBlockage`; `CTrafficLights::DisplayActualLight`;
`CDarkel::DrawMessages`, `StartFrenzy`, `RegisterKillByPlayer`, `DealWithWeaponChangeAtEndOfFrenzy`.

## Verification (pass 2, lane 2)

- Tests: `cargo test --release -p sa-hook --lib` 51 pass (every_hook_installs_on_the_exe,
  matches_catalogue_and_exe, callers ECX/EDX, keycells, patch_refs, data_reads); `cargo test -p sa-pop` 3
  pass; clippy `-D warnings` for sa-pop host (all targets), i686 (`--features in-process`), wasm32 and
  sa-hook i686; `tools/gen/pop_statics.py --check` ok.
- **All-ports L3**: `tools/e2e/verify-all.sh pop2c 2` on the final build (commit "Fix the UpdatePercentages
  cop floors ..."): IDENTICAL on all 11 recordings (grove 5200, air-water 5800, anim-input 3300, car-ai
  5000, c-bug 3220, driving 4338, menus-hud 5700, pause-restart 3700, police-gang 5200, weapon-combat
  3500, world-cheats 3276). An earlier air-water failure (all subsets "DIVERGED at frame 201") was a
  corrupt cached plain trace (`desyncs=5594` in the plain run itself); the cache entry was deleted and the
  rerun is identical.
- Split L2 (SA_L2_SPLIT, `--runs "plain split"`, SPLIT_STACKFILL=16384): statics on c-bug (106 PASS, 0
  undeclared); police-gang runs `pop2-mine-pg` (misc2/carctrl2 first 68 hooks), `pop2-pop-pg`
  (population2), `pop2-ab-pg` (population2 + pathfind); car-ai `pop2-final-ca` (every non-static hook,
  final build). Large drivers hit the 2 GiB spool quota (cases compared before it stay valid, marked
  "COUNT (spool quota cut)"). Undeclared writes of the drivers are their callees' (streaming, collision
  cache, replay buffers, vehicle counters on deletion); they are listed as "L2-split (undeclared writes to
  review)" as in pass 1.
- Failures found and fixed: `CPopCycle::UpdatePercentages` (cop floors reversed),
  `FUN_0044E790` (distance slot). Not explained: `CPopCycle::FindNewPedType` split police-gang 3/10
  mismatches (the Rust run wrote `*outType = 4` where the original left 0, frame 207, caller 0x614A31);
  its bytes were re-read against the port without finding a difference and the all-hooks runs are
  identical on all 11 recordings, so the hook stays on; it needs a split rerun with SPLIT_DUMP_SEQ.
- `pop2-final-ca` (car-ai, final build, 215 non-static hooks, `--max 100`, SPLIT_PER_FRAME=2): splitr
  trace == plain on all 5000 frames; the splito run (all original, sampled calls recorded) ended with a
  Wine page fault (write to 0x1 at 7918D4F2) at frame 2304, so cases after that frame are not compared;
  58 PASS, 27 COUNT (quota / early end), 6 DIVERGED, 1 FAIL (FindNewPedType, again 1 mismatch at its first
  case, frame 207, caller 0x614A31). The splito crash is a harness open item (an original-only run should
  not crash); it was not investigated.

### Verdicts (pass 2)

`docs/arch/pop-port-2-verdicts.tsv` (335 rows): **L2 152** (106 statics + 46; split, 0 mismatches,
0 undeclared), **L2-split with undeclared writes to review 37**, **implemented 14** (13 whose split inputs
diverged before a comparison, plus FindNewPedType), **implemented-needs-L2 118** (no sampled call in the
runs above: interiors, scripts, save/load, most CPathFind searches outside the recordings, emergency
services, ...), **ported-unhookable 14**. Every hooked row is also covered by the all-ports L3 (IDENTICAL on
all 11 recordings). No row claims L3 (no per-hook hook-off runs).

### Open items (pass 2)

- FindNewPedType split mismatch (see above); the splito crash on car-ai; split reruns with smaller
  windows for the 118 needs-L2 rows (other recordings / scripted situations: interiors, gang wars,
  emergency services, frenzies, save/load).
- l2-extras lines for the drivers' callee writes (streaming info, colmodel cache, pools, replay buffers).
- Requests from the helpers: sa-stream still declares 0x450DE0, 0x44DE00, 0x4529F0, 0x44D0F0 as
  `=> unported` (could point at the pathfind ports); `statics.rs` could use pathfind.rs's
  `NODES_TO_BE_CLEARED` / `EXTERIOR_NODE_LINKED_TO`; uninitialised stack words that the ports set to 0
  (best-node word in FindNodeClosestToCoors*, ComputeRoute locals, CPopCycle::Initialise sscanf locals);
  `LoadPed/CarGroups` stop at the 0x400 buffer end on a last line without newline (unreachable with the
  shipped files).

# Pass 3 (PORT-POP-3)

Third pass over the 139 POP rows still `specified` after pass 2, in the order of the brief: CCarCtrl driving
AI, CPopulation, CGangWars::Update, CRoadBlocks::GenerateRoadBlocks, CTrafficLights, then the rest. The lead
and two helpers worked in the same worktree on disjoint new modules; verification on lane 2 only.
Per-function verdicts: `docs/arch/pop-port-3-verdicts.tsv` (140 rows: the 139 plus `CPopCycle::FindNewPedType`).

| file | contents |
|---|---|
| `carctrl3.rs` (helper A) | the steering family: `SteerAICarWithPhysics` and its mission steering (`_OnlyMission` partial), FollowPath, HeadingForTarget, TryingToBlockTarget(_Stop), boats, helis, planes, FindMaximumSpeedForThisCarInTraffic, FindAngleToWeaveThroughTraffic, the Weave* and SlowCarDown* families, TestCollisionBetween2MovingRects*, FindGhostRoadHeight, `CAutoPilot::ModifySpeed`: 46 hooks |
| `carctrl4.rs` (lead) | node pickers (FindPathDirection, IsThisAnAppropriateNode, PickNextNodeRandomly / AccordingStrategy / ToChaseCar, ToFollowPath unhookable), the CCarAI helpers (GetCarToGoTo* (Accurate unhookable), EntitiesGoHeadOn, BackToCruisingIfNoWantedLevel, MellowOutChaseSpeed, MakeWayForCarWithSiren, AddPoliceCarOccupants), `UpdateCarAI` (partial), UpdateCarOnRails, ScanForPedDanger, FindLinksToGoWithTheseNodes, JoinCarWithRoadSystem / AccordingToMission, FindNodesThisCarIsNearestTo, ReconsiderRoute: 26 hooks + 2 unhookable |
| `carctrl5.rs`, `population3.rs`, `misc3.rs` (helper B) | GenerateOneRandomCar, GenerateCarCreationCoors2, ChooseModel, PossiblyRemoveVehicle, GetNewVehicleDependingOnCarModel, CreateCarForScript, the emergency-services generators (GenerateOneEmergencyServicesCar unhookable), SetCoordsOfScriptCar; `CPopulation::AddPed` (partial), AddPedAtAttractor, PedMICanBeCreatedAtThisAttractor; `CGangWars::Update` (partial), StrengthenPlayerInfluenceInZone, AttackWaveOvercome; `CRoadBlocks::GenerateRoadBlocks` (partial); `CDarkel::DrawMessages` (partial), RegisterKillByPlayer; `CCarGenerator::CheckForBlockage`, DoInternalProcessing (partial): 19 hooks + 1 unhookable |
| `misc4.rs` (lead) | `CDarkel::ReadStatus` (entry_eax), QueryModelsKilledByPlayer, `CWaterLevel::CreateBeachToy`, `CTheCarGenerators::Save` / `Load` |

Shared changes: `sa-core::abi::footprint_residue_u8` (the byte an uninitialised local of the original
reads, from the footprint snapshot; used by PickNextNodeToChaseCar and FindNodesThisCarIsNearestTo);
sa-hook `keycells::READERS` gains the UpdateCarAI entry reader (`xor ecx, [0x41DA32]` at 0x407284); the
merge of PORT-POP-2 needed five `orig!` declarations in sa-pop / sa-veh to return ST0 as `f64` (the 16-byte
X87 of master's 3064b95 cannot cross `orig!`; master did not build without this). `tools/e2e/l2-extras.txt`:
out-parameters (FindNewPedType, FindPathDirection, FindNodesThisCarIsNearestTo, ChooseModel,
GenerateCarCreationCoors2, AddPed) and the path-search regions (ThePaths 0x96F050, aNodesToBeCleared
0x972CD0) of the callers of DoPathSearch.

## ADR-002 partial ports (what runs the original)

- `CCarAI::UpdateCarAI`: Rust for cars that are not law enforcers with mission 0 or 1 (ambient traffic, the
  vast majority of calls); everything else the original body. The early returns (recording without
  `bUseCarAI`, type 3) are Rust.
- `SteerAICarWithPhysics_OnlyMission`: see the PORTED list in its doc comment (stopped, follow-path set,
  racing, pre-recorded, heading, block-stop, heli / plane towards target, boats, dog fights).
- `CPopulation::AddPed`: the original when one of the cheats 0x969140 / 0x96913E / 0x96915C / 0x96915D is on or
  the ped type is < 4, 21 or > 31. `CGangWars::Update`: the original while a gang war is active.
  `GenerateRoadBlocks`: the original when the player is wanted in a vehicle or this frame's script road
  block is active. `CDarkel::DrawMessages`: the original for frenzy status 1..4. `DoInternalProcessing`:
  the original for model 0x24C only (that path passes the caller's EDI to AddPed).

## Findings worth knowing (bytes over gta-reversed / specs)

- `UpdateCarAI`: the stuck test runs only for missions outside {0, 6, 0xB, 0x1B..0x20, 0x35} with a non-zero
  cruise speed (gta-reversed inverts it); the speed multiplier snaps to its target when within one step
  (gr keeps the old value); the mission 0x3B plane fallback also sets mission 0x16; type 3 vehicles always
  return at entry; up.z < -0.7 sets temp action 1 (wait), not reverse.
- `GetCarToGoToCoors`: missions 8 / 9 keep their mission while the *target* is within 5 of the stored
  destination (gr tests the vehicle position).
- `BackToCruisingIfNoWantedLevel` reaches FindPlayerPed through an obfuscated call (`[0x1560474] -
  [0x638D32]` = 0x56E210).
- `PickNextNodeToChaseCar` / `ToFollowPath` use opposite lane-count mappings; both kept. ChaseCar and
  FindNodesThisCarIsNearestTo can store node words of uninitialised locals; the ports read them from the
  footprint snapshot.
- `UpdateCarOnRails`: the detour at 0x43654F tests +0x3DC, not StopCarIfNodesAreInvalid's result.
- Helper A: TestCollisionBetween2MovingRects swaps the direction arguments; FindAngleToWeaveThroughTraffic
  scans each sector twice and repeats until stable; SlowCarDownForPedsSectorList grows the bike half-width per
  ped and uses `max(1, ...)`; the Predator fires on a signed turn < 0.15; FollowPath mission 8 / 0xC dots
  with the position.
- Helper B: GenerateCarCreationCoors2 accepts the point when `IsSphereVisible` is true (CCarCtrl.md has it
  reversed); bike / BMX allocation failure sets +0x614 through a null pointer (kept); decoded constants
  ScriptGenerateOneEmergencyServicesCar argument 0, StrengthenPlayerInfluenceInZone 1.0f, AttackWaveOvercome
  pool 0xB74490.

## FindNewPedType (pass-2 open item)

Not reproduced on the merged build: alone on police-gang 20/20 and car-ai 20/20, with all pass-2 hooks on
car-ai 3/3, and in the pass-3 batches on car-ai, driving and police-gang 100/100 each, 0 mismatches; with
the out-parameters declared, 0 undeclared. The pass-2 build predates master's X87 / footprint / harness
changes (3064b95, 100fb7a); the mismatch (`*outType = 4` vs 0 at the first case, caller 0x614A31) is
consistent with the Rust and original runs reading different stack residue in a callee
(ChooseCivilianOccupation runs at a different depth under the Rust caller), which the current stack fill /
frame layout no longer exposes. Verdict L2.

## Split testing of nested Rust callers

A Rust caller passes pointers into its own frame; a sampled callee writing through such a pointer then
appears with a shifted stack. Helper A measured false mismatches when a Rust ancestor of the sampled
function was in the same list, and none with the lists layered by call depth (X / 2 / 3 / 4). The lead saw
the same at one FindPathDirection case (ordinal 32, frame 1053) only in the batch that also had
UpdateCarOnRails on Rust; alone and in the other batches it passes.

## Not ported

- VM (0xCB73E0): JoinCarWithRoadSystemGotoCoors 0x42F870, AddAmbulanceOccupants 0x41C4A0,
  AddFiretruckOccupants 0x41C600, TellOccupantsToLeaveCar 0x41C760, TellGangMembersTo 0x444530,
  RegisterScriptRoadBlock 0x460DF0, Find2NodesForCarCreation 0x452090, RemoveInteriorLinks 0x44DF60,
  ReleasePedsInAttackWave 0x445C30.
- Too short / aliases: ClitargetOrientationToLink, FindPoliceBikeMissionForWantedLevel, ChooseGangCarModel,
  ChoosePolicePedOccupation, ChooseGangOccupation, `CTagManager::GetAlpha`.
- No time (every-frame callers first): `CPopulation::AddToPopulation` 0x614720 and
  `CTrafficLights::DisplayActualLight` 0x49DAB0 (both run every frame in the corpus), DragCarToPoint,
  FlyAIHeliInCertainDirection (police heli), FlyAIHeliToTarget_FixedOrientation, FlyAIPlaneInCertainDirection,
  FollowPreRecordedPath, FollowPath_Racing, DealWithBend_Racing, ParkParallel / ParkPerpendicular,
  CreatePoliceChase, CreateConvoy, AddDeadPedInFrontOfCar, CreateWaitingCoppers, PlaceCouple, PopulateInterior,
  the gang-war helpers (PickZoneToAttack, CheerVictory, Start*GangWar, CreateAttackWave, CreateDefendingGroup),
  the road-block helpers, `CDarkel::StartFrenzy` / DealWithWeaponChangeAtEndOfFrenzy.

## Verification (pass 3, lane 2)

- Tests: `cargo test --release -p sa-hook --lib` 51 pass; `cargo test -p sa-pop` 3 pass; clippy `-D warnings`
  for sa-pop host (all targets), i686 (`--features in-process`) and wasm32.
- Split runs (SPLIT_STACKFILL=16384, `--max 100`, `--runs "plain split rust"`): lead lists `pop3-L`,
  `pop3-L2`, `pop3-L3` on car-ai, driving, police-gang, grove; helper A lists `pop3-AX`, `A2`..`A7`; helper B
  `pop3-B1`, `B2`, `B12`, `B3`, `B5`. Every `rust` and `splitr` trace was IDENTICAL to plain; every compared
  case had 0 mismatches except the layering artefact above.
- All-ports L3: `tools/e2e/verify-all.sh pop3 2` on the final build (commit 524fd5b): IDENTICAL on all 11
  recordings (grove 5200, air-water 5800, anim-input 3300, car-ai 5000, c-bug 3220, driving 4338, menus-hud
  5700, pause-restart 3700, police-gang 5200, weapon-combat 3500, world-cheats 3276), 17 subsystems.

### Verdicts (pass 3)

`docs/arch/pop-port-3-verdicts.tsv` (140 rows): **L2 30**, **L2-split with undeclared writes to review 20**
(callee writes: construction, pools, references, collision cache; some declared since, reruns pending),
**partial-fallback 7**, **implemented-needs-L2 36** (no sampled call in car-ai, driving, police-gang,
grove), **ported-unhookable 3**, **not-ported 44**. No row claims L3 (no per-hook hook-off runs).

# Pass 4 (WRITER-POP-4, writer/verifier split)

Writer pass over pass 3's "not ported" list (minus the VM functions and the short aliases), every-frame
functions first. No game lanes: every new hook is `unverified` (PLAN §5.1) and waits for a verifier. The lead
and two helpers worked in the same worktree on disjoint new modules; `tools/e2e/l2-extras.txt` has the
out-parameters and pool regions of every new port.

| batch | file | hooks |
|---|---|---|
| `popw4-frame-1` | `misc6.rs`, `population4.rs` (lead) | `CTrafficLights::DisplayActualLight` (L2), `CPopulation::AddToPopulation` (split, footprint) |
| `popw4-pop-1` | `population4.rs`, `carctrl7.rs` (lead) | PopulateInterior, CreateWaitingCoppers, PlaceCouple, AddDeadPedInFrontOfCar, `CCarCtrl::CreatePoliceChase`, CreateConvoy (all split) |
| `popw4-drive-1` | `carctrl6.rs` (helper A) | FlyAIHeliInCertainDirection, FlyAIHeliToTarget_FixedOrientation, FlyAIPlaneInCertainDirection, SteerAICarParkParallel / Perpendicular, FollowPreRecordedPath, FollowPath_Racing, DragCarToPoint, DealWithBend_Racing (all L2) |
| `popw4-gang-1` | `misc5.rs` (helper B) | PickZoneToAttack, CheerVictory, StartDefensiveGangWar, StartOffensiveGangWar, CreateDefendingGroup, CreateAttackWave (split, footprint, new keycells reader 0x403A92), GetRoadBlockNodeInfo, ClearSpaceForRoadBlockObject, GenerateRoadBlockPedsForCar (partial), `CDarkel::StartFrenzy`, DealWithWeaponChangeAtEndOfFrenzy |
| `popw4-partial-1` | `misc3.rs`, `population3.rs` | `CDarkel::DrawMessages` completed (no fallback); `CPopulation::AddPed` cheat paths ported (fallback left for types < 4, 21, > 31); PedStreamedInForThisGang / PickStreamedInPedForThisGang back to unverified after the `model_loaded` fix |

## Findings (bytes over gta-reversed / specs)

- `AddToPopulation`: `FindNewPedType` can return 1 writing nothing, so the type / model locals are then the
  stack residue of earlier calls (`[entry ESP - 0x10C]` / `- 0xE8`); the port reads them from the footprint
  snapshot and records its final values. `PopulateInterior` has the same pattern (`- 0x14` / `- 0x18`, the
  previous try's values within a call).
- `PlaceCouple` is instruction for instruction the couple branch of `AddToPopulation` (shared `add_couple`).
- `CreateWaitingCoppers` stores `z + 1` into its own argument slot (recorded as a footprint write); the cop
  spot uses FSIN / FCOS of an 80-bit angle and the heading FPATAN.
- `CreatePoliceChase` / `CreateConvoy`: the ground search starts at 1e9 and the "no ground" test is `> 1e9`,
  so a car with no ground at all is placed at z = 1e9 + base distance (kept).
- `DisplayActualLight`: corona radius `[0xB7C4E0] * 0.175`, colour factor `[0xB7C4E4] * 0.07`; the pedestrian
  light state is `LightForPeds` inlined.
- `CDarkel::DrawMessages`: the second `GetYPosBasedOnHealth` call gets the first one's result as its float and
  player 1 (the result left on the stack becomes the next argument).
- Helper A: FlyAIPlaneInCertainDirection's entry `E9` jumps to a no-CD fragment at 0x401961 (not .HOODLUM): the
  hook's expect covers 8 bytes so it is installed inline. DragCarToPoint divides `L`, not `k1` as
  CCarCtrl.p2.md says; DealWithBend_Racing's parallel shortcut needs `|dy|` non-zero or NaN (kept).
- Helper B: `misc2::model_loaded` (pass 2) read the load state at 0x8E4CD0 + 0x10 although the bytes read
  `[model * 20 + 0x8E4CD0]` (`m_nLoadState` of 0x8E4CC0): fixed; this changes the verified
  PedStreamedInForThisGang 0x4439D0 and PickStreamedInPedForThisGang 0x443A20 (re-verify them).
  CreateAttackWave: the jump-table default's local is the player ped pointer, the two weapon-type locals are the
  uninitialised ones (footprint residue); Gang2's peds get type `Gang1 + 7` (kept).

## Not ported (pass 4)

- `CRoadBlocks::CreateRoadBlockBetween2Points` 0x4619C0 (about 4 KB with stack `CMatrix` locals, argument-slot
  reuse and fires; no time).

- Partial ports still with fallback: `UpdateCarAI` (law enforcers / other missions),
  `SteerAICarWithPhysics_OnlyMission` (inline-steering missions), `CGangWars::Update` (active war state
  machine), `GenerateRoadBlocks` (wanted-level road blocks), `CCarGenerator::DoInternalProcessing` (model 0x24C
  passes the caller's EDI), `CRoadBlocks::GenerateRoadBlockPedsForCar` (seat offsets past its table), `AddPed`
  (types < 4, 21, > 31).
