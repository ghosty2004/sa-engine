//! PORT-VEH-3 (batch `world3-1`, SA:MP-executed rows): the per-frame `CStats::UpdateStatsWhen*`
//! progress counters (cycling, swimming, driving, flying, motorbike). Each one compares an unsigned
//! millisecond counter with a stat threshold x 1000, raises the stat (and shows the stat message) when
//! it is passed, else adds a speed-dependent share of the frame time. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{ftol32, g, rd, x, xi};

orig! {
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    fn stats_increment_stat(stat: u32, v: u32) = cdecl 0x55C180 => unported;
    /// `CStats::DisplayScriptStatUpdateMessage(uint8, uint32, float)` (0x55B980)
    fn display_script_stat_update_message(state: u32, stat: u32, v: u32) = cdecl 0x55B980 => unported;
    /// `CStats::UpdateFatAndMuscleStats(uint32)` (0x55C470)
    fn update_fat_and_muscle_stats(v: i32) = cdecl 0x55C470 => unported;
    /// CRT `ceil(double)` (0x823820), ST0
    fn crt_ceil(v: f64) -> f64 = cdecl 0x823820 => unported;
}

/// `fild` of an unsigned dword (`+ 2^32` when the sign bit is set, 0x858C54).
fn u2f(v: u32) -> X87 {
    let f = xi(v as i32);
    if (v as i32) < 0 {
        f.fadd_m(4_294_967_296.0)
    } else {
        f
    }
}

/// `ftol(CTimer::ms_fTimeStep * 0.02 * 1000)`: the frame time in ms.
fn frame_ms() -> i32 {
    ftol32(x(g::<f32>(0xB7_CB5C).read()).fmul_m(f32::from_bits(0x3CA3_D70A)).fmul_m(1000.0))
}

/// `threshold * 1000 < counter` (`fcompp`, `test ah, 5`, `jp` not taken): the stat is due.
fn due(counter: X87, threshold: usize) -> bool {
    !x(g::<f32>(threshold).read()).fmul_m(1000.0).fcom(counter).test_parity(0x05)
}

/// Raise `stat` by the float at `amount` and show the message (the counter is cleared first).
fn raise(counter: usize, stat: u32, amount: usize) {
    g::<u32>(counter).write(0);
    stats_increment_stat(stat, g::<u32>(amount).read());
    display_script_stat_update_message(1, stat, g::<u32>(amount).read());
}

/// `|move speed|^2` as the original sums it: `(x^2 + y^2) + z^2`.
fn speed_sq(v: GPtr<u8>) -> X87 {
    let sx = x(rd::<f32>(v, 0x44));
    let sy = x(rd::<f32>(v, 0x48));
    let sz = x(rd::<f32>(v, 0x4C));
    sx.fmul(sx).fadd(sy.fmul(sy)).fadd(sz.fmul(sz))
}

/// `counter = ftol(u2f(frame_ms) * factor + counter_f32)`.
fn add_scaled(counter: usize, counter_f: f32, factor: f32) {
    let t = frame_ms() as u32;
    g::<i32>(counter).write(ftol32(u2f(t).fmul_m(factor).fadd_m(counter_f)));
}

/// `CStats::UpdateStatsWhenCycling(bool pedalling, CBmx*)` (0x55C780): fat/muscle from stat
/// 0xB78F7C (pedalling, or moving with the pedal +0x49C non-zero) or 0xB78F78 (moving without);
/// counter 0xB794DC -> stat 22 (fat? 0xB78F94, amount 0xB78F10); counter 0xB794E0 -> stat 230
/// (0xB78FAC, amount 0xB78F28), else `+= ftol(ceil(frame_ms * k))` with k = 3.0 for a wheelie-ish
/// state (+0x804 < 2 and speed > 0.05), 1.5/1.0 (pedalling or not) above speed 0.2, nothing below.
#[hook(addr = 0x55C780, conv = cdecl, name = "CStats::UpdateStatsWhenCycling",
       expect = "53 8A 5C 24 08",
       unverified = "world3-1", route = "L2", hazards = "none", recs = "driving")]
