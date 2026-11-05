//! SFX bank packs (`audio/SFX/<pak>`), as `CAEMP3BankLoader::Service` (0x4DFE30) reads them.
//!
//! `BankLkup.dat` gives each bank's pak, byte offset and sample-data size. At the offset lies a
//! 0x12C4-byte header (`AEAudioStream`): `i16` sound count, 2 more bytes, then 400 sound
//! entries ([`SoundEntry`], 12 bytes each). The sample data (`size` bytes) follows directly.
//! A whole-bank load reads `0x12C4 + size` bytes from the sector containing `offset`
//! (`CdStreamRead` with `offset / 2048` and `(bytes / 2048) + 2` sectors, the data pointer
//! advanced by `offset % 2048`). A single-sound load reads the header, then
//! `offset + 0x12C4 + sounds[id].offset`, length `sounds[id + 1].offset - sounds[id].offset`,
//! or `size - sounds[id].offset` for the last sound (`id + 1 >= count`).
//!
//! The payload is kept as a byte range: the packs are up to 1 GB and the sample data is not
//! interpreted here.

use super::config::{LookupEntry, MAX_SOUNDS, SoundEntry};
use crate::bytes::{Put, Reader};
use crate::error::{Error, Result};

/// Size of a bank header (`sizeof(AEAudioStream)`).
pub const BANK_HEADER_SIZE: usize = 0x12C4;

/// The header in front of every bank's sample data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankHeader {
    /// Number of valid entries in `sounds`.
    pub num_sounds: i16,
    /// The 2 bytes after the count (0 in every shipped bank).
    pub pad: [u8; 2],
    /// All 400 entries (only the first `num_sounds` are used).
    pub sounds: Vec<SoundEntry>,
}

impl BankHeader {
    /// Parses the first [`BANK_HEADER_SIZE`] bytes of `data`.
    pub fn parse(data: &[u8]) -> Result<BankHeader> {
        let mut r = Reader::new(data, "SFX bank header");
        r.check_count(1, BANK_HEADER_SIZE)?;
        let num_sounds = r.i16()?;
        let pad = r.array()?;
        let mut sounds = Vec::with_capacity(MAX_SOUNDS);
        for _ in 0..MAX_SOUNDS {
            sounds.push(SoundEntry::read(&mut r)?);
        }
        Ok(BankHeader {
            num_sounds,
            pad,
            sounds,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(BANK_HEADER_SIZE);
        o.put_i16(self.num_sounds);
        o.extend_from_slice(&self.pad);
        for s in &self.sounds {
            s.write(&mut o);
        }
        o
    }

    /// The used entries.
    pub fn used(&self) -> &[SoundEntry] {
        let n = (self.num_sounds.max(0) as usize).min(self.sounds.len());
        &self.sounds[..n]
    }

    /// Byte length of each used sound, as the single-sound loader computes it, given the
    /// bank's data size from `BankLkup.dat`. Errors when offsets decrease or exceed the size.
    pub fn sound_sizes(&self, data_size: u32) -> Result<Vec<u32>> {
        let used = self.used();
        if self.num_sounds < 0 || self.num_sounds as usize > MAX_SOUNDS {
            return Err(Error::invalid(
                "SFX bank header",
                format!("sound count {} outside 0..=400", self.num_sounds),
            ));
        }
        let mut v = Vec::with_capacity(used.len());
        for (i, s) in used.iter().enumerate() {
            let end = used.get(i + 1).map_or(data_size, |n| n.offset);
            if end < s.offset || end > data_size {
                return Err(Error::invalid(
                    "SFX bank header",
                    format!(
                        "sound {i}: offset {:#x}, next {end:#x}, bank data size {data_size:#x}",
                        s.offset
                    ),
                ));
            }
            v.push(end - s.offset);
        }
        Ok(v)
    }
}

/// One bank inside a pak, located by its lookup entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankLocation {
    /// Bank id (index in `BankLkup.dat`).
    pub bank: usize,
    pub lookup: LookupEntry,
}

impl BankLocation {
    /// Byte range of the header in the pak.
    pub fn header_range(&self) -> std::ops::Range<u64> {
        let o = self.lookup.offset as u64;
        o..o + BANK_HEADER_SIZE as u64
    }
    /// Byte range of the sample data in the pak.
    pub fn data_range(&self) -> std::ops::Range<u64> {
        let o = self.lookup.offset as u64 + BANK_HEADER_SIZE as u64;
        o..o + self.lookup.size as u64
    }
}

/// How the banks of one pak cover the file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PakLayout {
    /// Banks in file order.
    pub banks: Vec<BankLocation>,
    /// `(start, end)` of byte ranges no bank covers.
    pub gaps: Vec<(u64, u64)>,
    /// Pairs of banks whose ranges overlap.
    pub overlaps: Vec<(usize, usize)>,
}

