//! Addresses (1.0 US) and field offsets of the engine state the backend reads. The values are the ones the
//! ports use (ADR-001 keeps them in phase B/C) and the ones samp-rs's original backend reads; names follow
//! gta-reversed.

use crate::view::Pool;

/// `gGameState`.
pub const GAME_STATE: u32 = 0xC8D4C0;
/// `CTimer::ms_fTimeStep`.
pub const TIME_STEP: u32 = 0xB7CB5C;
/// `CTimer::ms_fTimeStepNonClipped`.
pub const TIME_STEP_NON_CLIPPED: u32 = 0xB7CB58;
/// `CTimer::m_snTimeInMilliseconds`.
pub const TIME_IN_MS: u32 = 0xB7CB84;
/// `RsGlobal.quit` (`RsGlobal` 0xC17040 + 0x10).
pub const RS_QUIT: u32 = 0xC17050;
/// `RsGlobal.maximumWidth` / `maximumHeight`.
pub const RS_MAX_WIDTH: u32 = 0xC17044;
/// See [`RS_MAX_WIDTH`].
pub const RS_MAX_HEIGHT: u32 = 0xC17048;

/// `CWorld::Players` (`CPlayerInfo[2]`, 0x190 bytes each; `m_pPed` at +0, `m_nMoney` at +0xB8).
pub const PLAYERS: u32 = 0xB7CD98;
/// `sizeof(CPlayerInfo)`.
pub const PLAYER_INFO_SIZE: u32 = 0x190;
/// `CWorld::Players[0].m_nMoney`.
pub const PLAYER_MONEY: u32 = PLAYERS + 0xB8;
/// Number of `CWorld::Players` slots in the unmodified game (samp relocates the array for its remote
/// players; that is `Limits::player_slots`, SAMP-4).
pub const PLAYER_SLOTS: u8 = 2;
/// `CWorld::PlayerInFocus` (u8).
pub const PLAYER_IN_FOCUS: u32 = 0xB7CD74;

/// `CPools::ms_pPedPool`, slot `sizeof(CCopPed)`.
pub const PED_POOL: Pool = Pool {
    global: 0xB74490,
    element: 0x7C4,
};
/// `CPools::ms_pVehiclePool`, slot `sizeof(CHeli)`.
pub const VEHICLE_POOL: Pool = Pool {
    global: 0xB74494,
    element: 0xA18,
};
/// `CPools::ms_pObjectPool`, slot `sizeof(CCutsceneObject)`.
pub const OBJECT_POOL: Pool = Pool {
    global: 0xB7449C,
    element: 0x19C,
};

/// `CModelInfo::ms_modelInfoPtrs` (20,000 entries).
pub const MODEL_INFO_PTRS: u32 = 0xA9B0C8;
/// Entries of [`MODEL_INFO_PTRS`]. samp-rs's original backend bounds the index by 20,001 (it reads one
/// dword past the array for model 20,000); the engine's array has 20,000 entries, so model 20,000 is
/// "not a ped model" here.
pub const MODEL_INFO_COUNT: u32 = 20_000;
/// `CPedModelInfo` vtable.
pub const VTBL_PED_MODEL_INFO: u32 = 0x85BDC0;
/// `CPlaceable` vtable (left in an entity after its destructor ran).
pub const VTBL_PLACEABLE: u32 = 0x863C40;

/// `CPad::NewKeyState` (i16 per game key).
pub const NEW_KEY_STATE: u32 = 0xB73190;
/// `CPad::Pads[0]`: `NewState` +0x0, `OldState` +0x30 (`CControllerState`, 0x30 bytes).
pub const PAD0: u32 = 0xB73458;
/// `CPad::NewMouseControllerState`: buttons +0x0..+0x6, `X` +0xC, `Y` +0x10.
pub const NEW_MOUSE: u32 = 0xB73418;

