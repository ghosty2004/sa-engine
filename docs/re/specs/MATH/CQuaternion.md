# CQuaternion (MATH) — RE-SPEC-REST

Layout (gr `Core/Quaternion.h`, confirmed by every access below): x +0x0, y +0x4, z +0x8, w (real) +0xC, all f32.
All these functions are "called from gta-reversed only" in the catalogue: gta-reversed forwards to the original
through `plugin::CallMethod` (Quaternion.cpp:42-98), so there is no gr C++ to compare against.
RwMatrix offsets used: right +0x00/+0x04/+0x08, up +0x10/+0x14/+0x18, at +0x20/+0x24/+0x28.

x87 transcendental summary (for `sa_math::x87`):

| addr | function | x87 ops needed |
|---|---|---|
| 0x4CF9B0 | Scale | fmul only |
| 0x4CF9E0 | Copy | none (integer moves) |
| 0x4D37D0 | Conjugate | fchs |
| 0x59C160 | Get(x,y,z) Euler | fpatan x3, fsin, fcos |
| 0x59C230 | Get(axis, angle) | `_CIacos` 0x822380 (fpatan + fsqrt inside the CRT), fsin, fdivr |
| 0x59C270 | Multiply | fmul/fadd/fsub only |
| 0x59C300 | Slerp(5 args) | fsin x2 |
| 0x59C3E0 | Set(RwMatrix) | fsqrt, fdivr |
| 0x59C530 | Set(x,y,z) Euler | fsin x3, fcos x3 |
| 0x59C600 | Set(axis, angle) | fsin, fcos |
| 0x59C630 | Slerp(3 args) | via 0x4D00E0 (`_CIacos`, fsin) and 0x59C300 |

## 0x4CF9B0 CQuaternion::Scale  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = CQuaternion*), arg f32 `k` at [esp+4], `ret 4`, returns void. Callers: 0 in game code (gr only).
- Behaviour: for each of x, y, z, w in that order: `fld k; fmul [this+i]; fstp [this+i]` (0x4CF9B0-0x4CF9D3), i.e. `q.i = k * q.i` rounded to f32.

## 0x4CF9E0 CQuaternion::Copy  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this), arg `const CQuaternion* src` at [esp+4], `ret 4`; returns EAX = this (0x4CF9E0 `mov eax, ecx`).
- Behaviour: copies the four dwords +0x0, +0x4, +0x8, +0xC from src to this as raw 32-bit moves (no FPU, NaN payloads preserved).

## 0x4D37D0 CQuaternion::Conjugate  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this), no args, plain `ret`.
- Behaviour: x = -x, y = -y, z = -z via `fld; fchs; fstp` (0x4D37D0-0x4D37E3); w untouched. Because the value passes through
  `fld`/`fstp`, a signalling NaN would be quieted; otherwise this is a sign-bit flip.

## 0x59C160 CQuaternion::Get(float* x, float* y, float* z)  (catalogue: specified/REA, port level 1)
- `__thiscall` (ECX = this), args `float* outX` [esp+4], `float* outY` [esp+8], `float* outZ` [esp+0xC] (seen as +0x44/+0x48/+0x4C
  after `sub esp, 0x40`), `ret 0xC`, void. Callers: 0 in game code. Callee: CQuaternion::Get(RwMatrix*) 0x59C080 (gr-reversed),
  thiscall with ECX = this still intact, filling a local 0x40-byte RwMatrix `m` at [esp] (0x59C163-0x59C168).
- Behaviour (Euler angles from the rotation matrix; all three outputs are wrapped into [0, 2π)):
  1. `a = fpatan(st1 = m.right.y [esp+4], st0 = m.up.y [esp+0x14])` = atan2(m.right.y, m.up.y) (0x59C16D-0x59C179).
     `*outZ = (f32)a` (0x59C17D). `fcom [0x858B50]=0.0; test ah, 5; jp` (0x59C17F-0x59C18A): when a < 0 (only C0 set)
     `*outZ = (f32)(a + [0x858CBC]=6.2831855f)` computed from the in-register a (0x59C18C-0x59C192); a >= 0 or NaN: keep.
  2. `sz = fsin(*outZ)`, `cz = fcos(*outZ)` — both reload the stored f32 `*outZ` (0x59C198-0x59C19E).
  3. `b = atan2(-m.at.y [esp+0x24], (sz * m.right.y [esp+4]) + (cz * m.up.y [esp+0x14]))` (0x59C1A4-0x59C1B8, fchs on at.y
     before the sum; the sum is `fld st2; fmul; fld st2; fmul; faddp`). `*outX = b`, then the same <0 → +2π [0x858CBC] fix
     (0x59C1BA-0x59C1D3).
  4. `N = -((m.right.z [esp+8] * cz) - (m.up.z [esp+0x18] * sz))`, `D = (m.right.x [esp] * cz) - (m.up.x [esp+0x10] * sz)`
     (0x59C1D7-0x59C1F7), `c = fpatan` = atan2(N, D) (0x59C1F9). Stack juggling `fstp st(2); fstp st(0)` leaves c only.
     `*outY = c`, the same <0 → +2π fix (0x59C1FF-0x59C216).
