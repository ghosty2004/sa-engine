//! WRITER-MVP-D (helper-world) COLL ports, batch `mvpd-coll-1`: `CCollision::TestLineTriangle` and
//! `CCollision::IsStoredPolyStillValidVerticalLine`. Ported from the 1.0 US bytes (the exe wins over
//! gta-reversed).
//!
//! Both intersect a line with a triangle's plane (`t = (d - n.s) / (n.(e - s))`, the hit point stored as f32)
//! and then run the same inlined 2D point-in-triangle test on the plane's dominant axis pair, selected by a
//! six-way switch (`dir` 0..5: +-x, +-y, +-z). The switch cases only choose which coordinates go where; the
//! shared tail is [`tri_2d_inside`].
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::world::common::{g, rd, wr, x, xi};

orig! {
    /// `CVector::CrossProduct(CVector* out, const CVector&, const CVector&)` (0x59C730, cdecl); returns `out`
    fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
}

/// The values the six-way switch puts in place for the shared 2D test: the first vertex's two coordinates in
/// ST0/ST1 (`a`, `b`) and six stack slots.
struct Tri2d {
    a: X87,
    b: X87,
    c0: f32,
    c4: f32,
    c8: f32,
    cc: f32,
    c2c: f32,
    c38: f32,
}

/// The switch on the dominant axis: cases 0/1 project on (z, y), 2/3 on (x, z), 4/5 on (y, x); odd cases swap
/// the second and third vertex. `p` is the hit point, `v` the three vertices.
fn axis_case(dir: u32, v: &[[f32; 3]; 3], p: &[f32; 3]) -> Tri2d {
    let (u, w) = match dir / 2 {
        0 => (2, 1),
        1 => (0, 2),
        _ => (1, 0),
    };
    let (v1, v2) = if dir & 1 == 0 { (v[1], v[2]) } else { (v[2], v[1]) };
    Tri2d { a: x(v[0][u]), b: x(v[0][w]), c0: p[u], c4: v1[w], c8: p[w], cc: v1[u], c2c: v2[u], c38: v2[w] }
}

/// The shared tail (0x41403E / 0x415158): three edge-side tests in x87 order. False when
/// `(c38 - b)(c0 - a) - (c2c - a)(c8 - b) < 0`, when `(c4 - b)(c0 - a) - (cc - a)(c8 - b) > 0`, or when
/// `(c0 - c2c)(c4 - c38) - (c8 - c38)(cc - c2c) < 0` (ordered compares; NaN passes every test).
fn tri_2d_inside(t: &Tri2d) -> bool {
    let zero = g::<f32>(0x858B50).read();
    let ca = x(t.c0).fsub(t.a);
    let q = x(t.c8).fsub(t.b);
    let r1 = x(t.c38).fsub(t.b).fmul(ca);
    let r2 = x(t.c2c).fsub(t.a).fmul(q);
    if !r1.fsub(r2).fcom(x(zero)).test_parity(5) {
        return false;
    }
    let m1 = x(t.c4).fsub(t.b).fmul(ca);
    let m2 = x(t.cc).fsub(t.a).fmul(q);
    if !m1.fsub(m2).fcom(x(zero)).test_any(0x41) {
        return false;
    }
    let l = x(t.c4).fsub_m(t.c38).fmul(x(t.c0).fsub_m(t.c2c));
    let r = x(t.cc).fsub_m(t.c2c).fmul(x(t.c8).fsub_m(t.c38));
    l.fsub(r).fcom(x(zero)).test_parity(5)
}

/// The line-plane intersection both functions share: with `n` the normal (`nx` kept in a register, `ny`/`nz`
/// f32), `pxs = f32(nx*sx)`, `pys = f32(ny*sy)`, `pz` = `nz*sz` (f32 in `TestLineTriangle`, a register value in
/// the stored-poly test), `t = f32((d - pxs - pys - pz) / ((dz*nz + dy*ny) + dx*nx))` with `dz` stored as f32,
/// and the point `(f32(f32(dx*t) + sx), f32(dy*t + sy), f32(dz*t + sz))`. Returns `(t, point)`.
#[allow(clippy::too_many_arguments)]
fn intersect(nx: X87, ny: f32, nz: f32, d: f32, s: [f32; 3], e: [f32; 3], pxs: f32, pys: f32, pz: X87) -> (f32, [f32; 3]) {
    let dx = x(e[0]).fsub_m(s[0]);
    let dy = x(e[1]).fsub_m(s[1]);
    let dz = x(e[2]).fsub_m(s[2]).to_f32();
    let num = x(d).fsub_m(pxs).fsub_m(pys).fsub(pz);
    let den = x(dz).fmul_m(nz).fadd(dy.fmul_m(ny)).fadd(dx.fmul(nx));
    let t = num.fdiv(den).to_f32();
    let dxt = dx.fmul_m(t).to_f32();
    let dyt = dy.fmul_m(t);
    let dzt = x(dz).fmul_m(t);
    let p = [x(dxt).fadd_m(s[0]).to_f32(), dyt.fadd_m(s[1]).to_f32(), dzt.fadd_m(s[2]).to_f32()];
    (t, p)
}

