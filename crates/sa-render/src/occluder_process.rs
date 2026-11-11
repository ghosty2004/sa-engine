//! WRITER-RENDER-BIG-2: `COccluder::ProcessOneOccluder` (0x71E5D0). Ported from the 1.0 US bytes;
//! gta-reversed `Occluder.cpp` as reading aid (its axes differ from the bytes, see below). x87 arithmetic
//! follows the original's instruction order on the `sa_math::x87::X87` model, including which partial
//! sums the original stores as f32 and which it keeps on the stack.
//!
//! `COccluder` (0x12 bytes): +0 centre (3 x int16, quarter units), +6 / +8 / +0xA the three sizes (int16,
//! quarter units), +0xC / +0xD / +0xE the z / y / x rotation (bytes, 2pi/256 units).
//! `CActiveOccluder`: +0x78 distance to the camera (u16), +0x7A lines used, +0x7B face count, +0x7C face
//! normals (3 x CVector), +0xA0 face offsets (3 floats).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne, jp, k, rd, wr, x, xi};

orig! {
    /// `Common::CalcScreenCoors(const CVector&, CVector*, float*, float*)` (0x71da00)
    fn calc_screen_coors_scaled(input: GPtr<[f32; 3]>, out: GPtr<[f32; 3]>, sx: GPtr<f32>, sy: GPtr<f32>) -> bool = cdecl 0x71da00 => sa_core::common::calc_screen_coors_scaled;
    /// `CMatrix::SetRotateX(float)` (0x59b060)
    fn matrix_set_rotate_x(m: GPtr<u8>, a: f32) = thiscall 0x59b060 => unported;
    /// `CMatrix::SetRotateY(float)` (0x59b0a0)
    fn matrix_set_rotate_y(m: GPtr<u8>, a: f32) = thiscall 0x59b0a0 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59b0e0)
    fn matrix_set_rotate_z(m: GPtr<u8>, a: f32) = thiscall 0x59b0e0 => unported;
    /// `operator*(const CMatrix&, const CMatrix&)` (0x59be30, result through a hidden pointer)
    fn matrix_mul(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59be30 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59bbc0)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59bbc0 => unported;
    /// `CMatrix::~CMatrix()` (0x59acd0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59acd0 => sa_world::world::mvpa_math::matrix_dtor;
    /// `operator*(const CMatrix&, const CVector&)` (0x59c890, result through a hidden pointer)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59c890 => unported;
    /// `CVector::Normalise()` (0x59c910)
    fn normalise(v: GPtr<CVector>) = thiscall 0x59c910 => sa_world::collision::data::normalise_port;
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59c730)
    fn cross_product(out: GPtr<CVector>, a: GPtr<CVector>, b: GPtr<CVector>) -> GPtr<CVector> = cdecl 0x59c730 => sa_world::collision::data::cross_product_port;
    /// `COccluder::ProcessLineSegment(int corner1, int corner2, CActiveOccluder*)` (0x71e130)
    fn process_line_segment(this: GPtr<u8>, c1: i32, c2: i32, occl: GPtr<u8>) -> u8 = thiscall 0x71e130 => crate::occlusion::process_line_segment;
}

/// `COcclusion::CenterOnScreen`, `OccluderCoors[8]`, `OccluderCoorsOnScreen[8]`, `OccluderCoorsValid[8]`.
const CENTER_ON_SCREEN: usize = 0xc7_9940;
const OCCLUDER_COORS: usize = 0xc7_98e0;
const OCCLUDER_COORS_ON_SCREEN: usize = 0xc7_9950;
const OCCLUDER_COORS_VALID: usize = 0xc7_3cb0;
/// `MaxYInOccluder`, `MinYInOccluder`, `MaxXInOccluder`, `MinXInOccluder`.
const MAX_Y: usize = 0xc7_3ca0;
const MIN_Y: usize = 0xc7_3ca4;
const MAX_X: usize = 0xc7_3ca8;
const MIN_X: usize = 0xc7_3cac;
/// `TheCamera.m_matrix` (pointer) and `TheCamera.m_placement` position.
const CAMERA_MATRIX: usize = 0xb6_f03c;
const CAMERA_PLACEMENT_POS: usize = 0xb6_f02c;

