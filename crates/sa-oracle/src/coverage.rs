//! Execution coverage of the original code (INF-10), host side. See `docs/re/l3-coverage.md`.
//!
//! - [`points`]: the breakpoint list for sa-hook's coverage mode (`SA_COV`): every catalogue function
//!   entry plus the basic-block starts found by decoding each function from its entry inside its Ghidra
//!   body ranges (`catalog/extents.csv`), with the exe byte at each point.
//! - [`Coverage`]: merged hits of one or more runs (`address,hits` CSVs written by sa-hook), joined
//!   with the points and the catalogue into a per-function table and a per-subsystem report.
//!
//! Block decoding is recursive descent: fall-through, direct branch targets and jump tables of the form
//! `jmp [table + reg*4]` whose entries land in the function's body. Only addresses that decode as an
//! instruction start inside the body become points, and no point may fall inside another decoded
//! instruction or a jump table, so a breakpoint never lands in data or mid-instruction. Functions
//! without body ranges (no Ghidra extent) get their entry only. `excluded:protection` and
//! `excluded:mis-split` rows are not armed (junk code and split artefacts inside other functions).

use std::collections::{BTreeMap, BTreeSet};

use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction, OpKind, Register};

use crate::pe::Image;

/// One catalogue row (the columns coverage needs).
#[derive(Debug, Clone)]
pub struct Row {
    pub addr: u32,
    pub region: String,
    pub state: String,
    pub subsystem: String,
    pub class: String,
    pub name: String,
}

impl Row {
    pub fn excluded(&self) -> bool {
        self.state == "excluded" || self.class.starts_with("excluded:")
    }
    /// Rows never armed (see the module doc).
    pub fn not_armed(&self) -> bool {
        self.class == "excluded:protection"
            || self.class == "excluded:mis-split"
            || self.region != "game-text"
    }
}

fn hex(s: &str) -> Option<u32> {
    u32::from_str_radix(s.trim().trim_start_matches("0x").trim_start_matches("0X"), 16).ok()
}

/// `catalog/procedures.csv` rows.
pub fn parse_procedures(text: &str) -> Result<Vec<Row>, String> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().ok_or("empty procedures.csv")?.split(',').collect();
    let col = |n: &str| {
        header
            .iter()
            .position(|h| *h == n)
            .ok_or(format!("procedures.csv: no column {n}"))
    };
    let (ca, cr, cs, csub, ccl, cgr, cgh, cn) = (
        col("address")?,
        col("region")?,
        col("state")?,
        col("subsystem")?,
        col("class")?,
        col("gr_name")?,
        col("ghidra_name")?,
        col("name")?,
    );
    let mut out = Vec::new();
    for l in lines {
        let f: Vec<&str> = l.split(',').collect();
        let Some(addr) = f.get(ca).and_then(|s| hex(s)) else {
            continue;
        };
        let g = |i: usize| f.get(i).copied().unwrap_or("").to_string();
        let name = [g(cgr), g(cn), g(cgh)]
            .into_iter()
            .find(|s| !s.is_empty())
            .unwrap_or_default();
        out.push(Row {
            addr,
            region: g(cr),
            state: g(cs),
            subsystem: g(csub),
            class: g(ccl),
            name,
        });
    }
    Ok(out)
}

