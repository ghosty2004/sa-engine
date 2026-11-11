# RW D3D9 driver: light plugin and fixed-function light setup (RE-SPEC-RW-2 writer B)

The D3D9 driver keeps a per-light extension (plugin) holding the D3D light index, two D3DLIGHT9 templates and three
index arrays. Lights are given a D3D index lazily, enabled per atomic by the lighting callbacks
(RwD3D9Instance.md 0x757400, sector 0x75e570) and committed by `_rwD3D9LightsEnable`. All `__cdecl`.
Engine = `RwEngineInstance` [0xc97b24]; memory functions engine+0x134 malloc(size, hint), +0x138 free(ptr),
+0x13c realloc(ptr, size, hint).

## Layout (offsets proven at the cited instructions)

RpLight fields used: +0x1 subType (u8: 1 DIRECTIONAL, 2 AMBIENT, 0x80 POINT, 0x81 SPOT, 0x82 SPOTSOFT), +0x2 flags
(u8: 1 LIGHTATOMICS, 2 LIGHTWORLD), +0x4 frame, +0x14 radius, +0x18/+0x1c/+0x20 color r, g, b (f32), +0x34 link in
the world directional list, +0x3c u16 lightFrame (0x757474, 0x75746e, 0x7562c4, 0x75756a, 0x7562a9-0x7562bb,
0x757467, 0x757529).

Plugin extension (size 0x10, offset [0xc926b0] written by 0x755bb5): `light + [0xc926b0]`:
+0x0 RwInt32 d3dIndex (-1 = none), +0x4 f32 attenuation constant (1.0), +0x8 f32 linear (0), +0xc f32 quadratic
(5.0) (ctor 0x755bce-0x755be5; field names from the RW 3.6 SDK `RpD3D9LightSetAttenuationParams` order, the values
are the bytes).

Globals:
| addr | type | meaning | evidence |
|---|---|---|---|
| 0xc926b0 | i32 | plugin offset | 0x755bb5 |
| 0xc926b4 | u32 | max active lights (D3DCAPS9.MaxActiveLights, 0 -> 0xffffffff) | 0x755e0e-0x755e1e |
| 0xc926b8 / 0xc926bc / 0xc926c4 | u32 / u32 / i32* | free-index stack: count, capacity, array | 0x755c1d, 0x755c61, 0x755cb5 |
| 0xc926c0 | u32 | next never-used index | 0x756294-0x7562a3 |
| 0xc926c8 / 0xc926cc / 0xc926d0 | i32* / u32 / u32 | lights enabled for the CURRENT object: array, count, capacity | 0x756378, 0x756307 |
| 0xc926d4 / 0xc926d8 / 0xc926dc | i32* / u32 / u32 | lights enabled for the PREVIOUS object (same layout) | 0x756630, 0x756699-0x7566c5 |
| 0x8e2418..0x8e2424 | RwRGBAReal | `AmbientSaturated` accumulated ambient (used by RwD3D9SetSurfaceProperties) | 0x755e28 |
| 0xc92648 | D3DLIGHT9 (0x68) | directional template: Type +0 (=3), Diffuse +4, Specular +0x14, Ambient +0x24, Position +0x34, Direction +0x40, Range +0x4c, Falloff +0x50, Attenuation0..2 +0x54..+0x5c, Theta +0x60, Phi +0x64 | 0x755e50-0x755f4a, 0x7562ac, 0x7562d3 |
| 0xc925e0 | D3DLIGHT9 | local-light template (Diffuse 0xc925e4, Position 0xc92614, Range 0xc9262c, Falloff 0xc92630, Attenuation 0xc92634.., written by `_rwD3D9LightLocalEnable` 0x756390) | 0x755f54-0x755fcc, 0x7563d6, 0x756432 |

## 0x755b90 _rpD3D9LightRegisterPlugin  (catalogue: specified/REA, port level 3)
- `RwBool ()`: `[0xc926b0] = RpLightRegisterPlugin(0x10, 0x505, ctor 0x755bc0, dtor 0x755bf0, copy 0x755ce0)`
  (0x755b90-0x755bb5); return `offset >= 0` (setge, 0x755bb2). Caller 0x74ea40 (D3D9 world plugin attach).
- librw: absent (librw has no D3D light extension).

## 0x755bc0 cb_LIGHTMODULE_RpLight_Ctor  (catalogue: specified/REA, port level 0)
- `void* (void* light, RwInt32 offset, RwInt32 size)`: light == NULL -> return 0 (0x755bed). Else ext = light +
  [0xc926b0] (the global, not the offset argument): d3dIndex = -1, att0 = 1.0f, att1 = 0, att2 = 5.0f (immediates
  0x3f800000 / 0x40a00000, 0x755bce-0x755be5); return light.
- librw: absent.