pub fn update_stats_when_cycling(pedalling: u8, bmx: GPtr<u8>) {
    // `test ah, 0x41; je` taken: |pedal| > 0
    let still = x(rd::<f32>(bmx, 0x49C)).fabs().fcom(x(0.0)).test_any(0x41);
    let src = if !still {
        Some(if pedalling != 0 { 0xB7_8F7C } else { 0xB7_8F78 })
    } else if pedalling != 0 {
        Some(0xB7_8F7C)
    } else {
        None
    };
    if let Some(s) = src {
        update_fat_and_muscle_stats(ftol32(x(g::<f32>(s).read())));
        let c = g::<u32>(0xB7_94DC).read();
        if due(u2f(c), 0xB7_8F94) {
            raise(0xB7_94DC, 0x16, 0xB7_8F10);
        } else {
            g::<i32>(0xB7_94DC).write((c as i32).wrapping_add(frame_ms()));
        }
    }
    if due(u2f(g::<u32>(0xB7_94E0).read()), 0xB7_8FAC) {
        raise(0xB7_94E0, 0xE6, 0xB7_8F28);
        return;
    }
    let k = |c: f32| x(c).fmul_m(c);
    let s2 = speed_sq(bmx);
    let mut factor = None;
    if rd::<u8>(bmx, 0x804) < 2 && !k(f32::from_bits(0x3D4C_CCCD)).fcom(s2).test_parity(0x05) {
        factor = Some(3.0f32);
    }
    if factor.is_none() {
        if k(f32::from_bits(0x3E4C_CCCD)).fcom(s2).test_parity(0x05) {
            return;
        }
        factor = Some(if pedalling != 0 { 1.5 } else { 1.0 });
    }
    let f = factor.unwrap_or(0.0);
    if x(f).fcom(x(0.0)).test_any(0x41) {
        return;
    }
    let t = frame_ms() as u32;
    let v = u2f(t).fmul_m(f);
    let r = ftol32(X87::from_f64_exact(crt_ceil(v.to_f64())));
    g::<i32>(0xB7_94E0).write(g::<i32>(0xB7_94E0).read().wrapping_add(r));
}

/// `CStats::UpdateStatsWhenSwimming(bool, bool)` (0x55C990): fat/muscle from 0xB78F84 (either flag)
/// or 0xB78F80; counter 0xB794E4 -> stat 22 (0xB78F98, amount 0xB78F14) else `+= frame_ms`; with the
/// first flag counter 0xB794E8 -> stat 225 (0xB78FC0, amount 0xB78F38) else `+= frame_ms`.
#[hook(addr = 0x55C990, conv = cdecl, name = "CStats::UpdateStatsWhenSwimming",
       expect = "51 8A 44 24 08",
       unverified = "world3-1", route = "L2", hazards = "none")]
pub fn update_stats_when_swimming(underwater: u8, fast: u8) {
    let s = if underwater != 0 || fast != 0 { 0xB7_8F84 } else { 0xB7_8F80 };
    update_fat_and_muscle_stats(ftol32(x(g::<f32>(s).read())));
    let c = g::<u32>(0xB7_94E4).read();
    if due(u2f(c), 0xB7_8F98) {
        raise(0xB7_94E4, 0x16, 0xB7_8F14);
    } else {
        g::<i32>(0xB7_94E4).write((c as i32).wrapping_add(frame_ms()));
    }
    if underwater == 0 {
        return;
    }
    let c = g::<u32>(0xB7_94E8).read();
    if due(u2f(c), 0xB7_8FC0) {
        raise(0xB7_94E8, 0xE1, 0xB7_8F38);
        return;
    }
    g::<i32>(0xB7_94E8).write((c as i32).wrapping_add(frame_ms()));
}

/// The motor-vehicle counters: `counter` (as an f32, stored) -> `stat` when due; else by the 3D
/// speed `s`: `fast` (true: factor `f_fast`), else `s > slow` gives `f_slow`, else nothing.
fn vehicle_counter(counter: usize, threshold: usize, stat: u32, amount: usize, decide: impl FnOnce(X87) -> Option<f32>, v: GPtr<u8>) {
    let cf = u2f(g::<u32>(counter).read()).to_f32();
    if due(x(cf), threshold) {
        raise(counter, stat, amount);
        return;
    }
    let s = speed_sq(v).fsqrt();
    if let Some(f) = decide(s) {
        add_scaled(counter, cf, f);
    }
}

/// `s > c` (`fcom`, `test ah, 0x41`, `je` taken).
fn above(s: X87, c: f32) -> bool {
    !s.fcom(x(c)).test_any(0x41)
}

