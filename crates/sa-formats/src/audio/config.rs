//! `audio/CONFIG/*.dat`: the audio index files. Each is read by the exe in one `Read` call into
//! an array of fixed-size records (a memory image), so every byte is kept for round trips,
//! including MSVC debug-heap fill bytes (`0xCC`/`0xCD`) in padding and after string NULs.
//!
//! | file | loader (1.0 US) | open | record | count |
//! |---|---|---|---|---|
//! | BankLkup.dat | `CAEBankLoader::LoadBankLookupFile` 0x4DFBD0 | `CFileMgr::OpenFile` `"rb"` | 12 bytes: `u8` pak, 3 pad, `u32` byte offset, `u32` byte size | `size / 12` |
//! | PakFiles.dat | `CAEBankLoader::LoadSFXPakLookupFile` 0x4DFC70 | `"rb"` | 0x34 bytes: `char[12]` name, `u32[10]` (DVD LSNs, unused on PC) | `size / 0x34` |
//! | BankSlot.dat | `CAEBankLoader::LoadBankSlotFile` 0x4E0590 | `"rb"` | `u16` count, then `size - 2` bytes into `count` x 0x12D4 | `u16` |
//! | StrmPaks.dat | `CAEMP3TrackLoader::LoadStreamPackTable` 0x4E0970 | `CreateFileA` | `char[16]` | `size >> 4` |
//! | TrakLkup.dat | `CAEMP3TrackLoader::LoadTrackLookupTable` 0x4E09F0 | `CreateFileA` | 12 bytes: `u8` pack, 3 pad, `u32` offset, `u32` size | `size / 12` |
//! | EventVol.dat | audio entity static init 0x5B9D50.. | `"r"` (text mode) | `i8` per audio event | fixed 0xB159 = 45,401 |

use super::nonzero;
use crate::bytes::{Put, Reader, c_str};
use crate::data::{DataFile, dump_bytes, dump_rec};
use crate::error::{Error, Result};

/// One 12-byte lookup record (`AEBankLookup` / `tTrackLookup`): `u8` file index, 3 padding
/// bytes, `u32` byte offset, `u32` byte size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookupEntry {
    /// Index into `PakFiles.dat` (banks) or `StrmPaks.dat` (tracks).
    pub file: u8,
    /// Structure padding (`0xCC` in BankLkup.dat, `0xCD` in TrakLkup.dat).
    pub pad: [u8; 3],
    /// Byte offset in the pack file.
    pub offset: u32,
    /// Byte size (bank: sample data after the 0x12C4 header; track: see [`super::stream`]).
    pub size: u32,
}

/// Size of one [`LookupEntry`].
pub const LOOKUP_ENTRY_SIZE: usize = 12;

fn parse_lookups(data: &[u8], context: &'static str) -> Result<(Vec<LookupEntry>, Vec<u8>)> {
    let mut r = Reader::new(data, context);
    let n = data.len() / LOOKUP_ENTRY_SIZE;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(LookupEntry {
            file: r.u8()?,
            pad: r.array()?,
            offset: r.u32()?,
            size: r.u32()?,
        });
    }
    Ok((v, r.rest().to_vec()))
}

fn put_lookups(o: &mut Vec<u8>, v: &[LookupEntry]) {
    for e in v {
        o.put_u8(e.file);
        o.extend_from_slice(&e.pad);
        o.put_u32(e.offset);
        o.put_u32(e.size);
    }
}

/// `BankLkup.dat`: one record per sound bank (bank id = index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankLookup {
    pub banks: Vec<LookupEntry>,
    /// Bytes after the last whole record (read into memory, never used).
    pub trailing: Vec<u8>,
}

impl BankLookup {
    pub fn parse(data: &[u8]) -> Result<BankLookup> {
        let (banks, trailing) = parse_lookups(data, "BankLkup.dat")?;
        Ok(BankLookup { banks, trailing })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(self.banks.len() * LOOKUP_ENTRY_SIZE + self.trailing.len());
        put_lookups(&mut o, &self.banks);
        o.extend_from_slice(&self.trailing);
        o
    }
}

