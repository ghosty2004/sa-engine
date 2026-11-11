# RenderWare 3.6 matrix / vector math (RW core) - full specs, writer A (RE-SPEC-RW)

RenderWare 3.6 code linked into gta_sa.exe 1.0 US; gta-reversed/plugin-sdk have thunks only. All `__cdecl`.

## Floating point model (applies to every function here)
- The game runs with x87 precision control = 24 bits (RW forces it in `_rwProcessorInitialize` 0x857404 at
  FINALIZESTART and again every camera begin-update at 0x857432, see docs/re/rw/README.md). Under PC=24 every x87
  arithmetic result is rounded to a 24-bit mantissa, but intermediate values that stay on the x87 stack keep the
  extended exponent range (no overflow/underflow at f32 limits) until they are stored with `fstp dword`. Port with
  `sa_math::x87` and keep the "stays on the stack" vs "stored to f32" distinction written below.
- Comparisons: `fcomp; fnstsw; test ah,0x41; jp L` jumps when ST0 > operand OR unordered; `and eax,0x4100; jne L`
  jumps when ST0 <= operand OR unordered; `and eax,0x100; jne L` jumps when ST0 < operand OR unordered;
  `test ah,5; jp L` jumps when ST0 >= operand OR unordered.
- Float return values are in ST0 (`fld dword [..]` before `ret` = f32 value; otherwise an unstored x87 value).
- The SSE2 kernels use MXCSR as set by RW (FTZ on, see README), plain f32 SSE arithmetic.

## Layouts
`RwV3d` {x +0, y +4, z +8}. `RwMatrix` (0x40): right +0x0, flags +0xC, up +0x10, pad +0x1C, at +0x20, pad +0x2C,
pos +0x30, pad +0x3C (proof: RwMatrixCreate 0x7F2A78..0x7F2A9B writes 1.0f to +0x0/+0x14/+0x28 and flags +0xC).
Flags: 0x1 rwMATRIXTYPENORMAL, 0x2 rwMATRIXTYPEORTHOGONAL (3 = orthonormal), 0x20000 rwMATRIXINTERNALIDENTITY.
`RwBBox` {sup +0x0, inf +0xC} (0x808F6A/0x808F84). Combine ops: 0 rwCOMBINEREPLACE, 1 PRECONCAT, 2 POSTCONCAT.

VECTORMODULE (engine plugin 0x401, 0x18 bytes), offset global 0xC97934 (0x7EDE9A), module `V = G + [0xC97934]`:
+0x0 sqrt table (4096 u32), +0x4 inverse-sqrt table (4096 u32) (0x7EDFC1, 0x7EE098), +0x8 `transformPoint(out, in,
matrix)`, +0xC `transformPoints(out, in, n, matrix)`, +0x10 `transformVector`, +0x14 `transformVectors`
(0x7EDEA2..0x7EDEE6). Instance counter 0xC97938.

MATRIXMODULE (engine plugin 0x402, 0x18 bytes), offset global 0xC979BC, module `M = G + [0xC979BC]`: +0x0 matrix
freelist, +0x4 identity-flag mask = 0x20000 (0x7F1716), +0x8 multiply function `void (dst, a, b)` (default
`_rwMatrixMultiplyDefault` 0x7F12F0, writes the 12 floats only, never flags/pads), +0xC RwMatrixTolerance
{normal, orthogonal, identity} = 0.01f each (0x7F1729..0x7F1769). Instance counter 0xC979C0.

Table lookup (used by _rwSqrt, _rwInvSqrt, RwV3dLength, RwV3dNormalize): `b = bits(x)` (u32 of the f32 argument);
if `b == 0` (only +0.0) the result is +0.0. Otherwise `b' = b + 0x800`, `idx = (b' >> 12) & 0xFFF`,
sqrt: `result_bits = sqrtTable[idx] + ((b' >> 1) & 0x3FC00000)`;
invsqrt: `result_bits = invTable[idx] + ((~b' >> 1) & 0x3FC00000)`. No sign/NaN handling.