/// `catalog/extents.csv`: address -> body ranges [lo, hi).
pub fn parse_extents(text: &str) -> Result<BTreeMap<u32, Vec<(u32, u32)>>, String> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().ok_or("empty extents.csv")?.split(',').collect();
    let col = |n: &str| {
        header
            .iter()
            .position(|h| *h == n)
            .ok_or(format!("extents.csv: no column {n}"))
    };
    let (ca, cr) = (col("address")?, col("ranges")?);
    let mut out = BTreeMap::new();
    for l in lines {
        let f: Vec<&str> = l.split(',').collect();
        let (Some(a), Some(rs)) = (f.get(ca).and_then(|s| hex(s)), f.get(cr)) else {
            continue;
        };
        let v: Vec<(u32, u32)> = rs
            .split(';')
            .filter_map(|r| {
                let (lo, hi) = r.split_once('-')?;
                Some((hex(lo)?, hex(hi)?))
            })
            .filter(|(lo, hi)| hi > lo)
            .collect();
        if !v.is_empty() {
            out.insert(a, v);
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// Function entry.
    Entry,
    /// Basic-block start inside a function (not its entry).
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Point {
    pub addr: u32,
    pub byte: u8,
    pub kind: Kind,
    /// Owning function (catalogue entry).
    pub func: u32,
}

/// Per-function decode statistics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decoded {
    pub blocks: Vec<u32>,
    pub insns: usize,
    pub tables: usize,
    pub unresolved_indirect: usize,
    pub bad: usize,
    /// (start, len) of every decoded instruction.
    pub spans: Vec<(u32, u32)>,
    /// (start, len) of every jump table read.
    pub table_spans: Vec<(u32, u32)>,
}

/// Recursive descent over one function. `code(addr)` returns up to 16 bytes at `addr`.
pub fn decode_function<'a>(entry: u32, ranges: &[(u32, u32)], code: impl Fn(u32) -> &'a [u8]) -> Decoded {
    let inr = |a: u32, n: u32| {
        ranges
            .iter()
            .any(|&(lo, hi)| a >= lo && a.saturating_add(n) <= hi)
    };
    let mut d = Decoded::default();
    let mut starts: BTreeSet<u32> = BTreeSet::from([entry]);
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut work = vec![entry];
    let mut ins = Instruction::default();
    while let Some(ip) = work.pop() {
        if !seen.insert(ip) {
            continue;
        }
        if !inr(ip, 1) {
            starts.remove(&ip);
            continue;
        }
        let bytes = code(ip);
        let mut dec = Decoder::with_ip(32, bytes, ip as u64, DecoderOptions::NONE);
        if !dec.can_decode() {
            starts.remove(&ip);
            d.bad += 1;
            continue;
        }
        dec.decode_out(&mut ins);
        let len = ins.len() as u32;
        if ins.is_invalid() || !inr(ip, len) {
            starts.remove(&ip);
            d.bad += 1;
            continue;
        }
        d.insns += 1;
        d.spans.push((ip, len));
        let next = ip + len;
        let mut push = |t: u32, block: bool, work: &mut Vec<u32>| {
            if inr(t, 1) {
                if block {
                    starts.insert(t);
                }
                work.push(t);
            }
        };
        match ins.flow_control() {
            FlowControl::Next | FlowControl::Call | FlowControl::IndirectCall => push(next, false, &mut work),
            FlowControl::ConditionalBranch => {
                push(ins.near_branch32(), true, &mut work);
                push(next, true, &mut work);
            }
            FlowControl::UnconditionalBranch => push(ins.near_branch32(), true, &mut work),
            FlowControl::IndirectBranch => {
                // jmp dword [disp + reg*4]: a switch table
                let table = (ins.op_count() == 1
                    && ins.op0_kind() == OpKind::Memory
                    && ins.memory_base() == Register::None
                    && ins.memory_index() != Register::None
                    && ins.memory_index_scale() == 4)
                    .then(|| ins.memory_displacement32());
                let mut n = 0u32;
                if let Some(t) = table {
                    while n < 1024 {
                        let e = code(t + 4 * n);
                        if e.len() < 4 || inr(t + 4 * n, 4) {
                            break;
                        }
                        let target = u32::from_le_bytes([e[0], e[1], e[2], e[3]]);
                        if !inr(target, 1) {
                            break;
                        }
                        push(target, true, &mut work);
                        n += 1;
                    }
                    if n > 0 {
                        d.tables += 1;
                        d.table_spans.push((t, 4 * n));
                    }
                }
                if n == 0 {
                    d.unresolved_indirect += 1;
                }
            }
            _ => {} // Return, Interrupt, Exception, Xbegin...: path ends
        }
    }
    // Keep only starts that decoded, and none inside another instruction or a table.
    d.spans.sort_unstable();
    d.spans.dedup();
    let spans = &d.spans;
    let tables = &d.table_spans;
    let inside = |p: u32| {
        let lo = spans.partition_point(|&(s, _)| s < p.saturating_sub(15));
        let hi = spans.partition_point(|&(s, _)| s < p);
        spans[lo..hi].iter().any(|&(s, n)| p < s + n) || tables.iter().any(|&(s, n)| s <= p && p < s + n)
    };
    let blocks: Vec<u32> = starts
        .into_iter()
        .filter(|&p| spans.binary_search_by_key(&p, |&(s, _)| s).is_ok() && !inside(p))
        .collect();
    d.blocks = blocks;
    d
}

/// Statistics of a points list.
#[derive(Debug, Clone, Default)]
pub struct PointStats {
    pub functions: usize,
    pub with_body: usize,
    pub entry_only: usize,
    pub entry_bad: usize,
    pub blocks: usize,
    pub tables: usize,
    pub unresolved_indirect: usize,
    pub not_armed: usize,
    pub shared: usize,
}

