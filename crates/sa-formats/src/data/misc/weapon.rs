//! `data/weapon.dat`, as `CWeaponInfo::LoadWeaponData` (0x5BE670) reads it.
//!
//! Opened `"rb"`, lines from `LoadLine` (0x536F80); lines starting with NUL or `#` are skipped.
//! Before each line the first byte of four name buffers (weapon name, fire type, combo/anim
//! group buffer and stealth/anim group buffer) is set to NUL; the numeric variables are not
//! reset (except speed, radius, lifespan and spread, which are set to 0 before every `$` line).
//! The first byte selects the record:
//!
//! * `\xA3` (`£`, melee): `%s %s %s %f %f %d %d %d %s %d %x %s`
//!   (tag, name, fire type, target range, weapon range, model 1, model 2, slot, base combo,
//!   combo count, flags, stealth anim group);
//! * `$` (gun): `%s %s %s %f %f %d %d %d %s %d %d %f %f %f %d %d %f %f %d %d %d %d %d %d %d %x %f %f %f %f`
//!   (tag, name, fire type, target range, weapon range, model 1, model 2, slot, anim group,
//!   ammo clip, damage, fire offset xyz, skill, required stat level, accuracy, move speed,
//!   anim loop start/end/fire, anim loop 2 start/end/fire, breakout time, flags, speed, radius,
//!   lifespan, spread);
//! * `%` (aim offsets): `%s %s %f %f %f %f %d %d %d %d`
//!   (tag, anim group, aim x, aim z, duck x, duck z, reload a, reload b, crouch reload a/b);
//! * anything else: `%s` into the weapon name buffer; the read ends when its first 13 bytes are
//!   `ENDWEAPONDATA`.
//!
//! gta-reversed reads the tag with `%*s` and searches `ENDWEAPONDATA` anywhere in the line; the
//! original stores the tag and compares only the first token's prefix. The original never checks
//! a `sscanf` return value.

use super::{Fields, Rec, counts};
use crate::data::{DataFile, Vars};
use crate::lines::{Mode, load_lines};
use crate::scanf::Val;

pub const FMT_MELEE: &str = "%s %s %s %f %f %d %d %d %s %d %x %s";
pub const FMT_GUN: &str =
    "%s %s %s %f %f %d %d %d %s %d %d %f %f %f %d %d %f %f %d %d %d %d %d %d %d %x %f %f %f %f";
pub const FMT_AIM: &str = "%s %s %f %f %f %f %d %d %d %d";

const MELEE_NAMES: [&str; 12] = [
    "tag",
    "name",
    "fire_type",
    "target_range",
    "range",
    "model1",
    "model2",
    "slot",
    "combo",
    "num_combos",
    "flags",
    "anim_group",
];
const GUN_NAMES: [&str; 30] = [
    "tag",
    "name",
    "fire_type",
    "target_range",
    "range",
    "model1",
    "model2",
    "slot",
    "anim_group",
    "ammo",
    "damage",
    "offset_x",
    "offset_y",
    "offset_z",
    "skill",
    "req_stat",
    "accuracy",
    "move_speed",
    "loop_start",
    "loop_end",
    "loop_fire",
    "loop2_start",
    "loop2_end",
    "loop2_fire",
    "breakout",
    "flags",
    "speed",
    "radius",
    "lifespan",
    "spread",
];
const AIM_NAMES: [&str; 10] = [
    "tag",
    "anim_group",
    "aim_x",
    "aim_z",
    "duck_x",
    "duck_z",
    "rload_a",
    "rload_b",
    "crouch_rload_a",
    "crouch_rload_b",
];

/// Record kind (first byte of the line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Melee,
    Gun,
    Aim,
}

/// A parsed `weapon.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WeaponDat {
    pub records: Vec<(Kind, Rec)>,
    /// Other lines (read with `%s`), with their first token.
    pub other: Vec<(usize, Option<Vec<u8>>)>,
    /// Line of `ENDWEAPONDATA`, if reached.
    pub end_line: Option<usize>,
}

