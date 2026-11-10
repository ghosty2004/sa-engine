//! `sa-oracle` CLI (see main.rs).
//!
//! ```text
//! sa-oracle list
//! sa-oracle l1 <fn|all> [--cases N] [--seed S] [--threads T] [--backend auto|unicorn|native]
//!                       [--cw 0x7f] [--mxcsr 0x9fa0] [--no-fixtures]
//! sa-oracle fidelity [--cases N] [--fn-cases N]
//! sa-oracle mutants [--cases N]          (needs --features mutants)
//! sa-oracle l3 --rec R --runner CMD [--rust ports] [--hookoff all]   (INF-9, see l3.rs)
//! sa-oracle coverage points|report ...                              (INF-10, see coverage.rs)
//! ```
//!
//! `--backend auto` (default): Unicorn for functions routed to L1; for functions routed to L2 the
//! native backend (real CPU under Wine) when available, Unicorn otherwise (marked not authoritative).

use sa_oracle::call::Fpu;
use sa_oracle::runner::{self, Backend, Cfg};
use sa_oracle::specs::{self, Route, Spec};
use std::process::ExitCode;

fn parse_num(s: &str) -> Result<u64, String> {
    let s = s.replace('_', "");
    match s.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => s.parse(),
    }
    .map_err(|e| format!("{s}: {e}"))
}

struct Args {
    pos: Vec<String>,
    opts: std::collections::HashMap<String, String>,
    flags: Vec<String>,
}

fn args() -> Args {
    let mut a = Args {
        pos: vec![],
        opts: Default::default(),
        flags: vec![],
    };
    let v: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < v.len() {
        if let Some(k) = v[i].strip_prefix("--") {
            if k.starts_with("no-") || i + 1 >= v.len() {
                a.flags.push(k.to_string());
            } else {
                a.opts.insert(k.to_string(), v[i + 1].clone());
                i += 1;
            }
        } else {
            a.pos.push(v[i].clone());
        }
        i += 1;
    }
    a
}

fn cfg_from(a: &Args) -> Result<Cfg, String> {
    let mut c = Cfg::default();
    if let Some(v) = a.opts.get("cases") {
        c.cases = parse_num(v)?;
    }
    if let Some(v) = a.opts.get("seed") {
        c.seed = parse_num(v)?;
    }
    if let Some(v) = a.opts.get("threads") {
        c.threads = parse_num(v)? as usize;
    }
    let mut fpu = Fpu::default();
    if let Some(v) = a.opts.get("cw") {
        fpu.cw = parse_num(v)? as u16;
    }
    if let Some(v) = a.opts.get("mxcsr") {
        fpu.mxcsr = parse_num(v)? as u32;
    }
    c.fpu = fpu;
    c.fixtures = !a.flags.iter().any(|f| f == "no-fixtures");
    Ok(c)
}

fn backend_for(spec: &Spec, a: &Args) -> Result<Backend, String> {
    match a.opts.get("backend").map(String::as_str).unwrap_or("auto") {
        "unicorn" => Ok(Backend::Unicorn),
        "native" => Ok(Backend::Native),
        "auto" => Ok(match spec.route {
            Route::L2 { .. } if sa_oracle::native::available() => Backend::Native,
            _ => Backend::Unicorn,
        }),
        b => Err(format!("unknown backend {b}")),
    }
}

fn l1(a: &Args) -> Result<bool, String> {
    let img = sa_oracle::image_cached().ok_or("gta_sa.exe 1.0 US not found (set SA_GTA_EXE)")?;
    let which = a.pos.get(1).ok_or("usage: l1 <fn|all>")?;
    let list: Vec<Spec> = if which == "all" {
        specs::all()
    } else {
        vec![specs::by_key(which).ok_or(format!("unknown function {which} (see `list`)"))?]
    };
    let base = cfg_from(a)?;
    println!(
        "L1 runner: seed {:#x}, {} threads, cw {:#06x}, mxcsr {:#010x}",
        base.seed, base.threads, base.fpu.cw, base.fpu.mxcsr
    );
    let mut ok = true;
    for spec in &list {
        let mut cfg = base.clone();
        cfg.backend = backend_for(spec, a)?;
        let rep = runner::run(spec, &img, &cfg)?;
        print!("{}", runner::print_report(spec, &rep));
        ok &= rep.mismatches == 0;
    }
    Ok(ok)
}

