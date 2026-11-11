//! STD-4 audio lane, part 2: `CAudioEngine::Initialise` (0x5B9C60) and the parts of its tree that
//! were not ported: `CAEAudioHardware::Initialise` / `InitDirectSoundListener` /
//! `Query3DSoundEffects`, the MP3 bank and track loaders' file loading, `getDvdGamePath`,
//! `CAESoundManager::Initialise`, `CAEFrontendAudioEntity::Initialise`,
//! `CAEAudioUtility::StaticInitialise` and `CAEUserRadioTrackManager::ScanUserTracks`. Ported from
//! the 1.0 US bytes (gta-reversed as reading aid; the bytes win).
//!
//! Exception frames (`new` + constructor pairs) are not modelled: no constructor here throws.

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::port4_hw::{
    com_query_interface, com_release, ds_create_sound_buffer, ds_get_caps, ds_get_speaker_config,
    ds_set_cooperative_level, operator_new, r16, r32, r8, w16, w32, w8, xf, xi,
};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

const HW: u32 = 0xB5F8B8;
const USER_RADIO: u32 = 0xB6B970;
const SMOOTH_FADE: u32 = 0xB608D0;

orig! {
    // Win32 / CRT
    fn co_initialize(r: u32) -> i32 = stdcall cell 0x858350 => unported;
    /// `EAXDirectSoundCreate(guid, &ds, outer)` (EAX.DLL ordinal 6)
    fn eax_direct_sound_create(guid: u32, out: u32, outer: u32) -> i32 = stdcall cell 0x85801C => unported;
    fn sleep(ms: u32) = stdcall cell 0x8580C4 => unported;
    fn create_file_a(name: u32, access: u32, share: u32, sec: u32, disp: u32, flags: u32, t: u32) -> u32 = stdcall cell 0x8580A0 => unported;
    fn get_file_size(h: u32, hi: u32) -> u32 = stdcall cell 0x858074 => unported;
    fn read_file(h: u32, buf: u32, n: u32, read: u32, ov: u32) -> i32 = stdcall cell 0x858098 => unported;
    fn close_handle(h: u32) -> i32 = stdcall cell 0x858078 => unported;
    fn lstrlen_a(s: u32) -> i32 = stdcall cell 0x8580DC => unported;
    fn lstrcpy_a(d: u32, s: u32) -> u32 = stdcall cell 0x8580D4 => unported;
    fn lstrcat_a(d: u32, s: u32) -> u32 = stdcall cell 0x8580D8 => unported;
    fn set_error_mode(m: u32) -> u32 = stdcall cell 0x85813C => unported;
    fn get_logical_drive_strings_a(n: u32, buf: u32) -> u32 = stdcall cell 0x858138 => unported;
    fn get_drive_type_a(root: u32) -> u32 = stdcall cell 0x858134 => unported;
    fn get_volume_information_a(root: u32, name: u32, n: u32, serial: u32, maxlen: u32, flags: u32, fs: u32, fsn: u32) -> i32 = stdcall cell 0x858130 => unported;
    fn create_thread(sec: u32, stack: u32, entry: u32, param: u32, flags: u32, id: u32) -> u32 = stdcall cell 0x8580B4 => unported;
    fn query_performance_frequency(out: u32) -> i32 = stdcall cell 0x858114 => unported;
    fn get_last_error() -> u32 = stdcall cell 0x858094 => unported;
    /// `operator new[]` (0x821195)
    fn operator_new_array(n: u32) -> u32 = cdecl 0x821195 => unported;
    /// CRT `free` (0x8214BD, `operator delete`)
    fn crt_free(p: u32) = cdecl 0x8214BD => unported;
    /// CRT `free` (0x8213AE, `operator delete[]`)
    fn crt_free_array(p: u32) = cdecl 0x8213AE => unported;
    /// CRT `printf` (0x821982; the release build's debug print)
    fn crt_printf(fmt: u32) -> i32 = cdecl 0x821982 => unported;

    // game functions
    fn loading_screen_pause() = cdecl 0x590310 => loading_screen_pause_native;
    fn loading_screen_continue() = cdecl 0x590320 => unported;
    fn file_open(name: u32, mode: u32) -> u32 = cdecl 0x538900 => unported;
    fn file_read(f: u32, buf: u32, n: u32) -> u32 = cdecl 0x538950 => unported;
    fn file_close(f: u32) -> i32 = cdecl 0x5389D0 => unported;
    fn file_total_size(f: u32) -> u32 = cdecl 0x5389E0 => unported;
    fn memory_malloc(n: u32, hint: u32) -> u32 = cdecl 0x72F420 => unported;
    fn cd_stream_open(name: u32, a: u32) -> u32 = cdecl 0x4067B0 => unported;
    fn bank_calculate_offsets(this: u32) = thiscall 0x4DFBA0 => unported;
    fn bank_reset_requests(this: u32) = thiscall 0x4DFB60 => bank_loader_reset_requests;
    fn bank_ctor(this: u32) -> u32 = thiscall 0x4DFB10 => unported;
    fn bank_initialise(this: u32) -> bool = thiscall 0x4E08F0 => mp3_bank_loader_initialise;
    fn bank_load_slot_file(this: u32) -> bool = thiscall 0x4E0590 => bank_loader_load_bank_slot_file;
    fn bank_load_lookup_file(this: u32) -> bool = thiscall 0x4DFBD0 => bank_loader_load_bank_lookup_file;
    fn bank_load_sfx_pak_lookup_file(this: u32) -> bool = thiscall 0x4DFC70 => bank_loader_load_sfx_pak_lookup_file;
    fn track_ctor(this: u32) -> u32 = thiscall 0x4E0930 => unported;
    fn track_initialise(this: u32) -> bool = thiscall 0x4E0C50 => mp3_track_loader_initialise;
    fn track_load_stream_pack_table(this: u32) -> bool = thiscall 0x4E0970 => mp3_track_loader_load_stream_pack_table;
    fn track_load_lookup_table(this: u32) -> bool = thiscall 0x4E09F0 => mp3_track_loader_load_track_lookup_table;
    fn get_dvd_game_path() -> u32 = cdecl 0x747300 => get_dvd_game_path_port;
    fn hw_initialise(this: u32) -> bool = thiscall 0x4D9930 => audio_hardware_initialise;
    fn hw_init_listener(this: u32, ch: u32, rate: u32, bits: u32) -> bool = thiscall 0x4D9640 => audio_hardware_init_direct_sound_listener;
    fn hw_query_3d_sound_effects(this: u32) = thiscall 0x4D8490 => audio_hardware_query_3d_sound_effects;
    fn hw_get_num_available_channels(this: u32) -> u32 = thiscall 0x4D8810 => unported;
    fn hw_allocate_channels(this: u32, n: u32) -> u32 = thiscall 0x5B9340 => unported;
    fn hw_load_sound_bank(this: u32, bank: u32, slot: u32) = thiscall 0x4D88A0 => unported;
    fn hw_load_sound(this: u32, bank: u32, sound: u32, slot: u32) = thiscall 0x4D8ED0 => unported;
    fn streaming_ctor(this: u32, ds: u32, id: u32) -> u32 = thiscall 0x4F1800 => crate::port4_hw::streaming_ctor;
    fn streaming_initialise(this: u32) -> u8 = thiscall 0x4F22F0 => unported;
    fn static_channel_ctor(this: u32, ds: u32, id: u32, is3d: u32, rate: u32, bits: u32) -> u32 = thiscall 0x4F0B10 => unported;
    fn channel_set_volume(this: u32, v: u32) = thiscall 0x4D7C60 => unported;
    fn user_radio_scan(this: u32) -> bool = thiscall 0x4F4BA0 => user_radio_scan_user_tracks;
    fn user_radio_initialise(this: u32) -> u8 = thiscall 0x4F35B0 => unported;
    fn smooth_fade_initialise(this: u32) -> u8 = thiscall 0x4EEEC0 => unported;
    fn smooth_fade_start(this: u32) = thiscall 0x4EEA10 => unported;
    fn stream_thread_initialise(this: u32, ch: u32) -> u8 = thiscall 0x4F1680 => unported;
    fn stream_thread_start(this: u32) = thiscall 0x4F11F0 => unported;
    fn radio_initialise(this: u32, slot: u32) -> u8 = thiscall 0x5B9390 => unported;
    fn sound_manager_initialise(this: u32) -> bool = thiscall 0x5B9690 => sound_manager_initialise_port;
    fn frontend_initialise(this: u32) = thiscall 0x5B9AB0 => frontend_audio_entity_initialise;
    fn engine_set_effects_fader_scaling_factor(this: u32, f: u32) = thiscall 0x506E50 => unported;
    fn utility_static_initialise() = cdecl 0x5B97F0 => audio_utility_static_initialise;
    fn ped_static_initialise() = cdecl 0x5B98A0 => unported;
    fn ped_speech_static_initialise() = cdecl 0x5B98C0 => unported;
    fn vehicle_static_initialise() = cdecl 0x5B99F0 => unported;
    fn explosion_static_initialise() = cdecl 0x5B9A60 => unported;
    fn weather_static_initialise() = cdecl 0x5B9A70 => unported;
    fn door_static_initialise() = cdecl 0x5B9A80 => unported;
    fn fire_static_initialise() = cdecl 0x5B9A90 => unported;
    fn police_scanner_static_initialise() = cdecl 0x5B9C30 => unported;
    fn script_entity_initialise(this: u32) = thiscall 0x5B9B60 => unported;
    fn pedless_speech_initialise(this: u32) = thiscall 0x4E4E10 => unported;
    fn collision_entity_initialise(this: u32) = thiscall 0x5B9BD0 => unported;
    fn ae_random_int(a: i32, b: i32) -> i32 = cdecl 0x4D9C10 => unported;
    fn current_time_ms() -> u64 = cdecl 0x4D9E80 => crate::port4_hw::get_current_time_in_ms;
}

