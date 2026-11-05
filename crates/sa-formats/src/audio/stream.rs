//! Stream packs (`audio/streams/<pack>`): radio, cutscene, ambience and beat tracks.
//!
//! * Encryption: `CAEDataStream::FillBuffer` (0x4DC1C0) XORs every byte it reads with
//!   `key[pos & 15]` (`CAEStreamTransformer::TransformBuffer` 0x4F17D0), where `pos` is the
//!   absolute file position (`SetFilePointer` result in `CAEDataStream::Initialise` 0x4DC2B0,
//!   advanced by each read). The key is built by `CAEStreamTransformer::Initialise`
//!   (0x4F1750) as `stack[i] ^ i` from 16 immediate bytes; the result is [`KEY`]. So the whole
//!   pack decrypts uniformly, independent of track boundaries.
//! * Layout: `TrakLkup.dat` gives each track's pack, offset and size. At the offset lies a
//!   0x1F84-byte [`TrackInfo`] (`CAEMP3TrackLoader::GetTrackInfo` 0x4E0A70 reads it with a
//!   stream starting at `offset`); the audio stream starts at `offset + 0x1F84`
//!   (`CAETrackLoader::GetDataStream` 0x4E0D20, `add edx, 0x1f84`) with length `size`.
//! * Payload: one Ogg logical stream (Vorbis) per track. This module walks the Ogg pages
//!   (RFC 3533 framing, CRC-32 checked) and reads the Vorbis identification header's channel
//!   count and sample rate; it does not decode audio.

use super::config::LookupEntry;
use crate::bytes::{Put, Reader};
use crate::error::{Error, Result};

/// The XOR key (`CAEStreamTransformer::table` after `Initialise`).
pub const KEY: [u8; 16] = [
    0xEA, 0x3A, 0xC4, 0xA1, 0x9A, 0xA8, 0x14, 0xF3, 0x48, 0xB0, 0xD7, 0x23, 0x9D, 0xE8, 0xFF, 0xF1,
];

/// XORs `buf` in place, `buf[0]` being at absolute file position `file_pos`. The operation is
/// its own inverse.
pub fn xor_in_place(buf: &mut [u8], file_pos: u64) {
    let k = (file_pos & 15) as usize;
    for (i, b) in buf.iter_mut().enumerate() {
        *b ^= KEY[(k + i) & 15];
    }
}

/// Size of a [`TrackInfo`] (`sizeof(tTrackInfo)`).
pub const TRACK_INFO_SIZE: usize = 0x1F84;
/// Beat entries in a [`TrackInfo`].
pub const NUM_BEATS: usize = 1000;

/// The header in front of every track (`tTrackInfo`).
///
/// The exe uses only the beat table (`CAEAudioHardware::GetBeatInfo` 0x4D8FA0 scans it while
/// `key != 0`). The 16 words at 0x1F40 and the `u16` at 0x1F80 are not read by any code
/// gta-reversed has reversed; they are kept raw (see `docs/re/formats.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackInfo {
    /// `(time, key)` pairs.
    pub beats: Vec<(u32, u32)>,
    /// Words at 0x1F40.
    pub extra: [u32; 16],
    /// `u16` at 0x1F80.
    pub field_1f80: u16,
    /// Structure padding at 0x1F82.
    pub pad: [u8; 2],
}

impl TrackInfo {
    pub fn parse(data: &[u8]) -> Result<TrackInfo> {
        let mut r = Reader::new(data, "stream track info");
        r.check_count(1, TRACK_INFO_SIZE)?;
        let mut beats = Vec::with_capacity(NUM_BEATS);
        for _ in 0..NUM_BEATS {
            beats.push((r.u32()?, r.u32()?));
        }
        let mut extra = [0u32; 16];
        for w in &mut extra {
            *w = r.u32()?;
        }
        Ok(TrackInfo {
            beats,
            extra,
            field_1f80: r.u16()?,
            pad: r.array()?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(TRACK_INFO_SIZE);
        for (t, k) in &self.beats {
            o.put_u32(*t);
            o.put_u32(*k);
        }
        for w in self.extra {
            o.put_u32(w);
        }
        o.put_u16(self.field_1f80);
        o.extend_from_slice(&self.pad);
        o
    }

    /// Number of beats before the first entry with `key == 0` (where `GetBeatInfo`'s scan
    /// stops); `None` if all 1000 entries have a non-zero key.
    pub fn beat_count(&self) -> Option<usize> {
        self.beats.iter().position(|&(_, k)| k == 0)
    }
}

/// One Ogg page (RFC 3533).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OggPage {
    /// Absolute offset of the page in the pack file.
    pub offset: u64,
    /// Total page length (header + lacing + body).
    pub len: u32,
    pub version: u8,
    /// Bit 0 continued, bit 1 first page (BOS), bit 2 last page (EOS).
    pub header_type: u8,
    pub granule: u64,
    pub serial: u32,
    pub sequence: u32,
    pub crc: u32,
    /// Whether `crc` matches the page's CRC-32 (polynomial 0x04C11DB7, init 0, no reflection,
    /// computed with the CRC field zeroed).
    pub crc_ok: bool,
}

const fn ogg_crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut r = (i as u32) << 24;
        let mut j = 0;
        while j < 8 {
            r = if r & 0x8000_0000 != 0 {
                (r << 1) ^ 0x04C1_1DB7
            } else {
                r << 1
            };
            j += 1;
        }
        t[i] = r;
        i += 1;
    }
    t
}

