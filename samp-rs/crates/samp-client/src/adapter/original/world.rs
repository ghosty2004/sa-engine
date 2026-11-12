//! World state and world entities (pickups, checkpoints, blips) on the original executable: the original backend's `sa_samp_api::world` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not routed through
//! the adapter yet; the client still does that work in its own modules.

use sa_samp_api::world::*;
use sa_samp_api::*;

use sa_samp_api::script::ScriptArg;

use super::{OriginalGame, write_error};

/// Client-side shorthands for the world-entity calls the client makes with script handles (main thread).
#[cfg(windows)]
pub mod calls {
    use sa_samp_api::world::WorldEntities as _;
    use sa_samp_api::{BlipRef, CheckpointRef, PickupRef};

    use crate::adapter::original::process;

    pub fn remove_pickup(h: i32) {
        let _ = process().remove_pickup(PickupRef(h as u32));
    }

    pub fn delete_checkpoint(h: i32) {
        let _ = process().delete_checkpoint(CheckpointRef(h as u32));
    }

    pub fn remove_blip(h: i32) {
        let _ = process().remove_blip(BlipRef(h as u32));
    }

    pub fn set_blip_colour(h: i32, colour: i32) {
        let _ = process().set_blip_colour(BlipRef(h as u32), colour as u32);
    }

    pub fn set_blip_scale(h: i32, scale: i32) {
        let _ = process().set_blip_scale(BlipRef(h as u32), scale);
    }
}
use crate::patch::Memory;
use crate::script::op;
use crate::{game, world};

impl<M: Memory> WorldState for OriginalGame<M> {
    fn clock(&self) -> (u8, u8) {
        (self.mem.read_u8(world::CLOCK_HOURS).unwrap_or(0), self.mem.read_u8(world::CLOCK_MINUTES).unwrap_or(0))
    }

    /// samp's way: hours and minutes written directly, then `SET_TIME_OF_DAY` (00C0).
    fn set_clock(&mut self, hour: u8, minute: u8) -> GameResult<()> {
        self.mem.write_data(world::CLOCK_HOURS, &[hour]).map_err(write_error)?;
        self.mem.write_data(world::CLOCK_MINUTES, &[minute]).map_err(write_error)?;
        self.run_op(op::SET_TIME_OF_DAY, &[ScriptArg::Int(i32::from(hour)), ScriptArg::Int(i32::from(minute))])
            .map(|_| ())
    }

    fn weather(&self) -> (i16, i16, i16) {
        let rd = |a| self.mem.read_u16(a).unwrap_or(0) as i16;
        (rd(world::WEATHER_FORCED), rd(world::WEATHER_NEW), rd(world::WEATHER_OLD))
    }

    /// Forced, new and old weather (a dword each, like samp). Mirrors samp.dll 0x100A0BA0 with the clock off.
    fn set_weather(&mut self, weather: u8) -> GameResult<()> {
        let v = u32::from(weather).to_le_bytes();
        for a in [world::WEATHER_FORCED, world::WEATHER_NEW, world::WEATHER_OLD] {
            self.mem.write_data(a, &v).map_err(write_error)?;
        }
        Ok(())
    }

    /// The forced weather only (a dword, like samp). Mirrors samp.dll 0x100A0BA0 with the clock on.
    fn force_weather(&mut self, weather: u8) -> GameResult<()> {
        self.mem.write_data(world::WEATHER_FORCED, &u32::from(weather).to_le_bytes()).map_err(write_error)
    }

    fn gravity(&self) -> f32 {
        self.mem.read_f32(game::GAME_GRAVITY).unwrap_or(0.0)
    }

    /// The constant lives in a read-only page; the write makes it writable first, as samp does.
    fn set_gravity(&mut self, g: f32) -> GameResult<()> {
        self.mem.write(game::GAME_GRAVITY, &g.to_le_bytes()).map_err(write_error)
    }

