# RW D3D9 driver: vertex-shader constants, VS matrix helpers, world-sector all-in-one node - pass-3 specs, writer A (RE-SPEC-RW-3)

RW 3.6 D3D9 code. gta-reversed only has thunks. All `__cdecl`. Extends docs/re/specs/RW/RwD3D9VertexShader.md (pass
1/2, read-only), whose descriptor bit table and globals (0xc94c70 world matrix, 0xc94c74 cached inverse, 0xc94c7c
cached orthonormalised matrix, 0x8d7390 world copy, 0x8d73d0 inverse, 0x8753d0 identity) apply.

## Layouts used (offsets proven by the instruction addresses given)

- `IDirect3DDevice9` vtbl+0x178 SetVertexShaderConstantF(start, data, count) (0x76189b). Dispatch descriptor bytes: +0 material
  colour register, +1 fog, +2 effect (UV anim / env / bump), +3 morph (0x761883, 0x76193c, 0x7619f1, 0x761984).
- `RwMatrix` flags (+0xc): `& 0x20000` = identity (0x764970), `& 3 == 3` = orthonormal (0x764a9c).
- `[0xc94c74]` cached inverse world (0x764e73), `[0xc94c7c]` cached orthonormalised world (0x764abb), `0x8d7450`
  orthonormalised storage (w column at 0x8d745c/0x8d746c/0x8d747c/0x8d748c, 0x764ad8..0x764af6), `0x8d73d0` inverse (w column
  0x8d73dc/0x8d73ec/0x8d73fc/0x8d740c), view transform `_RwD3D9D3D9ViewTransform` 0xc9bc80, projection
  `_RwD3D9D3D9ProjTransform` 0x8e2458. D3DX (third-party, catalogue X-d3dx9) helpers: 0x7675ee (matrix transpose: out, in),
  0x7676c7 (multiply then transpose: out, a, b), 0x766a32 (transform coord: out, in, m), 0x766ab7 (transform normal).
- Vertex-shader cache (see pass 1): entries at 0xc926f8 (0x14 each; +8 shader at 0xc92700), count `[0xc94bf8]`, last hit
  `[0xc94bfc]`.
- `RwCamera` +0x84 farPlane, +0x88 fogPlane (0x7618f5/0x7618fb); `RwGlobals` +0x0 current camera (0x7618f3), +0x4 current world
  (0x75e575), +0xa u16 lightFrame (0x75e599), +0x24 device render-state getter (0x760fc3); `RpWorld` +0x8 flags (0x75e578).
- `RpWorldSector`: +0x34 repEntry (0x75e42b), +0x40 light tie list (0x75e596), +0x78 mesh header (0x75e413; +4 u16 numMeshes,
  +6 u16 serialNum), +0x82 u16 numVertices (0x75e405). Resources module "used" list head `*(G + [0xc9bc58] + 0x24)` (0x75e4a0).
- Constants: `[0x859a3c] = 0.003921569f`, `[0x875294] = 1.442695f` (log2 e), `[0x858624] = 1.0f`, `[0x858b50] = 0.0f`.

## 0x75e300 _rpWorldPipeClose  (catalogue: specified/REA, port level 3)
- `_rpDestroyD3D9WorldSectorPipe()` (0x74eb40), `0x74ec00()`, then tail-jump to 0x74eac0 (0x75e300..0x75e30a).
- librw: absent.

## 0x75e3f0 _rwD3D9WorldSectorAllInOneNode  (catalogue: specified/REA, port level 1)
- `RwBool (RxPipelineNode* self, const RxPipelineNodeParam* p)`: `s = p->data`, `pd = self->privateData`. `s->numVertices == 0`
  or `mesh->numMeshes == 0` -> return 1 (0x75e405/0x75e41a). `wf = G->curWorld->flags`.
- If not NATIVE (`wf & 0x1000000`, 0x75e437): `res = s->repEntry`; if `res` and `res->serialNumber (+0x18) != mesh->serialNum`
  -> `RwResourcesFreeResEntry(res)` and re-instance; if they match and `res->link.next` -> move `res` to the head of the
  resources "used" list (inlined RwResourcesUseResEntry, 0x75e493..0x75e4e9). Re-instance: `res = _rpD3D9InstanceMeshes(s, s,
  0xff, &s->repEntry, mesh, pd->instanceCB (+0), (wf >> 25) & 1)` (0x756960, 0x75e47a); NULL -> return 0.
