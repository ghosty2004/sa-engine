# CCustomCarEnvMapPipeline (RENDER) - full specs (RE-SPEC-FINAL)

Both functions are unnamed helpers of CCustomCarEnvMapPipeline::CustomPipeRenderCB 0x5D9900 (their only caller). The UNK classification
proposal filed them under RW (game code in the RW render callback); they are game code and are specified here. gta-reversed inlines both
into its reversed CustomPipeRenderCB (CustomCarEnvMapPipeline.cpp:352 "Inline begin @ 0x5D8590" and :454 "Inlined 0x5DA790"); its inline
versions are simplified and differ from the bytes as listed.

## 0x5D8590 CCustomCarEnvMapPipeline::CustomPipeRenderCB_EnvMapMatrix (proposed; catalogue: unknown/REA, port level 4)
- Convention (non-standard, register args): ECX = `RpAtomic*`, ESI = material env-map plugin data (`EnvMapPlGetData(mat)`: +2 i8 trans/scale
  X, +3 i8 trans/scale Y, +6 u16 last render frame), EDI = atomic env-map data (`EnvMapAtmPlGetData`: +0 float OffsetU, +4 PrevPosX,
  +8 PrevPosY), `[esp+4]` = `float out[2]` (u, v). Plain `ret` (caller pops). Returns AL = 1. Call site 0x5D9D13.
- Callees: RwFrameGetLTM 0x7F0990, CVector::Normalise 0x59C910 (x2), `_ftol` 0x821B40 (x4). Globals: RwEngineInstance 0xC97B24
  (u16 renderFrame at +8).
- Behaviour:
  1. `frame = atomic->clump (+0x3C) ? clump->frame (+4) : atomic->frame (+4)`; `L = RwFrameGetLTM(frame)` (L+0x10 up, +0x20 at, +0x30 pos).
     Copies: `P = L->pos` (x, y, z), `off = atm->OffsetU`.
  2. `sx = ((float)(i8)mat[+2] * 0.125f [0x858C48]) * 50.0f [0x858B40]`, `sy` likewise from `mat[+3]` (stored f32).
  3. If `mat->renderFrame (u16 +6) != RwEngineInstance->renderFrame (u16 +8)`:
     a. `mat->renderFrame = renderFrame`. `ry = 1.0f [0x858624] / sy` (stored), `ny = _ftol(P.y * ry)`; `rx = 1.0f / sx` (stored),
        `nx = _ftol(P.x * rx)` (truncation toward zero, not floor).
     b. `F(n, s, r, p) = |((float)n * s - p) * r|`, and `if (n even) F = 1.0f - F` (`test al,1` on the low byte of n).
        `cur = F(ny, sy, ry, P.y) + F(nx, sx, rx, P.x)` (x term computed first, sum stored f32).
     c. Same with the previous position: `pny = _ftol(PrevY * ry)`, `pnx = _ftol(PrevX * rx)`; `prev = F(pny, sy, ry, PrevY) + F(pnx, sx, rx, PrevX)`
        (x term first; stored f32). PrevX/PrevY are read once at 0x5D8695.
     d. `D = (P.x - PrevX, P.y - PrevY, 0)` (stored f32). If `D.x*D.x + D.y*D.y > 0` (strict): `U = L->up` (copy); `Normalise(D)`, `Normalise(U)`;
        `dot = (U.z*D.z + D.x*U.x) + U.y*D.y`; if `dot < 0` (strict): `off = off - |prev - cur|`; `if (off < 0) off += 1.0f`; -> e.
        Otherwise (no movement, `dot >= 0` or NaN): `off = |prev - cur| + off`; `if (!(off < 1.0f)) off -= 1.0f` (NaN skips: `test ah,1`).
        (One wrap step only, each value stored f32.)
     e. `atm->OffsetU = off`; `atm->PrevPosX = P.x`; `atm->PrevPosY = P.y`.
  4. `v = L->at.y + L->at.x`; `v = (v < 0.1f [0x858B1C]) ? ((v > 0.0f) ? v : 0.0f) : 0.1f` (NaN -> 0.1f); `if (L->at.z < 0.0f) v = 1.0f - v`.
  5. `out[0] = -off` (the local copy: the stored atm value when step 3 ran, the old value otherwise); `out[1] = v`; return 1.
