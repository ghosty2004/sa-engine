# GTA custom RW pipelines (CCustomBuildingDNPipeline, CCustomBuildingRenderer, CCustomCarEnvMapPipeline helpers) - full specs, writer A (RE-SPEC-RW-3)

Game-side D3D9 pipeline code. gta-reversed (source/game_sa/Pipelines/...) has these installed `.Reversed = false`, as
`plugin::Call`, or inlined into other functions; differences are listed per function. All `__cdecl` unless stated.

## Layouts used (offsets proven by the instruction addresses given)

- `RpMaterial`: +0x0 texture (0x5d66e7), +0x4 RwRGBA color (alpha byte +0x7, 0x5d66a8), +0xc RwSurfaceProperties
  (0x5d66d8); GTA reuses the dword +0x10 (surfaceProps.specular) as pipeline flags: bit 0 = "has env map" (written
  0x5d7126/0x5d71ae, read 0x5d6559).
- Env-map material plugin: `ext = *(RpMaterial* + [0x8d12c4])` (`CCustomCarEnvMapPipeline::ms_envMapPluginOffset`, i32,
  0x5d6553) -> `CustomEnvMapPipeMaterialData` (gr Pipelines/CustomCarEnvMap): +0 s8 scale.x, +1 s8 scale.y, +2 s8
  transScale.x, +3 s8 transScale.y (fixed 1/8: `[0x858c48] = 0.125f`), +4 u8 shininess (x `[0x859a3c] = 0.003921569f`
  = 1/255), +6 u16 renderFrame (0x5d85f5), +8 RwTexture* (0x5d65d5).
- `CustomEnvMapPipeAtomicData` (0xc): +0 f32 offsetU, +4 f32 prevPosX, +8 f32 prevPosY (0x5d87de..0x5d87e7).
- MatFX material data `*(RpMaterial* + [0xc9ab74])` (`MatFXMaterialDataOffset`): +4 = env-map texture of effect 0
  (0x5d7157).
- `RxD3D9ResEntryHeader` at `res + 0x18`: +0x4 numMeshes (0x5d6537), +0x8 indexBuffer (0x5d650b), +0x10 vertexStream
  (0x5d6525), +0x30 useOffsets (0x5d6521), +0x34 vertexDeclaration (0x5d652e), +0x40 instances (0x24 each, 0x5d6709):
  +0x8 material, +0xc vertexAlpha (0x5d669e), +0x10 vertexShader (0x5da64c), +0x14 baseIndex, +0x18 numVertices, +0x1c
  startIndex, +0x20 numPrimitives (0x5da664..0x5da671).
- `CClock`: `ms_nGameClockHours` u8 `[0xb70153]`, `ms_nGameClockMinutes` u8 `[0xb70152]`, `ms_nGameClockSeconds` u16
  `[0xb70150]`. `CCustomBuildingDNPipeline::m_fDNBalanceParam` f32 `[0x8d12c0]` (initial 1.0).
- Static D3DMATERIAL9 at 0x8d12d8 (Diffuse 0x8d12d8..0x8d12e4, Ambient 0x8d12e8..0x8d12f4, Specular 0x8d12f8..0x8d1304,
  Emissive 0x8d1308..0x8d1314, Power 0x8d1318); `AmbientSaturated` RwRGBAReal at 0x8e2418 (r), 0x8e241c (g), 0x8e2420 (b).
- Constants: `[0x859044] = 0.016666668f` (1/60), `[0x859e2c] = 360.0f`, `[0x86be9c] = 420.0f`, `[0x86be98] = 1200.0f`,
  `[0x86be94] = 1260.0f`, `[0x858b40] = 50.0f`, `[0x858b1c] = 0.1f`, `[0x86be90] = 254.0f`, `[0x858624] = 1.0f`,
  `[0x858b50] = 0.0f`.
- `G` = RwEngineInstance `[0xc97b24]`; `u16 G+0x8` = render frame counter (0x5d85e0).

## 0x5d63e0 CCustomBuildingDNPipeline::CustomPipeReinstanceCB  (catalogue: specified/REA, port level 0)
- `RwBool (void* object, RwResEntry* res, RxD3D9AllInOneInstanceCallBack instanceCB)`: if `instanceCB == NULL` return 1
  (0x5d63e6); else `r = instanceCB(object, res + 0x18, 1)` (0x5d63f7); return `r != 0 ? 1 : 0` (0x5d63fc..0x5d6401).
