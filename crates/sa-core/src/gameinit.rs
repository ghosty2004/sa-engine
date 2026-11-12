//! STD-4 (load lane): `CGame::Initialise` (0x53BC80) and its `Init1` / `Init2` / `Init3` stages, with
//! the small initialisers they reach that had no port. Ported from the 1.0 US bytes (gta-reversed
//! `Game.cpp` as the reading aid; the bytes win where they differ). Every callee goes through
//! `orig!`, so each keeps its own hook toggle.

use sa_math::x87::X87;

use crate::abi::{HookReg, NativeReg};
use crate::mem::{GPtr, GlobalDesc};
use crate::{hook, orig};

fn g<T: crate::mem::GameData>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}
fn p8(a: u32) -> GPtr<u8> {
    GPtr::from_addr(a as usize)
}

/// "Loading the Game" (0x863A6C), the loading-screen title of every stage.
const LOADING_THE_GAME: u32 = 0x863A6C;

orig! {
    /// `CGame::Init1(datFile)` (0x5BF840)
    pub fn init1_call(dat: GPtr<u8>) -> u8 = cdecl 0x5BF840 => init1;
    /// `CGame::Init2(datFile)` (0x5BA1A0)
    pub fn init2_call(dat: GPtr<u8>) -> u8 = cdecl 0x5BA1A0 => init2;
    /// `CGame::Init3(datFile)` (0x5BA400)
    pub fn init3_call(dat: GPtr<u8>) -> u8 = cdecl 0x5BA400 => init3;
    /// `CColAccel::startCache()` (0x5B31A0)
    pub fn col_accel_start_cache() = cdecl 0x5B31A0 => unported;
    /// `CColAccel::endCache()` (0x5B2AD0)
    pub fn col_accel_end_cache() = cdecl 0x5B2AD0 => unported;
    /// `CFileLoader::LoadLevel(path)` (0x5B9030)
    pub fn load_level(path: GPtr<u8>) = cdecl 0x5B9030 => unported;
    /// `CStencilShadows::Init()` (0x70F9E0)
    pub fn stencil_shadows_init() = cdecl 0x70F9E0 => unported;
    /// `CLoadingScreen::LoadingScreen(title, message, unused)` (0x53DED0)
    pub fn loading_screen(title: u32, msg: u32, unused: u32) = cdecl 0x53DED0 => unported;
    /// `CTheScripts::StartTestScript()` (0x464D40)
    pub fn start_test_script() = cdecl 0x464D40 => unported;
    /// `CTheScripts::Process()` (0x46A000)
    pub fn scripts_process() = cdecl 0x46A000 => unported;
    /// `CCamera::Process()` (0x52B730)
    pub fn camera_process(this: GPtr<u8>) = thiscall 0x52B730 => unported;

    // ---- Init1
    pub fn init_maths_tables_call() = cdecl 0x59AC90 => init_maths_tables;
    pub fn pools_initialise() = cdecl 0x550F10 => unported;
    pub fn placeable_init_matrix_array() = cdecl 0x54F3A0 => unported;
    pub fn ini_load() = cdecl 0x56D070 => unported;
    pub fn set_use_d3d_resource_buffering_call(on: u32) = cdecl 0x730AC0 => set_use_d3d_resource_buffering;
    pub fn txd_add_slot(name: u32) -> i32 = cdecl 0x731C80 => unported;
    pub fn txd_create(slot: i32) = cdecl 0x731990 => unported;
    pub fn txd_add_ref(slot: i32) = cdecl 0x731A00 => unported;
    pub fn txd_load_txd(slot: i32, path: u32) -> u8 = cdecl 0x7320B0 => unported;
    pub fn txd_set_current(slot: i32) = cdecl 0x7319C0 => unported;
    pub fn txd_find_slot(name: u32) -> i32 = cdecl 0x731850 => unported;
    pub fn txd_push_current() = cdecl 0x7316A0 => unported;
    pub fn txd_pop_current() = cdecl 0x7316B0 => unported;
    pub fn game_logic_init_at_start() = cdecl 0x441210 => unported;
    pub fn gang_wars_init_at_start() = cdecl 0x443920 => unported;
    pub fn conversations_clear_call() = cdecl 0x43A7B0 => conversations_clear;
    pub fn ped_to_player_conversations_clear() = cdecl 0x43AAE0 => unported;
    pub fn quad_tree_init_pool() = cdecl 0x552C00 => unported;
    pub fn plant_mgr_initialise_call() -> u8 = cdecl 0x5DD910 => plant_mgr_initialise;
    pub fn custom_roadsign_mgr_initialise_call() -> u8 = cdecl 0x6FE120 => custom_roadsign_mgr_initialise;
    pub fn references_init() = cdecl 0x5719B0 => unported;
    pub fn debug_init_text_buffer() = cdecl 0x532240 => unported;
    pub fn tag_manager_init() = cdecl 0x49CC50 => unported;
    pub fn weather_init() = cdecl 0x72A480 => unported;
    pub fn cover_init_call() = cdecl 0x698710 => cover_init;
    pub fn cull_zones_init() = cdecl 0x72D6B0 => unported;
    pub fn occlusion_init() = cdecl 0x71DCA0 => unported;
    pub fn collision_init() = cdecl 0x416260 => unported;
    pub fn birds_init_call() = cdecl 0x711EC0 => birds_init;
    pub fn entry_exit_manager_init_call() = cdecl 0x43F880 => entry_exit_manager_init;
    pub fn stunt_jump_manager_init() = cdecl 0x49CA50 => unported;
    pub fn set_pieces_init() = cdecl 0x4994F0 => unported;
    pub fn the_zones_init() = cdecl 0x572670 => unported;
    pub fn user_display_init() = cdecl 0x571EE0 => unported;
    pub fn messages_init() = cdecl 0x69EE00 => unported;
    pub fn messages_clear_all_displayed_by_game(flag: u32) = cdecl 0x69EDC0 => unported;
    pub fn vehicle_recording_init() = cdecl 0x459390 => unported;
    pub fn restart_initialise() = cdecl 0x460630 => unported;
    pub fn world_initialise() = cdecl 0x5631E0 => unported;
    pub fn anim_manager_initialise() = cdecl 0x5BF6B0 => unported;
    pub fn cutscene_mgr_initialise() = cdecl 0x4D5A20 => unported;
    pub fn car_ctrl_init() = cdecl 0x4212E0 => unported;
    pub fn init_model_indices() = cdecl 0x5BCA10 => unported;
    pub fn model_info_initialise() = cdecl 0x4C6810 => unported;
    pub fn pickups_init() = cdecl 0x454A70 => unported;
    pub fn the_car_generators_init() = cdecl 0x6F3270 => unported;
}
orig! {
    pub fn garages_init() = cdecl 0x447120 => unported;
    pub fn audio_zones_init() = cdecl 0x5081A0 => unported;
    pub fn streaming_init_image_list() = cdecl 0x4083C0 => unported;
    pub fn streaming_read_ini_file() = cdecl 0x5BCCD0 => unported;
    pub fn path_find_init(this: GPtr<u8>) = thiscall 0x44D080 => unported;
    pub fn path_find_allocate_info_mem(this: GPtr<u8>) = thiscall 0x44D2B0 => unported;
    pub fn task_simple_fight_load_melee_data() = cdecl 0x5BEDC0 => unported;
    pub fn cheat_reset_cheats() = cdecl 0x438450 => unported;
    pub fn fx_init(this: GPtr<u8>) = thiscall 0x49EA90 => unported;

    // ---- Init2
    pub fn the_zones_post_zone_creation() = cdecl 0x572B70 => unported;
    pub fn entry_exit_post_creation() = cdecl 0x43F0A0 => unported;
    pub fn path_find_prepare_path_data(this: GPtr<u8>) = thiscall 0x44D0E0 => unported;
    pub fn player_info_clear(this: GPtr<u8>) = thiscall 0x56F330 => unported;
    pub fn player_info_load_player_skin(this: GPtr<u8>) = thiscall 0x56F7D0 => unported;
    pub fn test_model_indices() = cdecl 0x53ED30 => unported;
    pub fn clothes_init() = cdecl 0x5A80D0 => unported;
    pub fn water_level_initialise() = cdecl 0x6EAE80 => unported;
    pub fn draw_set_fov(fov: f32) = cdecl 0x6FF410 => unported;
    pub fn custom_car_plate_mgr_initialise() -> u8 = cdecl 0x6FD500 => unported;
    pub fn streaming_load_initial_vehicles() = cdecl 0x407F20 => unported;
    pub fn streaming_load_initial_peds() = cdecl 0x40D3D0 => unported;
    pub fn streaming_load_all_requested_models(only_priority: u32) = cdecl 0x40EA10 => unported;
    pub fn anim_manager_load_anim_files() = cdecl 0x4D5620 => unported;
    pub fn streaming_load_initial_weapons() = cdecl 0x40A120 => unported;
    pub fn ped_initialise() = cdecl 0x5DEBB0 => unported;
    pub fn renderer_init() = cdecl 0x5531C0 => unported;
    pub fn radar_initialise() = cdecl 0x587FB0 => unported;
    pub fn radar_load_textures() = cdecl 0x5827D0 => unported;
    pub fn weapon_initialise_weapons() = cdecl 0x73A300 => unported;
    pub fn road_blocks_init() = cdecl 0x461100 => unported;
    pub fn population_initialise() = cdecl 0x610E10 => unported;
    pub fn coronas_init() = cdecl 0x6FAA70 => unported;
    pub fn shadows_init() = cdecl 0x706CD0 => unported;
}
orig! {
    pub fn weapon_effects_init() = cdecl 0x742AB0 => unported;
    pub fn skidmarks_init() = cdecl 0x7204E0 => unported;
    pub fn glass_init() = cdecl 0x71A8D0 => unported;
    pub fn garages_init_after_restart() = cdecl 0x448B60 => unported;
    pub fn the_scripts_init() = cdecl 0x468D50 => unported;
    pub fn gangs_initialise() = cdecl 0x5DE680 => unported;
    pub fn clock_initialise(scale: u32) = cdecl 0x52CD90 => unported;
    pub fn heli_init_helis() = cdecl 0x6C4560 => unported;
    pub fn cranes_init_cranes() = cdecl 0x6F3FC0 => unported;
    pub fn moving_things_init() = cdecl 0x717D00 => unported;
    pub fn darkel_init() = cdecl 0x43CEB0 => unported;
    pub fn grid_ref_init() = cdecl 0x71D4E0 => unported;
    pub fn stats_init() = cdecl 0x55C0C0 => unported;
    pub fn clouds_init() = cdecl 0x7138D0 => unported;
    pub fn special_fx_init() = cdecl 0x7268F0 => unported;
    pub fn ropes_init() = cdecl 0x555DC0 => unported;
    pub fn water_cannons_init() = cdecl 0x728C80 => unported;
    pub fn bridge_init() = cdecl 0x41BC70 => unported;
    pub fn ped_groups_init() = cdecl 0x5FB8C0 => unported;
    pub fn inform_friends_event_queue_init_call() = cdecl 0x4B2990 => inform_friends_event_queue_init;
    pub fn inform_group_event_queue_init_call() = cdecl 0x4B2AD0 => inform_group_event_queue_init;
    pub fn replay_init() = cdecl 0x45E220 => unported;
    pub fn shopping_init() = cdecl 0x49C290 => unported;

    // ---- Init3
    pub fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    pub fn pad_clear(this: GPtr<u8>, a: u32, b: u32) = thiscall 0x541A70 => unported;
    pub fn interior_manager_init(this: GPtr<u8>) -> u8 = thiscall 0x5C0500 => unported;
    pub fn proc_object_man_init(this: GPtr<u8>) -> u8 = thiscall 0x5A3EA0 => unported;
    pub fn water_creature_manager_init(this: GPtr<u8>) -> u8 = thiscall 0x6E3F90 => unported;
    pub fn real_time_shadow_manager_init(this: GPtr<u8>) = thiscall 0x7067C0 => unported;

    // ---- leaves
    pub fn rw_texture_read(name: u32, mask: u32) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
    pub fn rw_raster_lock(raster: GPtr<u8>, level: u32, flags: u32) -> u32 = cdecl 0x7FB2D0 => unported;
    pub fn ptr_list_double_link_flush_call(this: GPtr<u8>) = thiscall 0x552470 => ptr_list_double_link_flush;
    pub fn inform_friends_event_queue_flush() = cdecl 0x4AC2A0 => unported;
    pub fn inform_group_event_queue_flush() = cdecl 0x4AC410 => unported;
}


