# RW plugin registry and plugin offsets

## Registry layout

Every pluggable RW object type has a static `RwPluginRegistry` (0x18 bytes, `rwplcore.h`) in `.data`:

| off | field | note |
|---|---|---|
| +0x00 | `sizeOfStruct` | grows by `(size + 3) & ~3` per registered plugin; final value = allocation size of the object |
| +0x04 | `origSizeOfStruct` | base struct size (checked against headers below) |
| +0x08 | `maxSizeOfStruct` | 0 everywhere (no limit) |
| +0x0c | `staticAlloc` | 0 |
| +0x10 | `firstRegEntry` | |
| +0x14 | `lastRegEntry` | |

`_rwPluginRegistryAddPlugin` 0x8084a0 (all `*RegisterPlugin` thunks end here): refuses registration once
an engine is open (`_rwGetNumEngineInstances` 0x7f2b90 ≠ 0 → `E_RW_PLUGININIT` 0x80000017); a duplicate
plugin ID sets the same error and **returns the existing offset**; otherwise it allocates a 0x3c-byte
`RwPluginRegEntry` from the registry-entry freelist (0xc9a6e4) and fills `{offset, size, pluginID,
readCB, writeCB, getSizeCB, alwaysCB, rightsCB, ctor, dtor, copy, errStrCB, next, prev, parentRegistry}`.
NULL ctor/dtor/copy are replaced by identity functions `_rwPluginRegistryDefaultConstructor` 0x8086b0,
`...Destructor` 0x8086c0, `...CopyConstructor` 0x8086d0. When the memory hooks are the default freelist
ones (`RwGlobals+0x144 == 0x801c30`) the registry pointer is also appended to a growable array at
0xc9a6e8/0xc9a6ec (used by `_rwResourcesClose` 0x808320). Stream callbacks are attached by
`_rwPluginRegistryAddPluginStream` 0x8088e0 (`AddPlgnStrmlwysCB` 0x808920, `AddPlgnStrmRightsCB`
0x808950). Objects run their plugins with `_rwPluginRegistryInitObject` 0x8086e0,
`DeInitObject` 0x808740, `CopyObject` 0x808770, `ReadDataChunks` 0x808980, `WriteDataChunks` 0x808b40,
`GetSize` 0x808b00, `SkipDataChunks` 0x808c10, `InvokeRights` 0x808ab0.

| registry (.data) | object | base size in exe | header `sizeof` | final size (derived) |
|---|---|---|---|---|
| 0x8e2298 | RwEngine (RwGlobals) | 0x158 | 0x158 (plugin-sdk) / 0x12c (gta-reversed, wrong) | 0x5b0 |
| 0x8e2270 | RwFrame | 0xa4 | 0xa4 | 0xc8 |
| 0x8e222c | RwCamera | 0x184 | 0x184 | 0x1ac |
| 0x8e2518 | RwRaster | 0x34 | 0x34 | 0x58 |
| 0x8e23cc | RwTexture | 0x58 | 0x58 | 0x5c |
| 0x8e23e4 | RwTexDictionary | 0x18 | 0x18 | 0x1c |
| 0x8d624c | RpAtomic | 0x70 | 0x70 | 0x8c |
| 0x8d6264 | RpClump | 0x2c | 0x2c | 0x40 |
| 0x8d628c | RpGeometry | 0x60 | 0x60 | 0x7c |
| 0x8d62a4 | RpMaterial | 0x1c | 0x1c | 0x50 |
| 0x8d62d0 | RpWorld | 0x70 | 0x70 | 0x70 |
| 0x8d62f8 | RpLight | 0x40 | 0x40 | 0x64 |
| 0x8d736c | RpWorldSector | 0x88 | 0x88 | 0x90 |

Base sizes are the `.data` initialisers of `sizeOfStruct/origSizeOfStruct` (read from the file).

## Every plugin the game registers, in registration order

