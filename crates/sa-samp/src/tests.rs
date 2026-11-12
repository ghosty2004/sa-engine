//! Host tests of the read side over a fake memory laid out like the engine's.

use sa_samp_api::camera::Camera;
use sa_samp_api::hooks::{FramePhase, GameHooks};
use sa_samp_api::hud::Hud;
use sa_samp_api::input::Input;
use sa_samp_api::objects::Objects;
use sa_samp_api::peds::Peds;
use sa_samp_api::script::{Opcode, ScriptArg, ScriptCommands, ScriptLocals};
use sa_samp_api::vehicles::{VehicleClass, Vehicles};
use sa_samp_api::world::WorldState;
use sa_samp_api::*;

use crate::addr::{self, off};
use crate::view::{FakeView, View};
use crate::{NativeGame, script};

const PED_HEADER: u32 = 0x0100_0000;
const PED_MAP: u32 = 0x0100_1000;
const PEDS: u32 = 0x0110_0000;
const VEH_HEADER: u32 = 0x0120_0000;
const VEH_MAP: u32 = 0x0120_1000;
const VEHS: u32 = 0x0130_0000;
const OBJ_HEADER: u32 = 0x0140_0000;
const OBJ_MAP: u32 = 0x0140_1000;
const OBJS: u32 = 0x0150_0000;
const MATRIX: u32 = 0x0160_0000;

fn put<T: Copy>(g: &mut NativeGame<FakeView>, a: u32, v: T) {
    // SAFETY: test values are plain integers / floats.
    let bytes = unsafe { std::slice::from_raw_parts((&raw const v).cast::<u8>(), size_of::<T>()) };
    g.view.write(a, bytes);
}

fn pool(g: &mut NativeGame<FakeView>, global: u32, header: u32, objects: u32, map: u32, size: i32) {
    put(g, global, header);
    put(g, header, objects);
    put(g, header + 4, map);
    put(g, header + 8, size);
    for i in 0..size as u32 {
        put(g, map + i, 0x80u8);
    }
}

/// A game with the three pools and the local player in ped slot 2 (reuse counter 5).
fn game() -> NativeGame<FakeView> {
    let mut g = NativeGame::with_executor(FakeView::default(), |_| 0);
    pool(&mut g, addr::PED_POOL.global, PED_HEADER, PEDS, PED_MAP, 140);
    pool(&mut g, addr::VEHICLE_POOL.global, VEH_HEADER, VEHS, VEH_MAP, 110);
    pool(&mut g, addr::OBJECT_POOL.global, OBJ_HEADER, OBJS, OBJ_MAP, 350);
    put(&mut g, PED_MAP + 2, 0x05u8);
    put(&mut g, addr::PLAYERS, PEDS + 2 * addr::PED_POOL.element);
    g
}

/// The local player's ped (slot 2 of the ped pool).
const LOCAL: u32 = PEDS + 2 * 0x7C4;

#[test]
fn reports_the_native_backend() {
    let g = game();
    let info = g.backend();
    assert_eq!(info.kind, BackendKind::EngineNative);
    assert_eq!(info.api_version, API_VERSION);
    assert!(!info.has_original_layout);
}

#[test]
fn reads_game_state_timer_and_quit() {
    let mut g = game();
    for (raw, state) in [
        (5, GameState::Starting(5)),
        (7, GameState::FrontendIdle),
        (8, GameState::LoadingStarted),
        (9, GameState::Playing),
        (42, GameState::Other(42)),
    ] {
        put(&mut g, addr::GAME_STATE, raw);
        assert_eq!(g.game_state(), state);
    }
    put(&mut g, addr::TIME_STEP, 1.5f32);
    put(&mut g, addr::TIME_STEP_NON_CLIPPED, 2.5f32);
    put(&mut g, addr::TIME_IN_MS, 123_456u32);
    assert_eq!(
        g.time_step(),
        TimeStep {
            clipped: 1.5,
            non_clipped: 2.5
        }
    );
    assert_eq!(g.time_step_clipped(), 1.5);
    assert_eq!(g.game_time_ms(), 123_456);
    g.request_quit();
    assert_eq!(g.view.read_u32(addr::RS_QUIT), 1);
}

