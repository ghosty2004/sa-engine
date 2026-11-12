# Input-recording corpus

The corpus is a set of scripted input programs (`tools/e2e/programs/*.txt`, language in
`crates/sa-oracle-format/src/program.rs`) recorded with `tools/e2e/record-program.sh` in determinism mode
from process start. Recordings are not committed; the canonical copies live in
`target/e2e/corpus/NAME.sarec` of the main checkout. Every recording must replay IDENTICAL to its record
run (`--replay`), and its screenshots must show the intended scenario before it is kept.

## Practical notes (all programs)

- The test setup runs the game in pad mode: mouse buttons and the C key do not reach the controls. Use the
  pad buttons (`cross`, `circle`, `square`, `triangle`, `l1`/`l2`/`r1`/`r2`, `l3`/`r3`, `select`, the D-pad,
  the sticks). Menus are driven with the D-pad plus `key=enter` / `key=esc`.
- Cheats are typed as `key=std:0xNN` (upper-case ASCII), one key every two frames. The CORPUS-1 programs
  were generated from a compact source in which a line `FRAME cheat WORD` expands into those key lines.
- Frames are 40 ms of virtual time. Every program starts like `grove-walk.txt`: gameplay begins at
  frame 202 and the intro cut-scene is skipped by frame 1200. CJ then stands in an Idlewood alley with the
  first mission (Big Smoke) running ("Press RETURN to jump on the bike").
- Camera-relative controls (on foot, jetpack) depend on where the camera points; the jetpack moves
  relative to the camera, so a long flight can reverse direction when the camera swings round.
- To see a scenario while tuning, poll the screenshots faster than the default once per second (a local
  change to the `sleep` in `record-program.sh`), then record the final version with the default script.

## Recordings

| program | frames | content |
|---|---|---|
| grove-walk | 5,200 | walk, sprint, camera turns around Idlewood (INF-4 acceptance) |
| pause-restart | 3,700 | pause menu twice, new game started from the pause menu |
| world-cheats | 3,276 | weather / time / wanted / traffic cheats, suicide, hospital restart |
| c-bug | 3,220 | desert eagle and sawn-off aimed fire with crouch cancels |
| weapon-combat | 3,500 | every weapon of KJKSZPJ fired at peds, cars, fires |
| anim-input | 3,300 | BMX, jump, punches, crouch, weapon cycling, keyboard and mouse input |
| driving | 4,338 | BMX, Rancher, Bloodring Banger, explosions, quad, Hunter lift-off, Vortex |
| car-ai | 5,000 | traffic and car AI (its program is not in this directory) |
| air-water (CORPUS-1) | 5,800 | Hunter flight, parachute jump, jetpack, water, swimming, diving |
| menus-hud (CORPUS-1) | 5,700 | pause menu pages, map, stats, controller and display setup, radio, busted in a car |
| police-gang (CORPUS-1) | 5,200 | gang fight, 5-star wanted level, chase on foot, killed by the police, two hospital restarts |
| menus-map-controls (PORT-MENU) | 4,000 | map pan / zoom / waypoint / legend options menu, stats.html saved, redefine-controls screen with one key redefined |

### air-water

HESOYAM, then OHDUDE spawns a Hunter on the street outside the alley; CJ walks to it and gets in
(triangle). Cross is the throttle, `r2` yaws slowly, and the stick pushed back (`ly` > 0) flies the
Hunter backwards, south-east over Ganton towards the docks. AIYPWZQP gives the parachute and CJ bails out
at altitude (triangle): free fall, the parachute opens on `circle`, and CJ lands in Willowfield. Then
YECGAA (jetpack: cross up, square down) carries him south to Ocean Docks, where he dips into the water
with the jetpack still on, flies back onto the quay and lands. The jetpack is taken off on the ground
(triangle; this does not work in the air or in the water). CJ walks off the quay edge and swims (cross
pulsed, with the swimming help boxes), dives (square) and swims underwater.

Not covered: a fixed-wing plane (the Hydra and the stunt plane need a runway or the VTOL controls), and a
boat (there is no boat-spawn cheat; none was within reach).

### menus-hud

Pause menu (esc), then with the D-pad and enter: Map (scroll, zoom attempts with `r1`/`l1`, `T` sets a
waypoint target, `L` shows and hides the legend, `Z` toggles the overview), Stats (all eight
categories), Brief, Options, Controller Setup (redefine controls and mouse settings pages, nothing
changed), Display Setup (rows visited, nothing changed). In the game: a Rancher (JQNTDMH) entered and
driven slowly with radio station changes (D-pad up and down) and camera modes (`select`). The Rancher is
spawned next to a police car; the collision gives a wanted star and the police arrest CJ in the car
("Busted"), followed by the police-station restart and the wanted-level help boxes.

**Audio Setup is skipped on purpose:** entering Options -> Audio Setup in determinism mode makes the
original divide by zero at 0x4F1464 (`wine: Unhandled division by zero at address 004F1464`), and the
game hangs. This is a determinism-mode audio issue to investigate before an audio-menu recording can be
made.