Order = `RwEngineInit` 0x7f3170, `_rwDeviceRegisterPlugin`, then `App::PluginAttach` 0x53d870 and the
functions it calls, in code order (the registration calls are straight-line; a failed registration aborts
start-up). The offset column is **derived** by replaying that order over the base sizes with 4-byte
rounding; measure it by reading the "offset stored in" global after start-up before relying on it.
Zero-size plugins (stream-only) take no space and share the next offset. `R*` IDs are Rockstar's
(vendor 0x253f2). Engine-module offsets are stored by each module's `Open` callback (`DAT = param_2`).

| registry | id | plugin | offset (derived) | size | offset stored in | registered by |
|---|---|---|---|---|---|---|
| RwEngine | 0x40f | ERRORMODULE | 0x158 | 0x8 | 0x00c9a6f0 | RwEngineInit |
| RwEngine | 0x401 | VECTORMODULE | 0x160 | 0x18 | 0x00c97934 | RwEngineInit |
| RwEngine | 0x40d | COLORMODULE | 0x178 | 0x0 | — | RwEngineInit |
| RwEngine | 0x402 | MATRIXMODULE | 0x178 | 0x18 | 0x00c979bc | RwEngineInit |
| RwEngine | 0x403 | FRAMEMODULE | 0x190 | 0x4 | 0x00c9796c | RwEngineInit |
| RwEngine | 0x404 | STREAMMODULE | 0x194 | 0x4 | 0x00c9792c | RwEngineInit |
| RwEngine | 0x405 | CAMERAMODULE | 0x198 | 0x4 | 0x00c97960 | RwEngineInit |
| RwEngine | 0x406 | IMAGEMODULE | 0x19c | 0x220 | 0x00c9a65c | RwEngineInit |
| RwEngine | 0x407 | RASTERMODULE | 0x3bc | 0x64 | 0x00c980d8 | RwEngineInit |
| RwEngine | 0x408 | TEXTUREMODULE | 0x420 | 0x34 | 0x00c97b4c | RwEngineInit |
| RwEngine | 0x409 | PIPEMODULE | 0x454 | 0x60 | 0x00c9bc60 | RwEngineInit |
| RwEngine | 0x412 | CHUNKGROUPMODULE | 0x4b4 | 0x4 | 0x00c9a6f8 | RwEngineInit |
| RwEngine | 0x40a | IMMEDIATEMODULE | 0x4b8 | 0x74 | 0x00c9c078 | RwEngineInit |
| RwEngine | 0x40b | RESOURCESMODULE | 0x52c | 0x28 | 0x00c9bc58 | RwEngineInit |
| RwRaster | 0x40c | DEVICEMODULE | 0x34 | 0x24 | 0x00b4e9e0 | _rwD3D9RasterPluginAttach 0x4c9ab0 |
| RwEngine | 0x501 | MATERIALMODULE | 0x554 | 0x4 | 0x00c92514 | RpWorldPluginAttach |
| RwEngine | 0x502 | MESHMODULE | 0x558 | 0x30 | — | RpWorldPluginAttach |
| RwEngine | 0x503 | GEOMETRYMODULE | 0x588 | 0x4 | 0x00c924e0 | RpWorldPluginAttach |
| RwEngine | 0x504 | CLUMPMODULE | 0x58c | 0x8 | 0x00c924ac | RpWorldPluginAttach |
| RwEngine | 0x505 | LIGHTMODULE | 0x594 | 0x4 | 0x00c925d8 | RpWorldPluginAttach |
| RwEngine | 0x50a | SECTORMODULE | 0x598 | 0x0 | — | RpWorldPluginAttach |
| RwEngine | 0x507 | WORLDMODULE | 0x598 | 0xc | 0x00c9254c | RpWorldPluginAttach |
| RwEngine | 0x50b | BINWORLDMODULE | 0x5a4 | 0x0 | — | RpWorldPluginAttach |
| RwEngine | 0x509 | WORLDOBJMODULE | 0x5a4 | 0xc | 0x00c9259c | _rpWorldObjRegisterPlugins |
| RwCamera | 0x509 | WORLDOBJMODULE | 0x184 | 0x1c | 0x00c925a4 | _rpWorldObjRegisterPlugins |
| RpAtomic | 0x509 | WORLDOBJMODULE | 0x70 | 0x8 | 0x00c925a8 | _rpWorldObjRegisterPlugins |
| RpClump | 0x509 | WORLDOBJMODULE | 0x2c | 0x8 | 0x00c925ac | _rpWorldObjRegisterPlugins |
| RpLight | 0x509 | WORLDOBJMODULE | 0x40 | 0x8 | 0x00c925b0 | _rpWorldObjRegisterPlugins |
| RpGeometry | 0x50e | BINMESHPLUGIN | 0x60 | 0x0 | — | _rpWorldObjRegisterPlugins |
| RpWorldSector | 0x50e | BINMESHPLUGIN | 0x88 | 0x0 | — | _rpWorldObjRegisterPlugins |
| RpGeometry | 0x510 | NATIVEDATAPLUGIN | 0x60 | 0x0 | — | _rpWorldObjRegisterPlugins |
| RpWorldSector | 0x510 | NATIVEDATAPLUGIN | 0x88 | 0x0 | — | _rpWorldObjRegisterPlugins |
| RpAtomic | 0x1f | RIGHTTORENDER | 0x78 | 0x0 | — | _rpWorldObjRegisterPlugins |
| RpWorld | 0x1f | RIGHTTORENDER | 0x70 | 0x0 | — | _rpWorldObjRegisterPlugins |
| RpWorldSector | 0x1f | RIGHTTORENDER | 0x88 | 0x0 | — | _rpWorldObjRegisterPlugins |
| RpMaterial | 0x1f | RIGHTTORENDER | 0x1c | 0x0 | — | _rpWorldObjRegisterPlugins |
| RwCamera | 0x10 | CLUMP | 0x1a0 | 0xc | 0x00c924d8 | _rpClumpRegisterExtensions |
| RpLight | 0x10 | CLUMP | 0x48 | 0xc | 0x00c924dc | _rpClumpRegisterExtensions |
| RpLight | 0x505 | LIGHTMODULE | 0x54 | 0x10 | 0x00c926b0 | _rpD3D9LightRegisterPlugin |
| RwEngine | 0x50f | RXWORLDDEVICEMODULE | 0x5b0 | 0x0 | — | _rpWorldDeviceRegisterPlugins2 |
| RpGeometry | 0x50f | RXWORLDDEVICEMODULE | 0x60 | 0x4 | 0x00c926e8 | _rpWorldDeviceRegisterPlugins2 |
| RpWorldSector | 0x50f | RXWORLDDEVICEMODULE | 0x88 | 0x4 | 0x00c926ec | _rpWorldDeviceRegisterPlugins2 |
| RwEngine | 0x116 | SKINPLUGIN | 0x5b0 | 0x0 | 0x00c978a0 | RpSkinPluginAttach |
| RpAtomic | 0x116 | SKINPLUGIN | 0x78 | 0x4 | 0x00c978a4 | RpSkinPluginAttach |
| RpGeometry | 0x116 | SKINPLUGIN | 0x64 | 0x4 | 0x00c978a8 | RpSkinPluginAttach |
| RwEngine | 0x1b7 | ANIMTOOLKIT | 0x5b0 | 0x0 | — | RtAnimInitialize |
| RwEngine | 0x11e | HANIMPLUGIN | 0x5b0 | 0x0 | — | RpHAnimPluginAttach |
| RwFrame | 0x11e | HANIMPLUGIN | 0xa4 | 0x8 | 0x00c9b8c8 | RpHAnimPluginAttach |
| RwFrame | 0x253f2fe | R*NODENAME | 0xac | 0x18 | 0x00c87c5c | NodeNamePlugin::NodeNamePluginAttach |
| RpAtomic | 0x253f200 | R*VISATOMIC | 0x7c | 0x4 | 0x008d608c | CVisibilityPlugins::PluginAttach |
| RwFrame | 0x253f202 | R*VISFRAME | 0xc4 | 0x4 | 0x008d6090 | CVisibilityPlugins::PluginAttach |
| RpClump | 0x253f201 | R*VISCLUMP | 0x34 | 0x8 | 0x008d6094 | CVisibilityPlugins::PluginAttach |
| RpClump | 0x253f2fb | R*ANIMBLEND | 0x3c | 0x4 | 0x00b5f878 | RpAnimBlend::RpAnimBlendPluginAttach |
| RwTexDictionary | 0x253f2f5 | R*TXDPARENT | 0x18 | 0x4 | 0x00c88018 | CTxdStore::PluginAttach 0x731650 |
| RwEngine | 0x120 | MATERIALEFFECTSPLUGIN | 0x5b0 | 0x0 | — | RpMatFXPluginAttach |
| RpMaterial | 0x120 | MATERIALEFFECTSPLUGIN | 0x1c | 0x4 | 0x00c9ab74 | RpMatFXPluginAttach |
| RpAtomic | 0x120 | MATERIALEFFECTSPLUGIN | 0x80 | 0x4 | 0x00c9ab84 | RpMatFXPluginAttach |
| RpWorldSector | 0x120 | MATERIALEFFECTSPLUGIN | 0x8c | 0x4 | 0x00c9ab88 | RpMatFXPluginAttach |
| RwEngine | 0x135 | UVANIMPLUGIN | 0x5b0 | 0x0 | — | RpUVAnimPluginAttach |
| RpMaterial | 0x135 | UVANIMPLUGIN | 0x20 | 0x28 | 0x00c9b8d0 | RpUVAnimPluginAttach |
| RpGeometry | 0x253f2f9 | R*EXTRAVERTCOL | 0x68 | 0xc | 0x008d12bc | CCustomBuildingDNPipeline::ExtraVertColourPluginAttach |
| RpMaterial | 0x253f2fc | R*ENVMAT | 0x48 | 0x4 | 0x008d12c4 | CCustomCarEnvMapPipeline::RegisterPlugin |
| RpAtomic | 0x253f2f4 | R*ENVATM | 0x84 | 0x4 | 0x008d12c8 | CCustomCarEnvMapPipeline::RegisterPlugin |
| RpMaterial | 0x253f2f6 | R*SPECMAT | 0x4c | 0x4 | 0x008d12cc | CCustomCarEnvMapPipeline::RegisterPlugin |
| RwEngine | 0x127 | ANISOTPLUGIN | 0x5b0 | 0x0 | — | RpAnisotPluginAttach |
| RwTexture | 0x127 | ANISOTPLUGIN | 0x58 | 0x1 | 0x00c92478 | RpAnisotPluginAttach |
| RpGeometry | 0x253f2fd | R*BREAKABLE | 0x74 | 0x4 | 0x00bb4238 | BreakablePlugin::BreakablePluginAttach |
| RpClump | 0x253f2fa | R*COLLISION | 0x40 | 0x0 | 0x9689dc | CCollisionPlugin::PluginAttach (0x41b310, .HOODLUM body 0x15671d0) |
| RpGeometry | 0x253f2f8 | R*2DFX | 0x78 | 0x4 | 0x00c3a1e0 | C2dEffect::PluginAttach |
| RpAtomic | 0x253f2f3 | R*PIPELINE | 0x88 | 0x4 | 0x008d6080 | PipelinePlugin::PipelinePluginAttach |

