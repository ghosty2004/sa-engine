# RW D3D9 driver: lights - pass-3 specs, writer A (RE-SPEC-RW-3)

RW 3.6 D3D9 world-pipeline light code. gta-reversed only has thunks. All `__cdecl`. Extends
docs/re/specs/RW/RwD3D9Lights.md (pass 1/2, read-only), whose layout section (RpLight fields, the extension at `light +
[0xc926b0]`, the index stacks 0xc926b8..0xc926dc and the D3DLIGHT9 templates 0xc92648 / 0xc925e0) applies.

## Layouts used (offsets proven by the instruction addresses given)

- Extension `ext = light + [0xc926b0]` (0x755cf0): +0x0 i32 d3dIndex (0x7560d2), +0x4/+0x8/+0xc attenuation constant /
  linear / quadratic (0x755d2a, 0x75642f..0x756448).
- Index allocation (inlined twice, 0x7560d8..0x756109 and 0x7563a1..0x7563ce): if `ext->d3dIndex < 0`: if the free stack
  count `[0xc926b8]` != 0 -> `index = [0xc926c4][--count]`; else `index = [0xc926c0]++`.
- "Current object" index list append (inlined, 0x75615d..0x7561e6 / 0x756550..0x7565e5): if `[0xc926cc] >= [0xc926d0]`
  (unsigned): capacity += 8, array `[0xc926c8]` = `rwrealloc(arr, cap*4, 0x103050d)` or `rwmalloc(cap*4, 0x103050d)` when
  NULL (no NULL check), new slots zeroed; then `arr[count++] = index`.
- Directional template 0xc92648: Diffuse rgb 0xc9264c/0xc92650/0xc92654, Direction 0xc92688/0xc9268c/0xc92690. Local
  template 0xc925e0: Type 0xc925e0, Diffuse 0xc925e4.., Position 0xc92614.., Direction 0xc92620.., Range 0xc9262c,
  Attenuation0/1/2 0xc92634/0xc92638/0xc9263c, Theta 0xc92640, Phi 0xc92644.
- `AmbientSaturated` 0x8e2418 (r), 0x8e241c (g), 0x8e2420 (b), 0x8e2424 (a). `RwGlobals` +0x4 current world (0x756096);
  `RpWorld` +0x3c directional light list head (links = light + 0x34, 0x756099/0x7560a8).
- `RxPipelineNode` +0x14 private data (`_rxD3D9InstanceNodeData`): +0x8 lighting callback, +0xc render callback (0x7573c8).
- Constants `[0x858b50] = 0.0f`, `[0x858cb8] = 3.1415927f`, `[0x858cdc] = 0.001f`.

## 0x755ce0 cb_LIGHTMODULE_RpLight_Copy  (catalogue: specified/REA, port level 0)
- `void* (void* dst, const void* src, i32, i32)`: if both non-NULL copy the 16 extension bytes (0x755cfb..0x755d0f).
  Returns EAX = dst (left from the load). The copied d3dIndex makes two lights share a D3D slot (kept).
- librw: absent.

## 0x755d20 RpD3D9LightSetAttenuationParams  (catalogue: specified/REA, port level 0)
- `(light, const f32 params[3])`: copy 3 dwords to `ext+4..+0xc` (0x755d32..0x755d3f); EAX = ext+... (garbage-ish; callers
  ignore).
- librw: absent.

## 0x755d50 RpD3D9LightGetAttenuationParams  (catalogue: specified/REA, port level 0)
- Copies `ext+4..+0xc` to the out array (0x755d62..0x755d6f).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x755fe0 _rwD3D9LightsClose  (catalogue: specified/REA, port level 0)
- `rwfree([0xc926c4])` if set and NULL it; `[0xc926b8] <- 0`, `[0xc926bc] <- 0`, `[0xc926c0] <- 0`; same free/NULL for
  `[0xc926c8]` (count `[0xc926cc]`, capacity `[0xc926d0]` zeroed) and `[0xc926d4]` (`[0xc926d8]`, `[0xc926dc]` zeroed)
  (0x755fe0..0x756063).
- librw: absent.

## 0x756070 _rwD3D9LightsGlobalEnable  (catalogue: specified/REA, port level 1)
- `RwBool (u8 lightFlagsMask)`: `AmbientSaturated = {0, 0, 0}` and `.a <- 1.0f` (0x75607a..0x756090); `any = 0`. For each
  link of `world->directionalLights` (0x7560a8..0x756224): `l = link - 0x34`; skip if `l == NULL` or `(mask & l->flags) == 0`;
  if `l->subType == 1` (DIRECTIONAL): allocate `ext->d3dIndex` (see layouts), template Diffuse = `l->color.rgb`, Direction =
  `RwFrameGetLTM(l->frame)->at` (0x75612e..0x756150), `RwD3D9SetLight(index, 0xc92648)` (0x756158), append index to the
  current list. Else: `AmbientSaturated.k = f32(AmbientSaturated.k + l->color.k)` for r, g, b (x87, 0x7561ed..0x756214).
  Every accepted light sets `any = 1` (0x75621a).
- End: `RwD3D9SetRenderState(0x8b D3DRS_AMBIENT, any ? 0xffffffff : 0)` (0x75622f / 0x756244); returns `any`.
- librw: d3d/d3d9render.cpp lighting setup differs (librw builds lights per object, no free-index stack).

## 0x756390 _rwD3D9LightLocalEnable  (catalogue: specified/REA, port level 1)
- `RwBool (RpLight* l)`: allocate `ext->d3dIndex` (before any test). Template Diffuse = color (0x7563d3..0x7563e8), Position
  = LTM pos (0x7563fb..0x75640f), `Range <- f32(radius)` (`fst`, 0x75641e). `radius == 0.0` (exactly C3, 0x756429) ->
  return 0 (the allocated index is kept). Attenuation0 = `ext->constant`, Attenuation1 = `f32(ext->linear / radius)`,
  Attenuation2 = `f32(ext->quadratic / (radius*radius))` (0x75642f..0x75644b).
  - subType 0x80 POINT: Type `<- 1`, Direction = 0, Theta = Phi = 0 (0x75645d..0x75648f).
  - 0x81 SPOT: Type `<- 2`, Direction = LTM at; `phi = 2*RpLightGetConeAngle(l)` (`fadd st0,st0`); Theta = `phi < pi` ?
    phi : `phi - 0.001` (0x7564d4..0x7564f1); Phi = phi (0x75653d).
  - 0x82 SPOTSOFT: Type `<- 2`, Direction = LTM at, Theta = 0, Phi = `2*coneAngle` (0x756504..0x75653d).
  - other types -> return 0 (0x7565f2).
  `RwD3D9SetLight(index, 0xc925e0)` (0x75654b); append index to the current list; return 1.
- Edge: Falloff (0xc92630) is never written here (static template value).
- librw: absent (librw sets D3D lights in its own code with different attenuation).

## 0x7573c0 RxD3D9AllInOneSetLightingCallBack  (catalogue: specified/REA, port level 0)
- `node->privateData(+0x14)->+0x8 = cb` (0x7573cb); EAX = node.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7573d0 RxD3D9AllInOneGetLightingCallBack  (catalogue: specified/REA, port level 0)
- Returns `node->privateData->+0x8` (0x7573d7).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7573f0 RxD3D9AllInOneGetRenderCallBack  (catalogue: specified/REA, port level 0)
- Returns `node->privateData->+0xc` (0x7573f7).
- librw: no direct equivalent identified in librw (not compared line by line).

