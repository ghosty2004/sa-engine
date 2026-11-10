//! CORE ports: the RenderWare skeleton (`Rs*`), the Win32 platform layer (`ps*`, `Win::*`) and the
//! small `App::*` frame helpers of gta_sa.exe 1.0 US (see `docs/arch/core-port.md`, fork C1).
//!
//! Every function follows the 1.0 US bytes (capstone over the exe; gta-reversed `skeleton.cpp`,
//! `WinPs.cpp`, `WinMain.cpp`, `app.cpp` as reading aid). Win32/DirectX imports are called through
//! `orig!` exactly as the original reaches them: `site` for a `call dword ptr [IAT]` instruction (so
//! sa-hook's call-site redirections apply), `cell` where the original first loads the IAT slot into a
//! register (`mov esi, [IAT]; call esi`), `vslot` for COM methods (stdcall, `this` first), `table` for
//! the `RwEngineInstance` function table (0xC97B24), `dyn` for a `GetProcAddress` result.
//!
//! Strings the original passes by address (`push offset aGtaSanAndreas`) are passed as the same
//! addresses (`rdata`); phase B will need owned copies (the imports are `unported` there anyway).
//!
//! Two functions start with a protection chunk (`stc; adc` over a `.HOODLUM` and a `.text` dword) that
//! computes the address of the global they read: the port computes it the same way at run time
//! ([`protected_cell`]).

use crate::mem::{GPtr, GameData, Global};
use crate::{globals, hook, orig};
use sa_math::x87::X87;

/// A pointer to a constant in the original image (`.rdata` string, IID, ...).
pub fn rdata<T>(addr: u32) -> GPtr<T> {
    GPtr::from_addr(addr as usize)
}

/// `RsInputDevice` (skeleton.h): +0 type, +4 used, +8 handler. 12 bytes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RsInputDevice {
    pub device_type: i32,
    pub used: i32,
    /// `RsEventStatus (*)(RsEvent, void*)` as a 32-bit code address.
    pub handler: u32,
}
// SAFETY: integers only.
#[allow(unsafe_code)]
unsafe impl GameData for RsInputDevice {}

/// `psGlobalType` (WinPs, at 0xC8CF88), original layout on 32-bit targets (offsets from the bytes:
/// `[ps]` window in psWindowSetText, `+0x0C/+0x10` lastMousePos in psMouseSetPos, `+0x18`
/// diInterface, `+0x20/+0x24` the two pads in 0x7477C0).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PsGlobalType {
    pub window: u32,
    pub instance: u32,
    pub full_screen: i32,
    pub last_mouse_pos: [f32; 2],
    pub last_render_time: u32,
    pub di_interface: u32,
    pub di_mouse: u32,
    pub di_device1: u32,
    pub di_device2: u32,
}
// SAFETY: integers and floats only.
#[allow(unsafe_code)]
unsafe impl GameData for PsGlobalType {}

const _: () = {
    use core::mem::{offset_of, size_of};
    assert!(size_of::<RsInputDevice>() == 12);
    assert!(offset_of!(PsGlobalType, last_mouse_pos) == 0x0C);
    assert!(offset_of!(PsGlobalType, di_interface) == 0x18);
    assert!(offset_of!(PsGlobalType, di_device1) == 0x20);
    assert!(offset_of!(PsGlobalType, di_device2) == 0x24);
    assert!(size_of::<PsGlobalType>() == 0x28);
};

/// Field `off` of the struct at `p`.
fn field<S, T>(p: GPtr<S>, off: usize) -> GPtr<T> {
    p.byte_add(off as isize).cast()
}

/// Key tables of `RsKeyFromScanCode` (`.data`, platform.cpp `KeysNormal` / `KeysShifted`).
const KEYS_NORMAL_INIT: [u8; 256] = [
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x0D, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x21,
    0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32,
    0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F, 0x40, 0x41, 0x42, 0x43,
    0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54,
    0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5C, 0x5D, 0x5E, 0x5F, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65,
    0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76,
    0x77, 0x78, 0x79, 0x7A, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F, 0x1B, 0xE9, 0xEA, 0xEB, 0xEC, 0xED, 0xEE, 0xEF,
    0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0x7F, 0xF7, 0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xF5, 0x7F,
    0xF7, 0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0x09, 0x2F, 0x2A, 0x2D, 0x2B, 0x0D, 0x20, 0x08, 0x09,
    0x14, 0x0D, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20,
];
const KEYS_SHIFTED_INIT: [u8; 256] = [
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x0D, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x21,
    0x22, 0x23, 0x24, 0x25, 0x26, 0x22, 0x28, 0x29, 0x2A, 0x2B, 0x3C, 0x5F, 0x3E, 0x3F, 0x29, 0x21, 0x40,
    0x23, 0x24, 0x25, 0x5E, 0x26, 0x2A, 0x28, 0x3A, 0x3A, 0x3C, 0x2B, 0x3E, 0x3F, 0x40, 0x41, 0x42, 0x43,
    0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54,
    0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x7B, 0x7C, 0x7D, 0x5E, 0x5F, 0x7E, 0x41, 0x42, 0x43, 0x44, 0x45,
    0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56,
    0x57, 0x58, 0x59, 0x5A, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F, 0x1B, 0xE9, 0xEA, 0xEB, 0xEC, 0xED, 0xEE, 0xEF,
    0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0x7F, 0xF7, 0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xF5, 0x7F,
    0xF7, 0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0x09, 0x2F, 0x2A, 0x2D, 0x2B, 0x0D, 0x20, 0x08, 0x09,
    0x14, 0x0D, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20,
];

globals! {
    list GLOBALS;
    /// `RsGlobal.appName` (RsGlobalType at 0xC17040)
    pub static RS_APP_NAME @ 0xC17040: GPtr<u8> = GPtr::NULL;
    /// `RsGlobal.maximumWidth`
    pub static RS_MAXIMUM_WIDTH @ 0xC17044: i32 = 0;
    /// `RsGlobal.maximumHeight`
    pub static RS_MAXIMUM_HEIGHT @ 0xC17048: i32 = 0;
    /// `RsGlobal.frameLimit`
    pub static RS_FRAME_LIMIT @ 0xC1704C: i32 = 0;
    /// `RsGlobal.quit`
    pub static RS_QUIT @ 0xC17050: i32 = 0;
    /// `RsGlobal.ps`
    pub static RS_PS @ 0xC17054: GPtr<PsGlobalType> = GPtr::NULL;
    /// `RsGlobal.keyboard`
    pub static RS_KEYBOARD @ 0xC17058: RsInputDevice = RsInputDevice { device_type: 0, used: 0, handler: 0 };
    /// `RsGlobal.mouse`
    pub static RS_MOUSE @ 0xC17064: RsInputDevice = RsInputDevice { device_type: 0, used: 0, handler: 0 };
    /// `RsGlobal.pad`
    pub static RS_PAD @ 0xC17070: RsInputDevice = RsInputDevice { device_type: 0, used: 0, handler: 0 };
    /// `Scene.camera` (RwCamera*)
    pub static SCENE_CAMERA @ 0xC1703C: GPtr<u8> = GPtr::NULL;
    /// `KeysNormal`
    pub static KEYS_NORMAL @ 0x8D2C00: [u8; 256] = KEYS_NORMAL_INIT;
    /// `KeysShifted`
    pub static KEYS_SHIFTED @ 0x8D2D00: [u8; 256] = KEYS_SHIFTED_INIT;
    /// `DefaultVM` (VideoMode.h; RwBool, cleared by `-vms`)
    pub static DEFAULT_VM @ 0x8D2E34: i32 = 1;
    /// `ForegroundApp` (app.h says `bool`; the original reads a dword: `mov eax, [8D621Ch]`)
    pub static FOREGROUND_APP @ 0x8D621C: u32 = 1;
    /// `AppClassName` (`char*` to "Grand theft auto San Andreas" at 0x872ED8)
    pub static APP_CLASS_NAME @ 0x8D6224: u32 = 0x872ED8;
    /// `RwMemoryFunctions` returned by `psGetMemoryFunctions`: CMemoryMgr Malloc/Free/Realloc/Calloc
    pub static MEMORY_FUNCTIONS @ 0x8D6228: [u32; 4] = [0x72F420, 0x72F430, 0x72F440, 0x72F460];
    /// `FrontEndMenuManager.m_bPrefsFrameLimiter` (CMenuManager at 0xBA6748, +0x4C)
    pub static PREFS_FRAME_LIMITER @ 0xBA6794: u8 = 0;
    /// `CLoadingScreen::m_bActive`
    pub static LOADING_SCREEN_ACTIVE @ 0xBAB318: u8 = 0;
    /// `FrontEndMenuManager.m_nTitleLanguage` (+0x7C)
    pub static TITLE_LANGUAGE @ 0xBA67C4: i32 = 0;
    /// `FrontEndMenuManager.m_nTextLanguage` (+0x80)
    pub static TEXT_LANGUAGE @ 0xBA67C8: i32 = 0;
    /// `FrontEndMenuManager.m_nPrefsLanguage` (+0x84, a byte store)
    pub static PREFS_LANGUAGE @ 0xBA67CC: u8 = 0;
    /// `trapDisplay` (Securom.cpp)
    pub static SECUROM_TRAP_DISPLAY @ 0xC8CDE8: [i32; 32] = [0; 32];
    /// `triggerDisplay` (Securom.cpp)
    pub static SECUROM_TRIGGER_DISPLAY @ 0xC8CE68: [i32; 32] = [0; 32];
    /// `CTxdStore::ms_txdPluginOffset`
    pub static TXD_PLUGIN_OFFSET @ 0xC88018: i32 = 0;
    /// Protection constants of `RsKeyboardEventHandler`'s entry chunk (0x40591D): `.HOODLUM` dword
    pub static PROT_KBD_A @ 0x15607D4: u32 = 0x2522_DB72;
    /// ... and the `.text` dword it is added to (with carry 1); sum = &RsGlobal.keyboard.used
    pub static PROT_KBD_B @ 0x648111: u32 = 0xDB9E_94E9;
    /// Protection constants of `App::DoRWStuffEndOfFrame`'s chunk (0x4037CB); sum = &Scene.camera
    pub static PROT_EOF_A @ 0x1560754: u32 = 0x18CC_C852;
    /// ... `.text` dword
    pub static PROT_EOF_B @ 0x58287B: u32 = 0xE7F4_A7E9;
    /// Protection constants of `InitUserDirectories`'s entry chunk (0x406807): `.HOODLUM` dword
    pub static PROT_SAM_A @ 0x156081C: u32 = 0xEC61_7D02;
    /// ... minus this `.text` dword = KEY_READ (0x20019), pushed as `samDesired`
    pub static PROT_SAM_B @ 0x53D84A: u32 = 0xEC5F_7CE9;
    /// User files directory (`char[256]`, returned by InitUserDirectories)
    pub static USER_DIR @ 0xC92368: [u8; 256] = [0; 256];
    /// `<user dir>\Gallery`
    pub static GALLERY_DIR @ 0xC92268: [u8; 256] = [0; 256];
    /// `<user dir>\User Tracks`
    pub static USER_TRACKS_DIR @ 0xC92168: [u8; 256] = [0; 256];
}

/// `pushfd; stc; mov eax, [a]; adc eax, [b]; mov eax, [eax]; popfd`: reads the dword at `a + b + 1`.
/// The sum is the address of `known` in the 1.0 US image; phases B/C (owned memory) read `known`.
pub fn protected_cell(a: &Global<u32>, b: &Global<u32>, known: GPtr<u32>) -> u32 {
    let addr = a.get().wrapping_add(b.get()).wrapping_add(1);
    if crate::PHASE_A {
        GPtr::<u32>::from_addr(addr as usize).read()
    } else {
        known.read()
    }
}

