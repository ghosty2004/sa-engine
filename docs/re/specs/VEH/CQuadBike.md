# CQuadBike (VEH) - full specs

Layout (gr `QuadBike.h`, size 0x9BC, derives CAutomobile 0x988): +0x988 `m_pHandling` (tBikeHandlingData*: +0x2C
`m_fWheelieAng`, +0x30 `m_fStoppieAng`), +0x98C `m_sRideAnimData` (CRideAnimData), +0x9A8 `field_9A8[4]`, +0x9B8
`m_nQuadFlags`. Vtable 0x871AE8.

## 0x6CDC30 CQuadBike::~CQuadBike  (catalogue: specified/REA, port level 16)
- Convention: `__thiscall void (CQuadBike* this)`, plain `ret`. SEH frame (handler 0x848618) only for unwinding.
- Callers: CQuadBike::vfunc_0 0x6CE440. Callees: 0x6D0B30 (CRideAnimData destructor; the body is a single `ret`),
  CAutomobile::Destructor 0x6A61E0.
- Behaviour: `*(void**)this = 0x871AE8`; destroy `m_sRideAnimData` (call 0x6D0B30 with ecx = this + 0x98C, no
  effect); `CAutomobile::~CAutomobile(this)`.
- gta-reversed: compiler-generated.

## 0x6CDCC0 CQuadBike::ProcessControl  (catalogue: specified/REA, port level 26)
- Convention: `__thiscall void (CQuadBike* this)`, plain `ret`. Locals 0x38. Virtual slot 10.
- Callees: 0x59C810 (`out = M^T * v`, cdecl (CVector* out, CVector* v, CMatrix* m): `out.x = (r.y*v.y + r.z*v.z) +
  v.x*r.x`, `out.y = (f.y*v.y + f.x*v.x) + f.z*v.z`, `out.z = (u.y*v.y + u.x*v.x) + u.z*v.z`), Multiply3x3 0x59C790
  (cdecl (CVector* out, CMatrix* m, CVector* v) = `M * v` rotation only: `out.x = (u.x*v.z + f.x*v.y) + r.x*v.x`, ...),
  CRT pow 0x822130 (x87: ST1 = base, ST0 = exponent), CPhysical::ApplyTurnForce 0x542A50 (thiscall (CVector force,
  CVector point) by value), CAutomobile::ProcessControl 0x6B1880.
- Globals: `bDoQuadDamping` 0x8D3450 (u8, 1), `vecQuadResistance` 0x8D3458 (CVector, data: 0.995f, 0.995f, 1.0f),
  `CTimer::ms_fTimeStep` 0xB7CB5C. Constants (.rdata): 0x871ABC 0.995f, 0x871AC0 0.5f, 0x871AC4 1.0f, 0x871AC8 1.0f,
  0x858624 1.0f, 0x858B50 0.0f, 0x858C84 0.25f, 0x858CA8 0.07f, 0x858C24 0.3f, 0x858B1C 0.1f, 0x858C20 0.9f,
  0x858C1C -1.0f.
- Behaviour:
  1. If `(byte[+0x36] & 0xF8) != 0` (status not PLAYER) or `bDoQuadDamping == 0`: go to step 6.
  2. Locals: `h = 0.5f` [0x871AC0], `k = 1.0f` [0x871AC4], `R = vecQuadResistance` (x, y, z copied).
     `v = M^T * m_vecTurnSpeed (+0x50)` (0x59C810 with the matrix +0x14).
  3. Select `X` (x87) and `A` (x87):
     - Front wheels in the air (`comp[0] (+0x7D4)` and `comp[2] (+0x7DC)` are exactly 0x3F800000):
       `A = 0.995f [0x871ABC]`. If a rear wheel touches (`comp[1] < 1.0f` or else `comp[3] < 1.0f`) and
       `m_matrix->forward.z > 0`: `m = fabs(m_pHandling->m_fWheelieAng - forward.z) * 0.25f`; `if (0.07f < m) m = 0.07f`;
       `X = R.x - m`. Otherwise: `k = 1.0f` [0x871AC8], `X = 0.995f` [0x871ABC] (the .rdata constant, not R.x).
     - Else, if `m_WheelCounts[1] (+0x7F8)` and `m_WheelCounts[3] (+0x800)` are exactly 1.0f: `A = 0.995f`
       [0x871ABC]; if `forward.z < 0`: `m = fabs(m_pHandling->m_fStoppieAng - forward.z) * 0.3f`; `if (0.1f < m) m =
       0.1f`; `X = (m + 0.9f) * R.x`; else `X = R.x`.
     - Else: `A = h` (0.5f), `X = R.x`.
  4. `p = f32(X / (v.x * v.x * A + 1.0f))` (0x6CDE2C-0x6CDE3E: `fld v.x; fmul v.x; fmul A; fadd 1; fdivr X`);
     `q = f32(R.y / (v.y * v.y * k + 1.0f))`; `p = f32(pow(p, ts))`; `q = pow(q, ts)` (x87).
     `a = f32(v.z * p - v.z)`; `b = q * v.y - v.y` (x87); `mass = m_fTurnMass (+0x90)`.
  5. Two turn forces, the matrix pointer re-read before each:
     - `F1 = (f32(-u.x * b) * mass, (-u.y * b) * mass, f32(-u.z * b) * mass)` where `-u = m_matrix->up * -1.0f`
       (each `fld; fmul -1`), stored f32 per component; `P1 = (M * m_vecCentreOfMass (+0xA4)) + m_matrix->right`
       (`out.c + right.c`); `ApplyTurnForce(F1, P1)`.
     - `F2 = ((a * u.x) * mass, (a * u.y) * mass, f32(a * u.z) * mass)` (only the z product is stored before the
       mass multiply; each result component is stored f32; `mass` re-read into the slot that held `a`); `P2 = m_matrix->forward + (M * COM)`
       (`forward.c + out.c`); `ApplyTurnForce(F2, P2)`.
  6. `CAutomobile::ProcessControl()`.
- gta-reversed: `plugin::CallMethod` followed by dead C++ (QuadBike.cpp), which differs from the bytes:
  (a) gr's x-damping uses `pow(vecQuadResistance.x, ts)` and multiplies `velocityOS.x`; the original uses
  `pow(X / (v.x^2 * A + 1), ts)` (X and A from step 3, so the wheelie/stoppie values are used) and multiplies `v.z`;
  (b) gr transforms the centre of mass with InverseTransformVector, the original with the forward rotation
  (Multiply3x3 0x59C790); (c) the original uses the .rdata 0.995f where gr reads `vecQuadResistance.y` (A) and, in
  the air branch, for X. Port the bytes, not the dead C++.
- Open questions: none.

## 0x6CE440 CQuadBike::vfunc_0  (catalogue: specified/REA, port level 17)
- Convention: `__thiscall CQuadBike* (CQuadBike* this, u8 flags [esp+4])`, `ret 4` (scalar deleting destructor, slot 0;
  proposed name `CQuadBike::scalar_deleting_destructor`).
- Callees: ~CQuadBike 0x6CDC30, CVehicle::operator delete 0x6E2D90.
- Behaviour: `~CQuadBike(this)`; `if (flags & 1) CVehicle::operator delete(this)`; return `this`.
