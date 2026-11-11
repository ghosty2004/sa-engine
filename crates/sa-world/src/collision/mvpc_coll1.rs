//! WRITER-MVP-C batch `mvpc-coll-1`: the `CCollision` line-of-sight entry points on the boot/loading
//! path (`TestLineOfSight`, `ProcessLineOfSight`, `ProcessVerticalLine`) and the line/primitive
//! processors they call (`ProcessLineTriangle`, ...). Ported from the 1.0 US bytes (SecuROM fragments
//! inlined, the `push; call next; ...; ret` dispatcher of `TestLineOfSight` decoded to its target
//! `CCollision::CalculateTrianglePlanes` 0x416330), x87 sequences through the [`Fpu`] model (PC = 24)
//! in the original's operand order; gta-reversed `Collision.cpp` was a reading aid only.
//! Unverified (docs/verify/HOWTO.md).
//!
//! Layouts: `CColModel` +0 bounding box (`CBox`), +0x2C `m_pColData`. `CCollisionData` +0 i16 spheres,
//! +2 i16 boxes, +4 i16 triangles, +8 spheres (0x14 bytes, surface +0x10), +0xC boxes (0x1C bytes,
//! surface +0x18), +0x14 compressed vertices (3 x i16, 1/128), +0x18 triangles (8 bytes: u16 a, b, c,
//! u8 surface +6, u8 light +7), +0x1C triangle planes (0xA bytes: i16 normal x, y, z (1/4096), i16
//! distance (1/128), u8 orientation +8). `CColLine` start +0, end +0x10. `CColPoint` point +0, normal
//! +0x10, surface A +0x20, piece A +0x21, light A +0x22, surface B +0x23, piece B +0x24, light B +0x25.
//! `CStoredCollPoly` three vertices +0/+0xC/+0x18, +0x24 bValidPolyStored, +0x28 light.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::fpu::Fpu;
use crate::world::common::{g, rd, wr};

/// `g_surfaceInfos` (0xB79538)
const SURFACE_INFOS: u32 = 0xB79538;
/// `CCollision::ms_iProcessLineNumCrossings`
const NUM_CROSSINGS: usize = 0x9655D0;
/// `1.0f / 4096` (0x858BFC)
const K_1_4096: u32 = 0x858BFC;
/// `1.0f / 128` (0x858B88)
const K_1_128: u32 = 0x858B88;
/// `0.0f` (0x858B50)
const K_0: u32 = 0x858B50;

