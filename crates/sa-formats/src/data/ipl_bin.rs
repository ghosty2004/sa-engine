//! Binary IPL (`bnry`), the streamed `*_streamN.ipl` entries of `gta3.img`/`gta_int.img`.
//!
//! `CIplStore::LoadIpl` (0x406080) treats data starting with `bnry` as `tBinaryIplFile`
//! (0x4C-byte header): six counts (instances, three unused sections, car generators, one more
//! unused section), then six `(offset, size)` pairs in the same order. It reads `count`
//! `CFileObjectInstance` (0x28 bytes) at the instance offset and `count` `CFileCarGenerator`
//! (0x30 bytes) at the car generator offset; offsets are from the start of the data. The size
//! fields are 0 in every file of the install and are not used by the loader.
//!
//! In the install the instances start at 0x4C, the car generators follow them directly, and the
//! rest of the entry is IMG sector padding (zeros). For byte-exact writing every byte not
//! covered by the header and the two arrays is kept as a gap.

use super::{DataFile, dump_rec};
use crate::bytes::{Put, Reader};
use crate::error::{Error, Result};

pub const MAGIC: [u8; 4] = *b"bnry";
pub const HEADER_SIZE: usize = 0x4C;
pub const INST_SIZE: usize = 0x28;
pub const CARGEN_SIZE: usize = 0x30;

/// `CFileObjectInstance`.
#[derive(Debug, Clone, PartialEq)]
pub struct FileInst {
    pub pos: [f32; 3],
    /// Quaternion x, y, z, w.
    pub rot: [f32; 4],
    pub model_id: i32,
    /// Area code in bits 0-7, then flags (redundant stream, underwater, tunnel, tunnel
    /// transition), the rest unused.
    pub instance_type: u32,
    pub lod: i32,
}

/// `CFileCarGenerator`.
#[derive(Debug, Clone, PartialEq)]
pub struct FileCarGen {
    pub pos: [f32; 3],
    pub angle: f32,
    pub model_id: i32,
    pub colour1: i32,
    pub colour2: i32,
    pub flags: u32,
    pub alarm_chance: i32,
    pub door_lock_chance: i32,
    pub min_delay: i32,
    pub max_delay: i32,
}

/// A binary IPL file.
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryIpl {
    /// The 18 header words after the magic: 6 counts, then 6 (offset, size) pairs.
    pub header: [u32; 18],
    pub inst: Vec<FileInst>,
    pub cargens: Vec<FileCarGen>,
    /// Bytes not covered by the header and the two arrays: `(offset, bytes)`.
    pub gaps: Vec<(usize, Vec<u8>)>,
    /// Total input length.
    pub len: usize,
}

impl BinaryIpl {
    pub fn num_inst(&self) -> usize {
        self.header[0] as usize
    }
    pub fn num_cargens(&self) -> usize {
        self.header[4] as usize
    }
    pub fn inst_offset(&self) -> usize {
        self.header[6] as usize
    }
    pub fn cargen_offset(&self) -> usize {
        self.header[14] as usize
    }

