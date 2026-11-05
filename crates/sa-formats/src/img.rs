//! IMG archive, version 2 (`VER2`), as used by GTA San Andreas.
//!
//! Layout (gta-reversed `CDirectory::DirectoryInfo`, verified on the 1.0 US install):
//! `"VER2"`, `u32 count`, then `count` entries of 32 bytes:
//! `u32 offset` (sectors), `u16 size` (sectors, the streaming size), `u16 size_in_archive`
//! (sectors; 0 in every SA archive), `char name[24]`.
//! A sector is 2048 bytes. Entry data follows the directory, starting at a sector boundary.
//!
//! Names are NUL-terminated, but the bytes after the NUL are not zeroed in the shipped archives
//! (uninitialised tool memory), so the full 24 raw bytes are kept for byte-exact round trips.
//!
//! IMG v1 (`.dir` + `.img`, GTA III/VC) does not occur in the SA install and is not implemented.

use crate::bytes::{Put, Reader, c_str};
use crate::error::{Error, Result};

/// Size of one IMG sector in bytes.
pub const SECTOR_SIZE: usize = 2048;
/// Magic at the start of a version 2 archive.
pub const VER2_MAGIC: [u8; 4] = *b"VER2";
/// Size of one directory entry in bytes.
pub const ENTRY_SIZE: usize = 32;

/// One directory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImgEntry {
    /// Offset of the entry data, in sectors from the start of the archive.
    pub offset: u32,
    /// Size in sectors (`CDirectory::DirectoryInfo::Size`, the streaming size).
    pub size: u16,
    /// Second size field (`SizeInArchive`); 0 in every 1.0 US archive.
    pub size_in_archive: u16,
    /// Raw name field. Only the bytes before the first NUL are the name.
    pub name_raw: [u8; 24],
}

impl ImgEntry {
    /// Name bytes up to the first NUL.
    pub fn name(&self) -> &[u8] {
        c_str(&self.name_raw)
    }

    /// Name as text (lossy for non-UTF-8 bytes).
    pub fn name_lossy(&self) -> String {
        String::from_utf8_lossy(self.name()).into_owned()
    }

    /// Byte offset of the entry data.
    pub fn byte_offset(&self) -> usize {
        self.offset as usize * SECTOR_SIZE
    }

    /// Byte length of the entry data (`size` sectors).
    pub fn byte_len(&self) -> usize {
        self.size as usize * SECTOR_SIZE
    }
}

/// The directory of a version 2 IMG archive.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImgDirectory {
    pub entries: Vec<ImgEntry>,
}

impl ImgDirectory {
    /// Number of bytes the directory occupies at the start of the archive (header + entries).
    pub fn header_len(&self) -> usize {
        8 + self.entries.len() * ENTRY_SIZE
    }

    /// Reads the number of bytes needed for the directory from the first 8 bytes of an archive.
    pub fn header_len_from_prefix(prefix: &[u8]) -> Result<usize> {
        let mut r = Reader::new(prefix, "IMG header");
        let magic = r.array::<4>()?;
        if magic != VER2_MAGIC {
            return Err(Error::invalid(
                "IMG header",
                format!("magic {magic:02x?} is not VER2"),
            ));
        }
        Ok(8 + r.u32()? as usize * ENTRY_SIZE)
    }

    /// Parses the directory from the start of an archive. Only the first
    /// [`header_len`](Self::header_len) bytes are read; the rest of `data` may be absent.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, "IMG header");
        let magic = r.array::<4>()?;
        if magic != VER2_MAGIC {
            return Err(Error::invalid(
                "IMG header",
                format!("magic {magic:02x?} is not VER2"),
            ));
        }
        let count = r.u32()? as usize;
        r.set_context("IMG directory");
        r.check_count(count, ENTRY_SIZE)?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(ImgEntry {
                offset: r.u32()?,
                size: r.u16()?,
                size_in_archive: r.u16()?,
                name_raw: r.array()?,
            });
        }
        Ok(ImgDirectory { entries })
    }

    /// Serialises the directory (header + entries), without padding to a sector boundary.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.header_len());
        out.extend_from_slice(&VER2_MAGIC);
        out.put_u32(self.entries.len() as u32);
        for e in &self.entries {
            out.put_u32(e.offset);
            out.put_u16(e.size);
            out.put_u16(e.size_in_archive);
            out.extend_from_slice(&e.name_raw);
        }
        out
    }

    /// Finds an entry by name, case-insensitively (as `CDirectory::FindItem` compares names).
    pub fn find(&self, name: &str) -> Option<&ImgEntry> {
        self.entries
            .iter()
            .find(|e| e.name().eq_ignore_ascii_case(name.as_bytes()))
    }

    /// Returns the data of `entry` inside `archive` (the full archive bytes).
    pub fn entry_data<'a>(&self, archive: &'a [u8], entry: &ImgEntry) -> Result<&'a [u8]> {
        let start = entry.byte_offset();
        let end = start + entry.byte_len();
        archive.get(start..end).ok_or_else(|| {
            Error::invalid(
                "IMG entry",
                format!(
                    "entry {:?} [{start:#x}, {end:#x}) is outside the archive ({:#x} bytes)",
                    entry.name_lossy(),
                    archive.len()
                ),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(offset: u32, size: u16, name: &[u8]) -> ImgEntry {
        let mut name_raw = [0u8; 24];
        name_raw[..name.len()].copy_from_slice(name);
        ImgEntry {
            offset,
            size,
            size_in_archive: 0,
            name_raw,
        }
    }

    #[test]
    fn round_trip_keeps_name_garbage() {
        let mut e = entry(1, 2, b"a.dff");
        e.name_raw[10] = 0xCD; // bytes after the NUL must survive
        let dir = ImgDirectory {
            entries: vec![e, entry(3, 1, b"b.txd")],
        };
        let bytes = dir.to_bytes();
        assert_eq!(bytes.len(), 8 + 64);
        assert_eq!(&bytes[..4], b"VER2");
        let back = ImgDirectory::parse(&bytes).unwrap();
        assert_eq!(back, dir);
        assert_eq!(back.to_bytes(), bytes);
        assert_eq!(back.entries[0].name(), b"a.dff");
        assert_eq!(ImgDirectory::header_len_from_prefix(&bytes[..8]).unwrap(), 72);
        assert_eq!(back.find("B.TXD").unwrap().offset, 3);
    }

    #[test]
    fn entry_data_bounds() {
        let dir = ImgDirectory {
            entries: vec![entry(1, 1, b"x"), entry(2, 1, b"y")],
        };
        let mut archive = dir.to_bytes();
        archive.resize(2 * SECTOR_SIZE, 0);
        archive[SECTOR_SIZE] = 7;
        assert_eq!(dir.entry_data(&archive, &dir.entries[0]).unwrap()[0], 7);
        assert!(dir.entry_data(&archive, &dir.entries[1]).is_err());
    }

    #[test]
    fn rejects_bad_magic_and_truncation() {
        assert!(ImgDirectory::parse(b"VER3\0\0\0\0").is_err());
        let mut b = b"VER2".to_vec();
        b.put_u32(2);
        b.extend_from_slice(&[0; 40]);
        assert!(matches!(ImgDirectory::parse(&b), Err(Error::Truncated { .. })));
    }
}
