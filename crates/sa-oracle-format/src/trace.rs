//! Hash trace file (`SATRACE`): one record of per-subsystem hashes per frame.
//!
//! ```text
//! magic    8 bytes  "SATRACE\0"
//! version  u32      1
//! count    u32      number of subsystems N
//! names    N x (u8 len, bytes)      subsystem names, in id order
//! records  until EOF: frame u32, tick u32, N x u64
//! ```
//! All integers little-endian. A truncated final record (writer killed mid-write) is ignored.

use crate::Reader;
use crate::subsys::{COUNT, Subsystem};

pub const MAGIC: &[u8; 8] = b"SATRACE\0";
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Frame boundary number (1 = first `CTimer::Update` of the process).
    pub frame: u32,
    /// Input ticks (`CPad::Update(0)` calls) so far.
    pub tick: u32,
    pub hashes: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub names: Vec<String>,
    pub records: Vec<Record>,
}

pub fn header() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(MAGIC);
    v.extend_from_slice(&VERSION.to_le_bytes());
    v.extend_from_slice(&(COUNT as u32).to_le_bytes());
    for s in Subsystem::ALL {
        v.push(s.name().len() as u8);
        v.extend_from_slice(s.name().as_bytes());
    }
    v
}

pub fn encode_record(frame: u32, tick: u32, hashes: &[u64; COUNT]) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 + COUNT * 8);
    v.extend_from_slice(&frame.to_le_bytes());
    v.extend_from_slice(&tick.to_le_bytes());
    for h in hashes {
        v.extend_from_slice(&h.to_le_bytes());
    }
    v
}

impl Trace {
    pub fn parse(buf: &[u8]) -> Result<Trace, String> {
        let mut r = Reader::new(buf);
        if r.bytes(8)? != MAGIC {
            return Err("not a SATRACE file".into());
        }
        let ver = r.u32()?;
        if ver != VERSION {
            return Err(format!("unsupported trace version {ver}"));
        }
        let n = r.u32()? as usize;
        if n == 0 || n > 4096 {
            return Err(format!("bad subsystem count {n}"));
        }
        let mut names = Vec::with_capacity(n);
        for _ in 0..n {
            let len = r.u8()? as usize;
            names.push(String::from_utf8_lossy(r.bytes(len)?).into_owned());
        }
        let rec_len = 8 + n * 8;
        let mut records = Vec::new();
        while buf.len() - r.pos >= rec_len {
            let frame = r.u32()?;
            let tick = r.u32()?;
            let hashes = (0..n).map(|_| r.u64()).collect::<Result<Vec<_>, _>>()?;
            records.push(Record { frame, tick, hashes });
        }
        Ok(Trace { names, records })
    }

    pub fn subsystem_index(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }
}

/// Where two traces first differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    pub frame: u32,
    /// Names of the subsystems that differ at `frame`.
    pub subsystems: Vec<String>,
}

/// Result of comparing two traces frame by frame over their common frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub common_frames: usize,
    pub first: Option<Divergence>,
    /// Per subsystem (in name order of `a`): first frame where it differs, if any.
    pub per_subsystem: Vec<(String, Option<u32>)>,
    /// Frames present in only one trace (by frame number).
    pub only_a: usize,
    pub only_b: usize,
}

/// Compares two traces. Subsystems listed in `ignore` are skipped (diagnostics).
pub fn compare(a: &Trace, b: &Trace, ignore: &[&str]) -> Result<Comparison, String> {
    if a.names != b.names {
        return Err(format!("subsystem lists differ: {:?} vs {:?}", a.names, b.names));
    }
    let active: Vec<usize> = (0..a.names.len())
        .filter(|&i| !ignore.contains(&a.names[i].as_str()))
        .collect();
    let bmap: std::collections::HashMap<u32, &Record> = b.records.iter().map(|r| (r.frame, r)).collect();
    let mut per: Vec<Option<u32>> = vec![None; a.names.len()];
    let mut first: Option<Divergence> = None;
    let mut common = 0;
    for ra in &a.records {
        let Some(rb) = bmap.get(&ra.frame) else { continue };
        common += 1;
        let mut diff = Vec::new();
        for &i in &active {
            if ra.hashes[i] != rb.hashes[i] {
                diff.push(a.names[i].clone());
                if per[i].is_none() {
                    per[i] = Some(ra.frame);
                }
            }
        }
        if first.is_none() && !diff.is_empty() {
            first = Some(Divergence {
                frame: ra.frame,
                subsystems: diff,
            });
        }
    }
    let afr: std::collections::HashSet<u32> = a.records.iter().map(|r| r.frame).collect();
    Ok(Comparison {
        common_frames: common,
        first,
        per_subsystem: a.names.iter().cloned().zip(per).collect(),
        only_a: a.records.len() - common,
        only_b: b.records.iter().filter(|r| !afr.contains(&r.frame)).count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(frames: &[(u32, u64)]) -> Vec<u8> {
        let mut v = header();
        for &(f, x) in frames {
            let mut h = [7u64; COUNT];
            h[Subsystem::Peds as usize] = x;
            v.extend(encode_record(f, f * 2, &h));
        }
        v
    }

    #[test]
    fn round_trip_and_compare() {
        let a = Trace::parse(&trace(&[(1, 1), (2, 2), (3, 3)])).unwrap();
        assert_eq!(a.records.len(), 3);
        assert_eq!(a.records[1].tick, 4);
        assert_eq!(a.subsystem_index("peds"), Some(Subsystem::Peds as usize));
        let mut bytes = trace(&[(1, 1), (2, 9), (3, 3), (4, 4)]);
        bytes.extend_from_slice(&[1, 2, 3]); // truncated tail is ignored
        let b = Trace::parse(&bytes).unwrap();
        let c = compare(&a, &b, &[]).unwrap();
        assert_eq!(c.common_frames, 3);
        assert_eq!((c.only_a, c.only_b), (0, 1));
        assert_eq!(
            c.first,
            Some(Divergence {
                frame: 2,
                subsystems: vec!["peds".into()]
            })
        );
        let peds = c.per_subsystem.iter().find(|(n, _)| n == "peds").unwrap();
        assert_eq!(peds.1, Some(2));
        let c = compare(&a, &b, &["peds"]).unwrap();
        assert_eq!(c.first, None);
        assert!(Trace::parse(b"SATRACE\0\x02\0\0\0").is_err());
    }
}
