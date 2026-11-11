# RenderWare 3.6 engine life cycle, errors and helpers (RW core) - full specs, writer A (RE-SPEC-RW)

RenderWare 3.6 code linked into gta_sa.exe 1.0 US; gta-reversed/plugin-sdk have thunks only. All `__cdecl`.

## Layouts and globals
- `RwEngineInstance` dword at 0xC97B24 (`G`); static RwGlobals image at 0xC979C8 (0x158 bytes = 0x56 dwords,
  0x7F2FF8); `G+0x10` RwDevice copy (`fpSystem` at device+0x4, 0x7F2ACF); `G+0x48` stdFunc[29]; `G+0xF0`
  RwStringFunctions (17 pointers, SDK rwplcore.h:3268 order); `G+0x134` memory functions; `G+0x144/+0x148` freelist
  alloc/free; `G+0x150` engineStatus (0 none, 1 INITED 0x7F3476, 2 OPENED 0x7F306C, 3 STARTED 0x7F2ED1);
  `G+0x154` resArenaInitSize (0x7F31C6). Static image words 0xC97B0C / 0xC97B10 = its +0x144/+0x148, 0xC97B00 = its
  +0x138 (rwfree).
- 0xC97B20 number of open engine instances (0x7F305F). Engine registry 0x8E2298 (sizeOfStruct 0x158 base).
- ERRORMODULE (engine plugin 0x40F, 8 bytes): offset global 0xC9A6F0, instance counter 0xC9A6F4; module =
  `RwError {i32 pluginID +0, i32 errorCode +4}`; "no error" = {0, 0x80000000 E_RW_NOERROR}.
- Device system requests (`_rwDeviceSystemRequest` second argument): 0 OPEN, 2 START, 3 STOP, 4 REGISTER, 5
  GETNUMMODES, 6 GETMODEINFO, 7 USEMODE, 0xA GETMODE, 0xB STANDARDS, 0xD GETNUMSUBSYSTEMS, 0xE GETSUBSYSTEMINFO,
  0xF GETCURRENTSUBSYSTEM, 0x10 SETSUBSYSTEM, 0x11 FINALIZESTART, 0x12 INITIATESTOP.

## 0x7F2AB0 _rwDeviceSystemRequest  (catalogue: specified/REA, port level 1)
- `RwBool (RwDevice* dev, RwInt32 request, void* pOut, void* pInOut, RwInt32 nIn)`; callers (17).
- Behaviour: `r = dev->fpSystem(request, pOut, pInOut, nIn)`; `r != 0` -> return r. Otherwise default handling
  (jump table 0x7F2B70 for requests 0xD..0x12):
  - 0xD: `*(i32*)pOut = 1`; return 1.
  - 0xE: if `nIn == 0`: `G->strcpy (G+0xF8)(pOut, (char*)0x8E23B0 "Only rendering sub system")`; return 1; else error.
  - 0xF: `*(i32*)pOut = 0`; return 1.
  - 0x10: `nIn == 0` -> return 1; else error.
  - 0x11, 0x12: return 1.
  - error / any other request: RwErrorSet({1, _rwerror(0x18 E_RW_SYSTEMREQUEST, request)}); return 0.

## 0x7F2B90 _rwGetNumEngineInstances  (catalogue: unknown/REA, port level 0)
- `RwInt32 (void)`: returns `[0xC97B20]`. Caller `_rwPluginRegistryAddPlugin`.

## 0x7F2C00 RwEngineGetNumSubSystems  (catalogue: specified/REA, port level 2)
- `RwInt32 (void)`: `n = 1; _rwDeviceSystemRequest(&G->dOpenDevice, 0xD, &n, NULL, 0)`; returns n (request result
  ignored). Caller Ps::psSelectDevice.

## 0x7F2C30 RwEngineGetSubSystemInfo  (catalogue: specified/REA, port level 2)
- `RwSubSystemInfo* (RwSubSystemInfo* info, RwInt32 index)`: request 0xE `(dev, 0xE, info, NULL, index)`; returns
  info if nonzero else NULL.

