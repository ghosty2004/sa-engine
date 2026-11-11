# RW D3D9 driver: vertex-declaration instancing helpers - pass-3 specs, writer A (RE-SPEC-RW-3)

Extends docs/re/specs/RW/RwD3D9Instance.md (pass 2, read-only), whose `_rpD3D9VertexDeclarationInstV3d` 0x752ad0
spec defines the per-D3DDECLTYPE encodings reused here. All `__cdecl`. gta-reversed has no code for these.

## Layouts used (offsets proven by the instruction addresses given)

`RxD3D9ResEntryHeader` (see RwD3D9Instance.md): +0x4 numMeshes (0x754e7e), +0x8 `IDirect3DIndexBuffer9*` indexBuffer
(0x754e68), +0xc primType (D3DPT: 4 TRIANGLELIST, 5 TRIANGLESTRIP; 0x754e93, 0x754ed9), +0x38 totalNumIndex
(0x754e6c), +0x3c totalNumVertex (0x754e30), +0x40 first `RxD3D9InstanceData` (0x24 bytes: +0x0 numIndex 0x754e99,
+0x4 minVert 0x754edf, +0x1c startIndex 0x754e8c; stride 0x24 at 0x755143).

Data constants: `[0x858624] = 1.0f`, `[0x859aac] = 255.0f`, `[0x86b2c0] = 32767.0f`, `[0x8729e8] = 65535.0f`,
`[0x874f3c] = 511.0f`, `[0x874f44] = 1e-12f` (exact f32 0x2b8cbccc), element-size table `[0x874ef8]` (u32 per
D3DDECLTYPE: 4, 8, 12, 16, 4, 4, 4, 8, 4, 4, 8, 4, 8, 4, 4, then 4, 8 for 0xf/0x10).
`[0x874ef8] = 0x4` (dword).

## 0x753b60 _rpD3D9VertexDeclarationInstV3dMorph  (catalogue: specified/REA, port level 0)
- Convention: `RwUInt32 (RwUInt32 type, RwUInt8* dst, const RwV3d* a, const RwV3d* b, f32 t, RwInt32 n, RwUInt32
  stride)` (stack after `push ecx` + 4 pushes: type +0x18, dst +0x1c, a +0x20, b +0x24, t +0x28, n +0x2c, stride +0x30;
  0x753b61, 0x753b79..0x753b91). Returns `[0x874ef8 + 4*type]` (e.g. 0x753bb4); `type > 0xe` (`ja`, 0x753b6c) only
  returns that value (types 0xf/0x10 give 4/8). Jump table 0x7544a4 (15 entries).
- Every case: `n <= 0` (signed `jle`) -> return without writing; else loop do-while `n` times, `a`/`b` advance 0xc,
  `dst` advances `stride`. Integer outputs: the f32 value is stored to a temp and reloaded, then `fistp` (current
  x87 rounding mode, round-to-nearest in game) and the low byte/word is stored.
- Lerp cases use `v = (b.k - a.k)*t + a.k` (x87: `fld b; fsub a; fmul t; fadd a`):
  - 0 FLOAT1 (0x753b95..0x753ba6): f32 x. 1 FLOAT2 (0x753bda..0x753bfa): x, y. 2 FLOAT3 (0x753c32..0x753c63): x, y, z.
    3 FLOAT4 (0x753c9f..0x753cd3): x, y, z, then dword 0x3f800000 (1.0f, 0x753c9a/0x753cd0).
  - 5 UBYTE4 (0x753dd7..0x753e34): bytes dst[0] = x, dst[1] = y, dst[2] = z, dst[3] = 1 (0x753dcb/0x753e31).
  - 6 SHORT2 (0x753e6e..0x753ead): u16 x, y. 7 SHORT4 (0x753eee..0x753f51): u16 x, y, z, u16[3] = 1 (0x753ee2).
  - 0xd UDEC3 (0x75433a..0x7543b0): `X, Y, Z` = fistp of the lerps; `dst = (((Z & 0x3ff) << 10 | (Y & 0x3ff)) << 10) |
    (X & 0x3ff)` (0x75438a..0x7543ab).
