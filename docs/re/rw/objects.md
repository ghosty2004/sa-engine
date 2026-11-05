# Core object layouts

Layouts come from the RW 3.6 D3D9 SDK headers shipped with plugin-sdk (`plugin_sa/game_sa/rw`) and gta-reversed
(`RenderWare/rw`), compiled for i686 with `clang -Xclang -fdump-record-layouts`. The two header copies agree on
every struct below except `RwGlobals` (gta-reversed lacks `fileFuncs`, README §6). Each struct is checked against the exe:
the plugin-registry base sizes in `.data` equal the header `sizeof` for all 13 registries (`plugins.md`), and the
field uses quoted under each struct are read from the decompilation. Sizes are **before plugins**; add the plugin
extensions from `plugins.md`.

## RwObject (0x8 bytes)

Exe evidence: type byte +0 used as object-type switch in RpClumpForAll*/frame sync; privateFlags +3 bit0 = world-bounding-sphere dirty (0x749d20, RpAtomicGetWorldBoundingSphere 0x749330)

| off | type | field |
|---|---|---|
| 0x0 | `RwUInt8` | type |
| 0x1 | `RwUInt8` | subType |
| 0x2 | `RwUInt8` | flags |
| 0x3 | `RwUInt8` | privateFlags |
| 0x4 | `void *` | parent |

## RwFrame (0xa4 bytes)

Exe evidence: child +0x98 / next +0x9c recursion (0x7f0830, 0x8076a0, 0x809700); root +0xa0 (`_rwSetHierarchyRoot` 0x7f0210); objectList +0x90 sync callbacks (0x809780); ltm +0x50 = parent ltm × modelling +0x10 (0x809700); dirty flags in object.privateFlags (4 = LTM, 8 = objects)

| off | type | field |
|---|---|---|
| 0x0 | `struct RwObject` | object |
| 0x8 | `struct RwLLLink` | inDirtyListLink |
| 0x10 | `struct RwMatrixTag` | modelling |
| 0x50 | `struct RwMatrixTag` | ltm |
| 0x90 | `struct RwLinkList` | objectList |
| 0x98 | `struct RwFrame *` | child |
| 0x9c | `struct RwFrame *` | next |
| 0xa0 | `struct RwFrame *` | root |

## RwObjectHasFrame (0x14 bytes)

Exe evidence: sync +0x10 set by RpAtomicCreate (0x749d20), RpLightCreate (0x7521a0), RwCameraCreate (0x7ee5a0)

| off | type | field |
|---|---|---|
| 0x0 | `struct RwObject` | object |
| 0x8 | `struct RwLLLink` | lFrame |
| 0x10 | `RwObjectHasFrameSyncFunction` | sync |

## RpInterpolator (0x14 bytes)

| off | type | field |
|---|---|---|
| 0x0 | `RwInt32` | flags |
| 0x4 | `RwInt16` | startMorphTarget |
| 0x6 | `RwInt16` | endMorphTarget |
| 0x8 | `RwReal` | time |
| 0xc | `RwReal` | recipTime |
| 0x10 | `RwReal` | position |

## RpAtomic (0x70 bytes)

Exe evidence: geometry +0x18, repEntry +0x14 (`_rpAtomicDestructor` 0x7496a0), bounding sphere +0x1c / world sphere +0x2c (0x749330), interpolator flags +0x4c (0x749d20), renderFrame +0x60 vs RwGlobals.renderFrame (0x74f590), pipeline +0x6c (`_rpSkinAtomicSetPipeline` 0x7c89b0, `_rpMatFXAtomicSetPipeline` 0x816260); registry base 0x70

| off | type | field |
|---|---|---|
| 0x0 | `struct RwObjectHasFrame` | object |
| 0x14 | `RwResEntry *` | repEntry |
| 0x18 | `RpGeometry *` | geometry |
| 0x1c | `struct RwSphere` | boundingSphere |
| 0x2c | `struct RwSphere` | worldBoundingSphere |
| 0x3c | `RpClump *` | clump |
| 0x40 | `struct RwLLLink` | inClumpLink |
| 0x48 | `RpAtomicCallBackRender` | renderCallBack |
| 0x4c | `struct RpInterpolator` | interpolator |
| 0x60 | `RwUInt16` | renderFrame |
| 0x62 | `RwUInt16` | pad |
| 0x64 | `struct RwLinkList` | llWorldSectorsInAtomic |
| 0x6c | `RxPipeline *` | pipeline |