fn local(p: &mut u32) -> u32 {
    GPtr::<u32>::from_raw(p).addr() as u32
}

/// `CLoadingScreen::Pause()` (0x590310): `ret`. `#[native]` (one byte).
#[sa_core::native(addr = 0x590310, conv = cdecl, name = "CLoadingScreen::Pause")]
pub fn loading_screen_pause_native() {}

// ------------------------------------------------------------------ CAudioEngine::Initialise -----

/// `CAudioEngine::Initialise()` (0x5B9C60). `CLoadingScreen::Pause`; the hardware (failure: AL from
/// it). One channel slot is taken inline (`AllocateChannels(1)`: the first free slot of
/// `[0xB5F948]` with the free count `[0xB5F944]`, -1 when none) and stored at +4 for the radio
/// (`CAERadioTrackManager::Initialise(slot)`, failure: false). The cutscene (0x8AE554..) and
/// ambience (0x8AC160..) track managers get their channel, defaults and -1 ids. The sound manager
/// (failure: false); `[0xBD00F8]` = `new[](0xB159)` filled from `AUDIO\CONFIG\EVENTVOL.DAT` (open
/// or short read: false). Frontend entity, effects fader 0, the entity static initialisers, the
/// script / pedless speech / collision entities; +2 = 0xFF, +0 / +1 = 0; `CLoadingScreen::Continue`.
/// AL = 1.
#[hook(addr = 0x5B9C60, conv = thiscall, name = "CAudioEngine::Initialise", expect = "56 8B F1 E8 A8 66 FD FF",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "one-time start-up")]
pub fn audio_engine_initialise(this: u32) -> bool {
    loading_screen_pause();
    if !hw_initialise(HW) {
        return false;
    }
    let mut slot: u32 = 0xFFFF_FFFF;
    if r16(0xB5F944) >= 1 {
        let mut i: u32 = 0;
        loop {
            let c = r16(0xB5F948 + i * 2) as i32;
            if c <= 0 {
                w16(0xB5F948 + i * 2, 1);
                w16(0xB5F944, r16(0xB5F944).wrapping_sub(1));
                slot = i;
                break;
            }
            i = i.wrapping_add(c as u32);
            if (i as i32) >= 0x40 {
                break;
            }
        }
    }
    let slot = slot as u16 as i16 as i32 as u32;
    w32(this + 4, slot);
    if radio_initialise(0x8CB6F8, slot) == 0 {
        return false;
    }
    w32(0x8AE558, r32(this + 4));
    w32(0x8AE55C, 8);
    w8(0x8AE554, 0);
    w8(0x8AE555, 0);
    w8(0x8AE570, 0);
    w32(0x8AE560, 0xFFFF_FFFF);
    w32(0x8AE564, 0xFFFF_FFFF);
    w32(0x8AE568, 0);
    w32(0x8AE56C, 0);
    w32(0x8AC160, r32(this + 4));
    w32(0x8AC164, 8);
    w32(0x8AC174, 0xC2C8_0000);
    w32(0x8AC178, 0x3F80_0000);
    w8(0x8AC184, 3);
    w32(0x8AC17C, 0xFFFF_FFFF);
    w32(0x8AC180, 0xFFFF_FFFF);
    if !sound_manager_initialise(0xB62CB0) {
        return false;
    }
    let ev = operator_new_array(0xB159);
    w32(0xBD00F8, ev);
    let f = file_open(0x86A440, 0x85A53C);
    if f == 0 {
        return false;
    }
    if file_read(f, r32(0xBD00F8), 0xB159) != 0xB159 {
        file_close(f);
        return false;
    }
    file_close(f);
    frontend_initialise(this + 0xB4);
    engine_set_effects_fader_scaling_factor(this, 0);
    utility_static_initialise();
    ped_static_initialise();
    ped_speech_static_initialise();
    vehicle_static_initialise();
    explosion_static_initialise();
    weather_static_initialise();
    door_static_initialise();
    fire_static_initialise();
    police_scanner_static_initialise();
    script_entity_initialise(this + 0x2A0);
    pedless_speech_initialise(this + 0x1E38);
    collision_entity_initialise(this + 0x4BC);
    w8(this + 2, 0xFF);
    w8(this, 0);
    w8(this + 1, 0);
    loading_screen_continue();
    true
}

