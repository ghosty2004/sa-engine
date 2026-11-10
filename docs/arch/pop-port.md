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
