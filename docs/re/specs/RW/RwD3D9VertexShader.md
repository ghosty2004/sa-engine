# RW D3D9 driver: generated vertex shaders (RE-SPEC-RW-2 writer B)

RW 3.6's D3D9 "vertex shader pipeline" (used by the skin plugin, `_rpD3D9SkinPipelineInit` 0x7cb240) builds vs_1_1 /
vs_2_0 / vs_2_x assembly text from a 32-bit descriptor, assembles it with D3DX and caches the result. All functions
`__cdecl`. Engine = [0xc97b24]; engine+0xf0 = `rwsprintf` (RwStringFunctions first slot; returns the number of
characters written). Device = [0xc97c28].

## Layouts and globals (each proven at the cited instruction)

Vertex-shader descriptor (u32 bit-field, read bytewise; names follow the RW 3.6 SDK `_rpD3D9VertexShaderDescriptor`,
the bit positions are the bytes' masks):
| byte | bits | meaning | evidence |
|---|---|---|---|
| 0 | 0-3 | numDirectionalLights | 0x75eddb, 0x76151a |
| 0 | 4-7 | numPointLights | 0x75edea, 0x7615b3 |
| 1 | 0-3 | numSpotLights | 0x75ee09, 0x7616f7 |
| 1 | 4-7 | numTexCoords | 0x760bde-0x760c0a |
| 2 | 0 | morphing | 0x75ee4b, 0x75f1ec |
| 2 | 1-3 | numWeights | 0x75f0f4/0x75f102 |
| 2 | 4 | prelit | 0x75f2e1 |
| 2 | 5 | normals | 0x75f148 |
| 2 | 6 | normalizeNormals (proposed) | 0x75f0f6/0x75f105 |
| 2 | 7 | modulateMaterial | 0x75ee2b, 0x75f0ee |
| 3 | 0-1 | fogMode | 0x75ee31, 0x75f108 |
| 3 | 2-7 | effect | 0x75ee3f, 0x75f0ff |

Dispatch descriptor (8 bytes, arg of GetVertexShader/Generate, filled with constant-register indices, 0xff = unused):
+0 material colour, +1 fog, +2 effect constants, +3 morph weights, +4 bone matrices (0x75f176-0x75f21d).

| addr | type | meaning | evidence |
|---|---|---|---|
| 0xc926f8 | entry[0x100], 0x14 each | shader cache sorted by key: +0 descriptor, +4 last-used renderFrame (u16 zero-ext), +8 IDirect3DVertexShader9*, +0xc/+0x10 dispatch descriptor | 0x75ef13, 0x75ef58, 0x75ef69 |
| 0xc93af8 | char[0x1000] | shader source text buffer | 0x75f24b, 0x760c58 |
| 0xc94af8 | u32[0x40] | constant registers used per effect (index = descriptor effect) | 0x760d03-0x760d6b, 0x75ee42 |
| 0xc94bf8 | i32 | cache entry count | 0x75eee6, 0x75f099 |
| 0xc94bfc | entry* | last hit | 0x75eed0, 0x75ef44 |
| 0xc94c00 | u32 | VS profile: 0, 10 (vs_1_1), 20 (vs_2_0), 25 (vs_2_x) | 0x760d78-0x760ddf, 0x75f222-0x75f234 |
| 0xc94c04 | u32 | usable constant registers (MaxVertexShaderConst, max 0x100) | 0x760d82-0x760d93, 0x75f210 |
| 0xc94c70 | RwMatrix* | active world matrix (NULL = identity) | 0x76465a |
| 0xc94c74 / 0xc94c78 / 0xc94c7c | RwMatrix* | cached inverse world / normal matrix / third cache (cleared only) | 0x7646cb-0x7646d7 |
| 0x8d7390 | D3DMATRIX | world matrix copy (rows at +0, +0x10, +0x20, +0x30; the w column keeps its initial 0,0,0,1) | 0x764663-0x7646c6 |
| 0x8d73d0 / 0x8d7410 | D3DMATRIX | inverse world / orthonormalised inverse | 0x7647f7, 0x764df3 |
| 0x8753d0 | D3DMATRIX (.rdata) | identity | 0x764867 |
| 0xc94c30 | D3DMATRIX | view*projection product | 0x7646f6 |
| 0xc97c64 | D3DMATRIX* | `_RwD3D9ActiveViewProjTransform` (NULL = recompute) | 0x7646e3 |

Constants: `[0x858624] = 1.0f`, `[0x863e2c] = 0.999f`, `[0x875290] = 999.99994f`.

## 0x75edd0 _rpD3D9GetNumConstantsUsed  (catalogue: specified/REA, port level 0)
- `RwUInt32 (const RwUInt32* desc)`; caller skin render 0x7c8060.
- `n = 5`; dirs d: if d: `n = 2d + 5` (0x75eddd-0x75ede6); points p: `n += (p+3)/4 + 2p` (0x75edea-0x75ee06; signed /4 via
  cdq/and 3/sar, p >= 0); spots s: `n += (s+3)/4 + 4s` (0x75ee09-0x75ee23); +1 if modulateMaterial (byte2 bit 7,
  0x75ee29); +1 if fogMode != 0 (0x75ee31); `n += [0xc94af8 + 4*effect]` if effect != 0 (0x75ee36-0x75ee42); +1 if
  morphing (0x75ee4b). Return n. (Bone matrices are not counted.)
- librw: absent (librw ships precompiled shaders).

## 0x75eed0 _rpD3D9GetVertexShader  (catalogue: specified/REA, port level 2)
- `IDirect3DVertexShader9* (const RwUInt32* desc, RwUInt8 dispatch[8] /*out*/)`; key = *desc.
- Hit on the last entry ([0xc94bfc]->key == key, 0x75eee2) -> step "found".
- Binary search over [0, count-1] (0x75eef2-0x75ef2f): `n = hi-lo+1`, `mid = lo + n/2 + (n&1) - 1`; key == entry ->
  found; key < entry (unsigned) -> hi = mid-1, else lo = mid+1; loop while n > 1. With one candidate (0x75ef7f): equal ->
  found; key < entry -> insert at lo; else insert at lo+1. Empty -> insert at lo (0).
- Found: last = entry; `entry.frame = engine.renderFrame` (u16 +8, 0x75ef53); dispatch = entry +0xc/+0x10; return
  entry.shader.
- Miss: `shader = _rpD3D9VertexShaderGenerate(desc, dispatch)` (0x75efa0, fills dispatch). If count >= 0x100: victim =
  first entry with the smallest frame (strict `<` scan over entries 1..count-1, 0x75efc6-0x75efd6); overwrite key and
  frame, `RwD3D9DeleteVertexShader(old)` if any, store shader and dispatch; the array is NOT re-sorted (original bug:
  later binary searches can miss this entry and generate a duplicate). Else `memmove` the tail one slot up
  (0x75f03f) and store at the insertion index; count++. last = the entry; return shader (also NULL is cached).
- librw: absent.

## 0x75f0b0 _rpD3D9VertexShaderGenerate  (catalogue: unknown/REA, port level 1)
- `IDirect3DVertexShader9* (const RwUInt32* desc, RwUInt8 dispatch[8])`; ~1757 insns, extents 0x75f0b0-0x75f360,
  0x75f3a2-0x76077b, 0x760bd6-0x760ca1 (the gap 0x76077b-0x760bd6 holds further emission cases reached by jumps from
  this body; the catalogue lists it separately). Text is appended at `p` (esi) starting at 0xc93af8 with
  `p += rwsprintf(p, fmt, ...)` (engine+0xf0; 166 call sites).
- Block 1, decode (0x75f0b0-0x75f164): fields as in the table above. `needNormals = normals && (byte0 != 0 ||
  numSpot != 0 || effect in {1,2,3,8,9,0xa,0xb})` (0x75f10f-0x75f14d); `tangentSpace = needNormals && effect in {8,9}`
  (0x75f152-0x75f160).
- Block 2, register allocation (0x75f164-0x75f21d): dispatch[0..7] = 0xff; `r = 5`, then `r` advances exactly as in
  0x75edd0 for directional/point/spot lights; dispatch[0] = r++ if modulateMaterial; dispatch[1] = r++ if fogMode > 0;
  dispatch[3] = r++ if morphing; dispatch[4] = r if numWeights > 0 (bones start here); if effect has constants
  (`k = [0xc94af8 + 4*effect]` != 0): with bones dispatch[2] = `u8([0xc94c04] - k)` (top of the register file), else
  dispatch[2] = r, r += k.
- Block 3, profile (0x75f222-0x75f285): [0xc94c00] == 10 -> "vs_1_1\n", 20 -> "vs_2_0\n", 25 -> "vs_2_x\n"; any other
  value -> return NULL (0x75f236-0x75f23f).
- Block 4, declarations (0x75f285-0x75f43f): "dcl_position0 v0"; numWeights > 0: "dcl_blendweight0 v1" (only if > 1,
  0x75f2ab) and "dcl_blendindices0 v2"; prelit (byte2 bit 4) -> "dcl_color0 v4" (0x75f2e1); needNormals -> "dcl_normal0 v3" (0x75f302);
  tangentSpace -> "dcl_tangent0 v13" (0x75f324); one "dcl_texcoord%u v%u" per texcoord (args i, i+5, 0x75f3b4-0x75f3de); morphing ->
  "dcl_position1 v14" and (normals) "dcl_normal1 v15". The normal source name defaults to "v3" (0x75f430).
- Block 5, position / skinning / morph (0x75f43f-0x75f944): no weights and no morph -> "m4x4 oPos.xyzw, v0, c[0]"
  (0x75f454) and skip to fog. Morph: r3 = v0*c[m].x + v14*c[m].y (and r5 for normals from v3/v15), or r2/r4 when also
  skinning (0x75f468-0x75f561). Skinning (0x75f561-0x75f918): one weight -> "mov a0.x, v2.x" (vs_1_1) or
  "mova a0.x, v2.x" then m4x3/m3x3 with `c[a0.x + bone]`; 2-4 weights -> "mova a0.%s, v2" and per-row mul/mad
  accumulation into r8/r9/r10 with the swizzle letters x/y/z/w (the vs_1_1 path reloads a0.x per weight with
  "mov a0.x, v2.%c"); then "m4x3 r3.xyz, %s, r8" / "m3x3 r5.xyz, %s, r8" or the per-weight m4x3 + mul/mad blend.
  Then "mov r3.w, v0.w" and "m4x4 oPos.xyzw, r3, c[0]" (0x75f91e-0x75f931).
- Block 6, fog (0x75f944-0x75fa35): fogMode != 0: "dp4 r0.w, %s, c[3]" then per mode: EXP2 (mul by c[fog].x, square,
  expp, rcp oFog), EXP (mul, expp, rcp oFog), LINEAR ("mad oFog, r0.w, c[%d].x, c[%d].y").
- Block 7, normalisation (0x75fa35-0x75fa92): normalizeNormals and needNormals: "dp3 r5.w, %s, %s", "rsq r5.w, r5.w",
  "mul r5.xyz, %s, r5.w"; the normal source becomes "r5".
- Block 8, lighting (0x75fa92-0x7600f3): ambient base r11 = "add r11, v4, c[4]" (prelit) or "mov r11, c[4]"; per
  directional light (registers from 5 upward, two per light): "dp3 r6.%c, c[%d], %s", "max r6.%s, c[5].w, -r6.%s",
  then the last light writes "mad oD0, c[%d], r6.%c, r11" and the others "mad r11.xyz, c[%d].xyz, r6.%s, r11.xyz";
  point lights (0x75fc88-0x75fdb0): vector to light, distance, "max r8..", 1 - d*invRadius attenuation, mul, mad; spot
  lights (0x75fe4e-0x760000) additionally the cone term "dp3 r7.w, r7, c[%d]" / "mad r9.%c, r7.w, c[%d].x, c[%d].y" /
  "max r9.%s, c[%d].wwww, r9.%s" / "mul r6.%s, r6.%s, r9.%s". Without lights: "add oD0, v4, c[4]" or "mov oD0, c[4]";
  modulateMaterial -> "mul oD0, r11, c[%d]" (0x760060 / 0x7600bb).
- Block 9, effects (0x7600f3-0x76077b and the gap 0x76077b-0x760bd6): tangent frame (r4 from v13 through bones or r8,
  normalised; bitangent r2 = cross(r4, normal)), environment / bump-map texture coordinates written to oT0..oT3, oD0,
  oD1 per effect (strings at 0x8d66c8..0x8d6b4c listed below).
- Block 10, texcoords (0x760bd6-0x760c4b): for i < numTexCoords: "mov oT%u.xy, v%u" with (i, i+5).
- Block 11, assemble (0x760c4d-0x760c95): `D3DXAssembleShader(0xc93af8, p - 0xc93af8, NULL, NULL, 4, &buf, NULL)`
  (0x7652ae, d3d9.md section 8); on hr >= 0: `RwD3D9CreateVertexShader(buf->GetBufferPointer() /*vtbl+0xc*/, &shader)`
  (0x7fac60); `buf->Release()` (vtbl+8) if buf; return shader (NULL if anything failed).
- Format strings (instruction address, data address, text; "\n" terminated in the exe):
  0x75f246 0x8d7364 `vs_2_x`; 0x75f25d 0x8d735c `vs_2_0`; 0x75f275 0x8d7354 `vs_1_1`; 0x75f296 0x8d7340
  `dcl_position0 v0`; 0x75f2b5 0x8d7328 `dcl_blendweight0 v1`; 0x75f2cc 0x8d7310 `dcl_blendindices0 v2`; 0x75f2ed
  0x8d7300 `dcl_color0 v4`; 0x75f30f 0x8d72f0 `dcl_normal0 v3`; 0x75f332 0x8d72dc `dcl_tangent0 v13`; 0x75f3be 0x8d72a0
  `dcl_texcoord%u v%u`; 0x75f3f0 0x8d728c `dcl_position1 v14`; 0x75f40f 0x8d7278 `dcl_normal1 v15`; 0x75f454 0x8d7254
  `m4x4 oPos.xyzw, v0, c[0]`; 0x75f486 0x8d723c `mul r3, v0, c[%d].x`; 0x75f49b 0x8d7220 `mad r3, v14, c[%d].y, r3`;
  0x75f4c6 0x8d7204 `mul r5, v3, c[%d].x`; 0x75f4db 0x8d71e8 `mad r5, v15, c[%d].y, r5`; 0x75f4f7 0x8d71d0
  `mul r2, v0, c[%d].x`; 0x75f50b 0x8d71b4 `mad r2, v14, c[%d].y, r2`; 0x75f533 0x8d7198 `mul r4, v3, c[%d].x`;
  0x75f548 0x8d717c `mad r4, v15, c[%d].y, r4`; 0x75f57c 0x8d7168 `mov a0.x, v2.x`; 0x75f590 0x8d7154
  `mova a0.x, v2.x`; 0x75f5ac 0x8d7134 `m4x3 r3.xyz, %s, c[a0.x + %d]`; 0x75f5d5 0x8d7114 `m3x3 r5.xyz, %s, c[a0.x + %d]`;
  0x75f613 0x8d7104 `mova a0.%s, v2`; 0x75f645 0x8d70e0 / 0x75f662 0x8d70bc / 0x75f67d 0x8d7098
  `mad r8|r9|r10, v1.%c, c[a0.%c + %d], r8|r9|r10`; 0x75f688 0x8d7078 / 0x75f6a0 0x8d7058 / 0x75f6b6 0x8d7038
  `mul r8|r9|r10, v1.%c, c[a0.%c + %d]`; 0x75f6f9 0x8d7024 `mov a0.x, v2.%c`; 0x75f71d 0x8d7000 / 0x75f73a 0x8d6fdc /
  0x75f754 0x8d6fb8 `mad r8|r9|r10, v1.%c, c[a0.x + %d], ...`; 0x75f763 0x8d6f98 / 0x75f77a 0x8d6f78 / 0x75f78e
  0x8d6f58 `mul r8|r9|r10, v1.%c, c[a0.x + %d]`; 0x75f7c0 0x8d6f40 `m4x3 r3.xyz, %s, r8`; 0x75f7d9 0x8d6f28
  `m3x3 r5.xyz, %s, r8`; 0x75f833 0x8d6f08 `m4x3 r5.xyz, %s, c[a0.%c + %d]`; 0x75f850 0x8d6eec `mul r3.xyz, v1.%c,
  r5.xyz`; 0x75f86b 0x8d6ec8 `mad r3.xyz, v1.%c, r5.xyz, r3.xyz`; 0x75f8b1 0x8d6ea8 `m4x3 r5.xyz, %s, c[a0.x + %d]`;
  0x75f91e 0x8d6e94 `mov r3.w, v0.w`; 0x75f931 0x8d6e78 `m4x4 oPos.xyzw, r3, c[0]`; 0x75f960 0x8d6e64
  `dp4 r0.w, %s, c[3]`; 0x75f99a 0x8d6e48 `mul r0.w, r0.w, c[%d].x`; 0x75f9ae 0x8d6e30 `mul r0.w, r0.w, r0.w`;
  0x75f9c2 0x8d6e1c `expp r0.w, r0.w`; 0x75f9d5 0x8d6e0c `rcp oFog, r0.w`; 0x75fa35 0x8d6de8
  `mad oFog, r0.w, c[%d].x, c[%d].y`; 0x75fa5d 0x8d6dd4 `dp3 r5.w, %s, %s`; 0x75fa71 0x8d6dc4 `rsq r5.w, r5.w`;
  0x75fa86 0x8d6dac `mul r5.xyz, %s, r5.w`; 0x75fa92 0x8d6ea4 `r5`; 0x75fae9 0x8d6d98 `add r11, v4, c[4]`; 0x75fafd
  0x8d6d88 `mov r11, c[4]`; 0x75fb6e 0x8d6d70 `dp3 r6.%c, c[%d], %s`; 0x75fb96 0x8d6d54 `max r6.%s, c[5].w, -r6.%s`;
  0x75fbce 0x8d6d38 `mad oD0, c[%d], r6.%c, r11`; 0x75fbeb 0x8d6d10 `mad r11.xyz, c[%d].xyz, r6.%s, r11.xyz`;
  0x75fc88 0x8d6cf0 `add r7.xyz, -c[%d].xyz, %s.xyz`; 0x75fca4 0x8d6cdc `dp3 r1.%c, r7, r7`; 0x75fcc0 0x8d6cc8
  `rsq r1.%c, r1.%c`; 0x75fce0 0x8d6cb4 `dp3 r6.%c, %s, r7`; 0x75fd0e 0x8d6c98 `max r8.%s, c[%d].%s, r1.%s`; 0x75fd27
  0x8d6c78 `max r6.%s, c[%d].wwww, -r6.%s`; 0x75fd3f 0x8d6c58 `add r8.%s, -c[%d].%s, r8.%s`; 0x75fd59 0x8d6c3c
  `mul r6.%s, r6.%s, r8.%s`; 0x75fec4 0x8d6c20 `mul r7.xyz, r7.xyz, r1.%c`; 0x75fede 0x8d6c08 `dp3 r7.w, r7, c[%d]`;
  0x75fefc 0x8d6be4 `mad r9.%c, r7.w, c[%d].x, c[%d].y`; 0x75ff45 0x8d6bc4 `max r9.%s, c[%d].wwww, r9.%s`; 0x75ffa8
  0x8d6ba8 `mul r6.%s, r6.%s, r9.%s`; 0x760060 0x8d6b90 `mul oD0, r11, c[%d]`; 0x7600d9 0x8d6b7c `add oD0, v4, c[4]`;
  0x7600f3 0x8d6b6c `mov oD0, c[4]`; 0x76011a 0x8d6b4c `m3x3 r4.xyz, v13, c[a0.x + %d]`; 0x760131 0x8d6b34
  `m3x3 r4.xyz, v13, r8`; 0x760150 0x8d6b20 `dp3 r4.w, r4, r4`; 0x760163 0x8d6b10 `rsq r4.w, r4.w`; 0x760177 0x8d6af8
  `mul r4.xyz, r4, r4.w`; 0x76018e 0x8d6ae0 `slt r4.w, v13.w, v13.w`; 0x7601a1 0x8d6ad0 `mov r4, v13`; 0x7601c7
  0x8d6ac0 `mov %s.w, v3.w`; 0x7601df 0x8d6aa4 `mul r2, r4.zxyw, %s.yzxw`; 0x7601f3 0x8d6a84
  `mad r2, r4.yzxw, %s.zxyw, -r2`; 0x760212 0x8d6a70 `dp3 r2.w, r2, r2`; 0x760226 0x8d6a60 `rsq r2.w, r2.w`; 0x760239
  0x8d6a48 `mul r2.xyz, r2, r2.w`; 0x760256 0x8d6a38 `mov oD0, v4`; 0x760277 0x8d6a24 `dp3 r1.x, c[5], r4`; 0x76028a
  0x8d6a10 `dp3 r1.y, c[5], r2`; 0x76029f 0x8d69fc `dp3 r1.z, c[5], %s`; 0x7602b3 0x8d69e8 `dp3 r1.w, r1, r1`; 0x7602c6
  0x8d69d8 `rsq r1.w, r1.w`; 0x7602da 0x8d69c0 `mul r1.xyz, r1, r1.w`; 0x7602f4 0x8d699c `mad oT2.xyz, r1, c[5].w,
  c[5].w`; 0x760309 0x8d6978 `mad oD1.xyz, r1, c[5].w, c[5].w`; 0x76031f 0x8d6954 `mad oD0.xyz, r1, c[5].w, c[5].w`;
  0x76033b 0x8d6930 `add r7.xyz, -c[1 + 5].xyz, %s.xyz`; 0x76034f 0x8d691c `dp3 r7.w, r7, r7`; 0x760363 0x8d690c
  `rsq r7.w, r7.w`; 0x760376 0x8d68f4 `mul r7.xyz, r7, r7.w`; 0x76038a 0x8d68e4 `rcp r7.w, r7.w`; 0x76039e 0x8d68c4
  `mad r7.w, r7.w, c[5].x, c[5].y`; 0x7603b1 0x8d68ac `max r7.w, r7.w, c[5].w`; 0x7603c5 0x8d6898 `dp3 r1.x, r7, r4`;
  0x7603dc 0x8d6884 `dp3 r1.y, r7, r2`; 0x7603f0 0x8d6870 `dp3 r1.z, r7, %s`; 0x76043f 0x8d6858 `mul r1.xyz, r1,
  r7.w`; 0x76045d 0x8d6834 `mad oT2.xyz, r1, c[5].z, c[5].z`; 0x760491 0x8d681c `dp3 oT2.x, c[%d], r4`; 0x7604a5
  0x8d6804 `dp3 oT2.y, c[%d], r2`; 0x7604bf 0x8d67ec `dp3 oT2.z, c[%d], %s`; 0x7604d5 0x8d67d4 `dp3 oT3.x, c[%d], r4`;
  0x7604e9 0x8d67bc `dp3 oT3.y, c[%d], r2`; 0x760502 0x8d67a4 `dp3 oT3.z, c[%d], %s`; 0x760528 0x8d6790
  `mov oT0.xy, v%d`; 0x76053c 0x8d6780 `mov oT1.xy, v5`; 0x760583 0x8d675c `mad oD1.xyz, r1, c[5].z, c[5].z`;
  0x760599 0x8d6738 `mad oD0.xyz, r1, c[5].z, c[5].z`; 0x760606 0x8d6720 `dp3 r9.w, r7, c[2 + 5]`; 0x76061a 0x8d66f8
  `mad r9.w, r9.w, c[3 + 5].x, c[3 + 5].y`; 0x76066b 0x8d66e0 `max r9.w, r9.w, c[5].w`; 0x76070d 0x8d66c8
  `mul r1.xyz, r1, r9.w`; 0x760bf3 / 0x760c2f 0x8d641c `mov oT%u.xy, v%u`; gap block: 0x76078b 0x8d66b8, 0x7607a4
  0x8d66a0, 0x7607bb 0x8d6688, 0x7607d7 0x8d6670 (texture-coordinate cases, see open questions).
- Confidence low: blocks 1-4 and 10-11 are decoded from the bytes; for blocks 5-9 the emission ORDER and strings are
  exact (table above) but the per-string branch conditions and the `%d`/`%c`/`%s` arguments must be ported
  instruction by instruction from the cited ranges.
- librw: absent (librw uses precompiled HLSL shaders in d3d/shaders).

## 0x760cf0 _rpD3D9VertexShaderCacheReset  (catalogue: unknown/REA, port level 0)
- `void ()`; caller `_rpCreateD3D9AtomicAllInOnePipe` 0x74eb80 (behaves as the cache/limits open).
- `[0xc926f8..0xc92708]` (first cache entry: key, frame, shader, dispatch) = 0xffffffff (0x760cf4-0x760d1a); count
  [0xc94bf8] = 0; table 0xc94af8 zeroed (0x40 dwords) then effect counts: [1] = 2, [2] = 2, [3] = 4, [6] = 2, [7] = 2,
  [9] = 2, [0xa] = 4, [0xb] = 6 (0x760d2d-0x760d6b); last-hit [0xc94bfc] = 0xc926f8.
- Profile from VertexShaderVersion (D3DCAPS9+0xc4 [0xc9bfc4] & 0xffff): >= 0x300 -> 25; >= 0x200 -> 25 if
  VS20Caps.DynamicFlowControlDepth ([0xc9bffc], signed) > 0 else 20 (setle/dec/and 5/add 0x14, 0x760da5-0x760db8);
  >= 0x100 -> 10 and DeclTypes [0xc9bfec] &= 0xc1 (the caps copy itself is modified, 0x760dc7-0x760dd8); else 0 and
  return without setting the limit (0x760ddf). Then `[0xc94c04] = min(MaxVertexShaderConst [0xc9bfc8], 0x100)`
  (unsigned, 0x760d82-0x760d93).
- Note: the existing cached shaders are not released here.

## 0x761000 _rxD3D9VertexShaderDefaultEndCallBack  (catalogue: specified/REA, port level 0)
- `void (...)`: `ret` only (0x761000).

## 0x761010 _rxD3D9VertexShaderDefaultGetMaterialShaderCallBack  (catalogue: specified/REA, port level 3)
- `IDirect3DVertexShader9* (const RpMaterial* /*unused*/, const RwUInt32* desc, RwUInt8* dispatch)`: tail of
  `_rpD3D9GetVertexShader(desc, dispatch)` (args [esp+8], [esp+0xc], 0x761010-0x76101a).

## 0x761030 _rxD3D9VertexShaderDefaultMeshRenderCallBack  (catalogue: specified/REA, port level 2)
- `void (RxD3D9ResEntryHeader* h, RxD3D9InstanceData* inst)`.
- If [0x8e244c] != 0: clear it and `device->SetPixelShader(NULL)` (vtbl+0x1ac, 0x76104d).
- material->texture: `RwD3D9SetTexture(tex, 0)`; stage 0 COLOROP 4 MODULATE, COLORARG1 2 TEXTURE, COLORARG2 0 DIFFUSE,
  ALPHAOP 4, ALPHAARG1 2, ALPHAARG2 0 (0x761064-0x7610aa). Else `RwD3D9SetTexture(NULL, 0)`, COLOROP 3 SELECTARG2,
  COLORARG2 0, ALPHAOP 3, ALPHAARG2 0 (0x7610b4-0x7610e2).
- `inst.vertexShader` set through the [0x8e2448] cache (+0x170, 0x761108); flush the render-state cache; draw with
  DrawIndexedPrimitive(h.primType, inst.baseIndex, 0, inst.numVertices, inst.startIndex, inst.numPrimitives)
  (+0x148, 0x76113c) when h.indexBuffer, else DrawPrimitive(h.primType, inst.baseIndex, inst.numPrimitives) (+0x144).
- librw: d3d9render.cpp drawInst equivalent for the draw part; texture stage setup absent.

## 0x761170 _rxD3D9VertexShaderDefaultLightingCallBack  (catalogue: specified/REA, port level 5)
- `RwV4d* (void* object, RwUInt32 type, RwV4d* out /*constant data*/, RwUInt32* desc)`; returns the pointer after the
  last written vec4. Locals: dir lights [15] (esp+0x20, count esp+0x1c), point [15] (esp+0x60/0x5c), spot [15]
  (esp+0xa0/0x9c).
- desc byte0 = 0, byte1 &= 0xf0 (0x761190-0x761192). flags = (type == 1) ? atomic->geometry->flags :
  curWorld->flags (+8) (0x761195-0x7611ab). out[0] = ambient (0, 0, 0, prelit (flags & 8) ? 0 : 1.0f) (0x7611b5-0x7611c6).
- If `!(flags & 0x20)` or no world: return out+1 (0x76170c).
- Atomic (type == 1, 0x7611fe-0x7613bc), normals = flags bit 4: world+0x3c list with `light.flags & 1`: subType 2
  AMBIENT -> ambient.rgb += color (x87 `fld color; fadd acc; fstp`, 0x761228-0x76123e, no lit flag); subType 1 ->
  if normals and dirCount < 15: append, lit = 1. If normals: lightFrame++, sector ties of the atomic as in 0x757400
  (same x87 sphere test `r2 > d2`, 0x7612fb-0x76133a): 0x80 -> point list, 0x81/0x82 -> spot list (each < 15), lit = 1.
- Sector (0x7613c1-0x7614b2): world list with `light.flags & 2 LIGHTWORLD`: ambient as above, directional appended
  (< 15) without a normals test; local lights from `sector+0x40` with flags & 2 and the lightFrame mark, NO sphere test.
- lit == 0 -> return out+1. Else write after the ambient (p = out+1):
  - each directional: `_rwD3D9VSGetNormalInLocalSpace(&LTM->at, p)`, p.w = 0; next vec4 = color (r, g, b, 0)
    (0x7614cf-0x761507); desc byte0 low nibble = count.
  - points: a vec4 "reciprocal radius" slot is reserved before lights 0, 4, 8, ... (`i & 3 == 0`, 0x761541-0x761548);
    per light: `_rwD3D9VSGetPointInLocalSpace(&LTM->pos, p)`, w = 0; color (r, g, b, 0);
    `_rwD3D9VSGetRadiusInLocalSpace(radius, slot)` then `*slot = 1.0f / *slot` (`[0x858624] = 1.0f`, fld/fdiv,
    0x761588-0x7615a3), slot advances by 4 bytes; desc byte0 high nibble = count.
  - spots: own slot grouping counter; per light: position, direction (`GetNormalInLocalSpace(&LTM->at)`), then
    `c = cos(RpLightGetConeAngle(light))` (x87 `fcos` on the ST0 return, 0x761646). subType 0x81 or `c >= 0.999f`
    (`[0x863e2c] = 0.999f`; `fcom` C0 clear; NaN takes the soft path): (999.99994f, -c * 999.99994f) with
    `[0x875290] = 999.99994f` (0x761676-0x761684); else `k = 1.0f / (1.0f - c)`, (k, -c * k) (0x761659-0x76166f);
    then .z = 0.5f, .w = 0; color vec4; radius slot as for points; desc byte1 low nibble = count.
- librw: absent (librw uploads lights in its own constant layout).

## 0x764650 _rwD3D9VSSetActiveWorldMatrix  (catalogue: specified/REA, port level 0)
- `void (const RwMatrix* m)`; [0xc94c70] = m; if m: copy right/up/at/pos (3 floats each) into the rows of 0x8d7390
  (0x764661-0x7646c6); always clear [0xc94c74], [0xc94c78], [0xc94c7c] (0x7646cb-0x7646d7).

## 0x7646e0 _rwD3D9VSGetComposedTransformMatrix  (catalogue: specified/REA, port level 0)
- `D3DMATRIX* (D3DMATRIX* out)`: if [0xc97c64] == NULL: `D3DXMatrixMultiply(0xc94c30, view 0xc9bc80, proj 0x8e2458)`
  (0x7672e1, stdcall) and [0xc97c64] = 0xc94c30. If a world matrix is active: return
  `D3DXMatrixMultiplyTranspose(out, 0x8d7390, viewProj)` (0x7676c7), else `D3DXMatrixTranspose(out, viewProj)` (0x7675ee).

## 0x7647b0 _rwD3D9VSGetInverseWorldMatrix  (catalogue: specified/REA, port level 2)
- `void (D3DMATRIX* out)`: if [0xc94c74] cached -> copy 16 dwords (0x764878). Else if no world or `flags & 0x20000`
  (identity): both caches = 0x8753d0 (identity), copy. Else if `(flags & 3) == 3` (orthonormal):
  `_rwD3D9MatrixInvertOrthonormal(0x8d73d0, world)`; [0xc94c74] = [0xc94c78] = 0x8d73d0. Else
  `RwMatrixInvert(0x8d73d0, world)`, then dwords +0xc/+0x1c/+0x2c = 0 and +0x3c = 1.0f (stores 0x764833-0x764851),
  [0xc94c74] = 0x8d73d0 (normal cache untouched). Copy 16 dwords to out.

## 0x764d30 _rwD3D9VSGetNormalInLocalSpace  (catalogue: specified/REA, port level 3)
- `void (const RwV3d* in, RwV3d* out)`. If [0xc94c78] is set: use it; else if no world / identity: caches = identity.
  Orthonormal world: inverse (cached in [0xc94c74] or computed by 0x764890) also becomes the normal matrix. Otherwise:
  inverse via RwMatrixInvert (+ w column fix as in 0x7647b0, if not cached), then
  `RwMatrixOrthoNormalize(0x8d7410, inverse)`, w column (0, 0, 0, 1.0f) (0x764e03-0x764e2c), [0xc94c78] = 0x8d7410.
- Identity -> `*out = *in` (0x764e4c-0x764e5f); else `D3DXVec3TransformNormal(out, in, M)` (0x766ab7).

## Open questions
- UNCLEAR: 0x75f0b0 blocks 5-9 branch conditions (0x75f43f-0x76077b) and the cases in 0x76077b-0x760bd6 (catalogued as a
  separate function; strings 0x8d66b8, 0x8d66a0, 0x8d6688, 0x8d6670 emitted there) were not decoded condition by
  condition within this pass.
- UNCLEAR: byte2 bit 6 meaning (named normalizeNormals here) - only its use in block 7 was seen (0x75f0f6).
