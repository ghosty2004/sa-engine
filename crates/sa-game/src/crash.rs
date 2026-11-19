//! Crash reports: when the game faults, a file with the fault, the Rust call chain and the last lines the mods
//! logged, so a crash in play can be looked at without reproducing it.
//!
//! The reports go to `SA_GAME_CRASH_DIR`, else to `crashes/` next to the executable.

use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Lines kept for the report.
const RECENT_LINES: usize = 300;

static RECENT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// Keeps `line` (a mod's log line) for a later report.
pub fn note(line: &str) {
    if let Ok(mut r) = RECENT.lock() {
        if r.len() == RECENT_LINES {
            r.pop_front();
        }
        r.push_back(line.to_owned());
    }
}

fn dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("SA_GAME_CRASH_DIR") {
        return Some(PathBuf::from(d));
    }
    Some(std::env::current_exe().ok()?.parent()?.join("crashes"))
}

/// Writes a report headed by `header` and returns its path. Runs inside the fault handler: the process is ending
/// anyway, so it allocates and takes locks it may not get (`try_lock`: a fault while the log was being written
/// leaves the lines out rather than hanging).
pub fn write_report(header: &str) -> Option<PathBuf> {
    let dir = dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let path = dir.join(format!("crash-{secs}.txt"));
    let mut f = std::fs::File::create(&path).ok()?;
    let _ = writeln!(f, "{header}");
    let _ = writeln!(f, "sa-game {} ({})", env!("CARGO_PKG_VERSION"), std::env::args().collect::<Vec<_>>().join(" "));
    let _ = writeln!(f, "\nCall chain:\n{}", std::backtrace::Backtrace::force_capture());
    let _ = writeln!(f, "Last mod log lines:");
    if let Ok(r) = RECENT.try_lock() {
        for l in r.iter() {
            let _ = writeln!(f, "  {l}");
        }
    }
    Some(path)
}
