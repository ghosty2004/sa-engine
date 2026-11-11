//! STD-4 audio lane, part 3: the per-frame service: `CAudioEngine::Service` (0x507750),
//! `CAESoundManager::Service` (0x4F0000), `CAEAudioHardware::Service` / `PlaySound` /
//! `SetChannelVolume` / `UpdateReverbEnvironment`, `CAEMP3BankLoader::Service` (bank loading
//! through CdStream), `CAESoundManager::RequestNewSound`, `CAEFrontendAudioEntity::AddAudioEvent`
//! (0x4DD4A0) and `CAETwinLoopSoundEntity::PlayTwinLoopSound`. Ported from the 1.0 US bytes
//! (gta-reversed as reading aid; the bytes win).
//!
//! Stack `CAESound`s: the original builds them in an uninitialised stack frame (only the physical
//! entity +8 is cleared); the ports start from a zeroed one (PORT-AUDIO-2's open item: the
//! unwritten bytes are copied into `AESoundManager`).

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::port4_hw::{ftol, p, r16, r32, r8, rf, w16, w32, w8, xf, xi};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

const HW: u32 = 0xB5F8B8;
const SM: u32 = 0xB62CB0;
/// `CTimer::m_snTimeInMilliseconds`, `m_snTimeInMillisecondsPauseMode`, `m_FrameCounter`.
const TIME_MS: u32 = 0xB7CB84;
const TIME_PAUSE_MS: u32 = 0xB7CB7C;
const FRAME_COUNTER: u32 = 0xB7CB4C;
/// `CTimer::m_UserPause` / `m_CodePause`.
fn game_paused() -> bool {
    r8(0xB7CB49) != 0 || r8(0xB7CB48) != 0
}

orig! {
    // CAEAudioHardware / channels
    fn hw_get_track_play_time(this: u32) -> u32 = thiscall 0x4D8F60 => unported;
    fn hw_get_channel_play_times(this: u32, slot: u32, out: u32) = thiscall 0x4D8820 => unported;
    fn hw_get_virtual_channel_sound_lengths(this: u32, out: u32) = thiscall 0x4D8E90 => unported;
    fn hw_get_virtual_channel_sound_loop_start_times(this: u32, out: u32) = thiscall 0x4D8EB0 => unported;
    fn hw_stop_sound(this: u32, slot: u32, ch: u32) = thiscall 0x4D88E0 => unported;
    fn hw_play_sound(this: u32, slot: u32, ch: u32, sfx: u32, bank: u32, pt: u32, flags: u32, speed: u32) = thiscall 0x4D86B0 => audio_hardware_play_sound;
    fn hw_set_channel_volume(this: u32, slot: u32, ch: u32, vol: u32, a: u32) = thiscall 0x4D8870 => audio_hardware_set_channel_volume;
    fn hw_set_channel_position(this: u32, slot: u32, ch: u32, pos: u32, a: u32) = thiscall 0x4D8920 => unported;
    fn hw_set_channel_frequency_scaling_factor(this: u32, slot: u32, ch: u32, f: u32) = thiscall 0x4D8960 => unported;
    fn hw_service(this: u32) = thiscall 0x4D9870 => audio_hardware_service;
    fn hw_update_reverb_environment(this: u32) = thiscall 0x4D8DA0 => audio_hardware_update_reverb_environment;
    fn hw_rescale_channel_volumes(this: u32) = thiscall 0x4D8990 => unported;
    fn hw_is_sound_bank_loaded(this: u32, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    fn hw_is_sound_loaded(this: u32, bank: u32, sfx: u32, slot: u32) -> u8 = thiscall 0x4D8EF0 => unported;
    fn hw_load_sound_bank(this: u32, bank: u32, slot: u32) = thiscall 0x4D88A0 => unported;
    fn hw_set_music_fader(this: u32, f: u32) = thiscall 0x4D9550 => unported;
    fn hw_set_effects_fader(this: u32, f: u32) = thiscall 0x4D9560 => unported;
    fn hw_set_stream_fader(this: u32, f: u32) = thiscall 0x4D95B0 => unported;
    fn hw_request_virtual_channel_sound_info(this: u32, idx: u32, sfx: u32, bank: u32) = thiscall 0x4D8E60 => unported;
    fn ch_synch_playback_raw(ch: GPtr<u8>) = thiscall vslot 0x18 => unported;
    fn ch_service_raw(ch: GPtr<u8>) = thiscall vslot 0x04 => unported;
    fn ch_get_length_raw(ch: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    fn ch_play_raw(ch: GPtr<u8>, start: u32, flags: u32, speed: u32) = thiscall vslot 0x14 => unported;
    fn ch_set_audio_buffer(ch: u32, data: u32, size: u32, id0: u32, id1: u32, lo: u32, rate: u32) -> u8 = thiscall 0x4F0C40 => unported;
    fn ch_set_reverb_and_depth(ch: u32, env: u32, depth: u32) -> u8 = thiscall 0x4D7AA0 => unported;
    fn ch_set_not_in_room(ch: u32, v: u32) = thiscall 0x4D7B50 => unported;
    fn li_commit_raw(l: GPtr<u8>) -> i32 = stdcall vslot 0x44 => unported;
    fn is_foreground_app() -> u8 = cdecl 0x746060 => unported;
    fn bank_get_sound_buffer(this: u32, sfx: u32, bank: u32, size: u32, rate: u32) -> u32 = thiscall 0x4E0280 => unported;
    fn bank_get_loop_offset(this: u32, sfx: u32, bank: u32) -> u32 = thiscall 0x4E0380 => unported;
    fn bank_update_virtual_channels(this: u32, a: u32, b: u32, c: u32) = thiscall 0x4E0450 => unported;
    fn bank_service(this: u32) = thiscall 0x4DFE30 => mp3_bank_loader_service;
    fn env_get_reverb(env: u32, depth: u32) = cdecl 0x4D8010 => unported;
    fn env_get_doppler(a: u32, b: u32, c: u32, d: u32, e: u32) -> f64 = cdecl 0x4D7E40 => unported;
    fn env_get_position_relative_to_camera(out: u32, pos: u32) = cdecl 0x4D80B0 => unported;
    fn floor_f(v: u32) -> f64 = cdecl 0x406CE0 => floor_of_float;

    // CAESound / CAESoundManager
    fn sound_initialise(s: u32, slot: u32, sfx: u32, ent: u32, x: u32, y: u32, z: u32, vol: u32, maxd: u32, speed: u32, ts: u32, ign: u32, flags: u32, var: u32, pp: u32) = thiscall 0x4EFE50 => unported;
    fn sound_dtor(s: u32) = thiscall 0x4EF660 => unported;
    fn sound_assign(s: u32, src: u32) -> u32 = thiscall 0x4EF680 => unported;
    fn sound_new_vpsl_entry(s: u32) = thiscall 0x4EF7A0 => unported;
    fn sound_set_position(s: u32, x: u32, y: u32, z: u32) = thiscall 0x4EF880 => sound_set_position_native;
    fn sound_update_parameters(s: u32, pt: u32) = thiscall 0x4EFF50 => unported;
    fn sound_calculate_volume(s: u32) = thiscall 0x4EFA10 => unported;
    fn sound_stop_and_forget(s: u32) = thiscall 0x4EF850 => unported;
    fn entity_update_parameters_raw(ent: GPtr<u8>, s: u32, pt: u32) = thiscall vslot 0x00 => unported;
    fn sm_request_new_sound(this: u32, s: u32) -> u32 = thiscall 0x4EFB10 => sound_manager_request_new_sound;
    fn sm_service(this: u32) = thiscall 0x4F0000 => sound_manager_service;
    fn sm_are_sounds_playing_in_bank_slot(this: u32, slot: u32) -> u16 = thiscall 0x4EF520 => unported;
    fn sm_are_sounds_of_event(this: u32, ev: u32, ent: u32) -> u16 = thiscall 0x4EF570 => unported;
    fn sm_cancel_sounds_of_event(this: u32, ev: u32, ent: u32) = thiscall 0x4EFB90 => unported;
    fn clean_up_old_reference(ent: u32, slot: u32) = thiscall 0x571A00 => unported;
    fn twin_initialise(t: u32, slot: u32, s1: u32, s2: u32, ent: u32, a: u32, b: u32, c: u32, d: u32) = thiscall 0x4F28A0 => unported;
    fn twin_play(t: u32, x: u32, y: u32, z: u32, vol: u32, a: u32, b: u32, c: u32, flags: u32) = thiscall 0x4F2CB0 => twin_loop_play_twin_loop_sound;
    fn twin_stop_and_forget(t: u32) = thiscall 0x4F2960 => unported;

    // other subsystems
    fn frontend_add_audio_event_call(this: u32, ev: u32, vol: u32, speed: u32) = thiscall 0x4DD4A0 => frontend_add_audio_event;
    fn radio_is_vehicle_radio_active(this: u32) -> u8 = thiscall 0x4E9800 => unported;
    fn radio_start_radio(this: u32, settings: u32) = thiscall 0x4EB550 => unported;
    fn radio_service(this: u32, pt: u32) = thiscall 0x4EB9A0 => unported;
    fn cutscene_service(this: u32, pt: u32) = thiscall 0x4DBFB0 => unported;
    fn ambience_service(this: u32, pt: u32) = thiscall 0x4D76C0 => unported;
    fn vehicle_player_settings_for_radio() -> u32 = cdecl 0x4F4ED0 => vehicle_get_player_settings_for_radio;
    fn find_player_ped(i: i32) -> u32 = cdecl 0x56E210 => unported;
    fn global_weapon_service_ambient_gun_fire(this: u32) = thiscall 0x4DF210 => unported;
    fn collision_service(this: u32) = thiscall 0x4DA2C0 => unported;
    fn script_service(this: u32) = thiscall 0x4EC900 => unported;
    fn ped_speech_service() = cdecl 0x4E3710 => unported;
    fn police_scanner_service() = cdecl 0x4E7630 => unported;
    fn timer_is_slow_motion_active() -> u8 = cdecl 0x561AD0 => unported;
    fn ae_random_int(a: i32, b: i32) -> i32 = cdecl 0x4D9C10 => unported;
    fn ae_resolve_probability(p: u32) -> u8 = cdecl 0x4D9C80 => unported;
    fn cd_stream_get_status(ch: i32) -> i32 = cdecl 0x4063E0 => unported;
    fn cd_stream_read(ch: i32, buf: u32, off: u32, n: u32) -> i32 = cdecl 0x406A20 => unported;
    fn memory_malloc(n: u32, hint: u32) -> u32 = cdecl 0x72F420 => unported;
    fn memory_free(p: u32) = cdecl 0x72F430 => unported;
}

fn local(v: &mut u32) -> u32 {
    GPtr::<u32>::from_raw(v).addr() as u32
}
fn local_arr<const N: usize>(a: &mut [u32; N]) -> u32 {
    GPtr::<u32>::from_raw(a.as_mut_ptr()).addr() as u32
}
fn copy_bytes(dst: u32, src: u32, n: u32) {
    for i in 0..n {
        w8(dst.wrapping_add(i), r8(src.wrapping_add(i)));
    }
}

// ------------------------------------------------------------------ small natives ----------------

/// `CAESound::SetPosition(CVector)` (0x4EF880, thiscall, the vector by value, `ret 0xC`): the
/// PORT-AUDIO-1 port ([`crate::sound::set_position`], unhooked because caller 0x4EFDE4 reads ECX
/// after the call), registered for phase B.
#[sa_core::native(addr = 0x4EF880, conv = thiscall, name = "CAESound::SetPosition")]
pub fn sound_set_position_native(this: u32, x: u32, y: u32, z: u32) {
    crate::sound::set_position(p(this), x, y, z);
}

/// `floor` of a float (0x406CE0): `floor((double)v)` through the CRT (0x8219F0), result in ST0.
#[hook(addr = 0x406CE0, conv = cdecl, name = "FUN_00406ce0", expect = "D9 44 24 04 83 EC 08",
       unverified = "std4-audio-3", route = "L2", hazards = "none")]