## 0x7ED5F0 _rwVectorSetMultFn  (catalogue: specified/REA, port level 0)
- `RwBool (pointFn, pointsFn, vectorFn, vectorsFn)`: each NULL argument is replaced by the default
  (`_rwV3dTransformPointDefault` 0x7ED730, `...PointsDefault` 0x7ED670, `...VectorDefault` 0x7ED880,
  `...VectorsDefault` 0x7ED7D0); stores V+0x8, +0xC, +0x10, +0x14 in that order; returns 1.
  Caller: D3D9System 0x7F5F70 (installs the SSE2 kernels below when the CPU has SSE2).

## 0x7ED9B0 RwV3dNormalize  (catalogue: specified/REA, port level 1)
- `RwReal (RwV3d* out, const RwV3d* in)`: returns the length (f32 in ST0). Callers (19): Fx_c::CreateMatFromVec, ...
- Behaviour:
  1. `s = (in.x*in.x + in.y*in.y) + in.z*in.z` on the x87 stack (load order y, x, z; 0x7ED9C4..0x7ED9DA).
  2. `len = sqrtLookup(f32(s))` (stored to a local, 0x7ED9E2..0x7EDA1F).
  3. `r = invSqrtLookup(f32(s))` (the f32 of s is stored over the `in` argument slot, 0x7EDA23).
  4. `out.x = f32(r * in.x)`, then `out.y = f32(in.y * r)`, then `out.z = f32(in.z * r)` (x written before y/z are read;
     out == in is fine).
  5. If NOT `len > [0x858B50] = 0.0f` (i.e. len <= 0 or NaN, 0x7EDA95): RwErrorSet({1, _rwerror(0x19 E_RW_ZEROLENGTH)}).
  6. Return len.
- Zero vector: len = 0, r = 0 -> out = 0 * in (0 for finite input), error set.
- librw: rwbase.h:normalize differs (exact sqrtf and division, no error, returns the vector).

## 0x7EDAC0 RwV3dLength  (catalogue: specified/REA, port level 0)
- `RwReal (const RwV3d* v)`: `s = (x*x + y*y) + z*z` (same order as above), stored to f32; return `sqrtLookup(s)` as f32 in ST0.
- librw: rwbase.h:length differs (sqrtf).

## 0x7EDB30 _rwSqrt  (catalogue: specified/REA, port level 0)
- `RwReal (RwReal x)` (x passed as a dword on the stack): sqrt table lookup above; result f32 in ST0.
- librw: absent (sqrtf).

## 0x7EDB90 _rwInvSqrt  (catalogue: specified/REA, port level 0)
- `RwReal (RwReal x)`: inverse-sqrt table lookup above (x = +0.0 gives 0.0, not inf); f32 in ST0.
- librw: absent.

## 0x7EDD60 RwV3dTransformPoint  (catalogue: specified/REA, port level 0)
- `RwV3d* (RwV3d* out, const RwV3d* in, const RwMatrix* m)`: calls `V+0x8(out, in, m)`; returns out.

## 0x7EDD90 RwV3dTransformPoints  (catalogue: specified/REA, port level 0)
- `RwV3d* (RwV3d* out, const RwV3d* in, RwInt32 n, const RwMatrix* m)`: calls `V+0xC(out, in, n, m)`; returns out.

## 0x7EDDC0 RwV3dTransformVector  (catalogue: specified/REA, port level 0)
- `RwV3d* (out, in, m)`: calls `V+0x10(out, in, m)`; returns out.

## 0x7EDDF0 RwV3dTransformVectors  (catalogue: specified/REA, port level 0)
- `RwV3d* (out, in, n, m)`: calls `V+0x14(out, in, n, m)`; returns out.

