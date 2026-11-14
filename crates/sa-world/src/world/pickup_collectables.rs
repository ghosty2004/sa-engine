//! WRITER-WORLD-2 (batch `world6-4`): the oyster and horseshoe collectable rewards. Ported from
//! the 1.0 US bytes; gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;
use sa_math::x87::X87;

use super::common::{ftol32, g, x};

use crate::bindings::garages_trigger_message as trigger_message;
use crate::bindings::get_stat_value;
use sa_core::bindings::stats_increment_stat as increment_stat;
use sa_core::bindings::audio_report_frontend_event as report_frontend_audio_event;

/// `CWorld::Players[0].m_nMoney` (0xB7CE50).
const MONEY: usize = 0xB7_CE50;

fn stat(id: u32) -> X87 {
    X87::from_f64_exact(get_stat_value(id as u16))
}

/// The common tail: money + 100; with every one collected (`stat(done) == f32(stat(have))`) the
/// "all" message (`all_key`, no numbers, 5 s) and money + 100,000; otherwise the progress message
/// (`key`, collected / total).
fn collected(have: u32, total: u32, key: u32, all_key: u32) {
    let m = g::<i32>(MONEY);
    m.write(m.read().wrapping_add(100));
    let got = stat(have).to_f32();
    if !stat(total).fcom(x(got)).test_parity(0x44) {
        trigger_message(all_key, -1, 0x1388, -1);
        m.write(m.read().wrapping_add(0x186A0));
    } else {
        let t = ftol32(stat(total));
        let h = ftol32(stat(have));
        trigger_message(key, h, 0x1388, t);
    }
}

/// `CPickups::PickedUpOyster()` (0x4552D0): the collect sound (13), oysters + 1 (stat 243), then
/// the money and message of [`collected`] against stat 244 ("all oysters" 0x859B1C, progress
/// 0x859B14).
#[hook(addr = 0x4552D0, conv = cdecl, name = "CPickups::PickedUpOyster",
       expect = "51 68 00 00 80 3F",
       unverified = "world6-4", route = "L2", hazards = "heap-writes")]
pub fn picked_up_oyster() {
    report_frontend_audio_event(GPtr::from_addr(0xB6_BC90), 0xD, 0.0, 1.0);
    increment_stat(0xF3, 1.0);
    collected(0xF3, 0xF4, 0x85_9B14, 0x85_9B1C);
}

/// `CPickups::PickedUpHorseShoe()` (0x455390): the collect sound, horseshoes + 1 (stat 241), luck
/// (stat 233) + `1000 / total horseshoes` (stat 242), then [`collected`] against stat 242 ("all
/// horseshoes" 0x859B2C, progress 0x859B24).
#[hook(addr = 0x455390, conv = cdecl, name = "CPickups::PickedUpHorseShoe",
       expect = "51 68 00 00 80 3F",
       unverified = "world6-4", route = "L2", hazards = "heap-writes")]
pub fn picked_up_horse_shoe() {
    report_frontend_audio_event(GPtr::from_addr(0xB6_BC90), 0xD, 0.0, 1.0);
    increment_stat(0xF1, 1.0);
    let luck = stat(0xF2).fdivr_m(1000.0).to_f32();
    increment_stat(0xE9, luck);
    collected(0xF1, 0xF2, 0x85_9B24, 0x85_9B2C);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PICKED_UP_OYSTER, HOOK_PICKED_UP_HORSE_SHOE];
