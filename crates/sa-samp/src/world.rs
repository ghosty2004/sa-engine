//! World state and world entities (pickups, checkpoints, blips) on the native engine: the native backend's `sa_samp_api::world` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::world::*;
use sa_samp_api::*;

use crate::addr;
use crate::{NativeGame, View};

impl<V: View> WorldState for NativeGame<V> {
    fn clock(&self) -> (u8, u8) {
        (
            self.view.read_u8(addr::CLOCK_HOURS),
            self.view.read_u8(addr::CLOCK_MINUTES),
        )
    }

    fn set_clock(&mut self, _hour: u8, _minute: u8) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_clock"))
    }

    fn weather(&self) -> (i16, i16, i16) {
        let rd = |a| self.view.read_i16(a);
        (
            rd(addr::WEATHER_FORCED),
            rd(addr::WEATHER_NEW),
            rd(addr::WEATHER_OLD),
        )
    }

    fn set_weather(&mut self, _weather: u8) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_weather"))
    }

    fn force_weather(&mut self, _weather: u8) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::force_weather"))
    }

    fn gravity(&self) -> f32 {
        self.view.read_f32(addr::GAME_GRAVITY)
    }

    fn set_gravity(&mut self, _g: f32) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_gravity"))
    }

    fn area(&self) -> AreaCode {
        AreaCode(self.view.read_i32(addr::CURR_AREA) as u8)
    }

    fn area_raw(&self) -> i32 {
        self.view.read_i32(addr::CURR_AREA)
    }

    fn set_area(&mut self, _area: AreaCode) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_area"))
    }

    fn set_stunt_bonus(&mut self, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_stunt_bonus"))
    }

    fn set_max_wanted_level(&mut self, _level: u8) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_max_wanted_level"))
    }

    fn set_densities(&mut self, _peds: f32, _cars: f32) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_densities"))
    }

    fn switch_roads_off(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::switch_roads_off"))
    }

    fn set_random_trains(&mut self, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_random_trains"))
    }

    fn set_restart_point(&mut self, _pos: Vec3, _heading_deg: f32) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_restart_point"))
    }

    fn ground_z(&self, _pos: Vec3) -> f32 {
        0.0
    }

    fn line_of_sight(&self, _from: Vec3, _to: Vec3, _filter: LineFilter) -> Option<LineHit> {
        None
    }

    fn line_clear(&self, _from: Vec3, _to: Vec3, _filter: LineFilter) -> bool {
        false
    }

    fn add_explosion(&mut self, _pos: Vec3, _kind: i32, _radius: f32, _shake: f32) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::add_explosion"))
    }

    fn create_fx(
        &mut self,
        _name: &str,
        _on: Option<PedRef>,
        _offset: Vec3,
        _dir: Vec3,
    ) -> GameResult<FxRef> {
        Err(GameError::Unsupported("WorldState::create_fx"))
    }

    fn kill_fx(&mut self, _fx: FxRef) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::kill_fx"))
    }

    fn load_shop(&mut self, _name: &str) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::load_shop"))
    }
}

impl<V: View> WorldEntities for NativeGame<V> {
    fn create_pickup(&mut self, _model: ModelId, _kind: i32, _pos: Vec3) -> GameResult<PickupRef> {
        Err(GameError::Unsupported("WorldEntities::create_pickup"))
    }

    fn remove_pickup(&mut self, _p: PickupRef) -> GameResult<()> {
        Err(GameError::Unsupported("WorldEntities::remove_pickup"))
    }

    fn pickup_collected(&self, _p: PickupRef) -> GameResult<bool> {
        Err(GameError::Unsupported("WorldEntities::pickup_collected"))
    }

    fn pickup_index(&self, _p: PickupRef) -> GameResult<u16> {
        Err(GameError::Unsupported("WorldEntities::pickup_index"))
    }

    fn create_checkpoint(
        &mut self,
        _kind: CheckpointKind,
        _pos: Vec3,
        _size: f32,
    ) -> GameResult<CheckpointRef> {
        Err(GameError::Unsupported("WorldEntities::create_checkpoint"))
    }

    fn delete_checkpoint(&mut self, _c: CheckpointRef) -> GameResult<()> {
        Err(GameError::Unsupported("WorldEntities::delete_checkpoint"))
    }

    fn locate_ped(&self, _ped: PedRef, _pos: Vec3, _radius: Vec3) -> GameResult<bool> {
        Err(GameError::Unsupported("WorldEntities::locate_ped"))
    }

    fn add_blip(&mut self, _target: BlipTarget) -> GameResult<BlipRef> {
        Err(GameError::Unsupported("WorldEntities::add_blip"))
    }

    fn remove_blip(&mut self, _b: BlipRef) -> GameResult<()> {
        Err(GameError::Unsupported("WorldEntities::remove_blip"))
    }

    fn set_blip_colour(&mut self, _b: BlipRef, _colour: u32) -> GameResult<()> {
        Err(GameError::Unsupported("WorldEntities::set_blip_colour"))
    }

    fn set_blip_scale(&mut self, _b: BlipRef, _scale: i32) -> GameResult<()> {
        Err(GameError::Unsupported("WorldEntities::set_blip_scale"))
    }

    fn set_blip_display(&mut self, _b: BlipRef, _display: i32) -> GameResult<()> {
        Err(GameError::Unsupported("WorldEntities::set_blip_display"))
    }
}