/// `CGame::Initialise(datFile)` (0x53BC80): `Init1`, the collision cache around the two level
/// loads (`DATA\DEFAULT.DAT` 0x863A90, then `datFile`), `Init2`, `CStencilShadows::Init`, the
/// "Start script" loading screen, `CTheScripts::StartTestScript` / `Process`, `TheCamera.Process()`,
/// `Init3`; the stages' results are ignored; returns 1.
///
/// Not registered: 0x53BC80 is hooked by sa-core mvpa_game.rs (duplicate port left unregistered when STD-5 merged both).
pub fn initialise(dat: GPtr<u8>) -> u8 {
    init1_call(dat);
    col_accel_start_cache();
    load_level(p8(0x863A90));
    load_level(dat);
    col_accel_end_cache();
    init2_call(dat);
    stencil_shadows_init();
    loading_screen(LOADING_THE_GAME, 0x863A80, 0);
    start_test_script();
    scripts_process();
    camera_process(p8(0xB6F028));
    init3_call(dat);
    1
}

/// `CGame::Init1(datFile)` (0x5BF840): maths tables, the dat file name (0xB728EC), pools, matrix
/// array, ini file, resource buffering off, level / area 0, the "generic" TXD slot (created, ref'd,
/// 0xB728E8) and "particle" (MODELS\PARTICLE.TXD), the game systems' start-of-game initialisers
/// in the original order; 0 when the plant or road-sign manager fails, else 1.
///
/// Not registered: 0x5BF840 is hooked by sa-core mvpa_game.rs (duplicate port left unregistered when STD-5 merged both).
pub fn init1(dat: GPtr<u8>) -> u8 {
    init_maths_tables_call();
    let mut i = 0isize;
    loop {
        let c = dat.byte_add(i).read();
        p8(0xB728EC).byte_add(i).write(c);
        i += 1;
        if c == 0 {
            break;
        }
    }
    pools_initialise();
    placeable_init_matrix_array();
    ini_load();
    set_use_d3d_resource_buffering_call(0);
    g::<u32>(0xB7290C).write(0);
    g::<u32>(0xB72914).write(0);
    let generic = txd_add_slot(0x858A34);
    g::<i32>(0xB728E8).write(generic);
    txd_create(generic);
    txd_add_ref(g::<i32>(0xB728E8).read());
    let particle = txd_add_slot(0x85C648);
    txd_load_txd(particle, 0x86AB6C);
    txd_add_ref(particle);
    txd_set_current(g::<i32>(0xB728E8).read());
    game_logic_init_at_start();
    gang_wars_init_at_start();
    conversations_clear_call();
    ped_to_player_conversations_clear();
    quad_tree_init_pool();
    if plant_mgr_initialise_call() == 0 || custom_roadsign_mgr_initialise_call() == 0 {
        return 0;
    }
    references_init();
    debug_init_text_buffer();
    tag_manager_init();
    weather_init();
    cover_init_call();
    cull_zones_init();
    occlusion_init();
    collision_init();
    birds_init_call();
    entry_exit_manager_init_call();
    stunt_jump_manager_init();
    set_pieces_init();
    the_zones_init();
    user_display_init();
    messages_init();
    messages_clear_all_displayed_by_game(0);
    vehicle_recording_init();
    restart_initialise();
    world_initialise();
    anim_manager_initialise();
    cutscene_mgr_initialise();
    car_ctrl_init();
    init_model_indices();
    model_info_initialise();
    pickups_init();
    the_car_generators_init();
    garages_init();
    audio_zones_init();
    streaming_init_image_list();
    streaming_read_ini_file();
    path_find_init(p8(0x96F050));
    path_find_allocate_info_mem(p8(0x96F050));
    task_simple_fight_load_melee_data();
    cheat_reset_cheats();
    fx_init(p8(0xA9AE00));
    1
}

/// `CGame::Init2(datFile)` (0x5BA1A0): post-creation of zones and entry-exits, path data, the two
/// players cleared and player 1's skin loaded, clothes, water, FOV 120 and LOD distance 500
/// (0xC3EF98), the car-plate manager (failure: 0), the initial vehicles / peds / weapons with the
/// animation files in between, peds, renderer, radar, weapons, road blocks, population, player in
/// focus 0, coronas, shadows, weapon effects, skidmarks, glass, garages, scripts, gangs, clock
/// (scale 1000), helis, cranes, moving things, Darkel, grid refs, stats, clouds, special FX, ropes,
/// water cannons, bridge, ped groups, the event queues, credits off, replay, shopping, with the
/// original loading-screen steps; 1.
///
/// Not registered: 0x5BA1A0 is hooked by sa-core mvpa_game.rs (duplicate port left unregistered when STD-5 merged both).
pub fn init2(_dat: GPtr<u8>) -> u8 {
    let ls = |m: u32| loading_screen(if m == 0 { 0 } else { LOADING_THE_GAME }, m, 0);
    ls(0x86A5A0);
    the_zones_post_zone_creation();
    entry_exit_post_creation();
    ls(0x86A594);
    path_find_prepare_path_data(p8(0x96F050));
    let mut p = 0xB7CD98u32;
    while p < 0xB7D0B8 {
        player_info_clear(p8(p));
        p += 0x190;
    }
    player_info_load_player_skin(p8(0xB7CD98));
    test_model_indices();
    ls(0x86A588);
    clothes_init();
    water_level_initialise();
    draw_set_fov(120.0);
    g::<u32>(0xC3EF98).write(0x43FA_0000);
    if custom_car_plate_mgr_initialise() == 0 {
        return 0;
    }
    ls(0x86A578);
    streaming_load_initial_vehicles();
    streaming_load_initial_peds();
    streaming_load_all_requested_models(0);
    ls(0x86A568);
    anim_manager_load_anim_files();
    ls(0x86A578);
    streaming_load_initial_weapons();
    streaming_load_all_requested_models(0);
    ls(0x86A55C);
    ped_initialise();
    renderer_init();
    ls(0x86A544);
    radar_initialise();
    radar_load_textures();
    ls(0);
    weapon_initialise_weapons();
    road_blocks_init();
    ls(0);
    population_initialise();
    ls(0);
    g::<u8>(0xB7CD74).write(0);
    coronas_init();
    shadows_init();
    ls(0);
    weapon_effects_init();
    skidmarks_init();
    glass_init();
    garages_init_after_restart();
    ls(0x86A534);
    the_scripts_init();
    gangs_initialise();
    ls(0x86A544);
    clock_initialise(1000);
    heli_init_helis();
    cranes_init_cranes();
    ls(0);
    moving_things_init();
    darkel_init();
    ls(0);
    grid_ref_init();
    stats_init();
    clouds_init();
    special_fx_init();
    ls(0);
    ropes_init();
    water_cannons_init();
    bridge_init();
    ped_groups_init();
    inform_friends_event_queue_init_call();
    inform_group_event_queue_init_call();
    ls(0);
    g::<u8>(0xC6E97C).write(0);
    replay_init();
    shopping_init();
    ls(0);
    1
}

