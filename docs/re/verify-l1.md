# Verifier report: INF-7 L1 harness and the six MATH functions marked L1 (2026-11-05)

Adversarial review of `crates/sa-oracle` (Unicorn + native backends, routing, runner, generators,
comparison) and of the six `crates/sa-math` functions in `catalog/state.csv` state L1. I formed this view
from the code, the oracle and the disassembly before reading `docs/re/l1-harness.md`. The comparison
with that doc is at the end.

Probes: `crates/sa-oracle/tests/verifier_l1.rs` (new file, nothing else changed). Run:

```
systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 \
  cargo test --release -p sa-oracle --features mutants --test verifier_l1 -- --test-threads=1 --nocapture
```

Knobs: `VERIFY_CASES` (Unicorn vs CPU, default 200k), `VERIFY_COV_CASES`, `VERIFY_MUTANT_CASES`,
`VERIFY_ATAN_CASES`, `VERIFY_LA_PER_OCTAVE`. `--test-threads=1` is required: native runs share
`target/sa-oracle-native/probes.bin`.

Host: Intel i7-14700F, Wine prefix `~/sa-engine/wine/prefix`, gta_sa.exe 1.0 US.

## Findings

Severity: **B** = breaks a verified claim, **W** = weakens it, **I** = informational.

### V-1 (W) GetATanOfXY: callers consume the 64-bit ST0, and the f32 API gives a different result

`GetATanOfXY` is in state L1, but its spec compares only the f32 (`st0_exact: false`), and the ledger entry
for it is still *pending*. PLAN §5 defines L1 as bit-exact, so the state is ahead of the evidence.

Call-site scan (all `call 0x53cc70` in game `.text`, first FPU use of the returned ST0): 61 store it as
f32 first, but **35 sites use ST0 directly in PC=24 arithmetic** (14 `fadd`, 11 `fsub`, 8 `fsubr`, 2 `fmul`),
for example 0x42acfe `fadd [PI]` (CCarCtrl::SteerAIHeliFlyingAwayFromPlayer) and 0x4201b8
`fsubr [esp+28]`. A further 7 sites start with `fld`, which keeps ST0 live in ST1. Such an op rounds the
*exact* sum of the 64-bit value, so equality of the stored f32 does not show that callers behave the same.

Probe `verifier_atan_st0_consumed_by_callers` runs `GetATanOfXY(x, y) + PI` (the 0x42acfe pattern) on
the CPU for 200,000 inputs on the raw-FPATAN path (x > 0, y > 0, x ≥ y):
- `get_atan_of_xy_x87(..).fadd_m(PI)` (53-bit X87 value): **0 differences**;
- `get_atan_of_xy(..)` (the f32 API) then `+ PI`: **4,579 differences (2.3 %)**.

Fix: callers that keep ST0 must be ported with `get_atan_of_xy_x87`, never with the f32 function.
Document this on the f32 function, or make the f32 function private to the crate. Add a caller-pattern spec
(for example "atan + PI") to L1 with `st0_exact` on the composite. Until then, either set the state to
`implemented` with the ledger entry open, or rewrite the ledger entry with this evidence. The evidence now
narrows the risk to rare near-ties between 53 and 64 bits.

### V-2 (W) GetATanOfXY / GetRadianAngleBetweenPoints: the authority is this host's CPU, not Unicorn

The two FPATAN functions are L1 on the native backend, as decided in the ROADMAP on 2026-11-05. PLAN §5
still defines L1 as "under Unicorn", and the state rows do not say "interim". FPATAN's 64-bit result is
implementation-specific: Intel and AMD x87 microcode are not guaranteed to agree in the last bits. The
oracle is therefore "gta_sa.exe on an i7-14700F", and players' machines may differ. This matters for ST0
bits (V-1) and for f32 near-ties.

Fix: write "CPU-decided (interim, host CPU model)" in the evidence column, and amend PLAN §5. INF-8 should
record the CPU model in each fixture.

### V-3 (W) LimitAngle: the generator never produces 2^24 < |a| < 2^33; the harness's hang boundary is off by one value

`limit_angle_gen` replaces every finite |a| > 2^24 with a value ≤ 1e5. The edge filter keeps values
above 2^24 only when the port says they hang. So the range where `a - 360` is inexact under PC=24 (the loop
step becomes 256/384/512…) was never compared, and neither was the edge value 2^33, which the filter
drops.