static OGG_CRC: [u32; 256] = ogg_crc_table();

/// CRC-32 of an Ogg page as stored in its header (the 4 CRC bytes at 22..26 count as zero).
pub fn ogg_page_crc(page: &[u8]) -> u32 {
    let mut c = 0u32;
    for (i, &b) in page.iter().enumerate() {
        let b = if (22..26).contains(&i) { 0 } else { b };
        c = (c << 8) ^ OGG_CRC[((c >> 24) as u8 ^ b) as usize];
    }
    c
}

/// Walks the Ogg pages that tile `data[start..end]` exactly. `base` is the absolute file
/// offset of `data[0]`.
pub fn walk_ogg_pages(data: &[u8], start: usize, end: usize, base: u64) -> Result<Vec<OggPage>> {
    let mut pages = Vec::new();
    let mut q = start;
    while q < end {
        let fail = |what: String| Error::invalid("Ogg stream", format!("at {:#x}: {what}", base + q as u64));
        if end - q < 27 || &data[q..q + 4] != b"OggS" {
            return Err(fail("no Ogg page".into()));
        }
        let nseg = data[q + 26] as usize;
        if end - q < 27 + nseg {
            return Err(fail("lacing table runs past the stream end".into()));
        }
        let body: usize = data[q + 27..q + 27 + nseg].iter().map(|&b| b as usize).sum();
        let len = 27 + nseg + body;
        if end - q < len {
            return Err(fail(format!("page of {len} bytes runs past the stream end")));
        }
        let page = &data[q..q + len];
        let mut r = Reader::new(&page[4..], "Ogg page");
        let version = r.u8()?;
        let header_type = r.u8()?;
        let granule = u64::from(r.u32()?) | (u64::from(r.u32()?) << 32);
        let serial = r.u32()?;
        let sequence = r.u32()?;
        let crc = r.u32()?;
        pages.push(OggPage {
            offset: base + q as u64,
            len: len as u32,
            version,
            header_type,
            granule,
            serial,
            sequence,
            crc,
            crc_ok: ogg_page_crc(page) == crc,
        });
        q += len;
    }
    Ok(pages)
}

/// How a track's `TrakLkup.dat` size relates to its Ogg stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeKind {
    /// `size` is the Ogg stream length.
    OggOnly,
    /// `size` is the Ogg stream length plus the 0x1F84-byte header (so the exe's stream
    /// length reaches 0x1F84 bytes into the next track, or past the end of the pack).
    IncludesHeader,
}

/// One track of a stream pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamTrack {
    /// Track id (index in `TrakLkup.dat`).
    pub track: usize,
    pub lookup: LookupEntry,
    pub info: TrackInfo,
    /// Absolute byte range of the Ogg stream (up to the next track or the end of the pack).
    pub ogg: std::ops::Range<u64>,
    pub pages: Vec<OggPage>,
    pub size_kind: SizeKind,
    /// From the Vorbis identification header (first packet): channels and sample rate.
    pub vorbis_channels: u8,
    pub vorbis_rate: u32,
}

impl StreamTrack {
    /// Whether the pages form one well-formed logical stream: a single serial, sequence
    /// numbers 0, 1, 2, ..., BOS only on the first page, EOS only on the last, every CRC ok.
    pub fn is_single_clean_stream(&self) -> bool {
        let n = self.pages.len();
        n > 0
            && self.pages.iter().enumerate().all(|(i, p)| {
                p.serial == self.pages[0].serial
                    && p.sequence == i as u32
                    && p.version == 0
                    && ((p.header_type & 2) != 0) == (i == 0)
                    && ((p.header_type & 4) != 0) == (i + 1 == n)
                    && p.crc_ok
            })
    }
}

