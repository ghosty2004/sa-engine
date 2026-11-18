//! The native backend's entry (ADR-003 §3.1): the command line, the boot from the install folder on disk and
//! the offscreen / windowed run of [`crate::app::Session`] (the web backend is [`super::web`]).

use std::path::PathBuf;

use sa_platform::native::{NativeFs, documents_dir};

use crate::app::{
    Args, INIT_FAILED, Session, TEMP_DOCS, boot_with, frame_intervals, frame_time_summary, synthetic_input,
};
use super::{game_window, screen};
use crate::{d3d9, det, native};

pub fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        game: PathBuf::new(),
        frames: None,
        window: false,
        exit_after: None,
        auto_input: false,
        frame_times: None,
        screenshot: None,
        report: None,
        documents: None,
        mods: None,
        image: None,
    };
    let mut system_heap = false;
    // `SA_GAME_PERF=FILE` is `--perf FILE`
    let mut perf: Option<PathBuf> = std::env::var_os("SA_GAME_PERF").filter(|v| !v.is_empty()).map(PathBuf::from);
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().ok_or(format!("{k} needs a value"));
        match k.as_str() {
            "--game" => a.game = PathBuf::from(val()?),
            "--frames" => a.frames = Some(val()?.parse().map_err(|e| format!("--frames: {e}"))?),
            "--window" => a.window = true,
            "--system-heap" => system_heap = true,
            "--exit-after" => a.exit_after = Some(val()?.parse().map_err(|e| format!("--exit-after: {e}"))?),
            "--auto-input" => a.auto_input = true,
            "--frame-times" => a.frame_times = Some(PathBuf::from(val()?)),
            "--perf" => perf = Some(PathBuf::from(val()?)),
            "--screenshot" => a.screenshot = Some(PathBuf::from(val()?)),
            "--report" => a.report = Some(PathBuf::from(val()?)),
            "--present-dir" => d3d9::set_present_dir(PathBuf::from(val()?)),
            "--documents" => a.documents = Some(PathBuf::from(val()?)),
            "--persistent-documents" => a.documents = Some(documents_dir()),
            "--start-in-world" => native::set_start_in_world(),
            "--mods" => a.mods = Some(val()?),
            "--image" => a.image = Some(crate::arena::ImageKind::parse(&val()?)?),
            "--fixed-step" => det::set_fixed_step(val()?.parse().map_err(|e| format!("--fixed-step: {e}"))?),
            "-h" | "--help" => {
                println!(
                    "sa-game --game DIR [--frames N] [--report FILE] [--present-dir DIR] [--documents DIR | --persistent-documents] [--start-in-world] [--fixed-step MS] [--frame-times FILE] [--perf FILE] [--mods DIR[,DIR...]] [--image exe|builtin|check] [--window [--exit-after SECONDS] [--auto-input] | --screenshot OUT.png]\n\
                     The user documents folder (settings, saves) is a fresh temporary folder per run unless\n\
                     --documents DIR or --persistent-documents ($XDG_DATA_HOME/sa-engine/documents) is given.\n\
                     --window runs the game in a native window with the window's keyboard and mouse; --auto-input adds\n\
                     the offscreen mode's synthetic input (Enter through the menus, SA_GAME_WALK).\n\
                     --fixed-step MS: deterministic mode (the game clock advances MS per main-loop iteration, streaming\n\
                     reads complete within their iteration; scripted input: SA_GAME_KEYS, SA_GAME_WALK, SA_GAME_MOUSE, SA_GAME_LMB, SA_GAME_POINTER).\n\
                     The game heap comes from the deterministic mode's fixed-address region at 0x18000000 (one heap\n\
                     layout everywhere, in the original's address range); --system-heap uses the C library's allocator\n\
                     instead (its addresses read as huge floats where the game reads stale pointers: not 1:1).\n\
                     --mods DIR[,DIR...] loads the WebAssembly mods in each DIR, in that order (a mod folder with mod.toml, or a\n\
                     folder of them; build with\n\
                     --features mods; docs/mods/getting-started.md).\n\
                     --image: where the arena's initial data comes from: exe (the data sections of DIR/gta_sa.exe, 1.0 US),\n\
                     builtin (the built-in image, no exe read) or check (the built-in image, compared byte for byte with\n\
                     DIR/gta_sa.exe first). Default: exe when DIR has gta_sa.exe, else builtin.\n\
                     --perf FILE (or SA_GAME_PERF=FILE): per-frame phase times and counters of every in-game frame to FILE\n\
                     (CSV) and a summary table at exit (docs/perf/measure.md)."
                );
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option {k}")),
        }
    }
    if a.game.as_os_str().is_empty() {
        return Err("--game DIR is required (the GTA San Andreas install)".into());
    }
    // decided before the first game allocation: a block must be freed by the allocator that made it. Every run (window
    // or offscreen) takes the fixed-address region, which keeps the game heap in the original's address range
    // (platform.md section 23); --system-heap opts out.
    det::set_fixed_heap(!system_heap);
    // before the GPU device and the registry exist (the timestamp feature, the probes)
    if let Some(p) = perf {
        crate::perf::enable(crate::perf::Output::File(p));
    }
    // the run report lists the registry's dispatch counts (counting is off otherwise: it costs on every dispatch)
    if a.report.is_some() {
        sa_core::orig::registry::set_counting(true);
    }
    Ok(a)
}

