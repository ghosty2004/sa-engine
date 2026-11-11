# PORT-RW-1: RenderWare core, first pass (crate `sa-rwcore`)

First pass over the RenderWare 3.6 core linked into `gta_sa.exe` 1.0 US, from the RE-SPEC-RW specs
(`docs/re/specs/RW/*.md`, 370 functions). The exe is authoritative: every constant, offset, store order
and x87 sequence was re-read from the bytes (objdump of the 1.0 US exe); the specs and librw were
reading aids. Follows ADR-001 (globals / `GPtr` / `orig!` / `#[hook]`) and the PORT-HUD template. The
main session sets `catalog/state.csv`; per-function verdicts are in `docs/arch/rw-port-verdicts.tsv`.

Ported: **292 functions**, **287 `#[hook]` registrations** (5 ported but not hookable, below), in this
order: math and memory helpers (agent), frames + plugin registry + atomics + clumps, then camera + Im3D
(helper A), streams + textures/TXD + materials + geometry, then RpMesh + RpLight +
`RpGeometryStreamRead` (helper B). Not reached in this pass: RpWorld, RwRaster/RwImage, RwEngine, the
D3D9 driver layer (render-state cache, Im2D, rasters), the two camera clip-plane builders and a few
large readers (below).

## Crate and its relation to `sa-rw`

`crates/sa-rwcore` (phase A, `#![no_std]`, `#![deny(unsafe_code)]`, host / i686 `--features
in-process` / wasm32). Depends on sa-core, sa-math, sa-macros only (RW is below every game subsystem).
sa-hook appends `sa_rwcore::hooks::ALL` after `sa_hud::hooks::ALL` (table.rs, callers.rs, sa-oracle
`hook_preserve`, `tests/l1.rs`).

- `crates/sa-rw` stays the **phase B native object model** used by `sa-viewer` (Rust-native structs built
  from `sa-formats`; untouched by this pass, so NATIVE-DEMO keeps building).
- `crates/sa-rwcore` is the **in-place port** working on the original memory layout through `GPtr`.
- Shared knowledge lives in `sa_rwcore::layout` (RwMatrix, freelist, module offset globals); helper
  modules keep further offsets next to their code. Decision: `sa-rw` does not depend on `sa-rwcore`
  yet; when phase B moves globals into an engine context (ADR-001 §6), `sa-rw`'s loaders will be
  replaced by the ported stream readers (`RwStreamRead`, `RpGeometryStreamRead`, ...) and its types
  will take their layout from `sa_rwcore::layout`. Until then the two do not share code, only the
  documented layout.

| file | contents |
|---|---|
| `common.rs` | field access, `RwGlobals` (`[0xC97B24]`) memory table calls (`rwmalloc` +0x134, `rwfree` +0x138, `memoryAlloc`/`Free` +0x144/+0x148 through the stored pointers), module bases `G + [offset]`, `RwErrorSet({1, _rwerror(..)})`, link-list helpers |
| `layout.rs` | 1.0 US layout constants (single source for both RW crates) |
| `vector.rs` | VECTORMODULE: `_rwSqrt`, `_rwInvSqrt` (12-bit table), `RwV3dLength`, `RwV3dNormalize`, the 4 `RwV3dTransform*` dispatchers, `_rwVectorSetMultFn`, the module Open incl. table construction (10) |
| `matrix.rs` | MATRIXMODULE and `RwMatrix*`: Determinant / Orthogonal / Normal error, Optimize, Update, Multiply, RotateOneMinusCosineSine, Rotate, Invert, the general inverse, Scale, Translate, Transform, Create, Destroy, Open; `RwBBoxCalculate`; the 4 SSE2 transform kernels (22) |
| `memory.rs` | freelists (Create, CreateInternal, Destroy, AllocReal, FreeReal, Purge, PurgeAll, SetFlags), `_rwMemoryOpen`, RESOURCESMODULE (Open, AllocateResEntry, FreeResEntry, Purge), `_rwResourcesOpen`, the resource arena (Init, FreeBlock, FindFree, SplitBlock) (18) |
| `frame.rs` | RwFrame incl. dirty list and LTM sync (helper A, 41 hooks + 1 unhookable) |
| `plugin.rs` | plugin registry (18) |
| `atomic.rs`, `clump.rs` | RpAtomic (17), RpClump (29 + 1 unhookable) |
| `stream.rs` | RwStream (15 + 1 unhookable) |
| `texture.rs` | RwTexture / RwTexDictionary (31) |
| `material.rs`, `geometry.rs` | RpMaterial / material lists (15), RpGeometry / morph targets (18) |
| `camera.rs` | RwCamera (15 hooks + 2 unhookable; clip-plane builders 0x7EE740 / 0x7EEDE0 still original) |
| `im3d.rs` | RwIm3D and the D3D9 Im3D open/close/instance node (14) |
| `mesh.rs`, `light.rs` | RpMesh / bin-mesh plugin (10), RpLight (12) |
| `engine.rs` | empty stub for the next pass |

