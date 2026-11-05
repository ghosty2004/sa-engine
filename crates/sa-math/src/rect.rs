//! `CRect`, ported from the 1.0 US machine code. Field names follow gta-reversed (`left`, `top`,
//! `right`, `bottom` at 0x0/0x4/0x8/0xC); the semantics come from the bytes: the original treats
//! `bottom` (0xC) as the lower and `top` (0x4) as the upper y bound.

use crate::consts as k;
use crate::vector::CVector2D;
use crate::x87::X87;

/// `CRect` (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

const _: () = {
    assert!(core::mem::size_of::<CRect>() == 0x10);
    assert!(core::mem::offset_of!(CRect, top) == 4);
    assert!(core::mem::offset_of!(CRect, right) == 8);
    assert!(core::mem::offset_of!(CRect, bottom) == 0xC);
};

fn ld(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `fld a; fcomp b; fnstsw; test ah, mask` then `je` (`!test_any`) / `jne` / `jp` / `jnp`.
fn cmp(a: X87, b: f32) -> crate::x87::Cmp {
    a.fcom(ld(b))
}

impl CRect {
    /// `CRect::IsFlipped` (0x404190): `left > right || bottom > top` (NaN compares are false).
    pub fn is_flipped(&self) -> bool {
        !cmp(ld(self.left), self.right).test_any(0x41) || !cmp(ld(self.bottom), self.top).test_any(0x41)
    }

    /// `CRect::Restrict(const CRect&)` (0x404200, returns `this`): grow to include `o` (raw copies):
    /// `o.left < left`, `o.right > right`, `o.bottom < bottom`, `o.top > top`, in that order.
    pub fn restrict(&mut self, o: &CRect) {
        if !cmp(ld(o.left), self.left).test_parity(0x05) {
            self.left = o.left;
        }
        if !cmp(ld(o.right), self.right).test_any(0x41) {
            self.right = o.right;
        }
        if !cmp(ld(o.bottom), self.bottom).test_parity(0x05) {
            self.bottom = o.bottom;
        }
        if !cmp(ld(o.top), self.top).test_any(0x41) {
            self.top = o.top;
        }
    }

    /// `CRect::Resize(float x, float y)` (0x404260): `left -= x; right = x + right; bottom -= y;
    /// top = y + top` (memory operand forms as in the original).
    pub fn resize(&mut self, x: f32, y: f32) {
        self.left = ld(self.left).fsub_m(x).to_f32();
        self.right = ld(x).fadd_m(self.right).to_f32();
        self.bottom = ld(self.bottom).fsub_m(y).to_f32();
        self.top = ld(y).fadd_m(self.top).to_f32();
    }

    /// `CRect::IsPointInside(const CVector2D&)` (0x404290): false when `p.x < left`, `p.x > right`,
    /// `p.y < bottom` or `p.y > top` (a NaN coordinate passes every test).
    pub fn is_point_inside(&self, p: &CVector2D) -> bool {
        !(!cmp(ld(p.x), self.left).test_parity(0x05)
            || !cmp(ld(p.x), self.right).test_any(0x41)
            || !cmp(ld(p.y), self.bottom).test_parity(0x05)
            || !cmp(ld(p.y), self.top).test_any(0x41))
    }

    /// `CRect::IsPointInside(const CVector2D&, float tolerance)` (0x4042D0).
    pub fn is_point_inside_tolerance(&self, p: &CVector2D, t: f32) -> bool {
        let px = ld(p.x);
        let py = ld(p.y);
        // (left - t) > p.x, (t + right) < p.x, (bottom - t) > p.y, (t + top) < p.y -> false
        !(!ld(self.left).fsub_m(t).fcom(px).test_any(0x41)
            || !ld(t).fadd_m(self.right).fcom(px).test_parity(0x05)
            || !ld(self.bottom).fsub_m(t).fcom(py).test_any(0x41)
            || !ld(t).fadd_m(self.top).fcom(py).test_parity(0x05))
    }

    /// `CRect::SetFromCenter(float x, float y, float size)` (0x43E020, returns `this`).
    pub fn set_from_center(&mut self, x: f32, y: f32, size: f32) {
        self.left = ld(x).fsub_m(size).to_f32();
        self.bottom = ld(y).fsub_m(size).to_f32();
        self.right = ld(x).fadd_m(size).to_f32();
        self.top = ld(y).fadd_m(size).to_f32();
    }

    /// `CRect::GetCenter(float* x, float* y)` (0x43E050): `((right + left) * 0.5, (bottom + top) * 0.5)`.
    pub fn get_center(&self) -> (f32, f32) {
        let h = f32::from_bits(k::F0_5);
        (
            ld(self.right).fadd_m(self.left).fmul_m(h).to_f32(),
            ld(self.bottom).fadd_m(self.top).fmul_m(h).to_f32(),
        )
    }

    /// `CRect::StretchToPoint(float x, float y)` (0x5327F0, returns `this`): raw copies of `x`/`y` into
    /// `left` if `x < left`, `right` if `x > right`, `bottom` if `y < bottom`, `top` if `y > top`.
    pub fn stretch_to_point(&mut self, x: f32, y: f32) {
        if !cmp(ld(x), self.left).test_parity(0x05) {
            self.left = x;
        }
        if !cmp(ld(x), self.right).test_any(0x41) {
            self.right = x;
        }
        if !cmp(ld(y), self.bottom).test_parity(0x05) {
            self.bottom = y;
        }
        if !cmp(ld(y), self.top).test_any(0x41) {
            self.top = y;
        }
    }
}
