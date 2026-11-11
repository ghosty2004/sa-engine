# RenderWare 3.6 RtQuat (quaternion toolkit) - full specs, writer C (RE-SPEC-RW-2)

Statically linked RW 3.6 toolkit code in gta_sa.exe 1.0 US; gta-reversed and plugin-sdk only call it. All functions are
`__cdecl`, all float math is x87 (no SSE); values loaded from f32 memory, intermediates kept in x87 registers unless an
f32 store is named. Under the game's 24-bit precision control every x87 result rounds to an f32 significand.

## Layouts and constants used

`RtQuat` (0x10): +0x0 imag.x, +0x4 imag.y, +0x8 imag.z, +0xc real (w). `RwMatrix`: right (+0x0, +0x4, +0x8), up (+0x10,
+0x14, +0x18), at (+0x20, +0x24, +0x28), pos (+0x30). `RtQuatSlerpCache` (0x28): +0x0 raFrom (RtQuat), +0x10 raTo (RtQuat),
+0x20 `f32 omega`, +0x24 `RwBool nearlyZeroOm` (stores 0x7ec2ef..0x7ec548).

Constants (.rdata): `[0x858624] = 1.0f`, `[0x858b50] = 0.0f`, `[0x858b8c] = 0.5f`, `[0x858c1c] = -1.0f`,
`[0x8631d4] = 0.008726646f` (pi/360), `[0x871258] = 0.99999f` (0x3f7fff58), `[0x85f0a0] = 0.16666667f`,
acos polynomial: `[0x874e90] = 3.4793309e-05f`, `[0x874e8c] = 0.00079153501f`, `[0x874e88] = 0.040055536f`,
`[0x874e84] = 0.20121253f`, `[0x874e80] = 0.32556581f`, `[0x874e7c] = 0.077038154f`, `[0x874e78] = 0.68828398f`,
`[0x874e74] = 2.0209458f`, `[0x874e70] = 2.4033949f`, `[0x874e6c] = 7.5497894e-08f`, `[0x874e68] = 1.5707963f` (0x3fc90fda),
`[0x874e64] = 3.1415925f` (0x40490fda); sine polynomial: `[0x86d2b0] = 1.5896910e-10f`, `[0x86d2ac] = 2.5050760e-08f`,
`[0x86d2a8] = 2.7557314e-06f`, `[0x86d2a4] = 0.00019841270f`, `[0x864e0c] = 0.0083333338f`.
`_rwSqrt` 0x7edb30: f32 argument on the stack, table-based result in ST0 (see RpAtomic.md); call it via `orig!`.

## 0x7EB5C0 RtQuatConvertFromMatrix  (catalogue: specified/REA, port level 2)
- Convention: `RwBool (RtQuat* q, const RwMatrix* m)`; returns 1, or 0 when q or m is NULL (0x7eb5c9/0x7eb5d1).
- Behaviour: `t = (m.at.z + m.right.x) + m.up.y` (0x7eb5d3..0x7eb5dd). `fcom 0.0[0x858b50]`; if `t > 0` (C0 and C3 clear,
  0x7eb5ed): `s = _rwSqrt((f32)(t + 1.0[0x858624]))` (0x7eb5ef..0x7eb5f9); `q.w = 0.5[0x858b8c] * s` (0x7eb60b);
  `r = 0.5 / s` (fdivr, 0x7eb60e); `q.x = (m.up.z - m.at.y) * r`, `q.y = (m.at.x - m.right.z) * r`, `q.z = (m.right.y -
  m.up.x) * r` (0x7eb614..0x7eb631). Return 1.
  Else (t <= 0 or NaN): if NOT (`m.right.x <= m.up.y`) (0x7eb644): if NOT (`m.right.x <= m.at.z`) -> `_rtQuatFromMatrixX(q, m)`
  (0x7eb665) else `_rtQuatFromMatrixZ`; else if NOT (`m.up.y <= m.at.z`) -> `_rtQuatFromMatrixY` else `_rtQuatFromMatrixZ`
  (0x7eb670..0x7eb68b). "<=" here means C0 or C3 set, so NaN also takes the "<=" branch. Return 1.
- librw: absent (librw has no matrix-to-quaternion helper).

## 0x7EB6A0 _rtQuatFromMatrixX  (catalogue: specified/REA, port level 1)
- `void (RtQuat* q, const RwMatrix* m)`; the f32 argument for _rwSqrt is built in the m argument slot.
- `s = _rwSqrt((f32)((m.right.x - (m.at.z + m.up.y)) + 1.0))` (fld at.z, fadd up.y, fsubr right.x, fadd 1.0, 0x7eb6a5..0x7eb6bc);
  `q.x = 0.5 * s` (0x7eb6d0); `r = 0.5 / s`; `q.w = (m.up.z - m.at.y) * r` (0x7eb6e0); `q.y = (m.up.x + m.right.y) * r` (0x7eb6eb);
  `q.z = (m.at.x + m.right.z) * r` (0x7eb6f7).
- librw: absent.

