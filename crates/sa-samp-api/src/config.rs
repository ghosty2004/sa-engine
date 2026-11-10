//! "samp mode": samp.dll's memory patches as engine configuration (ROADMAP SAMP-4, PLAN F13).
//!
//! Every samp.dll patch row (`~/samp-rs/docs/re/data/gta-patches-r4-annotated.csv`, classified in
//! `catalog/samp-usage.csv`) maps to one of:
//! * a [`Limits`] value (pool sizes, store sizes, relocated tables);
//! * a [`Feature`] in [`SampModeConfig::disabled`] (the 60 `patch-disable` entries, 55 labels);
//! * a [`Behaviour`] switch (`patch-modify` rows that change game logic);
//! * a [`RuntimeToggle`] (patches samp applies and restores while playing);
//! * a hook in [`crate::hooks::GameHooks`] (call/jmp/vtable redirects).
//!
//! The original backend applies the same bytes samp.dll writes. The engine backends read these values in
//! the ported functions; the default of every value is the single-player behaviour.

/// Pool and table capacities. `None` keeps the single-player value. Values in brackets are samp's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    /// `CPtrNodeSingleLink` pool (0x550F46; 70000 → \[100000\]).
    pub ptr_nodes_single: Option<u32>,
    /// `CPtrNodeDoubleLink` pool (0x550F82; 3200 → \[8000\]).
    pub ptr_nodes_double: Option<u32>,
    /// `CEntryInfoNode` pool (0x550FBA; 500 → \[5000\]).
    pub entry_info_nodes: Option<u32>,
    /// Ped pool (0x550FF2; 140 → \[240\]) and ped intelligence pool (0x551283; 140 → \[240\]).
    pub peds: Option<u32>,
    /// Vehicle pool (0x551024; 110 → \[710\]).
    pub vehicles: Option<u32>,
    /// Building pool (0x55105F; 13000 → \[20000\]).
    pub buildings: Option<u32>,
    /// Object pool (0x551097; 350 → \[3000\]).
    pub objects: Option<u32>,
    /// Dummy pool (0x5510CF; 2500 → \[4000\]).
    pub dummies: Option<u32>,
    /// Collision model pool (0x551106; 10150 → \[32511\]).
    pub col_models: Option<u32>,
    /// Task pool (0x551140; 500 → \[1524\]).
    pub tasks: Option<u32>,
    /// Event pool (0x551178; 200 → \[456\]).
    pub events: Option<u32>,
    /// Texture dictionary pool (`CTxdStore::Initialise` 0x731F60; 5000 → \[20000\]).
    pub txd_slots: Option<u32>,
    /// Vehicle structs (`CVehicleModelInfo::SetupCommonData` 0x5B8FE4; 50 → \[127\]).
    pub vehicle_structs: Option<u32>,
    /// Matrix array (`CPlaceable::InitMatrixArray` 0x54F3A1; 900 → \[6000\]).
    pub matrices: Option<u32>,
    /// Atomic model-info store (`ms_atomicModelInfoStore`, 14 references; → \[20000\]).
    pub atomic_model_infos: Option<u32>,
    /// Ped model-info store (`ms_pedModelInfoStore`, 0x4C67AD; → \[319\]).
    pub ped_model_infos: Option<u32>,
    /// Model index range is −N..=N instead of 0..20000 (`ms_modelInfoPtrs` moved, 707 sites; → \[32767\]).
    pub signed_model_range: Option<u32>,
    /// Vehicle colour palette entries (`ms_vehicleColourTable`, 7 references; 128 → \[256\]). The palette
    /// itself is passed in [`SampModeConfig::vehicle_colours`].
    pub vehicle_colours: Option<u32>,
    /// Player slots in `CWorld::Players`. samp uses player indices 2..=209 and relocates the sector grid
    /// (0xB7D0B8, 115200 → 160000 bytes, ~100 operand sites) so the out-of-range player array does not
    /// corrupt it. Natively this is simply the array length (\[210\]).
    pub player_slots: Option<u32>,
    /// Streaming memory budget in bytes (imm32 at 0x5B8E6A; 50 MB → \[96 MB..1 GB by RAM\]).
    pub streaming_memory: Option<u32>,
    /// `CFont::ProcessCurrentString` line buffer: samp grows the stack frame 0x12C → \[0x32C\] (15 sites).
    pub font_line_buffer: Option<u32>,
    /// Bullet world bounds (±3000 → \[±20000\], 0x7361B2..0x7361F5).
    pub bullet_world_bound: Option<u32>,
}