- If `wf & 0x2000000` -> return 1 (no render, 0x75e4eb). Else `pd->lighting (+8)(s)` if set; `RwD3D9SetTransformWorld(NULL)`;
  `RwD3D9SetRenderState(0x8f, 0)`; `pd->render (+0xc)(res, s, 0xff, wf)` if set (0x75e4f3..0x75e52d). Return 1.
- librw: d3d/d3d9render.cpp world-sector path differs (librw has no world-sector pipeline).

## 0x75e570 _rxD3D9WorldSectorDefaultLightingCallback  (catalogue: specified/REA, port level 0)
- `void (RpWorldSector* s)`: if `G->curWorld->flags & 0x20` (LIGHT): `any = _rwD3D9LightsGlobalEnable(2)`; `G->lightFrame++`
  (u16, 0x75e599); for each light tie in `s+0x40` (tie +8 = light): if `light && (light->flags & 2)` -> `any |=
  _rwD3D9LightLocalEnable(light)` (0x75e5a3..0x75e5b9); `_rwD3D9LightsEnable(any, -1)` (0x75e5c4). Else
  `_rwD3D9LightsEnable(0, -1)` (0x75e5d4).
- librw: absent.

## 0x75ee60 _rpD3D9VertexShaderCachePurge  (catalogue: specified/REA, port level 0)
- For `i < [0xc94bf8]` (signed): `RwD3D9DeleteVertexShader(entry[i].shader)` if non-NULL, NULL it (0x75ee72..0x75ee92). Then
  `[0xc94bf8] <- 0`, `[0xc94bfc] <- 0xc926f8`, and entry 0's five dwords 0xc926f8..0xc92708 set to -1 (0x75ee95..0x75eec1).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x760df0 _rxD3D9VertexShaderDefaultBeginCallBack  (catalogue: specified/REA, port level 0)
- `void (void* object, u8 type, u32* desc)`: `*desc = 0` (0x760e01).
- type 1 (atomic): `g = a->geometry`, `gf = g->flags`. Morphing bit (byte2 bit0) if `g->numMorphTargets != 1 &&
  interp.startMorphTarget (+0x50 u16) != endMorphTarget (+0x52) && (a+0x5c * a+0x58) != 0.0` (0x760e1c..0x760e3d; NaN sets
  it). byte1 high nibble = `g->numTexCoordSets` (0x760e41..0x760e54). PRELIT -> byte2 |= 0x10. NORMALS -> byte2 |= 0x20 and
  normalize (byte2 0x40) if `RpD3D9GeometryGetUsageFlags(g) & 0xfff` (0x760e67..0x760e7f), else if the atomic LTM is not
  `& 0x20001` and any axis length squared (f32, signed-int compare) is `> 1.1f` (0x3f8ccccd) or `< 0.9f` (0x3f666666)
  (0x760eaa..0x760f4e, same test as RpSkin 0x7c85b0). MODULATE (gf & 0x40) -> byte2 |= 0x80.
- other types (world sector): world flags `& 0x80` -> byte1 = 0x2x (2 tex coords), else `& 4` -> 0x1x; `& 8` -> byte2 |= 0x10;
  `& 0x10` -> 0x20; `& 0x40` -> 0x80 (0x760f59..0x760f9b).
- Fog: `RwD3D9GetRenderState(0x1c FOGENABLE, &on)`; if on: `(*(G+0x24))(0x10 /*rwRENDERSTATEFOGTYPE*/, &ft)` (0x760fc3); ft 1 ->
  byte3 = (b3 & 0xfd) | 1; 2 -> (b3 & 0xfe) | 2; 3 -> b3 | 3 (0x760fcd..0x760ff7).
- librw: absent.

## 0x761720 _rpD3D9VertexShaderUpdateLightsColors  (catalogue: specified/REA, port level 0)
- `void (f32* c, const u8* desc, f32 ambientScale, f32 lightScale)`: `c[0..2] *= ambientScale` (0x761725..0x761749). Then for each
  directional light (`desc[0] & 0xf`): the colour register (second of its two registers) rgb `*= lightScale` (0x761753..0x761778).
  Point lights (`desc[0] >> 4`): before every 4th light (`i & 3 == 0`) skip one register (attenuation), then scale the colour
  register (second of two) (0x761788..0x7617b7). Spot lights (`desc[1] & 0xf`): same skip rule, colour is the 4th of four
  registers (0x7617c6..0x7617f5). `SetVertexShaderConstantF(4, c, (end - c) / 16)` (0x761809). Products are x87, stored f32.
