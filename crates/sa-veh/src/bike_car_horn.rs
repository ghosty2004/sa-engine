//! `CBike::PlayCarHorn()` (0x6B7080, batch `std13-veh-1`): the AI horn of a bike (unreversed in gta-reversed).
//! Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::{rd, wr};

orig! {
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0), the float as its bits
    fn ped_say(p: GPtr<u8>, ctx: u32, offset: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// CRT `rand()` (0x821B1E)
    fn rand() -> i32 = cdecl 0x821B1E => sa_core::crt::rand_owned;
}

/// `CBike::PlayCarHorn()` (0x6B7080, thiscall). Nothing for an alarm state other than 0 / -1 (+0x45C, i16) unless
/// the status (+0x36 >> 3) is 5, nor while the horn counter (+0x514) runs. A horn timer (+0x519) above 0 is counted
/// down. Otherwise timer = (rand() & 0x7F) - 0x6A (byte), k = timer & 7: k < 2 → counter 45; k < 4 → the driver
/// (+0x460) says context 0x16 (volume 1.0) when its flag +0x3DB bit 0 is set, counter 45; else the driver (if any) says
/// it and the counter stays.
#[hook(addr = 0x6B7080, conv = thiscall, name = "CBike::PlayCarHorn", expect = "56 8B F1 66 8B 86 5C 04 00 00",
       unverified = "std13-veh-1", route = "L2", hazards = "none", recs = "grove-walk", note = "rand() at 0x6B70C0")]
pub fn play_car_horn(this: GPtr<u8>) {
    let alarm = rd::<u16>(this, 0x45C);
    if alarm != 0 && alarm != 0xFFFF && rd::<u8>(this, 0x36) & 0xF8 != 0x28 {
        return;
    }
    if rd::<u32>(this, 0x514) != 0 {
        return;
    }
    let t = rd::<u8>(this, 0x519);
    if t > 0 {
        wr::<u8>(this, 0x519, t - 1);
        return;
    }
    let r = at_site(0x6B70C5, rand) as u32;
    let b = ((r as u8) & 0x7F).wrapping_sub(0x6A);
    wr::<u8>(this, 0x519, b);
    let k = b & 7;
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if k >= 4 {
        if !driver.is_null() {
            ped_say(driver, 0x16, 0, 0x3F80_0000, 0, 0, 0);
        }
        return;
    }
    if k >= 2 && !driver.is_null() && rd::<u8>(this, 0x3DB) & 1 != 0 {
        ped_say(driver, 0x16, 0, 0x3F80_0000, 0, 0, 0);
    }
    wr::<u32>(this, 0x514, 45);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PLAY_CAR_HORN];
