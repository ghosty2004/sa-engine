# PORT-CAM: subsystem CAM, first pass (crate `sa-cam`)

First pass over the catalogue subsystem CAM (218 rows: `CCamera` ~120, `CCam` ~60, `CIdleCam` 16, `CCurves` 7,
`CHandShaker` 5, `CMBlur` 2, static initialisers / destructors). This pass ports the leaf and near-leaf
functions, chosen from `catalog/samp-priority.csv` (CAM rows executed under SA:MP) first, then by corpus
coverage, smallest first, plus a few per-frame workers (`CalculateFrustumPlanes`, `CalculateGroundHeight`,
`ConeCastCollisionResolve`, `CopyCameraMatrixToRWCam`, `UpdateSoundDistances`, `CCamera::Init`, the constructor,
`CalculateMirroredMatrix`). Follows ADR-001 and the
PORT-POP pattern. The main session sets `catalog/state.csv`; the per-function verdicts are in
`docs/arch/cam-port-verdicts.tsv` and the table at the end.

Ported: **133 functions**: 130 `#[hook]` registrations + 3 ported but not hooked (`CCamera::IsSphereVisible-`
and `CCamera::Fade`: protection reads their entry bytes; `CCamera::CalculateDerivedValues`: uninitialised
stack data reaches the output, see below). Not touched: the camera mode processors (`CCam::Process_*`,
`CCam::Process`, `CCamera::Process`, `CamControl`, `TryToStartNewCamMode`, `CameraColDetAndReact`,
`ImproveNearClip`, `AvoidTheGeometry`, `StartTransition`, `UpdateTargetEntity`, ...),
`CHandShaker::Process`, the rest of `CIdleCam`, `CCurves::CalcCurvePoint`, the cut-scene spline loaders.

## Crate

`crates/sa-cam` (PLAN §2 names it). Same rules as sa-world / sa-pop: `#![no_std]`, `#![deny(unsafe_code)]`,
game state through `globals!`/`GPtr`, every other catalogued function through `orig!` (so its toggle governs
calls from Rust), every port registered with `#[hook]` and listed in its module's `HOOKS`
(`sa_cam::hooks::ALL`, appended by sa-hook after `sa_pop::hooks::ALL`). Builds for host, i686
(`--features in-process`) and wasm32. Depends on sa-core, sa-math, sa-stream, sa-world (the `Fpu` machine,
field helpers, `FindPlayer*`, collision flags), sa-pop (`GetATanOfXY` declaration).

| file | contents |
|---|---|
| `common.rs` | sa-world's field helpers and `Fpu` re-exported, `TheCamera` address, `cam_of` / `active_cam` (CCam stride 0x238 at +0x174), `k(bits)` constants, `jp` / `jne` compare idioms |
| `camera.rs` | `CCamera` getters / setters / flags (35 hooks) |
| `camctl.rs` | `CCamera` control hand-over (`TakeControl*`, `Restore*`), cut-scene helpers, mirror set-up, sphere visibility, frustum planes, ground height, cone-cast resolve, sound distances, RW matrix copy, `Init`, the constructor, `TakeControlAttachToEntity`, `CalculateMirroredMatrix`, `CMBlur::MotionBlurOpen` (33 hooks + 2 unhooked) |
| `camfx.rs` | `CCamera` fades, shakes, FOV / vector lerps (`Process*Linear`, `ProcessFOVLerp`), ducking, collision vars, vehicle tweaks (30 hooks + `Fade` unhooked) |
| `cam.rs` | `CCam` leaves (`WrapAngle`, `ClipBeta`, `WellBufferMe`, `GetVectorsReadyForRW`, `Init`, `KeepTrackOfTheSpeed`, ...) (11) |
| `handshaker.rs` / `idlecam.rs` / `curves.rs` | `CHandShaker` (4), `CIdleCam` (7), `CCurves` (4) |
| `statics.rs` | the 6 CAM static initialisers (`__init_*`) |

Shared-crate changes: sa-hook `table.rs` (sa-cam's hooks appended), `callers.rs` test and sa-oracle
`hook_preserve` chain sa-cam's registrations; sa-oracle `specs_cam.rs` (6 L1 specs, registered in `specs.rs`).
Harness: `tools/e2e/port-l2.sh` now deletes the previous run's `wine.log` before a run starts (a Wine fault
during the previous run's process exit was read as a crash of the next run at once, which then never got
killed and hung on winedbg; seen twice on lane 1). `tools/e2e/l2-extras.txt` section `# PORT-CAM`; lists
`tools/e2e/lists/cam-all.txt` (130), `cam-l2.txt`, `cam-split.txt`, `cam-split2.txt`, `cam-split3.txt`,
`cam-l2b.txt`, `cam-l2c.txt`, `cam-l2d.txt`.

## Conventions and findings worth knowing

- Ported from the 1.0 US bytes (objdump of the exe); gta-reversed and RE-SPEC-2 (`docs/re/specs/CAM/*.md`) as
  reading aids. Every `expect` string is whole instructions from the exe. CAM has no `.HOODLUM` stubs.
- x87: the literal `Fpu` machine wherever the stack use or the operand form matters (memory vs register
  operands differ on NaN inputs); `fstp dword` rounds to f32, values kept on the stack do not. FSIN / FCOS /
  FPATAN use sa-math's f64 model (documented approximation, stored as f32 afterwards).
  `CCamera::Find3rdPersonQuickAimPitch` and `Find3rdPersonCamTargetVector` use FPTAN, which sa-math does not
  model: not ported.
- `GetATanOfXY` results (raw FPATAN, 64-bit significand in ST0) reach Rust through `orig!` as an f64 and are
  stored as f32: a 64 -> 53 -> 24 double rounding the original does not do; it can differ only when the 53-bit
  value lies on an f32 tie (`SetCameraDirectly*`, `StoreValuesDuringInterPol`).
