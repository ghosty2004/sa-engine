//! Monotonic clock.
//!
//! One implementation for every target: `web_time::Instant` is `std::time::Instant` natively and
//! `performance.now()` on wasm32 (in a window or a worker; 5 us resolution in a cross-origin-isolated page,
//! 100 us otherwise).

use std::sync::OnceLock;

/// A monotonic clock in seconds since an arbitrary origin.
pub trait Clock: Send + Sync {
    fn now_seconds(&self) -> f64;
}

/// Real time since the clock was made.
pub struct StdClock(web_time::Instant);

impl Default for StdClock {
    fn default() -> Self {
        StdClock(web_time::Instant::now())
    }
}

impl Clock for StdClock {
    fn now_seconds(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

/// The process-wide origin of [`now_ns`].
static ORIGIN: OnceLock<web_time::Instant> = OnceLock::new();

/// Nanoseconds since the first call in this process (monotonic).
#[inline]
pub fn now_ns() -> u64 {
    let o = ORIGIN.get_or_init(web_time::Instant::now);
    o.elapsed().as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotonic() {
        let c = StdClock::default();
        let (a, b) = (c.now_seconds(), c.now_seconds());
        assert!(b >= a);
        let (x, y) = (now_ns(), now_ns());
        assert!(y >= x);
    }
}