pub fn floor_of_float(v: u32) -> f64 {
    libm::floor(f32::from_bits(v) as f64)
}

/// `CAEVehicleAudioEntity::StaticGetPlayerVehicleAudioSettingsForRadio()` (0x4F4ED0): the entry
/// jumps into the protection chunk 0x405B0E, which returns `*(~[0x15607EC] ^ [0x727253])` (a
/// `.HOODLUM` dword and a `.text` dword: the address of the player's vehicle settings). `#[native]`.
#[sa_core::native(addr = 0x4F4ED0, conv = cdecl, name = "CAEVehicleAudioEntity::StaticGetPlayerVehicleAudioSettingsForRadio")]
pub fn vehicle_get_player_settings_for_radio() -> u32 {
    r32(!r32(0x15607EC) ^ r32(0x727253))
}

/// `CAEAudioHardware::SetChannelVolume(int16 slot, uint16 channel, float volume, int8)` (0x4D8870,
/// entry `jmp` into the `.text` chunk 0x402182, `ret 0x10`): a valid slot / channel stores the
/// volume at `+0x110 + (slot + channel) * 4`. `#[native]` (the entry is a jump).
#[sa_core::native(addr = 0x4D8870, conv = thiscall, name = "CAEAudioHardware::SetChannelVolume")]
pub fn audio_hardware_set_channel_volume(this: u32, slot: u32, ch: u32, vol: u32, _a: u32) {
    let s = slot as u16 as i16;
    if s < 0 {
        return;
    }
    let c = ch as u16;
    if c >= r16(this + 0x90 + (s as u32) * 2) {
        return;
    }
    w32(this + 0x110 + (c as u32 + s as u32) * 4, vol);
}

// ------------------------------------------------------------------ CAEAudioHardware -------------

/// `CAEAudioHardware::Service()` (0x4D9870): `IsForegroundApp` (result unused); the listener's
/// deferred settings committed; channel volumes rescaled; the reverb environment updated when the
/// 3D effects were queried (+0xDA8); every static channel's `SynchPlayback`, then its `Service`; the
/// bank loader's virtual channels and `Service`. `#[native]`: phase A's determinism mode hooks
/// this address.
#[sa_core::native(addr = 0x4D9870, conv = thiscall, name = "CAEAudioHardware::Service")]
pub fn audio_hardware_service(this: u32) {
    is_foreground_app();
    li_commit(r32(this + 0xE0C));
    hw_rescale_channel_volumes(this);
    if r8(this + 0xDA8) != 0 {
        hw_update_reverb_environment(this);
    }
    let mut i = 1u32;
    while i < r16(this + 0x8E) as u32 {
        ch_synch_playback(r32(this + 0xE68 + (i - 1) * 4));
        i += 1;
    }
    let mut i = 1u32;
    while i < r16(this + 0x8E) as u32 {
        ch_service(r32(this + 0xE68 + (i - 1) * 4));
        i += 1;
    }
    bank_update_virtual_channels(r32(this + 0xD98), this + 0x430, this + 0xB38, this + 0x8E0);
    bank_service(r32(this + 0xD98));
}

/// `CAEAudioHardware::UpdateReverbEnvironment()` (0x4D8DA0): the environment and depth
/// (`CAEAudioEnvironment::GetReverbEnvironmentAndDepth`); unchanged (+5 byte, +0x88) -> nothing;
/// else the first channel that accepts `SetReverbAndDepth` makes them current.
#[hook(addr = 0x4D8DA0, conv = thiscall, name = "CAEAudioHardware::UpdateReverbEnvironment", expect = "83 EC 08 56 8D 44 24 08",
       unverified = "std4-audio-3", route = "L2", hazards = "uninit-locals", this = "CAEAudioHardware",
       note = "the environment local's upper bytes are uninitialised in the original")]
