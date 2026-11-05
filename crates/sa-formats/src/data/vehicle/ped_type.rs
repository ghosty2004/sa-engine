//! `ped.dat` (ped type relationships), as `CPedType::LoadPedData` (0x608B30) reads it.
//!
//! Opened `"r"`, lines from `LoadLine` (0x536F80); lines starting with `#` or NUL are skipped.
//! `sscanf(line, "%s")` reads the first word. If it is exactly `Hate`, `Dislike`, `Like` or
//! `Respect`, the line is split with `strtok(line, " ,\t")`, the first token skipped, and each
//! further token is a ped type name (`CPedType::FindPedType`, 0x608790); bit `type` (only for
//! `type < 32`) is set in a mask that **replaces** the current type's acquaintance word
//! (offsets in the 0x14-byte entry: Respect +0, Like +4, Dislike +0xC, Hate +0x10). Any other
//! word selects the current ped type (`FindPedType(word)`). Before the first type line the
//! current type index is 32 (an out-of-range slot).

use crate::data::{DataFile, dump_bytes, dump_rec};
use crate::lines::{Mode, load_lines};
use crate::scanf::sscanf;

/// Relationship kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Respect,
    Like,
    Dislike,
    Hate,
}

impl Relation {
    pub fn name(self) -> &'static str {
        match self {
            Relation::Respect => "Respect",
            Relation::Like => "Like",
            Relation::Dislike => "Dislike",
            Relation::Hate => "Hate",
        }
    }
    /// Offset of the mask in the exe's 0x14-byte acquaintance entry.
    pub fn offset(self) -> u32 {
        match self {
            Relation::Respect => 0,
            Relation::Like => 4,
            Relation::Dislike => 0xC,
            Relation::Hate => 0x10,
        }
    }
}

/// One data line.
#[derive(Debug, Clone, PartialEq)]
pub enum PedTypeLine {
    /// Selects the current ped type.
    Type { line: usize, name: Vec<u8> },
    /// Sets the current type's mask for `relation` to the listed types.
    Relation {
        line: usize,
        relation: Relation,
        types: Vec<Vec<u8>>,
    },
}

/// A parsed `ped.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PedTypes {
    pub lines: Vec<PedTypeLine>,
}

impl PedTypes {
    pub fn parse(data: &[u8]) -> PedTypes {
        let mut out = PedTypes::default();
        for l in load_lines(data, Mode::Text) {
            if matches!(l.first(), b'#' | 0) {
                continue;
            }
            let word = sscanf(&l.text, "%s").string(0).unwrap_or_default();
            let rel = match &word[..] {
                b"Hate" => Some(Relation::Hate),
                b"Dislike" => Some(Relation::Dislike),
                b"Like" => Some(Relation::Like),
                b"Respect" => Some(Relation::Respect),
                _ => None,
            };
            out.lines.push(match rel {
                Some(relation) => PedTypeLine::Relation {
                    line: l.number,
                    relation,
                    types: super::strtok(&l.text, b" ,\t")
                        .into_iter()
                        .skip(1)
                        .map(<[u8]>::to_vec)
                        .collect(),
                },
                None => PedTypeLine::Type {
                    line: l.number,
                    name: word,
                },
            });
        }
        out
    }
}

impl DataFile for PedTypes {
    fn counts(&self) -> Vec<(String, u64)> {
        let t = self
            .lines
            .iter()
            .filter(|l| matches!(l, PedTypeLine::Type { .. }))
            .count() as u64;
        vec![
            ("ped.dat: ped types".into(), t),
            ("ped.dat: relation lines".into(), self.lines.len() as u64 - t),
        ]
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for l in &self.lines {
            match l {
                PedTypeLine::Type { line, name } => dump_rec!(&mut o, "type", line = *line, name = name),
                PedTypeLine::Relation {
                    line,
                    relation,
                    types,
                } => {
                    dump_rec!(&mut o, relation.name(), line = *line);
                    o.pop();
                    o.push_str(" types=[");
                    for (i, t) in types.iter().enumerate() {
                        if i > 0 {
                            o.push(' ');
                        }
                        dump_bytes(t, &mut o);
                    }
                    o.push_str("]\n");
                }
            }
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        let mut seen_type = false;
        for l in &self.lines {
            match l {
                PedTypeLine::Type { .. } => seen_type = true,
                PedTypeLine::Relation { line, .. } if !seen_type => v.push(format!(
                    "line {line}: relation before any type line (written to slot 32)"
                )),
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
    fn relations() {
        let p = PedTypes::parse(b"# x\r\nCIVMALE\t\r\n    Respect CIVMALE,COP\r\n  Hates X\r\n");
        assert_eq!(p.lines.len(), 3);
        assert!(
            matches!(&p.lines[1], PedTypeLine::Relation { relation: Relation::Respect, types, .. } if types.len() == 2)
        );
        // "Hates" is not "Hate": it selects a ped type.
        assert!(matches!(&p.lines[2], PedTypeLine::Type { name, .. } if name == b"Hates"));
    }
}
