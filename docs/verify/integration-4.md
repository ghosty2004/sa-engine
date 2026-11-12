# INTEGRATE-4: VERIFIER-7 (gen2 promotions) and WRITER-O

Integration branch `worktree-agent-a89ee9d4903f5a60d`, from master `ca917d6`.

## What came from where

| commit | source | content |
|---|---|---|
| `2db5f17` | VERIFIER-7 `963b06a` (pick) | l3 probe mode lists passed to the runner as `@FILE` |
| `53fa7a9` | VERIFIER-7 `6a855df` + `f79eec5`, merged by hand with INTEGRATE-3 `519494c` | `tools/gen/patterns.py`, `docs/arch/gen-all.md` (below) |
| `832a60c` | VERIFIER-7 `39d0278` + `30f58cc` (data only) | `tools/gen/patterns_verified.txt`, `docs/verify/verdicts-gen2.tsv`, `catalog/verify-status.csv`, the gen2 block of `tools/e2e/l2-extras.txt` |
| `0ba8327` | WRITER-O `19c4c83` (pick) | `CAEVehicleAudioEntity::AddAudioEvent` 0x4F6420 + keycells `Reader` at 0x401931 (READERS 32 -> 33) |
| `c97b6d1` | WRITER-O `2f6593e` (pick) | `FxEmitterBP_c::Render` 0x4A2C40, `FxEmitterBP_c::RenderHeatHaze` 0x4A1940 |
| `abe5e42` | WRITER-O `86975df` (pick, without its queue files) | RtQuat 0x7EB5C0, 0x7EB6A0, 0x7EB700, 0x7EB760, 0x7EB7C0, 0x7EBBB0, 0x7EC220; 0x7F1930 (RwMatrixOrthoNormalize body) |
| `e771134` | regeneration | `tools/gen/inflight.py` (123 exclusions) and `tools/gen/patterns.py` on the integrated tree |
| `160b066` | regeneration | `python3 -I tools/verify/queue.py`: `catalog/verify-queue.csv`, `docs/verify/QUEUE.md`, `catalog/verify-status.csv` |

Not taken: VERIFIER-7's generated `.rs` files and queue files (regenerated instead), WRITER-O's MVP-N cherry-picks
(already on master).

List files: `sa-audio`, `sa-render`, `sa-rwcore` `hooks.rs` MODULES and `lib.rs` mods are the union (one new module
each); keycells READERS: one new reader (`at` 0x401931), count 33, no duplicate `at`. `tools/e2e/l2-extras.txt`:
VERIFIER-7's 61 lines added; 121 exact duplicate lines that master already carried (earlier textual merges) removed.
No address is registered twice by this integration; the only repeats in `crates/` are master's known ones
(0x6F9DE0 / 0x407F20 hook plus native, STD-10; the `regconv_tests.rs` test hooks).

## Generator merge

Both VERIFIER-7 (`own_hooks`) and INTEGRATE-3 (`generated_addrs`) fixed the bug "catalogue rows hide the generator's
own hooks". Kept one mechanism: INTEGRATE-3's `generated_addrs` (it also counts path-qualified `#[native]`), widened
to VERIFIER-7's scope: a row at `implemented`, `L1`, `L2` or `L3` for an address the generator's own files register
does not hide it (needed once the main session records the promotions in `catalog/state.csv`). An `excluded` row
still hides it. Kept from VERIFIER-7: `void_ports` (forwarders into void ports generated void; 16 forwarders, now on
master's output too) and `PROMOTED` (own hooks at L1/L2/L3 plus `tools/gen/patterns_verified.txt`, generated
without `unverified`).

