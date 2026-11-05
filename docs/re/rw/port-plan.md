# RW port order (input for ROADMAP row RW)

Full table: `docs/re/rw/port-order.csv` (`level,address,name,module,class`), one row per `rw-names.csv`
function, sorted leaf-first.

## How it was computed

* **Level** = longest call chain to a leaf inside the RW set (level 0 calls no other RW-set function;
  cycles are cut where they close). Edges come from the decompilation of every function (direct calls,
  including calls the catalogue does not have because the callee is not a catalogue row). Calls through
  function pointers (std-funcs, RwDevice slots, plugin callbacks, node callbacks, engine memory hooks) are
  *not* edges; they make the caller stateful instead.
* **Class**:
  * `L1` = L1 candidate (PLAN §5): no access to `.data/.bss` globals (constants in `.rdata` allowed), no
    indirect calls, no D3D9 device calls, and every RW callee is itself L1 (CRT math/`qsort` allowed).
    Pointer arguments are allowed, so the L1 harness must generate the pointed-to structs.
  * `L2 (reasons)` = stateful: `globals` (reads/writes RW or game globals, including plugin offsets and
    `RwEngineInstance`), `indirect-calls` (memory hooks, std-funcs, callbacks), `D3D9 device`
    (`IDirect3DDevice9*` 0xc97c28 / `IDirect3D9*` 0xc97c20), `calls stateful`.
  * `unknown` = no decompilation available (24 game-side callbacks and two driver entries reached only from
    data; decompile them before porting).
* This is a static heuristic. A function that only reads a plugin offset global (most plugin
  callbacks) is classed L2 here but becomes L1-testable once the offset is fixed in the harness.

| level | L1 | L2 | total (incl. unknown) |
|---|---|---|---|
| 0 | 346 | 550 | 920 |
| 1 | 15 | 337 | 352 |
| 2 | 0 | 184 | 184 |
| 3 | 0 | 137 | 137 |
| 4 | 0 | 86 | 86 |
| 5 | 0 | 67 | 67 |
| 6 | 0 | 48 | 48 |
| 7 | 0 | 22 | 22 |
| 8–11 | 0 | 13 | 13 |

| module | L1 | L2 | unknown |
|---|---|---|---|
| core | 100 | 332 | 0 |
| pipeline | 26 | 59 | 0 |
| world | 61 | 294 | 0 |
| d3d9-driver | 80 | 243 | 2 |
| anim | 12 | 53 | 0 |
| skin | 28 | 49 | 0 |
| matfx | 1 | 72 | 0 |
| uvanim | 14 | 28 | 0 |
| toolkit | 5 | 44 | 0 |
| game-plugin | 30 | 197 | 21 |
| game-pipeline | 4 | 73 | 1 |

(L2 reasons over all rows: globals 1,147, indirect calls 504, calls-stateful 240, D3D9 device 100; a row can
have several.)

## Recommended port sequence

The engine is one state machine around `RwEngineInstance` (0xc97b24) and the plugin registries, so the
order is by layer, leaf-first inside each layer:

1. **Math kernels (L1).** `_rwMatrixMultiplyDefault` 0x7f12f0, `_rwMatrixDeterminant` 0x7f1450, the
   matrix error/orthonormal helpers 0x7f14a0–0x7f1590, `_rwMatrixInvertOrthoNormalizedOrGeneral`
   0x7f2160, `_rwMatrixToQuat` 0x7f2930, `RwMatrixUpdate` 0x7f18a0, the C transform defaults
   0x7ed670/0x7ed730/0x7ed7d0/0x7ed880, **and the variant the oracle actually runs** (x87 `_rwcseg`
   0x857000–0x8572a0 or SSE/SSE2 0x80d070–0x80d600, README §5), `RtQuat*` (0x7eb6a0..), `RtSlerp`.
   These need `sa_math::x87` where the x87 kernel is the oracle path.
