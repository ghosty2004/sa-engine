//! Per-frame phase timers and counters (PERF-0, docs/perf/measure.md).
//!
//! The mechanism only: the embedder names its phases and counters ([`Layout`]), opens [`scope`]s around the work it
//! measures, adds to counters ([`add`]) and closes one row per frame ([`end_frame`]). Phase times are *exclusive*:
//! while a scope is open its phase is charged and the enclosing one is not, so the phases of a row add up to the
//! wall time since the previous row (phase 0 takes whatever no scope covers). Columns filled after their frame
//! (GPU timestamps arrive frames later) are [`Layout::late`] columns, set with [`set_late`].
//!
//! Zero cost when off: every entry point first reads one relaxed atomic flag. The state is per thread and only the
//! thread that called [`enable`] (the game's main thread) measures; calls on other threads do nothing.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::clock::now_ns;

/// The columns of a measurement: phase names (index 0 = the time no scope covers), counter names, late columns.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub phases: &'static [&'static str],
    pub counters: &'static [&'static str],
    pub late: &'static [&'static str],
}

/// One frame.
struct Row {
    tag: u64,
    total_ns: u64,
    phase_ns: Vec<u64>,
    counters: Vec<u64>,
    late: Vec<Option<f64>>,
}

struct State {
    layout: Layout,
    /// the open scopes (phase indices); `stack[0]` is phase 0
    stack: Vec<usize>,
    /// the time of the last scope change
    last: u64,
    /// the start of the current frame
    start: u64,
    acc: Vec<u64>,
    cnt: Vec<u64>,
    rows: Vec<Row>,
}

impl State {
    /// Charges the time since the last change to the innermost open phase.
    fn charge(&mut self, now: u64) {
        let top = self.stack.last().copied().unwrap_or(0);
        self.acc[top] += now.saturating_sub(self.last);
        self.last = now;
    }

    fn reset(&mut self, now: u64) {
        self.stack.truncate(1);
        self.last = now;
        self.start = now;
        self.acc.iter_mut().for_each(|a| *a = 0);
        self.cnt.iter_mut().for_each(|c| *c = 0);
    }
}

static ENABLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Whether a measurement runs (on some thread).
#[inline(always)]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Starts measuring on this thread with `layout`; the first frame begins now.
pub fn enable(layout: Layout) {
    let now = now_ns();
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            layout,
            stack: vec![0],
            last: now,
            start: now,
            acc: vec![0; layout.phases.len()],
            cnt: vec![0; layout.counters.len()],
            rows: Vec::new(),
        })
    });
    ENABLED.store(true, Ordering::SeqCst);
}

/// Stops measuring (the rows stay readable on this thread).
pub fn disable() {
    ENABLED.store(false, Ordering::SeqCst);
}

#[inline]
fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    if !enabled() {
        return None;
    }
    STATE.with(|s| s.try_borrow_mut().ok().and_then(|mut s| s.as_mut().map(f)))
}

/// An open phase; closed when dropped (also while unwinding).
#[must_use = "the phase is closed when the guard is dropped"]
pub struct Scope(usize);

impl Drop for Scope {
    #[inline]
    fn drop(&mut self) {
        if self.0 == 0 {
            return;
        }
        let depth = self.0;
        with(|s| {
            // a frame boundary (or an iteration left without unwinding, wasm32) may have reset the stack meanwhile
            if s.stack.len() >= depth {
                s.charge(now_ns());
                s.stack.truncate(depth - 1);
            }
        });
    }
}

/// Opens phase `phase` until the guard is dropped.
#[inline]
pub fn scope(phase: usize) -> Scope {
    Scope(
        with(|s| {
            s.charge(now_ns());
            s.stack.push(phase);
            s.stack.len()
        })
        .unwrap_or(0),
    )
}

/// The innermost open phase becomes `to` if it is `from` (a phase boundary inside one call, e.g. game logic → render
/// preparation inside one frame function).
pub fn switch_top(from: usize, to: usize) {
    with(|s| {
        if s.stack.last() == Some(&from) && s.stack.len() > 1 {
            s.charge(now_ns());
            if let Some(t) = s.stack.last_mut() {
                *t = to;
            }
        }
    });
}

