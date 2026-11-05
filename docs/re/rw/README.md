# RenderWare in gta_sa.exe 1.0 US (task RW-RE)

Static reverse-engineering spec of the RenderWare 3.6 (D3D9 build) code linked into `gta_sa.exe` 1.0 US.
It is the input for porting RW (ROADMAP row `RW`). Every claim carries an address. Nothing here was
measured at run time: items marked *derived* are read from the code and still need a measurement before
an implementation relies on them (PLAN §4b).

Files:

| File | Contents |
|---|---|
| `README.md` (this file) | method, counts, engine init/open/start sequence, memory and freelists, CPU/FPU setup, open questions |
| `plugins.md` | plugin registry layout and the offset of every plugin the game registers (IDs, sizes, ctor/dtor/stream callbacks) |
| `objects.md` | core object layouts (RwObject, RwFrame, RpAtomic, RpClump, RpGeometry, RpMaterial, RwTexture, RwRaster + D3D9 extension, RwCamera, RpLight, RpWorld, sectors, RwGlobals, RwDevice), cross-checked with the exe |
| `d3d9.md` | D3D9 driver: device system, standard functions, render-state cache, rasters, resEntry/instancing path, D3D9 pipelines (atomic, sector, skin, MatFX, Im2D/Im3D), the game's custom pipelines, D3DX9 and libpng/zlib entry points |
| `port-plan.md` | leaf-first port order, L1 (pure) vs L2 (stateful) classes |
| `../../../catalog/rw-names.csv` | one row per RW function: `address,name,module,evidence,confidence` |

## 1. Method and sources

1. **Thunk names (862 addresses).** gta-reversed `source/game_sa/RenderWare/rw/*.cpp` (raw thunks
   `((R(__cdecl*)(..))0xADDR)(..)` and 5 `RH_ScopedGlobalInstall` hooks) and plugin-sdk
   `plugin_sa/game_sa/RenderWare.cpp` were parsed into address/name/signature. The two sources never
   disagree on a name (0 conflicts). 849 addresses are named by both (confidence `high`), 11 only by
   plugin-sdk and 2 only by gta-reversed (`medium`). All 862 are function starts in the catalogue
   (851 `CALLED-ONLY`, 5 `RW`, 1 `STREAM`). One plugin-sdk name is wrong: plugin-sdk calls 0x4c7510
   `RwTextureRead`, gta-reversed hooks it as `CVehicleModelInfo::FindTextureCB`; the real
   `RwTextureRead` is 0x7f3ac0 (see open questions).
2. **Decompilation.** Ghidra 12.1.2 headless (`tools/catalog/ghidra/DecompileList.java`, read-only, on a
   private copy of the analysed project) over every RW catalogue row and every thunk, then closed over
   all code addresses those functions reference (`&LAB_`, `UNK_`, `func_0x` targets, plus aligned
   pointers into the RW span found in `.rdata/.data/_rwdseg`). 1,866 functions were decompiled; the
   closure stops at D3DX9/libpng/CRT (excluded).
3. **Plugin registration.** Every `Rw*/Rp*RegisterPlugin(size,id,ctor,dtor,copy)`,
   `*RegisterPluginStream(id,read,write,getSize)` and `_rwPluginRegistryAddPlugin(&registry,..)` call was
   extracted; callbacks are named `cb_<PLUGINID>_<Owner>_<Role>` (role is certain from the API slot,
   plugin ID names from `rwplcore.h`/`rpworld.h`/toolkit headers). 155 callbacks.
4. **D3D9 driver tables.** The standard-function table filled by `_rwD3D9SetStandardFunctions` 0x7f72e0
   (27 `(index,fn)` pairs, read from the assembly 0x7f72fb–0x7f74e6) and the `RwDevice` block at
   .data 0x8e2498 (`fpSystem`, render-state set/get, Im2D) give 33 driver names by slot.
5. **Remaining functions** were named by reading the decompilation (callers, callees, chunk IDs,
   error codes, D3D vtable slots, header-declared but un-thunked API names such as `_rwerror`,
   `RpLightStreamWrite`, `_rpGeometryAddRef`, `RwD3D9CreateVertexShader`, `_rwIntel*supported`) and the
   names gta-reversed's `app/platform/win/WindowedMode.cpp` gives to driver functions it calls
   (`D3D9DeviceReleaseVideoMemory` 0x7f7f70, `_rxD3D9VideoMemoryRasterListRestore` 0x4cc970, ...).
   Names of RW *static* functions are not public: they are descriptive, `_rw/_rp/_rx` prefixed, and their
   confidence is `medium` (role proven by code/registration) or `low` (role read from the body only).

