//! STD-14: a phase-B registration of `CAERadioTrackManager::QueueUpTracksForStation` (0x4EA670), ported in
//! `port3_lead.rs` but not hookable in phase A (its entry is a `jmp` into the in-text detour 0x4020D0). Reached
//! when the radio of a car the player enters starts.

use sa_core::mem::GPtr;

/// `CAERadioTrackManager::QueueUpTracksForStation(eRadioID, int8*, int8, tRadioSettings&)` (0x4EA670, thiscall,
/// `ret 0x10`): [`crate::port3_lead::radio_queue_up_tracks_for_station`].
#[sa_core::native(addr = 0x4EA670, conv = thiscall, name = "CAERadioTrackManager::QueueUpTracksForStation")]
pub fn radio_queue_up_tracks_for_station(
    this: GPtr<u8>,
    station: u32,
    count: GPtr<u8>,
    ty: u32,
    s: GPtr<u8>,
) -> u8 {
    crate::port3_lead::radio_queue_up_tracks_for_station(this, station, count, ty, s)
}

/// This module's phase-B registrations.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_RADIO_QUEUE_UP_TRACKS_FOR_STATION];