impl WeaponDat {
    pub fn parse(data: &[u8]) -> WeaponDat {
        let mut out = WeaponDat::default();
        let mut v = Vars::default();
        for l in load_lines(data, Mode::Binary) {
            let t = &l.text[..];
            let c = l.first();
            if c == 0 || c == b'#' {
                continue;
            }
            for n in ["name", "fire_type", "combo", "anim_group"] {
                v.set(n, Val::Str(Vec::new()));
            }
            match c {
                0xA3 => {
                    let ret = v.scan(t, FMT_MELEE, &MELEE_NAMES);
                    out.records
                        .push((Kind::Melee, rec(l.number, ret, &v, &MELEE_NAMES)));
                }
                b'$' => {
                    for n in ["speed", "radius", "lifespan", "spread"] {
                        v.set(n, Val::Float(0.0));
                    }
                    let ret = v.scan(t, FMT_GUN, &GUN_NAMES);
                    out.records.push((Kind::Gun, rec(l.number, ret, &v, &GUN_NAMES)));
                }
                b'%' => {
                    let ret = v.scan(t, FMT_AIM, &AIM_NAMES);
                    out.records.push((Kind::Aim, rec(l.number, ret, &v, &AIM_NAMES)));
                }
                _ => {
                    v.scan(t, "%s", &["name"]);
                    let tok = v.s("name");
                    out.other.push((l.number, tok.clone()));
                    if tok.is_some_and(|t| t.starts_with(b"ENDWEAPONDATA")) {
                        out.end_line = Some(l.number);
                        break;
                    }
                }
            }
        }
        out
    }
}

fn rec(line: usize, ret: i32, v: &Vars, names: &[&'static str]) -> Rec {
    Rec {
        line,
        ret,
        fields: Fields::snapshot(v, names),
    }
}

impl DataFile for WeaponDat {
    fn counts(&self) -> Vec<(String, u64)> {
        let n = |k: Kind| self.records.iter().filter(|(x, _)| *x == k).count() as u64;
        counts(vec![
            ("weapon.dat: gun ($)".into(), n(Kind::Gun)),
            ("weapon.dat: melee (\u{a3})".into(), n(Kind::Melee)),
            ("weapon.dat: aim offsets (%)".into(), n(Kind::Aim)),
            ("weapon.dat: other lines".into(), self.other.len() as u64),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for (k, r) in &self.records {
            let kind = match k {
                Kind::Melee => "melee",
                Kind::Gun => "gun",
                Kind::Aim => "aim",
            };
            r.dump_to(kind, &mut o);
        }
        if let Some(n) = self.end_line {
            o.push_str(&format!("end line={n}\n"));
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for (k, r) in &self.records {
            let want = match k {
                Kind::Melee => 12,
                Kind::Gun => 30,
                Kind::Aim => 10,
            };
            // Guns may omit the last four optional floats (pre-set to 0).
            let ok = if *k == Kind::Gun {
                r.ret >= 26
            } else {
                r.ret == want
            };
            if !ok {
                v.push(format!(
                    "line {}: {k:?} sscanf returned {} of {want}",
                    r.line, r.ret
                ));
            }
        }
        if self.end_line.is_none() {
            v.push("no ENDWEAPONDATA line: read to the end of the file".into());
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records() {
        let src = b"# c\r\n$ PISTOL INSTANT_HIT 35.0 35.0 346 -1 2 colt45 17 25 0.11 0.04 0.13 0 40 0.5 1.0 \
16 32 24 16 32 24 10 3010\r\n\xa3 UNARMED MELEE 10.0 1.6 -1 -1 0 UNARMED 4 1 null\r\n\
% colt45 0.1 0.2 0.3 0.4 1 2 3 4\r\nENDWEAPONDATA\r\n$ X\r\n";
        let w = WeaponDat::parse(src);
        assert_eq!(w.records.len(), 3);
        let (k, g) = &w.records[0];
        assert_eq!(*k, Kind::Gun);
        assert_eq!(g.ret, 26);
        assert_eq!(g.fields.u32("flags"), Some(0x3010));
        assert_eq!(g.fields.f32("speed"), Some(0.0));
        assert_eq!(w.records[1].1.fields.u32("flags"), Some(1));
        assert_eq!(w.records[1].1.fields.bytes("anim_group"), Some(&b"null"[..]));
        assert_eq!(w.records[2].1.ret, 10);
        assert_eq!(w.end_line, Some(5));
    }
}
