//! L3 runner and automatic divergence bisection (INF-9, PLAN §5 L3).
//!
//! A recording is replayed in determinism mode with a set of hooks on Rust and the rest of the
//! *universe* (every Core hook with an L2 signature, i.e. every Rust port) on the original. Its
//! per-frame, per-subsystem hash trace is compared with the reference trace (whole universe on the
//! original). On a divergence the culprit hooks are found without help:
//!
//! 1. the first divergent frame F and its subsystems come from the full run;
//! 2. the Rust set is narrowed by switching halves back to the original: each probe replays up to
//!    F + margin and is "bad" when it diverges from the reference anywhere in that window. The search
//!    keeps the other half on Rust when neither half alone is bad (interaction), so it returns a
//!    minimal set M with every hook of M necessary;
//! 3. with `iterate`, M is taken out and the rest is replayed in full again, until it matches: every
//!    independent culprit is reported, each with the frame and subsystems where it alone diverges and
//!    a suggested L2 run (`SA_L2_FROM`).
//!
//! Hook-off check (PLAN §5 L3): for a function F of the set, replay with every hook of the set on Rust
//! except F; the trace must still match the reference.
//!
//! Game runs go through a [`Runner`]: [`CommandRunner`] calls an external command (normally
//! `tools/e2e/l3.sh --probe`) once per probe and caches every complete trace by recording, exe,
//! sa_hook.dll, modes, end frame and the relevant environment.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sa_oracle_format::fnv::Fnv64;
use sa_oracle_format::trace::Trace;

/// Where a probe first differs from the reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Div {
    pub frame: u32,
    /// Subsystems that differ at `frame`; `["<ended>"]` when the probe's trace stops before it.
    pub subsystems: Vec<String>,
}

impl Div {
    pub fn ended(&self) -> bool {
        self.subsystems.first().is_some_and(|s| s == "<ended>")
    }
}

impl std::fmt::Display for Div {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.ended() {
            write!(
                f,
                "trace ends before frame {} (game exited, hung or crashed)",
                self.frame
            )
        } else {
            write!(f, "frame {}: {}", self.frame, self.subsystems.join(","))
        }
    }
}

/// First frame ≤ `end` where `probe` differs from `reference` (frames the reference has). A frame the
/// probe lacks counts as a divergence (the run ended early).
pub fn first_divergence(reference: &Trace, probe: &Trace, end: u32) -> Result<Option<Div>, String> {
    if reference.names != probe.names {
        return Err(format!(
            "subsystem lists differ: {:?} vs {:?}",
            reference.names, probe.names
        ));
    }
    let pm: BTreeMap<u32, &Vec<u64>> = probe.records.iter().map(|r| (r.frame, &r.hashes)).collect();
    for r in reference.records.iter().filter(|r| r.frame <= end) {
        let Some(h) = pm.get(&r.frame) else {
            return Ok(Some(Div {
                frame: r.frame,
                subsystems: vec!["<ended>".into()],
            }));
        };
        let diff: Vec<String> = (0..reference.names.len())
            .filter(|&i| r.hashes[i] != h[i])
            .map(|i| reference.names[i].clone())
            .collect();
        if !diff.is_empty() {
            return Ok(Some(Div {
                frame: r.frame,
                subsystems: diff,
            }));
        }
    }
    Ok(None)
}

/// One game run: `rust` (hook indices) on Rust, the rest of the universe on the original, replayed up
/// to frame `end`. Returns the hash trace (possibly shorter than `end` if the game died).
pub trait Runner {
    fn run(&mut self, rust: &[usize], end: u32) -> Result<Trace, String>;
}

fn names(set: &[usize]) -> String {
    let v: Vec<&str> = set.iter().map(|&i| sa_hook::table::HOOKS[i].name).collect();
    format!("{{{}}}", v.join(", "))
}