/// Single-player features samp switches off (one per `patch-disable` label of `docs/samp/README.md` §4.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Feature {
    /// Ambient population peds (`CPopulation::AddPed` 0x612710 returns NULL).
    AmbientPeds,
    /// Seating ambient peds in created cars (0x613BA7).
    AmbientPedsInCars,
    /// Random traffic (`CCarCtrl::GenerateRandomCars` call 0x53C1C1).
    RandomTraffic,
    /// Car generators (`CTheCarGenerators::Process` call 0x53C06A).
    CarGenerators,
    /// Ambient planes (`CPlane::DoPlaneGenerationAndRemoval` call 0x434272).
    AmbientPlanes,
    /// Interior ambient peds (`CEntryExit::GenerateAmbientPeds` call 0x440833).
    InteriorAmbientPeds,
    /// IPL pickups (`CFileLoader::LoadPickup` 0x5B47B0).
    IplPickups,
    /// Interior pickups (`Interior_c::AddPickups` call 0x593D7C).
    InteriorPickups,
    /// Cutscene update (`CCutsceneMgr::Update` call 0x53BF28).
    Cutscenes,
    /// Replay (`CReplay::Update` call 0x53C090).
    Replay,
    /// Plant manager (`CPlantMgr::Update` call 0x53C159).
    PlantManager,
    /// Sun and moon coronas (`CCoronas::DoSunAndMoon` call 0x53C136).
    SunMoonCoronas,
    /// Real-time shadow update (`CRealTimeShadowManager::Update` call 0x53EA08).
    RealTimeShadowUpdate,
    /// Real-time shadow return in `CPhysical` destructor (0x542485).
    RealTimeShadowReturn,
    /// Speed blur (`CPostEffects::SpeedFX` call 0x704E8A).
    SpeedBlur,
    /// Idle camera (`CIdleCam::Process` call 0x524582).
    IdleCamera,
    /// Single-player help boxes (`CHud::SetHelpMessage` 0x588BE0).
    HelpMessages,
    /// HUD vehicle name (0x58FBE9).
    HudVehicleName,
    /// HUD vital stats (0x58FC2E).
    HudVitalStats,
    /// Zone-name display (`CPlaceName::Process` call 0x5720A5).
    ZoneNames,
    /// "Wasted" big message (`CPlayerInfo::KillPlayer` 0x56E5AD).
    WastedMessage,
    /// Menu map rectangle (0x575B0E).
    MenuMapRect,
    /// Loading-screen splash rectangle (0x590099).
    LoadingSplash,
    /// Start-up logos, intro movies and the main menu (0x747483 plus the `gGameState` writes).
    IntroAndMainMenu,
    /// Video-mode aspect-ratio filter (0x745B87, 0x7459E1).
    VideoModeFilter,
    /// Re-init on window focus regain (`CGame::InitAfterLostFocus` call 0x748063).
    ReinitOnFocus,
    /// Weapon lock-on / auto-aim (0x60DC50, 0x60E530).
    WeaponLockOn,
    /// Event facial expressions (0x4BC6C1).
    EventFacialExpressions,
    /// Peds reacting to guns aimed at them (0x4C3A5B).
    ReactToAimedGuns,
    /// Pay'n'Spray sprayable check (0x44AC75).
    PayNSprayCheck,
    /// `CVehicle::SetDriver` freebies (jump table 0x6D1874).
    SetDriverFreebies,
    /// Flying-car physics (`CAutomobile::ProcessFlyingCarStuff` 0x6A8500).
    FlyingCars,
    /// BMX fire ignition of the player (0x53A984).
    BmxFireIgnition,
    /// Sniper "fire failed" sound (0x60F289).
    SniperFailSound,
    /// Rocket "fire failed" sound (0x60F29D).
    RocketFailSound,
    /// Ambient gunfire audio (0x4DF7E2).
    AmbientGunfire,
    /// Cell-phone model (330) (0x6348F6, 0x634DE2).
    CellPhoneModel,
    /// Player idle animations (the "playidles" block name 0x86D1EC).
    PlayerIdleAnims,
    /// Player anim-block release / move-anim switch in `ProcessAnimGroups` (0x609C08).
    PlayerAnimBlockRelease,
    /// Train passenger block in `CTrain::ProcessControl` (0x6F8CF8).
    TrainPassengerBlock,
    /// `Interior_c::Exit` in `InteriorGroup_c::Exit` (0x5952A6).
    InteriorGroupExit,
    /// Unidentified task block in `CPlayerInfo::Process` (0x570535, 0x570546).
    PlayerInfoTaskBlock,
    /// `ClearPlayerWeaponMode` in `ClearWeaponTarget` (0x609CB4).
    ClearWeaponModeOnTargetClear,
    /// Streaming mass RW-object deletion (`CStreaming::DeleteAllRwObjects` 0x4090A0).
    StreamingDeleteAll,
    /// RW-object deletion after death (0x441482).
    StreamingDeleteAfterDeath,
    /// Game image-list registration (`CStreaming::InitImageList` 0x4083C0); samp registers the IMGs.
    GameImageList,
}