- Edge cases: `sx == 0` or `sy == 0` (plugin byte 0) gives `r = inf` and `_ftol(inf/NaN)` = 0x80000000 (original behaviour, keep).
- gta-reversed (inline, reading aid only): uses `p / s` (division) where the original multiplies by the stored reciprocal `p * (1/s)`, and
  `(p - n*s)/s` instead of `(n*s - p) * (1/s)` (same magnitude, different rounding); replaces step 3d by `wrap(off + copysign(prev - cur, dir), 0, 1)`
  with an unnormalised dot - the original subtracts only when the normalised dot is strictly negative, adds otherwise (also for zero movement),
  and wraps by a single +-1. Port the bytes. Open questions: none.

## 0x5DA790 CCustomCarEnvMapPipeline::CustomPipeRenderCB_SetupMaterial (proposed; catalogue: unknown/REA, port level 1)
- Convention: `__cdecl int (RwSurfaceProperties* sp [esp+4], RwRGBA* colour [esp+8], u32 rxGeoFlags [esp+0xC], float specular [esp+0x10],
  float power [esp+0x14])`, plain `ret`, EAX = 1. Call site 0x5D9ED2 (sp = &material->surfaceProperties).
- Callees: RwD3D9SetRenderState 0x7FC2D0 (cdecl (state, value)), RwD3D9SetMaterial 0x7FC430. Globals: static D3DMATERIAL9 `M` at 0x8D12D8
  (Diffuse r,g,b,a 0x8D12D8..0x8D12E4, Ambient 0x8D12E8..0x8D12F4, Specular 0x8D12F8..0x8D1304, Emissive 0x8D1308..0x8D1314, Power 0x8D1318);
  AmbientSaturated r,g,b 0x8E2418/0x8E241C/0x8E2420 (floats). Constant 1/255 = 0.003921569f [0x859A3C]. sp: +0 ambient, +4 specular, +8 diffuse.
  D3DRS ids: 0x8B AMBIENT, 0x8D COLORVERTEX, 0x93 AMBIENTMATERIALSOURCE, 0x94 EMISSIVEMATERIALSOURCE.
- Behaviour: `M.Specular.r = M.Specular.g = M.Specular.b = specular` (dword copies); `M.Power = power`; `f = rxGeoFlags & 0x48`.
  A. If `(f & 0x40)` (MODULATE) and `*(u32*)colour != 0xFFFFFFFF` (0x5DA7C4):
     `k = sp->diffuse * (1/255)`; `M.Diffuse.r = (float)colour.r * k`, `.g = (float)g * k`, `.b = (float)b * k`; `M.Diffuse.a = (float)a * (1/255)`;
     `s = sp->ambient * (1/255)` (stored f32).
     - If `f & 8` (PRELIT): `SetRenderState(0x8B, a<<24 | r<<16 | g<<8 | b)`; `(0x8D, 1)`; `(0x93, 1)`; `(0x94, 0)`; `M.Ambient.rgb = 0`;
       `M.Emissive.r = ((float)r * AmbientSaturated.r) * s`, `.g`, `.b` likewise.
     - Else: `(0x8B, 0xFFFFFFFF)`, `(0x8D, 0)`, `(0x93, 0)`, `(0x94, 0)`; `M.Ambient.r = ((float)r * AmbientSaturated.r) * s`, `.g`, `.b`
       (the Ambient r/g writes come before the Emissive zeroing, b after; all end values as stated); `M.Emissive.rgb = 0`.
  B. Else (0x5DA9B2): `M.Diffuse.r = M.Diffuse.g = M.Diffuse.b = sp->diffuse`, `M.Diffuse.a = 1.0f`; `(0x8B, 0xFFFFFFFF)`, `(0x93, 0)`;
     if PRELIT: `(0x8D, 1)`, `(0x94, 1)` (Emissive left unchanged from the previous call); else `(0x8D, 0)`, `(0x94, 0)`, `M.Emissive.rgb = 0`.
     Ambient: if `sp->ambient` has the bit pattern 0x3F800000 (integer compare, exactly 1.0f): `M.Ambient.rgb = AmbientSaturated.rgb` (copy);
     else `M.Ambient.c = AmbientSaturated.c * sp->ambient`.
  Then `RwD3D9SetMaterial(&M)`; return 1. Ambient.a, Emissive.a and Specular.a are never written.
- gta-reversed (inline, reading aid): its version starts the unmodulated branch with "`!(flags & MODULATE) || colour == white`" as here, but
  check every render-state value and the stale-Emissive case against this list. Open questions: none.
