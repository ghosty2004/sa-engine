//! Peds and players on the native engine: the native backend's `sa_samp_api::peds` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::peds::*;
use sa_samp_api::*;

use crate::addr::{self, off};
use crate::{NativeGame, View};

impl<V: View> NativeGame<V> {
    /// The address of a live ped, or `InvalidHandle`.
    pub(crate) fn ped_addr(&self, ped: PedRef) -> GameResult<u32> {
        addr::PED_POOL
            .at_ref(&self.view, ped.0)
            .ok_or(GameError::InvalidHandle)
    }

    /// The handle of the ped at `p` (a pointer read from game state), if it is a live pool ped.
    pub(crate) fn ped_ref(&self, p: u32) -> Option<PedRef> {
        if p == 0 {
            return None;
        }
        addr::PED_POOL.ref_of(&self.view, p).map(PedRef)
    }

    /// An entity's matrix: `m_matrix` when it has one, else the placement (position + heading about z).
    pub(crate) fn entity_matrix(&self, e: u32) -> Matrix {
        let m = self.view.read_u32(e + off::MATRIX);
        if m != 0 {
            return self.view.read_matrix(m);
        }
        let (s, c) = self.view.read_f32(e + off::PLACEMENT_HEADING).sin_cos();
        Matrix {
            right: Vec3::new(c, s, 0.0),
            forward: Vec3::new(-s, c, 0.0),
            up: Vec3::new(0.0, 0.0, 1.0),
            pos: self.view.read_vec3(e + off::PLACEMENT_POS),
        }
    }

    /// samp's "ped is live" test (samp.dll 0x1009F430, samp-rs `client::gta::ped_in_world`): the vtable is
    /// not `CPlaceable`'s (a destroyed entity) and the ped is in the moving list.
    fn ped_live(&self, p: u32) -> bool {
        let vt = self.view.read_u32(p + off::VTABLE);
        vt != 0 && vt != addr::VTBL_PLACEABLE && self.view.read_u32(p + off::MOVING_LIST) != 0
    }
}

impl<V: View> Peds for NativeGame<V> {
    fn local_player(&self) -> Option<PedRef> {
        self.ped_ref(self.view.read_u32(addr::PLAYERS))
    }

    fn player_ped(&self, slot: PlayerSlot) -> Option<PedRef> {
        if slot.0 >= addr::PLAYER_SLOTS {
            return None;
        }
        self.ped_ref(
            self.view
                .read_u32(addr::PLAYERS + u32::from(slot.0) * addr::PLAYER_INFO_SIZE),
        )
    }

    fn player_in_focus(&self) -> PlayerSlot {
        PlayerSlot(self.view.read_u8(addr::PLAYER_IN_FOCUS))
    }

    fn ped_script_handle(&self, ped: PedRef) -> GameResult<ScriptHandle> {
        // The handle is the pool reference itself (crate docs, "Handles").
        self.ped_addr(ped).map(|_| ScriptHandle(ped.0 as i32))
    }

    fn ped_from_script_handle(&self, h: ScriptHandle) -> Option<PedRef> {
        addr::PED_POOL
            .at_ref(&self.view, h.0 as u32)
            .map(|_| PedRef(h.0 as u32))
    }

    fn ped_in_world(&self, ped: PedRef) -> bool {
        self.ped_addr(ped).is_ok_and(|p| self.ped_live(p))
    }

    fn is_ped_model(&self, model: ModelId) -> bool {
        let Ok(m) = u32::try_from(model.0) else {
            return false;
        };
        if m >= addr::MODEL_INFO_COUNT {
            return false;
        }
        let info = self.view.read_u32(addr::MODEL_INFO_PTRS + 4 * m);
        info != 0 && self.view.read_u32(info) == addr::VTBL_PED_MODEL_INFO
    }

    fn create_player_ped(&mut self, _slot: PlayerSlot, _model: ModelId, _pos: Vec3) -> GameResult<PedRef> {
        Err(GameError::Unsupported("Peds::create_player_ped"))
    }

    fn create_actor(&mut self, _model: ModelId, _pos: Vec3, _heading_deg: f32) -> GameResult<PedRef> {
        Err(GameError::Unsupported("Peds::create_actor"))
    }

    fn destroy_ped(&mut self, _ped: PedRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::destroy_ped"))
    }

