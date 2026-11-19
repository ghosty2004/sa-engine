//! Switches of the game's own features that a host may turn off, and limits it may raise (a mod through the mod
//! API). Every feature is on and every limit is the original's by default, so a build or run that changes nothing
//! behaves as the original; a port that honours a switch skips exactly the original call or entry the feature names
//! when it is off, and one that sizes something by a limit takes the original's constant unless it was raised.


use core::sync::atomic::{AtomicU32, Ordering};

/// A switchable feature of the game; the bit of [`set`] / [`enabled`] is its declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Feature {
    /// Typed cheat codes: `CCheat::DoCheats` from `CGame::Process`.
    TypedCheats,
    /// The game clock: `CClock::Update` runs and `CHud::DrawPlayerInfo` prints "%02d:%02d".
    Clock,
    /// Ambient peds: `CPopulation::AddPed` 0x612710 (returns NULL when off), with the callers that would use that
    /// NULL: the `CCarEnterExit::SetPedInCarDirect` call 0x613BA7 in `CPopulation::AddPedInCar` and the
    /// `CEntryExit::GenerateAmbientPeds` call 0x440833 in `CEntryExit::TransitionFinished`.
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
    /// The vehicle name on the HUD: `CHud::DrawVehicleName` call 0x58FBE9 in `CHud::Draw`.
    VehicleNameOnHud,
    /// The vital stats on the HUD while their button is held: the `CHud::DrawVitalStats` branch 0x58FC2E of
    /// `CHud::Draw` (off, the radar is drawn instead).
    VitalStatsOnHud,
    /// The single-player pickups of the map: `CFileLoader::LoadPickup` 0x5B47B0 (returns at once when off; the
    /// IPL pickups) and the `Interior_c::AddPickups` call 0x593D7C in `Interior_c::Init` (interior pickups).
    SinglePlayerPickups,
    /// Ambient gunfire: the shot branch 0x4DF7E2 of `CAEGlobalWeaponAudioEntity::ServiceAmbientGunFire` (off, a
    /// due burst only resets the state).
    AmbientGunfire,
    /// Real-time shadows: `CRealTimeShadowManager::Update` call 0x53EA08 in `Idle` and the
    /// `ReturnRealTimeShadow` call 0x542485 in `CPhysical::~CPhysical`.
    RealTimeShadows,
    /// The pause menu when the window gets the focus back: `CGame::InitAfterLostFocus` call 0x748063 in
    /// `__MainWndProc` (`WM_SETFOCUS`).
    PauseMenuOnFocus,
    /// The models around the old position freed on a respawn: `CStreaming::DeleteRwObjectsAfterDeath` call
    /// 0x441482 in `CGameLogic::SortOutStreamingAndMemory`.
    DeathModelCleanup,
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

/// A limit of the game a host may raise before the game sizes it (read when the game starts: the pools in
/// `CPools::Initialise`, the matrices in `CPlaceable::InitMatrixArray`, the texture dictionaries in
/// `CTxdStore::Initialise`, the streaming budget in `CStreaming::Init2`, the vehicle structures in
/// `CVehicleModelInfo::SetupCommonData`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Limit {
    /// `CPools::ms_pPtrNodeSingleLinkPool` (70000).
    PtrNodesSingle,
    /// `CPools::ms_pPtrNodeDoubleLinkPool` (3200).
    PtrNodesDouble,
    /// `CPools::ms_pEntryInfoNodePool` (500).
    EntryInfoNodes,
    /// `CPools::ms_pPedPool` (140).
    Peds,
    /// `CPools::ms_pVehiclePool` (110).
    Vehicles,
    /// `CPools::ms_pBuildingPool` (13000).
    Buildings,
    /// `CPools::ms_pObjectPool` (350).
    Objects,
    /// `CPools::ms_pDummyPool` (2500).
    Dummies,
    /// `CPools::ms_pColModelPool` (10150).
    ColModels,
    /// `CPools::ms_pTaskPool` (500).
    Tasks,
    /// `CPools::ms_pEventPool` (200).
    Events,
    /// `CPools::ms_pPedIntelligencePool` (140).
    PedIntelligence,
    /// `gMatrixList`, the matrices placeables borrow (900).
    Matrices,
    /// `CTxdStore::ms_pTxdPool` (5000).
    TextureDictionaries,
    /// `CStreaming::ms_memoryAvailable` in bytes (0x3200000, 50 MiB).
    StreamingMemory,
    /// `CVehicleModelInfo::ms_pInfoPool`, one `CVehicleStructure` per loaded vehicle model (50).
    VehicleStructures,
}