## Conventions and findings worth knowing

- **Table sqrt (bit-exact).** `b = bits(x)`; `b == 0` returns the stored value (+0.0) untouched;
  otherwise `b' = b + 0x800`, `i = (b' >> 12) & 0xFFF`, sqrt = `T[i] + ((b' >> 1) & 0x3FC00000)`, inverse
  = `U[i] + ((!b' >> 1) & 0x3FC00000)`, wrapping u32 adds, no sign/NaN handling (`-0.0` and negative
  inputs go through the table). L1 on raw bit patterns: 0 mismatches.
- **VECTORMODULE Open table construction.** Sqrt table: entries 0x800.. from x = 1.0, then 0.. from
  x = 2.0, x advanced by 0x1000 in its bit pattern, `bits(f32(fsqrt x)) - 0x1FC00000` / `- 0x20000000`.
  Inverse table: `1.0 (qword 0x85A310) / fsqrt` with the opposite constants. The port computes the
  inverse table under PC=24 (the `X87` model). Open runs inside `RwEngineStart`; whether PC is already
  24 there decides the last bit of some inverse entries (spec open question). **Decision needed /
  evidence:** the L2 run of `cb_VECTORMODULE_RwEngine_Open` (tag `rw-a-g`) and the all-ports L3 runs
  settle it; see Verification.
- `RwV3dNormalize`: the zero-length error is raised when the length compares `< 0` or `== 0`; a NaN
  length does **not** raise it (`test ah, 0x41; jp` skips on unordered; the spec says otherwise).
- `RwMatrixRotateOneMinusCosineSine` / `RwMatrixTransform` / `RwMatrixRotate` (concat paths): the
  originals copy uninitialised stack into the three pad words of the target (local matrix pads, and
  the multiply temporary whose pads the kernel never writes). The port writes 0 there. L1 ignores those
  words; an L2 mismatch on them needs a normaliser rule like `RULE_FRAME_PADS` (sa-oracle-format,
  harness owner).
- `RwMatrixScale` / `RwMatrixTranslate` with a bad combine op: E_RW_BADPARAM, then the original
  dereferences NULL+0xC (crash); kept.
- `RwMatrixScale` operand order per element (`fld m; fmul s` vs `fld s; fmul m`) follows the bytes
  (matters only for signalling-NaN selection).
- `_rwMatrixInvertOrthoNormalizedOrGeneral` (0x7F2160) has no catalogue name: its hook is registered as
  `FUN_007f2160`; same for `FUN_008082e0` (`_rwResourcesOpen`, really `_rwPluginRegistryOpen`),
  `FUN_008100c0` (ArenaFindFree), `FUN_00810140` (ArenaSplitBlock), `FUN_00749d20`
  (`_rpAtomicDefaultFrameSyncCallBack`).
- SSE2 kernels: plain SSE f32 with the game's MXCSR (FTZ on, DAZ off): results subnormal after rounding
  flush to signed zero, NaN operands propagate the first operand's NaN quieted, invalid ops give
  0xFFC00000. Implemented explicitly (`matrix::sse`), so independent of the Rust MXCSR. n = 0 loops 2^32
  times as the original.
- Freelists: blocks are searched from the list head and new blocks go to the head; `RwFreeListPurge`
  returns `entrySize * freedBlocks`; FreeReal's range test uses the unaligned entry start and an
  inclusive upper bound; with heapSize 0 the "empty block" test is skipped (block freed).
