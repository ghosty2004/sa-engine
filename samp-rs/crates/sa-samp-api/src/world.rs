//! World state (time, weather, gravity, area, collision queries) and world entities (pickups, checkpoints,
//! blips, explosions, effects). samp-rs today: `world.rs`, `entities.rs`, `worldents.rs`,
//! `behaviourclient*.rs`, `client.rs`, `scm.rs`, `combatclient.rs`. Mapping: `docs/samp/game-adapter.md` §4.4.

use crate::{
    AreaCode, BlipRef, CheckpointRef, EntityRef, FxRef, GameError, GameResult, ModelId, PedRef, PickupRef, Vec3,
    VehicleRef,
};

/// Result of a line-of-sight query (`CWorld::ProcessLineOfSight` 0x56BA00).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHit {
    /// Hit point.
    pub point: Vec3,
    /// Surface normal.
    pub normal: Vec3,
    /// Entity hit, if any.
    pub entity: Option<EntityRef>,
}

/// Which entity kinds a line test considers (the bool arguments of `ProcessLineOfSight`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct LineFilter {
    /// Buildings.
    pub buildings: bool,
    /// Vehicles.
    pub vehicles: bool,
    /// Peds.
    pub peds: bool,
    /// Objects.
    pub objects: bool,
    /// Dummies.
    pub dummies: bool,
    /// See-through surfaces count as hits.
    pub see_through: bool,
    /// Camera-ignore entities are skipped (`CWorld::pIgnoreEntity` 0xB7CD68).
    pub ignore_camera_entity: bool,
}

/// Blip target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlipTarget {
    /// `ADD_BLIP_FOR_CHAR` (0187).
    Ped(PedRef),
    /// `ADD_BLIP_FOR_CAR_OLD` (0161).
    Vehicle(VehicleRef),
    /// `ADD_SPRITE_BLIP_FOR_COORD` (02A8) / short range `04CE`, contact point `02A7` / `0570`.
    Coord {
        /// Position.
        pos: Vec3,
        /// Radar sprite id (0 = plain marker).
        sprite: u8,
        /// Short range.
        short_range: bool,
        /// Contact-point style.
        contact_point: bool,
    },
}

/// A checkpoint kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CheckpointKind {
    /// samp's SetPlayerCheckpoint (a cylinder; `CREATE_CHECKPOINT` 06D5 type 1 / samp's own marker).
    Normal,
    /// Race checkpoint with type and next point.
    Race {
        /// samp race checkpoint type (0..=8).
        kind: u8,
        /// Next checkpoint (arrow direction).
        next: Vec3,
    },
}

/// A world rectangle drawn on the radar and the map (`CRadar::DrawAreaOnRadar` 0x5853D0; samp's gang zones).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadarArea {
    /// Lower x / y corner.
    pub min: (f32, f32),
    /// Upper x / y corner.
    pub max: (f32, f32),
    /// Colour, `0xRRGGBBAA`.
    pub colour: u32,
    /// The colour it alternates with (`0xRRGGBBAA`) and the period in milliseconds.
    pub flash: Option<(u32, u32)>,
}

