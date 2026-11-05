//! Path node files `nodesN.dat` (N = 0..63, one per 750x750 area), as
//! `CPathFind::LoadPathFindData(RwStream*, int32)` (0x4529F0) reads them from `gta3.img`.
//!
//! Layout (all little-endian), in read order:
//! 1. five `u32`: node count, vehicle node count, ped node count, navi node (`CCarPathLink`)
//!    count, link (`CNodeAddress`) count `L`;
//! 2. node count x `CPathNode` (0x1C bytes);
//! 3. navi count x `CCarPathLink` (0x0E bytes);
//! 4. when `L > 0`: `(L + 192)` x `CNodeAddress` (4 bytes), `L` x `CCarPathLinkAddress`
//!    (2 bytes), `(L + 192)` link lengths (1 byte), `(L + 192)` intersection flags (1 byte).
//!    The 192 extra entries (`NUM_DYNAMIC_LINKS_PER_AREA` 16 x 12) are the dynamic link slots;
//!    the files store them too.
//!
//! The loose copies in `data/Paths/NODES*.DAT` hold the same bytes without the IMG sector
//! padding; the exe's only reference to `data\paths\nodes%d.dat` is in
//! `CPathFind::LoadPathFindData(int32)` (0x452F40), which has no callers, so they are not loaded.

use super::{DataFile, dump_rec};
use crate::bytes::{Put, Reader};
use crate::error::{Error, Result};

/// Extra link slots read after the file's `L` links (`NUM_DYNAMIC_LINKS_PER_AREA * 12`).
pub const DYNAMIC_LINK_SLOTS: usize = 16 * 12;

/// `CPathNode` as stored (0x1C bytes). Bit fields are kept as the raw word.
#[derive(Debug, Clone, PartialEq)]
pub struct PathNode {
    /// `m_next`, `m_prev` (pointers, meaningless in the file).
    pub next: u32,
    pub prev: u32,
    /// Position in 1/8 units.
    pub pos: [i16; 3],
    pub base_link: i16,
    pub area: u16,
    pub node: u16,
    /// Path width in 1/16 units.
    pub width: u8,
    pub flood_fill: u8,
    /// Alignment bytes before the flag word.
    pub pad: [u8; 2],
    /// Bits 0-3 link count, 4 dead end, 5 switched off, 6 road blocks, 7 water, 8 switched off
    /// (original), 9 unused, 10 don't wander, 11 unused, 12 not highway, 13 highway, 14-15
    /// unused, 16-19 spawn probability, 20-23 behaviour type, 24-31 unused.
    pub flags: u32,
}

/// `CCarPathLink` ("navi node", 0x0E bytes).
#[derive(Debug, Clone, PartialEq)]
pub struct NaviNode {
    /// Position in 1/8 units.
    pub pos: [i16; 2],
    pub attached_area: u16,
    pub attached_node: u16,
    /// Direction in 1/100 units.
    pub dir: [i8; 2],
    pub width: i8,
    /// Bits 0-2 opposite-direction lanes, 3-5 same-direction lanes, 6 traffic light direction.
    pub lanes: u8,
    /// Bits 0-1 traffic light state, 2 bridge lights.
    pub flags: u16,
}

/// A path node file.
#[derive(Debug, Clone, PartialEq)]
pub struct PathNodes {
    pub num_nodes: u32,
    pub num_vehicle_nodes: u32,
    pub num_ped_nodes: u32,
    pub num_navi: u32,
    pub num_links: u32,
    pub nodes: Vec<PathNode>,
    pub navi: Vec<NaviNode>,
    /// `CNodeAddress` (area, node): `L + 192` entries when `L > 0`.
    pub links: Vec<(u16, u16)>,
    /// `CCarPathLinkAddress` raw words (bits 0-9 navi id, 10-15 area): `L` entries.
    pub navi_links: Vec<u16>,
    pub link_lengths: Vec<u8>,
    /// `CPathIntersectionInfo` raw bytes (bit 0 road cross, bit 1 ped traffic light).
    pub intersections: Vec<u8>,
    /// Bytes after the last array (IMG sector padding).
    pub trailing: Vec<u8>,
}

