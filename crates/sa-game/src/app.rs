//! The command-line front end and the boot / main-loop session shared by the native executable and the web build.

#![allow(dead_code)]

use crate::{arena, bind, boot, cinit, com, d3d9, d3d9_shader, det, dsound, dx, native, winmain};
#[cfg(not(target_arch = "wasm32"))]
use crate::{game_window, screen};
use std::fmt::Write as _;
use std::path::PathBuf;

use sa_platform::input::{InputEvent, InputState, Key, MouseButton};
#[cfg(not(target_arch = "wasm32"))]
use sa_platform::native::{NativeFs, documents_dir};
use sa_platform::fs::GameFs;
use sa_platform::vfs::{FileTable, Vfs};
use sa_platform::win32::{MessageQueue, Win32Time};

use boot::{Outcome, Step, run_step};

pub struct Args {
    pub game: PathBuf,
    /// main-loop iterations (`None`: 3 offscreen, unlimited in a window)
    pub frames: Option<u32>,
    pub window: bool,
    pub exit_after: Option<f32>,
    /// `--auto-input`: the offscreen mode's synthetic input in the window too (evidence runs)
    pub auto_input: bool,
    /// `--frame-times FILE`: the intervals between in-game presents (ms, one per line)
    pub frame_times: Option<PathBuf>,
    pub screenshot: Option<PathBuf>,
    pub report: Option<PathBuf>,
    /// the user documents folder: `None` = a fresh temporary folder for this run (removed at exit)
    pub documents: Option<PathBuf>,
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_args() -> Result<Args, String> {
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
    };
    let mut system_heap = false;
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
            "--screenshot" => a.screenshot = Some(PathBuf::from(val()?)),
            "--report" => a.report = Some(PathBuf::from(val()?)),
            "--present-dir" => d3d9::set_present_dir(PathBuf::from(val()?)),
            "--documents" => a.documents = Some(PathBuf::from(val()?)),
            "--persistent-documents" => a.documents = Some(documents_dir()),
            "--start-in-world" => native::set_start_in_world(),
            "--fixed-step" => det::set_fixed_step(val()?.parse().map_err(|e| format!("--fixed-step: {e}"))?),
            "-h" | "--help" => {
                println!(
                    "sa-game --game DIR [--frames N] [--report FILE] [--present-dir DIR] [--documents DIR | --persistent-documents] [--start-in-world] [--fixed-step MS] [--frame-times FILE] [--window [--exit-after SECONDS] [--auto-input] | --screenshot OUT.png]\n\
                     The user documents folder (settings, saves) is a fresh temporary folder per run unless\n\
                     --documents DIR or --persistent-documents ($XDG_DATA_HOME/sa-engine/documents) is given.\n\
                     --window runs the game in a native window with the window's keyboard and mouse; --auto-input adds\n\
                     the offscreen mode's synthetic input (Enter through the menus, SA_GAME_WALK).\n\
                     --fixed-step MS: deterministic mode (the game clock advances MS per main-loop iteration, streaming\n\
                     reads complete within their iteration; scripted input: SA_GAME_KEYS, SA_GAME_WALK, SA_GAME_MOUSE, SA_GAME_LMB).\n\
                     Window runs serve the game heap from the deterministic mode's fixed-address region (one heap\n\
                     layout everywhere); --system-heap uses the C library's allocator instead."
                );
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option {k}")),
        }
    }
    if a.game.as_os_str().is_empty() {
        return Err("--game DIR is required (the GTA San Andreas install)".into());
    }
    // decided before the first game allocation: a block must be freed by the allocator that made it
    det::set_fixed_heap(a.window && !system_heap);
    Ok(a)
}

/// Whether a main-loop step's detail (`gGameState A → B ...`) reports a state change.
fn state_changed(d: &str) -> bool {
    let Some((a, b)) = d.split_once('→') else { return false };
    a.split_whitespace().last() != b.split_whitespace().next()
}

/// Set by the web front end: successful main-loop iterations without a `gGameState` change are not logged.
pub static QUIET_FRAMES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct Report {
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
        // the web page logs only what changes: failed iterations and gGameState transitions
        let quiet = QUIET_FRAMES.load(std::sync::atomic::Ordering::Relaxed)
            && phase.starts_with("frame")
            && matches!(o, Outcome::Ok(d) if !state_changed(d));
        if quiet {
            return;
        }
        println!(
            "  [{phase}] {:<62} {status:<15} {detail}",
            format!("{} ({:#x})", s.name, s.addr)
        );
        self.rows
            .push((phase.into(), s.name.into(), s.addr, format!("{status}: {detail}")));
    }
}

