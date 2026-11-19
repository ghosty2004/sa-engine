//! The game state a mod sees (permission `state = "read"`): fixed records of little-endian 32-bit words that the host
//! writes into the mod's memory (`player_info`, `ped_info`, ...) and the SDK reads back. The field types are the
//! game's value types (`sa-types`); each record is filled from the engine's record of the same thing
//! (`sa-access`).

use sa_types::camera::CameraState;
use sa_types::ped::{DamageSource, PedDamage, PedSnapshot};
use sa_types::vehicle::VehicleSnapshot;
use sa_types::input::PadState;
use sa_types::{AreaCode, GameState, Matrix, ModelId, ObjectSnapshot, PedRef, TimeStep, Vec3, VehicleRef, WeaponType};

/// A value that travels as a fixed number of 32-bit words.
pub trait Words: Sized {
    /// Its number of words.
    const WORDS: usize;
    /// Writes its words to the front of `out` (at least [`Words::WORDS`] long).
    fn put(&self, out: &mut [u32]);
    /// Reads it from the front of `w` (at least [`Words::WORDS`] long).
    fn get(w: &[u32]) -> Self;
}

/// The word of an absent handle (a pool reference is `slot << 8 | reuse`, never all ones).
pub const NO_HANDLE: u32 = u32::MAX;

macro_rules! scalar_words {
    ($($t:ty: |$v:ident| $to:expr, |$w:ident| $from:expr;)*) => {$(
        impl Words for $t {
            const WORDS: usize = 1;
            fn put(&self, out: &mut [u32]) {
                let $v = *self;
                out[0] = $to;
            }
            fn get(w: &[u32]) -> Self {
                let $w = w[0];
                $from
            }
        }
    )*};
}

scalar_words! {
    u32: |v| v, |w| w;
    i32: |v| v as u32, |w| w as i32;
    f32: |v| v.to_bits(), |w| f32::from_bits(w);
    u8: |v| u32::from(v), |w| w as u8;
    u16: |v| u32::from(v), |w| w as u16;
    i16: |v| v as i32 as u32, |w| w as i32 as i16;
    bool: |v| u32::from(v), |w| w != 0;
    PedRef: |v| v.0, |w| PedRef(w);
    VehicleRef: |v| v.0, |w| VehicleRef(w);
    Option<PedRef>: |v| v.map_or(NO_HANDLE, |p| p.0), |w| (w != NO_HANDLE).then_some(PedRef(w));
    Option<VehicleRef>: |v| v.map_or(NO_HANDLE, |p| p.0), |w| (w != NO_HANDLE).then_some(VehicleRef(w));
    ModelId: |v| v.0 as u32, |w| ModelId(w as i32);
    WeaponType: |v| u32::from(v.0), |w| WeaponType(w as u8);
    AreaCode: |v| u32::from(v.0), |w| AreaCode(w as u8);
    GameState: |v| v.raw() as u32, |w| GameState::from_raw(w as i32);
}

impl Words for Vec3 {
    const WORDS: usize = 3;
    fn put(&self, out: &mut [u32]) {
        self.x.put(out);
        self.y.put(&mut out[1..]);
        self.z.put(&mut out[2..]);
    }
    fn get(w: &[u32]) -> Self {
        Vec3::new(f32::get(w), f32::get(&w[1..]), f32::get(&w[2..]))
    }
}

