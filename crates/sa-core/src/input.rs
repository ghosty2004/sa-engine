//! CORE ports: keyboard/pad/mouse input glue (`app_input`, `WinInput`, `WinPlatform`), the `CPad`
//! key-state queries, the Win32 message loop and the RenderWare default render states
//! (`App::DefinedState*`). See `docs/arch/core-port.md`.
//!
//! Ported from the 1.0 US bytes (gta-reversed is a reading aid; differences are listed per function).
//! Win32 imports are called through their IAT cells (`cell`/`site`), DirectInput 8 methods as COM
//! `vslot`s, RenderWare's `RwRenderStateSet` through `RwEngineInstance`'s function table. Switch
//! tables (`HandleKeyDown/Up`, `GTATranslateKey`) were extracted from the exe's jump tables.

use crate::game::MENU_ACTIVE;
use crate::mem::{GPtr, GameData};
use crate::platform::{RS_PS, RS_QUIT};
use crate::{globals, hook, orig};

/// Opaque COM object (`IDirectInput8A`, `IDirectInputDevice8A`): methods are vtable slots.
pub enum ComObj {}

/// `CMouseControllerState` (0x14 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CMouseControllerState {
    pub lmb: u8,
    pub rmb: u8,
    pub mmb: u8,
    pub wheel_up: u8,
    pub wheel_down: u8,
    pub bmx1: u8,
    pub bmx2: u8,
    /// never written by `GetMouseSetUp` (uninitialised stack in the original)
    pub pad: u8,
    pub z: f32,
    pub x: f32,
    pub y: f32,
}
// SAFETY: integers and floats.
#[allow(unsafe_code)]
unsafe impl GameData for CMouseControllerState {}
const _: () =
    assert!(core::mem::size_of::<CMouseControllerState>() == crate::layouts::size::CMouseControllerState);

/// `CKeyboardState` as 0x138 `int16` words (FKeys at +0, standardKeys at +0x18, named keys from +0x218).
pub type KeyboardState = [i16; 0x138];

globals! {
    list GLOBALS;
    /// `CPad::TempKeyState`
    pub static TEMP_KEY_STATE @ 0xB72CB0: KeyboardState = [0; 0x138];
    /// `CPad::OldKeyState`
    pub static OLD_KEY_STATE @ 0xB72F20: KeyboardState = [0; 0x138];
    /// `CPad::NewKeyState`
    pub static NEW_KEY_STATE @ 0xB73190: KeyboardState = [0; 0x138];
    /// `CPad::padNumber` (tested as a byte)
    pub static PAD_NUMBER @ 0xB73400: u8 = 0;
    /// `ControlsManager.m_ButtonStates[16]` (0xB70198 + 0xB5C)
    pub static JOY_BUTTON_STATES @ 0xB70CF4: [u8; 16] = [0; 16];
    /// `TheCamera.m_nActiveCam`
    pub static CAMERA_ACTIVE_CAM @ 0xB6F081: u8 = 0;
    /// `s_OSStatus.OSVer` (1 = Windows 98)
    pub static OS_VER @ 0xC8CF68: i32 = 0;
    /// `AllValidWinJoys` (two 0x10-byte entries)
    pub static ALL_VALID_WIN_JOYS @ 0xC92144: [u8; 0x20] = [0; 0x20];
    /// `EnumDevicesCallback`'s static joystick counter
    pub static ENUM_JOY_COUNT @ 0xC92468: i32 = 0;
    /// `CTimeCycle::m_CurrentColours` fog/sky colour words read by `DefinedState`
    pub static TC_DEFINED_STATE_RGB @ 0xB7C4CA: [u16; 3] = [0; 3];
}

/// `ControlsManager` (`CControllerConfigManager`, 0xB70198).
pub const CONTROLS_MANAGER: u32 = 0xB70198;
/// `TheCamera.m_aCams[0].m_nMode` (stride 0x238).
pub const CAMERA_CAM0_MODE: u32 = 0xB6F1A8;

