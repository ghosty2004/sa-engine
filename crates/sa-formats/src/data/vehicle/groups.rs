//! `pedgrp.dat` and `cargrp.dat`, as `CPopulation::LoadPedGroups` (0x5BCFE0) and
//! `CPopulation::LoadCarGroups` (0x5BD1A0) read them.
//!
//! Both open the file `"r"` (text mode, after `CFileMgr::SetDir("\DATA\")`) and read it with
//! `CFileMgr::ReadLine` (`fgets`, 1024-byte buffer that persists between lines). Per line:
//!
//! 1. From byte 0, every `,` and `\r` becomes a space until the first `\n` (the scan does not
//!    stop at NUL: on a line without `\n` it continues into bytes left in the buffer by earlier,
//!    longer lines); that `\n` becomes NUL.
//! 2. Up to 21 (ped) / 23 (car) tokens: blanks (`1..=0x20`) are skipped; a `#` or the end of the
//!    text stops the line; a token is a run of bytes `> 0x20`. Each token is looked up with
//!    `CModelInfo::GetModelInfo(name, &index)`; only found models are stored.
//! 3. Ped groups: a line with at least one token makes a group (even if no model is found); a
//!    line whose first non-blank is `#` or that is blank is skipped. Car groups: a group is made
//!    only when at least one model was **found**, so it depends on the model table (all names in
//!    the shipped file are expected to resolve; part b checks this). The unused slots of a group
//!    are filled with 2000; the group's count is the number of found models.
//!
//! The parser keeps the tokens per line; resolving them is the loader's job.

use crate::data::{DataFile, dump_bytes, dump_rec};
use crate::lines::{Mode, translate};

/// Which file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ped,
    Car,
}

impl Kind {
    /// Maximum tokens read per line.
    pub fn max_tokens(self) -> usize {
        match self {
            Kind::Ped => 21,
            Kind::Car => 23,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Kind::Ped => "pedgrp.dat",
            Kind::Car => "cargrp.dat",
        }
    }
}

/// A line that yields at least one token (a group candidate).
#[derive(Debug, Clone, PartialEq)]
pub struct GroupLine {
    pub line: usize,
    pub tokens: Vec<Vec<u8>>,
    /// Tokens on the line beyond the per-line maximum (not read).
    pub dropped: usize,
}

/// A parsed group file.
#[derive(Debug, Clone, PartialEq)]
pub struct Groups {
    pub kind: Kind,
    pub groups: Vec<GroupLine>,
    /// Lines whose `\n` search ran past the bytes `fgets` returned (into stale buffer bytes).
    pub stale_reads: Vec<usize>,
}

impl Groups {
    pub fn parse(data: &[u8], kind: Kind) -> Groups {
        let text = translate(data, Mode::Text);
        let mut buf = [0u8; 1024];
        let mut out = Groups {
            kind,
            groups: Vec::new(),
            stale_reads: Vec::new(),
        };
        let mut pos = 0;
        let mut number = 1;
        while pos < text.len() {
            // fgets(buf, 1024)
            let start = pos;
            while pos < text.len() && pos - start < 1023 {
                pos += 1;
                if text[pos - 1] == b'\n' {
                    break;
                }
            }
            let n = pos - start;
            buf[..n].copy_from_slice(&text[start..pos]);
            buf[n] = 0;
            let line = number;
            if text[pos - 1] == b'\n' {
                number += 1;
            }
            // Step 1: ',' and '\r' -> ' ' up to the first '\n' (not stopping at NUL).
            let mut i = 0;
            while i < buf.len() && buf[i] != b'\n' {
                if buf[i] == b',' || buf[i] == b'\r' {
                    buf[i] = b' ';
                }
                i += 1;
            }
            if i >= n {
                out.stale_reads.push(line);
            }
            if i < buf.len() {
                buf[i] = 0;
            }
            // Step 2: tokens.
            let mut tokens = Vec::new();
            let mut j = 0;
            let mut dropped = 0;
            loop {
                while j < buf.len() && buf[j] != 0 && buf[j] < 0x21 {
                    j += 1;
                }
                if j >= buf.len() || buf[j] == b'#' || buf[j] < 0x21 {
                    break;
                }
                let s = j;
                while j < buf.len() && buf[j] > 0x20 {
                    j += 1;
                }
                if tokens.len() < kind.max_tokens() {
                    tokens.push(buf[s..j].to_vec());
                } else {
                    dropped += 1;
                }
            }
            if !tokens.is_empty() {
                out.groups.push(GroupLine {
                    line,
                    tokens,
                    dropped,
                });
            }
        }
        out
    }
}

impl DataFile for Groups {
    fn counts(&self) -> Vec<(String, u64)> {
        let l = self.kind.label();
        vec![
            (format!("{l}: groups"), self.groups.len() as u64),
            (
                format!("{l}: model names"),
                self.groups.iter().map(|g| g.tokens.len() as u64).sum(),
            ),
        ]
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for (i, g) in self.groups.iter().enumerate() {
            dump_rec!(&mut o, "group", line = g.line, index = i, count = g.tokens.len());
            o.pop();
            o.push_str(" names=[");
            for (k, t) in g.tokens.iter().enumerate() {
                if k > 0 {
                    o.push(' ');
                }
                dump_bytes(t, &mut o);
            }
            o.push_str("]\n");
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .groups
            .iter()
            .filter(|g| g.dropped > 0)
            .map(|g| {
                format!(
                    "line {}: {} tokens beyond the per-line maximum are ignored",
                    g.line, g.dropped
                )
            })
            .collect();
        v.extend(
            self.stale_reads
                .iter()
                .map(|l| format!("line {l}: no newline, the comma scan reads stale buffer bytes")),
        );
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ped_and_car_groups() {
        let src = b"# comment\r\nA, B,C\t# trailing\r\n\r\n  #x\r\nD\r\n";
        let g = Groups::parse(src, Kind::Ped);
        assert_eq!(g.groups.len(), 2);
        assert_eq!(
            g.groups[0].tokens,
            vec![b"A".to_vec(), b"B".to_vec(), b"C".to_vec()]
        );
        assert_eq!(g.groups[1].line, 5);
        assert!(g.stale_reads.is_empty());
        let many: Vec<u8> = (0..30)
            .flat_map(|i| format!("m{i}, ").into_bytes())
            .chain(*b"\n")
            .collect();
        let g = Groups::parse(&many, Kind::Car);
        assert_eq!(g.groups[0].tokens.len(), 23);
        assert_eq!(g.groups[0].dropped, 7);
        // Last line without '\n': the scan runs into the previous line's bytes.
        let g = Groups::parse(b"LONGNAME,X\nab", Kind::Ped);
        assert_eq!(g.stale_reads, vec![2]);
        assert_eq!(g.groups[1].tokens, vec![b"ab".to_vec()]);
    }
}
