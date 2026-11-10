# PORT-PHYS: subsystem PHYS (crate `sa-world`, module `physical`)

Every row of `catalog/state.csv` with `subsystem == PHYS` (73 rows: `CPhysical`, `cBuoyancy`, the
`CTimer::UpdateTimeStep` helper 0x541F40) has a decision below. Follows ADR-001 and the PORT-COLL
pattern (`docs/arch/coll-port.md`). Agent states (the main session sets `catalog/state.csv`):

| state | rows | meaning here |
|---|---|---|
| L1 | 6 | `sa-oracle l1 <key> --cases 1000000`: 0 mismatches (Unicorn, `SA_WINEPREFIX=/nonexistent`; SNaN inputs unverified with the native backend off, as COLL); all but UpdateTimeStep also have L2 evidence |
| L2 | 31 | in-process L2 (`tools/e2e/port-l2.sh`, LANE=1): >= 1000 cases or every call, 0 mismatches, 0 undeclared writes (double-run on the grove walk `grove`/`sec` or pause-restart `pr`), or split-run PASS with every call compared (`split-grove`/`split-pr`) |
| implemented | 4 | ported and hooked, all-Rust traces identical, but the L2/split compare is not clean (reason in the row) |
| implemented-needs-L2 | 21 | ported and hooked, no call in either recording (no evidence) |
| deferred | 11 | not ported in this pass (reason in the row) |

Ported = 62 rows, 62 `#[hook]` registrations (all hookable).

## Module

`crates/sa-world/src/physical/` (no new crate: PLAN §2 places CPhysical in `sa-world`). Same rules as
`collision`: `#![no_std]`, `#![deny(unsafe_code)]`, game state through `globals!`/`GPtr`, every other
catalogued function through `orig!` (so its toggle governs calls from Rust), every port registered
with `#[hook]` and listed in its module's `HOOKS`; host, i686 (`in-process`) and wasm32 builds.

| file | contents |
|---|---|
| `shared.rs` | field accessors on a `CEntity*`/`CPhysical*` byte pointer (`rf/wf/ru/wu/rb/wb/rh/wh/rp/wp`, raw-word vector copies), the CPhysical field offsets, `.rdata` constants `k::K_<addr>` (f32 bits), the `.data` tuning globals (0x8CD784..0x8CD7FC, `CWorld::ms_listMovingEntityPtrs`), shared callees (`Multiply3x3`, `CrossProduct`, references, `GetColModel`, `rand`), `ci_pow` (`_CIpow` 0x822130) |
| `base.rs` | constructor/destructors, `Add`/`Remove`/`RemoveAndAdd` (sector lists), moving list, collision records, `GetBoundRect`, lighting, `CanPhysicalBeDamaged`, `SkipPhysics`, `SetVelocity`, `ApplyMoveForce(x,y,z)`, `CTimer::UpdateTimeStep` |
| `forces.rs` | the move/turn force parts (`ApplyMoveForce`, `ApplyTurnForce`, `ApplyForce`, the friction variants), `GetSpeed`, `ApplyMoveSpeed`, `ApplyTurnSpeed`, `ApplyGravity`, `ApplyAirResistance`, `SetDamagedPieceRecord` |
| `collide.rs` | `ApplyCollision(CEntity*, CColPoint&, float&)`, `ApplySoftCollision(CEntity*, ...)` |
| `spring.rs` | `ApplySpringCollision(Alt)`, `ApplySpringDampening(Old)` |
| `friction.rs` | `ApplyFriction(float, CColPoint&)` (incl. the spark loop with `rand()` attributed to 0x5458C5) |
| `attach.rs` | attach/detach, `PlacePhysicalRelativeToOtherPhysical`, `ApplyScriptCollision`, `ProcessEntityCollision`, `CheckCollision(_SimpleCar)`, `TestCollision`, `ProcessControl`, `UnsetIsInSafePosition`, `ApplyFriction()`, `ProcessShift`, `ApplySpeed` |
| `buoyancy.rs` | `cBuoyancy` constructor, `CalcBuoyancyForce`, `FindWaterLevel(Norm)`, `SimpleSumBuoyancyData`, `PreCalcSetup`, `SimpleCalcBuoyancy`, `ProcessBuoyancy` |