// ------------------------------------------------------------------ CAEAudioHardware -------------

/// `CAEAudioHardware::Initialise()` (0x4D9930). The MP3 bank loader (+0xD98, `new(0x6E4)`) and track
/// loader (+0xD9C, `new(0x1C)`) are created and initialised (failure: false). `CoInitialize`;
/// `EAXDirectSoundCreate` into +0xDA0 (failure: false); `GetCaps` (+0xDAC, `dwSize` 0x60);
/// `SetCooperativeLevel(hwnd, DSSCL_PRIORITY)` (failure: false); the listener (2 channels, 48 kHz,
/// 16 bit; failure: false); `GetSpeakerConfig` (+0xDA4). The streaming channel (+0xE10,
/// `new(0x60098)`, id 0) is initialised. The channel count (+0x8E) from the free hardware 3D
/// buffers (`dwFreeHw3DAllBuffers`, +0xDE4): at least 24 -> `min(n, 64) - 7` hardware channels
/// (+4 = 1, fade slots `[0xB610E8]` = 7), else 48 software channels (+4 = 0, 48 fade slots); static
/// channels 1.. (+0xE68.., `new(0x90)`, 44.1 kHz 16 bit); channel 0 = the streaming channel
/// (+0xE64), muted (-100 dB); +6 = 0x37. With user-track scanning on (`[0xBA680D]`):
/// `ScanUserTracks`, then wait (100 ms steps) while its state `[0xB6B988]` is 1 (2 / 3 reset to 0).
/// User radio, fade thread (initialised, started), stream thread (initialised on the streaming
/// channel, started); +0x8C = +0x8E; +0 = 1. AL = 1.
#[hook(addr = 0x4D9930, conv = thiscall, name = "CAEAudioHardware::Initialise", expect = "6A FF 68 3C BE 83 00",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "one-time start-up; creates threads")]
pub fn audio_hardware_initialise(this: u32) -> bool {
    let b = operator_new(0x6E4);
    let b = if b != 0 { bank_ctor(b) } else { 0 };
    w32(this + 0xD98, b);
    let t = operator_new(0x1C);
    let t = if t != 0 { track_ctor(t) } else { 0 };
    w32(this + 0xD9C, t);
    if !bank_initialise(r32(this + 0xD98)) || !track_initialise(r32(this + 0xD9C)) {
        return false;
    }
    co_initialize(0);
    if eax_direct_sound_create(0, this + 0xDA0, 0) < 0 {
        return false;
    }
    w32(this + 0xDAC, 0x60);
    ds_get_caps(r32(this + 0xDA0), this + 0xDAC);
    let hwnd = r32(r32(0xC17054));
    if ds_set_cooperative_level(r32(this + 0xDA0), hwnd, 2) < 0 {
        return false;
    }
    if !hw_init_listener(this, 2, 0xBB80, 0x10) {
        return false;
    }
    ds_get_speaker_config(r32(this + 0xDA0), this + 0xDA4);
    let m = operator_new(0x60098);
    let sc = if m != 0 { streaming_ctor(m, r32(this + 0xDA0), 0) } else { 0 };
    w32(this + 0xE10, sc);
    streaming_initialise(sc);
    let free3d = r32(this + 0xDE4);
    if free3d >= 0x18 {
        let n = if free3d < 0x40 { free3d } else { 0x40 };
        w16(this + 0x8E, n.wrapping_sub(7) as u16);
        w8(this + 4, 1);
        w32(0xB610E8, 7);
    } else {
        w16(this + 0x8E, 0x30);
        w8(this + 4, 0);
        w32(0xB610E8, 0x30);
    }
    let mut i: u32 = 1;
    while i < r16(this + 0x8E) as u32 {
        let m = operator_new(0x90);
        let ch = if m != 0 {
            static_channel_ctor(m, r32(this + 0xDA0), i, r8(this + 4) as u32, 0xAC44, 0x10)
        } else {
            0
        };
        w32(this + 0xE68 + (i - 1) * 4, ch);
        i += 1;
    }
    w32(this + 0xE64, r32(this + 0xE10));
    channel_set_volume(r32(this + 0xE10), 0xC2C8_0000);
    w16(this + 6, 0x37);
    if r8(0xBA680D) != 0 {
        user_radio_scan(USER_RADIO);
        loop {
            let s = r32(0xB6B988);
            if (2..=3).contains(&(s as i32)) {
                w32(0xB6B988, 0);
            }
            if s != 1 {
                break;
            }
            sleep(100);
        }
    }
    user_radio_initialise(USER_RADIO);
    smooth_fade_initialise(SMOOTH_FADE);
    smooth_fade_start(SMOOTH_FADE);
    stream_thread_initialise(this + 0xE14, r32(this + 0xE10));
    stream_thread_start(this + 0xE14);
    w16(this + 0x8C, r16(this + 0x8E));
    w8(this, 1);
    true
}

/// `CAEAudioHardware::InitDirectSoundListener(channels, rate, bits)` (0x4D9640, `ret 0xC`). A
/// primary buffer (`DSBCAPS_PRIMARYBUFFER | DSBCAPS_CTRL3D`); its format PCM (`channels`, `rate`,
/// `bits`; block `bits / 8 * channels`); its `IDirectSound3DListener` into +0xE0C (failure: the
/// primary released, false): position (0,0,0), orientation front (0,1,0) top (0,0,-1), roll-off 0,
/// Doppler 0, all deferred, then committed; the primary released; `Query3DSoundEffects`. AL = 1.
#[hook(addr = 0x4D9640, conv = thiscall, name = "CAEAudioHardware::InitDirectSoundListener", expect = "83 EC 3C 53 8B D9",
       unverified = "std4-audio-2", route = "L3-only", hazards = "none", note = "one-time start-up")]
