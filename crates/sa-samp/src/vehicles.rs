//! Vehicles on the native engine: the native backend's `sa_samp_api::vehicles` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::script::ScriptArg::{Float as F, Int as I, Local as L};
use sa_samp_api::vehicles::*;
use sa_samp_api::*;

use crate::addr::{self, off};
use crate::script::op;
use crate::{NativeGame, View};

impl<V: View> NativeGame<V> {
    /// The address of a live vehicle, or `InvalidHandle`.
    pub(crate) fn vehicle_addr(&self, v: VehicleRef) -> GameResult<u32> {
        addr::VEHICLE_POOL
            .at_ref(&self.view, v.0)
            .ok_or(GameError::InvalidHandle)
    }

    /// The handle of the vehicle at `p` (a pointer read from game state), if it is a live pool vehicle.
    pub(crate) fn vehicle_ref(&self, p: u32) -> Option<VehicleRef> {
        if p == 0 {
            return None;
        }
        addr::VEHICLE_POOL.ref_of(&self.view, p).map(VehicleRef)
    }
}

impl<V: View> Vehicles for NativeGame<V> {
    fn vehicle_script_handle(&self, v: VehicleRef) -> GameResult<ScriptHandle> {
        self.vehicle_addr(v).map(|_| ScriptHandle(v.0 as i32))
    }

    fn vehicle_from_script_handle(&self, h: ScriptHandle) -> Option<VehicleRef> {
        addr::VEHICLE_POOL
            .at_ref(&self.view, h.0 as u32)
            .map(|_| VehicleRef(h.0 as u32))
    }

    fn vehicle_snapshot(&self, v: VehicleRef) -> GameResult<VehicleSnapshot> {
        let p = self.vehicle_addr(v)?;
        let m = &self.view;
        let vtable = m.read_u32(p + off::VTABLE);
        let class = addr::VEHICLE_VTABLES
            .iter()
            .find(|(vt, _)| *vt == vtable)
            .map(|(_, c)| *c);
        let mut passengers = [None; 8];
        for (i, s) in passengers.iter_mut().enumerate() {
            *s = self.ped_ref(m.read_u32(p + off::VEH_PASSENGERS + 4 * i as u32));
        }
        Ok(VehicleSnapshot {
            matrix: self.entity_matrix(p),
            move_speed: m.read_vec3(p + off::MOVE_SPEED),
            turn_speed: m.read_vec3(p + off::TURN_SPEED),
            health: m.read_f32(p + off::VEH_HEALTH),
            model: ModelId(i32::from(m.read_i16(p + off::MODEL))),
            class,
            driver: self.ped_ref(m.read_u32(p + off::VEH_DRIVER)),
            passengers,
            max_passengers: m.read_u8(p + off::VEH_MAX_PASSENGERS),
            colours: [
                m.read_u8(p + off::VEH_COLOURS),
                m.read_u8(p + off::VEH_COLOURS + 1),
            ],
            siren: m.read_u8(p + off::VEH_SIREN_BYTE) & 0x80 != 0,
            // Landing gear and the special control value are not read yet (their fields differ per class).
            gear_down: false,
            area: AreaCode(m.read_u8(p + off::AREA)),
            train_speed: if class == Some(VehicleClass::Train) {
                m.read_f32(p + off::TRAIN_SPEED)
            } else {
                0.0
            },
            special: 0.0,
        })
    }

    fn vehicle_damage(&self, _v: VehicleRef) -> GameResult<VehicleDamage> {
        Err(GameError::Unsupported("Vehicles::vehicle_damage"))
    }

    fn vehicle_in_water(&self, _v: VehicleRef) -> GameResult<bool> {
        Err(GameError::Unsupported("Vehicles::vehicle_in_water"))
    }

    fn local_vehicle(&self) -> Option<VehicleRef> {
        // `FindPlayerVehicle(-1, false)` for the local player: its ped's `m_pVehicle` while `bInVehicle`.
        let ped = self.view.read_u32(addr::PLAYERS);
        if ped == 0 || self.ped_ref(ped).is_none() {
            return None;
        }
        if self.view.read_u32(ped + off::PED_FLAGS) & off::PED_IN_VEHICLE == 0 {
            return None;
        }
        self.vehicle_ref(self.view.read_u32(ped + off::PED_VEHICLE))
    }