## 0x7F2C60 RwEngineGetCurrentSubSystem  (catalogue: specified/REA, port level 2)
- `RwInt32 (void)`: request 0xF into a local; returns it, or -1 if the request returned 0.

## 0x7F2C90 RwEngineSetSubSystem  (catalogue: specified/REA, port level 2)
- `RwBool (RwInt32 index)`: request `(dev, 0x10, NULL, NULL, index)`; returns 1 if nonzero else 0.

## 0x7F2CC0 RwEngineGetNumVideoModes  (catalogue: specified/REA, port level 2)
- `RwInt32 (void)`: request 5 into a local; returns it or -1.

## 0x7F2CF0 RwEngineGetVideoModeInfo  (catalogue: specified/REA, port level 2)
- `RwVideoMode* (RwVideoMode* info, RwInt32 index)`: request `(dev, 6, info, NULL, index)`; info or NULL.

## 0x7F2D20 RwEngineGetCurrentVideoMode  (catalogue: specified/REA, port level 2)
- `RwInt32 (void)`: request 0xA into a local; returns it or -1.

## 0x7F2D50 RwEngineSetVideoMode  (catalogue: specified/REA, port level 2)
- `RwBool (RwInt32 index)`: request `(dev, 7, NULL, NULL, index)`; 1 if nonzero else 0.

## 0x7F2E70 RwEngineStart  (catalogue: specified/REA, port level 2)
- `RwBool (void)`; caller Rs::RsRwInitialize 0x619C90.
- Behaviour: 1. `_rwDeviceSystemRequest(&G->dOpenDevice, 2 START, NULL, NULL, 0)`; 0 -> return 0.
  2. `_rwPluginRegistryInitObject(0x8E2298, G)` (runs every engine module Open callback in registration order); NULL ->
  request 3 STOP, return 0. 3. `RwImageSetGamma(G->dOpenDevice.gammaCorrection)` (0x803FE0, float dword at G+0x10).
  4. request 0x11 FINALIZESTART (result ignored; sets x87 PC=24 via `_rwProcessorInitialize` 0x857404).
  5. `G->engineStatus = 3`; return 1. No engineStatus check on entry.

## 0x7F2F70 RwEngineOpen  (catalogue: specified/REA, port level 2)
- `RwBool (RwEngineOpenParams* params)`; caller Rs::RsRwInitialize.
- Behaviour:
  1. If `[0xC97B24] == NULL` -> `[0xC97B24] = 0xC979C8`.
  2. If `G->engineStatus != 1` -> error {1, _rwerror(0x80000001 E_RW_BADENGINESTATE)}; return 0.
  3. If `params == NULL` -> error {1, 0x80000016 E_RW_NULLP}; return 0.
  4. `dev = _rwDeviceGetHandle()` (0x7F9C20); NULL -> return 0 (no error).
  5. `g = G->rwmalloc([0x8E2298] (engine registry sizeOfStruct, 0x5B0 with all plugins), 0x40000)`; NULL ->
     `[0xC97B24] = 0xC979C8`; error {1, _rwerror(0x80000013, size)}; return 0.
  6. Copy 0x56 dwords from 0xC979C8 to g; `[0xC97B24] = g`. (Plugin data beyond 0x158 stays uninitialised until
     RwEngineStart.)
  7. `_rwDeviceSystemRequest(dev, 4 REGISTER, &g->dOpenDevice, &g->memoryFuncs, 0)` (result ignored).
  8. `_rwDeviceSystemRequest(dev, 0 OPEN, NULL, params, 0)`; 0 -> copy 0x56 dwords from g back to 0xC979C8,
     `[0xC97B24] = 0xC979C8`, call `[0xC97B00]` (the now-copied rwfree)(g); return 0.
  9. `_rwDeviceSystemRequest(dev, 0xB STANDARDS, &g->stdFunc, NULL, 0x1D)`; `[0xC97B20]++`; `g->engineStatus = 2`;
     return 1.

