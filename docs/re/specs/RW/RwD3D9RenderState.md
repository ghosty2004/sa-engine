# RW D3D9 driver: render-state cache (RE-SPEC-RW writer C)

The D3D9 driver of RW 3.6 in gta_sa.exe 1.0 US keeps several caches in front of `IDirect3DDevice9`
(`_RwD3DDevice` [0xc97c28]). Phase B re-implements them on wgpu: what matters is (a) the RW-level state values
returned by `RwRenderStateGet`, (b) which D3D state each RW state maps to, and (c) WHEN a D3D call is made
(immediately, or deferred until `_rwD3D9RenderStateFlushCache`). All functions `__cdecl`.

## Caches (addresses proven by the instructions cited)

| cache | layout | evidence |
|---|---|---|
| D3D render state, wanted value | `struct {u32 value; u32 queued;} rs[210]` at 0xc991d0 (value) / 0xc991d4 (queued), index = D3DRENDERSTATETYPE | 0x7fc2d8, 0x7fc2e8 |
| D3D render state, last sent | `u32 rsDev[210]` at 0xc98e88 | 0x7fc21e, 0x7fc236 |
| render-state dirty list | `u32 rsList[]` at 0xc98b40, count [0xc9a5ec] | 0x7fc304, 0x7fc30c |
| texture-stage state, wanted | `struct {u32 value; u32 queued;} tss[8*33]` at 0xc980f8 / 0xc980fc, index = stage*33 + type | 0x7fc34b-0x7fc375 |
| texture-stage state, last sent | `u32 tssDev[8*33]` at 0xc99860 | 0x7fc284, 0x7fc296 |
| TSS dirty list | pairs `{stage, type}` at 0xc99c80 / 0xc99c84, count [0xc9a5f0] | 0x7fc384-0x7fc393 |
| sampler state (immediate) | `u32 samp[8*14]` at 0xc98938, index = stage*14 + type | 0x7fc3ca-0x7fc3e3 |
| material | D3DMATERIAL9 (0x11 dwords) last sent at 0xc98af8; staging copy at 0x8e2538 | 0x7fc435, 0x7fc469 |
| per-stage RW texture state | 8 x 0x18 bytes at 0xc9a508: +0 current RwRaster*, +4 addressU, +8 addressV, +0xc filter, +0x10 border color, +0x14 max anisotropy (RW values) | 0x7fde57, 0x7fde92, 0x7fded2, 0x7fdf62, 0x7fdf29 |
| other driver caches | 0x8e2440 last FVF, 0x8e2444 last vertex declaration, 0x8e2448 last VS, 0x8e244c last PS, 0x8e2450 last index buffer; stream sources 4 x 0x10 at 0xc97bd8 ({vb, offset, stride, -}) | 0x7f6b05-0x7f6b31 |

RW-level mirrors (what `_rwD3D9RWGetRenderState` returns): 0xc9a4c0 ZWRITEENABLE, 0xc9a4c4 ZTESTENABLE, 0xc9a4c8
STENCILENABLE, 0xc9a4cc STENCILFAIL, 0xc9a4d0 STENCILZFAIL, 0xc9a4d4 STENCILPASS, 0xc9a4d8 STENCILFUNCTION,
0xc9a4dc STENCILFUNCTIONREF, 0xc9a4e0 STENCILFUNCTIONMASK, 0xc9a4e4 STENCILFUNCTIONWRITEMASK, 0xc9a4e8
VERTEXALPHAENABLE, 0xc9a4ec "current stage-0 texture has alpha", 0xc9a4f0 CULLMODE, 0xc9a4f4 FOGENABLE, 0xc9a4f8
FOGTYPE, 0xc9a4fc FOGDENSITY (f32), 0xc9a500 FOGCOLOR, 0xc9a504 SHADEMODE, 0xc9a5c8 SRCBLEND, 0xc9a5cc DESTBLEND,
0xc9a5d0 ALPHATESTFUNCTION, 0xc9a5d4 "alpha test wanted" (func != ALWAYS), 0xc9a5e8 anisotropy plugin offset in
RwTexture. Caps: D3DCAPS9 copy at 0xc9bf00 (+0x24 RasterCaps [0xc9bf24], +0x3c TextureCaps [0xc9bf3c], +0x40
TextureFilterCaps [0xc9bf40], +0x58/+0x5c MaxTextureWidth/Height).

