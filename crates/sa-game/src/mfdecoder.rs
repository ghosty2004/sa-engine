//! The user-track decoder `CAEMFDecoder` (vtable 0x85F370; type 4 in `CAEUserRadioTrackManager::LoadUserTrack`):
//! its methods decode through the media library gta_sa.exe loads at run time (`InitLibrary` 0x4E7C70 starts it),
//! served here by the pure-Rust `symphonia` MP3 decoder (MPL-2.0). Formats other than MP3 are not decoded:
//! `Initialise` fails for them, and the game skips the track as it does any track that cannot be opened.
//!
//! The decoder's contract (gta-reversed `AEMFDecoder.cpp` describes the original's): `Initialise` opens the file
//! named at +8 and fails for tracks shorter than 7 s; `FillBuffer` gives 16-bit stereo PCM at the file's sample
//! rate; `GetStreamPlayTimeMs` is the position of the first sample of the last `FillBuffer`; `SetCursor` seeks in
//! milliseconds; the lengths and times are -1 before a successful `Initialise`. The constructor and
//! `GetStreamID` are game code (sa-audio `user_track.rs`). The decoder state is kept here per object address;
//! the object's own fields keep the initialised flag (+0xC) and the sample rate (+0xBC).

use std::collections::HashMap;
use std::sync::Mutex;

use sa_core::mem::GPtr;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use sa_audio::audio_data_stream::streaming_decoder_destructor_call;

/// `CAEMFDecoder` fields.
const FILENAME: u32 = 0x08;
const INITIALISED: u32 = 0x0C;
const SAMPLE_RATE: u32 = 0xBC;
/// `CAEMFDecoder::quickTimeInitialized`, set by `InitLibrary`.
const LIBRARY_STARTED: usize = 0xB6_1D70;
/// The shortest track the game plays.
const MIN_LENGTH_MS: i64 = 7000;

/// A user track decoded to interleaved 16-bit stereo.
pub struct Track {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    rate: u32,
    frames: u64,
    pending: Vec<i16>,
    next: usize,
    position: u64,
    play_ms: i64,
}