/// Every Rust port: the hooks with an L2 signature that are not oracle instruments (sa-hook's own
/// ports and, once registered, the engine crates' ports; probes install the latter with
/// `SA_HOOK_PORTS=orig` and switch them with `SA_HOOK_MODE`). Unverified ports belong to it only when
/// `SA_UNVERIFIED` selects them (the probes' DLL reads the same variable), so default runs never
/// touch unverified code.
pub fn universe() -> Vec<usize> {
    use sa_hook::table::{Group, HOOKS};
    if let Err(e) = sa_hook::table::select_unverified_from_env() {
        eprintln!("warning: {e}");
    }
    (0..HOOKS.len())
        .filter(|&i| HOOKS[i].group != Group::Oracle && HOOKS[i].sig.is_some() && sa_hook::table::enabled(i))
        .collect()
}

/// Parses a comma list into universe indices: `ports`/`all` (the universe), `verified` (its verified
/// hooks), `unverified` (its selected unverified hooks), `batch:ID` (one selected batch), or hook
/// names/addresses.
pub fn hook_set(list: &str) -> Result<Vec<usize>, String> {
    let u = universe();
    let mut out = BTreeSet::new();
    for k in list.split(',').map(str::trim).filter(|k| !k.is_empty()) {
        if k == "ports" || k == "all" {
            out.extend(u.iter().copied());
            continue;
        }
        if k == "verified" || k == "unverified" {
            let want = k == "unverified";
            out.extend(u.iter().copied().filter(|&i| sa_hook::table::batch(i).is_some() == want));
            continue;
        }
        if let Some(b) = k.strip_prefix("batch:") {
            let set: Vec<usize> = u.iter().copied().filter(|&i| sa_hook::table::batch(i) == Some(b)).collect();
            if set.is_empty() {
                return Err(format!(
                    "batch {b:?}: no selected hook (unknown batch, or SA_UNVERIFIED does not select it)"
                ));
            }
            out.extend(set);
            continue;
        }
        let i = sa_hook::table::find(k).ok_or_else(|| format!("unknown hook {k:?}"))?;
        if let Some(b) = sa_hook::table::batch(i)
            && !sa_hook::table::enabled(i)
        {
            return Err(format!(
                "{} is unverified (batch {b}): select it with SA_UNVERIFIED={b} (or the hook name)",
                sa_hook::table::HOOKS[i].name
            ));
        }
        if !u.contains(&i) {
            return Err(format!(
                "{} is not a Rust port (an oracle hook or no L2 signature)",
                sa_hook::table::HOOKS[i].name
            ));
        }
        out.insert(i);
    }
    Ok(out.into_iter().collect())
}

