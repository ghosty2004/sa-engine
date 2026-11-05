//! Surface tables, as `SurfaceInfos_c::Init` reads them:
//!
//! * `surface.dat`: `LoadAdhesiveLimits` (0x55D0E0), mode `"rb"`;
//! * `surfinfo.dat`: `LoadSurfaceInfos` (0x55EB90), mode `"r"`;
//! * `surfaud.dat`: `LoadSurfaceAudioInfos` (0x55F2B0), mode `"r"`.
//!
//! All three read lines with `LoadLine` (0x536F80).
//!
//! **surface.dat.** Lines starting with `;` or NUL are skipped. Each other line is row `r`
//! (counting from 0 over non-skipped lines): `sscanf("%s")` reads the name, then the pointer
//! moves to the first blank (space or tab; this first loop has no NUL check, so a line made of
//! only a name would run past its end: [`AdhesiveRow::name_overrun`]). Then for each column
//! `0..=r`: skip blanks; the limit is 0 unless the byte is not `-`, in which case
//! `sscanf("%f")` reads it (unchecked; a failed read leaves 0); then move to the next blank or
//! NUL. The value goes to `[r][c]` and `[c][r]` (derived, not done here).
//!
//! **surfinfo.dat.** Lines starting with `#` or NUL are skipped; every other line gets one
//! `sscanf` with [`SURFINFO_FORMAT`] (36 conversions, no return check). The destinations are
//! function locals, so a short line keeps the previous line's values.
//!
//! **surfaud.dat.** Same skip rule; `sscanf("%s %d %d %d %d %d %d %d %d %d")`, no return check,
//! locals persist.

use super::counts;
use crate::data::{DataFile, Vars, dump_rec};
use crate::data::{Fields, Rec};
use crate::lines::{Mode, load_lines};
use crate::scanf::sscanf;

/// One row of `surface.dat`.
#[derive(Debug, Clone, PartialEq)]
pub struct AdhesiveRow {
    pub line: usize,
    pub row: usize,
    /// `sscanf("%s")` result (the previous row's name is kept on failure, which cannot happen
    /// for a non-empty line).
    pub name: Vec<u8>,
    /// `true` when the name is not followed by a blank inside the line (the exe would scan past
    /// the end of the line; columns are then not decoded).
    pub name_overrun: bool,
    /// `row + 1` limits; `-` and unreadable entries are 0.
    pub limits: Vec<f32>,
    /// Per column: `Some(ret)` of the `%f` read, `None` for a `-` entry.
    pub rets: Vec<Option<i32>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SurfaceDat {
    pub rows: Vec<AdhesiveRow>,
}

fn is_blank(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

impl SurfaceDat {
    pub fn parse(data: &[u8]) -> SurfaceDat {
        let mut out = SurfaceDat::default();
        let mut name: Vec<u8> = Vec::new();
        for l in load_lines(data, Mode::Binary) {
            if matches!(l.first(), b';' | 0) {
                continue;
            }
            let t = &l.text[..];
            let row = out.rows.len();
            if let Some(n) = sscanf(t, "%s").string(0) {
                name = n;
            }
            let mut p = 0;
            while p < t.len() && !is_blank(t[p]) {
                p += 1;
            }
            let mut r = AdhesiveRow {
                line: l.number,
                row,
                name: name.clone(),
                name_overrun: p >= t.len(),
                limits: Vec::new(),
                rets: Vec::new(),
            };
            if !r.name_overrun {
                for _ in 0..=row {
                    while p < t.len() && is_blank(t[p]) {
                        p += 1;
                    }
                    let mut limit = 0.0f32;
                    if t.get(p) != Some(&b'-') {
                        let s = sscanf(&t[p..], "%f");
                        if let Some(f) = s.f32(0) {
                            limit = f;
                        }
                        r.rets.push(Some(s.ret));
                    } else {
                        r.rets.push(None);
                    }
                    while p < t.len() && !is_blank(t[p]) {
                        p += 1;
                    }
                    r.limits.push(limit);
                }
            }
            out.rows.push(r);
        }
        out
    }
}

impl DataFile for SurfaceDat {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("surface.dat: rows".into(), self.rows.len() as u64),
            (
                "surface.dat: limits read".into(),
                self.rows
                    .iter()
                    .map(|r| r.rets.iter().filter(|x| **x == Some(1)).count() as u64)
                    .sum(),
            ),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for r in &self.rows {
            dump_rec!(
                &mut o,
                "adhesive",
                line = r.line,
                row = r.row,
                name = r.name,
                overrun = r.name_overrun
            );
            o.pop();
            for (i, x) in r.limits.iter().enumerate() {
                o.push_str(&format!(" c{i}={x:?}"));
            }
            o.push('\n');
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for r in &self.rows {
            if r.name_overrun {
                v.push(format!(
                    "line {}: name without a following blank: the exe reads past the line",
                    r.line
                ));
            }
            for (c, x) in r.rets.iter().enumerate() {
                if matches!(x, Some(n) if *n != 1) {
                    v.push(format!("line {}: column {c} not a number (limit 0)", r.line));
                }
            }
            if r.row >= 6 {
                v.push(format!("line {}: row {} beyond the 6x6 table", r.line, r.row));
            }
        }
        v
    }
}

pub const SURFINFO_FORMAT: &str = "%s %s %f %f %s %s %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %s";

/// gta-reversed `SurfaceInfo` member names, in format order.
pub const SURFINFO_NAMES: [&str; 36] = [
    "name",
    "adhesion_group",
    "tyre_grip",
    "wet_grip",
    "skidmark",
    "friction_effect",
    "soft_land",
    "see_through",
    "shoot_through",
    "sand",
    "water",
    "shallow_water",
    "beach",
    "steep_slope",
    "glass",
    "stairs",
    "skateable",
    "pavement",
    "roughness",
    "flame",
    "sparks",
    "sprint",
    "footsteps",
    "foot_dust",
    "car_dirt",
    "car_clean",
    "wheel_grass",
    "wheel_gravel",
    "wheel_mud",
    "wheel_dust",
    "wheel_sand",
    "wheel_spray",
    "proc_plant",
    "proc_obj",
    "climbable",
    "bullet_fx",
];

/// Shared shape of `surfinfo.dat` and `surfaud.dat`: one persistent-locals `sscanf` per line.
fn parse_lines(data: &[u8], mode: Mode, fmt: &str, names: &[&'static str]) -> Vec<Rec> {
    let mut v = Vars::default();
    let mut out = Vec::new();
    for l in load_lines(data, mode) {
        if matches!(l.first(), b'#' | 0) {
            continue;
        }
        let ret = v.scan(&l.text, fmt, names);
        out.push(Rec {
            line: l.number,
            ret,
            fields: Fields::snapshot(&v, names),
        });
    }
    out
}

fn dump_entries(kind: &str, e: &[Rec]) -> String {
    let mut o = String::new();
    for e in e {
        dump_rec!(&mut o, kind, line = e.line, ret = e.ret);
        o.pop();
        e.fields.dump_to(&mut o);
        o.push('\n');
    }
    o
}

fn short_notes(e: &[Rec], want: i32) -> Vec<String> {
    e.iter()
        .filter(|e| e.ret != want)
        .map(|e| format!("line {}: sscanf returned {} of {want}", e.line, e.ret))
        .collect()
}

/// `surfinfo.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SurfInfo {
    pub entries: Vec<Rec>,
}

impl SurfInfo {
    pub fn parse(data: &[u8]) -> SurfInfo {
        SurfInfo {
            entries: parse_lines(data, Mode::Text, SURFINFO_FORMAT, &SURFINFO_NAMES),
        }
    }
}

impl DataFile for SurfInfo {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("surfinfo.dat: entries".into(), self.entries.len() as u64),
            (
                "surfinfo.dat: entries with sscanf < 36".into(),
                self.entries.iter().filter(|e| e.ret != 36).count() as u64,
            ),
        ])
    }
    fn dump(&self) -> String {
        dump_entries("surfinfo", &self.entries)
    }
    fn notes(&self) -> Vec<String> {
        short_notes(&self.entries, 36)
    }
}