## 0x7EB700 _rtQuatFromMatrixY  (catalogue: specified/REA, port level 1)
- `s = _rwSqrt((f32)((m.up.y - (m.at.z + m.right.x)) + 1.0))` (0x7eb705..0x7eb71c); `q.y = 0.5 * s` (0x7eb730); `r = 0.5 / s`;
  `q.w = (m.at.x - m.right.z) * r` (0x7eb741); `q.z = (m.at.y + m.up.z) * r` (0x7eb74c); `q.x = (m.up.x + m.right.y) * r` (0x7eb758).
- librw: absent.

## 0x7EB760 _rtQuatFromMatrixZ  (catalogue: specified/REA, port level 1)
- `s = _rwSqrt((f32)((m.at.z - (m.up.y + m.right.x)) + 1.0))` (0x7eb765..0x7eb77c); `q.z = 0.5 * s` (0x7eb790); `r = 0.5 / s`;
  `q.w = (m.right.y - m.up.x) * r` (0x7eb7a1); `q.x = (m.at.x + m.right.z) * r` (0x7eb7ac); `q.y = (m.at.y + m.up.z) * r` (0x7eb7b7).
- librw: absent.

## 0x7EB7C0 RtQuatRotate  (catalogue: specified/REA, port level 2)
- Convention: `RtQuat* (RtQuat* q, const RwV3d* axis, f32 angleDeg, i32 combineOp)`; returns q or NULL. Frame `sub esp,
  0x20`; locals: copy Q of the old quaternion, normalised axis r, cos c; the `angleDeg` arg slot is reused for sin s.
- Errors: q or axis NULL -> `RwErrorSet({0x195, _rwerror(0x80000016)})`, return NULL (0x7eba48..0x7eba6b); combineOp not
  0/1/2 -> `RwErrorSet({0x195, _rwerror(0x80000003, 0x8e21fc)})`, return NULL (0x7eb7f3..0x7eb81b).
- Common: `a = angleDeg * 0.008726646[0x8631d4]` (half angle in radians); `s = fsin(a)`, `c = fcos(a)` (x87 fsin/fcos on the
  same register value). `RwV3dNormalize(dst, axis)` 0x7ed9b0 (its ST0 length result is discarded).
- combineOp 0 REPLACE (0x7eba02): `q.w = c` (0x7eba18); `RwV3dNormalize(&q.imag, axis)` (0x7eba1b); `q.x = s * q.x`,
  `q.y = q.y * s`, `q.z = q.z * s` (0x7eba22..0x7eba40). Return q.
- combineOp 2 POSTCONCAT (0x7eb822): Q = *q copied first (0x7eb833..0x7eb84f); `RwV3dNormalize(&r, axis)` (0x7eb85d);
  `A = s*r.x, B = r.y*s, C = r.z*s` (0x7eb864..0x7eb878); then
  `q.w = Q.w*c - ((Q.y*B + Q.x*A) + Q.z*C)` (0x7eb87c..0x7eb89f);
  `q.x = ((Q.z*B - C*Q.y) + A*Q.w) + Q.x*c` (0x7eb8a2..0x7eb8c2);
  `q.y = ((Q.x*C - Q.z*A) + Q.y*c) + B*Q.w` (0x7eb8c4..0x7eb8e6);
  `q.z = ((Q.y*A - B*Q.x) + C*Q.w) + Q.z*c` (0x7eb8e9..0x7eb907). Return q.
- combineOp 1 PRECONCAT (0x7eb911): same setup; `A, B, C` as above;
  `q.w = c*Q.w - ((B*Q.y + A*Q.x) + C*Q.z)` (0x7eb96b..0x7eb98e);
  `q.x = ((Q.y*C - Q.z*B) + Q.x*c) + A*Q.w` (0x7eb991..0x7eb9b1);
  `q.y = ((Q.z*A - C*Q.x) + Q.y*c) + B*Q.w` (0x7eb9b3..0x7eb9d5);
  `q.z = ((B*Q.x - Q.y*A) + Q.z*c) + C*Q.w` (0x7eb9d8..0x7eb9f8). Return q.
  (Each product is evaluated in the operand order written; sums left to right as written.)
- librw: absent (librw only has Quat::rotation(angle, axis) with different conventions).

## 0x7EBBB0 RtQuatTransformVectors  (catalogue: specified/REA, port level 0)
- Convention: `RwV3d* (RwV3d* out, const RwV3d* in, i32 n, const RtQuat* q)`; returns out. Caller IKChain_c::MoveBonesToTarget.
- Setup: `k = q.w*q.w - ((q.x*q.x + q.y*q.y) + q.z*q.z)` stored f32 in the q argument slot (0x7ebbc1..0x7ebbe6);
  `x2 = q.x + q.x`, `y2 = q.y + q.y`, `z2 = q.z + q.z`, `wx2 = q.w*x2`, `wy2 = q.w*y2`, `wz2 = q.w*z2` (all stored f32,
  0x7ebbf2..0x7ebc29). `n <= 0` -> return out (0x7ebc2d).
