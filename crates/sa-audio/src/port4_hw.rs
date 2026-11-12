//! STD-4 audio lane, part 1: the sound-output layer the native boot needs: `CAEAudioChannel`,
//! `CAEStaticChannel`, `CAEStreamingChannel`, `CAESmoothFadeThread` and `CAEStreamThread`, ported
//! from the 1.0 US bytes (gta-reversed `Audio/Hardware`, `AESmoothFadeThread.cpp`,
//! `AEStreamThread.cpp` as reading aids; the bytes win). DirectSound is called through `orig!`
//! `vslot` declarations exactly as the original does (phase B: sa-game's native DirectSound,
//! `crates/sa-game/src/dsound.rs`).
//!
//! Threads: `CAESmoothFadeThread::SmoothFadeProc` (0x4EEE90) and `CAEStreamThread::MainLoop`
//! (0x4F15C0) are the entry points the original passes to `CreateThread`. Phase B runs them on
//! real host threads (sa-game's `CreateThread` maps those two entry addresses to
//! [`smooth_fade_proc`] / [`stream_thread_main_loop`]; `ResumeThread` starts them), as the original
//! does: they share the audio singletons with the game thread, guarded only where the original
//! guards (the stream thread's critical section).
//!
//! `CAEAudioChannel` layout (0x60 bytes base): +0 vtable, +4 `IDirectSound8*`, +8
//! `IDirectSoundBuffer*`, +0xC `IDirectSound3DBuffer*`, +0x24 buffer bytes, +0x28, +0x2C sample
//! bytes, +0x34 volume (dB, f32), +0x38 "stopped by frequency 0" flag, +0x3A channel id (u16), +0x3C
//! frequency, +0x40 original frequency, +0x44 looping, +0x45, +0x47 `WAVEFORMATEX` (packed: +0x47
//! tag, +0x49 channels, +0x4B rate, +0x4F bytes/s, +0x53 block align, +0x55 bits, +0x57 cbSize),
//! +0x5C buffer status. `CAEStaticChannel`: +0x60 needs-loop-fill, +0x61 play-from-offset, +0x62
//! pending play, +0x64 loop start offset, +0x68 loop end offset, +0x6C loop repeats, +0x70 play
//! start time, +0x74 loop fill time (ms), +0x78 / +0x7C fill offset / loop length, +0x80 loops
//! filled (u16), +0x84 sample data, +0x88 / +0x8C sound ids (u16), +0x8A "is 3D" byte.
//! `CAEStreamingChannel`: +0x60 initialised, +0x61 looping, +0x62 / +0x63 next-stream switches,
//! +0x64 silence flag, +0x65 FX on, +0x66 / +0x67 half markers, +0x68 decode buffer (+0x6C, 0x60000
//! bytes), +0x6006C stream decoder, +0x60070 next decoder, +0x60074 state (-1 playing, -2 ready,
//! -3 starting, -4 ending, -5 stopping, -6 stopped, -7 paused), +0x60078 decoded play time, +0x6007C
//! play time, +0x60084 stop counter, +0x60088 time stamp (u64), +0x60090 silence buffer, +0x60094.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::g;

/// No globals of its own (singletons are addressed directly).
pub const GLOBALS: &[GlobalDesc] = &[];

/// `AESmoothFadeThread` (`CAESmoothFadeThread`, 0xB608D0).
pub const SMOOTH_FADE: u32 = 0xB608D0;
/// `g_numSoundChannelsUsed` (the count of live DirectSound buffers).
const CHANNELS_USED: u32 = 0xB5F898;
/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: u32 = 0xB7CB84;
/// `AEUserRadioTrackManager` (0xB6B970).
const USER_RADIO: u32 = 0xB6B970;

// ------------------------------------------------------------------ raw access --------------------

#[inline]
pub(crate) fn r32(a: u32) -> u32 {
    g::<u32>(a as usize).read()
}
#[inline]
pub(crate) fn w32(a: u32, v: u32) {
    g::<u32>(a as usize).write(v)
}
#[inline]
pub(crate) fn r16(a: u32) -> u16 {
    g::<u16>(a as usize).read()
}
#[inline]
pub(crate) fn w16(a: u32, v: u16) {
    g::<u16>(a as usize).write(v)
}
#[inline]
pub(crate) fn r8(a: u32) -> u8 {
    g::<u8>(a as usize).read()
}
#[inline]
pub(crate) fn w8(a: u32, v: u8) {
    g::<u8>(a as usize).write(v)
}
#[inline]
pub(crate) fn rf(a: u32) -> f32 {
    f32::from_bits(r32(a))
}
#[inline]
pub(crate) fn p(a: u32) -> GPtr<u8> {
    GPtr::from_addr(a as usize)
}
/// `fild dword` of an unsigned value with the compiler's `jge; fadd 2^32` fix: the exact value.
#[inline]
pub(crate) fn xu(v: u32) -> X87 {
    X87::from_f64_exact(v as f64)
}
#[inline]
pub(crate) fn xi(v: i32) -> X87 {
    X87::from_i32(v)
}
#[inline]
pub(crate) fn xf(v: f32) -> X87 {
    X87::from_f32(v)
}
/// `_ftol` (0x821B40) of ST0, low dword.
#[inline]
pub(crate) fn ftol(v: X87) -> u32 {
    v.ftol() as u32
}
/// x87 compare as `fcomp` + `test ah, 0x44` + `jnp` (taken when equal).
#[inline]
pub(crate) fn feq(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(0x44)
}
/// `fcomp` + `test ah, 5` + `jnp` (taken when below).
#[inline]
pub(crate) fn fbelow(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(0x05)
}
/// `fcomp` + `test ah, 0x41` + `jne` (taken when below or equal).
#[inline]
pub(crate) fn fbelow_eq(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `_CIpow` (0x822130): `base ^ exp`. Phase A calls the CRT intrinsic; phase B uses the host's
/// `pow` on f64 (the CRT computes in double precision as well; recorded as the phase B
/// replacement, like sa-game's `pow`).
pub(crate) fn ci_pow(base: f64, exp: X87) -> X87 {
    #[cfg(sa_phase_a)]
    let r = sa_core::orig::x87_binary(0x822130, base, exp.to_f64());
    #[cfg(not(sa_phase_a))]
    let r = libm::pow(base, exp.to_f64());
    X87::from_f64_exact(r)
}

orig! {
    // DirectSound (IDirectSound8 / IDirectSoundBuffer8 / IDirectSound3DBuffer / IKsPropertySet)
    fn com_query_interface_raw(obj: GPtr<u8>, iid: u32, out: u32) -> i32 = stdcall vslot 0x00 => unported;
    fn com_add_ref_raw(obj: GPtr<u8>) -> u32 = stdcall vslot 0x04 => unported;
    fn com_release_raw(obj: GPtr<u8>) -> u32 = stdcall vslot 0x08 => unported;
    fn ds_create_sound_buffer_raw(ds: GPtr<u8>, desc: u32, out: u32, outer: u32) -> i32 = stdcall vslot 0x0C => unported;
    fn ds_get_caps_raw(ds: GPtr<u8>, caps: u32) -> i32 = stdcall vslot 0x10 => unported;
    fn ds_set_cooperative_level_raw(ds: GPtr<u8>, hwnd: u32, level: u32) -> i32 = stdcall vslot 0x18 => unported;
    fn ds_get_speaker_config_raw(ds: GPtr<u8>, out: u32) -> i32 = stdcall vslot 0x20 => unported;
    fn buf_get_current_position_raw(b: GPtr<u8>, play: u32, write: u32) -> i32 = stdcall vslot 0x10 => unported;
    fn buf_get_volume_raw(b: GPtr<u8>, out: u32) -> i32 = stdcall vslot 0x18 => unported;
    fn buf_get_status_raw(b: GPtr<u8>, out: u32) -> i32 = stdcall vslot 0x24 => unported;
    fn buf_lock_raw(b: GPtr<u8>, off: u32, n: u32, p1: u32, n1: u32, p2: u32, n2: u32, flags: u32) -> i32 = stdcall vslot 0x2C => unported;
    fn buf_play_raw(b: GPtr<u8>, r: u32, prio: u32, flags: u32) -> i32 = stdcall vslot 0x30 => unported;
    fn buf_set_current_position_raw(b: GPtr<u8>, pos: u32) -> i32 = stdcall vslot 0x34 => unported;
    fn buf_set_volume_raw(b: GPtr<u8>, v: u32) -> i32 = stdcall vslot 0x3C => unported;
    fn buf_set_frequency_raw(b: GPtr<u8>, f: u32) -> i32 = stdcall vslot 0x44 => unported;
    fn buf_stop_raw(b: GPtr<u8>) -> i32 = stdcall vslot 0x48 => unported;
    fn buf_unlock_raw(b: GPtr<u8>, p1: u32, n1: u32, p2: u32, n2: u32) -> i32 = stdcall vslot 0x4C => unported;
    fn buf_restore_raw(b: GPtr<u8>) -> i32 = stdcall vslot 0x50 => unported;
    fn buf_set_fx_raw(b: GPtr<u8>, n: u32, desc: u32, res: u32) -> i32 = stdcall vslot 0x54 => unported;
    fn buf_get_object_in_path_raw(b: GPtr<u8>, guid: u32, idx: u32, iid: u32, out: u32) -> i32 = stdcall vslot 0x5C => unported;
    /// `IDirectSound3DBuffer::SetPosition(x, y, z, apply)` (+0x4C)
    fn b3d_set_position_raw(b: GPtr<u8>, x: u32, y: u32, z: u32, apply: u32) -> i32 = stdcall vslot 0x4C => unported;
    /// `IKsPropertySet::Set(guid, id, inst, instLen, data, dataLen)` (+0x10)
    fn ks_set_raw(ks: GPtr<u8>, guid: u32, id: u32, inst: u32, inst_len: u32, data: u32, len: u32) -> i32 = stdcall vslot 0x10 => unported;
    /// DirectSound FX parameter object `SetAllParameters(params)` (+0x0C)
    fn fx_set_all_parameters_raw(o: GPtr<u8>, params: u32) -> i32 = stdcall vslot 0x0C => unported;

    // channel virtuals (thiscall)
    fn ch_service_raw(ch: GPtr<u8>) = thiscall vslot 0x04 => unported;
    fn ch_is_sound_playing_raw(ch: GPtr<u8>) -> u8 = thiscall vslot 0x08 => unported;
    fn ch_get_play_time_raw(ch: GPtr<u8>) -> u32 = thiscall vslot 0x0C => unported;
    fn ch_get_length_raw(ch: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    fn ch_play_raw(ch: GPtr<u8>, offset: u32, flags: u32, freq: u32) = thiscall vslot 0x14 => unported;
    // decoder virtuals (`CAEStreamingDecoder`, thiscall)
    fn dec_initialise_raw(d: GPtr<u8>) -> u8 = thiscall vslot 0x00 => unported;
    fn dec_fill_buffer_raw(d: GPtr<u8>, buf: u32, size: u32) -> u32 = thiscall vslot 0x04 => unported;
    fn dec_get_stream_length_ms_raw(d: GPtr<u8>) -> u32 = thiscall vslot 0x08 => unported;
    fn dec_get_stream_play_time_ms_raw(d: GPtr<u8>) -> u32 = thiscall vslot 0x0C => unported;
    fn dec_set_cursor_raw(d: GPtr<u8>, ms: u32) = thiscall vslot 0x10 => unported;
    fn dec_get_sample_rate_raw(d: GPtr<u8>) -> u32 = thiscall vslot 0x14 => unported;
    fn dec_delete_raw(d: GPtr<u8>, flags: u32) = thiscall vslot 0x18 => unported;
    fn dec_get_stream_id_raw(d: GPtr<u8>) -> u32 = thiscall vslot 0x1C => unported;

    // Win32
    fn sleep(ms: u32) = stdcall cell 0x8580C4 => unported;
    fn create_thread(sec: u32, stack: u32, entry: u32, param: u32, flags: u32, id: u32) -> u32 = stdcall cell 0x8580B4 => unported;
    #[cfg(not(sa_phase_a))]
    fn resume_thread(h: u32) -> u32 = stdcall cell 0x8580A4 => unported;
    // Phase A: the Start functions' own `call [ResumeThread]` instructions, entered with ECX = this as
    // the original does (thiscall with one stack argument = stdcall(h) plus ECX). Determinism mode
    // redirects exactly these two sites to shims that record ECX and leave the thread suspended
    // (sa-hook detaudio.rs); calling the IAT cell directly would start a free-running thread.
    #[cfg(sa_phase_a)]
    fn resume_thread_at_stream_start(this: u32, h: u32) -> u32 = thiscall site 0x4F11F7 => unported;
    #[cfg(sa_phase_a)]
    fn resume_thread_at_fade_start(this: u32, h: u32) -> u32 = thiscall site 0x4EEA1A => unported;
    fn set_thread_priority(h: u32, prio: i32) -> i32 = stdcall cell 0x8580A8 => unported;
    fn initialize_critical_section(cs: u32) = stdcall cell 0x8580F0 => unported;
    fn enter_critical_section(cs: u32) = stdcall cell 0x8580EC => unported;
    fn leave_critical_section(cs: u32) = stdcall cell 0x8580E8 => unported;
    fn co_initialize(r: u32) -> i32 = stdcall cell 0x858350 => unported;

    /// `Win::IsForegroundApp()` (0x746060)
    fn is_foreground_app() -> u8 = cdecl 0x746060 => unported;
    /// `CAEAudioUtility::GetCurrentTimeInMS()` (0x4D9E80, EDX:EAX)
    pub(crate) fn current_time_ms() -> u64 = cdecl 0x4D9E80 => get_current_time_in_ms;
    /// `operator new` (0x82119A)
    pub(crate) fn operator_new(n: u32) -> u32 = cdecl 0x82119A => unported;

    // ports of this module, called as the original calls them
    fn ac_ctor(this: u32, ds: u32, id: u32, rate: u32, bits: u32) -> u32 = thiscall 0x4D7890 => unported;
    fn ac_bytes_to_ms(this: u32, bytes: u32) -> u32 = thiscall 0x4D79D0 => unported;
    fn ac_ms_to_bytes(this: u32, ms: u32) -> u32 = thiscall 0x4D79F0 => unported;
    fn ac_get_current_playback_position(this: u32) -> u32 = thiscall 0x4D79A0 => channel_get_current_playback_position;
    fn ac_lost(this: u32) -> bool = thiscall 0x4D7A10 => channel_lost;
    fn ac_set_frequency(this: u32, f: u32) = thiscall 0x4D7A50 => channel_set_frequency;
    fn ac_set_original_frequency(this: u32, f: u32) = thiscall 0x4D7A70 => channel_set_original_frequency;
    fn ac_update_status(this: u32) = thiscall 0x4D7BD0 => channel_update_status;
    fn ac_is_buffer_playing(this: u32) -> u8 = thiscall 0x4D7990 => unported;
    fn sf_request_fade(this: u32, buf: u32, vol: u32, time: u32, stop: u32) -> bool = thiscall 0x4EEAA0 => smooth_fade_request_fade;
    fn sf_set_buffer_volume(this: u32, buf: u32, vol: u32) = thiscall 0x4EEC80 => smooth_fade_set_buffer_volume;
    fn sf_cancel_fade(this: u32, buf: u32) = thiscall 0x4EEA50 => smooth_fade_cancel_fade;
    fn sf_service(this: u32) = thiscall 0x4EED10 => smooth_fade_service;
    fn sc_initialise_silence(this: u32) = thiscall 0x4F1C70 => streaming_initialise_silence;
    fn sc_update_play_time(this: u32) -> u32 = thiscall 0x4F18A0 => streaming_update_play_time;
    fn sc_get_playing_track_id(this: u32) -> u32 = thiscall 0x4F1A60 => streaming_get_playing_track_id;
    fn sc_get_active_track_id(this: u32) -> u32 = thiscall 0x4F1A40 => streaming_get_active_track_id;
    fn sc_set_next_stream(this: u32, d: u32) -> u32 = thiscall 0x4F1DE0 => streaming_set_next_stream;
    fn sc_set_ready(this: u32) = thiscall 0x4F1FF0 => streaming_set_ready;
    fn sc_prepare_stream(this: u32, d: u32, flags: u32, stop: u32) -> u32 = thiscall 0x4F23D0 => streaming_prepare_stream;
    fn sc_fill_buffer(this: u32, buf: u32, size: u32) -> u32 = thiscall 0x4F1E20 => streaming_fill_buffer;
    fn sc_stop_class(this: u32, set: u32) = thiscall 0x4F1A90 => streaming_stop_class;
    fn sc_pause(this: u32) = thiscall 0x4F2170 => streaming_pause;
    fn sc_add_fx(this: u32) -> bool = thiscall 0x4F1AE0 => streaming_add_fx;
    fn sc_remove_fx(this: u32) = thiscall 0x4F1C20 => streaming_remove_fx;
    fn st_service(this: u32) = thiscall 0x4F1290 => stream_thread_service;
    fn track_loader_ctor(this: u32) -> u32 = thiscall 0x4E0930 => unported;
    fn track_loader_initialise(this: u32) -> u8 = thiscall 0x4E0C50 => unported;
    fn track_loader_get_data_stream(this: u32, track: u32, a: u32) -> u32 = thiscall 0x4E0D20 => unported;
    fn vorbis_decoder_ctor(this: u32, stream: u32) -> u32 = thiscall 0x5026B0 => unported;
    fn user_radio_load_user_track(this: u32, track: u32) -> u32 = thiscall 0x4F35F0 => unported;
}

// ------------------------------------------------------------------ CAEAudioUtility --------------

/// `CAEAudioUtility::GetCurrentTimeInMS()` (0x4D9E80): `QueryPerformanceCounter`, then
/// `ftol(qpc / freq * 1000.0 - start)` with `freq` = [0xB610F0] and `start` = [0xB610F8] (u64,
/// loaded as a signed 63-bit part minus the sign bit, as the bytes do), under the game's x87
/// precision. Result in EDX:EAX. `#[native]`: phase A excludes it (its `QueryPerformanceCounter`
/// call site is redirected by determinism mode).
#[sa_core::native(addr = 0x4D9E80, conv = cdecl, name = "CAEAudioUtility::GetCurrentTimeInMS")]
pub fn get_current_time_in_ms() -> u64 {
    let mut c: u64 = 0;
    qpc(GPtr::from_raw(&mut c));
    let freq = g::<u64>(0xB610F0).read();
    let start = g::<u64>(0xB610F8).read();
    let q = X87::from_f64_exact(c as i64 as f64).fdiv(X87::from_f64_exact(freq as i64 as f64));
    let ms = q.fmul(xf(1000.0));
    let lo = X87::from_f64_exact((start & 0x7FFF_FFFF_FFFF_FFFF) as f64);
    let hi = X87::from_f64_exact(((start >> 63) << 63) as i64 as f64).fchs();
    ms.fsub(lo.fadd(hi)).ftol()
}

orig! {
    /// `QueryPerformanceCounter(&count)` through the call site 0x4D9E88
    fn qpc(out: GPtr<u64>) -> i32 = stdcall site 0x4D9E88 => unported;
}

// ------------------------------------------------------------------ CAEAudioChannel --------------

/// `CAEAudioChannel::SetPosition(const CVector*)` (0x4D7950): with both buffers,
/// `IDirectSound3DBuffer::SetPosition(x, y, z, DS3D_DEFERRED)`.
#[hook(addr = 0x4D7950, conv = thiscall, name = "CAEAudioChannel::SetPosition", expect = "8B 41 08 85 C0 74 20",
       route = "L2", hazards = "none")]
