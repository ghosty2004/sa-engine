# MATH-2: the MATH subsystem in `sa-math`, verified with the L1 runner

Every catalogue row with `subsystem == MATH` (99 rows) is ported to `crates/sa-math` from the 1.0 US
bytes (capstone disassembly of the catalogued extents, `.HOODLUM` bodies included; gta-reversed read for
names and intent only) and, where the function can run without state beyond its arguments, has an L1
spec in `crates/sa-oracle` (`src/math_specs.rs`, `src/list_specs.rs`, builder `src/mspec.rs`).

Counts: **99 ported**. **96 with an L1 spec, 0 mismatches** at 10^6 generated cases plus edge cases each
(seed 0x5A0001, game FPU state CW 0x007F / MXCSR 0x9FA0): 79 decided by Unicorn, 17 decided by the CPU
(FSIN/FCOS/FPATAN, interim authority until INF-8; host Intel i7-14700F under Wine). Of these 96, the six
INF-7 functions were re-run on the changed harness; `GetATanOfXY` still compares only the f32 (its ST0
differs beyond the f32 in 75,962 cases, verifier V-1), so it stays `implemented`. **3 need L2**
(heap: `CMatrixLinkList::Init`/`Shutdown`; global write: `CMaths::InitMathsTables`). **0 blocked.**
Partial coverage (a path needs L2; stated per row): GetRandomNumberInRange ×2 (the CRT `rand` state),
GetNodeHeadingFromVector (NaN angle → CRT errno path), CMatrix::Detach/Attach (owning path →
`RwMatrixDestroy`), GetSinFast/GetCosFast (the table is the runtime state of a global).

## Commands

```sh
cargo build -p sa-oracle --release
# one spec (key from `sa-oracle list`), or all:
systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 \
  ./target/release/sa-oracle l1 <key|all> --cases 1000000
cargo test --release -p sa-math -p sa-oracle          # includes 20k-case runs of every spec
cargo clippy -p sa-math -p sa-oracle --all-targets --features sa-oracle/mutants -- -D warnings
cargo build --release --target i686-pc-windows-gnu -p sa-hook
cargo build -p sa-math --target wasm32-unknown-unknown
```

Peak RSS of one 10^6-case run: about 0.7 GB (blocks of 50,000 cases).

## How the specs work (`mspec.rs`)

- Inputs are laid out as **regions in scratch memory** (`this` and every pointer argument point into
  them). Every float word of a region is a generated float input (the generic mix of `generate.rs`, plus
  rotation matrices / unit quaternions for 35 % of matrix and quaternion regions); flags, padding,
  bytes and integer arguments are generated raw words. The port side reads the same initial bytes into
  the `sa-math` structs, calls the port, writes the structs back, and the runner compares the **whole
  scratch image (0x500 bytes) byte for byte** plus EAX/AL/ST0 (ST0 always all 80 bits for these
  functions) and the x87 stack depth.
- **Aliasing**: specs whose C++ signature invites it run 10-30 % of the cases with an output pointer
  equal to an input pointer (column notes). Where the original interleaves loads and stores, the port has
  an explicit aliased variant (`from_multiply_in_place`, `cross_og_aliased`, `invert_in_place`).
- **Struct layouts**: `#[repr(C)]` with `const` size/offset assertions for `CVector`, `CVector2D`,
  `RwMatrix`, `CMatrix`, `CQuaternion`, `CRect`, `CBox`, `CColBox`, `CColLine`, `CSphere`,
  `CColSphere`, `CColSurface`, `CCompressedMatrixNotAligned` (sizes match `catalog/layouts.csv`).
  `CMatrixLink`/`CMatrixLinkList` are ported on a 32-bit memory model (`matrix_link::Mem32`) with
  their offsets as constants, because they only move pointers.
- **Purity check**: `catalog/rwsets.csv` shows only `.rdata` reads for every MATH function except
  GetSinFast/GetCosFast (`.data` read of the sine table) and InitMathsTables (the table). Callees outside
  MATH (`catalog/callgraph.csv`): `_ftol2` 0x821B40 (pure, allowed callee), `RwMatrixUpdate` 0x7F18A0
  (pure, allowed callee, ported as `matrix::rw_matrix_update`), `rand` 0x821B1E (TLS state), `floor`
  0x8219F0 (reads `__sse2_available`), `RwMatrixDestroy` 0x7F2A20, `operator new`, `free` and the EH
  vector iterators (heap). The Unicorn guard rails enforce this per case (allow-list, no global writes,
  no writable-image reads except the ranges a spec declares).

