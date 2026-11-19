//! samp-rs's game adapter (`sa_samp_api::GameAdapter`) over the mod API: what the client reads comes from the
//! state queries, what it changes goes through the mod API's changes, entities and the game's script commands.
//! What the mod API does not offer is `Unsupported`, as on any backend that lacks a feature.

use sa_mod::script::Arg;
use sa_mod::{Error, audio, entities, game, markers, models, objects, radar, script, state, world};
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

/// A spawn colour pair of this value keeps the model's own colours.
const MODEL_COLOURS: u8 = 0xFF;

/// The ped models: the skins `SetPlayerSkin` may name.
const PED_MODELS: std::ops::RangeInclusive<i32> = 0..=311;

/// The game as the mod API shows it.
#[derive(Default)]
pub struct ModGame {
    config: SampModeConfig,
    /// What the UI draws through; [`crate::draw_ui`] turns it into overlay quads.
    pub ui: samp_overlay::QuadUi,
    /// The trailers attached through [`Vehicles::set_trailer`]: (cab, trailer).
    trailers: Vec<(VehicleRef, VehicleRef)>,
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

/// The cut argument of the camera commands: 2 a jump cut, 1 a smooth move.
fn cut_arg(cut: CameraCut) -> i32 {
    match cut {
        CameraCut::Cut => 2,
        CameraCut::Smooth => 1,
    }
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
    fn start_radio(&mut self, station: i8) -> GameResult<()> {
        cmd(0x041E, &[Arg::Int(i32::from(station))])
    }
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
        cmd(0x0160, &[Arg::Float(target.x), Arg::Float(target.y), Arg::Float(target.z), Arg::Int(cut_arg(cut))])
    }
    fn point_at_ped(&mut self, ped: PedRef, mode: i32, cut: CameraCut) -> GameResult<()> {
        cmd(0x0159, &[Arg::Ped(ped), Arg::Int(mode), Arg::Int(cut_arg(cut))])
    }
    fn point_at_vehicle(&mut self, vehicle: VehicleRef, mode: i32, cut: CameraCut) -> GameResult<()> {
        cmd(0x0158, &[Arg::Vehicle(vehicle), Arg::Int(mode), Arg::Int(cut_arg(cut))])
    }
    fn behind_player(&mut self) -> GameResult<()> {
        cmd(0x0373, &[])
    }
    fn restore(&mut self) -> GameResult<()> {
        cmd(0x02EB, &[])
    }
    /// A position move resets the camera's scripted moves first (`CAMERA_RESET_NEW_SCRIPTABLES`) and keeps its end
    /// position (`CAMERA_PERSIST_POS`); a look-at move keeps its end target (`CAMERA_PERSIST_TRACK`).
    fn interpolate(&mut self, look_at: bool, from: Vec3, to: Vec3, time_ms: i32, cut: CameraCut) -> GameResult<()> {
        let f = Arg::Float;
        let args = [f(from.x), f(from.y), f(from.z), f(to.x), f(to.y), f(to.z), Arg::Int(time_ms), Arg::Int(cut_arg(cut))];
        if look_at {
            cmd(0x092F, &[Arg::Int(1)])?;
            cmd(0x0920, &args)
        } else {
            cmd(0x0925, &[])?;
            cmd(0x0930, &[Arg::Int(1)])?;
            cmd(0x0936, &args)
        }
    }
    fn set_widescreen(&mut self, on: bool) -> GameResult<()> {
        cmd(0x02A3, &[Arg::Int(i32::from(on))])
    }
    fn reset_scripted_moves(&mut self) -> GameResult<()> {
        cmd(0x0925, &[])
    }
    fn persist_position(&mut self, on: bool) -> GameResult<()> {
        cmd(0x0930, &[Arg::Int(i32::from(on))])
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
    fn set_text_entry(&mut self, key: &str, text: &[u8]) -> GameResult<()> {
        sa_mod::gxt::set(key, text).map_err(failed("set_text_entry"))
    }
    fn remove_text_entry(&mut self, key: &str) -> GameResult<()> {
        sa_mod::gxt::remove(key).map_err(failed("remove_text_entry"))
    }
    fn create_menu(&mut self, m: &MenuSpec<'_>) -> GameResult<MenuRef> {
        use sa_mod::menus::{BACKGROUND, INTERACTIVE};
        let flags = if m.interactive { INTERACTIVE } else { 0 } | if m.background { BACKGROUND } else { 0 };
        sa_mod::menus::create(m.title, m.pos.x, m.pos.y, m.width, m.columns, flags, i32::from(m.alignment))
            .map_err(failed("create_menu"))
    }
    fn set_menu_column(&mut self, menu: MenuRef, column: u8, header: &str, rows: &[&str]) -> GameResult<()> {
        sa_mod::menus::set_column(menu, column, header, rows).map_err(failed("set_menu_column"))
    }
    fn set_menu_column_width(&mut self, menu: MenuRef, column: u8, width: u16) -> GameResult<()> {
        sa_mod::menus::set_column_width(menu, column, width).map_err(failed("set_menu_column_width"))
    }
    fn set_menu_row_enabled(&mut self, menu: MenuRef, row: u8, on: bool) -> GameResult<()> {
        sa_mod::menus::set_row_enabled(menu, row, on).map_err(failed("set_menu_row_enabled"))
    }
    fn menu_selection(&self, menu: MenuRef) -> GameResult<i32> {
        sa_mod::menus::selected_row(menu).map_err(failed("menu_selection"))
    }
    fn delete_menu(&mut self, menu: MenuRef) -> GameResult<()> {
        sa_mod::menus::destroy(menu).map_err(failed("delete_menu"))
    }
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
    fn object_matrix(&self, o: ObjectRef) -> GameResult<Matrix> {
        objects::matrix(o).map(Into::into).ok_or(GameError::InvalidHandle)
    }
    fn object_has_rw(&self, o: ObjectRef) -> GameResult<bool> {
        objects::info(o).map(|i| i.has_rw).ok_or(GameError::InvalidHandle)
    }
    fn object_attached(&self, o: ObjectRef) -> GameResult<bool> {
        objects::info(o).map(|i| i.attached).ok_or(GameError::InvalidHandle)
    }
    fn object_bounds(&self, o: ObjectRef) -> GameResult<(Vec3, f32)> {
        objects::info(o).map(|i| (i.bound_centre, i.bound_radius)).ok_or(GameError::InvalidHandle)
    }
    fn object_ray_test(&self, _o: ObjectRef, _from: Vec3, _to: Vec3) -> GameResult<Option<ObjectHit>> { Err(GameError::Unsupported("Objects::object_ray_test")) }
    fn create_object(&mut self, model: ModelId, pos: Vec3) -> GameResult<ObjectRef> {
        entities::create_object(model.0, pos).map_err(failed("create_object"))
    }
    fn destroy_object(&mut self, o: ObjectRef, _fade: bool) -> GameResult<()> {
        entities::destroy_object(o).map_err(failed("destroy_object"))
    }
    fn set_object_transform(&mut self, o: ObjectRef, pos: Vec3, rot_deg: Vec3) -> GameResult<()> {
        objects::set_position(o, pos).map_err(failed("set_object_transform"))?;
        objects::set_rotation(o, rot_deg).map_err(failed("set_object_transform"))
    }
    fn set_object_matrix(&mut self, o: ObjectRef, m: &Matrix) -> GameResult<()> {
        objects::set_matrix(o, &m.into()).map_err(failed("set_object_matrix"))
    }
    fn set_object_speed(&mut self, o: ObjectRef, move_speed: Vec3, turn_speed: Vec3) -> GameResult<()> {
        objects::set_speed(o, move_speed, turn_speed).map_err(failed("set_object_speed"))
    }
    fn set_object_scale(&mut self, o: ObjectRef, scale: f32) -> GameResult<()> {
        objects::set_scale(o, scale).map_err(failed("set_object_scale"))
    }
    fn set_object_visible(&mut self, o: ObjectRef, visible: bool) -> GameResult<()> {
        objects::set_visible(o, visible).map_err(failed("set_object_visible"))
    }
    fn set_object_collision(&mut self, o: ObjectRef, on: bool) -> GameResult<()> {
        objects::set_collision(o, on).map_err(failed("set_object_collision"))
    }
    fn attach_object(&mut self, o: ObjectRef, to: AttachTarget, offset: Vec3, rot_deg: Vec3) -> GameResult<()> {
        let target = match to {
            AttachTarget::Vehicle(v) => objects::Target::Vehicle(v),
            AttachTarget::Object(t) => objects::Target::Object(t),
            AttachTarget::Ped(p) => objects::Target::Ped(p),
            AttachTarget::PedBone { .. } => return Err(GameError::Unsupported("Objects::attach_object (bone)")),
        };
        objects::attach(o, target, offset, rot_deg).map_err(failed("attach_object"))
    }
    fn detach_object(&mut self, o: ObjectRef) -> GameResult<()> {
        objects::detach(o).map_err(failed("detach_object"))
    }
    fn play_object_anim( &mut self, _o: ObjectRef, _anim: &str, _block: &str, _delta: f32, _looped: bool, ) -> GameResult<()> { Err(GameError::Unsupported("Objects::play_object_anim")) }
    fn set_object_material(&mut self, o: ObjectRef, index: u8, m: Option<MaterialOverride>) -> GameResult<()> {
        let rgba = |c: Rgba| sa_mod::Rgba(u32::from_be_bytes([c.r, c.g, c.b, c.a]));
        let r = match &m {
            None => objects::set_material(o, index, None, sa_mod::Rgba(0)),
            Some(MaterialOverride::Colour(c)) => objects::set_material(o, index, None, rgba(*c)),
            Some(MaterialOverride::Texture { model, txd, texture, colour }) => {
                objects::set_material(o, index, Some((model.0, txd, texture)), rgba(*colour))
            }
            Some(MaterialOverride::Rendered(_)) => return Err(GameError::Unsupported("Objects::set_object_material (rendered)")),
        };
        r.map_err(failed("set_object_material"))
    }
    fn remove_buildings(&mut self, model: ModelId, center: Vec3, radius: f32) -> GameResult<u32> {
        objects::remove_buildings(model.0, center, radius).map_err(failed("remove_buildings"))
    }
    fn camera_ignore(&mut self, _e: EntityRef) -> GameResult<()> { Err(GameError::Unsupported("Objects::camera_ignore")) }
}

impl Peds for ModGame {
    fn local_player(&self) -> Option<PedRef> {
        state::player().and_then(|p| p.handle)
    }
    fn player_ped(&self, _slot: PlayerSlot) -> Option<PedRef> { None }
    fn player_in_focus(&self) -> PlayerSlot { PlayerSlot(0) }
    /// The mod API's handles are the game's script handles (pool references).
    fn ped_script_handle(&self, ped: PedRef) -> GameResult<ScriptHandle> {
        state::ped(ped).map(|_| ScriptHandle(ped.0 as i32)).ok_or(GameError::InvalidHandle)
    }
    fn ped_from_script_handle(&self, h: ScriptHandle) -> Option<PedRef> {
        let ped = PedRef(h.0 as u32);
        state::ped(ped).map(|_| ped)
    }
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
    fn ped_weapons(&self, ped: PedRef) -> GameResult<[WeaponSlot; 13]> {
        let mut out = [WeaponSlot::default(); 13];
        for (slot, w) in (1..).zip(out.iter_mut()) {
            let o = script::run(0x04B8, &[Arg::Ped(ped), Arg::Int(slot), Arg::Out, Arg::Out, Arg::Out])
                .map_err(failed("ped_weapons"))?;
            *w = WeaponSlot { weapon: o.outputs[0] as u8, ammo_total: o.outputs[1], ..WeaponSlot::default() };
        }
        Ok(out)
    }
    /// The head and neck bones, from the head and neck nodes of the skeleton (the nodes the mod API reads).
    fn bone_position(&self, ped: PedRef, bone: BoneId, _transformed: bool) -> GameResult<Vec3> {
        let node = match bone.0 {
            5 => 17,
            6..=8 => 2,
            _ => return Err(GameError::Unsupported("Peds::bone_position (a bone off the head and neck)")),
        };
        objects::ped_bone_matrix(ped, node).map(|m| Matrix::from(m).pos).ok_or(GameError::NotLoaded)
    }
    fn node_matrix(&self, ped: PedRef, node: u8) -> GameResult<Matrix> {
        objects::ped_bone_matrix(ped, node).map(Into::into).ok_or(GameError::NotLoaded)
    }
    fn weapon_skill(&self, _ped: PedRef) -> GameResult<u8> { Err(GameError::Unsupported("Peds::weapon_skill")) }
    fn ped_on_fire(&self, _ped: PedRef) -> GameResult<bool> { Err(GameError::Unsupported("Peds::ped_on_fire")) }
    /// `IS_CHAR_PLAYING_ANIM` (0611) finds the animation by its name in any block.
    fn anim_playing(&self, ped: PedRef, _block: &str, name: &str) -> GameResult<bool> {
        cond(0x0611, &[Arg::Ped(ped), Arg::Text(name.as_bytes())])
    }
    fn player_money(&self) -> i32 {
        state::player().map_or(0, |p| p.money)
    }
    fn set_ped_position(&mut self, ped: PedRef, pos: Vec3) -> GameResult<()> {
        world::set_ped_position(ped, pos).map_err(failed("set_ped_position"))
    }
    fn set_ped_matrix(&mut self, _ped: PedRef, _m: &Matrix) -> GameResult<()> { Err(GameError::Unsupported("Peds::set_ped_matrix")) }
    fn set_ped_move_speed(&mut self, ped: PedRef, v: Vec3) -> GameResult<()> {
        world::set_ped_speed(ped, v).map_err(failed("set_ped_move_speed"))
    }
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
    fn set_proofs(&mut self, ped: PedRef, p: Proofs) -> GameResult<()> {
        let b = |v: bool| Arg::Int(i32::from(v));
        cmd(0x02AB, &[Arg::Ped(ped), b(p.bullet), b(p.fire), b(p.explosion), b(p.collision), b(p.melee)])
    }
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
    fn set_decision_maker(&mut self, ped: PedRef, maker: i32) -> GameResult<()> {
        cmd(0x060B, &[Arg::Ped(ped), Arg::Int(maker)])
    }
    fn set_player_money(&mut self, money: i32) -> GameResult<()> {
        world::set_money(money).map_err(failed("set_player_money"))
    }
    fn set_player_control(&mut self, enabled: bool) -> GameResult<()> {
        cmd(0x01B4, &[Arg::Int(0), Arg::Int(i32::from(enabled))])
    }
    fn set_drunkenness(&mut self, level: i32) -> GameResult<()> {
        cmd(0x052C, &[Arg::Int(0), Arg::Int(level)])
    }
    fn set_fighting_style(&mut self, ped: PedRef, style: u8, moves: u8) -> GameResult<()> {
        cmd(0x07FE, &[Arg::Ped(ped), Arg::Int(i32::from(style)), Arg::Int(i32::from(moves))])
    }
    fn set_ped_flags(&mut self, _ped: PedRef, _mask: [u32; 2], _value: [u32; 2]) -> GameResult<()> { Err(GameError::Unsupported("Peds::set_ped_flags")) }
    fn extinguish_ped(&mut self, _ped: PedRef) -> GameResult<()> { Err(GameError::Unsupported("Peds::extinguish_ped")) }
    fn explode_head(&mut self, ped: PedRef) -> GameResult<()> {
        cmd(0x0321, &[Arg::Ped(ped)])
    }
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
            PedTask::UseMobilePhone(on) => cmd(0x0729, &[p, i(i32::from(on))]),
            _ => Err(GameError::Unsupported("Peds::start_task (this task)")),
        }
    }
    fn flush_tasks(&mut self, ped: PedRef) -> GameResult<()> {
        cmd(0x0792, &[Arg::Ped(ped)])
    }
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
    fn look_at(&mut self, ped: PedRef, target: Vec3, time_ms: i32) -> GameResult<()> {
        let f = Arg::Float;
        cmd(0x06A9, &[Arg::Ped(ped), f(target.x), f(target.y), f(target.z), Arg::Int(time_ms)])
    }
    /// On: `TASK_JETPACK` (07A7); off: the ped's tasks cleared at once (`CLEAR_CHAR_TASKS_IMMEDIATELY` 0792), which
    /// ends the jetpack task.
    fn set_jetpack(&mut self, ped: PedRef, on: bool) -> GameResult<()> {
        if on { cmd(0x07A7, &[Arg::Ped(ped)]) } else { cmd(0x0792, &[Arg::Ped(ped)]) }
    }
    /// The local player's (`IS_PLAYER_USING_JETPACK` 0A0C).
    fn has_jetpack(&self, ped: PedRef) -> GameResult<bool> {
        if self.local_player() != Some(ped) {
            return Err(GameError::Unsupported("Peds::has_jetpack (another ped)"));
        }
        cond(0x0A0C, &[Arg::Int(0)])
    }
    fn wanted_level(&self) -> GameResult<u8> {
        state::player().map(|p| p.wanted).ok_or(GameError::InvalidHandle)
    }
    fn peds(&self) -> Vec<PedRef> {
        state::peds()
    }
    fn ped_damage(&self, ped: PedRef) -> GameResult<PedDamage> {
        sa_mod::damage::last(ped).ok_or(GameError::InvalidHandle)
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
    fn model_kind(&self, model: ModelId) -> ModelKind {
        models::info(model.0).map_or(ModelKind::None, |i| i.0)
    }
    fn request_model(&mut self, model: ModelId) -> GameResult<()> {
        models::request(model.0, false).map(drop).map_err(failed("request_model"))
    }
    fn model_loaded(&self, model: ModelId) -> bool {
        models::info(model.0).is_some_and(|i| i.1)
    }
    fn release_model(&mut self, model: ModelId) -> GameResult<()> {
        models::release(model.0).map_err(failed("release_model"))
    }
    fn set_model_draw_distance(&mut self, model: ModelId, distance: f32) -> GameResult<()> {
        models::set_draw_distance(model.0, distance).map_err(failed("set_model_draw_distance"))
    }
    fn load_all_now(&mut self) -> GameResult<()> {
        cmd(0x038B, &[])
    }
    fn request_collision(&mut self, at: Vec3) -> GameResult<()> {
        cmd(0x04E4, &[Arg::Float(at.x), Arg::Float(at.y)])
    }
    fn collision_loaded(&self, _at: Vec3, _area: u8) -> bool { false }
    fn request_anim_block(&mut self, block: &str) -> GameResult<()> {
        cmd(0x04ED, &[Arg::Text(block.as_bytes())])
    }
    fn anim_block_loaded(&self, block: &str) -> bool {
        cond(0x04EE, &[Arg::Text(block.as_bytes())]).unwrap_or(false)
    }
    fn request_component(&mut self, model: ModelId) -> GameResult<()> {
        cmd(0x06E9, &[Arg::Int(model.0)])
    }
    fn component_loaded(&self, model: ModelId) -> bool {
        cond(0x06EA, &[Arg::Int(model.0)]).unwrap_or(false)
    }
    fn streaming_memory(&self) -> (u32, u32) { Default::default() }
}

