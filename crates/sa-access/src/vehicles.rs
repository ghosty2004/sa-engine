//! Vehicle damage and class, physical speeds, gravity and statistics, and the argument checks of the script
//! commands whose handlers trust a vehicle's class or an index.

use sa_core::mem::GPtr;
use sa_types::script::ScriptArg;
use sa_types::vehicle::VehicleClass;
use sa_types::{EntityRef, GameError, GameResult, Matrix, ModelId, PedRef, Vec3, VehicleRef};

use crate::addr::{self, matrix, off};
use crate::{Game, View};

/// `CAutomobile`'s damage manager: the four wheel-status bytes (in address order), the door states (the four
/// bytes from the bonnet on), the light states and the panel states.
const DMG_WHEELS: [u32; 4] = [0x5A5, 0x5A6, 0x5A7, 0x5A8];
const DMG_DOORS: u32 = 0x5A9;
const DMG_LIGHTS: u32 = 0x5B0;
const DMG_PANELS: u32 = 0x5B4;
/// `CBike::m_nWheelStatus[2]`: front, rear.
const BIKE_TYRE_FRONT: u32 = 0x65C;
const BIKE_TYRE_REAR: u32 = 0x65D;
/// Wheel / tyre status: burst.
const TYRE_BURST: u8 = 1;
/// `CPed` flag bits `bIsStanding`, `bWasStanding`.
const PED_STANDING: u32 = 3;
/// `CStats::StatTypesFloat` and the number of float statistics.
const FLOAT_STATS: u32 = 0xB79380;
/// The float statistics (ids `0 ..`).
pub const FLOAT_STAT_COUNT: i32 = 82;
/// Vehicle upgrade (component) models.
const UPGRADE_MODELS: std::ops::RangeInclusive<i32> = 1000..=1193;
/// Explosion types (`eExplosionType`).
const EXPLOSION_TYPES: std::ops::RangeInclusive<i32> = 0..=12;
/// Radio channels `SET_RADIO_CHANNEL` takes (the stations and the user track player, then off).
const RADIO_CHANNELS: std::ops::RangeInclusive<i32> = 0..=13;
/// Fighting styles (`eFightingStyle`, the melee info table's rows).
const FIGHT_STYLES: std::ops::RangeInclusive<i32> = 4..=16;
/// Weapon slots `GET_CHAR_WEAPON_IN_SLOT` numbers (1-based).
const WEAPON_SLOTS: std::ops::RangeInclusive<i32> = 1..=13;
/// Paintjobs (`CVehicleModelInfo::m_anRemapTxds[4]`; -1 removes it).
const PAINTJOBS: std::ops::RangeInclusive<i32> = -1..=3;
/// A ped's weapon accuracy (`CPed::m_nWeaponAccuracy`, a percentage the shot spread is computed from).
const ACCURACY: std::ops::RangeInclusive<i32> = 0..=100;
/// The lengths a written matrix row may have (a rotation's rows are unit vectors).
const MATRIX_ROW_LENGTH: std::ops::RangeInclusive<f32> = 0.9..=1.1;
/// The positions a written matrix may have: the world sectors' range and then some.
const WORLD_XY: std::ops::RangeInclusive<f32> = -20000.0..=20000.0;
const WORLD_Z: std::ops::RangeInclusive<f32> = -10000.0..=100000.0;

/// The classes built on `CAutomobile` (car nodes, damage manager, upgrades).
fn automobile_based(c: VehicleClass) -> bool {
    matches!(
        c,
        VehicleClass::Automobile
            | VehicleClass::MonsterTruck
            | VehicleClass::QuadBike
            | VehicleClass::Heli
            | VehicleClass::Plane
            | VehicleClass::Trailer
    )
}

impl<V: View> Game<V> {
    /// The vehicle's class (from its vtable); `InvalidHandle` for a stale handle, `Other` for an unknown vtable.
    pub fn vehicle_class(&self, v: VehicleRef) -> GameResult<VehicleClass> {
        let p = self.vehicle_addr(v)?;
        let vtable = self.view.read_u32(p + off::VTABLE);
        addr::VEHICLE_VTABLES
            .iter()
            .find(|(vt, _)| *vt == vtable)
            .map(|(_, c)| *c)
            .ok_or_else(|| GameError::Other(format!("vehicle vtable {vtable:#x}")))
    }

