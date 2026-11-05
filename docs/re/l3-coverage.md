# L3 runner with divergence bisection (INF-9) and execution coverage of the original (INF-10)

Code: `crates/sa-oracle/src/l3.rs` (comparison, bisection, hook-off check, external runner with trace
cache, CLI `sa-oracle l3`), `crates/sa-oracle/src/coverage.rs` (breakpoint points, hit merge, reports,
CLI `sa-oracle coverage`), `crates/sa-hook/src/coverage.rs` (in-process first-hit breakpoints),
`tools/e2e/l3.sh` (game runs, self-test, L2 follow-up), `tools/e2e/inf10-coverage.sh`.

## INF-9: L3 runner and automatic bisection

**Universe and sets.** The universe is every hook with an L2 signature that is not an oracle instrument
(today the 8 sa-hook ports: CTimer::Update, CGeneral::LimitRadianAngle, CGameLogic::IsCoopGameGoingOn,
CGeneral::LimitAngle, GetRadianAngleBetweenPoints, GetATanOfXY, SolveQuadratic, CVector::Normalise;
after the ARCH-1 merge also every `Group::Port` hook, installed by the probes with
`SA_HOOK_PORTS=orig`). A probe sets `SA_HOOK_MODE` for the whole universe: `ADDR=rust` for its set,
`ADDR=orig` for the rest. The reference is the universe entirely on the original.

**Probe.** One determinism-mode replay of the recording from process start (`SA_DET=1`,
`SA_DET_REPLAY`, `SA_DET_END=E`, `SA_DET_EXIT=1`), writing the per-frame, per-subsystem hash trace
(INF-6). A probe "diverges" when its trace differs from the reference at some frame ≤ E, or ends
before E (game exited, crashed or hung: reported as `trace ends before frame N`).

**Algorithm** (`Bisector` in l3.rs):
1. Full run of the Rust set S over the whole recording → first divergent frame F and the subsystems
   that differ there (or MATCH: the L3 check passes).
2. `minimize(∅, S)` with window E = F + margin (default 100 frames): split the candidates in halves A,
   B; if A alone (plus the fixed context) diverges by E recurse into A, else if B does recurse into B,
   else the halves interact: minimise A with B kept on Rust, then B with the minimum of A kept on Rust.
   The result M is minimal: removing any hook of M stops the divergence within the window. Single
   culprits cost ⌈log2 |S|⌉ + 1 probes, each only as long as the window.
3. Report M with the frame/subsystems where M alone diverges, and per hook a suggested L2 run
   (`SA_L2=ADDR SA_L2_FROM=frame-1 …`, runnable as `tools/e2e/l3.sh --l2 ADDR --from N --end N`). The
   hash at frame boundary f covers the state left by frame f-1, so the culprit's first wrong result is
   in frame f-1 or earlier (earlier when it first changed unhashed state).
4. Iterate (default): take M out of S and run the rest in full again; repeat until the rest matches.
   Every independent culprit is reported. `--no-iterate` stops after the first.

Probe results are memoised within a session (a set run to E answers any window ≤ E, and a divergence
at f answers any window ≥ f) and complete traces are cached across sessions under `target/l3/cache`,
keyed by FNV-64 of: recording bytes, gta_sa.exe, sa_hook.dll, the universe modes, the end frame and the
environment variables that change a run (`SA_DET_STEP_US`, `SA_DET_SEED`, `SA_DET_PERTURB`, and
`SA_L2_MUTANT` for probes with at least one Rust hook, so the reference is shared). The reference is therefore cached per recording + exe (+ dll: the hash definitions
and the determinism layer live in the dll, so a rebuilt dll re-runs the reference).

**Hook-off check** (PLAN §5 L3): for every F of a matching set S (or the matching rest after the
culprits were taken out), replay S \ {F} on Rust over the whole recording; it must match the reference.
`sa-oracle l3 --hookoff all|A,B`.

**Mutant without L2.** `SA_L2_MUTANT` is now honoured without `SA_L2` (sa-hook `l2::mutants_from_env`,
read in `l2win::init`), so a deliberately wrong port can run as a plain Rust hook. The L3 self-test uses
the LimitRadianAngle mutant (last bit of the f32 result flipped for |angle| > 1).

Commands:
```
tools/e2e/l3.sh [--rec R] [--rust ports|A,B] [--hookoff all] [--mutant A,B]
tools/e2e/l3.sh --selftest
tools/e2e/l3.sh --l2 0x53cb50 --from 264 --end 400
sa-oracle l3 --rec R --runner "tools/e2e/l3.sh --probe" [--rust ports] [--margin 100] [--hookoff all]
sa-oracle l3 --print-modes ports --except A
```

