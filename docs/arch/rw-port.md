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

# PORT-RW-4: D3D9 driver layer, rasters, images, RpSkin / RpMatFX (crate `sa-rwcore`)

Agent + 2 helpers, 2026-11-11, from the 1.0 US bytes (objdump of the exe; the RW-1/2/3 specs as reading
aids). D3D9 COM methods are called through `orig!` `stdcall vslot OFF` with the original arguments in the
original order, so the D3D9 call stream (F7) is unchanged; every other callee goes through `orig!` by
address (callbacks through their stored pointers with `cdecl dyn`). Driver state (render-state caches,
stream/shader/declaration caches, light and transform caches, Im2D and VB-manager state) is accessed raw
through named constants; no new `globals!` declarations. Verdicts: `docs/arch/rw-port-4-verdicts.tsv`.

| file | contents |
|---|---|
| `d3d9state.rs` | render-state / texture-stage / sampler caches and flush, Open / Reset, `RwD3D9SetTexture`, `_rwD3D9SetTextureInternal`, materials and surface properties, `_rwD3D9RWSet/GetRenderState` and every RW render-state handler (z, stencil, shade, cull, blend, border, filter, vertex alpha), `_rwD3D9ClampRasterSize` (37) |
| `d3d9dev.rs` | FVF / declaration / shader / stream / index caches, the four draw wrappers, transforms and world matrix, light cache, vertex declarations and shaders, render targets, frustum tests, engine multisampling / refresh rate settings, the device cache reset at 0x7F9D50 (45) |
| `d3d9vb.rs` | static VB manager (per-stride free blocks), dynamic VBs and the four rotating Im buffers, `RwD3D9IndexBufferCreate` (14) |
| `im2d.rs` | Im2D open / close / line / triangle / primitive / indexed primitive and its render-state setup (7) |
| `d3d9pipe.rs` | atomic all-in-one node, default render and lighting callbacks, callback setters / getters, clipping (13) |
| `d3d9misc.rs` | dynamic-geometry list, world restore callback, default pipelines, declaration sizes, usage flags, world-sector init / lighting, VS cache purge / constant count, `RwCameraShowRaster`, node getters (20) |
| `raster.rs`, `image.rs` | helper A: RwRaster API and module, D3D9 raster driver (lock/unlock, palettes, VM raster list, format checks, native texture read/write/size), texture leftovers, anisotropy plugin; RwImage and the image/raster conversions (65 + 16) |
| `skin.rs`, `matfx.rs`, `d3d9light.rs`, `d3d9vsmat.rs` | helper B: RpSkin (29), RpMatFX (42), D3D9 lights (9), vertex-shader matrices / constants (22) |

## Findings (bytes vs specs; bugs kept)

- `RwD3D9SetRenderState` leaves EDX untouched and its callers in `_rwD3D9RenderStateOpen` (0x7FCEBE..)
  and `_rwD3D9RWSetRenderState` (0x7FE99F) keep EDX across it: hooked with `preserve = "edx"`; no
  register-comparing spec yet (**listed for the main session**).
- `RwD3D9SetSurfaceProperties`: the key compares are integer compares (not float), and the
  "ambient == 1.0" test is `cmp dword, 0x3F800000`; only the material words the original writes are
  written (diffuse alpha, specular and the ambient/emissive alphas keep their old values).
- `RwD3D9SetTexture`: the anisotropy cache / `SetSamplerState(MAXANISOTROPY)` is always stage 0 whatever
  the stage (original quirk); filter changes send MIN/MAG/MIP without comparing.
- `_rwD3D9RenderStateZTestEnable` (0x7FEA70) implements ZWRITEENABLE and 0x7FEB70 implements ZTESTENABLE
  (rw-names swapped; registered under their FUN_ names).
- `_rwD3D9RWSetRenderState` FOGTYPE with table fog stores RS 0x23 unconditionally (no compare) and
  FOGDENSITY compares on the x87 (unordered = changed); FOGTYPE / FOGDENSITY / `_rwD3D9EnableClippingIfNeeded`
  and `_rxD3D9DefaultRenderCallback` reuse their own argument slots as locals (not reproduced; their
  callers pop the arguments).