Notes:

* Offsets confirmed indirectly by the code: the D3D9 raster extension is accessed at
  `raster + [0xb4e9e0]` everywhere and has size 0x24 (= `_rwD3D9RasterExt`, see `objects.md`); the
  texture module data is used at `engine + [0xc97b4c] + 0x14/0x18/0x1c/0x20` (read/find callbacks,
  mipmapping flags); the pipe module holds the default pipelines (`engine + [0xc9bc60] + 0x3c` = default
  atomic pipeline used by `AtomicDefaultRenderCallBack` 0x7491c0, +0x58 = default sector pipeline,
  `_rpDestroyD3D9WorldSectorPipe` 0x74eb40).
* `RpAnisotPluginAttach`'s texture plugin has size 1 (one byte of anisotropy per texture) and is rounded to
  4. Its result is not checked by `App::PluginAttach`.
* The skin engine `Open` (0x7c68e0) queries `RwEngineGetPluginOffset(0x120)` (MatFX) and `0x12e`
  (rwID_NORMMAPPLUGIN? not registered by the game → -1) to choose skin pipeline variants; MatFX is
  attached *after* skin, but engine `Open` callbacks only run in `RwEngineStart`, when all plugins exist.

## Callbacks per plugin

All callbacks are in `catalog/rw-names.csv` as `cb_<ID>_<Owner>_<Role>` (role = API slot, `high`), except
where a better name exists (gta-reversed names for the Rockstar plugins, e.g.
`CVisibilityPlugins::AtomicDestructor`). Highlights:

