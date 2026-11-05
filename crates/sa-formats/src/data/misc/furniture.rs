//! `data/furnitur.dat`, as `FurnitureManager_c::LoadFurniture` (0x5C0280) reads it.
//!
//! Opened `"rb"`, `LoadLine` (0x536F80); lines starting with `#` or NUL are skipped. Each other
//! line is first read with `%s`; the token is compared (whole string, including the NUL) with
//! `GROUP:`, `SUBGROUP:` and `ITEM:`:
//!
//! * `GROUP:` -> `%s %s` (tag, group name);
//! * `SUBGROUP:` -> `%s %s %d %d %d %d %d %d %d` (tag, subgroup name, seven integers passed to
//!   the subgroup constructor in this order);
//! * `ITEM:` -> `%s %s %d %d %d %d` (tag, model name, four integers); an unknown model name
//!   only formats a "Cannot find model" message;
//! * any other token: line ignored.
//!
//! No return value is checked; the variables are function locals kept between lines.

use super::{Fields, Rec, counts};
use crate::data::{DataFile, Vars};
use crate::lines::{Mode, load_lines};

const SUB_NAMES: [&str; 9] = ["tag", "subgroup", "i0", "i1", "i2", "i3", "i4", "i5", "i6"];
const ITEM_NAMES: [&str; 6] = ["tag", "model", "i0", "i1", "i2", "i3"];

/// Record kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Group,
    SubGroup,
    Item,
}

/// A parsed `furnitur.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Furniture {
    pub records: Vec<(Kind, Rec)>,
    pub ignored: Vec<(usize, Vec<u8>)>,
}

impl Furniture {
    pub fn parse(data: &[u8]) -> Furniture {
        let mut out = Furniture::default();
        let mut v = Vars::default();
        // The tag buffer and the subgroup/item numbers are separate locals; keep them apart.
        let mut tag = Vars::default();
        for l in load_lines(data, Mode::Binary) {
            let t = &l.text[..];
            let c = l.first();
            if c == b'#' || c == 0 {
                continue;
            }
            tag.scan(t, "%s", &["tag"]);
            let tok = tag.s("tag").unwrap_or_default();
            let (kind, ret, names): (Kind, i32, &[&'static str]) = match &tok[..] {
                b"GROUP:" => (
                    Kind::Group,
                    v.scan(t, "%s %s", &["tag", "group"]),
                    &["tag", "group"],
                ),
                b"SUBGROUP:" => (
                    Kind::SubGroup,
                    v.scan(t, "%s %s %d %d %d %d %d %d %d", &SUB_NAMES),
                    &SUB_NAMES,
                ),
                b"ITEM:" => (
                    Kind::Item,
                    v.scan(t, "%s %s %d %d %d %d", &["tag", "model", "j0", "j1", "j2", "j3"]),
                    &ITEM_NAMES,
                ),
                _ => {
                    out.ignored.push((l.number, t.to_vec()));
                    continue;
                }
            };
            let fields = if kind == Kind::Item {
                // Item integers are different locals from the subgroup ones.
                let mut f = Fields::snapshot(&v, &["tag", "model"]);
                for (n, src) in [("i0", "j0"), ("i1", "j1"), ("i2", "j2"), ("i3", "j3")] {
                    f.0.push((n, v.get(src).cloned()));
                }
                f
            } else {
                Fields::snapshot(&v, names)
            };
            out.records.push((
                kind,
                Rec {
                    line: l.number,
                    ret,
                    fields,
                },
            ));
        }
        out
    }
}

impl DataFile for Furniture {
    fn counts(&self) -> Vec<(String, u64)> {
        let n = |k: Kind| self.records.iter().filter(|(x, _)| *x == k).count() as u64;
        counts(vec![
            ("furnitur.dat: groups".into(), n(Kind::Group)),
            ("furnitur.dat: subgroups".into(), n(Kind::SubGroup)),
            ("furnitur.dat: items".into(), n(Kind::Item)),
            ("furnitur.dat: ignored lines".into(), self.ignored.len() as u64),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for (k, r) in &self.records {
            let kind = match k {
                Kind::Group => "group",
                Kind::SubGroup => "subgroup",
                Kind::Item => "item",
            };
            r.dump_to(kind, &mut o);
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .records
            .iter()
            .filter_map(|(k, r)| {
                let want = match k {
                    Kind::Group => 2,
                    Kind::SubGroup => 9,
                    Kind::Item => 6,
                };
                (r.ret != want).then(|| format!("line {}: {k:?} sscanf returned {} of {want}", r.line, r.ret))
            })
            .collect();
        for (n, t) in &self.ignored {
            v.push(format!(
                "line {n}: no GROUP:/SUBGROUP:/ITEM: tag, ignored: {:?}",
                crate::data::lossy(t)
            ));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        let f = Furniture::parse(
            b"GROUP: IT_SHOP\r\nSUBGROUP: S1\t1 1 1 1 0 0 0\r\nITEM:\tshlf\t1\t0\t100\t0\r\nGROUPX y\r\n",
        );
        assert_eq!(f.records.len(), 3);
        assert_eq!(f.records[1].1.ret, 9);
        assert_eq!(f.records[2].1.fields.i32("i2"), Some(100));
        assert_eq!(f.ignored.len(), 1);
    }
}