impl DataFile for BankLookup {
    fn counts(&self) -> Vec<(String, u64)> {
        nonzero(vec![("BankLkup.dat: banks".into(), self.banks.len() as u64)])
    }
    fn dump(&self) -> String {
        let mut out = String::new();
        for (i, e) in self.banks.iter().enumerate() {
            dump_rec!(
                &mut out,
                "bank",
                id = i,
                pak = e.file,
                offset = e.offset,
                size = e.size,
                pad = e.pad
            );
        }
        out
    }
}

/// `TrakLkup.dat`: one record per stream track (track id = index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackLookup {
    pub tracks: Vec<LookupEntry>,
    /// Bytes after the last whole record.
    pub trailing: Vec<u8>,
}

impl TrackLookup {
    pub fn parse(data: &[u8]) -> Result<TrackLookup> {
        let (tracks, trailing) = parse_lookups(data, "TrakLkup.dat")?;
        Ok(TrackLookup { tracks, trailing })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(self.tracks.len() * LOOKUP_ENTRY_SIZE + self.trailing.len());
        put_lookups(&mut o, &self.tracks);
        o.extend_from_slice(&self.trailing);
        o
    }
}

impl DataFile for TrackLookup {
    fn counts(&self) -> Vec<(String, u64)> {
        nonzero(vec![("TrakLkup.dat: tracks".into(), self.tracks.len() as u64)])
    }
    fn dump(&self) -> String {
        let mut out = String::new();
        for (i, e) in self.tracks.iter().enumerate() {
            dump_rec!(
                &mut out,
                "track",
                id = i,
                pack = e.file,
                offset = e.offset,
                size = e.size,
                pad = e.pad
            );
        }
        out
    }
}

/// Size of one `AEPakLookup` record.
pub const PAK_ENTRY_SIZE: usize = 0x34;

/// One `PakFiles.dat` record (`AEPakLookup`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PakEntry {
    /// `char[12]`; the exe opens `AUDIO\SFX\<name>`. Bytes after the NUL are kept raw.
    pub name_raw: [u8; 12],
    /// Ten `u32` (gta-reversed: DVD copy LSNs); 0 everywhere, not read on PC.
    pub lsns: [u32; 10],
}

impl PakEntry {
    pub fn name(&self) -> &[u8] {
        c_str(&self.name_raw)
    }
}

/// `PakFiles.dat`: the SFX pack file names (pak id = index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PakFiles {
    pub paks: Vec<PakEntry>,
    pub trailing: Vec<u8>,
}

impl PakFiles {
    pub fn parse(data: &[u8]) -> Result<PakFiles> {
        let mut r = Reader::new(data, "PakFiles.dat");
        let n = data.len() / PAK_ENTRY_SIZE;
        let mut paks = Vec::with_capacity(n);
        for _ in 0..n {
            let name_raw = r.array()?;
            let mut lsns = [0u32; 10];
            for l in &mut lsns {
                *l = r.u32()?;
            }
            paks.push(PakEntry { name_raw, lsns });
        }
        Ok(PakFiles {
            paks,
            trailing: r.rest().to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(self.paks.len() * PAK_ENTRY_SIZE + self.trailing.len());
        for p in &self.paks {
            o.extend_from_slice(&p.name_raw);
            for l in p.lsns {
                o.put_u32(l);
            }
        }
        o.extend_from_slice(&self.trailing);
        o
    }
}

impl DataFile for PakFiles {
    fn counts(&self) -> Vec<(String, u64)> {
        nonzero(vec![("PakFiles.dat: SFX paks".into(), self.paks.len() as u64)])
    }
    fn dump(&self) -> String {
        let mut out = String::new();
        for (i, p) in self.paks.iter().enumerate() {
            dump_rec!(
                &mut out,
                "pak",
                id = i,
                name = p.name(),
                name_raw = &p.name_raw[..],
                lsns = p.lsns
            );
        }
        out
    }
}

/// Size of one `StrmPaks.dat` record.
pub const STREAM_PACK_ENTRY_SIZE: usize = 16;

/// `StrmPaks.dat`: stream pack file names (pack id = index), `char[16]` each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamPacks {
    pub names_raw: Vec<[u8; 16]>,
    pub trailing: Vec<u8>,
}

impl StreamPacks {
    pub fn parse(data: &[u8]) -> Result<StreamPacks> {
        let mut r = Reader::new(data, "StrmPaks.dat");
        let n = data.len() / STREAM_PACK_ENTRY_SIZE;
        let mut names_raw = Vec::with_capacity(n);
        for _ in 0..n {
            names_raw.push(r.array()?);
        }
        Ok(StreamPacks {
            names_raw,
            trailing: r.rest().to_vec(),
        })
    }