## 0x7F3170 RwEngineInit  (catalogue: specified/REA, port level 5)
- `RwBool (const RwMemoryFunctions* memFuncs, RwUInt32 initFlags, RwUInt32 resArenaSize)`; caller Rs::RsRwInitialize.
- Behaviour:
  1. `[0xC97B24] = 0xC979C8`.
  2. If `initFlags & 1` (rwENGINEINITNOFREELISTS, low byte tested): static `+0x144 = 0x7F3490` (`_rwMallocBypass`),
     `+0x148 = 0x7F34B0`; `_rwFreeListSetFlags(0)`. Else `+0x144 = 0x801C30`, `+0x148 = 0x801D50`; `_rwFreeListSetFlags(1)`.
  3. `G->resArenaInitSize = resArenaSize`.
  4. If `G->engineStatus != 0` -> return 0 (no error).
  5. `_rwStringOpen()`; 0 -> return 0. `_rwMemoryOpen(memFuncs)`; 0 -> `_rwStringClose` (0x80A440), return 0.
     `_rwFileSystemOpen()` (0x804140); 0 -> `_rwMemoryClose` (0x8020F0), `_rwStringClose`, return 0.
     `0x8082E0` (plugin-registry freelist, RwMemory.md); 0 -> `_rwFileSystemClose` 0x804240, memory close, string close,
     return 0.
  6. Register the core modules with `_rwPluginRegistryAddPlugin(0x8E2298, size, id, open, close, NULL)` in this order:
     ERROR (8, 0x40F, 0x8087D0, 0x808810), VECTOR (0x18, 0x401, 0x7EDE90, 0x7EDE20), COLOR (0, 0x40D, 0x80AA40, 0x80AA50),
     MATRIX (0x18, 0x402, 0x7F16C0, 0x7F1660), FRAME (4, 0x403, 0x7EFEF0, 0x7EFF70), STREAM (4, 0x404, 0x7EC780, 0x7EC7E0),
     CAMERA (4, 0x405, 0x7EE110, 0x7EE0B0), IMAGE (0x220, 0x406, 0x802280, 0x8024E0), RASTER (0x64, 0x407, 0x7FB370,
     0x7FB310), TEXTURE (0x34, 0x408, 0x7F3EA0, 0x7F3D00), PIPE (0x60, 0x409, 0x807C40, 0x807C60), CHUNKGROUP (4, 0x412,
     0x80A780, 0x80A7E0), then `_rwPipeAttachDummy` 0x807C70 (returns 1), then IMMEDIATE (0x74, 0x40A, 0x7EFE20,
     0x7EFDE0), RESOURCES (0x28, 0x40B, 0x807C90, 0x807D80). All results (offsets and the 0x807C70 result) are OR'ed;
     if the OR is negative (any -1, `jge` at 0x7F3445) -> cleanup: `_rwResourcesClose` 0x808320, file close, memory
     close, string close; return 0. Registration does not stop at the first failure.
  7. `_rwDeviceRegisterPlugin()` 0x7F5F60; 0 -> same cleanup, return 0.
  8. `G->engineStatus = 1`; return the (nonzero) result of `_rwDeviceRegisterPlugin`.

## 0x8087D0 cb_ERRORMODULE_RwEngine_Open  (catalogue: specified/REA, port level 0)
- `void* Open(instance, offset, size)`: `[0xC9A6F0] = offset; [0xC9A6F4]++`; module = {0, 0x80000000}; return instance.

## 0x808820 RwErrorSet  (catalogue: specified/REA, port level 0)
- `RwError* (RwError* err)`; returns err. Callers (124).
- Behaviour: module `e = G + [0xC9A6F0]`. Only if `e->pluginID == 0 && e->errorCode == 0x80000000` (nothing pending):
  `e->pluginID = (err->errorCode & 0x80000000) ? 0 : err->pluginID`; `e->errorCode = err->errorCode`. Otherwise the
  new error is dropped (the first error since the last RwErrorGet wins).
