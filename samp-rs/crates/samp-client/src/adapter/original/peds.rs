//! Peds and players on the original executable: the original backend's `sa_samp_api::peds` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not routed through
//! the adapter yet; the client still does that work in its own modules.

use sa_samp_api::peds::*;
use sa_samp_api::*;

use sa_samp_api::script::ScriptArg;

use super::{OriginalGame, read_error};
use crate::game;
use crate::patch::Memory;
use crate::script::op;

/// Client-side shorthands for the ped calls the client makes with script handles (main thread). A handle that
/// names no live ped issues nothing (the opcode would fault on it).
#[cfg(windows)]
pub mod calls {
    use sa_samp_api::peds::Peds as _;
    use sa_samp_api::{AreaCode, PedRef, ScriptHandle, WeaponType};

    use crate::adapter::original::process;

    fn ped(h: i32) -> Option<PedRef> {
        process().ped_from_script_handle(ScriptHandle(h))
    }

    pub fn set_current_weapon(h: i32, weapon: i32) {
        if let Some(p) = ped(h) {
            let _ = process().set_current_weapon(p, WeaponType(weapon as u8));
        }
    }

    pub fn set_suffers_critical_hits(h: i32, on: i32) {
        if let Some(p) = ped(h) {
            let _ = process().set_suffers_critical_hits(p, on != 0);
        }
    }

    pub fn set_drops_weapons_when_dead(h: i32, on: i32) {
        if let Some(p) = ped(h) {
            let _ = process().set_drops_weapons_when_dead(p, on != 0);
        }
    }

    pub fn freeze(h: i32, frozen: bool) {
        if let Some(p) = ped(h) {
            let _ = process().freeze_ped(p, frozen);
        }
    }

    pub fn set_area(h: i32, area: i32) {
        if let Some(p) = ped(h) {
            let _ = process().set_ped_area(p, AreaCode(area as u8));
        }
    }

    pub fn set_heading(h: i32, deg: f32) {
        if let Some(p) = ped(h) {
            let _ = process().set_ped_heading(p, deg, false);
        }
    }

    pub fn set_player_control(enabled: bool) {
        let _ = process().set_player_control(enabled);
    }

    pub fn set_drunkenness(level: i32) {
        let _ = process().set_drunkenness(level);
    }
}

/// `CPedIntelligence::FlushImmediately(bool)`.
#[cfg(windows)]
const FLUSH_IMMEDIATELY: u32 = 0x601640;

impl<M: Memory> Peds for OriginalGame<M> {
    fn local_player(&self) -> Option<PedRef> {
        match self.mem.read_u32(game::CWORLD_PLAYERS + game::off::PLAYER_INFO_PED) {
            Ok(ped) if ped != 0 => Some(PedRef(ped)),
            _ => None,
        }
    }

    fn player_ped(&self, _slot: PlayerSlot) -> Option<PedRef> {
        None
    }

    fn player_in_focus(&self) -> PlayerSlot {
        PlayerSlot(self.mem.read_u8(game::PLAYER_IN_FOCUS).unwrap_or(0))
    }

    fn ped_script_handle(&self, ped: PedRef) -> GameResult<ScriptHandle> {
        match game::ped_handle(&self.mem, ped.0) {
            Ok(Some(h)) => Ok(ScriptHandle(h)),
            Ok(None) => Err(GameError::InvalidHandle),
            Err(e) => Err(read_error(e)),
        }
    }

    /// `CPools::GetPed` (`CPool::GetAtRef`): slot `h >> 8` whose map byte equals the low byte.
    fn ped_from_script_handle(&self, h: ScriptHandle) -> Option<PedRef> {
        game::ped_at_ref(&self.mem, h.0).ok().flatten().map(PedRef)
    }

    fn ped_in_world(&self, _ped: PedRef) -> bool {
        false
    }

    fn is_ped_model(&self, model: ModelId) -> bool {
        game::is_ped_model(&self.mem, model.0 as u32)
    }

    fn create_player_ped(&mut self, _slot: PlayerSlot, _model: ModelId, _pos: Vec3) -> GameResult<PedRef> {
        Err(GameError::Unsupported("Peds::create_player_ped"))
    }

