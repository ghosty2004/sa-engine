# PORT-WORLD part 1: leaf-ward WORLD classes (crate `sa-world`, module `world`)

Scope (task PORT-WORLD part 1): the WORLD rows that do not depend on `CPhysical` — zones (`CTheZones`,
`CZoneDef`, `CGridRef`), `CWorld` helper leaves (sector/list accessors, `FindPlayer*`), pickups,
entry-exits, garages, escalators, bridge/cranes, stunt jumps, `CStats`, `CCheat`, `CRestart`, `CGameLogic`,
`CShopping`, and the static initialisers/`atexit` destructors of these globals: 468 catalogue rows (449 WORLD rows of these classes,
the unnamed rows whose callers are in them and the static initialisers, plus the 19 `CWorld` leaves ported
here). `CPhysical`/buoyancy (PHYS, the
other implementation agent), `CEntity`/`CPlaceable`/`CObject`, interiors and `CReplay` are not part of
this pass. Agent states (the main session sets `catalog/state.csv`) are in the table below.

Follows ADR-001, `docs/arch/coll-port.md` (same crate, same rules) and `docs/re/specs/WORLD/*.md` (the
gta-reversed deviation notes there were applied: ports follow the bytes, never gta-reversed's fixes).

| state | rows |
|---|---|
| L1 | 21 |
| L2 | 83 |
| L2-split (undeclared writes to review) | 18 |
| deferred | 220 |
| implemented-needs-L2 | 112 |
| ported-unhookable | 14 |

## Crate layout

New module `crates/sa-world/src/world/` (separate from `collision/`, and from whatever module PHYS adds),
registered in `lib.rs` and in `hooks.rs` (`MODULES`/`GLOBALS`, section "PORT-WORLD part 1", appended after
the COLL modules):

| file | contents |
|---|---|
| `common.rs` | byte-offset field access (`at`/`rd`/`wr`), `g()` for globals owned by other subsystems (as sa-stream does), x87 shorthands, `_ftol`, shared `orig!` (`FindPlayerCoors/Ped/Vehicle`, `CGenericGameStorage` work buffer) |
| `zones.rs` | `CTheZones` (arrays as byte-array globals with `CZone` 0x20 / `CZoneInfo` 0x11 offsets), `CZoneDef::IsPointWithin` |
| `cworld.rs` | `CWorld` sector/repeat-sector/LOD list accessors, line-test options, scan codes, `FindPlayer*` family, player-slot lookups |
| `misc.rs` | `CBridge`, `CCranes` (empty in the PC build: `ret` / `xor al,al; ret`, not hookable), `CGridRef` |
| `pickups.rs` | `CPickup`/`CPickups` leaves and `CPickups::Update` |
| `garages.rs` | `CGarage`/`CGarages` leaves, door state machine helpers, point-in-garage tests, `CGarages::Update` |
| `entryexit.rs` | `CEntryExit`/`CEntryExitManager` leaves (pool lookups, flags, links, outside-world position) |
| `escalators.rs` | `CEscalator`/`CEscalators` (ctor/dtor/SwitchOff/Init/Shutdown/AddOne/Update), `CStuntJumpManager` Init/AddOne/ShutdownForRestart |
| `gamelogic.rs` | `CGameLogic` leaves (skip set-up, death restart, city/line-area tests, weapon store) |
| `restart.rs` | `CRestart` (restart tables, override, closest hospital/police point) |
| `stats.rs` | `CStats` leaves (values, conversions, radio favourites, criminal rating, counters, decrement/modify) |
| `cheat.rs` | `CCheat` input (`DoCheats`, `AddToCheatString`, `ApplyCheat`, `ResetCheats`), the weather/time/wanted/toggle/stat/vehicle cheats |
| `shopping.rs` | `CShopping` lookups (keys, prices, extra info, bought flags, clothes store/restore) |
| `statics.rs` | static initialisers / `atexit` destructors (CRT `eh vector constructor/destructor iterator` with the original ctor/dtor addresses, as sa-stream's `CLoadingScreen` statics) |

Integration: none needed in sa-hook (it already appends `sa_world::hooks::ALL`). sa-oracle: new spec
file `specs_world_p1.rs` chained into `specs::all()` and declared in `lib.rs`. `tools/e2e/l2-extras.txt`:
new section "PORT-WORLD part 1" at the end. New lists `tools/e2e/lists/world-p1.txt` (all hooks),
`world-a.txt` (first batch). New input program `tools/e2e/programs/world-cheats.txt`.

## Conventions and findings

- **Globals.** This module's own globals are declared with `globals!` (initial values checked against the
  exe by the crate test: the gang colour tables at 0x8D1344/0x8D1350/0x8D135C needed their order fixed
  after the test failed). Globals owned by unported subsystems (camera, pad, weather, model infos,
  `CWorld::Players` whose +0 is sa-core's `PLAYER0_PED`) are addressed with `g()`. Merge note for PHYS:
  `CWorld` globals declared here are 0xB7CD64..0xB7CD78 (line-test options, `PlayerInFocus`,
  `ms_nCurrentScanCode`); if PHYS declares any of them the `globals_are_declared_once` test will name the
  pair.
- **Preserved registers** (sa-hook `callers` test): `CWorld::FindPlayerWanted` ECX,
  `CEntryExit::GetPositionRelativeToOutsideWorld` EDX, `CGarage::PlayerArrestedOrDied` ECX+EDX,
  `CGarage::IsPointInsideGarage` EDX, `...-CVector-float` ECX+EDX, `CPickup::IsVisible` EDX,
  `CStats::IsStatCapped` ECX.
- **Entry bytes read as data (new finding).** The no-CD protection code reads some function entries as
  data, so patching them breaks it: `sbb eax, [0x55C410]` at 0x4017B4 (`CStats::SetNewRecordStat`),
  `add eax, [0x563470]` at 0x4013A2 (`CWorld::ClearScanCodes`), `xor ecx, [0x572363]` at 0x406A9E
  (`CTheZones::FindSmallestZoneForPosition`). With these hooked the L2 run crashed at frame ~203-270
  (`page fault ... at 0x4017BA`). They are ported but not registered (`ported-unhookable`). A scan of the
  other crates' hooks (scratch `dataref.py`: every disp32 operand inside a hook's 5 patched bytes) finds
  the same pattern for 0x407AD0 (read at 0x401BCE, 0x40203A), 0x410860 (0x401E46) and 0x4C4570
  (0x1564881, 0x156B570). Proposal for the harness slot: make sa-hook's installer test reject such hooks.
- **Non-HOODLUM `jmp` entries** (`E9` into a chunk in low `.text`, e.g. `CWorld::ClearScanCodes`,
  `CStats::SetNewRecordStat`): the `expect` string gets one more byte so sa-hook treats the entry as
  inline (a 5-byte `E9` expect means "`.HOODLUM` stub").
- **Original bugs kept** (selection): `CTheZones` visited index `(u8)_ftol(...)` (out-of-map wrap);
  `AssignZoneInfoForThisZone` spins forever for `index < count`; `FindSmallestZoneForPosition` unsigned
  size compare; `SetZoneRadarColours` never writes alpha; `FindZoneByLabel` -1 for type != 2;
  `FindPlayerSlotWithVehiclePointer` returns 0 when not found; `FindPlayerCentreOfWorld` tests the focus
  player's vehicle, not `id`'s; `..._NoSniperShift` indexes `Players[-1]` for id -1; `CStats::SetStatValue`
  / `SetNewRecordStat` store int stats as `(int16)_ftol`; `CShopping::HasPlayerBought` /
  `SetPlayerHasBought` read/write the byte before the table for unknown keys; `CPickups::Update`'s
  player-2 test looks at player 1's intelligence; `CGameLogic::IsPointWithinLineArea` returns the
  parity in EAX.
- **Not reproduced** (stack-only, not observable): `CEntryExit::GetPositionRelativeToOutsideWorld` stores
  the z sum into its own argument slot; the SEH frames of `CStuntJumpManager::Init` (around the pool
  constructor) are not built.

## Deferred rows (220)

| reason | rows |
|---|---|
| not ported in this pass: mid-size bodies calling unported PED/VEH/HUD/streaming code (e.g. most `CStats::Update*`, `CStats::IncrementStat`, pickup effects, garage entity tests, shopping loaders' helpers) | 91 |
| orchestrators over unported subsystems (`CGameLogic::Update`, `CGarage::Update`, `CPickup::Update`, `CEntryExitManager::Update`, `CEntryExit::TransitionStarted/Finished`, `CStuntJumpManager::Update`, `CEscalator::Update`/`AddThisOne`, …) | 35 |
| cheats creating peds/vehicles/weapons/events (SEH frames, PED/VEH/AI/WEAPON code: weapon sets, `VehicleCheat`, `SuicideCheat`, `MayhemCheat`, …) | 22 |
| save/load paths (need a save/load recording; SAVE pass) | 18 |
| static initialisers/destructors of classes outside part 1 (`CReplay`, `gMatrixList`, furniture, interiors, break manager, water creatures) | 16 |
| unnamed callees of `CGarage::Update` (catalogue `unknown`) | 12 |
| loaders/initialisers with CRT `sscanf`/`_strupr` (`CGridRef::Init`, `CShopping::Load*`, `CStats::Load*`, `CEntryExitManager::Init`, `CStats::Init`, `CShopping::Init`) | 9 |
| unnamed / `CObject` helpers (CObject/CEntity pass) | 8 |
| bodies with no-CD protection chunks that compute a load/store address at run time (`CEntryExitManager::AddEntryExitToStack`, `GetEntryExitIndex`, `CEntryExit::GetEntryExitToDisplayNameOf`, `CGarages::TriggerMessage`, `CGameLogic::ClearSkip`, `IsPlayerUse2PlayerControls`, `CStuntJumpManager::Shutdown`): need a runtime measurement first (PLAN §4b) | 7 |
| `CSimpleTransform` helpers (`TransformPoint`, `TransformVectors`; entity pass) | 2 |
| `CTheZones::CreateZone` (load-time, 694 bytes; next pass) | 1 |


## Verification (commands, results)

All game runs on lane 2 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=2`, DXVK no-vsync, `L2_PER_FRAME=1000`),
outputs under the worktree's `target/e2e/lane-2/port-l2-<tag>/`. Recordings: grove walk
(`target/e2e/lane-2/inf4/rec.sarec`, 5200 frames), pause-restart (`target/e2e/corpus/pause-restart.sarec`,
3700 frames), and the new cheats recording (`tools/e2e/programs/world-cheats.txt`, recorded with
`tools/e2e/record-program.sh ... --replay`: replay == record IDENTICAL over 3276 frames; copied to
`/home/ghosty2004/sa-engine/target/e2e/corpus/world-cheats.sarec`, sha256 `0fca4b2e…a1ee0`; screenshots show
the dusk clock, a weapon-cheat weapon, "Wasted" after SZCMAWO and the hospital respawn).

Hook lists: `tools/e2e/lists/world-p1.txt` (all 234 registrations), split into `world-p1-l2.txt` (141, the
plain `l2` mode) and `world-p1-split.txt` (93: hooks that delete/create entities, allocate, register
references or `atexit` handlers, drive audio, cheats and the static initialisers; they cannot run twice on
the same state, so they are verified with the INF-17 split-run mode).

- **L1** (`SA_WINEPREFIX=/nonexistent sa-oracle l1 <key> --cases 1000000 --no-fixtures --threads 8`, specs
  in `crates/sa-oracle/src/specs_world_p1.rs`): 21 specs, every one `mismatches=0`; 10 PASS, 11
  INCOMPLETE only by their signalling-NaN cases (unverified with the native backend off, as in PORT-COLL).
  Logs `target/l1logs/*.log`.
- **L2 (plain mode)**: grove `g-l2` (0 mismatches; 9 FAIL rows were undeclared writes into computed-address
  arrays and out-parameters, declared in `l2-extras.txt` after it) → `g-l2b` 0 failures; pause-restart `p-l2`
  (2 FAIL rows, `CRestart::Initialise` / `CTheZones::InitZonesPopulationSettings`, undeclared array writes,
  declared) → `p-l2b` 0 failures; cheats `c-l2` 0 failures. (b) l2 trace == plain trace on every run.
- **L2 (split mode)**: `g-split` 28 PASS, `p-split` 29 PASS, `c-split` 43 PASS; 0 FAIL / DIVERGED; splito and
  splitr traces == plain on all frames. 18 hooks pass with undeclared-write cases reported by the
  split report (state `L2-split (undeclared writes to review)` in the table).
- **L3 (frame identity)**: `rust` run with every listed hook on Rust: grove (`g-l2`, `g-all`: all 234 hooks)
  IDENTICAL over 5200 frames, 17 subsystems; pause-restart (`p-l2`, `p-all`) IDENTICAL over 3700 frames;
  cheats (`c-l2`, `c-all`) IDENTICAL over 3276 frames. `port-l2.sh ports --runs "plain rust"` (`g-ports`,
  1131 engine ports including these) IDENTICAL over 5200 frames, no crash/panic/install failure. Per-hook
  hook-off runs were not made, so no row claims L3.
- **Crash found and fixed**: the first L2 runs crashed at frame ~203 (`page fault ... at 0x42480000`, and
  `... at 0x4017BA` in a half). An automatic bisect on lane 2 (scratch `autobis.sh`) isolated
  `CPickup::Remove` under the plain l2 mode (the object is deleted twice: original run, then Rust run), which
  led to the split list; the protection data reads (see Conventions) were found from the 0x4017BA fault.
- `cargo test --workspace --release` (`SA_WINEPREFIX=/nonexistent`, separate target dir): all pass
  (sa-world 3 tests incl. globals vs exe image and hook list; sa-oracle `--test l1` 16 incl.
  `preserved_registers_match_the_hook_registrations`; sa-hook 37 incl. catalogue/exe entry bytes,
  relocation and the ECX/EDX `callers` test). `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo clippy --workspace --target i686-pc-windows-gnu -- -D warnings`, `cargo clippy -p sa-world
  --target wasm32-unknown-unknown -- -D warnings`: clean. `cargo build --release --target
  i686-pc-windows-gnu -p sa-hook -p sa-launcher`: OK.


## Functions

| address | name | source | gr dev rows | port (sa-world `world::`) | state | evidence | behaviour / deviations kept |
|---|---|---|---|---|---|---|---|
| 0x407250 | CWorld::GetCurrentScanCode | gta-reversed | 0 | cworld::get_current_scan_code | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CWorld::GetCurrentScanCode` (0x407250). |
| 0x407260 | CWorld::GetSector | gta-reversed | 0 | cworld::get_sector | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `CWorld::GetSector(int32 x, int32 y)` (stub 0x407260 → 0x1566820): both clamped to 0..119 (negative → 0, `>= 0x77` → 0x77). |
| 0x4072a0 | CWorld::GetRepeatSector | gta-reversed | 1 | cworld::get_repeat_sector | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CWorld::GetRepeatSector(int32 x, int32 y)` (stub 0x4072A0 → 0x156C6B0): `(y & 15) * 16 + (x & 15)`. |
| 0x4072c0 | CWorld::GetLodPtrList | gta-reversed | 0 | cworld::get_lod_ptr_list | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CWorld::GetLodPtrList(int32 x, int32 y)` (0x4072C0): `y * 30 + x`, unchecked. |
| 0x407410 | CCheat::IsZoneStreamingAllowed | gta-reversed | 0 | cheat::is_zone_streaming_allowed | L2 | L2 split grove:1000 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1000 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1000 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::IsZoneStreamingAllowed` (stub 0x407410 → 0x1563A60): false while any of the cheats 0x27..0x2A, 0x2C, 0x2D, 0x46, 0x4B is active. |
| 0x41bc70 | CBridge::Init | gta-reversed | 0 | misc::bridge_init | ported-unhookable | lone `ret`: too short for the 5-byte entry patch |  |
| 0x41bc80 | CBridge::Update | gta-reversed | 0 | misc::bridge_update | ported-unhookable | lone `ret`: too short for the 5-byte entry patch |  |
| 0x41bc90 | CBridge::ShouldLightsBeFlashing | gta-reversed | 0 | misc::bridge_should_lights_be_flashing | ported-unhookable | `xor al,al; ret` (3 bytes): too short for the 5-byte entry patch |  |
| 0x41bca0 | CBridge::FindBridgeEntities | gta-reversed | 0 | misc::bridge_find_bridge_entities | ported-unhookable | lone `ret`: too short for the 5-byte entry patch |  |
| 0x438370 | CCheat::ApplyCheat | gta-reversed | 0 | cheat::apply_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::ApplyCheat(eCheats)` (0x438370): the cheat's function (tail jump) or a toggle. |
| 0x438450 | CCheat::ResetCheats | gta-reversed | 0 | cheat::reset_cheats | L2 | L2 split grove:2 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:4 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:2 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::ResetCheats` (stub 0x438450 → 0x1560F40): every cheat inactive, `ReleaseWeather`, time scale 1.0, empty cheat string, `m_bHasPlayerCheated` = 0. |
| 0x438480 | CCheat::AddToCheatString | gta-reversed | 0 | cheat::add_to_cheat_string | L2-split (undeclared writes to review) | L2 split cheats:118 compared, 0 mismatches, 117 undeclared (verdict PASS) | `CCheat::AddToCheatString(char)` (0x438480): not during a cutscene; shifts the string, puts the key first, then tries the hashes (`CKeyGen::GetKey`) of the longest prefix down to 6 characters against the 92 cheat keys. On a match: help text CHEAT1 (was off) / CHEAT8 (was on), stat 0x89 += 1, `m_bHas |
| 0x4385b0 | CCheat::WeaponCheat1 | REA | 0 | — | deferred | cheat body (723 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x438890 | CCheat::WeaponCheat2 | REA | 0 | — | deferred | cheat body (671 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x438b30 | CCheat::WeaponCheat3 | REA | 0 | — | deferred | cheat body (545 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x438e40 | CCheat::MoneyArmourHealthCheat | REA | 0 | — | deferred | cheat body (301 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x438e90 | CCheat::WantedLevelUpCheat | REA | 0 | cheat::wanted_level_up_cheat | L2-split (undeclared writes to review) | L2 split cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `CCheat::WantedLevelUpCheat` (0x438E90): `min(level + 2, 6)` (level re-read for the call). |
| 0x438f20 | CCheat::WantedLevelDownCheat | REA | 0 | cheat::wanted_level_down_cheat | L2 | L2 split cheats:2 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::WantedLevelDownCheat` (0x438F20): wanted level 0. |
| 0x438f40 | CCheat::SunnyWeatherCheat | REA | 0 | cheat::sunny_weather_cheat | L2 | L2 split cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::SunnyWeatherCheat` (0x438F40): `ForceWeatherNow(1)`. |
| 0x438f50 | CCheat::ExtraSunnyWeatherCheat | REA | 0 | cheat::extra_sunny_weather_cheat | L2 | L2 split cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::ExtraSunnyWeatherCheat` (0x438F50): `ForceWeatherNow(0)`. |
| 0x438f60 | CCheat::CloudyWeatherCheat | REA | 0 | cheat::cloudy_weather_cheat | L2 | L2 split cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::CloudyWeatherCheat` (0x438F60): `ForceWeatherNow(4)`. |
| 0x438f70 | CCheat::RainyWeatherCheat | REA | 0 | cheat::rainy_weather_cheat | L2 | L2 split cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::RainyWeatherCheat` (0x438F70): `ForceWeatherNow(16)`. |
| 0x438f80 | CCheat::FoggyWeatherCheat | REA | 0 | cheat::foggy_weather_cheat | L2 | L2 split cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::FoggyWeatherCheat` (0x438F80): `ForceWeatherNow(9)`. |
| 0x438f90 | CCheat::FastTimeCheat | REA | 0 | cheat::fast_time_cheat | L2 | L2 split cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::FastTimeCheat` (0x438F90): time scale doubled while below 4.0. |
| 0x438fc0 | CCheat::SlowTimeCheat | REA | 0 | cheat::slow_time_cheat | L2 | L2 split cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::SlowTimeCheat` (0x438FC0): time scale halved while above 0.25. |
| 0x438ff0 | CCheat::SuicideCheat | REA | 0 | — | deferred | cheat body (221 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x4390d0 | CCheat::PinkCarsCheat | REA | 0 | cheat::pink_cars_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::PinkCarsCheat` (0x4390D0): toggle; when on, black cars off. |
| 0x4390f0 | CCheat::BlackCarsCheat | REA | 0 | cheat::black_cars_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::BlackCarsCheat` (0x4390F0): toggle; when on, pink cars off. |
| 0x439110 | CCheat::FatCheat | REA | 0 | cheat::fat_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::FatCheat` (0x439110): stat 0x15 = 1000, rebuild the player. |
| 0x439150 | CCheat::MuscleCheat | REA | 0 | cheat::muscle_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::MuscleCheat` (0x439150): stat 0x17 = 1000, rebuild the player. |
| 0x439190 | CCheat::SkinnyCheat | REA | 0 | cheat::skinny_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::SkinnyCheat` (0x439190): stats 0x15, 0x17 = 0, rebuild the player. |
| 0x4391d0 | CCheat::ElvisLivesCheat | REA | 0 | — | deferred | cheat body (89 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439230 | CCheat::BeachPartyCheat | REA | 1 | — | deferred | cheat body (314 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439360 | CCheat::GangsCheat | REA | 0 | cheat::gangs_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::GangsCheat` (0x439360): toggle 0x96915A; when on, `StreamPedsIntoRandomSlots` with the gang models 0x66..0x6A, 0x6C..0x6E. |
| 0x4393d0 | CCheat::GangLandCheat | REA | 0 | cheat::gang_land_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::GangLandCheat` (0x4393D0): toggle, copied to `CGangWars::bGangWarsActive`... (0xC0FCB3). |
| 0x4393f0 | CCheat::LoveConquersAllCheat | REA | 0 | — | deferred | cheat body (183 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x4394b0 | CCheat::AllCarsAreShitCheat | REA | 0 | cheat::all_cars_are_shit_cheat | L2-split (undeclared writes to review) | L2 split cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `CCheat::AllCarsAreShitCheat` (0x4394B0): toggle 0x96915E; when on, `HandleSpecialCheats(0x2E)` and `CStreaming::ReclassifyLoadedCars` (tail jump). |
| 0x4394e0 | CCheat::AllCarsAreGreatCheat | REA | 0 | cheat::all_cars_are_great_cheat | L2-split (undeclared writes to review) | L2 split cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `CCheat::AllCarsAreGreatCheat` (0x4394E0): toggle 0x96915F; when on, `HandleSpecialCheats(0x2F)` and `CStreaming::ReclassifyLoadedCars` (tail jump). |
| 0x439510 | CCheat::MidnightCheat | REA | 0 | cheat::midnight_cheat | L2 | L2 split cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::MidnightCheat` (0x439510): toggle; when on, the clock (0xB70150..0xB70153) = 00:00:00. |
| 0x439540 | CCheat::DuskCheat | REA | 0 | cheat::dusk_cheat | L2 | L2 split cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::DuskCheat` (0x439540): toggle; when on, the clock = 21:00:00. |
| 0x439570 | CCheat::StormCheat | REA | 0 | cheat::storm_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::StormCheat` (0x439570): `ForceWeatherNow(16)`, rain 1.0 (0xC81324), wet 1.0 (0xC812F0). |
| 0x439590 | CCheat::SandstormCheat | REA | 0 | cheat::sandstorm_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::SandstormCheat` (0x439590): `ForceWeatherNow(19)`. |
| 0x4395a0 | CCheat::PredatorCheat | REA | 0 | cheat::predator_cheat | ported-unhookable | lone `ret`: too short for the 5-byte entry patch |  |
| 0x4395b0 | CCheat::ParachuteCheat | REA | 0 | — | deferred | cheat body (65 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439600 | CCheat::JetpackCheat | REA | 0 | — | deferred | cheat body (190 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x4396c0 | CCheat::NotWantedCheat | REA | 0 | cheat::not_wanted_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::NotWantedCheat` (0x4396C0): wanted level 0, toggle 0x969171. |
| 0x4396f0 | CCheat::WantedCheat | REA | 0 | cheat::wanted_cheat | L2-split (undeclared writes to review) | L2 split cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `CCheat::WantedCheat` (0x4396F0): wanted level 6. |
| 0x439710 | CCheat::RiotCheat | REA | 0 | cheat::riot_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::RiotCheat` (0x439710): toggle 0x969175. |
| 0x439720 | CCheat::FunhouseCheat | REA | 1 | — | deferred | cheat body (352 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439880 | CCheat::AdrenalineCheat | REA | 1 | — | deferred | cheat body (66 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x4398d0 | CCheat::DrivebyCheat | REA | 0 | — | deferred | cheat body (84 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439930 | CCheat::StaminaCheat | REA | 0 | cheat::stamina_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::StaminaCheat` (0x439930): stat 0x16 = 1000. |
| 0x439940 | CCheat::WeaponSkillsCheat | REA | 0 | cheat::weapon_skills_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::WeaponSkillsCheat` (0x439940): stats 0x45..0x4F = 1000. |
| 0x4399d0 | CCheat::VehicleSkillsCheat | REA | 0 | cheat::vehicle_skills_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::VehicleSkillsCheat` (0x4399D0): stats 0xA0, 0xDF, 0xE5, 0xE6 = 1000. |
| 0x439a10 | CCheat::HandleSpecialCheats | gta-reversed | 0 | — | deferred | cheat body (107 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439af0 | CCheat::DoCheats | gta-reversed | 0 | cheat::do_cheats | L2 | L2 split grove:1000 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1000 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1000 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CCheat::DoCheats` (stub 0x439AF0 → 0x1566D70): `AddToCheatString(k)` for every standard key just pressed (new state != 0, old state == 0), k = 0..255. |
| 0x439b20 | CCheat::MayhemCheat | REA | 3 | — | deferred | cheat body (333 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439c70 | CCheat::EverybodyAttacksPlayerCheat | REA | 1 | — | deferred | cheat body (259 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439d80 | CCheat::BlowUpCarsCheat | REA | 1 | — | deferred | cheat body (68 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439dd0 | CCheat::VillagePeopleCheat | REA | 0 | — | deferred | cheat body (124 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439e50 | CCheat::NinjaCheat | REA | 0 | — | deferred | cheat body (272 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x439f60 | CCheat::CountrysideInvasionCheat | REA | 0 | — | deferred | cheat body (323 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x43a0b0 | CCheat::VehicleCheat | gta-reversed | 2 | — | deferred | cheat body (970 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x43a4a0 | CCheat::TankCheat | REA | 0 | cheat::tank_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::TankCheat` (0x43A4A0): `VehicleCheat(0x1b0)`. |
| 0x43a4b0 | CCheat::StockCarCheat | REA | 0 | cheat::stock_car_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::StockCarCheat` (0x43A4B0): `VehicleCheat(0x1f8)`. |
| 0x43a4c0 | CCheat::StockCar2Cheat | REA | 0 | cheat::stock_car2_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::StockCar2Cheat` (0x43A4C0): `VehicleCheat(0x1f9)`. |
| 0x43a4d0 | CCheat::StockCar3Cheat | REA | 0 | cheat::stock_car3_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::StockCar3Cheat` (0x43A4D0): `VehicleCheat(0x1f6)`. |
| 0x43a4e0 | CCheat::StockCar4Cheat | REA | 0 | cheat::stock_car4_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::StockCar4Cheat` (0x43A4E0): `VehicleCheat(0x1f7)`. |
| 0x43a4f0 | CCheat::HearseCheat | REA | 0 | cheat::hearse_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::HearseCheat` (0x43A4F0): `VehicleCheat(0x1ba)`. |
| 0x43a500 | CCheat::LovefistCheat | REA | 0 | cheat::lovefist_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::LovefistCheat` (0x43A500): `VehicleCheat(0x199)`. |
| 0x43a510 | CCheat::TrashmasterCheat | REA | 0 | cheat::trashmaster_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::TrashmasterCheat` (0x43A510): `VehicleCheat(0x198)`. |
| 0x43a520 | CCheat::GolfcartCheat | REA | 0 | cheat::golfcart_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::GolfcartCheat` (0x43A520): `VehicleCheat(0x1c9)`. |
| 0x43a530 | CCheat::FlyboyCheat | REA | 0 | cheat::flyboy_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::FlyboyCheat` (0x43A530): `VehicleCheat(0x208)`. |
| 0x43a540 | CCheat::VortexCheat | REA | 0 | cheat::vortex_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::VortexCheat` (0x43A540): `VehicleCheat(0x21b)`. |
| 0x43a550 | CCheat::ApacheCheat | REA | 0 | cheat::apache_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::ApacheCheat` (0x43A550): `VehicleCheat(0x1a9)`. |
| 0x43a560 | CCheat::QuadCheat | REA | 0 | cheat::quad_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::QuadCheat` (0x43A560): `VehicleCheat(0x1d7)`. |
| 0x43a570 | CCheat::TankerCheat | REA | 0 | — | deferred | cheat body (231 bytes) creating peds/vehicles/weapons/events through unported PED/VEH/AI/WEAPON code (SEH frames); port with those subsystems |  |
| 0x43a660 | CCheat::DozerCheat | REA | 0 | cheat::dozer_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::DozerCheat` (0x43A660): `VehicleCheat(0x1e6)`. |
| 0x43a670 | CCheat::StuntPlaneCheat | REA | 0 | cheat::stunt_plane_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::StuntPlaneCheat` (0x43A670): `VehicleCheat(0x201)`. |
| 0x43a680 | CCheat::MonsterTruckCheat | REA | 0 | cheat::monster_truck_cheat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CCheat::MonsterTruckCheat` (0x43A680): `VehicleCheat(0x22c)`. |
| 0x43e400 | CEntryExitManager::WeAreInInteriorTransition | REA | 0 | entryexit::we_are_in_interior_transition | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CEntryExitManager::WeAreInInteriorTransition` (stub 0x43E400 → 0x1560F30): `ms_exitEnterState != 0`. |
| 0x43e410 | CEntryExitManager::AddEntryExitToStack | gta-reversed | 0 | — | deferred | no-CD protection chunk in the body computes a load/store address or value at run time (pushfd/rol/sbb/xchg sequences); needs a runtime measurement (PLAN §4b) before porting |  |
| 0x43e460 | CEntryExit::IsInArea | gta-reversed | 0 | — | deferred | not ported in part 1 (481 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43e650 | CEntryExit::GetEntryExitToDisplayNameOf | gta-reversed | 1 | — | deferred | no-CD protection chunk in the body computes a load/store address or value at run time (pushfd/rol/sbb/xchg sequences); needs a runtime measurement (PLAN §4b) before porting |  |
| 0x43e690 | CEntryExit::RequestObjectsInFrustum | gta-reversed | 0 | — | deferred | not ported in part 1 (49 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43e6d0 | CEntryExit::RequestAmbientPeds | gta-reversed | 0 | — | deferred | not ported in part 1 (465 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43e8b0 | CEntryExit::GenerateAmbientPeds | gta-reversed | 0 | — | deferred | not ported in part 1 (223 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43e990 | CEntryExit::ProcessStealableObjects | gta-reversed | 0 | — | deferred | not ported in part 1 (112 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43ea00 | CEntryExit::GetPositionRelativeToOutsideWorld | gta-reversed | 0 | entryexit::get_position_relative_to_outside_world | L1 | L1 1,060,000 cases, 0 mismatches, 50,487 SNaN cases unverified (native off) | `CEntryExit::GetPositionRelativeToOutsideWorld(CVector&)` (0x43EA00, thiscall, `ret 4`): when the entry-exit (or its link) has an area code, `pos += entrance centre (z: entrance z) - exit position`, with the f32 roundings of the bytes. The original also stores the z sum into its own stack argument s |
| 0x43ea90 | IsTeleportPointValid | REA | 0 | entryexit::is_teleport_point_valid | implemented-needs-L2 | no call in the recordings (NOCALLS) | `IsTeleportPointValid(const CVector& from, const CVector& to)` (0x43EA90): no world collision in a 0.35 sphere at `to` (`TestSphereAgainstWorld(to, 0.35, null, 1,1,1,1,1, 0)` returns null) and a clear line of sight `from` → `to` (`GetIsLineOfSightClear(.., 1,1,0,1,1,0,0)`). |
| 0x43eaf0 | CEntryExit::FindValidTeleportPoint | gta-reversed | 0 | — | deferred | orchestrator (512 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x43ecf0 | CEntryExitManager::SetAreaCodeForVisibleObjects | gta-reversed | 0 | — | deferred | not ported in part 1 (143 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43ed80 | CEntryExitManager::ResetAreaCodeForVisibleObjects | gta-reversed | 0 | — | deferred | not ported in part 1 (53 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43ef00 | CEntryExitManager::GetEntryExit | REA | 0 | entryexit::get_entry_exit | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CEntryExitManager::GetEntryExit(int32)` (stub 0x43EF00 → 0x156DA90): pool slot or null. |
| 0x43ef20 | CEntryExitManager::SetEntryExitFlag | REA | 0 | entryexit::set_entry_exit_flag | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CEntryExitManager::SetEntryExitFlag(const char* name, uint16 flag, bool set)` (stub 0x43EF20 → 0x15615C0): for every used slot (pool size read once, the pool pointer re-read) whose name matches (`_strnicmp(entry, name, 8)`), sets or clears the flag bits. |
| 0x43efd0 | CEntryExitManager::GetEntryExitIndex | gta-reversed | 1 | — | deferred | no-CD protection chunk in the body computes a load/store address or value at run time (pushfd/rol/sbb/xchg sequences); needs a runtime measurement (PLAN §4b) before porting |  |
| 0x43f050 | CEntryExitManager::LinkEntryExit | gta-reversed | 0 | entryexit::link_entry_exit | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CEntryExitManager::LinkEntryExit(CEntryExit*)` (0x43F050): the entry-exit with the same name and flag 4 (`GetEntryExitIndex(e, 4, 0)`) becomes `e`'s link; `e` becomes its link when it has none; its times are set to 0..24. |
| 0x43f0a0 | CEntryExitManager::PostEntryExitsCreation | gta-reversed | 1 | — | deferred | not ported in part 1 (141 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43f150 | CEntryExitManager::GetPositionRelativeToOutsideWorld | gta-reversed | 0 | entryexit::manager_get_position_relative_to_outside_world | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `CEntryExitManager::GetPositionRelativeToOutsideWorld(CVector&)` (stub 0x43F150 → 0x1566880): the stack entries `posn - 1` down to 0. |
| 0x43f180 | CEntryExitManager::EnableBurglaryHouses | gta-reversed | 1 | — | deferred | not ported in part 1 (102 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43f1f0 | CEntryExit::WarpGangWithPlayer | gta-reversed | 1 | — | deferred | orchestrator (702 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x43f4b0 | CEntryExitManager::FindNearestEntryExit | gta-reversed | 0 | — | deferred | not ported in part 1 (370 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43f630 | CEntryExitManager::FindNearestDoor | gta-reversed | 0 | — | deferred | not ported in part 1 (228 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43f7d0 | thunk_FUN_015651d0 | REA | 0 | — | deferred | unnamed / object-side helper (catalogue unknown or CObject); belongs to the CObject/CEntity pass |  |
| 0x43f880 | CEntryExitManager::Init | gta-reversed | 0 | — | deferred | loader / initialiser using CRT sscanf/_strupr and the file loader; next pass (needs an L2 run covering the load) |  |
| 0x43f9b0 | CEntryExitManager::SetEnabledByName | REA | 0 | entryexit::set_enabled_by_name | L2 | L2 grove:42/42, grove2:42/42, pr:84/84, cheats:42/42 (0 mismatches, 0 undeclared) | `CEntryExitManager::SetEnabledByName(const char*, bool)` (stub 0x43F9B0 → 0x156AE10): `SetEntryExitFlag(name, 0x4000, enable)`. |
| 0x43fa00 | CEntryExitManager::AddOne | gta-reversed | 2 | — | deferred | orchestrator (841 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x43fd50 | CEntryExitManager::DeleteOne | gta-reversed | 1 | — | deferred | not ported in part 1 (97 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x43ffd0 | CEntryExit::TransitionStarted | gta-reversed | 0 | — | deferred | orchestrator (1226 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x4404a0 | CEntryExit::TransitionFinished | REA | 0 | — | deferred | orchestrator (1772 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x440b90 | CEntryExitManager::Shutdown | gta-reversed | 1 | — | deferred | not ported in part 1 (192 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x440c40 | CEntryExitManager::ShutdownForRestart | gta-reversed | 1 | — | deferred | not ported in part 1 (207 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x440d10 | CEntryExitManager::Update | gta-reversed | 0 | — | deferred | orchestrator (909 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x441210 | CGameLogic::InitAtStartOfGame | gta-reversed | 0 | gamelogic::init_at_start_of_game | L2 | L2 grove:1/1, grove2:1/1, pr:2/2, cheats:1/1 (0 mismatches, 0 undeclared) | `CGameLogic::InitAtStartOfGame` (stub 0x441210 → 0x1563910). |
| 0x441240 | CGameLogic::ForceDeathRestart | gta-reversed | 0 | gamelogic::force_death_restart | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGameLogic::ForceDeathRestart` (stub 0x441240 → 0x15672E0): the focus player's state byte (+0xDC) = 1, `GameState` = 1, `TimeOfLastEvent` = now - 3001, fade colour black, `Fade(4.0, 0)`. |
| 0x441290 | CGameLogic::StopPlayerMovingFromDirection | gta-reversed | 0 | — | deferred | not ported in part 1 (249 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x441390 | CGameLogic::IsCoopGameGoingOn | gta-reversed | 0 | — | deferred | not ported in part 1 (32 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x4413c0 | CGameLogic::Remove2ndPlayerIfPresent | gta-reversed | 0 | — | deferred | not ported in part 1 (115 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x441440 | CGameLogic::SortOutStreamingAndMemory | gta-reversed | 0 | — | deferred | not ported in part 1 (120 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x4414c0 | CGameLogic::PassTime | gta-reversed | 0 | — | deferred | not ported in part 1 (146 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x441560 | CGameLogic::ClearSkip | gta-reversed | 0 | — | deferred | no-CD protection chunk in the body computes a load/store address or value at run time (pushfd/rol/sbb/xchg sequences); needs a runtime measurement (PLAN §4b) before porting |  |
| 0x4415c0 | CGameLogic::SkipCanBeActivated | gta-reversed | 0 | — | deferred | not ported in part 1 (308 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x4416c0 | CGameLogic::IsSkipWaitingForScriptToFadeIn | gta-reversed | 0 | gamelogic::is_skip_waiting_for_script_to_fade_in | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGameLogic::IsSkipWaitingForScriptToFadeIn` (stub 0x4416C0 → 0x1569C30): state 5 → 0 and true. |
| 0x4416e0 | CGameLogic::IsPointWithinLineArea | gta-reversed | 0 | gamelogic::is_point_within_line_area | L1 | L1 1,060,000 cases, 0 mismatches, 91,957 SNaN cases unverified (native off); L2 grove:686/686, grove2:686/686, pr:414/414, cheats:420/420 | `CGameLogic::IsPointWithinLineArea(const CVector* pts, int32 n, float x, float y)` (0x4416E0): counts the edges `k → (k+1) % n` crossed by the segment from (x, y) along (1e6, 0) (`Test2DLineAgainst2DLine`); the parity is the result (EAX). |
| 0x441770 | CGameLogic::SetPlayerWantedLevelForForbiddenTerritories | gta-reversed | 0 | — | deferred | not ported in part 1 (365 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x4418e0 | CGameLogic::CalcDistanceToForbiddenTrainCrossing | gta-reversed | 2 | — | deferred | orchestrator (647 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x441b70 | CGameLogic::FindCityClosestToPoint | gta-reversed | 2 | gamelogic::find_city_closest_to_point | L1 | L1 1,003,364 cases, 0 mismatches, 6,892 SNaN cases unverified (native off); L2 cheats:1/1 | `CGameLogic::FindCityClosestToPoint(float x, float y)` (0x441B70): 1 (LS, 1670/-1137... centres at 0x85984C/0x859848), 3 (0x859844/0x859840) when strictly closer (its distance kept as f32), 2 (0x85983C/0x859838) when strictly closer than the best. |
| 0x441c10 | CGameLogic::LaRiotsActiveHere | gta-reversed | 0 | — | deferred | not ported in part 1 (227 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x441d00 | CGameLogic::StorePedsWeapons | gta-reversed | 0 | gamelogic::store_peds_weapons | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGameLogic::StorePedsWeapons(CPed*)` (0x441D00): copies the 13 weapons (+0x5A0, 0x1C each). |
| 0x441d30 | CGameLogic::RestorePedsWeapons | gta-reversed | 1 | — | deferred | not ported in part 1 (125 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x441e10 | CGameLogic::IsPlayerAllowedToGoInThisDirection | gta-reversed | 0 | — | deferred | orchestrator (536 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x442020 | CGameLogic::IsPlayerUse2PlayerControls | gta-reversed | 0 | — | deferred | no-CD protection chunk in the body computes a load/store address or value at run time (pushfd/rol/sbb/xchg sequences); needs a runtime measurement (PLAN §4b) before porting |  |
| 0x442060 | CGameLogic::RestorePlayerStuffDuringResurrection | gta-reversed | 0 | — | deferred | orchestrator (857 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x4423c0 | CGameLogic::SetUpSkip | gta-reversed | 0 | gamelogic::set_up_skip | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGameLogic::SetUpSkip(CVector pos, float heading, bool afterMission, CEntity* vehicle, bool finishedByScript)` (stub 0x4423C0 → 0x1563E90): ends a pending fade-out skip (state 2: black fade in 0.5 s), clears pad 0's +0x10F bit 0, then stores the skip (state 4 after a mission, else 1), the time, the |
| 0x442480 | CGameLogic::UpdateSkip | gta-reversed | 0 | — | deferred | orchestrator (1051 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x4428b0 | CGameLogic::DoWeaponStuffAtStartOf2PlayerGame | gta-reversed | 0 | — | deferred | not ported in part 1 (273 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x442980 | CGameLogic::ResetStuffUponResurrection | gta-reversed | 0 | — | deferred | not ported in part 1 (322 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x442ad0 | CGameLogic::Update | gta-reversed | 2 | — | deferred | orchestrator (3572 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x4470e0 | CGarage::CGarage | REA | 0 | garages::ctor | L2 | L2 grove:50/50, grove2:50/50, pr:50/50, cheats:50/50 (0 mismatches, 0 undeclared) | `CGarage::CGarage` (0x4470E0, thiscall): +0x60 = 0, audio vtable 0x85998C, +0xCC..+0xD4 = 0; EAX = this. |
| 0x447110 | CGarage::~CGarage | REA | 0 | garages::dtor | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGarage::~CGarage` (0x447110, thiscall): audio entity vtable 0x859988, then `~CAESound` of +0x58. |
| 0x447120 | CGarages::Init | gta-reversed | 0 | garages::init | L2 | L2 grove:1/1, grove2:1/1, pr:1/1, cheats:1/1 (0 mismatches, 0 undeclared) | `CGarages::Init` (0x447120): the globals of the bytes, then model index 0 of every stored car (`aCarsInSafeHouse`, 20 per hideout, 4 rows of 0x40 bytes). |
| 0x4471b0 | CGarages::Shutdown | gta-reversed | 0 | garages::shutdown | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGarages::Shutdown` (stub 0x4471B0 → 0x1566E00): `NumGarages` = 0, `Reset` of all 50 door audio entities. |
| 0x4471e0 | CGarages::AddOne | REA | 0 | — | deferred | orchestrator (1044 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x447600 | CGarage::InitDoorsAtStart | REA | 0 | garages::init_doors_at_start | L2 | L2 grove:50/50, grove2:50/50, pr:100/100, cheats:50/50 (0 mismatches, 0 undeclared) | `CGarage::InitDoorsAtStart` (0x447600, thiscall): flags (+0x4E & 0x39) / 0x40, door state 0, +0x3C = 0; then by type: 1, 6..12, 14..45 closed; 2..5 open (1.0); others unchanged. |
| 0x447680 | CGarages::GetGarageNumberByName | gta-reversed | 0 | garages::get_garage_number_by_name | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `CGarages::GetGarageNumberByName(const char*)` (stub 0x447680 → 0x156DCF0): `_stricmp` against the names; -1 when none. |
| 0x4476d0 | CGarages::ChangeGarageType | gta-reversed | 0 | garages::change_garage_type | L2 | L2 grove:7/7 undecl 7, grove2:7/7, pr:14/14, cheats:7/7 (0 mismatches, 0 undeclared) | `CGarages::ChangeGarageType(int16, uint8 type, uint32)` (stub 0x4476D0 → 0x1561690). |
| 0x447720 | thunk_FUN_01565260 | REA | 0 | garages::vehicle_suits_garage | L2 | L2 grove:750/750, grove2:750/750, pr:1000/1001, cheats:750/750 (0 mismatches, 0 undeclared) | `thunk_FUN_01565260` (stub 0x447720 → 0x1565260, thiscall `CGarage*`, `ret 4`, called from `CGarage::Update`): whether the vehicle's handling flags (+0xD0) suit the garage type 0x24 (bit 25 set), 0x25 (bit 26 set), 0x26 (bits 25, 26 clear); false for other types or no vehicle. |
| 0x447790 | CGarages::PrintMessages | gta-reversed | 0 | — | deferred | orchestrator (513 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x4479a0 | CGarages::IsCarSprayable | gta-reversed | 1 | — | deferred | not ported in part 1 (75 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x4479f0 | CGarage::BuildRotatedDoorMatrix | gta-reversed | 0 | — | deferred | not ported in part 1 (389 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x447b80 | CGarages::TriggerMessage | gta-reversed | 0 | — | deferred | no-CD protection chunk in the body computes a load/store address or value at run time (pushfd/rol/sbb/xchg sequences); needs a runtime measurement (PLAN §4b) before porting |  |
| 0x447c40 | CGarages::SetTargetCarForMissionGarage | gta-reversed | 1 | — | deferred | not ported in part 1 (75 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x447cb0 | CGarages::DeActivateGarage | gta-reversed | 0 | garages::deactivate_garage | L2 | L2 grove:20/20 undecl 20, grove2:20/20, pr:40/40, cheats:20/20 (0 mismatches, 0 undeclared) | `CGarages::DeActivateGarage(int16)` (stub 0x447CB0 → 0x1563E70): flags /= 2. |
| 0x447cd0 | CGarages::ActivateGarage | gta-reversed | 0 | garages::activate_garage | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGarages::ActivateGarage(int16)` (stub 0x447CD0 → 0x15677F0): flags &= ~2; a type 0xB garage with a closed door starts opening (3). |
| 0x447d00 | CGarages::IsGarageOpen | gta-reversed | 0 | garages::is_garage_open | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `CGarages::IsGarageOpen(int16)` (stub 0x447D00 → 0x156D2D0): door state 1 or 4. |
| 0x447d30 | CGarages::IsGarageClosed | gta-reversed | 0 | garages::is_garage_closed | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGarages::IsGarageClosed(int16)` (stub 0x447D30 → 0x1560F70): door state 0. |
| 0x447d50 | CGarage::OpenThisGarage | gta-reversed | 0 | garages::open_this_garage | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGarage::OpenThisGarage` (stub 0x447D50 → 0x1567910, thiscall): door 0, 2, 5 → 3 (opening). |
| 0x447d70 | CGarage::CloseThisGarage | gta-reversed | 0 | garages::close_this_garage | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGarage::CloseThisGarage` (stub 0x447D70 → 0x156D5C0, thiscall): door 1, 3 → 2 (closing). |
| 0x447d80 | FUN_00447d80 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x448330 | FUN_00448330 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x448550 | thunk_FUN_0156d8d0 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x4485c0 | thunk_FUN_01561490 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x448650 | CGarages::CameraShouldBeOutside | REA | 0 | garages::camera_should_be_outside | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGarages::CameraShouldBeOutside` (stub 0x448650 → 0x1565080). |
| 0x448660 | CGarages::GivePlayerDetonator | REA | 0 | — | deferred | not ported in part 1 (91 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x4486c0 | CGarage::PlayerArrestedOrDied | REA | 0 | garages::player_arrested_or_died | L1 | L1 1,002,000 cases, 0 mismatches; L2 cheats:50/50 | `CGarage::PlayerArrestedOrDied` (0x4486C0, thiscall): types 2..5, 13: door 0, 2, 3 → 3; types 1, 6..12, 14..45: door 1..3 → 2; other types unchanged. |
| 0x448740 | CGarage::IsPointInsideGarage-CVector | REA | 0 | garages::is_point_inside_garage | L1 | L1 1,060,000 cases, 0 mismatches, 50,276 SNaN cases unverified (native off); L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 | `CGarage::IsPointInsideGarage(CVector)` (0x448740, thiscall, by value, `ret 0xC`). |
| 0x4487d0 | CGarage::IsPointInsideGarage-CVector-float | REA | 0 | garages::is_point_inside_garage_margin | L1 | L1 1,060,000 cases, 0 mismatches, 53,914 SNaN cases unverified (native off); L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 | `CGarage::IsPointInsideGarage(CVector, float margin)` (0x4487D0, thiscall, by value, `ret 0x10`). |
| 0x448890 | CGarages::StopCarFromBlowingUp | REA | 0 | — | deferred | not ported in part 1 (106 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x448900 | CGarages::IsPointWithinHideOutGarage | gta-reversed | 0 | — | deferred | not ported in part 1 (107 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x448990 | CGarages::IsPointWithinAnyGarage | REA | 0 | — | deferred | not ported in part 1 (83 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x4489f0 | CGarages::FindSafeHouseIndexForGarageType | REA | 0 | — | deferred | not ported in part 1 (136 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x448af0 | CGarages::IsModelIndexADoor | gta-reversed | 0 | garages::is_model_index_a_door | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `CGarages::IsModelIndexADoor(int32)` (0x448AF0): an atomic model info (slot 4, called twice) whose flags word +0x12 has bits 0x7800 == 0x3800. |
| 0x448b30 | CGarages::AllRespraysCloseOrOpen | gta-reversed | 0 | garages::all_resprays_close_or_open | L2 | L2 grove:1/1, grove2:1/1, pr:1/1, cheats:1/1 (0 mismatches, 0 undeclared) | `CGarages::AllRespraysCloseOrOpen(bool)` (stub 0x448B30 → 0x1563A10): door state of every type-5 garage = the flag. |
| 0x448b60 | CGarages::Init_AfterRestart | gta-reversed | 1 | — | deferred | not ported in part 1 (105 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x448be0 | CGarage::IsEntityEntirelyInside3D | REA | 0 | — | deferred | not ported in part 1 (333 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x448d30 | CGarage::IsEntityEntirelyOutside | REA | 0 | — | deferred | not ported in part 1 (281 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x448e50 | thunk_FUN_01569180 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x448ee0 | CGarage::IsEntityTouching3D | REA | 0 | — | deferred | not ported in part 1 (356 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x449050 | CGarage::EntityHasASphereWayOutsideGarage | REA | 0 | — | deferred | not ported in part 1 (162 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x449100 | thunk_FUN_01566680 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x449220 | FUN_00449220 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x4494f0 | thunk_FUN_0156d610 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x4495f0 | thunk_FUN_01561400 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x449690 | CGarage::RemoveCarsBlockingDoorNotInside | REA | 0 | — | deferred | not ported in part 1 (175 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x449900 | CGarage::StoreAndRemoveCarsForThisHideOut | gta-reversed | 0 | — | deferred | not ported in part 1 (319 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x449a50 | FUN_00449a50 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x449ba0 | CGarages::IsPointInAGarageCameraZone | REA | 0 | — | deferred | not ported in part 1 (93 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x449c50 | CGarage::TidyUpGarage | REA | 0 | — | deferred | not ported in part 1 (187 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x449d10 | CGarage::TidyUpGarageClose | REA | 0 | — | deferred | not ported in part 1 (326 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x449e60 | CGarages::PlayerArrestedOrDied | gta-reversed | 0 | — | deferred | not ported in part 1 (49 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x44a170 | CGarages::CloseHideOutGaragesBeforeSave | gta-reversed | 0 | — | deferred | not ported in part 1 (159 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x44a210 | CGarages::CountCarsInHideoutGarage | REA | 0 | — | deferred | not ported in part 1 (52 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x44a240 | CGarages::FindGarageForObject | REA | 0 | — | deferred | not ported in part 1 (370 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x44a3c0 | CGarages::StoreCarInNearestImpoundingGarage | REA | 0 | — | deferred | not ported in part 1 (263 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x44a660 | FUN_0044a660 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x44a750 | FUN_0044a750 | REA | 0 | — | deferred | unnamed callee of CGarage::Update (catalogue unknown); port with CGarage::Update |  |
| 0x44a830 | CGarage::IsStaticPlayerCarEntirelyInside | REA | 0 | — | deferred | not ported in part 1 (397 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x44a9c0 | CGarage::IsGarageEmpty | REA | 0 | — | deferred | not ported in part 1 (141 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x44aa50 | CGarage::Update | REA | 0 | — | deferred | orchestrator (7553 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x44c8c0 | CGarages::Update | gta-reversed | 1 | garages::update | L2-split (undeclared writes to review) | L2 split grove:1000 compared, 0 mismatches, 1 undeclared (verdict PASS), pr:1000 compared, 0 mismatches, 1 undeclared (verdict PASS), cheats:1000 compared, 0 mismatches, 1 undeclared (verdict PASS) | `CGarages::Update` (0x44C8C0): not during replay playback or a co-op game; camera-outside flag 0, two camera words reset (0xB6F844 saved to 0x96BFD8), `CGarage::Update(i)` of every garage with a type; every 16th frame (counter & 15 == 12) the next garage in turn is tidied: `TidyUpGarage` when the ca |
| 0x454920 | thunk_FUN_0156f730 | REA | 0 | pickups::is_type_3_4_5 | L1 | L1 1,002,000 cases, 0 mismatches | `thunk_FUN_0156f730` (stub 0x454920 → 0x156F730, called from `CPickup::Update`): true for 3, 4, 5. |
| 0x454960 | CPickup::SetPosn | gta-reversed | 0 | pickups::set_posn | L1 | L1 1,060,000 cases, 0 mismatches, 12,409 SNaN cases unverified (native off) | `CPickup::SetPosn(float x, float y, float z)` (0x454960, thiscall, `ret 0xC`): int16 of `_ftol(v * 8)`. |
| 0x4549a0 | CPickup::GetPosn | gta-reversed | 1 | pickups::get_posn | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:30/30 undecl 30, grove2:30/30, pr:30/30, cheats:18/18 | `CPickup::GetPosn(CVector* out)` (0x4549A0, thiscall, `ret 4`): `out = pos * 0.125`; returns `out`. |
| 0x454a70 | CPickups::Init | gta-reversed | 0 | pickups::init | L2 | L2 grove:1/1 undecl 1, grove2:1/1, pr:1/1, cheats:1/1 (0 mismatches, 0 undeclared) | `CPickups::Init` (stub 0x454A70 → 0x1564650): `NumMessages` = 0; every pickup: type 0, reference 1, object null; the collected array and its index 0; `DisplayHelpMessage` = 10. |
| 0x454ac0 | CPickups::ModelForWeapon | gta-reversed | 0 | pickups::model_for_weapon | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CPickups::ModelForWeapon(eWeaponType)` (stub 0x454AC0 → 0x1568BD0): `GetWeaponInfo(w, 1)->m_nModelId1`. |
| 0x454ae0 | CPickups::WeaponForModel | gta-reversed | 0 | pickups::weapon_for_model | L2 | L2 cheats:354/354 (0 mismatches, 0 undeclared) | `CPickups::WeaponForModel(int32 model)` (0x454AE0): 0x30 for the models at 0x8CD5C0 / 0x8CD5BC, 0x2F for the one at 0x8CD5C8 and 0x172; else (model != -1) the weapon type (+0x24) of a weapon model info (type 4); else 0. |
| 0x454b40 | CPickups::IsPickUpPickedUp | gta-reversed | 0 | pickups::is_pick_up_picked_up | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:468/468, cheats:1000/1001 (0 mismatches, 0 undeclared) | `CPickups::IsPickUpPickedUp(int32 handle)` (stub 0x454B40 → 0x156DA60): finds and clears the handle in the collected array. |
| 0x454b70 | CPickups::ModifyStringLabelForControlSetting | gta-reversed | 0 | — | deferred | not ported in part 1 (89 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x454be0 | CPickup::ExtractAmmoFromPickup | gta-reversed | 0 | — | deferred | not ported in part 1 (141 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x454c70 | CPickup::IsVisible | gta-reversed | 0 | pickups::is_visible | L1 | L1 1,060,000 cases, 0 mismatches, 6,189 SNaN cases unverified (native off); L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 | `CPickup::IsVisible` (0x454C70, thiscall): 2D distance camera → pickup below 100 (EAX 0/1). |
| 0x454cf0 | CPickup::GetRidOfObjects | gta-reversed | 0 | pickups::get_rid_of_objects | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CPickup::GetRidOfObjects` (0x454CF0, thiscall). |
| 0x454d20 | CPickup::PickUpShouldBeInvisible | gta-reversed | 0 | pickups::pick_up_should_be_invisible | L2 | L2 grove:71/71, grove2:71/71, pr:30/30, cheats:370/370 (0 mismatches, 0 undeclared) | `CPickup::PickUpShouldBeInvisible` (0x454D20, thiscall), see the module docs / spec. |
| 0x454e80 | CPickups::DoMoneyEffects | gta-reversed | 0 | — | deferred | not ported in part 1 (376 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x455000 | CPickups::RenderPickUpText | gta-reversed | 0 | — | deferred | not ported in part 1 (444 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x4551c0 | CPickups::FindPickUpForThisObject | gta-reversed | 0 | pickups::find_pick_up_for_this_object | L2 | L2 grove:41/41, grove2:41/41, cheats:352/352 (0 mismatches, 0 undeclared) | `CPickups::FindPickUpForThisObject(CObject*)` (stub 0x4551C0 → 0x156D870): the first pickup in use whose object is `obj`, else pickup 0. |
| 0x455200 | CPickups::PassTime | gta-reversed | 0 | pickups::pass_time | L2 | L2 cheats:1/1 (0 mismatches, 0 undeclared) | `CPickups::PassTime(uint32)` (0x455200): regeneration times of pickups whose type is neither 0 nor 0x10 decrease by `t`, floored at 0. |
| 0x455240 | CPickups::AddToCollectedPickupsArray | gta-reversed | 0 | pickups::add_to_collected_pickups_array | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CPickups::AddToCollectedPickupsArray(int32 index)` (stub 0x455240 → 0x1561580). |
| 0x455280 | CPickups::GetUniquePickupIndex | gta-reversed | 0 | pickups::get_unique_pickup_index | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CPickups::GetUniquePickupIndex(int32 index)` (0x455280). |
| 0x4552a0 | CPickups::GetActualPickupIndex | gta-reversed | 0 | pickups::get_actual_pickup_index | L2 | L2 grove:427/427, grove2:427/427, pr:841/841, cheats:427/427 (0 mismatches, 0 undeclared) | `CPickups::GetActualPickupIndex(int32 handle)` (stub 0x4552A0 → 0x1564BB0): the index, or -1. |
| 0x4552d0 | CPickups::PickedUpOyster | gta-reversed | 2 | — | deferred | not ported in part 1 (185 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x455390 | CPickups::PickedUpHorseShoe | gta-reversed | 3 | — | deferred | not ported in part 1 (217 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x455470 | CPickups::RemovePickupObjects | gta-reversed | 0 | pickups::remove_pickup_objects | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CPickups::RemovePickupObjects` (0x455470): every pickup in use with an object: object removed and deleted, flag bit 3 cleared. |
| 0x4554c0 | CPickups::PlayerCanPickUpThisWeaponTypeAtThisMoment | gta-reversed | 0 | pickups::player_can_pick_up_this_weapon_type_at_this_moment | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CPickups::PlayerCanPickUpThisWeaponTypeAtThisMoment(eWeaponType)` (stub 0x4554C0 → 0x156A950): true when weapon flag bit 1 (+0x18) is set or the player has no jetpack task. |
| 0x455500 | CPickup::FindTextIndexForString | gta-reversed | 0 | pickups::find_text_index_for_string | L2 | L2 grove:581/581, grove2:581/581, pr:1000/1001, cheats:581/581 (0 mismatches, 0 undeclared) | `CPickup::FindTextIndexForString(const char*)` (stub 0x455500 → 0x156EC50): null → 0, "PROP_3" → 1, "PROP_4" → 2 (case-insensitive), else 0. |
| 0x455540 | CPickup::FindStringForTextIndex | gta-reversed | 1 | pickups::find_string_for_text_index | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CPickup::FindStringForTextIndex(int32)` (stub 0x455540 → 0x1565750): 1 → "PROP_3", 2 → "PROP_4", else "FESZ_CA". |
| 0x4555a0 | CPickups::TryToMerge_WeaponType | gta-reversed | 0 | — | deferred | not ported in part 1 (216 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x455680 | CPickups::UpdateMoneyPerDay | gta-reversed | 0 | pickups::update_money_per_day | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CPickups::UpdateMoneyPerDay(int32 handle, uint16)` (stub 0x455680 → 0x156B360). |
| 0x4556c0 | CPickup::Remove | gta-reversed | 0 | pickups::remove | L2-split (undeclared writes to review) | L2 split grove:39 compared, 0 mismatches, 2 undeclared (verdict PASS), pr:65 compared, 0 mismatches, 4 undeclared (verdict PASS), cheats:42 compared, 0 mismatches, 3 undeclared (verdict PASS) | `CPickup::Remove` (0x4556C0, thiscall): radar blip of its handle cleared, object removed and deleted, type 0, flag bit 0 set. |
| 0x455720 | CPickups::DoPickUpEffects | REA | 0 | — | deferred | orchestrator (1789 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x455e20 | CPickups::DoCollectableEffects | gta-reversed | 0 | — | deferred | orchestrator (696 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x4560e0 | CPickups::DoMineEffects | gta-reversed | 0 | — | deferred | orchestrator (696 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x4563a0 | CPickups::RemoveUnnecessaryPickups | gta-reversed | 0 | — | deferred | not ported in part 1 (168 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x456450 | CPickups::TestForPickupsInBubble | gta-reversed | 0 | pickups::test_for_pickups_in_bubble | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CPickups::TestForPickupsInBubble(CVector pos, float radius)` (0x456450): any pickup (in use or not) whose 3D distance is below `radius`. |
| 0x4564f0 | CPickups::GivePlayerGoodiesWithPickUpMI | gta-reversed | 0 | — | deferred | orchestrator (755 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x4567e0 | CPickup::GiveUsAPickUpObject | gta-reversed | 0 | — | deferred | orchestrator (580 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x456a30 | CPickups::GetNewUniquePickupIndex | gta-reversed | 0 | pickups::get_new_unique_pickup_index | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CPickups::GetNewUniquePickupIndex(int32 index)` (0x456A30): reference + 1 (wrapping to 1 from 0xFFFE), the new handle. |
| 0x456a70 | CPickups::PictureTaken | gta-reversed | 1 | — | deferred | orchestrator (687 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x456d30 | CPickups::RemovePickUpsInArea | gta-reversed | 0 | — | deferred | not ported in part 1 (169 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x456de0 | CPickups::RemoveMissionPickUps | gta-reversed | 0 | pickups::remove_mission_pick_ups | L2 | L2 split grove:1 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CPickups::RemoveMissionPickUps` (0x456DE0): pickups of type 0x16 removed as `CPickup::Remove` does (inlined). |
| 0x456e60 | CPickups::ReInit | gta-reversed | 0 | — | deferred | not ported in part 1 (178 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x456f20 | CPickups::GenerateNewOne | REA | 0 | — | deferred | orchestrator (1118 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x457380 | CPickups::GenerateNewOne_WeaponType | gta-reversed | 0 | — | deferred | not ported in part 1 (80 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x4573d0 | CPickups::RemovePickUp | gta-reversed | 0 | pickups::remove_pick_up | L2-split (undeclared writes to review) | L2 split grove:54 compared, 0 mismatches, 26 undeclared (verdict PASS), pr:90 compared, 0 mismatches, 52 undeclared (verdict PASS), cheats:56 compared, 0 mismatches, 27 undeclared (verdict PASS) | `CPickups::RemovePickUp(int32 handle)` (0x4573D0): `CPickup::Remove` of a valid handle. |
| 0x457410 | CPickup::Update | gta-reversed | 3 | — | deferred | orchestrator (5199 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x4588b0 | CPickup::ProcessGunShot | gta-reversed | 0 | — | deferred | not ported in part 1 (225 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x458970 | CPickups::CreateSomeMoney | gta-reversed | 2 | — | deferred | not ported in part 1 (258 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x458a80 | CPickups::CreatePickupCoorsCloseToCoors- | REA | 0 | — | deferred | orchestrator (863 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x458de0 | CPickups::Update | gta-reversed | 0 | pickups::update | L2-split (undeclared writes to review) | L2 split grove:1000 compared, 0 mismatches, 10 undeclared (verdict PASS), pr:1000 compared, 0 mismatches, 10 undeclared (verdict PASS), cheats:1000 compared, 0 mismatches, 10 undeclared (verdict PASS) | `CPickups::Update` (0x458DE0): not during replay playback. A 1/32 slice of the pickups (by frame counter) updates visibility (flag bit 3; creates the object (`GiveUsAPickUpObject` + `CWorld::Add`) when visible and not disabled, removes it when not visible); the collect-button buffer and the weapon-p |
| 0x4590c0 | CPickups::DetonateMinesHitByGunShot | gta-reversed | 0 | — | deferred | not ported in part 1 (46 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x460630 | CRestart::Initialise | gta-reversed | 0 | restart::initialise | L2 | L2 grove:1/1, grove2:1/1, pr:2/2 undecl 1, pr2:2/2, cheats:1/1 (0 mismatches, 0 undeclared) | `CRestart::Initialise` (stub 0x460630 → 0x15658E0): headings and points of both tables 0 (the "when to use" tables are kept), counts 0, override position/heading 0, override flags 0, both fade flags 1, both extra radii 0. |
| 0x460730 | CRestart::AddHospitalRestartPoint | gta-reversed | 0 | restart::add_hospital_restart_point | L2 | L2 grove:8/8 undecl 8, grove2:8/8, pr:16/16, cheats:8/8 (0 mismatches, 0 undeclared) | `CRestart::AddHospitalRestartPoint(const CVector&, float, int32)` (0x460730); no bound check. |
| 0x460780 | CRestart::AddPoliceRestartPoint | gta-reversed | 0 | restart::add_police_restart_point | L2 | L2 grove:7/7 undecl 7, grove2:7/7, pr:14/14, cheats:7/7 (0 mismatches, 0 undeclared) | `CRestart::AddPoliceRestartPoint(const CVector&, float, int32)` (0x460780); no bound check. |
| 0x4607d0 | CRestart::OverrideNextRestart | gta-reversed | 0 | restart::override_next_restart | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CRestart::OverrideNextRestart(const CVector&, float)` (stub 0x4607D0 → 0x156B8F0). |
| 0x460800 | CRestart::CancelOverrideRestart | gta-reversed | 0 | restart::cancel_override_restart | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CRestart::CancelOverrideRestart` (0x460800). |
| 0x460810 | CRestart::SetRespawnPointForDurationOfMission | gta-reversed | 0 | restart::set_respawn_point_for_duration_of_mission | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CRestart::SetRespawnPointForDurationOfMission(CVector)` (stub 0x460810 → 0x156F610, by value). |
| 0x460840 | CRestart::ClearRespawnPointForDurationOfMission | gta-reversed | 0 | restart::clear_respawn_point_for_duration_of_mission | L2 | L2 grove:1/1, grove2:1/1, pr:1/1 (0 mismatches, 0 undeclared) | `CRestart::ClearRespawnPointForDurationOfMission` (stub 0x460840 → 0x15626A0). |
| 0x460850 | CRestart::FindClosestHospitalRestartPoint | gta-reversed | 0 | restart::find_closest_hospital_restart_point | L2 | L2 cheats:1/1 (0 mismatches, 0 undeclared) | `CRestart::FindClosestHospitalRestartPoint(CVector, CVector*, float*)` (0x460850). |
| 0x460a50 | CRestart::FindClosestPoliceRestartPoint | gta-reversed | 1 | restart::find_closest_police_restart_point | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CRestart::FindClosestPoliceRestartPoint(CVector, CVector*, float*)` (0x460A50). |
| 0x49aad0 | CShopping::GetPriceSectionFromName | gta-reversed | 1 | shopping::get_price_section_from_name | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CShopping::GetPriceSectionFromName(const char*)` (0x49AAD0): `_stricmp` against the 11 section names. |
| 0x49ab10 | CShopping::GetItemIndex | gta-reversed | 1 | shopping::get_item_index | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CShopping::GetItemIndex(uint32 key)` (stub 0x49AB10 → 0x156E330): the key's index or -1. |
| 0x49ab30 | CShopping::GetKey | gta-reversed | 0 | — | deferred | not ported in part 1 (115 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49abd0 | GetChangingStatIndex | REA | 0 | shopping::get_changing_stat_index | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `GetChangingStatIndex(const char*)` (stub 0x49ABD0 → 0x1565360): "-" → -1, "fat" 0, "respect" 1, "sexy" 2, "health" 3, "stamina" 4, "calories" 5, else -1 (`repe cmpsb` over the literal including its NUL). |
| 0x49ac90 | CShopping::RemoveLoadedPrices | gta-reversed | 0 | — | deferred | not ported in part 1 (51 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49ad20 | CShopping::FindItem | gta-reversed | 1 | shopping::find_item | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CShopping::FindItem(uint32 key)` (0x49AD20): index in the loaded prices or -1. |
| 0x49ad50 | CShopping::GetPrice | gta-reversed | 0 | shopping::get_price | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CShopping::GetPrice(uint32 key)` (0x49AD50): `_ftol(price * modifier[CTheZones::m_CurrLevel])` (index -1 reads the slot before the table, original). |
| 0x49ada0 | CShopping::GetNameTag | gta-reversed | 0 | shopping::get_name_tag | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CShopping::GetNameTag(uint32 key)` (stub 0x49ADA0 → 0x156F0D0): `&item.nameTag`. |
| 0x49ade0 | CShopping::GetExtraInfo | gta-reversed | 0 | shopping::get_extra_info | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CShopping::GetExtraInfo(uint32 key, int32 which)` (stub 0x49ADE0 → 0x156F080): extra 0 / 1, else 0. |
| 0x49ae30 | CShopping::RemoveLoadedShop | gta-reversed | 0 | — | deferred | not ported in part 1 (63 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49ae70 | CShopping::FindSection | gta-reversed | 0 | — | deferred | not ported in part 1 (161 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49af10 | CShopping::GetNextSection | gta-reversed | 0 | — | deferred | not ported in part 1 (141 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49af90 | CShopping::FindSectionInSection | gta-reversed | 0 | — | deferred | not ported in part 1 (49 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49afd0 | CShopping::IncrementStat2 | gta-reversed | 0 | — | deferred | not ported in part 1 (50 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49b010 | IsValidModForVehicle | REA | 0 | — | deferred | not ported in part 1 (240 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49b0e0 | FUN_0049b0e0 | REA | 0 | — | deferred | unnamed / object-side helper (catalogue unknown or CObject); belongs to the CObject/CEntity pass |  |
| 0x49b1f0 | CShopping::SetCurrentProperty | gta-reversed | 0 | shopping::set_current_property | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CShopping::SetCurrentProperty(int32)` (0x49B1F0). |
| 0x49b200 | CShopping::StoreClothesState | gta-reversed | 0 | shopping::store_clothes_state | L2 | L2 grove:1/1 undecl 1, grove2:1/1, pr:2/2, cheats:1/1 (0 mismatches, 0 undeclared) | `CShopping::StoreClothesState` (stub 0x49B200 → 0x156E2F0): copies the focus player's clothes description (0x78 bytes) aside; flag = 1. |
| 0x49b240 | CShopping::RestoreClothesState | gta-reversed | 0 | shopping::restore_clothes_state | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CShopping::RestoreClothesState` (stub 0x49B240 → 0x1565320): copies it back; flag = 0. |
| 0x49b280 | CShopping::StoreVehicleMods | gta-reversed | 0 | — | deferred | not ported in part 1 (259 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49b3c0 | CShopping::RestoreVehicleMods | gta-reversed | 0 | — | deferred | not ported in part 1 (483 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49b5e0 | CShopping::HasPlayerBought | gta-reversed | 0 | shopping::has_player_bought | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CShopping::HasPlayerBought(uint32 key)` (stub 0x49B5E0 → 0x156B770): the bought byte of the key (an unknown key reads the byte before the table). |
| 0x49b610 | CShopping::SetPlayerHasBought | gta-reversed | 0 | shopping::set_player_has_bought | L2 | L2 grove:4/4 undecl 4, grove2:4/4, pr:8/8, cheats:4/4 (0 mismatches, 0 undeclared) | `CShopping::SetPlayerHasBought(uint32 key)` (stub 0x49B610 → 0x156F3D0): bought byte = 1 (an unknown key writes the byte before the table, original). |
| 0x49b640 | CShopping::ShutdownForRestart | gta-reversed | 0 | — | deferred | not ported in part 1 (89 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49b6a0 | CShopping::LoadStats | gta-reversed | 0 | — | deferred | loader / initialiser using CRT sscanf/_strupr and the file loader; next pass (needs an L2 run covering the load) |  |
| 0x49b8d0 | CShopping::LoadPrices | gta-reversed | 0 | — | deferred | loader / initialiser using CRT sscanf/_strupr and the file loader; next pass (needs an L2 run covering the load) |  |
| 0x49bbe0 | CShopping::LoadShop | gta-reversed | 0 | — | deferred | loader / initialiser using CRT sscanf/_strupr and the file loader; next pass (needs an L2 run covering the load) |  |
| 0x49be70 | CShopping::IncrementStat | gta-reversed | 0 | — | deferred | not ported in part 1 (117 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49bef0 | CShopping::UpdateStats | gta-reversed | 0 | — | deferred | not ported in part 1 (124 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x49bf70 | CShopping::Buy | gta-reversed | 0 | — | deferred | orchestrator (751 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x49c290 | CShopping::Init | gta-reversed | 0 | — | deferred | loader / initialiser using CRT sscanf/_strupr and the file loader; next pass (needs an L2 run covering the load) |  |
| 0x49c3b0 | thunk_FUN_0156acc0 | REA | 0 | — | deferred | unnamed / object-side helper (catalogue unknown or CObject); belongs to the CObject/CEntity pass |  |
| 0x49c490 | CStuntJumpManager::Update | gta-reversed | 2 | — | deferred | orchestrator (1109 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x49ca50 | CStuntJumpManager::Init | gta-reversed | 0 | escalators::sj_init | L2 | L2 split grove:1 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CStuntJumpManager::Init` (stub 0x49CA50 → 0x156DDB0): `new` pool of 256 "Stunt Jumps" (null when `new` fails), enabled = 1. The SEH frame around the constructor is not reproduced. |
| 0x49cb10 | CStuntJumpManager::ShutdownForRestart | gta-reversed | 0 | escalators::sj_shutdown_for_restart | L2 | L2 split pr:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CStuntJumpManager::ShutdownForRestart` (stub 0x49CB10 → 0x156E3B0): pool `Clear`, state reset. |
| 0x49cb40 | CStuntJumpManager::AddOne | gta-reversed | 0 | escalators::sj_add_one | L2 | L2 split grove:70 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:140 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:70 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CStuntJumpManager::AddOne(const CBoundingBox& start, const CBoundingBox& end, const CVector& camera, int32 reward)` (stub 0x49CB40 → 0x1561A90): a new pool slot (-1 without a pool or a free slot) gets the boxes, camera and reward, done/found flags 0; `m_iNumJumps++`; returns the slot index. |
| 0x49cbc0 | CStuntJumpManager::Shutdown | gta-reversed | 0 | — | deferred | no-CD protection chunk in the body computes a load/store address or value at run time (pushfd/rol/sbb/xchg sequences); needs a runtime measurement (PLAN §4b) before porting |  |
| 0x49cd50 | FUN_0049cd50 | REA | 0 | — | deferred | unnamed / object-side helper (catalogue unknown or CObject); belongs to the CObject/CEntity pass |  |
| 0x5329a0 | FUN_005329a0 | REA | 0 | — | deferred | unnamed / object-side helper (catalogue unknown or CObject); belongs to the CObject/CEntity pass |  |
| 0x538ad0 | FUN_00538ad0 | REA | 0 | — | deferred | unnamed / object-side helper (catalogue unknown or CObject); belongs to the CObject/CEntity pass |  |
| 0x54ece0 | TransformPoint | REA | 0 | — | deferred | CSimpleTransform helpers used by entity code; belongs to the CPlaceable/CEntity pass |  |
| 0x54ee30 | TransformVectors | REA | 0 | — | deferred | CSimpleTransform helpers used by entity code; belongs to the CPlaceable/CEntity pass |  |
| 0x558de0 | CStats::GetStatID | gta-reversed | 0 | stats::get_stat_id | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::GetStatID(uint16)` (0x558DE0): `sprintf(gString, id < 0x52 ? "stat_f_%d" : "stat_i_%d", id)`; returns `gString`. |
| 0x558e30 | CStats::IsStatFloat | gta-reversed | 0 | stats::is_stat_float | L1 | L1 1,002,000 cases, 0 mismatches | `CStats::IsStatFloat(uint16)` (0x558E30): `id < 0x52`. |
| 0x558e40 | CStats::GetStatValue--OG | gta-reversed | 0 | stats::get_stat_value | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `CStats::GetStatValue(uint16)` (0x558E40): `fld` of the float stat or `fild` of the int stat, in ST0. |
| 0x558e70 | CStats::GetTimesMissionAttempted | gta-reversed | 0 | stats::get_times_mission_attempted | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::GetTimesMissionAttempted(uint8)` (0x558E70): the low byte of the count (AL). |
| 0x558e80 | CStats::RegisterMissionAttempted | gta-reversed | 0 | stats::register_mission_attempted | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::RegisterMissionAttempted(uint8)` (0x558E80): `++` unless the count is -1 (passed). |
| 0x558ea0 | CStats::RegisterMissionPassed | gta-reversed | 0 | stats::register_mission_passed | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::RegisterMissionPassed(uint8)` (0x558EA0): count = -1. |
| 0x558ec0 | CStats::PopulateFavoriteRadioStationList | REA | 0 | — | deferred | not ported in part 1 (189 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x558f90 | CStats::GetFullFavoriteRadioStationList | gta-reversed | 0 | stats::get_full_favorite_radio_station_list | L2 | L2 grove:2/2, grove2:2/2, pr:3/3, cheats:2/2 (0 mismatches, 0 undeclared) | `CStats::GetFullFavoriteRadioStationList` (0x558F90). |
| 0x558fa0 | CStats::FindMostFavoriteRadioStation | REA | 0 | stats::find_most_favorite_radio_station | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::FindMostFavoriteRadioStation` (0x558FA0): over stations 1..12, the first with the largest count above 0 (signed, strict); 1 when none. |
| 0x559010 | CStats::FindLeastFavoriteRadioStation | REA | 0 | stats::find_least_favorite_radio_station | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::FindLeastFavoriteRadioStation` (0x559010): over stations 1..12, the first with the smallest count (starting from station 1's). |
| 0x559080 | CStats::FindCriminalRatingNumber | gta-reversed | 0 | stats::find_criminal_rating_number | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::FindCriminalRatingNumber` (0x559080): the x87 sequence of the bytes (see the docs), with `_ftol` truncations at the same points. |
| 0x5591e0 | CStats::GetPercentageProgress | gta-reversed | 0 | stats::get_percentage_progress | L2 | L2 grove:66/66, grove2:66/66, pr:40/40, cheats:41/41 (0 mismatches, 0 undeclared) | `CStats::GetPercentageProgress` (0x5591E0): 0 when the total (float stat 1) == 0, else `progress / total * 100`, capped at 100 when above; ST0. |
| 0x559230 | CStats::BuildStatLine | REA | 0 | — | deferred | orchestrator (759 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x559540 | CStats::ConvertToMins | gta-reversed | 0 | stats::convert_to_mins | L1 | L1 1,002,000 cases, 0 mismatches | `CStats::ConvertToMins(int32)` (0x559540): 0 below 60, else `(uint32)(v - 60) / 60 + 1`. |
| 0x559560 | CStats::ConvertToSecs | gta-reversed | 0 | stats::convert_to_secs | L1 | L1 1,002,000 cases, 0 mismatches | `CStats::ConvertToSecs(int32)` (0x559560): `v` below 60, else `(v - 60) - (uint32)(v - 60) / 60 * 60`; then the absolute value (`neg`). |
| 0x559590 | CStats::SafeToShowThisStat | gta-reversed | 0 | stats::safe_to_show_this_stat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::SafeToShowThisStat(uint8)` (0x559590): true unless a German game and the id is one of 0xA7, 0xA8, 0xB1, 0xCD..0xD1 (the byte table at 0x5595C4). |
| 0x5595f0 | CStats::CheckForThreshold | gta-reversed | 0 | stats::check_for_threshold | L1 | L1 1,003,364 cases, 0 mismatches, 6,892 SNaN cases unverified (native off) | `CStats::CheckForThreshold(float* stat, float threshold)` (0x5595F0): when `*stat + 40 < t` or `*stat - 40 > t`, `*stat = t` (bit copy) and true. |
| 0x559630 | CStats::IsStatCapped | gta-reversed | 0 | stats::is_stat_capped | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1001, grove2:1000/1001, pr:990/990, cheats:438/438 | `CStats::IsStatCapped(uint16)` (0x559630). |
| 0x559730 | CStats::ProcessReactionStatsOnDecrement | gta-reversed | 0 | stats::process_reaction_stats_on_decrement | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::ProcessReactionStatsOnDecrement(uint8)` (0x559730): for stat 0xA5 below 0, fat (0x15) -= 23. |
| 0x559760 | CStats::CheckForStatsMessage | REA | 0 | — | deferred | not ported in part 1 (247 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x559860 | CStats::LoadStatUpdateConditions | REA | 0 | — | deferred | loader / initialiser using CRT sscanf/_strupr and the file loader; next pass (needs an L2 run covering the load) |  |
| 0x5599b0 | CStats::LoadActionReactionStats | gta-reversed | 0 | — | deferred | loader / initialiser using CRT sscanf/_strupr and the file loader; next pass (needs an L2 run covering the load) |  |
| 0x559a50 | CStats::FindMaxNumberOfGroupMembers | gta-reversed | 0 | — | deferred | not ported in part 1 (156 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x559af0 | CStats::GetFatAndMuscleModifier | REA | 0 | — | deferred | orchestrator (1129 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x559fa0 | CStats::DecrementStat | gta-reversed | 0 | stats::decrement_stat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::DecrementStat(uint16, float)` (0x559FA0): only for `v > 0`. Float stats: `max(s - v, 0)`; int stats: `s - (int16)_ftol(v)`, then 0 when not above 0; low byte 0xA5 below 0 → fat -= 23; then `CheckForStatsMessage` (tail jump). |
| 0x55a070 | CStats::SetStatValue | gta-reversed | 0 | stats::set_stat_value | L2 | L2 grove:50/50 undecl 8, grove2:50/50, pr:42/42, cheats:36/36 (0 mismatches, 0 undeclared) | `CStats::SetStatValue(uint16, float)` (0x55A070): float stats take the value (`fld`/`fstp`), int stats `(int16)_ftol(v)` sign-extended (original); then `CheckForStatsMessage` (tail jump). |
| 0x55a0b0 | CStats::RegisterFastestTime | gta-reversed | 0 | — | deferred | not ported in part 1 (174 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55a160 | CStats::RegisterBestPosition | gta-reversed | 0 | — | deferred | not ported in part 1 (174 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55a210 | CStats::FindCriminalRatingString | REA | 0 | — | deferred | orchestrator (1385 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x55a780 | CStats::ConstructStatLine | REA | 0 | — | deferred | orchestrator (3958 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x55b900 | CStats::ProcessReactionStatsOnIncrement | gta-reversed | 0 | stats::process_reaction_stats_on_increment | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:990/990, cheats:438/438 (0 mismatches, 0 undeclared) | `CStats::ProcessReactionStatsOnIncrement(uint8)` (0x55B900): stamina (0x16) / 0xE1 with stat 0xA5 below 0: fat = max(fat - 23, 0) and `CheckForStatsMessage`; stat 0xA5 above 1000: fat += stat - 1000. |
| 0x55b980 | CStats::DisplayScriptStatUpdateMessage | gta-reversed | 0 | — | deferred | not ported in part 1 (427 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55bc50 | CStats::UpdateRespectStat | REA | 0 | — | deferred | orchestrator (716 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x55bf20 | CStats::UpdateSexAppealStat | REA | 0 | — | deferred | not ported in part 1 (385 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55c0c0 | CStats::Init | gta-reversed | 0 | — | deferred | loader / initialiser using CRT sscanf/_strupr and the file loader; next pass (needs an L2 run covering the load) |  |
| 0x55c180 | CStats::IncrementStat | gta-reversed | 0 | — | deferred | orchestrator (649 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x55c410 | CStats::SetNewRecordStat | gta-reversed | 0 | stats::set_new_record_stat | ported-unhookable | entry dword read as data by protection code (`sbb eax, [0x55C410]` at 0x4017B4); hooking it crashed L2 runs |  |
| 0x55c470 | CStats::UpdateFatAndMuscleStats | REA | 0 | — | deferred | not ported in part 1 (491 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55c660 | CStats::UpdateStatsWhenSprinting | gta-reversed | 0 | stats::update_stats_when_sprinting | L2 | L2 grove:693/693, grove2:693/693, pr:87/87, cheats:93/93 (0 mismatches, 0 undeclared) | `CStats::UpdateStatsWhenSprinting` (0x55C660): fat/muscle update with reaction 30, then the sprint stamina counter (0xB794D8) against reaction 35 * 1000 (stamina += reaction 2 and the message), else counter += `_ftol(timeStep * 0.02 * 1000)`. |
| 0x55c6f0 | CStats::UpdateStatsWhenRunning | gta-reversed | 0 | stats::update_stats_when_running | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:472/472, cheats:86/86 (0 mismatches, 0 undeclared) | `CStats::UpdateStatsWhenRunning` (0x55C6F0): as sprinting with reaction 31, counter 0xB79500, threshold reaction 36, amount reaction 3. |
| 0x55c780 | CStats::UpdateStatsWhenCycling | REA | 0 | — | deferred | orchestrator (525 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x55c990 | CStats::UpdateStatsWhenSwimming | REA | 0 | — | deferred | not ported in part 1 (293 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55cac0 | CStats::UpdateStatsWhenDriving | REA | 0 | — | deferred | not ported in part 1 (319 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55cc00 | CStats::UpdateStatsWhenFlying | REA | 0 | — | deferred | not ported in part 1 (338 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55cd60 | CStats::UpdateStatsWhenOnMotorBike | gta-reversed | 0 | — | deferred | not ported in part 1 (325 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55ceb0 | CStats::UpdateStatsWhenWeaponHit | REA | 0 | — | deferred | not ported in part 1 (228 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55cfa0 | CStats::UpdateStatsWhenFighting | gta-reversed | 0 | stats::update_stats_when_fighting | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::UpdateStatsWhenFighting` (0x55CFA0): `UpdateFatAndMuscleStats(_ftol(reaction 32))`. |
| 0x55cfc0 | CStats::UpdateStatsOnRespawn | REA | 0 | — | deferred | not ported in part 1 (107 bytes; calls unported PED/VEH/HUD/streaming code); next pass |  |
| 0x55d030 | CStats::UpdateStatsAddToHealth | REA | 0 | stats::update_stats_add_to_health | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::UpdateStatsAddToHealth(uint32)` (0x55D030): counter 0xB79510 += amount; above reaction 58: stat 0x18 += reaction 11, the message, counter 0. |
| 0x55d090 | CStats::ModifyStat | gta-reversed | 0 | stats::modify_stat | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CStats::ModifyStat(uint16, float)` (0x55D090): `IncrementStat(id, v)` (tail jump with the caller's arguments) unless `v < 0` or NaN, then `DecrementStat(id, -v)`. |
| 0x5631c0 | CWorld::ResetLineTestOptions | gta-reversed | 0 | cworld::reset_line_test_options | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CWorld::ResetLineTestOptions` (0x5631C0). |
| 0x563470 | CWorld::ClearScanCodes | gta-reversed | 0 | cworld::clear_scan_codes | ported-unhookable | entry dword read as data by protection code (`add eax, [0x563470]` at 0x4013A2) |  |
| 0x563fa0 | CWorld::FindPlayerSlotWithPedPointer | gta-reversed | 0 | cworld::find_player_slot_with_ped_pointer | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `CWorld::FindPlayerSlotWithPedPointer(void*)` (0x563FA0): slot 0..1 whose ped is `p`, else -1. |
| 0x563fd0 | CWorld::FindPlayerSlotWithRemoteVehiclePointer | gta-reversed | 0 | cworld::find_player_slot_with_remote_vehicle_pointer | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CWorld::FindPlayerSlotWithRemoteVehiclePointer(void*)` (0x563FD0): slot whose remote vehicle (+0xB0) is `p`, else -1. |
| 0x564000 | CWorld::FindPlayerSlotWithVehiclePointer | gta-reversed | 0 | cworld::find_player_slot_with_vehicle_pointer | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CWorld::FindPlayerSlotWithVehiclePointer(CEntity*)` (0x564000): slot whose ped's `m_pVehicle` (+0x58C, no in-vehicle test) is `v`; **0** when none (original; gr returns -1). |
| 0x56e010 | CWorld::FindPlayerCoors | gta-reversed | 0 | cworld::find_player_coors | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `FindPlayerCoors(CVector* out, int32 id)` (0x56E010): the vehicle's position when the ped is in one, else the ped's; zero without a ped. Returns `out`. |
| 0x56e090 | CWorld::FindPlayerSpeed | gta-reversed | 0 | cworld::find_player_speed | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `FindPlayerSpeed(int32 id)` (0x56E090): `&vehicle->m_vecMoveSpeed` (+0x44) in a vehicle, else the ped's (no null check of the ped). |
| 0x56e0d0 | CWorld::FindPlayerVehicle | gta-reversed | 0 | cworld::find_player_vehicle | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `FindPlayerVehicle(int32 id, bool includeRemote)` (0x56E0D0): null without a ped or outside a vehicle; with `includeRemote` the remote vehicle when set; else `m_pVehicle`. |
| 0x56e120 | CWorld::FindPlayerEntity | gta-reversed | 0 | cworld::find_player_entity | L2 | L2 cheats:94/94 (0 mismatches, 0 undeclared) | `FindPlayerEntity(int32 id)` (0x56E120): the vehicle in a vehicle, else the ped (no null check). |
| 0x56e160 | CWorld::FindPlayerTrain | gta-reversed | 0 | cworld::find_player_train | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `FindPlayerTrain(int32 id)` (0x56E160): the vehicle when the player is in one whose type (+0x590) is 6, else null. |
| 0x56e210 | CWorld::FindPlayerPed | gta-reversed | 0 | cworld::find_player_ped | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `FindPlayerPed(int32 id)` (0x56E210). |
| 0x56e230 | CWorld::FindPlayerWanted | gta-reversed | 0 | cworld::find_player_wanted | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 | `FindPlayerWanted(int32 id)` (0x56E230): `Players[id]` +4. |
| 0x56e250 | CWorld::FindPlayerCentreOfWorld | gta-reversed | 0 | cworld::find_player_centre_of_world | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `FindPlayerCentreOfWorld(int32 id)` (0x56E250). With the camera byte 0x9690C1 set: the camera entity's matrix position (`[0xB6F03C] + 0x30`) or the vector at 0xB6F02C. Else the remote vehicle's position (of `id`, resolved); else, when the **focus** player (`PlayerInFocus`, not `id`) is in a vehicle, |
| 0x56e320 | CWorld::FindPlayerCentreOfWorld_NoSniperShift | gta-reversed | 0 | cworld::find_player_centre_of_world_no_sniper_shift | implemented-needs-L2 | no call in the recordings (NOCALLS) | `FindPlayerCentreOfWorld_NoSniperShift(int32 id)` (0x56E320). As above, but the remote-vehicle test and the final ped use the raw `id` (-1 indexes before the array, original); the in-vehicle test uses `id` resolved; without a ped it returns the vector at 0xB9B970. |
| 0x56e400 | CWorld::FindPlayerCentreOfWorldForMap | gta-reversed | 0 | cworld::find_player_centre_of_world_for_map | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `FindPlayerCentreOfWorldForMap(CVector* out, int32 id)` (0x56E400): copies `FindPlayerCentreOfWorld(id)` to a local, `CEntryExitManager::GetPositionRelativeToOutsideWorld` on it, copies it to `out`; returns `out`. |
| 0x5720d0 | CTheZones::InitZonesPopulationSettings | gta-reversed | 0 | zones::init_zones_population_settings | L2 | L2 grove:1/1, grove2:1/1, pr:2/2 undecl 1, pr2:3/3, cheats:1/1 (0 mismatches, 0 undeclared) | `CTheZones::InitZonesPopulationSettings` (0x5720D0): for every one of the 380 zone infos (fixed count): +0xF = (+0xF & 0xE5) / 5, +0x10 /= 0xF, +0xA = 0, dwords +0/+4 and word +8 = 0, then +0xF &= 0x1F. |
| 0x572110 | CTheZones::ResetZonesRevealed | gta-reversed | 0 | zones::reset_zones_revealed | L2 | L2 pr:1/1 (0 mismatches, 0 undeclared) | `CTheZones::ResetZonesRevealed` (0x572110): `ZonesVisited` = 0 (100 bytes), `ZonesRevealed` = 0. |
| 0x572130 | CTheZones::GetCurrentZoneLockedOrUnlocked | REA | 0 | zones::get_current_zone_locked_or_unlocked | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CTheZones::GetCurrentZoneLockedOrUnlocked(CVector2D)` (0x572130): `ZonesVisited[9 + idx] != 0` (out-of-map positions index outside the array, original bug). |
| 0x572180 | CTheZones::AssignZoneInfoForThisZone | gta-reversed | 0 | zones::assign_zone_info_for_this_zone | L2 | L2 grove:378/378, grove2:378/378, pr:378/378, cheats:378/378 (0 mismatches, 0 undeclared) | `CTheZones::AssignZoneInfoForThisZone(int16)` (0x572180): the first other navigation zone (u16 index below the count) whose label A (8 bytes, two dword compares) equals this one's gives its info index; else the index is `TotalNumberOfZoneInfos++`. |
| 0x572220 | CTheZones::ZoneIsEntirelyContainedWithinOtherZone | gta-reversed | 0 | zones::zone_is_entirely_contained_within_other_zone | L1 | L1 1,002,000 cases, 0 mismatches | `CTheZones::ZoneIsEntirelyContainedWithinOtherZone(CZone*, CZone*)` (0x572220): signed int16 compares a.X1 >= b.X1, a.X2 <= b.X2, Y, Z. |
| 0x572270 | CTheZones::PointLiesWithinZone | gta-reversed | 0 | zones::point_lies_within_zone | L1 | L1 1,060,000 cases, 0 mismatches, 6,189 SNaN cases unverified (native off); L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 | `CTheZones::PointLiesWithinZone(const CVector*, CZone*)` (0x572270): `(float)X1 <= p.x <= (float)X2` etc. (`fild` + `fcomp`, false on NaN). |
| 0x572300 | CTheZones::GetLevelFromPosition | gta-reversed | 0 | zones::get_level_from_position | L2 | L2 grove:3/3, grove2:3/3, pr:6/6, cheats:4/4 (0 mismatches, 0 undeclared) | `CTheZones::GetLevelFromPosition(const CVector&)` (0x572300): the level byte (+0x1F) of the first map zone 1.. (count read once, u16) containing the point (`PointLiesWithinZone`), else map zone 0's. |
| 0x572360 | CTheZones::FindSmallestZoneForPosition | gta-reversed | 0 | zones::find_smallest_zone_for_position | ported-unhookable | entry bytes read as data by protection code (`xor ecx, [0x572363]` at 0x406A9E) |  |
| 0x572400 | CTheZones::GetZoneInfo- | gta-reversed | 0 | zones::get_zone_info | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `CTheZones::GetZoneInfo(const CVector&, CZone**)` (0x572400). |
| 0x572440 | CTheZones::FillZonesWithGangColours | gta-reversed | 0 | zones::fill_zones_with_gang_colours | L2 | L2 grove:2/2, grove2:2/2, pr:3/3, cheats:3/3 (0 mismatches, 0 undeclared) | `CTheZones::FillZonesWithGangColours(bool disableRadarGangColors)` (0x572440). Component k of the colour is `sum_g table_k[g] * strength[g]` over gangs 0..2 (tables at 0x8D1344/0x8D1350/0x8D135C, indexed by gang), divided (signed) by `max(sum, 1)`; alpha `min(sum*3, 120)` raised to 55 when sum != 0. |
| 0x572590 | CTheZones::GetNavigationZone | gta-reversed | 0 | zones::get_navigation_zone | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CTheZones::GetNavigationZone(uint16)` (0x572590). |
| 0x5725a0 | CTheZones::GetMapZone | gta-reversed | 0 | zones::get_map_zone | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CTheZones::GetMapZone(uint16)` (0x5725A0). |
| 0x5725b0 | CTheZones::Calc2DDistanceBetween2Zones | gta-reversed | 0 | zones::calc_2d_distance_between_2_zones | L1 | L1 1,002,000 cases, 0 mismatches | `CTheZones::Calc2DDistanceBetween2Zones(CZone*, CZone*)` (0x5725B0): int gaps per axis (0 when the ranges overlap), `sqrt(dy*dy + dx*dx)` on the x87 (dx stored as f32), result in ST0. |
| 0x572670 | CTheZones::Init | gta-reversed | 0 | zones::init | L2 | L2 grove:1/1, grove2:1/1, pr:1/1, cheats:1/1 (0 mismatches, 0 undeclared) | `CTheZones::Init` (0x572670): clears every navigation zone (bytes 0..0x1E; the level byte +0x1F is kept) and every map zone (type +0x1E = 3), `InitZonesPopulationSettings`, clears `ZonesVisited`, then writes zone 0 of both arrays inline ("SAN_AND"/"THEMAP" in both labels, -3000..3000 x/y, -2000..200 |
| 0x572800 | CTheZones::SetCurrentZoneAsUnlocked | REA | 0 | zones::set_current_zone_as_unlocked | L2 | L2 grove:31/31, grove2:31/31, pr:15/15, cheats:16/16 (0 mismatches, 0 undeclared) | `CTheZones::SetCurrentZoneAsUnlocked` (0x572800), see the spec: level from the player position, then (on the map, area 0, zone not yet visited) `ZonesRevealed++` and the visited byte = 1. |
| 0x5728a0 | CTheZones::CreateZone | gta-reversed | 2 | — | deferred | orchestrator (694 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x572b70 | CTheZones::PostZoneCreation | gta-reversed | 0 | zones::post_zone_creation | ported-unhookable | lone `ret`: too short for the 5-byte entry patch |  |
| 0x572b80 | CTheZones::FindZone- | gta-reversed | 0 | zones::find_zone | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CTheZones::FindZone(CVector*, uint64 label, eZoneType)` (0x572B80): type 0 compares label B (+8), type 2 label A (+0) (8 bytes, two dwords); any other type: false. True when a matching navigation zone (count re-read each step for type 2... read once per loop, u16) contains the point. |
| 0x572c40 | CTheZones::FindZoneByLabel | gta-reversed | 1 | zones::find_zone_by_label | L2 | L2 grove:478/478, grove2:478/478, pr:956/956, cheats:478/478 (0 mismatches, 0 undeclared) | `CTheZones::FindZoneByLabel(const char*, eZoneType)` (0x572C40): copies the name (unbounded, into a zeroed 8-byte buffer: longer names overflow the original's stack, here they are cut at 8); type != 2 gives -1; else the first navigation zone whose label A equals the 8 bytes. |
| 0x572cc0 | CTheZones::SetZoneRadarColours | gta-reversed | 0 | zones::set_zone_radar_colours | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CTheZones::SetZoneRadarColours(int16, char radarMode, uint8 r, uint8 g, uint8 b)` (0x572CC0): radar mode bits 5..6 of +0xF from `mode << 5`, colour bytes +0xB..+0xD. Alpha is not written (gr writes 255). |
| 0x572d10 | CTheZones::Update | gta-reversed | 0 | zones::update | L2 | L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 (0 mismatches, 0 undeclared) | `CTheZones::Update` (0x572D10): the function-local `CountSeconds` starts at `time - 5000` (guard bit 0 of 0xBA6724); outside area 0 it follows the time; else after more than 5000 ms (unsigned) `SetCurrentZoneAsUnlocked` and `CountSeconds` = the time re-read. |
| 0x59f050 | FUN_0059f050 | REA | 0 | — | deferred | unnamed / object-side helper (catalogue unknown or CObject); belongs to the CObject/CEntity pass |  |
| 0x5a1df0 | CObject | REA | 0 | — | deferred | unnamed / object-side helper (catalogue unknown or CObject); belongs to the CObject/CEntity pass |  |
| 0x5d2e60 | CTheZones::Save | gta-reversed | 0 | zones::save | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CTheZones::Save` (0x5D2E60): level (4), the three counts (2 each), the zones (0x20), infos (0x11), map zones, `ZonesVisited` (100), `ZonesRevealed` (4); counts re-read in each loop test. Returns true. |
| 0x5d2f40 | CTheZones::Load | gta-reversed | 0 | zones::load | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CTheZones::Load` (0x5D2F40): `Init`, then the same layout as `Save` read back. Returns true. |
| 0x5d3020 | CSaveGarage::CopyGarageIntoSaveGarage | REA | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d30c0 | CSaveGarage::CopyGarageOutOfSaveGarage | REA | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d3160 | CGarages::Save | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d3270 | CGarages::Load | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d33c0 | CGameLogic::Save | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d3440 | CGameLogic::Load | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d3540 | CPickups::Save | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d35a0 | CPickups::Load | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d3620 | CRestart::Save | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d3770 | CRestart::Load | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d3b40 | CStats::Save | gta-reversed | 1 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d3bf0 | CStats::Load | gta-reversed | 1 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d3de0 | CShopping::Save | gta-reversed | 1 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d3e40 | CShopping::Load | gta-reversed | 1 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d5570 | CStuntJumpManager::Save | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d55c0 | CEntryExitManager::Load | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d5920 | CStuntJumpManager::Load | gta-reversed | 0 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x5d5970 | CEntryExitManager::Save | gta-reversed | 1 | — | deferred | save/load path: needs a save/load recording (SAVE pass) |  |
| 0x6f3fc0 | CCranes::InitCranes | gta-reversed | 0 | misc::cranes_init_cranes | ported-unhookable | lone `ret`: too short for the 5-byte entry patch |  |
| 0x6f3fd0 | CCranes::IsThisCarPickedUp | gta-reversed | 0 | misc::cranes_is_this_car_picked_up | ported-unhookable | `xor al,al; ret`: too short for the 5-byte entry patch |  |
| 0x6f3fe0 | CCranes::UpdateCranes | gta-reversed | 0 | misc::cranes_update_cranes | ported-unhookable | lone `ret`: too short for the 5-byte entry patch |  |
| 0x6f3ff0 | CCranes::IsThisCarBeingCarriedByAnyCrane | gta-reversed | 0 | misc::cranes_is_this_car_being_carried_by_any_crane | ported-unhookable | `xor al,al; ret`: too short for the 5-byte entry patch |  |
| 0x6f4000 | CCranes::IsThisCarBeingTargettedByAnyCrane | gta-reversed | 0 | misc::cranes_is_this_car_being_targetted_by_any_crane | ported-unhookable | `xor al,al; ret`: too short for the 5-byte entry patch |  |
| 0x717110 | CEscalator::Constructor | gta-reversed | 0 | escalators::constructor | L2 | L2 grove:32/32, grove2:32/32, pr:32/32, cheats:32/32 (0 mismatches, 0 undeclared) | `CEscalator::CEscalator` (0x717110, thiscall): +0x70, +0x74, the object pointers and +0x79 = 0. |
| 0x717130 | CEscalator::Destructor | gta-reversed | 0 | escalators::destructor | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CEscalator::~CEscalator` (0x717130, thiscall): `~CMatrix` of +0x30 (tail jump). |
| 0x717860 | CEscalator::SwitchOff | gta-reversed | 0 | escalators::switch_off | L2 | L2 split grove:64 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:160 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:64 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CEscalator::SwitchOff` (0x717860, thiscall): with objects created, removes and deletes every object of the three step counts (re-read each step; 0xC6E98C = 1 around the delete), then +0x79 = 0. |
| 0x717940 | CEscalators::Shutdown | gta-reversed | 0 | escalators::shutdown | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CEscalators::Shutdown` (0x717940): `SwitchOff` and `m_bExist` = 0 for every escalator. |
| 0x717970 | CEscalator::AddThisOne | gta-reversed | 0 | — | deferred | orchestrator (685 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x717c50 | CEscalators::Init | gta-reversed | 0 | escalators::init | L2 | L2 split grove:1 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:2 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CEscalators::Init` (0x717C50): as `Shutdown`, then `SwitchOff` of every escalator again. |
| 0x717c90 | CEscalators::AddOne | gta-reversed | 0 | escalators::add_one | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CEscalators::AddOne(...)` (0x717C90): `AddThisOne` on the first escalator without `m_bExist`, with the six argument dwords; nothing when all 32 exist. |
| 0x717d30 | CEscalator::Update | gta-reversed | 0 | — | deferred | orchestrator (2128 bytes) over unported subsystems (PED/VEH/AI/HUD/CAM/AUDIO); next pass |  |
| 0x718580 | CEscalators::Update | gta-reversed | 0 | escalators::update | L2 | L2 split grove:1000 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1000 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1000 compared, 0 mismatches, 0 undeclared (verdict PASS) | `CEscalators::Update` (0x718580): nothing during replay playback (`CReplay::Mode` 0xA43088 == 1); else `CEscalator::Update` of every existing escalator. |
| 0x71d4e0 | CGridRef::Init | gta-reversed | 0 | — | deferred | loader / initialiser using CRT sscanf/_strupr and the file loader; next pass (needs an L2 run covering the load) |  |
| 0x71d5a0 | CGridRef::GetGridRefPositions-vec | gta-reversed | 0 | misc::get_grid_ref_positions_vec | L1 | L1 1,060,000 cases, 0 mismatches, 12,452 SNaN cases unverified (native off) | `CGridRef::GetGridRefPositions(CVector pos, uint8* x, uint8* y)` (0x71D5A0, vector by value). |
| 0x71d5e0 | CGridRef::GetGridRefPositions- | gta-reversed | 0 | misc::get_grid_ref_positions | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGridRef::GetGridRefPositions(uint8* x, uint8* y)` (0x71D5E0): `FindPlayerCoors(-1)` twice (the first result unused, as in the original), the cell of the second. |
| 0x71d650 | CGridRef::GetArtistBugstarID | gta-reversed | 0 | misc::get_artist_bugstar_id | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGridRef::GetArtistBugstarID(uint8 x, uint8 y)` (0x71D650): "" for x, y < 10 (unsigned), else null. |
| 0x71d670 | CGridRef::GetAreaName | gta-reversed | 0 | misc::get_area_name | implemented-needs-L2 | no call in the recordings (NOCALLS) | `CGridRef::GetAreaName(uint8 x, uint8 y)` (0x71D670): `atol(GridRefList[x][y])` for x, y < 10, else 0. |
| 0x72d850 | CZoneDef::IsPointWithin | REA | 0 | zones::zone_def_is_point_within | L1 | L1 1,060,000 cases, 0 mismatches, 6,205 SNaN cases unverified (native off); L2 grove:1000/1001, grove2:1000/1001, pr:1000/1001, cheats:1000/1001 | `CZoneDef::IsPointWithin(CVector)` (0x72D850, thiscall, vector by value, `ret 0xC`): see `docs/re/specs/WORLD/CZoneDef.md`. |
| 0x849b20 | __init_g_96a718 | REA | 0 | statics::init_g_96a718 | L2 | L2 split grove:1 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `__init_g_96a718` (0x849B20): `[0x96A718] = [0x96A714] * 10.0` (f32). |
| 0x849b40 | __init_g_96a71c | REA | 0 | statics::init_g_96a71c | L2 | L2 split grove:1 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `__init_g_96a71c` (0x849B40): `[0x96A71C] = [0x858630] / 50.0` (f32). |
| 0x849ce0 | __init_CGameLogic__SavedWeapons | REA | 0 | statics::init_saved_weapons | L2-split (undeclared writes to review) | L2 split grove:1 compared, 0 mismatches, 1 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 1 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `__init_CGameLogic__SavedWeapons` (0x849CE0): 13 `CWeapon` (0x1C) at 0x96A9B8. |
| 0x849e90 | __init_CGarages__aGarages | REA | 0 | statics::init_garages | L2-split (undeclared writes to review) | L2 split grove:1 compared, 0 mismatches, 1 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 1 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `__init_CGarages__aGarages` (0x849E90): 50 `CGarage` (0xD8) at 0x96C048. |
| 0x84a260 | __init_g_97d64c | REA | 0 | statics::init_g_97d64c | L2 | L2 split grove:1 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `__init_g_97d64c` (0x84A260): `[0x97D64C] = 1.0 / 50.0` (f32). |
| 0x84a280 | __init_g_97d650 | REA | 0 | statics::init_g_97d650 | L2 | L2 split grove:1 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `__init_g_97d650` (0x84A280): `[0x97D650] = 100.0 / 3.6` (f32; 3.6 = bits 0x40666666). |
| 0x84a540 | __init_CReplay__PlayerInfo | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x84b1c0 | __init_CSetPieces__aSetPieces | REA | 0 | statics::init_set_pieces | L2 | L2 split grove:1 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `__init_CSetPieces__aSetPieces` (0x84B1C0): 210 set pieces (0x20): +0x1C = 0, +0 = 0. |
| 0x84b2a0 | __init_gStoredClothesState | REA | 0 | statics::init_stored_clothes_state | L2 | L2 split grove:1 compared, 0 mismatches, 0 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 0 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 0 undeclared (verdict PASS) | `__init_gStoredClothesState` (0x84B2A0): `CPedClothesDesc` constructor on 0xA9A810 (tail jump). |
| 0x84e300 | __init_gMatrixList | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x84e980 | __init_CWorld__Players | REA | 0 | statics::init_players | L2-split (undeclared writes to review) | L2 split grove:1 compared, 0 mismatches, 1 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 1 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `__init_CWorld__Players` (0x84E980): 2 `CPlayerInfo` (0x190) at 0xB7CD98. |
| 0x84e9b0 | __init_CWorld__ms_aSectors | REA | 0 | statics::init_sectors | L2-split (undeclared writes to review) | L2 split grove:1 compared, 0 mismatches, 1 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 1 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `__init_CWorld__ms_aSectors` (0x84E9B0): 14400 `CSector` (8) at 0xB7D0B8. |
| 0x84e9e0 | __init_CWorld__ms_aRepeatSectors | REA | 0 | statics::init_repeat_sectors | L2-split (undeclared writes to review) | L2 split grove:1 compared, 0 mismatches, 1 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 1 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `__init_CWorld__ms_aRepeatSectors` (0x84E9E0): 256 `CRepeatSector` (0xC) at 0xB992B8. |
| 0x84ea10 | __init_CWorld__ms_aLodPtrLists | REA | 0 | statics::init_lod_ptr_lists | L2-split (undeclared writes to review) | L2 split grove:1 compared, 0 mismatches, 1 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 1 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `__init_CWorld__ms_aLodPtrLists` (0x84EA10): 900 `CPtrListSingleLink` (4) at 0xB99EB8. |
| 0x84ea40 | __init_CWorld__ms_listMovingEntityPtrs | REA | 0 | statics::init_list_moving_entity_ptrs | L2-split (undeclared writes to review) | L2 split grove:1 compared, 0 mismatches, 1 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 1 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `__init_CWorld__ms_listMovingEntityPtrs` (0x84EA40): `_atexit` of its destructor only. |
| 0x84ea50 | __init_CWorld__ms_listObjectsWithControlCode | REA | 0 | statics::init_list_objects_with_control_code | L2-split (undeclared writes to review) | L2 split grove:1 compared, 0 mismatches, 1 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 1 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `__init_CWorld__ms_listObjectsWithControlCode` (0x84EA50): `_atexit` of its destructor only. |
| 0x84f300 | __init_g_furnitureMan | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x84f320 | __init_FurnitureManager_c__g_subGroupStore | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x84f350 | __init_g_bae1f8 | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x84f740 | __init_g_interiorMan | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x84fa80 | __init_g_breakMan | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x853f10 | __init_g_waterCreatureMan | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x854d50 | __init_CEscalators__aEscalators | REA | 0 | statics::init_escalators | L2-split (undeclared writes to review) | L2 split grove:1 compared, 0 mismatches, 1 undeclared (verdict PASS), pr:1 compared, 0 mismatches, 1 undeclared (verdict PASS), cheats:1 compared, 0 mismatches, 1 undeclared (verdict PASS) | `__init_CEscalators__aEscalators` (0x854D50): 32 `CEscalator` (0x150) at 0xC6E9A8. |
| 0x856110 | __dtor_CGameLogic__SavedWeapons | REA | 0 | statics::dtor_saved_weapons | implemented-needs-L2 | no call in the recordings (NOCALLS) | `__dtor_CGameLogic__SavedWeapons` (0x856110). |
| 0x856130 | __dtor_CGarages__aGarages | REA | 0 | statics::dtor_garages | implemented-needs-L2 | no call in the recordings (NOCALLS) | `__dtor_CGarages__aGarages` (0x856130). |
| 0x856160 | __dtor_CReplay__PlayerInfo | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x856490 | __dtor_gMatrixList | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x8564c0 | __dtor_CWorld__ms_aLodPtrLists | REA | 0 | statics::dtor_lod_ptr_lists | implemented-needs-L2 | no call in the recordings (NOCALLS) | `__dtor_CWorld__ms_aLodPtrLists` (0x8564C0). |
| 0x8564e0 | __dtor_CWorld__ms_listMovingEntityPtrs | REA | 0 | statics::dtor_list_moving_entity_ptrs | implemented-needs-L2 | no call in the recordings (NOCALLS) | `__dtor_CWorld__ms_listMovingEntityPtrs` (0x8564E0): `CPtrListDoubleLink::Flush` (tail jump). |
| 0x8564f0 | __dtor_CWorld__ms_listObjectsWithControlCode | REA | 0 | statics::dtor_list_objects_with_control_code | implemented-needs-L2 | no call in the recordings (NOCALLS) | `__dtor_CWorld__ms_listObjectsWithControlCode` (0x8564F0): `CPtrListDoubleLink::Flush` (tail jump). |
| 0x856500 | __dtor_CWorld__Players | REA | 0 | statics::dtor_players | implemented-needs-L2 | no call in the recordings (NOCALLS) | `__dtor_CWorld__Players` (0x856500). |
| 0x856520 | __dtor_CWorld__ms_aSectors | REA | 0 | statics::dtor_sectors | implemented-needs-L2 | no call in the recordings (NOCALLS) | `__dtor_CWorld__ms_aSectors` (0x856520). |
| 0x856540 | __dtor_CWorld__ms_aRepeatSectors | REA | 0 | statics::dtor_repeat_sectors | implemented-needs-L2 | no call in the recordings (NOCALLS) | `__dtor_CWorld__ms_aRepeatSectors` (0x856540). |
| 0x856600 | __dtor_FurnitureManager_c__g_subGroupStore | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x856620 | __dtor_g_bae1f8 | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x856640 | __dtor_g_furnitureMan | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x856650 | __dtor_g_interiorMan | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x856660 | __dtor_g_breakMan | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x856a80 | __dtor_g_waterCreatureMan | REA | 0 | — | deferred | static initialiser of a class outside part 1 (CReplay / gMatrixList / furniture / interior / break / water creatures) |  |
| 0x856b20 | __dtor_CEscalators__aEscalators | REA | 0 | statics::dtor_escalators | implemented-needs-L2 | no call in the recordings (NOCALLS) | `__dtor_CEscalators__aEscalators` (0x856B20). |