orig! {
    /// CRT `_atexit` (0x821D1E)
    fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// the no-CD protection VM entry (0xCB73E0, pops its two arguments), called from the static
    /// initialiser of `TestLineOfSight`'s inverse matrix
    fn protection_vm(code: u32, n: u32) = stdcall 0xCB73E0 => unported;
    /// `CMatrix::Invert(const CMatrix& src, CMatrix& dst)` (0x59B920, cdecl, returns `dst`)
    fn matrix_invert(src: GPtr<u8>, dst: GPtr<u8>) -> GPtr<u8> = cdecl 0x59B920 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, out first)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `Multiply3x3(const CMatrix&, const CVector&)` (0x59C790, cdecl, out first)
    fn multiply_3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CColLine::CColLine(const CVector& start, const CVector& end)` (0x40EF50, `ret 8`)
    fn col_line_ctor(l: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) -> GPtr<u8> = thiscall 0x40EF50 => unported;
    /// `CCollision::TestLineBox_DW(const CColLine&, const CBox&)` (0x412C70, AL)
    fn test_line_box_dw(line: GPtr<u8>, bx: GPtr<u8>) -> u8 = cdecl 0x412C70 => unported;
    /// `CCollision::TestLineSphere(const CColLine&, const CColSphere&)` (0x417470, AL)
    fn test_line_sphere(line: GPtr<u8>, sp: GPtr<u8>) -> u8 = cdecl 0x417470 => unported;
    /// `CCollision::TestLineTriangle(const CColLine&, const CompressedVector*, const CColTriangle&,
    /// const CColTrianglePlane&)` (0x413AC0, AL)
    fn test_line_triangle(line: GPtr<u8>, verts: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>) -> u8 = cdecl 0x413AC0 => unported;
    /// `CCollision::ProcessLineSphere(const CColLine&, const CColSphere&, CColPoint&, float&)` (0x412AA0, AL)
    fn process_line_sphere(line: GPtr<u8>, sp: GPtr<u8>, cp: GPtr<u8>, min: GPtr<u8>) -> u8 = cdecl 0x412AA0 => unported;
    /// `CCollision::ProcessLineBox(const CColLine&, const CColBox&, CColPoint&, float&)` (0x413100, AL)
    fn process_line_box_call(line: GPtr<u8>, bx: GPtr<u8>, cp: GPtr<u8>, min: GPtr<u8>) -> u8 = cdecl 0x413100 => unported;
    /// `CCollision::ProcessLineTriangle(...)` (0x4140F0, AL)
    fn process_line_triangle_call(line: GPtr<u8>, verts: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>, cp: GPtr<u8>,
        min: GPtr<u8>, poly: GPtr<u8>) -> u8 = cdecl 0x4140F0 => process_line_triangle;
    /// `CCollision::CalculateTrianglePlanes(CCollisionData*)` (0x416330)
    fn calculate_triangle_planes(cd: GPtr<u8>) = cdecl 0x416330 => unported;
    /// `SurfaceInfos_c::IsSeeThrough(uint32)` (0x55E6B0, AL)
    fn is_see_through(si: u32, id: u32) -> u8 = thiscall 0x55E6B0 => unported;
    /// `SurfaceInfos_c::IsShootThrough(uint32)` (0x55E6D0, AL)
    fn is_shoot_through(si: u32, id: u32) -> u8 = thiscall 0x55E6D0 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, out first; no catalogue name)
    fn vector_scale(out: GPtr<u8>, v: GPtr<u8>, f: u32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, cdecl, out first; no catalogue name)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `CompressedVector` -> `CVector` (0x411590, cdecl `(CVector* out, const CompressedVector*)`; no
    /// catalogue name)
    fn uncompress_vector(out: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x411590 => unported;
}

/// A zeroed Rust local of `N` dwords, addressed as game memory.
pub(crate) fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// Copies the three dwords at `src` to `dst` (the original's `mov` triple after a vector call).
pub(crate) fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..3 {
        wr::<u32>(dst, 4 * i, rd::<u32>(src, 4 * i));
    }
}

/// f32 constant at `addr`.
fn kf(addr: u32) -> f32 {
    g::<f32>(addr as usize).read()
}

/// First-call initialiser of a function-static `CMatrix` (guard bit 0 of the dword at `guard`):
/// sets the bit, clears the matrix's `m_pAttachMatrix` (+0x40) / `m_bOwnsAttachedMemory` (+0x44) and
/// registers its destructor with `atexit`.
fn static_matrix_init(guard: usize, m: usize, dtor: u32) {
    if g::<u32>(guard).read() & 1 == 0 {
        g::<u32>(guard).write(g::<u32>(guard).read() | 1);
        g::<u32>(m + 0x40).write(0);
        g::<u32>(m + 0x44).write(0);
        atexit(dtor);
    }
}

/// `line' = CColLine(inv * line.start, inv * line.end)` built in `buf` (0x20 bytes) as the original
/// builds it on its stack: end transformed first, then start, then the `CColLine` constructor.
fn transform_line(line: GPtr<u8>, inv: GPtr<u8>, buf: GPtr<u8>) {
    let mut e = [0u32; 3];
    let mut s = [0u32; 3];
    let pe = matrix_mul_vec(local(&mut e), inv, line.byte_add(0x10));
    let ps = matrix_mul_vec(local(&mut s), inv, line);
    col_line_ctor(buf, ps, pe);
}