pub fn audio_hardware_update_reverb_environment(this: u32) {
    let mut env: u32 = 0;
    let mut depth: u32 = 0;
    env_get_reverb(local(&mut env), local(&mut depth));
    if env as u8 == r8(this + 5) && depth == r32(this + 0x88) {
        return;
    }
    let mut i = 0u32;
    while i < r16(this + 0x8E) as u32 {
        let ch = r32(this + 0xE64 + i * 4);
        if ch != 0 && ch_set_reverb_and_depth(ch, env, depth) != 0 {
            w8(this + 5, env as u8);
            w32(this + 0x88, depth);
            return;
        }
        i += 1;
    }
}

/// `CAEAudioHardware::PlaySound(int16 slot, uint16 channel, uint16 sfx, uint16 bank, int16 playPos,
/// int16 flags, float speed)` (0x4D86B0, `ret 0x1C`). A valid slot / channel: the sound's buffer
/// from the bank loader (size and rate into the argument slots; none: return), its loop offset,
/// the channel `slot + channel` (at most 64; accepted as the bytes do) gets the buffer
/// (`SetAudioBuffer`; failure: return). Play position: negative -> 0; with flag 8 a percentage of
/// the length (`ftol(floor(pos * 0.01 * length))`); at most the length. With EAX, `SetNotInRoom(!(flags
/// & 0x200))`. `Play(pos, flags, speed)`; the channel's flags (+6 + index*2) and the channel's +0x28.
#[hook(addr = 0x4D86B0, conv = thiscall, name = "CAEAudioHardware::PlaySound", expect = "51 56 57 8B 7C 24 10",
       unverified = "std4-audio-3", route = "L2", hazards = "none", this = "CAEAudioHardware")]
#[allow(clippy::too_many_arguments)]
pub fn audio_hardware_play_sound(this: u32, slot: u32, ch: u32, sfx: u32, bank: u32, pt: u32, flags: u32, speed: u32) {
    let s = slot as u16 as i16;
    if s < 0 {
        return;
    }
    let c = ch as u16;
    if c >= r16(this + 0x90 + (s as u32) * 2) {
        return;
    }
    let mut size = sfx;
    let mut rate = bank;
    let buf = bank_get_sound_buffer(r32(this + 0xD98), sfx, bank, local(&mut size), local(&mut rate));
    if buf == 0 {
        return;
    }
    let lo = bank_get_loop_offset(r32(this + 0xD98), sfx, bank);
    let idx = (c as u32).wrapping_add(s as i32 as u32) as u16;
    if idx > 0x40 {
        return;
    }
    let chp = this + 0xE64 + idx as u32 * 4;
    if ch_set_audio_buffer(r32(chp), buf, size, sfx, bank, lo, rate) == 0 {
        return;
    }
    let mut pos = pt as u16 as i16;
    if pos < 0 {
        pos = 0;
    }
    let len = ch_get_length(r32(chp));
    if flags & 8 != 0 && pos > 0 {
        let v = xi(pos as i32).fmul(xf(f32::from_bits(0x3C23_D70A))).fmul(xi(len as i32)).to_f32();
        pos = ftol(X87::from_f64_exact(floor_f(v.to_bits()))) as u16 as i16;
    }
    if (pos as i32) > len as i32 {
        pos = len as u16 as i16;
    }
    if r8(this + 0xDA8) != 0 {
        let v = if flags as u16 & 0x200 != 0 { 0 } else { 1 };
        ch_set_not_in_room(r32(chp), v);
    }
    ch_play(r32(chp), pos as u16 as u32, flags, speed);
    w16(this + (c as u32 + s as i32 as u32) * 2 + 6, flags as u16);
    w32(r32(chp) + 0x28, flags as u16 as i16 as i32 as u32);
}

// ------------------------------------------------------------------ CAEMP3BankLoader::Service ----

/// `CAEMP3BankLoader::Service()` (0x4DFE30): the 50 request slots (0x20 each from +0x24: +0 slot
/// offset pointer, +4 stream offset, +8 size, +0xC data, +0x10 allocation, +0x14 state, +0x18
/// bank, +0x1A slot, +0x1C sound (-1 = whole bank), +0x1E pak) advance when the CdStream channel
/// (+0x66A) is idle. State 1: a read of the bank header (whole bank: the header and the data; one
/// sound: 4 sectors) into a new `CMemoryMgr` block -> state 2. State 2, whole bank: data copied to
/// the slot's place in the bank buffer (+0x1C), the 0x12C0-byte header to the slot info
/// (`+0x14` of the 0x12D4-byte slot), bank id, sound count, the slot's loaded sound -1; request
/// freed; `+0x666 - 1`. State 2, one sound: the header copied, slot bank / count / sound -1, the
/// sound's range from the header (next sound's offset, or the bank's size from the lookup), a read
/// of just that range -> state 3. State 3 (one sound): its data copied, slot bank, the slot's first
/// sound entry offset 0 and the next entry's offset = size, the slot's loaded sound; request freed;
/// `+0x666 - 1`; the queue head (+0x664) advanced when it was this request.
#[hook(addr = 0x4DFE30, conv = thiscall, name = "CAEMP3BankLoader::Service", expect = "51 53 55 56 8B E9 57",
       unverified = "std4-audio-3", route = "L2", hazards = "heap-writes")]