Shared-crate changes: `sa_core::orig::x87_binary` (phase A: call an original x87-stack intrinsic such
as `_CIpow` 0x822130 with two `fld qword` operands under the game's MXCSR, result `fstp qword`; phases
B/C: panic). `_CIpow` checks MXCSR/CW and picks an SSE2 path, so it cannot be modelled without RE of
the CRT; it is called as the original does. sa-oracle: `specs_phys.rs` (6 L1 specs) chained into
`specs::all()`. No harness core change.

## Conventions worth knowing

- **Float ports** are line-by-line transliterations with `collision::fpu::Fpu` (operand forms, f32
  stores, register reuse, `test ah, mask` + `jp`/`jne`), as in COLL. Callees that the original calls
  (`Multiply3x3`, `CrossProduct`, `GetSpeed`, `ApplyForce`, ...) are called through `orig!` with
  pointers to Rust locals, never inlined.
- **By-value vectors** are `CVector` parameters (3 stack words, i686 byval = MSVC layout); raw copies
  (`SetVelocity`, attach offsets) keep the bits (`[u32; 3]` internally).
- **FPATAN** (`ApplySpeed`'s heading `atan2(-fwd.x, fwd.y)`) uses `X87::fpatan` (f64 model, the
  documented sa-math approximation; stored to f32 at once). L2 decides it on the real CPU.
- **Preserved registers.** `ApplyMoveForce(CVector)` has `preserve = "edx"` (found by sa-hook's
  `callers` test: ApplyCollision 0x543643/0x544E14 push the EDX they loaded before the call).
  `ApplyMoveSpeed` has `preserve = "ecx"`: `ApplySpeed` calls `ApplyTurnSpeed` right after it without
  reloading `this` (0x547D96, 0x547EDD, 0x5480B0, 0x548248). **Harness gap (report):** sa-hook's
  `callers` test ends a path at the next `call`, so an *implicit* ECX use by a following thiscall is
  not detected; a scan of every PHYS hook's call sites (capstone, scratch script `ecxscan.py`) found
  only these four sites. The test should treat a `call` to a thiscall callee as an ECX read.
- **Uninitialised stack.** `ApplyFriction()` (bDisableZ objects), `ApplySpeed` (bounded door) and
  `ApplyScriptCollision` build a local `CColPoint` with only point and normal set; the original passes
  whatever its stack held in the other fields (`ApplyFriction(float, CColPoint&)` reads the surface
  bytes +0x20/+0x23 above speed 0.1). The ports pass 0 there (ledger candidate, like COLL's
  `PointInPoly`).
- **L2 extra regions** (`tools/e2e/l2-extras.txt`, section `# PORT-PHYS`): the PtrNodeDouble and
  EntryInfoNode pools (header, byte map, objects) for the sector/moving lists, `CWorld::
  ms_aRepeatSectors` (0xB992B8, 0xC00), the entity matrix `[ecx+0x14]` (0x48), the out-parameters,
  subclass fields outside the 0x138-byte `this` (`CVehicle` +0x45C, `CObject` +0x140/+0x148), the
  `CReferences` table and free head plus the entities' `m_pReferences` (+0x24), the audio-entity
  state 0xB5F000..0xB6F000 that `ReportCollision` writes. Each line cites the run that showed it was
  needed.

## Verification (commands, results)

- L1 (`specs_phys.rs`, 7 specs): `SA_WINEPREFIX=/nonexistent systemd-run ... ./target/release/sa-oracle
  l1 <key> --cases 1000000 --no-fixtures --threads 8` → `PhysUpdateTimeStep` 1,000,058,
  `PhysSetVelocity` 1,060,000, `PhysApplyMoveForceVec` 1,060,000 (EDX compared),
  `PhysApplyFrictionMoveForce` 1,060,000, `PhysCanPhysicalBeDamaged` 1,002,000, `PhysApplyMoveSpeedZero`
  1,060,000 (zeroing path only, ECX compared), `BuoyancyCtor` 1,002,000 cases: all `mismatches=0`
  (the float specs INCOMPLETE only by their SNaN cases, unverified with the native backend off, as COLL).
