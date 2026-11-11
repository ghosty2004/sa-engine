//! `SA_CRASH_LOG=1`: a last-chance vectored exception handler that writes the registers and the top of the
//! stack of the first access violations to sa-hook.log (verifier aid: `winedbg --auto` prints nothing under
//! the e2e runs, and a crashing port leaves no other trace of the faulting call chain). The handler only
//! logs and always continues the search, so the game's own SEH handlers still run first-chance as before.

use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::Foundation::EXCEPTION_ACCESS_VIOLATION;
use windows_sys::Win32::System::Diagnostics::Debug::{AddVectoredExceptionHandler, EXCEPTION_POINTERS};

use crate::win::{log, read};

const CONTINUE_SEARCH: i32 = 0;
/// Access violations logged at most (the game handles some itself).
const MAX_LOGGED: u32 = 8;
/// Stack dwords dumped from ESP (and from EBP).
const STACK_DWORDS: u32 = 96;

static LOGGED: AtomicU32 = AtomicU32::new(0);

fn dump(name: &str, from: u32) -> String {
    let mut s = format!("  {name} {from:#010x}:");
    for i in 0..STACK_DWORDS {
        let a = from.wrapping_add(4 * i);
        if i % 8 == 0 {
            s.push_str(&format!("\n    {a:#010x}:"));
        }
        match read(a, 4) {
            Some(b) => s.push_str(&format!(" {:08x}", u32::from_le_bytes([b[0], b[1], b[2], b[3]]))),
            None => {
                s.push_str(" ????????");
                break;
            }
        }
    }
    s
}

unsafe extern "system" fn handler(info: *mut EXCEPTION_POINTERS) -> i32 {
    // SAFETY: the OS passes valid exception and context records.
    let (rec, ctx) = unsafe { (&*(*info).ExceptionRecord, &*(*info).ContextRecord) };
    if rec.ExceptionCode != EXCEPTION_ACCESS_VIOLATION || LOGGED.fetch_add(1, Ordering::Relaxed) >= MAX_LOGGED {
        return CONTINUE_SEARCH;
    }
    log(&format!(
        "crashlog: access violation at {:#010x} (info {:#x} {:#010x}) eax={:08x} ebx={:08x} ecx={:08x} \
         edx={:08x} esi={:08x} edi={:08x} ebp={:08x} esp={:08x}\n{}\n{}",
        rec.ExceptionAddress as usize,
        rec.ExceptionInformation[0],
        rec.ExceptionInformation[1],
        ctx.Eax,
        ctx.Ebx,
        ctx.Ecx,
        ctx.Edx,
        ctx.Esi,
        ctx.Edi,
        ctx.Ebp,
        ctx.Esp,
        dump("esp", ctx.Esp),
        dump("ebp", ctx.Ebp),
    ));
    CONTINUE_SEARCH
}

/// Installs the handler when `SA_CRASH_LOG` is set (called once while the game is still suspended).
pub fn init() {
    if std::env::var_os("SA_CRASH_LOG").is_none() {
        return;
    }
    // first = 0: after the other vectored handlers (INF-10 coverage breakpoints).
    if unsafe { AddVectoredExceptionHandler(0, Some(handler)) }.is_null() {
        log("crashlog: AddVectoredExceptionHandler failed");
    } else {
        log("crashlog: on");
    }
}
