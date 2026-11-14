//! The statically linked VC7.1 CRT functions the game logic calls (catalogue class `excluded:crt`:
//! never ported as hooks, but phases B/C need their behaviour).

use core::sync::atomic::{AtomicU32, Ordering};

crate::orig! {
    /// `rand()` (0x821B1E): the MSVC LCG on the calling thread's `_holdrand` (ptd+0x14).
    pub fn rand() -> i32 = cdecl 0x821B1E => rand_owned;
    /// `srand(seed)` (0x821B11).
    pub fn srand(seed: u32) = cdecl 0x821B11 => srand_owned;
}

/// The game thread's `_holdrand` in phases B/C. The CRT initialises it to 1 (`_getptd`'s per-thread
/// data initialisation); the game seeds it with `srand` before using it.
static HOLDRAND: AtomicU32 = AtomicU32::new(1);

/// One step of the MSVC LCG (constants at 0x821B28 and 0x821B2E): new state and the value returned.
pub const fn lcg_step(state: u32) -> (u32, i32) {
    let s = state.wrapping_mul(214_013).wrapping_add(2_531_011);
    (s, ((s >> 16) & 0x7FFF) as i32)
}

/// Phases B/C `rand()`.
pub fn rand_owned() -> i32 {
    RANDS.fetch_add(1, Ordering::Relaxed);
    let (s, v) = lcg_step(HOLDRAND.load(Ordering::Relaxed));
    HOLDRAND.store(s, Ordering::Relaxed);
    v
}

/// `srand` calls so far and the last seed (sa-game's deterministic-mode trace).
pub static SEEDS: AtomicU32 = AtomicU32::new(0);
/// `rand` calls so far (the same trace).
pub static RANDS: AtomicU32 = AtomicU32::new(0);
pub static LAST_SEED: AtomicU32 = AtomicU32::new(0);

/// The owned `_holdrand` (sa-game's deterministic-mode trace).
pub fn holdrand() -> u32 {
    HOLDRAND.load(Ordering::Relaxed)
}

/// Phases B/C `srand(seed)`.
pub fn srand_owned(seed: u32) {
    LAST_SEED.store(seed, Ordering::Relaxed);
    SEEDS.fetch_add(1, Ordering::Relaxed);
    HOLDRAND.store(seed, Ordering::Relaxed);
}

/// Serialises host tests that use the owned `_holdrand`.
#[cfg(all(test, not(sa_phase_a)))]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn msvc_sequence() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // MSVC rand() after srand(1): 41, 18467, 6334, 26500, 19169
        srand(1);
        let v: [i32; 5] = core::array::from_fn(|_| rand());
        assert_eq!(v, [41, 18467, 6334, 26500, 19169]);
    }
}

/// `rand()` as the original call whose return address is `site` calls it (determinism mode hashes
/// `(caller, value)`; ADR-001 caller attribution). Shared by every port crate.
#[inline]
pub fn rand_at(site: u32) -> i32 {
    crate::orig::at_site(site, rand)
}

/// Phases B/C: CRT `pow(double, double)` (0x8220F0) as the ports use it, only `pow(2.0, integer)`, which is
/// exact (overflow → +inf, underflow → +0).
pub fn pow2_int(a: f64, b: f64) -> f64 {
    assert!(a == 2.0 && b == (b as i64) as f64, "crt::pow2_int: only pow(2, integer)");
    let e = b as i64;
    if e > 1023 {
        f64::INFINITY
    } else if e >= -1022 {
        f64::from_bits(((e + 1023) as u64) << 52)
    } else if e >= -1074 {
        f64::from_bits(1u64 << (e + 1074))
    } else {
        0.0
    }
}
