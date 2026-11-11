# Writer/verifier split: procedure and tool reference

Rules: docs/PLAN.md "Writer/verifier split". Queue: [QUEUE.md](QUEUE.md) (generated) and
`catalog/verify-queue.csv` (one row per unverified hook, with the exact verification command).

WRITERS port functions fast and need no game lane. VERIFIERS take one batch at a time onto a lane. The
unit that connects them is the **batch**: every port a writer adds is registered `unverified` with a batch
id, and a verifier enables, bisects and promotes whole batches. A divergence is first narrowed to one
batch (only that batch is enabled on top of the verified set), then to one hook (automatic bisection).

Unverified code never changes default behaviour: phase A installs an unverified port only when a run
selects it. Every all-ports run (`verify-all.sh`, `port-l2.sh ports`, `l3.sh --rust ports`,
`SA_HOOK_PORTS=rust`) runs exactly the verified set unless `SA_UNVERIFIED` says otherwise. Phase B/C
(native, sa-game) uses unverified ports like any other.

## 1. Registering an unverified port (writer)

```rust
#[hook(addr = 0x6D1A40, conv = thiscall, name = "CVehicle::Foo", this = "CVehicle",
       expect = "83 EC 10 56 8B F1",
       unverified = "veh3-a", route = "L2", hazards = "heap-writes,uninit-locals",
       recs = "driving,car-ai", note = "writes the vehicle's collision list")]
pub fn vehicle_foo(this: GPtr<CVehicle>) -> u32 { ... }
```

| key | required | values |
|---|---|---|
| `unverified` | yes (new hooks) | the batch id, `[a-z0-9][a-z0-9.-]*`, e.g. `veh3-a`. One batch = one writer commit, or at most ~60 hooks of one subsystem (the gate fails above 60). Use a fresh id per commit; reuse an id only to add hooks to your own not-yet-verified batch |
| `route` | yes | `L1-pure` (pure function: no game state beyond its arguments and constants; verified by `sa-oracle l1` on the real CPU, needs an L1 spec), `L2` (in-process double run, `port-l2.sh --runs "plain l2 rust"`), `split` (cannot run twice on one OS state: file/OS/thread effects, one-time initialisers; `--runs "plain split rust"`), `L3-only` (no usable L2 signature or every case is a nested call; L3 + hook-off only) |
| `hazards` | yes | `none`, or a comma list of the hazards below |
| `recs` | no | corpus recordings you expect to call it (names as in `target/e2e/corpus/*.sarec`, `grove-walk`); queue.py adds the ones INF-10 coverage shows |
| `note` | no | anything else the verifier must know |

The macro rejects an unverified hook without `route`/`hazards`, unknown routes or hazards and malformed
batch ids at compile time. Hazards (each one changes how the verifier runs it; see §5):

| hazard | meaning | the verifier's tool |
|---|---|---|
| `x87-transcendental` | uses FSIN/FCOS/FPTAN/FPATAN/FYL2X/F2XM1/FSQRT results bit-exactly | L1 on the native backend (CPU-decided), `sa_math::x87` models |
| `stack-residue` | original callers read the stack below their ESP after the call (residue) | `footprint = W` (ADR-001 §5b), split with `SPLIT_STACKFILL` |
| `entry-protection-read` | the no-CD protection reads the entry bytes as data | keycells / `catalog/code-data-reads.csv`; sa-hook lib test `every_code_key_read...` |
| `preserved-regs` | callers rely on ECX/EDX surviving | `preserve = "ecx,edx"`; L1 preserve specs, HARNESS-3 scans |
| `ax-al-return` | 16/8-bit result with the caller's upper EAX bits | `conv = reg("...; ret = al:entry")` / `ax:entry` (or `entry_eax`), `RetKind::Ax` specs |
| `uninit-locals` | the original reads locals it never wrote | split + `SPLIT_STACKFILL=4096..16384`, `SA_COV_PEEK` |
| `heap-writes` | writes heap memory the catalogue cannot name | lines in `tools/e2e/l2-extras.txt`, `--window-all` |