/// `CGame::Init3(datFile)` (0x5BA400): "Load scene", both pads cleared (`Clear(1, 1)`), resource
/// buffering on, "Procedural Interiors", the interior, procedural-object, water-creature and
/// real-time-shadow managers; 1.
///
/// Not registered: 0x5BA400 is hooked by sa-core mvpa_game.rs (duplicate port left unregistered when STD-5 merged both).
pub fn init3(_dat: GPtr<u8>) -> u8 {
    loading_screen(LOADING_THE_GAME, 0x86A5C8, 0);
    pad_clear(pad_get_pad(0), 1, 1);
    pad_clear(pad_get_pad(1), 1, 1);
    set_use_d3d_resource_buffering_call(1);
    loading_screen(LOADING_THE_GAME, 0x86A5B0, 0);
    interior_manager_init(p8(0xBAF670));
    proc_object_man_init(p8(0xBB7CB0));
    water_creature_manager_init(p8(0xC1DF30));
    real_time_shadow_manager_init(p8(0xC40350));
    1
}

/// `CMaths::InitMathsTables()` (0x59AC90): `ms_SinTable[i] = sin(i * [0x859BBC])` for i in 0..256
/// (0xBB3E00; x87 `fsin` of the product at the game's precision).
///
/// Not registered: 0x59AC90 is hooked by sa-world mvpa_math.rs (duplicate port left unregistered when STD-5 merged both).
pub fn init_maths_tables() {
    let c = g::<f32>(0x859BBC).read();
    for i in 0..256i32 {
        let v = X87::from_i32(i).fmul_m(c).fsin();
        g::<f32>(0xBB3E00 + 4 * i as usize).write(v.to_f32());
    }
}

/// `D3DResourceSystem::SetUseD3DResourceBuffering(on)` (0x730AC0): the byte 0x8D6084.
///
/// Not registered: 0x730AC0 is hooked by sa-render mvpa_render1.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn set_use_d3d_resource_buffering(on: u32) {
    g::<u8>(0x8D6084).write(on as u8);
}

/// `CConversations::Clear()` (0x43A7B0, `.HOODLUM` body): the 14 conversations (0x9691D8, 0x1C each:
/// two -1 indices, three 0 fields), the 50 nodes (0x969570, 0x18 each: flag byte 0, two -1 words,
/// three 0 dwords), the "awaiting player" byte 0x9691D0 and 0x9691C4 cleared.
///
/// Not registered: 0x43A7B0 is hooked by sa-ai mvpa_ai1.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn conversations_clear() {
    let mut a = 0x9691E0usize;
    while a < 0x969368 {
        g::<i32>(a - 8).write(-1);
        g::<i32>(a - 4).write(-1);
        g::<u32>(a).write(0);
        g::<u32>(a + 8).write(0);
        g::<u32>(a + 4).write(0);
        a += 0x1C;
    }
    let mut a = 0x969578usize;
    while a < 0x969A28 {
        g::<u8>(a - 8).write(0);
        g::<u16>(a + 2).write(0xFFFF);
        g::<u16>(a).write(0xFFFF);
        g::<u32>(a + 8).write(0);
        g::<u32>(a + 0xC).write(0);
        g::<u32>(a + 4).write(0);
        a += 0x18;
    }
    g::<u8>(0x9691D0).write(0);
    g::<u32>(0x9691C4).write(0);
}

/// `CCustomRoadsignMgr::Initialise()` (0x6FE120): the road-sign font texture (0x872590, no mask)
/// read from the "particle" TXD (current TXD pushed / popped), its filter byte (+0x50) 1 and
/// addressing byte (+0x51) 0x33 (no NULL check), stored at 0xC3EF84; its raster locked (level 0,
/// read) into 0xC3EF88; 1.
///
/// Not registered: 0x6FE120 is hooked by sa-render mvpa_render1.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn custom_roadsign_mgr_initialise() -> u8 {
    let slot = txd_find_slot(0x85C648);
    txd_push_current();
    txd_set_current(slot);
    let t = rw_texture_read(0x872590, 0);
    t.byte_add(0x50).write(1);
    t.byte_add(0x51).write(0x33);
    g::<u32>(0xC3EF84).write(t.addr() as u32);
    txd_pop_current();
    let raster = GPtr::<u32>::from_addr(g::<u32>(0xC3EF84).read() as usize).read();
    let px = rw_raster_lock(p8(raster), 0, 2);
    g::<u32>(0xC3EF88).write(px);
    1
}

/// `CCover::Init()` (0x698710): no cover points (0xC197A4), the cover entity list (0xC1A2B8)
/// flushed, every cover point's type byte (0xC197C8, 0x1C each) 0.
///
/// Not registered: 0x698710 is hooked by sa-ai mvpa_ai1.rs (duplicate port left unregistered when STD-5 merged both).
pub fn cover_init() {
    g::<u32>(0xC197A4).write(0);
    ptr_list_double_link_flush_call(p8(0xC1A2B8));
    let mut a = 0xC197C8usize;
    while a < 0xC1A2B8 {
        g::<u8>(a).write(0);
        a += 0x1C;
    }
}

/// `CPtrListDoubleLink::Flush()` (0x552470, thiscall): every node `{item, next +4, prev +8}` unlinked
/// from the head and its neighbours and returned to the double-link node pool (`[0xB74488]`, 12-byte
/// objects: the slot's flag byte gets 0x80, the first-free index lowered).
///
/// Not registered: 0x552470 is hooked by sa-world mvpa_math.rs (duplicate port left unregistered when STD-5 merged both).
pub fn ptr_list_double_link_flush(this: GPtr<u8>) {
    let head = this.cast::<u32>();
    let mut n = head.read();
    while n != 0 {
        let next = GPtr::<u32>::from_addr(n as usize + 4).read();
        if head.read() == n {
            head.write(next);
        }
        let prev = GPtr::<u32>::from_addr(n as usize + 8).read();
        if prev != 0 {
            GPtr::<u32>::from_addr(prev as usize + 4).write(next);
        }
        if next != 0 {
            GPtr::<u32>::from_addr(next as usize + 8).write(GPtr::<u32>::from_addr(n as usize + 8).read());
        }
        let pool = g::<u32>(0xB74488).read() as usize;
        let objs = GPtr::<u32>::from_addr(pool).read();
        let idx = (n.wrapping_sub(objs) as i32) / 12;
        let flags = GPtr::<u32>::from_addr(pool + 4).read();
        let f = GPtr::<u8>::from_addr(flags.wrapping_add(idx as u32) as usize);
        f.write(f.read() | 0x80);
        let first = GPtr::<i32>::from_addr(pool + 0xC);
        if idx < first.read() {
            first.write(idx);
        }
        n = next;
    }
}

/// `CBirds::Init()` (0x711EC0): every bird's "exists" byte (0xC6A8EF, 0x44 each) 0, the bird count
/// 0xC6A8A4 and the "has created birds" byte 0xC6A8A0 cleared.
///
/// Not registered: 0x711EC0 is hooked by sa-render mvpa_render1.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn birds_init() {
    let mut a = 0xC6A8EFusize;
    while a < 0xC6AA87 {
        g::<u8>(a).write(0);
        a += 0x44;
    }
    g::<u32>(0xC6A8A4).write(0);
    g::<u8>(0xC6A8A0).write(0);
}

/// `CInformFriendsEventQueue::Init()` (0x4B2990): a tail jump into `Flush` (0x4AC2A0).
#[crate::native(addr = 0x4B2990, conv = cdecl, name = "CInformFriendsEventQueue::Init")]
pub fn inform_friends_event_queue_init() {
    inform_friends_event_queue_flush();
}

/// `CInformGroupEventQueue::Init()` (0x4B2AD0): a tail jump into `Flush` (0x4AC410).
#[crate::native(addr = 0x4B2AD0, conv = cdecl, name = "CInformGroupEventQueue::Init")]
pub fn inform_group_event_queue_init() {
    inform_group_event_queue_flush();
}

orig! {
    pub fn tag_manager_get_percentage_tagged() -> i32 = cdecl 0x49CDA0 => unported;
    pub fn stats_check_for_stats_message() = cdecl 0x559760 => unported;
    pub fn stats_check_for_threshold(value: GPtr<f32>, range: f32) -> u8 = cdecl 0x5595F0 => unported;
    pub fn stats_display_script_stat_update_message(state: u32, stat: u32, value: f32) = cdecl 0x55B980 => unported;
}

