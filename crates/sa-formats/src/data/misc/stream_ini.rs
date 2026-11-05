//! `stream.ini` (game directory), as `CStreaming::ReadIniFile` (0x5BCCD0) reads it.
//!
//! Opened `"r"`, `LoadLine` (0x536F80); lines starting with `#` or NUL are skipped. Each other
//! line is split with `strtok(line, " ,\t")` (key) and `strtok(NULL, " ,\t")` (value, may be
//! NULL). Keys are compared with `_stricmp`:
//!
//! | key | value |
//! |---|---|
//! | `memory` | `atol(value) << 10` as the streaming memory limit, **only if no `devkit_memory` line came before** |
//! | `devkit_memory` | `atol(value) << 10`, and later `memory` lines are ignored |
//! | `vehicles` | `atol` (desired number of loaded vehicles) |
//! | `dontbuildpaths` | flag set, value not used |
//! | `pe_lightchangerate`, `pe_lightingbasecap`, `pe_lightingbasemult` | `(float)atof` |
//! | `pe_leftx`, `pe_rightx`, `pe_topy`, `pe_bottomy`, `def_brightness_pal` | `atol` |
//! | `pe_bRadiosity` | `atol(value) != 0` |
//!
//! Unknown keys are ignored. A known numeric key without a value would pass NULL to `atol`.

use super::{counts, stricmp_eq, strtok_all};
use crate::data::{DataFile, dump_rec};
use crate::lines::{Mode, load_lines};
use crate::scanf::{atof, atoi};

/// How the exe converts the value of a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conv {
    /// `atol` (the result is shown as `i32`; `memory`/`devkit_memory` are then shifted left 10).
    Int,
    /// `(float)atof`.
    Float,
    /// No value used.
    Flag,
}

/// Known keys, in the exe's comparison order.
pub const KEYS: [(&str, Conv); 13] = [
    ("memory", Conv::Int),
    ("devkit_memory", Conv::Int),
    ("vehicles", Conv::Int),
    ("dontbuildpaths", Conv::Flag),
    ("pe_lightchangerate", Conv::Float),
    ("pe_lightingbasecap", Conv::Float),
    ("pe_lightingbasemult", Conv::Float),
    ("pe_leftx", Conv::Int),
    ("pe_rightx", Conv::Int),
    ("pe_topy", Conv::Int),
    ("pe_bottomy", Conv::Int),
    ("pe_bRadiosity", Conv::Int),
    ("def_brightness_pal", Conv::Int),
];

/// One line.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub line: usize,
    /// The key token (`None` when the line has only delimiters).
    pub key: Option<Vec<u8>>,
    pub value: Option<Vec<u8>>,
    /// Index into [`KEYS`] when the key is known.
    pub known: Option<usize>,
    /// `false` for a `memory` line after `devkit_memory` (ignored by the exe).
    pub applied: bool,
    /// `atol(value)` for integer keys (`None` without a value).
    pub int_value: Option<i32>,
    /// `(float)atof(value)` for float keys.
    pub float_value: Option<f32>,
}

/// A parsed `stream.ini`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StreamIni {
    pub entries: Vec<Entry>,
}

impl StreamIni {
    pub fn parse(data: &[u8]) -> StreamIni {
        let mut out = StreamIni::default();
        let mut devkit = false;
        for l in load_lines(data, Mode::Text) {
            let c = l.first();
            if c == b'#' || c == 0 {
                continue;
            }
            let toks = strtok_all(&l.text, b" ,\t");
            let key = toks.first().map(|t| t.to_vec());
            let value = toks.get(1).map(|t| t.to_vec());
            let known = key
                .as_ref()
                .and_then(|k| KEYS.iter().position(|(n, _)| stricmp_eq(k, n.as_bytes())));
            let mut applied = known.is_some();
            if known == Some(0) && devkit {
                applied = false;
            }
            if known == Some(1) {
                devkit = true;
            }
            let conv = known.map(|k| KEYS[k].1);
            let int_value = (conv == Some(Conv::Int))
                .then(|| value.as_ref().map(|v| atoi(v)))
                .flatten();
            let float_value = (conv == Some(Conv::Float))
                .then(|| value.as_ref().map(|v| atof(v) as f32))
                .flatten();
            out.entries.push(Entry {
                line: l.number,
                key,
                value,
                known,
                applied,
                int_value,
                float_value,
            });
        }
        out
    }
}

impl DataFile for StreamIni {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            (
                "stream.ini: known keys".into(),
                self.entries.iter().filter(|e| e.known.is_some()).count() as u64,
            ),
            (
                "stream.ini: unknown keys".into(),
                self.entries.iter().filter(|e| e.known.is_none()).count() as u64,
            ),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for e in &self.entries {
            dump_rec!(
                &mut o,
                "key",
                line = e.line,
                key = e.key,
                value = e.value,
                applied = e.applied,
                int = e.int_value,
                float = e.float_value
            );
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for e in &self.entries {
            if e.known.is_none() {
                v.push(format!(
                    "line {}: unknown key {:?}, ignored",
                    e.line,
                    e.key.as_deref().map(crate::data::lossy)
                ));
            } else if !e.applied {
                v.push(format!("line {}: memory after devkit_memory, ignored", e.line));
            }
            if e.known.is_some_and(|k| KEYS[k].1 != Conv::Flag) && e.value.is_none() {
                v.push(format!(
                    "line {}: key without a value (the exe would pass NULL)",
                    e.line
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
    fn keys() {
        let s = StreamIni::parse(
            b"devkit_memory\t100\r\nMEMORY 5\r\npe_lightchangerate\t0.0005\r\ndontbuildpaths\r\nfoo 1\r\n",
        );
        assert_eq!(s.entries.len(), 5);
        assert_eq!(s.entries[0].int_value, Some(100));
        assert!(!s.entries[1].applied);
        assert_eq!(s.entries[2].float_value, Some(0.0005));
        assert!(s.entries[3].applied);
        assert_eq!(s.entries[4].known, None);
    }
}