### Harness changes made for MATH-2 (`sa-oracle`)

| change | why |
|---|---|
| scratch is 0x500 bytes, compared whole (`SCRATCH_LEN`); native records carry their length | matrices, three-matrix operators and the CMatrixLinkList (0x1FC + 8 links) |
| function extents = [start, next start) ∪ `catalog/extents.csv` ranges | `.HOODLUM` bodies behind stubs, chunks the compiler moved (SetUnity, Scale, operator=) |
| the stack below the arguments is filled with 0x5A5A5A5A before every call (both backends) | reads of uninitialised stack are deterministic, so Unicorn and the CPU can be compared on them |
| FS points at a fake TIB under Unicorn (IA32_FS_BASE MSR; `FS_BASE` is a no-op in 32-bit mode), SEH chain head checked after the call | SEH prologues (`CMatrix::operator*=`) |
| `Spec::patches` (image bytes per spec, both backends) and `Spec::reads` (writable image ranges a spec may read) | `rand()` stubbed to `mov eax, [scratch+0x10]; ret` so its value is an input; `__sse2_available = 1` and the sine table set to their runtime values |
| native backend: image and probe files named by content hash, written via a temporary name | specs with patches and parallel tests/runs no longer overwrite each other's `image.bin`/`probes.bin` (a parallel `cargo test` crashed ref.exe on a foreign probe blob) |
| `RetKind::Eax` (full EAX compared) and `Spec::ignore` (scratch words not compared, ledger-backed) | pointer/int returns; uninitialised stack copied into results |
| `generate::f32_tuple`: 3 % all-zero tuples with random signs | verifier V-8 |
| `scan`: hazard "FSQRT between FCOM and FNSTSW" (forces L2 routing) | verifier V-6; no MATH-2 function has it |
| `routes_match_hazards`: SSE allowed only for CPU-routed specs | GetNodeHeadingFromVector reaches the CRT's SSE2 `floor` |
| test `fsin_fcos_model_matches_the_cpu` | the FSIN/FCOS model against the CPU on the 256 sine-table arguments and 100,000 random arguments (f32 compared) |

### x87 additions (`sa_math::x87`)

- `fsin`/`fcos`: NaN → quieted, ±inf → indefinite, |x| ≥ 2^63 → the operand unchanged (the CPU sets C2),
  else the argument is reduced **against the CPU's 66-bit pi** `0x3.243F6A8885A308D3` by quadrants
  (`k = round(x / (P/2))`, remainder exact in 128-bit integers, |r| ≤ P/4) and evaluated in f64. A
  first version reduced by half turns and evaluated `cos(r)` near ±pi/2 in f64, which loses the relative
  precision of results near 0 (`cos(1.5707964)`): 156 mismatches in SetRotate, 0 after the change.
  Like FPATAN, results are 53-bit (CPU: 64-bit); not affected by PC=24.
- `ftol`: MSVC `_ftol2` (FISTP to nearest, corrected by the sign of the f32-stored difference; NaN and
  out-of-range give 0x8000000000000000).
- `from_i32` (`fild`), `fdouble` (`fadd st, st`), `from_f64_exact`.

### Findings from the bytes (port follows the original)

- `CMatrix::m_bOwnsAttachedMatrix` is read and written as a dword (`mov ecx,[esi+44h]; test ecx,ecx`;
  `Attach` stores `movzx`): the padding bytes take part in the test. Modelled as a `u32`.
- `CMatrix` operator* (Mat×Mat), operator+, `Invert(const CMatrix&)` and operator*= copy a stack
  temporary whose flags/padding words were never written (see ledger).
- `CVector::Cross_OG`, `FromMultiply`, `FromMultiply3x3` and `Invert(src, dst)` store components before
  loading later operands; with aliased pointers the result is not the mathematical one.
- `CMatrix::RotateY` multiplies the right row by the register `sin` (`fst`) but the other rows by its f32
  copy; RotateX/RotateZ use only the f32 copy. `Rotate` keeps R[2][2] in a register, `SetRotate` keeps
  `sin x` in a register and stores `cos y`.
