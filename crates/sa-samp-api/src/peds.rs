//! Peds: the local player, samp's remote players (real `CPlayerPed`s in slots 2..=209) and actors.
//!
//! samp-rs today: `game.rs`, `client/*.rs`, `players.rs`, `remote.rs`, `swap.rs`, `combatclient.rs`,
//! `specials.rs`, `pedanims.rs`, `behaviourclient/*.rs`. Address mapping per method:
//! `docs/samp/game-adapter.md` §4.1.

use crate::{
    AreaCode, GameResult, Matrix, ModelId, PedRef, PlayerSlot, ScriptHandle, TaskRef, Vec3, VehicleRef,
    WeaponType,
};

/// One weapon slot (`CWeapon`, 0x1C bytes at `CPed::m_aWeapons` +0x5A0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WeaponSlot {
    /// Weapon type.
    pub weapon: u8,
    /// `CWeapon::m_nState` (+0x4).
    pub state: u32,
    /// Ammo in clip.
    pub ammo_in_clip: u32,
    /// Total ammo.
    pub ammo_total: u32,
}

/// What samp's on-foot sync reads every frame (samp-rs `footsync::LocalSample`, `game::PedState`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PedSnapshot {
    /// Transform (`CPlaceable::m_matrix` +0x14).
    pub matrix: Matrix,
    /// Move speed (`CPhysical::m_vecMoveSpeed` +0x44).
    pub move_speed: Vec3,
    /// Health (+0x540).
    pub health: f32,
    /// Armour (+0x548).
    pub armour: f32,
    /// Current heading in radians (+0x558).
    pub heading: f32,
    /// Heading goal (+0x55C).
    pub heading_goal: f32,
    /// `ePedState` (+0x530).
    pub ped_state: u32,
    /// Model index (`CEntity::m_nModelIndex` +0x22).
    pub model: ModelId,
    /// Selected weapon slot (+0x718).
    pub weapon_slot: u8,
    /// Current weapon type.
    pub weapon: WeaponType,
    /// Vehicle the ped is in (`CPed::m_pVehicle` +0x58C, valid when the in-vehicle flag is set).
    pub vehicle: Option<VehicleRef>,
    /// Area code the ped is in.
    pub area: AreaCode,
    /// Ped flag words (first two dwords of `CPed` flags; in-vehicle, ducking, ...), raw.
    pub flags: [u32; 2],
    /// Ped is in the world (not a deleted placeable, in the moving list) — samp's "ped is live" test.
    pub in_world: bool,
}

/// Bone ids used by samp (`ePedBones`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BoneId(pub u16);

/// Ped proofs (`SET_CHAR_PROOFS` 02AB).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Proofs {
    /// Bullet proof.
    pub bullet: bool,
    /// Fire proof.
    pub fire: bool,
    /// Explosion proof.
    pub explosion: bool,
    /// Collision proof.
    pub collision: bool,
    /// Melee proof.
    pub melee: bool,
}

/// An animation request (samp's ApplyAnimation, RPC 86; `TASK_PLAY_ANIM_NON_INTERRUPTABLE` 0812 and the
/// direct `CAnimManager::BlendAnimation` path).
#[derive(Clone, Debug, PartialEq)]
pub struct AnimRequest<'a> {
    /// Animation name.
    pub name: &'a str,
    /// Animation block (IFP) name.
    pub block: &'a str,
    /// Blend delta.
    pub delta: f32,
    /// Loop.
    pub looped: bool,
    /// Lock X.
    pub lock_x: bool,
    /// Lock Y.
    pub lock_y: bool,
    /// Freeze on the last frame.
    pub freeze: bool,
    /// Time in ms (0 = until done).
    pub time_ms: i32,
}

