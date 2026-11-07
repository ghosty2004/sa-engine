# Verifier report: PORT-CORE and PORT-STREAM L1 claims (2026-11-07)

Adversarial review of the 126 functions in `catalog/state.csv` with state L1 and evidence PORT-CORE (56)
or PORT-STREAM (70). The method follows `docs/re/verify-l1.md`. I read the specs, the ports, the
disassembly and every direct call site. I re-ran the oracle and instrumented the original under Unicorn.
The implementers' notes were not used.

Probes: `crates/sa-oracle/tests/verifier_core_stream.rs` (new file; nothing else changed). Run:

```
SA_WINEPREFIX=/nonexistent systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 \
  cargo test --release -p sa-oracle --test verifier_core_stream -- --test-threads=1 --nocapture
```

`SA_WINEPREFIX=/nonexistent` turns the native backend off, so nothing touches the Wine prefix (INF-16).
Knobs: `VERIFY_AUDIT_CASES` (default 100k), `VERIFY_MUTANT_CASES` (default 100k, 20k for the store
mutants), `VERIFY_HOOKED_CASES` (default 10^6), `VERIFY_ONLY=key,key`. Call-site scans were done with
capstone (`.venv/bin/python -I tools/verify/{sadis,callsites,regs_after,flags_after}.py`) on `re/gta/1.0-US/gta_sa.exe`.

Severity: **B** = the L1 claim is wrong by the PLAN §5 definition, **W** = incomplete comparison or
generator gap that weakens the claim, **I** = informational.

## Findings

### V-9 (B) CBaseModelInfo::SetBaseModelInfoFlags (0x5B3AD0): a caller reads EDX after the call, and the spec does not compare it

The caller `CFileLoader::LoadAnimatedClumpObject` uses a whole-program-optimised convention at 0x5B4177:

```
5b4177: 8b d0        mov  edx, eax        ; edx = IDE flags
5b4179: e8 52 f9 ff  call 0x5b3ad0        ; SetBaseModelInfoFlags(mi, flags)
5b417e: 83 c4 08     add  esp, 8
5b4181: f6 c2 20     test dl, 0x20        ; EDX is expected to survive the call
5b4184: 74 06        je   ...             ; sets / clears +0x13 bit 2
```

The original never writes EDX: `verifier_edx_live_across_set_base_model_info_flags` found EDX changed
on 0 of 100,000 spec cases. The L1 comparison (`runner::compare`) checks only EAX/AL, ST0, scratch
and ESP, so EDX is never compared. The hook shim is a plain `extern "C"` function. In the
i686-pc-windows-gnu build (`cargo rustc -p sa-stream --release --target i686-pc-windows-gnu --lib --
--emit asm`, symbol `___sa_hook_shim_set_base_model_info_flags`), it returns with
`EDX = (saved MXCSR & ~0x8040)` in the upper half, DH = the new flags byte and DL = `old byte & 0xA1`.
The caller's `test dl, 0x20` therefore tests bit 5 of the model's old flags byte, not IDE flag 0x20.

The result goes wrong whenever 0x5B40C0 runs the original and 0x5B3AD0 runs Rust. That is the
configuration of the L3 hook-off check for 0x5B40C0, and of any partial hook set. Both functions as
Rust are fine: the Rust `load_animated_clump_object` tests `flags` itself.

- Reproduce: `cargo test ... verifier_edx_live_across_set_base_model_info_flags` (asserts the
  call-site bytes and that EDX is preserved).
- Call-site scan of all 126 functions: `tools/verify/regs_after.py`. For each `call` to a claimed
  function, it finds the first read of ECX/EDX before a write. Only this site is a real use. The other
  hits are `push ecx` stack reservations (0x43D176, 0x58B3CA) or partial-register false positives.
- Fix: make the preserved-register set part of the spec (compare EDX for 0x5B3AD0) and give this hook
  an EDX-preserving shim. Alternatively, never enable it without its caller, and run the same register
  liveness scan over every hooked function before L1 (or L3) is recorded.

### V-10 (W) 44 specs check a helper or a model in the spec, not the hooked entry

