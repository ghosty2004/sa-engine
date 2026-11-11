# RW pass-3 generated specs: byte-identical boilerplate shapes (RE-SPEC-RW-3 lead)

Each section below was generated from the bytes of its own function by tools in the pass-3 scratch directory: functions were grouped by
an exact instruction-shape match (same mnemonics and operands, differing only in addresses/immediates >= 0x10000), and each section quotes
that function's own addresses, globals and callees. Spot check: 6 of 104 sections (0x74d8c0, 0x7f9d40, 0x751de0, 0x811c70, 0x808cc0,
0x80a440) re-read against the disassembly, all correct. Calling convention for all: cdecl, arguments on the stack, caller pops.
RwEngineInstance = [0xc97b24].

## 0x5d6e90 CCustomBuildingDNPipeline::GetExtraVertColourPtr  (catalogue: specified/REA, port level 0)
- Shape group: plugin-offset getters (8 byte-identical functions; generated from the bytes 0x5d6e90-0x5d6e9d). Callers: 0x5d7f00 CCustomBuildingRenderer::AtomicSetup, 0x5d7f40 CCustomBuildingRenderer::IsCBPCPipelineAttached.
- `u32 (void* obj [esp+4])`, cdecl: EAX = `*(u32*)((u8*)obj + *(i32*)[0x8d12bc])` (0x5d6e94): reads the dword at the plugin/extension offset held in global [0x8d12bc] CCustomBuildingDNPipeline::ms_extraVertColourPluginOffset. No NULL check of obj.
- librw: absent (RW-internal boilerplate)

## 0x749010 cb_ANISOTPLUGIN_RwEngine_Close  (catalogue: specified/REA, port level 0)
- Shape group: module close (count only) (7 byte-identical functions; generated from the bytes 0x749010-0x74901f). Callers: 0x748f70 RpAnisotPluginAttach.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl: `[0xc92474] -= 1` (s32 module open count, read 0x749010 / write 0x749016); returns `instance` (arg 0) in EAX.
- librw: absent (RW-internal boilerplate)

## 0x749720 RpAtomicSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x749720-0x749733). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8d627c] (0x749728) and numBlocksToPreallocate -> [0x8d6280] (0x74972d); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8d627c] = 128, [0x8d6280] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x749740 RpClumpSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x749740-0x749753). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8d6284] (0x749748) and numBlocksToPreallocate -> [0x8d6288] (0x74974d); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8d6284] = 128, [0x8d6288] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x7498a0 cb_CLUMP_RwCamera_Dtor  (catalogue: specified/REA, port level 0)
- Shape group: return first argument (4 byte-identical functions; generated from the bytes 0x7498a0-0x7498a4). Callers: 0x749830 _rpClumpRegisterExtensions.
- cdecl, returns its first argument [esp+4] in EAX; no other effect.
- librw: absent (RW-internal boilerplate)

## 0x749e40 RpLightGetClump  (catalogue: specified/REA, port level 0)
- Shape group: plugin-offset getters (8 byte-identical functions; generated from the bytes 0x749e40-0x749e4d). Callers: none in callgraph (reached through data / function pointer).
- `u32 (void* obj [esp+4])`, cdecl: EAX = `*(u32*)((u8*)obj + *(i32*)[0xc924dc])` (0x749e44): reads the dword at the plugin/extension offset held in global [0xc924dc]. No NULL check of obj.
- librw: absent (RW-internal boilerplate)

## 0x749e50 RwCameraGetClump  (catalogue: specified/REA, port level 0)
- Shape group: plugin-offset getters (8 byte-identical functions; generated from the bytes 0x749e50-0x749e5d). Callers: none in callgraph (reached through data / function pointer).
- `u32 (void* obj [esp+4])`, cdecl: EAX = `*(u32*)((u8*)obj + *(i32*)[0xc924d8])` (0x749e54): reads the dword at the plugin/extension offset held in global [0xc924d8]. No NULL check of obj.
- librw: absent (RW-internal boilerplate)

## 0x74a260 _rpClumpDefaultCallBack  (catalogue: specified/REA, port level 0)
- Shape group: return first argument (4 byte-identical functions; generated from the bytes 0x74a260-0x74a264). Callers: 0x74a240 RpClumpSetCallBack, 0x74a270 RpClumpGetCallBack, 0x74a290 RpClumpCreate.
- cdecl, returns its first argument [esp+4] in EAX; no other effect.
- librw: absent (RW-internal boilerplate)

## 0x74bea0 RpClumpSetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x74bea0-0x74beb7). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmlwysCB` 0x808920 with `(registry = 0x8d6264, pluginID, callback)` (push order at 0x74beaa), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x74bee0 RpClumpGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x74bee0-0x74bef2). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8d6264, pluginID)` (call at 0x74beea); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x74bf00 RpAtomicValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x74bf00-0x74bf05). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x74bf10 RpClumpValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x74bf10-0x74bf15). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x74bfc0 cb_GEOMETRYMODULE_RwEngine_Close  (catalogue: specified/REA, port level 0)
- Shape group: module close (count only) (7 byte-identical functions; generated from the bytes 0x74bfc0-0x74bfcf). Callers: 0x74fda0 RpWorldPluginAttach.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl: `[0xc924e4] -= 1` (s32 module open count, read 0x74bfc0 / write 0x74bfc6); returns `instance` (arg 0) in EAX.
- librw: absent (RW-internal boilerplate)

