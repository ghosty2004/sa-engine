//! L2 oracle (INF-8), platform-independent part: configuration, sampling, per-function statistics, the
//! mutant switches and the callee/allocation logs. The in-process machinery (entry stubs, the generic
//! invoker, snapshot/restore, window diff, spool) is in `l2win` (Windows only); the case format, the
//! comparison and the normaliser are in `sa_oracle_format::{l2, normalise}`.
//!
//! Environment (read once at attach; `SA_L2` turns the feature on):
//!
//! | variable | meaning |
//! |---|---|
//! | `SA_L2` | hooks to compare: comma list of names or addresses, or `ports` (every hook with a signature) |
//! | `SA_L2_MAX` | cases per function per run (default 1000); then the hook goes back to the leave mode |
//! | `SA_L2_RATE` | sample every k-th call (default 1) |
//! | `SA_L2_PER_FRAME` | at most this many cases per function per frame (default 2), to spread cases over the run |
//! | `SA_L2_FROM` | first frame that may be sampled (default 0) |
//! | `SA_L2_LEAVE` | `orig` (default): the game continues with the original's results; `rust`: with Rust's |
//! | `SA_L2_OUT` | spool file (default `l2.spool` next to gta_sa.exe); a text report goes to OUT + `.txt` |
//! | `SA_L2_SPEC` | read/write-set spec (`sa-oracle l2 spec`); without it only `this`/pointer args are snapshotted |
//! | `SA_L2_EXTRA` | extra globals `ADDR:LEN[:R\|W\|RW],…` added to every enabled function's set |
//! | `SA_L2_DROP` | spec globals to drop (`ADDR,…`; self-test of the rwset check) |
//! | `SA_L2_WINDOW` | watched windows for undeclared writes: `data` (default: .data/.bss, _TEXT_HA, _rwdseg), `all` (every writable image section + the used part of the heap arena; slow), `none` |
//! | `SA_L2_WINDOWS` | extra watched ranges `START-END,…` |
//! | `SA_L2_STACK` | bytes of the caller's stack above the arguments to watch (default 1024) |
//! | `SA_L2_MUTANT` | hooks whose Rust implementation is made deliberately wrong (self-test) |
//! | `SA_HOOK_MODE` | start modes `NAME=orig\|rust\|l2,…` (independent of L2) |

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::table::{HOOKS, Mode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Window {
    #[default]
    Data,
    All,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Hook indices with L2 on.
    pub hooks: Vec<usize>,
    pub max: u32,
    pub rate: u32,
    pub per_frame: u32,
    pub from_frame: u32,
    pub leave_rust: bool,
    pub out: Option<String>,
    pub spec: Option<String>,
    /// (addr, len, access bits)
    pub extra: Vec<(u32, u32, u8)>,
    pub drop: Vec<u32>,
    pub window: Window,
    pub windows: Vec<(u32, u32)>,
    pub stack: u32,
    pub mutants: Vec<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            hooks: Vec::new(),
            max: 1000,
            rate: 1,
            per_frame: 2,
            from_frame: 0,
            leave_rust: false,
            out: None,
            spec: None,
            extra: Vec::new(),
            drop: Vec::new(),
            window: Window::Data,
            windows: Vec::new(),
            stack: 1024,
            mutants: Vec::new(),
        }
    }
}

