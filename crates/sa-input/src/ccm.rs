//! INPUT: `CControllerConfigManager` (`ControlsManager` = 0xB70198; PORT-ANIM-INPUT). Ported from the
//! 1.0 US bytes; gta-reversed `ControllerConfigManager.cpp` as a reading aid and
//! `docs/re/specs/INPUT/CControllerConfigManager.deviations.md` for the places where it differs (the
//! port follows the bytes). See `docs/arch/anim-input-port.md`.
//!
//! Layout (size 0x12E4): +0x000 `m_bJoyJustInitialised`, +0x004 `m_OldJoyState` (DIJOYSTATE2, buttons
//! at +0x34), +0x114 `m_NewJoyState` (buttons at +0x144), +0x224 `m_ControllerActionName[59][40]`,
//! +0xB5C `m_ButtonStates[17]`, +0xB70 `m_Actions[59]` (stride 0x20; `Keys[type]` stride 8: +0
//! initiator, +4 set order), +0x12D0/+0x12D4/+0x12D8/+0x12DC the "both down" flags of the four stick
//! axes (one byte per controller type), +0x12E0 `m_MouseFoundInitSet`. Controller types: 0 keyboard,
//! 1 optional extra key, 2 mouse, 3 joystick; `rsNULL` = 0x420.
//!
//! Calls to other catalogued functions (this class's own included, so each one's hook toggle
//! governs them) go through `orig!`. Globals of other subsystems (pad key/mouse states, the
//! front-end menu, the camera, the text buffers) are addressed directly (`g`).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::{GBool, GPtr};
use sa_core::orig;

use crate::common::{g, rd, wr};

/// `ControlsManager`.
pub const CONTROLS_MANAGER: usize = 0xB70198;
/// `rsNULL`.
pub const RS_NULL: u32 = 0x420;
/// `m_Actions[0].Keys[0]`.
pub const ACTIONS: i32 = 0xB70;
/// Number of actions.
pub const NUM_ACTIONS: i32 = 59;

