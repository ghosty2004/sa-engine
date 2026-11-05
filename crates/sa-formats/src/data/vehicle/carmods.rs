//! `carmods.dat`, as `CVehicleModelInfo::LoadVehicleUpgrades` (0x5B65A0) reads it.
//!
//! Opened `"r"`, lines from `LoadLine` (0x536F80). Lines starting with `#` or NUL are skipped.
//! Outside a section, `strncmp` with `link` (4), `mods` (4), `wheel` (5) opens a section; other
//! lines are ignored. Inside, `end` (3) closes it. Lines are split with `strtok(line, " \t,")`:
//!
//! * `link`: two model names (upgrade pair: `CModelInfo::GetModelInfo` on each, then linked);
//! * `mods`: vehicle model name, then any number of upgrade model names (stored in order from
//!   offset 0x2D6 of the vehicle model info);
//! * `wheel`: `sscanf(line, "%d")` gives the wheel set; the first `strtok` token is skipped, the
//!   others are wheel model names.

use crate::data::{DataFile, dump_bytes, dump_rec};
use crate::lines::{Mode, load_lines};
use crate::scanf::sscanf;

/// One data line.
#[derive(Debug, Clone, PartialEq)]
pub enum CarModLine {
    Link {
        line: usize,
        tokens: Vec<Vec<u8>>,
    },
    Mods {
        line: usize,
        tokens: Vec<Vec<u8>>,
    },
    /// `set` is the `%d` result (`None` if `sscanf` assigned nothing); `tokens` include the
    /// skipped first token.
    Wheel {
        line: usize,
        set: Option<i32>,
        tokens: Vec<Vec<u8>>,
    },
}

/// A parsed `carmods.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CarMods {
    pub lines: Vec<CarModLine>,
}

impl CarMods {
    pub fn parse(data: &[u8]) -> CarMods {
        let mut out = CarMods::default();
        let mut section = 0;
        for l in load_lines(data, Mode::Text) {
            let t = &l.text[..];
            if matches!(l.first(), b'#' | 0) {
                continue;
            }
            if section == 0 {
                if t.starts_with(b"link") {
                    section = 1;
                } else if t.starts_with(b"mods") {
                    section = 2;
                } else if t.starts_with(b"wheel") {
                    section = 3;
                }
                continue;
            }
            if t.starts_with(b"end") {
                section = 0;
                continue;
            }
            let tokens: Vec<Vec<u8>> = super::strtok(t, b" \t,")
                .into_iter()
                .map(<[u8]>::to_vec)
                .collect();
            let line = l.number;
            out.lines.push(match section {
                1 => CarModLine::Link { line, tokens },
                2 => CarModLine::Mods { line, tokens },
                _ => CarModLine::Wheel {
                    line,
                    set: sscanf(t, "%d").i32(0),
                    tokens,
                },
            });
        }
        out
    }
}

fn dump_tokens(t: &[Vec<u8>], o: &mut String) {
    o.pop();
    o.push_str(" tokens=[");
    for (i, x) in t.iter().enumerate() {
        if i > 0 {
            o.push(' ');
        }
        dump_bytes(x, o);
    }
    o.push_str("]\n");
}

impl DataFile for CarMods {
    fn counts(&self) -> Vec<(String, u64)> {
        let (mut a, mut b, mut c) = (0, 0, 0);
        for l in &self.lines {
            match l {
                CarModLine::Link { .. } => a += 1,
                CarModLine::Mods { .. } => b += 1,
                CarModLine::Wheel { .. } => c += 1,
            }
        }
        vec![
            ("carmods.dat: link lines".into(), a),
            ("carmods.dat: mods lines".into(), b),
            ("carmods.dat: wheel lines".into(), c),
        ]
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for l in &self.lines {
            match l {
                CarModLine::Link { line, tokens } => {
                    dump_rec!(&mut o, "link", line = *line);
                    dump_tokens(tokens, &mut o);
                }
                CarModLine::Mods { line, tokens } => {
                    dump_rec!(&mut o, "mods", line = *line);
                    dump_tokens(tokens, &mut o);
                }
                CarModLine::Wheel { line, set, tokens } => {
                    dump_rec!(&mut o, "wheel", line = *line, set = *set);
                    dump_tokens(tokens, &mut o);
                }
            }
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for l in &self.lines {
            match l {
                CarModLine::Link { line, tokens } if tokens.len() != 2 => v.push(format!(
                    "line {line}: link line has {} tokens (2 read)",
                    tokens.len()
                )),
                CarModLine::Wheel { line, set: None, .. } => {
                    v.push(format!("line {line}: wheel line without a set number"))
                }
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
    fn sections() {
        let src = b"# c\r\nlink\r\na, b\r\nend\r\nmods\r\nadmiral, nto_b_l, nto_b_s\r\nend\r\nwheel\r\n1, wheel_sr6, wheel_sr3\r\nend\r\n";
        let m = CarMods::parse(src);
        assert_eq!(m.lines.len(), 3);
        assert!(matches!(&m.lines[1], CarModLine::Mods { tokens, .. } if tokens.len() == 3));
        assert!(matches!(&m.lines[2], CarModLine::Wheel { set: Some(1), tokens, .. } if tokens.len() == 3));
        assert!(m.notes().is_empty());
    }
}
