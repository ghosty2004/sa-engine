//! `plants.dat`, as `CPlantSurfPropMgr::LoadPlantsDat` (0x5DD3B0) reads it.
//!
//! Opened `"r"` (`PLANTS.DAT` in `DATA`), lines from `LoadLine` (0x536F80). For each line:
//!
//! * if the 9 bytes from the line start equal `;the end\0` the read ends. With `"r"` the `\n`
//!   becomes a trailing space, so only a final `;the end` without a line break matches (as in
//!   the install);
//! * lines starting with `;` are skipped (empty lines are not: `strtok` then returns NULL and
//!   the surface lookup gets a null name; none occur in the install);
//! * the line is split with `strtok(line, " \t")`; token 0 is the surface name (looked up with
//!   `SurfaceInfos_c::GetSurfaceIdFromName`; an unknown name, id 0, aborts the whole load);
//!   tokens 1-10 are read with `atol` (plant slot 0-2, where values > 2 as `uint16` become 0;
//!   surface slot id; model id; UV offset; colour R, G, B; intensity; intensity variation;
//!   alpha), tokens 11-17 with `atof` cast to `float` (scale XY, scale Z, scale variation XY,
//!   scale variation Z, wind bending scale, wind bending variation, density). Further tokens
//!   are ignored. A line with fewer than 18 tokens aborts the load (`return false`, the file
//!   stays open).
//!
//! The parser keeps the tokens and their `atol`/`atof` values; the surface-name lookup needs the
//! surface table and is left to the loader port.

use super::counts;
use crate::data::{DataFile, dump_rec};
use crate::lines::{Mode, load_lines};
use crate::scanf::{atof, atoi};

pub const INT_FIELDS: [&str; 10] = [
    "plant_slot",
    "surface_slot",
    "model_id",
    "uv_offset",
    "colour_r",
    "colour_g",
    "colour_b",
    "intensity",
    "intensity_variation",
    "colour_a",
];
pub const FLOAT_FIELDS: [&str; 7] = [
    "scale_xy",
    "scale_z",
    "scale_variation_xy",
    "scale_variation_z",
    "wind_bending_scale",
    "wind_bending_variation",
    "density",
];

#[derive(Debug, Clone, PartialEq)]
pub struct PlantLine {
    pub line: usize,
    pub tokens: Vec<Vec<u8>>,
    /// `atol` of tokens 1..=10 (missing tokens `None`).
    pub ints: [Option<i32>; 10],
    /// `(float)atof` of tokens 11..=17.
    pub floats: [Option<f32>; 7],
}

impl PlantLine {
    pub fn surface(&self) -> Option<&[u8]> {
        self.tokens.first().map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Plants {
    pub lines: Vec<PlantLine>,
    /// Line of the `;the end` terminator, if reached.
    pub end_marker: Option<usize>,
    /// First line with fewer than 18 tokens (the exe aborts there; lines after it are not read).
    pub aborted_at: Option<usize>,
    /// Non-comment lines that are empty after `LoadLine` (the exe would look up a null name).
    pub empty_lines: Vec<usize>,
}

impl Plants {
    pub fn parse(data: &[u8]) -> Plants {
        let mut out = Plants::default();
        for l in load_lines(data, Mode::Text) {
            let mut t = l.text.clone();
            t.push(0);
            if t.len() >= 9 && &t[..9] == b";the end\0" {
                out.end_marker = Some(l.number);
                break;
            }
            if l.first() == b';' {
                continue;
            }
            let tokens: Vec<Vec<u8>> = l
                .text
                .split(|&b| b == b' ' || b == b'\t')
                .filter(|s| !s.is_empty())
                .map(<[u8]>::to_vec)
                .collect();
            if tokens.is_empty() {
                out.empty_lines.push(l.number);
                continue;
            }
            let ints = std::array::from_fn(|i| tokens.get(i + 1).map(|s| atoi(s)));
            let floats = std::array::from_fn(|i| tokens.get(i + 11).map(|s| atof(s) as f32));
            let short = tokens.len() < 18;
            out.lines.push(PlantLine {
                line: l.number,
                tokens,
                ints,
                floats,
            });
            if short {
                out.aborted_at = Some(l.number);
                break;
            }
        }
        out
    }
}

impl DataFile for Plants {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("plants.dat: entries".into(), self.lines.len() as u64),
            (
                "plants.dat: end marker reached".into(),
                self.end_marker.is_some() as u64,
            ),
            (
                "plants.dat: aborted (short line)".into(),
                self.aborted_at.is_some() as u64,
            ),
            ("plants.dat: empty lines".into(), self.empty_lines.len() as u64),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for p in &self.lines {
            dump_rec!(
                &mut o,
                "plant",
                line = p.line,
                surface = p.surface(),
                ntokens = p.tokens.len()
            );
            o.pop();
            for (n, v) in INT_FIELDS.iter().zip(&p.ints) {
                o.push_str(&format!(" {n}="));
                crate::data::DumpVal::dump_to(v, &mut o);
            }
            for (n, v) in FLOAT_FIELDS.iter().zip(&p.floats) {
                o.push_str(&format!(" {n}="));
                crate::data::DumpVal::dump_to(v, &mut o);
            }
            o.push('\n');
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(n) = self.aborted_at {
            v.push(format!(
                "line {n}: fewer than 18 tokens: the exe aborts the load here"
            ));
        }
        for n in &self.empty_lines {
            v.push(format!(
                "line {n}: empty line: the exe would look up a null surface name"
            ));
        }
        for p in &self.lines {
            if p.tokens.len() > 18 {
                v.push(format!(
                    "line {}: {} tokens, the extra ones are ignored",
                    p.line,
                    p.tokens.len()
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
    fn tokens_and_end() {
        let full = "GRASS\t0 1 2 3 4 5 6 7 8 9 0.5 1.5 2.5 3.5 4.5 5.5 6.5\n";
        let src = format!(";c\n{full};the end \nX 1\n;the end");
        let p = Plants::parse(src.as_bytes());
        // ";the end " (with the newline's space) does not match; "X 1" aborts.
        assert_eq!(p.lines.len(), 2);
        assert_eq!(p.lines[0].ints[2], Some(2));
        assert_eq!(p.lines[0].floats[6], Some(6.5));
        assert_eq!(p.aborted_at, Some(4));
        assert_eq!(p.end_marker, None);
        let p = Plants::parse(format!("{full};the end").as_bytes());
        assert_eq!(p.end_marker, Some(2));
    }
}