## 0x755bf0 cb_LIGHTMODULE_RpLight_Dtor  (catalogue: specified/REA, port level 0)
- `void* (void* light, RwInt32, RwInt32)`; returns light (also for NULL, 0x755cd0).
- If light != NULL and `ext->d3dIndex >= 0` (signed, 0x755c0e):
  - If `freeCount (0xc926b8) >= nextIndex (0xc926c0) - 1` (signed compare, 0x755c23), i.e. this is the last used
    index: reset the allocator: freeCount = freeCap = nextIndex = 0 (0x755c2c-0x755c3a) and, if the free array
    exists, `engine->free(array)` and array = NULL (0x755c4c-0x755c57).
  - Else push the index: if `freeCount >= freeCap` (signed `jl`, 0x755c68): freeCap += 0x100 and the array is
    `malloc(freeCap*4, 0x103050d)` (array NULL) or `realloc(array, freeCap*4, 0x103050d)` (0x755c70-0x755cb0, no NULL
    check). `array[freeCount++] = ext->d3dIndex` (0x755cc2-0x755ccb).
  - The D3D light itself is not disabled here and ext->d3dIndex is not reset.
- librw: absent.

## 0x755d80 _rwD3D9LightsOpen  (catalogue: specified/REA, port level 1)
- `RwBool ()`; caller `_rpCreateD3D9AtomicAllInOnePipe` 0x74eb80.
- Free (engine+0x138) and NULL the free stack 0xc926c4, the current array 0xc926c8 and the previous array 0xc926d4,
  zeroing their counts/capacities and nextIndex (0x755d80-0x755e03).
- `[0xc926b4] = RwD3D9GetCaps()->MaxActiveLights` (+0xa0, 0x755e0e); 0 -> 0xffffffff (0x755e1e).
- AmbientSaturated = (0, 0, 0, 1.0f) (stores 0x755e28-0x755e46).
- Directional template 0xc92648: Type = 3; Diffuse = (0,0,0,1.0f); Direction = 0; Specular = (0,0,0,1.0f);
  Ambient = (0,0,0,1.0f); Position = 0; Range, Falloff, Attenuation0..2, Theta, Phi = 0 (0x755e50-0x755f4a).
- Local template 0xc925e0: Diffuse (0,0,0,1.0f), Specular (0,0,0,1.0f), Ambient (0,0,0,1.0f), Falloff = 1.0f
  (store at 0x755fcc); Type, Position, Direction, Range, attenuation, angles are not written (0x755f54-0x755fc2).
- Return 1.
- librw: absent.

## 0x756260 _rwD3D9LightDirectionalEnable  (catalogue: specified/REA, port level 4)
- `RwBool (RpLight* light)`; caller 0x757400.
- Index: if `ext->d3dIndex < 0` (0x756274): if the free stack is not empty: `ext->d3dIndex = stack[--freeCount]`
  (0x756280-0x756290); else `ext->d3dIndex = nextIndex++` (0x756294-0x7562a3).
- Template Diffuse.rgb = light color (+0x18/+0x1c/+0x20 copied as dwords, 0x7562a9-0x7562be; alpha stays 1.0);
  Direction = `RwFrameGetLTM(light->frame)->at` (LTM+0x20/+0x24/+0x28, 0x7562c8-0x7562ea).
- `RwD3D9SetLight(ext->d3dIndex, &template 0xc92648)` (0x7562f2).
- Append the index to the current array: if `count (0xc926cc) >= capacity (0xc926d0)` (unsigned, 0x756309):
  capacity += 8; array = `malloc(cap*4, 0x103050d)` (array NULL) or `realloc(array, cap*4, 0x103050d)`; the new
  slots [oldCap, newCap) are zeroed (0x756354-0x75636a). `array[count++] = index` (0x756378-0x756382). Return 1.
- librw: d3d/d3d9render.cpp lightingCB_Fix / d3ddevice light setup differs (direct SetLight per frame with fixed
  indices, no index allocator).

## 0x756600 _rwD3D9LightsEnable  (catalogue: specified/REA, port level 1)
- `void (RwBool enable, RwUInt32 /*unused*/)`; callers 0x757400 and 0x75e570.
- enable == 0: tail call `RwD3D9SetRenderState(0x89 LIGHTING, 0)` (0x7566f7). Both lists are left untouched (the
  device lights stay enabled; the current count is not cleared).
- Else: if `count (0xc926cc) > maxActive (0xc926b4)` (unsigned) -> count = maxActive (0x756617-0x75661d).
  For each index p in the PREVIOUS list (0xc926d4, count 0xc926d8): linear search in the first `count` entries of
  the current list (0x756644-0x756652); not found -> `RwD3D9EnableLight(p, 0)` (0x756657).
  If count != 0: `RwD3D9EnableLight(cur[i], 1)` for i < count (0x75667a-0x756697; RwD3D9EnableLight 0x7fa860 only
  calls `device->LightEnable` (vtbl+0xd4) when its cache differs); then swap the two arrays and their capacities and
  set prevCount = count (0x756699-0x7566c5). If count == 0: prevCount = 0 (0x7566cd).
  current count = 0 (0x7566d7); tail call `RwD3D9SetRenderState(0x89 LIGHTING, 1)` (0x7566e2-0x7566f2).
- Edge: with count clamped to maxActive the extra indices stay in the array but are dropped (count only).
- librw: absent (librw enables lights directly in lightingCB).

## Open questions
- UNCLEAR: the copy callback 0x755ce0 and `_rwD3D9LightLocalEnable` 0x756390 are outside this assignment; the local
  template fields listed above come from the stores seen at 0x7563d6-0x75644b only.
