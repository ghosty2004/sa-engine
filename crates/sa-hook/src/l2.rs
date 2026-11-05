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
//! | `SA_L2_EXTRA` | extra regions `[HOOK@]ADDR:LEN[:R\|W\|RW],…` added to every enabled function's set, or only to HOOK's (name or address); ADDR and LEN are numbers or expressions evaluated at each case ([`Expr`]), e.g. `CPed::operator delete-anon@[[0xB74490]+4]:[[0xB74490]+8]:RW` |
//! | `SA_L2_DROP` | spec globals to drop (`ADDR,…`; self-test of the rwset check) |
//! | `SA_L2_WINDOW` | watched windows for undeclared writes: `data` (default: .data/.bss, _TEXT_HA, _rwdseg), `all` (every writable image section + the used part of the heap arena; slow), `none` |
//! | `SA_L2_WINDOWS` | extra watched ranges `START-END,…` |
//! | `SA_L2_STACK` | bytes of the caller's stack above the arguments to watch (default 1024) |
//! | `SA_L2_MUTANT` | hooks whose Rust implementation is made deliberately wrong (self-test); also honoured without `SA_L2` (the L3 bisection self-test runs the mutant in plain Rust mode) |
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
    pub extra: Vec<Extra>,
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
                c.extra.push(Extra::parse(e)?);
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
        c.mutants = mutants_from_env(&get)?;
        Ok(Some(c))
    }
}

/// An address or length of an extra region, evaluated when a case starts (before the original runs):
/// hex/decimal numbers, the registers at entry `eax ecx edx ebx esi edi ebp`, stack arguments `arg0`,
/// `arg1`, …, `[EXPR]` = the dword at EXPR, combined with `+` and `-` (wrapping). Pool storage, for
/// example, lives on the heap behind a global pointer: the ped pool's byte map is `[[0xB74490]+4]`,
/// its length `[[0xB74490]+8]`; for a thiscall pool method `[ecx+4]` / `[ecx+8]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Num(u32),
    /// Index in `regs_in` order: eax, ecx, edx, ebx, esi, edi, ebp.
    Reg(u8),
    Arg(u8),
    Deref(Box<Expr>),
    /// Terms with their sign (`true` = subtract).
    Sum(Vec<(bool, Expr)>),
}

const REGS: [&str; 7] = ["eax", "ecx", "edx", "ebx", "esi", "edi", "ebp"];

impl Expr {
    pub fn parse(s: &str) -> Result<Expr, String> {
        let b: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
        let mut pos = 0;
        let e = Self::sum(&b, &mut pos)?;
        if pos != b.len() {
            return Err(format!("expression {s:?}: unexpected {:?} at {pos}", b[pos]));
        }
        Ok(e)
    }

    fn sum(b: &[char], pos: &mut usize) -> Result<Expr, String> {
        let mut terms = vec![(false, Self::term(b, pos)?)];
        while *pos < b.len() && (b[*pos] == '+' || b[*pos] == '-') {
            let neg = b[*pos] == '-';
            *pos += 1;
            terms.push((neg, Self::term(b, pos)?));
        }
        Ok(if terms.len() == 1 {
            terms.pop().map(|t| t.1).unwrap_or(Expr::Num(0))
        } else {
            Expr::Sum(terms)
        })
    }

    fn term(b: &[char], pos: &mut usize) -> Result<Expr, String> {
        if b.get(*pos) == Some(&'[') {
            *pos += 1;
            let e = Self::sum(b, pos)?;
            if b.get(*pos) != Some(&']') {
                return Err("expression: missing ']'".into());
            }
            *pos += 1;
            return Ok(Expr::Deref(Box::new(e)));
        }
        let start = *pos;
        while *pos < b.len() && b[*pos].is_ascii_alphanumeric() {
            *pos += 1;
        }
        let w = b[start..*pos].iter().collect::<String>().to_ascii_lowercase();
        if let Some(r) = REGS.iter().position(|&r| r == w) {
            return Ok(Expr::Reg(r as u8));
        }
        if let Some(n) = w.strip_prefix("arg").and_then(|n| n.parse::<u8>().ok()) {
            return Ok(Expr::Arg(n));
        }
        num(&w)
            .map(Expr::Num)
            .ok_or_else(|| format!("expression: bad term {w:?}"))
    }