- Protection chunks: several entries jump into `.text` protection fragments that compute a value and jump
  back (`SetCamPositionForFixedMode` 0x401D1C: `eax = this + 0x86C`; `TakeControl` 0x408005: the
  `+0xB0 == 1` compare; `CCam::Init` 0x40808C: `ebx = 1`; `InitialiseScriptableComponents` 0x4010F8:
  `edx = -1.0f`); the ports inline them. `InitialiseScriptableComponents`' entry is `jmp 0x4010F8`, so its
  `expect` covers the following `xor eax,eax` too (otherwise the table takes it for a `.HOODLUM` stub).
- Unhookable (protection reads the entry bytes as data; sa-hook `every_hook_installs_on_the_exe` rejects
  them): `CCamera::IsSphereVisible-` 0x420C40 (`push 0x420C46` at 0x14664CF) and `CCamera::Fade` 0x50AC20
  (`sbb edx, [0x50AC20]` at 0x1566C27). Both are ported; their callers reach them through `orig!`.
- `CCamera::CalculateDerivedValues` 0x5150E0 is ported but **not hooked**: `CMatrix::Invert` (by value,
  0x59BDD0) builds its result in a stack local whose +0xC word (the RwMatrix `flags`, which
  `RwV3dTransformPoints` tests for the identity flag) it never writes, so `m_mMatInverse.flags` is whatever
  the caller's stack held (pause-restart L2: 1000/1000 cases differ only there, original 4). Reproducing it
  needs a stack-faithful call or a normaliser decision.
- Preserved registers (the callers test): `GetFadingDirection` (EDX), `GetScreenRect` (ECX),
  `InitCameraVehicleTweaks` (ECX), `ProcessVectorTrackLinear-wo-ratio` / `ProcessVectorMoveLinear-wo-ratio`
  (EDX: `ProcessScriptedCommands` keeps `this` in EDX), `CCam::WrapAngle` (EDX).
- gta-reversed differences kept from the bytes: `GetCutSceneFinishTime` reads the other cam without copying
  it (gr assigns `cam = m_aCams[...]`, a copy over cam 0); `TakeControl` with mode 0 derives the mode from
  the **current** target's type (+0x958), not the new one; `UpdateSoundDistances`' blend factor is
  `(frame % 12 + 1) / 6`, reaching 2.0; `CIdleCam::IdleCamGeneralProcess` zeroes the ticker through the
  absolute 0xB6FE34; `CCam::GetWeaponFirstPersonOn` leaves pointer bits in EAX (callers test AL).
- `camctl.rs` notes: `CCamera::Init` zeroes `this+0x18..+0xD78` with the `rep stos` count loaded in the
  protection chunk 0x406E9A, calls `InitialiseScriptableComponents` on the absolute `TheCamera`, and has no
  trace of gr's FIX_BUGS `m_fMouseAccelVertical = horizontal * 0.6` (the bytes store 0.0015 when the menu byte
  0xBA67A5 is 0; the CAM gr-deviations row, kept original). `Restore` / `RestoreWithJumpCut` share the
  target hand-back (player vehicle in mode 0x12, else the focused ped in mode 4; ped states 0x3B / 0x39 /
  0x35 force 0x12, 0x3D forces 4 with the ped); `RestoreWithJumpCut` adds the cooperative-mode targets.
  `CopyCameraMatrixToRWCam` snaps each RwFrame row back to the previous frame's when it moved by less than
  `0.00001^2` (position: `k^2 > |d|^2` via `test ah,0x41`; axes: `|d|^2 < (f32)k^2` via `test ah,5`).
  `CalculateGroundHeight` probes only after a > 20.0 move on some axis; `UpdateSoundDistances` probes every
  12th frame. `IsSphereVisible-` copies the centre's x as a dword and y/z through `fld`/`fstp` (a signalling
  NaN is quieted).
- `camfx.rs` notes: `VectorTrack/MoveRunning` load the end time as uint32 (`+2^32` when negative);
  `SetCamCollisionVarDataSet` compares the full int against the u8 `gCurCamColVars` and stores the low byte;
  `SetColVarsAimWeapon` / `SetColVarsVehicle` tail-jump into it (called through `orig!`), `SetColVarsPed`
  inlines it; the `-with-ratio` lerps keep the ease weight `(sin((270 - r*180) * 0.0174533) + 1) * 0.5` on the
  stack and multiply `(e - s) * w` for vectors but `w * (e - s)` for the FOV (operand order only matters
  for NaN); `CamShake` clamps the distance to 100 before 0 and computes the scale with the two f64 constants
  0x85A308 / 0x85A310; `ProcessFade`'s "instant" test is `time == 0.0` (`test ah,0x44`, NaN takes the
  gradual path), `FadeValue` = AL of `_ftol`; `HandleCameraMotionForDucking` calls `GetTaskDuck` twice (first
  result unused) and its goal is the zeroed ped argument slot when the ped is not ducking;
  `HandleCameraMotionForDuckingDuringAim` computes a speed it never uses (skipped, no side effect).
  `SetNearClipBasedOnPedCollision` passes the clip value to `RwCameraSetNearClipPlane` as an f32 argument; a
  signalling NaN could be quieted by the i686 Rust ABI there (not observed).
- `cam.rs` / `handshaker.rs` / `idlecam.rs` / `curves.rs` notes: `CCam::WrapAngle` loops on the stack with
  one final `fstp` (NaN untouched; +-inf or huge values never leave the loop, as in the original);
  `CCam::WellBufferMe`'s angle path wraps the f32-stored delta, the other path the unrounded ST0;
  `CCam::GetVectorsReadyForRW` computes `right = Normalise(front x up)`, `up = right x front` (Normalise /
  CrossProduct still original); `CCam::KeepTrackOfTheSpeed`'s first call keeps beta / FOV on the stack without
  storing the statics; `CHandShaker::Reset` consumes three `rand()` (sites 0x50D8A0 / 0x50D8BE / 0x50D8DC),
  the first two products stored as f32; `SetDefaults` tail-jumps to `Reset`;
  `CIdleCam::GetLookAtPositionOnTarget` reads the `.data` f32 0x8CCC8C as a variable (`g + z`);
  `CCurves::CalcSpeedScaleFactor`'s fallback adds two +0.0 slot values, kept literally. `__init_gIdleCam`
  calls `CIdleCam::Init`, not the constructor.