/// Points for every armed catalogue function (entries always; blocks when `blocks`).
pub fn points(
    img: &Image,
    rows: &[Row],
    extents: &BTreeMap<u32, Vec<(u32, u32)>>,
    blocks: bool,
) -> (Vec<Point>, PointStats) {
    let mut st = PointStats::default();
    let mut out: BTreeMap<u32, Point> = BTreeMap::new();
    let code = |a: u32| img.bytes(a, 16);
    for r in rows {
        if r.not_armed() {
            st.not_armed += 1;
            continue;
        }
        st.functions += 1;
        // The entry instruction itself always belongs to the body (Ghidra leaves the entry `jmp` of
        // some functions whose body is a chunk elsewhere out of their ranges).
        let first = Decoder::with_ip(32, code(r.addr), r.addr as u64, DecoderOptions::NONE).decode();
        if first.is_invalid() {
            st.entry_bad += 1;
            continue;
        }
        let own = (r.addr, r.addr + first.len() as u32);
        let ranges = extents.get(&r.addr).map(|v| {
            let mut v = v.clone();
            v.push(own);
            v
        });
        let d = decode_function(r.addr, ranges.as_deref().unwrap_or(&[own]), code);
        if !d.blocks.contains(&r.addr) {
            st.entry_bad += 1;
            if std::env::var("SA_COV_DEBUG").is_ok() {
                eprintln!(
                    "entry_bad {:#x} {} {} {:?} {:02x?}",
                    r.addr,
                    r.class,
                    r.subsystem,
                    ranges,
                    img.bytes(r.addr, 8)
                );
            }
            continue;
        }
        let list: Vec<u32> = if blocks && ranges.is_some() {
            st.with_body += 1;
            st.tables += d.tables;
            st.unresolved_indirect += d.unresolved_indirect;
            d.blocks.clone()
        } else {
            st.entry_only += 1;
            vec![r.addr]
        };
        for a in list {
            let kind = if a == r.addr { Kind::Entry } else { Kind::Block };
            match out.get_mut(&a) {
                // A block that is another function's entry belongs to that function.
                Some(p) if p.kind == Kind::Block && kind == Kind::Entry => {
                    *p = Point {
                        addr: a,
                        byte: img.bytes(a, 1)[0],
                        kind,
                        func: r.addr,
                    };
                    st.shared += 1;
                }
                Some(_) => st.shared += 1,
                None => {
                    out.insert(
                        a,
                        Point {
                            addr: a,
                            byte: img.bytes(a, 1)[0],
                            kind,
                            func: r.addr,
                        },
                    );
                }
            }
        }
    }
    let v: Vec<Point> = out.into_values().collect();
    st.blocks = v.iter().filter(|p| p.kind == Kind::Block).count();
    (v, st)
}

/// The points file read by sa-hook (`ADDR BYTE KIND FUNC`).
pub fn points_text(points: &[Point]) -> String {
    let mut s = String::from("# sa-oracle coverage points: ADDR BYTE KIND(F=entry,B=block) FUNC\n");
    for p in points {
        s += &format!(
            "{:#x} {:02x} {} {:#x}\n",
            p.addr,
            p.byte,
            if p.kind == Kind::Entry { "F" } else { "B" },
            p.func
        );
    }
    s
}

/// A point left unarmed by `catalog/coverage-exclude.csv` (its breakpoint changes the game, e.g. the
/// protection reads its bytes as data). `proxy`: a point that executes exactly when this one does (the
/// target of the `jmp rel32` at this point when that target has no other way in); the report counts
/// the excluded point with the proxy's hits. No proxy: the point stays unmeasured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Excluded {
    pub point: Point,
    pub proxy: Option<u32>,
}

/// `catalog/coverage-exclude.csv` (`address,proxy,reason`; proxy may be empty): address -> proxy.
pub fn parse_exclude_csv(text: &str) -> BTreeMap<u32, Option<u32>> {
    text.lines()
        .skip(1)
        .filter_map(|l| {
            let mut f = l.split(',');
            let a = hex(f.next()?.trim())?;
            Some((a, f.next().and_then(|s| hex(s.trim()))))
        })
        .collect()
}