    /// Panel, door and light states of an automobile or plane, and the burst-tyre bits (automobiles: bit 3 the
    /// first wheel status .. bit 0 the fourth; bikes: bit 1 front, bit 0 rear).
    pub fn vehicle_damage(&self, v: VehicleRef) -> GameResult<[u32; 4]> {
        let p = self.vehicle_addr(v)?;
        let class = self.vehicle_class(v)?;
        let m = &self.view;
        let auto = matches!(class, VehicleClass::Automobile | VehicleClass::Plane);
        let tyres = match class {
            VehicleClass::Automobile => {
                DMG_WHEELS.iter().enumerate().fold(0, |t, (i, &o)| t | (u32::from(m.read_u8(p + o) != 0) << (3 - i)))
            }
            VehicleClass::Bike => {
                u32::from(m.read_u8(p + BIKE_TYRE_FRONT) != 0) << 1 | u32::from(m.read_u8(p + BIKE_TYRE_REAR) != 0)
            }
            _ => 0,
        };
        Ok([
            if auto { m.read_u32(p + DMG_PANELS) } else { 0 },
            if auto { m.read_u32(p + DMG_DOORS) } else { 0 },
            if class == VehicleClass::Automobile { u32::from(m.read_u8(p + DMG_LIGHTS)) } else { 0 },
            tyres,
        ])
    }

    /// Writes the damage manager's panel, door and light states of an automobile or plane and rebuilds its parts
    /// from them (`CAutomobile::SetupDamageAfterLoad`); other classes are left as they are.
    pub fn set_vehicle_parts_damage(&mut self, v: VehicleRef, panels: u32, doors: u32, lights: u8) -> GameResult<()> {
        let p = self.vehicle_addr(v)?;
        if !matches!(self.vehicle_class(v)?, VehicleClass::Automobile | VehicleClass::Plane) {
            return Ok(());
        }
        self.view.write_u32(p + DMG_PANELS, panels);
        self.view.write_u32(p + DMG_DOORS, doors);
        self.view.write_u32(p + DMG_LIGHTS, u32::from(lights));
        sa_core::bindings::automobile_setup_damage_after_load(GPtr::from_addr(p as usize));
        Ok(())
    }

    /// Bursts (bit set) or mends the tyres of an automobile (bit 3 the first wheel .. bit 0 the fourth) or a bike
    /// (bit 1 front, bit 0 rear); other classes are left as they are.
    pub fn set_vehicle_tyres(&mut self, v: VehicleRef, tyres: u8) -> GameResult<()> {
        let p = self.vehicle_addr(v)?;
        let state = |bit: u32| if tyres & (1 << bit) != 0 { TYRE_BURST } else { 0 };
        match self.vehicle_class(v)? {
            VehicleClass::Automobile => {
                for (i, o) in DMG_WHEELS.iter().enumerate() {
                    self.view.write(p + o, &[state(3 - i as u32)]);
                }
            }
            VehicleClass::Bike => {
                self.view.write(p + BIKE_TYRE_REAR, &[state(0)]);
                self.view.write(p + BIKE_TYRE_FRONT, &[state(1)]);
            }
            _ => {}
        }
        Ok(())
    }

    /// Writes an entity's move and turn speed (`CPhysical` +0x44 / +0x50, units per 1/50 s).
    pub fn set_physical_speed(&mut self, e: EntityRef, move_speed: Vec3, turn_speed: Option<Vec3>) -> GameResult<()> {
        let p = self.entity_addr(e)?;
        let speeds = [(Some(move_speed), off::MOVE_SPEED), (turn_speed, off::TURN_SPEED)];
        for (v, at) in speeds.into_iter().filter_map(|(v, at)| Some((v?, at))) {
            for (i, c) in [v.x, v.y, v.z].into_iter().enumerate() {
                self.view.write(p + at + 4 * i as u32, &c.to_le_bytes());
            }
        }
        Ok(())
    }

    /// Writes the ped's move speed; a ped standing on the ground has its two standing flags toggled, so that it
    /// leaves the ground.
    pub fn set_ped_speed(&mut self, ped: PedRef, speed: Vec3) -> GameResult<()> {
        let p = self.ped_addr(ped)?;
        let flags = self.view.read_u32(p + off::PED_FLAGS);
        if flags & PED_STANDING != 0 {
            self.view.write_u32(p + off::PED_FLAGS, flags ^ PED_STANDING);
        }
        self.set_physical_speed(EntityRef::Ped(ped), speed, None)
    }

    /// Sets `GAME_GRAVITY`.
    pub fn set_gravity(&mut self, g: f32) -> GameResult<()> {
        self.view.write_image(addr::GAME_GRAVITY, &g.to_le_bytes());
        Ok(())
    }

    /// Sets float statistic `stat` (`0 .. FLOAT_STAT_COUNT`).
    pub fn set_float_stat(&mut self, stat: u16, value: f32) -> GameResult<()> {
        if i32::from(stat) >= FLOAT_STAT_COUNT {
            return Err(GameError::Other(format!("no float statistic {stat}")));
        }
        self.view.write(FLOAT_STATS + 4 * u32::from(stat), &value.to_le_bytes());
        Ok(())
    }

