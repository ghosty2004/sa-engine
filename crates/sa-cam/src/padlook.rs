//! The two unnamed `CPad` stick readers of the on-foot PC camera (`FUN_00540E80`, `FUN_00540F80`; thiscall, AX),
//! ported from the exe bytes for native sa-game (SAMP-4: the camera after a SA:MP spawn reaches them every frame).
//! Registered for phase B only (`#[native]`) until their L2 run.
//!
//! Both read pad 0's right stick from the globals (not from `this`): x at 0xB7345C, y at 0xB7345E (negated when
//! the invert flag 0xB73402 is set). Beyond 100 the stick drives the look directly (value ∓ 50), unless the
//! player's controls are enabled (`this` +0x10E == 0) and R3 is held (`this` +0x26 != 0); otherwise, while
//! `TheCamera.m_aCams[0]` (0xB6F19C) uses the third-person mouse camera, values beyond 50 give `(value ∓ 50) ·
//! 0.5` (float 0x858B8C) through `_ftol` (0x821B40); else 0.

use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{ftol32, g, rd};

/// `Pads[0].NewState.RightStickX` / `RightStickY`, the invert-look flag.
const RIGHT_STICK_X: usize = 0xB7345C;
const RIGHT_STICK_Y: usize = 0xB7345E;
const INVERT_LOOK: usize = 0xB73402;
/// `TheCamera.m_aCams[0]`.
const CAM0: usize = 0xB6F19C;
/// The floats 100.0 (0x858628), 50.0 (0x858B40), 0.5 (0x858B8C).
const K100: usize = 0x858628;
const K50: usize = 0x858B40;
const KHALF: usize = 0x858B8C;

/// The direct-stick branch is skipped while the controls are enabled and R3 is held.
fn r3_look(pad: GPtr<u8>) -> bool {
    rd::<u16>(pad, 0x10E) == 0 && rd::<u16>(pad, 0x26) != 0
}

fn mouse_cam() -> bool {
    crate::camproc::using_3rd_person_mouse_cam(GPtr::from_addr(CAM0)) != 0
}

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `FUN_00540E80`: the look left/right value (float compares, as the original).
pub fn pad_look_x(pad: GPtr<u8>) -> i16 {
    let s = X87::from_f32(f32::from(g::<i16>(RIGHT_STICK_X).read()));
    let neg = s.to_f32() <= 0.0;
    if s.to_f32().abs() > gf(K100) && !r3_look(pad) {
        let v = if neg {
            s.fadd_m(gf(K100)).fsub_m(gf(K50))
        } else {
            s.fsub_m(gf(K100)).fadd_m(gf(K50))
        };
        return ftol32(v) as i16;
    }
    if !mouse_cam() || s.to_f32().abs() <= gf(K50) {
        return 0;
    }
    let v = if neg { s.fadd_m(gf(K50)) } else { s.fsub_m(gf(K50)) };
    ftol32(v.fmul_m(gf(KHALF))) as i16
}

/// `FUN_00540F80`: the look up/down value (integer compares, as the original).
pub fn pad_look_y(pad: GPtr<u8>) -> i16 {
    let mut di = g::<i16>(RIGHT_STICK_Y).read();
    if g::<u8>(INVERT_LOOK).read() != 0 {
        di = di.wrapping_neg();
    }
    let esi = i32::from(di);
    if esi.abs() > 100 && !r3_look(pad) {
        return (if di > 0 { esi - 50 } else { esi + 50 }) as i16;
    }
    if !mouse_cam() || esi.abs() <= 50 {
        return 0;
    }
    let v = if di > 0 { esi - 50 } else { esi + 50 };
    ftol32(X87::from_i32(v).fmul_m(gf(KHALF))) as i16
}
