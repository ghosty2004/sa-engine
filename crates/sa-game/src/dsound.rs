//! Native DirectSound 8 for the ported audio hardware layer (`CAEAudioHardware`,
//! `CAEAudioChannel` / `CAEStaticChannel` / `CAEStreamingChannel`, `CAESmoothFadeThread`): the
//! `IDirectSound8` object `EAXDirectSoundCreate` (EAX.DLL ordinal 6) returns, sound buffers
//! (`IDirectSoundBuffer8`), their `IDirectSound3DBuffer` interface, the primary buffer's
//! `IDirectSound3DListener`, and a software mixer that plays the buffers into
//! `sa_platform::audio::AudioOut` (NullAudio by default; the cpal device with
//! `--features audio` and `SA_GAME_AUDIO=cpal`).
//!
//! Same scheme as [`crate::d3d9`]: objects are heap blocks `{vtable, handle}` whose vtable slots
//! are trap addresses registered in `orig!`'s address registry; an unimplemented method fails
//! loudly as `IDirectSoundBuffer8::<method>`. The ports call every method through `orig!` `vslot`
//! declarations exactly as the original does.
//!
//! Semantics (DirectSound 8 as the game observes it):
//! * A buffer's play cursor advances in real time while it plays, at its frequency
//!   (`SetFrequency`, else the format's rate); the mixer thread (every [`MIX_MS`] ms) advances
//!   every playing buffer and mixes the frames it passed into the sink at 44.1 kHz stereo 16 bit
//!   (linear interpolation, gain `10^(volume/2000)`, mono buffers centred). A non-looping buffer
//!   stops at its end (`GetStatus` then reports not playing); a looping one wraps.
//! * The write cursor is the play cursor plus [`WRITE_LEAD_MS`] ms of data (DirectSound's
//!   "safe to write" distance; not measured).
//! * 3D positioning, EAX (`IKsPropertySet` is not offered: `QueryInterface` fails with
//!   `E_NOINTERFACE`, so `Query3DSoundEffects` finds no EAX) and FX are accepted and recorded but
//!   not rendered: the mix is 2D. The primary buffer only carries the listener and the format.
//! * Buffers are never lost (`DSBSTATUS_BUFFERLOST` is never reported).

// COM method signatures are dictated by DirectSound.
#![allow(clippy::too_many_arguments)]
#![allow(unsafe_code)]

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use web_time::Instant;

use sa_core::mem::GPtr;
use sa_platform::audio::{AudioFormat, AudioOut, NullAudio};

use crate::bind::with;

/// The page holding the vtables.
const PAGE: usize = 0x0003_3000;
/// Method traps: `TRAP + 0x400 * class + 4 * slot`.
const TRAP: u32 = 0x0007_0000;
/// Mixer period.
pub const MIX_MS: u64 = 5;
/// Write-cursor lead over the play cursor.
pub const WRITE_LEAD_MS: u32 = 15;

const DS8_METHODS: [&str; 12] = [
    "QueryInterface",
    "AddRef",
    "Release",
    "CreateSoundBuffer",
    "GetCaps",
    "DuplicateSoundBuffer",
    "SetCooperativeLevel",
    "Compact",
    "GetSpeakerConfig",
    "SetSpeakerConfig",
    "Initialize",
    "VerifyCertification",
];
const BUFFER_METHODS: [&str; 24] = [
    "QueryInterface",
    "AddRef",
    "Release",
    "GetCaps",
    "GetCurrentPosition",
    "GetFormat",
    "GetVolume",
    "GetPan",
    "GetFrequency",
    "GetStatus",
    "Initialize",
    "Lock",
    "Play",
    "SetCurrentPosition",
    "SetFormat",
    "SetVolume",
    "SetPan",
    "SetFrequency",
    "Stop",
    "Unlock",
    "Restore",
    "SetFX",
    "AcquireResources",
    "GetObjectInPath",
];
const BUFFER3D_METHODS: [&str; 21] = [
    "QueryInterface",
    "AddRef",
    "Release",
    "GetAllParameters",
    "GetConeAngles",
    "GetConeOrientation",
    "GetConeOutsideVolume",
    "GetMaxDistance",
    "GetMinDistance",
    "GetMode",
    "GetPosition",
    "GetVelocity",
    "SetAllParameters",
    "SetConeAngles",
    "SetConeOrientation",
    "SetConeOutsideVolume",
    "SetMaxDistance",
    "SetMinDistance",
    "SetMode",
    "SetPosition",
    "SetVelocity",
];
const LISTENER_METHODS: [&str; 18] = [
    "QueryInterface",
    "AddRef",
    "Release",
    "GetAllParameters",
    "GetDistanceFactor",
    "GetDopplerFactor",
    "GetOrientation",
    "GetPosition",
    "GetRolloffFactor",
    "GetVelocity",
    "SetAllParameters",
    "SetDistanceFactor",
    "SetDopplerFactor",
    "SetOrientation",
    "SetPosition",
    "SetRolloffFactor",
    "SetVelocity",
    "CommitDeferredSettings",
];