/// `CClock::ms_nGameClockHours` / `ms_nGameClockMinutes` (u8).
pub const CLOCK_HOURS: u32 = 0xB70153;
/// See [`CLOCK_HOURS`].
pub const CLOCK_MINUTES: u32 = 0xB70152;
/// `CWeather::ForcedWeatherType` / `NewWeatherType` / `OldWeatherType` (i16).
pub const WEATHER_FORCED: u32 = 0xC81318;
/// See [`WEATHER_FORCED`].
pub const WEATHER_NEW: u32 = 0xC8131C;
/// See [`WEATHER_FORCED`].
pub const WEATHER_OLD: u32 = 0xC81320;
/// `GAME_GRAVITY` (f32, `.rdata` in the original; the engine keeps it as data).
pub const GAME_GRAVITY: u32 = 0x863984;
/// `CGame::currArea` (i32).
pub const CURR_AREA: u32 = 0xB72914;

/// `FrontEndMenuManager.m_bMenuActive` (`FrontEndMenuManager` 0xBA6748 + 0x5C).
pub const MENU_ACTIVE: u32 = 0xBA67A4;
/// `FrontEndMenuManager.m_bHudOn`.
pub const MENU_HUD_ON: u32 = 0xBA6769;
/// `CHud::bScriptDontDisplayRadar`.
pub const HUD_DONT_DISPLAY_RADAR: u32 = 0xBAA3FB;
/// `CStreaming::ms_aInfoForModel` (`CStreamingInfo`, 0x14 bytes each; `m_nLoadState` at +0x10).
pub const STREAMING_INFO: u32 = 0x8E4CC0;
/// `sizeof(CStreamingInfo)`.
pub const STREAMING_INFO_SIZE: u32 = 0x14;
/// Entries of `ms_aInfoForModel` (models, txds, collisions, ipls, ... up to the scripts).
pub const STREAMING_MODELS: u32 = 26_316;
/// `aWeaponInfo` (`CWeaponInfo`, 0x70 bytes; `m_nModelId1` at +0xC): types 0..47 at standard skill.
pub const WEAPON_INFO: u32 = 0xC8AAB8;
/// `sizeof(CWeaponInfo)`.
pub const WEAPON_INFO_SIZE: u32 = 0x70;
/// Weapon types with a standard-skill entry at their own index.
pub const WEAPON_INFO_STD_COUNT: u8 = 47;
/// `CAnimManager::ms_aAnimBlocks[180]` (`CAnimBlock`: `char szName[16]`, `bool bLoaded` +0x10, ... 0x20 bytes).
pub const ANIM_BLOCKS: u32 = 0xB5D4A0;
/// `sizeof(CAnimBlock)`.
pub const ANIM_BLOCK_SIZE: u32 = 0x20;
/// `CAnimManager::ms_numAnimBlocks`.
pub const ANIM_BLOCK_COUNT: u32 = 0xB4EA34;

/// `TheCamera` fields.
pub mod camera {
    /// `TheCamera.m_mCameraMatrix` (`CMatrix`; position at 0xB6F9CC).
    pub const MATRIX: u32 = 0xB6F99C;
    /// `TheCamera.Cams[0].Front`.
    pub const FRONT: u32 = 0xB6F32C;
    /// `TheCamera.Cams[0].Source`.
    pub const SOURCE: u32 = 0xB6F338;
    /// `TheCamera.Cams[0].Mode` (u16).
    pub const MODE: u32 = 0xB6F1A8;
    /// `TheCamera.Cams[0].FOV`.
    pub const FOV: u32 = 0xB6F250;
    /// `TheCamera.Cams[0].LookingLeft` / `LookingRight` (bool).
    pub const LOOKING_LEFT: u32 = 0xB6F1A4;
    /// See [`LOOKING_LEFT`].
    pub const LOOKING_RIGHT: u32 = 0xB6F1A5;
    /// `TheCamera.m_PlayerWeaponMode.m_nMode` (u16).
    pub const WEAPON_MODE: u32 = 0xB6F858;
    /// `CDraw::ms_fAspectRatio`.
    pub const ASPECT_RATIO: u32 = 0xC3EFA4;
}

/// `CMatrix` rows.
pub mod matrix {
    /// Right vector.
    pub const RIGHT: u32 = 0x00;
    /// Forward vector.
    pub const FORWARD: u32 = 0x10;
    /// Up vector.
    pub const UP: u32 = 0x20;
    /// Position.
    pub const POS: u32 = 0x30;
}