## RpClump (0x2c bytes)

Exe evidence: atomicList +0x8 walked by `_rpClumpDestructor` 0x749520; registry base 0x2c

| off | type | field |
|---|---|---|
| 0x0 | `struct RwObject` | object |
| 0x8 | `struct RwLinkList` | atomicList |
| 0x10 | `struct RwLinkList` | lightList |
| 0x18 | `struct RwLinkList` | cameraList |
| 0x20 | `struct RwLLLink` | inWorldLink |
| 0x28 | `RpClumpCallBack` | callback |

## RpGeometry (0x60 bytes)

Exe evidence: flags +0x8 (0x1000000 = rpGEOMETRYNATIVE, 8 = prelit; 0x74ce60), lockedSinceLastInst +0xc set to 0xfff to force reinstance (0x74ea80), refCount +0xe (`_rpGeometryAddRef` 0x74ccb0), mesh +0x54, morphTarget +0x5c (0x74d6d0), numMorphTargets +0x18 / morph array stride 0x1c (0x7491f0); registry base 0x60

| off | type | field |
|---|---|---|
| 0x0 | `struct RwObject` | object |
| 0x8 | `RwUInt32` | flags |
| 0xc | `RwUInt16` | lockedSinceLastInst |
| 0xe | `RwInt16` | refCount |
| 0x10 | `RwInt32` | numTriangles |
| 0x14 | `RwInt32` | numVertices |
| 0x18 | `RwInt32` | numMorphTargets |
| 0x1c | `RwInt32` | numTexCoordSets |
| 0x20 | `struct RpMaterialList` | matList |
| 0x2c | `RpTriangle *` | triangles |
| 0x30 | `RwRGBA *` | preLitLum |
| 0x34 | `RwTexCoords *[8]` | texCoords |
| 0x54 | `RpMeshHeader *` | mesh |
| 0x58 | `RwResEntry *` | repEntry |
| 0x5c | `RpMorphTarget *` | morphTarget |

## RpMorphTarget (0x1c bytes)

| off | type | field |
|---|---|---|
| 0x0 | `RpGeometry *` | parentGeom |
| 0x4 | `struct RwSphere` | boundingSphere |
| 0x14 | `RwV3d *` | verts |
| 0x18 | `RwV3d *` | normals |

## RpMaterialList (0xc bytes)

| off | type | field |
|---|---|---|
| 0x0 | `RpMaterial **` | materials |
| 0x4 | `RwInt32` | numMaterials |
| 0x8 | `RwInt32` | space |

## RpMaterial (0x1c bytes)

Exe evidence: registry base 0x1c; MatFX ext pointer at +[0xc9ab74], UV-anim ext (0x28 bytes, 8 slots) at +[0xc9b8d0]

| off | type | field |
|---|---|---|
| 0x0 | `RwTexture *` | texture |
| 0x4 | `struct RwRGBA` | color |
| 0x8 | `RxPipeline *` | pipeline |
| 0xc | `struct RwSurfaceProperties` | surfaceProps |
| 0x18 | `RwInt16` | refCount |
| 0x1a | `RwInt16` | pad |

## RpMeshHeader (0x10 bytes)

| off | type | field |
|---|---|---|
| 0x0 | `RwUInt32` | flags |
| 0x4 | `RwUInt16` | numMeshes |
| 0x6 | `RwUInt16` | serialNum |
| 0x8 | `RwUInt32` | totalIndicesInMesh |
| 0xc | `RwUInt32` | firstMeshOffset |

## RpMesh (0xc bytes)

| off | type | field |
|---|---|---|
| 0x0 | `RxVertexIndex *` | indices |
| 0x4 | `RwUInt32` | numIndices |
| 0x8 | `RpMaterial *` | material |

## RwTexture (0x58 bytes)

Exe evidence: refCount +0x54 incremented by RwTextureRead 0x7f3ac0; registry base 0x58; anisotropy byte at +[0xc92478]