pub fn audio_hardware_init_direct_sound_listener(this: u32, channels: u32, rate: u32, bits: u32) -> bool {
    let ds = r32(this + 0xDA0);
    if ds == 0 {
        return false;
    }
    let mut prim: u32 = 0;
    let mut desc: [u32; 9] = [0x24, 0x11, 0, 0, 0, 0, 0, 0, 0];
    if ds_create_sound_buffer(ds, GPtr::<u32>::from_raw(desc.as_mut_ptr()).addr() as u32, local(&mut prim), 0) < 0 {
        return false;
    }
    let ch = channels as u16 as u32;
    let align = ((bits as u16) >> 3) as u32 * ch;
    let align = align as u16 as u32;
    let mut wfx: [u32; 5] = [0; 5];
    let wp = GPtr::<u32>::from_raw(wfx.as_mut_ptr()).addr() as u32;
    w16(wp, 1);
    w16(wp + 2, ch as u16);
    w32(wp + 4, rate);
    w32(wp + 8, align.wrapping_mul(rate));
    w16(wp + 0xC, align as u16);
    w16(wp + 0xE, bits as u16);
    w16(wp + 0x10, 0);
    if buf_set_format(prim, wp) < 0 {
        return false;
    }
    if com_query_interface(prim, 0x8593A0, this + 0xE0C) < 0 {
        if prim != 0 {
            com_release(prim);
        }
        return false;
    }
    let l = r32(this + 0xE0C);
    li_set_position(l, 0, 0, 0, 1);
    let l = r32(this + 0xE0C);
    li_set_orientation(l, 0, 0x3F80_0000, 0, 0, 0, 0xBF80_0000, 1);
    let l = r32(this + 0xE0C);
    li_set_rolloff_factor(l, 0, 1);
    let l = r32(this + 0xE0C);
    li_set_doppler_factor(l, 0, 1);
    let l = r32(this + 0xE0C);
    li_commit(l);
    if prim != 0 {
        com_release(prim);
    }
    hw_query_3d_sound_effects(this);
    true
}

orig! {
    fn buf_set_format_raw(b: GPtr<u8>, wfx: u32) -> i32 = stdcall vslot 0x38 => unported;
    fn li_set_position_raw(l: GPtr<u8>, x: u32, y: u32, z: u32, apply: u32) -> i32 = stdcall vslot 0x38 => unported;
    fn li_set_orientation_raw(l: GPtr<u8>, xf: u32, yf: u32, zf: u32, xt: u32, yt: u32, zt: u32, apply: u32) -> i32 = stdcall vslot 0x34 => unported;
    fn li_set_rolloff_factor_raw(l: GPtr<u8>, f: u32, apply: u32) -> i32 = stdcall vslot 0x3C => unported;
    fn li_set_doppler_factor_raw(l: GPtr<u8>, f: u32, apply: u32) -> i32 = stdcall vslot 0x30 => unported;
    fn li_commit_raw(l: GPtr<u8>) -> i32 = stdcall vslot 0x44 => unported;
    fn ks_set_raw(ks: GPtr<u8>, guid: u32, id: u32, inst: u32, inst_len: u32, data: u32, len: u32) -> i32 = stdcall vslot 0x10 => unported;
}

/// `CAEAudioHardware::Query3DSoundEffects()` (0x4D8490): a temporary 0x400-byte mono 16-bit
/// 44.1 kHz 3D buffer, its `IDirectSound3DBuffer` and from that the `IKsPropertySet`; with it the
/// EAX properties (environment 2 = 0x85EFEC's value, 2 / -1000 room settings) are tried: all
/// accepted -> +0xDA8 = 1 (EAX). No property set (or a failed property) -> +0xDA8 = 2 unless set.
/// Everything is released. (Any failure creating the buffer or its 3D interface returns at once.)
#[hook(addr = 0x4D8490, conv = thiscall, name = "CAEAudioHardware::Query3DSoundEffects", expect = "83 EC 5C A1 50 91 85 00",
       unverified = "std4-audio-2", route = "L3-only", hazards = "none", note = "one-time start-up")]
pub fn audio_hardware_query_3d_sound_effects(this: u32) {
    let mut wfx: [u32; 5] = [0; 5];
    let wp = GPtr::<u32>::from_raw(wfx.as_mut_ptr()).addr() as u32;
    w16(wp, 1);
    w16(wp + 2, 1);
    w32(wp + 4, 0xAC44);
    w32(wp + 8, 0x15888);
    w16(wp + 0xC, 2);
    w16(wp + 0xE, 0x10);
    w16(wp + 0x10, 0);
    let mut desc: [u32; 9] = [0x24, 0x10, 0x400, 0, wp, r32(0x859150), r32(0x859154), r32(0x859158), r32(0x85915C)];
    let mut buf: u32 = 0;
    let mut b3: u32 = 0;
    let mut ks: u32 = 0;
    if ds_create_sound_buffer(r32(this + 0xDA0), GPtr::<u32>::from_raw(desc.as_mut_ptr()).addr() as u32, local(&mut buf), 0) < 0 {
        return;
    }
    if com_query_interface(buf, 0x859390, local(&mut b3)) < 0 {
        return;
    }
    if buf != 0 {
        if b3 != 0 {
            if com_query_interface(b3, 0x859340, local(&mut ks)) == 0 {
                // property 2 of set 0x85EFEC = the 16-byte GUID 0x85EFDC (copied to the stack),
                // then properties 2 = 2 and 5 = -1000 of set 0x85EFDC
                let mut guid: [u32; 4] = [r32(0x85EFDC), r32(0x85EFE0), r32(0x85EFE4), r32(0x85EFE8)];
                let mut two: u32 = 2;
                let mut minus: u32 = 0xFFFF_FC18;
                let gp = GPtr::<u32>::from_raw(guid.as_mut_ptr()).addr() as u32;
                if ks_set(ks, 0x85EFEC, 2, 0, 0, gp, 0x10) >= 0
                    && ks_set(ks, 0x85EFDC, 2, 0, 0, local(&mut two), 4) >= 0
                    && ks_set(ks, 0x85EFDC, 5, 0, 0, local(&mut minus), 4) >= 0
                {
                    w8(this + 0xDA8, 1);
                }
                if ks != 0 {
                    com_release(ks);
                }
            }
            if r8(this + 0xDA8) == 0 {
                w8(this + 0xDA8, 2);
            }
            if b3 != 0 {
                com_release(b3);
            }
        }
        if buf != 0 {
            com_release(buf);
        }
    }
}

// ------------------------------------------------------------------ bank / track loaders ---------

/// The bank loader's request queue reset (0x4DFB60, `this` in ECX, ECX kept): 50 requests (0x20
/// each from +0x24: +0 / +0x10 = 0, +0x1C / +0x1E / +0x20 = -1), +0x664..+0x668 = 0. `#[native]`:
/// its caller calls `LoadBankSlotFile` next without reloading ECX.
#[sa_core::native(addr = 0x4DFB60, conv = thiscall, name = "FUN_004dfb60")]
pub fn bank_loader_reset_requests(this: u32) {
    let mut a = this + 0x3E;
    for _ in 0..0x32 {
        w16(a - 2, 0xFFFF);
        w16(a, 0xFFFF);
        w16(a + 2, 0xFFFF);
        w32(a - 0x1A, 0);
        w32(a - 6, 0);
        a += 0x20;
    }
    w16(this + 0x664, 0);
    w16(this + 0x666, 0);
    w16(this + 0x668, 0);
}