fn fidelity(a: &Args) -> Result<bool, String> {
    let img = sa_oracle::image_cached().ok_or("gta_sa.exe 1.0 US not found (set SA_GTA_EXE)")?;
    if !sa_oracle::native::available() {
        return Err("the native backend needs i686-w64-mingw32-gcc, wine and the wine prefix".into());
    }
    let n = a
        .opts
        .get("cases")
        .map(|v| parse_num(v))
        .transpose()?
        .unwrap_or(20_000);
    let m = sa_oracle::fidelity::nan_matrix(&img)?;
    println!(
        "NaN/special-value matrix ({} forms x {} operand pairs = {} cases): sa_math::x87 model vs CPU: {} mismatches; Unicorn vs CPU: {} ST0 differences",
        sa_oracle::fidelity::FORMS.len(),
        sa_oracle::fidelity::MATRIX_VALUES.len().pow(2),
        m.cases,
        m.model_mismatches.len(),
        m.unicorn_mismatches
    );
    for x in m.model_mismatches.iter().take(20) {
        println!("  model: {x}");
    }
    if let Some(e) = &m.unicorn_example {
        println!("  unicorn e.g. {e}");
    }
    let ftz = !a.flags.iter().any(|f| f == "no-ftz-workaround");
    println!(
        "x87 probes: Unicorn vs real CPU (Wine), {n} cases per row, MXCSR {:#06x}, FZ workaround {}",
        sa_oracle::call::GAME_MXCSR,
        if ftz { "on" } else { "off" }
    );
    let rows = sa_oracle::fidelity::measure(&img, n, &[0x007F, 0x027F, 0x037F], 0xF1DE, ftz)?;
    print!("{}", sa_oracle::fidelity::print_rows(&rows));
    if a.flags.iter().any(|f| f == "no-functions") {
        return Ok(true);
    }
    let fn_cases = a
        .opts
        .get("fn-cases")
        .map(|v| parse_num(v))
        .transpose()?
        .unwrap_or(200_000);
    println!("\nfunctions: Unicorn vs real CPU, {fn_cases} generated cases + edges each (game FPU state)");
    for spec in specs::all() {
        let mut cfg = cfg_from(a)?;
        cfg.cases = fn_cases;
        cfg.backend = Backend::Native;
        cfg.fixtures = false;
        cfg.shrink = false;
        let rep = runner::run(&spec, &img, &cfg)?;
        println!(
            "  {:<40} route={:<3} on CPU={:>7}  unicorn!=cpu observable: {:>6} {:?}  flags only: {:>6}  (rust vs cpu mismatches: {})",
            spec.name,
            rep.route,
            rep.both,
            rep.unicorn_vs_cpu,
            rep.unicorn_vs_cpu_kinds,
            rep.unicorn_vs_cpu_flags_only,
            rep.mismatches
        );
    }
    Ok(true)
}

fn hook_indices(list: Option<&String>) -> Result<Vec<usize>, String> {
    let Some(list) = list else { return Ok(Vec::new()) };
    list.split(',')
        .filter(|k| !k.is_empty())
        .map(|k| sa_hook::table::find(k).ok_or(format!("unknown hook {k}")))
        .collect()
}

fn fixtures_dir(a: &Args) -> std::path::PathBuf {
    a.opts
        .get("fixtures")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| sa_oracle::workspace_root().join("fixtures/l2"))
}

fn read_spool(p: &str) -> Result<Vec<sa_oracle_format::l2::Case>, String> {
    read_spool_full(p).map(|s| s.0)
}

/// The cases and the quota trailer (INF-18) of a spool.
fn read_spool_full(p: &str) -> Result<(Vec<sa_oracle_format::l2::Case>, Option<String>), String> {
    let b = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
    sa_oracle_format::l2::parse_spool_full(&b).map_err(|e| format!("{p}: {e}"))
}

