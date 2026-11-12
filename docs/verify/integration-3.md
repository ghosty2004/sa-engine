# INTEGRATE-3: STD-7/8/9 (native boot into the world), PORT-MENU, WRITER-CLUMP, WRITER-INTRO

Integration branch `worktree-agent-a1ec5bfbbfdf8dd8b`, from master `4d484a0`. Not taken: WRITER-O, PORT-SAVE,
STD-10 (still running; their registered addresses are generator exclusions through `tools/gen/inflight.py`).

## Method

STD-9 (`worktree-agent-a5944be11b642b989`, head `622f85e`) holds STD-7 (`407b907`) and STD-8 (`51b8fcd`) as
ancestors and carries its own picks of INTEGRATE-2's sources (MVP-M, SCRIPT-4, VERIFIER-6, SRAND-FIX, REGCONV,
GEN-ALL-2, the search-light registration) plus WRITER-MVP-N, PORT-MENU and WRITER-CLUMP. Its merge base with master
(`7fd59f2`) is far older than the real split: STD-7 starts from the STD-6 tip `21b0439`, whose tree master integrated
as `309d8fe` (the two trees differ only in the catalogue, WORKLOG, `std6-ports.csv` and two lines of
`sa-oracle/tests/l1.rs`, all master-side). So the merge is a three-way merge with `21b0439` as the base
(`git merge-tree --merge-base=21b0439 master STD-9`), resolved per file:

* list files (`hooks.rs` MODULES / NATIVE_MODULES, `lib.rs` mods): union, deduplicated (`crate::transform_vectors`
  appeared twice from the two placements of the same module);
* generated files (`generated*.rs`, `*_generated.rs`, `verify-queue.csv`, `QUEUE.md`): STD-9's, then regenerated
  (below); `tools/gen/patterns.py`: STD-9's (it counts path-qualified `#[native]` registrations);
* `sa-macros/src/lib.rs`: STD-9's (the same HARNESS-5 / X87-2 merge as master's, plus `dead_code` on the shims);
* `tools/e2e/l2-extras.txt`: STD-9's (the textual merge duplicated the PORT-CAM-3 block; STD-9's file already holds
  master's lines and adds PORT-MENU's);
* `docs/arch/x87-2-verdicts.tsv`: master's verify-all line kept.

After resolution every crate and tool file equals STD-9's except the removals below; the catalogue
(`state.csv`, `progress.md`), WORKLOG and ROADMAP stay master's. Master's own crate changes since `21b0439` were all
already on STD-9 as picks.

WRITER-INTRO (`worktree-agent-a9ddddfa3348a8811`, own commits `4627242..dd7849a` on STD-8 `51b8fcd`) was merged on
top with `51b8fcd` as the base: new files only, plus union of the sa-ai / sa-anim MODULES and mods.

## What came from where

| commit | source | content |
|---|---|---|
| `da9de7f` | STD-7 + STD-8 + STD-9 (with PORT-MENU, WRITER-CLUMP, WRITER-MVP-N) | sa-game / sa-platform native boot, `--start-in-world`, run-all static initialisers, `CRope::Update`, the in-world render chain, std7/8/9 batches (20 / 21 / 32 hooks plus natives), PORT-MENU (menu-1..3), WRITER-CLUMP (clump-1), MVP-N |
| `5e0f61b` | WRITER-INTRO | intro-ai-1..5, intro-anim-1: CEventHandler responses, ped collision responses, `ComputeEntityBoundingBoxCornersUncached`, cutscene memory |
| `519494c` | fix + regeneration | `tools/gen/patterns.py` fix (below), `exclude_inflight.txt` and the generated hooks regenerated |
| `e5fa9cf` | regeneration | `catalog/verify-queue.csv`, `docs/verify/QUEUE.md` |

## Duplicate registrations settled

| address | function | kept | dropped |
|---|---|---|---|
| 0x4D2E10 | FrameUpdateCallBackOffscreen | STD-7 `sa-anim/src/frame_update.rs` (std7-anim-1; the native boot runs it) | REGCONV `sa-anim/src/rp_anim_blend.rs` (regconv-anim-1). Both unverified; this reverses integration-2.md's choice, as STD-9 did |
| 0x54EEA0 | TransformVectors | MVP-K's hook `sa-render/src/transform_vectors.rs` | STD-6's native in `mvpb_r1.rs` (STD-9's resolution, 489d318) |
| 0x4939F0 | CTheScripts::ProcessAllSearchLights | the full port `script_search_lights.rs` (as master) | — |

Left as they are (STD-10 settles them): 0x6F9DE0 `CPlantSurfPropMgr::GetSurfProperties` and 0x407F20
`CStreaming::LoadInitialVehicles` (hook plus native in `sa-core/gameinit.rs`; the registry uses the hook).

The three generated files that only master's GEN-ALL-2 run had (`sa-cam/camera_generated.rs`,
`sa-core/generated_ctors.rs`, `sa-render/proc_object_man_c_generated.rs`, the last duplicating 0x5A3120 / 0x5A3130 of
`generated_thunks.rs`) are removed, as STD-9's regeneration did.

