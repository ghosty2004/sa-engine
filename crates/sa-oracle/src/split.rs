//! Split-run L2 report (INF-17): compares a split-run spool recorded in the plain run (every sampled
//! call ran the ORIGINAL once, `SA_L2_SPLIT=orig`) with one recorded in the rust run (the Rust
//! implementations, `SA_L2_SPLIT=rust`), call by call.
//!
//! Alignment: cases are paired by (function, call ordinal). Determinism mode makes both runs identical
//! up to the first call whose outputs differ, so a pair is only compared when its INPUTS are identical:
//! frame, caller, stack arguments, the registers the calling convention passes (ECX for thiscall, ECX/EDX
//! for fastcall), x87 control word, MXCSR control bits and every pre-call region that is read (spec
//! globals, extras, `this`, pointed-to arguments, pre-call OS probes). The first pair of a function
//! whose inputs differ ends its comparison (`DIVERGED`, with the differing field); the earliest such
//! pair over all functions (in call order) is reported separately, because every later difference may
//! follow from it. Other registers at entry (scratch values the callee does not read) are listed as
//! soft differences. The hash of every 4 KiB page of the static watched windows (.data/.bss, _TEXT_HA,
//! _rwdseg) fingerprints the state outside the declared read set: a pair whose fingerprint differs is
//! still compared, and the function's verdict is `PASS-FP` instead of `PASS` (pages listed).
//!
//! Outputs are compared with the same rules as the in-process L2 (`sa_oracle_format::l2::compare`, the
//! one reviewed normaliser) on a merged case: the plain run's record with the rust run's outcome and
//! post-call region bytes. Every byte range the normaliser excludes (original uninitialised data) is
//! listed with both values when it actually differed (`excluded:` lines).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use sa_oracle_format::l2::{self as fmt, Case, RegionKind};
use sa_oracle_format::normalise;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FnResult {
    pub addr: u32,
    pub name: String,
    pub plain_cases: usize,
    pub rust_cases: usize,
    /// Pairs with identical inputs whose outputs were compared.
    pub compared: usize,
    pub mismatches: usize,
    pub undeclared_cases: usize,
    /// Undeclared write ranges (addr, len) -> pairs (spec / l2-extras.txt completeness, not a port verdict).
    pub undeclared: BTreeMap<(u32, u32), usize>,
    pub first_mismatch: Option<String>,
    /// First pair whose inputs differ: (ordinal, frame, what).
    pub diverged: Option<(u32, u32, String)>,
    pub soft_input_diffs: usize,
    /// Compared pairs whose page fingerprint differed (undeclared state differed; declared inputs
    /// equal): pairs, and the pages seen (page -> pairs).
    pub fingerprint_pairs: usize,
    pub fingerprint_pages: BTreeMap<u32, usize>,
    pub first_fingerprint: Option<(u32, u32)>,
    /// Excluded byte ranges that differed: rule -> uses.
    pub exclusions: BTreeMap<&'static str, usize>,
    pub exclusion_examples: Vec<String>,
    /// Compared pairs whose fingerprint-excluded input bytes (normalise::INPUT_EXCLUSIONS) differed.
    pub volatile_uses: usize,
    /// The same per excluded range (its address): pairs in which it differed.
    pub volatile_addrs: BTreeMap<u32, usize>,
    /// Compared pairs whose stacks were shifted (normalise.rs `Stack` class in use): delta -> pairs.
    pub stack_shifts: BTreeMap<i64, usize>,
    /// Uses of the `Foreign` class (OS object addresses differing raw): location -> pairs.
    pub foreign_uses: BTreeMap<String, usize>,
    /// Watched-window words holding OS object addresses that differed between the runs (masked in the
    /// fingerprint; distinct addresses).
    pub foreign_words: std::collections::BTreeSet<u32>,
}

