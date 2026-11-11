# PORT-AUDIO-1: subsystem AUDIO, first pass (crate `sa-audio`)

First pass over the catalogue subsystem AUDIO (842 rows in `catalog/port-order.csv`; 496 in
`catalog/samp-priority.csv`). This pass ports the game side — `CAudioEngine`, the `CAEAudioHardware`
wrappers, `CAESound` / `CAESoundManager`, the `CAE*AudioEntity` classes, the radio / ambience / cutscene /
user-radio track managers, `CAEAudioUtility`, `CAEAudioEnvironment`, `CAudioZones` and the AUDIO static
initialisers — leaves and small functions first, from the SA:MP priority list. Sound output stays
original in phase A: DirectSound channels (`CAEAudioChannel` buffer calls, `CAEStaticChannel`,
`CAEStreamingChannel`), the stream thread and its decoders (`CAEStreamThread`, `CAE*Decoder`,
`CAEDataStream`), the bank / track loaders and `CAESmoothFadeThread` are reached through `orig!` (direct
or `vslot`). Follows ADR-001 and the PORT-HUD pattern. The main session sets `catalog/state.csv`; the
per-function verdicts are in `docs/arch/audio-port-verdicts.tsv`.

Ported: **288 `#[hook]` registrations** plus 16 ported functions without a hook (below). Work split:
the lead ported `CAudioEngine` (68) and `CAEAudioHardware` (40); helper A `CAESound`/`CAESoundManager`,
utility/environment/zones, the track managers and the static initialisers (109); helper B the vehicle,
ped / speech and other entities (71 after the 5 unhooked below).

## Crate

`crates/sa-audio`. Same rules as sa-hud: `#![no_std]`, `#![deny(unsafe_code)]`, game state through
`globals!`/`GPtr`, every other catalogued function through `orig!`, every port registered with `#[hook]`
and listed in its module's `HOOKS` (`sa_audio::hooks::ALL`, which sa-hook appends after
`sa_hud::hooks::ALL`). Builds for host, i686 (`--features in-process`) and wasm32. Depends on sa-core,
sa-math, sa-stream, sa-world.