- L2 double-run (`DXVK_CONFIG=... SA_ROOT=/home/ghosty2004/sa-engine LANE=1 L2_PER_FRAME=1000
  tools/e2e/port-l2.sh @tools/e2e/lists/phys-l2b.txt REC --runs "plain l2 rust"`), grove walk
  (`phys-l2b`) and pause-restart (`pr-phys-l2b`); the sector-list trio rerun with the full extras
  (`phys-sec`, `L2_SPOOL_DISCARD=1`, `--runs "plain l2"`): 0 mismatches, 0 undeclared writes, >= 1000
  cases or every call for every row claimed L2 below; `phys-sec`: (b) l2 trace == plain on all 5200
  frames. (`phys-l2b`'s own (b) diverged at frame 204 because CPhysical::Add's big-building path wrote
  the world sectors/LOD lists/PtrNodeSingle pool outside its declared regions at that time; fixed by
  the extras, see `phys-sec`.)
- Split runs (INF-17, `--runs "plain split"`, groups `tools/e2e/lists/phys-s1..s5.txt`) for the
  functions that cannot run twice on one state (constructor/destructor, FX spark emitters, matrix
  movers, the collision checkers): PASS for Constructor 943/943, Destructor 767/767,
  ApplyFriction(float, CColPoint&) 1000/1000, ApplyFriction() 1000/1000, ProcessControl 1000/1000,
  ApplySpeed 1000/1000, UnsetIsInSafePosition 151/151, cBuoyancy::ProcessBuoyancy 1000/1000 (grove);
  splito/splitr traces identical to plain on all frames.
- L3: `port-l2.sh @tools/e2e/lists/phys-all.txt REC --runs "plain l2 rust"` (all 59 non-ctor/dtor PHYS
  hooks on Rust at once; `phys-all`, `phys-l2b`): (c) rust trace == all-original on all 5200 frames,
  17 subsystems. `port-l2.sh ports GW --runs "plain rust"` (959 engine ports incl. the 62 PHYS hooks,
  `ports-gw`): IDENTICAL over 5200 frames; `ports PR` (`ports-pr`): IDENTICAL over 3700 frames, 17 subsystems. Per-hook hook-off runs
  were not made, so no row claims L3.
- `cargo test -p sa-world` (3 pass), `cargo test --release -p sa-oracle --test l1` (16 pass),
  `cargo test -p sa-hook --lib -- every_hook_installs_on_the_exe matches_catalogue_and_exe
  callers_reading_ecx_edx_after_a_hook_are_declared` (3 pass); `cargo clippy -D warnings`: sa-world
  host (all targets), i686 (`--features in-process`), wasm32; sa-oracle, sa-core host; sa-hook i686;
  `cargo build --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook` OK.

## Divergence notes (ledger candidates)

- Uninitialised `CColPoint` fields: `ApplyFriction()` (0x5483D0, `bDisableZ` path), `ApplySpeed`
  (0x547B80, bounded-door path) and `ApplyScriptCollision` (0x546ED0) pass a local `CColPoint` whose
  fields other than point and normal are stack garbage in the original; `ApplyFriction(float,
  CColPoint&)` reads surfaceA/surfaceB (+0x20/+0x23) of it above speed 0.1 (spark effects). The ports
  pass 0. Observable only on those paths (no case in either recording reached them with speed > 0.1).
- `ApplySpeed` heading `atan2(-fwd.x, fwd.y)`: FPATAN through `X87::fpatan` (f64 model) stored to f32;
  can differ from the CPU's 64-bit FPATAN only within 2^-29 ulp of an f32 rounding boundary (sa-math's
  documented approximation). Split L2: 1000/1000 PASS.
- `ApplyMoveSpeed` preserves ECX and `ApplyMoveForce(CVector)` preserves EDX for original callers
  (behaviour-preserving, not a divergence; listed because the preserve wrappers are a deliberate ABI
  addition).

## Open items

- 11 deferred rows (table): the two-body responses (`ApplyCollision-2`, `ApplySoftCollision-2`,
  `ApplyCollisionAlt`, `ApplyFriction-other`), the sector drivers (`ProcessCollisionSectorList(_SimpleCar)`,
  `ProcessShiftSectorList`, `ProcessCollision`), `PositionAttachedEntity`, `ProcessBuoyancyBoat`,
  `AddSplashParticles`: ~12,000 instructions, each a dedicated pass.