/// `CPad::NewKeyState` / `OldKeyState` (`CKeyboardState`, int16 words; standard keys at +0x18).
const NEW_KEY_STATE: u32 = 0xB73190;
const OLD_KEY_STATE: u32 = 0xB72F20;
/// `CPad::TempMouseControllerState` / `NewMouseControllerState` / `OldMouseControllerState`.
const TEMP_MOUSE: u32 = 0xB73404;
const NEW_MOUSE: u32 = 0xB73418;
const OLD_MOUSE: u32 = 0xB7342C;
/// `CPad::padNumber` (byte).
const PAD_NUMBER: usize = 0xB73400;
/// `FrontEndMenuManager.m_bMenuActive`.
const MENU_ACTIVE: usize = 0xBA67A4;
/// `FrontEndMenuManager.m_ControlMethod` (byte; 0 mouse + keys, 1 joypad).
const CONTROL_METHOD: usize = 0xBA6818;
/// `FrontEndMenuManager.m_nTextLanguage`... the language word the key texts test (1 / 2).
const LANGUAGE: usize = 0xBA67C8;
/// `TheCamera.m_nActiveCam` (byte) and `TheCamera.m_aCams[0].m_nMode` (stride 0x238).
const CAMERA_ACTIVE_CAM: usize = 0xB6F081;
const CAMERA_CAM0_MODE: u32 = 0xB6F1A8;
/// `AllValidWinJoys.JoyStickNum[0]`: `bJoyAttachedToPort` (byte), `wVendorID`, `wProductID` (dwords).
const JOY0_ATTACHED: usize = 0xC92148;
const JOY0_VENDOR: usize = 0xC9214C;
const JOY0_PRODUCT: usize = 0xC92150;
/// `TheText`.
const THE_TEXT: usize = 0xC1B340;
/// Static text buffers: `GetControllerSettingTextJoystick` / `GetGxtStringOfCommandKeys` (0x40 bytes),
/// the function-key / numpad text (0x20), the one-character key text (0x32).
const JOY_TEXT_BUF: usize = 0xB7147C;
const KEY_TEXT_BUF: usize = 0xB7149C;
const CHAR_TEXT_BUF: u32 = 0xB714BC;
/// `RsGlobal.ps` (`PsGlobalType*`; +0x20 `diDevice1`).
const RS_PS: usize = 0xC17054;
/// The two dwords `InitDefaultControlConfiguration`'s no-CD chunk (0x409ABF) adds with carry set to make
/// the GO_FORWARD key: a `.HOODLUM` data word and the code bytes at 0x5DF174 (statically 0x3FB, `rsUP`).
const PROT_KEY_A: usize = 0x15608DC;
const PROT_KEY_B: usize = 0x5DF174;

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => crate::pad::get_pad;
    /// `FindPlayerVehicle(int32 playerId, bool bIncludeRemote)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `FindPlayerPed(int32 playerId)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CText::Get(const char* key)` (0x6A0050)
    pub fn text_get(text: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CMessages::InsertNumberInString(text, n1, .., n6, out)` (0x69DE90)
    pub fn insert_number_in_string(text: GPtr<u8>, n1: i32, n2: i32, n3: i32, n4: i32, n5: i32, n6: i32, out: GPtr<u8>) = cdecl 0x69DE90 => unported;
    /// `CMessages::StringCopy(dst, src, len)` (0x69DB70)
    pub fn string_copy(dst: GPtr<u8>, src: GPtr<u8>, len: i32) = cdecl 0x69DB70 => unported;
    /// `CMessages::GetStringLength(s)` (0x69DB50)
    pub fn get_string_length(s: GPtr<u8>) -> i32 = cdecl 0x69DB50 => unported;
    /// `Common::AsciiToGxtChar(src, dst)` (0x718600)
    pub fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;
    /// unnamed 0x719360: the font character of a key (`c >= 0x80` through the table at 0x8D5AD8)
    pub fn key_char(c: u32) -> u8 = cdecl 0x719360 => unported;
    /// `CFileMgr::Read(file, buf, size)` (0x538950)
    pub fn file_read(file: i32, buf: GPtr<u8>, size: i32) -> i32 = cdecl 0x538950 => unported;
    /// `CFileMgr::Write(file, buf, size)` (0x538970)
    pub fn file_write(file: i32, buf: GPtr<u8>, size: i32) -> i32 = cdecl 0x538970 => unported;
    /// `CFileMgr::Seek(file, offset, origin)` (0x538990, AL)
    pub fn file_seek(file: i32, offset: i32, origin: i32) -> u8 = cdecl 0x538990 => unported;
    /// CRT `strncmp` (0x8214D0)
    pub fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// `GTATranslateShiftKey(RsKeyCodes*)` (0x747CD0; the result is not used)
    pub fn gta_translate_shift_key(ck: GPtr<i32>) = cdecl 0x747CD0 => unported;
    /// `Input::GetMouseSetUp()` (0x53F2D0, hidden result pointer, `ret 4`)
    pub fn get_mouse_set_up(out: GPtr<u8>) -> GPtr<u8> = stdcall 0x53F2D0 => unported;
    /// `IDirectInputDevice8A::GetCapabilities(caps)`
    pub fn did_get_capabilities(dev: GPtr<u8>, caps: GPtr<u8>) -> i32 = stdcall vslot 0x0C => unported;

    // this class (hooked here: each call obeys the callee's toggle)
    /// `ResetSettingOrder(action)` (0x52F5F0)
    pub fn call_reset_setting_order(this: GPtr<u8>, action: i32) = thiscall 0x52F5F0 => reset_setting_order;
    /// `ClearSettingsAssociatedWithAction(action, type)` (0x52FD70)
    pub fn call_clear_settings(this: GPtr<u8>, action: i32, kind: i32) = thiscall 0x52FD70 => clear_settings_associated_with_action;
    /// `SetControllerKeyAssociatedWithAction(action, key, type)` (0x530490)
    pub fn call_set_key(this: GPtr<u8>, action: i32, key: u32, kind: i32) = thiscall 0x530490 => set_controller_key_associated_with_action;
    /// `SetMouseButtonAssociatedWithAction(action, button)` (0x52F590)
    pub fn call_set_mouse_button(this: GPtr<u8>, action: i32, button: u32) = thiscall 0x52F590 => set_mouse_button_associated_with_action;
    /// `MakeControllerActionsBlank()` (0x530500)
    pub fn call_make_blank(this: GPtr<u8>) = thiscall 0x530500 => make_controller_actions_blank;
    /// `InitDefaultControlConfiguration()` (0x530640)
    pub fn call_init_default(this: GPtr<u8>) = thiscall 0x530640 => init_default_control_configuration;
    /// `InitialiseControllerActionNameArray()` (0x52D260)
    pub fn call_init_names(this: GPtr<u8>) = thiscall 0x52D260 => initialise_controller_action_name_array;
    /// `InitDefaultControlConfigMouse(setUp, mouseControls)` (0x52F6F0)
    pub fn call_init_mouse(this: GPtr<u8>, set_up: GPtr<u8>, controls: u32) = thiscall 0x52F6F0 => init_default_control_config_mouse;
    /// `InitDefaultControlConfigJoyPad(buttons)` (0x530B00)
    pub fn call_init_joypad(this: GPtr<u8>, buttons: u32) = thiscall 0x530B00 => init_default_control_config_joy_pad;
    /// `GetIsKeyboardKeyDown(key)` (0x52DDB0, AL)
    pub fn call_key_down(this: GPtr<u8>, key: u32) -> u8 = thiscall 0x52DDB0 => get_is_keyboard_key_down;
    /// `GetIsMouseButtonDown(button)` (0x52EF30, AL)
    pub fn call_mouse_down(this: GPtr<u8>, button: u32) -> u8 = thiscall 0x52EF30 => get_is_mouse_button_down;
    /// `GetIsMouseButtonUp(button)` (0x52F020, AL)
    pub fn call_mouse_up(this: GPtr<u8>, button: u32) -> u8 = thiscall 0x52F020 => get_is_mouse_button_up;
    /// `AffectControllerStateOn_ButtonDown_Driving(key, type, state)` (0x52F7B0)
    pub fn call_down_driving(this: GPtr<u8>, key: u32, kind: i32, st: GPtr<u8>) = thiscall 0x52F7B0 => affect_button_down_driving;
    /// `..._FirstPersonOnly` (0x52F9E0)
    pub fn call_down_first(this: GPtr<u8>, key: u32, kind: i32, st: GPtr<u8>) = thiscall 0x52F9E0 => affect_button_down_first_person_only;
    /// `..._ThirdPersonOnly` (0x52FA20)
    pub fn call_down_third(this: GPtr<u8>, key: u32, kind: i32, st: GPtr<u8>) = thiscall 0x52FA20 => affect_button_down_third_person_only;
    /// `..._FirstAndThirdPersonOnly` (0x52FAB0)
    pub fn call_down_first_and_third(this: GPtr<u8>, key: u32, kind: i32, st: GPtr<u8>) = thiscall 0x52FAB0 => affect_button_down_first_and_third_person_only;
    /// `..._AllStates` (0x52FCA0)
    pub fn call_down_all(this: GPtr<u8>, key: u32, kind: i32, st: GPtr<u8>) = thiscall 0x52FCA0 => affect_button_down_all_states;
    /// `AffectControllerStateOn_ButtonUp_All_Player_States` (0x52FD40)
    pub fn call_up_all(this: GPtr<u8>, key: u32, kind: i32, st: GPtr<u8>) = thiscall 0x52FD40 => affect_button_up_all_player_states;
    /// `GetControllerSettingTextKeyBoard(action, type)` (0x52FE10)
    pub fn call_text_keyboard(this: GPtr<u8>, action: i32, kind: i32) -> GPtr<u8> = thiscall 0x52FE10 => get_controller_setting_text_keyboard;
    /// `GetControllerSettingTextMouse(action)` (0x52F390)
    pub fn call_text_mouse(this: GPtr<u8>, action: i32) -> GPtr<u8> = thiscall 0x52F390 => get_controller_setting_text_mouse;
    /// `GetControllerSettingTextJoystick(action)` (0x52F450)
    pub fn call_text_joystick(this: GPtr<u8>, action: i32) -> GPtr<u8> = thiscall 0x52F450 => get_controller_setting_text_joystick;
    /// `GetActionType(action)` (0x52F2F0)
    pub fn call_get_action_type(this: GPtr<u8>, action: i32) -> u32 = thiscall 0x52F2F0 => get_action_type;
    /// `DeleteMatchingCommonControls(action, key, type)` (0x531670)
    pub fn call_del_common(this: GPtr<u8>, action: i32, key: u32, kind: i32) = thiscall 0x531670 => delete_matching_common_controls;
    /// `DeleteMatching3rdPersonControls` (0x531730)
    pub fn call_del_3rd(this: GPtr<u8>, action: i32, key: u32, kind: i32) = thiscall 0x531730 => delete_matching_3rd_person_controls;
    /// `DeleteMatching1rst3rdPersonControls` (0x5318C0)
    pub fn call_del_1rst3rd(this: GPtr<u8>, action: i32, key: u32, kind: i32) = thiscall 0x5318C0 => delete_matching_1rst_3rd_person_controls;
    /// `DeleteMatchingVehicleControls` (0x5319D0)
    pub fn call_del_vehicle(this: GPtr<u8>, action: i32, key: u32, kind: i32) = thiscall 0x5319D0 => delete_matching_vehicle_controls;
    /// `DeleteMatchingVehicle_3rdPersonControls` (0x531BC0)
    pub fn call_del_vehicle_3rd(this: GPtr<u8>, action: i32, key: u32, kind: i32) = thiscall 0x531BC0 => delete_matching_vehicle_3rd_person_controls;
    /// `DeleteMatching1rstPersonControls` (0x531C20)
    pub fn call_del_1rst(this: GPtr<u8>, action: i32, key: u32, kind: i32) = thiscall 0x531C20 => delete_matching_1rst_person_controls;
}

// ------------------------------------------------------------------------------------------------
// field helpers

/// `&m_Actions[a].Keys[t].m_uiActionInitiator` (32-bit wrapping address math, no bounds check).
#[inline]
fn key_ptr(this: GPtr<u8>, a: i32, t: i32) -> GPtr<u32> {
    this.byte_add(ACTIONS.wrapping_add(a.wrapping_mul(0x20)).wrapping_add(t.wrapping_mul(8)) as isize).cast()
}
#[inline]
fn key(this: GPtr<u8>, a: i32, t: i32) -> u32 {
    key_ptr(this, a, t).read()
}
#[inline]
fn order_ptr(this: GPtr<u8>, a: i32, t: i32) -> GPtr<u32> {
    key_ptr(this, a, t).byte_add(4)
}
#[inline]
fn order(this: GPtr<u8>, a: i32, t: i32) -> u32 {
    order_ptr(this, a, t).read()
}
/// The byte at `this + base + t` (the stick "both down" flags).
#[inline]
fn flag(this: GPtr<u8>, base: i32, t: i32) -> GPtr<u8> {
    this.byte_add(base.wrapping_add(t) as isize)
}
#[inline]
fn byte_at(addr: u32) -> GPtr<u8> {
    GPtr::from_addr(addr as usize)
}
#[inline]
fn word_at(addr: u32) -> GPtr<i16> {
    GPtr::from_addr(addr as usize)
}
#[inline]
fn st16(st: GPtr<u8>, off: isize, v: i16) {
    wr::<i16>(st, off, v)
}
#[inline]
fn ld16(st: GPtr<u8>, off: isize) -> i16 {
    rd::<i16>(st, off)
}
fn menu_active() -> bool {
    g::<GBool>(MENU_ACTIVE).read().0 != 0
}
fn control_method() -> u8 {
    g::<u8>(CONTROL_METHOD).read()
}
fn the_text() -> GPtr<u8> {
    GPtr::from_addr(THE_TEXT)
}
/// `CText::Get` of the 8-byte key at `addr` (a string literal in .rdata).
fn text(addr: u32) -> GPtr<u8> {
    text_get(the_text(), GPtr::from_addr(addr as usize))
}

// ------------------------------------------------------------------------------------------------
// joystick buttons

/// `GetJoyButtonJustUp()` (0x52D1C0): the first button `i` (0..15) whose new state lacks bit 7 while
/// the old one has it, as `i + 1`; 0 when none.
/// Not hookable (the loop branches back to 0x52D1C4, inside the 11 patched bytes): ported, not registered.
pub fn get_joy_button_just_up(this: GPtr<u8>) -> u32 {
    for i in 0..16 {
        if rd::<u8>(this, 0x144 + i) & 0x80 == 0 && rd::<u8>(this, 0x34 + i) & 0x80 != 0 {
            return i as u32 + 1;
        }
    }
    0
}

/// `GetJoyButtonJustDown()` (0x52D1E0): new has bit 7, old has not.
/// Not hookable (the loop branches back to 0x52D1E4, inside the 11 patched bytes): ported, not registered.
pub fn get_joy_button_just_down(this: GPtr<u8>) -> u32 {
    for i in 0..16 {
        if rd::<u8>(this, 0x144 + i) & 0x80 != 0 && rd::<u8>(this, 0x34 + i) & 0x80 == 0 {
            return i as u32 + 1;
        }
    }
    0
}

/// `UpdateJoyButtonState(int32 padNumber)` (0x52F510): `m_ButtonStates[0..17] = NewJoyState.rgbButtons[i] >> 7`
/// (cleared first; `padNumber` unused).
#[hook(addr = 0x52F510, conv = thiscall, name = "CControllerConfigManager::UpdateJoyButtonState", expect = "8D 81 5C 0B 00 00", this = "CControllerConfigManager")]
pub fn update_joy_button_state(this: GPtr<u8>, _pad: i32) {
    for i in 0..17 {
        wr::<u8>(this, 0xB5C + i, 0);
    }
    for i in 0..17 {
        let b = rd::<u8>(this, 0x144 + i);
        wr::<u8>(this, 0xB5C + i, b >> 7);
    }
}

/// `StoreMouseButtonState(eMouseButtons button, bool state)` (0x52DA30): one byte of
/// `CPad::TempMouseControllerState`; 1 left, 2 middle, 3 right, 4 wheel up, 5 wheel down, 6 X1, 7 X2;
/// any other value writes nothing (gta-reversed: unreachable).
#[hook(addr = 0x52DA30, conv = thiscall, name = "CControllerConfigManager::StoreMouseButtonState", expect = "8B 44 24 04 83 F8 01", this = "CControllerConfigManager")]
pub fn store_mouse_button_state(_this: GPtr<u8>, button: i32, state: u8) {
    let off = match button {
        1 => 0,
        2 => 2,
        3 => 1,
        4 => 3,
        5 => 4,
        6 => 5,
        7 => 6,
        _ => return,
    };
    byte_at(TEMP_MOUSE + off).write(state);
}

/// The joystick button -> `PCTempJoyState` field of the vendor-independent buttons 5..16 (offsets in
/// the pad); `None` for 12 (Start, written only for pad 1).
fn joy_menu_field(button: i32) -> Option<isize> {
    Some(match button {
        5 => 0xB2,
        6 => 0xB6,
        7 => 0xB0,
        8 => 0xB4,
        9 => 0xC2,
        10 => 0xCC,
        11 => 0xCE,
        13 => 0xB8,
        14 => 0xBE,
        15 => 0xBA,
        16 => 0xBC,
        _ => return None,
    })
}

fn update_joy_in_config_menus(button: i32, pad_number: i32, v: i16) {
    if button == 0 {
        return;
    }
    let pad = get_pad(pad_number);
    if pad.is_null() {
        return;
    }
    if let Some(off) = joy_menu_field(button) {
        st16(pad, off, v);
    } else if button == 12 && pad_number == 1 {
        st16(pad, 0xC0, v);
    }
    let special = g::<u32>(JOY0_VENDOR).read() == 0x3427 && g::<u32>(JOY0_PRODUCT).read() == 0x1190;
    let off = match (special, button) {
        (true, 1) => 0xC6,
        (true, 2) => 0xCA,
        (true, 3) => 0xC8,
        (true, 4) => 0xC4,
        (false, 1) => 0xCA,
        (false, 2) => 0xC8,
        (false, 3) => 0xC4,
        (false, 4) => 0xC6,
        _ => return,
    };
    st16(pad, off, v);
}

/// `UpdateJoyInConfigMenus_ButtonDown(button, padNumber)` (0x52DAB0): 0xFF into the pad's
/// `PCTempJoyState` field of the button; Start (12) only for pad 1 (gta-reversed differs, see the
/// deviation notes); buttons 1..4 by the joystick's vendor/product.
#[hook(addr = 0x52DAB0, conv = thiscall, name = "CControllerConfigManager::UpdateJoyInConfigMenus_ButtonDown", expect = "57 8B 7C 24 08", this = "CControllerConfigManager")]
pub fn update_joy_in_config_menus_button_down(_this: GPtr<u8>, button: i32, pad_number: i32) {
    update_joy_in_config_menus(button, pad_number, 0xFF);
}

/// `UpdateJoyInConfigMenus_ButtonUp(button, padNumber)` (0x52DC20): the same fields cleared.
#[hook(addr = 0x52DC20, conv = thiscall, name = "CControllerConfigManager::UpdateJoyInConfigMenus_ButtonUp", expect = "53 8B 5C 24 08", this = "CControllerConfigManager")]
pub fn update_joy_in_config_menus_button_up(_this: GPtr<u8>, button: i32, pad_number: i32) {
    update_joy_in_config_menus(button, pad_number, 0);
}

/// `AffectControllerStateOn_ButtonDown_DebugStuff(key, type)` (0x52DC10): `ret 8` (not hookable).
pub fn affect_button_down_debug_stuff(_this: GPtr<u8>, _key: u32, _kind: i32) {}

/// `AffectControllerStateOn_ButtonUp_DebugStuff(key, type)` (0x52DD80): `ret 8` (not hookable).
pub fn affect_button_up_debug_stuff(_this: GPtr<u8>, _key: u32, _kind: i32) {}

/// `ClearSimButtonPressCheckers()` (0x52DD90): the 16 stick flag bytes +0x12D0..+0x12DF.
#[hook(addr = 0x52DD90, conv = thiscall, name = "CControllerConfigManager::ClearSimButtonPressCheckers", expect = "8D 81 D4 12 00 00", this = "CControllerConfigManager")]
pub fn clear_sim_button_press_checkers(this: GPtr<u8>) {
    clear_sim_flags(this);
}

/// The inlined loop of 0x52DD90 (also at the end of `InitDefaultControlConfiguration`): for i 0..3 the
/// bytes +0x12D0+i, +0x12D4+i, +0x12D8+i, +0x12DC+i.
fn clear_sim_flags(this: GPtr<u8>) {
    for i in 0..4 {
        for b in [0x12D0, 0x12D4, 0x12D8, 0x12DC] {
            wr::<u8>(this, b + i, 0);
        }
    }
}

// ------------------------------------------------------------------------------------------------
// keyboard / mouse queries

/// `CKeyboardState` word offset of key `1000 + i` (i = 0..55) as `GetIsKeyboardKeyDown` /
/// `GetIsKeyboardKeyJustDown` test it (their jump tables); `NO` for the function keys (handled by the
/// loop before the switch, so a function key that is not down reports false).
const NO: u16 = 0xFFFF;
const KEY_FIELD: [u16; 56] = [
    0x218, NO, NO, NO, NO, NO, NO, NO, NO, NO, NO, NO, NO, 0x21A, 0x21C, 0x21E, 0x220, 0x222, 0x224, 0x226,
    0x228, 0x22A, 0x22C, 0x234, 0x236, 0x23A, 0x238, 0x23E, 0x240, 0x242, 0x244, 0x246, 0x248, 0x232,
    0x24A, 0x24C, 0x24E, 0x250, 0x252, 0x23C, 0x22E, 0x230, 0x254, 0x256, 0x258, 0x25A, 0x25C, 0x25E,
    0x260, 0x262, 0x264, 0x266, 0x268, 0x26A, 0x26C, 0x26E,
];

/// The key's field in a key state, if the switch has one.
fn key_field(key: i32) -> Option<u32> {
    let k = key.wrapping_sub(1000) as u32;
    if k <= 0x37 && KEY_FIELD[k as usize] != NO { Some(u32::from(KEY_FIELD[k as usize])) } else { None }
}

/// `GetIsKeyboardKeyDown(RsKeyCodes key)` (0x52DDB0): every test is preceded by a `CPad::GetPad(0)`
/// call (result unused). Keys < 0xFF (signed) test `NewKeyState.standardKeys[key]` and fall through
/// to the function-key loop when up; F1..F12 (1001..1012) test `FKeys`; the rest by the jump table.
#[hook(addr = 0x52DDB0, conv = thiscall, name = "CControllerConfigManager::GetIsKeyboardKeyDown", expect = "57 8B 7C 24 08", this = "CControllerConfigManager")]
pub fn get_is_keyboard_key_down(_this: GPtr<u8>, key: u32) -> u8 {
    let k = key as i32;
    if k < 0xFF {
        let _ = get_pad(0);
        if word_at(NEW_KEY_STATE.wrapping_add(0x18).wrapping_add(key.wrapping_mul(2))).read() != 0 {
            return 1;
        }
    }
    for i in 0..12u32 {
        if 0x3E9 + i == key {
            let _ = get_pad(0);
            if word_at(NEW_KEY_STATE + 2 * i).read() != 0 {
                return 1;
            }
        }
    }
    if let Some(f) = key_field(k) {
        let _ = get_pad(0);
        if word_at(NEW_KEY_STATE + f).read() != 0 {
            return 1;
        }
    }
    0
}

/// `GetIsKeyboardKeyJustDown(RsKeyCodes key)` (0x52E450): as `GetIsKeyboardKeyDown`, with the old key
/// state (`OldKeyState`) tested after the new one (a second `GetPad(0)`): down now and not before.
#[hook(addr = 0x52E450, conv = thiscall, name = "CControllerConfigManager::GetIsKeyboardKeyJustDown", expect = "57 8B 7C 24 08", this = "CControllerConfigManager")]
pub fn get_is_keyboard_key_just_down(_this: GPtr<u8>, key: u32) -> u8 {
    let k = key as i32;
    let edge = |f: u32| -> Option<bool> {
        let _ = get_pad(0);
        if word_at(NEW_KEY_STATE.wrapping_add(f)).read() == 0 {
            return None;
        }
        let _ = get_pad(0);
        Some(word_at(OLD_KEY_STATE.wrapping_add(f)).read() == 0)
    };
    if k < 0xFF && edge(0x18u32.wrapping_add(key.wrapping_mul(2))) == Some(true) {
        return 1;
    }
    for i in 0..12u32 {
        if 0x3E9 + i == key && edge(2 * i) == Some(true) {
            return 1;
        }
    }
    if let Some(f) = key_field(k)
        && edge(f) == Some(true)
    {
        return 1;
    }
    0
}

/// Mouse button -> `CMouseControllerState` byte as `GetIsMouseButtonDown`/`Up` map it (1 left, 2 middle,
/// 3 right, 4/5 wheel, 6/7 X1/X2).
fn mouse_field(button: u32) -> Option<u32> {
    Some(match button {
        1 => 0,
        2 => 2,
        3 => 1,
        4 => 3,
        5 => 4,
        6 => 5,
        7 => 6,
        _ => return None,
    })
}

/// `GetIsMouseButtonDown(eMouseButtons)` (0x52EF30): `GetPad(0)` then the new mouse state's byte.
#[hook(addr = 0x52EF30, conv = thiscall, name = "CControllerConfigManager::GetIsMouseButtonDown", expect = "8B 44 24 04 48", this = "CControllerConfigManager")]
pub fn get_is_mouse_button_down(_this: GPtr<u8>, button: u32) -> u8 {
    let Some(f) = mouse_field(button) else { return 0 };
    let _ = get_pad(0);
    u8::from(byte_at(NEW_MOUSE + f).read() != 0)
}

/// `GetIsMouseButtonUp(eMouseButtons)` (0x52F020): `GetPad(0)` then "byte == 0".
#[hook(addr = 0x52F020, conv = thiscall, name = "CControllerConfigManager::GetIsMouseButtonUp", expect = "8B 44 24 04 48", this = "CControllerConfigManager")]
pub fn get_is_mouse_button_up(_this: GPtr<u8>, button: u32) -> u8 {
    let Some(f) = mouse_field(button) else { return 0 };
    let _ = get_pad(0);
    u8::from(byte_at(NEW_MOUSE + f).read() == 0)
}

/// `GetIsMouseButtonJustUp(eMouseButtons)` (0x52F110): new byte 0 and old byte set (two `GetPad(0)`).
/// Original bug kept: buttons 2 and 3 test the bytes in state order (2 -> +1 right, 3 -> +2 middle),
/// unlike `GetIsMouseButtonDown`/`Up`.
#[hook(addr = 0x52F110, conv = thiscall, name = "CControllerConfigManager::GetIsMouseButtonJustUp", expect = "8B 44 24 04 48", this = "CControllerConfigManager")]
pub fn get_is_mouse_button_just_up(_this: GPtr<u8>, button: u32) -> u8 {
    if !(1..=7).contains(&button) {
        return 0;
    }
    let f = button - 1;
    let _ = get_pad(0);
    if byte_at(NEW_MOUSE + f).read() != 0 {
        return 0;
    }
    let _ = get_pad(0);
    u8::from(byte_at(OLD_MOUSE + f).read() != 0)
}

/// `GetIsKeyBlank(key, type)` (0x52F2A0): types 0/1 `key == rsNULL`, 2/3 `key == 0`, any other type
/// true (gta-reversed: unreachable).
#[hook(addr = 0x52F2A0, conv = thiscall, name = "CControllerConfigManager::GetIsKeyBlank", expect = "8B 44 24 08 83 F8 03", this = "CControllerConfigManager")]
pub fn get_is_key_blank(_this: GPtr<u8>, key: u32, kind: u32) -> u8 {
    u8::from(is_blank(key, kind))
}

fn is_blank(key: u32, kind: u32) -> bool {
    match kind {
        0 | 1 => key == RS_NULL,
        2 | 3 => key == 0,
        _ => true,
    }
}

/// `GetActionType(action)` (0x52F2F0) table (byte table 0x52F350 -> jump table 0x52F338).
const ACTION_TYPE: [u8; 58] = [
    5, 5, 1, 1, 5, 5, 5, 5, 5, 5, 3, 4, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 5, 5, 2,
    2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 5, 4, 1, 1, 1, 1, 5, 5, 6, 4, 4, 4, 4,
];

/// `GetActionType(action)` (0x52F2F0): the action's group (1 first+third person, 2 vehicle, 3 vehicle +
/// third person, 4 common, 5 first/third + first person, 6 = none), 6 for actions > 57 (unsigned).
#[hook(addr = 0x52F2F0, conv = thiscall, name = "CControllerConfigManager::GetActionType", expect = "8B 44 24 04 83 F8 39", this = "CControllerConfigManager")]
pub fn get_action_type(_this: GPtr<u8>, action: i32) -> u32 {
    let a = action as u32;
    if a > 0x39 { 6 } else { u32::from(ACTION_TYPE[a as usize]) }
}

/// The GXT keys of the mouse buttons 1..7 (`FEC_MSL`, `FEC_MSM`, `FEC_MSR`, `FEC_MWF`, `FEC_MWB`,
/// `FEC_MXO`, `FEC_MXT`).
const MOUSE_TEXT: [u32; 7] = [0x863720, 0x863718, 0x863710, 0x863708, 0x863700, 0x8636F8, 0x8636F0];

/// `GetControllerSettingTextMouse(action)` (0x52F390): `TheText.Get` of the bound mouse button's key,
/// null when the binding is not 1..7.
#[hook(addr = 0x52F390, conv = thiscall, name = "CControllerConfigManager::GetControllerSettingTextMouse", expect = "8B 44 24 04 83 C0 5C", this = "CControllerConfigManager")]
pub fn get_controller_setting_text_mouse(this: GPtr<u8>, action: i32) -> GPtr<u8> {
    let b = key(this, action, 2);
    if b.wrapping_sub(1) > 6 {
        return GPtr::NULL;
    }
    text(MOUSE_TEXT[(b - 1) as usize])
}

/// The joystick button text: `InsertNumberInString(TheText.Get("FEC_JBO"), button, -1 x 5,
/// joy text buffer)`.
fn joy_button_text(button: u32) -> GPtr<u8> {
    let out: GPtr<u8> = GPtr::from_addr(JOY_TEXT_BUF);
    insert_number_in_string(text(0x863728), button as i32, -1, -1, -1, -1, -1, out);
    out
}

/// `GetControllerSettingTextJoystick(action)` (0x52F450): null when no button is bound, else the
/// "button N" text in the static buffer 0xB7147C.
#[hook(addr = 0x52F450, conv = thiscall, name = "CControllerConfigManager::GetControllerSettingTextJoystick", expect = "8B 44 24 04 C1 E0 05", this = "CControllerConfigManager")]
pub fn get_controller_setting_text_joystick(this: GPtr<u8>, action: i32) -> GPtr<u8> {
    let b = key(this, action, 3);
    if b == 0 {
        return GPtr::NULL;
    }
    joy_button_text(b)
}

/// The number of bindings of an action: keyboard and extra key when not `rsNULL`, mouse and joystick
/// when not 0.
fn num_settings(this: GPtr<u8>, action: i32) -> u32 {
    u32::from(key(this, action, 0) != RS_NULL)
        + u32::from(key(this, action, 1) != RS_NULL)
        + u32::from(key(this, action, 2) != 0)
        + u32::from(key(this, action, 3) != 0)
}

/// `GetNumOfSettingsForAction(action)` (0x52F4A0).
#[hook(addr = 0x52F4A0, conv = thiscall, name = "CControllerConfigManager::GetNumOfSettingsForAction", expect = "53 56 8B 74 24 0C", this = "CControllerConfigManager")]
pub fn get_num_of_settings_for_action(this: GPtr<u8>, action: i32) -> u32 {
    num_settings(this, action)
}

/// `GetControllerKeyAssociatedWithAction(action, type)` (0x52F4F0).
#[hook(addr = 0x52F4F0, conv = thiscall, name = "CControllerConfigManager::GetControllerKeyAssociatedWithAction", expect = "8B 44 24 08 8B 54 24 04", this = "CControllerConfigManager")]
pub fn get_controller_key_associated_with_action(this: GPtr<u8>, action: i32, kind: i32) -> u32 {
    key(this, action, kind)
}

/// `GetIsActionAButtonCombo(action)` (0x52F550): `xor al, al`.
#[hook(addr = 0x52F550, conv = thiscall, name = "CControllerConfigManager::GetIsActionAButtonCombo", expect = "32 C0 C2 04 00", this = "CControllerConfigManager")]
pub fn get_is_action_a_button_combo(_this: GPtr<u8>, _action: i32) -> u8 {
    0
}

/// `GetButtonComboText(action)` (0x52F560): `TheText.Get("FEC_LBC")` for action 36
/// (VEHICLE_LOOKBEHIND), else null.
#[hook(addr = 0x52F560, conv = thiscall, name = "CControllerConfigManager::GetButtonComboText", expect = "83 7C 24 04 24", this = "CControllerConfigManager")]
pub fn get_button_combo_text(_this: GPtr<u8>, action: i32) -> GPtr<u8> {
    if action != 0x24 {
        return GPtr::NULL;
    }
    text(0x863730)
}

/// `GetMouseButtonAssociatedWithAction(action)` (0x52F580).
#[hook(addr = 0x52F580, conv = thiscall, name = "CControllerConfigManager::GetMouseButtonAssociatedWithAction", expect = "8B 44 24 04 83 C0 5C", this = "CControllerConfigManager")]
pub fn get_mouse_button_associated_with_action(this: GPtr<u8>, action: i32) -> u32 {
    key(this, action, 2)
}

/// `SetMouseButtonAssociatedWithAction(action, button)` (0x52F590): the mouse binding := button, its
/// set order := (bindings before the store) + 1.
#[hook(addr = 0x52F590, conv = thiscall, name = "CControllerConfigManager::SetMouseButtonAssociatedWithAction", expect = "53 56 8B 74 24 0C", this = "CControllerConfigManager")]
pub fn set_mouse_button_associated_with_action(this: GPtr<u8>, action: i32, button: u32) {
    let n = num_settings(this, action);
    key_ptr(this, action, 2).write(button);
    order_ptr(this, action, 2).write(n + 1);
}

/// `ResetSettingOrder(action)` (0x52F5F0): renumbers the set orders 1..4 so they have no gaps. For each
/// order `o` = 1..4 not used by any binding: the binding with the smallest order above `o` (signed
/// compares, ties to the lowest type) takes `o`.
#[hook(addr = 0x52F5F0, conv = thiscall, name = "CControllerConfigManager::ResetSettingOrder", expect = "53 55 56 57 8B 7C 24 14", this = "CControllerConfigManager")]
pub fn reset_setting_order(this: GPtr<u8>, action: i32) {
    let o = |t: i32| order(this, action, t) as i32;
    let mut best = 0i32; // EAX: kept across iterations, only read when `found`
    for want in 1..5i32 {
        if (0..4).any(|t| o(t) == want) {
            continue;
        }
        let mut found = false;
        let o0 = o(0);
        if o0 > want && o0 != 0 {
            found = true;
            best = 0;
        }
        for t in 1..3 {
            let v = o(t);
            if v > want && v != 0 {
                if !found {
                    found = true;
                    best = t;
                } else if v < o(best) {
                    best = t;
                }
            }
        }
        let v = o(3);
        if v > want && v != 0 {
            if !found {
                order_ptr(this, action, 3).write(want as u32);
                continue;
            } else if v < o(best) {
                best = 3;
            }
        }
        if found {
            order_ptr(this, action, best).write(want as u32);
        }
    }
}

/// `InitDefaultControlConfigMouse(const CMouseControllerState& setUp, bool)` (0x52F6F0): binds the
/// buttons the mouse has (`SetMouseButtonAssociatedWithAction`); `m_MouseFoundInitSet := 1` when it has a
/// left button and is never cleared (gta-reversed clears it first); the bool is unused.
#[hook(addr = 0x52F6F0, conv = thiscall, name = "CControllerConfigManager::InitDefaultControlConfigMouse", expect = "56 57 8B 7C 24 0C", this = "CControllerConfigManager")]
pub fn init_default_control_config_mouse(this: GPtr<u8>, set_up: GPtr<u8>, _controls: u32) {
    if rd::<u8>(set_up, 0) != 0 {
        wr::<u8>(this, 0x12E0, 1);
        call_set_mouse_button(this, 0, 1);
        call_set_mouse_button(this, 0x12, 1);
    }
    if rd::<u8>(set_up, 1) != 0 {
        call_set_mouse_button(this, 0x2D, 3);
        call_set_mouse_button(this, 0x25, 3);
    }
    if rd::<u8>(set_up, 2) != 0 {
        call_set_mouse_button(this, 0x24, 2);
        call_set_mouse_button(this, 0x0E, 2);
    }
    if rd::<u8>(set_up, 3) != 0 || rd::<u8>(set_up, 4) != 0 {
        call_set_mouse_button(this, 3, 4);
        call_set_mouse_button(this, 2, 5);
        call_set_mouse_button(this, 0x1A, 4);
        call_set_mouse_button(this, 0x1B, 5);
        call_set_mouse_button(this, 8, 4);
        call_set_mouse_button(this, 9, 5);
    }
}

// ------------------------------------------------------------------------------------------------
// effect of a pressed binding on a CControllerState

/// `Keys[type]` of `action` is `key`.
#[inline]
fn is(this: GPtr<u8>, action: i32, kind: i32, k: u32) -> bool {
    key(this, action, kind) == k
}

/// Steering keys: when the axis' "both down" flag is set the axis is zeroed (and the flag set again),
/// else it takes `v`. The opposite key's value is not tested (gta-reversed does).
fn steer(this: GPtr<u8>, flags: i32, kind: i32, st: GPtr<u8>, off: isize, v: i16) {
    let f = flag(this, flags, kind);
    if f.read() != 0 {
        st16(st, off, 0);
        f.write(1);
    } else {
        st16(st, off, v);
    }
}

/// Look/turret keys: zeroed (flag set) when the axis already holds `opposite` or the flag is set,
/// else `v`.
fn turret(this: GPtr<u8>, flags: i32, kind: i32, st: GPtr<u8>, off: isize, opposite: i16, v: i16) {
    let f = flag(this, flags, kind);
    if ld16(st, off) == opposite || f.read() != 0 {
        st16(st, off, 0);
        f.write(1);
    } else {
        st16(st, off, v);
    }
}

/// `AffectControllerStateOn_ButtonDown_Driving(key, type, CControllerState*)` (0x52F7B0). Order of the
/// original: 18 Circle, 19 LeftShoulder1, 36 both shoulders 2, 34 LeftShoulder2, 35 RightShoulder2,
/// 37 vehicle mouse look, 29 ShockButtonL, 31 RightShoulder1, 24 Cross, 25 Square, 22/23 left stick Y,
/// 20/21 left stick X, 26 DPadUp, 27 DPadDown, 28 radio track skip, 30 ShockButtonR, 38..41 turrets.
#[hook(addr = 0x52F7B0, conv = thiscall, name = "CControllerConfigManager::AffectControllerStateOn_ButtonDown_Driving", expect = "8B 44 24 08 8B 94 C1 B0 0D 00 00", this = "CControllerConfigManager")]
pub fn affect_button_down_driving(this: GPtr<u8>, k: u32, kind: i32, st: GPtr<u8>) {
    let on = |a: i32| is(this, a, kind, k);
    if on(18) {
        st16(st, 0x22, 0xFF);
    }
    if on(19) {
        st16(st, 0x08, 0xFF);
    }
    if on(36) {
        st16(st, 0x0A, 0xFF);
        st16(st, 0x0E, 0xFF);
    }
    if on(34) {
        st16(st, 0x0A, 0xFF);
    }
    if on(35) {
        st16(st, 0x0E, 0xFF);
    }
    if on(37) {
        st16(st, 0x2C, 0xFF);
    }
    if on(29) {
        st16(st, 0x24, 0xFF);
    }
    if on(31) {
        st16(st, 0x0C, 0xFF);
    }
    if on(24) {
        st16(st, 0x20, 0xFF);
    }
    if on(25) {
        st16(st, 0x1C, 0xFF);
    }
    if on(22) {
        steer(this, 0x12D4, kind, st, 0x02, -128);
    }
    if on(23) {
        steer(this, 0x12D4, kind, st, 0x02, 128);
    }
    if on(20) {
        steer(this, 0x12D0, kind, st, 0x00, -128);
    }
    if on(21) {
        steer(this, 0x12D0, kind, st, 0x00, 128);
    }
    if on(26) {
        st16(st, 0x10, 0xFF);
    }
    if on(27) {
        st16(st, 0x12, 0xFF);
    }
    if on(28) {
        st16(st, 0x2E, 0xFF);
    }
    if on(30) {
        st16(st, 0x26, 0xFF);
    }
    if on(38) {
        turret(this, 0x12D8, kind, st, 0x04, 128, -128);
    }
    if on(39) {
        turret(this, 0x12D8, kind, st, 0x04, -128, 128);
    }
    if on(40) {
        turret(this, 0x12DC, kind, st, 0x06, -128, 128);
    }
    if on(41) {
        turret(this, 0x12DC, kind, st, 0x06, 128, -128);
    }
}

/// `..._FirstPersonOnly(key, type, state)` (0x52F9E0): 8 sniper zoom in -> Square, 9 zoom out ->
/// Cross, 15 duck -> ShockButtonL.
#[hook(addr = 0x52F9E0, conv = thiscall, name = "CControllerConfigManager::AffectControllerStateOn_ButtonDown_FirstPersonOnly", expect = "8B 54 24 08 8B 44 24 04", this = "CControllerConfigManager", preserve = "ecx")]
pub fn affect_button_down_first_person_only(this: GPtr<u8>, k: u32, kind: i32, st: GPtr<u8>) {
    if is(this, 8, kind, k) {
        st16(st, 0x1C, 0xFF);
    }
    if is(this, 9, kind, k) {
        st16(st, 0x20, 0xFF);
    }
    if is(this, 15, kind, k) {
        st16(st, 0x24, 0xFF);
    }
}

/// `..._ThirdPersonOnly(key, type, state)` (0x52FA20): 14 ShockButtonR, 12 Square, 16 LeftShoulder1,
/// 3 LeftShoulder2, 2 RightShoulder2, 13 Cross, 15 ShockButtonL; with the joypad control method 44
/// also LeftShoulder1.
#[hook(addr = 0x52FA20, conv = thiscall, name = "CControllerConfigManager::AffectControllerStateOn_ButtonDown_ThirdPersonOnly", expect = "8B 54 24 08 8B 44 24 04", this = "CControllerConfigManager")]
pub fn affect_button_down_third_person_only(this: GPtr<u8>, k: u32, kind: i32, st: GPtr<u8>) {
    for (a, off) in [(14, 0x26), (12, 0x1C), (16, 0x08), (3, 0x0A), (2, 0x0E), (13, 0x20), (15, 0x24)] {
        if is(this, a, kind, k) {
            st16(st, off, 0xFF);
        }
    }
    if control_method() == 1 && is(this, 44, kind, k) {
        st16(st, 0x08, 0xFF);
    }
}

/// DPad pair keys (group control / conversation): when the other direction is set both are cleared,
/// else this direction := 0xFF.
fn dpad_pair(st: GPtr<u8>, mine: isize, other: isize) {
    if ld16(st, other) != 0 {
        st16(st, other, 0);
        st16(st, mine, 0);
    } else {
        st16(st, mine, 0xFF);
    }
}

/// `..._FirstAndThirdPersonOnly(key, type, state)` (0x52FAB0): `GetPad(0)` (unused) first; 0 Circle,
/// 1 LeftShoulder1, 45 RightShoulder1, 4/5 left stick Y, 6/7 left stick X, 17 ped walk, 49/50 group
/// control (DPad up/down), 32/33 right stick X; with the joypad control method 51/52 right stick Y.
#[hook(addr = 0x52FAB0, conv = thiscall, name = "CControllerConfigManager::AffectControllerStateOn_ButtonDown_FirstAndThirdPersonOnly", expect = "53 55 56 57 33 DB", this = "CControllerConfigManager")]
pub fn affect_button_down_first_and_third_person_only(this: GPtr<u8>, k: u32, kind: i32, st: GPtr<u8>) {
    let _ = get_pad(0);
    let on = |a: i32| is(this, a, kind, k);
    if on(0) {
        st16(st, 0x22, 0xFF);
    }
    if on(1) {
        st16(st, 0x08, 0xFF);
    }
    if on(45) {
        st16(st, 0x0C, 0xFF);
    }
    if on(4) {
        steer(this, 0x12D4, kind, st, 0x02, -128);
    }
    if on(5) {
        steer(this, 0x12D4, kind, st, 0x02, 128);
    }
    if on(6) {
        steer(this, 0x12D0, kind, st, 0x00, -128);
    }
    if on(7) {
        steer(this, 0x12D0, kind, st, 0x00, 128);
    }
    if on(17) {
        st16(st, 0x2A, 0xFF);
    }
    if on(49) {
        // GROUP_CONTROL_FWD: DPadUp unless DPadDown is set
        dpad_pair(st, 0x10, 0x12);
    }
    if on(50) {
        dpad_pair(st, 0x12, 0x10);
    }
    if on(32) {
        turret(this, 0x12D8, kind, st, 0x04, 128, -128);
    }
    if on(33) {
        turret(this, 0x12D8, kind, st, 0x04, -128, 128);
    }
    if control_method() == 1 {
        if on(51) {
            turret(this, 0x12DC, kind, st, 0x06, -128, 128);
        }
        if on(52) {
            turret(this, 0x12DC, kind, st, 0x06, 128, -128);
        }
    }
}

/// `..._AllStates(key, type, state)` (0x52FCA0): 11 Select, 48 conversation no (DPadLeft unless
/// DPadRight), 47 yes (DPadRight unless DPadLeft), 46 chat indicated.
#[hook(addr = 0x52FCA0, conv = thiscall, name = "CControllerConfigManager::AffectControllerStateOn_ButtonDown_AllStates", expect = "8B 54 24 04 53", this = "CControllerConfigManager")]
pub fn affect_button_down_all_states(this: GPtr<u8>, k: u32, kind: i32, st: GPtr<u8>) {
    if is(this, 11, kind, k) {
        st16(st, 0x1A, 0xFF);
    }
    if is(this, 48, kind, k) {
        dpad_pair(st, 0x14, 0x16);
    }
    if is(this, 47, kind, k) {
        dpad_pair(st, 0x16, 0x14);
    }
    if is(this, 46, kind, k) {
        st16(st, 0x28, 0xFF);
    }
}

/// `..._VehicleAndThirdPersonOnly(key, type, state)` (0x52FD20): 10 enter/exit -> Triangle.
#[hook(addr = 0x52FD20, conv = thiscall, name = "CControllerConfigManager::AffectControllerStateOn_ButtonDown_VehicleAndThirdPersonOnly", expect = "8B 44 24 08 8B 8C C1 B0 0C 00 00", this = "CControllerConfigManager")]
pub fn affect_button_down_vehicle_and_third_person_only(this: GPtr<u8>, k: u32, kind: i32, st: GPtr<u8>) {
    if is(this, 10, kind, k) {
        st16(st, 0x1E, 0xFF);
    }
}

/// `AffectControllerStateOn_ButtonUp_All_Player_States(key, type, state)` (0x52FD40): 46 clears the
/// chat flag, 37 the vehicle mouse look.
#[hook(addr = 0x52FD40, conv = thiscall, name = "CControllerConfigManager::AffectControllerStateOn_ButtonUp_All_Player_States", expect = "8B 54 24 08 8B 44 24 04", this = "CControllerConfigManager")]
pub fn affect_button_up_all_player_states(this: GPtr<u8>, k: u32, kind: i32, st: GPtr<u8>) {
    if is(this, 46, kind, k) {
        st16(st, 0x28, 0);
    }
    if is(this, 37, kind, k) {
        st16(st, 0x2C, 0);
    }
}

/// `ClearSettingsAssociatedWithAction(action, type)` (0x52FD70): the binding := blank (0x420 for types
/// 0/1, 0 for 2/3), order 0; then `ResetSettingOrder(action)`. Any other type only re-sorts
/// (gta-reversed: unreachable).
#[hook(addr = 0x52FD70, conv = thiscall, name = "CControllerConfigManager::ClearSettingsAssociatedWithAction", expect = "8B 54 24 08 83 FA 03", this = "CControllerConfigManager")]
pub fn clear_settings_associated_with_action(this: GPtr<u8>, action: i32, kind: i32) {
    match kind {
        0 | 1 => {
            key_ptr(this, action, kind).write(RS_NULL);
            order_ptr(this, action, kind).write(0);
        }
        2 | 3 => {
            key_ptr(this, action, kind).write(0);
            order_ptr(this, action, kind).write(0);
        }
        _ => {}
    }
    call_reset_setting_order(this, action);
}

// ------------------------------------------------------------------------------------------------
// key texts

/// `GetControllerSettingTextKeyBoard` switch for keys 1001..1055 (index `key - 1001`): GXT key, or a
/// numpad digit (`FEC_NMN` with the number), or a function key (`FEC_FNC` with `key - 1000`).
enum KeyText {
    Fn,
    Num(i32),
    Gxt(u32),
}

const fn key_text(i: u32) -> KeyText {
    use KeyText::*;
    match i {
        0..=11 => Fn,
        12 => Gxt(0x863840),
        13 => Gxt(0x863838),
        14 => Gxt(0x863830),
        15 => Gxt(0x863828),
        16 => Gxt(0x863820),
        17 => Gxt(0x863818),
        18 => Gxt(0x863810),
        19 => Gxt(0x863808),
        20 => Gxt(0x863800),
        21 => Gxt(0x8637F8),
        22 => Gxt(0x8637F0),
        23 => Gxt(0x8637E8),
        24 => Gxt(0x8637E0),
        25 => Gxt(0x8637D8),
        26 => Gxt(0x8637D0),
        27 => Num(1),
        28 => Num(2),
        29 => Num(3),
        30 => Num(4),
        31 => Num(5),
        32 => Gxt(0x8637C8),
        33 => Num(6),
        34 => Num(7),
        35 => Num(8),
        36 => Num(9),
        37 => Num(0),
        38 => Gxt(0x8637B8),
        39 => Gxt(0x8637B0),
        40 => Gxt(0x8637A8),
        41 => Gxt(0x8637A0),
        42 => Gxt(0x863798),
        43 => Gxt(0x863790),
        44 => Gxt(0x863788),
        45 => Gxt(0x863780),
        46 => Gxt(0x863778),
        47 => Gxt(0x863738),
        48 => Gxt(0x863770),
        49 => Gxt(0x863768),
        50 => Gxt(0x863760),
        51 => Gxt(0x863758),
        52 => Gxt(0x863750),
        53 => Gxt(0x863748),
        _ => Gxt(0x863740),
    }
}

/// `GetControllerSettingTextKeyBoard(action, type)` (0x52FE10): clears the 50-byte buffer 0xB714BC,
/// then by the bound key: `rsNULL` -> null; 0..255: `*` -> `FEC_AST`, space -> `FEC_SPC`, `^` with
/// language 2 -> "|", 0xB2 with language 1 -> "2", else the font character of the key (or of `#`
/// when it has none) as a one-character string in the buffer; 1001..1055 from the table (function
/// keys and numpad digits through `InsertNumberInString` into 0xB7149C); anything else null.
#[hook(addr = 0x52FE10, conv = thiscall, name = "CControllerConfigManager::GetControllerSettingTextKeyBoard", expect = "83 EC 08 57 8B D1", this = "CControllerConfigManager")]
pub fn get_controller_setting_text_keyboard(this: GPtr<u8>, action: i32, kind: i32) -> GPtr<u8> {
    for i in 0..50 {
        byte_at(CHAR_TEXT_BUF + i).write(0);
    }
    let k = key(this, action, kind);
    if k == RS_NULL {
        return GPtr::NULL;
    }
    let buf: GPtr<u8> = GPtr::from_addr(CHAR_TEXT_BUF as usize);
    if (k as i32) >= 0 && (k as i32) <= 0xFF {
        let c = k as u8;
        if c == 0x2A {
            return text(0x863858);
        }
        if c == 0x20 {
            return text(0x863850);
        }
        let lang = g::<i32>(LANGUAGE).read();
        if c == 0x5E && lang == 2 {
            buf.write(0x7C);
            byte_at(CHAR_TEXT_BUF + 1).write(0);
            return buf;
        }
        if c == 0xB2 && lang == 1 {
            buf.write(0x32);
            byte_at(CHAR_TEXT_BUF + 1).write(0);
            return buf;
        }
        let mut ch = key_char(u32::from(c));
        buf.write(ch);
        if ch == 0 {
            ch = key_char(0x23);
            buf.write(ch);
        }
        byte_at(CHAR_TEXT_BUF + 1).write(0);
        return buf;
    }
    let i = k.wrapping_sub(0x3E9);
    if i > 0x36 {
        return GPtr::NULL;
    }
    let out: GPtr<u8> = GPtr::from_addr(KEY_TEXT_BUF);
    match key_text(i) {
        KeyText::Fn => {
            insert_number_in_string(text(0x863848), k.wrapping_sub(1000) as i32, -1, -1, -1, -1, -1, out);
            out
        }
        KeyText::Num(n) => {
            insert_number_in_string(text(0x8637C0), n, -1, -1, -1, -1, -1, out);
            out
        }
        KeyText::Gxt(a) => text(a),
    }
}

/// `GetGxtStringOfCommandKeys(uint16 action, GxtChar* out, int32 len)` (0x5303D0): the first text of
/// (joypad method: joystick, else mouse), keyboard, extra key, joystick, mouse; then
/// `StringCopy(out, text, len - GetStringLength(text))`.
#[hook(addr = 0x5303D0, conv = thiscall, name = "CControllerConfigManager::GetGxtStringOfCommandKeys", expect = "A0 18 68 BA 00", this = "CControllerConfigManager")]
pub fn get_gxt_string_of_command_keys(this: GPtr<u8>, action: u16, out: GPtr<u8>, len: i32) {
    let a = i32::from(action);
    let joy = |this: GPtr<u8>| -> u32 { key(this, a, 3) };
    let mut s = GPtr::NULL;
    let mut done = false;
    if control_method() != 0 {
        let j = joy(this);
        if j != 0 {
            s = joy_button_text(j);
            done = true;
        }
    } else {
        s = call_text_mouse(this, a);
        done = !s.is_null();
    }
    if !done {
        s = call_text_keyboard(this, a, 0);
        if s.is_null() {
            s = call_text_keyboard(this, a, 1);
            if s.is_null() {
                let j = joy(this);
                s = if j != 0 { joy_button_text(j) } else { call_text_mouse(this, a) };
            }
        }
    }
    let n = get_string_length(s);
    string_copy(out, s, len.wrapping_sub(n));
}

/// `SetControllerKeyAssociatedWithAction(action, key, type)` (0x530490): `ResetSettingOrder(action)`,
/// then the binding := key, its order := (bindings before the store) + 1.
#[hook(addr = 0x530490, conv = thiscall, name = "CControllerConfigManager::SetControllerKeyAssociatedWithAction", expect = "53 56 57 8B 7C 24 10", this = "CControllerConfigManager")]
pub fn set_controller_key_associated_with_action(this: GPtr<u8>, action: i32, k: u32, kind: i32) {
    call_reset_setting_order(this, action);
    let n = num_settings(this, action);
    key_ptr(this, action, kind).write(k);
    order_ptr(this, action, kind).write(n + 1);
}

/// `MakeControllerActionsBlank()` (0x530500): `ClearSettingsAssociatedWithAction(action, type)` for
/// every type (outer) and action.
#[hook(addr = 0x530500, conv = thiscall, name = "CControllerConfigManager::MakeControllerActionsBlank", expect = "53 56 57 8B D9", this = "CControllerConfigManager")]
pub fn make_controller_actions_blank(this: GPtr<u8>) {
    for t in 0..4 {
        for a in 0..NUM_ACTIONS {
            call_clear_settings(this, a, t);
        }
    }
}

/// `LoadSettings(FILE* file)` (0x530530): reads a 29-byte header; when its first 26 bytes equal the
/// string at 0x8D2BE4 (or the file is null) nothing is loaded and the result is true. Otherwise a
/// version dword (< 6: true, nothing loaded), then a check pass (every 4+8-byte record's action index
/// must match, else false) and the load pass into `ControlsManager` (the global, not `this`) after
/// `MakeControllerActionsBlank`.
#[hook(addr = 0x530530, conv = thiscall, name = "CControllerConfigManager::LoadSettings", expect = "83 EC 38 53 55", this = "CControllerConfigManager")]
pub fn load_settings(_this: GPtr<u8>, file: i32) -> u8 {
    if file == 0 {
        return 1;
    }
    let mut hdr = [0u8; 0x20];
    let hp = GPtr::from_raw(hdr.as_mut_ptr());
    file_read(file, hp, 0x1D);
    if strncmp(hp, GPtr::from_addr(0x8D2BE4), 0x1A) == 0 {
        return 1;
    }
    file_seek(file, 0, 0);
    let mut v = [0i32; 1];
    let vp = GPtr::<i32>::from_raw(v.as_mut_ptr());
    file_read(file, vp.cast(), 4);
    if vp.read() < 6 {
        return 1;
    }
    for _t in 0..4 {
        for a in 0..NUM_ACTIONS {
            file_read(file, vp.cast(), 4);
            if vp.read() != a {
                return 0;
            }
            file_seek(file, 8, 1);
        }
    }
    file_seek(file, 4, 0);
    let cm: GPtr<u8> = GPtr::from_addr(CONTROLS_MANAGER);
    call_make_blank(cm);
    for t in 0..4 {
        for a in 0..NUM_ACTIONS {
            file_seek(file, 4, 1);
            file_read(file, key_ptr(cm, a, t).cast(), 8);
        }
    }
    1
}

/// `SaveSettings(FILE* file)` (0x52D200): for each type and action, the action index (4 bytes) and
/// the binding of `ControlsManager` (8 bytes). Nothing for a null file. (The original keeps the
/// action index in its own argument slot.)
#[hook(addr = 0x52D200, conv = thiscall, name = "CControllerConfigManager::SaveSettings", expect = "56 8B 74 24 08", this = "CControllerConfigManager")]
pub fn save_settings(_this: GPtr<u8>, file: i32) {
    if file == 0 {
        return;
    }
    let cm: GPtr<u8> = GPtr::from_addr(CONTROLS_MANAGER);
    let mut v = [0i32; 1];
    let vp = GPtr::<i32>::from_raw(v.as_mut_ptr());
    for t in 0..4 {
        for a in 0..NUM_ACTIONS {
            vp.write(a);
            file_write(file, vp.cast(), 4);
            file_write(file, key_ptr(cm, a, t).cast(), 8);
        }
    }
}

/// `InitialiseControllerActionNameArray()` (0x52D260): (destination offset, ASCII name) in the
/// original's order: `AsciiToGxtChar` into a 40-byte local, then `CMessages::StringCopy(dst, local, 40)`.
/// Slots 28, 53 and 58 are never written; slot 45 is written twice.
const ACTION_NAMES: [(u16, u32); 57] = [
    (0x454, 0x8636E0),
    (0x29C, 0x8636C8),
    (0x274, 0x8636B0),
    (0x92C, 0x8636A0),
    (0x404, 0x863694),
    (0x42C, 0x863688),
    (0x4CC, 0x86367C),
    (0x8B4, 0x863664),
    (0x8DC, 0x86364C),
    (0x92C, 0x8636A0),
    (0x904, 0x86362C),
    (0x7C4, 0x863618),
    (0x47C, 0x86360C),
    (0x4A4, 0x8635F8),
    (0x544, 0x8635E4),
    (0x56C, 0x8635D0),
    (0x594, 0x8635C0),
    (0x5BC, 0x8635AC),
    (0x774, 0x863598),
    (0x79C, 0x863584),
    (0x7EC, 0x863570),
    (0x6AC, 0x863560),
    (0x6FC, 0x86354C),
    (0x5E4, 0x863538),
    (0x60C, 0x863528),
    (0x634, 0x86350C),
    (0x65C, 0x8634F0),
    (0x6D4, 0x8634DC),
    (0x364, 0x8634C8),
    (0x38C, 0x8634B4),
    (0x724, 0x863498),
    (0x74C, 0x86347C),
    (0xA1C, 0x863464),
    (0xA44, 0x863448),
    (0xB0C, 0x86342C),
    (0x3DC, 0x863408),
    (0x224, 0x8633F8),
    (0x24C, 0x8633E4),
    (0x4F4, 0x8633D0),
    (0x51C, 0x8633B8),
    (0x3B4, 0x8633A4),
    (0x9A4, 0x863394),
    (0x97C, 0x863380),
    (0x9CC, 0x86336C),
    (0x9F4, 0x863358),
    (0x314, 0x863350),
    (0x33C, 0x863344),
    (0x2C4, 0x863338),
    (0x2EC, 0x863330),
    (0x814, 0x86331C),
    (0x83C, 0x863308),
    (0x864, 0x8632F4),
    (0x88C, 0x8632E0),
    (0x954, 0x8632D0),
    (0xA94, 0x8632C4),
    (0xABC, 0x8632B0),
    (0xAE4, 0x86329C),
];

#[hook(addr = 0x52D260, conv = thiscall, name = "CControllerConfigManager::InitialiseControllerActionNameArray", expect = "83 EC 28 56 57", this = "CControllerConfigManager")]
pub fn initialise_controller_action_name_array(this: GPtr<u8>) {
    let mut local = [0u8; 0x28];
    let lp = GPtr::from_raw(local.as_mut_ptr());
    for (dst, src) in ACTION_NAMES {
        ascii_to_gxt_char(GPtr::from_addr(src as usize), lp);
        string_copy(this.byte_add(dst as isize), lp, 0x28);
    }
}

/// The calls of `InitDefaultControlConfiguration` (0x530640) as (action, key, type), in order, with the
/// language-dependent parts (`LANGUAGE` 1 / 2) and the first call's key (made by the no-CD chunk)
/// handled in the function.
const DEFAULT_KEYS_A: [(i32, u32, i32); 34] = [
    (5, 0x53, 1),
    (7, 0x44, 1),
    (2, 0x45, 0),
    (2, 0x40F, 1),
    (3, 0x403, 1),
    (0, 0x40E, 0),
    (0, 0x419, 1),
    (8, 0x3F9, 0),
    (9, 0x3FA, 0),
    (1, 0x5C, 0),
    (0x31, 0x47, 0),
    (0x32, 0x48, 0),
    (0x30, 0x4E, 0),
    (0xA, 0x46, 1),
    (0xA, 0x415, 0),
    (0xB, 0x56, 0),
    (0xB, 0x3F7, 1),
    (0xC, 0x416, 0),
    (0xC, 0x41A, 1),
    (0xD, 0x20, 0),
    (0xF, 0x43, 0),
    (0x10, 0x413, 0),
    (0x11, 0x41B, 0),
    (0x2D, 0x3F6, 0),
    (0x2D, 0x414, 1),
    (0x20, 0x407, 0),
    (0x21, 0x40A, 0),
    (0x33, 0x405, 0),
    (0x34, 0x40C, 0),
    (0x2C, 0x23, 0),
    (0xE, 0x404, 0),
    (0x16, 0x3FB, 0),
    (0x17, 0x3FC, 0),
    (0x1A, 0x3F5, 0),
];
const DEFAULT_KEYS_B: [(i32, u32, i32); 25] = [
    (0x1E, 0x401, 1),
    (0x12, 0x41A, 0),
    (0x12, 0x41B, 1),
    (0x13, 0x419, 0),
    (0x13, 0x40E, 1),
    (0x19, 0x53, 0),
    (0x15, 0x44, 0),
    (0x14, 0x3FD, 1),
    (0x15, 0x3FE, 1),
    (0x1D, 0x414, 0),
    (0x1D, 0x48, 1),
    (0x1F, 0x20, 0),
    (0x1F, 0x41A, 1),
    (0x23, 0x45, 0),
    (0x26, 0x407, 0),
    (0x27, 0x40A, 0),
    (0x28, 0x405, 0),
    (0x29, 0x40C, 0),
    (0x28, 0x3F8, 1),
    (0x29, 0x3F6, 1),
    (0x1C, 0x3ED, 0),
    (0x1B, 0x3F6, 0),
    (0x1B, 0x52, 1),
    (0x2A, 0x5B, 0),
    (0x2B, 0x5D, 0),
];

/// `InitDefaultControlConfiguration()` (0x530640): 71 `SetControllerKeyAssociatedWithAction` calls (the
/// AZERTY / QWERTZ layouts by language 1 / 2), then the stick flags cleared (inlined
/// `ClearSimButtonPressCheckers`). The first call's key comes from the no-CD chunk at 0x409ABF:
/// `[0x15608DC] + [0x5DF174] + 1` (0x3FB = `rsUP` with the shipped bytes; read at run time as the
/// original does). Original bindings kept: PED_FIRE_WEAPON_ALT '\\', PED_CENTER_CAMERA '#', no
/// keyboard look-behind / mouse-look (gta-reversed FixBugs differs).
#[hook(addr = 0x530640, conv = thiscall, name = "CControllerConfigManager::InitDefaultControlConfiguration", expect = "53 56 33 DB 53", this = "CControllerConfigManager")]
pub fn init_default_control_configuration(this: GPtr<u8>) {
    let set = |a: i32, k: u32, t: i32| call_set_key(this, a, k, t);
    let fwd = g::<u32>(PROT_KEY_A).read().wrapping_add(g::<u32>(PROT_KEY_B).read()).wrapping_add(1);
    set(4, fwd, 0);
    set(5, 0x3FC, 0);
    set(6, 0x3FD, 0);
    set(7, 0x3FE, 0);
    let lang = || g::<i32>(LANGUAGE).read();
    if lang() == 2 {
        set(0x2F, 0x5A, 0);
        set(8, 0x59, 1);
    } else {
        set(0x2F, 0x59, 0);
        set(8, 0x58, 1);
    }
    if lang() == 1 {
        set(4, 0x5A, 1);
        set(6, 0x51, 1);
        set(3, 0x41, 0);
        set(9, 0x57, 1);
    } else {
        set(4, 0x57, 1);
        set(6, 0x41, 1);
        set(3, 0x51, 0);
        set(9, 0x5A, 1);
    }
    for (a, k, t) in DEFAULT_KEYS_A {
        set(a, k, t);
    }
    if lang() == 1 {
        set(0x14, 0x51, 0);
        set(0x22, 0x41, 0);
        set(0x1A, 0x60, 1);
        set(0x1E, 0xE9, 0);
        set(0x18, 0x5A, 0);
    } else {
        set(0x14, 0x41, 0);
        set(0x22, 0x51, 0);
        set(0x1A, 0x34, 1);
        set(0x1E, 0x32, 0);
        set(0x18, 0x57, 0);
    }
    for (a, k, t) in DEFAULT_KEYS_B {
        set(a, k, t);
    }
    clear_sim_flags(this);
}

/// `InitDefaultControlConfigJoyPad` (0x530B00) bindings (action, button), type 3; a pad with `n`
/// buttons gets the entries with button <= n, in this order (the original's fall-through switch).
/// The first list is for the joystick with vendor 0x3427 / product 0x1190.
const JOYPAD_SPECIAL: [(i32, u32); 29] = [
    (0x30, 16), (0x32, 15), (0x1B, 15), (0x2F, 14), (0x31, 13), (0x1A, 13), (0x0E, 11), (0x1E, 11), (0x1D, 10),
    (0x0F, 10), (0x0B, 9), (0x1F, 8), (0x2D, 8), (0x10, 7), (0x01, 7), (0x13, 7), (0x02, 6), (0x23, 6),
    (0x03, 5), (0x22, 5), (0x19, 4), (0x0C, 4), (0x08, 4), (0x18, 3), (0x0D, 3), (0x09, 3), (0x00, 2),
    (0x12, 2), (0x0A, 1),
];
const JOYPAD_DEFAULT: [(i32, u32); 29] = [
    (0x30, 16), (0x32, 15), (0x1B, 15), (0x2F, 14), (0x31, 13), (0x1A, 13), (0x0E, 11), (0x1E, 11), (0x1D, 10),
    (0x0F, 10), (0x0B, 9), (0x1F, 8), (0x2D, 8), (0x10, 7), (0x01, 7), (0x13, 7), (0x02, 6), (0x23, 6),
    (0x03, 5), (0x22, 5), (0x0A, 4), (0x19, 3), (0x0C, 3), (0x08, 3), (0x18, 2), (0x0D, 2), (0x09, 2),
    (0x00, 1), (0x12, 1),
];

/// `InitDefaultControlConfigJoyPad(uint32 buttons)` (0x530B00): `m_bJoyJustInitialised := 1`; buttons
/// capped at 16 (unsigned); 0 buttons bind nothing.
#[hook(addr = 0x530B00, conv = thiscall, name = "CControllerConfigManager::InitDefaultControlConfigJoyPad", expect = "8B 44 24 04 83 F8 10", this = "CControllerConfigManager")]
pub fn init_default_control_config_joy_pad(this: GPtr<u8>, buttons: u32) {
    wr::<u8>(this, 0, 1);
    let n = buttons.min(16);
    let special = g::<u32>(JOY0_VENDOR).read() == 0x3427 && g::<u32>(JOY0_PRODUCT).read() == 0x1190;
    if n == 0 {
        return;
    }
    let list: &[(i32, u32)] = if special { &JOYPAD_SPECIAL } else { &JOYPAD_DEFAULT };
    for &(a, b) in list {
        if b <= n {
            call_set_key(this, a, b, 3);
        }
    }
}

// ------------------------------------------------------------------------------------------------
// applying bindings to the pad

/// The camera mode test of the button-down paths: the active camera's mode is one of the first-person
/// modes 16, 7, 8, 51, 34, 46.
fn first_person_cam() -> bool {
    let cam = u32::from(g::<u8>(CAMERA_ACTIVE_CAM).read());
    let mode = word_at(CAMERA_CAM0_MODE.wrapping_add(cam * 0x238)).read() as u16;
    matches!(mode, 0x10 | 7 | 8 | 0x33 | 0x22 | 0x2E)
}

/// `FindPlayerVehicle(-1, false)`, `FindPlayerPed(-1)` in state 50 (driving) and the pad's controls not
/// disabled.
fn driving(pad: GPtr<u8>) -> bool {
    if find_player_vehicle(-1, 0).is_null() {
        return false;
    }
    let ped = find_player_ped(-1);
    if ped.is_null() || rd::<i32>(ped, 0x530) != 0x32 {
        return false;
    }
    rd::<u16>(pad, 0x10E) == 0
}

/// The common "button down" dispatch: driving -> Driving + Triangle check + AllStates; else
/// FirstAndThird, then first person -> FirstPersonOnly + AllStates, third person -> ThirdPersonOnly +
/// Triangle check + AllStates. `enter` is the enter/exit binding compared for the Triangle.
fn button_down(this: GPtr<u8>, k: u32, kind: i32, st: GPtr<u8>, drive: bool, fp: bool, enter: u32) {
    if drive {
        call_down_driving(this, k, kind, st);
    } else {
        call_down_first_and_third(this, k, kind, st);
        if fp {
            call_down_first(this, k, kind, st);
            call_down_all(this, k, kind, st);
            return;
        }
        call_down_third(this, k, kind, st);
    }
    if enter == k {
        st16(st, 0x1E, 0xFF);
    }
    call_down_all(this, k, kind, st);
}

/// `UpdateJoy_ButtonDown(key, type)` (0x530ED0): applies a pressed binding of `type` to the pad's PC
/// state of that type (`GetPad(0)` + 0x78 keyboard / 0xD8 mouse / 0xA8 joystick). Blank keys do
/// nothing. Original quirk kept: for a type > 3 the state pointer is the type argument's slot (low
/// byte = the "driving" flag), never reached by the game.
#[hook(addr = 0x530ED0, conv = thiscall, name = "CControllerConfigManager::UpdateJoy_ButtonDown", expect = "51 53 55 8B 6C 24 10", this = "CControllerConfigManager")]
pub fn update_joy_button_down(this: GPtr<u8>, k: u32, kind: i32) {
    let mut valid = true;
    if (kind == 0 || kind == 1) && k == RS_NULL {
        valid = false;
    }
    if (kind == 3 || kind == 2) && k == 0 {
        return;
    }
    if !valid {
        return;
    }
    let pad = get_pad(0);
    let drive = driving(pad);
    let fp = first_person_cam();
    let st = match kind {
        0 | 1 => get_pad(0).byte_add(0x78),
        2 => get_pad(0).byte_add(0xD8),
        3 => get_pad(0).byte_add(0xA8),
        _ => GPtr::from_addr(((kind as u32) & !0xFF | u32::from(drive)) as usize),
    };
    if pad.is_null() {
        return;
    }
    button_down(this, k, kind, st, drive, fp, key(this, 10, kind));
}

/// `AffectControllerStateOn_ButtonUp(key, type)` (0x531070): the state of the type (`GetPad(0)`), then,
/// for a non-blank key with a pad and the menu inactive, `..._ButtonUp_All_Player_States`. A type > 3
/// passes the type value itself as the state pointer (original quirk).
#[hook(addr = 0x531070, conv = thiscall, name = "CControllerConfigManager::AffectControllerStateOn_ButtonUp", expect = "51 53 55 8B 6C 24 10", this = "CControllerConfigManager")]
pub fn affect_button_up(this: GPtr<u8>, k: u32, kind: i32) {
    let mut valid = true;
    if (kind == 0 || kind == 1) && k == RS_NULL {
        valid = false;
    }
    if (kind == 3 || kind == 2) && k == 0 {
        valid = false;
    }
    let st = match kind {
        0 | 1 => get_pad(0).byte_add(0x78),
        2 => get_pad(0).byte_add(0xD8),
        3 => get_pad(0).byte_add(0xA8),
        _ => GPtr::from_addr(kind as u32 as usize),
    };
    if valid && !get_pad(0).is_null() && !menu_active() {
        call_up_all(this, k, kind, st);
    }
}

/// One key of `AffectPadFromKeyBoard`'s key-down pass.
fn keyboard_down(this: GPtr<u8>, k: u32, kind: i32, in_menu: bool, enter: u32) {
    if call_key_down(this, k) == 0 || !in_menu || k == RS_NULL {
        return;
    }
    let pad = get_pad(0);
    let drive = driving(pad);
    let fp = first_person_cam();
    let st = get_pad(0).byte_add(0x78);
    if pad.is_null() {
        return;
    }
    button_down(this, k, kind, st, drive, fp, enter);
}

/// The key-up clears (chat, vehicle mouse look) of a released key.
fn release(k: u32, nul: u32, st_off: isize, chat: u32, look: u32) {
    let ok = k != nul;
    let st = get_pad(0).byte_add(st_off);
    if !ok || get_pad(0).is_null() || menu_active() {
        return;
    }
    if chat == k {
        st16(st, 0x28, 0);
    }
    if look == k {
        st16(st, 0x2C, 0);
    }
}

/// `AffectPadFromKeyBoard()` (0x531140): `GTATranslateShiftKey`, then for every action: its keyboard
/// key (type 0) and extra key (type 1) when down (pad 0 and menu inactive) are applied to
/// `PCTempKeyState`; then the key-up pass: when the type-0 key is up its chat / mouse-look effects are
/// cleared and the type-1 key is not looked at; only when the type-0 key is down is the type-1 key's
/// release handled (not gated by `padNumber`; gta-reversed differs).
#[hook(addr = 0x531140, conv = thiscall, name = "CControllerConfigManager::AffectPadFromKeyBoard", expect = "83 EC 14 53 55", this = "CControllerConfigManager")]
pub fn affect_pad_from_keyboard(this: GPtr<u8>) {
    let mut ck = [0i32; 1];
    gta_translate_shift_key(GPtr::from_raw(ck.as_mut_ptr()));
    let in_menu = g::<u8>(PAD_NUMBER).read() == 0 && !menu_active();
    for a in 0..NUM_ACTIONS {
        let k0 = key(this, a, 0);
        keyboard_down(this, k0, 0, in_menu, key(this, 10, 0));
        let k1 = key(this, a, 1);
        keyboard_down(this, k1, 1, in_menu, key(this, 10, 1));
        if call_key_down(this, k0) == 0 {
            release(k0, RS_NULL, 0x78, key(this, 46, 0), key(this, 37, 0));
        } else if call_key_down(this, k1) == 0 {
            release(k1, RS_NULL, 0x78, key(this, 46, 1), key(this, 37, 1));
        }
    }
}

/// `AffectPadFromMouse()` (0x5314A0): for every action, its mouse button when down (pad 0, menu
/// inactive) is applied to `PCTempMouseState`; when up (`GetIsMouseButtonUp`) the chat / mouse-look
/// effects are cleared.
#[hook(addr = 0x5314A0, conv = thiscall, name = "CControllerConfigManager::AffectPadFromMouse", expect = "83 EC 0C A0 00 34 B7 00", this = "CControllerConfigManager")]
pub fn affect_pad_from_mouse(this: GPtr<u8>) {
    let in_menu = g::<u8>(PAD_NUMBER).read() == 0 && !menu_active();
    for a in 0..NUM_ACTIONS {
        let m = key(this, a, 2);
        if call_mouse_down(this, m) != 0 && in_menu && m != 0 {
            let pad = get_pad(0);
            let drive = driving(pad);
            let fp = first_person_cam();
            let st = get_pad(0).byte_add(0xD8);
            if !pad.is_null() {
                button_down(this, m, 2, st, drive, fp, key(this, 10, 2));
            }
        }
        if call_mouse_up(this, m) != 0 {
            release(m, 0, 0xD8, key(this, 46, 2), key(this, 37, 2));
        }
    }
}

// ------------------------------------------------------------------------------------------------
// removing clashing bindings

/// The blank test the `DeleteMatching*` functions inline; `None` for a type > 3 (they return at once).
fn del_blank(key: u32, kind: i32) -> Option<bool> {
    match kind {
        0 | 1 => Some(key == RS_NULL),
        2 | 3 => Some(key == 0),
        _ => None,
    }
}

/// Clears every listed action whose binding of `kind` is `k`, in order.
fn clear_matching(this: GPtr<u8>, k: u32, kind: i32, actions: &[i32]) {
    for &a in actions {
        if key(this, a, kind) == k {
            call_clear_settings(this, a, kind);
        }
    }
}

/// `DeleteMatchingCommonControls(action, key, type)` (0x531670): 11, 46, 55, 54, 56, 57.
#[hook(addr = 0x531670, conv = thiscall, name = "CControllerConfigManager::DeleteMatchingCommonControls", expect = "56 8B 74 24 10", this = "CControllerConfigManager")]
pub fn delete_matching_common_controls(this: GPtr<u8>, _action: i32, k: u32, kind: i32) {
    if del_blank(k, kind) != Some(false) {
        return;
    }
    clear_matching(this, k, kind, &[0x0B, 0x2E, 0x37, 0x36, 0x38, 0x39]);
}

/// `DeleteMatching3rdPersonControls(action, key, type)` (0x531730): 6, 7, 49, 50, 48, 47, 3, 2, 12, 13,
/// 14, 15; 16 when (action != 1 and control method 1) or control method 0; 17; with control method 1
/// also 44.
#[hook(addr = 0x531730, conv = thiscall, name = "CControllerConfigManager::DeleteMatching3rdPersonControls", expect = "56 8B 74 24 10", this = "CControllerConfigManager")]
pub fn delete_matching_3rd_person_controls(this: GPtr<u8>, action: i32, k: u32, kind: i32) {
    if del_blank(k, kind) != Some(false) {
        return;
    }
    clear_matching(this, k, kind, &[6, 7, 0x31, 0x32, 0x30, 0x2F, 3, 2, 0xC, 0xD, 0xE, 0xF]);
    if key(this, 0x10, kind) == k {
        let m = control_method();
        if (action != 1 && m == 1) || m == 0 {
            call_clear_settings(this, 0x10, kind);
        }
    }
    clear_matching(this, k, kind, &[0x11]);
    if control_method() == 1 {
        clear_matching(this, k, kind, &[0x2C]);
    }
}

/// `DeleteMatching1rst3rdPersonControls(action, key, type)` (0x5318C0): 1 when (action != 16 and
/// control method 1) or control method 0; 0, 45, 4, 5; with control method 1 also 32, 33, 52, 51.
#[hook(addr = 0x5318C0, conv = thiscall, name = "CControllerConfigManager::DeleteMatching1rst3rdPersonControls", expect = "56 8B 74 24 10", this = "CControllerConfigManager")]
pub fn delete_matching_1rst_3rd_person_controls(this: GPtr<u8>, action: i32, k: u32, kind: i32) {
    if del_blank(k, kind) != Some(false) {
        return;
    }
    if key(this, 1, kind) == k {
        let m = control_method();
        if (action != 0x10 && m == 1) || m == 0 {
            call_clear_settings(this, 1, kind);
        }
    }
    clear_matching(this, k, kind, &[0, 0x2D, 4, 5]);
    if control_method() == 1 {
        clear_matching(this, k, kind, &[0x20, 0x21, 0x34, 0x33]);
    }
}

/// `DeleteMatchingVehicleControls(action, key, type)` (0x5319D0): 19, 18, 20..31, 34..41.
#[hook(addr = 0x5319D0, conv = thiscall, name = "CControllerConfigManager::DeleteMatchingVehicleControls", expect = "56 8B 74 24 10", this = "CControllerConfigManager")]
pub fn delete_matching_vehicle_controls(this: GPtr<u8>, _action: i32, k: u32, kind: i32) {
    if del_blank(k, kind) != Some(false) {
        return;
    }
    clear_matching(
        this,
        k,
        kind,
        &[0x13, 0x12, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x22, 0x23, 0x24, 0x25,
            0x26, 0x27, 0x28, 0x29],
    );
}

/// `DeleteMatchingVehicle_3rdPersonControls(action, key, type)` (0x531BC0): 10.
#[hook(addr = 0x531BC0, conv = thiscall, name = "CControllerConfigManager::DeleteMatchingVehicle_3rdPersonControls", expect = "8B 44 24 0C 83 F8 03", this = "CControllerConfigManager")]
pub fn delete_matching_vehicle_3rd_person_controls(this: GPtr<u8>, _action: i32, k: u32, kind: i32) {
    if del_blank(k, kind) != Some(false) {
        return;
    }
    clear_matching(this, k, kind, &[0x0A]);
}

/// `DeleteMatching1rstPersonControls(action, key, type)` (0x531C20): 8, 9.
#[hook(addr = 0x531C20, conv = thiscall, name = "CControllerConfigManager::DeleteMatching1rstPersonControls", expect = "56 8B 74 24 10", this = "CControllerConfigManager")]
pub fn delete_matching_1rst_person_controls(this: GPtr<u8>, _action: i32, k: u32, kind: i32) {
    if del_blank(k, kind) != Some(false) {
        return;
    }
    clear_matching(this, k, kind, &[8, 9]);
}

/// `DeleteMatchingActionInitiators(action, key, type)` (0x531C90): by `GetActionType(action)` the
/// matching groups' `DeleteMatching*` calls (in the original's order); type 6 / out of range nothing.
#[hook(addr = 0x531C90, conv = thiscall, name = "CControllerConfigManager::DeleteMatchingActionInitiators", expect = "55 8B 6C 24 10", this = "CControllerConfigManager")]
pub fn delete_matching_action_initiators(this: GPtr<u8>, action: i32, k: u32, kind: i32) {
    if del_blank(k, kind) != Some(false) {
        return;
    }
    let (a, t) = (action, kind);
    match call_get_action_type(this, action) {
        0 => {
            call_del_common(this, a, k, t);
            call_del_1rst(this, a, k, t);
            call_del_1rst3rd(this, a, k, t);
        }
        1 => {
            call_del_common(this, a, k, t);
            call_del_1rst3rd(this, a, k, t);
            call_del_3rd(this, a, k, t);
            call_del_vehicle_3rd(this, a, k, t);
        }
        2 => {
            call_del_common(this, a, k, t);
            call_del_vehicle(this, a, k, t);
            call_del_vehicle_3rd(this, a, k, t);
        }
        3 => {
            call_del_common(this, a, k, t);
            call_del_1rst3rd(this, a, k, t);
            call_del_3rd(this, a, k, t);
            call_del_vehicle(this, a, k, t);
        }
        4 => {
            call_del_common(this, a, k, t);
            call_del_1rst(this, a, k, t);
            call_del_1rst3rd(this, a, k, t);
            call_del_3rd(this, a, k, t);
            call_del_vehicle(this, a, k, t);
            call_del_vehicle_3rd(this, a, k, t);
        }
        5 => {
            call_del_common(this, a, k, t);
            call_del_1rst(this, a, k, t);
            call_del_1rst3rd(this, a, k, t);
            call_del_3rd(this, a, k, t);
            call_del_vehicle_3rd(this, a, k, t);
        }
        _ => {}
    }
}

/// `GetControllerSettingTextWithOrderNumber(action, order)` (0x531E20): the text of the first type whose
/// set order is `order` (keyboard / extra key / mouse / joystick), null when none.
#[hook(addr = 0x531E20, conv = thiscall, name = "CControllerConfigManager::GetControllerSettingTextWithOrderNumber", expect = "56 8B 74 24 08", this = "CControllerConfigManager")]
pub fn get_controller_setting_text_with_order_number(this: GPtr<u8>, action: i32, ord: u32) -> GPtr<u8> {
    for t in 0..4 {
        if order(this, action, t) == ord {
            return match t {
                0 | 1 => call_text_keyboard(this, action, t),
                2 => call_text_mouse(this, action),
                _ => call_text_joystick(this, action),
            };
        }
    }
    GPtr::NULL
}

/// `GetFirstKeyboardControllerText(action)` (0x531E90): `ResetSettingOrder(action)`, then the keyboard
/// text of the first of types 0/1 with a non-zero set order; null when neither.
#[hook(addr = 0x531E90, conv = thiscall, name = "CControllerConfigManager::GetFirstKeyboardControllerText", expect = "53 8B 5C 24 08", this = "CControllerConfigManager")]
pub fn get_first_keyboard_controller_text(this: GPtr<u8>, action: i32) -> GPtr<u8> {
    call_reset_setting_order(this, action);
    for t in 0..2 {
        if order(this, action, t) != 0 {
            return call_text_keyboard(this, action, t);
        }
    }
    GPtr::NULL
}

/// `CControllerConfigManager::CControllerConfigManager()` (0x531EE0): zeroes `m_Actions`,
/// `m_bJoyJustInitialised` and `m_MouseFoundInitSet`, then `MakeControllerActionsBlank`,
/// `InitDefaultControlConfiguration`, `InitialiseControllerActionNameArray`; returns `this`.
#[hook(addr = 0x531EE0, conv = thiscall, name = "CControllerConfigManager::Constructor", expect = "56 8B F1 8D 86 70 0B 00 00", this = "CControllerConfigManager")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    for i in 0..(0xEC * 2) {
        wr::<u32>(this, 0xB70 + 4 * i, 0);
    }
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 0x12E0, 0);
    call_make_blank(this);
    call_init_default(this);
    call_init_names(this);
    this
}