fn hook_list(v: &str, what: &str) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for k in v.split(',').map(str::trim).filter(|k| !k.is_empty()) {
        if k == "ports" {
            out.extend((0..HOOKS.len()).filter(|&i| HOOKS[i].sig.is_some()));
            continue;
        }
        let i = crate::table::find(k).ok_or_else(|| format!("{what}: unknown hook {k:?}"))?;
        if HOOKS[i].sig.is_none() {
            return Err(format!("{what}: {} has no L2 signature", HOOKS[i].name));
        }
        out.push(i);
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

fn num(v: &str) -> Option<u32> {
    sa_oracle_format::l2::parse_u32(v.trim())
}

impl Config {
    /// `None` when `SA_L2` is unset or empty.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Option<Config>, String> {
        let Some(list) = get("SA_L2").filter(|v| !v.trim().is_empty()) else {
            return Ok(None);
        };
        let mut c = Config {
            hooks: hook_list(&list, "SA_L2")?,
            ..Config::default()
        };
        let n = |k: &str, d: u32| -> Result<u32, String> {
            match get(k).filter(|v| !v.is_empty()) {
                Some(v) => num(&v).ok_or_else(|| format!("{k}: bad number {v:?}")),
                None => Ok(d),
            }
        };
        c.max = n("SA_L2_MAX", c.max)?;
        c.rate = n("SA_L2_RATE", c.rate)?.max(1);
        c.per_frame = n("SA_L2_PER_FRAME", c.per_frame)?.max(1);
        c.from_frame = n("SA_L2_FROM", 0)?;
        c.stack = n("SA_L2_STACK", c.stack)?.min(0x10000);
        c.leave_rust = match get("SA_L2_LEAVE").as_deref() {
            None | Some("") | Some("orig") => false,
            Some("rust") => true,
            Some(v) => return Err(format!("SA_L2_LEAVE: {v:?} (orig|rust)")),
        };
        c.out = get("SA_L2_OUT").filter(|v| !v.is_empty());
        c.spec = get("SA_L2_SPEC").filter(|v| !v.is_empty());
        c.window = match get("SA_L2_WINDOW").as_deref() {
            None | Some("") | Some("data") => Window::Data,
            Some("all") => Window::All,
            Some("none") => Window::None,
            Some(v) => return Err(format!("SA_L2_WINDOW: {v:?} (data|all|none)")),
        };
        if let Some(v) = get("SA_L2_EXTRA") {
            for e in v.split(',').map(str::trim).filter(|e| !e.is_empty()) {
                let p: Vec<&str> = e.split(':').collect();
                let bad = || format!("SA_L2_EXTRA: {e:?} (ADDR:LEN[:R|W|RW])");
                let addr = p.first().and_then(|s| num(s)).ok_or_else(bad)?;
                let len = p.get(1).and_then(|s| num(s)).ok_or_else(bad)?;
                let acc = match p.get(2).copied() {
                    None | Some("RW") => 3,
                    Some("R") => 1,
                    Some("W") => 2,
                    _ => return Err(bad()),
                };
                c.extra.push((addr, len, acc));
            }
        }
        if let Some(v) = get("SA_L2_DROP") {
            for e in v.split(',').map(str::trim).filter(|e| !e.is_empty()) {
                c.drop.push(num(e).ok_or_else(|| format!("SA_L2_DROP: {e:?}"))?);
            }
        }
        if let Some(v) = get("SA_L2_WINDOWS") {
            for e in v.split(',').map(str::trim).filter(|e| !e.is_empty()) {
                let (a, b) = e.split_once('-').ok_or_else(|| format!("SA_L2_WINDOWS: {e:?}"))?;
                let (a, b) = (
                    num(a).ok_or_else(|| format!("SA_L2_WINDOWS: {e:?}"))?,
                    num(b).ok_or_else(|| format!("SA_L2_WINDOWS: {e:?}"))?,
                );
                if b <= a {
                    return Err(format!("SA_L2_WINDOWS: empty range {e:?}"));
                }
                c.windows.push((a, b));
            }
        }
        if let Some(v) = get("SA_L2_MUTANT") {
            c.mutants = hook_list(&v, "SA_L2_MUTANT")?;
        }
        Ok(Some(c))
    }
}

/// `SA_HOOK_MODE=NAME=orig|rust|l2,…`
pub fn parse_hook_modes(v: &str) -> Result<Vec<(usize, Mode)>, String> {
    let mut out = Vec::new();
    for e in v.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        let (k, m) = e
            .rsplit_once('=')
            .ok_or_else(|| format!("SA_HOOK_MODE: {e:?} (NAME=orig|rust|l2)"))?;
        let i = crate::table::find(k).ok_or_else(|| format!("SA_HOOK_MODE: unknown hook {k:?}"))?;
        let m = match m {
            "orig" => Mode::Original,
            "rust" => Mode::Rust,
            "l2" => Mode::L2,
            _ => return Err(format!("SA_HOOK_MODE: {e:?} (NAME=orig|rust|l2)")),
        };
        out.push((i, m));
    }
    Ok(out)
}