## 0x7EDE90 cb_VECTORMODULE_RwEngine_Open  (catalogue: specified/REA, port level 1)
- `void* Open(instance, offset, size)`; returns instance or NULL.
- Behaviour:
  1. `[0xC97934] = offset`; V+0x8..+0x14 = the four default transform functions (as in _rwVectorSetMultFn).
  2. `t = G->rwmalloc(0x4000, 0x40401)`; NULL -> RwErrorSet({1, _rwerror(0x80000013, 0x4000)}); return NULL.
  3. Sqrt table: `x` starts as f32 bits 0x3F800000 (1.0). For i = 0..0x7FF: `t[0x800 + i] = bits(f32(fsqrt(x)))
     - 0x1FC00000`; `bits(x) += 0x1000`. Then (x continues from 2.0) for i = 0..0x7FF: `t[i] = bits(f32(fsqrt(x)))
     - 0x20000000`; `bits(x) += 0x1000` (0x7EDF39..0x7EDFA9). `V+0x0 = t`.
  4. `u = G->rwmalloc(0x4000, 0x40401)`; NULL -> same NOMEM error (t stays in V+0x0); return NULL.
  5. Inverse table: x restarts at 1.0; i = 0..0x7FF: `u[0x800 + i] = bits(f32(1.0 / fsqrt(x))) - 0x20000000`; then
     x from 2.0: `u[i] = bits(f32(1.0 / fsqrt(x))) - 0x1FC00000`. Division `fdivr qword [0x85A310] = 1.0` (f64 constant,
     0x7EE014). `V+0x4 = u`.
  6. `[0xC97938]++`; return instance.
- Precision: this Open runs inside RwEngineStart after the device START request and BEFORE FINALIZESTART forces
  PC=24 (RwEngine.md). fsqrt alone is correctly rounded to f32 in any precision, so the sqrt table is precision
  independent; the inverse table (sqrt then divide) can differ in the last bit between PC=24 and PC=53/64. See open questions.
- librw: absent (no tables).

## 0x7F12C0 _rwMatrixSetMultFn  (catalogue: specified/REA, port level 0)
- `RwBool (fn)`: `M+0x8 = fn ? fn : 0x7F12F0`; returns 1. Caller: D3D9System.

## 0x7F1450 _rwMatrixDeterminant  (catalogue: specified/REA, port level 0)
- `RwReal (const RwMatrix* m)`, result on the x87 stack (not stored):
  `c0 = at.z*up.y - at.y*up.z; c1 = up.z*at.x - at.z*up.x; c2 = at.y*up.x - up.y*at.x;`
  `det = (right.y*c1 + right.x*c0) + right.z*c2` (0x7F1454..0x7F148E).
- librw: absent.

## 0x7F14A0 _rwMatrixOrthogonalError  (catalogue: specified/REA, port level 0)
- `RwReal (const RwMatrix* m)`, unstored x87 result:
  `dUA = (at.y*up.y + at.x*up.x) + at.z*up.z; dAR = (at.y*right.y + at.x*right.x) + at.z*right.z;
  dUR = (up.y*right.y + up.x*right.x) + up.z*right.z; return (dAR*dAR + dUA*dUA) + dUR*dUR`.
- librw: base.cpp:Matrix::orthogonalError matches mathematically (different summation order).

## 0x7F1500 _rwMatrixNormalError  (catalogue: specified/REA, port level 0)
- `RwReal (const RwMatrix* m)`, unstored x87 result, constants `[0x858624] = 1.0f`:
  `eR = ((right.y^2 + right.x^2) + right.z^2) - 1; eU = ((up.x^2 + up.y^2) + up.z^2) - 1;
  eA = ((at.x^2 + at.y^2) + at.z^2) - 1; return (eU*eU + eR*eR) + eA*eA` (0x7F1572..0x7F1580).
- librw: base.cpp:Matrix::normalError matches mathematically (order differs).

## 0x7F16C0 cb_MATRIXMODULE_RwEngine_Open  (catalogue: specified/REA, port level 2)
- `void* Open(instance, offset, size)`.
- Behaviour: `[0xC979BC] = offset`; `M+0 = RwFreeListCreateAndPreallocateSpace(0x40, [0x8E2290] (init 0x100), 4,
  [0x8E2294] (init 1), (void*)0xC97998, 0x4000D)`; NULL -> return NULL. `M+4 = 0x20000`; `M+8 = 0x7F12F0`;
  `M+0xC/+0x10/+0x14 = 0x3C23D70A (0.01f)`; `[0xC979C0]++`; return instance.

