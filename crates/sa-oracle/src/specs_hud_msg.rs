//! L1 specs for PORT-HUD (`sa_hud`, see `docs/arch/hud-port.md`): the pure CMessages string helpers and
//! `CKeyArray::BinarySearch`. Input strings are generated around the markers "~a~" / "~1~" / "~k~",
//! embedded NULs and the u16 length boundaries; every read and write of the original stays inside the
//! scratch regions.

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_hud::{messages, text};

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0x4D5_5347;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// A GXT-ish string of `len` bytes (no NUL inside) built from marker pieces, letters, '~' and spaces.
fn gen_text(r: &mut Rng, len: usize) -> Vec<u8> {
    const PIECES: &[&[u8]] = &[
        b"~a~", b"~1~", b"~k~", b"~", b"a", b"1", b"k", b" ", b"~~", b"x", b"AB", b"~a", b"1~",
    ];
    let mut s = Vec::new();
    while s.len() < len {
        if r.chance(60) {
            s.extend_from_slice(PIECES[r.below(PIECES.len() as u32) as usize]);
        } else {
            let b = match r.below(4) {
                0 => b'a' + r.below(26) as u8,
                1 => b'0' + r.below(10) as u8,
                2 => 0x80 + r.below(0x7F) as u8,
                _ => 1 + r.below(0xFF) as u8,
            };
            s.push(b);
        }
    }
    s.truncate(len);
    s
}

/// Writes `bytes` (then filler) into region `ri`'s raw words.
fn put_region(v: &mut [u32], lay: &Layout, ri: usize, bytes: &[u8]) {
    let ws = &lay.word_slot[ri];
    for (wi, slot) in ws.iter().enumerate() {
        let Some(s) = slot else { continue };
        let mut w = [0u8; 4];
        for (k, b) in w.iter_mut().enumerate() {
            *b = bytes.get(4 * wi + k).copied().unwrap_or(0);
        }
        v[*s] = u32::from_le_bytes(w);
    }
}

/// A string of up to `max` bytes, its NUL, then (sometimes) garbage up to `cap` bytes.
fn string_bytes(r: &mut Rng, max: usize, cap: usize) -> Vec<u8> {
    let len = match r.below(10) {
        0 => 0,
        1 => max,
        2 => r.below(4) as usize,
        _ => r.below(max as u32 + 1) as usize,
    };
    let mut s = gen_text(r, len);
    s.push(0);
    if r.chance(50) {
        let extra = cap.saturating_sub(s.len());
        let n = r.below(extra as u32 + 1) as usize;
        let g = gen_text(r, n);
        s.extend_from_slice(&g);
    }
    s.truncate(cap);
    s
}

fn g_len16(r: &mut Rng) -> u32 {
    let hi = if r.chance(30) {
        r.next_u32() & 0xFFFF_0000
    } else {
        0
    };
    let lo = match r.below(10) {
        0 => 0,
        1 => 1,
        2 => 0xFFFF,
        _ => r.below(258),
    };
    hi | lo
}

fn g_cmp_len(r: &mut Rng) -> u32 {
    let hi = if r.chance(30) {
        r.next_u32() & 0xFFFF_0000
    } else {
        0
    };
    let lo = match r.below(10) {
        0 => 0,
        1 => 0xFFFF,
        _ => r.below(70),
    };
    hi | lo
}

/// Binary search bounds: low in [0, 63], high in [-1, 63] (all mids inside the 64-entry table), with
/// garbage upper 16 bits.
fn g_low(r: &mut Rng) -> u32 {
    let hi = if r.chance(30) {
        r.next_u32() & 0xFFFF_0000
    } else {
        0
    };
    hi | if r.chance(60) { 0 } else { r.below(64) }
}
fn g_high(r: &mut Rng) -> u32 {
    let hi = if r.chance(30) {
        r.next_u32() & 0xFFFF_0000
    } else {
        0
    };
    let v: i32 = if r.chance(60) { 63 } else { r.below(65) as i32 - 1 };
    hi | (v as u32 & 0xFFFF)
}

const TABLE: usize = 64;

