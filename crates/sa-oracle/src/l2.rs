//! L2 oracle, host side (INF-8): read/write-set spec generation from the catalogue, the content-addressed
//! fixture store, reports, and the replay of stored cases without the game.
//!
//! - `spec`: per hooked function, the globals of its read/write set from `catalog/rwsets.csv`, closed
//!   over its callees (`catalog/callgraph.csv`, kinds `call`/`tailcall`, transitively). `direct` accesses
//!   are exact; `based`/`indexed` accesses and address-taken (`A`) entries are `unbounded` (the catalogue
//!   knows the base, not the extent; an `A` entry counts as read and written). Unresolved indirect calls
//!   are listed in the spec comments: their callees' sets are missing from the spec.
//! - `ingest`: spool (written by sa-hook) -> `fixtures/l2/<addr>/<content id>.l2case.zst` (zstd), with
//!   identical cases stored once (the content id excludes the provenance: frame, ordinal, caller).
//! - `replay`: decodes stored cases and recomputes the Rust side on the host with the same Rust code the
//!   hooks run (sa-hook's hook functions / sa-math), then compares with the stored original outputs.
//!   The host recomputes what the function defines (return value, x87 stack, memory written through
//!   `this`/pointer arguments); ABI-level fields (callee-saved registers, bytes popped, CW/MXCSR) and
//!   callee logs are taken from the stored in-process Rust run.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sa_hook::table::{HOOKS, Id};
use sa_oracle_format::l2::{self as fmt, ACC_R, ACC_W, Case, FnSpec, Global, Outcome, RegionKind, Write};

/// Fixture quota (PLAN §6.2).
pub const QUOTA_BYTES: u64 = 20 << 30;

fn hex_u32(s: &str) -> Option<u32> {
    u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

// ------------------------------------------------------------------ spec ----------------------------

/// (addr, size, access, exact, name)
type RwEntry = (u32, u32, u8, bool, String);

pub struct RwCatalog {
    /// function -> entries (addr, size, access, exact, name)
    rw: BTreeMap<u32, Vec<RwEntry>>,
    calls: BTreeMap<u32, Vec<u32>>,
    unresolved: BTreeMap<u32, u32>,
    /// global address -> name (from the `name` column, else `near`)
    pub names: BTreeMap<u32, String>,
}

impl RwCatalog {
    pub fn load(root: &Path) -> Result<RwCatalog, String> {
        let rw_text = std::fs::read_to_string(root.join("catalog/rwsets.csv"))
            .map_err(|e| format!("rwsets.csv: {e}"))?;
        let cg_text = std::fs::read_to_string(root.join("catalog/callgraph.csv"))
            .map_err(|e| format!("callgraph.csv: {e}"))?;
        Ok(Self::parse(&rw_text, &cg_text))
    }

    pub fn parse(rw_text: &str, cg_text: &str) -> RwCatalog {
        let mut rw: BTreeMap<u32, Vec<RwEntry>> = BTreeMap::new();
        let mut names = BTreeMap::new();
        // function,address,section,name,near,size,access,mode
        for l in rw_text.lines().skip(1) {
            let f: Vec<&str> = l.split(',').collect();
            if f.len() < 8 {
                continue;
            }
            let (Some(func), Some(addr)) = (hex_u32(f[0]), hex_u32(f[1])) else {
                continue;
            };
            let size: u32 = f[5].parse().unwrap_or(0);
            let access = match f[6] {
                "R" => ACC_R,
                "W" => ACC_W,
                "RW" | "A" => ACC_R | ACC_W,
                _ => continue,
            };
            let exact = f[7] == "direct" && f[6] != "A" && size > 0;
            let name = if !f[3].is_empty() { f[3] } else { f[4] };
            if !f[3].is_empty() {
                names.insert(addr, f[3].to_string());
            }
            rw.entry(func)
                .or_default()
                .push((addr, size.max(1), access, exact, name.to_string()));
        }
        let mut calls: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        let mut unresolved = BTreeMap::new();
        // caller,callee,kind,callee_class,callee_name,sites
        for l in cg_text.lines().skip(1) {
            let f: Vec<&str> = l.split(',').collect();
            if f.len() < 3 {
                continue;
            }
            let Some(caller) = hex_u32(f[0]) else { continue };
            match f[2] {
                "call" | "tailcall" => {
                    if let Some(c) = hex_u32(f[1]) {
                        calls.entry(caller).or_default().push(c);
                    }
                }
                "indirect-unresolved" | "virtual" | "virtual-override" | "indirect-ghidra" => {
                    *unresolved.entry(caller).or_insert(0) +=
                        f.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);
                }
                _ => {}
            }
        }
        RwCatalog {
            rw,
            calls,
            unresolved,
            names,
        }
    }

    /// The function's set closed over its direct callees. Returns (spec, functions included,
    /// unresolved indirect call sites in the closure).
    pub fn spec(&self, addr: u32, name: &str) -> (FnSpec, Vec<u32>, u32) {
        let mut seen = BTreeSet::new();
        let mut todo = vec![addr];
        while let Some(f) = todo.pop() {
            if !seen.insert(f) {
                continue;
            }
            todo.extend(self.calls.get(&f).into_iter().flatten().copied());
        }
        let mut merged: BTreeMap<u32, Global> = BTreeMap::new();
        let mut unresolved = 0;
        for f in &seen {
            unresolved += self.unresolved.get(f).copied().unwrap_or(0);
            for (a, n, acc, exact, nm) in self.rw.get(f).into_iter().flatten() {
                let g = merged.entry(*a).or_insert_with(|| Global {
                    addr: *a,
                    len: *n,
                    access: 0,
                    exact: true,
                    name: nm.clone(),
                });
                g.len = g.len.max(*n);
                g.access |= acc;
                g.exact &= exact;
                if g.name.is_empty() {
                    g.name = nm.clone();
                }
            }
        }
        (
            FnSpec {
                addr,
                name: name.to_string(),
                globals: merged.into_values().collect(),
            },
            seen.into_iter().collect(),
            unresolved,
        )
    }

    /// `name+off` for a global address (nearest named global at or below, within 64 KiB).
    pub fn symbol(&self, a: u32) -> String {
        match self.names.range(..=a).next_back() {
            Some((b, n)) if *b == a => n.clone(),
            Some((b, n)) if a - b < 0x10000 => format!("{n}+{:#x}", a - b),
            _ => "?".into(),
        }
    }
}

