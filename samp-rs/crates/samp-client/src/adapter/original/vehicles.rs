//! Vehicles on the original executable: the original backend's `sa_samp_api::vehicles` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not routed through
//! the adapter yet; the client still does that work in its own modules.

use sa_samp_api::vehicles::*;
use sa_samp_api::*;

use sa_samp_api::script::ScriptArg;

use super::OriginalGame;
use crate::patch::Memory;
use crate::script::op;

/// The live vehicle pool through GTA's own pool functions (the DLL; host builds have no pool).
mod pool {
    /// `CPools::ms_pVehiclePool`, `CPool::GetAt` (thiscall), `CPool::GetRef` (thiscall).
    #[cfg(windows)]
    const VEHICLE_POOL: u32 = 0xB74494;
    #[cfg(windows)]
    const POOL_GET_AT: u32 = 0x4048E0;
    #[cfg(windows)]
    const POOL_GET_REF: u32 = 0x424160;

    #[cfg(windows)]
    fn pool() -> u32 {
        // SAFETY: a gta_sa.exe global, main thread.
        unsafe { (VEHICLE_POOL as usize as *const u32).read() }
    }

    #[cfg(windows)]
    pub fn vehicle_at(h: i32) -> Option<u32> {
        let pool = pool();
        if pool == 0 || h < 0 {
            return None;
        }
        // SAFETY: GTA's own pool lookup (thiscall) on the live vehicle pool.
        let v = unsafe {
            let f: extern "thiscall" fn(u32, i32) -> u32 = std::mem::transmute(POOL_GET_AT as usize);
            f(pool, h)
        };
        (v != 0).then_some(v)
    }

    #[cfg(windows)]
    pub fn vehicle_ref(v: u32) -> Option<i32> {
        let pool = pool();
        if pool == 0 {
            return None;
        }
        // SAFETY: GTA's own pool function (thiscall) on the live vehicle pool.
        let h = unsafe {
            let f: extern "thiscall" fn(u32, u32) -> i32 = std::mem::transmute(POOL_GET_REF as usize);
            f(pool, v)
        };
        Some(h)
    }

    #[cfg(not(windows))]
    pub fn vehicle_at(_h: i32) -> Option<u32> {
        None
    }

    #[cfg(not(windows))]
    pub fn vehicle_ref(_v: u32) -> Option<i32> {
        None
    }
}

/// Client-side shorthands for the vehicle calls the client makes with script handles (main thread). A handle
/// that names no vehicle issues nothing (the opcode would fault on it).
#[cfg(windows)]
pub mod calls {
    use sa_samp_api::vehicles::Vehicles as _;
    use sa_samp_api::{AreaCode, ScriptHandle, VehicleRef};

    use crate::adapter::original::process;

    fn veh(h: i32) -> Option<VehicleRef> {
        process().vehicle_from_script_handle(ScriptHandle(h))
    }

    pub fn delete(h: i32) {
        if let Some(v) = veh(h) {
            let _ = process().destroy_vehicle(v);
        }
    }

    pub fn set_heading(h: i32, deg: f32) {
        if let Some(v) = veh(h) {
            let _ = process().set_vehicle_heading(v, deg);
        }
    }

    pub fn set_hydraulics(h: i32, on: i32) {
        if let Some(v) = veh(h) {
            let _ = process().set_hydraulics(v, on != 0);
        }
    }

    pub fn set_tyres_can_burst(h: i32, on: i32) {
        if let Some(v) = veh(h) {
            let _ = process().set_tyres_can_burst(v, on != 0);
        }
    }

    pub fn set_petrol_tank_weakpoint(h: i32, on: i32) {
        if let Some(v) = veh(h) {
            let _ = process().set_petrol_tank_weakpoint(v, on != 0);
        }
    }

    /// `area` comes from a byte.
    pub fn set_area(h: i32, area: u8) {
        if let Some(v) = veh(h) {
            let _ = process().set_vehicle_area(v, AreaCode(area));
        }
    }
}

impl<M: Memory> Vehicles for OriginalGame<M> {
    /// `CPool<CVehicle>::GetRef` (0x424160) on the live vehicle pool.
    fn vehicle_script_handle(&self, v: VehicleRef) -> GameResult<ScriptHandle> {
        pool::vehicle_ref(v.0).map(ScriptHandle).ok_or(GameError::InvalidHandle)
    }

    /// `CPool<CVehicle>::GetAt` (0x4048E0) on the live vehicle pool.
    fn vehicle_from_script_handle(&self, h: ScriptHandle) -> Option<VehicleRef> {
        pool::vehicle_at(h.0).map(VehicleRef)
    }

