//! `--fixed-step MS`: the deterministic mode (platform.md 19.1, decision 1).
//!
//! The Win32 time sources (`timeGetTime`, `GetTickCount`, `QueryPerformanceCounter`) read a virtual clock that starts
//! at 0 and advances by MS at the start of every main-loop iteration and by 10 us at every time read on the main thread (polling
//! loops end; the count of reads is itself deterministic), so `CTimer` sees the same frame times in
//! every run; the frame limiter does not sleep, and `Sleep(ms)` on the main thread also advances the clock by `ms`.
//! Every streaming read completes at its request: after `CdStreamRead` queues it (`ReleaseSemaphore` on the stream
//! semaphore) the main thread waits until the `CdStream` queue is empty and the streaming thread idle (and once more
//! at the end of every iteration). Input comes from the iteration-keyed scripts (`SA_GAME_KEYS`, `SA_GAME_WALK`,
//! `SA_GAME_MOUSE`). The DirectSound play cursors advance by the step in the main loop (not in real time),
//! and the game heap is a fixed-address region ([`crate::detheap`]). Not covered: the audio threads' own timing
//! (the smooth fade and the stream decoder run in real time).

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::ThreadId;

use sa_platform::clock::Clock;

/// Real time since the clock was made (`web_time`: `performance.now()` on wasm32).
pub struct StdClock(web_time::Instant);

impl Default for StdClock {
    fn default() -> Self {
        StdClock(web_time::Instant::now())
    }
}

impl StdClock {
    fn now_seconds(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

/// The step in microseconds (0: real time).
static STEP_US: AtomicU64 = AtomicU64::new(0);
/// The virtual clock in microseconds.
static NOW_US: AtomicU64 = AtomicU64::new(0);
/// Microseconds every main-thread time read adds to the virtual clock.
const READ_US: u64 = 10;
/// The main thread (the one that called [`set_fixed_step`]).
static MAIN: OnceLock<ThreadId> = OnceLock::new();
/// Main-loop iterations begun (`SA_GAME_DET_BT=N`: backtraces of the time reads in iteration N, counted from 1).
static ITER: AtomicU64 = AtomicU64::new(0);
/// The streaming thread is between its semaphore wait and its next one.
static STREAM_BUSY: AtomicBool = AtomicBool::new(false);

/// Turns the deterministic mode on with a step of `ms` milliseconds per main-loop iteration.
pub fn set_fixed_step(ms: f64) {
    STEP_US.store((ms * 1000.0).round().max(1.0) as u64, Ordering::SeqCst);
    let _ = MAIN.set(std::thread::current().id());
}

pub fn on() -> bool {
    STEP_US.load(Ordering::Relaxed) != 0
}

/// The fixed-address game heap outside the deterministic mode (window runs; platform.md section 21.1: one heap layout
/// everywhere). `--system-heap` turns it off.
static FIXED_HEAP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Serves the game heap from [`crate::detheap`] in this run.
pub fn set_fixed_heap(on: bool) {
    FIXED_HEAP.store(on, Ordering::SeqCst);
}

/// Whether the game heap comes from [`crate::detheap`]: the deterministic mode, or [`set_fixed_heap`].
pub fn fixed_heap() -> bool {
    on() || FIXED_HEAP.load(Ordering::Relaxed)
}

fn is_main() -> bool {
    MAIN.get().is_some_and(|m| *m == std::thread::current().id())
}

/// Called at the start of every main-loop iteration: the virtual clock advances by one step.
pub fn begin_iteration() {
    ITER.fetch_add(1, Ordering::SeqCst);
    let s = STEP_US.load(Ordering::Relaxed);
    if s != 0 {
        NOW_US.fetch_add(s, Ordering::SeqCst);
        // the DirectSound buffers' play cursors advance with the virtual clock, in the main loop
        crate::dsound::advance(s as f64 / 1_000_000.0);
    } else if cfg!(target_arch = "wasm32") {
        crate::dsound::advance_real_time();
    }
}

/// `Sleep(ms)` on the main thread advances the virtual clock by `ms` (loops that sleep until a time has passed end).
pub fn slept(ms: u32) {
    if on() && is_main() {
        NOW_US.fetch_add(u64::from(ms) * 1000, Ordering::SeqCst);
    }
}

/// `ReleaseSemaphore` on the streaming semaphore by the main thread (`CdStreamRead` queued a request): waits until
/// the thread has read it, so every read completes at its request.
pub fn released(h: u32) {
    if on() && is_main() && h == sa_stream::cdstream::STREAM_SEMAPHORE.get().addr() as u32 {
        drain();
    }
}

/// Called at the end of every main-loop iteration: waits until the streaming thread has drained its queue.
pub fn end_iteration() {
    if on() {
        drain();
        // the trace two runs of one scenario are compared by
        println!(
            "det: clock {} us, rand state {:#010x}, rand calls {}, srand calls {} (last seed {:#x})",
            NOW_US.load(Ordering::SeqCst),
            sa_core::crt::holdrand(),
            sa_core::crt::RANDS.load(Ordering::Relaxed),
            sa_core::crt::SEEDS.load(Ordering::Relaxed),
            sa_core::crt::LAST_SEED.load(Ordering::Relaxed)
        );
    }
}

fn drain() {
    let start = web_time::Instant::now();
    loop {
        let empty = sa_stream::cdstream::QUEUE_HEAD.get() == sa_stream::cdstream::QUEUE_TAIL.get();
        if empty && !STREAM_BUSY.load(Ordering::SeqCst) {
            break;
        }
        // a thread that never picks the queue up (not started yet) must not hang the loop
        if start.elapsed().as_secs() >= 10 {
            eprintln!("sa-game: --fixed-step: the streaming queue did not drain in 10 s");
            break;
        }
        std::thread::sleep(std::time::Duration::from_micros(50));
    }
}

/// `WaitForSingleObject(Ex)`: marks the streaming thread idle while it waits on its semaphore (the end-of-iteration
/// barrier waits for that).
pub fn wait(h: u32, ms: u32, wait: impl FnOnce(u32, u32) -> u32) -> u32 {
    if !on() || is_main() || h != sa_stream::cdstream::STREAM_SEMAPHORE.get().addr() as u32 {
        return wait(h, ms);
    }
    STREAM_BUSY.store(false, Ordering::SeqCst);
    let r = wait(h, ms);
    STREAM_BUSY.store(true, Ordering::SeqCst);
    r
}

/// The Win32 time sources' clock: the virtual clock in the deterministic mode, else real time.
#[derive(Default)]
pub struct GameClock(StdClock);

impl Clock for GameClock {
    fn now_seconds(&self) -> f64 {
        if on() {
            // every read on the main thread moves the clock by READ_US, so loops that poll the time end
            let t = if is_main() {
                NOW_US.fetch_add(READ_US, Ordering::SeqCst)
            } else {
                NOW_US.load(Ordering::SeqCst)
            };
            if crate::env::var("SA_GAME_DET_BT")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                == Some(ITER.load(Ordering::SeqCst))
            {
                println!(
                    "det: time read on {:?}:\n{}",
                    std::thread::current().name(),
                    std::backtrace::Backtrace::force_capture()
                );
            }
            t as f64 / 1_000_000.0
        } else {
            self.0.now_seconds()
        }
    }
}
