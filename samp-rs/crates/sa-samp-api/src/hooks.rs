//! Callbacks from the game into samp-rs. They replace samp's 103 hook rows (41 call-site hooks, 4 jmp
//! hooks, 18 indirect jmps, 1 call retarget, 39 vtable/pointer redirects) and samp-rs's 157 hook
//! addresses, including continuation addresses and return-address tests: every callback states its call
//! site in a typed enum instead.
//!
//! samp-rs implements [`GameHooks`]; the backend calls it. Every method has a default that keeps the
//! single-player behaviour, so samp-rs can move its hooks over one at a time. Mapping per callback:
//! `docs/samp/game-adapter.md` §5.
//!
//! Frame callbacks get `&mut dyn GameAdapter`. Mid-frame callbacks get `&dyn GameAdapter` (reads only);
//! whatever they must change is either in their typed context or queued by samp-rs for the next frame
//! callback.

use crate::input::{EventResult, PadState, WindowEvent};
use crate::script::{CustomOpcode, CustomOpcodeParams};
use crate::world::LineHit;
use crate::{
    EntityRef, GameAdapter, GameResult, ModelId, ObjectRef, PedRef, PlayerSlot, Rect2, Rgba, TaskRef, Vec3,
    VehicleRef, WEAPON_SKILL_STATS, WeaponType,
};

/// Whether the game runs its own code at a hooked site.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HookAction {
    /// Run the original code (single-player behaviour).
    #[default]
    RunOriginal,
    /// Skip it.
    Skip,
}

/// The fixed points of a frame where samp-rs runs, in order (`Idle` 0x53E920 → `CGame::Process`
/// 0x53BEE0 → `RenderScene` → `Render2dStuff` 0x53E230 → `Present`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FramePhase {
    /// Before `CGame::Process` (samp-rs `game_process_hook`, the retargeted call 0x53E981): frame timer,
    /// object pre-step (streaming, attachments, MoveObject).
    BeforeGameProcess,
    /// Inside `CGame::Process`, instead of `TheCamera.Process()` (call 0x53C104 → `CCamera::Process`
    /// 0x52B730): samp's camera follow runs first, then the game's camera.
    CameraProcess,
    /// After `CGame::Process` (same hook): first-frame ped flags, chat render target, model previews.
    AfterGameProcess,
    /// Instead of `Render2dStuff` (call 0x53EB12): the netgame tick, audio, textdraws, then the game's 2D.
    /// samp-rs's main per-frame entry (`frame_hook`).
    Before2d,
    /// After `Render2dStuff` (`ui::frame_after_2d`): textdraws over the HUD.
    After2d,
    /// Before the frame is presented (D3D proxy `Present`/`EndScene` today): samp's UI pass through
    /// [`crate::render::Overlay`].
    Present,
}

/// Where the engine asks for a per-player context (samp-rs `swap.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContextSite {
    /// `CPlayerPed::ProcessControl` (vtable slot 0x86D190 → 0x60EA90).
    PlayerPedProcessControl,
    /// `CPhysical::ProcessCollision` of a player ped (slot 0x86D194 → 0x54DFB0).
    PlayerPedProcessCollision,
    /// `CTaskSimpleUseGun::SetPedPosition` (slot 0x86D744 → 0x624ED0).
    UseGunSetPedPosition,
    /// A vehicle's `ProcessControl` with a remote driver (10 vtable slots +0x28).
    VehicleProcessControl,
}

