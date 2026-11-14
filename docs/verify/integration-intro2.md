# INTRO-2: the intro past the police-car ride, natively

Branch `worktree-agent-ab80f5d4fe8147e79`, built on master `e52b9fcf` (INTEGRATE-10) and rebased (cleanly) onto master
`58a1c75b` (MOD-1 roadmap, `--window` pointer / surface fixes, GetBeatInfo / GetTrackInfo ports). master can
fast-forward to it.
Goal: New Game runs the original intro natively (no `--start-in-world`, no `SA_GAME_SCRIPT_SKIP`, no stubs):
airport cutscene → police-car ride → on foot in Ballas territory → BMX to Grove Street → free roam.

## 1. Integration

| from | commits | here | notes |
|---|---|---|---|
| AUDIO-STREAM (`worktree-agent-acc62cadcdbd3be53`) | `e1085d4a`, `24f24b5a`, `ffa6da91`, `c4d33884`, `22e226f8` | `eca373d3`..`1a0ce310` | `sa-game/Cargo.toml`: lewton added next to master's `samp-rs/` path dependencies (its own git pins dropped) |
| INTRO-1 (`worktree-agent-a2c16c35b48eab431`, `421ba072..a87cf585`) | 12 own commits | `0eb10920`..`83c6e71e` | its AUDIO-STREAM copies `aab7ae27` / `299ac780` not taken (AUDIO-STREAM's own commits instead); its `GetStreamID` copy in `5b378d18` dropped (AUDIO-STREAM's `ffa6da91` registers 0x502460); `a7c4c95c` (WRITER-SAMP's FrameUpdateCallBackNonSkinned) skipped: already on master; `c4513221` takes INTRO-1's ANPK port over master's fallback rewind (`99a295f1`) |
| SCRIPT-5 (`worktree-agent-a43f6b653f2e96d67`) | `ca133bcb` (script5-1) and `ca133bcb..17a4881f` (script5-2..13) | `139aacef`..`6951e29c` | keycells `READERS` unioned (45: INTRO-1's 0x4037BC, SCRIPT-5's 0x634440 reader) |
| NEWGAME (`newgame-1`) | not taken | | its CAEDataStream constructor duplicates AUDIO-STREAM's |

Generated files: `tools/gen/exclude_inflight.txt` regenerated (`inflight.py`); `ai_patterns.py --check` and
`patterns.py --check` clean; queue regenerated. One registration per address: `cargo test -p sa-hook --lib` (58 pass,
including `matches_catalogue_and_exe`, `every_hook_installs_on_the_exe`, the keycell and caller checks).

## 2. The intro, natively

Deterministic native run (`--fixed-step 40.5`, menus entered by the synthetic Enter, offscreen, `SA_GAME_PRESENT_GAME=1`;
the scratch script `scratchpad/intro2/run.sh`):

```sh
SA_GAME_PRESENT_GAME=1 GLIBC_TUNABLES=glibc.malloc.mmap_max=0 setarch i686 -R \
  target/i686-unknown-linux-gnu/release/sa-game --game wine/gta-sa --fixed-step 40.5 --frames 6000 \
  --present-dir OUT --report OUT/report.md          # + SA_GAME_PRESENT_SKIP / _EVERY, SA_GAME_KEYS for the BMX
```
 Blockers ported in the order the runs hit them (all byte-faithful
from the 1.0 US bytes; batches `intro3-1`..`intro3-16`, unverified, route L2):

| address | function | notes |
|---|---|---|
| 0x45A980 | `CVehicleRecording::StartPlaybackRecordedCar` (`.HOODLUM` 0x1565F10) | the 8th step of the unrolled free-slot search adds the no-CD VM 0xCB73E0's value (descriptor 0x155FF74): **evaluated on the exe image with unicorn: 7** (464 instructions, no write outside the stack; the trap-flag probe after it adds 0x28 only under a single-step debugger) |
| 0x6432A0, 0x6452C0 | `CTaskComplexDriveToPoint::IsTargetBlocked` (list / ped overloads) | `preserve = "ecx"` (callers 0x6454EC and 0x6452C0 call on with ECX); L1 preserve specs queued |
| 0x4D2100 | `FrameUpdateCallBackNonSkinnedWithVelocityExtraction` | planar variant (no bone position, z untouched) |
| 0x54E9D0 | `CMatrixLinkList::AddToList1` | `gMatrixList` runs out of free links during the intro |
| 0x42F870 | `CCarCtrl::JoinCarWithRoadSystemGotoCoors` (`.HOODLUM` 0x156A4A0) | inline port of the uncatalogued `CPathFind::RemoveBadStartNode` 0x44E4F0 (only caller), whose `this` comes from the VM (descriptor 0x155FFEC, **evaluated: 0x96F050 = ThePaths**) |
| 0x41DA30 | `CCarAI::UpdateCarAI` (partial, std12-pop-1) | missions 8 / 9 / 12 (go to coordinates, stop forever) join the ported cruise path |
| 0x427740 | `CCarCtrl::PickNextNodeToFollowPath` | the existing port registered `#[native]` (protection reads its entry) |
| 0x4B38D0 | `CEventDamage::AffectsPedGroup` | |
| 0x606060 | `FUN_00606060` (decision of a default decision maker) | |
| 0x656000 | `CTaskComplexEvasiveDiveAndGetUp::CreateNextSubTask` | |
| 0x43ADB0 | `CConversations::DoneSettingUpConversation` | |
| 0x70A630 | `CShadows::CastShadowSectorList` | shares the player walk's body without its `+0x1E` exclusion |
| 0x6E91D0 | `FUN_006e91d0`, one layer of a high-detail water rectangle | also ported on WRITER-SAMP-2's branch (`aa6be1d1`): keep one registration when that branch is integrated |
| 0x4590F0, 0x458970 | `CPed::CreateDeadPedMoney` (return-address-obfuscated call decoded: 0x458970), `CPickups::CreateSomeMoney` | |
| 0x65FB60 | `CTaskGangHassleVehicle` destructor | `#[native]` (protection reads its entry: `adc ecx, [0x65FB62]` at 0x401A26) |
| 0x65F700 | `CTaskComplexTrackEntity::CreateFirstSubTask` | |
| 0x4AF2D0 | `CEventPedToFlee` destructor | |
| 0x695090 | `CTaskComplexFollowPedFootsteps::ControlSubTask` | |
| 0x719360 | `FUN_00719360` (font character of a key code) | the BMX help text |

### Determinism

With AUDIO-STREAM the stream decoder really plays; the main thread's channel-volume pass then read the clock a
varying number of times (`hw_rescale_channel_volumes` → `StreamingChannel::GetPlayTime`), so two `--fixed-step` runs
drifted apart from frame ~30 (seen as different blockers per run). `741433bd`: natively `--fixed-step` now runs the
two audio threads (`CAESmoothFadeThread`, `CAEStreamThread`) cooperatively on the main thread, as wasm32 already did
(`bind::coop`), timed by the virtual clock; the original's busy-wait loops on the stream thread (`PreloadCutsceneTrack`,
`StopCutsceneTrack`, the radio / ambience stops) step it through `sa_core::orig::busy_wait()` (a no-op unless the host
installs a handler, so phase A is unchanged). Two runs are identical over 3000 frames (every `det:` line).

