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

- Not sampled, not the game thread, or a call made while a case runs (no nesting): the frame is unwound
  and the call tail-jumps to the leave target (original by default) with every register, flag, FPU and
  MXCSR value as the caller left them. Sampling is deterministic (counts only): every `SA_L2_RATE`-th
  call, at most `SA_L2_PER_FRAME` per function per frame, from frame `SA_L2_FROM`, `SA_L2_MAX` cases per
  function (default 1000); then the hook is switched to the leave mode.
- Sampled: (1) snapshot of the regions (read/write-set globals from the spec, `SA_L2_EXTRA` globals,
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
