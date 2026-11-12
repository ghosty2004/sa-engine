//! Input recording (`SAREC`), version 1.
//!
//! ```text
//! magic        8 bytes "SAREC\0\0\0"
//! version      u32     1
//! header_len   u32
//! header       header_len bytes of UTF-8 "key=value\n" lines (start conditions, see `Header`)
//! tick_count   u32
//! ticks        tick_count x { frame u32, kind u8, [PadInput (788 bytes) if kind == 1] }
//!                 kind 0 = same input as the previous tick, kind 1 = full input follows
//! ```
//! All integers little-endian. A tick is one `CPad::UpdatePads` pass (see [`crate::pad`]); `frame` is
//! the frame-boundary count when the tick ran (informational: replay consumes ticks in order and
//! checks that the frame matches).

use std::collections::BTreeMap;

use crate::Reader;
use crate::pad::{PAD_INPUT_LEN, PadInput};

pub const MAGIC: &[u8; 8] = b"SAREC\0\0\0";
pub const VERSION: u32 = 1;

/// Start conditions and settings. Stored as ordered `key=value` lines so new keys never break old
/// readers. Well-known keys:
///
/// | key | meaning |
/// |---|---|
/// | `start` | `boot` (ticks start at process start; menu navigation is part of the input) or `midrun:FRAME` |
/// | `det` | `1` if the recording ran in determinism mode |
/// | `det.step_us` | virtual frame step in microseconds |
/// | `det.clock_base` | virtual clock base (the determinism seed: `srand(timeGetTime())` at GameInit sees it) |
/// | `det.qpf` | QueryPerformanceFrequency seen by the game |
/// | `exe` | `fnv64:size` of gta_sa.exe |
/// | `data.PATH` | `fnv64:size` of a game data file (relative path) |
/// | `settings` | `fnv64:size` of gta_sa.set |
/// | `program` | `fnv64` of the input program that generated the input (scripted recordings) |
/// | `pokes` | harness pokes of the program (`program::encode_pokes`), applied at frame boundaries on replay |
/// | `frames` | last frame boundary seen while recording |
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Header {
    pub fields: BTreeMap<String, String>,
}

impl Header {
    pub fn set(&mut self, k: &str, v: impl ToString) {
        self.fields.insert(k.to_string(), v.to_string());
    }
    pub fn get(&self, k: &str) -> Option<&str> {
        self.fields.get(k).map(|s| s.as_str())
    }
    fn encode(&self) -> String {
        let mut s = String::new();
        for (k, v) in &self.fields {
            s += &format!("{k}={v}\n");
        }
        s
    }
    fn decode(s: &str) -> Result<Header, String> {
        let mut h = Header::default();
        for line in s.lines().filter(|l| !l.is_empty()) {
            let (k, v) = line
                .split_once('=')
                .ok_or_else(|| format!("bad header line {line:?}"))?;
            h.set(k, v);
        }
        Ok(h)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tick {
    pub frame: u32,
    pub input: PadInput,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recording {
    pub header: Header,
    pub ticks: Vec<Tick>,
}

impl Recording {
    pub fn encode(&self) -> Vec<u8> {
        let hdr = self.header.encode();
        let mut v = Vec::new();
        v.extend_from_slice(MAGIC);
        v.extend_from_slice(&VERSION.to_le_bytes());
        v.extend_from_slice(&(hdr.len() as u32).to_le_bytes());
        v.extend_from_slice(hdr.as_bytes());
        v.extend_from_slice(&(self.ticks.len() as u32).to_le_bytes());
        let mut prev: Option<&PadInput> = None;
        for t in &self.ticks {
            v.extend_from_slice(&t.frame.to_le_bytes());
            if prev == Some(&t.input) {
                v.push(0);
            } else {
                v.push(1);
                v.extend_from_slice(&t.input.to_bytes());
            }
            prev = Some(&t.input);
        }
        v
    }

    pub fn parse(buf: &[u8]) -> Result<Recording, String> {
        let mut r = Reader::new(buf);
        if r.bytes(8)? != MAGIC {
            return Err("not a SAREC file".into());
        }
        let ver = r.u32()?;
        if ver != VERSION {
            return Err(format!("unsupported recording version {ver}"));
        }
        let hl = r.u32()? as usize;
        let header = Header::decode(std::str::from_utf8(r.bytes(hl)?).map_err(|e| e.to_string())?)?;
        let n = r.u32()? as usize;
        let mut ticks = Vec::with_capacity(n.min(1 << 20));
        let mut prev: Option<PadInput> = None;
        for i in 0..n {
            let frame = r.u32()?;
            let input = match r.u8()? {
                0 => prev
                    .clone()
                    .ok_or_else(|| format!("tick {i}: repeat without a previous input"))?,
                1 => PadInput::from_bytes(r.bytes(PAD_INPUT_LEN)?).expect("length checked"),
                k => return Err(format!("tick {i}: bad kind {k}")),
            };
            prev = Some(input.clone());
            ticks.push(Tick { frame, input });
        }
        if !r.at_end() {
            return Err(format!("{} trailing bytes", buf.len() - r.pos));
        }
        Ok(Recording { header, ticks })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pad::ctrl;

    #[test]
    fn round_trip_with_repeats() {
        let mut rec = Recording::default();
        rec.header.set("start", "boot");
        rec.header.set("det.step_us", 40000);
        let a = PadInput::default();
        let mut b = PadInput::default();
        b.set_ctrl(ctrl::LEFT_STICK_Y, -128);
        for (f, p) in [(0, &a), (0, &a), (1, &b), (2, &b), (3, &a)] {
            rec.ticks.push(Tick {
                frame: f,
                input: p.clone(),
            });
        }
        let bytes = rec.encode();
        // 3 full inputs, 2 repeats
        assert_eq!(
            bytes.len(),
            8 + 4 + 4 + rec.header.encode().len() + 4 + 5 * 5 + 3 * PAD_INPUT_LEN
        );
        let back = Recording::parse(&bytes).unwrap();
        assert_eq!(back, rec);
        assert_eq!(back.header.get("start"), Some("boot"));
        assert!(Recording::parse(&bytes[..bytes.len() - 1]).is_err());
    }
}
