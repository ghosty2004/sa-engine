//! Free functions of gta-reversed's `common.cpp` / `GxtChar.cpp` in CORE: string helpers, screen
//! projection, the occlusion line tests, `WriteRaster`, `LittleTest`.
//!
//! Ported from the 1.0 US bytes. x87 arithmetic follows the original operand by operand under PC=24
//! with [`X87`] (`fsub m32`/`fmul m32` forms, compares through `fnstsw`/`test ah`). Where
//! gta-reversed differs the original is kept:
//! - `MakeUpperCase(dst, src)` (0x7186E0) returns the **start** of `dst` (gr: the end); the in-place
//!   form (0x718710) returns nothing meaningful (EAX is never set; gr: `dest`), so the port is `void`.
//!   Both uppercase only `a`..`z` (unsigned compares; gr: locale `toupper`).
//! - `CalcScreenCoors(in, out, sx, sy)` (0x71DA00): `sx = W * (1/z)` is stored as f32 first, then
//!   reloaded and turned into `sx / FOV * 70` (gr: `W * (1/z / FOV * 70)`, different rounding).
//! - `DoesInfiniteLineTouchScreen` (0x71DB80): the on-screen test is strict on all four sides
//!   (`0 < x < W`, `0 < y < H`), the first two edge tests are inlined with `-y*dx` / `-x*dy` and
//!   f32-stored intermediates, the edge order is top, left, right, bottom.
//! - `IsPointInsideLine` (0x71E050) returns true for NaN (`!(d < r)`; gr: `d >= r`).

use crate::mem::GPtr;
// `RsGlobal.maximumWidth/maximumHeight` are declared once, in `platform`.
use crate::platform::{RS_MAXIMUM_HEIGHT as SCREEN_HEIGHT, RS_MAXIMUM_WIDTH as SCREEN_WIDTH};
use crate::{globals, hook, orig};
use sa_math::x87::X87;

globals! {
    list GLOBALS;
    /// `g_nNumIm3dDrawCalls` (incremented by `LittleTest`)
    pub static NUM_IM3D_DRAW_CALLS @ 0xB73708: i32 = 0;
    /// The debug text buffer's entry count (`CDebug`, a `uint16` at 0xB71508)
    pub static DEBUG_TEXT_COUNT @ 0xB71508: u16 = 0;
    /// `CDraw::ms_fFOV`
    pub static DRAW_FOV @ 0x8D5038: f32 = 45.0;
}

/// `TheCamera.m_mViewMatrix` (0xB6FA2C)
pub const CAMERA_VIEW_MATRIX: u32 = 0xB6FA2C;
/// `.rdata` constants (f32 bits)
pub mod k {
    /// 0x858624: 1.0
    pub const ONE: u32 = 0x3F80_0000;
    /// 0x858CE0: 70.0
    pub const SEVENTY: u32 = 0x428C_0000;
}

orig! {
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, hidden return pointer)
    pub fn matrix_mul_vector(out: GPtr<[f32; 3]>, m: GPtr<u8>, v: GPtr<[f32; 3]>) -> GPtr<[f32; 3]> = cdecl 0x59C890 => unported;
    /// `RwImageCreate` (0x8026E0)
    pub fn rw_image_create(width: i32, height: i32, depth: i32) -> GPtr<u8> = cdecl 0x8026E0 => unported;
    /// `RwImageAllocatePixels` (0x8027A0)
    pub fn rw_image_allocate_pixels(img: GPtr<u8>) -> GPtr<u8> = cdecl 0x8027A0 => unported;
    /// `RwImageSetFromRaster` (0x804250)
    pub fn rw_image_set_from_raster(img: GPtr<u8>, raster: GPtr<u8>) -> GPtr<u8> = cdecl 0x804250 => unported;
    /// `RtPNGImageWrite` (0x7CF600)
    pub fn rt_png_image_write(img: GPtr<u8>, filename: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CF600 => unported;
    /// `RwImageDestroy` (0x802740)
    pub fn rw_image_destroy(img: GPtr<u8>) -> i32 = cdecl 0x802740 => unported;
    /// 0x71DB30 (gta-reversed `DoesInfiniteLineCrossFiniteLine`)
    pub fn does_infinite_line_cross_finite_line_call(ax: f32, ay: f32, bx: f32, by: f32, ox: f32, oy: f32, dx: f32, dy: f32) -> i32 = cdecl 0x71DB30 => does_infinite_line_cross_finite_line;
}

fn ld(v: f32) -> X87 {
    X87::from_f32(v)
}

