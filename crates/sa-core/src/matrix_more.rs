//! STD-6: the remaining small `CMatrix` members the native boot reaches (inverse, product, sum, the
//! axis rotations, orientation reset), ported from the 1.0 US bytes with the original's x87 grouping
//! (PC=24 register values, every element stored as f32).
//!
//! `CMatrix`: right +0, forward +0x10, up +0x20, position +0x30 (f32 x, y, z each), the RW matrix
//! pointer +0x40 and its ownership flag +0x44. The by-value results (`operator*`, `operator+`,
//! `Invert(const CMatrix&)`) copy a stack temporary whose padding dwords (+0xC, +0x1C, +0x2C, +0x3C) the
//! original never writes; they are 0 here.

use sa_math::x87::X87;

use crate::abi::HookReg;
use crate::hook;
use crate::mem::GPtr;

fn ld(p: GPtr<u8>, o: isize) -> X87 {
    X87::from_f32(p.byte_add(o).cast::<f32>().read())
}
fn ldf(p: GPtr<u8>, o: isize) -> f32 {
    p.byte_add(o).cast::<f32>().read()
}
fn st(p: GPtr<u8>, o: isize, v: X87) {
    p.byte_add(o).cast::<f32>().write(v.to_f32())
}
fn stu(p: GPtr<u8>, o: isize, v: u32) {
    p.byte_add(o).cast::<u32>().write(v)
}

const ONE: u32 = 0x3F80_0000;

/// `CMatrix::operator+=(const CMatrix& m)` (0x59ADF0, `ret 4`, returns `this`): the twelve elements
/// `this[i] = m[i] + this[i]`, column-major order as the original walks them.
#[hook(addr = 0x59ADF0, conv = thiscall, name = "CMatrix::operator+=", expect = "8B C1 8B 4C 24 04 D9 01",
       unverified = "std6-math-2", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn add_assign(this: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    for c in [0isize, 4, 8] {
        for r in [0isize, 0x10, 0x20] {
            st(this, r + c, ld(m, r + c).fadd_m(ldf(this, r + c)));
        }
    }
    for o in [0x30isize, 0x34, 0x38] {
        st(this, o, ld(m, o).fadd_m(ldf(this, o)));
    }
    this
}

/// `CMatrix::ResetOrientation()` (0x59AEA0): the 3x3 part becomes the identity (position untouched).
#[hook(addr = 0x59AEA0, conv = thiscall, name = "CMatrix::ResetOrientation", expect = "33 C0 BA 00 00 80 3F 89",
       unverified = "std6-math-2", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn reset_orientation(this: GPtr<u8>) {
    for (o, v) in [(0, ONE), (4, 0), (8, 0), (0x10, 0), (0x14, ONE), (0x18, 0), (0x20, 0), (0x24, 0), (0x28, ONE)] {
        stu(this, o, v);
    }
}

/// `CMatrix::SetTranslateOnly(float x, float y, float z)` (0x59AF80, `ret 0xC`): the position only.
#[hook(addr = 0x59AF80, conv = thiscall, name = "CMatrix::SetTranslateOnly", expect = "8B 44 24 04 8B 54 24 08",
       unverified = "std6-math-2", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn set_translate_only(this: GPtr<u8>, x: u32, y: u32, z: u32) {
    stu(this, 0x30, x);
    stu(this, 0x34, y);
    stu(this, 0x38, z);
}

fn rot_x(this: GPtr<u8>, angle: f32) {
    let c = X87::from_f32(angle).fcos();
    for (o, v) in [(0, ONE), (4, 0), (8, 0), (0x10, 0), (0x20, 0)] {
        stu(this, o, v);
    }
    let s = X87::from_f32(angle).fsin();
    st(this, 0x14, c);
    st(this, 0x18, s);
    st(this, 0x24, s.fchs());
    st(this, 0x28, c);
}

fn rot_y(this: GPtr<u8>, angle: f32) {
    let c = X87::from_f32(angle).fcos();
    for (o, v) in [(4, 0), (0x10, 0), (0x14, ONE), (0x18, 0), (0x24, 0)] {
        stu(this, o, v);
    }
    let s = X87::from_f32(angle).fsin();
    st(this, 0, c);
    st(this, 8, s.fchs());
    st(this, 0x20, s);
    st(this, 0x28, c);
}

fn zero_pos(this: GPtr<u8>) {
    for o in [0x30, 0x34, 0x38] {
        stu(this, o, 0);
    }
}

/// `CMatrix::SetRotateXOnly(float angle)` (0x59AFA0): right (1, 0, 0), forward (0, c, s), up (0, -s, c).
#[hook(addr = 0x59AFA0, conv = thiscall, name = "CMatrix::SetRotateXOnly", expect = "D9 44 24 04 33 C0 D9 FF C7 01",
       unverified = "std6-math-2", route = "L2", hazards = "x87-transcendental", recs = "new-game")]
