//! Vehicles. samp-rs today: `vehicle.rs`, `vehicles/*.rs`, `vehlabels.rs`, `plates.rs`, `vehpatches.rs`.
//! Address mapping per method: `docs/samp/game-adapter.md` §4.2.

use crate::{AreaCode, GameResult, Matrix, ModelId, PedRef, ScriptHandle, Vec3, VehicleRef};

/// Vehicle class, from the vtable (samp-rs `vehicle.rs` compares against 0x871120 … 0x872370).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VehicleClass {
    /// `CAutomobile` (0x871120).
    Automobile,
    /// `CBike` (0x871360).
    Bike,
    /// `CBmx` (0x871528).
    Bmx,
    /// `CHeli` (0x871680).
    Heli,
    /// `CMonsterTruck` (0x8717D8).
    MonsterTruck,
    /// `CPlane` (0x871948).
    Plane,
    /// `CQuadBike` (0x871AE8).
    QuadBike,
    /// `CTrailer` (0x871C28).
    Trailer,
    /// `CBoat` (0x8721A0).
    Boat,
    /// `CTrain` (0x872370).
    Train,
}

/// Damage state samp syncs (RPC 106 / vehicle sync).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VehicleDamage {
    /// Panel states (packed, `CDamageManager`).
    pub panels: u32,
    /// Door states (packed).
    pub doors: u32,
    /// Light states (packed).
    pub lights: u8,
    /// Tyre states (packed).
    pub tyres: u8,
}

/// What vehicle sync reads every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VehicleSnapshot {
    /// Transform.
    pub matrix: Matrix,
    /// Move speed (+0x44).
    pub move_speed: Vec3,
    /// Turn speed (+0x50).
    pub turn_speed: Vec3,
    /// Health (+0x4C0).
    pub health: f32,
    /// Model.
    pub model: ModelId,
    /// Class.
    pub class: Option<VehicleClass>,
    /// Driver (`CVehicle::m_pDriver` +0x460).
    pub driver: Option<PedRef>,
    /// Passenger seats (+0x464…), `max_passengers` (+0x488) valid entries.
    pub passengers: [Option<PedRef>; 8],
    /// Number of passenger seats.
    pub max_passengers: u8,
    /// Primary and secondary colour.
    pub colours: [u8; 2],
    /// Siren on.
    pub siren: bool,
    /// Landing gear down (planes).
    pub gear_down: bool,
    /// Area code.
    pub area: AreaCode,
    /// Train speed (trains only).
    pub train_speed: f32,
    /// Hydra thrust / special control value (+0x86A etc.), raw.
    pub special: f32,
}

/// Parameters of a samp vehicle (RPC 164 WorldVehicleAdd).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VehicleSpawn {
    /// Model.
    pub model: ModelId,
    /// Position.
    pub pos: Vec3,
    /// Heading in degrees.
    pub heading_deg: f32,
    /// Colours.
    pub colours: [u8; 2],
    /// Paintjob (3 = none).
    pub paintjob: u8,
    /// Interior.
    pub area: AreaCode,
}

/// Vehicle operations.
pub trait Vehicles {
    /// Script handle (`CPools::GetVehicleRef`, pool 0xB74494, `POOL_GET_AT` 0x4048E0).
    fn vehicle_script_handle(&self, v: VehicleRef) -> GameResult<ScriptHandle>;
    /// Vehicle from a script handle.
    fn vehicle_from_script_handle(&self, h: ScriptHandle) -> Option<VehicleRef>;
    /// Everything vehicle sync reads.
    fn vehicle_snapshot(&self, v: VehicleRef) -> GameResult<VehicleSnapshot>;
    /// Damage state.
    fn vehicle_damage(&self, v: VehicleRef) -> GameResult<VehicleDamage>;
    /// `IS_CAR_IN_WATER` (02BF).
    fn vehicle_in_water(&self, v: VehicleRef) -> GameResult<bool>;
    /// The local player's vehicle (`FindPlayerVehicle`).
    fn local_vehicle(&self) -> Option<VehicleRef>;

