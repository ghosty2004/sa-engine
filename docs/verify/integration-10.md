# INTEGRATE-10: SAMP-FX on master

Branch `worktree-agent-a623a7f9d5dd59528`, on master `b89db5ae` (INTEGRATE-9). It adds SAMP-FX
(`worktree-agent-ab013435f0f3841aa`, head `e6e0528e`): its own commits after WEB-3 `be0762be`, which include its
picks of WEB-AUDIO (latency), SAMP-SCM, HELI-CONE and WRITER-SAMP's search-light ports. master can fast-forward to it.

## How

SAMP-FX's 16 commits `be0762be..e6e0528e` were cherry-picked in order:

| SAMP-FX | here | content | notes |
|---|---|---|---|
| `bca82a34` | `9963bf7d` | web audio: shorter, adaptive output latency (WEB-AUDIO `e3336f89`) | clean |
| `da13b5d0` | `89a78023` | 12:00 clock while connecting; samp's main.scm / script.img behind `SA_SAMP_SCM` (SAMP-SCM `646e0f7b`) | clean |
| `cdf47fde` | `8d6f6b23` | set_ped_model, play_anim, stop_anims, anim blocks; SetPlayerSkin, ApplyAnimation, ClearAnimations, PlayerPlaySound | clean |
| `28df3c74` | `0ed70d17` | ported `CTaskManager::HasTaskSecondary` from the scripted-task record update; native-join `EXTRA_CMDS` | clean |
| `ea1da7d7` | `65ce7f3d` | `serve.py --samp-dir`; samp_assets reads sampgui.png / mouse.png through the file layer | clean |
| `b4678c4d` | `7f8e67c4` | frontend-only mission sounds (class-selection arrows) | clean |
| `2d3b563c` | `378ee4bf` | game clock toggle (`RuntimeToggle::GameClock`) | clean |
| `0f1b633b` | — | WRITER-SAMP search-light / checkpoint ports | **skipped**: already on master (`crates/sa-script/src/search_lights_checkpoints.rs` byte-identical, registered once) |
| `de34dd5a` | `88add7e2` | startup.md | clean |
| `3d3ccc15` | `cdf2224b` | hands up, phone, car enter / leave tasks; SetPlayerSpecialAction 10 / 11 / 13 | clean |
| `609f18cb` | `86de5ff7` | samp-demo.sh `SAMP_DIR` | clean |
| `27489018` | `9c09e9df` | sa-samp tests for skin / anim / hands-up | clean |
| `cd5f4a4c` | `a4a32285` | `CHeli::SearchLightCone` 0x6C58E0 (HELI-CONE, batch samp-heli-1) | clean |
| `bb4f327d` | `f111e7fe` | samp mode serves samp's main.scm and script.img by default (the "Fat +" fix; `SA_SAMP_SCM=0` keeps GTA's) | clean |
| `b5393f8d` | `8a9b068d` | give-weapon like samp (model first), SetPlayerArmedWeapon | clean |
| `e6e0528e` | `81ed6649` | sampcov class-selection camera | clean |

Then:
- `dade19d6`: `tools/web/samp-demo.sh` defaults `SAMP_DIR` to `$SA_ROOT/re/samp/client` when it exists (INTEGRATE-9's
  convention: third-party SA:MP files under `re/samp/`, gitignored; `SAMP_DIR=` serves the game folder alone).
  SAMP-FX's script changes needed nothing else: they already use the auth table variable `AUTH`
  (`$SA_ROOT/re/samp/auth_table.txt`) and no samp-rs path. `git grep '~/samp-rs\|/home/ghosty2004/samp-rs'` over
  `tools crates web` finds nothing.
- `d45ba8f4`: generated files regenerated. `tools/gen/exclude_inflight.txt` (`python3 -I tools/gen/inflight.py`);
  `ai_patterns.py --check` and `patterns.py --check` clean (no generated file changed); queue
  (`python3 -I tools/verify/queue.py`, `--check` clean): **2329 unverified hooks in 344 batches** (INTEGRATE-9 had
  2328 / 343; the new one is SearchLightCone, batch samp-heli-1).

## One registration per address

The SearchLightCone / search-light / checkpoint addresses (0x6C58E0, 0x485E00, 0x493000, 0x4934F0, 0x4935A0,
0x4936C0, 0x493E30) are registered once each. Native sa-game reports the same 5 second registrations as
INTEGRATE-9 (0x5B0440, 0x618E70, 0x84A540, 0x84E300, 0x84F320) and no new one.

## Tests

- Lib tests (release): sa-core 73, sa-hook 58, sa-samp 20, sa-samp-ui 39, sa-script 9: all pass.
- `cd samp-rs && cargo test --workspace`: 638 passed, 0 failed, 0 ignored.
- sa-oracle, sa-oracle-format (satool) rebuilt (host release); `i686-pc-windows-gnu` release sa-launcher + sa-hook:
  clean.
- sa-game `i686-unknown-linux-gnu` release, with and without `--features samp`: clean.
- `FEATURES=samp tools/web/build.sh`: clean (46.5 MB `sa_game_bg.wasm`).
- Native smoke run (no samp; `--start-in-world --fixed-step 40 --frames 300`, offscreen, `SA_GAME_SCRIPT_SKIP=1`):
  **316 ok, 0 blocked, 0 original-only, 0 crashed**, 778,723 calls served natively, 13,376 Rust ports in the
  `orig!` registry (+1: SearchLightCone).
- SA:MP native join (`tools/e2e/samp/native-join.sh`, `SAMP_LOCK=int10.lock`, `SAMP_PORT=7836`,
  `SA_SAMP_DIR=re/samp/client`, beside the running 8644 demo stack): samp.saa's loadscs.txd, main.scm and
  script.img served; connected → spawned; every sampcov command applied; **1616 ok, 0 blocked, 0 crashed**.
  The connecting PNGs show only samp's chat lines ("SA-MP 0.3.7-R4 Started", "Connecting to 127.0.0.1:7836...")
  and **no "Fat +" help box**; after `weapon|24|100` the HUD shows the Desert Eagle icon.

## verify-all

`SA_ROOT=~/sa-engine tools/e2e/verify-all.sh int10 4,5` at `d45ba8f4` (the code head; the commit after it touches
docs only), DLL sha256 `51e989ab…`. No recording was skipped by the disk guard (68 GB free throughout).
**IDENTICAL on all 13 recordings**, 17 subsystems each:

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

SearchLightCone (0x6C58E0) is unverified (batch samp-heli-1) and was not in the default set. No bisect was needed.

## Open items

- Branches still outside this tree: INTRO-1, SCRIPT-5, AUDIO-STREAM, NEWGAME, WRITER-WORLD-2, WRITER-SAMP-2 (see
  CONTINUE.md). SAMP-SCM and HELI-CONE are in through SAMP-FX.
- 5 doubled phase B registrations (above); 0x60A820 still needs a preserve-edx spec.
- `tools/gen/inflight.py` matches only `#[hook(` / `#[native(`, not `#[sa_core::native(`: 0x4413C0
  (`CGameLogic::Remove2ndPlayerIfPresent`) is listed as an exclusion although this tree registers it.
- `docs/samp/samp_usage.py` outputs and `catalog/progress.md` are still stale against the catalogue.