- `CheckCollision`, `CheckCollision_SimpleCar`, `ProcessShift`, `ProcessEntityCollision`: all-Rust
  traces identical (L3-level evidence), but the split-run compare is inconclusive: every one reaches
  `CCollision::ProcessColModels` (0x4185C0), which writes scratch statics at 0x968940..0x9689D0 (rwsets:
  `A imm` + direct W) that hold stack addresses / per-call counts; under a Rust frame the stack depth
  differs (ProcessShift ordinal 5: 0x96896C orig 0x0036ECDC, rust 0x0036EB74), so later inputs differ
  (`DIVERGED`/`FAIL` at the first such byte). Needs a reviewed normaliser rule (like gpColCache 0x9655C8)
  from the harness owner; CheckCollision's first mismatch (0x96894C: orig 0x20F3CAA0, rust 0, frame
  204) must be re-checked after that rule before calling it a port bug.
- 21 `implemented-needs-L2` rows have no call in either recording (attach/detach, script collision,
  spring collision, most of cBuoyancy (no water), TestCollision, the scalar deleting destructor).
- **Harness gap**: sa-hook's `callers` test stops at the next `call`, so a caller that passes ECX
  unchanged into a following thiscall (ApplySpeed after ApplyMoveSpeed) is not flagged; found by a
  capstone scan instead (`preserve = "ecx"` added). Suggest treating such calls as ECX reads.
- SNaN cases of the float L1 specs: rerun with the native backend (CPU-decided).

## Functions (all 73 PHYS rows)