PLAN §5 requires the Rust port to be compared. Several spec closures never call the function that
`#[hook]` installs:

| claims | what the spec runs | what is not tested |
|---|---|---|
| CPad::IsEnterJustPressed, IsMenuKeyJustPressed, IsTabJustPressed, IsStandardKeyJustPressed (4) | an expression written in the spec (`key_spec`, `nz16(new) && !nz16(old)`); `sa_core::input` is not called | the whole port (offsets 0x23C/0x25A/0x266/0x268/0x256, `0x18 + 2*key`) |
| Light::GetDayNightBalance, __init_g_bab358 (2) | the pure helpers `day_night_balance`, `g_bab358_value` | the entries (global addresses, byte order of the clock fields) |
| 10 `CPool::New` (CBuilding, CColModel, CPointRoute, CPatrolRoute, CNodeRoute, CEntryInfoNode, CPtrNodeSingle/Double, CPedIntelligence, CTxd 0x731B80), CEntryInfoNode/CDummy `operator new` (2) | `pool::new_slot`; the returned address is `objects + slot * stride` with the stride **written in the spec** | `pool_new` (`size_of::<T>()`, header offsets, the `ms_p*Pool` global) |
| CNodeRoute / CEntryInfoNode `operator delete` (2) | `delete_at`; the spec computes the index | `pool_delete_exact` (`index_from`, slot size) |
| 22 pool constructors (0x405900, 0x410F80, 0x550180–0x550E40, 0x552AF0, 0x5B8340, 0x731AD0) | `init_map` only; the spec writes the header, `0xFFFFFFFF`, the owns byte and the allocator arguments itself | `pool_construct` (allocation sizes and order, header writes) |
| CAtomicModelInfo::GetAtomicFromDistance (1) | `in_draw_range`; the spec returns word 7 (+0x1C) | which field the port returns |
| Win::GTATranslateKey (1) | `translate_key_case` / `translate_key_default` | `gta_translate_key` (`OS_VER`, the `MapVirtualKeyA` call) |

I reviewed the untested glue against the bytes and found no error: slot types `Slot<N>` equal the
spec strides, and the CPad offsets match the patched operand addresses. For six of these functions I
also added real evidence. `verifier_hooked_entries_through_host_globals` reuses the specs but runs the
hooked entries, with sa-core's and sa-stream's host globals filled from the scratch regions. At
10^6 generated cases plus edges:

| function | mismatches |
|---|---|
| CPadIsEnterJustPressed | 0 |
| CPadIsMenuKeyJustPressed | 0 |
| CPadIsTabJustPressed | 0 |
| CPadIsStandardKeyJustPressed | 0 |
| GetDayNightBalance | 0 |
| InitGBab358 | 0 (10^4 cases: constant input) |

Fix: call the hooked entry from every spec, through host globals as in the probe. For the pools, which
have a different host layout, use a 32-bit layout test or an L2 check. Otherwise record the untested
glue in the evidence column.

### V-11 (W) GTATranslateKey: the generator almost never reaches the documented gta-reversed deviation (`chr == 0xFF`)

The default case stores the `MapVirtualKeyA` char only if `< 0xFF` (`cmp eax, 0xff; jge` at 0x747B4F).
gta-reversed uses `<= 0xFF`, and the port documents this. The spec's raw generator produces a low half
of exactly 0xFF about twice per 10^6 cases:

- `VERIFY_ONLY=GTATranslateKey cargo test ... verifier_mutants_core_stream`: the gta-reversed mutant
  is **missed at 10^5 cases** (0 mismatches) and caught by only 2 cases at 10^6.
- `verifier_gta_translate_key_boundary` forces chr = 0xFF (upper half random). The real port gives
  0 mismatches; the gta-reversed mutant gives 74,577 of 100,000.

The claim stands, but the boundary it rests on is effectively unsampled. Fix: bias the
`MapVirtualKeyA` word towards 0xFE/0xFF/0x100 and to upper-half-only values.

### V-12 (I) FPU state: the float ports assume PC=24; under CW 0x027F seven of them differ

`sa-oracle l1 <key> --cases 200000 --backend unicorn --cw 0x27f` (native backend off) gives these
mismatches:

