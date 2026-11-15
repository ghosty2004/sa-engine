//! FRONTIER-2 (batch `frontier2-1`): `CAEPedAudioEntity::HandlePedHit` (0x4E1CC0, user-hit in melee). Ported from the 1.0 US bytes (gta-reversed has not reversed it): the weapon's hit
//! sound through the exe's byte jump table, then the event's second sound through three nested tables.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::bindings::{
    hw_is_sound_bank_loaded, hw_load_sound_bank, sm_request_new_sound, sound_initialise,
    sound_register_with_physical_entity, sound_set_flags,
};
use crate::common::{GPtr, X87, audio_engine, audio_hardware, g, rd, sound_manager, wr, x, xi};
use sa_world::bindings::{ae_random_int, engine_is_loading_tune_active, report_collision};
use sa_world::world::common::entity_pos;

/// The per-event volume table pointer (`[0xBD00F8]`, one int8 per event).
const EVENT_VOLUMES: usize = 0xBD_00F8;
/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: usize = 0xB7_CB84;

orig! {
    /// The surface volume 0x4DA510 (cdecl, ST0)
    fn surface_volume_o(surface: u32) -> X87 = reg("args = stack") 0x4DA510 => crate::port3_ent::collision_surface_volume;
}

/// Where the original continues after choosing the sounds.
#[derive(Clone, Copy, PartialEq)]
enum Next {
    /// 0x4E1EB4: the hit sound when its id (int16) is not negative, then `CheckSecond`
    CheckFirst,
    /// 0x4E1EC0: the hit sound, then `CheckSecond`
    First,
    /// 0x4E2164: the second sound when its id (int16) is not negative
    CheckSecond,
    /// 0x4E216D: the second sound
    Second,
}

/// `CAEPedAudioEntity::HandlePedHit(int32 event, CPhysical* physical, uint8 surface, float volume, uint32
/// maxVolume)` (0x4E1CC0, `ret 0x14`).
///
/// Needs sound bank 0x8F in slot 5 (else: unless the loading tune plays, it is requested, nothing else). The base
/// volume is `(int8)[[0xBD00F8] + event] + volume`. The hit sound (slot 5, speed variance 0.06) by the ped's current
/// weapon: 0 / 44 / 45 → 0x3A, the bat-like ones 0x2C, 2 → 0x30, 3 → 0x23, 4 → 0x39 (variance 0.03), 5 → 0x22, 6 →
/// 0x4B, 7 → 0x43, 8 → 0x38, 10..13 → 0x32, 14 → 0x36 (0.07), 15 → 0x2E, others none. Hitting a ped (events
/// 0x3D..0x44): a second sound (default slot 5, volume offset -3) by event and weapon (punch ranges 0x28..0x2B /
/// 0x24..0x27 with volume offsets 1 / -10 / -15 on the hit sound, 0x51 / 0x52 body hits, the chainsaw's bank 0x1B in
/// slot 3 with 7..9, 0x3B..0x3E); hitting anything else reports a collision (0x3E with the surface) to the audio
/// engine and for events 0x3D..0x3F and 0x41..0x43 offsets the hit sound by the surface volume (0x4DA510). Each sound:
/// `Initialise` at the ped, registered with it; a positive `maxVolume` sets a stop time `now + maxVolume` (+0x10 of the
/// sound), the event (+0xC) and flag 4; `RequestNewSound`.
#[hook(addr = 0x4E1CC0, conv = thiscall, name = "CAEPedAudioEntity::HandlePedHit", expect = "83 EC 14 53 55 56 8B F1",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes",
       note = "the hit-sound test runs through the protection thunk 0x4067B5 and back")]
#[allow(clippy::too_many_lines)]
pub fn handle_ped_hit(this: GPtr<u8>, event: u32, physical: u32, surface: u32, volume: u32, max_volume: u32) {
    let ped = rd::<GPtr<u8>>(this, 0x94);
    let slot = isize::from(rd::<i8>(ped, 0x718));
    let weapon = ped.byte_add(0x5A0 + slot * 0x1C);
    let mut bank: u32 = 5;
    let mut off1: u32 = 0;
    let mut off2: u32 = 0xC040_0000;
    let mut variance: u32 = 0x3D70_D845;
    if weapon.is_null() {
        return;
    }
    if hw_is_sound_bank_loaded(audio_hardware(), 0x8F, 5) == 0 {
        if engine_is_loading_tune_active(audio_engine()) != 0 {
            return;
        }
        hw_load_sound_bank(audio_hardware(), 0x8F, 5);
        return;
    }
    let base = g::<GPtr<i8>>(EVENT_VOLUMES).read().byte_add(event as isize).read();
    let vol = xi(i32::from(base)).fadd_m(f32::from_bits(volume)).to_f32();
    let w = rd::<u32>(weapon, 0);
    let mut hit: i32 = match w {
        0 | 44 | 45 => 0x3A,
        1 | 16..=18 | 22..=34 | 37..=43 => 0x2C,
        2 => 0x30,
        3 => 0x23,
        4 => {
            variance = 0x3CF5_C28F;
            0x39
        }
        5 => 0x22,
        6 => 0x4B,
        7 => 0x43,
        8 => 0x38,
        10..=13 => 0x32,
        14 => {
            variance = 0x3D8F_5C29;
            0x36
        }
        15 => 0x2E,
        _ => -1,
    };
    let entity = GPtr::<u8>::from_addr(physical as usize);
    let t = event.wrapping_sub(0x3D);
    let mut second: i32;
    let next;
    // the ped's position, as ReportCollision gets it
    let report = || {
        let ped = rd::<GPtr<u8>>(this, 0x94);
        report_collision(audio_engine(), ped, entity, 0x3E, surface & 0xFF, entity_pos(ped), GPtr::NULL, 1.0, 1.0, 1, 0);
    };
    if !entity.is_null() && rd::<u8>(entity, 0x36) & 7 == 3 {
        second = 3;
        let punch = |lo: i32, hi: i32, off: u32, off1: &mut u32| {
            *off1 = off;
            ae_random_int(lo, hi)
        };
        match t {
            0 | 1 => {
                let (lo, hi) = if t == 0 { (0x28, 0x2B) } else { (0x24, 0x27) };
                match w.wrapping_sub(1) {
                    0 => {
                        second = punch(lo, hi, 0x3F80_0000, &mut off1);
                        next = Next::CheckFirst;
                    }
                    3 | 7 => {
                        second = 0x51;
                        off2 = 0xC110_0000;
                        next = Next::Second;
                    }
                    4 => {
                        second = punch(lo, hi, 0xC120_0000, &mut off1);
                        next = Next::CheckFirst;
                    }
                    8 => {
                        if hw_is_sound_bank_loaded(audio_hardware(), 0x1B, second as u32) == 0 {
                            return;
                        }
                        bank = second as u32;
                        second = ae_random_int(7, 9);
                        next = Next::CheckSecond;
                    }
                    13 => {
                        second = -1;
                        next = Next::CheckFirst;
                    }
                    _ => {
                        second = punch(lo, hi, 0xC170_0000, &mut off1);
                        next = Next::CheckFirst;
                    }
                }
            }
            2 => match w.wrapping_sub(4) {
                0 | 4 => {
                    second = 0x51;
                    off2 = 0xC110_0000;
                    next = Next::Second;
                }
                1 => {
                    off1 = 0xC120_0000;
                    second = 0x52;
                    off2 = 0xC0C0_0000;
                    next = Next::CheckFirst;
                }
                5 => {
                    if hw_is_sound_bank_loaded(audio_hardware(), 0x1B, second as u32) == 0 {
                        return;
                    }
                    bank = second as u32;
                    second = ae_random_int(7, 9);
                    next = Next::CheckSecond;
                }
                10 => {
                    second = -1;
                    next = Next::CheckFirst;
                }
                _ => {
                    off1 = 0xC170_0000;
                    second = 0x52;
                    off2 = 0xC0C0_0000;
                    next = Next::CheckFirst;
                }
            },
            3 => {
                second = 0x52;
                off2 = 0xC0C0_0000;
                next = Next::Second;
            }
            4 | 6 => {
                hit = 0x3A;
                second = ae_random_int(0x28, 0x2B);
                next = Next::First;
            }
            5 => {
                hit = 0x3A;
                second = ae_random_int(0x24, 0x27);
                next = Next::First;
            }
            7 => {
                second = ae_random_int(0x3B, 0x3E);
                next = Next::CheckSecond;
            }
            _ => {
                second = -1;
                next = Next::CheckFirst;
            }
        }
    } else {
        second = -1;
        match t {
            0..=2 => {
                off1 = surface_volume_o(surface & 0xFF).to_f32().to_bits();
                report();
                next = Next::CheckFirst;
            }
            3 | 7 => {
                report();
                return;
            }
            4..=6 => {
                hit = 0x3A;
                off1 = surface_volume_o(surface & 0xFF).to_f32().to_bits();
                report();
                next = Next::First;
            }
            _ => next = Next::CheckFirst,
        }
    }
    let play = |bank: u32, sfx: i32, off: u32, variance: u32| {
        let ped = rd::<GPtr<u8>>(this, 0x94);
        let p = entity_pos(ped);
        let v = x(vol).fadd_m(f32::from_bits(off)).to_f32().to_bits();
        let sound = this.byte_add(8);
        sound_initialise(sound, bank, sfx as u32, this.addr() as u32, rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), v,
                         0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0, 0x80, variance, 0);
        sound_register_with_physical_entity(sound, rd::<GPtr<u8>>(this, 0x94));
        if max_volume > 0 {
            wr::<u32>(this, 0x28, 0);
            wr::<u32>(this, 0x24, 0);
            let t = g::<u32>(TIME_MS).read().wrapping_add(max_volume);
            let mut f = xi(t as i32);
            if (t as i32) < 0 {
                f = f.fadd_m(g::<f32>(0x85_8C54).read());
            }
            wr(this, 0x18, f.to_f32());
            wr::<u32>(this, 0x14, event);
            sound_set_flags(sound, 4, 1);
        }
        sm_request_new_sound(sound_manager().addr() as u32, sound.addr() as u32);
    };
    let first = match next {
        Next::CheckFirst => hit as i16 >= 0,
        Next::First => true,
        Next::CheckSecond | Next::Second => false,
    };
    if first {
        play(5, hit, off1, variance);
    }
    if next != Next::Second && (second as i16) < 0 {
        return;
    }
    play(bank, second, off2, 0x3D70_D845);
}

pub const HOOKS: &[HookReg] = &[HOOK_HANDLE_PED_HIT];