/// The globals GTA's player code reads, as one value. samp swaps these in around a remote player's code:
/// `CPad::Pads[0]` (0xB73458), `CWorld::PlayerInFocus` (0xB7CD74), `Cams[0]` mode/FOV/looking/vectors
/// (0xB6F1A8, 0xB6F250, 0xB6F1A4/5, 0xB6F32C), the aspect ratio (0xC3EFA4), the player weapon mode
/// (0xB6F858) and the weapon skill stats (0xB79494). The engine applies it for the duration of the call
/// and restores the local player's values afterwards.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerContext {
    /// Player slot to treat as "in focus".
    pub slot: Option<PlayerSlot>,
    /// Pad new and old state.
    pub pad: (PadState, PadState),
    /// Camera front vector the aim code reads.
    pub cam_front: Vec3,
    /// Camera source.
    pub cam_source: Vec3,
    /// Camera up.
    pub cam_up: Vec3,
    /// Camera mode.
    pub cam_mode: u16,
    /// Field of view.
    pub fov: f32,
    /// Aspect ratio.
    pub aspect_ratio: f32,
    /// Looking left / right.
    pub looking: (bool, bool),
    /// Player weapon mode.
    pub weapon_mode: u16,
    /// Weapon skill stats.
    pub weapon_skills: [f32; WEAPON_SKILL_STATS],
    /// `m_fAimingRotation` stores in `PlayerControlZelda`/`Ducked` run (0x6884C4 / 0x688200).
    pub store_aim_rotation: bool,
    /// `CPed::ProcessControl` calls `CPed::UpdatePosition` (0x5E92F4).
    pub update_position: bool,
}

/// A vehicle's `ProcessControl` (10 vtable slots +0x28, 0x871148 … 0x872398).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VehicleControl {
    /// Context for the driver's code (remote driver), or `None` for the local player / no driver.
    pub context: Option<PlayerContext>,
    /// Run the game's `ProcessControl`.
    pub action: HookAction,
    /// Call `CAEVehicleAudioEntity::Service` afterwards (samp NOPs the game's own call).
    pub service_audio: bool,
}

/// Which call site fired (replaces the return-address tests and the per-site hook stubs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FireSite {
    /// `CWeapon::Fire` → `FireInstantHit` (0x742495, 0x7424EC, 0x742548).
    InstantHit(u8),
    /// `CWeapon::Fire` → `FireSniper` (0x7424CB).
    Sniper,
}

/// Which line-of-sight call in the weapon code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LosSite {
    /// `FireInstantHit` first call (0x740721).
    InstantHitA,
    /// `FireInstantHit` second call (0x740B69).
    InstantHitB,
    /// `CBulletInfo::Update` (0x736247).
    BulletUpdate,
}

/// A shot about to be fired.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    /// Shooter.
    pub shooter: PedRef,
    /// Weapon.
    pub weapon: WeaponType,
    /// Origin.
    pub origin: Vec3,
    /// Target point (may be changed by the hook for remote shooters).
    pub target: Vec3,
    /// Call site.
    pub site: FireSite,
}

/// A damage event (`CPedDamageResponseCalculator::ComputeDamageResponse` 0x4B5AC0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Damage {
    /// Victim.
    pub victim: PedRef,
    /// Attacker entity.
    pub attacker: Option<EntityRef>,
    /// Weapon.
    pub weapon: WeaponType,
    /// Body part.
    pub body_part: u8,
    /// Damage amount (may be changed).
    pub amount: f32,
}

/// A siren/horn call site (`CAEVehicleAudioEntity` and `CVehicle` sites 0x4F77DA, 0x501B1D, 0x501B42,
/// 0x501FC2, 0x502067, 0x5021AE, 0x6B2BCB, 0x6E0954).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SirenSite(pub u8);

/// A render pass samp draws objects in (`RenderScene` hooks 0x53DFDD/0x53E019, `RenderEverythingBarRoads`
/// 0x553AA0, `RenderFadingInEntities` 0x5531E0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderPass {
    /// Before the opaque world pass.
    BeforeWorld,
    /// After everything except roads.
    AfterEverythingBarRoads,
    /// After fading-in entities (samp's second object pass).
    AfterFadingIn,
}

/// Draws on the radar/map in the game's radar space (`CRadar::DrawAreaOnRadar` 0x5853D0).
pub trait RadarPainter {
    /// Fills a world-space rectangle.
    fn draw_area(&mut self, world: Rect2, colour: Rgba, in_menu_map: bool);
}

