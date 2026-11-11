//! `WinMain` (0x748710) of gta_sa.exe 1.0 US, transcribed call by call for the native boot.
//!
//! WinMain itself (583 instructions) is not ported; this module replays its calls in the
//! original order (from the 1.0 US bytes) as a sequence of report steps, so that every callee runs
//! as its Rust port through `orig!`'s address registry, and the first unported callee of each step
//! is reported by name and address. Every callee is declared here `=> unported`: the registry
//! decides at run time (an import goes to its native implementation by IAT trap, see
//! [`crate::imports`]).
//!
//! Boot: [`boot_steps`] (0x748710–0x7489EF). Main loop: one [`frame_step`] per iteration of the
//! `while (!RsGlobal.quit && !FrontEndMenuManager.m_bStartGameLoading)` loop at 0x748A41: one
//! `PeekMessageA` (a message is translated and dispatched, and the iteration ends), otherwise the
//! `gGameState` machine (jump table 0x748EF8).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use sa_core::mem::GPtr;

use crate::boot::{Step, rust};

/// `ControlsManager` (CControllerConfigManager).
const CONTROLS_MANAGER: usize = 0xB70198;
/// `FrontEndMenuManager` (CMenuManager, 0xBA6748) fields WinMain reads.
const MENU_ACTIVE: usize = 0xBA67A4;
const MENU_START_GAME_LOADING: usize = 0xBA67A5;
const MENU_LOADING_DATA: usize = 0xBA67A8;
const MENU_PREFS_FRAME_LIMITER: usize = 0xBA6794;
const MENU_MAIN_SWITCH: usize = 0xBA6831;
const MENU_ACTIVATE_NEXT_FRAME: usize = 0xBA677B;
const MENU_LANGUAGE: usize = 0xBA67C4;
const MENU_PREFS_VIDEO_MODE: usize = 0xBA681C;
const MENU_DISPLAY_VIDEO_MODE: usize = 0xBA6820;
const MENU_MOUSE_UNUSED: usize = 0xBA6818;
/// `RsGlobal`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
const RS_FRAME_LIMIT: usize = 0xC1704C;
const RS_QUIT: usize = 0xC17050;
const RS_PS: usize = 0xC17054;
/// `gGameState`
const GAME_STATE: usize = 0xC8D4C0;
/// `ForegroundApp`, `isForeground` (WinMain's static), `RwInitialized`, `Windowed`,
/// `MultipleSubSystems`, `IsVMNotSelected`, `gCurrentVideoMode`, `anisotropySupportedByGFX`.
const FOREGROUND_APP: usize = 0x8D621C;
const IS_FOREGROUND: usize = 0xC920EC;
const RW_INITIALIZED: usize = 0xC920E8;
const WINDOWED: usize = 0xC920CC;
const MULTIPLE_SUBSYSTEMS: usize = 0xC92118;
const IS_VM_NOT_SELECTED: usize = 0x8D6218;
const CURRENT_VIDEO_MODE: usize = 0x8D6220;
const ANISOTROPY_SUPPORTED: usize = 0xC87FFC;
/// `VideoPlayer`'s DirectShow graph pointer (released through vtable slot 0x24 before a new movie).
const VIDEO_GRAPH: usize = 0xC920D8;
/// `CReplay::Mode`, `AudioEngine`, `gamma`.
const REPLAY_MODE: usize = 0xA43088;
const AUDIO_ENGINE: usize = 0xB6BC90;
const GAMMA: usize = 0xC92134;