## 0x7F17E0 RwMatrixOptimize  (catalogue: specified/REA, port level 1)
- `RwMatrix* (RwMatrix* m, const RwMatrixTolerance* tol)`; tol NULL -> `&M+0xC`. Callers: RpWorldAddClump,
  `_rwCameraFrameSyncCB` 0x7EE5A0, 0x7EF1F0.
- Behaviour: `normal = !(NormalError(m) > tol->normal || unordered)` (0x7F180C); `orth = !(OrthogonalError(m) >
  tol->orthogonal || unordered)`; `ident = normal && orth && IdentityError(m) <= tol->identity` (`_rwMatrixIdentityError`
  0x7F1590, only evaluated when both are true). Flags: bit 0x1 = normal, bit 0x2 = orth, bit 0x20000 = ident (each
  set or cleared, other bits kept). Return m.
- librw: base.cpp:Matrix::optimize matches.

## 0x7F18A0 RwMatrixUpdate  (catalogue: specified/REA, port level 0)
- `RwMatrix* (RwMatrix* m)`: `m->flags &= 0xFFFDFFFC`; returns m.

## 0x7F18B0 RwMatrixMultiply  (catalogue: specified/REA, port level 0)
- `RwMatrix* (RwMatrix* dst, const RwMatrix* a, const RwMatrix* b)`; returns dst. Callers (25) incl. the frame sync.
- Behaviour: `mask = M+4 & 0x20000`. If `a->flags & mask`: copy all 16 dwords of b to dst. Else if `b->flags & mask`:
  copy a to dst. Else `M+8(dst, a, b)` then `dst->flags = a->flags & b->flags` (flags read BEFORE the multiply,
  0x7F18C2/0x7F18CF). dst must not alias a or b in the general case (default kernel writes while reading).
- librw: base.cpp:Matrix::mult matches.

## 0x7F1920 RwMatrixOrthoNormalize  (catalogue: specified/REA, port level 2)
- `jmp 0x7F1930`; `RwMatrix* (RwMatrix* dst, const RwMatrix* src)`.

## 0x7F1930 _rwMatrixOrthoNormalizeInternal  (catalogue: unknown/REA, port level 1)
- Convention: same arguments as 0x7F1920 (it is its jump target); returns dst. Works on a local copy (R, U, A, P)
  of src, so dst == src is allowed (RwFrameOrthoNormalize does that).
- Behaviour:
  1. Copy src right/up/at/pos to locals R, U, A, P.
  2. `rinv = _rwInvSqrt(f32((R.y*R.y + R.x*R.x) + R.z*R.z))` (0x7F194D..0x7F19B2); `R = (f32(rinv*R.x), f32(rinv*R.y),
     f32(rinv*R.z))`. rinv is kept as an f32 local.
  3. `uinv = _rwInvSqrt(f32((U.y*U.y + U.x*U.x) + U.z*U.z))` (0x7F19DB..0x7F19FA); U scaled the same way; uinv kept (f32).
  4. `ainv = _rwInvSqrt(f32((A.y*A.y + A.x*A.x) + A.z*A.z))` (0x7F1A23..0x7F1A42); A scaled; ainv stays in ST0 (it is the
     f32 value loaded by _rwInvSqrt). Every length sum in this function groups as `(y*y + x*x) + z*z` except the two in
     steps 6/7, which group as `(x*x + y*y) + z*z`.
  5. Choose (X, Y, Z) = the pair of kept axes X, Y and the axis Z to rebuild:
     - if NOT `rinv > [0x858B50] = 0.0f` (0x7F1A79): rebuild R: X = U, Y = A, Z = R.
     - else if NOT `uinv > 0`: X = A, Y = R, Z = U.
     - else if NOT `ainv > 0`: X = R, Y = U, Z = A.
     - else compute (ESP-relative slots verified: R +0x10, U +0x1C, A +0x28) `dUA = |f32((A.y*U.y + A.x*U.x) + A.z*U.z)|`, `dRA = |f32((A.y*R.y + A.x*R.x) + A.z*R.z)|` (both
       stored to f32 then negated if < 0) and `dRU = |(U.y*R.y + U.x*R.x) + U.z*R.z|` (NOT stored, stays on the stack);
       if `dUA >= dRA` (or unordered): rebuild A if `dRA >= dRU` (or unordered) else rebuild U (X = A, Y = R, Z = U);
       else (dUA < dRA): rebuild R if `dUA < dRU` else rebuild A. (Rebuild A = X R, Y U, Z A; rebuild R = X U, Y A, Z R.)
       The rebuilt axis is the one shared by the two largest |dot products|.
  6. `Z = X x Y`: `Z.x = f32(Y.z*X.y - Y.y*X.z)`, `Z.y = f32(X.z*Y.x - Y.z*X.x)`, `Z.z = Y.y*X.x - X.y*Y.x` (stored f32
     but the unstored value is kept); `zinv = invSqrt(f32((Z.x^2 + Z.y^2) + Zz_unstored^2))`; `Z.x = f32(zinv*Z.x)`;
     `Z.y' = zinv*Z.y` and `Z.z' = Z.z*zinv` stored as f32 but the unstored values stay on the stack.
  7. `Y = Z x X`: `Y.x = f32(X.z*Zy' - X.y*Zz')` using the UNSTORED Zy', Zz' (0x7F1C13..0x7F1C1F);
     `Y.y = f32(Z.z*X.x - X.z*Z.x)`, `Y.z = Y.z_unstored = X.y*Z.x - Z.y*X.x` (f32 memory operands);
     `yinv = invSqrt(f32((Y.x^2 + Y.y^2) + Yz_unstored^2))`; `Y.x = f32(yinv*Y.x)`, `Y.y = f32(Y.y*yinv)`, `Y.z = f32(f32(Y.z)*yinv)`.
     X itself is not renormalised again.
  8. dst.right/up/at = locals R/U/A (whichever were rebuilt), dst.pos = P, `dst->flags = (dst->flags & ~0x20000) | 3`
     (0x7F1CE5..0x7F1CF1). dst pads untouched.