/// `CAEMP3BankLoader::Initialise()` (0x4E08F0): request reset, then the bank slot, bank lookup and
/// SFX pak lookup files; all loaded -> +0x14 = 1, AL = 1. (The original's ECX for
/// `LoadBankSlotFile` is the reset's preserved `this`.)
#[hook(addr = 0x4E08F0, conv = thiscall, name = "CAEMP3BankLoader::Initialise", expect = "56 8B F1 E8 68 F2 FF FF",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "one-time start-up")]
pub fn mp3_bank_loader_initialise(this: u32) -> bool {
    bank_reset_requests(this);
    if !bank_load_slot_file(this) || !bank_load_lookup_file(this) || !bank_load_sfx_pak_lookup_file(this) {
        return false;
    }
    w8(this + 0x14, 1);
    true
}

/// `CAEBankLoader::LoadBankSlotFile()` (0x4E0590): `AUDIO\CONFIG\BANKSLOT.DAT` (0x85F15C, mode
/// 0x859C6C); the slot count (u16 at +0xC), `new(count * 0x12D4)` slots (+0) read; a short read
/// frees them (+0 = NULL). Offsets computed; the buffer size (+0x18) = last slot's offset + size;
/// `CMemoryMgr::Malloc(size)` -> +0x1C. AL = success.
#[hook(addr = 0x4E0590, conv = thiscall, name = "CAEBankLoader::LoadBankSlotFile", expect = "56 57 68 6C 9C 85 00 68 5C F1 85 00",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "file loading")]
pub fn bank_loader_load_bank_slot_file(this: u32) -> bool {
    let f = file_open(0x85F15C, 0x859C6C);
    if (f as i32) <= 0 {
        return false;
    }
    let size = file_total_size(f);
    if size <= 2 {
        file_close(f);
        return false;
    }
    file_read(f, this + 0xC, 2);
    let n = r16(this + 0xC) as i16 as i32;
    let m = operator_new((n as u32).wrapping_mul(0x12D4));
    w32(this, m);
    if m == 0 {
        file_close(f);
        return false;
    }
    let got = file_read(f, m, size - 2);
    if size - 2 != got {
        file_close(f);
        crt_free(r32(this));
        w32(this, 0);
        return false;
    }
    file_close(f);
    bank_calculate_offsets(this);
    let n = r16(this + 0xC) as i16 as i32;
    let last = r32(this).wrapping_add((n as u32).wrapping_mul(0x12D4)).wrapping_sub(0x12D4);
    let total = r32(last + 4).wrapping_add(r32(last));
    w32(this + 0x18, total);
    w32(this + 0x1C, memory_malloc(total, 0));
    true
}

/// `CAEBankLoader::LoadBankLookupFile()` (0x4DFBD0): `AUDIO\CONFIG\BANKLKUP.DAT` (0x85F118): the
/// entry count (`size / 12`, +0xE), `new(count * 12)` (+4) read; a short read frees it (+4 = NULL).
#[hook(addr = 0x4DFBD0, conv = thiscall, name = "CAEBankLoader::LoadBankLookupFile", expect = "53 57 68 6C 9C 85 00 68 18 F1 85 00",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "file loading")]
pub fn bank_loader_load_bank_lookup_file(this: u32) -> bool {
    let f = file_open(0x85F118, 0x859C6C);
    if (f as i32) <= 0 {
        return false;
    }
    let size = file_total_size(f);
    if size == 0 {
        file_close(f);
        return false;
    }
    let n = (size / 12) as u16;
    w16(this + 0xE, n);
    let m = operator_new((n as i16 as i32 as u32).wrapping_mul(12));
    w32(this + 4, m);
    if file_read(f, m, size) != size {
        crt_free(r32(this + 4));
        w32(this + 4, 0);
        file_close(f);
        return false;
    }
    file_close(f);
    true
}

/// `CAEBankLoader::LoadSFXPakLookupFile()` (0x4DFC70): `AUDIO\CONFIG\PAKFILES.DAT` (0x85F140): the
/// pak count (`size / 52`, +0x10), names `new(count * 52)` (+8) read (short read: freed, +8 =
/// NULL, false); stream handles `new(count * 4)` (+0x20): each `AUDIO\SFX\` + name opened with
/// `CdStreamOpen(path, 0)`; the names freed (+8 = NULL). AL = 1.
#[hook(addr = 0x4DFC70, conv = thiscall, name = "CAEBankLoader::LoadSFXPakLookupFile", expect = "81 EC 84 00 00 00 55 57",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "file loading")]
pub fn bank_loader_load_sfx_pak_lookup_file(this: u32) -> bool {
    let f = file_open(0x85F140, 0x859C6C);
    if (f as i32) <= 0 {
        return false;
    }
    let size = file_total_size(f);
    if size == 0 {
        file_close(f);
        return false;
    }
    let n = (size / 52) as u16;
    w16(this + 0x10, n);
    let m = operator_new((n as i16 as i32 as u32).wrapping_mul(0x34));
    w32(this + 8, m);
    let got = file_read(f, m, size);
    file_close(f);
    if size != got {
        crt_free(r32(this + 8));
        w32(this + 8, 0);
        return false;
    }
    let h = operator_new((r16(this + 0x10) as i16 as i32 as u32) << 2);
    w32(this + 0x20, h);
    let mut i: i32 = 0;
    let mut off: u32 = 0;
    while i < r16(this + 0x10) as i16 as i32 {
        // "AUDIO\SFX\" (0x85F134..0x85F13E, 11 bytes) + the pak name, on the stack
        let mut path = [0u8; 0x80];
        for (k, b) in path.iter_mut().enumerate().take(11) {
            *b = r8(0x85F134 + k as u32);
        }
        let name = r32(this + 8).wrapping_add(off);
        let mut len = 0usize;
        while r8(name + len as u32) != 0 {
            len += 1;
        }
        let start = path.iter().position(|c| *c == 0).unwrap_or(10);
        for k in 0..=len {
            if start + k < path.len() {
                path[start + k] = r8(name + k as u32);
            }
        }
        let pp = GPtr::<u8>::from_raw(path.as_mut_ptr()).addr() as u32;
        let s = cd_stream_open(pp, 0);
        w32(r32(this + 0x20) + 4 * i as u32, s);
        off = off.wrapping_add(0x34);
        i += 1;
    }
    crt_free(r32(this + 8));
    w32(this + 8, 0);
    true
}