#[test]
fn handles_are_pool_references() {
    let mut g = game();
    let me = g.local_player().expect("local player");
    assert_eq!(me, PedRef((2 << 8) | 5));
    assert_eq!(g.player_ped(PlayerSlot(0)), Some(me));
    assert_eq!(g.player_ped(PlayerSlot(1)), None);
    assert_eq!(g.player_ped(PlayerSlot(2)), None);
    assert_eq!(g.ped_script_handle(me), Ok(ScriptHandle(0x205)));
    assert_eq!(g.ped_from_script_handle(ScriptHandle(0x205)), Some(me));
    // stale reuse counter, free slot, out of range, misaligned pointer
    assert_eq!(g.ped_from_script_handle(ScriptHandle(0x206)), None);
    assert_eq!(g.ped_script_handle(PedRef(0x305)), Err(GameError::InvalidHandle));
    assert_eq!(g.ped_from_script_handle(ScriptHandle(1000 << 8)), None);
    assert_eq!(g.ped_from_script_handle(ScriptHandle(-1)), None);
    put(&mut g, addr::PLAYERS, LOCAL + 4);
    assert_eq!(g.local_player(), None);
    // the slot is freed: the old handle stops resolving
    let mut g = game();
    put(&mut g, PED_MAP + 2, 0x85u8);
    assert_eq!(g.ped_script_handle(me), Err(GameError::InvalidHandle));
    assert_eq!(g.local_player(), None);
}

#[test]
fn reads_focus_money_and_models() {
    let mut g = game();
    put(&mut g, addr::PLAYER_IN_FOCUS, 3u8);
    put(&mut g, addr::PLAYER_MONEY, -25i32);
    assert_eq!(g.player_in_focus(), PlayerSlot(3));
    assert_eq!(g.player_money(), -25);
    let info = 0x0170_0000u32;
    put(&mut g, addr::MODEL_INFO_PTRS + 4 * 7, info);
    put(&mut g, info, addr::VTBL_PED_MODEL_INFO);
    assert!(g.is_ped_model(ModelId(7)));
    assert!(!g.is_ped_model(ModelId(8)));
    assert!(!g.is_ped_model(ModelId(-1)));
    assert!(!g.is_ped_model(ModelId(20_000)));
}

#[test]
fn ped_snapshot_reads_the_ped_fields() {
    let mut g = game();
    let p = LOCAL;
    put(&mut g, p + off::VTABLE, 0x86D168u32);
    put(&mut g, p + off::MOVING_LIST, 0x1234u32);
    put(&mut g, p + off::MATRIX, MATRIX);
    for (i, f) in [
        1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2488.5, -1666.8, 13.4,
    ]
    .into_iter()
    .enumerate()
    {
        put(&mut g, MATRIX + 4 * i as u32, f);
    }
    put(&mut g, p + off::MOVE_SPEED, 0.25f32);
    put(&mut g, p + off::PED_HEALTH, 100.0f32);
    put(&mut g, p + off::PED_ARMOUR, 50.0f32);
    put(&mut g, p + off::PED_HEADING, 1.25f32);
    put(&mut g, p + off::PED_HEADING_GOAL, 1.5f32);
    put(&mut g, p + off::PED_STATE, 1u32);
    put(&mut g, p + off::MODEL, 0i16);
    put(&mut g, p + off::AREA, 3u8);
    put(&mut g, p + off::PED_WEAPON_SLOT, 2u8);
    let w2 = p + off::PED_WEAPONS + 2 * off::WEAPON_SIZE;
    put(&mut g, w2, 24u32);
    put(&mut g, w2 + 4, 1u32);
    put(&mut g, w2 + 8, 7u32);
    put(&mut g, w2 + 0xC, 70u32);
    let me = g.local_player().unwrap();
    let s = g.ped_snapshot(me).unwrap();
    assert_eq!(s.matrix.pos, Vec3::new(2488.5, -1666.8, 13.4));
    assert_eq!(s.matrix.up, Vec3::new(0.0, 0.0, 1.0));
    assert_eq!(s.move_speed.x, 0.25);
    assert_eq!(
        (s.health, s.armour, s.heading, s.heading_goal),
        (100.0, 50.0, 1.25, 1.5)
    );
    assert_eq!((s.ped_state, s.model, s.area), (1, ModelId(0), AreaCode(3)));
    assert_eq!((s.weapon_slot, s.weapon), (2, WeaponType(24)));
    assert_eq!(s.vehicle, None);
    assert!(s.in_world);
    assert!(g.ped_in_world(me));
    let w = g.ped_weapons(me).unwrap();
    assert_eq!(
        (w[2].weapon, w[2].state, w[2].ammo_in_clip, w[2].ammo_total),
        (24, 1, 7, 70)
    );
    // a destroyed entity keeps CPlaceable's vtable
    put(&mut g, p + off::VTABLE, addr::VTBL_PLACEABLE);
    assert!(!g.ped_in_world(me));
}