/// Comment lines appended to the points file for the excluded points (sa-hook skips `#` lines):
/// `#X ADDR BYTE KIND FUNC PROXY|-`.
pub fn excluded_text(ex: &[Excluded]) -> String {
    let mut s = String::from("# excluded (catalog/coverage-exclude.csv, not armed): #X ADDR BYTE KIND FUNC PROXY\n");
    for e in ex {
        let p = &e.point;
        s += &format!(
            "#X {:#x} {:02x} {} {:#x} {}\n",
            p.addr,
            p.byte,
            if p.kind == Kind::Entry { "F" } else { "B" },
            p.func,
            e.proxy.map(|x| format!("{x:#x}")).unwrap_or_else(|| "-".into())
        );
    }
    s
}

pub fn parse_excluded_text(text: &str) -> Result<Vec<Excluded>, String> {
    let mut v = Vec::new();
    for (n, l) in text.lines().enumerate() {
        let Some(l) = l.strip_prefix("#X ") else { continue };
        let f: Vec<&str> = l.split_whitespace().collect();
        let bad = || format!("points line {}: {l:?}", n + 1);
        if f.len() < 5 {
            return Err(bad());
        }
        v.push(Excluded {
            point: Point {
                addr: hex(f[0]).ok_or_else(bad)?,
                byte: hex(f[1]).filter(|&b| b < 256).ok_or_else(bad)? as u8,
                kind: if f[2] == "F" { Kind::Entry } else { Kind::Block },
                func: hex(f[3]).ok_or_else(bad)?,
            },
            proxy: hex(f[4]),
        });
    }
    Ok(v)
}

/// Adds the excluded points to `points` and gives each one with a measured proxy the proxy's hits.
/// Returns the extended points, the hits and per excluded point its derived hits (None: unmeasured).
pub fn with_excluded(
    points: &[Point],
    hits: &BTreeMap<u32, u64>,
    ex: &[Excluded],
) -> (Vec<Point>, BTreeMap<u32, u64>, Vec<Option<u64>>) {
    let mut pts = points.to_vec();
    let mut h = hits.clone();
    let mut derived = Vec::new();
    for e in ex {
        let d = e.proxy.and_then(|p| hits.get(&p).copied());
        if let Some(x) = d {
            h.insert(e.point.addr, x);
        }
        pts.push(e.point.clone());
        derived.push(d);
    }
    (pts, h, derived)
}

/// Report section for the excluded points (none silently dropped).
pub fn excluded_report(ex: &[Excluded], derived: &[Option<u64>], rows: &[Row]) -> String {
    let name = |a: u32| {
        rows.iter()
            .find(|r| r.addr == a)
            .map(|r| r.name.clone())
            .unwrap_or_default()
    };
    let n = ex.len();
    let entries = ex.iter().filter(|e| e.point.kind == Kind::Entry).count();
    let funcs: BTreeSet<u32> = ex.iter().map(|e| e.point.func).collect();
    let by_proxy = derived.iter().filter(|d| d.is_some()).count();
    let hit = derived.iter().filter(|d| d.is_some_and(|x| x > 0)).count();
    let mut s = format!(
        "\npoints excluded from arming (catalog/coverage-exclude.csv): {n} ({entries} entries, {} blocks) in {} \
         functions; counted through their proxy point: {by_proxy} ({hit} hit); unmeasured: {}\n",
        n - entries,
        funcs.len(),
        n - by_proxy
    );
    for (e, d) in ex.iter().zip(derived) {
        s += &format!(
            "  {:#x} {} of {:#x} {:<40} proxy {:<9} {}\n",
            e.point.addr,
            if e.point.kind == Kind::Entry { "entry" } else { "block" },
            e.point.func,
            name(e.point.func),
            e.proxy.map(|x| format!("{x:#x}")).unwrap_or_else(|| "-".into()),
            match d {
                Some(x) => format!("hits {x}"),
                None => "unmeasured".into(),
            }
        );
    }
    s
}

pub fn parse_points_text(text: &str) -> Result<Vec<Point>, String> {
    let mut v = Vec::new();
    for (n, l) in text.lines().enumerate() {
        let l = l.split('#').next().unwrap_or("").trim();
        if l.is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split_whitespace().collect();
        let bad = || format!("points line {}: {l:?}", n + 1);
        if f.len() < 4 {
            return Err(bad());
        }
        v.push(Point {
            addr: hex(f[0]).ok_or_else(bad)?,
            byte: hex(f[1]).filter(|&b| b < 256).ok_or_else(bad)? as u8,
            kind: if f[2] == "F" { Kind::Entry } else { Kind::Block },
            func: hex(f[3]).ok_or_else(bad)?,
        });
    }
    Ok(v)
}