    fn create_vehicle(&mut self, spawn: &VehicleSpawn) -> GameResult<VehicleRef> {
        let p = spawn.pos;
        let h = self.run_op_out(op::CREATE_CAR, &[I(spawn.model.0), F(p.x), F(p.y), F(p.z), L(0)])? as i32;
        let v = self
            .vehicle_from_script_handle(ScriptHandle(h))
            .ok_or(GameError::NotLoaded)?;
        self.run_op(op::SET_CAR_HEADING, &[I(h), F(spawn.heading_deg)])?;
        self.run_op(
            op::CHANGE_CAR_COLOUR,
            &[
                I(h),
                I(i32::from(spawn.colours[0])),
                I(i32::from(spawn.colours[1])),
            ],
        )?;
        if spawn.area.0 != 0 {
            self.run_op(op::SET_VEHICLE_AREA_VISIBLE, &[I(h), I(i32::from(spawn.area.0))])?;
        }
        Ok(v)
    }

    fn destroy_vehicle(&mut self, v: VehicleRef) -> GameResult<()> {
        let h = self.vehicle_script_handle(v)?.0;
        self.run_op(op::DELETE_CAR, &[I(h)]).map(drop)
    }

    fn set_vehicle_matrix(&mut self, _v: VehicleRef, _m: &Matrix) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_matrix"))
    }

    fn set_vehicle_speed(&mut self, _v: VehicleRef, _move_speed: Vec3, _turn_speed: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_speed"))
    }

    fn set_vehicle_heading(&mut self, v: VehicleRef, heading_deg: f32) -> GameResult<()> {
        let h = self.vehicle_script_handle(v)?.0;
        self.run_op(op::SET_CAR_HEADING, &[I(h), F(heading_deg)])
            .map(drop)
    }

    fn set_train_position(&mut self, _v: VehicleRef, _pos: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_train_position"))
    }

    fn set_train_speed(&mut self, _v: VehicleRef, _speed: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_train_speed"))
    }

    fn set_vehicle_health(&mut self, v: VehicleRef, health: f32) -> GameResult<()> {
        let a = self.vehicle_addr(v)?;
        self.view.write(a + off::VEH_HEALTH, &health.to_le_bytes());
        Ok(())
    }

    fn set_vehicle_damage(&mut self, _v: VehicleRef, _d: VehicleDamage) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_damage"))
    }

    fn fix_vehicle(&mut self, _v: VehicleRef) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::fix_vehicle"))
    }

    fn set_vehicle_colours(&mut self, v: VehicleRef, colours: [u8; 2]) -> GameResult<()> {
        let h = self.vehicle_script_handle(v)?.0;
        self.run_op(
            op::CHANGE_CAR_COLOUR,
            &[I(h), I(i32::from(colours[0])), I(i32::from(colours[1]))],
        )
        .map(drop)
    }

    fn set_paintjob(&mut self, _v: VehicleRef, _paintjob: u8) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_paintjob"))
    }

    fn add_component(&mut self, _v: VehicleRef, _component: ModelId) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::add_component"))
    }

    fn remove_component(&mut self, _v: VehicleRef, _component: ModelId) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::remove_component"))
    }

    fn set_hydraulics(&mut self, _v: VehicleRef, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_hydraulics"))
    }

    fn set_vehicle_proofs(&mut self, _v: VehicleRef, _proofs: sa_samp_api::peds::Proofs) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_proofs"))
    }

    fn set_tyres_can_burst(&mut self, _v: VehicleRef, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_tyres_can_burst"))
    }

    fn set_petrol_tank_weakpoint(&mut self, _v: VehicleRef, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_petrol_tank_weakpoint"))
    }

    fn set_vehicle_area(&mut self, v: VehicleRef, area: AreaCode) -> GameResult<()> {
        let h = self.vehicle_script_handle(v)?.0;
        self.run_op(op::SET_VEHICLE_AREA_VISIBLE, &[I(h), I(i32::from(area.0))])
            .map(drop)
    }

    fn set_door(&mut self, _v: VehicleRef, _door: u8, _open_ratio: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_door"))
    }

    fn set_doors_locked(&mut self, _v: VehicleRef, _locked: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_doors_locked"))
    }

    fn set_window_open(&mut self, _v: VehicleRef, _window: u8, _open: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_window_open"))
    }

    fn set_landing_gear(&mut self, _v: VehicleRef, _down: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_landing_gear"))
    }

    fn set_siren(&mut self, _v: VehicleRef, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_siren"))
    }

    fn set_engine_lights(&mut self, _v: VehicleRef, _engine: bool, _lights: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_engine_lights"))
    }

    fn set_trailer(&mut self, _cab: VehicleRef, _trailer: Option<VehicleRef>) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_trailer"))
    }

    fn set_plate_text(&mut self, _v: VehicleRef, _text: &str) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_plate_text"))
    }
}
