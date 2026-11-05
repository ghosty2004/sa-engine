//! Line readers that reproduce how `gta_sa.exe` 1.0 US splits text files into lines.
//!
//! * [`Fgets`]: `fgets(buf, size, f)` on a file opened with `fopen(path, mode)` by the MSVC 7.1
//!   CRT. In binary mode (`"rb"`) bytes are returned unchanged; in text mode (`"r"`) `\r\n` reads
//!   as `\n` and a 0x1A byte ends the file. A line longer than `size - 1` bytes is returned in
//!   several pieces, exactly like `fgets`.
//! * [`load_line_sanitise`]: `CFileLoader::LoadLine(FILE*)` (0x536F80): `fgets` into the static
//!   512-byte buffer, every byte `< 0x20` and every `,` becomes a space, then leading bytes in
//!   `1..=0x20` are skipped.
//! * [`BufferLines`]: `CFileLoader::LoadLine(char*&, int&)` (0x536FE0), used for files read whole
//!   into memory: copies up to `\n`, NUL or the remaining size, with the same sanitising.
//!
//! The returned text is a C string: it ends at the first NUL, if any.

use crate::bytes::c_str;

/// `fopen` mode of the file being read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `"rb"`: no translation.
    Binary,
    /// `"r"`: CRT text mode (`\r\n` -> `\n`, 0x1A = end of file).
    Text,
}

/// The CRT's view of the file contents after text-mode translation.
pub fn translate(data: &[u8], mode: Mode) -> Vec<u8> {
    match mode {
        Mode::Binary => data.to_vec(),
        Mode::Text => {
            let end = data.iter().position(|&b| b == 0x1A).unwrap_or(data.len());
            let data = &data[..end];
            let mut out = Vec::with_capacity(data.len());
            let mut i = 0;
            while i < data.len() {
                if data[i] == b'\r' && data.get(i + 1) == Some(&b'\n') {
                    i += 1;
                    continue;
                }
                out.push(data[i]);
                i += 1;
            }
            out
        }
    }
}

/// Successive `fgets(buf, size, f)` results.
pub struct Fgets {
    data: Vec<u8>,
    pos: usize,
    size: usize,
    /// 1-based physical line number of the start of the next piece.
    line: usize,
}

/// One `fgets` result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    /// The bytes `fgets` stored (including the `\n` when it fit), without the terminating NUL.
    pub raw: Vec<u8>,
    /// 1-based line number (after text-mode translation) where the piece starts.
    pub line: usize,
}

impl Fgets {
    /// `size` is the buffer size passed to `fgets` (at most `size - 1` bytes per call).
    pub fn new(data: &[u8], mode: Mode, size: usize) -> Self {
        Fgets {
            data: translate(data, mode),
            pos: 0,
            size,
            line: 1,
        }
    }
}

impl Iterator for Fgets {
    type Item = Piece;
    fn next(&mut self) -> Option<Piece> {
        if self.pos >= self.data.len() || self.size < 2 {
            return None;
        }
        let max = self.size - 1;
        let start = self.pos;
        let mut end = start;
        while end < self.data.len() && end - start < max {
            end += 1;
            if self.data[end - 1] == b'\n' {
                break;
            }
        }
        self.pos = end;
        let line = self.line;
        if self.data[end - 1] == b'\n' {
            self.line += 1;
        }
        Some(Piece {
            raw: self.data[start..end].to_vec(),
            line,
        })
    }
}

/// The sanitising step of `CFileLoader::LoadLine` (both variants): bytes `< 0x20` and `,`
/// become spaces. Stops at the first NUL (C string).
pub fn sanitise(raw: &[u8]) -> Vec<u8> {
    c_str(raw)
        .iter()
        .map(|&b| if b < 0x20 || b == b',' { b' ' } else { b })
        .collect()
}

/// Skips leading bytes in `1..=0x20` (the loop at the end of both `LoadLine` variants).
pub fn skip_leading(s: &[u8]) -> &[u8] {
    let n = s.iter().take_while(|&&b| b != 0 && b <= 0x20).count();
    &s[n..]
}

/// A line as returned by one of the game's line readers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// 1-based physical line number.
    pub number: usize,
    /// The text the game's parser sees (after sanitising and skipping leading blanks).
    pub text: Vec<u8>,
}

impl Line {
    pub fn starts_with(&self, p: &[u8]) -> bool {
        self.text.starts_with(p)
    }
    pub fn first(&self) -> u8 {
        self.text.first().copied().unwrap_or(0)
    }
}

/// All lines as `CFileLoader::LoadLine(FILE*)` (0x536F80) returns them for a file opened with
/// `mode` (512-byte buffer).
pub fn load_lines(data: &[u8], mode: Mode) -> Vec<Line> {
    Fgets::new(data, mode, 512)
        .map(|p| Line {
            number: p.line,
            text: skip_leading(&sanitise(&p.raw)).to_vec(),
        })
        .collect()
}

/// All lines as successive `CFileLoader::LoadLine(char*& buf, int32& size)` (0x536FE0) calls
/// return them for a buffer of `data` (the caller's size argument is `data.len()`).
///
/// The copy stops at `\n`, NUL, or when the remaining size reaches 0; the pointer then skips one
/// byte (the `\n`). Reading ends when the size is `<= 0` or the next byte is NUL.
pub fn buffer_lines(data: &[u8]) -> Vec<Line> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let mut size = data.len() as isize;
    let mut number = 1;
    loop {
        if size <= 0 || pos >= data.len() || data[pos] == 0 {
            break;
        }
        let mut copy = Vec::new();
        loop {
            let b = data.get(pos).copied().unwrap_or(0);
            if b == b'\n' || b == 0 {
                break;
            }
            copy.push(if b < 0x20 || b == b',' { b' ' } else { b });
            pos += 1;
            size -= 1;
            if size <= 0 {
                break;
            }
        }
        pos += 1;
        out.push(Line {
            number,
            text: skip_leading(&copy).to_vec(),
        });
        number += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fgets_splits_long_lines_and_keeps_cr_in_binary_mode() {
        let data = b"abc\r\ndefgh\nx";
        let p: Vec<_> = Fgets::new(data, Mode::Binary, 4).map(|p| p.raw).collect();
        assert_eq!(
            p,
            vec![
                b"abc".to_vec(),
                b"\r\n".to_vec(),
                b"def".to_vec(),
                b"gh\n".to_vec(),
                b"x".to_vec()
            ]
        );
        let p: Vec<_> = Fgets::new(data, Mode::Text, 512).map(|p| p.raw).collect();
        assert_eq!(p, vec![b"abc\n".to_vec(), b"defgh\n".to_vec(), b"x".to_vec()]);
        let p: Vec<_> = Fgets::new(b"a\n\x1Ab\n", Mode::Text, 512)
            .map(|p| p.raw)
            .collect();
        assert_eq!(p, vec![b"a\n".to_vec()]);
    }

    #[test]
    fn load_line_sanitises() {
        let l = load_lines(b"  \t12, abc\r\n\r\n#x", Mode::Binary);
        assert_eq!(l[0].text, b"12  abc  ");
        assert_eq!(l[1].text, b"");
        assert_eq!(l[2].text, b"#x");
        assert_eq!(l[2].number, 3);
    }

    #[test]
    fn buffer_lines_stop_at_nul() {
        let l = buffer_lines(b"a,b\r\n c\n\0junk");
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].text, b"a b ");
        assert_eq!(l[1].text, b"c");
    }
}
