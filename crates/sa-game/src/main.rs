//! `sa-game`: the phase B executable (PLAN §2, F10; docs/arch/platform.md).
//!
//! Runs natively on Linux without Wine and without executing any original code (a 32-bit
//! process, `--target i686-unknown-linux-gnu`): it reads the user's install through sa-platform,
//! maps the legacy address arena with the data sections of the user's own gta_sa.exe ([`arena`]),
//! installs `orig!`'s address registry of every Rust port ([`native`]), binds the ported code's
//! Win32/CRT/DirectInput declarations to sa-platform ([`bind`]) and runs the ported entry points
//! of the original `WinMain` in order ([`boot`]), then a number of main-loop iterations. Every
//! step that is not in Rust, or that reaches an unported callee, is reported instead of crashing.
//!
//! ```text
//! sa-game --game wine/gta-sa [--frames N] [--report out.md] [--window [--exit-after S] | --screenshot out.png]
//! ```
#![deny(unsafe_code)]

mod arena;
mod bind;
mod boot;
mod cinit;
mod com;
mod crt;
mod crt_fmt;
mod d3d9;
mod dsound;
mod d3d9_shader;
mod dx;
mod screen;
mod imports;
mod native;
mod rw_standin;
mod samp;
mod sha256;
mod winmain;

use std::fmt::Write as _;
use std::path::PathBuf;

use sa_platform::clock::StdClock;
use sa_platform::input::{InputEvent, InputState, Key, MouseButton};
use sa_platform::native::{NativeFs, documents_dir};
use sa_platform::vfs::{FileTable, Vfs};
use sa_platform::win32::{MessageQueue, Win32Time};

use boot::{Outcome, Step, run_step};

