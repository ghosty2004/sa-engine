//! `water.dat` / `water1.dat`, as `CWaterLevel::WaterLevelInitialise` (0x6EAE80) reads them.
//!
//! The loader opens `DATA\water.dat` (configuration 0) or `DATA\water1.dat` (configuration 1)
//! with mode `"rb"` (gta-reversed: `"r"` and `DATA//water.dat`), lines from `LoadLine`
//! (0x536F80). Lines whose first byte is `;`, NUL, `*` or `p` (the `processed` header) are
//! skipped. For every other line the flag variable is set to 1, then:
//!
//! 1. `sscanf` with 28 `%f` + `%d` ([`QUAD_FLAGS`]); if it returns 29 the line is a quad;
//! 2. else `sscanf` with 28 `%f` ([`QUAD`]); if it returns 28 the line is a quad (flags stay 1
//!    or keep what the first call assigned);
//! 3. else `sscanf` with 21 `%f` + `%d` ([`TRIANGLE`]) without a return check: a triangle.
//!
//! Each vertex has 7 floats (x, y, z, then four per-vertex wave/flow values). The float
//! destinations are function locals that persist from line to line.

use super::counts;
use crate::data::Fields;
use crate::data::{DataFile, Vars, dump_rec};
use crate::lines::{Mode, load_lines};
use crate::scanf::Val;

pub const QUAD_FLAGS: &str =
    "%f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %d";
pub const QUAD: &str = "%f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f";
pub const TRIANGLE: &str = "%f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %f %d";

const V: [&str; 28] = [
    "v0_0", "v0_1", "v0_2", "v0_3", "v0_4", "v0_5", "v0_6", "v1_0", "v1_1", "v1_2", "v1_3", "v1_4", "v1_5",
    "v1_6", "v2_0", "v2_1", "v2_2", "v2_3", "v2_4", "v2_5", "v2_6", "v3_0", "v3_1", "v3_2", "v3_3", "v3_4",
    "v3_5", "v3_6",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// 29-field quad (with flags).
    QuadFlags,
    /// 28-field quad (flags stay as preset).
    Quad,
    /// Triangle form (third format).
    Triangle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaterEntry {
    pub line: usize,
    pub shape: Shape,
    /// Return values of the `sscanf` calls made, in order.
    pub rets: Vec<i32>,
    /// Vertex floats (`vN_k`) and `flags` after the calls; a triangle uses `v0`..`v2`.
    pub fields: Fields,
}

impl WaterEntry {
    /// Vertex `n`'s 7 floats.
    pub fn vertex(&self, n: usize) -> [Option<f32>; 7] {
        std::array::from_fn(|k| self.fields.f32(V[n * 7 + k]))
    }
    pub fn flags(&self) -> Option<i32> {
        self.fields.i32("flags")
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Water {
    pub entries: Vec<WaterEntry>,
    /// Skipped lines by first byte (`;`, `*`, `p`), not counting empty lines.
    pub skipped: usize,
}

impl Water {
    pub fn parse(data: &[u8]) -> Water {
        let mut out = Water::default();
        let mut v = Vars::default();
        let mut quad_names: Vec<&'static str> = V.to_vec();
        quad_names.push("flags");
        let mut tri_names: Vec<&'static str> = V[..21].to_vec();
        tri_names.push("flags");
        for l in load_lines(data, Mode::Binary) {
            match l.first() {
                0 => continue,
                b';' | b'*' | b'p' => {
                    out.skipped += 1;
                    continue;
                }
                _ => {}
            }
            v.set("flags", Val::Int(1));
            let mut rets = vec![v.scan(&l.text, QUAD_FLAGS, &quad_names)];
            let shape = if rets[0] == 29 {
                Shape::QuadFlags
            } else {
                rets.push(v.scan(&l.text, QUAD, &V));
                if rets[1] == 28 {
                    Shape::Quad
                } else {
                    rets.push(v.scan(&l.text, TRIANGLE, &tri_names));
                    Shape::Triangle
                }
            };
            out.entries.push(WaterEntry {
                line: l.number,
                shape,
                rets,
                fields: Fields::snapshot(&v, &quad_names),
            });
        }
        out
    }
}

impl DataFile for Water {
    fn counts(&self) -> Vec<(String, u64)> {
        let n = |s: Shape| self.entries.iter().filter(|e| e.shape == s).count() as u64;
        counts(vec![
            ("water: quads (29 fields)".into(), n(Shape::QuadFlags)),
            ("water: quads (28 fields)".into(), n(Shape::Quad)),
            ("water: triangles".into(), n(Shape::Triangle)),
            ("water: skipped lines".into(), self.skipped as u64),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for e in &self.entries {
            let kind = match e.shape {
                Shape::QuadFlags => "quad29",
                Shape::Quad => "quad28",
                Shape::Triangle => "tri",
            };
            let n = if e.shape == Shape::Triangle { 3 } else { 4 };
            dump_rec!(&mut o, kind, line = e.line, flags = e.flags());
            o.pop();
            for k in 0..n {
                o.push_str(&format!(" v{k}="));
                crate::data::DumpVal::dump_to(&e.vertex(k), &mut o);
            }
            o.push('\n');
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.shape == Shape::Triangle && e.rets[2] != 22)
            .map(|e| {
                format!(
                    "line {}: triangle sscanf returned {} of 22 (flags keep the preset 1)",
                    e.line, e.rets[2]
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes() {
        let q: String = (0..28).map(|i| format!("{i}.0 ")).collect();
        let t: String = (0..21).map(|i| format!("{i}.5 ")).collect();
        let src = format!("processed\n;c\n{q} 2\n{q}\n{t} 3\n*\n");
        let w = Water::parse(src.as_bytes());
        assert_eq!(w.entries.len(), 3);
        assert_eq!(w.entries[0].shape, Shape::QuadFlags);
        assert_eq!(w.entries[0].flags(), Some(2));
        assert_eq!(w.entries[1].shape, Shape::Quad);
        assert_eq!(w.entries[1].flags(), Some(1));
        assert_eq!(w.entries[1].vertex(3)[6], Some(27.0));
        assert_eq!(w.entries[2].shape, Shape::Triangle);
        assert_eq!(w.entries[2].rets, vec![22, 22, 22]);
        assert_eq!(w.entries[2].flags(), Some(3));
        assert_eq!(w.skipped, 3);
    }
}
