//! `vorbisfile.dll` replacement: the seven `ov_*` entry points gta_sa.exe imports (`ov_open_callbacks`, `ov_clear`,
//! `ov_read`, `ov_info`, `ov_time_total`, `ov_time_tell`, `ov_time_seek`), decoded with the pure-Rust `lewton`
//! decoder (MIT OR Apache-2.0; its `ogg` dependency is BSD-3-Clause).
//!
//! The game reaches them only through its import thunks 0x818062..0x818086 (`jmp [IAT]`, cdecl), from the ported
//! `CAEVorbisDecoder` (sa-audio `audio_data_stream.rs`). Semantics follow libvorbisfile 1.x as the game uses it:
//!
//! * `ov_open_callbacks(ds, vf, NULL, 0, callbacks)`: the callbacks are game functions (read 0x502580, seek
//!   0x5025B0, close 0x5025D0, tell 0x5025F0) over `CAEDataStream`, which already undoes the stream archives' XOR
//!   (`CAEStreamTransformer`). As libvorbisfile does for a seekable source, the end of the data is found with
//!   `seek(ds, 0, SEEK_END)` + `tell(ds)` (the track's length: `CAEDataStream::Seek` measures from the track start),
//!   then every read and seek is a `SEEK_SET` inside `[0, end)`. 0 on success, `OV_ENOTVORBIS` (-132) when the
//!   headers do not parse (the data source is not closed, as libvorbisfile leaves it to the caller).
//! * `ov_read(vf, buf, len, 0, 2, 1, &bitstream)`: 16-bit little-endian signed interleaved PCM, whole sample frames,
//!   at most `len` bytes; the float samples are converted as libvorbis does (`rint(x * 32768)`, clipped). 0 at the
//!   end of the first logical stream (the tracks are single-link files), `*bitstream = 0`.
//! * `ov_info(vf, -1)`: a `vorbis_info` (version, channels, rate, bitrate upper / nominal / lower, window, codec
//!   setup) kept inside the caller's `OggVorbis_File` block (0x2D0 bytes, offset 0x40), whose layout is private to
//!   the DLL.
//! * `ov_time_total(vf, -1)`: the last page's granule position / rate (seconds, double in ST0); `ov_time_tell`: the
//!   PCM position of the next sample `ov_read` returns / rate; `ov_time_seek(vf, s)`: sample-exact (page seek, then
//!   the leading samples dropped), `OV_EINVAL` (-131) outside `[0, total]`.
//! * `ov_clear(vf)`: frees the state and calls the close callback (libvorbisfile does), 0.
//!
//! The decoder state lives in a host map keyed by the `OggVorbis_File` address. Different from the DLL: lewton's
//! inverse MDCT is not libvorbis's, so the decoded floats can differ in the last bits (inaudible; the PCM is not
//! compared against the original).

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Mutex;

use lewton::inside_ogg::OggStreamReader;
use lewton::samples::InterleavedSamples;
use sa_core::mem::GPtr;

sa_core::orig! {
    /// A game `read_func(ptr, size, n, ds)` (cdecl), by address.
    fn cb_read(f: u32, ptr: u32, size: u32, n: u32, ds: u32) -> u32 = cdecl dyn => unported;
    /// A game `seek_func(ds, int64 offset, whence)` (cdecl), by address.
    fn cb_seek(f: u32, ds: u32, lo: u32, hi: u32, whence: u32) -> i32 = cdecl dyn => unported;
    /// A game `close_func(ds)` (cdecl), by address.
    fn cb_close(f: u32, ds: u32) -> i32 = cdecl dyn => unported;
    /// A game `tell_func(ds)` (cdecl), by address.
    fn cb_tell(f: u32, ds: u32) -> i32 = cdecl dyn => unported;
}

const OV_EINVAL: i32 = -131;
const OV_ENOTVORBIS: i32 = -132;
/// Offset of the `vorbis_info` inside the caller's `OggVorbis_File` block.
const INFO_OFF: usize = 0x40;

/// The game's data source seen through its callbacks, bounded to the track.
struct Source {
    ds: u32,
    read: u32,
    seek: u32,
    pos: u64,
    end: u64,
}

impl Read for Source {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = (buf.len() as u64).min(self.end.saturating_sub(self.pos)) as u32;
        if n == 0 {
            return Ok(0);
        }
        let got = cb_read(self.read, buf.as_mut_ptr() as usize as u32, 1, n, self.ds).min(n);
        self.pos += u64::from(got);
        Ok(got as usize)
    }
}

