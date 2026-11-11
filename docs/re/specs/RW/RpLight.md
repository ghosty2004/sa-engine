# RpLight (RenderWare 3.6 RpWorld plugin, statically linked) - full specs, writer B (RE-SPEC-RW)

All functions are `__cdecl` (caller pops). `Engine` = `[RwEngineInstance 0xc97b24]`; `+0x144`/`+0x148` freelist alloc/free.

## Layouts used (verified against the bytes)

RpLight (registry 0x8d62f8, base 0x40; freelist object; matches objects.md):

| off | field | proving instruction |
|---|---|---|
| +0x00 u8 | object.type = 3 (rpLIGHT) | 0x75213e |
| +0x01 u8 | object.subType = light type (>= 0x80 = local light: point/spot) | 0x752141, 0x751922 |
| +0x02 u8 | object.flags (create: 3 = LIGHTATOMICS\|LIGHTWORLD) | 0x75218a, 0x752010 |
| +0x03 u8 | object.privateFlags: 1 = colour is grey (r == g == b) | 0x75216b, 0x751acf |
| +0x04 ptr | object.parent (RwFrame*) | 0x75215b, 0x751a79 |
| +0x08 RwLLLink | lFrame (not initialised by create) | - |
| +0x10 fn | sync (default 0x7521a0) | 0x75215e |
| +0x14 f32 | radius | 0x752165, 0x751a7c |
| +0x18 RwRGBAReal | color r +0x18, g +0x1c, b +0x20, a +0x24 | 0x752149..0x752152 |
| +0x28 f32 | minusCosAngle | 0x752168, 0x751fd2 |
| +0x2c RwLinkList | WorldSectorsInLight (links = RpLightTie+0xc) | 0x752155, 0x75196c |
| +0x34 RwLLLink | inWorld | 0x752174/0x752177, 0x751943 |
| +0x3c u16 | lightFrame | 0x752190 |

RpLightTie (0x18, freelist `Engine + [0xc9259c] + 4` = WORLDOBJMODULE engine data +4): +0x0 RwLLLink lightInWorldSector,
+0x8 light, +0xc RwLLLink WorldSectorInLight, +0x14 sector (0x751981..0x75199e). RpWorld +0x34 lightList (local lights),
+0x3c directionalLightList (0x751929 / 0x75193e). WORLDOBJMODULE light ext at `light + [0xc925b0]`: +0 = RpWorld* (0x75191f).
RwGlobals +0xa u16 lightFrame (0x752186). LIGHTMODULE: offset global 0xc925d8 (engine data +0 = light freelist), open
counter 0xc925dc, static freelist 0xc925b4, params 0x8d6310 = 0x20 per block, 0x8d6314 = 1.

## 0x751910 RpWorldAddLight  (catalogue: specified/REA, port level 1)
- cdecl `RpWorld* (RpWorld* world, RpLight* light)`.
- `*(light + [0xc925b0]) = world`. If `subType < 0x80` (unsigned byte, 0x751925): list = `&world->directionalLightList (+0x3c)`.
  Else: if `light->parent`: `RwFrameUpdateObjects(parent)` (0x7f0910; the light sync then ties it to sectors); list =
  `&world->lightList (+0x34)`. Insert `light+0x34` at the head of list. Return world.
- librw: world.cpp:World::addLight matches in outline (librw: `if(l->getFrame()) l->getFrame()->updateObjects()` for local).

## 0x751960 RpWorldRemoveLight  (catalogue: specified/REA, port level 0)
- cdecl `RpWorld* (RpWorld* world, RpLight* light)`.
- `*(light + [0xc925b0]) = NULL`. For each link L of `light->WorldSectorsInLight` (+0x2c; next read before freeing):
  `tie = L - 0xc`; unlink L (`L.prev->next = L.next; L.next->prev = L.prev`); unlink `tie->lightInWorldSector` (+0/+4) from the
  sector's list; free tie to `*(Engine + [0xc9259c] + 4)` via `Engine+0x148`. Then unlink `light+0x34` from whichever world
  list holds it (link fields not cleared). Return world.
- librw: world.cpp:World::removeLight differs (no sector ties).

## 0x751a70 RpLightSetRadius  (catalogue: specified/REA, port level 1)
- cdecl `RpLight* (RpLight* light, f32 radius)`: `light->radius = radius` (raw dword); if `parent`: `RwFrameUpdateObjects(parent)`;
  return light.

