//! The smaller `CAE*AudioEntity` classes (PORT-AUDIO): `CAEWeaponAudioEntity`,
//! `CAEGlobalWeaponAudioEntity`, `CAECollisionAudioEntity`, `CAEDoorAudioEntity`, `CAEFireAudioEntity`,
//! `CAEExplosionAudioEntity`, `CAEWeatherAudioEntity`, `CAEScriptAudioEntity`, `CAEFrontendAudioEntity`,
//! `CAEPoliceScannerAudioEntity`, `CAETwinLoopSoundEntity`, `CAEWaterCannonAudioEntity`.
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids.
//!
//! Common `CAEAudioEntity` layout: +0 vtable, +4 entity, +0x7C.. class fields.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_core::timer::{TIME_MS, TIME_MS_PAUSE_MODE};

use crate::common::{
    AE_SOUND_MANAGER, ae_random_int, audio_engine, audio_hardware, ftol32, g, rd, sound_manager, wr, x,
};

sa_core::globals! {
    list GLOBALS;
    /// `CAEPoliceScannerAudioEntity::s_fVolumeOffset`
    pub static PS_VOLUME_OFFSET @ 0xB61CF8: u32 = 0;
    /// `CAEPoliceScannerAudioEntity::s_bStoppingScanner`
    pub static PS_STOPPING_SCANNER @ 0xB61CFC: u8 = 0;
    /// `CAEPoliceScannerAudioEntity::s_pSound` (`CAESound*`)
    pub static PS_SOUND @ 0xB61D00: u32 = 0;
    /// `CAEPoliceScannerAudioEntity::s_pCurrentSlots`
    pub static PS_CURRENT_SLOTS @ 0xB61D10: u32 = 0;
    /// `CAEPoliceScannerAudioEntity::s_pPSControlling`
    pub static PS_CONTROLLING @ 0xB61D48: u32 = 0;
    /// `CAEPoliceScannerAudioEntity::s_nScannerPlaybackState`
    pub static PS_PLAYBACK_STATE @ 0xB61D4C: u16 = 0;
    /// `CAEPoliceScannerAudioEntity::s_bScannerDisabled`
    pub static PS_SCANNER_DISABLED @ 0xB61D4E: u8 = 0;
    /// `CAEPoliceScannerAudioEntity::s_NextNewScannerDialogueTime`
    pub static PS_NEXT_DIALOGUE_TIME @ 0xB61D50: u32 = 0;
}

