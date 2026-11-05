# CAT-12b: code reached only through data

CAT-12 closed the catalogue over direct calls, tail calls and gta-reversed vtable slots. Code that the game
reaches only through a pointer stored in data (a function-pointer table, a struct field, an
initialiser table, an immediate passed as a callback) was still missing. Examples: 24 of the 27
`ProcessCommandsXToY` functions and 58 of the 65 cheat functions.

`tools/catalog/data_refs.py` finds these entries. `run_all.sh` runs it after `callgraph.py gaps`.
When it adds rows, the catalogue loop reruns from `build_catalog` in the same way as CAT-12.

## Result

| | before | after |
|---|---:|---:|
| catalogue functions (game-text + rw-cseg) | 19,425 | **22,161** |
| non-excluded (live) functions | 12,761 | **15,485** |
| data-referenced entries (`catalog/data-code-addresses.csv`) | 0 | **2,720** (12 of them excluded third-party/CRT) |
| CAT-7 call targets (`extra-code-addresses.csv`) | 46 | 62 (+16 found in the new bodies) |
| accepted data / immediate entries outside the catalogue | | **0** |
| `noncatalogue_targets` (CAT-7 call/vtable) | 0 | 0 |

All 2,720 new rows are `unknown` in `procedures.csv`. Their `ghidra_name` is
`(missed by ghidra: data-<kind>)`. 97 of them get a plugin-sdk name through CAT-5: all 24 script range
functions, all 58 cheat functions and 15 callbacks.

Entries by kind (the `kind` column of `data-code-addresses.csv`; `merge_ghidra.py` rule `data-<kind>`):

| kind | entries | what |
|---|---:|---|
| `crt-init` | 1,666 | C++ dynamic initialisers in `__xc_a..__xc_z` (0x8a4000..0x8a5a10), which `_cinit` (0x823b76) walks. 1,665 consecutive `$E` initialisers at 0x848f10..0x855fd0 plus 0x8339ca. All of them are game code that sets globals (e.g. 0x848f30: `fld [0x858624]; fdiv [0x8585c0]; fstp [0x8e3ee4]; ret`) |
| `code-immediate` | 595 | function address used as an operand in a catalogued body: `push offset fn` (RW plugin constructor/destructor/copy registration, `atexit` destructors registered by the `$E` initialisers, anim/stream callbacks), `mov [mem], offset fn` (callback structs, e.g. the libjpeg error manager at 0x5cd810), `cmp`/`sub` against a callback |
| `vtable` | 343 | slots of 164 vtables that gta-reversed does not know about (see the table kinds below) |
| `fnptr-table` | 58 | `CCheat::m_aCheatFunctions` 0x8a5b58 |
| `script-range-fns` | 24 | `CRunningScript` command-range table 0x8a6168 |
| `fnptr-table-unreferenced` | 17 | pointer tables that no catalogued body names (see below) |
| `rw-node-def` | 8 | RenderWare `RxNodeDefinition` methods |
| `rw-device` | 4 | RenderWare `RwDevice` functions |
| `struct-array` | 4 | function-pointer field of a struct array |
| `fnptr-variable` | 1 | QuickTime glue pointer variable |

By class: 2,708 `game`/`RW` (live) and 12 excluded (6 d3dx9, 3 libpng+zlib, 2 QuickTime, 1 CRT).

## Method

Inputs: the exe, `procedures.csv`, the Ghidra bodies (`catalog/raw/ghidra/functions.jsonl`), CAT-7
`rwsets.csv` / `extents.csv`, gta-reversed vtables (`vtables.py`). Another new input is
`catalog/raw/cat7-code-immediates.csv`: `callgraph.py build` now writes every immediate operand in a
catalogued body whose value lies in game code but is not a procedure.

1. **Candidate values.** Every aligned dword in `.rdata`, `.data`, `_TEXT_HA` and `_rwdseg` whose value
   lies in game code (0x401000–0x858000), plus every code immediate from the file above.
