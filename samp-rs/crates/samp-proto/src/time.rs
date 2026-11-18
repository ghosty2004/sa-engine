//! The clock the protocol and the client run on.
//!
//! Natively it is `std::time`. On wasm32, where `std::time` has no clock, it is the browser's (`web-time`, feature
//! `web`) or, in a module that imports nothing from JavaScript (a game mod), the clock its host installs once with
//! [`set_clock`].

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub use web_time::{Instant, SystemTime, UNIX_EPOCH};

#[cfg(all(target_arch = "wasm32", not(feature = "web")))]
pub use hosted::{Instant, SystemTime, UNIX_EPOCH, set_clock};

/// Builds with a clock of their own ignore the host's.
#[cfg(not(all(target_arch = "wasm32", not(feature = "web"))))]
pub fn set_clock(_clock: fn() -> HostTime) {}

/// What a host clock reads: the monotonic time since some fixed start, and the wall clock since 1970-01-01 UTC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostTime {
    /// Monotonic, from any fixed start.
    pub monotonic: std::time::Duration,
    /// Since the Unix epoch.
    pub wall: std::time::Duration,
}

#[cfg(any(test, all(target_arch = "wasm32", not(feature = "web"))))]
mod hosted {
    use std::ops::{Add, AddAssign, Sub, SubAssign};
    use std::sync::OnceLock;
    use std::time::Duration;

    use super::HostTime;

    static CLOCK: OnceLock<fn() -> HostTime> = OnceLock::new();

    /// Installs the clock (once; later calls are ignored). Until then the time stands at zero.
    pub fn set_clock(clock: fn() -> HostTime) {
        let _ = CLOCK.set(clock);
    }

    fn read() -> HostTime {
        CLOCK.get().map(|c| c()).unwrap_or_default()
    }

    /// A point of the host's monotonic clock (the subset of `std::time::Instant` the client uses).
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Instant(Duration);

    impl Instant {
        /// Now.
        pub fn now() -> Instant {
            Instant(read().monotonic)
        }

        /// The time from `earlier` to `self` (zero when `earlier` is later).
        pub fn duration_since(&self, earlier: Instant) -> Duration {
            self.0.saturating_sub(earlier.0)
        }

        /// As [`Instant::duration_since`].
        pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
            self.duration_since(earlier)
        }

        /// The time from `earlier` to `self`, `None` when `earlier` is later.
        pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
            self.0.checked_sub(earlier.0)
        }

        /// The time since `self`.
        pub fn elapsed(&self) -> Duration {
            Instant::now().duration_since(*self)
        }

        /// `self + d`.
        pub fn checked_add(&self, d: Duration) -> Option<Instant> {
            self.0.checked_add(d).map(Instant)
        }

        /// `self - d`, `None` before the clock's start.
        pub fn checked_sub(&self, d: Duration) -> Option<Instant> {
            self.0.checked_sub(d).map(Instant)
        }
    }

    impl Add<Duration> for Instant {
        type Output = Instant;
        fn add(self, d: Duration) -> Instant {
            Instant(self.0 + d)
        }
    }

    impl AddAssign<Duration> for Instant {
        fn add_assign(&mut self, d: Duration) {
            self.0 += d;
        }
    }

    impl Sub<Duration> for Instant {
        type Output = Instant;
        fn sub(self, d: Duration) -> Instant {
            Instant(self.0.saturating_sub(d))
        }
    }

    impl SubAssign<Duration> for Instant {
        fn sub_assign(&mut self, d: Duration) {
            self.0 = self.0.saturating_sub(d);
        }
    }

    impl Sub<Instant> for Instant {
        type Output = Duration;
        fn sub(self, earlier: Instant) -> Duration {
            self.duration_since(earlier)
        }
    }

    /// A point of the host's wall clock.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SystemTime(Duration);

    /// 1970-01-01 UTC.
    pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::ZERO);

    /// `SystemTime::duration_since` with `earlier` later than the time.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct SystemTimeError;

    impl SystemTime {
        /// Now.
        pub fn now() -> SystemTime {
            SystemTime(read().wall)
        }

        /// The time from `earlier` to `self`.
        pub fn duration_since(&self, earlier: SystemTime) -> Result<Duration, SystemTimeError> {
            self.0.checked_sub(earlier.0).ok_or(SystemTimeError)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn host_clock() {
            assert_eq!(Instant::now(), Instant(Duration::ZERO));
            set_clock(|| HostTime { monotonic: Duration::from_millis(1500), wall: Duration::from_secs(1_700_000_000) });
            let now = Instant::now();
            let earlier = now - Duration::from_millis(500);
            assert_eq!(now - earlier, Duration::from_millis(500));
            assert_eq!(earlier.duration_since(now), Duration::ZERO);
            assert_eq!(earlier.checked_duration_since(now), None);
            assert_eq!(earlier.saturating_duration_since(now), Duration::ZERO);
            assert_eq!(earlier.checked_add(Duration::from_millis(500)), Some(now));
            assert_eq!(now.checked_sub(Duration::from_secs(2)), None);
            assert_eq!(now.elapsed(), Duration::ZERO);
            assert_eq!(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(), 1_700_000_000);
        }
    }
}