- `_rwD3D9ClampRasterSize`: power-of-two rounding of 0 gives 0x80000000, a negative size never
  terminates (kept).
- `_rwD3D9DrawIndexedPrimitiveUP` / `DrawPrimitiveUP` unbind stream 0 and the index buffer before the
  flush; `RwD3D9SetTransform(state, NULL)` uses the identity and sets the world-is-identity flag for
  WORLD; `RwD3D9SetTransformWorld` leaves the w column of its staging matrix to its static values.
- `RwD3D9CreateVertexDeclaration` keeps an unchecked `rwmalloc` copy of the elements; a released cached
  declaration is re-created on the next hit; `RwD3D9DeleteVertexDeclaration` clears the slot only when
  `Release` returns 0.
- `RwD3D9DynamicVertexBufferCreate` treats any non-zero HRESULT as failure and leaves a new descriptor
  with uninitialised fields in the list (kept). `RwD3D9EngineSetRefreshRate` reads the mode of a failed
  `EnumAdapterModes` (original: stack garbage; port: zeros).
- Im2D: the own vertex buffer exists only without hardware T&L and with more than 8 MB texture memory;
  indexed primitives with more indices than vertices go through the index buffer (at most 0x2710
  indices, else 0 after the vertices were written), otherwise the vertices are expanded.
- 0x7F9D50 (catalogue `RwD3D9GetCurrentD3DDevice`) is the device cache reset (render targets, caches,
  transform freelist, world identity block, lights) that returns the device.
- Helper A / B findings: see their sections below.

### Helper A (rasters, images, textures)

- Ported but unhooked: `RwRasterUnlock` 0x7FAEC0, `RwRasterGetNumLevels` 0x7FB160, `RwRasterCreate`
  0x7FB230, `RwRasterLock` 0x7FB2D0: sa-hook's callers test flags their callers because a later callee
  reaches `_alloca_probe` 0x822085, whose `push ecx` is a save (test false positive; **harness fix
  needed**, then the four `#[hook]` lines can be restored).
- Not ported: the D3D9 raster render / clear / destroy / create / convert functions that call internal
  helpers with register arguments (EAX/EDX/ECX/ESI: 0x4CAE40, 0x4CAE60, 0x4CAE80, 0x4CB4C0, 0x4CB4E0,
  0x4CBB00, 0x4CCE60, 0x4CD250) and the ESI-entry functions 0x4CAEA0, 0x4CB560 (not hookable with the
  current ABI); 0x4CC970 (jumps into the protection stub 0x4059B5); 0x7F3AA0 (tail `jmp eax`, argument
  count undecidable); out of time: 0x7F40F0, 0x7FF270, 0x8001E0, 0x802AF0, 0x802FD0, 0x8035C0, 0x803FE0,
  0x80C600, 0x81E720, 0x81F360, 0x820A10.
- Kept bugs: failed LockRect leaks the level surface (`_rwD3D9RasterLock`); the swap chain pointer is not
  cleared after release (VM list release); a failed native-texture write leaves the raster locked; the
  default find callback abandons a dictionary at a NULL-texture name match; `RwRasterPush/PopContext`
  have no bounds checks. Divergence: `RwRasterRead` failure path when the driver's format search fails
  (original uses an uninitialised depth word, the port 0). The anisotropy Dtor 0x749030 is a 5-byte hook
  whose steal includes its `ret` (accepted by the installer tests).

### Helper B (skin, MatFX, D3D9 lights, VS matrices)

- Not ported: `_rpD3D9SkinAllInOneNode` 0x7C7B90 (~1000 instructions).
- `RwD3D9VSGetRadiusInLocalSpace` calls `_rwInvSqrt` (0x7EDB90), not `_rwSqrt` (kept); EAX is undefined
  on some paths of `GetInverseWorldMatrix` / `GetRadiusInLocalSpace` (ported as void).
- `_rpD3D9VertexShaderUpdateFogData` with fog mode 0 uploads 4 uninitialised stack floats (port: zeros;
  ledger candidate).
- `_rwD3D9LightLocalEnable` with radius 0 returns 0 after allocating the light index; spot theta/phi
  rule; `_rwD3D9LightsEnable(0)` leaves both lists alone.