impl Feature {
    /// Every feature, in declaration order.
    pub const ALL: [Feature; 46] = [
        Self::AmbientPeds,
        Self::AmbientPedsInCars,
        Self::RandomTraffic,
        Self::CarGenerators,
        Self::AmbientPlanes,
        Self::InteriorAmbientPeds,
        Self::IplPickups,
        Self::InteriorPickups,
        Self::Cutscenes,
        Self::Replay,
        Self::PlantManager,
        Self::SunMoonCoronas,
        Self::RealTimeShadowUpdate,
        Self::RealTimeShadowReturn,
        Self::SpeedBlur,
        Self::IdleCamera,
        Self::HelpMessages,
        Self::HudVehicleName,
        Self::HudVitalStats,
        Self::ZoneNames,
        Self::WastedMessage,
        Self::MenuMapRect,
        Self::LoadingSplash,
        Self::IntroAndMainMenu,
        Self::VideoModeFilter,
        Self::ReinitOnFocus,
        Self::WeaponLockOn,
        Self::EventFacialExpressions,
        Self::ReactToAimedGuns,
        Self::PayNSprayCheck,
        Self::SetDriverFreebies,
        Self::FlyingCars,
        Self::BmxFireIgnition,
        Self::SniperFailSound,
        Self::RocketFailSound,
        Self::AmbientGunfire,
        Self::CellPhoneModel,
        Self::PlayerIdleAnims,
        Self::PlayerAnimBlockRelease,
        Self::TrainPassengerBlock,
        Self::InteriorGroupExit,
        Self::PlayerInfoTaskBlock,
        Self::ClearWeaponModeOnTargetClear,
        Self::StreamingDeleteAll,
        Self::StreamingDeleteAfterDeath,
        Self::GameImageList,
    ];
}

/// A set of [`Feature`]s (bit = discriminant).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FeatureSet(pub u64);

impl FeatureSet {
    /// The empty set (single-player behaviour).
    pub const NONE: FeatureSet = FeatureSet(0);

    /// Whether `f` is in the set.
    pub const fn contains(self, f: Feature) -> bool {
        self.0 & (1u64 << f as u8) != 0
    }

    /// The set with `f` added.
    pub const fn with(self, f: Feature) -> FeatureSet {
        FeatureSet(self.0 | (1u64 << f as u8))
    }
}

/// `patch-modify` rows that change game logic (not limits, not plain disables). `false` = single player.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Behaviour {
    /// Remote players are `CPlayerPed`s without a `CWorld::Players` entry: remove the local-player-only
    /// lookups (`GetPlayerInfoForThisPlayerPed` in 4 sites, `FindPlayerSlotWithPedPointer` in
    /// `CPlayerPed::ProcessControl`, the remote-vehicle branch of `FindPlayerVehicle` 0x56E0FA, the
    /// `Players[0]` driver test in `CBike::ProcessAI` 0x6BC9EB, `DoDriveByShootings` 0x741FFF).
    pub remote_player_peds: bool,
    /// `COMMAND_POINT_CAMERA_AT_CHAR` always uses the ped camera mode (0x47C477), needed to spectate remote
    /// player peds.
    pub point_camera_at_char_ped_mode: bool,
    /// `COMMAND_CREATE_OBJECT` accepts negative model ids (the 0x46977F jge/jne toggle).
    pub create_object_negative_models: bool,
    /// Gun-aim movement uses the constant 2.0 instead of `CTimer::ms_fTimeStep` (0x61E0C2).
    pub gun_aim_constant_step: bool,
    /// `CPed::UpdatePosition` ignores `m_standingOnEntity` (0x5E1E70).
    pub update_position_ignores_standing_on: bool,
    /// `CPed::AttachPedToEntity` returns before giving the weapon / setting camera mode / ped state (0x5E7D62).
    pub attach_ped_no_side_effects: bool,
    /// `CPad::GetHandBrake` ignores `DisablePlayerControls` (0x540040).
    pub handbrake_ignores_disabled_controls: bool,
    /// Car-entry tasks keep the ped's weapon (0x63ADC8, 0x64B872, 0x64BC9F, 0x64DB49).
    pub car_entry_keeps_weapon: bool,
    /// `CPlayerPed` constructor's ped-group condition inverted (0x60D64E).
    pub player_ped_group_inverted: bool,
    /// The game's own `CAEVehicleAudioEntity::Service` calls in Automobile/Bike/Boat/Train `ProcessControl`
    /// are removed (0x6B18F1, 0x6B9298, 0x6F1793, 0x6F86B6); samp calls it from its hook.
    pub vehicle_audio_from_hook: bool,
    /// Stencil-shadow alpha 50 → 80 (0x71162C).
    pub stencil_shadow_alpha_80: bool,
    /// `CPed::Render` change at 0x5E7847.
    pub ped_render_patch: bool,
    /// `CTaskSimpleGangDriveBy::MakeAbortable` change at 0x62D331.
    pub gang_driveby_abortable: bool,
    /// Mouse init change at 0x7469AF (`Input::diMouseInit`, non-exclusive mouse).
    pub mouse_non_exclusive: bool,
    /// Frame limiter and timer start-up changes (`CTimer::Initialise` 0x561872 QPC bypass, 2 ms limiter).
    pub timer_startup: bool,
}

