//! PORT-VEH-4 (batch `veh4-3`): `CAutomobile::ProcessControlInputs`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use crate::common::{f32_of, g, jne_mask, jp5, rd, wr, x, xi};

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    fn find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// virtual `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPad::GetExitVehicle()` (0x5400D0, AL)
    fn pad_get_exit_vehicle(pad: GPtr<u8>) -> u8 = thiscall 0x5400D0 => unported;
    /// `CPad::GetHandBrake()` (0x540040): int16 in AX
    fn pad_get_hand_brake(pad: GPtr<u8>) -> i16 = thiscall 0x540040 => unported;
    /// `CPad::GetSteeringLeftRight()` (0x53FB80): int16 in AX
    fn pad_get_steering_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FB80 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0): int16 in AX
    fn pad_get_accelerate(pad: GPtr<u8>) -> i16 = thiscall 0x5403F0 => unported;
    /// `CPad::GetBrake()` (0x540080): int16 in AX
    fn pad_get_brake(pad: GPtr<u8>) -> i16 = thiscall 0x540080 => unported;
    /// `CGameLogic::IsPlayerAllowedToGoInThisDirection(CPed*, CVector, float)` (0x441E10, AL)
    fn is_player_allowed_direction(p: GPtr<u8>, dx: u32, dy: u32, dz: u32, limit: u32) -> u8 = cdecl 0x441E10 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(n: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlayerPed::KeepAreaAroundPlayerClear()` (0x60C1E0)
    fn keep_area_around_player_clear(p: GPtr<u8>) = thiscall 0x60C1E0 => unported;
}

/// `CTimer::ms_fTimeStep`.
fn ts() -> f32 {
    g::<f32>(0xB7_CB5C).read()
}

/// `CAutomobile::ProcessControlInputs(uint8 pedType)` (0x6AD690).
///
/// The handbrake (+0x428 bit 5) is forced on, and the car treated as being left, while the driver
/// has none of task 0x32E and is in state 63, has task 0x337/0x339/0x329 or presses exit; else it
/// follows the handbrake button. Steering (+0x58C) eases toward `-leftright / 128` (by `timeStep *
/// 0.2`), or in mouse steering (0xB6EC2E and 0xC1CC02; the mouse moved, or still steering in mouse mode
/// with the stick centred) takes `mouseX * -0.0035` (while the pad's +0x2C word is 0) and decays by
/// `pow(0.975, timeStep)` below 0.7 or while that word is set; clamped to [-1, 1]. The pedal value is
/// `(accel - brake) / 255` (in the aim-from-car task 0x3FE with camera mode 0x37 the pad's +0xE/+0xA
/// words instead), and 0 with the +0x42B bit 6 set unless +0x428 bit 4. Leaving: full brake, no gas.
/// Nearly stopped (`|forward speed| < 0.01`) with both pedals above 150: both applied (burnout, +0x86A =
/// 1); else gas. Moving forward: positive pedal is gas, negative is brake. Moving backward: negative is
/// gas; positive brakes unless the gas is already above 0.5 and the backward speed under 0.15. Gas
/// toward a direction the player may not go (forward axis, or its negation for reverse) is cut. Steer
/// angle (+0x494) `maxSteer * deg2rad * steer * |steer|` (the squared value at 0xC1C218), with the
/// handling flag 0x40 at +0xD0 the +0x498 value eased (handbrake) or damped by 0.9. Flag +0x429 bit 4
/// adds a random steering wobble state (+0x51A). Disabled controls (or the 0x96A8A0 / pad +0x118 cases):
/// handbrake, full brake, no gas, the area cleared, speed capped at 0.28. Flag +0x42D bit 1: brake
/// 0.05, no gas, handbrake off.
#[hook(addr = 0x6AD690, conv = thiscall, name = "CAutomobile::ProcessControlInputs", this = "CAutomobile",
       expect = "83 EC 18 53 8A 5C 24 20",
       unverified = "veh4-3", route = "L2", hazards = "x87-transcendental",
       recs = "driving",
       note = "pow(0.975, timeStep) through orig::x87_binary(0x822130), FSQRT of the speed; rand at_site 0x6ADD79 (wobble); the pad pointer is re-fetched per use, as the original")]
