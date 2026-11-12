//! Peds and players on the native engine: the native backend's `sa_samp_api::peds` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::peds::*;
use sa_samp_api::*;

use sa_samp_api::script::ScriptArg::{Float as F, Int as I, Text as T};
use sa_samp_api::streaming::Streaming;

use crate::addr::{self, off};
use crate::script::op;
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

    fn set_ped_model(&mut self, ped: PedRef, model: ModelId) -> GameResult<()> {
        // samp's `CPlayerPed::SetModelIndex`: a ped model only, loaded first (RequestModel +
        // LoadAllRequestedModels), then the model change. A player-slot ped goes through 09C7
        // SET_PLAYER_MODEL (`DeleteRwObject` + `SetModelIndex`, keeping the ped's +0x4D4).
        let p = self.ped_addr(ped)?;
        if !self.is_ped_model(model) {
            return Err(GameError::Other(format!("{} is not a ped model", model.0)));
        }
        let slot = (0..addr::PLAYER_SLOTS)
            .find(|&i| {
                self.view
                    .read_u32(addr::PLAYERS + u32::from(i) * addr::PLAYER_INFO_SIZE)
                    == p
            })
            .ok_or(GameError::Unsupported("Peds::set_ped_model (non-player ped)"))?;
        if !self.model_loaded(model) {
            self.request_model(model)?;
            self.load_all_now()?;
            if !self.model_loaded(model) {
                return Err(GameError::NotLoaded);
            }
        }
        self.run_op(op::SET_PLAYER_MODEL, &[I(i32::from(slot)), I(model.0)])
            .map(drop)
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

    fn set_ped_position(&mut self, ped: PedRef, pos: Vec3) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CHAR_COORDINATES, &[I(h), F(pos.x), F(pos.y), F(pos.z)])
            .map(drop)
    }

    fn set_ped_matrix(&mut self, _ped: PedRef, _m: &Matrix) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_matrix"))
    }

    fn set_ped_move_speed(&mut self, _ped: PedRef, _v: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_ped_move_speed"))
    }

    fn set_ped_heading(&mut self, ped: PedRef, heading: f32, raw: bool) -> GameResult<()> {
        if raw {
            // The raw current/goal heading fields in radians (samp's remote-player writes).
            let p = self.ped_addr(ped)?;
            self.view.write(p + off::PED_HEADING, &heading.to_le_bytes());
            self.view.write(p + off::PED_HEADING_GOAL, &heading.to_le_bytes());
            return Ok(());
        }
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CHAR_HEADING, &[I(h), F(heading)]).map(drop)
    }

    fn set_ped_health(&mut self, ped: PedRef, health: f32) -> GameResult<()> {
        let p = self.ped_addr(ped)?;
        self.view.write(p + off::PED_HEALTH, &health.to_le_bytes());
        Ok(())
    }

    fn set_ped_armour(&mut self, ped: PedRef, armour: f32) -> GameResult<()> {
        let p = self.ped_addr(ped)?;
        self.view.write(p + off::PED_ARMOUR, &armour.to_le_bytes());
        Ok(())
    }

    fn give_weapon(&mut self, ped: PedRef, weapon: WeaponType, ammo: u32) -> GameResult<()> {
        // samp (samp.dll 0x100AF7D0, samp-rs `client::ped::give_weapon`): the weapon's model loaded first (its
        // `CWeaponInfo::m_nModelId1`, standard skill), so the ped holds it and the HUD has its icon; then the gift.
        let h = self.ped_script_handle(ped)?.0;
        let model = if weapon.0 < addr::WEAPON_INFO_STD_COUNT {
            self.view
                .read_i32(addr::WEAPON_INFO + u32::from(weapon.0) * addr::WEAPON_INFO_SIZE + 0xC)
        } else {
            -1
        };
        if model > 0 && !self.model_loaded(ModelId(model)) {
            self.request_model(ModelId(model))?;
            self.load_all_now()?;
            if !self.model_loaded(ModelId(model)) {
                return Err(GameError::NotLoaded);
            }
        }
        self.run_op(
            op::GIVE_WEAPON_TO_CHAR,
            &[I(h), I(i32::from(weapon.0)), I(ammo as i32)],
        )
        .map(drop)
    }

    fn set_current_weapon(&mut self, ped: PedRef, weapon: WeaponType) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CURRENT_CHAR_WEAPON, &[I(h), I(i32::from(weapon.0))])
            .map(drop)
    }

    fn clear_weapons(&mut self, ped: PedRef) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::REMOVE_ALL_CHAR_WEAPONS, &[I(h)]).map(drop)
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

    fn set_suffers_critical_hits(&mut self, ped: PedRef, on: bool) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CHAR_SUFFERS_CRITICAL_HITS, &[I(h), I(i32::from(on))])
            .map(drop)
    }

    fn set_drops_weapons_when_dead(&mut self, ped: PedRef, on: bool) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CHAR_DROPS_WEAPONS_WHEN_DEAD, &[I(h), I(i32::from(on))])
            .map(drop)
    }

    fn freeze_ped(&mut self, ped: PedRef, frozen: bool) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::FREEZE_CHAR_POSITION, &[I(h), I(i32::from(frozen))])
            .map(drop)
    }

    fn set_ped_area(&mut self, ped: PedRef, area: AreaCode) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::SET_CHAR_AREA_VISIBLE, &[I(h), I(i32::from(area.0))])
            .map(drop)
    }

    fn set_decision_maker(&mut self, _ped: PedRef, _maker: i32) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::set_decision_maker"))
    }

    fn set_player_money(&mut self, money: i32) -> GameResult<()> {
        // `CPlayerInfo::m_nMoney` and the HUD's `m_nDisplayMoney` (+4), as samp's money reset leaves them.
        self.view.write(addr::PLAYER_MONEY, &money.to_le_bytes());
        self.view.write(addr::PLAYER_MONEY + 4, &money.to_le_bytes());
        Ok(())
    }

    fn set_player_control(&mut self, enabled: bool) -> GameResult<()> {
        self.run_op(op::SET_PLAYER_CONTROL, &[I(0), I(i32::from(enabled))])
            .map(drop)
    }

    fn set_drunkenness(&mut self, level: i32) -> GameResult<()> {
        self.run_op(op::SET_PLAYER_DRUNKENNESS, &[I(0), I(level)])
            .map(drop)
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

    fn warp_into_vehicle(&mut self, ped: PedRef, vehicle: VehicleRef, seat: Option<i32>) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        let v = sa_samp_api::vehicles::Vehicles::vehicle_script_handle(self, vehicle)?.0;
        match seat {
            None => self.run_op(op::WARP_CHAR_INTO_CAR, &[I(h), I(v)]),
            Some(s) => self.run_op(op::WARP_CHAR_INTO_CAR_AS_PASSENGER, &[I(h), I(v), I(s)]),
        }
        .map(drop)
    }

    fn warp_out_of_vehicle(&mut self, ped: PedRef, pos: Vec3) -> GameResult<()> {
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(
            op::WARP_CHAR_FROM_CAR_TO_COORD,
            &[I(h), F(pos.x), F(pos.y), F(pos.z)],
        )
        .map(drop)
    }

    fn start_task(&mut self, ped: PedRef, task: PedTask) -> GameResult<()> {
        // The script tasks samp starts (samp-rs `client::ped`, `client::rpc_player`), through the ported handlers.
        let h = self.ped_script_handle(ped)?.0;
        let veh = |g: &Self, v: VehicleRef| {
            sa_samp_api::vehicles::Vehicles::vehicle_script_handle(g, v).map(|s| s.0)
        };
        match task {
            PedTask::HandsUp { time_ms } => self.run_op(op::TASK_HANDS_UP, &[I(h), I(time_ms)]),
            PedTask::UseMobilePhone(on) => {
                // CTaskComplexUseMobilePhone's functions (0x6348E0 ctor, 0x634930 MakeAbortable, 0x634A40 Stop,
                // 0x634C10 CreateFirstSubTask) are unported: the task would block every frame it lives.
                use sa_core::orig::registry;
                let missing = [0x6348E0usize, 0x634930, 0x634A40, 0x634C10]
                    .iter()
                    .any(|&a| registry::name_of(a).is_none());
                if registry::is_installed() && missing {
                    return Err(GameError::Unsupported(
                        "Peds::start_task (CTaskComplexUseMobilePhone is not ported)",
                    ));
                }
                self.run_op(op::TASK_USE_MOBILE_PHONE, &[I(h), I(i32::from(on))])
            }
            PedTask::EnterCarAsDriver { vehicle, time_ms } => {
                let v = veh(self, vehicle)?;
                self.run_op(op::TASK_ENTER_CAR_AS_DRIVER, &[I(h), I(v), I(time_ms)])
            }
            PedTask::EnterCarAsPassenger {
                vehicle,
                time_ms,
                seat,
            } => {
                let v = veh(self, vehicle)?;
                self.run_op(
                    op::TASK_ENTER_CAR_AS_PASSENGER,
                    &[I(h), I(v), I(time_ms), I(seat)],
                )
            }
            PedTask::LeaveCar { vehicle } => {
                let v = veh(self, vehicle)?;
                self.run_op(op::TASK_LEAVE_CAR, &[I(h), I(v)])
            }
            _ => Err(GameError::Unsupported("Peds::start_task (this task)")),
        }
        .map(drop)
    }

    fn flush_tasks(&mut self, _ped: PedRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::flush_tasks"))
    }

    fn delete_task(&mut self, _task: TaskRef) -> GameResult<()> {
        Err(GameError::Unsupported("Peds::delete_task"))
    }

    fn play_anim(&mut self, ped: PedRef, anim: &AnimRequest<'_>) -> GameResult<()> {
        // samp's `CPlayerPed::ApplyAnimation`: the IFP block loaded first (04ED REQUEST_ANIMATION +
        // LoadAllRequestedModels, 04EE HAS_ANIMATION_LOADED), then 0812 TASK_PLAY_ANIM_NON_INTERRUPTABLE.
        let h = self.ped_script_handle(ped)?.0;
        if anim.name.is_empty() || anim.name.len() > 23 {
            return Err(GameError::Other(format!("bad animation name {:?}", anim.name)));
        }
        if self.anim_block_addr(anim.block).is_none() {
            return Err(GameError::Other(format!(
                "unknown animation block {:?}",
                anim.block
            )));
        }
        let block = anim.block.as_bytes();
        if !self.anim_block_loaded(anim.block) {
            self.run_op(op::REQUEST_ANIMATION, &[T(block)])?;
            self.load_all_now()?;
            if !self.run_op(op::HAS_ANIMATION_LOADED, &[T(block)])? {
                return Err(GameError::NotLoaded);
            }
        }
        self.run_op(
            op::TASK_PLAY_ANIM_NON_INTERRUPTABLE,
            &[
                I(h),
                T(anim.name.as_bytes()),
                T(block),
                F(anim.delta),
                I(i32::from(anim.looped)),
                I(i32::from(anim.lock_x)),
                I(i32::from(anim.lock_y)),
                I(i32::from(anim.freeze)),
                I(anim.time_ms),
            ],
        )
        .map(drop)
    }

    fn stop_anims(&mut self, ped: PedRef) -> GameResult<()> {
        // samp's ClearAnimations: 0687 CLEAR_CHAR_TASKS (`CPedIntelligence::ClearTasks(true, true)`).
        let h = self.ped_script_handle(ped)?.0;
        self.run_op(op::CLEAR_CHAR_TASKS, &[I(h)]).map(drop)
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