Confidence scale in `rw-names.csv`: `high` = two independent name sources agree, or the name follows from
an API slot (plugin callback role, std-func index, RwDevice slot, string-named node definition) or a
header declaration whose body matches; `medium` = role proven by structure, name descriptive or from one
source; `low` = role read from the body only.

## 2. Counts (`catalog/rw-names.csv`, 1,829 rows)

| module | high | medium | low | total | of which not in catalogue |
|---|---|---|---|---|---|
| core | 316 | 66 | 50 | 432 | 54 |
| world | 279 | 47 | 29 | 355 | 71 |
| pipeline | 58 | 4 | 23 | 85 | 4 |
| d3d9-driver | 187 | 56 | 82 | 325 | 56 |
| anim | 63 | 2 | 0 | 65 | 12 |
| skin | 35 | 7 | 35 | 77 | 33 |
| matfx | 53 | 9 | 11 | 73 | 21 |
| uvanim | 36 | 1 | 5 | 42 | 7 |
| toolkit | 39 | 8 | 2 | 49 | 12 |
| game-plugin | 246 | 2 | 0 | 248 | 20 |
| game-pipeline | 77 | 1 | 0 | 78 | 1 |
| **all** | 1389 | 203 | 237 | 1829 | 291 |

* All 651 catalogue rows with subsystem `RW` (state snapshot taken 2026-11-05 08:35) are named, including
  the 332 `unknown` ones.
* The 862 thunk addresses (catalogue subsystem `CALLED-ONLY` for 851 of them) are named.
* 291 functions were **not in the catalogue snapshot** this task started from (plugin/stream callbacks, node
  bodies, std-func slots, pixel converters, callbacks only reachable through pointers; marked
  `[not in catalogue]` in the evidence column). The concurrent CAT-12b regeneration (`data_refs.py`) has since
  added 290 of them to `catalog/procedures.csv`; only 0x7fff00 (`_rwD3D9ImageFindRasterFormat`, std-func slot 9)
  is still missing there.
* `game-plugin` / `game-pipeline` are the Rockstar side of the `RW` subsystem (CVisibilityPlugins,
  RpAnimBlend, NodeName, Collision, Breakable, 2dEffect, JPeg, RwHelper, CClothesBuilder, RenderBuffer;
  CCustomBuilding(DN)Pipeline, CCustomCarEnvMapPipeline, CCarFXRenderer). Their names come from
  gta-reversed. They are listed so the RW port sees every caller of the RW API.
* Rejected candidates: 0x817530, 0x8175e0, 0x8194d0 (aligned data values that point into code but
  decompile to garbage; not functions). 26 libpng/zlib and 1 D3DX9 function reached by the closure stay
  `excluded`.

## 3. Engine life cycle as the game drives it

`RsRwInitialize` 0x619c90 (gta-reversed hook; decompiled and checked):

1. `psGetMemoryFunctions` 0x745510, then **`RwEngineInit` 0x7f3170** (memFuncs, flags 0, resource arena
   0x400000):
   * engine pointer `RwEngineInstance` (.bss 0xc97b24) = static globals block 0xc979c8;
   * flag bit 0 selects the freelist/memory hint shims: clear → `_rwFreeListAllocReal` 0x801c30 /
     `_rwFreeListFreeReal` 0x801d50 into 0xc97b0c/0xc97b10; set → `_rwMallocBypass` 0x7f3490 /
     `_rwFreeBypass` 0x7f34b0; `_rwFreeListSetFlags` 0x801970;
   * `_rwStringOpen` (thunk) → `_rwMemoryOpen` 0x801fd0 (default malloc/realloc/calloc 0x8020a0/0x8020b0/
     0x8020d0, freelist-of-freelists 0x24 × 0x10) → `_rwFileSystemOpen` 0x804140 (CRT `fopen/fclose/
     fread/fwrite/fgets/...` into `RwGlobals.fileFuncs` +0xc4, exists-test 0x804200) →
     `_rwResourcesOpen` 0x8082e0 (plugin-registry entry freelist, 0x3c-byte entries);
   * 14 core modules are added to the engine registry 0x8e2298 with `_rwPluginRegistryAddPlugin`
     0x8084a0 in this order: ERROR 0x40f, VECTOR 0x401, COLOR 0x40d, MATRIX 0x402, FRAME 0x403,
     STREAM 0x404, CAMERA 0x405, IMAGE 0x406, RASTER 0x407, TEXTURE 0x408, PIPE 0x409,
     CHUNKGROUP 0x412, (`_rwPipeAttachDummy` 0x807c70), IMMEDIATE 0x40a, RESOURCES 0x40b (offsets in
     `plugins.md`);
   * `_rwDeviceRegisterPlugin` 0x7f5f60 (jmp to `_rwD3D9RasterPluginAttach` 0x4c9ab0): RwRaster plugin
     0x40c size 0x24 (the D3D9 raster extension, offset stored in 0xb4e9e0) and the
     `_rwD3D9PixelFormatInfo` table 0xb4e7e0; engine status (+0x150) = 1.
