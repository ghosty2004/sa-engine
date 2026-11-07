# PORT-COLL: subsystem COLL (crate `sa-world`, module `collision`)

Every row of `catalog/state.csv` with `subsystem == COLL` (145 rows: `CCollision`, `CCollisionData`,
`CColModel`, `CColTrianglePlane`, `CLinkList<CCollisionData*>::Insert`, `SurfaceInfos_c`, `CColAccel`,
`CTempColModels`, the camera-collision helpers) has a decision below. Follows ADR-001,
`docs/arch/core-port.md` and `docs/arch/stream-port.md`. Agent states (the main session sets
`catalog/state.csv`):

| state | rows | meaning here |
|---|---|---|
| L1 | 79 | `sa-oracle l1 <key> --cases 1000000`: 0 mismatches (Unicorn, `SA_WINEPREFIX=/nonexistent`). Signalling-NaN input cases are routed to the CPU by the runner; with the native backend off they are reported as *unverified* (count in the row; Unicorn is unfaithful only for that input class, docs/re/l1-harness.md). 54 of them also have L2 evidence |
| L2 | 30 | in-process L2 (`tools/e2e/port-l2.sh`, LANE=1): >= 1000 cases or every call, 0 mismatches, 0 undeclared writes, on the grove walk (`grove`), its targeted high-`L2_PER_FRAME` rerun (`burst`/`nested`) or the pause-restart recording (`pr`) |
| implemented-needs-L2 | 11 | ported and hooked, no call in either recording (no evidence) |
| ported-unhookable | 1 | `CCollision::Update` (a lone `ret`) |
| deferred | 24 | not ported in this pass (reasons in the table) |

Ported = 121 (L1 + L2 + needs-L2 + unhookable); 120 `#[hook]` registrations.

L3: with all 120 hooks on Rust the grove walk (5200 frames) and the pause-restart recording (3700
frames) are frame-identical to the all-original reference on all 17 subsystems; `port-l2.sh ports
--runs "plain rust"` (897 engine ports incl. these) is frame-identical on the grove walk. The per-hook
hook-off runs were not made, so no row claims L3.

## Crate decision

A new crate `crates/sa-world` (PLAN §2 fixes `sa-world` for CWorld, entities, CPhysical, collision,
zones, ...), module `collision`, depending on `sa-core`, `sa-math`, `sa-macros` and `sa-stream`
(`CFileLoader::LoadLine`, the `sscanf` helper, model infos, `CColStore` are phase B paths of its
`orig!` calls; the dependency is one-way). Same rules as sa-core/sa-stream: `#![no_std]`,
`#![deny(unsafe_code)]` (only `unsafe impl GameData` for POD layouts), game state through
`sa_core::globals!`/`GPtr`, calls into other catalogued functions through `sa_core::orig!` with the
original address (so every callee's hook toggle governs calls from Rust), every port registered with
`#[hook]` and listed in its module's `HOOKS`. Builds for the host, i686-pc-windows-gnu (phase A,
feature `in-process`) and wasm32-unknown-unknown.

| file | contents |
|---|---|
| `lib.rs`, `hooks.rs` | module list; `hooks::MODULES`/`ALL`/`COUNT`/`GLOBALS`; tests: every `#[hook(` listed, addresses unique and not hooked by sa-core/sa-stream, globals' initial values against the exe image, no global overlapping sa-core's/sa-stream's |
| `collision/shared.rs` | layouts (`CCollisionData`, `CColModel`, `CColTriangle`, `CColTrianglePlane`, `CompressedVector`, `CLink`, `CLinkList`, `CColPoint`; 32-bit size/offset asserts), `.rdata` constants, `field!`, `read_vec`/`write_vec` |
| `collision/fpu.rs` | `Fpu`: a literal x87 register stack over `sa_math::x87::X87` (PC = 24): `fld`, memory/register/pop arithmetic forms, `fxch`, `fstp st(i)`, compares and the `test ah, mask` + `jp`/`jne` branch helpers. The float ports transliterate the original instruction sequences with it |
| `collision/surface.rs` | `SurfaceInfos_c` (50 rows) |
| `collision/data.rs` | `CCollisionData`, `CColModel`, `CColTrianglePlane`, the triangle-plane cache, `CCollision::Init/Shutdown/...` |
| `collision/prim.rs` | the `CCollision` primitives (tests, distances, `Process*`), the vector helpers they inline (`vh`: 0x40FE60, 0x40FE30, 0x40FEC0, 0x4119D0, 0x406DA0, 0x59C890, 0x59C790; none of them is hooked) |
| `collision/accel.rs` | `CColAccel` |
| `collision/misc.rs` | `CheckPeds`, `ResetMadeInvisibleObjects`, `CTempColModels::Shutdown` |

Integration (registration only): `sa-hook` depends on `sa-world` (feature `in-process`); its table is
`LOCAL ++ sa_core ++ sa_stream ++ sa_world` (`table.rs` `N`/`build()`, test
`port_hooks_follow_the_local_ones`); `callers.rs`'s preserved-register lookup includes sa-world.
`sa-oracle` depends on `sa-world`; `lib.rs` `hook_preserve` and `tests/l1.rs`
`preserved_registers_match_the_hook_registrations` include sa-world's registrations; new spec files
`specs_coll_surface.rs`, `specs_coll_data.rs`, `specs_coll_prim.rs` are chained into `specs::all()`.
`specs_core::op_delete_spec` became `pub(crate)` (reused for `CColModel::operator delete`). No change to
the harness core (runner/emu/normalise).

