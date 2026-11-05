//! GXT text files (`text/*.gxt`, San Andreas layout).
//!
//! Loaders (1.0 US): `CText::Load` 0x6A01A0 (main table), `CText::LoadMissionText` 0x69FBF0
//! (one mission table), `CKeyArray::Load` 0x69F490, `CData::Load` 0x69F5D0,
//! `CMissionTextOffsets::Load` 0x69F670, key hash `CKeyGen::GetUppercaseKey` 0x53CF30.
//!
//! Layout: `u16 version`, `u16 bits per character` (both read and not checked by the exe), then
//! a run of chunks (`char fourcc[4]`, `u32 size`, data). `CText::Load` reads chunk headers until
//! it has seen both `TKEY` and `TDAT`: a header with size 0 is skipped, `TABL` loads the mission
//! table directory (12-byte entries `char name[8]`, `u32 offset`), `TKEY` the key array (8-byte
//! entries `u32 string offset into TDAT`, `u32 key hash`), `TDAT` the string data, anything
//! else is skipped by its size. A mission table (`LoadMissionText`) is read at its TABL offset:
//! `char name[8]`, then the same chunk loop (only `TKEY`/`TDAT` are recognised there).
//!
//! The `MAIN` TABL entry points at the main table's `TKEY` header (it has no name prefix and is
//! loaded by `CText::Load`, not by offset).
//!
//! Key hash: CRC-32 (reflected polynomial 0xEDB88320; the exe's table at 0x8CD068 equals the
//! standard table), initial value `0xFFFFFFFF`, **no final XOR**, each byte passed through the
//! CRT `toupper` (0x823A0E) first. Key lookup is a binary search over the hashes
//! (`CKeyArray::BinarySearch` 0x69F570, `int16` bounds), so the keys must be sorted by hash.

use crate::bytes::{Put, Reader};
use crate::error::{Error, Result};

const TABLE: [u32; 256] = crc_table();

const fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ 0xEDB8_8320
            } else {
                c >> 1
            };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}

/// `CKeyGen::GetUppercaseKey` (0x53CF30): CRC-32 of the upper-cased key without the final XOR.
/// Upper-casing is ASCII only (the CRT `toupper` in the "C" locale).
pub fn key_hash(key: &[u8]) -> u32 {
    let mut h = 0xFFFF_FFFFu32;
    for &b in key {
        let c = b.to_ascii_uppercase();
        h = TABLE[((u32::from(c) ^ h) & 0xFF) as usize] ^ (h >> 8);
    }
    h
}

/// `CKeyGen::GetKey` (case-sensitive variant, same CRC without upper-casing).
pub fn key_hash_case_sensitive(key: &[u8]) -> u32 {
    let mut h = 0xFFFF_FFFFu32;
    for &b in key {
        h = TABLE[((u32::from(b) ^ h) & 0xFF) as usize] ^ (h >> 8);
    }
    h
}

/// One TABL entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TablEntry {
    pub name_raw: [u8; 8],
    /// File offset of the table (of its name; for `MAIN` of its `TKEY` header).
    pub offset: u32,
}

impl TablEntry {
    pub fn name(&self) -> &[u8] {
        crate::bytes::c_str(&self.name_raw)
    }
}

/// One TKEY entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEntry {
    /// Byte offset of the string in TDAT.
    pub offset: u32,
    /// [`key_hash`] of the key name.
    pub hash: u32,
}

/// A chunk of a table's chunk run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkBody {
    /// A size-0 header (skipped by the exe).
    Empty,
    /// TABL directory (main run only).
    Tabl(Vec<TablEntry>),
    /// TKEY key array; `rest` holds `size % 8` leftover bytes.
    Tkey { keys: Vec<KeyEntry>, rest: Vec<u8> },
    /// TDAT string data.
    Tdat(Vec<u8>),
    /// Any other chunk (skipped by its size).
    Other(Vec<u8>),
}