sa_core::orig! {
    fn system_parameters_info_a(action: u32, p1: u32, p2: GPtr<u8>, win_ini: u32) -> i32 = stdcall cell 0x85826C => unported;
    fn is_already_running() -> i32 = cdecl 0x7468E0 => unported;
    fn rs_event_handler(event: i32, param: GPtr<u8>) -> i32 = cdecl 0x619B60 => unported;
    fn win32_init_application(instance: u32) -> u32 = cdecl 0x7486A0 => unported;
    fn get_command_line_a() -> GPtr<u8> = stdcall cell 0x858148 => unported;
    fn win32_init_instance(instance: u32) -> u32 = cdecl 0x745560 => unported;
    fn make_controller_actions_blank(this: GPtr<u8>) = thiscall 0x530500 => unported;
    fn init_default_control_configuration(this: GPtr<u8>) = thiscall 0x530640 => unported;
    fn get_module_handle_a(name: GPtr<u8>) -> u32 = stdcall cell 0x858068 => unported;
    fn direct_input8_create(inst: u32, version: u32, iid: GPtr<u8>, out: GPtr<u32>, outer: u32) -> i32 = stdcall 0x817523 => unported;
    fn di_mouse_init(exclusive: u32) -> i32 = cdecl 0x7469A0 => unported;
    fn di_pad_init() = cdecl 0x7485C0 => unported;
    fn get_mouse_set_up(out: GPtr<u8>) -> GPtr<u8> = stdcall 0x53F2D0 => unported;
    fn init_default_control_config_mouse(this: GPtr<u8>, set_up: GPtr<u8>, controls: u32) = thiscall 0x52F6F0 => unported;
    fn destroy_window(hwnd: u32) -> i32 = stdcall cell 0x858270 => unported;
    fn set_window_long_a(hwnd: u32, index: i32, value: u32) -> i32 = stdcall cell 0x858274 => unported;
    fn set_window_pos(hwnd: u32, after: u32, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32 = stdcall cell 0x8582E0 => unported;
    fn show_window(hwnd: u32, cmd: i32) -> i32 = stdcall cell 0x858278 => unported;
    fn update_window(hwnd: u32) -> i32 = stdcall cell 0x8582AC => unported;
    fn set_dir_my_documents() -> i32 = cdecl 0x538860 => unported;
    fn open_file(name: GPtr<u8>, mode: GPtr<u8>) -> i32 = cdecl 0x538900 => unported;
    fn load_settings(this: GPtr<u8>, file: i32) -> u8 = thiscall 0x530530 => unported;
    fn reinit_controls() = cdecl 0x531F20 => unported;
    fn close_file(file: i32) -> i32 = cdecl 0x5389D0 => unported;
    fn set_dir(dir: GPtr<u8>) -> i32 = cdecl 0x5387D0 => unported;
    fn set_error_mode(mode: u32) -> u32 = stdcall cell 0x85813C => unported;
    fn rs_mouse_set_pos(pos: GPtr<f32>) = cdecl 0x6194A0 => unported;
    fn gamma_init(this: GPtr<u8>) = thiscall 0x747180 => unported;
    fn peek_message_a(msg: GPtr<u8>, hwnd: u32, min: u32, max: u32, remove: u32) -> i32 = stdcall cell 0x8582BC => unported;
    fn translate_message(msg: GPtr<u8>) -> i32 = stdcall cell 0x8582C4 => unported;
    fn dispatch_message_a(msg: GPtr<u8>) -> i32 = stdcall cell 0x8582C0 => unported;
    fn sleep(ms: u32) = stdcall cell 0x8580C4 => unported;
    fn load_splashes(starting: u8, logo: u8) = cdecl 0x5900B0 => unported;
    fn loading_screen_init(unk1: u8, unk2: u8) = cdecl 0x5902B0 => unported;
    fn do_pc_title_fade_in() = cdecl 0x590860 => unported;
    fn do_pc_title_fade_out() = cdecl 0x590990 => unported;
    fn loading_screen_shutdown(unk: u8) = cdecl 0x58FF10 => unported;
    fn video_player_play(cmd_show: i32, file: GPtr<u8>) = cdecl 0x747660 => unported;
    fn video_player_release() = cdecl 0x746740 => unported;
    fn update_pads() = cdecl 0x541DD0 => unported;
    fn get_joy_button_just_down(this: GPtr<u8>) -> i32 = thiscall 0x52D1E0 => unported;
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    fn check_for_input(state: GPtr<u8>) -> u8 = thiscall 0x53EFF0 => unported;
    fn is_enter_just_pressed(pad: GPtr<u8>) -> u8 = thiscall 0x4D5980 => unported;
    fn is_standard_key_just_pressed(pad: GPtr<u8>, key: u32) -> u8 = thiscall 0x4D59B0 => unported;
    fn is_menu_key_just_pressed(pad: GPtr<u8>) -> u8 = thiscall 0x744D50 => unported;
    fn is_tab_just_pressed(pad: GPtr<u8>) -> u8 = thiscall 0x744D90 => unported;
    fn video_graph_release(graph: GPtr<u8>) -> u32 = stdcall vslot 0x24 => unported;
    fn initialise_essentials_after_rw() -> u8 = cdecl 0x5BA160 => unported;
    fn initialise_core_data_after_rw() = cdecl 0x5BFA90 => unported;
    fn rw_d3d9_get_caps() -> GPtr<u8> = cdecl 0x7FAD20 => unported;
    fn get_window_placement(hwnd: u32, wp: GPtr<u8>) -> i32 = stdcall cell 0x85828C => unported;
    fn start_loading_tune(this: GPtr<u8>) = thiscall 0x507410 => unported;
    fn initialise_game() = cdecl 0x53E580 => unported;
    fn initialise_post_loading(this: GPtr<u8>) = thiscall 0x5078F0 => unported;
    fn get_current_time_in_cycles() -> u32 = cdecl 0x561A80 => unported;
    fn get_cycles_per_millisecond() -> u32 = cdecl 0x561A40 => unported;
    fn is_beat_info_present(this: GPtr<u8>) -> u8 = thiscall 0x5071D0 => unported;
}
sa_core::orig! {
    fn menu_unload_textures(this: GPtr<u8>) = thiscall 0x574630 => unported;
    fn reset_cheats() = cdecl 0x438450 => unported;
    fn stop_pads_shaking() = cdecl 0x541D70 => unported;
    fn timer_stop() = cdecl 0x561AA0 => unported;
    fn shut_down_for_restart() = cdecl 0x53C550 => unported;
    fn initialise_when_restarting() = cdecl 0x53C680 => unported;
}

/// `FrontEndMenuManager` and its `m_nGameState` byte (0xBA67A6).
const MENU: usize = 0xBA6748;
const MENU_GAME_STATE: usize = 0xBA67A6;
/// WinMain's `isNewGameFirstTime` local (`[esp+0x13]`, 1 at entry).
static NEW_GAME_FIRST_TIME: AtomicBool = AtomicBool::new(true);

fn rd<T: sa_core::mem::GameData>(a: usize) -> T {
    GPtr::<T>::from_addr(a).read()
}
fn wr<T: sa_core::mem::GameData>(a: usize, v: T) {
    GPtr::<T>::from_addr(a).write(v)
}
fn ps() -> GPtr<u32> {
    rd::<GPtr<u32>>(RS_PS)
}
fn rdata(a: usize) -> GPtr<u8> {
    GPtr::from_addr(a)
}

/// `hInstance` and `nCmdShow` WinMain receives.
const INSTANCE: u32 = 0x0040_0000;
const CMD_SHOW: i32 = 10; // SW_SHOWDEFAULT
/// WinMain's locals that live across steps.
static ARGC: AtomicU32 = AtomicU32::new(1);
static ABORTED: AtomicBool = AtomicBool::new(false);

fn b1_is_already_running() -> String {
    system_parameters_info_a(0x2001, 0, GPtr::NULL, 2);
    let r = is_already_running();
    if r != 0 {
        ABORTED.store(true, Ordering::SeqCst);
    }
    format!("IsAlreadyRunning() = {r}")
}
fn b2_rs_initialize() -> String {
    let r = rs_event_handler(24, GPtr::NULL);
    format!("RsEventHandler(rsINITIALIZE) = {r}{}", if r == 0 { " (rsEVENTERROR: WinMain returns)" } else { "" })
}
fn b3_init_application() -> String {
    format!("Win32_InitApplication = {:#x}", win32_init_application(INSTANCE))
}
fn b4_command_line() -> String {
    // `call GetCommandLineA; push &argc; call 0x746480`: the line goes in EAX (Win::CommandLineToArgv's
    // register argument), so the replay calls the port's body with it directly
    let line = get_command_line_a();
    let mut argc = 0i32;
    let argv = sa_core::cmdline::command_line_to_argv(line, GPtr::from_raw(&mut argc));
    ARGC.store(argc as u32, Ordering::SeqCst);
    for i in 1..argc {
        rs_event_handler(37, argv.wrapping_index(i).read());
    }
    format!("argc = {argc}")
}
fn b5_init_instance() -> String {
    let hwnd = win32_init_instance(INSTANCE);
    ps().write(hwnd);
    if hwnd == 0 {
        ABORTED.store(true, Ordering::SeqCst);
        return "no window: WinMain returns".into();
    }
    ps().byte_add(4).write(INSTANCE);
    format!("ps->window = {hwnd:#x}")
}
fn b6_controls() -> String {
    let cm = GPtr::from_addr(CONTROLS_MANAGER);
    make_controller_actions_blank(cm);
    init_default_control_configuration(cm);
    "MakeControllerActionsBlank, InitDefaultControlConfiguration".into()
}
fn b7_direct_input() -> String {
    let di = ps().byte_add(0x18);
    let mut detail = String::new();
    if di.read() == 0 {
        let r = direct_input8_create(get_module_handle_a(GPtr::NULL), 0x800, rdata(0x8588B4), di, 0);
        detail = format!("DirectInput8Create = {r:#x}; ");
        if r < 0 {
            return detail + "diMouseInit/diPadInit skipped";
        }
    }
    let m = di_mouse_init(0);
    di_pad_init();
    detail + &format!("diMouseInit(0) = {m:#x}, diPadInit")
}
fn b8_mouse_setup() -> String {
    let unused = (rd::<u8>(MENU_MOUSE_UNUSED) == 0) as u32;
    let mut tmp = [0u8; 0x14];
    let r = get_mouse_set_up(GPtr::from_raw(tmp.as_mut_ptr()));
    init_default_control_config_mouse(GPtr::from_addr(CONTROLS_MANAGER), r, unused);
    format!("GetMouseSetUp: lmb {} rmb {} mmb {}", tmp[0], tmp[1], tmp[2])
}
fn b9_rw_initialize() -> String {
    let hwnd = ps().read();
    let r = rs_event_handler(21, GPtr::from_addr(hwnd as usize));
    if r == 0 {
        destroy_window(hwnd);
        rs_event_handler(25, GPtr::NULL);
        ABORTED.store(true, Ordering::SeqCst);
        return "RsEventHandler(rsRWINITIALIZE) = rsEVENTERROR: DestroyWindow, rsTERMINATE, WinMain returns".into();
    }
    format!("RsEventHandler(rsRWINITIALIZE) = {r}")
}
fn b10_window_style() -> String {
    let p = ps();
    let full = rd::<u32>(MULTIPLE_SUBSYSTEMS) != 0 || p.byte_add(8).read() != 0;
    if full {
        set_window_long_a(p.read(), -16, 0x8000_0000);
        set_window_pos(p.read(), 0, 0, 0, 0, 0, 0x27);
    }
    // rsCOMMANDLINE for argv[1..]: argc = 1 (GetCommandLineA returns the program name only)
    format!("fullscreen style: {full}; {} rsCOMMANDLINE events", ARGC.load(Ordering::SeqCst).saturating_sub(1))
}
fn b11_camera_size() -> String {
    let mut rect = [0, 0, rd::<i32>(RS_MAX_WIDTH), rd::<i32>(RS_MAX_HEIGHT)];
    let r = rs_event_handler(0, GPtr::from_raw(rect.as_mut_ptr()).cast());
    format!("RsEventHandler(rsCAMERASIZE, {}x{}) = {r}", rect[2], rect[3])
}
fn b12_show_window() -> String {
    // screen saver, power-off and the accessibility hotkeys (SPI 0x11, 0x56, 0x55, 0x3A, 0x3B)
    for a in [0x11, 0x56, 0x55] {
        system_parameters_info_a(a, 0, GPtr::NULL, 2);
    }
    let mut sticky = [8u32, 0];
    system_parameters_info_a(0x3A, 8, GPtr::from_raw(sticky.as_mut_ptr()).cast(), 2);
    let mut toggle = [8u32, 0x100];
    system_parameters_info_a(0x3B, 8, GPtr::from_raw(toggle.as_mut_ptr()).cast(), 2);
    let hwnd = ps().read();
    show_window(hwnd, CMD_SHOW);
    update_window(hwnd);
    "SystemParametersInfoA ×5, ShowWindow, UpdateWindow".into()
}
fn b13_settings() -> String {
    set_dir_my_documents();
    let f = open_file(rdata(0x865674), rdata(0x859C6C));
    let mut detail = String::from("gta_sa.set ");
    if f != 0 {
        let ok = load_settings(GPtr::from_addr(CONTROLS_MANAGER), f);
        if ok == 0 {
            reinit_controls();
        }
        close_file(f);
        detail += &format!("loaded (LoadSettings = {ok})");
    } else {
        detail += "not found";
    }
    set_dir(rdata(0x858B54));
    detail
}
fn b14_before_loop() -> String {
    set_error_mode(1);
    wr::<u32>(RW_INITIALIZED, 1);
    let mut pos = [
        (rd::<i32>(RS_MAX_WIDTH) as f32) * 0.5,
        (rd::<i32>(RS_MAX_HEIGHT) as f32) * 0.5,
    ];
    rs_mouse_set_pos(GPtr::from_raw(pos.as_mut_ptr()));
    gamma_init(GPtr::from_addr(GAMMA));
    "SetErrorMode(1), RwInitialized = 1, RsMouseSetPos(centre), gamma.Init".into()
}

/// WinMain up to the main loop, in the original order.
pub fn boot_steps() -> Vec<Step> {
    vec![
        rust("SystemParametersInfoA + Win::IsAlreadyRunning", 0x7468E0, b1_is_already_running),
        rust("RsEventHandler(rsINITIALIZE) → App::AppEventHandler → CGame::InitialiseOnceBeforeRW, RsInitialize", 0x619B60, b2_rs_initialize),
        rust("Win::Win32_InitApplication", 0x7486A0, b3_init_application),
        rust("GetCommandLineA + command-line split (0x746480) + rsPREINITCOMMANDLINE", 0x746480, b4_command_line),
        rust("Win::Win32_InitInstance → ps->window", 0x745560, b5_init_instance),
        rust("ControlsManager.MakeControllerActionsBlank + InitDefaultControlConfiguration", 0x530640, b6_controls),
        rust("DirectInput8Create + Input::diMouseInit + Input::diPadInit", 0x7469A0, b7_direct_input),
        rust("Input::GetMouseSetUp + InitDefaultControlConfigMouse", 0x53F2D0, b8_mouse_setup),
        rust("RsEventHandler(rsRWINITIALIZE) → Game::RwInitialize → RsRwInitialize (RW engine)", 0x5BF390, b9_rw_initialize),
        rust("rsCOMMANDLINE + fullscreen window style", 0x7488EE, b10_window_style),
        rust("RsEventHandler(rsCAMERASIZE) → CameraSize", 0x72FC70, b11_camera_size),
        rust("SystemParametersInfoA + ShowWindow + UpdateWindow", 0x748980, b12_show_window),
        rust("CFileMgr::SetDirMyDocuments + gta_sa.set (LoadSettings / ReinitControls) + SetDir", 0x530530, b13_settings),
        rust("SetErrorMode + RwInitialized + RsMouseSetPos + CGamma::Init", 0x747180, b14_before_loop),
    ]
}

/// True once WinMain would have returned (or the main loop ended).
/// `gGameState` (0xC8D4C0's machine), for the frame loop's stop rule.
pub fn game_state() -> u32 {
    rd::<u32>(GAME_STATE)
}

/// gGameState LOADING_STARTED: the iteration runs `CGame::Initialise` and the post-loading steps.
pub const STATE_LOADING_STARTED: u32 = 8;

pub fn aborted() -> bool {
    ABORTED.load(Ordering::SeqCst)
}

const STATE_NAMES: [&str; 10] = [
    "INITIAL",
    "LOGO",
    "PLAYING_LOGO",
    "TITLE",
    "PLAYING_INTRO",
    "FRONTEND_LOADING",
    "FRONTEND_LOADED",
    "FRONTEND_IDLE",
    "LOADING_STARTED",
    "IDLE",
];

fn state_name(s: u32) -> &'static str {
    STATE_NAMES.get(s as usize).copied().unwrap_or("?")
}