| plugin | owner | ctor / dtor / copy | stream read / write / size |
|---|---|---|---|
| WORLDOBJMODULE 0x509 | RwCamera | 0x7503e0 / 0x750760 / 0x750710 (ctor hooks camera begin/end update and sync: 0x750430 / 0x750460 / 0x750480) | — |
| WORLDOBJMODULE 0x509 | RpAtomic | 0x7507b0 / 0x7509b0 / 0x7509a0 (sync hook 0x7507e0) | — |
| WORLDOBJMODULE 0x509 | RpClump | 0x750a30 / 0x750a90 / 0x750a60 | — |
| WORLDOBJMODULE 0x509 | RpLight | 0x750aa0 / 0x750d30 / 0x750cd0 (sync hook 0x750ac0) | — |
| BINMESHPLUGIN 0x50e | RpGeometry / sector | — | 0x750db0 / 0x750d90 / 0x750de0; 0x750ed0 / 0x750ea0 / 0x750f00 |
| NATIVEDATAPLUGIN 0x510 | RpGeometry / sector | — | 0x750e20 / 0x750e00 / 0x750e40 (bodies 0x763a80 / 0x7637f0 / 0x763eb0); 0x750e70 / 0x750e50 / 0x750e90 (bodies 0x7641c0 / 0x763f20 / 0x7645c0) |
| RIGHTTORENDER 0x1f | atomic, world, sector, material | — | atomic 0x749120 / 0x749160 / 0x7491a0; world 0x761d60 / 0x761da0 / 0x761de0; sector 0x761e00 / 0x761e40 / 0x761e80; material 0x74d7d0 / 0x74d810 / 0x74d850 |
| CLUMP 0x10 | RwCamera, RpLight | 0x749880 / 0x7498a0; 0x7498b0 / 0x7498d0 | — |
| LIGHTMODULE 0x505 (D3D9) | RpLight | 0x755bc0 / 0x755bf0 / 0x755ce0 | — |
| RXWORLDDEVICEMODULE 0x50f | geometry + sector | 0x758850 / 0x758860 / 0x758870 | — |
| SKINPLUGIN 0x116 | RpAtomic | 0x7c6a80 / 0x7c6aa0 / 0x7c6ac0 | 0x7c72a0 / 0x7c7500 / 0x7c7510 |
| SKINPLUGIN 0x116 | RpGeometry | 0x7c69f0 / 0x7c6a10 / 0x7c6a70 | 0x7c6d20 / 0x7c6c30 / 0x7c6be0 |
| HANIMPLUGIN 0x11e | RwFrame | 0x7c4750 / 0x7c4770 / 0x7c4830 | 0x7c4a00 / 0x7c48d0 / 0x7c4bf0 |
| MATERIALEFFECTSPLUGIN 0x120 | RpMaterial | 0x810c40 / 0x810c60 / 0x810d20 | 0x811410 / 0x8111f0 / 0x811970 |
| MATERIALEFFECTSPLUGIN 0x120 | RpAtomic | 0x811a00 / 0x811a20 / 0x811a40 | 0x811a90 / 0x811a60 / 0x811ae0 |
| MATERIALEFFECTSPLUGIN 0x120 | RpWorldSector | 0x811b00 / 0x811b20 / 0x811b40 | 0x811b90 / 0x811b60 / 0x811be0 |
| UVANIMPLUGIN 0x135 | RpMaterial | 0x7cba80 / 0x7cbaa0 / 0x7cbb30 | 0x7cbc20 / 0x7cbb70 / 0x7cbf20 |
| ANISOTPLUGIN 0x127 | RwTexture | 0x749020 / 0x749030 / 0x749040 | 0x749060 / 0x7490b0 / 0x7490f0 |
| DEVICEMODULE 0x40c | RwRaster | 0x4c9a60 / 0x4c9a80 / — | — |
| R* NODENAME 0x253f2fe | RwFrame | 0x72f9c0 / 0x72f9e0 / 0x72f9f0 | 0x72fa50 / 0x72fa20 / 0x72fa80 |
| R* PIPELINE 0x253f2f3 | RpAtomic | 0x72fb50 / — / 0x72fb70 | 0x72fb90 / — / 0x72fbc0 |
| R* COLLISION 0x253f2fa | RpClump | 0x41b1a0 / 0x41b1c0 / 0x41b1b0 (size 0) | 0x41b1d0 / 0x41b2f0 / 0x41b300 |
| R* TXDPARENT 0x253f2f5 | RwTexDictionary | 0x731600 / 0x731620 / 0x731630 | — |