orig! {
    /// `CAEAudioHardware::LoadSoundBank(uint16 bankId, int16 slot)` (0x4D88A0)
    pub fn load_sound_bank(hw: GPtr<u8>, bank: u32, slot: u32) -> u32 = thiscall 0x4D88A0 => unported;
    /// `CAESoundManager::CancelSoundsOwnedByAudioEntity(CAEAudioEntity*, bool)` (0x4EFCD0)
    pub fn cancel_sounds_owned_by_audio_entity(sm: GPtr<u8>, entity: GPtr<u8>, stop: u32) = thiscall 0x4EFCD0 => unported;
    /// `CAESoundManager::AreSoundsOfThisEventPlayingForThisEntity(int16 event, CAEAudioEntity*)` (0x4EF570)
    pub fn are_sounds_of_this_event_playing_for_this_entity(sm: GPtr<u8>, event: u32, entity: GPtr<u8>) -> u32 = thiscall 0x4EF570 => unported;
    /// `CAESoundManager::AreSoundsPlayingInBankSlot(int16)` (0x4EF520): 0, 1, or 2 in AX
    pub fn are_sounds_playing_in_bank_slot(sm: GPtr<u8>, slot: u32) -> u32 = thiscall 0x4EF520 => unported;
    /// `CAESound::StopSoundAndForget()` (0x4EF850)
    pub fn sound_stop_and_forget(sound: GPtr<u8>) = thiscall 0x4EF850 => unported;
    /// `CAudioEngine::IsLoadingTuneActive()` (0x506D90)
    pub fn engine_is_loading_tune_active(engine: GPtr<u8>) -> u8 = thiscall 0x506D90 => unported;
    /// `CAEPoliceScannerAudioEntity::FinishedPlayingScannerDialogue()` (0x4E6C30, static)
    pub fn ps_finished_playing_scanner_dialogue() = cdecl 0x4E6C30 => unported;
    /// `CAEWeaponAudioEntity::Terminate()` (0x503480)
    pub fn weapon_terminate_call(this: GPtr<u8>) = thiscall 0x503480 => weapon_terminate;
    /// `CAETwinLoopSoundEntity::StopSoundAndForget()` (0x4F2960)
    pub fn twin_loop_stop_sound_and_forget_call(this: GPtr<u8>) = thiscall 0x4F2960 => twin_loop_stop_sound_and_forget;
    /// `CAETwinLoopSoundEntity::SwapSounds()` (0x4F2C10)
    pub fn twin_loop_swap_sounds_call(this: GPtr<u8>) = thiscall 0x4F2C10 => twin_loop_swap_sounds;
    /// `CAEScriptAudioEntity::ProcessMissionAudioEvent(event, CVector&, CPhysical*, float, float)` (0x4ECCF0)
    pub fn script_process_mission_audio_event(this: GPtr<u8>, event: u32, posn: u32, physical: u32, a4: u32, volume: u32) = thiscall 0x4ECCF0 => unported;
    /// `CAESound::~CAESound()` (0x4EF660)
    pub fn sound_destructor(sound: GPtr<u8>) -> u32 = thiscall 0x4EF660 => unported;
    /// `CAEWeaponAudioEntity::WeaponFire(eWeaponType, CPhysical*, int32 event)` (0x504F80)
    pub fn weapon_weapon_fire(this: GPtr<u8>, ty: u32, physical: u32, event: u32) = thiscall 0x504F80 => unported;
    /// `CAEWeaponAudioEntity::WeaponReload(eWeaponType, CPhysical*, int32 event)` (0x503690)
    pub fn weapon_weapon_reload(this: GPtr<u8>, ty: u32, physical: u32, event: u32) = thiscall 0x503690 => unported;
    /// `CAEWeaponAudioEntity::ReportStealthKill(eWeaponType, CPhysical*, int32 event)` (0x503B20)
    pub fn weapon_report_stealth_kill(this: GPtr<u8>, ty: u32, physical: u32, event: u32) = thiscall 0x503B20 => unported;
    /// `CAEWeaponAudioEntity::ReportChainsawEvent(CPhysical*, int32 event)` (0x503910)
    pub fn weapon_report_chainsaw_event(this: GPtr<u8>, physical: u32, event: u32) = thiscall 0x503910 => unported;
    /// `CAEGlobalWeaponAudioEntity::ProjectileFire(eWeaponType, CPhysical*, int32 event)` (0x4DF060)
    pub fn global_weapon_projectile_fire(this: GPtr<u8>, ty: u32, physical: u32, event: u32) = thiscall 0x4DF060 => unported;
    /// `CAEDoorAudioEntity::PlayDoorSound(int16 sfx, eAudioEvents, CVector&, float volDelta, float speed)` (0x4DC6D0)
    pub fn door_play_door_sound(this: GPtr<u8>, sfx: u32, event: u32, posn: u32, vol: u32, speed: u32) = thiscall 0x4DC6D0 => unported;
    /// `CAESound::SetPosition(CVector)` (0x4EF880, the vector by value)
    pub fn sound_set_position(sound: GPtr<u8>, px: u32, py: u32, pz: u32) = thiscall 0x4EF880 => unported;
    /// `CAEAudioHardware::IsSoundBankLoaded(uint16 bankId, int16 slot)` (0x4D88C0)
    pub fn hw_is_sound_bank_loaded(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    /// `CAECollisionAudioEntity::PlayBulletHitCollisionSound(eSurfaceType, CVector&, float angle)` (0x4DB7C0)
    pub fn collision_play_bullet_hit_collision_sound(this: GPtr<u8>, surface: u32, posn: u32, angle: u32) = thiscall 0x4DB7C0 => unported;
    /// `CAEFireAudioEntity::PlayFireSounds(eAudioEvents, CVector&)` (0x4DD0D0)
    pub fn fire_play_fire_sounds(this: GPtr<u8>, event: u32, posn: u32) = thiscall 0x4DD0D0 => unported;
    /// `CAEFireAudioEntity::PlayWaterSounds(eAudioEvents, CVector&)` (0x4DD270)
    pub fn fire_play_water_sounds(this: GPtr<u8>, event: u32, posn: u32) = thiscall 0x4DD270 => unported;
}

/// `CAECollisionAudioEntity` sound-id ranges per surface: 16-byte rows `{ .., +0 min, +4 max, .. }` at
/// 0x8AD908 (`.data`).
const COLLISION_SOUND_RANGES: usize = 0x8AD908;
/// 0.0f (`.rdata`)
const FLT_0: usize = 0x858B50;
/// 1.0f (`.rdata`)
const FLT_1: usize = 0x858624;

/// Stops and forgets the `CAESound*` stored at `this + off` and clears it.
pub fn stop_sound_field(this: GPtr<u8>, off: isize) {
    let s = rd::<u32>(this, off);
    if s != 0 {
        sound_stop_and_forget(g(s as usize));
        wr::<u32>(this, off, 0);
    }
}

// ---- CAEWeaponAudioEntity -------------------------------------------------------------------------

/// `CAEWeaponAudioEntity::Initialise()` (0x503450): +0x7F = 3, +0x80 = 4; bank 0x8F into slot 5 unless
/// the loading tune is playing.
#[hook(addr = 0x503450, conv = thiscall, name = "CAEWeaponAudioEntity::Initialise", expect = "C6 41 7F 03 C6 81 80 00 00 00 04")]
pub fn weapon_initialise(this: GPtr<u8>) {
    wr::<u8>(this, 0x7F, 3);
    wr::<u8>(this, 0x80, 4);
    if engine_is_loading_tune_active(audio_engine()) == 0 {
        load_sound_bank(audio_hardware(), 0x8F, 5);
    }
}

/// `CAEWeaponAudioEntity::Terminate()` (0x503480): cancels every sound it owns.
#[hook(addr = 0x503480, conv = thiscall, name = "CAEWeaponAudioEntity::Terminate", expect = "6A 01 51 B9 B0 2C B6 00")]
pub fn weapon_terminate(this: GPtr<u8>) {
    cancel_sounds_owned_by_audio_entity(sound_manager(), this, 1);
}

/// `CAEWeaponAudioEntity::StopFlameThrowerIdleGasLoop()` (0x5034E0): the sound at +0x9C.
#[hook(addr = 0x5034E0, conv = thiscall, name = "CAEWeaponAudioEntity::StopFlameThrowerIdleGasLoop", expect = "56 8B F1 8B 8E 9C 00 00 00")]
pub fn weapon_stop_flame_thrower_idle_gas_loop(this: GPtr<u8>) {
    stop_sound_field(this, 0x9C);
}

// ---- CAEGlobalWeaponAudioEntity ------------------------------------------------------------------

/// `CAEGlobalWeaponAudioEntity::AddAudioEvent(eAudioEvents, eWeaponType, CPhysical*)` (0x4DFAA0): with a
/// physical, events 0x91 / 0x95 fire, 0x92 / 0x93 reload, 0x94 fires a projectile; others ignored.
#[hook(addr = 0x4DFAA0, conv = thiscall, name = "CAEGlobalWeaponAudioEntity::AddAudioEvent", expect = "8B 54 24 0C 85 D2")]
pub fn global_weapon_add_audio_event(this: GPtr<u8>, event: u32, ty: u32, physical: u32) {
    if physical == 0 {
        return;
    }
    match event.wrapping_sub(0x91) {
        0 | 4 => weapon_weapon_fire(this, ty, physical, event),
        1 | 2 => weapon_weapon_reload(this, ty, physical, event),
        3 => global_weapon_projectile_fire(this, ty, physical, event),
        _ => {}
    }
}

// ---- CAECollisionAudioEntity ----------------------------------------------------------------------

/// `CAECollisionAudioEntity::InitialisePostLoading()` (0x4DA050): banks 0x27 -> slot 2, 0x1B -> slot 3.
#[hook(addr = 0x4DA050, conv = thiscall, name = "CAECollisionAudioEntity::InitialisePostLoading", expect = "6A 02 6A 27 B9 B8 F8 B5 00")]
pub fn collision_initialise_post_loading(_this: GPtr<u8>) {
    load_sound_bank(audio_hardware(), 0x27, 2);
    load_sound_bank(audio_hardware(), 0x1B, 3);
}

/// `CAECollisionAudioEntity::Initialise()` (0x5B9BD0): the 300 collision entries (0x14 bytes from
/// +0x208) cleared with both surfaces 0xC3, the last-sound-id words +0x7C..+0x201 and +0x202 = -1,
/// +0x204 = 0.
#[hook(addr = 0x5B9BD0, conv = thiscall, name = "CAECollisionAudioEntity::Initialise", expect = "53 56 8B F1 33 C9")]
pub fn collision_initialise(this: GPtr<u8>) {
    wr::<u32>(this, 0x204, 0);
    for i in 0..300isize {
        let e = this.byte_add(0x208 + i * 0x14);
        wr::<u32>(e, 0, 0);
        wr::<u32>(e, 4, 0);
        wr::<u8>(e, 0x11, 0xC3);
        wr::<u8>(e, 0x12, 0xC3);
        wr::<u32>(e, 8, 0);
        wr::<u8>(e, 0x10, 0);
        wr::<u32>(e, 0xC, 0);
    }
    for i in 0..0x61isize {
        wr::<u32>(this, 0x7C + i * 4, 0xFFFF_FFFF);
    }
    wr::<u16>(this, 0x200, 0xFFFF);
    wr::<u16>(this, 0x202, 0xFFFF);
}

/// `CAECollisionAudioEntity::Service()` (0x4DA2C0): every entry in state 2 whose end time (+0xC) has
/// passed (`now` read once, unsigned compare) stops its sound (+8, kept non-null check), is cleared
/// (surfaces 0xC3) and the active count (+0x204) drops.
#[hook(addr = 0x4DA2C0, conv = thiscall, name = "CAECollisionAudioEntity::Service", expect = "51 A1 84 CB B7 00")]
pub fn collision_service(this: GPtr<u8>) {
    let now = TIME_MS.get();
    for i in 0..300isize {
        let e = this.byte_add(0x208 + i * 0x14);
        if rd::<u8>(e, 0x10) != 2 || now < rd::<u32>(e, 0xC) {
            continue;
        }
        let snd = rd::<u32>(e, 8);
        if snd != 0 {
            sound_stop_and_forget(g(snd as usize));
        }
        wr::<u32>(e, 0, 0);
        wr::<u32>(e, 4, 0);
        wr::<u8>(e, 0x11, 0xC3);
        wr::<u8>(e, 0x12, 0xC3);
        wr::<u32>(e, 8, 0);
        wr::<u8>(e, 0x10, 0);
        wr::<u32>(this, 0x204, rd::<u32>(this, 0x204).wrapping_sub(1));
    }
}

/// `CAECollisionAudioEntity::AddCollisionSoundToList(CEntity*, CEntity*, uint8 surf1, uint8 surf2,
/// CAESound*, int32 status)` (0x4DAAC0): the first entry without a sound (+8) gets the arguments; status 2
/// ends at now + 100 ms, others 0; the active count (+0x204) grows. Nothing when all 300 are used.
#[hook(addr = 0x4DAAC0, conv = thiscall, name = "CAECollisionAudioEntity::AddCollisionSoundToList", expect = "33 C0 8D 91 10 02 00 00")]
#[allow(clippy::too_many_arguments)]
pub fn collision_add_collision_sound_to_list(
    this: GPtr<u8>,
    e1: u32,
    e2: u32,
    surf1: u32,
    surf2: u32,
    sound: u32,
    status: u32,
) {
    for i in 0..300isize {
        let e = this.byte_add(0x208 + i * 0x14);
        if rd::<u32>(e, 8) != 0 {
            continue;
        }
        wr::<u32>(e, 0, e1);
        wr::<u32>(e, 4, e2);
        wr::<u8>(e, 0x11, surf1 as u8);
        wr::<u8>(e, 0x12, surf2 as u8);
        wr::<u32>(e, 8, sound);
        wr::<u8>(e, 0x10, status as u8);
        let end = if status == 2 {
            TIME_MS.get().wrapping_add(100)
        } else {
            0
        };
        wr::<u32>(e, 0xC, end);
        wr::<u32>(this, 0x204, rd::<u32>(this, 0x204).wrapping_add(1));
        return;
    }
}

/// `CAECollisionAudioEntity::ReportBulletHit(CEntity*, eSurfaceType, CVector&, float angle)` (0x4DBDF0):
/// only with bank 0x1B in slot 3; a vehicle (`[+0x36] & 7 == 2`) replaces the low byte of the surface
/// dword with 0xBC when its subclass (+0x594) is 10, else 0x3F; then plays the bullet hit sound.
#[hook(addr = 0x4DBDF0, conv = thiscall, name = "CAECollisionAudioEntity::ReportBulletHit", expect = "56 6A 03 8B F1")]
pub fn collision_report_bullet_hit(this: GPtr<u8>, entity: GPtr<u8>, surface: u32, posn: u32, angle: u32) {
    if hw_is_sound_bank_loaded(audio_hardware(), 0x1B, 3) == 0 {
        return;
    }
    let mut sf = surface;
    if !entity.is_null() && rd::<u8>(entity, 0x36) & 7 == 2 {
        let b: u32 = if rd::<u32>(entity, 0x594) == 10 {
            0xBC
        } else {
            0x3F
        };
        sf = (sf & 0xFFFF_FF00) | b;
    }
    collision_play_bullet_hit_collision_sound(this, sf, posn, angle);
}

/// `CAECollisionAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4DA370): by the sound's
/// event (+0xC): 2 / 3 / 128 set the speed (+0x1C) to 1.26 / 0.94 / 0.56 once playing (`pos >= 0`) and
/// `now >= _ftol(sound +0x10)`; 1 nothing; any other event, when finished (`pos == -1`), clears the
/// collision entry holding the sound.
#[hook(addr = 0x4DA370, conv = thiscall, name = "CAECollisionAudioEntity::UpdateParameters", expect = "56 8B 74 24 08")]
pub fn collision_update_parameters(this: GPtr<u8>, sound: GPtr<u8>, pos: u32) {
    let p = pos as u16 as i16;
    let speed = match rd::<u32>(sound, 0xC) {
        1 => return,
        2 => 0x3FA1_47AE,
        3 => 0x3F70_A3D7,
        128 => 0x3F0F_5C29,
        _ => {
            if p != -1 {
                return;
            }
            for i in 0..300isize {
                let e = this.byte_add(0x208 + i * 0x14);
                if rd::<u32>(e, 8) != sound.addr() as u32 {
                    continue;
                }
                wr::<u32>(e, 0, 0);
                wr::<u32>(e, 4, 0);
                wr::<u8>(e, 0x11, 0xC3);
                wr::<u8>(e, 0x12, 0xC3);
                wr::<u32>(e, 8, 0);
                wr::<u8>(e, 0x10, 0);
                wr::<u32>(this, 0x204, rd::<u32>(this, 0x204).wrapping_sub(1));
                return;
            }
            return;
        }
    };
    if p < 0 {
        return;
    }
    let start = ftol32(x(rd::<f32>(sound, 0x10))) as u32;
    if TIME_MS.get() < start {
        return;
    }
    wr::<u32>(sound, 0x1C, speed);
}

/// `CAECollisionAudioEntity::ChooseCollisionSoundID(uint8 surface)` (0x4DAA50): the surface's single id,
/// or `GetRandomNumberInRange(min, max)` (the audio RNG, as the exe) redrawn until it differs from the
/// surface's last id (+0x7C + 2 * surface, compared as int16). Returns the full EAX.
#[hook(addr = 0x4DAA50, conv = thiscall, name = "CAECollisionAudioEntity::ChooseCollisionSoundID", expect = "53 56 57 0F B6 7C 24 10")]
pub fn collision_choose_collision_sound_id(this: GPtr<u8>, surface: u32) -> u32 {
    let sf = surface as u8 as usize;
    let row = COLLISION_SOUND_RANGES + (sf << 4);
    let min = g::<i32>(row).read();
    if min == g::<i32>(row + 4).read() {
        return min as u32;
    }
    loop {
        let r = ae_random_int(g::<i32>(row).read(), g::<i32>(row + 4).read());
        if r as u16 != rd::<u16>(this, 0x7C + sf as isize * 2) {
            return r as u32;
        }
    }
}

// ---- CAEDoorAudioEntity ---------------------------------------------------------------------------

/// `CAEDoorAudioEntity::StaticInitialise()` (0x5B9A80): bank 0x33 into slot 0x1F.
#[hook(addr = 0x5B9A80, conv = cdecl, name = "CAEDoorAudioEntity::StaticInitialise", expect = "6A 1F 6A 33 B9 B8 F8 B5 00")]
pub fn door_static_initialise() {
    load_sound_bank(audio_hardware(), 0x33, 0x1F);
}

/// `CAEDoorAudioEntity::Reset()` (0x4DC6B0): clears the two times at +0x7C / +0x80 and cancels its sounds.
#[hook(addr = 0x4DC6B0, conv = thiscall, name = "CAEDoorAudioEntity::Reset", expect = "33 C0 6A 01 89 41 7C")]
pub fn door_reset(this: GPtr<u8>) {
    wr::<u32>(this, 0x7C, 0);
    wr::<u32>(this, 0x80, 0);
    cancel_sounds_owned_by_audio_entity(sound_manager(), this, 1);
}

/// `CAEDoorAudioEntity::AddAudioEvent(eAudioEvents, CPhysical*)` (0x4DC9F0): only event 0xA3, when no
/// such sound plays for `this` and more than 4000 ms passed since the last one (+0x80, unsigned): plays
/// sfx 1 at the physical's position (matrix +0x30 or placement +4) and records the time (`now` read on
/// entry).
#[hook(addr = 0x4DC9F0, conv = thiscall, name = "CAEDoorAudioEntity::AddAudioEvent-1", expect = "81 7C 24 04 A3 00 00 00")]
pub fn door_add_audio_event_1(this: GPtr<u8>, event: u32, physical: GPtr<u8>) {
    let now = TIME_MS.get();
    if event != 0xA3 {
        return;
    }
    if are_sounds_of_this_event_playing_for_this_entity(sound_manager(), 0xA3, this) as u16 != 0 {
        return;
    }
    if now <= rd::<u32>(this, 0x80).wrapping_add(4000) {
        return;
    }
    let m = rd::<u32>(physical, 0x14);
    let pos = if m != 0 {
        m.wrapping_add(0x30)
    } else {
        physical.byte_add(4).addr() as u32
    };
    door_play_door_sound(this, 1, 0xA3, pos, 0, 0x3F80_0000);
    wr::<u32>(this, 0x80, now);
}

/// `CAEDoorAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4DCA60): while a door sound
/// plays (`pos > 0`), an opening (0x9F) / closing (0xA1) sound more than 10 s after the last change
/// (+0x7C, unsigned, `now` read on entry) becomes the "fully open / shut" sound (0xA0 / 0xA2, sfx 2, at
/// the sound's position +0x24, speed 1.0 / 0.79) when that is not yet playing and the moving sound still
/// is (all sounds of `this` cancelled first); the time is recorded and the sound forgotten. Within the
/// 10 s the sound is only forgotten when the end sound already plays.
#[hook(addr = 0x4DCA60, conv = thiscall, name = "CAEDoorAudioEntity::UpdateParameters", expect = "66 83 7C 24 08 00")]
pub fn door_update_parameters(this: GPtr<u8>, sound: GPtr<u8>, pos: u32) {
    let now = TIME_MS.get();
    if pos as u16 as i16 <= 0 {
        return;
    }
    let (end_event, speed) = match rd::<u32>(sound, 0xC) {
        0x9F => (0xA0u32, 0x3F80_0000u32),
        0xA1 => (0xA2, 0x3F4A_3D71),
        _ => return,
    };
    let playing =
        |ev: u32| are_sounds_of_this_event_playing_for_this_entity(sound_manager(), ev, this) as u16 != 0;
    if now <= rd::<u32>(this, 0x7C).wrapping_add(10000) {
        if playing(end_event) {
            sound_stop_and_forget(sound);
        }
        return;
    }
    if !playing(end_event) && playing(end_event - 1) {
        cancel_sounds_owned_by_audio_entity(sound_manager(), this, 1);
        door_play_door_sound(this, 2, end_event, sound.byte_add(0x24).addr() as u32, 0, speed);
    }
    wr::<u32>(this, 0x7C, now);
    sound_stop_and_forget(sound);
}

// ---- CAEFireAudioEntity ---------------------------------------------------------------------------

/// `CAEFireAudioEntity::StaticInitialise()` (0x5B9A90): bank 0x34 into slot 4.
#[hook(addr = 0x5B9A90, conv = cdecl, name = "CAEFireAudioEntity::StaticInitialise", expect = "6A 04 6A 34 B9 B8 F8 B5 00")]
pub fn fire_static_initialise() {
    load_sound_bank(audio_hardware(), 0x34, 4);
}

/// `CAEFireAudioEntity::Initialise(CEntity*)` (0x4DCF20): the fx pointer at +0x84, the two sounds cleared.
#[hook(addr = 0x4DCF20, conv = thiscall, name = "CAEFireAudioEntity::Initialise", expect = "8B 44 24 04 89 81 84 00 00 00")]
pub fn fire_initialise(this: GPtr<u8>, fx: u32) {
    wr::<u32>(this, 0x84, fx);
    wr::<u32>(this, 0x7C, 0);
    wr::<u32>(this, 0x80, 0);
}

/// `CAEFireAudioEntity::Terminate()` (0x4DCF40): cancels its sounds, then clears +0x84.
#[hook(addr = 0x4DCF40, conv = thiscall, name = "CAEFireAudioEntity::Terminate", expect = "56 8B F1 6A 01 56")]
pub fn fire_terminate(this: GPtr<u8>) {
    cancel_sounds_owned_by_audio_entity(sound_manager(), this, 1);
    wr::<u32>(this, 0x84, 0);
}

/// `CAEFireAudioEntity::AddAudioEvent(eAudioEvents, CVector&)` (0x4DD3C0): event 0x89 plays the water
/// sounds when the fx (+0x84) exists and neither sound (+0x7C, +0x80) is active; events 0x8A..=0x8C are
/// ignored; any other event plays the fire sounds when the fx exists and +0x7C is free.
#[hook(addr = 0x4DD3C0, conv = thiscall, name = "CAEFireAudioEntity::AddAudioEvent", expect = "8B 44 24 04 3D 89 00 00 00")]
pub fn fire_add_audio_event(this: GPtr<u8>, event: i32, posn: u32) {
    if event == 0x89 {
        if rd::<u32>(this, 0x84) != 0 && rd::<u32>(this, 0x7C) == 0 && rd::<u32>(this, 0x80) == 0 {
            fire_play_water_sounds(this, 0x89, posn);
        }
        return;
    }
    if event > 0x89 && event <= 0x8C {
        return;
    }
    if rd::<u32>(this, 0x84) != 0 && rd::<u32>(this, 0x7C) == 0 {
        fire_play_fire_sounds(this, event as u32, posn);
    }
}

// ---- CAEExplosionAudioEntity ----------------------------------------------------------------------

/// `CAEExplosionAudioEntity::StaticInitialise()` (0x5B9A60): bank 0x34 into slot 4.
#[hook(addr = 0x5B9A60, conv = cdecl, name = "CAEExplosionAudioEntity::StaticInitialise", expect = "6A 04 6A 34 B9 B8 F8 B5 00")]
pub fn explosion_static_initialise() {
    load_sound_bank(audio_hardware(), 0x34, 4);
}

/// `CAEExplosionAudioEntity::CAEExplosionAudioEntity()` (0x5074B0): entity = 0, vtable 0x862E60,
/// +0x7C = 0; returns `this`.
#[hook(addr = 0x5074B0, conv = thiscall, name = "CAEExplosionAudioEntity::CAEExplosionAudioEntity", expect = "8B C1 33 C9 89 48 10")]
pub fn explosion_constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0, 0x862E60);
    wr::<u8>(this, 0x7C, 0);
    this
}

