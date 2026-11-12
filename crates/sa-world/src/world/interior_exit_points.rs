//! WRITER-WORLD-2 (batch `world6-2`, SA:MP-executed): `Interior_c::CalcExitPts`, the door points
//! and their links to the walk nodes for the four sides of an interior. Ported from the 1.0 US
//! bytes; gta-reversed has it unreversed.

#![allow(clippy::needless_late_init)] // one assignment per branch, as the bytes compute them

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{rd, wr, x, xi};

orig! {
    /// `Interior_c::GetTileCentre(float x, float y, CVector* out)` (0x591BD0)
    fn get_tile_centre(it: GPtr<u8>, tx: f32, ty: f32, out: GPtr<u8>) = thiscall 0x591BD0 => unported;
}

/// `0.5` (0x858B8C), `0.25` (0x858C84).
const F_0_5: f32 = 0.5;
const F_0_25: f32 = 0.25;

/// `GetTileCentre(tx, ty)` into `this + dst` (copied as dwords).
fn centre_to(this: GPtr<u8>, tx: f32, ty: f32, dst: isize) {
    let mut out = [0u32; 3];
    get_tile_centre(this, tx, ty, GPtr::from_raw(&mut out as *mut [u32; 3]).cast());
    for (k, v) in out.iter().enumerate() {
        wr::<u32>(this, dst + 4 * k as isize, *v);
    }
}

/// `(float)int8` of a byte field (`movsx; fild`).
fn fb(p: GPtr<u8>, o: isize) -> X87 {
    xi(i32::from(rd::<i8>(p, o)))
}

/// The middle of a door span: `f32(n * 0.5 + start - 0.5)` (`fild n; fmul; fiadd start; fsub`).
fn span_mid(info: GPtr<u8>, a: isize) -> f32 {
    let s = i32::from(rd::<i8>(info, a));
    let n = i32::from(rd::<i8>(info, a + 1)) - s;
    xi(n).fmul_m(F_0_5).fadd(xi(s)).fsub_m(F_0_5).to_f32()
}

/// `Interior_c::CalcExitPts()` (0x5924A0). For each open side of the interior info (+5 / +6 / +0xA /
/// +8 > -1) the door's middle along its wall (`start - 0.5` for side A, the span middle for the
/// others), the two walk nodes it links to (bytes +0x512 / +0x513 for A, +0x532.. B, +0x552.. C,
/// +0x572.. D; -1 = none) and two tile-centre points: the inner one (+0x514.. A, +0x534.. B, +0x554..
/// C, +0x574.. D) beside the nearest walk node, the outer one (+0x524.., +0x544.., +0x564..,
/// +0x584..) just outside the wall. With at most two walk nodes (+0x40C) the links are -1 and the
/// inner point sits on the wall. Side A scans the even nodes' x (+0x410 + 0x20k) for the first one
/// past the middle, side C the odd nodes' x; side B compares the y of nodes 0 / 1 with the middle,
/// side D those of the last two nodes.
#[hook(addr = 0x5924A0, conv = thiscall, name = "Interior_c::CalcExitPts", this = "Interior_c",
       expect = "83 EC 20 53 56",
       unverified = "world6-2", route = "L2", hazards = "none")]