/// A decrypted stream pack, split into tracks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamPack {
    /// Tracks in file order.
    pub tracks: Vec<StreamTrack>,
    /// `(start, end)` ranges before the first track or between tracks that no track header
    /// starts (empty in the install).
    pub gaps: Vec<(u64, u64)>,
}

impl StreamPack {
    /// Parses a whole pack. `decrypted` is the pack file after [`xor_in_place`] with position
    /// 0; `tracks` are the `TrakLkup.dat` entries for this pack with their track ids.
    pub fn parse(decrypted: &[u8], tracks: &[(usize, LookupEntry)]) -> Result<StreamPack> {
        let mut sorted: Vec<&(usize, LookupEntry)> = tracks.iter().collect();
        sorted.sort_by_key(|(id, e)| (e.offset, *id));
        let mut out = Vec::with_capacity(sorted.len());
        let mut gaps = Vec::new();
        let mut pos = 0u64;
        for (i, (id, e)) in sorted.iter().enumerate() {
            let start = e.offset as usize;
            let region_end = sorted
                .get(i + 1)
                .map_or(decrypted.len(), |(_, n)| n.offset as usize);
            if (start as u64) < pos {
                return Err(Error::invalid(
                    "stream pack",
                    format!("track {id} at {start:#x} starts inside the previous track"),
                ));
            }
            if start as u64 > pos {
                gaps.push((pos, start as u64));
            }
            if region_end < start + TRACK_INFO_SIZE || region_end > decrypted.len() {
                return Err(Error::invalid(
                    "stream pack",
                    format!("track {id} at {start:#x}: no room for the track header before {region_end:#x}"),
                ));
            }
            let info = TrackInfo::parse(&decrypted[start..])?;
            let ogg_start = start + TRACK_INFO_SIZE;
            let pages = walk_ogg_pages(decrypted, ogg_start, region_end, 0)?;
            let ogg_len = (region_end - ogg_start) as u64;
            let size_kind = if u64::from(e.size) == ogg_len {
                SizeKind::OggOnly
            } else if u64::from(e.size) == ogg_len + TRACK_INFO_SIZE as u64 {
                SizeKind::IncludesHeader
            } else {
                return Err(Error::invalid(
                    "stream pack",
                    format!("track {id}: size {} vs Ogg stream length {ogg_len}", e.size),
                ));
            };
            let (vorbis_channels, vorbis_rate) =
                vorbis_id(&decrypted[ogg_start..region_end]).ok_or_else(|| {
                    Error::invalid(
                        "stream pack",
                        format!("track {id}: no Vorbis identification header"),
                    )
                })?;
            out.push(StreamTrack {
                track: *id,
                lookup: *e,
                info,
                ogg: ogg_start as u64..region_end as u64,
                pages,
                size_kind,
                vorbis_channels,
                vorbis_rate,
            });
            pos = region_end as u64;
        }
        if sorted.is_empty() && !decrypted.is_empty() {
            gaps.push((0, decrypted.len() as u64));
        }
        Ok(StreamPack { tracks: out, gaps })
    }

    /// Rebuilds the encrypted pack from the parsed headers and the decrypted payload ranges.
    pub fn rebuild(&self, decrypted: &[u8]) -> Vec<u8> {
        let mut o = Vec::with_capacity(decrypted.len());
        let mut gaps = self.gaps.iter().peekable();
        for t in &self.tracks {
            while let Some(&&(a, b)) = gaps.peek() {
                if a >= u64::from(t.lookup.offset) {
                    break;
                }
                o.extend_from_slice(&decrypted[a as usize..b as usize]);
                gaps.next();
            }
            o.extend_from_slice(&t.info.to_bytes());
            o.extend_from_slice(&decrypted[t.ogg.start as usize..t.ogg.end as usize]);
        }
        for &(a, b) in gaps {
            o.extend_from_slice(&decrypted[a as usize..b as usize]);
        }
        xor_in_place(&mut o, 0);
        o
    }
}