- librw: absent.

## 0x761820 _rpD3D9VertexShaderUpdateMaterialColor  (catalogue: specified/REA, port level 0)
- `void (const RwRGBA* col, const u8* disp)`: `v = {r, g, b, a} * (1/255)` (each `fild` u8 then `fmul [0x859a3c]`, f32;
  0x76182c..0x761895); `SetVertexShaderConstantF(disp[0], v, 1)` (0x76189b).
- librw: absent.

## 0x7618b0 _rpD3D9VertexShaderUpdateFogData  (catalogue: specified/REA, port level 1)
- `void (const u8* desc, const u8* disp)`: `mode = desc[3] & 3`. 1 (linear): `cam = G->curCamera`; `r = 1.0 / (far - fog)`
  (`fdivr`), `c = {r, -(r*fog), 0, 1.0}` (0x7618ed..0x76191f). 2/3 (exp/exp2): `RwD3D9GetRenderState(0x26 FOGDENSITY, &d)`; `c =
  {f32(d * 1.442695), 0, 0, 1.0}` (0x7618c6..0x76191f). Mode 0: `c` stays uninitialised (stack). Always
  `SetVertexShaderConstantF(disp[1], c, 1)` (0x761941).
- librw: absent.

## 0x761950 _rpD3D9VertexShaderUpdateMorphingCoef  (catalogue: specified/REA, port level 0)
- `void (f32 t, const u8* disp)`: `c = {t, f32(1.0 - t), 1.0, 0}` (0x761953..0x761987); `SetVertexShaderConstantF(disp[3], c, 1)`.
- librw: absent.

## 0x7619a0 _rpD3DVertexShaderSetUVAnimMatrix  (catalogue: specified/REA, port level 0)
- `void (const RwMatrix* m, const u8* disp)`: `c = {right.x, up.x, at.x, pos.x, right.y, up.y, at.y, pos.y}` (0x7619ae..0x7619ed);
  `SetVertexShaderConstantF(disp[2], c, 2)` (0x7619f8).
- librw: absent.

## 0x761b70 _rpD3D9VertexShaderSetBumpMatrix  (catalogue: specified/REA, port level 0)
- `void (RwFrame* lightFrame, f32 scale, const u8* disp)`: `f = lightFrame ? lightFrame : G->curCamera->frame` (0x761b80..0x761b8b);
  `inv = RwMatrixInvert(LTM(f))`; `t = _rwD3D9VSGetWorldMultiplyTransposeMatrix(&inv)` (0x761ba9); `c = {t0*s, t1*s, t2*s, 0,
  t4*s, t5*s, t6*s, 0}` (0x761bae..0x761c15); `SetVertexShaderConstantF(disp[2], c, 2)` (0x761c1b).
- librw: absent.

## 0x764730 _rwD3D9VSGetWorldViewTransposedMatrix  (catalogue: specified/REA, port level 0)
- `(D3DMATRIX* out)`: world set -> `0x7676c7(out, 0x8d7390, view)` (transpose(world*view), 0x76474a); else `0x7675ee(out, view)`.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x764760 _rwD3D9VSGetWorldViewMatrix  (catalogue: specified/REA, port level 0)
- world set -> `RwMatrixMultiply(out, world, view)` and `out` w column `(0, 0, 0, 1.0)` (0x764778..0x764789); else copy the view
  matrix (16 dwords).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x764890 _rwD3D9MatrixInvertOrthonormal  (catalogue: specified/REA, port level 0)
- `(RwMatrix* out, const RwMatrix* m)`: 3x3 transpose (`out[0]=m.right.x, out[0x10]=m.right.y, out[0x20]=m.right.z`, etc.) and
  `out.pos.x = -((m.pos.y*m.right.y + m.pos.x*m.right.x) + m.pos.z*m.right.z)` (0x764898..0x7648af), `out.pos.y = -((m.pos.x*m.up.x
  + m.up.y*m.pos.y) + m.pos.z*m.up.z)`, `out.pos.z = -((m.pos.x*m.at.x + m.at.y*m.pos.y) + m.pos.z*m.at.z)`; pad dwords untouched.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x764920 _rwD3D9VSGetWorldMultiplyMatrix  (catalogue: specified/REA, port level 0)
