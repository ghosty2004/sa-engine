//! PORT-VEH-3 (batch `veh3-3`): the monster truck and trailer `ProcessControl` wrappers and the
//! bike horn. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};

orig! {
    /// `CAutomobile::ProcessControl()` (0x6B1880)
    fn automobile_process_control(a: GPtr<u8>) = thiscall 0x6B1880 => unported;
    /// `CMonsterTruck::ExtendSuspension()` (0x6C7D80)
    fn monster_truck_extend_suspension(a: GPtr<u8>) = thiscall 0x6C7D80 => crate::vehicle_suspension_nodes::monster_truck_extend_suspension_port;
    /// `min(float, float)` (0x404330), ST0
    fn min_f(a: f32, b: f32) -> f64 = cdecl 0x404330 => crate::train_control::min_native;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0)
    fn ped_say(p: GPtr<u8>, id: u32, delay: u32, prob: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
}

/// `CMonsterTruck::ProcessControl()` (0x6C8250): per wheel a ratio +0x7D4 below 1.0 becomes
/// `(+0x878 - +0x838) / (+0x878 - +0x888)`, clamped to 0 when negative and the flag byte 0x8D33B0 is
/// set; others 1.0. Then `CAutomobile::ProcessControl`, and unless the entity is static (+0x1C bit 6)
/// a moving truck (any move/turn speed component non-zero) extends its suspension (a tail call).
#[hook(addr = 0x6C8250, conv = thiscall, name = "CMonsterTruck::ProcessControl", this = "CMonsterTruck",
       expect = "53 8A 1D B0 33 8D 00",
       unverified = "veh3-3", route = "L2", hazards = "heap-writes",
       note = "CAutomobile::ProcessControl runs the whole car simulation")]
pub fn monster_truck_process_control(this: GPtr<u8>) {
    let clamp = g::<u8>(0x8D_33B0).read() != 0;
    for k in 0..4isize {
        let r = 0x7D4 + 4 * k;
        if x(rd::<f32>(this, r)).fcom(x(1.0)).test_parity(0x05) {
            wr::<u32>(this, r, 0x3F80_0000);
            continue;
        }
        let a = x(rd::<f32>(this, r + 0xA4)).fsub_m(rd::<f32>(this, r + 0x64));
        let d = x(rd::<f32>(this, r + 0xA4)).fsub_m(rd::<f32>(this, r + 0xB4));
        let v = a.fdiv(d);
        wr::<f32>(this, r, v.to_f32());
        if !v.fcom(x(0.0)).test_parity(0x05) && clamp {
            wr::<u32>(this, r, 0);
        }
    }
    automobile_process_control(this);
    if (rd::<u32>(this, 0x1C) >> 6) & 1 != 0 {
        return;
    }
    let moving = (0..6isize).any(|k| x(rd::<f32>(this, 0x44 + 4 * k)).fcom(x(0.0)).test_parity(0x44));
    if moving {
        monster_truck_extend_suspension(this);
    }
}

/// `CTrailer::ProcessControl()` (0x6CED20): `CAutomobile::ProcessControl`; unless static or a
/// baggage-trailer marker (+0x9EC == -1000): the support leg +0x9EC retracts (towed: by
/// `timestep * 0.002` down to 0) or extends (free: by `timestep * 0.002` up to 1.0 above 0.1, by
/// `timestep * 0.0005` below, +0xB8 cleared); +0x9F0 = +0x9EC (towing states 10/11: at least 0.9),
/// and both support ratios +0x9E0 / +0x9E4 = +0x9F0.
#[hook(addr = 0x6CED20, conv = thiscall, name = "CTrailer::ProcessControl", this = "CTrailer",
       expect = "56 8B F1 E8 58 2B FE FF",
       unverified = "veh3-3", route = "L2", hazards = "heap-writes",
       note = "CAutomobile::ProcessControl runs the whole car simulation")]
