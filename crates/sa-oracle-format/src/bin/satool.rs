//! satool: inspect and compare oracle files.
//!
//! ```text
//! satool diff A.trace B.trace [--ignore NAME,NAME]   first divergent frame + per-subsystem first frame
//! satool show A.trace [FROM [TO]]                    print records (frame tick hashes...)
//! satool rec  R.sarec                                header + tick/frame summary of a recording
//! satool program P.txt                               parse an input program and summarise it
//! ```
//! `diff` exits 0 when the traces agree on every common frame (and have at least one), 1 otherwise.

use sa_oracle_format::program::Program;
use sa_oracle_format::recording::Recording;
use sa_oracle_format::trace::{Trace, compare};

fn load_trace(p: &str) -> Result<Trace, String> {
    let b = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
    Trace::parse(&b).map_err(|e| format!("{p}: {e}"))
}

fn run(args: &[String]) -> Result<i32, String> {
    match args.first().map(|s| s.as_str()) {
        Some("diff") if args.len() >= 3 => {
            let a = load_trace(&args[1])?;
            let b = load_trace(&args[2])?;
            let ignore: Vec<String> = match args.iter().position(|s| s == "--ignore") {
                Some(i) => args
                    .get(i + 1)
                    .ok_or("--ignore needs a list")?
                    .split(',')
                    .map(|s| s.to_string())
                    .collect(),
                None => Vec::new(),
            };
            let ig: Vec<&str> = ignore.iter().map(|s| s.as_str()).collect();
            let c = compare(&a, &b, &ig)?;
            let last = |t: &Trace| t.records.last().map(|r| r.frame).unwrap_or(0);
            println!(
                "frames: a={} (last {}) b={} (last {}) common={} only_a={} only_b={}",
                a.records.len(),
                last(&a),
                b.records.len(),
                last(&b),
                c.common_frames,
                c.only_a,
                c.only_b
            );
            if !ig.is_empty() {
                println!("ignored: {}", ig.join(","));
            }
            for (n, f) in &c.per_subsystem {
                let state = if ig.contains(&n.as_str()) {
                    "ignored".to_string()
                } else {
                    match f {
                        Some(f) => format!("first divergence at frame {f}"),
                        None => "identical".to_string(),
                    }
                };
                println!("  {n:<13} {state}");
            }
            match &c.first {
                Some(d) => {
                    println!("DIVERGED at frame {}: {}", d.frame, d.subsystems.join(","));
                    Ok(1)
                }
                None if c.common_frames == 0 => {
                    println!("NO COMMON FRAMES");
                    Ok(1)
                }
                None => {
                    println!("IDENTICAL over {} common frames", c.common_frames);
                    Ok(0)
                }
            }
        }
        Some("show") if args.len() >= 2 => {
            let t = load_trace(&args[1])?;
            let from: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
            let to: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(u32::MAX);
            println!("frame tick {}", t.names.join(" "));
            for r in t.records.iter().filter(|r| (from..=to).contains(&r.frame)) {
                let hs: Vec<String> = r.hashes.iter().map(|h| format!("{h:016x}")).collect();
                println!("{} {} {}", r.frame, r.tick, hs.join(" "));
            }
            Ok(0)
        }
        Some("rec") if args.len() >= 2 => {
            let b = std::fs::read(&args[1]).map_err(|e| format!("{}: {e}", args[1]))?;
            let r = Recording::parse(&b)?;
            for (k, v) in &r.header.fields {
                println!("{k}={v}");
            }
            let distinct = r.ticks.windows(2).filter(|w| w[0].input != w[1].input).count() + 1;
            println!(
                "ticks={} first_frame={} last_frame={} input_changes={} bytes={}",
                r.ticks.len(),
                r.ticks.first().map(|t| t.frame).unwrap_or(0),
                r.ticks.last().map(|t| t.frame).unwrap_or(0),
                if r.ticks.is_empty() { 0 } else { distinct },
                b.len()
            );
            Ok(0)
        }
        Some("program") if args.len() >= 2 => {
            let s = std::fs::read_to_string(&args[1]).map_err(|e| format!("{}: {e}", args[1]))?;
            let p = Program::parse(&s)?;
            println!("ok end={:?} fnv={:016x}", p.end, p.fnv);
            Ok(0)
        }
        _ => Err("usage: satool diff A B [--ignore N,N] | show A [FROM [TO]] | rec R | program P".into()),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("satool: {e}");
            std::process::exit(2);
        }
    }
}