- Normalised cases precompute `k0 = f32((1.0f [0x858624] - t) * S)` and `k1 = f32(t * S)` (k1 overwrites the `t`
  slot) and use `v = f32(k0*a.k + k1*b.k)` (or the other sum order where noted), then `fistp`:
  - 4 D3DCOLOR, `S = 255.0f [0x859aac]` (0x753ceb..0x753d0f): `dst[2] = x`, `dst[1] = y`, `dst[0] = z`, `dst[3] = 0xff`
    (0x753d50, 0x753d73, 0x753d94, 0x753d91); sums `k0*a + k1*b`.
  - 8 UBYTE4N, `S = 255.0f` (0x753f70..0x753f94): `dst[0] = x, dst[1] = y, dst[2] = z, dst[3] = 0xff` (0x753fd5..0x75401a).
  - 9 SHORT2N, `S = 32767.0f [0x86b2c0]` (0x754038..0x754060): u16 x, y; sums `k0*a + k1*b` (0x754076..0x7540a6).
  - 0xa SHORT4N, `S = 32767.0f` (0x7540d9..0x7540fd): u16 x, y, z, u16[3] = 0x7fff (0x754113/0x754188).
  - 0xb USHORT2N, `S = 65535.0f [0x8729e8]` (0x7541ab..0x7541d3): u16 x, y; x sum order `k1*b + k0*a` (0x7541e9..0x7541f5),
    y `k0*a + k1*b` (0x754208..0x754219).
  - 0xc USHORT4N, `S = 65535.0f` (0x75424c..0x754270): x `k1*b + k0*a` (0x754292..0x75429e), y and z `k0*a + k1*b`;
    u16[3] = 0xffff (0x75428d/0x7542fb).
  - 0xe DEC3N, `S = 511.0f [0x874f3c]` (0x7543c8..0x7543f0): x `k1*b + k0*a` (0x754406..0x754412), y, z `k0*a + k1*b`;
    packed as UDEC3 (0x754460..0x754481).
- Note the normalised cases interpolate the SCALED values (`(1-t)*S*a + t*S*b`) while the lerp cases use `(b-a)*t + a`;
  the two forms round differently. Kept as in the bytes.
- librw: absent (librw has no morph instancing; it interpolates in its own instMorph? differently - exe wins).

## 0x754e20 _rpD3D9VertexDeclarationInstTangent  (catalogue: specified/REA, port level 1)
- Convention: `RwUInt32 (RwUInt32 type, RwUInt8* dst, const RwV3d* pos, const RwTexCoords* uv,
  const RxD3D9ResEntryHeader* hdr, RwUInt32 stride)` (args at [esp+0x64..0x78] after `sub esp,0x50` + 4 pushes;
  hdr read 0x754e25, pos 0x754f90, uv 0x754fa9, final call 0x7551bb..0x7551cb). Returns the result of
  `_rpD3D9VertexDeclarationInstV3d(type, dst, tangents, hdr->totalNumVertex, stride)` (0x7551cc).
- Behaviour:
  1. `nv = hdr->totalNumVertex`; `T = G->rwmalloc(nv*12, 0x1050d)` (G+0x134, 0x754e43; no NULL check), zero-filled
     (`rep stosd`/`stosb`, 0x754e59/0x754e66).
  2. `hdr->indexBuffer->Lock(0, hdr->totalNumIndex*2, &idx, 0)` (vtbl+0x2c, 0x754e7b; result ignored).
  3. For each instance (`hdr->numMeshes` iterations, do-while `dec; jne` 0x755146..0x75514f, no zero test): `base =
     idx + inst->startIndex*2` (u16 indices), `cnt = inst->numIndex`, minus 2 if `hdr->primType == 5` (0x754e9b..0x754eab);
     `for (k = 0; k < cnt;)` (signed, entry `jle` 0x754eb7, loop `jl` 0x755135):
     - primType == 4 (re-read from `hdr` each triangle, 0x754ed9): `A = base[k] + mv`, `B = base[k+1] + mv`, `C =
       base[k+2] + mv` (mv = `inst->minVert`), `k += 3`; no degenerate test (0x754edf..0x754f14).
     - else (strip): k even: `A = base[k], B = base[k+1], C = base[k+2]`; k odd: `A = base[k+2], B = base[k+1], C =
       base[k]` (each + mv; 0x754f16..0x754f5e); `k += 1`; skip the triangle if `A == B || A == C || B == C`
       (0x754f74..0x754f8a).
     - Triangle (x87; f32 temps where stated): `e1x = f32(P[B].x - P[A].x)` (0x754fa3..0x754fb0), `du1 = U[B].u - U[A].u`
       (register), `dv1 = f32(U[B].v - U[A].v)` (0x754fce), `e2x = f32(P[C].x - P[A].x)` (0x754fd8), `du2 = U[C].u -
       U[A].u`, `dv2 = f32(U[C].v - U[A].v)` (`fst`, 0x754ff0); `det = du1*dv2 - du2*dv1` (0x754ff4..0x754ffa, register);
       `sx = f32(e2x*dv1 - dv2*e1x)` (0x754ffe..0x755010).
       If `|det| > [0x874f44] = 1e-12` (`fcomp`; C0|C3 set -> skip, so NaN skips; 0x755018..0x755025): `r = (1.0f / det) *
       sx` (`fdivr [0x858624]`, 0x755027), `T[A].x -= r; T[B].x -= r; T[C].x -= r` (in this order, 0x755031..0x755046).
       Then `e1y = P[B].y - P[A].y`, `e2y = P[C].y - P[A].y`; `det` is stored to an f32 temp `detf` (0x755061);
       `sy = e2y*dv1 - dv2*e1y` (register, 0x755065..0x75506f); if `|detf| > 1e-12` (0x755079): `r = sy * (1.0f / detf)`
       (0x755088..0x755092), subtract from `T[A].y, T[B].y, T[C].y`. Same for z with `e1z, e2z` (0x7550b4..0x75511e;
       `detf` stored again 0x7550c6).
  4. `hdr->indexBuffer->Unlock()` (vtbl+0x30, 0x75515c).
  5. For `v = 0 .. nv-1` (signed, entry `jle` 0x755165): `len2 = f32(y*y + x*x + z*z)` (0x75516c..0x755184), `inv =
     _rwInvSqrt(len2)` (0x7edb90, result in ST0, 0x755193); `T[v].x *= inv`, `.y *= inv`, `.z *= inv` (0x755198..0x7551b4).
     No zero-length guard (`_rwInvSqrt(0)` decides).
  6. `res = InstV3d(type, dst, T, nv, stride)`; `G->rwfree(T)` (G+0x138, 0x7551da); return res.