| key | mismatches |
|---|---|
| GetDayNightBalance | 294 |
| CColStoreSetIfCollisionIsRequiredReducedBB | 11 |
| DoesInfiniteLineCrossFiniteLine | 8 |
| DoesInfiniteLineTouchScreen | 7 |
| CQuadTreeNodeFindSectorVec | 7 |
| IsPointInsideLine | 2 |
| CAtomicModelInfoGetAtomicFromDistance | 1 |

FindSectorRect, InSector, SetIfCollisionIsRequired and GetWheelSize have 0. The only claimed function
known to run before the D3D device sets PC=24 is the CRT initialiser `__init_g_bab358`. It passes under
CW 0x7F, 0x27F and 0x37F, because its input is a constant. The callers of the float functions are
gameplay, rendering and streaming code. I did not prove for each one that it runs after device
creation (in particular CQuadTreeNode and the CColStore load path). As in V-5, the spec should record
the FPU state per call site.

### V-13 (I) Instrumented audit: no hidden writes, every real branch covered, catalogue extents include uncatalogued neighbours

`verifier_audit_claimed_specs` (10^5 generated cases plus edges per spec) runs the patched original
under a bare Unicorn with block, write and read hooks. Over all 127 specs (126 claims; TouchScreen has
two):

- **No write** beyond the compared 0x500 scratch bytes. No write to the argument area or the caller's
  frame. No other write. No read of uncompared scratch (0x500–0x10000). No read of uninitialised stack.
- Every instruction and both directions of every conditional branch in each function's real body
  are executed, by edges plus generator and by the generator alone.
- The reported "uncovered" code is all outside the function. Some of it is uncatalogued neighbouring
  functions inside the catalogue extent [start, next start): 0x410420 (a second SetIfCollisionIsRequired
  variant after 0x4103D0), 0x532920–0x53298E (after IncreaseAlpha), 0x718740 and 0x718770 (after
  MakeUpperCase-in), 0x5527C0 and 0x5527E0 (after InSector), 0x5B3450 and 0x5B3470 onwards (after
  SetOtherTimeModel), 0x404010, 0x40FBF0 and similar (the `Delete`/`GetIndex` after each `CPool::New`),
  and 0x4C4310 (after SetHasBeenPreRendered). The rest is data: the jump table at 0x747B68 inside
  GTATranslateKey, and the protection junk after the 0x41B870 and 0x410F80 stubs.
- Consequence: the Unicorn allow-list accepts a jump into those neighbours as "inside the function".
  None was taken in the run, but this is a guard-rail hole worth closing with catalogue entries.
- Register picture: functions returning in AL leave junk in EAX[31:8] (GetCdPosnAndSize, InSector,
  IsEnterJustPressed, IsPointInsideLine, TouchScreen, psPathGetSeparator, GTATranslateKey). Every
  direct caller uses only AL (`test al, al` or `mov bl, al`). No caller of any claimed function
  consumes flags after the call (`tools/verify/flags_after.py`).

### V-14 (I) Smaller observations

- `Rs::RsKeyFromScanCode` returns a zero-extended EAX in the original (`movzx`, then `mov al`). The
  port returns `u8` (AL only, upper EAX undefined in the i686 ABI), and the spec compares AL only. No
  direct call site or data reference was found in `.text`, so nothing is observably affected today.
  The return type is an `RsKeyCodes` int, so a future caller could read EAX.
- `Common::IsPointInsideLine` also has no direct call site.
- The four float specs I re-ran report INCOMPLETE only because the native backend was disabled for
  this audit. Their signalling-NaN cases then count as unverified (106,516; 36,320; 102,592; 80,700).
- `CEntryInfoNodePool::GetAt` (0x536D10) is a `CPool::New` (the port and spec note this). The
  catalogue name is wrong.

## Re-runs at 10^6 cases (Unicorn, seed 0x5A0001, game FPU state, native backend off)

`SA_WINEPREFIX=/nonexistent ./target/release/sa-oracle l1 <key> --cases 1000000 --backend unicorn --no-fixtures --threads 8`

