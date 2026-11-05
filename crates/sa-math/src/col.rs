//! Collision primitives of the MATH subsystem (`CBox`, `CColBox`, `CColLine`, `CSphere`, `CColSphere`),
//! ported from the 1.0 US machine code (conventions as in `vector.rs`).

use crate::consts as k;
use crate::vector::{self, CVector};
use crate::x87::X87;

/// `CColSurface` (4 bytes): material, flags, lighting, light (the `Set` functions write the first 3).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CColSurface {
    pub material: u8,
    pub flags: u8,
    pub lighting: u8,
    pub light: u8,
}

/// `CBox` (24 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CBox {
    pub min: CVector,
    pub max: CVector,
}

/// `CColBox` (28 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CColBox {
    pub bx: CBox,
    pub surface: CColSurface,
}

/// `CColLine` (32 bytes): start and end, each padded to 16 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CColLine {
    pub start: CVector,
    pub start_pad: f32,
    pub end: CVector,
    pub end_pad: f32,
}

/// `CSphere` (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CSphere {
    pub center: CVector,
    pub radius: f32,
}

/// `CColSphere` (20 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CColSphere {
    pub sphere: CSphere,
    pub surface: CColSurface,
}

const _: () = {
    use core::mem::{offset_of, size_of};
    assert!(size_of::<CColSurface>() == 4);
    assert!(size_of::<CBox>() == 0x18);
    assert!(offset_of!(CBox, max) == 0xC);
    assert!(size_of::<CColBox>() == 0x1C);
    assert!(offset_of!(CColBox, surface) == 0x18);
    assert!(size_of::<CColLine>() == 0x20);
    assert!(offset_of!(CColLine, end) == 0x10);
    assert!(size_of::<CSphere>() == 0x10);
    assert!(offset_of!(CSphere, radius) == 0xC);
    assert!(size_of::<CColSphere>() == 0x14);
    assert!(offset_of!(CColSphere, surface) == 0x10);
};

fn ld(v: f32) -> X87 {
    X87::from_f32(v)
}

fn mul(a: f32, b: f32) -> X87 {
    ld(a).fmul_m(b)
}

impl CBox {
    /// `CBox::Set(const CVector& min, const CVector& max)` (0x40EDE0): raw copies.
    pub fn set(&mut self, min: &CVector, max: &CVector) {
        self.min = *min;
        self.max = *max;
    }

    /// `CBox::Recalc` (0x40EE20): per axis, when `min > max` swap them (`max = fld min; fstp`, `min` a
    /// raw copy of the old `max`).
    pub fn recalc(&mut self) {
        let sw = |a: &mut f32, b: &mut f32| {
            if !ld(*a).fcom(ld(*b)).test_any(0x41) {
                let old_b = *b;
                *b = ld(*a).to_f32();
                *a = old_b;
            }
        };
        sw(&mut self.min.x, &mut self.max.x);
        sw(&mut self.min.y, &mut self.max.y);
        sw(&mut self.min.z, &mut self.max.z);
    }
}

impl CColBox {
    /// `CColBox::Set(min, max, material, flags, lighting)` (0x40EE70).
    pub fn set(&mut self, min: &CVector, max: &CVector, material: u8, flags: u8, lighting: u8) {
        self.bx.set(min, max);
        self.surface.material = material;
        self.surface.flags = flags;
        self.surface.lighting = lighting;
    }
}

impl CColLine {
    /// `CColLine::Set(const CVector& start, const CVector& end)` (0x40EF10): raw copies (padding kept).
    pub fn set(&mut self, start: &CVector, end: &CVector) {
        self.start = *start;
        self.end = *end;
    }
}

impl CSphere {
    /// `CSphere::Set(float radius, const CVector& center)` (0x40FCF0).
    pub fn set(&mut self, radius: f32, center: &CVector) {
        self.radius = radius;
        self.center = *center;
    }
}

/// `((dz*dz + dx*dx) + dy*dy)` of a difference kept in registers (the intersect tests' order).
fn dist2(dx: X87, dy: X87, dz: X87) -> X87 {
    dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy))
}

impl CColSphere {
    /// `CColSphere::Set(float radius, const CVector& center, material, flags, lighting)` (0x40FD10).
    pub fn set(&mut self, radius: f32, center: &CVector, material: u8, flags: u8, lighting: u8) {
        self.sphere.set(radius, center);
        self.surface.material = material;
        self.surface.flags = flags;
        self.surface.lighting = lighting;
    }

    /// `CColSphere::IntersectPoint(const CVector&)` (0x410040): `r*r > |c - p|^2` (strict; NaN false).
    pub fn intersect_point(&self, p: &CVector) -> bool {
        let c = self.sphere.center;
        let d = dist2(ld(c.x).fsub_m(p.x), ld(c.y).fsub_m(p.y), ld(c.z).fsub_m(p.z));
        let r = ld(self.sphere.radius);
        !r.fmul(r).fcom(d).test_any(0x41)
    }

