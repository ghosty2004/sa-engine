//! STD-5: `CMatrix::SetRotateZOnly` (0x59B020), `CMatrix::SetRotate(const CQuaternion&)` (0x59BBF0) and
//! `CRect::Restrict` (0x404200), ported from the 1.0 US bytes with the original's x87 evaluation order
//! (PC=24 register values, f32 stack temporaries).
//!
//! `CMatrix`: right +0, forward +0x10, up +0x20, position +0x30 (f32 x, y, z each; +0xC / +0x1C / +0x2C
//! untouched here).

use sa_math::x87::X87;

use crate::abi::HookReg;
use crate::hook;
use crate::mem::GPtr;

fn st(m: GPtr<u8>, o: isize, v: X87) {
    m.byte_add(o).cast::<f32>().write(v.to_f32())
}
fn ld(p: GPtr<u8>, o: isize) -> f32 {
    p.byte_add(o).cast::<f32>().read()
}

/// `CMatrix::SetRotateZOnly(float angle)` (0x59B020): +8, +0x18, +0x20, +0x24 = 0, +0x28 = 1.0; right =
/// (cos, sin), forward = (-sin, cos) (`fsin` / `fcos` of the argument).
#[hook(addr = 0x59B020, conv = thiscall, name = "CMatrix::SetRotateZOnly", expect = "D9 44 24 04 33 C0 D9 FF 89 41",
       unverified = "std5-math-1", route = "L2", hazards = "x87-transcendental", recs = "new-game")]
pub fn set_rotate_z_only(this: GPtr<u8>, angle: f32) {
    let c = X87::from_f32(angle).fcos();
    for o in [8, 0x18, 0x20, 0x24] {
        this.byte_add(o).cast::<u32>().write(0);
    }
    this.byte_add(0x28).cast::<u32>().write(0x3F80_0000);
    let s = X87::from_f32(angle).fsin();
    st(this, 0, c);
    st(this, 4, s);
    st(this, 0x10, s.fchs());
    st(this, 0x14, c);
}

/// `CMatrix::SetRotate(const CQuaternion& q)` (0x59BBF0): the rotation matrix of (x, y, z, w) from the
/// doubled components (2x, 2y, 2z stored as f32) and their products (some kept in x87 registers), each
/// element computed as the original orders it; `ret 4`.
#[hook(addr = 0x59BBF0, conv = thiscall, name = "CMatrix::SetRotate-quat", expect = "83 EC 20 8B 44 24 24 D9 00 8B",
       unverified = "std5-math-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn set_rotate_quat(this: GPtr<u8>, q: GPtr<u8>) {
    let (x, y, z, w) = (ld(q, 0), ld(q, 4), ld(q, 8), ld(q, 0xC));
    let rx = X87::from_f32(x);
    let ry = X87::from_f32(y);
    let x2 = rx.fadd(rx).to_f32();
    let y2 = ry.fadd(ry).to_f32();
    let z2 = X87::from_f32(z).fdouble().to_f32();
    let xx2 = X87::from_f32(x2).fmul(rx).to_f32();
    let xy2 = X87::from_f32(y2).fmul(rx).to_f32();
    let xz2 = X87::from_f32(z2).fmul(rx).to_f32();
    let yy2 = X87::from_f32(y2).fmul(ry).to_f32();
    let yz2 = X87::from_f32(z2).fmul(ry);
    let zz2 = X87::from_f32(z2).fmul_m(z);
    let xw2 = X87::from_f32(x2).fmul_m(w).to_f32();
    let yw2 = X87::from_f32(y2).fmul_m(w);
    let zw2 = X87::from_f32(z2).fmul_m(w);
    let one = GPtr::<f32>::from_addr(0x858624).read();
    st(this, 0, zz2.fadd_m(yy2).fsubr_m(one));
    st(this, 4, X87::from_f32(xy2).fadd(zw2));
    st(this, 8, X87::from_f32(xz2).fsub(yw2));
    st(this, 0x10, X87::from_f32(xy2).fsub(zw2));
    st(this, 0x14, zz2.fadd_m(xx2).fsubr_m(one));
    st(this, 0x18, X87::from_f32(xw2).fadd(yz2));
    st(this, 0x20, yw2.fadd_m(xz2));
    st(this, 0x24, yz2.fsub_m(xw2));
    st(this, 0x28, X87::from_f32(yy2).fadd_m(xx2).fsubr_m(one));
}

/// `CRect::Restrict(const CRect& r)` (0x404200; CRect: left +0, bottom +4, right +8, top +0xC): grows
/// `this` to contain `r`: left / top take `r`'s when smaller, right / bottom when larger (comparisons
/// false for NaN keep `this`); returns `this`.
#[hook(addr = 0x404200, conv = thiscall, name = "CRect::Restrict", expect = "8B 54 24 04 D9 02 D8 19 DF E0",
       unverified = "std5-math-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn rect_restrict(this: GPtr<u8>, r: GPtr<u8>) -> GPtr<u8> {
    let cp = |o: isize| this.byte_add(o).cast::<u32>().write(r.byte_add(o).cast::<u32>().read());
    if ld(r, 0) < ld(this, 0) {
        cp(0);
    }
    if ld(r, 8) > ld(this, 8) {
        cp(8);
    }
    if ld(r, 0xC) < ld(this, 0xC) {
        cp(0xC);
    }
    if ld(r, 4) > ld(this, 4) {
        cp(4);
    }
    this
}