| key | cases | mismatches | verdict |
|---|---|---|---|
| CBuildingPoolNew | 1,000,148 | 0 | PASS |
| CIplDefPoolConstructor | 1,000,015 | 0 | PASS |
| CStreamingInfoAddToList | 1,002,000 | 0 | PASS |
| CStreamingInfoRemoveFromList | 1,002,000 | 0 | PASS |
| CQuadTreeNodeFindSectorRect | 1,060,000 | 0 | INCOMPLETE (106,516 SNaN unverified, native off) |
| CKeyGenGetKeySized | 1,000,075 | 0 | PASS |
| GTATranslateKey | 1,002,000 | 0 | PASS |
| GetDayNightBalance | 1,002,000 | 0 | PASS |
| DoesInfiniteLineTouchScreen1366 | 1,100,000 | 0 | INCOMPLETE (36,320 SNaN unverified) |
| IsPointInsideLine | 1,100,000 | 0 | INCOMPLETE (102,592 SNaN unverified) |
| CPedModelInfoIncrementVoice | 1,002,000 | 0 | PASS |
| LinkedFindOtherUpgrade | 1,002,000 | 0 | PASS |
| CBaseModelInfoSetBaseModelInfoFlags | 1,002,000 | 0 | PASS (but see V-9) |
| CAtomicModelInfoSetupVehicleUpgradeFlags | 1,002,000 | 0 | PASS |
| CColStoreSetIfCollisionIsRequired | 1,060,000 | 0 | INCOMPLETE (80,700 SNaN unverified) |
| CReferencesListSize | 1,000,002 | 0 | PASS |
| CPadIsStandardKeyJustPressed | 1,002,000 | 0 | PASS |

## Mutants

All faults are injected by replacing a spec's port in the test file; the ports themselves are untouched.

- Generic store mutants (`verifier_store_mutants_all_claimed`, 2·10^4 cases each). Two faults per
  spec: the last byte the port writes is left out, or one extra byte is written after it. Every spec
  whose port writes memory catches both (80 specs, 160 mutants). The 47 read-only specs make both
  faults equivalent (94 mutants); the test checks that this is the only reason for a miss.
- Semantic mutants (`verifier_mutants_core_stream`, 10^5 cases each):

| spec | mutant | result |
|---|---|---|
| CPedModelInfoIncrementVoice | wrap when `cur >= max` | caught (209) |
| LinkedFindOtherUpgrade | search upwards | caught (34,108) |
| VehGetNumRemaps | no cap at 4 | caught (54,184) |
| CReferencesListSize | single node counted twice | caught (554) |
| RsKeyFromScanCode | index not masked to a byte | caught (25,565) |
| CStreamingInfoGetCdPosnAndSize | true for size 0 | caught (53,745) |
| DoesInfiniteLineTouchScreen1366 | width and height swapped | caught (6,990) |
| GetDayNightBalance | `/ 60` (gta-reversed) | caught (95) |
| GTATranslateKey | `chr <= 0xFF` (gta-reversed) | **missed** at 10^5 (V-11), caught by the boundary probe |

## Verdicts

- **0x5B3AD0 CBaseModelInfo::SetBaseModelInfoFlags: the claim does not hold** (V-9). A caller observes
  EDX, the spec does not compare it, and the hooked shim does not preserve it.
- 44 claims (V-10): the hooked entry is untested by L1, so the claims rest on code review. For
  CPad ×4, GetDayNightBalance and __init_g_bab358, this report now supplies hooked-entry evidence
  (0 mismatches at 10^6). The 36 pool claims, GetAtomicFromDistance and GTATranslateKey remain as described.
- GTATranslateKey: the claim stands, but its documented boundary is undersampled (V-11).
- The remaining claims stand. No write is hidden from the comparison, every real branch is
  covered, and the semantic mutants are caught.

## VER-2 follow-up (2026-11-07)

Harness changes that close V-9..V-12 and V-14. Commands as above (`SA_WINEPREFIX=/nonexistent`, Unicorn).

### V-9: preserved registers

