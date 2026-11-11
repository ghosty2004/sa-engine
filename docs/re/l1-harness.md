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
  stores) and `fstp tbyte` (all 80 bits). ECX is captured too (Unicorn: the register after the stub,
  which does not touch it; native: `thunk.S` stores it over the input `nstack` field, context offset 12). The native backend's thunk (`native/thunk.S`) uses the same
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

## Preserved registers (VER-2)

Some original callers keep a value in ECX or EDX across a call to a function they know does not write
it (whole-program optimisation; e.g. 0x5B4177 tests DL after calling `SetBaseModelInfoFlags` 0x5B3AD0).
A Rust hook entry is a C-convention function for which ECX/EDX are caller-saved, so it must restore
them explicitly:

- `#[hook(..., preserve = "edx")]` (or `"ecx"`, `"ecx,edx"`) makes the registered entry a naked wrapper:
  `push edx; push [esp+4(s+n)] ×n; call shim; add esp, 4n (cdecl); pop edx; ret / ret 4n`. EAX and
  ST0 pass through. `HookReg::preserve` records the registers.
- `Spec::preserve` declares that the original preserves them for its callers. The runner loads each
  such register that is not an argument with a per-case value (FNV of the inputs) and fails the case
  (`preserve-edx`) when the original returns another value, or when the port's hook registration does
  not declare it. `tests/l1.rs::preserved_registers_match_the_hook_registrations` checks that every
  preserving hook has a spec comparing the registers.
- Which hooks need it is decided by sa-hook's test `callers::callers_reading_ecx_edx_after_a_hook_are_declared`
  (`crates/sa-hook/src/callers.rs`): every direct call into every hook in game `.text` and `.HOODLUM`,
  every path after the call, byte-wise register tracking; exceptions documented there. Result on
  2026-11-07: 796 hooks, 569 with direct callers, 5,575 call sites; ECX/EDX read after the call at
  0x5B4179, 0x5B3B2B, 0x5B3C3B (EDX, callee 0x5B3AD0) and 0x71DC62 (ECX and EDX, callee 0x71DB30).
  Calls through pointers and virtual calls are not covered.

## Specs run the hooked entries (VER-2)

