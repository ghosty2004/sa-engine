//! Beat information of the playing audio stream: `CAEAudioHardware::GetBeatInfo` (0x4D8FA0) and the track-info
//! loader it uses, `CAEMP3TrackLoader::GetTrackInfo` (0x4E0A70). Ported from the 1.0 US bytes; gta-reversed
//! (`AEAudioHardware.cpp`, `AEMP3TrackLoader.cpp`) differs from them and is only a reading aid:
//! - with no active track the original returns at once and copies nothing to the caller's buffer;
//! - while beat info was present the window starts 50 ms back (clamped at 0) and every copied beat time is made
//!   relative to that adjusted start, not to the play time;
//! - the track-id check is `trackId > count` (an id equal to the count passes), the pack check `packId > count`.
//!
//! `CAEAudioHardware` beat state (inside the hardware object): +0xF64 beat window, 20 `{time, key}` pairs
//! (10 before the current beat, 10 from it), +0x1004 "beat info present", +0x1008 beat type this frame, +0x100C
//! index of the current beat (-1 none); the caller's `tBeatInfo` receives +0xF64..+0x1010 (0x2B dwords).
//! `tTrackInfo` (0x1F84 bytes): 1000 `{time, key}` beats at +0, read from the stream file's head.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::port4_hw::{r8, r32, w32};

const BEAT_WINDOW: u32 = 0xF64;
const BEAT_PRESENT: u32 = 0x1004;
const BEAT_TYPE_THIS_FRAME: u32 = 0x1008;
const BEAT_NUMBER: u32 = 0x100C;
/// `CAEAudioHardware` +0xE14: its `CAEStreamThread`; +0xD9C: the `CAEMP3TrackLoader*`.
const STREAM_THREAD: u32 = 0xE14;
const TRACK_LOADER: u32 = 0xD9C;
/// beats per `tTrackInfo`
const MAX_BEATS: u32 = 1000;
const TRACK_INFO_SIZE: u32 = 0x1F84;
/// "AUDIO\STREAMS\" in .rdata
const STREAMS_DIR: u32 = 0x85F1BC;

orig! {
    /// `CAEStreamThread::GetActiveTrackID()` (0x4F1560)
    fn stream_thread_get_active_track_id(this: u32) -> u32 = thiscall 0x4F1560 => unported;
    /// `CAEStreamThread::GetTrackPlayTime()` (0x4F1530)
    fn stream_thread_get_track_play_time(this: u32) -> u32 = thiscall 0x4F1530 => unported;
    /// `CAEMP3TrackLoader::GetTrackInfo(uint32)` (0x4E0A70)
    fn track_loader_get_track_info(this: u32, track: u32) -> u32 = thiscall 0x4E0A70 => unported;
    /// CRT `operator new` (0x82119A)
    fn operator_new(size: u32) -> u32 = cdecl 0x82119A => unported;
    /// CRT `operator delete` / `free` (0x8214BD)
    fn operator_delete(p: u32) = cdecl 0x8214BD => unported;
    /// `CAEDataStream::CAEDataStream(int32 track, char* name, uint32 start, uint32 length, bool encrypted)` (0x4DC620)
    fn data_stream_ctor(this: u32, track: u32, name: u32, start: u32, length: u32, encrypted: u32) -> u32 = thiscall 0x4DC620 => unported;
    /// `CAEDataStream::Initialise()` (0x4DC2B0)
    fn data_stream_initialise(this: u32) -> u8 = thiscall 0x4DC2B0 => unported;
    /// `CAEDataStream::FillBuffer(void*, uint32)` (0x4DC1C0)
    fn data_stream_fill_buffer(this: u32, dest: u32, size: u32) -> u32 = thiscall 0x4DC1C0 => unported;
    /// `CAEDataStream::~CAEDataStream()` (0x4DC490)
    fn data_stream_dtor(this: u32) = thiscall 0x4DC490 => unported;
}

fn zero_window(this: u32) {
    for i in 0..0x28 {
        w32(this + BEAT_WINDOW + 4 * i, 0);
    }
}

/// Window slot `slot` = beat `index` of `beats`, its time relative to `base`.
fn put(this: u32, slot: u32, beats: u32, index: u32, base: u32) {
    w32(this + BEAT_WINDOW + 8 * slot, r32(beats + 8 * index).wrapping_sub(base));
    w32(this + BEAT_WINDOW + 8 * slot + 4, r32(beats + 8 * index + 4));
}

/// The 10 beats before beat `i` into slots 9 (beat `i - 1`) down to 0.
fn put_before(this: u32, beats: u32, i: i32, base: u32) {
    for k in 0..10 {
        let index = i - 1 - k;
        if index >= 0 {
            put(this, (9 - k) as u32, beats, index as u32, base);
        }
    }
}

/// `CAEAudioHardware::GetBeatInfo(tBeatInfo*)` (0x4D8FA0).
#[hook(addr = 0x4D8FA0, conv = thiscall, name = "CAEAudioHardware::GetBeatInfo", expect = "51 56 8B F1 57",
       unverified = "batch", route = "L2", hazards = "heap-writes", recs = "grove-walk", note = "the corpus may not reach it: a recording with an active beat track (dance / lowrider) verifies it")]
