# SAMP-1D: what gta_sa.exe executes under SA:MP (dynamic measurement)

The static part (`docs/samp/README.md`) listed what the SA:MP client depends on and proposed the
closure E (10,425 functions) as the scope of an "SA:MP-playable standalone" milestone. This file is the
confirmation step from its §6. The original gta_sa.exe 1.0 US runs with the original samp.dll 0.3.7-R4
against a local open.mp server. Every catalogue function entry and basic-block start carries a first-hit
breakpoint (INF-10, `docs/re/l3-coverage.md`), and scripted scenarios exercise the game. Measured on
2026-11-10/11, lane 4.

## 1. Setup

| item | what |
|---|---|
| client | samp.dll 0.3.7-R4, build 2019-11-21 (fv0.3.7.3, sha256 `15db80c5…f4f9ff12ab`). This is the build samp-rs installs in its game copy. Plus samp.saa, `SAMP/`, bass.dll, sampgui.png, mouse.png and sampaux3.ttf. All are copied from `~/samp-rs` into `re/samp/client/` (gitignored; samp-rs itself moved into this repository as `samp-rs/` on 2026-11-12, these third-party files stay under `re/samp/`) |
| server | open.mp 1.6.7 (32-bit Linux), copied to `re/samp/server/`. Each run uses its own copy in `target/e2e/lane-4/samp/server` on **UDP 7810** (samp-rs uses 7777/7790). Settings: artwork, rcon and announce off; `max_bots` 4 |
| gamemode | `tools/e2e/samp/sampcov.pwn`, compiled with the server's pawncc. It sets up 3 classes and a static train, logs every player callback as `[cov] …`, and reads a command channel (`scriptfiles/sampcov.txt`, `SEQ CMD|ARG|…`) |
| patch list | samp-rs `docs/re/data/gta-patches-r4-annotated.csv`, copied to `re/samp/` |
| game | the lane-4 game dir. The client files are symlinked in for the run and removed afterwards. Nothing in `wine/gta-sa` is changed |
| injection | `sa-launcher --dll sa_hook.dll --dll samp.dll -- -c -h 127.0.0.1 -p 7810 -n CovPlayer`. sa_hook loads first, so its breakpoints are armed before samp.dll's DllMain runs. Same command line as samp.exe |
| mode | real time (no determinism mode). sa-hook's 8 core hooks and the cdsync fix (INF-15) are active. Ports are not installed (`SA_HOOK_PORTS` unset) |

```
LANE=4 SA_ROOT=~/sa-engine tools/e2e/samp-cov.sh [--no-cov] [--steps walk,ui,world,anim,weapons,death,vehicles,interior,remote,quit] [--name N]
.venv/bin/python -I tools/e2e/samp/samp-analysis.py --points catalog/raw/coverage-samp.points.txt \
    --excluded catalog/raw/coverage-samp.excluded.csv \
    --hits catalog/raw/coverage-samp-full2.hits.csv,catalog/raw/coverage-samp-full3.hits.csv,catalog/raw/coverage-samp-remote.hits.csv \
    --scm catalog/raw/coverage-samp-full2.scm.csv,catalog/raw/coverage-samp-full3.scm.csv,catalog/raw/coverage-samp-remote.scm.csv \
    --sp catalog/raw/coverage-all.csv --out-dir target/samp-analysis --priority catalog/samp-priority.csv
```
(`catalog/raw/` is gitignored. Its single-player `coverage-all.csv` is the corpus merge of the main
checkout, see l3-coverage.md.)

### 1.1 Breakpoints and samp.dll's patches (`tools/e2e/samp/samp-points.py`)
`sa-oracle coverage points` gives 169,689 points. 234 of them are dropped (in 155 functions; 19 of the
dropped points are function entries) because samp.dll writes, reads or tests those bytes:
* every row of the patch CSV, `[addr, addr + size)`. This covers hooks, NOP runs, temporary patch and
  restore pairs, the 20-byte auth read at 0x4D3AA0, and the three sites samp spins on until they hold
  an expected byte (0x551024, 0x561872, 0x4083C0). Without this exclusion, an int3 at one of those
  sites would hang samp's init thread;
* the table-driven patches, read from samp.dll's `.data`. The 707 model-info pointer sites
  (0x10114B10) matter most: samp patches a site only when its opcode byte matches, so an int3 there
  would make samp skip it silently. 129 of the 707 sites are points. Also dropped: the sector grid
  references (14 + 56 + 11) and the model-store references (14 + 5). Each such site is covered as
  `[site, site + 8)`.

