//! SCM scripts: `data/script/main.scm` (header segments, main code, missions) and the streamed
//! scripts in `data/script/script.img`, plus an instruction decoder ([`decode`]).
//!
//! # main.scm header (1.0 US loaders)
//! `CTheScripts::Init` 0x468D50 reads the whole file into `ScriptSpace` (0xA49960). The header
//! is a chain of segments, each starting with `GOTO <int32 next>` (`02 00 01 <i32>`) and a
//! segment id byte, so a running script skips it:
//!
//! | id | content | reader |
//! |---|---|---|
//! | `'s'` | global variable space (to the next segment) | variables are addressed by offset |
//! | 0 | `u32 count`, `count` × `char[24]` model names | `ReadObjectNamesFromScript` 0x486720 (body 0x156F5A0): reads only the **low `u16`** of the count; segment found at `[ScriptSpace + 3]` |
//! | 1 | `u32 main size`, `u32 largest mission`, `u16 missions`, `u16 exclusive missions`, `u32 largest mission locals`, `u32 offsets[missions]` | `ReadMultiScriptFileOffsetsFromScript` 0x4867C0 (body 0x1565E20); segment at `[seg0 + 3]` |
//! | 2 | `u32 largest streamed size`, `u32 count`, `count` × { `char name[20]`, `u32 offset`, `u32 size` } | `CStreamedScripts::ReadStreamedScriptData` 0x470750 (body 0x156F450): skips the count, iterates over the scripts registered from `script.img` instead and takes only the size of each entry (looked up by name, `_stricmp`) |
//! | 3 | `u32` (0 in the install) | not read |
//! | 4 | `u32 global variable space size`, `u32` (569 in the install) | not read |
//!
//! Every segment keeps the bytes after its known fields (`rest`), so the header round-trips.
//! Code starts at the last segment's `next` offset.

pub mod decode;
mod opcodes;

use crate::bytes::{Put, Reader};
use crate::error::{Error, Result};
pub use decode::{DecodeReport, Instruction, Param, ParamValue, ScriptKind, ScriptSet, usage_csv};

/// A header segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The 3 bytes before the jump target (`02 00 01`).
    pub goto: [u8; 3],
    /// Jump target: file offset of the next segment (or of the code).
    pub next: i32,
    /// Segment id byte.
    pub id: u8,
    pub body: SegmentBody,
    /// Bytes after the known fields up to `next`.
    pub rest: Vec<u8>,
}

/// Typed segment contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentBody {
    /// `'s'`: the global variable space (kept in `rest`).
    Globals,
    /// 0: model names used by the script.
    Models { count: u32, names: Vec<[u8; 24]> },
    /// 1: main script size, mission table.
    Missions {
        main_size: u32,
        largest_mission_size: u32,
        num_missions: u16,
        num_exclusive_missions: u16,
        largest_mission_locals: u32,
        offsets: Vec<u32>,
    },
    /// 2: streamed script table.
    Streamed {
        largest_size: u32,
        count: u32,
        entries: Vec<StreamedEntry>,
    },
    /// 3 or 4 or anything else: kept raw in `rest`.
    Raw,
}

/// One streamed script table entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamedEntry {
    pub name_raw: [u8; 20],
    /// Offset in the compiler's address space (main.scm size + running sum; not used by the exe).
    pub offset: u32,
    /// Size in bytes (the only field the exe takes).
    pub size: u32,
}

impl StreamedEntry {
    pub fn name(&self) -> &[u8] {
        crate::bytes::c_str(&self.name_raw)
    }
}

/// A parsed `main.scm`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainScm {
    pub segments: Vec<Segment>,
    /// File offset of the first code byte.
    pub code_start: usize,
    /// Everything from `code_start` to the end (main code followed by the missions).
    pub code: Vec<u8>,
}

