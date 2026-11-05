//! Decision maker files (`data/decision/allowed/*.ped|*.grp`) and `data/decision/PedEvent.txt`.
//!
//! * `CDecisionMakerTypesFileLoader::LoadDecisionMaker(const char*, CDecisionMaker*)`
//!   (0x6076B0): sets the directory to `data\decision\allowed\`, opens the file `"r"` (text
//!   mode) and reads raw lines with `CFileMgr::ReadLine` (`fgets`, 512 bytes, **no LoadLine
//!   sanitising**, so the commas stay). The first line is discarded. Every following line is read
//!   with one `sscanf` of [`FMT_DECISION`] (80 fields: event id, an unused int, then 6 choices of
//!   task id, 4 relationship probabilities, 2 flags, 6 facial probabilities); the return value
//!   is not checked and the variables are kept between lines. gta-reversed's version is a
//!   refactor with `%n` loops; the original uses the single format.
//! * `CDecisionMakerTypes::LoadEventIndices` (0x600840) -> FUN_005BB9F0: sets the directory to
//!   `data\decision\`, opens `PedEvent.txt` `"r"`, reads raw `fgets` lines of 256 bytes, skips
//!   lines whose first byte is NUL or `\n`, and reads `%s %d` (event name, event id); the line's
//!   ordinal among the non-skipped lines is stored at index `id`. The id variable is kept
//!   between lines.
//!
//! Only the files in `data/decision/allowed/` are opened by the exe (the default set in
//! `LoadDefaultDecisionMaker` 0x5BF400, plus the script-loadable names of `GetPedDMName`); the
//! other `.ped`/`.grp` files under `data/decision/` are never referenced.

use super::{Fields, Rec, counts};
use crate::data::{DataFile, ScanRec, Vars};
use crate::lines::{Fgets, Mode};

/// The exact format string of 0x6076B0 (at 0x86CDA0 in the exe; 80 conversions).
pub const FMT_DECISION: &str = "%d, %d, %d, %f, %f, %f, %f, %d, %d, %f, %f, %f, %f, %f, %f, %d, %f, %f, %f, %f, %d, %d, %f, %f, %f, %f, %f, %f, %d, %f, %f, %f, %f, %d, %d, %f, %f, %f, %f, %f, %f, %d, %f, %f, %f, %f, %d, %d, %f, %f, %f, %f, %f, %f, %d, %f, %f, %f, %f, %d, %d, %f, %f, %f, %f, %f, %f, %d, %f, %f, %f, %f, %d, %d, %f, %f, %f, %f, %f, %f \n";

/// A parsed decision maker file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DecisionFile {
    /// The discarded first line (raw).
    pub header: Vec<u8>,
    /// One scan per following `fgets` line.
    pub lines: Vec<ScanRec>,
}

impl DecisionFile {
    pub fn parse(data: &[u8]) -> DecisionFile {
        let mut out = DecisionFile::default();
        let mut it = Fgets::new(data, Mode::Text, 512);
        if let Some(h) = it.next() {
            out.header = h.raw;
        }
        for p in it {
            out.lines.push(ScanRec::run(p.line, &p.raw, &[FMT_DECISION]));
        }
        out
    }
}

impl DataFile for DecisionFile {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("decision: files".into(), 1),
            ("decision: event lines".into(), self.lines.len() as u64),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for r in &self.lines {
            o.push_str(&format!("event line={} ret={} vals=[", r.line, r.scans[0].1.ret));
            crate::data::dump_vals(&r.scans[0].1.vals, &mut o);
            o.push_str("]\n");
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        self.lines
            .iter()
            .filter(|r| r.scans[0].1.ret != 80)
            .map(|r| {
                format!(
                    "line {}: decision sscanf returned {} of 80 (earlier values kept)",
                    r.line, r.scans[0].1.ret
                )
            })
            .collect()
    }
}

/// A parsed `PedEvent.txt`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PedEvents {
    /// Fields `name`, `id`; the record's position is the stored index.
    pub records: Vec<Rec>,
}

impl PedEvents {
    pub fn parse(data: &[u8]) -> PedEvents {
        let mut out = PedEvents::default();
        let mut v = Vars::default();
        for p in Fgets::new(data, Mode::Text, 256) {
            let first = p.raw.first().copied().unwrap_or(0);
            if first == 0 || first == b'\n' {
                continue;
            }
            let ret = v.scan(&p.raw, "%s %d", &["name", "id"]);
            out.records.push(Rec {
                line: p.line,
                ret,
                fields: Fields::snapshot(&v, &["name", "id"]),
            });
        }
        out
    }
}

impl DataFile for PedEvents {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![("PedEvent.txt: events".into(), self.records.len() as u64)])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for r in &self.records {
            r.dump_to("event", &mut o);
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|r| r.ret != 2)
            .map(|r| format!("line {}: PedEvent sscanf returned {} of 2", r.line, r.ret))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_lines() {
        let mut line = String::from("16,4");
        for _ in 0..6 {
            line.push_str(",200,0.5,0.0,0.0,100.0,0,1,0,0,0,0,0,0");
        }
        let src = format!("data values2:\r\n{line}\r\n");
        let d = DecisionFile::parse(src.as_bytes());
        assert_eq!(d.header, b"data values2:\n");
        assert_eq!(d.lines.len(), 1);
        assert_eq!(d.lines[0].scans[0].1.ret, 80);
        assert_eq!(d.lines[0].scans[0].1.f32(3), Some(0.5));
    }

    #[test]
    fn ped_events() {
        let e = PedEvents::parse(b"EVENT_A 7 \r\n\r\nEVENT_B 8\r\n");
        assert_eq!(e.records.len(), 2);
        assert_eq!(e.records[1].fields.i32("id"), Some(8));
    }

    #[test]
    fn format_has_80_conversions() {
        assert_eq!(FMT_DECISION.matches('%').count(), 80);
    }
}
