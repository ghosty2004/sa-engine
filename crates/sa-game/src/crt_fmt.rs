//! `printf`/`scanf` formatting as the statically linked MSVC CRT of gta_sa.exe does it, for the
//! native CRT replacements ([`crate::crt`]). Arguments come as the raw 32-bit stack words the
//! caller pushed (a `double` takes two).

/// The variadic arguments: consecutive 32-bit stack words.
pub struct Args<'a> {
    words: &'a dyn Fn(usize) -> u32,
    next: usize,
}

impl<'a> Args<'a> {
    pub fn new(words: &'a dyn Fn(usize) -> u32) -> Args<'a> {
        Args { words, next: 0 }
    }
    fn u32(&mut self) -> u32 {
        let v = (self.words)(self.next);
        self.next += 1;
        v
    }
    fn u64(&mut self) -> u64 {
        let lo = self.u32() as u64;
        lo | (self.u32() as u64) << 32
    }
    fn f64(&mut self) -> f64 {
        f64::from_bits(self.u64())
    }
}

/// The NUL-terminated bytes at game address `p`.
pub fn cbytes(p: u32) -> Vec<u8> {
    let mut v = Vec::new();
    let mut a = p as usize;
    loop {
        let c = sa_core::mem::GPtr::<u8>::from_addr(a).read();
        if c == 0 {
            return v;
        }
        v.push(c);
        a += 1;
    }
}

/// MSVC's exponent: at least three digits (`1.500000e+002`).
fn msvc_exp(s: String) -> String {
    match s.find(['e', 'E']) {
        Some(i) => {
            let (m, e) = s.split_at(i);
            let (letter, rest) = e.split_at(1);
            let (sign, digits) = match rest.strip_prefix('-') {
                Some(d) => ("-", d),
                None => ("+", rest.strip_prefix('+').unwrap_or(rest)),
            };
            format!("{m}{letter}{sign}{digits:0>3}")
        }
        None => s,
    }
}

fn fmt_e(v: f64, prec: usize, upper: bool) -> String {
    let s = msvc_exp(format!("{v:.prec$e}"));
    if upper { s.to_uppercase() } else { s }
}

fn fmt_g(v: f64, prec: usize, alt: bool, upper: bool) -> String {
    let p = if prec == 0 { 1 } else { prec };
    if v == 0.0 {
        return if alt { format!("{:.*}", p - 1, 0.0) } else { "0".into() };
    }
    let exp = v.abs().log10().floor() as i32;
    // the exponent after rounding to p significant digits
    let rounded: f64 = format!("{v:.prec$e}", prec = p - 1).parse().unwrap_or(v);
    let exp = if rounded != 0.0 { rounded.abs().log10().floor() as i32 } else { exp };
    let mut s = if exp < -4 || exp >= p as i32 {
        fmt_e(v, p - 1, upper)
    } else {
        format!("{v:.prec$}", prec = (p as i32 - 1 - exp).max(0) as usize)
    };
    if !alt && s.contains('.') {
        let (m, e) = match s.find(['e', 'E']) {
            Some(i) => (s[..i].to_string(), s[i..].to_string()),
            None => (s.clone(), String::new()),
        };
        let m = m.trim_end_matches('0').trim_end_matches('.').to_string();
        s = m + &e;
    }
    s
}

/// Formats `fmt` with `args` (MSVC `sprintf`).
pub fn format(fmt: &[u8], args: &mut Args) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < fmt.len() {
        let c = fmt[i];
        i += 1;
        if c != b'%' {
            out.push(c);
            continue;
        }
        let (mut left, mut plus, mut space, mut alt, mut zero) = (false, false, false, false, false);
        while i < fmt.len() {
            match fmt[i] {
                b'-' => left = true,
                b'+' => plus = true,
                b' ' => space = true,
                b'#' => alt = true,
                b'0' => zero = true,
                _ => break,
            }
            i += 1;
        }
        let mut width = 0usize;
        if fmt.get(i) == Some(&b'*') {
            let w = args.u32() as i32;
            if w < 0 {
                left = true;
            }
            width = w.unsigned_abs() as usize;
            i += 1;
        } else {
            while let Some(d @ b'0'..=b'9') = fmt.get(i) {
                width = width * 10 + (d - b'0') as usize;
                i += 1;
            }
        }
        let mut prec: Option<usize> = None;
        if fmt.get(i) == Some(&b'.') {
            i += 1;
            let mut p = 0usize;
            if fmt.get(i) == Some(&b'*') {
                let v = args.u32() as i32;
                i += 1;
                prec = (v >= 0).then_some(v as usize);
            } else {
                while let Some(d @ b'0'..=b'9') = fmt.get(i) {
                    p = p * 10 + (d - b'0') as usize;
                    i += 1;
                }
                prec = Some(p);
            }
        }
        let mut wide = false;
        loop {
            match fmt.get(i) {
                Some(b'h') | Some(b'l') | Some(b'L') => i += 1,
                Some(b'I') if fmt.get(i + 1..i + 3) == Some(b"64") => {
                    wide = true;
                    i += 3;
                }
                _ => break,
            }
        }
        let Some(&ty) = fmt.get(i) else { break };
        i += 1;
        let mut sign = String::new();
        let body: Vec<u8> = match ty {
            b'%' => b"%".to_vec(),
            b'c' => vec![args.u32() as u8],
            b's' => {
                let p = args.u32();
                let s = if p == 0 { b"(null)".to_vec() } else { cbytes(p) };
                match prec {
                    Some(n) => s[..s.len().min(n)].to_vec(),
                    None => s,
                }
            }
            b'd' | b'i' => {
                let v = if wide { args.u64() as i64 } else { args.u32() as i32 as i64 };
                if v < 0 {
                    sign = "-".into();
                } else if plus {
                    sign = "+".into();
                } else if space {
                    sign = " ".into();
                }
                let mut d = v.unsigned_abs().to_string();
                if let Some(p) = prec {
                    if p == 0 && v == 0 {
                        d.clear();
                    }
                    while d.len() < p {
                        d.insert(0, '0');
                    }
                }
                d.into_bytes()
            }
            b'u' | b'x' | b'X' | b'o' => {
                let v = if wide { args.u64() } else { args.u32() as u64 };
                let mut d = match ty {
                    b'u' => v.to_string(),
                    b'x' => format!("{v:x}"),
                    b'X' => format!("{v:X}"),
                    _ => format!("{v:o}"),
                };
                if let Some(p) = prec {
                    if p == 0 && v == 0 {
                        d.clear();
                    }
                    while d.len() < p {
                        d.insert(0, '0');
                    }
                }
                if alt && v != 0 {
                    match ty {
                        b'x' => d.insert_str(0, "0x"),
                        b'X' => d.insert_str(0, "0X"),
                        b'o' => d.insert(0, '0'),
                        _ => {}
                    }
                }
                d.into_bytes()
            }
            b'p' => format!("{:08X}", args.u32()).into_bytes(),
            b'f' | b'e' | b'E' | b'g' | b'G' => {
                let v = args.f64();
                if v.is_sign_negative() && !(v == 0.0 && !v.is_sign_negative()) && !v.is_nan() {
                    sign = "-".into();
                } else if plus {
                    sign = "+".into();
                } else if space {
                    sign = " ".into();
                }
                let a = v.abs();
                let p = prec.unwrap_or(6);
                let s = if a.is_infinite() {
                    "1.#INF00".to_string()
                } else if a.is_nan() {
                    "1.#QNAN0".to_string()
                } else {
                    match ty {
                        b'f' => {
                            let s = format!("{a:.p$}");
                            if alt && p == 0 { s + "." } else { s }
                        }
                        b'e' => fmt_e(a, p, false),
                        b'E' => fmt_e(a, p, true),
                        b'g' => fmt_g(a, p, alt, false),
                        _ => fmt_g(a, p, alt, true),
                    }
                };
                s.into_bytes()
            }
            b'n' => {
                let p = args.u32();
                if p != 0 {
                    sa_core::mem::GPtr::<i32>::from_addr(p as usize).write(out.len() as i32);
                }
                Vec::new()
            }
            other => vec![b'%', other],
        };
        let len = sign.len() + body.len();
        let pad = width.saturating_sub(len);
        let numeric = matches!(ty, b'd' | b'i' | b'u' | b'x' | b'X' | b'o' | b'f' | b'e' | b'E' | b'g' | b'G');
        if left {
            out.extend_from_slice(sign.as_bytes());
            out.extend_from_slice(&body);
            out.extend(std::iter::repeat_n(b' ', pad));
        } else if zero && numeric && !(prec.is_some() && matches!(ty, b'd' | b'i' | b'u' | b'x' | b'X' | b'o')) {
            out.extend_from_slice(sign.as_bytes());
            out.extend(std::iter::repeat_n(b'0', pad));
            out.extend_from_slice(&body);
        } else {
            out.extend(std::iter::repeat_n(b' ', pad));
            out.extend_from_slice(sign.as_bytes());
            out.extend_from_slice(&body);
        }
    }
    out
}