## Generator

`tools/gen/patterns.py` on the merged tree first produced **0** hooks (111 candidates): INTEGRATE-2's catalogue record
(`4d484a0`) sets the generated hooks to `implemented` with the evidence "written (hook or native registration on
master)", which carries no `GEN2` tag, so the generator took every one of its own hooks for a port made elsewhere.
Fix: a row at `implemented` whose address the generator's own files register stays a candidate
(`generated_addrs`). Regenerated with `python3 -I tools/gen/inflight.py` (98 in-flight exclusions) and
`.venv/bin/python -I tools/gen/patterns.py`: 2,415 candidates, 952 generated, **841 hooked** (22 batches, 25
L1-pure); `--check` clean. Against STD-9's 845 hooks, 4 dropped, none added: 0x5D24B0, 0x5D2560 (ped setters),
0x5D2980 `CAutomobile::Load`, 0x5D47E0 `CAutomobile::Save` (registered on an in-flight branch); 0x6A44D0
`CAutomobile::PopBootUsingPhysics` moved from `automobile_generated.rs` to `generated_small.rs`. No generated hook
duplicates a hand port (address scan of every `#[hook]` / `#[native]` attribute: the only repeats are the
`regconv_tests.rs` test hooks and the `sa-macros` doc example).

`tools/gen/ai_patterns.py --check` and `tools/gen/audio_patterns.py --check` report differing files (gen_clones,
gen_ctors, gen_dtors, gen_leaves, gen_subtasks, gen_virtuals; gen_bodies, gen_dtors, gen_leaves); not part of this
integration's scope, left as they are.

Verify queue after integration: 2,236 unverified hooks in 265 batches.

## Verification

On head `e5fa9cf`:

- `cargo test -p sa-hook --lib` 58 passed; `-p sa-script --lib` 8 passed; `-p sa-core --lib` 70 passed.
- `cargo test -p sa-oracle --test l1 --release`: 18 passed, 1 ignored (654 s).
- i686-pc-windows-gnu DLL build (`sa-launcher`, `sa-hook`): OK. i686-unknown-linux-gnu `sa-game` release build: OK.
- `tools/e2e/verify-all.sh int3 4,5` (SA_ROOT = the main checkout, satool / sa-oracle built in this worktree):
  **IDENTICAL on all 13 recordings** (grove walk inf4 plus the 12 corpus recordings, save-load included). air-water,
  weapon-combat and world-cheats were first skipped by the e2e disk guard (/home briefly under 20 GB, other agents'
  builds) and were rerun on the same DLL as `int3b`: IDENTICAL. No bisection was needed; nothing dropped.

### Native smoke run (offscreen)

`SA_GAME_PRESENT_GAME=1 SA_GAME_SCRIPT_SKIP=1 SA_GAME_STUB=<platform.md 15.4's ten stubs> sa-game --game wine/gta-sa
--frames 120 --start-in-world --present-dir …` (i686 release, offscreen, no window):

| build | stubs | result |
|---|---|---|
| integrated | 15.4's ten | `START_MISSION 2` skipped at frame 29, in-world frames presented (Grove Street, HUD, help text), 7 presents; from frame 36 every rendering iteration blocks on the vehicle low-detail render callback 0x7331E0 (`RENDER_VEHICLE_REALLY_LOW_DETAIL_CB`) |
| STD-9 head `622f85e` (built for comparison) | 15.4's ten | the same block at 0x7331E0 from the same frames, 8 presents: not an integration regression; platform.md 15.4's 79 of 120 does not reproduce today |
| integrated | + `0x7331e0=0/0` | 66 in-world presents; from frame 87 blocks on `CColSphere::IntersectSphere` 0x410090 |
| STD-9 head | + `0x7331e0=0/0` | 56 presents; blocks on `ProcessRiderAnims` 0x6B7280 (18) and `CQuaternion::Get` 0x59C080 |

The integrated tree runs further than STD-9 with the same stubs (WRITER-INTRO's responses now run natively). Next
native gaps: 0x7331E0 (add it to the exploration stub list or port it with the hi-detail callbacks 0x733F80 /
0x733240) and 0x410090.

## Open items

- 0x6F9DE0 / 0x407F20 hook-plus-native duplicates: STD-10.
- The ai / audio generators' `--check` differences (above).
- platform.md 15.4's stub list needs 0x7331E0 for the in-world frames to continue.