/// `CAEExplosionAudioEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4DCB90): while playing
/// (`pos > 0`) a positive volume decays by 1.0 per call, clamped at 0.0.
#[hook(addr = 0x4DCB90, conv = thiscall, name = "CAEExplosionAudioEntity::UpdateParameters", expect = "66 83 7C 24 08 00")]
pub fn explosion_update_parameters(_this: GPtr<u8>, sound: GPtr<u8>, pos: u32) {
    if pos as u16 as i16 <= 0 {
        return;
    }
    let zero = g::<f32>(FLT_0).read();
    let v = x(rd::<f32>(sound, 0x14));
    if v.fcom(x(zero)).test_any(0x41) {
        return;
    }
    let t = v.fsub_m(g::<f32>(FLT_1).read());
    let r = if t.fcom(x(zero)).test_any(0x41) {
        zero
    } else {
        t.to_f32()
    };
    wr::<f32>(sound, 0x14, r);
}

// ---- CAEWeatherAudioEntity ------------------------------------------------------------------------

/// `CAEWeatherAudioEntity::StaticInitialise()` (0x5B9A70): bank 0x69 into slot 6.
#[hook(addr = 0x5B9A70, conv = cdecl, name = "CAEWeatherAudioEntity::StaticInitialise", expect = "6A 06 6A 69 B9 B8 F8 B5 00")]
pub fn weather_static_initialise() {
    load_sound_bank(audio_hardware(), 0x69, 6);
}

