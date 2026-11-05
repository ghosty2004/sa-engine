//! `data/gridref.dat`, as `CGridRef::Init` (0x71D4E0) reads it.
//!
//! Opened `"rb"`, `LoadLine` (0x536F80); lines starting with `#` or NUL are skipped; each other
//! line is read with `%c%d %s` (column letter as a byte, row number, artist name; no return
//! check, variables initialised to 0 once and kept between lines). The name goes through
//! FUN_00836C58 (a string case conversion) and is copied with `strcpy` to the 32-byte slot
//! `(row - 0x28B + letter * 10)`, i.e. `(letter - 'A') * 10 + (row - 1)` for upper-case letters.

use super::{Fields, Rec, counts};
use crate::data::{DataFile, Vars};
use crate::lines::{Mode, load_lines};
use crate::scanf::Val;

/// A parsed `gridref.dat`. Fields `col` (one byte), `row`, `name`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GridRef {
    pub records: Vec<Rec>,
}

impl GridRef {
    pub fn parse(data: &[u8]) -> GridRef {
        const N: [&str; 3] = ["col", "row", "name"];
        let mut v = Vars::default()
            .init("col", Val::Str(vec![0]))
            .init("row", Val::Int(0));
        let mut records = Vec::new();
        for l in load_lines(data, Mode::Binary) {
            let c = l.first();
            if c == b'#' || c == 0 {
                continue;
            }
            let ret = v.scan(&l.text, "%c%d %s", &N);
            records.push(Rec {
                line: l.number,
                ret,
                fields: Fields::snapshot(&v, &N),
            });
        }
        GridRef { records }
    }
}

impl DataFile for GridRef {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![("gridref.dat: entries".into(), self.records.len() as u64)])
    }
    fn dump(&self) -> String {
        let mut o = String::new();
        for r in &self.records {
            r.dump_to("cell", &mut o);
        }
        o
    }
    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for r in &self.records {
            if r.ret != 3 {
                v.push(format!("line {}: gridref sscanf returned {} of 3", r.line, r.ret));
            }
            if r.fields.bytes("name").is_some_and(|n| n.len() > 31) {
                v.push(format!("line {}: name longer than the 32-byte slot", r.line));
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells() {
        let g = GridRef::parse(b"# c\r\nA1\tGARY\r\nB10 x\r\nC\r\n");
        assert_eq!(g.records.len(), 3);
        assert_eq!(g.records[0].fields.bytes("col"), Some(&b"A"[..]));
        assert_eq!(g.records[1].fields.i32("row"), Some(10));
        assert_eq!(g.records[2].ret, 1);
        assert_eq!(g.records[2].fields.i32("row"), Some(10));
    }
}