pub fn channel_set_position(this: u32, pos: u32) {
    if r32(this + 8) == 0 {
        return;
    }
    let b3 = r32(this + 0xC);
    if b3 == 0 {
        return;
    }
    b3d_set_position(b3, r32(pos), r32(pos + 4), r32(pos + 8), 1);
}

/// `CAEAudioChannel::GetCurrentPlaybackPosition()` (0x4D79A0): the play cursor, 0 without a buffer.
#[hook(addr = 0x4D79A0, conv = thiscall, name = "CAEAudioChannel::GetCurrentPlaybackPosition", expect = "51 8B 49 08 33 C0",
       route = "L2", hazards = "none")]
pub fn channel_get_current_playback_position(this: u32) -> u32 {
    let b = r32(this + 8);
    let mut pos: u32 = 0;
    if b != 0 {
        buf_get_current_position(b, GPtr::<u32>::from_raw(&mut pos).addr() as u32, 0);
    }
    pos
}

/// `CAEAudioChannel::Lost()` (0x4D7A10): `Restore` until it no longer reports
/// `DSERR_BUFFERLOST` (sleeping 10 ms after each lost result). AL = 1.
#[hook(addr = 0x4D7A10, conv = thiscall, name = "CAEAudioChannel::Lost", expect = "56 57 8B 3D C4 80 85 00",
       unverified = "std4-audio-1", route = "L2", hazards = "none")]
pub fn channel_lost(this: u32) -> bool {
    loop {
        if buf_restore(r32(this + 8)) as u32 == 0x8878_0096 {
            sleep(10);
        }
        if buf_restore(r32(this + 8)) as u32 != 0x8878_0096 {
            return true;
        }
    }
}

/// `CAEAudioChannel::SetFrequency(uint16)` (0x4D7A50): stored (+0x3C) and set on the buffer when it
/// changes.
#[hook(addr = 0x4D7A50, conv = thiscall, name = "CAEAudioChannel::SetFrequency", expect = "0F B7 54 24 04 39 51 3C",
       route = "L2", hazards = "none")]
pub fn channel_set_frequency(this: u32, f: u32) {
    let f = f & 0xFFFF;
    if r32(this + 0x3C) == f {
        return;
    }
    let b = r32(this + 8);
    w32(this + 0x3C, f);
    if b != 0 {
        buf_set_frequency(b, f);
    }
}

/// `CAEAudioChannel::SetOriginalFrequency(uint16)` (0x4D7A70, entry `push esi; push edi; jmp` into
/// the `.text` chunk 0x406487): as `SetFrequency`, then +0x40 = the frequency. `#[native]`: the
/// entry jumps out of the function after two bytes.
#[sa_core::native(addr = 0x4D7A70, conv = thiscall, name = "CAEAudioChannel::SetOriginalFrequency")]
pub fn channel_set_original_frequency(this: u32, f: u32) {
    let f = f & 0xFFFF;
    if r32(this + 0x3C) != f {
        let b = r32(this + 8);
        w32(this + 0x3C, f);
        if b != 0 {
            buf_set_frequency(b, f);
        }
    }
    w32(this + 0x40, f);
}

/// `CAEAudioChannel::SetReverbAndDepth(int32 reverb, int32 depth)` (0x4D7AA0): with both buffers and
/// EAX present ([0xB60660] == 1): `IKsPropertySet` property 2 = the environment value from the table
/// 0x8AC198 (indexed by the reverb byte), property 5 = `depth * 100`; AL = both succeeded.
#[hook(addr = 0x4D7AA0, conv = thiscall, name = "CAEAudioChannel::SetReverbAndDepth", expect = "51 8B 44 24 0C 6B C0 64",
       route = "L2", hazards = "none")]
pub fn channel_set_reverb_and_depth(this: u32, reverb: u32, depth: u32) -> bool {
    let d = depth.wrapping_mul(100);
    if r32(this + 8) == 0 || r32(this + 0xC) == 0 || r8(0xB60660) != 1 {
        return false;
    }
    let b3 = r32(this + 0xC);
    let mut ks: u32 = 0;
    if com_query_interface(b3, 0x859340, GPtr::<u32>::from_raw(&mut ks).addr() as u32) < 0 {
        return false;
    }
    let idx = reverb as u8 as i8 as i32;
    let env_addr = (0x8AC198i32).wrapping_add(idx * 4) as u32;
    let mut ok = false;
    if ks_set(ks, 0x85EFDC, 2, 0, 0, env_addr, 4) >= 0 {
        let mut dv = d;
        if ks_set(ks, 0x85EFDC, 5, 0, 0, GPtr::<u32>::from_raw(&mut dv).addr() as u32, 4) >= 0 {
            ok = true;
        }
    }
    if ks != 0 {
        com_release(ks);
    }
    ok
}

/// `CAEAudioChannel::SetNotInRoom(uint8)` (0x4D7B50): with both buffers and EAX present: property 7
/// of `DSPROPSETID_EAX20_BufferProperties`-like set 0x85EF9C = 0 (in room) or -10000 (not).
#[hook(addr = 0x4D7B50, conv = thiscall, name = "CAEAudioChannel::SetNotInRoom", expect = "8B 41 08 83 EC 08 85 C0",
       route = "L2", hazards = "none")]
pub fn channel_set_not_in_room(this: u32, not_in_room: u32) {
    if r32(this + 8) == 0 || r32(this + 0xC) == 0 || r8(0xB60660) != 1 {
        return;
    }
    let b3 = r32(this + 0xC);
    let mut ks: u32 = 0;
    if com_query_interface(b3, 0x859340, GPtr::<u32>::from_raw(&mut ks).addr() as u32) != 0 {
        return;
    }
    let mut v: u32 = if not_in_room as u8 != 0 { 0 } else { 0xFFFF_D8F0 };
    ks_set(ks, 0x85EF9C, 7, 0, 0, GPtr::<u32>::from_raw(&mut v).addr() as u32, 4);
    if ks != 0 {
        com_release(ks);
    }
}

/// `CAEAudioChannel::UpdateStatus()` (0x4D7BD0): `GetStatus` into +0x5C; a lost buffer is restored
/// (`Lost`, tail call).
#[hook(addr = 0x4D7BD0, conv = thiscall, name = "CAEAudioChannel::UpdateStatus", expect = "56 8B F1 8B 46 08 8B 08",
       route = "L2", hazards = "none")]
pub fn channel_update_status(this: u32) {
    buf_get_status(r32(this + 8), this + 0x5C);
    if r8(this + 0x5C) & 2 != 0 {
        ac_lost(this);
    }
}

/// `CAEAudioChannel::SetVolume(float)` (0x4D7C60). With a buffer: while it plays and the change is
/// at least 60 dB (`|ftol(new - old)| > 60.0` fails the `<=` test), a smooth fade of type -2
/// (rising) or -1 (falling) is requested; when that is refused (or the buffer is not playing, or
/// the change is small) the volume is set through the fade thread's `SetBufferVolume`. +0x34 =
/// the new volume (also without a buffer: no, only with one).
#[hook(addr = 0x4D7C60, conv = thiscall, name = "CAEAudioChannel::SetVolume", expect = "51 56 57 8B F9 8B 77 08",
       route = "L2", hazards = "none")]
pub fn channel_set_volume(this: u32, vol: u32) {
    let b = r32(this + 8);
    if b == 0 {
        return;
    }
    let v = f32::from_bits(vol);
    let mut set_direct = true;
    if r8(this + 0x5C) & 1 != 0 {
        let d = ftol(xf(v).fsub(xf(rf(this + 0x34)))) as i32;
        let d = d.wrapping_abs();
        if !fbelow_eq(xi(d), xf(60.0)) {
            // rising (new > old: `fcomp; test ah, 0x41; jne` not taken) -> -2, else -1
            let rising = !fbelow_eq(xf(v), xf(rf(this + 0x34)));
            let kind = if rising { 0xFFFF_FFFE } else { 0xFFFF_FFFF };
            if sf_request_fade(SMOOTH_FADE, b, vol, kind, 0) {
                set_direct = false;
            } else {
                sf_set_buffer_volume(SMOOTH_FADE, r32(this + 8), vol);
                set_direct = false;
            }
        }
    }
    if set_direct {
        sf_set_buffer_volume(SMOOTH_FADE, b, vol);
    }
    w32(this + 0x34, vol);
}

/// `CAEAudioChannel::SetFrequencyScalingFactor(float)` (0x4D7D00, vtable slot 8 of the base). 0.0:
/// a playing buffer that is not already stopped this way fades out (or stops) and +0x38 = 1.
/// Otherwise the frequency becomes `ftol(original * factor)` (u16) and a buffer stopped by an
/// earlier 0.0 is restarted: muted, `Play(looping)`, then faded (or set) back to its volume.
#[hook(addr = 0x4D7D00, conv = thiscall, name = "CAEAudioChannel::SetFrequencyScalingFactor", expect = "51 D9 44 24 08 53 D8 1D",
       route = "L2", hazards = "none")]