- Edge cases: NaN results are stored as NaN (the jp skips the wrap). The value 2π itself is never produced by the wrap
  unless a rounds so that a + 2π rounds to 2π in f32 (tiny negative a), which the original allows.
- gta-reversed: `plugin::CallMethod` only (Quaternion.cpp:42).

## 0x59C230 CQuaternion::Get(RwV3d* axis, float* angle)  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this, saved in ESI), args `RwV3d* axis` [esp+4] (seen as [esp+8] after `push esi`), `float* angle` [esp+8]
  ([esp+0xC]), `ret 8`, void. Callers: 0 in game code. Callee: `_CIacos` 0x822380 (MSVC CRT; argument and result in ST0;
  computes atan2(sqrt((1+x)*(1-x)), x) with `fpatan`; |x| > 1 goes to the CRT domain-error path and yields NaN).
- Behaviour:
  1. `a = _CIacos(w + w)` — `fld [esi+0xC]; fadd st(0), st(0)` (0x59C233-0x59C236) then call (0x59C238).
  2. `*angle = (f32)a` (0x59C23D-0x59C243).
  3. `k = [0x858624]=1.0f / fsin(a)` (`fsin; fdivr [0x858624]`, 0x59C249-0x59C24B) using the in-register a.
  4. `axis.x = k * x`, `axis.y = k * y`, `axis.z = k * z` (0x59C251-0x59C263).
- ORIGINAL BUG (keep): the angle is `acos(2w)`, not the textbook `2*acos(w)`, and the axis divides by `sin(acos(2w))` instead of
  `sin(angle/2)`. For |w| > 0.5 the acos argument is out of range and the result (angle and axis) is NaN. w = 0.5 gives
  angle 0 and k = 1/0 = +inf. Ports must reproduce this exactly.
- gta-reversed: `plugin::CallMethod` only (Quaternion.cpp:47).

## 0x59C270 CQuaternion::Multiply(const CQuaternion& a, const CQuaternion& b)  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this = result), `a` at [esp+4] (EAX), `b` at [esp+8] (EDX), `ret 8`. Callers: 0 in game code.
- Behaviour (every step rounds to f32 on the `fstp`; the result is written into `this` in two passes, so `this` must not alias
  a or b — the original does not guard against it):
  1. `this.x = (b.z*a.y) - (a.z*b.y)` (0x59C278-0x59C286)
  2. `this.y = (a.z*b.x) - (a.x*b.z)` (0x59C288-0x59C294)
  3. `this.z = (a.x*b.y) - (b.x*a.y)` (0x59C297-0x59C2A3)
  4. `this.x = ((b.w*a.x) + (a.w*b.x)) + this.x` (0x59C2A6-0x59C2B4; reloads the stored f32)
  5. `this.y = ((a.w*b.y) + (b.w*a.y)) + this.y` (0x59C2B6-0x59C2C7)
  6. `this.z = ((a.z*b.w) + (a.w*b.z)) + this.z` (0x59C2CA-0x59C2DB)
  7. `this.w = (a.w*b.w) - (((a.x*b.x) + (a.z*b.z)) + (a.y*b.y))` (0x59C2DE-0x59C2FA; note the dot order x, z, then y).
- gta-reversed: `plugin::CallMethod` only (Quaternion.cpp:52).

## 0x59C300 CQuaternion::Slerp(const CQuaternion& from, const CQuaternion& to, float halfTheta, float sinThetaInv, float t)  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this = result), args: from [esp+4], to [esp+8], theta f32 [esp+0xC], invSin f32 [esp+0x10], t f32 [esp+0x14];
  `ret 0x14`. Callers: CAnimBlendNode::Update 0x4D06C0, CAnimBlendNode::UpdateCompressed 0x4D08D0, Slerp(3 args) 0x59C630.