/// Start-up options of samp's early block (applied before the world loads).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartupOptions {
    /// Skip logos/intro/menu and go straight to loading (`gGameState` 5 → 8 path, menu flags
    /// `FrontEndMenuManager + 0xE9/+0x5C/+0x33`).
    pub skip_frontend: bool,
    /// Frame limit written to `RsGlobal.frameLimit` (0xC1704C) at the first frame (\[200\]).
    pub frame_limit: Option<u32>,
    /// Replacement main script (samp.saa's `main.scm`, or samp-rs's fallback), opened instead of
    /// `data\script\main.scm` (`CTheScripts::Init` open call 0x468EC9).
    pub main_scm: Option<Vec<u8>>,
    /// Image archives to register instead of the game's list (samp registers its own, `InitImageList`
    /// is disabled).
    pub image_archives: Vec<String>,
}

/// The whole samp-mode configuration, handed over once in [`crate::SampHost::install`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SampModeConfig {
    /// Capacities.
    pub limits: Limits,
    /// Switched-off single-player features.
    pub disabled: FeatureSet,
    /// Logic changes.
    pub behaviour: Behaviour,
    /// Start-up options.
    pub startup: StartupOptions,
    /// The 256-entry vehicle colour palette (samp ships it in its `.data`; 1024 bytes, RGBA), or empty.
    pub vehicle_colours: Vec<[u8; 4]>,
}

/// Input lock modes (samp's cursor modes; `docs/re/subsystems/ui-rendering.md` §5 in samp-rs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputLock {
    /// Game gets keyboard and mouse.
    None,
    /// Mode 2: keyboard cut (the `AffectPadFromKeyBoard` call 0x541DF5 removed).
    Keyboard,
    /// Mode 3: keyboard and mouse cut (also 0x53F417/0x53F41F, `RsMouseSetPos` 0x6194A0 made `ret`).
    KeyboardAndMouse,
    /// Mode 4: as 3, plus the mouse state pointers redirected to a dummy (0x53F47A/0x53F49A/0x53F4B3).
    KeyboardAndMouseStateRedirected,
}

/// Patches samp applies and restores while playing. Each one is a value the engine reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RuntimeToggle {
    /// Game input lock while samp's UI owns input.
    Input(InputLock),
    /// `CClock::Update` runs (0x52CF10 `push esi` / `ret`) and the HUD clock format at 0x859A6C is shown.
    GameClock(bool),
    /// `CStats::SetStatValue` (0x55A070) has an effect (samp makes it `ret` on the first frame).
    StatChanges(bool),
    /// `CCamera::Fade` (0x50AC20) has an effect (`ret 8` at spawn).
    CameraFade(bool),
    /// `CClothes::RebuildPlayer` (0x5A82C0) runs (made `ret` around a skin change).
    ClothesRebuild(bool),
    /// `cb_RSCOLLISION_RpClump_StreamRead` (0x41B1D0) runs (off during samp model loads).
    ClumpCollisionStreamRead(bool),
    /// `CVehicle::SetupRender` (0x6D64F0) runs.
    VehicleSetupRender(bool),
    /// Manual engine/lights: `DoVehicleLights` follows bit 0x40 of vehicle +0x428 (InitGame).
    ManualVehicleLights(bool),
    /// CJ walk style from fat/muscle stats in `ProcessAnimGroups` (InitGame "CJ walk").
    CjWalkStyle(bool),
    /// HUD wanted level shown (immediate byte at 0x58DB60).
    WantedLevel(u8),
    /// HUD scale fix around radar/crosshair (sa-mp.cfg `nohudscalefix`; floats 0x859520/0x859524,
    /// 0x866B74/0x866B78).
    HudScaleFix(bool),
    /// Single-player cheat strings typed on the keyboard have an effect (`CCheat::AddToCheatString`).
    TypedCheats(bool),
}

/// The part of the adapter that changes samp-mode values while playing.
pub trait SampMode {
    /// The configuration given to [`crate::SampHost::install`].
    fn config(&self) -> &SampModeConfig;

    /// Applies a runtime toggle. The original backend writes/restores the documented bytes; engines set a flag.
    fn set_toggle(&mut self, toggle: RuntimeToggle) -> crate::GameResult<()>;
}
