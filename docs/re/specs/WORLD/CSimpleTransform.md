# CSimpleTransform and the free functions TransformPoint / TransformVectors (WORLD)

The free functions `TransformPoint` (0x54ece0) and `TransformVectors` (0x54ee30) are specified here too,
because they operate on a `CSimpleTransform` (gta-reversed keeps them in common.cpp).

Layout: `CSimpleTransform { CVector m_vPosn (+0x0, +0x4, +0x8); float m_fHeading (+0xc); }` (0x10 bytes).

x87 note for all three: `fsin`/`fcos` are **not** affected by the precision-control field, so `sin` and
`cos` are full 64-bit-significand x87 results of the f32 heading. Precision control (24 bits) applies to
the following `fmul`/`fadd`/`fsub`. Use the `sa_math::x87` primitives for `fsin`/`fcos` (not `f32::sin`).
Where the code stores `sin`/`cos` to an f32 slot first (TransformVectors) they are rounded to f32 there.

## 0x54ece0 TransformPoint  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl void TransformPoint(CVector* out [esp+4], const CSimpleTransform* t
  [esp+8], const CVector* p [esp+0xc])`, plain `ret`.
- Callers: 2 (`CEntity::TransformFromObjectSpace` ref 0x5334f0 and ptr 0x533560). Callees: none.
- Behaviour (exact x87 order; `c`, `s` stay in registers, unrounded):
  1. `c = fcos(t.heading)`; `s = fsin(t.heading)` (heading loaded twice from memory).
  2. `out.x = ((c * p.x) - (s * p.y)) + t.x` (products in that order, `fsubp` = first − second, then
     `fadd t.x`), stored f32.
  3. `out.y = ((c * p.y) + (s * p.x)) + t.y` — note the operand order of the sum: `c*p.y` is ST1,
     `s*p.x` is ST0, `faddp` (commutative, same value), then `fadd t.y`.
  4. `out.z = t.z + p.z` (`fld t.z; fadd p.z`).
- Edge cases: if `out` aliases `p`, `out.x` is written before `p.x` is re-read for `out.y` (step 3 reads
  `p.y` then `p.x` after the store at 0x54ed03) — the original then uses the new `x`. Keep the order.
- gta-reversed: provided (common.cpp:37). Same formula, but `std::cos/std::sin` of a float in float
  precision (differs from x87 `fsin/fcos`, and gr computes `cos` and `sin` once in f32). Port the bytes.
- Open questions: none.

## 0x54ee30 TransformVectors  (catalogue: specified/REA, port level 0)
- Convention / signature: `__cdecl void TransformVectors(CVector* out [esp+4], int32 count [esp+8],
  const CSimpleTransform* t [esp+0xc], const CVector* in [esp+0x10])`, plain `ret`.
- Callers: 1 (`CEntity::GetIsBoundingBoxOnScreen` 0x5345d0). Callees: none.
- Behaviour:
  1. `c = (f32) fcos(t.heading)` stored to a local f32; `s = (f32) fsin(t.heading)` stored to an f32
     slot (the `t` argument slot is reused). Both are rounded to f32 here. Computed even when count == 0.
  2. If `count == 0` return. Otherwise loop `do { ... } while (--count != 0)` (count compared as a
     32-bit value: a negative count loops ~2^32 times; original behaviour).
  3. Per element (in, out advance by 12 bytes):
     `out.x = (c * in.x) - (s * in.y)` (stored f32);
     `out.y = (s * in.x) + (c * in.y)` (re-reads `in.x`, `in.y` from memory after the `out.x` store);
     `out.z = in.z` copied as a raw dword (bit copy).
  **No translation is added**: this rotates vectors only.
- Edge cases: in-place use (`out == in`) makes `out.y` use the already rotated `x` (store then reload).
- gta-reversed: provided (common.cpp:56) but **different**: it calls `TransformPoint` per element,
  which adds `t.m_vPosn` (translation) and computes `z = in.z + t.z`. The original does not translate at
  all and copies `z`. CONFIRMED deviation; port the bytes.
- Open questions: none.

## 0x54ef90 CSimpleTransform::Invert  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall void (this = ECX: CSimpleTransform* out, const CSimpleTransform* src
  [esp+4])`, `ret 4`.
- Callers: 1 (`CEntity::GetIsBoundingBoxOnScreen` 0x5345d0). Callees: none.
- Behaviour (registers, unrounded `c`, `s`):
  1. `c = fcos(src.heading)`; `s = fsin(src.heading)`.
  2. `this.x = (-(c * src.x)) - (s * src.y)` (`fmul; fchs; ...; fsubp`), stored f32.
  3. `this.y = (s * src.x) - (c * src.y)` — `src.x` is re-read from memory **after** step 2's store.
  4. `this.z = -src.z`; `this.heading = -src.heading` (`fld; fchs; fstp`, sign flip only).
- Edge cases: with `this == src`, step 3 uses the new `x` (store-then-load order at 0x54efab/0x54efad).
- gta-reversed: provided (SimpleTransform.cpp:27), same formulas, but `cosf/sinf` in f32 instead of
  x87 `fcos/fsin` and no aliasing order guarantee. Port the bytes.
- Open questions: none.