2. `AppEventHandler(rsINITDEBUG)`, `psInstallFileSystem`, **`AppEventHandler(rsPLUGINATTACH)` →
   `App::PluginAttach` 0x53d870**: RpWorld, RpSkin, RtAnimInitialize, RpHAnim, NodeName, CVisibilityPlugins,
   RpAnimBlend, CTxdStore::PluginAttach 0x731650, RpMatFX, RpUVAnim, CCustomBuildingRenderer,
   CCarFXRenderer::RegisterPlugins, RpAnisot (return value ignored), Breakable, CCollisionPlugin,
   C2dEffect, PipelinePlugin. Order checked against the decompiled 0x53d870 (it matches gta-reversed
   `app.cpp:141`).
3. `rsINPUTDEVICEATTACH`, then **`RwEngineOpen` 0x7f2f70** (`{displayID = HWND}`):
   `_rwDeviceGetHandle` 0x7f9c20 returns the D3D9 `RwDevice` at .data 0x8e2498; the engine instance is
   allocated with `memFuncs.rwmalloc(registry.sizeOfStruct = 0x5b0 derived, hint 0x40000)` and the 0x158
   static bytes are copied in; device system calls (`_rwDeviceSystemRequest` 0x7f2ab0):
   `REGISTER (4)` copies the 14-dword RwDevice into `RwGlobals.dOpenDevice` (+0x10), `OPEN (0)` creates
   `IDirect3D9` (`Direct3DCreate9` import), enumerates adapters/modes, picks the first usable adapter,
   probes CPU features, `STANDARDS (0xb)` fills `stdFunc[29]` (+0x48). Status 2.