/// A chunk header plus body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub fourcc: [u8; 4],
    pub size: u32,
    pub body: ChunkBody,
}

/// One text table: the main table (chunks after the file header) or a mission table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GxtTable {
    /// File offset where the table starts (its name, or the first chunk for the main table).
    pub start: usize,
    /// Mission tables: the 8-byte name read first. `None` for the main table.
    pub name_raw: Option<[u8; 8]>,
    /// The chunks the exe's loop reads, in order, up to the one completing TKEY + TDAT.
    pub chunks: Vec<Chunk>,
    /// Characters are 16-bit (from the file header).
    wide: bool,
    /// Index into `chunks` of the TKEY and TDAT the exe keeps (the last ones read).
    tkey: usize,
    tdat: usize,
    /// Copy of the key array for lookups.
    pub keys: Vec<KeyEntry>,
}

impl GxtTable {
    /// Table name (`MAIN` for the main table).
    pub fn name_lossy(&self) -> String {
        match &self.name_raw {
            Some(n) => String::from_utf8_lossy(crate::bytes::c_str(n)).into_owned(),
            None => "MAIN".into(),
        }
    }

    /// The TDAT bytes.
    pub fn tdat(&self) -> &[u8] {
        match &self.chunks[self.tdat].body {
            ChunkBody::Tdat(d) => d,
            _ => &[],
        }
    }

    /// The raw string (code units without the terminator) at a TDAT offset: bytes for 8-bit
    /// files, little-endian `u16` pairs for 16-bit files.
    pub fn string_at(&self, offset: u32) -> Option<&[u8]> {
        let d = self.tdat().get(offset as usize..)?;
        if self.wide {
            let n = d.as_chunks::<2>().0.iter().position(|c| *c == [0, 0])?;
            Some(&d[..n * 2])
        } else {
            let n = d.iter().position(|&b| b == 0)?;
            Some(&d[..n])
        }
    }

    /// Looks a key up like `CKeyArray::Search` (binary search over the hashes).
    pub fn get(&self, key: &str) -> Option<&[u8]> {
        let h = key_hash(key.as_bytes());
        let i = self.keys.binary_search_by(|k| k.hash.cmp(&h)).ok()?;
        self.string_at(self.keys[i].offset)
    }

    /// Checks that the key array is strictly sorted by hash and that every key points at a
    /// terminated string inside TDAT. Returns the number of strings.
    pub fn check(&self) -> std::result::Result<usize, String> {
        if let Some(w) = self.keys.windows(2).find(|w| w[0].hash >= w[1].hash) {
            return Err(format!(
                "keys not strictly sorted: {:#x} then {:#x}",
                w[0].hash, w[1].hash
            ));
        }
        if self.wide && !self.tdat().len().is_multiple_of(2) {
            return Err("odd TDAT size in a 16-bit file".into());
        }
        for k in &self.keys {
            if self.wide && k.offset % 2 != 0 {
                return Err(format!("key {:#x}: odd offset in a 16-bit file", k.hash));
            }
            if self.string_at(k.offset).is_none() {
                return Err(format!(
                    "key {:#x}: offset {} has no terminated string",
                    k.hash, k.offset
                ));
            }
        }
        Ok(self.keys.len())
    }