- librw: absent (no orthonormalize in librw).

## 0x7F1D00 RwMatrixRotateOneMinusCosineSine  (catalogue: specified/REA, port level 1)
- `RwMatrix* (RwMatrix* m, const RwV3d* unitAxis, RwReal oneMinusCos, RwReal sin, RwOpCombineType op)`; returns m or NULL.
- Rotation matrix built in a local (x87 values, `f32()` = stored and reloaded):
  `tx = f32((1 - ax*ax) * omc)`, `ty = (1 - ay*ay) * omc` (unstored), `tz = f32(1 - az*az) * omc` (unstored);
  `Pxy = f32(ay*ax) * omc` (unstored), `Pyz = f32((ay*az) * omc)`, `Pzx = (az*ax) * omc` (unstored);
  `sx = f32(sin*ax)`, `sy = f32(ay*sin)`, `sz = az*sin` (unstored). 1.0 is `[0x858624] = 1.0f`.
  right = (f32(1 - tx), f32(sz + Pxy), f32(Pzx - sy)); up = (f32(Pxy - sz), f32(1 - ty), f32(sx + Pyz));
  at = (f32(sy + Pzx), f32(Pyz - sx), f32(1 - tz)); pos = (0,0,0); flags = 3. The three pad dwords of the local are
  never written (stack garbage, copied into m by REPLACE).
- Combine:
  - 0 REPLACE: copy the local (16 dwords) to m; return m.
  - 1 PRECONCAT: `mask = M+4 & 0x20000`; if `m->flags & mask`: m = local (via a temp copy). Else `M+8(tmp, local, m)`,
    `tmp.flags = m->flags & 3`, m = tmp (16 dwords, pads from tmp = garbage). The `test al,3` at 0x7F1F27 tests the low
    byte of `mask` (always 0) - the branch it guards is dead.
  - 2 POSTCONCAT: if `m->flags & mask`: m = local. Else (`test al,3` at 0x7F1ED6 is again always false) `M+8(tmp, m,
    local)`, `tmp.flags = m->flags & 3`, m = tmp.
  - other: RwErrorSet({1, _rwerror(0x80000003 E_RW_BADPARAM, (char*)0x8E21FC "Invalid combination type")}); return NULL.
