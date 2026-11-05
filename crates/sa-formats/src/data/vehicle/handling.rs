//! `handling.cfg`, as `cHandlingDataMgr::LoadHandlingData` (0x5BD830) reads it.
//!
//! Opened `"rb"`, lines from `LoadLine` (0x536F80). The loop stops at a line equal to
//! `;the end` (9-byte compare including the NUL); because `LoadLine` turns the `\r\n` into two
//! trailing spaces, that compare never matches the shipped file, whose `;the end` line is a
//! comment anyway, so reading always runs to the end of the file. A line is then dispatched on
//! its first byte: `;` comment, `!` bike, `$` flying, `%` boat, `^` vehicle anim group, anything
//! else (including an empty line) a car line. The line is split with `strtok(line, " \t")` and
//! every token is converted by its index (`atof` then `(float)`, `atol`, `sscanf("%x")`, or the
//! first byte); tokens past the last handled index are ignored. For `! $ % ^` lines token 0 is the
//! marker itself.
//!
//! The tables [`conv`] list, per kind and token index, the conversion and the byte offset the
//! exe stores the value at inside the destination struct (`tHandlingData` 0xE0,
//! `tBikeHandlingData` 0x40, `tFlyingHandlingData` 0x58, `tBoatHandlingData`), taken from the
//! decompile. Car token 13 is multiplied by 0.4 (x87) before the store; the parser keeps the
//! `atof` value. The vehicle name token is looked up in the exe's name table
//! (`FindExactWord`, 0x6F4F30, 210 names of 14 bytes); the parser keeps the name.
//!
//! Note: Ghidra shows a `return` after the `^` branch; the bytes at 0x5BE16E are a jump out to
//! 0x40825C and back to 0x5BE179 (protection trampoline), so the loop continues.

use crate::data::{DataFile, dump_rec, lossy};
use crate::lines::{Mode, load_lines};
use crate::scanf::{atof, atoi, sscanf};

/// Line kind, from the first byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Car,
    Bike,
    Flying,
    Boat,
    Anim,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Car => "car",
            Kind::Bike => "bike",
            Kind::Flying => "flying",
            Kind::Boat => "boat",
            Kind::Anim => "anim",
        }
    }
}

/// Token conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conv {
    /// Vehicle name, resolved with `FindExactWord`.
    Name,
    /// `(float)atof(token)`.
    Float,
    /// `atol(token)` (stored as byte/bool/int by the exe depending on the field).
    Int,
    /// `sscanf(token, "%x", &field)`.
    Hex,
    /// The token's first byte.
    Char,
    /// Anim lines, tokens 4..=21: `atol(token) != 0` sets bit `index - 4` of the flag word.
    Bit,
    /// Not read (the marker token, or past the last handled index).
    Ignored,
}