- Resource arena: `SplitBlock` absorbing a free next block sets `n.size = next.size - size + b.size`
  (no header added back), as the bytes compute it.
- Not reproduced (stack only): arguments slots reused as locals (`RwV3dNormalize` stores `f32(s)` over
  `in`; `RwMatrixRotate` stores `rad` over `angle`; the general inverse stores `det`/`ry` over `src`;
  `_rwFreeListCreateInternal` keeps the block size in `alignment`; `_rwFreeListAllocReal` keeps its
  counter in `fl`; helper lists in their sections).
- Helper A (frames/plugins/atomics/clumps) and helper B (streams/textures/materials/geometry) findings,
  bugs kept and differences from the specs: see "Helper findings" below.

### Not hookable (ported, `#[hook]` removed, reached through `orig!`)

| address | function | reason |
|---|---|---|
| 0x7F0210 | `_rwSetHierarchyRoot` | every frame-function caller keeps ECX and EDX across the call (88 uses, callers test); `preserve` needs an L1 spec and the function writes game memory |
| 0x74A260 | `_rpClumpDefaultCallBack` | `RpClumpGetCallBack` compares against the address (keycells test: 0x74A278 reads the patched range) |
| 0x7ED1E0 | `FUN_007ed1e0` (`_rwChunkIsComplex`) | its caller at 0x7ED5DE keeps `info` in ECX across the call |
| 0x7EE200 | `_rwCameraUpdateZShiftScale` | `RwCameraCreate` keeps EDX (= 0) across the call and stores it at 0x7EE58C |
| 0x807C80 | `FUN_00807c80` (`_rwCameraBeginUpdateHook`) | a single `ret` (too short) |

`RwMatrixOrthoNormalize` (0x7F1920) is a `jmp 0x7F1930`; `_rwMatrixOrthoNormalizeInternal` (0x7F1930,
catalogue `unknown`) is not ported in this pass.

### Not ported in this pass

- Math/memory: `_rwMatrixOrthoNormalizeInternal` (0x7F1930), the default x87 transform kernels and
  `_rwMatrixMultiplyDefault` / `_rwMatrixIdentityError` (not in the specs).
- Helper A: `_rpClumpAtomicStreamRead` (0x74B060: reads `len` bytes into a 16-byte stack local without a
  clamp; a stack overflow cannot be reproduced 1:1) and `RpClumpStreamRead` (0x74B420).
- Helper B: `RwTextureGenerateMipmapName` (0x7F3AA0, tail `jmp eax` forwarding the caller's stack
  arguments: argument count not decidable); `_rpMeshOptimise` (0x75D970), `FUN_0074d6d0`,
  `_rpMaterialSetDefaultSurfaceProperties` (0x74D870), `RwFreeListForAllUsed` (0x801E90) stay original.
- Helper A, camera / Im3D: `_rwCameraBuildPerspClipPlanes` (0x7EE740, ~1.7 KB register-heavy x87),
  the parallel builder `FUN_007eede0` (~1 KB), `_rwD3D9ImmRenderPrimitiveNode` (0x80E270, D3D9 draw node).
  Decidable from the bytes, not reached in the time budget.
- Whole spec files: RpWorld (39), RwRaster (10), RwImage (4), RwEngine (25), RwD3D9RenderState (31),
  RwD3D9Im2D (6), RwD3D9Raster (7).

## Helper findings

Helper A (`frame.rs`, `plugin.rs`, `atomic.rs`, `clump.rs`): bytes vs spec: `_rwObjectHasFrameReleaseFrame`
returns EAX = obj; `_rpAtomicResyncInterpolatedSphere` returns the atomic on every path;
`_rwFrameListStreamRead` bounds its loop by the stack local; `RpClumpRender` passes the frame as a hidden
second argument to the render callback; `_rwFrameInit` sets privateFlags `|= 0x10`;
`RwFrameCloneHierarchy` purges clones even when cloning failed; `RpClumpGetCallBack` returns 0 for the
default callback. Bugs kept: CloneRecurse failure path runs plugin destructors on an unconstructed frame;
`RwFrameDestroy` leaves children with a dangling root; `RemoveChild` has no parent NULL check;
FrameListStreamRead has no parent-index bound and leaks on failure; ReadDataChunks' unsigned length can
underflow; RpClumpClone with a frameless atomic reports `{2,5}` and succeeds; RpClumpDestroy does not unlink
atomics. Globals declared: 0xC9796C/70/74, 0x8E2270, 0x8E2288/8C, 0x8E26A8, 0xC9A6E4/E8/EC, 0x8E2298,
0x8D624C, 0xC924A8, 0xC92480, 0xC92484, 0xC924AC/B0/B4, 0xC924D8/DC, 0x8D6264, 0x8D627C (initial values
checked against the exe).