/// Spec text for the given hooks (all hooks with an L2 signature when empty).
pub fn spec_text(cat: &RwCatalog, which: &[usize]) -> String {
    let list: Vec<usize> = if which.is_empty() {
        (0..HOOKS.len()).filter(|&i| HOOKS[i].sig.is_some()).collect()
    } else {
        which.to_vec()
    };
    let mut specs = Vec::new();
    let mut notes = String::new();
    for i in list {
        let h = &HOOKS[i];
        let (s, fns, unresolved) = cat.spec(h.addr, h.name);
        notes += &format!(
            "# {:#x} {}: {} globals from {} function(s) {:x?}; {} unresolved indirect/virtual call site(s){}\n",
            h.addr,
            h.name,
            s.globals.len(),
            fns.len(),
            fns,
            unresolved,
            if unresolved > 0 {
                " (their callees' sets are NOT in this spec)"
            } else {
                ""
            }
        );
        specs.push(s);
    }
    notes + &fmt::format_spec(&specs)
}

// ------------------------------------------------------------------ fixtures ------------------------

fn dir_size(p: &Path) -> u64 {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let Ok(md) = e.metadata() else { continue };
            n += if md.is_dir() {
                dir_size(&e.path())
            } else {
                md.len()
            };
        }
    }
    n
}

pub struct IngestReport {
    pub cases: usize,
    pub new: usize,
    pub dup: usize,
    pub bytes: u64,
}

/// Stores every case of a spool content-addressed under `out/<addr>/<id>.l2case.zst`.
pub fn ingest(spool: &[Case], out: &Path) -> Result<IngestReport, String> {
    let used = dir_size(out);
    if used > QUOTA_BYTES {
        return Err(format!(
            "{}: {used} bytes exceed the fixture quota",
            out.display()
        ));
    }
    let mut r = IngestReport {
        cases: spool.len(),
        new: 0,
        dup: 0,
        bytes: 0,
    };
    for c in spool {
        let dir = out.join(format!("{:#x}", c.fn_addr));
        let path = dir.join(format!("{}.l2case.zst", c.content_id()));
        if path.exists() {
            r.dup += 1;
            continue;
        }
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let z = zstd::encode_all(&c.encode()[..], 19).map_err(|e| e.to_string())?;
        r.bytes += z.len() as u64;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &z).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        r.new += 1;
    }
    Ok(r)
}