/// `CCollision::TestLineTriangle(const CColLine&, const CompressedVector*, const CColTriangle&, const
/// CColTrianglePlane&)` (0x413AC0, cdecl, bool in AL): the compressed plane (normal `int16 * 1/4096`, distance
/// `int16 * 1/128`) must separate the line's start (+0) and end (+0x10) (`A * B > 0` or NaN: false); then the
/// intersection point is projected on the plane's dominant axis pair (`dir`, plane +8) and tested against the
/// triangle's three compressed vertices (`int16 * 1/128`, indices at tri +0/+2/+4, 6-byte vertices). Kept
/// original details: the start dot products and `t` are kept in the caller-owned line / plane argument slots
/// (not modelled); a `dir` above 5 tests against two never-written locals (modelled as 0).
#[hook(addr = 0x413AC0, conv = cdecl, name = "CCollision::TestLineTriangle", expect = "83 EC 28 8B 54 24 38",
       unverified = "mvpd-coll-1", route = "L2", hazards = "stack-residue,uninit-locals",
       note = "overwrites its line/plane argument slots; dir > 5 reads two uninitialised locals")]
pub fn test_line_triangle(line: GPtr<u8>, verts: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>) -> u8 {
    let k1 = g::<f32>(0x858BFC).read();
    let k2 = g::<f32>(0x858B88).read();
    let zero = g::<f32>(0x858B50).read();
    let nx = xi(rd::<i16>(plane, 0) as i32).fmul_m(k1);
    let ny = xi(rd::<i16>(plane, 2) as i32).fmul_m(k1).to_f32();
    let nz = xi(rd::<i16>(plane, 4) as i32).fmul_m(k1).to_f32();
    let d = xi(rd::<i16>(plane, 6) as i32).fmul_m(k2).to_f32();
    let s = [rd::<f32>(line, 0), rd::<f32>(line, 4), rd::<f32>(line, 8)];
    let e = [rd::<f32>(line, 0x10), rd::<f32>(line, 0x14), rd::<f32>(line, 0x18)];
    let pzs = x(nz).fmul_m(s[2]).to_f32();
    let pys = x(ny).fmul_m(s[1]).to_f32();
    let pxs = nx.fmul_m(s[0]).to_f32();
    let a = x(nz).fmul_m(e[2]).fadd(x(ny).fmul_m(e[1])).fadd(nx.fmul_m(e[0])).fsub_m(d);
    let b = x(pxs).fadd_m(pys).fadd_m(pzs).fsub_m(d);
    if a.fmul(b).fcom(x(zero)).test_parity(0x41) {
        return 0;
    }
    let dir = rd::<u8>(plane, 8) as u32;
    let (t, p) = intersect(nx, ny, nz, d, s, e, pxs, pys, x(pzs));
    let t2 = if dir <= 5 {
        let vert = |i: isize| {
            let idx = rd::<u16>(tri, 2 * i) as isize;
            let c = |k: isize| xi(rd::<i16>(verts, 6 * idx + k) as i32).fmul_m(k2).to_f32();
            [c(0), c(2), c(4)]
        };
        axis_case(dir, &[vert(0), vert(1), vert(2)], &p)
    } else {
        Tri2d { a: x(t), b: x(t), c0: 0.0, c4: 0.0, c8: pzs, cc: pys, c2c: pxs, c38: t }
    };
    tri_2d_inside(&t2) as u8
}

/// `|v|` as the original computes it (`fchs` when `v < 0`, ordered).
fn fabs_cmp(v: f32) -> X87 {
    if x(v).fcom(x(g::<f32>(0x858B50).read())).test_parity(5) { x(v) } else { x(v).fchs() }
}