- Static blocks owned here: `CalculateGroundHeight`'s 0xB70028..0xB70040, `CopyCameraMatrixToRWCam`'s previous
  rows 0xB6FF90..0xB6FFC0, gIdleCam 0xB6FDA0, gHandShaker 0xB6ECA0, the DW ciney-cam / KeepTrackOfTheSpeed
  statics (cam.rs).

## Verification (commands, results)

All game runs on lane 1 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=1 L2_PER_FRAME=1000`, under
`systemd-run --user --scope -p MemoryMax=8G`), outputs in the worktree's `target/e2e/lane-1/port-l2-cam-*`.
Recordings: grove walk (`grove`, 5200 frames), `pause-restart` (`pr`, 3700), `world-cheats` (`cheats`, 3276),
`c-bug` (`cbug`, 3220), `weapon-combat` (`wc`, 3500), `anim-input` (`anim`, 3300). Lists:
`cam-l2.txt` (plain l2), `cam-split.txt` / `cam-split2.txt` / `cam-split3.txt` (split mode: world scans with
entity scan codes, `rand()`, `free`, reference hand-overs whose heap lists the plain restore does not cover,
the RwFrame, constructors and static initialisers), `cam-l2b.txt` / `cam-l2c.txt` / `cam-l2d.txt` (reruns
after declaring regions, and the later ports); `cam-all.txt` = all 130.

- **L1** (`SA_WINEPREFIX=/nonexistent sa-oracle l1 <key> --cases 1000000 --no-fixtures --threads 4`,
  `crates/sa-oracle/src/specs_cam.rs`): `CameraCalculateMirroredMatrix` 1,060,000 cases, 0 mismatches
  (160,586 signalling-NaN cases unverified, as COLL/PHYS/POP); `CamWrapAngle` 1,000,058, 0 mismatches (+-inf
  and |a| >= 1e4 mapped to finite inputs: original and port both loop forever there);
  `CurvesDistForLineToCrossOtherLine`, `CurvesCalcSpeedVariationInBend`, `CurvesCalcSpeedScaleFactor`
  1,060,000 each, 0 mismatches; `CurvesCalcCorrectedDist` routed to L2 (`FSIN_REASON`: Unicorn's FCOS
  returns an infinite operand unchanged, the CPU and sa-math give the indefinite; 10,224 such cases).
  `cargo test --release -p sa-oracle --test l1`: 16 pass.
- Tests: `cargo test -p sa-cam` (3 pass: list complete / unique, globals declared once across crates incl.
  sa-pop's, initial values match the exe), `cargo test --release -p sa-hook --lib` 45 pass
  (`every_hook_installs_on_the_exe`, `matches_catalogue_and_exe`,
  `callers_reading_ecx_edx_after_a_hook_are_declared`, the keycells tests, the INF-20 `data_reads` tests);
  clippy `-D warnings`: sa-cam host (all targets), i686 (`--features in-process`), sa-hook i686, sa-oracle
  host (all targets); `cargo build -p sa-cam --target wasm32-unknown-unknown`; `cargo check --workspace`.
- **L2 extra regions** (`tools/e2e/l2-extras.txt`, section `# PORT-CAM`), each with the run that found it:
  `GetScreenRect` / `GetArrPosForVehicleType` / `WrapAngle` / `CalcCorrectedDist` /
  `HandleCameraMotionForDucking*` / `ConeCastCollisionResolve` out-parameters; `ProcessMusicFade`'s audio
  engine state 0xB5FCD0; `CalculateGroundHeight`'s static block and the vertical-line scan state;
  `CReferences::aRefs` for the target hand-overs; gIdleCam / CPad 0 +0x120 for `CCam::Init` /
  constructors; the RwFrame of `CopyCameraMatrixToRWCam`; the objects and the atexit slot of the static
  initialisers.

### Run results (lane 1, final builds)

| run | tags | result |
|---|---|---|
| plain l2 + rust, 113 hooks (first list) | `cam-{g,p,c,b}-l2` | (b) l2 == plain, (c) rust == plain IDENTICAL on all 17 subsystems, every recording. Mismatches only in `Restore` / `RestoreWithJumpCut` (the entity's reference list is heap state the plain restore leaves at the original's result: moved to split mode, `cam-split2`), undeclared writes in 6 functions (declared, rerun `cam-*-l2b`: PASS) |
| plain l2 + rust, 110 hooks (final list) | `cam-{w,a}-l2` | (b), (c) IDENTICAL; `GetArrPosForVehicleType` / `WrapAngle` out-parameters undeclared on anim-input, declared, rerun `cam-{a,w}-l2d`: PASS |
| plain l2 reruns | `cam-{g,p,c,b}-l2b`, `cam-{g,p,c,b}-l2c`, `cam-{a,w}-l2d` | (b), (c) IDENTICAL; every hook PASS or NOCALLS |
| split + rust (11 / 8 / 2 hooks) | `cam-*-split`, `cam-*-split2`, `cam-*-split3` on all six recordings | splito == plain, splitr == plain, rust == plain; split-report every hook PASS / PASS-FP / NOCASES, 0 FAIL / DIVERGED. The first split runs reported undeclared writes for the static initialisers, the constructor and `ConeCastCollisionResolve`; after declaring them (l2-extras) `cam-*-split` and `cam-*-split2` were rerun on all six recordings: 0 undeclared |
| all engine ports on Rust (`ports`, 1396 hooks incl. the 130 CAM ones) | `cam-{g,p,c,b}-ports` (first build), `cam-{g,p,c,b}-ports2`, `cam-{g,p,c,b,w,a}-ports3`, `cam-g-ports4` (formatted final build) | IDENTICAL over every frame of every recording, 17 subsystems, no crash / panic / install failure |

L3: every CAM hook on Rust together (and all engine ports together) is frame-identical on grove, pause-restart,
cheats, c-bug, weapon-combat and anim-input. Per-hook hook-off runs were not made, so no row claims L3.