    /// Writes the vehicle's matrix (the rows and the position as they are), as a physics step leaves it: no
    /// teleport, so the speeds, the suspension and the collision state stay; the game files the vehicle under the
    /// world sectors of its new position as its physics runs. Refused for a matrix that is not finite, rows that
    /// are not near unit length, or a position outside the world's range.
    pub fn set_vehicle_matrix(&mut self, v: VehicleRef, m: &Matrix) -> GameResult<()> {
        let p = self.vehicle_addr(v)?;
        let rows = [m.right, m.forward, m.up];
        let len = |r: Vec3| (r.x * r.x + r.y * r.y + r.z * r.z).sqrt();
        if !rows.iter().chain([&m.pos]).all(|r| r.x.is_finite() && r.y.is_finite() && r.z.is_finite()) {
            return Err(GameError::Other("matrix is not finite".into()));
        }
        if !rows.iter().all(|&r| (MATRIX_ROW_LENGTH).contains(&len(r))) {
            return Err(GameError::Other("matrix rows are not unit vectors".into()));
        }
        if !(WORLD_XY.contains(&m.pos.x) && WORLD_XY.contains(&m.pos.y) && WORLD_Z.contains(&m.pos.z)) {
            return Err(GameError::Other(format!("position {:?} outside the world", m.pos)));
        }
        let mat = self.view.read_u32(p + off::MATRIX);
        if mat == 0 {
            return Err(GameError::Other("the vehicle has no matrix".into()));
        }
        let at = [(m.right, matrix::RIGHT), (m.forward, matrix::FORWARD), (m.up, matrix::UP), (m.pos, matrix::POS)];
        for (row, at) in at {
            for (i, c) in [row.x, row.y, row.z].into_iter().enumerate() {
                self.view.write(mat + at + 4 * i as u32, &c.to_le_bytes());
            }
        }
        Ok(())
    }