/// `CStats::UpdateRespectStat(reset)` (0x55BC50). `reset`: the last and the displayed respect
/// (0x8CDEC8 / 0x8CDEC4) become -99. Otherwise the respect is computed in x87 single precision
/// from the stats as the original does (each `fcomp` / `test ah` branch transcribed with
/// `sa_math::x87::Cmp`), clamped at the lower limit 0x858B50, 1000 with the cheat flag 0x96917F;
/// when its truncated value differs from the last one's: stat 64 (0xB79480) stored,
/// `CheckForStatsMessage`, the update message (`DisplayScriptStatUpdateMessage(0 / 1, 64, value)`)
/// when a threshold is crossed (or the old value was below 0x858CA0) for a rise / fall, and the
/// last / displayed values updated as the original.
///
/// Not registered: 0x55BC50 is hooked by sa-world mvpb_stats.rs (duplicate port left unregistered when STD-5 merged both).
pub fn stats_update_respect_stat(reset: u32) {
    let last = g::<f32>(0x8CDEC8);
    let shown = g::<f32>(0x8CDEC4);
    if reset as u8 != 0 {
        last.write(-99.0);
        shown.write(-99.0);
        return;
    }
    let f = |a: usize| g::<f32>(a).read();
    let one = f(0x858624);
    let a = X87::from_f32(f(0xB79490)).fmul_m(f(0x858EE8));
    let b = X87::from_i32(g::<i32>(0xB791A0).read()).fmul_m(f(0x858C4C));
    let mut c = X87::from_i32(g::<i32>(0xB791B0).read());
    if !X87::from_f32(one).fcom(c).test_any(0x41) {
        c = X87::from_f32(one);
    }
    let t = b.fdiv(c).fmul_m(f(0x863E10));
    let s = a.fadd(t);
    let u = X87::from_f32(f(0xB79484)).fmul_m(f(0x858B10));
    let s1 = s.fadd(u);
    let lo = f(0x858B50);
    let mut d = X87::from_f32(f(0x96AB9C)).fsub_m(f(0x858CC4));
    if !X87::from_f32(lo).fcom(d).test_any(0x41) {
        d = X87::from_f32(lo);
    }
    let d2 = d.fmul_m(f(0x864314)).fmul_m(f(0x858C28));
    let t1 = d2.fadd(s1).to_f32();
    let player = g::<u8>(0xB7CD74).read() as usize;
    let e = X87::from_i32(g::<i32>(0xB7CE50 + player * 0x190).read()).fmul_m(f(0x864310));
    let e = if X87::from_f32(one).fcom(e).test_parity(5) { e.to_f32() } else { 1.0 };
    let z1 = f(0xB793DC);
    let z2 = f(0xB79488);
    let p = tag_manager_get_percentage_tagged().wrapping_mul(10);
    let x = X87::from_i32(p).fmul_m(f(0x858C28));
    let y = X87::from_f32(e).fmul_m(f(0x858C4C)).fmul_m(f(0x858C28)).fadd_m(t1);
    let y = y.fadd(X87::from_f32(z1).fmul_m(f(0x858B10)));
    let y = y.fadd(X87::from_f32(z2).fmul_m(f(0x858B10)));
    let rx = x.fadd(y);
    let mut r = rx.to_f32();
    if !rx.fcom(X87::from_f32(lo)).test_parity(5) {
        r = 0.0;
    }
    if g::<u8>(0x96917F).read() != 0 {
        r = 1000.0;
    }
    let old_i = X87::from_f32(last.read()).ftol() as u32;
    let new_i = X87::from_f32(r).ftol() as u32;
    if old_i == new_i {
        return;
    }
    let cmp = |v: f32, m: f32| X87::from_f32(v).fcom(X87::from_f32(m));
    g::<f32>(0xB79480).write(r);
    stats_check_for_stats_message();
    let lim = f(0x86430C);
    let k2 = f(0x858CA0);
    if cmp(r, shown.read()).test_any(0x41) && cmp(last.read(), k2).test_parity(5) {
        // not above the displayed value: a fall
        if cmp(last.read(), lim).test_parity(0x44) && cmp(shown.read(), lim).test_parity(0x44) {
            let display = stats_check_for_threshold(GPtr::from_addr(0x8CDEC4), r) != 0 || !cmp(r, k2).test_parity(5);
            if display {
                stats_display_script_stat_update_message(0, 0x40, r);
            }
        }
        if !cmp(r, last.read()).test_any(0x41) {
            shown.write(r);
        }
        last.write(r);
        return;
    }
    if cmp(last.read(), lim).test_parity(0x44) && cmp(shown.read(), lim).test_parity(0x44) {
        let display = stats_check_for_threshold(GPtr::from_addr(0x8CDEC4), r) != 0 || !cmp(last.read(), k2).test_parity(5);
        if display {
            stats_display_script_stat_update_message(1, 0x40, r);
        }
    }
    if !cmp(r, last.read()).test_parity(5) {
        shown.write(r);
    }
    last.write(r);
}

orig! {
    pub fn eh_vector_constructor_iterator(p: GPtr<u8>, size: u32, count: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
}

/// `CMatrixLink::CMatrixLink()` (0x54F0C0, thiscall): the matrix's attached RW matrix (+0x40) and
/// "owns it" flag (+0x44) cleared; returns `this`.
///
/// Not registered: 0x54F0C0 is hooked by sa-world mvpa_math.rs (duplicate port left unregistered when STD-5 merged both).
pub fn matrix_link_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(0x40).cast::<u32>().write(0);
    this.byte_add(0x44).cast::<u32>().write(0);
    this
}

/// `CMatrixLinkList::Init(count)` (0x54F0D0, thiscall): `count` links (0x54 bytes) allocated with a
/// count prefix (`operator new(count * 0x54 + 4)`, the array constructed by the CRT iterator with
/// 0x54F0C0 / 0x54E8E0), the used, allocated and free lists empty (head / tail sentinels linked),
/// then every link pushed after the free head from the last to the first (so the free list runs in
/// array order). An allocation failure leaves the objects NULL and links from NULL.
///
/// Not registered: 0x54F0D0 is hooked by sa-world mvpa_math.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn matrix_link_list_init(this: GPtr<u8>, count: u32) {
    let u = |o: isize| this.byte_add(o).cast::<u32>();
    let a = this.addr() as u32;
    let mem = operator_new(count.wrapping_mul(0x54).wrapping_add(4));
    let mut objs = 0u32;
    if !mem.is_null() {
        objs = mem.addr() as u32 + 4;
        mem.cast::<u32>().write(count);
        eh_vector_constructor_iterator(p8(objs), 0x54, count, 0x54F0C0, 0x54E8E0);
    }
    u(0x50).write(a + 0x54);
    u(0x1A0).write(a + 0x1A4);
    u(0x1F8).write(objs);
    u(0xA0).write(a);
    u(0xF8).write(a + 0xFC);
    u(0x148).write(a + 0xA8);
    u(0x1F0).write(a + 0x150);
    let free_head = a + 0x150;
    let mut i = count;
    while i != 0 {
        i -= 1;
        let l = u(0x1F8).read().wrapping_add(i * 0x54);
        let lp = |o: u32| GPtr::<u32>::from_addr(l.wrapping_add(o) as usize);
        let fh = |o: u32| GPtr::<u32>::from_addr((free_head + o) as usize);
        lp(0x50).write(fh(0x50).read());
        GPtr::<u32>::from_addr((fh(0x50).read() + 0x4C) as usize).write(l);
        lp(0x4C).write(free_head);
        fh(0x50).write(l);
    }
}

/// `CCamera::Fade(time, direction)` (0x50AC20, thiscall; the entry jumps through a protection
/// fragment that loads `direction` and comes back): fade time (+0xC04), "fading" (+0x51),
/// direction word (+0xC30) and its start time (+0xC40) stored; while a fade runs (+0x25) only a fade
/// in (1) continues. Then the music fade flag (+0x52) and direction (+0xC3A), the music fade time
/// (+0xC08) = `time * k` clamped into [k, time] with the original's x87 comparisons; a fade out
/// (0) also stores `time - that` (+0xC0C) and lowers the music time by 0x858B1C (at least
/// 0x858B50); a fade in clears +0xC0C. The music fade's start time (+0xC44). Not hookable: the
/// protection reads the entry bytes (0x1566C27), and the entry is a `jmp` into it.
#[crate::native(addr = 0x50AC20, conv = thiscall, name = "CCamera::Fade")]
pub fn camera_fade(this: GPtr<u8>, time: f32, direction: u32) {
    let dir = direction as u16;
    let fading = this.byte_add(0x25).read();
    this.byte_add(0xC04).cast::<f32>().write(time);
    this.byte_add(0x51).write(1);
    this.byte_add(0xC30).cast::<u16>().write(dir);
    this.byte_add(0xC40).cast::<u32>().write(g::<u32>(0xB7CB84).read());
    if fading != 0 && dir != 1 {
        return;
    }
    let k = g::<f32>(0x858C24).read();
    let x = X87::from_f32(time).fmul_m(k);
    this.byte_add(0x52).write(1);
    this.byte_add(0xC3A).cast::<u16>().write(dir);
    let y = if x.fcom(X87::from_f32(k)).test_any(0x41) { X87::from_f32(k) } else { x };
    let z = if y.fcom(X87::from_f32(time)).test_parity(5) {
        X87::from_f32(time)
    } else if !x.fcom(X87::from_f32(k)).test_any(0x41) {
        x
    } else {
        X87::from_f32(k)
    };
    this.byte_add(0xC08).cast::<f32>().write(z.to_f32());
    if dir != 0 {
        this.byte_add(0xC0C).cast::<u32>().write(0);
    } else {
        let w = X87::from_f32(time).fsub(z);
        this.byte_add(0xC0C).cast::<f32>().write(w.to_f32());
        let lo = g::<f32>(0x858B50).read();
        let mut z2 = z.fsub_m(g::<f32>(0x858B1C).read());
        if z2.fcom(X87::from_f32(lo)).test_any(0x41) {
            z2 = X87::from_f32(lo);
        }
        this.byte_add(0xC08).cast::<f32>().write(z2.to_f32());
    }
    this.byte_add(0xC44).cast::<u32>().write(g::<u32>(0xB7CB84).read());
}

orig! {
    pub fn rw_stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    pub fn env_map_from_buffer_call(buf: GPtr<u8>, out: GPtr<u8>) = cdecl 0x5D8BE0 => env_map_from_buffer;
    pub fn env_map_pool_new_call(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x5D9110 => env_map_pool_new;
}

/// `CustomEnvMapPipeMaterialData::FromBuffer(buf, out)` (0x5D8BE0): the four floats scaled by
/// 0x859000 and the fifth by 0x859AAC, each truncated (`_ftol`) to a byte (out +0..+4), the
/// texture word (+6) 0, the dword +0x14 copied to +8.
///
/// Not registered: 0x5D8BE0 is hooked by sa-render mvpa_render3.rs (duplicate port left unregistered when STD-5 merged both).
pub fn env_map_from_buffer(buf: GPtr<u8>, out: GPtr<u8>) {
    let f = |o: isize| buf.byte_add(o).cast::<f32>().read();
    let k = g::<f32>(0x859000).read();
    for i in 0..4isize {
        out.byte_add(i).write(X87::from_f32(f(4 * i)).fmul_m(k).ftol() as u8);
    }
    out.byte_add(4).write(X87::from_f32(f(0x10)).fmul_m(g::<f32>(0x859AAC).read()).ftol() as u8);
    out.byte_add(8).cast::<u32>().write(buf.byte_add(0x14).cast::<u32>().read());
    out.byte_add(6).cast::<u16>().write(0);
}

/// `CPool<CustomEnvMapPipeMaterialData>::New()` (0x5D9110, thiscall; the wrap-around store goes
/// through a protection fragment): 12-byte objects, `pool::new_slot` semantics; NULL when full.
///
/// Not registered: 0x5D9110 is hooked by sa-render mvpa_render2.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn env_map_pool_new(pool: GPtr<u8>) -> GPtr<u8> {
    let u = |o: isize| pool.byte_add(o).cast::<i32>();
    let mut map = p8(u(4).read() as u32);
    let (slot, ff) = crate::pool::new_slot(&mut map, u(8).read(), u(0xC).read());
    u(0xC).write(ff);
    match slot {
        Some(i) => p8((u(0).read() as u32).wrapping_add((i as u32).wrapping_mul(12))),
        None => GPtr::NULL,
    }
}

