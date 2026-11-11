# PORT-SCRIPT-1: the SCM script VM (crate `sa-script`)

Scope (task PORT-SCRIPT-1): (1) the `CRunningScript` VM core: `ProcessOneCommand`, `Process`, the
operand collectors/storers, `ReadTextLabelFromScript`, the variable-address helpers, `UpdateCompareFlag`,
`UpdatePC`; (2) the `ProcessCommandsXToY` switches holding the opcodes SA:MP executes
(`docs/samp/scm-opcodes-dynamic.csv`, 120 opcodes in 26 of the 27 switches), each replaced as a whole
function. This pass ports (1) completely and, of (2), `ProcessCommands0To99` as a whole switch; the
other 25 switches are not ported (see "Switches not ported" for why and what they need).

Follows ADR-001 and the PORT-WORLD/PORT-PHYS pattern: `#![no_std]`, `#![deny(unsafe_code)]`, game state
through `globals!`/`GPtr`, every other catalogued function through `orig!`, every port registered with
`#[hook]` and listed in `hooks::ALL` (sa-hook appends `sa_script::hooks::ALL` after sa-world's; sa-oracle's
preserve lookup and the INF-20 callers test include it). Host, i686 (`in-process`) and wasm32 build and
pass clippy `-D warnings`.

Verdict TSV (one row per hook): `docs/arch/script-port-verdicts.tsv`.

## Crate layout

| file | contents |
|---|---|
| `vm.rs` | script globals (`ScriptParams` 0xA43C78, `CommandsExecuted` 0xA447F4, `FailCurrentMission` 0xA444B0, `LocalVariablesForCurrentMission` 0xA48960, `ScriptSpace` 0xA49960 (269,000 bytes), `CHud::m_BigMessage[1][0]` 0xBAAD40, the handler table 0x8A6168 with its 27 initial pointers), the `CRunningScript` field offsets, the 14 VM-core ports, `orig!` declarations of every VM helper (the switches call them through these) |
| `cmd0.rs` | `ProcessCommands0To99` (0x465E60) and its globals (`bAlreadyRunningAMissionScript` 0xA444B1, `pIdleScripts` 0xA8B428, `pActiveScripts` 0xA8B42C) |
| `hooks.rs` | `MODULES`/`GLOBALS`/`ALL`, tests: initial values against the exe image, no overlap with sa-core/sa-stream/sa-world globals, every `#[hook(` listed, unique addresses |

## Conventions

- **Inlined vs called helpers.** Where the original inlines a helper the port inlines it (the local
  variable lookup `m_ThisMustBeTheOnlyMissionRunning ? &LocalVariablesForCurrentMission[i] :
  &m_LocalVars[i]` and the array-operand decode inside `CollectParameters`, `StoreParameters`,
  `ReadParametersForNewlyStartedScript`, `GetIndexOfGlobalVariable`; `UpdatePC` inside `Process`). Where
  it calls one (`CollectNextParameterWithoutIncreasingPC` → `GetPointerToLocalVariable`,
  `ReadArrayInformation(0)`, `GetPointerToLocalArrayElement`; `ReadTextLabelFromScript` → those and CRT
  `strncpy` 0x821F40; `GetPointerToScriptVariable` → `ReadArrayInformation(1)`,
  `GetPointerToLocalArrayElement`) the port calls it through `orig!`, so the L2 callee sequence and the
  hook-off check match.
- **Preserved ECX.** sa-hook's INF-20 callers test flagged callers that keep `this` in ECX across
  `GetPointerToLocalVariable`, `GetPointerToLocalArrayElement`, `ReadArrayInformation`,
  `CollectParameters` (e.g. 0x465EC5 → `UpdatePC`, 0x466252 → `GetPointerToScriptVariable` without
  `mov ecx, esi`) and `UpdateCompareFlag` (0x486FC2, 0x488E1A). These and the other VM helpers whose
  original never writes ECX (`CollectNext...`, `StoreParameters`, `ReadParametersForNewlyStartedScript`,
  `GetIndexOfGlobalVariable`, `UpdatePC`) are registered with `preserve = "ecx"`. With that the callers
  test passes (45/45 sa-hook lib tests, keycells and `every_hook_installs_on_the_exe` included).