## 0x74bfd0 sub_74bfd0 (proposed: _rpGeometryGetSortMaterialsDefault)  (catalogue: unknown/REA, port level 0)
- Shape group: global getters (4 byte-identical functions; generated from the bytes 0x74bfd0-0x74bfd5). Callers: 0x75dae0 RpGeometrySortByMaterial.
- `u32 ()`, cdecl: EAX = [0x8d629c] (dword global, 0x74bfd0).
- librw: absent (RW-internal boilerplate)

## 0x74cdd0 RpGeometrySetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x74cdd0-0x74cde7). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmlwysCB` 0x808920 with `(registry = 0x8d628c, pluginID, callback)` (push order at 0x74cdda), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x74cdf0 RpGeometryGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x74cdf0-0x74ce02). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8d628c, pluginID)` (call at 0x74cdfa); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x74ce10 RpGeometryValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x74ce10-0x74ce15). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x74d7d0 cb_RIGHTTORENDER_RpMaterial_StreamRead  (catalogue: specified/REA, port level 3)
- Shape group: right-to-render stream read (3 byte-identical functions; generated from the bytes 0x74d7d0-0x74d80b). Callers: 0x750050 _rpWorldObjRegisterPlugins.
- Plugin stream-read callback `RwStream* (RwStream* stream [esp+4], RwInt32 length [esp+8], void* object, RwInt32 offset, RwInt32 size)`, cdecl, ESI saved.
- `RwStreamReadInt32` 0x7ed540(stream, &[0xc92510], 4) (0x74d7dd): reads the plugin ID of the rights owner into the GLOBAL [0xc92510] (not into the object). On 0 returns 0 (EAX of the failed call).
- If length == 8 (`cmp [esp+0xc], 8` at 0x74d7eb, i.e. arg 2 after the ESI push): second `RwStreamReadInt32`(stream, &[0xc924e8], 4) reads the extra data dword into global [0xc924e8]; on 0 returns 0. Any other length: no second read.
- Returns stream. The object argument is never touched; the globals are consumed later by the stream-rights callback (RW 3.6 rights-to-render mechanism).
- librw: absent (librw does not implement right-to-render)

## 0x74d8c0 RpMaterialSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x74d8c0-0x74d8d3). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8d62c8] (0x74d8c8) and numBlocksToPreallocate -> [0x8d62cc] (0x74d8cd); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8d62c8] = 256, [0x8d62cc] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x74d950 cb_MATERIALMODULE_RwEngine_Close  (catalogue: specified/REA, port level 1)
- Shape group: module close (free list + count) (4 byte-identical functions; generated from the bytes 0x74d950-0x74d98c). Callers: 0x74fda0 RpWorldPluginAttach.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl.
- `fl = *(RwFreeList**)(RwEngineInstance [0xc97b24] + [0xc92514])` (module offset global, 0x74d955); if fl != NULL (je 0x74d97d at 0x74d960): `RwFreeListDestroy` 0x801b80(fl), then re-reads both globals and stores 0 to that slot (0x74d976).
- Always: `[0xc92518] -= 1` (module open count, 0x74d983); returns `instance` in EAX.
- librw: absent (RW-internal boilerplate)

## 0x74dc50 RpMaterialSetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x74dc50-0x74dc67). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmlwysCB` 0x808920 with `(registry = 0x8d62a4, pluginID, callback)` (push order at 0x74dc5a), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x74dc70 RpMaterialSetStreamRightsCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x74dc70-0x74dc87). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmRightsCB` 0x808950 with `(registry = 0x8d62a4, pluginID, callback)` (push order at 0x74dc7a), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x74dc90 RpMaterialGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x74dc90-0x74dca2). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8d62a4, pluginID)` (call at 0x74dc9a); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x74dcb0 RpMaterialValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x74dcb0-0x74dcb5). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x74eac0 sub_74eac0 (proposed: _rpWorldPipeDummyVoid)  (catalogue: unknown/REA, port level 0)
- Shape group: empty functions (7 byte-identical functions; generated from the bytes 0x74eac0-0x74eac0). Callers: 0x75e300 _rpWorldPipeClose, 0x75e310 _rpWorldPipeOpen.
- `void ()`: a single `ret` (cdecl, no stack args popped); no effect.
- librw: absent (RW-internal boilerplate)