Probe `verifier_limit_angle_large_range`: 462 values (both signs: each power of two from 2^24 to 2^33, ±1
ulp, and 20 log-uniform values per octave) run on the CPU directly, without Unicorn's budget. Result:
**0 mismatches** on the 420 values for which the port returns. The smallest |a| the port calls a hang is
2^33 + 1024. So **2^33 itself returns** (the step below 2^33 is 512, and `2^33 - 360` rounds to
`2^33 - 512`), and the ledger text "|a| ≥ 2^33" should read "|a| > 2^33".

Fix: add a native-only sample of that range to the spec's acceptance (as in this probe), and correct the
ledger wording. The claim itself now stands on stronger evidence.

### V-4 (I) "Hangs" means "Unicorn's budget ran out", not proven non-termination

`compare` accepts `Expect::Hangs` whenever Unicorn exhausts its budget. The CPU is never run on these
cases. Under CW 0x027F, for example, LimitAngle(1e10) returns after about 2.8·10^7 iterations, yet the run
reports "hangs … port agrees" (1,127 such cases in `l1 LimitAngle --cw 0x27f`). Under the game's CW the
port's fixed-point argument is exact, so the claim is unaffected.

Fix: for `Hangs`, also check that the original is at a fixed point. Re-run the loop body once on Unicorn
from the state at budget exhaustion and require ST0 to stay bit-identical. Alternatively, prove the hang
statically in the spec.

### V-5 (I) FPU state: every port hard-codes PC=24; CW 0x027F gives large differences, but no caller runs before device creation

`l1 all --cw 0x27f --cases 100000`: LimitAngle passes (0); LimitRadianAngle 43,480; GetATanOfXY 1,466;
GetRadianAngleBetweenPoints 194,920; SolveQuadratic 62,221; CVector::Normalise 98,419 mismatches. All callers (`catalog/callgraph.csv`: 2 / 58 / 53 / 62 / 1 / 229)
are gameplay, AI, camera or task code on the main thread. The only initialisation caller is
CTimeCycle::Initialise (via Normalise), which runs in `CGame::InitialiseCoreDataAfterRW`, after the D3D
device exists (gta-reversed Game.cpp:579–584). gta-reversed contains no `_controlfp` / `_control87` /
`_fpreset`. So CW 0x007F is the right assumption for these six functions.

Still: (a) INF-3 measured the CW under Wine's d3d9 at the frame boundary only. Real Windows d3d9 sets the
same value, but this was not measured. (b) Any future function reached from a non-main thread or before
`RwInitialise` (loading screens, CdStream, audio) runs under 0x027F and cannot use `sa_math::x87` as it
is today.

Fix: make the per-function FPU state part of the spec, taken from L2 call-site captures. Add a "CW at call"
column to the L2 snapshots.

### V-6 (I) Unicorn's C0/C2/C3 after return differ from the CPU outside the SNaN class

`verifier_unicorn_agrees_with_cpu_outside_snan` (1,000,000 generated cases plus edges):
- LimitAngle (976,700 cases compared) and LimitRadianAngle (987,572): 0 differences in any field except C1
  and the exception flags;
- SolveQuadratic: 0 caller-visible differences, but C0/C2/C3 after return differ in 50,666 of 1,141,102
  cases (example: a = 0, b = 0, c = inf: CPU fpsw 0x4505, Unicorn 0x0404);
- CVector::Normalise: 0 caller-visible differences, but C0/C2/C3 differ in 51,174 of 1,139,345 cases.

The cause is the documented FSQRT/C2 defect, and Unicorn also leaves C3/C0 different after the
NaN-discriminant path. This is not observable by these callers. It is a hazard for functions that branch on
condition codes set *before* an FSQRT (the MSVC `fcom; …; fnstsw` scheduling seen in GetATanOfXY at
0x53ccd1). The static scan does not look for that pattern.

Fix: either extend the scan to flag an `fnstsw` whose last C-code-setting instruction is separated from it
by an FSQRT (or another instruction with measured C-code divergence), or route such functions to the CPU.

### V-7 (I) Comparison window: what is not compared

