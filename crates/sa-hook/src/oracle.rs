//! Oracle features (INF-4/5/6): determinism mode, input recording/replay, frame-state hashing.
//!
//! Platform-independent parts live here (configuration, the virtual clock, the per-tick input state
//! machine); the Windows side (`detwin`) patches the game and calls into them. Everything is off by
//! default: with no `SA_DET*`/`SA_HASH` variable set, the Oracle hooks are not even installed.
//!
//! Environment (read once at attach):
//!
//! | variable | meaning |
//! |---|---|
//! | `SA_DET=1` | determinism mode: virtual clock, synchronous CdStream reads (implies hashing) |
//! | `SA_HASH=1` | frame-state hashing without determinism mode |
//! | `SA_DET_STEP_US` | virtual frame step, default 40000 (= 25 fps, the default frame limiter) |
//! | `SA_DET_SEED` | virtual clock base in QPC ticks, default 1000000000000 |
//! | `SA_DET_TRACE` | hash trace path, default `sa-trace.bin` next to gta_sa.exe |
//! | `SA_DET_PROGRAM` | input program to inject from process start |
//! | `SA_DET_RECORD` | write a recording of every input tick from process start to this path |
//! | `SA_DET_REPLAY` | inject a recording from process start |
//! | `SA_DET_END` | last frame to trace/record (default: the program's `end`) |
//! | `SA_DET_PERTURB` | add 1 ulp to the player's position x at this frame boundary (before hashing) |
//! | `SA_DET_RANDLOG` | write every main-thread rand/srand call (frame, caller, value) to this path |
//! | `SA_DET_EXIT=1` | terminate the game after the end frame was written |

use sa_oracle_format::pad::PadInput;
use sa_oracle_format::program::Program;
use sa_oracle_format::recording::{Recording, Tick};

pub const DEFAULT_STEP_US: u32 = 40_000;
pub const DEFAULT_CLOCK_BASE: u64 = 1_000_000_000_000;
/// 2005-06-07 00:00:00 UTC as a FILETIME (100 ns since 1601): the virtual wall clock's epoch.
pub const FILETIME_EPOCH: u64 = 127_625_760_000_000_000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub det: bool,
    pub hash: bool,
    pub step_us: u32,
    pub clock_base: u64,
    pub trace: Option<String>,
    pub program: Option<String>,
    pub record: Option<String>,
    pub replay: Option<String>,
    pub end_frame: Option<u32>,
    pub perturb: Option<u32>,
    pub randlog: Option<String>,
    pub exit_at_end: bool,
}

impl Config {
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Config, String> {
        let flag = |k: &str| get(k).is_some_and(|v| v == "1");
        let path = |k: &str| get(k).filter(|v| !v.is_empty());
        let num = |k: &str| -> Result<Option<u64>, String> {
            get(k)
                .filter(|v| !v.is_empty())
                .map(|v| {
                    v.trim()
                        .parse::<u64>()
                        .map_err(|_| format!("{k}: bad number {v:?}"))
                })
                .transpose()
        };
        let det = flag("SA_DET");
        let mut c = Config {
            det,
            hash: det || flag("SA_HASH"),
            step_us: num("SA_DET_STEP_US")?.unwrap_or(DEFAULT_STEP_US as u64) as u32,
            clock_base: num("SA_DET_SEED")?.unwrap_or(DEFAULT_CLOCK_BASE),
            trace: path("SA_DET_TRACE"),
            program: path("SA_DET_PROGRAM"),
            record: path("SA_DET_RECORD"),
            replay: path("SA_DET_REPLAY"),
            end_frame: num("SA_DET_END")?.map(|v| v as u32),
            perturb: num("SA_DET_PERTURB")?.map(|v| v as u32),
            randlog: path("SA_DET_RANDLOG"),
            exit_at_end: flag("SA_DET_EXIT"),
        };
        if c.step_us == 0 || c.step_us > 1_000_000 {
            return Err(format!("SA_DET_STEP_US out of range: {}", c.step_us));
        }
        if c.program.is_some() && c.replay.is_some() {
            return Err("SA_DET_PROGRAM and SA_DET_REPLAY are exclusive".into());
        }
        if c.record.is_some() || c.replay.is_some() || c.program.is_some() || c.perturb.is_some() {
            c.hash = true;
        }
        Ok(c)
    }

    /// True when the Oracle hook group must be installed.
    pub fn enabled(&self) -> bool {
        self.det || self.hash || self.randlog.is_some()
    }
}

/// The virtual clock of determinism mode, in QueryPerformanceCounter ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    pub qpf: u64,
    pub step_ticks: u64,
    pub now: u64,
}