pub fn trailer_process_control(this: GPtr<u8>) {
    automobile_process_control(this);
    if (rd::<u32>(this, 0x1C) >> 6) & 1 != 0 || rd::<u32>(this, 0x9EC) == 0xC47A_0000 {
        return;
    }
    let leg = rd::<f32>(this, 0x9EC);
    let ts = g::<f32>(0xB7_CB5C).read();
    if !rd::<GPtr<u8>>(this, 0x4C4).is_null() {
        if !x(leg).fcom(x(0.0)).test_any(0x41) {
            let mut v = x(ts).fmul_m(f32::from_bits(0x3B03_126F)).fsubr_m(leg);
            if !x(0.0).fcom(v).test_any(0x41) {
                v = x(0.0);
            }
            wr::<f32>(this, 0x9EC, v.to_f32());
        }
    } else if !x(leg).fcom(x(1.0)).test_parity(0x05) {
        let v = if !x(leg).fcom(x(f32::from_bits(0x3DCC_CCCD))).test_any(0x41) {
            let s = x(ts).fmul_m(f32::from_bits(0x3B03_126F)).fadd_m(leg).to_f32();
            X87::from_f64_exact(min_f(1.0, s))
        } else {
            x(ts).fmul_m(f32::from_bits(0x3A03_126F)).fadd_m(leg)
        };
        wr::<u8>(this, 0xB8, 0);
        wr::<f32>(this, 0x9EC, v.to_f32());
    }
    let st = rd::<u8>(this, 0x36) >> 3;
    if st != 10 && st != 11 {
        wr::<u32>(this, 0x9F0, rd::<u32>(this, 0x9EC));
    } else {
        let c = f32::from_bits(0x3F66_6666);
        let v = if x(c).fcom(x(rd::<f32>(this, 0x9EC))).test_any(0x41) { rd::<f32>(this, 0x9EC) } else { c };
        wr::<f32>(this, 0x9F0, x(v).to_f32());
    }
    let s = rd::<u32>(this, 0x9F0);
    wr::<u32>(this, 0x9E0, s);
    wr::<u32>(this, 0x9E4, s);
}

/// `CBike::PlayCarHorn()` (0x6B7080): not for a bike with an active alarm (+0x45C neither 0 nor
/// 0xFFFF) unless its state (+0x36 bits 3..7) is 5, nor while the horn counter (+0x514) runs; a
/// delay byte (+0x519) counts down; else a new delay `(rand & 0x7F) - 0x6A`, and by its low 3 bits:
/// 0/1 the horn (counter 45), 2/3 the horn with the driver's horn speech (id 22) when the bike's
/// +0x3DB bit 0 is set, 4..7 only the speech.
#[hook(addr = 0x6B7080, conv = thiscall, name = "CBike::PlayCarHorn", this = "CBike",
       expect = "56 8B F1 66 8B 86 5C 04 00 00",
       unverified = "veh3-3", route = "L2", hazards = "none",
       recs = "driving", note = "rand site 0x6B70C5")]
pub fn bike_play_car_horn(this: GPtr<u8>) {
    let a = rd::<u16>(this, 0x45C);
    if a != 0 && a != 0xFFFF && rd::<u8>(this, 0x36) & 0xF8 != 0x28 {
        return;
    }
    if rd::<u32>(this, 0x514) != 0 {
        return;
    }
    let d = rd::<u8>(this, 0x519);
    if d > 0 {
        wr::<u8>(this, 0x519, d - 1);
        return;
    }
    let r = sa_core::orig::at_site(0x6B_70C5, sa_core::crt::rand);
    let nd = ((r as u8) & 0x7F).wrapping_sub(0x6A);
    wr::<u8>(this, 0x519, nd);
    let v = nd & 7;
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if v < 2 {
        wr::<u32>(this, 0x514, 0x2D);
        return;
    }
    if v < 4 {
        if !driver.is_null() && rd::<u8>(this, 0x3DB) & 1 != 0 {
            ped_say(driver, 0x16, 0, 0x3F80_0000, 0, 0, 0);
        }
        wr::<u32>(this, 0x514, 0x2D);
        return;
    }
    if !driver.is_null() {
        ped_say(driver, 0x16, 0, 0x3F80_0000, 0, 0, 0);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_MONSTER_TRUCK_PROCESS_CONTROL, HOOK_TRAILER_PROCESS_CONTROL, HOOK_BIKE_PLAY_CAR_HORN];