/// `pushfd; clc; mov eax, [a]; sbb eax, [b]; mov eax, [eax]; popfd`: reads the dword at `a - b`
/// (see [`protected_cell`]).
pub fn protected_cell_sub(a: &Global<u32>, b: &Global<u32>, known: GPtr<u32>) -> u32 {
    let addr = a.get().wrapping_sub(b.get());
    if crate::PHASE_A {
        GPtr::<u32>::from_addr(addr as usize).read()
    } else {
        known.read()
    }
}

fn keyboard_used() -> GPtr<u32> {
    field(RS_KEYBOARD.ptr(), 4)
}

/// Calls into other functions (original or Rust, decided by their hooks) and imports.
pub mod call {
    use super::*;
    orig! {
        /// `Ps::psTimer` (0x745270)
        pub fn ps_timer() -> u32 = cdecl 0x745270 => super::ps_timer;
        /// `Ps::psWindowSetText` (0x7451B0)
        pub fn ps_window_set_text(text: GPtr<u8>) -> i32 = cdecl 0x7451B0 => super::ps_window_set_text;
        /// `Ps::psPathGetSeparator` (0x745500)
        pub fn ps_path_get_separator() -> u8 = cdecl 0x745500 => super::ps_path_get_separator;
        /// `Ps::psCameraBeginUpdate` (0x745210)
        pub fn ps_camera_begin_update(camera: GPtr<u8>) -> i32 = cdecl 0x745210 => super::ps_camera_begin_update;
        /// `Ps::psAlwaysOnTop` (0x7458B0)
        pub fn ps_always_on_top(on_top: i32) -> i32 = cdecl 0x7458B0 => super::ps_always_on_top;
        /// `Ps::psMouseSetVisibility` (0x7453E0)
        pub fn ps_mouse_set_visibility(visible: i32) -> i32 = cdecl 0x7453E0 => super::ps_mouse_set_visibility;
        /// `Ps::psMouseSetPos` (0x7453F0)
        pub fn ps_mouse_set_pos(pos: GPtr<[f32; 2]>) = cdecl 0x7453F0 => super::ps_mouse_set_pos;
        /// `Ps::psSelectDevice` (0x746190, not ported: deferred)
        pub fn ps_select_device() -> i32 = cdecl 0x746190 => unported;
        /// `Ps::psPathnameDestroy` (0x7454E0)
        pub fn ps_pathname_destroy(p: GPtr<u8>) = cdecl 0x7454E0 => super::ps_pathname_destroy;
        /// `Ps::psPathnameCreate` (0x745470)
        pub fn ps_pathname_create(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x745470 => super::ps_pathname_create;
        /// `Ps::psTerminate` (0x7458A0, a lone `ret`)
        pub fn ps_terminate() = cdecl 0x7458A0 => super::ps_terminate;
        /// `Ps::psGrabScreen` (0x7452B0)
        pub fn ps_grab_screen(camera: GPtr<u8>) -> GPtr<u8> = cdecl 0x7452B0 => super::ps_grab_screen;
        /// `Ps::psInitialize` (0x747420, deferred)
        pub fn ps_initialize() -> i32 = cdecl 0x747420 => unported;
        /// `Ps::psErrorMessage` (0x7451D0)
        pub fn ps_error_message(msg: GPtr<u8>) -> i32 = cdecl 0x7451D0 => super::ps_error_message;
        /// `Ps::psWarningMessage` (0x7451F0)
        pub fn ps_warning_message(msg: GPtr<u8>) -> i32 = cdecl 0x7451F0 => super::ps_warning_message;
        /// `Rs::RsEventHandler` (0x619B60)
        pub fn rs_event_handler(event: i32, param: GPtr<u8>) -> i32 = cdecl 0x619B60 => super::rs_event_handler;
        /// `Rs::RsInitialize` (0x619600)
        pub fn rs_initialize() -> i32 = cdecl 0x619600 => super::rs_initialize;
        /// `Rs::RsRwInitialize` (0x619C90, deferred)
        pub fn rs_rw_initialize(param: GPtr<u8>) -> i32 = cdecl 0x619C90 => unported;
        /// `Rs::RsRwTerminate` (0x6195E0)
        pub fn rs_rw_terminate() = cdecl 0x6195E0 => super::rs_rw_terminate;
        /// `RsCameraShowRaster` (0x619440, a `jmp psCameraShowRaster`; not a CORE row of this port)
        pub fn rs_camera_show_raster(camera: GPtr<u8>) -> GPtr<u8> = cdecl 0x619440 => super::ps_camera_show_raster;
        /// `App::AppEventHandler` (0x53EC10, blocked: FPTAN)
        pub fn app_event_handler(event: i32, param: GPtr<u8>) -> i32 = cdecl 0x53EC10 => unported;
        /// the handler stored in `RsGlobal.keyboard.inputEventHandler`
        pub fn keyboard_handler(event: i32, param: GPtr<u8>) -> i32 = cdecl cell 0xC17060 => unported;
        /// `RsGlobal.mouse.inputEventHandler`
        pub fn mouse_handler(event: i32, param: GPtr<u8>) -> i32 = cdecl cell 0xC1706C => unported;
        /// `RsGlobal.pad.inputEventHandler`
        pub fn pad_handler(event: i32, param: GPtr<u8>) -> i32 = cdecl cell 0xC17078 => unported;
    }
    orig! {
        // RenderWare
        /// `RwCameraBeginUpdate` (0x7EE190)
        pub fn rw_camera_begin_update(camera: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE190 => unported;
        /// `RwCameraEndUpdate` (0x7EE180)
        pub fn rw_camera_end_update(camera: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE180 => unported;
        /// `RwCameraShowRaster` (0x7EE370)
        pub fn rw_camera_show_raster(camera: GPtr<u8>, dev: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EE370 => unported;
        /// `RwEngineStop` (0x7F2E20)
        pub fn rw_engine_stop() -> i32 = cdecl 0x7F2E20 => unported;
        /// `RwEngineClose` (0x7F2F00)
        pub fn rw_engine_close() -> i32 = cdecl 0x7F2F00 => unported;
        /// `RwEngineTerm` (0x7F3130)
        pub fn rw_engine_term() -> i32 = cdecl 0x7F3130 => unported;
        /// `RwD3D9DeviceSupportsDXTTexture` (0x7F9C30)
        pub fn rw_d3d9_device_supports_dxt_texture() -> i32 = cdecl 0x7F9C30 => unported;
        /// `RwTexDictionaryRegisterPlugin` (0x7F3C10)
        pub fn rw_tex_dictionary_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x7F3C10 => unported;
        /// `RwEngineInstance->stringFuncs.rwstrlen` (+0x120)
        pub fn rw_strlen(s: GPtr<u8>) -> u32 = cdecl table 0xC97B24 0x120 => unported;
        /// `RwEngineInstance->stringFuncs.rwstrcpy` (+0xF8)
        pub fn rw_strcpy(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl table 0xC97B24 0xF8 => unported;
        /// `RwEngineInstance->stringFuncs.rwstrchr` (+0x10C)
        pub fn rw_strchr(s: GPtr<u8>, c: i32) -> GPtr<u8> = cdecl table 0xC97B24 0x10C => unported;
        /// `RwEngineInstance->memoryFuncs.rwfree` (+0x138)
        pub fn rw_free(p: GPtr<u8>) = cdecl table 0xC97B24 0x138 => unported;
        /// `CMemoryMgr::Malloc` (0x72F420) as psPathnameCreate calls it: `(size, 0)` (RwMalloc's hint
        /// argument stays on the stack)
        pub fn memory_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
        /// `CDebug::DebugDisplayTextBuffer` (0x532260, a lone `ret`)
        pub fn debug_display_text_buffer() = cdecl 0x532260 => unported;
        /// `FlushObrsPrintfs` (0x734640, a lone `ret`)
        pub fn flush_obrs_printfs() = cdecl 0x734640 => super::flush_obrs_printfs;
        /// `CText::Unload(bool)` (0x69FF20)
        pub fn text_unload(text: GPtr<u8>, keep_mission_packs: u32) = thiscall 0x69FF20 => unported;
        /// `CText::Load(bool)` (0x6A01A0)
        pub fn text_load(text: GPtr<u8>, keep_mission_packs: u32) = thiscall 0x6A01A0 => unported;
        /// `Input::diPadSetRanges(LPDIRECTINPUTDEVICE8, DWORD)` (0x746D80)
        pub fn di_pad_set_ranges(dev: u32, num: u32) -> i32 = cdecl 0x746D80 => unported;
    }
    orig! {
        // Win32 (import name, call site)
        /// USER32 `SetWindowTextA` at 0x7451BE
        pub fn set_window_text_a(hwnd: u32, text: GPtr<u8>) -> i32 = stdcall site 0x7451BE => unported;
        /// USER32 `MessageBoxA` at 0x7451E2 (psErrorMessage)
        pub fn message_box_a_error(hwnd: u32, text: GPtr<u8>, caption: GPtr<u8>, ty: u32) -> i32 = stdcall site 0x7451E2 => unported;
        /// USER32 `MessageBoxA` at 0x745202 (psWarningMessage)
        pub fn message_box_a_warning(hwnd: u32, text: GPtr<u8>, caption: GPtr<u8>, ty: u32) -> i32 = stdcall site 0x745202 => unported;
        /// WINMM `timeGetDevCaps` at 0x74527B
        pub fn time_get_dev_caps(caps: GPtr<[u32; 2]>, size: u32) -> u32 = stdcall site 0x74527B => unported;
        /// WINMM `timeBeginPeriod` at 0x745286
        pub fn time_begin_period(period: u32) -> u32 = stdcall site 0x745286 => unported;
        /// WINMM `timeGetTime` at 0x74528C
        pub fn time_get_time() -> u32 = stdcall site 0x74528C => unported;
        /// WINMM `timeEndPeriod` at 0x745299
        pub fn time_end_period(period: u32) -> u32 = stdcall site 0x745299 => unported;
        /// USER32 `ShowCursor` at 0x7453E5
        pub fn show_cursor(show: i32) -> i32 = stdcall site 0x7453E5 => unported;
        /// USER32 `ClientToScreen` at 0x74541D
        pub fn client_to_screen(hwnd: u32, pt: GPtr<[i32; 2]>) -> i32 = stdcall site 0x74541D => unported;
        /// USER32 `SetCursorPos` at 0x74542D
        pub fn set_cursor_pos(x: i32, y: i32) -> i32 = stdcall site 0x74542D => unported;
        /// KERNEL32 `OutputDebugStringA` (IAT 0x858118, loaded into a register)
        pub fn output_debug_string_a(s: GPtr<u8>) = stdcall cell 0x858118 => unported;
        /// USER32 `AdjustWindowRect` at 0x745592
        pub fn adjust_window_rect(rect: GPtr<[i32; 4]>, style: u32, menu: i32) -> i32 = stdcall site 0x745592 => unported;
        /// USER32 `CreateWindowExA` at 0x7455D5
        pub fn create_window_ex_a(ex_style: u32, class: GPtr<u8>, name: GPtr<u8>, style: u32, x: i32, y: i32, w: i32, h: i32, parent: u32, menu: u32, instance: u32, param: u32) -> u32 = stdcall site 0x7455D5 => unported;
        /// DDRAW `DirectDrawCreateEx` (import thunk 0x81805C)
        pub fn direct_draw_create_ex(guid: u32, dd: GPtr<u32>, iid: GPtr<u8>, outer: u32) -> i32 = stdcall 0x81805C => unported;
        /// `IDirectDraw7::GetAvailableVidMem` (vtable +0x5C)
        pub fn dd_get_available_vid_mem(dd: GPtr<u32>, caps: GPtr<[u32; 4]>, total: GPtr<u32>, free: GPtr<u32>) -> i32 = stdcall vslot 0x5C => unported;
        /// `IUnknown::Release` (vtable +8)
        pub fn com_release(obj: GPtr<u32>) -> u32 = stdcall vslot 0x08 => unported;
        /// ole32 `CoInitialize` at 0x74584B
        pub fn co_initialize_ds(reserved: u32) -> i32 = stdcall site 0x74584B => unported;
        /// DSOUND `DirectSoundCreate` (ordinal 11, import thunk 0x81803B)
        pub fn direct_sound_create(guid: u32, ds: GPtr<u32>, outer: u32) -> i32 = stdcall 0x81803B => unported;
        /// `IDirectSound::GetCaps` (vtable +0x10)
        pub fn ds_get_caps(ds: GPtr<u32>, caps: GPtr<[u32; 24]>) -> i32 = stdcall vslot 0x10 => unported;
        /// USER32 `GetWindowRect` at 0x7458C1
        pub fn get_window_rect(hwnd: u32, rect: GPtr<[i32; 4]>) -> i32 = stdcall site 0x7458C1 => unported;
        /// USER32 `SetWindowPos` at 0x7458EC (HWND_TOPMOST)
        pub fn set_window_pos_top(hwnd: u32, after: i32, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32 = stdcall site 0x7458EC => unported;
        /// USER32 `SetWindowPos` at 0x74590E (HWND_NOTOPMOST)
        pub fn set_window_pos_notop(hwnd: u32, after: i32, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32 = stdcall site 0x74590E => unported;
        /// KERNEL32 `CreateEventA` at 0x7468EC
        pub fn create_event_a(attr: u32, manual: i32, initial: i32, name: GPtr<u8>) -> u32 = stdcall site 0x7468EC => unported;
        /// KERNEL32 `GetLastError` at 0x7468F2
        pub fn get_last_error() -> u32 = stdcall site 0x7468F2 => unported;
        /// USER32 `FindWindowA` at 0x74690D
        pub fn find_window_a(class: GPtr<u8>, name: GPtr<u8>) -> u32 = stdcall site 0x74690D => unported;
        /// USER32 `SetForegroundWindow` at 0x746918 (the found window)
        pub fn set_foreground_window_found(hwnd: u32) -> i32 = stdcall site 0x746918 => unported;
        /// USER32 `SetForegroundWindow` at 0x74692C (our window)
        pub fn set_foreground_window_own(hwnd: u32) -> i32 = stdcall site 0x74692C => unported;
        /// KERNEL32 `GetSystemDefaultLCID` (IAT 0x858248, loaded into EDI, called twice)
        pub fn get_system_default_lcid() -> u32 = stdcall cell 0x858248 => unported;
        /// KERNEL32 `GetUserDefaultLCID` at 0x7465C4
        pub fn get_user_default_lcid_1() -> u32 = stdcall site 0x7465C4 => unported;
        /// KERNEL32 `GetUserDefaultLCID` at 0x7465E4
        pub fn get_user_default_lcid_2() -> u32 = stdcall site 0x7465E4 => unported;
        /// USER32 `GetKeyboardLayout` at 0x7465D4
        pub fn get_keyboard_layout_1(thread: u32) -> u32 = stdcall site 0x7465D4 => unported;
        /// USER32 `GetKeyboardLayout` at 0x7465F2
        pub fn get_keyboard_layout_2(thread: u32) -> u32 = stdcall site 0x7465F2 => unported;
        /// KERNEL32 `LoadLibraryA` (IAT 0x858070, loaded into EBP)
        pub fn load_library_a(name: GPtr<u8>) -> u32 = stdcall cell 0x858070 => unported;
        /// KERNEL32 `GetProcAddress` at 0x74567D
        pub fn get_proc_address(module: u32, name: GPtr<u8>) -> u32 = stdcall site 0x74567D => unported;
        /// KERNEL32 `FreeLibrary` (IAT 0x858110; called both through `call [IAT]` and a register)
        pub fn free_library(module: u32) -> i32 = stdcall cell 0x858110 => unported;
        /// `DirectDrawCreateEx` through the pointer `GetProcAddress` returned
        pub fn direct_draw_create_ex_dyn(f: u32, guid: u32, dd: GPtr<u32>, iid: GPtr<u8>, outer: u32) -> i32 = stdcall dyn => unported;
        /// `IDirectInput8::EnumDevices` (vtable +0x10)
        pub fn di_enum_devices(di: GPtr<u32>, class: u32, cb: u32, rf: u32, flags: u32) -> i32 = stdcall vslot 0x10 => unported;
    }
    orig! {
        // App::PluginAttach
        /// `RpWorldPluginAttach` (0x74FDA0)
        pub fn rp_world_plugin_attach() -> i32 = cdecl 0x74FDA0 => unported;
        /// `RpSkinPluginAttach` (0x7C6820)
        pub fn rp_skin_plugin_attach() -> i32 = cdecl 0x7C6820 => unported;
        /// `RtAnimInitialize` (0x7CCCA0)
        pub fn rt_anim_initialize() -> i32 = cdecl 0x7CCCA0 => unported;
        /// `RpHAnimPluginAttach` (0x7C4600)
        pub fn rp_hanim_plugin_attach() -> i32 = cdecl 0x7C4600 => unported;
        /// `NodeNamePlugin::NodeNamePluginAttach` (0x72FAB0)
        pub fn node_name_plugin_attach() -> i32 = cdecl 0x72FAB0 => unported;
        /// `CVisibilityPlugins::PluginAttach` (0x732E30)
        pub fn visibility_plugins_attach() -> i32 = cdecl 0x732E30 => unported;
        /// `RpAnimBlend::RpAnimBlendPluginAttach` (0x4D6150)
        pub fn rp_anim_blend_plugin_attach() -> i32 = cdecl 0x4D6150 => unported;
        /// `CTxdStore::PluginAttach` (0x731650, AL)
        pub fn txd_store_plugin_attach() -> u8 = cdecl 0x731650 => super::txd_store_plugin_attach;
        /// `RpMatFXPluginAttach` (0x810AA0)
        pub fn rp_mat_fx_plugin_attach() -> i32 = cdecl 0x810AA0 => unported;
        /// `RpUVAnimPluginAttach` (0x7CB940)
        pub fn rp_uv_anim_plugin_attach() -> i32 = cdecl 0x7CB940 => unported;
        /// `CCustomBuildingRenderer::PluginAttach` (0x5D7EF0)
        pub fn custom_building_renderer_plugin_attach() -> i32 = cdecl 0x5D7EF0 => unported;
        /// `CCarFXRenderer::RegisterPlugins` (0x5D5B00)
        pub fn car_fx_renderer_register_plugins() -> i32 = cdecl 0x5D5B00 => unported;
        /// `RpAnisotPluginAttach` (0x748F70)
        pub fn rp_anisot_plugin_attach() -> i32 = cdecl 0x748F70 => unported;
        /// `BreakablePlugin::BreakablePluginAttach` (0x59D100)
        pub fn breakable_plugin_attach() -> i32 = cdecl 0x59D100 => unported;
        /// `CCollisionPlugin::PluginAttach` (0x41B310, AL)
        pub fn collision_plugin_attach() -> u8 = cdecl 0x41B310 => unported;
        /// `C2dEffect::PluginAttach` (0x6FA970, AL)
        pub fn effect_2d_plugin_attach() -> u8 = cdecl 0x6FA970 => unported;
        /// `PipelinePlugin::PipelinePluginAttach` (0x72FBD0)
        pub fn pipeline_plugin_attach() -> i32 = cdecl 0x72FBD0 => unported;
    }
    orig! {
        // CFileMgr::InitUserDirectories
        /// ADVAPI32 `RegOpenKeyExA` at 0x744FD1
        pub fn reg_open_key_ex_a(key: u32, sub: GPtr<u8>, opts: u32, sam: u32, out: GPtr<u32>) -> i32 = stdcall site 0x744FD1 => unported;
        /// ADVAPI32 `RegQueryValueExA` at 0x745001
        pub fn reg_query_value_ex_a(key: u32, name: GPtr<u8>, reserved: u32, ty: GPtr<u32>, data: GPtr<u8>, size: GPtr<u32>) -> i32 = stdcall site 0x745001 => unported;
        /// ADVAPI32 `RegCloseKey` at 0x745018 (after a successful query)
        pub fn reg_close_key_ok(key: u32) -> i32 = stdcall site 0x745018 => unported;
        /// ADVAPI32 `RegCloseKey` at 0x74517A (the query failed)
        pub fn reg_close_key_fail(key: u32) -> i32 = stdcall site 0x74517A => unported;
        /// KERNEL32 `CreateFileA` (IAT 0x8580A0 in ESI)
        pub fn create_file_a(name: GPtr<u8>, access: u32, share: u32, sec: u32, disp: u32, flags: u32, tmpl: u32) -> u32 = stdcall cell 0x8580A0 => unported;
        /// KERNEL32 `CreateDirectoryA` (IAT 0x858120 in EBX)
        pub fn create_directory_a(name: GPtr<u8>, sec: u32) -> i32 = stdcall cell 0x858120 => unported;
        /// KERNEL32 `CloseHandle` (IAT 0x858078 in EBP)
        pub fn close_handle(h: u32) -> i32 = stdcall cell 0x858078 => unported;
    }
    orig! {
        // Securom::SecuromStateDisplay
        /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0)
        pub fn crgba_ctor(this: GPtr<u32>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u32> = thiscall 0x7170C0 => unported;
        /// `CFont::SetColor(CRGBA)` (0x719430)
        pub fn font_set_color(c: u32) = cdecl 0x719430 => unported;
        /// `CFont::SetFontStyle` (0x719490)
        pub fn font_set_font_style(s: u32) = cdecl 0x719490 => unported;
        /// `CFont::SetScale(w, h)` (0x719380)
        pub fn font_set_scale(w: f32, h: f32) = cdecl 0x719380 => unported;
        /// `CFont::SetEdge` (0x719590)
        pub fn font_set_edge(e: u32) = cdecl 0x719590 => unported;
        /// `CFont::SetDropColor(CRGBA)` (0x719510)
        pub fn font_set_drop_color(c: u32) = cdecl 0x719510 => unported;
        /// `CFont::SetProportional` (0x7195B0)
        pub fn font_set_proportional(p: u32) = cdecl 0x7195B0 => unported;
        /// `CFont::SetWrapx` (0x7194D0)
        pub fn font_set_wrapx(x: f32) = cdecl 0x7194D0 => unported;
        /// `CFont::SetRightJustifyWrap` (0x7194F0)
        pub fn font_set_right_justify_wrap(x: u32) = cdecl 0x7194F0 => unported;
        /// `CFont::SetOrientation` (0x719610)
        pub fn font_set_orientation(o: u32) = cdecl 0x719610 => unported;
        /// CRT `sprintf` (0x821BB5) with one integer
        pub fn crt_sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, n: i32) -> i32 = cdecl 0x821BB5 => unported;
        /// `Common::AsciiToGxtChar(src, dst)` (0x718600)
        pub fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;
        /// `CFont::PrintString(x, y, text)` (0x71A700)
        pub fn font_print_string(x: f32, y: f32, s: GPtr<u8>) = cdecl 0x71A700 => unported;
    }
    orig! {
        // Ps::psGrabScreen
        /// `RwD3D9GetCurrentD3DDevice` (0x7F9D50)
        pub fn rw_d3d9_get_current_d3d_device() -> u32 = cdecl 0x7F9D50 => unported;
        /// `IDirect3DDevice9::GetDisplayMode` (+0x20)
        pub fn d3d_dev_get_display_mode(dev: GPtr<u32>, swap: u32, mode: GPtr<[u32; 4]>) -> i32 = stdcall vslot 0x20 => unported;
        /// `IDirect3DDevice9::GetFrontBufferData` (+0x84)
        pub fn d3d_dev_get_front_buffer_data(dev: GPtr<u32>, swap: u32, surface: u32) -> i32 = stdcall vslot 0x84 => unported;
        /// `IDirect3DDevice9::CreateOffscreenPlainSurface` (+0x90)
        pub fn d3d_dev_create_offscreen_plain_surface(dev: GPtr<u32>, w: u32, h: u32, fmt: u32, pool: u32, out: GPtr<u32>, shared: u32) -> i32 = stdcall vslot 0x90 => unported;
        /// `IDirect3DSurface9::LockRect` (+0x34)
        pub fn d3d_surface_lock_rect(surface: GPtr<u32>, locked: GPtr<[u32; 2]>, rect: GPtr<[i32; 4]>, flags: u32) -> i32 = stdcall vslot 0x34 => unported;
        /// USER32 `GetWindowRect` at 0x74530B
        pub fn get_window_rect_grab(hwnd: u32, rect: GPtr<[i32; 4]>) -> i32 = stdcall site 0x74530B => unported;
        /// `RwImageCreate(w, h, depth)` (0x8026E0)
        pub fn rw_image_create(w: i32, h: i32, depth: i32) -> GPtr<u8> = cdecl 0x8026E0 => unported;
        /// `RwImageAllocatePixels` (0x8027A0)
        pub fn rw_image_allocate_pixels(image: GPtr<u8>) -> GPtr<u8> = cdecl 0x8027A0 => unported;
    }
    orig! {
        // Win::Win32_InitApplication
        /// USER32 `LoadIconA` at 0x7486A7
        pub fn load_icon_a(instance: u32, id: u32) -> u32 = stdcall site 0x7486A7 => unported;
        /// USER32 `LoadCursorA` at 0x7486D5
        pub fn load_cursor_a(instance: u32, id: u32) -> u32 = stdcall site 0x7486D5 => unported;
        /// USER32 `RegisterClassA` at 0x7486F5
        pub fn register_class_a(wc: GPtr<[u32; 10]>) -> u32 = stdcall site 0x7486F5 => unported;
    }
}

// ------------------------------------------------------------------------------- Rs (skeleton) --

/// `Rs::RsKeyFromScanCode(uint8 scan, RwBool shiftKeyDown)` (0x6193F0, cdecl, `RsKeyCodes` in EAX):
/// `KeysShifted[scan]` when the second argument is non-zero, else `KeysNormal[scan]`, zero-extended
/// (`movzx eax, byte [esp+4]; mov al, [eax+table]`: EAX = the table byte).
#[hook(addr = 0x6193F0, conv = cdecl, name = "Rs::RsKeyFromScanCode", expect = "8B 44 24 08 85 C0")]
pub fn rs_key_from_scan_code(scan: u32, shifted: i32) -> u32 {
    let i = (scan & 0xFF) as usize;
    let b = if shifted != 0 {
        KEYS_SHIFTED.get()[i]
    } else {
        KEYS_NORMAL.get()[i]
    };
    b as u32
}

/// `Rs::RsTimer` (0x619410): `jmp psTimer`.
#[hook(addr = 0x619410, conv = cdecl, name = "Rs::RsTimer", expect = "E9 5B BE 12 00 90")]
pub fn rs_timer() -> u32 {
    call::ps_timer()
}

/// `Rs::RsWindowSetText` (0x619420): `jmp psWindowSetText`.
#[hook(addr = 0x619420, conv = cdecl, name = "Rs::RsWindowSetText", expect = "E9 8B BD 12 00 90")]
pub fn rs_window_set_text(text: GPtr<u8>) -> i32 {
    call::ps_window_set_text(text)
}

/// `Rs::RsPathGetSeparator` (0x619430): `jmp psPathGetSeparator`.
#[hook(addr = 0x619430, conv = cdecl, name = "Rs::RsPathGetSeparator", expect = "E9 CB C0 12 00 90")]
pub fn rs_path_get_separator() -> u8 {
    call::ps_path_get_separator()
}

/// `Rs::RsCameraBeginUpdate` (0x619450): `jmp psCameraBeginUpdate`.
#[hook(addr = 0x619450, conv = cdecl, name = "Rs::RsCameraBeginUpdate", expect = "E9 BB BD 12 00 90")]
pub fn rs_camera_begin_update(camera: GPtr<u8>) -> i32 {
    call::ps_camera_begin_update(camera)
}

/// `Rs::RsAlwaysOnTop` (0x619460): `jmp psAlwaysOnTop`.
#[hook(addr = 0x619460, conv = cdecl, name = "Rs::RsAlwaysOnTop", expect = "E9 4B C4 12 00 90")]
pub fn rs_always_on_top(on_top: i32) -> i32 {
    call::ps_always_on_top(on_top)
}

/// `Rs::RsRegisterImageLoader` (0x619470): `return TRUE`.
#[hook(addr = 0x619470, conv = cdecl, name = "Rs::RsRegisterImageLoader", expect = "B8 01 00 00 00")]
pub fn rs_register_image_loader() -> i32 {
    1
}

/// `Rs::RsSetDebug` (0x619480): `return TRUE`.
#[hook(addr = 0x619480, conv = cdecl, name = "Rs::RsSetDebug", expect = "B8 01 00 00 00")]
pub fn rs_set_debug() -> i32 {
    1
}

/// `Rs::RsMouseSetVisibility` (0x619490): `jmp psMouseSetVisibility`.
#[hook(addr = 0x619490, conv = cdecl, name = "Rs::RsMouseSetVisibility", expect = "E9 4B BF 12 00 90")]
pub fn rs_mouse_set_visibility(visible: i32) -> i32 {
    call::ps_mouse_set_visibility(visible)
}

/// `Rs::RsMouseSetPos` (0x6194A0): `jmp psMouseSetPos`.
#[hook(addr = 0x6194A0, conv = cdecl, name = "Rs::RsMouseSetPos", expect = "E9 4B BF 12 00 90")]
pub fn rs_mouse_set_pos(pos: GPtr<[f32; 2]>) {
    call::ps_mouse_set_pos(pos)
}

/// `Rs::RsSelectDevice` (0x6194B0): `jmp psSelectDevice`.
#[hook(addr = 0x6194B0, conv = cdecl, name = "Rs::RsSelectDevice", expect = "E9 DB CC 12 00 90")]
pub fn rs_select_device() -> i32 {
    call::ps_select_device()
}

/// `Rs::RsInputDeviceAttach(RsInputDeviceType, RsInputEventHandler)` (0x6194C0): stores the handler
/// and then `used = TRUE` for keyboard (0) / mouse (1) / pad (2), returns TRUE; other types return
/// FALSE.
#[hook(addr = 0x6194C0, conv = cdecl, name = "Rs::RsInputDeviceAttach", expect = "8B 44 24 04 83 E8 00")]
pub fn rs_input_device_attach(device: i32, handler: u32) -> i32 {
    let dev = match device {
        0 => RS_KEYBOARD.ptr(),
        1 => RS_MOUSE.ptr(),
        2 => RS_PAD.ptr(),
        _ => return 0,
    };
    field::<_, u32>(dev, 8).write(handler);
    field::<_, i32>(dev, 4).write(1);
    1
}

/// `Rs::rsCommandLine(void* arg)` (0x619510): the argument arrives in **EAX** (whole-program
/// optimised convention): `RsEventHandler(rsFILELOAD, arg); return TRUE`. Not hookable with the
/// supported conventions.
pub fn rs_command_line(arg: GPtr<u8>) -> i32 {
    call::rs_event_handler(2, arg);
    1
}

/// `"-vms"` compared with `repe cmpsb` over 5 bytes (terminator included), stopping at the first
/// difference (shorter strings are never read past their mismatch).
fn is_vms(arg: GPtr<u8>) -> bool {
    const VMS: &[u8; 5] = b"-vms\0";
    (0..5).all(|i| arg.byte_add(i as isize).read() == VMS[i])
}

/// `Rs::rsPreInitCommandLine(char* arg)` (0x619530): argument in **EAX** (not hookable). `-vms`
/// clears `DefaultVM` and returns TRUE, anything else FALSE.
pub fn rs_pre_init_command_line(arg: GPtr<u8>) -> i32 {
    if is_vms(arg) {
        DEFAULT_VM.set(0);
        1
    } else {
        0
    }
}

/// `Rs::RsKeyboardEventHandler(RsEvent, void*)` (0x619560): `used` is read through the protection
/// chunk at 0x40591D ([`protected_cell`]); if set, tail-jumps to the keyboard handler with the same
/// arguments, else returns rsEVENTNOTPROCESSED (2).
#[hook(addr = 0x619560, conv = cdecl, name = "Rs::RsKeyboardEventHandler", expect = "E9 B8 C3 DE FF 85 C0")]
pub fn rs_keyboard_event_handler(event: i32, param: GPtr<u8>) -> i32 {
    if protected_cell(&PROT_KBD_A, &PROT_KBD_B, keyboard_used()) != 0 {
        call::keyboard_handler(event, param)
    } else {
        2
    }
}

/// `Rs::RsMouseEventHandler` (0x619580).
#[hook(addr = 0x619580, conv = cdecl, name = "Rs::RsMouseEventHandler", expect = "A1 68 70 C1 00")]
pub fn rs_mouse_event_handler(event: i32, param: GPtr<u8>) -> i32 {
    if RS_MOUSE.get().used != 0 {
        call::mouse_handler(event, param)
    } else {
        2
    }
}

/// `Rs::RsPadEventHandler` (0x6195A0).
#[hook(addr = 0x6195A0, conv = cdecl, name = "Rs::RsPadEventHandler", expect = "A1 74 70 C1 00")]
pub fn rs_pad_event_handler(event: i32, param: GPtr<u8>) -> i32 {
    if RS_PAD.get().used != 0 {
        call::pad_handler(event, param)
    } else {
        2
    }
}

/// `Rs::RsPathnameDestroy` (0x6195C0): `jmp psPathnameDestroy`.
#[hook(addr = 0x6195C0, conv = cdecl, name = "Rs::RsPathnameDestroy", expect = "E9 1B BF 12 00 90")]
pub fn rs_pathname_destroy(p: GPtr<u8>) {
    call::ps_pathname_destroy(p)
}

/// `Rs::RsPathnameCreate` (0x6195D0): `jmp psPathnameCreate`.
#[hook(addr = 0x6195D0, conv = cdecl, name = "Rs::RsPathnameCreate", expect = "E9 9B BE 12 00 90")]
pub fn rs_pathname_create(p: GPtr<u8>) -> GPtr<u8> {
    call::ps_pathname_create(p)
}

/// `Rs::RsRwTerminate` (0x6195E0): `RwEngineStop(); RwEngineClose(); RwEngineTerm()` (tail jump).
#[hook(addr = 0x6195E0, conv = cdecl, name = "Rs::RsRwTerminate", expect = "E8 3B 98 1D 00")]
pub fn rs_rw_terminate() {
    call::rw_engine_stop();
    call::rw_engine_close();
    call::rw_engine_term();
}

/// `Rs::RsTerminate` (0x6195F0): `jmp psTerminate`.
#[hook(addr = 0x6195F0, conv = cdecl, name = "Rs::RsTerminate", expect = "E9 AB C2 12 00 90")]
pub fn rs_terminate() {
    call::ps_terminate()
}

/// `Rs::RsInitialize` (0x619600): fills `RsGlobal` (appName = "GTA: San Andreas" 0x86D388,
/// 640x480, frameLimit 30, quit 0, devices keyboard/mouse/pad with used = handler = 0; `ps` is left
/// alone), then tail-jumps to `psInitialize`. Store order as in the bytes.
#[hook(addr = 0x619600, conv = cdecl, name = "Rs::RsInitialize", expect = "33 C0 C7 05 40 70 C1 00 88 D3 86 00")]
pub fn rs_initialize() -> i32 {
    RS_APP_NAME.set(rdata(0x86D388));
    RS_MAXIMUM_WIDTH.set(0x280);
    RS_MAXIMUM_HEIGHT.set(0x1E0);
    RS_FRAME_LIMIT.set(0x1E);
    RS_QUIT.set(0);
    let (k, m, p) = (RS_KEYBOARD.ptr(), RS_MOUSE.ptr(), RS_PAD.ptr());
    field::<_, i32>(k, 0).write(0);
    field::<_, u32>(k, 8).write(0);
    field::<_, i32>(k, 4).write(0);
    field::<_, i32>(m, 0).write(1);
    field::<_, u32>(m, 8).write(0);
    field::<_, i32>(m, 4).write(0);
    field::<_, i32>(p, 0).write(2);
    field::<_, u32>(p, 8).write(0);
    field::<_, i32>(p, 4).write(0);
    call::ps_initialize()
}

/// `Rs::RsGrabScreen` (0x619AF0): `jmp psGrabScreen`.
#[hook(addr = 0x619AF0, conv = cdecl, name = "Rs::RsGrabScreen", expect = "E9 BB B7 12 00 90")]
pub fn rs_grab_screen(camera: GPtr<u8>) -> GPtr<u8> {
    call::ps_grab_screen(camera)
}

/// `rsERRORMESSAGE`-style notification shared by RsErrorMessage / RsWarningMessage: if the
/// keyboard is used, `keyboard.handler(0x1D, &local)` with a local holding 0x420.
fn notify_keyboard_0x1d() {
    if RS_KEYBOARD.get().used != 0 {
        let mut local: u32 = 0x420;
        call::keyboard_handler(0x1D, GPtr::from_raw(&raw mut local).cast());
    }
}

/// `Rs::RsErrorMessage(const char*)` (0x619B00): keyboard event 0x1D with 0x420, then
/// `psErrorMessage(msg)`; returns its result.
#[hook(addr = 0x619B00, conv = cdecl, name = "Rs::RsErrorMessage", expect = "51 A1 5C 70 C1 00")]
pub fn rs_error_message(msg: GPtr<u8>) -> i32 {
    notify_keyboard_0x1d();
    call::ps_error_message(msg)
}

/// `Rs::RsWarningMessage(const char*)` (0x619B30): as RsErrorMessage with `psWarningMessage`.
#[hook(addr = 0x619B30, conv = cdecl, name = "Rs::RsWarningMessage", expect = "51 A1 5C 70 C1 00")]
pub fn rs_warning_message(msg: GPtr<u8>) -> i32 {
    notify_keyboard_0x1d();
    call::ps_warning_message(msg)
}

/// `Rs::RsEventHandler(RsEvent, void*)` (0x619B60). `AppEventHandler` first; event 0x1E
/// (rsQUITAPP) sets `RsGlobal.quit = 1` whatever it returned. Only when it returns
/// rsEVENTNOTPROCESSED (2) does the skeleton handle the event (jump table 0x619C40/0x619C64):
/// 1 → `RsEventHandler(2, param)`, TRUE; 3, 10 → TRUE; 0x15 → `RsRwInitialize(param) != 0`;
/// 0x16 → `RsRwTerminate()`, TRUE; 0x17 → `psSelectDevice() != 0`; 0x18 → `RsInitialize() != 0`;
/// 0x19 → `psTerminate()`, TRUE; 0x25 → the `-vms` test of rsPreInitCommandLine (clears
/// `DefaultVM`, TRUE) else FALSE; anything else → 2. Other results of AppEventHandler are returned
/// unchanged.
#[hook(addr = 0x619B60, conv = cdecl, name = "Rs::RsEventHandler", expect = "56 8B 74 24 0C 57")]
pub fn rs_event_handler(event: i32, param: GPtr<u8>) -> i32 {
    let r = call::app_event_handler(event, param);
    if event == 0x1E {
        RS_QUIT.set(1);
    }
    if r != 2 {
        return r;
    }
    match event {
        1 => {
            call::rs_event_handler(2, param);
            1
        }
        3 | 10 => 1,
        0x15 => (call::rs_rw_initialize(param) != 0) as i32,
        0x16 => {
            call::rs_rw_terminate();
            1
        }
        0x17 => (call::ps_select_device() != 0) as i32,
        0x18 => (call::rs_initialize() != 0) as i32,
        0x19 => {
            call::ps_terminate();
            1
        }
        0x25 => rs_pre_init_command_line(param),
        _ => 2,
    }
}

// ------------------------------------------------------------------------------ small leaves ----

/// `CMBlur::MotionBlurClose` (0x71D720): `return TRUE`.
#[hook(addr = 0x71D720, conv = cdecl, name = "CMBlur::MotionBlurClose", expect = "B8 01 00 00 00")]
pub fn motion_blur_close() -> i32 {
    1
}

/// `CTxdStore::PluginAttach` (0x731650): registers the TXD parent plugin (size 4, id 0x253F2F5,
/// ctor/dtor/copy 0x731600/0x731620/0x731630), stores the offset in `ms_txdPluginOffset`, returns
/// `offset != -1` in AL.
#[hook(addr = 0x731650, conv = cdecl, name = "CTxdStore::PluginAttach", expect = "68 30 16 73 00")]
pub fn txd_store_plugin_attach() -> u8 {
    let off = call::rw_tex_dictionary_register_plugin(4, 0x253F2F5, 0x731600, 0x731620, 0x731630);
    TXD_PLUGIN_OFFSET.set(off);
    (off != -1) as u8
}

/// `CreateDebugFont` (0x734610): a lone `ret` (not hookable).
pub fn create_debug_font() {}

/// `DestroyDebugFont` (0x734620): a lone `ret` (not hookable).
pub fn destroy_debug_font() {}

/// `FlushObrsPrintfs` (0x734640): a lone `ret` (not hookable).
pub fn flush_obrs_printfs() {}

/// `Securom::SecuromDisplayInit` (0x744A70): both 32-entry display arrays = 1000 (`rep stosd`).
#[hook(addr = 0x744A70, conv = cdecl, name = "Securom::SecuromDisplayInit", expect = "57 B8 E8 03 00 00")]
pub fn securom_display_init() {
    SECUROM_TRAP_DISPLAY.set([1000; 32]);
    SECUROM_TRIGGER_DISPLAY.set([1000; 32]);
}

/// `Win::IsForegroundApp` (0x746060): `ForegroundApp != 0` in AL.
#[hook(addr = 0x746060, conv = cdecl, name = "Win::IsForegroundApp", expect = "A1 1C 62 8D 00")]
pub fn is_foreground_app() -> u8 {
    (FOREGROUND_APP.get() != 0) as u8
}

// ------------------------------------------------------------------------------------- Ps -------

/// `Ps::psWindowSetText(const char*)` (0x7451B0): `SetWindowTextA(ps->window, text)`.
#[hook(addr = 0x7451B0, conv = cdecl, name = "Ps::psWindowSetText", expect = "8B 44 24 04 8B 0D 54 70 C1 00")]
pub fn ps_window_set_text(text: GPtr<u8>) -> i32 {
    let hwnd = field::<_, u32>(RS_PS.get(), 0).read();
    call::set_window_text_a(hwnd, text)
}

/// `Ps::psErrorMessage(const char*)` (0x7451D0): `MessageBoxA(NULL, msg, appName, 0x42010)`.
#[hook(addr = 0x7451D0, conv = cdecl, name = "Ps::psErrorMessage", expect = "A1 40 70 C1 00")]
pub fn ps_error_message(msg: GPtr<u8>) -> i32 {
    call::message_box_a_error(0, msg, RS_APP_NAME.get(), 0x42010)
}

/// `Ps::psWarningMessage(const char*)` (0x7451F0): `MessageBoxA(NULL, msg, appName, 0x42030)`.
#[hook(addr = 0x7451F0, conv = cdecl, name = "Ps::psWarningMessage", expect = "A1 40 70 C1 00")]
pub fn ps_warning_message(msg: GPtr<u8>) -> i32 {
    call::message_box_a_warning(0, msg, RS_APP_NAME.get(), 0x42030)
}

/// `Ps::psCameraBeginUpdate(RwCamera*)` (0x745210): note the original passes **`Scene.camera`**
/// (0xC1703C), not its argument, to `RwCameraBeginUpdate`; on failure
/// `RsEventHandler(0x26, NULL)` and FALSE.
#[hook(addr = 0x745210, conv = cdecl, name = "Ps::psCameraBeginUpdate", expect = "A1 3C 70 C1 00")]
pub fn ps_camera_begin_update(_camera: GPtr<u8>) -> i32 {
    if call::rw_camera_begin_update(SCENE_CAMERA.get()).is_null() {
        call::rs_event_handler(0x26, GPtr::NULL);
        0
    } else {
        1
    }
}

/// `Ps::psCameraShowRaster(RwCamera*)` (0x745240): flags = 1 (wait for vsync) when the frame
/// limiter or the loading screen is on, else 0; `RwCameraShowRaster(camera, ps->window, flags)`.
#[hook(addr = 0x745240, conv = cdecl, name = "Ps::psCameraShowRaster", expect = "A0 94 67 BA 00")]
pub fn ps_camera_show_raster(camera: GPtr<u8>) -> GPtr<u8> {
    let flags = (PREFS_FRAME_LIMITER.get() != 0 || LOADING_SCREEN_ACTIVE.get() != 0) as u32;
    let hwnd = field::<_, u32>(RS_PS.get(), 0).read();
    call::rw_camera_show_raster(camera, hwnd, flags)
}

/// `Ps::psTimer` (0x745270): `timeGetDevCaps(&caps, 8); timeBeginPeriod(caps.wPeriodMin);
/// t = timeGetTime(); timeEndPeriod(caps.wPeriodMin); return t` (gta-reversed: `OS_TimeMS()`).
#[hook(addr = 0x745270, conv = cdecl, name = "Ps::psTimer", expect = "83 EC 08 56 6A 08")]
pub fn ps_timer() -> u32 {
    let mut caps = [0u32; 2];
    call::time_get_dev_caps(GPtr::from_raw(&raw mut caps), 8);
    call::time_begin_period(caps[0]);
    let t = call::time_get_time();
    call::time_end_period(caps[0]);
    t
}

/// `Ps::psMouseSetVisibility(RwBool)` (0x7453E0): `ShowCursor(visible)` (its result in EAX).
#[hook(addr = 0x7453E0, conv = cdecl, name = "Ps::psMouseSetVisibility", expect = "8B 44 24 04 50")]
pub fn ps_mouse_set_visibility(visible: i32) -> i32 {
    call::show_cursor(visible)
}

/// `_ftol2` of an f32 as the original's `fld dword; call _ftol` (low dword).
fn ftol_f32(v: f32) -> i32 {
    X87::from_f32(v).ftol() as u32 as i32
}

/// `Ps::psMouseSetPos(RwV2d*)` (0x7453F0): `pt = {ftol(x), ftol(y)}; ClientToScreen(window, &pt);
/// SetCursorPos(pt.x, pt.y)`; then `lastMousePos = {(float)ftol(x), (float)ftol(y)}` (converted again
/// from the argument, re-read after the calls, `fild` = exact i32 → f32 rounding).
#[hook(addr = 0x7453F0, conv = cdecl, name = "Ps::psMouseSetPos", expect = "83 EC 10 56 8B 74 24 18")]
pub fn ps_mouse_set_pos(pos: GPtr<[f32; 2]>) {
    let p = pos.read();
    let mut pt = [ftol_f32(p[0]), ftol_f32(p[1])];
    let hwnd = field::<_, u32>(RS_PS.get(), 0).read();
    call::client_to_screen(hwnd, GPtr::from_raw(&raw mut pt));
    call::set_cursor_pos(pt[0], pt[1]);
    let p = pos.read();
    let (x, y) = (ftol_f32(p[0]), ftol_f32(p[1]));
    field::<_, f32>(RS_PS.get(), 0x0C).write(x as f32);
    field::<_, f32>(RS_PS.get(), 0x10).write(y as f32);
}

/// `Ps::psPathnameCreate(const char*)` (0x745470): `p = CMemoryMgr::Malloc(rwstrlen(s) + 1, 0)`
/// (a direct call, with RwMalloc's hint 0 left on the stack); if non-null `rwstrcpy(p, s)` and every
/// '/' found by `rwstrchr` becomes '\\'.
#[hook(addr = 0x745470, conv = cdecl, name = "Ps::psPathnameCreate", expect = "A1 24 7B C9 00")]
pub fn ps_pathname_create(s: GPtr<u8>) -> GPtr<u8> {
    let len = call::rw_strlen(s);
    let p = call::memory_malloc(len.wrapping_add(1), 0);
    if !p.is_null() {
        call::rw_strcpy(p, s);
        let mut c = call::rw_strchr(p, b'/' as i32);
        while !c.is_null() {
            c.write(b'\\');
            c = call::rw_strchr(p, b'/' as i32);
        }
    }
    p
}

/// `Ps::psPathnameDestroy(char*)` (0x7454E0): `if (p) RwFree(p)` (tail jump through the engine's
/// memory functions).
#[hook(addr = 0x7454E0, conv = cdecl, name = "Ps::psPathnameDestroy", expect = "8B 44 24 04 85 C0")]
pub fn ps_pathname_destroy(p: GPtr<u8>) {
    if !p.is_null() {
        call::rw_free(p);
    }
}

/// `Ps::psPathGetSeparator` (0x745500): '\\' in AL. `mov al, 5Ch; ret` is 3 bytes: not hookable.
pub fn ps_path_get_separator() -> u8 {
    b'\\'
}

/// `psGetMemoryFunctions` (0x745510): `&RwMemoryFunctions` (0x8D6228).
#[hook(addr = 0x745510, conv = cdecl, name = "psGetMemoryFunctions", expect = "B8 28 62 8D 00")]
pub fn ps_get_memory_functions() -> GPtr<[u32; 4]> {
    MEMORY_FUNCTIONS.ptr()
}

/// `Ps::psInstallFileSystem` (0x745520): `return TRUE`.
#[hook(addr = 0x745520, conv = cdecl, name = "Ps::psInstallFileSystem", expect = "B8 01 00 00 00")]
pub fn ps_install_file_system() -> i32 {
    1
}

/// `Ps::psNativeTextureSupport` (0x745530): `jmp RwD3D9DeviceSupportsDXTTexture`.
#[hook(addr = 0x745530, conv = cdecl, name = "Ps::psNativeTextureSupport", expect = "E9 FB 46 0B 00 90")]
pub fn ps_native_texture_support() -> i32 {
    call::rw_d3d9_device_supports_dxt_texture()
}

/// `Ps::psDebugMessageHandler(RwDebugType, const RwChar*)` (0x745540): `OutputDebugStringA(msg);
/// OutputDebugStringA("\n")` (0x874814), both through the IAT slot loaded into ESI.
#[hook(addr = 0x745540, conv = cdecl, name = "Ps::psDebugMessageHandler", expect = "8B 44 24 08 56")]
pub fn ps_debug_message_handler(_ty: i32, msg: GPtr<u8>) {
    call::output_debug_string_a(msg);
    call::output_debug_string_a(rdata(0x874814));
}

/// `Ps::psTerminate` (0x7458A0): a lone `ret` (not hookable).
pub fn ps_terminate() {}

/// `Win::Win32_InitInstance(HINSTANCE)` (0x745560): `rect = {0, 0, maxW, maxH};
/// AdjustWindowRect(&rect, WS_OVERLAPPEDWINDOW, FALSE); return CreateWindowExA(0, AppClassName,
/// appName, WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, right - left, bottom - top, NULL,
/// NULL, instance, NULL)`.
#[hook(addr = 0x745560, conv = cdecl, name = "Win::Win32_InitInstance", expect = "83 EC 10 A1 44 70 C1 00")]
pub fn win32_init_instance(instance: u32) -> u32 {
    let mut rect = [0, 0, RS_MAXIMUM_WIDTH.get(), RS_MAXIMUM_HEIGHT.get()];
    call::adjust_window_rect(GPtr::from_raw(&raw mut rect), 0xCF0000, 0);
    call::create_window_ex_a(
        0,
        rdata(APP_CLASS_NAME.get()),
        RS_APP_NAME.get(),
        0xCF0000,
        0x8000_0000u32 as i32,
        0x8000_0000u32 as i32,
        rect[2].wrapping_sub(rect[0]),
        rect[3].wrapping_sub(rect[1]),
        0,
        0,
        instance,
        0,
    )
}

/// `Ps::GetVideoMemInfo(LPDWORD total, LPDWORD available)` (0x7455E0): `DirectDrawCreateEx(NULL,
/// &dd, IID_IDirectDraw7 (0x859694), NULL)`; on failure its HRESULT; else `caps = {DDSCAPS_VIDEOMEMORY
/// (0x4000), 0, 0, 0}; hr = dd->GetAvailableVidMem(&caps, total, available); dd->Release(); hr`.
#[hook(addr = 0x7455E0, conv = cdecl, name = "Ps::GetVideoMemInfo", expect = "83 EC 14 6A 00")]
pub fn get_video_mem_info(total: GPtr<u32>, available: GPtr<u32>) -> i32 {
    let mut dd: u32 = 0;
    let hr = call::direct_draw_create_ex(0, GPtr::from_raw(&raw mut dd), rdata(0x859694), 0);
    if hr < 0 {
        return hr;
    }
    let mut caps = [0x4000u32, 0, 0, 0];
    let ddp = GPtr::<u32>::from_addr(dd as usize);
    let hr = call::dd_get_available_vid_mem(ddp, GPtr::from_raw(&raw mut caps), total, available);
    call::com_release(ddp);
    hr
}

/// `FUN_00745650` (gta-reversed: the DirectX check that `psInitialize` keeps as an original-only
/// comment): returns 0 (no DDRAW / DirectDrawCreateEx), 0x700 (no D3D8.DLL), 0x800 (no
/// dpnhpast.dll), 0x801 (no D3D9.DLL) or 0x900, printing the matching message with
/// OutputDebugStringA. The libraries are freed in the original's order (note: on the 0x800 path
/// `FreeLibrary(NULL)` is called first, as `push eax` with eax = 0).
#[hook(addr = 0x745650, conv = cdecl, name = "FUN_00745650", expect = "51 55 8B 2D 70 80 85 00")]
pub fn check_dx_version() -> u32 {
    let ddraw = call::load_library_a(rdata(0x874A7C));
    if ddraw == 0 {
        call::output_debug_string_a(rdata(0x874A28));
        return 0;
    }
    let create = call::get_proc_address(ddraw, rdata(0x874A14));
    if create == 0 {
        call::free_library(ddraw);
        call::output_debug_string_a(rdata(0x8749B0));
        return 0;
    }
    let mut dd: u32 = 0;
    if call::direct_draw_create_ex_dyn(create, 0, GPtr::from_raw(&raw mut dd), rdata(0x859694), 0) < 0 {
        call::free_library(ddraw);
        call::output_debug_string_a(rdata(0x874958));
        return 0;
    }
    call::com_release(GPtr::from_addr(dd as usize));
    let d3d8 = call::load_library_a(rdata(0x874948));
    if d3d8 == 0 {
        call::free_library(ddraw);
        call::output_debug_string_a(rdata(0x8748F0));
        return 0x700;
    }
    let dpnhpast = call::load_library_a(rdata(0x8748DC));
    if dpnhpast == 0 {
        call::free_library(0);
        call::free_library(ddraw);
        call::free_library(d3d8);
        call::output_debug_string_a(rdata(0x874880));
        return 0x800;
    }
    let d3d9 = call::load_library_a(rdata(0x874870));
    call::free_library(dpnhpast);
    call::free_library(ddraw);
    call::free_library(d3d8);
    if d3d9 == 0 {
        call::output_debug_string_a(rdata(0x874818));
        return 0x801;
    }
    call::free_library(d3d9);
    0x900
}

/// `Ps::CheckDirectSound` (0x745840): `CoInitialize(NULL)`; `DirectSoundCreate(NULL, &ds, NULL)`
/// failing → FALSE; else `caps.dwSize = 0x60; ok = ds->GetCaps(&caps) == 0; ds->Release(); ok`
/// (only `dwSize` of the caps is initialised by the original).
#[hook(addr = 0x745840, conv = cdecl, name = "Ps::CheckDirectSound", expect = "83 EC 64 56 6A 00")]
pub fn check_direct_sound() -> i32 {
    call::co_initialize_ds(0);
    let mut ds: u32 = 0;
    if call::direct_sound_create(0, GPtr::from_raw(&raw mut ds), 0) < 0 {
        return 0;
    }
    let mut caps = [0u32; 24];
    caps[0] = 0x60;
    let dsp = GPtr::<u32>::from_addr(ds as usize);
    let ok = (call::ds_get_caps(dsp, GPtr::from_raw(&raw mut caps)) == 0) as i32;
    call::com_release(dsp);
    ok
}

/// `Ps::psAlwaysOnTop(RwBool)` (0x7458B0): `GetWindowRect(window, &r)` then `SetWindowPos(window,
/// on ? HWND_TOPMOST : HWND_NOTOPMOST, r.left, r.top, r.right - r.left, r.bottom - r.top, 0)`.
#[hook(addr = 0x7458B0, conv = cdecl, name = "Ps::psAlwaysOnTop", expect = "A1 54 70 C1 00")]
pub fn ps_always_on_top(on_top: i32) -> i32 {
    let hwnd = field::<_, u32>(RS_PS.get(), 0).read();
    let mut r = [0i32; 4];
    call::get_window_rect(hwnd, GPtr::from_raw(&raw mut r));
    let (w, h) = (r[2].wrapping_sub(r[0]), r[3].wrapping_sub(r[1]));
    if on_top != 0 {
        call::set_window_pos_top(hwnd, -1, r[0], r[1], w, h, 0)
    } else {
        call::set_window_pos_notop(hwnd, -2, r[0], r[1], w, h, 0)
    }
}

/// `Win::IsAlreadyRunning` (0x7468E0): `CreateEventA(NULL, FALSE, TRUE, AppClassName)`; if
/// `GetLastError() == ERROR_ALREADY_EXISTS` (0xB7): bring `FindWindowA(AppClassName, appName)` (or,
/// if none, our own window) to the foreground and return TRUE; else FALSE.
#[hook(addr = 0x7468E0, conv = cdecl, name = "Win::IsAlreadyRunning", expect = "A1 24 62 8D 00")]
pub fn is_already_running() -> i32 {
    call::create_event_a(0, 0, 1, rdata(APP_CLASS_NAME.get()));
    if call::get_last_error() != 0xB7 {
        return 0;
    }
    let w = call::find_window_a(rdata(APP_CLASS_NAME.get()), RS_APP_NAME.get());
    if w != 0 {
        call::set_foreground_window_found(w);
    } else {
        call::set_foreground_window_own(field::<_, u32>(RS_PS.get(), 0).read());
    }
    1
}

/// The text language for a keyboard-layout primary language id (jump table 0x7466B0/0x7466C4):
/// German 7 → 2, Spanish 0xA → 4, French 0xC → 1, Italian 0x10 → 3, else 0 (American).
pub fn text_language_for(kb: u32) -> i32 {
    match kb {
        7 => 2,
        0xA => 4,
        0xC => 1,
        0x10 => 3,
        _ => 0,
    }
}

/// The value the original compares with LANG_SPANISH for the prefs language: the user LCID's primary
/// id if it is one of 7/0xC/0xA/0x10, else `(((sub != 3) - 1) & 0xFFFFFF94) + 9` with `sub` =
/// `(uint16)userLCID >> 10`.
pub fn prefs_language_key(user_primary: u32, user_lcid: u32) -> u32 {
    if matches!(user_primary, 7 | 0xC | 0xA | 0x10) {
        user_primary
    } else {
        let sub = (user_lcid & 0xFFFF) >> 10;
        ((((sub != 3) as u32).wrapping_sub(1)) & 0xFFFF_FF94).wrapping_add(9)
    }
}

/// `Ps::InitialiseLanguage` (0x7465B0). Calls `GetSystemDefaultLCID` and `GetKeyboardLayout(0)`
/// twice each and `GetUserDefaultLCID` twice, as the bytes do (*new* vs gta-reversed, which calls
/// each once; harmless). Title language = system LCID & 0x3FF; text language from the keyboard layout
/// ([`text_language_for`]); prefs language (a byte) = 4 if [`prefs_language_key`] is 0xA else 0;
/// then `TheText.Unload(false); TheText.Load(false)`.
#[hook(addr = 0x7465B0, conv = cdecl, name = "Ps::InitialiseLanguage", expect = "53 55 56 57 8B 3D 48 82 85 00")]
pub fn initialise_language() {
    let system = call::get_system_default_lcid() & 0x3FF;
    let user = call::get_user_default_lcid_1() & 0x3FF;
    let kb = call::get_keyboard_layout_1(0) & 0x3FF;
    call::get_system_default_lcid();
    let user_lcid = call::get_user_default_lcid_2();
    call::get_keyboard_layout_2(0);
    let key = prefs_language_key(user, user_lcid);
    TITLE_LANGUAGE.set(system as i32);
    TEXT_LANGUAGE.set(text_language_for(kb));
    PREFS_LANGUAGE.set(if key == 0xA { 4 } else { 0 });
    let the_text = GPtr::<u8>::from_addr(0xC1B340);
    call::text_unload(the_text, 0);
    call::text_load(the_text, 0);
}

/// `FUN_007477C0` (gta-reversed: part of `diPadInit`'s caller): `di->EnumDevices(DI8DEVCLASS_GAMECTRL
/// (4), EnumDevicesCallback 0x747020, NULL, DIEDFL_ATTACHEDONLY (1))`; a failure HRESULT is returned
/// as is; with no first pad → 1; else `diPadSetRanges(pad1, 0)`, `diPadSetRanges(pad2, 1)` if there
/// is a second, → 0.
#[hook(addr = 0x7477C0, conv = cdecl, name = "FUN_007477c0", expect = "A1 54 70 C1 00 8B 40 18")]
pub fn enum_pads() -> i32 {
    let ps = RS_PS.get();
    let di = GPtr::<u32>::from_addr(field::<_, u32>(ps, 0x18).read() as usize);
    let hr = call::di_enum_devices(di, 4, 0x747020, 0, 1);
    if hr != 0 {
        return hr;
    }
    let pad1 = field::<_, u32>(RS_PS.get(), 0x20).read();
    if pad1 == 0 {
        return 1;
    }
    call::di_pad_set_ranges(pad1, 0);
    let pad2 = field::<_, u32>(RS_PS.get(), 0x24).read();
    if pad2 != 0 {
        call::di_pad_set_ranges(pad2, 1);
    }
    0
}

// ----------------------------------------------------------------------------------- App --------

/// `App::DoRWStuffEndOfFrame` (0x53D840): `CDebug::DebugDisplayTextBuffer(); FlushObrsPrintfs();
/// RwCameraEndUpdate(Scene.camera); RsCameraShowRaster(Scene.camera)`; the first `Scene.camera` read
/// goes through the protection chunk at 0x4037CB ([`protected_cell`]).
#[hook(addr = 0x53D840, conv = cdecl, name = "App::DoRWStuffEndOfFrame", expect = "E8 1B 4A FF FF")]
pub fn do_rw_stuff_end_of_frame() {
    call::debug_display_text_buffer();
    call::flush_obrs_printfs();
    let cam = protected_cell(&PROT_EOF_A, &PROT_EOF_B, SCENE_CAMERA.ptr().cast());
    call::rw_camera_end_update(GPtr::from_addr(cam as usize));
    call::rs_camera_show_raster(SCENE_CAMERA.get());
}

/// `App::PluginAttach` (0x53D870): the RenderWare and game plugins in the original's order; the
/// first failure returns FALSE. `RpAnisotPluginAttach`'s result is ignored; the `CTxdStore`,
/// `CCollisionPlugin` and `C2dEffect` results are bytes (`test al, al`); the last result is
/// normalised to 0/1 (`neg; sbb; neg`).
#[hook(addr = 0x53D870, conv = cdecl, name = "App::PluginAttach", expect = "E8 2B 25 21 00")]
pub fn plugin_attach() -> i32 {
    let dword_steps: [fn() -> i32; 7] = [
        call::rp_world_plugin_attach,
        call::rp_skin_plugin_attach,
        call::rt_anim_initialize,
        call::rp_hanim_plugin_attach,
        call::node_name_plugin_attach,
        call::visibility_plugins_attach,
        call::rp_anim_blend_plugin_attach,
    ];
    if dword_steps.iter().any(|f| f() == 0) {
        return 0;
    }
    if call::txd_store_plugin_attach() == 0 {
        return 0;
    }
    let more: [fn() -> i32; 4] = [
        call::rp_mat_fx_plugin_attach,
        call::rp_uv_anim_plugin_attach,
        call::custom_building_renderer_plugin_attach,
        call::car_fx_renderer_register_plugins,
    ];
    if more.iter().any(|f| f() == 0) {
        return 0;
    }
    call::rp_anisot_plugin_attach();
    if call::breakable_plugin_attach() == 0
        || call::collision_plugin_attach() == 0
        || call::effect_2d_plugin_attach() == 0
    {
        return 0;
    }
    (call::pipeline_plugin_attach() != 0) as i32
}

/// `CFileMgr::InitUserDirectories` (0x744FB0; the entry pushes `KEY_READ` 0x20019 through the
/// protection chunk 0x406807, `.HOODLUM` 0x156081C minus `.text` 0x53D84A). Reads the "Personal"
/// shell folder from `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders` into
/// the user directory buffer (0xC92368, size 0x100). On success: append
/// `"\GTA San Andreas User Files"`, create it if `CreateFileA(OPEN_EXISTING, BACKUP_SEMANTICS)`
/// fails (else close the handle), then the same for `<dir>\Gallery` (0xC92268) and
/// `<dir>\User Tracks` (0xC92168). On failure the buffer becomes `"data"` (the key is closed when
/// only the query failed). Returns the buffer. The NOTSA wide-char version of gta-reversed is not
/// followed.
#[hook(addr = 0x744FB0, conv = cdecl, name = "CFileMgr::InitUserDirectories", expect = "83 EC 0C 8D 44 24 00")]
pub fn init_user_directories() -> GPtr<u8> {
    let user = USER_DIR.ptr().cast::<u8>();
    let sam = PROT_SAM_A.get().wrapping_sub(PROT_SAM_B.get());
    let mut hkey: u32 = 0;
    let fail = |close: Option<u32>| {
        if let Some(k) = close {
            call::reg_close_key_fail(k);
        }
        copy_bytes(user, rdata(0x874784), 5);
        user
    };
    if call::reg_open_key_ex_a(
        0x8000_0001,
        rdata(0x8747D0),
        0,
        sam,
        GPtr::from_raw(&raw mut hkey),
    ) != 0
    {
        return fail(None);
    }
    let (mut size, mut ty) = (0x100u32, 0u32);
    if call::reg_query_value_ex_a(
        hkey,
        rdata(0x8747C4),
        0,
        GPtr::from_raw(&raw mut ty),
        user,
        GPtr::from_raw(&raw mut size),
    ) != 0
    {
        return fail(Some(hkey));
    }
    call::reg_close_key_ok(hkey);
    let ensure_dir = |dir: GPtr<u8>| {
        let h = call::create_file_a(dir, 0x8000_0000, 1, 0, 3, 0x200_0080, 0);
        if h == 0xFFFF_FFFF {
            call::create_directory_a(dir, 0);
        } else {
            call::close_handle(h);
        }
    };
    append(user, rdata(0x8747A8), 28);
    ensure_dir(user);
    let gallery = GALLERY_DIR.ptr().cast::<u8>();
    copy_c_string(gallery, user);
    append(gallery, rdata(0x87479C), 9);
    ensure_dir(gallery);
    let tracks = USER_TRACKS_DIR.ptr().cast::<u8>();
    copy_c_string(tracks, user);
    append(tracks, rdata(0x87478C), 13);
    ensure_dir(tracks);
    user
}

/// Copies `n` bytes (the inlined `rep movsd` / dword+byte copies of string literals).
fn copy_bytes(dst: GPtr<u8>, src: GPtr<u8>, n: usize) {
    for i in 0..n as isize {
        dst.byte_add(i).write(src.byte_add(i).read());
    }
}

/// Inlined `strcpy` (byte loop up to and including the terminator).
fn copy_c_string(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut i = 0isize;
    loop {
        let c = src.byte_add(i).read();
        dst.byte_add(i).write(c);
        if c == 0 {
            break;
        }
        i += 1;
    }
}

/// Inlined `strcat` of an `n`-byte literal (terminator included) at the end of `dst`.
fn append(dst: GPtr<u8>, lit: GPtr<u8>, n: usize) {
    let mut end = 0isize;
    while dst.byte_add(end).read() != 0 {
        end += 1;
    }
    copy_bytes(dst.byte_add(end), lit, n);
}

/// `Securom::SecuromStateDisplay` (0x744AE0): font set-up (colour 200,200,200,255, style 1, scale
/// 0.5 x 0.8, edge 2, drop colour black, proportional, wrap 640, no right-justify wrap, orientation 1),
/// then for rows i = 0..16 at y = 140 + 12 i: `Trigger: i` at x 30 and `Trigger: i+16` at x 160
/// (`triggerDisplay`), then `Trap: i` at x 350 and `Trap: i+16` at x 450 (`trapDisplay`), each shown
/// while its counter is non-zero and then decremented. Text through the CRT `sprintf` and
/// `AsciiToGxtChar`; y via `fild` (+2^32 if negative, never here).
#[hook(addr = 0x744AE0, conv = cdecl, name = "Securom::SecuromStateDisplay", expect = "81 EC 04 02 00 00")]
pub fn securom_state_display() {
    let mut rgba = 0u32;
    let c = call::crgba_ctor(GPtr::from_raw(&raw mut rgba), 200, 200, 200, 0xFF).read();
    call::font_set_color(c);
    call::font_set_font_style(1);
    call::font_set_scale(f32::from_bits(0x3F00_0000), f32::from_bits(0x3F4C_CCCD));
    call::font_set_edge(2);
    let c = call::crgba_ctor(GPtr::from_raw(&raw mut rgba), 0, 0, 0, 0xFF).read();
    call::font_set_drop_color(c);
    call::font_set_proportional(1);
    call::font_set_wrapx(640.0);
    call::font_set_right_justify_wrap(0);
    call::font_set_orientation(1);
    let mut text = [0u8; 0x100];
    let mut gxt = [0u8; 0x100];
    let mut show = |arr: GPtr<i32>, i: i32, n: i32, fmt: u32, x: f32, y: u32| {
        let e = arr.wrapping_index(i);
        if e.read() != 0 {
            call::crt_sprintf1(GPtr::from_raw(text.as_mut_ptr()), rdata(fmt), n);
            call::ascii_to_gxt_char(
                GPtr::from_raw(text.as_mut_ptr()),
                GPtr::from_raw(gxt.as_mut_ptr()),
            );
            let yf = X87::from_i32(y as i32).to_f32();
            call::font_print_string(x, yf, GPtr::from_raw(gxt.as_mut_ptr()));
            e.write(e.read().wrapping_sub(1));
        }
    };
    let trigger = SECUROM_TRIGGER_DISPLAY.ptr().cast::<i32>();
    let trap = SECUROM_TRAP_DISPLAY.ptr().cast::<i32>();
    for i in 0..16 {
        let y = 0x8C + 12 * i as u32;
        show(trigger, i, i, 0x872ECC, 30.0, y);
        show(trigger, i + 16, i + 16, 0x872ECC, 160.0, y);
    }
    for i in 0..16 {
        let y = 0x8C + 12 * i as u32;
        show(trap, i, i, 0x872EC0, 350.0, y);
        show(trap, i + 16, i + 16, 0x872EC0, 450.0, y);
    }
}

/// `Ps::psGrabScreen(RwCamera*)` (0x7452B0), the original (non-`FIX_BUGS`) path: `dev =
/// RwD3D9GetCurrentD3DDevice()`; `dev->GetDisplayMode(0, &mode)`; `dev->CreateOffscreenPlainSurface(
/// mode.Width, mode.Height, D3DFMT_A8R8G8B8, D3DPOOL_SCRATCH, &surface, NULL)`;
/// `dev->GetFrontBufferData(0, surface)`; windowed: the window rect, whose width/height replace the
/// mode's; `surface->LockRect(&locked, windowed ? &rect : NULL, D3DLOCK_READONLY 0x10)`;
/// `image = RwImageCreate(w, h, 32)`; with an image: `RwImageAllocatePixels` and BGRA → RGBA with
/// alpha 255, rows advanced by the locked pitch but written contiguously (4 w bytes per row, the
/// image stride is not used); unsigned row/column loops. The surface is never unlocked or released
/// (original leak, kept). Returns the image.
#[hook(addr = 0x7452B0, conv = cdecl, name = "Ps::psGrabScreen", expect = "83 EC 2C 55 56")]
pub fn ps_grab_screen(_camera: GPtr<u8>) -> GPtr<u8> {
    let dev = GPtr::<u32>::from_addr(call::rw_d3d9_get_current_d3d_device() as usize);
    let mut mode = [0u32; 4];
    call::d3d_dev_get_display_mode(dev, 0, GPtr::from_raw(&raw mut mode));
    let mut surface: u32 = 0;
    call::d3d_dev_create_offscreen_plain_surface(
        dev,
        mode[0],
        mode[1],
        0x15,
        2,
        GPtr::from_raw(&raw mut surface),
        0,
    );
    call::d3d_dev_get_front_buffer_data(dev, 0, surface);
    let (mut w, mut h) = (mode[0], mode[1]);
    let mut rect = [0i32; 4];
    let ps = RS_PS.get();
    let rect_arg = if field::<_, i32>(ps, 8).read() == 0 {
        call::get_window_rect_grab(field::<_, u32>(ps, 0).read(), GPtr::from_raw(&raw mut rect));
        h = rect[3].wrapping_sub(rect[1]) as u32;
        w = rect[2].wrapping_sub(rect[0]) as u32;
        GPtr::from_raw(&raw mut rect)
    } else {
        GPtr::NULL
    };
    let mut locked = [0u32; 2]; // Pitch, pBits
    let surf = GPtr::<u32>::from_addr(surface as usize);
    call::d3d_surface_lock_rect(surf, GPtr::from_raw(&raw mut locked), rect_arg, 0x10);
    let image = call::rw_image_create(w as i32, h as i32, 32);
    if image.is_null() {
        return image;
    }
    call::rw_image_allocate_pixels(image);
    let mut dst = GPtr::<u8>::from_addr(image.byte_add(0x14).cast::<u32>().read() as usize);
    let mut row: u32 = 0;
    while row < h {
        let mut src = GPtr::<u8>::from_addr(locked[1] as usize);
        let mut col: u32 = 0;
        while col < w {
            dst.write(src.byte_add(2).read());
            dst.byte_add(1).write(src.byte_add(1).read());
            dst.byte_add(2).write(src.read());
            dst.byte_add(3).write(0xFF);
            dst = dst.byte_add(4);
            src = src.byte_add(4);
            col += 1;
        }
        locked[1] = locked[1].wrapping_add(locked[0]);
        row += 1;
    }
    image
}

/// `Win::Win32_InitApplication(HINSTANCE)` (0x7486A0): the instance arrives in **ESI** (not
/// hookable). `RegisterClassA({CS_ 0x2000, __MainWndProc 0x747EB0, 0, 0, instance,
/// LoadIconA(instance, 100), LoadCursorA(NULL, IDC_ARROW), NULL, NULL, AppClassName})`, the ATOM
/// zero-extended.
pub fn win32_init_application(instance: u32) -> u32 {
    let icon = call::load_icon_a(instance, 100);
    let cursor = call::load_cursor_a(0, 0x7F00);
    let mut wc: [u32; 10] = [
        0x2000,
        0x747EB0,
        0,
        0,
        instance,
        icon,
        cursor,
        0,
        0,
        APP_CLASS_NAME.get(),
    ];
    call::register_class_a(GPtr::from_raw(&raw mut wc)) & 0xFFFF
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_RS_KEY_FROM_SCAN_CODE,
    HOOK_RS_TIMER,
    HOOK_RS_WINDOW_SET_TEXT,
    HOOK_RS_PATH_GET_SEPARATOR,
    HOOK_RS_CAMERA_BEGIN_UPDATE,
    HOOK_RS_ALWAYS_ON_TOP,
    HOOK_RS_REGISTER_IMAGE_LOADER,
    HOOK_RS_SET_DEBUG,
    HOOK_RS_MOUSE_SET_VISIBILITY,
    HOOK_RS_MOUSE_SET_POS,
    HOOK_RS_SELECT_DEVICE,
    HOOK_RS_INPUT_DEVICE_ATTACH,
    HOOK_RS_KEYBOARD_EVENT_HANDLER,
    HOOK_RS_MOUSE_EVENT_HANDLER,
    HOOK_RS_PAD_EVENT_HANDLER,
    HOOK_RS_PATHNAME_DESTROY,
    HOOK_RS_PATHNAME_CREATE,
    HOOK_RS_RW_TERMINATE,
    HOOK_RS_TERMINATE,
    HOOK_RS_INITIALIZE,
    HOOK_RS_GRAB_SCREEN,
    HOOK_RS_ERROR_MESSAGE,
    HOOK_RS_WARNING_MESSAGE,
    HOOK_RS_EVENT_HANDLER,
    HOOK_MOTION_BLUR_CLOSE,
    HOOK_TXD_STORE_PLUGIN_ATTACH,
    HOOK_SECUROM_DISPLAY_INIT,
    HOOK_IS_FOREGROUND_APP,
    HOOK_PS_WINDOW_SET_TEXT,
    HOOK_PS_ERROR_MESSAGE,
    HOOK_PS_WARNING_MESSAGE,
    HOOK_PS_CAMERA_BEGIN_UPDATE,
    HOOK_PS_CAMERA_SHOW_RASTER,
    HOOK_PS_TIMER,
    HOOK_PS_MOUSE_SET_VISIBILITY,
    HOOK_PS_MOUSE_SET_POS,
    HOOK_PS_PATHNAME_CREATE,
    HOOK_PS_PATHNAME_DESTROY,
    HOOK_PS_GET_MEMORY_FUNCTIONS,
    HOOK_PS_INSTALL_FILE_SYSTEM,
    HOOK_PS_NATIVE_TEXTURE_SUPPORT,
    HOOK_PS_DEBUG_MESSAGE_HANDLER,
    HOOK_WIN32_INIT_INSTANCE,
    HOOK_GET_VIDEO_MEM_INFO,
    HOOK_CHECK_DX_VERSION,
    HOOK_CHECK_DIRECT_SOUND,
    HOOK_PS_ALWAYS_ON_TOP,
    HOOK_IS_ALREADY_RUNNING,
    HOOK_INITIALISE_LANGUAGE,
    HOOK_ENUM_PADS,
    HOOK_DO_RW_STUFF_END_OF_FRAME,
    HOOK_PLUGIN_ATTACH,
    HOOK_INIT_USER_DIRECTORIES,
    HOOK_SECUROM_STATE_DISPLAY,
    HOOK_PS_GRAB_SCREEN,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn key_tables_and_scan_codes() {
        assert_eq!(rs_key_from_scan_code(0x41, 0), b'A' as u32);
        assert_eq!(rs_key_from_scan_code(0x61, 1), b'A' as u32);
        assert_eq!(rs_key_from_scan_code(0x31, 1), b'!' as u32);
        // only the low byte of the scan code is used
        assert_eq!(rs_key_from_scan_code(0x161, 0), b'a' as u32);
    }

    #[test]
    fn language_tables() {
        assert_eq!(text_language_for(7), 2);
        assert_eq!(text_language_for(0xA), 4);
        assert_eq!(text_language_for(0xC), 1);
        assert_eq!(text_language_for(0x10), 3);
        assert_eq!(text_language_for(9), 0);
        assert_eq!(prefs_language_key(0xA, 0x40A), 0xA);
        assert_eq!(prefs_language_key(9, 0x409), 9);
        // sublanguage 3: (0 - 1) & 0xFFFFFF94 + 9
        assert_eq!(prefs_language_key(9, 3 << 10 | 9), 0xFFFF_FF9D);
    }

    #[test]
    fn vms_and_events() {
        let mut s = *b"-vms\0";
        assert!(is_vms(GPtr::from_raw(s.as_mut_ptr())));
        let mut s = *b"-vm\0\0";
        assert!(!is_vms(GPtr::from_raw(s.as_mut_ptr())));
        assert_eq!(rs_input_device_attach(3, 0x1234), 0);
        assert_eq!(rs_input_device_attach(1, 0x1234), 1);
        assert_eq!(RS_MOUSE.get().handler, 0x1234);
        assert_eq!(RS_MOUSE.get().used, 1);
        assert_eq!(rs_register_image_loader(), 1);
        assert_eq!(ps_path_get_separator(), b'\\');
    }

    #[test]
    fn protected_cells_resolve_to_the_known_globals() {
        let sum = |a: u32, b: u32| a.wrapping_add(b).wrapping_add(1);
        assert_eq!(sum(PROT_KBD_A.get(), PROT_KBD_B.get()), 0xC1705C);
        assert_eq!(sum(PROT_EOF_A.get(), PROT_EOF_B.get()), 0xC1703C);
        assert_eq!(RS_KEYBOARD.addr() + 4, 0xC1705C);
    }

    #[test]
    fn ftol_truncates() {
        assert_eq!(ftol_f32(1.9), 1);
        assert_eq!(ftol_f32(-1.9), -1);
        assert_eq!(ftol_f32(f32::NAN), 0);
    }
}
