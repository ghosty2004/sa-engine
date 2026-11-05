//! `data/ar_stats.dat` and `data/statdisp.dat` (CStats).
//!
//! * `ar_stats.dat`: `CStats::LoadActionReactionStats` (0x5599B0). Opened `"rb"`, `LoadLine`,
//!   lines starting with `#` or NUL skipped, each other line `%d %s %f` (stat index, name,
//!   value); no return check. The index and value variables are initialised to 0 once and
//!   keep their values between lines.
//! * `statdisp.dat`: `CStats::LoadStatUpdateConditions` (0x559860). Same reader; each line
//!   `%d %s %s %f %s` (stat, name, condition, value, GXT key), no return check, variables kept
//!   between lines (stat and value initialised to 0). The condition is compared with
//!   `lessthan`/`morethan` over 9 bytes (exact match); on neither the condition byte of the
//!   entry is not written. The GXT key is copied with `strcpy` into an 8-byte field.

use super::{Fields, Rec, counts};
use crate::data::{DataFile, Vars};
use crate::lines::{Mode, load_lines};
use crate::scanf::Val;

fn parse_lines(data: &[u8], fmt: &str, names: &[&'static str], init: Vars) -> Vec<Rec> {
    let mut v = init;
    let mut out = Vec::new();
    for l in load_lines(data, Mode::Binary) {
        let c = l.first();
        if c == b'#' || c == 0 {
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

fn short_notes(recs: &[Rec], want: i32, file: &str) -> Vec<String> {
    recs.iter()
        .filter(|r| r.ret != want)
        .map(|r| format!("line {}: {file} sscanf returned {} of {want}", r.line, r.ret))
        .collect()
}

fn dump(recs: &[Rec], kind: &str) -> String {
    let mut o = String::new();
    for r in recs {
        r.dump_to(kind, &mut o);
    }
    o
}

/// A parsed `ar_stats.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArStats {
    /// Fields `stat`, `name`, `value`.
    pub records: Vec<Rec>,
}

impl ArStats {
    pub fn parse(data: &[u8]) -> ArStats {
        let init = Vars::default()
            .init("stat", Val::Int(0))
            .init("value", Val::Float(0.0));
        ArStats {
            records: parse_lines(data, "%d %s %f", &["stat", "name", "value"], init),
        }
    }
}

impl DataFile for ArStats {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![("ar_stats.dat: entries".into(), self.records.len() as u64)])
    }
    fn dump(&self) -> String {
        dump(&self.records, "stat")
    }
    fn notes(&self) -> Vec<String> {
        short_notes(&self.records, 3, "ar_stats")
    }
}

/// A parsed `statdisp.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StatDisp {
    /// Fields `stat`, `name`, `condition`, `value`, `text`.
    pub records: Vec<Rec>,
}

impl StatDisp {
    pub fn parse(data: &[u8]) -> StatDisp {
        let init = Vars::default()
            .init("stat", Val::Int(0))
            .init("value", Val::Float(0.0));
        StatDisp {
            records: parse_lines(
                data,
                "%d %s %s %f %s",
                &["stat", "name", "condition", "value", "text"],
                init,
            ),
        }
    }
}

impl DataFile for StatDisp {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![("statdisp.dat: entries".into(), self.records.len() as u64)])
    }
    fn dump(&self) -> String {
        dump(&self.records, "cond")
    }
    fn notes(&self) -> Vec<String> {
        let mut v = short_notes(&self.records, 5, "statdisp");
        for r in &self.records {
            let c = r.fields.bytes("condition").unwrap_or(&[]);
            if c != b"lessthan" && c != b"morethan" {
                v.push(format!(
                    "line {}: condition {:?} is neither lessthan nor morethan",
                    r.line,
                    crate::data::lossy(c)
                ));
            }
            if r.fields.bytes("text").is_some_and(|t| t.len() > 7) {
                v.push(format!(
                    "line {}: text key longer than 7 bytes overflows the 8-byte field",
                    r.line
                ));
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence() {
        let a = ArStats::parse(b"# x\r\n1 A 2.5\r\nbad\r\n");
        assert_eq!(a.records.len(), 2);
        assert_eq!(a.records[1].ret, 0);
        // The failed line keeps the previous values.
        assert_eq!(a.records[1].fields.f32("value"), Some(2.5));
        let s = StatDisp::parse(b"21\tFAT\tmorethan\t250\tSBOFAT0\r\n");
        assert_eq!(s.records[0].ret, 5);
        assert_eq!(s.records[0].fields.bytes("text"), Some(&b"SBOFAT0"[..]));
    }
}
