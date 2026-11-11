# ADR-002: partial ports with a fallback to the original body (addendum to ADR-001)

Status: implemented (PORT-SCRIPT-2, 2026-11-11). Decision taken by the main session (ROADMAP PORT-SCRIPT):
the 25 remaining `CRunningScript::ProcessCommandsXToY` switches (~44,000 instructions) are ported case
by case, SA:MP opcodes first; unported cases run the original body. Code: `crates/sa-macros` (`fallback`),
`crates/sa-core/src/abi.rs` (`HookReg::set_original`), `crates/sa-core/src/orig.rs` (`PHASE_A`,
`fallback_entry`), `crates/sa-hook/src/table.rs` (`HookSpec::set_original`, `set_installed`),
`crates/sa-script/src/cmdlib.rs` (`fallback`), the `cmdNNN.rs` switch modules.

## 1. Problem

ADR-001 replaces a function as a whole: once its hook is on Rust, the original body is unreachable from
the port (an engine crate has no pointer to the trampoline sa-hook built for original mode). A script
switch holds 50-100 cases sharing one prologue, SEH frame and jump table; SA:MP runs 1-14 of them per
switch. Porting a switch therefore meant porting every case before the hook could be used at all.

## 2. Decision

`#[hook(..., fallback)]` marks a partial port. Next to the shim the macro emits

- a cell `__SA_HOOK_ORIGINAL_<FN>: AtomicU32` and `HookReg::set_original = Some(|a| cell.store(a))`
  (`None` for every other hook: ADR-001 ports are unchanged);
- `pub fn <fn>_original(args) -> R`: in phase A a typed call of the cell's address with the hook's
  convention through `__orig_body!` (i.e. `abi::call_original`, the same path as every `orig!` call); in
  phases B/C the `unported` panic of `orig!` (or a host stand-in for `<module>::<fn>_original`).

sa-hook's `set_installed` (run by `install_one` for every hook before its entry is patched) passes the
hook's original entry to `set_original`: the trampoline of an inline hook (`x86::relocate_keep_return`:
the stolen whole instructions, relocated, then `jmp` back) or the old target of a `.HOODLUM` stub. This is
the address original mode already jumps to (`table::ORIGINAL[i]`); no new code is generated.

A switch port is `match command { <ported opcodes> => …, _ => fallback(<fn>_original, NAME, s, command) }`.
`cmdlib::fallback` panics with the switch name and the opcode when `!sa_core::orig::PHASE_A`, so phase B
names the missing case, not just the function. In phase A the original body dispatches the opcode
itself, so unported cases and the default (-1) behave exactly as with the hook on the original.

## 3. Invariants

1. **Same patched bytes, same checks.** The fallback reuses the original-mode trampoline. The entry
   patch, the stolen range, the keycells redirect (`keycells.rs`), the INF-20 data-read scan
   (`every_hook_installs_on_the_exe`: relocation, `expect` covers the steal, no inbound branch into the
   patched range, no overlap) and the callers test (`callers.rs`, registers that callers keep across
   the call) apply unchanged; the 25 switch hooks pass all of them (sa-hook lib tests 45/45).
2. **Filled before reachable.** The cell is written before the entry is patched; the Rust shim is
   reachable only through that patch, so a fallback never sees 0 (`orig::fallback_entry` panics if it
   would). Hooks are never uninstalled; the cell is never cleared.
3. **Calling convention.** The fallback is an ordinary typed call with the hook's own convention and
   arguments (thiscall: `this` in ECX, the stack words copied), callee-saved registers preserved by the
   ABI on both sides, the result in AL/EAX. The original body runs one frame deeper than with the hook
   on the original (below the Rust shim, with a Rust return address); a body that read its caller's
   frame above its own arguments would see different bytes — the switches do not (they read only
   `[esp+4]`, the command). `preserve = "ecx"/"edx"` keeps applying to the hook's outer entry.
4. **FPU guard.** The shim's `FpuGuard` clears FTZ/DAZ for Rust; `call_original` loads the thread's
   game MXCSR for the original body and records what it leaves, and the guard's exit restores the game
   value (ADR-001 §5). x87 CW is never changed by Rust; the original body leaves the x87 stack balanced.
   Return types are restricted to their own ABI type (`R == <R as IntoAbi>::Abi`, a compile-time check),
   so an ST0 result is never passed through Rust.
5. **Hook toggles.** Original mode never enters Rust. Rust mode runs ported cases in Rust and the rest
   through the trampoline; calls the original body makes (`CollectParameters`, …) go through their own
   hooks, exactly as in original mode. The hook-off check of a partial switch is the whole original body.
6. **Caller attribution.** The original body's `call rand` sites keep their original return addresses;
   Rust cases that call rand use `orig::at_site` as in ADR-001.
7. **Phases B/C.** No trampoline exists; every unported path panics with its context. A partial port is
   phase-B ready only for the cases it lists.

## 4. Evidence and state rules

- A partial switch lists its Rust cases (`PORTED`); `hooks::PARTIAL` collects them and a test checks
  that every listed opcode has a match arm and every arm is listed.
- L2 double-run cases of a partial switch compare the ported cases only in substance (an unported case
  runs the original body in both runs). Per-case evidence comes from split runs (INF-17): the split
  report's verdict per switch hook covers every recorded call, and `tools/e2e/split-opcodes.py` lists
  the opcodes of the recorded calls (`sa-oracle l2 show SPOOL heads`); a ported opcode that appears in a
  PASS switch is PASS. A ported opcode never executed in the recordings stays `implemented`.
- A switch counts as fully ported (catalogue state above `implemented`, whole-function L2) only when
  every case of its jump table is in `PORTED` and the fallback arm is replaced by the original default
  (-1); the `fallback` flag is then removed.
- The fallback is the generic mechanism for any partial port (not only the script switches); each use
  needs the same per-path evidence and a `PORTED`-style list.

## 5. Alternatives considered

- Whole-switch ports only (ADR-001 as is): ~20-25 passes before any SA:MP opcode outside 0To99 runs in
  Rust. Rejected by the main session's decision.
- A trampoline pointer in sa-hook's table queried by the port at run time (`table::original_at(i)`):
  engine crates cannot depend on sa-hook (it is the phase-A injector), and the index is not known to
  the port. The cell inside the port's crate needs no dependency and no lookup.
- Re-dispatching unported opcodes through the jump table address (jumping into the middle of the
  original body): would skip its prologue/SEH frame; rejected.
