# INTEGRATE-2: MVP-M, REGCONV, VERIFIER-6, GEN-ALL-2

Integration branch `worktree-agent-a77af859541ac4b2c`, from master `1c3e9e0`. Each branch was brought in as its
own diff only (`git diff <its base or merge commit> <its head>` + `git apply --3way`), never as a squash of
the snapshots of older masters it carried. List conflicts (`hooks.rs` MODULES, `lib.rs` mods, `use crate::{…}`,
`tools/e2e/l2-extras.txt`) were resolved as unions. `catalog/verify-queue.csv` and `docs/verify/QUEUE.md` were
regenerated with `python3 -I tools/verify/queue.py` after every step.

## What came from where

| commit | source | diff taken | content |
|---|---|---|---|
| 1 | WRITER-MVP-M `a84f4f4` (holds WRITER-MVP-L, MVP-K/J/I via merges) | `7cc2a0d..a84f4f4` (`7cc2a0d` is the X87-2 / HARNESS-5 snapshot its merge was based on) | the mvpi..mvpm unverified batches, two keycells readers (0x401409, 0x402195) |
| 2 | WRITER-REGCONV `dacae5f` | `5446b75..dacae5f` (after its HARNESS-5 merge, which master has) | 26 register-convention ports (regconv-render/stream/anim/ped/rw-1), `regconv_check` improvements |
| 3 | VERIFIER-6 `f751a64` | `3bf386d..f751a64` (after its RENDER-BIG-2 merge, which master has) | 28 render promotions, DrawTriPlants srand `at_site`, normaliser exclusion of 0xC96438, `port-l2.sh` extras by hook address, `verdicts-render.tsv`, l2 extras |
| — | SRAND-FIX base | nothing | its first two commits (`d096685`, `7cc2a0d`) are WIP snapshots of HARNESS-5 / X87-2, already on master as `909b776` |
| 4 | GEN-ALL-2 `f722a1f` | tooling only (`tools/gen/*`, `docs/arch/gen-all.md`, the `generated` registrations, `specs_generated` hook-up) | generated output not merged by hand: `tools/gen/inflight.py` and `tools/gen/patterns.py` were rerun on the integrated tree |
| 5 | fix | — | 0x4939F0 registered once (below) |
| 6 | SRAND-FIX `df75d3a` | `7cc2a0d..df75d3a` | `at_site` for the remaining srand calls (below) |

GEN-ALL-2 regenerated: 2,549 candidates, 984 generated, **852 hooked** (22 batches, 25 L1-pure), 54 in-flight
exclusions. Against the branch's 860 hooks, 8 addresses dropped and none added: 0x45B160, 0x45C440, 0x464BD0,
0x464D70, 0x474800, 0x4CB4C0, 0x5DABC0, 0x5DAC00 (registered by now, or by an in-flight branch). The
0x4CB4C0 thunk (which ignored the ESI rect argument) is gone; REGCONV's `_rwD3D9RasterClearRect`
(`crates/sa-rwcore/src/d3d9rastrender.rs`) is the only registration. The counts table in
`docs/arch/gen-all.md` still describes the branch's GEN2 run (860).

## Duplicate registrations settled

| address | function | kept | dropped |
|---|---|---|---|
| 0x70A040 | CShadows::CastShadowEntityXYZ | REGCONV `sa-render/src/cast_shadow_entity.rs` | MVP-M `shadow_cast_entity_xyz.rs` (mvpm-render-4) |
| 0x5DC510 | CPlantMgr::_ColEntityCache_Update | REGCONV `plant_col_entity_cache.rs` | MVP-M's file of the same name (mvpm-render-3) |
| 0x711760 | CStencilShadows::RegisterStencilShadows | REGCONV `stencil_register.rs` | MVP-M `stencil_shadow_register.rs` (mvpm-render-5) |
| 0x5DC290 | CPlantLocTri::Add | REGCONV (`plant_loc_tris.rs`) | MVP-M `plant_loc_tri_add.rs` (mvpm-render-3) |
| 0x70FAE0 / 0x710310 | CStencilShadows::RenderForVehicle / RenderForObject | MVP-M `stencil_shadow_volumes.rs` | (REGCONV has none) |
| 0x4D2E10 | FrameUpdateCallBackOffscreen | REGCONV (`sa-anim/src/rp_anim_blend.rs`, regconv-anim-1) | MVP-M `offscreen_frame_update.rs` (mvpm-anim-1; same logic) |
| 0x4939F0 | CTheScripts::ProcessAllSearchLights | full port `script_search_lights.rs` (mvpk-script-2) | the std6-script-1 partial with fallback in `the_scripts_init.rs` (MVP-K had left the full port unregistered, which failed sa-script's `list_is_complete_and_unique`) |

0x561FE0 (`CWanted::ClearQdCrimes`, mvpl-ped-1) and 0x561FF0 (its loop body, regconv-ped-1) are different
entries and both stay.

## Verification

SRAND-FIX's own commit `df75d3a` arrived after the first pass and is integrated as its own commit (`7cc2a0d..df75d3a`):
`at_site` for the srand calls in carctrl4 (x4), CPPTriPlantBuffer::Flush (0x5DB120) and the enex
CreateEffects (0x5339F5); its DrawTriPlants change (0x5DADD2) was already identical to VERIFIER-6's.

On head `a109d55` (the tree before this note):

- `cargo test -p sa-hook --lib` 58 passed; `-p sa-script --lib` 8 passed; `-p sa-core --lib` 70 passed.
- `cargo test -p sa-oracle --test l1 --release`: 18 passed, 1 ignored (953 s).
- i686 DLL build (`sa-launcher`, `sa-hook`, by verify-all): OK.
- `tools/e2e/verify-all.sh int2 4,5,6` (SA_ROOT = the main checkout): **IDENTICAL on all 12 recordings**
  (grove walk inf4 plus the 11 corpus recordings). No bisection was needed.

Verify queue after integration: 2,086 unverified hooks in 222 batches.