| address | name | conv | port (sa-world `physical::`) | state | evidence | deviations kept / notes |
|---|---|---|---|---|---|---|
| 0x441130 | CPhysical::SetVelocity | thiscall | base::set_velocity | L1 | L1 1,060,000 cases, 0 mismatches; L2 grove:1/1 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x4ABBA0 | CPhysical::ApplyMoveForce | thiscall | base::apply_move_force_xyz | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x541F40 | FUN_00541f40 | cdecl | base::update_time_step | L1 | L1 1,000,058 cases, 0 mismatches | proposed name CTimer::UpdateTimeStep (spec CTimer.md) |
| 0x542260 | CPhysical::Constructor | thiscall | base::constructor | L2 | split-grove: PASS 943/943 compared, 0 mismatches; split-pr: PASS-FP 746/746 compared, 0 mismatches | gr-dev Physical.cpp:128 keep-original: +0xBA (contact surface) not initialised; also +0x3C, +0x9C, +0xA0, +0x100..+0x117 untouched |
| 0x542450 | CPhysical::Destructor | thiscall | base::destructor | L2 | split-grove: PASS 767/767 compared, 0 mismatches; split-pr: PASS 602/602 compared, 0 mismatches |  |
| 0x5424C0 | CPhysical::Remove | thiscall | base::remove | L2 | L2 sec:894/894 (0 mismatches, 0 undeclared); L2 pr:753/753 (0 mismatches, 0 undeclared) |  |
| 0x542560 | CPhysical::RemoveAndAdd | thiscall | base::remove_and_add | L2 | L2 sec:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) | gr-dev Physical.cpp:621 harmless; new nodes allocated link first, pushed at the list front; a non-physical type reuses the stale list pointer (starts as the integer y1) |
| 0x542800 | CPhysical::AddToMovingList | thiscall | base::add_to_moving_list | L2 | L2 grove:386/386 (0 mismatches, 0 undeclared); L2 pr:198/198 (0 mismatches, 0 undeclared) | a failed allocation still writes through the null link |
| 0x542860 | CPhysical::RemoveFromMovingList | thiscall | base::remove_from_moving_list | L2 | L2 grove:894/894 (0 mismatches, 0 undeclared); L2 pr:753/753 (0 mismatches, 0 undeclared) |  |
| 0x5428C0 | CPhysical::SetDamagedPieceRecord | thiscall | forces::set_damaged_piece_record | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:891/891 (0 mismatches, 0 undeclared) |  |
| 0x5429F0 | CPhysical::ApplyMoveForce-vec | thiscall | forces::apply_move_force | L1 | L1 1,060,000 cases, 0 mismatches; L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) | preserve = "edx" (callers 0x543643, 0x544E14) |
| 0x542A50 | CPhysical::ApplyTurnForce | thiscall | forces::apply_turn_force | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x542B50 | CPhysical::ApplyForce | thiscall | forces::apply_force | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x542CE0 | CPhysical::GetSpeed | thiscall | forces::get_speed | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x542DD0 | CPhysical::ApplyMoveSpeed | thiscall | forces::apply_move_speed | L2 | L1 (zeroing path only, ECX compared) 1,060,000 cases, 0 mismatches; L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) | preserve = "ecx" (ApplySpeed calls ApplyTurnSpeed after it without reloading this) |
| 0x542E20 | CPhysical::ApplyTurnSpeed | thiscall | forces::apply_turn_speed | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x542FE0 | CPhysical::ApplyGravity | thiscall | forces::apply_gravity | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5430A0 | CPhysical::ApplyFrictionMoveForce | thiscall | forces::apply_friction_move_force | L1 | L1 1,060,000 cases, 0 mismatches; L2 grove:410/410 (0 mismatches, 0 undeclared); L2 pr:156/156 (0 mismatches, 0 undeclared) |  |
| 0x543100 | CPhysical::ApplyFrictionTurnForce | thiscall | forces::apply_friction_turn_force | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x543220 | CPhysical::ApplyFrictionForce | thiscall | forces::apply_friction_force | L2 | L2 grove:622/622 (0 mismatches, 0 undeclared) |  |
| 0x5433B0 | CPhysical::SkipPhysics | thiscall | base::skip_physics | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x543490 | CPhysical::AddCollisionRecord | thiscall | base::add_collision_record | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x543540 | CPhysical::GetHasCollidedWith | thiscall | base::get_has_collided_with | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x543580 | CPhysical::GetHasCollidedWithAnyObject | thiscall | base::get_has_collided_with_any_object | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x5435C0 | CPhysical::ApplyCollision-1 | thiscall | collide::apply_collision | L2 | L2 grove:171/171 (0 mismatches, 0 undeclared) |  |
| 0x543890 | CPhysical::ApplySoftCollision-1 | thiscall | collide::apply_soft_collision | L2 | L2 grove:13/13 (0 mismatches, 0 undeclared) | with bDisableTurnForce ApplyCollision is called first and the function goes on (result ignored) |
| 0x543C90 | CPhysical::ApplySpringCollision | thiscall | spring::apply_spring_collision | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x543D60 | CPhysical::ApplySpringCollisionAlt | thiscall | spring::apply_spring_collision_alt | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x543E90 | CPhysical::ApplySpringDampening | thiscall | spring::apply_spring_dampening | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x544100 | CPhysical::ApplySpringDampeningOld | thiscall | spring::apply_spring_dampening_old | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x544280 | CPhysical::RemoveRefsToEntity | thiscall | base::remove_refs_to_entity | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x5442F0 | CPhysical::DettachEntityFromEntity | thiscall | attach::dettach_entity_from_entity | implemented-needs-L2 | no call in grove walk / pause-restart | local CMatrix positions 0 (uninitialised stack in the original; only the rotation is used) |
| 0x5446A0 | CPhysical::DettachAutoAttachedEntity | thiscall | attach::dettach_auto_attached_entity | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x5447B0 | CPhysical::GetLightingFromCol | thiscall | base::get_lighting_from_col | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) | constant 0x3EAAAA9F (0.333333), not 1/3 as gr |
| 0x544850 | CPhysical::GetLightingTotal | thiscall | base::get_lighting_total | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) | pushes the dword (this & ~0xFF) | flag as the original |
| 0x5448B0 | CPhysical::CanPhysicalBeDamaged | thiscall | base::can_physical_be_damaged | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:40/40 (0 mismatches, 0 undeclared) | jump table 0x544938/0x544954 transcribed |
| 0x544990 | CPhysical::`scalar deleting destructor' | thiscall | base::scalar_deleting_destructor | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x5449B0 | CPhysical::GetBoundRect | thiscall | base::get_bound_rect | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x544A30 | CPhysical::Add | thiscall | base::add | L2 | L2 sec:876/876 (0 mismatches, 0 undeclared); L2 pr:821/821 (0 mismatches, 0 undeclared) | node allocated before link; a NULL node/link is still written through (crash as the original) |
| 0x544C40 | CPhysical::ApplyAirResistance | thiscall | forces::apply_air_resistance | L2 | L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) | `_CIpow` (0x822130) called as the original (sa_core::orig::x87_binary) |
| 0x544D50 | CPhysical::ApplyCollisionAlt | — | — | deferred | not ported in this pass: 560 instructions (two-body collision response with the turn masses of both bodies), called from ApplyCollision-2 / ProcessCollisionSectorList; needs a dedicated pass |  |
| 0x5454C0 | CPhysical::ApplyFriction-self | thiscall | friction::apply_friction | L2 | split-grove: PASS 1000/1000 compared, 0 mismatches; split-pr: PASS 51/51 compared, 0 mismatches | gr-dev Physical.cpp:1755 keep-original: spark spread `t*0.4f - 0.2f`; rand() attributed to 0x5458C5; `bDisableTurnForce` path adds to friction move x/y only |
| 0x545980 | CPhysical::ApplyFriction-other | — | — | deferred | not ported in this pass: 1030 instructions (two-body friction with sparks and rand), called from ProcessCollisionSectorList; needs a dedicated pass |  |
| 0x546670 | CPhysical::ProcessShiftSectorList | — | — | deferred | not ported in this pass: 507 instructions (shift against the sector lists, ProcessColModels, uses the .data multipliers 0x8CD7D0/0x8CD7D4 per gr-dev Physical.cpp:2127); needs a dedicated pass |  |
| 0x546D00 | CPhysical::ProcessEntityCollision | thiscall | attach::process_entity_collision | implemented | split-grove: FAIL 1000/1000 compared, 3 mismatches; split-pr: FAIL 1000/1000 compared, 3 mismatches | as CheckCollision |
| 0x546DB0 | CPhysical::PlacePhysicalRelativeToOtherPhysical | cdecl | attach::place_physical_relative_to_other_physical | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x546ED0 | CPhysical::ApplyScriptCollision | thiscall | attach::apply_script_collision | implemented-needs-L2 | no call in grove walk / pause-restart | local CColPoint fields other than point/normal 0 (uninitialised stack in the original) |
| 0x546FF0 | CPhysical::PositionAttachedEntity | — | — | deferred | not ported in this pass: 801 instructions (attachment placement with quaternion slerp, DettachAutoAttachedEntity and GetRandomNumberInRange(-1,1) per gr-dev Physical.cpp:2272); needs a dedicated pass |  |
| 0x547B80 | CPhysical::ApplySpeed | thiscall | attach::apply_speed_port | L2 | split-grove: PASS 1000/1000 compared, 0 mismatches; split-pr: PASS-FP 1000/1000 compared, 0 mismatches | FPATAN via the f64 model; bounded-door CColPoint fields 0 (uninitialised stack in the original) |
| 0x548320 | CPhysical::UnsetIsInSafePosition | thiscall | attach::unset_is_in_safe_position | L2 | split-grove: PASS 151/151 compared, 0 mismatches; split-pr: PASS-FP 215/215 compared, 0 mismatches |  |
| 0x5483D0 | CPhysical::ApplyFriction-void | thiscall | attach::apply_friction_void | L2 | split-grove: PASS 1000/1000 compared, 0 mismatches; split-pr: PASS 1000/1000 compared, 0 mismatches | local CColPoint fields other than point/normal 0 (uninitialised stack in the original) |
| 0x5485E0 | CPhysical::ProcessControl | thiscall | attach::process_control | L2 | split-grove: PASS 1000/1000 compared, 0 mismatches; split-pr: PASS 1000/1000 compared, 0 mismatches |  |
| 0x548680 | CPhysical::ApplyCollision-2 | — | — | deferred | not ported in this pass: 2184 instructions, 72 calls (two-body ApplyCollision with stats, damage and the rand()%30+30 / rand()%100 sites of gr-dev Physical.cpp:2708/2734); needs a dedicated pass |  |
| 0x54A2C0 | CPhysical::ApplySoftCollision-2 | — | — | deferred | not ported in this pass: 1771 instructions (two-body soft collision; the unchecked CPed+0x568 read of gr-dev Physical.cpp:3339 and rand()%30+30); needs a dedicated pass |  |
| 0x54BA60 | CPhysical::ProcessCollisionSectorList | — | — | deferred | not ported in this pass: 1593 instructions, 82 calls (the main collision driver over the sector lists); needs a dedicated pass |  |
| 0x54CFF0 | CPhysical::ProcessCollisionSectorList_SimpleCar | — | — | deferred | not ported in this pass: 453 instructions (simple-car sector collision); needs a dedicated pass |  |
| 0x54D570 | CPhysical::AttachEntityToEntity-vec | thiscall | attach::attach_entity_to_entity_vec | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x54D690 | CPhysical::AttachEntityToEntity-quat | thiscall | attach::attach_entity_to_entity_quat | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x54D920 | CPhysical::CheckCollision | thiscall | attach::check_collision | implemented | split-grove: FAIL 7/1000 compared, 1 mismatches; split-pr: FAIL 7/1000 compared, 1 mismatches | split inconclusive: ProcessColModels scratch statics 0x968940..0x9689D0 (see Open items) |
| 0x54DAB0 | CPhysical::CheckCollision_SimpleCar | thiscall | attach::check_collision_simple_car | implemented | split-grove: DIVERGED 673/1000 compared, 0 mismatches; split-pr: DIVERGED 673/1000 compared, 0 mismatches | as CheckCollision |
| 0x54DB10 | CPhysical::ProcessShift | thiscall | attach::process_shift | implemented | split-grove: FAIL 46/476 compared, 30 mismatches; split-pr: FAIL 26/26 compared, 14 mismatches | as CheckCollision |
| 0x54DEC0 | CPhysical::TestCollision | thiscall | attach::test_collision | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x54DFB0 | CPhysical::ProcessCollision | — | — | deferred | not ported in this pass: 616 instructions (ProcessCollision: the step subdivision loop around CheckCollision/ApplySpeed); needs a dedicated pass |  |
| 0x6C2740 | cBuoyancy::cBuoyancy | thiscall | buoyancy::constructor | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1/1 (0 mismatches, 0 undeclared); L2 pr:1/1 (0 mismatches, 0 undeclared) |  |
| 0x6C2750 | cBuoyancy::CalcBuoyancyForce | thiscall | buoyancy::calc_buoyancy_force | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x6C2810 | cBuoyancy::FindWaterLevel | thiscall | buoyancy::find_water_level | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x6C28C0 | cBuoyancy::FindWaterLevelNorm | thiscall | buoyancy::find_water_level_norm | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x6C2970 | cBuoyancy::SimpleSumBuoyancyData | thiscall | buoyancy::simple_sum_buoyancy_data | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x6C2B90 | cBuoyancy::PreCalcSetup | thiscall | buoyancy::pre_calc_setup_port | implemented-needs-L2 | no call in grove walk / pause-restart | boat jump table 0x6C2F68/0x6C2F8C transcribed |
| 0x6C3030 | cBuoyancy::ProcessBuoyancyBoat | — | — | deferred | not ported in this pass: 288 instructions (boat buoyancy over the 3x3 volume distribution tables 0x8D32CC..); no call in the recordings (no water); needs a dedicated pass |  |
| 0x6C34E0 | cBuoyancy::AddSplashParticles | — | — | deferred | not ported in this pass: 386 instructions (splash particles via FxSystem_c::AddParticle, rand per gr-dev Buoyancy.cpp:295); needs a dedicated pass |  |
| 0x6C3B00 | cBuoyancy::SimpleCalcBuoyancy | thiscall | buoyancy::simple_calc_buoyancy_port | implemented-needs-L2 | no call in grove walk / pause-restart |  |
| 0x6C3EF0 | cBuoyancy::ProcessBuoyancy | thiscall | buoyancy::process_buoyancy | L2 | split-grove: PASS 1000/1000 compared, 0 mismatches; split-pr: PASS-FP 1000/1000 compared, 0 mismatches |  |