    fn set_ped_model(&mut self, _ped: PedRef, _model: ModelId) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_model"))
    }

    fn ped_snapshot(&self, ped: PedRef) -> GameResult<PedSnapshot> {
        let p = self.ped_addr(ped)?;
        let v = &self.view;
        let flags = [v.read_u32(p + off::PED_FLAGS), v.read_u32(p + off::PED_FLAGS + 4)];
        let weapon_slot = v.read_u8(p + off::PED_WEAPON_SLOT);
        let weapon = if weapon_slot < 13 {
            v.read_u32(p + off::PED_WEAPONS + u32::from(weapon_slot) * off::WEAPON_SIZE) as u8
        } else {
            0
        };
        let vehicle = if flags[0] & off::PED_IN_VEHICLE != 0 {
            self.vehicle_ref(v.read_u32(p + off::PED_VEHICLE))
        } else {
            None
        };
        Ok(PedSnapshot {
            matrix: self.entity_matrix(p),
            move_speed: v.read_vec3(p + off::MOVE_SPEED),
            health: v.read_f32(p + off::PED_HEALTH),
            armour: v.read_f32(p + off::PED_ARMOUR),
            heading: v.read_f32(p + off::PED_HEADING),
            heading_goal: v.read_f32(p + off::PED_HEADING_GOAL),
            ped_state: v.read_u32(p + off::PED_STATE),
            model: ModelId(i32::from(v.read_i16(p + off::MODEL))),
            weapon_slot,
            weapon: WeaponType(weapon),
            vehicle,
            area: AreaCode(v.read_u8(p + off::AREA)),
            flags,
            in_world: self.ped_live(p),
        })
    }

    fn ped_weapons(&self, ped: PedRef) -> GameResult<[WeaponSlot; 13]> {
        let p = self.ped_addr(ped)?;
        let mut out = [WeaponSlot::default(); 13];
        for (i, w) in out.iter_mut().enumerate() {
            let a = p + off::PED_WEAPONS + i as u32 * off::WEAPON_SIZE;
            *w = WeaponSlot {
                weapon: self.view.read_u32(a) as u8,
                state: self.view.read_u32(a + 4),
                ammo_in_clip: self.view.read_u32(a + 8),
                ammo_total: self.view.read_u32(a + 0xC),
            };
        }
        Ok(out)
    }

    fn bone_position(&self, _ped: PedRef, _bone: BoneId, _transformed: bool) -> GameResult<Vec3> {
        Err(GameError::Unsupported("Peds::bone_position"))
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
        self.view.read_i32(addr::PLAYER_MONEY)
    }

    fn set_ped_position(&mut self, _ped: PedRef, _pos: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_position"))
    }

    fn set_ped_matrix(&mut self, _ped: PedRef, _m: &Matrix) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_matrix"))
    }

    fn set_ped_move_speed(&mut self, _ped: PedRef, _v: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_move_speed"))
    }

    fn set_ped_heading(&mut self, _ped: PedRef, _heading: f32, _raw: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_heading"))
    }

    fn set_ped_health(&mut self, _ped: PedRef, _health: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_health"))
    }

    fn set_ped_armour(&mut self, _ped: PedRef, _armour: f32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_armour"))
    }

    fn give_weapon(&mut self, _ped: PedRef, _weapon: WeaponType, _ammo: u32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::give_weapon"))
    }

    fn set_current_weapon(&mut self, _ped: PedRef, _weapon: WeaponType) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_current_weapon"))
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

    fn set_proofs(&mut self, _ped: PedRef, _proofs: Proofs) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_proofs"))
    }

    fn set_suffers_critical_hits(&mut self, _ped: PedRef, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_suffers_critical_hits"))
    }

    fn set_drops_weapons_when_dead(&mut self, _ped: PedRef, _on: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_drops_weapons_when_dead"))
    }

    fn freeze_ped(&mut self, _ped: PedRef, _frozen: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::freeze_ped"))
    }

    fn set_ped_area(&mut self, _ped: PedRef, _area: AreaCode) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_area"))
    }

    fn set_decision_maker(&mut self, _ped: PedRef, _maker: i32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_decision_maker"))
    }

    fn set_player_money(&mut self, _money: i32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_player_money"))
    }

    fn set_player_control(&mut self, _enabled: bool) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_player_control"))
    }

    fn set_drunkenness(&mut self, _level: i32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_drunkenness"))
    }

    fn set_ped_flags(&mut self, _ped: PedRef, _mask: [u32; 2], _value: [u32; 2]) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_flags"))
    }

    fn extinguish_ped(&mut self, _ped: PedRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::extinguish_ped"))
    }

    fn explode_head(&mut self, _ped: PedRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::explode_head"))
    }

    fn warp_into_vehicle(
        &mut self,
        _ped: PedRef,
        _vehicle: VehicleRef,
        _seat: Option<i32>,
    ) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::warp_into_vehicle"))
    }

    fn warp_out_of_vehicle(&mut self, _ped: PedRef, _pos: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::warp_out_of_vehicle"))
    }

    fn start_task(&mut self, _ped: PedRef, _task: PedTask) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::start_task"))
    }

    fn flush_tasks(&mut self, _ped: PedRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::flush_tasks"))
    }

    fn delete_task(&mut self, _task: TaskRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::delete_task"))
    }

    fn play_anim(&mut self, _ped: PedRef, _anim: &AnimRequest<'_>) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::play_anim"))
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