    /// Name of pack `i` up to the NUL (empty for an unused slot).
    pub fn name(&self, i: usize) -> Option<&[u8]> {
        self.names_raw.get(i).map(|n| c_str(n))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(self.names_raw.len() * 16 + self.trailing.len());
        for n in &self.names_raw {
            o.extend_from_slice(n);
        }
        o.extend_from_slice(&self.trailing);
        o
    }
}

impl DataFile for StreamPacks {
    fn counts(&self) -> Vec<(String, u64)> {
        let empty = self.names_raw.iter().filter(|n| n[0] == 0).count() as u64;
        nonzero(vec![
            ("StrmPaks.dat: entries".into(), self.names_raw.len() as u64),
            ("StrmPaks.dat: empty names".into(), empty),
        ])
    }
    fn dump(&self) -> String {
        let mut out = String::new();
        for (i, n) in self.names_raw.iter().enumerate() {
            dump_rec!(&mut out, "pack", id = i, name = c_str(n), name_raw = &n[..]);
        }
        out
    }
}

/// Size of one `CAEBankSlot` (in memory and in `BankSlot.dat`).
pub const BANK_SLOT_SIZE: usize = 0x12D4;
/// Sound entries per bank slot / bank header (`AE_BANK_MAX_NUM_SOUNDS`).
pub const MAX_SOUNDS: usize = 400;

/// One sound entry (`CAEBankSlotItem`, 12 bytes), as stored in `BankSlot.dat` and in every SFX
/// bank header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SoundEntry {
    /// Byte offset of the sample data from the end of the bank header.
    pub offset: u32,
    /// Loop start (0xFFFFFFFF = no loop).
    pub loop_start: u32,
    /// Sample rate in Hz.
    pub sample_rate: u16,
    /// Headroom (`int16`).
    pub headroom: i16,
}

impl SoundEntry {
    pub(crate) fn read(r: &mut Reader) -> Result<SoundEntry> {
        Ok(SoundEntry {
            offset: r.u32()?,
            loop_start: r.u32()?,
            sample_rate: r.u16()?,
            headroom: r.i16()?,
        })
    }
    pub(crate) fn write(&self, o: &mut Vec<u8>) {
        o.put_u32(self.offset);
        o.put_u32(self.loop_start);
        o.put_u16(self.sample_rate);
        o.put_i16(self.headroom);
    }
}

/// One `CAEBankSlot` as stored in `BankSlot.dat`. The loader recomputes `offset` (running sum
/// of `num_bytes`, `CalculateBankSlotsInfosOffsets` 0x4DFBA0) and overwrites `bank`,
/// `num_sounds` and `sounds` when a bank is loaded; only `num_bytes` is used as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankSlot {
    /// Stale buffer offset (overwritten at load).
    pub offset: u32,
    /// Size of the slot's buffer region in bytes.
    pub num_bytes: u32,
    pub unused1: u32,
    pub unused2: u32,
    /// Bank loaded into the slot (`i16`).
    pub bank: i16,
    /// Number of sounds (`-1` = single-sound slot).
    pub num_sounds: i16,
    /// All 400 sound entries.
    pub sounds: Vec<SoundEntry>,
}

/// `BankSlot.dat`: the bank slot layout of the SFX buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankSlots {
    pub slots: Vec<BankSlot>,
    /// Bytes after `count` slots. The exe reads `size - 2` bytes into an array of `count`
    /// slots, so non-empty trailing bytes would overflow its heap block.
    pub trailing: Vec<u8>,
}

