# INTEGRATE-6 / VERIFIER-11: PORT-AI-7, VERIFIER-10, and the SA:MP-priority verification

Integration branch `worktree-agent-aa4c0e8384f020c37`, from master `9f712b9`, rebased onto master `76c533d`
(SAMP-3 `18671e6` + `76c533d`) before the all-ports runs.

## Part 1: what came from where

| commit | source | content |
|---|---|---|
| `0107631` | PORT-AI-7 `f9328ce` (pick) | 234 more generated AI hooks (`tools/gen/ai_patterns.py`: stale class exclusions lifted, virtual tail jumps and CRT callees, rustfmt, natives); `tools/gen/ai_patterns_unverified.txt` (batches ai7-1..ai7-5) |
| `fb5f07a` | PORT-AI-7 `462c48d` (without queue files) | 70 hand ports (ai7-6..ai7-10) incl. native 0x68D910 (`CTaskComplexPolicePursuit::CreateSubTask`, protection code reads its entry) and its `NATIVES` entry in sa-ports |
| `1e4d41e` | PORT-AI-7 `5e4797c` (without queue files) | 25 SA:MP-priority hand ports (ai7-11, ai7-12) |
| `1a6154d` | regeneration | `python3 -I tools/verify/queue.py`: 2,000 unverified hooks in 277 batches |
| `4d0c3b7` | VERIFIER-11 | L1 evidence for 0x70FA70 / 0x6FD7C0 (status rows) |
| `570c361` | VERIFIER-10 `8ba7aa9` (patterns.py + patterns_verified.txt as a diff) | forwarders returning a void port's EAX after a plain call are generated void (6 functions); 7 vanished entries leave patterns_verified.txt; generated files regenerated here (identical to the source commit's) |
| `e9f2a1a` | VERIFIER-10 `2fc5e63` (source / doc / extras / verdicts as a diff) | 12 gen2 promotions (patterns_verified.txt) and 11 writero hooks without `unverified`; `l2-extras.txt` lines (union with this tree's); `docs/verify/verdicts-gen2b.tsv`; gen-all.md note; `catalog/verify-status.csv` merged by address (309 rows applied, 0 conflicts); generated files and queue regenerated (1,977 unverified hooks in 272 batches) |
| `8e5a0fb` | regeneration | queue after the rebase (writer commit ids only) |
| `b09007a` | VERIFIER-11 fix | audio thread `Start` ports resume through the original call sites (below) |

List files: `crates/sa-ai/src/hooks.rs` MODULES and `lib.rs` module list are the union (12 new modules);
`crates/sa-ports/src/lib.rs` NATIVES gains `sa_ai::police_pursuit::NATIVES`; `tools/e2e/l2-extras.txt` is the union.
Generators: `tools/gen/ai_patterns.py --check`, `tools/gen/inflight.py --check`, `tools/gen/patterns.py --check`
all clean on the integrated tree, before and after the rebase; no generated hook duplicates a hand port; one
registration per address (the only repeated `#[hook(addr` in `crates/` is the 0x407180 doc example in sa-macros).
Queue files were regenerated, never merged.

### Part 1 verification

On `8e5a0fb` (rebased, VERIFIER-10 included):

- `cargo test -p sa-hook --lib` 58 passed; `-p sa-ai --lib` 13 passed; `-p sa-core --lib` 72 passed.
- `cargo test -p sa-oracle --test l1 --release`: 18 passed, 1 ignored (697 s; before the rebase 1,546 s, same result).
- host `sa-oracle` / `satool` rebuilt (release); i686-pc-windows-gnu `sa-launcher` + `sa-hook`: OK;
  i686-unknown-linux-gnu `sa-game` release: OK.
- `tools/e2e/verify-all.sh int6 4,5` (SA_ROOT = `~/sa-engine`, DLL sha256 `041334bb29d8eafe…`): **IDENTICAL on
  all 13 recordings** (grove walk, air-water, anim-input, car-ai, c-bug, driving, menus-hud,
  menus-map-controls, pause-restart, police-gang, save-load, weapon-combat, world-cheats); save-load: save files
  identical except stack residue (750 bytes, ledger SAVE-1).

### VERIFIER-10's 23 promotions (evidence: `docs/verify/verdicts-gen2b.tsv`, verify-all v10-promote IDENTICAL 13/13)

```
0x407D30 L2 gen2-pop-1
0x407DC0 L2 gen2-pop-1
0x45CD10 L2 gen2-world-1
0x464F50 L2 gen2-script-1
0x4A1940 L2 writero-render-1
0x4A2C40 L2 writero-render-1
0x4AE1A0 L2 gen2-pop-1
0x4F6420 L2 writero-audio-1
0x507400 L2 gen2-world-1
0x509A30 L2 gen2-cam-1
0x5A2CB0 L2 gen2-phys-1
0x5DE340 L2 gen2-ped-1
0x7226D0 L2 gen2-rw-5
0x732660 L2 gen2-rw-5
0x733630 L2 gen2-rw-5
0x7EB5C0 L2 writero-rw-1
0x7EB6A0 L2 writero-rw-1
0x7EB700 L2 writero-rw-1
0x7EB760 L2 writero-rw-1
0x7EB7C0 L2 writero-rw-1
0x7EBBB0 L2 writero-rw-1
0x7EC220 L2 writero-rw-1
0x7F1930 L2 writero-rw-1
```

## Part 2: SA:MP-priority verification (VERIFIER-11)

Scope: every unverified hook on master whose address is in `catalog/samp-priority.csv`, batches other than
gen2-* / writero-* (VERIFIER-10) and std6..9 / intro / clump / mvpn (VERIFIER-9): **737 hooks in 157 batches**
(ai7-*, boot-*, std4-*, std5-*, mvpa..mvpm, regconv-*, harness5-1, popw4, save leftovers), 1,111 hooks in those
batches in all.

### L3: union of the 157 batches

`l3.sh --rust unverified --fixed verified` with `SA_UNVERIFIED=@` the 157 batch ids, every recording:

- first pass (head `8e5a0fb`): the grove walk **diverged at frame 595 (objects)**, culprit
  `CAEStreamThread::Start` (0x4F11F0) alone; the rest of the set matched. Fixed in `b09007a` (below).
- with the fix (`b09007a`): **L3 MATCH on all 13 recordings**, 3 probes each, no culprit. Recorded as
  `L3-batch` for the 1,104 hooks of those batches without L1/L2 status (`b86afe2`).

### Bug fixed

`b09007a` — `CAEStreamThread::Start` (0x4F11F0) and `CAESmoothFadeThread::Start` (0x4EEA10), batch std4-audio-1:
the ports called `ResumeThread` through the IAT cell 0x8580A4. Determinism mode (sa-hook `detaudio.rs`) redirects
the original's own `call [ResumeThread]` instructions at 0x4F11F7 / 0x4EEA1A to shims that record `this` (ECX)
and keep the thread suspended, stepping its loop from `CAEAudioHardware::Service`; the ports bypassed that, so
the stream thread ran free and the replay diverged. Phase A now calls through those sites (`orig!` `site` form,
thiscall so ECX carries `this`, one stack argument as `ResumeThread`'s stdcall); phases B/C keep the import call.

### L2 / split

Lists of at most 50 hooks in SA:MP rank order (`samp-priority.csv`: hits, then non-init, then row), on
pause-restart (which covers boot, new game and the grove walk: 680 of the 737) and on grove-walk / driving /
car-ai / weapon-combat / air-water for the rest; L2 lists for route L2, split lists for route split, for
destroyers (Destroy/Delete/Remove/Shutdown/dtor names: an L2 double run frees twice) and for hooks called at
boot (D3D/RW/audio/loader batches: an L2 double run of a resource creator crashed the boot in four lists);
`SPLIT_STACKFILL=4096`, `SPLIT_PER_FRAME=20`, `L2_PER_FRAME=2` (8..16 in the fix runs), `L2_MAX=200`,
`L2_SPOOL_MAX=1G`. 46 runs (tags `i6-*`), all on lanes 4/5. Per-hook verdicts from `results.tsv`, or from the
host `l2-report` / `split-report` by address where the run ended early or two hooks share a name.

`tools/e2e/l2-extras.txt` additions (all VERIFIER-11 lines are commented):

- 150 hooks whose only finding was undeclared writes to static memory with 0 mismatches get
  `ADDR@0x8A4000:0x3FA000:D` (the game's .data/.bss as a declared write range, as `Game::InitialiseGame` has;
  D regions are not snapshotted, every byte written there is still compared as a window write); all re-run
  (fix runs) and passed.
- heap/pool regions: `CCollisionData::Copy` (this and source, 0x30 bytes), `CPool<CEntryExit>::New` (byte map),
  the AI-7 car task constructors (CReferences `aRefs`/free head and the vehicle's `m_pReferences`), the AI-7
  sub-task creators (task pool header and byte map). With them `CTaskComplexEnterCar::Constructor` (200 cases),
  the door constructors, `CTaskComplexDie::CreateFirstSubTask`, `CTaskComplexInWater::CreateFirstSubTask` and
  `CCollisionData::Copy` (33 of 33 calls) pass.

Result over the 737 hooks: **370 promoted** (232 L2, 136 split, 2 L1), 64 with an L2/split difference, 303
open: 178 never in a completed run (the 47 L3-only rows, and lists whose L2 run crashed at boot and were not
re-run in time), 44 + 22 without calls on their recording (NOCALLS / split NOCASES), 22 split DIVERGED/COUNT
(nested under another listed hook: one nesting level per run still to do), 27 with undeclared writes left
(caller-stack or heap), 10 with too few cases. Evidence per hook: `docs/verify/verdicts-samp.tsv`.

Promotion check: `tools/e2e/verify-all.sh i6-promote 4,5` on the promoted tree (all 370 in the default set,
DLL sha256 `89d1606395544405…`): **IDENTICAL on all 13 recordings** (save-load: save files identical except
stack residue, ledger SAVE-1). `cargo test -p sa-hook -p sa-core -p sa-ai --lib` 58 / 72 / 13 passed; generators
`--check` clean; host sa-oracle rebuilt after the promotions; queue regenerated: 1,607 unverified hooks in 248
batches.

### Differences found (not promoted; no port bug confirmed)

Most are L2 artefacts of functions that cannot run twice on one state (classified from the reports):

- allocators whose pool/heap state is not declared (second run gets the next slot): `CFileLoader::LoadObjectInstance`,
  `CPointRoute::operator new`, `new` 0x552C80, `CPickups::GenerateNewOne`, `CBike::Constructor`, pool New 0x43F7D0
  (passes with its extras but only 67 of 435 calls sampled);
- callees with OS/DirectSound effects or `rand`: `CAEAudioHardware::PlaySound` (0xB5F898 written by the first run
  only), `CAEStaticChannel::Stop`, `CAESmoothFadeThread::RequestFade`, `CAEPedAudioEntity::AddAudioEvent`,
  `FxEmitter_c::CreateParticle`, `FxSystem_c::Update`, `CBirds::CreateNumberOfBirds`;
- caches holding stack addresses (gpColCache 0x9655C8 / 0x96594C): `CCollision::CameraConeCastVsWorldCollision`,
  `BuildCacheOfCameraCollision`, `ProcessLineOfSight` (1 of 200), `ProcessLineTriangle`, `ProcessVerticalLine`,
  `CWorld::ProcessLineOfSight(Sector)`, `ProcessVerticalLine(Sector)`: need split with SPLIT_STACKFILL;
- `this` not declared (the first run's state change is seen by the second): `CTaskSimpleDie::ProcessPed`
  (orig starts the death anim, Rust sees it started), `CTaskSimpleIKChain::ProcessPed`.

Worth a writer's look (could be real): `CShopping::LoadStats` 0x49B6A0 (split FAIL, 1 case: the Rust run makes a
callee call the original does not, `callees: orig [9] rust [8, 9]`); `FUN_00606890` 0x606890 (walk-into-object
scanner, 5 of 200: the original writes 0xB7CD78 and CReferences where the Rust run writes nothing; stack-residue
hazard, needs split); `CObject::Render` 0x59F180 (2 of 200, window writes at 0x8E2448 differ).

### Harness notes

- Spool quota (1 GiB) was reached early in several lists by unrelated hooks (`CGameLogic::SkipCanBeActivated`,
  `CPed::RemoveGogglesModel`); comparisons before the quota stay valid and the in-process verdicts continue.
- L2 lists that mix in an allocator perturb the replay (`(b)` diverges at frame 202..1348); the per-hook
  comparisons stay valid (each case compares both implementations on the same state) but later hooks get fewer
  calls. Keep allocators / creators in their own split lists.
- `results.tsv` keys hooks by name: the several `(missed by ghidra: data-code-immediate)` hooks share one row's
  numbers there; take them from the host report (by address).

## Promoted addresses (VERIFIER-11)

For `catalog/state.csv` (`progress.py set ADDR L2|L1 EVIDENCE`; evidence per hook in
`docs/verify/verdicts-samp.tsv`). `unverified` removed from the attributes (all hand-written files; no
generated AI hook passed, so `tools/gen/ai_patterns_unverified.txt` is unchanged). Columns: address, level,
batch, run kind (l2 / split / l1). 370 addresses.

```
0x4059B0 L2 mvpl-stream-1 l2
0x406360 L2 std5-stream-1 l2
0x406450 L2 std5-stream-1 l2
0x406CE0 L2 std4-audio-3 l2
0x406D70 L2 mvpa-math-1 l2
0x406DA0 L2 mvpa-math-1 l2
0x40A560 L2 mvpk-stream-1 l2
0x40EEC0 L2 boot-coll-1 l2
0x40EF50 L2 mvpa-math-1 l2
0x40F030 L2 mvpa-math-1 l2
0x40F120 L2 mvpf-coll-1 l2
0x40F700 L2 mvpa-math-1 l2
0x40FE30 L2 mvpa-math-1 l2
0x40FE60 L2 mvpa-math-1 l2
0x40FE90 L2 mvpa-math-1 l2
0x40FEC0 L2 mvpa-math-1 l2
0x411030 L2 mvpl-stream-1 l2
0x411590 L2 mvpa-math-1 l2
0x4119D0 L2 mvpa-math-1 l2
0x411A00 L2 mvpa-math-1 l2
0x415320 L2 mvpe-coll-1 l2
0x417730 L2 mvpc-coll-1 l2
0x419F00 L2 mvpf-coll-1 l2
0x41A820 L2 mvpe-coll-1 l2
0x41AB20 L2 mvpe-coll-1 l2
0x41BCF0 L2 mvpl-world-1 l2
0x420800 L2 mvpa-math-1 l2
0x43E3F0 L2 std5-ctor-1 l2
0x43F720 L2 std4-load-1 l2
0x43F880 L2 mvpc-world-2 split
0x440D10 L2 mvpd-world-3 l2
0x4415C0 L2 mvpd-world-2 l2
0x441E00 L2 std5-ctor-1 l2
0x441E10 L2 mvpi-world-1 l2
0x442480 L2 mvpd-world-2 l2
0x442AD0 L2 mvpd-world-4 l2
0x4471E0 L2 std5-world-1 l2
0x447790 L2 mvpd-world-1 l2
0x44A4D0 L2 mvpd-world-2 l2
0x4541E0 L2 std5-ctor-1 l2
0x455000 L2 mvpd-world-5 l2
0x457410 L2 mvpd-world-5 l2
0x45C210 L2 mvpd-world-1 l2
0x45EBB0 L2 mvpb-world-1 l2
0x45EC20 L2 mvpb-world-1 l2
0x464BB0 L2 mvpj-script-1 l2
0x464C20 L2 mvph-script-1 l2
0x4650F0 L2 std4-data-1 l2
0x465AA0 L2 mvpi-script-1 split
0x46A7C0 L2 mvpf-script-1 l2
0x46A840 L2 mvpf-script-1 l2
0x46A8C0 L2 mvph-script-1 l2
0x46A930 L2 mvpj-script-1 l2
0x46A9C0 L2 mvph-script-1 l2
0x46ABC0 L2 mvph-script-1 split
0x46AE30 L2 std5-ctor-1 l2
0x46B3D0 L2 mvpi-script-2 l2
0x46FF20 L2 mvpi-script-1 l2
0x470000 L2 boot-veh-1 l2
0x470390 L2 mvpj-script-1 l2
0x470750 L2 mvpj-script-1 l2
0x470840 L2 mvpl-script-1 split
0x470960 L2 mvpi-script-2 l2
0x470980 L2 mvpj-script-1 l2
0x474730 L2 mvpi-script-2 l2
0x481200 L2 mvpj-script-1 l2
0x486780 L2 mvpf-script-1 l2
0x4867C0 L2 mvpf-script-1 l2
0x486D80 L2 mvpk-script-2 l2
0x4939F0 L2 mvpk-script-2 l2
0x49C490 L2 mvpd-world-3 l2
0x49C8F0 L2 std5-pool-1 l2
0x49E650 L2 mvpd-render-3 l2
0x49E660 L2 mvpc-fx-1 split
0x4A11E0 L2 mvpc-fx-1 l2
0x4A21D0 L2 mvpc-fx-2 l2
0x4A54E0 L2 mvpa-fx-1 l2
0x4A5D40 L2 mvpa-fx-1 l2
0x4A8470 L2 mvpa-fx-1 l2
0x4A8DD0 L2 std5-ctor-1 l2
0x4A8DF0 L2 std4-core-1 l2
0x4A9140 L2 mvpd-render-3 l2
0x4A92A0 L2 mvpd-render-3 l2
0x4A94E0 L2 std5-ctor-1 l2
0x4A9500 L2 mvpa-fx-2 l2
0x4A9810 L2 mvpa-render-2 split
0x4A9F20 L2 std5-fx-1 l2
0x4A9F40 L2 std5-fx-1 l2
0x4AA030 L2 mvpa-render-3 l2
0x4AA750 L2 mvpc-fx-1 l2
0x4AAC90 L2 mvpd-render-3 l2
0x4AB240 L2 mvpc-fx-1 split
0x4AB260 L2 mvpc-fx-1 split
0x4AC0D0 L2 mvpc-render-1 split
0x4C4A60 L2 std5-ctor-1 l2
0x4C5540 L2 std5-ctor-1 l2
0x4C55A0 L2 std5-ctor-1 l2
0x4C55E0 L2 std5-ctor-1 l2
0x4C5640 L2 std5-ctor-1 l2
0x4C5690 L2 std5-ctor-1 l2
0x4C56F0 L2 std5-ctor-1 l2
0x4C5760 L2 std5-ctor-1 l2
0x4C5CB0 L2 mvpc-ai-1 split
0x4C5D10 L2 mvpc-ai-1 split
0x4C5D70 L2 mvpc-ai-1 split
0x4C5DD0 L2 mvpc-ai-1 split
0x4C5E30 L2 mvpc-ai-1 split
0x4C5E90 L2 mvpc-ai-1 split
0x4C5EF0 L2 mvpc-ai-1 split
0x4C5F50 L2 mvpc-ai-1 split
0x4C67D0 L2 mvpc-ai-1 split
0x4C75E0 L2 mvpc-ai-1 split
0x4CBB00 L2 std4-d3d9-2 split
0x4CC970 L2 std4-d3d9-2 split
0x4CCE60 L2 std4-d3d9-2 split
0x4D7950 L2 std4-audio-1 l2
0x4D79A0 L2 std4-audio-1 l2
0x4D7A50 L2 std4-audio-1 l2
0x4D7AA0 L2 std4-audio-1 l2
0x4D7B50 L2 std4-audio-1 l2
0x4D7BD0 L2 std4-audio-1 l2
0x4D7C60 L2 std4-audio-1 l2
0x4D7D00 L2 std4-audio-1 l2
0x4D8DA0 L2 std4-audio-3 l2
0x4DC1C0 L2 mvpl-audio-2 split
0x4DC230 L2 mvpl-audio-2 split
0x4DC250 L2 mvpl-audio-2 split
0x4DC490 L2 mvpl-audio-2 split
0x4DD4A0 L2 std4-audio-3 split
0x4DFE30 L2 std4-audio-3 split
0x4E0D20 L2 mvpl-audio-2 split
0x4EEC80 L2 std4-audio-1 l2
0x4F0000 L2 std4-audio-3 split
0x4F0B10 L2 std4-audio-1 split
0x4F0BD0 L2 std4-audio-1 l2
0x4F0C40 L2 std4-audio-1 split
0x4F0F40 L2 std4-audio-1 l2
0x4F0F70 L2 std4-audio-1 l2
0x4F0FA0 L2 std4-audio-1 l2
0x4F1040 L2 std4-audio-1 l2
0x4F10D0 L2 std4-audio-1 split
0x4F1230 L2 std4-audio-1 l2
0x4F1530 L2 std4-audio-1 l2
0x4F1580 L2 std4-audio-1 l2
0x4F1750 L2 std4-audio-1 l2
0x4F1800 L2 std4-audio-1 split
0x4F1880 L2 std4-audio-1 l2
0x4F18A0 L2 std4-audio-1 l2
0x4F19E0 L2 std4-audio-1 l2
0x4F1A60 L2 std4-audio-1 l2
0x4F1C70 L2 std4-audio-1 split
0x4F1D40 L2 std4-audio-1 l2
0x4F1DE0 L2 std4-audio-1 split
0x4F1E20 L2 std4-audio-1 split
0x4F1F30 L2 std4-audio-1 l2
0x4F2060 L2 std4-audio-1 l2
0x4F21C0 L2 std4-audio-1 l2
0x4F22F0 L2 std4-audio-1 split
0x4F23D0 L2 std4-audio-1 split
0x4F2550 L2 std4-audio-1 split
0x4F2830 L2 mvpl-audio-2 split
0x5023F0 L2 mvpl-audio-2 split
0x502470 L2 mvpl-audio-2 split
0x5024D0 L2 mvpl-audio-2 split
0x502560 L2 mvpl-audio-2 l2
0x502580 L2 mvpl-audio-2 split
0x5025B0 L2 mvpl-audio-2 split
0x5025D0 L2 mvpl-audio-2 split
0x5025F0 L2 mvpl-audio-2 split
0x502610 L2 mvpl-audio-2 l2
0x502640 L2 mvpl-audio-2 l2
0x502670 L2 mvpl-audio-2 split
0x502700 L2 mvpl-audio-2 split
0x52CDE0 L2 mvpa-render-1 l2
0x52CEE0 L2 mvpa-render-1 l2
0x52CF10 L2 mvpb-render-1 l2
0x533610 L2 mvpa-math-1 split
0x533BF0 L2 mvpc-world-2 split
0x534AE0 L2 mvpd-world-1 split
0x535E90 L2 mvpb-world-1 split
0x541D80 L2 std5-ctor-1 l2
0x545980 L2 mvpf-phys-1 l2
0x54EEA0 L2 mvpk-math-1 l2
0x54EF40 L2 mvpa-math-1 l2
0x54F0C0 L2 mvpa-math-1 l2
0x54F440 L2 mvpa-math-1 split
0x5523D0 L2 std5-pool-1 split
0x552400 L2 mvpa-math-1 split
0x552470 L2 mvpa-math-1 split
0x552830 L2 std4-load-1 l2
0x556000 L2 mvpb-render-1 l2
0x559760 L2 mvpb-world-5 l2
0x55C0C0 L2 mvpa-world-1 split
0x565CB0 L2 mvpb-world-2 split
0x566140 L2 mvpc-world-1 l2
0x569E20 L2 mvpc-world-1 l2
0x56A0D0 L2 mvpd-world-3 split
0x56A490 L2 mvpc-world-1 split
0x591260 L2 std5-ctor-1 l2
0x59AC90 L2 mvpa-math-1 l2
0x59ACD0 L2 mvpa-math-1 l2
0x59C600 L2 mvpd-math-1 l2
0x59C790 L2 mvpa-math-1 l2
0x59C810 L2 mvpa-math-1 l2
0x59CEC0 L2 std4-load-1 split
0x59F0C0 L2 mvpb-coll-1 l2
0x5A3140 L2 mvpc-render-1 split
0x5A3E20 L2 std5-ctor-1 l2
0x5A7950 L2 mvpi-ped-1 l2
0x5BA8C0 L2 mvpe-tempcol-1 split
0x5BB890 L2 std4-data-1 l2
0x5BD830 L2 std4-data-1 l2
0x5BE670 L2 mvpf-weapon-1 split
0x5BF400 L2 std4-data-1 l2
0x5BF9D0 L2 std4-data-1 split
0x5C0280 L2 mvpc-world-2 split
0x5C05F0 L2 mvpc-fx-1 split
0x5C0B70 L2 mvpc-fx-1 split
0x5C16F0 L2 mvpc-fx-1 split
0x5C18F0 L2 mvpc-fx-1 split
0x5C1B10 L2 mvpc-fx-1 split
0x5C1D30 L2 mvpc-fx-1 split
0x5C1F50 L2 mvpc-fx-1 split
0x5C2420 L2 mvpc-fx-1 split
0x5D5BC0 L2 std5-rw-1 split
0x5D6750 L2 std4-rxpipe-3 split
0x5D7AA0 L2 std4-rxpipe-3 split
0x5D9F80 L2 std4-rxpipe-3 split
0x5DA020 L2 std4-rxpipe-3 split
0x5DA790 L2 boot-render-2 l2
0x5DABE0 L2 std4-load-1 l2
0x5DACC0 L2 std4-load-1 l2
0x5DB0C0 L2 mvpd-render-3 split
0x5DD0B0 L2 std4-load-1 l2
0x5DD100 L2 std4-load-1 split
0x5DD1E0 L2 regconv-render-1 split
0x5DD3B0 L2 std4-load-1 split
0x5DD780 L2 mvpc-render-1 split
0x5DD910 L2 std4-load-1 split
0x5DE920 L2 mvpl-audio-1 split
0x5DF170 L2 mvpi-ped-1 split
0x5F1600 L2 mvpa-ai-1 l2
0x5F1670 L2 mvpa-ai-1 l2
0x5FFF90 L2 mvpd-ai-1 split
0x608B30 L2 mvpf-ped-2 split
0x608E40 L2 mvpf-ped-2 split
0x60D5B0 L2 mvpi-ped-1 split
0x614720 L2 popw4-frame-1 split
0x6302D0 L2 ai7-12 l2
0x635200 L2 ai7-12 l2
0x63A220 L2 ai7-11 l2
0x63A690 L2 ai7-12 l2
0x63C460 L2 mvpa-ai-2 split
0x645E50 L2 ai7-11 l2
0x6464F0 L2 ai7-11 l2
0x64E4B0 L2 ai7-11 l2
0x6883D0 L2 mvpj-ai-3 l2
0x688810 L2 mvpj-ai-3 l2
0x68A9F0 L2 ai7-12 l2
0x694850 L2 std5-ctor-1 l2
0x698710 L2 mvpa-ai-1 l2
0x6A8C00 L2 mvpd-veh-5 l2
0x6AA290 L2 mvpd-veh-4 l2
0x6B5FB0 L2 mvpd-veh-3 l2
0x6B8EC0 L2 mvpb-veh-4 l2
0x6BC930 L2 mvpd-veh-6 l2
0x6D6C00 L2 mvpd-veh-1 l2
0x6D73B0 L2 mvpd-veh-1 l2
0x6DE240 L2 mvpd-veh-2 l2
0x6DF4A0 L2 mvpf-veh-1 l2
0x6E1780 L2 mvpb-veh-2 l2
0x6EA9F0 L2 mvpd-render-3 l2
0x6F55D0 L2 mvpc-veh-1 split
0x6F9DE0 L2 mvpa-render-1 l2
0x6FA6D0 L2 mvpc-render-1 split
0x6FA6F0 L2 mvpj-script-1 l2
0x6FA790 L2 mvpc-render-1 split
0x6FC4D0 L2 mvpb-render-1 l2
0x6FD7C0 L1 harness5-1 l1
0x6FF350 L2 mvpd-render-3 l2
0x6FF410 L2 mvpc-render-1 l2
0x6FF420 L2 mvpb-render-1 l2
0x700CC0 L2 mvpd-render-3 l2
0x703650 L2 mvpd-render-3 l2
0x706CD0 L2 mvpc-render-1 split
0x707FA0 L2 mvpd-render-1 l2
0x708300 L2 mvpd-render-1 l2
0x70C950 L2 mvpd-render-1 l2
0x70D490 L2 mvpd-render-5 l2
0x70E4A0 L2 mvpd-render-3 l2
0x70EAB0 L2 mvpd-render-5 l2
0x70FA70 L1 harness5-1 l1
0x710BA0 L2 mvpd-render-4 l2
0x711760 L2 regconv-render-1 split
0x712330 L2 mvpd-render-2 l2
0x717180 L2 mvpd-render-3 l2
0x7172D0 L2 mvpb-render-1 l2
0x7173A0 L2 mvpd-render-3 l2
0x7174F0 L2 mvpb-render-1 l2
0x717D00 L2 mvpa-render-1 l2
0x7205C0 L2 mvpb-render-1 l2
0x720640 L2 mvpd-render-3 l2
0x7240E0 L2 mvpd-render-3 l2
0x724770 L2 mvpc-render-1 l2
0x724890 L2 mvpd-render-3 l2
0x726060 L2 mvpd-render-3 l2
0x7268F0 L2 mvpc-render-1 split
0x726DF0 L2 mvpd-render-2 split
0x728520 L2 mvpb-render-1 l2
0x728640 L2 std4-render-1 l2
0x72B630 L2 mvpb-render-1 l2
0x72B850 L2 mvpd-render-4 l2
0x72C950 L2 mvpc-render-1 l2
0x72FFF0 L2 std4-d3d9-3 l2
0x7305E0 L2 std4-core-1 split
0x7306A0 L2 mvpd-d3d-1 split
0x730740 L2 std4-core-1 split
0x730B70 L2 mvpd-d3d-1 split
0x730D30 L2 mvpd-d3d-1 split
0x732820 L2 boot-render-1 split
0x7328C0 L2 std4-render-1 l2
0x733330 L2 boot-render-1 split
0x734240 L2 boot-render-1 split
0x734820 L2 std4-load-1 l2
0x734D80 L2 std4-load-1 l2
0x734FC0 L2 mvpc-render-1 l2
0x747200 L2 mvpb-render-1 split
0x748F20 L2 std4-render-1 l2
0x748F30 L2 std4-render-1 l2
0x748F50 L2 std4-render-1 l2
0x74B420 L2 std4-load-2 split
0x7507E0 L2 std5-rwworld-1 split
0x7591B0 L2 std5-tristrip-1 split
0x7591D0 L2 std5-tristrip-1 split
0x759640 L2 std5-tristrip-1 l2
0x75E9F0 L2 std4-rxpipe-1 l2
0x760CF0 L2 std4-d3d9-2 l2
0x7C68E0 L2 std4-rxpipe-2 split
0x7C6AE0 L2 std4-load-2 l2
0x7F6B00 L2 std4-d3d9-1 l2
0x7F6B40 L2 std4-d3d9-1 split
0x7F6BF0 L2 std4-d3d9-1 l2
0x7F6CB0 L2 std4-d3d9-1 l2
0x7F72E0 L2 std4-d3d9-1 l2
0x7F7540 L2 std4-d3d9-1 split
0x7F7730 L2 std4-d3d9-2 split
0x7F7F70 L2 std4-d3d9-1 split
0x804130 L2 std4-audio-3 l2
0x805950 L2 std4-rxpipe-1 l2
0x805F40 L2 std4-rxpipe-1 split
0x8064D0 L2 std4-rxpipe-1 split
0x806990 L2 std4-rxpipe-1 split
0x806B30 L2 std4-rxpipe-1 l2
0x806BE0 L2 std4-rxpipe-1 split
0x806DE0 L2 std4-rxpipe-1 split
0x80DBE0 L2 std4-d3d9-1 l2
0x80DC40 L2 std4-d3d9-1 l2
0x80DCA0 L2 std4-d3d9-1 l2
0x80DD00 L2 std4-d3d9-1 l2
0x80F0D0 L2 std4-rxpipe-1 split
0x80F460 L2 std4-rxpipe-1 split
0x80F690 L2 std4-rxpipe-1 split
0x80FC30 L2 std4-rxpipe-1 split
0x8103D0 L2 std4-rxpipe-1 split
0x810BF0 L2 std4-rxpipe-2 split
0x84A720 L2 boot-script-1 split
0x84E2E0 L2 boot-core-1 split
0x8508C0 L2 boot-render-1 split
0x850910 L2 boot-render-1 split
0x855FD0 L2 boot-input-1 split
```