/// `CAEWeatherAudioEntity::CAEWeatherAudioEntity()` (0x72A620): +0x10 = 0, vtable 0x872A74, +0x7C = 0;
/// returns `this`.
#[hook(addr = 0x72A620, conv = thiscall, name = "CAEWeatherAudioEntity::CAEWeatherAudioEntity", expect = "8B C1 33 C9 89 48 10")]
pub fn weather_constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0, 0x872A74);
    wr::<u8>(this, 0x7C, 0);
    this
}

// ---- CAEScriptAudioEntity -------------------------------------------------------------------------

/// `CAEScriptAudioEntity::IsMissionAudioSampleFinished(uint8 slot)` (0x4EBFE0): slots >= 4 are finished;
/// slots 2/3 when nothing plays in bank slot `slot + 0x1A`; slots 0/1 when the dword at
/// `this + (slot + 5) * 32` is 0.
#[hook(addr = 0x4EBFE0, conv = thiscall, name = "CAEScriptAudioEntity::IsMissionAudioSampleFinished", expect = "8A 44 24 04 3C 04")]
pub fn script_is_mission_audio_sample_finished(this: GPtr<u8>, slot: u32) -> u8 {
    let s = slot as u8;
    if s >= 4 {
        return 1;
    }
    if s >= 2 {
        let r = are_sounds_playing_in_bank_slot(g(AE_SOUND_MANAGER), s as u32 + 0x1A);
        return (r as u16 == 0) as u8;
    }
    (rd::<u32>(this, (s as isize + 5) << 5) == 0) as u8
}