Helper B (`stream.rs`, `texture.rs`, `material.rs`, `geometry.rs`): `_rwStreamReadChunkHeader` writes its
outputs in the order type, length, build, version and calls `_rwChunkIsComplex` on its local (result
dropped); `RwTextureRead` pushes the address 0x8E240C ("(null)"); `RwTexDictionaryDestroy` passes an
extra 0 to `RwTextureDestroy`; RpWorld-side errors use pluginID 2; `RpMaterialStreamRead` destroys the
material before reporting a bad texture-chunk version; stream freelist cells 0x8E2218 (per block 0x10),
0x8E221C (prealloc 1). Bugs kept: unchecked struct-chunk lengths in `RwTextureStreamRead` /
`RpMaterialStreamRead` (ports use `#[repr(C)]` frames laid out like the original, so an overflow hits the
same neighbours inside the frame; beyond the frame the effect differs), `_rwStringStreamFindAndRead` leak
and missing terminator on short ASCII reads, `RwTexDictionaryDestroy` leaving referenced textures linked,
texture leak on plugin-data failure, unchecked material-list index array, `RpGeometryCreate` overrunning
`texCoords[8]` with more than 8 sets, the dead branch in `RpMaterialClone`. Calls to freelist functions
from these modules still go through `orig!` with `=> unported` phase B paths (could point at
`memory.rs`; harmless in phase A).

Helper A, batch 2 (`camera.rs`, `im3d.rs`): ZShiftScale keeps zF2 stored (f32) and zN2 / scale / n+f
in registers, `zShift = ((zF2 + zN2) - (n + f) * scale) * 0.5`; view matrix per column: t0 and X/Y/Z in
registers, t1, t2 and o stored; pos `= 0.5 - (((p.row sum) + p.z*Z) + o)`; the perspective and parallel
paths order the pos products differently (kept). Im3D pipeline jump tables decoded (LINELIST +0x10,
POLYLINE +0x14, TRILIST +4, TRISTRIP +0xC, TRIFAN +8, POINTLIST +0x18); RenderIndexed/RenderPrimitive have
no POINTLIST case (error 0x25, then a NULL pipeline is executed: kept). CreateIndexBuffer fails on any
non-zero HRESULT. Bugs kept: pipeline-builder unlock failure leaks the pipeline; `RwIm3DTransform` stores
0x10000 vertices as u16 0; `RwCameraSetViewWindow` has no zero check; `RwCameraFrustumTestSphere`
treats unordered compares as inside. Globals: 0xC9793C, 0xC97960/64, 0x8E222C, 0x8E2244, 0xC97968,
0xC9C078/7C, 0xC9AB30..0xC9AB4C.

Helper B, batch 2 (`mesh.rs`, `light.rs`, `RpGeometryStreamRead`): `_rpMeshRead` sizes the index area
from the 32-bit `numMeshes` but loops over its u16; `_rpBuildMeshAddTriangle` store order material, v0,
v1, v2, matIndex, tex, ras, pipe; `RpGeometryStreamRead` failure path F calls `RpGeometryLock(g, 0xFFF)`;
`RpLightSetColor`'s grey flag uses x87 compares (`test ah, 0x44; jp`, NaN = not equal). Bugs kept:
geometry leak when the material-list chunk is missing; `_rpMeshRead` short-read leak; the bin-mesh
stream reader overwrites an existing mesh without freeing it; `RpLightStreamRead` unclamped length (the
port overwrites its own stack instead of the return address past 0x18 bytes) and light leak on plugin
failure. Light hook 0x7521A0 is registered under the catalogue's Ghidra-name column
`(missed by ghidra: data-code-immediate)` (empty `name`).