## 0x74fd30 RpWorldSetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x74fd30-0x74fd47). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmlwysCB` 0x808920 with `(registry = 0x8d62d0, pluginID, callback)` (push order at 0x74fd3a), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x74fd50 RpWorldSetStreamRightsCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x74fd50-0x74fd67). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmRightsCB` 0x808950 with `(registry = 0x8d62d0, pluginID, callback)` (push order at 0x74fd5a), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x74fd70 RpWorldGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x74fd70-0x74fd82). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8d62d0, pluginID)` (call at 0x74fd7a); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x74fd90 RpWorldValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x74fd90-0x74fd95). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x74ff60 RpTieSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x74ff60-0x74ff73). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8d62e8] (0x74ff68) and numBlocksToPreallocate -> [0x8d62ec] (0x74ff6d); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8d62e8] = 256, [0x8d62ec] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x74ff80 RpLightTieSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x74ff80-0x74ff93). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8d62f0] (0x74ff88) and numBlocksToPreallocate -> [0x8d62f4] (0x74ff8d); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8d62f0] = 32, [0x8d62f4] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x750e40 cb_NATIVEDATAPLUGIN_RpGeometry_StreamGetSize  (catalogue: specified/REA, port level 1)
- Shape group: one-argument forwarders (3 byte-identical functions; generated from the bytes 0x750e40-0x750e4d). Callers: 0x750050 _rpWorldObjRegisterPlugins.
- cdecl one-argument forwarder: returns `_rpD3D9NativeDataGeometryStreamGetSize` 0x763eb0(arg0 [esp+4]) (call at 0x750e45); the other arguments are ignored.
- librw: absent (RW-internal boilerplate)

## 0x750e90 cb_NATIVEDATAPLUGIN_RpWorldSector_StreamGetSize  (catalogue: specified/REA, port level 2)
- Shape group: one-argument forwarders (3 byte-identical functions; generated from the bytes 0x750e90-0x750e9d). Callers: 0x750050 _rpWorldObjRegisterPlugins.
- cdecl one-argument forwarder: returns `_rpD3D9NativeDataWorldSectorStreamGetSize` 0x7645c0(arg0 [esp+4]) (call at 0x750e95); the other arguments are ignored.
- librw: absent (RW-internal boilerplate)

## 0x7516b0 RpClumpGetWorld  (catalogue: specified/REA, port level 0)
- Shape group: plugin-offset getters (8 byte-identical functions; generated from the bytes 0x7516b0-0x7516bd). Callers: none in callgraph (reached through data / function pointer).
- `u32 (void* obj [esp+4])`, cdecl: EAX = `*(u32*)((u8*)obj + *(i32*)[0xc925ac])` (0x7516b4): reads the dword at the plugin/extension offset held in global [0xc925ac]. No NULL check of obj.
- librw: absent (RW-internal boilerplate)

## 0x7519e0 RpLightGetWorld  (catalogue: specified/REA, port level 0)
- Shape group: plugin-offset getters (8 byte-identical functions; generated from the bytes 0x7519e0-0x7519ed). Callers: none in callgraph (reached through data / function pointer).
- `u32 (void* obj [esp+4])`, cdecl: EAX = `*(u32*)((u8*)obj + *(i32*)[0xc925b0])` (0x7519e4): reads the dword at the plugin/extension offset held in global [0xc925b0]. No NULL check of obj.
- librw: absent (RW-internal boilerplate)

## 0x751d90 RpLightRegisterPluginStream  (catalogue: specified/REA, port level 1)
- Shape group: RegisterPluginStream (3 byte-identical functions; generated from the bytes 0x751d90-0x751db1). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], readCB [esp+8], writeCB [esp+0xc], getSizeCB [esp+0x10])`, cdecl: returns `_rwPluginRegistryAddPluginStream` 0x8088e0`(registry = 0x8d62f8, pluginID, readCB, writeCB, getSizeCB)` (call at 0x751da9); EAX unchanged.
- librw: base.cpp PluginList::registerStream matches in effect

## 0x751dc0 RpLightSetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x751dc0-0x751dd7). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmlwysCB` 0x808920 with `(registry = 0x8d62f8, pluginID, callback)` (push order at 0x751dca), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x751de0 RpLightGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x751de0-0x751df2). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8d62f8, pluginID)` (call at 0x751dea); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x751e00 RpLightValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x751e00-0x751e05). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x752250 RpLightSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x752250-0x752263). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8d6310] (0x752258) and numBlocksToPreallocate -> [0x8d6314] (0x75225d); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8d6310] = 32, [0x8d6314] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x7588a0 cb_RXWORLDDEVICEMODULE_RwEngine_Close  (catalogue: specified/REA, port level 0)
- Shape group: module close (count only) (7 byte-identical functions; generated from the bytes 0x7588a0-0x7588af). Callers: 0x7587d0 _rpWorldDeviceRegisterPlugins2.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl: `[0xc926e4] -= 1` (s32 module open count, read 0x7588a0 / write 0x7588a6); returns `instance` (arg 0) in EAX.
- librw: absent (RW-internal boilerplate)

## 0x758900 RpD3D9WorldSectorGetUsageFlags  (catalogue: specified/REA, port level 0)
- Shape group: plugin-offset getters (8 byte-identical functions; generated from the bytes 0x758900-0x75890d). Callers: 0x75e5e0 _rxD3D9WorldSectorDefaultInstanceCallback, 0x816120 _rpMatFXWorldSectorInstanceCallback.
- `u32 (void* obj [esp+4])`, cdecl: EAX = `*(u32*)((u8*)obj + *(i32*)[0xc926ec])` (0x758904): reads the dword at the plugin/extension offset held in global [0xc926ec]. No NULL check of obj.
- librw: absent (RW-internal boilerplate)

## 0x761c40 cb_SECTORMODULE_RwEngine_Close  (catalogue: specified/REA, port level 0)
- Shape group: module close (count only) (7 byte-identical functions; generated from the bytes 0x761c40-0x761c4f). Callers: 0x74fda0 RpWorldPluginAttach.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl: `[0xc94c0c] -= 1` (s32 module open count, read 0x761c40 / write 0x761c46); returns `instance` (arg 0) in EAX.
- librw: absent (RW-internal boilerplate)

## 0x761cf0 RpWorldSectorSetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x761cf0-0x761d07). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmlwysCB` 0x808920 with `(registry = 0x8d736c, pluginID, callback)` (push order at 0x761cfa), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x761d10 RpWorldSectorSetStreamRightsCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x761d10-0x761d27). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmRightsCB` 0x808950 with `(registry = 0x8d736c, pluginID, callback)` (push order at 0x761d1a), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x761d30 RpWorldSectorGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x761d30-0x761d42). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8d736c, pluginID)` (call at 0x761d3a); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x761d50 RpWorldSectorValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x761d50-0x761d55). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x761d60 cb_RIGHTTORENDER_RpWorld_StreamRead  (catalogue: specified/REA, port level 3)
- Shape group: right-to-render stream read (3 byte-identical functions; generated from the bytes 0x761d60-0x761d9b). Callers: 0x750050 _rpWorldObjRegisterPlugins.
- Plugin stream-read callback `RwStream* (RwStream* stream [esp+4], RwInt32 length [esp+8], void* object, RwInt32 offset, RwInt32 size)`, cdecl, ESI saved.
- `RwStreamReadInt32` 0x7ed540(stream, &[0xc94c18], 4) (0x761d6d): reads the plugin ID of the rights owner into the GLOBAL [0xc94c18] (not into the object). On 0 returns 0 (EAX of the failed call).
- If length == 8 (`cmp [esp+0xc], 8` at 0x761d7b, i.e. arg 2 after the ESI push): second `RwStreamReadInt32`(stream, &[0xc94c14], 4) reads the extra data dword into global [0xc94c14]; on 0 returns 0. Any other length: no second read.
- Returns stream. The object argument is never touched; the globals are consumed later by the stream-rights callback (RW 3.6 rights-to-render mechanism).
- librw: absent (librw does not implement right-to-render)