| off | type | field |
|---|---|---|
| 0x0 | `RwRaster *` | raster |
| 0x4 | `RwTexDictionary *` | dict |
| 0x8 | `struct RwLLLink` | lInDictionary |
| 0x10 | `RwChar[32]` | name |
| 0x30 | `RwChar[32]` | mask |
| 0x50 | `RwUInt32` | filterAddressing |
| 0x54 | `RwInt32` | refCount |

## RwTexDictionary (0x18 bytes)

| off | type | field |
|---|---|---|
| 0x0 | `struct RwObject` | object |
| 0x8 | `struct RwLinkList` | texturesInDict |
| 0x10 | `struct RwLLLink` | lInInstance |

## RwRaster (0x34 bytes)

Exe evidence: cType +0x20 (low 3 bits = rwRASTERTYPE*), cFlags +0x21, privateFlags +0x22, cFormat +0x23 (`_rwD3D9RasterCreateFormatSelect` 0x4cc5c0, `_rwD3D9RasterCreatePalettized` 0x4cb7c0); nOffsetX/Y +0x1c/+0x1e (Im2D 0x7fb650); registry base 0x34

| off | type | field |
|---|---|---|
| 0x0 | `RwRaster *` | parent |
| 0x4 | `RwUInt8 *` | cpPixels |
| 0x8 | `RwUInt8 *` | palette |
| 0xc | `RwInt32` | width |
| 0x10 | `RwInt32` | height |
| 0x14 | `RwInt32` | depth |
| 0x18 | `RwInt32` | stride |
| 0x1c | `RwInt16` | nOffsetX |
| 0x1e | `RwInt16` | nOffsetY |
| 0x20 | `RwUInt8` | cType |
| 0x21 | `RwUInt8` | cFlags |
| 0x22 | `RwUInt8` | privateFlags |
| 0x23 | `RwUInt8` | cFormat |
| 0x24 | `RwUInt8 *` | originalPixels |
| 0x28 | `RwInt32` | originalWidth |
| 0x2c | `RwInt32` | originalHeight |
| 0x30 | `RwInt32` | originalStride |

## RwImage (0x1c bytes)

| off | type | field |
|---|---|---|
| 0x0 | `RwInt32` | flags |
| 0x4 | `RwInt32` | width |
| 0x8 | `RwInt32` | height |
| 0xc | `RwInt32` | depth |
| 0x10 | `RwInt32` | stride |
| 0x14 | `RwUInt8 *` | cpPixels |
| 0x18 | `RwRGBA *` | palette |

## RwCamera (0x184 bytes)

Exe evidence: beginUpdate +0x18 / endUpdate +0x1c (0x7ef370/0x7ef340, replaced by world-obj hooks), frameBuffer +0x60 / zBuffer +0x64 (CGame::InitialiseRenderWare 0x5bd600), near +0x80 / far +0x84, zScale/zShift from `_rwCameraUpdateZShiftScale` 0x7ee200, frustum planes +0x94 and corners +0x124 (0x7ee740, 0x7eede0); registry base 0x184

| off | type | field |
|---|---|---|
| 0x0 | `struct RwObjectHasFrame` | object |
| 0x14 | `RwCameraProjection` | projectionType |
| 0x18 | `RwCameraBeginUpdateFunc` | beginUpdate |
| 0x1c | `RwCameraEndUpdateFunc` | endUpdate |
| 0x20 | `struct RwMatrixTag` | viewMatrix |
| 0x60 | `RwRaster *` | frameBuffer |
| 0x64 | `RwRaster *` | zBuffer |
| 0x68 | `struct RwV2d` | viewWindow |
| 0x70 | `struct RwV2d` | recipViewWindow |
| 0x78 | `struct RwV2d` | viewOffset |
| 0x80 | `RwReal` | nearPlane |
| 0x84 | `RwReal` | farPlane |
| 0x88 | `RwReal` | fogPlane |
| 0x8c | `RwReal` | zScale |
| 0x90 | `RwReal` | zShift |
| 0x94 | `RwFrustumPlane[6]` | frustumPlanes |
| 0x10c | `struct RwBBox` | frustumBoundBox |
| 0x124 | `RwV3d[8]` | frustumCorners |