    fn area(&self) -> AreaCode {
        AreaCode::default()
    }

    fn area_raw(&self) -> i32 {
        0
    }

    /// `SET_AREA_VISIBLE` (04BB).
    fn set_area(&mut self, area: AreaCode) -> GameResult<()> {
        self.run_op(op::SET_AREA_VISIBLE, &[ScriptArg::Int(i32::from(area.0))]).map(|_| ())
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

    fn create_fx(&mut self, _name: &str, _on: Option<PedRef>, _offset: Vec3, _dir: Vec3) -> GameResult<FxRef> {
        Err(GameError::Unsupported("WorldState::create_fx"))
    }

    fn kill_fx(&mut self, _fx: FxRef) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::kill_fx"))
    }

    fn load_shop(&mut self, _name: &str) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::load_shop"))
    }
}

impl<M: Memory> WorldEntities for OriginalGame<M> {
    fn create_pickup(&mut self, _model: ModelId, _kind: i32, _pos: Vec3) -> GameResult<PickupRef> {
        Err(GameError::Unsupported("WorldEntities::create_pickup"))
    }

    /// `REMOVE_PICKUP` (0215) with the pickup's script handle.
    fn remove_pickup(&mut self, p: PickupRef) -> GameResult<()> {
        self.run_op(crate::worldents::op::REMOVE_PICKUP, &[ScriptArg::Int(p.0 as i32)]).map(|_| ())
    }

    fn pickup_collected(&self, _p: PickupRef) -> GameResult<bool> {
        Err(GameError::Unsupported("WorldEntities::pickup_collected"))
    }

    fn pickup_index(&self, _p: PickupRef) -> GameResult<u16> {
        Err(GameError::Unsupported("WorldEntities::pickup_index"))
    }

    fn create_checkpoint(&mut self, _kind: CheckpointKind, _pos: Vec3, _size: f32) -> GameResult<CheckpointRef> {
        Err(GameError::Unsupported("WorldEntities::create_checkpoint"))
    }

    /// `DELETE_CHECKPOINT` (06D6) with the checkpoint's script handle.
    fn delete_checkpoint(&mut self, c: CheckpointRef) -> GameResult<()> {
        self.run_op(crate::worldents::op::DELETE_CHECKPOINT, &[ScriptArg::Int(c.0 as i32)]).map(|_| ())
    }

    fn locate_ped(&self, _ped: PedRef, _pos: Vec3, _radius: Vec3) -> GameResult<bool> {
        Err(GameError::Unsupported("WorldEntities::locate_ped"))
    }

    fn add_blip(&mut self, _target: BlipTarget) -> GameResult<BlipRef> {
        Err(GameError::Unsupported("WorldEntities::add_blip"))
    }

    /// `REMOVE_BLIP` (0164) with the blip's script handle.
    fn remove_blip(&mut self, b: BlipRef) -> GameResult<()> {
        self.run_op(op::REMOVE_BLIP, &[ScriptArg::Int(b.0 as i32)]).map(|_| ())
    }

    /// `CHANGE_BLIP_COLOUR` (0165).
    fn set_blip_colour(&mut self, b: BlipRef, colour: u32) -> GameResult<()> {
        self.run_op(op::CHANGE_BLIP_COLOUR, &[ScriptArg::Int(b.0 as i32), ScriptArg::Int(colour as i32)]).map(|_| ())
    }

    /// `CHANGE_BLIP_SCALE` (0168).
    fn set_blip_scale(&mut self, b: BlipRef, scale: i32) -> GameResult<()> {
        self.run_op(op::CHANGE_BLIP_SCALE, &[ScriptArg::Int(b.0 as i32), ScriptArg::Int(scale)]).map(|_| ())
    }

    fn set_blip_display(&mut self, _b: BlipRef, _display: i32) -> GameResult<()> {
        Err(GameError::Unsupported("WorldEntities::set_blip_display"))
    }
}
