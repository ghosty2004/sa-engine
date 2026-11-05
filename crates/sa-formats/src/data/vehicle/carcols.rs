//! `carcols.dat`, as `CVehicleModelInfo::LoadVehicleColours` (0x5B6890) reads it.
//!
//! Verified on the original (decompile of 0x5B6890 up to the protection trampoline at
//! 0x5B6D08): opened `"r"` (text mode), each line read with `CFileMgr::ReadLine` (`fgets`,
//! 1024-byte buffer); leading bytes `< 0x21` are skipped (stopping at NUL or `\n`), then `,` and
//! `\r` become spaces up to the `\n` or NUL, which is replaced by NUL.
//!
//! The rest of the function continues through an obfuscated jump (0x405FB1) that Ghidra does not
//! follow; it is taken from gta-reversed and is **not yet verified on the original** (see
//! docs/re/formats.md): empty lines and lines starting with `#` are skipped; outside a section,
//! `col`, `car4`, `car` (prefix compares in that order) open a section; inside, `end` closes it;
//! `col` lines are `sscanf("%d %d %d")` (r, g, b; no return check, the next palette slot is used
//! for every line), `car` lines `sscanf("%s" + 16 x " %d")` and `car4` lines
//! `sscanf("%s" + 32 x " %d")`, with `(ret - 1) / 2` resp. `(ret - 1) / 4` colour variations.
//! `sscanf` runs on the whole buffer (from before the skipped blanks; same result).

use crate::data::{DataFile, dump_rec, dump_vals, lossy};
use crate::lines::{Fgets, Mode};
use crate::scanf::{Scan, sscanf};

const CAR2: &str = "%s %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d";
const CAR4: &str =
    "%s %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d";

/// Section of a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// `col`: palette colours.
    Col,
    /// `car`: two colours per variation.
    Car,
    /// `car4`: four colours per variation.
    Car4,
}

/// One data line.
#[derive(Debug, Clone, PartialEq)]
pub struct CarColLine {
    pub line: usize,
    pub section: Section,
    /// The `sscanf` result: `col` = r, g, b; `car`/`car4` = model name, then the colour ids.
    pub scan: Scan,
}

impl CarColLine {
    /// `car`/`car4`: the number of colour variations the loader stores.
    pub fn variations(&self) -> i32 {
        match self.section {
            Section::Col => 0,
            Section::Car => (self.scan.ret - 1) / 2,
            Section::Car4 => (self.scan.ret - 1) / 4,
        }
    }
}

/// A parsed `carcols.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CarCols {
    pub lines: Vec<CarColLine>,
}

/// The line preparation of 0x5B6890: returns the text after the skipped blanks, cut at `\n`,
/// with `,` and `\r` turned into spaces.
fn prepare(raw: &[u8]) -> Vec<u8> {
    let raw = crate::bytes::c_str(raw);
    let mut i = 0;
    while i < raw.len() && raw[i] < 0x21 && raw[i] != b'\n' {
        i += 1;
    }
    raw[i..]
        .iter()
        .take_while(|&&b| b != b'\n')
        .map(|&b| if b == b',' || b == b'\r' { b' ' } else { b })
        .collect()
}

impl CarCols {
    pub fn parse(data: &[u8]) -> CarCols {
        let mut out = CarCols::default();
        let mut section: Option<Section> = None;
        for p in Fgets::new(data, Mode::Text, 1024) {
            let t = prepare(&p.raw);
            if t.is_empty() || t[0] == b'#' {
                continue;
            }
            let Some(s) = section else {
                if t.starts_with(b"col") {
                    section = Some(Section::Col);
                } else if t.starts_with(b"car4") {
                    section = Some(Section::Car4);
                } else if t.starts_with(b"car") {
                    section = Some(Section::Car);
                }
                continue;
            };
            if t.starts_with(b"end") {
                section = None;
                continue;
            }
            let fmt = match s {
                Section::Col => "%d %d %d",
                Section::Car => CAR2,
                Section::Car4 => CAR4,
            };
            out.lines.push(CarColLine {
                line: p.line,
                section: s,
                scan: sscanf(&t, fmt),
            });
        }
        out
    }

    pub fn colours(&self) -> impl Iterator<Item = &CarColLine> {
        self.lines.iter().filter(|l| l.section == Section::Col)
    }
}

impl DataFile for CarCols {
    fn counts(&self) -> Vec<(String, u64)> {
        let n = |s| self.lines.iter().filter(|l| l.section == s).count() as u64;
        vec![
            ("carcols.dat: palette colours".into(), n(Section::Col)),
            ("carcols.dat: car (2-colour) lines".into(), n(Section::Car)),
            ("carcols.dat: car4 lines".into(), n(Section::Car4)),
        ]
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        let mut palette = 0usize;
        for l in &self.lines {
            let kind = match l.section {
                Section::Col => "col",
                Section::Car => "car",
                Section::Car4 => "car4",
            };
            if l.section == Section::Col {
                dump_rec!(&mut o, kind, line = l.line, index = palette, ret = l.scan.ret);
                palette += 1;
            } else {
                dump_rec!(
                    &mut o,
                    kind,
                    line = l.line,
                    ret = l.scan.ret,
                    variations = l.variations()
                );
            }
            o.pop();
            o.push_str(" vals=[");
            dump_vals(&l.scan.vals, &mut o);
            o.push_str("]\n");
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for l in &self.lines {
            match l.section {
                Section::Col if l.scan.ret != 3 => v.push(format!(
                    "line {}: palette colour sscanf returned {} of 3 (unassigned components keep stack values)",
                    l.line, l.scan.ret
                )),
                Section::Car if l.scan.ret % 2 == 0 => {
                    v.push(format!("line {}: car line {:?} has an odd number of colour ids", l.line,
                        lossy(l.scan.bytes(0).unwrap_or_default())))
                }
                Section::Car4 if (l.scan.ret - 1) % 4 != 0 => {
                    v.push(format!("line {}: car4 line {:?} colour ids not a multiple of 4", l.line,
                        lossy(l.scan.bytes(0).unwrap_or_default())))
                }
                _ => {}
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections() {
        let src = b"# h\r\ncol\r\n0,0,0 # black\r\n77.93,96 # typo\r\nend\r\ncar\r\nmower, 94,1, 101,1, 116\r\nend\r\ncar4\r\nx, 1,2,3,4\r\nend\r\n";
        let c = CarCols::parse(src);
        assert_eq!(c.lines.len(), 4);
        assert_eq!(c.lines[0].scan.ret, 3);
        // "77.93": %d reads 77 and stops at '.', so green and blue are not assigned.
        assert_eq!(c.lines[1].scan.ret, 1);
        assert_eq!(c.lines[2].variations(), 2);
        assert_eq!(c.lines[3].variations(), 1);
        assert_eq!(c.notes().len(), 2);
    }
}
