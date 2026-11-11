# AI-GEN: generated AI ports (crate `sa-ai`, files `gen_*.rs`)

Second AI pass after PORT-AI-1 (`docs/arch/ai-port.md`): every repetitive shape across the AI classes
(`CTask*`, `CEvent*`, `CEventGroup`, `CPedGroup*`, `CDecisionMaker*`, attractors, task allocators, plus
the unnamed and `vt_*` functions of subsystem AI) is generated from the 1.0 US bytes by
`tools/gen/ai_patterns.py`. The generated Rust holds only semantics (offsets, constants, vtable
addresses, callee addresses); the exe is read at generation and test time and never copied. Per-function
verdicts are in `docs/arch/ai-gen-verdicts.tsv`.

## Generator

`.venv/bin/python -I tools/gen/ai_patterns.py` writes `crates/sa-ai/src/gen_{leaves,virtuals,ctors,clones,
dtors,subtasks}.rs` and `target/ai-gen/{report,rest}.tsv` (`rest.tsv`: every candidate not generated, with
the reason). `--check` regenerates in memory and exits 1 if a file differs (two runs are byte-identical:
candidates are processed in address order, no set or dict iteration reaches the output unsorted).
Input: the exe, `catalog/{functions,procedures,extents,code-data-reads}.csv` and the set of addresses already
ported anywhere in `crates/` (every `#[hook(addr = ...)]`, and every ``/// `Name` (0xADDR)`` doc line of
sa-ai outside `gen_*.rs`, so PORT-AI-1's `trivial.rs`/`dtors.rs`/hand ports are never duplicated). The
PORT-AI-1 script (`trivial.rs`/`dtors.rs`) is superseded: its eleven shapes are all special cases of the
interpreter below; those two files stay frozen as committed.

**Matching is an exact symbolic execution of the bytes**, not a template guess. Each instruction of every
path is decoded (capstone, detail mode) and executed by an interpreter that knows only this subset, with
its exact x86 semantics: `mov`/`movzx`/`movsx`/`lea`, `xor`/`and`/`or`/`add`/`sub`/`inc`/`dec`/`neg`/`not`,
`shl`/`shr`/`sar` by an immediate, `imul` (1/2/3-operand), `cdq`, byte/word register and memory forms
(AL/CL/DL/BL, AX..; partial-register writes keep the upper bits), `test`/`cmp` + `jcc`/`setcc` (signed,
unsigned and sign conditions; only the flags of the last `cmp`/`test` or of the arithmetic result for
`je/jne/js/jns`), `push`/`pop`, `sub/add esp, imm`, direct calls, virtual calls `call [vtbl + N]` where the
vtable was loaded from `[ECX]` of the object in ECX, direct tail jumps, forward branches (each path is
executed separately, emitted as `if`/`else`), `jmp [reg*4 + table]` with a bound proven by the path
condition (the table is read from the exe and included in the body hash), `ret`/`ret N`, and
`fld dword ptr [m]; ret` (ST0 result). Everything else rejects the function: any other instruction or
prefix (lock/rep/address size), `ah`..`bh`, a loop (a revisited address), a jump outside the catalogue
extent, a stack address escaping (`lea r, [esp+N]`), a partial stack write, an unbalanced stack at
`ret`, a callee-saved register or `fs:[0]` not restored, an unknown register or flag value reaching a
condition, a store, a call argument or the result.

Calls: the callee's stack cleanup comes from its own `ret N` instructions over its catalogue extent (all
must agree; a callee ending in a tail jump inherits its target's). A callee that cleans nothing gets its
argument count from the caller's `add esp, N` / `pop ecx` (at most three register moves after the call),
and the outstanding pushes since the previous call (saves of callee-saved registers, the SEH frame and
`push ecx` slot allocation in the prologue excluded) must be exactly its arguments, at the top of the
stack. The callee's first block must not read EAX/EDX/EBX/ESI/EDI/EBP before writing them (no
register-passed arguments of whole-program-optimised call sites). ECX is passed (thiscall) whenever it
holds a known value at the call, and the function is rejected when it does not but the callee reads ECX
anywhere (first ECX access in a linear sweep of its extent is a read). The first build passed ECX only
when the callee's *first block* read it: `CEventVehicleDamage::Constructor` (0x4B18D0),
`CPedAttractor::Constructor` (0x5EDFB0), 0x6930F0 and others read ECX after a branch and got a garbage
`this` (grove crashed at frames 202-2251); found by a per-file bisection, fixed in the second build. A caller-cleaned callee's argument slots are unknown after the call. Every callee
is declared in the file's `orig!` block (`c_<addr>_<conv><nargs>`, `vslot_<off>_<nargs>`).

