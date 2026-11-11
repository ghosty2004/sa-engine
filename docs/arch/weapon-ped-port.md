# PORT-WEAPON-PED: WEAPON and the leaf-ward part of PED (crates `sa-weapon`, `sa-ped`)

Scope (task PORT-WEAPON-PED): every catalogue row with `subsystem == WEAPON` (158: `CWeapon`, `CWeaponInfo`,
`CProjectile`/`CProjectileInfo`, `CBulletInfo`, `CShotInfo`, `CExplosion`, `CFire`/`CFireManager`,
`CCreepingFire`, `CWaterCannon(s)`, `CWeaponEffects`, their static initialisers/`atexit` destructors) and
every row with `subsystem == PED` (372), of which the leaf-ward part was ported: `CPedType`, `CPedStats`,
`CPlayerInfo`/`CPlayerPedData` helpers, clothes data (`CClothes`, `CPedClothesDesc`, `CPlayerSkin`),
`CWanted`/`CCrime`, the damage-response calculator, `CPed`/`CPlayerPed`/`CCopPed` accessors and small helpers.
The big `CPed`/`CPlayerPed` process functions, whole-ped constructors/destructors and everything that creates or
drives AI tasks/events are deferred with a reason per row. Follows ADR-001 and the PORT-WORLD/PORT-PHYS
pattern; ports follow the 1.0 US bytes (PLAN §4), never gta-reversed's fixes (`docs/re/specs/WEAPON/*.md`,
`docs/re/specs/PED/*.md` and `catalog/gr-deviations.csv` were applied).

Agent states (the main session sets `catalog/state.csv`):

| state | WEAPON | PED | total | meaning here |
|---|---|---|---|---|
| L1 | 11 | 20 | 31 | `sa-oracle l1 <key> --cases 1000000`: 0 mismatches (Unicorn, native backend off; float specs INCOMPLETE only by SNaN cases); most also have L2 evidence |
| L2 | 48 | 102 | 150 | in-process L2 on lane 5: plain double-run >= 1000 cases or every call, 0 mismatches, 0 undeclared writes, or split-run PASS with every recorded call compared (at most 150 per function and recording, `L2_MAX=150`: 16 L2 rows rest on such 150-case split evidence only), and no FAIL in any run |
| implemented | 4 | 8 | 12 | ported and hooked, all-Rust traces identical on all four recordings, but the L2 evidence is only PASS-FP / DIVERGED (split-run input differences, see Verification) |
| implemented-needs-L2 | 58 | 87 | 145 | ported and hooked, no call in any of the four recordings (no evidence) |
| ported-unhookable | 2 | 2 | 4 | ported, not registered (reason in the row) |
| deferred | 35 | 153 | 188 | not ported in this pass (reason in the row) |
| total | 158 | 372 | 530 | |

Ported = 342 rows, 338 `#[hook]` registrations (sa-weapon 121, sa-ped 217).

## Crates

Two new crates (PLAN §2 names), both `#![no_std]`, `#![deny(unsafe_code)]`, host + i686 (`in-process`) +
wasm32, depending on sa-core, sa-math, sa-macros and sa-world (for `world::common`'s byte-offset accessors,
the `Fpu` stack model and the PHYS/WORLD Rust paths). No dependency between them: calls across WEAPON and PED
go through `orig!` with the original address (so each callee's toggle decides). Each crate has `hooks.rs`
(`MODULES`/`GLOBALS`, the initial-value / declared-once / list-complete tests, which also check against
sa-world and, for sa-ped, sa-weapon). sa-hook appends `sa_weapon::hooks::ALL` then `sa_ped::hooks::ALL`
after sa-world's (`table.rs` `build`, `callers.rs`); sa-oracle chains `specs_weapon.rs` / `specs_ped.rs`.

| crate / module | contents |
|---|---|
| `sa-weapon/common.rs` | re-exports of the sa-world accessors, shared `orig!` (rand, `GetRandomNumberInRange`, `_CIpow`, camera, world line tests) |
| `weapon_info.rs` | `CWeaponInfo` (table `aWeaponInfo` 0xC8AAB8, 0x70 each), lookups, reload/anim helpers, unnamed 0x61DB30 / 0x73A280 / 0x73A2B0 |
| `weapon.rs` | `CWeapon` leaves and mid-size bodies (type predicates, Reload, Initialise/Shutdown, Update, AddGunshell, DoWeaponEffect, heat-seeker target selection, FireFromCar, ProcessLineOfSight, ...) |
| `fire.rs` | `CFire`, `CFireManager` (60 fires at 0xB71F80), `CCreepingFire` |
| `explosion.rs` | `CExplosion` (16 at 0xC88950) incl. `Update` |
| `bullet_shot.rs` | `CBulletInfo` (8 at 0xC88740), `CShotInfo` (100 at 0xC89690) incl. `Update` |
| `projectile.rs` | `CProjectileInfo` (32 at 0xC891A8), `CProjectile` |
| `water_cannon.rs` | `CWaterCannon(s)` (3 at 0xC80740) |
| `effects.rs` | `CWeaponEffects` cross-hairs (0xC8A838) |
| `statics.rs` | static initialisers / `atexit` destructors |
| `sa-ped/wanted.rs` | `CWanted`, `CCrime` |
| `ped_type.rs` | `CPedType`, `CPedStats` |
| `damage.rs` | `CPedDamageResponseCalculator` constructor, `AccountForPedArmour`, `AccountForPedDamageStats`, `ComputeWillForceDeath` |
| `ped.rs`, `ped_weapon.rs` | `CPed` accessors/predicates/small helpers; weapon slots (`SetAmmo` with `min`, `SetCurrentWeapon`, goggles, `Dress`) |
| `player_ped.rs`, `player_info.rs`, `clothes.rs`, `misc.rs`, `small.rs` | `CPlayerPed`/`CPlayerInfo`/`CPlayerPedData`, clothes, `CCopPed`/`CPlayerSkin`/`CPedPlacement` helpers, forwarding thunks |

## Conventions and findings

- **Protection reads of entry bytes (INF-20).** A scanner (`tools/verify/hookscan.py`; proposed for the harness slot as part of sa-hook's
  install test) searches the game
  `.text` and `.HOODLUM` for every dword constant within `[entry-3, entry+5)` of a hooked entry and decodes
  the instruction around it. It reproduces all three known cases and found two new ones:
  `CPhysical::AttachEntityToEntity-quat` 0x54D690 (`xor ebx, [0x54D691]` at 0x40A4C3, PHYS) and
  `CPlayerPed::ResetSprintEnergy` 0x60A530 (`sbb ebx, [0x60A533]` at 0x4037BC, PED). **Decision taken for
  INF-20 in this branch:** the three known registered hooks (`CStreaming::AreAnimsUsedByRequestedModels`
  0x407AD0, `CColStore::LoadCollision` 0x410860, `CAtomicModelInfo::SetupVehicleUpgradeFlags` 0x4C4570) and
  0x54D690 are unregistered (ported, unhookable): the first weapon-combat recordings with every port installed
  in original mode crashed at frame ~1650 with `page fault on read access ... at 0x401E4C`, the instruction
  after the protection read of 0x410860 at 0x401E46 (the patched entry changes the value the protection
  computes an address from). With the four unregistered the recording runs to the end and replays IDENTICAL.
  Section 2 of the scanner lists callers that read ECX/EDX after the call or pass ECX into a following
  thiscall (the PHYS harness gap); the `preserve` declarations below come from it and sa-hook's `callers` test.
- **Preserved registers**: `CFire::Initialise` ECX (`CFire::Constructor` returns ECX after the call),
  `CCrime::FindImmediateDetectionRange` ECX, `CWanted::SetMaximumWantedLevel` ECX (`CWanted::CheatWantedLevel`
  pushes its ECX as the level after the call), `CClothes::GetTextureDependency` ECX+EDX; each has an L1 spec
  comparing them. `CPlayerPed::GetButtonSprintResults` (0x60A820) needs `preserve = "edx"` (0x60B430 reads EDX)
  but has no hosted spec, so it is not registered.
- **Other unhookable**: `CFireManager::PlentyFiresAvailable` (a loop branches back into the patched bytes,
  0x539344), one-byte `ret` bodies (`CWeaponInfo::Shutdown`, `CBulletInfo::Shutdown`, `CExplosion::Shutdown`,
  `CShotInfo::Shutdown`, `CPed::Update`, `CWanted::ResetPolicePursuit`, ...).
- **Original bugs kept** (selection): `CShotInfo::Update` range growth `ts*0.075f` (gta-reversed 0.0075f) and
  `_CIpow(0.96, ts)` in x87; `CWeapon::AddGunshell` keeps the discarded `GetRandomNumberInRange(-20, 20)` (one
  extra `rand()` per shell) and the `(z²+x²)+y²` order; `CWeaponInfo::GetWeaponInfo` with skill > 3 returns
  entry 0x2F (EAX left by its split chunk); `CExplosion::ClearAllExplosions` inner loop rewrites fixed
  addresses; `CBulletInfo::AddBullet` / `CShotInfo::AddShot` read one element past the array when full;
  `CExplosion::Update` keeps gr-deviating details (random order, victim position, `TriggerExplosion` flag =
  type dword with AL cleared); `CPed::SetAmmo` clips with `min`; `CPlayerPed::GetWantedLevel`/`Busted`
  dereference null without player data; `RemoveWeaponWhenEnteringVehicle` picks slot 2 in its shotgun case;
  `GetClothesModelFromName` returns 0 for unknown names; `ComputePursuitCopToDisplace` returns a dead cop at
  once; `CCrime::ReportCrime` dereferences the victim without a null check (type 2). `rand()` calls are
  attributed to the original return addresses.
- **Not reproduced (stack-only)**: the original writes into its own argument slots in
  `EvaluateTargetForHeatSeekingMissile`, `PickTargetForHeatSeekingMissile`, `CWeapon::Update` (reload state),
  `CCreepingFire::TryToStartFireAtCoors`; `CWeaponEffects::IsLockedOn` does not reproduce `_ftol`'s EDX;
  `DoWeaponEffect` passes the `.rdata` fx name instead of a stack copy (same bytes); the SEH frame of
  `CPlayerPedData::AllocateData`.
- **L1 coverage**: the hosted L1 image (0x500 bytes scratch, 8-byte pointers on the host) cannot hold the fire
  manager, explosion/shot arrays or `CPed` objects, so most ports rely on L2.

## Verification (commands, results)

All game runs on lane 5 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=5`, DXVK no-vsync, `L2_PER_FRAME=1000`),
outputs in the worktree's `target/e2e/lane-5/port-l2-<tag>/`. Recordings: grove walk
(`target/e2e/lane-2/inf4/rec.sarec`, 5200 frames), pause-restart (`target/e2e/corpus/pause-restart.sarec`,
3700), world-cheats (`target/e2e/corpus/world-cheats.sarec`, 3276) and the new **weapon-combat** recording
(`tools/e2e/programs/weapon-combat.txt`: grove start, KJKSZPJ weapon cheat, knife, desert eagle, sawn-off,
tec9, M4, sniper, flamethrower, two grenades, extinguisher, with targeting; recorded with
`tools/e2e/record-program.sh --replay` (replay == record IDENTICAL over 3500 frames), copied to
`/home/ghosty2004/sa-engine/target/e2e/corpus/weapon-combat.sarec`, sha256 `53d559db…52b14`; screenshots show
the deagle hitting a car, the 1-star wanted level, a car on fire after the flamethrower, and "Wasted" after
the grenades at frame ~2347, then the hospital respawn).

Hook lists: `tools/e2e/lists/wp-all.txt` (every registration), `wp-l2.txt` (plain `l2` mode),
`wp-split.txt` (hooks that create/delete entities, FX systems, references, audio, events, `atexit` or the
world lists: INF-17 split-run mode), `wp-small.txt` (the small ports added last). Extra regions: section
`PORT-WEAPON-PED` of `tools/e2e/l2-extras.txt`.

- **L1** (`SA_WINEPREFIX=/nonexistent sa-oracle l1 <key> --cases 1000000 --no-fixtures --threads 6`, specs in
  `crates/sa-oracle/src/specs_weapon.rs` (11) and `specs_ped.rs` (20)): every spec `mismatches=0`; float specs
  INCOMPLETE only by their signalling-NaN cases with the native backend off; rerun with the native backend
  deciding the SNaN cases (Wine available, `--cases 50000`): all 31 PASS, 0 mismatches. Every `preserve`
  hook has a spec comparing the preserved register.
- **L3-level frame identity** (`port-l2.sh @tools/e2e/lists/wp-all.txt REC --runs "plain rust"`, all 338
  WEAPON/PED hooks on Rust at once): IDENTICAL on every frame and all 17 subsystems on the weapon-combat
  recording (`c-rust`, 3500 frames), grove walk (`g-rust`, 5200), pause-restart (`p-rust`, `p-rust2`, 3700)
  and world-cheats (`w-rust`, `w-rust2`, 3276); no crash, panic or install failure. `port-l2.sh ports REC
  --runs "plain rust"` (all 1,527 engine-crate ports incl. these): IDENTICAL on the grove walk (`g-ports`,
  5200 frames) and on weapon-combat (`c-ports`, 3500 frames). Per-hook hook-off runs were not made, so no row
  claims L3.
- **L2, plain double-run** (`wp-l2.txt`, `L2_PER_FRAME=1000`; tags `c-l2`/`c-l2b`/`c-l2d` combat, `g-l2`/
  `g-l2b`/`g-l2d` grove, `w-l2`/`w-cwl2` cheats, `p-l2` pause-restart; the `b` runs repeat the hooks that had
  no case in the first run, so that callers listed in the same run do not hide them): 0 mismatches on every
  hook except `CPlayerPed::CheatWantedLevel` before its extra region (below). The first runs' FAILs were
  undeclared writes only (module arrays written by the `Initialise` functions, the weapon model-info ref
  counts written by `CWeapon::Initialise/Shutdown`, the wanted stat 0xB7902C): declared in `l2-extras.txt`,
  then PASS. (b) l2 trace == plain trace on every run.
- **`CPlayerPed::CheatWantedLevel` mismatch, found and resolved**: `w-l2`/`w-cwl` showed 2 mismatches (the
  original wrote stat 0x83 at 0xB7902C, the Rust run did not). Cause: the callee `CWanted::UpdateWantedLevel`
  writes the player's heap `CWanted` (+0x2C level) and then `IncrementStat(0x83, new - old)`; the heap object
  was not a declared region, so the original run's level change was not undone before the Rust run, which then
  saw a delta of 0. With `[[ecx+0x480]]:0x29C` declared (`w-cwl2`): PASS 4/4. A harness artefact, not a port
  bug; the same region was added for the other CPlayerPed wanted wrappers.
- **L2, split-run** (`wp-split.txt`, `L2_MAX=150 SPLIT_PER_FRAME=2 L2_SPOOL_MAX=6G`; the default settings hit
  the 2 GiB spool quota at frame ~850 on both recordings): grove `g-split2` **56 PASS, 0 PASS-FP, 0 FAIL, 0
  DIVERGED**; combat `c-split2` (from frame 1400) 34 PASS-FP, 2 DIVERGED; `c-split3`/`c-split4`, `g-split3`,
  `w-split2`, `p-split2`: 11 PASS, 40-47 PASS-FP, 0 FAIL, 6-8 DIVERGED. splito/splitr traces == plain on all
  frames in every run. **Every PASS-FP lists only pages 0x8E4000 and 0xB60000**, and every DIVERGED pair
  diverges on an input before the call: `gStreamSemaphore` 0x8E4004 (plain 0xE8, rust 0xE4) and
  `gStreamFileHandles` 0x8E4010 (0xD0 / 0xCC), i.e. OS handle values that two processes number differently
  (the same list was clean in `g-split2` an hour earlier), plus two pairs (`CWanted::ReportCrimeNow`,
  `RegisterCrime_Immediately`) whose `CVector*` argument points into the Rust caller's stack frame (0x36EECC vs
  0x36EE48). Both are normaliser gaps for the harness owner (proposal: an OS-handle class for 0x8E4004..
  0x8E408F and the audio page 0xB60000, and the Stack class for pointer arguments below a Rust caller), not
  port differences: no pair whose inputs matched had a mismatch. Rows whose only split evidence is PASS-FP /
  DIVERGED are `implemented`.
- **Crashes found and fixed during verification**: (1) with every port installed in original mode the first
  weapon-combat recordings hung / crashed at frame ~1650 (`page fault ... at 0x401E4C`, winedbg suspended the
  process): the INF-20 protection read of `CColStore::LoadCollision`'s patched entry (0x401E46). The four
  protected entries were unregistered (Conventions). (2) `CPed::DeleteRwObject` in plain `l2` mode crashed
  (`page fault on read access to 00000064 at 0x7C755A`, the RW object destroyed twice by the double run): moved
  to the split list.
- `cargo test --release -p sa-weapon -p sa-ped` (3 + 3 pass); `cargo test --release -p sa-hook --lib --
  every_hook_installs_on_the_exe matches_catalogue_and_exe callers_reading_ecx_edx_after_a_hook_are_declared
  port_hooks_follow_the_local_ones` (4 pass); clippy `-D warnings`: sa-weapon, sa-ped, sa-world, sa-stream,
  sa-oracle, sa-core host (all targets), sa-weapon, sa-ped, sa-world, sa-stream (`--features in-process`) and
  sa-hook i686, sa-weapon, sa-ped, sa-world wasm32; `cargo build --release --target i686-pc-windows-gnu -p
  sa-hook -p sa-launcher` OK.

## Pass 2 (PORT-WEAPON-2, 2026-11-11)

Scope: the WEAPON functions pass 1 deferred, in SA:MP order (bullets, shots, explosions first). Agent + 2 helpers
(disjoint files: `weapon_fire.rs`/`weapon.rs`; `weapon_impact.rs`/`projectile.rs`/`explosion.rs`), all game runs on
lane 2 by the agent. Verdicts per function: `docs/arch/weapon-port-2-verdicts.tsv`.

New infrastructure:
- **`sa_core::orig::x87_unary(addr, st0) -> f64`** (twin of `x87_binary`) for the CRT's unary `_CI*` intrinsics
  (operand and result in ST0). The calling sequence lives in `sa_core::orig::raw::call_unary` (x86 and x86_64:
  `fld qword; call; fstp qword`, the output pointer kept on the stack across the call) so that a host test
  (`orig::tests::x87_unary_calling_sequence`) runs it against hand-written intrinsics: results, signed zero, NaN
  payload and a balanced FPU stack (status-word TOP before = after). Phases B/C panic like `x87_binary`.
- **0x821E70 is `_CIasin`, not acos**: its body computes `fpatan(x, sqrt((1+x)(1-x)))` and range-checks `|x| <= 1`
  (gta-reversed calls `asin`). It sets `fldcw (cw & 0x300) | 0x7F`, i.e. keeps the caller's precision control, so
  under the game's 24-bit precision its result is exact as a double. Its `|x| = 1` path returns the 80-bit
  constant pi/2; `x87_unary` rounds that to a double, and `pi/2 * 57.29578f` (the only use, `CBulletInfo::Update` /
  `DoBulletImpact`) rounds to the same f32 89.99999237 from both (checked with exact rationals).
- **Harness fix** (`tools/e2e/port-l2.sh`): `wine.log` is truncated before each run. The crash check greps it
  before the background launcher has truncated it, so a fault left by the previous run's process exit (seen on
  lane 2: `page fault ... at 7BCF0729` after `oracle: exiting`) ended the next run at once; the `rust` run of
  `bu-c`/`m1-c` produced no trace (`PASS rust reached the end frame: 0 frames`, a false PASS). For the harness owner.
- Split-run regions: section `PORT-WEAPON-2` of `tools/e2e/l2-extras.txt` (one block for all 13 hooks: the written
  parts of the audio .bss, collision caches, CReferences, bullet traces, point lights, permanent shadows, pads, stats,
  players, inform-group queue, g_fx/g_fxMan, pellet col model, explosion array, camera, anim statics). Declaring the
  whole audio block 0xB5F000..0xB6F000 made every pair DIVERGED (OS handles / heap pointers at 0xB606C4), so only
  the written ranges are declared. Hook list: `tools/e2e/lists/wp2-all.txt`.

Ported (14 functions, 13 hooks):

| function | file | notes (original kept; gta-reversed differs) |
|---|---|---|
| `CBulletInfo::Update` 0x7360D0 | bullet_shot.rs | expired bullet still moved/tested this frame; ONE hit-entity local for the whole loop (zeroed at entry, deactivates any later bullet that ProcessLineOfSight leaves it set for); only x/y bounds-checked; x step in extended precision; `IsSphereVisible(point)`; dead-ped anim flags `(f & 0xFFF7) | 1`; sniper `asin` via `x87_unary` |
| `CFireManager::Update` 0x53AF00 | fire.rs | riot block (raw rand formulas, ground/hit gates, `point - camera`, f64 0.2); strongest = first strictly greater than -1.0 (x87); shadow front `(1.2d, 0)` side `(0, -1.2d)`; colour `rand*(1/32767)*0.39999998+0.6`; corona offsets x/y extended, z rounded; dead `pos*ceil(strength)` accumulator not reproduced (stack-only) |
| `CFire::ProcessFire` 0x53A570 | fire.rs | strength `min3(s + ts*0.002)` stored, the OLD value written back when the integer parts differ; pools walked downwards; non-BMX StartFire with 0 generations; creeping-fire y gets the first rand, dir normalised, z + 2, `gens - 1`; merge writes `0.7*this + 0.3*other` into THIS fire, generations = signed MIN, burn `max(old, now+7000)`; FX const time `min(frac, (burn-now)/3500)`; removal test 2D |
| `CWeapon::Fire` 0x742300 | weapon_fire.rs | (helper) unhandled types count as fired when the low byte of `targetPosn` is non-zero; TakePhotograph gets the muzzle pointer; detonator always decrements; player reload quartered with 0xB7CEE5 |
| `CWeapon::FireSniper` 0x73AAC0 | weapon_fire.rs | (helper) GetWeaponInfo result discarded; counter 0x8D4B60 `(n+1)&7`; events built on the Rust stack through the original ctors/dtors |
| `CWeapon::FireM16_1stPerson` 0x741C00 | weapon_fire.rs | (helper) both gr deviations kept (`(rand() & 0x7F) - 64`); shake frequency dword's upper bytes from the reused slot |
| `CWeapon::FireInstantHitFromCar2` 0x73CBA0 | weapon_fire.rs | (helper) ReportCrime against the player whoever fires; silenced flag pushed as 0/1 (original: CL with stale upper ECX bytes; the ctor reads the byte) |
| `CWeapon::FireInstantHitFromCar` 0x73EC40 | weapon_fire.rs | (helper) DoDriveByAutoAiming always gets the player as owner; six rand() sites attributed |
| `CWeapon::CheckForShootingVehicleOccupant` 0x73F480 | weapon_fire.rs | (helper) ported, **not registered**: INF-20 reader `mov ecx, [0x73F482]` at 0x1567062 (.HOODLUM, followed by `rol edi, cl`); `catalog/code-data-reads.csv` has it with chunk = 0 (sweep decode) and sa-hook's `readers_match_the_exe_and_the_table` accepts keycells readers only with chunk = 1 |
| `CWeapon::GenerateDamageEvent` 0x73A530 | weapon_impact.rs | (helper) `GetWeaponInfo(type,1)->m_nWeaponFire` for every type; BlendAnimation result not null-checked; anim id 0xC0 gives false |
| `CWeapon::DoBulletImpact` 0x73B550 | weapon_impact.rs | (helper) `#else` stats condition, firedBy dereferenced without null check; objects with damage effect < 200 get the push AND `ObjectDamage(50)`; pad shake from three FindPlayerPed calls |
| `CExplosion::AddExplosion` 0x736A50 | explosion.rs | (helper) free-slot scan reads one element past the array when full; weak-rocket fields set for invisible explosions; mine ground probe discarded; molotov writes ground z back into its argument; 9 rand sites; jump tables transcribed from the EXE |
| `CProjectileInfo::RemoveProjectile` 0x7388F0 | projectile.rs | (helper) explosion type/owner per weapon type as the spec |
| `CProjectileInfo::Update` 0x738B20 | projectile.rs | (helper) heat-seeker turn boost keyed on the TARGET's speed (`CProjectileInfo.md` says missile; EBX = target at 0x73958B); reads the deleted object after RemoveProjectile and writes +0x1C into it on the molotov path (kept) |

Not ported in pass 2 (reason):
- `CWeapon::FireInstantHit` 0x73FB10: the early-exit path (ped firer with target, IK flag clear, forceAim 0) passes an
  uninitialised stack `dir` to `Fx_c::TriggerGunshot` and an uninitialised `CColPoint` to `DoBulletImpact`; needs a
  runtime measurement of those stack bytes at 0x740B71 (PLAN 4b), then the 1,700-instruction port.
- `CWeapon::FireAreaEffect` 0x73E800: `fptan` at 0x73E9CE; `sa_math::x87` has no `fptan` model (needs an `X87::ftan`
  checked by sa-oracle first).
- `CWeapon::FireProjectile` 0x741360, `CProjectileInfo::AddProjectile` 0x737C80 (entry jumps into protection code at
  0x401C3D: hookability to be checked), `CWeaponInfo::LoadWeaponData` 0x5BE670, the aiming helpers (DoDoomAiming,
  DoTankDoomAiming, DoDriveByAutoAiming, FindNearestTargetEntityWithScreenCoors), FireOneInstantHitRound,
  SetUpPelletCol, TakePhotograph, CWaterLevel 0x6E61B0: time box.

Verification (lane 2, `SA_ROOT=/home/ghosty2004/sa-engine`, `L2_MAX=150 SPLIT_PER_FRAME=2 L2_SPOOL_MAX=6G`,
`port-l2.sh @tools/e2e/lists/wp2-all.txt REC --runs "plain rust split" --tag ...`, outputs under the worktree's
`target/e2e/lane-2/port-l2-<tag>/`):

- **All-ports L3** (`port-l2.sh ports REC --runs "plain rust"`, every engine-crate port on Rust, 2,242 hooks installed,
  tags `p-*`): **IDENTICAL on every frame and all 17 subsystems** on weapon-combat (3500), c-bug (3220), driving
  (4338), world-cheats (3276), pause-restart (3700), anim-input (3300) and the grove walk (5200). The c-bug
  sequence (fire, crouch, fire) stays frame-exact with the whole Fire family on Rust.
- **Pass 2 list on Rust only** (`--runs "plain rust split"`, 13 hooks): rust trace IDENTICAL on weapon-combat
  (`w5-c`), c-bug (`w5-cb`), driving (`w5-d`), grove (`w5-g`), world-cheats (`w5-w`); splito/splitr traces == plain
  on every run.
- **Split-run L2** (`w5-c`, `w5-cb`, `w6-d` (driving, after the last region additions), `w5-g`, `w5-w`):
  0 mismatches in every compared pair, 0 undeclared writes in the final runs. Per hook (cases compared / recorded):
  CBulletInfo::Update PASS 150/150 on all five (the corpus has no active bullet: only the empty-slot path is
  exercised); CFireManager::Update PASS 150/150 on all five; CProjectileInfo::Update PASS 150/150 on all five;
  CWeapon::Fire PASS 135 (combat), 147 (c-bug), 45 (driving); CFire::ProcessFire PASS 150/150 (driving);
  CExplosion::AddExplosion PASS 1/1 (combat), 2/2 (driving); CWeapon::DoBulletImpact PASS-FP 134 / 147 / 45 (the
  CReferences pages differ before the call: reference slots on the stack of a Rust caller, `+0x110` stack shift);
  CWeapon::GenerateDamageEvent DIVERGED (3 of 11 compared on combat, 0 of 38 / 26 on c-bug / driving): the input
  difference is always in CReferences (0xB9C4FC plain 0x0036F06C rust 0x0036EF5C: the address of a `CEntity*` slot
  on the stack of the Rust DoBulletImpact, registered as a reference). No pair whose inputs matched had a mismatch.
  Harness proposal: a Stack normaliser class for CReferences `m_ppEntity` values that point below a Rust caller.
  No call in the corpus (implemented-needs-L2): FireSniper, FireM16_1stPerson, FireInstantHitFromCar(2) (no drive-by
  or sniper shot in the recordings; the weapon-combat sniper segment fires through FireInstantHit), RemoveProjectile.
- Earlier runs, for the record: `bu-c`/`m1-c` (first two ports; their `rust` runs were cut by the wine.log race,
  see above), `w2-c` (undeclared CWeapon::Fire writes, which led to the region block), `w3-c` (whole audio block
  declared: 6 DIVERGED), `w4-c` (explosion/camera/anim/bullet-trace writes).
- `cargo test --release -p sa-weapon -p sa-core` green (incl. `orig::tests::x87_unary_calling_sequence`);
  `cargo test --release -p sa-hook --lib` 45/45 (install, catalogue, callers, port_hooks, keycells, data_reads);
  clippy `-D warnings`: sa-weapon, sa-core, sa-oracle host (all targets), sa-hook + sa-weapon + sa-core i686
  (`--features sa-weapon/in-process`), sa-weapon + sa-core wasm32.

Ledger proposals (pass 2, pending):
- `CWeapon::FireInstantHitFromCar2`: the silenced flag is pushed as 0/1; the original pushes ECX with only CL set
  (upper bytes left by ReportCrime). Invisible if CEventGunShot's constructor reads only the byte (to confirm).
- `CExplosion::AddExplosion`: the StartFire bool word's upper bytes (ECX after `_ftol`) are not reproduced.
- `GenerateDamageEvent`: upper EAX bits not reproduced (callers read AL).
- Stack-only (argument-slot temporaries, SEH frames) in FireSniper, FireInstantHitFromCar(2), FireM16_1stPerson,
  DoBulletImpact, CBulletInfo::Update, CFireManager::Update (dead accumulator), CFire::ProcessFire.

## Ledger proposals

*pending*
- WEAPON: `EvaluateTargetForHeatSeekingMissile`, `PickTargetForHeatSeekingMissile`, `CWeapon::Update` (reload
  state) and `CCreepingFire::TryToStartFireAtCoors` write intermediate values into their own stack argument
  slots in the original; the ports do not (stack-only, invisible to callers; L2 compares argument words, so a
  case could show it).
- WEAPON: `CWeaponEffects::IsLockedOn` returns `_ftol`'s EAX only (EDX not reproduced; callers read AL).
- PED: `CPlayerInfo::EvaluateCarPosition` takes `GetATanOfXY`'s ST0 through f64 before a 24-bit `fsubr`
  (double rounding possible; same root as the `GetATanOfXY` ledger entry); `CPed::GetLocalDirection` uses the
  f64 FPATAN model (as PHYS `ApplySpeed`).

*original UB, not reproducible*
- `CBulletInfo::AddBullet` / `CShotInfo::AddShot` read the active flag one element past the array when every
  slot is in use (kept); `CPlayerPed::GetWantedLevel` / `Busted` dereference null without player data (kept);
  the SEH frame of `CPlayerPedData::AllocateData` is not built.

*harness (not ledger; for the harness slot)*
- INF-20: four registered hooks unregistered here (0x407AD0, 0x410860, 0x4C4570 known; 0x54D690 new) and two
  WEAPON/PED entries left unregistered (0x60A530 `CPlayerPed::ResetSprintEnergy`, read at 0x4037BC); the
  scanner should become part of sa-hook's install test.
- Split-run normaliser: OS handles at 0x8E4004 (`gStreamSemaphore`) / 0x8E4010.. (`gStreamFileHandles`) and
  page 0xB60000; stack-pointer arguments from a Rust caller.
- Pre-existing on master (not touched here): `cargo test -p sa-oracle --test l1` fails on the PHYS spec
  `PhysApplyMoveForceVec` (`CPhysical::ApplyMoveForce-vec`, 0x5429F0) when the native backend decides the
  signalling-NaN cases: 82 of ~20k SNaN cases differ in the NaN payload of the stored speed
  (`sa-oracle l1 PhysApplyMoveForceVec --cases 20000` with Wine available: e.g. orig 0x7FC00001, rust
  0x7FC00000). The PHYS evidence was taken with the native backend off.

## Functions (all 530 WEAPON and PED rows)

| address | name | sub | conv | port | state | evidence | deviations kept / notes |
|---|---|---|---|---|---|---|---|
| 0x41BCB0 | CBridge::ThisIsABridgeObjectMovingUp | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x41BE60 | CPlayerPed::GetWantedLevel | PED | thiscall | sa_ped::player_ped::get_wanted_level | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) | no player data: reads address 0x2C (crash) as the original |
| 0x43AB90 | CPed::PedIsInvolvedInConversation | PED | thiscall | sa_ped::ped::ped_is_involved_in_conversation | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x43ABA0 | CPed::PedIsReadyForConversation | PED | — | — | deferred |  | protection chunk computes a call target/pointer at run time (xchg/xor obfuscation); needs a runtime measurement first (PLAN 4b) |
| 0x455560 | CPed::PedCanPickUpPickUp | PED | cdecl | sa_ped::ped::ped_can_pick_up_pick_up | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x4590F0 | CPed::CreateDeadPedMoney | PED | — | — | deferred |  | protection chunk computes a call target/pointer at run time (xchg/xor obfuscation); needs a runtime measurement first (PLAN 4b) |
| 0x459180 | CPed::CreateDeadPedPickupCoors- | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x4591D0 | CPed::CreateDeadPedWeaponPickups | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x45B110 | CPlayerInfo::~CPlayerInfo | PED | thiscall | sa_ped::misc::player_info_destructor | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x479D10 | CPed::IsWearingGoggles | PED | thiscall | sa_ped::ped::is_wearing_goggles | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x481090 | CPed::SetStayInSamePlace | PED | thiscall | sa_ped::ped::set_stay_in_same_place | L1 | L1 1002000 cases, 0 mismatches |  |
| 0x4AD3F0 | CPedDamageResponseCalculator::Constructor | PED | thiscall | sa_ped::damage::constructor | L1 | L1 1000058 cases, 0 mismatches (INCOMPLETE only by SNaN float-argument cases); L2 combat:13/13 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared) |  |
| 0x4AD420 | CPedDamageResponseCalculator::~CPedDamageResponseCalculator | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x4AD430 | CPedDamageResponseCalculator::AccountForPedDamageStats | PED | thiscall | sa_ped::damage::account_for_ped_damage_stats | L2 | L2 combat:13/13 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared) |  |
| 0x4AD550 | CPedDamageResponseCalculator::AccountForPedArmour | PED | thiscall | sa_ped::damage::account_for_ped_armour | L2 | L2 combat:13/13 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared) |  |
| 0x4AD610 | CPedDamageResponseCalculator::ComputeWillForceDeath | PED | thiscall | sa_ped::damage::compute_will_force_death | L2 | L2 combat:13/13 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared) |  |
| 0x4B1370 | — | PED | thiscall | sa_ped::small::ctor_85b4c8 | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | constructor of a vtable-0x85B4C8 object (impl-1) |
| 0x4B3210 | CPedDamageResponseCalculator::ComputeWillKillPed | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x4B5AC0 | CPedDamageResponseCalculator::ComputeDamageResponse | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x4E6AA0 | CAEPedWeaponAudioEntity::Initialise | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x4E6AE0 | CAEPedWeaponAudioEntity::Service | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x531FC0 | CCrime::FindImmediateDetectionRange | PED | cdecl | sa_ped::wanted::find_immediate_detection_range | L1 | L1 1002000 cases, 0 mismatches; L2 combat:43/43 (0 mismatches, 0 undeclared) | preserve ecx (CCrime::ReportCrime keeps the position pointer in ECX) |
| 0x532010 | CCrime::ReportCrime | PED | cdecl | sa_ped::wanted::report_crime | implemented | split-combat: PASS-FP 45/45 compared, 0 mismatches | type-2 path dereferences the victim without null check (original) |
| 0x538B30 | CFire::Initialise | WEAPON | thiscall | sa_weapon::fire::initialise | L1 | L1 1,002,000 cases, 0 mismatches; L2 combat:60/60 (0 mismatches, 0 undeclared); L2 grove:60/60 (0 mismatches, 0 undeclared) | preserve = "ecx" (CFire::Constructor returns ECX after the call) |
| 0x538BB0 | CFireManager::Destructor | WEAPON | thiscall | sa_weapon::fire::manager_destructor | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x538BC0 | CFireManager::Init | WEAPON | thiscall | sa_weapon::fire::manager_init | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x538F10 | CFireManager::GetNumOfNonScriptFires | WEAPON | thiscall | sa_weapon::fire::get_num_of_non_script_fires | L2 | L2 combat:23/23 (0 mismatches, 0 undeclared); L2 grove:19/19 (0 mismatches, 0 undeclared); L2 cheats:23/23 (0 mismatches, 0 undeclared); L2 pr:20/20 (0 mismatches, 0 undeclared) |  |
| 0x538F40 | CFireManager::FindNearestFire | WEAPON | thiscall | sa_weapon::fire::find_nearest_fire | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x539340 | CFireManager::PlentyFiresAvailable | WEAPON | thiscall | sa_weapon::fire::plenty_fires_available | ported-unhookable |  | ported-unhookable: a loop branches back into the 5 patched bytes (0x539344) |
| 0x539360 | CFire::CreateFxSysForStrength | WEAPON | thiscall | sa_weapon::fire::create_fx_sys_for_strength | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5393F0 | CFire::Extinguish | WEAPON | thiscall | sa_weapon::fire::extinguish | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x539450 | CFireManager::ExtinguishPoint | WEAPON | thiscall | sa_weapon::fire::extinguish_point | L2 | split-combat: PASS-FP 5/5 compared, 0 mismatches; split-grove: PASS 2/2 compared, 0 mismatches; split-cheats: PASS-FP 5/5 compared, 0 mismatches; split-pr: PASS-FP 4/4 compared, 0 mismatches |  |
| 0x5394C0 | CFireManager::ExtinguishPointWithWater | WEAPON | thiscall | sa_weapon::fire::extinguish_point_with_water | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | rand() attributed to 0x539559/0x53957E/0x5395A3 |
| 0x5396E0 | CFireManager::IsScriptFireExtinguished | WEAPON | thiscall | sa_weapon::fire::is_script_fire_extinguished | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x539700 | CFireManager::RemoveScriptFire | WEAPON | thiscall | sa_weapon::fire::remove_script_fire | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x539720 | CFireManager::RemoveAllScriptFires | WEAPON | thiscall | sa_weapon::fire::remove_all_script_fires | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5397A0 | CFireManager::ClearAllScriptFireFlags | WEAPON | thiscall | sa_weapon::fire::clear_all_script_fire_flags | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:1/1 (0 mismatches, 0 undeclared) |  |
| 0x5397B0 | CFireManager::SetScriptFireAudio | WEAPON | thiscall | sa_weapon::fire::set_script_fire_audio | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5397E0 | CFireManager::GetScriptFireCoords | WEAPON | thiscall | sa_weapon::fire::get_script_fire_coords | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5397F0 | CFireManager::GetNumFiresInRange | WEAPON | thiscall | sa_weapon::fire::get_num_fires_in_range | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x539860 | CFireManager::GetNumFiresInArea | WEAPON | thiscall | sa_weapon::fire::get_num_fires_in_area | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x539CC0 | CCreepingFire::SetReadyToBurn | WEAPON | cdecl | sa_weapon::fire::set_ready_to_burn | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x539CE0 | CCreepingFire::Update | WEAPON | cdecl | sa_weapon::fire::creeping_fire_update | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x539D10 | CFireManager::DestroyAllFxSystems | WEAPON | thiscall | sa_weapon::fire::destroy_all_fx_systems | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x539D50 | CFireManager::CreateAllFxSystems | WEAPON | thiscall | sa_weapon::fire::create_all_fx_systems | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x539D90 | CFire::Constructor | WEAPON | thiscall | sa_weapon::fire::constructor | L2 | L2 combat:60/60 (0 mismatches, 0 undeclared); L2 grove:60/60 (0 mismatches, 0 undeclared); L2 cheats:60/60 (0 mismatches, 0 undeclared); L2 pr:60/60 (0 mismatches, 0 undeclared) |  |
| 0x539DA0 | CFireManager::Constructor | WEAPON | thiscall | sa_weapon::fire::manager_constructor | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x539DD0 | CFireManager::Shutdown | WEAPON | thiscall | sa_weapon::fire::manager_shutdown | implemented | split-pr: PASS-FP 1/1 compared, 0 mismatches |  |
| 0x539E50 | CFireManager::GetNextFreeFire | WEAPON | thiscall | sa_weapon::fire::get_next_free_fire | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x539F00 | CFireManager::StartFire-NoTarget | WEAPON | thiscall | sa_weapon::fire::start_fire_no_target | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | rand() attributed to 0x539F8C |
| 0x53A050 | CFireManager::StartFire- | WEAPON | thiscall | sa_weapon::fire::start_fire_target | implemented | split-combat: PASS-FP 1/1 compared, 0 mismatches |  |
| 0x53A270 | CFireManager::StartScriptFire | WEAPON | thiscall | sa_weapon::fire::start_script_fire | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x53A450 | CCreepingFire::TryToStartFireAtCoors | WEAPON | cdecl | sa_weapon::fire::try_to_start_fire_at_coors | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | ProcessVerticalLine writes the hit entity into the caller argument slot of x (not reproduced; L2 argument words may differ) |
| 0x53A570 | CFire::ProcessFire | WEAPON | thiscall | sa_weapon::fire::process_fire | L2 | split-combat: no call; split-c-bug: no call; split-driving: PASS 150/150 compared, 0 mismatches; split-grove: no call; split-cheats: no call | pass 2 |
| 0x53AF00 | CFireManager::Update | WEAPON | thiscall | sa_weapon::fire::manager_update | L2 | split-combat: PASS 150/150 compared, 0 mismatches; split-c-bug: PASS 150/150 compared, 0 mismatches; split-driving: PASS 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS 150/150 compared, 0 mismatches | pass 2 |
| 0x53EE60 | CPad::IsDPadUpPressed | PED | thiscall | sa_ped::small::is_dpad_up_pressed | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | impl-1 |
| 0x5533B0 | CPed::RemoveLighting | PED | thiscall | sa_ped::ped::remove_lighting | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 150/150 compared, 0 mismatches |  |
| 0x553F00 | CPed::SetupLighting | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x561C70 | CWanted::InitialiseStaticVariables | PED | cdecl | sa_ped::wanted::initialise_static_variables | L2 | L2 pr:1/1 (0 mismatches, 0 undeclared) |  |
| 0x561C90 | CWanted::UpdateWantedLevel | PED | thiscall | sa_ped::wanted::update_wanted_level | L2 | L2 combat:104/104 (0 mismatches, 0 undeclared); L2 grove:190/190 (0 mismatches, 0 undeclared); L2 cheats:120/120 (0 mismatches, 0 undeclared); L2 pr:115/115 (0 mismatches, 0 undeclared) |  |
| 0x561E70 | CWanted::SetMaximumWantedLevel | PED | cdecl | sa_ped::wanted::set_maximum_wanted_level | L1 | L1 1002000 cases, 0 mismatches; L2 combat:2/2 (0 mismatches, 0 undeclared); L2 grove:2/2 (0 mismatches, 0 undeclared); L2 cheats:2/2 (0 mismatches, 0 undeclared); L2 pr:4/4 (0 mismatches, 0 undeclared) | preserve ecx (CheatWantedLevel pushes ECX after the call) |
| 0x561F30 | CWanted::AreMiamiViceRequired | PED | thiscall | sa_ped::wanted::are_miami_vice_required | L1 | L1 1002000 cases, 0 mismatches |  |
| 0x561F40 | CWanted::AreSwatRequired | PED | thiscall | sa_ped::wanted::are_swat_required | L1 | L1 1002000 cases, 0 mismatches; L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x561F60 | CWanted::AreFbiRequired | PED | thiscall | sa_ped::wanted::are_fbi_required | L1 | L1 1002000 cases, 0 mismatches; L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x561F80 | CWanted::AreArmyRequired | PED | thiscall | sa_ped::wanted::are_army_required | L1 | L1 1002000 cases, 0 mismatches; L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x561FA0 | CWanted::NumOfHelisRequired | PED | thiscall | sa_ped::wanted::num_of_helis_required | L1 | L1 1002000 cases, 0 mismatches; L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x561FD0 | CWanted::ResetPolicePursuit | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x561FE0 | CWanted::ClearQdCrimes | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x562000 | CWanted::AddCrimeToQ | PED | thiscall | sa_ped::wanted::add_crime_to_q | L2 | L2 combat:41/41 (0 mismatches, 0 undeclared) |  |
| 0x562120 | CWanted::ReportCrimeNow | PED | thiscall | sa_ped::wanted::report_crime_now | implemented | split-combat: DIVERGED 0/4 compared, 0 mismatches |  |
| 0x562300 | CWanted::RemovePursuitCop-func | PED | cdecl | sa_ped::wanted::remove_pursuit_cop_func | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x562330 | CWanted::IsInPursuit | PED | thiscall | sa_ped::wanted::is_in_pursuit | L2 | L2 combat:630/630 (0 mismatches, 0 undeclared); L2 cheats:117/117 (0 mismatches, 0 undeclared) |  |
| 0x562360 | CWanted::UpdateEachFrame | PED | cdecl | sa_ped::wanted::update_each_frame | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x562390 | CWanted::Initialise | PED | thiscall | sa_ped::wanted::initialise | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x562400 | CWanted::Reset | PED | thiscall | sa_ped::wanted::reset | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared) |  |
| 0x562410 | CWanted::RegisterCrime | PED | thiscall | sa_ped::wanted::register_crime | L2 | L2 combat:2/2 (0 mismatches, 0 undeclared) |  |
| 0x562430 | CWanted::RegisterCrime_Immediately | PED | thiscall | sa_ped::wanted::register_crime_immediately | implemented | split-combat: DIVERGED 0/41 compared, 0 mismatches |  |
| 0x562470 | CWanted::SetWantedLevel | PED | thiscall | sa_ped::wanted::set_wanted_level | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x562540 | CWanted::CheatWantedLevel | PED | thiscall | sa_ped::wanted::cheat_wanted_level | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x562570 | CWanted::SetWantedLevelNoDrop | PED | thiscall | sa_ped::wanted::set_wanted_level_no_drop | L2 | L2 combat:41/41 (0 mismatches, 0 undeclared) |  |
| 0x5625A0 | CWanted::ClearWantedLevelAndGoOnParole | PED | thiscall | sa_ped::wanted::clear_wanted_level_and_go_on_parole | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5625F0 | CWanted::WorkOutPolicePresence | PED | cdecl | sa_ped::wanted::work_out_police_presence | L2 | L2 combat:933/933 (0 mismatches, 0 undeclared); L2 grove:280/280 (0 mismatches, 0 undeclared); L2 cheats:135/135 (0 mismatches, 0 undeclared); L2 pr:284/284 (0 mismatches, 0 undeclared) |  |
| 0x562760 | CWanted::UpdateCrimesQ | PED | thiscall | sa_ped::wanted::update_crimes_q | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 137/137 compared, 0 mismatches; split-pr: PASS-FP 150/150 compared, 0 mismatches |  |
| 0x5627D0 | CWanted::IsClosestCop | PED | thiscall | sa_ped::wanted::is_closest_cop | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | numCopsToCheck > 10 overruns the original stack arrays (not modelled) |
| 0x562B00 | CWanted::ComputePursuitCopToDisplace | PED | cdecl | sa_ped::wanted::compute_pursuit_cop_to_displace | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared) | a dead cop in the list is returned at once (original); threshold starts at 1.0 |
| 0x562C10 | CWanted::RemovePursuitCop-method | PED | thiscall | sa_ped::wanted::remove_pursuit_cop_method | L2 | L2 combat:7/7 (0 mismatches, 0 undeclared); L2 grove:8/8 (0 mismatches, 0 undeclared); L2 cheats:24/24 (0 mismatches, 0 undeclared); L2 pr:1/1 (0 mismatches, 0 undeclared) |  |
| 0x562C40 | CWanted::RemoveExcessPursuitCops | PED | thiscall | sa_ped::wanted::remove_excess_pursuit_cops | L2 | L2 combat:486/486 (0 mismatches, 0 undeclared); L2 cheats:117/117 (0 mismatches, 0 undeclared) |  |
| 0x562C90 | CWanted::Update | PED | thiscall | sa_ped::wanted::update | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 150/150 compared, 0 mismatches | debug printf calls kept |
| 0x562F60 | CWanted::CanCopJoinPursuit-func | PED | cdecl | sa_ped::wanted::can_cop_join_pursuit_func | L2 | L2 combat:152/152 (0 mismatches, 0 undeclared) |  |
| 0x562FB0 | CWanted::CanCopJoinPursuit-method | PED | thiscall | sa_ped::wanted::can_cop_join_pursuit_method | L2 | L2 combat:149/149 (0 mismatches, 0 undeclared); L2 cheats:10/10 (0 mismatches, 0 undeclared) |  |
| 0x563060 | CWanted::SetPursuitCop | PED | thiscall | sa_ped::wanted::set_pursuit_cop | L2 | L2 combat:3/3 (0 mismatches, 0 undeclared); L2 cheats:10/10 (0 mismatches, 0 undeclared) |  |
| 0x56DA80 | CPlayerInfo::SetLastTargetVehicle | PED | thiscall | sa_ped::player_info::set_last_target_vehicle | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56DAB0 | CPlayerInfo::IsPlayerInRemoteMode | PED | thiscall | sa_ped::player_info::is_player_in_remote_mode | L1 | L1 1002000 cases, 0 mismatches |  |
| 0x56DAD0 | CPlayerInfo::EvaluateCarPosition | PED | thiscall | sa_ped::player_info::evaluate_car_position | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | GetATanOfXY ST0 result taken as f64 (53-bit) before the PC=24 fsubr: double rounding possible (ledger candidate, like GetATanOfXY) |
| 0x56DBD0 | CPlayerInfo::FindObjectToSteal | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x56DF50 | CPlayerInfo::GetSpeed_Hook | PED | thiscall | sa_ped::player_info::get_speed | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56DFB0 | CPlayerInfo::GetPos_Hook | PED | thiscall | sa_ped::player_info::get_pos | L2 | L2 combat:7/7 (0 mismatches, 0 undeclared); L2 grove:3/3 (0 mismatches, 0 undeclared); L2 cheats:7/7 (0 mismatches, 0 undeclared); L2 pr:6/6 (0 mismatches, 0 undeclared) |  |
| 0x56E550 | CPlayerInfo::IsRestartingAfterDeath | PED | thiscall | sa_ped::player_info::is_restarting_after_death | L1 | L1 1002000 cases, 0 mismatches; L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x56E560 | CPlayerInfo::IsRestartingAfterArrest | PED | thiscall | sa_ped::player_info::is_restarting_after_arrest | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x56E570 | CPlayerInfo::IsRestartingAfterMissionFailed | PED | thiscall | sa_ped::player_info::is_restarting_after_mission_failed | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56E580 | CPlayerInfo::KillPlayer | PED | thiscall | sa_ped::misc::kill_player | implemented | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches |  |
| 0x56E5D0 | CPlayerInfo::ArrestPlayer | PED | thiscall | sa_ped::misc::arrest_player | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56E610 | CPlayerInfo::WorkOutEnergyFromHunger | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x56E830 | CPlayerInfo::PlayerFailedCriticalMission | PED | thiscall | sa_ped::misc::player_failed_critical_mission | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56E860 | CPlayerInfo::CancelPlayerEnteringCars | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x56E870 | CPlayerInfo::MakePlayerSafe | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x56EA30 | CPlayerInfo::BlowUpRCBuggy | PED | thiscall | sa_ped::misc::blow_up_rc_buggy | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56EA80 | CPlayerInfo::DeletePlayerSkin | PED | thiscall | sa_ped::player_info::delete_player_skin | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56EAB0 | CPlayerInfo::AddHealth | PED | thiscall | sa_ped::player_info::add_health | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56EB30 | CPlayerInfo::StreamParachuteWeapon | PED | thiscall | sa_ped::player_info::stream_parachute_weapon | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56EC40 | CPlayerInfo::GivePlayerParachute | PED | thiscall | sa_ped::misc::give_player_parachute | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56EF90 | (unnamed) | WEAPON | — | — | deferred |  | CPlayerCrossHair::Render (proposed): RW sprite rendering with FSIN/FCOS (render pass) |
| 0x56F210 | CPlayerPedData::SetInitialState | PED | thiscall | sa_ped::player_info::set_initial_state | L2 | L2 combat:2/2 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:2/2 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x56F330 | CPlayerInfo::Clear | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x56F4E0 | CPlayerInfo::FindClosestCarSectorList | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x56F7D0 | CPlayerInfo::LoadPlayerSkin | PED | thiscall | sa_ped::player_info::load_player_skin | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 1/1 compared, 0 mismatches |  |
| 0x56F810 | CPlayerPedData::Constructor | PED | thiscall | sa_ped::player_info::ped_data_constructor | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56F890 | CPlayerPedData::DeAllocateData | PED | thiscall | sa_ped::misc::ped_data_deallocate | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x56F8D0 | CPlayerInfo::Process | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x5717F0 | CPlayerInfo::SetPlayerSkin | PED | thiscall | sa_ped::player_info::set_player_skin | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x571840 | CPlayerPedData::Destructor | PED | thiscall | sa_ped::misc::ped_data_destructor | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x571880 | CPlayerPedData::AllocateData | PED | thiscall | sa_ped::player_info::allocate_data | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches | SEH frame around the clothes constructor not built |
| 0x571920 | CPlayerInfo::Constructor | PED | thiscall | sa_ped::player_info::constructor | L2 | L2 combat:3/3 (0 mismatches, 0 undeclared); L2 grove:3/3 (0 mismatches, 0 undeclared); L2 cheats:3/3 (0 mismatches, 0 undeclared); L2 pr:3/3 (0 mismatches, 0 undeclared) |  |
| 0x5A40F0 | CProjectile::`scalar deleting destructor' | WEAPON | thiscall | sa_weapon::projectile::scalar_deleting_destructor | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5A78F0 | CPedClothesDesc::Initialise | PED | thiscall | sa_ped::clothes::desc_initialise | L1 | L1 1002000 cases, 0 mismatches; L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x5A7910 | CPedClothesDesc::SetModel-modelHash | PED | thiscall | sa_ped::clothes::set_model_hash | L1 | L1 1002000 cases, 0 mismatches; L2 combat:2/2 (0 mismatches, 0 undeclared); L2 grove:2/2 (0 mismatches, 0 undeclared); L2 cheats:2/2 (0 mismatches, 0 undeclared); L2 pr:4/4 (0 mismatches, 0 undeclared) |  |
| 0x5A7920 | CPedClothesDesc::SetModel-modelName | PED | thiscall | sa_ped::clothes::set_model_name | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x5A7950 | CPedClothesDesc::GetIsWearingBalaclava | PED | — | — | deferred |  | protection chunk computes a call target/pointer at run time (xchg/xor obfuscation); needs a runtime measurement first (PLAN 4b) |
| 0x5A7970 | CPedClothesDesc::HasVisibleNewHairCut | PED | thiscall | sa_ped::clothes::has_visible_new_hair_cut | L2 | L2 combat:121/121 (0 mismatches, 0 undeclared) |  |
| 0x5A79D0 | CPedClothesDesc::HasVisibleTattoo | PED | thiscall | sa_ped::clothes::has_visible_tattoo | L1 | L1 1002000 cases, 0 mismatches |  |
| 0x5A7A20 | GetClothesModelFromName | PED | cdecl | sa_ped::clothes::get_clothes_model_from_name | L1 | L1 1002000 cases, 0 mismatches; L2 combat:99/99 (0 mismatches, 0 undeclared); L2 grove:99/99 (0 mismatches, 0 undeclared); L2 cheats:99/99 (0 mismatches, 0 undeclared); L2 pr:99/99 (0 mismatches, 0 undeclared) | unknown names return 0 (torso), the original setne/dec/and 9 |
| 0x5A7B30 | CClothes::LoadClothesFile | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5A7EA0 | CClothes::GetTextureDependency | PED | cdecl | sa_ped::clothes::get_texture_dependency | L1 | L1 1002000 cases, 0 mismatches; L2 combat:18/18 (0 mismatches, 0 undeclared); L2 grove:18/18 (0 mismatches, 0 undeclared); L2 cheats:18/18 (0 mismatches, 0 undeclared); L2 pr:18/18 (0 mismatches, 0 undeclared) | preserve ecx,edx (CClothes::Init, SetTextureAndModel) |
| 0x5A7F30 | CClothes::GetDependentTexture | PED | cdecl | sa_ped::clothes::get_dependent_texture | L1 | L1 1002000 cases, 0 mismatches |  |
| 0x5A7FB0 | CClothes::GetPlayerMotionGroupToLoad | PED | cdecl | sa_ped::clothes::get_player_motion_group_to_load | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5A8020 | CPedClothesDesc::Constructor | PED | thiscall | sa_ped::clothes::desc_constructor | L2 | L2 combat:4/4 (0 mismatches, 0 undeclared); L2 grove:4/4 (0 mismatches, 0 undeclared); L2 cheats:4/4 (0 mismatches, 0 undeclared); L2 pr:6/6 (0 mismatches, 0 undeclared) |  |
| 0x5A8050 | CPedClothesDesc::SetTextureAndModel-hash | PED | thiscall | sa_ped::clothes::set_texture_and_model_hash | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5A8080 | CPedClothesDesc::SetTextureAndModel-name | PED | thiscall | sa_ped::clothes::set_texture_and_model_name | L2 | L2 combat:4/4 (0 mismatches, 0 undeclared); L2 grove:4/4 (0 mismatches, 0 undeclared); L2 cheats:4/4 (0 mismatches, 0 undeclared); L2 pr:8/8 (0 mismatches, 0 undeclared) |  |
| 0x5A80D0 | CClothes::Init | PED | cdecl | sa_ped::clothes::init | L2 | split-combat: DIVERGED 0/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: DIVERGED 0/1 compared, 0 mismatches; split-pr: DIVERGED 0/1 compared, 0 mismatches | tail-calls LoadClothesFile (file IO) |
| 0x5A8120 | CClothes::RequestMotionGroupAnims | PED | cdecl | sa_ped::clothes::request_motion_group_anims | L2 | split-combat: PASS-FP 2/2 compared, 0 mismatches; split-grove: PASS 2/2 compared, 0 mismatches; split-cheats: PASS-FP 2/2 compared, 0 mismatches; split-pr: PASS-FP 3/3 compared, 0 mismatches |  |
| 0x5A81B0 | CClothes::GetDefaultPlayerMotionGroup | PED | — | — | deferred |  | protection chunk computes a call target/pointer at run time (xchg/xor obfuscation); needs a runtime measurement first (PLAN 4b) |
| 0x5A81E0 | CClothes::ConstructPedModel | PED | cdecl | sa_ped::clothes::construct_ped_model | L2 | split-combat: DIVERGED 0/2 compared, 0 mismatches; split-grove: PASS 2/2 compared, 0 mismatches; split-cheats: DIVERGED 0/2 compared, 0 mismatches; split-pr: DIVERGED 0/4 compared, 0 mismatches |  |
| 0x5A8270 | CClothes::RebuildCutscenePlayer | PED | cdecl | sa_ped::clothes::rebuild_cutscene_player | L2 | split-combat: DIVERGED 0/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: DIVERGED 0/1 compared, 0 mismatches; split-pr: DIVERGED 0/2 compared, 0 mismatches |  |
| 0x5A82C0 | CClothes::RebuildPlayer | PED | cdecl | sa_ped::clothes::rebuild_player | L2 | split-combat: DIVERGED 0/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: DIVERGED 0/1 compared, 0 mismatches; split-pr: DIVERGED 0/2 compared, 0 mismatches |  |
| 0x5A8390 | CClothes::RebuildPlayerIfNeeded | PED | cdecl | sa_ped::clothes::rebuild_player_if_needed | L2 | split-combat: DIVERGED 0/2 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: DIVERGED 0/2 compared, 0 mismatches; split-pr: DIVERGED 0/2 compared, 0 mismatches |  |
| 0x5BB890 | CPedStats::LoadPedStats | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5BCF30 | CWeaponInfo::FindWeaponFireType | WEAPON | cdecl | sa_weapon::weapon_info::find_weapon_fire_type | L1 | L1 1,002,000 cases, 0 mismatches; L2 combat:70/70 (0 mismatches, 0 undeclared); L2 grove:70/70 (0 mismatches, 0 undeclared); L2 cheats:70/70 (0 mismatches, 0 undeclared); L2 pr:140/140 (0 mismatches, 0 undeclared) |  |
| 0x5BE670 | CWeaponInfo::LoadWeaponData | WEAPON | — | — | deferred |  | CWeaponInfo::LoadWeaponData: 495-insn weapon.dat parser (CFileMgr, sscanf, model lookups); load-time, needs the loaders pass |
| 0x5BF750 | CWeaponInfo::Initialise | WEAPON | cdecl | sa_weapon::weapon_info::initialise | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches |  |
| 0x5BF9D0 | CPedStats::Initialise | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5D24B0 | — | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5D2560 | — | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5D3AC0 | CPlayerInfo::Save | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5D3B00 | CPlayerInfo::Load | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5D3CD0 | CPedType::Save | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5D3D10 | CPedType::Load | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5D4640 | CPed::Load | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5D46E0 | CPlayerPed::Load | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5D5730 | CPed::Save | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5D57E0 | CPlayerPed::Save | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x5DDB70 | CCivilianPed::Constructor | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5DDBE0 | CCivilianPed::~CCivilianPed | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5DDBF0 | CCivilianPed::ProcessControl | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5DDC30 | CCivilianPed::Destructor | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5DDC60 | CCopPed::Constructor | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5DDE80 | CCopPed::SetPartner | PED | thiscall | sa_ped::misc::set_partner | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DDEB0 | CCopPed::AddCriminalToKill | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5DE040 | CCopPed::RemoveCriminalToKill | PED | thiscall | sa_ped::misc::remove_criminal_to_kill | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DE070 | CCopPed::ClearCriminalsToKill | PED | thiscall | sa_ped::misc::clear_criminals_to_kill | L2 | split-combat: PASS-FP 4/4 compared, 0 mismatches; split-grove: PASS 8/8 compared, 0 mismatches; split-cheats: PASS-FP 13/13 compared, 0 mismatches; split-pr: PASS-FP 3/3 compared, 0 mismatches |  |
| 0x5DE0D0 | CCopPed::Destructor | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5DE160 | CCopPed::ProcessControl | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5DE2C0 | CCopPed::`scalar deleting destructor' | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5DE3B0 | CEmergencyPed::~CEmergencyPed | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5DE3E0 | CEmergencyPed::`scalar deleting destructor' | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5DE400 | CEmergencyPed::ProcessControl | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5DEBB0 | CPed::Initialise | PED | cdecl | sa_ped::small::initialise | implemented | split-combat: DIVERGED 0/1 compared, 0 mismatches; split-grove: DIVERGED 0/1 compared, 0 mismatches; split-cheats: DIVERGED 0/1 compared, 0 mismatches; split-pr: DIVERGED 0/2 compared, 0 mismatches | CPedType::Initialise then tail jump to 0x64F860 (impl-1) |
| 0x5DEBC0 | CPed::SetPedStats | PED | thiscall | sa_ped::ped::set_ped_stats | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DEBE0 | CPed::Update | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5DEBF0 | CPed::DeleteRwObject | PED | thiscall | sa_ped::small::delete_rw_object | implemented | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS-FP 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches | forwarding jmp to CEntity::DeleteRwObject (impl-1) |
| 0x5DEC00 | CPed::SetMoveState | PED | thiscall | sa_ped::ped::set_move_state | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5DEC10 | CPed::SetMoveAnimSpeed | PED | thiscall | sa_ped::ped::set_move_anim_speed | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5DED10 | CPed::StopNonPartialAnims | PED | thiscall | sa_ped::ped::stop_non_partial_anims | L2 | L2 combat:50/50 (0 mismatches, 0 undeclared); L2 grove:136/136 (0 mismatches, 0 undeclared); L2 cheats:40/40 (0 mismatches, 0 undeclared); L2 pr:48/48 (0 mismatches, 0 undeclared) |  |
| 0x5DED50 | CPed::RestartNonPartialAnims | PED | thiscall | sa_ped::ped::restart_non_partial_anims | L2 | L2 combat:7/7 (0 mismatches, 0 undeclared); L2 grove:2/2 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared) |  |
| 0x5DED90 | CPed::CanUseTorsoWhenLooking | PED | thiscall | sa_ped::ped::can_use_torso_when_looking | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DEDC0 | CPed::SetLookFlag-Heading | PED | thiscall | sa_ped::ped::set_look_flag_heading | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DEE40 | CPed::SetLookFlag-Entity | PED | thiscall | sa_ped::ped::set_look_flag_entity | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DEED0 | CPed::SetAimFlag-Entity | PED | thiscall | sa_ped::ped::set_aim_flag_entity | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DEF20 | CPed::ClearAimFlag | PED | thiscall | sa_ped::ped::clear_aim_flag | L2 | L2 combat:216/216 (0 mismatches, 0 undeclared); L2 grove:246/246 (0 mismatches, 0 undeclared); L2 cheats:126/126 (0 mismatches, 0 undeclared); L2 pr:64/64 (0 mismatches, 0 undeclared) |  |
| 0x5DEF60 | CPed::GetLocalDirection | PED | thiscall | sa_ped::ped::get_local_direction | L2 | L2 combat:7/7 (0 mismatches, 0 undeclared) | FPATAN via X87::fpatan (f64 model; L2 decides on the CPU) |
| 0x5DEFD0 | CPed::IsPedShootable | PED | thiscall | sa_ped::ped::is_ped_shootable | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5DEFE0 | CPed::UseGroundColModel | PED | thiscall | sa_ped::ped::use_ground_col_model | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DF000 | CPed::CanPedReturnToState | PED | thiscall | sa_ped::ped::can_ped_return_to_state | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DF030 | CPed::CanSetPedState | PED | thiscall | sa_ped::ped::can_set_ped_state | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DF060 | CPed::CanBeArrested | PED | thiscall | sa_ped::ped::can_be_arrested | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DF090 | CPed::CanStrafeOrMouseControl | PED | thiscall | sa_ped::ped::can_strafe_or_mouse_control | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DF100 | CPed::CanBeDeleted | PED | thiscall | sa_ped::ped::can_be_deleted | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5DF150 | CPed::CanBeDeletedEvenInVehicle | PED | thiscall | sa_ped::ped::can_be_deleted_even_in_vehicle | L1 | L1 1002000 cases, 0 mismatches |  |
| 0x5DF170 | CPed::RemoveGogglesModel | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5DF200 | CPed::GetWeaponSlot | PED | thiscall | sa_ped::ped::get_weapon_slot | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DF220 | CPed::GrantAmmo | PED | thiscall | sa_ped::ped::grant_ammo | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DF290 | CPed::SetAmmo | PED | thiscall | sa_ped::ped::set_ammo | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | SetAmmo: clip = min(clip, total) as the bytes (gr-dev); cap 99999 signed |
| 0x5DF300 | CPed::DoWeHaveWeaponAvailable | PED | thiscall | sa_ped::ped::do_we_have_weapon_available | L2 | L2 combat:969/969 (0 mismatches, 0 undeclared); L2 cheats:10/10 (0 mismatches, 0 undeclared) |  |
| 0x5DF340 | CPed::DoGunFlash | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5DF400 | CPed::SetGunFlashAlpha | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5DF4E0 | CPed::ResetGunFlashAlpha | PED | thiscall | sa_ped::ped::reset_gun_flash_alpha | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 34/34 compared, 0 mismatches |  |
| 0x5DF510 | CPed::GetBikeRidingSkill | PED | thiscall | sa_ped::ped::get_bike_riding_skill | L2 | L2 grove:642/642 (0 mismatches, 0 undeclared) |  |
| 0x5DF560 | CPed::ShoulderBoneRotation | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5DF8D0 | CPed::SetLookTimer | PED | thiscall | sa_ped::ped::set_look_timer | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5DF8F0 | CPed::IsPlayer | PED | thiscall | sa_ped::ped::is_player | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5DF910 | CPed::SetPedPositionInCar | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5DFD60 | CPed::RestoreHeadingRate | PED | thiscall | sa_ped::ped::restore_heading_rate | L2 | L2 combat:281/281 (0 mismatches, 0 undeclared); L2 grove:795/795 (0 mismatches, 0 undeclared); L2 cheats:602/602 (0 mismatches, 0 undeclared); L2 pr:198/198 (0 mismatches, 0 undeclared) |  |
| 0x5DFD70 | CPed::RestoreHeadingRateCB | PED | cdecl | sa_ped::ped::restore_heading_rate_cb | L2 | L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:1/1 (0 mismatches, 0 undeclared) |  |
| 0x5DFD90 | CPed::SetRadioStation | PED | thiscall | sa_ped::ped::set_radio_station | L2 | L2 combat:7/7 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared) |  |
| 0x5DFDF0 | CPed::PositionAttachedPed | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E00F0 | CPed::Undress | PED | thiscall | sa_ped::ped_weapon::undress | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E0130 | CPed::Dress | PED | thiscall | sa_ped::ped_weapon::dress | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches |  |
| 0x5E0170 | CPed::IsAlive | PED | thiscall | sa_ped::ped::is_alive | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5E01A0 | CPed::UpdateStatEnteringVehicle | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5E01B0 | CPed::UpdateStatLeavingVehicle | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5E01C0 | CPed::GetTransformedBonePosition | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E0270 | CPed::ReleaseCoverPoint | PED | thiscall | sa_ped::ped::release_cover_point | implemented | split-combat: PASS-FP 18/18 compared, 0 mismatches; split-cheats: PASS-FP 3/3 compared, 0 mismatches |  |
| 0x5E0290 | CPed::GetHoldingTask | PED | thiscall | sa_ped::ped::get_holding_task | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E02E0 | CPed::GetEntityThatThisPedIsHolding | PED | thiscall | sa_ped::ped::get_entity_that_this_ped_is_holding | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5E0360 | CPed::DropEntityThatThisPedIsHolding | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E0400 | CPed::CanThrowEntityThatThisPedIsHolding | PED | thiscall | sa_ped::ped::can_throw_entity_that_this_ped_is_holding | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E0460 | CPed::IsPlayingHandSignal | PED | thiscall | sa_ped::ped::is_playing_hand_signal | L2 | L2 combat:134/134 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:185/185 (0 mismatches, 0 undeclared) |  |
| 0x5E0480 | CPed::StopPlayingHandSignal | PED | thiscall | sa_ped::ped::stop_playing_hand_signal | L2 | split-combat: PASS-FP 22/22 compared, 0 mismatches; split-grove: PASS 17/17 compared, 0 mismatches; split-cheats: PASS-FP 14/14 compared, 0 mismatches; split-pr: PASS-FP 4/4 compared, 0 mismatches |  |
| 0x5E04B0 | CPed::GetWalkAnimSpeed | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E0540 | — | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5E06E0 | CPed::SetPedDefaultDecisionMaker | PED | thiscall | sa_ped::ped::set_ped_default_decision_maker | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E0730 | CPed::CanSeeEntity | PED | thiscall | sa_ped::ped::can_see_entity | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E0820 | CPed::PositionPedOutOfCollision | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x5E13C0 | CPed::PositionAnyPedOutOfCollision | PED | — | — | deferred |  | not ported in this pass: mid/large body (233 insns) calling unported AI/VEH/HUD code; next PED pass |
| 0x5E1660 | CPed::OurPedCanSeeThisEntity | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E17E0 | CPed::SortPeds | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5E1950 | CPed::ClearLookFlag | PED | thiscall | sa_ped::ped::clear_look_flag | L2 | L2 combat:216/216 (0 mismatches, 0 undeclared); L2 grove:246/246 (0 mismatches, 0 undeclared); L2 cheats:126/126 (0 mismatches, 0 undeclared); L2 pr:64/64 (0 mismatches, 0 undeclared) |  |
| 0x5E1A00 | CPed::WorkOutHeadingForMovingFirstPerson | PED | thiscall | sa_ped::ped::work_out_heading_for_moving_first_person | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E1B10 | CPed::UpdatePosition | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x5E1FA0 | CPed::ProcessBuoyancy | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x5E2530 | CPed::ProcessEntityCollision | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x5E3630 | — | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5E37C0 | — | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5E3960 | CPed::IsPedInControl | PED | thiscall | sa_ped::ped::is_ped_in_control | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5E3990 | CPed::RemoveWeaponModel | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E3A90 | CPed::AddGogglesModel | PED | thiscall | sa_ped::ped_weapon::add_goggles_model | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E3AE0 | CPed::PutOnGoggles | PED | thiscall | sa_ped::ped_weapon::put_on_goggles | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E3B60 | CPed::GetWeaponSkill-WeaponType | PED | thiscall | sa_ped::ped::get_weapon_skill_type | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) | stat read twice as the original |
| 0x5E3C10 | CPed::SetWeaponSkill | PED | thiscall | sa_ped::ped::set_weapon_skill | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E3C30 | CPed::SpecialEntityPreCollisionStuff | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E3E90 | CPed::SpecialEntityCalcCollisionSteps | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E3FF0 | CPed::ClearLook | PED | thiscall | sa_ped::ped::clear_look | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E4000 | CPed::TurnBody | PED | thiscall | sa_ped::ped::turn_body | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E4110 | CPed::Teleport | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E4220 | CPed::IsPointerValid | PED | — | — | deferred |  | protection chunk computes a call target/pointer at run time (xchg/xor obfuscation); needs a runtime measurement first (PLAN 4b) |
| 0x5E4280 | CPed::GetBonePosition-Original | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E4390 | CPed::GiveObjectToPedToHold | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E4500 | CPed::SetPedState | PED | thiscall | sa_ped::ped::set_ped_state | L2 | split-combat: PASS-FP 60/60 compared, 0 mismatches; split-grove: PASS 138/138 compared, 0 mismatches; split-cheats: PASS-FP 45/45 compared, 0 mismatches; split-pr: PASS-FP 49/49 compared, 0 mismatches |  |
| 0x5E4720 | CPed::operator new-anon | PED | — | — | deferred |  | already L3 (sa-hook port) |
| 0x5E4730 | CPed::operator new-poolIndexed | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5E4760 | CPed::operator delete-anon | PED | — | — | deferred |  | already L3 (sa-hook port) |
| 0x5E47E0 | CPed::SetCharCreatedBy | PED | thiscall | sa_ped::ped::set_char_created_by | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 4/4 compared, 0 mismatches; split-cheats: PASS-FP 3/3 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches |  |
| 0x5E4880 | CPed::SetModelIndex | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E4A00 | CPed::SetMoveAnim | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E4C50 | CPed::CalculateNewVelocity | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x5E52E0 | CPed::CalculateNewOrientation | PED | thiscall | sa_ped::ped::calculate_new_orientation | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E5320 | CPed::ClearAll | PED | thiscall | sa_ped::ped::clear_all | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared) |  |
| 0x5E5380 | CPed::DoFootLanded | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x5E57F0 | CPed::PlayFootSteps | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x5E5ED0 | CPed::AddWeaponModel | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E6010 | CPed::TakeOffGoggles | PED | thiscall | sa_ped::ped_weapon::take_off_goggles | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E6080 | CPed::GiveWeapon- | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E61F0 | CPed::SetCurrentWeapon-Slot | PED | thiscall | sa_ped::ped_weapon::set_current_weapon_slot | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 84/84 compared, 0 mismatches; split-pr: PASS-FP 57/57 compared, 0 mismatches |  |
| 0x5E6280 | CPed::SetCurrentWeapon-WepType | PED | thiscall | sa_ped::ped_weapon::set_current_weapon_type | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 10/10 compared, 0 mismatches |  |
| 0x5E62B0 | CPed::ClearWeapon | PED | thiscall | sa_ped::ped_weapon::clear_weapon | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E6320 | CPed::ClearWeapons | PED | thiscall | sa_ped::ped_weapon::clear_weapons | L2 | split-combat: PASS-FP 100/100 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 63/63 compared, 0 mismatches; split-pr: PASS-FP 40/40 compared, 0 mismatches |  |
| 0x5E6370 | CPed::RemoveWeaponWhenEnteringVehicle | PED | thiscall | sa_ped::ped_weapon::remove_weapon_when_entering_vehicle | L2 | split-combat: PASS-FP 48/48 compared, 0 mismatches; split-grove: PASS 134/134 compared, 0 mismatches; split-cheats: PASS-FP 39/39 compared, 0 mismatches; split-pr: PASS-FP 45/45 compared, 0 mismatches | shotgun (slot 3, type 0x1A) case selects slot 2 as the original |
| 0x5E6490 | CPed::ReplaceWeaponWhenExitingVehicle | PED | thiscall | sa_ped::ped_weapon::replace_weapon_when_exiting_vehicle | L2 | split-combat: PASS-FP 12/12 compared, 0 mismatches; split-grove: PASS 4/4 compared, 0 mismatches |  |
| 0x5E6530 | CPed::ReplaceWeaponForScriptedCutscene | PED | thiscall | sa_ped::ped_weapon::replace_weapon_for_scripted_cutscene | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E6550 | CPed::RemoveWeaponForScriptedCutscene | PED | thiscall | sa_ped::ped_weapon::remove_weapon_for_scripted_cutscene | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E6580 | CPed::GetWeaponSkill-Current | PED | thiscall | sa_ped::ped::get_weapon_skill_current | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x5E65A0 | CPed::PreRenderAfterTest | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x5E7680 | CPed::Render | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E7980 | CPed::SetIdle | PED | thiscall | sa_ped::ped::set_idle | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E79B0 | CPed::SetLook-Heading | PED | thiscall | sa_ped::ped::set_look_heading | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E7A60 | CPed::SetLook-Entity | PED | thiscall | sa_ped::ped::set_look_entity | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E7B20 | CPed::Look | PED | thiscall | sa_ped::ped::look | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5E7B70 | CPed::FlagToDestroyWhenNextProcessed | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E7CB0 | CPed::AttachPedToEntity | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E7E60 | CPed::AttachPedToBike | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E7EC0 | CPed::DettachPedFromEntity | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5E8030 | CPed::Constructor | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5E8620 | CPed::Destructor | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5E8830 | CPed::SetAimFlag-Heading | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5E88E0 | CPed::CanWeRunAndFireWithWeapon | PED | thiscall | sa_ped::small::can_we_run_and_fire_with_weapon | L2 | L2 combat:3/3 (0 mismatches, 0 undeclared) | GetWeaponInfo reads only the skill byte, so the zero-extended skill is exact (impl-1) |
| 0x5E8910 | CPed::RequestDelayedWeapon | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E89B0 | CPed::GiveDelayedWeapon | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E8A20 | CPed::PreRender | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E8AB0 | CPed::GiveWeaponAtStartOfFight | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E8BE0 | CPed::GiveWeaponWhenJoiningGang | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5E8C80 | CPed::`scalar deleting destructor' | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x5E8CD0 | CPed::ProcessControl | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x5EFF50 | CPed::GetPedTalking | PED | thiscall | sa_ped::ped::get_ped_talking | L2 | split-combat: PASS-FP 24/24 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 44/44 compared, 0 mismatches |  |
| 0x5EFF60 | CPed::DisablePedSpeech | PED | thiscall | sa_ped::small::disable_ped_speech | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | forwards to the speech entity at +0x294 (impl-1) |
| 0x5EFF70 | CPed::EnablePedSpeech | PED | thiscall | sa_ped::ped::enable_ped_speech | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 2/2 compared, 0 mismatches; split-pr: PASS-FP 1/1 compared, 0 mismatches |  |
| 0x5EFF80 | CPed::DisablePedSpeechForScriptSpeech | PED | thiscall | sa_ped::small::disable_ped_speech_for_script_speech | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | forwards to +0x294 (impl-1) |
| 0x5EFF90 | CPed::EnablePedSpeechForScriptSpeech | PED | thiscall | sa_ped::ped::enable_ped_speech_for_script_speech | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 1/1 compared, 0 mismatches |  |
| 0x5EFFA0 | CPed::CanPedHoldConversation | PED | thiscall | sa_ped::small::can_ped_hold_conversation | L2 | L2 combat:30/30 (0 mismatches, 0 undeclared); L2 grove:8/8 (0 mismatches, 0 undeclared) | forwards to +0x294 (impl-1) |
| 0x5EFFB0 | CPed::SayScript | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5EFFE0 | CPed::Say | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5F0060 | CPed::SetPedAtomicVisibilityCB | PED | cdecl | sa_ped::ped::set_ped_atomic_visibility_cb | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5F0140 | CPed::RemoveBodyPart | PED | thiscall | sa_ped::ped::remove_body_part | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x5F0190 | CPed::SpawnFlyingComponent | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x5F01A0 | CPed::DoesLOSBulletHitPed | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5F0250 | CPed::RemoveWeaponAnims | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x5F02C0 | CPed::IsPedHeadAbovePos | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x5F0360 | CPed::KillPedWithCar | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x6074A0 | CPedIntelligence::operator new | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x608790 | CPedType::FindPedType | PED | cdecl | sa_ped::ped_type::find_ped_type | L2 | L2 combat:408/408 (0 mismatches, 0 undeclared); L2 grove:408/408 (0 mismatches, 0 undeclared); L2 cheats:408/408 (0 mismatches, 0 undeclared); L2 pr:540/540 (0 mismatches, 0 undeclared) |  |
| 0x608830 | CPedType::GetPedFlag | PED | cdecl | sa_ped::ped_type::get_ped_flag | L1 | L1 1002000 cases, 0 mismatches; L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x608850 | CPedStats::Shutdown | PED | cdecl | sa_ped::ped_type::ped_stats_shutdown | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x6088D0 | CPedStats::GetPedStatType | PED | cdecl | sa_ped::ped_type::get_ped_stat_type | L2 | L2 combat:276/276 (0 mismatches, 0 undeclared); L2 grove:276/276 (0 mismatches, 0 undeclared); L2 cheats:276/276 (0 mismatches, 0 undeclared); L2 pr:276/276 (0 mismatches, 0 undeclared) |  |
| 0x608940 | CPedStats::FindIndexWithPedStat | PED | cdecl | sa_ped::ped_type::find_index_with_ped_stat | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x6089B0 | CPedType::GetPedTypeAcquaintances-CAcquaintance | PED | cdecl | sa_ped::ped_type::get_ped_type_acquaintances | L2 | L2 combat:147/147 (0 mismatches, 0 undeclared); L2 grove:206/206 (0 mismatches, 0 undeclared); L2 cheats:119/119 (0 mismatches, 0 undeclared); L2 pr:93/93 (0 mismatches, 0 undeclared) |  |
| 0x6089D0 | CPedType::GetPedTypeAcquaintances-uint32 | PED | cdecl | sa_ped::ped_type::get_ped_type_acquaintances_id | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x6089F0 | CPedType::ClearPedTypeAsAcquaintance | PED | cdecl | sa_ped::ped_type::clear_ped_type_as_acquaintance | L2 | L2 combat:960/960 (0 mismatches, 0 undeclared); L2 grove:960/960 (0 mismatches, 0 undeclared); L2 cheats:960/960 (0 mismatches, 0 undeclared); L2 pr:960/960 (0 mismatches, 0 undeclared) |  |
| 0x608A20 | CPedType::ClearPedTypeAcquaintances | PED | cdecl | sa_ped::ped_type::clear_ped_type_acquaintances | L2 | L2 combat:40/40 (0 mismatches, 0 undeclared); L2 grove:40/40 (0 mismatches, 0 undeclared); L2 cheats:40/40 (0 mismatches, 0 undeclared); L2 pr:40/40 (0 mismatches, 0 undeclared) |  |
| 0x608A40 | CPedType::PoliceDontCareAboutCrimesAgainstPedType | PED | cdecl | sa_ped::ped_type::police_dont_care_about_crimes_against_ped_type | L1 | L1 1002000 cases, 0 mismatches; L2 combat:2/2 (0 mismatches, 0 undeclared) |  |
| 0x608B00 | CPedType::Shutdown | PED | cdecl | sa_ped::ped_type::ped_type_shutdown | implemented | split-pr: PASS-FP 1/1 compared, 0 mismatches |  |
| 0x608B30 | CPedType::LoadPedData | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x608E20 | CPedType::SetPedTypeAsAcquaintance | PED | cdecl | sa_ped::ped_type::set_ped_type_as_acquaintance | L2 | L2 combat:22/22 (0 mismatches, 0 undeclared); L2 grove:22/22 (0 mismatches, 0 undeclared); L2 cheats:22/22 (0 mismatches, 0 undeclared); L2 pr:44/44 (0 mismatches, 0 undeclared) |  |
| 0x608E40 | CPedType::Initialise | PED | — | — | deferred |  | loader/save path (CFileMgr/sscanf or save buffers); needs a load/save recording; SAVE/loader pass |
| 0x609380 | — | PED | cdecl | sa_ped::small::any_of_96917c | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | impl-1 |
| 0x6093B0 | CPlayerPed::~CPlayerPed | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x609490 | CPlayerPed::SetMoveAnim | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x6094A0 | CPlayerPed::RemovePlayerPed | PED | cdecl | sa_ped::player_ped::remove_player_ped | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x609520 | CPlayerPed::DeactivatePlayerPed | PED | cdecl | sa_ped::player_ped::deactivate_player_ped | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches |  |
| 0x609540 | CPlayerPed::ReactivatePlayerPed | PED | cdecl | sa_ped::player_ped::reactivate_player_ped | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches |  |
| 0x609560 | CPlayerPed::GetPadFromPlayer | PED | thiscall | sa_ped::player_ped::get_pad_from_player | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x609590 | CPlayerPed::CanPlayerStartMission | PED | thiscall | sa_ped::player_ped::can_player_start_mission | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:507/507 (0 mismatches, 0 undeclared) |  |
| 0x609620 | CPlayerPed::IsHidden | PED | thiscall | sa_ped::player_ped::is_hidden | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x609650 | CPlayerPed::ReApplyMoveAnims | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x609710 | CPlayerPed::DoesPlayerWantNewWeapon | PED | thiscall | sa_ped::player_ped::does_player_want_new_weapon | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x6097F0 | CPlayerPed::ProcessPlayerWeapon | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x609800 | CPlayerPed::PickWeaponAllowedFor2Player | PED | thiscall | sa_ped::player_ped::pick_weapon_allowed_for_2_player | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x609830 | CPlayerPed::UpdateCameraWeaponModes | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x6098F0 | CPlayerPed::ProcessAnimGroups | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x609C80 | CPlayerPed::ClearWeaponTarget | PED | thiscall | sa_ped::player_ped::clear_weapon_target | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches |  |
| 0x609CD0 | CPlayerPed::GetWeaponRadiusOnScreen | PED | thiscall | sa_ped::player_ped::get_weapon_radius_on_screen | L2 | L2 combat:110/110 (0 mismatches, 0 undeclared) |  |
| 0x609D90 | CPlayerPed::PedCanBeTargettedVehicleWise | PED | cdecl | sa_ped::player_ped::ped_can_be_targetted_vehicle_wise | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x609DE0 | CPlayerPed::FindTargetPriority | PED | thiscall | sa_ped::player_ped::find_target_priority | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x609ED0 | CPlayerPed::Clear3rdPersonMouseTarget | PED | thiscall | sa_ped::player_ped::clear_3rd_person_mouse_target | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 150/150 compared, 0 mismatches |  |
| 0x609EF0 | CPlayerPed::Busted | PED | thiscall | sa_ped::player_ped::busted | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | no player data: writes address 0 (crash) as the original |
| 0x609F10 | CPlayerPed::SetWantedLevel | PED | thiscall | sa_ped::player_ped::set_wanted_level | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | no player data: this = null passed on (as the original) |
| 0x609F30 | CPlayerPed::SetWantedLevelNoDrop | PED | thiscall | sa_ped::player_ped::set_wanted_level_no_drop | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | no player data: this = null passed on (as the original) |
| 0x609F50 | CPlayerPed::CheatWantedLevel | PED | thiscall | sa_ped::player_ped::cheat_wanted_level | L2 | L2 cheats:4/4 (0 mismatches, 0 undeclared) | no player data: this = null passed on (as the original) |
| 0x609F80 | CPlayerPed::CanIKReachThisTarget | PED | thiscall | sa_ped::player_ped::can_ik_reach_this_target | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x609FF0 | CPlayerPed::GetPlayerInfoForThisPlayerPed | PED | thiscall | sa_ped::player_ped::get_player_info_for_this_player_ped | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared) |  |
| 0x60A020 | CPlayerPed::DoStuffToGoOnFire | PED | thiscall | sa_ped::player_ped::do_stuff_to_go_on_fire | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x60A040 | CPlayerPed::AnnoyPlayerPed | PED | thiscall | sa_ped::player_ped::annoy_player_ped | L2 | L2 combat:9/9 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared) |  |
| 0x60A070 | CPlayerPed::ClearAdrenaline | PED | thiscall | sa_ped::player_ped::clear_adrenaline | L2 | L2 combat:4/4 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:4/4 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x60A0A0 | CPlayerPed::DisbandPlayerGroup | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60A110 | CPlayerPed::MakeGroupRespondToPlayerTakingDamage | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60A1D0 | CPlayerPed::TellGroupToStartFollowingPlayer | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60A440 | CPlayerPed::MakePlayerGroupDisappear | PED | thiscall | sa_ped::player_ped::make_player_group_disappear | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x60A4B0 | CPlayerPed::MakePlayerGroupReappear | PED | thiscall | sa_ped::player_ped::make_player_group_reappear | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:1/1 (0 mismatches, 0 undeclared) |  |
| 0x60A530 | CPlayerPed::ResetSprintEnergy | PED | thiscall | sa_ped::player_ped::reset_sprint_energy | ported-unhookable |  | INF-20: the no-CD protection reads the entry as data (sbb ebx, [0x60A533] at 0x4037BC); not registered |
| 0x60A550 | CPlayerPed::HandleSprintEnergy | PED | thiscall | sa_ped::player_ped::handle_sprint_energy | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x60A610 | CPlayerPed::ControlButtonSprint | PED | — | — | deferred |  | not ported in this pass: mid/large body (164 insns) calling unported AI/VEH/HUD code; next PED pass |
| 0x60A820 | CPlayerPed::GetButtonSprintResults | PED | thiscall | sa_ped::player_ped::get_button_sprint_results | ported-unhookable |  | caller 0x60B430 reads EDX after the call; a preserving hook needs an L1 spec, which the hosted harness cannot run for a pointer-reading function (64-bit host); not registered |
| 0x60A8A0 | CPlayerPed::ResetPlayerBreath | PED | thiscall | sa_ped::player_ped::reset_player_breath | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x60A8D0 | CPlayerPed::HandlePlayerBreath | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60A9A0 | CPlayerPed::`scalar deleting destructor' | PED | — | — | deferred |  | whole-ped constructor/destructor (pool, intelligence, audio, SEH); dedicated pass |
| 0x60A9C0 | CPlayerPed::SetRealMoveAnim | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x60B460 | CPlayerPed::MakeChangesForNewWeapon- | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x60B550 | CPlayerPed::LOSBlockedBetweenPeds | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60B650 | CPlayerPed::Compute3rdPersonMouseTarget | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60BA80 | CPlayerPed::DrawTriangleForMouseRecruitPed | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60C0C0 | CPlayerPed::DoesTargetHaveToBeBroken | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60C1E0 | CPlayerPed::KeepAreaAroundPlayerClear | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60C520 | CPlayerPed::SetPlayerMoveBlendRatio | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60C5F0 | CPlayerPed::FindPedToAttack | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60C7C0 | CPlayerPed::ForceGroupToAlwaysFollow | PED | thiscall | sa_ped::player_ped::force_group_to_always_follow | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 1/1 compared, 0 mismatches |  |
| 0x60C800 | CPlayerPed::ForceGroupToNeverFollow | PED | thiscall | sa_ped::player_ped::force_group_to_never_follow | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 1/1 compared, 0 mismatches |  |
| 0x60C840 | CPlayerPed::MakeThisPedJoinOurGroup | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60CC50 | CPlayerPed::PlayerWantsToAttack | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60CD20 | CPlayerPed::SetInitialState | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x60D000 | CPlayerPed::MakeChangesForNewWeapon-BySlot | PED | thiscall | sa_ped::player_ped::make_changes_for_new_weapon_by_slot | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x60D020 | CPlayerPed::EvaluateTarget | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60D1C0 | CPlayerPed::EvaluateNeighbouringTarget | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x60D350 | CPlayerPed::ProcessGroupBehaviour | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x60D5A0 | CPlayerPed::PlayerHasJustAttackedSomeone | PED | thiscall | sa_ped::player_ped::player_has_just_attacked_someone | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x60D790 | CPlayerPed::SetupPlayerPed | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x60D850 | CPlayerPed::ProcessWeaponSwitch | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x60DC50 | CPlayerPed::FindWeaponLockOnTarget | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x60E530 | CPlayerPed::FindNextWeaponLockOnTarget | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x60EA90 | CPlayerPed::ProcessControl | PED | — | — | deferred |  | not ported in this pass: big CPed/CPlayerPed process function (ProcessControl/player control family); dedicated pass |
| 0x616860 | CPedPlacement::IsPositionClearForPed-OG | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x6168E0 | CPedPlacement::IsPositionClearOfCars-Pos | PED | cdecl | sa_ped::misc::is_position_clear_of_cars_pos | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x616920 | CPedPlacement::FindZCoorForPed | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x616A40 | CPedPlacement::IsPositionClearOfCars-Ped | PED | — | — | deferred |  | not ported in this pass: creates or drives AI tasks/events/groups (CTask*/CEvent*/CPedGroup); AI pass |
| 0x61DB30 | — | WEAPON | cdecl | sa_weapon::weapon_info::melee_anim_group | L2 | L2 combat:17/17 (0 mismatches, 0 undeclared); L2 grove:17/17 (0 mismatches, 0 undeclared); L2 cheats:17/17 (0 mismatches, 0 undeclared); L2 pr:34/34 (0 mismatches, 0 undeclared) |  |
| 0x678E80 | — | PED | — | — | deferred |  | not ported in this pass (time): small body, next PED pass |
| 0x685700 | CWeaponInfo::GetCrouchReloadAnimationID | WEAPON | thiscall | sa_weapon::weapon_info::get_crouch_reload_animation_id | L1 | L1 1,002,000 cases, 0 mismatches; L2 combat:76/76 (0 mismatches, 0 undeclared) |  |
| 0x6AE0D0 | CPed::MakeTyresMuddySectorList<CPtrListSingleLink<CPhysical*>> | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x6B4200 | CPed::DeadPedMakesTyresBloody | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x6E61B0 | (unnamed) | WEAPON | — | — | deferred |  | CWaterLevel::TestLineAgainstWater (proposed): 360 insns, water level geometry, called only from FireInstantHit; next pass |
| 0x6FF8A0 | CPlayerSkin::Initialise | PED | cdecl | sa_ped::misc::skin_initialise | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 1/1 compared, 0 mismatches |  |
| 0x6FF8D0 | CPlayerSkin::Shutdown | PED | cdecl | sa_ped::misc::skin_shutdown | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x6FF900 | CPlayerSkin::RenderFrontendSkinEdit | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x6FFA10 | CPlayerSkin::GetSkinTexture | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x71D730 | CMBlur::SetDrunkBlur | PED | — | — | deferred |  | not ported in this pass: render/anim/collision-heavy body (RW, anim blend, bones); dedicated pass |
| 0x728B10 | CWaterCannon::CWaterCannon | WEAPON | thiscall | sa_weapon::water_cannon::constructor | L1 | L1 1,002,000 cases, 0 mismatches; L2 combat:3/3 (0 mismatches, 0 undeclared); L2 grove:3/3 (0 mismatches, 0 undeclared); L2 cheats:3/3 (0 mismatches, 0 undeclared); L2 pr:3/3 (0 mismatches, 0 undeclared) |  |
| 0x728B30 | CWaterCannon::~CWaterCannon | WEAPON | thiscall | sa_weapon::water_cannon::destructor | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x728B40 | CWaterCannon::Init | WEAPON | thiscall | sa_weapon::water_cannon::init | L2 | split-combat: PASS-FP 2/2 compared, 0 mismatches; split-grove: PASS 2/2 compared, 0 mismatches; split-cheats: PASS-FP 2/2 compared, 0 mismatches; split-pr: PASS-FP 4/4 compared, 0 mismatches |  |
| 0x728C20 | CWaterCannon::Update_NewInput | WEAPON | thiscall | sa_weapon::water_cannon::update_new_input | L1 | L1 1,060,000 cases, 0 mismatches (INCOMPLETE only by 80,700 SNaN cases, unverified with the native backend off) |  |
| 0x728C80 | CWaterCannons::Init | WEAPON | cdecl | sa_weapon::water_cannon::cannons_init | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches |  |
| 0x728CB0 | CWaterCannons::UpdateOne | WEAPON | cdecl | sa_weapon::water_cannon::cannons_update_one | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x728DA0 | CWaterCannon::Render | WEAPON | — | — | deferred |  | CWaterCannon::Render: 546 insns, RW immediate-mode rendering; render pass |
| 0x7295E0 | CWaterCannon::PushPeds | WEAPON | — | — | deferred |  | CWaterCannon::PushPeds: 371 insns, drives ped events (CEventHitByWaterCannon) and physics; AI-adjacent, next pass |
| 0x729B30 | CWaterCannons::Render | WEAPON | cdecl | sa_weapon::water_cannon::cannons_render | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 150/150 compared, 0 mismatches |  |
| 0x72A280 | CWaterCannon::Update_OncePerFrame | WEAPON | thiscall | sa_weapon::water_cannon::update_once_per_frame | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | last-input time not refreshed on section advance (as the bytes) |
| 0x72A3C0 | CWaterCannons::Update | WEAPON | cdecl | sa_weapon::water_cannon::cannons_update | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 150/150 compared, 0 mismatches |  |
| 0x735FD0 | CBulletInfo::Initialise | WEAPON | cdecl | sa_weapon::bullet_shot::bullet_initialise | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x736000 | CBulletInfo::Shutdown | WEAPON | — | — | deferred |  | CBulletInfo::Shutdown: 1-byte body (ret): nothing to port, not hookable (excluded from hooking) |
| 0x736010 | CBulletInfo::AddBullet | WEAPON | cdecl | sa_weapon::bullet_shot::add_bullet | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | reads the active byte one element past the 8 bullets when all are in use (original OOB read kept) |
| 0x7360D0 | CBulletInfo::Update | WEAPON | cdecl | sa_weapon::bullet_shot::bullet_update | L2 | split-combat: PASS 150/150 compared, 0 mismatches; split-c-bug: PASS 150/150 compared, 0 mismatches; split-driving: PASS 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS 150/150 compared, 0 mismatches | pass 2 |
| 0x736840 | CExplosion::ClearAllExplosions | WEAPON | cdecl | sa_weapon::explosion::clear_all_explosions | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | inner loop writes three fixed addresses each iteration (original bug kept) |
| 0x7368F0 | CExplosion::Shutdown | WEAPON | — | — | deferred |  | CExplosion::Shutdown: 1-byte body (ret): nothing to port, not hookable |
| 0x736900 | CExplosion::GetExplosionActiveCounter | WEAPON | cdecl | sa_weapon::explosion::get_explosion_active_counter | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x736910 | CExplosion::ResetExplosionActiveCounter | WEAPON | cdecl | sa_weapon::explosion::reset_explosion_active_counter | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x736920 | CExplosion::DoesExplosionMakeSound | WEAPON | cdecl | sa_weapon::explosion::does_explosion_make_sound | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x736930 | CExplosion::GetExplosionType | WEAPON | cdecl | sa_weapon::explosion::get_explosion_type | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x736940 | CExplosion::GetExplosionPosition | WEAPON | cdecl | sa_weapon::explosion::get_explosion_position | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x736950 | CExplosion::TestForExplosionInArea | WEAPON | cdecl | sa_weapon::explosion::test_for_explosion_in_area | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x7369E0 | CExplosion::RemoveAllExplosionsInArea | WEAPON | cdecl | sa_weapon::explosion::remove_all_explosions_in_area | L2 | L2 combat:5/5 (0 mismatches, 0 undeclared); L2 grove:2/2 (0 mismatches, 0 undeclared); L2 cheats:5/5 (0 mismatches, 0 undeclared); L2 pr:4/4 (0 mismatches, 0 undeclared) |  |
| 0x736A40 | CExplosion::Initialise | WEAPON | cdecl | sa_weapon::explosion::initialise | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x736A50 | CExplosion::AddExplosion | WEAPON | cdecl | sa_weapon::explosion::add_explosion | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-c-bug: no call; split-driving: PASS 2/2 compared, 0 mismatches; split-grove: no call; split-cheats: no call | pass 2 |
| 0x737620 | CExplosion::Update | WEAPON | cdecl | sa_weapon::explosion::update | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 150/150 compared, 0 mismatches | gr deviations kept: GRN helper formula, first random value to y, VICTIM position, z+2; TriggerExplosion false flag = type dword with AL cleared; rand() attributed to 0x73771C |
| 0x737B40 | CProjectileInfo::Initialise | WEAPON | cdecl | sa_weapon::projectile::initialise | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x737B80 | CProjectileInfo::RemoveFXSystem | WEAPON | thiscall | sa_weapon::projectile::remove_fx_system | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x737BC0 | CProjectileInfo::Shutdown | WEAPON | cdecl | sa_weapon::projectile::shutdown | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x737BF0 | CProjectileInfo::GetProjectileInfo | WEAPON | cdecl | sa_weapon::projectile::get_projectile_info | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x737C00 | CProjectileInfo::RemoveNotAdd | WEAPON | cdecl | sa_weapon::projectile::remove_not_add | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x737C80 | CProjectileInfo::AddProjectile | WEAPON | — | — | deferred |  | pass 2: not ported; entry `6A FF E9` jumps into protection code at 0x401C3D (hookability to check first); time box |
| 0x738860 | CProjectileInfo::RemoveDetonatorProjectiles | WEAPON | cdecl | sa_weapon::projectile::remove_detonator_projectiles | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x7388F0 | CProjectileInfo::RemoveProjectile | WEAPON | cdecl | sa_weapon::projectile::remove_projectile | implemented-needs-L2 | split-combat: no call; split-c-bug: no call; split-driving: no call; split-grove: no call; split-cheats: no call | pass 2 |
| 0x738B20 | CProjectileInfo::Update | WEAPON | cdecl | sa_weapon::projectile::update | L2 | split-combat: PASS 150/150 compared, 0 mismatches; split-c-bug: PASS 150/150 compared, 0 mismatches; split-driving: PASS 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS 150/150 compared, 0 mismatches | pass 2 |
| 0x739860 | CProjectileInfo::IsProjectileInRange | WEAPON | cdecl | sa_weapon::projectile::is_projectile_in_range | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x7399B0 | CProjectileInfo::RemoveAllProjectiles | WEAPON | cdecl | sa_weapon::projectile::remove_all_projectiles | L2 | split-combat: PASS-FP 5/5 compared, 0 mismatches; split-grove: PASS 2/2 compared, 0 mismatches; split-cheats: PASS-FP 5/5 compared, 0 mismatches; split-pr: PASS-FP 5/5 compared, 0 mismatches |  |
| 0x739A40 | CProjectileInfo::RemoveIfThisIsAProjectile | WEAPON | cdecl | sa_weapon::projectile::remove_if_this_is_a_projectile | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x739B60 | CShotInfo::Initialise | WEAPON | cdecl | sa_weapon::bullet_shot::shot_initialise | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x739C20 | CShotInfo::Shutdown | WEAPON | — | — | deferred |  | CShotInfo::Shutdown: 1-byte body (ret): nothing to port, not hookable |
| 0x739C30 | CShotInfo::AddShot | WEAPON | cdecl | sa_weapon::bullet_shot::add_shot | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | reads the active byte one element past the 100 shots when all are in use (original OOB read kept) |
| 0x739DE0 | CShotInfo::GetFlameThrowerShotPosn | WEAPON | cdecl | sa_weapon::bullet_shot::get_flame_thrower_shot_posn | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x739E60 | CShotInfo::Update | WEAPON | cdecl | sa_weapon::bullet_shot::shot_update | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 150/150 compared, 0 mismatches | range growth ts*0.075f (gr 0.0075f), _CIpow(0.96 double, ts) kept in x87 (gr powf); scanner entries 16..1; spray-can reflection doubles the reflected vector as the bytes do |
| 0x73A280 | — | WEAPON | thiscall | sa_weapon::weapon_info::reload_loop_time_a | L2 | L2 combat:73/73 (0 mismatches, 0 undeclared) |  |
| 0x73A2B0 | — | WEAPON | thiscall | sa_weapon::weapon_info::reload_loop_time_b | L2 | L2 combat:73/73 (0 mismatches, 0 undeclared) |  |
| 0x73A300 | CWeapon::InitialiseWeapons | WEAPON | cdecl | sa_weapon::weapon::initialise_weapons | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches |  |
| 0x73A330 | CWeapon::ShutdownWeapons | WEAPON | cdecl | sa_weapon::weapon::shutdown_weapons | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x73A360 | CWeapon::UpdateWeapons | WEAPON | cdecl | sa_weapon::weapon::update_weapons | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 150/150 compared, 0 mismatches |  |
| 0x73A380 | CWeapon::Shutdown | WEAPON | thiscall | sa_weapon::weapon::shutdown | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:962/962 (0 mismatches, 0 undeclared) |  |
| 0x73A3E0 | CWeapon::AddGunshell | WEAPON | thiscall | sa_weapon::weapon::add_gunshell | implemented | split-combat: PASS-FP 135/135 compared, 0 mismatches | extra discarded GetRandomNumberInRange(-20, 20) kept (spec CONFIRMED); original sum order (z²+x²)+y² |
| 0x73A530 | CWeapon::GenerateDamageEvent | WEAPON | cdecl | sa_weapon::weapon_impact::generate_damage_event | implemented | split-combat: DIVERGED 3/11 compared, 0 mismatches; split-c-bug: DIVERGED 0/38 compared, 0 mismatches; split-driving: DIVERGED 0/26 compared, 0 mismatches; split-grove: no call; split-cheats: no call | pass 2; DIVERGED pairs: CReferences (0xB9B9A8..) holds stack addresses of reference slots below a Rust caller (plain 0x0036F06C / rust 0x0036EF5C); harness Stack-class gap, no pair with equal inputs mismatched |
| 0x73A8D0 | CWeapon::LaserScopeDot | WEAPON | — | — | deferred |  | CWeapon::LaserScopeDot: no callers in 1.0 US (dead); 145 insns with corona registration; low priority |
| 0x73AAC0 | CWeapon::FireSniper | WEAPON | thiscall | sa_weapon::weapon_fire::fire_sniper | implemented-needs-L2 | split-combat: no call; split-c-bug: no call; split-driving: no call; split-grove: no call; split-cheats: no call | pass 2 |
| 0x73AEB0 | CWeapon::Reload | WEAPON | thiscall | sa_weapon::weapon::reload | L2 | L2 combat:150/150 (0 mismatches, 0 undeclared); L2 grove:206/206 (0 mismatches, 0 undeclared); L2 cheats:118/118 (0 mismatches, 0 undeclared); L2 pr:93/93 (0 mismatches, 0 undeclared) |  |
| 0x73AF00 | CWeapon::FireOneInstantHitRound | WEAPON | — | — | deferred |  | CWeapon::FireOneInstantHitRound: 228 insns, Fire family; next pass |
| 0x73B1C0 | CWeapon::IsTypeMelee | WEAPON | thiscall | sa_weapon::weapon::is_type_melee | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:131/131 (0 mismatches, 0 undeclared) |  |
| 0x73B1E0 | CWeapon::IsType2Handed | WEAPON | thiscall | sa_weapon::weapon::is_type_2_handed | L1 | L1 1,002,000 cases, 0 mismatches |  |
| 0x73B210 | CWeapon::IsTypeProjectile | WEAPON | thiscall | sa_weapon::weapon::is_type_projectile | L1 | L1 1,002,000 cases, 0 mismatches |  |
| 0x73B240 | CWeapon::CanBeUsedFor2Player-Static | WEAPON | cdecl | sa_weapon::weapon::can_be_used_for_2_player | L1 | L1 1,002,000 cases, 0 mismatches |  |
| 0x73B2A0 | CWeapon::HasWeaponAmmoToBeUsed | WEAPON | thiscall | sa_weapon::weapon::has_weapon_ammo_to_be_used | L1 | L1 1,002,000 cases, 0 mismatches; L2 combat:4/4 (0 mismatches, 0 undeclared) |  |
| 0x73B300 | CWeapon::ProcessLineOfSight | WEAPON | cdecl | sa_weapon::weapon::process_line_of_sight | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | argument 11 ignored; see-through 0 and shoot-through 1 passed as the bytes do |
| 0x73B360 | CWeapon::StopWeaponEffect | WEAPON | thiscall | sa_weapon::weapon::stop_weapon_effect | implemented | split-combat: PASS-FP 4/4 compared, 0 mismatches |  |
| 0x73B380 | CWeapon::TargetWeaponRangeMultiplier | WEAPON | cdecl | sa_weapon::weapon::target_weapon_range_multiplier | L2 | L2 combat:95/95 (0 mismatches, 0 undeclared) |  |
| 0x73B430 | CWeapon::Constructor | WEAPON | thiscall | sa_weapon::weapon::constructor | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x73B4A0 | CWeapon::Initialise | WEAPON | thiscall | sa_weapon::weapon::initialise | L2 | L2 combat:33/33 (0 mismatches, 0 undeclared); L2 grove:46/46 (0 mismatches, 0 undeclared); L2 cheats:50/50 (0 mismatches, 0 undeclared); L2 pr:22/22 (0 mismatches, 0 undeclared) |  |
| 0x73B550 | CWeapon::DoBulletImpact | WEAPON | thiscall | sa_weapon::weapon_impact::do_bullet_impact | implemented | split-combat: PASS-FP 134/134 compared, 0 mismatches; split-c-bug: PASS-FP 147/147 compared, 0 mismatches; split-driving: PASS-FP 45/45 compared, 0 mismatches; split-grove: no call; split-cheats: no call | pass 2; PASS-FP: the CReferences pages differ before the call (stack addresses, as GenerateDamageEvent); 0 mismatches |
| 0x73C1F0 | CWeapon::TakePhotograph | WEAPON | — | — | deferred |  | CWeapon::TakePhotograph: 389 insns, ped/object pool scans with line-of-sight and screen tests; next pass |
| 0x73C710 | CWeapon::SetUpPelletCol | WEAPON | — | — | deferred |  | CWeapon::SetUpPelletCol: 329 insns, CColModel allocation + shotgun pellet geometry with rand; next pass |
| 0x73CBA0 | CWeapon::FireInstantHitFromCar2 | WEAPON | thiscall | sa_weapon::weapon_fire::fire_instant_hit_from_car2 | implemented-needs-L2 | split-combat: no call; split-c-bug: no call; split-driving: no call; split-grove: no call; split-cheats: no call | pass 2 |
| 0x73CDC0 | CWeapon::DoDoomAiming | WEAPON | — | — | deferred |  | CWeapon::DoDoomAiming: 354 insns, target search with angle tests; next pass |
| 0x73D1E0 | CWeapon::DoTankDoomAiming | WEAPON | — | — | deferred |  | CWeapon::DoTankDoomAiming: 435 insns, target search over pools with camera/screen projections; next pass |
| 0x73D720 | CWeapon::DoDriveByAutoAiming | WEAPON | — | — | deferred |  | CWeapon::DoDriveByAutoAiming: 371 insns, target search + CalcScreenCoors; next pass |
| 0x73DB40 | CWeapon::Update | WEAPON | thiscall | sa_weapon::weapon::update | L2 | split-combat: PASS-FP 150/150 compared, 0 mismatches; split-grove: PASS 150/150 compared, 0 mismatches; split-cheats: PASS-FP 150/150 compared, 0 mismatches; split-pr: PASS-FP 150/150 compared, 0 mismatches | reload branch writes byte 0 into its own argument slot (low byte of the ped pointer; not reproduced; L2 argument words may differ in state 2) |
| 0x73DEF0 | CWeapon::CanBeUsedFor2Player-Method | WEAPON | thiscall | sa_weapon::weapon::can_be_used_for_2_player_method | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x73E240 | CWeapon::FindNearestTargetEntityWithScreenCoors | WEAPON | — | — | deferred |  | CWeapon::FindNearestTargetEntityWithScreenCoors: 248 insns, screen-space target search; next pass |
| 0x73E560 | CWeapon::EvaluateTargetForHeatSeekingMissile | WEAPON | cdecl | sa_weapon::weapon::evaluate_target_for_heat_seeking_missile | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | the original stores d and l into its own argument slots 0/1 (not reproduced; L2 argument words may differ) |
| 0x73E690 | CWeapon::DoWeaponEffect | WEAPON | thiscall | sa_weapon::weapon::do_weapon_effect | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | passes the .rdata fx name instead of the stack copy (same characters) |
| 0x73E800 | CWeapon::FireAreaEffect | WEAPON | — | — | deferred |  | pass 2: not ported; `fptan` at 0x73E9CE has no sa_math::x87 model (needs X87::ftan with an sa-oracle check) |
| 0x73EC40 | CWeapon::FireInstantHitFromCar | WEAPON | thiscall | sa_weapon::weapon_fire::fire_instant_hit_from_car | implemented-needs-L2 | split-combat: no call; split-c-bug: no call; split-driving: no call; split-grove: no call; split-cheats: no call | pass 2 |
| 0x73F480 | CWeapon::CheckForShootingVehicleOccupant | WEAPON | cdecl | sa_weapon::weapon_fire::check_for_shooting_vehicle_occupant | ported-unhookable |  | pass 2: INF-20 reader `mov ecx, [0x73F482]` at 0x1567062 has chunk = 0 in catalog/code-data-reads.csv; sa-hook's data_reads test accepts keycells readers only with chunk = 1 |
| 0x73F910 | CWeapon::PickTargetForHeatSeekingMissile | WEAPON | cdecl | sa_weapon::weapon::pick_target_for_heat_seeking_missile | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | the original writes origin+dir*5 over its own origin arguments (not reproduced; L2 argument words may differ) |
| 0x73FA20 | CWeapon::FireFromCar | WEAPON | thiscall | sa_weapon::weapon::fire_from_car | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x73FB10 | CWeapon::FireInstantHit | WEAPON | — | — | deferred |  | pass 2: not ported; the early-exit path passes an uninitialised stack `dir` to Fx_c::TriggerGunshot and an uninitialised CColPoint to DoBulletImpact: needs a runtime measurement of those bytes at 0x740B71 (PLAN 4b) |
| 0x741360 | CWeapon::FireProjectile | WEAPON | — | — | deferred |  | pass 2: not ported (time box); entry stub `push -1; jmp 0x403C19` |
| 0x741C00 | CWeapon::FireM16_1stPerson | WEAPON | thiscall | sa_weapon::weapon_fire::fire_m16_1st_person | implemented-needs-L2 | split-combat: no call; split-c-bug: no call; split-driving: no call; split-grove: no call; split-cheats: no call | pass 2 |
| 0x742300 | CWeapon::Fire | WEAPON | thiscall | sa_weapon::weapon_fire::fire | L2 | split-combat: PASS 135/135 compared, 0 mismatches; split-c-bug: PASS 147/147 compared, 0 mismatches; split-driving: PASS 45/45 compared, 0 mismatches; split-grove: no call; split-cheats: no call | pass 2 |
| 0x742AB0 | CWeaponEffects::Init | WEAPON | cdecl | sa_weapon::effects::init | L2 | split-combat: PASS-FP 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS-FP 1/1 compared, 0 mismatches; split-pr: PASS-FP 2/2 compared, 0 mismatches |  |
| 0x742B80 | CWeaponEffects::Shutdown | WEAPON | cdecl | sa_weapon::effects::shutdown | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x742BD0 | CWeaponEffects::IsLockedOn | WEAPON | cdecl | sa_weapon::effects::is_locked_on | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat | returns _ftol EAX (EDX of _ftol not reproduced) |
| 0x742BF0 | CWeaponEffects::MarkTarget | WEAPON | cdecl | sa_weapon::effects::mark_target | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x742C60 | CWeaponEffects::ClearCrossHair | WEAPON | cdecl | sa_weapon::effects::clear_cross_hair | L2 | L2 combat:167/167 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:161/161 (0 mismatches, 0 undeclared); L2 pr:2/2 (0 mismatches, 0 undeclared) |  |
| 0x742C80 | CWeaponEffects::ClearCrossHairs | WEAPON | cdecl | sa_weapon::effects::clear_cross_hairs | L2 | L2 combat:128/128 (0 mismatches, 0 undeclared); L2 cheats:128/128 (0 mismatches, 0 undeclared) |  |
| 0x742CA0 | CWeaponEffects::ClearCrossHairImmediately | WEAPON | cdecl | sa_weapon::effects::clear_cross_hair_immediately | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x742CC0 | CWeaponEffects::ClearCrossHairsImmediately | WEAPON | cdecl | sa_weapon::effects::clear_cross_hairs_immediately | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x742CF0 | CWeaponEffects::Render | WEAPON | — | — | deferred |  | CWeaponEffects::Render: 1106 insns, RW immediate-mode rendering; render pass |
| 0x743C50 | CWeaponInfo::Shutdown | WEAPON | cdecl | sa_weapon::weapon_info::shutdown | ported-unhookable |  | ported-unhookable: 1-byte body (ret) |
| 0x743C60 | CWeaponInfo::GetWeaponInfo-original | WEAPON | cdecl | sa_weapon::weapon_info::get_weapon_info | L2 | L2 combat:1000/1001 (0 mismatches, 0 undeclared); L2 grove:1000/1001 (0 mismatches, 0 undeclared); L2 cheats:1000/1001 (0 mismatches, 0 undeclared); L2 pr:1000/1001 (0 mismatches, 0 undeclared) | skill > 3 uses index 0x2F (EAX of the split chunk 0x4075C9; original bug kept) |
| 0x743CD0 | CWeaponInfo::GetSkillStatIndex | WEAPON | cdecl | sa_weapon::weapon_info::get_skill_stat_index | L1 | L1 1,002,000 cases, 0 mismatches; L2 combat:1000/1001 (0 mismatches, 0 undeclared) |  |
| 0x743D10 | CWeaponInfo::FindWeaponType | WEAPON | cdecl | sa_weapon::weapon_info::find_weapon_type | L2 | L2 combat:90/90 (0 mismatches, 0 undeclared); L2 grove:90/90 (0 mismatches, 0 undeclared); L2 cheats:90/90 (0 mismatches, 0 undeclared); L2 pr:160/160 (0 mismatches, 0 undeclared) |  |
| 0x743D50 | CWeaponInfo::GetTargetHeadRange | WEAPON | thiscall | sa_weapon::weapon_info::get_target_head_range | L1 | L1 1,000,058 cases, 0 mismatches (INCOMPLETE only by 12,117 SNaN cases, unverified with the native backend off) |  |
| 0x743D70 | CWeaponInfo::GetWeaponReloadTime | WEAPON | thiscall | sa_weapon::weapon_info::get_weapon_reload_time | L2 | L2 combat:33/33 (0 mismatches, 0 undeclared) |  |
| 0x84DE70 | __init_gFireManager | WEAPON | cdecl | sa_weapon::statics::init_fire_manager | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x84FF30 | __init_PlayerClothes | PED | cdecl | sa_ped::misc::init_player_clothes | L2 | L2 combat:1/1 (0 mismatches, 0 undeclared); L2 grove:1/1 (0 mismatches, 0 undeclared); L2 cheats:1/1 (0 mismatches, 0 undeclared); L2 pr:1/1 (0 mismatches, 0 undeclared) |  |
| 0x855420 | __init_CWaterCannons__aCannons | WEAPON | cdecl | sa_weapon::statics::init_water_cannons | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x8559E0 | __init_CExplosion__m_ExplosionAudioEntity | WEAPON | cdecl | sa_weapon::statics::init_explosion_audio | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x855A00 | __init_g_c89190 | WEAPON | cdecl | sa_weapon::statics::init_g_c89190 | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x855AC0 | __init_g_c89678 | WEAPON | cdecl | sa_weapon::statics::init_g_c89678 | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x855C20 | __init_g_c8a7d8 | WEAPON | cdecl | sa_weapon::statics::init_g_c8a7d8 | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x855C40 | __init_CWeapon__ms_PelletTestCol | WEAPON | cdecl | sa_weapon::statics::init_pellet_test_col | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x855D20 | __init_gCrossHair | WEAPON | cdecl | sa_weapon::statics::init_cross_hair | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x855D70 | __init_g_c8aaa4 | WEAPON | cdecl | sa_weapon::statics::init_g_c8aaa4 | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x855E10 | __init_CWeaponInfo__aWeaponInfo | WEAPON | cdecl | sa_weapon::statics::init_weapon_infos | L2 | split-combat: PASS 1/1 compared, 0 mismatches; split-grove: PASS 1/1 compared, 0 mismatches; split-cheats: PASS 1/1 compared, 0 mismatches; split-pr: PASS 1/1 compared, 0 mismatches |  |
| 0x856420 | __dtor_gFireManager | WEAPON | cdecl | sa_weapon::statics::dtor_fire_manager | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x856BF0 | __dtor_CWaterCannons__aCannons | WEAPON | cdecl | sa_weapon::statics::dtor_water_cannons | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x856C20 | __dtor_CExplosion__m_ExplosionAudioEntity | WEAPON | cdecl | sa_weapon::statics::dtor_explosion_audio | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x856C30 | __dtor_CWeapon__ms_PelletTestCol | WEAPON | cdecl | sa_weapon::statics::dtor_pellet_test_col | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x856C40 | __dtor_gCrossHair | WEAPON | cdecl | sa_weapon::statics::dtor_cross_hair | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
| 0x856C60 | __dtor_CWeaponInfo__aWeaponInfo | WEAPON | cdecl | sa_weapon::statics::dtor_weapon_infos | implemented-needs-L2 | no call in grove walk / pause-restart / world-cheats / weapon-combat |  |
