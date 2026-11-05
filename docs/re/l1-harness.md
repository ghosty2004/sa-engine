# L1 harness (`sa-oracle`, INF-7) and x87 fidelity

The L1 oracle runs original functions of `gta_sa.exe` 1.0 US on Linux, without Wine, under Unicorn, and
compares them bit-exactly with their Rust ports (`crates/sa-math`). Where Unicorn is measured to be
unfaithful, a second backend runs the same original bytes on the real CPU (a tiny i686 program under Wine).
Everything below was measured on this machine on 2026-11-05; commands are given so it can be re-measured.

## Commands

```sh
cargo test -p sa-oracle -p sa-math                       # self-tests (exe + Wine optional, see below)
cargo run -p sa-oracle --release -- list                 # specs and their routes
cargo run -p sa-oracle --release -- l1 <fn|all> --cases 1000000 [--seed S] [--backend auto|unicorn|native]
cargo run -p sa-oracle --release -- fidelity --cases 20000 --fn-cases 200000
cargo run -p sa-oracle --release --features mutants -- mutants --cases 1000000
```

`--cw` / `--mxcsr` set the FPU state (default: the game's, CW 0x007F and MXCSR 0x9FA0, measured in-game by
INF-3). The exe is found via `$SA_GTA_EXE`, `<workspace>/re/gta/1.0-US/gta_sa.exe` or
`~/sa-engine/re/...`; the Wine prefix via `$SA_WINEPREFIX` or `~/sa-engine/wine/prefix`. Tests skip (with a
note) what they cannot run.

## How a call runs

| range | use |
|---|---|
| 0x10000 | exit stub (`native/exit_stub.S`), abort address, result block 0x11000, x87 env 0x11100 |
| 0x20000 | import traps: IAT slot *i* is patched to 0x20000 + 16·*i* (never allowed, reported by name) |
| 0x100000–0x200000 | stack (ESP before the arguments: 0x1F0000) |
| 0x400000–0x1577000 | the PE image at its preferred base (all 11 sections, IAT patched) |
| 0x10000000 | scratch memory, 64 bytes per call (pointer arguments, outputs) |
| 0x10100000 | probe code (fidelity) |

- **Calling conventions** (`call::Call::new`): cdecl, stdcall, thiscall (`this` in ECX), fastcall (first
  two integer/pointer args in ECX/EDX). Arguments: `I32`, `F32`, `F64` (two stack slots), `Ptr`. The
  ESP delta after return is checked against the convention (a wrong convention is reported).
- **Return capture.** The function returns into an exit stub that stores EAX, EDX, the x87 status and
  full tag word, MXCSR, and, if the x87 stack is not empty, ST0 with `fst dword` (what a caller's `fstp`
  stores) and `fstp tbyte` (all 80 bits). The native backend's thunk (`native/thunk.S`) uses the same
  instructions, so both backends observe the result the same way. ST0 is also read through Unicorn's
  register API (`Emu::last_st0_reg`); a test checks both agree.
- **Guard rails.** A block hook allows only blocks inside the function's extent, the extents of its
  allow-listed callees (extent = [start, next catalogued start) from `catalog/procedures.csv`) and the exit
  stub. Anything else (a call into the game or the CRT, an import, a bad return) stops the call with
  `Disallowed { target, from, what }`, `what` naming the catalogue function or `DLL!import`. Further stops:
  instruction budget (`Budget`), writes outside stack/scratch (`GlobalWrite`, image restored afterwards),
  reads of writable image sections (`GlobalRead`: mutable global state, not an L1 function), unmapped
  accesses (`Fault`), interrupts (`Interrupt`).
- **Static scan** (`scan`, iced-x86): each spec's code is swept for x87 instructions measured as
  unfaithful under Unicorn and for SSE instructions. A test (`routes_match_hazards`) enforces that a
  function using an unfaithful instruction is routed to L2.

## Property runner