/// `SA_HOOK_MODE` value: `ADDR=rust` for the set, `ADDR=orig` for the rest of the universe.
pub fn modes(universe: &[usize], rust: &[usize]) -> String {
    universe
        .iter()
        .map(|&i| {
            format!(
                "{:#x}={}",
                sa_hook::table::HOOKS[i].addr,
                if rust.contains(&i) { "rust" } else { "orig" }
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// One probe as it happened (for the report).
#[derive(Debug, Clone)]
pub struct ProbeLog {
    pub rust: Vec<usize>,
    pub end: u32,
    pub div: Option<Div>,
    /// Answered from an earlier probe of this session (no game run).
    pub memo: bool,
}

/// Progress callback, called after every probe.
pub type ProbeHook = Box<dyn FnMut(&ProbeLog)>;

pub struct Bisector<R: Runner> {
    pub runner: R,
    pub reference: Trace,
    /// set -> (end it was run to, first divergence ≤ end)
    memo: BTreeMap<Vec<usize>, (u32, Option<Div>)>,
    pub log: Vec<ProbeLog>,
    /// Called after every probe (progress output).
    pub on_probe: Option<ProbeHook>,
    /// Hooks on Rust in every probe besides the tested set (`--fixed`, e.g. the verified ports while
    /// one unverified batch is bisected). The tested sets and the report never list them.
    pub fixed: Vec<usize>,
}

/// A minimal culprit set and where it diverges on its own.
#[derive(Debug, Clone)]
pub struct Culprit {
    pub hooks: Vec<usize>,
    /// First divergence of a run with only `hooks` on Rust (within the probe window).
    pub alone: Option<Div>,
    /// The full-run divergence this culprit was searched from.
    pub from: Div,
}

#[derive(Debug, Clone)]
pub struct L3Report {
    pub rust: Vec<usize>,
    pub full_end: u32,
    /// First divergence of the full set over the whole recording.
    pub full: Option<Div>,
    pub culprits: Vec<Culprit>,
    /// The set minus every culprit found; matches the reference when `clean_rest` is true.
    pub rest: Vec<usize>,
    pub clean_rest: bool,
}

impl<R: Runner> Bisector<R> {
    pub fn new(runner: R, reference: Trace) -> Self {
        Bisector {
            runner,
            reference,
            memo: BTreeMap::new(),
            log: Vec::new(),
            on_probe: None,
            fixed: Vec::new(),
        }
    }

    pub fn reference_end(&self) -> u32 {
        self.reference.records.last().map(|r| r.frame).unwrap_or(0)
    }

    fn note(&mut self, p: ProbeLog) {
        if let Some(f) = self.on_probe.as_mut() {
            f(&p);
        }
        self.log.push(p);
    }

    /// First divergence of `set` (plus [`Self::fixed`], on Rust) within frames ≤ `end`. The empty set
    /// without fixed hooks is the reference.
    pub fn test(&mut self, set: &[usize], end: u32) -> Result<Option<Div>, String> {
        let mut shown = set.to_vec();
        shown.sort_unstable();
        shown.dedup();
        shown.retain(|i| !self.fixed.contains(i));
        let mut key: Vec<usize> = shown.iter().chain(&self.fixed).copied().collect();
        key.sort_unstable();
        key.dedup();
        if key.is_empty() {
            return Ok(None);
        }
        if let Some((e, d)) = self.memo.get(&key).cloned() {
            let known = match &d {
                Some(d) if d.frame <= end => Some(Some(d.clone())),
                _ if e >= end => Some(None),
                _ => None,
            };
            if let Some(r) = known {
                self.note(ProbeLog {
                    rust: shown,
                    end,
                    div: r.clone(),
                    memo: true,
                });
                return Ok(r);
            }
        }
        let t = self.runner.run(&key, end)?;
        let d = first_divergence(&self.reference, &t, end)?;
        self.memo.insert(key, (end, d.clone()));
        self.note(ProbeLog {
            rust: shown,
            end,
            div: d.clone(),
            memo: false,
        });
        Ok(d)
    }

    /// Minimal subset M of `cand` such that `fixed ∪ M` diverges by `end` (precondition:
    /// `fixed ∪ cand` diverges). Binary splitting; when neither half diverges on its own the two
    /// halves interact and each is minimised with the other's minimum kept on Rust.
    pub fn minimize(&mut self, fixed: &[usize], cand: &[usize], end: u32) -> Result<Vec<usize>, String> {
        if cand.len() <= 1 {
            return Ok(cand.to_vec());
        }
        let (a, b) = cand.split_at(cand.len() / 2);
        let with = |x: &[usize]| -> Vec<usize> { fixed.iter().chain(x).copied().collect() };
        if self.test(&with(a), end)?.is_some() {
            return self.minimize(fixed, a, end);
        }
        if self.test(&with(b), end)?.is_some() {
            return self.minimize(fixed, b, end);
        }
        let fb: Vec<usize> = with(b);
        let ma = self.minimize(&fb, a, end)?;
        let fa: Vec<usize> = with(&ma);
        let mb = self.minimize(&fa, b, end)?;
        let mut m: Vec<usize> = ma.into_iter().chain(mb).collect();
        m.sort_unstable();
        Ok(m)
    }

    /// The L3 check of `rust` over the whole recording, with bisection on divergence.
    pub fn l3(&mut self, rust: &[usize], margin: u32, iterate: bool) -> Result<L3Report, String> {
        let full_end = self.reference_end();
        let mut rest: Vec<usize> = rust.to_vec();
        rest.sort_unstable();
        rest.retain(|i| !self.fixed.contains(i));
        if !self.fixed.is_empty()
            && let Some(d) = self.test(&[], full_end)?
        {
            return Err(format!(
                "the fixed set alone diverges at {d}: it must match the reference before a batch on top of it is bisected"
            ));
        }
        let full = self.test(&rest, full_end)?;
        let mut culprits = Vec::new();
        let mut cur = full.clone();
        while let Some(d) = cur {
            let end = (d.frame + margin).min(full_end);
            let m = self.minimize(&[], &rest, end)?;
            let alone = self.test(&m, end)?;
            culprits.push(Culprit {
                hooks: m.clone(),
                alone,
                from: d,
            });
            rest.retain(|i| !m.contains(i));
            if !iterate || rest.is_empty() {
                break;
            }
            cur = self.test(&rest, full_end)?;
        }
        let clean_rest = rest.is_empty() || self.test(&rest, full_end)?.is_none();
        Ok(L3Report {
            rust: rust.to_vec(),
            full_end,
            full,
            culprits,
            rest,
            clean_rest,
        })
    }

    /// PLAN §5 hook-off check: for each `f` in `fns`, the set without `f` must match the reference over
    /// the whole recording. Returns (f, divergence) per function.
    pub fn hook_off(&mut self, rust: &[usize], fns: &[usize]) -> Result<Vec<(usize, Option<Div>)>, String> {
        let end = self.reference_end();
        let mut out = Vec::new();
        for &f in fns {
            let set: Vec<usize> = rust.iter().copied().filter(|&i| i != f).collect();
            out.push((f, self.test(&set, end)?));
        }
        Ok(out)
    }
}

/// Text report of an L3 run.
pub fn print_report(r: &L3Report, margin: u32) -> String {
    use sa_hook::table::HOOKS;
    let mut s = String::new();
    match &r.full {
        None => {
            s += &format!(
                "L3 MATCH: {} on Rust, identical to the reference for all {} frames\n",
                names(&r.rust),
                r.full_end
            );
            return s;
        }
        Some(d) => {
            s += &format!(
                "L3 DIVERGED: {} on Rust; first divergence at {d}\n",
                names(&r.rust)
            );
        }
    }
    for c in &r.culprits {
        s += &format!(
            "CULPRIT {} (searched from the divergence at {}, window up to frame {})\n",
            names(&c.hooks),
            c.from,
            (c.from.frame + margin).min(r.full_end)
        );
        match &c.alone {
            Some(d) => s += &format!("  alone on Rust: diverges at {d}\n"),
            None => s += "  alone on Rust: no divergence in the window (only together with others)\n",
        }
        let frame = c.alone.as_ref().unwrap_or(&c.from).frame;
        let from = frame.saturating_sub(1);
        for &i in &c.hooks {
            s += &format!(
                "  suggest L2: SA_L2={:#x} SA_L2_FROM={from} SA_L2_PER_FRAME=1000 SA_L2_MAX=200 \
                 (tools/e2e/l3.sh --l2 {:#x} --from {from} --end {}): {} at frame {frame}\n",
                HOOKS[i].addr,
                HOOKS[i].addr,
                (frame + margin).min(r.full_end),
                HOOKS[i].name,
            );
        }
    }
    s += &format!(
        "rest of the set {} {}\n",
        names(&r.rest),
        if r.clean_rest {
            format!("matches the reference for all {} frames", r.full_end)
        } else {
            "still diverges (search stopped; run without --no-iterate)".into()
        }
    );
    s
}

pub fn print_probe(p: &ProbeLog) -> String {
    format!(
        "probe {} end={} {}: {}",
        names(&p.rust),
        p.end,
        if p.memo { "(known)" } else { "(game run)" },
        match &p.div {
            Some(d) => format!("DIVERGED at {d}"),
            None => "match".into(),
        }
    )
}

// ------------------------------------------------------------------ external runner -------------

/// `fnv64:size` of a file ("missing" if unreadable).
pub fn fingerprint(p: &Path) -> String {
    match std::fs::read(p) {
        Ok(b) => {
            let mut h = Fnv64::new();
            h.write(&b);
            format!("{:016x}:{}", h.0, b.len())
        }
        Err(_) => "missing".into(),
    }
}

/// Environment variables that change a run's outcome and so belong to the cache key.
pub const KEY_ENV: &[&str] = &["SA_DET_STEP_US", "SA_DET_SEED", "SA_DET_PERTURB"];
/// Changes only runs with Rust hooks (a mutant is a wrong Rust implementation), so the reference
/// (everything on the original) is shared with and without it.
pub const KEY_ENV_RUST: &[&str] = &["SA_L2_MUTANT"];

/// Runs `cmd NAME END MODES OUT_TRACE` per probe (the command does one game replay and writes the
/// trace), with a trace cache.
pub struct CommandRunner {
    pub cmd: Vec<String>,
    pub universe: Vec<usize>,
    /// Identifies recording + exe + dll + environment.
    pub key_base: String,
    /// `KEY_ENV_RUST` part of the key (only for probes with a Rust hook).
    pub key_rust: String,
    pub cache: PathBuf,
    pub work: PathBuf,
    pub runs: usize,
    pub cache_hits: usize,
}

impl CommandRunner {
    pub fn new(cmd: Vec<String>, rec: &Path, exe: &Path, dll: &Path, cache: PathBuf, work: PathBuf) -> Self {
        let mut key = format!(
            "l3-v1 rec={} exe={} dll={}",
            fingerprint(rec),
            fingerprint(exe),
            fingerprint(dll)
        );
        for k in KEY_ENV {
            if let Ok(v) = std::env::var(k) {
                key += &format!(" {k}={v}");
            }
        }
        let mut key_rust = String::new();
        for k in KEY_ENV_RUST {
            if let Ok(v) = std::env::var(k) {
                key_rust += &format!(" {k}={v}");
            }
        }
        CommandRunner {
            cmd,
            universe: universe(),
            key_base: key,
            key_rust,
            cache,
            work,
            runs: 0,
            cache_hits: 0,
        }
    }

    fn key(&self, modes: &str, end: u32, any_rust: bool) -> String {
        let mut h = Fnv64::new();
        let rust = if any_rust { self.key_rust.as_str() } else { "" };
        h.write(format!("{}{rust} modes={modes} end={end}", self.key_base).as_bytes());
        format!("{:016x}", h.0)
    }
}

impl Runner for CommandRunner {
    fn run(&mut self, rust: &[usize], end: u32) -> Result<Trace, String> {
        let m = modes(&self.universe, rust);
        let key = self.key(&m, end, !rust.is_empty());
        let cached = self.cache.join(format!("{key}.trace"));
        if let Ok(b) = std::fs::read(&cached)
            && let Ok(t) = Trace::parse(&b)
        {
            self.cache_hits += 1;
            return Ok(t);
        }
        std::fs::create_dir_all(&self.work).map_err(|e| format!("{}: {e}", self.work.display()))?;
        self.runs += 1;
        let name = format!("p{:03}-{key}", self.runs);
        let out = self.work.join(format!("{name}.trace"));
        let _ = std::fs::remove_file(&out);
        let (prog, args) = self.cmd.split_first().ok_or("empty runner command")?;
        let st = std::process::Command::new(prog)
            .args(args)
            .arg(&name)
            .arg(end.to_string())
            .arg(&m)
            .arg(&out)
            .status()
            .map_err(|e| format!("runner {prog}: {e}"))?;
        if !st.success() {
            return Err(format!("runner {prog} {name}: {st}"));
        }
        let b = std::fs::read(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        let t = Trace::parse(&b)?;
        // Only complete traces are cached (an early end may be an environment problem).
        if t.records.last().is_some_and(|r| r.frame >= end) {
            std::fs::create_dir_all(&self.cache).map_err(|e| format!("{}: {e}", self.cache.display()))?;
            std::fs::write(&cached, &b).map_err(|e| format!("{}: {e}", cached.display()))?;
        }
        Ok(t)
    }
}

// ------------------------------------------------------------------ CLI ---------------------------

/// `--key value` options and `--no-x` flags.
pub fn parse_opts(args: &[String]) -> Result<(BTreeMap<String, String>, BTreeSet<String>), String> {
    let (mut o, mut f) = (BTreeMap::new(), BTreeSet::new());
    let mut i = 0;
    while i < args.len() {
        let k = args[i]
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument {:?}", args[i]))?;
        if k.starts_with("no-") {
            f.insert(k.to_string());
        } else {
            let v = args.get(i + 1).ok_or_else(|| format!("--{k} needs a value"))?;
            o.insert(k.to_string(), v.clone());
            i += 1;
        }
        i += 1;
    }
    Ok((o, f))
}

pub const USAGE: &str = "usage: sa-oracle l3 --rec R.sarec --runner \"CMD ARGS\" [--rust ports|A,B] [--margin 100] \
    [--hookoff none|all|A,B] [--end N] [--work DIR] [--cache DIR] [--exe gta_sa.exe] [--dll sa_hook.dll] \
    [--report FILE] [--no-iterate] [--fixed SET]\n       sa-oracle l3 --print-modes ports|A,B [--except A,B]";

/// `sa-oracle l3 ...`: returns Ok(true) when the set matches the reference and every hook-off run
/// matches.
pub fn cli(args: &[String]) -> Result<bool, String> {
    let (o, flags) = parse_opts(args)?;
    if let Some(list) = o.get("print-modes") {
        // SA_HOOK_MODE for a set (minus --except), as the probes use it
        let ex = o
            .get("except")
            .map(|e| hook_set(e))
            .transpose()?
            .unwrap_or_default();
        let set: Vec<usize> = hook_set(list)?.into_iter().filter(|i| !ex.contains(i)).collect();
        println!("{}", modes(&universe(), &set));
        return Ok(true);
    }
    let need = |k: &str| o.get(k).cloned().ok_or_else(|| format!("--{k} missing\n{USAGE}"));
    let rec_path = PathBuf::from(need("rec")?);
    let rec_bytes = std::fs::read(&rec_path).map_err(|e| format!("{}: {e}", rec_path.display()))?;
    let rec = sa_oracle_format::recording::Recording::parse(&rec_bytes)?;
    let full_end: u32 = match o.get("end") {
        Some(v) => v.parse().map_err(|_| format!("--end {v}"))?,
        None => rec
            .header
            .get("frames")
            .and_then(|f| f.parse().ok())
            .ok_or("recording has no `frames` header; give --end")?,
    };
    let mut rust = hook_set(o.get("rust").map(String::as_str).unwrap_or("ports"))?;
    let margin: u32 = o
        .get("margin")
        .map(|v| v.parse().map_err(|_| format!("--margin {v}")))
        .transpose()?
        .unwrap_or(100);
    let root = crate::workspace_root();
    let work = o
        .get("work")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/l3/work"));
    let cache = o
        .get("cache")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/l3/cache"));
    let exe = o
        .get("exe")
        .map(PathBuf::from)
        .or_else(crate::pe::find_exe)
        .ok_or("--exe missing")?;
    let dll = o
        .get("dll")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/i686-pc-windows-gnu/release/sa_hook.dll"));
    let cmd: Vec<String> = need("runner")?.split_whitespace().map(String::from).collect();
    let mut runner = CommandRunner::new(cmd, &rec_path, &exe, &dll, cache, work);
    let mut out = String::new();
    let say = |s: String, out: &mut String| {
        println!("{s}");
        out.push_str(&s);
        out.push('\n');
    };
    say(
        format!(
            "L3 {} ({} frames); rust set {}; universe {}; margin {margin}; key {}",
            rec_path.display(),
            full_end,
            names(&rust),
            names(&universe()),
            runner.key_base
        ),
        &mut out,
    );
    let reference = runner.run(&[], full_end)?;
    let rlast = reference.records.last().map(|r| r.frame).unwrap_or(0);
    say(
        format!(
            "reference (universe on the original): {} frames, last {rlast} ({})",
            reference.records.len(),
            if runner.cache_hits > 0 {
                "cached"
            } else {
                "game run"
            }
        ),
        &mut out,
    );
    if rlast < full_end {
        return Err(format!("the reference run ended at frame {rlast} < {full_end}"));
    }
    let mut b = Bisector::new(runner, reference);
    if let Some(f) = o.get("fixed") {
        b.fixed = hook_set(f)?;
        rust.retain(|i| !b.fixed.contains(i));
        say(
            format!(
                "FIXED {} hooks on Rust in every probe (not bisected); tested set {}",
                b.fixed.len(),
                names(&rust)
            ),
            &mut out,
        );
    }
    b.on_probe = Some(Box::new(|p: &ProbeLog| println!("  {}", print_probe(p))));
    let r = b.l3(&rust, margin, !flags.contains("no-iterate"))?;
    let rep = print_report(&r, margin);
    print!("{rep}");
    out += &rep;
    let mut ok = r.full.is_none();
    let off: Vec<usize> = match o.get("hookoff").map(String::as_str).unwrap_or("none") {
        "none" => Vec::new(),
        "all" => r.rest.clone(),
        l => hook_set(l)?,
    };
    if !off.is_empty() {
        if r.clean_rest && !r.rest.is_empty() {
            say(
                format!(
                    "hook-off check on the matching set {} (PLAN §5 L3)",
                    names(&r.rest)
                ),
                &mut out,
            );
            for (f, d) in b.hook_off(&r.rest, &off)? {
                ok &= d.is_none();
                say(
                    format!(
                        "HOOKOFF {} back on the original, rest on Rust: {}",
                        sa_hook::table::HOOKS[f].name,
                        match &d {
                            None => format!("MATCH ({} frames)", r.full_end),
                            Some(d) => format!("DIVERGED at {d}"),
                        }
                    ),
                    &mut out,
                );
            }
        } else {
            ok = false;
            say("hook-off check skipped: no matching Rust set".into(), &mut out);
        }
    }
    let memo = b.log.iter().filter(|p| p.memo).count();
    say(
        format!(
            "probes: {} ({} game runs, {} from the trace cache, {memo} answered from earlier probes)",
            b.log.len(),
            b.runner.runs,
            b.runner.cache_hits
        ),
        &mut out,
    );
    if let Some(p) = o.get("report") {
        std::fs::write(p, &out).map_err(|e| format!("{p}: {e}"))?;
    }
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_oracle_format::subsys::{COUNT, Subsystem};
    use sa_oracle_format::trace::{encode_record, header};

    /// Simulated game: each "bad" hook perturbs `subsys` from its onset frame on; an interaction pair
    /// only diverges when both are on Rust.
    struct Sim {
        end: u32,
        bad: Vec<(usize, u32, Subsystem)>,
        pair: Option<(usize, usize, u32)>,
        crash: Option<(usize, u32)>,
        runs: usize,
    }

    impl Sim {
        fn trace(&self, rust: &[usize], end: u32) -> Trace {
            let mut v = header();
            for f in 1..=end.min(self.end) {
                if self.crash.is_some_and(|(h, at)| rust.contains(&h) && f >= at) {
                    break;
                }
                let mut h = [f as u64; COUNT];
                for &(i, at, s) in &self.bad {
                    if rust.contains(&i) && f >= at {
                        h[s as usize] ^= 1 << i;
                    }
                }
                if let Some((a, b, at)) = self.pair
                    && rust.contains(&a)
                    && rust.contains(&b)
                    && f >= at
                {
                    h[Subsystem::Camera as usize] ^= 0x100;
                }
                v.extend(encode_record(f, f, &h));
            }
            Trace::parse(&v).unwrap()
        }
    }

    impl Runner for &mut Sim {
        fn run(&mut self, rust: &[usize], end: u32) -> Result<Trace, String> {
            self.runs += 1;
            Ok(self.trace(rust, end))
        }
    }

    fn sim(bad: Vec<(usize, u32, Subsystem)>) -> Sim {
        Sim {
            end: 1000,
            bad,
            pair: None,
            crash: None,
            runs: 0,
        }
    }

    const SET: [usize; 8] = [0, 1, 2, 14, 15, 16, 17, 18];

    #[test]
    fn finds_a_single_culprit() {
        let mut s = sim(vec![(1, 265, Subsystem::Peds)]);
        let reference = s.trace(&[], 1000);
        let mut b = Bisector::new(&mut s, reference);
        let r = b.l3(&SET, 50, true).unwrap();
        assert_eq!(
            r.full,
            Some(Div {
                frame: 265,
                subsystems: vec!["peds".into()]
            })
        );
        assert_eq!(r.culprits.len(), 1);
        assert_eq!(r.culprits[0].hooks, vec![1]);
        assert_eq!(r.culprits[0].alone.as_ref().unwrap().frame, 265);
        assert!(r.clean_rest);
        assert_eq!(r.rest.len(), 7);
        let txt = print_report(&r, 50);
        assert!(txt.contains("CULPRIT {CGeneral::LimitRadianAngle}"), "{txt}");
        assert!(txt.contains("SA_L2=0x53cb50 SA_L2_FROM=264"), "{txt}");
        // full + log2(8) halving probes + the clean rest; no more than ~2 log2 n + 2 game runs
        let runs = s.runs;
        assert!(runs <= 8, "{runs} runs");
    }

    #[test]
    fn finds_independent_culprits_and_interactions() {
        let mut s = sim(vec![(16, 400, Subsystem::Camera), (18, 300, Subsystem::Peds)]);
        s.pair = Some((2, 15, 500));
        let reference = s.trace(&[], 1000);
        let mut b = Bisector::new(&mut s, reference);
        let r = b.l3(&SET, 20, true).unwrap();
        let found: Vec<Vec<usize>> = r.culprits.iter().map(|c| c.hooks.clone()).collect();
        assert_eq!(found, vec![vec![18], vec![16], vec![2, 15]]);
        assert_eq!(r.culprits[2].alone.as_ref().unwrap().frame, 500);
        assert!(r.clean_rest);
        let r1 = b.l3(&SET, 20, false).unwrap();
        assert_eq!(r1.culprits.len(), 1);
        assert!(!r1.clean_rest);
    }

    #[test]
    fn match_hook_off_and_crash() {
        let mut s = sim(vec![]);
        s.crash = Some((17, 700));
        let reference = s.trace(&[], 1000);
        let mut b = Bisector::new(&mut s, reference);
        let r = b.l3(&[0, 1, 2], 50, true).unwrap();
        assert_eq!(r.full, None);
        assert!(print_report(&r, 50).starts_with("L3 MATCH"));
        let off = b.hook_off(&[0, 1, 2], &[0, 1, 2]).unwrap();
        assert!(off.iter().all(|(_, d)| d.is_none()));
        let r = b.l3(&SET, 50, true).unwrap();
        assert_eq!(r.culprits[0].hooks, vec![17]);
        assert!(r.full.as_ref().unwrap().ended());
        // hook-off of a set that only matches without 17
        let off = b.hook_off(&SET, &[17, 1]).unwrap();
        assert_eq!(off[0].1, None);
        assert!(off[1].1.is_some());
    }

    #[test]
    fn memo_answers_shorter_windows() {
        let mut s = sim(vec![(1, 265, Subsystem::Peds)]);
        let reference = s.trace(&[], 1000);
        let mut b = Bisector::new(&mut s, reference);
        assert!(b.test(&[1, 2], 1000).unwrap().is_some());
        assert!(b.test(&[2], 1000).unwrap().is_none());
        assert!(b.test(&[2, 1], 300).unwrap().is_some());
        assert!(b.test(&[2], 500).unwrap().is_none());
        assert!(b.test(&[1], 200).unwrap().is_none());
        assert!(b.test(&[1], 100).unwrap().is_none());
        drop(b);
        assert_eq!(s.runs, 3);
    }

    #[test]
    fn sets_and_modes() {
        let u = universe();
        assert_eq!(u[..SET.len()], SET);
        assert_eq!(hook_set("ports").unwrap(), u);
        assert_eq!(
            hook_set("CGeneral::LimitRadianAngle, 0x441390").unwrap(),
            vec![1, 2]
        );
        assert!(hook_set("_rand").is_err());
        assert!(hook_set("nope").is_err());
        let m = modes(&u, &[1]);
        assert!(m.starts_with("0x561b10=orig,0x53cb50=rust,0x441390=orig"), "{m}");
        assert_eq!(sa_hook::l2::parse_hook_modes(&m).unwrap().len(), u.len());
    }
}