## 0x761e00 cb_RIGHTTORENDER_RpWorldSector_StreamRead  (catalogue: specified/REA, port level 3)
- Shape group: right-to-render stream read (3 byte-identical functions; generated from the bytes 0x761e00-0x761e3b). Callers: 0x750050 _rpWorldObjRegisterPlugins.
- Plugin stream-read callback `RwStream* (RwStream* stream [esp+4], RwInt32 length [esp+8], void* object, RwInt32 offset, RwInt32 size)`, cdecl, ESI saved.
- `RwStreamReadInt32` 0x7ed540(stream, &[0xc94c1c], 4) (0x761e0d): reads the plugin ID of the rights owner into the GLOBAL [0xc94c1c] (not into the object). On 0 returns 0 (EAX of the failed call).
- If length == 8 (`cmp [esp+0xc], 8` at 0x761e1b, i.e. arg 2 after the ESI push): second `RwStreamReadInt32`(stream, &[0xc94c10], 4) reads the extra data dword into global [0xc94c10]; on 0 returns 0. Any other length: no second read.
- Returns stream. The object argument is never touched; the globals are consumed later by the stream-rights callback (RW 3.6 rights-to-render mechanism).
- librw: absent (librw does not implement right-to-render)

## 0x7637d0 cb_BINWORLDMODULE_RwEngine_Close  (catalogue: specified/REA, port level 0)
- Shape group: module close (count only) (7 byte-identical functions; generated from the bytes 0x7637d0-0x7637df). Callers: 0x74fda0 RpWorldPluginAttach.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl: `[0xc94c24] -= 1` (s32 module open count, read 0x7637d0 / write 0x7637d6); returns `instance` (arg 0) in EAX.
- librw: absent (RW-internal boilerplate)

## 0x7c45e0 RpHAnimHierarchySetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x7c45e0-0x7c45f3). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8decd8] (0x7c45e8) and numBlocksToPreallocate -> [0x8decdc] (0x7c45ed); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8decd8] = 128, [0x8decdc] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x7c5190 RpHAnimFrameGetID  (catalogue: specified/REA, port level 0)
- Shape group: plugin-offset getters (8 byte-identical functions; generated from the bytes 0x7c5190-0x7c519d). Callers: none in callgraph (reached through data / function pointer).
- `u32 (void* obj [esp+4])`, cdecl: EAX = `*(u32*)((u8*)obj + *(i32*)[0xc9b8c8])` (0x7c5194): reads the dword at the plugin/extension offset held in global [0xc9b8c8]. No NULL check of obj.
- librw: absent (RW-internal boilerplate)

