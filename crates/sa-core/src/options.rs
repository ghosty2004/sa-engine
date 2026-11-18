//! Switches of the game's own features that a host may turn off (a mod through the mod API). Every feature is on
//! by default, so a build or run that turns nothing off behaves as the original; a port that honours a switch
//! skips exactly the original call or entry the feature names when it is off.

use core::sync::atomic::{AtomicU32, Ordering};

/// A switchable feature of the game; the bit of [`set`] / [`enabled`] is its declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Feature {
    /// Typed cheat codes: `CCheat::DoCheats` from `CGame::Process`.
    TypedCheats,
    /// The game clock: `CClock::Update` runs and `CHud::DrawPlayerInfo` prints "%02d:%02d".
    Clock,
    /// Ambient peds: `CPopulation::AddPed` 0x612710 (returns NULL when off).
    AmbientPeds,
    /// Random traffic: `CCarCtrl::GenerateRandomCars` call 0x53C1C1 in `CGame::Process`.
    RandomTraffic,
    /// Parked-car generators: `CTheCarGenerators::Process` call 0x53C06A in `CGame::Process`.
    CarGenerators,
    /// Ambient planes: `CPlane::DoPlaneGenerationAndRemoval` call 0x434272 in `CCarCtrl::GenerateRandomCars`.
    AmbientPlanes,
    /// Cutscenes: `CCutsceneMgr::Update` call 0x53BF28 in `CGame::Process`.
    Cutscenes,
    /// The replay buffer: `CReplay::Update` call 0x53C090 in `CGame::Process`.
    Replay,
    /// Plants and grass: `CPlantMgr::Update` call 0x53C159 in `CGame::Process`.
    Plants,
    /// The sun and moon coronas: `CCoronas::DoSunAndMoon` call 0x53C136 in `CGame::Process`.
    SunAndMoon,
    /// The idle camera: `CIdleCam::Process` call 0x524582 in `CCam::Process_FollowPed_SA`.
    IdleCamera,
    /// Help boxes: `CHud::SetHelpMessage` 0x588BE0 (returns at once when off).
    HelpMessages,
    /// Zone names: `CPlaceName::Process` call 0x5720A5 in `CUserDisplay::Process`.
    ZoneNames,
    /// The start screens: the logos and the intro movie (`gGameState` 5 at start-up with the write in
    /// `psInitialize` 0x747483 skipped) and the main menu (the menu flags set and `gGameState` 8 at frontend idle),
    /// so the game loads straight into the world.
    StartScreens,
}

/// The features turned off, as bits.
static OFF: AtomicU32 = AtomicU32::new(0);

/// Turns `feature` on or off.
pub fn set(feature: Feature, on: bool) {
    let bit = 1u32 << feature as u8;
    if on {
        OFF.fetch_and(!bit, Ordering::Relaxed);
    } else {
        OFF.fetch_or(bit, Ordering::Relaxed);
    }
}

/// Whether `feature` is on.
#[inline]
pub fn enabled(feature: Feature) -> bool {
    OFF.load(Ordering::Relaxed) & (1u32 << feature as u8) == 0
}

/// Turns every feature back on.
pub fn reset() {
    OFF.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_default_to_on() {
        assert!(enabled(Feature::RandomTraffic));
        set(Feature::RandomTraffic, false);
        assert!(!enabled(Feature::RandomTraffic) && enabled(Feature::AmbientPeds));
        reset();
        assert!(enabled(Feature::RandomTraffic));
    }
}