/// 0x532240 (cdecl; gta-reversed `CDebug::DebugInitTextBuffer`, called from `CGame::Init1` where gr
/// calls it): clears the `uint16` count at 0xB71508 (gr reserves a `std::vector` instead).
#[hook(addr = 0x532240, conv = cdecl, name = "FUN_00532240", expect = "66 C7 05 08 15 B7 00 00 00")]
pub fn debug_init_text_buffer() {
    DEBUG_TEXT_COUNT.set(0);
}

/// `CDebug::DebugDisplayTextBuffer()` (0x532260): empty in 1.0 US (`ret`; gr draws its own debug
/// strings). Not hookable (lone `ret`).
pub fn debug_display_text_buffer() {}

/// `LittleTest()` (0x541330, cdecl): `++g_nNumIm3dDrawCalls` (the increment sits in a protection chunk
/// at 0x409B56).
#[hook(addr = 0x541330, conv = cdecl, name = "Common::LittleTest", expect = "90 E9 20 88 EC FF")]
pub fn little_test() {
    NUM_IM3D_DRAW_CALLS.set(NUM_IM3D_DRAW_CALLS.get().wrapping_add(1));
}

/// `WriteRaster(raster, filename)` (0x5A4150, cdecl): RwImage of the raster's width/height/depth
/// (+0x0C/+0x10/+0x14), allocate, copy from the raster, write as PNG, destroy.
#[hook(addr = 0x5A4150, conv = cdecl, name = "Common::WriteRaster", expect = "56 57 8B 7C 24 0C")]
pub fn write_raster(raster: GPtr<u8>, filename: GPtr<u8>) {
    let rd = |o: isize| raster.byte_add(o).cast::<i32>().read();
    let img = rw_image_create(rd(0xC), rd(0x10), rd(0x14));
    rw_image_allocate_pixels(img);
    rw_image_set_from_raster(img, raster);
    rt_png_image_write(img, filename);
    rw_image_destroy(img);
}

/// `AsciiToGxtChar(src, dst)` (0x718600, cdecl): byte copy including the NUL (`GxtChar` is a byte
/// on PC).
#[hook(addr = 0x718600, conv = cdecl, name = "Common::AsciiToGxtChar", expect = "8B 4C 24 04 8A 01")]
pub fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) {
    let mut i = 0;
    loop {
        let c = src.wrapping_index(i).read();
        dst.wrapping_index(i).write(c);
        if c == 0 {
            return;
        }
        i += 1;
    }
}

fn upper(c: u8) -> u8 {
    if c.is_ascii_lowercase() { c - 0x20 } else { c }
}

/// `MakeUpperCase(dst, src)` (0x7186E0, cdecl): returns `dst` (its start).
#[hook(addr = 0x7186E0, conv = cdecl, name = "Common::MakeUpperCase-", expect = "8B 44 24 04 56")]
pub fn make_upper_case(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let mut i = 0;
    loop {
        let c = src.wrapping_index(i).read();
        if c == 0 {
            break;
        }
        dst.wrapping_index(i).write(upper(c));
        i += 1;
    }
    dst.wrapping_index(i).write(0);
    dst
}

/// `MakeUpperCase(dst)` (0x718710, cdecl), in place. Only lowercase letters are written back.
#[hook(addr = 0x718710, conv = cdecl, name = "Common::MakeUpperCase-in", expect = "8B 4C 24 04 80 39 00")]
pub fn make_upper_case_in(dst: GPtr<u8>) {
    let mut i = 0;
    loop {
        let c = dst.wrapping_index(i).read();
        if c == 0 {
            return;
        }
        if c.is_ascii_lowercase() {
            dst.wrapping_index(i).write(c - 0x20);
        }
        i += 1;
    }
}

/// `CalcScreenCoors(in, out)` (0x71DAB0, cdecl, bool in AL): `out = view * in`; false unless
/// `out.z > 1` (NaN: false); then `x *= W * (1/z)`, `y *= H * (1/z)` (`W`, `H` by `fild`).
#[hook(addr = 0x71DAB0, conv = cdecl, name = "Common::CalcScreenCoors-VV", expect = "8B 44 24 04 83 EC 0C")]
pub fn calc_screen_coors(input: GPtr<[f32; 3]>, out: GPtr<[f32; 3]>) -> bool {
    project(input, out).is_some()
}