impl MainScm {
    /// Parses the header chain: segments are read while the bytes at the current offset are a
    /// `GOTO` with an `int32` operand pointing forward.
    pub fn parse(data: &[u8]) -> Result<MainScm> {
        let mut segments = Vec::new();
        let mut pos = 0usize;
        while let Some(h) = data.get(pos..pos + 8) {
            if h[..3] != [0x02, 0x00, 0x01] {
                break;
            }
            let next = i32::from_le_bytes([h[3], h[4], h[5], h[6]]);
            let id = h[7];
            if next as usize <= pos + 8 || next as usize > data.len() || next < 0 {
                return Err(Error::invalid(
                    "SCM header",
                    format!("segment at {pos:#x} jumps to {next:#x}"),
                ));
            }
            let body_bytes = &data[pos + 8..next as usize];
            let mut r = Reader::new(body_bytes, "SCM header segment");
            let body = match (segments.len(), id) {
                (_, b's') => SegmentBody::Globals,
                (1, 0) => {
                    let count = r.u32()?;
                    let n = (count & 0xFFFF) as usize;
                    r.check_count(n, 24)?;
                    let names = (0..n).map(|_| r.array()).collect::<Result<_>>()?;
                    SegmentBody::Models { count, names }
                }
                (2, 1) => {
                    let main_size = r.u32()?;
                    let largest_mission_size = r.u32()?;
                    let num_missions = r.u16()?;
                    let num_exclusive_missions = r.u16()?;
                    let largest_mission_locals = r.u32()?;
                    r.check_count(num_missions as usize, 4)?;
                    let offsets = (0..num_missions).map(|_| r.u32()).collect::<Result<_>>()?;
                    SegmentBody::Missions {
                        main_size,
                        largest_mission_size,
                        num_missions,
                        num_exclusive_missions,
                        largest_mission_locals,
                        offsets,
                    }
                }
                (3, 2) => {
                    let largest_size = r.u32()?;
                    let count = r.u32()?;
                    r.check_count(count as usize, 28)?;
                    let mut entries = Vec::with_capacity(count as usize);
                    for _ in 0..count {
                        entries.push(StreamedEntry {
                            name_raw: r.array()?,
                            offset: r.u32()?,
                            size: r.u32()?,
                        });
                    }
                    SegmentBody::Streamed {
                        largest_size,
                        count,
                        entries,
                    }
                }
                _ => SegmentBody::Raw,
            };
            let rest = r.rest().to_vec();
            segments.push(Segment {
                goto: [h[0], h[1], h[2]],
                next,
                id,
                body,
                rest,
            });
            pos = next as usize;
        }
        if segments.len() < 4 {
            return Err(Error::invalid(
                "SCM header",
                format!("only {} header segments", segments.len()),
            ));
        }
        Ok(MainScm {
            segments,
            code_start: pos,
            code: data[pos..].to_vec(),
        })
    }

