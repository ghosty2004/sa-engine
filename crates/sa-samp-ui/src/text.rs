//! Text helpers: control characters, samp's character-count line split of player chat, and pixel-width
//! word wrapping for the wider texts (dialog bodies are not wrapped by samp; labels and chat are).

use crate::colour::{CODE_LEN, last_colour, parse_code};
use crate::font::{FontId, Measure};

/// Replaces control characters 1..=31 with spaces (samp.dll 0x10067420).
pub fn replace_control_chars(s: &str) -> String {
    s.chars()
        .map(|c| if ('\u{1}'..='\u{1f}').contains(&c) { ' ' } else { c })
        .collect()
}

/// Splits `text` once at `width` characters, preferring a space at most `backtrack` characters before it
/// (the space is dropped); otherwise a hard split. samp wraps player chat at 72 with a 12-character
/// backtrack.
pub fn split_line(text: &str, width: usize, backtrack: usize) -> (String, Option<String>) {
    let c: Vec<char> = text.chars().collect();
    if c.len() <= width {
        return (text.into(), None);
    }
    let low = width.saturating_sub(backtrack);
    match (low..=width).rev().find(|&i| c[i] == ' ') {
        Some(i) => (c[..i].iter().collect(), Some(c[i + 1..].iter().collect())),
        None => (c[..width].iter().collect(), Some(c[width..].iter().collect())),
    }
}

/// Wraps `text` (with colour codes) to lines no wider than `max_w` pixels, breaking at spaces where
/// possible. Codes do not count towards the width, and a line that continues after a break starts with the
/// colour in effect at the break (`{RRGGBB}` prefix), so every line can be drawn on its own.
pub fn wrap_pixels(m: &dyn Measure, font: FontId, text: &str, max_w: f32) -> Vec<String> {
    let mut out = Vec::new();
    // The colour carries over line breaks, explicit or not (D3DX draws the block glyph by glyph).
    let mut prefix = String::new();
    for para in text.split('\n') {
        let c: Vec<char> = para.chars().collect();
        let mut line_start = 0usize;
        loop {
            // Walk visible characters until the width is exceeded.
            let mut i = line_start;
            let mut w = 0.0;
            let mut last_space: Option<usize> = None;
            let mut broke = None;
            while i < c.len() {
                if parse_code(&c[i..]).is_some() {
                    i += CODE_LEN;
                    continue;
                }
                let cw = m.width(font, &c[i].to_string());
                if w + cw > max_w && i > line_start {
                    broke = Some(i);
                    break;
                }
                if c[i] == ' ' {
                    last_space = Some(i);
                }
                w += cw;
                i += 1;
            }
            let Some(at) = broke else {
                let full = format!("{prefix}{}", c[line_start..].iter().collect::<String>());
                let col = last_colour(&full, 0);
                prefix = if col != 0 {
                    format!("{{{:06X}}}", col & 0xFF_FFFF)
                } else {
                    String::new()
                };
                out.push(full);
                break;
            };
            let (end, next) = match last_space {
                _ if c[at] == ' ' => (at, at + 1),
                Some(s) if s > line_start => (s, s + 1),
                _ => (at, at),
            };
            let line: String = c[line_start..end].iter().collect();
            let full = format!("{prefix}{line}");
            let col = last_colour(&full, 0);
            prefix = if col != 0 {
                format!("{{{:06X}}}", col & 0xFF_FFFF)
            } else {
                String::new()
            };
            out.push(full);
            line_start = next;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::MonoMeasure;

    #[test]
    fn control_chars_become_spaces() {
        assert_eq!(replace_control_chars("a\u{1}b\tc\nd\u{7f}"), "a b c d\u{7f}");
    }

    #[test]
    fn player_chat_split_at_72_with_12_backtrack() {
        let t = "x".repeat(72);
        assert_eq!(split_line(&t, 72, 12), (t.clone(), None));
        let mut t: Vec<char> = "a".repeat(100).chars().collect();
        t[61] = ' ';
        let s: String = t.iter().collect();
        let (a, b) = split_line(&s, 72, 12);
        assert_eq!((a.len(), b.unwrap().len()), (61, 38));
        t[61] = 'a';
        t[59] = ' ';
        let s: String = t.iter().collect();
        let (a, b) = split_line(&s, 72, 12);
        assert_eq!((a.len(), b.unwrap().len()), (72, 28));
    }

    #[test]
    fn pixel_wrap_breaks_at_spaces_and_carries_colour() {
        let m = MonoMeasure {
            char_w: 10.0,
            line_h: 16.0,
        };
        let l = wrap_pixels(&m, FontId::Small, "aaaa bbbb cccc", 95.0);
        assert_eq!(l, vec!["aaaa bbbb", "cccc"]);
        let l = wrap_pixels(&m, FontId::Small, "{FF0000}aaaa bbbb", 50.0);
        assert_eq!(l, vec!["{FF0000}aaaa", "{FF0000}bbbb"]);
        // A word longer than the line is hard-split.
        let l = wrap_pixels(&m, FontId::Small, "abcdefgh", 40.0);
        assert_eq!(l, vec!["abcd", "efgh"]);
        // Explicit newlines.
        assert_eq!(wrap_pixels(&m, FontId::Small, "a\nb", 100.0), vec!["a", "b"]);
    }
}