## 0x7c67f0 RpSkinSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x7c67f0-0x7c6803). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8dece0] (0x7c67f8) and numBlocksToPreallocate -> [0x8dece4] (0x7c67fd); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8dece0] = 20, [0x8dece4] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x7c6a70 cb_SKINPLUGIN_RpGeometry_Copy  (catalogue: specified/REA, port level 0)
- Shape group: return first argument (4 byte-identical functions; generated from the bytes 0x7c6a70-0x7c6a74). Callers: 0x7c6820 RpSkinPluginAttach.
- cdecl, returns its first argument [esp+4] in EAX; no other effect.
- librw: absent (RW-internal boilerplate)

## 0x7c7500 cb_SKINPLUGIN_RpAtomic_StreamWrite  (catalogue: specified/REA, port level 0)
- Shape group: return first argument (4 byte-identical functions; generated from the bytes 0x7c7500-0x7c7504). Callers: 0x7c6820 RpSkinPluginAttach.
- cdecl, returns its first argument [esp+4] in EAX; no other effect.
- librw: absent (RW-internal boilerplate)

## 0x7cb2a0 RxNodeDefinitionGetD3D9SkinAtomicAllInOne  (catalogue: specified/REA, port level 0)
- Shape group: constant getters (4 byte-identical functions; generated from the bytes 0x7cb2a0-0x7cb2a5). Callers: none in callgraph (reached through data / function pointer).
- `void* ()`, cdecl: EAX = 0x8ded08 (immediate address/constant loaded at 0x7cb2a0).
- librw: absent (RW-internal boilerplate)

## 0x7cc470 RpMaterialUVAnimSetCurrentTime  (catalogue: specified/REA, port level 4)
- Shape group: UV-anim interpolator loops (3 byte-identical functions; generated from the bytes 0x7cc470-0x7cc4a6). Callers: none in callgraph (reached through data / function pointer).
- `RpMaterial* (RpMaterial* material [esp+4], RwReal t [esp+8])`, cdecl: slot base = `material + [0xc9b8d0] + 8` (UV-anim plugin offset global, 0x7cc470); for i = 0..7 (counter EDI = 8, decrement): `interp = slot[i]`; if non-NULL call `RtAnimInterpolatorSetCurrentTime` 0x7cdab0(interp, t) (t passed as the raw 4-byte stack value, no x87 conversion). Returns material.
- librw: absent (RW-internal boilerplate)

## 0x7cc4b0 RpMaterialUVAnimAddAnimTime  (catalogue: specified/REA, port level 2)
- Shape group: UV-anim interpolator loops (3 byte-identical functions; generated from the bytes 0x7cc4b0-0x7cc4e6). Callers: 0x532d70 CEntity::MaterialUpdateUVAnimCB.
- `RpMaterial* (RpMaterial* material [esp+4], RwReal t [esp+8])`, cdecl: slot base = `material + [0xc9b8d0] + 8` (UV-anim plugin offset global, 0x7cc4b0); for i = 0..7 (counter EDI = 8, decrement): `interp = slot[i]`; if non-NULL call `RtAnimInterpolatorAddAnimTime` 0x7cd8d0(interp, t) (t passed as the raw 4-byte stack value, no x87 conversion). Returns material.
- librw: absent (RW-internal boilerplate)

## 0x7cc4f0 RpMaterialUVAnimSubAnimTime  (catalogue: specified/REA, port level 3)
- Shape group: UV-anim interpolator loops (3 byte-identical functions; generated from the bytes 0x7cc4f0-0x7cc526). Callers: none in callgraph (reached through data / function pointer).
- `RpMaterial* (RpMaterial* material [esp+4], RwReal t [esp+8])`, cdecl: slot base = `material + [0xc9b8d0] + 8` (UV-anim plugin offset global, 0x7cc4f0); for i = 0..7 (counter EDI = 8, decrement): `interp = slot[i]`; if non-NULL call `RtAnimInterpolatorSubAnimTime` 0x7cd760(interp, t) (t passed as the raw 4-byte stack value, no x87 conversion). Returns material.
- librw: absent (RW-internal boilerplate)

## 0x7cc740 RpUVAnimParamKeyFrameMulRecip  (catalogue: specified/REA, port level 0)
- Shape group: empty functions (7 byte-identical functions; generated from the bytes 0x7cc740-0x7cc740). Callers: none in callgraph (reached through data / function pointer).
- `void ()`: a single `ret` (cdecl, no stack args popped); no effect.
- librw: absent (RW-internal boilerplate)

## 0x7cc750 RpUVAnimParamKeyFrameAdd  (catalogue: specified/REA, port level 0)
- Shape group: empty functions (7 byte-identical functions; generated from the bytes 0x7cc750-0x7cc750). Callers: none in callgraph (reached through data / function pointer).
- `void ()`: a single `ret` (cdecl, no stack args popped); no effect.
- librw: absent (RW-internal boilerplate)

## 0x7ccc80 RtAnimAnimationFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x7ccc80-0x7ccc93). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8dedec] (0x7ccc88) and numBlocksToPreallocate -> [0x8dedf0] (0x7ccc8d); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8dedec] = 128, [0x8dedf0] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x7cf9a0 sub_7cf9a0  (catalogue: unknown/REA, port level 0)
- Shape group: empty functions (7 byte-identical functions; generated from the bytes 0x7cf9a0-0x7cf9a0). Callers: 0x7cf600 RtPNGImageWrite.
- `void ()`: a single `ret` (cdecl, no stack args popped); no effect.
- librw: absent (RW-internal boilerplate)