- **Narrow arguments.** `int16 count`, `uint8 length`, `bool state` are taken as their stack dword and
  narrowed as the original reads them (`CollectParameters`: `test ax,ax; je` then `movzx` — a negative
  count runs up to 65,535 slots; `StoreParameters`: `test ax,ax; jle` — signed; `UpdateCompareFlag`: the
  raw byte is stored, not normalised to 0/1, unless `m_NotFlag`).
- **Switch handler calls.** `Process` and `ProcessOneCommand` call `s_OriginalCommandHandlerTable[id/100]`
  read from 0x8A6168 at run time (`orig!` `thiscall dyn`), pushing the masked id (0..0x7FFF) with
  `this` in ECX, as the exe does at 0x469FEB/0x469EED.
- **Floats in the switch.** `fld a; fop b; fstp` → `X87::from_f32(a).fop_m(b).to_f32()` (PC=24, memory
  operand NaN rules); operand order as in the bytes (`ADD_FLOAT_LVAR_TO_FLOAT_VAR` 0x5F loads `b` first).
  Compares: `test ah,0x41; je` = `>`, `test ah,1; je` = `>=`, `test ah,0x44; jnp` = `==`, all false when
  unordered, which are the IEEE f32 relations on the loaded values.

## Divergences from gta-reversed kept (the exe wins)

- `ReadTextLabelFromScript` 0xF (static 16-byte string): raw byte copy of `min(length,16)` bytes, then
  `m_IP` skips the rest of the 16 (`length == 0` copies nothing); gr uses `strncpy` (stops at NUL, pads).
  Long-string variables/arrays: `strncpy(buf, p, length < 16 ? length : 16)`; the local long-string
  variable form passes the length to `strncpy` as gr does, the short forms always 8.
- `GetPointerToScriptVariable`: types 4/5/6/9/0xE/0xF and unknown types return NULL (gr: unreachable).
- `GetIndexOfGlobalVariable`: unknown types return the low word of `this` (the word pushed to save ECX)
  and consume only the type byte (gr: unreachable).
- `CollectParameters`: unknown operand types leave their `ScriptParams` slot untouched but still use it.
- 0052 COMMAND_LINE collects its 6 parameters and returns 0 (gr: unimplemented → -1 without reading
  them; specs `ProcessCommands0To99.md`).
- 004F START_NEW_SCRIPT: a negative offset is replaced by the command id 0x4F (`mov eax,[esp+0x2c]`).
- CREATE_PLAYER: ground z when `z <= -100` (ordered), then `+ GetDistanceFromCentreOfMassToBaseOfModel`
  (x87, memory operand z), `CTaskSimplePlayerOnFoot` set in slot 4.

## Divergence notes (ledger candidates)

- `GetIndexOfGlobalVariable` returns a `uint16`; the original's upper EAX half is the caller's EDI upper
  half (type 7) or `type - 7` (unknown types). The port returns the word zero-extended. Callers that read
  only AX are unaffected; no call in the recordings, not measured.
- The float compares of `ProcessCommands0To99` write the 0/1 flag into the low byte of the switch's own
  command-argument stack slot (`mov byte [esp+0x2c], 1`) before pushing it; the slot is dead after `ret 4`
  (callee-cleaned). The port does not write it.

## Verification

