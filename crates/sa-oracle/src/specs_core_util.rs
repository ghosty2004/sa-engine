//! L1 specs for the CORE utility ports in `sa-core` (PORT-CORE): `CKeyGen` string keys, the string
//! helpers of `common.cpp`, and the occlusion line tests. See `docs/arch/core-port.md`.
//!
//! String functions get their bytes in scratch memory (inputs = the argument words followed by the
//! whole scratch image as words); the Rust side runs the actual `sa_core` port on a copy of that image
//! (through `GPtr`s into the copy) and must leave it byte-identical to what the original leaves in the
//! emulator. `CKeyGen` reads its CRC table from `.data` (0x8CD068, never written by the game): the
//! spec allows that read (`reads`), the port's table is checked against the image by sa-core.
//! `DoesInfiniteLineTouchScreen` reads `RsGlobal.maximumWidth/Height`: each spec patches them to one
//! screen size and allows the read; two sizes are covered.

use crate::call::*;
use crate::generate::{self, Rng};
use crate::specs::*;
use sa_core::common;
use sa_core::keygen;
use sa_core::mem::GPtr;
use std::sync::Arc;

const WORDS: usize = SCRATCH_LEN / 4;

fn f(x: u32) -> f32 {
    f32::from_bits(x)
}

fn bits(v: Vec<f32>) -> Inputs {
    v.into_iter().map(f32::to_bits).collect()
}

/// Inputs = `args` followed by the scratch image.
fn pack(args: &[u32], s: &[u8; SCRATCH_LEN]) -> Inputs {
    let mut v = args.to_vec();
    v.extend(s.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])));
    v
}

fn image(i: &Inputs, nargs: usize) -> [u8; SCRATCH_LEN] {
    let mut s = [0u8; SCRATCH_LEN];
    for k in 0..WORDS {
        s[4 * k..4 * k + 4].copy_from_slice(&i[nargs + k].to_le_bytes());
    }
    s
}

/// A string byte: mostly letters/digits/punctuation, some high bytes, never NUL.
fn str_byte(r: &mut Rng) -> u8 {
    match r.below(10) {
        0..=3 => b'a' + r.below(26) as u8,
        4..=5 => b'A' + r.below(26) as u8,
        6 => r.pick(b"_.\\/ 09@[`{zaZ"),
        _ => 1 + r.below(255) as u8,
    }
}

fn str_len(r: &mut Rng, max: usize) -> usize {
    let l = match r.below(10) {
        0 => 0,
        1..=6 => r.below(40) as usize,
        7..=8 => r.below(300) as usize,
        _ => r.below(max as u32) as usize,
    };
    l.min(max)
}

/// Scratch filled with random bytes, a string of length `len` (plus NUL) at `off`.
fn scratch_with_string(r: &mut Rng, off: usize, len: usize) -> [u8; SCRATCH_LEN] {
    let mut s = [0u8; SCRATCH_LEN];
    for b in s.iter_mut() {
        *b = if r.chance(50) { FILL } else { r.next_u32() as u8 };
    }
    for k in 0..len {
        s[off + k] = str_byte(r);
    }
    s[off + len] = 0;
    s
}

fn ptr(s: &mut [u8; SCRATCH_LEN], off: u32) -> GPtr<u8> {
    GPtr::from_raw(s.as_mut_ptr().wrapping_add(off as usize))
}

fn base_spec(key: &'static str, name: &'static str, addr: u32, ret: RetKind) -> Spec {
    Spec {
        key,
        name,
        addr,
        ret,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        budget: 40_000,
        floats: 0,
        generate: Arc::new(|_| vec![]),
        edges: Arc::new(Vec::new),
        call: Arc::new(|_| Call::new(0, Conv::Cdecl, &[])),
        rust: Arc::new(|_, s| Expect::Returns { ret: ExpRet::Void, scratch: *s }),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
    }
}

// ------------------------------------------------------------------------------- CKeyGen ---------

const KEY_TABLE: (u32, u32) = (0x8CD068, 0x8CD068 + 0x400);

fn key_table() -> [u32; 256] {
    keygen::crc_table()
}