## 0x751a90 RpLightSetColor  (catalogue: specified/REA, port level 0)
- cdecl `RpLight* (RpLight* light, const RwRGBAReal* c)`: copy the 4 floats to +0x18. Then
  `privateFlags (+3) = (r == g && r == b) ? 1 : 0` (fcomp + `test ah,0x44; jp`: unordered (NaN) counts as not equal). Return light.
- librw: light.cpp:Light::setColor matches (sets a "gray" private flag).

## 0x751d60 RpLightRegisterPlugin  (catalogue: specified/REA, port level 2)
- cdecl `i32 (size, pluginID, ctor, dtor, copy)` -> `_rwPluginRegistryAddPlugin(0x8d62f8, ...)` 0x8084a0.

## 0x751f00 RpLightStreamRead  (catalogue: specified/REA, port level 5)
- Convention: cdecl `RpLight* (RwStream* stream)`; called after rwID_LIGHT (0x12) was found. The argument slot is reused as
  the chunk `len`.
- Behaviour:
  1. `FindChunk(stream, 1, &len, &ver)`; fail -> NULL (no error). `ver` outside [0x34000, 0x36003] -> error `{2, 0x80000004}`, NULL.
  2. Local 0x18-byte struct (zeroed): `{f32 radius; f32 red; f32 green; f32 blue; f32 minusCosAngle; u16 flags; u16 type}`.
     `RwStreamRead(stream, &s, len) != len` -> NULL (len not clamped).
  3. `l = RpLightCreate((u8)s.type)`; NULL -> NULL.
  4. `radius = s.radius; color.r/g/b = s.red/green/blue` (alpha stays 1.0f from create).
  5. If `ver < 0x30300` (dead here because of step 1): `minusCosAngle = -_rwSqrt( [0x858624]=1.0f / (s.mca*s.mca + 1.0f) )`
     (x87: `t = s.mca * s.mca; t += 1.0f; t = 1.0f / t` stored f32, sqrt, fchs, stored f32). Else `minusCosAngle = s.minusCosAngle` (raw).
  6. `privateFlags = (r == g && r == b) ? 1 : 0` (as in RpLightSetColor). `object.flags = (u8)s.flags`.
  7. `_rwPluginRegistryReadDataChunks(0x8d62f8, stream, l)`; return `result ? l : NULL` (the light is leaked on failure).
- librw: light.cpp:Light::streamRead differs: reads a fixed 24 bytes, uses `setColor`, version-gated cone formula with
  different constants, no leak.

## 0x7520d0 RpLightDestroy  (catalogue: specified/REA, port level 1)
- cdecl `RwBool (RpLight* light)`: `DeInitObject(0x8d62f8, light)` (WORLDOBJ / CLUMP / D3D9 dtors); `_rwObjectHasFrameReleaseFrame(light)`
  (0x804f40); free to the light freelist `*(Engine + [0xc925d8])`; return 1.

## 0x752110 RpLightCreate  (catalogue: specified/REA, port level 1)
- cdecl `RpLight* (i32 type)`. `l = Engine->memoryAlloc(*(Engine+[0xc925d8]), 0x30012)`; NULL -> NULL.
- Init (store order): `type = 3; subType = (u8)type; color = {1,1,1,1}` ([imm 0x3f800000] = 1.0f); `flags = 0; parent = NULL;
  sync = 0x7521a0; radius = 0; minusCosAngle = 0; privateFlags = 1; WorldSectorsInLight` empty; `inWorld = {0,0}`; `flags = 3`;
  `lightFrame = Engine->lightFrame (+0xa) - 1` (u16). `_rwPluginRegistryInitObject(0x8d62f8, l)`; return l.
- Edge: does not call _rwObjectHasFrameSetFrame (lFrame left uninitialised; parent NULL is what matters).
- librw: light.cpp:Light::create differs (setFrame(nil), cone angle defaults).

## 0x7521a0 _rpLightDefaultSyncCallBack  (catalogue: unknown/REA, port level 0)
- Name proposed: stored as the light sync by RpLightCreate (0x75215e). cdecl `RpLight* (RpLight* l)`: return l.

## 0x752270 cb_LIGHTMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- `void* (void* instance, i32 offset, i32 size)`: `[0xc925d8] = offset`; freelist =
  `RwFreeListCreateAndPreallocateSpace([0x8d62f8], [0x8d6310], 4, [0x8d6314], 0xc925b4, 0x40012)` at `Engine+offset`; NULL -> NULL.
  `[0xc925dc]++`; return instance.
