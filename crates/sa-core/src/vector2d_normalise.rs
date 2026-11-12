//! `CVector2D::Normalise` (batch `std8-math-1`): the hook entry of sa-math's model. Unverified (docs/verify/HOWTO.md).

use sa_math::vector::CVector2D;

use crate::abi::HookReg;
use crate::hook;
use crate::mem::GPtr;

/// `CVector2D::Normalise()` (0x44E480, thiscall): `s = x*x + y*y` (x87); `s > 0`: `k = 1 / sqrt(s)`, `x = k*x`,
/// `y = k*y`; otherwise (0, negative zero, NaN) only `x = 1.0` (`sa_math::vector::CVector2D::normalise`).
#[hook(addr = 0x44E480, conv = thiscall, name = "CVector2D::Normalise",
       expect = "D9 41 04 D9 01 D9 C0 D8 C9", unverified = "std8-math-1", route = "L1-pure",
       hazards = "x87-transcendental", note = "FSQRT")]
pub fn vector2d_normalise(v: GPtr<u8>) {
    let (px, py) = (v.cast::<f32>(), v.byte_add(4).cast::<f32>());
    let mut w = CVector2D { x: px.read(), y: py.read() };
    w.normalise();
    px.write(w.x);
    py.write(w.y);
}

pub const HOOKS: &[HookReg] = &[HOOK_VECTOR2D_NORMALISE];