/// The cases of the functions at `only` (and the quota trailer) of a spool, read record by record, so
/// a multi-GB split-run spool does not have to fit in memory (`split-report --fn`).
fn read_spool_only(
    p: &str,
    only: &[u32],
) -> Result<(Vec<sa_oracle_format::l2::Case>, Option<String>), String> {
    use std::io::Read;
    let f = std::fs::File::open(p).map_err(|e| format!("{p}: {e}"))?;
    let mut r = std::io::BufReader::with_capacity(1 << 20, f);
    let mut magic = [0u8; 8];
    r.read_exact(&mut magic).map_err(|e| format!("{p}: {e}"))?;
    // the header check of parse_spool_full, on the magic alone
    sa_oracle_format::l2::parse_spool_full(&magic).map_err(|e| format!("{p}: {e}"))?;
    let (mut out, mut buf, mut at) = (Vec::new(), Vec::new(), 8u64);
    loop {
        let mut h = [0u8; 4];
        match r.read_exact(&mut h) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok((out, None)),
            Err(e) => return Err(format!("{p}: {e}")),
        }
        let n = u32::from_le_bytes(h);
        if n == sa_oracle_format::l2::TRAILER_MARK {
            let mut k = [0u8; 4];
            r.read_exact(&mut k)
                .map_err(|_| format!("{p}: truncated trailer"))?;
            let mut t = vec![0u8; u32::from_le_bytes(k) as usize];
            r.read_exact(&mut t)
                .map_err(|_| format!("{p}: truncated trailer"))?;
            return Ok((out, Some(String::from_utf8_lossy(&t).into_owned())));
        }
        buf.resize(n as usize, 0);
        r.read_exact(&mut buf)
            .map_err(|_| format!("{p}: truncated record at {at}"))?;
        // fn_addr follows the 8-byte case magic (Case::decode)
        if buf.len() >= 12 && only.contains(&u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]])) {
            out.push(
                sa_oracle_format::l2::Case::decode(&buf).map_err(|e| format!("{p}: record at {at}: {e}"))?,
            );
        }
        at += 4 + n as u64;
    }
}