## RpLight (0x40 bytes)

Exe evidence: radius +0x14, colour +0x18, minusCosAngle +0x28 (RpLightStreamWrite 0x751e30); object.subType ≥ 0x80 = local light (0x7513d0); registry base 0x40

| off | type | field |
|---|---|---|
| 0x0 | `struct RwObjectHasFrame` | object |
| 0x14 | `RwReal` | radius |
| 0x18 | `struct RwRGBAReal` | color |
| 0x28 | `RwReal` | minusCosAngle |
| 0x2c | `struct RwLinkList` | WorldSectorsInLight |
| 0x34 | `struct RwLLLink` | inWorld |
| 0x3c | `RwUInt16` | lightFrame |
| 0x3e | `RwUInt16` | pad |

## RpWorld (0x70 bytes)

Exe evidence: lightList +0x34, directionalLightList +0x3c (0x7513d0), flags +0x8 bit 0x20 = rpWORLDLIGHT (0x75e570), flag 0x1000000 = native (0x762440); registry base 0x70

| off | type | field |
|---|---|---|
| 0x0 | `struct RwObject` | object |
| 0x8 | `RwUInt32` | flags |
| 0xc | `RpWorldRenderOrder` | renderOrder |
| 0x10 | `struct RpMaterialList` | matList |
| 0x1c | `RpSector *` | rootSector |
| 0x20 | `RwInt32` | numTexCoordSets |
| 0x24 | `RwInt32` | numClumpsInWorld |
| 0x28 | `RwLLLink *` | currentClumpLink |
| 0x2c | `struct RwLinkList` | clumpList |
| 0x34 | `struct RwLinkList` | lightList |
| 0x3c | `struct RwLinkList` | directionalLightList |
| 0x44 | `struct RwV3d` | worldOrigin |
| 0x50 | `struct RwBBox` | boundingBox |
| 0x68 | `RpWorldSectorCallBackRender` | renderCallBack |
| 0x6c | `RxPipeline *` | pipeline |

## RpWorldSector (0x88 bytes)

Exe evidence: numVertices +0x82 (0x75e3f0), mesh +0x78, pipeline +0x7c (0x816270), repEntry +0x34 (0x763f20); registry base 0x88

| off | type | field |
|---|---|---|
| 0x0 | `RwInt32` | type |
| 0x4 | `RpTriangle *` | triangles |
| 0x8 | `RwV3d *` | vertices |
| 0xc | `RpVertexNormal *` | normals |
| 0x10 | `RwTexCoords *[8]` | texCoords |
| 0x30 | `RwRGBA *` | preLitLum |
| 0x34 | `RwResEntry *` | repEntry |
| 0x38 | `struct RwLinkList` | collAtomicsInWorldSector |
| 0x40 | `struct RwLinkList` | lightsInWorldSector |
| 0x48 | `struct RwBBox` | boundingBox |
| 0x60 | `struct RwBBox` | tightBoundingBox |
| 0x78 | `RpMeshHeader *` | mesh |
| 0x7c | `RxPipeline *` | pipeline |
| 0x80 | `RwUInt16` | matListWindowBase |
| 0x82 | `RwUInt16` | numVertices |
| 0x84 | `RwUInt16` | numTriangles |
| 0x86 | `RwUInt16` | pad |

## RpTie (0x18 bytes)

| off | type | field |
|---|---|---|
| 0x0 | `struct RwLLLink` | lAtomicInWorldSector |
| 0x8 | `RpAtomic *` | apAtom |
| 0xc | `struct RwLLLink` | lWorldSectorInAtomic |
| 0x14 | `RpWorldSector *` | worldSector |

## RpLightTie (0x18 bytes)

| off | type | field |
|---|---|---|
| 0x0 | `struct RwLLLink` | lightInWorldSector |
| 0x8 | `RpLight *` | light |
| 0xc | `struct RwLLLink` | WorldSectorInLight |
| 0x14 | `RpWorldSector *` | sect |

## RwResEntry (0x18 bytes)