/// The common part of both `CalcScreenCoors`: returns `1/z` (still on the x87 stack) on success.
fn project(input: GPtr<[f32; 3]>, out: GPtr<[f32; 3]>) -> Option<X87> {
    let mut tmp = [0f32; 3];
    let t = matrix_mul_vector(GPtr::from_raw(tmp.as_mut_ptr().cast()), GPtr::from_addr(CAMERA_VIEW_MATRIX as usize), input);
    // three dword moves through the returned pointer
    let r = t.cast::<[u32; 3]>().read();
    out.cast::<[u32; 3]>().write(r);
    let o = out.cast::<f32>();
    let z = o.wrapping_index(2).read();
    if ld(z).fcom(X87::constant(k::ONE)).test_any(0x41) {
        return None;
    }
    let recip = X87::constant(k::ONE).fdiv_m(o.wrapping_index(2).read());
    let x = X87::from_i32(SCREEN_WIDTH.get()).fmul(recip).fmul_m(o.read());
    o.write(x.to_f32());
    let y = X87::from_i32(SCREEN_HEIGHT.get()).fmul(recip).fmul_m(o.wrapping_index(1).read());
    o.wrapping_index(1).write(y.to_f32());
    Some(recip)
}

/// `CalcScreenCoors(in, out, scaleX, scaleY)` (0x71DA00, cdecl, bool in AL): as the two-argument
/// form, then `*sx = W/z`, `*sy = H/z` (f32 stores), then each reloaded and turned into
/// `v / CDraw::ms_fFOV * 70`.
#[hook(addr = 0x71DA00, conv = cdecl, name = "Common::CalcScreenCoors-VVff", expect = "8B 44 24 04 83 EC 0C")]
pub fn calc_screen_coors_scaled(input: GPtr<[f32; 3]>, out: GPtr<[f32; 3]>, sx: GPtr<f32>, sy: GPtr<f32>) -> bool {
    let Some(recip) = project(input, out) else {
        return false;
    };
    sx.write(X87::from_i32(SCREEN_WIDTH.get()).fmul(recip).to_f32());
    sy.write(X87::from_i32(SCREEN_HEIGHT.get()).fmul(recip).to_f32());
    let seventy = f32::from_bits(k::SEVENTY);
    sx.write(ld(sx.read()).fdiv_m(DRAW_FOV.get()).fmul_m(seventy).to_f32());
    sy.write(ld(sy.read()).fdiv_m(DRAW_FOV.get()).fmul_m(seventy).to_f32());
    true
}

/// `(p - o) x d` as the original computes it: `(px - ox) * dy - (py - oy) * dx` (PC=24).
fn side(px: f32, py: f32, ox: f32, oy: f32, dx: f32, dy: f32) -> X87 {
    ld(px).fsub_m(ox).fmul_m(dy).fsub(ld(py).fsub_m(oy).fmul_m(dx))
}

/// 0x71DB30 (cdecl, EAX = 0/1; gta-reversed `DoesInfiniteLineCrossFiniteLine`, `@unk` there): the
/// finite segment `a`–`b` has its ends strictly on opposite sides of the infinite line `o + t*d`
/// (`side(a) * side(b) < 0`; NaN: 0).
#[hook(addr = 0x71DB30, conv = cdecl, name = "FUN_0071db30", expect = "D9 44 24 04 D8 64 24 14")]
#[allow(clippy::too_many_arguments)]
pub fn does_infinite_line_cross_finite_line(ax: f32, ay: f32, bx: f32, by: f32, ox: f32, oy: f32, dx: f32, dy: f32) -> i32 {
    let p = side(ax, ay, ox, oy, dx, dy).fmul(side(bx, by, ox, oy, dx, dy));
    // fcomp 0.0; test ah, 5; jp -> 0
    (!p.fcom(X87::zero()).test_parity(5)) as i32
}

/// `DoesInfiniteLineTouchScreen(ox, oy, dx, dy)` (0x71DB80, cdecl, bool in AL), for a given screen
/// size (`RsGlobal.maximumWidth/Height`).
pub fn infinite_line_touches_screen(w: i32, h: i32, ox: f32, oy: f32, dx: f32, dy: f32) -> bool {
    let zero = X87::zero();
    if ld(ox).fcom(zero) == sa_math::x87::Cmp::Greater
        && ld(oy).fcom(zero) == sa_math::x87::Cmp::Greater
        && !X87::from_i32(w).fcom(ld(ox)).test_any(0x41)
        && !X87::from_i32(h).fcom(ld(oy)).test_any(0x41)
    {
        return true;
    }
    let wf = X87::from_i32(w).to_f32();
    let t1 = ld(oy).fchs().fmul_m(dx);
    let t2 = ld(ox).fchs().fmul_m(dy);
    let s0 = t2.to_f32();
    let s1 = t2.fsub(t1).to_f32();
    // top edge (0,0)-(W,0)
    let v = ld(wf).fsub_m(ox).fmul_m(dy).fsub(t1).fmul_m(s1);
    if !v.fcom(zero).test_parity(5) {
        return true;
    }
    // left edge (0,0)-(0,H): the exact `fild` value of H, its f32 copy for the calls below
    let hx = X87::from_i32(h);
    let hf = hx.to_f32();
    let v = hx.fsub_m(oy).fmul_m(dx).fsubr_m(s0).fmul_m(s1);
    if !v.fcom(zero).test_parity(5) {
        return true;
    }
    if does_infinite_line_cross_finite_line_call(wf, 0.0, wf, hf, ox, oy, dx, dy) != 0 {
        return true;
    }
    does_infinite_line_cross_finite_line_call(0.0, hf, wf, hf, ox, oy, dx, dy) != 0
}