/// `CStats::UpdateStatsWhenDriving(CVehicle*)` (0x55CAC0): counter 0xB794EC -> stat 160 (0xB78FA4,
/// amount 0xB78F20); else speed > 0.8 or (an automobile with +0x960 == 0): +frame x 1.5; speed > 0.2:
/// +frame x 0.5.
#[hook(addr = 0x55CAC0, conv = cdecl, name = "CStats::UpdateStatsWhenDriving",
       expect = "51 8B 54 24 08",
       unverified = "world3-1", route = "L2", hazards = "x87-transcendental", recs = "driving",
       note = "the counter float is kept in the argument slot")]
pub fn update_stats_when_driving(v: GPtr<u8>) {
    let car = rd::<u32>(v, 0x590) == 0;
    vehicle_counter(
        0xB7_94EC,
        0xB7_8FA4,
        0xA0,
        0xB7_8F20,
        |s| {
            if above(s, f32::from_bits(0x3F4C_CCCD)) || (car && rd::<u8>(v, 0x960) == 0) {
                Some(1.5)
            } else if above(s, f32::from_bits(0x3E4C_CCCD)) {
                Some(0.5)
            } else {
                None
            }
        },
        v,
    );
}

/// `CStats::UpdateStatsWhenFlying(CVehicle*)` (0x55CC00): automobiles only (+0x590 == 0); counter
/// 0xB794F0 -> stat 223 (0xB78FA8, amount 0xB78F24); else only with +0x960 == 0: speed > 1.3 or
/// upside down (up.z < 0): +frame x 1.5; speed > 0.5: +frame x 0.5.
#[hook(addr = 0x55CC00, conv = cdecl, name = "CStats::UpdateStatsWhenFlying",
       expect = "51 8B 4C 24 08",
       unverified = "world3-1", route = "L2", hazards = "x87-transcendental",
       note = "the counter float is kept in the argument slot")]
pub fn update_stats_when_flying(v: GPtr<u8>) {
    if rd::<u32>(v, 0x590) != 0 {
        return;
    }
    let cf = u2f(g::<u32>(0xB7_94F0).read()).to_f32();
    if due(x(cf), 0xB7_8FA8) {
        raise(0xB7_94F0, 0xDF, 0xB7_8F24);
        return;
    }
    if rd::<u8>(v, 0x960) != 0 {
        return;
    }
    let s = speed_sq(v).fsqrt();
    let upz = x(rd::<f32>(rd::<GPtr<u8>>(v, 0x14), 0x28));
    let f = if above(s, f32::from_bits(0x3FA6_6666)) || !upz.fcom(x(0.0)).test_parity(0x05) {
        1.5
    } else if above(s, 0.5) {
        0.5
    } else {
        return;
    };
    add_scaled(0xB7_94F0, cf, f);
}

/// `CStats::UpdateStatsWhenOnMotorBike(CBike*)` (0x55CD60): counter 0xB794F8 -> stat 229 (0xB78FB0,
/// amount 0xB78F2C); else speed > 0.6, or fewer than 3 wheels on the ground (+0x804) and speed > 0.1:
/// +frame x 1.5; speed > 0.2: +frame x 0.5.
#[hook(addr = 0x55CD60, conv = cdecl, name = "CStats::UpdateStatsWhenOnMotorBike",
       expect = "A1 F8 94 B7 00",
       unverified = "world3-1", route = "L2", hazards = "x87-transcendental", recs = "driving")]
pub fn update_stats_when_on_motor_bike(v: GPtr<u8>) {
    vehicle_counter(
        0xB7_94F8,
        0xB7_8FB0,
        0xE5,
        0xB7_8F2C,
        |s| {
            if above(s, f32::from_bits(0x3F19_999A))
                || (rd::<u8>(v, 0x804) < 3 && above(s, f32::from_bits(0x3DCC_CCCD)))
            {
                Some(1.5)
            } else if above(s, f32::from_bits(0x3E4C_CCCD)) {
                Some(0.5)
            } else {
                None
            }
        },
        v,
    );
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_UPDATE_STATS_WHEN_CYCLING,
    HOOK_UPDATE_STATS_WHEN_SWIMMING,
    HOOK_UPDATE_STATS_WHEN_DRIVING,
    HOOK_UPDATE_STATS_WHEN_FLYING,
    HOOK_UPDATE_STATS_WHEN_ON_MOTOR_BIKE,
];