- `#[hook(preserve = "edx")]` (`"ecx"`, `"ecx,edx"`) in sa-macros: the registered entry becomes a
  naked wrapper that saves the registers, copies the stack arguments, calls the shim and restores
  them (`HookReg::preserve`). Emitted i686 code for 0x5B3AD0 (`cargo rustc -p sa-stream --release
  --target i686-pc-windows-gnu --lib -- --emit asm`):
  ```
  __sa_hook_preserve_set_base_model_info_flags:
      pushl %edx
      pushl 12(%esp)
      pushl 12(%esp)
      calll __sa_hook_shim_set_base_model_info_flags
      addl  $8, %esp
      popl  %edx
      retl
  ```
- L1: `Spec::preserve`. The runner loads each preserved register that is not an argument with a
  per-case value, requires the original to return it unchanged and the port's hook to declare it
  (kind `preserve-ecx`/`preserve-edx`). ECX is now captured by both backends. The 0x5B3AD0 spec
  compares EDX; `verifier_edx_live_across_set_base_model_info_flags` shows both checks discriminate
  (a hook without `preserve`: every case fails; an original that writes EDX: caught).
- Caller scan: sa-hook test `callers::callers_reading_ecx_edx_after_a_hook_are_declared` (all 796
  hooks, 569 with direct callers, 5,575 call sites in `.text` and `.HOODLUM`, every path after the
  call, byte-wise register tracking). It found more than the audit's straight-line scan:
  - 0x5B3AD0 is also relied on by its callers 0x5B3B2B (`SetAtomicModelInfoFlags`) and 0x5B3C3B
    (`SetClumpModelInfoFlags`): they keep a value in DX across the call. Covered by the same
    `preserve = "edx"`.
  - **New:** `DoesInfiniteLineTouchScreen` (0x71DB80) keeps the screen width/height in ECX/EDX
    across its first call to `DoesInfiniteLineCrossFiniteLine` (0x71DB30, site 0x71DC62) and pushes
    them again for the second call (0x71DC7C..7E). The original 0x71DB30 never writes ECX/EDX. The
    hook now has `preserve = "ecx,edx"` (shim-only change in `sa-core/src/common.rs`), and its spec
    compares both registers.

### V-10: specs run the hooked entries