impl Seek for Source {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let target = match to {
            SeekFrom::Start(p) => p as i64,
            SeekFrom::Current(d) => self.pos as i64 + d,
            SeekFrom::End(d) => self.end as i64 + d,
        };
        if target < 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek before the start"));
        }
        let r = cb_seek(self.seek, self.ds, target as u32, 0, 0);
        self.pos = if r < 0 { target as u64 } else { r as u32 as u64 };
        Ok(self.pos)
    }
}

struct File {
    rdr: Option<OggStreamReader<Source>>,
    ds: u32,
    close: u32,
    channels: usize,
    rate: u32,
    /// Granule position of the last page (samples per channel).
    total: u64,
    /// PCM position (per channel) of `pending[0]`.
    pos: u64,
    /// Decoded, not yet returned interleaved samples.
    pending: Vec<i16>,
    /// Samples (per channel) still to drop after a seek.
    skip: u64,
    /// After a seek: the position is known only at the next page end.
    pos_unknown: bool,
    eof: bool,
}

static FILES: Mutex<Option<HashMap<u32, Box<File>>>> = Mutex::new(None);

fn take(vf: u32) -> Option<Box<File>> {
    FILES.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).remove(&vf)
}
fn put(vf: u32, f: Box<File>) {
    FILES.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(vf, f);
}

/// `SA_GAME_VORBIS_TRACE=1`: opens, seeks and the first reads are logged.
fn trace_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| crate::env::var_os("SA_GAME_VORBIS_TRACE").is_some())
}

/// libvorbis's float → 16-bit conversion (`vorbis_ftoi`: round to nearest, then clip).
fn to_i16(x: f32) -> i16 {
    (x * 32768.0).round_ties_even().clamp(-32768.0, 32767.0) as i16
}

/// The granule position of the last Ogg page of `serial` in `[0, end)`, scanning back in 64 KiB windows.
fn last_granule(src: &mut Source, serial: u32) -> u64 {
    const WIN: u64 = 0x10000;
    let mut hi = src.end;
    while hi > 0 {
        let lo = hi.saturating_sub(WIN);
        let mut buf = vec![0u8; (hi - lo + 27).min(src.end - lo) as usize];
        if src.seek(SeekFrom::Start(lo)).is_err() {
            return 0;
        }
        let mut got = 0;
        while got < buf.len() {
            match src.read(&mut buf[got..]) {
                Ok(0) | Err(_) => break,
                Ok(n) => got += n,
            }
        }
        buf.truncate(got);
        let mut best = None;
        let mut i = 0;
        while i + 27 <= buf.len() {
            if &buf[i..i + 4] == b"OggS" && (lo + i as u64) < hi {
                let gp = u64::from_le_bytes(buf[i + 6..i + 14].try_into().unwrap_or_default());
                let ser = u32::from_le_bytes(buf[i + 14..i + 18].try_into().unwrap_or_default());
                if ser == serial && gp != u64::MAX {
                    best = Some(gp);
                }
            }
            i += 1;
        }
        if let Some(g) = best {
            return g;
        }
        hi = lo;
    }
    0
}

/// Decodes the next packet into `pending`; false at the end of the stream.
fn decode(f: &mut File) -> bool {
    if f.eof {
        return false;
    }
    let Some(rdr) = f.rdr.as_mut() else {
        f.eof = true;
        return false;
    };
    let serial = rdr.stream_serial();
    let pck: Option<InterleavedSamples<f32>> = rdr.read_dec_packet_generic().unwrap_or_default();
    let (serial_now, absgp) = (rdr.stream_serial(), rdr.get_last_absgp());
    let Some(p) = pck else {
        f.eof = true;
        return false;
    };
    if serial_now != serial {
        // a second link (the next track's data): the end of this one
        f.eof = true;
        return false;
    }
    f.pending.extend(p.samples.iter().map(|&x| to_i16(x)));
    if f.pos_unknown
        && let Some(g) = absgp
    {
        // `g` ends the samples decoded so far
        let have = (f.pending.len() / f.channels) as u64;
        f.pos = g.saturating_sub(have);
        f.pos_unknown = false;
    }
    true
}

/// Drops the samples a seek asked to skip, once the position is known.
fn apply_skip(f: &mut File) {
    if f.pos_unknown || f.skip == 0 {
        return;
    }
    let frames = (f.pending.len() / f.channels) as u64;
    let n = f.skip.min(frames);
    f.pending.drain(..(n as usize * f.channels));
    f.pos += n;
    f.skip -= n;
}

