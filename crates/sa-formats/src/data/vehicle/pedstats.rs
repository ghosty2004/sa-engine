//! `pedstats.dat`, as `CPedStats::LoadPedStats` (0x5BB890) reads it.
//!
//! Opened `"r"`, lines from `LoadLine` (0x536F80); lines starting with `#` or NUL are skipped.
//! Every other line is `sscanf(line, "%s %f %f %d %d %d %d %f %f %d %d")` without a return
//! check and becomes the next stats entry (id = running index, 0x34-byte entries): name copied
//! to +4, then +0x1C flee distance, +0x20 heading change rate, bytes +0x24..+0x27 fear, temper,
//! lawfulness, sexiness, +0x28 attack strength, +0x2C defend weakness, u16 +0x30 shooting rate,
//! byte +0x32 default decision maker. The destination locals are not reset between lines, so a
//! field `sscanf` does not reach keeps the previous line's value (`None` here only before any
//! line assigned it).

use crate::data::{DataFile, Vars, dump_rec};
use crate::lines::{Mode, load_lines};

const NAMES: [&str; 11] = [
    "name", "flee", "heading", "fear", "temper", "lawful", "sexy", "attack", "defend", "shoot", "dm",
];

/// One stats entry (the loader's variables after the line's `sscanf`).
#[derive(Debug, Clone, PartialEq)]
pub struct PedStat {
    pub line: usize,
    pub id: usize,
    pub ret: i32,
    pub name: Option<Vec<u8>>,
    pub flee_distance: Option<f32>,
    pub heading_change_rate: Option<f32>,
    pub fear: Option<i32>,
    pub temper: Option<i32>,
    pub lawfulness: Option<i32>,
    pub sexiness: Option<i32>,
    pub attack_strength: Option<f32>,
    pub defend_weakness: Option<f32>,
    pub shooting_rate: Option<i32>,
    pub default_decision_maker: Option<i32>,
}

/// A parsed `pedstats.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PedStats {
    pub stats: Vec<PedStat>,
}

impl PedStats {
    pub fn parse(data: &[u8]) -> PedStats {
        let mut out = PedStats::default();
        let mut v = Vars::default();
        for l in load_lines(data, Mode::Text) {
            if matches!(l.first(), b'#' | 0) {
                continue;
            }
            let ret = v.scan(&l.text, "%s %f %f %d %d %d %d %f %f %d %d", &NAMES);
            out.stats.push(PedStat {
                line: l.number,
                id: out.stats.len(),
                ret,
                name: v.s("name"),
                flee_distance: v.f("flee"),
                heading_change_rate: v.f("heading"),
                fear: v.i("fear"),
                temper: v.i("temper"),
                lawfulness: v.i("lawful"),
                sexiness: v.i("sexy"),
                attack_strength: v.f("attack"),
                defend_weakness: v.f("defend"),
                shooting_rate: v.i("shoot"),
                default_decision_maker: v.i("dm"),
            });
        }
        out
    }
}

impl DataFile for PedStats {
    fn counts(&self) -> Vec<(String, u64)> {
        vec![("pedstats.dat: entries".into(), self.stats.len() as u64)]
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for s in &self.stats {
            dump_rec!(
                &mut o,
                "stat",
                line = s.line,
                id = s.id,
                ret = s.ret,
                name = s.name,
                flee = s.flee_distance,
                heading = s.heading_change_rate,
                fear = s.fear,
                temper = s.temper,
                lawful = s.lawfulness,
                sexy = s.sexiness,
                attack = s.attack_strength,
                defend = s.defend_weakness,
                shoot = s.shooting_rate,
                dm = s.default_decision_maker
            );
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        self.stats
            .iter()
            .filter(|s| s.ret != 11)
            .map(|s| {
                format!(
                    "line {}: sscanf returned {} of 11 (missing fields keep the previous line's values)",
                    s.line, s.ret
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carry_over() {
        let p = PedStats::parse(b"#\r\nA 1.0 2.0 1 2 3 4 0.5 0.6 40 2\r\nB 3.0\r\n");
        assert_eq!(p.stats.len(), 2);
        assert_eq!(p.stats[1].ret, 2);
        assert_eq!(p.stats[1].id, 1);
        // Fields after the second keep the first line's values.
        assert_eq!(p.stats[1].shooting_rate, Some(40));
        assert_eq!(p.stats[1].flee_distance, Some(3.0));
        assert_eq!(p.notes().len(), 1);
    }
}