- Edge: the tangent is accumulated with a minus sign (`T -= ...`). Strip winding flips A and C on odd k.
- librw: absent (librw has no tangent generation).
## 0x7522f0 _rpD3D9VertexDeclarationGetStride  (catalogue: specified/REA, port level 0)
- `u32 (const D3DVERTEXELEMENT9* e)`: sum of `[0x874ef8 + 4*e[i].Type]` over the elements while `Type (+4, u8) < 0x11`
  (the D3DDECL_END type 0x11 stops; 0x7522fc..0x752319); returns the sum (0 for an empty declaration).
- librw: absent (librw computes strides itself).

## 0x7531b0 _rpD3D9VertexDeclarationInstV3dComp  (catalogue: specified/REA, port level 0)
- Convention as InstV3d: `u32 (u32 type, u8* dst, const RpVertexNormal* src /*4 x s8 per vertex*/, i32 n, u32 stride)`; jump
  table 0x753b1c (15 cases, 0x7531cb..0x753a42); `type > 0xe` -> only the size. Returns `[0x874ef8 + 4*type]` (0x753b0b).
- Every case: `n <= 0` -> no write; per vertex the components are `c_k = f32((s8)src[k] * [0x858b88] = 0.0078125)` (`fild`,
  `fmul`; 0x7531e3..0x7531f7) and then encoded like the matching InstV3d case (FLOAT1..FLOAT4, D3DCOLOR / UBYTE4N with
  `[0x859aac] = 255.0f`, SHORT*N with `[0x86b2c0] = 32767.0f`, USHORT*N with `[0x8729e8] = 65535.0f`, DEC3N with
  `[0x874f3c] = 511.0f`); `src` advances 4, `dst` by `stride`.
- Confidence: the per-case encodings were checked for FLOAT1/FLOAT2 (0x7531cb..0x753255) and by constant usage only for
  the others. UNCLEAR (0x75334c..0x753a42): exact operand order inside the normalised cases.
- librw: absent.

## 0x754b40 _rpD3D9VertexDeclarationInstIndices  (catalogue: specified/REA, port level 0)
- `u32 (u32 type, u8* dst, const u32* src /*4 bone bytes per vertex*/, i32 n, u32 stride)`; `type - 4 > 3` -> size only
  (0x754b4e); table 0x754c64. Per vertex (`n <= 0` -> nothing):
  - 4 D3DCOLOR: `dst = {src[2], src[1], src[0], src[3]}` (byte swizzle, 0x754b76..0x754b8d).
  - 5 UBYTE4: the 4 bytes copied in order (0x754bba..0x754bd1).
  - 6 SHORT2: `u16 dst[0..1] = src[0..1]` zero-extended (0x754bf4..0x754c00).
  - 7 SHORT4: `u16 dst[0..3] = src[0..3]` (0x754c2d..0x754c4b).
  Returns `[0x874ef8 + 4*type]`.