impl Track {
    /// The first audio track of an MP3 file; `None` when it does not parse or decode.
    pub fn open(bytes: Vec<u8>) -> Option<Track> {
        let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
        let mut hint = Hint::new();
        hint.with_extension("mp3");
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .ok()?;
        let mut format = probed.format;
        let track = format.default_track()?;
        let (track_id, params) = (track.id, track.codec_params.clone());
        let rate = params.sample_rate?;
        let decoder = symphonia::default::get_codecs().make(&params, &DecoderOptions::default()).ok()?;
        let mut frames = 0;
        while let Ok(p) = format.next_packet() {
            if p.track_id() == track_id {
                frames += p.dur;
            }
        }
        format.seek(SeekMode::Accurate, SeekTo::TimeStamp { ts: 0, track_id }).ok()?;
        Some(Track { format, decoder, track_id, rate, frames, pending: Vec::new(), next: 0, position: 0, play_ms: 0 })
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    pub fn length_ms(&self) -> i64 {
        (self.frames * 1000 / u64::from(self.rate)) as i64
    }

    pub fn play_time_ms(&self) -> i64 {
        self.play_ms
    }

    /// Fills `out` with whole stereo frames; returns how many samples (two per frame) were written, fewer at the end.
    pub fn fill(&mut self, out: &mut [i16]) -> usize {
        self.play_ms = (self.position * 1000 / u64::from(self.rate)) as i64;
        let want = out.len() & !1;
        let mut n = 0;
        while n < want {
            if self.next == self.pending.len() && !self.decode_packet() {
                break;
            }
            let k = (want - n).min(self.pending.len() - self.next);
            out[n..n + k].copy_from_slice(&self.pending[self.next..self.next + k]);
            self.next += k;
            n += k;
        }
        self.position += n as u64 / 2;
        n
    }

    /// Moves to `ms` milliseconds from the start (the end when past it).
    pub fn seek_ms(&mut self, ms: u32) {
        let ts = (u64::from(ms) * u64::from(self.rate) / 1000).min(self.frames);
        let to = SeekTo::TimeStamp { ts, track_id: self.track_id };
        let landed = self.format.seek(SeekMode::Accurate, to).map(|s| s.actual_ts).unwrap_or(self.frames);
        self.decoder.reset();
        self.pending.clear();
        self.next = 0;
        self.position = landed;
        let mut skip = ts.saturating_sub(landed) as usize * 2;
        while skip > 0 && self.decode_packet() {
            let k = skip.min(self.pending.len());
            self.next = k;
            skip -= k;
        }
        self.position = ts;
    }

    /// Decodes the next packet of the track into `pending` as stereo; false at the end.
    fn decode_packet(&mut self) -> bool {
        loop {
            let Ok(packet) = self.format.next_packet() else { return false };
            if packet.track_id() != self.track_id {
                continue;
            }
            let Ok(buf) = self.decoder.decode(&packet) else { continue };
            let spec = *buf.spec();
            let channels = spec.channels.count();
            let mut samples = SampleBuffer::<i16>::new(buf.capacity() as u64, spec);
            samples.copy_interleaved_ref(buf);
            self.pending.clear();
            self.next = 0;
            for frame in samples.samples().chunks(channels) {
                let left = frame[0];
                self.pending.extend_from_slice(&[left, *frame.get(1).unwrap_or(&left)]);
            }
            if !self.pending.is_empty() {
                return true;
            }
        }
    }
}

static TRACKS: Mutex<Option<HashMap<u32, Track>>> = Mutex::new(None);

fn with_track<R>(this: u32, f: impl FnOnce(Option<&mut Track>) -> R) -> R {
    let mut s = TRACKS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    f(s.get_or_insert_with(HashMap::new).get_mut(&this))
}

fn field(this: u32, off: u32) -> GPtr<u32> {
    GPtr::from_addr(this.wrapping_add(off) as usize)
}

/// `CAEMFDecoder::Initialise()` (0x4E7CB0): the file opened and decoded from its start; false without the started
/// library, for a file that is not MP3 or a track shorter than 7 s.
fn initialise(this: u32) -> bool {
    if GPtr::<u8>::from_addr(LIBRARY_STARTED).read() == 0 {
        return false;
    }
    let path = crate::bind::cstr(GPtr::from_addr(field(this, FILENAME).read() as usize));
    let Some(track) = crate::bind::read_whole_file(&path).and_then(Track::open) else {
        eprintln!("sa-game: user track {path:?}: not an MP3 file this build decodes; skipped");
        return false;
    };
    if track.length_ms() < MIN_LENGTH_MS {
        return false;
    }
    field(this, SAMPLE_RATE).write(track.sample_rate());
    GPtr::<u8>::from_addr((this + INITIALISED) as usize).write(1);
    let mut s = TRACKS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    s.get_or_insert_with(HashMap::new).insert(this, track);
    true
}

/// `CAEMFDecoder::FillBuffer(void* dest, size_t size)` (0x4E7860): up to `size / 4` stereo frames; returns bytes.
fn fill_buffer(this: u32, dest: u32, size: u32) -> u32 {
    with_track(this, |t| {
        let Some(t) = t else { return 0 };
        let mut pcm = vec![0i16; size as usize / 2];
        let n = t.fill(&mut pcm);
        for (i, s) in pcm[..n].iter().enumerate() {
            GPtr::<i16>::from_addr(dest as usize + 2 * i).write(*s);
        }
        2 * n as u32
    })
}

/// `CAEMFDecoder::GetStreamLengthMs()` (0x4E7920).
fn get_stream_length_ms(this: u32) -> i32 {
    with_track(this, |t| t.map_or(-1, |t| t.length_ms() as i32))
}

/// `CAEMFDecoder::GetStreamPlayTimeMs()` (0x4E7940).
fn get_stream_play_time_ms(this: u32) -> i32 {
    with_track(this, |t| t.map_or(-1, |t| t.play_time_ms() as i32))
}

/// `CAEMFDecoder::SetCursor(unsigned long ms)` (0x4E78E0).
fn set_cursor(this: u32, ms: u32) {
    with_track(this, |t| {
        if let Some(t) = t {
            t.seek_ms(ms);
        }
    });
}

/// `CAEMFDecoder::GetSampleRate()` (0x4E78C0).
fn get_sample_rate(this: u32) -> i32 {
    with_track(this, |t| t.map_or(-1, |t| t.sample_rate() as i32))
}

/// `CAEMFDecoder::~CAEMFDecoder()` (0x4E77C0): the decoder state and the file name (`new[]` by `GetTrackPath`)
/// freed, then the base destructor.
fn destructor(this: u32) {
    let mut s = TRACKS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    s.get_or_insert_with(HashMap::new).remove(&this);
    drop(s);
    let name = field(this, FILENAME);
    if name.read() != 0 {
        crate::crt::free(GPtr::from_addr(name.read() as usize));
        name.write(0);
    }
    streaming_decoder_destructor_call(GPtr::from_addr(this as usize));
}

/// The scalar deleting destructor (vtable slot 6, 0x4E7C50): the destructor, then `operator delete` when bit 0 of
/// `flags` is set; returns `this`.
fn scalar_deleting_destructor(this: u32, flags: u32) -> u32 {
    destructor(this);
    if flags & 1 != 0 {
        crate::crt::free(GPtr::from_addr(this as usize));
    }
    this
}

/// The media library's start-up as `InitLibrary` calls it: the loader (0x8195C0) and `MFStartup` (0x819820), both
/// successful (0), so MP3 user tracks reach this decoder.
fn library_startup(_flags: u32) -> u32 {
    0
}

fn library_mf_startup() -> u32 {
    0
}

/// `(address, name, native entry)` of the decoder's methods and the library start-up.
pub fn entries() -> Vec<(u32, &'static str, usize)> {
    use crate::platform::registry_entry::{cdecl_entry, thiscall_entry};
    vec![
        cdecl_entry!(0x0081_95C0, "media library loader start-up (MP3 user tracks through symphonia)", library_startup; fn(flags: u32) -> u32),
        cdecl_entry!(0x0081_9820, "media library MFStartup (MP3 user tracks through symphonia)", library_mf_startup; fn() -> u32),
        thiscall_entry!(0x004E_7CB0, "CAEMFDecoder::Initialise (symphonia)", initialise; fn(this: u32) -> bool),
        thiscall_entry!(0x004E_7860, "CAEMFDecoder::FillBuffer (symphonia)", fill_buffer; fn(this: u32, dest: u32, size: u32) -> u32),
        thiscall_entry!(0x004E_7920, "CAEMFDecoder::GetStreamLengthMs (symphonia)", get_stream_length_ms; fn(this: u32) -> i32),
        thiscall_entry!(0x004E_7940, "CAEMFDecoder::GetStreamPlayTimeMs (symphonia)", get_stream_play_time_ms; fn(this: u32) -> i32),
        thiscall_entry!(0x004E_78E0, "CAEMFDecoder::SetCursor (symphonia)", set_cursor; fn(this: u32, ms: u32) -> ()),
        thiscall_entry!(0x004E_78C0, "CAEMFDecoder::GetSampleRate (symphonia)", get_sample_rate; fn(this: u32) -> i32),
        thiscall_entry!(0x004E_77C0, "CAEMFDecoder::~CAEMFDecoder (symphonia)", destructor; fn(this: u32) -> ()),
        thiscall_entry!(0x004E_7C50, "CAEMFDecoder scalar deleting destructor (symphonia)", scalar_deleting_destructor; fn(this: u32, flags: u32) -> u32),
    ]
}

#[cfg(test)]
mod tests {
    use super::Track;