/// [key, off, size] + image; the string at `off`.
fn gen_key(r: &mut Rng) -> Inputs {
    let off = if r.chance(50) { 0 } else { r.below(0x40) as usize };
    let len = str_len(r, SCRATCH_LEN - 1 - off);
    let s = scratch_with_string(r, off, len);
    let key = if r.chance(20) { r.pick(&[0, 0xFFFF_FFFF, 0x8000_0000]) } else { r.next_u32() };
    let size = match r.below(6) {
        0 => r.pick(&[0u32, 0xFFFF_FFFF, 0x8000_0000, 1]),
        1 => r.below((SCRATCH_LEN - off) as u32 + 1),
        _ => len as u32,
    };
    pack(&[key, off as u32, size], &s)
}

fn key_edges() -> Vec<Inputs> {
    let mut e = Vec::new();
    for len in [0usize, 1, 2, 255, SCRATCH_LEN - 1] {
        for fillb in [0x01u8, 0x7F, 0x80, 0xFF, b'a'] {
            let mut s = [FILL; SCRATCH_LEN];
            s[..len].fill(fillb);
            s[len] = 0;
            for size in [0u32, len as u32, 0xFFFF_FFFF] {
                e.push(pack(&[0xFFFF_FFFF, 0, size], &s));
            }
        }
    }
    e
}

pub fn get_key() -> Spec {
    Spec {
        generate: Arc::new(gen_key),
        edges: Arc::new(key_edges),
        call: Arc::new(|i| {
            let mut c = Call::new(0x53CF00, Conv::Cdecl, &[Arg::Ptr(SCRATCH + i[1])]);
            c.scratch = image(i, 3);
            c
        }),
        rust: Arc::new(|i, s| {
            let mut m = *s;
            let k = keygen::get_key_with(&key_table(), ptr(&mut m, i[1]));
            Expect::Returns { ret: ExpRet::Eax(k), scratch: m }
        }),
        reads: vec![KEY_TABLE],
        ..base_spec("CKeyGenGetKey", "CKeyGen::GetKey-null-terminated", 0x53CF00, RetKind::Eax)
    }
}

pub fn get_key_sized() -> Spec {
    Spec {
        generate: Arc::new(gen_key),
        edges: Arc::new(key_edges),
        call: Arc::new(|i| {
            let mut c = Call::new(0x53CED0, Conv::Cdecl, &[Arg::Ptr(SCRATCH + i[1]), Arg::I32(i[2])]);
            c.scratch = image(i, 3);
            c
        }),
        rust: Arc::new(|i, s| {
            let mut m = *s;
            let k = keygen::get_key_sized_with(&key_table(), ptr(&mut m, i[1]), i[2] as i32);
            Expect::Returns { ret: ExpRet::Eax(k), scratch: m }
        }),
        reads: vec![KEY_TABLE],
        ..base_spec("CKeyGenGetKeySized", "CKeyGen::GetKey-sized", 0x53CED0, RetKind::Eax)
    }
}

pub fn append_string_to_key() -> Spec {
    Spec {
        generate: Arc::new(gen_key),
        edges: Arc::new(key_edges),
        call: Arc::new(|i| {
            let mut c = Call::new(0x53CF70, Conv::Cdecl, &[Arg::I32(i[0]), Arg::Ptr(SCRATCH + i[1])]);
            c.scratch = image(i, 3);
            c
        }),
        rust: Arc::new(|i, s| {
            let mut m = *s;
            let k = keygen::append_string_to_key_with(&key_table(), i[0], ptr(&mut m, i[1]));
            Expect::Returns { ret: ExpRet::Eax(k), scratch: m }
        }),
        reads: vec![KEY_TABLE],
        ..base_spec("CKeyGenAppendStringToKey", "CKeyGen::AppendStringToKey", 0x53CF70, RetKind::Eax)
    }
}

// ------------------------------------------------------------------- string copies/case ----------

/// [src, dst] + image: a string at `src`; `dst` either a separate area or `<= src` (a forward copy
/// with `dst` inside `(src, src + len]` never ends in the original either).
fn gen_copy(r: &mut Rng) -> Inputs {
    let half = SCRATCH_LEN / 2;
    let (src, dst, len) = match r.below(4) {
        0 => {
            let src = r.below(0x40);
            let len = str_len(r, SCRATCH_LEN - 1 - src as usize);
            (src, src.saturating_sub(r.below(4)), len)
        }
        1 => {
            let src = half as u32 + r.below(0x20);
            let len = str_len(r, SCRATCH_LEN - 1 - src as usize);
            (src, r.below(0x20), len)
        }
        _ => {
            let src = r.below(0x20);
            let len = str_len(r, half - 0x40);
            (src, half as u32 + r.below(0x40), len)
        }
    };
    let s = scratch_with_string(r, src as usize, len);
    pack(&[src, dst], &s)
}