- `RpSkinCreate` no NULL check; `_rpSkinInitialize` sorts weights by signed integer compares of the float
  bits; MatFX effect searches return NULL that the setters dereference; `SetBumpMapTexture` name buffer
  interleave (30 of 32 bytes); MatFX instance callbacks are do-while over `numMeshes` (0 wraps).

## PORT-RW-4 verification (lane 4, `SA_ROOT=/home/ghosty2004/sa-engine`)

319 hooks (list `tools/e2e/lists/rw4-all.txt`), 4 ported-unhookable. Split runs on the grove walk
(`SPLIT_STACKFILL=16384`, spool quota 1-1.5 GB):

| run | hooks | result |
|---|---|---|
| `rw4-sb` (`SPLIT_PER_FRAME=2`, max 1000) | all 319 | splito / splitr traces IDENTICAL with plain (5200 frames); quota at frame 260; 85 PASS, 2 FAIL, 2 DIVERGED, 21 COUNT, rest no case |
| `rw4-sc` (max 100) | 259 not yet clean, with the new extras | IDENTICAL; quota at frame 334 (raster render wrappers); found two port bugs: the frustum tests had the plane test inverted (0x7FAD30 / 0x7FAD90), and `RwD3D9SetRenderState` must return `state` in EAX (`_rwD3D9LightsEnable` tail-returns it) — both fixed |
| `rw4-sd` (`SPLIT_PER_FRAME=1`, max 100, final build) | 221 not yet clean + the changed hooks | IDENTICAL; frustum tests / clipping / all-in-one node / lights enable now 0 mismatches (PASS-FP); quota at frame 351 |
| all ports on Rust, final build: `verify-all.sh rw4-va 4` + `rw4-va2` (air-water, first attempt skipped by the 20 GB disk guard) | every engine port | **IDENTICAL on all 11 recordings**: grove 5200, air-water 5800, anim-input 3300, car-ai 5000, c-bug 3220, driving 4338, menus-hud 5700, pause-restart 3700, police-gang 5200, weapon-combat 3500, world-cheats 3276 |

