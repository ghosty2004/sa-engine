//! `data/animgrp.dat`, as `CAnimManager::ReadAnimAssociationDefinitions` (0x5BC910) reads it.
//!
//! Opened `"rb"`, `LoadLine` (0x536F80); lines starting with `#` or NUL are skipped. Outside a
//! group, a line is read with `%s %s %s %d` (group name, animation block, model/type name, anim
//! count; no return check) and a group starts. Inside a group each line is read with `%s`; when
//! that returns 1 the token is compared with `end` over 4 bytes (exact match): `end` closes the
//! group, any other token is added as an animation name. The header variables are not reset
//! between groups.

use super::{Fields, counts};
use crate::data::{DataFile, Vars, dump_rec};
use crate::lines::{Mode, load_lines};

/// One group.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Group {
    pub line: usize,
    pub ret: i32,
    /// `name`, `block`, `kind`, `count`.
    pub header: Fields,
    /// Animation names with their line numbers.
    pub anims: Vec<(usize, Vec<u8>)>,
    /// Line of the closing `end` (`None` if the file ends inside the group).
    pub end_line: Option<usize>,
}

/// A parsed `animgrp.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimGrp {
    pub groups: Vec<Group>,
}

impl AnimGrp {
    pub fn parse(data: &[u8]) -> AnimGrp {
        let mut out = AnimGrp::default();
        let mut v = Vars::default();
        let mut tok = Vars::default();
        let mut in_group = false;
        for l in load_lines(data, Mode::Binary) {
            let t = &l.text[..];
            let c = l.first();
            if c == b'#' || c == 0 {
                continue;
            }
            if in_group {
                if tok.scan(t, "%s", &["tok"]) == 1 {
                    let s = tok.s("tok").unwrap_or_default();
                    let g = out.groups.last_mut().expect("group open");
                    if s == b"end" {
                        in_group = false;
                        g.end_line = Some(l.number);
                    } else {
                        g.anims.push((l.number, s));
                    }
                }
            } else {
                const N: [&str; 4] = ["name", "block", "kind", "count"];
                let ret = v.scan(t, "%s %s %s %d", &N);
                out.groups.push(Group {
                    line: l.number,
                    ret,
                    header: Fields::snapshot(&v, &N),
                    anims: Vec::new(),
                    end_line: None,
                });
                in_group = true;
            }
        }
        out
    }
}

impl DataFile for AnimGrp {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("animgrp.dat: groups".into(), self.groups.len() as u64),
            (
                "animgrp.dat: animations".into(),
                self.groups.iter().map(|g| g.anims.len() as u64).sum(),
            ),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for g in &self.groups {
            o.push_str(&format!("group line={} ret={}", g.line, g.ret));
            g.header.dump_to(&mut o);
            o.push('\n');
            for (n, a) in &g.anims {
                dump_rec!(&mut o, "anim", line = *n, name = a);
            }
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for g in &self.groups {
            if g.ret != 4 {
                v.push(format!(
                    "line {}: group header sscanf returned {} of 4",
                    g.line, g.ret
                ));
            }
            if g.end_line.is_none() {
                v.push(format!("line {}: group not closed by end", g.line));
            }
            if g.header.i32("count").is_some_and(|c| c as usize != g.anims.len()) {
                v.push(format!(
                    "line {}: group declares {:?} anims, lists {}",
                    g.line,
                    g.header.i32("count"),
                    g.anims.len()
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
    fn groups() {
        let a = AnimGrp::parse(b"man, ped, walkcycle, 2\r\n\twalk_civi\r\n run\r\nend\r\nx y z 0\r\nend\r\n");
        assert_eq!(a.groups.len(), 2);
        assert_eq!(a.groups[0].anims.len(), 2);
        assert_eq!(a.groups[0].header.bytes("block"), Some(&b"ped"[..]));
        assert_eq!(a.groups[1].end_line, Some(6));
        assert!(a.notes().is_empty());
    }
}