impl BankSlots {
    pub fn parse(data: &[u8]) -> Result<BankSlots> {
        let mut r = Reader::new(data, "BankSlot.dat");
        let count = r.u16()? as usize;
        r.check_count(count, BANK_SLOT_SIZE)?;
        let mut slots = Vec::with_capacity(count);
        for _ in 0..count {
            let offset = r.u32()?;
            let num_bytes = r.u32()?;
            let unused1 = r.u32()?;
            let unused2 = r.u32()?;
            let bank = r.i16()?;
            let num_sounds = r.i16()?;
            let mut sounds = Vec::with_capacity(MAX_SOUNDS);
            for _ in 0..MAX_SOUNDS {
                sounds.push(SoundEntry::read(&mut r)?);
            }
            slots.push(BankSlot {
                offset,
                num_bytes,
                unused1,
                unused2,
                bank,
                num_sounds,
                sounds,
            });
        }
        Ok(BankSlots {
            slots,
            trailing: r.rest().to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(2 + self.slots.len() * BANK_SLOT_SIZE + self.trailing.len());
        o.put_u16(self.slots.len() as u16);
        for s in &self.slots {
            o.put_u32(s.offset);
            o.put_u32(s.num_bytes);
            o.put_u32(s.unused1);
            o.put_u32(s.unused2);
            o.put_i16(s.bank);
            o.put_i16(s.num_sounds);
            for e in &s.sounds {
                e.write(&mut o);
            }
        }
        o.extend_from_slice(&self.trailing);
        o
    }

    /// Slot offsets as the loader computes them (running sum of `num_bytes`), and the SFX
    /// buffer size (`last offset + last num_bytes`).
    pub fn computed_offsets(&self) -> (Vec<u32>, u32) {
        let mut off = 0u32;
        let mut v = Vec::with_capacity(self.slots.len());
        for s in &self.slots {
            v.push(off);
            off = off.wrapping_add(s.num_bytes);
        }
        (v, off)
    }
}

impl DataFile for BankSlots {
    fn counts(&self) -> Vec<(String, u64)> {
        nonzero(vec![
            ("BankSlot.dat: slots".into(), self.slots.len() as u64),
            (
                "BankSlot.dat: buffer bytes".into(),
                self.computed_offsets().1 as u64,
            ),
        ])
    }
    fn dump(&self) -> String {
        let mut out = String::new();
        for (i, s) in self.slots.iter().enumerate() {
            dump_rec!(
                &mut out,
                "slot",
                id = i,
                offset = s.offset,
                num_bytes = s.num_bytes,
                unused1 = s.unused1,
                unused2 = s.unused2,
                bank = s.bank,
                num_sounds = s.num_sounds,
            );
            // Sound entries are all identical in the shipped file except where noted; dump
            // only the ones that differ from the first.
            for (k, e) in s.sounds.iter().enumerate() {
                if k == 0 || *e != s.sounds[0] {
                    dump_rec!(
                        &mut out,
                        " sound",
                        index = k,
                        offset = e.offset,
                        loop_start = e.loop_start,
                        sample_rate = e.sample_rate,
                        headroom = e.headroom,
                    );
                }
            }
        }
        out
    }
}

/// Number of bytes `EventVol.dat` contributes (`fread(buf, 1, 0xB159, f)` must return
/// 0xB159 or the audio engine fails to initialise).
pub const EVENT_VOLUME_COUNT: usize = 0xB159;

/// `EventVol.dat`: default volume (dB, `int8`) per audio event id.
///
/// The exe opens it in **text mode** (`"r"`, 0x85A53C), so a CR LF pair would read as one LF
/// byte and a 0x1A byte would end the read. Neither byte occurs in the read range of the
/// shipped file; [`EventVolumes::parse`] rejects such input rather than modelling the
/// translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventVolumes {
    pub volumes: Vec<i8>,
    /// Bytes after the 45,401 read ones (never read).
    pub trailing: Vec<u8>,
}

impl EventVolumes {
    pub fn parse(data: &[u8]) -> Result<EventVolumes> {
        if data.len() < EVENT_VOLUME_COUNT {
            return Err(Error::Truncated {
                context: "EventVol.dat",
                offset: 0,
                needed: EVENT_VOLUME_COUNT,
                available: data.len(),
            });
        }
        let (read, rest) = data.split_at(EVENT_VOLUME_COUNT);
        if let Some(p) = read.iter().position(|&b| b == 0x0D || b == 0x1A) {
            return Err(Error::unsupported(
                "EventVol.dat",
                format!(
                    "byte {:#04x} at {p:#x} is translated by the text-mode read",
                    read[p]
                ),
            ));
        }
        Ok(EventVolumes {
            volumes: read.iter().map(|&b| b as i8).collect(),
            trailing: rest.to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o: Vec<u8> = self.volumes.iter().map(|&v| v as u8).collect();
        o.extend_from_slice(&self.trailing);
        o
    }
}

impl DataFile for EventVolumes {
    fn counts(&self) -> Vec<(String, u64)> {
        let set = self.volumes.iter().filter(|&&v| v != -128).count() as u64;
        nonzero(vec![
            ("EventVol.dat: events".into(), self.volumes.len() as u64),
            ("EventVol.dat: events with volume != -128".into(), set),
            (
                "EventVol.dat: unread trailing bytes".into(),
                self.trailing.len() as u64,
            ),
        ])
    }
    fn dump(&self) -> String {
        let mut out = String::new();
        for (i, v) in self.volumes.iter().enumerate() {
            if *v != -128 {
                dump_rec!(&mut out, "event", id = i, volume = *v);
            }
        }
        let mut t = String::new();
        dump_bytes(&self.trailing, &mut t);
        out.push_str("trailing ");
        out.push_str(&t);
        out.push('\n');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups_round_trip_with_padding_and_trailing() {
        let mut b = Vec::new();
        b.push(3);
        b.extend_from_slice(&[0xCC; 3]);
        b.put_u32(0x1234);
        b.put_u32(99);
        b.push(7); // partial record
        let l = BankLookup::parse(&b).unwrap();
        assert_eq!(l.banks.len(), 1);
        assert_eq!(l.banks[0].file, 3);
        assert_eq!(l.banks[0].offset, 0x1234);
        assert_eq!(l.trailing, vec![7]);
        assert_eq!(l.to_bytes(), b);
        let t = TrackLookup::parse(&b).unwrap();
        assert_eq!(t.to_bytes(), b);
    }

    #[test]
    fn pak_and_stream_names() {
        let mut b = b"FEET\0\xcd\xcd\xcd\xcd\xcd\xcd\xcd".to_vec();
        b.extend_from_slice(&[0; 40]);
        let p = PakFiles::parse(&b).unwrap();
        assert_eq!(p.paks[0].name(), b"FEET");
        assert_eq!(p.to_bytes(), b);

        let mut s = [0u8; 32];
        s[..2].copy_from_slice(b"AA");
        let sp = StreamPacks::parse(&s).unwrap();
        assert_eq!(sp.name(0), Some(&b"AA"[..]));
        assert_eq!(sp.name(1), Some(&b""[..]));
        assert_eq!(sp.counts()[1].1, 1);
        assert_eq!(sp.to_bytes(), s);
    }

    #[test]
    fn bank_slots_round_trip_and_offsets() {
        let mut b = Vec::new();
        b.put_u16(2);
        for n in [100u32, 50] {
            b.put_u32(777); // stale offset
            b.put_u32(n);
            b.put_u32(u32::MAX);
            b.put_u32(u32::MAX);
            b.put_i16(-1);
            b.put_i16(-1);
            for _ in 0..MAX_SOUNDS {
                SoundEntry {
                    offset: 0,
                    loop_start: u32::MAX,
                    sample_rate: 22050,
                    headroom: -3,
                }
                .write(&mut b);
            }
        }
        assert_eq!(b.len(), 2 + 2 * BANK_SLOT_SIZE);
        let s = BankSlots::parse(&b).unwrap();
        assert_eq!(s.computed_offsets(), (vec![0, 100], 150));
        assert_eq!(s.to_bytes(), b);
        assert!(BankSlots::parse(&b[..b.len() - 1]).is_err());
    }

    #[test]
    fn event_volumes() {
        let mut b = vec![0x80u8; EVENT_VOLUME_COUNT + 1];
        b[5] = 0xFE;
        let e = EventVolumes::parse(&b).unwrap();
        assert_eq!(e.volumes[5], -2);
        assert_eq!(e.trailing.len(), 1);
        assert_eq!(e.to_bytes(), b);
        b[7] = 0x1A;
        assert!(matches!(EventVolumes::parse(&b), Err(Error::Unsupported { .. })));
        assert!(EventVolumes::parse(&b[..10]).is_err());
    }
}
