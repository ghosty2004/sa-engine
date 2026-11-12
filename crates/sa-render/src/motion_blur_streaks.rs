//! `CMotionBlurStreaks::RegisterStreak` and `CRegisteredMotionBlurStreak::Update` / `Render` (batch
//! `samp-render-2`), ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
//!
//! `CMotionBlurStreaks::aStreaks[4]` (0x54 each) at 0xC7CA08: +0 id, +4..+7 red, green, blue, alpha, +8/+0x14/+0x20
//! the three start points (newest first), +0x2C/+0x38/+0x44 the three end points, +0x50..+0x52 the three valid
//! flags. The render vertices are `TempBufferVertices` 0xC7C628 (4 `RwIm3DVertex`, 0x24 each) and the indices
//! 0xC7C6B8.
#![allow(clippy::too_many_arguments)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, k, rd, wr};

orig! {
    /// `Common::LittleTest()` (0x541330)
    fn little_test() = cdecl 0x541330 => unported;
    /// `RwIm3DTransform(RwIm3DVertex*, uint32 count, RwMatrix*, uint32 flags)` (0x7EF450)
    fn rw_im3d_transform(v: u32, n: u32, m: u32, flags: u32) -> u32 = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(RwPrimitiveType, RwImVertexIndex*, int32)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(ty: u32, idx: u32, n: u32) -> u32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> u32 = cdecl 0x7EF520 => unported;
}

const STREAKS: usize = 0xC7_CA08;
const STREAKS_END: usize = 0xC7_CB58;
const STREAK_SIZE: usize = 0x54;
const VERTS: usize = 0xC7_C628;
const INDICES: u32 = 0xC7_C6B8;

fn gp<T: sa_core::mem::GameData>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

/// The record fields `RegisterStreak` writes: colour bytes +4..+7, start point +8, end point +0x2C, valid[0] = 1.
fn fill(s: GPtr<u8>, rgba: [u8; 4], start: [u32; 3], end: [u32; 3]) {
    for (i, c) in rgba.into_iter().enumerate() {
        wr::<u8>(s, 4 + i as isize, c);
    }
    for (i, v) in start.into_iter().enumerate() {
        wr::<u32>(s, 8 + 4 * i as isize, v);
    }
    for (i, v) in end.into_iter().enumerate() {
        wr::<u32>(s, 0x2C + 4 * i as isize, v);
    }
    wr::<u8>(s, 0x50, 1);
}

/// `CMotionBlurStreaks::RegisterStreak(uint32 id, uint8 r, uint8 g, uint8 b, uint8 a, CVector start, CVector end)`
/// (0x721DC0, cdecl; its body continues at 0x721DD0, the catalogue's FUN_00721dd0, through the no-CD chunk 0x401C79
/// which only loads EAX = 0xC7CA08): the record with this id gets the colour, the newest start and end points and
/// valid[0] = 1; otherwise the first record with id 0 gets the id, the same fields and valid[1] = valid[2] = 0;
/// with neither, nothing.
#[hook(addr = 0x721DC0, conv = cdecl, name = "CMotionBlurStreaks::RegisterStreak", expect = "8B 54 24 04 33 C9 E9 AE FE CD FF",
       unverified = "samp-render-2", route = "L2", hazards = "none", recs = "driving")]
pub fn motion_blur_streaks_register_streak(id: u32, r: u32, g: u32, b: u32, a: u32, sx: u32, sy: u32, sz: u32, ex: u32,
    ey: u32, ez: u32) {
    let rgba = [r as u8, g as u8, b as u8, a as u8];
    let mut p = STREAKS;
    while p < STREAKS_END {
        if gp::<u32>(p).read() == id {
            fill(gp(p), rgba, [sx, sy, sz], [ex, ey, ez]);
            return;
        }
        p += STREAK_SIZE;
    }
    let mut p = STREAKS;
    while p < STREAKS_END {
        if gp::<u32>(p).read() == 0 {
            let s = gp::<u8>(p);
            wr::<u32>(s, 0, id);
            fill(s, rgba, [sx, sy, sz], [ex, ey, ez]);
            wr::<u16>(s, 0x51, 0);
            return;
        }
        p += STREAK_SIZE;
    }
}