/// `CCustomCarEnvMapPipeline::pluginEnvMatStreamReadCB(stream, len, material, offset, size)`
/// (0x5D9660): `len` bytes read into the 0x18-byte record (`len` is not clamped in the original),
/// converted (0x5D8BE0); data equal to the default (0xC02D18) points the material extension at the
/// default, else at a new pool entry (`[0xC02D28]`; full: the default) holding the data. Returns the
/// material.
#[hook(addr = 0x5D9660, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginEnvMatStreamReadCB", expect = "8B 44 24 08 8B 54 24 04",
       unverified = "std4-load-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn env_mat_stream_read_cb(s: GPtr<u8>, len: u32, obj: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    let mut rec = [0u8; 0x18];
    if len <= 0x18 {
        rw_stream_read(s, GPtr::from_raw(rec.as_mut_ptr()), len);
    } else {
        rw_stream_read(s, GPtr::from_raw(rec.as_mut_ptr()), 0x18);
        let mut skip = [0u8; 64];
        let mut rest = len - 0x18;
        while rest > 0 {
            let n = rest.min(64);
            rw_stream_read(s, GPtr::from_raw(skip.as_mut_ptr()), n);
            rest -= n;
        }
    }
    let mut data = [0u32; 3];
    env_map_from_buffer_call(GPtr::from_raw(rec.as_mut_ptr()), GPtr::from_raw(data.as_mut_ptr()).cast());
    let ext = obj.byte_add(offset as isize).cast::<u32>();
    let default = [g::<u32>(0xC02D18).read(), g::<u32>(0xC02D1C).read(), g::<u32>(0xC02D20).read()];
    if data != default {
        let p = env_map_pool_new_call(p8(g::<u32>(0xC02D28).read()));
        ext.write(p.addr() as u32);
        if !p.is_null() {
            for (k, v) in data.iter().enumerate() {
                p.cast::<u32>().wrapping_index(k as i32).write(*v);
            }
            return obj;
        }
    }
    ext.write(0xC02D18);
    obj
}

/// `BreakablePlugin::BreakableStreamRead(stream, len, geometry, offset, size)` (0x59CEC0): the
/// plugin's header dword read into the geometry extension (`[0xBB4238]`); non-zero: the 0x34-byte
/// break info, one `operator new` block for the info and its arrays (positions 12·v, texture
/// coordinates 8·v, colours 4·v, triangles 6·t, material indices 2·t, texture and mask names 32·m
/// each, material colours 12·m, textures 4·m), the info copied to its start and stored in the
/// extension, each array read in that order with its pointer set (the textures' pointer last), then
/// every material's texture `RwTextureRead(name, mask or NULL for an empty mask)` (NULL kept).
/// Returns the stream.
#[hook(addr = 0x59CEC0, conv = cdecl, name = "BreakablePlugin::BreakableStreamRead", expect = "8B 44 24 0C 8B 0D 38 42 BB 00",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn breakable_stream_read(s: GPtr<u8>, _len: i32, obj: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let ext = obj.byte_add(g::<i32>(0xBB4238).read() as isize).cast::<u32>();
    rw_stream_read(s, ext.cast(), 4);
    if ext.read() == 0 {
        return s;
    }
    let mut info = [0u8; 0x34];
    rw_stream_read(s, GPtr::from_raw(info.as_mut_ptr()), 0x34);
    let w = |o: usize| u16::from_le_bytes([info[o], info[o + 1]]) as u32;
    let (nv, nt, nm) = (w(4), w(0x14), w(0x20));
    let sizes = [nv * 12, nv * 8, nv * 4, nt * 6, nt * 2, nm * 32, nm * 32, nm * 12];
    let total: u32 = 0x34 + sizes.iter().sum::<u32>() + nm * 4;
    let b = operator_new(total);
    ext.write(b.addr() as u32);
    for (k, v) in info.iter().enumerate() {
        b.byte_add(k as isize).write(*v);
    }
    let base = b.addr() as u32;
    let mut off = 0x34u32;
    for (field, size) in [0x8u32, 0xC, 0x10, 0x18, 0x1C, 0x28, 0x2C, 0x30].iter().zip(sizes.iter()) {
        p8(base + field).cast::<u32>().write(base + off);
        rw_stream_read(s, p8(base + off), *size);
        off += size;
    }
    p8(base + 0x24).cast::<u32>().write(base + off);
    let names = p8(base + 0x28).cast::<u32>().read();
    let masks = p8(base + 0x2C).cast::<u32>().read();
    for i in 0..nm {
        let mask = if p8(masks + 32 * i).read() == 0 { 0 } else { masks + 32 * i };
        let t = rw_texture_read(names + 32 * i, mask);
        p8(p8(base + 0x24).cast::<u32>().read() + 4 * i).cast::<u32>().write(t.addr() as u32);
    }
    s
}

// ------------------------------------------------------------------ lone `ret`s ------------------

/// `CPathFind::AllocatePathFindInfoMem` (0x44D2B0): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x44D2B0, conv = thiscall, name = "FUN_0044d2b0")]
pub fn path_find_allocate_info_mem_port(_this: GPtr<u8>) {}
/// `CPathFind::PreparePathData` (0x44D0E0): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x44D0E0, conv = thiscall, name = "FUN_0044d0e0")]
pub fn path_find_prepare_path_data_port(_this: GPtr<u8>) {}
/// `CTheZones::PostZoneCreation` (0x572B70): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x572B70, conv = cdecl, name = "CTheZones::PostZoneCreation")]
pub fn the_zones_post_zone_creation_port() {}
/// `TestModelIndices` (0x53ED30): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x53ED30, conv = cdecl, name = "TestModelIndices")]
pub fn test_model_indices_port() {}
/// `CRenderer::Init` (0x5531C0): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x5531C0, conv = cdecl, name = "CRenderer::Init")]
pub fn renderer_init_port() {}
/// `CCranes::InitCranes` (0x6F3FC0): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x6F3FC0, conv = cdecl, name = "CCranes::InitCranes")]
pub fn cranes_init_cranes_port() {}
/// `CBridge::Init` (0x41BC70): a lone `ret` in 1.0 US.
#[crate::native(addr = 0x41BC70, conv = cdecl, name = "CBridge::Init")]
pub fn bridge_init_port() {}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[

    HOOK_PLANT_MGR_INITIALISE,
    HOOK_PLANT_MGR_LOAD_MODELS,
    HOOK_GEOMETRY_SET_PRELIT_CONSTANT_COLOR,
    HOOK_PLANT_SURF_PROP_MGR_LOAD_PLANTS_DAT,
    HOOK_GRASS_SET_PLANT_MODELS_TAB,
    HOOK_GRASS_SET_CLOSE_FAR_ALPHA_DIST,
    HOOK_PLANT_MATERIAL_CB,
    HOOK_ATOMIC_CREATE_PRELIT_IF_NEEDED,
    HOOK_RW_HELPER_GET_FIRST_ATOMIC,
    HOOK_RW_HELPER_SET_FILTER_MODE_ON_ATOMICS_TEXTURES,
    HOOK_QUAD_TREE_NODE_NEW_PORT,
    HOOK_QUAD_TREE_NODE_CTOR,
    HOOK_ENTRY_EXIT_POOL_CTOR,
    HOOK_ENV_MAT_STREAM_READ_CB,
    HOOK_BREAKABLE_STREAM_READ,
];

/// `CTxdStore::TxdStoreLoadCB(name, mask)` (0x731710): the texture read callback while TXDs are
/// loaded: `xor eax, eax; ret` (no texture outside the dictionary). 3 bytes, not hookable.
#[crate::native(addr = 0x731710, conv = cdecl, name = "CTxdStore::TxdStoreLoadCB")]
pub fn txd_store_load_cb(_name: u32, _mask: u32) -> u32 {
    0
}

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[
    NATIVE_PATH_FIND_ALLOCATE_INFO_MEM_PORT,
    NATIVE_PATH_FIND_PREPARE_PATH_DATA_PORT,
    NATIVE_THE_ZONES_POST_ZONE_CREATION_PORT,
    NATIVE_TEST_MODEL_INDICES_PORT,
    NATIVE_RENDERER_INIT_PORT,
    NATIVE_CRANES_INIT_CRANES_PORT,
    NATIVE_BRIDGE_INIT_PORT,
    NATIVE_TXD_STORE_LOAD_CB,
    NATIVE_CAMERA_FADE,
    NATIVE_INFORM_FRIENDS_EVENT_QUEUE_INIT,
    NATIVE_INFORM_GROUP_EVENT_QUEUE_INIT,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];

// ------------------------------------------------------------------ CPlantMgr, entry-exits ------