"Queue RS s = v" below means: `rs[s].value = v; if (!rs[s].queued) { rs[s].queued = 1; rsList[count++] = s; }`
(the value store is unconditional, no compare with the old wanted value unless stated).

Mapping tables (u32): stencil op 0x8849a0 = identity 0..8; compare func 0x8849c4 = identity 0..8; shade 0x8848f0 =
{0,1,2}; cull 0x884990 = {0,1,2,3}; blend 0x88490c = identity 0..11; texture address 0x88493c = identity 0..4; fog
0x8848fc = {0, 3 LINEAR, 1 EXP, 2 EXP2}; filter 0x884950 = 8 pairs (minmag, mip): 0 (0,0), 1 NEAREST (1,0), 2 LINEAR
(2,0), 3 MIPNEAREST (1,1), 4 MIPLINEAR (2,1), 5 LINEARMIPNEAREST (1,2), 6 LINEARMIPLINEAR (2,2), 7 anisotropic (3,2).

## 0x7f6b00 _rwD3D9RenderStateCacheInvalidate  (catalogue: unknown/REA, port level 0)
- `void ()`: [0x8e2440], [0x8e2444], [0x8e2448], [0x8e244c], [0x8e2450] = -1; for the 4 stream entries at 0xc97bd8
  (stride 0x10): vb = -1, offset = 0, stride = 0 (+0xc untouched).

## 0x7fc200 _rwD3D9RenderStateFlushCache  (catalogue: specified/REA, port level 0)
- `void ()`. If [0xc9a5ec] != 0: for i = 0 while i < [0xc9a5ec] (re-read each pass): `s = rsList[i]`;
  `v = rs[s].value`; `rs[s].queued = 0`; if `rsDev[s] != v`: `rsDev[s] = v`, `device->SetRenderState(s, v)` (vtbl
  +0xe4). Then [0xc9a5ec] = 0.
- Then if [0xc9a5f0] != 0: same over the TSS list with `SetTextureStageState(stage, type, v)` (+0x10c); count = 0.
- D3D calls happen in queue order (order of first change since the last flush).

## 0x7fc2d0 RwD3D9SetRenderState  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 state, RwUInt32 value)`: if `rs[state].value != value`: `rs[state].value = value` and queue it if
  not queued (0x7fc2d8-0x7fc30c). No D3D call. (Return value: EAX garbage.)

## 0x7fc320 RwD3D9GetRenderState  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 state, void* out)`: `*(u32*)out = rs[state].value` (the wanted value, not the device value).

## 0x7fc340 RwD3D9SetTextureStageState  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 stage, RwUInt32 type, RwUInt32 value)`: index `stage*33 + type`; if wanted != value: store and append
  `{stage, type}` to the TSS list if not queued. Deferred.

## 0x7fc3c0 RwD3D9SetSamplerState  (catalogue: specified/REA, port level 0)
- `void (RwUInt32 stage, RwUInt32 type, RwUInt32 value)`: if `samp[stage*14 + type] != value`: store and IMMEDIATELY
  `device->SetSamplerState(stage, type, value)` (+0x114).

