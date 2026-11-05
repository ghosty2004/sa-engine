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
- `sa-oracle l2 report SPOOL`, `sa-oracle l2 show SPOOL|FIXTURE [N]`.
- Control channel: `l2` (summary), `l2 flush`, `hook NAME l2`.

## Extra regions (heap memory the catalogue cannot name)

`catalog/rwsets.csv` names globals only. Memory a function reaches through a global pointer or through
`this` (pool headers and byte maps on the heap, for example) is declared with `SA_L2_EXTRA` entries
`[HOOK@]ADDR:LEN[:R|W|RW]` (comma list). Without `HOOK@` an entry applies to every enabled hook,
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

## Verifying a porting batch: `tools/e2e/port-l2.sh`

```text
tools/e2e/port-l2.sh <HOOKS|ports> [RECORDING] [--runs "plain l2 rust hookoff"] [--max N]
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

## Limits

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