/// `CAEScriptAudioEntity::Initialise()` (0x5B9B60): the four mission-audio slots (0x20 bytes from
/// +0x90): position = (-1000, -1000, -1000), two cleared dwords, three ids = -1.
#[hook(addr = 0x5B9B60, conv = thiscall, name = "CAEScriptAudioEntity::Initialise", expect = "83 EC 0C 53 55")]
pub fn script_initialise(this: GPtr<u8>) {
    const MINUS_1000: u32 = 0xC47A_0000;
    for i in 0..4isize {
        let b = this.byte_add(0x90 + i * 0x20);
        wr::<u32>(b, 0x14, 0xFFFF_FFFF);
        wr::<u32>(b, 0x18, 0xFFFF_FFFF);
        wr::<u32>(b, 0x1C, 0xFFFF_FFFF);
        wr::<u32>(b, 0, MINUS_1000);
        wr::<u32>(b, 0xC, 0);
        wr::<u32>(b, 0x10, 0);
        wr::<u32>(b, 4, MINUS_1000);
        wr::<u32>(b, 8, MINUS_1000);
    }
}

/// `CAEScriptAudioEntity::CAEScriptAudioEntity()` (0x5074D0): vtables (0x862E58; the embedded entities
/// at +0x104 / +0x18C get 0x85998C / 0x862E60), their entity pointers and the script state cleared,
/// +0x88 = 1.0; returns `this`.
#[hook(addr = 0x5074D0, conv = thiscall, name = "CAEScriptAudioEntity::Constructor", expect = "8B C1 33 C9 89 48 10")]
pub fn script_constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0, 0x862E58);
    wr::<u32>(this, 0x124, 0);
    wr::<u32>(this, 0x114, 0x85998C);
    wr::<u32>(this, 0x190, 0);
    wr::<u32>(this, 0x194, 0);
    wr::<u32>(this, 0x198, 0);
    wr::<u32>(this, 0x1AC, 0);
    wr::<u32>(this, 0x19C, 0x862E60);
    wr::<u8>(this, 0x218, 0);
    wr::<u32>(this, 0x80, 0);
    wr::<u8>(this, 0x7E, 0);
    wr::<u8>(this, 0x7C, 0);
    wr::<u32>(this, 0x84, 0);
    wr::<u32>(this, 0x88, 0x3F80_0000);
    wr::<u32>(this, 0x110, 0);
    wr::<u8>(this, 0x7D, 0);
    wr::<u32>(this, 0x8C, 0);
    this
}