    /// Serialises back to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.code_start + self.code.len());
        for s in &self.segments {
            out.extend_from_slice(&s.goto);
            out.put_i32(s.next);
            out.push(s.id);
            match &s.body {
                SegmentBody::Globals | SegmentBody::Raw => {}
                SegmentBody::Models { count, names } => {
                    out.put_u32(*count);
                    for n in names {
                        out.extend_from_slice(n);
                    }
                }
                SegmentBody::Missions {
                    main_size,
                    largest_mission_size,
                    num_missions,
                    num_exclusive_missions,
                    largest_mission_locals,
                    offsets,
                } => {
                    out.put_u32(*main_size);
                    out.put_u32(*largest_mission_size);
                    out.put_u16(*num_missions);
                    out.put_u16(*num_exclusive_missions);
                    out.put_u32(*largest_mission_locals);
                    for o in offsets {
                        out.put_u32(*o);
                    }
                }
                SegmentBody::Streamed {
                    largest_size,
                    count,
                    entries,
                } => {
                    out.put_u32(*largest_size);
                    out.put_u32(*count);
                    for e in entries {
                        out.extend_from_slice(&e.name_raw);
                        out.put_u32(e.offset);
                        out.put_u32(e.size);
                    }
                }
            }
            out.extend_from_slice(&s.rest);
        }
        out.extend_from_slice(&self.code);
        out
    }

    /// The whole file as one buffer (offsets are file offsets).
    pub fn bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    /// The mission segment (id 1).
    pub fn missions(&self) -> Option<(u32, &[u32])> {
        self.segments.iter().find_map(|s| match &s.body {
            SegmentBody::Missions {
                main_size, offsets, ..
            } => Some((*main_size, offsets.as_slice())),
            _ => None,
        })
    }

    /// The streamed script table (id 2).
    pub fn streamed(&self) -> &[StreamedEntry] {
        self.segments
            .iter()
            .find_map(|s| match &s.body {
                SegmentBody::Streamed { entries, .. } => Some(entries.as_slice()),
                _ => None,
            })
            .unwrap_or(&[])
    }

    /// Model names (id 0), up to their NUL.
    pub fn models(&self) -> Vec<&[u8]> {
        self.segments
            .iter()
            .find_map(|s| match &s.body {
                SegmentBody::Models { names, .. } => {
                    Some(names.iter().map(|n| crate::bytes::c_str(n)).collect())
                }
                _ => None,
            })
            .unwrap_or_default()
    }

    /// Size of the global variable space segment (its body).
    pub fn global_space_size(&self) -> usize {
        self.segments
            .iter()
            .find(|s| matches!(s.body, SegmentBody::Globals))
            .map_or(0, |s| s.rest.len())
    }

    /// Header statistics.
    pub fn counts(&self) -> Vec<(String, u64)> {
        let mut v = vec![
            ("header segments".to_string(), self.segments.len() as u64),
            (
                "global variable space bytes".into(),
                self.global_space_size() as u64,
            ),
            ("model names".into(), self.models().len() as u64),
            (
                "streamed script table entries".into(),
                self.streamed().len() as u64,
            ),
            ("code start offset".into(), self.code_start as u64),
        ];
        if let Some((main_size, offs)) = self.missions() {
            v.push(("main script size".into(), u64::from(main_size)));
            v.push(("missions".into(), offs.len() as u64));
        }
        for s in &self.segments {
            if !s.rest.is_empty() && !matches!(s.body, SegmentBody::Globals) {
                v.push((format!("segment id {} trailing bytes", s.id), s.rest.len() as u64));
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn synthetic_main(code: &[u8], mission: &[u8], streamed: &[(&str, u32)]) -> Vec<u8> {
        // segment 's' (8 bytes of globals), 0 (2 models), 1 (1 mission), 2 (streamed), 3, 4
        let mut segs: Vec<(u8, Vec<u8>)> = Vec::new();
        segs.push((b's', vec![0; 8]));
        let mut m = Vec::new();
        m.put_u32(2);
        m.extend_from_slice(&[0; 24]);
        let mut n = [0u8; 24];
        n[..4].copy_from_slice(b"INFO");
        m.extend_from_slice(&n);
        segs.push((0, m));
        segs.push((1, Vec::new())); // filled below
        let mut st = Vec::new();
        st.put_u32(streamed.iter().map(|s| s.1).max().unwrap_or(0));
        st.put_u32(streamed.len() as u32);
        for (name, size) in streamed {
            let mut nm = [0u8; 20];
            nm[..name.len()].copy_from_slice(name.as_bytes());
            st.extend_from_slice(&nm);
            st.put_u32(0);
            st.put_u32(*size);
        }
        segs.push((2, st));
        segs.push((3, vec![0; 4]));
        segs.push((4, vec![8, 0, 0, 0, 0x39, 2, 0, 0]));
        let mut header_len: usize = segs.iter().map(|s| 8 + s.1.len()).sum();
        header_len += 4 * 4 + 4; // mission segment body
        let main_size = header_len + code.len();
        let mut ms = Vec::new();
        ms.put_u32(main_size as u32);
        ms.put_u32(mission.len() as u32);
        ms.put_u16(1);
        ms.put_u16(0);
        ms.put_u32(4);
        ms.put_u32(main_size as u32);
        segs[2].1 = ms;
        let mut out = Vec::new();
        for (id, body) in &segs {
            let next = out.len() + 8 + body.len();
            out.extend_from_slice(&[2, 0, 1]);
            out.put_i32(next as i32);
            out.push(*id);
            out.extend_from_slice(body);
        }
        assert_eq!(out.len(), header_len);
        out.extend_from_slice(code);
        out.extend_from_slice(mission);
        out
    }

    #[test]
    fn header_round_trip() {
        let code = [0x4E, 0x00]; // TERMINATE_THIS_SCRIPT
        let b = synthetic_main(&code, &[0x4E, 0x00], &[("TEST", 2)]);
        let m = MainScm::parse(&b).unwrap();
        assert_eq!(m.to_bytes(), b);
        assert_eq!(m.segments.len(), 6);
        assert_eq!(m.models(), vec![&b""[..], &b"INFO"[..]]);
        assert_eq!(m.streamed()[0].name(), b"TEST");
        assert_eq!(m.global_space_size(), 8);
        let (main_size, offs) = m.missions().unwrap();
        assert_eq!(offs, &[main_size]);
        assert_eq!(m.code_start + 2, main_size as usize);
    }
}