2. **Entry check** (`entry_check`), done on the bytes:
   - the value is 16-byte aligned. 15,455 of the 15,477 live game-text functions are (final catalogue);
   - the byte before it is padding (0x90/0xCC) or `ret`, or the bytes before it end an MSVC switch
     table of the preceding function (jump-table dwords pointing just below, optionally followed by a
     byte index table);
   - the linear decode reaches `ret`/`jmp` without `int3`, privileged/BCD opcodes or padding;
   - the value is not inside another Ghidra body. **Exception (absorbed entry):** the value is inside a
     body, but no branch of that body targets it, and it either starts a separate chunk of the body
     (a computed tail jump) or follows padding (the host ends in a call that Ghidra does not know
     never returns). Two cases: 0x5cd6d0, the libjpeg `output_message`, which follows
     `error_exit` 0x5cd6b0 → `exit` 0x823cae; and 0x8194d0, a QuickTime `jmp` thunk that is a chunk
     of 0x81daa0.
   - **weak**: the value passes every test except the preceding-byte test. For example, 0x481300
     `ProcessCommands900To999` follows code displaced by SecuROM. A weak value is accepted only inside
     a table that has at least as many catalogue/full entries as weak ones, and at least 2 of them.
3. **Tables.** Runs of consecutive pointer slots, where each value is a catalogue entry or a
   (weak) plausible entry. A run may contain up to 16 zero slots. A new table starts at every address
   that code names (rwsets), at every gta-reversed vtable start and end, and at the `_cinit` bounds.
   Two structure rules claim their slots first:
   - `rw-node-def`: a dword pointing to an ASCII string ending in `.csl`, followed by 7 method slots
     that are each 0 or an entry (the `RxNodeDefinition` layout: name, `RxNodeMethods`);
   - `rw-device`: the `RwDevice` at 0x8e2498. It is identified by its accessor 0x7f9c20
     `mov eax, 0x8e2498; ret`, and the bytes are asserted. Layout: gamma 1.0, `fpSystem` 0x7f5f70,
     zNear/zFar, then the render-state and Im2D/Im3D functions.
   Three or more single slots at a constant stride of 8–64 bytes, with no code pointer between them,
   form a `struct-array`.
4. **Table kinds** (the first rule that matches wins):
   - `crt-init`: inside the `_cinit` bounds. The `mov` bytes at 0x823b83/88 and 0x823bb6/bd are asserted;
   - `script-range-fns`: 0x8a6168;
   - `vtable-gr:<class>`: a gta-reversed vtable;
   - `vtable`: the base is stored into `[reg]` by a function that references it (vtable install),
     or the table holds `_purecall` (0x82263a), or it has ≥ 2 slots equal, at the same index, to a
     gta-reversed vtable (a derived class);
   - `fnptr-table`: code names the base or a slot;
   - `fnptr-table-unreferenced`: nothing above applies;
   - for single slots: `fnptr-variable` when code reads or writes the slot directly, otherwise `single`.
5. **Acceptance.** A slot value is accepted when its table has ≥ 2 pointers, or when the table is a
   `vtable` (install-proven), `fnptr-variable`, `rw-node-def` or `rw-device`. Single slots are never
   accepted. A code immediate is accepted when it passes the full entry check.

The output files are sorted and contain no timestamps:

- `catalog/data-code-addresses.csv`: tracked input with columns `address, kind, evidence`. Rows are
  only ever added. Their kind/evidence is refreshed from the current analysis, and any change rewrites
  the file and loops the pipeline.
- `catalog/data-tables.csv`: 1,615 tables, with `start, end, slots, pointers, in_catalogue_before, new,
  kind, kind_evidence, name, members` (classes of the members), `base_code_refs` and
  `n_referencing_functions`.
- `catalog/data-refs.csv`: 6,772 rows, one per data slot or immediate that holds a game-code value
  outside the catalogue (or a value this step added), each with a verdict and a reason.

