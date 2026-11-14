//! WRITER-WORLD-2 (batch `world6-3`): `CStats::RegisterFastestTime` / `RegisterBestPosition`, which
//! keep the minimum of a stat. Ported from the 1.0 US bytes (the two bodies are identical).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_math::x87::X87;

use super::common::{ftol32, g, x, xi};

use crate::bindings::stats_check_for_stats_message as check_for_stats_message;

/// `CStats::StatTypesFloat` (0xB79380, ids below 82) and `CStats::StatTypesInt` (0xB78E20, by id).
const FLOAT_STATS: usize = 0xB7_9380;
const INT_STATS: usize = 0xB7_8E20;

/// The shared body: an unset (0) stat takes `value`; otherwise the smaller of the stat and
/// `(float)value` (an unordered compare takes the value). Float stats store the f32, int stats
/// `(int16)ftol(...)` sign-extended. Ends in a tail call of `CheckForStatsMessage`.
fn register_min(id: u16, value: i32) {
    let i = usize::from(id);
    let float = id < 0x52;
    let cur = if float { x(g::<f32>(FLOAT_STATS + 4 * i).read()) } else { xi(g::<i32>(INT_STATS + 4 * i).read()) };
    let res: X87 = if !cur.fcom(x(0.0)).test_parity(0x44) {
        xi(value)
    } else {
        let v = xi(value).to_f32();
        if !cur.fcom(x(v)).test_parity(0x05) { cur } else { x(v) }
    };
    if float {
        g::<f32>(FLOAT_STATS + 4 * i).write(res.to_f32());
    } else {
        g::<i32>(INT_STATS + 4 * i).write(i32::from(ftol32(res) as i16));
    }
    check_for_stats_message();
}

/// `CStats::RegisterFastestTime(eStats id, int32 time)` (0x55A0B0): see [`register_min`].
#[hook(addr = 0x55A0B0, conv = cdecl, name = "CStats::RegisterFastestTime",
       expect = "8B 4C 24 04 66 83 F9 52",
       unverified = "world6-3", route = "L2", hazards = "none",
       note = "the min path overwrites the id argument slot with (float)time before the tail call (no arguments read)")]
pub fn register_fastest_time(id: u32, time: i32) {
    register_min(id as u16, time);
}

/// `CStats::RegisterBestPosition(eStats id, int32 position)` (0x55A160): see [`register_min`].
#[hook(addr = 0x55A160, conv = cdecl, name = "CStats::RegisterBestPosition",
       expect = "8B 4C 24 04 66 83 F9 52",
       unverified = "world6-3", route = "L2", hazards = "none")]
pub fn register_best_position(id: u32, position: i32) {
    register_min(id as u16, position);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_REGISTER_FASTEST_TIME, HOOK_REGISTER_BEST_POSITION];