- Case *i* of seed *s*: edge case *i* if *i* < #edges, else generated from `Rng::for_case(s, i)`
  (SplitMix64), so any case is reproducible alone.
- Edge values per float argument (59): ±0, NaN payloads (quiet and signalling, both signs), min/max
  subnormals, ±min normal, ±max, ±inf, 1e±30, 1e10, 2^24, 2^33, ±π, ±2π, ±π/2, 3π/2, ±25, ±180, ±360, 1e-4
  and the ±1-ulp neighbours of the angle boundaries. Full cartesian product for up to 3 arguments
  (205,379 tuples), 250,000 combinations for 4.
- Generated floats: 25 % random bit patterns, 35 % uniform in realistic ranges (±1 … ±1e6), 10 % tiny
  magnitudes down into the subnormals, 15 % edge values, 15 % edge values ±4 ulp; 20 % of tuples get
  correlated arguments (equal, negated, a few ulp apart) to hit `x == y` / `|x| == |y|` branches.
- Comparison: bit-exact. St0 functions: the stored f32 always, all 80 bits when the spec says
  `st0_exact`; x87 stack depth; AL for bool returns; the 64 scratch bytes (output pointers, aliasing);
  convention (ESP). A port can also say "the original hangs" (checked against the budget).
- Mismatches are shrunk greedily (one float at a time towards 0, ±1, fewer significand bits, smaller
  exponent, while still mismatching), printed with the minimal reproducer and written to
  `fixtures/l1/<fn>/s<seed>-c<case>.txt` (gitignored; at most 8 per run).

## Measured x87 fidelity of Unicorn (2.1.5 = QEMU 5.0.1)

Reference: the real CPU running the same bytes under Wine (`native`), same inputs, same CW/MXCSR.
`sa-oracle fidelity`: 21 single-instruction probes × f32/f64 operands × CW 0x007F/0x027F/0x037F,
20,000 random cases per row (MXCSR 0x9FA0).

| finding | evidence | consequence |
|---|---|---|
| **MXCSR.FZ leaks into the x87.** QEMU 5.0 `update_mxcsr_status` sets `flush_to_zero` on the x87 `fp_status`. With the game's MXCSR every x87 store of an f32/f64-subnormal value becomes ±0 (CPU: subnormal). | without the workaround: `fld/fst f32` 755, `fmul f32` 717, `fdiv f32` 618 st0-f32 differences per 20,000 (cw 0x7F); with it: 0 | Workaround (default): the emulated MXCSR has FZ cleared, the reported MXCSR gets it back. Exact for code without SSE; the runner enables it only when the scan finds no SSE instruction. |
| **`fld m32/m64` of a signalling NaN is not quieted** (CPU: quiet bit set, payload kept). Also wrong NaN choice when an SNaN memory operand meets a QNaN. | every SNaN-argument row differs (e.g. `fld/fst f32` 250 cases); NaN matrix: 530 of 4,860 ST0 values differ | Input-class routing: a case with a signalling-NaN float argument is decided by the CPU (native backend); without Wine it is reported as unverified. |
| **Transcendentals use host doubles**: FPATAN, FSIN, FCOS, FSINCOS, FPTAN, F2XM1, FYL2X, FYL2XP1 (53-bit results; CPU: 64-bit). FPREM/FPREM1 differ in value and C0/C2/C3; FSCALE is rounded to the precision control (CPU: not) and mishandles NaN/inf; FXTRACT is wrong for NaN/inf/subnormals. | `fpatan f32` st0-ext 18,231/20,000, st0-f32 3; `fsin f32` st0-f32 587; `fscale f64 cw7F` st0-ext 9,878; `fxtract f32` 757 | Per-function route **L2** for any function containing one of them (`fidelity::UNFAITHFUL`, enforced by the scan). |
| C1 is never set by arithmetic; exception flags (IE/DE/ZE/OE/UE/PE, ES) are not raised. | C1 and exc-flags columns of every arithmetic row | Not observable by game code (MSVC code only tests C0/C2/C3 after a compare; all exceptions are masked). Ignored by the runner, reported as "flags only". |
| FSQRT leaves C2 set (CPU: cleared; architecturally undefined). | `fsqrt` C0/C2/C3 ≈ 9,500/20,000 | Not observable (no compare-free branch on it). |
| Exact otherwise: FADD/FSUB/FMUL/FDIV/FSQRT/FCOM/FRNDINT/FISTP results and condition codes under PC 24/53/64 for f32 and f64 operands, f32/f64 stores, precision control. | those rows show only C1/exc-flags (and SNaN rows) | L1 under Unicorn is authoritative for functions built from these. |