pub fn channel_set_frequency_scaling_factor(this: u32, factor: u32) {
    let f = f32::from_bits(factor);
    if feq(xf(f), xf(0.0)) {
        let b = r32(this + 8);
        if b != 0 && r8(this + 0x38) == 0 && r8(this + 0x5C) & 1 != 0
            && !sf_request_fade(SMOOTH_FADE, b, 0xC2C8_0000, 0xFFFF_FFFF, 1)
        {
            buf_stop(r32(this + 8));
        }
        w8(this + 0x38, 1);
        return;
    }
    let freq = ftol(xu(r32(this + 0x40)).fmul(xf(f))) & 0xFFFF;
    if r32(this + 0x3C) != freq {
        let b = r32(this + 8);
        w32(this + 0x3C, freq);
        if b != 0 {
            buf_set_frequency(b, freq);
        }
    }
    if r8(this + 0x38) == 0 {
        return;
    }
    let b = r32(this + 8);
    if b != 0 {
        let mut pos: u32 = 0;
        buf_get_current_position(b, GPtr::<u32>::from_raw(&mut pos).addr() as u32, 0);
        let moved = pos > 0;
        if moved {
            buf_set_volume(r32(this + 8), 0xFFFF_D8F0);
        }
        buf_play(r32(this + 8), 0, 0, (r8(this + 0x44) != 0) as u32);
        if moved
            && !sf_request_fade(SMOOTH_FADE, r32(this + 8), r32(this + 0x34), 0xFFFF_FFFE, 0)
        {
            let v = ftol(xf(rf(this + 0x34)).fmul(xf(100.0)));
            buf_set_volume(r32(this + 8), v);
        }
    }
    w8(this + 0x38, 0);
}

// ------------------------------------------------------------------ CAEStaticChannel -------------

/// `CAEStaticChannel::CAEStaticChannel(ds, id, is3D, rate, bits)` (0x4F0B10, `ret 0x14`): the base
/// constructor (0x4D7890), vtable 0x85F3CC, +0x60 / +0x62 = 0, +0x8A = is3D.
#[hook(addr = 0x4F0B10, conv = thiscall, name = "FUN_004f0b10", expect = "8B 44 24 14 8B 54 24 08",
       route = "L2", hazards = "heap-writes")]
pub fn static_channel_ctor(this: u32, ds: u32, id: u32, is3d: u32, rate: u32, bits: u32) -> u32 {
    ac_ctor(this, ds, id, rate, bits);
    w8(this + 0x60, 0);
    w8(this + 0x62, 0);
    w32(this, 0x85F3CC);
    w8(this + 0x8A, is3d as u8);
    this
}

/// `CAEStaticChannel::Service()` (0x4F10D0, vtable slot 1). A looping sound whose loop region is
/// not yet filled (+0x60) gets it filled once `CTimer` time - play start > the fill time: the
/// region after the first pass (`Lock(+0x78, +0x7C)`) receives the loop body +0x80 times, then
/// `Unlock`. Then `UpdateStatus`; a buffer that stopped by itself (not muted by frequency 0) is
/// released (`g_numSoundChannelsUsed - 1`). Without a buffer the status is cleared.
#[hook(addr = 0x4F10D0, conv = thiscall, name = "CAEStaticChannel::Service", expect = "83 EC 08 55 8B E9 8B 45 08",
       route = "L2", hazards = "heap-writes")]
pub fn static_channel_service(this: u32) {
    let b = r32(this + 8);
    if b == 0 {
        w32(this + 0x5C, 0);
        return;
    }
    if r8(this + 0x60) != 0 && r32(TIME_MS).wrapping_sub(r32(this + 0x70)) > r32(this + 0x74) {
        let (mut p1, mut n1) = (0u32, 0u32);
        buf_lock(
            b,
            r32(this + 0x78),
            r32(this + 0x7C),
            GPtr::<u32>::from_raw(&mut p1).addr() as u32,
            GPtr::<u32>::from_raw(&mut n1).addr() as u32,
            0,
            0,
            0,
        );
        let mut k = 0u32;
        while k < r16(this + 0x80) as u32 {
            let len = r32(this + 0x7C);
            let src = r32(this + 0x84).wrapping_add(r32(this + 0x64));
            let dst = p1.wrapping_add(len.wrapping_mul(k));
            copy_bytes(dst, src, len);
            k += 1;
        }
        buf_unlock(r32(this + 8), p1, n1, 0, 0);
        w8(this + 0x60, 0);
    }
    ac_update_status(this);
    if r8(this + 0x38) != 0 || r8(this + 0x5C) & 1 != 0 {
        return;
    }
    if r32(this + 8) != 0 {
        w32(CHANNELS_USED, r32(CHANNELS_USED).wrapping_sub(1));
    }
    let b = r32(this + 8);
    if b != 0 {
        com_release(b);
    }
    w32(this + 8, 0);
}

fn copy_bytes(dst: u32, src: u32, n: u32) {
    for i in 0..n {
        w8(dst.wrapping_add(i), r8(src.wrapping_add(i)));
    }
}
fn zero_bytes(dst: u32, n: u32) {
    for i in 0..n {
        w8(dst.wrapping_add(i), 0);
    }
}

/// `CAEStaticChannel::IsSoundPlaying()` (0x4F0F40): no buffer -> false; muted by frequency 0 or a
/// pending play -> true; else the status's playing bit.
#[hook(addr = 0x4F0F40, conv = thiscall, name = "CAEStaticChannel::IsSoundPlaying", expect = "8B 41 08 85 C0 75 03",
       route = "L2", hazards = "ax-al-return")]
pub fn static_channel_is_sound_playing(this: u32) -> bool {
    if r32(this + 8) == 0 {
        return false;
    }
    if r8(this + 0x38) != 0 || r8(this + 0x62) != 0 {
        return true;
    }
    r32(this + 0x5C) & 1 != 0
}

/// `CAEStaticChannel::GetPlayTime()` (0x4F0F70): -1 when not playing, else the play cursor in ms
/// (sign-extended 16-bit).
#[hook(addr = 0x4F0F70, conv = thiscall, name = "CAEStaticChannel::GetPlayTime", expect = "56 8B F1 8B 06 FF 50 08",
       route = "L2", hazards = "none")]
pub fn static_channel_get_play_time(this: u32) -> i32 {
    if ch_is_sound_playing(this) == 0 {
        return -1;
    }
    let pos = ac_get_current_playback_position(this);
    ac_bytes_to_ms(this, pos) as u16 as i16 as i32
}

/// `CAEStaticChannel::GetLength()` (0x4F0FA0): the sample's length in ms (+0x2C bytes; sign-extended
/// 16-bit).
#[hook(addr = 0x4F0FA0, conv = thiscall, name = "CAEStaticChannel::GetLength", expect = "8B 41 2C 50 E8 27 6A FE FF",
       route = "L2", hazards = "none")]
pub fn static_channel_get_length(this: u32) -> i32 {
    ac_bytes_to_ms(this, r32(this + 0x2C)) as u16 as i16 as i32
}

/// `CAEStaticChannel::Play(int16 startMs, int8 flags, float freq)` (0x4F0BD0, `ret 0xC`): a start
/// offset (not for a looping sound with a loop region) sets the play cursor and +0x61; +0x62 = 1
/// (pending until `SynchPlayback`); +0x38 = (freq == 0.0).
#[hook(addr = 0x4F0BD0, conv = thiscall, name = "CAEStaticChannel::Play", expect = "53 56 8B F1 8A 46 44",
       route = "L2", hazards = "none")]
pub fn static_channel_play(this: u32, start: u32, _flags: u32, freq: u32) {
    let s = start as u16;
    if (r8(this + 0x44) == 0 || r32(this + 0x64) == 0) && s != 0 {
        let bytes = ac_ms_to_bytes(this, s as i16 as i32 as u32);
        buf_set_current_position(r32(this + 8), bytes);
        w8(this + 0x61, 1);
    } else {
        w8(this + 0x61, 0);
    }
    w8(this + 0x62, 1);
    w8(this + 0x38, feq(xf(f32::from_bits(freq)), xf(0.0)) as u8);
}

/// `CAEStaticChannel::SynchPlayback()` (0x4F1040): a pending play (not muted by frequency 0) starts:
/// after a cursor move the buffer is muted and faded (or set) back to its volume; `Play(looping)`;
/// play start = `CTimer` time; pending cleared.
#[hook(addr = 0x4F1040, conv = thiscall, name = "CAEStaticChannel::SynchPlayback", expect = "56 8B F1 8B 46 08 85 C0 74 7A",
       route = "L2", hazards = "none")]
pub fn static_channel_synch_playback(this: u32) {
    let b = r32(this + 8);
    if b == 0 || r8(this + 0x62) == 0 || r8(this + 0x38) != 0 {
        return;
    }
    if r8(this + 0x61) != 0 {
        buf_set_volume(b, 0xFFFF_D8F0);
        if !sf_request_fade(SMOOTH_FADE, r32(this + 8), r32(this + 0x34), 0xFFFF_FFFE, 0) {
            let v = ftol(xf(rf(this + 0x34)).fmul(xf(100.0)));
            buf_set_volume(r32(this + 8), v);
        }
    }
    buf_play(r32(this + 8), 0, 0, (r8(this + 0x44) != 0) as u32);
    w32(this + 0x70, r32(TIME_MS));
    w8(this + 0x62, 0);
}

/// `CAEStaticChannel::Stop()` (0x4F0FB0): a playing buffer fades out (type -1, stop) or stops; the
/// buffer and the 3D buffer are released (`g_numSoundChannelsUsed - 1`); +0x38 = 0.
#[hook(addr = 0x4F0FB0, conv = thiscall, name = "CAEStaticChannel::Stop", expect = "53 56 8B F1 8B 46 08 33 DB",
       unverified = "std4-audio-1", route = "L2", hazards = "none")]
pub fn static_channel_stop(this: u32) {
    let b = r32(this + 8);
    if b != 0 && r8(this + 0x5C) & 1 != 0
        && !sf_request_fade(SMOOTH_FADE, b, 0xC2C8_0000, 0xFFFF_FFFF, 1)
    {
        buf_stop(r32(this + 8));
    }
    release_buffers(this);
    w8(this + 0x38, 0);
}

/// Releases +8 (counting it off `g_numSoundChannelsUsed`) and +0xC, clearing both.
fn release_buffers(this: u32) {
    if r32(this + 8) != 0 {
        w32(CHANNELS_USED, r32(CHANNELS_USED).wrapping_sub(1));
    }
    let b = r32(this + 8);
    if b != 0 {
        com_release(b);
    }
    let b3 = r32(this + 0xC);
    w32(this + 8, 0);
    if b3 != 0 {
        com_release(b3);
    }
    w32(this + 0xC, 0);
}

/// `CAEStaticChannel::SetAudioBuffer(data, size, id0, id1, loopOffset, rate)` (0x4F0C40, `ret 0x18`).
///
/// A NULL buffer or rate 0 fails. The old buffers are released; the sample's fields set (+0x84 data,
/// +0x2C size, +0x88 / +0x8C ids). A loop offset (not -1, in samples) makes a looping sound:
/// +0x64 = offset * 2 bytes, +0x68 = size, loop length +0x7C = size - offset, repeats +0x6C =
/// `max(size, 0x5DC0) / loopLength + 1` and a buffer of `repeats * loopLength` bytes. A 16-bit mono
/// PCM buffer (`DSBCAPS_CTRL3D` when +0x8A is 0, else not: flags 0x80B4 / 0x80B8) at `rate`;
/// `g_numSoundChannelsUsed + 1`; `Lock` of the whole buffer (failure: buffer released, false). A
/// one-shot copies the sample and zero-fills the rest; a looping one copies the sample at the end
/// of the loop region layout (the first pass up to `bytes - offset`), then as many loop bodies as fit
/// before the loop fill point (+0x80 loops filled, +0x78 fill offset), the fill time +0x74 and
/// +0x60 = 1. `Unlock`, cursor 0, the 3D buffer (`QueryInterface`), volume -100 dB. AL = 1.
#[hook(addr = 0x4F0C40, conv = thiscall, name = "CAEStaticChannel::SetAudioBuffer", expect = "83 EC 30 53 55 56 57 8B 7C 24 48",
       route = "L2", hazards = "heap-writes")]
pub fn static_channel_set_audio_buffer(this: u32, data: u32, size: u32, id0: u32, id1: u32, loop_off: u32, rate: u32) -> bool {
    let rate = rate & 0xFFFF;
    let mut p1: u32 = 0;
    let mut n1: u32 = 0;
    let mut first_len: u32 = 0;
    if size == 0 || rate == 0 {
        return false;
    }
    release_buffers_counted(this);
    w32(this + 0x84, data);
    w32(this + 0xC, 0);
    w32(this + 0x64, 0);
    w32(this + 0x68, 0);
    w8(this + 0x44, 0);
    w8(this + 0x60, 0);
    w8(this + 0x38, 0);
    w32(this + 0x6C, 0);
    w32(this + 0x2C, size);
    w16(this + 0x88, id0 as u16);
    w16(this + 0x8C, id1 as u16);
    let lo = loop_off as u16;
    if lo != 0xFFFF {
        let off = ((lo as i16 as i32) << 4 >> 3) as u32;
        w8(this + 0x44, 1);
        w32(this + 0x64, off);
        w32(this + 0x68, size);
    }
    let buf_bytes = if r8(this + 0x44) != 0 && r32(this + 0x64) != 0 {
        let end = r32(this + 0x68);
        let len = end.wrapping_sub(r32(this + 0x64));
        w32(this + 0x7C, len);
        let m = if end < 0x5DC0 { 0x5DC0 } else { end };
        let reps = m / len + 1;
        w32(this + 0x6C, reps);
        reps.wrapping_mul(len)
    } else {
        size
    };
    w32(this + 0x24, buf_bytes);
    // DSBUFFERDESC {size 0x24, flags, bytes, 0, &wfx, guid3DAlgorithm (0x859150, zero)}
    let flags = ((r8(this + 0x8A) == 0) as u32) * 4 + 0x80B4;
    w32(this + 0x4F, rate * 2);
    w32(this + 0x4B, rate);
    w32(this + 0x40, rate);
    w32(this + 0x3C, rate);
    w16(this + 0x57, 0);
    w16(this + 0x49, 1);
    w16(this + 0x55, 0x10);
    w16(this + 0x53, 2);
    w16(this + 0x47, 1);
    let mut desc: [u32; 9] = [0x24, flags, buf_bytes, 0, this + 0x47, r32(0x859150), r32(0x859154), r32(0x859158), r32(0x85915C)];
    if ds_create_sound_buffer(r32(this + 4), GPtr::<u32>::from_raw(desc.as_mut_ptr()).addr() as u32, this + 8, 0) < 0 {
        return false;
    }
    w32(CHANNELS_USED, r32(CHANNELS_USED).wrapping_add(1));
    if buf_lock(
        r32(this + 8),
        0,
        r32(this + 0x2C),
        GPtr::<u32>::from_raw(&mut p1).addr() as u32,
        GPtr::<u32>::from_raw(&mut n1).addr() as u32,
        0,
        0,
        2,
    ) < 0
    {
        let b = r32(this + 8);
        if b != 0 {
            com_release(b);
        }
        w32(this + 8, 0);
        return false;
    }
    let src = r32(this + 0x84);
    if r32(this + 0x64) == 0 {
        copy_bytes(p1, src, size);
        let total = r32(this + 0x2C);
        if size < total {
            zero_bytes(p1.wrapping_add(size), total - size);
        }
        w8(this + 0x60, 0);
    } else {
        let off = r32(this + 0x64);
        first_len = r32(this + 0x24).wrapping_sub(off);
        copy_bytes(p1.wrapping_add(first_len), src, off);
        let len = r32(this + 0x7C);
        let filled = (r32(this + 0x64) / len).wrapping_add(1) as u16;
        w16(this + 0x80, filled);
        w32(this + 0x78, r32(this + 0x24).wrapping_sub((filled as u32).wrapping_mul(len)));
        let mut k: u16 = 0;
        while (k as u32) < r32(this + 0x6C).wrapping_sub(r16(this + 0x80) as u32) {
            let len = r32(this + 0x7C);
            copy_bytes(
                (k as u32).wrapping_mul(len).wrapping_add(p1),
                r32(this + 0x84).wrapping_add(r32(this + 0x64)),
                len,
            );
            k = k.wrapping_add(1);
        }
        let ms = ac_bytes_to_ms(this, r32(this + 0x64));
        w32(this + 0x74, ms as u16 as i16 as i32 as u32);
        w8(this + 0x60, 1);
    }
    buf_unlock(r32(this + 8), p1, n1, 0, 0);
    buf_set_current_position(r32(this + 8), first_len);
    com_query_interface(r32(this + 8), 0x859390, this + 0xC);
    w32(this + 0x34, 0xC2C8_0000);
    buf_set_volume(r32(this + 8), 0xFFFF_D8F0);
    true
}