- `CMatrix::ConvertTo/FromEulerAngles` decode the flags through the exe's byte tables (0x866D9C
  `{0,1,2,0}`, 0x866D94 `{1,2,0,1,0,0,0,0}`); every 32-bit flag value is valid (bits 3-4 = 3 selects axis 0).
- `CColSphere::IntersectEdge` returns `(t1 < len) ? start + t1*dir : end` into its **4th** argument and
  `(t2 > 0) ? start + t2*dir : start` into the 3rd, after rejecting `t2 > len` and `t1 < 0`.
- `CGeneral::GetRandomNumberInRange`: both versions from the bytes (gta-reversed's formulas differ,
  `catalog/gr-deviations.csv`): int `min + ftol((rand & 0xFFFF) * (1/32768) * (max - min))` with wrapping
  `max - min`; float `(max - min) * (rand * (1/32767)) + min`.
- gta-reversed's RotateX/Y/Z have extra NOTSA arguments; the ports have the original one-argument form.

## Results (seed 0x5A0001, 10^6 generated cases + edge cases each)

"CPU" = decided by the native backend (real CPU under Wine; interim authority for FSIN/FCOS/FPATAN until
INF-8, host CPU model recorded: Intel i7-14700F). "Unicorn; N SNaN cases by the CPU" = Unicorn decided
except for signalling-NaN inputs (L1 harness rule). Every row has 0 mismatches unless marked.

