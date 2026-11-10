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

Outputs (KEY = NAME, or `rust-SET-NAME` with `--rust SET`, so runs with ports on Rust never mix with
plain ones): `catalog/raw/coverage-KEY.hits.csv` (every armed point), `catalog/raw/coverage-KEY.csv`
(per function: `address,hits,blocks_hit,blocks_total,subsystem,class,state,name`),
`catalog/raw/coverage-all.*` / `coverage-rust-SET-all.*` (merge of every recording of the same port mode:
union of points, hits summed), and the per-subsystem report (functions hit / non-excluded catalogue
total, blocks hit / armed blocks; excluded classes listed separately for F2). In
`target/e2e/lane-N/inf10/`: `cov-KEY.trace` and `ref-KEY.trace` (coverage replay and plain replay of
the same recording and port modes), `ref-vs-cov-KEY.txt`, `report-KEY.txt`. `catalog/raw/` is gitignored.

```
tools/e2e/inf10-coverage.sh [--rec R] [--name grove-walk] [--cap 1] [--rust none|ports]
sa-oracle coverage points --out P [--no-blocks]
sa-oracle coverage report --points P --hits A.csv,B.csv [--csv F] [--merged M] [--out REPORT] [--rust-modes MODES.txt]
tools/e2e/cov-data-reads.py POINTS [--hooks HOOKS.txt]   # static scan: points the protection reads as data
tools/e2e/cov-bisect.py POINTS REC END                    # dynamic: the point whose int3 stops a replay
```

`--rust-modes MODES.txt` (the `sa-oracle l3 --print-modes ports` string of the run) adds the F2 section:
per subsystem the original functions still executed (entry hit, not on Rust) and their blocks, the
hooks on Rust, how many were entered, and the Rust hooks whose original body blocks still ran.

Use for F2 (`--rust ports`), F8 (block coverage of the corpus) and SAMP-1D (the same points with
samp.dll loaded; the coverage mode does not need determinism mode).

### Points the game reads as data (`catalog/coverage-exclude.csv`)

The no-CD build's protection splices chunks into game code (in the 0x401000–0x40B000 area of game
`.text` and in `.HOODLUM`) that compute pointers and keys from **code bytes**, e.g.
`9C F9 A1 7C 06 56 01 1B 05 10 C4 55 00 8B 00 9D` = `pushfd; stc; mov eax,[0x156067C];
sbb eax,[0x55C410]; mov eax,[eax]; popfd`. An `int3` on 0x55C410 changes the key, the computed pointer
is wrong and the game goes astray (the original symptom: the replay hangs in the loading-screen fade
loop). These points are not armed; each is a `jmp rel32` whose target is reached only through it, so
the report counts the excluded point with the target's hits (`proxy` column).