type V3 = [f32; 3];

fn f(v: X87) -> f32 {
    v.to_f32()
}

/// A local `CMatrix` (0x48 bytes) with the inlined constructor's `m_pAttachMatrix = 0`, `m_bOwnsAttachedMatrix = 0`.
fn new_matrix() -> [u32; 18] {
    [0; 18]
}

fn mp(m: &mut [u32; 18]) -> GPtr<u8> {
    GPtr::<u32>::from_raw(m.as_mut_ptr()).cast()
}

/// `D * v` through `operator*(CMatrix, CVector)` into a scratch vector.
fn transform(d: &mut [u32; 18], v: V3) -> V3 {
    let mut vin = v;
    let mut out = [0f32; 3];
    let r = matrix_mul_vec(
        GPtr::<f32>::from_raw(out.as_mut_ptr()).cast(),
        mp(d),
        GPtr::<f32>::from_raw(vin.as_mut_ptr()).cast(),
    );
    [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)]
}

fn size_q(this: GPtr<u8>, off: isize) -> i32 {
    i32::from(rd::<u16>(this, off) as i16)
}

/// `CalcScreenCoors` of every corner into `OccluderCoorsOnScreen` / `OccluderCoorsValid`.
fn project_corners(n: usize) {
    for i in 0..n {
        let c = OCCLUDER_COORS + 12 * i;
        let mut v = [g::<f32>(c).read(), g::<f32>(c + 4).read(), g::<f32>(c + 8).read()];
        let (mut w, mut h) = (0f32, 0f32);
        let ok = calc_screen_coors_scaled(
            GPtr::from_raw(&mut v),
            GPtr::from_addr(OCCLUDER_COORS_ON_SCREEN + 12 * i),
            GPtr::from_raw(&mut w),
            GPtr::from_raw(&mut h),
        );
        g::<u8>(OCCLUDER_COORS_VALID + i).write(u8::from(ok));
    }
}

fn set_corner(i: usize, v: V3) {
    for (j, c) in v.into_iter().enumerate() {
        g::<f32>(OCCLUDER_COORS + 12 * i + 4 * j).write(c);
    }
}

/// `(maxA - minA)` against `screen * frac` (`fcompp`, `test ah, 0x41; je` fails when the screen part is
/// greater).
fn big_enough(max: usize, min: usize, screen: usize, frac: f32) -> bool {
    let diff = x(g::<f32>(max).read()).fsub_m(g::<f32>(min).read());
    let s = xi(g::<i32>(screen).read()).fmul_m(frac);
    jne(s, diff, 0x41)
}

/// `COccluder::ProcessOneOccluder(CActiveOccluder* out)` (0x71E5D0). Lines used = 0; the centre (int16 /
/// 4) through `CalcScreenCoors` into `CenterOnScreen` (false: 0); z < -150 or > 300: 0. Distance
/// `ftol(z - |size / 4|)` (u16). The transform `D = (rotY * rotX) * rotZ` (`SetRotateX/Y/Z` of the
/// angle bytes * 2pi/256, two `operator*`, `operator=`, the temporaries destroyed). Min / max reset to
/// +/-999999.88.
///
/// All three sizes non-zero (a box): `V1 = D * (0, size6 / 8, 0)`, `V2 = D * (size8 / 8, 0, 0)`, `V3 = D *
/// (0, 0, sizeA / 8)` (the bytes' axes: gta-reversed puts size6 on x and size8 on y); directions `V1,
/// -V1, V2, -V2, V3, -V3`; face `i` is seen when `(centre + d_i - camera) . d_i < 0`; the 8 corners
/// `centre +/- V1 +/- V2 +/- V3` and their screen coordinates; the 12 silhouette edges through
/// `ProcessLineSegment` (any true: 0); the extent must be at least 15% of the width and 10% of the
/// height; the seen faces (normalised direction, offset `(centre + d) . n`) are stored; 1.
///
/// One size zero (a plane): `r, u` = (size8 on x, sizeA on z) when size6 is 0, (size6 on y, sizeA on z)
/// when size8 is 0, (size6 on y, size8 on x) when sizeA is 0; corners `C+r+u, C-r+u, C-r-u, C+r-u`;
/// edges 0-1, 1-2, 2-3, 3-0; extent >= 10% / 7%; one face `normalise(r x u)` (gta-reversed does not
/// normalise) with offset `n . centre`; 1. The local matrices are destroyed on every path past the
/// distance check.
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x71e5d0, conv = thiscall, name = "COccluder::ProcessOneOccluder", this = "COccluder", expect = "6A FF 68 6F 8C 84 00",
       route = "L2", hazards = "x87-transcendental,uninit-locals",
       recs = "grove-walk,driving",
       note = "FSQRT; SEH frame for the local CMatrix objects (not reproduced: nothing in them throws); the planar path with no zero size (unreachable) would read uninitialised vectors (the port uses zeros)")]