pub const SURFAUD_FORMAT: &str = "%s %d %d %d %d %d %d %d %d %d";
pub const SURFAUD_NAMES: [&str; 10] = [
    "name",
    "concrete",
    "grass",
    "sand",
    "gravel",
    "wood",
    "water",
    "metal",
    "long_grass",
    "tile",
];

/// `surfaud.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SurfAud {
    pub entries: Vec<Rec>,
}

impl SurfAud {
    pub fn parse(data: &[u8]) -> SurfAud {
        SurfAud {
            entries: parse_lines(data, Mode::Text, SURFAUD_FORMAT, &SURFAUD_NAMES),
        }
    }
}

impl DataFile for SurfAud {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("surfaud.dat: entries".into(), self.entries.len() as u64),
            (
                "surfaud.dat: entries with sscanf < 10".into(),
                self.entries.iter().filter(|e| e.ret != 10).count() as u64,
            ),
        ])
    }
    fn dump(&self) -> String {
        dump_entries("surfaud", &self.entries)
    }
    fn notes(&self) -> Vec<String> {
        short_notes(&self.entries, 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adhesive_rows() {
        let src = b"; c\r\nRubber 6.0\r\nHard\t3.6 - \r\nRoad 1 2 x\r\nLast";
        let s = SurfaceDat::parse(src);
        assert_eq!(s.rows.len(), 4);
        assert_eq!(s.rows[0].limits, vec![6.0]);
        assert_eq!(s.rows[1].limits, vec![3.6, 0.0]);
        assert_eq!(s.rows[1].rets, vec![Some(1), None]);
        assert_eq!(s.rows[2].limits, vec![1.0, 2.0, 0.0]);
        assert_eq!(s.rows[2].rets[2], Some(0));
        assert!(s.rows[3].name_overrun);
    }

    #[test]
    fn surfinfo_and_aud() {
        assert_eq!(crate::scanf::sscanf(b"", SURFINFO_FORMAT).vals.len(), 36);
        let mut line = String::from("DEFAULT ROAD 1.0 -0.5 DEFAULT NONE");
        for i in 0..29 {
            line.push_str(&format!(" {}", i % 2));
        }
        line.push_str(" SPARKS\r\n# x\r\nSHORT HARD\r\n");
        let s = SurfInfo::parse(line.as_bytes());
        assert_eq!(s.entries.len(), 2);
        assert_eq!(s.entries[0].ret, 36);
        assert_eq!(s.entries[0].fields.f32("wet_grip"), Some(-0.5));
        assert_eq!(s.entries[1].ret, 2);
        assert_eq!(s.entries[1].fields.bytes("bullet_fx"), Some(&b"SPARKS"[..]));
        let a = SurfAud::parse(b"# h\nA 1 0 0 0 0 0 0 0 1\n");
        assert_eq!(a.entries[0].ret, 10);
        assert_eq!(a.entries[0].fields.i32("tile"), Some(1));
    }
}