orig! {
    /// `CPad::GetPad(n)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `RsKeyboardEventHandler(event, param)` (0x619560)
    pub fn rs_keyboard_event_handler(event: i32, param: GPtr<i32>) -> i32 = cdecl 0x619560 => unported;
    /// `RsInputDeviceAttach(device, handler)` (0x6194C0)
    pub fn rs_input_device_attach(device: i32, handler: u32) -> i32 = cdecl 0x6194C0 => unported;
    /// `Input::HandleKeyDown` (0x743DF0)
    pub fn call_handle_key_down(ks: GPtr<i32>) -> i32 = cdecl 0x743DF0 => handle_key_down;
    /// `Input::HandleKeyUp` (0x7443C0)
    pub fn call_handle_key_up(ks: GPtr<i32>) -> i32 = cdecl 0x7443C0 => handle_key_up;
    /// `CControllerConfigManager::UpdateJoyButtonState(pad)` (0x52F510)
    pub fn ccm_update_joy_button_state(this: GPtr<u8>, pad: i32) = thiscall 0x52F510 => unported;
    /// `CControllerConfigManager::UpdateJoy_ButtonDown(button, type)` (0x530ED0)
    pub fn ccm_update_joy_button_down(this: GPtr<u8>, button: i32, kind: i32) = thiscall 0x530ED0 => unported;
    /// `CControllerConfigManager::UpdateJoyInConfigMenus_ButtonDown(button, pad)` (0x52DAB0)
    pub fn ccm_update_joy_in_config_menus_button_down(this: GPtr<u8>, button: i32, pad: i32) = thiscall 0x52DAB0 => unported;
    /// `CControllerConfigManager::AffectControllerStateOn_ButtonUp(button, type)` (0x531070)
    pub fn ccm_affect_controller_state_on_button_up(this: GPtr<u8>, button: i32, kind: i32) = thiscall 0x531070 => unported;
    /// `CControllerConfigManager::UpdateJoyInConfigMenus_ButtonUp(button, pad)` (0x52DC20)
    pub fn ccm_update_joy_in_config_menus_button_up(this: GPtr<u8>, button: i32, pad: i32) = thiscall 0x52DC20 => unported;
    /// `CControllerConfigManager::InitDefaultControlConfigJoyPad(buttons)` (0x530B00)
    pub fn ccm_init_default_control_config_joy_pad(this: GPtr<u8>, buttons: u32) = thiscall 0x530B00 => unported;
    /// `VideoMode::IsVideoModeExclusive()` (0x745CA0, AL)
    pub fn is_video_mode_exclusive() -> u8 = cdecl 0x745CA0 => unported;
    /// `Input::diMouseInit(exclusive)` (0x7469A0)
    pub fn call_di_mouse_init(exclusive: u32) -> i32 = cdecl 0x7469A0 => di_mouse_init;
    /// `FUN_007477C0`: `EnumDevices(GAMECTRL, EnumDevicesCallback)` + `diPadSetRanges` for both pads
    pub fn di_enum_pads() -> i32 = cdecl 0x7477C0 => unported;
    /// `RwRenderStateSet(state, value)` (`RwEngineInstance->dOpenDevice.fpRenderStateSet`, +0x20)
    pub fn rw_render_state_set(state: i32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `GetKeyState` (USER32, through its IAT cell)
    pub fn get_key_state(vk: i32) -> i32 = stdcall cell 0x8582B0 => unported;
    /// `MapVirtualKeyA` as `GTATranslateKey` calls it (`call [cell]` at 0x747B46)
    pub fn map_virtual_key_a(code: u32, kind: u32) -> u32 = stdcall site 0x747B46 => unported;
    /// `PeekMessageA` (USER32, IAT cell)
    pub fn peek_message_a(msg: GPtr<u8>, hwnd: u32, min: u32, max: u32, remove: u32) -> i32 = stdcall cell 0x8582BC => unported;
    /// `DispatchMessageA` (USER32, IAT cell)
    pub fn dispatch_message_a(msg: GPtr<u8>) -> i32 = stdcall cell 0x8582C0 => unported;
    /// `TranslateMessage` (USER32, IAT cell)
    pub fn translate_message(msg: GPtr<u8>) -> i32 = stdcall cell 0x8582C4 => unported;
    /// `IUnknown::Release`
    pub fn com_release(this: GPtr<ComObj>) -> u32 = stdcall vslot 0x08 => unported;
    /// `IDirectInput8A::CreateDevice(guid, out, outer)`
    pub fn di_create_device(this: GPtr<ComObj>, guid: GPtr<u8>, out: GPtr<GPtr<ComObj>>, outer: u32) -> i32 = stdcall vslot 0x0C => unported;
    /// `IDirectInput8A::EnumDevices(class, callback, ref, flags)`
    pub fn di_enum_devices(this: GPtr<ComObj>, class: u32, cb: u32, rf: u32, flags: u32) -> i32 = stdcall vslot 0x10 => unported;
    /// `IDirectInputDevice8A::GetCapabilities(caps)`
    pub fn did_get_capabilities(this: GPtr<ComObj>, caps: GPtr<u8>) -> i32 = stdcall vslot 0x0C => unported;
    /// `IDirectInputDevice8A::GetProperty(prop, header)`
    pub fn did_get_property(this: GPtr<ComObj>, prop: u32, header: GPtr<u8>) -> i32 = stdcall vslot 0x14 => unported;
    /// `IDirectInputDevice8A::SetProperty(prop, header)`
    pub fn did_set_property(this: GPtr<ComObj>, prop: u32, header: GPtr<u8>) -> i32 = stdcall vslot 0x18 => unported;
    /// `IDirectInputDevice8A::Acquire()`
    pub fn did_acquire(this: GPtr<ComObj>) -> i32 = stdcall vslot 0x1C => unported;
    /// `IDirectInputDevice8A::SetDataFormat(format)`
    pub fn did_set_data_format(this: GPtr<ComObj>, format: GPtr<u8>) -> i32 = stdcall vslot 0x2C => unported;
    /// `IDirectInputDevice8A::SetCooperativeLevel(hwnd, flags)`
    pub fn did_set_cooperative_level(this: GPtr<ComObj>, hwnd: u32, flags: u32) -> i32 = stdcall vslot 0x34 => unported;
    /// `IDirectInputDevice8A::GetObjectInfo(info, obj, how)`
    pub fn did_get_object_info(this: GPtr<ComObj>, info: GPtr<u8>, obj: u32, how: u32) -> i32 = stdcall vslot 0x38 => unported;
}

fn ps() -> GPtr<u8> {
    RS_PS.get().cast()
}

/// `RsGlobal.ps->field` at byte offset `off` (a pointer-sized field).
fn ps_field(off: isize) -> GPtr<GPtr<ComObj>> {
    ps().byte_add(off).cast()
}

fn ccm() -> GPtr<u8> {
    GPtr::from_addr(CONTROLS_MANAGER as usize)
}

fn key_word(state: &crate::mem::Global<KeyboardState>, byte_off: i32) -> GPtr<i16> {
    state.ptr().cast::<u8>().byte_add(byte_off as isize).cast()
}

// --------------------------------------------------------------------------------- CPad ---

fn just_pressed(byte_off: i32) -> bool {
    key_word(&NEW_KEY_STATE, byte_off).read() != 0 && key_word(&OLD_KEY_STATE, byte_off).read() == 0
}

/// `CPad::IsEnterJustPressed()` (0x4D5980, AL): `enter` (+0x23C) or `extenter` (+0x25A).
#[hook(addr = 0x4D5980, conv = cdecl, name = "CPad::IsEnterJustPressed", expect = "66 83 3D CC 33 B7 00 00")]
pub fn is_enter_just_pressed() -> bool {
    just_pressed(0x23C) || just_pressed(0x25A)
}

/// `CPad::IsStandardKeyJustPressed(key)` (0x4D59B0, thiscall, `ret 4`, EAX = 0/1; `this` unused).
#[hook(addr = 0x4D59B0, conv = thiscall, name = "CPad::IsStandardKeyJustPressed", expect = "0F B6 44 24 04 D1 E0", this = "CPad")]
pub fn is_standard_key_just_pressed(_pad: GPtr<u8>, key: u32) -> u32 {
    just_pressed(0x18 + 2 * (key & 0xFF) as i32) as u32
}

/// `CPad::IsMenuKeyJustPressed()` (0x744D50, EAX = 0/1): `lalt` (+0x266) **or `ralt`** (+0x268);
/// gta-reversed tests `lalt` only.
#[hook(addr = 0x744D50, conv = cdecl, name = "CPad::IsMenuKeyJustPressed", expect = "66 83 3D F6 33 B7 00 00")]
pub fn is_menu_key_just_pressed() -> u32 {
    (just_pressed(0x266) || just_pressed(0x268)) as u32
}

/// `CPad::IsTabJustPressed()` (0x744D90, EAX = 0/1): `tab` (+0x256).
#[hook(addr = 0x744D90, conv = cdecl, name = "CPad::IsTabJustPressed", expect = "66 83 3D E6 33 B7 00 00")]
pub fn is_tab_just_pressed() -> u32 {
    just_pressed(0x256) as u32
}

// ------------------------------------------------------------------------ keyboard events ---

/// `rsNULL`
pub const RS_NULL: i32 = 0x420;

/// `TempKeyState` byte offset per RsKeyCode 0x3E8..=0x41F (the shared jump table of 0x743DF0 /
/// 0x7443C0 at 0x7442E0 / 0x744798; F1..F12 = 0x3E9..0x3F4 are `FKeys[code - rsF1]`).
pub const KEY_FIELD: [u16; 0x38] = [
    0x218, 0x000, 0x002, 0x004, 0x006, 0x008, 0x00A, 0x00C, 0x00E, 0x010, 0x012, 0x014, 0x016, 0x21A, 0x21C,
    0x21E, 0x220, 0x222, 0x224, 0x226, 0x228, 0x22A, 0x22C, 0x234, 0x236, 0x23A, 0x238, 0x23E, 0x240, 0x242,
    0x244, 0x246, 0x248, 0x232, 0x24A, 0x24C, 0x24E, 0x250, 0x252, 0x23C, 0x22E, 0x230, 0x254, 0x256, 0x258,
    0x25A, 0x25C, 0x25E, 0x260, 0x262, 0x264, 0x266, 0x268, 0x26A, 0x26C, 0x26E,
];

/// Second pad's `PCTempKeyState` word (offset in `CPad`) and its key-down value, per key code.
pub fn pad_key(code: i32) -> Option<(i32, i16)> {
    Some(match code {
        0x44 => (0x78, 0x80),  // D
        0x41 => (0x78, -0x80), // A
        0x57 => (0x7A, 0x80),  // W
        0x53 => (0x7A, -0x80), // S
        0x4A => (0x7C, 0x80),  // J
        0x47 => (0x7C, -0x80), // G
        0x59 => (0x7E, 0x80),  // Y
        0x48 => (0x7E, -0x80), // H
        0x5A => (0x80, 0xFF),  // Z
        0x58 => (0x82, 0xFF),  // X
        0x43 => (0x84, 0xFF),  // C
        0x56 => (0x86, 0xFF),  // V
        0x4F => (0x88, 0xFF),  // O
        0x4C => (0x8A, 0xFF),  // L
        0x4B => (0x8C, 0xFF),  // K
        0x3B => (0x8E, 0xFF),  // ;
        0x42 => (0x90, 0xFF),  // B
        0x4E => (0x92, 0xFF),  // N
        0x4D => (0x94, 0xFF),  // M
        0x2C => (0x96, 0xFF),  // ,
        0x2E => (0x98, 0xFF),  // .
        0x2F => (0x9A, 0xFF),  // /
        0x417 => (0x9C, 0xFF), // rsRSHIFT
        0x41A => (0x9E, 0xFF), // rsRCTRL
        _ => return None,
    })
}

/// `TempKeyState` byte offset written for `code`, if any (`code < 255` signed for plain keys: a
/// negative code writes before `standardKeys`, as in the original).
pub fn key_field(code: i32) -> Option<i32> {
    let k = code.wrapping_sub(0x3E8) as u32;
    if k <= 0x37 {
        Some(KEY_FIELD[k as usize] as i32)
    } else if code < 0xFF {
        Some(0x18i32.wrapping_add(code.wrapping_mul(2)))
    } else {
        None
    }
}

fn handle_key(ks: GPtr<i32>, down: bool) -> i32 {
    let _ = get_pad(0);
    let pad1 = get_pad(1);
    let code = ks.read();
    if code == RS_NULL {
        return 1;
    }
    if let Some(off) = key_field(code) {
        key_word(&TEMP_KEY_STATE, off).write(if down { 0xFF } else { 0 });
    }
    if PAD_NUMBER.get() == 0 {
        return 1;
    }
    if let Some((off, v)) = pad_key(code) {
        pad1.byte_add(off as isize)
            .cast::<i16>()
            .write(if down { v } else { 0 });
    }
    1
}

/// `HandleKeyDown(RsKeyStatus*)` (0x743DF0). `CPad::GetPad(0)` is called and its result unused.
#[hook(addr = 0x743DF0, conv = cdecl, name = "Input::HandleKeyDown", expect = "6A 00 E8 79 BD DF FF")]
pub fn handle_key_down(ks: GPtr<i32>) -> i32 {
    handle_key(ks, true)
}

/// `HandleKeyUp(RsKeyStatus*)` (0x7443C0). The `padNumber` test sits in a chunk at 0x407F85.
#[hook(addr = 0x7443C0, conv = cdecl, name = "Input::HandleKeyUp", expect = "53 33 DB 53 E8 A7 B7 DF FF")]
pub fn handle_key_up(ks: GPtr<i32>) -> i32 {
    handle_key(ks, false)
}

/// `KeyboardHandler(event, param)` (0x744880): rsKEYDOWN (0x1C) / rsKEYUP (0x1D), else 2.
#[hook(addr = 0x744880, conv = cdecl, name = "Input::KeyboardHandler", expect = "8B 44 24 04 83 E8 1C")]
pub fn keyboard_handler(event: i32, param: GPtr<i32>) -> i32 {
    match event {
        0x1C => call_handle_key_down(param),
        0x1D => call_handle_key_up(param),
        _ => 2,
    }
}

/// `HandlePadButtonDown(RsPadButtonStatus*)` (0x7448B0): the argument is passed in **EAX** (no
/// supported calling convention), so this is not hooked; `PadHandler`'s port calls it directly.
/// Original: the pad number is 1 when `CPad::padNumber != 0`, else **0** (gta-reversed keeps
/// `padID`); every button 1..16 is reported, with button 0 for buttons that are not down
/// (gta-reversed skips them).
pub fn handle_pad_button_down(status: GPtr<i32>) -> i32 {
    let _ = get_pad(status.read());
    let two = PAD_NUMBER.get() != 0;
    let pad = two as i32;
    ccm_update_joy_button_state(ccm(), pad);
    let states = JOY_BUTTON_STATES.ptr().cast::<u8>();
    for i in 1..0x11 {
        let btn = if states.wrapping_index(i - 1).read() == 1 {
            i
        } else {
            0
        };
        if MENU_ACTIVE.get().get() || two {
            ccm_update_joy_in_config_menus_button_down(ccm(), btn, pad);
        } else {
            ccm_update_joy_button_down(ccm(), btn, 3);
        }
    }
    1
}

/// `HandlePadButtonUp(RsPadButtonStatus*)` (0x744930): argument in **EAX** (not hooked). Buttons
/// 2..16; button 0 is reported for buttons still down. Camera modes 0x11/0xF of the active camera
/// route to the config-menu handler.
pub fn handle_pad_button_up(status: GPtr<i32>) -> i32 {
    let _ = get_pad(status.read());
    let two = PAD_NUMBER.get() != 0;
    let pad = two as i32;
    let active = CAMERA_ACTIVE_CAM.get() as usize;
    let mode = GPtr::<i16>::from_addr(CAMERA_CAM0_MODE as usize + active * 0x238).read();
    let cam = mode == 0x11 || mode == 0xF;
    ccm_update_joy_button_state(ccm(), pad);
    let states = JOY_BUTTON_STATES.ptr().cast::<u8>();
    for i in 2..0x11 {
        let btn = if states.wrapping_index(i - 1).read() == 0 {
            i
        } else {
            0
        };
        if MENU_ACTIVE.get().get() || two || cam {
            ccm_update_joy_in_config_menus_button_up(ccm(), btn, pad);
        } else {
            ccm_affect_controller_state_on_button_up(ccm(), btn, 3);
        }
    }
    1
}

/// `PadHandler(event, param)` (0x7449F0): rsPADBUTTONDOWN (0x1F) / rsPADBUTTONUP (0x20), else 2. Its
/// callees take EAX and have no hooks, so the Rust versions are called directly.
#[hook(addr = 0x7449F0, conv = cdecl, name = "Input::PadHandler", expect = "8B 44 24 04 83 E8 1F")]
pub fn pad_handler(event: i32, param: GPtr<i32>) -> i32 {
    match event {
        0x1F => handle_pad_button_down(param),
        0x20 => handle_pad_button_up(param),
        _ => 2,
    }
}

/// `AttachInputDevices()` (0x744A20, EAX = 1).
#[hook(addr = 0x744A20, conv = cdecl, name = "Input::AttachInputDevices", expect = "68 80 48 74 00")]
pub fn attach_input_devices() -> u32 {
    let _ = rs_input_device_attach(0, 0x744880);
    let _ = rs_input_device_attach(2, 0x7449F0);
    1
}

// --------------------------------------------------------------------------- DirectInput ---

/// `GUID_SysMouse` in `.rdata`.
pub const GUID_SYS_MOUSE: u32 = 0x858754;
/// `c_dfDIMouse2`.
pub const DF_DI_MOUSE2: u32 = 0x88584C;
/// `c_dfDIJoystick2`.
pub const DF_DI_JOYSTICK2: u32 = 0x885A54;

/// `diMouseInit(exclusive)` (0x7469A0): returns the first failing HRESULT, else 0 (also when
/// `Acquire` fails). Cooperative level 5 (foreground|exclusive) or 6 (foreground|non-exclusive).
#[hook(addr = 0x7469A0, conv = cdecl, name = "Input::diMouseInit", expect = "8A 44 24 04 8B 0D 54 70 C1 00")]
pub fn di_mouse_init(exclusive: u32) -> i32 {
    let flags = if exclusive as u8 != 0 { 5 } else { 6 };
    let di = ps_field(0x18).read();
    let r = di_create_device(di, GPtr::from_addr(GUID_SYS_MOUSE as usize), ps_field(0x1C), 0);
    if r < 0 {
        return r;
    }
    let r = did_set_data_format(ps_field(0x1C).read(), GPtr::from_addr(DF_DI_MOUSE2 as usize));
    if r < 0 {
        return r;
    }
    let hwnd = ps().cast::<u32>().read();
    let r = did_set_cooperative_level(ps_field(0x1C).read(), hwnd, flags);
    if r < 0 {
        return r;
    }
    let _ = did_acquire(ps_field(0x1C).read());
    0
}

/// `FUN_00747000` (called from `__MainWndProc`): release `ps->diInterface` and clear it.
#[hook(addr = 0x747000, conv = cdecl, name = "FUN_00747000", expect = "A1 54 70 C1 00")]
pub fn release_di_interface() {
    let di = ps_field(0x18).read();
    if !di.is_null() {
        let _ = com_release(di);
        ps_field(0x18).write(GPtr::NULL);
    }
}

/// `GetMouseSetUp()` (0x53F2D0): returns a `CMouseControllerState` through a hidden pointer (callee
/// pops it: `ret 4`). Initialises the mouse (exclusive unless the menu is active or the video mode
/// is not exclusive) when there is none, then reads its capabilities: buttons 1 -> lmb, 2 -> +rmb,
/// 3..8 -> +mmb; 3 axes -> wheel up/down. Original UB: the struct's padding byte and the
/// `DIDEVCAPS` fields when `GetCapabilities` fails are uninitialised stack (port: 0).
#[hook(addr = 0x53F2D0, conv = stdcall, name = "Input::GetMouseSetUp", expect = "83 EC 44 A1 54 70 C1 00")]
pub fn get_mouse_set_up(out: GPtr<CMouseControllerState>) -> GPtr<CMouseControllerState> {
    let mut s = CMouseControllerState::default();
    if ps_field(0x1C).read().is_null() {
        let mut exclusive = 0u32;
        if !MENU_ACTIVE.get().get() && is_video_mode_exclusive() != 0 {
            exclusive = 1;
        }
        let _ = call_di_mouse_init(exclusive);
    }
    let mouse = ps_field(0x1C).read();
    if !mouse.is_null() {
        let mut caps = [0u32; 11];
        caps[0] = 0x2C;
        let _ = did_get_capabilities(mouse, GPtr::from_raw(caps.as_mut_ptr()).cast());
        match caps[4].wrapping_sub(1) {
            0 => s.lmb = 1,
            1 => {
                s.rmb = 1;
                s.lmb = 1;
            }
            2..=7 => {
                s.mmb = 1;
                s.rmb = 1;
                s.lmb = 1;
            }
            _ => {}
        }
        if caps[3] == 3 {
            s.wheel_up = 1;
            s.wheel_down = 1;
        }
    }
    out.write(s);
    out
}

/// `DIPROPRANGE` as `diPadSetRanges` builds it: size 0x18, header 0x10, `dwObj`, how = 1 (by
/// offset), min -2000, max 2000.
#[repr(C)]
#[derive(Clone, Copy)]
struct DiPropRange([u32; 6]);

/// `diPadSetRanges(dev, padNum)` (0x746D80): for axes X (0), Y (4), Z (8), RZ (0x14): if
/// `GetObjectInfo` succeeds, `SetProperty(DIPROP_RANGE)`; a failing `SetProperty` returns 1. Z/RZ set
/// `AllValidWinJoys[pad].bZAxisPresent`/`bZRotPresent` only after a successful `SetProperty`
/// (gta-reversed sets them before, and passes DIJOFS_Z for the RZ axis too).
#[hook(addr = 0x746D80, conv = cdecl, name = "Input::diPadSetRanges", expect = "81 EC 54 01 00 00")]
pub fn di_pad_set_ranges(dev: GPtr<ComObj>, pad: u32) -> i32 {
    let mut range = DiPropRange([0x18, 0x10, 0, 1, 0xFFFF_F830, 0x7D0]);
    let mut info = [0u32; 0x13C / 4];
    info[0] = 0x13C;
    let info_p = GPtr::from_raw(info.as_mut_ptr()).cast::<u8>();
    let range_p = GPtr::from_raw(range.0.as_mut_ptr()).cast::<u8>();
    for (obj, flag) in [(0u32, None), (4, None), (8, Some(1)), (0x14, Some(2))] {
        range.0[2] = obj;
        if dev.is_null() || did_get_object_info(dev, info_p, obj, 1) < 0 {
            continue;
        }
        if did_set_property(dev, 4, range_p) < 0 {
            return 1;
        }
        if let Some(f) = flag {
            ALL_VALID_WIN_JOYS
                .ptr()
                .cast::<u8>()
                .byte_add((pad.wrapping_shl(4)) as i32 as isize + 4 + f)
                .write(1);
        }
    }
    0
}

/// `EnumDevicesCallback(lpddi, pvRef)` (0x747020, stdcall): creates joystick 0/1 (`ps+0x20`/`+0x24`;
/// a third device gets a null out-pointer), joystick data format, cooperative level 6; a failure
/// releases the device and continues. Returns `count != 2` after a success (DIENUM_STOP at 2).
#[hook(addr = 0x747020, conv = stdcall, name = "Input::EnumDevicesCallback", expect = "8B 0D 68 24 C9 00")]
pub fn enum_devices_callback(lpddi: GPtr<u8>, _pv_ref: u32) -> i32 {
    let slot: GPtr<GPtr<ComObj>> = match ENUM_JOY_COUNT.get() {
        0 => ps_field(0x20),
        1 => ps_field(0x24),
        _ => GPtr::NULL,
    };
    let di = ps_field(0x18).read();
    if di_create_device(di, lpddi.byte_add(4), slot, 0) != 0 {
        return 1;
    }
    if did_set_data_format(slot.read(), GPtr::from_addr(DF_DI_JOYSTICK2 as usize)) != 0 {
        let _ = com_release(slot.read());
        return 1;
    }
    ENUM_JOY_COUNT.set(ENUM_JOY_COUNT.get().wrapping_add(1));
    let hwnd = ps().cast::<u32>().read();
    if did_set_cooperative_level(slot.read(), hwnd, 6) != 0 {
        let _ = com_release(slot.read());
        return 1;
    }
    (ENUM_JOY_COUNT.get() != 2) as i32
}

/// `diPadInit()` (0x7485C0): clears both `AllValidWinJoys` entries, enumerates the pads
/// (`FUN_007477C0`), then reads `DIPROP_VIDPID` per pad (pad 0 also gets `GetCapabilities` and
/// `InitDefaultControlConfigJoyPad(32)`). Original UB: pad 1 reuses pad 0's `DIPROPDWORD`, whose
/// header is only written on the pad-0 path, and the `DIDEVCAPS` beyond `dwSize` (port: 0).
#[hook(addr = 0x7485C0, conv = cdecl, name = "Input::diPadInit", expect = "83 EC 40 53 B8 48 21 C9 00")]
pub fn di_pad_init() {
    let joys = ALL_VALID_WIN_JOYS.ptr().cast::<u8>();
    for e in 0..2 {
        let p = joys.byte_add(e * 0x10);
        p.cast::<u32>().write(0);
        p.byte_add(4).write(0);
        p.byte_add(5).write(0);
        p.byte_add(6).write(0);
    }
    let _ = di_enum_pads();
    let mut prop = [0u32; 5];
    let prop_p = GPtr::from_raw(prop.as_mut_ptr()).cast::<u8>();
    let j0 = ps_field(0x20).read();
    if !j0.is_null() {
        joys.byte_add(4).write(1);
        let mut caps = [0u32; 11];
        caps[0] = 0x2C;
        let _ = did_get_capabilities(j0, GPtr::from_raw(caps.as_mut_ptr()).cast());
        prop = [0x14, 0x10, 0, 0, prop[4]];
        let _ = did_get_property(ps_field(0x20).read(), 0x18, prop_p);
        joys.byte_add(8).cast::<u32>().write(prop[4] & 0xFFFF);
        joys.byte_add(0xC).cast::<u32>().write(prop[4] >> 16);
        ccm_init_default_control_config_joy_pad(ccm(), 0x20);
    }
    let j1 = ps_field(0x24).read();
    if !j1.is_null() {
        let _ = did_get_property(j1, 0x18, prop_p);
        joys.byte_add(0x18).cast::<u32>().write(prop[4] & 0xFFFF);
        joys.byte_add(0x1C).cast::<u32>().write(prop[4] >> 16);
        joys.byte_add(0x14).write(1);
    }
}

// ------------------------------------------------------------------------------- Win32 ---

/// `MessageLoop()` (0x746870): `PeekMessageA(PM_REMOVE|PM_NOYIELD)` until empty; WM_QUIT sets
/// `RsGlobal.quit = 1` (and the loop keeps peeking), anything else is translated and dispatched.
#[hook(addr = 0x746870, conv = cdecl, name = "Win::MessageLoop", expect = "83 EC 1C 56 8B 35 BC 82 85 00")]
pub fn message_loop() {
    let mut msg = [0u32; 7];
    let m = GPtr::from_raw(msg.as_mut_ptr()).cast::<u8>();
    while peek_message_a(m, 0, 0, 0, 3) != 0 {
        if msg[1] == 0x12 {
            RS_QUIT.set(1);
        } else {
            let _ = translate_message(m);
            let _ = dispatch_message_a(m);
        }
    }
}

/// `GTATranslateKey` cases (vk, code, code with KF_EXTENDED), extracted from the two-level jump
/// table at 0x747C38/0x747B68. `SHIFT` = VK_SHIFT (0x418 on Windows 98, else rsNULL and false).
const SHIFT: u16 = 0;
const TRANSLATE: [(u8, u16, u16); 51] = [
    (0x08, 0x412, 0x412),
    (0x09, 0x413, 0x413),
    (0x0D, 0x415, 0x40F),
    (0x10, SHIFT, SHIFT),
    (0x11, 0x419, 0x41A),
    (0x12, 0x41B, 0x41C),
    (0x13, 0x411, 0x411),
    (0x14, 0x414, 0x414),
    (0x1B, 0x3E8, 0x3E8),
    (0x21, 0x40D, 0x3F9),
    (0x22, 0x406, 0x3FA),
    (0x23, 0x404, 0x3F8),
    (0x24, 0x40B, 0x3F7),
    (0x25, 0x407, 0x3FD),
    (0x26, 0x40C, 0x3FB),
    (0x27, 0x40A, 0x3FE),
    (0x28, 0x405, 0x3FC),
    (0x2D, 0x40E, 0x3F5),
    (0x2E, 0x403, 0x3F6),
    (0x5B, 0x41D, 0x41D),
    (0x5C, 0x41E, 0x41E),
    (0x5D, 0x41F, 0x41F),
    (0x60, 0x40E, 0x40E),
    (0x61, 0x404, 0x404),
    (0x62, 0x405, 0x405),
    (0x63, 0x406, 0x406),
    (0x64, 0x407, 0x407),
    (0x65, 0x408, 0x408),
    (0x66, 0x40A, 0x40A),
    (0x67, 0x40B, 0x40B),
    (0x68, 0x40C, 0x40C),
    (0x69, 0x40D, 0x40D),
    (0x6A, 0x400, 0x400),
    (0x6B, 0x401, 0x401),
    (0x6D, 0x402, 0x402),
    (0x6E, 0x403, 0x403),
    (0x6F, 0x3FF, 0x3FF),
    (0x70, 0x3E9, 0x3E9),
    (0x71, 0x3EA, 0x3EA),
    (0x72, 0x3EB, 0x3EB),
    (0x73, 0x3EC, 0x3EC),
    (0x74, 0x3ED, 0x3ED),
    (0x75, 0x3EE, 0x3EE),
    (0x76, 0x3EF, 0x3EF),
    (0x77, 0x3F0, 0x3F0),
    (0x78, 0x3F1, 0x3F1),
    (0x79, 0x3F2, 0x3F2),
    (0x7A, 0x3F3, 0x3F3),
    (0x7B, 0x3F4, 0x3F4),
    (0x90, 0x409, 0x409),
    (0x91, 0x410, 0x410),
];

/// What `GTATranslateKey` decides for `vk` without calling Win32: `Some((code, ret))` for a table
/// case, `None` for the `MapVirtualKeyA` default.
pub fn translate_key_case(vk: u32, lparam: u32, os_ver: i32) -> Option<(i32, bool)> {
    if vk.wrapping_sub(8) > 0x89 {
        return None;
    }
    let (_, n, e) = TRANSLATE.iter().find(|t| t.0 as u32 == vk)?;
    if *n == SHIFT {
        return Some(if os_ver == 1 {
            (0x418, true)
        } else {
            (RS_NULL, false)
        });
    }
    Some((if lparam & 0x0100_0000 != 0 { *e } else { *n } as i32, true))
}

/// The default case: `chr = (uint16)MapVirtualKeyA(vk, MAPVK_VK_TO_CHAR)`, stored if `chr < 0xFF`
/// (gta-reversed: `<= 0xFF`); returns `code != rsNULL`.
pub fn translate_key_default(chr: u32) -> (i32, bool) {
    let c = chr & 0xFFFF;
    let code = if c < 0xFF { c as i32 } else { RS_NULL };
    (code, code != RS_NULL)
}

/// `GTATranslateKey(ck, lParam, vk)` (0x747820, AL).
#[hook(addr = 0x747820, conv = cdecl, name = "Win::GTATranslateKey", expect = "8B 4C 24 0C 56 8B 74 24 08")]
pub fn gta_translate_key(ck: GPtr<i32>, lparam: u32, vk: u32) -> bool {
    ck.write(RS_NULL);
    let (code, r) = match translate_key_case(vk, lparam, OS_VER.get()) {
        Some(x) => x,
        None => translate_key_default(map_virtual_key_a(vk, 2)),
    };
    ck.write(code);
    r
}

fn shift_event(ck: GPtr<i32>, vk: i32, code: i32, down: bool) {
    ck.write(RS_NULL);
    let pressed = get_key_state(vk) as u32 & 0x8000 != 0;
    if pressed == down {
        ck.write(code);
    }
    if ck.read() != RS_NULL {
        let _ = rs_keyboard_event_handler(if down { 0x1C } else { 0x1D }, ck);
    }
}

/// `GTATranslateShiftKey(ck)` (0x747CD0): nothing on Windows 98; else four `GetKeyState` probes
/// (L down, R down, L up, R up) reported through `*ck` and `RsKeyboardEventHandler`. EAX is not set
/// by the original (gta-reversed returns a BOOL).
#[hook(addr = 0x747CD0, conv = cdecl, name = "Win::GTATranslateShiftKey", expect = "83 3D 68 CF C8 00 01")]
pub fn gta_translate_shift_key(ck: GPtr<i32>) {
    if OS_VER.get() == 1 {
        return;
    }
    shift_event(ck, 0xA0, 0x416, true);
    shift_event(ck, 0xA1, 0x417, true);
    shift_event(ck, 0xA0, 0x416, false);
    shift_event(ck, 0xA1, 0x417, false);
}

// ------------------------------------------------------------------- RenderWare defaults ---

/// `DefinedState()`'s fog colour: `((((r | 0xFFFFFF00) << 8) | g) << 8) | b` on the zero-extended
/// words (an out-of-range g/b word spills into the higher bytes exactly as in the original).
pub fn defined_state_fog_colour(rgb: [u16; 3]) -> u32 {
    ((((rgb[0] as u32 | 0xFFFF_FF00) << 8) | rgb[1] as u32) << 8) | rgb[2] as u32
}

/// `App::DefinedState()` (0x734650): sixteen `RwRenderStateSet` calls.
#[hook(addr = 0x734650, conv = cdecl, name = "App::DefinedState", expect = "A1 24 7B C9 00 6A 01")]
pub fn defined_state() {
    for (s, v) in [
        (2, 1),
        (5, 1),
        (6, 1),
        (8, 1),
        (7, 2),
        (9, 2),
        (0xC, 0),
        (0xA, 5),
        (0xB, 6),
        (0xD, 0xFF00_0000),
        (0xE, 0),
    ] {
        let _ = rw_render_state_set(s, v);
    }
    let _ = rw_render_state_set(0xF, defined_state_fog_colour(TC_DEFINED_STATE_RGB.get()));
    for (s, v) in [(0x10, 1), (0x14, 1), (0x1D, 5), (0x1E, 2)] {
        let _ = rw_render_state_set(s, v);
    }
}

/// `App::DefinedState2d()` (0x734750): fourteen `RwRenderStateSet` calls.
#[hook(addr = 0x734750, conv = cdecl, name = "App::DefinedState2d", expect = "A1 24 7B C9 00 6A 01")]
pub fn defined_state_2d() {
    for (s, v) in [
        (2, 1),
        (5, 0),
        (6, 0),
        (8, 0),
        (7, 2),
        (9, 2),
        (0xC, 0),
        (0xA, 5),
        (0xB, 6),
        (0xD, 0xFF00_0000),
        (0xE, 0),
        (0x14, 1),
        (0x1D, 5),
        (0x1E, 2),
    ] {
        let _ = rw_render_state_set(s, v);
    }
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_IS_ENTER_JUST_PRESSED,
    HOOK_IS_STANDARD_KEY_JUST_PRESSED,
    HOOK_IS_MENU_KEY_JUST_PRESSED,
    HOOK_IS_TAB_JUST_PRESSED,
    HOOK_HANDLE_KEY_DOWN,
    HOOK_HANDLE_KEY_UP,
    HOOK_KEYBOARD_HANDLER,
    HOOK_PAD_HANDLER,
    HOOK_ATTACH_INPUT_DEVICES,
    HOOK_DI_MOUSE_INIT,
    HOOK_RELEASE_DI_INTERFACE,
    HOOK_GET_MOUSE_SET_UP,
    HOOK_DI_PAD_SET_RANGES,
    HOOK_ENUM_DEVICES_CALLBACK,
    HOOK_DI_PAD_INIT,
    HOOK_MESSAGE_LOOP,
    HOOK_GTA_TRANSLATE_KEY,
    HOOK_GTA_TRANSLATE_SHIFT_KEY,
    HOOK_DEFINED_STATE,
    HOOK_DEFINED_STATE_2D,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn key_fields() {
        assert_eq!(key_field(0x3E8), Some(0x218)); // esc
        assert_eq!(key_field(0x3E9), Some(0)); // F1
        assert_eq!(key_field(0x3F4), Some(0x16)); // F12
        assert_eq!(key_field(0x41F), Some(0x26E)); // apps
        assert_eq!(key_field(b'A' as i32), Some(0x18 + 0x82));
        assert_eq!(key_field(0xFF), None);
        assert_eq!(key_field(0x420), None);
        assert_eq!(key_field(-1), Some(0x16)); // before standardKeys, no bounds check
    }

    #[test]
    fn translate_cases() {
        assert_eq!(translate_key_case(0x0D, 0, 0), Some((0x415, true)));
        assert_eq!(translate_key_case(0x0D, 0x0100_0000, 0), Some((0x40F, true)));
        assert_eq!(translate_key_case(0x10, 0, 0), Some((RS_NULL, false)));
        assert_eq!(translate_key_case(0x10, 0, 1), Some((0x418, true)));
        assert_eq!(translate_key_case(0x41, 0, 0), None);
        assert_eq!(translate_key_case(7, 0, 0), None);
        assert_eq!(translate_key_case(0x92, 0, 0), None);
        assert_eq!(translate_key_default(0xFF), (RS_NULL, false));
        assert_eq!(translate_key_default(0x1_0041), (0x41, true));
        assert_eq!(translate_key_default(0), (0, true));
    }

    #[test]
    fn fog_colour_bits() {
        assert_eq!(defined_state_fog_colour([0x12, 0x34, 0x56]), 0xFF12_3456);
        assert_eq!(defined_state_fog_colour([0, 0x1FF, 0]), 0xFF01_FF00);
    }
}