**Register conventions** (ADR-001 §4b, HARNESS-5). A function whose arguments arrive in registers other
than thiscall's ECX / fastcall's ECX+EDX (EAX, EBX, ESI, EDI, a byte in AL, a word in DI), whose
result is AL/AX with the caller's upper EAX, or whose ST0 result has 64 significant bits is no longer a
reason to skip it:

```rust
#[hook(addr = 0x70FA70, conv = reg("args = ecx edx di stack; keep = ecx edx"), name = "FUN_0070fa70",
       expect = "53 0F B7 1A 55", unverified = "harness5-1", route = "L1-pure", hazards = "preserved-regs")]
pub fn toggle_edge(edges: GPtr<u8>, count: GPtr<u16>, a: u16, b: u16) { ... }
```

- Read the convention off the disassembly: the registers read before they are written are the
  arguments (`args`, one location per Rust argument, in order; `stack` for the stack slots), `ret` vs
  `ret n` decides `pop`, a `push r ... pop r` pair is a save, not an argument. sa-hook's
  `regconv_check` test re-derives exactly this from the exe and fails on a mismatch, and fails when a
  caller reads EAX bytes the hook leaves undefined (declare `keep = eax` for a void function, `ret =
  al:entry` / `ax:entry` for an AL/AX result). ECX/EDX that callers keep across the call: `keep = ecx
  edx` (the `callers` test reports them as for `preserve`).
- Return `X87` for an ST0 result: the entry loads all 80 bits. A port calling such a function goes
  through `orig! { fn f(..) -> X87 = reg("...") 0xADDR => path; }` (not the Rust function directly), so
  the hook toggle governs the call and phase A gets the original's full ST0.
- L1 specs build the call with `Call::regconv(addr, &sa_oracle::hook_regconv(addr).unwrap(), &args,
  garbage)` (garbage fills the register bytes an AL/DI location does not cover); see
  `crates/sa-oracle/src/specs_harness5.rs`.

Writers do not run the game. Before hand-back:

```sh
tools/verify/writer-gate.sh [BASE]          # BASE = the commit you started from (default: merge-base with master)
python3 -I tools/verify/queue.py            # regenerate the queue (the gate checks it is up to date)
```

## 2. The writer gate (`tools/verify/writer-gate.sh`)

Steps (each a PASS/FAIL line in `target/writer-gate/summary.txt`, logs next to it):

| step | what |
|---|---|
| hooks | `queue.py gate BASE`: every hook added since BASE is unverified with batch, route, hazards; batch ≤ 60 hooks; notes when a batch spans subsystems or joins an existing batch; lists promotions |
| queue | `queue.py --check`: `catalog/verify-queue.csv`, `docs/verify/QUEUE.md` regenerated |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` (host build of everything) |
| i686 / i686clip | release build of `sa-launcher` + `sa-hook` for i686-pc-windows-gnu (every naked entry) and clippy of sa-hook (all port crates) for that target |
| wasm32 | `cargo build --target wasm32-unknown-unknown -p sa-ports` (every port crate) |
| hooktest | `cargo test -p sa-hook --lib`: table vs exe and catalogue, callers, patch_refs, keycells, INF-20 data reads; unverified hooks are checked like any other |
| l1test | `cargo test --release -p sa-oracle --test l1` |
| l1 | `sa-oracle l1 0xADDR --backend native --cases 1000000` for each new `L1-pure` hook; FAIL without a spec |

`GATE_SKIP="wasm32 l1test"` skips steps (state it in the hand-back), `GATE_CLEAN=1` deletes
`target/debug` afterwards, `L1_CASES`, `JOBS`, `MEM` as named. Every cargo call runs under
`systemd-run --user --scope -p MemoryMax=6G`. Keep ≥ 20 GB free on /home (each step checks).

## 3. Selecting unverified ports (`SA_UNVERIFIED`)

`SA_UNVERIFIED` = comma/newline list of `all`, batch ids, hook names or `0x` addresses, or `@FILE` (one
entry per line, `#` comments). It is read by:

- sa-hook (phase A DLL): the selected unverified ports are installed like every port; the others are not
  installed at all (the original's bytes stay untouched). A hook named in `SA_HOOK_MODE`, `SA_L2` or an
  `SA_HOOK_PORTS` list is selected too (so `port-l2.sh batch:ID` and `l3.sh --rust 0xADDR` work without the
  variable). `SA_L2=ports` and `SA_HOOK_PORTS=orig|rust` include only enabled hooks. sa-hook.log says
  `unverified ports: K of M selected (batches [...])`.
- sa-oracle l3 (`universe()`, so `--rust ports`, `--print-modes`) and `tools/e2e/hook-names.py`
  (`ports`, `all`; so `port-l2.sh ports` and `verify-all.sh`): the selected unverified hooks join `ports`.
- New set names (hook-names.py and `sa-oracle l3`): `batch:ID`, `unverified`, `verified`.
- The scripts convert `@/unix/path` to the Windows path for sa-hook (`lib.sh e2e_unverified_env`).

## 4. Verifier procedure (one batch at a time)

Build once per batch (the batch's code is in the DLL whether selected or not):

```sh
export CARGO_TARGET_DIR=$PWD/target SA_ROOT=~/sa-engine   # worktree: SA_ROOT holds wine/ and the corpus
systemd-run --user --scope -q -p MemoryMax=6G cargo build -j8 --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook
systemd-run --user --scope -q -p MemoryMax=6G cargo build -j8 --release -p sa-oracle-format -p sa-oracle
```

1. **Pick** the oldest batch with status `written` in QUEUE.md. Read its rows in
   `catalog/verify-queue.csv` (route, hazards, recordings, command).
2. **L3 on top of the verified set, narrowed automatically** on each recording that calls the batch
   (QUEUE.md lists them; start with the one most of its hooks hit):
   ```sh
   tools/verify/bisect-batch.sh BATCH $SA_ROOT/target/e2e/corpus/driving.sarec LANE
   ```
   Every verified port stays on Rust in every probe (`--fixed verified`); the batch is the tested set.
   IDENTICAL → next recording. DIVERGED → the report names each culprit: the hook, its catalogue subsystem
   and source line, the first divergent frame and the hash subsystems that differ, and the L2 command for
   that frame. A pair that only diverges together is reported as a minimal set. The rest of the batch is
   re-run without the culprits until it matches, so one run finds every independent culprit.
3. **L2 / split** of the batch (all hooks at once; rows with route `split` in a second run):
   ```sh
   SA_UNVERIFIED=BATCH LANE=N tools/e2e/port-l2.sh batch:BATCH REC --runs "plain l2 rust" --tag BATCH
   SA_UNVERIFIED=BATCH LANE=N SPLIT_STACKFILL=4096 tools/e2e/port-l2.sh @LIST REC --runs "plain split rust" --tag BATCH-split
   ```
   `queue.py list BATCH > LIST` gives an @FILE list; the per-hook commands in the CSV run one hook.
   Hooks with `NOCALLS` need another recording (or a list without their outer hook, l2-oracle.md).
4. **L1-pure** rows: `target/release/sa-oracle l1 0xADDR --backend native --cases 1000000`.
5. **Record** each result: `python3 -I tools/verify/queue.py set 0xADDR|batch:BATCH L1|L2|L3-batch|failed "EVIDENCE"`
   (evidence = the run tag/report path and the verdict line). `bisect-batch.sh` with `RECORD=1` records
   culprits as `failed` itself.
6. **Promote** (only with L2/split evidence for every row that has calls, or L1 for L1-pure, and the
   batch IDENTICAL on every recording): remove `unverified = "..."` from the attributes (route/hazards
   may stay as documentation), `tools/catalog/progress.py set ADDR L2|L3 EVIDENCE`, regenerate the queue
   (`queue.py` drops the status rows of promoted hooks), then the all-ports run of the new default set on
   all recordings must be IDENTICAL:
   ```sh
   tools/e2e/verify-all.sh promote-BATCH 1,2,3      # lanes; every recording IDENTICAL
   ```
   A promotion commit contains only the attribute edits, catalogue state and queue files.
7. **Failed** hooks go back to the writer: status `failed` with the bisect report line as evidence. The
   writer fixes the port in a new commit (same batch id, or the culprit moved to a new batch), and the
   batch is verified again from step 2. Never promote a batch with a `failed` row; promote the rest by
   moving the failed hook to its own batch first.

Lanes: every game run holds `target/locks/lane-N.lock` (flock, shared with other agents on that lane).
`verify-all.sh` spreads recordings over several lanes.

## 5. Tool reference

### `tools/e2e/port-l2.sh` (L2 + L3 of a hook set; docs/re/l2-oracle.md)

```text
port-l2.sh <HOOKS> [RECORDING] [--runs "plain l2 l2all rust hookoff split"] [--max N] [--window-all HOOKS] [--tag NAME]
```

HOOKS: names/addresses, `@FILE`, `state:STATE[:DOC]`, `doc:DOC`, `ports`, `batch:ID`, `unverified`,
`verified`. Runs: `plain` (all-original reference, cached in `target/l2cache`), `l2` (double run per
sampled call, `SA_L2_WINDOW=data`), `l2all` (with `--window-all`: heap arena watched too, slow), `rust`
(the list on Rust: L3 vs plain), `hookoff` (one run per hook, others on Rust; `HOOKOFF_ONLY=0x..`),
`split` (`splito` + `splitr` + `sa-oracle l2 split-report`). Checks (a) ≥ MAX cases or every call, 0
mismatches, 0 undeclared writes; (b) l2 trace == plain; (c) rust == plain on every frame, 17 subsystems;
(d) hook-off; (e) split verdicts PASS / PASS-FP / FAIL / DIVERGED / COUNT / NOCASES and 0 undeclared.
Output `target/e2e/lane-N/port-l2-TAG/{summary.txt,results.tsv}`.

Env: `L2_MAX` (= `--max`, default 1000), `L2_PER_FRAME` (default 4; raise for bursty hooks),
`L2_FROM` (first sampled frame), `L2_EXTRA` (extra regions for every run), `RUN_TIMEOUT` (1500 s),
`CRASH_WAIT` (seconds to let winedbg write the backtrace of a crashing run to `wine.log`; under the e2e runs it
prints nothing, so set `SA_CRASH_LOG=1` instead: sa-hook logs the registers and 96 stack dwords from ESP and
EBP of the first access violations to sa-hook.log (port-l2: `RUN.log`, e.g. `rust.log`); a return address in sa_hook.dll names a port),
`PLAIN_CACHE=0`, `L2_SPOOL_DISCARD=1` (spool to a drained FIFO: bounded disk, no host report),
`SA_COV*` passthrough.

Split flags: `SPLIT_PER_FRAME` (default 1000), `SPLIT_STACKFILL=BYTES` (zero that much stack below the
invoker before every sampled call in both runs; for `uninit-locals`/`stack-residue`), `SPLIT_DUMP_SEQ=N,…`
(dump the watched windows at those cases; compare with `sa-oracle l2 dump-diff A.win B.win`).

Quotas: `L2_SPOOL_MAX` (bytes, `k/M/G`, default 2 GiB): sa-hook stops spooling at the quota and the run
gets a FAIL line (cases before it stay valid); every run checks ≥ 20 GB free on /home.

Extras (`tools/e2e/l2-extras.txt`): `HOOK@ADDR:LEN[:R|W|RW]` with expressions (`ecx`, `argN`, `[EXPR]`,
`+`/`-`), `post:` regions, `os:file:EXPR`/`os:cwd`/`os:cursor` probes, `doc:DOC@REGION` for a whole doc
table. A `heap-writes` hook without its extras shows undeclared writes; `tools/e2e/l2-writers.py
undeclared` lists the instructions that can write them.

Footprint (`#[hook(footprint = W)]`, ADR-001 §5b): for callees whose stack residue original code reads;
the port records its writes with `sa_core::abi::stack_footprint`.

### L1 (`sa-oracle`, docs/re/l1-harness.md)

`sa-oracle list` (specs and routes), `sa-oracle l1 <key|name|0xADDR|all> [--cases N] [--seed S]
[--backend auto|unicorn|native] [--cw 0x7f] [--mxcsr 0x9fa0]`. Specs live in
`crates/sa-oracle/src/specs*.rs`; the native backend runs the original on the real CPU under Wine
(authoritative for x87 transcendentals; Unicorn is not). `cargo test --release -p sa-oracle --test l1`
self-tests the harness, the preserve declarations and the routes.

### L3 and bisection (`tools/e2e/l3.sh`, `sa-oracle l3`; docs/re/l3-coverage.md)

```text
l3.sh [--rec R] [--rust SET] [--fixed SET] [--hookoff none|all|SET] [--margin N] [--mutant A,B]
l3.sh --l2 HOOK --from F --end E [--rec R] [--rust SET]
```

A reference replay with every port of the universe on the original (cached by recording, exe, DLL,
modes, end frame), then the `--rust` set; on a divergence the set is halved down to a minimal culprit
set (window = first divergent frame + margin, default 100), repeated until the rest matches. `--fixed
SET` keeps SET on Rust in every probe and bisects only `--rust` on top of it (the fixed set alone must
match first). SETs: `ports`, `verified`, `unverified`, `batch:ID`, names/addresses. `SA_HOOK_MODE`
lists over thousands of hooks go to sa-hook as `@FILE`. Report: `target/e2e/lane-N/l3/run.txt`
(`CULPRIT`, `alone on Rust: diverges at frame F: SUBSYSTEMS`, `suggest L2: …`, `HOOKOFF`, `probes`).

### `tools/verify/bisect-batch.sh BATCH RECORDING LANE [--margin N] [--hookoff]`

`l3.sh --rust batch:BATCH --fixed verified` with `SA_UNVERIFIED=BATCH`, plus a culprit summary
(hook, catalogue subsystem, source line, route, hazards, first divergent frame, hash subsystems, the
next L2 command and the `queue.py set … failed` line). Output `target/verify/BATCH/bisect-REC.{txt,log}`
and `l3-REC.txt`. Exit 0 identical, 1 culprits, 2 setup error. `RECORD=1` records culprits.
The report is copied out of the lane's `l3/run.txt` by l3.sh itself while it holds the lane lock
(`L3_REPORT_COPY=FILE`), so concurrent runs on the same lane cannot swap reports.

### `tools/e2e/verify-all.sh TAG LANES [REC...]`

`port-l2.sh ports --runs "plain rust"` on every corpus recording, spread over lanes; every recording must be
IDENTICAL. Includes unverified hooks only with `SA_UNVERIFIED`.

### `tools/verify/queue.py`

`queue.py` (regenerate), `--check`, `set KEY STATUS [EVIDENCE]`, `list BATCH`, `show BATCH`, `gate BASE`.
Status values: `written` (default), `L1`, `L2` (L2 or split evidence), `L3-batch` (the batch was IDENTICAL
on every recording on top of the verified set), `failed`. Sources: the attributes, `catalog/procedures.csv`,
`catalog/verify-status.csv` (written by `set`), INF-10 coverage `catalog/raw/coverage-KEY.csv` of the
data checkout (SA_ROOT or the main checkout), git (first commit adding the batch id).