2. **Memory, lists, strings, errors.** Freelists (`_rwFreeListCreateInternal` 0x8019b0,
   alloc/free 0x801c30/0x801d50), `_rwMemoryOpen/Close` 0x801fd0/0x8020f0, `_rwerror` 0x8088d0,
   `RwErrorSet/Get`, string functions 0x80a350–0x80a420, `_rwFileSystemOpen` 0x804140, resource arena
   0x80ffe0–0x810140 and `RwResourcesAllocateResEntry/FreeResEntry`. Allocation hints are part of the ABI
   (README §4) and must be passed through unchanged.
3. **Plugin registry** 0x808430–0x808c10 and the default identity callbacks 0x8086b0–0x8086d0. Port with
   the exact registration order of `plugins.md`; offsets must come out identical (measure first).
4. **Streams and chunks.** `RwStream*` thunks, `_rwStreamReadChunkHeader`, `_rwChunkIsComplex`
   0x7ed1e0, `RwStreamFindChunk`, versioned chunk writers. FMT-1's chunk reader must follow these.
5. **Objects: frames, cameras (CPU side), images, textures, tex dictionaries.** `_rwFrameInternalInit`
   0x7f0360, frame clone/destroy recursion 0x7f0050/0x7f0830, dirty-sync 0x809700/0x809780/0x809850,
   `_rwFrameListFlattenRecurse` 0x8076a0; camera frustum 0x7ee200–0x7ef370; image read/path/mask/
   resample 0x8030c0–0x803950, 0x80ca20, palette quantiser 0x80ad90–0x80c5a0; texture read path
   0x7f3ac0 + defaults 0x7f40f0–0x7f53d0.
6. **Engine life cycle** `RwEngineInit/Open/Start/Stop/Close/Term` with module Open/Close callbacks in
   registration order (`plugins.md`), `_rwDeviceSystemRequest` 0x7f2ab0.
7. **D3D9 driver.** Device system 0x7f5f70 and helpers (0x7f6bf0, 0x7f7100, 0x7f7540, 0x7f8300,
   0x7f7f70), standard functions (`d3d9.md` §2), render-state cache 0x7fcac0–0x7fed70, rasters
   0x4c9970–0x4cd820 and converters 0x7ff450–0x801390, dynamic/static VB manager 0x7f5500–0x7f5d20,
   Im2D 0x7fb480–0x7fbd30, Im3D 0x80dfb0–0x80e9a0, lights 0x755b90–0x755ce0. This layer is verified
   with the D3D call diff (ROADMAP: "L2 + D3D diff"), not by L1.
8. **Rx pipeline core.** Heap 0x8098b0–0x809f90, pipeline lock/unlock/topsort 0x8050c0–0x8067b0 and
   0x80f0d0–0x8103d0, `RxLockedPipeAddFragment` 0x806be0, cluster functions.
9. **RpWorld.** Geometry/material/mesh/tri-strip builders (0x7591d0–0x75e230, mostly L1/L2 data
   transforms), atomic/clump/light (including stream read/write and the world-object sync hooks
   0x750430–0x751630), world/sector BSP streaming 0x762040–0x763310, default pipelines 0x75e310.
10. **D3D9 instancing and default nodes.** `_rpD3D9VertexDeclarationInst*` (L1), `_rpD3D9InstanceMeshes`
    0x756960, atomic/sector all-in-one nodes 0x7575f0/0x75e3f0 and their default callbacks, vertex-shader
    generation 0x75eed0/0x75f0b0 (needs D3DX `D3DXAssembleShader` in phase A), native-data streams
    0x7637f0–0x7645c0.
11. **Toolkits/plugins**: RtAnim + RpHAnim (anim), RpSkin (+ D3D9 skin pipeline 0x7c7b90–0x7cb8b0),
    RpMatFX (0x810aa0–0x816290), RpUVAnim, RpAnisot, RtBMP/RtPNG (libpng stays original in phase A),
    RtDict.
12. **Game side** (`game-plugin`, `game-pipeline`): ported with their gta-reversed sources once the RW API
    they call exists; their callbacks are reached through RW (render callbacks, plugin ctors/streams).

Every step keeps the original as oracle (hooks on original vs Rust); D3D-touching functions need the D3D
call recorder for L2.