### menus-map-controls

Made for PORT-MENU (docs/arch/hud-port.md) to reach `CMenuManager::AdditionalOptionInput`'s map path and the
controller setup screens, which menus-hud does not (its Controller Setup visit never opens screen 0x26).
Pause menu, Map: pan with the D-pad (left, up) and the left stick, zoom in with `l2` and out with `r2`
(held: one step per frame), the waypoint set and cleared with `circle` and set again with `T`, the legend
options menu opened by holding space (one row toggled twice with `cross`, so the blip prefs end as they
were), the legend toggled with `L`. Stats: `S` writes `stats.html` into the lane prefix's user files
(`SaveStatsToFile`). Options -> Controller Setup -> Redefine Controls -> Foot Controls (screen 0x26, mouse +
keys layout, 22 rows): seven rows down to Forward, enter, `W` (Forward's own key: the lane's `gta_sa.set`
is byte-identical after the run, sha256 13c58cc3...), up, then esc back to the game.

Every lane's `gta_sa.set` is part of every replay's start state (it is not reset between runs and the
recording header only fingerprints it). A program that changes a setting or a binding must leave the file
as it found it; check its hash before and after the record run.

Recorded on lane 7 with `--replay`: replay == record IDENTICAL over 4,000 frames.

### police-gang

Walk out of the alley, HESOYAM and KJKSZPJ, the M4 (R2 five times), targeted fire (r1 + circle) at the
peds on the street. Ballas members shoot back and police cars arrive. OSRBLHH twice raises the wanted
level to 5 stars; CJ runs from the police into the East Los Santos backyards and fires from cover until
the police kill him ("Wasted", around frame 2970). He restarts at the Jefferson hospital with the health
help boxes. The JQNTDMH block at frame 2520 runs while CJ is cornered and he does not reach the car. After
the restart SZCMAWO kills him again (second hospital restart), then he walks into Idlewood.

### Not made in CORPUS-1

- mission-start: the first mission (Big Smoke) is already running when the intro ends, but reaching its
  destination (the CJ radar icon, Grove Street) needs closed-loop steering behind the AI riders or a
  well-aimed jetpack flight; a first jetpack attempt overshot to Los Flores. Not finished in the time box.
- sa-mp-like: the SA:MP opcode set cannot be reached from the pad without an SCM path; skipped.

## Verification of the CORPUS-1 recordings

All three recordings were recorded on lane 2 with `--replay`: replay == record IDENTICAL over every frame.

All engine ports on Rust (`tools/e2e/port-l2.sh ports REC --runs "plain rust"`, tags `corpus1-NAME`):

| recording | result |
|---|---|
| air-water | DIVERGED at frame 2742 on the CORPUS-1 build. Bisection (`l3.sh --rust ports`, after its mode-list fix) found two culprits: `CTaskSimpleCarJumpOut::ProcessPed` (0x64DD60; the port skipped the `_CIpow` factor update when the task has no anim yet, which the original runs: `je 0x64DDDA` at 0x64DD9C; frame 2742, peds/player) and `CStreaming::StreamOneNewCar` (0x40B4F0; its boat `rand()` was attributed to 0x1563BF1 instead of the return address 0x1563BF7; frame 4997, randcalls only). Both fixed: IDENTICAL over 5,800 frames. |
| menus-hud | IDENTICAL over 5,700 frames |
| police-gang | IDENTICAL over 5,200 frames |

Newly reached ports. The NOCALLS list is every port hook whose row in `docs/arch/*` (tables and verdict
TSVs) says "no call" / NOCALLS: 772 hooks. An L2 run of that list (`--runs "plain l2"`) is not usable as a
call counter: one of the 772 hooks makes the L2 double run fault at frame 1280 (`page fault ... at address
00734864`), before any new scenario starts, and a run that dies reports no call counts. The counts below
come instead from INF-10 entry coverage (`tools/e2e/inf10-coverage.sh --no-blocks`, ports on the
original; the coverage trace was IDENTICAL to the plain replay on all three recordings):

| recording | NOCALLS ports executed | of those, not executed by grove-walk + pause-restart |
|---|---|---|
| air-water | 87 | 54 |
| menus-hud | 63 | 29 |
| police-gang | 62 | 28 |
| union | 110 | 76 |

39 of the 772 are already executed by grove-walk or pause-restart (the only recordings with coverage
files so far), so the documented NOCALLS lists are partly stale; the "not executed by grove-walk +
pause-restart" column is the better measure of new reach, though some of those may also be reached by
recordings without a coverage file yet (driving, weapon-combat, c-bug, anim-input, world-cheats,
car-ai). Examples of new reach: radio track manager (station choice, StartRadio), heli renderer
callbacks, CWanted::SetWantedLevel / CheatWantedLevel / RemovePursuitCop, CPlayerInfo::ArrestPlayer,
CRestart::FindClosestPoliceRestartPoint, CStats::FindCriminalRatingNumber, CPad::IsDPadUpPressed,
CCamera::InitCameraVehicleTweaks, door-state queries of CAutomobile.