/// `CAEMP3TrackLoader::Initialise()` (0x4E0C50): the stream pack and track lookup tables (failure:
/// false); +0x14 = 0; a path buffer `new(strlen(+0x10 name...) + 0x2F)`: `AUDIO\STREAMS\` + the
/// first pack name, opened with `CFileMgr::OpenFile` -> found locally (+0x14 = 0); else
/// `getDvdGamePath()` (+0x18; NULL: false) + `AUDIO\STREAMS\` + name -> on DVD (+0x14 = 1;
/// failure: false). The file closed, the buffer freed, +0 = 1. AL = 1.
#[hook(addr = 0x4E0C50, conv = thiscall, name = "CAEMP3TrackLoader::Initialise", expect = "57 8B F9 E8 18 FD FF FF",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "file loading")]
pub fn mp3_track_loader_initialise(this: u32) -> bool {
    if !track_load_stream_pack_table(this) || !track_load_lookup_table(this) {
        return false;
    }
    w8(this + 0x14, 0);
    let n = lstrlen_a(r32(this + 0x10));
    let buf = operator_new((n as u32).wrapping_add(0x2F));
    lstrcpy_a(buf, 0x85F1BC);
    lstrcat_a(buf, r32(this + 0x10));
    let mut f = file_open(buf, 0x85A53C);
    if (f as i32) > 0 {
        w8(this + 0x14, 0);
    } else {
        let dvd = get_dvd_game_path();
        w32(this + 0x18, dvd);
        if dvd == 0 {
            crt_free(buf);
            return false;
        }
        lstrcpy_a(buf, dvd);
        lstrcat_a(buf, 0x85F1BC);
        lstrcat_a(buf, r32(this + 0x10));
        f = file_open(buf, 0x85A53C);
        if (f as i32) <= 0 {
            crt_free(buf);
            return false;
        }
        w8(this + 0x14, 1);
    }
    file_close(f);
    crt_free(buf);
    w8(this, 1);
    true
}

/// `CAEMP3TrackLoader::LoadStreamPackTable()` (0x4E0970): `AUDIO\CONFIG\STRMPAKS.DAT` (0x85F184)
/// read whole with `CreateFileA` / `ReadFile` into `CMemoryMgr::Malloc(size)` (+0x10); pack count
/// `size / 16` (+8). Open failure: false.
#[hook(addr = 0x4E0970, conv = thiscall, name = "CAEMP3TrackLoader::LoadStreamPackTable", expect = "51 56 57 6A 00 6A 00 6A 03",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "file loading")]
pub fn mp3_track_loader_load_stream_pack_table(this: u32) -> bool {
    let h = create_file_a(0x85F184, 0x8000_0000, 1, 0, 3, 0, 0);
    if h == 0xFFFF_FFFF {
        return false;
    }
    let size = get_file_size(h, 0);
    let m = memory_malloc(size, 0);
    w32(this + 0x10, m);
    let mut read: u32 = 0;
    read_file(h, m, size, local(&mut read), 0);
    close_handle(h);
    w32(this + 8, size >> 4);
    true
}

/// `CAEMP3TrackLoader::LoadTrackLookupTable()` (0x4E09F0): `AUDIO\CONFIG\TRAKLKUP.DAT` (0x85F1A0)
/// into `CMemoryMgr::Malloc(size)` (+0xC); track count `size / 12` (+4).
#[hook(addr = 0x4E09F0, conv = thiscall, name = "CAEMP3TrackLoader::LoadTrackLookupTable", expect = "51 53 56 6A 00 6A 00 6A 03",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "file loading")]
pub fn mp3_track_loader_load_track_lookup_table(this: u32) -> bool {
    let h = create_file_a(0x85F1A0, 0x8000_0000, 1, 0, 3, 0, 0);
    if h == 0xFFFF_FFFF {
        return false;
    }
    let size = get_file_size(h, 0);
    let m = memory_malloc(size, 0);
    w32(this + 0xC, m);
    let mut read: u32 = 0;
    read_file(h, m, size, local(&mut read), 0);
    w32(this + 4, size / 12);
    close_handle(h);
    true
}

/// `getDvdGamePath()` (0x747300): with critical-error boxes off, the logical drives are walked; a
/// CD-ROM drive (type 5) whose volume label equals the 16 bytes at 0x874B08 gives a new string
/// with its root (`new[](len + 0x21)`); NULL when none. The drive list is freed.
#[hook(addr = 0x747300, conv = cdecl, name = "getDvdGamePath", expect = "81 EC 10 01 00 00 53 55",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "OS query")]
pub fn get_dvd_game_path_port() -> u32 {
    let mut result: u32 = 0;
    set_error_mode(1);
    let n = get_logical_drive_strings_a(0, 0);
    let list = operator_new_array(n);
    let n = get_logical_drive_strings_a(n, list);
    let mut root = [0u8; 0x10];
    let mut label = [0u8; 0x100];
    let mut j = 0usize;
    let mut i = 0u32;
    while i < n {
        let c = r8(list + i);
        if j < root.len() {
            root[j] = c;
        }
        j += 1;
        if c == 0 {
            let rp = GPtr::<u8>::from_raw(root.as_mut_ptr()).addr() as u32;
            if get_drive_type_a(rp) == 5 {
                let lp = GPtr::<u8>::from_raw(label.as_mut_ptr()).addr() as u32;
                if get_volume_information_a(rp, lp, 0xFF, 0, 0, 0, 0, 0) != 0
                    && (0..0x10).all(|k| label[k] == r8(0x874B08 + k as u32))
                {
                    let len = lstrlen_a(rp);
                    let s = operator_new_array((len as u32).wrapping_add(0x21));
                    lstrcpy_a(s, rp);
                    result = s;
                    break;
                }
            }
            j = 0;
        }
        i += 1;
    }
    crt_free_array(list);
    result
}

// ------------------------------------------------------------------ managers / entities ----------

