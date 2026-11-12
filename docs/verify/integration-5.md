# INTEGRATE-5: VERIFIER-8, WRITER-P and PORT-SAVE

Integration branch `worktree-agent-aea710a33e9789b7c`, from master `7f75957`.

## What came from where

| commit | source | content |
|---|---|---|
| `2e4eaa3` | VERIFIER-8 `a3eb233` (pick) | `FUN_006070F0` 0x6070F0: three stack arguments; `CDecisionMakerTypes::AddDecisionMaker` 0x607050 declared with three (`ret 0xC`) |
| `fb36314` | VERIFIER-8 `a3bbfe4` (pick) | `CWorld::ProcessVerticalLineSectorList` port: see-through flag and poly in the original argument order |
| `7f55141` | VERIFIER-8 `ae2e536` (pick) | `CEntity::IsEntityOccluded` port: the four corners the original tests (max corner) |
| `e16a7a6` | VERIFIER-8 `15bb4d6` (status rows only) | `catalog/verify-status.csv`: L3-batch rows of the 116 mvpc..mvpm / popw4 / harness5-1 / regconv batches, union with master's rows by address |
| `a666fdf` | VERIFIER-8 `d015b4f` (without queue files) | 54 promotions (`unverified` removed), `docs/verify/verdicts-writers.tsv`, one `tools/e2e/l2-extras.txt` line (`CCollision::IsStoredPolyStillValidVerticalLine@arg2:0x2C:RW`) |
| `9cc73f6` | VERIFIER-8 `e75d4c3` (without queue files) | 13 more promotions, verdicts updated |
| `399478d` | WRITER-P `1b4c361` (without queue and MVP-gap files) | hooks 0x84A720, 0x84E2E0, 0x8508C0, 0x850910, 0x855FD0 (static initialisers), 0x40EEC0 (CColBox::operator=), 0x470000 (float pow wrapper), 0x732820, 0x733330, 0x734240 (train / big-vehicle visibility callbacks), 0x5D9900, 0x5D84C0, 0x5D8590, 0x5DA790 (car env-map render callback and helpers), 0x4C9990 (`_rxD3D9DefaultDestroyInstanceCallback`), all unverified (boot-* batches); native 0x4C73F0 |
| `c492619` | regeneration | `tools/gen/inflight.py`, `tools/gen/patterns.py` (the unhooked generated leaf 0x4C73F0 gives way to WRITER-P's native) |
| `ec238b9` | PORT-SAVE `21ca980` (pick) | harness pokes in recordings, save-file collection in the e2e scripts (port-l2 check (f)), `footprint_entry_esp`, the save-load program |
| `9b1fb21` | PORT-SAVE `95f3cfb` (without queue files) | SAVE-1 ports (C_PcSave, CGenericGameStorage, the save blocks, the stats strings), `tools/accept/save-diff.py`, `tools/accept/f5-saves.sh`, l2-extras |
| `254e9f8` | PORT-SAVE `eb1178d` (without queue files) | CTheScripts::Load returns the script count, BuildStatLine, split extras, corpus doc, `docs/arch/save-port-verdicts.tsv` |
| `424c7c4` | PORT-SAVE `cc909ec` (without queue files) + main-session decision | 59 promotions; GenericSave 0x5D13E0 and SaveSlot 0x619060 lose `unverified` (ledger SAVE-1, named in their hook notes) |
| `961b28b` | PORT-SAVE `aaa7f6a` (pick) | SAVE-1 docs |
| `13d0e7b` | PORT-SAVE `e1483b8` (pick) | `verify-all.sh` accepts residue-only save differences (ledger SAVE-1) |
| `9b2f957` | regeneration | `inflight.py` after SAVE-1 (87 exclusions); `patterns.py` output unchanged, `--check` clean |
| `c071917` | regeneration | `python3 -I tools/verify/queue.py`: 1,671 unverified hooks in 265 batches |
| `2fb94da` | regeneration | `tools/native/mvp_gaps.py`: `docs/arch/platform-mvp-gaps.csv` |

Not taken: VERIFIER-8 `8663457` (l3 `@FILE` mode lists: master's `2db5f17` is the same fix) and `794cbd9`
(CreateIndexBuffer 0x7306A0 format argument: master already has `(n, _format, out)` from STD-9 / INTEGRATE-3);
every branch's `catalog/verify-queue.csv` / `docs/verify/QUEUE.md` (regenerated); WRITER-P's
`docs/arch/platform-mvp-gaps.csv` (regenerated); PORT-SAVE's `-s ours` merge of PORT-MENU `f61a145`.

List files: `hooks.rs` MODULES / NATIVES and `lib.rs` / `mod.rs` module lists are the union (sa-core, sa-ped,
sa-script, sa-world, sa-render, sa-rwcore, sa-veh, sa-stream, sa-input); `tools/e2e/l2-extras.txt` is the union
with no duplicate entry line. No keycells READERS change. One registration per address: the only repeated
`#[hook(addr` in `crates/` is master's 0x407180 doc example in `sa-macros/src/lib.rs`. The four addresses held
back from the generator for PORT-SAVE (0x5D24B0, 0x5D2560, 0x5D2980, 0x5D47E0) are its hand ports now; the
generator still emits the same 841 hooks.

Ledger: GenericSave / SaveSlot are promoted under divergence-ledger entry **SAVE-1** (the save bytes differ only
in uninitialised-stack residue; the state after the load is identical). The ROADMAP ledger row is left to the
main session.

## Verification

On head `2fb94da`:

- `cargo test -p sa-hook --lib` 58 passed; `-p sa-script --lib` 9 passed; `-p sa-core --lib` 71 passed.
- `cargo test -p sa-oracle --test l1 --release`: 18 passed, 1 ignored (1,221 s; the machine was shared with
  another L1 run and the verify-all lanes).
- Host `sa-oracle` / `satool` rebuilt in this worktree (release). i686-pc-windows-gnu DLL (`sa-launcher`, `sa-hook`):
  OK. i686-unknown-linux-gnu `sa-game` release: OK.
- `tools/e2e/verify-all.sh int5 6,7` (SA_ROOT = `~/sa-engine`, DLL sha256 `52da2500…c3d81f7c`): **IDENTICAL on all 13
  recordings** (inf4 grove walk, air-water, anim-input, car-ai, c-bug, driving, menus-hud, menus-map-controls,
  pause-restart, police-gang, save-load, weapon-combat, world-cheats). save-load: save files identical except
  stack residue (750 differing bytes, all residue, ledger SAVE-1). None skipped by the disk guard; no bisection
  needed; nothing dropped.

## Promoted addresses

For `catalog/state.csv`. All promotions are L2 (split PASS, 0 mismatches, 0 undeclared writes) except the two
SAVE-1 ledger entries.

### VERIFIER-8 (evidence: `docs/verify/verdicts-writers.tsv`): 67 addresses, all L2

```
0x406D10 L2 mvpl-world-1
0x407000 L2 mvpl-world-1
0x40FEF0 L2 mvpd-d3d-1
0x413100 L2 mvpe-coll-1
0x413AC0 L2 mvpd-coll-1
0x414D70 L2 mvpd-coll-1
0x415CF0 L2 mvpf-coll-1
0x4165B0 L2 mvpe-coll-1
0x416BA0 L2 mvpe-coll-1
0x43AA40 L2 mvpc-ai-1
0x43ABA0 L2 mvpm-ai-1
0x43C590 L2 mvpm-ai-1
0x43CEC0 L2 popw4-partial-1
0x4648E0 L2 mvpf-script-1
0x46A810 L2 mvpl-script-1
0x46A890 L2 mvpl-script-1
0x4700E0 L2 mvpj-script-1
0x470480 L2 mvpf-script-1
0x4704B0 L2 mvph-script-1
0x470620 L2 mvpi-script-2
0x470660 L2 mvpf-script-1
0x4706C0 L2 mvpf-script-1
0x470810 L2 mvph-script-1
0x470910 L2 mvpi-script-2
0x4810C0 L2 mvpi-script-2
0x4810E0 L2 mvpi-script-2
0x4866C0 L2 mvpi-script-1
0x49AA00 L2 mvpd-world-2
0x49EA70 L2 mvpm-render-1
0x4CAD90 L2 regconv-rw-1
0x4CAE60 L2 regconv-rw-1
0x4D9E50 L2 harness5-1
0x4DC2B0 L2 mvpl-audio-2
0x559AF0 L2 mvpd-world-2
0x55B980 L2 mvpd-world-2
0x55C470 L2 mvpi-world-1
0x561FE0 L2 mvpl-ped-1
0x565000 L2 mvpc-world-1
0x56EC80 L2 mvpm-ped-1
0x598A60 L2 mvpd-world-3
0x59C300 L2 mvpd-math-1
0x59C3E0 L2 mvpk-math-1
0x59FEE0 L2 mvpk-world-2
0x5A02E0 L2 mvpk-world-3
0x5A81B0 L2 mvpi-ped-1
0x5DBE40 L2 harness5-1
0x600180 L2 mvpc-ai-2
0x603720 L2 mvpk-ai-4
0x603A30 L2 mvpc-ai-1
0x603E70 L2 mvpk-ai-4
0x6042B0 L2 mvpc-ai-1
0x6060A0 L2 mvpl-ai-1
0x606580 L2 mvpk-ai-4
0x61A990 L2 mvpi-ai-1
0x654A00 L2 mvpj-ai-6
0x66D440 L2 mvpc-ai-1
0x66FDA0 L2 mvpl-ai-2
0x6857E0 L2 mvpj-ai-3
0x6A7090 L2 mvpd-veh-2
0x6AFB10 L2 mvpd-veh-1
0x6D6640 L2 mvpk-veh-1
0x70C750 L2 mvpk-render-1
0x711160 L2 harness5-1
0x7111F0 L2 harness5-1
0x71FAE0 L2 mvpd-world-3
0x7229C0 L2 mvpd-render-4
0x734940 L2 mvpd-unk-1
```

### PORT-SAVE (evidence: `docs/arch/save-port-verdicts.tsv`): 59 L2 + 2 L2 under ledger SAVE-1

```
0x44A170 L2 save-world-1
0x559230 L2 save-stats-1
0x55A210 L2 save-stats-1
0x55A780 L2 save-stats-1
0x5D0D20 L2 save-core-1
0x5D0DD0 L2 save-core-1
0x5D0E90 L2 save-core-1
0x5D0F40 L2 save-core-1
0x5D0F50 L2 save-core-1
0x5D0F80 L2 save-core-1
0x5D10B0 L2 save-core-1
0x5D1170 L2 save-core-1
0x5D1270 L2 save-core-1
0x5D1300 L2 save-core-1
0x5D1380 L2 save-core-1
0x5D17B0 L2 save-core-1
0x5D1B80 L2 save-core-1
0x5D1EA0 L2 save-core-1
0x5D24B0 L2 save-ped-1
0x5D2560 L2 save-ped-1
0x5D2620 L2 save-pop-1
0x5D2740 L2 save-pop-1
0x5D3160 L2 save-world-1
0x5D3270 L2 save-world-1
0x5D33C0 L2 save-world-1
0x5D3440 L2 save-world-1
0x5D3540 L2 save-world-1
0x5D35A0 L2 save-world-1
0x5D3620 L2 save-world-1
0x5D3770 L2 save-world-1
0x5D3AC0 L2 save-ped-1
0x5D3B00 L2 save-ped-1
0x5D3B40 L2 save-world-1
0x5D3BF0 L2 save-world-1
0x5D3C70 L2 save-world-1
0x5D3CA0 L2 save-world-1
0x5D3CD0 L2 save-ped-1
0x5D3D10 L2 save-ped-1
0x5D3DE0 L2 save-world-1
0x5D3E40 L2 save-world-1
0x5D43D0 L2 save-ped-1
0x5D44B0 L2 save-ped-1
0x5D4640 L2 save-ped-1
0x5D46E0 L2 save-ped-1
0x5D4940 L2 save-core-2
0x5D4C40 L2 save-script-1
0x5D4FD0 L2 save-script-1
0x5D5530 L2 save-pop-1
0x5D5570 L2 save-world-1
0x5D55C0 L2 save-world-1
0x5D5730 L2 save-ped-1
0x5D57E0 L2 save-ped-1
0x5D5920 L2 save-world-1
0x5D5970 L2 save-world-1
0x618D00 L2 save-core-1
0x618E90 L2 save-core-1
0x618F50 L2 save-core-1
0x6190A0 L2 save-core-1
0x69F910 L2 save-core-1
0x5D13E0 L2 save-core-1 (ledger SAVE-1)
0x619060 L2 save-core-1 (ledger SAVE-1)
```