fn copy_edges() -> Vec<Inputs> {
    let mut e = Vec::new();
    for len in [0usize, 1, 2, 0x200] {
        for b in [b'a', b'z', b'`', b'{', b'A', 0x80, 0xE1, 0xFF] {
            let mut s = [FILL; SCRATCH_LEN];
            s[..len].fill(b);
            s[len] = 0;
            e.push(pack(&[0, 0x280], &s));
            e.push(pack(&[0, 0], &s));
        }
    }
    e
}

pub fn ascii_to_gxt_char() -> Spec {
    Spec {
        generate: Arc::new(gen_copy),
        edges: Arc::new(copy_edges),
        call: Arc::new(|i| {
            let mut c = Call::new(0x718600, Conv::Cdecl, &[Arg::Ptr(SCRATCH + i[0]), Arg::Ptr(SCRATCH + i[1])]);
            c.scratch = image(i, 2);
            c
        }),
        rust: Arc::new(|i, s| {
            let mut m = *s;
            common::ascii_to_gxt_char(ptr(&mut m, i[0]), ptr(&mut m, i[1]));
            Expect::Returns { ret: ExpRet::Void, scratch: m }
        }),
        ..base_spec("AsciiToGxtChar", "Common::AsciiToGxtChar", 0x718600, RetKind::Void)
    }
}

pub fn make_upper_case() -> Spec {
    Spec {
        generate: Arc::new(gen_copy),
        edges: Arc::new(copy_edges),
        call: Arc::new(|i| {
            let mut c = Call::new(0x7186E0, Conv::Cdecl, &[Arg::Ptr(SCRATCH + i[1]), Arg::Ptr(SCRATCH + i[0])]);
            c.scratch = image(i, 2);
            c
        }),
        rust: Arc::new(|i, s| {
            let mut m = *s;
            let base = m.as_ptr() as usize;
            let r = common::make_upper_case(ptr(&mut m, i[1]), ptr(&mut m, i[0]));
            let eax = SCRATCH + (r.addr() - base) as u32;
            Expect::Returns { ret: ExpRet::Eax(eax), scratch: m }
        }),
        ..base_spec("MakeUpperCase", "Common::MakeUpperCase-", 0x7186E0, RetKind::Eax)
    }
}

pub fn make_upper_case_in() -> Spec {
    Spec {
        generate: Arc::new(gen_copy),
        edges: Arc::new(copy_edges),
        call: Arc::new(|i| {
            let mut c = Call::new(0x718710, Conv::Cdecl, &[Arg::Ptr(SCRATCH + i[0])]);
            c.scratch = image(i, 2);
            c
        }),
        rust: Arc::new(|i, s| {
            let mut m = *s;
            common::make_upper_case_in(ptr(&mut m, i[0]));
            Expect::Returns { ret: ExpRet::Void, scratch: m }
        }),
        ..base_spec("MakeUpperCaseIn", "Common::MakeUpperCase-in", 0x718710, RetKind::Void)
    }
}

// ------------------------------------------------------------------------- line tests ------------

fn cdecl_f32(addr: u32, i: &Inputs, n: usize) -> Call {
    let args: Vec<Arg> = i[..n].iter().map(|&x| Arg::F32(f(x))).collect();
    let mut c = Call::new(addr, Conv::Cdecl, &args);
    c.scratch = [FILL; SCRATCH_LEN];
    c
}

fn float_spec(key: &'static str, name: &'static str, addr: u32, n: usize, ret: RetKind) -> Spec {
    Spec {
        floats: n,
        budget: 2_000,
        generate: Arc::new(move |r| bits(generate::f32_tuple(r, n))),
        edges: Arc::new(move || generate::edge_tuples(n, 100_000).into_iter().map(bits).collect()),
        call: Arc::new(move |i| cdecl_f32(addr, i, n)),
        ..base_spec(key, name, addr, ret)
    }
}