- `(out, m)`: world ? `RwMatrixMultiply(out, world, m)` : copy m (0x764920..0x76494c).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x764960 _rwD3D9VSGetWorldMultiplyTransposeMatrix  (catalogue: specified/REA, port level 0)
- `(out, m)`: world and not identity -> `tmp = world*m`, `transpose(out, tmp)`; else `transpose(out, m)`; then `out` row 3 =
  `(0, 0, 0, 1.0)` (0x7649a7..0x7649bc).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x7649d0 _rwD3D9VSGetWorldViewMultiplyTransposeMatrix  (catalogue: specified/REA, port level 0)
- `(out, m)`: world non-identity -> `t = (world*view)*m`; else `t = view*m`; `t` w column `(0,0,0,1)`; `transpose(out, t)`.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x764a70 _rwD3D9VSGetWorldNormalizedMultiplyTransposeMatrix  (catalogue: specified/REA, port level 0)
- world non-identity: orthonormal -> `t = world*m`; else `n = [0xc94c7c]` or build it (`RwMatrixOrthoNormalize(0x8d7450, world)`, w
  column 0/1, cache), `t = n*m`; `transpose(out, t)`; identity/no world -> `transpose(out, m)`; out row 3 `(0,0,0,1)`.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x764b50 _rwD3D9VSGetWorldNormalizedViewMultiplyTransposeMatrix  (catalogue: specified/REA, port level 0)
- As 0x764a70 with the view inserted: `t = ((orthonormal ? world : n) * view) * m`, no world -> `view*m`; w column (0,0,0,1);
  transpose.
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x764c60 _rwD3D9VSGetWorldNormalizedTransposeMatrix  (catalogue: specified/REA, port level 0)
- world non-identity: orthonormal -> `transpose(out, 0x8d7390)`; else `transpose(out, n)` (n built/cached as above). Else copy
  identity 0x8753d0 and set its w column (0x764cf5..0x764d11).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x764d20 _rwD3D9VSGetProjectionTransposedMatrix  (catalogue: specified/REA, port level 0)
- `transpose(out, 0x8e2458)` (0x764d2a).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x764e70 _rwD3D9VSGetPointInLocalSpace  (catalogue: specified/REA, port level 0)
- `(const RwV3d* in, RwV3d* out)`: cached inverse present: identity (0x8753d0) -> copy; else `0x766a32(out, in, inv)`. Not cached:
  no world or identity -> cache 0x8753d0 and copy; orthonormal -> `_rwD3D9MatrixInvertOrthonormal(0x8d73d0, world)`; else
  `RwMatrixInvert(0x8d73d0, world)` and set its w column; cache 0x8d73d0; transform (0x764e81..0x764f17).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x764f60 _rwD3D9VSGetRadiusInLocalSpace  (catalogue: specified/REA, port level 0)
- `(f32 r, f32* out)`: no world or orthonormal -> `*out = r` (0x76501f). Else ensure the inverse cache (RwMatrixInvert + w
  column, no identity test); `v = (0.57735026, 0.57735026, 0.57735026)` (0x3f13cd3a); `0x766ab7(&v, &v, inv)` (normal
  transform); `l2 = f32(v.y*v.y + v.x*v.x + v.z*v.z)`; `*out = _rwInvSqrt(l2) * r` (0x764fee..0x765019).
- librw: no direct equivalent identified in librw (not compared line by line).

## 0x75e5e0 _rxD3D9WorldSectorDefaultInstanceCallback  (catalogue: specified/REA, port level 1)
- `RwBool (RpWorldSector* s, RxD3D9ResEntryHeader* hdr, RwBool reinstance)` (s 0x75e5f1, hdr 0x75e602; the third argument is unused).
  `wf = G->curWorld->flags`; `usage = RpD3D9WorldSectorGetUsageFlags(s)` (0x75e5fd). `hdr->totalNumVertex = s->numVertices (u16
  +0x82)` (0x75e61b). Stream 0: `stride = 0xc`, `geometryFlags = 2` (0x75e64b/0x75e64e).