/// `CCollision::IsStoredPolyStillValidVerticalLine(const CVector& pos, float z, CColPoint&, CStoredCollPoly*)`
/// (0x414D70, cdecl, bool in AL): with a valid poly (+0x24), its normal `Normalise(CrossProduct(v2 - v0, v1 -
/// v0))` and `d = n.v0` define the plane; the vertical line from `pos` to `(pos.x, pos.y, z)` must cross it and
/// the hit point must lie in the triangle (dominant axis: x, then y, else z, sign picks the even/odd case).
/// Success stores the point in the colpoint (+0); any failure after the validity check clears the poly's valid
/// flag. Kept original detail: `d` and `t` are kept in the caller-owned poly argument slot (not modelled).
#[hook(addr = 0x414D70, conv = cdecl, name = "CCollision::IsStoredPolyStillValidVerticalLine", expect = "83 EC 64 53 56",
       unverified = "mvpd-coll-1", route = "L2", hazards = "stack-residue",
       note = "overwrites its poly argument slot with the plane distance and t")]
pub fn is_stored_poly_still_valid_vertical_line(pos: GPtr<u8>, z: f32, cp: GPtr<u8>, poly: GPtr<u8>) -> u8 {
    if rd::<u8>(poly, 0x24) == 0 {
        return 0;
    }
    let pv = |i: isize| [rd::<f32>(poly, 12 * i), rd::<f32>(poly, 12 * i + 4), rd::<f32>(poly, 12 * i + 8)];
    let v = [pv(0), pv(1), pv(2)];
    let s = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
    let e = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), z];
    let sub = |a: [f32; 3], b: [f32; 3]| [x(a[0]).fsub_m(b[0]).to_f32(), x(a[1]).fsub_m(b[1]).to_f32(), x(a[2]).fsub_m(b[2]).to_f32()];
    let mut e1 = sub(v[1], v[0]);
    let mut e2 = sub(v[2], v[0]);
    let mut out = [0f32; 3];
    let r = cross_product(GPtr::from_raw(&mut out).cast(), GPtr::from_raw(&mut e2).cast(), GPtr::from_raw(&mut e1).cast());
    let mut n = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    vector_normalise(GPtr::from_raw(&mut n).cast());
    let d = x(n[2]).fmul_m(v[0][2]).fadd(x(n[1]).fmul_m(v[0][1])).fadd(x(n[0]).fmul_m(v[0][0])).to_f32();
    let zero = x(g::<f32>(0x858B50).read());
    let le = |p: X87, q: X87| p.fcom(q).test_any(0x41);
    let dir: u32 = if !le(fabs_cmp(n[0]), fabs_cmp(n[1])) && !le(fabs_cmp(n[0]), fabs_cmp(n[2])) {
        if x(n[0]).fcom(zero).test_any(0x41) { 1 } else { 0 }
    } else if !le(fabs_cmp(n[1]), fabs_cmp(n[2])) {
        if x(n[1]).fcom(zero).test_any(0x41) { 3 } else { 2 }
    } else if x(n[2]).fcom(zero).test_any(0x41) {
        5
    } else {
        4
    };
    let pz = x(s[2]).fmul_m(n[2]);
    let pys = x(s[1]).fmul_m(n[1]).to_f32();
    let pxs = x(s[0]).fmul_m(n[0]).to_f32();
    let a = x(e[1]).fmul_m(n[1]).fadd(x(e[0]).fmul_m(n[0])).fadd(x(n[2]).fmul_m(e[2])).fsub_m(d);
    let b = x(pxs).fadd_m(pys).fadd(pz).fsub_m(d);
    let inside = if a.fmul(b).fcom(zero).test_parity(0x41) {
        false
    } else {
        let (_, p) = intersect(x(n[0]), n[1], n[2], d, s, e, pxs, pys, pz);
        if tri_2d_inside(&axis_case(dir, &v, &p)) {
            wr::<f32>(cp, 0, p[0]);
            wr::<f32>(cp, 4, p[1]);
            wr::<f32>(cp, 8, p[2]);
            return 1;
        }
        false
    };
    if !inside {
        wr::<u8>(poly, 0x24, 0);
    }
    0
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_TEST_LINE_TRIANGLE, HOOK_IS_STORED_POLY_STILL_VALID_VERTICAL_LINE];