/// `address,hits` CSV (sa-hook output, or a merged file).
pub fn parse_hits(text: &str) -> Result<BTreeMap<u32, u64>, String> {
    let mut m = BTreeMap::new();
    for (n, l) in text.lines().enumerate() {
        if n == 0 && l.starts_with("address") {
            continue;
        }
        let mut f = l.split(',');
        let (Some(a), Some(h)) = (
            f.next().and_then(hex),
            f.next().and_then(|s| s.trim().parse::<u64>().ok()),
        ) else {
            if l.trim().is_empty() {
                continue;
            }
            return Err(format!("hits line {}: {l:?}", n + 1));
        };
        *m.entry(a).or_insert(0) += h;
    }
    Ok(m)
}

/// Merges hit maps: union of addresses, hits summed.
pub fn merge(maps: &[BTreeMap<u32, u64>]) -> BTreeMap<u32, u64> {
    let mut m = BTreeMap::new();
    for x in maps {
        for (&a, &h) in x {
            *m.entry(a).or_insert(0) += h;
        }
    }
    m
}

pub fn hits_text(m: &BTreeMap<u32, u64>) -> String {
    let mut s = String::from("address,hits\n");
    for (a, h) in m {
        s += &format!("{a:#x},{h}\n");
    }
    s
}

/// Per-function coverage.
#[derive(Debug, Clone, Default)]
pub struct FnCov {
    pub entry_hits: u64,
    pub blocks_hit: usize,
    pub blocks_total: usize,
    /// The function was armed in the run(s).
    pub measured: bool,
}

pub struct Coverage {
    pub funcs: BTreeMap<u32, FnCov>,
}

impl Coverage {
    /// `hits` holds every armed point (0 = armed, never executed); points missing from `hits` were not
    /// armed in the run (byte mismatch or outside the image) and count as unmeasured.
    pub fn new(points: &[Point], hits: &BTreeMap<u32, u64>) -> Coverage {
        let mut funcs: BTreeMap<u32, FnCov> = BTreeMap::new();
        for p in points {
            let Some(&h) = hits.get(&p.addr) else { continue };
            let f = funcs.entry(p.func).or_default();
            f.blocks_total += 1;
            f.blocks_hit += (h > 0) as usize;
            if p.kind == Kind::Entry {
                f.entry_hits = h;
                f.measured = true;
            }
        }
        Coverage { funcs }
    }

    /// `address,hits,blocks_hit,blocks_total,subsystem,class,state,name` per measured function.
    pub fn function_csv(&self, rows: &[Row]) -> String {
        let by: BTreeMap<u32, &Row> = rows.iter().map(|r| (r.addr, r)).collect();
        let mut s = String::from("address,hits,blocks_hit,blocks_total,subsystem,class,state,name\n");
        for (a, f) in self.funcs.iter().filter(|(_, f)| f.measured) {
            let r = by.get(a);
            s += &format!(
                "{a:#x},{},{},{},{},{},{},{}\n",
                f.entry_hits,
                f.blocks_hit,
                f.blocks_total,
                r.map(|r| r.subsystem.as_str()).unwrap_or(""),
                r.map(|r| r.class.as_str()).unwrap_or(""),
                r.map(|r| r.state.as_str()).unwrap_or(""),
                r.map(|r| r.name.as_str()).unwrap_or(""),
            );
        }
        s
    }

    /// Per-subsystem report: functions hit / non-excluded catalogue total, blocks hit / armed blocks.
    pub fn report(&self, rows: &[Row], title: &str) -> String {
        #[derive(Default)]
        struct Agg {
            total: usize,
            measured: usize,
            hit: usize,
            bhit: usize,
            btotal: usize,
        }
        let mut per: BTreeMap<String, Agg> = BTreeMap::new();
        let mut excl: BTreeMap<String, Agg> = BTreeMap::new();
        let (mut all, mut all_x) = (Agg::default(), Agg::default());
        for r in rows {
            let (map, tot) = if r.excluded() {
                (&mut excl, &mut all_x)
            } else {
                (&mut per, &mut all)
            };
            let key = if r.excluded() {
                r.class.clone()
            } else {
                r.subsystem.clone()
            };
            let a = map.entry(key).or_default();
            a.total += 1;
            tot.total += 1;
            if let Some(f) = self.funcs.get(&r.addr).filter(|f| f.measured) {
                for x in [&mut *a, &mut *tot] {
                    x.measured += 1;
                    x.hit += (f.entry_hits > 0) as usize;
                    x.bhit += f.blocks_hit;
                    x.btotal += f.blocks_total;
                }
            }
        }
        let pct = |a: usize, b: usize| if b == 0 { 0.0 } else { 100.0 * a as f64 / b as f64 };
        let line = |name: &str, a: &Agg| {
            format!(
                "{name:<44} {:>6} / {:<6} {:>5.1}%   (measured {:>6})   blocks {:>7} / {:<7} {:>5.1}%\n",
                a.hit,
                a.total,
                pct(a.hit, a.total),
                a.measured,
                a.bhit,
                a.btotal,
                pct(a.bhit, a.btotal)
            )
        };
        let mut s = format!("{title}\n");
        s += &format!(
            "{:<44} {:>15} {:>7}   {:>17}   {}\n",
            "subsystem (non-excluded game code)", "functions hit/total", "", "", "blocks hit/armed"
        );
        let mut keys: Vec<&String> = per.keys().collect();
        keys.sort_by_key(|k| std::cmp::Reverse(per[*k].total));
        for k in keys {
            s += &line(k, &per[k]);
        }
        s += &line("TOTAL non-excluded", &all);
        s += "\nexcluded classes (not part of F1/F8; listed for F2)\n";
        for (k, a) in &excl {
            s += &line(k, a);
        }
        s += &line("TOTAL excluded", &all_x);
        s
    }
}

