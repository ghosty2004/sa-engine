# INTEGRATE-9: master and the INTEGRATE-8 tree combined

Branch `worktree-agent-ab58bdb347d7ad153`. It starts from the INTEGRATE-8 tree
(`worktree-agent-ac8a20a070c956b08`, `f94f1ee3`: web build, SA:MP natively and in the browser, driving, climbing,
browser audio, SAMP-4 / SAMP-START / SAMP-UI-2; based on master `b239e888`) and replays master's commits
`b239e888..284fd692` on top, in order. master can fast-forward to it.

## How

master's 15 commits since `b239e888` were cherry-picked in order:

| master | here | content | conflicts |
|---|---|---|---|
| `a9f08b48` | `03e20313` | CONTINUE.md branch list | none |
| `95b9f784` | `c61e9781` | L1 evidence 0x70FA70 / 0x6FD7C0 (status rows) | queue files |
| `9b27b6d4` | `f320ebdb` | patterns.py: void forwarders (VERIFIER-10 8ba7aa9) | none |
| `cc4e8e36` | `187403da` | 12 gen2 + 11 writero promotions (VERIFIER-10 2fc5e63) | queue files |
| `5eb94a81` | `1d56be42` | audio thread `Start` ports resume through the original call sites | none |
| `388eeac5` | `932ea486` | L3-batch status of the 157 SA:MP-priority batches | queue files |
| `8a7a9cdb` | `63859ea5` | 370 SA:MP-priority promotions (VERIFIER-11) | queue files |
| `7e13e332`, `bb13f5eb` | `671bcb9b`, `b68cfa93` | integration-6.md | none |
| `cff47307` | `fd4a5a16` | catalogue states of the INTEGRATE-6 promotions | none |
| `2286dfe2` | `cf78cf2e` | CONTINUE.md | none |
| `79b0bfe2`, `a881a1e5` | `6cd40a5d`, `d08cbd86` | README, getting-started guide | none |
| `737f7e76` | `8d57ff7b` | samp-rs imported as `samp-rs/` | none |
| `284fd692` | `f0d0851f` | samp-rs takes sa-samp-api / sa-net by path; sa-samp-api v3 | none |

- Queue files (`catalog/verify-queue.csv`, `docs/verify/QUEUE.md`) were kept at the INTEGRATE-8 side during the
  picks and regenerated once at the end. `catalog/verify-status.csv` merged without conflicts every time.
- The 393 promotions (370 + 12 + 11) applied cleanly to the INTEGRATE-8 sources: every promoted hook is in a file
  both trees share.
- **SA:MP crates.** sa-samp-api was already identical on both sides (INTEGRATE-8 carries SAMP-4's API v3, master
  raised its copy to the same v3). `284fd692` added an `Unsupported` `force_weather` stub to sa-samp's
  `world.rs`, next to the write side's real implementation; `e70899ba` keeps the real one (the INTEGRATE-8
  file). sa-net unchanged. `samp-rs/` comes from master.

## samp-rs in the repository

- `cb717f23`: sa-game's `samp-client`, `samp-proto` and `samp-saa` dependencies are `path =
  "../../samp-rs/crates/…"`; the workspace `[patch."file:///home/ghosty2004/sa-engine"]` is gone (samp-client takes
  sa-samp-api and sa-net by path already). `Cargo.lock` lost the four `git+file:///…samp-rs` sources.
- `dd815843`: every document and script points at `samp-rs/`:
  - `tools/e2e/samp/native-handshake.sh` and `proxy-handshake.sh` default `SAMP_RS_DIR` to `samp-rs/` (no more
    scratch copy with the step-4 patch: the test is in `samp-rs/crates/samp-client/tests/native_handshake.rs`).
  - The auth table (third-party, never committed) defaults to `$SA_ROOT/re/samp/auth_table.txt`
    (`AUTH_TABLE` / second argument overrides) in native-join, samp-rs-ab, the handshake scripts and
    `tools/web/samp-demo.sh`. A copy was placed there in the main checkout. GETTING-STARTED §3 lists it.
  - `docs/samp/samp_usage.py` reads `samp-rs/` and the patch list from `samp-rs/docs/re/data/` (gitignored) or
    `re/samp/gta-patches-r4-annotated.csv`. Its output was **not** regenerated here: with the old source
    (`~/samp-rs`, branch main) and the new (`samp-rs/`, adapter fb89010) the address set and
    `required-functions.csv` are identical; only source line numbers / file names (the adapter's
    `adapter/original/*.rs`), 6 more "outside game sections" constants and a few extra `global-read` uses
    differ. Both differ from the committed outputs by the same large amount, which is catalogue drift since the
    last run, not this change.
  - samp-rs's own e2e scripts default `SAMP_RS_ROOT` to their tree instead of `/home/ghosty2004/samp-rs`.
  - Active docs rewritten (CLAUDE.md, PLAN.md rules, docs/samp README / game-adapter / overlay / web-proxy,
    sa-samp-api `config.rs`, tools/specs/BRIEF.md); historical ones keep their text with a note (FEASIBILITY,
    re/cat16-fallthrough, samp/native-plan, samp/dynamic); ROADMAP gains the 2026-11-12 decision. WORKLOG and
    earlier integration-*.md are records and stay as they are.
