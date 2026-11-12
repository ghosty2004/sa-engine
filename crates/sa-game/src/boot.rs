//! The step runner of the native boot. The steps are WinMain's ([`crate::winmain`]): each runs
//! ported Rust code; a step that reaches an `orig!` declaration with no Rust port reports the
//! callee (name and address, from the `orig!` panic) and the boot carries on with the next step,
//! so one run lists the blockers on the path the ported code reaches today. A memory fault is
//! found by running the step in a forked child first.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use sa_core::mem::GPtr;

use crate::arena;

/// What a step did.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Ran to completion in Rust.
    Ok(String),
    /// The step's own function is not in Rust (state < implemented in catalog/state.csv).
    OriginalOnly,
    /// A Rust step called an `orig!` declaration that has no Rust path in phase B.
    Blocked { callee: String, addr: String },
    /// Another panic.
    Panicked(String),
    /// A memory fault (found in a forked probe; the step was then skipped).
    Crashed(String),
}

/// One step of the boot or frame sequence.
pub struct Step {
    /// Catalogue name.
    pub name: &'static str,
    pub addr: u32,
    /// The Rust entry, `None` when the function is original-only.
    pub run: Option<fn() -> String>,
    /// Try the step in a forked child first (off for steps that need threads started by
    /// earlier steps: fork copies only the calling thread).
    pub probe: bool,
}

pub const fn rust(name: &'static str, addr: u32, run: fn() -> String) -> Step {
    Step {
        name,
        addr,
        run: Some(run),
        probe: true,
    }
}

thread_local! {
    static LAST_PANIC: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Silences the default panic output (the runner reports panics itself).
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_owned()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "panic".to_owned()
        };
        let at = info
            .location()
            .map(|l| format!(" ({}:{})", l.file(), l.line()))
            .unwrap_or_default();
        LAST_PANIC.with(|p| *p.borrow_mut() = format!("{msg}{at}"));
        // `SA_GAME_BACKTRACE=1` (exploration): the Rust call chain of every panic, e.g. of a blocked step
        if std::env::var_os("SA_GAME_BACKTRACE").is_some() {
            eprintln!("panic: {msg}{at}\n{}", std::backtrace::Backtrace::force_capture());
        }
    }));
}

/// Parses `orig!`'s phase B panic: `NAME is not ported; phases B/C have no original code to call (ADDR)`.
fn parse_unported(msg: &str) -> Option<(String, String)> {
    let (name, rest) = msg.split_once(" is not ported; ")?;
    let addr = rest
        .split_once('(')
        .map(|(_, a)| a.split(')').next().unwrap_or("").to_owned())
        .unwrap_or_default();
    Some((name.to_owned(), addr))
}

/// Set by `--window`: no step is probed in a forked child (the window's GPU device and display connection exist
/// before the boot).
pub static NO_PROBE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Runs a step in this process without the forked probe (main-loop steps known not to fault).
pub fn run_step_direct(s: &Step) -> Outcome {
    let Some(f) = s.run else {
        return Outcome::OriginalOnly;
    };
    arena::set_step(s.name);
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(detail) => Outcome::Ok(detail),
        Err(_) => {
            let msg = LAST_PANIC.with(|p| p.borrow().clone());
            match parse_unported(&msg) {
                Some((callee, addr)) => Outcome::Blocked { callee, addr },
                None => Outcome::Panicked(msg),
            }
        }
    }
}

pub fn run_step(s: &Step) -> Outcome {
    let Some(f) = s.run else {
        return Outcome::OriginalOnly;
    };
    arena::set_step(s.name);
    // a memory fault cannot be unwound: try the step in a child process first (SA_GAME_NO_PROBE=1
    // runs every step directly, e.g. under gdb). Once RW START has created the GPU device the probe
    // is off for good: forking a process with wgpu threads is unsafe (the crash reporter still names
    // the step of a fault).
    if s.probe
        && std::env::var_os("SA_GAME_NO_PROBE").is_none()
        && !NO_PROBE.load(std::sync::atomic::Ordering::SeqCst)
        && crate::d3d9::with_device(|_| ()).is_none()
        && let Some(report) = arena::probe(|| {
            f();
        })
    {
        return Outcome::Crashed(report);
    }
    let _watchdog = (!s.probe).then(|| arena::Watchdog::start(s.name));
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(detail) => Outcome::Ok(detail),
        Err(_) => {
            let msg = LAST_PANIC.with(|p| p.borrow().clone());
            match parse_unported(&msg) {
                Some((callee, addr)) => Outcome::Blocked { callee, addr },
                None => Outcome::Panicked(msg),
            }
        }
    }
}

// -------------------------------------------------------------------------- extra checks ---

/// `CdStreamInit` + `CdStreamOpen(gta3.img)` + `CdStreamRead` + `CdStreamSync`: one sector
/// through the streaming thread (a check outside WinMain's order: the real path opens the image
/// from `CStreaming::Init2`, which the boot does not reach yet).
fn cd_stream_read_img() -> String {
    use sa_stream::cdstream as cd;
    let name = b"MODELS\\GTA3.IMG\0";
    cd::cd_stream_init(5);
    let file = cd::cd_stream_open_2(GPtr::from_raw(name.as_ptr().cast_mut()), 0);
    let mut buf = vec![0u8; 2048];
    // position = file id in the top byte, sector below (CStreaming's encoding)
    let r = cd::cd_stream_read(0, GPtr::from_raw(buf.as_mut_ptr()), file << 24, 1);
    let s = cd::cd_stream_sync(0);
    let entries = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    let first: String = buf[8 + 8..8 + 32]
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as char)
        .collect();
    format!(
        "file id {file}, CdStreamRead {r}, CdStreamSync {s} (streaming thread): header {:?}, {entries} entries, first {first:?}",
        String::from_utf8_lossy(&buf[..4])
    )
}

/// Checks run after the boot, outside WinMain's order.
pub fn checks() -> Vec<Step> {
    vec![rust(
        "check: CdStreamInit + CdStreamOpen(gta3.img) + CdStreamRead + CdStreamSync (streaming thread)",
        0x406A20,
        cd_stream_read_img,
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_orig_panics() {
        let m = "call::rs_rw_initialize is not ported; phases B/C have no original code to call (0x619C90 as usize) (crates/x.rs:1:2)";
        assert_eq!(
            parse_unported(m),
            Some(("call::rs_rw_initialize".into(), "0x619C90 as usize".into()))
        );
        assert_eq!(parse_unported("index out of bounds"), None);
    }
}