## Verification

All game runs on lane 2 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=2`, `E2E_BIN` = a copy of the release
DLL), outputs in the worktree's `target/e2e/lane-2/port-l2-<tag>`; lists `tools/e2e/lists/rw-*.txt`
(`rw-core-all` 287, `rw-core-a` 50 = agent's, `rw-core-b` 237 = helpers', `rw-math` 28, `rw-mem-split` 22).

- **L1** (`SA_WINEPREFIX=/nonexistent sa-oracle l1 <key> --cases 1000000 --no-fixtures --threads 3`,
  specs in `crates/sa-oracle/src/specs_rw.rs`; VECTORMODULE/MATRIXMODULE patched into `.data` space
  0xB00000.., tables from the port's own fill functions): `RwSqrt`, `RwInvSqrt` (raw u32 bit patterns,
  1,002,000 cases each): **PASS, 0 mismatches** (table semantics bit-exact). 0 mismatches, INCOMPLETE only
  by their signalling-NaN cases (native backend off, as HUD/COLL/PHYS): `RwSqrtF`, `RwInvSqrtF`
  (1,000,058), `RwV3dLength`, `RwV3dNormalize` (fixed away from the zero-length error path),
  `RwMatrixDeterminant`, `RwMatrixOrthogonalError`, `RwMatrixNormalError`, `RwMatrixUpdate`,
  `RwMatrixInvert` (with the general inverse as original callee), `RwMatrixInvertGeneral`,
  `RwMatrixRotateOneMinusCosineSineReplace` (local pads ignored), `RwMatrixRotateReplace` (angles kept
  finite and within +-3600 degrees: Unicorn's FSIN is QEMU's libm and leaves infinity unchanged, the CPU
  and the `X87` model do not), `RwMatrixScale`, `RwMatrixTranslate` (ops 0..2), `RwMatrixTransformReplace`,
  `RwBBoxCalculate` (1..8 vertices): 1,060,000 cases each.
  `RwMatrixRotateReplace` is routed to L2 in the spec list (FSIN/FCOS hazard, `routes_match_hazards`);
  its 1,060,000-case Unicorn run above had 0 mismatches. SSE2 kernels: no L1 spec — Unicorn does not
  apply MXCSR.FZ to SSE results (the game runs with FZ on) and picks between two NaN operands by the x87
  rule (400-8000 mismatches per million in a trial spec, all subnormal-flush cases), and the hazard
  scanner has no SSE/FZ hazard to route them to L2 (harness owner). Their parity comes from L2 below.
- **L2, math** (`rw-math`, 28 hooks, grove walk, `L2_PER_FRAME=4`, tag `rw-math-g`, l2 extras section
  `# PORT-RW-1`): **0 mismatches and 0 undeclared** on 21 hooks (1000 cases each where called: sqrt,
  inverse sqrt, length, normalise, the dispatchers, error functions, Optimize, Update, Multiply, Invert,
  general inverse, Scale, Translate, BBox, SSE2 point/vector, the SetMultFn pair; `_rwMatrixDeterminant`
  671 of 5805 calls sampled). `RwMatrixRotateOneMinusCosineSine` (1000), `RwMatrixRotate` (336),
  `RwMatrixTransform` (75): mismatches **only in the matrix pad words** (first difference always at
  +0x1C; original = stack residue such as 0x0036FD18, port = 0), the uninitialised-stack copy described
  above — needs a normaliser rule (harness owner). `RwV3dTransformPoints/Vectors`,
  `_rwSSE2V3dTransformPoints`: 0 mismatches, undeclared writes = the `n * 12`-byte output arrays (the
  extras syntax cannot multiply; harness owner). `_rwSSE2V3dTransformVectors`: no call.
- **Split, memory and module opens** (`rw-mem-split`, 22 hooks, `SPLIT_PER_FRAME=4`, tags `rw-mem-sg`,
  `rw-mem-sg2` with the extras): every hook **PASS** (21) or NOCASES (`RwMatrixDestroy`), 0 mismatches,
  (b)/(e) splito == splitr == plain on all 5200 frames. This includes **`cb_VECTORMODULE_RwEngine_Open`:
  the PC=24 inverse-sqrt table equals the original's** (spec open question resolved).
  The first L2 attempt in `l2` (double-run) mode on allocators hung the game (the restore does not cover
  RW heap blocks); allocating functions need split runs.