- librw: absent.

## 0x7551f0 _rpD3D9VertexDeclarationUnInstV3d  (catalogue: specified/REA, port level 0)
- `void (u32 type, RwV3d* dst, const u8* src)` (one element; table 0x755598): 0 FLOAT1 -> `(x, 0, 0)`; 1 FLOAT2 -> `(x, y, 0)`;
  2/3 -> `(x, y, z)` dword copies; 4 D3DCOLOR -> `(src[2], src[1], src[0]) * (1/255) [0x859a3c]` (0x755260..0x7552a2); 5
  UBYTE4 -> `(src[0], src[1], src[2])` as floats; 6 SHORT2 -> `((i16)s[0], (i16)s[1], 0)`; 7 SHORT4 -> three i16; 8 UBYTE4N ->
  `src[0..2] * (1/255)`; 9 SHORT2N -> `(i16)*[0x858c7c] = 3.051851e-05` for x, y, z = 0; 0xa SHORT4N -> three; 0xb USHORT2N /
  0xc USHORT4N -> `movsx` (SIGNED, values >= 0x8000 decode negative; kept) times `[0x874f48] = 1.5259022e-05`; 0xd UDEC3 ->
  `(v & 0x3ff, (v >> 10) & 0x3ff, (v >> 20) & 0x3ff)` via 64-bit `fild` (0x7554af..0x7554f6); 0xe DEC3N -> the three 10-bit
  fields sign-extended (bit 9) times `[0x874f40] = 0.0019569471` (1/511) (0x755505..0x75557e); other types -> `(0, 0, 0)`
  (0x755589..).
- librw: pipeline.cpp:uninstV3d supports only FLOAT3 and its own compressed format (differs).

## 0x7555e0 _rpD3D9VertexDeclarationUnInstV2d  (catalogue: specified/REA, port level 0)
- `void (u32 type, RwV2d* dst, const u8* src)`; table 0x7557f4: 0 -> `(x, 0)`; 1/2/3 -> `(x, y)`; 4 D3DCOLOR -> `(src[2],
  src[1]) * (1/255)` (0x755624..0x755654); 5 UBYTE4 -> `(src[0], src[1])`; 6/7 -> `((i16)s[0], (i16)s[1])`; 8 UBYTE4N ->
  `(src[0], src[1]) * (1/255)`; 9/0xa -> `(i16) * [0x858c7c]`; 0xb/0xc -> signed `movsx` * `[0x874f48]` (kept); 0xd UDEC3 ->
  `(v & 0x3ff, (v >> 10) & 0x3ff)`; 0xe DEC3N -> sign-extended fields * `[0x874f40]` (0x755788..0x7557dd); other -> `(0, 0)`.
- librw: pipeline.cpp:uninstTexCoords differs (FLOAT2 only).

## 0x755830 _rpD3D9GetMinMaxValuesV3d  (catalogue: specified/REA, port level 0)
- `void (const RwV3d* v, i32 n, RwV3d* min, RwV3d* max)`: `min = max = v[0]`; for `i = 1 .. n-1` (do-while on `n - 1`, entry
  `n <= 1` skip, 0x755861) per component k: if `!(min.k <= v.k)` (C0|C3 clear, i.e. `min > v`, NaN excluded) `min.k = v.k`
  (0x75586e..0x75587d); else if `max.k < v.k` (exactly C0, 0x755881..0x75588e) `max.k = v.k`. A value that lowers min is
  never tested against max.
- librw: absent.

## 0x755980 _rpD3D9FindFormatV3d  (catalogue: specified/REA, port level 1)
- `u32 (const RwV3d* v, i32 n)`: min/max via 0x755830 (0x755997). Default result 2 (FLOAT3). If any `max.k > 1.0` (`test
  ah,0x41; jp` -> exactly... C0|C3 both clear means greater) -> 2 (0x7559a0..0x7559dd). If all `min.k >= 0.0` (C0 clear,
  0x7559e3..0x755a1a): `[0xc9bfec]` (D3DCAPS9.DeclTypes low byte) sign bit (DEC3N 0x80) -> 0xe; else bit 0x20 (USHORT4N) -> 0xc;
  else 2. Else if all `min.k >= -1.0` `[0x858c1c] = -1.0f` (0x755a3f..0x755a76): DEC3N cap -> 0xe; else bit 8 (SHORT4N) -> 0xa;
  else 2.
- librw: absent.

