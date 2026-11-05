//! `popcycle.dat`, as `CPopCycle::Initialise` (0x5BC090) reads it.
//!
//! Opened `"rb"` (`POPCYCLE.DAT` in `DATA`; gta-reversed uses `"r"`), lines from `LoadLine`
//! (0x536F80). Loop order: 20 zone types, then 2 day types (weekday, weekend), then 12 time
//! slots (2 hours each). For each record the loader skips lines whose first byte is `/` or NUL,
//! then applies `sscanf` with 24 `%d` (gta-reversed uses `%hhu`; the original reads `int`s and
//! stores their low bytes). The return value is not checked and the 24 destinations are locals of
//! the function, so a short line keeps the previous record's values. Afterwards the loader
//! rescales the 18 group percentages (derived data, not part of this parse). 480 records are
//! read; anything after them is not.

use super::counts;
use crate::data::Fields;
use crate::data::{DataFile, Vars, dump_rec};
use crate::lines::{Mode, load_lines};

pub const FORMAT: &str = "%d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d";
pub const NUM_ZONE_TYPES: usize = 20;
pub const NUM_DAY_TYPES: usize = 2;
pub const NUM_SLOTS: usize = 12;

pub const NAMES: [&str; 24] = [
    "max_peds",
    "max_cars",
    "perc_dealers",
    "perc_gang",
    "perc_cops",
    "perc_other",
    "grp0",
    "grp1",
    "grp2",
    "grp3",
    "grp4",
    "grp5",
    "grp6",
    "grp7",
    "grp8",
    "grp9",
    "grp10",
    "grp11",
    "grp12",
    "grp13",
    "grp14",
    "grp15",
    "grp16",
    "grp17",
];

#[derive(Debug, Clone, PartialEq)]
pub struct PopCycleEntry {
    pub zone_type: usize,
    pub day_type: usize,
    pub slot: usize,
    pub line: usize,
    pub ret: i32,
    pub fields: Fields,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PopCycle {
    pub entries: Vec<PopCycleEntry>,
    /// Records still expected when the file ended (the exe would pass NULL to `sscanf`).
    pub missing: usize,
    pub unread_lines: usize,
}

impl PopCycle {
    pub fn parse(data: &[u8]) -> PopCycle {
        let lines = load_lines(data, Mode::Binary);
        let mut it = lines.iter();
        let mut out = PopCycle::default();
        let mut v = Vars::default();
        let total = NUM_ZONE_TYPES * NUM_DAY_TYPES * NUM_SLOTS;
        'outer: for zone_type in 0..NUM_ZONE_TYPES {
            for day_type in 0..NUM_DAY_TYPES {
                for slot in 0..NUM_SLOTS {
                    let l = loop {
                        match it.next() {
                            None => {
                                out.missing = total - out.entries.len();
                                break 'outer;
                            }
                            Some(l) if l.first() == b'/' || l.first() == 0 => continue,
                            Some(l) => break l,
                        }
                    };
                    let ret = v.scan(&l.text, FORMAT, &NAMES);
                    out.entries.push(PopCycleEntry {
                        zone_type,
                        day_type,
                        slot,
                        line: l.number,
                        ret,
                        fields: Fields::snapshot(&v, &NAMES),
                    });
                }
            }
        }
        out.unread_lines = it.filter(|l| l.first() != b'/' && l.first() != 0).count();
        out
    }
}

impl DataFile for PopCycle {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("popcycle: records".into(), self.entries.len() as u64),
            (
                "popcycle: records with sscanf < 24".into(),
                self.entries.iter().filter(|e| e.ret != 24).count() as u64,
            ),
            ("popcycle: missing records".into(), self.missing as u64),
            ("popcycle: data lines never read".into(), self.unread_lines as u64),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for e in &self.entries {
            dump_rec!(
                &mut o,
                "pc",
                zone = e.zone_type,
                day = e.day_type,
                slot = e.slot,
                line = e.line,
                ret = e.ret
            );
            o.pop();
            e.fields.dump_to(&mut o);
            o.push('\n');
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .entries
            .iter()
            .filter(|e| e.ret != 24)
            .map(|e| format!("line {}: sscanf returned {} of 24", e.line, e.ret))
            .collect();
        if self.missing > 0 {
            v.push(format!("file ends {} records early", self.missing));
        }
        if self.unread_lines > 0 {
            v.push(format!(
                "{} data lines after the 480th record are not read",
                self.unread_lines
            ));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_and_comments() {
        let mut src = String::from("// header\r\n");
        for i in 0..481 {
            src.push_str(&format!(
                "{i} 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 // c\r\n"
            ));
            if i % 12 == 0 {
                src.push_str("//\r\n\r\n");
            }
        }
        let p = PopCycle::parse(src.as_bytes());
        assert_eq!(p.entries.len(), 480);
        assert_eq!(p.entries[12].day_type, 1);
        assert_eq!(p.entries[24].zone_type, 1);
        assert_eq!(p.entries[24].fields.i32("max_peds"), Some(24));
        assert_eq!(p.entries[0].ret, 24);
        assert_eq!(p.unread_lines, 1);
    }
}