pub fn is_point_inside_line() -> Spec {
    Spec {
        rust: Arc::new(|i, s| {
            let v = common::is_point_inside_line(f(i[0]), f(i[1]), f(i[2]), f(i[3]), f(i[4]), f(i[5]), f(i[6]));
            Expect::Returns { ret: ExpRet::Al(v as u8), scratch: *s }
        }),
        ..float_spec("IsPointInsideLine", "Common::IsPointInsideLine", 0x71E050, 7, RetKind::Al)
    }
}

pub fn does_infinite_line_cross_finite_line() -> Spec {
    Spec {
        rust: Arc::new(|i, s| {
            let v = common::does_infinite_line_cross_finite_line(
                f(i[0]),
                f(i[1]),
                f(i[2]),
                f(i[3]),
                f(i[4]),
                f(i[5]),
                f(i[6]),
                f(i[7]),
            );
            Expect::Returns { ret: ExpRet::Eax(v as u32), scratch: *s }
        }),
        ..float_spec("DoesInfiniteLineCrossFiniteLine", "FUN_0071db30", 0x71DB30, 8, RetKind::Eax)
    }
}

/// Screen sizes the `DoesInfiniteLineTouchScreen` specs patch into `RsGlobal`.
const SCREENS: [(&str, &str, i32, i32); 2] = [
    ("DoesInfiniteLineTouchScreen", "Common::DoesInfiniteLineTouchScreen", 640, 480),
    ("DoesInfiniteLineTouchScreen1366", "Common::DoesInfiniteLineTouchScreen", 1366, 768),
];

/// Coordinates around the screen, plus the generic float tuples.
fn gen_touch(r: &mut Rng, w: i32, h: i32) -> Inputs {
    if r.chance(50) {
        return bits(generate::f32_tuple(r, 4));
    }
    let span = (w.max(h) * 2) as f32;
    let pick = |r: &mut Rng, lim: f32| match r.below(5) {
        0 => r.pick(&[0.0, -0.0, lim, -lim]),
        1 => (r.below(lim as u32 + 1)) as f32,
        _ => r.uniform(span),
    };
    let v = vec![pick(r, w as f32), pick(r, h as f32), r.uniform(2.0), r.uniform(2.0)];
    bits(v)
}

pub fn does_infinite_line_touch_screen(k: usize) -> Spec {
    let (key, name, w, h) = SCREENS[k];
    let mut p = w.to_le_bytes().to_vec();
    p.extend_from_slice(&h.to_le_bytes());
    Spec {
        callees: &[0x71DB30],
        generate: Arc::new(move |r| gen_touch(r, w, h)),
        rust: Arc::new(move |i, s| {
            let v = common::infinite_line_touches_screen(w, h, f(i[0]), f(i[1]), f(i[2]), f(i[3]));
            Expect::Returns { ret: ExpRet::Al(v as u8), scratch: *s }
        }),
        patches: vec![(0xC17044, p)],
        reads: vec![(0xC17044, 0xC1704C)],
        ..float_spec(key, name, 0x71DB80, 4, RetKind::Al)
    }
}

/// Every CORE utility spec.
pub fn all() -> Vec<Spec> {
    vec![
        get_key(),
        get_key_sized(),
        append_string_to_key(),
        ascii_to_gxt_char(),
        make_upper_case(),
        make_upper_case_in(),
        is_point_inside_line(),
        does_infinite_line_cross_finite_line(),
        does_infinite_line_touch_screen(0),
        does_infinite_line_touch_screen(1),
    ]
    .into_iter()
    .chain(c2::all())
    .chain(c1::all())
    .collect()
}

/// PORT-CORE fork C2's specs (input, light), kept in their own file.
#[path = "specs_core_c2.rs"]
mod c2;

/// PORT-CORE fork C1's specs (platform, video), kept in their own file.
#[path = "specs_core_c1.rs"]
mod c1;

#[cfg(test)]
mod tests {
    use super::*;

    /// Generated cases keep every string (and its copy) inside scratch: the Rust side would read or
    /// write outside the image otherwise.
    #[test]
    fn generated_cases_stay_in_scratch() {
        for spec in all().into_iter().filter(|s| s.floats == 0) {
            for k in 0..5_000u64 {
                let i = (spec.generate)(&mut Rng::for_case(3, k));
                let c = (spec.call)(&i);
                let _ = (spec.rust)(&i, &c.scratch);
            }
            for i in (spec.edges)() {
                let c = (spec.call)(&i);
                let _ = (spec.rust)(&i, &c.scratch);
            }
        }
    }
}
