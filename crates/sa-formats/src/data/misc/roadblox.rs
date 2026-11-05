//! `data/paths/roadblox.dat`, as `CRoadBlocks::Init` (0x461100, a `jmp` stub to the body at
//! 0x1568D90 in `.HOODLUM`) reads it.
//!
//! Opened `"rb"`; `CFileMgr::Read(f, &NumRoadBlocks, 4)`, then `CFileMgr::Read(f,
//! RoadBlockNodes, 0x514)`: a fixed array of 325 `CNodeAddress` (`u16` area, `u16` node),
//! independent of the count. The 1.0 US file is exactly 4 + 0x514 bytes.

use super::counts;
use crate::bytes::{Put, Reader};
use crate::data::{DataFile, dump_rec};
use crate::error::Result;

/// Entries read by the exe.
pub const NUM_NODES: usize = 0x514 / 4;

/// A parsed `roadblox.dat`.
#[derive(Debug, Clone, PartialEq)]
pub struct RoadBlox {
    pub count: i32,
    /// All 325 node addresses (area, node), including those past `count`.
    pub nodes: Vec<(u16, u16)>,
    /// Bytes after the array (not read).
    pub trailing: Vec<u8>,
}

impl RoadBlox {
    pub fn parse(data: &[u8]) -> Result<RoadBlox> {
        let mut r = Reader::new(data, "roadblox.dat");
        let count = r.i32()?;
        r.check_count(NUM_NODES, 4)?;
        let mut nodes = Vec::with_capacity(NUM_NODES);
        for _ in 0..NUM_NODES {
            nodes.push((r.u16()?, r.u16()?));
        }
        Ok(RoadBlox {
            count,
            nodes,
            trailing: r.rest().to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::new();
        o.put_i32(self.count);
        for (a, n) in &self.nodes {
            o.put_u16(*a);
            o.put_u16(*n);
        }
        o.extend_from_slice(&self.trailing);
        o
    }
}

impl DataFile for RoadBlox {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![(
            "roadblox.dat: road blocks".into(),
            self.count.max(0) as u64,
        )])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        dump_rec!(&mut o, "count", n = self.count);
        for (i, (a, n)) in self.nodes.iter().enumerate() {
            dump_rec!(&mut o, "node", index = i, area = *a, node = *n);
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.count < 0 || self.count as usize > NUM_NODES {
            v.push(format!("count {} outside 0..=325", self.count));
        }
        if !self.trailing.is_empty() {
            v.push(format!(
                "{} bytes after the node array are not read",
                self.trailing.len()
            ));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut d = vec![2, 0, 0, 0];
        d.extend((0..0x514).map(|i| i as u8));
        let r = RoadBlox::parse(&d).unwrap();
        assert_eq!(r.nodes.len(), 325);
        assert_eq!(r.nodes[0], (0x0100, 0x0302));
        assert_eq!(r.to_bytes(), d);
        assert!(RoadBlox::parse(&d[..100]).is_err());
    }
}