A spec's Rust side is the function `#[hook]` installs (PLAN §5), run on the host
(`sa_oracle::hosted`): globals the original reads through patched operands are copied into sa-core's /
sa-stream's Rust-owned globals (under `sa_oracle::host_globals()`, one lock for all specs), unported
callees get stand-ins registered with `sa_core::orig::host` (the allocator stub's queue and argument
record, `MapVirtualKeyA`'s recorded arguments and result, the original `CDummyPool::New`), and the
32-bit `CPool` header is bridged to the 64-bit host layout by running the entry on two host views and
splicing them (module docs of `hosted.rs`). Slot sizes come from the entries' types and are asserted
equal to the strides encoded in the original. Exception: `CPool<TxdDef>` (`TxdDef` holds a pointer, so
its host size is 16): those two specs run the same generic code on a 0x0C-byte slot type.

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
- FPATAN, FSIN, FCOS and FSINCOS return 64-bit significands and are not rounded to 24 bits (PC does not
  apply to them on the CPU: e.g. `fpatan` under CW 0x7F returns `c90fdaa22168c235`); see the X87-2
  section below.
- No unsafe, no host-dependent float behaviour; `sa-math` builds for `wasm32-unknown-unknown`.

### FYL2X and FPTAN (X87-MODELS, 2026-11-11)

Both return a 64-bit significand whatever the precision control. `X87` holds such "wide" values as an
exact f64 pair (`hi` = the value rounded to 53 bits, `lo` = the rest); every op that meets a wide
operand is computed exactly in integers (`sa_math::x87_wide::Big`) and rounded once, as the CPU does
(narrow values keep the old f64 path, unchanged). `to_ext`/`from_ext` carry all 64 bits; `fldlg2`,
`fldln2`, `fldl2e`, `fldl2t`, `fldpi` are the CPU's round-to-nearest constants. The `Fpu` stack machine
(`sa_world::collision::fpu`) has `fldlg2`/`fldln2`/`fldl2e`/`fldpi`, `fyl2x`, `fptan`.

Models (the CPU's microcode is not reproducible exactly; these are the best simple models found, by
fitting exploration data with mpmath):
- **FYL2X** `y * log2(x)`: `log2(x)` in double-double (~104 bits), **truncated toward zero to 67
  significant bits**, times `y` exactly, rounded to nearest at 64 bits. (Correct rounding of the exact
  `y*log2 x` matches only 93%; truncation to 66/68 bits 94%/96%.) Special operands as the SDM table:
  `x<0` → indefinite, `x=±0` → `-sign(y)·inf` (`y=0`: indefinite), `x=+inf` → `sign(y)·inf` (`y=0`:
  indefinite), `x=1` → `sign(y)·0` (`y=±inf`: indefinite); NaNs as the arithmetic ops.
- **FPTAN**: the 66-bit-pi quadrant reduction of FSIN/FCOS (`r = x - k·P/2` exact), `tan(r)` or
  `-1/tan(r)` (k odd) in double-double, rounded to nearest at 64 bits; 1.0 is pushed. `|x| >= 2^63`:
  nothing happens (C2). **A NaN or infinite operand pushes the NaN result, not 1.0** (measured; both
  ST0 and ST1 are the NaN / the indefinite).

Measured on the CPU (Intel i7-14700F, native backend; `fyl2x_fptan_model_vs_cpu_1m`, 10^6 cases each
plus ~900 edge values, and the 100k version in the normal test run):

| instruction / use | cases | 80-bit ST0 exact | stored f32 exact | worst |
|---|---|---|---|---|
| `fldlg2; fld x; fyl2x` (AudioLog10, x ≥ 1e-5 mostly) | 1,000,000 | 994,423 (99.44%) | 100% | 1 ulp of 64 |
| `fyl2x` y·log2 x, random y, x | 1,000,000 | 999,608 (99.96%) | 100% | 1 ulp of 64 |
| `fptan` | 1,000,000 | 958,654 (95.87%) | 100% | 1 ulp of 64 |
| `fptan` pushed value | 1,000,000 | 100% | 100% | — |
| AudioLog10(x) · 20.0 / 10.0 at PC=24 (what every caller does) | 1,000,000 | 100% | 100% | — |
| PC=53 and PC=64 (same models) | 40,000 each | 99.2% / 95.8% | — | 1 ulp of 64 |

Special operands (NaN, ±inf, ±0, subnormals, 1, |x| ≥ 2^63, the AudioLog10 threshold): 0 differences.
The inexact cases are where the exact value lies close to a 64-bit rounding boundary (FPTAN: within
~0.1 ulp; FYL2X: mostly for x near 1, where the CPU's log2 has fewer correct bits than the 67-bit model
assumes). A difference of 1 ulp in the 64th bit changes a later PC=24 result only when the exact
product lies within ~2^-40 relative of a 24-bit tie: never seen (0 in 10^6 composite cases, and the
stored f32 matched in all 4·10^6 cases).

L1 specs (`specs_audio.rs`, CPU-decided): `AudioLog10*c` runs the original AudioLog10 through a
caller-shaped wrapper patched into its own `nop` padding (0x4D9E71: `push [esp+4]; call 0x4D9E50; pop
ecx; fmul [esp+8]; ret`) and compares the full ST0 of the product: 1,002,176 cases, 0 mismatches.
`AudioLog10` compares the stored f32: 1,001,088 cases, 0 mismatches (ST0 beyond f32 differs in 30,492
— this generator puts a fifth of its inputs within 1/64 of 1.0, the model's weakest region).

### FPATAN, FSIN, FCOS, FSINCOS (X87-2, 2026-11-11)

Fitted against the CPU (native backend, Intel i7-14700F; P-core and E-core results are identical,
checked with `taskset` on CPUs 2 and 22) from dumps of the 80-bit results (`tests/x87_trig.rs`:
`X87_TRIG_DUMP` / `X87_TRIG_IN` write them for offline fitting with mpmath). The error of the CPU's
result against the exact value is not noise: it is piecewise constant in table intervals, which gives
the microcode's structure away.

- **FPATAN** (`X87::fpatan`): `q = min(|y|,|x|) / max(|y|,|x|)` exact, `k = round(32 q)`.
  `k < 2`: `t = trunc67(q)`, `a = atan(t)` (for `t < 2^-40` exactly `t`: the `-t^3/3` correction decides
  64-bit ties for larger `t` only, measured over 2^-5 ... 2^-121). `k >= 2`: `t = trunc67((q - k/32) /
  (1 + q k/32))`, `a = A[k] + trunc67(atan t)`, `A[k] = atan(k/32)` rounded to nearest at 67 bits (the
  CPU's error is exactly these roundings: e.g. -0.051, +0.050, -0.087 x 2^-66 for k = 10, 11, 12).
  `|y| <= |x|, x > 0`: round-to-nearest-64 of `a`; otherwise `a` is truncated to 67 bits and
  `P/2 - a`, `P/2 + a` or `P - a` (`P` = pi rounded to 67 bits) is rounded to 64 bits; sign of y.
  Special operands as the SDM (pi, pi/2, pi/4, 3pi/4 are the round-to-nearest 64-bit constants).
- **FSIN/FCOS** (`X87::fsin`, `X87::fcos`): reduction by the 66-bit pi as before (exact remainder),
  `|r| < 0.25` direct; else a table at the centre `c` of r's quarter-binade (`0.28125, 0.34375, ...,
  0.9375`) with `sin c` and `cos c` rounded to nearest at 67 bits: `S cos t + C sin t` / `C cos t - S sin
  t`, evaluated to ~100 bits, rounded to 64. The CPU's internal rounding of the polynomial terms
  (residual noise growing with |t|) is not modelled. **FSINCOS** (`X87::fsincos`) = (FSIN, FCOS);
  the CPU's FSINCOS values are not always identical to separate FSIN/FCOS (the measured rates differ
  slightly), the model uses the same values.

Measured (`cargo test --release -p sa-oracle --test x87_trig -- --include-ignored`; the 10^6 tests are
`fpatan_model_vs_cpu_1m`, `trig_model_vs_cpu_1m`; f64 operands: edge values, the CMaths angle steps and
multiples of pi/4 with neighbours, uniform game ranges, random bit patterns; results whose exponent is
outside the f64 range minus 64 are skipped: an `X87` pair cannot hold them):

| instruction | cases | 80-bit ST0 exact | stored f32 exact | before (53-bit f64 model) |
|---|---|---|---|---|
| FPATAN | 991,340 | 991,335 (99.9995%) | 100% | ~97.5% |
| FSIN | 996,881 | 992,116 (99.52%) | 100% | ~98.1% |
| FCOS | 1,000,000 | 994,966 (99.50%) | 100% | ~98.1% |
| FSINCOS (cos) | 1,000,000 | 995,317 (99.53%) | 100% | — |
| FSINCOS (sin) | 996,881 | 992,355 (99.55%) | 100% | — |

Every difference is 1 ulp of the 64-bit significand. The 5 FPATAN ones are table cases where
`atan(t)` lies within ~0.02 units of a 67-bit truncation boundary: the CPU's polynomial error (not
modelled) decides them. Special operands (NaN, ±inf, ±0, |x| >= 2^63, |y| = |x|): 0 differences.

L1 specs on the CPU with the models (200k generated + edges each, `--backend native`): GetATanOfXY,
GetRadianAngleBetweenPoints, GetNodeHeadingFromVector, GetAngleBetweenPoints, CMatrix
ConvertTo/FromEulerAngles, SetRotate{X,Y,Z}{,Only}, SetRotate, Rotate{X,Y,Z}, Rotate, BoneNode_c::EulerToQuat,
CCurves::CalcCorrectedDist, CameraProcessVector{Track,Move}Linear (+ the new `...Eased` variants, which run
the callees 0x50D350 / 0x50D430 on the eased path: before, 29,091 of 1.06M differed there), RwMatrixRotate:
0 mismatches each. RwMatrixRotate left `KNOWN_CPU_MISMATCHES` (now empty).

Hooks returning `X87` give back all 80 bits: `IntoAbi for X87` also stores the extended value in
`sa_core::abi::ST0_EXT`, and the `#[hook]` entry wrapper (generated for every `X87`-returning hook;
combined with `preserve`, `footprint` and `entry_eax` wrappers) does `fstp st(0); fld tbyte [ST0_EXT]`
after the shim. sa-hook's hand-written `GetATanOfXY` entry does the same (`game::get_atan_of_xy_entry`).
`orig!` declarations still read an ST0 result as f64 (rounded to 53 bits).

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

- **FPATAN / FSIN / FCOS raw 64-bit results** (X87-2): the models differ from the CPU in 1 ulp of the
  64-bit significand for 5 in 10^6 FPATAN and ~0.5% of FSIN/FCOS operands; after rounding to f32 0 differences
  in 10^6 each. A function returning or keeping the raw result (GetATanOfXY's raw path, GetHeading,
  GetRoll, FindPlayerHeading) can show such a difference in its full ST0 (GetATanOfXY: 1 in 10^6 L1 cases).
  Ledger: pending (accepted only for uses that round). The former `GetATanOfXY` ledger exception in
  sa-hook's L2 (`l2win::is_ledger`) is gone: the hook returns 80 bits now.
- **FYL2X / FPTAN raw 64-bit results** (`X87::fyl2x`, `X87::fptan`): a function that returns or keeps
  the raw result differs from the CPU in the last (64th) significand bit for 0.56% of AudioLog10 inputs
  (up to ~3% when inputs cluster within 1/64 of 1.0), 0.04% of general FYL2X operands and 4.1% of FPTAN
  operands; never more than 1 ulp. After rounding to f32, or after the PC=24 multiply every AudioLog10
  caller does, 0 differences in 10^6 cases each. Ledger: pending (accepted only for uses that round).
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