- `git grep -E '~/samp-rs|/home/ghosty2004/samp-rs|samp-rs-adapter|file:///'` outside `samp-rs/` now finds only
  those historical records (plus CLAUDE.md's statement that the external repository is no longer used).

## Generator promotion rule

The open item from integration-6.md. `catalog/state.csv` has `implemented` for every registered hook an
integration found ("written (hook or native registration on master)"), and `tools/gen/ai_patterns.py` gave
`unverified` only to hooks in `unknown`/`specified`. INTEGRATE-8's regeneration had therefore **already dropped
`unverified` from 554 PORT-AI-7 generated hooks** (batches ai7-1..ai7-5 in `gen_*.rs`) on `f94f1ee3`; they were
in its verify-all default set, but they were never promoted.

`c28e0198`: a hook listed in `tools/gen/ai_patterns_unverified.txt` stays `unverified` until its state is
L1/L2/L3. Unlisted hooks in `unknown`/`specified` still get a batch; the pre-split AI-GEN hooks (unlisted,
`implemented-needs-L2`, generated without `unverified` since AI-GEN) are unchanged. `patterns.py` already
promotes only on L1/L2/L3 or `patterns_verified.txt` (its `implemented` clause only stops its own output from
hiding candidates), and `inflight.py` does not read states; both needed no change.

Regenerated `inflight.py`, `ai_patterns.py`, `patterns.py`; all three `--check` clean afterwards. Comparison of
the `unverified` attribute per address (every `#[hook]`/`#[native]` in `crates/`):
- against master `284fd692`: **0** addresses gained or lost `unverified`;
- against INTEGRATE-8 `f94f1ee3`: 554 gained (the ai7 hooks above) and 393 lost (exactly master's 370 + 12 + 11
  promotions with their L2/split/L1 evidence).

Queue (`30df2074`, `python3 -I tools/verify/queue.py`): **2328 unverified hooks in 343 batches**.

## One registration per address

`cargo test --release -p sa-hook --lib`: 58 passed. Native sa-game reports 5 addresses with a second
registration, the known pre-existing ones: 0x5B0440, 0x618E70 (`std7_natives` + `#[native]`), 0x84A540, 0x84E300,
0x84F320 (`static_inits` + PORT-VEH-3 hooks).

## Tests

- Lib tests (release): sa-hook 58, sa-script 9, sa-core 73, sa-rw-wgpu 58, sa-samp 18, sa-samp-api 1,
  sa-samp-ui 39, sa-audio 11: all pass.
- `cd samp-rs && cargo test --workspace`: 638 passed, 0 failed, 0 ignored.
- `cargo test -p sa-oracle --test l1 --release`: 18 passed, 1 ignored, 0 failed (1346 s, beside verify-all; no
  hang).
- sa-oracle, satool, sa-oracle-format rebuilt (host release); `i686-pc-windows-gnu` release sa-launcher + sa-hook:
  clean.
- sa-game `i686-unknown-linux-gnu` release, with and without `--features samp`: clean (samp from `samp-rs/` by
  path).
- wasm32: `tools/web/build.sh` and `FEATURES=samp tools/web/build.sh`: clean (46.1 MB / 46.3 MB
  `sa_game_bg.wasm`; needs wasm-bindgen 0.2.130 in `target/tools/bin`).
- Native smoke run (`--start-in-world --fixed-step 40 --frames 300`, offscreen, `SA_GAME_SCRIPT_SKIP=1`):
  **316 ok, 0 blocked, 0 original-only, 0 crashed**, 777,279 calls served natively, 13,375 Rust ports in the
  `orig!` registry, the 5 doubles above.

## verify-all

`SA_ROOT=~/sa-engine tools/e2e/verify-all.sh int9 4,5` at `30df2074` (the code head; the commits after it touch
docs only), DLL sha256 `8d871e1c…`, run beside the L1 test. No recording was skipped by the disk guard (71–74 GB
free throughout). **IDENTICAL on all 13 recordings**, 17 subsystems each:

| recording | frames |
|---|---|
| grove walk (inf4) | 5200 |
| air-water | 5800 |
| anim-input | 3300 |
| car-ai | 5000 |
| c-bug | 3220 |
| driving | 4338 |
| menus-hud | 5700 |
| menus-map-controls | 4000 |
| pause-restart | 3700 |
| police-gang | 5200 |
| save-load | 3900 (save files identical except 750 stack-residue bytes, ledger SAVE-1, as before) |
| weapon-combat | 3500 |
| world-cheats | 3276 |

The default set of this run is master's verified set plus INTEGRATE-8's; the 554 ai7 hooks are unverified again and
were not in it (INTEGRATE-8's int8 run had them in). No bisect was needed.

## Open items

- Branches still outside this tree: see CONTINUE.md (SAMP-FX, INTRO-1, SCRIPT-5, AUDIO-STREAM, NEWGAME,
  WRITER-WORLD-2, WRITER-SAMP-2, SAMP-SCM, HELI-CONE).
- 5 doubled phase B registrations (above); 0x60A820 still needs a preserve-edx spec (`PRESERVE_SPECS_PENDING`).
- `docs/samp/samp_usage.py` outputs are stale against the catalogue (not regenerated here, see above).
- `catalog/progress.md` was not regenerated.