- Vertex declaration (local array of `D3DVERTEXELEMENT9`, 0x75e62b..0x75e7b5), offsets accumulate:
  - POSITION FLOAT3 at 0.
  - `wf & 0x10` (NORMALS): NORMAL, type DEC3N (0xe, +4 bytes) if `[0xc9bfec] & 0x80` (DeclTypes DEC3N) else FLOAT3 (+0xc); stride
    0x10 / 0x18; geometryFlags = 6 (0x75e65a..0x75e69b).
  - `wf & 8` (PRELIT): COLOR (usage 0xa) D3DCOLOR, +4, flags |= 8 (0x75e6ac..0x75e6d8).
  - `wf & 0x84` (TEXTURED or TEXTURED2): TEXCOORD0 FLOAT2, +8, flags |= 0x10; `wf & 0x80`: TEXCOORD1 FLOAT2, +8, flags |= 0x20
    (0x75e6e9..0x75e74b).
  - `usage & 0x10000`: TANGENT (usage 6), DEC3N (+4) or FLOAT3 (+0xc) by the same caps bit (0x75e75c..0x75e799).
  - terminator `{0xff, 0, 0x11, 0, 0, 0}` (0x75e79a..0x75e7b5).
  `RwD3D9CreateVertexDeclaration(elems, &hdr->vertexDeclaration)` (0x75e7c3, result ignored).
- `stream0.managed = 1`; `RwD3D9CreateVertexBuffer(stride, stride*totalNumVertex, &stream0.vb, &stream0.offset)` (0x75e7dc); failure
  -> return 0. `useOffsets = 0`; each instance `baseIndex = offset / stride + minVert` (unsigned, 0x75e804..0x75e81f).
  `vb->Lock(offset, bytes, &p, 0)` (vtbl+0x2c, 0x75e831).
- Fill (running byte offset `o` into the vertex): `o = InstV3d(posType, p, s->vertices (+8), n, stride)` (0x75e857); normals:
  `o += _rpD3D9VertexDeclarationInstV3dComp(type, p + o, s->normals (+0xc, compressed), n, stride)` (0x75e88e); prelit: for each
  instance `inst.vertexAlpha = InstColor(p + o + minVert*stride, s->preLitLum (+0x30) + minVert*4, inst.numVertices, stride)`
  (0x75e8c6..0x75e8f8), `o += 4`; else every `inst.vertexAlpha = 0` (0x75e90f..0x75e91f); texcoords: `InstV2d(type, p + o,
  s->texCoords0 (+0x10), n, stride)`, second set from `+0x14` (0x75e947/0x75e980); tangents: `InstTangent(type, p + o, s->vertices,
  textured2 ? texCoords1 : texCoords0, hdr, stride)` (0x75e9c7). `Unlock` (vtbl+0x30); return 1.
- librw: absent (librw has no world sectors).

## 0x761a10 _rpD3D9VertexShaderSetEnvMatrix  (catalogue: specified/REA, port level 0)
- `void (RwFrame* envFrame, const u8* desc, const u8* disp)`: `effect = desc[3] >> 2` (0x761a22..0x761a27).
- effect 2 or 3: `M = (envFrame ? GetWorldNormalizedMultiplyTranspose(invert(LTM(envFrame)) * K) : GetWorldNormalizedViewMultiplyTranspose(K))`
  with `K` = the .rdata matrix at 0x875250 `{0.5,0,0,0, 0,-0.5,0,0, 0,0,1,0, 0,0,0,0}` (0x761ae1..0x761b2a); `M[3] <- 0.5f`,
  `M[7] <- 0.5f` (0x761b41/0x761b49); `SetVertexShaderConstantF(disp[2] + (effect == 2 ? 0 : 2), M, 2)` (0x761b62).
- other effects: `p = GetPointInLocalSpace(LTM(G->curCamera->frame)->pos)` (0x761a56), `N = GetWorldNormalizedTransposeMatrix()`
  (0x761a60); effect 0xa: `SetVertexShaderConstantF(disp[2], &p, 1)` and `(disp[2] + 1, N, 3)`; else `(disp[2] + 2, &p, 1)` and
  `(disp[2] + 3, N, 3)` (0x761a78..0x761acf).
- `[0x875250] = 0.5f`.
- librw: absent.