/// Field offsets of `CPlaceable` / `CEntity` / `CPhysical` / `CPed` / `CVehicle`.
pub mod off {
    /// vtable.
    pub const VTABLE: u32 = 0x0;
    /// `CPlaceable::m_placement.m_vPosn` (used when `m_matrix` is null).
    pub const PLACEMENT_POS: u32 = 0x4;
    /// `CPlaceable::m_placement.m_fHeading`.
    pub const PLACEMENT_HEADING: u32 = 0x10;
    /// `CPlaceable::m_matrix` (`CMatrixLink*`).
    pub const MATRIX: u32 = 0x14;
    /// `CEntity::m_pRwObject`.
    pub const RW_OBJECT: u32 = 0x18;
    /// `CEntity::m_nModelIndex` (u16).
    pub const MODEL: u32 = 0x22;
    /// `CEntity::m_AreaCode` (u8).
    pub const AREA: u32 = 0x2F;
    /// `CPhysical::m_vecMoveSpeed`.
    pub const MOVE_SPEED: u32 = 0x44;
    /// `CPhysical::m_vecTurnSpeed`.
    pub const TURN_SPEED: u32 = 0x50;
    /// `CPhysical::m_pMovingList` (non-zero while the entity is in the world's moving list).
    pub const MOVING_LIST: u32 = 0xB4;

    /// `CPed` flag dwords.
    pub const PED_FLAGS: u32 = 0x46C;
    /// `bInVehicle` in the first flag dword.
    pub const PED_IN_VEHICLE: u32 = 1 << 8;
    /// `CPed::m_nPedState`.
    pub const PED_STATE: u32 = 0x530;
    /// `CPed::m_fHealth`.
    pub const PED_HEALTH: u32 = 0x540;
    /// `CPed::m_fArmour`.
    pub const PED_ARMOUR: u32 = 0x548;
    /// `CPed::m_fCurrentRotation`.
    pub const PED_HEADING: u32 = 0x558;
    /// `CPed::m_fAimingRotation`.
    pub const PED_HEADING_GOAL: u32 = 0x55C;
    /// `CPed::m_pVehicle`.
    pub const PED_VEHICLE: u32 = 0x58C;
    /// `CPed::m_aWeapons` (13 `CWeapon`s of 0x1C bytes: type +0x0, state +0x4, ammo in clip +0x8, total +0xC).
    pub const PED_WEAPONS: u32 = 0x5A0;
    /// `sizeof(CWeapon)`.
    pub const WEAPON_SIZE: u32 = 0x1C;
    /// `CPed::m_nActiveWeaponSlot` (u8).
    pub const PED_WEAPON_SLOT: u32 = 0x718;

    /// `CVehicle::m_nVehicleFlags` byte with the siren bit (0x80).
    pub const VEH_SIREN_BYTE: u32 = 0x42D;
    /// `CVehicle::m_nPrimaryColor` / `m_nSecondaryColor`.
    pub const VEH_COLOURS: u32 = 0x434;
    /// `CVehicle::m_pDriver`.
    pub const VEH_DRIVER: u32 = 0x460;
    /// `CVehicle::m_apPassengers[8]`.
    pub const VEH_PASSENGERS: u32 = 0x464;
    /// `CVehicle::m_nMaxPassengers` (u8).
    pub const VEH_MAX_PASSENGERS: u32 = 0x488;
    /// `CVehicle::m_fHealth`.
    pub const VEH_HEALTH: u32 = 0x4C0;
    /// `CTrain::m_fTrainSpeed`.
    pub const TRAIN_SPEED: u32 = 0x5A4;
}

/// Vehicle class vtables.
pub const VEHICLE_VTABLES: [(u32, sa_samp_api::vehicles::VehicleClass); 10] = {
    use sa_samp_api::vehicles::VehicleClass as C;
    [
        (0x871120, C::Automobile),
        (0x871360, C::Bike),
        (0x871528, C::Bmx),
        (0x871680, C::Heli),
        (0x8717D8, C::MonsterTruck),
        (0x871948, C::Plane),
        (0x871AE8, C::QuadBike),
        (0x871C28, C::Trailer),
        (0x8721A0, C::Boat),
        (0x872370, C::Train),
    ]
};