    fn vehicle_snapshot(&self, _v: VehicleRef) -> GameResult<VehicleSnapshot> {
        Err(GameError::Unsupported("Vehicles::vehicle_snapshot"))
    }

    fn vehicle_damage(&self, _v: VehicleRef) -> GameResult<VehicleDamage> {
        Err(GameError::Unsupported("Vehicles::vehicle_damage"))
    }

    fn vehicle_in_water(&self, _v: VehicleRef) -> GameResult<bool> {
        Err(GameError::Unsupported("Vehicles::vehicle_in_water"))
    }

    fn local_vehicle(&self) -> Option<VehicleRef> {
        None
    }

    fn create_vehicle(&mut self, _spawn: &VehicleSpawn) -> GameResult<VehicleRef> {
        Err(GameError::Unsupported("Vehicles::create_vehicle"))
    }

    /// `DELETE_CAR` (00A6).
    fn destroy_vehicle(&mut self, v: VehicleRef) -> GameResult<()> {
        let h = self.vehicle_script_handle(v)?.0;
        self.run_op(op::DELETE_CAR, &[ScriptArg::Int(h)]).map(|_| ())
    }

    fn set_vehicle_matrix(&mut self, _v: VehicleRef, _m: &Matrix) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_matrix"))
    }

    fn set_vehicle_speed(&mut self, _v: VehicleRef, _move_speed: Vec3, _turn_speed: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_speed"))
    }

    /// `SET_CAR_HEADING` (0175).
    fn set_vehicle_heading(&mut self, v: VehicleRef, heading_deg: f32) -> GameResult<()> {
        let h = self.vehicle_script_handle(v)?.0;
        self.run_op(op::SET_CAR_HEADING, &[ScriptArg::Int(h), ScriptArg::Float(heading_deg)]).map(|_| ())
    }

    fn set_train_position(&mut self, _v: VehicleRef, _pos: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_train_position"))
    }

    fn set_train_speed(&mut self, _v: VehicleRef, _speed: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_train_speed"))
    }

    fn set_vehicle_health(&mut self, _v: VehicleRef, _health: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_health"))
    }

    fn set_vehicle_damage(&mut self, _v: VehicleRef, _d: VehicleDamage) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_damage"))
    }

    fn fix_vehicle(&mut self, _v: VehicleRef) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::fix_vehicle"))
    }

    fn set_vehicle_colours(&mut self, _v: VehicleRef, _colours: [u8; 2]) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_colours"))
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

    /// `SET_CAR_HYDRAULICS`.
    fn set_hydraulics(&mut self, v: VehicleRef, on: bool) -> GameResult<()> {
        let h = self.vehicle_script_handle(v)?.0;
        self.run_op(op::SET_CAR_HYDRAULICS, &[ScriptArg::Int(h), ScriptArg::Int(i32::from(on))]).map(|_| ())
    }

    fn set_vehicle_proofs(&mut self, _v: VehicleRef, _proofs: sa_samp_api::peds::Proofs) -> GameResult<()> {
        Err(GameError::Unsupported("Vehicles::set_vehicle_proofs"))
    }

    /// `SET_CAN_BURST_CAR_TYRES`.
    fn set_tyres_can_burst(&mut self, v: VehicleRef, on: bool) -> GameResult<()> {
        let h = self.vehicle_script_handle(v)?.0;
        self.run_op(op::SET_CAN_BURST_CAR_TYRES, &[ScriptArg::Int(h), ScriptArg::Int(i32::from(on))]).map(|_| ())
    }

    /// `SET_PETROL_TANK_WEAKPOINT`.
    fn set_petrol_tank_weakpoint(&mut self, v: VehicleRef, on: bool) -> GameResult<()> {
        let h = self.vehicle_script_handle(v)?.0;
        self.run_op(op::SET_PETROL_TANK_WEAKPOINT, &[ScriptArg::Int(h), ScriptArg::Int(i32::from(on))]).map(|_| ())
    }

    /// `SET_VEHICLE_AREA_VISIBLE` (0840).
    fn set_vehicle_area(&mut self, v: VehicleRef, area: AreaCode) -> GameResult<()> {
        let h = self.vehicle_script_handle(v)?.0;
        self.run_op(op::SET_VEHICLE_AREA_VISIBLE, &[ScriptArg::Int(h), ScriptArg::Int(i32::from(area.0))]).map(|_| ())
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