/// The platform setup and the boot (WinMain up to its main loop).
pub fn boot(args: &Args) -> Result<Session, String> {
    let game = NativeFs::new(&args.game).map_err(|e| format!("--game {}: {e}", args.game.display()))?;
    if !sa_platform::fs::GameFs::exists(&game, "models/gta3.img") {
        return Err(format!(
            "{} does not look like a GTA San Andreas folder (no models/gta3.img)",
            args.game.display()
        ));
    }
    let docs = match &args.documents {
        Some(d) => d.clone(),
        None => {
            let d = std::env::temp_dir().join(format!("sa-game-documents-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            TEMP_DOCS.with(|t| *t.borrow_mut() = Some(d.clone()));
            d
        }
    };
    std::fs::create_dir_all(&docs).map_err(|e| format!("{}: {e}", docs.display()))?;
    let docs_fs = NativeFs::new(&docs).map_err(|e| e.to_string())?;
    let exe_path = args.game.join("gta_sa.exe");
    let default = if exe_path.is_file() { crate::arena::ImageKind::Exe } else { crate::arena::ImageKind::Builtin };
    let image = args.image.unwrap_or(default).image(|| {
        std::fs::read(&exe_path).map_err(|e| format!("{}: {e} (--image builtin runs without it)", exe_path.display()))
    })?;
    boot_with(
        Box::new(game),
        Box::new(docs_fs),
        image,
        &args.game.display().to_string(),
        &docs.display().to_string(),
        args.mods.as_deref(),
    )
}

pub fn remove_temp_docs() {
    if let Some(d) = TEMP_DOCS.with(|t| t.borrow_mut().take()) {
        let _ = std::fs::remove_dir_all(d);
    }
}

/// The offscreen run: the boot, then `--frames` iterations (3 by default) with the synthetic input.
pub fn run_offscreen(args: &Args) -> Result<Session, String> {
    let mut s = boot(args)?;
    for n in 0..args.frames.unwrap_or(3) {
        if !s.iterate(synthetic_input(n)) {
            break;
        }
    }
    Ok(s)
}

pub fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("sa-game: {e}");
            std::process::exit(2);
        }
    };
    let session = if args.window { game_window::run(&args) } else { run_offscreen(&args) };
    match session {
        Ok(s) => {
            let md = s.report();
            if let Some(p) = &args.report
                && let Err(e) = std::fs::write(p, md)
            {
                eprintln!("sa-game: {}: {e}", p.display());
            }
            if let Some(ft) = frame_time_summary() {
                println!("sa-game: frame pacing (in-game presents): {ft}");
            }
            crate::perf::finish();
            #[cfg(feature = "mods")]
            if let Some(m) = crate::mods::summary() {
                print!("{}", m.lines().map(|l| format!("sa-game: {l}\n")).collect::<String>());
            }
            if let Some(p) = &args.frame_times {
                let text: String = frame_intervals().iter().map(|v| format!("{v:.3}\n")).collect();
                if let Err(e) = std::fs::write(p, text) {
                    eprintln!("sa-game: {}: {e}", p.display());
                }
            }
            let r = match &args.screenshot {
                Some(p) if !args.window => screen::screenshot(p).map(|()| println!("sa-game: wrote {}", p.display())),
                _ => Ok(()),
            };
            remove_temp_docs();
            if let Err(e) = r {
                eprintln!("sa-game: {e}");
                std::process::exit(1);
            }
            if INIT_FAILED.load(std::sync::atomic::Ordering::SeqCst) {
                std::process::exit(4);
            }
        }
        Err(e) => {
            remove_temp_docs();
            eprintln!("sa-game: {e}");
            std::process::exit(1);
        }
    }
}