/// `ReinitControls()` (0x531F20, `ControlsManager`): clears every binding (`ClearSettingsAssociatedWithAction`
/// per type and action), `InitDefaultControlConfiguration`, the mouse bindings from
/// `GetMouseSetUp()`; with a joystick attached `diDevice1->GetCapabilities` (result ignored) and
/// `InitDefaultControlConfigJoyPad(caps.dwButtons)`. Original UB: when `GetCapabilities` fails,
/// `dwButtons` is uninitialised stack; the port passes 0 then.
#[hook(addr = 0x531F20, conv = cdecl, name = "CControllerConfigManager::ReinitControls", expect = "83 EC 40 56 57")]
pub fn reinit_controls() {
    let cm: GPtr<u8> = GPtr::from_addr(CONTROLS_MANAGER);
    for t in 0..4 {
        for a in 0..NUM_ACTIONS {
            call_clear_settings(cm, a, t);
        }
    }
    call_init_default(cm);
    let mouse_controls = u32::from(control_method() == 0);
    let mut tmp = [0u8; 0x14];
    let r = get_mouse_set_up(GPtr::from_raw(tmp.as_mut_ptr()));
    call_init_mouse(cm, r, mouse_controls);
    if g::<u8>(JOY0_ATTACHED).read() != 0 {
        let mut caps = [0u32; 11];
        caps[0] = 0x2C;
        let dev = rd::<GPtr<u8>>(g::<GPtr<u8>>(RS_PS).read(), 0x20);
        let _ = did_get_capabilities(dev, GPtr::from_raw(caps.as_mut_ptr()).cast());
        call_init_joypad(cm, caps[4]);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SAVE_SETTINGS,
    HOOK_INITIALISE_CONTROLLER_ACTION_NAME_ARRAY,
    HOOK_STORE_MOUSE_BUTTON_STATE,
    HOOK_UPDATE_JOY_IN_CONFIG_MENUS_BUTTON_DOWN,
    HOOK_UPDATE_JOY_IN_CONFIG_MENUS_BUTTON_UP,
    HOOK_CLEAR_SIM_BUTTON_PRESS_CHECKERS,
    HOOK_GET_IS_KEYBOARD_KEY_DOWN,
    HOOK_GET_IS_KEYBOARD_KEY_JUST_DOWN,
    HOOK_GET_IS_MOUSE_BUTTON_DOWN,
    HOOK_GET_IS_MOUSE_BUTTON_UP,
    HOOK_GET_IS_MOUSE_BUTTON_JUST_UP,
    HOOK_GET_IS_KEY_BLANK,
    HOOK_GET_ACTION_TYPE,
    HOOK_GET_CONTROLLER_SETTING_TEXT_MOUSE,
    HOOK_GET_CONTROLLER_SETTING_TEXT_JOYSTICK,
    HOOK_GET_NUM_OF_SETTINGS_FOR_ACTION,
    HOOK_GET_CONTROLLER_KEY_ASSOCIATED_WITH_ACTION,
    HOOK_UPDATE_JOY_BUTTON_STATE,
    HOOK_GET_IS_ACTION_A_BUTTON_COMBO,
    HOOK_GET_BUTTON_COMBO_TEXT,
    HOOK_GET_MOUSE_BUTTON_ASSOCIATED_WITH_ACTION,
    HOOK_SET_MOUSE_BUTTON_ASSOCIATED_WITH_ACTION,
    HOOK_RESET_SETTING_ORDER,
    HOOK_INIT_DEFAULT_CONTROL_CONFIG_MOUSE,
    HOOK_AFFECT_BUTTON_DOWN_DRIVING,
    HOOK_AFFECT_BUTTON_DOWN_FIRST_PERSON_ONLY,
    HOOK_AFFECT_BUTTON_DOWN_THIRD_PERSON_ONLY,
    HOOK_AFFECT_BUTTON_DOWN_FIRST_AND_THIRD_PERSON_ONLY,
    HOOK_AFFECT_BUTTON_DOWN_ALL_STATES,
    HOOK_AFFECT_BUTTON_DOWN_VEHICLE_AND_THIRD_PERSON_ONLY,
    HOOK_AFFECT_BUTTON_UP_ALL_PLAYER_STATES,
    HOOK_CLEAR_SETTINGS_ASSOCIATED_WITH_ACTION,
    HOOK_GET_CONTROLLER_SETTING_TEXT_KEYBOARD,
    HOOK_GET_GXT_STRING_OF_COMMAND_KEYS,
    HOOK_SET_CONTROLLER_KEY_ASSOCIATED_WITH_ACTION,
    HOOK_MAKE_CONTROLLER_ACTIONS_BLANK,
    HOOK_LOAD_SETTINGS,
    HOOK_INIT_DEFAULT_CONTROL_CONFIGURATION,
    HOOK_INIT_DEFAULT_CONTROL_CONFIG_JOY_PAD,
    HOOK_UPDATE_JOY_BUTTON_DOWN,
    HOOK_AFFECT_BUTTON_UP,
    HOOK_AFFECT_PAD_FROM_KEYBOARD,
    HOOK_AFFECT_PAD_FROM_MOUSE,
    HOOK_DELETE_MATCHING_COMMON_CONTROLS,
    HOOK_DELETE_MATCHING_3RD_PERSON_CONTROLS,
    HOOK_DELETE_MATCHING_1RST_3RD_PERSON_CONTROLS,
    HOOK_DELETE_MATCHING_VEHICLE_CONTROLS,
    HOOK_DELETE_MATCHING_VEHICLE_3RD_PERSON_CONTROLS,
    HOOK_DELETE_MATCHING_1RST_PERSON_CONTROLS,
    HOOK_DELETE_MATCHING_ACTION_INITIATORS,
    HOOK_GET_CONTROLLER_SETTING_TEXT_WITH_ORDER_NUMBER,
    HOOK_GET_FIRST_KEYBOARD_CONTROLLER_TEXT,
    HOOK_CONSTRUCTOR,
    HOOK_REINIT_CONTROLS,
];