/// One `sscanf` result to store.
#[derive(Debug, PartialEq)]
pub enum Store {
    I32(i32),
    I16(i16),
    F32(f32),
    F64(f64),
    Bytes(Vec<u8>, bool),
}

/// Scans `input` with `fmt` (MSVC `sscanf`): calls `store(index, value)` for every assigned
/// conversion and returns the number assigned, or -1 if the input ended before the first
/// conversion.
pub fn scan(input: &[u8], fmt: &[u8], store: &mut dyn FnMut(usize, Store)) -> i32 {
    let (mut i, mut f, mut n) = (0usize, 0usize, 0usize);
    let ws = |c: u8| matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C);
    let mut any_input = false;
    while f < fmt.len() {
        let c = fmt[f];
        if ws(c) {
            while f < fmt.len() && ws(fmt[f]) {
                f += 1;
            }
            while i < input.len() && ws(input[i]) {
                i += 1;
            }
            continue;
        }
        if c != b'%' || fmt.get(f + 1) == Some(&b'%') {
            let lit = if c == b'%' { f += 2; b'%' } else { f += 1; c };
            if input.get(i) == Some(&lit) {
                i += 1;
                continue;
            }
            break;
        }
        f += 1;
        let suppress = fmt.get(f) == Some(&b'*');
        if suppress {
            f += 1;
        }
        let mut width = 0usize;
        while let Some(d @ b'0'..=b'9') = fmt.get(f) {
            width = width * 10 + (d - b'0') as usize;
            f += 1;
        }
        let (mut half, mut long) = (false, false);
        while let Some(m) = fmt.get(f) {
            match m {
                b'h' => half = true,
                b'l' | b'L' => long = true,
                _ => break,
            }
            f += 1;
        }
        let Some(&ty) = fmt.get(f) else { break };
        f += 1;
        let maxw = if width == 0 { usize::MAX } else { width };
        if ty != b'c' && ty != b'[' && ty != b'n' {
            while i < input.len() && ws(input[i]) {
                i += 1;
            }
        }
        if i >= input.len() && ty != b'n' {
            return if any_input || n > 0 { n as i32 } else { -1 };
        }
        any_input = true;
        match ty {
            b'd' | b'i' | b'u' | b'x' | b'X' => {
                let radix = if matches!(ty, b'x' | b'X') { 16 } else { 10 };
                let start = i;
                let mut j = i;
                if j < input.len() && (input[j] == b'-' || input[j] == b'+') && j - start < maxw {
                    j += 1;
                }
                if radix == 16 && input.get(j) == Some(&b'0') && matches!(input.get(j + 1), Some(b'x' | b'X')) {
                    j += 2;
                }
                let digits_start = j;
                while j < input.len() && j - start < maxw && (input[j] as char).is_digit(radix) {
                    j += 1;
                }
                if j == digits_start {
                    break;
                }
                let neg = input[start] == b'-';
                let mag = u64::from_str_radix(
                    std::str::from_utf8(&input[digits_start..j]).unwrap_or("0"),
                    radix,
                )
                .unwrap_or(u64::MAX) as u32;
                let v = if neg { mag.wrapping_neg() } else { mag } as i32;
                i = j;
                if !suppress {
                    store(n, if half { Store::I16(v as i16) } else { Store::I32(v) });
                    n += 1;
                }
            }
            b'f' | b'e' | b'g' | b'E' | b'G' => {
                let start = i;
                let mut j = i;
                let mut seen_digit = false;
                if j < input.len() && (input[j] == b'-' || input[j] == b'+') {
                    j += 1;
                }
                while j < input.len() && j - start < maxw && input[j].is_ascii_digit() {
                    j += 1;
                    seen_digit = true;
                }
                if j < input.len() && j - start < maxw && input[j] == b'.' {
                    j += 1;
                    while j < input.len() && j - start < maxw && input[j].is_ascii_digit() {
                        j += 1;
                        seen_digit = true;
                    }
                }
                if seen_digit && j < input.len() && j - start < maxw && matches!(input[j], b'e' | b'E') {
                    let mut k = j + 1;
                    if k < input.len() && (input[k] == b'-' || input[k] == b'+') {
                        k += 1;
                    }
                    if k < input.len() && input[k].is_ascii_digit() {
                        while k < input.len() && k - start < maxw && input[k].is_ascii_digit() {
                            k += 1;
                        }
                        j = k;
                    }
                }
                if !seen_digit {
                    break;
                }
                let v: f64 = std::str::from_utf8(&input[start..j]).ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                i = j;
                if !suppress {
                    store(n, if long { Store::F64(v) } else { Store::F32(v as f32) });
                    n += 1;
                }
            }
            b's' => {
                let start = i;
                while i < input.len() && !ws(input[i]) && i - start < maxw {
                    i += 1;
                }
                if !suppress {
                    store(n, Store::Bytes(input[start..i].to_vec(), true));
                    n += 1;
                }
            }
            b'c' => {
                let w = if width == 0 { 1 } else { width };
                let end = (i + w).min(input.len());
                if !suppress {
                    store(n, Store::Bytes(input[i..end].to_vec(), false));
                    n += 1;
                }
                i = end;
            }
            b'n' => {
                if !suppress {
                    store(n, Store::I32(i as i32));
                }
            }
            _ => break,
        }
    }
    n as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprintf(fmt: &str, words: &[u32]) -> String {
        let w = |i: usize| words.get(i).copied().unwrap_or(0);
        String::from_utf8(format(fmt.as_bytes(), &mut Args::new(&w))).unwrap()
    }
    fn d(v: f64) -> [u32; 2] {
        let b = v.to_bits();
        [b as u32, (b >> 32) as u32]
    }

    #[test]
    fn printf_msvc() {
        assert_eq!(sprintf("%d|%5d|%-5d|%05d|%+d", &[42, 7, 7, 7, 3]), "42|    7|7    |00007|+3");
        assert_eq!(sprintf("%x %X %08x %#x", &[255, 255, 255, 255]), "ff FF 000000ff 0xff");
        assert_eq!(sprintf("%u %i", &[u32::MAX, (-5i32) as u32]), "4294967295 -5");
        assert_eq!(sprintf("%c%%", &[b'A' as u32]), "A%");
        assert_eq!(sprintf("%.2f", &d(1.23456)), "1.23");
        assert_eq!(sprintf("%f", &d(-1.5)), "-1.500000");
        assert_eq!(sprintf("%e", &d(150.0)), "1.500000e+002");
        assert_eq!(sprintf("%g", &d(0.0001)), "0.0001");
        assert_eq!(sprintf("%g", &d(1234567.0)), "1.23457e+006");
        assert_eq!(sprintf("%g", &d(100.0)), "100");
        assert_eq!(sprintf("%p", &[0x1234]), "00001234");
    }

    #[test]
    fn scanf_msvc() {
        let mut got = Vec::new();
        let n = scan(b" 12 -3.5 abc x", b"%d %f %s %c", &mut |_, v| got.push(v));
        assert_eq!(n, 4);
        assert_eq!(
            got,
            vec![
                Store::I32(12),
                Store::F32(-3.5),
                Store::Bytes(b"abc".to_vec(), true),
                Store::Bytes(b"x".to_vec(), false)
            ]
        );
        assert_eq!(scan(b"", b"%d", &mut |_, _| {}), -1);
        assert_eq!(scan(b"a", b"%d", &mut |_, _| {}), 0);
        let mut h = Vec::new();
        assert_eq!(scan(b"ff,7", b"%x,%*d", &mut |_, v| h.push(v)), 1);
        assert_eq!(h, vec![Store::I32(255)]);
    }
}
