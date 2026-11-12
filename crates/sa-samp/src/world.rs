//! World state and world entities (pickups, checkpoints, blips) on the native engine: the native backend's `sa_samp_api::world` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::script::ScriptArg::{Float as F, Int as I, Local as L};
use sa_samp_api::world::*;
use sa_samp_api::*;

use crate::addr;
use crate::script::op;
use crate::{NativeGame, View};

impl<V: View> WorldState for NativeGame<V> {
    fn clock(&self) -> (u8, u8) {
        (
            self.view.read_u8(addr::CLOCK_HOURS),
            self.view.read_u8(addr::CLOCK_MINUTES),
        )
    }

    fn set_clock(&mut self, hour: u8, minute: u8) -> GameResult<()> {
        // samp's way: the clock bytes, then SET_TIME_OF_DAY (CClock::SetGameClock).
        self.view.write(addr::CLOCK_HOURS, &[hour]);
        self.view.write(addr::CLOCK_MINUTES, &[minute]);
        self.run_op(op::SET_TIME_OF_DAY, &[I(i32::from(hour)), I(i32::from(minute))])
            .map(drop)
    }

    fn weather(&self) -> (i16, i16, i16) {
        let rd = |a| self.view.read_i16(a);
        (
            rd(addr::WEATHER_FORCED),
            rd(addr::WEATHER_NEW),
            rd(addr::WEATHER_OLD),
        )
    }

    fn set_weather(&mut self, weather: u8) -> GameResult<()> {
        let v = u32::from(weather).to_le_bytes();
        for a in [addr::WEATHER_FORCED, addr::WEATHER_NEW, addr::WEATHER_OLD] {
            self.view.write(a, &v);
        }
        Ok(())
    }

    fn force_weather(&mut self, weather: u8) -> GameResult<()> {
        self.view
            .write(addr::WEATHER_FORCED, &u32::from(weather).to_le_bytes());
        Ok(())
    }

    fn gravity(&self) -> f32 {
        self.view.read_f32(addr::GAME_GRAVITY)
    }

    fn set_gravity(&mut self, g: f32) -> GameResult<()> {
        // `GAME_GRAVITY` is a constant in `.rdata`, which the native arena maps read-only (samp unprotects the
        // page first). The default value needs no write; other values wait for an engine-side gravity setting.
        if self.view.read_f32(addr::GAME_GRAVITY).to_bits() == g.to_bits() {
            return Ok(());
        }
        Err(GameError::Unsupported(
            "WorldState::set_gravity: GAME_GRAVITY is read-only natively",
        ))
    }

    fn area(&self) -> AreaCode {
        AreaCode(self.view.read_i32(addr::CURR_AREA) as u8)
    }

    fn area_raw(&self) -> i32 {
        self.view.read_i32(addr::CURR_AREA)
    }

    fn set_area(&mut self, area: AreaCode) -> GameResult<()> {
        self.run_op(op::SET_AREA_VISIBLE, &[I(i32::from(area.0))])
            .map(drop)
    }

    fn set_stunt_bonus(&mut self, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("WorldState::set_stunt_bonus"))
    }

    fn set_max_wanted_level(&mut self, level: u8) -> GameResult<()> {
        self.run_op(op::SET_MAX_WANTED_LEVEL, &[I(i32::from(level))])
            .map(drop)
    }

    fn set_densities(&mut self, peds: f32, cars: f32) -> GameResult<()> {
        self.run_op(op::SET_PED_DENSITY_MULTIPLIER, &[F(peds)])?;
        self.run_op(op::SET_CAR_DENSITY_MULTIPLIER, &[F(cars)]).map(drop)
    }

    fn switch_roads_off(&mut self) -> GameResult<()> {
        // The whole map, as samp does at start-up.
        self.run_op(
            op::SWITCH_ROADS_OFF,
            &[F(-4000.0), F(-4000.0), F(-500.0), F(4000.0), F(4000.0), F(1500.0)],
        )
        .map(drop)
    }

    fn set_random_trains(&mut self, on: bool) -> GameResult<()> {
        self.run_op(op::SWITCH_RANDOM_TRAINS, &[I(i32::from(on))])
            .map(drop)
    }

    fn set_restart_point(&mut self, pos: Vec3, heading_deg: f32) -> GameResult<()> {
        self.run_op(
            op::ADD_HOSPITAL_RESTART,
            &[F(pos.x), F(pos.y), F(pos.z), F(heading_deg), I(0)],
        )
        .map(drop)
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
    fn create_pickup(&mut self, model: ModelId, kind: i32, pos: Vec3) -> GameResult<PickupRef> {
        let h = self.run_op_out(
            op::CREATE_PICKUP,
            &[I(model.0), I(kind), F(pos.x), F(pos.y), F(pos.z), L(0)],
        )?;
        Ok(PickupRef(h))
    }

    fn remove_pickup(&mut self, p: PickupRef) -> GameResult<()> {
        self.run_op(op::REMOVE_PICKUP, &[I(p.0 as i32)]).map(drop)
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