/// The release at the start of `SetAudioBuffer`: +8 (counted) and +0xC (+8 cleared before the 3D
/// buffer's release; +0xC cleared by the caller's later store).
fn release_buffers_counted(this: u32) {
    if r32(this + 8) != 0 {
        w32(CHANNELS_USED, r32(CHANNELS_USED).wrapping_sub(1));
    }
    let b = r32(this + 8);
    if b != 0 {
        com_release(b);
    }
    let b3 = r32(this + 0xC);
    w32(this + 8, 0);
    if b3 != 0 {
        com_release(b3);
    }
}

// ------------------------------------------------------------------ CAEStreamingChannel ----------

/// `CAEStreamingChannel::CAEStreamingChannel(ds, id)` (0x4F1800, `ret 8`): the base constructor at
/// 48 kHz / 16 bit, flags and pointers cleared, vtable 0x85F3F0, state -6 (stopped), +0x60094 = 1.0.
#[hook(addr = 0x4F1800, conv = thiscall, name = "CAEStreamingChannel::Constructor", expect = "8B 44 24 08 56 6A 10",
       route = "L2", hazards = "heap-writes")]
pub fn streaming_ctor(this: u32, ds: u32, id: u32) -> u32 {
    ac_ctor(this, ds, id, 0xBB80, 0x10);
    for o in 0x60..0x68 {
        w8(this + o, 0);
    }
    w32(this + 0x68, 0);
    w32(this + 0x6006C, 0);
    w32(this + 0x60070, 0);
    w32(this + 0x60078, 0);
    w32(this + 0x60084, 0);
    w32(this, 0x85F3F0);
    w32(this + 0x60074, 0xFFFF_FFFA);
    w32(this + 0x60094, 0x3F80_0000);
    this
}

/// `CAEStreamingChannel::Initialise()` (0x4F22F0): `CoInitialize`; a 0xC0000-byte stereo 16-bit
/// 48 kHz buffer (flags 0x182A8) into +8;
/// success: +0x60 = 1, original frequency 48000, decode buffer +0x68 = +0x6C, +0x44 = 1. Then the
/// silence buffer (0x4F1C70). AL = success.
#[hook(addr = 0x4F22F0, conv = thiscall, name = "CAEStreamingChannel::Initialise", expect = "83 EC 24 53 56 57 33 DB",
       route = "L2", hazards = "heap-writes")]
pub fn streaming_initialise(this: u32) -> bool {
    co_initialize(0);
    w16(this + 0x47, 1);
    w16(this + 0x57, 0);
    w16(this + 0x49, 2);
    w16(this + 0x55, 0x10);
    w32(this + 0x4B, 0xBB80);
    w32(this + 0x4F, 0x2EE00);
    w16(this + 0x53, 4);
    // DSBUFFERDESC {0x24, flags 0x182A8, 0xC0000 bytes (two 0x60000 halves), 0, &wfx, guid}
    let mut desc: [u32; 9] = [0x24, 0x182A8, 0xC0000, 0, this + 0x47, r32(0x859150), r32(0x859154), r32(0x859158), r32(0x85915C)];
    let mut ok = false;
    if ds_create_sound_buffer(r32(this + 4), GPtr::<u32>::from_raw(desc.as_mut_ptr()).addr() as u32, this + 8, 0) >= 0 {
        ok = true;
        w8(this + 0x60, 1);
        ac_set_original_frequency(this, r16(this + 0x4B) as u32);
        w32(this + 0x68, this + 0x6C);
        w8(this + 0x44, 1);
    }
    sc_initialise_silence(this);
    ok
}

/// `CAEStreamingChannel::InitialiseSilence()` (0x4F1C70): a 0x8000-byte silent looping buffer in
/// the channel's format (flags 0x8008) into +0x60090: locked, zeroed, unlocked, played looping.
#[hook(addr = 0x4F1C70, conv = thiscall, name = "CAEStreamingChannel::InitialiseSilence", expect = "83 EC 2C A1 50 91 85 00",
       route = "L2", hazards = "heap-writes")]
pub fn streaming_initialise_silence(this: u32) {
    let mut desc: [u32; 9] = [0x24, 0x8008, 0x8000, 0, this + 0x47, r32(0x859150), r32(0x859154), r32(0x859158), r32(0x85915C)];
    if ds_create_sound_buffer(r32(this + 4), GPtr::<u32>::from_raw(desc.as_mut_ptr()).addr() as u32, this + 0x60090, 0) < 0 {
        return;
    }
    let (mut p1, mut n1) = (0u32, 0u32);
    let b = r32(this + 0x60090);
    buf_lock(b, 0, 0, GPtr::<u32>::from_raw(&mut p1).addr() as u32, GPtr::<u32>::from_raw(&mut n1).addr() as u32, 0, 0, 2);
    zero_bytes(p1, n1);
    buf_unlock(r32(this + 0x60090), p1, n1, 0, 0);
    buf_play(r32(this + 0x60090), 0, 0, 1);
}

/// `CAEStreamingChannel::GetPlayingTrackID()` (0x4F1A60): the decoder's stream id while playing,
/// starting, ending or stopping; else -1.
#[hook(addr = 0x4F1A60, conv = thiscall, name = "CAEStreamingChannel::GetPlayingTrackID", expect = "8B C1 8B 88 6C 00 06 00",
       route = "L2", hazards = "none")]
pub fn streaming_get_playing_track_id(this: u32) -> u32 {
    let d = r32(this + 0x6006C);
    if d == 0 {
        return 0xFFFF_FFFF;
    }
    match r32(this + 0x60074) as i32 {
        -3 | -4 | -1 | -5 => dec_get_stream_id(d),
        _ => 0xFFFF_FFFF,
    }
}

/// `CAEStreamingChannel::GetActiveTrackID()` (0x4F1A40, entry `nop; jmp` into the `.text` chunk
/// 0x40208F): the decoder's stream id, -1 without one. `#[native]`.
#[sa_core::native(addr = 0x4F1A40, conv = thiscall, name = "CAEStreamingChannel::GetActiveTrackID")]
pub fn streaming_get_active_track_id(this: u32) -> u32 {
    let d = r32(this + 0x6006C);
    if d == 0 { 0xFFFF_FFFF } else { dec_get_stream_id(d) }
}

/// `CAEStreamingChannel::IsSoundPlaying()` (0x4F2040): starting, ending, stopping or playing.
#[hook(addr = 0x4F2040, conv = thiscall, name = "CAEStreamingChannel::IsSoundPlaying", expect = "8B 81 74 00 06 00 83 F8 FD",
       unverified = "std4-audio-1", route = "L2", hazards = "ax-al-return")]
pub fn streaming_is_sound_playing(this: u32) -> bool {
    matches!(r32(this + 0x60074) as i32, -3 | -4 | -5 | -1)
}

/// `CAEStreamingChannel::GetLength()` (0x4F1880): the decoder's stream length in ms, -1 without one.
#[hook(addr = 0x4F1880, conv = thiscall, name = "CAEStreamingChannel::GetLength", expect = "8B 89 6C 00 06 00 85 C9 75 04",
       route = "L2", hazards = "none")]
pub fn streaming_get_length(this: u32) -> u32 {
    let d = r32(this + 0x6006C);
    if d == 0 { 0xFFFF_FFFF } else { dec_get_stream_length_ms(d) }
}

/// `CAEStreamingChannel::GetPlayTime()` (0x4F19E0): while playing or starting and less than 1000 ms
/// after the last `UpdatePlayTime` stamp: the stamped time plus the time since; else the state, or
/// (playing/starting, stamp older) `UpdatePlayTime()` (tail call).
#[hook(addr = 0x4F19E0, conv = thiscall, name = "CAEStreamingChannel::GetPlayTime", expect = "83 EC 08 56 8B F1 8B 86 74 00 06 00",
       route = "L2", hazards = "none")]
pub fn streaming_get_play_time(this: u32) -> u32 {
    let st = r32(this + 0x60074);
    if st != 0xFFFF_FFFF && st != 0xFFFF_FFFD {
        return st;
    }
    let stamp = g::<u64>((this + 0x60088) as usize).read();
    let d = current_time_ms().wrapping_sub(stamp);
    if d >> 32 == 0 && (d as u32) < 1000 {
        let now = current_time_ms() as u32;
        return now.wrapping_add(r32(this + 0x6007C).wrapping_sub(r32(this + 0x60088)));
    }
    sc_update_play_time(this)
}

/// `CAEStreamingChannel::UpdatePlayTime()` (0x4F18A0). Not playing / starting: the state is the
/// play time (+0x6007C) and returned. Less than 1000 ms since the stamp: `GetPlayTime()` (vtable).
/// Else from the play cursor: `ms = decoded + cursor / (freq * 0.004)` with the half-buffer
/// corrections of the bytes (`one = 393216.0 / (freq * 0.004)` subtracted per the half markers),
/// minus `one` again when not below it; clamped at 0; the stamp renewed. Returns the play time.
#[hook(addr = 0x4F18A0, conv = thiscall, name = "CAEStreamingChannel::UpdatePlayTime", expect = "83 EC 10 56 8B F1 8B 86 74 00 06 00",
       route = "L2", hazards = "none")]
pub fn streaming_update_play_time(this: u32) -> u32 {
    let st = r32(this + 0x60074);
    if st != 0xFFFF_FFFD && st != 0xFFFF_FFFF {
        w32(this + 0x6007C, st);
        return st;
    }
    let stamp = g::<u64>((this + 0x60088) as usize).read();
    let d = current_time_ms().wrapping_sub(stamp);
    if d >> 32 == 0 && (d as u32) < 1000 {
        return ch_get_play_time(this);
    }
    let mut play: u32 = 0;
    buf_get_current_position(r32(this + 8), GPtr::<u32>::from_raw(&mut play).addr() as u32, 0);
    let decoded = xi(r32(this + 0x60078) as i32);
    let bl = r8(this + 0x67);
    let bpm = xu(r32(this + 0x3C)).fmul(xf(f32::from_bits(0x3B83_126F)));
    let one = xf(393_216.0).fdiv(bpm).to_f32();
    let one_x = xf(one);
    let pos = xu(play).fdiv(bpm);
    let half = (play / 0x60000) as u8;
    let r = if half == 0 {
        if bl == 0 { ftol(decoded.fsub(one_x).fadd(pos)) } else { ftol(pos.fadd(decoded)) }
    } else if bl == 1 {
        ftol(pos.fsub(one_x).fadd(decoded.fsub(one_x)))
    } else {
        ftol(pos.fsub(one_x).fadd(decoded))
    };
    w32(this + 0x6007C, r);
    if !xi(r as i32).fcom(one_x).test_any(0x01) {
        w32(this + 0x6007C, ftol(xi(r as i32).fsub(one_x)));
    }
    let v = r32(this + 0x6007C) as i32;
    w32(this + 0x6007C, if v <= 0 { 0 } else { v as u32 });
    let now = current_time_ms();
    g::<u64>((this + 0x60088) as usize).write(now);
    let v = r32(this + 0x6007C) as i32;
    if v < 0 { 0 } else { v as u32 }
}

/// `CAEStreamingChannel::SetNextStream(decoder)` (0x4F1DE0): a queued next decoder is deleted; the
/// new one queued. EAX = 0.
#[hook(addr = 0x4F1DE0, conv = thiscall, name = "CAEStreamingChannel::SetNextStream", expect = "56 8B F1 8B 8E 70 00 06 00",
       route = "L2", hazards = "heap-writes")]
pub fn streaming_set_next_stream(this: u32, d: u32) -> u32 {
    let old = r32(this + 0x60070);
    if old != 0 {
        dec_delete(old, 1);
        w32(this + 0x60070, 0);
    }
    w32(this + 0x60070, d);
    0
}

/// `CAEStreamingChannel::SetReady()` (0x4F1FF0): a stopping stream's fade is cancelled and the
/// buffer stopped (state -6); a stopped one with a decoder becomes ready (-2).
#[hook(addr = 0x4F1FF0, conv = thiscall, name = "CAEStreamingChannel::SetReady", expect = "56 8B F1 83 BE 74 00 06 00 FB",
       unverified = "std4-audio-1", route = "L2", hazards = "none")]
pub fn streaming_set_ready(this: u32) {
    if r32(this + 0x60074) == 0xFFFF_FFFB {
        sf_cancel_fade(SMOOTH_FADE, r32(this + 8));
        buf_stop(r32(this + 8));
        w32(this + 0x60074, 0xFFFF_FFFA);
    }
    if r32(this + 0x60074) == 0xFFFF_FFFA && r32(this + 0x6006C) != 0 {
        w32(this + 0x60074, 0xFFFF_FFFE);
    }
}

/// `CAEStreamingChannel::FillBuffer(buf, size)` (0x4F1E20, `ret 8`): decodes `size` bytes. Short:
/// a looping stream restarts its decoder at 0 and continues; else a queued next stream with the same
/// sample rate takes over (the old decoder deleted); with a different rate the rest is silence and
/// +0x63 = 1; without a next stream the state becomes -4 (ending). +0x60078 = the decoder's play
/// time. Returns the bytes produced.
#[hook(addr = 0x4F1E20, conv = thiscall, name = "CAEStreamingChannel::FillBuffer", expect = "53 55 8B 6C 24 10 56 57",
       route = "L2", hazards = "heap-writes")]