pub fn process_one_occluder(this: GPtr<u8>, out: GPtr<u8>) -> u8 {
    wr::<u8>(out, 0x7a, 0);
    let q = k(0x3e80_0000);
    let cz = f(xi(size_q(this, 4)).fmul_m(q));
    let cy = f(xi(size_q(this, 2)).fmul_m(q));
    let cx = f(xi(size_q(this, 0)).fmul_m(q));
    let mut c = [cx, cy, cz];
    let (mut w, mut h) = (0f32, 0f32);
    if !calc_screen_coors_scaled(GPtr::from_raw(&mut c), GPtr::from_addr(CENTER_ON_SCREEN), GPtr::from_raw(&mut w), GPtr::from_raw(&mut h)) {
        return 0;
    }
    let sz_ = g::<f32>(CENTER_ON_SCREEN + 8).read();
    // -150 (0x86D1D8), 300 (0x858FD8)
    if !jp(x(sz_), x(k(0xc316_0000)), 0x5) || !jne(x(sz_), x(k(0x4396_0000)), 0x41) {
        return 0;
    }
    let a6 = xi(size_q(this, 6)).fmul_m(q);
    let a8 = xi(size_q(this, 8)).fmul_m(q);
    let aa = xi(size_q(this, 0xa)).fmul_m(q);
    let mag = a6.fmul(a6).fadd(a8.fmul(a8)).fadd(aa.fmul(aa)).fsqrt();
    wr::<u16>(out, 0x78, ftol32(mag.fsubr_m(g::<f32>(CENTER_ON_SCREEN + 8).read())) as u16);

    // 2pi / 256 (0x859BBC)
    let ang = |off: isize| f(xi(i32::from(rd::<u8>(this, off))).fmul_m(k(0x3cc9_0fdb)));
    let mut ma = new_matrix();
    let mut mb = new_matrix();
    let mut mc = new_matrix();
    let mut md = new_matrix();
    matrix_set_rotate_x(mp(&mut ma), ang(0xe));
    matrix_set_rotate_y(mp(&mut mb), ang(0xd));
    matrix_set_rotate_z(mp(&mut mc), ang(0xc));
    let mut t1 = new_matrix();
    let mut t2 = new_matrix();
    let r1 = matrix_mul(mp(&mut t1), mp(&mut mb), mp(&mut ma));
    let r2 = matrix_mul(mp(&mut t2), r1, mp(&mut mc));
    matrix_assign(mp(&mut md), r2);
    matrix_dtor(mp(&mut t2));
    matrix_dtor(mp(&mut t1));
    g::<u32>(MIN_Y).write(0x4974_23fe);
    g::<u32>(MIN_X).write(0x4974_23fe);
    g::<u32>(MAX_Y).write(0xc974_23fe);
    g::<u32>(MAX_X).write(0xc974_23fe);

    let zero = |v: X87| !jp(v, x(0.0), 0x44);
    let ok = if zero(a6) || zero(a8) || zero(aa) {
        planar(this, out, &mut md, c)
    } else {
        boxed(this, out, &mut md, c)
    };
    matrix_dtor(mp(&mut md));
    matrix_dtor(mp(&mut mc));
    matrix_dtor(mp(&mut mb));
    matrix_dtor(mp(&mut ma));
    u8::from(ok)
}

