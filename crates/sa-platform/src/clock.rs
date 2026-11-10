//! Monotonic clock.

/// A monotonic clock in seconds since an arbitrary origin.
pub trait Clock: Send + Sync {
    fn now_seconds(&self) -> f64;
}

/// `std::time::Instant` clock (native targets).
#[cfg(not(target_arch = "wasm32"))]
pub struct StdClock(std::time::Instant);

#[cfg(not(target_arch = "wasm32"))]
impl Default for StdClock {
    fn default() -> Self {
        StdClock(std::time::Instant::now())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Clock for StdClock {
    fn now_seconds(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}