/// Conversion and destination offset (inside the kind's struct; `None` = a local) for token
/// `i` of a line of kind `k`.
pub fn conv(k: Kind, i: usize) -> (Conv, Option<u16>) {
    const CAR: [(Conv, u16); 36] = [
        (Conv::Name, 0x00),
        (Conv::Float, 0x04),
        (Conv::Float, 0x0C),
        (Conv::Float, 0x10),
        (Conv::Float, 0x14),
        (Conv::Float, 0x18),
        (Conv::Float, 0x1C),
        (Conv::Int, 0x20),
        (Conv::Float, 0x28),
        (Conv::Float, 0xA4),
        (Conv::Float, 0xA8),
        (Conv::Int, 0x76),
        (Conv::Float, 0x84),
        (Conv::Float, 0x7C),
        (Conv::Float, 0x80),
        (Conv::Char, 0x74),
        (Conv::Char, 0x75),
        (Conv::Float, 0x94),
        (Conv::Float, 0x98),
        (Conv::Int, 0x9C),
        (Conv::Float, 0xA0),
        (Conv::Float, 0xAC),
        (Conv::Float, 0xB0),
        (Conv::Float, 0xB4),
        (Conv::Float, 0xB8),
        (Conv::Float, 0xBC),
        (Conv::Float, 0xC0),
        (Conv::Float, 0xC4),
        (Conv::Float, 0xD4),
        (Conv::Float, 0xC8),
        (Conv::Int, 0xD8),
        (Conv::Hex, 0xCC),
        (Conv::Hex, 0xD0),
        (Conv::Int, 0xDC),
        (Conv::Int, 0xDD),
        (Conv::Int, 0xDE),
    ];
    match k {
        Kind::Car => CAR.get(i).map_or((Conv::Ignored, None), |&(c, o)| (c, Some(o))),
        Kind::Bike | Kind::Flying => {
            let last = if k == Kind::Bike { 16 } else { 22 };
            match i {
                1 => (Conv::Name, Some(0)),
                2.. if i <= last => (Conv::Float, Some(((i - 1) * 4) as u16)),
                _ => (Conv::Ignored, None),
            }
        }
        Kind::Boat => match i {
            1 => (Conv::Name, Some(0)),
            2..=8 => (Conv::Float, Some(((i - 1) * 4) as u16)),
            9..=14 => (Conv::Float, Some((i * 4) as u16)),
            15 => (Conv::Float, Some(0x20)),
            _ => (Conv::Ignored, None),
        },
        Kind::Anim => match i {
            1..=3 | 35 => (Conv::Int, None),
            4..=21 => (Conv::Bit, None),
            22..=34 => (Conv::Float, None),
            _ => (Conv::Ignored, None),
        },
    }
}

/// A converted token value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Name(Vec<u8>),
    Float(f32),
    Int(i32),
    /// `None` when `%x` matched nothing (the field keeps its previous value).
    Hex(Option<u32>),
    Char(u8),
    Ignored(Vec<u8>),
}

/// One data line.
#[derive(Debug, Clone, PartialEq)]
pub struct HandlingLine {
    pub line: usize,
    pub kind: Kind,
    /// The `strtok` tokens.
    pub tokens: Vec<Vec<u8>>,
    /// The converted values, one per token.
    pub values: Vec<Value>,
}

impl HandlingLine {
    /// The vehicle name token (`None` for anim lines or a line without it).
    pub fn name(&self) -> Option<&[u8]> {
        self.values.iter().find_map(|v| match v {
            Value::Name(n) => Some(&n[..]),
            _ => None,
        })
    }

    /// Anim lines: the flag word built from tokens 4..=21.
    pub fn anim_flags(&self) -> u32 {
        let mut f = 0;
        for (i, t) in self.tokens.iter().enumerate() {
            if conv(Kind::Anim, i).0 == Conv::Bit && atoi(t) != 0 {
                f |= 1 << (i - 4);
            }
        }
        f
    }
}

/// A parsed `handling.cfg`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Handling {
    pub lines: Vec<HandlingLine>,
    pub comments: usize,
}

impl Handling {
    pub fn parse(data: &[u8]) -> Handling {
        let mut h = Handling::default();
        for l in load_lines(data, Mode::Binary) {
            if l.text == b";the end" {
                break;
            }
            let kind = match l.first() {
                b';' => {
                    h.comments += 1;
                    continue;
                }
                b'!' => Kind::Bike,
                b'$' => Kind::Flying,
                b'%' => Kind::Boat,
                b'^' => Kind::Anim,
                _ => Kind::Car,
            };
            let tokens: Vec<Vec<u8>> = super::strtok(&l.text, b" \t")
                .into_iter()
                .map(<[u8]>::to_vec)
                .collect();
            let values = tokens
                .iter()
                .enumerate()
                .map(|(i, t)| match conv(kind, i).0 {
                    Conv::Name => Value::Name(t.clone()),
                    Conv::Float => Value::Float(atof(t) as f32),
                    Conv::Int | Conv::Bit => Value::Int(atoi(t)),
                    Conv::Hex => Value::Hex(sscanf(t, "%x").u32(0)),
                    Conv::Char => Value::Char(t[0]),
                    Conv::Ignored => Value::Ignored(t.clone()),
                })
                .collect();
            h.lines.push(HandlingLine {
                line: l.number,
                kind,
                tokens,
                values,
            });
        }
        h
    }

