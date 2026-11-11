# L2 oracle (INF-8): original vs Rust at real call sites

The L2 oracle compares a hooked function's Rust implementation with the original **inside the running
original game** (determinism mode on), at its real call sites, from the real game state. It is the
authority for functions L1 cannot decide (FPATAN etc., docs/re/l1-harness.md) and the L2 level of
PLAN §5 for every function.

Code: `crates/sa-hook/src/l2.rs` (config, sampling, stats), `crates/sa-hook/src/l2win.rs` (entry stubs,
generic invoker, snapshot/restore, window diff, spool), `crates/sa-oracle-format/src/l2.rs` (signatures,
spec text, case format, comparison, undeclared-write check), `crates/sa-oracle-format/src/normalise.rs`
(the one reviewed normaliser; it lives in sa-oracle-format because it must run in-process),
`crates/sa-oracle/src/l2.rs` (host: spec from the catalogue, fixture store, reports, host replay).
Acceptance: `tools/e2e/inf8-l2.sh`.

## How one case runs

A hook in mode `l2` (`SA_L2`, `SA_HOOK_MODE=NAME=l2`, or the control command `hook NAME l2`) sends every
call to a per-hook stub (`push i; jmp l2_common`, in the hook's RWX slot at +80). `l2_common` saves all
registers, EFLAGS, the FPU environment (`fnstenv`) and MXCSR into a frame and calls the dispatcher.

- Not sampled, not the game thread (the main thread guessed at attach by determinism mode, confirmed
  at every frame boundary; so start-up calls before frame 1 such as `CTimer::Initialise` are sampled
  too), or a call made while a case runs (no nesting): the frame is unwound
  and the call tail-jumps to the leave target (original by default) with every register, flag, FPU and
  MXCSR value as the caller left them. Sampling is deterministic (counts only): every `SA_L2_RATE`-th
  call, at most `SA_L2_PER_FRAME` per function per frame, from frame `SA_L2_FROM`, `SA_L2_MAX` cases per
  function (default 1000); then the hook is switched to the leave mode.
- Sampled: (1) snapshot of the regions (read/write-set globals from the spec, `SA_L2_EXTRA` regions,
  `this` sized by the hook signature's layouts.csv type, pointed-to arguments) and of the watched windows
  (default: the game's writable sections .data/.bss 0x8A4000–0xC9E000, _TEXT_HA, _rwdseg; plus 1 KiB of
  the caller's stack above the arguments; `SA_L2_WINDOW=all` adds the no-CD sections and the used heap
  arena), plus side state outside game memory (rand counters/hash, the main thread's `_holdrand`, the
  arena bookkeeping, checkpointed at the first heap change in a run). (2) The original runs through
  `l2_invoke`: the caller's EAX..EBP, FPU environment and MXCSR, copied stack arguments; captured are
  EAX..EBP, bytes popped, the whole x87 stack drained with `fxam`/`fstp tbyte` (80 bits), CW/SW/MXCSR,
  the argument words afterwards, every changed byte in the windows, hooked callees entered (Rust-side
  hooks only: `table::count`), arena allocations. (3) Everything is restored and the Rust implementation
  runs the same way. (4) Restore again, then apply the leave run's memory, side state, registers, x87
  stack (`fld tbyte`), FPU environment, MXCSR and argument words, and return with the leave run's
  `ret N`. (5) Compare, check undeclared writes, append the case to the spool.

Comparison (sa-oracle-format `l2::compare`): the return per the signature (EAX, AL for bool, EAX:EDX, or
the x87 stack: depth + all 80 bits), callee-saved EBX/ESI/EDI/EBP, bytes popped, x87 CW, MXCSR control
bits, every region after the call, every byte written in the windows, the callee sequence. Words that
differ raw are compared through `normalise.rs` (allocation ordinal within the case, entity pool slot,
model info id, image offset). Not compared: x87 exception flags and condition codes after return, EFLAGS,
ECX/EDX for non-EDX returns, MXCSR sticky flags (none is read by game code without a prior compare).

Rust ports that compute in f64 (`sa_math::x87`) return their ST0 value as an f64 (`fld qword`, exact) and
run under the default MXCSR (saved/restored around the computation), so ST0 holds the model's value bit
for bit and the game's MXCSR is untouched.

Undeclared writes (`l2::undeclared_writes`): bytes a run changed in the windows that are not in a
declared written region (spec W/RW globals, `this`, pointer arguments). Writes within 64 KiB above an
unbounded (base/index) catalogue entry are marked as such.

## Spec, fixtures, host replay

- `sa-oracle l2 spec [--hooks A,B] [--out F]`: globals per function from `catalog/rwsets.csv`, closed
  over `call`/`tailcall` edges of `catalog/callgraph.csv`; `direct` = exact, `based`/`indexed`/`A` =
  unbounded; unresolved indirect call sites are listed (their callees' sets are missing).
- `sa-oracle l2 ingest SPOOL`: `fixtures/l2/<addr>/<content id>.l2case.zst` (zstd 19); the content id is
  two FNV-1a 64 hashes of the case without provenance (frame, ordinal, caller), so identical cases are
  stored once. Quota 20 GB.
- `sa-oracle l2 replay [--fn F] [--mutant F]`: recomputes the Rust side on the host from the stored inputs
  with the same Rust hook code and compares with the stored original outputs (return values, x87 stack,
  memory through `this`/pointers; ABI fields come from the stored in-process Rust run).
- `sa-oracle l2 report SPOOL [--undeclared]`, `sa-oracle l2 show SPOOL|FIXTURE [N]`, `sa-oracle l2
  split-report PLAIN RUST [--undeclared]` and `l2 dump-diff A B` (split-run mode, below). `--undeclared`
  (last argument) prints every undeclared write range as `undeclared FN NAME ADDR+LEN xCASES SYMBOL`.
- Control channel: `l2` (summary), `l2 flush`, `hook NAME l2`.

## Extra regions (heap memory the catalogue cannot name)

`catalog/rwsets.csv` names globals only. Memory a function reaches through a global pointer or through
`this` (pool headers and byte maps on the heap, for example) is declared with `SA_L2_EXTRA` entries
`[HOOK@]ADDR:LEN[:R|W|RW|D]` (comma list). Without `HOOK@` an entry applies to every enabled hook,
with it only to that hook (name or address). ADDR and LEN are numbers or expressions evaluated when
the case starts, before the original runs: registers at entry (`eax ecx edx ebx esi edi ebp`; `ecx` =
`this` for thiscall), stack arguments `arg0`, `arg1`, …, `[EXPR]` = the dword at EXPR, joined with `+`
and `-`. Examples: the ped pool's header `[0xB74490]:0x14`, its byte map `[[0xB74490]+4]:[[0xB74490]+8]`,
a thiscall pool method's byte map `[ecx+4]:[ecx+8]`. An extra region is snapshotted, restored between
the original and the Rust run (so a function that changes heap memory starts both runs from the same
state), compared after both runs, and counts as declared for the undeclared-write check when it has
`W`. An expression that cannot be evaluated (null pointer) gives an unreadable region (counted in the
summary's `unreadable=`). Regions are capped at 16 MiB.

`tools/e2e/l2-extras.txt` holds the extra regions of every hooked function that needs them; add lines
there when a port reads or writes heap memory.

Access `D` (L2-EXTRAS): a declared write range that is neither snapshotted nor compared as a region
(`RegionKind::Decl`, encoded as region kind 6 with the length in place of the pre bytes). It only counts
as written for the undeclared-write check. Use it for static memory inside the watched windows: the
bytes a run writes there are compared anyway (window writes), and the state before the call is part of
the split-run page fingerprint, so a whole array (`CStreaming::ms_aInfoForModel`, 0x80860 bytes) or the
whole `.data`/`.bss` of an orchestrator costs nothing per case. It gives nothing for heap memory (not
watched unless `SA_L2_WINDOW=all`): use `RW`/`W` there. Not allowed with `post:`.

Finding the writers (`tools/e2e/l2-writers.py ROOT …`, capstone over the catalogue's function bodies,
index in `target/l2writers/index.tsv`): `index` builds the index of every store with an absolute
displacement into the writable sections and every instruction that takes such an address (`push imm`,
`mov reg, imm`, `lea`); `undeclared REPORT` lists, for each coalesced undeclared range of
`sa-oracle l2 split-report … --undeclared` / `l2 report … --undeclared` (every range, not only the first 8),
the candidate instructions (exact stores first, then the nearest object base, by call depth from the
hook); `who FN ADDR+LEN` the same for one range; `writes HOOK FN,…` extras lines for every absolute
store in the static closure of FN (for a callee reached through a function pointer); `draft REPORT
HOOK` one reviewed-before-use line per range. `port-l2.sh` now passes `--undeclared` to both reports
and fails a split run with undeclared writes (`(e) split-report: 0 undeclared writes`; PLAN §5), and
it refuses a host `sa-oracle` whose spec lacks a listed hook (a stale host build declares nothing
for newer hooks: the PHYS 0xB9ACC8 case).

## Verifying a porting batch: `tools/e2e/port-l2.sh`

```text
tools/e2e/port-l2.sh <HOOKS|ports> [RECORDING] [--runs "plain l2 rust hookoff split"] [--max N]
                     [--window-all HOOKS] [--tag NAME]
```

The reusable L2 + L3 check for any set of hooks. HOOKS (resolved by `tools/e2e/hook-names.py`) is a comma
list of names or addresses, `@FILE` (one per line, e.g. `tools/e2e/lists/core-pilots.txt` = the 27 ARCH-1
pilots), `state:STATE[:DOC]` (the rows of the `docs/arch/*.md` (or only `DOC.md`) function tables in that state, e.g.
`state:implemented-needs-L2:core-port` = PORT-CORE's 191), or `ports` = every engine-crate `#[hook]` registration.
Hooks are passed to sa-hook by address. RECORDING is a corpus
recording from process start (default the INF-4 grove walk). Every run replays it in determinism mode
with `SA_HOOK_PORTS=orig` and the earlier sa-hook math ports on the original, so the reference is
all-original:

| run | what |
|---|---|
| `plain` | everything on the original: the reference trace |
| `l2` | `SA_L2` = the list, spec from `sa-oracle l2 spec`, extra regions from `l2-extras.txt`, `SA_L2_WINDOW=data` |
| `l2all` | with `--window-all HOOKS`: L2 on those hooks with `SA_L2_WINDOW=all` (heap arena watched: writes to heap memory outside the declared regions are reported; slow) |
| `rust` | the list on Rust (`ports`: `SA_HOOK_PORTS=rust`, otherwise `SA_HOOK_MODE=NAME=rust`) |
| `hookoff` | opt-in: one run per hook with every other listed hook on Rust (PLAN §5 hook-off check) |
| `split` | opt-in: `splito` + `splitr` split-run recordings and `split-report` (INF-17, below) |

Checks: per hook ≥ MAX cases (default 1000) or every call, 0 mismatches, 0 undeclared writes (a hook
without calls is `NOCALLS`: no failure, no evidence); the l2 trace equals the plain trace (the oracle
does not perturb the run); the rust trace equals the plain trace on every frame and all 17 subsystems
(L3; the `randcalls` line is printed separately); hook-off traces equal plain; `sa-oracle l2 report`
of each spool. Output in `target/e2e/lane-N/port-l2[-TAG]/`: `summary.txt`, `results.tsv` (addr, name,
calls, cases, mismatches, undeclared cases, l2, l3, hookoff per hook), traces, spools, logs and the
`plain-vs-RUN.txt` diffs. Exit 0 = no failure.

Two things to watch in the results: a hook reached only from inside another listed hook (CPedPool::New
is entered only through `CPed::operator new`'s `jmp`) runs during the outer hook's case and is never
sampled, so it shows `NOCALLS`: verify it in a second invocation without the outer hook. And
`SA_L2_PER_FRAME` (env `L2_PER_FRAME`, default 4) can leave a hook with bursts of calls (pool `New`
during loading) below both 1000 cases and every call: rerun it with a large `L2_PER_FRAME`.

New corpus recordings: `tools/e2e/record-program.sh PROGRAM [--shot-every N] [--replay]` records an
input program from `tools/e2e/programs/` (screenshots every N frames, optional replay check); pass the
resulting `rec.sarec` to `port-l2.sh`.

## Spool quota (INF-18)

sa-hook counts the bytes it spools. When the next record would pass `SA_L2_SPOOL_MAX` (bytes, default
2 GiB; `k`/`M`/`G` suffixes), it stops spooling for the rest of the run: it writes a trailer
(`[u32 0xFFFFFFFF][u32 len][text]`, `sa_oracle_format::l2::spool_trailer`) and logs
`l2: spool quota reached for HOOK at frame F: … (cases compared before it still valid)`. Comparisons go on
in-process (the summary gets `spool_quota_reached=HOOK@frameF spooled=N`); only the spool stops.
`sa-oracle l2 report` prints the trailer (`TRAILER …`), `split-report` fails on one. `port-l2.sh` passes
`L2_SPOOL_MAX` through, reports a run that reached the quota as a FAIL line (`spool quota reached for
HOOK…, cases compared before it still valid`) and checks `df` (≥ 20 GB free on /home) before every run.
Test: CTimer::UpdateVariables with a 16 MiB extra region (`L2_EXTRA='CTimer::UpdateVariables@0x400000:0x1000000:R'`,
48 MiB per case): quota reached at frame 43 after 42 records (2,113,969,760 bytes); the in-process run
went on to 1,000 cases, 0 mismatches.

## Split-run L2 (INF-17): functions that cannot run twice on the same OS state

The double-run case runs the original and the Rust implementation one after the other on the same
process; effects outside process memory (file positions, descriptors, CRT thread state, D3D/OS objects,
one-time initialisers) are not undone between them. Split-run mode runs each implementation in its own
process and compares call by call:

- `SA_L2_SPLIT=orig` (the plain run) / `rust` (the rust run), with `SA_L2` = the hooks. Every sampled call
  runs ONE implementation on the live state (no restore; unsampled and nested calls run the same
  implementation), and is recorded like an L2 case with that outcome in `orig` and `leave` = 2 / 3:
  arguments, entry registers, FPU env, MXCSR, the regions before and after (spec globals, `SA_L2_EXTRA`,
  `this`, pointer arguments), window writes with the whole aligned words around them before the call,
  the callee log, arena allocations, plus (case extension `EXT1`, plain L2 cases are encoded unchanged)
  a global sequence number, the hash of every 4 KiB page of the static windows, the entry stack pointer
  and the thread's stack range, the OS object addresses seen and the raw values of the reviewed
  volatile bytes. Cases may nest (the invoker restores its block; callee logs are saved/restored).
- `SA_L2_EXTRA` gains `post:` (address/length evaluated after the call with the exit registers, e.g.
  `post:[0xC886EC]:0x30` for an object the call creates; only post bytes compared) and OS-effect probes
  `os:file:EXPR` (CRT `FILE*` at EXPR: the FILE block, the descriptor's CRT flags from `__pioinfo`
  0xC9C300 / `_nhandle` 0xC9C2FC as read from `_get_osfhandle` 0x833D2C, and the OS file: final path,
  position from a zero `FILE_CURRENT` move, size; the handle value is never recorded), `os:cwd`,
  `os:cursor`. Probes without `post:` are taken before and after the call. OS probes compare raw.
- `sa-oracle l2 split-report PLAIN.spool RUST.spool [--timeline N] [--nesting]` pairs cases by (function,
  ordinal). Determinism mode keeps both runs identical up to the first differing output, so a pair is
  compared only when its inputs are identical: frame, caller (soft when either return address is
  outside the exe: a Rust or Wine frame between), stack arguments and convention registers, x87 CW,
  MXCSR control bits, every read region before the call, the pre-call OS probes, the window bytes both
  runs wrote. The first differing pair ends that function's comparison (`DIVERGED`, field shown); the
  earliest one over all functions is printed first because later differences may follow from it.
  Outputs: `l2::compare` on a merged case (plain record + rust outcome, each run's own window bytes
  before the call); callee-saved registers whose entry values differ compare as "preserved". The page
  fingerprint does not stop the comparison: a function whose outputs and declared inputs matched while
  some page differed is `PASS-FP` (pages listed). Verdicts: PASS, PASS-FP, FAIL, DIVERGED, COUNT (case
  counts differ), NOCASES. Undeclared write ranges are listed (spec/extras completeness).
  `--nesting` lists recorded calls made inside other recorded calls; `--timeline` lists every pair
  with differing inputs in call order.
- `port-l2.sh … --runs "plain split"` runs `splito` and `splitr` (`SPLIT_PER_FRAME`, default 1000),
  checks both traces against `plain` and writes `split-report.txt` and a `split` column in results.tsv.
  Diagnostics: `SPLIT_DUMP_SEQ=N,…` (`SA_L2_DUMP_SEQ`) dumps the static windows at those cases to
  `SPOOL.seqN.win`; `sa-oracle l2 dump-diff A.win B.win` lists the differing ranges with symbols.

What two processes legitimately disagree on, and how each is handled (all in `normalise.rs`, each with
a test, every use listed by `split-report`):

| what | rule | evidence |
|---|---|---|
| CRT `__security_cookie` 0x8E31BC, `gStreamingThreadId` 0x8E4000, MEMORYSTATUS free phys/virtual 0xC8CF74/0xC8CF7C, `rdtsc` copy 0xC92104 (8), AESmoothFadeThread handle/thread id 0xB608D0 (8), audio stream thread handle 0xB606D0, gpColCache2/gpColCache 0x9655C8 (8), the pad words (+0x0C/0x1C/0x2C/0x3C) of ProcessColModels' two static CMatrix 0x968940/0x968988 and pad1..3 of TheCamera.m_mMatInverse 0xB6FA74 (uninitialised stack residue copied by CMatrix::operator* 0x59BE30 / Invert-2 0x59BDD0 → operator= 0x59BBC0), CRT `__NLG_Destination` frame 0x8E2B8C (EBP stored for debuggers, never read) | `INPUT_EXCLUSIONS`: zeroed in the page fingerprint and, in merged pairs, excluded from the comparison; raw values recorded per case (`input exclusion used` lines) | disassembly addresses in the rule texts; window dumps of plain vs rust (`dump-diff`): these were the only differing non-pointer bytes |
| OS object addresses (IDirect3D9, surfaces, vertex declarations, DirectInput/DirectSound objects) | `Foreign` class: a word that points into committed memory outside the image, the arena and the stack (VirtualQuery map before and after the call) equals another such word; the page fingerprint masks every word whose value lies below the arena outside image/stack (value-only decision, identical in both runs) | Rust code allocates from the process heap, so later Wine allocations move (identical windows up to Win32_InitInstance, then e.g. RwEngineInstance+0x94 0x0301A530 vs 0x0301A340) |
| addresses of regions read through a pointer into OS-allocated memory (`[0x8E3FEC]:0x18`: the CdStream request queue's buffer, LocalAlloc'd in CdStreamInit) | sa-hook adds every region address to the case's `foreign` list when it lies in such memory; `split-report` then accepts two different `Foreign` region addresses as the same object (pre and post regions) and still compares their bytes; listed as `region address …` uses | STREAM-L2 split groups: `region Extra 0x2f3ab10 vs Extra 0x2fde1e8` at the first pair of every function of 5 of the 9 groups, the plain run's address changing between processes too (0x2f3ab10 / 0x2fde548); `l2 show` of sp-g04b: the 0x18-byte region after gpCdStreams |
| stack addresses below a Rust ancestor | `Stack` class: compared by offset from the call's argument block; rust-run stack addresses rebased by the entry-ESP difference | SetVideoMode (Rust) above the RW file reads: whole stack 0x30 lower |
| uninitialised original bytes | `exclusions()`: Light::LightsCreate RpLight alpha (+0x24) and pDirect's frame matrix pads | 0x5BA54E-0x5BA55E (RGB only), RpLightSetColor 0x751A90 copies 16 bytes; RwFrameRotate pads |
| Win32 handle values (HARNESS-2): `gStreamSemaphore` 0x8E4004 and the streaming thread handle 0x8E4008, `gStreamFileHandles[32]` 0x8E4010..0x8E408F, the audio stream thread's `CAEStreamThread::m_nHandle` 0xB606CC; on the heap, gCdStreams' (`*0x8E3FFC`, 0x30-byte elements) semaphore +0x14 and hFile +0x18 | `OS_HANDLES` / `HEAP_HANDLES` (the heap array is found through the pointer global, declared as `doc:stream-port@0x8E3FFC:4:R`): each word is hashed and compared by its class (0, `INVALID_HANDLE_VALUE`, "a handle" = non-zero multiple of 4 below 0x10000, else the raw value): in the page fingerprint, in region bytes before and after the call, in window writes of merged pairs; raw values recorded per case | stores of CreateSemaphoreA / CreateThread / CreateFileA results at 0x406978, 0x4069C8, 0x406794 (+ CdStreamOpen), 0x4F16EE; PORT-WEAPON-PED c-split2..4 / g-split3 / w-split2 / p-split2: plain 0xE8 / 0xD0, rust 0xE4 / 0xCC from frame 201 (a one-handle shift, intermittent: equal in g-split2, hz-e1c, hz-e2g); hz-f3g (core list, grove): gCdStreams +0x14 plain 0xD4 rust 0xD0, +0x18 0xD0 / 0xCC from frame 0 |
| known residue of a ported function (ledger item; HARNESS-2) | `RESIDUE`: excluded from the fingerprint and from every other function's merged comparison, kept for the writer's own: the skimmer's CVehicleModelInfo +0x40/+0x44/+0x48 (0xB2AC74, 10 bytes), written by LoadVehicleObject 0x5B6F30 | hz-e1c / hz-e2g window dumps (seq 14..1500): 0xB2AC78 plain `3333333fffff` rust `000000000000`, the only differing bytes of page 0xB2A000; STREAM-L2 frame 201 ordinal 60 |
| pointer arguments into the caller's frames (a Rust caller lays its frame out differently; HARNESS-2) | `split.rs stack_pointees`: when both runs pass a pointer at or above the call's argument block and the pointee is DECLARED (`ArgPtr`, or an extra region `HOOK@argN:SIZE:R`) at that pointer, the argument and the region address compare by the pointee's bytes before the call; writes into the pointee move to the plain run's address; uses listed per function. An undeclared stack pointer stays a difference | CWanted::ReportCrimeNow / RegisterCrime_Immediately below a Rust CCrime::ReportCrime: `CVector*` plain 0x36EECC rust 0x36EE48 (c-split3), 0x36EEDC / 0x36EE58 (hz-e1c); extras `CWanted::ReportCrimeNow@arg1:0xC:R`, `CWanted::RegisterCrime_Immediately@arg1:0xC:R` |

No rule normalises CRT descriptor numbers: they compared equal in every pair.

Grouping: a recorded call below another listed Rust function runs below Rust frames (other stack
depth, the FpuGuard's MXCSR, other return path). Lists for the CORE-L2b set: `tools/e2e/lists/core-split.txt`
(all 54), `core-split-l.txt` (leaves), `core-split-m.txt`, `core-split-h.txt` (outer); see `--nesting`.

## Hooked code bytes read as data (key cells)

Some no-CD protection chunks decode a global's address from a dword of game *code*: `0x401E46 adc ecx,
[0x410860]` adds the entry bytes of `CColStore::LoadCollision` to decode CTaskAllocator's pool 0xB744BC
for `CTaskAllocator::operator new` (0x69D980). With that function hooked the chunk read the hook's `jmp`
and faulted (`Unhandled page fault on read access to 173715BC at address 00401E4C`). This was the
"crash at frame 4,096" of the STREAM-L2 sR300 batch (reproduced on lane 4; sR3 and sAF show the same
signature): their l2 traces had left the plain trace at frame 202 (rand), so peds reached CTaskAllocator
around frame 4,100 (the trace is flushed every 256 frames, so the trace file ended at 4,096; winedbg
then held the process until the 1,500 s timeout).
Any run that reaches that path would crash the same way, not only L2 runs.

sa-hook (`keycells.rs`) gives every such reader its own cell holding the original dword and points
the instruction's `disp32` at it, before the hooks are installed. Five readers exist for the current
table (0x401BCE, 0x40203A: AreAnimsUsedByRequestedModels; 0x401E46: LoadCollision; 0x1564881,
0x156B570: SetupVehicleUpgradeFlags); the test `every_code_key_read_of_a_hooked_range_is_redirected`
scans the exe for absolute reads of every hook's patched bytes and fails on an unlisted one (new hooks
are covered when the test runs). `SA_KEYCELLS=0` disables it. `port-l2.sh` now stops a run as soon as
Wine reports an unhandled exception (`run NAME: crashed: …`) and keeps Wine's output as `NAME.wine.log`.
A fault after `oracle: exiting` (inside `ExitProcess`, after the trace, spool and summary are flushed;
seen in 7 of 43 lane-4 runs on 2026-11-10, always `at address 7BCF0729`, plain runs too) is reported as a
note and kept as `NAME.wine-exitfault.txt`; it does not fail the run.

### Data-read scan of every hook (INF-20)

The no-CD protection decodes pointers, rotate counts and pushed constants from dwords of game *code*
(`0x401E46 adc ecx, [0x410860]` adds the entry bytes of `CColStore::LoadCollision` to decode
CTaskAllocator's pool 0xB744BC). A hook rewrites those bytes. `tools/catalog/code_data_reads.py`
generates `catalog/code-data-reads.csv`: every instruction in every code section (game `.text`,
`_rwcseg`, the no-CD `.text`, `.init`, the executable `.data`, `.HOODLUM`) whose memory operand or
immediate address lies in the first 24 bytes of a catalogued function (3,326 candidates, 311 in
protection chunks; flags `chunk`/`sweep` mark the decodes that are in line). sa-hook's install test
(`every_hook_installs_on_the_exe`) fails for a hook whose changed bytes (stub: rel32; inline: the
relocated instructions) such an instruction sees, unless it is redirected; negative test
`data_reads::tests::a_hook_on_a_read_entry_is_rejected` (0x55C410, 0x563470, 0x410860 without redirect).

Readers of registered hooks (all redirected by `keycells.rs`) and what they decode (values computed
from the exe):

| reader | hook | decodes | effect without the redirect |
|---|---|---|---|
| 0x401BCE, 0x40203A | 0x407AD0 stub (AreAnimsUsedByRequestedModels) | rotate count = low 5 bits of the stub's `E9` | none (a stub keeps its `E9`) |
| 0x401E46 | 0x410860 stub (CColStore::LoadCollision) | CTaskAllocator pool pointer, dereferenced | page fault at 0x401E4C (measured, STREAM-L2) |
| 0x1564881 | 0x4C4570 `nop; jmp` (SetupVehicleUpgradeFlags) | rotate count for pointer 0x96A8A0 (SkipCanBeActivated) | wrong pointer (INF-10 peek: 0x4B545, unreadable) |
| 0x156B570 | 0x4C4570 | pushed constant 0 (ScriptTaskPickUpObject call argument) | non-zero argument |
| 0x40A4C3 | 0x54D690 `nop; jmp` (AttachEntityToEntity-quat, PORT-PHYS) | SEH handler 0x841408 of a prologue | wrong handler if an exception unwinds |

A hook on an entry read by anything other than an absolute memory operand (immediates, indexed
operands) cannot be redirected this way and is rejected by the install test. Not hooked yet but read:
0x55C410 (at 0x4017B4), 0x563470 (0x4013A2), 0x572360 region (0x406A9E); a port of one of these needs a
`READERS` entry (absolute reads: the install test says which).

### References into the inside of a patched entry (HARNESS-2, from `tools/verify/hookscan.py`)

`crates/sa-hook/src/patch_refs.rs`, test `nothing_references_the_inside_of_a_patched_entry`, over every
registered hook and the whole image (complementing `data_reads.rs` and the installer's 0x400-byte
forward branch scan): (1) every direct branch (`call`/`jmp`/`jcc`/`loop`/`jecxz`, rel8/rel32) in the
code sections whose target lies strictly inside a patched range, confirmed on a linear decode of a
catalogued code range (`catalog/extents.csv`; the no-CD junk after most `.HOODLUM` stubs decodes into
"branches" nothing executes, e.g. 0x441135.. after `CPhysical::SetVelocity`'s stub, so bytes outside
the catalogued ranges are listed as notes only); (2) every dword of the executable sections that is the
32-bit immediate of an instruction there and points strictly inside a patch, or its displacement and a
dword read there overlaps the changed bytes (`[entry - 3, end)` for inline hooks; the `E9` of a stub
stays). Redirected `READERS` do not count. 2026-11-11: 0 offenders over every registered hook (master);
258 branch candidates and 237 dwords outside catalogued code ranges listed as notes (the 10
inside game `.text` checked by hand: SIB / opcode byte coincidences). Non-executable sections are not
scanned (no relocation table; the 29 dwords found there were ASCII / UTF-16 text). Negative tests: a
branch into a synthetic patch is found, the entry itself is not; 0x645602 (GetAnimDoorOffset+2, read as
an immediate by the protection in the executable `.data`) is listed. hookscan's ECX/EDX check is
`callers.rs` (path-following).

### ECX/EDX live across two consecutive hooked calls (HARNESS-3)

`CTaskSimpleFight::LoadMeleeData` keeps its table row in ECX across `call 0x5BD360` (0x5BF1FD) AND
`call 0x5BD3B0` (0x5BF219), and reads it at 0x5BF226 (`mov [ecx+0xC1712C], edx`). `callers.rs` ended the
path at the second call because `callee_effect(0x5BD3B0)` met its jump table (`jmp [eax*4+0x5BD3F4]`) and
answered `Clobbers`, so only an L3 rand divergence found that 0x5BD360 needs `preserve = "ecx"` (PORT-AI
v4). Now the path steps over a direct call (1) into a hook that declares `preserve` for the register (the
declaration is the evidence that the original keeps it, and the hook keeps it at run time), and (2) the
callee analysis and the path follow bounded MSVC jump tables (`cmp r, N; ja; jmp [r*4+T]`,
`switch_targets`), which alone already proves that 0x5BD3B0 keeps ECX. Tests:
`ecx_kept_across_two_hooked_calls_is_flagged` (negative: 0x5BD360 without its `preserve` is reported at
0x5BF1FD → 0x5BF226, by the hook declaration alone and by the jump table alone; without both the path
ends, the old gap) and `no_use_hides_behind_a_hooked_callee` (upper bound: assuming EVERY hooked callee
keeps ECX/EDX adds no use beyond the declared ones). On 2026-11-11 (2936 hooks, 18752 call sites) every
registered hook passes and the upper bound finds nothing: no new offender. Not covered: a register kept
across a hooked call and then returned to the caller's caller (the path ends at `ret`).

### Stack addresses above an enclosing recorded call (HARNESS-3)

The `Stack` class compared a stack address by its offset from the case's own argument block, which
assumes one shift for the whole stack. Each Rust frame moves the locations below it by its own size
difference, so an address in the frames above an enclosing recorded call moved by THAT call's shift.
Seen as: `CReferences` (0xB9B9A8 `aRefs`) holds the address of a `CEntity*` slot of a Rust caller
(PORT-WEAPON-2 split-combat: 0xB9C4FC plain 0x0036F06C rust 0x0036EF5C, 0x110 apart like the enclosing
DoBulletImpact calls, while GenerateDamageEvent's own blocks were further apart: DIVERGED on the window
byte, PASS-FP for DoBulletImpact through the page hash).

- `normalise.rs`: `NormCtx::frames` = the argument blocks of the case's enclosing recorded calls,
  index-aligned between the two runs; `Stack { frame, off }` classifies a stack address by the enclosing
  call with the highest block at or below it (`frame` 1..), else by the case's own block (`frame` 0).
- `split.rs`: `enclosing()` finds the recorded calls each case runs inside (entry order `seq`,
  completion order = spool order); `with_frames()` pairs them by (function, ordinal); `rebase()` moves
  rust-run stack writes by the shift of the frame they lie in; `page_diff()` compares the listed stack
  words (below).
- `sa-hook l2win.rs`: stack-valued words of the static windows are masked in the page fingerprint and
  listed with the OS object words (`Case::foreign_words`); `page_diff()` counts their page as differing
  unless the two words are equal through `Stack`. Before, such a word made the page hash differ
  (PASS-FP) whatever it pointed to.
- Tests: `normalise::stack_addresses_above_an_enclosing_call_compare_by_its_frame` (the 0x36F06C /
  0x36EF5C numbers, a neighbouring slot, a missing or misplaced enclosing block, two levels) and
  `split::stack_words_above_an_enclosing_call_compare_by_its_frame` (DIVERGED without the enclosing
  case, PASS with it, PASS-FP for a listed stack word at another offset).
- Limit: the enclosing call must itself be recorded (listed, and under its `SA_L2_MAX` case cap; use
  `L2_FROM` of `port-l2.sh` to start sampling late).
- Evidence (2026-11-11, lane 7, weapon-combat, list CWeapon::Fire + DoBulletImpact + GenerateDamageEvent,
  `--max 3000`, sa-hook with the stack-word listing): master's sa-oracle reports GenerateDamageEvent
  DIVERGED 3/11 at ordinal 3 frame 1758, `region Extra 0xb9b9a8 ... at 0xb9c4fc: plain 3cf03600 rust
  2cef3600` (0x36F03C / 0x36EF2C, 0x110 apart = the DoBulletImpact pairs' shift, +0x110 in 134 pairs);
  this branch's sa-oracle on the same spools: GenerateDamageEvent PASS 11/11 (own shift +0x234),
  DoBulletImpact PASS 134/134, Fire PASS 135/135. Without the Rust Fire above it (only DoBulletImpact
  listed), DoBulletImpact's own shift is 0 and no CReferences difference appears at all (master build,
  weapon-combat / c-bug / driving: GenerateDamageEvent PASS 11 / 38 / 26).
- `FUN_00606060` (0x606060, unregistered; the same family): its task / facial out-parameters point into
  a frame above the Rust `CDecisionMakerTypes::MakeDecision-ped-ptr` (0x606E70), at the SAME address in
  both runs (the frame holding them did not move), while 0x606060's own block moved +0x64. The rust
  run's writes were moved by +0x64 (0x36FBA8 → 0x36FC0C): the plain address showed no rust write
  (mismatch) and the moved one an undeclared write. `stack_pointees` now also lists a declared pointee
  region (here `FUN_00606060@arg7:2:W` / `@arg8:2:W`, tools/e2e/l2-extras.txt) at the same caller-frame
  address in both runs when the call's block moved, so its writes stay in place. Evidence (c-bug,
  0x606060 re-registered in an experiment build only, list 0x606060 + 0x606E70, `L2_FROM=2450`,
  `--max 3000`): master's sa-oracle FAIL 172/572 mismatches + 172 undeclared (first: ordinal 120 frame
  2571, `0x36fba8: orig fc01 rust 0000`; earlier runs from frame 0: 239/572 on c-bug, 7/200 on grove);
  this branch's sa-oracle on the same spools PASS 572/572, 0 undeclared, 884 same-address pointee uses.
  Test: `split::out_parameter_at_the_same_address_is_not_moved`. The hook stays unregistered in this
  branch (re-registering it is a porting decision; the grove run was not repeated with this rule).

## Stale traces (HARNESS-2)

`winepath` right after `wineserver -k` sometimes printed nothing: `SA_DET_TRACE` was empty, sa-hook
wrote its trace to the default `sa-trace.bin`, and `port-l2.sh` compared the trace an earlier run with
the same tag had left (PORT-VEH `veh-d-split`: the `rust` and `splito` logs show `trace: None`; the
compared `rust.trace` was 4 minutes older than the run and came from the DLL before the
FuckCarCompletely fix, hence "rust diverges at frame 2614"). `lib.sh e2e_winpath` falls back to the
lane prefix's `Z:` mapping; `port-l2.sh` deletes a run's old outputs before it starts and fails the run
when sa-hook's log does not show the expected trace path; `l3.sh --probe` deletes its output trace
first. `l3.sh` takes `L3_CACHE` (trace cache directory).

## Limits

- Split-run mode cannot see state outside game memory that it does not probe (D3D device state, the
  message queue, DirectShow), compares heap objects only through declared regions, and needs both runs
  to stay in step: a port that changes later state (directly or through original UB on its stack
  residue) makes every later pair `PASS-FP` or `DIVERGED`.
- Memory outside the regions and windows (heap objects other than `this`/pointer arguments) is neither
  restored between the two runs nor compared, unless `SA_L2_WINDOW=all`. A function with such effects
  can therefore leave the Rust run starting from a different state; the case then shows a mismatch.
- Another thread writing a watched window during a run would be attributed to the function.
- The spec's depth is one level (`this` and pointer arguments); layouts.csv has sizes but no pointer
  members, so pointed-to structs of `this` are not followed.
- The callee log sees hooks whose entry runs Rust code (oracle hooks, Rust-mode ports), not hooks in
  original mode.
- The arguments are copied, so the original runs with ESP lower than at the real site; functions whose
  behaviour depends on stack addresses (stack pointers stored into globals) would show as mismatches.