/// samp-rs's side of the interface: everything the game calls back. All methods default to the
/// single-player behaviour.
#[allow(unused_variables)]
pub trait GameHooks {
    // --- frame -------------------------------------------------------------------------------------
    /// A fixed frame point. All [`GameAdapter`] calls are allowed.
    fn frame(&mut self, phase: FramePhase, game: &mut dyn GameAdapter) {}
    /// The game is shutting down (`CGame::Shutdown` 0x53C900): disconnect, flush logs.
    fn shutdown(&mut self, game: &mut dyn GameAdapter) {}
    /// A window event before the game sees it (samp-rs `ui/wndproc.rs` subclass).
    fn window_event(&mut self, event: WindowEvent, game: &dyn GameAdapter) -> EventResult {
        EventResult::PassToGame
    }
    /// The graphics device was lost (`true`) or reset (`false`); samp frees/recreates its textures.
    fn device_lost(&mut self, lost: bool) {}
    /// Before `Idle`'s busy wait (0x53E930); samp sleeps 1 ms first.
    fn idle_wait(&mut self) {}

    // --- peds --------------------------------------------------------------------------------------
    /// Context for a player ped's code, `None` for the local player (no swap).
    fn player_context(
        &mut self,
        ped: PedRef,
        site: ContextSite,
        game: &dyn GameAdapter,
    ) -> Option<PlayerContext> {
        None
    }
    /// `CCivilianPed::ProcessControl` (slot 0x86C0D0): actors pinned by samp skip it.
    fn civilian_process_control(&mut self, ped: PedRef, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// After a ped is rendered (`CPed::Render` slots 0x86C0F0, 0x86C168, 0x86C248, 0x86C3A0, 0x86D1B0):
    /// samp draws attached objects and applies fog.
    fn ped_rendered(&mut self, ped: PedRef, game: &dyn GameAdapter) {}
    /// An animation is blended or added on a ped (`CAnimManager::BlendAnimation` 0x4D4610,
    /// `AddAnimation` 0x4D3AA0, `UncompressAnimation` 0x4D41C0): samp records the local player's animation
    /// for sync.
    fn animation_started(&mut self, ped: PedRef, group: u32, anim: u32, game: &dyn GameAdapter) {}
    /// A remote player's enter-car task was created (`CTaskComplexEnterCarAsDriver` ctor 0x6402F0, slot
    /// 0x6919BB); `Skip` refuses the entry.
    fn enter_vehicle_task(
        &mut self,
        ped: PedRef,
        vehicle: VehicleRef,
        task: TaskRef,
        game: &dyn GameAdapter,
    ) -> HookAction {
        HookAction::RunOriginal
    }
    /// A leave-car task was created (`CTaskComplexLeaveCar` ctor 0x63B8C0, slot 0x63B8BA).
    fn leave_vehicle_task(
        &mut self,
        ped: PedRef,
        vehicle: VehicleRef,
        task: TaskRef,
        game: &dyn GameAdapter,
    ) {
    }
    /// `CPlayerInfo::Process` enter/leave decisions (callers 0x5703FC, 0x5704A1, 0x570A20, 0x570A99).
    fn enter_refused(&mut self, ped: PedRef, vehicle: VehicleRef, game: &dyn GameAdapter) {}

    // --- combat ------------------------------------------------------------------------------------
    /// A shot is fired (`FireInstantHit` 0x73FB10 / `FireSniper` 0x73AAC0 via their `CWeapon::Fire` sites).
    fn fire(&mut self, shot: &mut Shot, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// A bullet is added (`CBulletInfo::AddBullet` 0x736010, site 0x73AC4B).
    fn add_bullet(&mut self, shot: &mut Shot, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// The weapon code's line test (`ProcessLineOfSight` at the [`LosSite`]s); samp replaces the hit with
    /// the synced one for remote shooters.
    fn bullet_line_of_sight(
        &mut self,
        site: LosSite,
        from: Vec3,
        to: Vec3,
        hit: &mut Option<LineHit>,
        game: &dyn GameAdapter,
    ) {
    }
    /// Damage to a ped; `amount` may be changed, `Skip` cancels.
    fn ped_damage(&mut self, damage: &mut Damage, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// `CEventDamage::AffectsPed` (0x4B35A0): override the result.
    fn damage_affects_ped(
        &mut self,
        victim: PedRef,
        attacker: Option<EntityRef>,
        game: &dyn GameAdapter,
    ) -> Option<bool> {
        None
    }
    /// `CVehicle::InflictDamage` (0x6D7C90).
    fn vehicle_damage(
        &mut self,
        vehicle: VehicleRef,
        attacker: Option<EntityRef>,
        weapon: WeaponType,
        amount: f32,
        game: &dyn GameAdapter,
    ) -> HookAction {
        HookAction::RunOriginal
    }
    /// Detonator use (`CProjectileInfo::RemoveDetonatorProjectiles` 0x738860).
    fn detonate(&mut self, owner: PedRef, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// Satchel update (`CProjectileInfo::Update` site 0x738F3A).
    fn satchel_update(&mut self, owner: Option<PedRef>, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// Camera photo (`CWeapon::TakePhotograph` 0x73C252).
    fn take_photo(&mut self, ped: PedRef, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// Weapon skill for a ped (`CPed::GetWeaponSkill` 0x5E6580, `RenderWeaponPedsForPC` 0x7330A2).
    fn weapon_skill(&mut self, ped: PedRef, weapon: WeaponType, game: &dyn GameAdapter) -> Option<u8> {
        None
    }
    /// Camera shake request (`CCamera::CamShakeNoPos` 0x50A970, sniper site 0x73ACE2).
    fn camera_shake(&mut self, strength: f32, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// Crosshair drawing (`CHud::DrawCrossHairs` 0x58E020 from `CHud::Draw` 0x58FBBF).
    fn draw_crosshair(&mut self, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }

    // --- vehicles ----------------------------------------------------------------------------------
    /// Before a vehicle's `ProcessControl`.
    fn vehicle_process_control(&mut self, vehicle: VehicleRef, game: &dyn GameAdapter) -> VehicleControl {
        VehicleControl::default()
    }
    /// `ProcessEntityCollision` of a vehicle (8 classes; slots 0x871178 …): `false` = no collision.
    fn vehicle_collision(&mut self, vehicle: VehicleRef, other: EntityRef, game: &dyn GameAdapter) -> bool {
        true
    }
    /// `CAutomobile::BreakTowLink` (0x6A4400, 5 slots).
    fn break_tow_link(&mut self, vehicle: VehicleRef, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// `CVehicle::UsesSiren` (0x6D8470) and the siren/horn sites: override the answer.
    fn uses_siren(
        &mut self,
        vehicle: VehicleRef,
        site: Option<SirenSite>,
        game: &dyn GameAdapter,
    ) -> Option<bool> {
        None
    }
    /// Train derail (site 0x6F8CF8): samp clamps remote trains.
    fn train_derail(&mut self, train: VehicleRef, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// Vehicle render (`SetDirtTextures` 0x5D5DB0 via 0x6D0E7E): override the dirt level.
    fn vehicle_dirt(&mut self, vehicle: VehicleRef, game: &dyn GameAdapter) -> Option<f32> {
        None
    }
    /// `FindPlayerSpeed` (0x56E090) as seen by the radar (passenger radar, 0x586C0A).
    fn player_speed(&mut self, slot: PlayerSlot, game: &dyn GameAdapter) -> Option<Vec3> {
        None
    }
    /// The ped the police scanner reports (0x4E7427 → `FindPlayerPed`).
    fn police_scanner_ped(&mut self, game: &dyn GameAdapter) -> Option<PedRef> {
        None
    }

    // --- world -------------------------------------------------------------------------------------
    /// A samp render pass for objects.
    fn render_pass(&mut self, pass: RenderPass, game: &dyn GameAdapter) {}
    /// An object is rendered (`CObject::Render` 0x59F180, slot 0x866FA8); materials are applied by the
    /// backend from [`crate::objects::Objects::set_object_material`].
    fn object_render(&mut self, object: ObjectRef, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// An object got its RW object (`CObject::CreateRwObject` 0x59F110, slot 0x866F7C).
    fn object_streamed_in(&mut self, object: ObjectRef, game: &dyn GameAdapter) {}
    /// An entity lost its RW object (`CEntity::DeleteRwObject` 0x534030, slots 0x866F80, 0x8585E8).
    fn entity_streamed_out(&mut self, entity: EntityRef, game: &dyn GameAdapter) {}
    /// A world instance is being loaded from an IPL (`CFileLoader::LoadObjectInstance` 0x538090):
    /// `false` drops it (RemoveBuildingForPlayer replay).
    fn load_instance(&mut self, model: ModelId, pos: Vec3, game: &dyn GameAdapter) -> bool {
        true
    }
    /// Entity is added to the render list (`CRenderer::AddEntityToRenderList` 0x5534B0).
    fn add_to_render_list(&mut self, entity: EntityRef, game: &dyn GameAdapter) -> bool {
        true
    }
    /// Entity render effects (`CEntity::RenderEffects` site 0x5342F9).
    fn render_effects(&mut self, entity: EntityRef, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// Attached entities were positioned (`CWorld::ProcessAttachedEntities` site 0x5648D3).
    fn attached_entities_done(&mut self, game: &dyn GameAdapter) {}
    /// Vertical line test (`CWorld::ProcessVerticalLine` 0x5674E0): `false` ignores `entity`.
    fn vertical_line_entity(&mut self, entity: EntityRef, game: &dyn GameAdapter) -> bool {
        true
    }
    /// Object break (`BreakObject_c::Init`, caller 0x59E8F9): `false` keeps the object whole.
    fn object_break(&mut self, object: ObjectRef, game: &dyn GameAdapter) -> bool {
        true
    }
    /// Camera collision with an entity (`CheckCameraCollisionObjects` 0x41AB78, `BuildCacheOfCameraCollision`
    /// 0x41AF80, `CameraConeCastVsWorldCollision` 0x41B02E): `false` ignores it.
    fn camera_collides(&mut self, entity: EntityRef, game: &dyn GameAdapter) -> bool {
        true
    }
    /// Escalators update (`CEscalators::Update` 0x718599).
    fn escalators(&mut self, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// A pickup is collected (`CPickup::Update` site 0x4579C6): samp reports it to the server; `Skip`
    /// keeps the game from giving it.
    fn pickup_collected(&mut self, pickup_index: u16, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }
    /// Radar blip colour (`CRadar::GetRadarTraceColour` 0x584770).
    fn radar_trace_colour(&mut self, colour: u32, game: &dyn GameAdapter) -> Option<u32> {
        None
    }
    /// Draws gang zones on the radar (`DrawRadarGangOverlay` sites 0x5869BF, 0x584A79) or the menu map
    /// (`CMenuManager::PrintMap` 0x5759E4).
    fn radar_overlay(&mut self, painter: &mut dyn RadarPainter, in_menu_map: bool, game: &dyn GameAdapter) {}
    /// Radar drawing (`CHud::DrawRadar` 0x58A330 from 0x58FC53); samp applies its HUD scale fix.
    fn draw_radar(&mut self, game: &dyn GameAdapter) -> HookAction {
        HookAction::RunOriginal
    }

    // --- text and script ---------------------------------------------------------------------------
    /// GXT lookup before the game's table (`CText::Get` 0x6A0050): samp's `SAMP…` menu keys.
    fn gxt(&self, key: &str) -> Option<&str> {
        None
    }
    /// A typed cheat character (`CCheat::AddToCheatString` 0x438576).
    fn cheat_char(&mut self, ch: u8) -> HookAction {
        HookAction::RunOriginal
    }
    /// samp's custom opcodes, run by samp's own `main.scm`.
    fn custom_opcode(
        &mut self,
        op: CustomOpcode,
        params: &mut dyn CustomOpcodeParams,
        game: &dyn GameAdapter,
    ) -> GameResult<()> {
        Ok(())
    }
}