- gta-reversed: reversed copy elsewhere (not installed for this address); matches. librw: d3d9 reinstance differs (none).

## 0x5d6480 CCustomBuildingDNPipeline::CustomPipeRenderCB  (catalogue: specified/REA, port level 6)
- Convention: RW D3D9 render callback `void (RwResEntry* res, void* object, u8 type, u32 flags)`. Locals: `lit` (the
  `push ecx` slot), `noLight` reuses the `type` stack slot, the loop counter reuses the `object` slot.
- Behaviour:
  1. `_rwD3D9EnableClippingIfNeeded(object, type)` (0x5d6495). `RwD3D9GetRenderState(0x89 LIGHTING, &lit)` (0x5d64a4).
  2. If `lit == 0 && !(flags & 8 /*PRELIT*/)` (0x5d64b0..0x5d64b9): `noLight = 1`; `RwD3D9SetTexture(NULL, 0)`;
     `RwD3D9SetRenderState(0x3c TEXTUREFACTOR, 0xff000000)`; `SetTextureStageState(0, COLOROP 1, SELECTARG2 3)`,
     `(0, COLORARG2 3, TFACTOR 3)`, `(0, ALPHAOP 4, 3)`, `(0, ALPHAARG2 6, 3)` (0x5d64bb..0x5d64f9). Else `noLight = 0`.
  3. `hdr = res + 0x18`; if `hdr->indexBuffer` -> `_rwD3D9SetIndices(ib)` (0x5d6519); `_rwD3D9SetStreams(&hdr->vertexStream,
     hdr->useOffsets)` (0x5d6529); `_rwD3D9SetVertexDeclaration(hdr->vertexDeclaration)` (0x5d6532).
  4. If `hdr->numMeshes > 0` (signed, 0x5d653f) loop do-while over the instances:
     a. `mat = inst->material`, `ext = *(mat + [0x8d12c4])`, `env = mat->flags(+0x10) & 1` (0x5d6550..0x5d6565);
        `TSS(1, COLOROP, DISABLE)`, `TSS(2, COLOROP, DISABLE)` (0x5d6568/0x5d6573).
     b. If `env`: `(*(G + 0x20))(2, 1)` (render-state set through the engine device block: TEXTUREADDRESS = WRAP,
        0x5d658d); the env texture matrix globals `[0xc02c28] <- f32((s8)ext[0] * 0.125)`, `[0xc02c3c] <- f32((s8)ext[1] *
        0.125)`, `[0xc02c64] <- 1.0f`, `[0xc02c50] <- 1.0f` (0x5d6599..0x5d65cf); `RwD3D9SetTexture(ext->texture, 1)`
        (0x5d65d9); `k = _ftol((u8)ext[4] * (1/255) * 254.0)` (x87, 0x5d65e9..0x5d65f9; 0x821b40 truncates), `if k > 0xff
        k = 0xff` (signed, 0x5d65fe); `RwD3D9SetRenderState(0x3c, 0xff000000 | k<<16 | k<<8 | k)` (0x5d660a..0x5d6622);
        `TSS(1, COLOROP, 0x19 MULTIPLYADD)`, `(1, COLORARG0 0x1a, CURRENT 1)`, `(1, COLORARG1 2, TEXTURE 2)`, `(1, COLORARG2 3,
        0)`, `(1, ALPHAOP 4, SELECTARG2 3)`, `(1, ALPHAARG1 5, TEXTURE 2)`, `(1, ALPHAARG2 6, CURRENT 1)`, `(1, TEXCOORDINDEX
        0xb, 0x10001)`, `(1, TEXTURETRANSFORMFLAGS 0x18, 0)`, `(2, COLOROP, DISABLE)` (0x5d6627..0x5d6696).
     c. `_rwD3D9RenderStateVertexAlphaEnable(inst->vertexAlpha != 0 || mat->color.alpha != 0xff)` (0x5d669e..0x5d66b4).
     d. If `noLight == 0`: if `lit`: `RwD3D9SetSurfaceProperties(&mat->surfaceProps, &mat->color, flags)` (0x5d66dc);
        `RxD3D9InstanceDataRenderLighting(hdr, inst, flags, mat->texture)` (0x5da6a0, 0x5d66f1). Else
        `RxD3D9InstanceDataRender(hdr, inst)` (0x5da640, 0x5d66fd).
  5. After the loop: `TSS(1, COLOROP, DISABLE)`, `TSS(1, ALPHAOP, DISABLE)`, `TSS(1, TEXCOORDINDEX, 1)`,
     `TSS(1, TEXTURETRANSFORMFLAGS, 0)` (0x5d6718..0x5d673f).
