//! `procobj.dat`, as `ProcObjectMan_c::LoadDataFile` (0x5A3140) reads it.
//!
//! Opened `"r"`, lines from `LoadLine` (0x536F80). Lines starting with `#` or NUL are skipped.
//! Every other line gets `sscanf("%s %s %f %f %d %d %f %f %f %f %f %f %d %d")` (no return check;
//! the destinations are function locals that persist across lines) and adds one procedural
//! object surface entry (the entry count grows for every such line).

use super::counts;
use crate::data::{DataFile, Vars};
use crate::data::{Fields, Rec};
use crate::lines::{Mode, load_lines};

pub const FORMAT: &str = "%s %s %f %f %d %d %f %f %f %f %f %f %d %d";
pub const NAMES: [&str; 14] = [
    "surface",
    "object",
    "spacing",
    "min_dist",
    "min_rot",
    "max_rot",
    "min_scale",
    "max_scale",
    "min_scale_z",
    "max_scale_z",
    "z_offset_min",
    "z_offset_max",
    "align",
    "use_grid",
];

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProcObj {
    pub entries: Vec<Rec>,
}

impl ProcObj {
    pub fn parse(data: &[u8]) -> ProcObj {
        let mut v = Vars::default();
        let mut out = ProcObj::default();
        for l in load_lines(data, Mode::Text) {
            if matches!(l.first(), b'#' | 0) {
                continue;
            }
            let ret = v.scan(&l.text, FORMAT, &NAMES);
            out.entries.push(Rec {
                line: l.number,
                ret,
                fields: Fields::snapshot(&v, &NAMES),
            });
        }
        out
    }
}

impl DataFile for ProcObj {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("procobj.dat: entries".into(), self.entries.len() as u64),
            (
                "procobj.dat: entries with sscanf < 14".into(),
                self.entries.iter().filter(|e| e.ret != 14).count() as u64,
            ),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for e in &self.entries {
            e.dump_to("procobj", &mut o);
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.ret != 14)
            .map(|e| format!("line {}: sscanf returned {} of 14", e.line, e.ret))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries() {
        let p = ProcObj::parse(b"# h\n\nGRASS obj 1.0 2 0 360 1 1 1 1 0 0 1 0\nSAND x\n");
        assert_eq!(p.entries.len(), 2);
        assert_eq!(p.entries[0].ret, 14);
        assert_eq!(p.entries[0].fields.i32("max_rot"), Some(360));
        assert_eq!(p.entries[1].ret, 2);
        assert_eq!(p.entries[1].fields.f32("spacing"), Some(1.0));
    }
}