## Conventions worth knowing

- **Float ports.** Each x87 function is a line-by-line transliteration with `Fpu` (operand form, f32
  stores, register reuse, compare + parity branch). gta-reversed was a reading aid only; several of its
  bodies are different algorithms (see the deviations column).
- **f32 copies.** Values the original moves with integer instructions (`mov`, `rep movsd`) are copied as
  `u32` so signalling NaNs keep their bits (an f32 returned through the i686 ABI passes ST0 and would be
  quieted).
- **Preserved registers.** `CCollision::TestSphereSphere` has `preserve = "edx"`: its caller
  `SphereCastVsSphere` keeps spB in EDX across the call (site 0x417F76, `push edx` at 0x417F83; found by
  sa-hook's `callers` test); the L1 spec compares EDX.
- **Pointer fields.** Functions that read pointers out of structs (colData, links, pools) cannot run
  hosted (64-bit `GPtr`), so they have no L1 spec; their evidence is L2.
- **L2 extra regions** (`tools/e2e/l2-extras.txt`, section `# PORT-COLL`): the 50 cache links in the
  heap block `[0x96595C]` and the list itself (0x96592C), the colData/colModel a cdecl collision function
  gets (`arg0`, `[arg0+0x2C]`, `[ecx+0x2C]`), the col-model pool `[0xB744A4]` and the quad-tree node pool
  `[0xB745BC]`, the out-parameters (CVector/CColPoint/float/IplDef/ColDef), `CFileLoader::ms_line`
  (0xB71848) and the CRT lock word 0xC9AC4C for the surface loaders. Each line cites the run that showed
  it was needed (coll1/coll2: mismatches/undeclared writes without it; 0 after).

## Verification (commands, results)

- L1, every spec of `specs_coll_*.rs` (79):
  `SA_WINEPREFIX=/nonexistent systemd-run ... ./target/release/sa-oracle l1 <key> --cases 1000000
  --no-fixtures --threads 8` → 79 × `mismatches=0` (surface getters/IdFromName/plane/pool: PASS; the
  float specs: INCOMPLETE only by their SNaN cases, unverified with the native backend off).
- L2/L3 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=1 tools/e2e/port-l2.sh @<120 hooks> --runs "plain
  l2 rust"`): grove walk `--tag coll3`: (b) l2 trace = plain, (c) rust trace = plain on 5200 frames, 17
  subsystems; 0 mismatches, 0 undeclared writes; the only FAILs are hooks below 1000 cases from
  `L2_PER_FRAME=4` (load bursts). `L2_PER_FRAME=1000` rerun of those 22 (`coll-burst`): every one >= 1000
  cases or every call, 0 mismatches. Pause-restart recording (`coll-pr`): (b), (c) PASS over 3700
  frames, 0 mismatches. Hooks reached only from inside another listed hook (the loaders inside
  `SurfaceInfos_c::Init`, `CColTrianglePlane::Set` inside the plane calculation) were run alone
  (`coll-nested`, `L2_PER_FRAME=1000`): 0 mismatches. `port-l2.sh ports --runs "plain rust"`
  (`coll-ports`): PASS (897 ports, frame-identical).
- `cargo test --workspace --release` (SA_WINEPREFIX=/nonexistent): all pass. `cargo clippy --workspace
  --all-targets -- -D warnings` and `cargo clippy --workspace --target i686-pc-windows-gnu -- -D
  warnings`: clean. `cargo build -p sa-world --target wasm32-unknown-unknown`: OK.

## Divergence notes (ledger candidates)

- `CCollision::PointInPoly` (0x415730): for a `GetPrincipleAxis` result other than 0/2/4 (never returned)
  the original compares three uninitialised stack bytes; the port compares zeros.
- `SurfaceInfos_c` loaders: the sscanf output locals are uninitialised stack in the original until the
  first line fills them; the ports start them at 0 (only visible for malformed first lines).
- `CColAccel::cacheLoadCol`: the SEH frame around the `CColModel` constructor is not reproduced.

## Open items

- 24 deferred rows (table). The larger ones need a dedicated pass each: the triangle tests/processes
  (orientation jump tables), `ProcessLineBox`, the line-of-sight/vertical-line drivers,
  `ProcessColModels`, the camera collision group, `SphereCast*`, `CTempColModels::Initialise` and
  `CCollisionData::Copy` (protection call chunks). `ProcessSphereTriangle`'s k = 0 path needs a runtime
  measurement of the uninitialised point first (spec open question).
- The 11 `implemented-needs-L2` rows need a recording that calls them (e.g. `CColModel::operator=`,
  `CTempColModels::Shutdown`, `CCollision::Shutdown` run at shutdown only).
- SNaN cases of the float L1 specs: rerun with the native backend (CPU-decided) to turn INCOMPLETE into
  PASS.
- Catalogue: 0x4161F0 is `CLinkList<CCollisionData*>::Insert` (spec CLinkList.md; catalogue name empty,
  Ghidra `thunk_FUN_01570010`), 0x50D590 is gr's `GetNearestDistanceOfPedSphereToCameraNearClip`.

## Functions (all 145 COLL rows)

| address | name | conv | port (sa-world `collision::`) | state | evidence | deviations kept / notes |
|---|---|---|---|---|---|---|
| 0x40F070 | CCollisionData::RemoveCollisionVolumes | thiscall | data::remove_collision_volumes | L2 | L2 grove:9/9 (0 mismatches, 0 undeclared) | shadow counts and flags byte not cleared (bytes) |
| 0x40F120 | CCollisionData::Copy | — | — | deferred | 463 instructions through no-CD protection call chunks (computed CMemoryMgr::Free calls `call next; mov r,[cell]; adc/sbb r,[x]; xchg; ret`), not decoded yet; called through orig! by MakeMultipleAlloc/operator= | — |
| 0x40F590 | CCollisionData::CalculateTrianglePlanes | thiscall | data::calculate_triangle_planes | L2 | L2 grove:89/89, pr:75/75 (0 mismatches, 0 undeclared) | count read sign-extended (movsx) |
| 0x40F5E0 | CCollisionData::GetTrianglePoint | thiscall | data::get_triangle_point | L2 | L2 grove:1000/276404, pr:1000/276404 (0 mismatches, 0 undeclared) | none |
| 0x40F640 | CCollisionData::GetShadTrianglePoint | thiscall | data::get_shad_triangle_point | L2 | L2 grove:1000/215827, pr:1000/215827 (0 mismatches, 0 undeclared) | none |
| 0x40F6A0 | CCollisionData::RemoveTrianglePlanes | thiscall | data::remove_triangle_planes | L2 | L2 burst:53/53 (0 mismatches, 0 undeclared) | none |
| 0x40F6C0 | CCollisionData::SetLinkPtr | thiscall | data::set_link_ptr | L2 | L2 grove:89/89, pr:75/75 (0 mismatches, 0 undeclared) | gr-dev CollisionData.cpp:210 harmless (same address); movsx count |
| 0x40F6E0 | CCollisionData::GetLinkPtr | thiscall | data::get_link_ptr | L2 | L2 grove:1000/4390, pr:1000/4390 (0 mismatches, 0 undeclared) | gr-dev CollisionData.cpp:221/228 harmless (no check) |
| 0x40F740 | CColModel::MakeMultipleAlloc | thiscall | data::make_multiple_alloc | L2 | L2 burst:31/31 (0 mismatches, 0 undeclared) | new block keeps +7 bits 3..7; Copy called on a null block when new fails |
| 0x40F7C0 | CColModel::operator= | thiscall | data::assign | implemented-needs-L2 | no call in grove walk / pause-restart | sphere copied before box; +0x28..0x2B not copied |
| 0x40F810 | CColModel::AllocateData-void | thiscall | data::allocate_data | implemented-needs-L2 | no call in grove walk / pause-restart | gr-dev ColModel.cpp:76 (assert) harmless; +7 bits 3..7 keep heap bytes |
| 0x40F870 | CColModel::AllocateData-params | thiscall | data::allocate_data_params | L2 | L2 burst:66/66 (0 mismatches, 0 undeclared) | gr-dev ColModel.cpp:127 keep-original: shadow fields/flag bits 1..7 not initialised; :93/94/101 harmless (round-up) |
| 0x40F9E0 | CColModel::RemoveCollisionVolumes | thiscall | data::col_model_remove_collision_volumes | L2 | L2 burst:1000/1001 (0 mismatches, 0 undeclared) | single block: CCollision::RemoveTrianglePlanes + CMemoryMgr::Free; else RemoveCollisionVolumes + CRT free |
| 0x40FA30 | CColModel::CalculateTrianglePlanes | thiscall | data::col_model_calculate_triangle_planes | implemented-needs-L2 | no call in grove walk / pause-restart | calls CCollisionData::CalculateTrianglePlanes directly (no cache) |
| 0x40FA40 | CColModel::RemoveTrianglePlanes | thiscall | data::col_model_remove_triangle_planes | implemented-needs-L2 | no call in grove walk / pause-restart | none |
| 0x40FC30 | CColModel::operator new | cdecl | data::col_model_operator_new | L1 | L1 1,000,148 cases, 0 mismatches; L2 burst:1000/1001 | none |
| 0x40FC40 | CColModel::operator delete | cdecl | data::col_model_operator_delete | L1 | L1 1,000,030 cases, 0 mismatches; L2 burst:204/204 | first_free re-read after the map write, written only when lower; no null check |
| 0x411610 | CColTrianglePlane::GetNormal-Out | thiscall | data::get_normal | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x411660 | CColTrianglePlane::Set | thiscall | data::triangle_plane_set | L1 | L1 1,002,000 cases, 0 mismatches; L2 nested:1000/1001 | orientation from the re-expanded int16 normal; _ftol truncation |
| 0x411E20 | CCollision::Update | — | collision::data::update | ported-unhookable | lone `ret` (relocation TooShort); collision::data::update | — |
| 0x411E30 | CCollision::SortOutCollisionAfterLoad | cdecl | data::sort_out_collision_after_load | implemented-needs-L2 | no call in grove walk / pause-restart | none |
| 0x411E70 | CCollision::TestSphereSphere | cdecl | prim::test_sphere_sphere | L1 | L1 1,060,000 cases, 0 mismatches, 48,252 SNaN cases unverified (native off); L2 grove:1000/1001, pr:1000/1001 | strict R*R > d2, NaN false (gr <=, x,y,z order); preserve EDX (caller 0x417F83) |
| 0x411EC0 | CCollision::CalculateColPointInsideBox | cdecl | prim::calculate_col_point_inside_box | L1 | L1 1,060,000 cases, 0 mismatches, 53,566 SNaN cases unverified (native off); L2 grove:65/65 | gr-dev Collision.cpp:121 REFUTED; keep-original: only point/normal/depth written, NaN d -> "<= 0" side |
| 0x4120C0 | CCollision::TestSphereBox | cdecl | prim::test_sphere_box | L1 | L1 1,060,000 cases, 0 mismatches, 59,088 SNaN cases unverified (native off); L2 grove:1000/72898, pr:1000/72898 | none |
| 0x412130 | CCollision::ProcessSphereBox | cdecl | prim::process_sphere_box | L1 | L1 1,060,000 cases, 0 mismatches, 64,878 SNaN cases unverified (native off); L2 grove:1000/1606, pr:1000/2127 | gr-dev Collision.cpp:160 keep-original (dist == r accepted, normal dir*(1/dist), pieces copied); :279 pieces keep-original; :289 harmless |
| 0x412700 | CCollision::PointInTriangle | stdcall | prim::point_in_triangle | L1 | L1 1,060,000 cases, 0 mismatches, 69,922 SNaN cases unverified (native off) | none |
| 0x412850 | CCollision::DistToLineSqr | cdecl | prim::dist_to_line_sqr | L1 | L1 1,060,000 cases, 0 mismatches, 53,566 SNaN cases unverified (native off); L2 burst:1000/1001 | branch on f32-stored t; middle case clamped at 0 |
| 0x412970 | CCollision::DistToMathematicalLine | cdecl | prim::dist_to_mathematical_line | L1 | L1 1,060,000 cases, 0 mismatches, 53,566 SNaN cases unverified (native off) | none |
| 0x412A30 | CCollision::DistToMathematicalLine2D | cdecl | prim::dist_to_mathematical_line_2d | L1 | L1 1,060,000 cases, 0 mismatches, 36,324 SNaN cases unverified (native off); L2 grove:1000/1161, pr:1000/1135 | none |
| 0x412A80 | CCollision::DistAlongLine2D | cdecl | prim::dist_along_line_2d | L1 | L1 1,060,000 cases, 0 mismatches, 36,324 SNaN cases unverified (native off); L2 grove:1000/1081, pr:1000/1081 | none |
| 0x412AA0 | CCollision::ProcessLineSphere | cdecl | prim::process_line_sphere | L1 | L1 1,060,000 cases, 0 mismatches, 75,505 SNaN cases unverified (native off); L2 grove:1000/1041, pr:1000/1108 | gr-dev Collision.cpp:926 harmless; keep-original: sum orders, no c<=0/b>0/a<=0 rejections, surfA/pieceA=0, lightA untouched |
| 0x412C70 | CCollision::TestLineBox_DW | cdecl | prim::test_line_box_dw | L1 | L1 1,060,000 cases, 0 mismatches, 80,719 SNaN cases unverified (native off); L2 grove:1000/24776, pr:1000/24776 | none |
| 0x413070 | CCollision::TestLineBox | cdecl | prim::test_line_box | L1 | L1 1,060,000 cases, 0 mismatches, 80,719 SNaN cases unverified (native off) | none |
| 0x413080 | CCollision::TestVerticalLineBox | cdecl | prim::test_vertical_line_box | L1 | L1 1,060,000 cases, 0 mismatches, 80,719 SNaN cases unverified (native off) | none |
| 0x413100 | CCollision::ProcessLineBox | — | — | deferred | 624 instructions of x87 slab clipping (ProcessLineBox); not ported in this pass | — |
| 0x4138D0 | CCollision::Test2DLineAgainst2DLine | cdecl | prim::test_2d_line_against_2d_line | L1 | L1 1,060,000 cases, 0 mismatches, 48,252 SNaN cases unverified (native off); L2 grove:1000/18645 | none |
| 0x413960 | CCollision::ProcessDiscCollision | cdecl | prim::process_disc_collision | L1 | L1 1,060,000 cases, 0 mismatches, 207,167 SNaN cases unverified (native off) | gr-dev Collision.cpp:1242 keep-original (no max(0) clamp, dy subtracted first) |
| 0x413AC0 | CCollision::TestLineTriangle | — | — | deferred | 408 instructions, jump table on the plane orientation (0x4140D8) with uninitialised-local paths; needs a byte-level pass | — |
| 0x4140F0 | CCollision::ProcessLineTriangle | — | — | deferred | 487 instructions (ProcessLineTriangle), same orientation jump table + CStoredCollPoly output | — |
| 0x4147E0 | CCollision::ProcessVerticalLineTriangle | — | — | deferred | 331 instructions + chunk 0x414BB2 (ProcessVerticalLineTriangle) | — |
| 0x414D70 | CCollision::IsStoredPolyStillValidVerticalLine | — | — | deferred | 369 instructions (IsStoredPolyStillValidVerticalLine) | — |
| 0x415230 | CCollision::GetBoundingBoxFromTwoSpheres | cdecl | prim::get_bounding_box_from_two_spheres | L1 | L1 1,060,000 cases, 0 mismatches, 48,252 SNaN cases unverified (native off); L2 grove:1000/1001, pr:1000/1001 | both spheres use a's radius (gr: each its own); EAX not set |
| 0x4152C0 | CCollision::IsThisVehicleSittingOnMe | cdecl | prim::is_this_vehicle_sitting_on_me | L2 | L2 grove:1000/1001, pr:1000/1001 (0 mismatches, 0 undeclared) | none |
| 0x415320 | CCollision::CheckCameraCollisionPeds | — | — | deferred | camera collision (CheckCameraCollisionPeds) not ported in this pass | — |
| 0x4154A0 | CCollision::CheckPeds | cdecl | misc::check_peds | L2 | L2 grove:1000/1001, pr:1000/1001 (0 mismatches, 0 undeclared) | none |
| 0x415540 | CCollision::ResetMadeInvisibleObjects | cdecl | misc::reset_made_invisible_objects | L2 | L2 grove:1000/1001, pr:1000/1001 (0 mismatches, 0 undeclared) | none |
| 0x415590 | CCollision::SphereCastVsBBox | cdecl | prim::sphere_cast_vs_bbox | L1 | L1 1,060,000 cases, 0 mismatches, 80,719 SNaN cases unverified (native off); L2 grove:1000/3829, pr:1000/3829 | none |
| 0x415620 | CCollision::RayPolyPOP | cdecl | prim::ray_poly_pop | L1 | L1 1,060,000 cases, 0 mismatches, 69,922 SNaN cases unverified (native off) | gr-dev Collision.cpp:2678 (no gr body): ported from the bytes (dead code, 0 callers) |
| 0x4156D0 | CCollision::GetPrincipleAxis | cdecl | prim::get_principle_axis | L1 | L1 1,060,000 cases, 0 mismatches, 18,463 SNaN cases unverified (native off); L2 grove:1000/9458, pr:1000/9458 | |x| register vs stored |y|,|z| |
| 0x415730 | CCollision::PointInPoly | cdecl | prim::point_in_poly | L1 | L1 1,060,000 cases, 0 mismatches, 85,689 SNaN cases unverified (native off); L2 grove:1000/10454, pr:1000/10454 | axis 2 tests <= 0, axes 0/4 >= 0; impossible axis compares zeros (orig: uninitialised stack) |
| 0x415950 | CCollision::Closest3 | cdecl | prim::closest3 | L1 | L1 1,060,000 cases, 0 mismatches, 69,922 SNaN cases unverified (native off) | gr-dev Collision.cpp:499 (no gr body): bytes; first distance (z,x,y), others (z,y,x) sum order |
| 0x415A40 | CCollision::ClosestSquaredDistanceBetweenFiniteLines | cdecl | prim::closest_squared_distance_between_finite_lines | L1 | L1 1,060,000 cases, 0 mismatches, 75,505 SNaN cases unverified (native off); L2 grove:1000/31352, pr:1000/31352 | gr-dev Collision.cpp:624 keep-original: Sunday algorithm from the bytes (gr: Ericson) |
| 0x415CF0 | CCollision::SphereCastVersusVsPoly | — | — | deferred | 302 instructions (SphereCastVersusVsPoly; its callee ClosestSquaredDistanceBetweenFiniteLines is ported, L1) | — |
| 0x4161F0 | (unnamed) | thiscall | data::link_list_insert_port | L2 | L2 grove:110/110, pr:75/75 (0 mismatches, 0 undeclared) | CLinkList::Insert (spec CLinkList.md); catalogue name thunk_FUN_01570010 |
| 0x416260 | CCollision::Init | cdecl | data::init | L2 | L2 grove:1/1, pr:1/1 (0 mismatches, 0 undeclared) | none |
| 0x4162E0 | CCollision::Shutdown | cdecl | data::shutdown | implemented-needs-L2 | no call in grove walk / pause-restart | none |
| 0x416330 | CCollision::CalculateTrianglePlanes-colData | cdecl | data::calculate_triangle_planes_col_data | L2 | L2 grove:1000/6740, pr:1000/6740 (0 mismatches, 0 undeclared) | none |
| 0x416400 | CCollision::RemoveTrianglePlanes-colData | cdecl | data::remove_triangle_planes_col_data | L2 | L2 burst:1000/1001 (0 mismatches, 0 undeclared) | none |
| 0x416450 | CCollision::ProcessSphereSphere | cdecl | prim::process_sphere_sphere | L1 | L1 1,060,000 cases, 0 mismatches, 53,566 SNaN cases unverified (native off); L2 grove:1000/7446, pr:1000/6141 | gr-dev Collision.cpp:1506 keep-original (no (ra+rb)^2 pre-test; z,x,y sum) |
| 0x4165B0 | CCollision::TestSphereTriangle | — | — | deferred | 431 instructions, 15 callees (TestSphereTriangle) | — |
| 0x416BA0 | CCollision::ProcessSphereTriangle | — | — | deferred | 621 instructions; spec open question (k = 0 path uses uninitialised stack Q, needs a runtime measurement before porting, PLAN 4b) | — |
| 0x417470 | CCollision::TestLineSphere | cdecl | prim::test_line_sphere | L1 | L1 1,060,000 cases, 0 mismatches, 69,922 SNaN cases unverified (native off); L2 grove:1000/1003, pr:1000/1003 | none |
| 0x417610 | CCollision::DistToLine | cdecl | prim::dist_to_line | L1 | L1 1,060,000 cases, 0 mismatches, 53,566 SNaN cases unverified (native off); L2 grove:14/14, pr:6/6 | none |
| 0x417730 | CCollision::TestLineOfSight | — | — | deferred | TestLineOfSight (159 instructions + chunk, indirect call) | — |
| 0x417950 | CCollision::ProcessLineOfSight | — | — | deferred | ProcessLineOfSight (239 instructions, 18 callees) | — |
| 0x417BF0 | CCollision::ProcessVerticalLine | — | — | deferred | ProcessVerticalLine (288 instructions; see-through filter/stored poly static) | — |
| 0x417F20 | CCollision::SphereCastVsSphere | cdecl | prim::sphere_cast_vs_sphere | L1 | L1 1,060,000 cases, 0 mismatches, 69,922 SNaN cases unverified (native off); L2 grove:1000/1001, pr:1000/1001 | none |
| 0x417FD0 | CCollision::ClosestPointOnLine | cdecl | prim::closest_point_on_line | L1 | L1 1,060,000 cases, 0 mismatches, 53,566 SNaN cases unverified (native off) | none |
| 0x418100 | CCollision::ClosestPointsOnPoly | cdecl | prim::closest_points_on_poly | L1 | L1 1,060,000 cases, 0 mismatches, 69,922 SNaN cases unverified (native off) | gr-dev Collision.cpp:1867 (no gr body): bytes (dead code) |
| 0x418150 | CCollision::ClosestPointOnPoly | cdecl | prim::closest_point_on_poly | L1 | L1 1,060,000 cases, 0 mismatches, 69,922 SNaN cases unverified (native off) | gr-dev Collision.cpp:1872 (no gr body): bytes (dead code) |
| 0x4181B0 | CCollision::SphereCastVsCaches | — | — | deferred | 282 instructions (SphereCastVsCaches; spec notes swapped-sphere back-face call kept) | — |
| 0x418580 | CCollision::CalculateTrianglePlanes-CColModelPtr | cdecl | data::calculate_triangle_planes_col_model | L2 | L2 grove:1000/1003, pr:1000/1003 (0 mismatches, 0 undeclared) | none |
| 0x4185A0 | CCollision::RemoveTrianglePlanes-CColModelPtr | cdecl | data::remove_triangle_planes_col_model | implemented-needs-L2 | no call in grove walk / pause-restart | none |
| 0x4185C0 | CCollision::ProcessColModels | — | — | deferred | ProcessColModels (1920 instructions, 39 deviation rows; disk index list layout open question) | — |
| 0x419F00 | CCollision::SphereCastVsEntity | — | — | deferred | 502 instructions (SphereCastVsEntity; spA radius bug to keep) | — |
| 0x41A5A0 | CCollision::SphereVsEntity | — | — | deferred | 210 instructions, dead code (SphereVsEntity, 0 callers) | — |
| 0x41A820 | CCollision::CheckCameraCollisionBuildings | — | — | deferred | camera collision (sector lists, entity scan codes) | — |
| 0x41A990 | CCollision::CheckCameraCollisionVehicles | — | — | deferred | camera collision | — |
| 0x41AB20 | CCollision::CheckCameraCollisionObjects | — | — | deferred | camera collision (gr && vs original || ignore test noted in the spec) | — |
| 0x41AC40 | CCollision::BuildCacheOfCameraCollision | — | — | deferred | 282 instructions, 28 callees (BuildCacheOfCameraCollision) | — |
| 0x41B000 | CCollision::CameraConeCastVsWorldCollision | — | — | deferred | CameraConeCastVsWorldCollision | — |
| 0x41B360 | CTempColModels::Shutdown | cdecl | misc::temp_col_models_shutdown | implemented-needs-L2 | no call in grove walk / pause-restart | none |
| 0x50D590 | (unnamed) | — | — | deferred | GetNearestDistanceOfPedSphereToCameraNearClip (spec in CCollision.md); not ported in this pass | — |
| 0x55D0E0 | SurfaceInfos_c::LoadAdhesiveLimits | thiscall | surface::load_adhesive_limits | L2 | L2 nested:1/1 (0 mismatches, 0 undeclared) | gr-dev SurfaceInfos_c.cpp:258 keep-original (no null-file check); first-token skip does not stop at NUL |
| 0x55D220 | SurfaceInfos_c::GetSurfaceIdFromName | thiscall | surface::get_surface_id_from_name | L1 | L1 1,002,000 cases, 0 mismatches; L2 burst:521/521 | exact compare (repe cmpsb incl. NUL); unknown -> 0 |
| 0x55E5C0 | SurfaceInfos_c::GetAdhesionGroup | thiscall | surface::get_adhesion_group | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1049, pr:1000/1049 | none |
| 0x55E5E0 | SurfaceInfos_c::GetTyreGrip | thiscall | surface::get_tyre_grip | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55E600 | SurfaceInfos_c::GetWetMultiplier | thiscall | surface::get_wet_multiplier | L1 | L1 1,000,058 cases, 0 mismatches, 12,117 SNaN cases unverified (native off) | none |
| 0x55E630 | SurfaceInfos_c::GetSkidmarkType | thiscall | surface::get_skidmark_type | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/2293, pr:1000/2265 | none |
| 0x55E650 | SurfaceInfos_c::GetFrictionEffect | thiscall | surface::get_friction_effect | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:2/2 | none |
| 0x55E670 | SurfaceInfos_c::GetBulletFx | thiscall | surface::get_bullet_fx | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55E690 | SurfaceInfos_c::IsSoftLanding | thiscall | surface::is_soft_landing | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55E6B0 | SurfaceInfos_c::IsSeeThrough | thiscall | surface::is_see_through | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/163530 | none |
| 0x55E6D0 | SurfaceInfos_c::IsShootThrough | thiscall | surface::is_shoot_through | L1 | L1 1,002,000 cases, 0 mismatches; L2 burst:1000/1001 | none |
| 0x55E6F0 | SurfaceInfos_c::IsSand | thiscall | surface::is_sand | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55E710 | SurfaceInfos_c::IsWater | thiscall | surface::is_water | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/2569, pr:1000/3567 | none |
| 0x55E730 | SurfaceInfos_c::IsShallowWater | thiscall | surface::is_shallow_water | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:739/739, pr:249/249 | none |
| 0x55E750 | SurfaceInfos_c::IsBeach | thiscall | surface::is_beach | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55E770 | SurfaceInfos_c::IsSteepSlope | thiscall | surface::is_steep_slope | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/2219, pr:1000/2219 | none |
| 0x55E790 | SurfaceInfos_c::IsGlass | thiscall | surface::is_glass | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55E7B0 | SurfaceInfos_c::IsStairs | thiscall | surface::is_stairs | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1867, pr:1000/1583 | none |
| 0x55E7D0 | SurfaceInfos_c::IsSkateable | thiscall | surface::is_skateable | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55E7F0 | SurfaceInfos_c::IsPavement | thiscall | surface::is_pavement | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/2227, pr:1000/1443 | none |
| 0x55E810 | SurfaceInfos_c::GetRoughness | thiscall | surface::get_roughness | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55E830 | SurfaceInfos_c::GetFlammability | thiscall | surface::get_flammability | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55E850 | SurfaceInfos_c::CreatesSparks | thiscall | surface::creates_sparks | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55E870 | SurfaceInfos_c::CantSprintOn | thiscall | surface::cant_sprint_on | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1001, pr:1000/1001 | none |
| 0x55E890 | SurfaceInfos_c::LeavesFootsteps | thiscall | surface::leaves_footsteps | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:738/738, pr:249/249 | none |
| 0x55E8B0 | SurfaceInfos_c::ProducesFootDust | thiscall | surface::produces_foot_dust | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:419/419, pr:72/72 | none |
| 0x55E8D0 | SurfaceInfos_c::MakesCarDirty | thiscall | surface::makes_car_dirty | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1121, pr:1000/1001 | none |
| 0x55E8F0 | SurfaceInfos_c::MakesCarClean | thiscall | surface::makes_car_clean | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1121, pr:1000/1001 | none |
| 0x55E910 | SurfaceInfos_c::CreatesWheelGrass | thiscall | surface::creates_wheel_grass | L1 | L1 1,002,000 cases, 0 mismatches; L2 burst:445/445, pr:152/152 | none |
| 0x55E930 | SurfaceInfos_c::CreatesWheelGravel | thiscall | surface::creates_wheel_gravel | L1 | L1 1,002,000 cases, 0 mismatches; L2 burst:445/445, pr:152/152 | none |
| 0x55E950 | SurfaceInfos_c::CreatesWheelMud | thiscall | surface::creates_wheel_mud | L1 | L1 1,002,000 cases, 0 mismatches; L2 burst:445/445, pr:152/152 | none |
| 0x55E970 | SurfaceInfos_c::CreatesWheelDust | thiscall | surface::creates_wheel_dust | L1 | L1 1,002,000 cases, 0 mismatches; L2 burst:445/445, pr:152/152 | none |
| 0x55E990 | SurfaceInfos_c::CreatesWheelSand | thiscall | surface::creates_wheel_sand | L1 | L1 1,002,000 cases, 0 mismatches; L2 burst:445/445, pr:152/152 | none |
| 0x55E9B0 | SurfaceInfos_c::CreatesWheelSpray | thiscall | surface::creates_wheel_spray | L1 | L1 1,002,000 cases, 0 mismatches; L2 burst:445/445, pr:152/152 | none |
| 0x55E9D0 | SurfaceInfos_c::CreatesPlants | thiscall | surface::creates_plants | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/90509, pr:1000/90509 | none |
| 0x55E9F0 | SurfaceInfos_c::CreatesObjects | thiscall | surface::creates_objects | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/90423, pr:1000/90423 | none |
| 0x55EA10 | SurfaceInfos_c::CanClimb | thiscall | surface::can_climb | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55EA30 | SurfaceInfos_c::IsAudioConcrete | thiscall | surface::is_audio_concrete | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:399/399, pr:69/69 | none |
| 0x55EA50 | SurfaceInfos_c::IsAudioGrass | thiscall | surface::is_audio_grass | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1135, pr:1000/1111 | none |
| 0x55EA70 | SurfaceInfos_c::IsAudioSand | thiscall | surface::is_audio_sand | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1117, pr:1000/1116 | none |
| 0x55EA90 | SurfaceInfos_c::IsAudioGravel | thiscall | surface::is_audio_gravel | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1117, pr:1000/1116 | none |
| 0x55EAB0 | SurfaceInfos_c::IsAudioWood | thiscall | surface::is_audio_wood | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55EAD0 | SurfaceInfos_c::IsAudioWater | thiscall | surface::is_audio_water | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1138, pr:1000/1307 | none |
| 0x55EAF0 | SurfaceInfos_c::IsAudioMetal | thiscall | surface::is_audio_metal | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:92/92, pr:15/15 | none |
| 0x55EB10 | SurfaceInfos_c::IsAudioLongGrass | thiscall | surface::is_audio_long_grass | L1 | L1 1,002,000 cases, 0 mismatches; L2 grove:1000/1117, pr:1000/1116 | none |
| 0x55EB30 | SurfaceInfos_c::IsAudioTile | thiscall | surface::is_audio_tile | L1 | L1 1,002,000 cases, 0 mismatches | none |
| 0x55EB50 | SurfaceInfos_c::GetAdhesiveLimit | thiscall | surface::get_adhesive_limit | L1 | L1 1,060,000 cases, 0 mismatches, 13,998 SNaN cases unverified (native off); L2 grove:1000/2462, pr:1000/2282 | none |
| 0x55EB90 | SurfaceInfos_c::LoadSurfaceInfos | thiscall | surface::load_surface_infos | L2 | L2 nested:1/1 (0 mismatches, 0 undeclared) | gr-dev :304 keep-original (no null check), :329 keep-original (int8(_ftol(x*100))); SANDY=2/MUDDY=1; bit 0 of flag columns |
| 0x55F2B0 | SurfaceInfos_c::LoadSurfaceAudioInfos | thiscall | surface::load_surface_audio_infos | L2 | L2 nested:1/1 (0 mismatches, 0 undeclared) | gr-dev :381 keep-original (no null check); bit 0 of the values |
| 0x55F420 | SurfaceInfos_c::Init | thiscall | surface::init | L2 | L2 grove:1/1, pr:1/1 (0 mismatches, 0 undeclared) | none |
| 0x5B2AC0 | CColAccel::isCacheLoading | cdecl | accel::is_cache_loading | L2 | L2 burst:1000/1001 (0 mismatches, 0 undeclared) | state read through protection chunk 0x40220F (decoded: 0xBC40A0) |
| 0x5B2AD0 | CColAccel::endCache | cdecl | accel::end_cache | L2 | L2 grove:1/1, pr:1/1 (0 mismatches, 0 undeclared) | none |
| 0x5B2C20 | CColAccel::addCacheCol | cdecl | accel::add_cache_col | L2 | L2 burst:1000/1001 (0 mismatches, 0 undeclared) | none |
| 0x5B2CC0 | CColAccel::cacheLoadCol | cdecl | accel::cache_load_col | implemented-needs-L2 | no call in grove walk / pause-restart | no null check of new; SEH frame not reproduced |
| 0x5B2DD0 | CColAccel::addColDef | cdecl | accel::add_col_def | L2 | L2 burst:251/251 (0 mismatches, 0 undeclared) | none |
| 0x5B2E60 | CColAccel::getColDef | cdecl | accel::get_col_def | implemented-needs-L2 | no call in grove walk / pause-restart | EAX = incremented bound count |
| 0x5B2ED0 | CColAccel::setIplDef | cdecl | accel::set_ipl_def | L2 | L2 burst:190/190 (0 mismatches, 0 undeclared) | none |
| 0x5B2EF0 | CColAccel::getIplDef | cdecl | accel::get_ipl_def | implemented-needs-L2 | no call in grove walk / pause-restart | none |
| 0x5B2F10 | CColAccel::cacheIPLSection | cdecl | accel::cache_ipl_section | L2 | L2 burst:52/52 (0 mismatches, 0 undeclared) | none |
| 0x5B3040 | CColAccel::addIPLEntity | cdecl | accel::add_ipl_entity | L2 | L2 burst:1000/1001 (0 mismatches, 0 undeclared) | LOD pointer dereferenced without null check |
| 0x5B31A0 | CColAccel::startCache | cdecl | accel::start_cache | L2 | L2 grove:1/1, pr:1/1 (0 mismatches, 0 undeclared) | none |
| 0x5BA8C0 | CTempColModels::Initialise | — | — | deferred | CTempColModels::Initialise (939 instructions of static col model setup, 83 callees) | — |
