//! Property runner: for a [`Spec`], run the edge cases plus N generated cases on the original (Unicorn,
//! or the real CPU through the native backend) and on the Rust port, compare bit-exactly, shrink the
//! mismatches to minimal reproducers and store them as fixtures under `fixtures/l1/` (gitignored).
//!
//! Determinism: case `i` of seed `s` is `edges[i]` for `i < edges.len()`, else generated from
//! `Rng::for_case(s, i)`.
//!
//! Which backend decides a case:
//! - every case runs under Unicorn first (guard rails: allow-list, budget, global accesses);
//! - backend `Native`: every case that returned under Unicorn is re-run on the real CPU, which decides;
//! - backend `Unicorn`: Unicorn decides, except for the input class Unicorn is measured to get wrong for
//!   every function — a signalling-NaN float input (QEMU 5.0 `fld` does not quiet it). Those cases go to
//!   the real CPU when the native backend is available, else they are reported as unverified.
//! - Unicorn runs with the MXCSR.FZ workaround (`emu`) when the static scan finds no SSE instruction.

use crate::call::*;
use crate::emu::{Allow, Emu};
use crate::generate::Rng;
use crate::pe::Image;
use crate::specs::{ExpRet, Expect, Inputs, RetKind, Route, Spec};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Unicorn,
    /// Real CPU under Wine (see the module docs).
    Native,
}

#[derive(Clone, Debug)]
pub struct Cfg {
    pub cases: u64,
    pub seed: u64,
    pub threads: usize,
    pub backend: Backend,
    pub fpu: Fpu,
    /// write failing cases under fixtures/l1/
    pub fixtures: bool,
    /// mismatches kept with details (and shrunk)
    pub keep: usize,
    /// cases per block (memory bound)
    pub block: usize,
    /// send signalling-NaN input cases to the CPU when the backend is Unicorn
    pub snan_native: bool,
    /// shrink kept mismatches
    pub shrink: bool,
}