/// A booted game: the report so far and the main loop's state.
pub struct Session {
    pub(crate) rep: Report,
    /// main-loop iterations run
    pub(crate) n: u32,
    pub(crate) last: Option<Outcome>,
    pub(crate) repeats: u32,
    /// the loop stopped (aborted, `CGame::Initialise` failed, or the same failure three times)
    pub(crate) stopped: bool,
    /// `gGameState` at the start of the current iteration
    pub(crate) iter_state: u32,
}

/// The platform setup and the boot (WinMain up to its main loop).
#[cfg(not(target_arch = "wasm32"))]
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
    let exe = std::fs::read(&exe_path).map_err(|e| {
        format!("{}: {e} (phase B reads the data sections of your own 1.0 US exe at start-up)", exe_path.display())
    })?;
    boot_with(
        Box::new(game),
        Box::new(docs_fs),
        exe,
        &args.game.display().to_string(),
        &docs.display().to_string(),
    )
}

/// The platform setup and the boot on any target: `game` is the install (read-only), `docs` the user documents
/// folder, `exe` the bytes of the install's gta_sa.exe (its data sections fill the arena).
pub fn boot_with(game: Box<dyn GameFs>, docs_fs: Box<dyn GameFs>, exe: Vec<u8>, game_label: &str, docs_label: &str) -> Result<Session, String> {
    if !game.exists("models/gta3.img") {
        return Err(format!("{game_label} does not look like a GTA San Andreas folder (no models/gta3.img)"));
    }
    let arena_info = arena::map_image(exe, &format!("{game_label}/gta_sa.exe"))?;
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
        vfs: Vfs::new(game, docs_fs),
        files: FileTable::default(),
        time: Win32Time::new(det::GameClock::default()),
        queue: MessageQueue::new(bind::HWND),
        input: InputState::default(),
        last_error: 0,
        undelivered: Default::default(),
        calls: Default::default(),
    });

    println!("sa-game: game folder {game_label} (read-only), user documents {docs_label}");
    println!("sa-game: arena: {arena_info}");
    println!(
        "sa-game: orig! address registry: {} Rust ports, {} CRT replacements, {} of {} imports ({} addresses with a second #[hook] ignored){}",
        reg.ports,
        reg.crt,
        reg.imports,
        arena::imports().len(),
        reg.duplicates.len(),
        if cfg!(target_arch = "wasm32") {
            "; wasm32: type-erased entries (sa_core::wabi)"
        } else if cfg!(target_arch = "x86") {
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
    Ok(Session {
        rep,
        n: 0,
        last: None,
        repeats: 0,
        stopped: false,
        iter_state: 0,
    })
}

/// The offscreen mode's synthetic input for main-loop iteration `n`.
///
/// Enter and the left mouse button are pressed and released every 8 iterations (skips the logo and title movie
/// states, presses the menu's default entry; Enter reaches CPad through the window procedure, the mouse through
/// DirectInput). `SA_GAME_WALK=START:COUNT` (evidence runs): from iteration START the W key (GO_FORWARD) is pressed
/// and held for COUNT iterations, then released; the synthetic Enter stops 8 iterations before START (Enter enters
/// vehicles in the world).
pub(crate) fn synthetic_input(n: u32) -> InputState {
    let walk = walk_spec();
    let mut i = InputState::default();
    if let Some((start, count)) = walk {
        if n >= start && n < start + count {
            // key_down first: it queues the key-down message only for a key not yet held
            if n == start {
                i.key_down(Key::W);
            }
            i.held.insert(Key::W);
        } else if n == start + count {
            i.events.push(InputEvent::KeyUp { key: Key::W });
        }
    }
    // SA_GAME_KEYS: scripted key presses (held from their down iteration to their up iteration)
    let plan = keys_spec();
    for (key, down, up) in &plan {
        if n == *down {
            i.key_down(*key);
        }
        if n >= *down && up.is_none_or(|u| n < u) {
            i.held.insert(*key);
        } else if Some(n) == *up {
            i.events.push(InputEvent::KeyUp { key: *key });
        }
    }
    // in game (gGameState IDLE) the menu input stops: the left button is the fire / punch key and Enter
    // enters vehicles
    let first_key = plan.iter().map(|p| p.1).min();
    let enter = walk.is_none_or(|(start, _)| n + 8 < start)
        && first_key.is_none_or(|start| n + 8 < start)
        && winmain::game_state() != winmain::STATE_IDLE;
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
    // SA_GAME_MOUSE: scripted mouse motion
    for (dx, dy, from, to) in mouse_spec() {
        if n >= from && n < to {
            i.mouse_delta.0 += dx;
            i.mouse_delta.1 += dy;
        }
    }
    // SA_GAME_LMB: the left mouse button (fire / punch) held over iteration ranges
    for (from, to) in lmb_spec() {
        if n >= from && n < to {
            i.mouse_held.insert(MouseButton::Left);
        }
    }
    i
}

/// `extra` added to `base`: events appended, held keys and buttons united, motion summed.
pub(crate) fn merge_input(base: &InputState, extra: &InputState) -> InputState {
    let mut i = base.clone();
    i.events.extend(extra.events.iter().cloned());
    i.pressed.extend(extra.pressed.iter().copied());
    i.held.extend(extra.held.iter().copied());
    i.mouse_held.extend(extra.mouse_held.iter().copied());
    i.mouse_delta.0 += extra.mouse_delta.0;
    i.mouse_delta.1 += extra.mouse_delta.1;
    i.wheel += extra.wheel;
    i
}

impl Session {
    /// One main-loop iteration with `input` (its events are queued as window messages, its state is the
    /// DirectInput state). Returns false when the loop has stopped.
    pub fn iterate(&mut self, input: InputState) -> bool {
        if self.stopped || winmain::aborted() {
            self.stopped = true;
            return false;
        }
        let n = self.n;
        self.n += 1;
        det::begin_iteration();
        bind::with("input", |p| {
            let t = p.time.time_get_time();
            for e in &input.events {
                p.queue.push_event(e, t);
            }
            p.input = input;
        });
        if !QUIET_FRAMES.load(std::sync::atomic::Ordering::Relaxed) {
            println!("frame {n}:");
        }
        teleport_player(n);
        let state = winmain::game_state();
        self.iter_state = state;
        let frame = winmain::frame_step();
        let o = run_step(&frame);
        self.finish(n, state, &frame, o)
    }

    /// wasm32: the iteration that an unported callee left through a JS exception (no unwinding there; see
    /// `web::recover`) ends as natively after a caught panic: a blocked step, with the same stop rules.
    #[cfg(target_arch = "wasm32")]
    pub fn finish_blocked(&mut self, msg: &str) -> bool {
        let o = match crate::boot::parse_unported(msg) {
            Some((callee, addr)) => Outcome::Blocked { callee, addr },
            None => Outcome::Panicked(msg.to_owned()),
        };
        let frame = winmain::frame_step();
        self.finish(self.n.wrapping_sub(1), self.iter_state, &frame, o)
    }

    /// The bookkeeping after an iteration's step: the report row and when the loop stops.
    fn finish(&mut self, n: u32, state: u32, frame: &Step, o: Outcome) -> bool {
        det::end_iteration();
        self.rep.add(&format!("frame {n}"), frame, &o);
        // decision 2 of STD-5 (platform.md section 12.1): a failure inside `CGame::Initialise`
        // leaves the game half-initialised; the next iteration would retry it from that state, so
        // the run reports the blocked function and stops (exit code 4)
        if state == winmain::STATE_LOADING_STARTED && !matches!(o, Outcome::Ok(_)) {
            println!("main loop: CGame::Initialise did not complete; stopping (no retry)");
            INIT_FAILED.store(true, std::sync::atomic::Ordering::SeqCst);
            self.stopped = true;
            return false;
        }
        // a loop that cannot get further makes no progress: stop after three identical failures (not in the
        // deterministic mode, whose runs collect the whole blocker list of their scenario)
        let failed = !matches!(o, Outcome::Ok(_));
        self.repeats = if failed && self.last.as_ref() == Some(&o) { self.repeats + 1 } else { 0 };
        self.last = Some(o);
        if failed && self.repeats >= 2 && !det::on() {
            println!("main loop: the same failure three times in a row; stopping");
            self.stopped = true;
            return false;
        }
        true
    }

    /// The run's report (markdown).
    fn report(&self) -> String {
        let rep = &self.rep;
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
        let count = |p: &str| rep.rows.iter().filter(|r| r.3.starts_with(p)).count();
        let (ok, blocked, orig, crashed) =
            (count("rust: ok"), count("rust: blocked"), count("ORIGINAL-ONLY"), count("rust: CRASHED"));
        let _ = writeln!(
            md,
            "\nSummary: {ok} steps ran in Rust, {blocked} blocked on an unported callee, {orig} original-only, {crashed} crashed (skipped).\nFull MVP gap list: docs/arch/platform-mvp-gaps.csv."
        );
        if let Some(ft) = frame_time_summary() {
            let _ = writeln!(md, "\nFrame pacing (in-game presents): {ft}");
        }
        // the blocker list: every unported callee or panic that stopped an iteration, with its count and first frame
        let mut blockers: Vec<(String, usize, String)> = Vec::new();
        for (ph, _, _, r) in rep.rows.iter().filter(|r| r.0.starts_with("frame ")) {
            let Some(what) = r
                .strip_prefix("rust: blocked: ")
                .or_else(|| r.strip_prefix("rust: panicked: "))
            else {
                continue;
            };
            match blockers.iter_mut().find(|b| b.0 == what) {
                Some(b) => b.1 += 1,
                None => blockers.push((what.to_string(), 1, ph.clone())),
            }
        }
        blockers.sort_by_key(|b| std::cmp::Reverse(b.1));
        let _ = writeln!(md, "\n## Blocker list ({} distinct)\n", blockers.len());
        println!("blocker list ({} distinct):", blockers.len());
        for (what, n, first) in &blockers {
            let _ = writeln!(md, "- {what}: {n} iterations, first in {first}");
            println!("  {n:6}  {what}  (first in {first})");
        }
        println!(
            "summary: {ok} ok, {blocked} blocked, {orig} original-only, {crashed} crashed; calls served natively: {}",
            calls.values().sum::<u64>()
        );
        md
    }
}

/// The intervals between the in-game presents (ms).
fn frame_intervals() -> Vec<f64> {
    let t = d3d9::frame_times();
    t.windows(2).map(|w| (w[1] - w[0]).as_secs_f64() * 1000.0).collect()
}

/// Count, mean, percentiles and the rate of the in-game present intervals.
fn frame_time_summary() -> Option<String> {
    let mut v = frame_intervals();
    if v.is_empty() {
        return None;
    }
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    v.sort_by(f64::total_cmp);
    let pct = |p: f64| v[((v.len() - 1) as f64 * p).round() as usize];
    Some(format!(
        "{} intervals, mean {mean:.2} ms ({:.1} fps), min {:.2}, median {:.2}, p95 {:.2}, p99 {:.2}, max {:.2} ms",
        v.len(),
        1000.0 / mean,
        v[0],
        pct(0.5),
        pct(0.95),
        pct(0.99),
        v[v.len() - 1]
    ))
}

/// Set when `CGame::Initialise` did not complete: the process exits with code 4.
static INIT_FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

thread_local! {
    /// The temporary documents folder of this run, removed at exit.
    static TEMP_DOCS: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(not(target_arch = "wasm32"))]
fn remove_temp_docs() {
    if let Some(d) = TEMP_DOCS.with(|t| t.borrow_mut().take()) {
        let _ = std::fs::remove_dir_all(d);
    }
}

/// The offscreen run: the boot, then `--frames` iterations (3 by default) with the synthetic input.
#[cfg(not(target_arch = "wasm32"))]
fn run_offscreen(args: &Args) -> Result<Session, String> {
    let mut s = boot(args)?;
    for n in 0..args.frames.unwrap_or(3) {
        if !s.iterate(synthetic_input(n)) {
            break;
        }
    }
    Ok(s)
}

#[cfg(not(target_arch = "wasm32"))]
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

/// `SA_GAME_KEYS=KEY:DOWN[:UP],...` (evidence runs): `KEY` (a letter, `space`, `enter`, `shift`, `ctrl`, `up`,
/// `down`, `left`, `right`) is pressed at main-loop iteration DOWN and released at UP (held to the end without UP).
/// The synthetic Enter stops 8 iterations before the first press.
fn keys_spec() -> Vec<(Key, u32, Option<u32>)> {
    let Ok(v) = crate::env::var("SA_GAME_KEYS") else {
        return Vec::new();
    };
    v.split(',')
        .filter_map(|e| {
            let mut it = e.trim().split(':');
            let name = it.next()?.to_ascii_lowercase();
            let key = match name.as_str() {
                "space" => Key::Space,
                "enter" => Key::Enter,
                "shift" => Key::ShiftLeft,
                "ctrl" => Key::ControlLeft,
                "up" => Key::Up,
                "down" => Key::Down,
                "left" => Key::Left,
                "right" => Key::Right,
                n if n.len() == 1 && n.as_bytes()[0].is_ascii_lowercase() => letter_key(n.as_bytes()[0]),
                _ => return None,
            };
            let down = it.next()?.parse().ok()?;
            let up = it.next().and_then(|u| u.parse().ok());
            Some((key, down, up))
        })
        .collect()
}

/// The `Key` of a lowercase ASCII letter.
fn letter_key(c: u8) -> Key {
    const KEYS: [Key; 26] = [
        Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K, Key::L, Key::M,
        Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X, Key::Y, Key::Z,
    ];
    KEYS[usize::from(c - b'a')]
}

/// `SA_GAME_MOUSE=DX:DY:FROM:TO,...` (evidence runs): relative mouse motion (DX, DY) added in every main-loop
/// iteration from FROM up to (not including) TO, e.g. to turn the player camera.
fn mouse_spec() -> Vec<(f64, f64, u32, u32)> {
    let Ok(v) = crate::env::var("SA_GAME_MOUSE") else {
        return Vec::new();
    };
    v.split(',')
        .filter_map(|e| {
            let f: Vec<&str> = e.trim().split(':').collect();
            if f.len() != 4 {
                return None;
            }
            Some((
                f[0].parse().ok()?,
                f[1].parse().ok()?,
                f[2].parse().ok()?,
                f[3].parse().ok()?,
            ))
        })
        .collect()
}

/// `SA_GAME_LMB=FROM:TO,...` (evidence runs): the left mouse button (DirectInput state) is held from main-loop
/// iteration FROM up to (not including) TO.
fn lmb_spec() -> Vec<(u32, u32)> {
    let Ok(v) = crate::env::var("SA_GAME_LMB") else {
        return Vec::new();
    };
    v.split(',')
        .filter_map(|e| {
            let (a, b) = e.trim().split_once(':')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        })
        .collect()
}

sa_core::orig! {
    fn entity_teleport(e: sa_core::mem::GPtr<u8>, x: u32, y: u32, z: u32, reset_rotation: u32) = thiscall vslot 0x38 => unported;
}

/// `SA_GAME_TELEPORT=ITER:X:Y:Z[:HEADING]` (exploration only): at main-loop iteration ITER the player ped is moved
/// to (X, Y, Z) through its `Teleport` virtual, and its current and aimed rotations set to HEADING (radians).
fn teleport_player(n: u32) {
    let Ok(v) = crate::env::var("SA_GAME_TELEPORT") else { return };
    let f: Vec<f32> = v.split(':').filter_map(|x| x.trim().parse().ok()).collect();
    if f.len() < 4 || f[0] as u32 != n {
        return;
    }
    let ped = sa_core::mem::GPtr::<u32>::from_addr(0xB7CD98).read();
    if ped < 0x1000 {
        println!("teleport: no player ped");
        return;
    }
    let ped = sa_core::mem::GPtr::<u8>::from_addr(ped as usize);
    entity_teleport(ped, f[1].to_bits(), f[2].to_bits(), f[3].to_bits(), 0);
    if let Some(h) = f.get(4) {
        for off in [0x558, 0x55C] {
            sa_core::mem::GPtr::<f32>::from_addr(ped.addr() + off).write(*h);
        }
    }
    println!("teleport: player to ({}, {}, {})", f[1], f[2], f[3]);
}

/// `SA_GAME_WALK=START:COUNT` (see [`synthetic_input`]).
fn walk_spec() -> Option<(u32, u32)> {
    let v = crate::env::var("SA_GAME_WALK").ok()?;
    let (a, b) = v.split_once(':')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}
