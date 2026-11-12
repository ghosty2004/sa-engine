//! SA:MP dispatch points (docs/samp/game-adapter.md §3, docs/samp/native-plan.md §3).
//!
//! samp-rs runs at fixed points of the frame. On the original exe it gets there through call retargets
//! (`Idle` → `CGame::Process` 0x53E981, `Idle` → `Render2dStuff` 0x53EB12, `CGame::Process` →
//! `CCamera::Process` 0x53C104). Once those functions are ported, a byte patch has no effect, so the ports call
//! [`frame`] at the same sites instead. Nothing is registered in single-player builds: [`frame`] is then one
//! relaxed load and a not-taken branch, and the ports behave as before.
//!
//! The host (native sa-game with `--samp`, `sa-samp`) registers one handler with [`set_frame_handler`]; it
//! receives the [`FramePhase`] as its `u8` value (sa-core does not depend on `sa-samp-api`; the values are
//! `sa_samp_api::hooks::FramePhase`'s declaration order).
// The handler is a plain `fn` stored in an atomic word; turning the word back into the `fn` is the one unsafe
// operation of this module.
#![allow(unsafe_code)]

use core::sync::atomic::{AtomicUsize, Ordering};

/// A frame point, in frame order (same order as `sa_samp_api::hooks::FramePhase`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FramePhase {
    /// In `Game::Idle`, before `CGame::Process` (samp: retargeted call 0x53E981).
    BeforeGameProcess = 0,
    /// In `CGame::Process`, before `TheCamera.Process()` (samp: call 0x53C104).
    CameraProcess = 1,
    /// In `CGame::Process`, after `TheCamera.Process()` (same samp hook).
    AfterGameProcess = 2,
    /// In `Game::Idle`, before `Render2dStuff` (samp: retargeted call 0x53EB12).
    Before2d = 3,
    /// In `Game::Idle`, after `Render2dStuff`.
    After2d = 4,
    /// Before the frame is presented (the platform's present path).
    Present = 5,
}

/// The registered handler as a `fn(u8)` address, 0 = none.
static HANDLER: AtomicUsize = AtomicUsize::new(0);

/// Registers the frame handler (replaces any earlier one).
pub fn set_frame_handler(f: fn(u8)) {
    HANDLER.store(f as usize, Ordering::Release);
}

/// Removes the frame handler.
pub fn clear_frame_handler() {
    HANDLER.store(0, Ordering::Release);
}

/// Whether a handler is registered (samp mode is hosted).
pub fn active() -> bool {
    HANDLER.load(Ordering::Relaxed) != 0
}

/// A dispatch point: runs the registered handler for `phase`, if any.
#[inline]
pub fn frame(phase: FramePhase) {
    let h = HANDLER.load(Ordering::Acquire);
    if h != 0 {
        // SAFETY: `HANDLER` only ever holds 0 or a value stored by `set_frame_handler` from a `fn(u8)`.
        let f: fn(u8) = unsafe { core::mem::transmute::<usize, fn(u8)>(h) };
        f(phase as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::AtomicU32;

    static SEEN: AtomicU32 = AtomicU32::new(0);

    fn record(p: u8) {
        SEEN.fetch_add(1 << (4 * u32::from(p)), Ordering::SeqCst);
    }

    #[test]
    fn dispatches_only_when_registered() {
        frame(FramePhase::Before2d);
        assert_eq!(SEEN.load(Ordering::SeqCst), 0);
        assert!(!active());
        set_frame_handler(record);
        assert!(active());
        frame(FramePhase::Before2d);
        frame(FramePhase::CameraProcess);
        clear_frame_handler();
        frame(FramePhase::Before2d);
        assert_eq!(SEEN.load(Ordering::SeqCst), (1 << 12) | (1 << 4));
    }
}