/// Ped tasks samp starts by script command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PedTask {
    /// `TASK_ENTER_CAR_AS_DRIVER` (05CB).
    EnterCarAsDriver {
        /// Vehicle.
        vehicle: VehicleRef,
        /// Time limit in ms.
        time_ms: i32,
    },
    /// `TASK_ENTER_CAR_AS_PASSENGER` (05CA).
    EnterCarAsPassenger {
        /// Vehicle.
        vehicle: VehicleRef,
        /// Time limit in ms.
        time_ms: i32,
        /// Seat index (−1 = any).
        seat: i32,
    },
    /// `TASK_LEAVE_CAR` (05CD).
    LeaveCar {
        /// Vehicle.
        vehicle: VehicleRef,
    },
    /// `TASK_HANDS_UP` (05C4).
    HandsUp {
        /// Duration in ms.
        time_ms: i32,
    },
    /// `TASK_USE_MOBILE_PHONE` (0729).
    UseMobilePhone(bool),
    /// `TASK_DRIVE_BY` (0713), passenger drive-by.
    DriveBy {
        /// Target point.
        target: Vec3,
        /// Radius.
        radius: f32,
    },
    /// `TASK_PICK_UP_OBJECT` (070A), held items.
    PickUpObject {
        /// Object handle.
        object: ScriptHandle,
        /// Bone attach offset.
        offset: Vec3,
        /// Bone id.
        bone: BoneId,
    },
    /// `GIVE_MELEE_ATTACK_TO_CHAR` (07FE).
    MeleeAttack {
        /// Attack kind.
        kind: i32,
        /// Combo level.
        level: i32,
    },
    /// Secondary duck task (`CPedIntelligence::SetTaskDuckSecondary` 0x601230).
    DuckSecondary(u16),
}

/// Ped operations. Reads take `&self` (allowed in mid-frame hooks), writes `&mut self`.
pub trait Peds {
    // --- lookup -------------------------------------------------------------------------------------
    /// The local player's ped (`FindPlayerPed` 0x56E210 / `CWorld::Players[0].m_pPed` 0xB7CD98).
    fn local_player(&self) -> Option<PedRef>;
    /// The ped in a player slot (`CWorld::Players[slot].m_pPed`).
    fn player_ped(&self, slot: PlayerSlot) -> Option<PedRef>;
    /// `CWorld::PlayerInFocus` (0xB7CD74).
    fn player_in_focus(&self) -> PlayerSlot;
    /// Script handle of a ped (`CPools::GetPedRef`, ped pool 0xB74490).
    fn ped_script_handle(&self, ped: PedRef) -> GameResult<ScriptHandle>;
    /// Ped from a script handle (`CPools::GetPed`).
    fn ped_from_script_handle(&self, h: ScriptHandle) -> Option<PedRef>;
    /// samp's "ped is live" test (vtable not `CPlaceable` 0x863C40, moving list +0xB4 set).
    fn ped_in_world(&self, ped: PedRef) -> bool;
    /// Whether `model` is a ped model (model-info vtable 0x85BDC0).
    fn is_ped_model(&self, model: ModelId) -> bool;

    // --- lifecycle ----------------------------------------------------------------------------------
    /// Creates a remote player ped in `slot` (`CREATE_PLAYER` 0053, `GET_PLAYER_CHAR` 01F5, then the
    /// initial-state call `CPlayerPed::SetInitialState` 0x60CD20 and `CAEPedSpeechAudioEntity::Initialise`
    /// 0x4E68D0).
    fn create_player_ped(&mut self, slot: PlayerSlot, model: ModelId, pos: Vec3) -> GameResult<PedRef>;
    /// Creates an actor (`CREATE_CHAR` 009A, ped type 4).
    fn create_actor(&mut self, model: ModelId, pos: Vec3, heading_deg: f32) -> GameResult<PedRef>;
    /// Deletes a ped created by samp (samp's ped destruction; clears `CWorld::Players[slot].m_pPed`).
    fn destroy_ped(&mut self, ped: PedRef) -> GameResult<()>;
    /// Changes the model (samp's skin change: `CClothes::RebuildPlayer` gated, `BUILD_PLAYER_MODEL` 070D for CJ).
    fn set_ped_model(&mut self, ped: PedRef, model: ModelId) -> GameResult<()>;