| off | type | field |
|---|---|---|
| 0x0 | `struct RwLLLink` | link |
| 0x8 | `RwInt32` | size |
| 0xc | `void *` | owner |
| 0x10 | `RwResEntry **` | ownerRef |
| 0x14 | `RwResEntryDestroyNotify` | destroyNotify |

## RxD3D9ResEntryHeader (0x40 bytes)

Exe evidence: 0x40 header + 0x24 × numMeshes allocated by `_rpD3D9InstanceMeshes` 0x756960 and `_rpD3D9SkinInstanceMeshes` 0x7c8d60

| off | type | field |
|---|---|---|
| 0x0 | `RwUInt32` | serialNumber |
| 0x4 | `RwUInt32` | numMeshes |
| 0x8 | `void *` | indexBuffer |
| 0xc | `RwUInt32` | primType |
| 0x10 | `RxD3D9VertexStream[2]` | vertexStream |
| 0x30 | `RwBool` | useOffsets |
| 0x34 | `void *` | vertexDeclaration |
| 0x38 | `RwUInt32` | totalNumIndex |
| 0x3c | `RwUInt32` | totalNumVertex |

## RxD3D9InstanceData (0x24 bytes)

Exe evidence: per-mesh, 0x24 bytes (same allocations)

| off | type | field |
|---|---|---|
| 0x0 | `RwUInt32` | numIndex |
| 0x4 | `RwUInt32` | minVert |
| 0x8 | `RpMaterial *` | material |
| 0xc | `RwBool` | vertexAlpha |
| 0x10 | `void *` | vertexShader |
| 0x14 | `RwUInt32` | baseIndex |
| 0x18 | `RwUInt32` | numVertices |
| 0x1c | `RwUInt32` | startIndex |
| 0x20 | `RwUInt32` | numPrimitives |

## RwGlobals (0x158 bytes)

Exe evidence: see README §6: +0xc4 fileFuncs (0x804140), +0x134.. memoryFuncs, +0x144/+0x148 freelist alloc/free, +0x150 engineStatus (RwEngineInit/Open/Start), +0x48 stdFunc[29] (RwEngineOpen STANDARDS, count 0x1d)

| off | type | field |
|---|---|---|
| 0x0 | `void *` | curCamera |
| 0x4 | `void *` | curWorld |
| 0x8 | `RwUInt16` | renderFrame |
| 0xa | `RwUInt16` | lightFrame |
| 0xc | `RwUInt16[2]` | pad |
| 0x10 | `struct RwDevice` | dOpenDevice |
| 0x48 | `RwStandardFunc[29]` | stdFunc |
| 0xbc | `struct RwLinkList` | dirtyFrameList |
| 0xc4 | `struct RwFileFunctions` | fileFuncs |
| 0xf0 | `struct RwStringFunctions` | stringFuncs |
| 0x134 | `struct RwMemoryFunctions` | memoryFuncs |
| 0x144 | `RwMemoryAllocFn` | memoryAlloc |
| 0x148 | `RwMemoryFreeFn` | memoryFree |
| 0x14c | `RwMetrics *` | metrics |
| 0x150 | `RwEngineStatus` | engineStatus |
| 0x154 | `RwUInt32` | resArenaInitSize |

## RwDevice (0x38 bytes)

Exe evidence: D3D9 instance at .data 0x8e2498: {1.0, D3D9System 0x7f5f70, 0.0, 1.0, 0x7fe420, 0x7fd810, 0x7fb650, 0x7fb8e0, 0x7fbb00, 0x7fbd30, Im3D slots filled at run time}

| off | type | field |
|---|---|---|
| 0x0 | `RwReal` | gammaCorrection |
| 0x4 | `RwSystemFunc` | fpSystem |
| 0x8 | `RwReal` | zBufferNear |
| 0xc | `RwReal` | zBufferFar |
| 0x10 | `RwRenderStateSetFunction` | fpRenderStateSet |
| 0x14 | `RwRenderStateGetFunction` | fpRenderStateGet |
| 0x18 | `RwIm2DRenderLineFunction` | fpIm2DRenderLine |
| 0x1c | `RwIm2DRenderTriangleFunction` | fpIm2DRenderTriangle |
| 0x20 | `RwIm2DRenderPrimitiveFunction` | fpIm2DRenderPrimitive |
| 0x24 | `RwIm2DRenderIndexedPrimitiveFunction` | fpIm2DRenderIndexedPrimitive |
| 0x28 | `RwIm3DRenderLineFunction` | fpIm3DRenderLine |
| 0x2c | `RwIm3DRenderTriangleFunction` | fpIm3DRenderTriangle |
| 0x30 | `RwIm3DRenderPrimitiveFunction` | fpIm3DRenderPrimitive |
| 0x34 | `RwIm3DRenderIndexedPrimitiveFunction` | fpIm3DRenderIndexedPrimitive |