pub fn hw_get_beat_info(this: u32, out: u32) {
    let thread = this + STREAM_THREAD;
    if stream_thread_get_active_track_id(thread) as i32 == -1 {
        w32(this + BEAT_PRESENT, 0);
        zero_window(this);
        return;
    }
    let track = stream_thread_get_active_track_id(thread);
    let beats = track_loader_get_track_info(r32(this + TRACK_LOADER), track);
    let mut base = stream_thread_get_track_play_time(thread);
    if (base as i32) < 0 || (r32(beats) as i32) < 0 {
        if base as i32 != -7 {
            w32(this + BEAT_PRESENT, 0);
            zero_window(this);
        }
    } else {
        let previous = r32(this + BEAT_NUMBER) as i32;
        if previous >= 0 {
            // 50 ms back, clamped at 0
            base = (base as i32 - 50).max(0) as u32;
        }
        w32(this + BEAT_PRESENT, 1);
        w32(this + BEAT_TYPE_THIS_FRAME, 0);
        w32(this + BEAT_NUMBER, 0xFFFF_FFFF);
        zero_window(this);
        // the first beat at or after the window start (signed compares, as the original's jge/jl)
        let mut i: i32 = 0;
        if (r32(beats) as i32) < base as i32 {
            while (r32(beats + 8 * i as u32 + 4) as i32) > 0 {
                i += 1;
                if (r32(beats + 8 * i as u32) as i32) >= base as i32 {
                    break;
                }
            }
        }
        if (r32(beats + 8 * i as u32 + 4) as i32) > 0 {
            if i != previous && i > 0 {
                w32(this + BEAT_TYPE_THIS_FRAME, r32(beats + 8 * (i - 1) as u32 + 4));
            }
            put_before(this, beats, i, base);
            for k in 0..10 {
                let index = i as u32 + k;
                if index < MAX_BEATS && (r32(beats + 8 * index + 4) as i32) > 0 {
                    put(this, 10 + k, beats, index, base);
                }
            }
            w32(this + BEAT_NUMBER, i as u32);
        } else if i > 0 {
            put_before(this, beats, i, base);
        }
    }
    for k in 0..0x2B {
        w32(out + 4 * k, r32(this + BEAT_WINDOW + 4 * k));
    }
    operator_delete(beats);
}

fn strlen(mut p: u32) -> u32 {
    let start = p;
    while r8(p) != 0 {
        p += 1;
    }
    p - start
}

fn strcpy(mut d: u32, mut s: u32) {
    loop {
        let c = r8(s);
        crate::port4_hw::w8(d, c);
        if c == 0 {
            return;
        }
        d += 1;
        s += 1;
    }
}

fn strcat(d: u32, s: u32) {
    strcpy(d + strlen(d), s);
}

/// `CAEMP3TrackLoader::GetTrackInfo(uint32)` (0x4E0A70): the track's beat table (a new `tTrackInfo`, the
/// caller deletes it) read from the head of its stream, or null for an unknown track / pack. Loader layout:
/// +4 track count, +8 stream pack count, +0xC track lookups (12 bytes: +0 pack id byte, +4 offset, +8 size),
/// +0x10 stream packs (16-byte names), +0x14 "streaming from DVD", +0x18 DVD drive path.
#[hook(addr = 0x4E0A70, conv = thiscall, name = "CAEMP3TrackLoader::GetTrackInfo", expect = "6A FF 68 7B BF 83 00",
       unverified = "batch", route = "L2", hazards = "heap-writes", recs = "grove-walk", note = "the corpus may not reach it: a recording with an active beat track (dance / lowrider) verifies it")]
pub fn track_loader_get_track_info_port(this: u32, track: u32) -> u32 {
    if track > r32(this + 4) {
        return 0;
    }
    let lookup = r32(this + 0xC) + 12 * track;
    let pack = u32::from(r8(lookup));
    if pack > r32(this + 8) {
        return 0;
    }
    let dvd = r8(this + 0x14) != 0;
    let name = r32(this + 0x10) + 16 * pack;
    let length = if dvd {
        strlen(name) + strlen(r32(this + 0x18)) + 0x15
    } else {
        strlen(name) + 0x15
    };
    let path = operator_new(length);
    if dvd {
        strcpy(path, r32(this + 0x18));
        strcat(path, STREAMS_DIR);
    } else {
        strcpy(path, STREAMS_DIR);
    }
    // the original re-reads the pack id from the lookup here
    let name = r32(this + 0x10) + 16 * u32::from(r8(r32(this + 0xC) + 12 * track));
    strcat(path, name);
    let info = operator_new(TRACK_INFO_SIZE);
    let mem = operator_new(0x28);
    let stream = if mem != 0 {
        let lookup = r32(this + 0xC) + 12 * track;
        data_stream_ctor(mem, track, path, r32(lookup + 4), r32(lookup + 8), 1)
    } else {
        0
    };
    data_stream_initialise(stream);
    data_stream_fill_buffer(stream, info, TRACK_INFO_SIZE);
    if stream != 0 {
        data_stream_dtor(stream);
        operator_delete(stream);
    }
    info
}

pub const HOOKS: &[HookReg] = &[HOOK_HW_GET_BEAT_INFO, HOOK_TRACK_LOADER_GET_TRACK_INFO_PORT];