- Behaviour:
  1. `fld theta; fcomp [0x858B50]=0.0; test ah, 0x44; jp` (0x59C300-0x59C30F): exactly equal (C3 only) → copy the four dwords
     of **to** (not from) into this as integer moves and return (0x59C311-0x59C32B). Unordered (NaN) or not equal → continue.
  2. `fld theta; fcomp [0x858FE4]=1.5707964f; test ah, 0x41; jne` (0x59C32E-0x59C33D): theta <= π/2 or NaN → step 4.
  3. theta > π/2 (0x59C33F-0x59C371): `theta = (f32)([0x858CB8]=3.1415927f - theta)` written back into the argument slot (rounds to
     f32); `w1 = fsin(([0x858624]=1.0f - t) * theta) * invSin`; `w2 = -(fsin(theta * t) * invSin)` (fchs after the multiply).
     invSin is NOT recomputed. Jump to 5.
  4. (0x59C373-0x59C391) `w1 = fsin((1.0f [0x858624] - t) * theta) * invSin`; `w2 = fsin(theta * t) * invSin`.
  5. Blend (0x59C395-0x59C3D1), EAX = from, EDX = to, w1/w2 kept in registers (not rounded between components):
     `this.x = (w1*from.x) + (w2*to.x)`; `this.y = (w2*to.y) + (w1*from.y)`; `this.z = (w2*to.z) + (w1*from.z)`;
     `this.w = (w2*to.w) + (w1*from.w)`. Each stored as f32.
- Notes: the multiplication order inside fsin is `(1 - t) * theta` (theta as memory operand) and `theta * t`.
- gta-reversed: `plugin::CallMethod` only (Quaternion.cpp:57).

## 0x59C3E0 CQuaternion::Set(const RwMatrix& m)  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this), `m` at [esp+4] (EDX), `ret 4`. Callers: CRunningScript::ProcessCommands1900To1999 0x46B460,
  CVehicleModelInfo::PreprocessHierarchy 0x4C8E60.
- Behaviour: four cases tried in order; "t >= 0" below means `fcom [0x858B50]=0.0; fnstsw; test ah, 1; jne next` — the next case
  is taken when C0 is set, i.e. t < 0 or t is NaN. In every case: `s = fsqrt(t + [0x858624]=1.0f)`, the "major" component is
  `[0x858B8C]=0.5f * s` (fld 0.5; fmul st(1)), then `k = 0.5f [0x858B8C] / s` (`fdivr`), and the other three are `(p ± q) * k`,
  each stored as f32 in the listed order. t is kept in extended form between the sum and the sqrt.
  - Case 1 (0x59C3E4): `t = (m.up.y + m.right.x) + m.at.z`. If t >= 0: w = 0.5*s; x = (m.up.z - m.at.y)*k; y = (m.at.x - m.right.z)*k;
    z = (m.right.y - m.up.x)*k; return (0x59C434).
  - Case 2 (0x59C439): `t = (m.right.x - m.up.y) - m.at.z`. If t >= 0: x = 0.5*s; y = (m.up.x + m.right.y)*k; z = (m.at.x + m.right.z)*k;
    w = (m.up.z - m.at.y)*k; return (0x59C489).
  - Case 3 (0x59C48E): `t = (m.up.y - m.right.x) - m.at.z`. If t >= 0: y = 0.5*s; w = (m.at.x - m.right.z)*k;
    x = (m.up.x - m.right.y)*k (0x59C4C7 `fld [edx+0x10]; fsub [edx+4]`); z = (m.at.y + m.up.z)*k; return (0x59C4DE).
  - Case 4 (0x59C4E3, no test): `t = m.at.z - (m.up.y + m.right.x)` (`fld up.y; fadd right.x; fsubr at.z`); z = 0.5*s;
    w = (m.right.y - m.up.x)*k; x = (m.at.x + m.right.z)*k; y = (m.at.y + m.up.z)*k.
- ORIGINAL BUG? (keep): case 3 computes x = (up.x - right.y)*k, while cases 2 and 4 use sums for the off-diagonal pairs; the
  standard conversion would be (up.x + right.y)*k. Bytes at 0x59C4C7-0x59C4CA are unambiguous; ports reproduce them.
- NaN: a NaN trace falls to case 4 and produces NaN.
- gta-reversed: `plugin::CallMethod` only (Quaternion.cpp:62).

## 0x59C530 CQuaternion::Set(float x, float y, float z)  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this), args f32 rx [esp+4], ry [esp+8], rz [esp+0xC] (+0x10/+0x14/+0x18 after `sub esp, 0xC`), `ret 0xC`.
  Callers: 0 in game code. The argument slots are reused as scratch (stack-only effect).