## RwFreeList (0x24 bytes)

Exe evidence: 0x24-byte entries in the freelist-of-freelists (`_rwMemoryOpen` 0x801fd0)

| off | type | field |
|---|---|---|
| 0x0 | `RwUInt32` | entrySize |
| 0x4 | `RwUInt32` | entriesPerBlock |
| 0x8 | `RwUInt32` | heapSize |
| 0xc | `RwUInt32` | alignment |
| 0x10 | `struct RwLinkList` | blockList |
| 0x18 | `RwUInt32` | flags |
| 0x1c | `struct RwLLLink` | link |

## RwPluginRegistry (0x18 bytes)

Exe evidence: see plugins.md

| off | type | field |
|---|---|---|
| 0x0 | `RwInt32` | sizeOfStruct |
| 0x4 | `RwInt32` | origSizeOfStruct |
| 0x8 | `RwInt32` | maxSizeOfStruct |
| 0xc | `RwInt32` | staticAlloc |
| 0x10 | `RwPluginRegEntry *` | firstRegEntry |
| 0x14 | `RwPluginRegEntry *` | lastRegEntry |

## RxPipeline (0x34 bytes)

Exe evidence: pluginId +0x2c set to 2 (world D3D9 pipes 0x74ead0/0x74eb80), 1 (Im3D 0x80a0a0), 0x120 (MatFX 0x815e60)

| off | type | field |
|---|---|---|
| 0x0 | `RwBool` | locked |
| 0x4 | `RwUInt32` | numNodes |
| 0x8 | `RxPipelineNode *` | nodes |
| 0xc | `RwUInt32` | packetNumClusterSlots |
| 0x10 | `rxEmbeddedPacketState` | embeddedPacketState |
| 0x14 | `RxPacket *` | embeddedPacket |
| 0x18 | `RwUInt32` | numInputRequirements |
| 0x1c | `RxPipelineRequiresCluster *` | inputRequirements |
| 0x20 | `void *` | superBlock |
| 0x24 | `RwUInt32` | superBlockSize |
| 0x28 | `RwUInt32` | entryPoint |
| 0x2c | `RwUInt32` | pluginId |
| 0x30 | `RwUInt32` | pluginData |

## RxPipelineNode (0x28 bytes)

| off | type | field |
|---|---|---|
| 0x0 | `RxNodeDefinition *` | nodeDef |
| 0x4 | `RwUInt32` | numOutputs |
| 0x8 | `RwUInt32 *` | outputs |
| 0xc | `RxPipelineCluster **` | slotClusterRefs |
| 0x10 | `RwUInt32 *` | slotsContinue |
| 0x14 | `void *` | privateData |
| 0x18 | `RwUInt32 *` | inputToClusterSlot |
| 0x1c | `RxPipelineNodeTopSortData *` | topSortData |
| 0x20 | `void *` | initializationData |
| 0x24 | `RwUInt32` | initializationDataSize |

## RxNodeDefinition (0x40 bytes)

Exe evidence: D3D9 node definitions: atomic all-in-one (body 0x7575f0 at .data 0x8d633c, init 0x757890 at 0x8d6348), sector (0x75e3f0 / 0x75e540 at 0x8d63bc / 0x8d63c8), skin (0x7c7b90 / 0x7cb240 at 0x8ded0c / 0x8ded18), "ImmInstance.csl" 0x8e2918, "nodeD3D9SubmitNoLight.csl" 0x8e2978

| off | type | field |
|---|---|---|
| 0x0 | `RwChar *` | name |
| 0x4 | `struct RxNodeMethods` | nodeMethods |
| 0x20 | `struct RxIoSpec` | io |
| 0x34 | `RwUInt32` | pipelineNodePrivateDataSize |
| 0x38 | `RxNodeDefEditable` | editable |
| 0x3c | `RwInt32` | InputPipesCnt |