struct Class {
    name: &'static str,
    methods: &'static [&'static str],
}
const CLASSES: [Class; 4] = [
    Class {
        name: "IDirectSound8",
        methods: &DS8_METHODS,
    },
    Class {
        name: "IDirectSoundBuffer8",
        methods: &BUFFER_METHODS,
    },
    Class {
        name: "IDirectSound3DBuffer",
        methods: &BUFFER3D_METHODS,
    },
    Class {
        name: "IDirectSound3DListener",
        methods: &LISTENER_METHODS,
    },
];
const DS8: usize = 0;
const BUFFER: usize = 1;
const BUFFER3D: usize = 2;
const LISTENER: usize = 3;

fn vtable(class: usize) -> u32 {
    (PAGE + 0x100 * class) as u32
}
fn trap(class: usize, slot: usize) -> u32 {
    TRAP + 0x400 * class as u32 + 4 * slot as u32
}

const DS_OK: i32 = 0;
const E_NOINTERFACE: i32 = 0x8000_4002u32 as i32;
const DSERR_INVALIDPARAM: i32 = 0x8007_0057u32 as i32;
const DSERR_INVALIDCALL: i32 = 0x8878_0032u32 as i32;

/// `DSBCAPS_PRIMARYBUFFER`, `DSBCAPS_CTRL3D`.
const DSBCAPS_PRIMARYBUFFER: u32 = 1;
const DSBCAPS_CTRL3D: u32 = 0x10;

/// The IIDs the game asks for (`Data1` of the GUIDs at 0x8593A0 / 0x859390 / 0x859340, read from the
/// image when needed: `IID_IDirectSound3DListener` 279AFA84, `IID_IDirectSound3DBuffer` 279AFA86).
const IID_LISTENER: u32 = 0x279A_FA84;
const IID_BUFFER3D: u32 = 0x279A_FA86;
const IID_BUFFER: u32 = 0x279A_FA85;
const IID_BUFFER8: u32 = 0x6825_A449;

/// One sound buffer.
struct Buffer {
    /// The buffer memory (leaked block; the game writes it through `Lock`).
    mem: u32,
    size: u32,
    flags: u32,
    channels: u32,
    rate: u32,
    bits: u32,
    block: u32,
    /// Playback frequency (0 = the format's rate).
    freq: u32,
    /// Volume in hundredths of a dB (0 = full, -10000 = silence).
    volume: i32,
    pan: i32,
    playing: bool,
    looping: bool,
    /// Play cursor in frames (fractional).
    pos: f64,
    refs: u32,
    /// 3D parameters (recorded only).
    pos3d: [f32; 3],
    min_dist: f32,
    max_dist: f32,
    mode3d: u32,
}

struct State {
    next: u32,
    buffers: HashMap<u32, Buffer>,
    /// The primary buffer's format `(channels, rate, bits)`.
    primary: (u32, u32, u32),
    listener: [f32; 3],
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(s.get_or_insert_with(|| State {
        next: 1,
        buffers: HashMap::new(),
        primary: (2, 44_100, 16),
        listener: [0.0; 3],
    }))
}

fn new_object(class: usize, h: u32) -> u32 {
    let obj: &'static mut [u32; 2] = Box::leak(Box::new([vtable(class), h]));
    obj.as_ptr() as u32
}
fn handle(obj: u32) -> u32 {
    GPtr::<u32>::from_addr(obj as usize).wrapping_index(1).read()
}
fn with_buf<R>(obj: u32, f: impl FnOnce(&mut Buffer) -> R) -> Option<R> {
    let h = handle(obj);
    with_state(|s| s.buffers.get_mut(&h).map(f))
}
fn w32(p: GPtr<u32>, i: i32, v: u32) {
    p.wrapping_index(i).write(v);
}

/// The `IDirectSound8` object (one; `EAXDirectSoundCreate` hands it out).
static DS8_OBJECT: Mutex<u32> = Mutex::new(0);

/// Maps the vtable page and starts the mixer.
pub fn map() -> Result<(), String> {
    crate::arena::map_fixed(PAGE, 0x1000).map_err(|e| format!("{}: {e}", format!("cannot map the DirectSound vtable page at {PAGE:#x}")))?;
    for (c, cl) in CLASSES.iter().enumerate() {
        for i in 0..cl.methods.len() {
            GPtr::<u32>::from_addr(vtable(c) as usize + 4 * i).write(trap(c, i));
        }
    }
    Ok(())
}

/// Every method trap's name.
pub fn names() -> Vec<(u32, &'static str)> {
    let mut v = Vec::new();
    for (c, cl) in CLASSES.iter().enumerate() {
        for (i, m) in cl.methods.iter().enumerate() {
            v.push((
                trap(c, i),
                &*Box::leak(format!("{}::{m}", cl.name).into_boxed_str()),
            ));
        }
    }
    v
}

// ------------------------------------------------------------------ mixer -------------------------