/// `DoesInfiniteLineTouchScreen(ox, oy, dx, dy)` (0x71DB80, cdecl, bool in AL).
#[hook(addr = 0x71DB80, conv = cdecl, name = "Common::DoesInfiniteLineTouchScreen", expect = "D9 44 24 04 83 EC 10")]
pub fn does_infinite_line_touch_screen(ox: f32, oy: f32, dx: f32, dy: f32) -> bool {
    infinite_line_touches_screen(SCREEN_WIDTH.get(), SCREEN_HEIGHT.get(), ox, oy, dx, dy)
}

/// `IsPointInsideLine(ox, oy, dx, dy, px, py, r)` (0x71E050, cdecl, bool in AL):
/// `!((px - ox) * dy - (py - oy) * dx < r)`.
#[hook(addr = 0x71E050, conv = cdecl, name = "Common::IsPointInsideLine", expect = "D9 44 24 14 D8 64 24 04")]
pub fn is_point_inside_line(ox: f32, oy: f32, dx: f32, dy: f32, px: f32, py: f32, r: f32) -> bool {
    side(px, py, ox, oy, dx, dy).fcom(ld(r)).test_parity(5)
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_DEBUG_INIT_TEXT_BUFFER,
    HOOK_LITTLE_TEST,
    HOOK_WRITE_RASTER,
    HOOK_ASCII_TO_GXT_CHAR,
    HOOK_MAKE_UPPER_CASE,
    HOOK_MAKE_UPPER_CASE_IN,
    HOOK_CALC_SCREEN_COORS,
    HOOK_CALC_SCREEN_COORS_SCALED,
    HOOK_DOES_INFINITE_LINE_CROSS_FINITE_LINE,
    HOOK_DOES_INFINITE_LINE_TOUCH_SCREEN,
    HOOK_IS_POINT_INSIDE_LINE,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn strings() {
        let src = *b"aZ{z\xE9\0";
        let mut dst = [0xAAu8; 8];
        let d = GPtr::from_raw(dst.as_mut_ptr());
        assert_eq!(make_upper_case(d, GPtr::from_raw(src.as_ptr() as *mut u8)), d);
        assert_eq!(&dst[..6], b"AZ{Z\xE9\0");
        let mut s = *b"abc\0x";
        make_upper_case_in(GPtr::from_raw(s.as_mut_ptr()));
        assert_eq!(&s, b"ABC\0x");
        let mut g = [9u8; 4];
        ascii_to_gxt_char(GPtr::from_raw(c"hi".as_ptr() as *mut u8), GPtr::from_raw(g.as_mut_ptr()));
        assert_eq!(g, [b'h', b'i', 0, 9]);
        little_test();
        assert_eq!(NUM_IM3D_DRAW_CALLS.get(), 1);
        DEBUG_TEXT_COUNT.set(3);
        debug_init_text_buffer();
        assert_eq!(DEBUG_TEXT_COUNT.get(), 0);
        debug_display_text_buffer();
    }

    #[test]
    fn lines() {
        assert!(is_point_inside_line(0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0));
        assert!(!is_point_inside_line(0.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0));
        assert!(is_point_inside_line(0.0, 0.0, 0.0, 1.0, f32::NAN, 0.0, 0.0));
        assert_eq!(does_infinite_line_cross_finite_line(-1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0), 1);
        assert_eq!(does_infinite_line_cross_finite_line(0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0), 0);
        assert!(infinite_line_touches_screen(640, 480, 10.0, 10.0, 1.0, 0.0));
        assert!(!infinite_line_touches_screen(640, 480, 0.0, 10.0, 0.0, 0.0));
        // horizontal line y = 100 starting left of the screen crosses the left edge
        assert!(infinite_line_touches_screen(640, 480, -5.0, 100.0, 1.0, 0.0));
        assert!(!infinite_line_touches_screen(640, 480, -5.0, 500.0, 1.0, 0.0));
    }
}