Regeneration: 2,395 candidates, 952 generated, **841 hooked** (22 batches, 25 L1-pure), the same 841 addresses as
master (no hook added or dropped; WRITER-O's ports replace no generated hook). `--check` clean. 532 hooks lose
`unverified`; 309 stay unverified (VERIFIER-7's 313 minus the 4 INTEGRATE-3 dropped: 0x5D24B0, 0x5D2560, 0x5D2980,
0x5D47E0; `queue.py` dropped their status rows). Verify queue: 1,715 unverified hooks in 265 batches.

### Promotions that vanished

Seven of the 539 addresses in `patterns_verified.txt` are no longer generated: master hand-ports them (their
catalogue rows are `implemented` from master's registrations). They are not promoted by this integration and stay
in `patterns_verified.txt` (harmless: the generator only applies the list to hooks it emits).

| address | name | batch | now registered in |
|---|---|---|---|
| 0x40FC80 | thunk_FUN_015637e0 | gen2-coll-1 | `sa-world/src/collision/col_point_assign.rs` |
| 0x46B260 | CScriptsForBrains | gen2-script-1 | `sa-core/src/trivial_ctors.rs` |
| 0x4A8DB0 | ListItem_c::ListItem_c | gen2-core-1 | `sa-core/src/object_ctors.rs` |
| 0x4A8E30 | FUN_004a8e30 | gen2-core-1 | `sa-render/src/list_remove_item.rs` |
| 0x722220 | MarkerAtomicCB | gen2-render-4 | `sa-render/src/rw_search_callbacks.rs` |
| 0x7316D0 | FUN_007316d0 | gen2-stream-1 | `sa-ped/src/default_txd_slot.rs` |
| 0x734930 | GetFirstTextureCallback | gen2-rw-3 | `sa-render/src/rw_search_callbacks.rs` |

## Verification

On head `160b066`:

- `cargo test -p sa-hook --lib` 58 passed; `-p sa-script --lib` 8 passed; `-p sa-core --lib` 70 passed.
- `cargo test -p sa-oracle --test l1 --release`: 18 passed, 1 ignored (193 s).
- Host `sa-oracle` / `satool` rebuilt in this worktree (release). i686-pc-windows-gnu DLL (`sa-launcher`, `sa-hook`):
  OK. i686-unknown-linux-gnu `sa-game` release: OK.
- `tools/e2e/verify-all.sh int4 4,5` (SA_ROOT = `~/sa-engine`, DLL sha256 `ed67c4da…bea4f6b`): **IDENTICAL on all 13
  recordings** (inf4 grove walk, air-water, anim-input, car-ai, c-bug, driving, menus-hud, menus-map-controls,
  pause-restart, police-gang, save-load, weapon-combat, world-cheats). None skipped by the disk guard; no bisection
  needed; nothing dropped.

## Promoted generated addresses

For `catalog/state.csv` (evidence: `docs/verify/verdicts-gen2.tsv`, VERIFIER-7; L1 = native backend, 1,002,000 cases,
0 mismatches; L2 = split PASS, 0 mismatches, 0 undeclared writes; all L3 MATCH on 13 recordings). 532 addresses:
25 L1, 507 L2.

```
0x41BD50 L2
0x424160 L2
0x45B150 L2
0x45C850 L1
0x464D20 L1
0x470900 L1
0x4A8E70 L2
0x4A8EB0 L2
0x4A9DC0 L2
0x4AB270 L2
0x4AB290 L2
0x4C88F0 L1
0x4CFA90 L2
0x4CFB10 L2
0x571970 L1
0x571980 L1
0x5B1A80 L2
0x5B1DE0 L2
0x5D5BA0 L2
0x5DAAC0 L2
0x5DAC10 L2
0x5DB250 L2
0x5DB590 L2
0x5E46D0 L2
0x6E7680 L2
0x6FA640 L1
0x6FA670 L1
0x6FA6A0 L1
0x705620 L2
0x706440 L2
0x722150 L2
0x7221C0 L2
0x72F500 L1
0x7323C0 L2
0x732B30 L2
0x734530 L2
0x734540 L2
0x746070 L2
0x748F70 L2
0x749000 L2
0x74BF00 L1
0x74BF10 L1
0x74CE10 L1
0x74DCB0 L1
0x74FD90 L1
0x751E00 L1
0x758240 L2
0x761D50 L1
0x7C69F0 L2
0x7C6A70 L1
0x7C6A80 L2
0x7C6AA0 L2
0x7C6AC0 L2
0x7C7500 L1
0x7C8A00 L2
0x7CC010 L1
0x7EE4A0 L1
0x7F12B0 L1
0x7F2BA0 L1
0x7F3C60 L1
0x8052F0 L2
0x810030 L1
0x810C40 L2
0x811A00 L2
0x811A20 L2
0x811A40 L2
0x811B00 L2
0x848F10 L2
0x848F30 L2
0x848F50 L2
0x848F90 L2
0x848FB0 L2
0x848FD0 L2
0x848FF0 L2
0x849010 L2
0x849050 L2
0x849070 L2
0x849090 L2
0x8490B0 L2
0x8490D0 L2
0x849110 L2
0x849130 L2
0x849150 L2
0x849170 L2
0x849190 L2
0x8491D0 L2
0x8491F0 L2
0x849210 L2
0x849230 L2
0x849250 L2
0x849290 L2
0x8492B0 L2
0x8492D0 L2
0x8492F0 L2
0x849310 L2
0x849350 L2
0x849370 L2
0x849450 L2
0x84A580 L2
0x84A5A0 L2
0x84A5C0 L2
0x84A600 L2
0x84A620 L2
0x84A660 L2
0x84A680 L2
0x84A6A0 L2
0x84A6E0 L2
0x84A700 L2
0x84A740 L2
0x84A7E0 L2
0x84A800 L2
0x84A820 L2
0x84A860 L2
0x84A880 L2
0x84A960 L2
0x84A980 L2
0x84A9A0 L2
0x84A9E0 L2
0x84AA00 L2
0x84AC60 L2
0x84AC80 L2
0x84ACA0 L2
0x84ACE0 L2
0x84AD00 L2
0x84B510 L2
0x84B530 L2
0x84B550 L2
0x84B590 L2
0x84B5B0 L2
0x84B630 L2
0x84B730 L2
0x84B750 L2
0x84B770 L2
0x84B7B0 L2
0x84B7D0 L2
0x84BB50 L2
0x84BB70 L2
0x84BB90 L2
0x84BBB0 L2
0x84BBD0 L2
0x84BD10 L2
0x84BD30 L2
0x84BD50 L2
0x84BD70 L2
0x84BD90 L2
0x84BDD0 L2
0x84BDF0 L2
0x84BE10 L2
0x84BE30 L2
0x84BE50 L2
0x84BE90 L2
0x84BEB0 L2
0x84BED0 L2
0x84BEF0 L2
0x84BF10 L2
0x84BF30 L2
0x84BF70 L2
0x84BF90 L2
0x84BFB0 L2
0x84C020 L2
0x84C040 L2
0x84C060 L2
0x84C0A0 L2
0x84C0C0 L2
0x84D7A0 L2
0x84D7C0 L2
0x84D7E0 L2
0x84D820 L2
0x84D840 L2
0x84D860 L2
0x84D880 L2
0x84D8A0 L2
0x84D8E0 L2
0x84D900 L2
0x84D930 L2
0x84D950 L2
0x84D970 L2
0x84D9B0 L2
0x84D9D0 L2
0x84D9F0 L2
0x84DA10 L2
0x84DA30 L2
0x84DA70 L2
0x84DA90 L2
0x84DB70 L2
0x84DB90 L2
0x84DBB0 L2
0x84DBF0 L2
0x84DC10 L2
0x84DC30 L2
0x84DC50 L2
0x84DC70 L2
0x84DCB0 L2
0x84DCD0 L2
0x84DCF0 L2
0x84DD10 L2
0x84DD30 L2
0x84DD70 L2
0x84DD90 L2
0x84DDB0 L2
0x84DDD0 L2
0x84DDF0 L2
0x84DE30 L2
0x84DE50 L2
0x84DF50 L2
0x84DF70 L2
0x84DF90 L2
0x84DFD0 L2
0x84DFF0 L2
0x84E030 L2
0x84E050 L2
0x84E070 L2
0x84E0B0 L2
0x84E0D0 L2
0x84E130 L2
0x84E150 L2
0x84E170 L2
0x84E1B0 L2
0x84E1D0 L2
0x84E220 L2
0x84E240 L2
0x84E260 L2
0x84E2A0 L2
0x84E2C0 L2
0x84E320 L2
0x84E340 L2
0x84E360 L2
0x84E3A0 L2
0x84E3C0 L2
0x84E3E0 L2
0x84E400 L2
0x84E420 L2
0x84E460 L2
0x84E480 L2
0x84E4A0 L2
0x84E4C0 L2
0x84E4E0 L2
0x84E520 L2
0x84E540 L2
0x84E620 L2
0x84E640 L2
0x84E660 L2
0x84E6A0 L2
0x84E6C0 L2
0x84E6F0 L2
0x84E710 L2
0x84E730 L2
0x84E770 L2
0x84E790 L2
0x84EA60 L2
0x84EA80 L2
0x84EAA0 L2
0x84EAE0 L2
0x84EB00 L2
0x84EB20 L2
0x84EB40 L2
0x84EB60 L2
0x84EBA0 L2
0x84EBC0 L2
0x84F9A0 L2
0x84F9C0 L2
0x84F9E0 L2
0x84FA20 L2
0x84FA40 L2
0x84FA60 L2
0x84FAA0 L2
0x84FAC0 L2
0x84FAE0 L2
0x84FB20 L2
0x84FB40 L2
0x84FB60 L2
0x84FB80 L2
0x84FBA0 L2
0x84FBE0 L2
0x84FC00 L2
0x84FCE0 L2
0x84FD00 L2
0x84FD20 L2
0x84FD40 L2
0x84FD80 L2
0x84FDA0 L2
0x84FDE0 L2
0x84FE00 L2
0x84FE70 L2
0x84FE90 L2
0x84FEB0 L2
0x84FEF0 L2
0x84FF10 L2
0x84FF60 L2
0x84FF80 L2
0x84FFA0 L2
0x84FFC0 L2
0x850000 L2
0x850020 L2
0x850120 L2
0x850140 L2
0x850160 L2
0x850180 L2
0x8501C0 L2
0x8501E0 L2
0x8502C0 L2
0x8502E0 L2
0x850300 L2
0x850340 L2
0x850360 L2
0x850380 L2
0x8503A0 L2
0x8503C0 L2
0x850400 L2
0x850420 L2
0x850440 L2
0x850460 L2
0x850480 L2
0x8504A0 L2
0x850500 L2
0x850520 L2
0x850540 L2
0x850560 L2
0x850580 L2
0x8505A0 L2
0x8505E0 L2
0x850600 L2
0x850620 L2
0x850640 L2
0x850660 L2
0x8506A0 L2
0x8506C0 L2
0x8506E0 L2
0x850700 L2
0x850720 L2
0x850760 L2
0x850780 L2
0x8507A0 L2
0x8507C0 L2
0x8507E0 L2
0x850800 L2
0x850820 L2
0x850840 L2
0x850880 L2
0x8508A0 L2
0x8508F0 L2
0x850AC0 L2
0x850D10 L2
0x850D30 L2
0x850D50 L2
0x850D90 L2
0x850DB0 L2
0x8516F0 L2
0x851730 L2
0x851750 L2
0x8519B0 L2
0x8519D0 L2
0x8519F0 L2
0x851A30 L2
0x851A50 L2
0x851C30 L2
0x851C50 L2
0x851C70 L2
0x851CB0 L2
0x851CD0 L2
0x852010 L2
0x852780 L2
0x8527A0 L2
0x8527C0 L2
0x852800 L2
0x852820 L2
0x853700 L2
0x853720 L2
0x853740 L2
0x853780 L2
0x8537A0 L2
0x853E50 L2
0x853E70 L2
0x853E90 L2
0x853ED0 L2
0x853EF0 L2
0x854430 L2
0x854450 L2
0x854490 L2
0x8544B0 L2
0x8544D0 L2
0x8544F0 L2
0x854510 L2
0x854530 L2
0x854570 L2
0x854590 L2
0x854610 L2
0x854630 L2
0x854650 L2
0x854690 L2
0x8546B0 L2
0x8546D0 L2
0x8546F0 L2
0x854710 L2
0x854730 L2
0x854750 L2
0x854770 L2
0x854790 L2
0x8547D0 L2
0x8547F0 L2
0x854810 L2
0x8548A0 L2
0x8548B0 L2
0x8548C0 L2
0x8548E0 L2
0x854900 L2
0x854940 L2
0x854960 L2
0x8549A0 L2
0x8549C0 L2
0x8549E0 L2
0x854A20 L2
0x854A40 L2
0x854A60 L2
0x854A80 L2
0x854AA0 L2
0x854AC0 L2
0x854AE0 L2
0x854B20 L2
0x854B40 L2
0x854B90 L2
0x854BB0 L2
0x854BD0 L2
0x854BF0 L2
0x854C30 L2
0x854C50 L2
0x854E20 L2
0x854E40 L2
0x854E60 L2
0x854EA0 L2
0x854EC0 L2
0x854F10 L2
0x854F30 L2
0x854F50 L2
0x854F90 L2
0x854FB0 L2
0x854FF0 L2
0x855010 L2
0x855030 L2
0x855050 L2
0x855090 L2
0x8550B0 L2
0x8550D0 L2
0x8550F0 L2
0x855110 L2
0x855150 L2
0x855170 L2
0x855190 L2
0x8551B0 L2
0x8551D0 L2
0x855210 L2
0x855230 L2
0x855250 L2
0x855270 L2
0x855290 L2
0x8552D0 L2
0x8552F0 L2
0x855340 L2
0x855360 L2
0x855380 L2
0x8553A0 L2
0x8553E0 L2
0x855400 L2
0x855450 L2
0x855470 L2
0x855490 L2
0x8554D0 L2
0x8554F0 L2
0x855530 L2
0x855550 L2
0x855570 L2
0x855590 L2
0x8555B0 L2
0x8555F0 L2
0x855610 L2
0x855630 L2
0x855650 L2
0x855670 L2
0x855680 L2
0x8556A0 L2
0x8556C0 L2
0x8556E0 L2
0x855700 L2
0x855720 L2
0x855760 L2
0x855780 L2
0x8557A0 L2
0x8557C0 L2
0x8557E0 L2
0x855820 L2
0x855840 L2
0x855860 L2
0x855880 L2
0x8558A0 L2
0x8558E0 L2
0x855900 L2
0x855920 L2
0x855940 L2
0x855960 L2
0x8559A0 L2
0x8559C0 L2
0x855A20 L2
0x855A40 L2
0x855A80 L2
0x855AA0 L2
0x855AE0 L2
0x855B00 L2
0x855B40 L2
0x855B60 L2
0x855B80 L2
0x855BA0 L2
0x855BC0 L2
0x855C00 L2
0x855C60 L2
0x855C80 L2
0x855CA0 L2
0x855CE0 L2
0x855D00 L2
0x855D50 L2
0x855D90 L2
0x855DD0 L2
0x855DF0 L2
0x855E40 L2
0x855E60 L2
0x855E80 L2
0x855EC0 L2
0x855EE0 L2
0x855F00 L2
0x855F20 L2
0x855F40 L2
0x855F80 L2
0x855FA0 L2
```

## Open items

- 0x6F9DE0 / 0x407F20 hook-plus-native duplicates (STD-10, unchanged).
- The 7 vanished promotions could be removed from `tools/gen/patterns_verified.txt` (left for the record).
- The ai / audio generators' `--check` differences noted in integration-3.md are unchanged (not rerun here).