/// `CAEScriptAudioEntity::ReportMissionAudioEvent(eAudioEvents, CPhysical*)` (0x4EE940): forwards to
/// `ProcessMissionAudioEvent(event, physical, 0, 0, 1.0)`.
#[hook(addr = 0x4EE940, conv = thiscall, name = "CAEScriptAudioEntity::ReportMissionAudioEvent-2", expect = "8B 44 24 08 8B 54 24 04")]
pub fn script_report_mission_audio_event_2(this: GPtr<u8>, event: u32, a2: u32) {
    script_process_mission_audio_event(this, event, a2, 0, 0, 0x3F80_0000);
}

// ---- CAEFrontendAudioEntity -----------------------------------------------------------------------

/// `CAEFrontendAudioEntity::IsLoadingTuneActive()` (0x4DD470): sounds of event 0x32 playing for `this`
/// (the sound manager's EAX is returned unchanged).
#[hook(addr = 0x4DD470, conv = thiscall, name = "CAEFrontendAudioEntity::IsLoadingTuneActive", expect = "51 6A 32 B9 B0 2C B6 00")]
pub fn frontend_is_loading_tune_active(this: GPtr<u8>) -> u32 {
    are_sounds_of_this_event_playing_for_this_entity(sound_manager(), 0x32, this)
}

// ---- CAEPoliceScannerAudioEntity ------------------------------------------------------------------

/// `CAEPoliceScannerAudioEntity::StaticInitialise()` (0x5B9C30): every static cleared.
#[hook(addr = 0x5B9C30, conv = cdecl, name = "CAEPoliceScannerAudioEntity::StaticInitialise", expect = "33 C0 A3 50 1D B6 00")]
pub fn police_scanner_static_initialise() {
    PS_NEXT_DIALOGUE_TIME.set(0);
    PS_SCANNER_DISABLED.set(0);
    PS_PLAYBACK_STATE.set(0);
    PS_CONTROLLING.set(0);
    PS_CURRENT_SLOTS.set(0);
    PS_VOLUME_OFFSET.set(0);
}

