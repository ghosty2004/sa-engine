//! `CAEScriptAudioEntity::ProcessMissionAudioEvent` (0x4ECCF0, FRONTIER-2 lane C turn 6). Ported from the 1.0 US
//! bytes; gta-reversed's commented-out body is a reading aid with wrong case labels (the exe wins).
//!
//! The original is one switch on `(uint16) event` (a compare tree with five jump tables, 137 case blocks). The
//! dispatch has no side effects, so it is a `match`; every case block is one arm, and the blocks' recurring
//! shapes (a mission bank sound, a resident sound, a frontend event, a special ambience track, a cancel that
//! picks the physical variant, a door event at the physical's or the given position) are the helpers below.
//! The arguments passed on keep the full 32-bit event (`edi`), as the original pushes it.
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_world::world::common::entity_pos;

use crate::bindings::{
    ambience_stop_special_mission_ambience_track, are_sounds_of_this_event_playing_for_this_entity, hw_is_sound_bank_loaded, hw_load_sound_bank, report_weapon_event,
    script_play_mission_bank_sound, script_play_resident_sound_event, sm_are_sounds_of_event_playing_for_entity_and_physical,
    sm_are_sounds_playing_in_bank_slot, sm_cancel_sounds_in_bank_slot, sm_cancel_sounds_of_event, explosion_audio_add_event,
};
use crate::common::{AE_AMBIENCE_TRACK_MANAGER, AE_AUDIO_HARDWARE, AE_SOUND_MANAGER, AUDIO_ENGINE, GPtr, ae_random_int, g, rd, wr, xi};
use sa_ai::bindings::localisation_blood;
use sa_core::bindings::audio_report_frontend_event;
use sa_world::bindings::{audio_report_water_splash, door_audio_add_event, ped_audio_add_audio_event};

orig! {
    /// `CAEVehicleAudioEntity::DisableHelicoptors()` (0x4F4EE0, static; called with ECX = this)
    fn disable_helicoptors(this: GPtr<u8>) = thiscall 0x4F4EE0 => unported;
    /// `CAEVehicleAudioEntity::EnableHelicoptors()` (0x4F4EF0, static; called with ECX = this)
    fn enable_helicoptors(this: GPtr<u8>) = thiscall 0x4F4EF0 => unported;
    /// `CAEAmbienceTrackManager::PlaySpecialMissionAmbienceTrack(int32 track)` (0x4D6D50)
    fn ambience_play_special_mission_ambience_track(this: GPtr<u8>, track: u32) = thiscall 0x4D6D50 => unported;
    /// `CAESoundManager::CancelSoundsOfThisEventPlayingForThisEntityAndPhysical(int16 event, CAEAudioEntity*,
    /// CPhysical*)` (0x4EFBF0)
    fn sm_cancel_sounds_of_event_for_physical(sm: u32, event: u32, entity: u32, physical: u32) = thiscall 0x4EFBF0 => unported;
    /// `CAudioEngine::ReportWaterSplash(CVector posn, float volume)` (0x506F10, the vector by value)
    fn engine_report_water_splash_at(engine: GPtr<u8>, x: u32, y: u32, z: u32, volume: u32) = thiscall 0x506F10 => unported;
}

/// `[0xB7CB84]`, `CTimer::m_snTimeInMilliseconds`.
use sa_core::timer::TIME_MS;

/// The case blocks' common operands: the entity, the 32-bit event, the position and the physical.
#[derive(Clone, Copy)]
struct Ev {
    this: GPtr<u8>,
    event: u32,
    posn: GPtr<u8>,
    phys: u32,
}

/// `PlayMissionBankSound(event, posn, physical, sfx, slot, skipIfPlaying, volumeDelta, maxDistance, speed)` (0x4EC6D0).
fn bank(e: Ev, sfx: u32, slot: u32, skip: u32, vol: u32, dist: u32, speed: u32) {
    script_play_mission_bank_sound(e.this, e.event, e.posn, e.phys, sfx, slot, skip, vol, dist, speed);
}

/// The most common bank sound: volume delta 0, max distance 2.0, speed 1.0.
fn bank2(e: Ev, sfx: u32, slot: u32, skip: u32) {
    bank(e, sfx, slot, skip, 0, 0x4000_0000, 0x3F80_0000);
}

/// `PlayResidentSoundEvent(slot, bank, sfx, event, posn, physical, volumeDelta, speed, playTime, maxDistance)`
/// (0x4EC550).
#[allow(clippy::too_many_arguments)]
fn resident(e: Ev, slot: u32, bank_id: u32, sfx: u32, vol: u32, speed: u32, play_time: u32, dist: u32) {
    script_play_resident_sound_event(e.this, slot, bank_id, sfx, e.event, e.posn, e.phys, vol, speed, play_time, dist);
}

/// The common resident sound: volume delta 0, speed 1.0, play time 0, max distance 1.0.
fn resident1(e: Ev, slot: u32, bank_id: u32, sfx: u32) {
    resident(e, slot, bank_id, sfx, 0, 0x3F80_0000, 0, 0x3F80_0000);
}

/// `AudioEngine.ReportFrontendAudioEvent(event, volume, speed)` (0x506EA0).
fn frontend(event: u32, vol: u32, speed: u32) {
    audio_report_frontend_event(g(AUDIO_ENGINE), event, f32::from_bits(vol), f32::from_bits(speed));
}

/// `AEAmbienceTrackManager.PlaySpecialMissionAmbienceTrack(track)` (0x4D6D50).
fn ambience_play(track: u32) {
    ambience_play_special_mission_ambience_track(g(AE_AMBIENCE_TRACK_MANAGER), track);
}

/// `AEAmbienceTrackManager.StopSpecialMissionAmbienceTrack()` (0x4D6D60).
fn ambience_stop() {
    ambience_stop_special_mission_ambience_track(g(AE_AMBIENCE_TRACK_MANAGER));
}

/// `AESoundManager.CancelSoundsOfThisEventPlayingForThisEntity(event, this)` (0x4EFB90).
fn cancel(e: Ev, event: u32) {
    sm_cancel_sounds_of_event(AE_SOUND_MANAGER as u32, event, e.this.addr() as u32);
}

/// With a physical `CancelSoundsOfThisEventPlayingForThisEntityAndPhysical(event, this, physical)` (0x4EFBF0),
/// else [`cancel`].
fn cancel_for(e: Ev, event: u32) {
    if e.phys != 0 {
        sm_cancel_sounds_of_event_for_physical(AE_SOUND_MANAGER as u32, event, e.this.addr() as u32, e.phys);
    } else {
        cancel(e, event);
    }
}

/// `AESoundManager.AreSoundsOfThisEventPlayingForThisEntity(event, this)` (0x4EF570), the AX result tested.
fn playing(e: Ev) -> bool {
    are_sounds_of_this_event_playing_for_this_entity(g(AE_SOUND_MANAGER), e.event, e.this) != 0
}

/// `AESoundManager.AreSoundsOfThisEventPlayingForThisEntityAndPhysical(event, this, physical)` (0x4EF5D0).
fn playing_for(e: Ev) -> bool {
    sm_are_sounds_of_event_playing_for_entity_and_physical(g(AE_SOUND_MANAGER), e.event, e.this.addr() as u32, e.phys)
        != 0
}

/// `AEAudioHardware.IsSoundBankLoaded(bank, slot)` (0x4D88C0), the AL result tested.
fn bank_loaded(bank_id: u32, slot: u32) -> bool {
    hw_is_sound_bank_loaded(g(AE_AUDIO_HARDWARE), bank_id, slot) != 0
}

/// The physical's position (`m_matrix ? &m_matrix->pos : &m_placement.pos`) or `posn`.
fn pos_of(e: Ev) -> GPtr<u8> {
    if e.phys != 0 { entity_pos(GPtr::from_addr(e.phys as usize)) } else { e.posn }
}

/// `m_GarageAudio.AddAudioEvent(event, pos, 0.0, speed)` (0x4DC860, the door entity at +0x114) at [`pos_of`].
fn door(e: Ev, event: u32, speed: u32) {
    door_audio_add_event(e.this.byte_add(0x114), event, pos_of(e), 0.0, f32::from_bits(speed));
}

/// `+0x80` = the current time (`m_nLastTimeHornPlayed` in gta-reversed).
fn stamp(e: Ev) {
    wr::<u32>(e.this, 0x80, TIME_MS.get());
}

/// `+0x7E` = `((int8) +0x7E + 1) % 2` (signed remainder, as the original's `and 0x80000001` sequence).
fn toggle_7e(e: Ev) {
    let v = (i32::from(rd::<i8>(e.this, 0x7E)) + 1) % 2;
    wr::<u8>(e.this, 0x7E, v as u8);
}

/// 0x68 (crane winch): with +0x7D set, the resident winch sound unless one plays for this physical; then
/// speed +0x88, volume +0x84 = `(int8) [[0xBD00F8] + (uint16) event] + volume` (x87) and the time +0x80.
fn crane_winch(e: Ev, volume: u32, speed: u32) {
    if rd::<u8>(e.this, 0x7D) == 0 {
        return;
    }
    if !playing_for(e) {
        resident(e, 0x28, 0x2C, 1, volume, speed, 0, 0x4020_0000);
    }
    let table = g::<u8>(g::<u32>(0xBD00F8).read() as usize);
    let base = i32::from(rd::<i8>(table, e.event as u16 as isize));
    wr::<u32>(e.this, 0x88, speed);
    wr::<u32>(e.this, 0x84, xi(base).fadd_m(f32::from_bits(volume)).to_f32().to_bits());
    stamp(e);
}

/// 0x3FB: bank 0x2C into bank slot 0x28 unless loaded (cancelling that slot's sounds first when some play), then
/// the attached physical +0x110 and the flag +0x7D = 1.
fn attach_crane(e: Ev) {
    if !bank_loaded(0x2C, 0x28) {
        if sm_are_sounds_playing_in_bank_slot(g(AE_SOUND_MANAGER), 0x28) != 0 {
            sm_cancel_sounds_in_bank_slot(g(AE_SOUND_MANAGER), 0x28, 0);
        }
        hw_load_sound_bank(g(AE_AUDIO_HARDWARE), 0x2C, 0x28);
    }
    wr::<u32>(e.this, 0x110, e.phys);
    wr::<u8>(e.this, 0x7D, 1);
}

/// 0x3FC: with +0x7D, resident sfx 1 of the crane bank unless one plays for this physical.
fn crane_move(e: Ev) {
    if rd::<u8>(e.this, 0x7D) == 0 || playing_for(e) {
        return;
    }
    resident(e, 0x28, 0x2C, 1, 0, 0x3F80_0000, 0, 0x4020_0000);
}

/// 0x3FD: with +0x7D, cancels 0x3FC (physical variant) and plays sfx 2.
fn crane_stop(e: Ev) {
    if rd::<u8>(e.this, 0x7D) == 0 {
        return;
    }
    cancel_for(e, 0x3FC);
    resident(e, 0x28, 0x2C, 2, 0, 0x3F80_0000, 0, 0x4020_0000);
}

/// 0x3FE: with +0x7D, cancels bank slot 0x28 (forget), plays sfx 3, detaches (+0x110 = 0, +0x7D = 0).
fn detach_crane(e: Ev) {
    if rd::<u8>(e.this, 0x7D) == 0 {
        return;
    }
    sm_cancel_sounds_in_bank_slot(g(AE_SOUND_MANAGER), 0x28, 1);
    resident(e, 0x28, 0x2C, 3, 0, 0x3F80_0000, 0, 0x4020_0000);
    wr::<u32>(e.this, 0x110, 0);
    wr::<u8>(e.this, 0x7D, 0);
}

/// 0x404 / 0x453: sfx `first` of slot 2 when bank `first_bank` is loaded in slot 0x1C, else sfx `second` when bank
/// 0x159 is.
fn bank_by_loaded(e: Ev, first_bank: u32, first: u32, second: u32) {
    if bank_loaded(first_bank, 0x1C) {
        bank2(e, first, 2, 0);
    } else if bank_loaded(0x159, 0x1C) {
        bank2(e, second, 2, 0);
    }
}

/// 0x44C / 0x44D: one of two resident sounds of bank 0x27 by the +0x7E toggle, then the toggle.
#[allow(clippy::too_many_arguments)]
fn alternate(e: Ev, sfx_set: u32, speed_set: u32, time_set: u32, sfx_clear: u32, time_clear: u32) {
    if rd::<u8>(e.this, 0x7E) != 0 {
        resident(e, 2, 0x27, sfx_set, 0, speed_set, time_set, 0x3F80_0000);
    } else {
        resident(e, 2, 0x27, sfx_clear, 0, 0x3F80_0000, time_clear, 0x3F80_0000);
    }
    toggle_7e(e);
}

/// 0x48C: slot 3 sfx 0 at a local (-1, 0, 0) and sfx 1 at (1, 0, 0), both without a physical.
fn stereo_pair(e: Ev) {
    for (sfx, x) in [(0u32, 0xBF80_0000u32), (1, 0x3F80_0000)] {
        let mut v = [x, 0, 0];
        let p = GPtr::from_raw(v.as_mut_ptr().cast::<u8>());
        script_play_mission_bank_sound(e.this, e.event, p, 0, sfx, 3, 0, 0, 0x4000_0000, 0x3F80_0000);
    }
}

/// 0x4A5: with blood (`CLocalisation::Blood`) and a ped physical (type bits of +0x36 == 3), the ped's audio
/// entity (+0x138) `AddAudioEvent(0x77, 0.0, 1.0, physical, 0, 0, 0)`.
fn ped_crunch(e: Ev) {
    if localisation_blood() == 0 || e.phys == 0 {
        return;
    }
    let p = GPtr::<u8>::from_addr(e.phys as usize);
    if rd::<u8>(p, 0x36) & 7 != 3 {
        return;
    }
    ped_audio_add_audio_event(p.byte_add(0x138), 0x77, 0.0, 1.0, p, 0, 0, 0);
}

/// 0x478: a water splash (volume -6.0) on the physical, else at a copy of `posn`.
fn water_splash(e: Ev) {
    if e.phys != 0 {
        audio_report_water_splash(g(AUDIO_ENGINE), GPtr::from_addr(e.phys as usize), -6.0, 0);
    } else {
        let p = e.posn;
        engine_report_water_splash_at(g(AUDIO_ENGINE), rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), 0xC0C0_0000);
    }
}

/// `CAEScriptAudioEntity::ProcessMissionAudioEvent(eAudioEvents event, CVector& posn, CPhysical* physical, float
/// volume, float speed)` (0x4ECCF0, `ret 0x14`): one case block per `(uint16) event`, anything else does nothing.
#[hook(addr = 0x4ECCF0, conv = thiscall, name = "CAEScriptAudioEntity::ProcessMissionAudioEvent",
       expect = "83 EC 18 53 55 56 57", unverified = "frontier2-c12", route = "L2", hazards = "heap-writes",
       note = "the 0x4A6 block pushes bank 0x8F through the SecuROM fragment 0x405BA1 ([0x15607F8] - [0x156491C])")]
pub fn process_mission_audio_event(this: GPtr<u8>, event: u32, posn: GPtr<u8>, physical: u32, volume: u32, speed: u32) {
    let e = Ev { this, event, posn, phys: physical };
    match event as u16 {
        0x3E8 => disable_helicoptors(this),
        0x68 => crane_winch(e, volume, speed),
        0x3E9 => enable_helicoptors(this),
        0x3ED => bank2(e, 1, 2, 0),
        0x3EA => resident(e, 2, 39, 64, 0, 0x3F4A3D71, 35, 0x3F800000),
        0x3EE => cancel(e, 0x3ED),
        0x3F2 => {
            let sfx = ae_random_int(0, 2) as u32;
            bank2(e, sfx, 3, 0);
        }
        0x3F0 => cancel(e, 0x3EF),
        0x3F6 => bank2(e, 10, 3, 0),
        0x3F9 => {
            let sfx = ae_random_int(3, 5) as u32;
            bank2(e, sfx, 3, 0);
        }
        0x3FA | 0x49C => bank2(e, 7, 3, 0),
        0x3F4 => {
            if !playing(e) {
                bank2(e, 4, 3, 0);
            }
        }
        0x3FB => attach_crane(e),
        0x3FE => detach_crane(e),
        0x3FC => crane_move(e),
        0x3FD => crane_stop(e),
        0x401 => bank2(e, 1, 3, 0),
        0x402 => bank2(e, 0, 3, 0),
        0x412 => {
            bank2(e, 0, 3, 1);
            stamp(e);
        }
        0x413 | 0x462 => bank2(e, 2, 3, 1),
        0x414 => bank2(e, 1, 3, 1),
        0x415 | 0x431 => bank2(e, 5, 3, 0),
        0x416 | 0x42F | 0x49B => bank2(e, 4, 3, 0),
        0x417 => bank2(e, 3, 3, 1),
        0x418 => bank2(e, 7, 3, 1),
        0x419 => bank2(e, 6, 3, 1),
        0x41A => bank2(e, 9, 3, 1),
        0x3F8 | 0x411 => bank2(e, 8, 3, 0),
        0x40F => resident1(e, 5, 143, 65),
        0x403 => resident(e, 2, 39, 19, 0, 0x3F800000, 70, 0x3F800000),
        0x404 => bank_by_loaded(e, 0xF2, 0, 3),
        0x3EF => bank2(e, 0, 2, 0),
        0x406 | 0x457 | 0x484 => bank2(e, 1, 2, 0),
        0x408 => bank2(e, 1, 3, 0),
        0x409 => bank2(e, 0, 3, 0),
        0x40A => cancel_for(e, 0x409),
        0x407 => {
            let sfx = ae_random_int(2, 6) as u32;
            bank2(e, sfx, 3, 0);
        }
        0x41B => bank2(e, 10, 3, 1),
        0x42C => ambience_play(150),
        0x423 => bank2(e, 5, 3, 0),
        0x424 => bank2(e, 2, 3, 0),
        0x3F3 | 0x425 | 0x43F | 0x486 => bank2(e, 3, 3, 0),
        0x428 | 0x492 => bank2(e, 6, 3, 0),
        0x429 => bank2(e, 0, 3, 0),
        0x42A | 0x432 => bank2(e, 1, 3, 0),
        0x42B => bank2(e, 4, 3, 1),
        0x426 => ambience_play(153),
        0x42E => bank2(e, 6, 3, 0),
        0x430 => bank2(e, 2, 3, 0),
        0x433 => bank(e, 1, 3, 0, 0, 0x40000000, 0x3F4A3D71),
        0x436 | 0x49A => bank2(e, 3, 3, 1),
        0x434 => ambience_play(141),
        0x441 => bank2(e, 0, 3, 0),
        0x440 => {
            cancel_for(e, 0x441);
            bank2(e, 2, 3, 0);
        }
        0x442 => bank2(e, 1, 3, 0),
        0x43B => frontend(1, 0, 0x3F800000),
        0x43C => frontend(2, 0, 0x3F800000),
        0x43D => frontend(4, 0, 0x3F800000),
        0x43E => {
            bank2(e, 0, 3, 0);
            stamp(e);
            wr::<u8>(this, 0x7C, 0);
        }
        0x447 => resident(e, 2, 39, 65, 0, 0x3F800000, 16, 0x3F800000),
        0x438 => bank(e, 1, 2, 1, 0, 0x40400000, 0x3F800000),
        0x437 => bank(e, 2, 2, 1, 0, 0x40400000, 0x3F800000),
        0x439 => {
            bank(e, 3, 2, 1, 0, 0x40400000, 0x3F800000);
            bank(e, 0, 2, 0, 0, 0x40400000, 0x3F800000);
        }
        0x43A => {
            cancel(e, 0x439);
            bank(e, 4, 2, 1, 0, 0x40400000, 0x3F800000);
        }
        0x449 => ambience_play(145),
        0x41C | 0x41E => frontend(40, 0, 0x3F800000),
        0x41D | 0x41F => frontend(14, 0, 0x3F800000),
        0x420 => frontend(44, 0, 0x3F800000),
        0x421 => frontend(45, 0, 0x3F800000),
        0x44C => alternate(e, 0x45, 0x3F800000, 0x3F, 0x46, 0x3E),
        0x460 => bank2(e, 0, 3, 0),
        0x461 | 0x493 => bank2(e, 1, 3, 0),
        0x463 => bank2(e, 4, 3, 0),
        0x464 => bank2(e, 5, 3, 1),
        0x476 => ambience_play(160),
        0x48F => {
            bank2(e, 0, 3, 1);
            stamp(e);
        }
        0x45B | 0x494 => bank2(e, 2, 3, 0),
        0x495 => bank(e, 5, 3, 0, 0, 0x40000000, 0x3F2B851F),
        0x496 => bank(e, 5, 3, 0, 0, 0x40000000, 0x3F4A3D71),
        0x497 => bank2(e, 5, 3, 0),
        0x498 => bank(e, 8, 3, 1, 0, 0x40000000, 0x3EC28F5C),
        0x499 => {
            bank2(e, 0, 3, 1);
            stamp(e);
        }
        0x49D => ambience_play(172),
        0x40B | 0x481 => door(e, 0x9F, 0x3F800000),
        0x40C | 0x482 => door(e, 0xA0, 0x3F800000),
        0x485 => {
            if physical != 0 {
                report_weapon_event(g(AUDIO_ENGINE), 0x91, 0x1F, physical);
            }
        }
        0x46C => {
            if physical != 0 {
                report_weapon_event(g(AUDIO_ENGINE), 0x91, 0x2B, physical);
            } else {
                resident1(e, 5, 143, 45);
            }
        }
        0x487 => explosion_audio_add_event(this.byte_add(0x19C), 0x81, pos_of(e), 0.0),
        0x46B => {
            resident1(e, 2, 39, 33);
            resident(e, 2, 39, 50, 0, 0x3F4A3D71, 22, 0x3F800000);
        }
        0x479 => resident(e, 2, 39, 19, 0, 0x3F800000, 65, 0x3F800000),
        0x44D => alternate(e, 0x45, 0x3F70A3D7, 0x21, 0x46, 0x19),
        0x46A => {
            resident1(e, 5, 143, 58);
            resident1(e, 5, 143, 40);
        }
        0x46F => {
            resident(e, 5, 143, 34, 0x40A00000, 0x3F800000, 0, 0x3F800000);
            resident1(e, 5, 143, 40);
        }
        0x470 => resident(e, 5, 143, 82, 0xC0400000, 0x3F800000, 0, 0x3F800000),
        0x4A5 => ped_crunch(e),
        0x468 => {
            bank2(e, 2, 2, 0);
            bank2(e, 0, 2, 0);
        }
        0x469 => {
            cancel_for(e, 0x468);
            bank2(e, 3, 2, 0);
        }
        0x489 => bank2(e, 1, 2, 0),
        0x48A => cancel_for(e, 0x489),
        0x454 => {
            bank2(e, 1, 2, 0);
            bank2(e, 0, 2, 0);
        }
        0x455 => {
            cancel(e, 0x454);
            bank2(e, 2, 2, 0);
        }
        0x451 => bank2(e, 2, 2, 0),
        0x453 => bank_by_loaded(e, 0xA7, 3, 4),
        0x48C => stereo_pair(e),
        0x48D => door(e, 0x9F, 0x3F4A3D71),
        0x48E => door(e, 0xA0, 0x3F800000),
        0x48B => resident1(e, 41, 0, 0),
        0x459 => {
            bank(e, 0, 3, 1, 0xC1900000, 0x40000000, 0x3F800000);
            wr::<u32>(this, 0x8C, 0x3F800000);
        }
        0x45C => bank2(e, 3, 3, 0),
        0x45D => bank2(e, 7, 3, 0),
        0x45E => {
            bank(e, 1, 3, 1, 0xC1900000, 0x40000000, 0x3F800000);
            wr::<u32>(this, 0x8C, 0x3F800000);
        }
        0x45A | 0x45F => wr::<u32>(this, 0x8C, 0x40000000),
        0x3F1 | 0x474 | 0x47C => {
            let sfx = ae_random_int(20, 28) as u32;
            resident1(e, 2, 39, sfx);
        }
        0x475 => {
            let sfx = ae_random_int(20, 28) as u32;
            resident1(e, 2, 39, sfx);
            resident1(e, 2, 39, 65);
        }
        0x44E => bank2(e, 0, 2, 0),
        0x44F => bank2(e, 1, 2, 0),
        0x465 => bank2(e, 0, 2, 0),
        0x450 | 0x47F => bank2(e, 1, 2, 0),
        0x456 | 0x480 | 0x483 => bank2(e, 0, 2, 0),
        0x466 => {
            bank2(e, 1, 2, 0);
            bank2(e, 0, 2, 0);
        }
        0x467 => {
            cancel(e, 0x466);
            bank2(e, 2, 2, 0);
        }
        0x405 => bank2(e, 2, 2, 0),
        0x47B => {
            if !playing(e) {
                resident1(e, 17, 74, 7);
                stamp(e);
            }
        }
        0x478 => water_splash(e),
        0x490 => bank2(e, 0, 3, 0),
        0x491 => bank2(e, 1, 3, 0),
        0x49F => ambience_play(149),
        0x4A1 => ambience_play(142),
        0x4A3 => ambience_play(161),
        0x427 | 0x42D | 0x435 | 0x44A | 0x477 | 0x49E | 0x4A0 | 0x4A2 | 0x4A4 => ambience_stop(),
        0x471 => frontend(43, 0, 0x3F8F5C29),
        0x473 => frontend(43, 0, 0x3FA147AE),
        0x46D => frontend(46, 0, 0x3F800000),
        0x46E => frontend(16, 0, 0x3F800000),
        0x422 | 0x472 | 0x47D => frontend(43, 0, 0x3F800000),
        0x47E => frontend(7, 0, 0x3F800000),
        0x4A6 => {
            let sfx = ae_random_int(78, 80) as u32;
            resident1(e, 5, 143, sfx);
        }
        _ => {}
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_MISSION_AUDIO_EVENT];