pub fn mp3_bank_loader_service(this: u32) {
    let chan = r16(this + 0x66A) as i16 as i32;
    for i in 0..0x32u32 {
        let rq = this + 0x24 + i * 0x20;
        let state = r32(rq + 0x14);
        let slot_info = |slot: u32| r32(this).wrapping_add((r16(slot) as i16 as i32 as u32).wrapping_mul(0x12D4));
        let reset = || {
            w16(rq + 0x18, 0xFFFF);
            w16(rq + 0x1A, 0xFFFF);
            w16(rq + 0x1C, 0xFFFF);
            w32(rq + 0x10, 0);
            w32(rq + 0x14, 0);
        };
        match state {
            1 => {
                if cd_stream_get_status(chan) != 0 {
                    continue;
                }
                let n = if r16(rq + 0x1C) == 0xFFFF { (r32(rq + 8).wrapping_add(0x12C4) >> 11) + 2 } else { 4 };
                let sec = r32(rq + 4) >> 11;
                let buf = memory_malloc(n << 11, 0);
                w32(rq + 0x10, buf);
                w32(rq + 0xC, r32(rq + 4).wrapping_sub(sec << 11).wrapping_add(buf));
                let pak = r32(r32(this + 0x20) + r8(rq + 0x1E) as u32 * 4);
                cd_stream_read(chan, buf, pak | (sec & 0xFF_FFFF), n);
                w32(rq + 0x14, 2);
            }
            2 => {
                if cd_stream_get_status(chan) != 0 {
                    continue;
                }
                let data = r32(rq + 0xC);
                if r16(rq + 0x1C) == 0xFFFF {
                    let dst = r32(r32(rq)).wrapping_add(r32(this + 0x1C));
                    copy_bytes(dst, data + 0x12C4, r32(rq + 8));
                    let si = slot_info(rq + 0x1A);
                    copy_bytes(si + 0x14, data + 4, 0x4B0 * 4);
                    w16(slot_info(rq + 0x1A) + 0x10, r16(rq + 0x18));
                    w16(slot_info(rq + 0x1A) + 0x12, r16(data));
                    w16(this + 0x66C + (r16(rq + 0x1A) as i16 as i32 as u32) * 2, 0xFFFF);
                    memory_free(r32(rq + 0x10));
                    reset();
                    w16(this + 0x666, r16(this + 0x666).wrapping_sub(1));
                } else {
                    let si = slot_info(rq + 0x1A);
                    copy_bytes(si + 0x14, data + 4, 0x4B0 * 4);
                    w16(slot_info(rq + 0x1A) + 0x10, 0xFFFF);
                    w16(slot_info(rq + 0x1A) + 0x12, 0xFFFF);
                    w16(this + 0x66C + (r16(rq + 0x1A) as i16 as i32 as u32) * 2, 0xFFFF);
                    let hdr = r32(rq + 0xC);
                    let count = r16(hdr) as i16 as i32 - 1;
                    let snd = r16(rq + 0x1C) as i16 as i32;
                    let ent = hdr.wrapping_add((snd * 12) as u32).wrapping_add(4);
                    let next = if snd < count {
                        r32(hdr.wrapping_add((snd * 12) as u32) + 0x10)
                    } else {
                        let bank = r16(rq + 0x18) as i16 as i32;
                        r32(r32(this + 4).wrapping_add((bank * 12) as u32) + 8)
                    };
                    let size = next.wrapping_sub(r32(ent));
                    w32(rq + 8, size);
                    let off = r32(rq + 4).wrapping_add(r32(ent).wrapping_add(0x12C4));
                    w32(rq + 4, off);
                    let sec = off >> 11;
                    let n = (size >> 11) + 2;
                    memory_free(r32(rq + 0x10));
                    let buf = memory_malloc(n << 11, 0);
                    w32(rq + 0x10, buf);
                    w32(rq + 0xC, r32(rq + 4).wrapping_sub(sec << 11).wrapping_add(buf));
                    let pak = r32(r32(this + 0x20) + r8(rq + 0x1E) as u32 * 4);
                    cd_stream_read(chan, buf, pak | (sec & 0xFF_FFFF), n);
                    w32(rq + 0x14, 3);
                }
            }
            3 => {
                if cd_stream_get_status(chan) != 0 || r16(rq + 0x1C) == 0xFFFF {
                    continue;
                }
                let dst = r32(r32(rq)).wrapping_add(r32(this + 0x1C));
                copy_bytes(dst, r32(rq + 0xC), r32(rq + 8));
                w16(slot_info(rq + 0x1A) + 0x10, r16(rq + 0x18));
                let snd = r16(rq + 0x1C) as i16 as i32;
                w32(slot_info(rq + 0x1A).wrapping_add((snd * 12) as u32) + 0x14, 0);
                let nx = (snd + 1) % 0x190;
                w32(slot_info(rq + 0x1A).wrapping_add((nx * 12) as u32) + 0x14, r32(rq + 8));
                w16(this + 0x66C + (r16(rq + 0x1A) as i16 as i32 as u32) * 2, r16(rq + 0x1C));
                memory_free(r32(rq + 0x10));
                reset();
                w32(rq + 0x10, 0);
                w16(this + 0x666, r16(this + 0x666).wrapping_sub(1));
                let head = r16(this + 0x664) as i16 as i32;
                if head == i as i32 {
                    w16(this + 0x664, ((head + 1) % 0x32) as u16);
                }
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------------------ CAESoundManager --------------

/// Sound `i` of the manager (0x74 bytes from +4).
fn snd(i: u32) -> u32 {
    SM + 4 + i * 0x74
}
/// The manager's pause state (game paused or `+0x8CB5`).
fn sm_paused() -> bool {
    game_paused() || r8(SM + 0x8CB5) != 0
}

/// `CAESoundManager::RequestNewSound(CAESound*)` (0x4EFB10, `ret 4`): the first free sound slot
/// (+0x58 == 0) of 300 copies the sound (`operator=`), the source's physical reference is released
/// and cleared, `NewVPSLEntry`, and the hardware is asked for the virtual channel's sound info.
/// Returns the slot, or NULL when all are busy. `#[native]`: its loop branches back into the
/// patched entry.
#[sa_core::native(addr = 0x4EFB10, conv = thiscall, name = "CAESoundManager::RequestNewSound")]
pub fn sound_manager_request_new_sound(this: u32, src: u32) -> u32 {
    let mut i: u32 = 0;
    while i < 0x12C {
        if r16(this + i * 0x74 + 0x5C) == 0 {
            let s = this + i * 0x74 + 4;
            sound_assign(s, src);
            let ent = r32(src + 8);
            if ent != 0 {
                clean_up_old_reference(ent, src + 8);
                w32(src + 8, 0);
            }
            sound_new_vpsl_entry(s);
            hw_request_virtual_channel_sound_info(HW, i, r16(s + 2) as u32, r16(s) as u32);
            return s;
        }
        i += 1;
    }
    0
}

/// The frequency factor of a sound (`relative frequency (with Doppler unless +0x56 bit 0) *
/// slow-motion factor`), as the two loops of `Service` compute it (`base` = the sound).
fn sound_frequency(s: u32) -> f32 {
    let f = if r8(s + 0x56) & 1 != 0 {
        xf(rf(s + 0x64))
    } else {
        X87::from_f64_exact(env_get_doppler(r32(s + 0x4C), r32(s + 0x48), r32(s + 0x44), r32(s + 0x40), r32(s + 0x50))).fmul(xf(rf(s + 0x64)))
    };
    let f = f.to_f32();
    let slomo = if r8(s + 0x56) & 0x10 == 0
        && timer_is_slow_motion_active() != 0
        && r16(0xB6F1A8 + r8(0xB6F081) as u32 * 0x238) != 0x2E
    {
        rf(0x8CBA6C)
    } else {
        1.0
    };
    xf(slomo).fmul(xf(f)).to_f32()
}

/// The sound's position for the hardware: relative to the camera unless +0x56 bit 0 (already
/// relative): `out` = 3 dwords.
fn sound_position(s: u32, out: &mut [u32; 3]) {
    let pos = s + 0x24;
    if r8(s + 0x56) & 1 == 0 {
        env_get_position_relative_to_camera(local_arr(out), pos);
    } else {
        out[0] = r32(pos);
        out[1] = r32(pos + 4);
        out[2] = r32(pos + 8);
    }
}

/// `CAESoundManager::Service()` (0x4F0000). See the module notes; in order: the prioritised list
/// cleared; the elapsed time (paused: `CTimer` pause-mode time, the unpaused time kept; else `CTimer`
/// time since the last call, u16 differences); the channel play times, virtual-channel lengths and
/// loop starts; percentage start positions resolved; physical sounds' play times taken from their
/// channels (stop requests stopped); play times advanced by `elapsed * speed` (paused sounds by 0;
/// past the end: -1 or wrapped into the loop); ended channels freed; ended sounds finished (moving
/// with their entity, the entity's `UpdateParameters(-1)`, references released); parameters and
/// volumes updated; the uncancellable physical sounds first in the prioritised list, the others
/// inserted by volume and "play physically" priority; sounds dropped from the list stopped; new
/// ones played on free channels (flags packed from the sound's flags, volume, position, frequency);
/// every physical sound's volume / frequency (paused, not unpausable: -100 dB and 0) / position;
/// `CAEAudioHardware::Service`; the sounds marked hardware-aware and their frame delays counted
/// down (unless paused and pausable).
#[hook(addr = 0x4F0000, conv = thiscall, name = "CAESoundManager::Service", expect = "83 EC 28 53 55 56 8B F1",
       unverified = "std4-audio-3", route = "L2", hazards = "heap-writes")]
pub fn sound_manager_service(this: u32) {
    let count = |t: u32| r16(t) as u32;
    let phys = |k: u32| r32(this + 0x87F4) + k * 2;
    let pos_of = |k: u32| r32(this + 0x87F8) + k * 2;
    let prio = |k: u32| r32(this + 0x87FC) + k * 2;
    let s_of = |r: u16| this + 4 + (r as i16 as i32 as u32).wrapping_mul(0x74);
    let mut k = 0u32;
    while k < count(this) {
        w16(prio(k), 0xFFFF);
        k += 1;
    }
    let progress: u32;
    if !sm_paused() {
        let t = r32(TIME_MS);
        let base = if r8(this + 0x8CB4) == 0 { r16(this + 0x8CB0) } else { r16(this + 0x8CB8) };
        progress = (t as u16).wrapping_sub(base) as u32;
        w32(this + 0x8CB0, t);
        w8(this + 0x8CB4, 0);
    } else {
        let t = r32(TIME_PAUSE_MS);
        if r8(this + 0x8CB4) == 0 {
            w32(this + 0x8CB8, r32(this + 0x8CB0));
            progress = 0;
        } else {
            progress = (t as u16).wrapping_sub(r16(this + 0x8CB0)) as u32;
        }
        w32(this + 0x8CB0, t);
        w8(this + 0x8CB4, 1);
    }
    hw_get_channel_play_times(HW, r16(this + 2) as u32, r32(this + 0x87F8));
    hw_get_virtual_channel_sound_lengths(HW, this + 0x8800);
    hw_get_virtual_channel_sound_loop_start_times(HW, this + 0x8A58);
    for i in 0..0x12Cu32 {
        let s = snd(i);
        if r16(s + 0x58) != 0 && r16(s + 0x5A) != 0 && r8(s + 0x56) & 0x20 != 0 {
            w8(s + 0x56, r8(s + 0x56) & 0xDF);
            if r16(s + 0x5E) == 0 {
                let prod = (r16(s + 0x5C) as i16 as i32).wrapping_mul(r16(this + 0x8800 + i * 2) as i16 as i32);
                w16(s + 0x5C, ftol(xi(prod).fmul(xf(f32::from_bits(0x3C23_D70A)))) as u16);
            }
        }
    }
    let mut c: u32 = 0;
    while c < count(this) {
        let r = r16(phys(c));
        if r != 0xFFFF {
            w16(s_of(r) + 0x5C, r16(pos_of(c)));
            if r16(s_of(r16(phys(c))) + 0x68) != 0 {
                hw_stop_sound(HW, r16(this + 2) as u32, c);
            }
        }
        c = (c as u16).wrapping_add(1) as i16 as i32 as u32;
    }
    for i in 0..0x12Cu32 {
        let s = snd(i);
        if r16(s + 0x58) == 0 || r16(s + 0x5A) == 0 || r8(s + 0x54) != 0 {
            continue;
        }
        let len = r16(this + 0x8800 + i * 2);
        let ls = r16(this + 0x8A58 + i * 2);
        w16(s + 0x70, len);
        if r16(s + 0x5E) != 0 {
            continue;
        }
        if r16(s + 0x68) != 0 {
            w16(s + 0x5C, 0xFFFF);
            continue;
        }
        let t = if sm_paused() && r8(s + 0x56) & 0x10 == 0 {
            ftol(xf(rf(s + 0x64)).fmul(xf(0.0)).fadd(xi(r16(s + 0x5C) as i16 as i32)))
        } else {
            ftol(xi(progress as u16 as i16 as i32).fmul(xf(rf(s + 0x64))).fadd(xi(r16(s + 0x5C) as i16 as i32)))
        } as u16;
        w16(s + 0x5C, t);
        if (t as i16) < (len as i16) {
            continue;
        }
        if ls == 0xFFFF {
            w16(s + 0x5C, ls);
        } else {
            let rem = (t as i16 as i32) % (len as i16 as i32);
            w16(s + 0x5C, (rem + ls as i16 as i32) as u16);
        }
    }
    let mut c: u32 = 0;
    while c < count(this) {
        if r16(pos_of(c)) == 0xFFFF {
            w16(phys(c), 0xFFFF);
        }
        c += 1;
    }
    for i in 0..0x12Cu32 {
        let s = snd(i);
        if r16(s + 0x58) == 0 || r8(s + 0x54) != 0 || r16(s + 0x5C) != 0xFFFF {
            continue;
        }
        if (r8(s + 0x56) as i8) < 0 {
            let ent = r32(s + 8);
            if ent != 0 {
                let m = r32(ent + 0x14);
                let pa = if m != 0 { m + 0x30 } else { ent + 4 };
                sound_set_position(s, r32(pa), r32(pa + 4), r32(pa + 8));
            } else {
                w16(s + 0x68, 1);
            }
        }
        if r8(s + 0x56) & 4 != 0 {
            let ae = r32(s + 4);
            if ae != 0 {
                entity_update_parameters(ae, s, 0xFFFF_FFFF);
                if crate::port4_hw::feq(xf(rf(s + 0x20)), xf(0.0)) {
                    w32(s + 0x64, r32(s + 0x1C));
                }
            }
        }
        let ent = r32(s + 8);
        w16(s + 0x58, 0);
        if ent != 0 {
            clean_up_old_reference(ent, s + 8);
            w32(s + 8, 0);
        }
        w16(s + 0x5E, 0);
        w16(s + 0x5C, 0);
    }
    for i in 0..0x12Cu32 {
        let s = snd(i);
        if r16(s + 0x58) != 0 {
            sound_update_parameters(s, r16(s + 0x5C) as u32);
        }
    }
    for i in 0..0x12Cu32 {
        let s = snd(i);
        if r16(s + 0x58) != 0 {
            sound_calculate_volume(s);
        }
    }
    let mut n: u32 = 0;
    let mut c: u32 = 0;
    while c < count(this) {
        let r = r16(phys(c));
        if r != 0xFFFF && r8(s_of(r) + 0x56) & 2 != 0 {
            w16(prio(n as u16 as i16 as i32 as u32), r);
            n += 1;
        }
        c = (c as u16).wrapping_add(1) as i16 as i32 as u32;
    }
    for i in 0..0x12Cu32 {
        let s = snd(i);
        if r16(s + 0x58) == 0 {
            continue;
        }
        if r16(s + 0x5E) != 0 && r8(s + 0x56) & 2 != 0 {
            continue;
        }
        if r8(s + 0x54) != 0 {
            continue;
        }
        let bl = (r8(s + 0x56) >> 3) & 1;
        let vol = rf(s + 0x60);
        let last = (count(this) as u16).wrapping_sub(1) as i16;
        let mut ax = last;
        while ax >= n as i16 && r16(prio(ax as i32 as u32)) == 0xFFFF {
            ax -= 1;
        }
        let mut dx = ax;
        while dx >= n as i16 {
            let o = s_of(r16(prio(dx as i32 as u32)));
            let other = xf(rf(o + 0x60));
            let cl = (r8(o + 0x56) >> 3) & 1;
            // `fcomp` of vol with other; `test ah, 5; jp` -> continue when not below (or unordered)
            let below = !xf(vol).fcom(other).test_parity(0x05);
            if below && bl <= cl {
                break;
            }
            dx -= 1;
        }
        let edi = dx as i32;
        if edi == count(this) as i32 - 1 {
            continue;
        }
        let mut kk = last as i32;
        while kk > edi + 1 {
            w16(prio(kk as u32), r16(prio(kk as u32 - 1)));
            kk -= 1;
        }
        w16(prio((edi + 1) as u32), i as u16);
    }
    let mut c: u32 = 0;
    while c < count(this) {
        let r = r16(phys(c));
        if r != 0xFFFF {
            let mut k: u32 = 0;
            while k < count(this) && r != r16(prio(k)) {
                k = (k as u16).wrapping_add(1) as i16 as i32 as u32;
            }
            if k != count(this) {
                w16(prio(k), 0xFFFF);
            } else {
                w16(s_of(r) + 0x5E, 0);
                w16(phys(c), 0xFFFF);
                hw_stop_sound(HW, r16(this + 2) as u32, c);
            }
        }
        c = (c as u16).wrapping_add(1) as i16 as i32 as u32;
    }
    let mut chn: u32 = 0;
    let mut i: u32 = 0;
    while i < count(this) {
        let r = r16(prio(i));
        if r != 0xFFFF && (chn as u16 as i16 as i32) < count(this) as i32 {
            let mut free = true;
            while r16(phys(chn as u16 as i16 as i32 as u32)) != 0xFFFF {
                chn = (chn as u16).wrapping_add(1) as u32;
                if (chn as u16 as i16 as i32) >= count(this) as i32 {
                    free = false;
                    break;
                }
            }
            if free {
                let ch = chn as u16 as i16 as i32 as u32;
                w16(phys(ch), r);
                let s = s_of(r16(prio(i)));
                w16(s + 0x5E, 1);
                let freq = sound_frequency(s);
                let f = r16(s + 0x56) as u32;
                let bit = |b: u32| (f >> b) & 1;
                let pausable = (bit(0) != 0 && f & 0x10 != 0) as u32;
                let flags = bit(0) | bit(9) << 1 | bit(8) << 2 | bit(5) << 3 | bit(6) << 4 | bit(10) << 6 | bit(11) << 7
                    | bit(12) << 8 | pausable << 9;
                hw_play_sound(HW, r16(this + 2) as u32, ch, r16(s + 2) as u32, r16(s) as u32, r16(s + 0x5C) as u32, flags, r32(s + 0x1C));
                hw_set_channel_volume(HW, r16(this + 2) as u32, ch, r32(s + 0x60), 0);
                let mut pos = [0u32; 3];
                sound_position(s, &mut pos);
                hw_set_channel_position(HW, r16(this + 2) as u32, ch, local_arr(&mut pos), 0);
                hw_set_channel_frequency_scaling_factor(HW, r16(this + 2) as u32, ch, freq.to_bits());
                chn = chn.wrapping_add(1);
            }
        }
        i = (i as u16).wrapping_add(1) as i16 as i32 as u32;
    }
    let mut c: u32 = 0;
    while c < count(this) {
        let r = r16(phys(c));
        if r != 0xFFFF {
            let s = s_of(r);
            if r16(s + 0x58) != 0 {
                if !sm_paused() || r8(s + 0x56) & 0x10 != 0 {
                    hw_set_channel_volume(HW, r16(this + 2) as u32, c, r32(s + 0x60), 0);
                    let freq = sound_frequency(s);
                    hw_set_channel_frequency_scaling_factor(HW, r16(this + 2) as u32, c, freq.to_bits());
                } else {
                    hw_set_channel_volume(HW, r16(this + 2) as u32, c, 0xC2C8_0000, 0);
                    hw_set_channel_frequency_scaling_factor(HW, r16(this + 2) as u32, c, 0);
                }
                let mut pos = [0u32; 3];
                sound_position(s, &mut pos);
                hw_set_channel_position(HW, r16(this + 2) as u32, c, local_arr(&mut pos), 0);
            }
        }
        c = (c as u16).wrapping_add(1) as i16 as i32 as u32;
    }
    hw_service(HW);
    for i in 0..0x12Cu32 {
        let s = snd(i);
        if r16(s + 0x58) == 0 {
            continue;
        }
        let cl = r8(s + 0x54) as i8;
        w16(s + 0x5A, 1);
        if cl <= 0 {
            continue;
        }
        if sm_paused() && r8(s + 0x56) & 0x10 == 0 {
            continue;
        }
        w8(s + 0x54, (cl - 1) as u8);
    }
}

// ------------------------------------------------------------------ CAudioEngine::Service --------

/// `CAudioEngine::Service()` (0x507750, entry `push esi; push edi; mov esi, ecx; jmp` into the
/// protection chunk 0x40139B, which leaves `[0x1560628] + [0x563470]` on the stack as the speed
/// argument of the first `AddAudioEvent`). Frontend events 0x1F (volume 0) and, unpaused, 0x24
/// (speed 1.0); the track play time; the radio channel's play times; cutscene track finished
/// (+0 set, play time -4) -> +1; a finished one (-6) restarts a vehicle radio; cutscene / ambience /
/// radio track services; with a player, the ambient gunfire; collisions; faders (paused: music,
/// effects and stream 1.0; else stream 1.0 or 0 by `[0xB5F851]`); script, ped speech, sound manager
/// and police scanner services. `#[native]`.
#[sa_core::native(addr = 0x507750, conv = thiscall, name = "CAudioEngine::Service")]
pub fn audio_engine_service(this: u32) {
    let hidden = r32(0x1560628).wrapping_add(r32(0x563470));
    frontend_add_audio_event_call(this + 0xB4, 0x1F, 0, hidden);
    if !game_paused() {
        frontend_add_audio_event_call(this + 0xB4, 0x24, 0, 0x3F80_0000);
    }
    let pt = hw_get_track_play_time(HW);
    hw_get_channel_play_times(HW, r16(this + 4) as u32, 0);
    if r8(this) != 0 && pt == 0xFFFF_FFFC {
        w8(this + 1, 1);
    } else if r8(this + 1) != 0 && pt == 0xFFFF_FFFA {
        w8(this + 1, 0);
        if radio_is_vehicle_radio_active(0x8CB6F8) != 0 {
            let s = vehicle_player_settings_for_radio();
            radio_start_radio(0x8CB6F8, s);
        }
    }
    cutscene_service(0x8AE554, pt);
    ambience_service(0x8AC15C, pt);
    radio_service(0x8CB6F8, pt);
    if find_player_ped(0) != 0 {
        global_weapon_service_ambient_gun_fire(r32(this + 0x1E34));
    }
    collision_service(this + 0x4BC);
    if game_paused() {
        hw_set_music_fader(HW, 0x3F80_0000);
        hw_set_effects_fader(HW, 0x3F80_0000);
        hw_set_stream_fader(HW, 0x3F80_0000);
    } else if r8(0xB5F851) != 0 {
        hw_set_stream_fader(HW, 0);
    } else {
        hw_set_stream_fader(HW, 0x3F80_0000);
    }
    script_service(this + 0x2A0);
    ped_speech_service();
    sm_service(SM);
    police_scanner_service();
}

// ------------------------------------------------------------------ AddAudioEvent ----------------

/// The frontend entity's event sound builder: a stack `CAESound` (zeroed; the original's is
/// uninitialised except +8).
struct EvSound {
    s: [u32; 0x1D],
}

impl EvSound {
    fn addr(&mut self) -> u32 {
        GPtr::<u32>::from_raw(self.s.as_mut_ptr()).addr() as u32
    }
    /// `CAESound::Initialise(slot, sfx, entity, pos, volume, maxDistance, speed, timeScale, 0, 0, 0, 0)`.
    #[allow(clippy::too_many_arguments)]
    fn init(&mut self, slot: u32, sfx: u32, ent: u32, pos: [u32; 3], vol: u32, maxd: u32, speed: u32, ts: u32) {
        let a = self.addr();
        sound_initialise(a, slot, sfx, ent, pos[0], pos[1], pos[2], vol, maxd, speed, ts, 0, 0, 0, 0);
    }
    fn set32(&mut self, off: u32, v: u32) {
        let a = self.addr();
        w32(a + off, v);
    }
    fn set16(&mut self, off: u32, v: u16) {
        let a = self.addr();
        w16(a + off, v);
    }
    fn request(&mut self) -> u32 {
        let a = self.addr();
        sm_request_new_sound(SM, a)
    }
}

const ONE: u32 = 0x3F80_0000;

/// `CAEFrontendAudioEntity::AddAudioEvent(event, volumeBoost, speed)` (0x4DD4A0, `ret 0xC`).
///
/// Volume = `(int8) EVENTVOL[event] + boost`. Events 0..0x65 (byte table 0x4DED2C, 34 handlers of
/// jump table 0x4DECA0): the menu / game sounds of banks 0x3B (slot 0) and 0x3C (slot 1), most as
/// a left / right pair at (-1,0,0) / (1,0,0) with the event's speed and flags 0x151B / 0x111B; some
/// rate-limited to one per 5 frames (+0x84 / +0x88); the bullet-pass sounds of slot 0x2A from
/// rotating banks 0x1C..0x1F at the four corner positions (+0x80 count, +0x82 bank, +0x8C frame);
/// the "mission complete" twin loops (+0x9C / +0x144); single sounds at (0,1,0); the loading tune
/// pair (bank 0x52, sounds `2 * seed (+1)` in slots 2 / 5, event 0x32), its cancel (0x33). Every
/// case is transcribed from the bytes (the per-case constants are in the code).
#[hook(addr = 0x4DD4A0, conv = thiscall, name = "CAEFrontendAudioEntity::AddAudioEvent", expect = "6A FF 68 3B BF 83 00",
       unverified = "std4-audio-3", route = "L2", hazards = "uninit-locals,heap-writes",
       note = "stack CAESound zeroed (the original's is uninitialised)")]