## 0x7ec760 RwStreamSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x7ec760-0x7ec773). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8e2218] (0x7ec768) and numBlocksToPreallocate -> [0x8e221c] (0x7ec76d); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8e2218] = 16, [0x8e221c] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x7ee0b0 cb_CAMERAMODULE_RwEngine_Close  (catalogue: specified/REA, port level 1)
- Shape group: module close (free list + count) (4 byte-identical functions; generated from the bytes 0x7ee0b0-0x7ee0ec). Callers: 0x7f3170 RwEngineInit.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl.
- `fl = *(RwFreeList**)(RwEngineInstance [0xc97b24] + [0xc97960])` (module offset global, 0x7ee0b5); if fl != NULL (je 0x7ee0dd at 0x7ee0c0): `RwFreeListDestroy` 0x801b80(fl), then re-reads both globals and stores 0 to that slot (0x7ee0d6).
- Always: `[0xc97964] -= 1` (module open count, 0x7ee0e3); returns `instance` in EAX.
- librw: absent (RW-internal boilerplate)

## 0x7ee0f0 RwCameraSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x7ee0f0-0x7ee103). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8e2244] (0x7ee0f8) and numBlocksToPreallocate -> [0x8e2248] (0x7ee0fd); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8e2244] = 4, [0x8e2248] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x7ee480 RwCameraGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x7ee480-0x7ee492). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8e222c, pluginID)` (call at 0x7ee48a); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x7ee4a0 RwCameraValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x7ee4a0-0x7ee4a5). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x7efed0 RwFrameSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x7efed0-0x7efee3). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8e2288] (0x7efed8) and numBlocksToPreallocate -> [0x8e228c] (0x7efedd); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8e2288] = 50, [0x8e228c] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x7eff70 cb_FRAMEMODULE_RwEngine_Close  (catalogue: specified/REA, port level 1)
- Shape group: module close (free list + count) (4 byte-identical functions; generated from the bytes 0x7eff70-0x7effac). Callers: 0x7f3170 RwEngineInit.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl.
- `fl = *(RwFreeList**)(RwEngineInstance [0xc97b24] + [0xc9796c])` (module offset global, 0x7eff75); if fl != NULL (je 0x7eff9d at 0x7eff80): `RwFreeListDestroy` 0x801b80(fl), then re-reads both globals and stores 0 to that slot (0x7eff96).
- Always: `[0xc97970] -= 1` (module open count, 0x7effa3); returns `instance` in EAX.
- librw: absent (RW-internal boilerplate)

## 0x7f1290 RwFrameGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x7f1290-0x7f12a2). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8e2270, pluginID)` (call at 0x7f129a); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x7f12b0 RwFrameValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x7f12b0-0x7f12b5). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x7f1660 cb_MATRIXMODULE_RwEngine_Close  (catalogue: specified/REA, port level 1)
- Shape group: module close (free list + count) (4 byte-identical functions; generated from the bytes 0x7f1660-0x7f169c). Callers: 0x7f3170 RwEngineInit.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl.
- `fl = *(RwFreeList**)(RwEngineInstance [0xc97b24] + [0xc979bc])` (module offset global, 0x7f1665); if fl != NULL (je 0x7f168d at 0x7f1670): `RwFreeListDestroy` 0x801b80(fl), then re-reads both globals and stores 0 to that slot (0x7f1686).
- Always: `[0xc979c0] -= 1` (module open count, 0x7f1693); returns `instance` in EAX.
- librw: absent (RW-internal boilerplate)

## 0x7f16a0 RwMatrixSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x7f16a0-0x7f16b3). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8e2290] (0x7f16a8) and numBlocksToPreallocate -> [0x8e2294] (0x7f16ad); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8e2290] = 256, [0x8e2294] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x7f2ba0 RwEngineGetVersion  (catalogue: specified/REA, port level 0)
- Shape group: constant getters (4 byte-identical functions; generated from the bytes 0x7f2ba0-0x7f2ba5). Callers: none in callgraph (reached through data / function pointer).
- `void* ()`, cdecl: EAX = 0x36003 (immediate address/constant loaded at 0x7f2ba0).
- librw: absent (RW-internal boilerplate)

## 0x7f3be0 RwTextureGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x7f3be0-0x7f3bf2). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8e23cc, pluginID)` (call at 0x7f3bea); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x7f3c00 RwTextureValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x7f3c00-0x7f3c05). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x7f3c40 RwTexDictionaryGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x7f3c40-0x7f3c52). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8e23e4, pluginID)` (call at 0x7f3c4a); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x7f3c60 RwTexDictionaryValidatePlugins  (catalogue: specified/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x7f3c60-0x7f3c65). Callers: none in callgraph (reached through data / function pointer).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x7f3e80 RwTexDictionarySetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x7f3e80-0x7f3e93). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8e2404] (0x7f3e88) and numBlocksToPreallocate -> [0x8e2408] (0x7f3e8d); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8e2404] = 5, [0x8e2408] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x7f57f0 RwD3D9VertexBufferManagerChangeDefaultSize  (catalogue: specified/REA, port level 0)
- Shape group: global setters (4 byte-identical functions; generated from the bytes 0x7f57f0-0x7f57f9). Callers: none in callgraph (reached through data / function pointer).
- `(u32 value [esp+4])`, cdecl: `[0x8e2414] = value` (0x7f57f4); EAX = value (RW 3.6 prototypes return void/RwBool: callers must not rely on more than EAX = argument).
- librw: absent (RW-internal boilerplate)

