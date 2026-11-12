# INTEGRATE-8: WEB-3's tree on master

Integration branch `worktree-agent-ac8a20a070c956b08`, from master `b239e888`. It brings in WEB-3
(`worktree-agent-a51e2199757a44c0b`, head `421ba072`): the web build (wasm32, WebGL2/WebGPU, input, full window,
menu cursor), SA:MP in the browser through sa-udp-proxy, SAMP-4 + SAMP-START + SAMP-UI-2, STD-15 driving, the
WRITER-NATIVE / WRITER-CLIMB / WORLD (world6-1/2/3) picks and WEB-AUDIO.

## How

WEB-3 sits on WEB-SAMP → WEB-2 → STD-14 → STD-13, which master already carries under other hashes (INTEGRATE-7's
picks). Both histories start at `efe5120`, so WEB-3 came in as one three-way merge with that base (`d605f11f`):
the commits both sides carry are identical changes there and merged without conflicts. 58 files conflicted.
They were settled like this:

- Generated files (`catalog/verify-queue.csv`, `docs/verify/QUEUE.md`, `crates/sa-ai/src/gen_*.rs`): one side
  was taken, then everything was regenerated. `tools/gen/inflight.py` was rerun for `tools/gen/exclude_inflight.txt`
  (88 addresses), then `tools/gen/ai_patterns.py` and `tools/gen/patterns.py` ran, and `--check` is clean on both.
  `python3 -I tools/verify/queue.py` gives 2168 unverified hooks in 365 batches. The regeneration also adds
  `sa-ped/src/ped_generated.rs` (three bodies too short to hook) and drops `sa-veh/src/bike_generated.rs`.
- Module, hook and native lists (`lib.rs`, `hooks.rs`, `world/mod.rs`, sa-ports `NATIVES`,
  `tools/gen/ai_patterns_exclude.txt`): unions, with the lines the merge doubled removed (`car_enter_quit` and
  three STD-12 hook lines in sa-veh, 20 module lines in `world/mod.rs`, three exclude lines).
- Source files that both sides added or changed (sa-game, sa-samp-ui, sa-samp overlay, the STD-14 AI/VEH files,
  `docs/samp/overlay.md`, `docs/arch/platform.md`): WEB-3's version. It is the later state of the same lanes
  (STD-15 on STD-14, SAMP-UI-2 on SAMP-UI). Many of these hunks were only rustfmt differences.
- `crates/sa-ai/src/ped_attractor_base.rs`: master's version. It has INTEGRATE-7's f64 (ST0) `orig!` fix
  (`b34c610`), without which the phase A DLL does not build.
- `crates/sa-oracle/tests/l1.rs` `PRESERVE_SPECS_PENDING`: master's union (0x4F5F30, 0x4F6000, 0x6289F0,
  0x4ADE90, 0x4B13C0).
- keycells `READERS`: the union by `at` gives **43** readers, with no duplicate `at`. This is WEB-3's 41 plus
  WRITER-SAMP's 0x1561116 (CProjectile ctor) and 0x403968 (CPed::IsPointerValid).
- `crates/sa-game/src/samp.rs`: the `parses_the_server_address` test was given the `proxy: None` field that
  WEB-SAMP added to `Target`. Without it the sa-game test target does not compile, on WEB-3 too.
- `crates/sa-game/Cargo.toml`: the samp-rs dependency now points at `file:///home/ghosty2004/samp-rs` (git, rev
  `fb890107…`, branch `sa-engine/adapter`) instead of SAMP-4's temporary clone. `cargo update -p samp-client` was
  run, and `Cargo.lock` now names the same rev at the new path.

## One registration per address

A scan of every `#[hook(addr …)]` / `#[native(addr …)]` in `crates/` found these new doubles after the merge.
They were settled in favour of WEB-3's later lanes:

| address | kept | removed |
|---|---|---|
| 0x540E80, 0x540F80 (CPad mouse-steering readers) | STD-15 `sa-input/src/pad_mouse_steer.rs` (as on WEB-3) | WRITER-SAMP `pad_look_axes.rs` |
| 0x49F750 `Fx_c::AddDebris` | STD-15 `sa-render/src/fx_debris_wheel_grass.rs` | `fx_add_debris.rs` |
| 0x700EC0 `CPostEffects::DrawQuad` | STD-15 `sa-render/src/postfx_speed_fx.rs` | WRITER-SAMP's copy in `quad_marker_wind_index_buffer.rs` (the other three hooks of that file stay) |
| 0x595380 `InteriorGroup_c::ArePathsLoaded` | PORT-VEH-4 `interior_group_paths.rs` (WEB-3's world6-2 pick calls it through the registry) | WRITER-WORLD `interior_paths_loaded.rs` |
| native 0x41BCB0 `CBridge::ThisIsABridgeObjectMovingUp` | STD-15 `sa-ai/src/short_body_natives.rs` | the copy in `sa-core/src/short_entry_natives.rs` |
| native 0x6F3FF0 (crane) | one copy in `short_entry_natives.rs` | the second copy the merge added |
| 0x5E4220 / 0x5E8A30 | both: these are two different functions that shared the file name `ped_pointer_valid.rs` | WRITER-SAMP's `CPed::IsPointerValid` moved to `sa-ped/src/ped_is_pointer_valid.rs` |

WEB-3's settlement also covers these:
- PORT-VEH-4's `BreakObject_c`, `WaterCreature_c`, `GivePlayerGoodiesWithPickUpMI` and the
  `Remove2ndPlayerIfPresent` native are kept, and WRITER-WORLD's world5 copies of the same files are dropped. This
  is the reverse of INTEGRATE-7.
- STD-15's `FxInfoColourBright_c::GetValue` 0x4A6860 and its CEventDeath destructor 0x4ADE90 are kept, and
  WRITER-SAMP's copies are dropped.

WRITER-WORLD's world5 files that have no double (`cheat_player_toggles.rs`, `furniture_hand_object_mines.rs`)
stay.

WEB-3 itself registers these addresses twice. Its own `cargo test -p sa-hook --lib` fails because of the first one:
- `65557b1c`: 0x733670 `CVisibilityPlugins::RenderWeaponCB` stays with STD-10's `visibility_weapon.rs`, as on
  master, and SAMP-4's copy in `visibility_ped.rs` is removed. This fixes `every_hook_installs_on_the_exe`
  ("RenderWeaponCB overlaps RenderWeaponCB") and `unverified_selection`.
- `9bb545a7`: SAMP-4's phase B natives for 0x540E80 / 0x540F80 (`sa-cam` padlook), 0x680600 (`fall.rs`) and
  0x6D1E20 (`vehicle2.rs`) are no longer registered. The hooks for these addresses win:
  `pad_mouse_steer`, `task_in_air_process_ped` and `wake_locks_component_rotation`. The functions stay, because
  Rust callers use them.

Native sa-game now reports 5 addresses with a second registration. All 5 were already doubled on master, as
INTEGRATE-7 noted:
- 0x5B0440 and 0x618E70: `std7_natives` plus a `#[native]`.
- 0x84A540, 0x84E300 and 0x84F320: `static_inits` plus the PORT-VEH-3 hooks.

0x407180 is only sa-macros test text.

## Not taken

- WRITER-WORLD-2 (`worktree-agent-a33f0e44a9994a468`, beyond world6-1/2/3) and WRITER-SAMP-2
  (`worktree-agent-aa6be1d1b3cc0f218`, apart from CSkidmarks::RegisterOne, which WEB-3 already took) were not
  taken. There was no time for a second verify-all before the session ends. The open items from INTEGRATE-7 still
  apply to them: FireInstantHit 0x73FB10 needs detwin to skip its scrub, and its `Fx_c::AddSparks` file is a
  duplicate.

## Tests

- `cargo test --release -p sa-hook --lib`: 58 passed, at `65557b1c` and again at `9bb545a7`. At `d605f11f` and on
  WEB-3 it fails twice, because of 0x733670. `-p sa-script --lib`: 9 passed. `-p sa-core --lib`: 73 passed.
  `-p sa-rw-wgpu --lib`: 58 passed.
- `cargo test -p sa-oracle --test l1 --release`: 17 passed, 1 ignored and 1 failed, in 589 s with no hang. The
  failure was `preserved_registers_match_the_hook_registrations`: WRITER-NATIVE's
  `CPlayerPed::GetButtonSprintResults` 0x60A820 has `preserve = "edx"` but no spec. `907043d6` puts it in
  `PRESERVE_SPECS_PENDING`, and that test now passes.
- sa-oracle, satool and sa-oracle-format were rebuilt in this worktree (release).
- `i686-pc-windows-gnu` release build of sa-launcher and sa-hook: clean.
- sa-game `i686-unknown-linux-gnu` release build: clean, with and without `--features samp`. The samp build uses
  samp-rs from `file:///home/ghosty2004/samp-rs` at rev fb89010.
- wasm32 (`tools/web/build.sh`): clean, 46 MB `sa_game_bg.wasm`. The `wasm-bindgen` on PATH is 0.2.128, and the
  build needs 0.2.130 (WEB-3's `target/tools/bin`).
- Native smoke run at `9bb545a7`: `--start-in-world --fixed-step 40 --frames 300`, offscreen,
  `SA_GAME_SCRIPT_SKIP=1`. Result: **316 ok, 0 blocked, 0 original-only, 0 crashed**, 788,323 calls served
  natively, 13,375 Rust ports in the `orig!` registry, 5 addresses with a second registration (see above).

## verify-all

The run was `SA_ROOT=~/sa-engine tools/e2e/verify-all.sh int8 6,7` at `65557b1c`, DLL sha256 `ecfd0bb8…`.
INTEGRATE-6 was holding lanes 4 and 5 with its own verify-all (`i6-final`) the whole time, so this run used the
idle lanes 6 and 7. Two extra workers on lanes 2 and 3 took recordings from the same index; they ran the same
worker body against the same DLL.

The first two recordings, the grove walk and air-water, skipped their rust run. The disk guard found exactly
20 GB free on /home at that moment, because other lanes' spools were filling the disk. Both were rerun with the
same DLL (`VERIFY_NO_BUILD=1`) as `int8inf` (lane 3) and `int8aw` (lane 3). **IDENTICAL on all 13 recordings**,
17 subsystems each:

| recording | frames |
|---|---|
| grove walk (inf4) | 5200 (rerun `int8inf`) |
| air-water | 5800 (rerun `int8aw`) |
| anim-input | 3300 |
| car-ai | 5000 |
| c-bug | 3220 |
| driving | 4338 |
| menus-hud | 5700 |
| menus-map-controls | 4000 |
| pause-restart | 3700 |
| police-gang | 5200 |
| save-load | 3900 (save files identical except 750 stack-residue bytes, ledger SAVE-1, as on master) |
| weapon-combat | 3500 |
| world-cheats | 3276 |

Changes after `65557b1c`:
- `9bb545a7` removes four phase B natives. This changes the DLL bytes.
- `907043d6` touches only the L1 test.

The final head `907043d6` was rechecked as `verify-all.sh int8b 1` on driving (4338 frames) and on the grove walk
(inf4, 5200 frames), with that head's DLL (`af62e0a8…`). Result: **IDENTICAL on both**. This follows
INTEGRATE-7's `int7b` precedent.

## Open items

- WRITER-WORLD-2 and WRITER-SAMP-2 are not taken (see above). NEWGAME `newgame-1` and AUDIO-STREAM
  (`worktree-agent-acc62cadcdbd3be53`) are not taken either. Prefer AUDIO-STREAM's
  CAEDataStream / CAEStreamingDecoder ports over NEWGAME's.
- 5 doubled phase B registrations remain from master: 0x5B0440, 0x618E70, 0x84A540, 0x84E300 and 0x84F320.
- 0x60A820 needs a preserve-edx spec. It is listed in `PRESERVE_SPECS_PENDING`.
- `tools/gen/exclude_inflight.txt` lists the branches as they stood at 11:40.
- The wasm build needs wasm-bindgen 0.2.130. `~/.cargo/bin` has 0.2.128.