/// `CRect::StretchToPoint(float x, float y)` (0x5327F0): left = x when x < left, right = x when x >
/// right, top (+0xC) = y when y < top, bottom (+4) = y when y > bottom (NaN keeps); returns `this`.
#[hook(addr = 0x5327F0, conv = thiscall, name = "CRect::StretchToPoint", expect = "D9 44 24 04 D8 19 DF E0 F6 C4",
       unverified = "std5-math-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn rect_stretch_to_point(this: GPtr<u8>, x: f32, y: f32) -> GPtr<u8> {
    let set = |o: isize, v: f32| this.byte_add(o).cast::<u32>().write(v.to_bits());
    if x < ld(this, 0) {
        set(0, x);
    }
    if x > ld(this, 8) {
        set(8, x);
    }
    if y < ld(this, 0xC) {
        set(0xC, y);
    }
    if y > ld(this, 4) {
        set(4, y);
    }
    this
}

/// `CMatrix::SetRotateZ(float angle)` (0x59B0E0): as [`set_rotate_z_only`] and the position zeroed.
#[hook(addr = 0x59B0E0, conv = thiscall, name = "CMatrix::SetRotateZ", expect = "D9 44 24 04 33 C0 D9 FF 89 41",
       unverified = "std5-math-1", route = "L2", hazards = "x87-transcendental", recs = "new-game")]
pub fn set_rotate_z(this: GPtr<u8>, angle: f32) {
    let c = X87::from_f32(angle).fcos();
    for o in [8, 0x18, 0x20, 0x24] {
        this.byte_add(o).cast::<u32>().write(0);
    }
    this.byte_add(0x28).cast::<u32>().write(0x3F80_0000);
    for o in [0x30, 0x34, 0x38] {
        this.byte_add(o).cast::<u32>().write(0);
    }
    let s = X87::from_f32(angle).fsin();
    st(this, 0, c);
    st(this, 4, s);
    st(this, 0x10, s.fchs());
    st(this, 0x14, c);
}

/// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, result through the hidden pointer):
/// each component `((m.up * v.z + m.fwd * v.y) + m.right * v.x) + m.pos` in the original's order (x),
/// `((up * z + right * x) + fwd * y) + pos` for y and z.
#[hook(addr = 0x59C890, conv = cdecl, name = "CMatrix::operator*-Vec", expect = "83 EC 0C 8B 44 24 14 8B 4C 24",
       unverified = "std5-math-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> {
    let (vx, vy, vz) = (ld(v, 0), ld(v, 4), ld(v, 8));
    let x = X87::from_f32(ld(m, 0x20)).fmul_m(vz)
        .fadd(X87::from_f32(ld(m, 0x10)).fmul_m(vy))
        .fadd(X87::from_f32(ld(m, 0)).fmul_m(vx))
        .fadd_m(ld(m, 0x30));
    let y = X87::from_f32(ld(m, 0x24)).fmul_m(vz)
        .fadd(X87::from_f32(ld(m, 4)).fmul_m(vx))
        .fadd(X87::from_f32(ld(m, 0x14)).fmul_m(vy))
        .fadd_m(ld(m, 0x34));
    let z = X87::from_f32(ld(m, 0x28)).fmul_m(vz)
        .fadd(X87::from_f32(ld(m, 8)).fmul_m(vx))
        .fadd(X87::from_f32(ld(m, 0x18)).fmul_m(vy))
        .fadd_m(ld(m, 0x38));
    st(out, 0, x);
    st(out, 4, y);
    st(out, 8, z);
    out
}

/// `CMatrix::SetRotate(x, y, z)` (0x59B120, `ret 0xC`): the rotation matrix of the Euler angles; the
/// position zeroed, the flags untouched. The original keeps `cos y`, `sin z` and `cos z * sin x` in its
/// argument slots (rounded to floats) and the other terms on the x87 stack.
#[hook(addr = 0x59B120, conv = thiscall, name = "CMatrix::SetRotate-xyz", expect = "D9 44 24 04 33 C0 D9 FF",
       unverified = "std6-math-1", route = "L2", hazards = "x87-transcendental", recs = "new-game")]
