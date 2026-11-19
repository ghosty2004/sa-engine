//! Peds, vehicles and objects: lookups, records, changes, creation and deletion.

use sa_types::ped::{PedSnapshot, WeaponSlot};
use sa_types::script::ScriptArg::{Float as F, Int as I, Local as L};
use sa_types::vehicle::{VehicleClass, VehicleSnapshot, VehicleSpawn};
use sa_types::{AreaCode, GameError, GameResult, Matrix, ModelId, ObjectRef, PedRef, Vec3, VehicleRef, WeaponType};

use crate::addr::{self, off};
use crate::script::op;
use crate::{Game, View};

/// `ePedType` of a civilian (`PED_TYPE_CIVMALE`), the type a created ped gets.
const PED_TYPE_CIVMALE: i32 = 4;

impl<V: View> Game<V> {
    /// The address of a live ped, or `InvalidHandle`.
    pub fn ped_addr(&self, ped: PedRef) -> GameResult<u32> {
        addr::PED_POOL.at_ref(&self.view, ped.0).ok_or(GameError::InvalidHandle)
    }

    /// The handle of the ped at `p` (a pointer read from game state), if it is a live pool ped.
    pub fn ped_ref(&self, p: u32) -> Option<PedRef> {
        if p == 0 {
            return None;
        }
        addr::PED_POOL.ref_of(&self.view, p).map(PedRef)
    }

    /// The address of a live vehicle, or `InvalidHandle`.
    pub fn vehicle_addr(&self, v: VehicleRef) -> GameResult<u32> {
        addr::VEHICLE_POOL.at_ref(&self.view, v.0).ok_or(GameError::InvalidHandle)
    }

    /// The handle of the vehicle at `p` (a pointer read from game state), if it is a live pool vehicle.
    pub fn vehicle_ref(&self, p: u32) -> Option<VehicleRef> {
        if p == 0 {
            return None;
        }
        addr::VEHICLE_POOL.ref_of(&self.view, p).map(VehicleRef)
    }

    /// The address of a live object, or `InvalidHandle`.
    pub fn object_addr(&self, o: ObjectRef) -> GameResult<u32> {
        addr::OBJECT_POOL.at_ref(&self.view, o.0).ok_or(GameError::InvalidHandle)
    }