How the list was made complete (systematic, 2026-11-10):
1. Static byte scan, `tools/e2e/cov-data-reads.py`: capstone decodes, at every executable offset in game
   `.text` and `.HOODLUM` where a dword pointing into game `.text` follows a disp32 ModRM/SIB or moffs
   opcode, the instruction that would read it, and keeps non-indexed memory reads that decode in line
   from a `pushfd`/`stc`/`clc` a few bytes before (the chunks' shape). 246 protection reads; the points
   inside a read range (`[addr, addr + operand size)`) are exactly the 12 rows of the CSV, each with
   its reader bytes. Without the prologue filter (373 reads) 17 more points are hit, all by misaligned
   decodes of ordinary code or of encrypted `.HOODLUM` bytes (`les eax,[...]` = VEX junk,
   `fadd qword [0x6A0000]` inside other instructions, 0x1559BF8/0x155A0F5 in undecodable data); none is
   a real read.
2. Ghidra data references (headless dump of every non-flow reference into game `.text`, 28,617 refs:
   READ 1,589, READ_WRITE 66, WRITE 353, DATA 26,610). Restricted to referencing code in game `.text` or
   `.HOODLUM` (the second `.text`/`.data` at 0xCB1000–0x1556000 is the unexecuted original image):
   READ/READ_WRITE/WRITE refs whose 4-byte range contains a point = the excluded points (11 of 12;
   Ghidra did not disassemble the reader of 0x50AC20, at 0x1566C27) plus one artefact
   (`jmp [edx*4+0x815E18]` with `ja` guard, Ghidra's ref to table-4 = 0x815E14). No wider (qword, rep)
   read touches a point. DATA refs from instructions are address-taking (`push offset`,
   `xchg [esp],reg` of a return address), not reads.
3. Dynamic check: the grove walk (5,200 frames) and pause-restart (3,700) coverage replays with every
   other point armed complete and their traces are identical to plain replays, with ports on the
   original and with ports on Rust. So no further point of the corpus' paths is read as data, also by
   readers the two static scans cannot see (indexed / register-based reads).
4. Per-point validation (each excluded point armed alone on top of every armed point, grove walk,
   trace compared with the plain replay): see the results below.

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

Rerun without 0x55C410 (tree merged with 376b089): the replay still stopped in the same fade loop
(frame 204). The stopped agent then added the static scan and 11 more exclusions (not re-verified at
the time). The 2026-11-05 partial table is superseded by the results below.

## Results (2026-11-10, lane 5, worktree merged with master 3770f5f: 1,139 port hooks + core hooks)

Points: 169,689 armed (20,741 entries, 148,948 blocks; 15,471 functions with body ranges, 5,282
entry-only; 730 jump tables; 1,409 rows not armed) + 12 excluded with a proxy. All armed with 0 byte
mismatches (1,148 hook entries). DXVK without vsync (master 5ced978's `tools/wine-env.sh`): about
35 s per 5,200-frame replay. Every run below **reached its end frame with a trace IDENTICAL to the
plain replay of the same recording and port modes**:

| run | frames | first-hit points | trace vs plain replay |
|---|---|---|---|
| grove walk, ports original | 5,200 | 51,932 | IDENTICAL over 5,200 |
| pause-restart, ports original | 3,700 | 47,229 | IDENTICAL over 3,700 |
| grove walk, `--rust ports` | 5,200 | 47,270 | IDENTICAL over 5,200 |
| pause-restart, `--rust ports` | 3,700 | 42,646 | IDENTICAL over 3,700 |

### Coverage per subsystem (ports on the original)

Functions hit / non-excluded catalogue functions, blocks hit / armed block points (entries included;
excluded points counted through their proxy). Merged = union of both recordings.

| subsystem | functions: grove walk | pause-restart | merged | blocks: grove walk | pause-restart | merged |
|---|---|---|---|---|---|---|
| AI | 1156 / 4047 (28.6%) | 872 (21.5%) | 1184 (29.3%) | 7712 / 35180 (21.9%) | 5795 (16.5%) | 7969 (22.7%) |
| UNK-game | 1915 / 2883 (66.4%) | 1877 (65.1%) | 1918 (66.5%) | 3049 / 9618 (31.7%) | 2893 (30.1%) | 3077 (32.0%) |
| RW | 820 / 1805 (45.4%) | 824 (45.7%) | 826 (45.8%) | 5948 / 18108 (32.8%) | 5957 (32.9%) | 5984 (33.0%) |
| WORLD | 356 / 918 (38.8%) | 353 (38.5%) | 371 (40.4%) | 3574 / 13045 (27.4%) | 3306 (25.3%) | 3683 (28.2%) |
| AUDIO | 442 / 842 (52.5%) | 435 (51.7%) | 448 (53.2%) | 3216 / 9730 (33.1%) | 3004 (30.9%) | 3276 (33.7%) |
| RENDER | 518 / 832 (62.3%) | 519 (62.4%) | 534 (64.2%) | 5083 / 10018 (50.7%) | 4839 (48.3%) | 5175 (51.7%) |
| VEH | 262 / 714 (36.7%) | 210 (29.4%) | 264 (37.0%) | 3794 / 13741 (27.6%) | 3016 (21.9%) | 3826 (27.8%) |
| STREAM | 365 / 553 (66.0%) | 363 (65.6%) | 370 (66.9%) | 3431 / 5518 (62.2%) | 3401 (61.6%) | 3492 (63.3%) |
| POP | 204 / 430 (47.4%) | 194 (45.1%) | 208 (48.4%) | 3440 / 9093 (37.8%) | 2941 (32.3%) | 3468 (38.1%) |
| PED | 168 / 372 (45.2%) | 165 (44.4%) | 173 (46.5%) | 1660 / 5760 (28.8%) | 1617 (28.1%) | 1690 (29.3%) |
| HUD | 191 / 342 (55.8%) | 194 (56.7%) | 194 (56.7%) | 1729 / 6309 (27.4%) | 1758 (27.9%) | 1794 (28.4%) |
| CORE | 202 / 319 (63.3%) | 207 (64.9%) | 207 (64.9%) | 1054 / 2290 (46.0%) | 1077 (47.0%) | 1079 (47.1%) |
| ANIM | 172 / 265 (64.9%) | 159 (60.0%) | 175 (66.0%) | 1311 / 2036 (64.4%) | 1179 (57.9%) | 1319 (64.8%) |
| SCRIPT | 121 / 223 (54.3%) | 122 (54.7%) | 124 (55.6%) | 1489 / 7350 (20.3%) | 1418 (19.3%) | 1507 (20.5%) |
| CAM | 90 / 218 (41.3%) | 90 (41.3%) | 90 (41.3%) | 1055 / 5498 (19.2%) | 1041 (18.9%) | 1056 (19.2%) |
| WEAPON | 52 / 158 (32.9%) | 53 (33.5%) | 53 (33.5%) | 267 / 3420 (7.8%) | 271 (7.9%) | 271 (7.9%) |
| INPUT | 73 / 155 (47.1%) | 73 (47.1%) | 73 (47.1%) | 616 / 2051 (30.0%) | 612 (29.8%) | 618 (30.1%) |
| COLL | 107 / 145 (73.8%) | 105 (72.4%) | 107 (73.8%) | 1611 / 2064 (78.1%) | 1494 (72.4%) | 1612 (78.1%) |
| MATH | 71 / 112 (63.4%) | 69 (61.6%) | 71 (63.4%) | 208 / 308 (67.5%) | 203 (65.9%) | 208 (67.5%) |
| PHYS | 47 / 73 (64.4%) | 43 (58.9%) | 47 (64.4%) | 991 / 2174 (45.6%) | 740 (34.0%) | 995 (45.8%) |
| CALLED-ONLY | 23 / 37 (62.2%) | 21 (56.8%) | 23 (62.2%) | 367 / 699 (52.5%) | 341 (48.8%) | 375 (53.6%) |
| SAVE | 2 / 36 (5.6%) | 2 (5.6%) | 2 (5.6%) | 8 / 409 (2.0%) | 8 (2.0%) | 8 (2.0%) |
| **TOTAL non-excluded** | 7357 / 15479 (47.5%) | 6950 (44.9%) | 7462 (48.2%) | 51613 / 164419 (31.4%) | 46911 (28.5%) | 52482 (31.9%) |
| excluded:crt | 220 / 510 (43.1%) | 218 (42.7%) | 220 (43.1%) | 220 / 510 (43.1%) | 218 (42.7%) | 220 (43.1%) |
| excluded:eh-funclet | 0 / 3556 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0 / 3556 (0.0%) | 0 (0.0%) | 0 (0.0%) |
| excluded:linker-thunk | 15 / 26 (57.7%) | 15 (57.7%) | 15 (57.7%) | 15 / 26 (57.7%) | 15 (57.7%) | 15 (57.7%) |
| excluded:mis-split | 0 / 660 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0 / 0 (-) | 0 (-) | 0 (-) |
| excluded:protection | 0 / 741 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0 / 0 (-) | 0 (-) | 0 (-) |
| excluded:thirdparty:d3dx9 | 89 / 955 (9.3%) | 89 (9.3%) | 89 (9.3%) | 89 / 955 (9.3%) | 89 (9.3%) | 89 (9.3%) |
| excluded:thirdparty:libpng-1.0.12+zlib-1.1.3 | 0 / 197 (0.0%) | 0 (0.0%) | 0 (0.0%) | 0 / 197 (0.0%) | 0 (0.0%) | 0 (0.0%) |
| excluded:thirdparty:quicktime-qtml | 2 / 38 (5.3%) | 2 (5.3%) | 2 (5.3%) | 2 / 38 (5.3%) | 2 (5.3%) | 2 (5.3%) |
| **TOTAL excluded** | 326 / 6683 (4.9%) | 324 (4.8%) | 326 (4.9%) | 326 / 5282 (6.2%) | 324 (6.1%) | 326 (6.2%) |

### F2: original code still executed with every port on Rust

`--rust ports` (1,139 hooks on Rust: the 1,138 engine/sa-hook ports + the CTimer::Update passthrough),
`coverage report --rust-modes`. "orig functions hit" = entry executed and not a Rust hook; "ports" =
Rust hooks of the subsystem, "entered" = their entry executed (the Rust code ran), "orig body" = Rust
hooks whose original body blocks still ran.

| subsystem | orig functions hit, ports on Rust: grove walk | pause-restart | merged / catalogue | merged, all on original | orig blocks (merged) | Rust hooks | entered | orig body |
|---|---|---|---|---|---|---|---|---|
| UNK-game | 1914 | 1876 | 1917 / 2883 | 1918 | 3076 | 0 | 0 | 0 |
| AI | 1156 | 872 | 1184 / 4047 | 1184 | 7969 | 0 | 0 | 0 |
| RW | 820 | 824 | 826 / 1805 | 826 | 5983 | 0 | 0 | 0 |
| RENDER | 518 | 519 | 534 / 832 | 534 | 5175 | 0 | 0 | 0 |
| AUDIO | 442 | 435 | 448 / 842 | 448 | 3276 | 0 | 0 | 0 |
| WORLD | 264 | 260 | 277 / 918 | 371 | 3208 | 235 | 94 | 0 |
| VEH | 261 | 209 | 263 / 714 | 264 | 3822 | 1 | 1 | 0 |
| POP | 204 | 194 | 208 / 430 | 208 | 3468 | 0 | 0 | 0 |
| HUD | 191 | 194 | 194 / 342 | 194 | 1794 | 0 | 0 | 0 |
| ANIM | 172 | 159 | 175 / 265 | 175 | 1319 | 0 | 0 | 0 |
| PED | 166 | 163 | 171 / 372 | 173 | 1686 | 2 | 2 | 0 |
| SCRIPT | 121 | 122 | 124 / 223 | 124 | 1507 | 0 | 0 | 0 |
| CAM | 90 | 90 | 90 / 218 | 90 | 1056 | 0 | 0 | 0 |
| INPUT | 73 | 73 | 73 / 155 | 73 | 618 | 0 | 0 | 0 |
| MATH | 64 | 62 | 64 / 112 | 71 | 160 | 8 | 7 | 0 |
| WEAPON | 52 | 53 | 53 / 158 | 53 | 271 | 0 | 0 | 0 |
| PHYS | 47 | 43 | 47 / 73 | 47 | 995 | 0 | 0 | 0 |
| CORE | 31 | 34 | 34 / 319 | 207 | 330 | 269 | 173 | 1 |
| STREAM | 28 | 28 | 28 / 553 | 370 | 196 | 504 | 335 | 0 |
| CALLED-ONLY | 23 | 21 | 23 / 37 | 23 | 375 | 0 | 0 | 0 |
| COLL | 23 | 23 | 23 / 145 | 107 | 752 | 120 | 84 | 0 |
| SAVE | 2 | 2 | 2 / 36 | 2 | 8 | 0 | 0 | 0 |
| excluded:crt | 220 | 218 | 220 / 510 | 220 | 220 | 0 | 0 | 0 |
| excluded:thirdparty:d3dx9 | 89 | 89 | 89 / 955 | 89 | 89 | 0 | 0 | 0 |
| excluded:linker-thunk | 15 | 15 | 15 / 26 | 15 | 15 | 0 | 0 | 0 |
| excluded:thirdparty:quicktime-qtml | 2 | 2 | 2 / 38 | 2 | 2 | 0 | 0 | 0 |
| excluded:eh-funclet | 0 | 0 | 0 / 3556 | 0 | 0 | 0 | 0 | 0 |
| excluded:mis-split | 0 | 0 | 0 / 660 | 0 | 0 | 0 | 0 | 0 |
| excluded:protection | 0 | 0 | 0 / 741 | 0 | 0 | 0 | 0 | 0 |
| excluded:thirdparty:libpng-1.0.12+zlib-1.1.3 | 0 | 0 | 0 / 197 | 0 | 0 | 0 | 0 | 0 |
| **TOTAL non-excluded** | 6662 | 6256 | 6758 / 15479 | 7462 | 47044 | 1139 | 696 | 1 |
| **TOTAL excluded classes** | 326 | 324 | 326 / 6683 | 326 | 326 | 0 | 0 | 0 |

The only Rust hook whose original body ran is `CTimer::Update` (0x561B10, 11 of 15 points): its "Rust"
hook is sa-hook's passthrough to the original (`game::timer_update_hook`), as designed. No engine port
let its original body run. 443 ports were never entered by the two recordings (their parity rests on
L2/L3 runs of other recordings or on none).

Timing: the first set of these runs (master 557a49f, 905 ports, DXVK with vsync, ~110 s per replay)
also passed all four checks. Its plain grove-walk hits differ from this run in 14 points, all in
`Ps::psSelectDevice` (0x746190) and RW video-mode code it calls: the DXVK setting changes the mode list
the game enumerates at boot; pause-restart hits are identical.

### Validation of the excluded points (2026-11-10)

Each excluded point armed on top of every armed point (grove walk, ports original, DXVK with vsync),
trace compared with the plain replay (`target/e2e/lane-5/inf10/validate/`):

| point | function | alone on top of all points | point executed | reader executed (corpus) |
|---|---|---|---|---|
| 0x55C410 | CStats::SetNewRecordStat (entry) | **STOPS**: trace 256 frames, last status frame 201 | no | 0x4017B4 yes |
| 0x587B84 | CRadar::DrawEntityBlip (block) | **STOPS**: trace 256 frames, last status frame 302 | no | 0x4038C8 yes |
| 0x407AD0 | CStreaming::AreAnimsUsedByRequestedModels | IDENTICAL 5,200 | yes | no |
| 0x410860 | CColStore::LoadCollision | IDENTICAL 5,200 | yes | no |
| 0x4A0D70 | Fx_c::TriggerWaterHydrant | IDENTICAL 5,200 | no | 0x401C02 yes |
| 0x4D3940 | CAnimManager::GetAnimationBlock | IDENTICAL 5,200 | yes | no |
| 0x4F748A | CAEVehicleAudioEntity::AddAudioEvent (block) | IDENTICAL 5,200 | yes | no |
| 0x50AC20 | CCamera::Fade | IDENTICAL 5,200 | yes | no |
| 0x52A00D | CCamera::CamControl (block) | IDENTICAL 5,200 | no | no |
| 0x563470 | CWorld::ClearScanCodes | IDENTICAL 5,200 | yes | 0x4013A2 yes |
| 0x58FD50 | CHudColours::GetIntColour | IDENTICAL 5,200 | yes | no |
| 0x7034B0 | CPostEffects::FilterFX_StoreAndSetDayNightBalance | IDENTICAL 5,200 | no | 0x4084D2 yes |

Also: every point armed **including** all 12 stops both recordings at frame 256; every point armed
except 0x55C410 stops the grove walk at 256 too (the second culprit, 0x587B84, explains the
2026-11-05 rerun that still stopped without 0x55C410). "Reader executed" comes from a run arming only
the 246 protection reader instructions (113 executed in the grove walk, 101 in pause-restart; both
traces IDENTICAL to the plain replays). The ten non-stopping points stay excluded on their static
evidence: their readers compute pointers from the bytes, so a path that runs the reader while the
int3 is still in place (before the point's first execution) would go wrong; in the corpus that does
not happen or does not reach hashed state. `cov-bisect.py` was not needed: the static list was
checked point by point instead (12 probes instead of a bisection over 169,689 points).

### Finding: two port hooks overwrite bytes the protection reads (not a coverage issue)

`cov-data-reads.py --hooks` with the real patch lengths (stub retarget = bytes 1–4, inline = the
relocated instructions) leaves two hook entries whose patch overlaps a protection read:
- **0x4C4570 `CAtomicModelInfo::SetupVehicleUpgradeFlags`** (inline, 6 bytes `90 E9 B8 DC F3 FF`):
  `CGameLogic::SkipCanBeActivated` (0x4415C0, body in `.HOODLUM`) runs `pushfd; mov eax,[0x15604E8];
  push ecx; mov ecx,[0x4C4572]; ror eax,cl; pop ecx; mov eax,[eax]; popfd` at 0x156487A, and
  `CRunningScript::ScriptTaskPickUpObject` reads `xor esi,[0x4C4572]` at 0x156B570. Original: key
  0xA00096A8 (same at run time, `peek`), cl = 0xB8 → ror 24 → reads 0x0096A8A0. With the hook
  installed (`peek 0x4C4570` = `E9 6B 9D 1A 02 90` in one run of the 557a49f tree), cl = 0x9D → ror 29 → 0x0004B545, which
  `peek` reports as not readable. The block at 0x156487A and the reader at 0x1564881 do execute in
  both recordings (first-hit points), yet neither run faults and a replay with the ports not installed
  (`L3_HOOK_PORTS=` in l3.sh) is IDENTICAL to one with them installed (5,200 and 3,700 frames). Why
  the read does not fault is not resolved; the rotate count depends on where VirtualAlloc puts the
  hook slots, so the effect can change from run to run. Needs a decision in sa-hook (e.g. hook the
  chunk at 0x40222E instead of the entry, or keep 0x4C4572–0x4C4575 intact).
- **0x410860 `CColStore::LoadCollision`** (stub retarget): the chunk at 0x401E3E (`adc ecx,[0x410860];
  mov ecx,[ecx]`, reached from 0x69D981) gets 0x03C6700E instead of 0x00B744BC with the slot of one
  run (557a49f tree); the chunk did not execute in the corpus.
The other two `--hooks` hits (0x7316A0, 0x7320B0) read the byte after the 5-byte patch: no overlap.

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
- Excluded points are counted through their proxy (the stub's jump target). With a port on Rust the
  target does not run, so 0x407AD0 and 0x410860 (both ports) count as not entered in `--rust ports`
  runs even when their Rust code ran.
- The static scans see absolute-address reads only; register-based reads of code (checksum loops) are
  covered only by the dynamic check (traces identical), i.e. only on the corpus' paths.