// ------------------------------------------------------------------ sampling / statistics ---------

#[derive(Debug, Clone, Default)]
pub struct FnStats {
    pub calls: u64,
    pub cases: u32,
    pub mismatches: u32,
    /// Mismatches that match a pending divergence-ledger entry (reported separately).
    pub ledger: u32,
    /// Cases with at least one undeclared write.
    pub undeclared_cases: u32,
    /// Undeclared write ranges: (addr, len) -> (cases, runs "orig"/"rust"/"both")
    pub undeclared: std::collections::BTreeMap<(u32, u32), (u32, String)>,
    pub unreadable_regions: u32,
    pub first_mismatch: Option<String>,
    pub frame_seen: u32,
    pub in_frame: u32,
    pub retired: bool,
    /// Time spent in cases (µs: snapshot, both runs, restores, compare, spool).
    pub us: u64,
}

impl FnStats {
    /// Sampling decision for the next call at `frame` (deterministic: counts only).
    pub fn sample(&mut self, cfg: &Config, frame: u32) -> bool {
        self.calls += 1;
        if self.cases >= cfg.max || frame < cfg.from_frame {
            return false;
        }
        if self.frame_seen != frame {
            self.frame_seen = frame;
            self.in_frame = 0;
        }
        if self.in_frame >= cfg.per_frame || !(self.calls - 1).is_multiple_of(cfg.rate as u64) {
            return false;
        }
        self.in_frame += 1;
        true
    }

    pub fn line(&self, name: &str) -> String {
        let und: Vec<String> = self
            .undeclared
            .iter()
            .take(8)
            .map(|((a, n), (k, r))| format!("{a:#x}+{n}x{k}:{r}"))
            .collect();
        format!(
            "{name} calls={} cases={} mismatches={} ledger={} undeclared_cases={} undeclared=[{}]{} \
             unreadable={} avg_us={} first_mismatch=\"{}\"",
            self.calls,
            self.cases,
            self.mismatches,
            self.ledger,
            self.undeclared_cases,
            und.join(","),
            if self.undeclared.len() > 8 { ",…" } else { "" },
            self.unreadable_regions,
            self.us / self.cases.max(1) as u64,
            self.first_mismatch.clone().unwrap_or_default()
        )
    }
}

// ------------------------------------------------------------------ mutants / logs ---------------

static MUTANTS: AtomicU32 = AtomicU32::new(0);

pub fn set_mutants(ids: &[usize]) {
    let m = ids.iter().fold(0u32, |m, &i| m | 1 << i);
    MUTANTS.store(m, Ordering::Release);
}

/// The Rust implementation of `id` must be deliberately wrong (L2 self-test).
pub fn mutant(id: crate::table::Id) -> bool {
    MUTANTS.load(Ordering::Relaxed) & (1 << id as u32) != 0
}

/// Set while one run of a case executes (callee and allocation logs on).
pub static RUN_ACTIVE: AtomicBool = AtomicBool::new(false);
static RUN_TID: AtomicU32 = AtomicU32::new(0);
static RUN_CALLS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static RUN_ALLOCS: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());

fn tid() -> u32 {
    #[cfg(windows)]
    {
        // SAFETY: plain Win32 call.
        unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() }
    }
    #[cfg(not(windows))]
    1
}

/// Starts the logs of one run on the current thread.
/// Hook id of the function under test: its own entry (the Rust implementation counting itself) is not
/// a callee and is skipped once per run.
static RUN_SELF: AtomicU32 = AtomicU32::new(u32::MAX);

pub fn run_begin(self_id: u32) {
    RUN_SELF.store(self_id, Ordering::Release);
    if let Ok(mut v) = RUN_CALLS.lock() {
        v.clear();
    }
    if let Ok(mut v) = RUN_ALLOCS.lock() {
        v.clear();
    }
    RUN_TID.store(tid(), Ordering::Release);
    RUN_ACTIVE.store(true, Ordering::Release);
}