    /// samp's actor: `CREATE_CHAR` (009A) with ped type 5, then `SET_CHAR_HEADING` (0173). Mirrors samp.dll
    /// 0x1009C310.
    fn create_actor(&mut self, model: ModelId, pos: Vec3, heading_deg: f32) -> GameResult<PedRef> {
        let (i, f) = (ScriptArg::Int, ScriptArg::Float);
        let args = [i(crate::worldents::ACTOR_PED_TYPE), i(model.0), f(pos.x), f(pos.y), f(pos.z), ScriptArg::Local(0)];
        let h = self.run_op_out(crate::worldents::op::CREATE_CHAR, &args)? as i32;
        let ped = self.ped_from_script_handle(ScriptHandle(h)).ok_or(GameError::InvalidHandle)?;
        self.run_op(op::SET_CHAR_HEADING, &[i(h), f(heading_deg)])?;
        Ok(ped)
    }

    /// An actor's deletion: the ped's own scalar deleting destructor (no script command), while its pool slot still
    /// holds it. Mirrors samp.dll's actor wrapper destructor.
    fn destroy_ped(&mut self, ped: PedRef) -> GameResult<()> {
        self.ped_script_handle(ped)?;
        let vt = self.mem.read_u32(ped.0).map_err(read_error)?;
        if vt == game::VTBL_PLACEABLE {
            return Ok(());
        }
        #[cfg(windows)]
        {
            let dtor = self.mem.read_u32(vt).map_err(read_error)?;
            // SAFETY: the live ped's own scalar deleting destructor (thiscall, flag 1).
            unsafe {
                let f: extern "thiscall" fn(u32, u32) = std::mem::transmute(dtor as usize);
                f(ped.0, 1);
            }
            Ok(())
        }
        #[cfg(not(windows))]
        Err(GameError::Unsupported("Peds::destroy_ped outside the game"))
    }

