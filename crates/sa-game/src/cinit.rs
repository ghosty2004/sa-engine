//! The C++ static initialisers of gta_sa.exe 1.0 US, run as the CRT runs them before `WinMain`.
//!
//! The CRT entry (0x824570) calls `_cinit` (0x823B76), which runs the C initialiser table
//! `[0x8A5A14, 0x8A5A30)` (`_initterm_e`: CRT internals the native process does not need) and then
//! the C++ table `__xc_a..__xc_z` = `[0x8A4000, 0x8A5A10)` (1,668 slots, NULL slots skipped), in
//! table order. Every slot is the address of a `__init_*` function (constructors of the game's
//! global objects, constant initialisers, `atexit` registrations). Phase B runs this second table
//! as the first boot step: each slot whose address has a registered port (`orig!`'s registry) is
//! called; slots without one are skipped and listed (name and address), so the gap is visible in
//! the step's detail and the `--report`. A port that panics (an unported callee) is recorded and
//! the next slot runs.

#![allow(unsafe_code)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use sa_core::mem::GPtr;
use sa_core::orig::registry;

/// `__xc_a` (first slot of the C++ initialiser table).
pub const XC_A: u32 = 0x8A_4000;
/// `__xc_z` (end of the table).
pub const XC_Z: u32 = 0x8A_5A10;

/// The skipped slots of the last run `(address, name)`.
static SKIPPED: std::sync::Mutex<Vec<(u32, String)>> = std::sync::Mutex::new(Vec::new());

/// The skipped (unported) initialisers of the last run.
pub fn skipped() -> Vec<(u32, String)> {
    SKIPPED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Runs the C++ initialiser table; returns the step's summary.
pub fn run() -> String {
    let (mut ran, mut skipped, mut failed) = (0u32, Vec::new(), Vec::new());
    let mut a = XC_A;
    while a < XC_Z {
        let f = GPtr::<u32>::from_addr(a as usize).read();
        a += 4;
        if f == 0 {
            continue;
        }
        match registry::lookup(f as usize) {
            Some(e) => {
                let entry = e.entry;
                // SAFETY: a registered entry of a `__init_*` port: an `extern "C-unwind" fn()` with
                // the original's cdecl convention and no arguments (every slot of the table is
                // called that way by `_initterm`).
                let call: extern "C-unwind" fn() =
                    unsafe { std::mem::transmute::<usize, extern "C-unwind" fn()>(entry) };
                match catch_unwind(AssertUnwindSafe(|| call())) {
                    Ok(()) => ran += 1,
                    Err(p) => {
                        let why = p
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_default();
                        failed.push(format!("{} ({f:#x}): {why}", e.name))
                    }
                }
            }
            None => skipped.push((f, registry::describe(f as usize).to_string())),
        }
    }
    let n = skipped.len();
    *SKIPPED.lock().unwrap_or_else(|e| e.into_inner()) = skipped;
    let mut s = format!("{ran} static initialisers run, {n} unported skipped");
    if !failed.is_empty() {
        s += &format!(", {} panicked: {}", failed.len(), failed.join("; "));
    }
    s
}