- librw: base.cpp:Matrix::makeRotation differs (computes c + x*x*t, librw sets pads 0, uses Matrix::mult with
  identity shortcut on both operands).

## 0x7F1FD0 RwMatrixRotate  (catalogue: specified/REA, port level 2)
- `RwMatrix* (RwMatrix* m, const RwV3d* axis, RwReal angle, RwOpCombineType op)`; returns m (always, even if the inner
  call fails). Callers (13): CVehicleModelInfo::SetRenderPipelines, Interior_c::CalcMatrix, ...
- Behaviour: `rad = f32(angle * [0x8595EC] = 0.017453292f)` (stored over the angle slot, 0x7F1FE2);
  `inv = _rwInvSqrt(f32((axis.x^2 + axis.y^2) + axis.z^2))` (load order y, x, z, sum `(x*x + y*y) + z*z`);
  `u = (f32(inv*axis.x), f32(axis.y*inv), f32(axis.z*inv))`; `s = f32(fsin(rad))`; `omc = f32(1.0 - fcos(rad))`
  (`fsubr [0x858624]`); `RwMatrixRotateOneMinusCosineSine(m, &u, omc, s, op)`. fsin/fcos are the x87 instructions
  (port through sa_math::x87, not libm).
- Zero axis: inv = 0 -> u = 0 -> rotation = identity-like matrix (right = (1-omc*1...)); not special-cased.
- librw: base.cpp:Matrix::rotate differs (exact 1/sqrtf, sinf/cosf, angle*M_PI/180).

## 0x7F2070 RwMatrixInvert  (catalogue: specified/REA, port level 1)
- `RwMatrix* (RwMatrix* dst, const RwMatrix* src)`; returns dst. dst must not alias src (the transpose writes dst
  before all of src is read). Callers (17): CPedModelInfo::CreateHitColModelSkinned, CShadows::CastShadowEntityXYZ, ...
- Behaviour:
  1. If `src->flags & M+4 & 0x20000`: copy 16 dwords src -> dst; return dst.
  2. Else if `(src->flags & 3) == 3` (low byte compare, 0x7F20A6): dst 3x3 = transpose(src) (dst.right = (src.right.x,
     src.up.x, src.at.x), dst.up = (src.right.y, src.up.y, src.at.y), dst.at = (src.right.z, src.up.z, src.at.z));
     `dst.pos.x = -((p.y*r.y + p.x*r.x) + r.z*p.z)`, `dst.pos.y = -((p.x*u.x + p.y*u.y) + p.z*u.z)`,
     `dst.pos.z = -((a.x*p.x + p.y*a.y) + a.z*p.z)` (r/u/a/p = src rows); `dst->flags = 3`; return dst.
  3. Else `_rwMatrixInvertOrthoNormalizedOrGeneral(dst, src)` (0x7F2160); return dst.
- librw: base.cpp:Matrix::invert matches (invertOrthonormal identical structure).

## 0x7F2160 _rwMatrixInvertOrthoNormalizedOrGeneral  (catalogue: unknown/REA, port level 0)
- Evidence: general 3x4 inverse by cofactors (name from the assignment; it is the general path, RW 3.6
  `_rwMatrixInvertGeneral` would be a better name). `void (RwMatrix* dst, const RwMatrix* src)`, EAX = dst.
  Callers: RwMatrixInvert, RwMatrixQueryRotate 0x7F2720. dst must not alias src.