### Milestones (evidence in `target/intro2-evidence/`, not committed)

| # | milestone | reached | evidence |
|---|---|---|---|
| 1 | airport cutscene (INTRO-1) | yes | frames 0..~2290 of every run |
| 2 | the police-car ride | **yes**: the recorded ride through Los Santos (car recording playback, the Ballas car's drive-by, the train-track shots, the drop-off), 0 blocked to frame 3900 | `m2-police-ride/` (200 PNGs, frames 2290..3890 every 8), `m2-police-ride.mp4` (2.2 MB) |
| 3 | on foot in Ballas territory | **yes**: "Jefferson", HUD, "Get on the bike." with the BMX marker; without input the run stays there, 0 blocked to frame 6000 | `m3-m4-foot-bmx/` (frames 4400..) and `m3-m4-foot-bmx.mp4` (1.8 MB) |
| 4 | BMX to Grove Street | **partly**: with `SA_GAME_KEYS=w:4700:4728,d:4708:4714,f:4732:4738,w:4760:5600,d:4790:4812` CJ walks to the bike, gets on ("BMX", "Follow the 'CJ' icon..."), pedals across the road and the tracks (East Los Santos), turns south through the alleys and reaches the street where the Ballas wait; there (frame 5231) the bike hits a ped: `CPed::KillPedWithCar` 0x5F0360 (1109 instructions) is unported and every later frame blocks. The scripted route also turns the wrong way (west; the CJ icon is behind-left at the end) | same files (frames 4700..5231) |
| 5 | free roam with the Big Smoke marker | no | |

The runs are deterministic: the keyed run gives the same 5247 ok / first blocker frame twice.

## 3. Landing checks

- Lib tests (release): sa-hook 58 pass; sa-script, sa-core, sa-audio, sa-anim, sa-ai (one run): 14 + 5 + 11 + 73 + 9
  pass, 0 fail.
- `cargo test -p sa-oracle --test l1 --release`: 18 pass, 1 ignored, after queuing the three new preserve specs
  (`PRESERVE_SPECS_PENDING`: 0x6432A0 / 0x6452C0 from this lane, 0x4861F0 `IsVehicleStopped` from SCRIPT-5).
- sa-oracle and satool (sa-oracle-format) rebuilt (host release); `i686-pc-windows-gnu` sa-hook + sa-launcher
  release: clean (the existing `draw_quad_bits` dead-code warning only).
- sa-game `i686-unknown-linux-gnu` release with and without `--features samp`: clean.
- `tools/web/build.sh` and `FEATURES=samp tools/web/build.sh`: clean (46.6 / 46.9 MB `sa_game_bg.wasm`; `41da1eef`
  fixes the wasm32 build of the cooperative pass closure).
- Native smoke run (`--start-in-world --fixed-step 40 --frames 300`, `SA_GAME_SCRIPT_SKIP=1`, offscreen): **316 ok, 0
  blocked, 0 crashed**; 13,445 Rust ports in the `orig!` registry; the same 5 second registrations as INTEGRATE-10.
- Intro run (no `--start-in-world`, no `SA_GAME_SCRIPT_SKIP`, no stubs): **6016 ok, 0 blocked to frame 6000** without
  input (milestones 1-3); with the BMX keys 5247 ok, then blocked from frame 5231 on `CPed::KillPedWithCar`.
- After the rebase: `SA_ROOT=~/sa-engine tools/e2e/verify-all.sh intro2b 4,5` at `1fa2bd6c` (DLL sha256 `b2b68fe3…`):
  **IDENTICAL on all 13 recordings**, 17 subsystems each, same frame counts as below. Generated files regenerated
  on the rebased tree (queue: 2398 unverified hooks in 376 batches; `--check` clean); sa-hook lib tests 58 pass;
  native sa-game with and without `samp`, `FEATURES=samp tools/web/build.sh`: clean; smoke run 316 ok / 0 blocked
  (13,447 ports); the keyed intro run gives the same 5247 ok / first blocker at frame 5231.
- Before the rebase: `SA_ROOT=~/sa-engine tools/e2e/verify-all.sh intro2 4,5` at `fb55a6eb` (the code head for phase A; the commits
  after it touch the wasm32 build and docs only), DLL sha256 `4d478f82…`: **IDENTICAL on all 13 recordings**, 17
  subsystems each (grove walk 5200, air-water 5800, anim-input 3300, car-ai 5000, c-bug 3220, driving 4338,
  menus-hud 5700, menus-map-controls 4000, pause-restart 3700, police-gang 5200, save-load 3900, weapon-combat
  3500, world-cheats 3276 frames). No bisect needed: the new ports are unverified (not in the default set), the
  `#[native]` ones are phase B only, and `busy_wait()` is a no-op in phase A. AUDIO-STREAM's ports are hooks in the
  default set only where already verified; its mixer / stream changes are phase B.

## 4. Open items

Blockers ahead on the intro, in order:
1. `CPed::KillPedWithCar` 0x5F0360 (a vehicle hits a ped; 1109 instructions) — the next blocker on the BMX ride;
   a route that misses the peds may get further, but the Ballas wait at the end of the alley.
2. The BMX route to Grove Street (milestone 4) needs a key script that follows the CJ icon east; then the arrival
   cutscene and free roam with the Big Smoke marker (milestone 5).

Other notes:
- 0x6E91D0 is also ported on WRITER-SAMP-2's branch: keep one registration when integrating it.
- The VM values (0xCB73E0 → 7 for descriptor 0x155FF74, 0x96F050 for 0x155FFEC) come from an emulation of the exe
  image (unicorn, `scratchpad/intro2/vm.py`); other lanes' "assumed" VM values (cdstream, colstore, conversation,
  shopping, script resources) can be evaluated the same way.
- `--fixed-step` audio now runs cooperatively; window / real-time runs keep the real audio threads.