/// The sink the mixer plays into.
fn open_sink() -> Box<dyn AudioOut> {
    #[cfg(feature = "audio")]
    if crate::env::var("SA_GAME_AUDIO").as_deref() == Ok("cpal") {
        match sa_platform::native::CpalAudio::new() {
            Ok(a) => return Box::new(a),
            Err(e) => eprintln!("sa-game: no audio device ({e}); playing into NullAudio"),
        }
    }
    Box::new(NullAudio::default())
}

static MIXER_STARTED: Mutex<bool> = Mutex::new(false);

/// The mixer's sink, the fraction of a frame carried over and its scratch buffers.
struct Mixer {
    sink: Box<dyn AudioOut>,
    carry: f64,
    mix: Vec<i32>,
    pcm: Vec<i16>,
}

static MIXER: Mutex<Option<Mixer>> = Mutex::new(None);

/// Advances every playing buffer by `secs` seconds and mixes the frames passed into the sink (the mixer thread every
/// [`MIX_MS`] ms; the main loop once per iteration in the deterministic mode, `--fixed-step`).
pub fn advance(secs: f64) {
    let out = AudioFormat::GAME;
    let mut g = MIXER.lock().unwrap_or_else(|e| e.into_inner());
    let m = g.get_or_insert_with(|| Mixer {
        sink: open_sink(),
        carry: 0.0,
        mix: Vec::new(),
        pcm: Vec::new(),
    });
    let frames_f = secs * out.sample_rate as f64 + m.carry;
    let frames = frames_f as usize;
    m.carry = frames_f - frames as f64;
    if frames == 0 {
        return;
    }
    m.mix.clear();
    m.mix.resize(frames * 2, 0);
    let mix = &mut m.mix;
    with_state(|s| {
        for b in s.buffers.values_mut() {
            mix_buffer(b, mix, frames, out.sample_rate);
        }
    });
    m.pcm.clear();
    let pcm = &mut m.pcm;
    pcm.extend(m.mix.iter().map(|v| (*v).clamp(-32768, 32767) as i16));
    m.sink.push(&m.pcm);
}

/// wasm32: advances the buffers by the real time since the last call (once per main-loop iteration).
pub fn advance_real_time() {
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    let now = Instant::now();
    let mut l = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(prev) = l.replace(now) {
        advance(now.duration_since(prev).as_secs_f64().min(0.25));
    }
}

fn start_mixer() {
    let mut started = MIXER_STARTED.lock().unwrap_or_else(|e| e.into_inner());
    if *started {
        return;
    }
    *started = true;
    // wasm32: no threads; the main loop advances the buffers in real time ([`advance_real_time`])
    if cfg!(target_arch = "wasm32") {
        return;
    }
    std::thread::Builder::new()
        .name("sa-game dsound mixer".into())
        .spawn(|| {
            let mut last = Instant::now();
            loop {
                std::thread::sleep(Duration::from_millis(MIX_MS));
                let now = Instant::now();
                let secs = now.duration_since(last).as_secs_f64();
                last = now;
                // the deterministic mode advances the buffers from the main loop ([`advance`])
                if !crate::det::on() {
                    advance(secs);
                }
            }
        })
        .expect("sa-game: cannot start the DirectSound mixer thread");
}

/// Reads one sample (as i32, 16-bit scale) of channel `ch` at frame `f` of `b`.
fn sample(b: &Buffer, f: u32, ch: u32) -> i32 {
    let ch = ch.min(b.channels.max(1) - 1);
    let off = f.wrapping_mul(b.block).wrapping_add(ch * (b.bits / 8));
    if off >= b.size {
        return 0;
    }
    // SAFETY: `mem` is a leaked block of `size` bytes owned by this buffer; the game writes it
    // concurrently through `Lock` pointers exactly as with DirectSound's hardware mixing (torn
    // samples are audible glitches, not memory errors: plain byte reads).
    unsafe {
        let p = (b.mem + off) as *const u8;
        if b.bits == 8 {
            (core::ptr::read_volatile(p) as i32 - 128) << 8
        } else {
            i16::from_le_bytes([core::ptr::read_volatile(p), core::ptr::read_volatile(p.add(1))]) as i32
        }
    }
}

/// Advances a playing buffer by `frames` output frames and adds its samples to `mix`.
fn mix_buffer(b: &mut Buffer, mix: &mut [i32], frames: usize, out_rate: u32) {
    if !b.playing || b.flags & DSBCAPS_PRIMARYBUFFER != 0 || b.block == 0 {
        return;
    }
    let len = (b.size / b.block) as f64;
    if len <= 0.0 {
        return;
    }
    let rate = if b.freq != 0 { b.freq } else { b.rate };
    let step = rate as f64 / out_rate as f64;
    let gain = if b.volume <= -10000 {
        0.0
    } else {
        10f64.powf(b.volume as f64 / 2000.0)
    };
    for i in 0..frames {
        if b.pos >= len {
            if b.looping {
                b.pos %= len;
            } else {
                b.playing = false;
                b.pos = 0.0;
                return;
            }
        }
        if gain > 0.0 {
            let f0 = b.pos as u32;
            let t = b.pos - f0 as f64;
            let f1 = if (f0 + 1) as f64 >= len {
                if b.looping { 0 } else { f0 }
            } else {
                f0 + 1
            };
            for ch in 0..2u32 {
                let a = sample(b, f0, ch) as f64;
                let c = sample(b, f1, ch) as f64;
                mix[i * 2 + ch as usize] += ((a + (c - a) * t) * gain) as i32;
            }
        }
        b.pos += step;
    }
}