/// `CCollision::TestLineOfSight(const CColLine& line, const CMatrix& transform, CColModel& colModel,
/// bool doSeeThroughCheck, bool doShootThroughCheck)` (0x417730, a `.HOODLUM` stub into 0x1564C40, AL).
/// No collision data → false. First call: the function-static inverse matrix (0x965960, guard
/// 0x9659A8) is initialised through the protection VM (0xCB73E0 with 0x155FF68, 0x586A; the
/// constructor's effect), `m_bOwnsAttachedMemory` (0x9659A4) cleared and its destructor (0x855FF0)
/// registered. `Invert(transform)`, the line moved into model space; false unless it hits the model's
/// bounding box (`TestLineBox_DW`). Then, skipping see-through / shoot-through surfaces when asked:
/// any sphere (`TestLineSphere`), any box (`TestLineBox_DW`), and, after `CalculateTrianglePlanes`
/// (reached through the obfuscated `push; call next; ...; ret` dispatcher), any triangle
/// (`TestLineTriangle`) → true. The counts are re-read (sign-extended) every iteration.
#[hook(addr = 0x417730, conv = cdecl, name = "CCollision::TestLineOfSight", expect = "E9 0B D5 14 01",
       unverified = "mvpc-coll-1", route = "L2", hazards = "none", recs = "grove-walk,driving",
       note = "first call runs the protection VM 0xCB73E0 (static matrix ctor), kept as an orig call")]
pub fn test_line_of_sight(line: GPtr<u8>, transform: GPtr<u8>, model: GPtr<u8>, see: u8, shoot: u8) -> u8 {
    let cd = rd::<GPtr<u8>>(model, 0x2C);
    if cd.is_null() {
        return 0;
    }
    const GUARD: usize = 0x9659A8;
    if g::<u32>(GUARD).read() & 1 == 0 {
        g::<u32>(GUARD).write(g::<u32>(GUARD).read() | 1);
        protection_vm(0x155FF68, 0x586A);
        g::<u32>(0x9659A4).write(0);
        atexit(0x855FF0);
    }
    let inv = GPtr::<u8>::from_addr(0x965960);
    matrix_invert(transform, inv);
    let mut lb = [0u32; 8];
    let l = local(&mut lb);
    transform_line(line, inv, l);
    if test_line_box_dw(l, model) == 0 {
        return 0;
    }
    let skip = |surf: u8| -> bool {
        (see != 0 && is_see_through(SURFACE_INFOS, surf as u32) != 0)
            || (shoot != 0 && is_shoot_through(SURFACE_INFOS, surf as u32) != 0)
    };
    let mut i = 0i32;
    while i < rd::<i16>(cd, 0) as i32 {
        let sp = rd::<GPtr<u8>>(cd, 8).byte_add(i as isize * 0x14);
        if !skip(rd::<u8>(sp, 0x10)) && test_line_sphere(l, sp) != 0 {
            return 1;
        }
        i += 1;
    }
    let mut i = 0i32;
    while i < rd::<i16>(cd, 2) as i32 {
        let bx = rd::<GPtr<u8>>(cd, 0xC).byte_add(i as isize * 0x1C);
        if !skip(rd::<u8>(bx, 0x18)) && test_line_box_dw(l, bx) != 0 {
            return 1;
        }
        i += 1;
    }
    calculate_triangle_planes(cd);
    let mut i = 0i32;
    while i < rd::<i16>(cd, 4) as i32 {
        let tri = rd::<GPtr<u8>>(cd, 0x18).byte_add(i as isize * 8);
        if !skip(rd::<u8>(tri, 6)) {
            let plane = rd::<GPtr<u8>>(cd, 0x1C).byte_add(i as isize * 0xA);
            if test_line_triangle(l, rd::<GPtr<u8>>(cd, 0x14), tri, plane) != 0 {
                return 1;
            }
        }
        i += 1;
    }
    0
}

/// `CCollision::ProcessLineOfSight(const CColLine& line, const CMatrix& transform, CColModel& colModel,
/// CColPoint& colPoint, float& maxTouchDistance, bool seeThrough, bool shootThrough)` (0x417950, AL).
/// Function-static inverse matrix 0x9659B0 (guard 0x9659F8, `m_pAttachMatrix`/`m_bOwns...` cleared,
/// destructor 0x856000 with `atexit`). No collision data → false; the line is moved into model space
/// and must hit the bounding box (`TestLineBox_DW`). The running minimum starts as `*maxTouchDistance`
/// and is kept in the `colModel` argument slot (`mov [esp+0x54], eax`; the callees get its address).
/// Spheres (`ProcessLineSphere`), boxes (`ProcessLineBox`), then `CalculateTrianglePlanes` and the
/// triangles (`ProcessLineTriangle` with no stored poly; each hit increments
/// `ms_iProcessLineNumCrossings`), skipping see-/shoot-through surfaces when asked. Unless
/// `min < *maxTouchDistance` (f32 compare, NaN false) → false; else the colpoint is moved back
/// (`point = transform * point`, `normal = Multiply3x3(transform, normal)`), `*maxTouchDistance = min`.
/// Stack: the original overwrites its `colModel` argument slot (S+0xC) with the running minimum
/// (not reproduced: no footprint because the processors it calls are hooks themselves).
#[hook(addr = 0x417950, conv = cdecl, name = "CCollision::ProcessLineOfSight", expect = "A0 F8 59 96 00 83 EC 38",
       unverified = "mvpc-coll-1", route = "L2", hazards = "stack-residue", recs = "grove-walk,driving",
       note = "writes the running minimum into its colModel argument slot (S+0xC); callers push fresh args")]