- librw: error.cpp:setError differs (librw always overwrites).

## 0x808880 RwErrorGet  (catalogue: specified/REA, port level 0)
- `RwError* (RwError* out)`: `*out = module`; module = {0, 0x80000000}; returns out. Caller: MatFX material stream read.

## 0x8088D0 _rwerror  (catalogue: specified/REA, port level 0)
- `RwInt32 (RwInt32 code, ...)`: returns code (extra arguments ignored; the debug build would format them).

## 0x809160 _rwSListCreate  (catalogue: specified/REA, port level 1)
- `RwSList* (RwInt32 entrySize, RwUInt32 hint)`; RwSList = {elements +0x0, numElementsFilled +0x4, numElementsAlloced
  +0x8, entrySize +0xC}. Callers: RtDict schema functions.
- Behaviour: `s = G->rwmalloc(0x10, 0x30000)`; NULL -> error {1, _rwerror(0x80000013, 0x10)}, return NULL.
  `s->entrySize = entrySize; s->numFilled = 0; s->numAlloced = 0x14`; `s->elements = G->rwmalloc(entrySize * 20,
  hint | 0x1000000)`; NULL -> error {1, _rwerror(0x80000013, 20 * entrySize)}, `G->rwfree(s)`, return NULL. Return s.

## 0x80A240 _rwStringOpen  (catalogue: specified/REA, port level 0)
- `RwBool (void)`: fills G+0xF0..G+0x130 in SDK order: sprintf 0x821BB5, vsprintf 0x8230B8, strcpy 0x826590, strncpy
  0x821F40, strcat 0x8265A0, strncat 0x826450, strrchr 0x80A420, strchr 0x80A400, strstr 0x822650, strcmp 0x8263C0,
  strncmp 0x8214D0, stricmp 0x80A350, strlen 0x826330, strupr 0x80A3A0, strlwr 0x80A3D0, strtok 0x82244B, sscanf 0x8220AD.
  Returns 1.

## 0x80A780 cb_CHUNKGROUPMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- `void* Open(instance, offset, size)`: `[0xC9A6F8] = offset`; `G+offset+0 = RwFreeListCreateAndPreallocateSpace(0x21,
  [0x8E26D8] (init 0x10), 4, [0x8E26DC] (init 1), (void*)0xC9A700, 0x40412)`; NULL -> return NULL; `[0xC9A6FC]++`;
  return instance.

## 0x80AA40 cb_COLORMODULE_RwEngine_Open  (catalogue: specified/REA, port level 0)
- `void* Open(instance, offset, size)`: `[0xC9A728]++`; return instance (size-0 module).

## 0x80DBE0 _rwIntelMMXsupported  (catalogue: specified/REA, port level 0)
- `RwBool (void)`: CPUID availability test (toggle EFLAGS bit 21 via pushf/popf; restored afterwards); unavailable ->
  0. Else `cpuid(1)`; return `(edx >> 23) & 1`. Caller D3D9System. Port: query the host CPU (or return the value the
  original would see) - only selects transform kernels.

## 0x80DC40 _rwIntelSSEsupported  (catalogue: specified/REA, port level 0)
- Same test; returns `(edx & 0x06000000) != 0` (SSE OR SSE2 bit).

## 0x80DCA0 _rwIntelSSE2supported  (catalogue: specified/REA, port level 0)
- Same test; returns `(edx >> 26) & 1`.

## 0x80DD00 _rwAMD3DNowSupported  (catalogue: specified/REA, port level 0)
- Same EFLAGS test; `cpuid(0x80000000)`: if `eax <= 0x80000000` return 0; `cpuid(0x80000001)`: return `(edx & 0x80000000) != 0`.

## Open questions
- None.