- Behaviour (s = src, d = dst; r/u/a/p rows):
  1. `d.r.x = f32(s.a.z*s.u.y - s.a.y*s.u.z)`; `d.r.y = f32(-(s.a.z*s.r.y - s.a.y*s.r.z))`;
     `c = s.u.z*s.r.y - s.u.y*s.r.z` (unstored), `d.r.z = f32(c)`.
  2. `det = f32((s.u.x*d.r.y + s.a.x*c) + d.r.x*s.r.x)` (0x7F219C..0x7F21AF; d.r.x/d.r.y are the stored f32 values,
     c unstored); `inv = 1.0` (`[0x858624] = 1.0f`); if `bits(det) != 0` then `inv = 1.0 / det` (unstored; -0.0 divides).
  3. `rx = inv*d.r.x` (kept unstored), `d.r.x = f32(rx)`; `d.r.y = f32(inv*d.r.y)` (also kept as f32 copy `ry`);
     `d.r.z = f32(inv*d.r.z)`.
  4. `d.u.x = f32(-((s.a.z*s.u.x - s.a.x*s.u.z)*inv))`; `d.u.y = f32((s.a.z*s.r.x - s.a.x*s.r.z)*inv)`;
     `d.u.z = f32(-((s.u.z*s.r.x - s.u.x*s.r.z)*inv))`; `d.a.x = f32((s.a.y*s.u.x - s.a.x*s.u.y)*inv)`;
     `d.a.y = f32(-((s.a.y*s.r.x - s.a.x*s.r.y)*inv))`; `d.a.z = f32((s.u.y*s.r.x - s.u.x*s.r.y)*inv)`.
  5. `d.p.x = f32(-((d.a.x*s.p.z + s.p.y*d.u.x) + s.p.x*rx))` (rx unstored); `d.p.y = f32(-((d.a.y*s.p.z + s.p.y*d.u.y)
     + s.p.x*ry))`; `d.flags = 0`; `d.p.z = f32(-((s.p.y*d.u.z + s.p.x*d.r.z) + d.a.z*s.p.z))`.
- Singular matrix: det = +0 -> inv = 1.0, result is the unscaled adjugate.
- librw: base.cpp:Matrix::invertGeneral differs (flags &= ~IDENTITY instead of 0; pos sums ordered differently).

## 0x7F22C0 RwMatrixScale  (catalogue: specified/REA, port level 1)
- `RwMatrix* (RwMatrix* m, const RwV3d* s, RwOpCombineType op)`; returns m. Callers (5): CCam::Process_1rstPersonPedOnPC, ...
- 0 REPLACE: m = identity (1.0f diagonal, others 0, pos 0), `flags |= 0x20003`, then `right.x = s.x, up.y = s.y,
  at.z = s.z` (raw dword copies), `flags &= 0xFFFDFFFC` (net: old flags with bits 0x20003 cleared).
- 1 PRECONCAT: right *= s.x, up *= s.y, at *= s.z (each component `f32(c * s.k)`); pos unchanged.
- 2 POSTCONCAT: every row (right, up, at, pos) scaled per column: `row.x *= s.x, row.y *= s.y, row.z *= s.z`.
- Both concat cases end with `flags &= 0xFFFDFFFC`.
- Other op: RwErrorSet({1, _rwerror(0x80000003, 0x8E21FC)}), then the compiler-shared tail executes
  `xor eax,eax; mov ecx,[eax+0xC]` (0x7F2301..0x7F2306): a read of address 0xC -> access violation. Original crash.
- librw: base.cpp:Matrix::scale differs (via mult; REPLACE sets flags = 3 exactly; bad op ignored).

## 0x7F2450 RwMatrixTranslate  (catalogue: specified/REA, port level 1)
- `RwMatrix* (RwMatrix* m, const RwV3d* t, RwOpCombineType op)`; returns m. Callers (10).
- 0 REPLACE: identity, `flags |= 0x20003`, `pos = *t` (dword copies), `flags &= 0xFFFDFFFF` (net: old | 3, minus 0x20000).
- 1 PRECONCAT: `pos.x = f32(((at.x*t.z + up.x*t.y) + right.x*t.x) + pos.x)`, same for y (column .y) and z (column .z).
- 2 POSTCONCAT: `pos.x = f32(pos.x + t.x)`, `pos.y = f32(t.y + pos.y)`, `pos.z = f32(t.z + pos.z)`.
- Concat cases end with `flags &= 0xFFFDFFFF`.
- Other op: E_RW_BADPARAM error then the same NULL dereference crash as RwMatrixScale (0x7F248D..0x7F2492).
- librw: base.cpp:Matrix::translate differs (via mult; REPLACE flags = 3).