pub fn automobile_process_control_inputs(this: GPtr<u8>, ped_type: u32) {
    let id = i32::from(ped_type as u8);
    let m = rd::<GPtr<u8>>(this, 0x14);
    let fwd = f32_of(
        x(rd::<f32>(this, 0x4C))
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(x(rd::<f32>(this, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
            .fadd(x(rd::<f32>(this, 0x44)).fmul_m(rd::<f32>(m, 0x10))),
    );
    let tm = |d: GPtr<u8>| rd::<GPtr<u8>>(d, 0x47C).byte_add(4);
    let mut leaving = false;
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if !driver.is_null() && find_active_task_by_type(tm(driver), 0x32E).is_null() {
        let d = || rd::<GPtr<u8>>(this, 0x460);
        leaving = rd::<u32>(d(), 0x530) == 0x3F
            || !find_active_task_by_type(tm(d()), 0x337).is_null()
            || !find_active_task_by_type(tm(d()), 0x339).is_null()
            || !find_active_task_by_type(tm(d()), 0x329).is_null()
            || pad_get_exit_vehicle(pad_get_pad(id)) != 0;
    }
    if leaving {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
    } else {
        let hb = pad_get_hand_brake(pad_get_pad(id)) != 0;
        let f = rd::<u8>(this, 0x428);
        wr::<u8>(this, 0x428, (f & !0x20) | if hb { 0x20 } else { 0 });
    }
    steering(this, id);
    let s = rd::<f32>(this, 0x58C);
    let clamped = if !jp5(x(1.0), x(s)) {
        1.0
    } else if !jne_mask(x(-1.0), x(s), 0x41) {
        -1.0
    } else {
        s
    };
    wr::<f32>(this, 0x58C, clamped);
    let acc = i32::from(pad_get_accelerate(pad_get_pad(id)));
    let brk = i32::from(pad_get_brake(pad_get_pad(id)));
    let mut pedal = f32_of(xi(acc - brk).fmul_m(f32::from_bits(0x3B80_8081)));
    if x(pedal).fcom(x(0.0)) == Cmp::Equal {
        let d = rd::<GPtr<u8>>(this, 0x460);
        if !d.is_null() && !get_simplest_active_task(tm(d)).is_null() {
            let t = get_simplest_active_task(tm(rd::<GPtr<u8>>(this, 0x460)));
            if v_task_get_type(t) == 0x3FE && g::<u16>(0xB6_F1A8).read() == 0x37 {
                let a = rd::<i16>(pad_get_pad(id), 0xE);
                let b = rd::<i16>(pad_get_pad(id), 0xA);
                pedal = f32_of(xi(i32::from(a) - i32::from(b)).fmul_m(f32::from_bits(0x3B80_8081)));
            }
        }
    }
    if rd::<i16>(this, 0x22) == -2 && x(pedal).fcom(x(0.0)) == Cmp::Less {
        pedal = f32_of(x(pedal).fmul_m(f32::from_bits(0x3E99_999A)));
    }
    if rd::<u8>(this, 0x428) & 0x10 == 0 && rd::<u8>(this, 0x42B) & 0x40 != 0 {
        pedal = 0.0;
    }
    pedals(this, id, fwd, pedal, leaving);
    let gas = || rd::<f32>(this, 0x49C);
    if !jne_mask(x(gas()), x(0.0), 0x41) && !rd::<GPtr<u8>>(this, 0x460).is_null() {
        let f = rd::<GPtr<u8>>(this, 0x14).byte_add(0x10);
        let d = rd::<GPtr<u8>>(this, 0x460);
        if is_player_allowed_direction(d, rd::<u32>(f, 0), rd::<u32>(f, 4), rd::<u32>(f, 8), 0) == 0 {
            wr::<u32>(this, 0x49C, 0);
        }
    }
    if x(gas()).fcom(x(0.0)) == Cmp::Less && !rd::<GPtr<u8>>(this, 0x460).is_null() {
        let d = rd::<GPtr<u8>>(this, 0x460);
        let f = rd::<GPtr<u8>>(this, 0x14).byte_add(0x10);
        let n = |o: isize| f32_of(x(rd::<f32>(f, o)).fmul_m(-1.0)).to_bits();
        let (nx, ny, nz) = (n(0), n(4), n(8));
        if is_player_allowed_direction(d, nx, ny, nz, 0) == 0 {
            wr::<u32>(this, 0x49C, 0);
        }
    }
    let s = x(rd::<f32>(this, 0x58C));
    let mut sq = s.fmul(s);
    if s.fcom(x(0.0)).c0() {
        sq = sq.fchs();
    }
    g::<f32>(0xC1_C218).write(f32_of(sq));
    let h = rd::<GPtr<u8>>(this, 0x384);
    let angle = f32_of(
        x(rd::<f32>(h, 0xA0))
            .fmul_m(f32::from_bits(0x3C8E_FA35))
            .fmul_m(g::<f32>(0xC1_C218).read()),
    );
    wr::<f32>(this, 0x494, angle);
    if rd::<u8>(h, 0xD0) & 0x40 != 0 {
        let v = if rd::<u8>(this, 0x428) & 0x20 != 0 {
            let c = g::<f32>(0x8D_3208).read();
            x(c).fmul_m(rd::<f32>(this, 0x498)).fsub(x(1.0).fsub_m(c).fmul_m(angle))
        } else {
            x(g::<f32>(0x8D_3204).read()).fmul_m(rd::<f32>(this, 0x498))
        };
        wr::<f32>(this, 0x498, f32_of(v));
    }
    if rd::<u8>(this, 0x429) & 0x10 != 0 {
        let r = (sa_core::orig::at_site(0x6A_DD79, sa_core::crt::rand) % 10) as u16;
        let step = f32::from_bits(0x3D4C_CCCD);
        match rd::<u8>(this, 0x51A) {
            0 => {
                if r < 2 {
                    wr::<u8>(this, 0x51A, 1);
                } else if r < 4 {
                    wr::<u8>(this, 0x51A, 2);
                }
            }
            1 | 2 => {
                let a = x(rd::<f32>(this, 0x494));
                let a = if rd::<u8>(this, 0x51A) == 1 { a.fadd_m(step) } else { a.fsub_m(step) };
                wr::<f32>(this, 0x494, f32_of(a));
                if r < 2 {
                    wr::<u8>(this, 0x51A, 0);
                }
            }
            _ => {}
        }
    }
    let disabled = if rd::<u16>(pad_get_pad(0), 0x10E) != 0 && g::<i32>(0x96_A8A0).read() != 2 {
        true
    } else {
        rd::<u8>(pad_get_pad(0), 0x118) != 0
    };
    if disabled {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
        wr::<u32>(this, 0x4A0, 0x3F80_0000);
        wr::<u32>(this, 0x49C, 0);
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
    if rd::<u8>(this, 0x42D) & 2 != 0 {
        wr::<u32>(this, 0x4A0, 0x3D4C_CCCD);
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xDF);
        wr::<u32>(this, 0x49C, 0);
    }
}

/// The steering input (0x6AD79F..0x6AD941).
fn steering(this: GPtr<u8>, id: i32) {
    let state = g::<i32>(0xC1_CC04);
    let steer = || rd::<f32>(this, 0x58C);
    let target = |lr: i16| xi(-i32::from(lr)).fmul_m(f32::from_bits(0x3C00_0000));
    if g::<u8>(0xB6_EC2E).read() == 0 || g::<u8>(0xC1_CC02).read() == 0 {
        let t = ts();
        let lr = pad_get_steering_left_right(pad_get_pad(id));
        let v = target(lr).fsub_m(steer()).fmul_m(f32::from_bits(0x3E4C_CCCD)).fmul_m(t).fadd_m(steer());
        wr::<f32>(this, 0x58C, f32_of(v));
        state.write(0);
        return;
    }
    pad_get_pad(id);
    let mut mouse = x(g::<f32>(0xB7_3424).read()).fcom(x(0.0)).test_parity(0x44);
    if !mouse
        && !jne_mask(x(steer()).fabs(), x(0.0), 0x41)
        && state.read() == 2
        && pad_get_steering_left_right(pad_get_pad(id)) == 0
    {
        mouse = true;
    }
    if mouse {
        if rd::<u16>(pad_get_pad(id), 0x2C) == 0 {
            pad_get_pad(id);
            let v = x(g::<f32>(0xB7_3424).read()).fmul_m(f32::from_bits(0xBB65_6042)).fadd_m(steer());
            wr::<f32>(this, 0x58C, f32_of(v));
        }
        state.write(2);
        if x(steer()).fabs().fcom(x(f32::from_bits(0x3F33_3333))) == Cmp::Less
            || rd::<u16>(pad_get_pad(id), 0x2C) != 0
        {
            let p = X87::from_f64_exact(sa_core::orig::x87_binary(
                0x822130,
                x(f32::from_bits(0x3F79_999A)).to_f64(),
                x(ts()).to_f64(),
            ));
            wr::<f32>(this, 0x58C, f32_of(p.fmul_m(steer())));
        }
        return;
    }
    if pad_get_steering_left_right(pad_get_pad(id)) == 0 && state.read() == 2 {
        return;
    }
    let t = ts();
    let lr = pad_get_steering_left_right(pad_get_pad(id));
    let v = target(lr).fsub_m(steer()).fmul_m(t).fmul_m(f32::from_bits(0x3E4C_CCCD)).fadd_m(steer());
    wr::<f32>(this, 0x58C, f32_of(v));
    state.write(0);
}

/// The gas (+0x49C) and brake (+0x4A0) pedals (0x6ADA87..0x6ADC18).
fn pedals(this: GPtr<u8>, id: i32, fwd: f32, pedal: f32, leaving: bool) {
    if leaving {
        wr::<u32>(this, 0x4A0, 0x3F80_0000);
        wr::<u32>(this, 0x49C, 0);
        return;
    }
    let mut a = x(fwd);
    if a.fcom(x(0.0)) == Cmp::Less {
        a = a.fchs();
    }
    if a.fcom(x(f32::from_bits(0x3C23_D70A))) == Cmp::Less {
        let over = |v: i16| !jne_mask(xi(i32::from(v)), x(150.0), 0x41);
        if over(pad_get_accelerate(pad_get_pad(id))) && over(pad_get_brake(pad_get_pad(id))) {
            let g1 = i32::from(pad_get_accelerate(pad_get_pad(id)));
            wr::<f32>(this, 0x49C, f32_of(xi(g1).fmul_m(f32::from_bits(0x3B80_8081))));
            let b1 = i32::from(pad_get_brake(pad_get_pad(id)));
            wr::<u16>(this, 0x86A, 1);
            wr::<f32>(this, 0x4A0, f32_of(xi(b1).fmul_m(f32::from_bits(0x3B80_8081))));
            return;
        }
        wr::<f32>(this, 0x49C, pedal);
        wr::<u32>(this, 0x4A0, 0);
        return;
    }
    let fwd_neg = x(fwd).fcom(x(0.0)).c0();
    let ped_neg = x(pedal).fcom(x(0.0)).c0();
    if !fwd_neg {
        if !ped_neg {
            wr::<f32>(this, 0x49C, pedal);
            wr::<u32>(this, 0x4A0, 0);
        } else {
            wr::<u32>(this, 0x49C, 0);
            wr::<f32>(this, 0x4A0, f32_of(x(pedal).fchs()));
        }
        return;
    }
    if ped_neg {
        wr::<f32>(this, 0x49C, pedal);
        wr::<u32>(this, 0x4A0, 0);
        return;
    }
    if !jne_mask(x(rd::<f32>(this, 0x49C)), x(0.5), 0x41)
        && !jne_mask(x(fwd), x(f32::from_bits(0xBE19_999A)), 0x41)
    {
        wr::<f32>(this, 0x49C, pedal);
        wr::<u32>(this, 0x4A0, 0);
    } else {
        wr::<u32>(this, 0x49C, 0);
        wr::<f32>(this, 0x4A0, pedal);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_PROCESS_CONTROL_INPUTS];