## 0x7f84e0 RwD3D9EngineGetMaxMultiSamplingLevels  (catalogue: specified/REA, port level 0)
- Shape group: global getters (4 byte-identical functions; generated from the bytes 0x7f84e0-0x7f84e5). Callers: 0x57cd50 CMenuManager::ProcessPCMenuOptions.
- `u32 ()`, cdecl: EAX = [0x8e242c] (dword global, 0x7f84e0).
- librw: absent (RW-internal boilerplate)

## 0x7f8620 RwD3D9EngineSetMultiThreadSafe  (catalogue: specified/REA, port level 0)
- Shape group: global setters (4 byte-identical functions; generated from the bytes 0x7f8620-0x7f8629). Callers: none in callgraph (reached through data / function pointer).
- `(u32 value [esp+4])`, cdecl: `[0xc97c4c] = value` (0x7f8624); EAX = value (RW 3.6 prototypes return void/RwBool: callers must not rely on more than EAX = argument).
- librw: absent (RW-internal boilerplate)

## 0x7f8630 RwD3D9EngineSetSoftwareVertexProcessing  (catalogue: specified/REA, port level 0)
- Shape group: global setters (4 byte-identical functions; generated from the bytes 0x7f8630-0x7f8639). Callers: none in callgraph (reached through data / function pointer).
- `(u32 value [esp+4])`, cdecl: `[0xc97c50] = value` (0x7f8634); EAX = value (RW 3.6 prototypes return void/RwBool: callers must not rely on more than EAX = argument).
- librw: absent (RW-internal boilerplate)

## 0x7f9d40 RwD3D9GetStencilClear  (catalogue: specified/REA, port level 0)
- Shape group: global getters (4 byte-identical functions; generated from the bytes 0x7f9d40-0x7f9d45). Callers: none in callgraph (reached through data / function pointer).
- `u32 ()`, cdecl: EAX = [0xc97c44] StencilClearValue (dword global, 0x7f9d40).
- librw: absent (RW-internal boilerplate)

## 0x7fae40 RwRasterGetPluginOffset  (catalogue: specified/REA, port level 1)
- Shape group: GetPluginOffset (11 byte-identical functions; generated from the bytes 0x7fae40-0x7fae52). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4])`, cdecl: returns `_rwPluginRegistryGetPluginOffset` 0x808470`(registry = 0x8e2518, pluginID)` (call at 0x7fae4a); -1 when the plugin is not registered (callee behaviour).
- librw: base.cpp PluginList::getPluginOffset matches in effect (returns offset or -1)

## 0x7fb350 RwRasterSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x7fb350-0x7fb363). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8e2530] (0x7fb358) and numBlocksToPreallocate -> [0x8e2534] (0x7fb35d); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8e2530] = 128, [0x8e2534] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x8020a0 _rwMemoryDefaultMalloc  (catalogue: specified/REA, port level 0)
- Shape group: one-argument forwarders (3 byte-identical functions; generated from the bytes 0x8020a0-0x8020ad). Callers: 0x801fd0 _rwMemoryOpen.
- cdecl one-argument forwarder: returns `0x824257` 0x824257(arg0 [esp+4]) (call at 0x8020a5); the other arguments are ignored.
- librw: absent (RW-internal boilerplate)

## 0x804240 sub_804240 (proposed: _rwFileSystemClose)  (catalogue: unknown/REA, port level 0)
- Shape group: empty functions (7 byte-identical functions; generated from the bytes 0x804240-0x804240). Callers: 0x7f3130 RwEngineTerm, 0x7f3170 RwEngineInit.
- `void ()`: a single `ret` (cdecl, no stack args popped); no effect.
- librw: absent (RW-internal boilerplate)

## 0x804580 RwTextureSetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x804580-0x804597). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmlwysCB` 0x808920 with `(registry = 0x8e23cc, pluginID, callback)` (push order at 0x80458a), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x8048e0 RwTexDictionaryRegisterPluginStream  (catalogue: specified/REA, port level 1)
- Shape group: RegisterPluginStream (3 byte-identical functions; generated from the bytes 0x8048e0-0x804901). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], readCB [esp+8], writeCB [esp+0xc], getSizeCB [esp+0x10])`, cdecl: returns `_rwPluginRegistryAddPluginStream` 0x8088e0`(registry = 0x8e23e4, pluginID, readCB, writeCB, getSizeCB)` (call at 0x8048f9); EAX unchanged.
- librw: base.cpp PluginList::registerStream matches in effect

## 0x804910 RwTexDictionarySetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x804910-0x804927). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmlwysCB` 0x808920 with `(registry = 0x8e23e4, pluginID, callback)` (push order at 0x80491a), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x807570 RwFrameListSetAutoUpdate  (catalogue: specified/REA, port level 0)
- Shape group: global setters (4 byte-identical functions; generated from the bytes 0x807570-0x807579). Callers: none in callgraph (reached through data / function pointer).
- `(u32 value [esp+4])`, cdecl: `[0x8e26a8] = value` (0x807574); EAX = value (RW 3.6 prototypes return void/RwBool: callers must not rely on more than EAX = argument).
- librw: absent (RW-internal boilerplate)