/// `CAEPoliceScannerAudioEntity::StopScanner(bool instantly)` (0x4E6DC0, static): only while a dialogue
/// plays; sets `s_bStoppingScanner`, and when `instantly` stops the sound and finishes the dialogue.
#[hook(addr = 0x4E6DC0, conv = cdecl, name = "CAEPoliceScannerAudioEntity::StopScanner", expect = "66 83 3D 4C 1D B6 00 00")]
pub fn police_scanner_stop_scanner(instantly: u32) {
    if PS_PLAYBACK_STATE.get() == 0 {
        return;
    }
    PS_STOPPING_SCANNER.set(1);
    if instantly as u8 == 0 {
        return;
    }
    let s = PS_SOUND.get();
    if s != 0 {
        sound_stop_and_forget(g(s as usize));
        PS_SOUND.set(0);
    }
    ps_finished_playing_scanner_dialogue();
}

// ---- CAETwinLoopSoundEntity -----------------------------------------------------------------------

/// `CAETwinLoopSoundEntity::StopSoundAndForget()` (0x4F2960): both sounds (+0xA0, +0xA4), then the bank
/// slot word at +0x88 cleared.
#[hook(addr = 0x4F2960, conv = thiscall, name = "CAETwinLoopSoundEntity::StopSoundAndForget", expect = "56 8B F1 8B 8E A0 00 00 00")]
pub fn twin_loop_stop_sound_and_forget(this: GPtr<u8>) {
    stop_sound_field(this, 0xA0);
    stop_sound_field(this, 0xA4);
    wr::<u16>(this, 0x88, 0);
}

/// `CAETwinLoopSoundEntity::UpdateParameters(CAESound*, int16 curPlayPos)` (0x4F2E90): a finished sound
/// (`pos == -1`) is forgotten; while the first sound (+0xA0) plays past the swap time (+0x94, against
/// `CTimer::m_snTimeInMillisecondsPauseMode`) the two sounds swap.
#[hook(addr = 0x4F2E90, conv = thiscall, name = "CAETwinLoopSoundEntity::UpdateParameters", expect = "66 83 7C 24 08 FF")]
pub fn twin_loop_update_parameters(this: GPtr<u8>, sound: u32, pos: u32) {
    if pos as u16 == 0xFFFF {
        if sound == rd::<u32>(this, 0xA0) {
            wr::<u32>(this, 0xA0, 0);
        }
        if sound == rd::<u32>(this, 0xA4) {
            wr::<u32>(this, 0xA4, 0);
        }
    }
    if sound == rd::<u32>(this, 0xA0) && TIME_MS_PAUSE_MODE.get() > rd::<u32>(this, 0x94) {
        twin_loop_swap_sounds_call(this);
    }
}

/// `CAETwinLoopSoundEntity::Initialise(int16 bank, int16 sfx1, int16 sfx2, CAEAudioEntity* parent,
/// uint16 swapMin, uint16 swapMax, int16 playPercentage, int16 swapPercentage)` (0x4F28A0): stores the
/// arguments, no sounds, no previous sfx (-1), first sound flag +0x98 = 1, bank slot word +0x88 = 1.
#[hook(addr = 0x4F28A0, conv = thiscall, name = "CAETwinLoopSoundEntity::Initialise", expect = "66 8B 44 24 04")]
#[allow(clippy::too_many_arguments)]
pub fn twin_loop_initialise(
    this: GPtr<u8>,
    bank: u32,
    sfx1: u32,
    sfx2: u32,
    parent: u32,
    a5: u32,
    a6: u32,
    a7: u32,
    a8: u32,
) {
    wr::<u16>(this, 0x7C, bank as u16);
    wr::<u16>(this, 0x80, sfx2 as u16);
    wr::<u16>(this, 0x8E, a5 as u16);
    wr::<u16>(this, 0x7E, sfx1 as u16);
    wr::<u16>(this, 0x8A, 0xFFFF);
    wr::<u16>(this, 0x8C, 0xFFFF);
    wr::<u32>(this, 0x84, parent);
    wr::<u32>(this, 0xA0, 0);
    wr::<u32>(this, 0xA4, 0);
    wr::<u16>(this, 0x90, a6 as u16);
    wr::<u16>(this, 0x9A, a7 as u16);
    wr::<u8>(this, 0x98, 1);
    wr::<u16>(this, 0x9C, a8 as u16);
    wr::<u16>(this, 0x88, 1);
}

/// `CAETwinLoopSoundEntity::SwapSounds()` (0x4F2C10): with both sounds alive, the playing one's volume
/// moves to the other and it is muted (-100.0), the "first is playing" flag (+0x98) flips, and the next
/// swap time = `CTimer::m_snTimeInMillisecondsPauseMode + GetRandomNumberInRange(min +0x8E, max +0x90)`.
#[hook(addr = 0x4F2C10, conv = thiscall, name = "CAETwinLoopSoundEntity::SwapSounds", expect = "56 8B F1 8B 8E A0 00 00 00")]
pub fn twin_loop_swap_sounds(this: GPtr<u8>) {
    let s1 = rd::<u32>(this, 0xA0);
    let s2 = rd::<u32>(this, 0xA4);
    if s1 == 0 || s2 == 0 {
        return;
    }
    let (p1, p2): (GPtr<u8>, GPtr<u8>) = (g(s1 as usize), g(s2 as usize));
    if rd::<u8>(this, 0x98) != 0 {
        wr::<f32>(p2, 0x14, x(rd::<f32>(p1, 0x14)).to_f32());
        wr::<u32>(g::<u8>(rd::<u32>(this, 0xA0) as usize), 0x14, 0xC2C8_0000);
        wr::<u8>(this, 0x98, 0);
    } else {
        wr::<f32>(p1, 0x14, x(rd::<f32>(p2, 0x14)).to_f32());
        wr::<u32>(g::<u8>(rd::<u32>(this, 0xA4) as usize), 0x14, 0xC2C8_0000);
        wr::<u8>(this, 0x98, 1);
    }
    let max = rd::<u16>(this, 0x90) as i32;
    let min = rd::<u16>(this, 0x8E) as i32;
    let now = TIME_MS_PAUSE_MODE.get();
    let r = ae_random_int(min, max);
    wr::<u32>(this, 0x94, (r as u32).wrapping_add(now));
}