/// Adds `n` to counter `c` of the current frame.
#[inline]
pub fn add(c: usize, n: u64) {
    with(|s| {
        if let Some(v) = s.cnt.get_mut(c) {
            *v += n;
        }
    });
}

/// Drops what the current frame accumulated (it is not a measured frame) and starts a new one now.
pub fn discard() {
    with(|s| s.reset(now_ns()));
}

/// Closes the current frame as a row tagged `tag` (e.g. the main-loop iteration), with `counters` (index, value)
/// added to its counters, and starts the next one. Returns the row's index.
pub fn end_frame(tag: u64, counters: &[(usize, u64)]) -> Option<usize> {
    with(|s| {
        let now = now_ns();
        s.charge(now);
        for &(c, v) in counters {
            if let Some(x) = s.cnt.get_mut(c) {
                *x += v;
            }
        }
        s.rows.push(Row {
            tag,
            total_ns: now.saturating_sub(s.start),
            phase_ns: s.acc.clone(),
            counters: s.cnt.clone(),
            late: vec![None; s.layout.late.len()],
        });
        let i = s.rows.len() - 1;
        s.reset(now);
        i
    })
}

/// Sets late column `col` of row `row`.
pub fn set_late(row: usize, col: usize, v: f64) {
    with(|s| {
        if let Some(x) = s.rows.get_mut(row).and_then(|r| r.late.get_mut(col)) {
            *x = Some(v);
        }
    });
}

/// Rows recorded so far.
pub fn rows() -> usize {
    STATE.with(|s| s.borrow().as_ref().map_or(0, |s| s.rows.len()))
}

fn ms(ns: u64) -> f64 {
    ns as f64 / 1e6
}

/// The rows as CSV: `frame,tag,total_ms,<phase>_ms...,<counter>...,<late>...` (late columns empty when missing).
pub fn csv() -> String {
    STATE.with(|s| {
        let s = s.borrow();
        let Some(s) = s.as_ref() else { return String::new() };
        let mut out = String::from("frame,tag,total_ms");
        for p in s.layout.phases {
            let _ = write!(out, ",{p}_ms");
        }
        for c in s.layout.counters.iter().chain(s.layout.late) {
            let _ = write!(out, ",{c}");
        }
        out.push('\n');
        for (i, r) in s.rows.iter().enumerate() {
            let _ = write!(out, "{i},{},{:.4}", r.tag, ms(r.total_ns));
            for p in &r.phase_ns {
                let _ = write!(out, ",{:.4}", ms(*p));
            }
            for c in &r.counters {
                let _ = write!(out, ",{c}");
            }
            for l in &r.late {
                match l {
                    Some(v) => {
                        let _ = write!(out, ",{v:.4}");
                    }
                    None => out.push(','),
                }
            }
            out.push('\n');
        }
        out
    })
}

/// The distribution of a column.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dist {
    pub mean: f64,
    pub median: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

/// The distribution of `v` (sorted in place; nearest-rank percentiles).
pub fn stats(v: &mut [f64]) -> Dist {
    if v.is_empty() {
        return Dist::default();
    }
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    v.sort_by(f64::total_cmp);
    let pct = |p: f64| v[((v.len() - 1) as f64 * p).round() as usize];
    Dist {
        mean,
        median: pct(0.5),
        p95: pct(0.95),
        p99: pct(0.99),
        max: v[v.len() - 1],
    }
}