/// `CRegisteredMotionBlurStreak::Update()` (0x721EE0, thiscall): the history moves one step older (start, end and
/// valid flag of entry 1 to 2, then 0 to 1); valid[0] := 0; when neither moved flag was set the id := 0.
#[hook(addr = 0x721EE0, conv = thiscall, name = "CRegisteredMotionBlurStreak::Update", expect = "83 EC 08 53 55",
       unverified = "samp-render-2", route = "L2", hazards = "none", recs = "driving")]
pub fn registered_motion_blur_streak_update(this: GPtr<u8>) {
    let mut any = false;
    for i in [2isize, 1] {
        for j in 0..3 {
            wr::<u32>(this, 8 + 0xC * i + 4 * j, rd::<u32>(this, 8 + 0xC * (i - 1) + 4 * j));
        }
        for j in 0..3 {
            wr::<u32>(this, 0x2C + 0xC * i + 4 * j, rd::<u32>(this, 0x2C + 0xC * (i - 1) + 4 * j));
        }
        let v = rd::<u8>(this, 0x50 + i - 1);
        wr::<u8>(this, 0x50 + i, v);
        if any || v != 0 {
            any = true;
        }
    }
    wr::<u8>(this, 0x50, 0);
    if !any {
        wr::<u32>(this, 0, 0);
    }
}

/// `CRegisteredMotionBlurStreak::Render()` (0x721F70, thiscall): for the segments i = 0, 1 whose two ends are valid:
/// alphas `a0 = _ftol(((0xFF - 0x55 i) / 3 * alpha) * (1/255))` and `a1 = _ftol(((0xAA - 0x55 i) / 3 * alpha) *
/// (1/255))` (integer division, then `fild`/`fmul`), the four vertices' colours `a << 24 | r << 16 | g << 8 | b`
/// (a0 for the newer pair), positions start[i], end[i], start[i+1], end[i+1]; `Common::LittleTest()`;
/// `RwIm3DTransform(verts, 4, nullptr, 1)`; when it succeeds `RwIm3DRenderIndexedPrimitive(3, indices, 12)` and
/// `RwIm3DEnd()`.
#[hook(addr = 0x721F70, conv = thiscall, name = "CRegisteredMotionBlurStreak::Render", expect = "83 EC 0C 53 55 56",
       unverified = "samp-render-2", route = "L2", hazards = "none", recs = "driving")]
pub fn registered_motion_blur_streak_render(this: GPtr<u8>) {
    let mut e = 0xAAi32;
    let mut i = 0isize;
    while e > 0 {
        if rd::<u8>(this, 0x50 + i) != 0 && rd::<u8>(this, 0x51 + i) != 0 {
            let alpha = i32::from(rd::<u8>(this, 7));
            let a0 = ftol32(X87::from_i32(((e + 0x55) / 3).wrapping_mul(alpha)).fmul_m(k(0x3B80_8081)));
            let a1 = ftol32(X87::from_i32((e / 3).wrapping_mul(alpha)).fmul_m(k(0x3B80_8081)));
            let (r, g, b) = (u32::from(rd::<u8>(this, 4)), u32::from(rd::<u8>(this, 5)), u32::from(rd::<u8>(this, 6)));
            let col = |a: i32| ((((a as u32) << 8 | r) << 8 | g) << 8) | b;
            let v = gp::<u8>(VERTS);
            wr::<u32>(v, 0x18, col(a0));
            wr::<u32>(v, 0x3C, col(a0));
            wr::<u32>(v, 0x60, col(a1));
            wr::<u32>(v, 0x84, col(a1));
            for (vo, so) in [(0, 8), (0x24, 0x2C), (0x48, 0x14), (0x6C, 0x38)] {
                for j in 0..3 {
                    wr::<u32>(v, vo + 4 * j, rd::<u32>(this, so + 0xC * i + 4 * j));
                }
            }
            little_test();
            if rw_im3d_transform(VERTS as u32, 4, 0, 1) != 0 {
                rw_im3d_render_indexed_primitive(3, INDICES, 0xC);
                rw_im3d_end();
            }
        }
        i += 1;
        e -= 0x55;
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_MOTION_BLUR_STREAKS_REGISTER_STREAK,
    HOOK_REGISTERED_MOTION_BLUR_STREAK_UPDATE,
    HOOK_REGISTERED_MOTION_BLUR_STREAK_RENDER,
];