// ------------------------------------------------------------------ entry points -----------------

/// `EAXDirectSoundCreate(guid, &ds, outer)` (EAX.DLL ordinal 6): the native `IDirectSound8`.
pub fn eax_direct_sound_create(_guid: u32, out: GPtr<u32>, _outer: u32) -> i32 {
    with("EAXDirectSoundCreate", |_| ());
    let mut o = DS8_OBJECT.lock().unwrap_or_else(|e| e.into_inner());
    if *o == 0 {
        *o = new_object(DS8, 0);
    }
    out.write(*o);
    start_mixer();
    DS_OK
}

fn add_ref(_this: u32) -> u32 {
    1
}
fn ds_release(_this: u32) -> u32 {
    0
}
fn ds_query_interface(_this: u32, _iid: GPtr<u32>, out: GPtr<u32>) -> i32 {
    out.write(0);
    E_NOINTERFACE
}
/// `GetCaps(DSCAPS*)`: `dwSize` kept, flags `DSCAPS_PRIMARYMONO|STEREO|8BIT|16BIT|CONTINUOUSRATE|
/// SECONDARY*`, 64 hardware 3D buffers (not measured: the game reads `dwMaxHw3DAllBuffers` +0x2C
/// to choose the channel count; 64 gives the original's 57 channels).
fn ds_get_caps(_this: u32, caps: GPtr<u32>) -> i32 {
    with("IDirectSound8::GetCaps", |_| ());
    for i in 1..24 {
        w32(caps, i, 0);
    }
    w32(caps, 1, 0x0F1F);
    w32(caps, 2, 100);
    w32(caps, 3, 200_000);
    w32(caps, 5, 64); // dwMaxHwMixingAllBuffers
    w32(caps, 6, 64);
    w32(caps, 7, 64);
    w32(caps, 8, 64);
    w32(caps, 9, 64);
    w32(caps, 10, 64);
    w32(caps, 11, 64); // dwMaxHw3DAllBuffers (+0x2C)
    w32(caps, 12, 64);
    w32(caps, 13, 64);
    w32(caps, 14, 64);
    w32(caps, 15, 64);
    w32(caps, 16, 64);
    DS_OK
}
fn ds_set_cooperative_level(_this: u32, _hwnd: u32, _level: u32) -> i32 {
    with("IDirectSound8::SetCooperativeLevel", |_| ());
    DS_OK
}
/// `GetSpeakerConfig`: `DSSPEAKER_STEREO` (4) with the wide geometry (not measured).
fn ds_get_speaker_config(_this: u32, out: GPtr<u32>) -> i32 {
    out.write(0x0005_0004);
    DS_OK
}

/// `CreateSoundBuffer(DSBUFFERDESC*, &buffer, outer)`: desc `{dwSize, dwFlags, dwBufferBytes,
/// dwReserved, lpwfxFormat, guid3DAlgorithm}`; `WAVEFORMATEX {wFormatTag u16, nChannels u16,
/// nSamplesPerSec, nAvgBytesPerSec, nBlockAlign u16, wBitsPerSample u16}`.
fn ds_create_sound_buffer(_this: u32, desc: GPtr<u32>, out: GPtr<u32>, _outer: u32) -> i32 {
    with("IDirectSound8::CreateSoundBuffer", |_| ());
    let flags = desc.wrapping_index(1).read();
    let bytes = desc.wrapping_index(2).read();
    let fmt = desc.wrapping_index(4).read();
    let (channels, rate, bits, block) = if fmt != 0 {
        let f = GPtr::<u8>::from_addr(fmt as usize);
        let u16_at = |o: isize| f.byte_add(o).cast::<u16>().read() as u32;
        (
            u16_at(2),
            f.byte_add(4).cast::<u32>().read(),
            u16_at(14),
            u16_at(12),
        )
    } else {
        (2, 44_100, 16, 4)
    };
    if flags & DSBCAPS_PRIMARYBUFFER == 0 && (bytes == 0 || fmt == 0) {
        out.write(0);
        return DSERR_INVALIDPARAM;
    }
    let mem = if bytes != 0 {
        Box::leak(vec![0u8; bytes as usize].into_boxed_slice()).as_mut_ptr() as u32
    } else {
        0
    };
    let h = with_state(|s| {
        let h = s.next;
        s.next += 1;
        s.buffers.insert(
            h,
            Buffer {
                mem,
                size: bytes,
                flags,
                channels,
                rate,
                bits,
                block,
                freq: 0,
                volume: 0,
                pan: 0,
                playing: false,
                looping: false,
                pos: 0.0,
                refs: 1,
                pos3d: [0.0; 3],
                min_dist: 1.0,
                max_dist: 1.0e9,
                mode3d: 0,
            },
        );
        h
    });
    out.write(new_object(BUFFER, h));
    DS_OK
}