/// `ov_open_callbacks(void* ds, OggVorbis_File* vf, char* initial, long ibytes, ov_callbacks cb)`.
#[allow(clippy::too_many_arguments)] // the C signature: four callbacks passed by value
pub fn ov_open_callbacks(ds: u32, vf: u32, _initial: u32, _ibytes: u32, read: u32, seek: u32, close: u32, tell: u32) -> i32 {
    if let Some(old) = take(vf) {
        drop(old);
    }
    cb_seek(seek, ds, 0, 0, 2);
    let end = cb_tell(tell, ds).max(0) as u32 as u64;
    cb_seek(seek, ds, 0, 0, 0);
    let mut src = Source { ds, read, seek, pos: 0, end };
    // the first page's serial, for the length scan
    let mut head = [0u8; 18];
    let _ = src.read(&mut head);
    let serial = u32::from_le_bytes([head[14], head[15], head[16], head[17]]);
    let total = if &head[..4] == b"OggS" { last_granule(&mut src, serial) } else { 0 };
    if src.seek(SeekFrom::Start(0)).is_err() {
        return OV_ENOTVORBIS;
    }
    let rdr = match OggStreamReader::new(src) {
        Ok(r) => r,
        Err(e) => {
            if trace_on() {
                eprintln!("vorbisfile: open {vf:#x} ds {ds:#x} end {end} head {head:02x?}: not Vorbis ({e:?})");
            }
            return OV_ENOTVORBIS;
        }
    };
    if trace_on() {
        let h = &rdr.ident_hdr;
        eprintln!("vorbisfile: open {vf:#x} ds {ds:#x} end {end}: {} ch {} Hz, {total} samples", h.audio_channels, h.audio_sample_rate);
    }
    let h = &rdr.ident_hdr;
    let (channels, rate) = (usize::from(h.audio_channels.max(1)), h.audio_sample_rate);
    let info = [
        0u32,
        u32::from(h.audio_channels),
        rate,
        h.bitrate_maximum as u32,
        h.bitrate_nominal as u32,
        h.bitrate_minimum as u32,
        0,
        0,
    ];
    for (i, v) in info.iter().enumerate() {
        GPtr::<u32>::from_addr(vf as usize + INFO_OFF + 4 * i).write(*v);
    }
    put(
        vf,
        Box::new(File {
            rdr: Some(rdr),
            ds,
            close,
            channels,
            rate,
            total,
            pos: 0,
            pending: Vec::new(),
            skip: 0,
            pos_unknown: false,
            eof: false,
        }),
    );
    0
}

/// `ov_clear(OggVorbis_File* vf)`: frees the state, closes the data source; 0.
pub fn ov_clear(vf: u32) -> i32 {
    if let Some(f) = take(vf) {
        let (ds, close) = (f.ds, f.close);
        drop(f);
        if ds != 0 && close != 0 {
            cb_close(close, ds);
        }
    }
    0
}

/// `ov_read(vf, buffer, length, bigendianp, word, sgned, int* bitstream)`: bytes written, 0 at the end.
pub fn ov_read(vf: u32, buf: u32, len: u32, big: u32, word: u32, sgn: u32, bitstream: u32) -> i32 {
    let Some(mut f) = take(vf) else {
        return OV_EINVAL;
    };
    let frame = f.channels * word.max(1) as usize;
    let want = len as usize / frame;
    while (f.pending.is_empty() || f.pos_unknown || f.skip > 0) && decode(&mut f) {
        apply_skip(&mut f);
    }
    apply_skip(&mut f);
    let frames = want.min(f.pending.len() / f.channels);
    let out = GPtr::<u8>::from_addr(buf as usize);
    let mut o = 0isize;
    for &s in &f.pending[..frames * f.channels] {
        let v = if sgn != 0 { s as u16 } else { (s as u16) ^ 0x8000 };
        if word == 1 {
            out.byte_add(o).write((v >> 8) as u8);
            o += 1;
        } else {
            let b = if big != 0 { v.to_be_bytes() } else { v.to_le_bytes() };
            out.byte_add(o).write(b[0]);
            out.byte_add(o + 1).write(b[1]);
            o += 2;
        }
    }
    f.pending.drain(..frames * f.channels);
    f.pos += frames as u64;
    static READS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if trace_on() && { let n = READS.fetch_add(1, std::sync::atomic::Ordering::Relaxed); n < 4 || n.is_multiple_of(200) } {
        eprintln!("vorbisfile: read {vf:#x} {len} -> {o} bytes, pos {}", f.pos);
    }
    if bitstream != 0 {
        GPtr::<i32>::from_addr(bitstream as usize).write(0);
    }
    put(vf, f);
    o as i32
}

/// `ov_info(vf, link)`: the `vorbis_info` inside the caller's block; NULL before a successful open.
pub fn ov_info(vf: u32, _link: i32) -> u32 {
    let ok = FILES.lock().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|m| m.contains_key(&vf));
    if ok { vf + INFO_OFF as u32 } else { 0 }
}

