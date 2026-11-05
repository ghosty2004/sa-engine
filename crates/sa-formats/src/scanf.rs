//! `sscanf` as implemented by the MSVC 7.1 C runtime that `gta_sa.exe` 1.0 US links statically.
//!
//! The game parses almost every text data file by passing a sanitised line to `sscanf` with a
//! fixed format. Reproducing the loaders exactly needs the CRT's semantics, not Rust's, so this
//! module re-implements the subset of `_input.c` the game uses:
//!
//! * whitespace in the format skips any run of input whitespace (`isspace` in the "C" locale:
//!   0x09-0x0D and 0x20; bytes >= 0x80 are not whitespace);
//! * `%d %i %u %x %o` (optionally `h`/`l` sized) accumulate into a 32-bit `long` with wrap-around
//!   (no overflow check), after an optional sign; `%x` accepts a `0x`/`0X` prefix; a field without
//!   any digit is a matching failure;
//! * `%f %e %g` collect `[sign] digits [. digits] [e [sign] digits]` (the exponent only after a
//!   digit) and convert the collected text; with no digit at all it is a matching failure;
//! * `%s` reads a run of non-whitespace bytes, `%c` reads `width` (default 1) bytes without
//!   skipping whitespace, `%[...]` reads a scan set (with `^` and `a-z` ranges), `%n` stores the
//!   number of bytes consumed, `%%` matches `%`, `*` suppresses assignment;
//! * any other format byte must match the next input byte;
//! * the return value is the number of assigned conversions, or -1 (`EOF`) when the input ended
//!   before the first conversion.
//!
//! Float conversion: the CRT converts the collected text with `__strgtold12` (80-bit mantissa)
//! and then rounds once to `float`/`double` (`_fassign`). That matches a correctly rounded
//! decimal-to-binary conversion except in double-rounding cases that need more than 64
//! significant bits; [`parse_float_text`] uses Rust's correctly rounded parser. The data files
//! only contain short decimals, and FMT-2 part b compares the results with the original's
//! memory (see docs/re/formats.md, open questions).

/// One assigned conversion result.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    /// `%d`, `%i`, `%u`, `%x`, `%o`, `%n`: the 32-bit value as the CRT stores it (`%u`/`%x`
    /// results are the same bits; read them with [`Val::as_u32`]).
    Int(i32),
    /// `%f`, `%e`, `%g` without `l`: a `float`.
    Float(f32),
    /// `%lf`, `%le`, `%lg`: a `double`.
    Double(f64),
    /// `%s`, `%[...]`, `%c`: the bytes stored (without the NUL that `%s` appends).
    Str(Vec<u8>),
}

impl Val {
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Val::Int(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_u32(&self) -> Option<u32> {
        self.as_i32().map(|v| v as u32)
    }
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Val::Float(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Val::Double(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Val::Str(v) => Some(v),
            _ => None,
        }
    }
}

/// Result of one `sscanf` call.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scan {
    /// The return value of `sscanf`.
    pub ret: i32,
    /// One slot per non-suppressed conversion in the format (including `%n`), in order.
    /// `None` = the conversion was not reached, so the destination variable keeps whatever it
    /// held before the call.
    pub vals: Vec<Option<Val>>,
}

impl Scan {
    pub fn get(&self, i: usize) -> Option<&Val> {
        self.vals.get(i).and_then(|v| v.as_ref())
    }
    pub fn i32(&self, i: usize) -> Option<i32> {
        self.get(i).and_then(Val::as_i32)
    }
    pub fn u32(&self, i: usize) -> Option<u32> {
        self.get(i).and_then(Val::as_u32)
    }
    pub fn f32(&self, i: usize) -> Option<f32> {
        self.get(i).and_then(Val::as_f32)
    }
    pub fn f64(&self, i: usize) -> Option<f64> {
        self.get(i).and_then(Val::as_f64)
    }
    pub fn bytes(&self, i: usize) -> Option<&[u8]> {
        self.get(i).and_then(Val::as_bytes)
    }
    pub fn string(&self, i: usize) -> Option<Vec<u8>> {
        self.bytes(i).map(<[u8]>::to_vec)
    }
}