impl Vehicles for ModGame {
    /// The mod API's handles are the game's script handles (pool references).
    fn vehicle_script_handle(&self, v: VehicleRef) -> GameResult<ScriptHandle> {
        state::vehicle(v).map(|_| ScriptHandle(v.0 as i32)).ok_or(GameError::InvalidHandle)
    }
    fn vehicle_from_script_handle(&self, h: ScriptHandle) -> Option<VehicleRef> {
        let v = VehicleRef(h.0 as u32);
        state::vehicle(v).map(|_| v)
    }
    /// The passengers only of the vehicle the local player rides in as one (the mod API's vehicle record has none;
    /// a seat is a script command each).
    fn vehicle_snapshot(&self, v: VehicleRef) -> GameResult<VehicleSnapshot> {
        let i = state::vehicle(v).ok_or(GameError::InvalidHandle)?;
        let m = state::vehicle_matrix(v).ok_or(GameError::InvalidHandle)?;
        let mut passengers = [None; 8];
        let me = self.local_player();
        if me.is_some() && i.driver != me && self.local_vehicle() == Some(v) {
            for (seat, p) in (0..).zip(passengers.iter_mut()) {
                *p = script::run(0x0432, &[Arg::Vehicle(v), Arg::Int(seat), Arg::Out])
                    .ok()
                    .map(|o| PedRef(o.outputs[0]));
            }
        }
        Ok(VehicleSnapshot {
            passengers,
            matrix: m.into(),
            move_speed: i.move_speed,
            health: i.health,
            model: i.model,
            driver: i.driver,
            class: world::vehicle_class(v).ok(),
            colours: [i.colour1, i.colour2],
            siren: i.siren,
            area: i.area,
            ..VehicleSnapshot::default()
        })
    }
    fn vehicle_damage(&self, v: VehicleRef) -> GameResult<VehicleDamage> {
        let [panels, doors, lights, tyres] = world::vehicle_damage(v).map_err(failed("vehicle_damage"))?;
        Ok(VehicleDamage { panels, doors, lights: lights as u8, tyres: tyres as u8 })
    }
    fn vehicle_in_water(&self, v: VehicleRef) -> GameResult<bool> {
        cond(0x02BF, &[Arg::Vehicle(v)])
    }
    fn local_vehicle(&self) -> Option<VehicleRef> {
        state::player().and_then(|p| p.ped.vehicle)
    }
    fn create_vehicle(&mut self, spawn: &VehicleSpawn) -> GameResult<VehicleRef> {
        let v = entities::create_vehicle(spawn.model.0, spawn.pos, spawn.heading_deg).map_err(failed("create_vehicle"))?;
        if spawn.colours != [MODEL_COLOURS; 2] {
            world::set_vehicle_colours(v, spawn.colours[0], spawn.colours[1]).map_err(failed("create_vehicle"))?;
        }
        Ok(v)
    }
    fn destroy_vehicle(&mut self, v: VehicleRef) -> GameResult<()> {
        entities::destroy_vehicle(v).map_err(failed("destroy_vehicle"))
    }
    /// The position (`SET_CAR_COORDINATES` 00AB) and the heading of the forward row (`SET_CAR_HEADING` 0175); the
    /// pitch and roll of the matrix are not taken.
    fn set_vehicle_matrix(&mut self, v: VehicleRef, m: &Matrix) -> GameResult<()> {
        world::set_vehicle_position(v, m.pos).map_err(failed("set_vehicle_matrix"))?;
        let heading = (-m.forward.x).atan2(m.forward.y).to_degrees();
        world::set_vehicle_heading(v, heading).map_err(failed("set_vehicle_matrix"))
    }
    fn set_vehicle_speed(&mut self, v: VehicleRef, move_speed: Vec3, turn_speed: Vec3) -> GameResult<()> {
        world::set_vehicle_speed(v, move_speed, turn_speed).map_err(failed("set_vehicle_speed"))
    }
    fn set_vehicle_heading(&mut self, v: VehicleRef, heading_deg: f32) -> GameResult<()> {
        world::set_vehicle_heading(v, heading_deg).map_err(failed("set_vehicle_heading"))
    }
    fn set_train_position(&mut self, v: VehicleRef, pos: Vec3) -> GameResult<()> {
        cmd(0x07C7, &[Arg::Vehicle(v), Arg::Float(pos.x), Arg::Float(pos.y), Arg::Float(pos.z)])
    }
    fn set_train_speed(&mut self, v: VehicleRef, speed: f32) -> GameResult<()> {
        cmd(0x06DC, &[Arg::Vehicle(v), Arg::Float(speed)])
    }
    fn set_vehicle_health(&mut self, v: VehicleRef, health: f32) -> GameResult<()> {
        world::set_vehicle_health(v, health).map_err(failed("set_vehicle_health"))
    }
    fn set_vehicle_damage(&mut self, v: VehicleRef, d: VehicleDamage) -> GameResult<()> {
        let words = [d.panels, d.doors, u32::from(d.lights), u32::from(d.tyres)];
        world::set_vehicle_damage(v, words).map_err(failed("set_vehicle_damage"))
    }
    fn fix_vehicle(&mut self, v: VehicleRef) -> GameResult<()> {
        cmd(0x0A30, &[Arg::Vehicle(v)])
    }
    fn set_vehicle_colours(&mut self, v: VehicleRef, colours: [u8; 2]) -> GameResult<()> {
        world::set_vehicle_colours(v, colours[0], colours[1]).map_err(failed("set_vehicle_colours"))
    }
    fn set_paintjob(&mut self, v: VehicleRef, paintjob: u8) -> GameResult<()> {
        cmd(0x06ED, &[Arg::Vehicle(v), Arg::Int(i32::from(paintjob))])
    }
    /// `ADD_VEHICLE_MOD` (06E7) of a loaded upgrade (`Streaming::request_component` loads it).
    fn add_component(&mut self, v: VehicleRef, component: ModelId) -> GameResult<()> {
        cmd(0x06E7, &[Arg::Vehicle(v), Arg::Int(component.0), Arg::Out])
    }
    fn remove_component(&mut self, v: VehicleRef, component: ModelId) -> GameResult<()> {
        cmd(0x06E8, &[Arg::Vehicle(v), Arg::Int(component.0)])
    }
    fn set_hydraulics(&mut self, v: VehicleRef, on: bool) -> GameResult<()> {
        cmd(0x07FF, &[Arg::Vehicle(v), Arg::Int(i32::from(on))])
    }
    fn set_vehicle_proofs(&mut self, v: VehicleRef, p: Proofs) -> GameResult<()> {
        let b = |v: bool| Arg::Int(i32::from(v));
        cmd(0x02AC, &[Arg::Vehicle(v), b(p.bullet), b(p.fire), b(p.explosion), b(p.collision), b(p.melee)])
    }
    fn set_tyres_can_burst(&mut self, v: VehicleRef, on: bool) -> GameResult<()> {
        cmd(0x053F, &[Arg::Vehicle(v), Arg::Int(i32::from(on))])
    }
    fn set_petrol_tank_weakpoint(&mut self, v: VehicleRef, on: bool) -> GameResult<()> {
        cmd(0x09C4, &[Arg::Vehicle(v), Arg::Int(i32::from(on))])
    }
    fn set_vehicle_area(&mut self, v: VehicleRef, area: AreaCode) -> GameResult<()> {
        cmd(0x0840, &[Arg::Vehicle(v), Arg::Int(i32::from(area.0))])
    }
    /// `OPEN_CAR_DOOR_A_BIT` (08A6): the door's node from the game's door table; a missing door stays as it is.
    fn set_door(&mut self, v: VehicleRef, door: u8, open_ratio: f32) -> GameResult<()> {
        cmd(0x08A6, &[Arg::Vehicle(v), Arg::Int(i32::from(door)), Arg::Float(open_ratio)])
    }
    /// `LOCK_CAR_DOORS` (020A): locked (2) or no lock (0).
    fn set_doors_locked(&mut self, v: VehicleRef, locked: bool) -> GameResult<()> {
        cmd(0x020A, &[Arg::Vehicle(v), Arg::Int(if locked { 2 } else { 0 })])
    }
    fn set_window_open(&mut self, _v: VehicleRef, _window: u8, _open: bool) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_window_open")) }
    fn set_landing_gear(&mut self, v: VehicleRef, down: bool) -> GameResult<()> {
        cmd(0x08E6, &[Arg::Vehicle(v), Arg::Int(i32::from(!down))])
    }
    fn set_siren(&mut self, v: VehicleRef, on: bool) -> GameResult<()> {
        cmd(0x0397, &[Arg::Vehicle(v), Arg::Int(i32::from(on))])
    }
    /// `SET_CAR_ENGINE_ON` (0918) and `FORCE_CAR_LIGHTS` (067F: 2 on, 1 off).
    fn set_engine_lights(&mut self, v: VehicleRef, engine: bool, lights: bool) -> GameResult<()> {
        cmd(0x0918, &[Arg::Vehicle(v), Arg::Int(i32::from(engine))])?;
        cmd(0x067F, &[Arg::Vehicle(v), Arg::Int(if lights { 2 } else { 1 })])
    }
    /// `ATTACH_TRAILER_TO_CAB` (0893) / `DETACH_TRAILER_FROM_CAB` (07AC) of the trailer attached here.
    fn set_trailer(&mut self, cab: VehicleRef, trailer: Option<VehicleRef>) -> GameResult<()> {
        if let Some(i) = self.trailers.iter().position(|(c, _)| *c == cab) {
            let (_, old) = self.trailers.swap_remove(i);
            cmd(0x07AC, &[Arg::Vehicle(old), Arg::Vehicle(cab)])?;
        }
        if let Some(t) = trailer {
            cmd(0x0893, &[Arg::Vehicle(t), Arg::Vehicle(cab)])?;
            self.trailers.push((cab, t));
        }
        Ok(())
    }
    fn set_plate_text(&mut self, _v: VehicleRef, _text: &str) -> GameResult<()> { Err(GameError::Unsupported("Vehicles::set_plate_text")) }
    fn vehicles(&self) -> Vec<VehicleRef> {
        state::vehicles()
    }
    fn create_train(&mut self, kind: i32, pos: Vec3, clockwise: bool) -> GameResult<VehicleRef> {
        sa_mod::trains::create(kind, pos, clockwise).map_err(failed("create_train"))
    }
    fn destroy_train(&mut self, first: VehicleRef) -> GameResult<()> {
        sa_mod::trains::destroy(first).map_err(failed("destroy_train"))
    }
    fn train_carriages(&self, first: VehicleRef) -> GameResult<Vec<VehicleRef>> {
        sa_mod::trains::carriages(first).map_err(failed("train_carriages"))
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
    fn force_weather(&mut self, weather: u8) -> GameResult<()> {
        cmd(0x01B6, &[Arg::Int(i32::from(weather))])
    }
    fn gravity(&self) -> f32 {
        state::world().map_or(0.0, |w| w.gravity)
    }
    fn set_gravity(&mut self, g: f32) -> GameResult<()> {
        if self.gravity().to_bits() == g.to_bits() {
            return Ok(());
        }
        world::set_gravity(g).map_err(failed("set_gravity"))
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
    fn set_stunt_bonus(&mut self, on: bool) -> GameResult<()> {
        script::set_global(samp_client::scm::VAR_STUNT_BONUS, i32::from(on)).map_err(failed("set_stunt_bonus"))
    }
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
    /// `GET_GROUND_Z_FOR_3D_COORD` (02CE); 0 when the command fails.
    fn ground_z(&self, pos: Vec3) -> f32 {
        let f = Arg::Float;
        script::run(0x02CE, &[f(pos.x), f(pos.y), f(pos.z), Arg::Out]).map_or(0.0, |o| f32::from_bits(o.outputs[0]))
    }
    fn override_restart(&mut self, pos: Vec3, heading_deg: f32) -> GameResult<()> {
        let f = Arg::Float;
        cmd(0x016E, &[f(pos.x), f(pos.y), f(pos.z), f(heading_deg)])
    }
    fn line_of_sight(&self, _from: Vec3, _to: Vec3, _filter: LineFilter) -> Option<LineHit> { None }
    fn line_clear(&self, from: Vec3, to: Vec3, f: LineFilter) -> bool {
        let (x, i) = (Arg::Float, |b: bool| Arg::Int(i32::from(b)));
        let args = [x(from.x), x(from.y), x(from.z), x(to.x), x(to.y), x(to.z)];
        let flags = [i(f.buildings), i(f.vehicles), i(f.peds), i(f.objects), i(f.dummies)];
        cond(0x06BD, &[args.as_slice(), flags.as_slice()].concat()).unwrap_or(false)
    }
    /// `ADD_EXPLOSION_VARIABLE_SHAKE` (0948): the game's explosion of `kind` with the camera shake `shake` (the
    /// command has no radius).
    fn add_explosion(&mut self, pos: Vec3, kind: i32, _radius: f32, shake: f32) -> GameResult<()> {
        let f = Arg::Float;
        cmd(0x0948, &[f(pos.x), f(pos.y), f(pos.z), Arg::Int(kind), f(shake)])
    }
    fn create_fx(&mut self, _name: &str, _on: Option<PedRef>, _offset: Vec3, _dir: Vec3) -> GameResult<FxRef> { Err(GameError::Unsupported("WorldState::create_fx")) }
    fn kill_fx(&mut self, _fx: FxRef) -> GameResult<()> { Err(GameError::Unsupported("WorldState::kill_fx")) }
    fn load_shop(&mut self, name: &str) -> GameResult<()> {
        cmd(0x075E, &[Arg::Text(name.as_bytes())])
    }
    fn disable_entry_exits(&mut self) -> GameResult<()> {
        world::set_entry_exits_enabled(false).map_err(failed("set_entry_exits_enabled"))
    }
    fn set_float_stat(&mut self, stat: u16, value: f32) -> GameResult<()> {
        world::set_float_stat(stat, value).map_err(failed("set_float_stat"))
    }
}

impl WorldEntities for ModGame {
    fn create_pickup(&mut self, model: ModelId, kind: i32, pos: Vec3) -> GameResult<PickupRef> {
        markers::create_pickup(model.0, kind, pos).map_err(failed("create_pickup"))
    }
    fn remove_pickup(&mut self, p: PickupRef) -> GameResult<()> {
        markers::destroy_pickup(p).map_err(failed("remove_pickup"))
    }
    fn pickup_collected(&self, p: PickupRef) -> GameResult<bool> {
        markers::pickup_collected(p).map_err(failed("pickup_collected"))
    }
    fn pickup_index(&self, _p: PickupRef) -> GameResult<u16> { Err(GameError::Unsupported("WorldEntities::pickup_index")) }
    fn pickup_touched(&self, p: PickupRef) -> GameResult<bool> {
        markers::pickup_touched(p).map_err(failed("pickup_touched"))
    }
    /// A race checkpoint's marker; the normal checkpoint is drawn with `locate_ped`.
    fn create_checkpoint(&mut self, kind: CheckpointKind, pos: Vec3, size: f32) -> GameResult<CheckpointRef> {
        let CheckpointKind::Race { kind, next } = kind else {
            return Err(GameError::Unsupported("WorldEntities::create_checkpoint (normal)"));
        };
        markers::create_checkpoint(i32::from(kind), pos, next, size).map_err(failed("create_checkpoint"))
    }
    fn delete_checkpoint(&mut self, c: CheckpointRef) -> GameResult<()> {
        markers::destroy_checkpoint(c).map_err(failed("delete_checkpoint"))
    }
    fn locate_ped(&mut self, ped: PedRef, pos: Vec3, radius: Vec3, marker: bool) -> GameResult<bool> {
        let f = Arg::Float;
        cond(0x00FE, &[Arg::Ped(ped), f(pos.x), f(pos.y), f(pos.z), f(radius.x), f(radius.y), f(radius.z), Arg::Int(i32::from(marker))])
    }
    fn add_blip(&mut self, target: BlipTarget) -> GameResult<BlipRef> {
        let r = match target {
            BlipTarget::Coord { pos, sprite, short_range, contact_point } => {
                let flags = if short_range { 0 } else { sa_mod::blip::GLOBAL }
                    | if contact_point { sa_mod::blip::CONTACT_POINT } else { 0 };
                markers::create_coord_blip(pos, sprite, flags)
            }
            BlipTarget::Ped(p) => markers::create_ped_blip(p),
            BlipTarget::Vehicle(v) => markers::create_vehicle_blip(v),
        };
        r.map_err(failed("add_blip"))
    }
    fn remove_blip(&mut self, b: BlipRef) -> GameResult<()> {
        markers::destroy_blip(b).map_err(failed("remove_blip"))
    }
    fn set_blip_colour(&mut self, b: BlipRef, colour: u32) -> GameResult<()> {
        markers::set_blip_colour(b, colour).map_err(failed("set_blip_colour"))
    }
    fn set_blip_scale(&mut self, b: BlipRef, scale: i32) -> GameResult<()> {
        markers::set_blip_scale(b, scale).map_err(failed("set_blip_scale"))
    }
    fn set_blip_display(&mut self, b: BlipRef, display: i32) -> GameResult<()> {
        markers::set_blip_display(b, display).map_err(failed("set_blip_display"))
    }
    fn show_radar_area(&mut self, id: u16, area: &RadarArea) -> GameResult<()> {
        radar::set_area(id, area.min, area.max, sa_mod::Rgba(area.colour)).map_err(failed("show_radar_area"))?;
        if let Some((colour, period)) = area.flash {
            radar::flash_area(id, sa_mod::Rgba(colour), period).map_err(failed("show_radar_area"))?;
        }
        Ok(())
    }
    fn hide_radar_area(&mut self, id: u16) -> GameResult<()> {
        radar::remove_area(id).map_err(failed("hide_radar_area"))
    }
}
