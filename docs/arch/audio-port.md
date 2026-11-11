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
| 0x4F6150, 0x4F51F0, 0x4F53D0, 0x4FA0C0, 0x4F8DF0 | `GetFlyingMetalVolume`, `GetVolumeForDummyIdle`, `GetVolumeForDummyRev`, `ProcessBoatMovingOverWater`, `ProcessReverseGear` | the original multiplies `AudioLog10`'s full-precision ST0 (`fyl2x`) directly; the port rounds it to f32 first. While hooked: 1-ulp ST0 mismatches (`GetVolumeForDummyIdle` 30/1001, `...Rev` 4/1001 on grove, `port-l2-ab-g`). Unhooked; needs an `X87` `fyl2x` model in sa-math (also blocks `AudioLog10` 0x4D9E50, `CAEAudioUtility::StaticInitialise`, `CAERadioTrackManager::UpdateRadioVolumes`) |

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

- `X87` `fyl2x` (log) model in sa-math: unblocks `AudioLog10` and the five unhooked vehicle functions.
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