fn buf_query_interface(this: u32, iid: GPtr<u32>, out: GPtr<u32>) -> i32 {
    let d1 = iid.read();
    let h = handle(this);
    let flags = with_state(|s| s.buffers.get(&h).map(|b| b.flags)).unwrap_or(0);
    let obj = match d1 {
        IID_LISTENER if flags & DSBCAPS_PRIMARYBUFFER != 0 => new_object(LISTENER, h),
        IID_BUFFER3D if flags & DSBCAPS_CTRL3D != 0 => new_object(BUFFER3D, h),
        IID_BUFFER | IID_BUFFER8 => this,
        _ => 0,
    };
    out.write(obj);
    if obj == 0 {
        return E_NOINTERFACE;
    }
    with_buf(this, |b| b.refs += 1);
    DS_OK
}
fn buf_add_ref(this: u32) -> u32 {
    with_buf(this, |b| {
        b.refs += 1;
        b.refs
    })
    .unwrap_or(1)
}
/// `Release`: the buffer (and its memory) goes away with its last reference.
fn buf_release(this: u32) -> u32 {
    let h = handle(this);
    with_state(|s| {
        let left = match s.buffers.get_mut(&h) {
            Some(b) => {
                b.refs = b.refs.saturating_sub(1);
                b.refs
            }
            None => return 0,
        };
        if left == 0
            && let Some(b) = s.buffers.remove(&h)
            && b.mem != 0
        {
            {
                // SAFETY: the block was leaked from a boxed slice of `size` bytes in
                // CreateSoundBuffer and is not referenced any more (the buffer is gone).
                drop(unsafe {
                    Box::from_raw(core::ptr::slice_from_raw_parts_mut(
                        b.mem as *mut u8,
                        b.size as usize,
                    ))
                });
            }
        }
        left
    })
}
/// `GetCaps(DSBCAPS*)`: `{dwSize, dwFlags, dwBufferBytes, dwUnlockTransferRate, dwPlayCpuOverhead}`.
fn buf_get_caps(this: u32, caps: GPtr<u32>) -> i32 {
    with_buf(this, |b| {
        w32(caps, 1, b.flags);
        w32(caps, 2, b.size);
        w32(caps, 3, 0);
        w32(caps, 4, 0);
    })
    .map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
fn buf_get_current_position(this: u32, play: GPtr<u32>, write: GPtr<u32>) -> i32 {
    with_buf(this, |b| {
        let p = if b.block != 0 {
            (b.pos as u32).wrapping_mul(b.block) % b.size.max(1)
        } else {
            0
        };
        if !play.is_null() {
            play.write(p);
        }
        if !write.is_null() {
            let rate = if b.freq != 0 { b.freq } else { b.rate };
            let lead = if b.playing {
                rate / 1000 * WRITE_LEAD_MS * b.block
            } else {
                0
            };
            write.write(p.wrapping_add(lead) % b.size.max(1));
        }
    })
    .map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
/// `GetFormat(WAVEFORMATEX*, size, &written)`: PCM in the buffer's format.
fn buf_get_format(this: u32, wfx: GPtr<u8>, size: u32, written: GPtr<u32>) -> i32 {
    with_buf(this, |b| {
        if !wfx.is_null() && size >= 16 {
            wfx.cast::<u16>().write(1);
            wfx.byte_add(2).cast::<u16>().write(b.channels as u16);
            wfx.byte_add(4).cast::<u32>().write(b.rate);
            wfx.byte_add(8).cast::<u32>().write(b.rate * b.block);
            wfx.byte_add(12).cast::<u16>().write(b.block as u16);
            wfx.byte_add(14).cast::<u16>().write(b.bits as u16);
        }
        if !written.is_null() {
            written.write(16);
        }
    })
    .map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
fn buf_get_volume(this: u32, out: GPtr<i32>) -> i32 {
    with_buf(this, |b| out.write(b.volume)).map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
fn buf_get_pan(this: u32, out: GPtr<i32>) -> i32 {
    with_buf(this, |b| out.write(b.pan)).map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
fn buf_get_frequency(this: u32, out: GPtr<u32>) -> i32 {
    with_buf(this, |b| out.write(if b.freq != 0 { b.freq } else { b.rate }))
        .map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
/// `GetStatus`: `DSBSTATUS_PLAYING` 1, `DSBSTATUS_LOOPING` 4 (+ `LOCHARDWARE` 8 for 3D buffers).
fn buf_get_status(this: u32, out: GPtr<u32>) -> i32 {
    with_buf(this, |b| {
        let mut s = 0;
        if b.playing {
            s |= 1;
            if b.looping {
                s |= 4;
            }
        }
        if b.flags & DSBCAPS_CTRL3D != 0 {
            s |= 8;
        }
        out.write(s);
    })
    .map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
/// `Lock(offset, bytes, &p1, &n1, &p2, &n2, flags)`: `DSBLOCK_FROMWRITECURSOR` 1,
/// `DSBLOCK_ENTIREBUFFER` 2; a region past the end wraps into the second pointer.
fn buf_lock(
    this: u32,
    offset: u32,
    bytes: u32,
    p1: GPtr<u32>,
    n1: GPtr<u32>,
    p2: GPtr<u32>,
    n2: GPtr<u32>,
    flags: u32,
) -> i32 {
    let r = with_buf(this, |b| {
        if b.size == 0 {
            return None;
        }
        let (mut off, mut n) = (offset, bytes);
        if flags & 1 != 0 {
            let rate = if b.freq != 0 { b.freq } else { b.rate };
            off = ((b.pos as u32).wrapping_mul(b.block) + rate / 1000 * WRITE_LEAD_MS * b.block) % b.size;
        }
        if flags & 2 != 0 {
            off = 0;
            n = b.size;
        }
        if off >= b.size || n > b.size {
            return None;
        }
        let first = n.min(b.size - off);
        Some((b.mem + off, first, if n > first { b.mem } else { 0 }, n - first))
    });
    match r.flatten() {
        Some((a1, l1, a2, l2)) => {
            p1.write(a1);
            n1.write(l1);
            if !p2.is_null() {
                p2.write(a2);
            }
            if !n2.is_null() {
                n2.write(l2);
            }
            DS_OK
        }
        None => DSERR_INVALIDPARAM,
    }
}
/// `Play(reserved1, priority, flags)`: `DSBPLAY_LOOPING` 1.
fn buf_play(this: u32, _r: u32, _prio: u32, flags: u32) -> i32 {
    with_buf(this, |b| {
        b.playing = true;
        b.looping = flags & 1 != 0;
    })
    .map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
fn buf_set_current_position(this: u32, pos: u32) -> i32 {
    with_buf(this, |b| {
        if let Some(p) = (pos % b.size.max(1)).checked_div(b.block) {
            b.pos = p as f64;
        }
    })
    .map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
/// `SetFormat(WAVEFORMATEX*)` (primary buffer): recorded.
fn buf_set_format(this: u32, wfx: GPtr<u8>) -> i32 {
    let f = (
        wfx.byte_add(2).cast::<u16>().read() as u32,
        wfx.byte_add(4).cast::<u32>().read(),
        wfx.byte_add(14).cast::<u16>().read() as u32,
    );
    let _ = this;
    with_state(|s| s.primary = f);
    DS_OK
}
fn buf_set_volume(this: u32, v: i32) -> i32 {
    if !(-10000..=0).contains(&v) {
        return DSERR_INVALIDPARAM;
    }
    with_buf(this, |b| b.volume = v).map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
fn buf_set_pan(this: u32, v: i32) -> i32 {
    with_buf(this, |b| b.pan = v).map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
/// `SetFrequency(hz)`: 0 = the format's rate; 100..200000 Hz.
fn buf_set_frequency(this: u32, f: u32) -> i32 {
    if f != 0 && !(100..=200_000).contains(&f) {
        return DSERR_INVALIDPARAM;
    }
    with_buf(this, |b| b.freq = f).map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
fn buf_stop(this: u32) -> i32 {
    with_buf(this, |b| b.playing = false).map_or(DSERR_INVALIDCALL, |_| DS_OK)
}
fn buf_unlock(_this: u32, _p1: u32, _n1: u32, _p2: u32, _n2: u32) -> i32 {
    DS_OK
}
fn buf_restore(_this: u32) -> i32 {
    DS_OK
}

// --- IDirectSound3DBuffer (parameters recorded, not rendered) ---

fn b3_query_interface(_this: u32, _iid: GPtr<u32>, out: GPtr<u32>) -> i32 {
    out.write(0);
    E_NOINTERFACE
}
fn b3_release(_this: u32) -> u32 {
    0
}
fn b3_set_position(this: u32, x: f32, y: f32, z: f32, _apply: u32) -> i32 {
    with_buf(this, |b| b.pos3d = [x, y, z]);
    DS_OK
}
fn b3_set_velocity(_this: u32, _x: f32, _y: f32, _z: f32, _apply: u32) -> i32 {
    DS_OK
}
fn b3_set_min_distance(this: u32, d: f32, _apply: u32) -> i32 {
    with_buf(this, |b| b.min_dist = d);
    DS_OK
}
fn b3_set_max_distance(this: u32, d: f32, _apply: u32) -> i32 {
    with_buf(this, |b| b.max_dist = d);
    DS_OK
}
fn b3_set_mode(this: u32, m: u32, _apply: u32) -> i32 {
    with_buf(this, |b| b.mode3d = m);
    DS_OK
}
/// `SetAllParameters(DS3DBUFFER*, apply)`: position (+4), min / max distance (+0x34 / +0x38), mode (+0x3C).
fn b3_set_all_parameters(this: u32, p: GPtr<u32>, _apply: u32) -> i32 {
    let f = |i: i32| f32::from_bits(p.wrapping_index(i).read());
    with_buf(this, |b| {
        b.pos3d = [f(1), f(2), f(3)];
        b.min_dist = f(13);
        b.max_dist = f(14);
        b.mode3d = p.wrapping_index(15).read();
    });
    DS_OK
}

// --- IDirectSound3DListener (recorded) ---

fn li_set_position(_this: u32, x: f32, y: f32, z: f32, _apply: u32) -> i32 {
    with_state(|s| s.listener = [x, y, z]);
    DS_OK
}
fn li_set_orientation(_this: u32, _a: f32, _b: f32, _c: f32, _d: f32, _e: f32, _f: f32, _apply: u32) -> i32 {
    DS_OK
}
fn li_set_factor(_this: u32, _v: f32, _apply: u32) -> i32 {
    DS_OK
}
fn li_set_velocity(_this: u32, _x: f32, _y: f32, _z: f32, _apply: u32) -> i32 {
    DS_OK
}
fn li_commit(_this: u32) -> i32 {
    DS_OK
}

macro_rules! entries {
    ($( $addr:expr, $name:literal => $f:path : fn($($a:ident : $t:ty),*) -> $r:ty ;)*) => {
        /// `(address, name, native stdcall entry)` of every implemented method.
        pub fn entries() -> Vec<(u32, &'static str, usize)> {
            let mut v: Vec<(u32, &'static str, usize)> = Vec::new();
            $({
                #[cfg(target_arch = "x86")]
                extern "stdcall-unwind" fn w($($a: $t),*) -> $r {
                    $f($($a),*)
                }
                #[cfg(not(target_arch = "x86"))]
                extern "C-unwind" fn w($($a: $t),*) -> $r {
                    $f($($a),*)
                }
                v.push(($addr, $name, {
                    #[cfg(target_arch = "wasm32")]
                    let e = sa_core::wabi_wrap!(w; $($a)*);
                    #[cfg(not(target_arch = "wasm32"))]
                    let e = w as *const () as usize;
                    e
                }));
            })*
            v
        }
    };
}

entries! {
    trap(DS8, 0), "IDirectSound8::QueryInterface" => ds_query_interface: fn(this: u32, iid: GPtr<u32>, out: GPtr<u32>) -> i32;
    trap(DS8, 1), "IDirectSound8::AddRef" => add_ref: fn(this: u32) -> u32;
    trap(DS8, 2), "IDirectSound8::Release" => ds_release: fn(this: u32) -> u32;
    trap(DS8, 3), "IDirectSound8::CreateSoundBuffer" => ds_create_sound_buffer: fn(this: u32, desc: GPtr<u32>, out: GPtr<u32>, outer: u32) -> i32;
    trap(DS8, 4), "IDirectSound8::GetCaps" => ds_get_caps: fn(this: u32, caps: GPtr<u32>) -> i32;
    trap(DS8, 6), "IDirectSound8::SetCooperativeLevel" => ds_set_cooperative_level: fn(this: u32, hwnd: u32, level: u32) -> i32;
    trap(DS8, 8), "IDirectSound8::GetSpeakerConfig" => ds_get_speaker_config: fn(this: u32, out: GPtr<u32>) -> i32;
    trap(BUFFER, 0), "IDirectSoundBuffer8::QueryInterface" => buf_query_interface: fn(this: u32, iid: GPtr<u32>, out: GPtr<u32>) -> i32;
    trap(BUFFER, 1), "IDirectSoundBuffer8::AddRef" => buf_add_ref: fn(this: u32) -> u32;
    trap(BUFFER, 2), "IDirectSoundBuffer8::Release" => buf_release: fn(this: u32) -> u32;
    trap(BUFFER, 3), "IDirectSoundBuffer8::GetCaps" => buf_get_caps: fn(this: u32, caps: GPtr<u32>) -> i32;
    trap(BUFFER, 4), "IDirectSoundBuffer8::GetCurrentPosition" => buf_get_current_position: fn(this: u32, play: GPtr<u32>, write: GPtr<u32>) -> i32;
    trap(BUFFER, 5), "IDirectSoundBuffer8::GetFormat" => buf_get_format: fn(this: u32, wfx: GPtr<u8>, size: u32, written: GPtr<u32>) -> i32;
    trap(BUFFER, 6), "IDirectSoundBuffer8::GetVolume" => buf_get_volume: fn(this: u32, out: GPtr<i32>) -> i32;
    trap(BUFFER, 7), "IDirectSoundBuffer8::GetPan" => buf_get_pan: fn(this: u32, out: GPtr<i32>) -> i32;
    trap(BUFFER, 8), "IDirectSoundBuffer8::GetFrequency" => buf_get_frequency: fn(this: u32, out: GPtr<u32>) -> i32;
    trap(BUFFER, 9), "IDirectSoundBuffer8::GetStatus" => buf_get_status: fn(this: u32, out: GPtr<u32>) -> i32;
    trap(BUFFER, 11), "IDirectSoundBuffer8::Lock" => buf_lock: fn(this: u32, offset: u32, bytes: u32, p1: GPtr<u32>, n1: GPtr<u32>, p2: GPtr<u32>, n2: GPtr<u32>, flags: u32) -> i32;
    trap(BUFFER, 12), "IDirectSoundBuffer8::Play" => buf_play: fn(this: u32, r: u32, prio: u32, flags: u32) -> i32;
    trap(BUFFER, 13), "IDirectSoundBuffer8::SetCurrentPosition" => buf_set_current_position: fn(this: u32, pos: u32) -> i32;
    trap(BUFFER, 14), "IDirectSoundBuffer8::SetFormat" => buf_set_format: fn(this: u32, wfx: GPtr<u8>) -> i32;
    trap(BUFFER, 15), "IDirectSoundBuffer8::SetVolume" => buf_set_volume: fn(this: u32, v: i32) -> i32;
    trap(BUFFER, 16), "IDirectSoundBuffer8::SetPan" => buf_set_pan: fn(this: u32, v: i32) -> i32;
    trap(BUFFER, 17), "IDirectSoundBuffer8::SetFrequency" => buf_set_frequency: fn(this: u32, f: u32) -> i32;
    trap(BUFFER, 18), "IDirectSoundBuffer8::Stop" => buf_stop: fn(this: u32) -> i32;
    trap(BUFFER, 19), "IDirectSoundBuffer8::Unlock" => buf_unlock: fn(this: u32, p1: u32, n1: u32, p2: u32, n2: u32) -> i32;
    trap(BUFFER, 20), "IDirectSoundBuffer8::Restore" => buf_restore: fn(this: u32) -> i32;
    trap(BUFFER3D, 0), "IDirectSound3DBuffer::QueryInterface" => b3_query_interface: fn(this: u32, iid: GPtr<u32>, out: GPtr<u32>) -> i32;
    trap(BUFFER3D, 1), "IDirectSound3DBuffer::AddRef" => add_ref: fn(this: u32) -> u32;
    trap(BUFFER3D, 2), "IDirectSound3DBuffer::Release" => b3_release: fn(this: u32) -> u32;
    trap(BUFFER3D, 12), "IDirectSound3DBuffer::SetAllParameters" => b3_set_all_parameters: fn(this: u32, p: GPtr<u32>, apply: u32) -> i32;
    trap(BUFFER3D, 16), "IDirectSound3DBuffer::SetMaxDistance" => b3_set_max_distance: fn(this: u32, d: f32, apply: u32) -> i32;
    trap(BUFFER3D, 17), "IDirectSound3DBuffer::SetMinDistance" => b3_set_min_distance: fn(this: u32, d: f32, apply: u32) -> i32;
    trap(BUFFER3D, 18), "IDirectSound3DBuffer::SetMode" => b3_set_mode: fn(this: u32, m: u32, apply: u32) -> i32;
    trap(BUFFER3D, 19), "IDirectSound3DBuffer::SetPosition" => b3_set_position: fn(this: u32, x: f32, y: f32, z: f32, apply: u32) -> i32;
    trap(BUFFER3D, 20), "IDirectSound3DBuffer::SetVelocity" => b3_set_velocity: fn(this: u32, x: f32, y: f32, z: f32, apply: u32) -> i32;
    trap(LISTENER, 1), "IDirectSound3DListener::AddRef" => add_ref: fn(this: u32) -> u32;
    trap(LISTENER, 2), "IDirectSound3DListener::Release" => b3_release: fn(this: u32) -> u32;
    trap(LISTENER, 11), "IDirectSound3DListener::SetDistanceFactor" => li_set_factor: fn(this: u32, v: f32, apply: u32) -> i32;
    trap(LISTENER, 12), "IDirectSound3DListener::SetDopplerFactor" => li_set_factor: fn(this: u32, v: f32, apply: u32) -> i32;
    trap(LISTENER, 13), "IDirectSound3DListener::SetOrientation" => li_set_orientation: fn(this: u32, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, apply: u32) -> i32;
    trap(LISTENER, 14), "IDirectSound3DListener::SetPosition" => li_set_position: fn(this: u32, x: f32, y: f32, z: f32, apply: u32) -> i32;
    trap(LISTENER, 15), "IDirectSound3DListener::SetRolloffFactor" => li_set_factor: fn(this: u32, v: f32, apply: u32) -> i32;
    trap(LISTENER, 16), "IDirectSound3DListener::SetVelocity" => li_set_velocity: fn(this: u32, x: f32, y: f32, z: f32, apply: u32) -> i32;
    trap(LISTENER, 17), "IDirectSound3DListener::CommitDeferredSettings" => li_commit: fn(this: u32) -> i32;
}