/// `CAESoundManager::Initialise()` (0x5B9690). The channel count: available channels (more than 10,
/// at most 300) -> +0 (u16); `AllocateChannels` -> +2 (-1: false; 10 or fewer: false). Per channel a
/// `CAESound*` slot array (`new[](count * 2 + 4)` with the count in front; entries constructed by
/// the vector iterator: 2-byte elements, constructor 0x5B9330) -> +0x87F4, and two more `count * 2`
/// arrays -> +0x87F8 / +0x87FC. The 300 sounds' +0x5C / +0x62 cleared; every channel's sound -1; the
/// time `CTimer` -> +0x8CB0, +0x8CB4 / +0x8CB8 / +0x8CB5 = 0; the debug print (0x86A41C). AL = 1.
#[hook(addr = 0x5B9690, conv = thiscall, name = "CAESoundManager::Initialise", expect = "6A FF 68 7B D5 83 00",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "one-time start-up")]
pub fn sound_manager_initialise_port(this: u32) -> bool {
    let n = hw_get_num_available_channels(HW) as u16;
    if n <= 10 {
        return false;
    }
    let n = if n < 0x12C { n } else { 0x12C };
    w16(this, n);
    let got = hw_allocate_channels(HW, n as u32) as u16;
    w16(this + 2, got);
    if got == 0xFFFF {
        return false;
    }
    let count = r16(this) as u32;
    let m = operator_new_array(count * 2 + 4);
    let arr = if m != 0 {
        w32(m, count);
        // `eh vector constructor iterator(m + 4, 2, count, 0x5B9330, 0x4EF070)`: the element
        // constructor 0x5B9330 is `mov eax, ecx; ret` (writes nothing), so its calls are not repeated
        m + 4
    } else {
        0
    };
    w32(this + 0x87F4, arr);
    w32(this + 0x87F8, operator_new_array(r16(this) as u32 * 2));
    w32(this + 0x87FC, operator_new_array(r16(this) as u32 * 2));
    let mut a = this + 0x62;
    for _ in 0..0x12C {
        w16(a - 6, 0);
        w16(a, 0);
        a += 0x74;
    }
    let mut i: i32 = 0;
    while i < r16(this) as i32 {
        w16(r32(this + 0x87F4) + 2 * i as u32, 0xFFFF);
        i += 1;
    }
    w32(this + 0x8CB0, r32(0xB7CB84));
    w8(this + 0x8CB4, 0);
    w32(this + 0x8CB8, 0);
    w8(this + 0x8CB5, 0);
    crt_printf(0x86A41C);
    true
}

/// `CAEFrontendAudioEntity::Initialise()` (0x5B9AB0): state cleared (+0x84 / +0x88 / +0x8C / +0x80
/// / +0x90 / +0x94 / +0x7C / +0x98; +0x82 = 0x1C, +0x7E = -1); sound banks 0x3C (slot 1), 0x3B
/// (slot 0), 0x1C (slot 0x2A); a random 0..3 (+0x7D); bank 0x52 sounds `2r` (slot 2) and `2r + 1`
/// (slot 5). The second sound's dword argument has the original's upper ECX bits replaced by 0
/// (the callee reads 16 bits).
#[hook(addr = 0x5B9AB0, conv = thiscall, name = "CAEFrontendAudioEntity::Initialise", expect = "53 33 DB 56 8B F1 6A 01",
       unverified = "std4-audio-2", route = "L3-only", hazards = "heap-writes", note = "one-time start-up")]
pub fn frontend_audio_entity_initialise(this: u32) {
    w32(this + 0x84, 0);
    w32(this + 0x88, 0);
    w32(this + 0x8C, 0);
    w16(this + 0x80, 0);
    w16(this + 0x82, 0x1C);
    w16(this + 0x7E, 0xFFFF);
    w32(this + 0x90, 0);
    w32(this + 0x94, 0);
    w8(this + 0x7C, 0);
    w32(this + 0x98, 0);
    hw_load_sound_bank(HW, 0x3C, 1);
    hw_load_sound_bank(HW, 0x3B, 0);
    hw_load_sound_bank(HW, 0x1C, 0x2A);
    let r = ae_random_int(0, 3);
    w8(this + 0x7D, r as u8);
    let s0 = ((r as u8 as i8 as i16 as u16 as u32) << 1) | (r as u32 & 0xFFFF_0000);
    hw_load_sound(HW, 0x52, s0, 2);
    let v = r8(this + 0x7D) as i8 as i16 as u16 as u32;
    hw_load_sound(HW, 0x52, (v * 2 + 1) & 0xFFFF, 5);
}

/// `CAEAudioUtility::StaticInitialise()` (0x5B97F0): `[0xB61100]` = 0, `[0xB61104]` = -100.0,
/// `[0xB61108]` = 1e-5; for k = 47 down to 0 the table at 0xB61110 (8 bytes per entry):
/// `v = 10.0 / 1.2^k` (stored f32) and `log10(v)` (stored to `[0xB6110C]` each time, as the
/// bytes do); `QueryPerformanceFrequency` -> 0xB610F0 (failure: `GetLastError`); the start time
/// cleared, then `GetCurrentTimeInMS()` -> 0xB610F8.
#[hook(addr = 0x5B97F0, conv = cdecl, name = "CAEAudioUtility::StaticInitialise", expect = "51 56 57 C7 05 00 11 B6 00 00 00 00 00",
       unverified = "std4-audio-2", route = "L3-only", hazards = "x87-transcendental", note = "one-time start-up")]
pub fn audio_utility_static_initialise() {
    w32(0xB61100, 0);
    w32(0xB61104, 0xC2C8_0000);
    w32(0xB61108, 0x3727_C5AC);
    let base = f64::from_bits(((r32(0x86A43C) as u64) << 32) | r32(0x86A438) as u64);
    let mut k: i32 = 0x2F;
    let mut e = 0xB61110u32;
    for _ in 0..0x30 {
        let pw = crate::port4_hw::ci_pow(base, xi(k));
        let v = xf(10.0).fdiv(pw);
        e += 8;
        k -= 1;
        w32(e - 8, v.to_f32().to_bits());
        let l = X87::lg2().fyl2x(v);
        w32(0xB6110C, l.to_f32().to_bits());
    }
    if query_performance_frequency(0xB610F0) == 0 {
        get_last_error();
    }
    w32(0xB610F8, 0);
    w32(0xB610FC, 0);
    let t = current_time_ms();
    w32(0xB610F8, t as u32);
    w32(0xB610FC, (t >> 32) as u32);
}

/// `CAEUserRadioTrackManager::ScanUserTracks()` (0x4F4BA0): by the scan state (+0x18, jump table
/// 0x4F4BDC): 0 -> state 1 and the scanning thread (`WriteUserTracksThread` 0x4F4A20, id +0x10,
/// handle +0x14), AL = 0; 1 -> AL = 0; 2 / 3 -> AL = 1; other -> AL = 0. (Phase B has no
/// user-track scanning thread: sa-game's `CreateThread` returns NULL for that entry and the state
/// stays 1; the game only scans with the "user tracks automatic scan" option on.)
#[hook(addr = 0x4F4BA0, conv = thiscall, name = "CAEUserRadioTrackManager::ScanUserTracks", expect = "56 8B F1 8B 4E 18 32 C0",
       unverified = "std4-audio-2", route = "L2", hazards = "none")]
pub fn user_radio_scan_user_tracks(this: u32) -> bool {
    match r32(this + 0x18) {
        0 => {
            w32(this + 0x18, 1);
            let h = create_thread(0, 0, 0x4F4A20, this, 0, this + 0x10);
            w32(this + 0x14, h);
            false
        }
        2 | 3 => true,
        _ => false,
    }
}