/// `isspace` in the "C" locale, for an input byte read as `unsigned char`.
pub fn is_space(b: u8) -> bool {
    matches!(b, 0x09..=0x0D | 0x20)
}

/// Normalises a CRT-collected float text (`[-]digits[.digits][e[-]digits]`, possibly with an
/// exponent marker without digits, or an empty integer/fraction part) into Rust syntax.
fn normalise_float_text(text: &[u8]) -> String {
    let mut s: Vec<u8> = text.to_vec();
    if let Some(e) = s.iter().position(|&c| c == b'e' || c == b'E')
        && !s[e + 1..].iter().any(|c| c.is_ascii_digit())
    {
        s.truncate(e);
    }
    let st = String::from_utf8_lossy(&s).replace("-.", "-0.");
    let st = if st.starts_with('.') { format!("0{st}") } else { st };
    let st = st.replace(".e", ".0e");
    if st.ends_with('.') { format!("{st}0") } else { st }
}

/// Converts a CRT-collected float text to `f64`, correctly rounded (the `double` path and
/// `atof`).
pub fn parse_float_text(text: &[u8]) -> f64 {
    normalise_float_text(text).parse::<f64>().unwrap_or(0.0)
}

/// Converts a CRT-collected float text to `f32`, rounded once (the CRT's `float` path).
pub fn parse_float_text_f32(text: &[u8]) -> f32 {
    normalise_float_text(text).parse::<f32>().unwrap_or(0.0)
}

/// `atof` (MSVC `strtod` semantics, then the caller's `(float)` cast is separate): skips leading
/// whitespace, reads the longest float prefix, 0.0 when there is none.
pub fn atof(s: &[u8]) -> f64 {
    let mut i = 0;
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }
    let start = i;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        i += 1;
    }
    let mut digits = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() && s[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return 0.0;
    }
    if i < s.len() && (s[i] == b'e' || s[i] == b'E') {
        let mut j = i + 1;
        if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
            j += 1;
        }
        let ds = j;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
        }
        if j > ds {
            i = j;
        }
    }
    let text: Vec<u8> = s[start..i].iter().copied().filter(|&c| c != b'+').collect();
    parse_float_text(&text)
}

/// `atoi` (MSVC: skip whitespace, optional sign, decimal digits, 32-bit wrap-around).
pub fn atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut v: i32 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((s[i] - b'0') as i32);
        i += 1;
    }
    if neg { v.wrapping_neg() } else { v }
}