All of the following are correct for these six functions. They are holes for future specs:
- Scratch: 64 KiB are writable but only 64 bytes are compared. A write to SCRATCH+0x40..0x10000 is silent.
  Stack writes (including the argument area and the caller's frame above it) are allowed and not compared.
- EAX/EDX are compared only for `RetKind::Al` (AL only). SolveQuadratic leaves `EAX[31:8]` equal to the
  upper bytes of the x2 pointer (root path) or garbage:AH = FPU status (no-root path). Its one caller
  (CColSphere::IntersectRay, 0x40ff8e) uses `test al, al`, so this is safe.
- TOP and the tag word are checked only as stack depth. C0–C3, C1 and the exception flags are never compared.
  MXCSR is not compared against the port (no SSE in these functions).
- NaN payload and sign are compared bit-exactly (f32 and the 80-bit value when `st0_exact`). No tolerance
  exists anywhere in `compare`. The only relaxation is `st0_exact: false` (GetATanOfXY, V-1).
- The native backend calls with an empty x87 stack (`fninit`). This matches MSVC's calling convention
  (x87 stack empty at every call).

### V-8 (I) Generator and branch coverage

`verifier_branch_coverage` hooks every instruction of the original under bare Unicorn. Seed 0x5A0001
generates 200,000 cases; the edge cases are run separately and together with them.
- **Every instruction and both directions of every conditional branch are covered** in all six functions
  when edges and generated cases are combined (LimitAngle 4 jcc, LimitRadianAngle 6, GetATanOfXY 11,
  GetRadianAngleBetweenPoints 4, SolveQuadratic 1, Normalise 1).
- CVector::Normalise's zero branch (0x59c93b–0x59c943, `jp` fall-through) is reached **only by the edge
  set** (all components ±0). The generator never produces it. The same is true for `dy == -0` in
  GetRadianAngleBetweenPoints: the mutant that ignores -0 is caught by only 44 of 450,000 cases, all from
  edges.

Fix (cheap): let `f32_tuple` sometimes zero every component with random signs.

### V-9 Mutation strength: 15 additional realistic mutants, 15 caught

`verifier_mutants`, 200,000 generated cases plus edges each. The unmutated reference written in the probe
gives 0 mismatches for all three functions.

| function (route) | mutant | mismatches |
|---|---|---|
| Normalise (Unicorn) | sum order `xx + (yy + zz)` | 4,200 |
| | whole function in f32 | 75,689 |
| | NaN sum takes the zero branch (`s > 0` instead of the parity test) | 75,309 |
| | `1.0` constant +1 ulp | 298,189 |
| | `c / sqrt(s)` instead of `c * (1/sqrt(s))` | 126,717 |
| SolveQuadratic (Unicorn) | x2 by `/a` instead of `* (1/a)` | 28,572 |
| | `0.5` +1 ulp (x1 only) | 161,733 |
| | stores x1 before x2 (aliasing) | 28,295 |
| | discriminant stored to f32 before the sqrt | 34,842 |
| | NaN discriminant → no root (`d >= 0` written in Rust) | 75,007 |
| GetRadianAngleBetweenPoints (CPU) | only +0 is replaced by 0.0001 (-0 missed) | 44 (edges only, see V-8) |
| | dx/dy computed in f32 | 1,525 |
| | atan in f32 | 4,337 |
| | dx == 0 counted as positive | 2,518 |
| | final π/2 constant +1 ulp | 233,520 |

## Verdicts

| function | verdict |
|---|---|
| CGeneral::LimitAngle 0x53CB00 | **claim stands** (now also checked on the CPU for 2^24 < \|a\| ≤ 2^34). Ledger wording: hang for \|a\| > 2^33, not ≥ (V-3) |
| CGeneral::LimitRadianAngle 0x53CB50 | **claim stands** |
| CGeneral::GetRadianAngleBetweenPoints 0x53CBE0 | **claim stands**, with the qualification that the authority is the host CPU (V-2) |
| CGeneral::GetATanOfXY 0x53CC70 | **claim needs re-verification**: L1 is recorded while only the f32 is compared and the ledger is pending. 35 call sites consume the 64-bit ST0 (V-1). The x87 API matched 200k caller-pattern probes, the f32 API did not |
| CGeneral::SolveQuadratic 0x53CE30 | **claim stands** |
| CVector::Normalise 0x59C910 | **claim stands** (zero branch covered by edges only, V-8) |

## Comparison with docs/re/l1-harness.md (read after the above)

The doc already covers: the 2^24 generator limit (it gives the reason but does not list it as an
unverified range, V-3); FSQRT leaving C2 set (the source of V-6); the GetATanOfXY ST0 risk (V-1, as a
pending ledger item without the call-site evidence); and the FPU state measured by INF-3.

It does not cover: that 2^33 itself returns; the budget-equals-hang weakness (V-4); the host-CPU dependence
of FPATAN (V-2); the condition-code hazard for future routing (V-6); the 64-byte compare window (V-7); and
the edge-only coverage of two branches (V-8).