Verdicts (`docs/arch/rw-port-4-verdicts.tsv`): **L2 97** (split PASS, 0 mismatches, 0 undeclared), 
**implemented-needs-L2 222**, **ported-unhookable 4**. The 222: no case before the spool quota (most
load-time functions: plugin attach, pipelines, images, MatFX/skin setters), PASS-FP / COUNT / DIVERGED
(inputs on a Rust caller's stack, or fingerprint pairing), and these open items:

- `_rwD3D9DynamicVertexBufferRestore` 0x7F58D0 / `FUN_007f5940`: the only difference is the
  `IDirect3DVertexBuffer9*` at 0xC97BB8 (a different D3D object address per process) — needs the
  OS-object normaliser for 0xC97BB8 (harness).
- `_rxD3D9VertexShaderDefaultGetMaterialShaderCallBack` 0x761010: its callee writes the stack-address
  cell 0xC96438 (the RW-1 stack-address class).
- Undeclared writes remain for `_rwD3D9RasterLock`, `_rwD3D9NativeTextureRead` (caller-stack bytes),
  the pipeline creators, `_rwD3D9AtomicAllInOneNode`, VS matrix getters (out-parameters on the stack).
- Extras added (`tools/e2e/l2-extras.txt`, PORT-RW-4 block): 20 `doc:rw-port@` lines for the driver
  tables (each one driver table, not a whole section) and per-hook static cells for the helper modules.

## PORT-RW-4 decisions needed / items for the main session

- `RwD3D9SetRenderState` 0x7FC2D0 has `preserve = "edx"` without a register-comparing L1 spec.
- sa-hook callers test: `_alloca_probe` 0x822085 `push ecx` treated as a read (blocks 4 raster hooks).
- Normaliser for the Im vertex-buffer pointer 0xC97BB8 (OS object address).
- Split-run spool records of the raster and VS functions are large (100-260 KB); a further split run per
  load-time group (images / skin / MatFX / pipelines) with a small `--max` is needed for their L2.
- Build fix carried in this branch: `crates/sa-veh/src/vehicle2.rs` declared two `orig!` ST0 results as
  `X87` (compile error on i686 at the base commit); now `f64` + `X87::from_f64_exact` at the call sites.

### Driver hooks (PORT-RW-4 agent; `doc:rw-port` extras apply to these rows)

| address | name | file |
|---|---|---|
| 0x7FC200 | _rwD3D9RenderStateFlushCache | d3d9state.rs |
| 0x7FC2D0 | RwD3D9SetRenderState | d3d9state.rs |
| 0x7FC320 | RwD3D9GetRenderState | d3d9state.rs |
| 0x7FC340 | RwD3D9SetTextureStageState | d3d9state.rs |
| 0x7FC3A0 | RwD3D9GetTextureStageState | d3d9state.rs |
| 0x7FC3C0 | RwD3D9SetSamplerState | d3d9state.rs |
| 0x7FC400 | RwD3D9GetSamplerState | d3d9state.rs |
| 0x7FC430 | RwD3D9SetMaterial | d3d9state.rs |
| 0x7FC4A0 | RwD3D9SetClipPlane | d3d9state.rs |
| 0x7FC4D0 | RwD3D9SetSurfaceProperties | d3d9state.rs |
| 0x7FCAC0 | _rwD3D9RenderStateOpen | d3d9state.rs |
| 0x7FD100 | _rwD3D9RenderStateReset | d3d9state.rs |
| 0x7FD810 | _rwD3D9RWGetRenderState | d3d9state.rs |
| 0x7FDB80 | FUN_007fdb80 | d3d9state.rs |
| 0x7FDC30 | _rwD3D9RenderStateSrcBlend | d3d9state.rs |
| 0x7FDC80 | _rwD3D9RenderStateDestBlend | d3d9state.rs |
| 0x7FDCD0 | FUN_007fdcd0 | d3d9state.rs |
| 0x7FDE50 | _rwD3D9GetCurrentTexture | d3d9state.rs |
| 0x7FDE60 | _rwD3D9SetMaxAnisotropy | d3d9state.rs |
| 0x7FDE70 | RwD3D9SetTexture | d3d9state.rs |
| 0x7FE0A0 | _rwD3D9RenderStateVertexAlphaEnable | d3d9state.rs |
| 0x7FE190 | _rwD3D9RenderStateIsVertexAlphaEnable | d3d9state.rs |
| 0x7FE1A0 | FUN_007fe1a0 | d3d9state.rs |
| 0x7FE1F0 | FUN_007fe1f0 | d3d9state.rs |
| 0x7FE240 | FUN_007fe240 | d3d9state.rs |
| 0x7FE290 | FUN_007fe290 | d3d9state.rs |
| 0x7FE2E0 | FUN_007fe2e0 | d3d9state.rs |
| 0x7FE330 | FUN_007fe330 | d3d9state.rs |
| 0x7FE380 | FUN_007fe380 | d3d9state.rs |
| 0x7FE3D0 | FUN_007fe3d0 | d3d9state.rs |
| 0x7FE420 | _rwD3D9RWSetRenderState | d3d9state.rs |
| 0x7FEA70 | FUN_007fea70 | d3d9state.rs |
| 0x7FEB70 | FUN_007feb70 | d3d9state.rs |
| 0x7FEC70 | FUN_007fec70 | d3d9state.rs |
| 0x7FECC0 | _rwD3D9RenderStateTextureBorderColor | d3d9state.rs |
| 0x7FED00 | FUN_007fed00 | d3d9state.rs |
| 0x7FED70 | FUN_007fed70 | d3d9state.rs |
| 0x7F5EF0 | _rwD3D9SetDepthStencilSurface | d3d9dev.rs |
| 0x7F5F20 | _rwD3D9SetRenderTarget | d3d9dev.rs |
| 0x7F84E0 | RwD3D9EngineGetMaxMultiSamplingLevels | d3d9dev.rs |
| 0x7F84F0 | RwD3D9EngineSetMultiSamplingLevels | d3d9dev.rs |
| 0x7F8580 | RwD3D9EngineSetRefreshRate | d3d9dev.rs |
| 0x7F8620 | RwD3D9EngineSetMultiThreadSafe | d3d9dev.rs |
| 0x7F8630 | RwD3D9EngineSetSoftwareVertexProcessing | d3d9dev.rs |
| 0x7F9C20 | _rwDeviceGetHandle | d3d9dev.rs |
| 0x7F9C30 | RwD3D9DeviceSupportsDXTTexture | d3d9dev.rs |
| 0x7F9D30 | RwD3D9SetStencilClear | d3d9dev.rs |
| 0x7F9D40 | RwD3D9GetStencilClear | d3d9dev.rs |
| 0x7F9D50 | RwD3D9GetCurrentD3DDevice | d3d9dev.rs |
| 0x7F9E80 | RwD3D9GetCurrentD3DRenderTarget | d3d9dev.rs |
| 0x7F9E90 | RwD3D9SetRenderTarget | d3d9dev.rs |
| 0x7F9F30 | _rwD3D9SetFVF | d3d9dev.rs |
| 0x7F9F70 | _rwD3D9SetVertexDeclaration | d3d9dev.rs |
| 0x7F9FB0 | _rwD3D9SetVertexShader | d3d9dev.rs |
| 0x7F9FF0 | _rwD3D9SetPixelShader | d3d9dev.rs |
| 0x7FA030 | RwD3D9SetStreamSource | d3d9dev.rs |
| 0x7FA090 | _rwD3D9SetStreams | d3d9dev.rs |
| 0x7FA1C0 | _rwD3D9SetIndices | d3d9dev.rs |
| 0x7FA1F0 | _rwD3D9DrawIndexedPrimitiveUP | d3d9dev.rs |
| 0x7FA290 | _rwD3D9DrawPrimitiveUP | d3d9dev.rs |
| 0x7FA320 | _rwD3D9DrawIndexedPrimitive | d3d9dev.rs |
| 0x7FA360 | _rwD3D9DrawPrimitive | d3d9dev.rs |
| 0x7FA390 | RwD3D9SetTransform | d3d9dev.rs |
| 0x7FA4F0 | RwD3D9GetTransform | d3d9dev.rs |
| 0x7FA520 | RwD3D9SetTransformWorld | d3d9dev.rs |
| 0x7FA660 | RwD3D9SetLight | d3d9dev.rs |
| 0x7FA820 | RwD3D9GetLight | d3d9dev.rs |
| 0x7FA860 | RwD3D9EnableLight | d3d9dev.rs |
| 0x7FA8B0 | FUN_007fa8b0 | d3d9dev.rs |
| 0x7FAA30 | RwD3D9CreateVertexDeclaration | d3d9dev.rs |
| 0x7FAC10 | RwD3D9DeleteVertexDeclaration | d3d9dev.rs |
| 0x7FAC60 | RwD3D9CreateVertexShader | d3d9dev.rs |
| 0x7FAC90 | RwD3D9DeleteVertexShader | d3d9dev.rs |
| 0x7FACA0 | _rwD3D9SetVertexShaderConstant | d3d9dev.rs |
| 0x7FACC0 | RwD3D9CreatePixelShader | d3d9dev.rs |
| 0x7FACF0 | RwD3D9DeletePixelShader | d3d9dev.rs |
| 0x7FAD00 | _rwD3D9SetPixelShaderConstant | d3d9dev.rs |
| 0x7FAD20 | RwD3D9GetCaps | d3d9dev.rs |
| 0x7FAD30 | RwD3D9CameraIsSphereFullyInsideFrustum | d3d9dev.rs |
| 0x7FAD90 | RwD3D9CameraIsBBoxFullyInsideFrustum | d3d9dev.rs |
| 0x7FAE20 | _rwD3D9DeviceSetRestoreCallback | d3d9dev.rs |
| 0x7FAE30 | _rwD3D9DeviceGetRestoreCallback | d3d9dev.rs |
| 0x4C9970 | RwD3D9IndexBufferCreate | d3d9vb.rs |
| 0x7F5500 | RwD3D9CreateVertexBuffer | d3d9vb.rs |
| 0x7F56A0 | RwD3D9DestroyVertexBuffer | d3d9vb.rs |
| 0x7F57F0 | RwD3D9VertexBufferManagerChangeDefaultSize | d3d9vb.rs |
| 0x7F5800 | FUN_007f5800 | d3d9vb.rs |
| 0x7F5840 | _rwD3D9DynamicVertexBufferRelease | d3d9vb.rs |
| 0x7F58D0 | _rwD3D9DynamicVertexBufferRestore | d3d9vb.rs |
| 0x7F5940 | FUN_007f5940 | d3d9vb.rs |
| 0x7F5A00 | RwD3D9DynamicVertexBufferCreate | d3d9vb.rs |
| 0x7F5AE0 | RwD3D9DynamicVertexBufferDestroy | d3d9vb.rs |
| 0x7F5B10 | RwD3D9DynamicVertexBufferLock | d3d9vb.rs |
| 0x7F5C90 | RwD3D9DynamicVertexBufferUnlock | d3d9vb.rs |
| 0x7F5CB0 | _rwD3D9VertexBufferManagerOpen | d3d9vb.rs |
| 0x7F5D20 | _rwD3D9VertexBufferManagerClose | d3d9vb.rs |
| 0x7FB480 | _rwD3D9Im2DRenderOpen | im2d.rs |
| 0x7FB5F0 | _rwD3D9Im2DRenderClose | im2d.rs |
| 0x7FB810 | FUN_007fb810 | im2d.rs |
| 0x7FB650 | _rwD3D9Im2DRenderLine | im2d.rs |
| 0x7FB8E0 | _rwD3D9Im2DRenderTriangle | im2d.rs |
| 0x7FBB00 | _rwD3D9Im2DRenderPrimitive | im2d.rs |
| 0x7FBD30 | _rwD3D9Im2DRenderIndexedPrimitive | im2d.rs |
| 0x756D90 | _rwD3D9EnableClippingIfNeeded | d3d9pipe.rs |
| 0x756DF0 | _rxD3D9DefaultRenderCallback | d3d9pipe.rs |
| 0x757380 | RxD3D9AllInOneSetInstanceCallBack | d3d9pipe.rs |
| 0x757390 | RxD3D9AllInOneGetInstanceCallBack | d3d9pipe.rs |
| 0x7573A0 | RxD3D9AllInOneSetReinstanceCallBack | d3d9pipe.rs |
| 0x7573B0 | RxD3D9AllInOneGetReinstanceCallBack | d3d9pipe.rs |
| 0x7573C0 | RxD3D9AllInOneSetLightingCallBack | d3d9pipe.rs |
| 0x7573D0 | RxD3D9AllInOneGetLightingCallBack | d3d9pipe.rs |
| 0x7573E0 | RxD3D9AllInOneSetRenderCallBack | d3d9pipe.rs |
| 0x7573F0 | RxD3D9AllInOneGetRenderCallBack | d3d9pipe.rs |
| 0x757400 | _rxD3D9AtomicDefaultLightingCallback | d3d9pipe.rs |
| 0x7575F0 | _rwD3D9AtomicAllInOneNode | d3d9pipe.rs |
| 0x757890 | _rwD3D9AtomicAllInOnePipelineInit | d3d9pipe.rs |
| 0x74E970 | _rpD3D9AddDynamicGeometry | d3d9misc.rs |
| 0x74E9E0 | _rpD3D9RemoveDynamicGeometry | d3d9misc.rs |
| 0x74EA80 | _rpD3D9WorldRestoreCallback | d3d9misc.rs |
| 0x74EAD0 | _rpCreateD3D9WorldSectorAllInOnePipe | d3d9misc.rs |
| 0x74EB40 | _rpDestroyD3D9WorldSectorPipe | d3d9misc.rs |
| 0x74EB80 | _rpCreateD3D9AtomicAllInOnePipe | d3d9misc.rs |
| 0x7522E0 | _rpD3D9VertexDeclarationGetSize | d3d9misc.rs |
| 0x7522F0 | _rpD3D9VertexDeclarationGetStride | d3d9misc.rs |
| 0x7588B0 | RpD3D9GeometrySetUsageFlags | d3d9misc.rs |
| 0x7588D0 | RpD3D9GeometryGetUsageFlags | d3d9misc.rs |
| 0x7588E0 | RpD3D9WorldSectorSetUsageFlags | d3d9misc.rs |
| 0x758900 | RpD3D9WorldSectorGetUsageFlags | d3d9misc.rs |
| 0x75E540 | _rwD3D9WorldSectorAllInOnePipelineInit | d3d9misc.rs |
| 0x75E570 | _rxD3D9WorldSectorDefaultLightingCallback | d3d9misc.rs |
| 0x75EDD0 | _rpD3D9GetNumConstantsUsed | d3d9misc.rs |
| 0x75EE60 | _rpD3D9VertexShaderCachePurge | d3d9misc.rs |
| 0x761010 | _rxD3D9VertexShaderDefaultGetMaterialShaderCallBack | d3d9misc.rs |
| 0x7EE370 | RwCore::RwCameraShowRaster | d3d9misc.rs |
| 0x80E990 | RxNodeDefinitionGetD3D9ImmInstance | d3d9misc.rs |
| 0x80E9A0 | RxNodeDefinitionGetD3D9SubmitNoLight | d3d9misc.rs |

# PORT-RW-5: writer pass (unverified batches)

WRITER-RW-5 + 2 helpers, 2026-11-11, from the 1.0 US bytes, under the writer/verifier split (docs/PLAN.md
§5.1): every hook below is `#[hook(unverified = ...)]` and is installed only when a run selects its
batch (docs/verify/QUEUE.md). Same conventions as PORT-RW-4 (D3D9 COM calls through `orig!` `stdcall
vslot` in the original argument order; every other callee through `orig!`; argument slots the original
reuses as locals are not reproduced, since the callers pop them).

| batch | file | contents |
|---|---|---|
| `rw5-raster` | `raster.rs` | the four raster hooks RW-4 left unhooked (RwRasterUnlock / GetNumLevels / Create / Lock), after the callers-test fix |
| `rw5-node-1` | `d3d9node.rs` | world-sector all-in-one node, atomic node-definition getter, `_rpD3D9GetVertexShader` (VS cache), default VS begin / mesh render callbacks, UV-anim matrix upload |
| `rw5-native-1` | `d3d9native.rs` | native-data geometry stream read / write / size, world-sector size, the plugin's two stream-read callbacks |
| `rw5-cam-1` | `d3d9cam.rs` | `_rwD3D9CameraEndUpdate`, `RwD3D9ChangeVideoMode` |
| `rw5-image-1` | `image2.rs` (helper B) | `_rwTextureDefaultReadCallBack`, `_rwD3D9ImageGetRaster`, `_rwD3D9RasterSetImage`, `RwImageApplyMask`, `RwImageRead`, `RwImageReadMaskedImage`, `RwImageResample`, `_rwD3D9DecompressDXT1`, `RwD3D9DDSTextureRead` |
| `rw5-core-1` | `core5.rs` | versioned chunk header, atomic / geometry / material-list / texture-dictionary / frame-list stream sizes and small readers / writers, rights-to-render writers, string helpers, MatFX engine close / material dtor / atomic and sector stream callbacks |
| `rw5-inst-1` | `d3d9inst.rs` (helper A) | the vertex-declaration instancing family 0x752320..0x755AA0 and the default instance / reinstance callbacks |
| `rw5-core-2` | `core5.rs` | world / atomic iterators, `RpLightSetConeAngle`, mesh header destroy / size, camera / matrix / immediate module close, `RwCameraClear`, engine getters / focus, the default CRT memory wrappers |

## Harness fix (sa-hook callers test)

`push ecx` / `push edx` that saves the register and gets it back is not a read: besides pop and
`xchg [esp], r`, a `mov r, [slot]` into the pushed register counts as the restore, the slot stays
tracked through ESP aliases after an ESP write that has an alias (`_alloca_probe` 0x822085: `push ecx;
...; mov eax, esp; ...; mov esp, ecx; mov ecx, [eax]`), and a state (address, slot, aliases) seen before
is covered (its page-walk loop). Unit test `push_saved_and_restored_is_not_a_read`.

## Findings (bugs kept)

- `_rpD3D9GetVertexShader`: a full cache (0x100) overwrites the entry with the smallest frame stamp in
  place, breaking the sorted order the binary search relies on.
- `_rxD3D9VertexShaderDefaultBeginCallBack`: `RwRenderStateGet(FOGTYPE)` writes into the original's
  own `desc` argument slot (port: a local initialised with the slot's value).
- Native-data geometry write: on any failure the meshes keep material indices instead of pointers.
  Read: no NULL check on the entry allocation; after one stream succeeded, a failed vertex-buffer
  creation of the next still locks / reads it; the baseIndex loops are do-whiles over `numMeshes`.
- `RwD3D9ChangeVideoMode`: `SetWindowPos` gets the bottom as `y` (ignored: SWP_NOMOVE).
- `_rwD3D9CameraEndUpdate`: a failed surface query leaves the surface local uninitialised (port 0).
- `RpAtomicStreamWrite` / `RpAtomicStreamRead` / the two engine getters: uninitialised stack words
  passed to the callee (port 0; hazard `uninit-locals`).
- `RpWorldForAllWorldSectors` reads the never-written stack slot 0 after the last sector.
- Helper B (images): `_rwD3D9ImageGetRaster` returns 1 after its format errors; `_rwD3D9RasterSetImage`
  does not check the resample result, leaks the image when a lock fails and keeps the pixel lock when
  the palette lock fails, a 16-bit raster with a 4-bit image indexes a 16-entry table with the whole
  pixel byte (stack garbage in the original, zeros in the port); `_rwTextureDefaultReadCallBack` points
  the image palette at a stack buffer; `RwImageApplyMask` writes through NULL when its helpers fail
  (kept as the crash); `RwD3D9DDSTextureRead` ignores `maskName`, its 256-byte path buffer has no bound,
  and a failed `RwTextureCreate` leaks the raster. `RwImageResample` follows the x87 sequence under
  PC=24 (only final values, not the scratch slots).
- Helper A (instancing): GetMinMaxValues skip the last element (the loop starts at 0 and runs n-1
  times), a value that lowers min is not tested against max, NaN never updates; InstWeights FLOAT3
  copies 16 bytes; the block copies have no `n` test (negative n wraps); UnInst decodes USHORT*N with
  `movsx`; InstTangent has no zero-length guard and odd strip triangles swap A and C; the default
  instance callback's single-stream `offset / stride` faults on stride 0 (Rust panics). Float→int is
  `fistp` under the game's control word (`fist` helper: round to nearest even, invalid → 0x80000000).

## Not done / skipped

| address | function | reason |
|---|---|---|
| 0x7C7B90 | `_rpD3D9SkinAllInOneNode` | ~1000 instructions; not reached in the time budget |
| 0x761170 | `_rxD3D9VertexShaderDefaultLightingCallBack` | ~670 instructions with FCOS and four helper calls; not reached |
| 0x761000 | `_rxD3D9VertexShaderDefaultEndCallBack` | a single `ret` (too short to hook) |
| 0x75E5E0 | `_rxD3D9WorldSectorDefaultInstanceCallback` | helper A ran out of time |
| 0x81F360 | `RwD3D9RasterStreamReadDDS` | ~1800 instructions, 66 calls; not reached (helper B) |
| 0x763F20, 0x7641C0 | native-data world-sector stream write / read | not reached (same shape as the geometry pair, with the world's material list) |
| 0x7F5F70 | `D3D9System` | 23-way switch, 2 KB, 0x470-byte frame; not reached |
| 0x7F8F20 | `_rwD3D9CameraBeginUpdate` | 2.4 KB; not reached |
| 0x80E270 | `_rwD3D9ImmRenderPrimitiveNode` | not reached |
| 0x7590E0 | `_rpMeshGetNextSerialNumber` | 16-bit return with the caller's upper EAX bits (`ax-al-return`); left for an `entry_eax` port |
| 0x75E9F0, 0x803FE0 | node getter, `RwImageSetGamma` | taken by another branch (worktree-agent-a4f0…, …a76f…) |

Register-entry functions: none in these groups (no `entry_eax` / ESI ports needed).