## RpHAnimHierarchy (0x24 bytes)

| off | type | field |
|---|---|---|
| 0x0 | `RwInt32` | flags |
| 0x4 | `RwInt32` | numNodes |
| 0x8 | `RwMatrix *` | pMatrixArray |
| 0xc | `void *` | pMatrixArrayUnaligned |
| 0x10 | `RpHAnimNodeInfo *` | pNodeInfo |
| 0x14 | `RwFrame *` | parentFrame |
| 0x18 | `RpHAnimHierarchy *` | parentHierarchy |
| 0x1c | `RwInt32` | rootParentOffset |
| 0x20 | `RtAnimInterpolator *` | currentAnim |

## RpHAnimNodeInfo (0x10 bytes)

| off | type | field |
|---|---|---|
| 0x0 | `RwInt32` | nodeID |
| 0x4 | `RwInt32` | nodeIndex |
| 0x8 | `RwInt32` | flags |
| 0xc | `RwFrame *` | pFrame |

## RwMatrixTag (0x40 bytes)

| off | type | field |
|---|---|---|
| 0x0 | `struct RwV3d` | right |
| 0xc | `RwUInt32` | flags |
| 0x10 | `struct RwV3d` | up |
| 0x1c | `RwUInt32` | pad1 |
| 0x20 | `struct RwV3d` | at |
| 0x2c | `RwUInt32` | pad2 |
| 0x30 | `struct RwV3d` | pos |
| 0x3c | `RwUInt32` | pad3 |

## D3D9 raster extension `_rwD3D9RasterExt` (0x24 bytes, at raster + [0xb4e9e0])

Layout from gta-reversed `app/platform/win/WindowedMode.cpp:66` (not in the SDK headers); size matches
`RwRasterRegisterPlugin(0x24, 0x40c, ...)` in `_rwD3D9RasterPluginAttach` 0x4c9ab0.

| off | type | field | exe use |
|---|---|---|---|
| +0x00 | `IDirect3DTexture9*` | texture | released via vtbl+8 in `_rwD3D9RasterDestroy` 0x4cbb00 |
| +0x04 | `_rwD3D9Palette*` | palette | 0x404-byte palettes from freelist 0xb4e9f8 (`_rwD3D9RasterCreatePalettized` 0x4cb7c0); `globalindex` at +0x400 |
| +0x08 | `RwUInt8` | alpha | `_rwD3D9RasterHasAlpha` 0x4c9ea0 |
| +0x09 | `RwUInt8` | cube:4, face:4 | `_rwD3D9RasterIsCubeMap` 0x4c9f70 (& 0xf) |
| +0x0a | `RwUInt8` | automipmapgen:4, compressed:4 | `_rwD3D9RasterCreateFromDDSFormat` 0x4cd050 |
| +0x0b | `RwUInt8` | lockedMipLevel | set to 0xff on unlock (`_rwD3D9RasterUnlock` 0x4ca290) |
| +0x0c | `IDirect3DSurface9*` | lockedSurface | |
| +0x10 | `D3DLOCKED_RECT` | lockedRect | |
| +0x18 | `D3DFORMAT` | d3dFormat | converter selection 0x7ff8c0 / 0x800810, Z formats 0x4ccd70 |
| +0x1c | `IDirect3DSwapChain9*` | swapChain | |
| +0x20 | `HWND` | window | |

Other D3D9 driver globals (names from gta-reversed `WindowedMode.cpp:105-133`): `IDirect3D9*` 0xc97c20, adapter index
0xc97c24, device type 0x8e2428, `IDirect3DDevice9*` 0xc97c28, render/depth surfaces 0xc97c30/0xc97c2c, window 0xc97c1c,
present parameters 0xc9c040, caps 0xc9bf00, adapter info 0xc9bce0, display modes 0xc97c48/0xc97c40, view/projection
matrices 0xc9bc80/0x8e2458, lights cache 0xc98088/0xc98084, restore callback 0xc980b0, pixel-format table 0xb4e7e0.