pub fn streaming_fill_buffer(this: u32, buf: u32, size: u32) -> u32 {
    let mut got = dec_fill_buffer(r32(this + 0x6006C), buf, size);
    if got < size {
        if r8(this + 0x61) != 0 {
            while got < size {
                dec_set_cursor(r32(this + 0x6006C), 1);
                got = got.wrapping_add(dec_fill_buffer(r32(this + 0x6006C), got.wrapping_add(buf), size - got));
            }
        } else {
            let next = r32(this + 0x60070);
            if next != 0 {
                let cur = r32(this + 0x6006C);
                let rn = dec_get_sample_rate(next);
                if dec_get_sample_rate(cur) == rn {
                    let cur = r32(this + 0x6006C);
                    if cur != 0 {
                        dec_delete(cur, 1);
                    }
                    let n = r32(this + 0x60070);
                    w32(this + 0x6006C, n);
                    w32(this + 0x60070, 0);
                    got = got.wrapping_add(dec_fill_buffer(n, buf.wrapping_add(got), size - got));
                } else {
                    zero_bytes(buf.wrapping_add(got), size - got);
                    got = size;
                    w8(this + 0x63, 1);
                }
            } else {
                w32(this + 0x60074, 0xFFFF_FFFC);
            }
        }
    }
    let d = r32(this + 0x6006C);
    if d != 0 {
        w32(this + 0x60078, dec_get_stream_play_time_ms(d));
    }
    got
}

/// `CAEStreamingChannel::PrepareStream(decoder, flags, stop)` (0x4F23D0, `ret 0xC`; a mid-body
/// `jmp` to the `.text` chunk 0x406039 and back). NULL decoder or buffer: -1. With `stop`, a
/// playing / starting / ending stream fades out (or stops) and the state becomes -6. The old
/// decoder is deleted, the new one installed (+0x61 = looping flag), the whole buffer locked and
/// filled (short: zero-filled, +0x64 = 1), every dword OR 0x10001 (as the bytes do), unlocked; the
/// state becomes ready (-2) unless starting / playing / paused; +0x66 = 0, the decoder's sample rate
/// as original frequency, +0x63 / +0x62 / +0x45 = 0, cursor 0. EAX = 0.
#[hook(addr = 0x4F23D0, conv = thiscall, name = "CAEStreamingChannel::PrepareStream", expect = "53 56 57 8B 7C 24 10 33 DB",
       route = "L2", hazards = "heap-writes")]
pub fn streaming_prepare_stream(this: u32, d: u32, flags: u32, stop: u32) -> u32 {
    if d == 0 || r32(this + 8) == 0 {
        return 0xFFFF_FFFF;
    }
    if stop as u8 != 0 {
        let st = r32(this + 0x60074) as i32;
        if matches!(st, -3 | -4 | -1)
            && !sf_request_fade(SMOOTH_FADE, r32(this + 8), 0xC2C8_0000, 0x23, 1)
        {
            buf_stop(r32(this + 8));
        }
        w32(this + 0x60074, 0xFFFF_FFFA);
    }
    let old = r32(this + 0x6006C);
    if old != 0 {
        dec_delete(old, 1);
        w32(this + 0x6006C, 0);
    }
    w8(this + 0x61, flags as u8 & 1);
    w32(this + 0x6006C, d);
    let (mut p1, mut n1) = (0u32, 0u32);
    if buf_lock(r32(this + 8), 0, 0, GPtr::<u32>::from_raw(&mut p1).addr() as u32, GPtr::<u32>::from_raw(&mut n1).addr() as u32, 0, 0, 2) >= 0 {
        let got = sc_fill_buffer(this, p1, n1);
        if got != n1 {
            zero_bytes(p1.wrapping_add(got), n1 - got);
            w8(this + 0x64, 1);
        } else {
            w8(this + 0x64, 0);
        }
        let mut i = 0u32;
        while n1 & !3 != 0 && i < n1 >> 2 {
            w32(p1 + 4 * i, r32(p1 + 4 * i) | 0x10001);
            i += 1;
        }
        buf_unlock(r32(this + 8), p1, n1, 0, 0);
    }
    let st = r32(this + 0x60074) as i32;
    if !matches!(st, -3 | -1 | -7) {
        w32(this + 0x60074, 0xFFFF_FFFE);
    }
    w8(this + 0x66, 0);
    let rate = dec_get_sample_rate(r32(this + 0x6006C));
    ac_set_original_frequency(this, rate);
    w8(this + 0x63, 0);
    w8(this + 0x62, 0);
    w8(this + 0x45, 0);
    buf_set_current_position(r32(this + 8), 0);
    0
}

/// `CAEStreamingChannel::Play(int16, int8 flags, float)` (0x4F1D40, vtable slot 5, `ret 0xC`): with
/// a decoder and a buffer: the decoder's rate as original frequency; unless paused (-7) the looping
/// flag from `flags & 1`; a stopping stream's fade is cancelled and the buffer stopped; state
/// playing (-1); the volume set (`ftol(volume * 100)`); `Play(looping)`.
#[hook(addr = 0x4F1D40, conv = thiscall, name = "CAEStreamingChannel::Play", expect = "56 8B F1 8B 8E 6C 00 06 00 85 C9",
       route = "L2", hazards = "none")]
pub fn streaming_play(this: u32, _start: u32, flags: u32, _freq: u32) {
    let d = r32(this + 0x6006C);
    if d == 0 || r32(this + 8) == 0 {
        return;
    }
    let rate = dec_get_sample_rate(d);
    ac_set_original_frequency(this, rate);
    let st = r32(this + 0x60074);
    if st != 0xFFFF_FFF9 {
        w8(this + 0x61, flags as u8 & 1);
    }
    if st == 0xFFFF_FFFB {
        sf_cancel_fade(SMOOTH_FADE, r32(this + 8));
        buf_stop(r32(this + 8));
        w32(this + 0x60074, 0xFFFF_FFFA);
    }
    let v = ftol(xf(rf(this + 0x34)).fmul(xf(100.0)));
    let b = r32(this + 8);
    w32(this + 0x60074, 0xFFFF_FFFF);
    buf_set_volume(b, v);
    buf_play(r32(this + 8), 0, 0, 1);
}

/// `CAEStreamingChannel::SynchPlayback()` (0x4F1870): `ret`. `#[native]` (one byte).
#[sa_core::native(addr = 0x4F1870, conv = thiscall, name = "CAEStreamingChannel::SynchPlayback")]
pub fn streaming_synch_playback(_this: u32) {}

/// `CAEStreamingChannel::Stop()` (0x4F21C0, vtable slot 7): fade out over 35 ms (or stop); state -5
/// (stopping), stop counter 0.
#[hook(addr = 0x4F21C0, conv = thiscall, name = "CAEStreamingChannel::Stop-virtual", expect = "56 6A 01 8B F1 8B 46 08 6A 23",
       route = "L2", hazards = "none")]
pub fn streaming_stop_virtual(this: u32) {
    if !sf_request_fade(SMOOTH_FADE, r32(this + 8), 0xC2C8_0000, 0x23, 1) {
        buf_stop(r32(this + 8));
    }
    w32(this + 0x60074, 0xFFFF_FFFB);
    w32(this + 0x60084, 0);
}

/// `CAEStreamingChannel::Stop(bool)` (0x4F1A90, `ret 4`): fade out (or stop); with the argument,
/// state -5 and stop counter 0.
#[hook(addr = 0x4F1A90, conv = thiscall, name = "CAEStreamingChannel::Stop-class", expect = "56 6A 01 8B F1 8B 46 08 6A 23",
       unverified = "std4-audio-1", route = "L2", hazards = "none")]
pub fn streaming_stop_class(this: u32, set: u32) {
    if !sf_request_fade(SMOOTH_FADE, r32(this + 8), 0xC2C8_0000, 0x23, 1) {
        buf_stop(r32(this + 8));
    }
    if set as u8 != 0 {
        w32(this + 0x60074, 0xFFFF_FFFB);
        w32(this + 0x60084, 0);
    }
}

/// `CAEStreamingChannel::Pause()` (0x4F2170): a playing buffer fades out (or stops); state -7.
#[hook(addr = 0x4F2170, conv = thiscall, name = "CAEStreamingChannel::Pause", expect = "56 8B F1 8B 46 08 85 C0 74 37",
       unverified = "std4-audio-1", route = "L2", hazards = "none")]
pub fn streaming_pause(this: u32) {
    if r32(this + 8) == 0 || ac_is_buffer_playing(this) == 0 {
        return;
    }
    if !sf_request_fade(SMOOTH_FADE, r32(this + 8), 0xC2C8_0000, 0x23, 1) {
        buf_stop(r32(this + 8));
    }
    w32(this + 0x60074, 0xFFFF_FFF9);
}

/// `CAEStreamingChannel::SetFrequencyScalingFactor(float)` (0x4F2060, vtable slot 8). 0.0: a playing
/// (not paused) buffer fades out (or stops) and the state becomes paused (-7). Else the frequency
/// `ftol(original * factor)`; a paused stream restarts muted, `Play(looping)`, and fades (or is
/// set) back to its volume; state playing.
#[hook(addr = 0x4F2060, conv = thiscall, name = "CAEStreamingChannel::SetFrequencyScalingFactor", expect = "51 D9 44 24 08 56 D8 1D",
       route = "L2", hazards = "none")]
pub fn streaming_set_frequency_scaling_factor(this: u32, factor: u32) {
    let f = f32::from_bits(factor);
    if feq(xf(f), xf(0.0)) {
        if r32(this + 8) == 0 || r32(this + 0x60074) == 0xFFFF_FFF9 || ac_is_buffer_playing(this) == 0 {
            return;
        }
        if !sf_request_fade(SMOOTH_FADE, r32(this + 8), 0xC2C8_0000, 0x23, 1) {
            buf_stop(r32(this + 8));
        }
        w32(this + 0x60074, 0xFFFF_FFF9);
        return;
    }
    let freq = ftol(xu(r32(this + 0x40)).fmul(xf(f)));
    ac_set_frequency(this, freq & 0xFFFF);
    if r32(this + 0x60074) != 0xFFFF_FFF9 {
        return;
    }
    if r32(this + 8) != 0 {
        buf_set_volume(r32(this + 8), 0xFFFF_D8F0);
        buf_play(r32(this + 8), 0, 0, (r8(this + 0x44) != 0) as u32);
        if !sf_request_fade(SMOOTH_FADE, r32(this + 8), r32(this + 0x34), 0x23, 0) {
            let v = ftol(xf(rf(this + 0x34)).fmul(xf(100.0)));
            buf_set_volume(r32(this + 8), v);
        }
    }
    w32(this + 0x60074, 0xFFFF_FFFF);
}

/// `CAEStreamingChannel::AddFX()` (0x4F1AE0): once (+0x65): two parametric-EQ effects
/// (`DSFXDESC` from 0x8591E0) through `SetFX` (failure: AL = 0), their parameters from 0x8CBA70 /
/// 0x8CBA7C (`GetObjectInPath` + `SetAllParameters`); a playing buffer is stopped around it and
/// restarted looping. AL = 1.
#[hook(addr = 0x4F1AE0, conv = thiscall, name = "CAEStreamingChannel::AddFX", expect = "83 EC 50 53 56 8B F1 8A 46 65",
       unverified = "std4-audio-1", route = "L2", hazards = "none")]
pub fn streaming_add_fx(this: u32) -> bool {
    if r8(this + 0x65) != 0 {
        return true;
    }
    let guid = [r32(0x8591E0), r32(0x8591E4), r32(0x8591E8), r32(0x8591EC)];
    let mut fx: [u32; 16] = [0x20, 0, guid[0], guid[1], guid[2], guid[3], 0, 0, 0x20, 0, guid[0], guid[1], guid[2], guid[3], 0, 0];
    let mut res: [u32; 2] = [0; 2];
    let mut was_playing = false;
    if r8(this + 0x5C) & 1 != 0 {
        buf_stop(r32(this + 8));
        was_playing = true;
    }
    if buf_set_fx(r32(this + 8), 2, GPtr::<u32>::from_raw(fx.as_mut_ptr()).addr() as u32, GPtr::<u32>::from_raw(res.as_mut_ptr()).addr() as u32) < 0 {
        return false;
    }
    for (i, params) in [(0u32, 0x8CBA70u32), (1, 0x8CBA7C)] {
        let mut o: u32 = 0;
        buf_get_object_in_path(r32(this + 8), 0x8593B0, i, 0x8592D0, GPtr::<u32>::from_raw(&mut o).addr() as u32);
        fx_set_all_parameters(o, params);
        com_release(o);
    }
    if was_playing {
        buf_play(r32(this + 8), 0, 0, 1);
    }
    w8(this + 0x65, 1);
    true
}

/// `CAEStreamingChannel::RemoveFX()` (0x4F1C20): a playing buffer is stopped, `SetFX(0)`, restarted
/// looping; +0x65 = 0.
#[hook(addr = 0x4F1C20, conv = thiscall, name = "CAEStreamingChannel::RemoveFX", expect = "53 56 8B F1 8A 46 5C 32 DB",
       unverified = "std4-audio-1", route = "L2", hazards = "none")]
pub fn streaming_remove_fx(this: u32) {
    let playing = r8(this + 0x5C) & 1 != 0;
    if playing {
        buf_stop(r32(this + 8));
    }
    buf_set_fx(r32(this + 8), 0, 0, 0);
    if playing {
        buf_play(r32(this + 8), 0, 0, 1);
    }
    w8(this + 0x65, 0);
}

/// `CAEStreamingChannel::SetBassEQ(int8 setting, float gain)` (0x4F1F30, `ret 8`): 0 removes the FX;
/// else (adding them when needed) both EQ bands get the table entry `0x8CBA70 + (2*setting + i)*12`
/// with its gain scaled by `gain`.
#[hook(addr = 0x4F1F30, conv = thiscall, name = "CAEStreamingChannel::SetBassEQ", expect = "83 EC 0C 53 8A 5C 24 14",
       route = "L2", hazards = "none")]
pub fn streaming_set_bass_eq(this: u32, setting: u32, gain: u32) {
    let s = setting as u8;
    if s == 0 {
        if r8(this + 0x65) != 0 {
            sc_remove_fx(this);
        }
        return;
    }
    if r8(this + 0x65) == 0 && !sc_add_fx(this) {
        return;
    }
    for i in 0..2i32 {
        let mut o: u32 = 0;
        if buf_get_object_in_path(r32(this + 8), 0x8593B0, i as u32, 0x8592D0, GPtr::<u32>::from_raw(&mut o).addr() as u32) < 0 {
            continue;
        }
        let e = (0x8CBA70i32 + (i + 2 * (s as i8 as i32)) * 12) as u32;
        let g = xf(rf(e + 8)).fmul(xf(f32::from_bits(gain))).to_f32();
        let mut params: [u32; 3] = [r32(e), r32(e + 4), g.to_bits()];
        fx_set_all_parameters(o, GPtr::<u32>::from_raw(params.as_mut_ptr()).addr() as u32);
        com_release(o);
    }
}