Run `sa-oracle fidelity` for the full table (the rows above are its output, condensed).

## The `sa_math::x87` model (what the ports use)

`X87` is one register value under PC=24 held in an f64: 24-bit significand, unbounded exponent (the f64
range covers everything reachable from f32 inputs). Rules, all checked against the CPU:

- FADD/FSUB/FMUL/FDIV/FSQRT round the exact result once to 24 bits. Operands that are not 24-bit (an
  FPATAN result) need the exact error of the f64 operation (TwoSum, `fma`) to break ties; without it,
  `GetATanOfXY(1, 2^24)` = π/2 − atan(2^-24) lands exactly on an f32 tie in f64 and rounds the wrong way
  (found by the runner: 7 mismatches in 20,000 before, 0 after).
- NaNs (NaN matrix, 15 instruction forms × 324 operand pairs, `x87_model_matches_the_cpu_on_special_values`,
  0 mismatches): a loaded SNaN is quieted; two QNaNs → larger significand, **ties → the positive one**; an
  **SNaN memory operand loses to a QNaN register** (so register and `m32` forms differ: the API has
  `fadd`/`fadd_m` etc., and the ports follow the original's operand forms); invalid operations → the
  indefinite (0xFFC00000 as f32). FCHS flips a NaN's sign.
- FPATAN is computed in f64 (`libm::atan2`, deterministic on every target) and not rounded to 24 bits
  (PC does not apply to it on the CPU: e.g. `fpatan` under CW 0x7F returns `c90fdaa22168c235`).
- No unsafe, no host-dependent float behaviour; `sa-math` builds for `wasm32-unknown-unknown`.

## Self-test: six ported functions (source: the 1.0 US bytes; gta-reversed for intent)

| function | addr | conv | route | notes |
|---|---|---|---|---|
| `CGeneral::LimitAngle` | 0x53CB00 | cdecl f32→ST0 | L1 | hangs for ±inf and \|a\| ≥ 2^33 (loop step rounds back): port returns `None` there; generated inputs limited to \|a\| ≤ 2^24 (≤ 46,603 iterations), hang edge cases included |
| `CGeneral::LimitRadianAngle` | 0x53CB50 | cdecl f32→ST0 | L1 | |
| `CGeneral::GetATanOfXY` | 0x53CC70 | cdecl f32,f32→ST0 | **L2** (FPATAN) | computes `(-1/y)*x` where gta-reversed writes `-x/y`; returns 2π for (x>0, y=0); one path returns the raw FPATAN value (64-bit ST0, see divergences) |
| `CGeneral::GetRadianAngleBetweenPoints` | 0x53CBE0 | cdecl 4×f32→ST0 | **L2** (FPATAN) | `PI - t` is computed as `(C - t) + C` etc.; every path ends with a PC=24 op, so ST0 is compared exactly |
| `CGeneral::SolveQuadratic` | 0x53CE30 | cdecl, 2 out-pointers, bool in AL | L1 | `(1/a)*0.5`; stores `*x2` before `*x1` (aliasing case generated); NaN discriminant takes the root path |
| `CVector::Normalise` | 0x59C910 | thiscall | L1 | s == 0 sets only x = 1.0 |

### Results, 10^6 generated cases + edge cases each (seed 0x5A0001, game FPU state)

| function | backend (auto) | cases | mismatches | other | Unicorn vs CPU, all cases (`--backend native`) |
|---|---|---|---|---|---|
| LimitAngle | Unicorn | 1,000,057 | 0 | 10,871 hangs, port agrees; 12,486 SNaN cases decided by the CPU | 12,486 differ, all SNaN-input cases |
| LimitRadianAngle | Unicorn | 1,000,058 | 0 | 12,486 SNaN cases by the CPU | 12,486, all SNaN-input |
| GetATanOfXY | CPU | 1,003,364 | 0 | ST0 beyond f32 differs in 78,162 (raw FPATAN path, allowed: f32 compared) | 78,771 (609 visible in the f32) |
| GetRadianAngleBetweenPoints | CPU | 1,250,000 | 0 | ST0 compared exactly | 1,360 (all in the f32) |
| SolveQuadratic | Unicorn | 1,195,112 | 0 | 54,010 SNaN cases by the CPU | 9,268, all SNaN-input |
| CVector::Normalise | Unicorn | 1,195,112 | 0 | 55,767 SNaN cases by the CPU | 0 |

So for the four L1-routed functions Unicorn agrees with the CPU on every observable field outside the
signalling-NaN input class (which is routed), and for the two FPATAN functions it does not (which is why
they are L2). C1/exception-flag-only differences are not counted.

L2-routed functions were decided by the real CPU (the native backend is the interim authority until the
in-process L2 of INF-8 exists); their Unicorn results are reported but not counted.

## Mutation smoke test (INF-12 seed)

Three faults injected into `SolveQuadratic` (feature `mutants`), 10^6 generated cases each:

| mutant | result | mismatches | minimal reproducer (a, b, c, alias) |
|---|---|---|---|
| F32Discriminant (naive f32 discriminant) | caught | 114,634 (al 4,479, memory 110,155) | (0, 1e-45, 0, 0) |
| ZeroDiscriminantIsNoRoot (`<= 0`) | caught | 50,891 (al) | (0, 0, 0, 1) |
| SwappedRoots | caught | 752,475 (memory) | (0, 1, 0, 0) |
| GtaReversedDenominator (`1/(2a)`) | equivalent, 0 mismatches (scaling by 0.5 is exact) | 0 | — |

The equivalent mutant was first meant as a fault; the runner showed it is not one.

## Divergences and limits

- **`GetATanOfXY` raw FPATAN path** (x > 0, y > 0, |x| ≥ |y|): ST0 holds the CPU's 64-bit FPATAN result;
  the port has 53 bits. The stored f32 matches in every case run; a caller that keeps using ST0 without
  storing it could see a difference. Ledger: pending, until an x87-exact atan exists or the callers are
  shown to store first.
- **`LimitAngle` hang**: the original never returns for ±inf and |a| ≥ 2^33; `limit_angle` panics there
  instead of spinning. Ledger: original UB / not reproducible.
- FPATAN in the ports is f64: a CPU/port difference is possible when the exact value lies within about
  2^-29 ulp of an f32 rounding boundary *and* the f64 value is on the other side. Not observed in
  2.25·10^6 FPATAN cases.
- Memory: one Unicorn instance per worker thread (~25 MB; threads capped at 16, tests use 4), cases in
  blocks of 250,000; `l1 <fn> --cases 1000000` peaks at ~750 MB RSS, `fidelity --cases 20000` at ~1.3 GB
  (probe rows are run in chunks of ≤ 200k calls; all at once it was 2.7 GB and, together with other
  processes, triggered the OOM killer once). The CLI prints its peak RSS. Run it under
  `systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`.
- The native backend has no guard rails; only calls that returned under Unicorn are sent to it.
- Extents come from the catalogue (next start); padding counts as inside. FS-relative accesses (SEH
  prologues) fault (FS base 0) — not needed by leaf math functions.
- Unicorn is GPL-2.0; `sa-oracle` is a host-only test tool, never shipped, and compiles to nothing for
  Windows targets.