/// Runs `sscanf(input, format, ...)`. `input` ends at its first NUL (C string).
pub fn sscanf(input: &[u8], format: &str) -> Scan {
    let input = crate::bytes::c_str(input);
    let fmt = format.as_bytes();
    let mut s = Scanner { inp: input, pos: 0 };
    let mut vals: Vec<Option<Val>> = Vec::new();
    // Pre-count the assignment slots so that unreached conversions are reported as None.
    let slots = count_slots(fmt);
    let mut count: i32 = 0;
    let mut any_conversion_attempted = false;
    let mut f = 0usize;
    let mut eof_before_first = false;

    'outer: while f < fmt.len() {
        let c = fmt[f];
        if is_space(c) {
            while f < fmt.len() && is_space(fmt[f]) {
                f += 1;
            }
            s.skip_ws();
            continue;
        }
        if c != b'%' {
            // Literal byte.
            if s.peek() == Some(c) {
                s.pos += 1;
                f += 1;
                continue;
            }
            if s.peek().is_none() && !any_conversion_attempted {
                eof_before_first = true;
            }
            break;
        }
        f += 1;
        if f < fmt.len() && fmt[f] == b'%' {
            s.skip_ws();
            if s.peek() == Some(b'%') {
                s.pos += 1;
                f += 1;
                continue;
            }
            break;
        }
        let mut suppress = false;
        if f < fmt.len() && fmt[f] == b'*' {
            suppress = true;
            f += 1;
        }
        let mut width: Option<usize> = None;
        while f < fmt.len() && fmt[f].is_ascii_digit() {
            width = Some(width.unwrap_or(0) * 10 + (fmt[f] - b'0') as usize);
            f += 1;
        }
        let mut long = false;
        while f < fmt.len() && matches!(fmt[f], b'h' | b'l' | b'L' | b'F' | b'N') {
            if fmt[f] == b'l' || fmt[f] == b'L' {
                long = true;
            }
            f += 1;
        }
        let Some(&conv) = fmt.get(f) else { break };
        f += 1;
        // The CRT treats a width of 0 like no width.
        let width = width.filter(|&w| w > 0);

        if conv == b'n' {
            if !suppress {
                vals.push(Some(Val::Int(s.pos as i32)));
            }
            continue;
        }
        if conv != b'c' && conv != b'[' {
            s.skip_ws();
        }
        if s.peek().is_none() {
            if !any_conversion_attempted {
                eof_before_first = true;
            }
            break;
        }
        any_conversion_attempted = true;

        let val = match conv {
            b'd' | b'u' | b'i' | b'x' | b'X' | b'o' | b'p' => match s.integer(conv, width) {
                Some(v) => Val::Int(v),
                None => break 'outer,
            },
            b'f' | b'e' | b'E' | b'g' | b'G' => match s.float_text(width) {
                Some(t) => {
                    if long {
                        Val::Double(parse_float_text(&t))
                    } else {
                        Val::Float(parse_float_text_f32(&t))
                    }
                }
                None => break 'outer,
            },
            b's' => {
                let max = width.unwrap_or(usize::MAX);
                let start = s.pos;
                while s.pos - start < max {
                    match s.peek() {
                        Some(b) if !is_space(b) => s.pos += 1,
                        _ => break,
                    }
                }
                Val::Str(input[start..s.pos].to_vec())
            }
            b'c' => {
                let n = width.unwrap_or(1);
                let start = s.pos;
                let end = (start + n).min(input.len());
                s.pos = end;
                Val::Str(input[start..end].to_vec())
            }
            b'[' => {
                // Parse the scan set.
                let mut negate = false;
                if f < fmt.len() && fmt[f] == b'^' {
                    negate = true;
                    f += 1;
                }
                let set_start = f;
                if f < fmt.len() && fmt[f] == b']' {
                    f += 1;
                }
                while f < fmt.len() && fmt[f] != b']' {
                    f += 1;
                }
                let set = &fmt[set_start..f];
                f += 1; // ']'
                let in_set = |b: u8| -> bool {
                    let mut i = 0;
                    let mut hit = false;
                    while i < set.len() {
                        if i + 2 < set.len() && set[i + 1] == b'-' {
                            let (lo, hi) = (set[i].min(set[i + 2]), set[i].max(set[i + 2]));
                            if (lo..=hi).contains(&b) {
                                hit = true;
                            }
                            i += 3;
                        } else {
                            if set[i] == b {
                                hit = true;
                            }
                            i += 1;
                        }
                    }
                    hit != negate
                };
                let max = width.unwrap_or(usize::MAX);
                let start = s.pos;
                while s.pos - start < max {
                    match s.peek() {
                        Some(b) if in_set(b) => s.pos += 1,
                        _ => break,
                    }
                }
                if s.pos == start {
                    break 'outer;
                }
                Val::Str(input[start..s.pos].to_vec())
            }
            _ => break 'outer,
        };
        if !suppress {
            vals.push(Some(val));
            count += 1;
        }
    }
    while vals.len() < slots {
        vals.push(None);
    }
    let ret = if count == 0 && eof_before_first { -1 } else { count };
    Scan { ret, vals }
}

