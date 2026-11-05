//! `gta.dat` / `default.dat`, as `CFileLoader::LoadLevel` (0x5B9030) reads them.
//!
//! Opened with mode `"r"` (text: `\r\n` reads as `\n`), lines from `LoadLine` (0x536F80), so
//! the `\n` becomes a trailing space that stays part of every path argument. A line starting
//! with `#` is skipped; then the first matching `strncmp` decides, in this order:
//!
//! | test | argument | action |
//! |---|---|---|
//! | `EXIT` (4) | — | stop reading |
//! | `TEXDICTION` (10) | `line + 11` (copied into a 64-byte buffer) | load a TXD into the current dictionary |
//! | `IMG` (3) | `line + 4` | `CStreaming::AddImageToList(path, true)` unless the 20 bytes from `line + 4` equal `MODELS\GTA_INT.IMG\0` |
//! | `COLFILE` (7) | `line + 10` (skips the zone digit and its blank) | load a collision file into slot 0 |
//! | `MODELFILE` (9) | `line + 10` | load an atomic DFF |
//! | `HIERFILE` (8) | `line + 9` | load a clump DFF |
//! | `IDE` (3) | `line + 4` | `LoadObjectTypes` |
//! | `IPL` (3) | `line + 4` (copied into a 64-byte buffer) | first time: object.dat, vehicle data, streaming init, collision bounds; then `LoadScene` |
//! | `SPLASH` (6) | — | compared, no action |
//!
//! Any other line (including empty ones) is ignored.

use super::{DataFile, dump_rec};
use crate::lines::{Mode, load_lines};

/// Directive kinds, in the exe's comparison order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Directive {
    Exit,
    TexDiction,
    Img,
    ColFile,
    ModelFile,
    HierFile,
    Ide,
    Ipl,
    Splash,
}

impl Directive {
    pub fn name(self) -> &'static str {
        match self {
            Directive::Exit => "EXIT",
            Directive::TexDiction => "TEXDICTION",
            Directive::Img => "IMG",
            Directive::ColFile => "COLFILE",
            Directive::ModelFile => "MODELFILE",
            Directive::HierFile => "HIERFILE",
            Directive::Ide => "IDE",
            Directive::Ipl => "IPL",
            Directive::Splash => "SPLASH",
        }
    }

    /// Offset of the argument from the line start.
    fn arg_offset(self) -> usize {
        match self {
            Directive::Exit | Directive::Splash => 0,
            Directive::TexDiction => 11,
            Directive::Img | Directive::Ide | Directive::Ipl => 4,
            Directive::ColFile | Directive::ModelFile => 10,
            Directive::HierFile => 9,
        }
    }
}

/// One recognised line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub line: usize,
    pub directive: Directive,
    /// The argument exactly as the exe passes it (including the trailing space; empty for
    /// `EXIT`/`SPLASH`, or when the line is shorter than the offset: the exe would then read
    /// past the line's NUL, which does not happen in the install).
    pub arg: Vec<u8>,
    /// `IMG` only: `false` when the exe skips the archive (the `GTA_INT.IMG` test matched).
    pub img_added: bool,
}

impl Entry {
    /// The argument without trailing blanks (Windows ignores them when opening the file).
    pub fn path(&self) -> &[u8] {
        let n = self.arg.iter().rposition(|&c| c != b' ').map_or(0, |p| p + 1);
        &self.arg[..n]
    }
}

/// A parsed level file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LevelFile {
    pub entries: Vec<Entry>,
    /// Non-comment lines that match no directive.
    pub ignored: Vec<(usize, Vec<u8>)>,
}

impl LevelFile {
    pub fn parse(data: &[u8]) -> LevelFile {
        let mut out = LevelFile::default();
        for l in load_lines(data, Mode::Text) {
            let t = &l.text[..];
            if l.first() == b'#' {
                continue;
            }
            let order = [
                Directive::Exit,
                Directive::TexDiction,
                Directive::Img,
                Directive::ColFile,
                Directive::ModelFile,
                Directive::HierFile,
                Directive::Ide,
                Directive::Ipl,
                Directive::Splash,
            ];
            let Some(d) = order.into_iter().find(|d| t.starts_with(d.name().as_bytes())) else {
                if !t.is_empty() {
                    out.ignored.push((l.number, t.to_vec()));
                }
                continue;
            };
            let mut arg = t.get(d.arg_offset()..).unwrap_or(&[]).to_vec();
            if matches!(d, Directive::TexDiction | Directive::Ipl) {
                // Copied into `char local_40[64]`; longer arguments would overflow (none do).
                debug_assert!(arg.len() < 64);
            }
            let mut img_added = false;
            if d == Directive::Img {
                // 0x13 bytes of "MODELS\GTA_INT.IMG" plus the NUL are compared.
                let mut want = b"MODELS\\GTA_INT.IMG".to_vec();
                want.push(0);
                let mut got = arg.clone();
                got.push(0);
                img_added = got.get(..want.len()) != Some(&want[..]);
            }
            if d == Directive::Exit {
                arg.clear();
            }
            if d == Directive::Splash {
                arg.clear();
            }
            out.entries.push(Entry {
                line: l.number,
                directive: d,
                arg,
                img_added,
            });
            if d == Directive::Exit {
                break;
            }
        }
        out
    }

    /// Paths of the given directive, trailing blanks removed.
    pub fn paths(&self, d: Directive) -> impl Iterator<Item = &[u8]> {
        self.entries
            .iter()
            .filter(move |e| e.directive == d)
            .map(Entry::path)
    }
}

impl DataFile for LevelFile {
    fn counts(&self) -> Vec<(String, u64)> {
        let mut m: Vec<(String, u64)> = Vec::new();
        for e in &self.entries {
            let k = format!("level: {}", e.directive.name());
            match m.iter_mut().find(|(n, _)| *n == k) {
                Some(x) => x.1 += 1,
                None => m.push((k, 1)),
            }
        }
        m
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for e in &self.entries {
            dump_rec!(
                &mut o,
                e.directive.name(),
                line = e.line,
                arg = e.arg,
                img_added = e.img_added
            );
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        self.ignored
            .iter()
            .map(|(n, t)| format!("line {n}: no directive, ignored: {:?}", super::lossy(t)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directives() {
        let src = b"# x\r\nIMG MODELS\\GTA_INT.IMG\r\nCOLFILE 0 MODELS\\COLL\\WEAPONS.COL\r\nIDE DATA\\A.IDE\r\nfoo\r\nEXIT\r\nIDE B\r\n";
        let f = LevelFile::parse(src);
        assert_eq!(f.entries.len(), 4);
        // The trailing space defeats the GTA_INT test.
        assert!(f.entries[0].img_added);
        assert_eq!(f.entries[1].arg, b"MODELS\\COLL\\WEAPONS.COL ");
        assert_eq!(f.entries[1].path(), b"MODELS\\COLL\\WEAPONS.COL");
        assert_eq!(f.entries[3].directive, Directive::Exit);
        assert_eq!(f.ignored.len(), 1);
        // Without a newline at the end the test matches.
        let f = LevelFile::parse(b"IMG MODELS\\GTA_INT.IMG");
        assert!(!f.entries[0].img_added);
    }
}