    /// An entity's matrix: `m_matrix` when it has one, else the placement (position + heading about z).
    pub fn entity_matrix(&self, e: u32) -> Matrix {
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

    /// Whether the ped at `p` is in the world: its vtable is not `CPlaceable`'s (a destroyed entity) and it is in
    /// the moving list.
    pub fn ped_live(&self, p: u32) -> bool {
        let vt = self.view.read_u32(p + off::VTABLE);
        vt != 0 && vt != addr::VTBL_PLACEABLE && self.view.read_u32(p + off::MOVING_LIST) != 0
    }

    /// The local player's ped (`CWorld::Players[0].m_pPed`).
    pub fn local_player(&self) -> Option<PedRef> {
        self.ped_ref(self.view.read_u32(addr::PLAYERS))
    }

    /// The ped of player slot `slot` (`CWorld::Players[slot]`).
    pub fn player_ped(&self, slot: u8) -> Option<PedRef> {
        if slot >= addr::PLAYER_SLOTS {
            return None;
        }
        self.ped_ref(self.view.read_u32(addr::PLAYERS + u32::from(slot) * addr::PLAYER_INFO_SIZE))
    }

    /// The player slot whose ped is `ped`.
    pub fn player_slot_of(&self, ped: PedRef) -> Option<u8> {
        let p = self.ped_addr(ped).ok()?;
        (0..addr::PLAYER_SLOTS).find(|&i| self.view.read_u32(addr::PLAYERS + u32::from(i) * addr::PLAYER_INFO_SIZE) == p)
    }

    /// Whether `model` is a ped model (its model info is a `CPedModelInfo`).
    pub fn is_ped_model(&self, model: ModelId) -> bool {
        let Ok(m) = u32::try_from(model.0) else {
            return false;
        };
        if m >= addr::MODEL_INFO_COUNT {
            return false;
        }
        let info = self.view.read_u32(addr::MODEL_INFO_PTRS + 4 * m);
        info != 0 && self.view.read_u32(info) == addr::VTBL_PED_MODEL_INFO
    }

    /// The ped's state.
    pub fn ped_snapshot(&self, ped: PedRef) -> GameResult<PedSnapshot> {
        let p = self.ped_addr(ped)?;
        let v = &self.view;
        let flags = [v.read_u32(p + off::PED_FLAGS), v.read_u32(p + off::PED_FLAGS + 4)];
        let weapon_slot = v.read_u8(p + off::PED_WEAPON_SLOT);
        let weapon = if weapon_slot < 13 {
            v.read_u32(p + off::PED_WEAPONS + u32::from(weapon_slot) * off::WEAPON_SIZE) as u8
        } else {
            0
        };
        let vehicle =
            if flags[0] & off::PED_IN_VEHICLE != 0 { self.vehicle_ref(v.read_u32(p + off::PED_VEHICLE)) } else { None };
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

    /// The ped's 13 weapon slots.
    pub fn ped_weapons(&self, ped: PedRef) -> GameResult<[WeaponSlot; 13]> {
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

    /// The local player's money (`CWorld::Players[0].m_nMoney`).
    pub fn player_money(&self) -> i32 {
        self.view.read_i32(addr::PLAYER_MONEY)
    }

    /// The local player's wanted level (`CWanted::m_WantedLevel` of its `CPlayerPedData`).
    pub fn wanted_level(&self) -> GameResult<u8> {
        let ped = self.view.read_u32(addr::PLAYERS);
        if self.ped_ref(ped).is_none() {
            return Err(GameError::InvalidHandle);
        }
        let data = self.view.read_u32(ped + off::PED_PLAYER_DATA);
        let wanted = if data == 0 { 0 } else { self.view.read_u32(data + off::PLAYER_DATA_WANTED) };
        if wanted == 0 {
            return Err(GameError::InvalidHandle);
        }
        Ok(self.view.read_u32(wanted + off::WANTED_LEVEL).min(u32::from(u8::MAX)) as u8)
    }

    /// Every ped in the ped pool, in slot order.
    pub fn peds(&self) -> Vec<PedRef> {
        addr::PED_POOL.refs(&self.view).into_iter().map(PedRef).collect()
    }

    /// The vehicle's state.
    pub fn vehicle_snapshot(&self, v: VehicleRef) -> GameResult<VehicleSnapshot> {
        let p = self.vehicle_addr(v)?;
        let m = &self.view;
        let vtable = m.read_u32(p + off::VTABLE);
        let class = addr::VEHICLE_VTABLES.iter().find(|(vt, _)| *vt == vtable).map(|(_, c)| *c);
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
            colours: [m.read_u8(p + off::VEH_COLOURS), m.read_u8(p + off::VEH_COLOURS + 1)],
            siren: m.read_u8(p + off::VEH_SIREN_BYTE) & 0x80 != 0,
            // Landing gear and the special control value are not read yet (their fields differ per class).
            gear_down: false,
            area: AreaCode(m.read_u8(p + off::AREA)),
            train_speed: if class == Some(VehicleClass::Train) { m.read_f32(p + off::TRAIN_SPEED) } else { 0.0 },
            special: 0.0,
        })
    }

    /// The local player's vehicle (`FindPlayerVehicle(-1, false)`: its ped's `m_pVehicle` while `bInVehicle`).
    pub fn local_vehicle(&self) -> Option<VehicleRef> {
        let ped = self.view.read_u32(addr::PLAYERS);
        if ped == 0 || self.ped_ref(ped).is_none() {
            return None;
        }
        if self.view.read_u32(ped + off::PED_FLAGS) & off::PED_IN_VEHICLE == 0 {
            return None;
        }
        self.vehicle_ref(self.view.read_u32(ped + off::PED_VEHICLE))
    }

    /// Every vehicle in the vehicle pool, in slot order.
    pub fn vehicles(&self) -> Vec<VehicleRef> {
        addr::VEHICLE_POOL.refs(&self.view).into_iter().map(VehicleRef).collect()
    }

    /// Every object in the object pool, in slot order.
    pub fn objects(&self) -> Vec<ObjectRef> {
        addr::OBJECT_POOL.refs(&self.view).into_iter().map(ObjectRef).collect()
    }

    /// Moves the ped (`SET_CHAR_COORDINATES` 00A1: out of a vehicle first, onto the ground).
    pub fn set_ped_position(&mut self, ped: PedRef, pos: Vec3) -> GameResult<()> {
        self.ped_addr(ped)?;
        self.run_op(op::SET_CHAR_COORDINATES, &[I(ped.0 as i32), F(pos.x), F(pos.y), F(pos.z)]).map(drop)
    }

    /// Turns the ped to `heading` (`SET_CHAR_HEADING` 0173, degrees).
    pub fn set_ped_heading(&mut self, ped: PedRef, heading_deg: f32) -> GameResult<()> {
        self.ped_addr(ped)?;
        self.run_op(op::SET_CHAR_HEADING, &[I(ped.0 as i32), F(heading_deg)]).map(drop)
    }

    /// Writes the ped's current and goal heading in radians, as they are (no script command).
    pub fn set_ped_heading_raw(&mut self, ped: PedRef, heading: f32) -> GameResult<()> {
        let p = self.ped_addr(ped)?;
        self.view.write(p + off::PED_HEADING, &heading.to_le_bytes());
        self.view.write(p + off::PED_HEADING_GOAL, &heading.to_le_bytes());
        Ok(())
    }

    /// Sets the ped's health (`CPed::m_fHealth`).
    pub fn set_ped_health(&mut self, ped: PedRef, health: f32) -> GameResult<()> {
        let p = self.ped_addr(ped)?;
        self.view.write(p + off::PED_HEALTH, &health.to_le_bytes());
        Ok(())
    }

    /// Sets the ped's armour (`CPed::m_fArmour`).
    pub fn set_ped_armour(&mut self, ped: PedRef, armour: f32) -> GameResult<()> {
        let p = self.ped_addr(ped)?;
        self.view.write(p + off::PED_ARMOUR, &armour.to_le_bytes());
        Ok(())
    }

    /// Gives the ped `weapon` with `ammo` (`GIVE_WEAPON_TO_CHAR` 01B2), the weapon's model (its `CWeaponInfo`'s
    /// `m_nModelId1`, standard skill) loaded first so the ped holds it and the HUD has its icon.
    pub fn give_weapon(&mut self, ped: PedRef, weapon: WeaponType, ammo: u32) -> GameResult<()> {
        self.ped_addr(ped)?;
        let model = if weapon.0 < addr::WEAPON_INFO_STD_COUNT {
            self.view.read_i32(addr::WEAPON_INFO + u32::from(weapon.0) * addr::WEAPON_INFO_SIZE + 0xC)
        } else {
            -1
        };
        if model > 0 {
            self.load_model_now(ModelId(model))?;
        }
        self.run_op(op::GIVE_WEAPON_TO_CHAR, &[I(ped.0 as i32), I(i32::from(weapon.0)), I(ammo as i32)]).map(drop)
    }

    /// Sets the local player's money and the HUD's displayed money (`m_nDisplayMoney`, +4) to `money`.
    pub fn set_player_money(&mut self, money: i32) -> GameResult<()> {
        self.view.write(addr::PLAYER_MONEY, &money.to_le_bytes());
        self.view.write(addr::PLAYER_MONEY + 4, &money.to_le_bytes());
        Ok(())
    }

    /// Sets the local player's wanted level (`ALTER_WANTED_LEVEL` 010D: `CWanted::SetWantedLevel`).
    pub fn set_wanted_level(&mut self, level: u8) -> GameResult<()> {
        self.local_player().ok_or(GameError::InvalidHandle)?;
        self.run_op(op::ALTER_WANTED_LEVEL, &[I(0), I(i32::from(level.min(6)))]).map(drop)
    }

    /// Creates a civilian ped of `model` at `pos` (`CREATE_CHAR` 009A), its model loaded first.
    pub fn create_ped(&mut self, model: ModelId, pos: Vec3) -> GameResult<PedRef> {
        if !self.is_ped_model(model) {
            return Err(GameError::Other(format!("{} is not a ped model", model.0)));
        }
        self.load_model_now(model)?;
        let h = self.run_op_out(op::CREATE_CHAR, &[I(PED_TYPE_CIVMALE), I(model.0), F(pos.x), F(pos.y), F(pos.z), L(0)])?;
        addr::PED_POOL.at_ref(&self.view, h).map(|_| PedRef(h)).ok_or(GameError::NotLoaded)
    }

    /// Deletes a ped (`DELETE_CHAR` 009B); the player's ped is refused.
    pub fn destroy_ped(&mut self, ped: PedRef) -> GameResult<()> {
        self.ped_addr(ped)?;
        if self.player_slot_of(ped).is_some() {
            return Err(GameError::Other("the player's ped cannot be deleted".into()));
        }
        self.run_op(op::DELETE_CHAR, &[I(ped.0 as i32)]).map(drop)
    }

    /// Creates a vehicle (`CREATE_CAR` 00A5, then its heading, colours and area); its model must be loaded.
    pub fn create_vehicle(&mut self, spawn: &VehicleSpawn) -> GameResult<VehicleRef> {
        let v = self.create_car(spawn.model, spawn.pos)?;
        let h = v.0 as i32;
        self.run_op(op::SET_CAR_HEADING, &[I(h), F(spawn.heading_deg)])?;
        self.run_op(op::CHANGE_CAR_COLOUR, &[I(h), I(i32::from(spawn.colours[0])), I(i32::from(spawn.colours[1]))])?;
        if spawn.area.0 != 0 {
            self.run_op(op::SET_VEHICLE_AREA_VISIBLE, &[I(h), I(i32::from(spawn.area.0))])?;
        }
        Ok(v)
    }

    /// Creates a vehicle of `model` at `pos` with the colours the game picks (`CREATE_CAR` 00A5); its model must be
    /// loaded.
    pub fn create_car(&mut self, model: ModelId, pos: Vec3) -> GameResult<VehicleRef> {
        let h = self.run_op_out(op::CREATE_CAR, &[I(model.0), F(pos.x), F(pos.y), F(pos.z), L(0)])?;
        addr::VEHICLE_POOL.at_ref(&self.view, h).map(|_| VehicleRef(h)).ok_or(GameError::NotLoaded)
    }

    /// Deletes a vehicle (`DELETE_CAR` 00A6).
    pub fn destroy_vehicle(&mut self, v: VehicleRef) -> GameResult<()> {
        self.vehicle_addr(v)?;
        self.run_op(op::DELETE_CAR, &[I(v.0 as i32)]).map(drop)
    }

    /// Moves the vehicle (`SET_CAR_COORDINATES` 00AB).
    pub fn set_vehicle_position(&mut self, v: VehicleRef, pos: Vec3) -> GameResult<()> {
        self.vehicle_addr(v)?;
        self.run_op(op::SET_CAR_COORDINATES, &[I(v.0 as i32), F(pos.x), F(pos.y), F(pos.z)]).map(drop)
    }

    /// Turns the vehicle to `heading_deg` (`SET_CAR_HEADING` 0175).
    pub fn set_vehicle_heading(&mut self, v: VehicleRef, heading_deg: f32) -> GameResult<()> {
        self.vehicle_addr(v)?;
        self.run_op(op::SET_CAR_HEADING, &[I(v.0 as i32), F(heading_deg)]).map(drop)
    }

    /// Sets the vehicle's health (`CVehicle::m_fHealth`).
    pub fn set_vehicle_health(&mut self, v: VehicleRef, health: f32) -> GameResult<()> {
        let a = self.vehicle_addr(v)?;
        self.view.write(a + off::VEH_HEALTH, &health.to_le_bytes());
        Ok(())
    }

    /// Sets the vehicle's colours (`CHANGE_CAR_COLOUR` 0229).
    pub fn set_vehicle_colours(&mut self, v: VehicleRef, colours: [u8; 2]) -> GameResult<()> {
        self.vehicle_addr(v)?;
        self.run_op(op::CHANGE_CAR_COLOUR, &[I(v.0 as i32), I(i32::from(colours[0])), I(i32::from(colours[1]))])
            .map(drop)
    }

    /// Creates an object of `model` at `pos` (`CREATE_OBJECT` 0107); its model must be loaded.
    pub fn create_object(&mut self, model: ModelId, pos: Vec3) -> GameResult<ObjectRef> {
        let h = self.run_op_out(op::CREATE_OBJECT, &[I(model.0), F(pos.x), F(pos.y), F(pos.z), L(0)])?;
        addr::OBJECT_POOL.at_ref(&self.view, h).map(|_| ObjectRef(h)).ok_or(GameError::NotLoaded)
    }

    /// Deletes an object (`DELETE_OBJECT` 0108).
    pub fn destroy_object(&mut self, o: ObjectRef) -> GameResult<()> {
        self.object_addr(o)?;
        self.clear_object_materials(o);
        self.run_op(op::DELETE_OBJECT, &[I(o.0 as i32)]).map(drop)
    }

    /// Moves the object (`SET_OBJECT_COORDINATES` 01BC).
    pub fn set_object_position(&mut self, o: ObjectRef, pos: Vec3) -> GameResult<()> {
        self.object_addr(o)?;
        self.run_op(op::SET_OBJECT_COORDINATES, &[I(o.0 as i32), F(pos.x), F(pos.y), F(pos.z)]).map(drop)
    }

    /// Whether the game defines `model` (`CModelInfo::ms_modelInfoPtrs[model]` is set).
    pub fn model_exists(&self, model: ModelId) -> bool {
        (0..addr::MODEL_INFO_COUNT as i32).contains(&model.0)
            && self.view.read_u32(addr::MODEL_INFO_PTRS + 4 * model.0 as u32) != 0
    }

    /// Whether `model` is loaded (`CStreaming::ms_aInfoForModel[model].m_nLoadState == LOADSTATE_LOADED`).
    pub fn model_loaded(&self, model: ModelId) -> bool {
        (0..addr::STREAMING_MODELS as i32).contains(&model.0)
            && self.view.read_u8(addr::STREAMING_INFO + model.0 as u32 * addr::STREAMING_INFO_SIZE + 0x10) == 1
    }

    /// Requests `model` (`REQUEST_MODEL` 0247).
    pub fn request_model(&mut self, model: ModelId) -> GameResult<()> {
        self.run_op(op::REQUEST_MODEL, &[I(model.0)]).map(drop)
    }

    /// Marks `model` as no longer needed (`MARK_MODEL_AS_NO_LONGER_NEEDED` 0249).
    pub fn release_model(&mut self, model: ModelId) -> GameResult<()> {
        self.run_op(op::MARK_MODEL_AS_NO_LONGER_NEEDED, &[I(model.0)]).map(drop)
    }

    /// Loads every requested model now (`LOAD_ALL_MODELS_NOW` 038B).
    pub fn load_all_now(&mut self) -> GameResult<()> {
        self.run_op(op::LOAD_ALL_MODELS_NOW, &[]).map(drop)
    }

    /// Requests `model` and loads it now unless it is loaded; `NotLoaded` when it is still not loaded after.
    pub fn load_model_now(&mut self, model: ModelId) -> GameResult<()> {
        if self.model_loaded(model) {
            return Ok(());
        }
        self.request_model(model)?;
        self.load_all_now()?;
        if self.model_loaded(model) { Ok(()) } else { Err(GameError::NotLoaded) }
    }
}