orig! {
    fn load_library_a(name: u32) -> u32 = stdcall cell 0x858070 => unported;
    fn get_proc_address(m: u32, name: u32) -> u32 = stdcall cell 0x85806C => unported;
    fn free_library(m: u32) -> i32 = stdcall cell 0x858110 => unported;
    /// `WMCreateSyncReader(unk, rights, &reader)` through the pointer `GetProcAddress` returned
    fn call_create_sync_reader(f: u32, unk: u32, rights: u32, out: u32) -> i32 = stdcall dyn => unported;
    fn com_release_raw(obj: GPtr<u8>) -> u32 = stdcall vslot 0x08 => unported;
}

/// `CAEWMADecoder::InitLibrary()` (0x502B80): already loaded (`[0xB6BAD8]`) -> AL = 1. Else
/// `LoadLibraryA("wmvcore.dll")` -> `[0xB6BAD4]` (-1: AL = 0), `GetProcAddress("WMCreateSyncReader")`
/// -> `[0xB6BAD8]` (NULL: the library freed, AL = 0); a test reader is created (failure: the
/// pointer cleared, AL = 0) and released; AL = 1. (Phase B has no wmvcore.dll: `LoadLibraryA`
/// returns NULL, `GetProcAddress` NULL, so user WMA tracks are unavailable, AL = 0.)
#[hook(addr = 0x502B80, conv = cdecl, name = "CAEWMADecoder::InitLibrary", expect = "51 A1 D8 BA B6 00",
       unverified = "std4-audio-2", route = "L3-only", hazards = "none", note = "OS library probe")]
pub fn wma_decoder_init_library() -> bool {
    if r32(0xB6BAD8) != 0 {
        return true;
    }
    let h = load_library_a(0x862DC4);
    w32(0xB6BAD4, h);
    if h == 0xFFFF_FFFF {
        return false;
    }
    let f = get_proc_address(h, 0x862DB0);
    w32(0xB6BAD8, f);
    if f == 0 {
        free_library(r32(0xB6BAD4));
        return false;
    }
    let mut reader: u32 = 0;
    if call_create_sync_reader(f, 0, 0, local(&mut reader)) < 0 {
        w32(0xB6BAD8, 0);
        return false;
    }
    com_release_raw(GPtr::from_addr(reader as usize));
    true
}

orig! {
    /// The Media Foundation loader's startup (0x8195C0; AX = HRESULT-like result)
    fn mf_startup(flags: u32) -> u32 = cdecl 0x8195C0 => unported;
    /// `MFStartup` thunk (0x819820)
    fn mf_startup2() -> u32 = cdecl 0x819820 => unported;
}

/// `CAEMFDecoder::InitLibrary()` (0x4E7C70): once (`[0xB61D70]`): the loader's startup (0x8195C0(0))
/// and `MFStartup` (0x819820), AX results; both 0 -> flag set. AL = (result == 0); already
/// initialised -> AL = 1.
#[hook(addr = 0x4E7C70, conv = cdecl, name = "CAEMFDecoder::InitLibrary", expect = "A0 70 1D B6 00 84 C0",
       unverified = "std4-audio-2", route = "L3-only", hazards = "none", note = "OS library probe")]
pub fn mf_decoder_init_library() -> bool {
    let mut r: u32 = 0;
    if r8(0xB61D70) == 0 {
        r = mf_startup(0);
        if r as u16 == 0 {
            r = mf_startup2();
        }
        if r as u16 == 0 {
            w8(0xB61D70, 1);
            r = 0;
        }
    }
    r as u16 == 0
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_MF_DECODER_INIT_LIBRARY,
    HOOK_WMA_DECODER_INIT_LIBRARY,
    HOOK_AUDIO_ENGINE_INITIALISE,
    HOOK_AUDIO_HARDWARE_INITIALISE,
    HOOK_AUDIO_HARDWARE_INIT_DIRECT_SOUND_LISTENER,
    HOOK_AUDIO_HARDWARE_QUERY_3D_SOUND_EFFECTS,
    HOOK_MP3_BANK_LOADER_INITIALISE,
    HOOK_BANK_LOADER_LOAD_BANK_SLOT_FILE,
    HOOK_BANK_LOADER_LOAD_BANK_LOOKUP_FILE,
    HOOK_BANK_LOADER_LOAD_SFX_PAK_LOOKUP_FILE,
    HOOK_MP3_TRACK_LOADER_INITIALISE,
    HOOK_MP3_TRACK_LOADER_LOAD_STREAM_PACK_TABLE,
    HOOK_MP3_TRACK_LOADER_LOAD_TRACK_LOOKUP_TABLE,
    HOOK_GET_DVD_GAME_PATH_PORT,
    HOOK_SOUND_MANAGER_INITIALISE_PORT,
    HOOK_FRONTEND_AUDIO_ENTITY_INITIALISE,
    HOOK_AUDIO_UTILITY_STATIC_INITIALISE,
    HOOK_USER_RADIO_SCAN_USER_TRACKS,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_LOADING_SCREEN_PAUSE_NATIVE, NATIVE_BANK_LOADER_RESET_REQUESTS];

// thin wrappers over the `vslot` declarations (they take the object as a `GPtr`)
#[inline]
fn buf_set_format(b: u32, wfx: u32) -> i32 {
    buf_set_format_raw(GPtr::from_addr(b as usize), wfx)
}
#[inline]
fn li_set_position(l: u32, x: u32, y: u32, z: u32, apply: u32) -> i32 {
    li_set_position_raw(GPtr::from_addr(l as usize), x, y, z, apply)
}
#[inline]
#[allow(clippy::too_many_arguments)]
fn li_set_orientation(l: u32, xf: u32, yf: u32, zf: u32, xt: u32, yt: u32, zt: u32, apply: u32) -> i32 {
    li_set_orientation_raw(GPtr::from_addr(l as usize), xf, yf, zf, xt, yt, zt, apply)
}
#[inline]
fn li_set_rolloff_factor(l: u32, f: u32, apply: u32) -> i32 {
    li_set_rolloff_factor_raw(GPtr::from_addr(l as usize), f, apply)
}
#[inline]
fn li_set_doppler_factor(l: u32, f: u32, apply: u32) -> i32 {
    li_set_doppler_factor_raw(GPtr::from_addr(l as usize), f, apply)
}
#[inline]
fn li_commit(l: u32) -> i32 {
    li_commit_raw(GPtr::from_addr(l as usize))
}
#[inline]
fn ks_set(ks: u32, guid: u32, id: u32, inst: u32, inst_len: u32, data: u32, len: u32) -> i32 {
    ks_set_raw(GPtr::from_addr(ks as usize), guid, id, inst, inst_len, data, len)
}