## 0x8075b0 RwFrameSetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x8075b0-0x8075c7). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmlwysCB` 0x808920 with `(registry = 0x8e2270, pluginID, callback)` (push order at 0x8075ba), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x8087b0 RwPluginRegistrySetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x8087b0-0x8087c3). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8e26ac] (0x8087b8) and numBlocksToPreallocate -> [0x8e26b0] (0x8087bd); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8e26ac] = 64, [0x8e26b0] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x808810 cb_ERRORMODULE_RwEngine_Close  (catalogue: specified/REA, port level 0)
- Shape group: module close (count only) (7 byte-identical functions; generated from the bytes 0x808810-0x80881f). Callers: 0x7f3170 RwEngineInit.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl: `[0xc9a6f4] -= 1` (s32 module open count, read 0x808810 / write 0x808816); returns `instance` (arg 0) in EAX.
- librw: absent (RW-internal boilerplate)

## 0x808c90 RwCameraRegisterPluginStream  (catalogue: specified/REA, port level 1)
- Shape group: RegisterPluginStream (3 byte-identical functions; generated from the bytes 0x808c90-0x808cb1). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], readCB [esp+8], writeCB [esp+0xc], getSizeCB [esp+0x10])`, cdecl: returns `_rwPluginRegistryAddPluginStream` 0x8088e0`(registry = 0x8e222c, pluginID, readCB, writeCB, getSizeCB)` (call at 0x808ca9); EAX unchanged.
- librw: base.cpp PluginList::registerStream matches in effect

## 0x808cc0 RwCameraSetStreamAlwaysCallBack  (catalogue: specified/REA, port level 1)
- Shape group: Set stream callbacks (13 byte-identical functions; generated from the bytes 0x808cc0-0x808cd7). Callers: none in callgraph (reached through data / function pointer).
- `RwInt32 (RwUInt32 pluginID [esp+4], callback [esp+8])`, cdecl: tail-calls `_rwPluginRegistryAddPlgnStrmlwysCB` 0x808920 with `(registry = 0x8e222c, pluginID, callback)` (push order at 0x808cca), pops 0xc, returns the callee's EAX unchanged.
- librw: absent (librw has no always/rights stream callbacks)

## 0x80a440 _rwStringClose  (catalogue: specified/REA, port level 0)
- Shape group: empty functions (7 byte-identical functions; generated from the bytes 0x80a440-0x80a440). Callers: 0x7f3170 RwEngineInit.
- `void ()`: a single `ret` (cdecl, no stack args popped); no effect.
- librw: absent (RW-internal boilerplate)

## 0x80aa50 cb_COLORMODULE_RwEngine_Close  (catalogue: specified/REA, port level 0)
- Shape group: module close (count only) (7 byte-identical functions; generated from the bytes 0x80aa50-0x80aa5f). Callers: 0x7f3170 RwEngineInit.
- RwEngine close callback `void* (void* instance [esp+4], RwInt32 offset, RwInt32 size)`, cdecl: `[0xc9a728] -= 1` (s32 module open count, read 0x80aa50 / write 0x80aa56); returns `instance` (arg 0) in EAX.
- librw: absent (RW-internal boilerplate)

## 0x810030 sub_810030 (proposed: _rwResourcesArenaTerm)  (catalogue: unknown/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x810030-0x810035). Callers: 0x807d80 cb_RESOURCESMODULE_RwEngine_Close, 0x8080c0 RwResourcesSetArenaSize.
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x810700 RpMatFXMaterialDataSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x810700-0x810713). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8e29b8] (0x810708) and numBlocksToPreallocate -> [0x8e29bc] (0x81070d); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8e29b8] = 128, [0x8e29bc] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x811c70 RpMatFXWorldSectorQueryEffects  (catalogue: specified/REA, port level 0)
- Shape group: plugin-offset getters (8 byte-identical functions; generated from the bytes 0x811c70-0x811c7d). Callers: none in callgraph (reached through data / function pointer).
- `u32 (void* obj [esp+4])`, cdecl: EAX = `*(u32*)((u8*)obj + *(i32*)[0xc9ab88])` (0x811c74): reads the dword at the plugin/extension offset held in global [0xc9ab88]. No NULL check of obj.
- librw: absent (RW-internal boilerplate)

## 0x816280 sub_816280 (proposed: _rpMatFXDummyTrue)  (catalogue: unknown/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x816280-0x816285). Callers: 0x810d20 cb_MATERIALEFFECTSPLUGIN_RpMaterial_Copy, 0x811410 cb_MATERIALEFFECTSPLUGIN_RpMaterial_StreamRead, 0x811c80 RpMatFXMaterialSetEffects, 0x812040 RpMatFXMaterialSetupDualTexture (+2).
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent
