//! `CAEPedAudioEntity::AddAudioEvent` (batch `mvpi-audio-1`): the ped audio entity's event dispatcher.
//! Ported from the 1.0 US bytes (the SEH handler push sits in the fragment 0x406864); unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::{GPtr, audio_hardware, g, rd, sound_manager, wr, xi};
use crate::entities::{are_sounds_of_this_event_playing_for_this_entity, hw_is_sound_bank_loaded};

/// The per-event volume table pointer (`[0xBD00F8]`, one int8 per event).
const EVENT_VOLUMES: usize = 0xBD00F8;

use sa_world::bindings::ae_random_int;
use crate::bindings::sm_request_new_sound;
use sa_world::bindings::sound_dtor;
use crate::bindings::sound_initialise;
orig! {
    /// `CAEPedAudioEntity::HandleFootstepEvent(event, volume, speed, surface)` (0x4E13A0)
    fn handle_footstep_event(this: GPtr<u8>, event: u32, vol: f32, speed: u32, surface: u32) = thiscall 0x4E13A0 => crate::port2_b::ped_handle_footstep_event;
    /// `CAEPedAudioEntity::HandleSkateEvent(event, volume, speed)` (0x4E17E0)
    fn handle_skate_event(this: GPtr<u8>, event: u32, vol: f32, speed: u32) = thiscall 0x4E17E0 => crate::port2_b::ped_handle_skate_event;
    /// `CAEPedAudioEntity::HandleLandingEvent(event)` (0x4E18E0)
    fn handle_landing_event(this: GPtr<u8>, event: u32) = thiscall 0x4E18E0 => crate::port2_b::ped_handle_landing_event;
    /// `CAEPedAudioEntity::HandlePedSwing(event, animSfx, delay)` (0x4E1A40)
    fn handle_ped_swing(this: GPtr<u8>, event: u32, sfx: u32, delay: u32) = thiscall 0x4E1A40 => crate::port2_b::ped_handle_ped_swing;
    /// `CAEPedAudioEntity::HandlePedHit(event, CPhysical*, surface, volume, maxVolume)` (0x4E1CC0)
    fn handle_ped_hit(this: GPtr<u8>, event: u32, physical: u32, surface: u32, vol: u32, max_vol: u32) = thiscall 0x4E1CC0 => crate::ped_hit::handle_ped_hit;
    /// `CAEPedAudioEntity::HandlePedJacked(event)` (0x4E2350)
    fn handle_ped_jacked(this: GPtr<u8>, event: u32) = thiscall 0x4E2350 => unported;
    /// `CAEPedAudioEntity::HandleSwimSplash(event)` (0x4E26A0)
    fn handle_swim_splash(this: GPtr<u8>, event: u32) = thiscall 0x4E26A0 => crate::port2_b::ped_handle_swim_splash;
    /// `CAEPedAudioEntity::HandleSwimWake(event)` (0x4E2790)
    fn handle_swim_wake(this: GPtr<u8>, event: u32) = thiscall 0x4E2790 => crate::port2_b::ped_handle_swim_wake;
}

use sa_world::world::common::entity_pos_u as entity_pos;

/// One of the two event-0x77 sounds: `Initialise(2, sfx, this, pos, volume, 1, 1, 1, 0, 0, 0, 0)`,
/// +0x1C = `speed`, +0x20 = 0.06, +0x18 = 1.5, event +0xC = `event`, `RequestNewSound`.
fn play(sound: GPtr<u8>, this: GPtr<u8>, physical: GPtr<u8>, sfx: u32, vol: u32, speed: u32, event: u32) {
    let p = entity_pos(physical);
    let (px, py, pz) = (p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read());
    sound_initialise(sound, 2, sfx, this.addr() as u32, px, py, pz, vol, 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0, 0, 0, 0);
    wr::<u32>(sound, 0x1C, speed);
    wr::<u32>(sound, 0x20, 0x3D75_C290);
    wr::<u32>(sound, 0x18, 0x3FC0_0000);
    wr::<u32>(sound, 0xC, event);
    sm_request_new_sound(GPtr::<u8>::addr(sound_manager()) as u32, sound.addr() as u32);
}

/// `CAEPedAudioEntity::AddAudioEvent(event, float volume, float speed, CPhysical*, uint8 surface, int32
/// a5, int32 a6)` (0x4E2BB0, thiscall, `ret 0x1C`). A stack `CAESound` (only +8 cleared; destroyed with
/// `~CAESound` on every path). Nothing when +0x7C (initialised) is clear, +0x94 (the ped) is null or
/// the event is outside 0x36..0x77. Through the exe's byte jump table: 0x36-0x37 →
/// `HandleFootstepEvent(event, volume, speed, surface)`; 0x38-0x39 → `HandleSkateEvent(event, volume,
/// speed)`; 0x3A-0x3B → `HandleLandingEvent`; 0x3C → `HandlePedSwing(event, a5, a6)`; 0x3D-0x44 →
/// `HandlePedHit(event, physical, surface, volume, a6)`; 0x45-0x49 → `HandlePedJacked`; 0x4A-0x4B →
/// `HandleSwimSplash`; 0x4C → `HandleSwimWake`; 0x77: volume = (float)(int8)[[0xBD00F8] + 0x77] +
/// volume (x87, stored as f32); unless bank 0x27 is loaded in slot 2 or a 0x77 sound of this entity
/// plays, nothing; else sound 0x1D of slot 2 at the physical's position (event 0x77), then, unless a
/// 0x79 sound of this entity plays, a random sfx 0x2F..0x31 (`GetRandomNumberInRange(0x2F, 0x31)`) as
/// event 0x79; each with +0x1C = speed, +0x20 = 0.06, +0x18 = 1.5. The other events do nothing.
///
/// The stack sound's bytes that `Initialise` does not write are stack leftovers in the original (zero
/// here) and `RequestNewSound` copies them.
#[hook(addr = 0x4e2bb0, conv = thiscall, name = "CAEPedAudioEntity::AddAudioEvent", expect = "6A FF E9 AD 3C F2 FF",
       unverified = "mvpi-audio-1", route = "L2", hazards = "uninit-locals", recs = "grove-walk")]
#[allow(clippy::too_many_arguments)]
pub fn ped_add_audio_event(
    this: GPtr<u8>,
    event: u32,
    vol: u32,
    speed: u32,
    physical: GPtr<u8>,
    surface: u32,
    a5: u32,
    a6: u32,
) {
    let mut buf = [0u32; 0x74 / 4];
    let sound = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(sound, 8, 0);
    if rd::<u8>(this, 0x7C) != 0 && rd::<u32>(this, 0x94) != 0 {
        match event {
            0x36 | 0x37 => handle_footstep_event(this, event, f32::from_bits(vol), speed, surface),
            0x38 | 0x39 => handle_skate_event(this, event, f32::from_bits(vol), speed),
            0x3A | 0x3B => handle_landing_event(this, event),
            0x3C => handle_ped_swing(this, event, a5, a6),
            0x3D..=0x44 => handle_ped_hit(this, event, physical.addr() as u32, surface, vol, a6),
            0x45..=0x49 => handle_ped_jacked(this, event),
            0x4A | 0x4B => handle_swim_splash(this, event),
            0x4C => handle_swim_wake(this, event),
            0x77 => {
                let table = g::<GPtr<u8>>(EVENT_VOLUMES).read();
                let v = xi(i32::from(rd::<i8>(table, 0x77))).fadd_m(f32::from_bits(vol)).to_f32().to_bits();
                if hw_is_sound_bank_loaded(audio_hardware(), 0x27, 2) != 0
                    && (are_sounds_of_this_event_playing_for_this_entity(sound_manager(), 0x77, this) as u32) as u16 == 0
                {
                    play(sound, this, physical, 0x1D, v, speed, 0x77);
                    if (are_sounds_of_this_event_playing_for_this_entity(sound_manager(), 0x79, this) as u32) as u16 == 0 {
                        let sfx = ae_random_int(0x2F, 0x31);
                        play(sound, this, physical, sfx as u32, v, speed, 0x79);
                    }
                }
            }
            _ => {}
        }
    }
    sound_dtor(sound);
}

pub const HOOKS: &[HookReg] = &[HOOK_PED_ADD_AUDIO_EVENT];
