//! STD-5: `CVector` members and helpers, ported from the 1.0 US bytes with the original's x87
//! evaluation order (PC=24 register arithmetic, f32 stores). ST0 results are returned as [`X87`].

use sa_math::x87::X87;

use crate::abi::HookReg;
use crate::hook;
use crate::mem::GPtr;

fn ld(p: GPtr<u8>, o: isize) -> f32 {
    p.byte_add(o).cast::<f32>().read()
}
fn st(p: GPtr<u8>, o: isize, v: X87) {
    p.byte_add(o).cast::<f32>().write(v.to_f32())
}
fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `(x * x + y * y) + z * z` of the vector at `p` (register operands).
fn square_sum(p: GPtr<u8>) -> X87 {
    let (vx, vy, vz) = (x(ld(p, 0)), x(ld(p, 4)), x(ld(p, 8)));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz))
}

/// `CVector::Magnitude()` (0x4082C0): `fsqrt((x² + y²) + z²)` in ST0.
#[hook(addr = 0x4082C0, conv = thiscall, name = "CVector::Magnitude", expect = "D9 41 08 D9 41 04 D9 01 D9 C0",
       unverified = "std5-math-3", route = "L2", hazards = "none", recs = "new-game")]
pub fn magnitude(this: GPtr<u8>) -> X87 {
    square_sum(this).fsqrt()
}

/// `DotProduct(const CVector*, const CVector*)` (0x40FDB0): `(a.z b.z + a.y b.y) + a.x b.x` in ST0.
#[hook(addr = 0x40FDB0, conv = cdecl, name = "CVector::DotProduct-v3d*v3d*", expect = "8B 44 24 04 8B 4C 24 08 D9 40",
       unverified = "std5-math-3", route = "L2", hazards = "none", recs = "new-game")]
pub fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> X87 {
    x(ld(a, 8)).fmul_m(ld(b, 8)).fadd(x(ld(a, 4)).fmul_m(ld(b, 4))).fadd(x(ld(a, 0)).fmul_m(ld(b, 0)))
}

/// `CVector::Difference(const CVector& a, const CVector& b)` (0x40FE00): `this = a - b`.
#[hook(addr = 0x40FE00, conv = thiscall, name = "CVector::Difference", expect = "8B 44 24 04 D9 00 8B 54 24 08",
       unverified = "std5-math-3", route = "L2", hazards = "none", recs = "new-game")]
pub fn difference(this: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) {
    for o in [0, 4, 8] {
        st(this, o, x(ld(a, o)).fsub_m(ld(b, o)));
    }
}

/// `CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730, cdecl, returns `out`):
/// `(b.z a.y - a.z b.y, a.z b.x - b.z a.x, a.x b.y - b.x a.y)`.
#[hook(addr = 0x59C730, conv = cdecl, name = "CVector::CrossProduct-out", expect = "83 EC 0C 8B 4C 24 18 8B 44 24",
       unverified = "std5-math-3", route = "L2", hazards = "none", recs = "new-game")]
pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> {
    let cx = x(ld(b, 8)).fmul_m(ld(a, 4)).fsub(x(ld(a, 8)).fmul_m(ld(b, 4))).to_f32();
    let cy = x(ld(a, 8)).fmul_m(ld(b, 0)).fsub(x(ld(b, 8)).fmul_m(ld(a, 0))).to_f32();
    let cz = x(ld(a, 0)).fmul_m(ld(b, 4)).fsub(x(ld(b, 0)).fmul_m(ld(a, 4))).to_f32();
    out.cast::<f32>().write(cx);
    out.byte_add(4).cast::<f32>().write(cy);
    out.byte_add(8).cast::<f32>().write(cz);
    out
}

/// The rotation part of `m * v` for component `k` (`(m.up[k] v.z + m.fwd... )` in the original's order:
/// x: `(up.x z + fwd.x y) + right.x x`; y and z: `(up z + right x) + fwd y`).
fn rotate(m: GPtr<u8>, v: GPtr<u8>, k: isize) -> X87 {
    let up = x(ld(m, 0x20 + 4 * k)).fmul_m(ld(v, 8));
    if k == 0 {
        up.fadd(x(ld(m, 0x10)).fmul_m(ld(v, 4))).fadd(x(ld(m, 0)).fmul_m(ld(v, 0)))
    } else {
        up.fadd(x(ld(m, 4 * k)).fmul_m(ld(v, 0))).fadd(x(ld(m, 0x10 + 4 * k)).fmul_m(ld(v, 4)))
    }
}

/// `CVector::FromMultiply(const CMatrix&, const CVector&)` (0x59C670): `this = m * v` (with position).
#[hook(addr = 0x59C670, conv = thiscall, name = "CVector::FromMultiply", expect = "8B 44 24 04 8B 54 24 08 D9 40",
       unverified = "std5-math-3", route = "L2", hazards = "none", recs = "new-game")]
pub fn from_multiply(this: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) {
    for k in 0..3 {
        st(this, 4 * k, rotate(m, v, k).fadd_m(ld(m, 0x30 + 4 * k)));
    }
}

/// `CVector::FromMultiply3x3(const CMatrix&, const CVector&)` (0x59C6D0): `this = m * v` (rotation
/// only).
#[hook(addr = 0x59C6D0, conv = thiscall, name = "CVector::FromMultiply3x3", expect = "8B 54 24 04 8B 44 24 08 D9 42",
       unverified = "std5-math-3", route = "L2", hazards = "none", recs = "new-game")]
pub fn from_multiply_3x3(this: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) {
    for k in 0..3 {
        st(this, 4 * k, rotate(m, v, k));
    }
}

/// `CVector::NormaliseAndMag()` (0x59C970): with `s = (x² + y²) + z²` not greater than 0 (ordered),
/// x = 1 and 1.0 is returned; else each component times `1 / fsqrt(s)` and `1 / (1 / fsqrt(s))` returned.
#[hook(addr = 0x59C970, conv = thiscall, name = "CVector::NormaliseAndMag", expect = "D9 41 08 D9 41 04 D9 01 D9 C0",
       unverified = "std5-math-3", route = "L2", hazards = "none", recs = "new-game")]
pub fn normalise_and_mag(this: GPtr<u8>) -> X87 {
    let s = square_sum(this);
    let zero = GPtr::<f32>::from_addr(0x858B50).read();
    let one = GPtr::<f32>::from_addr(0x858624).read();
    // `fcom; test ah, 0x41; jp`: greater or unordered normalises
    if s.to_f64() <= f64::from(zero) {
        this.cast::<u32>().write(0x3F80_0000);
        return X87::from_f32(one);
    }
    let inv = s.fsqrt().fdivr_m(one);
    for o in [0, 4, 8] {
        st(this, o, inv.fmul_m(ld(this, o)));
    }
    inv.fdivr_m(one)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_MAGNITUDE,
    HOOK_DOT_PRODUCT,
    HOOK_DIFFERENCE,
    HOOK_CROSS_PRODUCT,
    HOOK_FROM_MULTIPLY,
    HOOK_FROM_MULTIPLY_3X3,
    HOOK_NORMALISE_AND_MAG,
];