    /// `None` when a dereference cannot be read or an argument is missing.
    pub fn eval(&self, regs: &[u32; 7], args: &[u32], read: &dyn Fn(u32) -> Option<u32>) -> Option<u32> {
        Some(match self {
            Expr::Num(n) => *n,
            Expr::Reg(r) => regs[*r as usize],
            Expr::Arg(a) => *args.get(*a as usize)?,
            Expr::Deref(e) => read(e.eval(regs, args, read)?)?,
            Expr::Sum(t) => {
                let mut v = 0u32;
                for (neg, e) in t {
                    let x = e.eval(regs, args, read)?;
                    v = if *neg {
                        v.wrapping_sub(x)
                    } else {
                        v.wrapping_add(x)
                    };
                }
                v
            }
        })
    }
}

/// One `SA_L2_EXTRA` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extra {
    /// Only for this hook (index); `None`: every enabled hook.
    pub hook: Option<usize>,
    pub addr: Expr,
    pub len: Expr,
    pub acc: u8,
}

impl Extra {
    /// `[HOOK@]ADDR:LEN[:R|W|RW]`
    pub fn parse(e: &str) -> Result<Extra, String> {
        let bad = |why: String| format!("SA_L2_EXTRA: {e:?} ([HOOK@]ADDR:LEN[:R|W|RW]): {why}");
        let (hook, rest) = match e.rsplit_once('@') {
            Some((h, r)) => {
                let i = crate::table::find(h.trim()).ok_or_else(|| bad(format!("unknown hook {h:?}")))?;
                (Some(i), r)
            }
            None => (None, e),
        };
        let p: Vec<&str> = rest.split(':').collect();
        if p.len() > 3 {
            return Err(bad("too many fields".into()));
        }
        let addr = Expr::parse(p.first().copied().unwrap_or("")).map_err(bad)?;
        let len = Expr::parse(p.get(1).copied().unwrap_or("")).map_err(bad)?;
        let acc = match p.get(2).copied() {
            None | Some("RW") => 3,
            Some("R") => 1,
            Some("W") => 2,
            Some(a) => return Err(bad(format!("access {a:?}"))),
        };
        Ok(Extra { hook, addr, len, acc })
    }
}

/// `SA_L2_MUTANT` on its own: read even when `SA_L2` is unset, so a mutant can run as a plain Rust
/// hook (INF-9 L3 self-test: the bisection must find the wrong hook from frame hashes alone).
pub fn mutants_from_env(get: &impl Fn(&str) -> Option<String>) -> Result<Vec<usize>, String> {
    match get("SA_L2_MUTANT") {
        Some(v) => hook_list(&v, "SA_L2_MUTANT"),
        None => Ok(Vec::new()),
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
        assert_eq!(
            c.extra,
            vec![
                Extra {
                    hook: None,
                    addr: Expr::Num(0xB7CB4C),
                    len: Expr::Num(4),
                    acc: 3
                },
                Extra {
                    hook: None,
                    addr: Expr::Num(0x10),
                    len: Expr::Num(2),
                    acc: 1
                },
            ]
        );
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
        let get = env(&[("SA_L2_MUTANT", "CGeneral::LimitRadianAngle")]);
        assert_eq!(Config::from_env(&get).unwrap(), None);
        assert_eq!(mutants_from_env(&get).unwrap(), vec![1]);
        assert_eq!(mutants_from_env(&env(&[])).unwrap(), Vec::<usize>::new());
    }

    #[test]
    fn extra_expressions() {
        let e = Extra::parse("CPed::operator delete-anon@[[0xB74490]+4]:[[0xb74490]+8]:R").unwrap();
        assert_eq!(e.hook, crate::table::find("CPed::operator delete-anon"));
        assert_eq!(e.acc, 1);
        let mem = |a: u32| match a {
            0xB74490 => Some(0x1000),
            0x1004 => Some(0x2000),
            0x1008 => Some(140),
            _ => None,
        };
        let regs = [0, 0x1000, 0, 0, 0, 0, 0];
        assert_eq!(e.addr.eval(&regs, &[], &mem), Some(0x2000));
        assert_eq!(e.len.eval(&regs, &[], &mem), Some(140));
        let t = Extra::parse("[ecx+4]-arg1+0x10:8").unwrap();
        assert_eq!(t.hook, None);
        assert_eq!(t.addr.eval(&regs, &[0, 0x20], &mem), Some(0x2000 - 0x20 + 0x10));
        assert_eq!(t.addr.eval(&regs, &[0], &mem), None, "missing argument");
        assert_eq!(
            Expr::parse("[0x5]").unwrap().eval(&regs, &[], &mem),
            None,
            "unreadable"
        );
        assert!(Extra::parse("nope@0x10:4").is_err());
        assert!(Extra::parse("[0x10:4").is_err());
        assert!(Extra::parse("0x10:4:X").is_err());
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