All runs on lane 4 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=4`), outputs under
`target/e2e/lane-4/port-l2-<tag>/` of the worktree. Recordings: grove walk
(`target/e2e/lane-2/inf4/rec.sarec`, 5200 frames), pause-restart (`target/e2e/corpus/pause-restart.sarec`,
3700), world-cheats (`target/e2e/corpus/world-cheats.sarec`, 3276).

| state (agent proposal) | rows |
|---|---|
| L2 | 9 (ReadArrayInformation, ReadTextLabelFromScript, CollectParameters, CollectNextParameterWithoutIncreasingPC, StoreParameters, ReadParametersForNewlyStartedScript, GetPointerToScriptVariable, UpdatePC, UpdateCompareFlag) |
| implemented (L3 + hook-off only) | 2 (Process, ProcessCommands0To99) |
| implemented-needs-L2 | 4 (GetPointerToLocalVariable, GetPointerToLocalArrayElement, GetIndexOfGlobalVariable, ProcessOneCommand: no call in any recording) |

15 hooks in all.

- **L2 double run**, `tools/e2e/lists/script-core.txt` (the 13 VM hooks without Process),
  `--runs "plain l2"`: grove (`sc-grove` L2_PER_FRAME=20, then `sc-grove2` L2_PER_FRAME=1000 for
  CollectParameters/CollectNext/StoreParameters after adding their extras), pause-restart (`sc-pr`) and
  world-cheats (`sc-wc`, both L2_PER_FRAME=1000): every called hook >= 1000 cases or every call,
  0 mismatches, 0 undeclared writes; (b) l2 trace == plain on every frame. Per-recording counts in the TSV.
  `sc-grove` first showed undeclared writes of CollectParameters (ScriptParams 0xA43C79..) and
  StoreParameters (ScriptSpace 0xA4996C..), orig and Rust alike: fixed by the extras lines (section
  PORT-SCRIPT-1 of `tools/e2e/l2-extras.txt`).
- **L3** (`--runs "plain rust"`, `tools/e2e/lists/script-all.txt` = all 15 SCRIPT hooks on Rust, Process and
  the 0To99 switch included): grove (`sa-grove`) IDENTICAL over 5200 frames, pause-restart (`sa-pr`) 3700,
  world-cheats (`sa-wc`) 3276, all 17 subsystems. All engine ports on Rust (`port-l2.sh ports`, 1208 hooks
  incl. these 15, `ports-grove`): IDENTICAL over 5200 frames. The 0To99 switch alone on Rust (`sc0-grove`):
  IDENTICAL over 5200 frames.
- **Hook-off** (`sa-grove`, `--runs hookoff`, every SCRIPT hook on Rust except one): 14 of 15 IDENTICAL over
  5200 frames; `hookoff-4` (CollectParameters back on the original) reached frame 5200 and exited
  normally but left no trace file, so (d) was not evaluated for it (harness; rerun needed).
- **Tests**: `cargo test -p sa-script` (3 pass: initial values vs the exe image, no overlap with the other
  crates' globals, list complete/unique); `cargo test -p sa-hook --lib` 45/45 (INF-20 callers test,
  `every_hook_installs_on_the_exe`, keycells `every_code_key_read_of_a_hooked_range_is_redirected`,
  `port_hooks_follow_the_local_ones`); clippy `-D warnings`: sa-script host (all targets), i686
  (`in-process`), wasm32; sa-hook i686; sa-oracle host.

## Switches not ported (PORT-SCRIPT-1 state; superseded by "PORT-SCRIPT-2" below)

The 120 SA:MP opcodes live in 26 of the 27 switches (all except `ProcessCommands2600To2699`). A switch is
replaced as a whole function, so porting them means porting the 26 whole switches:

| switch | addr | insns | direct calls | SA:MP opcodes |
|---|---|---|---|---|
| ProcessCommands0To99 (ported, cmd0) | 0x465E60 | 1160 | 168 | 1: 0053 |
| ProcessCommands100To199 (not ported) | 0x466DE0 | 1721 | 205 | 5: 009A 00A5 00A6 00BE 00C0 |
| ProcessCommands200To299 (not ported) | 0x469390 | 733 | 105 | 4: 00FE 0107 0108 0109 |
| ProcessCommands300To399 (not ported) | 0x47C100 | 1268 | 142 | 14: 0158 0159 015F 0160 0161 0164 0165 0168 016C 0173 0175 0187 018B 018C |
| ProcessCommands400To499 (not ported) | 0x47D210 | 1032 | 103 | 5: 01B4 01B9 01E8 01EB 01F0 |
| ProcessCommands500To599 (not ported) | 0x47E090 | 1350 | 127 | 7: 01F5 0213 0214 0215 0247 0248 0249 |
| ProcessCommands600To699 (not ported) | 0x47F370 | 526 | 57 | 5: 02A3 02A7 02A8 02AB 02AC |
| ProcessCommands700To799 (not ported) | 0x47FA30 | 1619 | 159 | 3: 02BF 02CE 02EB |
| ProcessCommands800To899 (not ported) | 0x481300 | 2484 | 175 | 4: 0321 0362 036A 0373 |
| ProcessCommands900To999 (not ported) | 0x483BD0 | 1921 | 211 | 2: 038B 03DE |
| ProcessCommands1000To1099 (not ported) | 0x489500 | 891 | 126 | 3: 03FE 0430 0446 |
| ProcessCommands1100To1199 (not ported) | 0x48A320 | 1308 | 141 | 2: 0453 0470 |
| ProcessCommands1200To1299 (not ported) | 0x48B590 | 1619 | 196 | 6: 04BB 04CE 04D7 04E4 04ED 04EE |
| ProcessCommands1300To1399 (not ported) | 0x48CDD0 | 1223 | 126 | 3: 052C 053F 0570 |
| ProcessCommands1400To1499 (not ported) | 0x48EAA0 | 2317 | 238 | 4: 05C4 05CA 05CB 05CD |
| ProcessCommands1500To1599 (not ported) | 0x490DB0 | 2052 | 300 | 1: 060B |
| ProcessCommands1600To1699 (not ported) | 0x493FE0 | 2130 | 267 | 8: 064C 0650 066A 0681 0682 0685 069A 069B |
| ProcessCommands1700To1799 (not ported) | 0x496E00 | 2345 | 291 | 10: 06BD 06D5 06D6 06D7 06D8 06E7 06E8 06E9 06EA 06ED |
| ProcessCommands1800To1899 (not ported) | 0x46D050 | 2112 | 256 | 6: 070A 070D 0713 0729 0750 075A |
| ProcessCommands1900To1999 (not ported) | 0x46B460 | 1660 | 234 | 4: 077E 07AC 07BD 07C7 |
| ProcessCommands2000To2099 (not ported) | 0x472310 | 2275 | 244 | 3: 07FE 07FF 0812 |
| ProcessCommands2100To2199 (not ported) | 0x470A90 | 1606 | 196 | 4: 0840 0860 087E 0893 |
| ProcessCommands2200To2299 (not ported) | 0x474900 | 1727 | 215 | 6: 08D2 08D4 08D7 08D9 08DA 08DB |
| ProcessCommands2300To2399 (not ported) | 0x4762D0 | 1963 | 201 | 6: 0920 0925 092F 0930 0936 0948 |
| ProcessCommands2400To2499 (not ported) | 0x478000 | 29 | 0 | 2: 09A2 09BA |
| ProcessCommands2500To2599 (not ported) | 0x47A760 | 1715 | 241 | 2: 09C4 09DB |
| ProcessCommands2600To2699 (not ported) | 0x479DA0 | 663 | 74 | 0:  |

Together ~44,000 instructions with ~4,900 direct calls into every subsystem (peds, vehicles, tasks,
camera, HUD, text, streaming, audio, garages, pickups, ...), plus the inlined task constructors and the
x87 code of each case. `ProcessCommands0To99` (1,160 instructions, 85 cases) took one pass here. At that
rate the rest is ~20–25 dedicated passes, one or two switches each. Why it could not be done in this pass:

1. Size (above). The SA:MP opcodes are a small part of each switch (1–14 of ~50–100 cases), but a
   partial switch cannot be hooked: the cases share one prologue/SEH frame and jump table, and an
   engine-crate port has no access to the original body (sa-hook gives engine ports no trampoline), so a
   Rust switch cannot fall back to the original for unported cases.
2. Many cases construct tasks/objects with inlined constructors and EH state (as CREATE_PLAYER does
   with `CTaskSimplePlayerOnFoot`), which have to be read from the bytes one by one; the
   `docs/re/specs/SCRIPT/commands/*.md` specs cover the unregistered/deviating cases (948), the ~1,500
   gta-reversed handlers still need checking against the bytes.
3. L2 of a whole switch is a double run of every case it executes (see Open items: the double-run L2 of
   `ProcessCommands0To99` did not finish); the switches need split-run L2 (INF-17), which is one run per
   switch and recording.

Options for the main session: (a) continue switch by switch in later passes (order by SA:MP use and
size: 200To299 (733 insns), 600To699 (526), 300To399 (2400To2499: its catalogue extent stops at an unresolved computed jump, size unknown),
1000To1099, ...); (b) give engine ports access to the original body (a trampoline pointer per hook in
`HookReg`, written by sa-hook at install) so a Rust switch can port only the SA:MP cases and fall back to
the original for the rest: smaller per-switch passes, but the hook would no longer be a whole-function
replacement until every case is ported (needs a decision, ADR-001 change).

## Open items

- `ProcessCommands0To99` double-run L2 (`sc0-grove`, L2_PER_FRAME=50, extras: script globals, lists and
  ScriptsArray): 5 cases compared, 0 mismatches, then the game stopped advancing at frame 202 (100% CPU)
  until the 1500 s timeout. Not investigated; a whole switch executes cases with side effects outside any
  declared region (START_NEW_SCRIPT/TERMINATE move scripts, CREATE_PLAYER creates a task), so the switch
  and `Process` should be verified with split runs (`--runs "plain split"`, INF-17) instead. Not run in
  this pass.
- `Process` has no L2 evidence (it runs every command of a script; split run needed); L3 + hook-off only. Done in PORT-SCRIPT-2: split PASS (grove, 1000 pairs).
- Hook-off of CollectParameters: rerun (missing trace file, see above). Done in PORT-SCRIPT-2: IDENTICAL over 5200 frames.
- 4 hooks without calls in the three recordings (GetPointerToLocalVariable, GetPointerToLocalArrayElement,
  GetIndexOfGlobalVariable, ProcessOneCommand — the last one has no caller in the exe at all; sa-hook's
  `SA_COV_SCM` coverage detour also targets 0x469EB0's first instruction, the two must not be used together).
- The 25 switches not ported (above), and with them the SA:MP opcodes outside 0To99 (0053 CREATE_PLAYER is
  in 0To99 and ported).
- Catalogue rows: the main session sets `catalog/state.csv` from the TSV (agent did not edit it).

# PORT-SCRIPT-2: partial switch ports (ADR-002)

Decision (main session, ROADMAP PORT-SCRIPT): the 25 remaining `ProcessCommandsXToY` switches are ported
case by case with a fallback to the original body for unported cases, SA:MP opcodes first. Mechanism:
`docs/arch/ADR-002-partial-ports.md` (`#[hook(..., fallback)]`, `<fn>_original` = the original-mode
trampoline; phases B/C panic naming switch and opcode). Verdicts: `docs/arch/script-port-verdicts.tsv`
(one row per switch: ported cases / cases, SA:MP opcodes ported, split verdict per recording, the ported
opcodes with split PASS evidence).

## Layout

| file | contents |
|---|---|
| `cmdlib.rs` | `fallback(orig, NAME, s, cmd)`, `ScriptParams` helpers (`sp`/`spi`/`spf`/`set_sp`), `set_flag`, `fld_fst`, pool getters and common `orig!` declarations, the shared short-range coordinate blip (04CE/0570) |
| `cmd100.rs` … `cmd2500.rs` | one partial switch each: `PORTED` (ascending opcodes with a Rust case), the `#[hook(..., fallback)]` switch, cases as private functions or inline arms, each with opcode, name, case address and the deviations kept from the exe |
| `hooks.rs` | `PARTIAL` (switch, `PORTED`); test `partial_switches_list_their_rust_cases` (every switch is a fallback hook, every listed opcode has an arm of the switch's `match command`, every arm is listed) |

## Cases ported

343 opcodes in the 25 switches (opcodes with a jump-table case: 1,845; `PORTED` per switch in the TSV).
SA:MP (`docs/samp/scm-opcodes-dynamic.csv`, 120 opcodes = the samp-rs list + the dynamic measurement):
114 of the 119 outside `ProcessCommands0To99` (0053 was ported in pass 1). Not ported (fallback):

| opcode | case | why |
|---|---|---|
| 0321 EXPLODE_CHAR_HEAD | 0x48135B | builds `CPedDamageResponseCalculator` + `CEventDamage` on the stack with SEH state |
| 066A CREATE_FX_SYSTEM_ON_CHAR_WITH_DIRECTION | 0x494A6E | 32-byte label, an `fcom` result kept on the x87 stack across branches, stack matrix temporaries |
| 06D8 CREATE_MISSION_TRAIN | 0x497F85 | result is an out-parameter `CTrain::CreateMissionTrain` (0x6F7550, not reversed) writes only inside its carriage loop: not confirmed it is always written |
| 08D4 CREATE_MENU, 08DB SET_MENU_COLUMN | 0x475500, 0x4756B4 | strcpy-style loop over 8-byte labels without NUL reads/writes stale stack bytes |

Other cases left on the fallback on purpose: 0174/0176 (`CPlaceable::GetHeading` returns a raw FPATAN
ST0, not exact through f64), 02F6/02F7 (FSIN/FCOS), the 8-byte text-label cases passed to
`TheText.Get`/`GetGarageNumberByName` (0299, 02B9, 02E4, 0360, 0361, 03B0, 03B1, 03D5, 03D6: an 8-character
label has no NUL, the callee reads stale stack bytes).

## Divergences from gta-reversed kept (the exe wins)

- 0159 POINT_CAMERA_AT_CHAR: the exe passes nullptr to `TakeControl` for a player, the ped otherwise (gr: reversed).
- 01B9 SET_CURRENT_CHAR_WEAPON: P1 == 0 selects slot 0 directly (gr loops and ends on the last slot holding type 0).
- 036A/0430: `m_bIsWarpingPedIntoCar` is set after the task constructor; 0430 computes the door only for seat >= 0 (signed).
- 02CE GET_GROUND_Z_FOR_3D_COORD: a pointer to a local bool is passed as `outResult` (gr: nullptr).
- 0173 SET_CHAR_HEADING: one wrap in ST0 (+360 when < 0, -360 when > 360); "in vehicle" tested inline (+0x46C bit 8, +0x58C).
- 01A1..01AA, 01AB/01AC, 01AD..01B0, 0202..0207, 0471..0476: gr `unimplemented`, the exe calls the
  `*AreaCheckCommand`/`Locate*Command` helpers with the command.
- 0936: duration is `fild SP6` (gr reads a float); 07FE: gr's registration is commented out, the exe has the case;
  0920/0936: `(&SP3..5, &SP0..2)` argument order (matches gr's swapped names).
- Original bugs kept: 0296 decrements ScriptParams[0] in place; 041E rewrites P0 11 → 12 and increments only the
  low byte of the radio channel; 0180 skips the operand type byte unchecked; 04D7/053F/04EE do not null-check.
- 05CA/05CB copy the dword at 0x8D237C (unnamed) into the task and reset it to 6.
- 1700To1799 stores through the protection stub 0x4021A4 (`mov [[0x1560728] + [0x5D720D] + 1], eax`), decoded
  from the image: 0xA43C78 = ScriptParams[0]; ported as `SP0 = eax`. Valid while 0x5D720D's bytes are unhooked
  (INF-20 scan covers it).

## Divergence notes (ledger candidates)

- Text-label buffers (04ED, 04EE, 075A): the original's stack buffer is not cleared before
  `ReadTextLabelFromScript`; the port's starts zeroed (075A: one extra zero byte). Differs only for a label
  that leaves its terminator to stale stack bytes (not observed).
- Bools pushed with stale upper bytes (06BD, 0920, 0936, 0713, the UpdateCompareFlag `setcc` forms) are
  passed as 0/1; the callees read the low byte (checked for 0x6217D0 and UpdateCompareFlag).
- Dead stack writes not ported: EH state slots, `operator new` result slots, flag/temp bytes in the
  command-argument slot, the 700To799 prologue's `[esp+0x54] = 0`.
- 800To899: every path ends at the common tail that runs `~CMatrix` (0x59ACD0) on a frame matrix the prologue
  zeroes; ported cases call `~CMatrix` on a zeroed local to keep the callee sequence (no-op with zeroed fields).
- 036A/0430 build their task on the Rust stack (0x1C/0x20 bytes) as the original builds it on its frame.

## Verification (lane 4, worktree outputs `target/e2e/lane-4/port-l2-<tag>/`)

- **Mechanism, all fallback** (`s2-fb-grove`, the 25 switch hooks with no Rust case, all 40 SCRIPT hooks on
  Rust): L3 IDENTICAL over 5200 frames, 17 subsystems.
- **All-ports L3** (`port-l2.sh ports`, 1,306 hooks incl. the 40 SCRIPT hooks with the 343 Rust cases):
  grove (`s3-ports-rec`) 5200, pause-restart 3700, world-cheats 3276, c-bug 3220 frames: all IDENTICAL.
- **Split runs** (INF-17) of the 25 switches, `SPLIT_PER_FRAME=2 --max 300`, per recording (`s3-split-*`):
  see the TSV (verdict per switch and recording) and `opcodes.txt` in each run directory
  (`tools/e2e/split-opcodes.py`: opcodes of the recorded calls; `*` = Rust case). A ported opcode is PASS
  when it appears in a switch whose split verdict is PASS. Result: all 25 switches SPLIT PASS on all four
  recordings (100 verdicts, up to 300 pairs each, 0 mismatches, no DIVERGED/COUNT). 60 ported opcodes
  were executed and compared, 31 of them SA:MP opcodes: 00A5 00BE 00C0 0107 015F 0160 0164 0173 0175 018B
  01B4 01F0 01F5 0213 0247 0248 02A3 02A7 02A8 02EB 0373 0453 04BB 04CE 04E4 0570 060B 070D 077E 0925 09BA.
  The other 83 ported SA:MP opcodes were not executed by the recordings (L3 only).
- **Split runs of `Process` and `ProcessCommands0To99`** (`s2-split-vm`, grove): both SPLIT PASS, 1000 pairs
  compared each, 0 mismatches; 0To99's pairs cover 0001 0002 0004 0005 0015 001A 004D 0053.
- **Hook-off of CollectParameters** (`s2-hookoff-cp`, `HOOKOFF_ONLY=0x464080`, every other SCRIPT hook on
  Rust): IDENTICAL over 5200 frames (pass 1's missing trace is resolved).
- **Undeclared writes**: every split run of a switch (and of `Process`, 0To99) reports writes outside the
  declared sets (`FAIL (e) split-report: 0 undeclared writes`): a switch reaches every subsystem through its
  cases, and `catalog/rwsets.csv` + `l2-extras.txt` declare only the script globals. The outputs (regions,
  window writes, registers, callee sequence) are compared regardless; the undeclared check would need a
  write set per case (or a `D` region over `.data/.bss`, which makes it vacuous as for the L2-EXTRAS
  orchestrators). Decision for the main session.
- Tests: `cargo test -p sa-hook --lib` 45/45 (INF-20 install scan, keycells, callers incl. the 25 new hooks);
  `cargo test -p sa-script` 4/4; clippy `-D warnings` clean: sa-script host (all targets), i686
  (`in-process`), wasm32; sa-hook i686; sa-oracle, sa-macros, sa-core host.

## Open items (PORT-SCRIPT-2)

- The 5 SA:MP opcodes above and ~1,500 non-SA:MP cases remain on the fallback; no switch is fully ported.
- Ported opcodes never executed in the four recordings have no L2 evidence (state `implemented`); SA:MP's
  own opcodes need a recording with SA:MP-like scripts (e.g. a corpus script running the SA:MP opcode set).
- The undeclared-write policy for switch hooks (above).