/// `CAETwinLoopSoundEntity::~CAETwinLoopSoundEntity()` (0x4F2B80): stops both sounds when initialised
/// (+0x88), then the base part (vtable 0x859988) destroys its embedded `CAESound` (+8). Void, as the C++
/// destructor (the original's EAX after its SEH epilogue is not the callee's; no caller reads it).
#[hook(addr = 0x4F2B80, conv = thiscall, name = "CAETwinLoopSoundEntity::~CAETwinLoopSoundEntity", expect = "6A FF 68 58 C1 83 00")]
pub fn twin_loop_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85F438);
    if rd::<u16>(this, 0x88) != 0 {
        stop_sound_field(this, 0xA0);
        stop_sound_field(this, 0xA4);
        wr::<u16>(this, 0x88, 0);
    }
    wr::<u32>(this, 0, 0x859988);
    sound_destructor(this.byte_add(8));
}

/// `CAETwinLoopSoundEntity::UpdateTwinLoopSound(CVector posn, float volume, float speed)` (0x4F29A0):
/// both sounds get the position and speed (+0x1C); only the audible one (flag +0x98: first, else second)
/// gets the volume (+0x14, raw copy).
#[hook(addr = 0x4F29A0, conv = thiscall, name = "CAETwinLoopSoundEntity::UpdateTwinLoopSound", expect = "53 8B 5C 24 0C")]
pub fn twin_loop_update_twin_loop_sound(this: GPtr<u8>, px: u32, py: u32, pz: u32, volume: u32, speed: u32) {
    let s1 = rd::<u32>(this, 0xA0);
    if s1 != 0 {
        sound_set_position(g(s1 as usize), px, py, pz);
        if rd::<u8>(this, 0x98) != 0 {
            wr::<u32>(g::<u8>(rd::<u32>(this, 0xA0) as usize), 0x14, volume);
        }
        wr::<u32>(g::<u8>(rd::<u32>(this, 0xA0) as usize), 0x1C, speed);
    }
    let s2 = rd::<u32>(this, 0xA4);
    if s2 != 0 {
        sound_set_position(g(s2 as usize), px, py, pz);
        if rd::<u8>(this, 0x98) == 0 {
            wr::<u32>(g::<u8>(rd::<u32>(this, 0xA4) as usize), 0x14, volume);
        }
        wr::<u32>(g::<u8>(rd::<u32>(this, 0xA4) as usize), 0x1C, speed);
    }
}

// ---- CAEWaterCannonAudioEntity --------------------------------------------------------------------

/// `CAEWaterCannonAudioEntity::Initialise(CEntity*)` (0x503060): entity at +4, the three sounds
/// (+0x80..+0x88), +0x7D and +0x8C cleared, initialised flag +0x7C = 1.
#[hook(addr = 0x503060, conv = thiscall, name = "CAEWaterCannonAudioEntity::Initialise", expect = "83 EC 0C 8B 44 24 10")]
pub fn water_cannon_initialise(this: GPtr<u8>, entity: u32) {
    wr::<u32>(this, 4, entity);
    wr::<u32>(this, 0x80, 0);
    wr::<u32>(this, 0x84, 0);
    wr::<u32>(this, 0x88, 0);
    wr::<u8>(this, 0x7D, 0);
    wr::<u32>(this, 0x8C, 0);
    wr::<u8>(this, 0x7C, 1);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_WEAPON_INITIALISE,
    HOOK_WEAPON_TERMINATE,
    HOOK_WEAPON_STOP_FLAME_THROWER_IDLE_GAS_LOOP,
    HOOK_GLOBAL_WEAPON_ADD_AUDIO_EVENT,
    HOOK_COLLISION_INITIALISE_POST_LOADING,
    HOOK_COLLISION_INITIALISE,
    HOOK_COLLISION_SERVICE,
    HOOK_COLLISION_UPDATE_PARAMETERS,
    HOOK_COLLISION_REPORT_BULLET_HIT,
    HOOK_COLLISION_ADD_COLLISION_SOUND_TO_LIST,
    HOOK_COLLISION_CHOOSE_COLLISION_SOUND_ID,
    HOOK_DOOR_STATIC_INITIALISE,
    HOOK_DOOR_RESET,
    HOOK_DOOR_ADD_AUDIO_EVENT_1,
    HOOK_DOOR_UPDATE_PARAMETERS,
    HOOK_FIRE_STATIC_INITIALISE,
    HOOK_FIRE_INITIALISE,
    HOOK_FIRE_TERMINATE,
    HOOK_FIRE_ADD_AUDIO_EVENT,
    HOOK_EXPLOSION_STATIC_INITIALISE,
    HOOK_EXPLOSION_CONSTRUCTOR,
    HOOK_EXPLOSION_UPDATE_PARAMETERS,
    HOOK_WEATHER_STATIC_INITIALISE,
    HOOK_WEATHER_CONSTRUCTOR,
    HOOK_SCRIPT_IS_MISSION_AUDIO_SAMPLE_FINISHED,
    HOOK_SCRIPT_INITIALISE,
    HOOK_SCRIPT_CONSTRUCTOR,
    HOOK_SCRIPT_REPORT_MISSION_AUDIO_EVENT_2,
    HOOK_FRONTEND_IS_LOADING_TUNE_ACTIVE,
    HOOK_POLICE_SCANNER_STATIC_INITIALISE,
    HOOK_POLICE_SCANNER_STOP_SCANNER,
    HOOK_TWIN_LOOP_STOP_SOUND_AND_FORGET,
    HOOK_TWIN_LOOP_INITIALISE,
    HOOK_TWIN_LOOP_SWAP_SOUNDS,
    HOOK_TWIN_LOOP_DESTRUCTOR,
    HOOK_TWIN_LOOP_UPDATE_TWIN_LOOP_SOUND,
    HOOK_TWIN_LOOP_UPDATE_PARAMETERS,
    HOOK_WATER_CANNON_INITIALISE,
];