/// Declares a record: a struct whose fields travel in declaration order.
macro_rules! record {
    ($(#[$m:meta])* pub struct $name:ident { $($(#[$fm:meta])* pub $f:ident: $t:ty,)* }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct $name {
            $($(#[$fm])* pub $f: $t,)*
        }

        impl Words for $name {
            const WORDS: usize = 0 $(+ <$t as Words>::WORDS)*;
            fn put(&self, out: &mut [u32]) {
                let mut _at = 0;
                $(
                    self.$f.put(&mut out[_at..]);
                    _at += <$t as Words>::WORDS;
                )*
            }
            fn get(w: &[u32]) -> Self {
                let mut _at = 0;
                $(
                    let $f = <$t>::get(&w[_at..]);
                    _at += <$t as Words>::WORDS;
                )*
                $name { $($f),* }
            }
        }
    };
}

record! {
    /// A ped (`ped_info`; the local player's ped is [`PlayerInfo::ped`]).
    pub struct PedInfo {
        /// Model.
        pub model: ModelId,
        /// Position.
        pub pos: Vec3,
        /// Move speed (units per 1/50 s).
        pub move_speed: Vec3,
        /// Current heading in radians.
        pub heading: f32,
        /// Health.
        pub health: f32,
        /// Armour.
        pub armour: f32,
        /// `ePedState` (54 dying, 55 dead).
        pub ped_state: u32,
        /// Current weapon.
        pub weapon: WeaponType,
        /// The vehicle it is in.
        pub vehicle: Option<VehicleRef>,
        /// Area / interior.
        pub area: AreaCode,
        /// In the world (not a deleted entity).
        pub in_world: bool,
    }
}

record! {
    /// The local player (`player_info`).
    pub struct PlayerInfo {
        /// Its ped; the rest is zero without one.
        pub handle: Option<PedRef>,
        /// Money.
        pub money: i32,
        /// Wanted level (0..=6).
        pub wanted: u8,
        /// The ped.
        pub ped: PedInfo,
    }
}

record! {
    /// A vehicle (`vehicle_info`).
    pub struct VehicleInfo {
        /// Model.
        pub model: ModelId,
        /// Position.
        pub pos: Vec3,
        /// Forward vector (heading: `atan2(-forward.x, forward.y)`).
        pub forward: Vec3,
        /// Move speed (units per 1/50 s).
        pub move_speed: Vec3,
        /// Health (1000 new, below 250 burning).
        pub health: f32,
        /// Driver.
        pub driver: Option<PedRef>,
        /// Primary colour.
        pub colour1: u8,
        /// Secondary colour.
        pub colour2: u8,
        /// Siren on.
        pub siren: bool,
        /// Area / interior.
        pub area: AreaCode,
    }
}

record! {
    /// Time, weather and the world (`world_info`).
    pub struct WorldInfo {
        /// The game state machine.
        pub game_state: GameState,
        /// Game time in milliseconds.
        pub time_ms: u32,
        /// The frame's time step (1.0 = 1/50 s).
        pub time_step: f32,
        /// Game clock hours.
        pub hour: u8,
        /// Game clock minutes.
        pub minute: u8,
        /// Weather the clock blends from.
        pub weather_old: i32,
        /// Weather the clock blends to.
        pub weather_new: i32,
        /// Forced weather (-1: none).
        pub weather_forced: i32,
        /// Current area / interior (`CGame::currArea`).
        pub area: i32,
        /// Gravity.
        pub gravity: f32,
    }
}

record! {
    /// The camera (`camera_info`).
    pub struct CameraInfo {
        /// Position.
        pub pos: Vec3,
        /// Front (looking direction).
        pub front: Vec3,
        /// Up.
        pub up: Vec3,
        /// Field of view in degrees.
        pub fov: f32,
        /// Active camera mode.
        pub mode: u16,
        /// Aspect ratio.
        pub aspect_ratio: f32,
    }
}

impl From<&PedSnapshot> for PedInfo {
    fn from(s: &PedSnapshot) -> PedInfo {
        PedInfo {
            model: s.model,
            pos: s.matrix.pos,
            move_speed: s.move_speed,
            heading: s.heading,
            health: s.health,
            armour: s.armour,
            ped_state: s.ped_state,
            weapon: s.weapon,
            vehicle: s.vehicle,
            area: s.area,
            in_world: s.in_world,
        }
    }
}

impl From<&VehicleSnapshot> for VehicleInfo {
    fn from(s: &VehicleSnapshot) -> VehicleInfo {
        VehicleInfo {
            model: s.model,
            pos: s.matrix.pos,
            forward: s.matrix.forward,
            move_speed: s.move_speed,
            health: s.health,
            driver: s.driver,
            colour1: s.colours[0],
            colour2: s.colours[1],
            siren: s.siren,
            area: s.area,
        }
    }
}

record! {
    /// A transform (`ped_matrix`, `vehicle_matrix`, 1.5): the entity's axes and position.
    pub struct MatrixInfo {
        /// Right axis.
        pub right: Vec3,
        /// Forward axis.
        pub forward: Vec3,
        /// Up axis.
        pub up: Vec3,
        /// Position.
        pub pos: Vec3,
    }
}

record! {
    /// The local player's pad (`pad_state`, 1.5): the game's controller state this frame, raw values (sticks
    /// -128..=128, buttons 0 or 255).
    pub struct PadInfo {
        /// Left stick x.
        pub left_x: i16,
        /// Left stick y.
        pub left_y: i16,
        /// Right stick x.
        pub right_x: i16,
        /// Right stick y.
        pub right_y: i16,
        /// L1.
        pub left_shoulder1: i16,
        /// L2.
        pub left_shoulder2: i16,
        /// R1.
        pub right_shoulder1: i16,
        /// R2.
        pub right_shoulder2: i16,
        /// D-pad up.
        pub dpad_up: i16,
        /// D-pad down.
        pub dpad_down: i16,
        /// D-pad left.
        pub dpad_left: i16,
        /// D-pad right.
        pub dpad_right: i16,
        /// Start.
        pub start: i16,
        /// Select.
        pub select: i16,
        /// Square.
        pub square: i16,
        /// Triangle.
        pub triangle: i16,
        /// Cross.
        pub cross: i16,
        /// Circle.
        pub circle: i16,
        /// Left stick pressed.
        pub shock_left: i16,
        /// Right stick pressed.
        pub shock_right: i16,
        /// The chat indicator.
        pub chat_indicated: i16,
        /// The walk key.
        pub ped_walk: i16,
    }
}

record! {
    /// An object (`object_info`, API 1.8).
    pub struct ObjectInfo {
        /// Model.
        pub model: ModelId,
        /// Position.
        pub pos: Vec3,
        /// Area / interior.
        pub area: AreaCode,
        /// Streamed in: its RenderWare object exists.
        pub has_rw: bool,
        /// Drawn.
        pub visible: bool,
        /// Collides.
        pub collision: bool,
        /// Attached to another entity (the game places it).
        pub attached: bool,
        /// Scale.
        pub scale: f32,
        /// Move speed (units per 1/50 s).
        pub move_speed: Vec3,
        /// Turn speed.
        pub turn_speed: Vec3,
        /// Centre of its collision model's bounding sphere in the world.
        pub bound_centre: Vec3,
        /// Radius of that sphere (0 without a collision model).
        pub bound_radius: f32,
    }
}

impl From<&ObjectSnapshot> for ObjectInfo {
    fn from(s: &ObjectSnapshot) -> ObjectInfo {
        ObjectInfo {
            model: s.model,
            pos: s.matrix.pos,
            area: s.area,
            has_rw: s.has_rw,
            visible: s.visible,
            collision: s.collision,
            attached: s.attached,
            scale: s.scale,
            move_speed: s.move_speed,
            turn_speed: s.turn_speed,
            bound_centre: s.bound_centre,
            bound_radius: s.bound_radius,
        }
    }
}

impl From<&Matrix> for MatrixInfo {
    fn from(m: &Matrix) -> MatrixInfo {
        MatrixInfo { right: m.right, forward: m.forward, up: m.up, pos: m.pos }
    }
}

impl From<MatrixInfo> for Matrix {
    fn from(m: MatrixInfo) -> Matrix {
        Matrix { right: m.right, forward: m.forward, up: m.up, pos: m.pos }
    }
}

impl From<&PadState> for PadInfo {
    fn from(p: &PadState) -> PadInfo {
        PadInfo {
            left_x: p.left_x,
            left_y: p.left_y,
            right_x: p.right_x,
            right_y: p.right_y,
            left_shoulder1: p.shoulders[0],
            left_shoulder2: p.shoulders[1],
            right_shoulder1: p.shoulders[2],
            right_shoulder2: p.shoulders[3],
            dpad_up: p.dpad[0],
            dpad_down: p.dpad[1],
            dpad_left: p.dpad[2],
            dpad_right: p.dpad[3],
            start: p.start_select[0],
            select: p.start_select[1],
            square: p.face[0],
            triangle: p.face[1],
            cross: p.face[2],
            circle: p.face[3],
            shock_left: p.shock[0],
            shock_right: p.shock[1],
            chat_indicated: p.chat_indicated,
            ped_walk: p.ped_walk,
        }
    }
}

impl From<PadInfo> for PadState {
    fn from(p: PadInfo) -> PadState {
        PadState {
            left_x: p.left_x,
            left_y: p.left_y,
            right_x: p.right_x,
            right_y: p.right_y,
            shoulders: [p.left_shoulder1, p.left_shoulder2, p.right_shoulder1, p.right_shoulder2],
            dpad: [p.dpad_up, p.dpad_down, p.dpad_left, p.dpad_right],
            start_select: [p.start, p.select],
            face: [p.square, p.triangle, p.cross, p.circle],
            shock: [p.shock_left, p.shock_right],
            chat_indicated: p.chat_indicated,
            ped_walk: p.ped_walk,
        }
    }
}

impl From<&CameraState> for CameraInfo {
    fn from(s: &CameraState) -> CameraInfo {
        CameraInfo {
            pos: s.matrix.pos,
            front: s.front,
            up: s.matrix.up,
            fov: s.fov,
            mode: s.mode,
            aspect_ratio: s.aspect_ratio,
        }
    }
}

impl WorldInfo {
    /// The record from the game's values: the game state, timer, clock `(h, m)`, weather `(forced, new, old)`,
    /// area and gravity.
    pub fn new(
        game_state: GameState,
        time_ms: u32,
        step: TimeStep,
        clock: (u8, u8),
        weather: (i16, i16, i16),
        area: i32,
        gravity: f32,
    ) -> WorldInfo {
        WorldInfo {
            game_state,
            time_ms,
            time_step: step.clipped,
            hour: clock.0,
            minute: clock.1,
            weather_old: weather.2.into(),
            weather_new: weather.1.into(),
            weather_forced: weather.0.into(),
            area,
            gravity,
        }
    }
}

record! {
    /// How a vehicle moves and what it tows (`vehicle_motion`).
    pub struct VehicleMotion {
        /// Turn speed (radians per 1/50 s about each axis).
        pub turn_speed: Vec3,
        /// In the world's moving list: its physics runs.
        pub moving: bool,
        /// The cab towing it.
        pub tractor: Option<VehicleRef>,
        /// The trailer it tows.
        pub trailer: Option<VehicleRef>,
        /// Its passenger seats (`CVehicle::m_nMaxPassengers`).
        pub max_passengers: u8,
    }
}

record! {
    /// What last damaged a ped (`ped_damage`): at most one of `ped`, `vehicle` and `other`.
    pub struct DamageInfo {
        /// The weapon type the game recorded (`eWeaponType`, or 49 rammed, 50 helicopter blades, 51 explosion, 53
        /// drowned, 54 collision or fall; 255 none).
        pub weapon: WeaponType,
        /// The ped that dealt it.
        pub ped: Option<PedRef>,
        /// The vehicle that dealt it.
        pub vehicle: Option<VehicleRef>,
        /// Something else dealt it (an object, a building).
        pub other: bool,
    }
}

impl From<&PedDamage> for DamageInfo {
    fn from(d: &PedDamage) -> DamageInfo {
        let (ped, vehicle, other) = match d.source {
            DamageSource::None => (None, None, false),
            DamageSource::Ped(p) => (Some(p), None, false),
            DamageSource::Vehicle(v) => (None, Some(v), false),
            DamageSource::Other => (None, None, true),
        };
        DamageInfo { weapon: d.weapon, ped, vehicle, other }
    }
}

impl From<DamageInfo> for PedDamage {
    fn from(d: DamageInfo) -> PedDamage {
        let source = match (d.ped, d.vehicle, d.other) {
            (Some(p), _, _) => DamageSource::Ped(p),
            (None, Some(v), _) => DamageSource::Vehicle(v),
            (None, None, true) => DamageSource::Other,
            (None, None, false) => DamageSource::None,
        };
        PedDamage { weapon: d.weapon, source }
    }
}

/// The record's bytes as the host writes them into the mod's memory.
pub fn to_bytes<T: Words>(v: &T) -> Vec<u8> {
    let mut w = vec![0u32; T::WORDS];
    v.put(&mut w);
    w.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// The record from the bytes the host wrote (`None`: too short).
pub fn from_bytes<T: Words>(b: &[u8]) -> Option<T> {
    if b.len() < 4 * T::WORDS {
        return None;
    }
    let w: Vec<u32> = b.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
    Some(T::get(&w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip() {
        let p = PlayerInfo {
            handle: Some(PedRef(0x205)),
            money: -7,
            wanted: 3,
            ped: PedInfo {
                model: ModelId(0),
                pos: Vec3::new(1.0, -2.5, 13.0),
                health: 100.0,
                vehicle: None,
                area: AreaCode(3),
                in_world: true,
                ..Default::default()
            },
        };
        let b = to_bytes(&p);
        assert_eq!(b.len(), 4 * PlayerInfo::WORDS);
        assert_eq!(PlayerInfo::WORDS, 3 + PedInfo::WORDS);
        assert_eq!(PedInfo::WORDS, 15);
        assert_eq!(from_bytes::<PlayerInfo>(&b), Some(p));
        assert_eq!(from_bytes::<PlayerInfo>(&b[4..]), None);
        let w = WorldInfo { game_state: GameState::Playing, weather_forced: -1, ..Default::default() };
        assert_eq!(from_bytes::<WorldInfo>(&to_bytes(&w)), Some(w));
    }
}
