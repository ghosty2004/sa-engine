//! `object.dat`, as `CObjectData::Initialise` (0x5B5360) reads it.
//!
//! Opened `"rb"` (path from `LoadLevel`: `DATA\OBJECT.DAT`), lines from `LoadLine` (0x536F80).
//! Lines whose first byte is `;` or NUL are skipped; **a line starting with `*` ends the read**
//! (the install's `*********melee weapons*************` line, so the melee weapon entries after
//! it are never loaded). `#` is not a comment for this loader (gta-reversed skips it; the
//! original does not): such a line is scanned with `%s` = `#` and matches no model.
//!
//! Each other line gets one `sscanf` with [`FORMAT`] (24 conversions, no return check). Before
//! the call the loader zeroes the free `CObjectData` slot it scans into, so the slot fields
//! (mass, turn mass, air resistance, elasticity, uproot limit, collision damage multiplier, FX
//! offset, smash multiplier, break velocity and its randomness, gun break mode, sparks on impact)
//! default to 0 per line, while the stack variables (name, percent submerged, collision damage
//! effect, special collision response, camera avoid, causes explosion, FX type, FX name) keep the
//! previous line's values when the line is short. The two last fields are `%d` written into the
//! slot's float members (raw integer bits).

use super::counts;
use crate::data::Fields;
use crate::data::{DataFile, Vars, dump_rec};
use crate::lines::{Mode, load_lines};
use crate::scanf::Val;

pub const FORMAT: &str = "%s %f %f %f %f %f %f %f %d %d %d %d %d %f %f %f %s %f %f %f %f %f %d %d";

pub const NAMES: [&str; 24] = [
    "name",
    "mass",
    "turn_mass",
    "air_resistance",
    "elasticity",
    "percent_submerged",
    "uproot_limit",
    "col_damage_multiplier",
    "col_damage_effect",
    "special_col_response",
    "camera_avoid",
    "causes_explosion",
    "fx_type",
    "fx_offset_x",
    "fx_offset_y",
    "fx_offset_z",
    "fx_name",
    "smash_multiplier",
    "break_velocity_x",
    "break_velocity_y",
    "break_velocity_z",
    "break_velocity_rand",
    "gun_break_mode",
    "sparks_on_impact",
];

/// Fields stored in the zeroed slot (reset to 0 before each line).
const SLOT_FLOATS: [&str; 13] = [
    "mass",
    "turn_mass",
    "air_resistance",
    "elasticity",
    "uproot_limit",
    "col_damage_multiplier",
    "fx_offset_x",
    "fx_offset_y",
    "fx_offset_z",
    "smash_multiplier",
    "break_velocity_x",
    "break_velocity_y",
    "break_velocity_z",
];
const SLOT_FLOATS2: [&str; 1] = ["break_velocity_rand"];
const SLOT_INTS: [&str; 2] = ["gun_break_mode", "sparks_on_impact"];

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectDatEntry {
    pub line: usize,
    pub ret: i32,
    pub fields: Fields,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectDat {
    pub entries: Vec<ObjectDatEntry>,
    /// Line of the `*` line that ended the read, and the number of non-skipped lines after it.
    pub stopped_at: Option<(usize, usize)>,
}

impl ObjectDat {
    pub fn parse(data: &[u8]) -> ObjectDat {
        let mut out = ObjectDat::default();
        let mut v = Vars::default();
        let lines = load_lines(data, Mode::Binary);
        for (i, l) in lines.iter().enumerate() {
            match l.first() {
                b';' | 0 => continue,
                b'*' => {
                    let rest = lines[i + 1..]
                        .iter()
                        .filter(|l| !matches!(l.first(), b';' | 0))
                        .count();
                    out.stopped_at = Some((l.number, rest));
                    break;
                }
                _ => {}
            }
            for n in SLOT_FLOATS.iter().chain(&SLOT_FLOATS2) {
                v.set(n, Val::Float(0.0));
            }
            for n in SLOT_INTS {
                v.set(n, Val::Int(0));
            }
            let ret = v.scan(&l.text, FORMAT, &NAMES);
            out.entries.push(ObjectDatEntry {
                line: l.number,
                ret,
                fields: Fields::snapshot(&v, &NAMES),
            });
        }
        out
    }
}

impl DataFile for ObjectDat {
    fn counts(&self) -> Vec<(String, u64)> {
        let n = |r: i32| self.entries.iter().filter(|e| e.ret == r).count() as u64;
        counts(vec![
            ("object.dat: entries".into(), self.entries.len() as u64),
            ("object.dat: entries with 24 fields".into(), n(24)),
            ("object.dat: entries with 17 fields (FX)".into(), n(17)),
            ("object.dat: entries with 13 fields".into(), n(13)),
            (
                "object.dat: stopped by a '*' line".into(),
                self.stopped_at.is_some() as u64,
            ),
            (
                "object.dat: lines not read after '*'".into(),
                self.stopped_at.map_or(0, |s| s.1 as u64),
            ),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for e in &self.entries {
            dump_rec!(&mut o, "obj", line = e.line, ret = e.ret);
            o.pop();
            e.fields.dump_to(&mut o);
            o.push('\n');
        }
        if let Some((n, rest)) = self.stopped_at {
            dump_rec!(&mut o, "stop", line = n, unread = rest);
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .entries
            .iter()
            .filter(|e| !matches!(e.ret, 13 | 17 | 24))
            .map(|e| {
                format!(
                    "line {}: sscanf returned {} (expected 13, 17 or 24)",
                    e.line, e.ret
                )
            })
            .collect();
        if let Some((n, rest)) = self.stopped_at {
            v.push(format!(
                "line {n}: '*' line ends the read; {rest} following lines are not loaded"
            ));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_reset_and_persistence() {
        let src = b"; c\r\nA 1.0, 2.0 3 4 5 6 7 1 2 3 4 5 1 2 3 fx 9 1 2 3 4 1 1\r\nB 10 20\r\n# x\r\n*stop\r\nC 1\r\n";
        let d = ObjectDat::parse(src);
        assert_eq!(d.entries.len(), 3);
        assert_eq!(d.entries[0].ret, 24);
        assert_eq!(d.entries[0].fields.i32("sparks_on_impact"), Some(1));
        let b = &d.entries[1].fields;
        assert_eq!(d.entries[1].ret, 3);
        assert_eq!(b.f32("turn_mass"), Some(20.0));
        // Slot fields reset, stack fields persist.
        assert_eq!(b.f32("air_resistance"), Some(0.0));
        assert_eq!(b.f32("percent_submerged"), Some(5.0));
        assert_eq!(b.bytes("fx_name"), Some(&b"fx"[..]));
        assert_eq!(d.entries[2].fields.bytes("name"), Some(&b"#"[..]));
        assert_eq!(d.stopped_at, Some((5, 1)));
    }
}