    /// Creates a vehicle (`CREATE_CAR` 00A5, then colours/paintjob/area; trains `CREATE_MISSION_TRAIN` 06D8).
    fn create_vehicle(&mut self, spawn: &VehicleSpawn) -> GameResult<VehicleRef>;
    /// Deletes a vehicle (`DELETE_CAR` 00A6 / `DELETE_MISSION_TRAIN` 07BD).
    fn destroy_vehicle(&mut self, v: VehicleRef) -> GameResult<()>;
    /// Writes the matrix.
    fn set_vehicle_matrix(&mut self, v: VehicleRef, m: &Matrix) -> GameResult<()>;
    /// Writes move and turn speed.
    fn set_vehicle_speed(&mut self, v: VehicleRef, move_speed: Vec3, turn_speed: Vec3) -> GameResult<()>;
    /// `SET_CAR_HEADING` (0175).
    fn set_vehicle_heading(&mut self, v: VehicleRef, heading_deg: f32) -> GameResult<()>;
    /// `SET_MISSION_TRAIN_COORDINATES` (07C7).
    fn set_train_position(&mut self, v: VehicleRef, pos: Vec3) -> GameResult<()>;
    /// Writes the train speed.
    fn set_train_speed(&mut self, v: VehicleRef, speed: f32) -> GameResult<()>;
    /// Sets health.
    fn set_vehicle_health(&mut self, v: VehicleRef, health: f32) -> GameResult<()>;
    /// Writes damage state (`CAutomobile::SetupDamageAfterLoad` 0x6B3E90 afterwards).
    fn set_vehicle_damage(&mut self, v: VehicleRef, d: VehicleDamage) -> GameResult<()>;
    /// Repairs (`CAutomobile::Fix` 0x6A3440).
    fn fix_vehicle(&mut self, v: VehicleRef) -> GameResult<()>;
    /// Sets colours.
    fn set_vehicle_colours(&mut self, v: VehicleRef, colours: [u8; 2]) -> GameResult<()>;
    /// `GIVE_VEHICLE_PAINTJOB` (06ED).
    fn set_paintjob(&mut self, v: VehicleRef, paintjob: u8) -> GameResult<()>;
    /// `ADD_VEHICLE_MOD` (06E7); the model must be loaded (`REQUEST_VEHICLE_MOD` 06E9 / 06EA).
    fn add_component(&mut self, v: VehicleRef, component: ModelId) -> GameResult<()>;
    /// `REMOVE_VEHICLE_MOD` (06E8).
    fn remove_component(&mut self, v: VehicleRef, component: ModelId) -> GameResult<()>;
    /// `SET_CAR_HYDRAULICS` (07FF).
    fn set_hydraulics(&mut self, v: VehicleRef, on: bool) -> GameResult<()>;
    /// `SET_CAR_PROOFS` (02AC).
    fn set_vehicle_proofs(&mut self, v: VehicleRef, proofs: crate::peds::Proofs) -> GameResult<()>;
    /// `SET_CAN_BURST_CAR_TYRES` (053F).
    fn set_tyres_can_burst(&mut self, v: VehicleRef, on: bool) -> GameResult<()>;
    /// `SET_PETROL_TANK_WEAKPOINT` (09C4).
    fn set_petrol_tank_weakpoint(&mut self, v: VehicleRef, on: bool) -> GameResult<()>;
    /// `SET_VEHICLE_AREA_VISIBLE` (0840).
    fn set_vehicle_area(&mut self, v: VehicleRef, area: AreaCode) -> GameResult<()>;
    /// Doors: open (`CAutomobile::OpenDoor` 0x6A6AE0), lock state.
    fn set_door(&mut self, v: VehicleRef, door: u8, open_ratio: f32) -> GameResult<()>;
    /// Locks/unlocks doors (`CVehicle::m_nDoorLock`).
    fn set_doors_locked(&mut self, v: VehicleRef, locked: bool) -> GameResult<()>;
    /// Windows (`SetWindowOpenFlag` 0x6D3080 / `ClearWindowOpenFlag` 0x6D30B0).
    fn set_window_open(&mut self, v: VehicleRef, window: u8, open: bool) -> GameResult<()>;
    /// Landing gear (`CPlane::SetGearUp` 0x6CAC20 / `SetGearDown` 0x6CAC70).
    fn set_landing_gear(&mut self, v: VehicleRef, down: bool) -> GameResult<()>;
    /// Siren state.
    fn set_siren(&mut self, v: VehicleRef, on: bool) -> GameResult<()>;
    /// Engine and lights in manual mode (bit 0x40 of +0x428 with
    /// [`crate::config::RuntimeToggle::ManualVehicleLights`]).
    fn set_engine_lights(&mut self, v: VehicleRef, engine: bool, lights: bool) -> GameResult<()>;
    /// Attaches a trailer (`ATTACH_TRAILER_TO_CAB` 0893) or detaches it (`DETACH_TRAILER_FROM_CAB` 07AC).
    fn set_trailer(&mut self, cab: VehicleRef, trailer: Option<VehicleRef>) -> GameResult<()>;
    /// Number plate text (RPC 123). The engine renders the plate; the original backend paints a D3D texture
    /// that replaces the shared plate raster (`CCustomCarPlateMgr::CreatePlateTexture` hook 0x6FDED6,
    /// `RwRasterCreate`/`RwRasterDestroy` 0x7FB230/0x7FB020).
    fn set_plate_text(&mut self, v: VehicleRef, text: &str) -> GameResult<()>;
}