impl FnResult {
    pub fn verdict(&self) -> &'static str {
        if self.plain_cases == 0 && self.rust_cases == 0 {
            "NOCASES"
        } else if self.mismatches > 0 {
            "FAIL"
        } else if self.diverged.is_some() {
            "DIVERGED"
        } else if self.plain_cases != self.rust_cases {
            "COUNT"
        } else if self.fingerprint_pairs > 0 {
            "PASS-FP"
        } else {
            "PASS"
        }
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub fns: Vec<FnResult>,
    /// Earliest input divergence over all functions (plain-run call order): text.
    pub first_divergence: Option<String>,
    pub trailers: Vec<String>,
    pub errors: Vec<String>,
    /// Every pair whose inputs differ, in call order: (seq, text) (diagnostic `--timeline`).
    pub timeline: Vec<(u32, String)>,
    /// Examples of differing fingerprint-excluded input bytes per function.
    pub volatile_rules: BTreeMap<u32, Vec<String>>,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Normaliser contexts for comparing INPUTS of a pair (no allocation class: the outcome's
/// allocations say nothing about values before the call).
fn input_ctx(a: &Case, b: &Case) -> (normalise::NormCtx, normalise::NormCtx) {
    let (mut x, _) = a.norm_ctx();
    let (mut y, _) = b.norm_ctx();
    x.allocs.clear();
    y.stack = b.stack;
    y.allocs.clear();
    (x, y)
}

/// Stack shift of the rust run against the plain run (`a.esp - b.esp`), 0 without stack info.
pub fn stack_delta(a: &Case, b: &Case) -> u32 {
    if a.esp == 0 || b.esp == 0 {
        0
    } else {
        a.esp.wrapping_sub(b.esp)
    }
}

/// `v` of the rust run moved to the plain run's stack layout when it is a stack address.
fn rebase(b: &Case, delta: u32, v: u32) -> u32 {
    if delta != 0 && v >= b.stack.0 && v < b.stack.1 {
        v.wrapping_add(delta)
    } else {
        v
    }
}

/// `b` (at `addr`) with the reviewed volatile input bytes (normalise::INPUT_EXCLUSIONS) zeroed; the raw
/// values are compared and reported apart (`Case::volatile`).
fn mask_volatile(addr: u32, b: &[u8]) -> Vec<u8> {
    normalise::fingerprint_bytes(addr, b).into_owned()
}

/// Static windows of a case: the windows that the page hashes cover.
fn is_static_window(c: &Case, w: (u32, u32)) -> bool {
    c.page_hashes.iter().any(|&(p, _)| p >= w.0 && p < w.1)
}

/// The first strict input difference of a pair, plus whether soft fields (other entry registers)
/// differ.
pub fn input_diff(a: &Case, b: &Case) -> (Option<String>, bool) {
    if a.frame != b.frame {
        return (Some(format!("frame: plain {} rust {}", a.frame, b.frame)), false);
    }
    // A return address outside the exe (a Rust implementation or a Wine/D3D frame between the caller
    // and the hook) belongs to the call path, not the call: soft. Two game addresses must match.
    let in_image = |x: u32| (a.image.0..a.image.1).contains(&x);
    let caller_soft = a.caller != b.caller && !(in_image(a.caller) && in_image(b.caller));
    if a.caller != b.caller && !caller_soft {
        return (
            Some(format!("caller: plain {:#x} rust {:#x}", a.caller, b.caller)),
            false,
        );
    }
    let (ca, cb) = input_ctx(a, b);
    let delta = stack_delta(a, b);
    if a.args.len() != b.args.len()
        || a.args
            .iter()
            .zip(&b.args)
            .any(|(&x, &y)| !normalise::words_equal(x, &ca, y, &cb))
    {
        return (
            Some(format!("args: plain {:x?} rust {:x?}", a.args, b.args)),
            false,
        );
    }
    let used: &[usize] = match a.conv {
        2 => &[1],    // thiscall: ECX
        3 => &[1, 2], // fastcall: ECX, EDX
        _ => &[],
    };
    for &r in used {
        if !normalise::words_equal(a.regs_in[r], &ca, b.regs_in[r], &cb) {
            return (
                Some(format!(
                    "{}: plain {:#x} rust {:#x}",
                    fmt::REG_NAMES[r],
                    a.regs_in[r],
                    b.regs_in[r]
                )),
                false,
            );
        }
    }
    if a.fpu_env.get(..2) != b.fpu_env.get(..2) {
        return (Some("x87 control word at entry".into()), false);
    }
    if a.mxcsr & 0xFFC0 != b.mxcsr & 0xFFC0 {
        return (
            Some(format!("mxcsr: plain {:#x} rust {:#x}", a.mxcsr, b.mxcsr)),
            false,
        );
    }
    if a.regions.len() != b.regions.len() {
        return (
            Some(format!(
                "region count: plain {} rust {}",
                a.regions.len(),
                b.regions.len()
            )),
            false,
        );
    }
    for (x, y) in a.regions.iter().zip(&b.regions) {
        // A region whose address was read from game memory (`[0x8E3FFC]`, an extra region) can be an
        // OS-allocated object (the CdStream queue buffer `[0x8E3FEC]`, LocalAlloc'd): two such addresses are the same object
        // through the `Foreign` class (as for post regions in `merge`); its bytes are still compared.
        if x.kind != y.kind
            || (x.has_pre_memory()
                && x.addr != rebase(b, delta, y.addr)
                && !normalise::words_equal(x.addr, &ca, y.addr, &cb))
        {
            return (
                Some(format!(
                    "region {:?} {:#x} vs {:?} {:#x}",
                    x.kind, x.addr, y.kind, y.addr
                )),
                false,
            );
        }
        let pre_differs = match x.kind {
            RegionKind::Os(_) => x.pre != y.pre,
            // A write-only region's bytes before the call are not an input (caller's scratch buffer).
            _ if x.access == fmt::ACC_W => false,
            _ => {
                let (p, q) = (mask_volatile(x.addr, &x.pre), mask_volatile(x.addr, &y.pre));
                normalise::bytes_equal(x.addr, &p, &ca, &q, &cb).is_some()
            }
        };
        if x.readable != y.readable || pre_differs {
            let what = match x.kind {
                RegionKind::Os(p) => format!(
                    "OS probe before the call: plain {} rust {}",
                    fmt::describe_os(p, &x.pre),
                    fmt::describe_os(p, &y.pre)
                ),
                _ => {
                    let i = x.pre.iter().zip(&y.pre).position(|(p, q)| p != q).unwrap_or(0);
                    format!(
                        "region {:?} {:#x} before the call, first difference at {:#x}: plain {} rust {}",
                        x.kind,
                        x.addr,
                        x.addr as u64 + i as u64,
                        hex(&x.pre[i..(i + 8).min(x.pre.len())]),
                        hex(&y.pre[i..(i + 8).min(y.pre.len())])
                    )
                }
            };
            return (Some(what), false);
        }
    }
    // (the page fingerprint is checked apart: `page_diff`; it does not stop the comparison)
    // Window bytes both runs wrote: pre values in the static windows must agree (covered by the page
    // hashes; checked again byte-wise because they are what the comparison uses).
    let pre_b: BTreeMap<u32, u8> = b
        .window_pre
        .iter()
        .flat_map(|w| {
            w.bytes
                .iter()
                .enumerate()
                .map(move |(k, &v)| (rebase(b, delta, w.addr + k as u32), v))
        })
        .collect();
    let pre_a: BTreeMap<u32, u8> = a
        .window_pre
        .iter()
        .flat_map(|w| {
            w.bytes
                .iter()
                .enumerate()
                .map(move |(k, &v)| (w.addr + k as u32, v))
        })
        .collect();
    for (&at, &v) in &pre_a {
        let st = a
            .windows
            .iter()
            .any(|&r| at >= r.0 && at < r.1 && is_static_window(a, r));
        let Some(&u) = pre_b.get(&at) else { continue };
        if !st
            || u == v
            || normalise::INPUT_EXCLUSIONS
                .iter()
                .any(|e| at.wrapping_sub(e.addr) < e.len)
        {
            continue;
        }
        // The aligned word, when both runs have all four bytes: compared through the normaliser
        // (OS object addresses, stack addresses).
        let ws = at & !3;
        let word = |m: &BTreeMap<u32, u8>| -> Option<u32> {
            Some(u32::from_le_bytes([
                m.get(&ws).copied()?,
                m.get(&(ws + 1)).copied()?,
                m.get(&(ws + 2)).copied()?,
                m.get(&(ws + 3)).copied()?,
            ]))
        };
        if let (Some(x), Some(y)) = (word(&pre_a), word(&pre_b))
            && normalise::words_equal(x, &ca, y, &cb)
        {
            continue;
        }
        return (
            Some(format!(
                "window byte {at:#x} before the call: plain {v:#04x} rust {u:#04x}"
            )),
            false,
        );
    }
    let soft = caller_soft
        || a.regs_in
            .iter()
            .zip(&b.regs_in)
            .any(|(&x, &y)| !normalise::words_equal(x, &ca, y, &cb));
    (None, soft)
}

/// Pages of the static watched windows whose fingerprint differs before the call (state outside the
/// declared read set), or None.
pub fn page_diff(a: &Case, b: &Case) -> Option<Vec<u32>> {
    if a.page_hashes == b.page_hashes {
        return None;
    }
    if a.page_hashes.len() != b.page_hashes.len() {
        return Some(vec![u32::MAX]);
    }
    Some(
        a.page_hashes
            .iter()
            .zip(&b.page_hashes)
            .filter(|(p, q)| p != q)
            .map(|(p, _)| p.0)
            .collect(),
    )
}

/// The plain run's record with the rust run's outputs (the case `compare` understands). Post-region
/// addresses that differ are returned as an extra difference.
pub fn merge(a: &Case, b: &Case) -> (Case, Option<String>) {
    let delta = stack_delta(a, b);
    let mut c = a.clone();
    c.leave = 0;
    c.merged = true;
    // Callee-saved registers whose entry values differ (a Rust ancestor keeps its own frame pointer
    // in EBP where the original used EBP as a general register): compare "preserved" with "preserved".
    let mut rust_out = b.orig.clone();
    for k in 3..7 {
        if a.regs_in[k] != b.regs_in[k] && a.orig.regs[k] == a.regs_in[k] && rust_out.regs[k] == b.regs_in[k]
        {
            rust_out.regs[k] = a.orig.regs[k];
        }
    }
    c.esp_rust = b.esp;
    c.foreign_rust = b.foreign.clone();
    if b.esp != 0 {
        c.stack = (a.stack.0.min(b.stack.0), a.stack.1.max(b.stack.1));
    }
    c.rust = rust_out;
    // The rust run's stack-located writes, moved to the plain run's stack layout.
    for w in c.rust.writes.iter_mut() {
        w.addr = rebase(b, delta, w.addr);
    }
    let mut post_addr = None;
    let (ca, cb) = (a.norm_ctx().0, b.norm_ctx().0);
    for (x, y) in c.regions.iter_mut().zip(&b.regions) {
        x.post_rust = y.post_orig.clone();
        if x.kind == RegionKind::Post
            && x.addr != rebase(b, delta, y.addr)
            && !normalise::words_equal(x.addr, &ca, y.addr, &cb)
            && post_addr.is_none()
        {
            post_addr = Some(format!(
                "post region address: plain {:#x} rust {:#x} (different object)",
                x.addr, y.addr
            ));
        }
    }
    // each run's bytes before the call (the rust run's moved to the plain run's stack layout)
    c.window_pre = a.window_pre.clone();
    c.window_pre_rust = b
        .window_pre
        .iter()
        .map(|w| fmt::Write {
            addr: rebase(b, delta, w.addr),
            bytes: w.bytes.clone(),
        })
        .collect();
    (c, post_addr)
}

/// Words that differ raw but are equal through the `Foreign` class (OS object addresses): where.
fn foreign_uses(a: &Case, b: &Case, m: &Case) -> Vec<String> {
    let (ia, ib) = input_ctx(a, b);
    let (oa, ob) = m.norm_ctx();
    let mut out = Vec::new();
    let mut word = |what: String, x: u32, cx: &normalise::NormCtx, y: u32, cy: &normalise::NormCtx| {
        if x != y
            && cx.normalise(x) == normalise::Norm::Foreign
            && cy.normalise(y) == normalise::Norm::Foreign
        {
            out.push(what);
        }
    };
    for (k, (&x, &y)) in a.args.iter().zip(&b.args).enumerate() {
        word(format!("arg{k}"), x, &ia, y, &ib);
    }
    for k in 0..7 {
        word(
            format!("{} in", fmt::REG_NAMES[k]),
            a.regs_in[k],
            &ia,
            b.regs_in[k],
            &ib,
        );
        word(
            format!("{} out", fmt::REG_NAMES[k]),
            m.orig.regs[k],
            &oa,
            m.rust.regs[k],
            &ob,
        );
    }
    for (x, y) in a.regions.iter().zip(&b.regions) {
        if matches!(x.kind, RegionKind::Os(_)) {
            continue;
        }
        word(format!("region address {:#x}", x.addr), x.addr, &ia, y.addr, &ib);
        let o = ((4 - (x.addr & 3)) & 3) as usize;
        let at = |k: usize| x.addr.wrapping_add((o + 4 * k) as u32);
        let w = |v: &[u8], k: usize| {
            let i = o + 4 * k;
            v.get(i..i + 4)
                .map(|q| u32::from_le_bytes([q[0], q[1], q[2], q[3]]))
        };
        let n = x.pre.len().max(x.post_orig.len()) / 4 + 1;
        for k in 0..n {
            if let (Some(p), Some(q)) = (w(&x.pre, k), w(&y.pre, k)) {
                word(format!("{:#x} before", at(k)), p, &ia, q, &ib);
            }
            if let (Some(p), Some(q)) = (w(&x.post_orig, k), w(&y.post_orig, k)) {
                word(format!("{:#x} after", at(k)), p, &oa, q, &ob);
            }
        }
    }
    out
}

/// Excluded ranges of a merged case whose bytes actually differ between the runs.
fn exclusion_uses(c: &Case) -> Vec<(normalise::Exclusion, String, String)> {
    let mut out = Vec::new();
    for e in normalise::exclusions(c) {
        let get = |orig: bool| -> Option<Vec<u8>> {
            for r in c.regions.iter().filter(|r| r.readable && r.extent() > 0) {
                let b = if orig { &r.post_orig } else { &r.post_rust };
                if e.addr >= r.addr && (e.addr - r.addr) as u64 + e.len as u64 <= b.len() as u64 {
                    let o = (e.addr - r.addr) as usize;
                    return Some(b[o..o + e.len as usize].to_vec());
                }
            }
            None
        };
        if let (Some(x), Some(y)) = (get(true), get(false))
            && x != y
        {
            out.push((e, hex(&x), hex(&y)));
        }
    }
    out
}

pub fn split_report(plain: &[Case], rust: &[Case]) -> Report {
    let mut rep = Report::default();
    for (what, v, want) in [
        ("plain", plain, fmt::LEAVE_SPLIT_ORIG),
        ("rust", rust, fmt::LEAVE_SPLIT_RUST),
    ] {
        if let Some(c) = v.iter().find(|c| c.leave != want) {
            rep.errors.push(format!(
                "{what} spool: {} frame {} is not a split-run record of the {} (leave {})",
                c.name,
                c.frame,
                if want == fmt::LEAVE_SPLIT_ORIG {
                    "original"
                } else {
                    "Rust implementation"
                },
                c.leave
            ));
        }
    }
    let key = |c: &Case| (c.fn_addr, c.ordinal);
    let rmap: BTreeMap<(u32, u32), &Case> = rust.iter().map(|c| (key(c), c)).collect();
    let mut by_fn: BTreeMap<u32, FnResult> = BTreeMap::new();
    for c in plain.iter().chain(rust) {
        let e = by_fn.entry(c.fn_addr).or_insert_with(|| FnResult {
            addr: c.fn_addr,
            name: c.name.clone(),
            ..Default::default()
        });
        if c.leave == fmt::LEAVE_SPLIT_ORIG {
            e.plain_cases += 1;
        } else {
            e.rust_cases += 1;
        }
    }
    let mut ordered: Vec<&Case> = plain.iter().collect();
    ordered.sort_by_key(|c| c.seq);
    let mut first_div: Option<(u32, String)> = None;
    for a in ordered {
        let Some(b) = rmap.get(&key(a)) else {
            continue;
        };
        if by_fn[&a.fn_addr].diverged.is_some() {
            if let (Some(d), _) = input_diff(a, b) {
                rep.timeline.push((
                    a.seq,
                    format!(
                        "{} ordinal {} frame {} caller {:#x}: {d}",
                        a.name, a.ordinal, a.frame, a.caller
                    ),
                ));
            }
            continue;
        }
        let e = by_fn.get_mut(&a.fn_addr).expect("counted above");
        let (d, soft) = input_diff(a, b);
        if let Some(d) = &d {
            rep.timeline.push((
                a.seq,
                format!(
                    "{} ordinal {} frame {} caller {:#x}: {d}",
                    a.name, a.ordinal, a.frame, a.caller
                ),
            ));
        }
        if let Some(d) = d {
            if first_div.as_ref().is_none_or(|f| a.seq < f.0) {
                first_div = Some((
                    a.seq,
                    format!(
                        "seq {} {} ordinal {} frame {} caller {:#x}: {d}",
                        a.seq, a.name, a.ordinal, a.frame, a.caller
                    ),
                ));
            }
            e.diverged = Some((a.ordinal, a.frame, d));
            continue;
        }
        e.soft_input_diffs += soft as usize;
        if let Some(pages) = page_diff(a, b) {
            e.fingerprint_pairs += 1;
            if e.first_fingerprint.is_none() {
                e.first_fingerprint = Some((a.ordinal, a.frame));
            }
            for p in pages {
                *e.fingerprint_pages.entry(p).or_default() += 1;
            }
            rep.timeline.push((
                a.seq,
                format!(
                    "{} ordinal {} frame {} caller {:#x}: fingerprint only: pages {:x?}",
                    a.name,
                    a.ordinal,
                    a.frame,
                    a.caller,
                    e.fingerprint_pages.keys().take(8).collect::<Vec<_>>()
                ),
            ));
        }
        let d = stack_delta(a, b);
        if d != 0 {
            *e.stack_shifts.entry(d as i32 as i64).or_default() += 1;
        }
        if a.volatile != b.volatile {
            e.volatile_uses += 1;
            let r = rep.volatile_rules.entry(a.fn_addr).or_default();
            for (x, y) in a.volatile.iter().zip(&b.volatile) {
                if x != y {
                    *e.volatile_addrs.entry(x.addr).or_default() += 1;
                }
                if x != y && r.len() < 2 {
                    r.push(format!(
                        "{:#x}: plain {} rust {}",
                        x.addr,
                        hex(&x.bytes),
                        hex(&y.bytes)
                    ));
                }
            }
        }
        let (m, post_addr) = merge(a, b);
        for u in foreign_uses(a, b, &m) {
            *e.foreign_uses.entry(u).or_default() += 1;
        }
        let fw = |c: &Case| -> BTreeMap<u32, u32> {
            c.foreign_words
                .chunks(2)
                .filter(|p| p.len() == 2)
                .map(|p| (p[0], p[1]))
                .collect()
        };
        let (fa, fb) = (fw(a), fw(b));
        for (addr, va) in &fa {
            if fb.get(addr) != Some(va) {
                e.foreign_words.insert(*addr);
            }
        }
        for addr in fb.keys() {
            if !fa.contains_key(addr) {
                e.foreign_words.insert(*addr);
            }
        }
        let mut diffs: Vec<String> = fmt::compare(&m)
            .iter()
            .map(|x| format!("{}: {}", x.field, x.detail))
            .collect();
        if let Some(p) = post_addr {
            diffs.insert(0, p);
        }
        e.compared += 1;
        if !diffs.is_empty() {
            e.mismatches += 1;
            if e.first_mismatch.is_none() {
                e.first_mismatch = Some(format!(
                    "ordinal {} frame {} caller {:#x} args {:x?}: {}",
                    a.ordinal,
                    a.frame,
                    a.caller,
                    a.args,
                    diffs.join(" | ")
                ));
            }
        }
        let und = fmt::undeclared_writes(&m);
        if !und.is_empty() {
            e.undeclared_cases += 1;
        }
        for u in und {
            *e.undeclared.entry((u.addr, u.len)).or_default() += 1;
        }
        for (x, o, r) in exclusion_uses(&m) {
            *e.exclusions.entry(x.rule).or_default() += 1;
            if e.exclusion_examples.len() < 12 {
                e.exclusion_examples.push(format!(
                    "ordinal {} {:#x}+{}: orig {o} rust {r}",
                    a.ordinal, x.addr, x.len
                ));
            }
        }
    }
    rep.first_divergence = first_div.map(|f| f.1);
    rep.fns = by_fn.into_values().collect();
    rep
}

/// Differing byte ranges of two split-run window dumps (`SA_L2_DUMP_SEQ`): (address, hex A, hex B).
pub fn dump_diff(a: &[u8], b: &[u8]) -> Result<Vec<(u32, String, String)>, String> {
    let parse = |d: &[u8]| -> Result<Vec<(u32, Vec<u8>)>, String> {
        let mut out = Vec::new();
        let mut p = 0usize;
        while p < d.len() {
            let h = d.get(p..p + 8).ok_or("truncated dump")?;
            let start = u32::from_le_bytes([h[0], h[1], h[2], h[3]]);
            let n = u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as usize;
            out.push((start, d.get(p + 8..p + 8 + n).ok_or("truncated dump")?.to_vec()));
            p += 8 + n;
        }
        Ok(out)
    };
    let (wa, wb) = (parse(a)?, parse(b)?);
    let mut out = Vec::new();
    for ((sa, x), (sb, y)) in wa.iter().zip(&wb) {
        if sa != sb || x.len() != y.len() {
            return Err(format!(
                "window layout differs: {sa:#x}+{} vs {sb:#x}+{}",
                x.len(),
                y.len()
            ));
        }
        let mut k = 0;
        while k < x.len() {
            if x[k] == y[k] {
                k += 1;
                continue;
            }
            let s0 = k;
            while k < x.len() && (x[k] != y[k] || (k + 4 < x.len() && x[k..k + 4] != y[k..k + 4])) {
                k += 1;
            }
            out.push((sa + s0 as u32, hex(&x[s0..k]), hex(&y[s0..k])));
        }
    }
    Ok(out)
}

/// Nesting of recorded calls in one split-run spool (spool order = completion order, `seq` = entry
/// order): (outer function, inner function) -> pairs. Functions nested in a listed function run below
/// its implementation (in the rust run: below Rust frames, with Rust's stack and MXCSR); verify them
/// in a run without the outer function.
pub fn nesting(spool: &[Case]) -> BTreeMap<(String, String), usize> {
    let mut out = BTreeMap::new();
    // open cases at each completion: a case nests in every case that entered before it and completes
    // after it.
    let mut done: Vec<(u32, usize)> = spool.iter().enumerate().map(|(i, c)| (c.seq, i)).collect();
    done.sort_unstable();
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (completion index, case index) of open outer cases
    for &(_, i) in &done {
        while stack.last().is_some_and(|&(end, _)| end < i) {
            stack.pop();
        }
        if let Some(&(_, o)) = stack.last() {
            *out.entry((spool[o].name.clone(), spool[i].name.clone()))
                .or_default() += 1;
        }
        stack.push((i, i));
    }
    out
}

/// The `--timeline` listing: every pair whose inputs differ, in call order (at most `max` lines).
pub fn print_timeline(rep: &Report, max: usize) -> String {
    let mut t = rep.timeline.clone();
    t.sort_by_key(|x| x.0);
    let mut s = format!("{} pair(s) with differing inputs\n", t.len());
    for (q, x) in t.iter().take(max) {
        let _ = writeln!(s, "  seq {q}: {x}");
    }
    s
}

pub fn print_report(rep: &Report) -> String {
    let mut s = String::new();
    for e in &rep.errors {
        let _ = writeln!(s, "ERROR {e}");
    }
    for t in &rep.trailers {
        let _ = writeln!(s, "TRAILER {t}");
    }
    let _ = writeln!(
        s,
        "first input divergence (call order): {}",
        rep.first_divergence.as_deref().unwrap_or("none")
    );
    for e in normalise::INPUT_EXCLUSIONS {
        let _ = writeln!(
            s,
            "input fingerprint exclusion (reviewed, normalise.rs) {:#x}+{}: {}",
            e.addr, e.len, e.rule
        );
    }
    for f in &rep.fns {
        let _ = writeln!(
            s,
            "SPLIT {} {:#010x} {} plain_cases={} rust_cases={} compared={} mismatches={} undeclared_cases={} soft_input_diffs={} excluded_uses={} input_exclusion_uses={} fingerprint_pairs={}",
            f.verdict(),
            f.addr,
            f.name,
            f.plain_cases,
            f.rust_cases,
            f.compared,
            f.mismatches,
            f.undeclared_cases,
            f.soft_input_diffs,
            f.exclusions.values().sum::<usize>(),
            f.volatile_uses,
            f.fingerprint_pairs
        );
        if let Some((o, fr)) = f.first_fingerprint {
            let v: Vec<String> = f
                .fingerprint_pages
                .iter()
                .take(10)
                .map(|(p, n)| format!("{p:#x} x{n}"))
                .collect();
            let _ = writeln!(
                s,
                "    undeclared state differed before {} compared pair(s) (first: ordinal {o} frame {fr}); pages: {}",
                f.fingerprint_pairs,
                v.join(", ")
            );
        }
        if !f.stack_shifts.is_empty() {
            let v: Vec<String> = f
                .stack_shifts
                .iter()
                .map(|(d, n)| format!("{d:+#x} in {n} pair(s)"))
                .collect();
            let _ = writeln!(
                s,
                "    stack shifted (rust vs plain; stack pointers compared by offset, normalise.rs Stack): {}",
                v.join(", ")
            );
        }
        if !f.foreign_uses.is_empty() {
            let v: Vec<String> = f
                .foreign_uses
                .iter()
                .take(16)
                .map(|(w, n)| format!("{w} x{n}"))
                .collect();
            let _ = writeln!(
                s,
                "    OS object addresses compared as such (normalise.rs Foreign), {} location(s): {}{}",
                f.foreign_uses.len(),
                v.join(", "),
                if f.foreign_uses.len() > 16 { ", …" } else { "" }
            );
        }
        if !f.foreign_words.is_empty() {
            let v: Vec<String> = f
                .foreign_words
                .iter()
                .take(12)
                .map(|a| format!("{a:#x}"))
                .collect();
            let _ = writeln!(
                s,
                "    window words holding differing OS object addresses (masked in the fingerprint): {} address(es): {}{}",
                f.foreign_words.len(),
                v.join(","),
                if f.foreign_words.len() > 12 { ",…" } else { "" }
            );
        }
        for v in rep.volatile_rules.get(&f.addr).into_iter().flatten() {
            let _ = writeln!(
                s,
                "    input exclusion used (fingerprint-excluded bytes differed): {v}"
            );
        }
        if !f.volatile_addrs.is_empty() {
            let v: Vec<String> = f
                .volatile_addrs
                .iter()
                .map(|(a, n)| format!("{a:#x} x{n}"))
                .collect();
            let _ = writeln!(
                s,
                "    input exclusions that differed (address x pairs): {}",
                v.join(", ")
            );
        }
        if !f.undeclared.is_empty() {
            let v: Vec<String> = f
                .undeclared
                .iter()
                .take(8)
                .map(|((a, n), k)| format!("{a:#x}+{n} x{k}"))
                .collect();
            let _ = writeln!(
                s,
                "    writes outside the declared write set (add to tools/e2e/l2-extras.txt): {}{}",
                v.join(", "),
                if f.undeclared.len() > 8 { ", …" } else { "" }
            );
        }
        if let Some(m) = &f.first_mismatch {
            let _ = writeln!(s, "    first mismatch: {m}");
        }
        if let Some((o, fr, d)) = &f.diverged {
            let _ = writeln!(
                s,
                "    inputs diverge at ordinal {o} (frame {fr}), outputs not compared from there: {d}"
            );
        }
        for (rule, n) in &f.exclusions {
            let _ = writeln!(s, "    excluded ({n} use(s)) by the reviewed normaliser: {rule}");
        }
        for x in &f.exclusion_examples {
            let _ = writeln!(s, "    excluded: {x}");
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_oracle_format::l2::{Outcome, Region, Write};

    fn case(leave: u8, ordinal: u32, seq: u32, eax: u32) -> Case {
        Case {
            fn_addr: 0x538900,
            name: "CFileMgr::OpenFile".into(),
            frame: 1,
            ordinal,
            seq,
            leave,
            args: vec![0x1000, 0x2000],
            regions: vec![Region {
                addr: 0xB71A60,
                kind: RegionKind::Global { exact: true },
                access: 3,
                readable: true,
                pre: vec![1, 2],
                post_orig: vec![1, 3],
                post_rust: vec![],
            }],
            windows: vec![(0x8A4000, 0x8A6000)],
            page_hashes: vec![(0x8A4000, 1), (0x8A5000, 2)],
            orig: Outcome {
                regs: [eax, 0, 0, 4, 5, 6, 7],
                writes: vec![Write {
                    addr: 0x8A4010,
                    bytes: vec![9],
                }],
                ..Outcome::default()
            },
            window_pre: vec![Write {
                addr: 0x8A4010,
                bytes: vec![0],
            }],
            ret: 1,
            ..Case::default()
        }
    }

    #[test]
    fn verdicts() {
        let p = vec![case(2, 0, 0, 0x8E2C30), case(2, 1, 1, 0x8E2C50)];
        let r = vec![case(3, 0, 0, 0x8E2C30), case(3, 1, 1, 0x8E2C50)];
        let rep = split_report(&p, &r);
        assert_eq!(rep.fns[0].verdict(), "PASS");
        assert_eq!(rep.fns[0].compared, 2);
        // an output difference with identical inputs
        let mut r2 = r.clone();
        r2[1].orig.regs[0] = 0x8E2C70;
        let rep = split_report(&p, &r2);
        assert_eq!(rep.fns[0].verdict(), "FAIL");
        assert!(rep.fns[0].first_mismatch.as_ref().unwrap().contains("eax"));
        // an input difference ends the comparison
        let mut r3 = r.clone();
        r3[1].page_hashes[1].1 = 3;
        let rep = split_report(&p, &r3);
        assert_eq!(
            rep.fns[0].verdict(),
            "PASS-FP",
            "undeclared state differed, outputs equal"
        );
        assert_eq!(rep.fns[0].compared, 2);
        assert_eq!(rep.fns[0].fingerprint_pages.get(&0x8A5000), Some(&1));
        let mut r4 = r.clone();
        r4[0].regions[0].pre[1] = 7;
        assert!(
            split_report(&p, &r4).fns[0]
                .diverged
                .as_ref()
                .unwrap()
                .2
                .contains("0xb71a61")
        );
        // scratch registers are soft
        let mut r5 = r.clone();
        r5[0].regs_in[2] = 99;
        let rep = split_report(&p, &r5);
        assert_eq!((rep.fns[0].verdict(), rep.fns[0].soft_input_diffs), ("PASS", 1));
        // a shifted stack: a stack-pointer argument and a write to the caller's stack
        let mut p2 = p.clone();
        let mut r6 = r.clone();
        for (c, esp) in p2
            .iter_mut()
            .map(|c| (c, 0x36F1F0))
            .chain(r6.iter_mut().map(|c| (c, 0x36F1C0)))
        {
            c.esp = esp;
            c.stack = (0x270000, 0x370000);
            c.args[0] = esp + 0x18;
            c.orig.writes.push(Write {
                addr: esp + 0x20,
                bytes: vec![5],
            });
            c.window_pre.push(Write {
                addr: esp + 0x20,
                bytes: vec![0],
            });
        }
        let rep = split_report(&p2, &r6);
        assert_eq!(rep.fns[0].verdict(), "PASS", "{:?}", rep.fns[0]);
        assert_eq!(rep.fns[0].stack_shifts.get(&0x30), Some(&2));
        r6[0].orig.writes.last_mut().unwrap().bytes[0] = 6;
        assert_eq!(split_report(&p2, &r6).fns[0].verdict(), "FAIL");
        r6[1].args[0] += 4;
        assert!(split_report(&p2, &r6).fns[0].diverged.is_some());
        // OS object addresses: equal through the Foreign class, listed as uses
        let mut p3 = p.clone();
        let mut r7 = r.clone();
        p3[0].orig.regs[0] = 0x0301_A530;
        p3[0].foreign = vec![0x0301_A530];
        r7[0].orig.regs[0] = 0x0301_A340;
        r7[0].foreign = vec![0x0301_A340];
        let rep = split_report(&p3, &r7);
        assert_eq!(rep.fns[0].verdict(), "PASS");
        assert_eq!(rep.fns[0].foreign_uses.get("eax out"), Some(&1));
        r7[0].foreign.clear();
        assert_eq!(split_report(&p3, &r7).fns[0].verdict(), "FAIL");
        // a region at an OS-allocated address (an extra region `[0x8E3FFC]:...`): same object
        let mut p5 = p.clone();
        let mut r9 = r.clone();
        for (c, at) in [(&mut p5[0], 0x02F3_AB10u32), (&mut r9[0], 0x02FD_E530)] {
            c.regions[0].addr = at;
            c.foreign = vec![at];
        }
        let rep = split_report(&p5, &r9);
        assert_eq!(rep.fns[0].verdict(), "PASS", "{:?}", rep.fns[0]);
        assert_eq!(rep.fns[0].foreign_uses.get("region address 0x2f3ab10"), Some(&1));
        r9[0].foreign.clear();
        assert!(
            split_report(&p5, &r9).fns[0].diverged.is_some(),
            "a raw address in one run differs"
        );
        // callee-saved EBP with different entry values: preserved in both runs = equal
        let mut p4 = p.clone();
        let mut r8 = r.clone();
        (p4[0].regs_in[6], p4[0].orig.regs[6]) = (1, 1);
        (r8[0].regs_in[6], r8[0].orig.regs[6]) = (0x36F21C, 0x36F21C);
        let rep = split_report(&p4, &r8);
        assert_eq!((rep.fns[0].verdict(), rep.fns[0].soft_input_diffs), ("PASS", 1));
        r8[0].orig.regs[6] = 5;
        assert_eq!(split_report(&p4, &r8).fns[0].verdict(), "FAIL", "not preserved");
        // missing calls
        let rep = split_report(&p, &r[..1]);
        assert_eq!(rep.fns[0].verdict(), "COUNT");
        // wrong spool kind
        assert!(!split_report(&r, &p).errors.is_empty());
    }
}