    fn parse(r: &mut Reader<'_>, start: usize, named: bool, main: bool, wide: bool) -> Result<GxtTable> {
        let name_raw = if named { Some(r.array::<8>()?) } else { None };
        let mut chunks = Vec::new();
        let (mut tkey, mut tdat) = (None, None);
        while tkey.is_none() || tdat.is_none() {
            let fourcc = r.array::<4>()?;
            let size = r.u32()?;
            let body = if size == 0 {
                ChunkBody::Empty
            } else {
                let data = r.bytes(size as usize)?;
                match &fourcc {
                    b"TABL" if main => {
                        let mut tr = Reader::new(data, "GXT TABL");
                        // CMissionTextOffsets::Load reads whole 12-byte entries; a remainder
                        // would be skipped by nothing (the exe reads entries until the count).
                        if size % 12 != 0 {
                            return Err(Error::invalid(
                                "GXT TABL",
                                format!("size {size} is not a multiple of 12"),
                            ));
                        }
                        let mut v = Vec::new();
                        while tr.remaining() > 0 {
                            v.push(TablEntry {
                                name_raw: tr.array()?,
                                offset: tr.u32()?,
                            });
                        }
                        ChunkBody::Tabl(v)
                    }
                    b"TKEY" => {
                        let mut tr = Reader::new(data, "GXT TKEY");
                        let mut keys = Vec::with_capacity(data.len() / 8);
                        while tr.remaining() >= 8 {
                            keys.push(KeyEntry {
                                offset: tr.u32()?,
                                hash: tr.u32()?,
                            });
                        }
                        tkey = Some(chunks.len());
                        ChunkBody::Tkey {
                            keys,
                            rest: tr.rest().to_vec(),
                        }
                    }
                    b"TDAT" => {
                        tdat = Some(chunks.len());
                        ChunkBody::Tdat(data.to_vec())
                    }
                    _ => ChunkBody::Other(data.to_vec()),
                }
            };
            chunks.push(Chunk { fourcc, size, body });
        }
        let (tkey, tdat) = (tkey.unwrap_or(0), tdat.unwrap_or(0));
        let keys = match &chunks[tkey].body {
            ChunkBody::Tkey { keys, .. } => keys.clone(),
            _ => Vec::new(),
        };
        Ok(GxtTable {
            start,
            name_raw,
            chunks,
            wide,
            tkey,
            tdat,
            keys,
        })
    }

    fn write(&self, out: &mut Vec<u8>) {
        if let Some(n) = &self.name_raw {
            out.extend_from_slice(n);
        }
        for c in &self.chunks {
            out.extend_from_slice(&c.fourcc);
            out.put_u32(c.size);
            match &c.body {
                ChunkBody::Empty => {}
                ChunkBody::Tabl(v) => {
                    for e in v {
                        out.extend_from_slice(&e.name_raw);
                        out.put_u32(e.offset);
                    }
                }
                ChunkBody::Tkey { keys, rest } => {
                    for k in keys {
                        out.put_u32(k.offset);
                        out.put_u32(k.hash);
                    }
                    out.extend_from_slice(rest);
                }
                ChunkBody::Tdat(d) | ChunkBody::Other(d) => out.extend_from_slice(d),
            }
        }
    }

    /// Index of the TKEY chunk the exe keeps.
    pub fn tkey_index(&self) -> usize {
        self.tkey
    }
}

/// A whole GXT file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gxt {
    pub version: u16,
    /// Bits per character: 8 or 16.
    pub char_bits: u16,
    /// The TABL directory (empty if the main run has none).
    pub tabl: Vec<TablEntry>,
    /// `tables[0]` is the main table; the rest are the mission tables in file order.
    pub tables: Vec<GxtTable>,
    /// Bytes between tables (`gaps[i]` precedes `tables[i + 1]`).
    pub gaps: Vec<Vec<u8>>,
    /// Bytes after the last table.
    pub trailing: Vec<u8>,
}