#[allow(clippy::too_many_arguments)]
pub fn process_line_of_sight(line: GPtr<u8>, transform: GPtr<u8>, model: GPtr<u8>, cp: GPtr<u8>, max: GPtr<u8>,
    see: u8, shoot: u8) -> u8 {
    static_matrix_init(0x9659F8, 0x9659B0, 0x856000);
    let cd = rd::<GPtr<u8>>(model, 0x2C);
    if cd.is_null() {
        return 0;
    }
    let inv = GPtr::<u8>::from_addr(0x9659B0);
    matrix_invert(transform, inv);
    let mut lb = [0u32; 8];
    let l = local(&mut lb);
    transform_line(line, inv, l);
    if test_line_box_dw(l, model) == 0 {
        return 0;
    }
    let mut minb = [rd::<u32>(max, 0)];
    let min = local(&mut minb);
    let skip = |surf: u8| -> bool {
        (see != 0 && is_see_through(SURFACE_INFOS, surf as u32) != 0)
            || (shoot != 0 && is_shoot_through(SURFACE_INFOS, surf as u32) != 0)
    };
    let mut i = 0i32;
    while i < rd::<i16>(cd, 0) as i32 {
        let sp = rd::<GPtr<u8>>(cd, 8).byte_add(i as isize * 0x14);
        if !skip(rd::<u8>(sp, 0x10)) {
            process_line_sphere(l, sp, cp, min);
        }
        i += 1;
    }
    let mut i = 0i32;
    while i < rd::<i16>(cd, 2) as i32 {
        let bx = rd::<GPtr<u8>>(cd, 0xC).byte_add(i as isize * 0x1C);
        if !skip(rd::<u8>(bx, 0x18)) {
            process_line_box_call(l, bx, cp, min);
        }
        i += 1;
    }
    calculate_triangle_planes(cd);
    let mut i = 0i32;
    while i < rd::<i16>(cd, 4) as i32 {
        let tri = rd::<GPtr<u8>>(cd, 0x18).byte_add(i as isize * 8);
        if !skip(rd::<u8>(tri, 6)) {
            let plane = rd::<GPtr<u8>>(cd, 0x1C).byte_add(i as isize * 0xA);
            if process_line_triangle_call(l, rd::<GPtr<u8>>(cd, 0x14), tri, plane, cp, min, GPtr::NULL) != 0 {
                g::<u32>(NUM_CROSSINGS).write(g::<u32>(NUM_CROSSINGS).read().wrapping_add(1));
            }
        }
        i += 1;
    }
    if !finish_colpoint(min, max, cp, transform) {
        return 0;
    }
    1
}

/// The common tail: `fld min; fcomp [max]; test ah, 5; jp fail` (go on only when `min < *max`), then
/// `point = transform * point`, `normal = Multiply3x3(transform, normal)`, and (written by the
/// caller's tail) `*max = min`.
fn finish_colpoint(min: GPtr<u8>, max: GPtr<u8>, cp: GPtr<u8>, transform: GPtr<u8>) -> bool {
    let mut f = Fpu::new();
    f.fld(rd::<f32>(min, 0));
    f.fcomp_m(rd::<f32>(max, 0));
    if f.jp(5) {
        return false;
    }
    let mut t = [0u32; 3];
    let r = matrix_mul_vec(local(&mut t), transform, cp);
    copy3(cp, r);
    let mut t2 = [0u32; 3];
    let r = multiply_3x3(local(&mut t2), transform, cp.byte_add(0x10));
    copy3(cp.byte_add(0x10), r);
    wr::<u32>(max, 0, rd::<u32>(min, 0));
    true
}

