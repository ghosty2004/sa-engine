# MATH: functions without gta-reversed source (RE-SPEC-UNK group A)

## 0x404330 std::min<float> (proposed)  (catalogue: unknown/REA, port level 0)
- Convention: `__cdecl float (float a [esp+4], float b [esp+8])`, result in ST0, plain `ret`.
- Behaviour: `fld a; fcomp b` (0x404330/0x404334); `test ah,5; jp` (0x40433a): taken when C0=C2 (a >= b or unordered) -> returns b (0x404344); else (a < b) returns a.
- So `a < b ? a : b`; NaN in either -> b; equal -> b. 24 callers in AI/VEH/AUDIO/PED.
- gta-reversed: matches `std::min(b, a)` ordering semantics (`b < a ? b : a` differs only for equal/NaN: here equal returns b). Port: implement as written.

## 0x406ce0 floorf (out-of-line wrapper) (proposed)  (catalogue: unknown/REA, port level 0)
- Convention: `__cdecl float (float x)`, returns ST0 (the f64 result of the CRT routine left on the x87 stack, not re-rounded here).
- Behaviour: `fld x; fstp qword [esp]` (exact widening) then `call 0x8219f0` (CRT floor, SSE2/x87 dispatch), `add esp,8`.
- Callers: CAEAudioHardware::PlaySound, CPlayerInfo::Process (3).
- Open question: callee identified as floor from its error-path opcode 0xb (0x828aaf), not from a symbol.

## 0x406d10 std::fabs(float) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl float (float x)`: `fld x; fabs`, ST0 return (0x406d10). 4 callers (CCarCtrl, CAutomobile::TowTruckControl).

## 0x406d70 CVector::operator-= (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CVector& (const CVector& v)`, returns EAX = this, `ret 4`.
- `x = x - v.x` (0x406d76), then y, then z; each `fld this; fsub v; fstp this` (f32 stores per component).
- gta-reversed: `CVector::operator-=` (Core/Vector.h inline).

## 0x406da0 CVector::SquaredMagnitude (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall float ()` (this = const CVector*), ST0 return.
- x87 order: loads z, y, x (0x406da0-0x406da6); `x*x` (0x406daa); `y*y` (0x406dae); `xx + yy` (0x406db0); `z*z` (0x406db4); `(xx+yy) + zz` (0x406db6); stack cleanup leaves the sum in ST0 (0x406db8-0x406dbc). Result not stored, so it keeps x87 precision (24-bit PC in game).
- gta-reversed: `CVector::SquaredMagnitude()` inline (Core/Vector.h): `x*x + y*y + z*z`, same order.

## 0x40eec0 CColBox::operator= (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CColBox& (const CColBox& src)`, EAX = this, `ret 4`; body 0x156d230.
- Copies dwords +0,+4,+8 then +0xc,+0x10,+0x14, then word +0x18 and byte +0x1a (CColSurface material/piece/lighting; byte +0x1b is not copied).
- gta-reversed: compiler-generated `CColBox::operator=` (Collision/ColBox.h); note the 4th surface byte (+0x1b, `m_nLight`?) is not copied (0x156d26e copies only `[ecx+2]`).
- Open question: name of the +0x1b byte in this build and whether it is padding (0x156d26e).

## 0x40ef50 CColLine::CColLine (proposed)  (catalogue: specified/REA, port level 0)
- `__thiscall CColLine* (const CVector& start, const CVector& end)`, EAX = this, `ret 8`; body 0x15611b0.
- Copies start (3 dwords) to +0..+8, end to +0x10..+0x18 (+0xc and +0x1c untouched padding).
- gta-reversed: `CColLine::CColLine(const CVector&, const CVector&)` (Collision/ColLine.cpp).

## 0x40fe30 operator+(const CVector&, const CVector&) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl CVector* (CVector* out, const CVector* a, const CVector* b)`, returns EAX = out, plain `ret`.
- Computes `a.z+b.z`, `a.y+b.y`, `a.x+b.x` (that order, 0x40fe34-0x40fe4a), stores x, y, z.
- gta-reversed: `operator+(CVector, CVector)` (Core/Vector.h).

## 0x40fe60 operator-(const CVector&, const CVector&) (proposed)  (catalogue: specified/REA, port level 0)
- `__cdecl CVector* (CVector* out, const CVector* a, const CVector* b)`, EAX = out. `a.z-b.z`, `a.y-b.y`, `a.x-b.x`, stores x,y,z (0x40fe64-0x40fe81).

## 0x40fe90 operator*(float, const CVector&) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl CVector* (CVector* out, float s, const CVector* v)`, EAX = out. `s*v.z`, `s*v.y`, `s*v.x` (each `fld s; fmul v.c`), stores x,y,z (0x40fe94-0x40feb1).