/// The box path (0x71E8C1..0x71F425).
fn boxed(this: GPtr<u8>, out: GPtr<u8>, md: &mut [u32; 18], c: V3) -> bool {
    // 1/8 (0x858C48)
    let e = k(0x3e00_0000);
    let s6 = f(xi(size_q(this, 6)));
    let v1 = transform(md, [0.0, f(x(s6).fmul_m(e)), 0.0]);
    let v2 = transform(md, [f(xi(size_q(this, 8)).fmul_m(e)), 0.0, 0.0]);
    let v3 = transform(md, [0.0, 0.0, f(xi(size_q(this, 0xa)).fmul_m(e))]);
    let neg = |v: V3| [f(x(v[0]).fchs()), f(x(v[1]).fchs()), f(x(v[2]).fchs())];
    let dirs = [v1, neg(v1), v2, neg(v2), v3, neg(v3)];
    let m = g::<GPtr<u8>>(CAMERA_MATRIX).read();
    let cp = if m.is_null() { GPtr::<u8>::from_addr(CAMERA_PLACEMENT_POS) } else { m.byte_add(0x30) };
    let cam = [rd::<f32>(cp, 0), rd::<f32>(cp, 4), rd::<f32>(cp, 8)];
    let lt0 = |v: X87| !jp(v, x(0.0), 0x5);
    // patterns A (V1, V2), B (-V1, -V2), C (V3), D (-V3): the bytes' summation orders
    let pat_a = |d: V3| {
        let dx = f(x(d[0]).fadd_m(c[0]).fsub_m(cam[0]));
        let dy = x(c[1]).fadd_m(d[1]).fsub_m(cam[1]);
        let dz = x(f(x(c[2]).fadd_m(d[2]))).fsub_m(cam[2]);
        lt0(x(dx).fmul_m(d[0]).fadd(dy.fmul_m(d[1])).fadd(dz.fmul_m(d[2])))
    };
    let pat_b = |d: V3| {
        let dx = f(x(d[0]).fadd_m(c[0]).fsub_m(cam[0]));
        let dz = x(f(x(c[2]).fadd_m(d[2]))).fsub_m(cam[2]);
        let dy = x(c[1]).fadd_m(d[1]).fsub_m(cam[1]);
        lt0(x(dx).fmul_m(d[0]).fadd(dz.fmul_m(d[2])).fadd(dy.fmul_m(d[1])))
    };
    let os = [
        pat_a(dirs[0]),
        pat_b(dirs[1]),
        pat_a(dirs[2]),
        pat_b(dirs[3]),
        {
            let d = dirs[4];
            let dz = x(f(x(c[2]).fadd_m(d[2]))).fsub_m(cam[2]);
            let dy = x(c[1]).fadd_m(d[1]).fsub_m(cam[1]);
            let dx = f(x(c[0]).fadd_m(d[0]).fsub_m(cam[0]));
            lt0(x(d[2]).fmul(dz).fadd(dy.fmul_m(d[1])).fadd(x(dx).fmul_m(d[0])))
        },
        {
            let d = dirs[5];
            let sx = f(x(d[0]).fadd_m(c[0]));
            let dz = f(x(c[2]).fadd_m(d[2]).fsub_m(cam[2]));
            let dy = x(c[1]).fadd_m(d[1]).fsub_m(cam[1]);
            lt0(x(d[2]).fmul_m(dz).fadd(x(d[1]).fmul(dy)).fadd(x(sx).fsub_m(cam[0]).fmul_m(d[0])))
        },
    ];

    let tx = f(x(v1[0]).fadd_m(c[0]));
    let ty = f(x(c[1]).fadd_m(v1[1]));
    let tz = f(x(c[2]).fadd_m(v1[2]));
    let bx = f(x(c[0]).fsub_m(v1[0]));
    let by = x(c[1]).fsub_m(v1[1]);
    let bz = f(x(c[2]).fsub_m(v1[2]));
    set_corner(0, [
        f(x(f(x(tx).fadd_m(v2[0]))).fadd_m(v3[0])),
        f(x(v3[1]).fadd_m(f(x(v2[1]).fadd_m(ty)))),
        f(x(v3[2]).fadd_m(f(x(v2[2]).fadd_m(tz)))),
    ]);
    set_corner(1, [
        f(x(f(x(c[0]).fsub_m(v1[0]).fadd_m(v2[0]))).fadd_m(v3[0])),
        f(x(v3[1]).fadd_m(f(x(v2[1]).fadd(by)))),
        f(x(v3[2]).fadd_m(f(x(v2[2]).fadd_m(bz)))),
    ]);
    set_corner(2, [
        f(x(f(x(tx).fsub_m(v2[0]))).fadd_m(v3[0])),
        f(x(v3[1]).fadd_m(f(x(ty).fsub_m(v2[1])))),
        f(x(v3[2]).fadd_m(f(x(tz).fsub_m(v2[2])))),
    ]);
    set_corner(3, [
        f(x(f(x(bx).fsub_m(v2[0]))).fadd_m(v3[0])),
        f(x(v3[1]).fadd_m(f(by.fsub_m(v2[1])))),
        f(x(v3[2]).fadd_m(f(x(c[2]).fsub_m(v1[2]).fsub_m(v2[2])))),
    ]);
    set_corner(4, [
        f(x(f(x(tx).fadd_m(v2[0]))).fsub_m(v3[0])),
        f(x(f(x(v2[1]).fadd_m(ty))).fsub_m(v3[1])),
        f(x(f(x(v2[2]).fadd_m(tz))).fsub_m(v3[2])),
    ]);
    set_corner(5, [
        f(x(f(x(bx).fadd_m(v2[0]))).fsub_m(v3[0])),
        f(x(f(x(v2[1]).fadd(by))).fsub_m(v3[1])),
        f(x(f(x(v2[2]).fadd(x(c[2]).fsub_m(v1[2])))).fsub_m(v3[2])),
    ]);
    set_corner(6, [
        f(x(f(x(tx).fsub_m(v2[0]))).fsub_m(v3[0])),
        f(x(f(x(ty).fsub_m(v2[1]))).fsub_m(v3[1])),
        f(x(f(x(tz).fsub_m(v2[2]))).fsub_m(v3[2])),
    ]);
    set_corner(7, [
        f(x(f(x(bx).fsub_m(v2[0]))).fsub_m(v3[0])),
        f(x(f(by.fsub_m(v2[1]))).fsub_m(v3[1])),
        f(x(f(x(bz).fsub_m(v2[2]))).fsub_m(v3[2])),
    ]);
    project_corners(8);

    let edges: [(usize, usize, i32, i32); 12] = [
        (0, 2, 0, 4), (0, 3, 2, 6), (0, 4, 0, 2), (0, 5, 4, 6),
        (1, 2, 1, 5), (1, 3, 3, 7), (1, 4, 1, 3), (1, 5, 5, 7),
        (4, 2, 0, 1), (4, 3, 2, 3), (5, 3, 6, 7), (5, 2, 4, 5),
    ];
    for (a, b, c1, c2) in edges {
        if os[a] != os[b] && process_line_segment(this, c1, c2, out) != 0 {
            return false;
        }
    }
    // 0.15 (0x858FCC), 0.1 (0x858B1C)
    if !big_enough(MAX_X, MIN_X, 0xc1_7044, k(0x3e19_999a)) || !big_enough(MAX_Y, MIN_Y, 0xc1_7048, k(0x3dcc_cccd)) {
        return false;
    }
    wr::<u8>(out, 0x7b, 0);
    for (i, d) in dirs.iter().enumerate() {
        if !os[i] {
            continue;
        }
        let cdz = f(x(d[2]).fadd_m(c[2]));
        let cdy = f(x(d[1]).fadd_m(c[1]));
        let cdx = f(x(d[0]).fadd_m(c[0]));
        let mut n = *d;
        normalise(GPtr::<f32>::from_raw(n.as_mut_ptr()).cast());
        let nf = i32::from(rd::<u8>(out, 0x7b) as i8);
        let slot = out.byte_add(0x7c + 12 * nf as isize);
        wr::<f32>(slot, 0, n[0]);
        wr::<f32>(slot, 4, n[1]);
        wr::<f32>(slot, 8, n[2]);
        let nf = i32::from(rd::<u8>(out, 0x7b) as i8);
        let off = x(cdx).fmul_m(n[0]).fadd(x(n[1]).fmul_m(cdy)).fadd(x(n[2]).fmul_m(cdz));
        wr::<f32>(out, 0xa0 + 4 * nf as isize, off.to_f32());
        wr::<u8>(out, 0x7b, rd::<u8>(out, 0x7b).wrapping_add(1));
    }
    true
}