Verdict counts (130 hooks + 3 not hooked): **L1 5**, **L2 81**, **implemented-needs-L2 44**, **ported-unhookable 2**, **ported-not-hooked 1**. No mismatch in any final run.


## Divergence notes (ledger candidates)

- `CCamera::CalculateDerivedValues` (not hooked): the inverse matrix's RwMatrix flags word is uninitialised
  stack data in the original (see above); a 1:1 port needs either a stack-faithful `Invert` call or a
  reviewed normaliser rule plus a decision on what Rust writes there (the flags word is read by
  `RwV3dTransformPoints` in `IsSphereVisible` for its identity fast path).
- `GetATanOfXY` double rounding (64 -> 53 -> 24 bits) in `SetCameraDirectly*` and `StoreValuesDuringInterPol`;
  FSIN / FCOS / FPATAN modelled in f64 (`CalculateFrustumPlanes`, `CalculateDerivedValues`, the `-with-ratio`
  lerps, `CalcCorrectedDist`); no difference seen in any run.
- `CCamera::ConeCastCollisionResolve`: `CameraConeCastVsWorldCollision` stores pointers to the two local
  spheres in globals (0x9655C8 / 0x9655CC), so their values are stack addresses that differ between the
  original and the Rust frame; nothing reads them after the call in the corpus (split PASS, L3 identical).
- `SetNearClipBasedOnPedCollision`: a signalling-NaN clip value passed as an f32 argument may be quieted by
  the Rust i686 ABI.

## Open items

- 44 hooks without a call in any of the six recordings (`implemented-needs-L2`): script-driven camera
  commands (`TakeControl*`, `SetZoomValue*Script`, `SetParametersForScriptInterpolation`, `Vector*Linear`,
  `LerpFOV`, shakes), cut-scene / mirror / cooperative-mode paths, `CIdleCam` (idle camera after 90 s), some
  `CCam` leaves. Needs recordings that reach them (mission scripts, idle, mirrors, vehicles with
  `CameraVehicleModeSpecialCases`).
- Not ported (FPTAN, not modelled by sa-math): `Find3rdPersonQuickAimPitch`, `Find3rdPersonCamTargetVector`.
  `DealWithMirrorBeforeConstructRenderList` has the same uninitialised-pad issue as `CalculateDerivedValues`.
- The large processors (`CCamera::Process`, `CamControl`, `TryToStartNewCamMode`, `CCam::Process_*`,
  `CameraColDetAndReact`, `ImproveNearClip`, `AvoidTheGeometry`, `StartTransition`, `UpdateTargetEntity`),
  `CHandShaker::Process`, the remaining `CIdleCam`, `CCurves::CalcCurvePoint`, the cut-scene loaders.
- Hook-off (per-hook L3) runs.

## Functions

