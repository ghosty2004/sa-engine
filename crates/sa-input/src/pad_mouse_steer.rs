//! STD-15 (batch `std15-input-1`): the `CPad` mouse-steering reader 0x540E80, reached every frame after the player
//! leaves a car. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::rd;

orig! {
    /// `CCam::Using3rdPersonMouseCam()` (0x50A850, on `TheCamera.Cams[0]` 0xB6F19C)
    fn cam_using_3rd_person_mouse_cam(cam: GPtr<u8>) -> u8 = thiscall 0x50A850 => unported;
}

fn gf(a: usize) -> f32 {
    GPtr::<f32>::from_addr(a).read()
}

/// `CPad` 0x540E80 (thiscall, EAX from `_ftol`; callers read AX). v = the mouse x delta (int16 0xB7345C) as f32.
/// With |v| > 100 and neither +0x10E nor +0x26 set: `_ftol(v - 100 + 50)` (v > 0) or `_ftol(v + 100 - 50)`.
/// Otherwise, with the third-person mouse camera and |v| > 50: `_ftol((v - 50) * 0.5)` / `_ftol((v + 50) * 0.5)`;
/// else 0 (the exe clears AX only).
#[hook(addr = 0x540E80, conv = thiscall, name = "FUN_00540e80", expect = "51 0F BF 05 5C 34 B7 00",
       unverified = "std15-input-1", route = "L2", hazards = "ax-al-return", recs = "car-enter",
       note = "the zero paths clear AX only (the upper EAX half is left from the previous instruction); 0 here")]
pub fn pad_mouse_steer_540e80(pad: GPtr<u8>) -> u32 {
    let v = X87::from_i32(i32::from(GPtr::<i16>::from_addr(0xB7345C).read())).to_f32();
    let zero = X87::from_f32(gf(0x858B50));
    let abs = |v: f32| {
        if X87::from_f32(v).fcom(zero).test_parity(0x05) {
            X87::from_f32(v)
        } else {
            X87::from_f32(v).fchs()
        }
    };
    let c100 = gf(0x858628);
    let c50 = gf(0x858B40);
    let big = !abs(v).fcom(X87::from_f32(c100)).test_any(0x41);
    if big && rd::<i16>(pad, 0x10E) == 0 && rd::<i16>(pad, 0x26) != 0 {
        // falls to the mouse-camera path
    } else if big {
        let r = if !X87::from_f32(v).fcom(zero).test_any(0x41) {
            X87::from_f32(v).fsub_m(c100).fadd_m(c50)
        } else {
            X87::from_f32(v).fadd_m(c100).fsub_m(c50)
        };
        return r.ftol() as u32;
    }
    if cam_using_3rd_person_mouse_cam(GPtr::from_addr(0xB6F19C)) == 0 {
        return 0;
    }
    if abs(v).fcom(X87::from_f32(c50)).test_any(0x41) {
        return 0;
    }
    let half = gf(0x858B8C);
    let r = if !X87::from_f32(v).fcom(zero).test_any(0x41) {
        X87::from_f32(v).fsub_m(c50).fmul_m(half)
    } else {
        X87::from_f32(v).fadd_m(c50).fmul_m(half)
    };
    r.ftol() as u32
}

/// `CPad` 0x540F80 (thiscall, EAX; callers read AX): the mouse y delta (int16 0xB7345E), negated (16-bit) with the
/// invert flag 0xB73402. With |v| > 100 and neither +0x10E nor +0x26 set: `v - 50` (v > 0) or `v + 50` (full
/// 32-bit EAX). Otherwise, with the third-person mouse camera and |v| > 50: `_ftol((float)(v - 50) * 0.5)` /
/// `_ftol((float)(v + 50) * 0.5)`; else 0 (AX).
#[hook(addr = 0x540F80, conv = thiscall, name = "FUN_00540f80", expect = "51 A0 02 34 B7 00",
       unverified = "std15-input-1", route = "L2", hazards = "ax-al-return", recs = "car-enter",
       note = "the zero paths clear AX only; 0 here")]
pub fn pad_mouse_steer_540f80(pad: GPtr<u8>) -> u32 {
    let mut v = GPtr::<i16>::from_addr(0xB7345E).read();
    if GPtr::<u8>::from_addr(0xB73402).read() != 0 {
        v = v.wrapping_neg();
    }
    let e = i32::from(v);
    let big = e.unsigned_abs() > 100;
    if big && !(rd::<i16>(pad, 0x10E) == 0 && rd::<i16>(pad, 0x26) != 0) {
        return if v > 0 { (e - 50) as u32 } else { (e + 50) as u32 };
    }
    if cam_using_3rd_person_mouse_cam(GPtr::from_addr(0xB6F19C)) == 0 {
        return 0;
    }
    if e.unsigned_abs() <= 50 {
        return 0;
    }
    let half = gf(0x858B8C);
    let r = if v > 0 { e - 50 } else { e + 50 };
    X87::from_i32(r).fmul_m(half).ftol() as u32
}

pub const HOOKS: &[HookReg] = &[HOOK_PAD_MOUSE_STEER_540E80, HOOK_PAD_MOUSE_STEER_540F80];