/// One iteration of WinMain's main loop (0x748A41–0x748DCF).
fn process_game_logic() -> String {
    if rd::<u32>(RS_QUIT) != 0 || rd::<u8>(MENU_START_GAME_LOADING) != 0 {
        // 0x748DD5: the loop ended
        wr::<u32>(RW_INITIALIZED, 0);
        menu_unload_textures(GPtr::from_addr(MENU));
        if rd::<u8>(MENU_START_GAME_LOADING) == 0 {
            ABORTED.store(true, Ordering::SeqCst);
            return "loop ends (RsGlobal.quit): the shutdown (0x748E62) is not replayed".into();
        }
        // 0x748DED: start a game, then back into WinMain at 0x7489F0
        reset_cheats();
        stop_pads_shaking();
        timer_stop();
        let what = if rd::<u8>(MENU_LOADING_DATA) != 0 {
            shut_down_for_restart();
            initialise_when_restarting();
            wr::<u8>(MENU_LOADING_DATA, 0);
            "loading data: ShutDownForRestart + InitialiseWhenRestarting"
        } else if NEW_GAME_FIRST_TIME.load(Ordering::SeqCst) {
            timer_stop();
            let s = if rd::<u8>(MENU_GAME_STATE) != 1 { 8 } else { 6 };
            wr::<u32>(GAME_STATE, s);
            "first new game: gGameState set"
        } else {
            reset_cheats();
            shut_down_for_restart();
            timer_stop();
            initialise_when_restarting();
            "new game: ShutDownForRestart + InitialiseWhenRestarting"
        };
        NEW_GAME_FIRST_TIME.store(false, Ordering::SeqCst);
        wr::<u8>(MENU_GAME_STATE, 0);
        wr::<u8>(MENU_START_GAME_LOADING, 0);
        // 0x7489F0
        wr::<u32>(RW_INITIALIZED, 1);
        let mut pos = [
            (rd::<i32>(RS_MAX_WIDTH) as f32) * 0.5,
            (rd::<i32>(RS_MAX_HEIGHT) as f32) * 0.5,
        ];
        rs_mouse_set_pos(GPtr::from_raw(pos.as_mut_ptr()));
        gamma_init(GPtr::from_addr(GAMMA));
        return format!(
            "m_bStartGameLoading (0x748DED): {what}; gGameState {}",
            state_name(rd::<u32>(GAME_STATE))
        );
    }
    let mut msg = [0u32; 7];
    let m = GPtr::from_raw(msg.as_mut_ptr()).cast::<u8>();
    if peek_message_a(m, 0, 0, 0, 3) != 0 {
        if msg[1] == 0x12 {
            ABORTED.store(true, Ordering::SeqCst);
            return "WM_QUIT: loop ends".into();
        }
        translate_message(m);
        dispatch_message_a(m);
        return format!("message {:#06x} dispatched", msg[1]);
    }
    let before = rd::<u32>(GAME_STATE);
    if rd::<u32>(FOREGROUND_APP) == 0 {
        if rd::<u32>(IS_FOREGROUND) == 1 {
            wr::<u32>(IS_FOREGROUND, 0);
        }
        sleep(100);
        return "not the foreground app: Sleep(100)".into();
    }
    match before {
        0 => {
            for logo in [0u8, 1] {
                load_splashes(1, logo);
                loading_screen_init(1, 1);
                do_pc_title_fade_in();
                do_pc_title_fade_out();
                loading_screen_shutdown(0);
            }
            wr::<u32>(GAME_STATE, 1);
        }
        1 => {
            if rd::<u32>(WINDOWED) == 0 {
                video_player_play(CMD_SHOW, rdata(0x874C04));
            }
            wr::<u32>(GAME_STATE, 2);
        }
        2 | 4 => {
            update_pads();
            let skip = rd::<u32>(WINDOWED) != 0
                || get_joy_button_just_down(GPtr::from_addr(CONTROLS_MANAGER)) != 0
                || check_for_input(get_pad(0)) != 0
                || {
                    let _ = get_pad(0);
                    rd::<u8>(0xB73418) != 0 && rd::<u8>(0xB7342C) == 0
                }
                || is_enter_just_pressed(get_pad(0)) != 0
                || is_standard_key_just_pressed(get_pad(0), 0x20) != 0
                || is_menu_key_just_pressed(get_pad(0)) != 0
                || is_tab_just_pressed(get_pad(0)) != 0;
            if skip {
                wr::<u32>(GAME_STATE, before + 1);
            }
        }
        3 => {
            let g = rd::<GPtr<u8>>(VIDEO_GRAPH);
            if !g.is_null() {
                video_graph_release(g);
            }
            video_player_release();
            let lang = rd::<u32>(MENU_LANGUAGE);
            let movie = if lang == 0xC || lang == 7 { 0x874BD4 } else { 0x874BEC };
            video_player_play(CMD_SHOW, rdata(movie));
            wr::<u32>(GAME_STATE, 4);
        }
        5 => {
            let g = rd::<GPtr<u8>>(VIDEO_GRAPH);
            if !g.is_null() {
                video_graph_release(g);
            }
            video_player_release();
            loading_screen_init(1, 0);
            do_pc_title_fade_in();
            if initialise_essentials_after_rw() == 0 {
                wr::<u32>(RS_QUIT, 1);
            }
            initialise_core_data_after_rw();
            wr::<u32>(GAME_STATE, 6);
            let caps = rw_d3d9_get_caps();
            wr::<u8>(ANISOTROPY_SUPPORTED, ((caps.byte_add(0x24).cast::<u32>().read() >> 17) & 1) as u8);
        }
        6 => {
            wr::<u8>(MENU_ACTIVATE_NEXT_FRAME, 1);
            wr::<u8>(MENU_MAIN_SWITCH, 1);
            if rd::<u32>(IS_VM_NOT_SELECTED) != 0 {
                let vm = rd::<u32>(CURRENT_VIDEO_MODE);
                wr::<u32>(MENU_PREFS_VIDEO_MODE, vm);
                wr::<u32>(MENU_DISPLAY_VIDEO_MODE, vm);
                wr::<u32>(IS_VM_NOT_SELECTED, 0);
            }
            wr::<u32>(GAME_STATE, 7);
            do_pc_title_fade_out();
        }
        7 => {
            let mut wp = [0u32; 11];
            wp[0] = 0x2C;
            get_window_placement(ps().read(), GPtr::from_raw(wp.as_mut_ptr()).cast());
            if wp[2] != 2 {
                rs_event_handler(27, GPtr::NULL);
            }
            let loading = rd::<u8>(MENU_LOADING_DATA);
            if rd::<u8>(MENU_ACTIVE) == 0 || loading != 0 {
                wr::<u32>(GAME_STATE, 8);
                if loading != 0 {
                    loading_started();
                }
            }
        }
        8 => loading_started(),
        9 => {
            let t = get_current_time_in_cycles() as f32 / get_cycles_per_millisecond() as f32;
            if rd::<u32>(RW_INITIALIZED) != 0 {
                let unlimited = rd::<u8>(MENU_PREFS_FRAME_LIMITER) == 0
                    && rd::<u8>(REPLAY_MODE) != 1
                    && is_beat_info_present(GPtr::from_addr(AUDIO_ENGINE)) == 0;
                // otherwise the frame limiter (0x748D81): Idle only once `1000 / frameLimit < t`
                // (`fcomp; test ah, 5; jp skip`: equal, greater or unordered skip)
                if unlimited || 1000.0 / (rd::<i32>(RS_FRAME_LIMIT) as f32) < t {
                    rs_event_handler(26, GPtr::from_addr(1));
                }
            }
        }
        _ => {}
    }
    if rd::<u32>(IS_FOREGROUND) == 0 {
        wr::<u32>(IS_FOREGROUND, 1);
    }
    let after = rd::<u32>(GAME_STATE);
    format!("gGameState {} → {}", state_name(before), state_name(after))
}

fn loading_started() {
    start_loading_tune(GPtr::from_addr(AUDIO_ENGINE));
    initialise_game();
    wr::<u32>(GAME_STATE, 9);
    wr::<u8>(MENU_MAIN_SWITCH, 0);
    initialise_post_loading(GPtr::from_addr(AUDIO_ENGINE));
}

/// One main-loop iteration as a report step.
pub fn frame_step() -> Step {
    let mut s = rust("WinMain main loop iteration (PeekMessageA or the gGameState machine)", 0x748A41, process_game_logic);
    s.probe = true;
    s
}



