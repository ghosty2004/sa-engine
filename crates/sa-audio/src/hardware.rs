//! `CAEAudioHardware` (PORT-AUDIO): the game-side wrappers of the hardware layer (`AEAudioHardware`
//! 0xB5F8B8, 0x1014 bytes). DirectSound itself (channel objects, `Initialise`, `Service`, `PlaySound`,
//! `RescaleChannelVolumes`, the listener) stays original and is reached through `orig!` / the channel
//! vtables. Ported from the 1.0 US bytes; gta-reversed `AEAudioHardware.cpp` and
//! `docs/re/specs/AUDIO/CAEAudioHardware.deviations.md` as reading aids.
//!
//! Layout used here: +0 initialised flag, +1 "effects loading disabled", +6 channel flags (u16 per
//! channel), +0x8E number of channels (u16), +0x90 channels per slot (u16 per slot; a slot's first
//! channel index is the slot id), +0x110 channel volumes (f32), +0x310 channel frequency scaling
//! factors (f32), +0x410/+0x414/+0x418/+0x41C/+0x420/+0x424 music master / effects master / music
//! fader / effects fader / non-stream fader / stream fader scaling factors (f32), +0x430 / +0x688
//! virtual-channel loop start times / lengths requested (u16 x 300), +0x8E0 / +0xB38 the arrays
//! `GetVirtualChannelSound*` copy (0x96 dwords), +0xD90 bass setting (dword), +0xD94 bass gain,
//! +0xD98 `CAEMP3BankLoader*`, +0xD9C `CAEMP3TrackLoader*`, +0xE10 `CAEStreamingChannel*`, +0xE14
//! `CAEStreamThread`, +0xE64 channel pointers (`CAEAudioChannel*`), +0x1010 track flags (int8).
//!
//! Tail jumps are ported as calls of the same target with `this` adjusted and every stack word passed
//! through unchanged (`u32`); the return follows the C++ type (`bool` = AL, `int32`/pointers = EAX,
//! `void` = none: e.g. `StopTrack`'s target leaves the caller's EAX, which no caller reads).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, rd, wr, x};