pub fn all() -> Vec<Spec> {
    vec![
        b(
            "HudMsgGetStringLength",
            "CMessages::GetStringLength",
            0x69DB50,
            Cdecl,
            Eax,
        )
        .regions(vec![raw(80)])
        .args(vec![A::P(0)])
        .fix(|v, l| {
            let mut r = Rng::new(seed(v));
            let s = string_bytes(&mut r, 300, 320);
            put_region(v, l, 0, &s);
        })
        .rust(|c| ExpRet::Eax(messages::get_string_length(tp(c, 0)))),
        b("HudMsgStringCopy", "CMessages::StringCopy", 0x69DB70, Cdecl, Void)
            .regions(vec![raw(80), raw(80)])
            .args(vec![A::P(1), A::P(0), A::I(g_len16)])
            .fix(|v, l| {
                let mut r = Rng::new(seed(v));
                let s = string_bytes(&mut r, 300, 320);
                put_region(v, l, 0, &s);
                let d = gen_text(&mut r, 320);
                put_region(v, l, 1, &d);
                // keep the zero-fill / copy inside the 320-byte destination
                let k = l.arg_slot[2].unwrap();
                if v[k] & 0xFFFF > 320 {
                    v[k] = (v[k] & 0xFFFF_0000) | (v[k] % 321);
                }
            })
            .rust(|c| {
                let len = c.i(2);
                messages::string_copy(tp(c, 1), tp(c, 0), len);
                ExpRet::Void
            }),
        b(
            "HudMsgStringCopyNull",
            "CMessages::StringCopy",
            0x69DB70,
            Cdecl,
            Void,
        )
        .regions(vec![raw(80)])
        .args(vec![A::P(0), A::K(0), A::I(g_len16)])
        .fix(|v, l| {
            let mut r = Rng::new(seed(v));
            let d = gen_text(&mut r, 320);
            put_region(v, l, 0, &d);
            let k = l.arg_slot[2].unwrap();
            if v[k] & 0xFFFF > 320 {
                v[k] = (v[k] & 0xFFFF_0000) | (v[k] % 321);
            }
        })
        .rust(|c| {
            let len = c.i(2);
            messages::string_copy(tp(c, 0), GPtr::NULL, len);
            ExpRet::Void
        }),
        b(
            "HudMsgStringCompare",
            "CMessages::StringCompare",
            0x69DBD0,
            Cdecl,
            Al,
        )
        .regions(vec![raw(20), raw(20)])
        .args(vec![A::P(0), A::P(1), A::I(g_cmp_len)])
        .fix(|v, l| {
            let mut r = Rng::new(seed(v));
            let a = string_bytes(&mut r, 70, 80);
            let mut b = a.clone();
            match r.below(5) {
                0 => b = string_bytes(&mut r, 70, 80),
                1 if !b.is_empty() => {
                    let i = r.below(b.len() as u32) as usize;
                    b[i] = b[i].wrapping_add(1 + r.below(3) as u8);
                }
                2 if !b.is_empty() => {
                    let i = r.below(b.len() as u32) as usize;
                    b[i] = 0;
                }
                3 => {
                    let n = b.iter().position(|x| *x == 0).unwrap_or(b.len());
                    b.truncate(n);
                    let n = 1 + r.below(4) as usize;
                    let e = gen_text(&mut r, n);
                    b.extend_from_slice(&e);
                    b.push(0);
                }
                _ => {}
            }
            b.resize(80, 0);
            b[79] = 0;
            put_region(v, l, 0, &a);
            put_region(v, l, 1, &b);
        })
        .rust(|c| {
            let len = c.i(2);
            ExpRet::Al(messages::string_compare(tp(c, 0), tp(c, 1), len))
        }),
        b(
            "HudMsgInsertStringInString",
            "CMessages::InsertStringInString",
            0x69E040,
            Cdecl,
            Void,
        )
        .regions(vec![raw(104), raw(20)])
        .args(vec![A::P(0), A::P(1)])
        // Unicorn loses the flags of `cmp ax, 0x190` (0x69E11F) across the following `rep movsb`
        // (0x69E123) before `jae` (0x69E125): the zero fill was skipped whenever the byte copy ran
        // (measured: 0 mismatches with this patch, many without). The patch only swaps the two
        // instructions (`rep movsb; cmp ax, 0x190`), the same semantics on hardware (neither reads
        // what the other writes).
        .patch(vec![(0x69E11F, vec![0xF3, 0xA4, 0x66, 0x3D, 0x90, 0x01])])
        .fix(|v, l| {
            let mut r = Rng::new(seed(v));
            let t = string_bytes(&mut r, 250, 416);
            let p = string_bytes(&mut r, 60, 80);
            put_region(v, l, 0, &t);
            put_region(v, l, 1, &p);
        })
        .rust(|c| {
            messages::insert_string_in_string(tp(c, 0), tp(c, 1));
            ExpRet::Void
        }),
        b(
            "HudMsgBinarySearch",
            "CKeyArray::BinarySearch",
            0x69F570,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(2 * TABLE)])
        .args(vec![
            A::K(0),
            A::I(|r| r.next_u32()),
            A::P(0),
            A::I(g_low),
            A::I(g_high),
        ])
        .fix(|v, l| {
            let mut r = Rng::new(seed(v));
            let mut hs: Vec<u32> = (0..TABLE)
                .map(|_| if r.chance(20) { r.below(200) } else { r.next_u32() })
                .collect();
            if r.chance(90) {
                hs.sort_unstable();
            }
            let mut bytes = Vec::new();
            for h in &hs {
                bytes.extend_from_slice(&r.next_u32().to_le_bytes());
                bytes.extend_from_slice(&h.to_le_bytes());
            }
            put_region(v, l, 0, &bytes);
            let k = l.arg_slot[1].unwrap();
            if r.chance(60) {
                v[k] = hs[r.below(TABLE as u32) as usize];
            } else if r.chance(30) {
                v[k] =
                    hs[r.below(TABLE as u32) as usize].wrapping_add(if r.chance(50) { 1 } else { u32::MAX });
            }
        })
        .rust(|c| {
            let base = tp::<u8>(c, 0);
            let e = text::binary_search(GPtr::NULL, c.i(1), base, c.i(3) as i32, c.i(4) as i32);
            ExpRet::Eax(if e.is_null() {
                0
            } else {
                c.addr(0) + (e.addr() - base.addr()) as u32
            })
        }),
    ]
}