fn count_slots(fmt: &[u8]) -> usize {
    let mut n = 0;
    let mut f = 0;
    while f < fmt.len() {
        if fmt[f] != b'%' {
            f += 1;
            continue;
        }
        f += 1;
        if f < fmt.len() && fmt[f] == b'%' {
            f += 1;
            continue;
        }
        let suppress = f < fmt.len() && fmt[f] == b'*';
        while f < fmt.len() && !fmt[f].is_ascii_alphabetic() && fmt[f] != b'[' {
            f += 1;
        }
        while f < fmt.len() && matches!(fmt[f], b'h' | b'l' | b'L' | b'F' | b'N') {
            f += 1;
        }
        if f < fmt.len() && fmt[f] == b'[' {
            f += 1;
            if f < fmt.len() && fmt[f] == b'^' {
                f += 1;
            }
            if f < fmt.len() && fmt[f] == b']' {
                f += 1;
            }
            while f < fmt.len() && fmt[f] != b']' {
                f += 1;
            }
        }
        f += 1;
        if !suppress {
            n += 1;
        }
    }
    n
}

struct Scanner<'a> {
    inp: &'a [u8],
    pos: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<u8> {
        self.inp.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b) if is_space(b)) {
            self.pos += 1;
        }
    }

    /// MSVC `getnum`: sign, optional `0x` for hex, digits; wrap-around accumulation.
    fn integer(&mut self, conv: u8, width: Option<usize>) -> Option<i32> {
        let mut left = width.unwrap_or(usize::MAX);
        let mut neg = false;
        let mut base: u32 = match conv {
            b'x' | b'X' | b'p' => 16,
            b'o' => 8,
            b'i' => 0,
            _ => 10,
        };
        if let Some(b @ (b'+' | b'-')) = self.peek() {
            neg = b == b'-';
            self.pos += 1;
            left -= 1;
            if left == 0 {
                return None;
            }
        }
        let mut started = false;
        if (base == 16 || base == 0) && self.peek() == Some(b'0') {
            // Leading 0: may be a 0x prefix (hex/%i) or an octal marker (%i).
            self.pos += 1;
            left -= 1;
            started = true;
            if left > 0 && matches!(self.peek(), Some(b'x' | b'X')) {
                self.pos += 1;
                left -= 1;
                started = false;
                base = 16;
            } else if base == 0 {
                base = 8;
            }
        }
        if base == 0 {
            base = 10;
        }
        let mut v: i32 = 0;
        while left > 0 {
            let Some(b) = self.peek() else { break };
            let d = match (b as char).to_digit(base) {
                Some(d) => d,
                None => break,
            };
            v = if base == 16 {
                v.wrapping_shl(4).wrapping_add(d as i32)
            } else if base == 8 {
                v.wrapping_shl(3).wrapping_add(d as i32)
            } else {
                v.wrapping_mul(10).wrapping_add(d as i32)
            };
            started = true;
            self.pos += 1;
            left -= 1;
        }
        if !started {
            return None;
        }
        Some(if neg { v.wrapping_neg() } else { v })
    }

    /// MSVC float field collection. Returns the collected text, or None when no digit occurred.
    fn float_text(&mut self, width: Option<usize>) -> Option<Vec<u8>> {
        // The CRT caps the width at its conversion buffer size (CVTBUFSIZE = 349).
        let mut left: isize = width.map_or(349, |w| w.min(349)) as isize;
        let mut out = Vec::new();
        let mut started = false;
        if let Some(b @ (b'+' | b'-')) = self.peek() {
            if b == b'-' {
                out.push(b'-');
            }
            self.pos += 1;
            left -= 1;
        }
        while left > 0 {
            match self.peek() {
                Some(b) if b.is_ascii_digit() => {
                    out.push(b);
                    self.pos += 1;
                    left -= 1;
                    started = true;
                }
                _ => break,
            }
        }
        if self.peek() == Some(b'.') && left > 0 {
            left -= 1;
            self.pos += 1;
            out.push(b'.');
            while left > 0 {
                match self.peek() {
                    Some(b) if b.is_ascii_digit() => {
                        out.push(b);
                        self.pos += 1;
                        left -= 1;
                        started = true;
                    }
                    _ => break,
                }
            }
        }
        if started && matches!(self.peek(), Some(b'e' | b'E')) && left > 0 {
            left -= 1;
            self.pos += 1;
            out.push(b'e');
            if let Some(b @ (b'+' | b'-')) = self.peek()
                && left > 0
            {
                left -= 1;
                self.pos += 1;
                if b == b'-' {
                    out.push(b'-');
                }
            }
            while left > 0 {
                match self.peek() {
                    Some(b) if b.is_ascii_digit() => {
                        out.push(b);
                        self.pos += 1;
                        left -= 1;
                    }
                    _ => break,
                }
            }
        }
        started.then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_fields() {
        let s = sscanf(b"  12 abc 3.5 -7", "%d %s %f %d");
        assert_eq!(s.ret, 4);
        assert_eq!(s.i32(0), Some(12));
        assert_eq!(s.bytes(1), Some(&b"abc"[..]));
        assert_eq!(s.f32(2), Some(3.5));
        assert_eq!(s.i32(3), Some(-7));
    }

    #[test]
    fn partial_and_eof() {
        let s = sscanf(b"5 x", "%d %d %d");
        assert_eq!(s.ret, 1);
        assert_eq!(s.vals.len(), 3);
        assert_eq!(s.get(1), None);
        assert_eq!(sscanf(b"", "%d").ret, -1);
        assert_eq!(sscanf(b"   ", "%s").ret, -1);
        assert_eq!(sscanf(b"x", "%d").ret, 0);
        // Input ends after the first conversion: count, not EOF.
        assert_eq!(sscanf(b"1 ", "%d %d").ret, 1);
    }

    #[test]
    fn integers() {
        assert_eq!(sscanf(b"1.5", "%d").i32(0), Some(1));
        assert_eq!(sscanf(b"4294967295", "%d").i32(0), Some(-1));
        assert_eq!(sscanf(b"0x1F", "%x").i32(0), Some(31));
        assert_eq!(sscanf(b"ff", "%x").i32(0), Some(255));
        assert_eq!(sscanf(b"-", "%d").ret, 0);
        assert_eq!(sscanf(b"010", "%i").i32(0), Some(8));
    }

    #[test]
    fn floats() {
        assert_eq!(sscanf(b"1.", "%f").f32(0), Some(1.0));
        assert_eq!(sscanf(b".25", "%f").f32(0), Some(0.25));
        assert_eq!(sscanf(b"1e3", "%f").f32(0), Some(1000.0));
        assert_eq!(sscanf(b"2e", "%f").f32(0), Some(2.0));
        assert_eq!(sscanf(b"-0.5x", "%f%s").f32(0), Some(-0.5));
        assert_eq!(sscanf(b".", "%f").ret, 0);
        assert_eq!(sscanf(b"0.1", "%lf").f64(0), Some(0.1));
        // A float followed directly by text: the scanner stops at the first non-float byte.
        let s = sscanf(b"5.0f 1", "%f %d");
        assert_eq!(s.ret, 1);
    }

    #[test]
    fn literals_sets_and_chars() {
        let s = sscanf(b"a=12", "a=%d");
        assert_eq!(s.i32(0), Some(12));
        let s = sscanf(b"abc,def", "%[^,],%s");
        assert_eq!(s.bytes(0), Some(&b"abc"[..]));
        assert_eq!(s.bytes(1), Some(&b"def"[..]));
        let s = sscanf(b" xy", "%c%c");
        assert_eq!(s.bytes(0), Some(&b" "[..]));
        let s = sscanf(b"12 rest", "%d %*s %n");
        assert_eq!(s.ret, 1);
        assert_eq!(s.i32(1), Some(7));
        let s = sscanf(b"abcdef", "%3s%s");
        assert_eq!(s.bytes(0), Some(&b"abc"[..]));
        assert_eq!(s.bytes(1), Some(&b"def"[..]));
    }

    #[test]
    fn atof_atoi() {
        assert_eq!(atof(b"  1.5abc"), 1.5);
        assert_eq!(atof(b"x"), 0.0);
        assert_eq!(atof(b"-2e2"), -200.0);
        assert_eq!(atoi(b" -12z"), -12);
    }
}