/// `CAEStreamingChannel::Service()` (0x4F2550, vtable slot 1). Status update; the state machine of
/// the bytes: a playing buffer turns starting (-1 -> -3); a stopping one is stopped after six
/// services; a stopped one (-6) is stopped again; a non-playing starting one becomes paused-ended
/// (-7). While starting / ending / paused-ended: the decoder's rate is kept as original frequency
/// and, when the play cursor crosses into the other half of the 0x60000-byte buffer (+0x66), the
/// half just played is refilled: ending (-4) stops the buffer (state -6) the second time; a pending
/// rate switch (+0x63) or a queued next stream (+0x62) restarts with it; a silence-ended stream
/// fills silence; else the decoder fills it (every dword OR 0x10001).
#[hook(addr = 0x4F2550, conv = thiscall, name = "CAEStreamingChannel::Service", expect = "83 EC 10 53 55 56 57 8B E9",
       route = "L2", hazards = "heap-writes")]
pub fn streaming_service(this: u32) {
    ac_update_status(this);
    let st_addr = this + 0x60074;
    if r8(this + 0x5C) & 1 != 0 {
        match r32(st_addr) as i32 {
            -1 => w32(st_addr, 0xFFFF_FFFD),
            -3 | -4 => {}
            -5 => {
                let c = r32(this + 0x60084).wrapping_add(1);
                w32(this + 0x60084, c);
                if c as i32 > 6 {
                    buf_stop(r32(this + 8));
                }
            }
            v => {
                w32(this + 0x60084, 0);
                if v != -2 && v == -6 {
                    buf_stop(r32(this + 8));
                }
            }
        }
    } else if r32(st_addr) == 0xFFFF_FFFB {
        w32(st_addr, 0xFFFF_FFFA);
    }
    if r32(st_addr) == 0xFFFF_FFFD && r32(this + 0x5C) == 0 {
        w32(st_addr, 0xFFFF_FFF9);
    }
    let st = r32(st_addr) as i32;
    if !matches!(st, -3 | -4 | -7) {
        return;
    }
    let rate = dec_get_sample_rate(r32(this + 0x6006C));
    if rate != r32(this + 0x40) {
        let rate = dec_get_sample_rate(r32(this + 0x6006C));
        ac_set_original_frequency(this, rate);
    }
    let mut play: u32 = 0;
    buf_get_current_position(r32(this + 8), GPtr::<u32>::from_raw(&mut play).addr() as u32, 0);
    let half = (play / 0x60000) as u8;
    if half == r8(this + 0x66) {
        return;
    }
    let got: u32;
    if r32(st_addr) == 0xFFFF_FFFC {
        if r8(this + 0x45) != 0 {
            buf_stop(r32(this + 8));
            w32(st_addr, 0xFFFF_FFFA);
            w8(this + 0x45, 0);
            return;
        }
        got = 0;
        w8(this + 0x45, 1);
    } else if r8(this + 0x63) != 0 {
        got = 0;
        w8(this + 0x63, 0);
        w8(this + 0x62, 1);
    } else if r8(this + 0x62) != 0 {
        let next = r32(this + 0x60070);
        w32(this + 0x60070, 0);
        w32(st_addr, 0xFFFF_FFF9);
        sc_stop_class(this, 0);
        sc_prepare_stream(this, next, 0, 0);
        ch_play(this, 0, 0, 0x3F80_0000);
        return;
    } else if r8(this + 0x64) == 0 {
        got = sc_fill_buffer(this, r32(this + 0x68), 0x60000);
        w8(this + 0x67, half);
    } else {
        w32(st_addr, 0xFFFF_FFFC);
        return;
    }
    let (mut p1, mut n1) = (0u32, 0u32);
    let off = (r8(this + 0x66) as u32 * 3) << 0x11;
    if buf_lock(r32(this + 8), off, 0x60000, GPtr::<u32>::from_raw(&mut p1).addr() as u32, GPtr::<u32>::from_raw(&mut n1).addr() as u32, 0, 0, 0) >= 0 {
        if got > 0 {
            let n = if got < n1 { got } else { n1 };
            copy_bytes(p1, r32(this + 0x68), n);
        }
        zero_bytes(p1.wrapping_add(got), 0x60000u32.wrapping_sub(got));
        let mut i = 0u32;
        while n1 & !3 != 0 && i < n1 >> 2 {
            w32(p1 + 4 * i, r32(p1 + 4 * i) | 0x10001);
            i += 1;
        }
        buf_unlock(r32(this + 8), p1, n1, 0, 0);
    }
    w8(this + 0x66, half);
}

// ------------------------------------------------------------------ CAESmoothFadeThread ----------

/// Slot `i` of the fade thread (0x20 bytes from +8): +0 buffer, +4 start volume, +8 target volume,
/// +0xC target - start, +0x10 current volume, +0x14 stop-at-end, +0x16 duration (ms, u16), +0x18
/// state (0 free, 1 fading, 2 cancelled), +0x1C start time.
fn slot(this: u32, i: u32) -> u32 {
    this + 8 + i * 0x20
}

/// `CAESmoothFadeThread::Initialise()` (0x4EEEC0): the 64 slots freed, the thread created suspended
/// (`SmoothFadeProc`, id at +4); failure -1: +0x808 = 0, AL = 0. Normal priority, +0x814 = 0,
/// +0x808 = 1. AL = 1.
#[hook(addr = 0x4EEEC0, conv = thiscall, name = "CAESmoothFadeThread::Initialise", expect = "53 56 8B F1 8D 46 08",
       unverified = "std4-audio-1", route = "L3-only", hazards = "none", note = "creates a thread")]
pub fn smooth_fade_initialise(this: u32) -> bool {
    for i in 0..0x40 {
        w32(slot(this, i) + 0x18, 0);
        w32(slot(this, i), 0);
    }
    let h = create_thread(0, 0, 0x4EEE90, this, 4, this + 4);
    w32(this, h);
    if h == 0xFFFF_FFFF {
        w8(this + 0x808, 0);
        return false;
    }
    set_thread_priority(h, 0);
    w8(this + 0x814, 0);
    w8(this + 0x808, 1);
    true
}

/// `CAESmoothFadeThread::InitialiseRequestSlots()` (0x4EE9F0): the 64 slots freed.
#[hook(addr = 0x4EE9F0, conv = thiscall, name = "CAESmoothFadeThread::InitialiseRequestSlots", expect = "8D 41 08 B9 40 00 00 00",
       unverified = "std4-audio-1", route = "L2", hazards = "none")]
pub fn smooth_fade_initialise_request_slots(this: u32) {
    for i in 0..0x40 {
        w32(slot(this, i) + 0x18, 0);
        w32(slot(this, i), 0);
    }
}

/// `CAESmoothFadeThread::Start()` (0x4EEA10): +0x809 (running) = 1, `ResumeThread`.
#[hook(addr = 0x4EEA10, conv = thiscall, name = "CAESmoothFadeThread::Start", expect = "8B 01 50 C6 81 09 08 00 00 01",
       unverified = "std4-audio-1", route = "L3-only", hazards = "none", note = "starts a thread")]
pub fn smooth_fade_start(this: u32) {
    let h = r32(this);
    w8(this + 0x809, 1);
    #[cfg(sa_phase_a)]
    resume_thread_at_fade_start(this, h);
    #[cfg(not(sa_phase_a))]
    resume_thread(h);
}

/// `CAESmoothFadeThread::SmoothFadeProc(this)` (0x4EEE90): `Service(); Sleep(1)` while running.
/// The thread entry (stdcall, `ret 4`); phase B runs it on a host thread.
#[hook(addr = 0x4EEE90, conv = stdcall, name = "CAESmoothFadeThread::SmoothFadeProc", expect = "56 8B 74 24 08 8A 86 09 08 00 00",
       unverified = "std4-audio-1", route = "L3-only", hazards = "none", note = "thread entry")]
pub fn smooth_fade_proc(this: u32) -> u32 {
    while smooth_fade_step(this) {
        sleep(1);
    }
    0
}

/// One pass of [`smooth_fade_proc`]'s loop: `Service()` while running; false once stopped. wasm32 (no threads)
/// calls it from the main loop.
pub fn smooth_fade_step(this: u32) -> bool {
    if r8(this + 0x809) == 0 {
        return false;
    }
    sf_service(this);
    true
}

/// `CAESmoothFadeThread::RequestFade(buffer, volume, time, stop)` (0x4EEAA0, `ret 0x10`).
///
/// Refused (AL = 0) when not initialised, shut down, or the buffer is not playing. Already at the
/// target (equal, or within 0.01 dB): a `stop` request stops the buffer; AL = 1. Else among the
/// first `[+0x818]` slots: the first free one, or a fading slot of this buffer whose
/// `start - now + time > 1` (unsigned); none: AL = 0. A fading slot of this buffer that stops keeps
/// going (AL = 1). A new slot takes a reference (`AddRef`, `g_numSoundChannelsUsed + 1`). The fade:
/// start = current = the buffer's volume (`GetVolume * 0.01`), target, difference, stop flag,
/// duration (`time`; -2 = 0x1C ms software / 0x14 hardware, -1 = 0x1E / 0x14), start time, state 1.
#[hook(addr = 0x4EEAA0, conv = thiscall, name = "CAESmoothFadeThread::RequestFade", expect = "83 EC 10 53 55 56 57 8B F9",
       unverified = "std4-audio-1", route = "L2", hazards = "none")]
pub fn smooth_fade_request_fade(this: u32, buf: u32, vol: u32, time: u32, stop: u32) -> bool {
    if r8(this + 0x808) == 0 || r8(this + 0x814) != 0 {
        return false;
    }
    let mut status: u32 = 0;
    buf_get_status(buf, GPtr::<u32>::from_raw(&mut status).addr() as u32);
    if status & 1 == 0 {
        return false;
    }
    let mut v: i32 = 0;
    buf_get_volume(buf, GPtr::<i32>::from_raw(&mut v).addr() as u32);
    let cur = xi(v).fmul(xf(f32::from_bits(0x3C23_D70A))).to_f32();
    let target = f32::from_bits(vol);
    if feq(xf(cur), xf(target)) || fbelow(xf(cur).fsub(xf(target)).fabs(), xf(f32::from_bits(0x3C23_D70A))) {
        if stop as u8 != 0 {
            buf_stop(buf);
        }
        return true;
    }
    let n = r32(this + 0x818) as i32;
    let mut found = false;
    let mut idx = 0u32;
    let mut i = 0i32;
    while i < n {
        let s = slot(this, i as u32);
        let state = r32(s + 0x18);
        if state == 0 {
            if !found {
                found = true;
                idx = i as u32;
            }
        } else if state == 1 && r32(s) == buf {
            let d = r32(s + 0x1C)
                .wrapping_sub(r32(this + 0x810))
                .wrapping_add(time as u16 as i16 as i32 as u32);
            if d > 1 {
                idx = i as u32;
                found = true;
                break;
            }
        }
        i += 1;
    }
    if !found {
        return false;
    }
    let s = slot(this, idx);
    if r32(s + 0x18) == 1 {
        if r8(s + 0x14) != 0 {
            return true;
        }
    } else {
        w32(CHANNELS_USED, r32(CHANNELS_USED).wrapping_add(1));
        com_add_ref(buf);
        w32(s, buf);
    }
    w32(s + 8, vol);
    let cur_bits = cur.to_bits();
    w32(s + 0x10, cur_bits);
    w32(s + 4, cur_bits);
    w32(s + 0xC, xf(target).fsub(xf(cur)).to_f32().to_bits());
    w8(s + 0x14, stop as u8);
    let t = time as u16 as i16;
    let dur: u32 = match t {
        -2 => if status & 8 == 0 { 0x1C } else { 0x14 },
        -1 => if status & 8 == 0 { 0x1E } else { 0x14 },
        _ => time,
    };
    w16(s + 0x16, dur as u16);
    w32(s + 0x1C, r32(this + 0x810));
    w32(s + 0x18, 1);
    true
}

/// `CAESmoothFadeThread::SetBufferVolume(buffer, volume)` (0x4EEC80, `ret 8`): a fading slot of the
/// buffer that stops keeps going (nothing set); one fading to this volume (equal or within 0.01)
/// keeps going too; any other fade of it is cancelled (state 2). Then `SetVolume(ftol(vol * 100))`.
#[hook(addr = 0x4EEC80, conv = thiscall, name = "CAESmoothFadeThread::SetBufferVolume", expect = "8A 81 14 08 00 00 84 C0 56 57",
       route = "L2", hazards = "none")]
pub fn smooth_fade_set_buffer_volume(this: u32, buf: u32, vol: u32) {
    let v = f32::from_bits(vol);
    if r8(this + 0x814) == 0 {
        for i in 0..0x40 {
            let s = slot(this, i);
            if r32(s + 0x18) == 1 && r32(s) == buf {
                if r8(s + 0x14) != 0 {
                    return;
                }
                let t = xf(rf(s + 8));
                if feq(xf(v), t) || fbelow(t.fsub(xf(v)).fabs(), xf(f32::from_bits(0x3C23_D70A))) {
                    return;
                }
                w32(s + 0x18, 2);
                break;
            }
        }
    }
    let lv = ftol(xf(v).fmul(xf(100.0)));
    buf_set_volume(buf, lv);
}

/// `CAESmoothFadeThread::CancelFade(buffer)` (0x4EEA50, `ret 4`): every fading slot of the buffer
/// is cancelled (state 2).
#[hook(addr = 0x4EEA50, conv = thiscall, name = "CAESmoothFadeThread::CancelFade", expect = "8A 81 14 08 00 00 84 C0 75 22",
       unverified = "std4-audio-1", route = "L2", hazards = "none")]
pub fn smooth_fade_cancel_fade(this: u32, buf: u32) {
    if r8(this + 0x814) != 0 {
        return;
    }
    for i in 0..0x40 {
        let s = slot(this, i);
        if r32(s + 0x18) == 1 && r32(s) == buf {
            w32(s + 0x18, 2);
        }
    }
}