- **Split, helper hooks** (`rw-core-b`, 237 hooks, `SPLIT_PER_FRAME=4`, tag `rw-b-sg`; the 19 hottest
  frame/camera/clump hooks again alone with `SPLIT_PER_FRAME=1`, tag `rw-hot-sg`): (b)/(e) splito ==
  splitr == plain on all 5200 frames. 147 PASS + 13 PASS-FP, **0 mismatches** except
  `_rwFrameListStreamRead` 0x807970 (1 of 24 cases: first difference in the caller's `RwFrameList.numFrames`
  stack word 0x36FC80, "orig wrote 7 bytes, rust 9": a write detected by value change on caller-stack
  residue; not yet confirmed either way). 7 DIVERGED (`RwStreamRead`, `_rwStreamReadChunkHeader`,
  `RwStreamFindChunk`, `RwStreamReadInt32`, `_rwStringStreamFindAndRead`, `RwTexDictionaryFindNamedTexture`,
  `RwTextureRead`): their arguments point into the caller's stack, which differs when the caller is a
  Rust port (inputs cannot be paired; 0 mismatches where paired). 6 COUNT (plain one case more than
  Rust): both spools hit the 2 GiB quota at frame 555/556 and stopped at different calls. Both runs hit the
  spool quota (`RwFrameForAllObjects` / `RwFrameGetLTM` records), so later frames are only covered by L3.
  Undeclared writes (original and Rust alike) remain for ~65 helper hooks (frame / atomic / clump /
  texture / geometry heap objects); the extras for them are not written yet.
- **Verdicts** (`docs/arch/rw-port-verdicts.tsv`, 292 rows): **L2 139** (0 mismatches and 0 undeclared in
  an l2 or split run), **implemented-needs-L2 148** (64 with no call in the grove walk; ~65 with
  undeclared writes only; 15 DIVERGED/COUNT; 3 pad-word mismatches; `_rwFrameListStreamRead`),
  **ported-unhookable 5**. Every hooked row is also part of the identical all-ports L3 runs.
- **L3, all engine ports on Rust** (`ports`, every engine crate incl. 287 RW hooks; tags `rwp2-*`):
  **IDENTICAL** to the all-original trace on grove walk 5200, pause-restart 3700, world-cheats 3276,
  c-bug 3220, anim-input 3300, weapon-combat 3500 and driving 4338 frames; no crash, panic or install
  failure. (First pass with the 235 hooks of batch 1, tag `rwp1-g`: grove IDENTICAL.) Per-hook hook-off
  runs were not made, so no row claims L3.
- Tests: `cargo test -p sa-rwcore` (list complete/unique against all other crates, globals declared once,
  initial values match the exe), `cargo test --release -p sa-hook --lib` 48/48 (keycells, INF-20 data
  reads, callers, patch_refs, catalogue/exe entry bytes); clippy `-D warnings`: sa-rwcore host
  (all targets), i686 `--features in-process`, wasm32; sa-oracle host (all targets).

## Open items / decisions needed

- Normaliser rule for the RwMatrix pad words written from uninitialised stack by
  `RwMatrixRotateOneMinusCosineSine`, `RwMatrixRotate`, `RwMatrixTransform` (and every caller copying such
  a matrix), like `RULE_FRAME_PADS` (harness owner).
- L2 extras syntax for `arg0 : arg2 * 12` (vertex arrays of the transform functions).
- L2/split evidence for the 237 helper hooks (frames, plugins, atomics, clumps, streams, textures,
  materials, geometry, meshes, lights, camera, Im3D): only L3 (all-ports identical on 7 recordings) so
  far; see the verdicts file.
- Unported: the camera clip-plane builders, `_rwD3D9ImmRenderPrimitiveNode`, the clump stream readers,
  `_rwMatrixOrthoNormalizeInternal`, RpWorld, rasters/images, RwEngine, the D3D9 render-state cache and
  Im2D.
- Phase B: freelist calls in helper modules still use `=> unported` paths for some freelist functions;
  point them at `memory.rs` when phase B starts.