## 0x7F25A0 RwMatrixTransform  (catalogue: specified/REA, port level 1)
- `RwMatrix* (RwMatrix* m, const RwMatrix* t, RwOpCombineType op)`; returns m, NULL on bad op. Callers (10).
- `mask = M+4 & 0x20000`.
- 0 REPLACE: m = t (16 dwords).
- 1 PRECONCAT: if `t->flags & mask`: m unchanged (copied through a temp); else if `m->flags & mask`: m = t; else
  `M+8(tmp, t, m)`, `tmp.flags = m->flags & t->flags`, m = tmp.
- 2 POSTCONCAT: if `m->flags & mask`: m = t; else if `t->flags & mask`: unchanged; else `M+8(tmp, m, t)`,
  `tmp.flags = t->flags & m->flags`, m = tmp.
- Other: RwErrorSet({1, _rwerror(0x80000003, 0x8E21FC)}); return NULL (no crash here).
- librw: base.cpp:Matrix::transform matches.

## 0x7F2A20 RwMatrixDestroy  (catalogue: specified/REA, port level 0)
- `RwBool (RwMatrix* m)`: `G->memoryFree(M+0, m)`; returns 1.

## 0x7F2A50 RwMatrixCreate  (catalogue: specified/REA, port level 0)
- `RwMatrix* (void)`: `m = G->memoryAlloc(M+0, 0x3000D)`; if non-NULL: identity (pads untouched), flags = 0x20003. Returns m
  (NULL without error on failure).
- librw: base.cpp:Matrix::create matches (librw also zeroes pads).

## 0x808F60 RwBBoxCalculate  (catalogue: specified/REA, port level 0)
- `RwBBox* (RwBBox* box, const RwV3d* verts, RwInt32 n)`; returns box. Callers: RpMorphTargetCalcBoundingSphere, `_rwCameraFrameSyncCB`.
- Behaviour: `box->inf = box->sup = verts[0]` (dword copies, inf first). For the remaining `n - 1` vertices (count kept
  as `n - 1` and decremented to 0; n == 0 therefore loops 2^32-1 times - original): for each axis k (x, y, z):
  if NOT `inf.k <= v.k` (strictly greater, NaN v never replaces) `inf.k = v.k`; then for each axis:
  if `sup.k < v.k` (`test ah,5; jp` skip on >= or unordered) `sup.k = v.k`. The three inf updates happen before the
  three sup updates for each vertex.
- librw: absent (librw BBox::calculate uses its own loop with the same semantics).

## 0x80D440 _rwSSE2V3dTransformPoints  (catalogue: specified/REA, port level 0)
- `RwV3d* (RwV3d* out, const RwV3d* in, RwInt32 n, const RwMatrix* m)` (EBP frame); installed in V+0xC by D3D9System.
- Loads the four matrix rows (`movaps` if m is 16-byte aligned, else `movups`; lane 3 replaced by lane 2, irrelevant).
  Per point (n decremented first-tested at the end: n == 0 loops 2^32 times): per lane
  `o = (z*at + y*up) + (x*right + pos)` in SSE f32 (0x80D4A2..0x80D4B7); writes x, y, z. Returns out.
- librw: absent.

## 0x80D4E0 _rwSSE2V3dTransformVectors  (catalogue: specified/REA, port level 0)
- Same as above without pos: `o = z*at + (y*up + x*right)` (0x80D542/0x80D548). Returns out.

## 0x80D570 _rwSSE2V3dTransformPoint  (catalogue: specified/REA, port level 0)
- `RwV3d* (RwV3d* out, const RwV3d* in, const RwMatrix* m)`: `o = (z*at + y*up) + (x*right + pos)`; returns out.
  Note: all three input components are read before out is written (out == in allowed).

## 0x80D600 _rwSSE2V3dTransformVector  (catalogue: specified/REA, port level 0)
- `RwV3d* (out, in, m)`: `o = z*at + (y*up + x*right)`; returns out.

## Open questions
- 0x7EE00E: the inverse-sqrt tables depend on the x87 precision control at the time VECTORMODULE Open runs (after the
  D3D9 START request, which may already have switched the FPU to 24 bits; FINALIZESTART 0x857404 forces it only
  later). Resolve by dumping both tables from a running original (`G + [0xC97934]` +0/+4) and comparing with both
  computations; the sqrt table is unaffected.
