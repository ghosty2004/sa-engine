//! PORT-VEH-4 (batch `veh4-3`): `CHeli::ProcessControlInputs`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, jne_mask, jp5, rd, wr, x, xi};

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0): int16 in AX
    fn pad_get_accelerate(pad: GPtr<u8>) -> i16 = thiscall 0x5403F0 => unported;
    /// `CPad::GetBrake()` (0x540080): int16 in AX
    fn pad_get_brake(pad: GPtr<u8>) -> i16 = thiscall 0x540080 => unported;
    /// `CPad::GetSteeringLeftRight()` (0x53FB80): int16 in AX
    fn pad_get_steering_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FB80 => unported;
    /// `CPad::GetSteeringUpDown()` (0x53FBD0): int16 in AX
    fn pad_get_steering_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FBD0 => unported;
    /// `CPad::GetLookLeft()` (0x53FDD0, AL)
    fn pad_get_look_left(pad: GPtr<u8>) -> u8 = thiscall 0x53FDD0 => unported;
    /// `CPad::GetLookRight()` (0x53FE10, AL)
    fn pad_get_look_right(pad: GPtr<u8>) -> u8 = thiscall 0x53FE10 => unported;
    /// `CPad::GetHorn()` (0x53FEE0, AL)
    fn pad_get_horn(pad: GPtr<u8>) -> u8 = thiscall 0x53FEE0 => unported;
    /// `CrossProduct(CVector* out, const CVector&, const CVector&)` (0x59C730)
    fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(n: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlayerPed::KeepAreaAroundPlayerClear()` (0x60C1E0)
    fn keep_area_around_player_clear(p: GPtr<u8>) = thiscall 0x60C1E0 => unported;
}

/// A stack buffer of `N` dwords as a game pointer.
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// `pow(0.98, timeStep)` (`_CIpow`).
fn decay() -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_binary(
        0x822130,
        x(g::<f32>(0x87_167C).read()).to_f64(),
        x(g::<f32>(0xB7_CB5C).read()).to_f64(),
    ))
}

/// Clamps to [-1, 1] as the original compares (`1 < v` gives 1, `-1 > v` gives -1, NaN stays).
fn clamp1(v: f32) -> f32 {
    if !jp5(x(1.0), x(v)) {
        1.0
    } else if !jne_mask(x(-1.0), x(v), 0x41) {
        -1.0
    } else {
        v
    }
}

/// `(a.y * ms.y + a.z * ms.z) + a.x * ms.x` scaled by a flying-handling factor, clamped to [-2, 2]
/// (recomputed per comparison, as the original does).
fn stabiliser(this: GPtr<u8>, a: &[u32; 3], k: isize) -> f32 {
    let s = || {
        let f = |i: usize| f32::from_bits(a[i]);
        x(f(1))
            .fmul_m(rd::<f32>(this, 0x48))
            .fadd(x(f(2)).fmul_m(rd::<f32>(this, 0x4C)))
            .fadd(x(f(0)).fmul_m(rd::<f32>(this, 0x44)))
            .fmul_m(rd::<f32>(rd::<GPtr<u8>>(this, 0x388), k))
    };
    if !s().fcom(x(2.0)).test_any(0x41) || jne_mask(x(-2.0), s(), 0x41) {
        if !jne_mask(s(), x(2.0), 0x41) {
            2.0
        } else {
            f32_of(s())
        }
    } else {
        -2.0
    }
}

/// `CHeli::ProcessControlInputs(uint8 pedType)` (0x6C4830): throttle (+0x998) `(accel - brake) / 255`;
/// pitch (+0x990) and yaw (+0x994) from the stick (`updown / 128`, `-leftright / 128`) or, with mouse
/// flying enabled (0xB6EC2E and 0xC1CC03), from the mouse (`-x * 0.0025` / `y * 0.0025` while the pad's
/// +0x2C word is 0, each decaying by `pow(0.98, timeStep)` below 0.5) whenever the mouse moved or the
/// stick is centred while already in mouse mode; both clamped to [-1, 1]. Roll (+0x98C) from the look
/// buttons (right 1, left -1). The horn button with the heli upright levels it: pitch and yaw become the
/// move speed along `up x right` / `forward x up` (normalised) times the flying handling's +0x24 / +0x1C,
/// clamped to [-2, 2], roll 0. Steering, gas and handbrake are cleared (brake 1). While the player's
/// controls are disabled the area around the player is cleared and the speed capped at 0.28. Below 250
/// health it sinks (throttle -0.1, roll +0.5).
#[hook(addr = 0x6C4830, conv = thiscall, name = "CHeli::ProcessControlInputs", this = "CHeli",
       expect = "83 EC 24 53 55 56 57",
       unverified = "veh4-3", route = "L2", hazards = "x87-transcendental",
       note = "pow(0.98, timeStep) through orig::x87_binary(0x822130), FSQRT of the speed; the pad pointer is re-fetched per use, as the original")]
pub fn heli_process_control_inputs(this: GPtr<u8>, ped_type: u32) {
    let id = i32::from(ped_type as u8);
    let pad = pad_get_pad(id);
    let acc = i32::from(pad_get_accelerate(pad));
    let brk = i32::from(pad_get_brake(pad));
    wr::<f32>(this, 0x998, f32_of(xi(acc - brk).fmul_m(f32::from_bits(0x3B80_8081))));
    let state = g::<i32>(0xC1_CC04);
    let stick = |this: GPtr<u8>| {
        state.write(0);
        let ud = pad_get_steering_up_down(pad);
        wr::<f32>(this, 0x990, f32_of(xi(i32::from(ud)).fmul_m(f32::from_bits(0x3C00_0000))));
        let lr = pad_get_steering_left_right(pad);
        wr::<f32>(this, 0x994, f32_of(xi(-i32::from(lr)).fmul_m(f32::from_bits(0x3C00_0000))));
    };
    if g::<u8>(0xB6_EC2E).read() != 0 && g::<u8>(0xC1_CC03).read() != 0 {
        pad_get_pad(id);
        let mut mouse = x(g::<f32>(0xB7_3424).read()).fcom(x(0.0)).test_parity(0x44);
        if !mouse {
            pad_get_pad(id);
            mouse = x(g::<f32>(0xB7_3428).read()).fcom(x(0.0)).test_parity(0x44);
        }
        if !mouse {
            let moving = !jne_mask(x(rd::<f32>(this, 0x994)).fabs(), x(0.0), 0x41)
                || !jne_mask(x(rd::<f32>(this, 0x990)).fabs(), x(0.0), 0x41);
            if moving
                && state.read() == 2
                && pad_get_steering_left_right(pad_get_pad(id)) == 0
                && pad_get_steering_up_down(pad_get_pad(id)) == 0
            {
                mouse = true;
            }
        }
        if mouse {
            state.write(2);
            if rd::<u16>(pad_get_pad(id), 0x2C) == 0 {
                pad_get_pad(id);
                let k = g::<f32>(0x87_1674).read();
                wr::<f32>(this, 0x994, f32_of(x(g::<f32>(0xB7_3424).read()).fmul_m(k).fsubr_m(rd::<f32>(this, 0x994))));
                pad_get_pad(id);
                wr::<f32>(this, 0x990, f32_of(x(g::<f32>(0xB7_3428).read()).fmul_m(k).fadd_m(rd::<f32>(this, 0x990))));
            }
            let lim = x(g::<f32>(0x87_1678).read());
            if !jp5(x(rd::<f32>(this, 0x994)).fabs(), lim) {
                wr::<f32>(this, 0x994, f32_of(decay().fmul_m(rd::<f32>(this, 0x994))));
            }
            if !jp5(x(rd::<f32>(this, 0x990)).fabs(), lim) {
                wr::<f32>(this, 0x990, f32_of(decay().fmul_m(rd::<f32>(this, 0x990))));
            }
        } else if pad_get_steering_left_right(pad) != 0 || pad_get_steering_up_down(pad) != 0 || state.read() != 2 {
            stick(this);
        }
    } else {
        stick(this);
    }
    wr::<f32>(this, 0x990, clamp1(rd::<f32>(this, 0x990)));
    wr::<f32>(this, 0x994, clamp1(rd::<f32>(this, 0x994)));
    let r = pad_get_look_right(pad);
    wr::<f32>(this, 0x98C, f32_of(xi(i32::from(r))));
    if pad_get_look_left(pad) != 0 {
        wr::<u32>(this, 0x98C, 0xBF80_0000);
    }
    if pad_get_horn(pad) != 0 && !jne_mask(x(rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x28)), x(0.0), 0x41) {
        let m = rd::<GPtr<u8>>(this, 0x14);
        wr::<u32>(this, 0x98C, 0);
        let mut up = [0u32, 0, 0x3F80_0000];
        let mut a = [0u32; 3];
        cross_product(buf(&mut a), buf(&mut up), m);
        vector_normalise(buf(&mut a));
        let pitch = stabiliser(this, &a, 0x24);
        wr::<f32>(this, 0x990, pitch);
        let mut up2 = [0u32, 0, 0x3F80_0000];
        let mut b = [0u32; 3];
        let rr = cross_product(buf(&mut b), rd::<GPtr<u8>>(this, 0x14).byte_add(0x10), buf(&mut up2));
        let mut a2 = [rd::<u32>(rr, 0), rd::<u32>(rr, 4), rd::<u32>(rr, 8)];
        vector_normalise(buf(&mut a2));
        let yaw = stabiliser(this, &a2, 0x1C);
        wr::<f32>(this, 0x994, yaw);
    }
    wr::<u32>(this, 0x494, 0);
    wr::<u32>(this, 0x4A0, 0x3F80_0000);
    wr::<u32>(this, 0x49C, 0);
    wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xDF);
    if rd::<u16>(pad, 0x10E) != 0 {
        keep_area_around_player_clear(find_player_ped(-1));
        let (mx, my, mz) = (x(rd::<f32>(this, 0x44)), x(rd::<f32>(this, 0x48)), x(rd::<f32>(this, 0x4C)));
        let speed = mx.fmul(mx).fadd(my.fmul(my)).fadd(mz.fmul(mz)).fsqrt();
        let cap = g::<f32>(0x87_1254).read();
        if !jne_mask(speed, x(cap), 0x41) {
            let f = speed.fdivr_m(cap);
            let nx = f32_of(f.fmul_m(rd::<f32>(this, 0x44)));
            let ny = f32_of(f.fmul_m(rd::<f32>(this, 0x48)));
            let nz = f32_of(f.fmul_m(rd::<f32>(this, 0x4C)));
            wr::<f32>(this, 0x44, nx);
            wr::<f32>(this, 0x48, ny);
            wr::<f32>(this, 0x4C, nz);
        }
    }
    if x(rd::<f32>(this, 0x4C0)).fcom(x(250.0)) == sa_math::x87::Cmp::Less {
        wr::<u32>(this, 0x998, 0xBDCC_CCCD);
        wr::<f32>(this, 0x98C, f32_of(x(rd::<f32>(this, 0x98C)).fadd_m(0.5)));
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_HELI_PROCESS_CONTROL_INPUTS];