| address | name | cases (edges) | mismatches | backend | notes |
|---|---|---|---|---|---|
| 0x404190 | `CRect::IsFlipped` | 1,060,000 (60,000) | 0 | Unicorn; 54,444 SNaN cases by the CPU | returns 0/1 in EAX (full EAX compared) |
| 0x404200 | `CRect::Restrict` | 1,060,000 (60,000) | 0 | Unicorn; 106,516 SNaN cases by the CPU | returns `this`; alias case `&o == this` (10 %) |
| 0x404260 | `CRect::Resize` | 1,060,000 (60,000) | 0 | Unicorn; 80,700 SNaN cases by the CPU | `right = x + right`, `top = y + top` (memory-operand forms) |
| 0x404290 | `CRect::IsPointInside-` | 1,060,000 (60,000) | 0 | Unicorn; 80,700 SNaN cases by the CPU | AL; a NaN coordinate passes every test |
| 0x4042d0 | `CRect::IsPointInside-Tolerance` | 1,060,000 (60,000) | 0 | Unicorn; 93,807 SNaN cases by the CPU | AL |
| 0x406d50 | `CVector::Magnitude2D` | 1,060,000 (60,000) | 0 | Unicorn; 41,136 SNaN cases by the CPU | ST0 exact |
| 0x407180 | `CGeneral::GetRandomNumberInRange_int32` | 1,002,000 (2,000) | 0 | Unicorn | `min + ftol((rand & 0xFFFF) * (1/32768) * (max - min))`, from the bytes (gta-reversed differs, gr-deviations.csv). `rand()` (0x821B1E) patched to `mov eax, [scratch]; ret`, so its value is an input: arithmetic L1, the CRT rand state needs L2. Callees: rand stub, `_ftol2` |
| 0x4082c0 | `CVector::Magnitude` | 1,060,000 (60,000) | 0 | Unicorn; 41,136 SNaN cases by the CPU | ST0 exact |
| 0x40ede0 | `CBox::Set` | 1,060,000 (60,000) | 0 | Unicorn; 155,707 SNaN cases by the CPU | `.HOODLUM` body (extent from catalog/extents.csv) |
| 0x40ee20 | `CBox::Recalc` | 1,060,000 (60,000) | 0 | Unicorn; 80,700 SNaN cases by the CPU | swap writes `max = fld min; fstp` (quiets nothing: only taken for ordered `min > max`) |
| 0x40ee70 | `CColBox::Set` | 1,060,000 (60,000) | 0 | Unicorn; 155,707 SNaN cases by the CPU | `.HOODLUM` body; material/flags/lighting bytes from the low byte of each stack dword |
| 0x40ef10 | `CColLine::Set` | 1,060,000 (60,000) | 0 | Unicorn; 155,707 SNaN cases by the CPU | `.HOODLUM` body; padding words kept |
| 0x40fcf0 | `CSphere::Set` | 1,060,000 (60,000) | 0 | Unicorn; 106,516 SNaN cases by the CPU | `.HOODLUM` body; `Set(radius, center)` |
| 0x40fd10 | `CColSphere::Set` | 1,060,000 (60,000) | 0 | Unicorn; 106,516 SNaN cases by the CPU |  |
| 0x40fdb0 | `CVector::DotProduct-v3d*v3d*` | 1,060,000 (60,000) | 0 | Unicorn; 80,700 SNaN cases by the CPU | ST0 exact; `(a.z*b.z + a.y*b.y) + a.x*b.x` |
| 0x40fdd0 | `CVector::Sum` | 1,060,000 (60,000) | 0 | Unicorn; 119,134 SNaN cases by the CPU | alias case `a == this` (25 %): per-component, alias-safe |
| 0x40fe00 | `CVector::Difference` | 1,060,000 (60,000) | 0 | Unicorn; 119,134 SNaN cases by the CPU | alias case `b == this` (25 %) |
| 0x40ff20 | `CColSphere::IntersectRay` | 1,060,000 (60,000) | 0 | Unicorn; 143,379 SNaN cases by the CPU | calls SolveQuadratic (allowed callee) with b, c rounded to f32; z products stored as f32 before the add, x/y not. Out-pointer aliasing with origin/dir not modelled |
| 0x410040 | `CColSphere::IntersectPoint` | 1,060,000 (60,000) | 0 | Unicorn; 106,516 SNaN cases by the CPU | EAX 0/1; sum order `(dz^2 + dx^2) + dy^2` |
| 0x410090 | `CColSphere::IntersectSphere` | 1,060,000 (60,000) | 0 | Unicorn; 131,411 SNaN cases by the CPU | AL |
| 0x4100e0 | `CColSphere::IntersectEdge` | 1,060,000 (60,000) | 0 | Unicorn; 143,379 SNaN cases by the CPU | calls CVector::Normalise; writes arg 4 then arg 3; root slots reuse the argument slots. Out-pointer aliasing not modelled |
| 0x411a80 | `CMatrix::operator*=` | 1,060,000 (60,000) | 0 | Unicorn; 293,667 SNaN cases by the CPU | SEH frame (fs:[0], fake TIB under Unicorn); callees operator*, operator=, ~CMatrix, UpdateRwMatrix, RwMatrixUpdate; attached RwMatrix in scratch (half the cases). The 4 flags/padding words of `this` are uninitialised stack in the original: not compared (ledger: original UB) |
| 0x41bd90 | `CGeneral::GetRandomNumberInRange<float>-` | 1,003,364 (3,364) | 0 | Unicorn; 23,675 SNaN cases by the CPU | `(max - min) * (rand * (1/32767)) + min`, from the bytes; rand stubbed as for 0x407180. ST0 exact |
| 0x420860 | `CVector2D::Magnitude` | 1,003,364 (3,364) | 0 | Unicorn; 23,675 SNaN cases by the CPU | ST0 exact |
| 0x43e020 | `CRect::SetFromCenter` | 1,060,000 (60,000) | 0 | Unicorn; 93,807 SNaN cases by the CPU | returns `this` |
| 0x43e050 | `CRect::GetCenter-` | 1,060,000 (60,000) | 0 | Unicorn; 54,444 SNaN cases by the CPU | two out-pointers |
| 0x43e110 | `CVector2D::operator=` | 1,060,000 (60,000) | 0 | Unicorn; 54,444 SNaN cases by the CPU | returns `this` |
| 0x44e480 | `CVector2D::Normalise` | 1,003,364 (3,364) | 0 | Unicorn; 23,675 SNaN cases by the CPU | `s == 0` sets only x |
| 0x459350 | `CMatrix::Scale` | 1,060,000 (60,000) | 0 | Unicorn; 124,968 SNaN cases by the CPU | 3x3 only |
| 0x4a1340 | `CMaths::GetSinFast` | 1,000,058 (58) | 0 | Unicorn; 12,117 SNaN cases by the CPU | reads the `.bss` sine table (0xBB3E00): the spec patches in the table `InitMathsTables` builds (values checked against the CPU) and allows that read; port `maths::get_sin_fast(table, a)`. Partial: the table is runtime state of a global (L2 for the state) |
| 0x4a1360 | `CMaths::GetCosFast` | 1,000,058 (58) | 0 | Unicorn; 12,117 SNaN cases by the CPU | as GetSinFast (+64 before `_ftol2`) |
| 0x4d1610 | `CQuaternion::Normalise` | 1,060,000 (60,000) | 0 | Unicorn; 37,998 SNaN cases by the CPU | `s == 0` sets only w; NaN `s` normalises (`test ah,44h`) |
| 0x5327f0 | `CRect::StretchToPoint` | 1,060,000 (60,000) | 0 | Unicorn; 80,700 SNaN cases by the CPU | returns `this`; raw copies |
| 0x53cb00 | `CGeneral::LimitAngle` | 1,000,057 (57) | 0 | Unicorn; 12,486 SNaN cases by the CPU | re-run on the new harness (verifier V-3 range not changed here) |
| 0x53cb50 | `CGeneral::LimitRadianAngle` | 1,000,058 (58) | 0 | Unicorn; 12,117 SNaN cases by the CPU | re-run |
| 0x53cbe0 | `CGeneral::GetRadianAngleBetweenPoints-` | 1,250,000 (250,000) | 0 | CPU (interim, i7-14700F) | re-run; CPU-decided |
| 0x53cc70 | `CGeneral::GetATanOfXY` | 1,003,364 (3,364) | 0 | CPU (interim, i7-14700F) | re-run; CPU-decided; f32 compared only (state `implemented` per the verifier, unchanged here) |
| 0x53cdc0 | `CGeneral::GetNodeHeadingFromVector` | 1,003,364 (3,364) | 0 | CPU (interim, i7-14700F) | FPATAN (callee) + CRT `floor` (0x8219F0, SSE2 path: `__sse2_available` at 0xC9C400 patched to its runtime value 1, read allowed) + `_ftol2`. Inputs whose angle is NaN are not generated (floor's error path writes errno): that path needs L2. CPU-decided |
| 0x53ce30 | `CGeneral::SolveQuadratic` | 1,195,112 (195,112) | 0 | Unicorn; 52,930 SNaN cases by the CPU | re-run |
| 0x53cea0 | `CGeneral::GetAngleBetweenPoints` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | radian angle (callee, FPATAN) times 57.295776; ST0 exact. CPU-decided |
| 0x54e8f0 | `CMatrixLink::Insert` | 1,000,256 (256) | 0 | Unicorn | on a `Mem32` model with the original's load/store order; `this` = head sentinel or member, `link` any of 8 links (aliasing included) |
| 0x54e910 | `CMatrixLink::Remove` | 1,000,256 (256) | 0 | Unicorn |  |
| 0x54e990 | `CMatrixLinkList::Shutdown` | — | — | needs L2 | `eh vector destructor iterator` + `free`: heap. Port `matrix_link::shutdown` with the destructor and `free` as closures |
| 0x54e9d0 | `CMatrixLinkList::AddToList1` | 1,000,256 (256) | 0 | Unicorn | EAX = link or 0 (empty free list generated) |
| 0x54ea10 | `CMatrixLinkList::AddToList2` | 1,000,256 (256) | 0 | Unicorn | EAX = link or 0 |
| 0x54ea60 | `CMatrixLinkList::MoveToList1` | 1,000,256 (256) | 0 | Unicorn |  |
| 0x54ea90 | `CMatrixLinkList::MoveToList2` | 1,000,256 (256) | 0 | Unicorn |  |
| 0x54ead0 | `CMatrixLinkList::MoveToFreeList` | 1,000,256 (256) | 0 | Unicorn |  |
| 0x54eb10 | `CMatrixLinkList::GetNumFree` | 1,000,256 (256) | 0 | Unicorn | EAX = count |
| 0x54eb30 | `CMatrixLinkList::GetNumUsed1` | 1,000,256 (256) | 0 | Unicorn | EAX = count |
| 0x54eb50 | `CMatrixLinkList::GetNumUsed2` | 1,000,256 (256) | 0 | Unicorn | EAX = count |
| 0x54f0d0 | `CMatrixLinkList::Init` | — | — | needs L2 | `operator new` + `eh vector constructor iterator` + SEH: heap. Port `matrix_link::init` with the allocator as a closure |
| 0x59a840 | `CMatrix::ConvertToEulerAngles` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FPATAN; flags from the exe's index tables (0x866D94/0x866D9C), all 32-bit flag values (90 % in 0..31); three out-pointers. CPU-decided |
| 0x59aa40 | `CMatrix::ConvertFromEulerAngles` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS with the 66-bit-pi reduction model; flags as above. CPU-decided |
| 0x59ac90 | `CMaths::InitMathsTables` | — | — | needs L2 | writes the `.bss` sine table (global write) with FSIN. Port `maths::init_maths_tables()`: all 256 values equal the CPU's (`fsin_fcos_model_matches_the_cpu`) |
| 0x59acf0 | `CMatrix::Detach` | 1,060,000 (60,000) | 0 | Unicorn; 209,500 SNaN cases by the CPU | non-owning paths only: `RwMatrixDestroy` (0x7F2A20, RW engine globals) is never reached by the generated inputs; the owning path needs L2. The owns flag is tested as a dword |
| 0x59ad20 | `CMatrix::UpdateMatrix` | 1,060,000 (60,000) | 0 | Unicorn; 209,500 SNaN cases by the CPU |  |
| 0x59ad70 | `CMatrix::UpdateRwMatrix` | 1,060,000 (60,000) | 0 | Unicorn; 209,500 SNaN cases by the CPU | callee `RwMatrixUpdate` (0x7F18A0, `flags &= 0xFFFDFFFC`, ported as `matrix::rw_matrix_update`) |
| 0x59add0 | `CMatrix::CopyOnlyMatrix` | 1,060,000 (60,000) | 0 | Unicorn; 209,500 SNaN cases by the CPU | 16 dwords (flags/padding included) |
| 0x59adf0 | `CMatrix::operator+=` | 1,060,000 (60,000) | 0 | Unicorn; 209,500 SNaN cases by the CPU | returns `this`; alias case `&rhs == this` (20 %) |
| 0x59ae70 | `CMatrix::SetUnity` | 1,060,000 (60,000) | 0 | Unicorn; 112,353 SNaN cases by the CPU |  |
| 0x59aea0 | `CMatrix::ResetOrientation` | 1,060,000 (60,000) | 0 | Unicorn; 112,353 SNaN cases by the CPU |  |
| 0x59aed0 | `CMatrix::SetScale-f` | 1,060,000 (60,000) | 0 | Unicorn; 124,968 SNaN cases by the CPU |  |
| 0x59af00 | `CMatrix::SetScale-fff` | 1,060,000 (60,000) | 0 | Unicorn; 148,575 SNaN cases by the CPU |  |
| 0x59af40 | `CMatrix::SetTranslate` | 1,060,000 (60,000) | 0 | Unicorn; 148,575 SNaN cases by the CPU |  |
| 0x59af80 | `CMatrix::SetTranslateOnly` | 1,060,000 (60,000) | 0 | Unicorn; 148,575 SNaN cases by the CPU |  |
| 0x59afa0 | `CMatrix::SetRotateXOnly` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS. CPU-decided |
| 0x59afe0 | `CMatrix::SetRotateYOnly` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS. CPU-decided |
| 0x59b020 | `CMatrix::SetRotateZOnly` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS. CPU-decided |
| 0x59b060 | `CMatrix::SetRotateX` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS. CPU-decided |
| 0x59b0a0 | `CMatrix::SetRotateY` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS. CPU-decided |
| 0x59b0e0 | `CMatrix::SetRotateZ` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS. CPU-decided |
| 0x59b120 | `CMatrix::SetRotate-xyz` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS; cos/sin products of two register values. CPU-decided |
| 0x59b1e0 | `CMatrix::RotateX` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS. CPU-decided |
| 0x59b2c0 | `CMatrix::RotateY` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS; the right row uses the register sin (`fst`), the others the f32 copy. CPU-decided |
| 0x59b390 | `CMatrix::RotateZ` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS. CPU-decided |
| 0x59b460 | `CMatrix::Rotate` | 1,060,000 (60,000) | 0 | CPU (interim, i7-14700F) | FSIN/FCOS; R[2][2] stays in a register. CPU-decided |
| 0x59b6a0 | `CMatrix::Reorthogonalise` | 1,060,000 (60,000) | 0 | Unicorn; 112,353 SNaN cases by the CPU |  |
| 0x59b7e0 | `CMatrix::ForceUpVector` | 1,060,000 (60,000) | 0 | Unicorn; 148,575 SNaN cases by the CPU | `CVector` by value (3 stack floats) |
| 0x59b8b0 | `CMatrix::CopyToRwMatrix` | 1,060,000 (60,000) | 0 | Unicorn; 209,500 SNaN cases by the CPU | returns the RwMatrix; callee `RwMatrixUpdate` |
| 0x59b920 | `CMatrix::Invert-1` | 1,060,000 (60,000) | 0 | Unicorn; 209,500 SNaN cases by the CPU | returns `dst`; alias case `src == dst` (20 %, port `invert_in_place`: zeroes pos before reading it) |
| 0x59b9f0 | `CCompressedMatrixNotAligned::DecompressIntoFullMatrix` | 1,060,000 (60,000) | 0 | Unicorn; 148,575 SNaN cases by the CPU | callees CrossProduct, Reorthogonalise |
| 0x59bad0 | `CCompressedMatrixNotAligned::CompressFromFullMatrix` | 1,060,000 (60,000) | 0 | Unicorn; 148,575 SNaN cases by the CPU | callee `_ftol2` (0x821B40), low byte kept |
| 0x59bb60 | `CMatrix::Update` | 1,060,000 (60,000) | 0 | Unicorn; 209,500 SNaN cases by the CPU | `m_pAttachMatrix` always set (the original does not check it) |
| 0x59bbb0 | `CMatrix::UpdateRW` | 1,060,000 (60,000) | 0 | Unicorn; 209,500 SNaN cases by the CPU | attached in half the cases; callees UpdateRwMatrix, RwMatrixUpdate |
| 0x59bbc0 | `CMatrix::operator=` | 1,060,000 (60,000) | 0 | Unicorn; 293,667 SNaN cases by the CPU | attached in half the cases (pointer read before the copy) |
| 0x59bbf0 | `CMatrix::SetRotate-quat` | 1,060,000 (60,000) | 0 | Unicorn; 145,634 SNaN cases by the CPU |  |
| 0x59bd10 | `CMatrix::Attach` | 1,060,000 (60,000) | 0 | Unicorn; 293,667 SNaN cases by the CPU | non-owning paths only (see Detach); owns stored zero-extended into the dword |
| 0x59bdd0 | `CMatrix::Invert-2` | 1,060,000 (60,000) | 0 | Unicorn; 209,500 SNaN cases by the CPU | callee Invert(src, dst); the result's 4 flags/padding words are uninitialised stack: not compared (ledger) |
| 0x59be30 | `CMatrix::operator*-Mat` | 1,060,000 (60,000) | 0 | Unicorn; 293,667 SNaN cases by the CPU | hidden return pointer; the result's 4 flags/padding words are uninitialised stack: not compared (ledger) |
| 0x59bfa0 | `CMatrix::operator+-` | 1,060,000 (60,000) | 0 | Unicorn; 293,667 SNaN cases by the CPU | as operator* |
| 0x59c080 | `CQuaternion::Get-` | 1,060,000 (60,000) | 0 | Unicorn; 145,833 SNaN cases by the CPU | writes the 3x3 of the RwMatrix only |
| 0x59c670 | `CVector::FromMultiply` | 1,060,000 (60,000) | 0 | Unicorn; 183,424 SNaN cases by the CPU | alias case `&v == this` (30 %, port `from_multiply_in_place`) |
| 0x59c6d0 | `CVector::FromMultiply3x3` | 1,060,000 (60,000) | 0 | Unicorn; 183,424 SNaN cases by the CPU | alias case `&v == this` (30 %, port `from_multiply_3x3_in_place`) |
| 0x59c730 | `CVector::CrossProduct-out` | 1,060,000 (60,000) | 0 | Unicorn; 119,134 SNaN cases by the CPU | returns `out`; alias case `out == a` (20 %): every load precedes the stores |
| 0x59c890 | `CMatrix::operator*-Vec` | 1,060,000 (60,000) | 0 | Unicorn; 183,424 SNaN cases by the CPU | returns `out`; alias case `out == v` (20 %) |
| 0x59c910 | `CVector::Normalise` | 1,195,112 (195,112) | 0 | Unicorn; 54,641 SNaN cases by the CPU | re-run |
| 0x59c970 | `CVector::NormaliseAndMag` | 1,060,000 (60,000) | 0 | Unicorn; 41,136 SNaN cases by the CPU | ST0 exact (`1 / (1 / sqrt(s))`) |
| 0x70f890 | `CVector::Cross_OG` | 1,060,000 (60,000) | 0 | Unicorn; 119,134 SNaN cases by the CPU | alias case `a == this` (30 %, port `cross_og_aliased`) |

## Rows that need L2

| address | name | why | port |
|---|---|---|---|
| 0x54f0d0 | `CMatrixLinkList::Init` | `operator new` (heap), `eh vector constructor iterator`, SEH | `matrix_link::init(mem, list, count, new_array)`: allocation as a closure; constructor (attach fields 0) and the free-list build (last object inserted first) in the original order |
| 0x54e990 | `CMatrixLinkList::Shutdown` | `eh vector destructor iterator` (→ `~CMatrix` → `RwMatrixDestroy`) and `free` | `matrix_link::shutdown(mem, list, destroy_link, free)` |
| 0x59ac90 | `CMaths::InitMathsTables` | writes the global sine table (0xBB3E00) | `maths::init_maths_tables()`; its 256 values match the CPU's FSIN (`fsin_fcos_model_matches_the_cpu`), so the arithmetic is verified, the global write is L2 |

## Ledger proposals

*pending*
- **FSIN/FCOS/FPATAN precision and authority** (CMatrix::SetRotate*, Rotate*, ConvertFrom/ToEulerAngles,
  GetAngleBetweenPoints, GetNodeHeadingFromVector, InitMathsTables): `sa_math::x87` computes these in
  f64 (53 bits, FSIN/FCOS with the 66-bit-pi reduction); the CPU returns 64 bits. Every stored f32 and every
  PC=24 product of them matched the CPU in all cases run (table above; 0 mismatches), so a difference
  needs an exact result within about 2^-29 ulp of a 24-bit rounding boundary. Also, the authority is the
  host CPU (Intel i7-14700F): x87 transcendental microcode is not specified bit-exactly, so another CPU
  model (AMD) may round differently. Resolve with an x87-exact (64-bit) implementation or accept per
  CPU family with the user.
- **Aliased out-pointers not modelled** for `CColSphere::IntersectRay`/`IntersectEdge` (outputs written
  before `origin`/`start` and `dir` are re-read), `CRect::GetCenter`, `CMatrix::ConvertToEulerAngles`
  (three out-pointers) and `operator*(Mat, Mat)` / `operator+` / `Invert(const CMatrix&)` with the result
  aliasing an operand. The L1 specs use distinct buffers for these. Resolve by an L2 call-site capture
  (do callers ever alias?) or by porting them on a memory model like `matrix::invert_mem`.

*original UB, not reproducible*
- **Uninitialised stack in matrix results**: `operator*(const CMatrix&, const CMatrix&)` (0x59BE30),
  `operator+(…)` (0x59BFA0), `Invert(const CMatrix&)` (0x59BDD0) and through them `CMatrix::operator*=`
  (0x411A80) build the result in a stack temporary whose four flags/padding dwords (offsets 0xC, 0x1C,
  0x2C, 0x3C) are never written, then copy all 16 dwords into the result. Those words hold whatever the
  stack held. The port writes 0; the L1 specs do not compare these 16 bytes (`Spec::ignore`). No MATH
  function reads them back as data (`UpdateRwMatrix`/`UpdateMatrix` skip them; `CopyOnlyMatrix` and
  `operator=` copy them along).

*not divergences, recorded for the verifier*
- GetRandomNumberInRange (int/float): `rand()` is stubbed in the image to return an input; the CRT's
  per-thread seed (`_getptd`) is L2.
- GetNodeHeadingFromVector: `__sse2_available` patched to 1 (its value after CRT start-up on any SSE2
  CPU), so `floor` runs its SSE2 path as in the game; inputs with a NaN angle are excluded (floor then
  calls the CRT error handler, which writes errno).
- GetSinFast/GetCosFast: the sine table is patched in with the values `init_maths_tables()` computes
  (checked against the CPU); the spec verifies the index arithmetic (`_ftol2`, `& 0xFF`) and the lookup.
- CMatrix::Detach/Attach: inputs never take the `RwMatrixDestroy` path (attached and owning); that
  path needs L2.
