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

use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

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

// ---- samp mode (SAMP-4, docs/samp/game-adapter.md §6) -------------------------------------------------------

/// The `sa_samp_api::config::Feature`s samp disables, as bits (`1 << feature`). 0 = single-player.
static DISABLED: AtomicU64 = AtomicU64::new(0);
/// Runtime toggles (bits of [`toggle`]).
static TOGGLES: AtomicU32 = AtomicU32::new(0);

/// Bit numbers of the features the ports test, in `sa_samp_api::config::Feature` declaration order (sa-core does
/// not depend on `sa-samp-api`; sa-samp's tests check that the numbers match). Each names the original call or
/// entry samp NOPs; the port skips exactly that call when the bit is set.
pub mod feature {
    /// `CPopulation::AddPed` 0x612710 returns NULL.
    pub const AMBIENT_PEDS: u8 = 0;
    /// `CCarCtrl::GenerateRandomCars` call 0x53C1C1 in `CGame::Process`.
    pub const RANDOM_TRAFFIC: u8 = 2;
    /// `CTheCarGenerators::Process` call 0x53C06A in `CGame::Process`.
    pub const CAR_GENERATORS: u8 = 3;
    /// `CPlane::DoPlaneGenerationAndRemoval` call 0x434272 in `CCarCtrl::GenerateRandomCars`.
    pub const AMBIENT_PLANES: u8 = 4;
    /// `CCutsceneMgr::Update` call 0x53BF28 in `CGame::Process`.
    pub const CUTSCENES: u8 = 8;
    /// `CReplay::Update` call 0x53C090 in `CGame::Process`.
    pub const REPLAY: u8 = 9;
    /// `CPlantMgr::Update` call 0x53C159 in `CGame::Process`.
    pub const PLANT_MANAGER: u8 = 10;
    /// `CCoronas::DoSunAndMoon` call 0x53C136 in `CGame::Process`.
    pub const SUN_MOON_CORONAS: u8 = 11;
    /// `CIdleCam::Process` call 0x524582 in `CCam::Process_FollowPed_SA`.
    pub const IDLE_CAMERA: u8 = 15;
    /// `CHud::SetHelpMessage` 0x588BE0 returns at once.
    pub const HELP_MESSAGES: u8 = 16;
    /// `CPlaceName::Process` call 0x5720A5 in `CUserDisplay::Process`.
    pub const ZONE_NAMES: u8 = 19;
}

/// Runtime toggle bits (`sa_samp_api::config::RuntimeToggle`).
pub mod toggle {
    /// Typed cheats off: `CCheat::DoCheats` is not called from `CGame::Process` (samp blocks the typed
    /// characters at 0x438576; natively the whole cheat poll is skipped).
    pub const NO_CHEATS: u32 = 1 << 0;
    /// samp's start-up (`StartupOptions`, docs/samp/startup.md): `gGameState` := 5 at attach with the
    /// `mov [gGameState],esi` in `psInitialize` (0x747483) NOPed, so the logo and intro-movie states are skipped;
    /// at `gGameState` 7 (frontend idle) samp's init sets the menu flags (+0xE9 = 1, +0x5C = 0, +0x33 = 0) and
    /// `gGameState` := 8, so the game loads straight into the world without the main menu.
    pub const SAMP_STARTUP: u32 = 1 << 1;
    /// The game clock off (samp's `RuntimeToggle::GameClock(false)`, its Game +0x69 = 0): samp writes `ret` over
    /// `CClock::Update`'s first byte (0x52CF10), so the clock stops, and zeroes the HUD clock format "%02d:%02d"
    /// (0x859A6C), so `CHud::DrawPlayerInfo` prints an empty string. Natively both ports test this bit.
    pub const NO_GAME_CLOCK: u32 = 1 << 2;
}

/// Sets the disabled features (`FeatureSet` bits) at install; 0 restores single-player behaviour.
pub fn set_disabled_features(mask: u64) {
    DISABLED.store(mask, Ordering::Relaxed);
}

/// Whether samp mode disables `feature` (one of [`feature`]).
#[inline]
pub fn disabled(feature: u8) -> bool {
    DISABLED.load(Ordering::Relaxed) & (1u64 << feature) != 0
}

/// Sets or clears runtime toggle bits ([`toggle`]).
pub fn set_toggle(bit: u32, on: bool) {
    if on {
        TOGGLES.fetch_or(bit, Ordering::Relaxed);
    } else {
        TOGGLES.fetch_and(!bit, Ordering::Relaxed);
    }
}

/// Whether a runtime toggle bit is set.
#[inline]
pub fn toggled(bit: u32) -> bool {
    TOGGLES.load(Ordering::Relaxed) & bit != 0
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
    fn samp_mode_flags_default_to_single_player() {
        assert!(!disabled(feature::RANDOM_TRAFFIC));
        set_disabled_features(1 << feature::RANDOM_TRAFFIC);
        assert!(disabled(feature::RANDOM_TRAFFIC) && !disabled(feature::AMBIENT_PEDS));
        set_disabled_features(0);
        assert!(!toggled(toggle::NO_CHEATS));
        set_toggle(toggle::NO_CHEATS, true);
        assert!(toggled(toggle::NO_CHEATS));
        set_toggle(toggle::NO_CHEATS, false);
        assert!(!toggled(toggle::NO_CHEATS));
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