/// All stored cases under `dir` (optionally only one function address).
pub fn load_fixtures(dir: &Path, only: Option<u32>) -> Result<Vec<(PathBuf, Case)>, String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Ok(out);
    };
    let mut fdirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    fdirs.sort();
    for fd in fdirs {
        let addr = fd.file_name().and_then(|n| n.to_str()).and_then(hex_u32);
        if only.is_some() && addr != only {
            continue;
        }
        let mut files: Vec<PathBuf> = std::fs::read_dir(&fd)
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().ends_with(".l2case.zst"))
            .collect();
        files.sort();
        for p in files {
            let z = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            let b = zstd::decode_all(&z[..]).map_err(|e| format!("{}: {e}", p.display()))?;
            out.push((
                p.clone(),
                Case::decode(&b).map_err(|e| format!("{}: {e}", p.display()))?,
            ));
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ host replay ---------------------

/// What `fld qword` makes of an f64 (exact; a signalling NaN is quieted).
pub fn f64_to_ext(v: f64) -> [u8; 10] {
    let b = v.to_bits();
    let sign = ((b >> 63) as u16) << 15;
    let exp = ((b >> 52) & 0x7FF) as u16;
    let frac = b & ((1u64 << 52) - 1);
    let (e, m): (u16, u64) = if exp == 0x7FF {
        let m = (1u64 << 63) | (frac << 11);
        (0x7FFF, if frac != 0 { m | (1 << 62) } else { m })
    } else if exp == 0 {
        if frac == 0 {
            (0, 0)
        } else {
            let lz = frac.leading_zeros() - 11;
            (16383 - 1022 - lz as u16, frac << (lz + 11))
        }
    } else {
        (exp + (16383 - 1023), (1u64 << 63) | (frac << 11))
    };
    let mut out = [0u8; 10];
    out[..8].copy_from_slice(&m.to_le_bytes());
    out[8..].copy_from_slice(&(sign | e).to_le_bytes());
    out
}

/// Memory model of a case: the snapshotted pre-call bytes plus the writes of a host run.
struct Mem<'a> {
    c: &'a Case,
    writes: BTreeMap<u32, u8>,
}

impl Mem<'_> {
    fn pre(&self, a: u32) -> Option<u8> {
        for r in self.c.regions.iter().filter(|r| r.has_pre_memory()) {
            if r.readable && a >= r.addr && a - r.addr < r.len() {
                return Some(r.pre[(a - r.addr) as usize]);
            }
        }
        for w in &self.c.window_pre {
            if a >= w.addr && a - w.addr < w.bytes.len() as u32 {
                return Some(w.bytes[(a - w.addr) as usize]);
            }
        }
        None
    }
    fn get(&self, a: u32) -> Option<u8> {
        self.writes.get(&a).copied().or_else(|| self.pre(a))
    }
    fn get_u32(&self, a: u32) -> Option<u32> {
        Some(u32::from_le_bytes([
            self.get(a)?,
            self.get(a + 1)?,
            self.get(a + 2)?,
            self.get(a + 3)?,
        ]))
    }
    fn put(&mut self, a: u32, b: &[u8]) {
        for (k, &x) in b.iter().enumerate() {
            self.writes.insert(a + k as u32, x);
        }
    }
}