impl PakLayout {
    /// Checks the bank geometry of one pak of `file_len` bytes. Errors when a bank lies
    /// outside the file.
    pub fn check(file_len: u64, mut banks: Vec<BankLocation>) -> Result<PakLayout> {
        banks.sort_by_key(|b| (b.lookup.offset, b.bank));
        let mut gaps = Vec::new();
        let mut overlaps = Vec::new();
        let mut pos = 0u64;
        let mut prev: Option<usize> = None;
        for b in &banks {
            let r = b.header_range().start..b.data_range().end;
            if r.end > file_len {
                return Err(Error::invalid(
                    "SFX pak",
                    format!(
                        "bank {} [{:#x}, {:#x}) beyond file end {file_len:#x}",
                        b.bank, r.start, r.end
                    ),
                ));
            }
            if r.start > pos {
                gaps.push((pos, r.start));
            } else if r.start < pos {
                overlaps.push((prev.unwrap_or(b.bank), b.bank));
            }
            pos = pos.max(r.end);
            prev = Some(b.bank);
        }
        if pos < file_len {
            gaps.push((pos, file_len));
        }
        Ok(PakLayout {
            banks,
            gaps,
            overlaps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(offsets: &[u32]) -> BankHeader {
        let mut sounds = vec![
            SoundEntry {
                offset: 0,
                loop_start: u32::MAX,
                sample_rate: 22050,
                headroom: 0
            };
            MAX_SOUNDS
        ];
        for (s, o) in sounds.iter_mut().zip(offsets) {
            s.offset = *o;
        }
        BankHeader {
            num_sounds: offsets.len() as i16,
            pad: [0; 2],
            sounds,
        }
    }

    #[test]
    fn header_round_trip_and_sizes() {
        let h = header(&[0, 100, 250]);
        let b = h.to_bytes();
        assert_eq!(b.len(), BANK_HEADER_SIZE);
        let back = BankHeader::parse(&b).unwrap();
        assert_eq!(back, h);
        assert_eq!(back.sound_sizes(300).unwrap(), vec![100, 150, 50]);
        assert!(back.sound_sizes(200).is_err());
        assert!(BankHeader::parse(&b[..100]).is_err());
    }

    #[test]
    fn layout_gaps_and_bounds() {
        let loc = |bank, offset, size| BankLocation {
            bank,
            lookup: LookupEntry {
                file: 0,
                pad: [0; 3],
                offset,
                size,
            },
        };
        let h = BANK_HEADER_SIZE as u32;
        let l = PakLayout::check(2 * h as u64 + 20, vec![loc(1, h + 10, 10), loc(0, 0, 10)]).unwrap();
        assert_eq!(l.banks[0].bank, 0);
        assert!(l.overlaps.is_empty());
        assert!(l.gaps.is_empty());
        let l = PakLayout::check(2 * h as u64 + 40, vec![loc(0, 0, 10), loc(1, h + 10, 10)]).unwrap();
        assert_eq!(l.gaps, vec![(2 * h as u64 + 20, 2 * h as u64 + 40)]);
        let l = PakLayout::check(2 * h as u64 + 20, vec![loc(0, 0, 20), loc(1, h + 10, 10)]).unwrap();
        assert_eq!(l.overlaps, vec![(0, 1)]);
        assert!(PakLayout::check(h as u64, vec![loc(0, 0, 1)]).is_err());
    }
}