impl Clock {
    pub fn new(qpf: u64, step_us: u32, base: u64) -> Clock {
        let qpf = qpf.max(1);
        Clock {
            qpf,
            step_ticks: (step_us as u128 * qpf as u128 / 1_000_000) as u64,
            now: base,
        }
    }
    /// One frame boundary: the frame that starts now lasted exactly one step.
    pub fn advance(&mut self) {
        self.now += self.step_ticks;
    }
    /// `timeGetTime()` view: milliseconds.
    pub fn ms(&self) -> u32 {
        (self.now as u128 * 1000 / self.qpf as u128) as u32
    }
    /// `GetSystemTimeAsFileTime()` view: 100 ns units since 1601.
    pub fn filetime(&self) -> u64 {
        FILETIME_EPOCH + (self.now as u128 * 10_000_000 / self.qpf as u128) as u64
    }
}

/// What feeds the pad this tick.
#[derive(Debug, Clone)]
pub enum Source {
    /// Real input (nothing injected).
    Live,
    Program(Program),
    Replay {
        rec: Recording,
        pos: usize,
    },
}

/// The per-tick input state machine. `begin_tick` runs at `CPad::UpdateMouse` entry, `end_tick` at
/// `CPad::Update(0)` entry with the input captured at the record points (after any injection).
#[derive(Debug, Clone)]
pub struct InputState {
    pub source: Source,
    pub recording: Option<Recording>,
    /// Ticks completed since the source started (process start for env-configured runs).
    pub ticks: u32,
    pub replay_done: bool,
    pub desyncs: u32,
    pub first_desync: Option<String>,
    inject: Option<PadInput>,
    in_tick: bool,
}

impl InputState {
    pub fn new(source: Source, record: bool) -> InputState {
        InputState {
            source,
            recording: record.then(Recording::default),
            ticks: 0,
            replay_done: false,
            desyncs: 0,
            first_desync: None,
            inject: None,
            in_tick: false,
        }
    }

    pub fn mode(&self) -> &'static str {
        match (&self.source, self.recording.is_some()) {
            (Source::Live, false) => "live",
            (Source::Live, true) => "record-live",
            (Source::Program(_), false) => "program",
            (Source::Program(_), true) => "record-program",
            (Source::Replay { .. }, false) => "replay",
            (Source::Replay { .. }, true) => "replay-record",
        }
    }

    fn desync(&mut self, msg: String) {
        self.desyncs += 1;
        if self.first_desync.is_none() {
            self.first_desync = Some(msg);
        }
    }

    /// Start of a tick at frame boundary count `frame`: returns the input to inject, if any.
    pub fn begin_tick(&mut self, frame: u32) -> Option<&PadInput> {
        if self.in_tick {
            self.desync(format!(
                "tick {}: UpdateMouse twice without CPad::Update(0)",
                self.ticks
            ));
        }
        self.in_tick = true;
        let tick = self.ticks;
        self.inject = match &mut self.source {
            Source::Live => None,
            Source::Program(p) => Some(p.input(frame, tick)),
            Source::Replay { rec, pos } => match rec.ticks.get(*pos) {
                Some(t) => {
                    let (want, input) = (t.frame, t.input.clone());
                    if want != frame {
                        let m = format!("tick {tick}: recorded at frame {want}, replayed at frame {frame}");
                        self.desync(m);
                    }
                    Some(input)
                }
                None => {
                    self.replay_done = true;
                    None
                }
            },
        };
        self.inject.as_ref()
    }

    /// The input injected this tick (between `begin_tick` and `end_tick`).
    pub fn injected(&self) -> Option<&PadInput> {
        self.inject.as_ref()
    }

    pub fn end_tick(&mut self, frame: u32, captured: PadInput) {
        if !self.in_tick {
            // CPad::Update(0) without a preceding UpdateMouse: count it, do not record a half tick.
            self.desync(format!(
                "tick {}: CPad::Update(0) without UpdateMouse",
                self.ticks
            ));
            return;
        }
        self.in_tick = false;
        if let Some(i) = &self.inject
            && *i != captured
        {
            let m = format!(
                "tick {}: captured input differs from the injected input",
                self.ticks
            );
            self.desync(m);
        }
        if let Source::Replay { pos, .. } = &mut self.source
            && !self.replay_done
        {
            *pos += 1;
        }
        if let Some(r) = &mut self.recording {
            r.ticks.push(Tick {
                frame,
                input: captured,
            });
        }
        self.ticks += 1;
        self.inject = None;
    }
}

/// Running hash of the main thread's rand/srand calls (`RandCalls` subsystem): FNV-1a over
/// (kind, caller, value) little-endian.
pub fn rand_seq_step(state: u64, kind: u8, caller: u32, value: u32) -> u64 {
    let mut h = sa_oracle_format::fnv::Fnv64(state);
    h.u8(kind);
    h.u32(caller);
    h.u32(value);
    h.finish()
}

// ------------------------------------------------------------------ hook entry addresses -------