impl Limit {
    /// Every limit, in declaration order.
    pub const ALL: [Limit; 16] = [
        Limit::PtrNodesSingle,
        Limit::PtrNodesDouble,
        Limit::EntryInfoNodes,
        Limit::Peds,
        Limit::Vehicles,
        Limit::Buildings,
        Limit::Objects,
        Limit::Dummies,
        Limit::ColModels,
        Limit::Tasks,
        Limit::Events,
        Limit::PedIntelligence,
        Limit::Matrices,
        Limit::TextureDictionaries,
        Limit::StreamingMemory,
        Limit::VehicleStructures,
    ];

    /// The original's value.
    pub const fn original(self) -> u32 {
        match self {
            Limit::PtrNodesSingle => 70000,
            Limit::PtrNodesDouble => 3200,
            Limit::EntryInfoNodes => 500,
            Limit::Peds => 140,
            Limit::Vehicles => 110,
            Limit::Buildings => 13000,
            Limit::Objects => 350,
            Limit::Dummies => 2500,
            Limit::ColModels => 10150,
            Limit::Tasks => 500,
            Limit::Events => 200,
            Limit::PedIntelligence => 140,
            Limit::Matrices => 900,
            Limit::TextureDictionaries => 5000,
            Limit::StreamingMemory => 0x0320_0000,
            Limit::VehicleStructures => 50,
        }
    }

    /// The highest value a host may set: the game keeps some slot indices in signed 16-bit fields (collision
    /// models, texture dictionaries), so the pools stay below 0x8000 slots; the link node pools only hold list
    /// links; the streaming budget stays a quarter of the 32-bit address space the game heap shares.
    pub const fn max(self) -> u32 {
        match self {
            Limit::PtrNodesSingle | Limit::PtrNodesDouble | Limit::EntryInfoNodes => 1 << 20,
            Limit::Matrices => 0x10000,
            Limit::StreamingMemory => 0x4000_0000,
            _ => 0x7FFF,
        }
    }
}

/// The limits a host raised (0: the original's).
static LIMITS: [AtomicU32; Limit::ALL.len()] = [const { AtomicU32::new(0) }; Limit::ALL.len()];

/// Sets `limit` to `value` (between its original and its [`Limit::max`]; outside, `false` and nothing changes).
/// Only values set before the game reads the limit take effect.
pub fn set_limit(limit: Limit, value: u32) -> bool {
    if value < limit.original() || value > limit.max() {
        return false;
    }
    LIMITS[limit as usize].store(value, Ordering::Relaxed);
    true
}

/// The value of `limit`: the original's unless a host raised it.
#[inline]
pub fn limit(limit: Limit) -> u32 {
    match LIMITS[limit as usize].load(Ordering::Relaxed) {
        0 => limit.original(),
        v => v,
    }
}

/// Every limit back to the original's.
pub fn reset_limits() {
    for l in &LIMITS {
        l.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_default_to_the_original_and_only_grow() {
        for l in Limit::ALL {
            assert_eq!(limit(l), l.original());
            assert!(l.max() >= l.original());
        }
        for (i, l) in Limit::ALL.iter().enumerate() {
            assert_eq!(*l as usize, i, "Limit::ALL must follow declaration order");
        }
        assert!(!set_limit(Limit::Peds, 139));
        assert!(!set_limit(Limit::Peds, 0x8000));
        assert!(set_limit(Limit::Peds, 240));
        assert_eq!(limit(Limit::Peds), 240);
        reset_limits();
        assert_eq!(limit(Limit::Peds), 140);
    }

    #[test]
    fn features_default_to_on() {
        assert!(enabled(Feature::RandomTraffic));
        set(Feature::RandomTraffic, false);
        assert!(!enabled(Feature::RandomTraffic) && enabled(Feature::AmbientPeds));
        reset();
        assert!(enabled(Feature::RandomTraffic));
    }
}
