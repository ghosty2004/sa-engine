//! `sa-oracle` CLI (see main.rs).
//!
//! ```text
//! sa-oracle list
//! sa-oracle l1 <fn|all> [--cases N] [--seed S] [--threads T] [--backend auto|unicorn|native]
//!                       [--cw 0x7f] [--mxcsr 0x9fa0] [--no-fixtures]
//! sa-oracle fidelity [--cases N] [--fn-cases N]
//! sa-oracle mutants [--cases N]          (needs --features mutants)
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
        _ => Err("usage: sa-oracle list | l1 <fn|all> [--cases N] [--seed S] [--backend auto|unicorn|native] | fidelity | mutants".into()),
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