Engine module Open/Close pairs (core): ERROR 0x8087d0/0x808810, VECTOR 0x7ede90/0x7ede20, COLOR
0x80aa40/0x80aa50, MATRIX 0x7f16c0/0x7f1660, FRAME 0x7efef0/0x7eff70, STREAM 0x7ec780/0x7ec7e0, CAMERA
0x7ee110/0x7ee0b0, IMAGE 0x802280/0x8024e0, RASTER 0x7fb370/0x7fb310, TEXTURE 0x7f3ea0/0x7f3d00, PIPE
0x807c40/0x807c60, CHUNKGROUP 0x80a780/0x80a7e0, IMMEDIATE 0x7efe20/0x7efde0, RESOURCES 0x807c90/0x807d80;
world: MATERIAL 0x74d8e0/0x74d950, MESH 0x758970/0x758940 (`_rpMeshOpen/_rpMeshClose`), GEOMETRY
0x74bfa0/0x74bfc0, CLUMP 0x749760/0x749480, LIGHT 0x752270/0x7521b0, SECTOR 0x761c30/0x761c40, WORLD
0x74fee0/0x74fe90 (calls `_rpWorldPipeOpen` 0x75e310 / `_rpWorldPipeClose` 0x75e300), BINWORLD
0x7637e0/0x7637d0, WORLDOBJ 0x750280/0x750370, RXWORLDDEVICE 0x758890/0x7588a0; toolkits: SKIN
0x7c68e0/0x7c69a0, ANIMTOOLKIT 0x7cccd0/0x7ccd10, HANIM 0x7c4670/0x7c4720, MATFX 0x810bf0/0x810bb0,
UVANIM 0x7cb9b0/0x7cba20, ANISOT 0x749000/0x749010.