/// No globals of its own (`AEAudioHardware` is reached by address).
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CAEMP3BankLoader::LoadSoundBank(uint16 bank, int16 slot)` (0x4E0670)
    pub fn bank_load_sound_bank(this: GPtr<u8>, bank: u32, slot: u32) = thiscall 0x4E0670 => unported;
    /// `CAEMP3BankLoader::IsSoundBankLoaded(uint16 bank, int16 slot)` (0x4E0220)
    pub fn bank_is_sound_bank_loaded(this: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4E0220 => unported;
    /// `CAEMP3BankLoader::GetSoundBankLoadingStatus(uint16 bank, int16 slot)` (0x4E0250)
    pub fn bank_get_sound_bank_loading_status(this: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4E0250 => unported;
    /// `CAEMP3BankLoader::GetSoundHeadroom(uint16 sound, int16 slot)` (0x4E01E0)
    pub fn bank_get_sound_headroom(this: GPtr<u8>, sound: u32, slot: u32) -> u32 = thiscall 0x4E01E0 => unported;
    /// `CAEMP3BankLoader::LoadSound(uint16 bank, uint16 sound, int16 slot)` (0x4E07A0)
    pub fn bank_load_sound(this: GPtr<u8>, bank: u32, sound: u32, slot: u32) = thiscall 0x4E07A0 => unported;
    /// `CAEMP3BankLoader::IsSoundLoaded(uint16 bank, uint16 sound, int16 slot)` (0x4E03B0)
    pub fn bank_is_sound_loaded(this: GPtr<u8>, bank: u32, sound: u32, slot: u32) -> u8 = thiscall 0x4E03B0 => unported;
    /// `CAEMP3BankLoader::GetSoundLoadingStatus(uint16 bank, uint16 sound, int16 slot)` (0x4E0400)
    pub fn bank_get_sound_loading_status(this: GPtr<u8>, bank: u32, sound: u32, slot: u32) -> u8 = thiscall 0x4E0400 => unported;
    /// `CAEMP3TrackLoader::IsCurrentAudioStreamAvailable()` (0x4E0BD0)
    pub fn track_loader_is_current_stream_available(this: GPtr<u8>) -> u8 = thiscall 0x4E0BD0 => unported;
    /// `CAEStreamThread::PlayTrack(uint32 track, int32 next, uint32 offset, int8 flags, bool user, bool nextUser)` (0x4F1230)
    pub fn stream_play_track(this: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32) = thiscall 0x4F1230 => unported;
    /// `CAEStreamThread::StopTrack()` (0x4F1580)
    pub fn stream_stop_track(this: GPtr<u8>) = thiscall 0x4F1580 => unported;
    /// `CAEStreamThread::GetTrackPlayTime()` (0x4F1530)
    pub fn stream_get_track_play_time(this: GPtr<u8>) -> u32 = thiscall 0x4F1530 => unported;
    /// `CAEStreamThread::GetTrackLengthMs()` (0x4F1550)
    pub fn stream_get_track_length_ms(this: GPtr<u8>) -> u32 = thiscall 0x4F1550 => unported;
    /// `CAEStreamThread::GetActiveTrackID()` (0x4F1560)
    pub fn stream_get_active_track_id(this: GPtr<u8>) -> u32 = thiscall 0x4F1560 => unported;
    /// `CAEStreamThread::GetPlayingTrackID()` (0x4F1570)
    pub fn stream_get_playing_track_id(this: GPtr<u8>) -> u32 = thiscall 0x4F1570 => unported;
    /// `CAEStreamingChannel::SetBassEQ(uint8 setting, float gain)` (0x4F1F30)
    pub fn streaming_set_bass_eq(this: GPtr<u8>, setting: u32, gain: u32) = thiscall 0x4F1F30 => unported;
    /// `CAEAudioChannel::SetPosition(const CVector*)` (0x4D7950)
    pub fn channel_set_position(this: GPtr<u8>, pos: u32) -> u32 = thiscall 0x4D7950 => unported;
    /// `CAEAudioHardware::RescaleChannelVolumes()` (0x4D8990)
    pub fn rescale_channel_volumes(this: GPtr<u8>) -> u32 = thiscall 0x4D8990 => unported;
    /// `CAEAudioChannel` vtable +0x0C (`GetPlayTime`; AX)
    pub fn channel_v_get_play_time(this: GPtr<u8>) -> u32 = thiscall vslot 0x0C => unported;
    /// `CAEAudioChannel` vtable +0x1C (`Stop`)
    pub fn channel_v_stop(this: GPtr<u8>) -> u32 = thiscall vslot 0x1C => unported;
    /// `CAEAudioChannel` vtable +0x20 (`SynchPlayback`-slot: `Pause`-like, argument 0)
    pub fn channel_v_slot20(this: GPtr<u8>, a: u32) -> u32 = thiscall vslot 0x20 => unported;
    /// `CAEStreamingChannel` vtable +0x14 (`Play(int16 startOffset, int8 flags, float freq)`)
    pub fn streaming_v_play(this: GPtr<u8>, offset: u32, flags: u32, freq: u32) = thiscall vslot 0x14 => unported;
}

fn bank_loader(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0xD98)
}

fn stream_thread(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(0xE14)
}

fn streaming_channel(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0xE10)
}

/// The channel index `slot + idx` when `slot >= 0` and `idx < channels[slot]` (u16 compare), else None.
fn channel_index(this: GPtr<u8>, slot: u32, idx: u32) -> Option<isize> {
    let s = slot as u16 as i16;
    if s < 0 {
        return None;
    }
    let s = s as isize;
    let i = idx as u16;
    if i >= rd::<u16>(this, 0x90 + s * 2) {
        return None;
    }
    Some(i as isize + s)
}

fn channel(this: GPtr<u8>, ch: isize) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0xE64 + ch * 4)
}

/// `fld dword` / `fstp dword` of a float word (a signalling NaN comes out quiet, as on the x87).
fn fld_fstp(v: u32) -> f32 {
    f32_of(x(f32::from_bits(v)))
}

/// `CAEAudioHardware::GetChannelPlayTimes(int16 slot, int16* out)` (0x4D8820): for `out != null`,
/// stores the play time (vtable +0x0C, AX) of channels `slot .. slot + n - 2` where `n` =
/// `channels[slot]` — one channel fewer than the slot has (the original's `lea edx, [edx+eax-1]` bound,
/// compared unsigned), as the bytes.
#[hook(addr = 0x4D8820, conv = thiscall, name = "CAEAudioHardware::GetChannelPlayTimes", expect = "0F BF 44 24 04", this = "CAEAudioHardware")]
pub fn get_channel_play_times(this: GPtr<u8>, slot: u32, out: GPtr<u16>) {
    let s = slot as u16 as i16 as i32;
    let n = u32::from(rd::<u16>(this, 0x90 + s as isize * 2));
    if out.is_null() {
        return;
    }
    let end = n.wrapping_add(s as u32).wrapping_sub(1);
    if s as u32 >= end {
        return;
    }
    let count = end.wrapping_sub(s as u32);
    for k in 0..count {
        let ch = rd::<GPtr<u8>>(this, 0xE64 + (s as isize + k as isize) * 4);
        let t = channel_v_get_play_time(ch);
        out.wrapping_index(k as i32).write(t as u16);
    }
}

/// `CAEAudioHardware::LoadSoundBank(uint16 bank, int16 slot)` (0x4D88A0): the bank loader's
/// `LoadSoundBank` unless effects loading is disabled (+1).
#[hook(addr = 0x4D88A0, conv = thiscall, name = "CAEAudioHardware::LoadSoundBank", expect = "8A 41 01 84 C0", this = "CAEAudioHardware")]
pub fn load_sound_bank(this: GPtr<u8>, bank: u32, slot: u32) {
    if rd::<u8>(this, 1) == 0 {
        bank_load_sound_bank(bank_loader(this), bank, slot);
    }
}

/// `CAEAudioHardware::IsSoundBankLoaded(uint16 bank, int16 slot)` (0x4D88C0): the bank loader's (tail jump).
#[hook(addr = 0x4D88C0, conv = thiscall, name = "CAEAudioHardware::IsSoundBankLoaded", expect = "8B 89 98 0D 00 00", this = "CAEAudioHardware")]
pub fn is_sound_bank_loaded(this: GPtr<u8>, bank: u32, slot: u32) -> u8 {
    bank_is_sound_bank_loaded(bank_loader(this), bank, slot)
}

/// `CAEAudioHardware::GetSoundBankLoadingStatus(uint16 bank, int16 slot)` (0x4D88D0): the bank loader's (tail jump).
#[hook(addr = 0x4D88D0, conv = thiscall, name = "CAEAudioHardware::GetSoundBankLoadingStatus", expect = "8B 89 98 0D 00 00", this = "CAEAudioHardware")]
pub fn get_sound_bank_loading_status(this: GPtr<u8>, bank: u32, slot: u32) -> u8 {
    bank_get_sound_bank_loading_status(bank_loader(this), bank, slot)
}

/// `CAEAudioHardware::StopSound(int16 slot, uint16 idx)` (0x4D88E0): channel vtable +0x1C when the
/// channel index is valid and the channel exists.
#[hook(addr = 0x4D88E0, conv = thiscall, name = "CAEAudioHardware::StopSound", expect = "66 8B 44 24 04", this = "CAEAudioHardware")]
pub fn stop_sound(this: GPtr<u8>, slot: u32, idx: u32) {
    if let Some(ch) = channel_index(this, slot, idx) {
        let c = channel(this, ch);
        if !c.is_null() {
            channel_v_stop(c);
        }
    }
}

/// `CAEAudioHardware::SetChannelPosition(int16 slot, uint16 idx, CVector* pos, uint8)` (0x4D8920):
/// `CAEAudioChannel::SetPosition(pos)` on a valid, existing channel.
#[hook(addr = 0x4D8920, conv = thiscall, name = "CAEAudioHardware::SetChannelPosition", expect = "66 8B 44 24 04", this = "CAEAudioHardware")]
pub fn set_channel_position(this: GPtr<u8>, slot: u32, idx: u32, pos: u32, _unused: u32) {
    if let Some(ch) = channel_index(this, slot, idx) {
        let c = channel(this, ch);
        if !c.is_null() {
            channel_set_position(c, pos);
        }
    }
}

/// `CAEAudioHardware::SetChannelFrequencyScalingFactor(int16 slot, uint16 idx, float)` (0x4D8960):
/// stores the factor (`fld`/`fstp`) at +0x310 for a valid channel index.
#[hook(addr = 0x4D8960, conv = thiscall, name = "CAEAudioHardware::SetChannelFrequencyScalingFactor", expect = "66 8B 44 24 04", this = "CAEAudioHardware")]
pub fn set_channel_frequency_scaling_factor(this: GPtr<u8>, slot: u32, idx: u32, factor: u32) {
    if let Some(ch) = channel_index(this, slot, idx) {
        wr::<f32>(this, 0x310 + ch * 4, fld_fstp(factor));
    }
}

/// `CAEAudioHardware::GetSoundHeadroom(uint16 sound, int16 slot)` (0x4D8E30): the bank loader's (tail jump).
#[hook(addr = 0x4D8E30, conv = thiscall, name = "CAEAudioHardware::GetSoundHeadroom", expect = "8B 89 98 0D 00 00", this = "CAEAudioHardware")]
pub fn get_sound_headroom(this: GPtr<u8>, sound: u32, slot: u32) -> u32 {
    bank_get_sound_headroom(bank_loader(this), sound, slot)
}

/// `CAEAudioHardware::EnableEffectsLoading()` (0x4D8E40): clears +1.
#[hook(addr = 0x4D8E40, conv = thiscall, name = "CAEAudioHardware::EnableEffectsLoading", expect = "C6 41 01 00 C3", this = "CAEAudioHardware")]
pub fn enable_effects_loading(this: GPtr<u8>) {
    wr::<u8>(this, 1, 0);
}

/// `CAEAudioHardware::DisableEffectsLoading()` (0x4D8E50): sets +1.
#[hook(addr = 0x4D8E50, conv = thiscall, name = "CAEAudioHardware::DisableEffectsLoading", expect = "C6 41 01 01 C3", this = "CAEAudioHardware")]
pub fn disable_effects_loading(this: GPtr<u8>) {
    wr::<u8>(this, 1, 1);
}

/// `CAEAudioHardware::RequestVirtualChannelSoundInfo(uint16 idx, uint16 soundId, uint16 bankSlot)`
/// (0x4D8E60): for `idx < 300`, word 3 at +0x430 and word 2 at +0x688.
#[hook(addr = 0x4D8E60, conv = thiscall, name = "CAEAudioHardware::RequestVirtualChannelSoundInfo", expect = "66 8B 44 24 04", this = "CAEAudioHardware")]
pub fn request_virtual_channel_sound_info(this: GPtr<u8>, idx: u32, a2: u32, a3: u32) {
    let i = idx as u16;
    if i >= 300 {
        return;
    }
    let i = i as isize;
    wr::<u16>(this, 0x430 + i * 2, a3 as u16);
    wr::<u16>(this, 0x688 + i * 2, a2 as u16);
}

/// Copies 0x96 dwords from `this + off` to `out` (`rep movsd`).
fn copy_dwords(this: GPtr<u8>, off: isize, out: GPtr<u32>) {
    for k in 0..0x96isize {
        out.wrapping_index(k as i32).write(rd::<u32>(this, off + k * 4));
    }
}

/// `CAEAudioHardware::GetVirtualChannelSoundLengths(int16* out)` (0x4D8E90): 0x96 dwords from +0xB38.
#[hook(addr = 0x4D8E90, conv = thiscall, name = "CAEAudioHardware::GetVirtualChannelSoundLengths", expect = "56 57 8B 7C 24 0C", this = "CAEAudioHardware")]
pub fn get_virtual_channel_sound_lengths(this: GPtr<u8>, out: GPtr<u32>) {
    copy_dwords(this, 0xB38, out);
}

/// `CAEAudioHardware::GetVirtualChannelSoundLoopStartTimes(int16* out)` (0x4D8EB0): 0x96 dwords from +0x8E0.
#[hook(addr = 0x4D8EB0, conv = thiscall, name = "CAEAudioHardware::GetVirtualChannelSoundLoopStartTimes", expect = "56 57 8B 7C 24 0C", this = "CAEAudioHardware")]
pub fn get_virtual_channel_sound_loop_start_times(this: GPtr<u8>, out: GPtr<u32>) {
    copy_dwords(this, 0x8E0, out);
}

/// `CAEAudioHardware::LoadSound(uint16 bank, uint16 sound, int16 slot)` (0x4D8ED0): the bank loader's
/// `LoadSound` unless effects loading is disabled (+1).
#[hook(addr = 0x4D8ED0, conv = thiscall, name = "CAEAudioHardware::LoadSound", expect = "8A 41 01 84 C0", this = "CAEAudioHardware")]
pub fn load_sound(this: GPtr<u8>, bank: u32, sound: u32, slot: u32) {
    if rd::<u8>(this, 1) == 0 {
        bank_load_sound(bank_loader(this), bank, sound, slot);
    }
}

/// `CAEAudioHardware::IsSoundLoaded(uint16 bank, uint16 sound, int16 slot)` (0x4D8EF0): the bank loader's (tail jump).
#[hook(addr = 0x4D8EF0, conv = thiscall, name = "CAEAudioHardware::IsSoundLoaded", expect = "8B 89 98 0D 00 00", this = "CAEAudioHardware")]
pub fn is_sound_loaded(this: GPtr<u8>, bank: u32, sound: u32, slot: u32) -> u8 {
    bank_is_sound_loaded(bank_loader(this), bank, sound, slot)
}

/// `CAEAudioHardware::GetSoundLoadingStatus(uint16 bank, uint16 sound, int16 slot)` (0x4D8F00): the bank loader's (tail jump).
#[hook(addr = 0x4D8F00, conv = thiscall, name = "CAEAudioHardware::GetSoundLoadingStatus", expect = "8B 89 98 0D 00 00", this = "CAEAudioHardware")]
pub fn get_sound_loading_status(this: GPtr<u8>, bank: u32, sound: u32, slot: u32) -> u8 {
    bank_get_sound_loading_status(bank_loader(this), bank, sound, slot)
}

/// `CAEAudioHardware::PlayTrack(uint32, int32, uint32, int8 flags, bool, bool)` (0x4D8F10): stores the
/// flags byte at +0x1010, then the stream thread's `PlayTrack` with the same arguments (tail jump).
#[hook(addr = 0x4D8F10, conv = thiscall, name = "CAEAudioHardware::PlayTrack", expect = "8B 44 24 10 88 81 10 10 00 00", this = "CAEAudioHardware")]
#[allow(clippy::too_many_arguments)]
pub fn play_track(this: GPtr<u8>, a0: u32, a1: u32, a2: u32, flags: u32, a4: u32, a5: u32) {
    wr::<u8>(this, 0x1010, flags as u8);
    stream_play_track(stream_thread(this), a0, a1, a2, flags, a4, a5);
}

/// `CAEAudioHardware::StartTrackPlayback()` (0x4D8F30): the streaming channel's `Play(0, flags, 1.0)`
/// (vtable +0x14) with the sign-extended flags byte at +0x1010; EAX is the call's.
#[hook(addr = 0x4D8F30, conv = thiscall, name = "CAEAudioHardware::StartTrackPlayback", expect = "8B C1 8B 88 10 0E 00 00", this = "CAEAudioHardware")]
pub fn start_track_playback(this: GPtr<u8>) {
    let flags = rd::<u8>(this, 0x1010) as i8 as i32 as u32;
    streaming_v_play(streaming_channel(this), 0, flags, 0x3F80_0000);
}

/// `CAEAudioHardware::StopTrack()` (0x4D8F50): the stream thread's (tail jump).
#[hook(addr = 0x4D8F50, conv = thiscall, name = "CAEAudioHardware::StopTrack", expect = "81 C1 14 0E 00 00", this = "CAEAudioHardware")]
pub fn stop_track(this: GPtr<u8>) {
    stream_stop_track(stream_thread(this));
}

/// `CAEAudioHardware::GetTrackPlayTime()` (0x4D8F60): the stream thread's (tail jump).
#[hook(addr = 0x4D8F60, conv = thiscall, name = "CAEAudioHardware::GetTrackPlayTime", expect = "81 C1 14 0E 00 00", this = "CAEAudioHardware")]
pub fn get_track_play_time(this: GPtr<u8>) -> u32 {
    stream_get_track_play_time(stream_thread(this))
}

/// `CAEAudioHardware::GetTrackLengthMs()` (0x4D8F70): the stream thread's (tail jump).
#[hook(addr = 0x4D8F70, conv = thiscall, name = "CAEAudioHardware::GetTrackLengthMs", expect = "81 C1 14 0E 00 00", this = "CAEAudioHardware")]
pub fn get_track_length_ms(this: GPtr<u8>) -> u32 {
    stream_get_track_length_ms(stream_thread(this))
}

/// `CAEAudioHardware::GetActiveTrackID()` (0x4D8F80): the stream thread's (tail jump).
#[hook(addr = 0x4D8F80, conv = thiscall, name = "CAEAudioHardware::GetActiveTrackID", expect = "81 C1 14 0E 00 00", this = "CAEAudioHardware")]
pub fn get_active_track_id(this: GPtr<u8>) -> u32 {
    stream_get_active_track_id(stream_thread(this))
}

/// `CAEAudioHardware::GetPlayingTrackID()` (0x4D8F90): the stream thread's (tail jump).
#[hook(addr = 0x4D8F90, conv = thiscall, name = "CAEAudioHardware::GetPlayingTrackID", expect = "81 C1 14 0E 00 00", this = "CAEAudioHardware")]
pub fn get_playing_track_id(this: GPtr<u8>) -> u32 {
    stream_get_playing_track_id(stream_thread(this))
}

/// `CAEAudioHardware::SetBassSetting(int8 setting, float gain)` (0x4D94A0, body continued in the
/// `.text` chunk 0x4014AA): +0xD90 = the sign-extended setting (a dword store), +0xD94 = gain, then the
/// streaming channel's `SetBassEQ(setting, gain)` with the argument words unchanged (tail jump).
#[hook(addr = 0x4D94A0, conv = thiscall, name = "CAEAudioHardware::SetBassSetting", expect = "8B 44 24 04 0F BE D0", this = "CAEAudioHardware")]
pub fn set_bass_setting(this: GPtr<u8>, setting: u32, gain: u32) {
    wr::<i32>(this, 0xD90, setting as u8 as i8 as i32);
    wr::<u32>(this, 0xD94, gain);
    streaming_set_bass_eq(streaming_channel(this), setting, gain);
}

/// `CAEAudioHardware::DisableBassEq()` (0x4D94D0): the streaming channel's `SetBassEQ(0, 0)`.
#[hook(addr = 0x4D94D0, conv = thiscall, name = "CAEAudioHardware::DisableBassEq", expect = "8B 89 10 0E 00 00", this = "CAEAudioHardware")]
pub fn disable_bass_eq(this: GPtr<u8>) {
    streaming_set_bass_eq(streaming_channel(this), 0, 0);
}

/// `CAEAudioHardware::EnableBassEq()` (0x4D94E0): the streaming channel's `SetBassEQ(byte +0xD90, +0xD94)`.
#[hook(addr = 0x4D94E0, conv = thiscall, name = "CAEAudioHardware::EnableBassEq", expect = "8B 81 94 0D 00 00", this = "CAEAudioHardware")]
pub fn enable_bass_eq(this: GPtr<u8>) {
    let gain = rd::<u32>(this, 0xD94);
    let setting = u32::from(rd::<u8>(this, 0xD90));
    streaming_set_bass_eq(streaming_channel(this), setting, gain);
}

/// `CAEAudioHardware::SetChannelFlags(int16 slot, uint16 idx, int16 flags)` (0x4D9500): the word at +6
/// for a valid channel index.
#[hook(addr = 0x4D9500, conv = thiscall, name = "CAEAudioHardware::SetChannelFlags", expect = "66 8B 44 24 04", this = "CAEAudioHardware")]
pub fn set_channel_flags(this: GPtr<u8>, slot: u32, idx: u32, flags: u32) {
    if let Some(ch) = channel_index(this, slot, idx) {
        wr::<u16>(this, 6 + ch * 2, flags as u16);
    }
}

/// `CAEAudioHardware::SetMusicMasterScalingFactor(float)` (0x4D9530): +0x410 (the word as passed).
#[hook(addr = 0x4D9530, conv = thiscall, name = "CAEAudioHardware::SetMusicMasterScalingFactor", expect = "8B 44 24 04 89 81 10 04 00 00", this = "CAEAudioHardware")]
pub fn set_music_master_scaling_factor(this: GPtr<u8>, f: u32) {
    wr::<u32>(this, 0x410, f);
}

/// `CAEAudioHardware::SetEffectsMasterScalingFactor(float)` (0x4D9540): +0x414.
#[hook(addr = 0x4D9540, conv = thiscall, name = "CAEAudioHardware::SetEffectsMasterScalingFactor", expect = "8B 44 24 04 89 81 14 04 00 00", this = "CAEAudioHardware")]
pub fn set_effects_master_scaling_factor(this: GPtr<u8>, f: u32) {
    wr::<u32>(this, 0x414, f);
}

/// `CAEAudioHardware::SetMusicFaderScalingFactor(float)` (0x4D9550): +0x418.
#[hook(addr = 0x4D9550, conv = thiscall, name = "CAEAudioHardware::SetMusicFaderScalingFactor", expect = "8B 44 24 04 89 81 18 04 00 00", this = "CAEAudioHardware")]
pub fn set_music_fader_scaling_factor(this: GPtr<u8>, f: u32) {
    wr::<u32>(this, 0x418, f);
}

/// `CAEAudioHardware::SetEffectsFaderScalingFactor(float)` (0x4D9560): +0x41C.
#[hook(addr = 0x4D9560, conv = thiscall, name = "CAEAudioHardware::SetEffectsFaderScalingFactor", expect = "8B 44 24 04 89 81 1C 04 00 00", this = "CAEAudioHardware")]
pub fn set_effects_fader_scaling_factor(this: GPtr<u8>, f: u32) {
    wr::<u32>(this, 0x41C, f);
}

/// `CAEAudioHardware::GetEffectsMasterScalingFactor()` (0x4D9570): `fld` +0x414.
#[hook(addr = 0x4D9570, conv = thiscall, name = "CAEAudioHardware::GetEffectsMasterScalingFactor", expect = "D9 81 14 04 00 00", this = "CAEAudioHardware")]
pub fn get_effects_master_scaling_factor(this: GPtr<u8>) -> X87 {
    x(rd::<f32>(this, 0x414))
}

/// `CAEAudioHardware::GetMusicMasterScalingFactor()` (0x4D9580): `fld` +0x410.
#[hook(addr = 0x4D9580, conv = thiscall, name = "CAEAudioHardware::GetMusicMasterScalingFactor", expect = "D9 81 10 04 00 00", this = "CAEAudioHardware")]
pub fn get_music_master_scaling_factor(this: GPtr<u8>) -> X87 {
    x(rd::<f32>(this, 0x410))
}

/// `CAEAudioHardware::GetEffectsFaderScalingFactor()` (0x4D9590): `fld` +0x41C.
#[hook(addr = 0x4D9590, conv = thiscall, name = "CAEAudioHardware::GetEffectsFaderScalingFactor", expect = "D9 81 1C 04 00 00", this = "CAEAudioHardware")]
pub fn get_effects_fader_scaling_factor(this: GPtr<u8>) -> X87 {
    x(rd::<f32>(this, 0x41C))
}

/// `CAEAudioHardware::SetNonStreamFaderScalingFactor(float)` (0x4D95A0): +0x420.
#[hook(addr = 0x4D95A0, conv = thiscall, name = "CAEAudioHardware::SetNonStreamFaderScalingFactor", expect = "8B 44 24 04 89 81 20 04 00 00", this = "CAEAudioHardware")]
pub fn set_non_stream_fader_scaling_factor(this: GPtr<u8>, f: u32) {
    wr::<u32>(this, 0x420, f);
}

/// `CAEAudioHardware::SetStreamFaderScalingFactor(float)` (0x4D95B0): +0x424.
#[hook(addr = 0x4D95B0, conv = thiscall, name = "CAEAudioHardware::SetStreamFaderScalingFactor", expect = "8B 44 24 04 89 81 24 04 00 00", this = "CAEAudioHardware")]
pub fn set_stream_fader_scaling_factor(this: GPtr<u8>, f: u32) {
    wr::<u32>(this, 0x424, f);
}

/// `CAEAudioHardware::IsStreamingFromDVD()` (0x4D95C0): AL = byte +0x14 of the track loader (+0xD9C).
#[hook(addr = 0x4D95C0, conv = thiscall, name = "CAEAudioHardware::IsStreamingFromDVD", expect = "8B 81 9C 0D 00 00", this = "CAEAudioHardware")]
pub fn is_streaming_from_dvd(this: GPtr<u8>) -> u8 {
    rd::<u8>(rd::<GPtr<u8>>(this, 0xD9C), 0x14)
}

/// `CAEAudioHardware::CheckDVD()` (0x4D95E0): the track loader's `IsCurrentAudioStreamAvailable` (tail jump).
#[hook(addr = 0x4D95E0, conv = thiscall, name = "CAEAudioHardware::CheckDVD", expect = "8B 89 9C 0D 00 00", this = "CAEAudioHardware")]
pub fn check_dvd(this: GPtr<u8>) -> u8 {
    track_loader_is_current_stream_available(rd::<GPtr<u8>>(this, 0xD9C))
}

/// `CAEAudioHardware::PauseAllSounds()` (0x4D95F0): when initialised, channel vtable +0x20 with 0 on
/// every channel (the count +0x8E is re-read after every call).
#[hook(addr = 0x4D95F0, conv = thiscall, name = "CAEAudioHardware::PauseAllSounds", expect = "53 8B D9 80 3B 00", this = "CAEAudioHardware")]
pub fn pause_all_sounds(this: GPtr<u8>) {
    if rd::<u8>(this, 0) == 0 {
        return;
    }
    let mut i: i32 = 0;
    if rd::<u16>(this, 0x8E) == 0 {
        return;
    }
    loop {
        channel_v_slot20(channel(this, i as isize), 0);
        i += 1;
        if i >= i32::from(rd::<u16>(this, 0x8E)) {
            break;
        }
    }
}

/// `CAEAudioHardware::ResumeAllSounds()` (0x4D9630): `RescaleChannelVolumes()` when initialised (tail jump).
#[hook(addr = 0x4D9630, conv = thiscall, name = "CAEAudioHardware::ResumeAllSounds", expect = "80 39 00 74 05", this = "CAEAudioHardware")]
pub fn resume_all_sounds(this: GPtr<u8>) {
    if rd::<u8>(this, 0) != 0 {
        rescale_channel_volumes(this);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_CHANNEL_PLAY_TIMES,
    HOOK_LOAD_SOUND_BANK,
    HOOK_IS_SOUND_BANK_LOADED,
    HOOK_GET_SOUND_BANK_LOADING_STATUS,
    HOOK_STOP_SOUND,
    HOOK_SET_CHANNEL_POSITION,
    HOOK_SET_CHANNEL_FREQUENCY_SCALING_FACTOR,
    HOOK_GET_SOUND_HEADROOM,
    HOOK_ENABLE_EFFECTS_LOADING,
    HOOK_DISABLE_EFFECTS_LOADING,
    HOOK_REQUEST_VIRTUAL_CHANNEL_SOUND_INFO,
    HOOK_GET_VIRTUAL_CHANNEL_SOUND_LENGTHS,
    HOOK_GET_VIRTUAL_CHANNEL_SOUND_LOOP_START_TIMES,
    HOOK_LOAD_SOUND,
    HOOK_IS_SOUND_LOADED,
    HOOK_GET_SOUND_LOADING_STATUS,
    HOOK_PLAY_TRACK,
    HOOK_START_TRACK_PLAYBACK,
    HOOK_STOP_TRACK,
    HOOK_GET_TRACK_PLAY_TIME,
    HOOK_GET_TRACK_LENGTH_MS,
    HOOK_GET_ACTIVE_TRACK_ID,
    HOOK_GET_PLAYING_TRACK_ID,
    HOOK_SET_BASS_SETTING,
    HOOK_DISABLE_BASS_EQ,
    HOOK_ENABLE_BASS_EQ,
    HOOK_SET_CHANNEL_FLAGS,
    HOOK_SET_MUSIC_MASTER_SCALING_FACTOR,
    HOOK_SET_EFFECTS_MASTER_SCALING_FACTOR,
    HOOK_SET_MUSIC_FADER_SCALING_FACTOR,
    HOOK_SET_EFFECTS_FADER_SCALING_FACTOR,
    HOOK_GET_EFFECTS_MASTER_SCALING_FACTOR,
    HOOK_GET_MUSIC_MASTER_SCALING_FACTOR,
    HOOK_GET_EFFECTS_FADER_SCALING_FACTOR,
    HOOK_SET_NON_STREAM_FADER_SCALING_FACTOR,
    HOOK_SET_STREAM_FADER_SCALING_FACTOR,
    HOOK_IS_STREAMING_FROM_DVD,
    HOOK_CHECK_DVD,
    HOOK_PAUSE_ALL_SOUNDS,
    HOOK_RESUME_ALL_SOUNDS,
];