Emission: values are `u32` (game addresses are 32-bit), pointers via `p(a) = GPtr::from_addr(a)`; every
memory read and call is a `let` at its program point (reads that only feed a vtable call are kept, so the
read sequence is the original's); deep pure expressions are bound to `let`s; returns are `u32` (EAX),
`u8` (only AL defined: `mov al, 0/1`, `setcc`; the upper 24 bits stay the caller's EAX, as in PORT-AI-1),
`()` (EAX untouched) or `X87` (ST0). The SEH frame (`push -1; push handler; fs:[0] = esp ... fs:[0] =
saved`) is executed and checked to be restored; its stack writes have no Rust counterpart (as in every
hand port). `BOOL_METHODS` (`MakeAbortable`, `ProcessPed`, `SetPedPosition`, `IsValid`, `AffectsPed`, ...) return `u8`
even when a path returns a callee's full EAX: their callers test AL only, and the upper bytes a callee
leaves are not part of the result (the second build compared full EAX and failed on
`CTaskComplexKillPedOnFoot::MakeAbortable` and `...ObserveTrafficLightsAndAchieveHeading::MakeAbortable`,
whose sub-task's `MakeAbortable` left different upper bytes). Complete destructors whose every path returns a
callee's EAX return `()` (EAX after `vtbl = X; jmp base` is whatever the base leaves, usually the caller's
EAX passed through; no caller reads it; eight split FAILs in build two were this register only).
Hook convention: `thiscall` when ECX is read or the function cleans its arguments (an
unused `_this` otherwise), `cdecl` for caller-cleaned functions and static initialisers that take nothing.

Families (one label per function, by catalogue name first, then by shape): `const-id` (constant
`GetTaskType`/`GetEventType`/`GetEventPriority`/`GetLifeTime`/`GetType`/`GetWanderType`), `const`,
`const-bool` (`IsInterruptable`/`IsValid`/`AffectsPedGroup`/... returning a constant AL), `empty`,
`getter`, `getter-f32`, `setter`, `leaf`, `virtual-leaf` (small named virtuals: `AffectsPed`,
`TakesPriorityOver`, `ProcessPed`, `SetPedPosition`, ...), `ctor`, `clone` (`Clone`/`CloneEditable`:
pool `operator new` through `orig!`, null check, constructor with copied fields), `dtor` (complete
destructors, SEH destructors that reset anim callbacks / release entity references, scalar deleting
destructors with `CTask::operator delete` or the inlined event/task pool delete), `make-abortable`,
`subtask` (`CreateSubTask`/`CreateFirstSubTask`/`CreateNextSubTask`/`ControlSubTask` bodies within the
subset, including switch-table dispatch to constructors). A function outside every family is listed in
`rest.tsv` ("not an AI-GEN family") even when it executes.

Excluded (other agents' tasks): `CTaskSimpleUseGun*`, `CTaskSimpleDuck*`, `CTaskSimpleFight*` (so also
`CTaskSimpleFightingControl`), `CTaskManager*`, `CEventHandler*`, `CPedGroupIntelligence*`,
`CTaskComplexCar*`, `CTaskSimpleCar*`, `CTaskComplexDrive*`, `CTaskComplexChase*`,
`CTaskComplexEnterCar*`, `CTaskComplexLeaveCar*`, and conservatively `CTaskComplexEnterAnyCar`/
`CTaskComplexLeaveAnyCar`; an unnamed or `vt_*` function is excluded when its nearest named neighbour
belongs to one of these classes. Functions of these classes are still called (through `orig!`).

Hookability: as PORT-AI-1 (a body shorter than the 5-byte patch up to its `ret`; a bare `jmp rel32`
entry, which sa-hook would class as a `.HOODLUM` stub; an INF-20 reader of the first bytes in
`catalog/code-data-reads.csv`): the port is generated with `/// Not hookable` and not registered.
`PRESERVE` in the script lists hooks whose original callers keep using ECX/EDX (sa-hook `callers` test):
0x4B1F80 (`CEventFireNearby::~CEventFireNearby`, ECX).

## Tests

- `gen_*::tests::bodies_match_the_exe` (via `gen_check.rs`): every generated function lists the spans it
  was matched on (stub, every executed instruction, switch tables) with a 64-bit FNV-1a hash; the test
  re-reads the exe and checks them all.
- `gen_*::tests::pattern_<family>`: every `const-id`/`const`/`const-bool`/`empty` port is called and must
  return the constant the bytes load.
- sa-hook lib tests (install on the exe, catalogue match, `callers` ECX/EDX, keycells, INF-20
  `data_reads`, `patch_refs`) with all generated hooks registered.

## Counts (generator run of commit 509f2ac)

1,698 functions generated (1,584 hooked, 114 ported-unhookable), 1,342 candidates left in `rest.tsv`.
`tools/gen/ai_patterns_exclude.txt` removes 35 addresses AI-2/AI-3 ported on their branches (16 of them
had been generated by the earlier builds).

| family | generated | hooked |
|---|---|---|
| const-id | 384 | 320 |
| const | 17 | 17 |
| const-bool | 80 | 68 |
| empty | 5 | 0 |
| getter | 50 | 40 |
| getter-f32 | 15 | 15 |
| setter | 20 | 20 |
| leaf | 3 | 3 |
| virtual-leaf | 46 | 45 |
| make-abortable | 62 | 61 |
| ctor | 224 | 224 |
| clone | 232 | 231 |
| dtor | 473 | 456 |
| subtask | 87 | 84 |

Unhookable: 97 bodies shorter than the patch up to `ret`, 13 bare `jmp rel32` entries, 4 INF-20 readers.
Main reasons left in `rest.tsv`: x87 code other than `fld; ret` (171), unknown value in a condition
(143, mostly protection-island junk and calls whose result feeds a test of ECX/EDX), outside the families
(113), stack addresses passed to callees (112), loops (105), call argument shapes (~220).

## Verification

Lane 6, `SA_ROOT=/home/ghosty2004/sa-engine`, frozen source exports (`git archive` into `target/agNsrc`, own
`CARGO_TARGET_DIR`), `tools/e2e/port-l2.sh`, recordings grove `target/e2e/lane-2/inf4/rec.sarec` (5200
frames) and corpus `c-bug` (3220), `pause-restart` (3700), `world-cheats` (3276), `anim-input` (3300),
`weapon-combat` (3500), `driving` (4338). Lists: `tools/e2e/lists/ai-gen2-l2.txt` (hooks that neither write
nor call: plain `l2`), `ai-gen2-split{0,1}.txt` (everything else: split mode, `L2_MAX=150
SPLIT_PER_FRAME=2`, with the 13 PORT-AI-1 global regions of `l2-extras.txt` passed in `L2_EXTRA`).

| build | run | result |
|---|---|---|
| ag1 (9e09841) | all ports on Rust, grove | **crash** at frame ~256 (write to 0x170 in `CEventEditableResponse::Constructor`, `this` = 0x16C); per-file bisection: ctors, clones, virtuals, subtasks crashed; root cause: callees that read ECX only after their first block (`CEventVehicleDamage::Constructor` 0x4B18D0, `CPedAttractor::Constructor` 0x5EDFB0, 0x6930F0, 0x685000, ...) were called without ECX. Fixed in e492778 |
| ag2 (e492778) | per-file bisection | every file reaches frame 5200 (exit-time Wine faults after the end frame only) |
| ag2 | all ports L3, 7 recordings | IDENTICAL on all 17 subsystems and every frame |
| ag2 | split grove/c-bug | 12 FAIL, all on EAX only: complete destructors returning the base's EAX, `MakeAbortable` upper EAX bytes. Fixed in 25785ae |
| ag3 (87e330a; 1,600 hooks, a superset of the final 1,584) | all ports L3, 7 recordings | **IDENTICAL** on all 17 subsystems and every frame of all seven recordings, 0 failures |
| ag3 | plain `l2` (474 pure hooks), 7 recordings | 0 failures: every called hook 0 mismatches, 0 undeclared; l2 trace == plain |
| ag3 | split part 0/1, 7 recordings | splito/splitr traces == plain everywhere; FAIL rows: 0x6389F0 and 31 unnamed destructors returning the base's EAX (fixed after the run in 8b9b4e9/a48edc4: return `()`, rerun pending), 0x4B1B60 (see below); undeclared writes: event constructors writing their own `this` object (no size declared for the heap object) |

Code changes after ag3 only make return types narrower (`u32` -> `()`) for 33 destructors and remove the 16
functions now in `ai_patterns_exclude.txt`; no generated body changed.

Open: `CEventGotKnockedOverByCar::CEventGotKnockedOverByCar` (0x4B1B60) split FAIL on driving: `this` is a
stack object of the caller (`CEventHandler` 0x4C079F) and the window comparison sees the original's SEH frame
bytes (`push -1` ...) written just below the object, which the port does not write (dead stack below the
caller's ESP); the object's fields compare equal. Still registered; a harness rule (stack below the
callee's entry ESP is not a write of the function) or unregistering it is a main-session decision.
Per-function verdicts: `docs/arch/ai-gen-verdicts.tsv` (state column, agent's proposal; the main session sets
`catalog/state.csv`):

| state | count |
|---|---|
| L2 (plain l2 PASS, or split PASS with 0 undeclared, on every recording where called) | 394 |
| L2-split (0 mismatches; undeclared writes in some run) | 40 |
| implemented (split FAIL on EAX only before the return was made `()`; rerun pending) | 5 |
| FAIL (0x4B1B60, above) | 1 |
| implemented-needs-L2 (hooked, no call in the seven recordings; covered by the all-ports L3 runs) | 1,144 |
| ported-unhookable | 114 |