/// `ov_time_total(vf, link)`: seconds (`OV_EINVAL` as a double when not open).
pub fn ov_time_total(vf: u32, _link: i32) -> f64 {
    let m = FILES.lock().unwrap_or_else(|e| e.into_inner());
    match m.as_ref().and_then(|m| m.get(&vf)) {
        Some(f) if f.rate != 0 => f.total as f64 / f64::from(f.rate),
        _ => f64::from(OV_EINVAL),
    }
}

/// `ov_time_tell(vf)`: seconds of the next sample `ov_read` returns.
pub fn ov_time_tell(vf: u32) -> f64 {
    let m = FILES.lock().unwrap_or_else(|e| e.into_inner());
    match m.as_ref().and_then(|m| m.get(&vf)) {
        Some(f) if f.rate != 0 => (f.pos + f.skip) as f64 / f64::from(f.rate),
        _ => f64::from(OV_EINVAL),
    }
}

/// `ov_time_seek(vf, seconds)`: 0, or `OV_EINVAL` outside the stream.
pub fn ov_time_seek(vf: u32, s: f64) -> i32 {
    let Some(mut f) = take(vf) else {
        return OV_EINVAL;
    };
    let target = s * f64::from(f.rate);
    if !(0.0..=f.total as f64).contains(&target) {
        put(vf, f);
        return OV_EINVAL;
    }
    let target = target as u64;
    if trace_on() {
        eprintln!("vorbisfile: seek {vf:#x} to {s:.3} s ({target} of {})", f.total);
    }
    f.pending.clear();
    f.eof = false;
    f.skip = 0;
    // a fresh reader from the start (headers re-read), then a page seek for a later target
    let rdr = f.rdr.take().map(|r| r.into_inner().into_inner());
    f.rdr = rdr.and_then(|mut src| {
        src.seek(SeekFrom::Start(0)).ok()?;
        OggStreamReader::new(src).ok()
    });
    f.pos = 0;
    f.pos_unknown = false;
    if f.rdr.is_none() {
        f.eof = true;
    } else if target > 0 {
        let ok = f.rdr.as_mut().is_some_and(|r| r.seek_absgp_pg(target).is_ok());
        f.pos_unknown = true;
        // decode until a page end gives the position, then drop up to the target
        while ok && f.pos_unknown && decode(&mut f) {}
        if trace_on() {
            eprintln!("vorbisfile: seek {vf:#x}: at {} (+{} pending), known {}", f.pos, f.pending.len(), !f.pos_unknown);
        }
        if !f.pos_unknown {
            f.skip = target.saturating_sub(f.pos);
            apply_skip(&mut f);
        }
    }
    put(vf, f);
    0
}

/// `(address, name, native entry)` of the import thunks.
pub fn thunks() -> Vec<(u32, &'static str, usize)> {
    macro_rules! entry {
        ($addr:literal, $name:literal, $f:path; fn($($a:ident : $t:ty),*) -> $r:ty) => {{
            // cdecl on x86 (the thunks' convention)
            #[allow(clippy::too_many_arguments)]
            extern "C-unwind" fn w($($a: $t),*) -> $r {
                $f($($a),*)
            }
            #[cfg(target_arch = "wasm32")]
            let e = sa_core::wabi_wrap!(w; $($a)*);
            #[cfg(not(target_arch = "wasm32"))]
            let e = w as *const () as usize;
            ($addr, $name, e)
        }};
    }
    vec![
        entry!(0x0081_8062, "vorbisfile.dll!ov_clear (thunk)", ov_clear; fn(vf: u32) -> i32),
        entry!(0x0081_8068, "vorbisfile.dll!ov_read (thunk)", ov_read; fn(vf: u32, buf: u32, len: u32, big: u32, word: u32, sgn: u32, bs: u32) -> i32),
        entry!(0x0081_806E, "vorbisfile.dll!ov_info (thunk)", ov_info; fn(vf: u32, link: i32) -> u32),
        entry!(0x0081_8074, "vorbisfile.dll!ov_open_callbacks (thunk)", ov_open_callbacks; fn(ds: u32, vf: u32, initial: u32, ibytes: u32, read: u32, seek: u32, close: u32, tell: u32) -> i32),
        entry!(0x0081_807A, "vorbisfile.dll!ov_time_total (thunk)", ov_time_total; fn(vf: u32, link: i32) -> f64),
        entry!(0x0081_8080, "vorbisfile.dll!ov_time_tell (thunk)", ov_time_tell; fn(vf: u32) -> f64),
        entry!(0x0081_8086, "vorbisfile.dll!ov_time_seek (thunk)", ov_time_seek; fn(vf: u32, s: f64) -> i32),
    ]
}