pub fn frontend_add_audio_event(this: u32, event: u32, boost: u32, speed: u32) {
    let ent = this;
    let m: [u32; 3] = [0xBF80_0000, 0, 0];
    let pp: [u32; 3] = [ONE, 0, 0];
    let f: [u32; 3] = [0, ONE, 0];
    let ul: [u32; 3] = [0xBDCC_CCCD, ONE, 0];
    let ur: [u32; 3] = [0x3DCC_CCCD, ONE, 0];
    let lr: [u32; 3] = [0x3DCC_CCCD, 0xBF80_0000, 0];
    let ll: [u32; 3] = [0xBDCC_CCCD, 0xBF80_0000, 0];
    let evvol = r8(r32(0xBD00F8).wrapping_add(event)) as i8 as i32;
    let vol = xi(evvol).fadd(xf(f32::from_bits(boost))).to_f32().to_bits();
    let mut sd = EvSound { s: [0; 0x1D] };
    let loaded = |bank: u32, slot: u32| hw_is_sound_bank_loaded(HW, bank, slot) != 0;
    let frame_ok = |off: u32| {
        let fr = r32(FRAME_COUNTER);
        if fr < r32(this + off).wrapping_add(5) {
            return false;
        }
        w32(this + off, fr);
        true
    };
    // a pair of sounds (left / right) with the event's speed
    let pair = |sd: &mut EvSound, slot: u32, a: u32, b: u32, flags: u16| {
        sd.init(slot, a, ent, m, vol, ONE, ONE, ONE);
        sd.set32(0x1C, speed);
        sd.set16(0x56, flags);
        sd.request();
        sd.init(slot, b, ent, pp, vol, ONE, ONE, ONE);
        sd.set32(0x1C, speed);
        sd.set16(0x56, flags);
        sd.request();
    };
    match event {
        0x00 => {
            if loaded(0x3B, 0) {
                pair(&mut sd, 0, 0x19, 0x1A, 0x151B);
            }
        }
        0x0B | 0x20 => {
            if loaded(0x3B, 0) {
                pair(&mut sd, 0, 0xE, 0xF, 0x151B);
            }
        }
        0x01 => {
            if loaded(0x3C, 1) {
                pair(&mut sd, 1, 6, 7, 0x151B);
            }
        }
        0x02 => {
            if loaded(0x3C, 1) {
                pair(&mut sd, 1, 0, 1, 0x151B);
            }
        }
        0x03 => {
            if loaded(0x3C, 1) {
                pair(&mut sd, 1, 4, 5, 0x111B);
            }
        }
        0x04 => {
            if loaded(0x3C, 1) {
                pair(&mut sd, 1, 2, 3, 0x151B);
            }
        }
        0x05 => {
            if loaded(0x3C, 1) {
                sd.init(1, 8, ent, f, vol, ONE, ONE, ONE);
                sd.set32(0x1C, speed);
                sd.set16(0x56, 0x11B);
                sd.request();
            }
        }
        0x06 | 0x12 | 0x13 | 0x14 | 0x28 => {
            if loaded(0x3B, 0) && frame_ok(0x84) {
                pair(&mut sd, 0, 0x1B, 0x1C, 0x111B);
            }
        }
        0x07..=0x0A => {
            if loaded(0x3B, 0) && frame_ok(0x84) {
                pair(&mut sd, 0, 0x10, 0x11, 0x111B);
            }
        }
        0x0E | 0x0F | 0x11 => {
            if loaded(0x3B, 0) && frame_ok(0x84) {
                for (sfx, pos) in [(0x1B, m), (0x1C, pp)] {
                    sd.init(0, sfx, ent, pos, vol, ONE, 0x3F57_4BC7, ONE);
                    sd.set16(0x56, 0x111B);
                    sd.request();
                }
            }
        }
        0x10 => {
            if loaded(0x8F, 5) {
                let (a, b) = if ae_resolve_probability(0x3F00_0000) != 0 { (0x3F98_3809, ONE) } else { (ONE, 0x3F98_3809) };
                for (pos, sp) in [(m, a), (pp, b)] {
                    sd.init(5, 0x1C, ent, pos, vol, ONE, sp, ONE);
                    sd.set16(0x56, 0x110F);
                    sd.set32(0xC, 0x10);
                    sd.request();
                }
                w32(this + 0x90, r32(TIME_MS));
            }
        }
        0x2E => {
            if loaded(0x3B, 0) {
                sd.init(0, 9, ent, f, vol, ONE, ONE, ONE);
                sd.set16(0x56, 0x11B);
                sd.set32(0x20, 0x3D70_D845);
                sd.request();
            }
        }
        0x1B..=0x1E => {
            let bank = r16(this + 0x82) as u32;
            if !loaded(bank, 0x2A) || r32(FRAME_COUNTER) < r32(this + 0x8C).wrapping_add(5) {
                return end(&mut sd);
            }
            if r16(this + 0x80) as i16 > 10 {
                if sm_are_sounds_playing_in_bank_slot(SM, 0x2A) != 0 {
                    return end(&mut sd);
                }
                w16(this + 0x82, r16(this + 0x82).wrapping_add(1));
                if r16(this + 0x82) as i16 > 0x1F {
                    w16(this + 0x82, 0x1C);
                }
                hw_load_sound_bank(HW, r16(this + 0x82) as u32, 0x2A);
                w16(this + 0x80, 0);
                return end(&mut sd);
            }
            let pos = match event {
                0x1B => ll,
                0x1C => ul,
                0x1D => lr,
                _ => ur,
            };
            w32(this + 0x8C, r32(FRAME_COUNTER));
            let sfx = ae_random_int(0, 2) as u32;
            sd.init(0x2A, sfx, ent, pos, vol, ONE, ONE, ONE);
            sd.set32(0xC, event);
            sd.set32(0x20, 0x3D00_0000);
            sd.set16(0x56, 7);
            sd.request();
            w16(this + 0x80, r16(this + 0x80).wrapping_add(1));
        }
        0x1F => {
            if r8(this + 0x7C) == 0 && loaded(0x3B, 0) {
                sd.init(0, 8, ent, f, vol, ONE, ONE, ONE);
                sd.set32(0x1C, speed);
                sd.set16(0x56, 0x11F);
                let r = sd.request();
                w32(this + 0x98, r);
                w8(this + 0x7C, (r != 0) as u8);
            }
        }
        0x21 => {
            if sm_are_sounds_of_event(SM, 0x21, ent) == 0 {
                if !loaded(0x3B, 0) {
                    return end(&mut sd);
                }
                for (sfx, pos) in [(4, m), (5, pp)] {
                    sd.init(0, sfx, ent, pos, vol, ONE, ONE, ONE);
                    sd.set32(0x1C, speed);
                    sd.set16(0x56, 0x110F);
                    sd.set32(0xC, 0x21);
                    sd.request();
                }
            }
            w32(this + 0x94, r32(TIME_MS));
        }
        0x22 => {
            if loaded(0x3B, 0) {
                let (twin, busy, flags) = if !game_paused() { (this + 0x9C, 0x124, 0x14B) } else { (this + 0x144, 0x1CC, 0x15B) };
                if r16(this + busy) == 0 {
                    twin_initialise(twin, 0, 2, 1, ent, 0xC8, 0x28A, 0xFFFF_FFFF, 0xFFFF_FFFF);
                    twin_play(twin, f[0], f[1], f[2], vol, ONE, ONE, ONE, flags);
                }
            }
        }
        0x23 => {
            if !game_paused() {
                if r16(this + 0x124) != 0 {
                    twin_stop_and_forget(this + 0x9C);
                }
            } else if r16(this + 0x1CC) != 0 {
                twin_stop_and_forget(this + 0x144);
            }
        }
        0x24 => {
            if r16(this + 0x1CC) != 0 {
                twin_stop_and_forget(this + 0x144);
            }
        }
        0x25 => {
            if loaded(0x3B, 0) {
                sd.init(0, 0x17, ent, f, vol, ONE, ONE, ONE);
                sd.set16(0x56, 0x11B);
                sd.request();
            }
        }
        0x26 => {
            if loaded(0x3B, 0) {
                sd.init(0, 0x17, ent, f, vol, ONE, 0x3F64_1206, ONE);
                sd.set16(0x56, 0x11B);
                sd.request();
            }
        }
        0x29 | 0x2A => {
            if loaded(0x3B, 0) {
                sd.init(0, if event == 0x29 { 0xA } else { 0xB }, ent, f, vol, ONE, ONE, ONE);
                sd.set32(0x1C, speed);
                sd.set16(0x56, 3);
                sd.request();
            }
        }
        0x0C | 0x0D | 0x2B => {
            if loaded(0x3B, 0) && frame_ok(0x88) {
                pair(&mut sd, 0, 0x12, 0x13, 0x151B);
            }
        }
        0x2C => {
            if loaded(0x3B, 0) {
                pair(&mut sd, 0, 6, 7, 0x151B);
            }
        }
        0x2D => {
            if loaded(0x3B, 0) {
                pair(&mut sd, 0, 0xC, 0xD, 0x151B);
            }
        }
        0x65 | 0x2F | 0x31 => {
            if loaded(0x3B, 0) && sm_are_sounds_of_event(SM, event, ent) == 0 {
                let (sfx, flags) = match event {
                    0x65 => (0xD, 0x101),
                    0x2F => (3, 0x103),
                    _ => (0x18, 0x103),
                };
                sd.init(0, sfx, ent, f, vol, ONE, ONE, ONE);
                sd.set32(0x1C, speed);
                sd.set16(0x56, flags);
                sd.set32(0xC, event);
                sd.request();
            }
        }
        0x30 => sm_cancel_sounds_of_event(SM, 0x2F, ent),
        0x32 => {
            let seed = r8(this + 0x7D) as i8 as i16 as u16 as u32;
            if hw_is_sound_loaded(HW, 0x52, (seed << 1) & 0xFFFF, 2) != 0
                && hw_is_sound_loaded(HW, 0x52, (seed * 2 + 1) & 0xFFFF, 5) != 0
                && sm_are_sounds_of_event(SM, 0x32, ent) == 0
            {
                for (slot, sfx, pos) in [(2, (seed << 1) & 0xFFFF, m), (5, (seed * 2 + 1) & 0xFFFF, pp)] {
                    sd.init(slot, sfx, ent, pos, vol, ONE, ONE, ONE);
                    sd.set32(0x1C, speed);
                    sd.set16(0x56, 0x111B);
                    sd.set32(0xC, 0x32);
                    sd.request();
                }
            }
        }
        0x33 => sm_cancel_sounds_of_event(SM, 0x32, ent),
        _ => {}
    }
    end(&mut sd);
}