/// Channels and sample rate from the Vorbis identification header in the first page of an Ogg
/// stream (`\x01vorbis`, version, channels, rate).
fn vorbis_id(ogg: &[u8]) -> Option<(u8, u32)> {
    let nseg = *ogg.get(26)? as usize;
    let p = ogg.get(27 + nseg..27 + nseg + 16)?;
    if &p[..7] != b"\x01vorbis" {
        return None;
    }
    Some((p[11], u32::from_le_bytes([p[12], p[13], p[14], p[15]])))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ogg_page(header_type: u8, seq: u32, body: &[u8]) -> Vec<u8> {
        let mut p = b"OggS".to_vec();
        p.push(0);
        p.push(header_type);
        p.extend_from_slice(&0u64.to_le_bytes());
        p.put_u32(0x1234);
        p.put_u32(seq);
        p.put_u32(0);
        assert!(body.len() < 255);
        p.push(1);
        p.push(body.len() as u8);
        p.extend_from_slice(body);
        let c = ogg_page_crc(&p);
        p[22..26].copy_from_slice(&c.to_le_bytes());
        p
    }

    fn info() -> TrackInfo {
        let mut beats = vec![(0u32, 0u32); NUM_BEATS];
        beats[0] = (100, 1);
        beats[1] = (200, 2);
        TrackInfo {
            beats,
            extra: [0xCDCD_CDCD; 16],
            field_1f80: 1,
            pad: [0xCD; 2],
        }
    }

    #[test]
    fn key_matches_exe_construction() {
        // 0x4F175D..0x4F17A8 store these bytes; the loop at 0x4F17B0 XORs each with its index.
        let imm = [
            0xEA, 0x3B, 0xC6, 0xA2, 0x9E, 0xAD, 0x12, 0xF4, 0x40, 0xB9, 0xDD, 0x28, 0x91, 0xE5, 0xF1, 0xFE,
        ];
        let k: Vec<u8> = imm.iter().enumerate().map(|(i, b)| b ^ i as u8).collect();
        assert_eq!(k, KEY);
        let mut b = [0u8; 20];
        xor_in_place(&mut b[3..], 3);
        assert_eq!(b[3], KEY[3]);
        assert_eq!(b[16], KEY[0]);
        xor_in_place(&mut b[3..], 3);
        assert!(b.iter().all(|&x| x == 0));
    }

    #[test]
    fn crc_known_vector() {
        // A page whose CRC is computed independently: same algorithm as libogg
        // (ogg_page_checksum_set); check the table entry for 1 (poly) and stability.
        assert_eq!(OGG_CRC[1], 0x04C1_1DB7);
        // CRC-32/CKSUM check value 0x765E7680 before its final XOR with 0xFFFFFFFF.
        assert_eq!(ogg_page_crc(b"123456789"), 0x765E_7680 ^ 0xFFFF_FFFF);
        let p = ogg_page(2, 0, b"abc");
        let c = u32::from_le_bytes([p[22], p[23], p[24], p[25]]);
        assert_eq!(ogg_page_crc(&p), c);
    }

    #[test]
    fn pack_round_trip() {
        let mut id = b"\x01vorbis".to_vec();
        id.extend_from_slice(&0u32.to_le_bytes());
        id.push(2);
        id.extend_from_slice(&32000u32.to_le_bytes());
        let mut track = info().to_bytes();
        assert_eq!(track.len(), TRACK_INFO_SIZE);
        let s1 = track.len();
        track.extend(ogg_page(2, 0, &id));
        track.extend(ogg_page(4, 1, b"data"));
        let ogg_len = (track.len() - s1) as u32;
        let mut file = track.clone();
        file.extend(&track); // second track, size includes the header
        let entries = [
            (
                0usize,
                LookupEntry {
                    file: 0,
                    pad: [0xCD; 3],
                    offset: 0,
                    size: ogg_len,
                },
            ),
            (
                1usize,
                LookupEntry {
                    file: 0,
                    pad: [0xCD; 3],
                    offset: track.len() as u32,
                    size: ogg_len + TRACK_INFO_SIZE as u32,
                },
            ),
        ];
        let mut enc = file.clone();
        xor_in_place(&mut enc, 0);
        let mut dec = enc.clone();
        xor_in_place(&mut dec, 0);
        assert_eq!(dec, file);
        let pack = StreamPack::parse(&dec, &entries).unwrap();
        assert_eq!(pack.tracks.len(), 2);
        assert!(pack.gaps.is_empty());
        let t = &pack.tracks[0];
        assert_eq!(t.size_kind, SizeKind::OggOnly);
        assert_eq!(pack.tracks[1].size_kind, SizeKind::IncludesHeader);
        assert_eq!((t.vorbis_channels, t.vorbis_rate), (2, 32000));
        assert!(t.is_single_clean_stream());
        assert_eq!(t.info.beat_count(), Some(2));
        assert_eq!(pack.rebuild(&dec), enc);

        // A corrupt CRC is reported, not fatal.
        let mut bad = dec.clone();
        let last = bad.len() - 1;
        bad[last] ^= 1;
        let p = StreamPack::parse(&bad, &entries).unwrap();
        assert!(!p.tracks[1].is_single_clean_stream());
        // A size that matches neither interpretation is an error.
        let mut wrong = entries;
        wrong[0].1.size += 1;
        assert!(StreamPack::parse(&dec, &wrong).is_err());
    }
}