169,455 points are armed with 0 byte mismatches. With the SCM counter on (§4), 169,454 are armed: its
detour replaces the entry of `ProcessOneCommand`. **No crash, no hang and no samp misbehaviour came
from the breakpoints in any run.** For functions whose entry is not armed, the analysis counts the
function as executed when any of its blocks was hit.

## 2. Scenarios (docs/samp/README.md §6)

All steps run in the full runs `full2` and `full3`. The `remote` run has only the NPC step. Each step
was checked in the server log (`[cov]` callbacks) and in gamescope screenshots
(`target/e2e/lane-4/runs/*/`).

| step | how | reached |
|---|---|---|
| boot → connect → class selection | samp's `-c` command line; Right/Left keys browse 3 classes | yes (`connect`, `class` ×4) |
| spawn | Shift | yes |
| on foot | W/A, sprint, jump, crouch | yes. The walk enters CJ's house through its entry marker (interior 0→3) |
| chat, command | `T`, typed text, `/covtest` | yes (`OnPlayerText`, `OnPlayerCommandText`) |
| dialogs | message box, input box, list (styles 0/1/2), answered with Enter/Down | yes, 3 responses |
| game text, textdraws | GameText style 3; text, sprite and model-preview textdraws | yes (screenshots) |
| menu | CreateMenu/ShowMenuForPlayer, row chosen | yes (`menu-row 1`) |
| scoreboard, chat modes | Tab, F7 | yes |
| world entities | objects (+ MoveObject), pickups, 3D label, map icon, gang zone (flashing), actor, attached object, checkpoint, race checkpoint | created; the checkpoint and pickups were walked through |
| animations, camera, effects | ApplyAnimation, drunk, weather, time, SetPlayerCameraPos, wanted level, money, sound, spectate on/off | yes |
| weapons and damage | Deagle (aim + fire), M4 (mouse and LCtrl), grenade, knife, fist, armour/health, explosion | yes: 12 `OnPlayerWeaponShot` in full3 |
| death and respawn | SetPlayerHealth 0 | yes (`death`, second `spawn`) |
| vehicles | Sultan (drive, turn), NRG-500, BMX, Maverick, Dodo, Speeder (exit = swimming), Rhino, train (static 538), car entered with F, burning car (health 300), exploding car (health 0), swimming | yes, driver state for each (screenshots of heli, plane, boat, tank, train) |
| interiors | entry marker (CJ's house 0→3, walking); SetPlayerInterior 17 + 24/7 position | yes |
| remote players | open.mp server NPCs: two NPCs on foot, following, dance animation, aim (fire), melee, an NPC driving a Sultan with the player as passenger, NPC death, NPC removal | yes (NPC ids 48/49 stream in; player state 3 = passenger) |
| disconnect | `/q` | yes: `disconnect reason=1`, game exit code 0; coverage is flushed at DLL_PROCESS_DETACH |

Not covered: other servers' gamemodes, trailers, the mod shop, Pay'n'Spray, parachute, jetpack, remote
players run by a real second client (samp-rs `samp-bot` is that project's binary; the server NPCs
stand in for it), object editing, `SetPlayerAttachedObject` editing, audio streams and the samp UI
dialogs (server browser, settings).

## 3. Results

Merged runs: `full2` + `full3` + `remote`, about 15 minutes of game time. `full1` is not in the merge:
it crashed at the third vehicle (see §5).
`sa-oracle coverage report` over the same hits: 7,003 / 15,479 non-excluded functions have their
**entry** hit (45.2 %), and 52,005 / 163,378 armed blocks were hit (31.8 %). The tables below also count
the 17 functions whose entry was dropped (§1.1) but whose blocks ran: 7,020.

### 3.1 Functions executed under SA:MP per subsystem

live = functions of `docs/samp/required-functions.csv` (the live catalogue). samp = executed under
SA:MP. E = static closure E. SP = single-player corpus coverage (grove walk + pause-restart).

| subsystem | live | E | samp | samp ∩ E | samp ∖ E | E ∖ samp | SP | samp ∖ SP | SP ∖ samp | samp ∪ SP | blocks hit |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| UNK-game | 2883 | 833 | 1881 | 315 | 1566 | 518 | 1877 | 55 | 51 | 1932 | 2872 / 9615 |
| RW | 1805 | 666 | 822 | 488 | 334 | 178 | 824 | 18 | 20 | 842 | 5819 / 18106 |
| AI | 4047 | 3602 | 797 | 746 | 51 | 2856 | 872 | 397 | 472 | 1269 | 5364 / 35176 |
| RENDER | 832 | 651 | 512 | 478 | 34 | 173 | 519 | 61 | 68 | 580 | 5361 / 10003 |
| AUDIO | 842 | 700 | 496 | 438 | 58 | 262 | 435 | 106 | 45 | 541 | 4477 / 9722 |
| WORLD | 918 | 733 | 428 | 399 | 29 | 334 | 353 | 119 | 44 | 472 | 4731 / 13020 |
| STREAM | 553 | 352 | 352 | 270 | 82 | 82 | 363 | 17 | 28 | 380 | 3168 / 5459 |
| VEH | 714 | 612 | 305 | 285 | 20 | 327 | 210 | 136 | 41 | 346 | 5332 / 13721 |
| CORE | 319 | 220 | 189 | 168 | 21 | 52 | 207 | 5 | 23 | 212 | 1037 / 2276 |
| PED | 372 | 313 | 189 | 186 | 3 | 127 | 165 | 53 | 29 | 218 | 2373 / 5739 |
| HUD | 342 | 284 | 162 | 146 | 16 | 138 | 194 | 20 | 52 | 214 | 1521 / 6297 |
| ANIM | 265 | 176 | 133 | 121 | 12 | 55 | 159 | 18 | 44 | 177 | 826 / 2028 |
| POP | 430 | 338 | 130 | 122 | 8 | 216 | 194 | 22 | 86 | 216 | 1422 / 9060 |
| SCRIPT | 223 | 198 | 111 | 103 | 8 | 95 | 122 | 8 | 19 | 130 | 1272 / 7343 |
| COLL | 145 | 127 | 109 | 108 | 1 | 19 | 105 | 9 | 5 | 114 | 1608 / 2062 |
| CAM | 218 | 167 | 94 | 80 | 14 | 87 | 90 | 29 | 25 | 119 | 1569 / 5496 |
| INPUT | 155 | 125 | 90 | 85 | 5 | 40 | 73 | 27 | 10 | 100 | 805 / 2050 |
| WEAPON | 158 | 119 | 77 | 63 | 14 | 56 | 53 | 27 | 3 | 80 | 887 / 3412 |
| MATH | 112 | 76 | 64 | 63 | 1 | 13 | 69 | 3 | 8 | 72 | 171 / 308 |
| PHYS | 73 | 68 | 55 | 54 | 1 | 14 | 43 | 15 | 3 | 58 | 1197 / 2172 |
| CALLED-ONLY | 37 | 27 | 23 | 20 | 3 | 7 | 21 | 2 | 0 | 23 | 366 / 699 |
| SAVE | 36 | 34 | 1 | 1 | 0 | 33 | 2 | 0 | 1 | 2 | 1 / 409 |
| **total** | 15479 | 10421 | 7020 | 4739 | 2281 | 5682 | 6950 | 1147 | 1077 | 8097 | 52179 / 164173 |

(E is 10,421 here, not 10,425: four E functions are not live in the current catalogue.)

Reading:
* **The SA:MP set is about the size of the single-player corpus set** (7,020 vs 6,950), but the two
  sets differ. SA:MP adds 1,147 functions that the corpus does not reach: AI +397 (tasks of remote and
  scripted peds, NPC and passenger tasks, combat), VEH +136 (heli, plane, boat, bike, train, tank,
  damage), WORLD +119, AUDIO +106, RENDER +61, PED +53. The corpus has 1,077 functions that SA:MP does
  not reach: missions and the story script, ambient population, cutscenes and the menu.
  `samp ∪ SP` = 8,097.
* **E is not a superset.** 2,281 executed functions are outside E. 1,665 of them are CRT static
  initialisers and destructors (`role` init/dtor; the CRT calls them through its `_initterm` / atexit
  tables). The other 616 are callbacks and vtable methods that the closure did not resolve:

| subsystem | samp ∖ E | init/dtor | other |
|---|---:|---:|---:|
| UNK-game | 1566 | 1464 | 102 |
| RW | 334 | 2 | 332 |
| STREAM | 82 | 11 | 71 |
| AUDIO | 58 | 44 | 14 |
| AI | 51 | 36 | 15 |
| RENDER | 34 | 20 | 14 |
| WORLD | 29 | 22 | 7 |
| CORE | 21 | 1 | 20 |
| VEH | 20 | 16 | 4 |
| HUD | 16 | 8 | 8 |
| CAM | 14 | 6 | 8 |
| WEAPON | 14 | 10 | 4 |
| ANIM | 12 | 6 | 6 |
| POP | 8 | 7 | 1 |
| SCRIPT | 8 | 8 | 0 |
| INPUT | 5 | 2 | 3 |
| PED | 3 | 1 | 2 |
| CALLED-ONLY | 3 | 0 | 3 |
| COLL | 1 | 0 | 1 |
| PHYS | 1 | 0 | 1 |
| MATH | 1 | 1 | 0 |
| **total** | 2281 | 1665 | 616 |

  Examples of the "other" misses:
  * RW plugin, pipeline and stream callbacks (332), registered through function pointers;
  * the model-info vtable methods: `CVehicleModelInfo::*` callbacks and `CClumpModelInfo` /
    `CAtomicModelInfo` / `CPedModelInfo` `Init`, `SetClump`, `CreateInstance` and so on (STREAM 71).
    samp builds its relocated model stores itself and stores the game's vtables in them, so no
    reached game code references those tables;
  * `CdStream::CdStreamThread` / `CdStreamInitThread` (a thread start routine);
  * the `cb_RSCOLLISION_RpClump_*` plugin callbacks.

  **Fix for SAMP-1:** add the CRT `_initterm` / atexit tables and the vtables that samp itself
  instantiates (`0x85BBF0`, `0x85BDC0`, …) as roots, and treat RW plugin registration arguments as
  `fnref` edges.
* **E ∖ samp = 5,682.** These are the candidates to skip that this run supports. 4,737 of them also
  appear in neither the SP corpus nor the samp runs. 2,856 of the 5,682 are AI tasks (E holds 3,602 of
  them through instantiated task vtables). This number is a measurement of these scenarios, not a
  proof: features nobody played (§2, "not covered") sit in it.
* Of the 182 functions that SAMP-1 found reachable only without samp's cuts, 9 ran:
  * `CGame::InitAfterLostFocus` (it has a second caller);
  * `cb_RSCOLLISION_RpClump_StreamRead`. It is a runtime toggle, so the RET patch is only temporary;
  * `CColModel::MakeMultipleAlloc`, `RpHAnimFrameGetHierarchy`, `RwHelper::GetAnimHierarchyFrom*` and
    3 unnamed helpers. All are reached by other paths.
* 256 of the 318 direct functions (named by a samp row) ran.

### 3.2 Porting priority: `catalog/samp-priority.csv`

The CSV lists every function executed under SA:MP whose `catalog/state.csv` state is below L1, i.e.
`unknown`, `specified` or `implemented`. 6,426 rows. Columns:
* `address,name,subsystem,state`;
* `hits`: in how many of the 3 merged runs the entry fired (1–3). For a function whose entry is not
  armed it is the number of hit blocks. Breakpoints are first-hit only, so this is not a frequency
  (see §5);
* `blocks_hit,blocks_total`: block coverage, a measure of how much of the body ran;
* `in_E`, `in_sp` (also executed in the single-player corpus), `role` (init/dtor).

Without the 1,649 static initialisers and destructors, 4,777 functions are left.

| subsystem | samp executed | L1/L2/L3 | priority | unknown | specified | implemented | priority without init/dtor |
|---|---:|---:|---:|---:|---:|---:|---:|
| UNK-game | 1881 | 0 | 1881 | 233 | 1648 | 0 | 417 |
| RW | 822 | 0 | 822 | 84 | 738 | 0 | 820 |
| AI | 797 | 0 | 797 | 26 | 771 | 0 | 761 |
| RENDER | 512 | 0 | 512 | 14 | 498 | 0 | 492 |
| AUDIO | 496 | 0 | 496 | 5 | 491 | 0 | 452 |
| WORLD | 428 | 73 | 355 | 6 | 316 | 33 | 338 |
| VEH | 305 | 1 | 304 | 1 | 303 | 0 | 288 |
| PED | 189 | 2 | 187 | 3 | 184 | 0 | 186 |
| HUD | 162 | 0 | 162 | 0 | 162 | 0 | 154 |
| STREAM | 352 | 201 | 151 | 0 | 5 | 146 | 150 |
| ANIM | 133 | 0 | 133 | 1 | 132 | 0 | 127 |
| POP | 130 | 0 | 130 | 1 | 129 | 0 | 123 |
| SCRIPT | 111 | 0 | 111 | 1 | 110 | 0 | 103 |
| CAM | 94 | 0 | 94 | 0 | 94 | 0 | 88 |
| INPUT | 90 | 0 | 90 | 0 | 90 | 0 | 88 |
| WEAPON | 77 | 0 | 77 | 5 | 72 | 0 | 67 |
| CORE | 189 | 139 | 50 | 1 | 19 | 30 | 50 |
| CALLED-ONLY | 23 | 0 | 23 | 4 | 19 | 0 | 23 |
| PHYS | 55 | 33 | 22 | 0 | 9 | 13 | 22 |
| COLL | 109 | 88 | 21 | 0 | 21 | 0 | 21 |
| MATH | 64 | 57 | 7 | 0 | 6 | 1 | 6 |
| SAVE | 1 | 0 | 1 | 0 | 1 | 0 | 1 |
| **total** | 7020 | 594 | 6426 | 385 | 5818 | 223 | 4777 |

594 of the functions SA:MP executes are already verified (L1/L2/L3), mainly in STREAM, CORE, COLL,
WORLD, MATH and PHYS. Only 223 more are implemented and waiting for verification.

### 3.3 samp's disables: zero-hit check

Cut functions (the entry is patched to `ret` at init, so the entry point is not armed; the body
blocks are the measure). **All 8 have zero hits:**

| function | blocks hit / armed |
|---|---:|
| 0x4083c0 CStreaming::InitImageList | 0 / 15 |
| 0x4090a0 CStreaming::DeleteAllRwObjects | 0 / 29 |
| 0x588be0 CHud::SetHelpMessage | 0 / 15 |
| 0x5b47b0 CFileLoader::LoadPickup | 0 / 50 |
| 0x60dc50 CPlayerPed::FindWeaponLockOnTarget | 0 / 143 |
| 0x60e530 CPlayerPed::FindNextWeaponLockOnTarget | 0 / 85 |
| 0x612710 CPopulation::AddPed | 0 / 79 |
| 0x6a8500 CAutomobile::ProcessFlyingCarStuff | 0 / 1 |

Removed call edges (NOPed call sites, `aggregates.json/removed_call_edges`): 19 of the 23 callees
never ran. The callees include `CReplay::Update`, `CTheCarGenerators::Process`,
`CCutsceneMgr::Update`, `CPlantMgr::Update`, `CCarCtrl::GenerateRandomCars`,
`CPlane::DoPlaneGenerationAndRemoval`, `CCoronas::DoSunAndMoon`, `CRealTimeShadowManager::Update` /
`ReturnRealTimeShadow`, `CIdleCam::Process`, `CPlaceName::Process`, `CHud::DrawVehicleName`,
`CPostEffects::SpeedFX`, `CEntryExit::GenerateAmbientPeds`, `Interior_c::AddPickups` / `Exit`,
`CGarages::IsCarSprayable`, `CStreaming::DeleteRwObjectsAfterDeath` and
`CCarEnterExit::SetPedInCarDirect`. The 4 callees that did run also have other callers:

| callee (removed edge) | why it runs |
|---|---|
| `CMessages::AddBigMessage` (from `CPlayerInfo::KillPlayer`) | samp's game text uses it |
| `CSprite2d::DrawRect` (from `CLoadingScreen::RenderSplash`) | used everywhere |
| `CCamera::ClearPlayerWeaponMode` (from `CPlayerPed::ClearWeaponTarget`) | other callers |
| `CGame::InitAfterLostFocus` (from `__MainWndProc`) | other caller |

So no disabled feature ran through its disabled path. The runtime toggles (input lock, clock,
`CVehicle::SetupRender` and the others) are patched in place. Their sites are excluded from the
points, so this measurement says nothing about them.

## 4. SCM opcodes executed (`docs/samp/scm-opcodes-dynamic.csv`)

Measured with a detour on `CRunningScript::ProcessOneCommand` (0x469EB0; `SA_COV_SCM`, see
`crates/sa-hook/src/coverage.rs`). A stub counts the command word `& 0x7FFF` at `m_pCurrentIP` and then
runs the relocated `inc word ptr [0xA447F4]`. This counts every command that the game's script engine
executes: samp's own main.scm and script.img threads, and the commands samp runs through the
engine's dispatcher.

* **78 distinct opcodes, 1,058,977 commands** in the 3 merged runs. The busiest are 0248 HAS_MODEL_LOADED
  (381,828), 04EE HAS_ANIMATION_LOADED (381,039), 077E GET_AREA_VISIBLE (76,177), 0470
  GET_CURRENT_CHAR_WEAPON (60,050), 00C0 SET_TIME_OF_DAY, 052C, 09BA DISPLAY_ZONE_NAMES and 06BD.
* Compared with the samp-rs static list (SAMP-1 §4.4, 113 opcodes): 71 executed. 42 were not executed;
  they belong to features these scenarios did not use.
* **7 executed opcodes are missing from the samp-rs list:** 06BD IS_LINE_OF_SIGHT_CLEAR, the menu
  family 08D4 CREATE_MENU, 08D7 GET_MENU_ITEM_SELECTED, 08D9 ACTIVATE_MENU_ITEM, 08DA DELETE_MENU,
  08DB SET_MENU_COLUMN and 09DB SET_MENU_COLUMN_WIDTH. samp.dll's menus go through the script engine,
  so the opcode list of §4.4 must grow by these 7.
* samp's custom opcodes 09C6 / 09D3 (patch rows 2/3) were never counted. samp.dll does not send them
  through `ProcessOneCommand` in these scenarios.

## 5. Problems met

* **Wine session without a display (fixed, affects any Wine call before the display starts).** The
  first coverage runs exited before the first frame, with exit code 0 and 2,768 points hit. The cause
  was not the breakpoints. `winepath` ran for the `SA_COV` paths before the headless display started,
  so it started the prefix's Wine session without a display. `CreateWindowExA` in `Win32_InitInstance`
  then failed: `err:winediag:nodrv_CreateWindow` under `WINEDEBUG=err+all`. A bisection over the points
  first blamed one harmless entry. That was a red herring: its single passing probe most likely
  reused a Wine session that still had a display. samp-cov.sh now starts gamescope first; lib.sh already warns about this order.
* **Steam stub audio tracks (worked around).** `full1` died at the third vehicle with
  `0xC0000094` (integer division by zero at 0x4F1464 in the stream thread), and samp's crash dialog
  appeared. The cause is a radio station that starts on a zero-sample Steam stub track. This is a
  property of the 1.0 exe + Steam data install, and samp-rs documents it too. samp-cov.sh now gives the
  lane a private `audio/CONFIG` whose TrakLkup.dat aliases the 122 stubs (samp-rs
  `tools/audio-stub-tracks.py`, copied to `re/samp/tools/`), and restores the lane's symlink at teardown.
* **Frequency counts do not work under samp.** `SA_COV_CAP=64` (re-arming by single step) hung during
  early init: 2,821 points hit, then no connect within 180 s. The runs are therefore first-hit only.
  `hits` in the priority CSV counts runs, not executions.
* Coverage at exit. The game exits at `/q`, so sa-hook now also writes the hits and opcode counts at
  DLL_PROCESS_DETACH, with a `try_lock`: a writer thread ended while holding the lock must not hang the
  exit.
* Server NPC aim/shoot did not damage the player. The local player's own shots, explosions and the
  death were measured.

## 6. Files

* `tools/e2e/samp-cov.sh`: setup, server, injection and scenario driver.
* `tools/e2e/samp/sampcov.pwn`: the gamemode.
* `tools/e2e/samp/samp-points.py`: drops points in samp-touched ranges.
* `tools/e2e/samp/samp-analysis.py`: the report, `samp-functions.csv` (every live function with its
  samp/E/SP flags) and the priority CSV.
* `crates/sa-hook/src/coverage.rs`: `SA_COV_SCM` opcode counter and the flush at process exit.
* `catalog/samp-priority.csv`, `docs/samp/scm-opcodes-dynamic.csv`.
* Raw (gitignored): `catalog/raw/coverage-samp-{full2,full3,remote}.{hits,scm}.csv`,
  `coverage-samp.points.txt`, `coverage-samp.excluded.csv`, `coverage-samp-all.{csv,hits.csv}`.
  Run logs and screenshots: `target/e2e/lane-4/runs/`.
