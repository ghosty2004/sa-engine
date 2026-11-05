//! `data/melee.dat`, as the melee data loader at 0x5BEDC0 reads it (`CTaskSimpleFight` combo
//! table at 0xC170D0; not in gta-reversed).
//!
//! Opened `"rb"`, lines from `LoadLine` (0x536F80); lines starting with `#` or NUL are skipped.
//! A line starting with `END_MELEE_DATA` (14 bytes) ends the read. Outside a block,
//! `START_COMBO` (11) opens a combo block and `START_LEVELS` (12) a hit-level block; other lines
//! are ignored. Inside a block, `END_COMBO` (9) closes it (the combo index advances only for
//! combo blocks) and resets the line index; every other line uses the line index (0-based,
//! counted per block, the `START_*` line not included):
//!
//! * level block: `%s %f %f %f` for every line (level `index`);
//! * combo block: 0 `%s %s` (anim group), 1 `%s %f` (range), 2..=6 `%s %f %f %f %s %d %d %d %f`
//!   (attack 1..3, ground attack, moving attack: hit time, chain time, radius, hit level, damage,
//!   hit anim, alt hit anim, ground loop; the ground loop variable is reset to 0 before each of
//!   these lines), 7 `%s %f %f` (block), 8 `%s %x` (flags); lines from index 9 on are counted but
//!   not read.
//!
//! The scan variables are function locals that are not reset between lines (apart from the
//! ground loop), so a short line keeps earlier values; the records here hold the `sscanf`
//! results per line.

use super::counts;
use crate::data::{DataFile, ScanRec};
use crate::lines::{Mode, load_lines};

/// Where a line belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    /// Hit-level block, with the line index.
    Levels(usize),
    /// Combo number (0-based among combo blocks) and line index inside the combo.
    Combo(usize, usize),
}

/// A parsed `melee.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeleeDat {
    /// Data lines in file order. Combo lines with index >= 9 have no scan.
    pub lines: Vec<(Block, ScanRec)>,
    pub combos: usize,
    /// Line of `END_MELEE_DATA`, if reached.
    pub end_line: Option<usize>,
}

pub const FMT_LEVEL: &str = "%s %f %f %f";
pub const FMT_ATTACK: &str = "%s %f %f %f %s %d %d %d %f";

impl MeleeDat {
    pub fn parse(data: &[u8]) -> MeleeDat {
        let mut out = MeleeDat::default();
        let (mut in_combo, mut in_levels) = (false, false);
        let mut index = 0usize;
        for l in load_lines(data, Mode::Binary) {
            let t = &l.text[..];
            let c = l.first();
            if c == b'#' || c == 0 {
                continue;
            }
            if t.starts_with(b"END_MELEE_DATA") {
                out.end_line = Some(l.number);
                break;
            }
            if !in_combo && !in_levels {
                if t.starts_with(b"START_COMBO") {
                    in_combo = true;
                } else if t.starts_with(b"START_LEVELS") {
                    in_levels = true;
                }
                continue;
            }
            if t.starts_with(b"END_COMBO") {
                if in_combo {
                    out.combos += 1;
                }
                index = 0;
                in_combo = false;
                in_levels = false;
                continue;
            }
            let block = if in_levels {
                Block::Levels(index)
            } else {
                Block::Combo(out.combos, index)
            };
            let fmt: Option<&'static str> = if in_levels {
                Some(FMT_LEVEL)
            } else {
                match index {
                    0 => Some("%s %s"),
                    1 => Some("%s %f"),
                    2..=6 => Some(FMT_ATTACK),
                    7 => Some("%s %f %f"),
                    8 => Some("%s %x"),
                    _ => None,
                }
            };
            let fmts: Vec<&'static str> = fmt.into_iter().collect();
            out.lines.push((block, ScanRec::run(l.number, t, &fmts)));
            index += 1;
        }
        out
    }
}

impl DataFile for MeleeDat {
    fn counts(&self) -> Vec<(String, u64)> {
        let levels = self
            .lines
            .iter()
            .filter(|(b, _)| matches!(b, Block::Levels(_)))
            .count();
        counts(vec![
            ("melee.dat: combos".into(), self.combos as u64),
            (
                "melee.dat: combo lines".into(),
                (self.lines.len() - levels) as u64,
            ),
            ("melee.dat: level lines".into(), levels as u64),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for (b, r) in &self.lines {
            match b {
                Block::Levels(i) => o.push_str(&format!("level index={i} line={}", r.line)),
                Block::Combo(c, i) => o.push_str(&format!("combo combo={c} index={i} line={}", r.line)),
            }
            r.dump_to(&mut o);
            o.push('\n');
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for (b, r) in &self.lines {
            let Some((fmt, s)) = r.scans.first() else {
                v.push(format!("line {}: combo line index >= 9, not read", r.line));
                continue;
            };
            let want = s.vals.len() as i32;
            let ok = if *fmt == FMT_ATTACK {
                s.ret >= 8
            } else {
                s.ret == want
            };
            if !ok {
                v.push(format!(
                    "line {}: {b:?} sscanf {fmt:?} returned {} of {want}",
                    r.line, s.ret
                ));
            }
        }
        if self.end_line.is_none() {
            v.push("no END_MELEE_DATA line".into());
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks() {
        let src = b"START_LEVELS\r\n LEVEL_H 0.0 0.7 0.5\r\nEND_COMBO\r\nignored\r\n\
START_COMBO\tX\r\n ANIMGROUP melee_1\r\n RANGES 1.6\r\n # c\r\n ATTACK1 5.0 10.0 0.4 H 6 5 6\r\nEND_COMBO\r\nEND_MELEE_DATA\r\n";
        let m = MeleeDat::parse(src);
        assert_eq!(m.combos, 1);
        assert_eq!(m.lines.len(), 4);
        assert_eq!(m.lines[0].0, Block::Levels(0));
        assert_eq!(m.lines[3].0, Block::Combo(0, 2));
        assert_eq!(m.lines[3].1.scans[0].1.ret, 8);
        assert_eq!(m.end_line, Some(11));
    }
}