/// The common exit of `AddAudioEvent`: `~CAESound()` of the stack sound.
fn end(sd: &mut EvSound) {
    let a = sd.addr();
    sound_dtor(a);
}

/// `CAETwinLoopSoundEntity::PlayTwinLoopSound(CVector pos, float volume, float maxDistance?,
/// float speed?, float timeScale, int16 flags)` (0x4F2CB0, `ret 0x20`; the SEH frame's handler
/// slot is a protection-computed value). Both previous sounds stopped and forgotten; sound 1
/// (`+0x7C` slot, `+0x7E` sfx, the volume, flags | 0x24) starts at play position `+0x9A` (or a random
/// 0..0x63 when -1) -> +0xA0; sound 2 (`+0x80` sfx, -100 dB) at `+0x9C` (or random) -> +0xA4; the
/// next swap time `rand(+0x8E, +0x90) + CTimer pause-mode time` -> +0x94; +0x98 = 1.
///
/// Not registered: 0x4F2CB0 is hooked by sa-audio port3_ent.rs (L2, PORT-AUDIO-3) (duplicate port left unregistered when STD-6 merged both).
#[allow(clippy::too_many_arguments)]
pub fn twin_loop_play_twin_loop_sound(this: u32, x: u32, y: u32, z: u32, vol: u32, a: u32, b: u32, c: u32, flags: u32) {
    let mut sd = EvSound { s: [0; 0x1D] };
    if r32(this + 0xA0) != 0 {
        sound_stop_and_forget(r32(this + 0xA0));
    }
    if r32(this + 0xA4) != 0 {
        sound_stop_and_forget(r32(this + 0xA4));
    }
    let fl = flags | 0x24;
    let mut pt = r16(this + 0x9A) as u32;
    if pt == 0xFFFF {
        pt = ae_random_int(0, 0x63) as u32;
    }
    let sa = sd.addr();
    sound_initialise(sa, r16(this + 0x7C) as u32, r16(this + 0x7E) as u32, this, x, y, z, vol, b, a, c, 0, fl, 0, 0);
    sd.set16(0x5C, pt as u16);
    w32(this + 0xA0, sd.request());
    let mut pt = r16(this + 0x9C) as u32;
    if pt == 0xFFFF {
        pt = ae_random_int(0, 0x63) as u32;
    }
    let sa = sd.addr();
    sound_initialise(sa, r16(this + 0x7C) as u32, r16(this + 0x80) as u32, this, x, y, z, 0xC2C8_0000, b, a, c, 0, fl, 0, 0);
    sd.set16(0x5C, pt as u16);
    w32(this + 0xA4, sd.request());
    let t = r32(TIME_PAUSE_MS);
    let rr = ae_random_int(r16(this + 0x8E) as i32, r16(this + 0x90) as i32) as u32;
    w32(this + 0x94, rr.wrapping_add(t));
    w8(this + 0x98, 1);
    let sa = sd.addr();
    sound_dtor(sa);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_FLOOR_OF_FLOAT,
    HOOK_AUDIO_HARDWARE_UPDATE_REVERB_ENVIRONMENT,
    HOOK_AUDIO_HARDWARE_PLAY_SOUND,
    HOOK_MP3_BANK_LOADER_SERVICE,
    HOOK_SOUND_MANAGER_SERVICE,
    HOOK_FRONTEND_ADD_AUDIO_EVENT,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[
    NATIVE_SOUND_SET_POSITION_NATIVE,
    NATIVE_VEHICLE_GET_PLAYER_SETTINGS_FOR_RADIO,
    NATIVE_AUDIO_HARDWARE_SET_CHANNEL_VOLUME,
    NATIVE_AUDIO_HARDWARE_SERVICE,
    NATIVE_SOUND_MANAGER_REQUEST_NEW_SOUND,
    NATIVE_AUDIO_ENGINE_SERVICE,
];

// thin wrappers over the `vslot` declarations (they take the object as a `GPtr`)
#[inline]
fn ch_synch_playback(ch: u32) {
    ch_synch_playback_raw(GPtr::from_addr(ch as usize))
}
#[inline]
fn ch_service(ch: u32) {
    ch_service_raw(GPtr::from_addr(ch as usize))
}
#[inline]
fn ch_get_length(ch: u32) -> u32 {
    ch_get_length_raw(GPtr::from_addr(ch as usize))
}
#[inline]
fn ch_play(ch: u32, start: u32, flags: u32, speed: u32) {
    ch_play_raw(GPtr::from_addr(ch as usize), start, flags, speed)
}
#[inline]
fn li_commit(l: u32) -> i32 {
    li_commit_raw(GPtr::from_addr(l as usize))
}
#[inline]
fn entity_update_parameters(ent: u32, s: u32, pt: u32) {
    entity_update_parameters_raw(GPtr::from_addr(ent as usize), s, pt)
}