macro_rules! entry {
    ($name:ident, $imp:path) => {
        pub fn $name() -> usize {
            #[cfg(windows)]
            {
                $imp as *const () as usize
            }
            #[cfg(not(windows))]
            {
                0
            }
        }
    };
}

entry!(pad_update_entry, crate::detwin::pad_update_hook);
entry!(pad_update_mouse_entry, crate::detwin::pad_update_mouse_hook);
entry!(cd_stream_read_entry, crate::detwin::cd_stream_read_hook);
entry!(rand_entry, crate::detwin::rand_shim);
entry!(srand_entry, crate::detwin::srand_shim);
entry!(heap_alloc_entry, crate::detwin::heap_alloc_hook);
entry!(free_entry, crate::detwin::free_hook);
entry!(realloc_entry, crate::detwin::realloc_hook);
entry!(calloc_entry, crate::detwin::calloc_hook);
entry!(msize_entry, crate::detwin::msize_hook);
entry!(audio_hw_service_entry, crate::detaudio::hw_service_hook);

#[cfg(test)]
mod tests {
    use super::*;
    use sa_oracle_format::pad::ctrl;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let m: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| m.get(k).cloned()
    }

    #[test]
    fn config() {
        let c = Config::from_env(env(&[])).unwrap();
        assert!(!c.enabled());
        let c = Config::from_env(env(&[("SA_DET", "1"), ("SA_DET_END", "3000")])).unwrap();
        assert!(c.det && c.hash && c.enabled());
        assert_eq!((c.step_us, c.end_frame), (DEFAULT_STEP_US, Some(3000)));
        let c = Config::from_env(env(&[("SA_DET_RECORD", "x.sarec")])).unwrap();
        assert!(!c.det && c.hash);
        assert!(Config::from_env(env(&[("SA_DET_STEP_US", "0")])).is_err());
        assert!(Config::from_env(env(&[("SA_DET_END", "x")])).is_err());
        assert!(Config::from_env(env(&[("SA_DET_PROGRAM", "a"), ("SA_DET_REPLAY", "b")])).is_err());
    }

    #[test]
    fn clock() {
        let mut c = Clock::new(10_000_000, 40_000, DEFAULT_CLOCK_BASE);
        assert_eq!(c.step_ticks, 400_000);
        assert_eq!(c.ms(), 100_000_000);
        c.advance();
        assert_eq!(c.ms(), 100_000_040);
        assert_eq!(c.filetime(), FILETIME_EPOCH + 1_000_000_400_000);
    }

    fn input(ly: i16) -> PadInput {
        let mut p = PadInput::default();
        p.set_ctrl(ctrl::LEFT_STICK_Y, ly);
        p
    }

    #[test]
    fn program_record_then_replay() {
        let prog = Program::parse("2..3 ly=-128\nend 4").unwrap();
        let mut s = InputState::new(Source::Program(prog), true);
        // two ticks at frame 0 (intro), then one per frame
        for frame in [0, 0, 1, 2, 3, 4] {
            let inj = s.begin_tick(frame).cloned().unwrap();
            s.end_tick(frame, inj);
        }
        assert_eq!(s.desyncs, 0);
        let rec = s.recording.take().unwrap();
        assert_eq!(rec.ticks.len(), 6);
        assert_eq!(rec.ticks[3].input, input(-128));
        assert_eq!(rec.ticks[5].input, input(0));

        let mut r = InputState::new(
            Source::Replay {
                rec: rec.clone(),
                pos: 0,
            },
            false,
        );
        for (i, frame) in [0, 0, 1, 2, 3, 4].into_iter().enumerate() {
            let inj = r.begin_tick(frame).cloned().unwrap();
            assert_eq!(inj, rec.ticks[i].input);
            r.end_tick(frame, inj);
        }
        assert_eq!((r.desyncs, r.replay_done), (0, false));
        assert!(r.begin_tick(5).is_none());
        assert!(r.replay_done);
        r.end_tick(5, PadInput::default());

        // a replay that reaches a tick at another frame reports a desync
        let mut r = InputState::new(Source::Replay { rec, pos: 0 }, false);
        r.begin_tick(1);
        r.end_tick(1, input(0));
        assert_eq!(r.desyncs, 1);
        assert!(r.first_desync.as_deref().unwrap().contains("recorded at frame 0"));
    }

    #[test]
    fn live_capture_and_misuse() {
        let mut s = InputState::new(Source::Live, true);
        assert!(s.begin_tick(7).is_none());
        s.end_tick(7, input(5));
        assert_eq!(s.recording.as_ref().unwrap().ticks[0].input, input(5));
        s.end_tick(8, input(5));
        assert_eq!((s.desyncs, s.ticks), (1, 1));
        assert_eq!(s.mode(), "record-live");
        assert_ne!(rand_seq_step(1, 0, 2, 3), rand_seq_step(1, 0, 3, 2));
    }
}