/// `CAESmoothFadeThread::Service()` (0x4EED10): `now` = `GetCurrentTimeInMS` (+0x810). Each fading
/// slot past its duration: unless the buffer's volume was changed meanwhile (more than 0.01 dB away
/// from the current fade volume, and not stopping), the target is set (and a stopping fade stops
/// the buffer); the reference is dropped and the slot freed. A running fade sets
/// `start + 20 * log10(1 + (10^(diff / 20) - 1) * elapsed / duration)` (current volume +0x10).
/// Cancelled slots drop their reference and are freed.
#[hook(addr = 0x4EED10, conv = thiscall, name = "CAESmoothFadeThread::Service", expect = "83 EC 14 53 55 56 57 8B F9",
       unverified = "std4-audio-1", route = "L3-only", hazards = "x87-transcendental", note = "runs on the fade thread")]
pub fn smooth_fade_service(this: u32) {
    w32(this + 0x810, current_time_ms() as u32);
    for i in 0..0x40 {
        let s = slot(this, i);
        let state = r32(s + 0x18);
        if state == 1 && r32(s) != 0 {
            let b = r32(s);
            let now = r32(this + 0x810);
            let start = r32(s + 0x1C);
            if now < start {
                continue;
            }
            let el = now - start;
            let dur = r16(s + 0x16) as u32;
            if el >= dur {
                let mut v: i32 = 0;
                buf_get_volume(b, GPtr::<i32>::from_raw(&mut v).addr() as u32);
                let cur = xi(v).fmul(xf(f32::from_bits(0x3C23_D70A))).to_f32();
                let stopping = r8(s + 0x14) != 0;
                let fc = xf(rf(s + 0x10));
                let set = stopping
                    || feq(xf(cur), fc)
                    || fbelow(xf(cur).fsub(fc).fabs(), xf(f32::from_bits(0x3C23_D70A)));
                if set {
                    let lv = ftol(xf(rf(s + 8)).fmul(xf(100.0)));
                    buf_set_volume(r32(s), lv);
                    if r8(s + 0x14) != 0 {
                        buf_stop(r32(s));
                    }
                }
                w32(CHANNELS_USED, r32(CHANNELS_USED).wrapping_sub(1));
                let b = r32(s);
                if b != 0 {
                    com_release(b);
                }
                w32(s, 0);
                w32(s + 0x18, 0);
            } else {
                let ratio = ci_pow(10.0, xf(rf(s + 0xC)).fmul(xf(f32::from_bits(0x3D4C_CCCD))));
                let frac = xu(el).fdiv(xi(dur as i32));
                let lin = ratio.fsub(xf(1.0)).fmul(frac).fadd(xf(1.0));
                let db = X87::lg2().fyl2x(lin).fmul(xf(20.0)).fadd(xf(rf(s + 4)));
                w32(s + 0x10, db.to_f32().to_bits());
                let lv = ftol(db.fmul(xf(100.0)));
                buf_set_volume(b, lv);
            }
        } else if state == 2 {
            w32(CHANNELS_USED, r32(CHANNELS_USED).wrapping_sub(1));
            let b = r32(s);
            if b != 0 {
                com_release(b);
            }
            w32(s, 0);
            w32(s + 0x18, 0);
        }
    }
}

// ------------------------------------------------------------------ CAEStreamThread --------------

/// `CAEStreamThread::Initialise(channel)` (0x4F1680, `ret 4`): flags (+0x10 active = 1, +0x11 /
/// +0x12 / +0x15 / +0x16 = 0), the channel's playing track, play time, active track and length
/// cached (+0x34 / +0x28 / +0x30 / +0x2C), the thread created suspended (`MainLoop`, id at +4),
/// normal priority, the critical section (+0x38), the channel (+8), a new `CAEMP3TrackLoader`
/// (+0xC, initialised). AL = +0x10.
#[hook(addr = 0x4F1680, conv = thiscall, name = "CAEStreamThread::Initialise", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "std4-audio-1", route = "L3-only", hazards = "heap-writes", note = "creates a thread")]
pub fn stream_thread_initialise(this: u32, ch: u32) -> bool {
    w8(this + 0x10, 1);
    w8(this + 0x11, 0);
    w8(this + 0x12, 0);
    w8(this + 0x15, 0);
    w8(this + 0x16, 0);
    w32(this + 0x34, sc_get_playing_track_id(ch));
    w32(this + 0x28, ch_get_play_time(ch));
    w32(this + 0x30, sc_get_active_track_id(ch));
    w32(this + 0x2C, ch_get_length(ch));
    let h = create_thread(0, 0, 0x4F15C0, this, 4, this + 4);
    w32(this, h);
    set_thread_priority(h, 0);
    initialize_critical_section(this + 0x38);
    w32(this + 8, ch);
    let tl = operator_new(0x1C);
    let tl = if tl != 0 { track_loader_ctor(tl) } else { 0 };
    w32(this + 0xC, tl);
    track_loader_initialise(tl);
    r8(this + 0x10) != 0
}

/// `CAEStreamThread::Start()` (0x4F11F0): +0x11 (running) = 1, `ResumeThread`.
#[hook(addr = 0x4F11F0, conv = thiscall, name = "CAEStreamThread::Start", expect = "8B 01 50 C6 41 11 01",
       unverified = "std4-audio-1", route = "L3-only", hazards = "none", note = "starts a thread")]
pub fn stream_thread_start(this: u32) {
    let h = r32(this);
    w8(this + 0x11, 1);
    #[cfg(sa_phase_a)]
    resume_thread_at_stream_start(this, h);
    #[cfg(not(sa_phase_a))]
    resume_thread(h);
}

/// `CAEStreamThread::MainLoop(this)` (0x4F15C0, thread entry, stdcall `ret 4`). While running:
/// in the foreground, a stream paused by losing the focus is played again, then `Service()` and the
/// channel's `Service()`, sleeping up to 5 ms; on losing the focus a playing channel is paused
/// (and remembered). `Sleep(100)` each pass. Returns 0. Phase B runs it on a host thread.
#[hook(addr = 0x4F15C0, conv = stdcall, name = "CAEStreamThread::MainLoop", expect = "51 53 57 8B 7C 24 10",
       unverified = "std4-audio-1", route = "L3-only", hazards = "none", note = "thread entry")]
pub fn stream_thread_main_loop(this: u32) -> u32 {
    let mut st = StreamLoopState::default();
    while stream_thread_step(this, &mut st) {
        sleep(100);
    }
    0
}

/// The locals [`stream_thread_main_loop`] keeps between passes.
#[derive(Debug, Clone, Copy)]
pub struct StreamLoopState {
    was_fg: bool,
    paused: bool,
}

impl Default for StreamLoopState {
    fn default() -> Self {
        StreamLoopState { was_fg: true, paused: false }
    }
}

/// One pass of [`stream_thread_main_loop`]'s loop without its trailing `Sleep(100)`; false once the thread is
/// stopped (+0x11 = 0). wasm32 (no threads) calls it from the main loop at the same cadence.
pub fn stream_thread_step(this: u32, st: &mut StreamLoopState) -> bool {
    if r8(this + 0x11) == 0 {
        return false;
    }
    let fg = is_foreground_app() != 0;
    if fg {
        if !st.was_fg && st.paused {
            ch_play(r32(this + 8), 0, 0, 0x3F80_0000);
        }
        let t0 = current_time_ms() as u32;
        st_service(this);
        ch_service(r32(this + 8));
        let t1 = current_time_ms() as u32;
        if t1.wrapping_sub(t0) < 5 {
            sleep(t0.wrapping_sub(t1).wrapping_add(5));
        }
    } else if st.was_fg {
        if ac_is_buffer_playing(r32(this + 8)) != 0 {
            sc_pause(r32(this + 8));
            st.paused = true;
        } else {
            st.paused = false;
        }
    }
    st.was_fg = fg;
    true
}

/// Caches the channel's playing track (+0x34), play time (+0x28), active track (+0x30), length (+0x2C).
fn stream_thread_cache(this: u32) {
    let ch = r32(this + 8);
    w32(this + 0x34, sc_get_playing_track_id(ch));
    let ch = r32(this + 8);
    w32(this + 0x28, ch_get_play_time(ch));
    let ch = r32(this + 8);
    w32(this + 0x30, sc_get_active_track_id(ch));
    let ch = r32(this + 8);
    w32(this + 0x2C, ch_get_length(ch));
}

/// A decoder for `track` (user track through `AEUserRadioTrackManager`, else a new
/// `CAEVorbisDecoder` over the track loader's data stream); one that fails `Initialise` is deleted.
fn stream_thread_load_decoder(this: u32, user: bool, track: u32) -> u32 {
    if user {
        user_radio_load_user_track(USER_RADIO, track)
    } else {
        let m = operator_new(0x28);
        if m != 0 {
            let s = track_loader_get_data_stream(r32(this + 0xC), track, 0);
            vorbis_decoder_ctor(m, s)
        } else {
            0
        }
    }
}

/// `CAEStreamThread::Service()` (0x4F1290). The channel's play time is updated; a stop request
/// (+0x16) stops the channel. Without a request (+0x12): unless preparing (+0x15) the channel's
/// track state is cached. With one (under the critical section +0x38): the next track (+0x20, -1 =
/// none) is decoded first (user track +0x14 or Vorbis; a failed `Initialise` deletes it); when the
/// channel already plays the requested track (+0x18) the next one is queued and the channel set
/// ready; otherwise the requested track (user +0x13) is decoded, its cursor set to `+0x1C % length`,
/// the next one queued and the stream prepared (flags +0x24), or without it the next one prepared,
/// or preparing marked (+0x15 = 1). Then the track state is cached, or while preparing the requested
/// (or next) id is reported as playing and active.
#[hook(addr = 0x4F1290, conv = thiscall, name = "CAEStreamThread::Service", expect = "64 A1 00 00 00 00 6A FF 68 06 C1 83 00",
       unverified = "std4-audio-1", route = "L3-only", hazards = "heap-writes", note = "runs on the stream thread")]
pub fn stream_thread_service(this: u32) {
    sc_update_play_time(r32(this + 8));
    if r8(this + 0x16) != 0 {
        w8(this + 0x16, 0);
        crate::port4_hw::ch_stop(r32(this + 8));
    }
    if r8(this + 0x12) == 0 {
        if r8(this + 0x15) != 0 {
            return;
        }
        stream_thread_cache(this);
        return;
    }
    let cs = this + 0x38;
    enter_critical_section(cs);
    let user = r8(this + 0x13) != 0;
    let next = r32(this + 0x20);
    let track = r32(this + 0x18);
    let flags = r8(this + 0x24) as u32;
    let offset = r32(this + 0x1C);
    let next_user = r8(this + 0x14) != 0;
    w8(this + 0x12, 0);
    let mut next_dec = 0u32;
    let mut skip_main = false;
    if next != 0xFFFF_FFFF {
        let d = stream_thread_load_decoder(this, next_user, next);
        next_dec = d;
        if d != 0 && dec_initialise(d) == 0 {
            dec_delete(d, 1);
            next_dec = 0;
        }
        if sc_get_playing_track_id(r32(this + 8)) == track {
            if next_dec != 0 {
                sc_set_next_stream(r32(this + 8), next_dec);
            }
            sc_set_ready(r32(this + 8));
            skip_main = true;
        }
    } else {
        sc_set_next_stream(r32(this + 8), 0);
    }
    if !skip_main {
        let d = stream_thread_load_decoder(this, user, track);
        let mut ok = false;
        if d != 0 {
            if dec_initialise(d) != 0 {
                let len = dec_get_stream_length_ms(d);
                dec_set_cursor(d, offset % len);
                sc_set_next_stream(r32(this + 8), next_dec);
                sc_prepare_stream(r32(this + 8), d, flags, 1);
                w8(this + 0x15, 0);
                ok = true;
            } else {
                dec_delete(d, 1);
            }
        }
        if !ok {
            sc_set_next_stream(r32(this + 8), 0);
            if next_dec != 0 {
                sc_prepare_stream(r32(this + 8), next_dec, flags, 1);
                w8(this + 0x15, 0);
            } else {
                w8(this + 0x15, 1);
            }
        }
    }
    if r8(this + 0x15) == 0 {
        stream_thread_cache(this);
    } else if next == 0xFFFF_FFFF {
        w32(this + 0x34, track);
        w32(this + 0x30, track);
    } else {
        w32(this + 0x34, next);
        w32(this + 0x30, next);
    }
    leave_critical_section(cs);
}

orig! {
    /// channel virtual `Stop()` (+0x1C)
    fn ch_stop_raw(ch: GPtr<u8>) = thiscall vslot 0x1C => unported;
}

/// `CAEStreamThread::PlayTrack(track, next, offset, flags, user, nextUser)` (0x4F1230, `ret 0x18`):
/// under the critical section: a ready channel (play time -2) is stopped; the request stored
/// (+0x18 track, +0x20 next, +0x1C offset, +0x24 flags, +0x13 / +0x14 user flags) and flagged (+0x12).
#[hook(addr = 0x4F1230, conv = thiscall, name = "CAEStreamThread::PlayTrack", expect = "56 57 8B F1 8D 7E 38",
       route = "L2", hazards = "none")]
pub fn stream_thread_play_track(this: u32, track: u32, next: u32, offset: u32, flags: u32, user: u32, next_user: u32) {
    let cs = this + 0x38;
    enter_critical_section(cs);
    let ch = r32(this + 8);
    if ch_get_play_time(ch) == 0xFFFF_FFFE {
        ch_stop(r32(this + 8));
    }
    w32(this + 0x18, track);
    w32(this + 0x20, next);
    w32(this + 0x1C, offset);
    w8(this + 0x24, flags as u8);
    w8(this + 0x13, user as u8);
    w8(this + 0x14, next_user as u8);
    w8(this + 0x12, 1);
    leave_critical_section(cs);
}

/// `CAEStreamThread::GetTrackPlayTime()` (0x4F1530): -8 while preparing, else the channel's
/// `GetPlayTime()` (tail call through the vtable).
#[hook(addr = 0x4F1530, conv = thiscall, name = "CAEStreamThread::GetTrackPlayTime", expect = "8A 41 15 84 C0 75 08",
       route = "L2", hazards = "none")]
pub fn stream_thread_get_track_play_time(this: u32) -> u32 {
    if r8(this + 0x15) != 0 {
        return 0xFFFF_FFF8;
    }
    ch_get_play_time(r32(this + 8))
}

/// `CAEStreamThread::GetTrackLengthMs()` (0x4F1550): +0x2C. `#[native]` (4-byte body).
#[sa_core::native(addr = 0x4F1550, conv = thiscall, name = "CAEStreamThread::GetTrackLengthMs")]
pub fn stream_thread_get_track_length_ms(this: u32) -> u32 {
    r32(this + 0x2C)
}

/// `CAEStreamThread::GetActiveTrackID()` (0x4F1560): +0x30. `#[native]` (4-byte body).
#[sa_core::native(addr = 0x4F1560, conv = thiscall, name = "CAEStreamThread::GetActiveTrackID")]
pub fn stream_thread_get_active_track_id(this: u32) -> u32 {
    r32(this + 0x30)
}