#[test]
fn placement_is_used_without_a_matrix() {
    let mut g = game();
    let p = LOCAL;
    put(&mut g, p + off::PLACEMENT_POS, 10.0f32);
    put(&mut g, p + off::PLACEMENT_POS + 4, 20.0f32);
    put(&mut g, p + off::PLACEMENT_HEADING, 0.0f32);
    let s = g.ped_snapshot(g.local_player().unwrap()).unwrap();
    assert_eq!(s.matrix.pos, Vec3::new(10.0, 20.0, 0.0));
    assert_eq!(s.matrix.right, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(s.matrix.forward, Vec3::new(0.0, 1.0, 0.0));
}

#[test]
fn vehicles_resolve_through_their_pool() {
    let mut g = game();
    let v = VEHS + 4 * addr::VEHICLE_POOL.element;
    put(&mut g, VEH_MAP + 4, 0x01u8);
    put(&mut g, v + off::VTABLE, 0x871120u32);
    put(&mut g, v + off::VEH_HEALTH, 1000.0f32);
    put(&mut g, v + off::MODEL, 411i16);
    put(&mut g, v + off::VEH_DRIVER, LOCAL);
    put(&mut g, v + off::VEH_MAX_PASSENGERS, 1u8);
    put(&mut g, v + off::VEH_COLOURS, [6u8, 1u8]);
    put(&mut g, v + off::VEH_SIREN_BYTE, 0x80u8);
    let p = LOCAL;
    // not in a vehicle yet
    put(&mut g, p + off::PED_VEHICLE, v);
    assert_eq!(g.local_vehicle(), None);
    put(&mut g, p + off::PED_FLAGS, off::PED_IN_VEHICLE);
    let veh = g.local_vehicle().expect("local vehicle");
    assert_eq!(veh, VehicleRef((4 << 8) | 1));
    assert_eq!(g.vehicle_script_handle(veh), Ok(ScriptHandle(0x401)));
    assert_eq!(g.vehicle_from_script_handle(ScriptHandle(0x401)), Some(veh));
    let s = g.vehicle_snapshot(veh).unwrap();
    assert_eq!(s.class, Some(VehicleClass::Automobile));
    assert_eq!(
        (s.health, s.model, s.max_passengers, s.colours),
        (1000.0, ModelId(411), 1, [6, 1])
    );
    assert_eq!(s.driver, g.local_player());
    assert_eq!(s.passengers, [None; 8]);
    assert!(s.siren);
    assert_eq!(
        g.ped_snapshot(g.local_player().unwrap()).unwrap().vehicle,
        Some(veh)
    );
}

#[test]
fn objects_resolve_through_their_pool() {
    let mut g = game();
    let o = OBJS + 9 * addr::OBJECT_POOL.element;
    put(&mut g, OBJ_MAP + 9, 0x02u8);
    put(&mut g, o + off::RW_OBJECT, 0xABCDu32);
    let h = g.object_from_script_handle(ScriptHandle(0x902)).expect("object");
    assert_eq!(g.object_script_handle(h), Ok(ScriptHandle(0x902)));
    assert_eq!(g.object_has_rw(h), Ok(true));
    assert_eq!(g.object_matrix(h).unwrap().pos, Vec3::default());
    assert_eq!(g.object_from_script_handle(ScriptHandle(0x903)), None);
}

#[test]
fn reads_world_input_hud_and_camera() {
    let mut g = game();
    put(&mut g, addr::CLOCK_HOURS, 21u8);
    put(&mut g, addr::CLOCK_MINUTES, 7u8);
    put(&mut g, addr::WEATHER_FORCED, -1i16);
    put(&mut g, addr::WEATHER_NEW, 10i16);
    put(&mut g, addr::WEATHER_OLD, 3i16);
    put(&mut g, addr::GAME_GRAVITY, 0.008f32);
    put(&mut g, addr::CURR_AREA, 18i32);
    assert_eq!(g.clock(), (21, 7));
    assert_eq!(g.weather(), (-1, 10, 3));
    assert_eq!(g.gravity(), 0.008);
    assert_eq!((g.area(), g.area_raw()), (AreaCode(18), 18));

    put(&mut g, addr::NEW_KEY_STATE + 0x20, 255u16);
    assert!(g.game_key_down(0x20));
    assert!(!g.game_key_down(0x22));
    put(&mut g, addr::PAD0, -128i16);
    put(&mut g, addr::PAD0 + 2 * 15, 255i16);
    put(&mut g, addr::PAD0 + 2 * 21, 255i16);
    put(&mut g, addr::PAD0 + 0x30 + 2 * 16, 255i16);
    let (new, old) = g.pad();
    assert_eq!((new.left_x, new.face[1], new.ped_walk), (-128, 255, 255));
    assert_eq!(old.face[2], 255);
    put(&mut g, addr::NEW_MOUSE, 1u8);
    put(&mut g, addr::NEW_MOUSE + 0xC, 3.5f32);
    put(&mut g, addr::NEW_MOUSE + 0x10, -2.0f32);
    let m = g.mouse();
    assert_eq!((m.dx, m.dy, m.buttons[0], m.buttons[1]), (3.5, -2.0, true, false));
    put(&mut g, addr::RS_MAX_WIDTH, 1280u32);
    put(&mut g, addr::RS_MAX_HEIGHT, 720u32);
    assert_eq!(g.screen_size(), (1280, 720));

    assert!(!g.menu_active());
    put(&mut g, addr::MENU_ACTIVE, 0x0100_0001u32);
    assert!(g.menu_active());
    assert_eq!(g.menu_active_dword(), 0x0100_0001);

    put(&mut g, addr::camera::MATRIX + 0x30, 1.0f32);
    put(&mut g, addr::camera::FRONT + 4, 1.0f32);
    put(&mut g, addr::camera::MODE, 4u16);
    put(&mut g, addr::camera::FOV, 70.0f32);
    put(&mut g, addr::camera::LOOKING_RIGHT, 1u8);
    put(&mut g, addr::camera::ASPECT_RATIO, 16.0f32 / 9.0);
    put(&mut g, addr::camera::WEAPON_MODE, 7u16);
    let c = g.camera();
    assert_eq!((c.matrix.pos.x, c.front.y, c.mode, c.fov), (1.0, 1.0, 4, 70.0));
    assert_eq!((c.looking_left, c.looking_right, c.weapon_mode), (false, true, 7));
    assert_eq!(c.aspect_ratio, 16.0 / 9.0);
}

#[test]
fn encodes_script_commands_like_samp() {
    let mut b = Vec::new();
    script::encode(Opcode(0x01B4), &[ScriptArg::Int(0), ScriptArg::Int(1)], &mut b);
    assert_eq!(b, [0xB4, 0x01, 0x01, 0, 0, 0, 0, 0x01, 1, 0, 0, 0]);
    script::encode(
        Opcode(0x04E4),
        &[ScriptArg::Float(1.0), ScriptArg::Local(3)],
        &mut b,
    );
    assert_eq!(b, [0xE4, 0x04, 0x06, 0x00, 0x00, 0x80, 0x3F, 0x03, 3, 0]);
    script::encode(
        Opcode(0x04EE),
        &[ScriptArg::Text(b"BAR"), ScriptArg::Global(0xB14)],
        &mut b,
    );
    assert_eq!(b, [0xEE, 0x04, 0x0E, 3, b'B', b'A', b'R', 0x02, 0x14, 0x0B]);
}

/// Stands in for `ProcessOneCommand`: checks the instruction the IP points at, stores local 0 + 1 into
/// local 1 and sets the condition flag.
fn fake_exec(s: sa_core::mem::GPtr<u8>) -> i8 {
    let p = s.raw();
    // SAFETY: `s` is the thread buffer of `Threads::run`, alive for the call; on a 64-bit host the IP
    // fields hold a truncated address, so the test reads the code through the thread's own fields only.
    unsafe {
        let local0 = (p.add(script::layout::LOCAL_VARS) as *const u32).read_unaligned();
        (p.add(script::layout::LOCAL_VARS + 4) as *mut u32).write_unaligned(local0 + 1);
        *p.add(script::layout::COND_RESULT) = 1;
    }
    0
}

#[test]
fn script_threads_keep_their_locals() {
    let mut g = NativeGame::with_executor(FakeView::default(), fake_exec);
    let t = g.create_script_thread().unwrap();
    g.set_thread_local(t, 0, 41).unwrap();
    assert_eq!(
        g.run_on_thread(t, Opcode(0x0470), &[ScriptArg::Int(1), ScriptArg::Local(1)]),
        Ok(true)
    );
    assert_eq!(g.thread_local(t, 1), Ok(42));
    assert_eq!(g.last_command(), (Opcode(0x0470), false));
    assert_eq!(g.thread_local(t, 32), Err(GameError::InvalidHandle));
    assert!(
        g.run_on_thread(sa_samp_api::script::ScriptThreadRef(9), Opcode(1), &[])
            .is_err()
    );
    let mut locals = ScriptLocals::default();
    locals.0[0] = 6;
    assert_eq!(g.run_command(Opcode(0x0470), &[], &mut locals), Ok(true));
    assert_eq!(locals.int(1), 7);
    // A destroyed thread is gone and its slot is reused; the one-shot thread survives a destroy.
    g.destroy_script_thread(t);
    assert_eq!(g.thread_local(t, 0), Err(GameError::InvalidHandle));
    let u = g.create_script_thread().unwrap();
    assert_eq!((u, g.thread_local(u, 0)), (t, Ok(0)));
    g.destroy_script_thread(sa_samp_api::script::ScriptThreadRef(1));
    assert_eq!(g.run_command(Opcode(0x0470), &[], &mut locals), Ok(true));
}

type Seen = std::sync::Arc<std::sync::Mutex<Vec<(FramePhase, Option<PedRef>)>>>;

#[derive(Default)]
struct Recorder(Seen);

impl GameHooks for Recorder {
    fn frame(&mut self, phase: FramePhase, game: &mut dyn GameAdapter) {
        self.0.lock().unwrap().push((phase, game.local_player()));
    }
}

#[test]
fn install_and_frame_dispatch() {
    let mut g = game();
    g.dispatch_frame(FramePhase::Before2d);
    let rec = Recorder::default();
    let seen = rec.0.clone();
    let cfg = sa_samp_api::config::SampModeConfig::default();
    g.install(&cfg, Box::new(rec), None).unwrap();
    assert!(g.installed());
    assert!(g.install(&cfg, Box::new(Recorder::default()), None).is_err());
    g.dispatch_frame(FramePhase::BeforeGameProcess);
    g.dispatch_frame(FramePhase::Before2d);
    let me = g.local_player();
    assert_eq!(
        *seen.lock().unwrap(),
        [(FramePhase::BeforeGameProcess, me), (FramePhase::Before2d, me)]
    );
}

/// The encoded instruction the backend issued last (`script::encode`'s format).
fn issued(g: &NativeGame<FakeView>, back: usize) -> Vec<u8> {
    let v = &g.threads.issued;
    v[v.len() - 1 - back].clone()
}

fn enc(op: u16, args: &[ScriptArg<'_>]) -> Vec<u8> {
    let mut b = Vec::new();
    script::encode(Opcode(op), args, &mut b);
    b
}

#[test]
fn ped_writes_go_through_opcodes_and_fields() {
    use ScriptArg::{Float as F, Int as I};
    let mut g = game();
    let me = g.local_player().unwrap();
    let h = 0x205;
    g.set_ped_position(me, Vec3::new(1.0, 2.0, 3.0)).unwrap();
    assert_eq!(issued(&g, 0), enc(0x00A1, &[I(h), F(1.0), F(2.0), F(3.0)]));
    g.set_ped_heading(me, 90.0, false).unwrap();
    assert_eq!(issued(&g, 0), enc(0x0173, &[I(h), F(90.0)]));
    g.set_ped_heading(me, 1.5, true).unwrap();
    assert_eq!(g.view.read_f32(LOCAL + off::PED_HEADING), 1.5);
    assert_eq!(g.view.read_f32(LOCAL + off::PED_HEADING_GOAL), 1.5);
    g.set_ped_health(me, 55.0).unwrap();
    g.set_ped_armour(me, 20.0).unwrap();
    assert_eq!(g.ped_snapshot(me).map(|s| (s.health, s.armour)), Ok((55.0, 20.0)));
    g.give_weapon(me, WeaponType(24), 100).unwrap();
    assert_eq!(issued(&g, 0), enc(0x01B2, &[I(h), I(24), I(100)]));
    g.set_current_weapon(me, WeaponType(24)).unwrap();
    g.clear_weapons(me).unwrap();
    assert_eq!(issued(&g, 1), enc(0x01B9, &[I(h), I(24)]));
    assert_eq!(issued(&g, 0), enc(0x048F, &[I(h)]));
    g.set_ped_area(me, AreaCode(17)).unwrap();
    assert_eq!(issued(&g, 0), enc(0x0860, &[I(h), I(17)]));
    g.set_player_control(false).unwrap();
    assert_eq!(issued(&g, 0), enc(0x01B4, &[I(0), I(0)]));
    g.set_player_money(5000).unwrap();
    assert_eq!(g.player_money(), 5000);
    // A stale ped issues nothing.
    let n = g.threads.issued.len();
    assert_eq!(
        g.set_ped_position(PedRef(0x305), Vec3::default()),
        Err(GameError::InvalidHandle)
    );
    assert_eq!(g.threads.issued.len(), n);
}

#[test]
fn world_hud_camera_and_streaming_writes() {
    use ScriptArg::{Float as F, Int as I};
    let mut g = game();
    g.set_clock(22, 30).unwrap();
    assert_eq!(g.clock(), (22, 30));
    assert_eq!(issued(&g, 0), enc(0x00C0, &[I(22), I(30)]));
    g.set_weather(8).unwrap();
    assert_eq!(g.weather(), (8, 8, 8));
    g.force_weather(10).unwrap();
    assert_eq!(g.weather(), (10, 8, 8));
    put(&mut g, addr::GAME_GRAVITY, 0.008f32);
    assert_eq!(g.set_gravity(0.008), Ok(()));
    assert!(g.set_gravity(0.005).is_err());
    assert_eq!(g.gravity(), 0.008);
    g.set_area(AreaCode(3)).unwrap();
    assert_eq!(issued(&g, 0), enc(0x04BB, &[I(3)]));
    g.set_hud_visible(false).unwrap();
    assert_eq!(
        (
            g.view.read_u8(addr::MENU_HUD_ON),
            g.view.read_u8(addr::HUD_DONT_DISPLAY_RADAR)
        ),
        (0, 1)
    );
    g.set_zone_names(false).unwrap();
    assert_eq!(issued(&g, 0), enc(0x09BA, &[I(0)]));
    g.add_score(-250).unwrap();
    assert_eq!(issued(&g, 0), enc(0x0109, &[I(0), I(-250)]));
    g.point_at(Vec3::new(1.0, 2.0, 3.0), sa_samp_api::camera::CameraCut::Cut)
        .unwrap();
    assert_eq!(issued(&g, 0), enc(0x0160, &[F(1.0), F(2.0), F(3.0), I(2)]));
    g.behind_player().unwrap();
    assert_eq!(issued(&g, 0), enc(0x0373, &[]));
    use sa_samp_api::streaming::Streaming;
    g.request_model(ModelId(560)).unwrap();
    assert_eq!(issued(&g, 0), enc(0x0247, &[I(560)]));
    assert!(!g.model_loaded(ModelId(560)));
    put(
        &mut g,
        addr::STREAMING_INFO + 560 * addr::STREAMING_INFO_SIZE + 0x10,
        1u8,
    );
    assert!(g.model_loaded(ModelId(560)));
    assert!(!g.model_loaded(ModelId(-1)));
}

/// Stands in for CREATE_CAR / CREATE_OBJECT: stores a handle into local 0.
fn create_exec(s: sa_core::mem::GPtr<u8>) -> i8 {
    // SAFETY: the thread buffer of `Threads::run`, alive for the call.
    unsafe {
        (s.raw().add(script::layout::LOCAL_VARS) as *mut u32).write_unaligned((3 << 8) | 1);
    }
    0
}

#[test]
fn created_entities_come_back_as_handles() {
    use ScriptArg::{Float as F, Int as I, Local as L};
    let mut g = game();
    g.threads.exec = create_exec;
    put(&mut g, VEH_MAP + 3, 0x01u8);
    let spawn = sa_samp_api::vehicles::VehicleSpawn {
        model: ModelId(560),
        pos: Vec3::new(1.0, 2.0, 3.0),
        heading_deg: 90.0,
        colours: [1, 2],
        paintjob: 3,
        area: AreaCode(0),
    };
    assert_eq!(g.create_vehicle(&spawn), Ok(VehicleRef(0x301)));
    assert_eq!(
        issued(&g, 2),
        enc(0x00A5, &[I(560), F(1.0), F(2.0), F(3.0), L(0)])
    );
    assert_eq!(issued(&g, 1), enc(0x0175, &[I(0x301), F(90.0)]));
    assert_eq!(issued(&g, 0), enc(0x0229, &[I(0x301), I(1), I(2)]));
    g.destroy_vehicle(VehicleRef(0x301)).unwrap();
    assert_eq!(issued(&g, 0), enc(0x00A6, &[I(0x301)]));
    put(&mut g, OBJ_MAP + 3, 0x01u8);
    assert_eq!(
        g.create_object(ModelId(1337), Vec3::new(1.0, 2.0, 3.0)),
        Ok(ObjectRef(0x301))
    );
    // A creation the engine refused (slot free) is reported, not a wild handle.
    put(&mut g, OBJ_MAP + 3, 0x81u8);
    assert_eq!(
        g.create_object(ModelId(1337), Vec3::default()),
        Err(GameError::NotLoaded)
    );
}

#[test]
fn feature_bits_match_the_api() {
    use sa_core::samp::feature as f;
    use sa_samp_api::config::Feature;
    for (bit, feat) in [
        (f::AMBIENT_PEDS, Feature::AmbientPeds),
        (f::RANDOM_TRAFFIC, Feature::RandomTraffic),
        (f::CAR_GENERATORS, Feature::CarGenerators),
        (f::AMBIENT_PLANES, Feature::AmbientPlanes),
        (f::CUTSCENES, Feature::Cutscenes),
        (f::REPLAY, Feature::Replay),
        (f::PLANT_MANAGER, Feature::PlantManager),
        (f::SUN_MOON_CORONAS, Feature::SunMoonCoronas),
        (f::IDLE_CAMERA, Feature::IdleCamera),
        (f::HELP_MESSAGES, Feature::HelpMessages),
        (f::ZONE_NAMES, Feature::ZoneNames),
    ] {
        assert_eq!(bit, feat as u8, "{feat:?}");
    }
}
