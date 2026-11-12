# CAT-16: fall-through into another catalogue entry

Problem (found by SAMP-1): `Win::NOTSA_WinMain` (0x748710) had a 206-byte extent that ended at
0x7487cf on `mov [eax+4], esi`. The next catalogue entry, 0x7487cf, is the gta-reversed hook
`Input::Initialise`, so execution simply continued there. `callgraph.csv` had no edge for that
flow, so `CGame::Initialise` (0x53bc80) and most of the game could not be reached from WinMain.

## Method

`callgraph.py build` checks every instruction of every live body (the Ghidra body plus the `.HOODLUM`
body, after `complete_body`). A fall-out is an instruction that does not end the flow (not
ret/retf/jmp/int3/hlt/ud2) and whose next address is outside the body but is a catalogue or Ghidra
entry. Each case gets one decision:

| decision | rule | effect |
|---|---|---|
| `noreturn-call` | the instruction, or the call that starts its straight-line block (walking back to a branch target or the range start), is a call to a non-returning function | no edge; the bytes after the call are dead |
| `fallthrough` | the target is referenced from outside the faller: rel32 call/jmp/jcc, any 4-byte occurrence in the image (pointer or immediate), a Ghidra CALL ref, any `callgraph.csv` edge, or a gta-reversed `plugin::Call` (`gr-called-only`) | a real function that the faller shares as its tail: `fallthrough` edge faller → target |
| `mis-split` | no reference outside the faller, and the instruction before the target is ordinary code (not a call, not nop padding) | the target is a mid-function label: merged into the faller |
| `undecided` | everything else (for example nop padding after a call that is not known to be non-returning) | `run_all.sh` stops (exit 4) until a hand row is added |

Non-returning functions (28): the MSVC runtime functions that Ghidra names (`_abort`, `__exit`, `_exit`,
`__CxxThrowException@8`, `_longjmp`, `__amsg_exit`), the `ExitProcess`/`TerminateProcess` imports,
MSVC `exit` (0x823cae, unnamed in Ghidra). `exit` is recognised by shape: it has the same three
pushes and call to `doexit` (0x823bdb) as `__exit`, but with `push 0` where `__exit` has `push 1`. To
a fixpoint, the set also takes every function whose body has no ret, only leaves the body towards a
non-returning function, and calls at least one. `RaiseException` is deliberately left out, because it
can return for continuable exceptions.

Merging: `callgraph.py fallthrough` (a new `run_all.sh` step after `data_refs.py`, in the same fixpoint
loop) appends every `mis-split` target to **`catalog/fallthrough-splits.csv`** (`address, host, rule,
evidence`, rule `cat16-auto`) and exits 3 so that the pipeline runs again. Hand decisions go in the same
file (rule `cat16-hand`). `classify_unknowns.py` classifies each listed address `excluded:mis-split`
whatever its state, gta-reversed hooks included. `subsystems.py` then sets their state to `excluded`
(subsystem `X-mis-split`). On the next pass `DumpCat7.java` removes their Ghidra functions (the existing
`REMOVE_CLASSES` rule), and the host body flows through. Generated report: **`catalog/fallthrough.csv`**,
one row per remaining fall-out with its decision and evidence. Edge kind `fallthrough` is in
`ORDER_KINDS` (port order) and in `docs/samp/samp_usage.py` `FOLLOW`.

## Result

The pipeline converges in 2 passes (3 with the names_extra loop). It merged 7 mid-function entries
(6 by the automatic rule, 1 by hand). It found 1 `noreturn-call` and left 0 `fallthrough` edges and 0
undecided cases. A `fallthrough` edge would be correct for a target that has references of its own,
but none exists in this binary. Every fall-out found was either a mid-function label or dead code
after `exit`. The edge kind stays in place for any future catalogue entry.