    /// Why `op` must not run with `args`: the game's handler would read past a table or use a vehicle as a class
    /// it is not (none of them checks).
    pub fn refused_arguments(&self, op: u16, args: &[ScriptArg<'_>]) -> Option<String> {
        let int = |i: usize| match args.get(i) {
            Some(ScriptArg::Int(v)) => Some(*v),
            _ => None,
        };
        let class = |i: usize| int(i).and_then(|h| self.vehicle_class(VehicleRef(h as u32)).ok());
        let range = |i: usize, r: std::ops::RangeInclusive<i32>, what: &str| {
            int(i).filter(|v| !r.contains(v)).map(|v| format!("{what} {v} out of range"))
        };
        let needs = |i: usize, ok: fn(VehicleClass) -> bool, what: &str| {
            class(i).filter(|c| !ok(*c)).map(|c| format!("vehicle class {c:?} is no {what}"))
        };
        match op {
            // ADD_VEHICLE_MOD, REMOVE_VEHICLE_MOD: an automobile and a loaded upgrade model
            0x06E7 | 0x06E8 => needs(0, automobile_based, "automobile").or_else(|| range(1, UPGRADE_MODELS, "upgrade")).or_else(|| {
                let m = int(1)?;
                (op == 0x06E7 && !self.model_loaded(ModelId(m))).then(|| format!("upgrade {m} not loaded"))
            }),
            // REQUEST_VEHICLE_MOD, HAS_VEHICLE_MOD_LOADED
            0x06E9 | 0x06EA => range(0, UPGRADE_MODELS, "upgrade"),
            // GIVE_VEHICLE_PAINTJOB
            0x06ED => needs(0, automobile_based, "automobile").or_else(|| range(1, PAINTJOBS, "paintjob")),
            // OPEN_CAR_DOOR_A_BIT reads the car nodes
            0x08A6 => needs(0, automobile_based, "automobile").or_else(|| range(1, 0..=5, "door")),
            // SET_PLANE_UNDERCARRIAGE_UP
            0x08E6 => needs(0, |c| c == VehicleClass::Plane, "plane"),
            // SET_MISSION_TRAIN_COORDINATES, SET_TRAIN_SPEED, SET_TRAIN_CRUISE_SPEED
            0x07C7 | 0x06DC | 0x06DD => needs(0, |c| c == VehicleClass::Train, "train"),
            // ADD_EXPLOSION, ADD_EXPLOSION_VARIABLE_SHAKE
            0x020C | 0x0948 => range(3, EXPLOSION_TYPES, "explosion type"),
            0x041E => range(0, RADIO_CHANNELS, "radio channel"),
            0x07FE => range(1, FIGHT_STYLES, "fighting style"),
            0x04B8 => range(1, WEAPON_SLOTS, "weapon slot"),
            // SET_CHAR_ACCURACY: the spread is computed from 100 - accuracy
            0x02E2 => range(1, ACCURACY, "accuracy"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{self, put};
    use sa_types::ObjectRef;

    /// A vehicle of `vtable` in slot 1 of the fake vehicle pool (reuse counter 1): handle 0x101.
    fn vehicle(g: &mut Game<fake::FakeView>, vtable: u32) -> (VehicleRef, u32) {
        let p = fake::VEHS + addr::VEHICLE_POOL.element;
        put(g, fake::VEH_MAP + 1, 0x01u8);
        put(g, p + off::VTABLE, vtable);
        (VehicleRef(0x101), p)
    }

    #[test]
    fn damage_round_trip_of_an_automobile_and_a_bike() {
        let mut g = fake::game();
        let (v, p) = vehicle(&mut g, 0x871120);
        put(&mut g, p + DMG_PANELS, 0x0003_0000u32);
        put(&mut g, p + DMG_DOORS, 0x0404_0404u32);
        put(&mut g, p + DMG_LIGHTS, 5u8);
        g.set_vehicle_tyres(v, 0b1001).unwrap();
        assert_eq!(g.view.read_u8(p + DMG_WHEELS[0]), TYRE_BURST);
        assert_eq!(g.view.read_u8(p + DMG_WHEELS[1]), 0);
        assert_eq!(g.vehicle_damage(v).unwrap(), [0x0003_0000, 0x0404_0404, 5, 0b1001]);
        put(&mut g, p + off::VTABLE, 0x871360u32);
        g.set_vehicle_tyres(v, 0b10).unwrap();
        assert_eq!(g.vehicle_damage(v).unwrap(), [0, 0, 0, 0b10]);
        assert_eq!(g.vehicle_class(VehicleRef(0x102)), Err(GameError::InvalidHandle));
    }

    #[test]
    fn ped_speed_toggles_the_standing_flags() {
        let mut g = fake::game();
        put(&mut g, fake::LOCAL + off::PED_FLAGS, 0x0101u32);
        g.set_ped_speed(PedRef(0x205), Vec3::new(0.0, 0.0, 0.5)).unwrap();
        assert_eq!(g.view.read_u32(fake::LOCAL + off::PED_FLAGS), 0x0102);
        assert_eq!(g.view.read_f32(fake::LOCAL + off::MOVE_SPEED + 8), 0.5);
        let none = g.set_physical_speed(EntityRef::Object(ObjectRef(0x101)), Vec3::default(), None);
        assert_eq!(none, Err(GameError::InvalidHandle));
    }

    #[test]
    fn gravity_and_float_stats() {
        let mut g = fake::game();
        g.set_gravity(0.004).unwrap();
        assert_eq!(g.gravity(), 0.004);
        g.set_float_stat(69, 999.0).unwrap();
        assert_eq!(g.view.read_f32(FLOAT_STATS + 4 * 69), 999.0);
        assert!(g.set_float_stat(82, 1.0).is_err());
    }

    #[test]
    fn refused_arguments_by_class_and_range() {
        let mut g = fake::game();
        let (v, p) = vehicle(&mut g, 0x871360);
        let (h, i, f) = (ScriptArg::Int(v.0 as i32), ScriptArg::Int, ScriptArg::Float);
        assert!(g.refused_arguments(0x06ED, &[h, i(1)]).is_some());
        assert!(g.refused_arguments(0x07C7, &[h, f(0.0), f(0.0), f(0.0)]).is_some());
        put(&mut g, p + off::VTABLE, 0x871120u32);
        assert_eq!(g.refused_arguments(0x06ED, &[h, i(1)]), None);
        assert!(g.refused_arguments(0x06ED, &[h, i(4)]).is_some());
        assert!(g.refused_arguments(0x06E8, &[h, i(999)]).is_some());
        assert_eq!(g.refused_arguments(0x06E8, &[h, i(1080)]), None);
        assert!(g.refused_arguments(0x06E7, &[h, i(1080), ScriptArg::Local(0)]).is_some(), "not loaded");
        assert!(g.refused_arguments(0x08E6, &[h, i(1)]).is_some());
        assert!(g.refused_arguments(0x0948, &[f(0.0), f(0.0), f(0.0), i(13), f(1.0)]).is_some());
        assert_eq!(g.refused_arguments(0x0948, &[f(0.0), f(0.0), f(0.0), i(12), f(1.0)]), None);
        assert!(g.refused_arguments(0x04B8, &[i(0), i(0)]).is_some());
        assert_eq!(g.refused_arguments(0x0373, &[]), None);
    }
}
