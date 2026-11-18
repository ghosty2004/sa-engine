//! samp-rs's game adapter (`sa_samp_api::GameAdapter`) over the mod API: what the client reads comes from the
//! state queries, what it changes goes through the mod API's changes, entities and the game's script commands.
//! What the mod API does not offer is `Unsupported`, as on any backend that lacks a feature.

use sa_mod::script::Arg;
use sa_mod::{Error, audio, entities, game, script, state, world};
use sa_samp_api::audio::*;
use sa_samp_api::camera::*;
use sa_samp_api::config::*;
use sa_samp_api::hud::*;
use sa_samp_api::input::*;
use sa_samp_api::objects::*;
use sa_samp_api::peds::*;
use sa_samp_api::render::*;
use sa_samp_api::script::*;
use sa_samp_api::streaming::*;
use sa_samp_api::vehicles::*;
use sa_samp_api::world::*;
use sa_samp_api::*;

/// The ped models: the skins `SetPlayerSkin` may name.
const PED_MODELS: std::ops::RangeInclusive<i32> = 0..=311;

/// The game as the mod API shows it.
#[derive(Default)]
pub struct ModGame {
    config: SampModeConfig,
    /// What the UI draws through; [`crate::draw_ui`] turns it into overlay quads.
    pub ui: samp_overlay::QuadUi,
}


/// A mod API error as the adapter's.
fn failed(what: &'static str) -> impl Fn(Error) -> GameError {
    move |e| match e {
        Error::NotFound => GameError::InvalidHandle,
        e => GameError::Other(format!("{what}: {e:?}")),
    }
}