/// `l2 spec|ingest|report|split-report|replay|show`
fn l2(a: &Args) -> Result<bool, String> {
    use sa_oracle::l2 as l2h;
    let root = sa_oracle::workspace_root();
    let cat = l2h::RwCatalog::load(&root)?;
    match a.pos.get(1).map(String::as_str) {
        Some("spec") => {
            let t = l2h::spec_text(&cat, &hook_indices(a.opts.get("hooks"))?);
            match a.opts.get("out") {
                Some(p) => std::fs::write(p, &t).map_err(|e| format!("{p}: {e}"))?,
                None => print!("{t}"),
            }
            Ok(true)
        }
        Some("ingest") => {
            let p = a.pos.get(2).ok_or("usage: l2 ingest SPOOL [--fixtures DIR]")?;
            let cases = read_spool(p)?;
            if cases.iter().any(|c| c.is_split()) {
                return Err(format!("{p} is a split-run spool (one outcome per case); not ingested"));
            }
            let dir = fixtures_dir(a);
            let r = l2h::ingest(&cases, &dir)?;
            println!(
                "ingested {}: {} cases, {} new fixtures ({} bytes zstd), {} already stored, into {}",
                p,
                r.cases,
                r.new,
                r.bytes,
                r.dup,
                dir.display()
            );
            Ok(true)
        }
        Some("report") => {
            let p = a.pos.get(2).ok_or("usage: l2 report SPOOL")?;
            let (cases, trailer) = read_spool_full(p)?;
            if cases.iter().any(|c| c.is_split()) {
                return Err(format!("{p} is a split-run spool: use `l2 split-report PLAIN RUST`"));
            }
            let t = l2h::tally(&cases, false);
            println!("L2 report {p}: {} cases (stored verdicts: original vs in-process Rust)", cases.len());
            if let Some(t) = &trailer {
                println!("TRAILER {t}");
            }
            print!("{}", l2h::print_tally(&t, Some(&cat)));
            Ok(t.values().all(|x| x.mismatches == 0))
        }
        Some("dump-diff") => {
            let (Some(x), Some(y)) = (a.pos.get(2), a.pos.get(3)) else {
                return Err("usage: l2 dump-diff A.win B.win".into());
            };
            let rd = |p: &String| std::fs::read(p).map_err(|e| format!("{p}: {e}"));
            let d = sa_oracle::split::dump_diff(&rd(x)?, &rd(y)?)?;
            for (addr, ba, bb) in &d {
                println!("{addr:#x} {} plain {} rust {}", cat.symbol(*addr), ba, bb);
            }
            println!("{} differing range(s)", d.len());
            Ok(d.is_empty())
        }
        Some("split-report") => {
            let (Some(pp), Some(rp)) = (a.pos.get(2), a.pos.get(3)) else {
                return Err("usage: l2 split-report PLAIN_SPOOL RUST_SPOOL".into());
            };
            // --fn A,B: only those functions' cases (read record by record; for spools larger than memory)
            let ((plain, tp), (rust, tr)) = match a.opts.get("fn") {
                Some(k) => {
                    let only: Vec<u32> =
                        hook_indices(Some(k))?.iter().map(|&i| sa_hook::table::HOOKS[i].addr).collect();
                    (read_spool_only(pp, &only)?, read_spool_only(rp, &only)?)
                }
                None => (read_spool_full(pp)?, read_spool_full(rp)?),
            };
            let mut rep = sa_oracle::split::split_report(&plain, &rust);
            rep.trailers.extend(tp.map(|t| format!("plain: {t}")));
            rep.trailers.extend(tr.map(|t| format!("rust: {t}")));
            println!(
                "L2 split-run report: plain {pp} ({} cases), rust {rp} ({} cases)",
                plain.len(),
                rust.len()
            );
            print!("{}", sa_oracle::split::print_report(&rep));
            if a.flags.iter().any(|f| f == "nesting") || a.opts.contains_key("nesting") {
                for ((o, i), n) in sa_oracle::split::nesting(&plain) {
                    println!("nested (plain run): {i} inside {o}: {n} case(s)");
                }
            }
            if let Some(n) = a.opts.get("timeline") {
                print!("{}", sa_oracle::split::print_timeline(&rep, parse_num(n)? as usize));
            }
            Ok(rep.errors.is_empty()
                && rep.trailers.is_empty()
                && rep.fns.iter().all(|f| matches!(f.verdict(), "PASS" | "PASS-FP" | "NOCASES")))
        }
        Some("replay") => {
            if let Some(m) = a.opts.get("mutant") {
                sa_hook::l2::set_mutants(&hook_indices(Some(m))?);
            }
            let only = match a.opts.get("fn") {
                Some(k) => Some(sa_hook::table::HOOKS[*hook_indices(Some(k))?.first().ok_or("--fn")?].addr),
                None => None,
            };
            let dir = fixtures_dir(a);
            let cases: Vec<_> = l2h::load_fixtures(&dir, only)?.into_iter().map(|(_, c)| c).collect();
            let t = l2h::tally(&cases, true);
            println!(
                "L2 host replay of {} stored cases from {} (Rust recomputed on the host, original outputs from the fixtures)",
                cases.len(),
                dir.display()
            );
            print!("{}", l2h::print_tally(&t, None));
            Ok(!cases.is_empty() && t.values().all(|x| x.mismatches == 0))
        }
        Some("show") => {
            let p = a.pos.get(2).ok_or("usage: l2 show SPOOL|FIXTURE [N]")?;
            let b = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
            let cases = if p.ends_with(".zst") {
                let d = zstd::decode_all(&b[..]).map_err(|e| e.to_string())?;
                vec![sa_oracle_format::l2::Case::decode(&d)?]
            } else {
                sa_oracle_format::l2::parse_spool(&b)?
            };
            let n: usize = a.pos.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
            let c = cases.get(n).ok_or("no such case")?;
            print!("{}", l2h::show_case(c));
            Ok(true)
        }
        _ => Err("usage: l2 spec [--hooks A,B] [--out F] | ingest SPOOL | report SPOOL | split-report PLAIN RUST [--fn F,G] [--timeline N] [--nesting] | dump-diff A B | replay [--fn F] [--mutant F] | show SPOOL [N]".into()),
    }
}