/// Ends the run: (callees entered, arena allocations).
pub fn run_end() -> (Vec<u32>, Vec<(u32, u32)>) {
    RUN_ACTIVE.store(false, Ordering::Release);
    let c = RUN_CALLS
        .lock()
        .map(|mut v| std::mem::take(&mut *v))
        .unwrap_or_default();
    let a = RUN_ALLOCS
        .lock()
        .map(|mut v| std::mem::take(&mut *v))
        .unwrap_or_default();
    (c, a)
}

fn in_run() -> bool {
    RUN_ACTIVE.load(Ordering::Relaxed) && RUN_TID.load(Ordering::Relaxed) == tid()
}

/// Hook entry (from `table::count`): logged when a case run is active on this thread.
pub fn note_call(id: u32) {
    if in_run() && id == RUN_SELF.load(Ordering::Relaxed) {
        RUN_SELF.store(u32::MAX, Ordering::Relaxed);
        return;
    }
    if in_run()
        && let Ok(mut v) = RUN_CALLS.lock()
    {
        v.push(id);
    }
}

/// Arena allocation made on this thread (from the heap hooks).
pub fn note_alloc(p: u32, size: u32) {
    if in_run()
        && let Ok(mut v) = RUN_ALLOCS.lock()
    {
        v.push((p, size));
    }
}

/// True when a heap change happens inside a case run on this thread (the arena must be checkpointed).
pub fn heap_change_in_run() -> bool {
    in_run()
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(Config::from_env(env(&[])).unwrap(), None);
        let c = Config::from_env(env(&[
            ("SA_L2", "CGeneral::LimitRadianAngle,0x59c910"),
            ("SA_L2_MAX", "50"),
            ("SA_L2_LEAVE", "rust"),
            ("SA_L2_EXTRA", "0xB7CB4C:4,0x10:2:R"),
            ("SA_L2_DROP", "0xb7cb4c"),
            ("SA_L2_WINDOWS", "0x1000-0x2000"),
            ("SA_L2_MUTANT", "CVector::Normalise"),
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(c.hooks, vec![1, 18]);
        assert_eq!((c.max, c.leave_rust), (50, true));
        assert_eq!(c.extra, vec![(0xB7CB4C, 4, 3), (0x10, 2, 1)]);
        assert_eq!(
            (c.drop.clone(), c.windows.clone(), c.mutants.clone()),
            (vec![0xB7CB4C], vec![(0x1000, 0x2000)], vec![18])
        );
        let all = Config::from_env(env(&[("SA_L2", "ports")])).unwrap().unwrap();
        assert!(all.hooks.len() >= 8 && all.hooks.contains(&0));
        assert!(
            Config::from_env(env(&[("SA_L2", "_rand")])).is_err(),
            "no signature"
        );
        assert!(Config::from_env(env(&[("SA_L2", "nope")])).is_err());
        assert!(Config::from_env(env(&[("SA_L2", "ports"), ("SA_L2_WINDOW", "x")])).is_err());
        assert_eq!(
            parse_hook_modes("CGeneral::LimitRadianAngle=orig, 0x441390=l2").unwrap(),
            vec![(1, Mode::Original), (2, Mode::L2)]
        );
        assert!(parse_hook_modes("x=orig").is_err());
    }

    #[test]
    fn sampling_spreads_over_frames() {
        let cfg = Config {
            max: 5,
            per_frame: 2,
            rate: 1,
            from_frame: 10,
            ..Config::default()
        };
        let mut s = FnStats::default();
        let mut got = Vec::new();
        for frame in 9..14 {
            for _ in 0..4 {
                if s.sample(&cfg, frame) {
                    s.cases += 1;
                    got.push(frame);
                }
            }
        }
        assert_eq!(got, vec![10, 10, 11, 11, 12]);
        assert_eq!(s.calls, 20);
    }
}