pub fn set_rotate_x_only(this: GPtr<u8>, angle: f32) {
    rot_x(this, angle);
}

/// `CMatrix::SetRotateYOnly(float angle)` (0x59AFE0): right (c, 0, -s), forward (0, 1, 0), up (s, 0, c).
#[hook(addr = 0x59AFE0, conv = thiscall, name = "CMatrix::SetRotateYOnly", expect = "D9 44 24 04 33 C0 D9 FF 89 41 04",
       unverified = "std6-math-2", route = "L2", hazards = "x87-transcendental", recs = "new-game")]
pub fn set_rotate_y_only(this: GPtr<u8>, angle: f32) {
    rot_y(this, angle);
}

/// `CMatrix::SetRotateX(float angle)` (0x59B060): `SetRotateXOnly` and the position cleared.
#[hook(addr = 0x59B060, conv = thiscall, name = "CMatrix::SetRotateX", expect = "D9 44 24 04 33 C0 D9 FF C7 01",
       unverified = "std6-math-2", route = "L2", hazards = "x87-transcendental", recs = "new-game")]
pub fn set_rotate_x(this: GPtr<u8>, angle: f32) {
    zero_pos(this);
    rot_x(this, angle);
}

/// `CMatrix::SetRotateY(float angle)` (0x59B0A0): `SetRotateYOnly` and the position cleared.
#[hook(addr = 0x59B0A0, conv = thiscall, name = "CMatrix::SetRotateY", expect = "D9 44 24 04 33 C0 D9 FF 89 41 04",
       unverified = "std6-math-2", route = "L2", hazards = "x87-transcendental", recs = "new-game")]
pub fn set_rotate_y(this: GPtr<u8>, angle: f32) {
    zero_pos(this);
    rot_y(this, angle);
}

/// `Invert(const CMatrix& in, CMatrix& out)` (0x59B920, cdecl, returns `out`): out's position is
/// zeroed, its 3x3 part is in's transpose, then the position is accumulated from out's stored rows
/// (`p = in.pos.x * row0 + in.pos.y * row1 + in.pos.z * row2`, each step stored f32) and negated
/// (`* -1.0`). Padding dwords untouched.
#[hook(addr = 0x59B920, conv = cdecl, name = "CMatrix::Invert-1", expect = "8B 44 24 08 33 C9 89 48",
       unverified = "std6-math-2", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn invert_into(inp: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    zero_pos(out);
    for r in 0..3isize {
        for c in 0..3isize {
            stu(out, r * 0x10 + c * 4, inp.byte_add(c * 0x10 + r * 4).cast::<u32>().read());
        }
    }
    let (px, py, pz) = (ldf(inp, 0x30), ldf(inp, 0x34), ldf(inp, 0x38));
    st(out, 0x30, X87::from_f32(px).fmul_m(ldf(out, 0)));
    st(out, 0x34, ld(out, 4).fmul_m(px));
    st(out, 0x38, ld(out, 8).fmul_m(px));
    st(out, 0x30, X87::from_f32(py).fmul_m(ldf(out, 0x10)).fadd_m(ldf(out, 0x30)));
    st(out, 0x34, ld(out, 0x14).fmul_m(py).fadd_m(ldf(out, 0x34)));
    st(out, 0x38, ld(out, 0x18).fmul_m(py).fadd_m(ldf(out, 0x38)));
    st(out, 0x30, ld(out, 0x20).fmul_m(pz).fadd_m(ldf(out, 0x30)));
    st(out, 0x34, X87::from_f32(pz).fmul_m(ldf(out, 0x24)).fadd_m(ldf(out, 0x34)));
    st(out, 0x38, ld(out, 0x28).fmul_m(pz).fadd_m(ldf(out, 0x38)));
    for o in [0x30, 0x34, 0x38] {
        st(out, o, ld(out, o).fmul_m(-1.0));
    }
    out
}

/// Writes a by-value `CMatrix` result: the 16 dwords of `tmp`, then +0x40 / +0x44 = 0.
fn emit(out: GPtr<u8>, tmp: &[u32; 16]) -> GPtr<u8> {
    stu(out, 0x40, 0);
    for (i, v) in tmp.iter().enumerate() {
        stu(out, i as isize * 4, *v);
    }
    stu(out, 0x44, 0);
    out
}

