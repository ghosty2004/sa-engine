//! Vehicles on the native engine: the native backend's `sa_samp_api::vehicles` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::script::ScriptArg::Int as I;
use sa_samp_api::vehicles::*;
use sa_samp_api::*;

use crate::addr;
use crate::script::op;
use crate::{NativeGame, View};

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
        self.game.vehicle_snapshot(v)
    }

    fn vehicle_damage(&self, _v: VehicleRef) -> GameResult<VehicleDamage> {
        Err(GameError::Unsupported("Vehicles::vehicle_damage"))
    }

    fn vehicle_in_water(&self, _v: VehicleRef) -> GameResult<bool> {
        Err(GameError::Unsupported("Vehicles::vehicle_in_water"))
    }

    fn local_vehicle(&self) -> Option<VehicleRef> {
        self.game.local_vehicle()
    }

    fn vehicles(&self) -> Vec<VehicleRef> {
        self.game.vehicles()
    }

    fn create_vehicle(&mut self, spawn: &VehicleSpawn) -> GameResult<VehicleRef> {
        self.game.create_vehicle(spawn)
    }

    fn destroy_vehicle(&mut self, v: VehicleRef) -> GameResult<()> {
        self.game.destroy_vehicle(v)
    }

    fn set_vehicle_matrix(&mut self, _v: VehicleRef, _m: &Matrix) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_matrix"))
    }

    fn set_vehicle_speed(&mut self, _v: VehicleRef, _move_speed: Vec3, _turn_speed: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_speed"))
    }

    fn set_vehicle_heading(&mut self, v: VehicleRef, heading_deg: f32) -> GameResult<()> {
        self.game.set_vehicle_heading(v, heading_deg)
    }

    fn set_train_position(&mut self, _v: VehicleRef, _pos: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_train_position"))
    }

    fn set_train_speed(&mut self, _v: VehicleRef, _speed: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_train_speed"))
    }

    fn set_vehicle_health(&mut self, v: VehicleRef, health: f32) -> GameResult<()> {
        self.game.set_vehicle_health(v, health)
    }

    fn set_vehicle_damage(&mut self, _v: VehicleRef, _d: VehicleDamage) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_damage"))
    }

    fn fix_vehicle(&mut self, _v: VehicleRef) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::fix_vehicle"))
    }

    fn set_vehicle_colours(&mut self, v: VehicleRef, colours: [u8; 2]) -> GameResult<()> {
        self.game.set_vehicle_colours(v, colours)
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