struct Args {
    game: PathBuf,
    frames: u32,
    window: bool,
    exit_after: Option<f32>,
    screenshot: Option<PathBuf>,
    report: Option<PathBuf>,
    /// the user documents folder: `None` = a fresh temporary folder for this run (removed at exit)
    documents: Option<PathBuf>,
    /// `--samp HOST[:PORT]`: host the SA:MP client (skeleton, `samp.rs`)
    samp: Option<String>,
    /// `--name NAME`: the SA:MP nickname
    name: String,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        game: PathBuf::new(),
        frames: 3,
        window: false,
        exit_after: None,
        screenshot: None,
        report: None,
        documents: None,
        samp: None,
        name: "Player".into(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().ok_or(format!("{k} needs a value"));
        match k.as_str() {
            "--game" => a.game = PathBuf::from(val()?),
            "--frames" => a.frames = val()?.parse().map_err(|e| format!("--frames: {e}"))?,
            "--window" => a.window = true,
            "--exit-after" => a.exit_after = Some(val()?.parse().map_err(|e| format!("--exit-after: {e}"))?),
            "--screenshot" => a.screenshot = Some(PathBuf::from(val()?)),
            "--report" => a.report = Some(PathBuf::from(val()?)),
            "--present-dir" => d3d9::set_present_dir(PathBuf::from(val()?)),
            "--documents" => a.documents = Some(PathBuf::from(val()?)),
            "--persistent-documents" => a.documents = Some(documents_dir()),
            "--start-in-world" => native::set_start_in_world(),
            "--samp" => a.samp = Some(val()?),
            "--name" => a.name = val()?,
            "-h" | "--help" => {
                println!(
                    "sa-game --game DIR [--frames N] [--report FILE] [--present-dir DIR] [--documents DIR | --persistent-documents] [--start-in-world] [--samp HOST[:PORT] [--name NAME]] [--window [--exit-after SECONDS] | --screenshot OUT.png]\n\
                     The user documents folder (settings, saves) is a fresh temporary folder per run unless\n\
                     --documents DIR or --persistent-documents ($XDG_DATA_HOME/sa-engine/documents) is given.\n\
                     --samp hosts the SA:MP client (skeleton: the native game adapter with a stand-in client, no network yet)."
                );
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option {k}")),
        }
    }
    if let Some(s) = &a.samp {
        samp::parse_target(s, &a.name)?;
    }
    if a.game.as_os_str().is_empty() {
        return Err("--game DIR is required (the GTA San Andreas install)".into());
    }
    Ok(a)
}

struct Report {
    rows: Vec<(String, String, u32, String)>,
}

impl Report {
    fn add(&mut self, phase: &str, s: &Step, o: &Outcome) {
        let (status, detail) = match o {
            Outcome::Ok(d) => ("rust: ok", d.clone()),
            Outcome::OriginalOnly => ("ORIGINAL-ONLY", "not in Rust (catalog/state.csv)".into()),
            Outcome::Blocked { callee, addr } => {
                ("rust: blocked", format!("calls unported {callee} ({addr})"))
            }
            Outcome::Panicked(m) => ("rust: panicked", m.clone()),
            Outcome::Crashed(m) => ("rust: CRASHED", format!("{m} (step skipped)")),
        };
        println!(
            "  [{phase}] {:<62} {status:<15} {detail}",
            format!("{} ({:#x})", s.name, s.addr)
        );
        self.rows
            .push((phase.into(), s.name.into(), s.addr, format!("{status}: {detail}")));
    }
}

fn run(args: &Args) -> Result<String, String> {
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

    let arena_info = arena::map(&args.game)?;
    com::map()?;
    dx::map()?;
    d3d9::map()?;
    dsound::map()?;
    d3d9_shader::map()?;
    let reg = native::install();
    native::install_script_skip();
    arena::install_crash_reporter();
    boot::install_panic_hook();
    bind::install(bind::Platform {
        vfs: Vfs::new(Box::new(game), Box::new(docs_fs)),
        files: FileTable::default(),
        time: Win32Time::new(StdClock::default()),
        queue: MessageQueue::new(bind::HWND),
        input: InputState::default(),
        last_error: 0,
        undelivered: Default::default(),
        calls: Default::default(),
    });

    let samp_target = match &args.samp {
        Some(s) => {
            let t = samp::parse_target(s, &args.name)?;
            samp::install(&t)?;
            Some(t)
        }
        None => None,
    };
    println!(
        "sa-game: game folder {} (read-only), user documents {}",
        args.game.display(),
        docs.display()
    );
    println!("sa-game: arena: {arena_info}");
    println!(
        "sa-game: orig! address registry: {} Rust ports, {} CRT replacements, {} of {} imports ({} addresses with a second #[hook] ignored){}",
        reg.ports,
        reg.crt,
        reg.imports,
        arena::imports().len(),
        reg.duplicates.len(),
        if cfg!(target_arch = "x86") {
            ""
        } else {
            "; NOT dispatched on this target (build for i686-unknown-linux-gnu)"
        }
    );
    for (a, first, other) in &reg.duplicates {
        println!("  duplicate #[hook] at {a:#x}: {first} (used), {other}");
    }
    let (nglob, bad) = native::check_globals();
    println!(
        "sa-game: {nglob} globals! declarations alias the arena; {} differ from the exe image's initial value{}",
        bad.len(),
        if bad.is_empty() { "" } else { " (the exe's value is used):" }
    );
    for b in &bad {
        println!("  {b}");
    }
    let mut rep = Report { rows: Vec::new() };
    println!("boot (WinMain 0x748710, original order):");
    // the CRT's C++ static initialisers run before WinMain (cinit.rs)
    let s = boot::Step {
        name: "CRT _cinit: C++ static initialisers (__xc_a 0x8A4000 .. __xc_z 0x8A5A10)",
        addr: 0x823B76,
        run: Some(cinit::run),
        probe: false,
    };
    let o = run_step(&s);
    rep.add("boot", &s, &o);
    for s in winmain::boot_steps() {
        if winmain::aborted() {
            break;
        }
        let o = run_step(&s);
        rep.add("boot", &s, &o);
    }
    for s in boot::checks() {
        let o = run_step(&s);
        rep.add("check", &s, &o);
    }
    let frame = [winmain::frame_step()];
    let (mut last, mut repeats): (Option<Outcome>, u32) = (None, 0);
    for n in 0..args.frames {
        if winmain::aborted() {
            break;
        }
        // synthetic input: Enter and the left mouse button pressed and released every 8 iterations
        // (skips the logo and title movie states, presses the menu's default entry; Enter reaches
        // CPad through the window procedure, the mouse through DirectInput)
        // `SA_GAME_WALK=START:COUNT` (evidence runs): from iteration START the W key (GO_FORWARD) is
        // pressed and held for COUNT iterations, then released; the synthetic Enter stops 8
        // iterations before START (Enter enters vehicles in the world)
        let walk = walk_spec();
        bind::with("input", |p| {
            let t = p.time.time_get_time();
            let mut i = InputState::default();
            if let Some((start, count)) = walk {
                if n >= start && n < start + count {
                    i.held.insert(Key::W);
                    if n == start {
                        i.key_down(Key::W);
                    }
                } else if n == start + count {
                    i.events.push(InputEvent::KeyUp { key: Key::W });
                }
            }
            // in game (gGameState IDLE) the menu input stops: the left button is the fire / punch key and Enter
            // enters vehicles
            let enter = walk.is_none_or(|(start, _)| n + 8 < start) && winmain::game_state() != winmain::STATE_IDLE;
            match if enter { n % 8 } else { 0 } {
                4 => {
                    i.key_down(Key::Enter);
                    i.mouse_held.insert(MouseButton::Left);
                }
                // the button (DirectInput state only, no window message) stays down while the key
                // messages take their iterations
                // the key-down message takes iteration 4; 5 and 6 run game frames with Enter held
                // (the window procedure feeds CPad), the key-up message is queued at 7
                5 | 6 => {
                    i.mouse_held.insert(MouseButton::Left);
                }
                7 => {
                    i.events.push(InputEvent::KeyUp { key: Key::Enter });
                }
                _ => {}
            }
            // the synthetic mouse movement stops in game too: it turns the player camera (pitching it at the ground)
            if winmain::game_state() != winmain::STATE_IDLE {
                i.mouse_delta = (4.0, -2.0);
            }
            for e in &i.events {
                p.queue.push_event(e, t);
            }
            p.input = i;
        });
        println!("frame {n}:");
        let state = winmain::game_state();
        let o = run_step(&frame[0]);
        rep.add(&format!("frame {n}"), &frame[0], &o);
        // decision 2 of STD-5 (platform.md section 12.1): a failure inside `CGame::Initialise`
        // leaves the game half-initialised; the next iteration would retry it from that state, so
        // the run reports the blocked function and stops (exit code 4)
        if state == winmain::STATE_LOADING_STARTED && !matches!(o, Outcome::Ok(_)) {
            println!("main loop: CGame::Initialise did not complete; stopping (no retry)");
            INIT_FAILED.store(true, std::sync::atomic::Ordering::SeqCst);
            break;
        }
        // a loop that cannot get further makes no progress: stop after three identical failures
        let failed = !matches!(o, Outcome::Ok(_));
        repeats = if failed && last.as_ref() == Some(&o) { repeats + 1 } else { 0 };
        last = Some(o);
        if failed && repeats >= 2 {
            println!("main loop: the same failure three times in a row; stopping");
            break;
        }
    }

    let mut md = String::new();
    let _ = writeln!(
        md,
        "# sa-game native run\n\n| phase | step | address | result |\n|---|---|---|---|"
    );
    for (ph, name, addr, r) in &rep.rows {
        let _ = writeln!(md, "| {ph} | {name} | {addr:#x} | {} |", r.replace('|', "\\|"));
    }
    let skipped = cinit::skipped();
    let _ = writeln!(
        md,
        "\n## C++ static initialisers without a port (skipped, {})\n",
        skipped.len()
    );
    for (a, name) in &skipped {
        let _ = writeln!(md, "- {a:#x} {name}");
    }
    let (calls, undelivered) = bind::with("report", |p| (p.calls.clone(), p.undelivered.clone()));
    let _ = writeln!(md, "\n## Platform calls served natively\n");
    for (k, v) in calls.iter().filter(|(k, _)| **k != "report" && **k != "input") {
        let _ = writeln!(md, "- `{k}`: {v}");
    }
    let disp = native::dispatched();
    let _ = writeln!(
        md,
        "\n## Native implementations reached through the orig! address registry ({} functions, {} calls)\n",
        disp.len(),
        disp.iter().map(|d| d.2 as u64).sum::<u64>()
    );
    for (name, addr, n) in &disp {
        let _ = writeln!(md, "- `{name}` ({addr:#x}): {n}");
    }
    if !undelivered.is_empty() {
        let _ = writeln!(
            md,
            "\n## Window messages without a WndProc (Win::__MainWndProc 0x747EB0 is original-only)\n"
        );
        for (m, n) in &undelivered {
            let _ = writeln!(md, "- message {m:#06x}: {n}");
        }
    }
    if let Some(t) = &samp_target {
        samp::report(&mut md, t);
    }
    let crashed = rep
        .rows
        .iter()
        .filter(|r| r.3.starts_with("rust: CRASHED"))
        .count();
    let ok = rep.rows.iter().filter(|r| r.3.starts_with("rust: ok")).count();
    let blocked = rep
        .rows
        .iter()
        .filter(|r| r.3.starts_with("rust: blocked"))
        .count();
    let orig = rep
        .rows
        .iter()
        .filter(|r| r.3.starts_with("ORIGINAL-ONLY"))
        .count();
    let _ = writeln!(
        md,
        "\nSummary: {ok} steps ran in Rust, {blocked} blocked on an unported callee, {orig} original-only, {crashed} crashed (skipped).\nFull MVP gap list: docs/arch/platform-mvp-gaps.csv."
    );
    println!(
        "summary: {ok} ok, {blocked} blocked, {orig} original-only, {crashed} crashed; calls served natively: {}",
        calls.values().sum::<u64>()
    );
    Ok(md)
}

/// Set when `CGame::Initialise` did not complete: the process exits with code 4.
static INIT_FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

thread_local! {
    /// The temporary documents folder of this run, removed at exit.
    static TEMP_DOCS: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

fn remove_temp_docs() {
    if let Some(d) = TEMP_DOCS.with(|t| t.borrow_mut().take()) {
        let _ = std::fs::remove_dir_all(d);
    }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("sa-game: {e}");
            std::process::exit(2);
        }
    };
    match run(&args) {
        Ok(md) => {
            if let Some(p) = &args.report
                && let Err(e) = std::fs::write(p, md)
            {
                eprintln!("sa-game: {}: {e}", p.display());
            }
            let r = if let Some(p) = &args.screenshot {
                screen::screenshot(p).map(|()| println!("sa-game: wrote {}", p.display()))
            } else if args.window {
                screen::run_window(args.exit_after)
            } else {
                Ok(())
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

/// `SA_GAME_WALK=START:COUNT` (see the main loop).
fn walk_spec() -> Option<(u32, u32)> {
    let v = std::env::var("SA_GAME_WALK").ok()?;
    let (a, b) = v.split_once(':')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}
