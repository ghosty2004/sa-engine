//! Vehicles.

use crate::{AreaCode, Matrix, ModelId, PedRef, Vec3};

/// Vehicle class, from the vtable (0x871120 … 0x872370).
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

/// A vehicle's state at one moment.
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

/// What a new vehicle is made with.
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