/// One component of `ProcessVerticalLine`'s inverse transform: `(d.x*a.x + d.z*a.z) + d.y*a.y` with
/// `d = p - transform.pos` (each difference and product rounded as the x87 sequence does).
fn rot_t(f: &mut Fpu, m: GPtr<u8>, row: isize) -> f32 {
    f.fld_st(2);
    f.fmul_m(rd::<f32>(m, row));
    f.fld_st(1);
    f.fmul_m(rd::<f32>(m, row + 8));
    f.faddp(1);
    f.fld_st(2);
    f.fmul_m(rd::<f32>(m, row + 4));
    f.faddp(1);
    f.fstp()
}

/// `p' = transposed(rotation) * (p - pos)` as `ProcessVerticalLine` computes it (right, forward, up
/// rows; the last row with `fxch` but the same sum order).
fn inv_transform_point(p: GPtr<u8>, m: GPtr<u8>) -> [f32; 3] {
    let mut f = Fpu::new();
    f.fld(rd::<f32>(p, 0));
    f.fsub_m(rd::<f32>(m, 0x30));
    f.fld(rd::<f32>(p, 4));
    f.fsub_m(rd::<f32>(m, 0x34));
    f.fld(rd::<f32>(p, 8));
    f.fsub_m(rd::<f32>(m, 0x38));
    let x = rot_t(&mut f, m, 0);
    let y = rot_t(&mut f, m, 0x10);
    // fxch st2; fmul [0x20]; fxch st2; fmul [0x28]; faddp st2; fmul [0x24]; faddp
    f.fxch(2);
    f.fmul_m(rd::<f32>(m, 0x20));
    f.fxch(2);
    f.fmul_m(rd::<f32>(m, 0x28));
    f.faddp(2);
    f.fmul_m(rd::<f32>(m, 0x24));
    f.faddp(1);
    let z = f.fstp();
    [x, y, z]
}

/// `CCollision::ProcessVerticalLine(const CColLine& line, const CMatrix& transform, CColModel& colModel,
/// CColPoint& colPoint, float& maxTouchDistance, bool seeThrough, bool shootThrough,
/// CStoredCollPoly* storedCollPoly)` (0x417BF0, AL). No collision data → false. Sets the guard bit of
/// the function-static `CStoredCollPoly` (0x9659FC, guard 0x965A28; trivial constructor). The line is
/// moved into model space with the transposed rotation (`(p - pos)` dotted with right / forward / up,
/// stored f32; the `CColLine` padding words stay unwritten) and must hit the bounding box. The running
/// minimum starts as `*maxTouchDistance` and lives in the `colModel` argument slot. Spheres and boxes
/// (see-through skipped; `shootThrough` is never read), `CalculateTrianglePlanes`, the static poly's
/// `bValidPolyStored` (0x965A20) cleared, triangles through `ProcessLineTriangle` with the static poly.
/// Unless `min < *max` → false; else the colpoint is moved back, and when the static poly was stored
/// and `storedCollPoly` is given, the 0x2C-byte poly is copied there (`rep movsd`) and its three
/// vertices transformed by `transform`. `*maxTouchDistance = min` → true.
#[hook(addr = 0x417BF0, conv = cdecl, name = "CCollision::ProcessVerticalLine", expect = "8B 4C 24 0C 83 EC 2C 56",
       unverified = "mvpc-coll-1", route = "L2", hazards = "stack-residue,uninit-locals", recs = "grove-walk,driving",
       note = "minimum kept in the colModel argument slot (S+0xC); the local CColLine's padding is never written")]
