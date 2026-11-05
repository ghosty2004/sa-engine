//! `timecyc.dat`, as `CTimeCycle::Initialise` (0x5BBAC0) reads it.
//!
//! Opened `"rb"` (`TIMECYC.DAT` in `DATA`), lines from `LoadLine` (0x536F80). For each of the
//! 23 weathers and, inside, each of the 8 hours, the loader skips lines whose first byte is `/`
//! or NUL and applies one `sscanf` (format [`FORMAT`], 52 conversions) to the next line. The
//! return value is not checked, and the 52 destinations are locals of the whole function: a
//! short line leaves the remaining fields with the previous record's values. Exactly 184 records
//! are read; the rest of the file is never read. A file with fewer records would pass a null
//! line to `sscanf` (crash); the parser reports that as `missing`.
//!
//! The install's lines have 51 columns: the 52nd variable (`dir_mult`) is never assigned and
//! keeps its uninitialised stack value for every record (dumped as `~`).
//!
//! The exe has no reference to `timecycp.dat` (no such string in the binary); that file has the
//! same layout with one extra column, and is parsed with the same reader for information only.

use super::counts;
use crate::data::Fields;
use crate::data::{DataFile, Vars, dump_rec};
use crate::lines::{Mode, load_lines};

pub const FORMAT: &str = "%d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %f %f %f %d %d %d %f %f %f %d %d %d %d %d %d %f %f %f %f  %f %f %f %f  %f %f %f %f %f %d %d %f";

pub const NUM_WEATHERS: usize = 23;
pub const NUM_HOURS: usize = 8;

/// Variable names in format order (gta-reversed names).
pub const NAMES: [&str; 52] = [
    "amb_r",
    "amb_g",
    "amb_b",
    "amb_obj_r",
    "amb_obj_g",
    "amb_obj_b",
    "dir_r",
    "dir_g",
    "dir_b",
    "sky_top_r",
    "sky_top_g",
    "sky_top_b",
    "sky_bot_r",
    "sky_bot_g",
    "sky_bot_b",
    "sun_core_r",
    "sun_core_g",
    "sun_core_b",
    "sun_corona_r",
    "sun_corona_g",
    "sun_corona_b",
    "sun_size",
    "sprite_size",
    "sprite_brightness",
    "shadow_strength",
    "light_shadow_strength",
    "pole_shadow_strength",
    "far_clip",
    "fog_start",
    "light_on_ground",
    "low_cloud_r",
    "low_cloud_g",
    "low_cloud_b",
    "bottom_cloud_r",
    "bottom_cloud_g",
    "bottom_cloud_b",
    "water_r",
    "water_g",
    "water_b",
    "water_a",
    "postfx1_a",
    "postfx1_r",
    "postfx1_g",
    "postfx1_b",
    "postfx2_a",
    "postfx2_r",
    "postfx2_g",
    "postfx2_b",
    "cloud_alpha",
    "highlight_min_intensity",
    "water_fog_alpha",
    "dir_mult",
];

/// One (weather, hour) record.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeCycEntry {
    pub weather: usize,
    pub hour: usize,
    /// 1-based line number.
    pub line: usize,
    pub ret: i32,
    /// The 52 variables after this record's `sscanf`.
    pub fields: Fields,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TimeCyc {
    pub entries: Vec<TimeCycEntry>,
    /// Records the loop would still need when the file ended (the exe would crash).
    pub missing: usize,
    /// Non-skipped lines after the 184th record (never read by the exe).
    pub unread_lines: usize,
}

impl TimeCyc {
    pub fn parse(data: &[u8]) -> TimeCyc {
        let lines = load_lines(data, Mode::Binary);
        let mut it = lines.iter().peekable();
        let mut out = TimeCyc::default();
        let mut v = Vars::default();
        'outer: for weather in 0..NUM_WEATHERS {
            for hour in 0..NUM_HOURS {
                let l = loop {
                    match it.next() {
                        None => {
                            out.missing = NUM_WEATHERS * NUM_HOURS - out.entries.len();
                            break 'outer;
                        }
                        Some(l) if l.first() == b'/' || l.first() == 0 => continue,
                        Some(l) => break l,
                    }
                };
                let ret = v.scan(&l.text, FORMAT, &NAMES);
                out.entries.push(TimeCycEntry {
                    weather,
                    hour,
                    line: l.number,
                    ret,
                    fields: Fields::snapshot(&v, &NAMES),
                });
            }
        }
        out.unread_lines = it.filter(|l| l.first() != b'/' && l.first() != 0).count();
        out
    }
}

impl DataFile for TimeCyc {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("timecyc: records".into(), self.entries.len() as u64),
            (
                "timecyc: records with 51 fields (no dir_mult column)".into(),
                self.entries.iter().filter(|e| e.ret == 51).count() as u64,
            ),
            (
                "timecyc: records with fewer than 51 fields".into(),
                self.entries.iter().filter(|e| e.ret < 51).count() as u64,
            ),
            ("timecyc: missing records".into(), self.missing as u64),
            ("timecyc: data lines never read".into(), self.unread_lines as u64),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for e in &self.entries {
            dump_rec!(
                &mut o,
                "tc",
                weather = e.weather,
                hour = e.hour,
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
            .filter(|e| e.ret < 51)
            .map(|e| {
                format!(
                    "line {}: weather {} hour {}: sscanf returned {} (fields shift; the rest keep the previous record's values)",
                    e.line, e.weather, e.hour, e.ret
                )
            })
            .collect();
        if self.missing > 0 {
            v.push(format!(
                "file ends {} records early (the exe would pass NULL to sscanf)",
                self.missing
            ));
        }
        if self.unread_lines > 0 {
            v.push(format!(
                "{} data lines after the 184th record are not read",
                self.unread_lines
            ));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: i32) -> String {
        let mut s = String::new();
        for i in 0..52 {
            s.push_str(&format!("{} ", n * 100 + i));
        }
        s.push_str("\r\n");
        s
    }

    #[test]
    fn records_persist_and_skip() {
        let mut src = String::from("// comment\r\n\r\n");
        src.push_str(&line(1));
        src.push_str("7 8\r\n"); // short: the rest keeps record 0's values
        for i in 0..200 {
            src.push_str(&line(i));
        }
        let t = TimeCyc::parse(src.as_bytes());
        assert_eq!(t.entries.len(), 184);
        assert_eq!(t.entries[0].line, 3);
        assert_eq!(t.entries[0].fields.i32("amb_r"), Some(100));
        assert_eq!(t.entries[1].ret, 2);
        assert_eq!(t.entries[1].fields.i32("amb_g"), Some(8));
        assert_eq!(t.entries[1].fields.i32("amb_b"), Some(102));
        assert_eq!(t.entries[1].fields.f32("dir_mult"), Some(151.0));
        assert_eq!(t.entries[8].weather, 1);
        assert_eq!(t.unread_lines, 200 - 182);
        let short = TimeCyc::parse(line(1).as_bytes());
        assert_eq!(short.missing, 183);
    }
}