/// Recomputes the Rust side of a stored case on the host. Err = not replayable (reason).
pub fn host_rust(c: &Case) -> Result<Case, String> {
    let i = HOOKS
        .iter()
        .position(|h| h.addr == c.fn_addr)
        .ok_or_else(|| format!("{:#x}: no hook", c.fn_addr))?;
    let Some(id) = HOOKS[i].id else {
        return Err(format!(
            "{}: engine-crate port (sa-core), no host replay yet",
            HOOKS[i].name
        ));
    };
    let f = |k: usize| -> Result<f32, String> {
        c.args
            .get(k)
            .map(|&w| f32::from_bits(w))
            .ok_or_else(|| "missing argument".to_string())
    };
    let mut m = Mem {
        c,
        writes: BTreeMap::new(),
    };
    // Start from the stored in-process Rust run (ABI fields), replace what the function computes.
    let mut o: Outcome = c.rust.clone();
    let st = |v: f64| vec![f64_to_ext(v)];
    match id {
        Id::LimitRadianAngle => o.st = st(sa_hook::game::limit_radian_angle_hook(f(0)?)),
        Id::GetATanOfXY => o.st = st(sa_hook::game::get_atan_of_xy_hook(f(0)?, f(1)?)),
        Id::GetRadianAngleBetweenPoints => {
            o.st = st(sa_hook::game::get_radian_angle_between_points_hook(
                f(0)?,
                f(1)?,
                f(2)?,
                f(3)?,
            ))
        }
        Id::LimitAngle => match sa_math::general::limit_angle_checked(f(0)?) {
            Some(_) => o.st = st(sa_hook::game::limit_angle_hook(f(0)?)),
            None => return Err("the Rust port spins for this input".into()),
        },
        Id::SolveQuadratic => {
            let (p1, p2) = (c.args[3], c.args[4]);
            let mut cell = [
                f32::from_bits(m.get_u32(p1).unwrap_or(0)),
                f32::from_bits(m.get_u32(p2).unwrap_or(0)),
            ];
            let base = cell.as_mut_ptr();
            let (q1, q2) = if p1 == p2 {
                (base, base)
            } else {
                (base, base.wrapping_add(1))
            };
            // SAFETY: q1/q2 point into `cell` (aliased when the case's pointers alias).
            let r = unsafe { sa_hook::game::solve_quadratic_hook(f(0)?, f(1)?, f(2)?, q1, q2) };
            o.regs[0] = (o.regs[0] & !0xFF) | r as u32;
            if r {
                if p1 == p2 {
                    m.put(p1, &cell[0].to_bits().to_le_bytes());
                } else {
                    m.put(p2, &cell[1].to_bits().to_le_bytes());
                    m.put(p1, &cell[0].to_bits().to_le_bytes());
                }
            }
        }
        Id::VectorNormalise => {
            let this = c.regs_in[1];
            let v: Vec<f32> = (0..3)
                .map(|k| {
                    m.get_u32(this + 4 * k)
                        .map(f32::from_bits)
                        .ok_or("this not snapshotted")
                })
                .collect::<Result<_, _>>()?;
            let r = sa_math::vector::normalise([v[0], v[1], v[2]]);
            m.put(this, &r[0].to_bits().to_le_bytes());
            m.put(this + 4, &r[1].to_bits().to_le_bytes());
            if !sa_hook::l2::mutant(Id::VectorNormalise) {
                m.put(this + 8, &r[2].to_bits().to_le_bytes());
            }
        }
        Id::IsCoopGameGoingOn => {
            use sa_hook::game::game_logic::{PLAYER_INFO_SIZE, PLAYERS};
            let p1 = m.get_u32(PLAYERS).ok_or("CWorld::Players not snapshotted")?;
            let p2 = m
                .get_u32(PLAYERS + PLAYER_INFO_SIZE)
                .ok_or("CWorld::Players[1] not snapshotted")?;
            o.regs[0] = sa_hook::game::game_logic::is_coop_game_going_on(p1, p2)
                ^ sa_hook::l2::mutant(Id::IsCoopGameGoingOn) as u32;
        }
        _ => {
            return Err(format!(
                "{}: no host replay (its Rust side calls the original)",
                HOOKS[i].name
            ));
        }
    }
    // Memory effects: region posts and window writes from the host run's writes.
    let mut out = c.clone();
    for r in out.regions.iter_mut().filter(|r| r.readable) {
        r.post_rust = (0..r.len()).map(|k| m.get(r.addr + k).unwrap_or(0)).collect();
    }
    let mut changed: BTreeMap<u32, u8> = BTreeMap::new();
    for (&a, &b) in &m.writes {
        let in_window = c.windows.iter().any(|&(s, e)| a >= s && a < e);
        if in_window && m.pre(a) != Some(b) {
            changed.insert(a, b);
        }
    }
    let mut writes: Vec<Write> = Vec::new();
    for (a, b) in changed {
        match writes.last_mut() {
            Some(w) if w.addr + w.bytes.len() as u32 == a => w.bytes.push(b),
            _ => writes.push(Write {
                addr: a,
                bytes: vec![b],
            }),
        }
    }
    o.writes = writes;
    out.rust = o;
    Ok(out)
}

