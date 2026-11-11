//! L1 specs for PORT-HUD-2 (`sa_hud::font_print`): the pure `GxtChar` string helpers. Strings are built
//! from GXT token pieces ('~', spaces, letters, high bytes) and always end inside their scratch region,
//! so every read and write of the original stays in the regions.

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_hud::font_print;

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0x6C7_8E2;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// `len` bytes without NUL, from token pieces and random bytes.
fn gen_text(r: &mut Rng, len: usize, balanced: bool) -> Vec<u8> {
    const PIECES: &[&[u8]] = &[b"~r~", b"~w~", b"~k~", b" ", b"  ", b"a", b"AB", b"~1~", b"x y", b"~n~"];
    let mut s = Vec::new();
    while s.len() < len {
        if r.chance(60) {
            s.extend_from_slice(PIECES[r.below(PIECES.len() as u32) as usize]);
        } else {
            let c = match r.below(4) {
                0 => b'a' + r.below(26) as u8,
                1 if !balanced => b'~',
                1 => b' ',
                2 => 0x80 + r.below(0x7F) as u8,
                _ => 1 + r.below(0xFF) as u8,
            };
            if balanced && c == b'~' {
                continue;
            }
            s.push(c);
        }
    }
    s.truncate(len);
    if balanced && s.iter().filter(|c| **c == b'~').count() % 2 == 1 {
        // drop the last '~' so every token is closed
        let k = s.iter().rposition(|c| *c == b'~').unwrap();
        s[k] = b'x';
    }
    s
}

/// Writes `bytes` (then zero filler) into region `ri`'s raw words.
fn put_region(v: &mut [u32], lay: &Layout, ri: usize, bytes: &[u8]) {
    for (wi, slot) in lay.word_slot[ri].iter().enumerate() {
        let Some(s) = slot else { continue };
        let mut w = [0u8; 4];
        for (k, b) in w.iter_mut().enumerate() {
            *b = bytes.get(4 * wi + k).copied().unwrap_or(0);
        }
        v[*s] = u32::from_le_bytes(w);
    }
}

/// A NUL-terminated string of at most `max` bytes, then garbage (with NULs) up to `cap`.
fn string(r: &mut Rng, max: usize, cap: usize, balanced: bool) -> Vec<u8> {
    let len = match r.below(8) {
        0 => 0,
        1 => max,
        _ => r.below(max as u32 + 1) as usize,
    };
    let mut s = gen_text(r, len, balanced);
    s.push(0);
    let n = cap.saturating_sub(s.len());
    let g = gen_text(r, n, false);
    s.extend_from_slice(&g);
    for k in (len + 1..cap).step_by(37) {
        s[k] = 0;
    }
    s.truncate(cap);
    s
}

pub fn all() -> Vec<Spec> {
    vec![
        b("HudGxtCharStrcat", "GxtCharStrcat", 0x718630, Cdecl, Eax)
            .regions(vec![raw(100), raw(50)])
            .args(vec![A::P(0), A::P(1)])
            .fix(|v, l| {
                let mut r = Rng::new(seed(v));
                put_region(v, l, 0, &string(&mut r, 190, 400, false));
                put_region(v, l, 1, &string(&mut r, 190, 200, false));
            })
            .rust(|c| {
                let (d, s) = (tp(c, 0), tp(c, 1));
                let (base, emu) = (d.addr(), c.addr(0));
                let r = font_print::gxt_char_strcat(d, s);
                ExpRet::Eax(emu + (r.addr() - base) as u32)
            }),
        b("HudGxtCharStrcpy", "GxtCharStrcpy", 0x718660, Cdecl, Eax)
            .regions(vec![raw(50), raw(50)])
            .args(vec![A::P(0), A::P(1)])
            .fix(|v, l| {
                let mut r = Rng::new(seed(v));
                put_region(v, l, 0, &string(&mut r, 190, 200, false));
                put_region(v, l, 1, &string(&mut r, 199, 200, false));
            })
            .rust(|c| {
                let (d, s) = (tp(c, 0), tp(c, 1));
                let (base, emu) = (d.addr(), c.addr(0));
                let r = font_print::gxt_char_strcpy(d, s);
                ExpRet::Eax(emu + (r.addr() - base) as u32)
            }),
        b("HudGxtCharStrlen", "FUN_00718690", 0x718690, Cdecl, Eax)
            .regions(vec![raw(64)])
            .args(vec![A::P(0)])
            .fix(|v, l| {
                let mut r = Rng::new(seed(v));
                put_region(v, l, 0, &string(&mut r, 255, 256, false));
            })
            .rust(|c| ExpRet::Eax(font_print::gxt_char_strlen(tp(c, 0)))),
        b("HudFontGetNextSpace", "CFont::GetNextSpace", 0x718ED0, Cdecl, Eax)
            .regions(vec![raw(64)])
            .args(vec![A::P(0)])
            .fix(|v, l| {
                let mut r = Rng::new(seed(v));
                put_region(v, l, 0, &string(&mut r, 255, 256, false));
            })
            .rust(|c| {
                let s = tp::<u8>(c, 0);
                let (base, emu) = (s.addr(), c.addr(0));
                let r = font_print::get_next_space(s);
                ExpRet::Eax(emu + (r.addr() - base) as u32)
            }),
        b("HudGxtStripTokens", "FUN_00719240", 0x719240, Cdecl, Void)
            .regions(vec![raw(68)])
            .args(vec![A::P(0)])
            .fix(|v, l| {
                let mut r = Rng::new(seed(v));
                put_region(v, l, 0, &string(&mut r, 250, 272, true));
            })
            .rust(|c| {
                font_print::strip_tokens(tp(c, 0));
                ExpRet::Void
            }),
    ]
}