| target | faller (host) | evidence | decision |
|---|---|---|---|
| 0x7487cf `Input::Initialise` (gr hook) | 0x748710 `Win::NOTSA_WinMain` | `mov [eax+4], esi` at 0x7487cc runs on into `call 0x530500` (`CControllerConfigManager::MakeControllerActionsBlank`); no reference to 0x7487cf anywhere. gta-reversed's own WinMain.cpp marks it `// 0x7487CF` *inside* `NOTSA_WinMain`: `WinInput::Initialise` is inlined there | mis-split |
| 0x4c88f4 `CVehicleModelInfo::SetRenderPipelinesCB` (gr hook) | 0x4c88f0 (unnamed) | 0x4c88f0 = `mov eax,[esp+4]; ret` (`return atomic`, called from 0x6d3608/0x6d380d/0x6d390b); 0x4c88f4 is its `ret` | mis-split |
| 0x503219 (Ghidra `FUN_00503219`) | 0x5030d0 `CAEWaterCannonAudioEntity::Service` | `fld [0x8cc290]` at 0x503213 is followed by `sub esp,8; fsub ...` (the same expression continues); no reference | mis-split |
| 0x732307 `CVisibilityPlugins::SetClumpForAllAtomicsFlag` (gr hook) | 0x7322f0 (unnamed) | 0x732307 is the `ret` after `add esp,0xc` of 0x7322f0 = `RpClumpForAllAtomics(clump, 0x7322d0, id)`. gta-reversed's source comment says `// 0x7322F0`; the hook address 0x732307 is a typo | mis-split |
| 0x7f6781 `WindowedModeHooks::...HookDirect3DDeviceReplacerSA` (gr hook) | 0x7f5f70 `D3D9System` | `push ecx` at 0x7f6780, then `call [edx+0x40]` at 0x7f6781: a naked mid-function patch point (`__declspec(naked)` in WindowedMode.cpp), not a function | mis-split |
| 0x7f7c41 `WindowedModeHooks::...HookDxCamFix` (gr hook, commented out) | 0x7f7730 `_rwD3D9CameraClear` | `mov [0xc9bee8], ecx` at 0x7f7c3b falls into it; its only other reference is `je 0x7f7c41` at 0x7f7955 inside the host. A naked patch point (returns to 0x7F7C4C) | mis-split |
| 0x43a7a0 `CConversationNode::ClearRecursively` (gr hook) | (faller itself) | not an instruction boundary: the address lies inside the 7-byte `mov [edx+0x10], 0` at 0x43a79f of uncatalogued code 0x43a770–0x43a7b0. Decoded from 0x43a7a0 it is junk that falls into 0x43a7b0 `CConversations::Clear`. gta-reversed's source comment gives 0x43A710 (catalogue row 0x43a710, now named with high confidence) | mis-split (hand) |
| 0x5cd6d0 (unknown) | 0x5cd6b0 (unknown) | `push 1; call 0x823cae` (= `exit(1)`) at 0x5cd6c6; `pop esi` and nops after it are dead. 0x5cd6d0 is a real function (immediate at 0x5cd823 in 0x5cd810) | noreturn-call, no edge |

Extents after merging (`extents.csv`): WinMain 206 B in 2 ranges → 2,023 B in 1 range (0x748710–0x748ef7);
Service 334 → 381 B; D3D9System 2,492 → 2,732 B; `_rwD3D9CameraClear` 1,334 → 2,107 B; 0x4c88f0 4 → 5 B;
0x7322f0 23 → 24 B. Every merged host is now one contiguous range.

Catalogue: 15,486 → 15,479 live functions. `excluded:mis-split` 653 → 660. 6 gta-reversed hooks go from
`specified` to `excluded` (none was above `specified`).

### Verification

- **WinMain → CGame::Initialise**: reachable over the `ORDER_KINDS` edges, through
  `0x748710 -call-> 0x53e580 Game::InitialiseGame -call-> 0x53bc80 CGame::Initialise` (before: not
  reachable). Functions reachable from WinMain over all non-`fnref` edges: 474 → 3,654. WinMain's port
  level: 15 → 41 (44 levels in total, unchanged).
- **Spot checks with disassembly (10)**: the 8 cases in the table, checked by hand against
  capstone disassembly and the gta-reversed source. Two negative checks: 0x4f41b0 and 0x5ecab0 end on
  `call 0x822d5d` (`__CxxThrowException@8`) followed by `int3`/nop padding. The next address is not an
  entry and the call does not return, so neither is reported.
- An earlier test confirmed that no live catalogue entry lies inside another live function's body. A
  shared tail would always show up as a fall-out, never hidden inside a body.
- `tools/catalog/run_all.sh` twice: all 35 files under `catalog/` (except `raw/ghidra`) sha256-identical.
- `docs/samp/samp_usage.py` rerun (`FOLLOW` now has `fallthrough`). It already had its own range-end
  fall-through workaround (`fallthrough_edges`), so its closures change only slightly: B 6,037 → 6,054,
  D 6,615 → 6,629. C/E/F/G each drop by 3 or 4 because the 7 merged rows are no longer live. The
  `samp_rs_*` deltas (for example `scm-opcode` 104 → 114) come from changes in `~/samp-rs` since the
  last run, not from CAT-16. (samp-rs moved into this repository as `samp-rs/` on 2026-11-12;
  `samp_usage.py` now reads it there.)

## Follow-ups (not done here)

- `catalog/rw-names.csv` row 0x732307 `CVisibilityPlugins::SetClumpForAllAtomicsFlag` (copied from the
  gta-reversed hook) blocks the correct name on 0x7322f0 (`names-extra-rejected.csv`). Move it to
  0x7322f0 the next time that hand file is edited.
- gta-reversed hooks at wrong addresses (0x43a7a0, 0x4c88f4, 0x732307) would patch mid-instruction or
  `ret` bytes if they were enabled. Our ports hook the real entries (0x43a710, 0x4c88f0, 0x7322f0).
- `crates/sa-oracle/src/l2.rs` builds its transitive read/write sets from `call`/`tailcall` only. It
  should also follow `fallthrough` once such an edge exists.
- 0x43a770–0x43a7b0 is code with no reference (`ClearRecursively`-like body with a bool argument). It is
  not in the catalogue.