orig! {
    pub fn plant_mgr_reload_config_call() -> u8 = cdecl 0x5DD780 => plant_mgr_reload_config;
    pub fn plant_surf_prop_mgr_initialise_call() -> u8 = cdecl 0x5DD6C0 => plant_surf_prop_mgr_initialise;
    pub fn plant_surf_prop_mgr_load_plants_dat_call(path: u32) -> u8 = cdecl 0x5DD3B0 => plant_surf_prop_mgr_load_plants_dat;
    pub fn plant_surf_prop_mgr_get_surf_properties_call(id: u32) -> u32 = cdecl 0x6F9DE0 => unported;
    pub fn grass_set_plant_models_tab_call(idx: u32, tab: u32) -> u8 = cdecl 0x5DACC0 => grass_set_plant_models_tab;
    pub fn grass_set_close_far_alpha_dist_call(a: f32, b: f32) = cdecl 0x5DABE0 => grass_set_close_far_alpha_dist;
    pub fn atomic_create_prelit_if_needed_call(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x5DD100 => atomic_create_prelit_if_needed;
    pub fn rw_helper_get_first_atomic_call(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => rw_helper_get_first_atomic;
    pub fn rw_helper_set_filter_mode_call(a: GPtr<u8>, mode: u32) -> u8 = cdecl 0x734D80 => rw_helper_set_filter_mode_on_atomics_textures;
    pub fn streaming_make_space_for(size: u32) = cdecl 0x40E120 => unported;
    pub fn streaming_im_gonna_use_streaming_memory() = cdecl 0x407BE0 => unported;
    pub fn streaming_i_have_used_streaming_memory() = cdecl 0x407BF0 => unported;
    pub fn set_dir(dir: u32) = cdecl 0x5387D0 => unported;
    pub fn open_file(path: u32, mode: u32) -> i32 = cdecl 0x538900 => unported;
    pub fn close_file(f: i32) -> i32 = cdecl 0x5389D0 => unported;
    pub fn load_line(f: i32) -> GPtr<u8> = cdecl 0x536F80 => unported;
    pub fn get_surface_id_from_name(this: GPtr<u8>, name: GPtr<u8>) -> u32 = thiscall 0x55D220 => unported;
    pub fn crt_strtok(s: GPtr<u8>, delim: GPtr<u8>) -> GPtr<u8> = cdecl 0x82244B => unported;
    pub fn crt_atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;
    pub fn crt_atof(s: GPtr<u8>) -> f64 = cdecl 0x823CEE => unported;
    pub fn crt_sprintf4(buf: GPtr<u8>, fmt: u32, a: u32, b: u32) -> i32 = cdecl 0x821BB5 => unported;
    pub fn crt_sprintf3(buf: GPtr<u8>, fmt: u32, a: u32) -> i32 = cdecl 0x821BB5 => unported;
}
orig! {
    pub fn rw_stream_open(ty: u32, access: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7ECEF0 => unported;
    pub fn rw_stream_find_chunk(s: GPtr<u8>, ty: u32, len: u32, ver: u32) -> i32 = cdecl 0x7ED2D0 => unported;
    pub fn rw_stream_close(s: GPtr<u8>, data: u32) -> i32 = cdecl 0x7ECE20 => unported;
    pub fn rp_clump_stream_read(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x74B420 => unported;
    pub fn rp_clump_destroy(c: GPtr<u8>) -> i32 = cdecl 0x74A310 => unported;
    pub fn rp_clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B70 => unported;
    pub fn rp_geometry_lock(g: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x74C7D0 => unported;
    pub fn rp_geometry_unlock(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C800 => unported;
    pub fn rp_geometry_for_all_materials(g: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x74C790 => unported;
    pub fn rp_geometry_create(verts: u32, tris: u32, flags: u32) -> GPtr<u8> = cdecl 0x74CA90 => unported;
    pub fn rp_geometry_triangle_get_material(g: GPtr<u8>, tri: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C760 => unported;
    pub fn rp_geometry_triangle_set_material(g: GPtr<u8>, tri: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C6C0 => unported;
    pub fn rp_atomic_set_geometry(a: GPtr<u8>, g: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x749D40 => unported;
    pub fn rp_atomic_clone(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x749E60 => unported;
    pub fn rp_atomic_set_frame(a: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
    pub fn rw_frame_create() -> GPtr<u8> = cdecl 0x7F0410 => unported;
    pub fn rp_material_set_texture(m: GPtr<u8>, t: u32) -> GPtr<u8> = cdecl 0x74DBC0 => unported;
    pub fn quad_tree_node_new(size: u32) -> GPtr<u8> = cdecl 0x552C80 => quad_tree_node_new_port;
    pub fn quad_tree_node_ctor_call(this: GPtr<u8>, rect: GPtr<u8>, depth: u32) -> GPtr<u8> = thiscall 0x552830 => quad_tree_node_ctor;
    pub fn entry_exit_pool_ctor_call(this: GPtr<u8>, count: i32, name: u32) -> GPtr<u8> = thiscall 0x43F720 => entry_exit_pool_ctor;
    pub fn operator_new_array(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
}

/// `CPlantMgr::Initialise()` (0x5DD910): `ReloadConfig` (failure: 0), streaming memory for the
/// grass TXD (`MakeSpaceFor(0x8800)`), the "grass_pc" TXD slot (found or added, loaded, ref'd,
/// current) and its nine textures (filter 2, addressing 0x11) into 0xC039A0.. / 0xC09174, the
/// current TXD restored, the texture-table pointers, the two plant model sets (`LoadModels` into
/// 0xC039F0 / 0xC03A00, failure: 0), the model-set table (0xC03A30..) and the grass renderer's
/// four model tables and close / far alpha distances (3, 60); 1.
#[hook(addr = 0x5DD910, conv = cdecl, name = "CPlantMgr::Initialise", expect = "83 EC 10 E8 68 FE FF FF",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn plant_mgr_initialise() -> u8 {
    if plant_mgr_reload_config_call() == 0 {
        return 0;
    }
    streaming_make_space_for(0x8800);
    streaming_im_gonna_use_streaming_memory();
    txd_push_current();
    let mut slot = txd_find_slot(0x86BF14);
    if slot == -1 {
        slot = txd_add_slot(0x86BF14);
    }
    txd_load_txd(slot, 0x86C080);
    txd_add_ref(slot);
    txd_set_current(slot);
    let names = [0x86C074u32, 0x86C068, 0x86C05C, 0x86C050, 0x86C044, 0x86C038, 0x86C02C, 0x86C020, 0x86C014];
    let dsts = [0xC039A0usize, 0xC039A4, 0xC039A8, 0xC039AC, 0xC039B0, 0xC039B4, 0xC039B8, 0xC039BC, 0xC09174];
    for (n, d) in names.iter().zip(dsts.iter()) {
        let t = rw_texture_read(*n, 0);
        t.byte_add(0x50).write(2);
        t.byte_add(0x51).write(0x11);
        g::<u32>(*d).write(t.addr() as u32);
    }
    txd_pop_current();
    streaming_i_have_used_streaming_memory();
    g::<u32>(0xC039E0).write(0xC039A0);
    g::<u32>(0xC039E4).write(0xC039B0);
    g::<u32>(0xC039E8).write(0xC039A0);
    g::<u32>(0xC039EC).write(0xC039B0);
    let mut set1 = [0x86C004u32, 0x86BFF4, 0x86BFE4, 0x86BFD4];
    if plant_mgr_load_models_call(p8(0xC039F0), GPtr::from_raw(set1.as_mut_ptr())) == 0 {
        return 0;
    }
    let mut set2 = [0x86BFC4u32, 0x86BFB4, 0x86BFA4, 0x86BF94];
    if plant_mgr_load_models_call(p8(0xC03A00), GPtr::from_raw(set2.as_mut_ptr())) == 0 {
        return 0;
    }
    g::<u32>(0xC03A30).write(0xC039F0);
    g::<u32>(0xC03A34).write(0xC03A00);
    g::<u32>(0xC03A38).write(0xC039F0);
    g::<u32>(0xC03A3C).write(0xC03A00);
    grass_set_plant_models_tab_call(0, 0xC039F0);
    grass_set_plant_models_tab_call(1, g::<u32>(0xC03A30).read());
    grass_set_plant_models_tab_call(2, g::<u32>(0xC03A30).read());
    grass_set_plant_models_tab_call(3, g::<u32>(0xC03A30).read());
    grass_set_close_far_alpha_dist_call(3.0, 60.0);
    1
}

/// `CPlantMgr::ReloadConfig()` (0x5DD780): `CPlantSurfPropMgr::Initialise` (failure: 0), the
/// four list heads (0xC03988..) cleared and 0xC03984 = the plant array, the 256 plant locations
/// (0xC03A48, 0x54 each: 0x30 bytes, +0x40, +0x44, +0x46 cleared) doubly linked (+0x4C next,
/// +0x50 prev), the free list 0xC03998 / 0xC0399C of the 40 entries at 0xC08E48 (0x14 each: 8
/// bytes and +8 word cleared, +0xC next, +0x10 prev) likewise; 1.
///
/// Not registered: 0x5DD780 is hooked by sa-render mvpc_render1.rs (duplicate port left unregistered when STD-5 merged both).
pub fn plant_mgr_reload_config() -> u8 {
    if plant_surf_prop_mgr_initialise_call() == 0 {
        return 0;
    }
    for a in [0xC03988usize, 0xC0398C, 0xC03990, 0xC03994] {
        g::<u32>(a).write(0);
    }
    g::<u32>(0xC03984).write(0xC03A48);
    for i in 0..0x100usize {
        let e = 0xC03A48 + i * 0x54;
        for k in 0..12 {
            g::<u32>(e + 4 * k).write(0);
        }
        g::<u8>(e + 0x46).write(0);
        g::<u32>(e + 0x40).write(0);
        g::<u16>(e + 0x44).write(0);
        if i == 0 {
            g::<u32>(0xC03A98).write(0);
        } else {
            g::<u32>(e + 0x50).write((e - 0x54) as u32);
            if i == 0xFF {
                g::<u32>(0xC08E40).write(0);
                continue;
            }
        }
        g::<u32>(e + 0x4C).write((e + 0x54) as u32);
    }
    g::<u32>(0xC0399C).write(0);
    g::<u32>(0xC03998).write(0xC08E48);
    for i in 0..0x28usize {
        let e = 0xC08E48 + i * 0x14;
        g::<u32>(e).write(0);
        g::<u32>(e + 4).write(0);
        g::<u16>(e + 8).write(0);
        if i == 0 {
            g::<u32>(0xC08E58).write(0);
        } else {
            g::<u32>(e + 0x10).write((e - 0x14) as u32);
            if i == 0x27 {
                g::<u32>(0xC09160).write(0);
                continue;
            }
        }
        g::<u32>(e + 0xC).write((e + 0x14) as u32);
    }
    1
}

/// `CPlantSurfPropMgr::Initialise()` (0x5DD6C0): no surface properties (0xC39ED4 = 0, the 0xB2
/// surface pointers at 0xC38070 cleared), every property record (0xC38338, 0x7C each: word 0, then
/// three plant entries of 0x28 at +4: model -1, uv 0, colour opaque white, intensity 0xFF, 0,
/// scales 1 / 1, the rest 0), then `LoadPlantsDat("PLANTS.DAT")`; its result.
///
/// Not registered: 0x5DD6C0 is hooked by sa-render mvpa_render1.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn plant_surf_prop_mgr_initialise() -> u8 {
    g::<u32>(0xC39ED4).write(0);
    for i in 0..0xB2usize {
        g::<u32>(0xC38070 + 4 * i).write(0);
    }
    let mut e = 0xC38338usize;
    while e < 0xC39ED4 {
        g::<u16>(e).write(0);
        for k in 0..3usize {
            let u = e + 4 + k * 0x28;
            g::<u16>(u).write(0xFFFF);
            g::<u16>(u + 2).write(0);
            g::<u32>(u + 4).write(0xFFFF_FFFF);
            g::<u8>(u + 8).write(0xFF);
            g::<u8>(u + 9).write(0);
            g::<u32>(u + 0xC).write(0x3F80_0000);
            g::<u32>(u + 0x10).write(0x3F80_0000);
            g::<u32>(u + 0x14).write(0);
            g::<u32>(u + 0x18).write(0);
            g::<u32>(u + 0x20).write(0);
            g::<u32>(u + 0x24).write(0);
            g::<u32>(u + 0x1C).write(0);
        }
        e += 0x7C;
    }
    (plant_surf_prop_mgr_load_plants_dat_call(0x86BF88) != 0) as u8
}

/// `CPlantSurfPropMgr::LoadPlantsDat(path)` (0x5DD3B0): the DATA file read line by line up to
/// ";the end"; lines starting ';' skipped; each other line split on " \t" into 18 fields: the surface
/// name (unknown: error text formatted, 0 returned), its property record (allocated from the 57
/// records on first use, full: 0), the plant slot (0..2, larger 0), the record's word, the slot's
/// model, uv, colour bytes R, G, B, intensity, intensity variation, alpha and the seven floats;
/// fewer than 18 fields: 0. The file is closed only on success (1).
#[hook(addr = 0x5DD3B0, conv = cdecl, name = "CPlantSurfPropMgr::LoadPlantsDat", expect = "81 EC 90 00 00 00 53",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn plant_surf_prop_mgr_load_plants_dat(path: u32) -> u8 {
    set_dir(0x86A784);
    let f = open_file(path, 0x85A53C);
    set_dir(0x858B54);
    let mut line_no: u32 = 0;
    let mut line = load_line(f);
    while !line.is_null() {
        line_no += 1;
        let end = g::<[u8; 9]>(0x86A958).read();
        if (0..9).all(|k| line.byte_add(k as isize).read() == end[k]) {
            break;
        }
        if line.read() != b';' {
            let mut delim = [g::<u8>(0x85A510).read(), g::<u8>(0x85A511).read(), g::<u8>(0x85A512).read()];
            let dp = GPtr::from_raw(delim.as_mut_ptr());
            let mut field: u32 = 0;
            let mut rec: u32 = 0;
            let mut slot: u32 = 0;
            let mut tok = crt_strtok(line, dp);
            while !tok.is_null() {
                let ta = tok.addr() as u32;
                match field {
                    0 => {
                        let id = get_surface_id_from_name(p8(0xB79538), tok) as u16;
                        if id == 0 {
                            let mut buf = [0u8; 0x80];
                            crt_sprintf4(GPtr::from_raw(buf.as_mut_ptr()), 0x86BF38, ta, line_no);
                            return 0;
                        }
                        rec = plant_surf_prop_mgr_get_surf_properties_call(id as u32);
                        if rec == 0 {
                            let n = g::<u32>(0xC39ED4).read();
                            if n >= 0x39 {
                                return 0;
                            }
                            rec = 0xC38338 + n * 0x7C;
                            g::<u32>(0xC39ED4).write(n + 1);
                            g::<u32>(0xC38070 + 4 * id as usize).write(rec);
                        }
                    }
                    1 => {
                        let mut v = crt_atol(tok) as u16;
                        if v > 2 {
                            v = 0;
                        }
                        slot = rec + v as u32 * 40 + 4;
                    }
                    2 => p8(rec).cast::<u16>().write(crt_atol(tok) as u16),
                    3 => p8(slot).cast::<u16>().write(crt_atol(tok) as u16),
                    4 => p8(slot + 2).cast::<u16>().write(crt_atol(tok) as u16),
                    5 => p8(slot + 4).write(crt_atol(tok) as u8),
                    6 => p8(slot + 5).write(crt_atol(tok) as u8),
                    7 => p8(slot + 6).write(crt_atol(tok) as u8),
                    8 => p8(slot + 8).write(crt_atol(tok) as u8),
                    9 => p8(slot + 9).write(crt_atol(tok) as u8),
                    10 => p8(slot + 7).write(crt_atol(tok) as u8),
                    11..=17 => {
                        let off = [0xC, 0x10, 0x14, 0x18, 0x20, 0x24, 0x1C][(field - 11) as usize];
                        p8(slot + off).cast::<f32>().write(crt_atof(tok) as f32);
                    }
                    _ => {}
                }
                tok = crt_strtok(GPtr::NULL, dp);
                field += 1;
            }
            if field < 0x12 {
                return 0;
            }
        }
        line = load_line(f);
    }
    close_file(f);
    1
}

/// `CGrassRenderer::SetPlantModelsTab(index, table)` (0x5DACC0): table stored for index < 4 (1),
/// else 0.
#[hook(addr = 0x5DACC0, conv = cdecl, name = "CGrassRenderer::SetPlantModelsTab", expect = "8B 44 24 04 83 F8 04",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn grass_set_plant_models_tab(idx: u32, tab: u32) -> u8 {
    if idx >= 4 {
        return 0;
    }
    g::<u32>(0xC03970 + 4 * idx as usize).write(tab);
    1
}

/// `CGrassRenderer::SetCloseFarAlphaDist(close, far)` (0x5DABE0): 0xC02DBC / 0x8D132C.
#[hook(addr = 0x5DABE0, conv = cdecl, name = "CGrassRenderer::SetCloseFarAlphaDist", expect = "8B 44 24 04 8B 4C 24 08",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn grass_set_close_far_alpha_dist(close: f32, far: f32) {
    g::<f32>(0xC02DBC).write(close);
    g::<f32>(0x8D132C).write(far);
}

orig! {
    /// `CPlantMgr::LoadModels` (0x5DD220): ECX = the output array, EAX = the four names, AL result
    fn plant_mgr_load_models_call(out: GPtr<u8>, names: GPtr<u32>) -> u8 = reg("args = ecx eax") 0x5DD220 => plant_mgr_load_models;
    /// `GeometrySetPrelitConstantColor` (0x5DD1E0): ESI = geometry, stack colour, AL result
    fn geometry_set_prelit_constant_color_call(geo: GPtr<u8>, color: u32) -> u8 = reg("args = esi stack") 0x5DD1E0 => geometry_set_prelit_constant_color;
}

/// `CPlantMgr::LoadModels(names, out)` (0x5DD220; ECX = the output array, EAX = the four names, AL = 1):
/// each "models\grass\<name>" clump streamed (`RwStreamOpen(FILENAME, READ)`, CLUMP chunk,
/// `RpClumpStreamRead`; the stream closed even when it did not open), its first atomic: point
/// filtering, prelit geometry (0x5DD100), geometry flags `(& ~0x30) | 0x40` under a lock, constant
/// prelit white (0x5DD1E0), every material RGBA(0, 0, 0, 50) with the grass texture (0x5DD0B0); the
/// atomic cloned, the clump destroyed, the clone bilinear with a new frame stored; 1.
#[hook(addr = 0x5DD220, conv = reg("args = ecx eax"), name = "CPlantMgr::LoadModels", expect = "81 EC 94 00 00 00",
       unverified = "regconv-render-1", route = "L2", hazards = "heap-writes", recs = "new-game",
       note = "register convention ECX/EAX -> AL (HARNESS-5; was a phase B #[native]); the loop counter is EAX - ECX in the original")]
pub fn plant_mgr_load_models(out: GPtr<u8>, names: GPtr<u32>) -> u8 {
    let mut color: u32 = 0x3200_0000;
    for k in 0..4 {
        let mut buf = [0u8; 0x80];
        let bp = GPtr::from_raw(buf.as_mut_ptr());
        crt_sprintf3(bp, 0x86BF28, names.wrapping_index(k).read());
        let s = rw_stream_open(2, 1, bp);
        let mut clump = GPtr::NULL;
        if !s.is_null() && rw_stream_find_chunk(s, 0x10, 0, 0) != 0 {
            clump = rp_clump_stream_read(s);
        }
        rw_stream_close(s, 0);
        let a = rw_helper_get_first_atomic_call(clump);
        rw_helper_set_filter_mode_call(a, 4);
        atomic_create_prelit_if_needed_call(a);
        let geo = p8(a.byte_add(0x18).cast::<u32>().read());
        rp_geometry_lock(geo, 0xFFF);
        let fl = geo.byte_add(8).cast::<u32>();
        fl.write((fl.read() & 0xFFFF_FFCF) | 0x40);
        rp_geometry_unlock(geo);
        geometry_set_prelit_constant_color_call(geo, 0xFFFF_FFFF);
        rp_geometry_for_all_materials(p8(a.byte_add(0x18).cast::<u32>().read()), 0x5DD0B0, GPtr::from_raw(&mut color).addr() as u32);
        let clone = rp_atomic_clone(a);
        rp_clump_destroy(clump);
        rw_helper_set_filter_mode_call(clone, 2);
        rp_atomic_set_frame(clone, rw_frame_create());
        out.cast::<u32>().wrapping_index(k).write(clone.addr() as u32);
    }
    1
}

/// `GeometrySetPrelitConstantColor(color)` (0x5DD1E0; ESI = geometry, colour on the stack, caller
/// pops): a prelit geometry's colours (+0x30, one per vertex, `rep stosd`) set to `color` under a
/// full lock, AL = 1; not prelit: AL = 0 (`xor al, al`; EAX's upper bytes are left as they were).
#[hook(addr = 0x5DD1E0, conv = reg("args = esi stack"), name = "GeometrySetPrelitConstantColor", expect = "F6 46 08 08 75 03",
       route = "L2", hazards = "heap-writes", recs = "new-game",
       note = "register convention ESI + stack -> AL (HARNESS-5; was a phase B #[native])")]
pub fn geometry_set_prelit_constant_color(geo: GPtr<u8>, color: u32) -> u8 {
    if geo.byte_add(8).read() & 8 == 0 {
        return 0;
    }
    rp_geometry_lock(geo, 0xFFF);
    let cols = geo.byte_add(0x30).cast::<u32>().read();
    let n = geo.byte_add(0x14).cast::<i32>().read();
    if cols != 0 && n > 0 {
        for i in 0..n {
            GPtr::<u32>::from_addr(cols as usize).wrapping_index(i).write(color);
        }
    }
    rp_geometry_unlock(geo);
    1
}

/// The plant material callback (0x5DD0B0, `RpGeometryForAllMaterials`): the material colour
/// (+4) = `*data`, texture = the grass texture 0xC09174; returns the material.
#[hook(addr = 0x5DD0B0, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 15 74 91 C0 00",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn plant_material_cb(m: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    let tex = g::<u32>(0xC09174).read();
    m.byte_add(4).cast::<u32>().write(data.read());
    rp_material_set_texture(m, tex);
    m
}

/// `AtomicCreatePrelitIfNeeded(atomic)` (0x5DD100): a geometry without prelight is replaced by a
/// new one (`RpGeometryCreate(verts, tris, 0xD)`) with the same vertices, texture coordinates,
/// triangles and triangle materials, unlocked, flag 2 added, set on the atomic (`RpAtomicSetGeometry`
/// flag 1). Returns the atomic.
#[hook(addr = 0x5DD100, conv = cdecl, name = "AtomicCreatePrelitIfNeeded", expect = "8B 44 24 04 83 EC 0C",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn atomic_create_prelit_if_needed(a: GPtr<u8>) -> GPtr<u8> {
    let old = p8(a.byte_add(0x18).cast::<u32>().read());
    if old.byte_add(8).read() & 8 != 0 {
        return a;
    }
    let u = |p: GPtr<u8>, o: isize| p.byte_add(o).cast::<u32>().read();
    let tris = u(old, 0x10);
    let verts = u(old, 0x14);
    let new = rp_geometry_create(verts, tris, 0xD);
    let copy = |dst: u32, src: u32, bytes: u32| {
        for i in 0..bytes {
            p8(dst.wrapping_add(i)).write(p8(src.wrapping_add(i)).read());
        }
    };
    copy(u(p8(u(new, 0x5C)), 0x14), u(p8(u(old, 0x5C)), 0x14), verts.wrapping_mul(12));
    copy(u(new, 0x34), u(old, 0x34), verts.wrapping_mul(8));
    let (nt, ot) = (u(new, 0x2C), u(old, 0x2C));
    copy(nt, ot, tris.wrapping_mul(8));
    let mut i = 0i32;
    while i < tris as i32 {
        let m = rp_geometry_triangle_get_material(old, p8(ot.wrapping_add(8 * i as u32)));
        rp_geometry_triangle_set_material(new, p8(nt.wrapping_add(8 * i as u32)), m);
        i += 1;
    }
    rp_geometry_unlock(new);
    let fl = new.byte_add(8).cast::<u32>();
    fl.write(fl.read() | 2);
    rp_atomic_set_geometry(a, new, 1);
    a
}

/// `RwHelper::GetFirstAtomic(clump)` (0x734820): the first atomic `RpClumpForAllAtomics` visits
/// (0x734810), NULL for none.
#[hook(addr = 0x734820, conv = cdecl, name = "RwHelper::GetFirstAtomic", expect = "51 8B 4C 24 08",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn rw_helper_get_first_atomic(c: GPtr<u8>) -> GPtr<u8> {
    let mut first: u32 = 0;
    rp_clump_for_all_atomics(c, 0x734810, GPtr::from_raw(&mut first).cast());
    p8(first)
}

/// `RwHelper::SetFilterModeOnAtomicsTextures(atomic, mode)` (0x734D80): every material texture of
/// the atomic's geometry gets the filter mode (0x734D60); 1.
#[hook(addr = 0x734D80, conv = cdecl, name = "RwHelper::SetFilterModeOnAtomicsTextures", expect = "8B 44 24 08 8B 4C 24 04",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn rw_helper_set_filter_mode_on_atomics_textures(a: GPtr<u8>, mode: u32) -> u8 {
    rp_geometry_for_all_materials(p8(a.byte_add(0x18).cast::<u32>().read()), 0x734D60, mode);
    1
}