All 44 specs (and the pilot `CPed/CVehicle/CObjectPool::New`, `CPed::operator new/delete` specs
built by the same builders) now call the function `#[hook]` installs, on the host
(`sa_oracle::hosted`, documented in `docs/re/l1-harness.md`): host globals for the key states, the
clock, the OS version, the LOD multiplier and the pool pointers; `sa_core::orig::host` stand-ins for
the allocators (the port must call the allocator the original calls), `MapVirtualKeyA` (its arguments
are now recorded by the original's stub and by the stand-in, and compared) and the original
`CDummyPool::New`; the 32-bit `CPool` header bridged to the host layout. Slot sizes come from the
entries' types, asserted against the strides in the original's code. Remaining gap: the two
`CPool<TxdDef>` specs (`TxdDef` holds a pointer, host size 16) run the same generic code on a
0x0C-byte slot type instead of the entries. `verifier_hosted_pool_entries_discriminate`: a wrong slot
size, the wrong allocator and the wrong pool global are caught.

### V-11: GTATranslateKey boundary

The generator now sets the low half of the `MapVirtualKeyA` result to 0xFF (5/16), 0xFE, 0x100 (2/16
each), 0xFFFF or 0 (1/16 each), with the upper half random or zero. The gta-reversed mutant
(`chr <= 0xFF`) is caught at 10^5 cases: 60,333 mismatches (was 0).

### V-12: FPU state of the callers of the seven float functions

Call graph from `catalog/callgraph.csv` (`tools/verify/fpu_callers.py`) and the exe:

- PC=24 is set by `_rwProcessorInitialize` (0x857404) from `D3D9System` case 17
  (rwDEVICESYSTEMFINALIZESTART, 0x7F68E8), i.e. by `RwEngineStart` inside `RsRwInitialize`, reached
  from WinMain's `RsEventHandler(0x15 = rsRWINITIALISE)` at 0x748847. It is forced again per camera
  update (`_rwProcessorForceSinglePrecision` 0x857432 from `_rwD3D9CameraBeginUpdate`). The saved CW is
  restored only by `_rwProcessorRelease` (0x85742B) in rwDEVICESYSTEMSTOP (`RwEngineStop`: shutdown
  after `CGame::Shutdown`, and the video-mode change, which restarts the engine right after). Other
  CW writes on the main thread are save/restore pairs inside one function (CRT `_ftol` and math
  `__ctrlfp`, RW image functions) or D3DX9 objects (0x76B89B/0x76B940) that set `_PC_53` with
  `_controlfp(0x10000, 0x30000)` for their lifetime inside RW's vertex-shader setup; no game code runs
  inside those scopes.
- Per function:

| function | direct callers | entry points from WinMain / roots | runs at |
|---|---|---|---|
| GetDayNightBalance 0x6FAB30 | `CEntity::ProcessLightsForEntity` (0x6FC7AC) | only the 11 `*::PreRender` virtuals (render loop) | PC=24 |
| SetIfCollisionIsRequiredReducedBB 0x410470 | none direct; callback pushed from `.HOODLUM` 0x156CFB4 (`SetCollisionRequired`, `LoadCollision`) | WinMain only via `Game::InitialiseGame` (0x748CFB), `CGame::ShutDownForRestart`/`InitialiseWhenRestarting` (0x748E04–0x748E48), `CGame::Shutdown` (0x748E6B), all after 0x748847; other roots: game loop, script commands, `ProcessControl` virtuals | PC=24 |
| DoesInfiniteLineCrossFiniteLine 0x71DB30 | `DoesInfiniteLineTouchScreen` (0x71DC62, 0x71DC81) | `COccluder::ProcessLineSegment` → `Game::Idle` | PC=24 |
| DoesInfiniteLineTouchScreen 0x71DB80 | `COccluder::ProcessLineSegment` (0x71E520) | `Game::Idle` | PC=24 |
| CQuadTreeNode::FindSector-vec 0x552640 | `GetAllMatching-vec` (0x55295B), `ForAllMatching-vec` (0x552A1A) | as 0x410470, plus `CIplStore::RequestIpls` | PC=24 |
| IsPointInsideLine 0x71E050 | none (no call, no data reference) | never runs in 1.0 US | — |
| GetAtomicFromDistance 0x4C44B0 | none (no call, no data reference) | never runs in 1.0 US | — |

`Win32_InitApplication` (which registers `MainWndProc`, the only code that runs before 0x748847
besides the CRT initialisers and WinMain's start) reaches none of them. No spec needs another CW; V-12
is closed without spec changes. Limitation: indirect calls the catalogue has not resolved are not in
the graph.

### V-14

- `Rs::RsKeyFromScanCode` returns `u32` (the table byte zero-extended, as `movzx eax, …; mov al, …`
  leaves EAX); the spec compares the whole EAX. Mutant "upper EAX bits left set" caught (102,000/102,000).
- Catalogue name of 0x536D10: `CEntryInfoNodePool::GetAt` (from gta-reversed's `RH_ScopedInstall`,
  `catalog/functions.csv` row 2966 / `procedures.csv`). The bytes are `CPool<CEntryInfoNode>::New`
  (slot 0x14, target of `CEntryInfoNode::operator new`); the name should be `CEntryInfoNodePool::New`.
  Not edited (catalogue is out of scope for this task).

### Re-runs at 10^6 cases of every changed spec (Unicorn, seed 0x5A0001, native off)

All 53: 0 mismatches. PASS, except `DoesInfiniteLineCrossFiniteLine` (1,100,000 cases, 116,280 SNaN
cases unverified) and `CAtomicModelInfoGetAtomicFromDistance` (1,060,000, 41,136 unverified):
INCOMPLETE only because the native backend was off. Specs: SetBaseModelInfoFlags (EDX compared),
DoesInfiniteLineCrossFiniteLine (ECX/EDX compared), RsKeyFromScanCode, GTATranslateKey, CPad ×4,
GetDayNightBalance, InitGBab358, GetAtomicFromDistance, 13 `CPool::New` (incl. the three pilots and
CTxd), 4 `operator new` (CNodeRoute, CEntryInfoNode, CPed, CDummy), 3 `operator delete`, 22 pool
constructors.