impl PathNodes {
    pub fn parse(data: &[u8]) -> Result<PathNodes> {
        let mut r = Reader::new(data, "nodes header");
        let num_nodes = r.u32()?;
        let num_vehicle_nodes = r.u32()?;
        let num_ped_nodes = r.u32()?;
        let num_navi = r.u32()?;
        let num_links = r.u32()?;
        r.set_context("nodes: path nodes");
        r.check_count(num_nodes as usize, 0x1C)?;
        let mut nodes = Vec::with_capacity(num_nodes as usize);
        for _ in 0..num_nodes {
            nodes.push(PathNode {
                next: r.u32()?,
                prev: r.u32()?,
                pos: [r.i16()?, r.i16()?, r.i16()?],
                base_link: r.i16()?,
                area: r.u16()?,
                node: r.u16()?,
                width: r.u8()?,
                flood_fill: r.u8()?,
                pad: r.array()?,
                flags: r.u32()?,
            });
        }
        r.set_context("nodes: navi nodes");
        r.check_count(num_navi as usize, 0x0E)?;
        let mut navi = Vec::with_capacity(num_navi as usize);
        for _ in 0..num_navi {
            let pos = [r.i16()?, r.i16()?];
            let attached_area = r.u16()?;
            let attached_node = r.u16()?;
            let d = r.array::<4>()?;
            navi.push(NaviNode {
                pos,
                attached_area,
                attached_node,
                dir: [d[0] as i8, d[1] as i8],
                width: d[2] as i8,
                lanes: d[3],
                flags: r.u16()?,
            });
        }
        let (mut links, mut navi_links, mut link_lengths, mut intersections) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        if num_links > 0 {
            let l = num_links as usize;
            let ld = l + DYNAMIC_LINK_SLOTS;
            r.set_context("nodes: links");
            r.check_count(ld, 4)?;
            for _ in 0..ld {
                links.push((r.u16()?, r.u16()?));
            }
            r.set_context("nodes: navi links");
            r.check_count(l, 2)?;
            for _ in 0..l {
                navi_links.push(r.u16()?);
            }
            r.set_context("nodes: link lengths");
            link_lengths = r.bytes(ld)?.to_vec();
            r.set_context("nodes: intersections");
            intersections = r.bytes(ld)?.to_vec();
        }
        let trailing = r.rest().to_vec();
        if num_nodes != num_vehicle_nodes.wrapping_add(num_ped_nodes) {
            return Err(Error::invalid(
                "nodes header",
                format!("{num_nodes} nodes != {num_vehicle_nodes} vehicle + {num_ped_nodes} ped"),
            ));
        }
        Ok(PathNodes {
            num_nodes,
            num_vehicle_nodes,
            num_ped_nodes,
            num_navi,
            num_links,
            nodes,
            navi,
            links,
            navi_links,
            link_lengths,
            intersections,
            trailing,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut o = Vec::new();
        for v in [
            self.num_nodes,
            self.num_vehicle_nodes,
            self.num_ped_nodes,
            self.num_navi,
            self.num_links,
        ] {
            o.put_u32(v);
        }
        for n in &self.nodes {
            o.put_u32(n.next);
            o.put_u32(n.prev);
            for p in n.pos {
                o.put_i16(p);
            }
            o.put_i16(n.base_link);
            o.put_u16(n.area);
            o.put_u16(n.node);
            o.put_u8(n.width);
            o.put_u8(n.flood_fill);
            o.extend_from_slice(&n.pad);
            o.put_u32(n.flags);
        }
        for n in &self.navi {
            o.put_i16(n.pos[0]);
            o.put_i16(n.pos[1]);
            o.put_u16(n.attached_area);
            o.put_u16(n.attached_node);
            o.extend_from_slice(&[n.dir[0] as u8, n.dir[1] as u8, n.width as u8, n.lanes]);
            o.put_u16(n.flags);
        }
        for (a, n) in &self.links {
            o.put_u16(*a);
            o.put_u16(*n);
        }
        for v in &self.navi_links {
            o.put_u16(*v);
        }
        o.extend_from_slice(&self.link_lengths);
        o.extend_from_slice(&self.intersections);
        o.extend_from_slice(&self.trailing);
        o
    }
}

impl DataFile for PathNodes {
    fn counts(&self) -> Vec<(String, u64)> {
        let mut v = vec![
            ("nodes: files".to_string(), 1),
            ("nodes: path nodes".into(), self.nodes.len() as u64),
            ("nodes: vehicle nodes".into(), self.num_vehicle_nodes as u64),
            ("nodes: ped nodes".into(), self.num_ped_nodes as u64),
            ("nodes: navi nodes".into(), self.navi.len() as u64),
            ("nodes: links".into(), self.num_links as u64),
        ];
        if self.trailing.iter().any(|&b| b != 0) {
            v.push(("nodes: files with NON-ZERO trailing bytes".into(), 1));
        }
        if self.links[self.num_links as usize..]
            .iter()
            .any(|&(a, n)| a != 0 || n != 0)
        {
            v.push(("nodes: files with non-zero dynamic link slots".into(), 1));
        }
        if self.nodes.iter().any(|n| n.pad != [0, 0]) {
            v.push(("nodes: files with non-zero node pad bytes".into(), 1));
        }
        v
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        dump_rec!(
            &mut o,
            "header",
            nodes = self.num_nodes,
            vehicle = self.num_vehicle_nodes,
            ped = self.num_ped_nodes,
            navi = self.num_navi,
            links = self.num_links
        );
        for (i, n) in self.nodes.iter().enumerate() {
            dump_rec!(
                &mut o,
                "node",
                index = i,
                pos = n.pos,
                base_link = n.base_link,
                area = n.area,
                node = n.node,
                width = n.width,
                flood = n.flood_fill,
                flags = n.flags
            );
        }
        for (i, n) in self.navi.iter().enumerate() {
            dump_rec!(
                &mut o,
                "navi",
                index = i,
                pos = n.pos,
                area = n.attached_area,
                node = n.attached_node,
                dir = n.dir,
                width = n.width,
                lanes = n.lanes,
                flags = n.flags
            );
        }
        for (i, (a, n)) in self.links.iter().enumerate() {
            dump_rec!(
                &mut o,
                "link",
                index = i,
                area = *a,
                node = *n,
                navi_link = self.navi_links.get(i).copied(),
                length = self.link_lengths[i],
                intersection = self.intersections[i]
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
        for v in [1u32, 1, 0, 1, 1] {
            d.put_u32(v);
        }
        d.extend((0..0x1C).map(|i| i as u8));
        d.extend((0..0x0E).map(|i| 0x80 + i as u8));
        d.extend(std::iter::repeat_n(0xAA, 4 * (1 + DYNAMIC_LINK_SLOTS)));
        d.extend([1, 2]);
        d.extend(std::iter::repeat_n(3, 2 * (1 + DYNAMIC_LINK_SLOTS)));
        d.extend([0; 5]);
        let n = PathNodes::parse(&d).unwrap();
        assert_eq!(n.links.len(), 1 + DYNAMIC_LINK_SLOTS);
        assert_eq!(n.navi[0].dir, [-0x80 + 8, -0x80 + 9]);
        assert_eq!(n.trailing.len(), 5);
        assert_eq!(n.to_bytes(), d);
        assert!(PathNodes::parse(&d[..100]).is_err());
    }
}