/// `CEntryExitManager::Init()` (0x43F880, `.HOODLUM` body): the entry-exit quad tree (a node from
/// the quad-tree pool over (-3000, 3000, 3000, -3000), depth 4) at 0x96A7D0, the state fields
/// 0x96A7CC / 0x96A7C8 / 0x96A7C4 / 0x96A730 cleared, the entry-exit pool (400 entries of 0x3C,
/// `operator new` 0x14 bytes) at 0x96A7D8 with its "owns" byte (+0x11) set.
///
/// Not registered: 0x43F880 is hooked by sa-world mvpc_world2.rs (duplicate port left unregistered when STD-5 merged both).
pub fn entry_exit_manager_init() {
    let mut rect = [0xC53B_8000u32, 0x453B_8000, 0x453B_8000, 0xC53B_8000];
    let n = quad_tree_node_new(0x28);
    let node = if n.is_null() { GPtr::NULL } else { quad_tree_node_ctor_call(n, GPtr::from_raw(rect.as_mut_ptr()).cast(), 4) };
    g::<u32>(0x96A7D0).write(node.addr() as u32);
    g::<u32>(0x96A7CC).write(0);
    g::<u8>(0x96A7C8).write(0);
    g::<u32>(0x96A7C4).write(0);
    g::<u8>(0x96A730).write(0);
    let p = operator_new(0x14);
    let pool = if p.is_null() { GPtr::NULL } else { entry_exit_pool_ctor_call(p, 0x190, 0x8595F4) };
    g::<u32>(0x96A7D8).write(pool.addr() as u32);
    pool.byte_add(0x11).write(1);
}