## 0x40fec0 operator*(const CVector&, float) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl CVector* (CVector* out, const CVector* v, float s)`, EAX = out. `s*v.z`, `s*v.y`, `s*v.x` (`fld s; fmul`), stores x,y,z (0x40fec4-0x40fee1).

## 0x40fef0 CVector::operator*=(float) (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CVector& (float s)`, EAX = this, `ret 4`. For x, y, z in order: `fld s; fmul this.c; fstp this.c` (0x40fef0-0x40ff0b).

## 0x411590 UncompressVector (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl void (CVector* out, const CompressedVector* in)` (plain `ret`).
- For x, y, z: `movsx` i16 -> i32 stored over the arg slot, `fild`, `fmul 0.0078125f [0x858b88]`, `fstp out.c` (0x411594-0x4115d2).
- EAX on return is NOT `out` (it ends as the sign-extended z, 0x4115c0); callers must not rely on EAX.
- gta-reversed: `UncompressVector` (Collision/CompressedVector.h) divides by 128.0f; multiply by 1/128 is exact, same result.

## 0x4119d0 operator/(const CVector&, float) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl CVector* (CVector* out, const CVector* v, float d)`, EAX = out.
- `inv = 1.0f [0x858624] / d` (0x4119da); `fst [esp+0xc]` (inv rounded to f32 over the arg slot); `z = inv(ST0, unrounded) * v.z`; `y = inv_f32 * v.y`; `x = inv_f32 * v.x` (0x4119e2-0x4119f0); stores x, y, z.
- Under 24-bit precision control the ST0 copy and the stored f32 are equal. gta-reversed `operator/(CVector, float)` (Core/Vector.h) also multiplies by the reciprocal.

## 0x411a00 CVector::operator+= (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CVector& (const CVector& v)`, EAX = this, `ret 4`. For x, y, z: `fld v.c; fadd this.c; fstp this.c` (0x411a06-0x411a1b).

## 0x411ba0 CMatrix::GetForward (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall CVector& ()`, EAX = this + 0x10 (0x411ba0). Out-of-line `CMatrix::GetForward()` (Core/Matrix.h, `m_forward` at +0x10). Any struct with a member at +0x10 fits; the CMatrix reading is from the caller context.
- Open question: exact owner class (0x411ba0).

## 0x41bce0 std::sin(float) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl float (float x)`: `fld x; fsin` (0x41bce4), ST0 return. Raw x87 FSIN (no range reduction beyond the FPU's own |x| < 2^63).

## 0x41bcf0 std::cos(float) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl float (float x)`: `fld x; fcos` (0x41bcf4), ST0 return.

## 0x41bd10 CMatrix::GetForward (proposed)  (catalogue: unknown/REA, port level 0)
- Same body as 0x411ba0: `__thiscall`, EAX = this + 0x10. Open question: owner class (0x41bd10).

## 0x420800 std::max<float> (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl float (float a [esp+4], float b [esp+8])`, ST0 return.
- `fld a; fcomp b; test ah,0x41` (0x420804/0x42080a): jump (C0 or C3 set: a < b, a == b, or unordered) -> return b (0x420814); else (a > b) return a.
- So `a > b ? a : b`; equal or NaN -> b. 35 callers (CCarCtrl, ...).

## 0x46ae00 CMatrix position += CVector (proposed)  (catalogue: unknown/REA, port level 0)
- `__thiscall void (const CVector& v)`, `ret 4`.
- x87: `fld v.z; fld v.y; fld pos.x [+0x30]; fadd v.x; fstp pos.x; fadd pos.y [+0x34] (v.y + pos.y); fstp; fadd pos.z [+0x38]; fstp` (0x46ae04-0x46ae1b).
- Likely an out-of-line `GetPosition() += v` on a CMatrix (pos at +0x30). Open question: exact gr inline name (0x46ae00).

## 0x470000 std::pow(float, float) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl float (float base [esp+4], float exp [esp+8])`: `fld base; fld exp; call 0x822130` (CRT x87-register pow: ST1^ST0), ST0 return.
- Open question: 0x822130 identified as _CIpow from argument shape and the gr std::pow call in ProcessCarWheelPair, not by symbol.

## 0x509760 operator==(const CVector&, const CVector&) (proposed)  (catalogue: unknown/REA, port level 0)
- `__cdecl bool (const CVector* a, const CVector* b)`.
- For x, y, z in order: `fld a.c; fcomp b.c; test ah,0x44; jp -> false` (0x50976a-0x50978b): equal only when C3=1,C2=0. All equal -> AL = 1, else AL = 0 (via a redundant `test al,al; sete` that inverts twice, 0x50978d-0x509799 / 0x50979c-0x5097a5). Upper EAX = 0.