    // --- state reads --------------------------------------------------------------------------------
    /// Everything on-foot sync reads, in one call.
    fn ped_snapshot(&self, ped: PedRef) -> GameResult<PedSnapshot>;
    /// Weapon slots (13).
    fn ped_weapons(&self, ped: PedRef) -> GameResult<[WeaponSlot; 13]>;
    /// Bone position (`CPed::GetBonePosition` 0x5E4280; `transformed` = `GetTransformedBonePosition` 0x5E01C0).
    fn bone_position(&self, ped: PedRef, bone: BoneId, transformed: bool) -> GameResult<Vec3>;
    /// Current weapon skill level (`CPed::GetWeaponSkill` 0x5E6580).
    fn weapon_skill(&self, ped: PedRef) -> GameResult<u8>;
    /// Fire attached to the ped (`CPed::m_pFire` +0x730).
    fn ped_on_fire(&self, ped: PedRef) -> GameResult<bool>;
    /// Whether an animation is playing on the ped (`CAnimManager::GetAnimAssociation` 0x4D3A60).
    fn anim_playing(&self, ped: PedRef, block: &str, name: &str) -> GameResult<bool>;
    /// Local player money (`CWorld::Players[0] + 0xB8` 0xB7CE50).
    fn player_money(&self) -> i32;

    // --- state writes -------------------------------------------------------------------------------
    /// Sets position, keeping rotation (teleport).
    fn set_ped_position(&mut self, ped: PedRef, pos: Vec3) -> GameResult<()>;
    /// Writes the full matrix (remote sync).
    fn set_ped_matrix(&mut self, ped: PedRef, m: &Matrix) -> GameResult<()>;
    /// Writes move speed.
    fn set_ped_move_speed(&mut self, ped: PedRef, v: Vec3) -> GameResult<()>;
    /// Sets heading in degrees (`SET_CHAR_HEADING` 0173), or the raw current/goal radians when `raw`.
    fn set_ped_heading(&mut self, ped: PedRef, heading: f32, raw: bool) -> GameResult<()>;
    /// Sets health.
    fn set_ped_health(&mut self, ped: PedRef, health: f32) -> GameResult<()>;
    /// Sets armour.
    fn set_ped_armour(&mut self, ped: PedRef, armour: f32) -> GameResult<()>;
    /// Gives a weapon (`CPed::GiveWeapon` 0x5E6080).
    fn give_weapon(&mut self, ped: PedRef, weapon: WeaponType, ammo: u32) -> GameResult<()>;
    /// Selects the current weapon (`CPed::SetCurrentWeapon` 0x5E6280 / `SET_CURRENT_CHAR_WEAPON` 01B9).
    fn set_current_weapon(&mut self, ped: PedRef, weapon: WeaponType) -> GameResult<()>;
    /// Removes all weapons (`CPed::ClearWeapons` 0x5E6320).
    fn clear_weapons(&mut self, ped: PedRef) -> GameResult<()>;
    /// Writes a weapon slot's state and ammo directly (remote sync).
    fn set_weapon_slot(&mut self, ped: PedRef, slot: u8, value: WeaponSlot) -> GameResult<()>;
    /// Removes the weapon when entering a vehicle (`CPed::RemoveWeaponWhenEnteringVehicle` 0x5E6370).
    fn remove_weapon_entering_vehicle(&mut self, ped: PedRef) -> GameResult<()>;
    /// Sets ped proofs (`SET_CHAR_PROOFS` 02AB).
    fn set_proofs(&mut self, ped: PedRef, proofs: Proofs) -> GameResult<()>;
    /// `SET_CHAR_SUFFERS_CRITICAL_HITS` (0446).
    fn set_suffers_critical_hits(&mut self, ped: PedRef, on: bool) -> GameResult<()>;
    /// `SET_CHAR_DROPS_WEAPONS_WHEN_DEAD` (087E).
    fn set_drops_weapons_when_dead(&mut self, ped: PedRef, on: bool) -> GameResult<()>;
    /// `FREEZE_CHAR_POSITION` (04D7).
    fn freeze_ped(&mut self, ped: PedRef, frozen: bool) -> GameResult<()>;
    /// `SET_CHAR_AREA_VISIBLE` (0860).
    fn set_ped_area(&mut self, ped: PedRef, area: AreaCode) -> GameResult<()>;
    /// `SET_CHAR_DECISION_MAKER` (060B).
    fn set_decision_maker(&mut self, ped: PedRef, maker: i32) -> GameResult<()>;
    /// `SET_CHAR_MONEY` (03FE) / money write.
    fn set_player_money(&mut self, money: i32) -> GameResult<()>;
    /// `SET_PLAYER_CONTROL` (01B4).
    fn set_player_control(&mut self, enabled: bool) -> GameResult<()>;
    /// `SET_PLAYER_DRUNKENNESS` (052C).
    fn set_drunkenness(&mut self, level: i32) -> GameResult<()>;
    /// Writes ped flag words (the first-frame ped flags and per-frame bits samp sets).
    fn set_ped_flags(&mut self, ped: PedRef, mask: [u32; 2], value: [u32; 2]) -> GameResult<()>;
    /// Extinguishes the ped (`CFire::Extinguish` 0x5393F0 on `m_pFire`).
    fn extinguish_ped(&mut self, ped: PedRef) -> GameResult<()>;
    /// `EXPLODE_CHAR_HEAD` (0321).
    fn explode_head(&mut self, ped: PedRef) -> GameResult<()>;