/// Weather and time.
pub trait WorldState {
    /// Game clock hours and minutes (`CClock::ms_nGameClockHours/Minutes` 0xB70153/0xB70152).
    fn clock(&self) -> (u8, u8);
    /// Sets the clock (`SET_TIME_OF_DAY` 00C0 or the direct writes samp uses every frame).
    fn set_clock(&mut self, hour: u8, minute: u8) -> GameResult<()>;
    /// Weather ids: forced, new, old (`CWeather::ForcedWeatherType/NewWeatherType/OldWeatherType`
    /// 0xC81318/0xC8131C/0xC81320).
    fn weather(&self) -> (i16, i16, i16);
    /// Forces a weather now (writes all three like samp while the game clock is off).
    fn set_weather(&mut self, weather: u8) -> GameResult<()>;
    /// Forces a weather through `ForcedWeatherType` only (samp while the game clock runs: the weather
    /// blends over as the game's weather update moves on).
    fn force_weather(&mut self, weather: u8) -> GameResult<()>;
    /// Gravity (`GAME_GRAVITY` 0x863984).
    fn gravity(&self) -> f32;
    /// Sets gravity.
    fn set_gravity(&mut self, g: f32) -> GameResult<()>;
    /// Current area / interior (`CGame::currArea` 0xB72914, `GET_AREA_VISIBLE` 077E).
    fn area(&self) -> AreaCode;
    /// `CGame::currArea` as the full `i32` it is in the game (samp-rs reads it as `i32`).
    fn area_raw(&self) -> i32;
    /// `SET_AREA_VISIBLE` (04BB).
    fn set_area(&mut self, area: AreaCode) -> GameResult<()>;
    /// Stunt bonus enabled (SCM global 709, ScriptSpace + 0xB14 = 0xA4A474).
    fn set_stunt_bonus(&mut self, on: bool) -> GameResult<()>;
    /// `SET_MAX_WANTED_LEVEL` (01F0).
    fn set_max_wanted_level(&mut self, level: u8) -> GameResult<()>;
    /// Ped and car density multipliers (`SET_PED_DENSITY_MULTIPLIER` 03DE, `SET_CAR_DENSITY_MULTIPLIER` 01EB).
    fn set_densities(&mut self, peds: f32, cars: f32) -> GameResult<()>;
    /// `SWITCH_ROADS_OFF` (01E8) for the whole map.
    fn switch_roads_off(&mut self) -> GameResult<()>;
    /// `SWITCH_RANDOM_TRAINS` (06D7).
    fn set_random_trains(&mut self, on: bool) -> GameResult<()>;
    /// `ADD_HOSPITAL_RESTART` (016C) / the restart override samp writes at spawn.
    fn set_restart_point(&mut self, pos: Vec3, heading_deg: f32) -> GameResult<()>;
    /// `OVERRIDE_NEXT_RESTART` (016E): the next restart after death or arrest at this point.
    fn override_restart(&mut self, _pos: Vec3, _heading_deg: f32) -> GameResult<()> {
        Err(crate::GameError::Unsupported("WorldState::override_restart"))
    }
    /// `GET_GROUND_Z_FOR_3D_COORD` (02CE).
    fn ground_z(&self, pos: Vec3) -> f32;
    /// Line of sight with hit details (`CWorld::ProcessLineOfSight` 0x56BA00).
    fn line_of_sight(&self, from: Vec3, to: Vec3, filter: LineFilter) -> Option<LineHit>;
    /// Clear line test (`IS_LINE_OF_SIGHT_CLEAR` 06BD).
    fn line_clear(&self, from: Vec3, to: Vec3, filter: LineFilter) -> bool;
    /// `ADD_EXPLOSION_VARIABLE_SHAKE` (0948).
    fn add_explosion(&mut self, pos: Vec3, kind: i32, radius: f32, shake: f32) -> GameResult<()>;
    /// Particle effects (`CREATE_FX_SYSTEM_ON_CHAR_WITH_DIRECTION` 066A, `PLAY_FX_SYSTEM` 064C).
    fn create_fx(&mut self, name: &str, on: Option<PedRef>, offset: Vec3, dir: Vec3) -> GameResult<FxRef>;
    /// `KILL_FX_SYSTEM` (0650).
    fn kill_fx(&mut self, fx: FxRef) -> GameResult<()>;
    /// Loads a shop's data (`CShopping::LoadShop` 0x49BBE0; mod shop and clothes).
    fn load_shop(&mut self, name: &str) -> GameResult<()>;
    /// Takes every entry-exit's access away: the flags word (+0x30) of each `CEntryExitManager::mp_poolEntryExits`
    /// (0x96A7D8) entry becomes 0, so no marker shows and no ped enters or leaves through them, until the game
    /// rebuilds them.
    fn disable_entry_exits(&mut self) -> GameResult<()>;
    /// Sets a float statistic (`CStats::StatTypesFloat` 0xB79380; 69..=79 are the weapon skills).
    fn set_float_stat(&mut self, _stat: u16, _value: f32) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_float_stat"))
    }
}

/// Pickups, checkpoints, blips and radar areas.
pub trait WorldEntities {
    /// `CREATE_PICKUP` (0213).
    fn create_pickup(&mut self, model: ModelId, kind: i32, pos: Vec3) -> GameResult<PickupRef>;
    /// `REMOVE_PICKUP` (0215).
    fn remove_pickup(&mut self, p: PickupRef) -> GameResult<()>;
    /// `HAS_PICKUP_BEEN_COLLECTED` (0214). samp also intercepts collection in `CPickup::Update`
    /// (hook 0x4579C6) — see [`crate::hooks::GameHooks::pickup_collected`].
    fn pickup_collected(&self, p: PickupRef) -> GameResult<bool>;
    /// Index of a pickup in the game's array (samp sends it; `CPickups::aPickUps` 0x9788C0).
    fn pickup_index(&self, p: PickupRef) -> GameResult<u16>;
    /// Whether the local player stands where `CPickup::Update` picks the pickup up (the state samp's collision
    /// hook at 0x4579C6 sees), for hosts that cannot hook that site.
    fn pickup_touched(&self, p: PickupRef) -> GameResult<bool>;

    /// Creates a checkpoint.
    fn create_checkpoint(&mut self, kind: CheckpointKind, pos: Vec3, size: f32) -> GameResult<CheckpointRef>;
    /// `DELETE_CHECKPOINT` (06D6).
    fn delete_checkpoint(&mut self, c: CheckpointRef) -> GameResult<()>;
    /// `LOCATE_CHAR_ANY_MEANS_3D` (00FE): is the ped within `radius` (box) of `pos`; `marker` also places the
    /// game's area marker there for this frame.
    fn locate_ped(&mut self, ped: PedRef, pos: Vec3, radius: Vec3, marker: bool) -> GameResult<bool>;

    /// Adds a blip.
    fn add_blip(&mut self, target: BlipTarget) -> GameResult<BlipRef>;
    /// `REMOVE_BLIP` (0164).
    fn remove_blip(&mut self, b: BlipRef) -> GameResult<()>;
    /// `CHANGE_BLIP_COLOUR` (0165).
    fn set_blip_colour(&mut self, b: BlipRef, colour: u32) -> GameResult<()>;
    /// `CHANGE_BLIP_SCALE` (0168).
    fn set_blip_scale(&mut self, b: BlipRef, scale: i32) -> GameResult<()>;
    /// `CHANGE_BLIP_DISPLAY` (018B).
    fn set_blip_display(&mut self, b: BlipRef, display: i32) -> GameResult<()>;

    /// Shows radar area `id` (drawn in id order where the game draws its gang areas), replacing one of that id.
    fn show_radar_area(&mut self, id: u16, area: &RadarArea) -> GameResult<()>;
    /// Removes radar area `id`.
    fn hide_radar_area(&mut self, id: u16) -> GameResult<()>;
}
