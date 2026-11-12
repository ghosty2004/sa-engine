# SAVE-1: Save / Load game (PLAN F5)

Port of the 1.0 US save game path: `C_PcSave`, `CGenericGameStorage`, every per-subsystem `Save` / `Load`
block the save path reaches, and the stats strings. Every new hook is registered `unverified` with a batch,
route and hazards (docs/verify/HOWTO.md); the verdicts are in `docs/arch/save-port-verdicts.tsv`.

## Files and batches

| file | functions | batch |
|---|---|---|
| `crates/sa-core/src/save.rs` | `C_PcSave::SaveSlot` 0x619060, `GenerateGameFilename` 0x6190A0, `DeleteSlot` 0x6190D0; `CGenericGameStorage::ReportError` 0x5D08C0, `OpenFileForReading` / `Writing` 0x5D0D20 / 0x5D0DD0, `CloseFile` 0x5D0E30, `MakeValidSaveName` 0x5D0E90, `GetNameOfSavedGame` 0x5D0F40, `GetCurrentVersionNumber` 0x5D0F50, `SaveWorkBuffer` / `LoadWorkBuffer` 0x5D0F80 / 0x5D10B0, `CheckDataNotCorrupt` 0x5D1170, `SaveDataToWorkBuffer` / `LoadDataFromWorkBuffer` 0x5D1270 / 0x5D1300, `CheckSlotDataValid` 0x5D1380, `GenericSave` 0x5D13E0, `GenericLoad` 0x5D17B0, `GetSavedGameDateAndTime` 0x618D00, `DoGameSpecificStuffAfterSucessLoad` / `BeforeSave` 0x618E90 / 0x618F50; `CSimpleVariablesSaveStructure::Construct` / `Extract` 0x5D1B80 / 0x5D1EA0; `TextCopy` 0x69F910. Natives (not hookable in phase A): `InitRadioStationPositionList` 0x618E70 (lone `ret`), `RestoreForStartLoad` 0x619000 (3 bytes), the `CPad::UpdatePads` thunk 0x541E50 (a `jmp` outside `.HOODLUM`), `CTheScripts::DoScriptSetupAfterPoolsHaveLoaded` 0x5D3390 (its loop branches into the patched entry) | `save-core-1` |
| `crates/sa-core/src/pools.rs` | `CPools::SaveObjectPool` 0x5D4940 (now takes its structure's unwritten bytes from the footprint snapshot) | `save-core-2` |
| `crates/sa-world/src/world/save_blocks.rs` | `CGarages::Save` / `Load` 0x5D3160 / 0x5D3270, `CGarages::CloseHideOutGaragesBeforeSave` 0x44A170, `CGameLogic` 0x5D33C0 / 0x5D3440, `CPickups` 0x5D3540 / 0x5D35A0, `CRestart` 0x5D3620 / 0x5D3770, `CStats` 0x5D3B40 / 0x5D3BF0, `CSetPieces` 0x5D3C70 / 0x5D3CA0, `CShopping` 0x5D3DE0 / 0x5D3E40, `CStuntJumpManager` 0x5D5570 / 0x5D5920, `CEntryExitManager` 0x5D5970 / 0x5D55C0 | `save-world-1` |
| `crates/sa-world/src/world/stats_strings.rs` | `CStats::FindCriminalRatingString` 0x55A210, `CStats::ConstructStatLine` 0x55A780 | `save-stats-1` |
| `crates/sa-ped/src/save_blocks.rs` | `CPlayerInfo::Save` / `Load` 0x5D3AC0 / 0x5D3B00 and their structure 0x5D24B0 / 0x5D2560, `CPedType` 0x5D3CD0 / 0x5D3D10, `CPed::Save` / `Load` 0x5D5730 / 0x5D4640, `CPedSaveStructure::Construct` / `Extract` 0x5D43D0 / 0x5D44B0, `CPlayerPed::Save` / `Load` 0x5D57E0 / 0x5D46E0 | `save-ped-1` |
| `crates/sa-script/src/save.rs` | `CTheScripts::Save` / `Load` 0x5D4C40 / 0x5D4FD0 | `save-script-1` |
| `crates/sa-veh/src/vehicle_save.rs`, `vehicle2.rs` | `CVehicle::Save` 0x5D4760 (registered now), `CAutomobile::Save` / `Load` 0x5D47E0 / 0x5D2980, `CVehicleSaveStructure::Construct` / `Extract` 0x5D2330 / 0x5D23B0 | `save-veh-1` |
| `crates/sa-pop/src/misc2.rs` | `CGangWars::Save` 0x5D5530 (footprint residue), `CGangWarsSaveStructure::Construct` / `Extract` 0x5D2620 / 0x5D2740 | `save-pop-1` |

Not ported: the unwind-only helpers 0x5D28B0 (`~CPedSaveStructure`), 0x5D28D0 (`CPlayerPedSaveStructure`
ctor, no code callers) and 0x5D28F0 (`jmp ~CMatrix`), reached only by exception unwinding. `CStats::BuildStatLine`
0x559230 (called by `ConstructStatLine`) stays original.

## Findings (bytes vs gta-reversed)

- `GenericLoad` checks no version: it calls `GetCurrentVersionNumber()` after `Extract` and drops the result
  (gta-reversed's mismatch handling is NOTSA). Its out flag is only cleared. A block tag that cannot be read
  returns with `ms_bLoading` still set; a wrong tag clears it.
- The block tag is written / read inline against the literal 0xC800 (`SaveDataToWorkBuffer` compares with
  0xC800, `LoadDataFromWorkBuffer` with `ms_WorkBufferSize` but splits at `0xC800 - pos`).
- `SaveWorkBuffer(true)` above 0xC7FC flushes recursively: the flushed bytes are summed twice (kept).
- `CTheScripts::Load` reads the five trailing values `Save` wrote from 0xA444C4 / C0 / BC / B8 / B4 into locals
  and drops them.
- `CStats::Save` first adds stat 0xB0 into stat 0xB1 and clears 0xB0, and calls
  `PopulateFavoriteRadioStationList()` between two of its arrays.
- `C_PcSave::DeleteSlot` probes the deleted file with `CFileMgr::OpenFile` and fails when it still opens.
- `CSimpleVariablesSaveStructure::Extract` runs the cheat functions at 0x8A5CC4 / 0x8A5CC0 for the two cheat
  flags (toggling 0x96918B / 0x96918A when the entry is null); it forces the clock to 1 above 0xF6C36300.
- `CPed::Save` / `Load` construct and destroy the structure's 13 `CWeapon`s with the CRT `eh vector`
  iterators; the constructor's bytes are all overwritten and `~CWeapon` is a lone `ret`, so the ports skip them.
- `CPlayerPed::Save` uses an `and esp, -8` frame: its structure's place depends on the entry ESP; the footprint
  wrapper now records the entry ESP (`sa_core::abi::footprint_entry_esp`).

## Stack residue in the save file (ledger SAVE-1)

The 1.0 code builds several save structures in uninitialised stack locals that their `Construct` fills only
partly and writes whole: `CSimpleVariablesSaveStructure` (`GenericSave`, 0x138 bytes; padding after the
name, +0x1C..+0x67, +0x69..+0x6B, +0x91..+0x93, +0xAE..+0xAF, +0xC9..+0xCB, +0xD1..+0xD3, +0xE2..+0xE3,
+0xEF..+0x11B, +0x11D, +0x12E..+0x12F, +0x137), `CPedSaveStructure` (+0x183, +0x18A, +0x18B),
`CPlayerPedSaveStructure` (+5..+7, +0x81..+0x83), `CObjectSaveStructure` (+0x12..+0x17, +0x19, +0x25..+0x27),
`CSaveGarage` (+3, the name's tail, +0x4D..+0x4F), the player info structure (+7, +0x11..+0x13, +0x25),
`CGangWarsSaveStructure` (+1..+3, +0x4D..+0x4F), `CVehicleSaveStructure` (+0x48, +0x4D, +0x51..+0x53). The
padding after the last block repeats the work buffer (stream position p holds the byte of p - 0xC800), so it
copies those bytes too.

Those bytes are whatever earlier calls left on the stack. Each port that writes such a structure is
registered with `footprint = 0x1000` and takes the unwritten bytes from the wrapper's entry snapshot, which
is the original's residue whenever its caller is original code (every L2 / split run). With the callers on
Rust the residue is Rust's (the frames differ), so a save written with every port on Rust differs from the
original's in exactly those positions; `tools/accept/save-diff.py --mask` locates them and checks that no
other byte differs. Loading never reads them back (the `Extract` functions skip the same bytes).

## Verification

Lanes 6 and 7 (`SA_ROOT=/home/ghosty2004/sa-engine`), DLL and host tools built from this branch; outputs under
the worktree's `target/e2e/lane-{6,7}/port-l2-*` (spools deleted after each report).

### Harness additions

- **Harness pokes** (`crates/sa-oracle-format/src/program.rs`, `crates/sa-hook/src/detwin.rs`): a program line
  `FRAME poke=0xADDR:0xVV` writes one byte at that frame boundary (after its hash, before its `CTimer::Update`);
  the pokes go into the recording header (`pokes`) and every replay applies them. They stand in for the two
  inputs no pad input reaches from the corpus start: the save point request (0xBA67A7, the byte
  ACTIVATE_SAVE_MENU sets) and the Load Game screen of the PC pause menu (screen byte 0xBA68A5 = 9).
- **Save files** (`tools/e2e/lib.sh`): every game start and every cleanup removes `GTASAsf1..8.b` from the lane
  prefix's user files (the boot reads them: no run may see another run's saves); `port-l2.sh`, `l3.sh` and
  `record-program.sh` move a run's save files next to its trace (`RUN.GTASAsfN.b`); the plain cache keeps them.
  `port-l2.sh` check (f): a Rust run's save files are byte-identical to plain's (rust, l2) or, for the split
  runs, to splito's (both call the sampled hooks through the L2 invoker). `verify-all.sh` turns a failed (f)
  into a failed recording and notes a passed one.
- **`tools/accept/save-diff.py A.b B.b [--mask]`**: block-by-block comparison of two saves; `--mask` allows only
  the stack-residue bytes (previous section) and their copies in the padding, and checks both checksums.
- **`tools/accept/f5-saves.sh LANE`** (PLAN F5): the save-load recording with every save and load port on Rust
  (`all`), only the save side (`save`: the original loads the Rust save) and only the load side (`load`: Rust
  loads the original's save); each must replay IDENTICAL to the all-original run, and its save file is
  compared raw and with the mask.

### Recording save-load (docs/e2e/corpus.md)

`tools/e2e/programs/save-load.txt`, 3,900 frames: grove-walk start, SZCMAWO (the first mission fails, so no
mission script is saved: a save made during the mission crashes the original itself on load, in
`CTheScripts::Process`, 0x469FF7 calling address 9), hospital respawn, the save menu through the poke, the
cheat warning, slot 1, "Save Successful", then Load Game of slot 1 from the pause menu; CJ restarts at County
General and walks. Recorded on lane 7, replay == record IDENTICAL over 3,900 frames, record and replay save
files identical (sha256 40be2e70...). Canonical copy: the main checkout's `target/e2e/corpus/save-load.sarec`
(the save file it writes is never committed).

### Split runs (per-hook verdicts: save-port-verdicts.tsv)

A split run of all 66 hooks at once (`save1`) shows why one run is not enough: a port's caller on Rust moves
the stack, so the original callees below it read other residue, the work buffer and `ms_CheckSum` differ, and
every later call's inputs diverge. The hooks were therefore split into call-depth groups whose members never
call each other (`sg1-g1..g4`, `s3-g5`), then the hooks whose inputs a residue difference still reached were run
alone (`s2-*`, `s3-*`, `s5-*`), and the residue readers with `SPLIT_STACKFILL=4096` (`s4-gsf*`, `s6-ggw`: the
stack below the invoker is zeroed before every sampled call in both runs, so original and port read the same
residue; there the save files of splito and splitr are compared too).

Results (every split run: splito and splitr traces IDENTICAL to the all-original trace on every frame):

- **59 hooks L2** (promoted): PASS, PASS-FP or COUNT with every compared case matching and 0 undeclared writes;
  the cited run per hook is in `save-port-verdicts.tsv`. `SaveDataToWorkBuffer` / `LoadDataFromWorkBuffer`
  end in COUNT because the 400 MB spool quota stops the run (822 / 939 cases compared, no mismatch);
  `ConstructStatLine` / `FindCriminalRatingString` (menus-hud) in COUNT because splito / splitr sample a few
  stats-page calls more or less (714 / 706; 706 compared, no mismatch). With `SPLIT_STACKFILL=4096` the save file
  written with `CPed::Save` + `CSimpleVariablesSaveStructure::Construct` (`s4-gsf2`) or `CGangWars::Save`
  (`s6-ggw`) on Rust is byte-identical to splito's; so are those of the leaf group (`s5-gleaf`),
  `LoadDataFromWorkBuffer` and `GenericLoad` (`s5-*`).
- The first split (`save1`) found `CTheScripts::Load`'s result: the bytes return the loaded script count in EAX
  (AL 0x1C), not 1; fixed. The undeclared writes the first runs listed are now `tools/e2e/l2-extras.txt`
  lines (section SAVE-1): CRT stdio state, file name buffers, the pad states `UpdatePads` writes after a save,
  the arrays the block loads fill whole, the world state the after-load step changes, and the save structures
  the `Construct` functions fill (`D` regions).
- **Ledger** (not promoted): `GenericSave`, `C_PcSave::SaveSlot` (below).
- **No corpus case**: `CVehicle::Save`, `CAutomobile::Save` / `Load`, `CVehicleSaveStructure::Construct` /
  `Extract` (no vehicle of the save-load recording has +0x4A4 == 2), `C_PcSave::DeleteSlot`, `ReportError`,
  `CloseFile` (error paths only).

### Ledger SAVE-1 (stack residue)

- `GenericSave` and `C_PcSave::SaveSlot` on Rust: the original block functions (`CPed::Save`, `CPools::Save`
  ...) they call run on a Rust frame and read other stack residue into their uninitialised structures, so the
  written bytes differ in exactly the residue positions: split `s4-gsf3` (GenericSave, stackfill) 486 bytes,
  `s4-gsf4` (SaveSlot) 108 bytes, every one inside the save-diff mask, checksums valid; the only mismatching
  output is `ms_CheckSum`. The trace after the load is IDENTICAL (the `Extract` functions skip those bytes). Kept
  `unverified` with this evidence; promoting them is a decision for the main session (the residue cannot be
  modelled: it comes from the frames of the original callees' own callees).
- With every save port on Rust the save file differs from the original's in residue positions only
  (`save1` rust run: 570 bytes, all inside the mask) and the game state after loading it is IDENTICAL.

### F5 (tools/accept/f5-saves.sh 6, tag f5a)

| set | ports on Rust | trace vs all-original | save file vs the original's |
|---|---|---|---|
| all | every save and load port (incl. the unverified GenericSave, SaveSlot, vehicles) | IDENTICAL, 3,900 frames | identical except stack residue (710 bytes, all inside the mask; checksums valid) |
| save | the save side only: the original loads the Rust save | IDENTICAL, 3,900 frames | identical except stack residue (710 bytes) |
| load | the load side only: Rust loads the original's save | IDENTICAL, 3,900 frames | IDENTICAL |

So: original saves load in Rust and Rust saves load in the original with identical state hashes after the
load; the save bytes are identical except the uninitialised-stack bytes of the ledger entry.

### All ports (verify-all)

`tools/e2e/verify-all.sh save1va 6,7` with the 59 promoted ports in the default set: **IDENTICAL on all 13
recordings** (grove walk 5,200, air-water 5,800, anim-input 3,300, car-ai 5,000, c-bug 3,220, driving 4,338,
menus-hud 5,700, menus-map-controls 4,000, pause-restart 3,700, police-gang 5,200, save-load 3,900,
weapon-combat 3,500, world-cheats 3,276 frames). On save-load the default set writes a save that differs from
the original's in 653 residue bytes only (`GenericSave` stays original, but its block callees run on Rust);
`verify-all.sh` now accepts a save difference that `save-diff.py --mask` classifies as residue only and says so
(`save1va2`).

### Gate

`tools/verify/writer-gate.sh 1c3e9e0` (GATE_SKIP=l1test): clippy (host, all targets), i686 release build and
clippy, wasm32 build of sa-ports, sa-hook lib tests (58) PASS; the `hooks` step lists the 59 promoted hooks as
problems (this branch both wrote and verified them).

## Open items

- `GenericSave` / `C_PcSave::SaveSlot`: ledger SAVE-1 (stack residue below a Rust frame); promotion is the main
  session's call.
- The vehicle save functions, `DeleteSlot`, `ReportError`, `CloseFile` have no corpus case (a recording with a
  vehicle stored in a safe house garage, a delete from the Load / Delete menu, and a corrupt save would cover
  them).
- The harness pokes stand in for a save point and the main menu's Load Game: a recording that reaches a real
  save point (after the first mission) would cover the script path to the save menu.