pub fn calc_exit_pts(this: GPtr<u8>) {
    let info = || rd::<GPtr<u8>>(this, 0x14);
    let count = || rd::<i8>(this, 0x40C);
    // side A: the y = 0 wall, x = start - 0.5
    if rd::<i8>(info(), 5) > -1 {
        let mid = fb(info(), 5).fsub_m(F_0_5).to_f32();
        let ty;
        if count() > 2 {
            let n = i32::from(count());
            let mut k: i32 = 0;
            while k < i32::from(count()) {
                if !fb(this, 0x410 + 0x10 * k as isize).fcom(x(mid)).test_any(0x41) {
                    break;
                }
                k += 2;
            }
            if k == 0 {
                wr::<u8>(this, 0x512, 0);
                wr::<u8>(this, 0x513, 0xFF);
            } else if k == n {
                wr::<u8>(this, 0x512, (n as u8).wrapping_sub(2));
                wr::<u8>(this, 0x513, 0xFF);
            } else {
                wr::<u8>(this, 0x512, k as u8);
                wr::<u8>(this, 0x513, (k as u8).wrapping_sub(2));
            }
            ty = fb(this, 0x411).fsub_m(F_0_25).to_f32();
        } else {
            wr::<u8>(this, 0x512, 0xFF);
            wr::<u8>(this, 0x513, 0xFF);
            ty = 0.0;
        }
        centre_to(this, mid, ty, 0x514);
        centre_to(this, mid, -0.25, 0x524);
    }
    // side B: the x = 0 wall
    if rd::<i8>(info(), 6) > -1 {
        let mid = span_mid(info(), 6);
        let tx;
        if count() > 2 {
            if !fb(this, 0x411).fcom(x(mid)).test_any(0x41) {
                wr::<u8>(this, 0x533, 0xFF);
                wr::<u8>(this, 0x532, 0);
            } else if !fb(this, 0x421).fcom(x(mid)).test_parity(0x05) {
                wr::<u8>(this, 0x532, 1);
                wr::<u8>(this, 0x533, 0xFF);
            } else {
                wr::<u8>(this, 0x533, 1);
                wr::<u8>(this, 0x532, 0);
            }
            tx = fb(this, 0x410).fsub_m(F_0_25).to_f32();
        } else {
            wr::<u8>(this, 0x532, 0xFF);
            wr::<u8>(this, 0x533, 0xFF);
            tx = 0.0;
        }
        centre_to(this, tx, mid, 0x534);
        centre_to(this, -0.25, mid, 0x544);
    }
    // side C: the y = depth - 1 wall
    if rd::<i8>(info(), 0xA) > -1 {
        let mid = span_mid(info(), 0xA);
        let ty;
        if count() > 2 {
            let n = i32::from(count());
            let mut k: i32 = 1;
            if n > 1 {
                while k < i32::from(count()) {
                    if !fb(this, 0x420 + 0x10 * (k as isize - 1)).fcom(x(mid)).test_any(0x41) {
                        break;
                    }
                    k += 2;
                }
            }
            if k == 0 {
                wr::<u8>(this, 0x552, 1);
                wr::<u8>(this, 0x553, 0xFF);
            } else if k == n {
                wr::<u8>(this, 0x552, (n as u8).wrapping_sub(1));
                wr::<u8>(this, 0x553, 0xFF);
            } else {
                wr::<u8>(this, 0x552, k as u8);
                wr::<u8>(this, 0x553, (k as u8).wrapping_sub(2));
            }
            ty = fb(this, 0x421).fadd_m(F_0_25).to_f32();
        } else {
            wr::<u8>(this, 0x552, 0xFF);
            wr::<u8>(this, 0x553, 0xFF);
            ty = xi(i32::from(rd::<u8>(info(), 3)) - 1).to_f32();
        }
        centre_to(this, mid, ty, 0x554);
        let ty2 = xi(i32::from(rd::<u8>(info(), 3)) - 1).fadd_m(F_0_25).to_f32();
        centre_to(this, mid, ty2, 0x564);
    }
    // side D: the x = width - 1 wall
    if rd::<i8>(info(), 8) > -1 {
        let mid = span_mid(info(), 8);
        let tx;
        let c = count();
        if c > 2 {
            let n = isize::from(c);
            let cb = c as u8;
            if !fb(this, 0x3F1 + n * 0x10).fcom(x(mid)).test_any(0x41) {
                wr::<u8>(this, 0x572, cb.wrapping_sub(2));
                wr::<u8>(this, 0x573, 0xFF);
            } else if !fb(this, 0x401 + n * 0x10).fcom(x(mid)).test_parity(0x05) {
                wr::<u8>(this, 0x572, cb.wrapping_sub(1));
                wr::<u8>(this, 0x573, 0xFF);
            } else {
                wr::<u8>(this, 0x572, cb.wrapping_sub(1));
                wr::<u8>(this, 0x573, cb.wrapping_sub(2));
            }
            tx = fb(this, 0x3F0 + n * 0x10).fadd_m(F_0_25).to_f32();
        } else {
            wr::<u8>(this, 0x572, 0xFF);
            wr::<u8>(this, 0x573, 0xFF);
            tx = xi(i32::from(rd::<u8>(info(), 2)) - 1).to_f32();
        }
        centre_to(this, tx, mid, 0x574);
        let tx2 = xi(i32::from(rd::<u8>(info(), 2)) - 1).fadd_m(F_0_25).to_f32();
        centre_to(this, tx2, mid, 0x584);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CALC_EXIT_PTS];