    fn set_ped_model(&mut self, _ped: PedRef, _model: ModelId) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_model"))
    }

    fn ped_snapshot(&self, _ped: PedRef) -> GameResult<PedSnapshot> {
        Err(GameError::Unsupported("Peds::ped_snapshot"))
    }

    fn ped_weapons(&self, _ped: PedRef) -> GameResult<[WeaponSlot; 13]> {
        Err(GameError::Unsupported("Peds::ped_weapons"))
    }

    fn bone_position(&self, _ped: PedRef, _bone: BoneId, _transformed: bool) -> GameResult<Vec3> {
        Err(GameError::Unsupported("Peds::bone_position"))
    }

    fn node_matrix(&self, _ped: PedRef, _node: u8) -> GameResult<Matrix> {
        Err(GameError::Unsupported("Peds::node_matrix"))
    }

    fn weapon_skill(&self, _ped: PedRef) -> GameResult<u8> {
        Err(GameError::Unsupported("Peds::weapon_skill"))
    }

    fn ped_on_fire(&self, _ped: PedRef) -> GameResult<bool> {
        Err(GameError::Unsupported("Peds::ped_on_fire"))
    }

    fn anim_playing(&self, _ped: PedRef, _block: &str, _name: &str) -> GameResult<bool> {
        Err(GameError::Unsupported("Peds::anim_playing"))
    }

    fn player_money(&self) -> i32 {
        self.mem.read_i32(game::PLAYER_MONEY).unwrap_or(0)
    }

    /// The entity's own `Teleport` virtual (vtable +0x38: position, no rotation reset). Mirrors samp.dll 0x1009F0A0.
    fn set_ped_position(&mut self, ped: PedRef, pos: Vec3) -> GameResult<()> {
        let vt = self.mem.read_u32(ped.0).map_err(read_error)?;
        if vt == game::VTBL_PLACEABLE {
            return Ok(());
        }
        #[cfg(windows)]
        {
            let target = self.mem.read_u32(vt + 0x38).map_err(read_error)?;
            // SAFETY: the entity's own virtual (thiscall, CVector by value, bool).
            unsafe {
                let f: extern "thiscall" fn(u32, f32, f32, f32, u32) = std::mem::transmute(target as usize);
                f(ped.0, pos.x, pos.y, pos.z, 0);
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = pos;
            Err(GameError::Unsupported("Peds::set_ped_position outside the game"))
        }
    }

    fn set_ped_matrix(&mut self, _ped: PedRef, _m: &Matrix) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_matrix"))
    }

    fn set_ped_move_speed(&mut self, _ped: PedRef, _v: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_move_speed"))
    }

    /// `SET_CHAR_HEADING` (0173); the raw form writes the heading goal (radians) only, as samp's actors do.
    fn set_ped_heading(&mut self, ped: PedRef, heading: f32, raw: bool) -> GameResult<()> {
        if raw {
            return self.mem.write_data(ped.0 + game::off::PED_HEADING_GOAL, &heading.to_le_bytes()).map_err(super::write_error);
        }
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CHAR_HEADING, &[ScriptArg::Int(h), ScriptArg::Float(heading)]).map(|_| ())
    }

    /// `CPed::m_fHealth`, written directly.
    fn set_ped_health(&mut self, ped: PedRef, health: f32) -> GameResult<()> {
        self.mem.write_data(ped.0 + game::off::PED_HEALTH, &health.to_le_bytes()).map_err(super::write_error)
    }

    fn set_ped_armour(&mut self, _ped: PedRef, _armour: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_armour"))
    }

    fn give_weapon(&mut self, _ped: PedRef, _weapon: WeaponType, _ammo: u32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::give_weapon"))
    }

    /// `SET_CURRENT_CHAR_WEAPON` (01B9).
    fn set_current_weapon(&mut self, ped: PedRef, weapon: WeaponType) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CURRENT_CHAR_WEAPON, &[ScriptArg::Int(h), ScriptArg::Int(i32::from(weapon.0))]).map(|_| ())
    }

    fn clear_weapons(&mut self, _ped: PedRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::clear_weapons"))
    }

    fn set_weapon_slot(&mut self, _ped: PedRef, _slot: u8, _value: WeaponSlot) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_weapon_slot"))
    }

    fn remove_weapon_entering_vehicle(&mut self, _ped: PedRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::remove_weapon_entering_vehicle"))
    }

    /// `SET_CHAR_PROOFS` (02AB): bullet, fire, explosion, collision, melee.
    fn set_proofs(&mut self, ped: PedRef, p: Proofs) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        let b = |v: bool| ScriptArg::Int(i32::from(v));
        let args = [ScriptArg::Int(h), b(p.bullet), b(p.fire), b(p.explosion), b(p.collision), b(p.melee)];
        self.run_op(op::SET_CHAR_PROOFS, &args).map(|_| ())
    }

    /// `SET_CHAR_SUFFERS_CRITICAL_HITS` (0446).
    fn set_suffers_critical_hits(&mut self, ped: PedRef, on: bool) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CHAR_SUFFERS_CRITICAL_HITS, &[ScriptArg::Int(h), ScriptArg::Int(i32::from(on))]).map(|_| ())
    }

    /// `SET_CHAR_DROPS_WEAPONS_WHEN_DEAD` (087E).
    fn set_drops_weapons_when_dead(&mut self, ped: PedRef, on: bool) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CHAR_DROPS_WEAPONS_WHEN_DEAD, &[ScriptArg::Int(h), ScriptArg::Int(i32::from(on))])
            .map(|_| ())
    }

    /// `FREEZE_CHAR_POSITION` (04D7).
    fn freeze_ped(&mut self, ped: PedRef, frozen: bool) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::FREEZE_CHAR_POSITION, &[ScriptArg::Int(h), ScriptArg::Int(i32::from(frozen))]).map(|_| ())
    }

    /// `SET_CHAR_AREA_VISIBLE` (0860).
    fn set_ped_area(&mut self, ped: PedRef, area: AreaCode) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CHAR_AREA_VISIBLE, &[ScriptArg::Int(h), ScriptArg::Int(i32::from(area.0))]).map(|_| ())
    }

    /// `SET_CHAR_DECISION_MAKER` (060B).
    fn set_decision_maker(&mut self, ped: PedRef, maker: i32) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(crate::worldents::op::SET_CHAR_DECISION_MAKER, &[ScriptArg::Int(h), ScriptArg::Int(maker)]).map(|_| ())
    }

    fn set_player_money(&mut self, _money: i32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_player_money"))
    }

    /// `SET_PLAYER_CONTROL(0, enabled)` (01B4).
    fn set_player_control(&mut self, enabled: bool) -> GameResult<()> {
        self.run_op(op::SET_PLAYER_CONTROL, &[ScriptArg::Int(0), ScriptArg::Int(i32::from(enabled))]).map(|_| ())
    }

    /// `SET_PLAYER_DRUNKENNESS(0, level)` (052C).
    fn set_drunkenness(&mut self, level: i32) -> GameResult<()> {
        self.run_op(op::SET_PLAYER_DRUNKENNESS, &[ScriptArg::Int(0), ScriptArg::Int(level)]).map(|_| ())
    }

    fn set_ped_flags(&mut self, _ped: PedRef, _mask: [u32; 2], _value: [u32; 2]) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_flags"))
    }

    fn extinguish_ped(&mut self, _ped: PedRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::extinguish_ped"))
    }

    /// `EXPLODE_CHAR_HEAD` (0321).
    fn explode_head(&mut self, ped: PedRef) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(crate::worldents::op::EXPLODE_CHAR_HEAD, &[ScriptArg::Int(h)]).map(|_| ())
    }

    fn warp_into_vehicle(&mut self, _ped: PedRef, _vehicle: VehicleRef, _seat: Option<i32>) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::warp_into_vehicle"))
    }

    fn warp_out_of_vehicle(&mut self, _ped: PedRef, _pos: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::warp_out_of_vehicle"))
    }

    fn start_task(&mut self, _ped: PedRef, _task: PedTask) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::start_task"))
    }

    /// `CPedIntelligence::FlushImmediately(true)` on the ped's intelligence.
    fn flush_tasks(&mut self, ped: PedRef) -> GameResult<()> {
        let intel = self.mem.read_u32(ped.0 + game::off::PED_INTELLIGENCE).map_err(read_error)?;
        if intel == 0 {
            return Ok(());
        }
        #[cfg(windows)]
        {
            // SAFETY: CPedIntelligence::FlushImmediately(bool) (thiscall) on the ped's live intelligence.
            unsafe {
                let f: extern "thiscall" fn(u32, u32) = std::mem::transmute(FLUSH_IMMEDIATELY as usize);
                f(intel, 1);
            }
            Ok(())
        }
        #[cfg(not(windows))]
        Err(GameError::Unsupported("Peds::flush_tasks outside the game"))
    }

    fn delete_task(&mut self, _task: TaskRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::delete_task"))
    }

    /// The block loaded if needed (`REQUEST_ANIMATION` 04ED, polled with `HAS_ANIMATION_LOADED` 04EE for at most
    /// 15 ms: the animation is dropped when the block is not there by then), then `TASK_PLAY_ANIM_NON_INTERRUPTABLE`
    /// (0812). Mirrors samp.dll 0x1009C4C0.
    fn play_anim(&mut self, ped: PedRef, anim: &AnimRequest<'_>) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        let block = ScriptArg::Text(anim.block.as_bytes());
        if !self.run_op(op::HAS_ANIMATION_LOADED, &[block])? {
            self.run_op(op::REQUEST_ANIMATION, &[block])?;
            let t0 = samp_proto::time::Instant::now();
            while !self.run_op(op::HAS_ANIMATION_LOADED, &[block])? {
                if t0.elapsed() > std::time::Duration::from_millis(15) {
                    return Err(GameError::NotLoaded);
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        let b = |v: bool| ScriptArg::Int(i32::from(v));
        let args = [
            ScriptArg::Int(h),
            ScriptArg::Text(anim.name.as_bytes()),
            block,
            ScriptArg::Float(anim.delta),
            b(anim.looped),
            b(anim.lock_x),
            b(anim.lock_y),
            b(anim.freeze),
            ScriptArg::Int(anim.time_ms),
        ];
        self.run_op(crate::worldents::op::TASK_PLAY_ANIM_NON_INTERRUPTABLE, &args).map(|_| ())
    }

    fn stop_anims(&mut self, _ped: PedRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::stop_anims"))
    }

    fn look_at(&mut self, _ped: PedRef, _target: Vec3, _time_ms: i32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::look_at"))
    }

    fn set_jetpack(&mut self, _ped: PedRef, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_jetpack"))
    }

    fn has_jetpack(&self, _ped: PedRef) -> GameResult<bool> {
        Err(GameError::Unsupported("Peds::has_jetpack"))
    }
}
