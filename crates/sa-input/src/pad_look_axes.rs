//! The two right-stick look helpers of `CPad` that the catalogue left unnamed (gta-reversed `Pad.h` lists them
//! as `0x540E80` / `0x540F80` without names), batch `samp-unk-1`. Proposed names `CPad::GetLookLeftRightAxis` /
//! `CPad::GetLookUpDownAxis` (`docs/samp/native-unported-names.csv`): callers are the camera's aim and follow modes
//! 0x50EB70, 0x50F970 and `CPlayerPed`'s look code 0x527FA0. Ported from the 1.0 US bytes. Both return a 16-bit
//! value in AX (every caller reads AX only: `movsx` or `test ax, ax`). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd};
use sa_core::mem::GPtr;

orig! {
    /// `CCam::Using3rdPersonMouseCam()` (0x50A850, AL)
    fn cam_using_3rd_person_mouse_cam(cam: GPtr<u8>) -> u8 = thiscall 0x50A850 => unported;
}

/// `CPad::GetPad(0)->NewState.RightStickX` (0xB7345C) and `.RightStickY` (0xB7345E): read from pad 0 whatever
/// `this` is.
const PAD0_RIGHT_STICK_X: usize = 0xB7_345C;
const PAD0_RIGHT_STICK_Y: usize = 0xB7_345E;
/// `CPad::bInvertLook4Pad` (0xB73402)
const INVERT_LOOK_4_PAD: usize = 0xB7_3402;
/// `TheCamera.m_aCams[0]` (0xB6F19C)
const CAM0: usize = 0xB6_F19C;
/// `.rdata`: 0.0 (0x858B50), 100.0 (0x858628), 50.0 (0x858B40), 0.5 (0x858B8C)
const K_ZERO: usize = 0x85_8B50;
const K_100: usize = 0x85_8628;
const K_50: usize = 0x85_8B40;
const K_HALF: usize = 0x85_8B8C;

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// The stick outside the pad dead zone is used unless the pad is the one being driven by the mouse:
/// `this->DisablePlayerControls` (+0x10E word) set, or `this +0x26` word clear.
fn stick_mode(pad: GPtr<u8>) -> bool {
    rd::<u16>(pad, 0x10E) != 0 || rd::<u16>(pad, 0x26) == 0
}

/// `x87 > 0` as `fcomp [0.0]; test ah, 0x41; jne` reads it (false for zero, negative and unordered).
fn positive(v: X87) -> bool {
    !v.fcom(X87::from_f32(k(K_ZERO))).test_any(0x41)
}

/// `|v|` the way the bytes compute it: `fchs` only when `fcomp [0.0]` sets C0 alone (`test ah, 5; jp` skips it).
fn abs_x87(v: X87) -> X87 {
    if v.fcom(X87::from_f32(k(K_ZERO))).test_parity(5) {
        v
    } else {
        v.fchs()
    }
}

/// `FUN_00540e80` (0x540E80, thiscall `CPad*`; proposed `CPad::GetLookLeftRightAxis`): `x` = pad 0's right stick X
/// as float (`fild`, stored by `fst`). When `|x| > 100` and [`stick_mode`]: `_ftol(x - 100 + 50)` for `x > 0`,
/// else `_ftol(x + 100 - 50)`. Otherwise, when `TheCamera.m_aCams[0].Using3rdPersonMouseCam()` and `|x| > 50`:
/// `_ftol((x - 50) * 0.5)` / `_ftol((x + 50) * 0.5)`; else 0 (`xor ax, ax`).
#[hook(addr = 0x540E80, conv = thiscall, name = "FUN_00540e80", expect = "51 0F BF 05 5C 34 B7 00",
       unverified = "samp-unk-1", route = "L2", hazards = "none", recs = "anim-input",
       note = "AX result; the upper EAX bits differ by path in the original, no caller reads them")]
pub fn pad_look_left_right_axis(pad: GPtr<u8>) -> i16 {
    let xv = X87::from_i32(i32::from(g::<i16>(PAD0_RIGHT_STICK_X).read()));
    let x = X87::from_f32(xv.to_f32());
    if !abs_x87(x).fcom(X87::from_f32(k(K_100))).test_any(0x41) && stick_mode(pad) {
        let r = if positive(x) {
            x.fsub_m(k(K_100)).fadd_m(k(K_50))
        } else {
            x.fadd_m(k(K_100)).fsub_m(k(K_50))
        };
        return r.ftol() as u16 as i16;
    }
    if cam_using_3rd_person_mouse_cam(GPtr::from_addr(CAM0)) != 0
        && !abs_x87(x).fcom(X87::from_f32(k(K_50))).test_any(0x41)
    {
        let r = if positive(x) {
            x.fsub_m(k(K_50)).fmul_m(k(K_HALF))
        } else {
            x.fadd_m(k(K_50)).fmul_m(k(K_HALF))
        };
        return r.ftol() as u16 as i16;
    }
    0
}

/// `FUN_00540f80` (0x540F80, thiscall `CPad*`; proposed `CPad::GetLookUpDownAxis`): `y` = pad 0's right stick Y,
/// negated (16-bit) when `bInvertLook4Pad`. When `|y| > 100` and [`stick_mode`]: `y - 50` for `y > 0`, else
/// `y + 50` (integer). Otherwise, when `TheCamera.m_aCams[0].Using3rdPersonMouseCam()` and `|y| > 50`:
/// `_ftol(float(y - 50) * 0.5)` / `_ftol(float(y + 50) * 0.5)`; else 0.
#[hook(addr = 0x540F80, conv = thiscall, name = "FUN_00540f80", expect = "51 A0 02 34 B7 00",
       unverified = "samp-unk-1", route = "L2", hazards = "none", recs = "anim-input",
       note = "AX result; the upper EAX bits differ by path in the original, no caller reads them")]
pub fn pad_look_up_down_axis(pad: GPtr<u8>) -> i16 {
    let mut y = g::<i16>(PAD0_RIGHT_STICK_Y).read();
    if g::<u8>(INVERT_LOOK_4_PAD).read() != 0 {
        y = y.wrapping_neg();
    }
    let yi = i32::from(y);
    if yi.abs() > 100 && stick_mode(pad) {
        return if y > 0 { (yi - 50) as i16 } else { (yi + 50) as i16 };
    }
    if cam_using_3rd_person_mouse_cam(GPtr::from_addr(CAM0)) != 0 && yi.abs() > 50 {
        let v = if y > 0 { yi - 50 } else { yi + 50 };
        return X87::from_i32(v).fmul_m(k(K_HALF)).ftol() as u16 as i16;
    }
    0
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PAD_LOOK_LEFT_RIGHT_AXIS, HOOK_PAD_LOOK_UP_DOWN_AXIS];
