//! L1 specs for PORT-AUDIO (`sa_audio`): `CAEAudioUtility::AudioLog10` (FYL2X, CPU-decided).
//!
//! The raw ST0 of AudioLog10 is FYL2X's 64-bit result, which the `sa_math::x87` model matches in ~99.4%
//! of inputs (ledger). Every caller multiplies it by an f32 constant at PC=24 at once (20.0 at
//! 0x858BA4 or 10.0 at 0x85862C; 17 call sites), so the observable output is that product: the
//! `AudioLog10*c` spec runs the original AudioLog10 through a caller-shaped wrapper patched into its own
//! padding and compares the product's full ST0. `AudioLog10` itself compares the stored f32.

use crate::call::*;
use crate::generate::{self, Rng};
use crate::specs::{ExpRet, Expect, Inputs, RetKind, Route, Spec};
use sa_math::x87::X87;
use std::sync::Arc;

const FYL2X_REASON: &str =
    "FYL2X: Unicorn's result is not the CPU's (CPU-decided, sa_math::x87::fyl2x model)";
const ADDR: u32 = 0x4D9E50;
/// The 15 `nop` bytes after AudioLog10's `ret` (still inside its catalogued extent).
const WRAPPER: u32 = 0x4D9E71;
/// The callers' multipliers: 20.0 ([0x858BA4]) and 10.0 ([0x85862C]).
const MULTIPLIERS: [u32; 2] = [0x41A0_0000, 0x4120_0000];

/// `push dword [esp+4]; call AudioLog10; pop ecx; fmul dword [esp+8]; ret` (15 bytes).
fn wrapper() -> Vec<u8> {
    let rel = ADDR.wrapping_sub(WRAPPER + 4 + 5).to_le_bytes();
    let mut c = vec![0xFF, 0x74, 0x24, 0x04, 0xE8];
    c.extend_from_slice(&rel);
    c.extend_from_slice(&[0x59, 0xD8, 0x4C, 0x24, 0x08, 0xC3]);
    assert_eq!(c.len(), 15);
    c
}

/// Volumes and ratios the game passes (>= 1e-5), the whole f32 range, and values near 1.
fn gen_x(r: &mut Rng) -> u32 {
    match r.below(5) {
        0 => r.below(0x7F80_0000 - 0x3727_C5AC) + 0x3727_C5AC,
        1 => r.uniform(2.0).abs().to_bits(),
        2 => r.uniform(100.0).abs().to_bits(),
        3 => (1.0 + r.uniform(1.0 / 64.0)).to_bits(),
        _ => generate::f32_any(r).to_bits(),
    }
}

fn edges() -> Vec<Inputs> {
    let mut v: Vec<u32> = generate::edge_f32().into_iter().map(f32::to_bits).collect();
    v.extend([
        0x3727_C5AC,
        0x3727_C5AB,
        0x3727_C5AD,
        0x4120_0000,
        0x42C8_0000,
        0x3DCC_CCCD,
    ]);
    for i in 0..512u32 {
        v.push(0x3F80_0000 + i);
        v.push(0x3F80_0000 - i);
    }
    v.into_iter().map(|x| vec![x]).collect()
}

fn st0(v: X87, s: &[u8; SCRATCH_LEN]) -> Expect {
    Expect::Returns {
        ret: ExpRet::St0 {
            f32: v.to_f32().to_bits(),
            ext: v.to_ext(),
        },
        scratch: *s,
    }
}

fn base(key: &'static str, st0_exact: bool) -> Spec {
    Spec {
        preserve: 0,
        key,
        name: "CAEAudioUtility::AudioLog10",
        addr: ADDR,
        ret: RetKind::St0,
        st0_exact,
        route: Route::L2 { reason: FYL2X_REASON },
        callees: &[],
        budget: 1_000,
        floats: 1,
        generate: Arc::new(|r| vec![gen_x(r)]),
        edges: Arc::new(edges),
        call: Arc::new(|i| {
            let mut c = Call::new(ADDR, Conv::Cdecl, &[Arg::F32(f32::from_bits(i[0]))]);
            c.scratch = [crate::specs::FILL; SCRATCH_LEN];
            c
        }),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
        rust: Arc::new(|i, s| st0(sa_audio::utility::audio_log10(f32::from_bits(i[0])), s)),
    }
}

pub fn all() -> Vec<Spec> {
    let raw = base("AudioLog10", false);
    let mut prod = base("AudioLog10*c", true);
    // input 1 (raw) selects the multiplier
    prod.generate = Arc::new(|r| vec![gen_x(r), r.below(2)]);
    prod.edges = Arc::new(|| {
        edges()
            .into_iter()
            .flat_map(|e| [vec![e[0], 0], vec![e[0], 1]])
            .collect()
    });
    prod.patches = vec![(WRAPPER, wrapper())];
    prod.call = Arc::new(|i| {
        let c = f32::from_bits(MULTIPLIERS[i[1] as usize & 1]);
        let mut call = Call::new(
            WRAPPER,
            Conv::Cdecl,
            &[Arg::F32(f32::from_bits(i[0])), Arg::F32(c)],
        );
        call.scratch = [crate::specs::FILL; SCRATCH_LEN];
        call
    });
    prod.rust = Arc::new(|i, s| {
        let c = f32::from_bits(MULTIPLIERS[i[1] as usize & 1]);
        st0(sa_audio::utility::audio_log10(f32::from_bits(i[0])).fmul_m(c), s)
    });
    vec![raw, prod]
}

#[cfg(test)]
mod tests {
    #[test]
    fn constants_match_the_exe() {
        let Some(img) = crate::image_cached() else { return };
        assert_eq!(
            img.u32(0x858C14),
            Some(sa_audio::utility::AUDIO_LOG10_MIN.to_bits())
        );
        assert_eq!(
            img.u32(0x858FC8),
            Some(sa_audio::utility::AUDIO_LOG10_FLOOR.to_bits())
        );
        assert_eq!(img.u32(0x858BA4), Some(super::MULTIPLIERS[0]));
        assert_eq!(img.u32(0x85862C), Some(super::MULTIPLIERS[1]));
        // the wrapper goes into AudioLog10's nop padding
        assert_eq!(img.bytes(super::WRAPPER, 15), &[0x90; 15]);
    }
}