/// Runs a script command for its effect.
fn cmd(opcode: u16, args: &[Arg<'_>]) -> GameResult<()> {
    script::run(opcode, args).map(drop).map_err(failed("script command"))
}

/// Runs a script command for its condition.
fn cond(opcode: u16, args: &[Arg<'_>]) -> GameResult<bool> {
    script::run(opcode, args).map(|o| o.condition).map_err(failed("script command"))
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x)
}

impl GameAdapter for ModGame {
    fn backend(&self) -> BackendInfo {
        BackendInfo {
            kind: BackendKind::EngineNative,
            api_version: API_VERSION,
            build: format!("sa-engine mod API {}.{}", sa_mod::API_MAJOR, sa_mod::API_MINOR),
            has_original_layout: false,
        }
    }

    fn game_state(&self) -> GameState {
        state::game_state().unwrap_or(GameState::from_raw(0))
    }

    fn time_step(&self) -> TimeStep {
        let step = state::world().map_or(0.0, |w| w.time_step);
        TimeStep { clipped: step, non_clipped: step }
    }

    fn game_time_ms(&self) -> u32 {
        state::world().map_or(0, |w| w.time_ms)
    }

    fn set_game_time_ms(&mut self, ms: u32) -> GameResult<()> {
        sa_mod::world::set_game_time(ms).map_err(failed("set_game_time"))
    }

    fn request_quit(&mut self) {}

    fn ui(&mut self) -> Option<&mut dyn sa_samp_api::ui::UiBackend> {
        Some(&mut self.ui)
    }
}

impl GameAudio for ModGame {
    fn radio_volume(&self) -> u8 { Default::default() }
    fn stop_radio(&mut self) -> GameResult<()> { Err(GameError::Unsupported("GameAudio::stop_radio")) }
    fn start_radio(&mut self, _station: i8) -> GameResult<()> { Err(GameError::Unsupported("GameAudio::start_radio")) }
    fn mission_ambience(&mut self, _track: Option<i32>) -> GameResult<()> { Err(GameError::Unsupported("GameAudio::mission_ambience")) }
    fn one_off_sound(&mut self, pos: Vec3, sound: i32) -> GameResult<()> {
        audio::at(sound, pos).map_err(failed("one_off_sound"))
    }
    fn set_weather_sound_muted(&mut self, _muted: bool) -> GameResult<()> { Err(GameError::Unsupported("GameAudio::set_weather_sound_muted")) }
}

impl Camera for ModGame {
    fn camera(&self) -> CameraState {
        let Some(c) = state::camera() else { return CameraState::default() };
        let right = cross(c.front, c.up);
        CameraState {
            matrix: Matrix { right, forward: c.front, up: c.up, pos: c.pos },
            front: c.front,
            fov: c.fov,
            mode: c.mode,
            aspect_ratio: c.aspect_ratio,
            ..CameraState::default()
        }
    }
    fn world_to_screen(&self, p: Vec3) -> Option<(Point2, f32)> {
        let (x, y, depth) = state::world_to_screen(p)?;
        Some((Point2 { x, y }, depth))
    }
    fn sphere_visible(&self, _center: Vec3, _radius: f32) -> bool { false }
    fn set_fixed_position(&mut self, pos: Vec3, rot: Vec3) -> GameResult<()> {
        let f = Arg::Float;
        cmd(0x015F, &[f(pos.x), f(pos.y), f(pos.z), f(rot.x), f(rot.y), f(rot.z)])
    }
    fn point_at(&mut self, target: Vec3, cut: CameraCut) -> GameResult<()> {
        let mode = match cut {
            CameraCut::Cut => 2,
            CameraCut::Smooth => 1,
        };
        cmd(0x0160, &[Arg::Float(target.x), Arg::Float(target.y), Arg::Float(target.z), Arg::Int(mode)])
    }
    fn point_at_ped(&mut self, _ped: PedRef, _mode: i32, _cut: CameraCut) -> GameResult<()> { Err(GameError::Unsupported("Camera::point_at_ped")) }
    fn point_at_vehicle(&mut self, _vehicle: VehicleRef, _mode: i32, _cut: CameraCut) -> GameResult<()> { Err(GameError::Unsupported("Camera::point_at_vehicle")) }
    fn behind_player(&mut self) -> GameResult<()> {
        cmd(0x0373, &[])
    }
    fn restore(&mut self) -> GameResult<()> {
        cmd(0x02EB, &[])
    }
    fn interpolate( &mut self, _look_at: bool, _from: Vec3, _to: Vec3, _time_ms: i32, _cut: CameraCut, ) -> GameResult<()> { Err(GameError::Unsupported("Camera::interpolate")) }
    fn set_widescreen(&mut self, on: bool) -> GameResult<()> {
        cmd(0x02A3, &[Arg::Int(i32::from(on))])
    }
    fn shake(&mut self, _strength: f32) -> GameResult<()> { Err(GameError::Unsupported("Camera::shake")) }
    fn set_fov(&mut self, _fov: f32) -> GameResult<()> { Err(GameError::Unsupported("Camera::set_fov")) }
}

impl SampMode for ModGame {
    fn config(&self) -> &SampModeConfig {
        &self.config
    }
    fn set_toggle(&mut self, toggle: RuntimeToggle) -> GameResult<()> {
        let (option, on) = match toggle {
            RuntimeToggle::TypedCheats(on) => (game::GameOption::TypedCheats, on),
            RuntimeToggle::GameClock(on) => (game::GameOption::Clock, on),
            _ => return Err(GameError::Unsupported("SampMode::set_toggle")),
        };
        game::set_option(option, on).map_err(failed("set_toggle"))
    }
}

impl Hud for ModGame {
    fn set_hud_visible(&mut self, on: bool) -> GameResult<()> {
        game::set_hud_visible(on).map_err(failed("set_hud_visible"))
    }
    fn menu_active(&self) -> bool {
        state::menu_active()
    }
    fn menu_active_dword(&self) -> u32 { Default::default() }
    fn big_message(&mut self, text: &str, time_ms: u32, style: u16) -> GameResult<()> {
        game::big_message(text, time_ms, style as u8).map_err(failed("big_message"))
    }
    fn clear_prints(&mut self) -> GameResult<()> {
        cmd(0x00BE, &[])
    }
    fn set_zone_names(&mut self, on: bool) -> GameResult<()> {
        cmd(0x09BA, &[Arg::Int(i32::from(on))])
    }
    fn add_score(&mut self, delta: i32) -> GameResult<()> {
        cmd(0x0109, &[Arg::Int(0), Arg::Int(delta)])
    }
    fn expand_game_text(&self, _text: &str, _numbers: &[i32]) -> String { Default::default() }
    fn gxt(&self, _key: &str) -> Option<String> { None }
}

impl Text2d for ModGame {
    fn print(&mut self, _style: &FontStyle, _at: Point2, _text: &str) -> GameResult<()> { Err(GameError::Unsupported("Text2d::print")) }
    fn text_width(&self, _style: &FontStyle, _text: &str) -> f32 { Default::default() }
    fn load_txd(&mut self, _name: &str, _path: Option<&str>) -> GameResult<TxdSlot> { Err(GameError::Unsupported("Text2d::load_txd")) }
    fn sprite(&mut self, _txd: TxdSlot, _texture: &str) -> GameResult<TextureRef> { Err(GameError::Unsupported("Text2d::sprite")) }
    fn draw_sprite(&mut self, _sprite: TextureRef, _rect: Rect2, _colour: Rgba) -> GameResult<()> { Err(GameError::Unsupported("Text2d::draw_sprite")) }
    fn release_sprite(&mut self, _sprite: TextureRef) -> GameResult<()> { Err(GameError::Unsupported("Text2d::release_sprite")) }
}

impl Menus for ModGame {
    fn show_menu(&mut self, _menu: &MenuSpec) -> GameResult<MenuRef> { Err(GameError::Unsupported("Menus::show_menu")) }
    fn menu_selection(&self, _menu: MenuRef) -> GameResult<i32> { Err(GameError::Unsupported("Menus::menu_selection")) }
    fn delete_menu(&mut self, _menu: MenuRef) -> GameResult<()> { Err(GameError::Unsupported("Menus::delete_menu")) }
}

impl Input for ModGame {
    fn pad(&self) -> (PadState, PadState) {
        (state::pad().map(PadState::from).unwrap_or_default(), PadState::default())
    }
    fn set_pad(&mut self, _new: PadState) -> GameResult<()> { Err(GameError::Unsupported("Input::set_pad")) }
    fn game_key_down(&self, _key_offset: u16) -> bool { false }
    fn key_held(&self, vk: u16) -> bool {
        u8::try_from(vk).is_ok_and(state::key_down)
    }
    fn mouse(&self) -> MouseState { Default::default() }
    fn clear_mouse_history(&mut self) -> GameResult<()> { Err(GameError::Unsupported("Input::clear_mouse_history")) }
    fn update_pads(&mut self) -> GameResult<()> { Err(GameError::Unsupported("Input::update_pads")) }
    fn set_pad_counter(&mut self, _v: u8) -> GameResult<()> { Err(GameError::Unsupported("Input::set_pad_counter")) }
    fn set_cursor_visible(&mut self, _visible: bool) -> GameResult<()> { Err(GameError::Unsupported("Input::set_cursor_visible")) }
    fn screen_size(&self) -> (u32, u32) { Default::default() }
    fn native_window(&self) -> usize { Default::default() }
}

impl Objects for ModGame {
    fn object_script_handle(&self, _o: ObjectRef) -> GameResult<ScriptHandle> { Err(GameError::Unsupported("Objects::object_script_handle")) }
    fn object_from_script_handle(&self, _h: ScriptHandle) -> Option<ObjectRef> { None }
    fn object_matrix(&self, _o: ObjectRef) -> GameResult<Matrix> { Err(GameError::Unsupported("Objects::object_matrix")) }
    fn object_has_rw(&self, _o: ObjectRef) -> GameResult<bool> { Err(GameError::Unsupported("Objects::object_has_rw")) }
    fn object_attached(&self, _o: ObjectRef) -> GameResult<bool> { Err(GameError::Unsupported("Objects::object_attached")) }
    fn object_bounds(&self, _o: ObjectRef) -> GameResult<(Vec3, f32)> { Err(GameError::Unsupported("Objects::object_bounds")) }
    fn object_ray_test(&self, _o: ObjectRef, _from: Vec3, _to: Vec3) -> GameResult<Option<ObjectHit>> { Err(GameError::Unsupported("Objects::object_ray_test")) }
    fn create_object(&mut self, _model: ModelId, _pos: Vec3) -> GameResult<ObjectRef> { Err(GameError::Unsupported("Objects::create_object")) }
    fn destroy_object(&mut self, _o: ObjectRef, _fade: bool) -> GameResult<()> { Err(GameError::Unsupported("Objects::destroy_object")) }
    fn set_object_transform(&mut self, _o: ObjectRef, _pos: Vec3, _rot_deg: Vec3) -> GameResult<()> { Err(GameError::Unsupported("Objects::set_object_transform")) }
    fn set_object_matrix(&mut self, _o: ObjectRef, _m: &Matrix) -> GameResult<()> { Err(GameError::Unsupported("Objects::set_object_matrix")) }
    fn set_object_scale(&mut self, _o: ObjectRef, _scale: f32) -> GameResult<()> { Err(GameError::Unsupported("Objects::set_object_scale")) }
    fn set_object_visible(&mut self, _o: ObjectRef, _visible: bool) -> GameResult<()> { Err(GameError::Unsupported("Objects::set_object_visible")) }
    fn set_object_collision(&mut self, _o: ObjectRef, _on: bool) -> GameResult<()> { Err(GameError::Unsupported("Objects::set_object_collision")) }
    fn attach_object( &mut self, _o: ObjectRef, _to: AttachTarget, _offset: Vec3, _rot_deg: Vec3, ) -> GameResult<()> { Err(GameError::Unsupported("Objects::attach_object")) }
    fn detach_object(&mut self, _o: ObjectRef) -> GameResult<()> { Err(GameError::Unsupported("Objects::detach_object")) }
    fn play_object_anim( &mut self, _o: ObjectRef, _anim: &str, _block: &str, _delta: f32, _looped: bool, ) -> GameResult<()> { Err(GameError::Unsupported("Objects::play_object_anim")) }
    fn set_object_material(&mut self, _o: ObjectRef, _index: u8, _m: Option<MaterialOverride>) -> GameResult<()> { Err(GameError::Unsupported("Objects::set_object_material")) }
    fn remove_buildings(&mut self, _model: ModelId, _center: Vec3, _radius: f32) -> GameResult<u32> { Err(GameError::Unsupported("Objects::remove_buildings")) }
    fn camera_ignore(&mut self, _e: EntityRef) -> GameResult<()> { Err(GameError::Unsupported("Objects::camera_ignore")) }
}

impl Peds for ModGame {
    fn local_player(&self) -> Option<PedRef> {
        state::player().and_then(|p| p.handle)
    }
    fn player_ped(&self, _slot: PlayerSlot) -> Option<PedRef> { None }
    fn player_in_focus(&self) -> PlayerSlot { PlayerSlot(0) }
    fn ped_script_handle(&self, _ped: PedRef) -> GameResult<ScriptHandle> { Err(GameError::Unsupported("Peds::ped_script_handle")) }
    fn ped_from_script_handle(&self, _h: ScriptHandle) -> Option<PedRef> { None }
    fn ped_in_world(&self, ped: PedRef) -> bool {
        state::ped(ped).is_some_and(|p| p.in_world)
    }
    fn is_ped_model(&self, model: ModelId) -> bool {
        PED_MODELS.contains(&model.0)
    }
    fn create_player_ped(&mut self, _slot: PlayerSlot, _model: ModelId, _pos: Vec3) -> GameResult<PedRef> { Err(GameError::Unsupported("Peds::create_player_ped")) }
    fn create_actor(&mut self, model: ModelId, pos: Vec3, heading_deg: f32) -> GameResult<PedRef> {
        let ped = entities::create_ped(model.0, pos).map_err(failed("create_actor"))?;
        world::set_ped_heading(ped, heading_deg).map_err(failed("create_actor"))?;
        Ok(ped)
    }
    fn destroy_ped(&mut self, ped: PedRef) -> GameResult<()> {
        entities::destroy_ped(ped).map_err(failed("destroy_ped"))
    }
    fn set_ped_model(&mut self, ped: PedRef, model: ModelId) -> GameResult<()> {
        game::set_ped_model(ped, model.0).map_err(failed("set_ped_model"))
    }
    fn ped_snapshot(&self, ped: PedRef) -> GameResult<PedSnapshot> {
        let p = state::ped(ped).ok_or(GameError::InvalidHandle)?;
        let m = state::ped_matrix(ped).ok_or(GameError::InvalidHandle)?;
        Ok(PedSnapshot {
            matrix: m.into(),
            move_speed: p.move_speed,
            health: p.health,
            armour: p.armour,
            heading: p.heading,
            heading_goal: p.heading,
            ped_state: p.ped_state,
            model: p.model,
            weapon: p.weapon,
            vehicle: p.vehicle,
            area: p.area,
            in_world: p.in_world,
            ..PedSnapshot::default()
        })
    }
    fn ped_weapons(&self, _ped: PedRef) -> GameResult<[WeaponSlot; 13]> { Err(GameError::Unsupported("Peds::ped_weapons")) }
    fn bone_position(&self, _ped: PedRef, _bone: BoneId, _transformed: bool) -> GameResult<Vec3> { Err(GameError::Unsupported("Peds::bone_position")) }
    fn weapon_skill(&self, _ped: PedRef) -> GameResult<u8> { Err(GameError::Unsupported("Peds::weapon_skill")) }
    fn ped_on_fire(&self, _ped: PedRef) -> GameResult<bool> { Err(GameError::Unsupported("Peds::ped_on_fire")) }
    fn anim_playing(&self, _ped: PedRef, _block: &str, _name: &str) -> GameResult<bool> { Err(GameError::Unsupported("Peds::anim_playing")) }
    fn player_money(&self) -> i32 {
        state::player().map_or(0, |p| p.money)
    }
    fn set_ped_position(&mut self, ped: PedRef, pos: Vec3) -> GameResult<()> {
        world::set_ped_position(ped, pos).map_err(failed("set_ped_position"))
    }
    fn set_ped_matrix(&mut self, _ped: PedRef, _m: &Matrix) -> GameResult<()> { Err(GameError::Unsupported("Peds::set_ped_matrix")) }
    fn set_ped_move_speed(&mut self, _ped: PedRef, _v: Vec3) -> GameResult<()> { Err(GameError::Unsupported("Peds::set_ped_move_speed")) }
    fn set_ped_heading(&mut self, ped: PedRef, heading: f32, raw: bool) -> GameResult<()> {
        let degrees = if raw { heading.to_degrees() } else { heading };
        world::set_ped_heading(ped, degrees).map_err(failed("set_ped_heading"))
    }
    fn set_ped_health(&mut self, ped: PedRef, health: f32) -> GameResult<()> {
        world::set_ped_health(ped, health).map_err(failed("set_ped_health"))
    }
    fn set_ped_armour(&mut self, ped: PedRef, armour: f32) -> GameResult<()> {
        world::set_ped_armour(ped, armour).map_err(failed("set_ped_armour"))
    }
    fn give_weapon(&mut self, ped: PedRef, weapon: WeaponType, ammo: u32) -> GameResult<()> {
        world::give_weapon(ped, weapon.0, ammo).map_err(failed("give_weapon"))
    }
    fn set_current_weapon(&mut self, ped: PedRef, weapon: WeaponType) -> GameResult<()> {
        cmd(0x01B9, &[Arg::Ped(ped), Arg::Int(i32::from(weapon.0))])
    }
    fn clear_weapons(&mut self, ped: PedRef) -> GameResult<()> {
        cmd(0x048F, &[Arg::Ped(ped)])
    }
    fn set_weapon_slot(&mut self, _ped: PedRef, _slot: u8, _value: WeaponSlot) -> GameResult<()> { Err(GameError::Unsupported("Peds::set_weapon_slot")) }
    fn remove_weapon_entering_vehicle(&mut self, _ped: PedRef) -> GameResult<()> { Err(GameError::Unsupported("Peds::remove_weapon_entering_vehicle")) }
    fn set_proofs(&mut self, _ped: PedRef, _proofs: Proofs) -> GameResult<()> { Err(GameError::Unsupported("Peds::set_proofs")) }
    fn set_suffers_critical_hits(&mut self, ped: PedRef, on: bool) -> GameResult<()> {
        cmd(0x0446, &[Arg::Ped(ped), Arg::Int(i32::from(on))])
    }
    fn set_drops_weapons_when_dead(&mut self, ped: PedRef, on: bool) -> GameResult<()> {
        cmd(0x087E, &[Arg::Ped(ped), Arg::Int(i32::from(on))])
    }
    fn freeze_ped(&mut self, ped: PedRef, frozen: bool) -> GameResult<()> {
        cmd(0x04D7, &[Arg::Ped(ped), Arg::Int(i32::from(frozen))])
    }
    fn set_ped_area(&mut self, ped: PedRef, area: AreaCode) -> GameResult<()> {
        cmd(0x0860, &[Arg::Ped(ped), Arg::Int(i32::from(area.0))])
    }
    fn set_decision_maker(&mut self, _ped: PedRef, _maker: i32) -> GameResult<()> { Err(GameError::Unsupported("Peds::set_decision_maker")) }
    fn set_player_money(&mut self, money: i32) -> GameResult<()> {
        world::set_money(money).map_err(failed("set_player_money"))
    }
    fn set_player_control(&mut self, enabled: bool) -> GameResult<()> {
        cmd(0x01B4, &[Arg::Int(0), Arg::Int(i32::from(enabled))])
    }
    fn set_drunkenness(&mut self, level: i32) -> GameResult<()> {
        cmd(0x052C, &[Arg::Int(0), Arg::Int(level)])
    }
    fn set_ped_flags(&mut self, _ped: PedRef, _mask: [u32; 2], _value: [u32; 2]) -> GameResult<()> { Err(GameError::Unsupported("Peds::set_ped_flags")) }
    fn extinguish_ped(&mut self, _ped: PedRef) -> GameResult<()> { Err(GameError::Unsupported("Peds::extinguish_ped")) }
    fn explode_head(&mut self, _ped: PedRef) -> GameResult<()> { Err(GameError::Unsupported("Peds::explode_head")) }
    fn warp_into_vehicle(&mut self, ped: PedRef, vehicle: VehicleRef, seat: Option<i32>) -> GameResult<()> {
        match seat {
            None => cmd(0x036A, &[Arg::Ped(ped), Arg::Vehicle(vehicle)]),
            Some(s) => cmd(0x0430, &[Arg::Ped(ped), Arg::Vehicle(vehicle), Arg::Int(s)]),
        }
    }
    fn warp_out_of_vehicle(&mut self, ped: PedRef, pos: Vec3) -> GameResult<()> {
        cmd(0x0362, &[Arg::Ped(ped), Arg::Float(pos.x), Arg::Float(pos.y), Arg::Float(pos.z)])
    }
    fn start_task(&mut self, ped: PedRef, task: PedTask) -> GameResult<()> {
        let (p, v, i) = (Arg::Ped(ped), Arg::Vehicle, Arg::Int);
        match task {
            PedTask::HandsUp { time_ms } => cmd(0x05C4, &[p, i(time_ms)]),
            PedTask::EnterCarAsDriver { vehicle, time_ms } => cmd(0x05CB, &[p, v(vehicle), i(time_ms)]),
            PedTask::EnterCarAsPassenger { vehicle, time_ms, seat } => {
                cmd(0x05CA, &[p, v(vehicle), i(time_ms), i(seat)])
            }
            PedTask::LeaveCar { vehicle } => cmd(0x05CD, &[p, v(vehicle)]),
            _ => Err(GameError::Unsupported("Peds::start_task (this task)")),
        }
    }
    fn flush_tasks(&mut self, _ped: PedRef) -> GameResult<()> { Err(GameError::Unsupported("Peds::flush_tasks")) }
    fn delete_task(&mut self, _task: TaskRef) -> GameResult<()> { Err(GameError::Unsupported("Peds::delete_task")) }
    fn play_anim(&mut self, ped: PedRef, anim: &AnimRequest<'_>) -> GameResult<()> {
        if anim.name.is_empty() || anim.name.len() > 23 || anim.block.is_empty() || anim.block.len() > 15 {
            return Err(GameError::Other(format!("bad animation {:?}/{:?}", anim.block, anim.name)));
        }
        let block = Arg::Text(anim.block.as_bytes());
        if !cond(0x04EE, &[block])? {
            cmd(0x04ED, &[block])?;
            cmd(0x038B, &[])?;
            if !cond(0x04EE, &[block])? {
                return Err(GameError::NotLoaded);
            }
        }
        let i = |b: bool| Arg::Int(i32::from(b));
        cmd(
            0x0812,
            &[
                Arg::Ped(ped),
                Arg::Text(anim.name.as_bytes()),
                block,
                Arg::Float(anim.delta),
                i(anim.looped),
                i(anim.lock_x),
                i(anim.lock_y),
                i(anim.freeze),
                Arg::Int(anim.time_ms),
            ],
        )
    }
    fn stop_anims(&mut self, ped: PedRef) -> GameResult<()> {
        cmd(0x0687, &[Arg::Ped(ped)])
    }
    fn look_at(&mut self, _ped: PedRef, _target: Vec3, _time_ms: i32) -> GameResult<()> { Err(GameError::Unsupported("Peds::look_at")) }
    fn set_jetpack(&mut self, _ped: PedRef, _on: bool) -> GameResult<()> { Err(GameError::Unsupported("Peds::set_jetpack")) }
    fn has_jetpack(&self, _ped: PedRef) -> GameResult<bool> { Err(GameError::Unsupported("Peds::has_jetpack")) }
    fn wanted_level(&self) -> GameResult<u8> {
        state::player().map(|p| p.wanted).ok_or(GameError::InvalidHandle)
    }
    fn peds(&self) -> Vec<PedRef> {
        state::peds()
    }
}

impl Overlay for ModGame {
    fn render_api(&self) -> RenderApi { RenderApi::Engine }
    fn create_texture(&mut self, _width: u32, _height: u32, _format: PixelFormat) -> GameResult<TextureRef> { Err(GameError::Unsupported("Overlay::create_texture")) }
    fn update_texture( &mut self, _tex: TextureRef, _region: Rect2, _format: PixelFormat, _pixels: &[u8], ) -> GameResult<()> { Err(GameError::Unsupported("Overlay::update_texture")) }
    fn destroy_texture(&mut self, _tex: TextureRef) -> GameResult<()> { Err(GameError::Unsupported("Overlay::destroy_texture")) }
    fn draw_triangles( &mut self, _tex: Option<TextureRef>, _vertices: &[OverlayVertex], _clip: Option<Rect2>, ) -> GameResult<()> { Err(GameError::Unsupported("Overlay::draw_triangles")) }
    fn draw_lines(&mut self, _points: &[Point2], _colour: Rgba) -> GameResult<()> { Err(GameError::Unsupported("Overlay::draw_lines")) }
    fn render_model_preview(&mut self, _preview: &ModelPreview) -> GameResult<TextureRef> { Err(GameError::Unsupported("Overlay::render_model_preview")) }
    fn screenshot(&mut self) -> GameResult<String> { Err(GameError::Unsupported("Overlay::screenshot")) }
}

impl ScriptCommands for ModGame {
    fn run_command( &mut self, _op: Opcode, _args: &[ScriptArg<'_>], _locals: &mut ScriptLocals, ) -> GameResult<bool> { Err(GameError::Unsupported("ScriptCommands::run_command")) }
    fn create_script_thread(&mut self) -> GameResult<ScriptThreadRef> { Err(GameError::Unsupported("ScriptCommands::create_script_thread")) }
    fn run_on_thread( &mut self, _thread: ScriptThreadRef, _op: Opcode, _args: &[ScriptArg<'_>], ) -> GameResult<bool> { Err(GameError::Unsupported("ScriptCommands::run_on_thread")) }
    fn thread_local(&self, _thread: ScriptThreadRef, _i: usize) -> GameResult<u32> { Err(GameError::Unsupported("ScriptCommands::thread_local")) }
    fn set_thread_local(&mut self, _thread: ScriptThreadRef, _i: usize, _value: u32) -> GameResult<()> { Err(GameError::Unsupported("ScriptCommands::set_thread_local")) }
    fn last_command(&self) -> (Opcode, bool) { (Opcode(0), false) }
}


impl Streaming for ModGame {
    fn model_kind(&self, _model: ModelId) -> ModelKind { ModelKind::None }
    fn request_model(&mut self, model: ModelId) -> GameResult<()> {
        cmd(0x0247, &[Arg::Int(model.0)])
    }
    fn model_loaded(&self, _model: ModelId) -> bool { false }
    fn release_model(&mut self, _model: ModelId) -> GameResult<()> { Err(GameError::Unsupported("Streaming::release_model")) }
    fn load_all_now(&mut self) -> GameResult<()> {
        cmd(0x038B, &[])
    }
    fn request_collision(&mut self, at: Vec3) -> GameResult<()> {
        cmd(0x04E4, &[Arg::Float(at.x), Arg::Float(at.y)])
    }
    fn collision_loaded(&self, _at: Vec3, _area: u8) -> bool { false }
    fn request_anim_block(&mut self, _block: &str) -> GameResult<()> { Err(GameError::Unsupported("Streaming::request_anim_block")) }
    fn anim_block_loaded(&self, _block: &str) -> bool { false }
    fn request_component(&mut self, _model: ModelId) -> GameResult<()> { Err(GameError::Unsupported("Streaming::request_component")) }
    fn component_loaded(&self, _model: ModelId) -> bool { false }
    fn streaming_memory(&self) -> (u32, u32) { Default::default() }
}

impl Vehicles for ModGame {
    fn vehicle_script_handle(&self, _v: VehicleRef) -> GameResult<ScriptHandle> { Err(GameError::Unsupported("Vehicles::vehicle_script_handle")) }
    fn vehicle_from_script_handle(&self, _h: ScriptHandle) -> Option<VehicleRef> { None }
    fn vehicle_snapshot(&self, v: VehicleRef) -> GameResult<VehicleSnapshot> {
        let i = state::vehicle(v).ok_or(GameError::InvalidHandle)?;
        let m = state::vehicle_matrix(v).ok_or(GameError::InvalidHandle)?;
        Ok(VehicleSnapshot {
            matrix: m.into(),
            move_speed: i.move_speed,
            health: i.health,
            model: i.model,
            driver: i.driver,
            colours: [i.colour1, i.colour2],
            siren: i.siren,
            area: i.area,
            ..VehicleSnapshot::default()
        })
    }
    fn vehicle_damage(&self, _v: VehicleRef) -> GameResult<VehicleDamage> { Err(GameError::Unsupported("Vehicles::vehicle_damage")) }
    fn vehicle_in_water(&self, _v: VehicleRef) -> GameResult<bool> { Err(GameError::Unsupported("Vehicles::vehicle_in_water")) }
    fn local_vehicle(&self) -> Option<VehicleRef> {
        state::player().and_then(|p| p.ped.vehicle)
    }
    fn create_vehicle(&mut self, spawn: &VehicleSpawn) -> GameResult<VehicleRef> {
        let v = entities::create_vehicle(spawn.model.0, spawn.pos, spawn.heading_deg).map_err(failed("create_vehicle"))?;
        world::set_vehicle_colours(v, spawn.colours[0], spawn.colours[1]).map_err(failed("create_vehicle"))?;
        Ok(v)
    }
    fn destroy_vehicle(&mut self, v: VehicleRef) -> GameResult<()> {
        entities::destroy_vehicle(v).map_err(failed("destroy_vehicle"))
    }
    fn set_vehicle_matrix(&mut self, _v: VehicleRef, _m: &Matrix) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_vehicle_matrix")) }
    fn set_vehicle_speed(&mut self, _v: VehicleRef, _move_speed: Vec3, _turn_speed: Vec3) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_vehicle_speed")) }
    fn set_vehicle_heading(&mut self, _v: VehicleRef, _heading_deg: f32) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_vehicle_heading")) }
    fn set_train_position(&mut self, _v: VehicleRef, _pos: Vec3) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_train_position")) }
    fn set_train_speed(&mut self, _v: VehicleRef, _speed: f32) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_train_speed")) }
    fn set_vehicle_health(&mut self, v: VehicleRef, health: f32) -> GameResult<()> {
        world::set_vehicle_health(v, health).map_err(failed("set_vehicle_health"))
    }
    fn set_vehicle_damage(&mut self, _v: VehicleRef, _d: VehicleDamage) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_vehicle_damage")) }
    fn fix_vehicle(&mut self, _v: VehicleRef) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::fix_vehicle")) }
    fn set_vehicle_colours(&mut self, _v: VehicleRef, _colours: [u8; 2]) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_vehicle_colours")) }
    fn set_paintjob(&mut self, _v: VehicleRef, _paintjob: u8) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_paintjob")) }
    fn add_component(&mut self, _v: VehicleRef, _component: ModelId) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::add_component")) }
    fn remove_component(&mut self, _v: VehicleRef, _component: ModelId) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::remove_component")) }
    fn set_hydraulics(&mut self, _v: VehicleRef, _on: bool) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_hydraulics")) }
    fn set_vehicle_proofs(&mut self, _v: VehicleRef, _proofs: Proofs) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_vehicle_proofs")) }
    fn set_tyres_can_burst(&mut self, _v: VehicleRef, _on: bool) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_tyres_can_burst")) }
    fn set_petrol_tank_weakpoint(&mut self, _v: VehicleRef, _on: bool) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_petrol_tank_weakpoint")) }
    fn set_vehicle_area(&mut self, _v: VehicleRef, _area: AreaCode) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_vehicle_area")) }
    fn set_door(&mut self, _v: VehicleRef, _door: u8, _open_ratio: f32) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_door")) }
    fn set_doors_locked(&mut self, _v: VehicleRef, _locked: bool) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_doors_locked")) }
    fn set_window_open(&mut self, _v: VehicleRef, _window: u8, _open: bool) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_window_open")) }
    fn set_landing_gear(&mut self, _v: VehicleRef, _down: bool) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_landing_gear")) }
    fn set_siren(&mut self, _v: VehicleRef, _on: bool) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_siren")) }
    fn set_engine_lights(&mut self, _v: VehicleRef, _engine: bool, _lights: bool) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_engine_lights")) }
    fn set_trailer(&mut self, _cab: VehicleRef, _trailer: Option<VehicleRef>) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_trailer")) }
    fn set_plate_text(&mut self, _v: VehicleRef, _text: &str) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_plate_text")) }
    fn vehicles(&self) -> Vec<VehicleRef> {
        state::vehicles()
    }
}

impl WorldState for ModGame {
    fn clock(&self) -> (u8, u8) {
        state::world().map_or((0, 0), |w| (w.hour, w.minute))
    }
    fn set_clock(&mut self, hour: u8, minute: u8) -> GameResult<()> {
        world::set_clock(hour, minute).map_err(failed("set_clock"))
    }
    fn weather(&self) -> (i16, i16, i16) {
        state::world().map_or((0, 0, 0), |w| (w.weather_forced as i16, w.weather_new as i16, w.weather_old as i16))
    }
    fn set_weather(&mut self, weather: u8) -> GameResult<()> {
        world::set_weather(weather).map_err(failed("set_weather"))
    }
    fn force_weather(&mut self, _weather: u8) -> GameResult<()> { Err(GameError::Unsupported("WorldState::force_weather")) }
    fn gravity(&self) -> f32 {
        state::world().map_or(0.0, |w| w.gravity)
    }
    fn set_gravity(&mut self, g: f32) -> GameResult<()> {
        if self.gravity().to_bits() == g.to_bits() {
            return Ok(());
        }
        Err(GameError::Unsupported("WorldState::set_gravity"))
    }
    fn area(&self) -> AreaCode {
        AreaCode(self.area_raw() as u8)
    }
    fn area_raw(&self) -> i32 {
        state::world().map_or(0, |w| w.area)
    }
    fn set_area(&mut self, area: AreaCode) -> GameResult<()> {
        cmd(0x04BB, &[Arg::Int(i32::from(area.0))])
    }
    fn set_stunt_bonus(&mut self, _on: bool) -> GameResult<()> { Err(GameError::Unsupported("WorldState::set_stunt_bonus")) }
    fn set_max_wanted_level(&mut self, level: u8) -> GameResult<()> {
        cmd(0x01F0, &[Arg::Int(i32::from(level))])
    }
    fn set_densities(&mut self, peds: f32, cars: f32) -> GameResult<()> {
        cmd(0x03DE, &[Arg::Float(peds)])?;
        cmd(0x01EB, &[Arg::Float(cars)])
    }
    fn switch_roads_off(&mut self) -> GameResult<()> {
        let f = Arg::Float;
        cmd(0x01E8, &[f(-4000.0), f(-4000.0), f(-500.0), f(4000.0), f(4000.0), f(1500.0)])
    }
    fn set_random_trains(&mut self, on: bool) -> GameResult<()> {
        cmd(0x06D7, &[Arg::Int(i32::from(on))])
    }
    fn set_restart_point(&mut self, pos: Vec3, heading_deg: f32) -> GameResult<()> {
        let f = Arg::Float;
        cmd(0x016C, &[f(pos.x), f(pos.y), f(pos.z), f(heading_deg), Arg::Int(0)])
    }
    fn ground_z(&self, _pos: Vec3) -> f32 { Default::default() }
    fn line_of_sight(&self, _from: Vec3, _to: Vec3, _filter: LineFilter) -> Option<LineHit> { None }
    fn line_clear(&self, from: Vec3, to: Vec3, f: LineFilter) -> bool {
        let (x, i) = (Arg::Float, |b: bool| Arg::Int(i32::from(b)));
        let args = [x(from.x), x(from.y), x(from.z), x(to.x), x(to.y), x(to.z)];
        let flags = [i(f.buildings), i(f.vehicles), i(f.peds), i(f.objects), i(f.dummies)];
        cond(0x06BD, &[args.as_slice(), flags.as_slice()].concat()).unwrap_or(false)
    }
    fn add_explosion(&mut self, _pos: Vec3, _kind: i32, _radius: f32, _shake: f32) -> GameResult<()> { Err(GameError::Unsupported("WorldState::add_explosion")) }
    fn create_fx(&mut self, _name: &str, _on: Option<PedRef>, _offset: Vec3, _dir: Vec3) -> GameResult<FxRef> { Err(GameError::Unsupported("WorldState::create_fx")) }
    fn kill_fx(&mut self, _fx: FxRef) -> GameResult<()> { Err(GameError::Unsupported("WorldState::kill_fx")) }
    fn load_shop(&mut self, _name: &str) -> GameResult<()> { Err(GameError::Unsupported("WorldState::load_shop")) }
}

impl WorldEntities for ModGame {
    fn create_pickup(&mut self, _model: ModelId, _kind: i32, _pos: Vec3) -> GameResult<PickupRef> { Err(GameError::Unsupported("WorldEntities::create_pickup")) }
    fn remove_pickup(&mut self, _p: PickupRef) -> GameResult<()> { Err(GameError::Unsupported("WorldEntities::remove_pickup")) }
    fn pickup_collected(&self, _p: PickupRef) -> GameResult<bool> { Err(GameError::Unsupported("WorldEntities::pickup_collected")) }
    fn pickup_index(&self, _p: PickupRef) -> GameResult<u16> { Err(GameError::Unsupported("WorldEntities::pickup_index")) }
    fn create_checkpoint(&mut self, _kind: CheckpointKind, _pos: Vec3, _size: f32) -> GameResult<CheckpointRef> { Err(GameError::Unsupported("WorldEntities::create_checkpoint")) }
    fn delete_checkpoint(&mut self, _c: CheckpointRef) -> GameResult<()> { Err(GameError::Unsupported("WorldEntities::delete_checkpoint")) }
    fn locate_ped(&self, _ped: PedRef, _pos: Vec3, _radius: Vec3) -> GameResult<bool> { Err(GameError::Unsupported("WorldEntities::locate_ped")) }
    fn add_blip(&mut self, _target: BlipTarget) -> GameResult<BlipRef> { Err(GameError::Unsupported("WorldEntities::add_blip")) }
    fn remove_blip(&mut self, _b: BlipRef) -> GameResult<()> { Err(GameError::Unsupported("WorldEntities::remove_blip")) }
    fn set_blip_colour(&mut self, _b: BlipRef, _colour: u32) -> GameResult<()> { Err(GameError::Unsupported("WorldEntities::set_blip_colour")) }
    fn set_blip_scale(&mut self, _b: BlipRef, _scale: i32) -> GameResult<()> { Err(GameError::Unsupported("WorldEntities::set_blip_scale")) }
    fn set_blip_display(&mut self, _b: BlipRef, _display: i32) -> GameResult<()> { Err(GameError::Unsupported("WorldEntities::set_blip_display")) }
}