/// `CAEStreamThread::GetPlayingTrackID()` (0x4F1570): +0x34. `#[native]` (4-byte body).
#[sa_core::native(addr = 0x4F1570, conv = thiscall, name = "CAEStreamThread::GetPlayingTrackID")]
pub fn stream_thread_get_playing_track_id(this: u32) -> u32 {
    r32(this + 0x34)
}

/// `CAEStreamThread::StopTrack()` (0x4F1580): +0x16 (stop request) = 1.
#[hook(addr = 0x4F1580, conv = thiscall, name = "CAEStreamThread::StopTrack", expect = "C6 41 16 01 C3",
       route = "L2", hazards = "none")]
pub fn stream_thread_stop_track(this: u32) {
    w8(this + 0x16, 1);
}

/// `CAEPedSpeechAudioEntity::Reset()` (0x4E37B0): PORT-AUDIO-1's port ([`crate::ped::speech_reset`],
/// unhooked because the protection reads its entry bytes), registered for phase B.
#[sa_core::native(addr = 0x4E37B0, conv = cdecl, name = "CAEPedSpeechAudioEntity::Reset")]
pub fn speech_reset_native() {
    crate::ped::speech_reset();
}

/// `CAEStreamThread::CAEStreamThread()` (0x4F11D0): `mov eax, ecx; ret` (nothing initialised).
/// `#[native]` (3-byte body).
#[sa_core::native(addr = 0x4F11D0, conv = thiscall, name = "CAEStreamThread::CAEStreamThread")]
pub fn stream_thread_ctor(this: u32) -> u32 {
    this
}

/// `CAEStreamTransformer::Initialise()` (0x4F1750): the 16-byte stream key: byte `i` = the
/// constant key byte `i` (on the stack: EA 3B C6 A2 9E AD 12 F4 40 B9 DD 28 91 E5 F1 FE) XOR `i`.
#[hook(addr = 0x4F1750, conv = thiscall, name = "CAEStreamTransformer::Initialise", expect = "83 EC 10 56 57 8B F1",
       route = "L2", hazards = "none")]
pub fn stream_transformer_initialise(this: u32) {
    const KEY: [u8; 16] = [0xEA, 0x3B, 0xC6, 0xA2, 0x9E, 0xAD, 0x12, 0xF4, 0x40, 0xB9, 0xDD, 0x28, 0x91, 0xE5, 0xF1, 0xFE];
    for (i, k) in KEY.iter().enumerate() {
        w8(this + i as u32, k ^ i as u8);
    }
}

/// `CAESound::CAESound()` (0x446FE0): +8 (physical entity) = NULL; returns `this`. The element
/// constructor of `AESoundManager`'s sound array. `#[native]` (10-byte body).
#[sa_core::native(addr = 0x446FE0, conv = thiscall, name = "CAESound::CAESound")]
pub fn sound_ctor(this: u32) -> u32 {
    w32(this + 8, 0);
    this
}

/// `CAEPedSpeechSlot` constructor (0x4E3330): `mov eax, ecx; ret` (nothing initialised); the
/// element constructor of `CAEPedSpeechAudioEntity::s_PedSpeechSlots`. `#[native]` (3-byte body).
#[sa_core::native(addr = 0x4E3330, conv = thiscall, name = "CAEPedSpeechSlot::CAEPedSpeechSlot")]
pub fn ped_speech_slot_ctor(this: u32) -> u32 {
    this
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_STREAM_TRANSFORMER_INITIALISE,
    HOOK_STREAM_THREAD_GET_TRACK_PLAY_TIME,
    HOOK_STREAM_THREAD_STOP_TRACK,
    HOOK_CHANNEL_SET_POSITION,
    HOOK_CHANNEL_GET_CURRENT_PLAYBACK_POSITION,
    HOOK_CHANNEL_LOST,
    HOOK_CHANNEL_SET_FREQUENCY,
    HOOK_CHANNEL_SET_REVERB_AND_DEPTH,
    HOOK_CHANNEL_SET_NOT_IN_ROOM,
    HOOK_CHANNEL_UPDATE_STATUS,
    HOOK_CHANNEL_SET_VOLUME,
    HOOK_CHANNEL_SET_FREQUENCY_SCALING_FACTOR,
    HOOK_STATIC_CHANNEL_CTOR,
    HOOK_STATIC_CHANNEL_SERVICE,
    HOOK_STATIC_CHANNEL_IS_SOUND_PLAYING,
    HOOK_STATIC_CHANNEL_GET_PLAY_TIME,
    HOOK_STATIC_CHANNEL_GET_LENGTH,
    HOOK_STATIC_CHANNEL_PLAY,
    HOOK_STATIC_CHANNEL_SYNCH_PLAYBACK,
    HOOK_STATIC_CHANNEL_STOP,
    HOOK_STATIC_CHANNEL_SET_AUDIO_BUFFER,
    HOOK_STREAMING_CTOR,
    HOOK_STREAMING_INITIALISE,
    HOOK_STREAMING_INITIALISE_SILENCE,
    HOOK_STREAMING_GET_PLAYING_TRACK_ID,
    HOOK_STREAMING_IS_SOUND_PLAYING,
    HOOK_STREAMING_GET_LENGTH,
    HOOK_STREAMING_GET_PLAY_TIME,
    HOOK_STREAMING_UPDATE_PLAY_TIME,
    HOOK_STREAMING_SET_NEXT_STREAM,
    HOOK_STREAMING_SET_READY,
    HOOK_STREAMING_FILL_BUFFER,
    HOOK_STREAMING_PREPARE_STREAM,
    HOOK_STREAMING_PLAY,
    HOOK_STREAMING_STOP_VIRTUAL,
    HOOK_STREAMING_STOP_CLASS,
    HOOK_STREAMING_PAUSE,
    HOOK_STREAMING_SET_FREQUENCY_SCALING_FACTOR,
    HOOK_STREAMING_ADD_FX,
    HOOK_STREAMING_REMOVE_FX,
    HOOK_STREAMING_SET_BASS_EQ,
    HOOK_STREAMING_SERVICE,
    HOOK_SMOOTH_FADE_INITIALISE,
    HOOK_SMOOTH_FADE_INITIALISE_REQUEST_SLOTS,
    HOOK_SMOOTH_FADE_START,
    HOOK_SMOOTH_FADE_PROC,
    HOOK_SMOOTH_FADE_REQUEST_FADE,
    HOOK_SMOOTH_FADE_SET_BUFFER_VOLUME,
    HOOK_SMOOTH_FADE_CANCEL_FADE,
    HOOK_SMOOTH_FADE_SERVICE,
    HOOK_STREAM_THREAD_INITIALISE,
    HOOK_STREAM_THREAD_START,
    HOOK_STREAM_THREAD_MAIN_LOOP,
    HOOK_STREAM_THREAD_SERVICE,
    HOOK_STREAM_THREAD_PLAY_TRACK,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    NATIVE_GET_CURRENT_TIME_IN_MS,
    NATIVE_CHANNEL_SET_ORIGINAL_FREQUENCY,
    NATIVE_STREAMING_GET_ACTIVE_TRACK_ID,
    NATIVE_STREAMING_SYNCH_PLAYBACK,
    NATIVE_STREAM_THREAD_GET_TRACK_LENGTH_MS,
    NATIVE_STREAM_THREAD_GET_ACTIVE_TRACK_ID,
    NATIVE_STREAM_THREAD_GET_PLAYING_TRACK_ID,
    NATIVE_SPEECH_RESET_NATIVE,
    NATIVE_STREAM_THREAD_CTOR,
    NATIVE_SOUND_CTOR,
    NATIVE_PED_SPEECH_SLOT_CTOR,
];

// thin wrappers over the `vslot` declarations (they take the object as a `GPtr`)
#[inline]
pub(crate) fn com_query_interface(obj: u32, iid: u32, out: u32) -> i32 {
    com_query_interface_raw(GPtr::from_addr(obj as usize), iid, out)
}
#[inline]
pub(crate) fn com_add_ref(obj: u32) -> u32 {
    com_add_ref_raw(GPtr::from_addr(obj as usize))
}
#[inline]
pub(crate) fn com_release(obj: u32) -> u32 {
    com_release_raw(GPtr::from_addr(obj as usize))
}
#[inline]
pub(crate) fn ds_create_sound_buffer(ds: u32, desc: u32, out: u32, outer: u32) -> i32 {
    ds_create_sound_buffer_raw(GPtr::from_addr(ds as usize), desc, out, outer)
}
#[inline]
pub(crate) fn ds_get_caps(ds: u32, caps: u32) -> i32 {
    ds_get_caps_raw(GPtr::from_addr(ds as usize), caps)
}
#[inline]
pub(crate) fn ds_set_cooperative_level(ds: u32, hwnd: u32, level: u32) -> i32 {
    ds_set_cooperative_level_raw(GPtr::from_addr(ds as usize), hwnd, level)
}
#[inline]
pub(crate) fn ds_get_speaker_config(ds: u32, out: u32) -> i32 {
    ds_get_speaker_config_raw(GPtr::from_addr(ds as usize), out)
}
#[inline]
pub(crate) fn buf_get_current_position(b: u32, play: u32, write: u32) -> i32 {
    buf_get_current_position_raw(GPtr::from_addr(b as usize), play, write)
}
#[inline]
pub(crate) fn buf_get_volume(b: u32, out: u32) -> i32 {
    buf_get_volume_raw(GPtr::from_addr(b as usize), out)
}
#[inline]
pub(crate) fn buf_get_status(b: u32, out: u32) -> i32 {
    buf_get_status_raw(GPtr::from_addr(b as usize), out)
}
#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) fn buf_lock(b: u32, off: u32, n: u32, p1: u32, n1: u32, p2: u32, n2: u32, flags: u32) -> i32 {
    buf_lock_raw(GPtr::from_addr(b as usize), off, n, p1, n1, p2, n2, flags)
}
#[inline]
pub(crate) fn buf_play(b: u32, r: u32, prio: u32, flags: u32) -> i32 {
    buf_play_raw(GPtr::from_addr(b as usize), r, prio, flags)
}
#[inline]
pub(crate) fn buf_set_current_position(b: u32, pos: u32) -> i32 {
    buf_set_current_position_raw(GPtr::from_addr(b as usize), pos)
}
#[inline]
pub(crate) fn buf_set_volume(b: u32, v: u32) -> i32 {
    buf_set_volume_raw(GPtr::from_addr(b as usize), v)
}
#[inline]
pub(crate) fn buf_set_frequency(b: u32, f: u32) -> i32 {
    buf_set_frequency_raw(GPtr::from_addr(b as usize), f)
}
#[inline]
pub(crate) fn buf_stop(b: u32) -> i32 {
    buf_stop_raw(GPtr::from_addr(b as usize))
}
#[inline]
pub(crate) fn buf_unlock(b: u32, p1: u32, n1: u32, p2: u32, n2: u32) -> i32 {
    buf_unlock_raw(GPtr::from_addr(b as usize), p1, n1, p2, n2)
}
#[inline]
pub(crate) fn buf_restore(b: u32) -> i32 {
    buf_restore_raw(GPtr::from_addr(b as usize))
}
#[inline]
pub(crate) fn buf_set_fx(b: u32, n: u32, desc: u32, res: u32) -> i32 {
    buf_set_fx_raw(GPtr::from_addr(b as usize), n, desc, res)
}
#[inline]
pub(crate) fn buf_get_object_in_path(b: u32, guid: u32, idx: u32, iid: u32, out: u32) -> i32 {
    buf_get_object_in_path_raw(GPtr::from_addr(b as usize), guid, idx, iid, out)
}
#[inline]
pub(crate) fn b3d_set_position(b: u32, x: u32, y: u32, z: u32, apply: u32) -> i32 {
    b3d_set_position_raw(GPtr::from_addr(b as usize), x, y, z, apply)
}
#[inline]
pub(crate) fn ks_set(ks: u32, guid: u32, id: u32, inst: u32, inst_len: u32, data: u32, len: u32) -> i32 {
    ks_set_raw(GPtr::from_addr(ks as usize), guid, id, inst, inst_len, data, len)
}
#[inline]
pub(crate) fn fx_set_all_parameters(o: u32, params: u32) -> i32 {
    fx_set_all_parameters_raw(GPtr::from_addr(o as usize), params)
}
#[inline]
pub(crate) fn ch_service(ch: u32) {
    ch_service_raw(GPtr::from_addr(ch as usize))
}
#[inline]
pub(crate) fn ch_is_sound_playing(ch: u32) -> u8 {
    ch_is_sound_playing_raw(GPtr::from_addr(ch as usize))
}
#[inline]
pub(crate) fn ch_get_play_time(ch: u32) -> u32 {
    ch_get_play_time_raw(GPtr::from_addr(ch as usize))
}
#[inline]
pub(crate) fn ch_get_length(ch: u32) -> u32 {
    ch_get_length_raw(GPtr::from_addr(ch as usize))
}
#[inline]
pub(crate) fn ch_play(ch: u32, offset: u32, flags: u32, freq: u32) {
    ch_play_raw(GPtr::from_addr(ch as usize), offset, flags, freq)
}
#[inline]
pub(crate) fn dec_initialise(d: u32) -> u8 {
    dec_initialise_raw(GPtr::from_addr(d as usize))
}
#[inline]
pub(crate) fn dec_fill_buffer(d: u32, buf: u32, size: u32) -> u32 {
    dec_fill_buffer_raw(GPtr::from_addr(d as usize), buf, size)
}
#[inline]
pub(crate) fn dec_get_stream_length_ms(d: u32) -> u32 {
    dec_get_stream_length_ms_raw(GPtr::from_addr(d as usize))
}
#[inline]
pub(crate) fn dec_get_stream_play_time_ms(d: u32) -> u32 {
    dec_get_stream_play_time_ms_raw(GPtr::from_addr(d as usize))
}
#[inline]
pub(crate) fn dec_set_cursor(d: u32, ms: u32) {
    dec_set_cursor_raw(GPtr::from_addr(d as usize), ms)
}
#[inline]
pub(crate) fn dec_get_sample_rate(d: u32) -> u32 {
    dec_get_sample_rate_raw(GPtr::from_addr(d as usize))
}
#[inline]
pub(crate) fn dec_delete(d: u32, flags: u32) {
    dec_delete_raw(GPtr::from_addr(d as usize), flags)
}
#[inline]
pub(crate) fn dec_get_stream_id(d: u32) -> u32 {
    dec_get_stream_id_raw(GPtr::from_addr(d as usize))
}
#[inline]
pub(crate) fn ch_stop(ch: u32) {
    ch_stop_raw(GPtr::from_addr(ch as usize))
}