    /// `n` MPEG-1 Layer III frames (128 kbit/s, 44.1 kHz, stereo, 417 bytes) whose side information is all zero: valid
    /// frames that decode to silence.
    fn silent_mp3(n: usize) -> Vec<u8> {
        let mut frame = vec![0u8; 417];
        frame[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        frame.repeat(n)
    }

    #[test]
    fn generated_mp3_decodes_to_stereo_pcm_and_seeks() {
        let mut t = Track::open(silent_mp3(400)).expect("open");
        assert_eq!(t.sample_rate(), 44_100);
        let len = t.length_ms();
        assert!((10_400..=10_460).contains(&len), "length {len} ms");
        let mut pcm = vec![1i16; 4096];
        assert_eq!(t.fill(&mut pcm), 4096);
        assert!(pcm.iter().all(|&s| s == 0));
        assert_eq!(t.play_time_ms(), 0);
        t.seek_ms(5000);
        assert_eq!(t.fill(&mut pcm), 4096);
        assert_eq!(t.play_time_ms(), 5000);
        t.seek_ms(len as u32);
        assert!(t.fill(&mut pcm) < 4096);
    }

    #[test]
    fn data_that_is_not_mp3_is_rejected() {
        assert!(Track::open(b"RIFF\0\0\0\0WAVEfmt nothing here".to_vec()).is_none());
    }
}