#[cfg(feature = "mutants")]
fn mutants(a: &Args) -> Result<bool, String> {
    use sa_math::general::mutants::ALL;
    let img = sa_oracle::image_cached().ok_or("gta_sa.exe 1.0 US not found (set SA_GTA_EXE)")?;
    let mut cfg = cfg_from(a)?;
    cfg.fixtures = false;
    cfg.keep = 1;
    println!(
        "mutation smoke test: CGeneral::SolveQuadratic, {} generated cases per mutant",
        cfg.cases
    );
    let mut caught = 0;
    for m in ALL {
        let spec = specs::solve_quadratic_with(move |a, b, c| {
            sa_math::general::mutants::solve_quadratic(m, a, b, c)
        });
        let rep = runner::run(&spec, &img, &cfg)?;
        let ok = rep.mismatches > 0;
        caught += ok as usize;
        println!(
            "  {:<28} {}  mismatches={} {:?}",
            format!("{m:?}"),
            if ok { "CAUGHT" } else { "MISSED" },
            rep.mismatches,
            rep.kinds
        );
        if let Some(e) = rep.examples.first()
            && let Some((i, d)) = &e.shrunk
        {
            println!("      minimal: [{}] {d}", runner::fmt_inputs(&spec, i));
        }
    }
    println!("  caught {caught}/{}", ALL.len());
    let mut equivalent_ok = true;
    for m in sa_math::general::mutants::EQUIVALENT {
        let spec = specs::solve_quadratic_with(move |a, b, c| {
            sa_math::general::mutants::solve_quadratic(m, a, b, c)
        });
        let rep = runner::run(&spec, &img, &cfg)?;
        equivalent_ok &= rep.mismatches == 0;
        println!(
            "  equivalent mutant {m:?}: mismatches={} ({})",
            rep.mismatches,
            if rep.mismatches == 0 {
                "equivalent, as expected"
            } else {
                "NOT equivalent"
            }
        );
    }
    Ok(caught == ALL.len() && equivalent_ok)
}

#[cfg(not(feature = "mutants"))]
fn mutants(_: &Args) -> Result<bool, String> {
    Err("build with `--features mutants`".into())
}

pub fn main() -> ExitCode {
    let a = args();
    let r = match a.pos.first().map(String::as_str) {
        Some("list") => {
            for s in specs::all() {
                let route = match s.route {
                    Route::L1 => "L1".to_string(),
                    Route::L2 { reason } => format!("L2 ({reason})"),
                };
                println!("{:<28} {:#x}  {:<40} {route}", s.key, s.addr, s.name);
            }
            Ok(true)
        }
        Some("l1") => l1(&a),
        Some("fidelity") => fidelity(&a),
        Some("mutants") => mutants(&a),
        Some("l2") => l2(&a),
        // INF-9 / INF-10 (own argument parsing: `--no-x` flags, `--k v` options)
        Some("l3") => sa_oracle::l3::cli(&std::env::args().skip(2).collect::<Vec<_>>()),
        Some("coverage") => sa_oracle::coverage::cli(&std::env::args().skip(2).collect::<Vec<_>>()),
        _ => Err("usage: sa-oracle list | l1 <fn|all> [--cases N] [--seed S] [--backend auto|unicorn|native] | fidelity | mutants | l2 spec|ingest|report|split-report|replay|show | l3 ... | coverage points|report".into()),
    };
    if let Some(hwm) = std::fs::read_to_string("/proc/self/status").ok().and_then(|s| {
        s.lines()
            .find(|l| l.starts_with("VmHWM:"))
            .map(|l| l.split_whitespace().nth(1).unwrap_or("?").to_string())
    }) {
        eprintln!("peak RSS: {hwm} kB");
    }
    match r {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}