Exit codes: 3 = rows added or reclassified (the loop reruns); 4 = a listed row is missing from
`procedures.csv`; 0 = closed.

## Tables found (final state)

| table kind | tables | pointer slots | new entries |
|---|---:|---:|---:|
| `vtable-gr` | 379 | 4,816 | 0 (CAT-12 had them) |
| `vtable` (not in gta-reversed) | 164 | 1,447 | 343 |
| `struct-array` | 391 | 2,904 | 4 |
| `crt-init` | 3 | 1,670 | 1,666 |
| `fnptr-table-unreferenced` | 234 | 710 | 17 |
| `fnptr-table` | 2 | 69 | 58 |
| `script-range-fns` | 1 | 27 | 24 |
| `rw-node-def` | 5 | 8 | 8 |
| `rw-device` | 1 | 7 | 4 |
| `fnptr-variable` | 5 | 5 | 1 |
| `single` (rejected unless already catalogued) | 430 | 430 | 0 |

Most `struct-array` and `fnptr-table-unreferenced` tables are already catalogued: MSVC EH unwind maps
(stride 8, funclets), the d3dx9 and CRT internal tables, and the anim callback table.

Notable tables:

- **0x8a6168 `CRunningScript::s_OriginalCommandHandlerTable`**: 27 slots, 3 were catalogued, 24 added
  (one weak: 0x481300). All 27 equal the plugin-sdk `ProcessCommands{100i}To{100i+99}` addresses.
- **0x8a5b58 `CCheat::m_aCheatFunctions`**: 92 slots, 65 non-zero, 7 were catalogued, 58 added. All 92
  slots equal gta-reversed's `Cheat.cpp` table index for index, zeros included. The table is read by
  `ApplyCheat` 0x438370 and `ResetCheats` 0x438480.
- **0x8d0954**: 42 anim-finish callbacks (`CTaskSimple*::Finish*CB`, `CPed::RestoreHeadingRateCB`, …).
  31 were catalogued (29 gta-reversed), 11 added. No catalogued body names the table, so it is
  classified as unreferenced. Its user is not identified yet.
- **0x85b5d0**: a `CEvent`-derived vtable. 11 slots equal `CEventAttractor`'s, and 6 were added.
- **0x862e70**: an abstract class vtable (`_purecall` slot). 4 added.
- **0x887d68 / 0x887d74**: `std::exception`-derived vtables. The dword before 0x887d68 is 0x88e048,
  an MSVC RTTI complete-object locator (`{0, 0, 0, TypeDescriptor 0x8cbbc0, ClassHierarchy 0x88e038}`). The copy constructor 0x4f42a0 comes from the MSVC `CatchableType` array at 0x8935e8
  (stride 0x1c, `struct-array`).
- **0x8832b0**: 10 × `{func, 4 × ush}`, stride 0xc, with 3 distinct functions. This matches zlib's
  `deflate` `configuration_table` (the zlib CRC table 0x77073096… follows at 0x883324). CAT-5
  classifies the 3 functions as libpng+zlib.
- **RW**: `RxNodeDefinition` for `nodeD3D9AtomicAllInOne.csl`, `nodeD3D9WorldSectorAllInOne.csl`,
  `nodeD3D9SkinAtomicAllInOne.csl`, `ImmInstance.csl`, `nodeD3D9SubmitNoLight.csl`; `RwDevice` 0x8e2498.
- d3dx9 vtables at 0x87ae0c, 0x87aeec, 0x87af6c, 0x8807d8, 0x8d7530 and 0x8d75e4 add 6 d3dx9 entries
  (classified excluded).

Convergence on the first run (from the CAT-14 state): pass 1 → 13 call targets, pass 2 → 3, pass 3 →
data_refs +203 (immediates in the new bodies, mostly `atexit` destructors registered by the static
initialisers; 1 row reclassified to `vtable` once 0x4f42a0 became a body), pass 4 → +4, pass 5 → closed.
This came after a bootstrap run of `data_refs.py` on the old catalogue, which added 2,513 rows. From the
present committed state, a single pass is enough.