| file | contents |
|---|---|
| `common.rs` | field helpers (sa-world's), the audio singletons' addresses, `ae_random_int` / `ae_resolve_probability` |
| `engine.rs` | `CAudioEngine` (68): 18 hand-ported members and 50 tail-jump forwarders (generated from the bytes) |
| `hardware.rs` | `CAEAudioHardware` (40): bank/sound loading wrappers, channel setters, scaling factors, track wrappers, pause/resume |
| `sound.rs` | `CAESound` and `CAESoundManager` (17) |
| `utility.rs` | `CAEAudioUtility` (6), `CAEAudioChannel` byte/ms conversions and constructor, `CAudioZones` (4), `CAEAudioEnvironment` (5) (18) |
| `radio.rs` | `CAERadioTrackManager` (18), `CAEAmbienceTrackManager` (6), `CAECutsceneTrackManager` (3), `CAEUserRadioTrackManager` (3) |
| `statics.rs` | 44 AUDIO static initialisers (`__init_g_*` constants, the audio singletons' constructors + `atexit`) |
| `vehicle.rs` | `CAEVehicleAudioEntity` (17 hooked) |
| `ped.rs` | `CAEPedAudioEntity`, `CAEPedWeaponAudioEntity`, `CAEPedSpeechAudioEntity`, `CAEPedlessSpeechAudioEntity` (16) |
| `entities.rs` | weapon, global weapon, collision, door, fire, explosion, weather, script, frontend, police scanner, twin loop, water cannon entities (38) |

Shared-crate changes: sa-hook `Cargo.toml`, `table.rs` (sa-audio's hooks appended after sa-hud's),
`callers.rs` test; sa-oracle `Cargo.toml`, `hook_preserve` (lib.rs) and `tests/l1.rs` chain sa-audio's
registrations. `tools/e2e/l2-extras.txt` section `# PORT-AUDIO` (below), lists `tools/e2e/lists/audio-*.txt`.

## Conventions and findings worth knowing

- Ported from the 1.0 US bytes (objdump of the exe incl. the `.HOODLUM` bodies); gta-reversed and the
  RE-SPEC-3 specs (`docs/re/specs/AUDIO/*.md`) as reading aids.
- **Tail-jump forwarders** (`CAudioEngine` members are mostly `mov ecx, <manager>` / `add ecx, <member>`
  / `mov ecx, [ecx+0x1E34]` + `jmp`): ported as a call of the same target with `this` adjusted; every
  stack word is passed through as `u32` (bit-exact for float and byte arguments whose upper bytes the
  original forwards unchanged; no x87 round trip of a signalling NaN). The return follows the C++ type:
  `bool`/`int8` = AL, `int32`/pointer = EAX, `void` = none. First run: returning the target's whole EAX
  for `bool` targets that set only AL (`HasRadioRetuneJustStarted`, `IsRadioRetuneInProgress`,
  `IsCutsceneTrackActive`, `GetCurrentRadioStationID`) and for a void target (`CAEAudioHardware::StopTrack`)
  gave EAX mismatches (upper bytes = the caller's leftovers); fixed by typing the returns.
- `CAudioEngine::StopCutsceneTrack(bool wait)` / `StopAmbienceTrack(bool)` loop on
  `CAEAudioHardware::Service` (hooked by determinism mode) until the track is inactive, as the bytes; the
  cutscene variant then restarts the radio: station byte +2 < 0 → `StartRadio(StaticGetPlayerVehicleAudioSettingsForRadio())`,
  else `StartRadio(station, 0, 0, 0)` and +2 = 0xFF.
- `CAEAudioHardware::GetChannelPlayTimes` stores `channels[slot] - 1` play times starting at channel index
  `slot` (the original's `lea edx, [edx+eax-1]` bound, compared unsigned) — one fewer than the slot has.
- `CAEAudioHardware::SetBassSetting` (entry `jmp` into the `.text` chunk 0x4014AA, inline hook with a
  7-byte expect) stores the sign-extended setting as a dword at +0xD90 and forwards the argument words.
- `CAEAudioHardware` hooks declare `this = "CAEAudioHardware"` (0x1014) for L2.
- RNG (helper findings): `CAEAudioUtility::GetRandomNumberInRange<int>` / `<float>` / `ResolveProbability`
  call CRT rand inside `orig::at_site` with the bytes' return addresses 0x4D9C17 / 0x4D9C56 / 0x4D9C97;
  callers that the exe routes through 0x4D9C10 (not CGeneral) use it: `GetHornState` (0, 7),
  `ChooseCollisionSoundID`, `SetNextPlayTime` (1, 1000), `SwapSounds`, `CAEPedSpeechAudioEntity::StaticInitialise`
  (3000, 7000; it draws 720 times: `Reset` then 360 inline redraws).
- Helper A: `CAudioZones::Update` reads the frame counter through the protection (0x403A48 decodes
  0xB7CB4C), test `counter & 15 == 6`; `StartRadio-manual` clamps stations ≥ 14 to 13 inside the argument
  slot's low byte; `CheckForStationRetune` / `SetBassEnhanceOnOff` pass `(this & ~0xFF) | byte` (the
  original's `push ecx` slot); `CAESound::Initialise` writes the flags after `SetPosition`;
  `ChooseTracksForStation` follows the bytes' branch order.
- Helper B: `IsGlobalContextImportantForInterupting` / `...ForStreaming` follow the exe's jump tables;
  `UpdateVehicleEngineSound` returns the sign-extended type in EAX; `ReportBulletHit` replaces only the
  low byte of the surface argument dword; `GetVolForPlayerEngineSound` returns the argument dword read as
  a float for unknown states.
- `CAETwinLoopSoundEntity::~CAETwinLoopSoundEntity` is void (first run returned the `CAESound`
  destructor's EAX; the original's EAX after its SEH epilogue differs; no caller reads it).
  `CAERadioTrackManager::DisplayRadioStationName` likewise void (its early return left EAX 0).

### Not hooked (ported, reached through `orig!`)

| address | function | reason |
|---|---|---|
| 0x4D9E10 | `CAEAudioUtility::FindVehicleOfPlayer` | protection reads the entry as data (`sbb ebx, [0x4D9E12]` at 0x1561A3A) |
| 0x4E37B0 | `CAEPedSpeechAudioEntity::Reset` | protection reads the dword at 0x4E37B4 (at 0x156FF88) |
| 0x4EF880 | `CAESound::SetPosition` | caller 0x4EFDE4 reads ECX after the call (needs `preserve = "ecx"` + an L1 spec) |
| 0x4E44F0 | `CAEPedSpeechAudioEntity::IsGlobalContextPain` | caller 0x4E6683 keeps EDX |
| 0x4E4600 | `CAEPedSpeechAudioEntity::IsGlobalContextImportantForInterupting` | callers 0x4E5F29, 0x4E5F76, 0x4E668D keep ECX/EDX |
| 0x4F4FF0 | `CAEVehicleAudioEntity::IsAccInhibitedForLowSpeed` | callers 0x4FBDE9, 0x4FBF55, 0x4FC29B, 0x4FC7F8 keep ECX |
| 0x4F6000 / 0x4F5F30 | `GetVehicleNonDriveWheelSkidValue` / `GetVehicleDriveWheelSkidValue` | callers keep EDX (0x4F9062, 0x4F907D / 0x4F9094) |
| 0x4F5D00 | `CAEVehicleAudioEntity::GetVolForPlayerEngineSound` | ten callers keep ECX (0x4FBE51, ...) |
| 0x4F5C60 / 0x4F60B0 | `GetFreqForIdle` / `GetBaseVolumeForBicycleTyre` | their loops branch back to entry+2, inside the patch |
| 0x4F6150, 0x4F51F0, 0x4F53D0, 0x4FA0C0, 0x4F8DF0 | `GetFlyingMetalVolume`, `GetVolumeForDummyIdle`, `GetVolumeForDummyRev`, `ProcessBoatMovingOverWater`, `ProcessReverseGear` | **hooked again (X87-MODELS, 2026-11-11), L2 pending**: the port now multiplies `utility::audio_log10`'s full 64-bit FYL2X result (`X87::fyl2x` model) as the original does; the AudioLog10-times-constant composite is exact against the CPU on 10^6 cases (L1 spec `AudioLog10*c`). Before: the port rounded AudioLog10 to f32 first (1-ulp ST0 mismatches, `GetVolumeForDummyIdle` 30/1001, `...Rev` 4/1001 on grove, `port-l2-ab-g`) |

Not hookable and not ported: entries below 5 bytes — `CAERadioTrackManager::HasRadioRetuneJustStarted`
0x4E8370 and `GetRadioStationListenTimes` 0x4E83E0 (4 bytes), `CAEStreamThread::CAEStreamThread` 0x4F11D0
(3), `CAEStreamThread::GetTrackLengthMs` / `GetActiveTrackID` / `GetPlayingTrackID` 0x4F1550 / 0x4F1560 /
0x4F1570 (4). `CAEAudioHardware::SetChannelVolume` 0x4D8870 (entry `jmp` into an obfuscated `.text`
chunk at 0x402182) and `CAESoundManager::RequestNewSound` 0x4EFB10 (loop back into its patch) were not
ported. Excluded by scope: `CAEAudioHardware::Service` 0x4D9870 (determinism hook), the thread `Start`s
0x4F11F0 / 0x4EEA10 (patched `ResumeThread` sites inside), `CAEAudioUtility::GetCurrentTimeInMS` 0x4D9E80
(patched QPC site), the stream thread, decoders, loaders and DirectSound channel calls.

Helper B also left (protection code inside the body): `StaticGetPlayerVehicleAudioSettingsForRadio`,
`PlayerAboutToExitVehicleAsDriver`, `~CAEPedAudioEntity`, `CAEPedSpeechAudioEntity::Initialise`,
`StopCurrentSpeech`, `LoadAndPlaySpeech`, `UpdateGenericWaterCannonSound`, `UpdateGenericVehicleSound`,
`ProcessVehicleSirenAlarmHorn`, `ProcessRainOnVehicle`, `PlaySkidSound`; functions that build an
uninitialised `CAESound`/`CVector` on the stack and copy it (`StartVehicleEngineSound`, the `Play*`
functions, `PlayShirtFlap`, script `Service`); `CAEFrontendAudioEntity::Initialise` (a `LoadSound`
argument is `2*ECX+1` with ECX left by the previous call); `CanWePlayGlobalSpeechContext` (AX result,
path-dependent upper EAX).

## Verification (commands, results)

All game runs on lane 6 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=6 L2_PER_FRAME=1000 SPLIT_PER_FRAME=1000`,
`E2E_BIN` = a snapshot of the release DLL, under `systemd-run --user --scope -p MemoryMax=14G`), outputs in the
worktree's `target/e2e/lane-6/port-l2-<tag>/`. Snapshots: `lead2` (lead hooks only, helper modules
stubbed: the helpers' code was still changing) for the lead lists, `full2` (all 288 hooks) for the helper
lists and the all-ports runs. Lists: `tools/e2e/lists/audio-lead.txt` (108), `audio-lead-split.txt` (20),
`audio-a.txt` / `audio-b.txt` (helpers' own lists), `audio-ab.txt` (180 = a + b), `audio-ab-l2.txt` (150),
`audio-ab-split.txt` (30), `audio-ab-l2b.txt` (4, rerun after the last extras).

- **L2, lead hooks** (`audio-lead.txt`, plain/l2/rust): grove (`audio-lead-g2`), driving (`al-dr`),
  weapon-combat (`al-wc`), pause-restart (`al-pr`), world-cheats (`al-wch`): every hook with calls 0
  mismatches and 0 undeclared writes except the four that cannot run twice on the same state
  (`CAEAudioHardware::StopSound`, `CAudioEngine::StopCutsceneTrack`, `StartLoadingTune`,
  `ReportWeaponEvent`: channels / heap entities / references the first run already changed), which are
  judged by split runs. (b) l2 == plain and (c) rust == plain on all frames in all five.
- **Split, lead** (`audio-lead-split.txt`, `als-g` / `als-dr` / `als-wc`): 12 PASS (+1 PASS-FP on driving
  and weapon-combat), 0 FAIL / DIVERGED / COUNT, 0 undeclared after the extras.
- **L2, helper hooks** (`audio-ab-l2.txt`, `abl-{g,dr,wc,pr,wch}` + `abl2-{dr,wc,pr,wch}`): 0 mismatches
  everywhere; the remaining undeclared writes of the first pass (SwapSounds, CAEWeaponAudioEntity::Terminate,
  ambience StopAmbienceTrack / StartTrackPlayback) were declared and the four reruns are clean. (b) and (c)
  PASS on every run.
- **Split, helper hooks** (`audio-ab-split.txt`, `abs-{g,dr,wc}`): grove 15 PASS / 2 DIVERGED / 4 COUNT,
  driving 22 PASS / 2 DIVERGED / 2 COUNT, weapon-combat 20 PASS / 1 FAIL / 1 DIVERGED / 3 COUNT. COUNT =
  the 2 GiB spool quota was reached at frame 1831 (whole `AESoundManager` regions, 36 KB per case); the
  hooks concerned PASS on the other recordings. DIVERGED / FAIL are `CAESound::Initialise` and
  `CAESound::operator=`: their `CAESound` lives in the caller's stack frame, whose unwritten bytes (padding,
  uninitialised fields) differ between the original and the Rust run (input divergence at +0x55..), so the
  split cannot pair them; the weapon-combat FAIL came from a write-only declaration of `this` tried for
  that run (garbage bytes compared), reverted to RW. Both stay `implemented` (open item).
  `CAESound::UpdateParameters` PASSes but its splits reported writes after `CCollision::ms_colModelCache`,
  declared afterwards and not rerun.
- **L3, all engine ports on Rust** (`ports`: 2662 hooks incl. the 288 AUDIO ones, snapshot `full2`,
  `port-l2-ports-{g,pr,wch,cb,ai,wc,dr}`): IDENTICAL over grove walk 5200, pause-restart 3700,
  world-cheats 3276, c-bug 3220, anim-input 3300, weapon-combat 3500 and driving 4338 frames, 17
  subsystems; no crash / panic / install failure. Per-hook hook-off runs were not made.
- **First-pass fixes found by L2** (before the final snapshot): EAX vs AL returns of the forwarders (above);
  the 5 `AudioLog10` users (unhooked); `CAETwinLoopSoundEntity::~CAETwinLoopSoundEntity` and
  `CAERadioTrackManager::DisplayRadioStationName` returned a stray EAX (now void). Every L3 (c) check of
  every run, including the first ones, was IDENTICAL.
- **L1**: no committed spec (see Open items: `ConvertFromBytesToMS` / `ConvertFromMSToBytes`, 1,002,000
  Unicorn cases each, 0 mismatches, cannot be routed under the current hazard rules).
- Tests: `cargo test -p sa-audio` 3 pass (list complete/unique, globals declared once across all engine
  crates, initial values match the exe); `cargo test --release -p sa-hook --lib` 48 pass (installs,
  catalogue names, keycells, INF-20 data reads, callers, patch_refs); `cargo test --release -p sa-oracle
  --test l1`: 15 pass, 1 fails (`preserved_registers_match_the_hook_registrations`: sa-script's
  `CRunningScript::GetPointerToLocalVariable` 0x463CA0, pre-existing, not from this pass). clippy `-D
  warnings`: sa-audio host (all targets), i686 (`--features in-process`), wasm32; sa-oracle host (all
  targets); sa-hook i686. `cargo fmt -p sa-audio --check` clean.

Verdict counts (288 hooks + 16 ported without a hook): **L2 219**, **implemented-needs-L2 65** (no call
in the five recordings: radio station changes, mission audio, script/police scanner entities, most
`CAEAudioHardware` getters, `PauseAllSounds`, SA:MP-relevant `ReportMissionAudioEvent` variants),
**implemented-inconclusive 2** (`CAESound::NewVPSLEntry` COUNT on all three splits, `CAESound::operator=`
DIVERGED), **implemented-L2-FAIL 1** (`CAESound::Initialise`, split on stack garbage, see above),
**implemented-needs-rerun 1** (`CAESound::UpdateParameters`), **ported-unhooked-x87 5**,
**ported-unhookable 11**. No mismatch in any L2 case of the final code.

### L2 extra regions

`tools/e2e/l2-extras.txt`, sections `# PORT-AUDIO`: whole audio singletons (layouts.csv sizes:
`AEAudioHardware` 0x1014, `AESoundManager` 0x8CBC, `AESmoothFadeThread` 0x81C, `AudioEngine` 0x1FD8,
`AERadioTrackManager` 0x370, the ambience / cutscene / user-radio managers, `g_numSoundChannelsUsed`),
`CReferences::aRefs` + free head (sounds register with their entity), `CAudioZones` (0xB6DC6C, 0xF9C), the
PedSpeech (0xB613D0, 0x984), vehicle (0xB6B98C, 0x3C) and radio-history (0xB61D78, 0xEFC) statics, the CRT
`atexit` byte 0xC9D428 for the static initialisers, `this` (`ecx:<class size>`) for the thiscall entity
and `CAESound` methods (heap-resident counters), and the out-pointers (`GetVirtualChannelSound*` 600
bytes, `GetReverbEnvironmentAndDepth`, `GetPositionRelativeToCamera`, `GetAccelAndBrake` params). Each
group's comment names the run whose undeclared writes it covers.

## Open items

- Done (X87-MODELS): `X87::fyl2x` model; `utility::audio_log10` returns the wide ST0 and the five vehicle
  functions are hooked again (need L2 on a game lane). `AudioLog10` 0x4D9E50 itself stays unhooked: its
  raw ST0 matches the CPU in ~99.4% only (1 ulp of 64 otherwise), and an `X87` hook return goes back
  through `fld qword` (53 bits); hooking it needs an 80-bit `AbiRet` (shim ends with `fld tbyte`).
  `CAEAudioUtility::StaticInitialise` and `CAERadioTrackManager::UpdateRadioVolumes` can now use
  `audio_log10` the same way.
- The ECX/EDX-preserving callers above: hookable with `preserve` once L1 specs comparing the registers
  exist (L1 cannot run functions that write globals; most of these read entity state).
- L1 for the pure conversions: `CAEAudioUtility::ConvertFromBytesToMS` / `ConvertFromMSToBytes` ran
  1,002,000 cases each under Unicorn with 0 mismatches, but CRT `floor` takes its SSE2 path (`stmxcsr`),
  so `tests/l1.rs routes_match_hazards` forbids Route::L1 and its L2 route requires an x87 hazard the
  function does not have; the specs were therefore not committed (decision for the sa-oracle owner: a
  route for SSE-only hazards, or native-backend runs). Their parity evidence is L2.
- Hook-off (per-hook L3) runs were not made, so no row claims L3.
- Recordings with radio, missions (mission audio, script entity), cutscenes and SA:MP sessions are needed
  for the `implemented-needs-L2` rows.

## Functions

One row per hook (state and evidence per recording) in `docs/arch/audio-port-verdicts.tsv`.

# PORT-AUDIO-2: second pass (generated ports + hand ports)

Second pass over the AUDIO rows still `specified` after PORT-AUDIO-1 (529 + 9 `unknown`) and the 173 UNK rows
`catalog/unk-classification-proposal.csv` assigns to AUDIO; SA:MP priority first (`catalog/samp-priority.csv`).
Lead + 2 helpers in new files only: helper A `port2_a.rs` (radio, police scanner, script, frontend, cutscene /
ambience managers, `CAudioEngine`, `CAESound` / `CAESoundManager`, `CAEAudioHardware` wrappers, the MP3 bank /
track loaders' game-side lookups, `CAudioZones::SwitchAudioZone`, the 133 AUDIO static initialisers); helper B
`port2_b.rs` (ped / ped speech / pedless speech / vehicle / weapon / global weapon / fire / collision / door /
weather / water-cannon / twin-loop entity bodies). Per-function verdicts: `docs/arch/audio-port-2-verdicts.tsv`
(column `origin`: `gen`, `hand-a`, `hand-b`).

Totals: **370 `#[hook]` registrations** (75 generated, 219 helper A, 76 helper B) plus **26 ported without a
hook** (13 generated, 3 A, 10 B). Sound output and decoding stay original (channels, stream thread, decoders,
data streams, smooth-fade thread), as do `fyl2x` users (waiting for the X87 branch).

## Generated ports (`tools/gen/audio_patterns.py` → `gen_{leaves,virtuals,ctors,dtors,bodies}.rs`)

`audio_patterns.py` imports `tools/gen/ai_patterns.py` unchanged — AI-GEN's exact symbolic x86 interpreter,
emitter, hookability rules and body-hash test (`docs/arch/ai-gen.md`) — and changes only the candidate set and
the output: AUDIO + UNK-proposed-AUDIO rows minus every `#[hook(addr` in `crates/` (so the hand ports are never
duplicated), PORT-AUDIO-1's unhooked ports, the out-of-scope classes and `EXCLUDE_ADDR`
(`CAEAudioHardware::Service`, the thread `Start`s, `GetCurrentTimeInMS`, `AudioLog10`). Every body the
interpreter executes completely is emitted; bodies outside AI-GEN's named families go to `gen_bodies.rs`.
`--check` regenerates in memory and compares; `gen_check.rs` (sa-ai's) re-hashes every matched span against the
exe in `cargo test -p sa-audio`; the generated modules are `#[rustfmt::skip]` so `--check` stays byte-exact.
Report: `target/audio-gen/{report,rest}.tsv` (169 candidates left on this tree: x87 bodies, stack locals, loops, ...).
Note: the candidate set excludes every address hooked anywhere in `crates/`, so `--check` must run on the same
tree (it passes on this branch).

AUDIO-specific generator rules, each found by a run:
- `bool` members (`Can*`/`Is*`/`Has*`/`Does*`/`Will*`/`Are*`) return AL only, also when a path returns a
  callee's EAX: `CAEPedSpeechAudioEntity::CanPedHoldConversation` returned `CanPedSayGlobalContext`'s upper EAX
  bytes, which are its caller's leftovers (port-l2-a2gen-air-water: orig 0x850000, rust 0x4E5700).
- Placeholder catalogue names (`(missed by ghidra: ...)`) are skipped: sa-hook's table test wants the catalogue
  name as the hook name, which no hook list can carry.
- `CAEPedSpeechAudioEntity::Constructor` 0x4E4F10 is skipped: caller 0x4E6072 reads EDX at 0x4E6077 (sa-hook
  `callers` test; the first menus-hud rust run, before the exclusion, faulted there writing through EDX = 0).

## Hand ports: conventions and findings (helpers' reports, checked by the runs)

- Every expect re-decoded with objdump; no entry in `catalog/code-data-reads.csv`; every direct caller checked for
  ECX/EDX kept across the call; the whole exe scanned for branches into entry+1..+4 (one hit,
  `ReservePlayerConversationSpeechSlot` 0x4E3887 → 0x4E3872, left unhooked).
- RNG: every draw the exe routes through `CAEAudioUtility::GetRandomNumberInRange` 0x4D9C10 / 0x4D9C50 /
  `ResolveProbability` 0x4D9C80 goes through `orig!` to it (never CGeneral).
- x87: f32 arithmetic where every intermediate is rounded by the 24-bit precision control; `ftol` through
  sa-math `X87`. The 133 static initialisers (`fld; f{mul,div,sub}; fstp`, `fld a; fmul b; fdivr n; fstp`,
  `push X; call atexit`) run under the CRT's PC=53 (product exact, computed in f64): they would mismatch if a
  harness ever called them under PC=24.
- Exe-vs-spec / gta-reversed differences (the exe wins): `GetVoiceForMood` draws `rand(0,1)` before reading
  the mood, result `rand + 2*mood`, moods outside 0..9 get +10; `SetCJMood` keeps only moods 1/5/7/9 (else -1);
  `DemandBankSlot` evicts the first slot 1..9 whose count is below slot 0's (not the minimum);
  `CAEGlobalWeaponAudioEntity::UpdateParameters` addresses 0xB612F0.. directly, not through `this`;
  `HandleLandingEvent` on water clamps a non-positive (or NaN) volume to 0, play position 0x32, sfx 6 for event
  0x3A; `HandleSkateEvent` sfx = 7 + (event != 0x38); `WeaponReload` type 33 plays only for event 0x92 (volume
  -6); `ReportWaterSplash` second sound speed 0.0; `PlayMiniGunFireSounds` 0x97 sound time-scale 0.0;
  `GetSoundAndBankIDs` picks the candidate with the largest phrase-memory index when all are remembered;
  `CanWePlayGlobalSpeechContext` pain contexts 0x154..0x166 use a per-entity next-play table at `this`+0xB4;
  `ChooseDJBanterIndex`'s sunny branch tests `hours >= 8 && hours < 6` (never true; the forecast calls still
  run); `ChooseTalkRadioShow` returns EAX = drawn index with the low byte replaced by the pick;
  `TrackRadioStation` fails after 5 game hours / record total + 160 s / resumed type 6, elapsed ≥ 7000 ms,
  station 12 queues `QueueUpTracksForStation(12, 4)` twice; police-scanner `PlayLoadedDialogue` searches from the
  first *playing* slot; `CheckForStationRetuneDuringPause` continues only for a non-zero station byte +0xE9 or a
  fully-off radio; `CalculateFrequency` randomises only when `0 < var < freq`; `GetMissionAudioPosition`
  returns null only for bitwise (-1000,-1000,-1000) or an all-0.0 position; `CAEScriptAudioEntity::UpdateParameters`
  plays bank sound 0x488 in the literal slot 3; `CAEAudioHardware::PlaySound` (not ported) accepts channel index
  64 (`cmp cx,0x40; ja`). `UpdateAmbienceTrackAndVolume` 0x4D6E60 was not ported: it contains `fldlg2; fyl2x` at
  0x4D6ED0 (its 9 spec differences remain to be listed with the X87 branch).
- Stack `CAESound`s (`PlayDoorSound`, `PlayFireSounds`, `PlayWaterSounds`, `PlayLoadedSound`, police-scanner
  `PlayLoadedDialogue`, script `PlayLoadedMissionAudio`): the original leaves the local uninitialised except
  what `Initialise` writes and `RequestNewSound` copies it; the ports zero it. Same open item as PORT-AUDIO-1's
  `CAESound::Initialise`; `ReportCollision` showed it in L2 (below) and is unhooked.

### Not hooked (ported, reached through `orig!`)

| address | function | reason |
|---|---|---|
| 0x4D8810, 0x4EF630, 0x4E6090, 0x4E60B0, 0x4E4120, 0x4E4130, 0x4E3CD0, 0x4E5F10, 0x4F4F00 | `GetNumAvailableChannels`, `GetVirtualChannelForPhysicalChannel`, pedless `GetAllocatedVoice` / `GetPedType`, speech `GetAllocatedVoice` / `GetPedType` / `GetVoice` / `CanWePlayGlobalSpeechContext`, `GetVehicleTypeForAudio` | AX-only results whose upper EAX is the caller's leftover on some paths (menus-hud L2: e.g. `GetVoice` orig 0x36000A rust 0xA, `CanWePlayGlobalSpeechContext` 92 cases); `AbiRet` has no AX kind, so L2 compares EAX |
| 0x4DBA10 | `CAECollisionAudioEntity::ReportCollision` | stack `CAESound` garbage copied into AESoundManager (4..222 mismatching cases per recording) |
| 0x4E52A0 | `CAEPedSpeechAudioEntity::ReleasePedConversation` | 1 case each on air-water, police-gang, grove: a sound's physical pointer stays set in the Rust run after `StopCurrentSpeech`; cause not found |
| 0x4E3870 | `ReservePlayerConversationSpeechSlot` | loop branches back into the patched entry (0x4E3887 → 0x4E3872) |
| 0x4DFB60 | MP3 request-queue reset | its only caller calls `LoadBankSlotFile` next without reloading ECX |
| 13 generated | short entries (< 5 bytes), `AttachMissionAudioToPhysical` (protection read) | see the verdicts file |

Not ported (helpers' reports): `fyl2x` users (`UpdateAmbienceTrackAndVolume`, `RescaleChannelVolumes`,
`UpdateRadioVolumes`, weather `Service` / `UpdateParameters` / `AddAudioEvent`, `UpdateLoopingCollisionSound`,
vehicle `UpdateParameters`, `AddAudioEvent-1`, 0x4DA510); protection entries (`CAEPedAudioEntity::AddAudioEvent`,
`CAEExplosionAudioEntity::AddAudioEvent`, pass 1's list); `ProcessSpecialVehicle` (AL as an implicit callee input);
the very large `CAEFrontendAudioEntity::AddAudioEvent` / `ProcessMissionAudioEvent` / `CAESoundManager::Service`;
sound output, OS, user-track scan thread and file loading; the rest of the vehicle `Process*` / `Play*` bodies.

## Verification

Lane 7, `E2E_BIN` = snapshots of the release DLL (`target/e2e-bin/h5`, final `h6` = h5 minus the hooks unhooked
above), `L2_PER_FRAME=10 L2_MAX=300`, `L2_EXTRA` = the audio singletons / statics as `D` for every hook (below).
L2 list `tools/e2e/lists/audio2-all.txt` = `audio2-gen.txt` + `audio2-hand.txt` minus ten functions an L2 run
cannot repeat (they loop on `CAEAudioHardware::Service` until the stream thread reports a track state, or build /
tear down an audio singleton: `CAECutsceneTrackManager::PreloadCutsceneTrack`, `CAudioEngine::PreloadBeatTrack`
/ `Reset` / `Constructor` / `Destructor` / `InitialisePostLoading`, `CAERadioTrackManager::Service`,
`CAEAudioHardware::Constructor`, `CAESoundManager::Terminate`, `CAudioEngine::PreloadCutsceneTrack`); an h3 L2
run that included them hung at frame 248 (13 s of CPU in 9 min); the list without them never hung. Those ten are
judged by L3 only.

- **L2 extras.** One RW line per hook and singleton made `SA_L2_EXTRA` 139 KB, longer than a Windows environment
  variable (`/usr/bin/env: Argument list too long`, the l2 run reached 0 frames). The singletons and statics are
  static memory inside the watched windows, so the runs pass them once for every hook as `D` through
  `L2_EXTRA`:
  `0xB5F8B8:0x1014:D,0xB62CB0:0x8CBC:D,0xB608D0:0x81C:D,0xB5F898:0x4:D,0xB6BC90:0x1FD8:D,0x8CB6F8:0x370:D,0x8AC15C:0x2C:D,0x8AE554:0x34:D,0xB613D0:0x984:D,0xB6B98C:0x3C:D,0xB61D78:0xEFC:D,0xB9B9A8:0x5DC4:D,0xBA1768:0x4:D`.
  `tools/e2e/l2-extras.txt` section `PORT-AUDIO-2` holds the heap and per-hook regions: `this` (layouts.csv
  class size) for every thiscall hook, the entity's `CEntity` head (`m_pReferences` +0x24) for ped / speech /
  vehicle entity hooks (`[ecx+4]`; `CAEPedAudioEntity`: `[ecx+0x94]` = `m_pPed`) and for the physical argument
  of `WeaponFire` / `WeaponReload` / `ProjectileFire` / `ReportCollision` (without it the Rust run linked its
  reference node to the one the original run had just added: aRefs next-pointer mismatches), out-pointers by
  argument, per-hook statics (`D`), the static initialisers' globals.
- **Results** (`port-l2-a2-<rec>`: menus-hud and police-gang on h6, air-water, weapon-combat, driving and the
  grove walk on h5): every run (b) l2 == plain and (c) rust == plain IDENTICAL on all frames, 17 subsystems.
  Verdicts (370 hooks + 26 unhooked): **L2 230** (every run with cases: 0 mismatches, 0 undeclared; the
  remaining FAIL lines are the `>= 300 cases` sampling check only), **implemented-needs-L2 138** (no call in
  the six recordings — mission audio, scanner dialogue, user radio, script entity, destructors, `__dtor_*` —
  plus the ten L2-excluded functions), **ported-unhookable 26**. The first runs found and fixed: the EDX
  caller crash (0x4E4F10), `CanPedHoldConversation`'s AL result, the nine AX-only results (unhooked), the
  missing entity / physical reference heads (`HandleFootstepEvent` 300 cases, `HandleLandingEvent`,
  `WeaponFire`, `WeaponReload`), out-pointer and static declarations (`GetSoundBuffer`,
  `GetCollisionSoundStatus`, `GetSoundAndBankIDs`, 0x4DEFE0, the two constructors), and unhooked
  `ReportCollision` / `ReleasePedConversation`.
- **verify-all** (`tools/e2e/verify-all.sh a2final 7`, every engine port on Rust, h6): **IDENTICAL on all 11** (grove 5200, air-water 5800, anim-input 3300, car-ai 5000, c-bug 3220, driving 4338, menus-hud 5700, pause-restart 3700, police-gang 5200, weapon-combat 3500, world-cheats 3276 frames; 17 subsystems).
- Tests: `cargo test -p sa-audio` 10 pass (list complete/unique, globals, generated bodies match the exe);
  `cargo test --release -p sa-hook --lib` 51 pass; clippy `-D warnings` sa-audio host (all targets), i686
  (`--features in-process`) and wasm32 clean; `cargo fmt -p sa-audio --check` clean;
  `cargo test --release -p sa-oracle --test l1`: 15 pass, 1 fails (`l2_specs_pass_on_the_cpu`:
  `RwMatrixRotate` 0x7F1FD0, FSIN/FCOS on the native backend, 2476 mismatches; not AUDIO, not from this pass).

### Open items / decisions needed

- An `AbiRet::Ax` return kind (AX compared, upper EAX the caller's) in sa-core / sa-hook / sa-oracle would let
  the nine AX-only functions above be hooked.
- Stack `CAESound` locals: the original copies uninitialised stack bytes into AESoundManager. Either an L2/split
  normaliser for the unwritten `CAESound` fields, or a decision that ports may zero them (ledger entry). Decides
  `ReportCollision` and the six hooked `Play*` ports that build a stack sound.
- `ReleasePedConversation` 1-case mismatch: investigate with a split run (`SPLIT_PER_FRAME ≤ 10`).
- X87 `fyl2x`: unblocks `UpdateAmbienceTrackAndVolume` and the other users listed above.
- Recordings with missions / cutscenes / scanner dialogue / user radio for the `implemented-needs-L2` rows.

# PORT-AUDIO-3: third pass (fyl2x users, vehicle / entity bodies, AX results, stack residue)

Third pass over the AUDIO rows still `specified` after PORT-AUDIO-2 (192 in scope after removing the sound-output
classes). Lead + 2 helpers in new files: lead `port3_lead.rs` (fyl2x users outside the vehicle / weather entities,
radio, environment, utility, destructors, the AX-only getters), helper A `port3_veh.rs` (`CAEVehicleAudioEntity`),
helper B `port3_ent.rs` (collision, ped, speech, weather, water cannon, twin loop). Per-function verdicts:
`docs/arch/audio-port-3-verdicts.tsv` (198 rows: address, name, file, state, evidence; `L2` only with 0 mismatches and
0 undeclared writes on every recording that called the function, from L2 runs or, where marked `split`, split runs).

Totals: **99 hooks verified or registered this pass** (68 `L2`, 31 `implemented-needs-L2`: no call in the recordings),
7 `ported-unhookable`, 3 `protection`, 89 `not-ported` (each with its reason: out of phase-A scope, entries shorter
than the patch, placeholder catalogue names, deferred large bodies). The 99 include the nine AX-only getters and
`ReleasePedConversation` (pass-2 ports now hooked) and the six pass-2 `Play*` ports re-verified with `peek` locals.

## Infrastructure changes (this branch)

- Merged PORT-AUDIO-2 (8e74eb0) and PORT-HUD-3's branch head (a220d7f: `sa_core::abi::Ax` return kind, `peek`
  hook attribute). The two pass-2 `orig!` declarations returning X87 (`vector_magnitude_sqr`,
  `GetSpeechContextVolumeOffset`) are `_st0 -> f64` wrappers (X87 is 16 bytes since the X87 branch).
- `SA_L2_EXTRA` accepts `@FILE` like `SA_L2` / `SA_HOOK_MODE` (sa-hook `list_value`; port-l2.sh passes long values
  through `e2e_list_env`).
- `SA_L2_STACKFILL` now applies to L2 runs too, with `SA_L2_STACKFILL_VALUE` (port-l2.sh `L2_STACKFILL`,
  `L2_STACKFILL_VALUE`): an L2 run calls the original and then the Rust port from sa-hook's invoker, so the stack
  below the entry ESP holds harness residue that differs between the two calls; a fill makes both calls see the same
  bytes. The PORT-AUDIO-3 runs use `0x2000` bytes of `0xA5A5A5A5`: a port that zeroed a local the original leaves
  uninitialised still fails (a zero fill would hide it).

## Decisions from the main session, outcome

- **(a) Stack `CAESound` locals.** The six pass-2 ports that build a stack sound (`PlayDoorSound`, `PlayFireSounds`,
  `PlayWaterSounds`, `CAEPedSpeechAudioEntity::PlayLoadedSound`, police-scanner `PlayLoadedDialogue`, script
  `PlayLoadedMissionAudio`) no longer zero it: each hook has `peek = 0x80` and builds the local from the original's
  stack residue (`port3_lead::stack_local`, the local is at entry ESP - 0x80 in all six, from the prologues). Helper A
  used the same for every vehicle port whose local leaves fields unwritten on some path (`RequestNewPlayerCarEngineSound`,
  `PlayAircraftSound`, `ProcessVehicle`'s cVehicleParams, `ProcessPlayerTrainBrakes`, `ProcessVehicleSirenAlarmHorn`).
  `ReportCollision`: the pass-2 mismatches were **not** residue: the pass-2 port passed `speed` where
  `UpdateLoopingCollisionSound` takes the position pointer (0x4DBCCE pushes `[esp+0x28]` = posn). Helper B re-ported
  the collision functions from the bytes; the bytes of their stack sounds that the original never writes (+0x55,
  +0x5A, +0x6A, +0x72) are not copied by `CAESound::operator=` and not read by the destructor, so they are dead and
  the port may zero them. `ReportCollision` is L2 on all five recordings (300 cases each). The buggy pass-2 copy
  is removed from `port2_b.rs`. No normaliser was added.
- **(b) `ReleasePedConversation`.** Split runs (`SPLIT_PER_FRAME=10`): PASS on grove (4/4), air-water (7/7),
  police-gang (1/1); split-report 0 undeclared. The one L2 case per recording is an L2 double-run artefact:
  `StopCurrentSpeech` (original in both calls) changes heap state the snapshot does not restore (a sound's physical
  reference: the Rust call leaves the sound's entity pointer set at 0xB63968 / 0xB63AC4 / 0xB63D7C because the original
  call already unlinked the reference node). Declaring the slot owners' entity reference heads did not change it. The
  port is hooked; state `L2` from the split runs. `CAEAudioHardware::RescaleChannelVolumes` behaves the same (one
  L2-mode case at frame 276, the channel-count global 0xB5F898 decremented by the original's channel query; split
  runs PASS 300/300 on grove and driving).
- **(c) AX-only results.** PORT-HUD-3 landed `AbiRet::Ax` (bebbe76) and it was merged; the nine getters are hooked
  through `Ax` wrappers in `port3_lead.rs` after checking every caller reads AX only (direct callers and the speech /
  pedless vtable slots 0x10 / 0x18, used only by `RequestPedConversation`; `GetVirtualChannelForPhysicalChannel` has
  no reference). L2 and split PASS where called (GetVoice, GetAllocatedVoice, CanWePlayGlobalSpeechContext,
  GetVehicleTypeForAudio, GetNumAvailableChannels); the other four have no call in the recordings.

## Lead ports (`port3_lead.rs`)

- fyl2x users: `UpdateAmbienceTrackAndVolume` (sandstorm `lg2 * log2(S) * 20 - 6`; the box-zone `zonePos` local the
  bytes never write is read from the residue with `peek`), `RescaleChannelVolumes` (CRT `_CIpow` through
  `orig::x87_binary`, `AudioLog10` = `utility::audio_log10`), `UpdateRadioVolumes` (gta-reversed skips the duck volume
  when `s_bForceAudible` is set; the bytes jump straight to it), plus `CheckForTrackConcatenation`, radio `Save` / `Load`,
  `GetPositionRelativeToCamera` (writes `d.z` into its `pos` argument slot: reproduced with `footprint`),
  `GetBankAndSoundFromScriptSlotAudioEvent` (same, slot 0), the speech constructor (`preserve = "edx"`: its pedless
  caller keeps `this` in EDX), `~CAESound`, five destructors / forwarders.
- `QueueUpTracksForStation` is ported but not hooked: its entry is a 5-byte `jmp` into the in-text detour 0x4020D0,
  which sa-hook classifies as a `.HOODLUM` stub (table test). `CAESoundManager::RequestNewSound` cannot be hooked (its
  loop branches back to entry + 4).
- Deferred (next pass): `CAESoundManager::Service` (856 instructions), `CAEFrontendAudioEntity::AddAudioEvent` (1,712,
  102 cases; ADR-002 partial port once the recordings' event ids are listed), `ProcessMissionAudioEvent` (2,382; no
  recording plays mission audio).

## Helper reports (summarised; details in the verdicts file)

- Helper A: 45 vehicle hooks (35 L2 on driving / car-ai / air-water / police-gang, 10 no calls), `StaticService`
  unhookable (1 byte), `AddAudioEvent-0` and `StaticGetPlayerVehicleAudioSettingsForRadio` protection; not ported:
  `ProcessPlayerVehicleEngine`, the player / dummy prop, player heli and sea plane bodies. Exe-vs-gta-reversed:
  ProcessSpecialVehicle's AL is not a callee input (the pass-2 note was wrong); RoadNoise frequency
  `max(dist/72, 1)*0.5+0.75`; DummyStateTransition engine types; horn fade path and early return; hovercraft pedals
  from its own argument slot; no `+0xA9` reset in train / boat re-requests; DummyJet has no 0.2 throttle floor;
  DummyHeli smoothing 0.03 (double) below 0.63.
- Helper B: 23 entity hooks (19 L2 on weapon-combat / grove / driving / police-gang / air-water, 4 no calls); 7 ported
  but unhooked (two destructors whose entry bytes protection reads, a placeholder-named row, short entries); not
  ported: `PlayGunSounds`, `ServiceAmbientGunFire`, `UpdateJetPack` (FSIN), weather `UpdateParameters` /
  `AddAudioEvent`, `HandlePedHit`, three AX-result speech functions, data-stream / decoder internals. Extras that
  matter: `CAEPedSpeechAudioEntity::Initialise@0xB478F8:0x49DC:D` (ped model-info store, `IncrementVoice`) and the
  `gMatrixList` link array for `CAEGlobalWeaponAudioEntity::Constructor`. Exe findings: HandlePedJacked falls through
  into a third slot-0x29 sound; PlayOneShotCollisionSound's two passes; looping collision speed sources and caps;
  PlayTwinLoopSound's second sound at -100 dB with flags `| 0x24`.

## Verification

Lane 7 (flock'd), `L2_PER_FRAME=10 L2_MAX=300`, `L2_STACKFILL=0x2000 L2_STACKFILL_VALUE=0xA5A5A5A5`, the PORT-AUDIO-2
singleton `D` regions through `L2_EXTRA`; per-hook regions in `tools/e2e/l2-extras.txt` section PORT-AUDIO-3; hook
list `tools/e2e/lists/audio3-all.txt` (99). Runs: `port-l2-a3lead{2,3}-*` (grove, driving, air-water, menus-hud,
police-gang, weapon-combat, car-ai), `a3ax-*` and `a3leadsp-*` (split), `a3veh-{3,5}-*`, `a3ent-{5,6}-*`. Every run:
(b) l2 == plain and (c) rust == plain IDENTICAL on all frames, 17 subsystems.
Tests: `cargo test -p sa-audio` 10 pass; `cargo test --release -p sa-hook --lib` 51 pass; clippy `-D warnings` sa-audio
host (all targets) and i686 (`--features in-process`) clean; `cargo fmt -p sa-audio --check` clean;
`tools/gen/audio_patterns.py --check` passes (165 candidates, 77 rest).

**verify-all: not passed on this branch, cause outside AUDIO.** `verify-all.sh a3final1 7` (snapshot final1 = HEAD): the
all-ports Rust run of the grove walk crashes at boot (`page fault ... at address 50000000`, 0 frames; the plain run
passes); stopped after the first recording. All-ports Rust runs (inf4) with groups of hooks forced to the original
(`target/a3/bisect*.sh`) bisect it to the 78 `CVehicleModelInfo` hooks of `sa-stream/src/modelinfo/vehicle.rs`:
every AUDIO hook on the original still crashes, every sa-stream hook (or just `modelinfo/vehicle.rs`) on the
original reaches all 5,200 frames. `vehicle.rs` is identical on master, whose verify-all (`main-g3`) passes; this
branch's base is `c1a27ca WIP RENDER-2` (not on master, which has the final RENDER-2 8b77953) plus PORT-HUD-3's
work-in-progress head, so the crash comes from code the vehicle model info ports call on this base, not from the
AUDIO-3 ports. The integration into master should rerun verify-all there.

### Open items / decisions needed

- `unverified = "audio3-<n>"` (route / hazards) on the 31 hooks without L2 evidence: the attribute exists only on
  master's sa-macros (not on this branch, which would not compile with it); add it when merging (list: the
  `implemented-needs-L2` rows of the verdicts file).
- Deferred bodies: `CAESoundManager::Service`, `CAEFrontendAudioEntity::AddAudioEvent`, `ProcessMissionAudioEvent`,
  `ProcessPlayerVehicleEngine`, player/dummy prop, player heli / sea plane, `PlayGunSounds`, `ServiceAmbientGunFire`,
  `UpdateJetPack`, weather `UpdateParameters` / `AddAudioEvent`, `HandlePedHit`.
- Recordings with missions, RC vehicles, jets, sea planes, combines, golf carts, a player-driven train, stealth kills
  and chainsaws for the `implemented-needs-L2` rows.