/// The planar path (0x71F42A..0x71F88C).
fn planar(this: GPtr<u8>, out: GPtr<u8>, md: &mut [u32; 18], c: V3) -> bool {
    let e = k(0x3e00_0000);
    let q = k(0x3e80_0000);
    let s6f = f(xi(size_q(this, 6)));
    let zero = |off: isize| !jp(xi(size_q(this, off)).fmul_m(q), x(0.0), 0x44);
    let (r, u) = if zero(6) {
        let r = transform(md, [f(xi(size_q(this, 8)).fmul_m(e)), 0.0, 0.0]);
        (r, transform(md, [0.0, 0.0, f(xi(size_q(this, 0xa)).fmul_m(e))]))
    } else if zero(8) {
        let r = transform(md, [0.0, f(x(s6f).fmul_m(e)), 0.0]);
        (r, transform(md, [0.0, 0.0, f(xi(size_q(this, 0xa)).fmul_m(e))]))
    } else if zero(0xa) {
        let r = transform(md, [0.0, f(x(s6f).fmul_m(e)), 0.0]);
        (r, transform(md, [f(xi(size_q(this, 8)).fmul_m(e)), 0.0, 0.0]))
    } else {
        // unreachable (one size is zero here); the original would read uninitialised locals
        ([0.0; 3], [0.0; 3])
    };
    let az = f(x(c[2]).fadd_m(r[2]));
    let ay = f(x(c[1]).fadd_m(r[1]));
    let ax = f(x(r[0]).fadd_m(c[0]));
    set_corner(0, [f(x(ax).fadd_m(u[0])), f(x(u[1]).fadd_m(ay)), f(x(u[2]).fadd_m(az))]);
    let bz = f(x(c[2]).fsub_m(r[2]));
    let by = f(x(c[1]).fsub_m(r[1]));
    let bx = f(x(c[0]).fsub_m(r[0]));
    set_corner(1, [f(x(bx).fadd_m(u[0])), f(x(u[1]).fadd_m(by)), f(x(u[2]).fadd_m(bz))]);
    set_corner(2, [f(x(bx).fsub_m(u[0])), f(x(by).fsub_m(u[1])), f(x(bz).fsub_m(u[2]))]);
    set_corner(3, [f(x(ax).fsub_m(u[0])), f(x(ay).fsub_m(u[1])), f(x(az).fsub_m(u[2]))]);
    project_corners(4);
    for (c1, c2) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
        if process_line_segment(this, c1, c2, out) != 0 {
            return false;
        }
    }
    // 0.1 (0x858B1C), 0.07 (0x858CA8)
    if !big_enough(MAX_X, MIN_X, 0xc1_7044, k(0x3dcc_cccd)) || !big_enough(MAX_Y, MIN_Y, 0xc1_7048, k(0x3d8f_5c29)) {
        return false;
    }
    let mut rr = r;
    let mut uu = u;
    let mut n = [0f32; 3];
    cross_product(
        GPtr::<f32>::from_raw(n.as_mut_ptr()).cast(),
        GPtr::<f32>::from_raw(rr.as_mut_ptr()).cast(),
        GPtr::<f32>::from_raw(uu.as_mut_ptr()).cast(),
    );
    normalise(GPtr::<f32>::from_raw(n.as_mut_ptr()).cast());
    let off = x(n[0]).fmul_m(c[0]).fadd(x(n[1]).fmul_m(c[1])).fadd(x(n[2]).fmul_m(c[2]));
    wr::<f32>(out, 0x7c, n[0]);
    wr::<f32>(out, 0x80, n[1]);
    wr::<f32>(out, 0x84, n[2]);
    wr::<u8>(out, 0x7b, 1);
    wr::<f32>(out, 0xa0, off.to_f32());
    true
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_ONE_OCCLUDER];