- Behaviour (0x59C533-0x59C5E9): hx = rx*[0x858B8C]=0.5f, hy = ry*0.5f, hz = rz*0.5f (each `fmul` of an f32 arg). The code computes,
  in this order (0x59C533-0x59C55B): cx = fcos(hx), cy = fcos(hy), cz = fcos(hz) (each `fld st0; fcos`, the h values stay on the
  stack). Stores: `fstp [esp+0x10]` = cz (f32, 0x59C55D); `fxch st4; fsin; fstp [esp+0x18]` = sx (f32); `fxch st1; fsin; fstp [esp]`
  = sy (f32); `fxch st2; fsin; fstp [esp+0x14]` = sz (f32). cx and cy stay in registers. Then (0x59C579-0x59C59B):
  `A = cy*cx` (f32 [esp+4]); `B = sy*sx` (f32 [esp+8]); `C = sx*cy` (f32 [esp+0x18]); `D = cx*sy` (register, not rounded to memory).
  - `w = (B*sz) + (A*cz)` (0x59C59F-0x59C5B1)
  - `x = (A*sz) - (B*cz)` (0x59C5B4-0x59C5C6)
  - `y = (sz*D) + (C*cz)` (0x59C5C8-0x59C5D8)
  - `z = (D*cz) - (C*sz)` (0x59C5DB-0x59C5E9)
  Store order: w, x, y, z. Products A, B, C are rounded to f32 (memory), D is not.
- Stack choreography 0x59C55D-0x59C599 traced instruction by instruction (lead, by hand).
- gta-reversed: `plugin::CallMethod` only (Quaternion.cpp:67).

## 0x59C600 CQuaternion::Set(RwV3d* axis, float angle)  (catalogue: specified/REA, port level 0)
- `__thiscall` (ECX = this), `axis` [esp+4], f32 `angle` [esp+8], `ret 8`. Callers: CBike::PreRender 0x6BD090, CBmx::PreRender
  0x6C0810, CPlane::PreRender 0x6C94A0 (5 sites).
- Behaviour: `h = angle * [0x858B8C]=0.5f` (0x59C608); `s = fsin(h)` (h kept); `x = s*axis.x`, `y = s*axis.y`, `z = s*axis.z`
  (s kept in register, 0x59C612-0x59C623); `w = fcos(h)` (0x59C626-0x59C628). The axis is not normalised.
- gta-reversed: `plugin::CallMethod` only (Quaternion.cpp:72).

## 0x59C630 CQuaternion::Slerp(const CQuaternion& from, const CQuaternion& to, float t)  (catalogue: specified/REA, port level 1)
- `__thiscall` (ECX = this, kept in ESI), from [esp+4] (EBX), to [esp+8] (EDI), f32 t [esp+0xC]; `ret 0xC`. Callers: 0 in game code.
- Behaviour: calls the cdecl helper at 0x4D00E0 as `CalcTheta(from, to, &theta, &invSin)` where theta and invSin are written into
  the caller's own argument slots of `from` and `to` (`lea eax,[esp+0x14]` / `lea ecx,[esp+0x14]` at 0x59C63B/0x59C642; `add esp,
  0x10` after); then `this->Slerp(from, to, theta, invSin, t)` 0x59C300 (0x59C65D-0x59C664). The pointers in EBX/EDI were read
  before the slots were overwritten.
- Callee 0x4D00E0 (behaviour from its bytes, for reference): `d = ((a.w*b.w + a.z*b.z) + a.y*b.y) + a.x*b.x` stored f32; if d > 1.0f
  [0x858624] (test ah,0x41 → only strictly greater; NaN keeps d) d = 1.0f; `theta = _CIacos(d)` 0x822380 → `*pTheta`; if theta == 0.0f
  [0x858B50] (test ah,0x44; jnp) `*pInvSin = 0` else `*pInvSin = 1.0f / fsin(theta)`.
- NAME CORRECTION: 0x4D00E0 is catalogued (and hooked by gta-reversed, AnimBlendNode.cpp:15) as `CAnimBlendNode::CalcTheta(float)`,
  a thiscall member. The bytes are a **cdecl free function** `CalcTheta(const CQuaternion&, const CQuaternion&, float* theta,
  float* invSinTheta)` (args at [esp+4..+0x10], plain `ret`, called with `add esp, 0x10`). gr's hook at that address therefore has
  the wrong signature (a gta-reversed error, outside this file's scope; reported for CAT-17).
- gta-reversed: `plugin::CallMethod` only (Quaternion.cpp:77).