/// The end-of-run table: per phase (and late column) mean / median / p95 / p99 / max in ms and the share of the mean
/// frame, phases by mean; per counter mean / median / p95 / max per frame.
pub fn summary() -> String {
    STATE.with(|s| {
        let s = s.borrow();
        let Some(s) = s.as_ref() else { return String::new() };
        let n = s.rows.len();
        let mut out = String::new();
        if n == 0 {
            out.push_str("perf: no frames recorded\n");
            return out;
        }
        let mut total: Vec<f64> = s.rows.iter().map(|r| ms(r.total_ns)).collect();
        let t = stats(&mut total);
        let _ = writeln!(
            out,
            "perf: {n} frames, frame time mean {:.2} ms ({:.1} fps), median {:.2}, p95 {:.2}, p99 {:.2}, max {:.2} ms",
            t.mean,
            1000.0 / t.mean.max(1e-9),
            t.median,
            t.p95,
            t.p99,
            t.max
        );
        let _ = writeln!(out, "  {:<22} {:>9} {:>9} {:>9} {:>9} {:>9} {:>7}", "phase (ms)", "mean", "median", "p95", "p99", "max", "share");
        let mut phases: Vec<(usize, Dist)> = (0..s.layout.phases.len())
            .map(|i| {
                let mut v: Vec<f64> = s.rows.iter().map(|r| ms(r.phase_ns[i])).collect();
                (i, stats(&mut v))
            })
            .collect();
        phases.sort_by(|a, b| b.1.mean.total_cmp(&a.1.mean));
        for (i, p) in &phases {
            let _ = writeln!(
                out,
                "  {:<22} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>6.1}%",
                s.layout.phases[*i],
                p.mean,
                p.median,
                p.p95,
                p.p99,
                p.max,
                100.0 * p.mean / t.mean.max(1e-9)
            );
        }
        for (i, name) in s.layout.late.iter().enumerate() {
            let mut v: Vec<f64> = s.rows.iter().filter_map(|r| r.late[i]).collect();
            if v.is_empty() {
                let _ = writeln!(out, "  {name:<22} (not available)");
                continue;
            }
            let k = v.len();
            let p = stats(&mut v);
            let _ = writeln!(
                out,
                "  {name:<22} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3}  ({k} of {n} frames; not part of the sum)",
                p.mean, p.median, p.p95, p.p99, p.max
            );
        }
        let _ = writeln!(out, "  {:<22} {:>12} {:>12} {:>12} {:>12}", "counter (per frame)", "mean", "median", "p95", "max");
        for (i, name) in s.layout.counters.iter().enumerate() {
            let mut v: Vec<f64> = s.rows.iter().map(|r| r.counters[i] as f64).collect();
            let p = stats(&mut v);
            let _ = writeln!(out, "  {name:<22} {:>12.1} {:>12.0} {:>12.0} {:>12.0}", p.mean, p.median, p.p95, p.max);
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: Layout = Layout {
        phases: &["other", "a", "b"],
        counters: &["n"],
        late: &["gpu"],
    };

    #[test]
    fn exclusive_phases_add_up() {
        enable(L);
        {
            let _a = scope(1);
            std::thread::sleep(std::time::Duration::from_millis(3));
            {
                let _b = scope(2);
                std::thread::sleep(std::time::Duration::from_millis(3));
                add(0, 5);
            }
            switch_top(1, 2);
        }
        let r = end_frame(7, &[(0, 1)]).unwrap();
        set_late(r, 0, 1.5);
        let row = STATE.with(|s| {
            let s = s.borrow();
            let r = &s.as_ref().unwrap().rows[0];
            (r.total_ns, r.phase_ns.clone(), r.counters.clone(), r.late.clone())
        });
        assert_eq!(row.1.iter().sum::<u64>(), row.0);
        assert!(row.1[1] >= 3_000_000 && row.1[2] >= 3_000_000);
        assert_eq!(row.2, vec![6]);
        assert_eq!(row.3, vec![Some(1.5)]);
        // a scope left open (an iteration left without unwinding) does not break the next frame
        std::mem::forget(scope(1));
        discard();
        {
            let _b = scope(2);
        }
        end_frame(8, &[]);
        let c = csv();
        assert!(c.starts_with("frame,tag,total_ms,other_ms,a_ms,b_ms,n,gpu\n"));
        assert_eq!(c.lines().count(), 3);
        assert!(summary().contains("2 frames"));
        disable();
        assert!(scope(1).0 == 0);
    }
}