- Per vector i (do-while, 0x7ebc40..0x7ebcf8), v = in[i], o = out[i]:
  `d = (v.y*y2 + x2*v.x) + v.z*z2` (kept in ST0);
  `o.x = v.z*wy2 - v.y*wz2`; `o.y = wz2*v.x - v.z*wx2`; `o.z = v.y*wx2 - wy2*v.x` (each rounded to f32 on a stack slot, then
  stored, 0x7ebc5a..0x7ebca7);
  `o.x = d*q.x + o.x`; `o.y = q.y*d + o.y`; `o.z = q.z*d + o.z` (0x7ebca5..0x7ebcc3);
  `o.x = k*v.x + o.x`; `o.y = v.y*k + o.y`; `o.z = v.z*k + o.z` (0x7ebcc8..0x7ebcf4; v re-read from `in`).
- Edge: in-place use (out == in) gives different results because `in` is re-read after `out` was written.
- librw: absent.

## 0x7EC220 RtQuatSetupSlerpCache  (catalogue: specified/REA, port level 1)
- Convention: `void (const RtQuat* from, const RtQuat* to, RtQuatSlerpCache* c)`; the `from` and `to` argument slots are
  reused as f32 scratch. EAX at return = the nearlyZeroOm value (incidental). Caller BoneNode_c::BlendKeyframe.
- Behaviour:
  1. `c.raFrom = *from` (dword copies, 0x7ec22e..0x7ec241). `d = ((from.y*to.y + from.x*to.x) + from.z*to.z) + from.w*to.w`
     (0x7ec248..0x7ec262), stored f32 (`fst`, 0x7ec264).
  2. If `d < 0` (fcom 0.0, `test ah, 5; jp` not taken, 0x7ec277): if `d < -1.0[0x858c1c]` -> `d = 1.0f` (0x7ec286) else
     `d = -d` (0x7ec290..0x7ec296); `c.raTo = -to` written in the order w (+0x1c), x (+0x10), y (+0x14), z (+0x18)
     (0x7ec29a..0x7ec2b8). Else (d >= 0 or NaN): if `d > 1.0` -> `d = 1.0f` (0x7ec2cc); `c.raTo = *to` (0x7ec2d4..0x7ec2ec).
  3. omega (integer tests on the bits of the stored f32 d, `ax = bits & 0x7fffffff`, 0x7ec2ef..0x7ec300):
     - `ax >= 0x3f800000`: `omega = (bits > 0) ? 0.0 : 3.1415927f (0x40490fdb)` (0x7ec30b / 0x7ec317).
     - `ax < 0x3f000000` (|d| < 0.5): if `ax <= 0x23000000` -> `omega = 1.5707964f (0x3fc90fdb)` (0x7ec335). Else with `z = d*d`:
       `P = ((((([0x874e90]*z + [0x874e8c])*z - [0x874e88])*z + [0x874e84])*z - [0x874e80])*z + [0x85f0a0])*z`,
       `Q = ((([0x874e7c]*z - [0x874e78])*z + [0x874e74])*z - [0x874e70])*z + 1.0`;
       `omega = [0x874e68] - (d - ([0x874e6c] - (P/Q)*d))` (0x7ec341..0x7ec3af).
     - `d <= -0.5` (bits negative, 0x7ec3bc): `z = (f32)((d + 1.0) * 0.5)`; `s = _rwSqrt(z)`; P, Q as above with z (read from the
       f32 slot); `omega = [0x874e64] - (((P/Q)*s - [0x874e6c]) + s) * 2` (as `t = r*s; t -= c6c; t += s; t += t; omega = pi_hi - t`,
       0x7ec3c2..0x7ec45b).
     - `d >= 0.5`: `z = (f32)((1.0 - d) * 0.5)`; `s = _rwSqrt(z)`; `df` = bits of (f32)s with the low 12 bits cleared
       (`and 0xfffff000`, 0x7ec49b); P, Q with z; `r = (P/Q)*s`; `cc = (z - df*df) / (df + s)`; `omega = ((r + cc) + df) * 2`
       (0x7ec466..0x7ec523).
     omega is stored as f32 at c+0x20 (0x7ec525).
  4. `c.nearlyZeroOm = (d < 0.99999[0x871258]) ? 0 : 1` (C0 test, NaN gives 0; 0x7ec52a..0x7ec548).
  5. If nearlyZeroOm == 0: `o = c.omega` (f32), `z = o*o`; `p = (((([0x86d2b0]*z - [0x86d2ac])*z + [0x86d2a8])*z - [0x86d2a4])*z
     + [0x864e0c])*z - [0x85f0a0]`; `sinO = o + p*(z*o)`; `inv = 1.0 / sinO` (0x7ec551..0x7ec58e); then multiply, in this order,
     raFrom.w, raFrom.x, raFrom.y, raFrom.z, raTo.w, raTo.x, raTo.y, raTo.z by inv (each stored f32, 0x7ec594..0x7ec5cd).
- librw: absent (librw has a plain slerp in base.cpp without a cache).

## Open questions
- None.