impl Default for Cfg {
    fn default() -> Self {
        Cfg {
            cases: 1_000_000,
            seed: 0x5A_0001,
            // one Unicorn instance (~25 MB) per thread; capped to bound memory
            threads: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
                .min(16),
            backend: Backend::Unicorn,
            fpu: Fpu::default(),
            fixtures: true,
            keep: 8,
            block: 50_000,
            snan_native: true,
            shrink: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Mismatch {
    pub index: u64,
    pub input: Inputs,
    pub kind: String,
    pub detail: String,
    pub shrunk: Option<(Inputs, String)>,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub key: String,
    pub name: String,
    pub addr: u32,
    pub backend: String,
    pub route: String,
    pub edge_cases: u64,
    pub generated: u64,
    pub hangs: u64,
    pub mismatches: u64,
    pub kinds: BTreeMap<String, u64>,
    pub examples: Vec<Mismatch>,
    /// ST0 differs in the extended bits only (counted, not a mismatch, when the spec is not st0_exact)
    pub st0_ext_only: u64,
    /// cases decided by the CPU because of a signalling-NaN input (backend Unicorn)
    pub snan_routed: u64,
    /// cases nobody faithful decided (signalling-NaN input, native backend unavailable)
    pub unverified: u64,
    /// cases run on both: Unicorn and the CPU disagree on an observable field
    pub both: u64,
    pub unicorn_vs_cpu: u64,
    pub unicorn_vs_cpu_kinds: BTreeMap<String, u64>,
    /// cases where they disagree only in C1 / exception flags / undefined condition codes
    pub unicorn_vs_cpu_flags_only: u64,
    pub ftz_workaround: bool,
    pub hazards: String,
    pub elapsed_s: f64,
    pub fixtures_written: usize,
}

impl Report {
    pub fn total(&self) -> u64 {
        self.edge_cases + self.generated
    }
}

/// Inputs of case `i`.
pub fn case_input(spec: &Spec, edges: &[Inputs], seed: u64, i: u64) -> Inputs {
    if (i as usize) < edges.len() {
        edges[i as usize].clone()
    } else {
        (spec.generate)(&mut Rng::for_case(seed, i))
    }
}

/// A float input that is a signalling NaN.
pub fn has_snan(spec: &Spec, i: &Inputs) -> bool {
    i[..spec.floats.min(i.len())]
        .iter()
        .any(|&b| b & 0x7F80_0000 == 0x7F80_0000 && b & 0x7F_FFFF != 0 && b & 0x40_0000 == 0)
}

/// Static facts that decide how Unicorn runs a spec.
#[derive(Clone, Debug)]
pub struct Prepared {
    pub allow: Allow,
    pub ftz_workaround: bool,
    pub hazards: crate::scan::Hazards,
}

pub fn prepare(img: &Image, spec: &Spec) -> Result<Prepared, String> {
    let cat = crate::catalog_cached().ok_or("catalog/procedures.csv not found")?;
    let mut allow = Allow::function(cat, spec.addr, spec.callees)?;
    allow.reads = spec.reads.clone();
    let hazards = crate::scan::scan(img, cat, spec.addr, spec.callees)?;
    Ok(Prepared {
        allow,
        ftz_workaround: hazards.sse.is_none(),
        hazards,
    })
}

/// Runs `calls` under Unicorn on `threads` threads (one instance each).
pub fn unicorn_batch(
    img: &Arc<Image>,
    spec: &Spec,
    prep: &Prepared,
    calls: &[Call],
    threads: usize,
) -> Result<Vec<CallResult>, String> {
    let threads = threads.max(1).min(calls.len().max(1));
    let chunk = calls.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let hs: Vec<_> = calls
            .chunks(chunk)
            .map(|part| {
                let img = img.clone();
                s.spawn(move || -> Result<Vec<CallResult>, String> {
                    let mut emu = Emu::new(img)?;
                    emu.budget = spec.budget;
                    emu.ftz_workaround = prep.ftz_workaround;
                    Ok(part.iter().map(|c| emu.call(c, &prep.allow)).collect())
                })
            })
            .collect();
        let mut out = Vec::with_capacity(calls.len());
        for h in hs {
            out.extend(h.join().map_err(|_| "worker panicked".to_string())??);
        }
        Ok(out)
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Unicorn,
    Cpu,
    Unverified,
}

pub struct Batch {
    /// authoritative result per case
    pub res: Vec<CallResult>,
    pub source: Vec<Source>,
    /// Unicorn's result per case
    pub uni: Vec<CallResult>,
}

/// Runs a batch (see the module docs for which backend decides).
pub fn oracle_batch(
    img: &Arc<Image>,
    spec: &Spec,
    prep: &Prepared,
    inputs: &[Inputs],
    calls: &[Call],
    cfg: &Cfg,
) -> Result<Batch, String> {
    let uni = unicorn_batch(img, spec, prep, calls, cfg.threads)?;
    let mut source = vec![Source::Unicorn; calls.len()];
    let native_ok = cfg.backend == Backend::Native || (cfg.snan_native && crate::native::available());
    let mut idx = Vec::new();
    for k in 0..calls.len() {
        let snan = has_snan(spec, &inputs[k]);
        let want = cfg.backend == Backend::Native || snan;
        if want && uni[k].is_ok() {
            if native_ok {
                idx.push(k);
                source[k] = Source::Cpu;
            } else {
                source[k] = Source::Unverified;
            }
        }
    }
    let mut res = uni.clone();
    if !idx.is_empty() {
        let sub: Vec<Call> = idx.iter().map(|&k| calls[k].clone()).collect();
        let cpu = crate::native::run(img, &sub, None)?;
        for (k, o) in idx.into_iter().zip(cpu) {
            res[k] = Ok(o);
        }
    }
    Ok(Batch { res, source, uni })
}

/// Compares the oracle with the port. `Ok(ext_only)` when equal (`ext_only`: ST0 differs only beyond
/// the f32 bits and the spec does not require them); `Err((kind, detail))` otherwise.
pub fn compare(
    spec: &Spec,
    call: &Call,
    oracle: &CallResult,
    exp: &Expect,
) -> Result<bool, (String, String)> {
    let o = match (oracle, exp) {
        (Err(CallError::Budget { .. }), Expect::Hangs) => return Ok(false),
        (Err(CallError::Budget { budget }), _) => {
            return Err((
                "hang".into(),
                format!("original exhausted {budget} instructions, port returns"),
            ));
        }
        (Err(e), _) => return Err(("oracle-error".into(), e.to_string())),
        (Ok(_), Expect::Hangs) => {
            return Err(("hang".into(), "port says the original hangs, it returned".into()));
        }
        (Ok(o), Expect::Returns { .. }) => o,
    };
    let Expect::Returns { ret, scratch } = exp else {
        unreachable!()
    };
    if o.esp_delta != call.expected_esp_delta() {
        return Err((
            "convention".into(),
            format!(
                "esp delta {} (expected {} for {:?})",
                o.esp_delta,
                call.expected_esp_delta(),
                call.conv
            ),
        ));
    }
    let want_depth = if spec.ret == RetKind::St0 { 1 } else { 0 };
    if o.depth() != want_depth {
        return Err((
            "x87-depth".into(),
            format!(
                "x87 stack depth {} after return, expected {want_depth}",
                o.depth()
            ),
        ));
    }
    let mut ext_only = false;
    match ret {
        ExpRet::St0 { f32, ext } => {
            let (of, oe) = (o.st0_f32.unwrap_or(0), o.st0.unwrap_or_default());
            if of != *f32 {
                return Err((
                    "st0-f32".into(),
                    format!(
                        "orig {of:#010x} ({:e}) ext {} | rust {f32:#010x} ({:e}) ext {}",
                        f32::from_bits(of),
                        ext_hex(&oe),
                        f32::from_bits(*f32),
                        ext_hex(ext)
                    ),
                ));
            }
            if oe != *ext {
                if spec.st0_exact {
                    return Err((
                        "st0-ext".into(),
                        format!(
                            "orig {} | rust {} (f32 {of:#010x} equal)",
                            ext_hex(&oe),
                            ext_hex(ext)
                        ),
                    ));
                }
                ext_only = true;
            }
        }
        ExpRet::Al(al) => {
            if (o.eax & 0xFF) as u8 != *al {
                return Err(("al".into(), format!("orig al={} | rust al={al}", o.eax & 0xFF)));
            }
        }
        ExpRet::Eax(v) => {
            if o.eax != *v {
                return Err(("eax".into(), format!("orig eax={:#x} | rust eax={v:#x}", o.eax)));
            }
        }
        ExpRet::Void => {}
    }
    let ignored = |k: usize| spec.ignore.iter().any(|&(o, n)| k >= o && k < o + n);
    if o.scratch.len() != scratch.len() {
        return Err((
            "memory".into(),
            format!("scratch length {} vs {}", o.scratch.len(), scratch.len()),
        ));
    }
    if let Some(k) = (0..scratch.len()).find(|&k| o.scratch[k] != scratch[k] && !ignored(k)) {
        let w = k & !3;
        let e = (w + 4).min(scratch.len());
        return Err((
            "memory".into(),
            format!(
                "first difference at {:#x} (+{w:#x}): orig {:02x?} | rust {:02x?}",
                SCRATCH as usize + w,
                &o.scratch[w..e],
                &scratch[w..e]
            ),
        ));
    }
    Ok(ext_only)
}

/// Fields a caller can observe after the call (C1, exception flags and the condition codes after the
/// return are not: callers compare again before branching).
fn observable(spec: &Spec, field: &str) -> bool {
    match field {
        "st0-f32" | "st0-ext" | "tag" | "TOP" => spec.ret == RetKind::St0 || field == "tag" || field == "TOP",
        "memory" | "esp" | "eax" | "mxcsr" => true,
        _ => false,
    }
}

fn eax_mask(spec: &Spec) -> u32 {
    match spec.ret {
        RetKind::Al => 0xFF,
        RetKind::Eax => 0xFFFF_FFFF,
        _ => 0,
    }
}

fn shrink_candidates(x: f32) -> Vec<f32> {
    let mut c = vec![0.0f32, 1.0, -1.0, 2.0, 0.5, -0.0];
    if x.is_finite() {
        c.push(x.trunc());
        c.push(x.round());
        c.push(-x);
        c.push(x / 2.0);
        let b = x.to_bits();
        for keep in [0u32, 1, 2, 4, 8, 12, 16, 20] {
            let mask = !((1u32 << (23 - keep)) - 1);
            c.push(f32::from_bits(b & mask));
        }
    }
    c
}

/// Simplicity order for floats: fewer significand bits, then smaller |exponent|, then positive.
fn score(x: f32) -> (u32, i32, u32) {
    if x.is_nan() {
        return (100, 1000, 1);
    }
    let b = x.to_bits();
    let e = ((b >> 23) & 0xFF) as i32 - 127;
    (
        (b & 0x7F_FFFF).count_ones(),
        if x == 0.0 { -1000 } else { e.abs() },
        b >> 31,
    )
}

fn make_calls(spec: &Spec, cfg: &Cfg, inputs: &[Inputs]) -> Vec<Call> {
    inputs
        .iter()
        .map(|i| {
            let mut c = (spec.call)(i);
            c.fpu = cfg.fpu;
            c
        })
        .collect()
}

/// For each input: the mismatch (kind, detail), or None.
fn verdicts(
    spec: &Spec,
    img: &Arc<Image>,
    prep: &Prepared,
    cfg: &Cfg,
    inputs: &[Inputs],
) -> Result<Vec<Option<(String, String)>>, String> {
    let calls = make_calls(spec, cfg, inputs);
    let b = oracle_batch(img, spec, prep, inputs, &calls, cfg)?;
    Ok((0..inputs.len())
        .map(|k| {
            if b.source[k] == Source::Unverified {
                return None;
            }
            compare(
                spec,
                &calls[k],
                &b.res[k],
                &(spec.rust)(&inputs[k], &calls[k].scratch),
            )
            .err()
        })
        .collect())
}

/// Greedy shrink of all kept mismatches at once (one batch per round): replace one float at a time by a
/// simpler value while the case still mismatches.
pub fn shrink_all(
    spec: &Spec,
    img: &Arc<Image>,
    prep: &Prepared,
    cfg: &Cfg,
    ms: &mut [Mismatch],
) -> Result<(), String> {
    let mut cur: Vec<Inputs> = ms.iter().map(|m| m.input.clone()).collect();
    let mut detail: Vec<String> = ms.iter().map(|m| format!("{}: {}", m.kind, m.detail)).collect();
    let mut done = vec![false; ms.len()];
    for _round in 0..12 {
        let mut cands: Vec<(usize, Inputs)> = Vec::new();
        for (m, c) in cur.iter().enumerate() {
            if done[m] {
                continue;
            }
            for p in 0..spec.floats.min(c.len()) {
                let x = f32::from_bits(c[p]);
                for y in shrink_candidates(x) {
                    if score(y) < score(x) {
                        let mut n = c.clone();
                        n[p] = y.to_bits();
                        cands.push((m, n));
                    }
                }
            }
        }
        if cands.is_empty() {
            break;
        }
        let inputs: Vec<Inputs> = cands.iter().map(|(_, i)| i.clone()).collect();
        let v = verdicts(spec, img, prep, cfg, &inputs)?;
        let mut progressed = vec![false; ms.len()];
        for ((m, inp), r) in cands.into_iter().zip(v) {
            if let Some((k, d)) = r
                && !progressed[m]
            {
                progressed[m] = true;
                cur[m] = inp;
                detail[m] = format!("{k}: {d}");
            }
        }
        for m in 0..ms.len() {
            done[m] |= !progressed[m];
        }
        if done.iter().all(|d| *d) {
            break;
        }
    }
    for (m, x) in ms.iter_mut().enumerate() {
        x.shrunk = Some((cur[m].clone(), detail[m].clone()));
    }
    Ok(())
}

pub fn fmt_inputs(spec: &Spec, i: &Inputs) -> String {
    i.iter()
        .enumerate()
        .map(|(k, &v)| {
            if k < spec.floats {
                format!("{:e} ({v:#010x})", f32::from_bits(v))
            } else {
                format!("{v}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn write_fixture(spec: &Spec, cfg: &Cfg, m: &Mismatch) -> std::io::Result<()> {
    let dir = crate::workspace_root().join("fixtures/l1").join(spec.key);
    std::fs::create_dir_all(&dir)?;
    let mut s = String::new();
    let _ = writeln!(s, "function: {} @ {:#x}", spec.name, spec.addr);
    let _ = writeln!(
        s,
        "backend: {:?}  cw: {:#06x}  mxcsr: {:#010x}",
        cfg.backend, cfg.fpu.cw, cfg.fpu.mxcsr
    );
    let _ = writeln!(s, "seed: {:#x}  case: {}", cfg.seed, m.index);
    let _ = writeln!(s, "input: {}", fmt_inputs(spec, &m.input));
    let _ = writeln!(s, "{}: {}", m.kind, m.detail);
    if let Some((i, d)) = &m.shrunk {
        let _ = writeln!(s, "minimal input: {}", fmt_inputs(spec, i));
        let _ = writeln!(s, "minimal: {d}");
    }
    std::fs::write(dir.join(format!("s{:x}-c{}.txt", cfg.seed, m.index)), s)
}

/// Runs the spec. `spec.route` is reported, not enforced (the CLI chooses the backend from it).
/// The image with the spec's patches applied (the same image when there are none).
pub fn patched(img: &Arc<Image>, spec: &Spec) -> Arc<Image> {
    if spec.patches.is_empty() {
        return img.clone();
    }
    let mut i = (**img).clone();
    for (a, bytes) in &spec.patches {
        let o = (a - i.base) as usize;
        i.flat[o..o + bytes.len()].copy_from_slice(bytes);
    }
    Arc::new(i)
}

pub fn run(spec: &Spec, img: &Arc<Image>, cfg: &Cfg) -> Result<Report, String> {
    let t0 = Instant::now();
    let img = &patched(img, spec);
    let prep = prepare(img, spec)?;
    let edges = (spec.edges)();
    let total = edges.len() as u64 + cfg.cases;
    let mut rep = Report {
        key: spec.key.into(),
        name: spec.name.into(),
        addr: spec.addr,
        backend: format!("{:?}", cfg.backend),
        route: match spec.route {
            Route::L1 => "L1".into(),
            Route::L2 { .. } => "L2".into(),
        },
        edge_cases: edges.len() as u64,
        generated: cfg.cases,
        ftz_workaround: prep.ftz_workaround,
        hazards: format!(
            "{} instructions scanned; unfaithful x87: [{}]; SSE: {}",
            prep.hazards.instructions,
            prep.hazards
                .x87
                .iter()
                .map(|(m, a)| format!("{m} at {a:#x}"))
                .collect::<Vec<_>>()
                .join(", "),
            prep.hazards
                .sse
                .as_ref()
                .map(|(m, a)| format!("{m} at {a:#x}"))
                .unwrap_or("none".into())
        ),
        ..Default::default()
    };
    let mut start = 0u64;
    while start < total {
        let end = (start + cfg.block as u64).min(total);
        let inputs: Vec<Inputs> = (start..end)
            .map(|i| case_input(spec, &edges, cfg.seed, i))
            .collect();
        let calls = make_calls(spec, cfg, &inputs);
        let b = oracle_batch(img, spec, &prep, &inputs, &calls, cfg)?;
        for k in 0..inputs.len() {
            match b.source[k] {
                Source::Unverified => {
                    rep.unverified += 1;
                    continue;
                }
                Source::Cpu => {
                    if cfg.backend == Backend::Unicorn {
                        rep.snan_routed += 1;
                    }
                    if let (Ok(u), Ok(c)) = (&b.uni[k], &b.res[k]) {
                        rep.both += 1;
                        let d = u.diff(c, eax_mask(spec));
                        let obs: Vec<_> = d.iter().filter(|f| observable(spec, f)).collect();
                        if !obs.is_empty() {
                            rep.unicorn_vs_cpu += 1;
                            for f in obs {
                                *rep.unicorn_vs_cpu_kinds.entry(f.to_string()).or_default() += 1;
                            }
                        } else if !d.is_empty() {
                            rep.unicorn_vs_cpu_flags_only += 1;
                        }
                    }
                }
                Source::Unicorn => {}
            }
            if matches!(b.res[k], Err(CallError::Budget { .. })) {
                rep.hangs += 1;
            }
            let exp = (spec.rust)(&inputs[k], &calls[k].scratch);
            match compare(spec, &calls[k], &b.res[k], &exp) {
                Ok(ext_only) => rep.st0_ext_only += ext_only as u64,
                Err((kind, detail)) => {
                    rep.mismatches += 1;
                    *rep.kinds.entry(kind.clone()).or_default() += 1;
                    if rep.examples.len() < cfg.keep {
                        rep.examples.push(Mismatch {
                            index: start + k as u64,
                            input: inputs[k].clone(),
                            kind,
                            detail,
                            shrunk: None,
                        });
                    }
                }
            }
        }
        start = end;
    }
    if cfg.shrink {
        let mut ms: Vec<Mismatch> = rep
            .examples
            .iter()
            .filter(|m| m.kind != "oracle-error")
            .cloned()
            .collect();
        if !ms.is_empty() {
            shrink_all(spec, img, &prep, cfg, &mut ms)?;
            for m in ms {
                if let Some(e) = rep.examples.iter_mut().find(|e| e.index == m.index) {
                    e.shrunk = m.shrunk;
                }
            }
        }
    }
    if cfg.fixtures {
        for m in &rep.examples {
            if write_fixture(spec, cfg, m).is_ok() {
                rep.fixtures_written += 1;
            }
        }
    }
    rep.elapsed_s = t0.elapsed().as_secs_f64();
    Ok(rep)
}

pub fn print_report(spec: &Spec, rep: &Report) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{} @ {:#x}  route={}  backend={}  cases={} ({} edge + {} generated)  {:.1}s",
        rep.name,
        rep.addr,
        rep.route,
        rep.backend,
        rep.total(),
        rep.edge_cases,
        rep.generated,
        rep.elapsed_s
    );
    let _ = writeln!(
        s,
        "  scan: {}; MXCSR.FZ workaround {}",
        rep.hazards,
        if rep.ftz_workaround { "on" } else { "off" }
    );
    let authoritative = match spec.route {
        Route::L2 { reason } => {
            let _ = writeln!(s, "  routed to L2: {reason}");
            if rep.backend == "Unicorn" {
                let _ = writeln!(
                    s,
                    "  NOTE: Unicorn verdict below is NOT authoritative for this function"
                );
                false
            } else {
                let _ = writeln!(
                    s,
                    "  interim authority until L2 (INF-8): the real CPU (native backend)"
                );
                true
            }
        }
        Route::L1 => true,
    };
    let _ = writeln!(
        s,
        "  mismatches={}  hangs (original never returns, port agrees)={}  st0 beyond f32 differs={}{}",
        rep.mismatches,
        rep.hangs,
        rep.st0_ext_only,
        if spec.st0_exact {
            " (st0 compared exactly)"
        } else {
            " (allowed: spec compares the f32 only)"
        }
    );
    if rep.snan_routed > 0 || rep.unverified > 0 {
        let _ = writeln!(
            s,
            "  signalling-NaN inputs decided by the CPU={}  unverified={}",
            rep.snan_routed, rep.unverified
        );
    }
    if rep.both > 0 {
        let _ = writeln!(
            s,
            "  unicorn vs real CPU on {} cases: {} differ in observable fields {:?}, {} only in C1/exception flags",
            rep.both, rep.unicorn_vs_cpu, rep.unicorn_vs_cpu_kinds, rep.unicorn_vs_cpu_flags_only
        );
    }
    for (k, n) in &rep.kinds {
        let _ = writeln!(s, "  kind {k}: {n}");
    }
    for m in &rep.examples {
        let _ = writeln!(
            s,
            "  case {} [{}]: {}",
            m.index,
            fmt_inputs(spec, &m.input),
            m.kind
        );
        let _ = writeln!(s, "    {}", m.detail);
        if let Some((i, d)) = &m.shrunk {
            let _ = writeln!(s, "    minimal: [{}] {}", fmt_inputs(spec, i), d);
        }
    }
    if rep.fixtures_written > 0 {
        let _ = writeln!(
            s,
            "  fixtures: {} written to fixtures/l1/{}/",
            rep.fixtures_written, rep.key
        );
    }
    let verdict = match (rep.mismatches == 0, rep.unverified == 0, authoritative) {
        (_, _, false) => "NOT AUTHORITATIVE",
        (true, true, true) => "PASS",
        (true, false, true) => "INCOMPLETE",
        (false, _, true) => "FAIL",
    };
    let _ = writeln!(s, "  {verdict}");
    s
}
