//! `data/fonts.dat`, as `CFont::LoadFontValues` (0x7187C0) reads it.
//!
//! Opened `"rb"`, `LoadLine` (0x536F80). Lines starting with `#` or NUL are skipped; any other
//! line is read with `%s` and the token compared (exact, including the NUL) with:
//!
//! * `[TOTAL_FONTS]`, `[FONT_ID]`, `[REPLACEMENT_SPACE_CHAR]`, `[UNPROP]`: the **next line
//!   returned by `LoadLine`** (not skipped even when empty or a comment) is read with `%d`;
//!   `[FONT_ID]` selects the font the following keys write to (its low byte);
//! * `[PROP]`: the next **26** `LoadLine` lines are each read with
//!   `%d  %d  %d  %d  %d  %d  %d  %d`, and the low byte of each of the 8 variables is stored
//!   (26 x 8 = 208 proportional widths).
//!
//! The 8 integer variables are shared by all keys and never reset, so a short or empty line
//! repeats the previous values. The value lines are not checked for `#` or emptiness; in the
//! install every `[PROP]` block has exactly 26 rows and every single value follows its key
//! directly, so every read returns its full count.

use super::{Fields, Rec, counts};
use crate::data::{DataFile, Vars};
use crate::lines::{Mode, load_lines};

const V: [&str; 8] = ["v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7"];
pub const FMT_PROP: &str = "%d  %d  %d  %d  %d  %d  %d  %d";

/// Key kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    TotalFonts,
    FontId,
    ReplacementSpaceChar,
    Prop,
    Unprop,
}

impl Key {
    fn name(self) -> &'static str {
        match self {
            Key::TotalFonts => "[TOTAL_FONTS]",
            Key::FontId => "[FONT_ID]",
            Key::ReplacementSpaceChar => "[REPLACEMENT_SPACE_CHAR]",
            Key::Prop => "[PROP]",
            Key::Unprop => "[UNPROP]",
        }
    }
}

/// One key and the value lines it consumed.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub key: Key,
    /// Line of the key.
    pub line: usize,
    /// Current font id (low byte of the last `[FONT_ID]` value; 0 before any).
    pub font: u8,
    /// Value lines read: `v0` for single values, `v0`..`v7` for `[PROP]` rows. `line` 0 marks a
    /// read past the end of the file (the exe would pass NULL to `sscanf`).
    pub values: Vec<Rec>,
}

/// A parsed `fonts.dat`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fonts {
    pub entries: Vec<Entry>,
    /// Lines with an unknown first token.
    pub ignored: Vec<(usize, Vec<u8>)>,
}

impl Fonts {
    pub fn parse(data: &[u8]) -> Fonts {
        let lines = load_lines(data, Mode::Binary);
        let mut out = Fonts::default();
        let mut v = Vars::default();
        let mut tok = Vars::default();
        let mut font = 0u8;
        let mut i = 0;
        while i < lines.len() {
            let l = &lines[i];
            i += 1;
            let c = l.first();
            if c == b'#' || c == 0 {
                continue;
            }
            tok.scan(&l.text, "%s", &["tok"]);
            let t = tok.s("tok").unwrap_or_default();
            let key = [
                Key::TotalFonts,
                Key::FontId,
                Key::ReplacementSpaceChar,
                Key::Prop,
                Key::Unprop,
            ]
            .into_iter()
            .find(|k| k.name().as_bytes() == &t[..]);
            let Some(key) = key else {
                out.ignored.push((l.number, l.text.clone()));
                continue;
            };
            let (n, fmt, names): (usize, &str, &[&'static str]) = if key == Key::Prop {
                (26, FMT_PROP, &V)
            } else {
                (1, "%d", &V[..1])
            };
            let mut values = Vec::new();
            for _ in 0..n {
                let (number, ret) = match lines.get(i) {
                    Some(vl) => (vl.number, v.scan(&vl.text, fmt, names)),
                    None => (0, -1),
                };
                i += 1;
                values.push(Rec {
                    line: number,
                    ret,
                    fields: Fields::snapshot(&v, names),
                });
            }
            if key == Key::FontId {
                font = v.i("v0").unwrap_or(0) as u8;
            }
            out.entries.push(Entry {
                key,
                line: l.number,
                font,
                values,
            });
        }
        out
    }
}

impl DataFile for Fonts {
    fn counts(&self) -> Vec<(String, u64)> {
        let n = |k: Key| self.entries.iter().filter(|e| e.key == k).count() as u64;
        counts(vec![
            ("fonts.dat: [FONT_ID] keys".into(), n(Key::FontId)),
            ("fonts.dat: [PROP] blocks".into(), n(Key::Prop)),
            (
                "fonts.dat: other keys".into(),
                self.entries.len() as u64 - n(Key::FontId) - n(Key::Prop),
            ),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for e in &self.entries {
            o.push_str(&format!("{} line={} font={}\n", e.key.name(), e.line, e.font));
            for r in &e.values {
                r.dump_to("  value", &mut o);
            }
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for e in &self.entries {
            let want = if e.key == Key::Prop { 8 } else { 1 };
            for r in &e.values {
                if r.ret != want {
                    v.push(format!(
                        "line {}: {} value line read with ret {} (of {want}); variables keep earlier values",
                        r.line,
                        e.key.name(),
                        r.ret
                    ));
                }
            }
        }
        for (n, t) in &self.ignored {
            v.push(format!(
                "line {n}: unknown key, ignored: {:?}",
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
    fn keys_and_prop_rows() {
        let mut src = b"[TOTAL_FONTS]\r\n2\r\n[FONT_ID]\r\n1\r\n[PROP]\r\n".to_vec();
        for i in 0..25 {
            src.extend(format!("{i} 1 2 3 4 5 6 7 # c\r\n").bytes());
        }
        src.extend(b"\r\n[UNPROP]\r\n# not skipped\r\n");
        let f = Fonts::parse(&src);
        assert_eq!(f.entries.len(), 4);
        let p = &f.entries[2];
        assert_eq!(p.font, 1);
        assert_eq!(p.values.len(), 26);
        // The 26th line is empty: values repeat row 25.
        assert_eq!(p.values[25].ret, -1);
        assert_eq!(p.values[25].fields.i32("v0"), Some(24));
        // [UNPROP]'s value line is a comment: read anyway, 0 fields.
        assert_eq!(f.entries[3].values[0].ret, 0);
    }
}