pub fn set_rotate_xyz(this: GPtr<u8>, ax: f32, ay: f32, az: f32) {
    let cx = X87::from_f32(ax).fcos();
    for o in [0x30, 0x34, 0x38] {
        this.byte_add(o).cast::<u32>().write(0);
    }
    let sx = X87::from_f32(ax).fsin();
    let cy = X87::from_f32(X87::from_f32(ay).fcos().to_f32());
    let sy = X87::from_f32(ay).fsin();
    let cz = X87::from_f32(az).fcos();
    let sz_ext = X87::from_f32(az).fsin();
    let sz = X87::from_f32(sz_ext.to_f32());
    let szsx = sz_ext.fmul(sx);
    let czsx = X87::from_f32(cz.fmul(sx).to_f32());
    st(this, 0, cz.fmul(cy).fsub(szsx.fmul(sy)));
    st(this, 4, czsx.fmul(sy).fadd(sz.fmul(cy)));
    st(this, 8, sy.fmul(cx).fchs());
    st(this, 0x10, sz.fmul(cx).fchs());
    st(this, 0x14, cz.fmul(cx));
    st(this, 0x18, sx);
    st(this, 0x20, cz.fmul(sy).fadd(szsx.fmul(cy)));
    st(this, 0x24, sz.fmul(sy).fsub(czsx.fmul(cy)));
    st(this, 0x28, cy.fmul(cx));
}

/// Rotates the four rows of a matrix about one axis: for each row `(a, b)` (offsets `i`, `j` within the
/// row) `a' = c * a - s * b`, `b' = s * a + c * b`, with `s` the float-rounded sine; `first_s` replaces
/// `s` in the first row's `b * s` term (`RotateY` multiplies with the unrounded sine there).
fn rotate_rows(this: GPtr<u8>, angle: f32, i: isize, j: isize, first_s: Option<X87>) {
    let c = X87::from_f32(angle).fcos();
    let s_ext = X87::from_f32(angle).fsin();
    let s = X87::from_f32(s_ext.to_f32());
    for (k, row) in [0isize, 0x10, 0x20, 0x30].into_iter().enumerate() {
        let a = ld(this, row + i);
        let b = ld(this, row + j);
        let sb = if k == 0 { first_s.unwrap_or(s) } else { s };
        let na = c.fmul_m(a).fsub(sb.fmul_m(b));
        let nb = s.fmul_m(a).fadd(c.fmul_m(b));
        st(this, row + i, na);
        st(this, row + j, nb);
    }
}

/// `CMatrix::RotateX(angle)` (0x59B1E0): every row's (y, z) rotated: `y' = c*y - s*z`, `z' = s*y + c*z`.
#[hook(addr = 0x59B1E0, conv = thiscall, name = "CMatrix::RotateX", expect = "83 EC 20 D9 44 24 24 D9 FF D9 44 24 24 D9 FE D9 5C",
       unverified = "std6-math-1", route = "L2", hazards = "x87-transcendental", recs = "new-game")]
pub fn rotate_x(this: GPtr<u8>, angle: f32) {
    rotate_rows(this, angle, 4, 8, None);
}

/// `CMatrix::RotateY(angle)` (0x59B2C0): every row's (x, z): `x' = s*z + c*x`, `z' = c*z - s*x` (the
/// first row's `s*z` with the unrounded sine).
#[hook(addr = 0x59B2C0, conv = thiscall, name = "CMatrix::RotateY", expect = "83 EC 20 D9 44 24 24 D9 FF D9 44 24 24 D9 FE D9 54",
       unverified = "std6-math-1", route = "L2", hazards = "x87-transcendental", recs = "new-game")]
pub fn rotate_y(this: GPtr<u8>, angle: f32) {
    // (z, x) as the pair: z' = c*z - s*x, x' = s*z + c*x
    let s_ext = X87::from_f32(angle).fsin();
    let c = X87::from_f32(angle).fcos();
    let s = X87::from_f32(s_ext.to_f32());
    for (k, row) in [0isize, 0x10, 0x20, 0x30].into_iter().enumerate() {
        let xv = ld(this, row);
        let zv = ld(this, row + 8);
        let sz = if k == 0 { s_ext } else { s };
        let nx = sz.fmul_m(zv).fadd(c.fmul_m(xv));
        let nz = c.fmul_m(zv).fsub(s.fmul_m(xv));
        st(this, row, nx);
        st(this, row + 8, nz);
    }
}

/// `CMatrix::RotateZ(angle)` (0x59B390): every row's (x, y): `x' = c*x - s*y`, `y' = s*x + c*y`.
#[hook(addr = 0x59B390, conv = thiscall, name = "CMatrix::RotateZ", expect = "83 EC 20 D9 44 24 24 D9 FF D9 44 24 24 D9 FE D9 5C",
       unverified = "std6-math-1", route = "L2", hazards = "x87-transcendental", recs = "new-game")]
pub fn rotate_z(this: GPtr<u8>, angle: f32) {
    rotate_rows(this, angle, 0, 4, None);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SET_ROTATE_XYZ, HOOK_ROTATE_X, HOOK_ROTATE_Y, HOOK_ROTATE_Z, HOOK_SET_ROTATE_Z, HOOK_MATRIX_MUL_VEC, HOOK_SET_ROTATE_Z_ONLY, HOOK_SET_ROTATE_QUAT, HOOK_RECT_RESTRICT, HOOK_RECT_STRETCH_TO_POINT];