| address | name | state | evidence |
|---|---|---|---|
| 0x50AC00 | CCamera::Enable1rstPersonCamCntrlsScript | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50AC10 | CCamera::Enable1rstPersonWeaponsCamera | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50AE50 | CCamera::GetGameCamPosition | L2 | L2 grove:623/623 PASS (0 mismatches, 0 undeclared); L2 pr:384/384 PASS (0 mismatches, 0 undeclared); L2 cheats:383/383 PASS (0 mismatches, 0 undeclared); L2 cbug:376/376 PASS (0 mismatches, 0 undeclared); L2 wc:411/411 PASS (0 mismatches, 0 undeclared); L2 anim:386/386 PASS (0 mismatches, 0 undeclared) |
| 0x50AF80 | CCamera::GetPositionAlongSpline | L2 | L2 grove:26/26 PASS (0 mismatches, 0 undeclared); L2 pr:150/150 PASS (0 mismatches, 0 undeclared); L2 cheats:26/26 PASS (0 mismatches, 0 undeclared); L2 cbug:26/26 PASS (0 mismatches, 0 undeclared); L2 wc:26/26 PASS (0 mismatches, 0 undeclared); L2 anim:26/26 PASS (0 mismatches, 0 undeclared) |
| 0x50AB40 | CCamera::DontProcessObbeCinemaCamera | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50C140 | CCamera::SetWideScreenOn | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:2/2 PASS (0 mismatches, 0 undeclared); L2 cheats:2/2 PASS (0 mismatches, 0 undeclared); L2 cbug:1/1 PASS (0 mismatches, 0 undeclared); L2 wc:2/2 PASS (0 mismatches, 0 undeclared); L2 anim:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x50C150 | CCamera::SetWideScreenOff | L2 | L2 grove:4/4 PASS (0 mismatches, 0 undeclared); L2 pr:7/7 PASS (0 mismatches, 0 undeclared); L2 cheats:5/5 PASS (0 mismatches, 0 undeclared); L2 cbug:4/4 PASS (0 mismatches, 0 undeclared); L2 wc:5/5 PASS (0 mismatches, 0 undeclared); L2 anim:5/5 PASS (0 mismatches, 0 undeclared) |
| 0x515BC0 | CCamera::StartTransitionWhenNotFinishedInter | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50C280 | CCamera::AllowShootingWith2PlayersInCar | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50BF80 | CCamera::SetMotionBlurAlpha | L2 | L2 grove:25/25 PASS (0 mismatches, 0 undeclared); L2 pr:148/148 PASS (0 mismatches, 0 undeclared); L2 cheats:625/625 PASS (0 mismatches, 0 undeclared); L2 cbug:25/25 PASS (0 mismatches, 0 undeclared); L2 wc:625/625 PASS (0 mismatches, 0 undeclared); L2 anim:625/625 PASS (0 mismatches, 0 undeclared) |
| 0x50C260 | CCamera::StartCooperativeCamMode | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50C270 | CCamera::StopCooperativeCamMode | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50BF90 | CCamera::SetNearClipScript | L2 | L2 grove:2/2 PASS (0 mismatches, 0 undeclared); L2 pr:4/4 PASS (0 mismatches, 0 undeclared); L2 cheats:2/2 PASS (0 mismatches, 0 undeclared); L2 cbug:2/2 PASS (0 mismatches, 0 undeclared); L2 wc:2/2 PASS (0 mismatches, 0 undeclared); L2 anim:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x50AD20 | CCamera::FindCamFOV | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50CD80 | CCamera::CameraPedModeSpecialCases | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50ADF0 | CCamera::GetFadingDirection | L2 | L2 grove:37/37 PASS (0 mismatches, 0 undeclared); L2 pr:76/76 PASS (0 mismatches, 0 undeclared); L2 cheats:112/112 PASS (0 mismatches, 0 undeclared); L2 cbug:112/112 PASS (0 mismatches, 0 undeclared); L2 wc:112/112 PASS (0 mismatches, 0 undeclared); L2 anim:57/57 PASS (0 mismatches, 0 undeclared) |
| 0x50BD20 | CCamera::SetCamCutSceneOffSet | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:2/2 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared); L2 cbug:1/1 PASS (0 mismatches, 0 undeclared); L2 wc:1/1 PASS (0 mismatches, 0 undeclared); L2 anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x50CB10 | CCamera::UpdateAimingCoors | L2 | L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:316/316 PASS (0 mismatches, 0 undeclared) |
| 0x50AB10 | CCamera::ClearPlayerWeaponMode | L2 | L2 grove:2/2 PASS (0 mismatches, 0 undeclared); L2 pr:5/5 PASS (0 mismatches, 0 undeclared); L2 cheats:162/162 PASS (0 mismatches, 0 undeclared); L2 cbug:208/208 PASS (0 mismatches, 0 undeclared); L2 wc:175/175 PASS (0 mismatches, 0 undeclared); L2 anim:151/151 PASS (0 mismatches, 0 undeclared) |
| 0x50BFB0 | CCamera::SetNewPlayerWeaponMode | L2 | L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:316/316 PASS (0 mismatches, 0 undeclared) |
| 0x50AE60 | CCamera::GetLookingLRBFirstPerson | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x50AED0 | CCamera::GetLookingForwardFirstPerson | L2 | L2 anim:151/151 PASS (0 mismatches, 0 undeclared) |
| 0x50AE90 | CCamera::GetLookDirection | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x50CAE0 | CCamera::TakeControlWithSpline | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:2/2 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared); L2 cbug:1/1 PASS (0 mismatches, 0 undeclared); L2 wc:1/1 PASS (0 mismatches, 0 undeclared); L2 anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x50AE20 | CCamera::GetScreenFadeStatus | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x50CE80 | CCamera::IsExtraEntityToIgnore | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x50C030 | CCamera::SetParametersForScriptInterpolation | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50BF40 | CCamera::SetMotionBlur | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:3/3 PASS (0 mismatches, 0 undeclared); L2 cheats:2/2 PASS (0 mismatches, 0 undeclared); L2 cbug:2/2 PASS (0 mismatches, 0 undeclared); L2 wc:2/2 PASS (0 mismatches, 0 undeclared); L2 anim:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x50BFF0 | CCamera::Using1stPersonWeaponMode | L2 | L2 grove:4/4 PASS (0 mismatches, 0 undeclared); L2 pr:8/8 PASS (0 mismatches, 0 undeclared); L2 cheats:8/8 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:665/665 PASS (0 mismatches, 0 undeclared); L2 anim:50/50 PASS (0 mismatches, 0 undeclared) |
| 0x50CDA0 | CCamera::CameraPedAimModeSpecialCases | L2 | L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:356/356 PASS (0 mismatches, 0 undeclared) |
| 0x50CD30 | CCamera::CameraGenericModeSpecialCases | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x50AD90 | CCamera::GetCutSceneFinishTime | L2 | L2 grove:30/30 PASS (0 mismatches, 0 undeclared); L2 pr:158/158 PASS (0 mismatches, 0 undeclared); L2 cheats:30/30 PASS (0 mismatches, 0 undeclared); L2 cbug:30/30 PASS (0 mismatches, 0 undeclared); L2 wc:30/30 PASS (0 mismatches, 0 undeclared); L2 anim:30/30 PASS (0 mismatches, 0 undeclared) |
| 0x50BEC0 | CCamera::SetCamPositionForFixedMode | L2 | L2 cheats:1/1 PASS (0 mismatches, 0 undeclared); L2 wc:1/1 PASS (0 mismatches, 0 undeclared); L2 anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x50C160 | CCamera::SetZoomValueFollowPedScript | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50B890 | CCamera::ProcessWideScreenOn | L2 | L2 grove:29/29 PASS (0 mismatches, 0 undeclared); L2 pr:156/156 PASS (0 mismatches, 0 undeclared); L2 cheats:630/630 PASS (0 mismatches, 0 undeclared); L2 cbug:29/29 PASS (0 mismatches, 0 undeclared); L2 wc:630/630 PASS (0 mismatches, 0 undeclared); L2 anim:630/630 PASS (0 mismatches, 0 undeclared) |
| 0x50BD40 | CCamera::SetCameraDirectlyBehindForFollowPed_CamOnAString | L2 | L2 grove:2/2 PASS (0 mismatches, 0 undeclared); L2 pr:4/4 PASS (0 mismatches, 0 undeclared); L2 cheats:3/3 PASS (0 mismatches, 0 undeclared); L2 cbug:3/3 PASS (0 mismatches, 0 undeclared); L2 wc:3/3 PASS (0 mismatches, 0 undeclared); L2 anim:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x50BD70 | CCamera::SetCameraDirectlyInFrontForFollowPed_CamOnAString | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50BDA0 | CCamera::SetCameraDirectlyBehindForFollowPed_ForAPed_CamOnAString | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50BE30 | CCamera::SetCameraDirectlyInFrontForFollowPed_ForAPed_CamOnAString | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x71D710 | CMBlur::MotionBlurOpen | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50C100 | CCamera::SetRwCamera | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:2/2 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared); L2 cbug:1/1 PASS (0 mismatches, 0 undeclared); L2 wc:1/1 PASS (0 mismatches, 0 undeclared); L2 anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x50B8F0 | CCamera::RenderMotionBlur | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x420D40 | CCamera::IsSphereVisible | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x516AE0 | CCamera::ProcessScriptedCommands | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x516B00 | CCamera::GetRoughDistanceToGround | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x514950 | CCamera::FinishCutscene | L2 | L2 grove:2/2 PASS (0 mismatches, 0 undeclared); L2 pr:4/4 PASS (0 mismatches, 0 undeclared); L2 cheats:2/2 PASS (0 mismatches, 0 undeclared); L2 cbug:2/2 PASS (0 mismatches, 0 undeclared); L2 wc:2/2 PASS (0 mismatches, 0 undeclared); L2 anim:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x5B24A0 | CCamera::DeleteCutSceneCamDataMemory | L2 | split grove:2/2 PASS (0 mismatches, 0 undeclared); split pr:4/4 PASS (0 mismatches, 0 undeclared); split cheats:2/2 PASS (0 mismatches, 0 undeclared); split cbug:2/2 PASS (0 mismatches, 0 undeclared); split wc:2/2 PASS (0 mismatches, 0 undeclared); split anim:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x50C8B0 | CCamera::TakeControlNoEntity | L2 | L2 cheats:1/1 PASS (0 mismatches, 0 undeclared); L2 wc:1/1 PASS (0 mismatches, 0 undeclared); L2 anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x50AF00 | CCamera::GetArrPosForVehicleType | L2 | L2 anim:496/496 PASS (0 mismatches, 0 undeclared) |
| 0x50C070 | CCamera::SetPercentAlongCutScene | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50CDE0 | CCamera::CameraVehicleModeSpecialCases | L2 | L2 anim:248/248 PASS (0 mismatches, 0 undeclared) |
| 0x51A560 | CCamera::SetCameraUpForMirror | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x51A5A0 | CCamera::RestoreCameraAfterMirror | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50AB50 | CCamera::GetScreenRect | L2 | L2 grove:25/25 PASS (0 mismatches, 0 undeclared); L2 pr:148/148 PASS (0 mismatches, 0 undeclared); L2 cheats:625/625 PASS (0 mismatches, 0 undeclared); L2 cbug:25/25 PASS (0 mismatches, 0 undeclared); L2 wc:625/625 PASS (0 mismatches, 0 undeclared); L2 anim:625/625 PASS (0 mismatches, 0 undeclared) |
| 0x50C1B0 | CCamera::SetZoomValueCamStringScript | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50C7C0 | CCamera::TakeControl | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50C290 | CCamera::StoreValuesDuringInterPol | L2 | L2 cbug:416/416 PASS (0 mismatches, 0 undeclared); L2 wc:60/60 PASS (0 mismatches, 0 undeclared); L2 anim:106/106 PASS (0 mismatches, 0 undeclared) |
| 0x514D60 | CCamera::CalculateFrustumPlanes | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x514B80 | CCamera::CalculateGroundHeight | L2 | split grove:1000/1000 PASS (0 mismatches, 0 undeclared); split pr:1000/1000 PASS (0 mismatches, 0 undeclared); split cheats:1000/1000 PASS (0 mismatches, 0 undeclared); split cbug:1000/1000 PASS (0 mismatches, 0 undeclared); split wc:1000/1000 PASS (0 mismatches, 0 undeclared); split anim:1000/1000 PASS (0 mismatches, 0 undeclared) |
| 0x51A5D0 | CCamera::ConeCastCollisionResolve | L2 | split grove:1000/1000 PASS (0 mismatches, 0 undeclared); split pr:1000/1000 PASS (0 mismatches, 0 undeclared); split cheats:1000/1000 PASS (0 mismatches, 0 undeclared); split cbug:1000/1000 PASS (0 mismatches, 0 undeclared); split wc:1000/1000 PASS (0 mismatches, 0 undeclared); split anim:1000/1000 PASS (0 mismatches, 0 undeclared) |
| 0x50B930 | CCamera::Restore | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared); split cbug:1/1 PASS (0 mismatches, 0 undeclared); split wc:1/1 PASS (0 mismatches, 0 undeclared); split anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x50BAB0 | CCamera::RestoreWithJumpCut | L2 | split grove:2/2 PASS (0 mismatches, 0 undeclared); split pr:4/4 PASS (0 mismatches, 0 undeclared); split cheats:4/4 PASS (0 mismatches, 0 undeclared); split cbug:3/3 PASS (0 mismatches, 0 undeclared); split wc:4/4 PASS (0 mismatches, 0 undeclared); split anim:4/4 PASS (0 mismatches, 0 undeclared) |
| 0x515BD0 | CCamera::UpdateSoundDistances | L2 | split grove:1000/1000 PASS (0 mismatches, 0 undeclared); split pr:1000/1000 PASS (0 mismatches, 0 undeclared); split cheats:1000/1000 PASS (0 mismatches, 0 undeclared); split cbug:1000/1000 PASS (0 mismatches, 0 undeclared); split wc:1000/1000 PASS (0 mismatches, 0 undeclared); split anim:1000/1000 PASS (0 mismatches, 0 undeclared) |
| 0x51A450 | CCamera::Constructor | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared); split cbug:1/1 PASS (0 mismatches, 0 undeclared); split wc:1/1 PASS (0 mismatches, 0 undeclared); split anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x50AFA0 | CCamera::CopyCameraMatrixToRWCam | L2 | split grove:1000/1000 PASS (0 mismatches, 0 undeclared); split pr:1000/1000 PASS (0 mismatches, 0 undeclared); split cheats:1000/1000 PASS (0 mismatches, 0 undeclared); split cbug:1000/1000 PASS (0 mismatches, 0 undeclared); split wc:1000/1000 PASS (0 mismatches, 0 undeclared); split anim:1000/1000 PASS (0 mismatches, 0 undeclared) |
| 0x5BC520 | CCamera::Init | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:3/3 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared); split cbug:1/1 PASS (0 mismatches, 0 undeclared); split wc:1/1 PASS (0 mismatches, 0 undeclared); split anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x50C910 | CCamera::TakeControlAttachToEntity | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50B380 | CCamera::CalculateMirroredMatrix | L1 | L1 1,060,000 cases, 0 mismatches (160,586 signalling-NaN inputs unverified); no call in grove / pr / cheats / cbug / wc / anim |
| 0x474870 | CCamera::VectorTrackRunning | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x4748A0 | CCamera::VectorMoveRunning | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50BF00 | CCamera::SetFadeColour | L2 | L2 grove:2/2 PASS (0 mismatches, 0 undeclared); L2 pr:4/4 PASS (0 mismatches, 0 undeclared); L2 cheats:4/4 PASS (0 mismatches, 0 undeclared); L2 cbug:4/4 PASS (0 mismatches, 0 undeclared); L2 wc:4/4 PASS (0 mismatches, 0 undeclared); L2 anim:4/4 PASS (0 mismatches, 0 undeclared) |
| 0x50D240 | CCamera::AddShakeSimple | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50CB60 | CCamera::SetCamCollisionVarDataSet | L2 | L2 cbug:191/191 PASS (0 mismatches, 0 undeclared) |
| 0x50CBF0 | CCamera::SetColVarsAimWeapon | L2 | L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:356/356 PASS (0 mismatches, 0 undeclared) |
| 0x50CC50 | CCamera::SetColVarsPed | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x50CCA0 | CCamera::SetColVarsVehicle | L2 | L2 anim:248/248 PASS (0 mismatches, 0 undeclared) |
| 0x51A6F0 | CCamera::ProcessShake-void | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50D280 | CCamera::LerpFOV | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50CEB0 | CCamera::ConsiderPedAsDucking | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50CB90 | CCamera::SetNearClipBasedOnPedCollision | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50D160 | CCamera::VectorMoveLinear | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50D1D0 | CCamera::VectorTrackLinear | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x516440 | CCamera::ProcessVectorTrackLinear-wo-ratio | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x5164A0 | CCamera::ProcessVectorMoveLinear-wo-ratio | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x516500 | CCamera::ProcessFOVLerp-wo-ratio | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x50D350 | CCamera::ProcessVectorTrackLinear-with-ratio | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50D430 | CCamera::ProcessVectorMoveLinear-with-ratio | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50D510 | CCamera::ProcessFOVLerp-with-ratio | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50A970 | CCamera::CamShakeNoPos | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50A9F0 | CCamera::CamShake | L2 | L2 wc:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x50D2D0 | CCamera::InitialiseScriptableComponents | L2 | L2 grove:4/4 PASS (0 mismatches, 0 undeclared); L2 pr:7/7 PASS (0 mismatches, 0 undeclared); L2 cheats:4/4 PASS (0 mismatches, 0 undeclared); L2 cbug:4/4 PASS (0 mismatches, 0 undeclared); L2 wc:4/4 PASS (0 mismatches, 0 undeclared); L2 anim:4/4 PASS (0 mismatches, 0 undeclared) |
| 0x50CEF0 | CCamera::ResetDuckingSystem | L2 | L2 grove:4/4 PASS (0 mismatches, 0 undeclared); L2 pr:7/7 PASS (0 mismatches, 0 undeclared); L2 cheats:6/6 PASS (0 mismatches, 0 undeclared); L2 cbug:63/63 PASS (0 mismatches, 0 undeclared); L2 wc:12/12 PASS (0 mismatches, 0 undeclared); L2 anim:6/6 PASS (0 mismatches, 0 undeclared) |
| 0x50CFA0 | CCamera::HandleCameraMotionForDucking | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x50D090 | CCamera::HandleCameraMotionForDuckingDuringAim | L2 | L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:356/356 PASS (0 mismatches, 0 undeclared) |
| 0x50A3B0 | CCamera::InitCameraVehicleTweaks | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50A480 | CCamera::ApplyVehicleCameraTweaks | L2 | L2 anim:248/248 PASS (0 mismatches, 0 undeclared) |
| 0x50B5D0 | CCamera::ProcessFade | L2 | L2 grove:72/72 PASS (0 mismatches, 0 undeclared); L2 pr:146/146 PASS (0 mismatches, 0 undeclared); L2 cheats:199/199 PASS (0 mismatches, 0 undeclared); L2 cbug:199/199 PASS (0 mismatches, 0 undeclared); L2 wc:199/199 PASS (0 mismatches, 0 undeclared); L2 anim:199/199 PASS (0 mismatches, 0 undeclared) |
| 0x50B6D0 | CCamera::ProcessMusicFade | L2 | L2 grove:28/28 PASS (0 mismatches, 0 undeclared); L2 pr:57/57 PASS (0 mismatches, 0 undeclared); L2 cheats:101/101 PASS (0 mismatches, 0 undeclared); L2 cbug:101/101 PASS (0 mismatches, 0 undeclared); L2 wc:101/101 PASS (0 mismatches, 0 undeclared); L2 anim:101/101 PASS (0 mismatches, 0 undeclared) |
| 0x509BE0 | CCam::WrapAngle | L1 | L1 1,000,058 cases, 0 mismatches (signalling-NaN inputs unverified; +-inf and |a| >= 1e4 excluded: both loop forever); L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x509C50 | CCam::ClipBeta | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x50A850 | CCam::Using3rdPersonMouseCam | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x509DC0 | CCam::GetWeaponFirstPersonOn | L2 | L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:356/356 PASS (0 mismatches, 0 undeclared) |
| 0x50CB30 | CCam::DoCamBump | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x509AE0 | CCam::WellBufferMe | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x509CE0 | CCam::GetVectorsReadyForRW | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x517730 | CCam::Constructor | L2 | L2 grove:3/3 PASS (0 mismatches, 0 undeclared); L2 pr:3/3 PASS (0 mismatches, 0 undeclared); L2 cheats:3/3 PASS (0 mismatches, 0 undeclared); L2 cbug:3/3 PASS (0 mismatches, 0 undeclared); L2 wc:3/3 PASS (0 mismatches, 0 undeclared); L2 anim:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x50E490 | CCam::Init | L2 | L2 grove:3/3 PASS (0 mismatches, 0 undeclared); L2 pr:9/9 PASS (0 mismatches, 0 undeclared); L2 cheats:3/3 PASS (0 mismatches, 0 undeclared); L2 cbug:61/61 PASS (0 mismatches, 0 undeclared); L2 wc:9/9 PASS (0 mismatches, 0 undeclared); L2 anim:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x509DF0 | CCam::KeepTrackOfTheSpeed | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x50D7A0 | CCam::CacheLastSettingsDWCineyCam | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x509810 | CHandShaker::~CHandShaker | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x517740 | CHandShaker::CHandShaker | L2 | L2 grove:6/6 PASS (0 mismatches, 0 undeclared); L2 pr:6/6 PASS (0 mismatches, 0 undeclared); L2 cheats:6/6 PASS (0 mismatches, 0 undeclared); L2 cbug:6/6 PASS (0 mismatches, 0 undeclared); L2 wc:6/6 PASS (0 mismatches, 0 undeclared); L2 anim:6/6 PASS (0 mismatches, 0 undeclared) |
| 0x517330 | CHandShaker::SetDefaults | L2 | split grove:6/6 PASS (0 mismatches, 0 undeclared); split pr:6/6 PASS (0 mismatches, 0 undeclared); split cheats:6/6 PASS (0 mismatches, 0 undeclared); split cbug:6/6 PASS (0 mismatches, 0 undeclared); split wc:6/6 PASS (0 mismatches, 0 undeclared); split anim:6/6 PASS (0 mismatches, 0 undeclared) |
| 0x50D860 | CHandShaker::Reset | L2 | split grove:6/6 PASS (0 mismatches, 0 undeclared); split pr:6/6 PASS (0 mismatches, 0 undeclared); split cheats:6/6 PASS (0 mismatches, 0 undeclared); split cbug:6/6 PASS (0 mismatches, 0 undeclared); split wc:6/6 PASS (0 mismatches, 0 undeclared); split anim:6/6 PASS (0 mismatches, 0 undeclared) |
| 0x517760 | CIdleCam::CIdleCam | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50E6D0 | CIdleCam::Init | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:1/1 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared); L2 cbug:1/1 PASS (0 mismatches, 0 undeclared); L2 wc:1/1 PASS (0 mismatches, 0 undeclared); L2 anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x50A160 | CIdleCam::Reset | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared); L2 pr:1/1 PASS (0 mismatches, 0 undeclared); L2 cheats:1/1 PASS (0 mismatches, 0 undeclared); L2 cbug:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x50E690 | CIdleCam::IdleCamGeneralProcess | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50EB50 | CIdleCam::SetTargetPlayer | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50EAE0 | CIdleCam::GetLookAtPositionOnTarget | implemented-needs-L2 | no call in grove / pr / cheats / cbug / wc / anim |
| 0x50A200 | CIdleCam::ProcessIdleCamTicker | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x43C610 | CCurves::DistForLineToCrossOtherLine | L1 | L1 1,060,000 cases, 0 mismatches (signalling-NaN inputs unverified); no call in grove / pr / cheats / cbug / wc / anim |
| 0x43C880 | CCurves::CalcCorrectedDist | L2 | L1 not authoritative (Unicorn FSIN/FCOS; 10,224 of 1,060,000 differ, minimal case curr = inf: Unicorn returns the operand, CPU and port the indefinite), routed to L2; L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x43C660 | CCurves::CalcSpeedVariationInBend | L1 | L1 1,060,000 cases, 0 mismatches (signalling-NaN inputs unverified); L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x43C710 | CCurves::CalcSpeedScaleFactor | L1 | L1 1,060,000 cases, 0 mismatches (signalling-NaN inputs unverified); L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared); L2 pr:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cheats:1000/1001 PASS (0 mismatches, 0 undeclared); L2 cbug:1000/1001 PASS (0 mismatches, 0 undeclared); L2 wc:1000/1001 PASS (0 mismatches, 0 undeclared); L2 anim:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x84D6D0 | __init_gHandShaker | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared); split cbug:1/1 PASS (0 mismatches, 0 undeclared); split wc:1/1 PASS (0 mismatches, 0 undeclared); split anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x84D700 | __init_TheCamera | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared); split cbug:1/1 PASS (0 mismatches, 0 undeclared); split wc:1/1 PASS (0 mismatches, 0 undeclared); split anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x84D720 | __init_gIdleCam | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared); split cbug:1/1 PASS (0 mismatches, 0 undeclared); split wc:1/1 PASS (0 mismatches, 0 undeclared); split anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x84D740 | __init_preMirrorMat | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared); split cbug:1/1 PASS (0 mismatches, 0 undeclared); split wc:1/1 PASS (0 mismatches, 0 undeclared); split anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x84D750 | __init_gpCamColVars | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared); split cbug:1/1 PASS (0 mismatches, 0 undeclared); split wc:1/1 PASS (0 mismatches, 0 undeclared); split anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x84D770 | __init_g_b6fec0 | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared); split pr:1/1 PASS (0 mismatches, 0 undeclared); split cheats:1/1 PASS (0 mismatches, 0 undeclared); split cbug:1/1 PASS (0 mismatches, 0 undeclared); split wc:1/1 PASS (0 mismatches, 0 undeclared); split anim:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x420C40 | CCamera::IsSphereVisible- | ported-unhookable | protection reads the entry bytes as data (push 0x420C46 at 0x14664CF); camctl::is_sphere_visible_with_matrix, reached through orig! by the Rust CCamera::IsSphereVisible (L2 PASS) |
| 0x50AC20 | CCamera::Fade | ported-unhookable | protection reads the entry bytes as data (sbb edx, [0x50AC20] at 0x1566C27); camfx::fade |
| 0x5150E0 | CCamera::CalculateDerivedValues | ported-not-hooked | L2 pause-restart (first build, hooked): 1000/1000 cases differ only in m_mMatInverse +0xC (RwMatrix flags): CMatrix::Invert-2 leaves its stack local's pad unwritten, so the value is the caller's stack garbage (orig 4); camctl::calculate_derived_values_port |