#[allow(clippy::too_many_arguments)]
pub fn process_vertical_line(line: GPtr<u8>, transform: GPtr<u8>, model: GPtr<u8>, cp: GPtr<u8>, max: GPtr<u8>,
    see: u8, _shoot: u8, poly: GPtr<u8>) -> u8 {
    let cd = rd::<GPtr<u8>>(model, 0x2C);
    if cd.is_null() {
        return 0;
    }
    if g::<u32>(0x965A28).read() & 1 == 0 {
        g::<u32>(0x965A28).write(g::<u32>(0x965A28).read() | 1);
    }
    let s = inv_transform_point(line, transform);
    let e = inv_transform_point(line.byte_add(0x10), transform);
    let mut lb = [0u32; 8];
    let l = local(&mut lb);
    for i in 0..3 {
        wr::<f32>(l, 4 * i as isize, s[i]);
        wr::<f32>(l, 0x10 + 4 * i as isize, e[i]);
    }
    if test_line_box_dw(l, model) == 0 {
        return 0;
    }
    let mut minb = [rd::<u32>(max, 0)];
    let min = local(&mut minb);
    let skip = |surf: u8| see != 0 && is_see_through(SURFACE_INFOS, surf as u32) != 0;
    let mut i = 0i32;
    while i < rd::<i16>(cd, 0) as i32 {
        let sp = rd::<GPtr<u8>>(cd, 8).byte_add(i as isize * 0x14);
        if !skip(rd::<u8>(sp, 0x10)) {
            process_line_sphere(l, sp, cp, min);
        }
        i += 1;
    }
    let mut i = 0i32;
    while i < rd::<i16>(cd, 2) as i32 {
        let bx = rd::<GPtr<u8>>(cd, 0xC).byte_add(i as isize * 0x1C);
        if !skip(rd::<u8>(bx, 0x18)) {
            process_line_box_call(l, bx, cp, min);
        }
        i += 1;
    }
    calculate_triangle_planes(cd);
    g::<u8>(0x965A20).write(0);
    let stat = GPtr::<u8>::from_addr(0x9659FC);
    let mut i = 0i32;
    while i < rd::<i16>(cd, 4) as i32 {
        let tri = rd::<GPtr<u8>>(cd, 0x18).byte_add(i as isize * 8);
        if !skip(rd::<u8>(tri, 6)) {
            let plane = rd::<GPtr<u8>>(cd, 0x1C).byte_add(i as isize * 0xA);
            process_line_triangle_call(l, rd::<GPtr<u8>>(cd, 0x14), tri, plane, cp, min, stat);
        }
        i += 1;
    }
    let mut f = Fpu::new();
    f.fld(rd::<f32>(min, 0));
    f.fcomp_m(rd::<f32>(max, 0));
    if f.jp(5) {
        return 0;
    }
    let mut t = [0u32; 3];
    let r = matrix_mul_vec(local(&mut t), transform, cp);
    copy3(cp, r);
    let r = multiply_3x3(local(&mut t), transform, cp.byte_add(0x10));
    copy3(cp.byte_add(0x10), r);
    if g::<u8>(0x965A20).read() != 0 && !poly.is_null() {
        for k in 0..11 {
            wr::<u32>(poly, 4 * k, rd::<u32>(stat, 4 * k));
        }
        for v in [0isize, 0xC, 0x18] {
            let r = matrix_mul_vec(local(&mut t), transform, poly.byte_add(v));
            copy3(poly.byte_add(v), r);
        }
    }
    wr::<u32>(max, 0, rd::<u32>(min, 0));
    1
}

/// `CCollision::ProcessLineTriangle(const CColLine& line, const CompressedVector* verts,
/// const CColTriangle& tri, const CColTrianglePlane& plane, CColPoint& colPoint, float& minDist,
/// CStoredCollPoly* collPoly)` (0x4140F0, AL). Plane normal `n = i16 * (1/4096)` (stored f32), distance
/// `d = i16 * (1/128)` (kept on the x87 stack). The start's dot products are stored per component
/// (`nz*sz`, `ny*sy`, `nx*sx`, f32) and summed `(x + y) + z`; the end's dot product is
/// `(nz*ez + ny*ey) + nx*ex` unrounded. `(startDot - d) * (endDot - d) > 0` or unordered → false.
/// `t = ((d - sx') - sy') - sz') / ((dz*nz + dy*ny) + dx*nx)` (f32; `dz` stored f32 first), the hit
/// point `P = start + delta * t` (f32 per component). The triangle is projected by the plane's
/// orientation byte (0: y/z, 1: y/z with vertices 1 and 2 swapped, 2: z/x, 3: swapped, 4: x/y,
/// 5: swapped; vertex coordinates `i16 * (1/128)`); the three edge tests (`< 0` → false on the first,
/// `> 0` → false on the second, `< 0` → false on the third, NaN passes) follow the x87 sequence. Then
/// `t < *minDist` (else false): `colPoint.point = start + (end - start) * t` through the original's
/// vector helpers, `normal = n`, surface B = tri surface, piece B 0, light B = tri light, surface A
/// and piece A 0; with `collPoly` the three vertices are uncompressed into it, `bValidPolyStored = 1`,
/// `light = tri light`. `*minDist = t` → true. Orientation > 5 reads locals the switch never wrote
/// (zero here; the original's garbage).
/// Stack: the original stores intermediates into its `line` (S+4) and `plane` (S+0x10) argument
/// slots (not reproduced; no caller reads them).
#[hook(addr = 0x4140F0, conv = cdecl, name = "CCollision::ProcessLineTriangle", expect = "83 EC 44 8B 4C 24 54",
       unverified = "mvpc-coll-1", route = "L2", hazards = "stack-residue", recs = "grove-walk,driving",
       note = "overwrites its line/plane argument slots with intermediates; orientation > 5 reads uninitialised locals")]