## 0x7fc430 RwD3D9SetMaterial  (catalogue: specified/REA, port level 0)
- `RwBool (const D3DMATERIAL9* m)`: compare the 0x11 dwords with 0xc98af8 (integer compare); all equal -> return 1.
  Else copy into 0xc98af8, `[0xc9a5f4] = 0` (invalidates the SetSurfaceProperties key), `device->SetMaterial(m)` (+0xc4,
  with the caller's pointer), return `hr >= 0`.

## 0x7fc4d0 RwD3D9SetSurfaceProperties  (catalogue: specified/REA, port level 0)
- `RwBool (const RwSurfaceProperties* sp /*ambient +0, specular +4, diffuse +8*/, const RwRGBA* color, RwUInt32 flags)`.
  Key: `f = flags & 0x48` (0x40 MODULATEMATERIALCOLOR, 8 PRELIT). If `sp->diffuse`, `sp->ambient`, `*(u32*)color`, `f`
  and the three ambient-light floats [0x8e2418], [0x8e241c], [0x8e2420] all equal the cached key (0xc9a5fc, 0xc9a600,
  0xc9a5f8, 0xc9a5f4, 0xc9a5d8/dc/e0; float compares) -> return 1. Else store the new key and build the material
  at 0x8e2538 (D3DMATERIAL9: Diffuse +0, Ambient +0x10, Specular +0x20, Emissive +0x30, Power +0x40):
  - If `!(flags & 0x40)` or the color is opaque white (`*(u32*)color == 0xffffffff`): Diffuse = (d, d, d, 1.0) with
    `d = sp->diffuse`; queue RS 0x8b AMBIENT = 0xffffffff (compare-and-queue), RS 0x93 AMBIENTMATERIALSOURCE = 0
    (compare-and-queue); if `flags & 8`: RS 0x8d COLORVERTEX = 1 and RS 0x94 EMISSIVEMATERIALSOURCE = 1, else both 0
    and Emissive.rgb = 0. Ambient.rgb = ambientLight.rgb if `sp->ambient == 1.0`, else `ambientLight.c * sp->ambient`.
  - Else (modulate with a non-white color), with `k = 1/255` [0x859a3c] = 0.003921569f:
    `s = sp->diffuse * k`; Diffuse = ((float)r * s, (float)g * s, (float)b * s, (float)a * k); `amb = sp->ambient * k`.
    Not prelit: RS AMBIENT = 0xffffffff, COLORVERTEX = 0, AMBIENTMATERIALSOURCE = 0, EMISSIVEMATERIALSOURCE = 0;
    Ambient.c = ((float)color.c * ambientLight.c) * amb; Emissive = 0.
    Prelit: RS AMBIENT = the color as D3DCOLOR (a<<24 | r<<16 | g<<8 | b), COLORVERTEX = 1, AMBIENTMATERIALSOURCE = 1,
    EMISSIVEMATERIALSOURCE = 0; Ambient = 0; Emissive.c = ((float)color.c * ambientLight.c) * amb.
  - Finally copy the 0x11 dwords to 0xc98af8 and `device->SetMaterial(0x8e2538)` (+0xc4); return `hr >= 0`.
- Confidence medium: the branch structure is from the decompile and the float order spot-checked at 0x7fc72c-0x7fc768
  (`fild c; fmul ambient; fmul amb`) and 0x7fc59d-0x7fc60b; port the exact x87 order from 0x7fc4d0-0x7fca9f.
- Open question: the exact role of [0x8e2418..] (`AmbientSaturated`, written by the light code outside this file).

## 0x7fcac0 _rwD3D9RenderStateOpen  (catalogue: specified/REA, port level 1)
- `void ()`; called from D3D9System START. Steps:
  1. `rsDev[0..209] = -1`, then rsDev[0x22 FOGCOLOR] = rsDev[0x8b AMBIENT] = rsDev[0x3a STENCILMASK] =
     rsDev[0x3b STENCILWRITEMASK] = 0; `rs[i] = {rsDev[i], queued 0}` for all 210; `tssDev[0..263] = -1`;
     `tss[*] = {-1, 0}`; `samp[0..111] = -1`; material cache zeroed; RS/TSS list counts 0; surface key zeroed.
  2. RW mirrors and queued defaults (in this order): fog: FOGENABLE mirror 0, FOGTYPE 1, FOGDENSITY 1.0f, FOGCOLOR 0;
     queue RS 0x1c FOGENABLE = 0 (if different); table fog (RasterCaps & 0x100 and & 0x100000): RS 0x23 FOGTABLEMODE = 3,
     RS 0x8c FOGVERTEXMODE = 0, else RS 0x23 = 0, RS 0x8c = 3; RS 0x22 FOGCOLOR = 0; RS 9 SHADEMODE = 2 (mirror 2);
     RS 0x17 ZFUNC = 4; RS 0xe ZWRITEENABLE = 1; RS 7 ZENABLE = 1 (mirrors ZWRITE = ZTEST = 1); stencil: RS 0x34 = 0,
     0x35/0x36/0x37 = 1 (KEEP), 0x38 = 8 (ALWAYS), 0x39 = 0, then `RwD3D9SetRenderState(0x3a, -1)`, `(0x3b, -1)`;
     mirrors func 8, mask/writemask -1, ops 1, enable 0, ref 0.
  3. For stage 0..7: `device->SetTexture(stage, NULL)` (+0x104) directly; stage.raster = NULL; if TextureFilterCaps
     (0xc9bf40) & 0x200 (MINFLINEAR): direct `SetSamplerState(stage, MINFILTER, 2)` if cached != 2, then
     `RwD3D9SetSamplerState(stage, MAGFILTER, 2)`, stage.filter = 2; else `RwD3D9SetSamplerState(stage, 6, 1)`,
     `(stage, 5, 1)`, filter = 1. Then MIPFILTER 0, ADDRESSU 1, ADDRESSV 1 (mirrors 1/1), BORDERCOLOR 0xff000000,
     MAXANISOTROPY 1 (mirrors); for stages 1..7 also `RwD3D9SetTextureStageState(stage, COLOROP, 1 DISABLE)`,
     `(stage, ALPHAOP, 1 DISABLE)`.
  4. `RwD3D9SetRenderState(0x13 SRCBLEND, 5)`, `(0x14 DESTBLEND, 6)`; mirrors 5/6; `(0x19 ALPHAFUNC, 5)` mirror 5,
     alpha-test-wanted 1; `(0x18 ALPHAREF, 0)`, `(0x1b ALPHABLENDENABLE, 0)`, `(0xf ALPHATESTENABLE, 0)`; vertex
     alpha / texture alpha mirrors 0; `(0x89 LIGHTING, 0)`; `(0x16 CULLMODE, 2)` mirror 2; TSS stage 0: COLOROP 3,
     COLORARG2 0, ALPHAOP 3, ALPHAARG2 0; RS 0x91 = 0, 0x92 = 0, 0x93 = 0, 0x1a DITHERENABLE = RasterCaps & 1,
     0x1d SPECULARENABLE = 0, 0x8e LOCALVIEWER = 0, 0x8b AMBIENT = 0xffffffff, 0x8f NORMALIZENORMALS = 0;
     `_rwD3D9RenderStateFlushCache()`.
- Confidence medium (from the decompile; values cross-checked at 0x7fcfec-0x7fd025 for blend/alpha).

## 0x7fd100 _rwD3D9RenderStateReset  (catalogue: specified/REA, port level 1)
- `void ()` (after device Reset): same cache wipe as step 1 of 0x7fcac0, then RE-APPLIES the current RW mirrors
  (instead of defaults): FOGDENSITY, FOGENABLE, fog table/vertex mode from FOGTYPE (and FOGSTART/FOGEND 0x24/0x25
  from the camera when LINEAR), FOGCOLOR, SHADEMODE, ZWRITEENABLE = mirror, ZFUNC = 4 or 8 and ZENABLE from the
  ZTEST/ZWRITE mirrors, stencil states from mirrors (func via 0x8849c4), per-stage SetTexture(NULL) + sampler states
  from each stage's mirrors (filter pairs 0x884950, address 0x88493c, border, anisotropy) + COLOROP/ALPHAOP DISABLE,
  SRCBLEND/DESTBLEND/ALPHAFUNC from mirrors (tables 0x88490c/0x8849c4), ALPHAREF, ALPHABLENDENABLE 0 or 1 from the
  vertex-alpha mirror, ALPHATESTENABLE, LIGHTING 0, CULLMODE (0x884990), the same stage-0 TSS and fixed RS tail as
  0x7fcac0 step 4, flush.
- Confidence low: written from the decompile only; port from 0x7fd100-0x7fd7f0 instruction by instruction.

## 0x7fd810 _rwD3D9RWGetRenderState  (catalogue: specified/REA, port level 0)
- `RwBool (RwRenderState s, void* out)`, jump table 0x7fda18 over `s - 1` (s in 1..30, else return 0):
  1 TEXTURERASTER -> stage0.raster [0xc9a508]; 2 TEXTUREADDRESS -> addressU if U == V (return 1), else writes 0 and
  returns 0; 3 U [0xc9a50c]; 4 V [0xc9a510]; 5 TEXTUREPERSPECTIVE -> return 0 (nothing written); 6 ZTESTENABLE
  [0xc9a4c4]; 7 SHADEMODE [0xc9a504]; 8 ZWRITEENABLE [0xc9a4c0]; 9 TEXTUREFILTER [0xc9a514]; 10 SRCBLEND; 11 DESTBLEND;
  12 VERTEXALPHAENABLE [0xc9a4e8]; 13 BORDERCOLOR [0xc9a518]; 14 FOGENABLE; 15 FOGCOLOR; 16 FOGTYPE; 17 FOGDENSITY
  (f32 copy through x87); 18, 19 -> return 0; 20 CULLMODE; 21..28 stencil mirrors; 29 ALPHATESTFUNCTION [0xc9a5d0];
  30 ALPHATESTFUNCTIONREF -> `rs[0x18].value` [0xc99290] (the D3D cache, not a mirror). All others return 1.

## 0x7fdb80 _rwD3D9RenderStateTextureFilter  (catalogue: unknown/REA, port level 0)
- `RwBool (RwTextureFilterMode f)` (stage 0 only): if stage0.aniso [0xc9a51c] > 1: set 1, `samp[0][MAXANISOTROPY]` =
  1, `device->SetSamplerState(0, 0xa, 1)`. If `f != stage0.filter`: store; `(mm, mip) = table[f]`; direct
  `SetSamplerState(0, MINFILTER 6, mm)`, `(0, MAGFILTER 5, mm)` (caches updated), then MIPFILTER 7 only if the cached
  value differs from mip. Returns 1.

## 0x7fdcd0 _rwD3D9SetTextureInternal  (catalogue: unknown/REA, port level 0)
- Proposed name: `_rwD3D9RWSetRasterStage` (RW state 1 TEXTURERASTER handler and tail of RwD3D9SetTexture).
- `RwBool (RwRaster* r, RwUInt32 stage)`; ext = r + [0xb4e9e0] (D3D9 raster extension: +0 IDirect3DTexture9*,
  +4 palette, +8 u8 alpha).
- Stage 0 only: if `r && ext->alpha`: if texture-alpha mirror [0xc9a4ec] == 0: set 1; if vertex alpha [0xc9a4e8] == 0:
  queue RS 0x1b ALPHABLENDENABLE = 1 and, if `rs[0xf].value != [0xc9a5d4]`, queue RS 0xf ALPHATESTENABLE = [0xc9a5d4].
  Else (no raster or no alpha) and the mirror was 1: clear it; if vertex alpha == 0: queue ALPHABLENDENABLE = 0 and, if
  ALPHATESTENABLE != 0, queue it = 0.
- If `stage.raster != r`: store; r != NULL -> `device->SetTexture(stage, ext->texture)` (+0x104), then if
  `ext->palette`: `device->SetCurrentTexturePalette(palette->index (+0x400))` (+0x124); r == NULL -> SetTexture(stage,
  NULL). Returns 1.

## 0x7fde50 _rwD3D9GetCurrentTexture  (catalogue: specified/REA, port level 0)
- `RwRaster* (RwUInt32 stage)`: returns `[0xc9a508 + stage*0x18]` (the current RASTER, despite the name).

## 0x7fde60 _rwD3D9SetMaxAnisotropy  (catalogue: specified/REA, port level 0)
- `void (RwInt32 off)`: `[0xc9a5e8] = off` (the RpAnisot plugin offset inside RwTexture; set by RpAnisot open).

## 0x7fde70 RwD3D9SetTexture  (catalogue: specified/REA, port level 1)
- `RwBool (RwTexture* t, RwUInt32 stage)`; returns 1.
- t != NULL: `fa = t->filterAddressing` (+0x50).
  1. `u = (fa >> 8) & 0xf`; if != stage.addressU: store, `samp[stage][1] = table[u]`, direct
     `SetSamplerState(stage, ADDRESSU, table[u])` (+0x114).
  2. `v = (fa >> 12) & 0xf`: same for ADDRESSV (2).
  3. `filter = fa & 0xff`. If [0xc9a5e8] > 0: `an = *(u8*)(t + [0xc9a5e8])`; if an != stage.aniso: store, `samp[0][0xa] = an`
     and `SetSamplerState(0, MAXANISOTROPY, an)` - ALWAYS stage 0, whatever `stage` is (original quirk); if an > 1:
     filter = 7.
  4. If filter != stage.filter: store; direct `SetSamplerState(stage, MINFILTER, mm)`, `(stage, MAGFILTER, mm)`,
     `(stage, MIPFILTER, mip)` with caches updated (no compare).
  5. `_rwD3D9SetTextureInternal(t->raster, stage)`.
- t == NULL: if stage == 0 and texture-alpha mirror != 0: clear it; if vertex alpha == 0: queue ALPHABLENDENABLE = 0,
  and ALPHATESTENABLE = 0 if not already 0. Then if stage.raster != NULL: stage.raster = NULL,
  `device->SetTexture(stage, NULL)`.

## 0x7fe1a0 _rwD3D9RenderStateStencilEnable  (catalogue: unknown/REA, port level 0)
- `RwBool (RwBool v)`: if v != [0xc9a4c8]: queue RS 0x34 STENCILENABLE = v; mirror = v. Returns 1.

## 0x7fe1f0 _rwD3D9RenderStateStencilFail  (catalogue: unknown/REA, port level 0)
- As 0x7fe1a0: mirror [0xc9a4cc], RS 0x35 = [0x8849a0][v].

## 0x7fe240 _rwD3D9RenderStateStencilZFail  (catalogue: unknown/REA, port level 0)
- Mirror [0xc9a4d0], RS 0x36 = [0x8849a0][v].

## 0x7fe290 _rwD3D9RenderStateStencilPass  (catalogue: unknown/REA, port level 0)
- Mirror [0xc9a4d4], RS 0x37 = [0x8849a0][v].

## 0x7fe2e0 _rwD3D9RenderStateStencilFunction  (catalogue: unknown/REA, port level 0)
- Mirror [0xc9a4d8], RS 0x38 = [0x8849c4][v].

## 0x7fe330 _rwD3D9RenderStateStencilRef  (catalogue: unknown/REA, port level 0)
- Mirror [0xc9a4dc], RS 0x39 = v.

## 0x7fe380 _rwD3D9RenderStateStencilMask  (catalogue: unknown/REA, port level 0)
- Mirror [0xc9a4e0], RS 0x3a = v.

## 0x7fe3d0 _rwD3D9RenderStateStencilWriteMask  (catalogue: unknown/REA, port level 0)
- Mirror [0xc9a4e4], RS 0x3b = v.

## 0x7fe420 _rwD3D9RWSetRenderState  (catalogue: specified/REA, port level 1)
- `RwBool (RwRenderState s, void* param)` (param is a value except for FOGDENSITY, where it points to an f32).
  Jump table 0x7fe9f8 over `s - 1`; s outside 1..30 -> return 0. Cases:
  - 1 TEXTURERASTER -> `_rwD3D9SetTextureInternal(param, 0)` 0x7fdcd0.
  - 2 TEXTUREADDRESS: if v != stage0.addressU: store, samp cache, direct `SetSamplerState(0, 1, table[v])`; then the
    same for addressV (`SetSamplerState(0, 2, ...)`). 3 ADDRESSU / 4 ADDRESSV: one of the two. Return 1.
  - 5 TEXTUREPERSPECTIVE: return `param != 0` (nothing set).
  - 6 ZTESTENABLE -> 0x7feb70; 8 ZWRITEENABLE -> 0x7fea70 (see the name note there).
  - 7 SHADEMODE -> 0x7fec70; 9 TEXTUREFILTER -> 0x7fdb80; 12 VERTEXALPHAENABLE -> 0x7fe0a0; 13 BORDERCOLOR -> 0x7fecc0;
    20 CULLMODE -> 0x7fed00; 21..28 -> 0x7fe1a0, 0x7fe1f0, 0x7fe240, 0x7fe290, 0x7fe2e0, 0x7fe330, 0x7fe380, 0x7fe3d0.
  - 10 SRCBLEND / 11 DESTBLEND: if v != mirror (0xc9a5c8 / 0xc9a5cc): queue RS 0x13 / 0x14 = [0x88490c][v]; mirror = v.
  - 14 FOGENABLE: v != 0: if mirror off and RasterCaps & 0x180: queue RS 0x1c = 1, mirror 1 (no caps -> nothing);
    v == 0: if mirror on: queue RS 0x1c = 0, mirror 0. Return 1.
  - 15 FOGCOLOR: if changed queue RS 0x22 = v, mirror [0xc9a500].
  - 16 FOGTYPE: if v != mirror [0xc9a4f8]: with table fog caps (RasterCaps & 0x100 and & 0x100000): queue RS 0x23 =
    [0x8848fc][v] and set RS 0x8c = 0 (queued only if it was non-zero); without: RS 0x23 = 0 (if non-zero), RS 0x8c =
    [0x8848fc][v] (queued if not queued); mirror = v. Then (also when unchanged) if v == 1 (LINEAR): with `c =
    [0xc9bcc0]` (driver global, its +0x84/+0x88 match RwCamera farPlane/fogPlane): RS 0x24 FOGSTART = c+0x88 and
    RS 0x25 FOGEND = c+0x84 (dword compare-and-queue each). Return 1.
  - 17 FOGDENSITY: `d = *(f32*)param`; unless `d == mirror` (x87 compare; NaN counts as different): queue RS 0x26 = d's
    bits, mirror = d. Return 1.
  - 18 FOGTABLE, 19 ALPHAPRIMITIVEBUFFER: return 0.
  - 29 ALPHATESTFUNCTION: if v != [0xc9a5d0]: mirror = v; wanted = (v != 8 ALWAYS) -> [0xc9a5d4]; if
    `rs[0x1b].value` (ALPHABLENDENABLE) != 0: `RwD3D9SetRenderState(0xf, wanted)`; queue RS 0x19 ALPHAFUNC =
    [0x8849c4][v]. Return 1.
  - 30 ALPHATESTFUNCTIONREF: `RwD3D9SetRenderState(0x18 ALPHAREF, v)`; return 1.
  The helper cases return the helper's result.

## 0x7fea70 _rwD3D9RenderStateZTestEnable  (catalogue: unknown/REA, port level 0)
- NAME: this function implements rwRENDERSTATEZWRITEENABLE (case 8 of 0x7fe420 calls it at 0x7fe7e7; it owns mirror
  [0xc9a4c0], which 0x7fd810 returns for state 8, and it drives D3DRS_ZWRITEENABLE). Proposed name
  `_rwD3D9RenderStateZWriteEnable` (rw-names swapped the two).
- `RwBool (RwBool v)`. v != 0: if mirror already on -> return 1; else if the ztest mirror [0xc9a4c4] == 0: queue RS 7
  ZENABLE = 1; queue RS 0xe ZWRITEENABLE = 1; mirror = 1. v == 0: if mirror off -> return 1; else if ztest mirror == 0:
  queue ZENABLE = 0; queue ZWRITEENABLE = 0; mirror = 0. Returns 1.

## 0x7feb70 _rwD3D9RenderStateZWriteEnable  (catalogue: unknown/REA, port level 0)
- NAME: implements rwRENDERSTATEZTESTENABLE (case 6 at 0x7fe7f7; mirror [0xc9a4c4], returned for state 6). Proposed name
  `_rwD3D9RenderStateZTestEnable`.
- `RwBool (RwBool v)`. v != 0: if mirror on -> return 1; else if the zwrite mirror [0xc9a4c0] == 0: queue ZENABLE = 1;
  queue RS 0x17 ZFUNC = 4 (LESSEQUAL); mirror = 1. v == 0: if mirror off -> return 1; else if zwrite mirror == 0: queue
  ZENABLE = 0; queue ZFUNC = 8 (ALWAYS); mirror = 0. Returns 1.

## 0x7fec70 _rwD3D9RenderStateShadeMode  (catalogue: unknown/REA, port level 0)
- `RwBool (v)`: if v != [0xc9a504]: queue RS 9 SHADEMODE = [0x8848f0][v]; mirror = v. Returns 1.

## 0x7fecc0 _rwD3D9RenderStateTextureBorderColor  (catalogue: specified/REA, port level 0)
- `RwBool (RwUInt32 c)`: if c != stage0.border [0xc9a518]: store, `samp[0][4] = c`, direct
  `SetSamplerState(0, BORDERCOLOR 4, c)`. Returns 1.

## 0x7fed00 _rwD3D9RenderStateCullMode  (catalogue: unknown/REA, port level 0)
- `RwBool (v)`: if v != [0xc9a4f0]: queue RS 0x16 CULLMODE = [0x884990][v]; mirror = v. Returns 1.

## 0x7fed70 _rwD3D9ClampRasterSize  (catalogue: unknown/REA, port level 0)
- `void (RwInt32* w, RwInt32* h, RwUInt32 flags)`.
  1. Clamp `*w <= MaxTextureWidth [0xc9bf58]`, `*h <= MaxTextureHeight [0xc9bf5c]` (signed compares).
  2. If `(flags & 7) != 0` and `(flags & 0x8000)` (MIPMAP): if TextureCaps [0xc9bf3c] & 2 (POW2): round both down to a
     power of two. Else (not mip or type 0): if TextureCaps & 2 and NOT & 0x100 (NONPOW2CONDITIONAL): round both down.
     Rounding: `n = -1; x2 = x; while (x2 != 0) { n++; x2 >>= 1 (arithmetic) }; x = 1 << (n & 31)`; for x == 0 this
     gives `1 << 31` (0x80000000) - original quirk.
  3. If TextureCaps & 0x20 (SQUAREONLY): both = min(w, h).
- Name note: rw-names calls it `_rwD3D9ClampRasterSize` (fits).
- librw: absent (librw has no caps clamping).