    /// `CColSphere::IntersectSphere(const CColSphere&)` (0x410090): `(o.r + r)^2 > |c - o.c|^2`.
    pub fn intersect_sphere(&self, o: &CColSphere) -> bool {
        let (c, oc) = (self.sphere.center, o.sphere.center);
        let d = dist2(ld(c.x).fsub_m(oc.x), ld(c.y).fsub_m(oc.y), ld(c.z).fsub_m(oc.z));
        let r = ld(o.sphere.radius).fadd_m(self.sphere.radius);
        !r.fmul(r).fcom(d).test_any(0x41)
    }

    /// `CColSphere::IntersectRay(origin, dir, p1, p2)` (0x40FF20): solves
    /// `SolveQuadratic(1, b, c)` with `b = 2 * (d . dir)` and `c = |d|^2 - r^2` (`d = origin - center`,
    /// both passed as f32) and returns `origin + t * dir` for the two roots. `None` (nothing written)
    /// when there is no root. The z component's product is stored as f32 before the add, x and y are not.
    pub fn intersect_ray(&self, origin: &CVector, dir: &CVector) -> Option<(CVector, CVector)> {
        let c = self.sphere.center;
        let (dx, dy, dz) = (
            ld(origin.x).fsub_m(c.x),
            ld(origin.y).fsub_m(c.y),
            ld(origin.z).fsub_m(c.z),
        );
        let r = ld(self.sphere.radius);
        let b = dx
            .fmul_m(dir.x)
            .fadd(dz.fmul_m(dir.z))
            .fadd(dy.fmul_m(dir.y))
            .fdouble()
            .to_f32();
        let cc = dist2(dx, dy, dz).fsub(r.fmul(r)).to_f32();
        let (t1, t2) = crate::general::solve_quadratic(f32::from_bits(k::F1), b, cc)?;
        let point = |t: f32| {
            let z = mul(t, dir.z).to_f32();
            CVector::new(
                mul(t, dir.x).fadd_m(origin.x).to_f32(),
                mul(t, dir.y).fadd_m(origin.y).to_f32(),
                ld(z).fadd_m(origin.z).to_f32(),
            )
        };
        Some((point(t1), point(t2)))
    }

    /// `CColSphere::IntersectEdge(start, end, p1, p2)` (0x4100E0). Returns `None` (nothing written) or
    /// `(p1, p2)`: with `dir = normalise(end - start)` and the roots `t1 = (s - b) / 2`,
    /// `t2 = (-b - s) / 2` of the ray equation, the original rejects `t2 > |end - start|` and `t1 < 0`;
    /// then `p2 = (t1 < len) ? start + t1*dir : end` and `p1 = (t2 > 0) ? start + t2*dir : start`.
    pub fn intersect_edge(&self, start: &CVector, end: &CVector) -> Option<(CVector, CVector)> {
        let c = self.sphere.center;
        let rf = self.sphere.radius;
        let o = CVector::new(
            ld(start.x).fsub_m(c.x).to_f32(),
            ld(start.y).fsub_m(c.y).to_f32(),
            ld(start.z).fsub_m(c.z).to_f32(),
        );
        let dzv = ld(end.z).fsub_m(start.z);
        let mut dir = CVector::new(
            ld(end.x).fsub_m(start.x).to_f32(),
            ld(end.y).fsub_m(start.y).to_f32(),
            dzv.to_f32(),
        );
        let len = dzv
            .fmul_m(dir.z)
            .fadd(mul(dir.y, dir.y))
            .fadd(mul(dir.x, dir.x))
            .fsqrt()
            .to_f32();
        let [nx, ny, nz] = vector::normalise([dir.x, dir.y, dir.z]);
        dir = CVector::new(nx, ny, nz);
        let b = mul(dir.z, o.z)
            .fadd(mul(dir.x, o.x))
            .fadd(mul(dir.y, o.y))
            .fdouble();
        let oo = mul(o.z, o.z).fadd(mul(o.x, o.x)).fadd(mul(o.y, o.y));
        let disc = b.fmul(b).fsub(oo.fsub(mul(rf, rf)).fmul_m(f32::from_bits(k::F4)));
        if !disc.fcom(X87::zero()).test_parity(0x05) {
            return None;
        }
        let s = disc.fsqrt();
        let h = f32::from_bits(k::F0_5);
        let t1 = s.fsub(b).fmul_m(h).to_f32();
        let t2 = b.fchs().fsub(s).fmul_m(h).to_f32();
        if !ld(t2).fcom(ld(len)).test_any(0x41) || !ld(t1).fcom(X87::zero()).test_parity(0x05) {
            return None;
        }
        let point = |t: f32| {
            let z = mul(t, dir.z).to_f32();
            CVector::new(
                mul(t, dir.x).fadd_m(start.x).to_f32(),
                mul(t, dir.y).fadd_m(start.y).to_f32(),
                ld(z).fadd_m(start.z).to_f32(),
            )
        };
        let p2 = if ld(t1).fcom(ld(len)).test_parity(0x05) {
            *end
        } else {
            point(t1)
        };
        let p1 = if ld(t2).fcom(X87::zero()).test_any(0x41) {
            *start
        } else {
            point(t2)
        };
        Some((p1, p2))
    }
}