impl Gxt {
    /// Parses a GXT file: the main run as `CText::Load` reads it, then every TABL table at its
    /// offset as `LoadMissionText` reads it.
    pub fn parse(data: &[u8]) -> Result<Gxt> {
        let mut r = Reader::new(data, "GXT header");
        let version = r.u16()?;
        let char_bits = r.u16()?;
        let wide = match char_bits {
            8 => false,
            16 => true,
            b => {
                return Err(Error::unsupported(
                    "GXT header",
                    format!("{b} bits per character"),
                ));
            }
        };
        r.set_context("GXT main table");
        let main = GxtTable::parse(&mut r, 4, false, true, wide)?;
        let tabl = main
            .chunks
            .iter()
            .find_map(|c| match &c.body {
                ChunkBody::Tabl(v) => Some(v.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let main_end = data.len() - r.remaining();
        let mut offsets: Vec<usize> = tabl
            .iter()
            .map(|e| e.offset as usize)
            .filter(|&o| o >= main_end)
            .collect();
        offsets.sort_unstable();
        offsets.dedup();
        let mut pos = main_end;
        let mut tables = vec![main];
        let mut gaps = Vec::new();
        for o in offsets {
            if o < pos {
                return Err(Error::invalid(
                    "GXT TABL",
                    format!("table at {o:#x} overlaps the previous table (ends at {pos:#x})"),
                ));
            }
            let gap = data
                .get(pos..o)
                .ok_or_else(|| Error::invalid("GXT TABL", format!("offset {o:#x} past the end")))?;
            gaps.push(gap.to_vec());
            let mut tr = Reader::new(&data[o..], "GXT mission table");
            let t = GxtTable::parse(&mut tr, o, true, false, wide)?;
            pos = data.len() - tr.remaining();
            tables.push(t);
        }
        Ok(Gxt {
            version,
            char_bits,
            tabl,
            tables,
            gaps,
            trailing: data[pos..].to_vec(),
        })
    }

    /// Serialises back to the exact input bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.put_u16(self.version);
        out.put_u16(self.char_bits);
        for (i, t) in self.tables.iter().enumerate() {
            if i > 0 {
                out.extend_from_slice(&self.gaps[i - 1]);
            }
            t.write(&mut out);
        }
        out.extend_from_slice(&self.trailing);
        out
    }

    /// Finds a mission table by name (exact, as `LoadMissionText` compares).
    pub fn table(&self, name: &str) -> Option<&GxtTable> {
        self.tables.iter().skip(1).find(|t| {
            t.name_raw
                .is_some_and(|n| crate::bytes::c_str(&n) == name.as_bytes())
        })
    }

    /// Structural observations: TABL entries whose offset does not point at a table start,
    /// non-zero gaps, table names differing from their TABL entry.
    pub fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for e in &self.tabl {
            let name = String::from_utf8_lossy(e.name()).into_owned();
            match self.tables.iter().skip(1).find(|t| t.start == e.offset as usize) {
                Some(t) => {
                    if t.name_raw != Some(e.name_raw) {
                        v.push(format!(
                            "TABL {name}: table name field {:?} differs",
                            t.name_lossy()
                        ));
                    }
                }
                None => {
                    let main = &self.tables[0];
                    let mut at = main.start;
                    let mut hit = None;
                    for (i, c) in main.chunks.iter().enumerate() {
                        if at == e.offset as usize {
                            hit = Some(i);
                        }
                        at += 8 + c.size as usize;
                    }
                    match hit {
                        Some(i) => v.push(format!(
                            "TABL {name} -> main table chunk {} ({})",
                            i,
                            String::from_utf8_lossy(&main.chunks[i].fourcc)
                        )),
                        None => v.push(format!(
                            "TABL {name}: offset {:#x} is not a table start",
                            e.offset
                        )),
                    }
                }
            }
        }
        for (i, g) in self.gaps.iter().enumerate() {
            if !g.is_empty() {
                let z = g.iter().all(|&b| b == 0);
                v.push(format!(
                    "{} {}gap bytes before table {}",
                    g.len(),
                    if z { "zero " } else { "NON-ZERO " },
                    self.tables[i + 1].name_lossy()
                ));
            }
        }
        if !self.trailing.is_empty() {
            v.push(format!("{} trailing bytes", self.trailing.len()));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_hash_known_values() {
        // Standard CRC-32 of "123456789" is 0xCBF43926 (final XOR applied).
        assert_eq!(key_hash_case_sensitive(b"123456789") ^ 0xFFFF_FFFF, 0xCBF4_3926);
        assert_eq!(key_hash(b"cderror"), key_hash(b"CDERROR"));
        // Verified against american.gxt (TKEY of MAIN).
        assert_eq!(key_hash(b"CDERROR"), 0xEF12_8EC3);
        assert_eq!(key_hash(b"FEM_MM"), 0x440B_851C);
    }

    fn chunk(out: &mut Vec<u8>, cc: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(cc);
        out.put_u32(data.len() as u32);
        out.extend_from_slice(data);
    }

    fn build(wide: bool) -> Vec<u8> {
        let enc = |s: &str| -> Vec<u8> {
            let mut v = Vec::new();
            for c in s.bytes() {
                v.push(c);
                if wide {
                    v.push(0);
                }
            }
            v.extend_from_slice(if wide { &[0, 0] } else { &[0] });
            v
        };
        let mk_table = |keys: &[(&str, &str)]| -> (Vec<u8>, Vec<u8>) {
            let mut tdat = Vec::new();
            let mut ks: Vec<(u32, u32)> = Vec::new();
            for (k, s) in keys {
                ks.push((tdat.len() as u32, key_hash(k.as_bytes())));
                tdat.extend(enc(s));
            }
            ks.sort_by_key(|k| k.1);
            let mut tkey = Vec::new();
            for (o, h) in ks {
                tkey.put_u32(o);
                tkey.put_u32(h);
            }
            (tkey, tdat)
        };
        let (k0, d0) = mk_table(&[("HELLO", "Hello"), ("BYE", "Bye")]);
        let (k1, d1) = mk_table(&[("M1_A", "Mission text")]);
        let mut out = Vec::new();
        out.put_u16(4);
        out.put_u16(if wide { 16 } else { 8 });
        // TABL with MAIN (-> main TKEY) and one mission table.
        let main_tkey = 4 + 8 + 24;
        let mission = main_tkey + 8 + k0.len() + 8 + d0.len() + 3; // 3 gap bytes
        let mut tabl = Vec::new();
        tabl.extend_from_slice(b"MAIN\0\0\0\0");
        tabl.put_u32(main_tkey as u32);
        tabl.extend_from_slice(b"MIS1\0\0\0\0");
        tabl.put_u32(mission as u32);
        chunk(&mut out, b"TABL", &tabl);
        chunk(&mut out, b"TKEY", &k0);
        chunk(&mut out, b"TDAT", &d0);
        out.extend_from_slice(&[0, 0, 0]);
        out.extend_from_slice(b"MIS1\0\0\0\0");
        chunk(&mut out, b"XTRA", &[]); // size 0: skipped
        chunk(&mut out, b"TKEY", &k1);
        chunk(&mut out, b"TDAT", &d1);
        out.push(0x55);
        out
    }

    #[test]
    fn round_trip_and_lookup_8_and_16_bit() {
        for wide in [false, true] {
            let b = build(wide);
            let g = Gxt::parse(&b).unwrap();
            assert_eq!(g.to_bytes(), b);
            assert_eq!(g.tables.len(), 2);
            assert_eq!(g.tables[0].check(), Ok(2));
            assert_eq!(g.tables[1].check(), Ok(1));
            let hello = g.tables[0].get("hello").unwrap();
            if wide {
                assert_eq!(hello, b"H\0e\0l\0l\0o\0");
            } else {
                assert_eq!(hello, b"Hello");
            }
            assert!(g.table("MIS1").unwrap().get("M1_A").is_some());
            assert_eq!(g.trailing, vec![0x55]);
            let notes = g.notes();
            assert!(
                notes
                    .iter()
                    .any(|n| n.contains("MAIN -> main table chunk 1 (TKEY)")),
                "{notes:?}"
            );
            assert!(notes.iter().any(|n| n.starts_with("3 zero gap")), "{notes:?}");
        }
    }

    #[test]
    fn rejects_bad_char_width() {
        let mut b = vec![];
        b.put_u16(4);
        b.put_u16(12);
        assert!(Gxt::parse(&b).is_err());
    }
}