    // --- vehicles -----------------------------------------------------------------------------------
    /// `WARP_CHAR_INTO_CAR` (036A) or `WARP_CHAR_INTO_CAR_AS_PASSENGER` (0430) when `seat` is `Some`.
    fn warp_into_vehicle(&mut self, ped: PedRef, vehicle: VehicleRef, seat: Option<i32>) -> GameResult<()>;
    /// `WARP_CHAR_FROM_CAR_TO_COORD` (0362).
    fn warp_out_of_vehicle(&mut self, ped: PedRef, pos: Vec3) -> GameResult<()>;

    // --- tasks and animation -------------------------------------------------------------------------
    /// Starts a task.
    fn start_task(&mut self, ped: PedRef, task: PedTask) -> GameResult<()>;
    /// Drops all tasks now (`CPedIntelligence::FlushImmediately` 0x601640).
    fn flush_tasks(&mut self, ped: PedRef) -> GameResult<()>;
    /// Deletes a task samp created and kept (the enter/leave task hooks; `CTask::operator delete` 0x61A5B0,
    /// the deleting destructors 0x6275E0, 0x6801D0, 0x639330).
    fn delete_task(&mut self, task: TaskRef) -> GameResult<()>;
    /// Plays an animation.
    fn play_anim(&mut self, ped: PedRef, anim: &AnimRequest<'_>) -> GameResult<()>;
    /// Stops animations / returns to idle.
    fn stop_anims(&mut self, ped: PedRef) -> GameResult<()>;
    /// Head look-at (`IKChainManager_c::LookAt` 0x618970 on `g_ikChainMan` 0xC15448).
    fn look_at(&mut self, ped: PedRef, target: Vec3, time_ms: i32) -> GameResult<()>;
    /// Starts or stops the jetpack (task vtable 0x8705C4 test / deleting destructor 0x6801D0).
    fn set_jetpack(&mut self, ped: PedRef, on: bool) -> GameResult<()>;
    /// Whether the ped uses a jetpack.
    fn has_jetpack(&self, ped: PedRef) -> GameResult<bool>;
}