#[allow(clippy::too_many_arguments)]
pub fn process_line_triangle(line: GPtr<u8>, verts: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>, cp: GPtr<u8>,
    min: GPtr<u8>, poly: GPtr<u8>) -> u8 {
    let q4096 = kf(K_1_4096);
    let q128 = kf(K_1_128);
    let zero = kf(K_0);
    let mut f = Fpu::new();
    let n = |f: &mut Fpu, o: isize| {
        f.fild(rd::<i16>(plane, o) as i32);
        f.fmul_m(q4096);
        f.fstp()
    };
    let nx = n(&mut f, 0);
    let ny = n(&mut f, 2);
    let nz = n(&mut f, 4);
    f.fild(rd::<i16>(plane, 6) as i32);
    f.fmul_m(q128); // st0 = d
    let lw = |o: isize| rd::<f32>(line, o);
    let prod = |f: &mut Fpu, a: f32, b: f32| {
        f.fld(a);
        f.fmul_m(b);
        f.fstp()
    };
    let pz = prod(&mut f, nz, lw(8));
    let py = prod(&mut f, ny, lw(4));
    let px = prod(&mut f, nx, lw(0));
    f.fld(nz);
    f.fmul_m(lw(0x18));
    f.fld(ny);
    f.fmul_m(lw(0x14));
    f.faddp(1);
    f.fld(nx);
    f.fmul_m(lw(0x10));
    f.faddp(1);
    f.fsub_st(1);
    f.fld(px);
    f.fadd_m(py);
    f.fadd_m(pz);
    f.fsub_st(2);
    f.fmulp(1);
    f.fcomp_m(zero);
    if f.jp(0x41) {
        return 0;
    }
    // st0 = d
    f.fld(lw(0x10));
    f.fsub_m(lw(0));
    f.fld(lw(0x14));
    f.fsub_m(lw(4));
    f.fld(lw(0x18));
    f.fsub_m(lw(8));
    let dz = f.fstp(); // st0 = dy, st1 = dx, st2 = d
    f.fxch(2); // st0 = d, st1 = dx, st2 = dy
    f.fsub_m(px);
    f.fsub_m(py);
    f.fsub_m(pz);
    f.fld(dz);
    f.fmul_m(nz);
    f.fld_st(3);
    f.fmul_m(ny);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_m(nx);
    f.faddp(1);
    f.fdivp(1);
    let t = f.fstp(); // st0 = dx, st1 = dy
    f.fld_st(0);
    f.fmul_m(t);
    let dxt = f.fstp();
    f.fxch(1);
    f.fmul_m(t);
    f.fxch(1);
    f.fstp_st(0); // st0 = dy*t
    f.fld(dz);
    f.fmul_m(t); // st0 = dz*t, st1 = dy*t
    f.fld(dxt);
    f.fadd_m(lw(0));
    let px_ = f.fstp();
    f.fxch(1);
    f.fadd_m(lw(4));
    let py_ = f.fstp();
    f.fadd_m(lw(8));
    let pz_ = f.fstp();
    let p = [px_, py_, pz_];
    // projected vertices: coordinate `c` of vertex `k`
    let vc = |k: isize, c: isize| -> f32 {
        let idx = rd::<u16>(tri, 2 * k) as isize;
        let mut g = Fpu::new();
        g.fild(rd::<i16>(verts, idx * 6 + 2 * c) as i32);
        g.fmul_m(q128);
        g.fstp()
    };
    let orient = rd::<u8>(plane, 8);
    let (a, b, swap) = match orient {
        0 => (1, 2, false),
        1 => (1, 2, true),
        2 => (2, 0, false),
        3 => (2, 0, true),
        4 => (0, 1, false),
        5 => (0, 1, true),
        _ => (3, 3, false),
    };
    let (a0, b0, a1, b1, a2, b2, pa, pb);
    if a == 3 {
        // `ja` default: `fld [esp+0x64]` twice (the plane argument slot, holding nx*sx); B2 = the line
        // argument slot (ny*sy); the other locals were never written (zero here)
        a0 = px;
        b0 = px;
        a1 = 0.0;
        b1 = 0.0;
        a2 = px;
        b2 = py;
        pa = 0.0;
        pb = 0.0;
    } else {
        a0 = vc(0, a);
        b0 = vc(0, b);
        let (k1, k2) = if swap { (2, 1) } else { (1, 2) };
        a1 = vc(k1, a);
        b1 = vc(k1, b);
        a2 = vc(k2, a);
        b2 = vc(k2, b);
        pa = p[a as usize];
        pb = p[b as usize];
    }
    // stack: st0 = B0, st1 = A0 (d was consumed by the fdivp above)
    let mut f = Fpu::new();
    f.fld(a0);
    f.fld(b0);
    f.fld(pb);
    f.fsub_st(1);
    f.fld(pa);
    f.fsub_st(3); // st0 = dPa, st1 = dPb, st2 = B0, st3 = A0
    f.fld(a2);
    f.fsub_st(4);
    f.fmul_st(2);
    f.fld(b2);
    f.fsub_st(4);
    f.fmul_st(2);
    f.fsubp(1);
    f.fcomp_m(zero);
    if !f.jp(5) {
        return 0;
    }
    f.fld(a1);
    f.fsub_st(4);
    f.fmul_st(2);
    f.fld(b1);
    f.fsub_st(4);
    f.fmul_st(2);
    f.fsubp(1);
    f.fcomp_m(zero);
    if !f.jne(0x41) {
        return 0;
    }
    let mut f = Fpu::new();
    f.fld(pb);
    f.fsub_m(b2);
    f.fld(a1);
    f.fsub_m(a2);
    f.fmulp(1);
    f.fld(pa);
    f.fsub_m(a2);
    f.fld(b1);
    f.fsub_m(b2);
    f.fmulp(1);
    f.fsubp(1);
    f.fcomp_m(zero);
    if !f.jp(5) {
        return 0;
    }
    f.fld(t);
    f.fcomp_m(rd::<f32>(min, 0));
    if f.jp(5) {
        return 0;
    }
    let mut diff = [0u32; 3];
    let mut sc = [0u32; 3];
    let mut sum = [0u32; 3];
    let d = vector_sub(local(&mut diff), line.byte_add(0x10), line);
    let s = vector_scale(local(&mut sc), d, t.to_bits());
    let r = vector_add(local(&mut sum), line, s);
    copy3(cp, r);
    wr::<f32>(cp, 0x10, nx);
    wr::<f32>(cp, 0x14, ny);
    wr::<f32>(cp, 0x18, nz);
    wr::<u8>(cp, 0x23, rd::<u8>(tri, 6));
    wr::<u8>(cp, 0x24, 0);
    wr::<u8>(cp, 0x25, rd::<u8>(tri, 7));
    wr::<u8>(cp, 0x20, 0);
    wr::<u8>(cp, 0x21, 0);
    if !poly.is_null() {
        for (k, o) in [(0isize, 0isize), (1, 0xC), (2, 0x18)] {
            let idx = rd::<u16>(tri, 2 * k) as isize;
            uncompress_vector(poly.byte_add(o), verts.byte_add(idx * 6));
        }
        wr::<u8>(poly, 0x24, 1);
        wr::<u32>(poly, 0x28, rd::<u8>(tri, 7) as u32);
    }
    wr::<f32>(min, 0, t);
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_TEST_LINE_OF_SIGHT,
    HOOK_PROCESS_LINE_OF_SIGHT,
    HOOK_PROCESS_VERTICAL_LINE,
    HOOK_PROCESS_LINE_TRIANGLE,
];
