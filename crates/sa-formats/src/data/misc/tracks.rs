//! Train tracks `data/paths/tracks.dat`, `tracks2.dat`, `tracks3.dat`, `tracks4.dat`, as
//! `CTrain::ReadAndInterpretTrackFile` (0x6F55D0) reads them.
//!
//! The whole file is read with `CFileMgr::LoadFile(path, buf, 0xB530, "rb")`, then lines are
//! copied by hand (up to `\n`, no sanitising, `\r` kept) into the shared scratch buffer at
//! 0xB71670:
//!
//! 1. the first line is copied and NUL-terminated; if it equals `processed` (strcmp, so a
//!    `\r` before the `\n` defeats it) the next line is copied the same way;
//! 2. that line is read with `%d`: the point count;
//! 3. exactly `count` further lines are copied **without a terminating NUL** and each is read
//!    with `%f %f %f %d` (x, y, z, flag). A line shorter than the previous one leaves the older
//!    bytes behind its end; they only matter when the line itself has fewer than 4 fields. The
//!    flag variable starts as the pointer to the new point array (non-zero) and keeps its value
//!    when a line does not supply it.
//!
//! No return value is checked. A file over 0xB530 bytes would be cut (the install's largest is
//! 25,474 bytes).

use super::counts;
use crate::data::{DataFile, Vars, dump_rec};
use crate::error::{Error, Result};

/// Size of the `LoadFile` buffer.
pub const BUFFER_SIZE: usize = 0xB530;

/// One point line.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub line: usize,
    pub ret: i32,
    pub pos: [Option<f32>; 3],
    /// `None` = never assigned (the exe's variable still holds the array pointer).
    pub flag: Option<i32>,
    /// `true` when the scan reached bytes left over from earlier lines (see module docs).
    pub stale: bool,
}

/// A parsed track file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackFile {
    /// `true` when the first line was `processed`.
    pub processed: bool,
    pub count_ret: i32,
    pub count: Option<i32>,
    pub points: Vec<Point>,
    /// Lines after the last point that are not read.
    pub unread_lines: usize,
}

/// Copies one line (up to `\n`) from `data` at `pos`; returns the bytes and the position after
/// the `\n`, or `None` when the data ends before a `\n` (the exe would run past the file).
fn take_line(data: &[u8], pos: usize) -> Option<(&[u8], usize)> {
    let rest = data.get(pos..)?;
    let n = rest.iter().position(|&b| b == b'\n')?;
    Some((&rest[..n], pos + n + 1))
}

impl TrackFile {
    pub fn parse(data: &[u8]) -> Result<TrackFile> {
        if data.len() > BUFFER_SIZE {
            return Err(Error::unsupported(
                "track file",
                format!("{} bytes exceed the 0xB530-byte load buffer", data.len()),
            ));
        }
        let mut out = TrackFile::default();
        let line_err = |n: usize| Error::invalid("track file", format!("line {n} has no terminating \\n"));
        let (first, mut pos) = take_line(data, 0).ok_or_else(|| line_err(1))?;
        let mut number = 2;
        let mut header = first;
        if first == b"processed" {
            out.processed = true;
            let (l, p) = take_line(data, pos).ok_or_else(|| line_err(2))?;
            header = l;
            pos = p;
            number += 1;
        }
        let mut v = Vars::default();
        out.count_ret = v.scan(header, "%d", &["count"]);
        out.count = v.i("count");
        // The scratch buffer: the header copy plus its NUL; beyond that the content is unknown.
        let mut buf = header.to_vec();
        buf.push(0);
        let mut pv = Vars::default();
        for _ in 0..out.count.unwrap_or(0).max(0) {
            let (l, p) = take_line(data, pos).ok_or_else(|| line_err(number))?;
            pos = p;
            if buf.len() < l.len() {
                buf.resize(l.len(), 0);
            }
            buf[..l.len()].copy_from_slice(l);
            let own = crate::scanf::sscanf(l, "%f %f %f %d");
            let over = crate::scanf::sscanf(&buf, "%f %f %f %d");
            let ret = pv.scan(&buf, "%f %f %f %d", &["x", "y", "z", "flag"]);
            // Past the known bytes the buffer holds data from before this file was read, so a
            // short scan without a NUL in the known bytes is undetermined.
            let stale = own != over || (ret < 4 && !buf.contains(&0));
            out.points.push(Point {
                line: number,
                ret,
                pos: [pv.f("x"), pv.f("y"), pv.f("z")],
                flag: pv.i("flag"),
                stale,
            });
            number += 1;
        }
        out.unread_lines = data[pos..]
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .count();
        Ok(out)
    }
}

impl DataFile for TrackFile {
    fn counts(&self) -> Vec<(String, u64)> {
        counts(vec![
            ("tracks: files".into(), 1),
            ("tracks: points".into(), self.points.len() as u64),
        ])
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        dump_rec!(
            &mut o,
            "header",
            processed = self.processed,
            ret = self.count_ret,
            count = self.count
        );
        for p in &self.points {
            dump_rec!(
                &mut o,
                "point",
                line = p.line,
                ret = p.ret,
                pos = p.pos,
                flag = p.flag,
                stale = p.stale
            );
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for p in &self.points {
            if p.ret != 4 || p.stale {
                v.push(format!(
                    "line {}: point read ret {} stale {}",
                    p.line, p.ret, p.stale
                ));
            }
        }
        if self.unread_lines > 0 {
            v.push(format!(
                "{} non-empty lines after the last point are not read",
                self.unread_lines
            ));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_and_stale_bytes() {
        let t = TrackFile::parse(b"3\r\n1.5 2 3 1\r\n4 5 6 0\r\n7 8\r\nextra\r\n").unwrap();
        assert_eq!(t.count, Some(3));
        assert_eq!(t.points[0].pos, [Some(1.5), Some(2.0), Some(3.0)]);
        assert_eq!(t.points[1].flag, Some(0));
        // "7 8\r" overwrites the start of "4 5 6 0\r": the buffer reads "7 8\r6 0\r".
        assert_eq!(t.points[2].ret, 4);
        assert_eq!(t.points[2].pos[2], Some(6.0));
        assert!(t.points[2].stale);
        assert_eq!(t.unread_lines, 1);
        assert!(TrackFile::parse(b"2\r\n1 2 3 0\r\n").is_err());
    }
}
