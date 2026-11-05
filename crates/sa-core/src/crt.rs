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
    let (s, v) = lcg_step(HOLDRAND.load(Ordering::Relaxed));
    HOLDRAND.store(s, Ordering::Relaxed);
    v
}

/// Phases B/C `srand(seed)`.
pub fn srand_owned(seed: u32) {
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
