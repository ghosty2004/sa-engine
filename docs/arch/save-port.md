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

(see below; filled in by the runs)