/// `CMatrix Invert(const CMatrix& in)` (0x59BDD0, cdecl, hidden result pointer first): `Invert(in, tmp)`
/// into a temporary without RW matrix, copied to the result with +0x40 / +0x44 = 0 (the temporary's
/// destructor has nothing to free).
#[hook(addr = 0x59BDD0, conv = cdecl, name = "CMatrix::Invert-2", expect = "83 EC 4C 8B 4C 24 54 53",
       unverified = "std6-math-2", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn invert(out: GPtr<u8>, inp: GPtr<u8>) -> GPtr<u8> {
    let mut tmp = [0u32; 18];
    let t = GPtr::<u32>::from_raw(tmp.as_mut_ptr()).cast::<u8>();
    invert_into(inp, t);
    let mut m = [0u32; 16];
    m.copy_from_slice(&tmp[..16]);
    emit(out, &m)
}

/// `CMatrix operator*(const CMatrix& a, const CMatrix& b)` (0x59BE30, cdecl, hidden result pointer
/// first): `a * b` with the original's grouping per element (`(p + q) + r`, then `+ a.pos` for the
/// position).
#[hook(addr = 0x59BE30, conv = cdecl, name = "CMatrix::operator*-Mat", expect = "83 EC 4C 8B 44 24 54 8B",
       unverified = "std6-math-2", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn mul(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> {
    let m = |p: GPtr<u8>, o: isize| ldf(p, o);
    // ((x1 * y1) + (x2 * y2)) + (x3 * y3), every product formed from two f32 loads.
    let s3 = |t: [(f32, f32); 3]| {
        X87::from_f32(t[0].0)
            .fmul_m(t[0].1)
            .fadd(X87::from_f32(t[1].0).fmul_m(t[1].1))
            .fadd(X87::from_f32(t[2].0).fmul_m(t[2].1))
    };
    let mut r = [0u32; 16];
    let mut put = |i: usize, v: X87| r[i] = v.to_f32().to_bits();
    put(0, s3([(m(a, 0x10), m(b, 4)), (m(a, 0), m(b, 0)), (m(b, 8), m(a, 0x20))]));
    put(1, s3([(m(a, 0x24), m(b, 8)), (m(a, 4), m(b, 0)), (m(a, 0x14), m(b, 4))]));
    put(2, s3([(m(a, 0x18), m(b, 4)), (m(a, 0x28), m(b, 8)), (m(b, 0), m(a, 8))]));
    put(4, s3([(m(b, 0x18), m(a, 0x20)), (m(a, 0x10), m(b, 0x14)), (m(b, 0x10), m(a, 0))]));
    put(5, s3([(m(a, 0x24), m(b, 0x18)), (m(a, 0x14), m(b, 0x14)), (m(a, 4), m(b, 0x10))]));
    put(6, s3([(m(b, 0x10), m(a, 8)), (m(a, 0x28), m(b, 0x18)), (m(a, 0x18), m(b, 0x14))]));
    put(8, s3([(m(a, 0x10), m(b, 0x24)), (m(b, 0x28), m(a, 0x20)), (m(b, 0x20), m(a, 0))]));
    put(9, s3([(m(a, 0x24), m(b, 0x28)), (m(a, 0x14), m(b, 0x24)), (m(b, 0x20), m(a, 4))]));
    put(10, s3([(m(b, 0x20), m(a, 8)), (m(a, 0x28), m(b, 0x28)), (m(a, 0x18), m(b, 0x24))]));
    put(12, s3([(m(a, 0x10), m(b, 0x34)), (m(b, 0x38), m(a, 0x20)), (m(b, 0x30), m(a, 0))]).fadd_m(m(a, 0x30)));
    put(13, s3([(m(b, 0x38), m(a, 0x24)), (m(b, 0x34), m(a, 0x14)), (m(b, 0x30), m(a, 4))]).fadd_m(m(a, 0x34)));
    put(14, s3([(m(b, 0x30), m(a, 8)), (m(a, 0x18), m(b, 0x34)), (m(a, 0x28), m(b, 0x38))]).fadd_m(m(a, 0x38)));
    emit(out, &r)
}

/// `CMatrix operator+(const CMatrix& a, const CMatrix& b)` (0x59BFA0, cdecl, hidden result pointer
/// first): element-wise `a + b`.
#[hook(addr = 0x59BFA0, conv = cdecl, name = "CMatrix::operator+-", expect = "83 EC 4C 8B 44 24 54 8B",
       unverified = "std6-math-2", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> {
    let mut r = [0u32; 16];
    for i in [0usize, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14] {
        let o = i as isize * 4;
        r[i] = ld(a, o).fadd_m(ldf(b, o)).to_f32().to_bits();
    }
    emit(out, &r)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_ADD_ASSIGN,
    HOOK_RESET_ORIENTATION,
    HOOK_SET_TRANSLATE_ONLY,
    HOOK_SET_ROTATE_X_ONLY,
    HOOK_SET_ROTATE_Y_ONLY,
    HOOK_SET_ROTATE_X,
    HOOK_SET_ROTATE_Y,
    HOOK_INVERT_INTO,
    HOOK_INVERT,
    HOOK_MUL,
    HOOK_ADD,
];