4. `rsSELECTDEVICE` / `psSelectDevice` (video mode), then **`RwEngineStart` 0x7f2e70**:
   `START (2)` → `D3D9System` 0x7f5f70 case 2: present parameters (`D3D9SetPresentParameters` 0x7f6cb0),
   behaviour flags from caps (SW 0x20 / HW 0x40 / HW|PURE 0x50), `IDirect3D9::CreateDevice` (vtbl+0x40;
   gta-reversed's windowed-mode hook sits at 0x7f6781), default render target / depth surface, then
   `_rwD3D9RenderStateCacheInvalidate` 0x7f6b00, `_rwD3D9DeviceLightCacheOpen` 0x7f6b40,
   `_rxD3D9VideoMemoryRasterListCreate` 0x4cc150, `_rwD3D9Im2DRenderOpen` 0x7fb480,
   `_rwD3D9RenderStateOpen` 0x7fcac0, `_rwD3D9VertexBufferManagerOpen` 0x7f5cb0.
   Then `_rwPluginRegistryInitObject(engine registry, RwEngineInstance)` runs every module/plugin
   **Open** callback in registration order (`cb_*_RwEngine_Open`), `RwImageSetGamma`, and
   `FINALIZESTART (0x11)`: `_rwProcessorInitialize` 0x857404 (x87 control word → single precision,
   exceptions masked; old word saved at `_rwdseg` 0xcaf004), MXCSR flush-to-zero (bit 0x8000) if SSE,
   `_rwMatrixSetMultFn` 0x7f12c0 and `_rwVectorSetMultFn` 0x7ed5f0 select the transform kernels (§5).
   Status 3.
5. `rsREGISTERIMAGELOADER`, `RwD3D9DeviceSupportsDXTTexture`, `RwTextureSetMipmapping(TRUE)`,
   `RwTextureSetAutoMipmapping(FALSE)`.
6. `CGame::InitialiseRenderWare` 0x5bd600: `CGame::ValidateVersion`, 0x730830 (D3DResourceSystem init),
   `CTxdStore::Initialise`, `CVisibilityPlugins::Initialise`, `RwCameraCreate` + `RwFrameCreate` +
   camera raster (`RwRasterCreate(w,h,0,rwRASTERTYPECAMERA)` → camera +0x60) and z-raster (type 1 →
   +0x64), far 2000.0 / near 0.9, `RpWorldCreate(bbox ±10000)`, `RpWorldAddCamera`, lights, fonts,
   HUD, `_rwD3D9DeviceSetRestoreCallback(0x5ba120)` after saving the old one in 0xbd0114.

Shutdown: `RwEngineStop` → `D3D9DeviceSystemStop` 0x7f7100 (`STOP 3`), `RwEngineClose` 0x7f2f00
(`CLOSE 1`, frees the instance, restores the static block), `RwEngineTerm` 0x7f3130 (`_rwResourcesClose`
0x808320, `_rwFileSystemClose` 0x804240, `_rwMemoryClose` 0x8020f0).

Per frame the camera path is `RwCameraBeginUpdate` (gta-reversed hook 0x7ee190) → camera `beginUpdate`
(+0x18): `_rpWorldObjCameraBeginUpdate` 0x750430 (sets `RwGlobals.curWorld`, ++renderFrame) →
`_rwCameraDefaultBeginUpdate` 0x7ef370 → `stdFunc[1]` = `_rwD3D9CameraBeginUpdate` 0x7f8f20, which
calls `_rwProcessorForceSinglePrecision` 0x857432 every frame. Device loss is handled in
`_rwD3D9TestState` 0x7f8300 (TestCooperativeLevel → `D3D9DeviceReleaseVideoMemory` 0x7f7f70 → Reset →
restore chain `_rxD3D9VideoMemoryRasterListRestore` 0x4cc970, `_rwD3D9DynamicVertexBufferRestore`
0x7f58d0, `_rwD3D9RenderStateReset` 0x7fd100, Im2D/Im3D open, restore callback).

## 4. Memory and freelists

* `RwGlobals.memoryFuncs` (+0x134 malloc, +0x138 free, +0x13c realloc, +0x140 calloc) come from the game
  (`psGetMemoryFunctions` 0x745510). Every RW allocation passes a hint (`0x3000e` frame, `0x30116` skin,
  `0x3010d` PNG, `0x40000` engine/registry, `0x1040409` pipeline, ...); the hint is ignored by the
  default functions but is part of the call ABI.
* `RwGlobals.memoryAlloc/memoryFree` (+0x144/+0x148) are the freelist alloc/free used everywhere
  (`(**(code **)(RwEngineInstance + 0x144))(freelist, hint)`).
* `RwFreeList` (0x24 bytes) is created by `_rwFreeListCreate` 0x801980 /
  `RwFreeListCreateAndPreallocateSpace` 0x801b70, both through `_rwFreeListCreateInternal` 0x8019b0; the
  global list of freelists starts at 0xc9a604, its own freelist at 0xc9a634; `RwFreeListForAllUsed`,
  `RwFreeListDestroy` are thunks. Freelists created by RW at start-up (entry size × count): registry
  entries 0x3c × [0x8e26ac]; video raster list 8 × 0x7f (0xb4ea00); palettes 0x404 × 0x40 (0xb4e9f8);
  VB manager 0x10×0x10, 0x14×100, 8×100, 0x14×0x2a (0xc97b80..0xc97b90); D3D lights 0x40 × 0xf align 0x10
  (0xc98080).
* Resource arena: `RwResourcesAllocateResEntry` 0x807ed0 / `RwResourcesFreeResEntry` 0x807de0 over an
  arena initialised by `_rwResourcesArenaInit` 0x80ffe0 (32-byte aligned blocks, header 0x20 before each
  `RwResEntry`; helpers 0x810040, 0x8100c0, 0x810140). Arena size 0x400000 from `RwEngineInit`.

## 5. CPU and FPU state (parity-relevant)

* `D3D9System` OPEN probes `_rwIntelMMXsupported` 0x80dbe0, `_rwIntelSSEsupported` 0x80dc40,
  `_rwIntelSSE2supported` 0x80dca0, `_rwAMD3DNowSupported` 0x80dd00 into 0xc980a0..0xc980ac.
* FINALIZESTART picks the `RwV3dTransformPoint(s)/Vector(s)` kernels: no SSE → `_rwcseg` x87 assembly
  (0x857000, 0x85707f, 0x8570ef, 0x857160; matrix multiply 0x8572a0); SSE2 without 3DNow → 0x80d440 /
  0x80d4e0 / 0x80d570 / 0x80d600; otherwise SSE → 0x80d070 / 0x80d180 / 0x80d280 / 0x80d370. The C
  defaults 0x7ed670/0x7ed730/0x7ed7d0/0x7ed880 and `_rwMatrixMultiplyDefault` 0x7f12f0 are only used
  before start. **The kernel used on the reference machine decides the float results**: the Rust port must
  reproduce the variant the oracle ran (measure which pointer is installed in vector module +8..+0x14,
  engine offset 0x160).
* `_rwProcessorInitialize` 0x857404 forces x87 24-bit precision at start and
  `_rwProcessorForceSinglePrecision` 0x857432 re-forces it in every camera begin-update (this is the
  "x87 at 24 bits" assumption in PLAN §5). `_rwProcessorRelease` 0x85742b restores the word at stop.

## 6. Differences against gta-reversed / catalogue

* **`RwGlobals` in gta-reversed is missing `RwFileFunctions fileFuncs`** (0x2c bytes at +0xc4). gta-reversed
  `rwplcore.h:6280` gives `sizeof(RwGlobals)` = 0x12c; the exe's engine registry starts at 0x158
  (.data 0x8e2298) and the code uses +0xc4..+0xf0 for file functions (0x804140), +0xf8/+0xfc/+0x10c/+0x120
  for string functions and +0x134.. for memory functions. plugin-sdk's header (0x158) matches the exe.
  Any gta-reversed code that reads `RwEngineInstance->memoryFuncs` etc. through its header is off by 0x2c.
* `catalog/layouts.csv` has no RenderWare struct rows (only `D3DIndexDataBuffer`, `D3DTextureBuffer`,
  `RpHAnimBlendInterpFrame`, `RwObjectNameIdAssocation`). The layouts in `objects.md` come from the RW
  SDK headers (plugin-sdk and gta-reversed copies, compiled for i686 with clang record-layout dumps;
  both agree on every RW struct except `RwGlobals`) and are checked against the exe (registry base sizes,
  field uses quoted per struct).
* plugin-sdk mislabels 0x4c7510 as `RwTextureRead` (gta-reversed: `CVehicleModelInfo::FindTextureCB`).
* 0x7f5f60 `_rwDeviceRegisterPlugin` is a 5-byte `jmp` to 0x4c9ab0; a part of the D3D9 driver
  (0x4c9970–0x4cd820: raster create/lock/format checks, video-memory raster list, native texture
  read/write) is linked into the game half of `.text`, which is why CAT-5 classified those rows as
  `UNK-game`.

## 7. Open questions (need measurement or a decision)

1. Plugin offsets in `plugins.md` are *derived* from the registration order in the code. Measure them by
   reading the offset globals listed there after `App::PluginAttach` (one oracle read, no guessing).
2. Which `RwV3dTransform*` kernel (x87/SSE/SSE2) the oracle machine installs (§5); the Rust port has to
   match it bit-for-bit, possibly per machine.
3. D3DX9 identities (`d3dx9.md` section of `d3d9.md`) are inferred from argument shapes and call sites;
   confirm them against a D3DX9 build of the same era before phase B replaces them.
4. 237 `low` names (mostly D3D9 pixel converters, tri-strip builder internals, MatFX/skin render paths,
   pipeline unlock internals): roles are clear, exact semantics must be specified function by function
   when they are ported.
5. 0x7fff00 (`_rwD3D9ImageFindRasterFormat`, reached only from the std-func table on the stack of 0x7f72e0)
   is still not a catalogue row after CAT-12b; the `[catalogue: ...]` tags in `rw-names.csv` refer to the
   08:35 state snapshot and should be refreshed when the catalogue is merged.
6. `RwTextureRead` exists twice in caller code: the game calls 0x7f3ac0 directly and some game paths
   go through `CVehicleModelInfo::FindTextureCB` 0x4c7510; keep both names distinct.