- Edge: with `noLight == 1` the stage-0 TFACTOR setup is never restored here. The `(*(G+0x20))` call is the device
  render-state setter of the RW engine block (`G+0x10` device + 0x10); UNCLEAR (0x5d658d) whether a port should map it
  to `RwRenderStateSet` directly (same effect in RW 3.6).
- gta-reversed: CustomBuildingDNPipeline.cpp has an unhooked version; compare when porting (bytes win).
- librw: absent (game-side pipeline code; librw has no equivalent).

## 0x5d7120 CCustomBuildingDNPipeline::CustomPipeMaterialSetup  (catalogue: specified/REA, port level 2)
- `RpMaterial* (RpMaterial* mat, void* data)`: `mat->flags(+0x10) = 0` (0x5d7126). If `RpMatFXMaterialGetEffects(mat) ==
  2` (ENVMAP, 0x5d7135): `e = SetFxEnvTexture(mat + [0x8d12c4])` (0x5d9570; argument = address of the plugin slot); if
  `e`: `tex = (*(mat + [0xc9ab74]))->+4`; `e->texture (+8) = tex` (0x5d715c); if `tex`: `tex+0x51 = 0x11`, `tex+0x50 = 2`
  (filter LINEAR, address WRAP/WRAP, 0x5d7161/0x5d7165).
- Then `e = *(mat + [0x8d12c4])`; `bit = (e && f32((u8)e[4]) * (1/255) != 0.0 && e->texture) ? 1 : 0` (0x5d7169..0x5d71a1; the
  float compare `test ah,0x44; jnp` treats NaN as non-zero); `mat->flags = (mat->flags & ~7) | bit` (0x5d71a6..0x5d71ae);
  return mat.
- gta-reversed: unhooked C++ exists; port follows the bytes.
- librw: absent (game-side pipeline code; librw has no equivalent).

## 0x5d7f00 CCustomBuildingRenderer::AtomicSetup  (catalogue: specified/REA, port level 2)
- `void (RpAtomic* a)`: `g = a->geometry` (+0x18, 0x5d7f06); if `GetExtraVertColourPtr(g) != NULL && g->preLitLum
  (+0x30) != NULL` (0x5d7f0a..0x5d7f1b): `CCustomBuildingDNPipeline::CustomPipeAtomicSetup(a)` (0x5d71c0); else
  `CCustomBuildingPipeline::CustomPipeAtomicSetup(a)` (0x5d7e50). Return value of the callee left in EAX.
- gta-reversed: `plugin::Call` only.
- librw: absent (game-side pipeline code; librw has no equivalent).

## 0x5d7f40 CCustomBuildingRenderer::IsCBPCPipelineAttached  (catalogue: specified/REA, port level 1)
- `bool (RpAtomic* a)`: `id = PipelinePlugin::GetPipelineID(a)` (0x5d7f46); the compare `id == 0x53f2009c` is reached
  through a `.HOODLUM` hop (0x5d7f51 -> 0x401b27 `cmp eax, 0x53f2009c` -> 0x5d7f56, flags only); `id == 0x53f2009c` or
  `id == 0x53f20098` -> return 1 (0x5d7f56/0x5d7f5d). Else return `GetExtraVertColourPtr(a->geometry) != NULL &&
  a->geometry->+0x30 != NULL` (0x5d7f5f..0x5d7f77).
- gta-reversed: `plugin::CallAndReturn` only.
- librw: absent (game-side pipeline code; librw has no equivalent).

## 0x5d7f80 CCustomBuildingRenderer::UpdateDayNightBalanceParam  (catalogue: specified/REA, port level 0)
- `void (void)`: `t = f32((i32)(hours*60 + minutes) + (i32)seconds * [0x859044])` (x87 `fild; fild; fmul; faddp`,
  0x5d7f81..0x5d7fb3, stored with `fst` to an f32 temp which later loads use; the first compare uses the register value).
  - `t < 360.0` (0x5d7fb7, `jnp` on exactly C0) -> `[0x8d12c0] = 1.0` (0x5d803b).
  - else `t < 420.0` (0x5d7fc8) -> `[0x8d12c0] = (420.0 - t) * (1/60)` (0x5d7fd5..0x5d7fe5).
  - else `t < 1200.0` (0x5d7ff1) -> `0.0` (0x5d7ffe).
  - else `t < 1260.0` (0x5d8010) -> `1.0 - (1260.0 - t) * (1/60)` (0x5d801d..0x5d8033); else `1.0`.
  NaN falls to 1.0 at the first test (C0|C2|C3 set -> `jnp` not taken? no: C0 and C2 set -> even parity -> branch
  taken to 1.0 at 0x5d7fc2).