pub const USAGE: &str = "usage: sa-oracle coverage points [--out FILE] [--no-blocks]\n       \
    sa-oracle coverage report --points FILE --hits A.csv[,B.csv...] [--title T] [--csv FUNCS.csv] \
    [--merged HITS.csv] [--out REPORT]";

/// `sa-oracle coverage points|report`.
pub fn cli(args: &[String]) -> Result<bool, String> {
    let root = crate::workspace_root();
    let read = |p: &std::path::Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let rows = parse_procedures(&read(&root.join("catalog/procedures.csv"))?)?;
    let (o, flags) = crate::l3::parse_opts(args.get(1..).unwrap_or(&[]))?;
    let write = |k: &str, text: &str| -> Result<(), String> {
        if let Some(p) = o.get(k) {
            std::fs::write(p, text).map_err(|e| format!("{p}: {e}"))?;
        }
        Ok(())
    };
    match args.first().map(String::as_str) {
        Some("points") => {
            let img = crate::image_cached().ok_or("gta_sa.exe 1.0 US not found (set SA_GTA_EXE)")?;
            let ext = parse_extents(&read(&root.join("catalog/extents.csv"))?)?;
            let (mut p, st) = points(&img, &rows, &ext, !flags.contains("no-blocks"));
            // Hand-maintained: points whose breakpoint changes the game (catalog/coverage-exclude.csv,
            // each with its evidence). They stay in the file as `#X` comment lines for the report.
            let excl = parse_exclude_csv(
                &std::fs::read_to_string(root.join("catalog/coverage-exclude.csv")).unwrap_or_default(),
            );
            let mut ex = Vec::new();
            p.retain(|x| match excl.get(&x.addr) {
                Some(&proxy) => {
                    ex.push(Excluded { point: x.clone(), proxy });
                    false
                }
                None => true,
            });
            for e in &mut ex {
                if e.proxy.is_some_and(|q| !p.iter().any(|x| x.addr == q)) {
                    eprintln!("coverage-exclude: proxy {:#x?} of {:#x} is not a point; unmeasured", e.proxy, e.point.addr);
                    e.proxy = None;
                }
            }
            let missing: Vec<String> = excl
                .keys()
                .filter(|a| !ex.iter().any(|e| e.point.addr == **a))
                .map(|a| format!("{a:#x}"))
                .collect();
            eprintln!(
                "points excluded by catalog/coverage-exclude.csv: {} ({} with a proxy){}",
                ex.len(),
                ex.iter().filter(|e| e.proxy.is_some()).count(),
                if missing.is_empty() { String::new() } else { format!("; not points: {}", missing.join(" ")) }
            );
            eprintln!(
                "points: {} ({} entries, {} blocks); functions armed {} ({} with body ranges, {} entry only), \
                 not armed {} (protection/mis-split/other region), entry not decodable {}; jump tables {}, \
                 unresolved indirect jumps {}, shared {}",
                p.len(),
                p.len() - st.blocks,
                st.blocks,
                st.functions - st.entry_bad,
                st.with_body,
                st.entry_only,
                st.not_armed,
                st.entry_bad,
                st.tables,
                st.unresolved_indirect,
                st.shared
            );
            let t = points_text(&p) + &excluded_text(&ex);
            match o.get("out") {
                Some(_) => write("out", &t)?,
                None => print!("{t}"),
            }
            Ok(true)
        }
        Some("report") => {
            let ptext = read(std::path::Path::new(o.get("points").ok_or(USAGE)?))?;
            let pts = parse_points_text(&ptext)?;
            let ex = parse_excluded_text(&ptext)?;
            let mut maps = Vec::new();
            for h in o.get("hits").ok_or(USAGE)?.split(',').filter(|s| !s.is_empty()) {
                maps.push(parse_hits(&read(std::path::Path::new(h))?)?);
            }
            let hits = merge(&maps);
            let (all_pts, all_hits, derived) = with_excluded(&pts, &hits, &ex);
            let c = Coverage::new(&all_pts, &all_hits);
            let title = o.get("title").cloned().unwrap_or_else(|| {
                format!(
                    "coverage of {} run(s): {}",
                    maps.len(),
                    o.get("hits").cloned().unwrap_or_default()
                )
            });
            let rep = c.report(&rows, &title) + &excluded_report(&ex, &derived, &rows);
            print!("{rep}");
            write("out", &rep)?;
            write("csv", &c.function_csv(&rows))?;
            write("merged", &hits_text(&hits))?;
            Ok(true)
        }
        _ => Err(USAGE.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-assembled function at 0x1000:
    /// ```text
    /// 1000: 85 c0          test eax,eax
    /// 1002: 74 04          je 1008
    /// 1004: 31 c0          xor eax,eax
    /// 1006: eb 08          jmp 1010
    /// 1008: ff 24 8d 20 10 00 00   jmp [ecx*4+0x1020]
    /// 100f: cc
    /// 1010: c3             ret
    /// 1011..1020: cc padding
    /// 1020: table {0x1004, 0x1010}
    /// ```
    fn image() -> Vec<u8> {
        let mut m = vec![0xCCu8; 0x40];
        m[..0x0F].copy_from_slice(&[
            0x85, 0xC0, 0x74, 0x04, 0x31, 0xC0, 0xEB, 0x08, 0xFF, 0x24, 0x8D, 0x20, 0x10, 0x00, 0x00,
        ]);
        m[0x10] = 0xC3;
        m[0x20..0x28].copy_from_slice(&[0x04, 0x10, 0, 0, 0x10, 0x10, 0, 0]);
        m
    }

    #[test]
    fn decodes_blocks_and_jump_tables() {
        let m = image();
        let code = |a: u32| -> &[u8] {
            let o = (a - 0x1000) as usize;
            if o >= m.len() {
                &[]
            } else {
                &m[o..(o + 16).min(m.len())]
            }
        };
        let d = decode_function(0x1000, &[(0x1000, 0x1011)], code);
        assert_eq!(d.blocks, vec![0x1000, 0x1004, 0x1008, 0x1010]);
        assert_eq!((d.tables, d.unresolved_indirect, d.bad), (1, 0, 0));
        // a function entered in the middle: its blocks only
        let d = decode_function(0x1004, &[(0x1000, 0x1011)], code);
        assert_eq!(d.blocks, vec![0x1004, 0x1010]);
        // the table itself is outside the body; a body that wrongly covers it stops the table read
        let d = decode_function(0x1000, &[(0x1000, 0x1030)], code);
        assert_eq!(d.tables, 0);
        assert_eq!(d.unresolved_indirect, 1);
        // a range too short for the entry instruction: nothing
        let d = decode_function(0x1000, &[(0x1000, 0x1001)], code);
        assert!(d.blocks.is_empty());
    }

    #[test]
    fn coverage_report_and_merge() {
        let pts = vec![
            Point {
                addr: 0x1000,
                byte: 0x85,
                kind: Kind::Entry,
                func: 0x1000,
            },
            Point {
                addr: 0x1004,
                byte: 0x31,
                kind: Kind::Block,
                func: 0x1000,
            },
            Point {
                addr: 0x2000,
                byte: 0x55,
                kind: Kind::Entry,
                func: 0x2000,
            },
        ];
        let t = points_text(&pts);
        assert_eq!(parse_points_text(&t).unwrap(), pts);
        // sa-hook's own parser reads the same file
        let hp = sa_hook::coverage::parse_points(&t).unwrap();
        assert_eq!(hp.len(), 3);
        assert_eq!((hp[2].addr, hp[2].byte), (0x2000, 0x55));
        let a = parse_hits("address,hits\n0x1000,1\n0x1004,0\n0x2000,0\n").unwrap();
        let b = parse_hits("address,hits\n0x1000,1\n0x1004,1\n0x2000,0\n").unwrap();
        let m = merge(&[a.clone(), b]);
        assert_eq!(hits_text(&m), "address,hits\n0x1000,2\n0x1004,1\n0x2000,0\n");
        let rows = parse_procedures(
            "address,region,state,gr_name,ghidra_name,thunk,subsystem,class,name\n\
             0x1000,game-text,gr-reversed,CFoo::Bar,,no,PED,game,\n\
             0x2000,game-text,unknown,,FUN_2000,no,PED,game,\n\
             0x3000,game-text,excluded,,FUN_3000,no,X-crt,excluded:crt,\n",
        )
        .unwrap();
        let c = Coverage::new(&pts, &a);
        assert_eq!(c.funcs[&0x1000].blocks_hit, 1);
        let csv = c.function_csv(&rows);
        assert!(
            csv.contains("0x1000,1,1,2,PED,game,gr-reversed,CFoo::Bar\n"),
            "{csv}"
        );
        let rep = c.report(&rows, "t");
        assert!(rep.contains("PED"), "{rep}");
        assert!(
            rep.lines()
                .any(|l| l.starts_with("TOTAL non-excluded") && l.contains("1 / 2")),
            "{rep}"
        );
        assert!(
            rep.lines()
                .any(|l| l.starts_with("excluded:crt") && l.contains("0 / 1")),
            "{rep}"
        );
    }

    #[test]
    fn excluded_points_counted_through_proxy() {
        let ex_csv = "address,proxy,reason\n0x1000,0x1004,\"jmp stub, key read\"\n0x2000,,no proxy\n";
        let m = parse_exclude_csv(ex_csv);
        assert_eq!(m[&0x1000], Some(0x1004));
        assert_eq!(m[&0x2000], None);
        let pts = vec![Point {
            addr: 0x1004,
            byte: 0x31,
            kind: Kind::Block,
            func: 0x1000,
        }];
        let ex = vec![
            Excluded {
                point: Point {
                    addr: 0x1000,
                    byte: 0xE9,
                    kind: Kind::Entry,
                    func: 0x1000,
                },
                proxy: Some(0x1004),
            },
            Excluded {
                point: Point {
                    addr: 0x2000,
                    byte: 0xE9,
                    kind: Kind::Entry,
                    func: 0x2000,
                },
                proxy: None,
            },
        ];
        let t = points_text(&pts) + &excluded_text(&ex);
        assert_eq!(parse_points_text(&t).unwrap(), pts);
        assert_eq!(parse_excluded_text(&t).unwrap(), ex);
        // sa-hook arms only the real points
        assert_eq!(sa_hook::coverage::parse_points(&t).unwrap().len(), 1);
        let hits = parse_hits("address,hits\n0x1004,1\n").unwrap();
        let (p2, h2, d) = with_excluded(&pts, &hits, &ex);
        assert_eq!(d, vec![Some(1), None]);
        let c = Coverage::new(&p2, &h2);
        assert!(c.funcs[&0x1000].measured && c.funcs[&0x1000].entry_hits == 1);
        assert!(!c.funcs.get(&0x2000).is_some_and(|f| f.measured));
        let r = excluded_report(&ex, &d, &[]);
        assert!(r.contains("2 (2 entries, 0 blocks) in 2 functions"), "{r}");
        assert!(r.contains("proxy point: 1 (1 hit); unmeasured: 1"), "{r}");
    }

    /// The real catalogue and exe (when present): entries decode, points are instruction starts.
    #[test]
    fn real_points() {
        let Some(img) = crate::image_cached() else { return };
        let root = crate::workspace_root();
        let rows =
            parse_procedures(&std::fs::read_to_string(root.join("catalog/procedures.csv")).unwrap()).unwrap();
        let ext = parse_extents(&std::fs::read_to_string(root.join("catalog/extents.csv")).unwrap()).unwrap();
        let (p, st) = points(&img, &rows, &ext, true);
        assert!(st.functions > 15_000, "{st:?}");
        assert!(st.entry_bad < st.functions / 50, "{st:?}");
        assert!(st.blocks > 100_000, "{st:?}");
        // every hook entry of sa-hook's table is a point (an entry) unless the catalogue lacks it
        for h in sa_hook::table::HOOKS.iter() {
            if rows.iter().any(|r| r.addr == h.addr && !r.not_armed()) {
                assert!(
                    p.iter().any(|x| x.addr == h.addr && x.kind == Kind::Entry),
                    "{}",
                    h.name
                );
            }
        }
    }
}