    /// Number of tokens each kind's loader consumes (including token 0).
    pub fn expected_tokens(k: Kind) -> usize {
        match k {
            Kind::Car => 36,
            Kind::Bike => 17,
            Kind::Flying => 23,
            Kind::Boat => 16,
            Kind::Anim => 36,
        }
    }
}

impl DataFile for Handling {
    fn counts(&self) -> Vec<(String, u64)> {
        let mut v = Vec::new();
        for k in [Kind::Car, Kind::Bike, Kind::Flying, Kind::Boat, Kind::Anim] {
            let n = self.lines.iter().filter(|l| l.kind == k).count() as u64;
            v.push((format!("handling.cfg: {} lines", k.name()), n));
        }
        v
    }

    fn dump(&self) -> String {
        let mut o = String::new();
        for l in &self.lines {
            o.push_str(l.kind.name());
            dump_rec!(&mut o, "", line = l.line);
            o.pop();
            for (i, v) in l.values.iter().enumerate() {
                o.push_str(&format!(" t{i}="));
                match v {
                    Value::Name(n) | Value::Ignored(n) => crate::data::dump_bytes(n, &mut o),
                    Value::Float(f) => o.push_str(&format!("{f:?}")),
                    Value::Int(x) => o.push_str(&x.to_string()),
                    Value::Hex(x) => o.push_str(&x.map_or("~".to_string(), |x| format!("{x:#x}"))),
                    Value::Char(c) => crate::data::dump_bytes(&[*c], &mut o),
                }
            }
            if l.kind == Kind::Anim {
                o.push_str(&format!(" flags={:#x}", l.anim_flags()));
            }
            o.push('\n');
        }
        o
    }

    fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        for l in &self.lines {
            let want = Handling::expected_tokens(l.kind);
            if l.tokens.len() != want {
                v.push(format!(
                    "line {}: {} line {:?} has {} tokens, the loader converts {}",
                    l.line,
                    l.kind.name(),
                    l.name().map(lossy).unwrap_or_default(),
                    l.tokens.len(),
                    want
                ));
            }
            for (i, val) in l.values.iter().enumerate() {
                if *val == Value::Hex(None) {
                    v.push(format!("line {}: token {i} is not hex", l.line));
                }
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_conversions() {
        let mut car = b"LANDSTAL 1700.0 5008.3 2.5 0.0 0.0 -0.3 85 0.75 0.85 0.5 5 160.0 25.0 20.0 4 D 6.2 0.60 0 35.0 2.4 0.08 0.0 0.28 -0.14 0.5 0.25 0.27 0.23 25000 20 500002 0 1 0".to_vec();
        car.extend_from_slice(b"\r\n;c\r\n!\tBIKE\t0.35\r\n^\t0\t1\t2\t3\t1\t0\t1\r\n;the end\r\n%\tX 1\r\n");
        let h = Handling::parse(&car);
        assert_eq!(h.lines.len(), 4);
        let c = &h.lines[0];
        assert_eq!(c.kind, Kind::Car);
        assert_eq!(c.tokens.len(), 36);
        assert_eq!(c.values[1], Value::Float(1700.0));
        assert_eq!(c.values[7], Value::Int(85));
        assert_eq!(c.values[16], Value::Char(b'D'));
        assert_eq!(c.values[32], Value::Hex(Some(0x500002)));
        assert_eq!(h.lines[1].name(), Some(&b"BIKE"[..]));
        assert_eq!(h.lines[2].anim_flags(), 0b1011);
        // ";the end" with its CRLF is not the 8-byte string, so the boat line is still read.
        assert_eq!(h.lines[3].kind, Kind::Boat);
        // Without trailing bytes it stops the read.
        assert_eq!(Handling::parse(b";the end\0").lines.len(), 0);
        assert_eq!(conv(Kind::Boat, 15), (Conv::Float, Some(0x20)));
        assert_eq!(conv(Kind::Boat, 9), (Conv::Float, Some(0x24)));
    }
}