- gta-reversed: commented C++ divides by `60.0f` (exe multiplies by the f32 constant 1/60) and uses
  `CClock::GetMinutesToday()` (gr's float conversion may differ); otherwise matches.
- librw: absent (game-side pipeline code; librw has no equivalent).

## 0x5d8050 CCustomBuildingRenderer::Update  (catalogue: specified/REA, port level 1)
- `void (void)`: `UpdateDayNightBalanceParam()` (0x5d8050); `sub_5d6830(0)` (0x5d8057). gta-reversed: `plugin::Call`
  (its comment duplicates both calls; the bytes call each once).
- librw: absent (game-side pipeline code; librw has no equivalent).

## 0x5d8590 sub_5d8590 (proposed: CCustomCarEnvMapPipeline::CalcEnvMapUVOffset; catalogue: CustomPipeRenderCB_EnvMapMatrix)  (catalogue: unknown/REA, port level 4)
- Convention (register + stack): ECX = `RpAtomic*`, ESI = `CustomEnvMapPipeMaterialData*`, EDI =
  `CustomEnvMapPipeAtomicData*`, `[esp+4]` = `f32 out[2]`; returns AL = 1 (0x5d885d); caller cleans. Inlined in gr
  (CustomCarEnvMapPipeline.cpp:352 "Inline begin (@ 0x5D8590)").
- Behaviour (x87; `ftol` = 0x821b40, truncation):
  1. `f = a->clump (+0x3c) ? clump->frame : a->frame`; `M = RwFrameGetLTM(f)` (0x5d8599..0x5d85a5). `sx = f32((s8)d[2] *
     0.125 * 50.0)`, `sy = f32((s8)d[3] * 0.125 * 50.0)` (0x5d85aa..0x5d860d). `px = M->pos.x`, `py = M->pos.y`
     (0x5d85c6..0x5d85e8). `w = ad->offsetU`.
  2. If `d->renderFrame (+6) != u16 G+0x8` (0x5d85f5): `d->renderFrame = frame` (0x5d8622); `a = 1/sy`, `b = 1/sx` (f32
     temps); `ix = ftol(a * py)`, `iy = ftol(b * px)` (0x5d862e/0x5d864b); `t1 = |(iy*sx - px) * b|`, if `iy` even `t1 =
     1 - t1`; `t2 = |(ix*sy - py) * a|`, if `ix` even `t2 = 1 - t2`; `U = f32(t2 + t1)` (0x5d8656..0x5d868f). Same with the
     previous position `(qx, qy) = (ad->prevPosX, ad->prevPosY)`: `jx = ftol(a*qy)`, `jy = ftol(b*qx)`, `s1`, `s2`, `V =
     f32(s2 + s1)` (0x5d8695..0x5d86fb). `dir = (px - qx, py - qy, 0)` (0x5d8701..0x5d8711). If `dir.x^2 + dir.y^2 > 0.0`
     (0x5d8719..0x5d8736): `fwd = M->up` (+0x10); `CVector::Normalise(&dir)`, `Normalise(&fwd)`; if `fwd.z*dir.z + dir.x*fwd.x
     + fwd.y*dir.y < 0.0` (0x5d8761..0x5d8788): `w = w - |V - U|`, if `w < 0.0` `w += 1.0` (0x5d878a..0x5d87af); otherwise
     (and when not moving): `w = |V - U| + w`, if `!(w < 1.0)` `w -= 1.0` (0x5d87b1..0x5d87d0). `ad->offsetU = w`,
     `ad->prevPosX = px`, `ad->prevPosY = py` (0x5d87da..0x5d87e7).
  3. `s = M->at.y + M->at.x` (0x5d87fa..0x5d8802); if `!(s < 0.1)` -> `s = 0.1`; else if `!(s > 0.0)` -> `s = 0.0`
     (0x5d880b..0x5d8831); if `M->at.z < 0.0` -> `s = 1.0 - s` (0x5d8837..0x5d8848). `out[0] = -w` (0x5d8856), `out[1] = s`.
- librw: not RW code.

## 0x5d8c50 CustomEnvMapPipeMaterialData::ToBuffer  (catalogue: unknown/REA, port level 0)
- `void (const CustomEnvMapPipeMaterialData* d, f32* buf)`: `buf[0..3] = f32((s8)d[k] * 0.125)` for k = 0..3
  (0x5d8c54..0x5d8ca7), `buf[4] = f32((u8)d[4] * (1/255))` (0x5d8caa..0x5d8cbc), `buf[5] = d->texture` (dword copy,
  0x5d8cbf). Caller `pluginEnvMatStreamWriteCB` 0x5d8cd0.
- gta-reversed: has `FromBuffer` (0x5D8BE0) and a ToBuffer equivalent in the header; same scales.
- librw: absent (game-side pipeline code; librw has no equivalent).

## 0x5da640 RxD3D9InstanceDataRender  (catalogue: specified/REA, port level 2)
- `void (RxD3D9ResEntryHeader* hdr, RxD3D9InstanceData* inst)`: `_rwD3D9SetPixelShader(NULL)` (0x5da643);
  `_rwD3D9SetVertexShader(inst->vertexShader)` (0x5da650). If `hdr->indexBuffer` (0x5da662):
  `_rwD3D9DrawIndexedPrimitive(hdr->primType, inst->baseIndex, 0, inst->numVertices, inst->startIndex,
  inst->numPrimitives)` (0x5da679); else `_rwD3D9DrawPrimitive(hdr->primType, inst->baseIndex, inst->numPrimitives)`
  (0x5da688).
- librw: d3d9render.cpp drawInst matches in structure (librw sets no pixel shader).

## 0x5da790 sub_5da790 (proposed: CCustomCarEnvMapPipeline::SetupMaterialColors; catalogue: CustomPipeRenderCB_SetupMaterial)  (catalogue: unknown/REA, port level 1)
- Convention: `RwBool (const RwSurfaceProperties* sp, const RwRGBA* color, u32 flags, f32 specular, f32 power)`; returns 1.
- Behaviour: `Specular.r = Specular.g = Specular.b = specular` (dword stores 0x5da79d..0x5da7b3), `Power = power`
  (0x5da7b9). `f = flags & 0x48`.
  - If `f & 0x40` (MODULATEMATERIALCOLOR) and `*(u32*)color != 0xffffffff` (0x5da7be..0x5da7cb): `k = sp->diffuse * (1/255)`
    (register); `Diffuse.rgb = f32(color.c * k)`, `Diffuse.a = f32(color.a * (1/255))` (0x5da7df..0x5da830);
    `amb = f32(sp->ambient * (1/255))` (0x5da836..0x5da83e).
    - prelit (`f & 8`): `RwD3D9SetRenderState(0x8b AMBIENT, a<<24|r<<16|g<<8|b)` (0x5da848..0x5da867), `(0x8d COLORVERTEX,
      1)`, `(0x93 DIFFUSEMATERIALSOURCE, 1)`, `(0x94 AMBIENTMATERIALSOURCE, 0)`; `Ambient.rgb = 0`; `Emissive.k =
      f32(color.k * AmbientSaturated.k * amb)` (0x5da890..0x5da8fe).
    - else: `(0x8b, 0xffffffff)`, `(0x8d, 0)`, `(0x93, 0)`, `(0x94, 0)`; `Ambient.k = f32(color.k * AmbientSaturated.k *
      amb)`; `Emissive.rgb = 0` (0x5da909..0x5da9a7).
  - Else (0x5da9b2): `Diffuse.rgb = sp->diffuse` (raw, NOT scaled by 1/255), `Diffuse.a = 1.0f`; `(0x8b, 0xffffffff)`,
    `(0x93, 0)`; prelit: `(0x8d, 1)`, `(0x94, 1)` (Emissive left as it was); else `(0x8d, 0)`, `(0x94, 0)`, `Emissive.rgb =
    0`. `Ambient`: if the dword of `sp->ambient == 0x3f800000` -> `Ambient.rgb = AmbientSaturated.rgb` (dword copies,
    0x5daa7d); else `Ambient.k = f32(AmbientSaturated.k * sp->ambient)` (0x5daa51..0x5daa75).
  - `RwD3D9SetMaterial(0x8d12d8)` (0x5daaa4). Ambient.a/Emissive.a/Specular.a are never written here.
- gta-reversed: inlined into its CustomPipeRenderCB (CustomCarEnvMapPipeline.cpp:454); bytes win.
- librw: absent (game-side pipeline code; librw has no equivalent).