pub fn is_ledger(c: &Case, diffs: &[fmt::Diff]) -> bool {
    c.fn_addr == 0x53CC70
        && diffs.iter().all(|d| d.field == "st0")
        && c.orig.st.len() == 1
        && c.rust.st.len() == 1
        && fmt::ext_to_f32_bits(&c.orig.st[0]) == fmt::ext_to_f32_bits(&c.rust.st[0])
}

/// Per-function counts of a set of cases.
#[derive(Default, Debug)]
pub struct Tally {
    pub cases: usize,
    pub mismatches: usize,
    pub ledger: usize,
    pub not_replayable: usize,
    pub undeclared_cases: usize,
    /// (addr, len) -> (count, runs)
    pub undeclared: BTreeMap<(u32, u32), (usize, BTreeSet<&'static str>)>,
    pub examples: Vec<String>,
}

pub fn describe_case(c: &Case) -> String {
    let args: Vec<String> = c
        .args
        .iter()
        .map(|&w| format!("{w:#x}({})", f32::from_bits(w)))
        .collect();
    format!(
        "{} frame {} caller {:#x} ecx {:#x} args [{}]",
        c.name,
        c.frame,
        c.caller,
        c.regs_in[1],
        args.join(",")
    )
}

/// Tallies stored outcomes (`host = false`) or host-recomputed Rust outcomes (`host = true`).
pub fn tally(cases: &[Case], host: bool) -> BTreeMap<(u32, String), Tally> {
    let mut t: BTreeMap<(u32, String), Tally> = BTreeMap::new();
    for c in cases {
        let e = t.entry((c.fn_addr, c.name.clone())).or_default();
        e.cases += 1;
        let c2;
        let cc = if host {
            match host_rust(c) {
                Ok(x) => {
                    c2 = x;
                    &c2
                }
                Err(err) => {
                    e.not_replayable += 1;
                    if e.examples.len() < 3 {
                        e.examples.push(format!("not replayable: {err}"));
                    }
                    continue;
                }
            }
        } else {
            c
        };
        let d = fmt::compare(cc);
        if !d.is_empty() {
            if is_ledger(cc, &d) {
                e.ledger += 1;
            } else {
                e.mismatches += 1;
                if e.examples.len() < 3 {
                    e.examples.push(format!(
                        "MISMATCH {}: {}",
                        describe_case(cc),
                        d.iter()
                            .map(|x| format!("{}: {}", x.field, x.detail))
                            .collect::<Vec<_>>()
                            .join("; ")
                    ));
                }
            }
        }
        if !host {
            let u = fmt::undeclared_writes(cc);
            if !u.is_empty() {
                e.undeclared_cases += 1;
            }
            for x in u {
                let v = e.undeclared.entry((x.addr, x.len)).or_default();
                v.0 += 1;
                v.1.insert(x.run);
            }
        }
    }
    t
}

pub fn print_tally(t: &BTreeMap<(u32, String), Tally>, cat: Option<&RwCatalog>) -> String {
    let mut s = String::new();
    for ((addr, name), x) in t {
        s += &format!(
            "{addr:#010x} {name:<42} cases={:<6} mismatches={:<5} ledger={:<5} not_replayable={:<4} undeclared_cases={}\n",
            x.cases, x.mismatches, x.ledger, x.not_replayable, x.undeclared_cases
        );
        for ((a, n), (k, runs)) in x.undeclared.iter().take(12) {
            s += &format!(
                "    write outside the declared set: {a:#x}+{n} ({}) in {k} case(s), runs {:?}\n",
                cat.map(|c| c.symbol(*a)).unwrap_or_default(),
                runs
            );
        }
        for e in &x.examples {
            s += &format!("    {e}\n");
        }
    }
    s
}

/// Region kinds of a case, for `show`.
pub fn show_case(c: &Case) -> String {
    let mut s = format!("{}\n", describe_case(c));
    s += &format!(
        "  regs_in {:x?} mxcsr {:#x} leave {} mutant {} seq {} page_hashes {}\n",
        c.regs_in,
        c.mxcsr,
        c.leave,
        c.mutant,
        c.seq,
        c.page_hashes.len()
    );
    for r in &c.regions {
        let k = match r.kind {
            RegionKind::Global { exact } => format!("global{}", if exact { "" } else { "(unbounded)" }),
            RegionKind::This => "this".into(),
            RegionKind::ArgPtr(i) => format!("*arg{i}"),
            RegionKind::Extra => "extra".into(),
            RegionKind::Post => "post".into(),
            RegionKind::Os(p) => {
                s += &format!(
                    "  os-{} {:#x} pre [{}] orig [{}] rust [{}]\n",
                    p.name(),
                    r.addr,
                    fmt::describe_os(p, &r.pre),
                    fmt::describe_os(p, &r.post_orig),
                    fmt::describe_os(p, &r.post_rust)
                );
                continue;
            }
        };
        let h = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        s += &format!(
            "  {k} {:#x} acc {} pre {} orig {} rust {}\n",
            r.addr,
            r.access,
            h(&r.pre),
            h(&r.post_orig),
            h(&r.post_rust)
        );
    }
    for (n, o) in [("orig", &c.orig), ("rust", &c.rust)] {
        s += &format!(
            "  {n}: regs {:x?} pop {} st [{}] cw {:#x} mxcsr {:#x} writes {} calls {:?} allocs {:?}\n",
            o.regs,
            o.pop,
            o.st.iter().map(fmt::ext_str).collect::<Vec<_>>().join(","),
            o.cw,
            o.mxcsr,
            o.writes
                .iter()
                .map(|w| format!("{:#x}+{}", w.addr, w.bytes.len()))
                .collect::<Vec<_>>()
                .join(","),
            o.calls,
            o.allocs
        );
    }
    let d = fmt::compare(c);
    s += &format!(
        "  verdict: {}\n",
        if d.is_empty() {
            "identical".to_string()
        } else {
            d.iter()
                .map(|x| format!("{}: {}", x.field, x.detail))
                .collect::<Vec<_>>()
                .join("; ")
        }
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_closure_and_modes() {
        let rw = "function,address,section,name,near,size,access,mode\n\
                  0x100,0xb00,.data,G1,,4,R,direct\n\
                  0x100,0xb00,.data,G1,,4,W,direct\n\
                  0x200,0xc00,.data,,G2+0x4,4,W,indexed\n\
                  0x200,0xd00,.data,,,0,A,imm\n";
        let cg = "caller,callee,kind,callee_class,callee_name,sites\n\
                  0x100,0x200,call,game,f,1\n0x200,0x100,call,game,g,1\n0x200,,indirect-unresolved,,,2\n";
        let c = RwCatalog::parse(rw, cg);
        let (s, fns, un) = c.spec(0x100, "F");
        assert_eq!(fns, vec![0x100, 0x200]);
        assert_eq!(un, 2);
        assert_eq!(s.globals.len(), 3);
        assert_eq!((s.globals[0].access, s.globals[0].exact), (ACC_R | ACC_W, true));
        assert_eq!((s.globals[1].access, s.globals[1].exact), (ACC_W, false));
        assert_eq!(
            (s.globals[2].access, s.globals[2].exact, s.globals[2].len),
            (ACC_R | ACC_W, false, 1)
        );
        assert_eq!(c.symbol(0xb02), "G1+0x2");
    }

    #[test]
    fn ext_of_f64() {
        let e = f64_to_ext(1.0);
        assert_eq!(fmt::ext_str(&e), "3fff:8000000000000000");
        assert_eq!(fmt::ext_to_f32_bits(&f64_to_ext(-2.5)), (-2.5f32).to_bits());
        assert_eq!(
            sa_math::x87::X87::from_f32(0.1).to_ext(),
            f64_to_ext(0.1f32 as f64),
            "same encoding as the x87 model"
        );
    }
}