## Rejected candidates

`catalog/data-refs.csv` gives the reason for each one.

| reason | `.data` | `.rdata` | `_TEXT_HA` | immediates |
|---|---:|---:|---:|---:|
| inside a Ghidra body (not an entry) | 1,862 | 1,813 | 220 | 8 |
| not 16-byte aligned | 242 | 395 | 46 | 10 |
| weak (no padding/ret/switch table before) and not in a supporting table | 6 | 12 | 2 | 4 |
| single slot that no code reads or writes | 9 | 5 | 1 | |
| decodes `hlt` | | 1 | | |

What the rejected values are, checked by hand:

- **Inside a body / unaligned**: mostly coincidental data such as floats, packed ints, strings
  (0x6f7270 = `"pro"`, which also lands on a loop head), and MSVC SEH scope tables and switch tables
  of the CRT and d3dx9 (e.g. 0x888338: `{-1, filter, handler}` blocks inside CRT functions; 0x881b28: a
  d3dx9 jump table).
- **Single slots, left out**:
  - 0x8e2a0c… is accepted (`fnptr-variable`).
  - 0x7adc10 (d3dx9 switch table), 0x827380 / 0x827b00 (CRT `__except` blocks), 0x430040 / 0x470040 /
    0x850080 / 0x840080 / 0x850000 (round numbers in data), 0x4d3830 / 0x473830 / 0x453830 (strings
    `"08M"`, …), 0x505250, 0x5fc3a0 and 0x484c30 (decode as junk) are not entries.
- **Ambiguous, left out (no guessing):** none of the singles was a convincing function. The struct at
  0x8a1ee8 (stride 0x20, 4 weak values 0x484c30…) decodes as junk at its first value and was
  rejected.

Counted only: `.data2` (SecuROM data, 4,678 game-code-range dwords, 123 of them catalogue entries) and
the dwords inside `.text` (16,388 game-code-range values, mostly switch jump tables, which point into
their own function) are not scanned for entries. `.data2` is protection data and game code never reads through it.

## Verification

- `run_all.sh` converged (`data_refs: 0 accepted data-referenced entries outside the catalogue (2720
  rows in data-code-addresses.csv, all in the catalogue)`, `gaps: 0`, `noncatalogue_targets` = 0).
- A second `run_all.sh` needed one pass and gave sha256-identical files for every file in `catalog/`
  (top level, the tracked set).
- Cross-checks: the cheat table equals gta-reversed `Cheat.cpp` (92/92 slots). The script table
  equals the plugin-sdk addresses (27/27). Vtable kinds were spot-checked with capstone: 0x4a4f50 is a
  ctor with an SEH prologue; 0x859cfc is a `CTaskSimple` vtable with `CTaskSimple::IsSimple` /
  `GetSubTask` slots. Immediates, by the instruction that holds them (all 595 found again in the final bodies):
  `push` 558, `mov [mem], imm` 134, `mov reg, imm` 26, `cmp [mem], imm` 2 (0x808344), `sub reg, imm` 1
  (0x74a279, `x == fn ? 0 : x`). The non-`push`/`mov` forms were checked by hand.

## Caveats

- The 1,666 static initialisers and the 343 unknown-vtable slots are `unknown` and `UNK-game`. Giving
  them subsystems and names is CAT-6 / CAT-5 work.
- A table that only code outside the catalogue references (for example from `.HOODLUM`) counts as
  `unreferenced`. The entries are still accepted when the table rules hold.
- The entry check assumes MSVC 16-byte function alignment. 22 live game-text functions are not aligned, so
  an unaligned data-only function would be missed. The 693 rejected unaligned values were not
  reviewed one by one: a sample is interior addresses or data.
