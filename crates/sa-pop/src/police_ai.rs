//! FRONTIER-2: the police car choices `CCarAI::FindPolice*ForWantedLevel` (0x41C9D0, 0x41CA40,
//! 0x41CAA0), on the police path the crossing scenario reaches. Ported from the 1.0 US bytes.

use sa_core::bindings::random_float_in_range;
use sa_core::crt::rand_at;
use sa_core::field::gf;
use sa_core::mem::GPtr;
use sa_stream::bindings::find_player_wanted;

use crate::common::*;

/// The player's wanted level (`FindPlayerWanted(-1) + 0x2C`).
fn wanted_level() -> u32 {
    rd::<u32>(find_player_wanted(-1), 0x2C)
}

/// `CCarAI::FindPoliceCarMissionForWantedLevel()` (0x41C9D0, cdecl, AL): wanted 0 / 1 and above 6 → 4 (block player
/// far away); 2 / 3 / 4..6 → `(rand() & 3) < 3 / 2 / 1 ? 4 : 2`.
/// Registered for phase B only (`#[native]`): protection code calls into the entry's patched bytes (0x4049D4).
#[sa_core::native(addr = 0x41C9D0, conv = cdecl, name = "CCarAI::FindPoliceCarMissionForWantedLevel")]
pub fn find_police_car_mission_for_wanted_level() -> u8 {
    let lim = match wanted_level() {
        2 => (3u8, 0x41_C9EE),
        3 => (2, 0x41_C9FF),
        4..=6 => (1, 0x41_CA10),
        _ => return 4,
    };
    let r = (rand_at(lim.1) as u8) & 3;
    u8::from(r < lim.0) * 2 + 2
}

/// `CCarAI::FindPoliceBikeMissionForWantedLevel()` (0x41CA40, cdecl, AL): 0x2B.
/// Registered for phase B only (`#[native]`): the body is 3 bytes, shorter than the hook's jump.
#[sa_core::native(addr = 0x41CA40, conv = cdecl, name = "CCarAI::FindPoliceBikeMissionForWantedLevel")]
pub fn find_police_bike_mission_for_wanted_level() -> u8 {
    0x2B
}

/// `CCarAI::FindPoliceCarSpeedForWantedLevel(CVehicle*)` (0x41CAA0, cdecl): wanted 0 → `ftol(RandomFloat(12, 16))`,
/// 1 → 25, 2 → 34, 3..6 → `ftol(handling(+0x384) +0x88 * [0x858C94 / 0x858C90 / 0x858C8C / 0x858C88])`, else 0.
/// Registered for phase B only (`#[native]`): the no-CD code reads and writes the entry bytes as data (0x14B1FBF, 0x14B351B).
#[sa_core::native(addr = 0x41CAA0, conv = cdecl, name = "CCarAI::FindPoliceCarSpeedForWantedLevel")]
pub fn find_police_car_speed_for_wanted_level(veh: GPtr<u8>) -> i32 {
    let by = |k: usize| ftol32(x(rd::<f32>(rd::<GPtr<u8>>(veh, 0x384), 0x88)).fmul_m(gf(k)));
    match wanted_level() {
        0 => ftol32(random_float_in_range(12.0, 16.0)),
        1 => 0x19,
        2 => 0x22,
        3 => by(0x85_8C94),
        4 => by(0x85_8C90),
        5 => by(0x85_8C8C),
        6 => by(0x85_8C88),
        _ => 0,
    }
}

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    NATIVE_FIND_POLICE_CAR_MISSION_FOR_WANTED_LEVEL,
    NATIVE_FIND_POLICE_BIKE_MISSION_FOR_WANTED_LEVEL,
    NATIVE_FIND_POLICE_CAR_SPEED_FOR_WANTED_LEVEL,
];