/// `CQuadTreeNode::operator new` (0x552C80): a node from the quad-tree node pool `[0xB745BC]`
/// (`CPool::New` 0x552B50, the size argument unused).
#[hook(addr = 0x552C80, conv = cdecl, name = "new", expect = "8B 0D BC 45 B7 00",
       unverified = "std4-load-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn quad_tree_node_new_port(_size: u32) -> GPtr<u8> {
    quad_tree_node_pool_new(p8(g::<u32>(0xB745BC).read()))
}

/// `CPool<CQuadTreeNode>::New()` (0x552B50, thiscall): the next free slot after the first-free
/// index (one wrap around), its flag byte made used with the id incremented (`pool::new_slot`);
/// the 0x28-byte object, NULL when full.
///
/// Not registered: 0x552B50 is hooked by sa-core mvpb_app.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn quad_tree_node_pool_new(pool: GPtr<u8>) -> GPtr<u8> {
    let u = |o: isize| pool.byte_add(o).cast::<i32>();
    let mut map = p8(u(4).read() as u32);
    let (slot, ff) = crate::pool::new_slot(&mut map, u(8).read(), u(0xC).read());
    u(0xC).write(ff);
    match slot {
        Some(i) => p8((u(0).read() as u32).wrapping_add((i as u32).wrapping_mul(0x28))),
        None => GPtr::NULL,
    }
}

/// `CQuadTreeNode::CQuadTreeNode(rect, depth)` (0x552830, thiscall): the rectangle copied, item
/// list +0x10 and the four children +0x14..+0x20 cleared, depth +0x24; returns `this`.
#[hook(addr = 0x552830, conv = thiscall, name = "CQuadTreeNode", expect = "8B C1 8B 4C 24 04 56",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn quad_tree_node_ctor(this: GPtr<u8>, rect: GPtr<u8>, depth: u32) -> GPtr<u8> {
    for k in 0..4 {
        this.byte_add(4 * k).cast::<u32>().write(rect.byte_add(4 * k).cast::<u32>().read());
    }
    this.byte_add(0x10).cast::<u32>().write(0);
    this.byte_add(0x24).cast::<u32>().write(depth);
    for k in 0..4 {
        this.byte_add(0x14 + 4 * k).cast::<u32>().write(0);
    }
    this
}

/// `CPool<CEntryExit>::CPool(count, name)` (0x43F720 → `.HOODLUM` 0x1561900, thiscall): objects
/// (`count * 0x3C`, `operator new[]`), flag bytes (`count`), owns 1, size, first free -1, every
/// flag 0x80 (free, id 0); returns `this`.
#[hook(addr = 0x43F720, conv = thiscall, name = "thunk_FUN_01561900", expect = "E9 DB 21 12 01",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn entry_exit_pool_ctor(this: GPtr<u8>, count: i32, _name: u32) -> GPtr<u8> {
    let objs = operator_new_array((count as u32).wrapping_mul(0x3C));
    this.cast::<u32>().write(objs.addr() as u32);
    let flags = operator_new_array(count as u32);
    this.byte_add(4).cast::<u32>().write(flags.addr() as u32);
    this.byte_add(0x10).write(1);
    this.byte_add(8).cast::<i32>().write(count);
    this.byte_add(0xC).cast::<i32>().write(-1);
    for i in 0..count.max(0) {
        let f = flags.wrapping_index(i);
        f.write(f.read() | 0x80);
        f.write(f.read() & 0x80);
    }
    this
}