## INF-10: execution coverage of the original

### DynamoRIO drcov under Wine: does not work here (measured)

DynamoRIO-Windows 11.91.20756 (32-bit `bin32/drrun.exe -t drcov`) under Wine 11.20 (lane-3 prefix,
`#arch=win64`, Wine's new WoW64 mode): `drrun -t drcov -- C:\windows\syswow64\cmd.exe /c echo hello`
crashes in DynamoRIO's injector before the target runs: `Unhandled exception: page fault on read access
to 0xffffffff in wow64 32-bit code (0x7abd40ef)`, `drinjectlib+0x40ef: movl %gs:0x60, %eax` (the injector
reads the 64-bit PEB through `gs`, which it assumes on a WOW64 Windows; Wine's new WoW64 has no such
segment for 32-bit code). A pure 32-bit prefix, where DynamoRIO would take its non-WOW64 path, cannot be
created: `wine: WINEARCH is set to 'win32' but this is not supported in wow64 mode.` (the installed Wine is
a wow64-only build). drcov is therefore not usable without a different Wine build (a classic
multilib/win32 Wine), which would be a system install. Log: `target/dynamorio/drcov-wine-t2.log`.

### sa-hook first-hit breakpoints (used)

`sa-oracle coverage points` lists, for every catalogue row in game `.text` except `excluded:protection`
and `excluded:mis-split`:
- the function entry (always; it must decode as an instruction);
- with a Ghidra body (`catalog/extents.csv` ranges plus the entry instruction itself): every basic-block
  start found by recursive descent from the entry inside those ranges: fall-through, conditional and
  direct jump targets, and switch tables `jmp [table + reg*4]` (entries read while they land in the
  body). An address becomes a point only if it decoded as an instruction start inside the body and lies
  inside no other decoded instruction and no jump table, so a breakpoint never lands in data or
  mid-instruction. `.HOODLUM` bodies are reached from their stubs and count for the stub's function.

Each point carries the exe byte. With `SA_COV=points` sa-hook arms, after every hook and determinism
patch is installed and while the game's threads are still suspended, an `int3` at every point whose
current byte matches (a hook entry, patched to `jmp thunk`, is armed too: it then counts entries to the
hook whatever its mode). A vectored exception handler counts the trap, restores the byte and resumes at
the same address: each point costs one exception the first time it runs and nothing afterwards
(`SA_COV_CAP=N` re-arms by single-stepping one instruction, until N hits). The hits are written as
`address,hits` (`SA_COV_OUT`) every 10 s and at the end frame.

Check that it does not perturb the game: the coverage replay's hash trace must equal a plain replay's
(`inf10-coverage.sh` step 3).

Outputs: `catalog/raw/coverage-NAME.hits.csv` (every armed point), `catalog/raw/coverage-NAME.csv`
(per function: `address,hits,blocks_hit,blocks_total,subsystem,class,state,name`),
`catalog/raw/coverage-all.*` (merge of every recording: union of points, hits summed), and the
per-subsystem report (functions hit / non-excluded catalogue total, blocks hit / armed blocks; excluded
classes listed separately for F2).

```
tools/e2e/inf10-coverage.sh [--rec R] [--name grove-walk] [--cap 1] [--rust none|ports]
sa-oracle coverage points --out P [--no-blocks]
sa-oracle coverage report --points P --hits A.csv,B.csv [--csv F] [--merged M] [--out REPORT]
```

Use for F2 (`--rust ports`: what original code still runs with every port on Rust; hook entries are
expected), F8 (block coverage of the corpus) and SAMP-1D (the same points with samp.dll loaded; the
coverage mode does not need determinism mode).

## Results (2026-11-05, lane 3, grove-walk recording `target/e2e/lane-2/inf4/rec.sarec`, 5,200 frames)

**L3 self-test, tree merged with master 696726a** (universe = 35 hooks: the 8 sa-hook ports + 27 sa-core
ports), `tools/e2e/l3.sh --selftest`:
- clean: all 35 on Rust → `L3 MATCH ... identical to the reference for all 5200 frames`; hook-off check:
  35 runs (each hook back on the original alone), all MATCH. 37 game runs of ~110 s.
- mutant (`SA_L2_MUTANT=CGeneral::LimitRadianAngle`): full run `DIVERGED at frame 210: peds`; bisection
  probes {first 17} → {first 8} → {first 4} → {CTimer::Update, LimitRadianAngle} →
  {LimitRadianAngle}: `CULPRIT {CGeneral::LimitRadianAngle} ... alone on Rust: diverges at frame 210:
  peds`, `suggest L2: SA_L2=0x53cb50 SA_L2_FROM=209 ...`; the rest (34 hooks) matches for all 5200 frames.
  10 probes, 9 game runs (5 of them only to frame 310). Verdict `PASS injected fault found without help`.

**Same self-test on the tree merged with master 376b089** (785 hooks registered: sa-core + sa-stream;
Rust set = the same 35 hooks, hook-off on the 8 sa-hook ports, `L3_SELFTEST_HOOKOFF`): clean MATCH,
8/8 hook-off MATCH, mutant culprit {CGeneral::LimitRadianAngle} at frame 210 (peds), PASS.

**INF-10 coverage of the grove walk: partial.** Points: 169,618 (20,753 function entries, 148,865 block
starts; 15,471 functions with body ranges, 5,282 entry-only; 728 jump tables; 1,409 rows not armed).
All armed with 0 byte mismatches. int3 calibration under Wine 11.20 WoW64: `ExceptionAddress=int3+0
Eip=int3+0` (same as Windows).

With every point armed the replay stops in the loading screen (frame 202; the last first-hits are in
`CLoadingScreen::DoPCScreenChange` 0x590AC0 and its fade loop). `tools/e2e/cov-bisect.py` (24 probes,
binary splitting over the address-sorted points, each probe a replay to frame 320) isolated **one**
point: 0x55C410, the entry of `CStats::SetNewRecordStat` (`jmp 0x408235`, a 5-byte stub into a chunk
that jumps back to 0x55C415). That point alone stops the replay; any half without it reaches frame 320.
No sa-hook/sa-core/sa-stream code reads that address. Suspected: the game or its protection layer
reads these entry stubs. Not analysed further (no REA in this task). It is listed in the hand-maintained
`catalog/coverage-exclude.csv`, which `coverage points` honours.

Rerun without 0x55C410 (tree merged with 376b089, 796 hook entries armed): the replay still stops. The
trace reaches frame 256 (frames 1–256 identical to the plain replay), and the control channel's last
status was frame 204, again in the `DoPCScreenChange` fade loop. So there is at least one more such
point; the next step is `tools/e2e/cov-bisect.py` with this points file. Coverage up to the stop (boot,
menus, new-game loading) from `catalog/raw/coverage-grove-walk.csv`:

```
subsystem (non-excluded)   functions hit/total        blocks hit/armed
AI                            61 / 4047   1.5%      210 / 35180   0.6%
UNK-game                    1630 / 2886  56.5%     1830 / 9621   19.0%
RW                           467 / 1805  25.9%     2594 / 18108  14.3%
AUDIO                        236 / 842   28.0%     1046 / 9730   10.8%
RENDER                       137 / 832   16.5%      587 / 10018   5.9%
STREAM                        72 / 552   13.0%      402 / 5517    7.3%
HUD                          113 / 342   33.0%      804 / 6309   12.7%
CORE                         128 / 319   40.1%      629 / 2290   27.5%
... (full table: target/e2e/lane-3/inf10/report-grove-walk.txt)
TOTAL non-excluded          3084 / 15479 19.9%     9681 / 164326  5.9%
excluded:crt 185/510, eh-funclet 0/3556, linker-thunk 8/26 (mis-split/protection not armed)
```

## Limits

- Bisection assumes a culprit set that diverges within F + margin when run on its own context; a hook
  whose effect appears only much later than F with other hooks off is still found by the interaction
  branch, but with more probes. Divergences outside the hashed state (docs/re/determinism.md §7) are
  invisible to L3.
- Block points: functions without a Ghidra body (5,282 of 20,760 armed functions, mostly gta-reversed
  addresses Ghidra did not create) get their entry only. 121 indirect jumps are not switch tables of the
  recognised form (their targets are found only through other edges). Block starts reached only through
  those, through exception handlers or through call-return sites are not points.
- Hits are capped (default 1 = executed or not). Two threads hitting one point at once may count
  twice; with a cap > 1 a point executed by another thread while it is disarmed is not counted.
- The `--l2` follow-up mode of l3.sh was written but not exercised in the acceptance runs.
- Hook-off with the full registered universe (785 hooks after PORT-STREAM) means 785 full replays
  (~110 s each); pass a list (`--hookoff A,B`, `L3_SELFTEST_HOOKOFF`).
- Not armed: `excluded:protection` (protection junk) and `excluded:mis-split` rows (their code belongs to
  the containing function), the rw-cseg region.
