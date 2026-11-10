//! Lost-wake-up fix for `CdStreamSync` (INF-15, docs/re/lanes.md).
//!
//! gta_sa.exe 1.0 US, threaded streaming (`gbCdStreamAsync`/0x8E3FE4 != 0):
//!
//! ```text
//! CdStreamSync (.HOODLUM 0x156CD80)          CdStreamThread (0x406560)
//!   0x156CD9D  if (s->nSectorsToRead) {       0x406669  s->nSectorsToRead = 0;
//!   0x156CDA4    s->bLocked = 1;              0x40666C  if (s->bLocked)
//!   0x156CDAE    WaitForSingleObject(s->sema,  0x406678      ReleaseSemaphore(s->sema, 1, 0);
//!                               INFINITE); }  0x40667E  s->bInUse = 0;
//! ```
//!
//! When the streaming thread runs 0x406669..0x40666C between the main thread's check (0x156CD9D) and
//! its `bLocked = 1` (0x156CDA4) — or both stores sit in the store buffers at once (x86-TSO allows the
//! store→load reorder) — the thread sees `bLocked == 0` and never releases the semaphore, and the main
//! thread waits forever (measured: the loading screen of a new game hangs in `CStreaming::LoadScene`
//! → `LoadAllRequestedModels` → `CdStreamSync`). Being descheduled inside that window makes it far more
//! likely, which is why it showed up when several game lanes ran at once.
//!
//! The fix redirects only the `call [WaitForSingleObject]` at 0x156CDAE: the wait polls, and once the
//! thread has finished the stream (`nSectorsToRead == 0 && bInUse == 0`; the release, if any, happens
//! before `bInUse = 0`) the wait ends. A release that raced with the poll is consumed so that the
//! semaphore stays balanced. Game state is untouched; only the deadlock is removed.
//! `SA_CDSYNC_FIX=0` keeps the original call (to reproduce the hang).

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::WaitForSingleObject;

use crate::win::{self, log};

/// `call dword ptr [0x858088]` (WaitForSingleObject) inside CdStreamSync.
const SITE: u32 = 0x156CDAE;
const SITE_BYTES: [u8; 6] = [0xFF, 0x15, 0x88, 0x80, 0x85, 0x00];
/// `CdStream* gCdStreams`, element size 0x30: +4 nSectorsToRead, +0xD bLocked, +0xE bInUse, +0x14 sema.
const CD_STREAMS: u32 = 0x8E3FFC;
/// Request queue size (`gChannelRequestQ.size` = number of streams + 1).
const CD_QUEUE_SIZE: u32 = 0x8E3FF8;
const POLL_MS: u32 = 20;

static CELL: AtomicU32 = AtomicU32::new(0);
/// Waits that would have hung in the original (the thread finished without releasing).
static RECOVERED: AtomicU32 = AtomicU32::new(0);
/// Waits that outlived one poll interval (slow reads; informational).
static SLOW: AtomicU32 = AtomicU32::new(0);

fn rd_u32(addr: u32) -> u32 {
    // SAFETY: game globals / the game's stream array, mapped for the life of the process.
    unsafe { std::ptr::read_volatile(addr as usize as *const u32) }
}

fn rd_u8(addr: u32) -> u8 {
    // SAFETY: as above.
    unsafe { std::ptr::read_volatile(addr as usize as *const u8) }
}

/// The stream whose semaphore is `sema`.
fn stream_of(sema: HANDLE) -> Option<u32> {
    let base = rd_u32(CD_STREAMS);
    let n = rd_u32(CD_QUEUE_SIZE).saturating_sub(1).min(64);
    if base == 0 {
        return None;
    }
    (0..n)
        .map(|i| base + i * 0x30)
        .find(|&s| rd_u32(s + 0x14) == sema as usize as u32)
}

/// Replaces `WaitForSingleObject(s->sema, INFINITE)` in CdStreamSync (stdcall, same signature).
extern "system" fn cd_sync_wait(sema: HANDLE, timeout: u32) -> u32 {
    // SAFETY: plain Win32 waits on the game's own semaphore handle.
    unsafe {
        let Some(s) = stream_of(sema) else {
            return WaitForSingleObject(sema, timeout);
        };
        let mut polls = 0u32;
        loop {
            let r = WaitForSingleObject(sema, POLL_MS);
            if r != WAIT_TIMEOUT {
                return r;
            }
            polls += 1;
            if polls == 1 {
                SLOW.fetch_add(1, Ordering::Relaxed);
            }
            if rd_u32(s + 4) == 0 && rd_u8(s + 0xE) == 0 {
                // The thread is done with this stream. If it did release, that happened before its
                // `bInUse = 0`, so a zero-timeout wait consumes it now.
                if WaitForSingleObject(sema, 0) == WAIT_OBJECT_0 {
                    return WAIT_OBJECT_0;
                }
                let n = RECOVERED.fetch_add(1, Ordering::Relaxed) + 1;
                log(&format!(
                    "cdsync: lost wake-up #{n} recovered (stream {:#x}, frame {}, after {} ms)",
                    s,
                    win::frames(),
                    polls * POLL_MS
                ));
                return WAIT_OBJECT_0;
            }
        }
    }
}

/// Patches the wait site (at attach, game threads suspended). Real-time and determinism mode alike;
/// in determinism mode CdStreamRead completes synchronously, so the wait is never reached.
pub(crate) fn install() {
    if std::env::var("SA_CDSYNC_FIX").is_ok_and(|v| v == "0") {
        log("cdsync: SA_CDSYNC_FIX=0, CdStreamSync keeps the original wait");
        return;
    }
    match win::read(SITE, 6) {
        Some(b) if b == SITE_BYTES => {}
        other => {
            log(&format!(
                "cdsync: {SITE:#x} unexpected bytes {other:02x?}, not patched"
            ));
            return;
        }
    }
    CELL.store(cd_sync_wait as *const c_void as usize as u32, Ordering::Release);
    let cell = &CELL as *const AtomicU32 as usize as u32;
    match win::patch(SITE + 2, &cell.to_le_bytes()) {
        Ok(()) => log(&format!(
            "cdsync: CdStreamSync wait {SITE:#x} -> polling wait (lost wake-up fix, INF-15)"
        )),
        Err(e) => log(&format!("cdsync: patch failed: {e}")),
    }
}

/// `recovered=N slow=M` for the control channel.
pub(crate) fn status() -> String {
    format!(
        "cdsync recovered={} slow={}",
        RECOVERED.load(Ordering::Relaxed),
        SLOW.load(Ordering::Relaxed)
    )
}
