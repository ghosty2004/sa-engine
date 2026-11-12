//! Embedded colour codes `{RRGGBB}` (samp.dll 0x100b5fd0 parse, 0x100b6100 strip) and the colouring of
//! the glyphs that follow them.
//!
//! Colours are `0xAARRGGBB` (D3DCOLOR) inside this crate. A code sets `0xFF000000 | RRGGBB`: the base
//! colour's alpha only applies until the first code, as in samp.

/// Length of one code: `{` + six hex digits + `}`.
pub const CODE_LEN: usize = 8;

/// Parses a code at the start of `s`: exactly `{` + six hex digits (either case) + `}`.
pub fn parse_code(s: &[char]) -> Option<u32> {
    if s.len() < CODE_LEN || s[0] != '{' || s[7] != '}' {
        return None;
    }
    let mut v = 0u32;
    for c in &s[1..7] {
        v = (v << 4) | c.to_digit(16)?;
    }
    Some(v)
}

/// Removes every valid code. After a removal the scan continues at the same index, so a code that only
/// forms after a removal is removed too (samp's in-place removal).
pub fn strip_codes(s: &str) -> String {
    let mut out: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < out.len() {
        if parse_code(&out[i..]).is_some() {
            out.drain(i..i + CODE_LEN);
        } else {
            i += 1;
        }
    }
    out.into_iter().collect()
}

/// Characters D3DX draws no glyph for.
pub const fn is_blank(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// A run of stripped text in one colour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColourRun {
    /// The text (no codes).
    pub text: String,
    /// `0xAARRGGBB`.
    pub colour: u32,
}

/// Splits `text` into runs of one colour, colouring glyph by glyph as samp's colour-code font wrapper does:
/// for every visible glyph of the stripped text a cursor in the original text skips blanks, consumes any
/// run of codes, then steps over one character. Blanks join the run before them.
pub fn colour_runs(text: &str, base: u32) -> Vec<ColourRun> {
    let orig: Vec<char> = text.chars().collect();
    let stripped: Vec<char> = strip_codes(text).chars().collect();
    let mut colour = base;
    let mut cur = 0usize;
    let mut runs: Vec<ColourRun> = Vec::new();
    for &c in &stripped {
        if is_blank(c) {
            match runs.last_mut() {
                Some(r) => r.text.push(c),
                None => runs.push(ColourRun {
                    text: c.to_string(),
                    colour: base,
                }),
            }
            continue;
        }
        while cur < orig.len() && is_blank(orig[cur]) {
            cur += 1;
        }
        while let Some(rgb) = orig.get(cur..).and_then(parse_code) {
            colour = 0xFF00_0000 | rgb;
            cur += CODE_LEN;
        }
        cur += 1;
        match runs.last_mut() {
            Some(r) if r.colour == colour => r.text.push(c),
            _ => runs.push(ColourRun {
                text: c.to_string(),
                colour,
            }),
        }
    }
    runs
}

/// The colour in effect at the end of `text` (to continue a wrapped line in the same colour).
pub fn last_colour(text: &str, base: u32) -> u32 {
    let c: Vec<char> = text.chars().collect();
    let mut colour = base;
    for i in 0..c.len() {
        if let Some(rgb) = parse_code(&c[i..]) {
            colour = 0xFF00_0000 | rgb;
        }
    }
    colour
}

/// samp's wire colour `0xRRGGBBAA` → `0xFFRRGGBB` (alpha forced, as the chat does).
pub const fn rgba_to_argb_opaque(rgba: u32) -> u32 {
    (rgba >> 8) | 0xFF00_0000
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn code_is_brace_six_hex_brace() {
        assert_eq!(parse_code(&chars("{FF00aa}x")), Some(0xFF00AA));
        assert_eq!(parse_code(&chars("{FF00A}")), None);
        assert_eq!(parse_code(&chars("{FF00AG}")), None);
        assert_eq!(parse_code(&chars("FF00AA}")), None);
    }

    #[test]
    fn strip_removes_valid_codes_only_in_place() {
        assert_eq!(
            strip_codes("{FF0000}red {00ff00}green {xyz} {12345}"),
            "red green {xyz} {12345}"
        );
        assert_eq!(strip_codes("{FF0000}{00FF00}"), "");
        assert_eq!(strip_codes("{{FF0000}FF0000}"), "{FF0000}");
    }

    #[test]
    fn runs_follow_codes_and_alpha_applies_until_first_code() {
        let r = colour_runs("ab{112233}cd", 0x80FF_FFFF);
        assert_eq!(
            r,
            vec![
                ColourRun {
                    text: "ab".into(),
                    colour: 0x80FF_FFFF
                },
                ColourRun {
                    text: "cd".into(),
                    colour: 0xFF11_2233
                }
            ]
        );
        let r = colour_runs("hi {FF0000}there you", 0xFFFF_FFFF);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].text, "hi ");
        assert_eq!((r[1].text.as_str(), r[1].colour), ("there you", 0xFFFF_0000));
    }

    #[test]
    fn code_followed_by_blank_shifts_like_the_original() {
        // The cursor steps over the blank as if it were the glyph; the second code applies one glyph late.
        let r = colour_runs("{FF0000} {00FF00}xy", 0xFFFF_FFFF);
        let got: Vec<(String, u32)> = r.into_iter().map(|r| (r.text, r.colour)).collect();
        assert_eq!(
            got,
            vec![
                (" ".into(), 0xFFFF_FFFF),
                ("x".into(), 0xFFFF_0000),
                ("y".into(), 0xFF00_FF00)
            ]
        );
    }

    #[test]
    fn last_colour_and_wire() {
        assert_eq!(last_colour("a{00FF00}b{0000FF}c", 0xFFFF_FFFF), 0xFF00_00FF);
        assert_eq!(last_colour("plain", 0x1234_5678), 0x1234_5678);
        assert_eq!(rgba_to_argb_opaque(0x1122_3300), 0xFF11_2233);
    }
}