    pub fn parse(data: &[u8]) -> Result<BinaryIpl> {
        let mut r = Reader::new(data, "binary IPL header");
        if r.array::<4>()? != MAGIC {
            return Err(Error::invalid("binary IPL", "missing bnry magic"));
        }
        let mut header = [0u32; 18];
        for h in &mut header {
            *h = r.u32()?;
        }
        for (i, name) in [(1, "1"), (2, "2"), (3, "3"), (5, "5")] {
            if header[i] != 0 {
                return Err(Error::unsupported(
                    "binary IPL",
                    format!(
                        "unused section {name} has {} entries (none in the install)",
                        header[i]
                    ),
                ));
            }
        }
        let n_inst = header[0] as usize;
        let n_cg = header[4] as usize;
        let o_inst = header[6] as usize;
        let o_cg = header[14] as usize;
        let mut spans: Vec<(usize, usize)> = vec![(0, HEADER_SIZE)];
        let mut inst = Vec::new();
        if n_inst > 0 {
            let mut r = Reader::new(data.get(o_inst..).unwrap_or(&[]), "binary IPL instances");
            r.check_count(n_inst, INST_SIZE)?;
            for _ in 0..n_inst {
                inst.push(FileInst {
                    pos: r.f32s()?,
                    rot: r.f32s()?,
                    model_id: r.i32()?,
                    instance_type: r.u32()?,
                    lod: r.i32()?,
                });
            }
            spans.push((o_inst, o_inst + n_inst * INST_SIZE));
        }
        let mut cargens = Vec::new();
        if n_cg > 0 {
            let mut r = Reader::new(data.get(o_cg..).unwrap_or(&[]), "binary IPL car generators");
            r.check_count(n_cg, CARGEN_SIZE)?;
            for _ in 0..n_cg {
                cargens.push(FileCarGen {
                    pos: r.f32s()?,
                    angle: r.f32()?,
                    model_id: r.i32()?,
                    colour1: r.i32()?,
                    colour2: r.i32()?,
                    flags: r.u32()?,
                    alarm_chance: r.i32()?,
                    door_lock_chance: r.i32()?,
                    min_delay: r.i32()?,
                    max_delay: r.i32()?,
                });
            }
            spans.push((o_cg, o_cg + n_cg * CARGEN_SIZE));
        }
        spans.sort_unstable();
        if spans.windows(2).any(|w| w[0].1 > w[1].0) {
            return Err(Error::invalid("binary IPL", "header and arrays overlap"));
        }
        let mut gaps = Vec::new();
        let mut pos = 0;
        for (s, e) in spans {
            if s > pos {
                gaps.push((pos, data[pos..s].to_vec()));
            }
            pos = e;
        }
        if pos < data.len() {
            gaps.push((pos, data[pos..].to_vec()));
        }
        Ok(BinaryIpl {
            header,
            inst,
            cargens,
            gaps,
            len: data.len(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![0u8; self.len];
        let mut h = Vec::with_capacity(HEADER_SIZE);
        h.extend_from_slice(&MAGIC);
        for w in self.header {
            h.put_u32(w);
        }
        out[..HEADER_SIZE].copy_from_slice(&h);
        let mut b = Vec::new();
        for i in &self.inst {
            b.put_f32s(&i.pos);
            b.put_f32s(&i.rot);
            b.put_i32(i.model_id);
            b.put_u32(i.instance_type);
            b.put_i32(i.lod);
        }
        let o = self.inst_offset();
        if !b.is_empty() {
            out[o..o + b.len()].copy_from_slice(&b);
        }
        let mut b = Vec::new();
        for c in &self.cargens {
            b.put_f32s(&c.pos);
            b.put_f32(c.angle);
            b.put_i32(c.model_id);
            b.put_i32(c.colour1);
            b.put_i32(c.colour2);
            b.put_u32(c.flags);
            b.put_i32(c.alarm_chance);
            b.put_i32(c.door_lock_chance);
            b.put_i32(c.min_delay);
            b.put_i32(c.max_delay);
        }
        let o = self.cargen_offset();
        if !b.is_empty() {
            out[o..o + b.len()].copy_from_slice(&b);
        }
        for (o, g) in &self.gaps {
            out[*o..*o + g.len()].copy_from_slice(g);
        }
        out
    }
}

impl DataFile for BinaryIpl {
    fn counts(&self) -> Vec<(String, u64)> {
        let mut v = vec![
            ("bnry ipl: files".to_string(), 1),
            ("bnry ipl: instances".into(), self.inst.len() as u64),
            ("bnry ipl: car generators".into(), self.cargens.len() as u64),
        ];
        if self.gaps.iter().any(|(_, g)| g.iter().any(|&b| b != 0)) {
            v.push(("bnry ipl: files with NON-ZERO gap bytes".into(), 1));
        }
        v
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        dump_rec!(&mut o, "header", words = self.header);
        for (i, r) in self.inst.iter().enumerate() {
            dump_rec!(
                &mut o,
                "inst",
                index = i,
                model = r.model_id,
                itype = r.instance_type,
                pos = r.pos,
                rot = r.rot,
                lod = r.lod
            );
        }
        for (i, r) in self.cargens.iter().enumerate() {
            dump_rec!(
                &mut o,
                "cars",
                index = i,
                pos = r.pos,
                angle = r.angle,
                model = r.model_id,
                colour1 = r.colour1,
                colour2 = r.colour2,
                flags = r.flags,
                alarm = r.alarm_chance,
                lock = r.door_lock_chance,
                min_delay = r.min_delay,
                max_delay = r.max_delay
            );
        }
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut d = Vec::new();
        d.extend_from_slice(b"bnry");
        let mut h = [0u32; 18];
        h[0] = 1;
        h[4] = 1;
        h[6] = 0x4C;
        h[14] = 0x4C + 0x28;
        for w in h {
            d.put_u32(w);
        }
        for i in 0..10 {
            d.put_u32(i);
        }
        for i in 0..12 {
            d.put_u32(100 + i);
        }
        d.extend_from_slice(&[0, 0, 7, 0]);
        let ipl = BinaryIpl::parse(&d).unwrap();
        assert_eq!(ipl.inst.len(), 1);
        assert_eq!(ipl.inst[0].model_id, 7);
        assert_eq!(ipl.cargens[0].model_id, 104);
        assert_eq!(ipl.gaps.len(), 1);
        assert_eq!(ipl.to_bytes(), d);
        assert!(BinaryIpl::parse(&d[..0x60]).is_err());
    }
}
