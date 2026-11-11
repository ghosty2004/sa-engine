//! INPUT: `CPad` and its state classes (`CControllerState`, `CKeyboardState`,
//! `CMouseControllerState`) (PORT-ANIM-INPUT). Ported from the 1.0 US bytes; gta-reversed `Pad.cpp`
//! / `Pad.h` as a reading aid, `docs/re/specs/INPUT/CPad*.md` for the REA-only rows and the
//! deviations. See `docs/arch/anim-input-port.md`.
//!
//! Layout (`VALIDATE_SIZE(CPad, 0x134)`): +0x00 `NewState` (`CControllerState`, 0x30 bytes of int16),
//! +0x30 `OldState`, +0x60 `SteeringLeftRightBuffer[10]`, +0x74 `DrunkDrivingBufferUsed`, +0x78
//! `PCTempKeyState`, +0xA8 `PCTempJoyState`, +0xD8 `PCTempMouseState`, +0x108 `Phase`, +0x10A `Mode`,
//! +0x10C `ShakeDur`, +0x10E `DisablePlayerControls`, +0x110 `ShakeFreq`, +0x111 `iCurrHornHistory`
//! flags[5], +0x116 history index, +0x117 `JustOutOfFrontEnd`, +0x118 `bApplyBrakes`, +0x119..+0x11F
//! the `DisableControls` bytes (exit vehicle, duck, weapon, ..., jump, vital stats), +0x120
//! `LastTimeTouched`, +0x124/+0x128, +0x12C shake end time, +0x130 shake strength.
//! `CPad::Pads` = 0xB73458.
//!
//! Return registers: a getter that sets the whole EAX on every path (`mov eax, 1` / `xor eax, eax`,
//! `movzx`/`movsx`) returns `u32`; one with a path that only sets AL returns `u8` (L2 compares AL);
//! one that writes AX while the upper half still holds the caller's EAX is `#[hook(entry_eax)]` and
//! builds EAX with [`sa_core::abi::with_entry_hi`].

use sa_core::abi::{HookReg, with_entry_hi};
use sa_core::hook;
use sa_core::input::{ALL_VALID_WIN_JOYS, CMouseControllerState, ComObj, NEW_KEY_STATE, OLD_KEY_STATE, PAD_NUMBER, TEMP_KEY_STATE};
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_core::timer::TIME_MS;
use sa_math::x87::{Cmp, X87};

use crate::common::{ftol32, g, rd, wr, x, xi};

/// `CPad::Pads` (`CPad[2]`, stride 0x134).
pub const PADS: usize = 0xB73458;
/// `sizeof(CPad)`.
pub const PAD_SIZE: u32 = 0x134;

// CPad field offsets
pub const NEW_STATE: isize = 0x00;
pub const OLD_STATE: isize = 0x30;
pub const STEERING_BUFFER: isize = 0x60;
pub const DRUNK_BUFFER_USED: isize = 0x74;
pub const PC_TEMP_KEY_STATE: isize = 0x78;
pub const PC_TEMP_JOY_STATE: isize = 0xA8;
pub const PC_TEMP_MOUSE_STATE: isize = 0xD8;
pub const PHASE: isize = 0x108;
pub const MODE: isize = 0x10A;
pub const SHAKE_DUR: isize = 0x10C;
pub const DISABLE_PLAYER_CONTROLS: isize = 0x10E;
pub const SHAKE_FREQ: isize = 0x110;
pub const LAST_TIME_TOUCHED: isize = 0x120;

// CControllerState field offsets (int16 each)
pub const LEFT_STICK_X: isize = 0x00;
pub const LEFT_STICK_Y: isize = 0x02;
pub const RIGHT_STICK_X: isize = 0x04;
pub const RIGHT_STICK_Y: isize = 0x06;
pub const LEFT_SHOULDER1: isize = 0x08;
pub const LEFT_SHOULDER2: isize = 0x0A;
pub const RIGHT_SHOULDER1: isize = 0x0C;
pub const RIGHT_SHOULDER2: isize = 0x0E;
pub const DPAD_UP: isize = 0x10;
pub const DPAD_DOWN: isize = 0x12;
pub const DPAD_LEFT: isize = 0x14;
pub const DPAD_RIGHT: isize = 0x16;
pub const START: isize = 0x18;
pub const SELECT: isize = 0x1A;
pub const BUTTON_SQUARE: isize = 0x1C;
pub const BUTTON_TRIANGLE: isize = 0x1E;
pub const BUTTON_CROSS: isize = 0x20;
pub const BUTTON_CIRCLE: isize = 0x22;
pub const SHOCK_BUTTON_L: isize = 0x24;
pub const SHOCK_BUTTON_R: isize = 0x26;

sa_core::globals! {
    list GLOBALS;
    /// `CPad::Pads` (both pads, read by the static `GetAnaloguePad*` queries)
    pub static PADS_ARRAY @ 0xB73458: [u8; 0x268] = [0; 0x268];
    /// `byte_B73401` (cleared by `CPad::Initialise`, otherwise unused)
    pub static BYTE_B73401 @ 0xB73401: u8 = 0;
    /// `CPad::bInvertLook4Pad`
    pub static INVERT_LOOK_4_PAD @ 0xB73402: u8 = 0;
    /// `byte_B73403` (never written by the game code)
    pub static BYTE_B73403 @ 0xB73403: u8 = 0;
    /// `CPad::TempMouseControllerState`
    pub static TEMP_MOUSE @ 0xB73404: CMouseControllerState = MOUSE0;
    /// `CPad::NewMouseControllerState`
    pub static NEW_MOUSE @ 0xB73418: CMouseControllerState = MOUSE0;
    /// `CPad::OldMouseControllerState`
    pub static OLD_MOUSE @ 0xB7342C: CMouseControllerState = MOUSE0;
    /// `ReconcileTwoControllersInput`'s static result (`CControllerState`)
    pub static RECONCILED @ 0xB736C0: [i16; 24] = [0; 24];
    /// `GetAnaloguePadUp/Down/Left/Right`'s previous stick values (int16 in dword slots)
    pub static ANALOGUE_OLD_UP @ 0xB736F0: i16 = 0;
    pub static ANALOGUE_OLD_DOWN @ 0xB736F4: i16 = 0;
    pub static ANALOGUE_OLD_LEFT @ 0xB736F8: i16 = 0;
    pub static ANALOGUE_OLD_RIGHT @ 0xB736FC: i16 = 0;
    /// `byte_8CD782` (1 in .data, never written): left stick walks, right stick looks
    pub static BYTE_8CD782 @ 0x8CD782: u8 = 1;
    // foreign (owner: CCamera), declared here so the hosted L1 specs can set it
    /// `CCamera::m_bUseMouse3rdPerson`
    pub static USE_MOUSE_3RD_PERSON @ 0xB6EC2E: u8 = 0;
}

const MOUSE0: CMouseControllerState =
    CMouseControllerState { lmb: 0, rmb: 0, mmb: 0, wheel_up: 0, wheel_down: 0, bmx1: 0, bmx2: 0, pad: 0, z: 0.0, x: 0.0, y: 0.0 };

/// `ControlsManager` (`CControllerConfigManager`, 0xB70198).
pub const CONTROLS_MANAGER: usize = 0xB70198;
/// `FrontEndMenuManager.m_PrefsUseVibration`
const PREFS_USE_VIBRATION: usize = 0xBA6768;
/// `CCutsceneMgr::ms_running`
const CUTSCENE_RUNNING: usize = 0xB5F851;
/// `TheCamera.m_pMat` (the camera matrix pointer) and `TheCamera.placement.pos`
const CAMERA_MATRIX: usize = 0xB6F03C;
const CAMERA_PLACEMENT_POS: usize = 0xB6F02C;
/// `FrontEndMenuManager.m_bMenuActive`
const MENU_ACTIVE: usize = 0xBA67A4;
/// `FrontEndMenuManager.bInvertMouseX` / `bInvertMouseY`
const INVERT_MOUSE_X: usize = 0xBA6744;
const INVERT_MOUSE_Y: usize = 0xBA6745;
/// `PsGlobal.diMouse` (`RsGlobal.ps` +0x1C)
const DI_MOUSE: usize = 0xC8CFA4;
/// `PsGlobal` joystick axes (+0x28 .. +0x34): right X, right Y, left X, left Y
const JOY_RX: usize = 0xC8CFB0;
const JOY_RY: usize = 0xC8CFB4;
const JOY_LX: usize = 0xC8CFB8;
const JOY_LY: usize = 0xC8CFBC;
/// `RsGlobal.ps`
const RS_PS: usize = 0xC17054;

orig! {
    /// `CControllerState::Clear` (0x53EF80)
    pub fn call_controller_state_clear(s: GPtr<u8>) = thiscall 0x53EF80 => controller_state_clear;
    /// `CControllerState::CheckForInput` (0x53EFF0, AL)
    pub fn call_controller_state_check_for_input(s: GPtr<u8>) -> u8 = thiscall 0x53EFF0 => controller_state_check_for_input;
    /// `CKeyboardState::Clear` (0x53F090)
    pub fn call_keyboard_state_clear(s: GPtr<u8>) = thiscall 0x53F090 => keyboard_state_clear;
    /// `CMouseControllerState::CheckForInput` (0x53F270, AL)
    pub fn call_mouse_check_for_input(s: GPtr<u8>) -> u8 = thiscall 0x53F270 => mouse_check_for_input;
    /// `CPad::IsLeftShoulder1Pressed` (0x53EDC0)
    pub fn call_is_left_shoulder1_pressed(p: GPtr<u8>) -> u32 = thiscall 0x53EDC0 => is_left_shoulder1_pressed;
    /// `CPad::IsRightShoulder1Pressed` (0x53EE20)
    pub fn call_is_right_shoulder1_pressed(p: GPtr<u8>) -> u32 = thiscall 0x53EE20 => is_right_shoulder1_pressed;
    /// `CPad::IsTrianglePressed` (0x53EF40)
    pub fn call_is_triangle_pressed(p: GPtr<u8>) -> u32 = thiscall 0x53EF40 => is_triangle_pressed;
    /// `CPad::IsCirclePressed` (0x53EF60)
    pub fn call_is_circle_pressed(p: GPtr<u8>) -> u32 = thiscall 0x53EF60 => is_circle_pressed;
    /// `CPad::IsCrossPressed` (0x4D59E0)
    pub fn call_is_cross_pressed(p: GPtr<u8>) -> u32 = thiscall 0x4D59E0 => is_cross_pressed;
    /// `CPad::ReconcileTwoControllersInput` (0x53F530)
    pub fn call_reconcile(p: GPtr<u8>, out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = thiscall 0x53F530 => reconcile_two_controllers_input;
    /// `CPad::Clear(bool enablePlayerControls, bool resetPhase)` (0x541A70)
    pub fn call_pad_clear(p: GPtr<u8>, enable: u32, phase: u32) = thiscall 0x541A70 => clear;
    /// `CPad::Update(int16 padNumber)` (0x541C40)
    pub fn call_pad_update(p: GPtr<u8>, n: u32) = thiscall 0x541C40 => update;
    /// `CPad::UpdateMouse` (0x53F3C0)
    pub fn call_update_mouse(p: GPtr<u8>) = thiscall 0x53F3C0 => update_mouse;
    /// `CPad::ProcessPad(ePadID)` (0x746A10)
    pub fn call_process_pad(id: i32) -> u32 = cdecl 0x746A10 => process_pad;
    /// `CPad::GetMouseState(DIMOUSESTATE2*)` (0x746ED0)
    pub fn call_get_mouse_state(out: GPtr<u8>) -> i32 = cdecl 0x746ED0 => get_mouse_state;
    /// `CControllerConfigManager::ClearSimButtonPressCheckers` (0x52DD90)
    pub fn ccm_clear_sim_button_press_checkers(m: GPtr<u8>) = thiscall 0x52DD90 => unported;
    /// `CControllerConfigManager::AffectPadFromKeyBoard` (0x531140)
    pub fn ccm_affect_pad_from_keyboard(m: GPtr<u8>) = thiscall 0x531140 => unported;
    /// `CControllerConfigManager::AffectPadFromMouse` (0x5314A0)
    pub fn ccm_affect_pad_from_mouse(m: GPtr<u8>) = thiscall 0x5314A0 => unported;
    /// `CPedIntelligence::GetTaskUseGun` (0x600F70)
    pub fn get_task_use_gun(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CTaskManager::GetSimplestActiveTask` (0x6819D0)
    pub fn get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    pub fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `Win::IsForegroundApp` (0x746060, AL)
    pub fn is_foreground_app() -> u8 = cdecl 0x746060 => unported;
    /// `Input::diMouseInit(bool exclusive)` (0x7469A0)
    pub fn di_mouse_init(exclusive: u32) -> i32 = cdecl 0x7469A0 => unported;
    /// `VideoMode::IsVideoModeExclusive` (0x745CA0, AL)
    pub fn is_video_mode_exclusive() -> u8 = cdecl 0x745CA0 => unported;
    /// `Rs::RsPadEventHandler(event, param)` (0x6195A0)
    pub fn rs_pad_event_handler(event: i32, param: GPtr<u32>) -> i32 = cdecl 0x6195A0 => unported;
    /// `IDirectInputDevice8A::Acquire()`
    pub fn did_acquire(this: GPtr<ComObj>) -> i32 = stdcall vslot 0x1C => unported;
    /// `IDirectInputDevice8A::GetDeviceState(size, data)`
    pub fn did_get_device_state(this: GPtr<ComObj>, size: u32, data: GPtr<u8>) -> i32 = stdcall vslot 0x24 => unported;
    /// `IDirectInputDevice8A::Poll()`
    pub fn did_poll(this: GPtr<ComObj>) -> i32 = stdcall vslot 0x64 => unported;
}

/// `DIERR_INPUTLOST`
const DIERR_INPUTLOST: i32 = 0x8007_001E_u32 as i32;

#[inline]
fn nz(p: GPtr<u8>, off: isize) -> bool {
    rd::<i16>(p, off) != 0
}

#[inline]
fn disabled(this: GPtr<u8>) -> bool {
    rd::<u16>(this, DISABLE_PLAYER_CONTROLS) != 0
}

#[inline]
fn mode(this: GPtr<u8>) -> u32 {
    u32::from(rd::<u16>(this, MODE))
}

#[inline]
fn byte_set(this: GPtr<u8>, off: isize) -> bool {
    rd::<u8>(this, off) != 0
}

/// `NewState.f != 0 && OldState.f == 0` as the "Pressed" helpers compute it (whole EAX = 1 / 0).
#[inline]
fn pressed(this: GPtr<u8>, f: isize) -> u32 {
    u32::from(rd::<i16>(this, NEW_STATE + f) != 0 && rd::<i16>(this, OLD_STATE + f) == 0)
}

/// Zero-extended int16 field (AX after a `movzx eax, Mode`: upper half 0).
#[inline]
fn zx(this: GPtr<u8>, off: isize) -> u32 {
    u32::from(rd::<u16>(this, off))
}

/// Sign-extended int16 field (`movsx eax, word`).
#[inline]
fn sx(this: GPtr<u8>, off: isize) -> i32 {
    i32::from(rd::<i16>(this, off))
}

// ------------------------------------------------------------------------ pressed helpers ---

/// `CPad::GetPad(int32 padNumber)` (0x53FB70): `&Pads[n]`, `imul` 32-bit, no bounds check.
#[hook(addr = 0x53FB70, conv = cdecl, name = "CPad::GetPad", expect = "8B 44 24 04 69 C0 34 01 00 00")]
pub fn get_pad(n: i32) -> GPtr<u8> {
    GPtr::from_addr(n.wrapping_mul(PAD_SIZE as i32).wrapping_add(PADS as i32) as u32 as usize)
}

/// `CPad::IsCrossPressed` (0x4D59E0): `NewState.ButtonCross != 0 && OldState.ButtonCross == 0`.
#[hook(addr = 0x4D59E0, conv = thiscall, name = "CPad::IsCrossPressed", expect = "66 83 79 20 00", this = "CPad")]
pub fn is_cross_pressed(this: GPtr<u8>) -> u32 {
    pressed(this, BUTTON_CROSS)
}

/// `CPad::IsLeftShoulder1Pressed` (0x53EDC0).
#[hook(addr = 0x53EDC0, conv = thiscall, name = "CPad::IsLeftShoulder1Pressed", expect = "66 83 79 08 00", this = "CPad")]
pub fn is_left_shoulder1_pressed(this: GPtr<u8>) -> u32 {
    pressed(this, LEFT_SHOULDER1)
}

/// `CPad::IsRightShoulder1Pressed` (0x53EE20).
#[hook(addr = 0x53EE20, conv = thiscall, name = "CPad::IsRightShoulder1Pressed", expect = "66 83 79 0C 00", this = "CPad")]
pub fn is_right_shoulder1_pressed(this: GPtr<u8>) -> u32 {
    pressed(this, RIGHT_SHOULDER1)
}

/// `CPad::IsTrianglePressed` (0x53EF40).
#[hook(addr = 0x53EF40, conv = thiscall, name = "CPad::IsTrianglePressed", expect = "66 83 79 1E 00", this = "CPad")]
pub fn is_triangle_pressed(this: GPtr<u8>) -> u32 {
    pressed(this, BUTTON_TRIANGLE)
}

/// `CPad::IsCirclePressed` (0x53EF60).
#[hook(addr = 0x53EF60, conv = thiscall, name = "CPad::IsCirclePressed", expect = "66 83 79 22 00", this = "CPad")]
pub fn is_circle_pressed(this: GPtr<u8>) -> u32 {
    pressed(this, BUTTON_CIRCLE)
}

/// `CPad::~CPad` (0x53ED60): a lone `ret` (not hookable).
pub fn destructor(_this: GPtr<u8>) {}

// ------------------------------------------------------------------------- state classes ---

/// `CControllerState::Clear` (0x53EF80): all 24 int16 fields = 0.
#[hook(addr = 0x53EF80, conv = thiscall, name = "CControllerState::Clear", expect = "33 C0 66 89 41 06", this = "CControllerState")]
pub fn controller_state_clear(this: GPtr<u8>) {
    for k in 0..24 {
        wr::<i16>(this, 2 * k, 0);
    }
}

/// `CControllerState::CheckForInput` (0x53EFF0, AL): any stick, shoulder, d-pad or button field
/// (not the four PC flags at +0x28..+0x2E) non-zero.
#[hook(addr = 0x53EFF0, conv = thiscall, name = "CControllerState::CheckForInput", expect = "66 83 79 04 00", this = "CControllerState")]
pub fn controller_state_check_for_input(this: GPtr<u8>) -> u8 {
    const F: [isize; 20] =
        [4, 6, 0, 2, 0x10, 0x12, 0x14, 0x16, 0x1E, 0x20, 0x22, 0x1C, 0x18, 0x1A, 8, 0xA, 0xC, 0xE, 0x24, 0x26];
    u8::from(F.iter().any(|&f| nz(this, f)))
}

/// The named-key words `CKeyboardState::Clear` writes (+0x218..+0x26E): every one except +0x23C
/// (`enter`), which the original never clears (kept).
const KEYBOARD_CLEARED: [isize; 43] = [
    0x224, 0x222, 0x220, 0x21E, 0x21C, 0x21A, 0x218, 0x22C, 0x22A, 0x228, 0x226, 0x232, 0x23A, 0x238, 0x236,
    0x234, 0x246, 0x244, 0x242, 0x240, 0x23E, 0x24E, 0x24C, 0x24A, 0x248, 0x230, 0x22E, 0x252, 0x250, 0x25A,
    0x258, 0x256, 0x254, 0x268, 0x266, 0x264, 0x262, 0x25E, 0x260, 0x25C, 0x26E, 0x26C, 0x26A,
];

/// `CKeyboardState::Clear` (0x53F090): FKeys (+0, 0x18 bytes) and standardKeys (+0x18, 0x200 bytes)
/// zeroed, then the named keys in [`KEYBOARD_CLEARED`]. Original bug kept: `enter` (+0x23C) is not
/// cleared.
#[hook(addr = 0x53F090, conv = thiscall, name = "CKeyboardState::Clear", expect = "8B D1 33 C0 57", this = "CKeyboardState")]
pub fn keyboard_state_clear(this: GPtr<u8>) {
    for k in 0..(0x218 / 4) {
        wr::<u32>(this, 4 * k, 0);
    }
    for off in KEYBOARD_CLEARED {
        wr::<i16>(this, off, 0);
    }
}

/// `CPad::ClearKeyBoardHistory` (0x53F1E0, static): `NewKeyState`, `OldKeyState`, `TempKeyState`
/// cleared, in that order.
#[hook(addr = 0x53F1E0, conv = cdecl, name = "CPad::ClearKeyBoardHistory", expect = "B9 90 31 B7 00")]
pub fn clear_keyboard_history() {
    call_keyboard_state_clear(NEW_KEY_STATE.ptr().cast());
    call_keyboard_state_clear(OLD_KEY_STATE.ptr().cast());
    call_keyboard_state_clear(TEMP_KEY_STATE.ptr().cast());
}

/// `CPad::SetTouched` (0x53F200): `LastTimeTouched = CTimer::m_snTimeInMilliseconds`.
#[hook(addr = 0x53F200, conv = thiscall, name = "CPad::SetTouched", expect = "A1 84 CB B7 00", this = "CPad")]
pub fn set_touched(this: GPtr<u8>) {
    wr::<u32>(this, LAST_TIME_TOUCHED, TIME_MS.get());
}

/// `CPad::GetTouchedTimeDelta` (0x53F210): `m_snTimeInMilliseconds - LastTimeTouched` (wrapping).
#[hook(addr = 0x53F210, conv = thiscall, name = "CPad::GetTouchedTimeDelta", expect = "A1 84 CB B7 00", this = "CPad")]
pub fn get_touched_time_delta(this: GPtr<u8>) -> u32 {
    TIME_MS.get().wrapping_sub(rd::<u32>(this, LAST_TIME_TOUCHED))
}

/// `CMouseControllerState::CMouseControllerState` (0x53F220): bytes +0..+6 and the three floats
/// zeroed (byte +7 untouched); returns `this`.
#[hook(
    addr = 0x53F220,
    conv = thiscall,
    name = "CMouseControllerState::Constructor",
    expect = "8B C1 33 C9 88 08",
    this = "CMouseControllerState"
)]
pub fn mouse_state_constructor(this: GPtr<u8>) -> GPtr<u8> {
    for k in 0..5 {
        wr::<u8>(this, k, 0);
    }
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 0x10, 0);
    wr::<u8>(this, 5, 0);
    wr::<u8>(this, 6, 0);
    this
}

/// `CMouseControllerState::Clear` (0x53F250): the seven button bytes = 0 (floats kept).
#[hook(
    addr = 0x53F250,
    conv = thiscall,
    name = "CMouseControllerState::Clear",
    expect = "32 C0 88 01 88 41 01",
    this = "CMouseControllerState"
)]
pub fn mouse_state_clear(this: GPtr<u8>) {
    for k in 0..7 {
        wr::<u8>(this, k, 0);
    }
}

/// `CMouseControllerState::CheckForInput` (0x53F270, AL): any button byte set, or `x` / `y` (+0xC,
/// +0x10) not equal to 0.0 (NaN counts as input; the wheel float +8 is not tested).
#[hook(
    addr = 0x53F270,
    conv = thiscall,
    name = "CMouseControllerState::CheckForInput",
    expect = "80 39 00 75 4D",
    this = "CMouseControllerState"
)]
pub fn mouse_check_for_input(this: GPtr<u8>) -> u8 {
    if (0..7).any(|k| rd::<u8>(this, k) != 0) {
        return 1;
    }
    let ne0 = |off| x(rd::<f32>(this, off)).fcom(X87::zero()) != Cmp::Equal;
    u8::from(ne0(0xC) || ne0(0x10))
}

/// `CPad::ReconcileTwoControllersInput(CControllerState* out, const CControllerState& a,
/// const CControllerState& b)` (0x53F530, `ret 0xC`, `this` unused): merges two controller states
/// into the static `RECONCILED` (0xB736C0), copies it to `out` and returns `out`.
///
/// Buttons: 0xFF where either is set. Sticks: both >= 0 → max, both <= 0 → min, opposite signs → 0
/// (the right stick's min/max are written field by field as in the original). D-pad: 0xFF where
/// either is set; then up+down (or one of them against the left stick Y's sign) cancel both and the
/// left stick Y, left/right the same with the left stick X.
#[hook(
    addr = 0x53F530,
    conv = thiscall,
    name = "CPad::ReconcileTwoControllersInput",
    expect = "55 56 57 B9 C0 36 B7 00"
)]
pub fn reconcile_two_controllers_input(_this: GPtr<u8>, out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> {
    let s: GPtr<u8> = RECONCILED.ptr().cast();
    call_controller_state_clear(s);
    for f in [8isize, 0xA, 0xC, 0xE, 0x18, 0x1A, 0x1C, 0x1E, 0x20, 0x22, 0x24, 0x26, 0x28, 0x2A, 0x2E, 0x2C] {
        if nz(a, f) || nz(b, f) {
            wr::<i16>(s, f, 0xFF);
        }
    }
    // left stick X / Y: the decided value stays in a register (bp / bx) for the d-pad checks below
    let left = |off: isize| -> i16 {
        let mut v: i16 = rd(s, off);
        let (ca, cb) = (rd::<i16>(a, off), rd::<i16>(b, off));
        if ca >= 0 && cb >= 0 {
            v = if ca > cb { ca } else { cb };
            wr(s, off, v);
        }
        let (ca, cb) = (rd::<i16>(a, off), rd::<i16>(b, off));
        if ca <= 0 && cb <= 0 {
            v = if ca < cb { ca } else { cb };
            wr(s, off, v);
        }
        v
    };
    let mut bp = left(LEFT_STICK_X);
    let mut bx = left(LEFT_STICK_Y);
    let opposite = |off: isize| {
        let (ca, cb) = (rd::<i16>(a, off), rd::<i16>(b, off));
        (ca > 0 && cb < 0) || (ca < 0 && cb > 0)
    };
    if opposite(LEFT_STICK_X) {
        bp = 0;
        wr::<i16>(s, LEFT_STICK_X, 0);
    }
    if opposite(LEFT_STICK_Y) {
        bx = 0;
        wr::<i16>(s, LEFT_STICK_Y, 0);
    }
    for off in [RIGHT_STICK_X, RIGHT_STICK_Y] {
        let (ca, cb) = (rd::<i16>(a, off), rd::<i16>(b, off));
        if ca >= 0 && cb >= 0 {
            wr(s, off, ca);
            if ca <= cb {
                wr(s, off, cb);
            }
        }
        let (ca, cb) = (rd::<i16>(a, off), rd::<i16>(b, off));
        if ca <= 0 && cb <= 0 {
            wr(s, off, ca);
            if ca >= cb {
                wr(s, off, cb);
            }
        }
    }
    for off in [RIGHT_STICK_X, RIGHT_STICK_Y] {
        if opposite(off) {
            wr::<i16>(s, off, 0);
        }
    }
    for f in [DPAD_UP, DPAD_DOWN, DPAD_LEFT, DPAD_RIGHT] {
        if nz(a, f) || nz(b, f) {
            wr::<i16>(s, f, 0xFF);
        }
    }
    if (nz(s, DPAD_UP) || bx < 0) && (nz(s, DPAD_DOWN) || bx > 0) {
        wr::<i16>(s, DPAD_UP, 0);
        wr::<i16>(s, DPAD_DOWN, 0);
        wr::<i16>(s, LEFT_STICK_Y, 0);
    }
    if (nz(s, DPAD_LEFT) || bp < 0) && (nz(s, DPAD_RIGHT) || bp > 0) {
        wr::<i16>(s, DPAD_LEFT, 0);
        wr::<i16>(s, DPAD_RIGHT, 0);
        wr::<i16>(s, LEFT_STICK_X, 0);
    }
    for k in 0..12 {
        wr::<u32>(out, 4 * k, rd::<u32>(s, 4 * k));
    }
    out
}

/// `CPad::SetDrunkInputDelay(int32)` (0x53F910): `DrunkDrivingBufferUsed = delay`.
#[hook(addr = 0x53F910, conv = thiscall, name = "CPad::SetDrunkInputDelay", expect = "8B 44 24 04 89 41 74", this = "CPad")]
pub fn set_drunk_input_delay(this: GPtr<u8>, delay: i32) {
    wr::<i32>(this, DRUNK_BUFFER_USED, delay);
}

/// The camera position the shake functions use: `TheCamera.m_pMat ? &m_pMat->pos : &placement.pos`.
fn camera_pos() -> GPtr<f32> {
    let m = g::<GPtr<u8>>(CAMERA_MATRIX).read();
    if m.is_null() { g::<f32>(CAMERA_PLACEMENT_POS) } else { m.byte_add(0x30).cast() }
}

fn shake_allowed() -> bool {
    g::<u8>(PREFS_USE_VIBRATION).read() != 0 && g::<u8>(CUTSCENE_RUNNING).read() == 0
}

/// `CPad::StartShake(int16 time, uint8 freq, uint32 delay)` (0x53F920, entry `jmp` into a chunk at
/// 0x40A0B5): nothing without vibration or during a cutscene; `freq == 0` stops the shake; else,
/// unless an earlier shake is still pending (`time < end`, unsigned) with a stronger or equal
/// frequency, the longer duration wins (`time > ShakeDur`, signed) and the end time / strength
/// (+0x12C / +0x130) are set from `delay` and `freq`.
#[hook(addr = 0x53F920, conv = thiscall, name = "CPad::StartShake", expect = "E9 90 A7 EC FF 33", this = "CPad")]
pub fn start_shake(this: GPtr<u8>, time: u32, freq: u32, delay: u32) {
    if !shake_allowed() {
        return;
    }
    let freq = freq as u8;
    if freq == 0 {
        wr::<i16>(this, SHAKE_DUR, 0);
        wr::<u8>(this, SHAKE_FREQ, 0);
        return;
    }
    let now = TIME_MS.get();
    if now < rd::<u32>(this, 0x12C) && freq <= rd::<u8>(this, 0x130) {
        return;
    }
    let t = time as i16;
    if t > rd::<i16>(this, SHAKE_DUR) {
        wr::<i16>(this, SHAKE_DUR, t);
        wr::<u8>(this, SHAKE_FREQ, freq);
    }
    wr::<u32>(this, 0x12C, TIME_MS.get().wrapping_add(delay));
    wr::<u8>(this, 0x130, freq);
}

/// `CPad::StartShake_Distance(int16 time, uint8 freq, float x, float y, float z)` (0x53F9A0): only
/// within 70 m of the camera (`sqrt(dz² + dy² + dx²)` in that order, x87); `freq == 0` stops,
/// else a longer `time` (signed) replaces the shake.
#[hook(addr = 0x53F9A0, conv = thiscall, name = "CPad::StartShake_Distance", expect = "A0 68 67 BA 00", this = "CPad")]
pub fn start_shake_distance(this: GPtr<u8>, time: u32, freq: u32, px: f32, py: f32, pz: f32) {
    if !shake_allowed() {
        return;
    }
    let c = camera_pos();
    let d = |k: i32, v: f32| {
        let t = x(c.wrapping_index(k).read()).fsub_m(v);
        t.fmul(x(c.wrapping_index(k).read()).fsub_m(v))
    };
    let s = d(2, pz).fadd(d(1, py)).fadd(d(0, px)).fsqrt();
    if s.fcom(x(70.0)) != Cmp::Less {
        return;
    }
    let freq = freq as u8;
    if freq == 0 {
        wr::<i16>(this, SHAKE_DUR, 0);
        wr::<u8>(this, SHAKE_FREQ, 0);
        return;
    }
    let t = time as i16;
    if t > rd::<i16>(this, SHAKE_DUR) {
        wr::<i16>(this, SHAKE_DUR, t);
        wr::<u8>(this, SHAKE_FREQ, freq);
    }
}

/// `CPad::StartShake_Train(float x, float y)` (0x53FA70): not in a train (vehicle type +0x590 == 6),
/// within 70 m (squared distance `dy² + dx²` < 4900 on the x87 value); if `ShakeDur < 100`:
/// `ShakeDur = 100`, `ShakeFreq = _ftol(70 - sqrt(d²f32) + 30)`.
#[hook(addr = 0x53FA70, conv = thiscall, name = "CPad::StartShake_Train", expect = "A0 68 67 BA 00", this = "CPad")]
pub fn start_shake_train(this: GPtr<u8>, px: f32, py: f32) {
    if !shake_allowed() {
        return;
    }
    if !find_player_vehicle(-1, 0).is_null() && rd::<i32>(find_player_vehicle(-1, 0), 0x590) == 6 {
        return;
    }
    let c = camera_pos();
    let d = |k: i32, v: f32| {
        let t = x(c.wrapping_index(k).read()).fsub_m(v);
        t.fmul(x(c.wrapping_index(k).read()).fsub_m(v))
    };
    let d2 = d(1, py).fadd(d(0, px));
    let d2f = d2.to_f32();
    if d2.fcom(x(4900.0)) != Cmp::Less {
        return;
    }
    if rd::<i16>(this, SHAKE_DUR) >= 100 {
        return;
    }
    wr::<i16>(this, SHAKE_DUR, 100);
    let f = x(d2f).fsqrt().fsubr_m(70.0).fadd_m(30.0);
    wr::<u8>(this, SHAKE_FREQ, ftol32(f) as u8);
}

/// `CPad::ProcessPCSpecificStuff` (0x53FB40): `ret` (not hookable).
pub fn process_pc_specific_stuff(_this: GPtr<u8>) {}

/// `CPad::StopShaking(int16)` (0x53FB50): `ret 4` (not hookable).
pub fn stop_shaking(_this: GPtr<u8>, _n: u32) {}

/// `CPad::StopPadsShaking` (0x541D70): `ret` (not hookable).
pub fn stop_pads_shaking() {}

// --------------------------------------------------------------------------- AX getters ---

/// `CPad::GetSteeringLeftRight` (0x53FB80): 0 while `DisablePlayerControls` (`xor ax, ax`: the upper
/// half of EAX is the caller's); Mode 0..3: `SteeringLeftRightBuffer[0] = NewState.LeftStickX`, then
/// `SteeringLeftRightBuffer[DrunkDrivingBufferUsed]` (zero-extended: EAX was `movzx` of Mode); other
/// modes 0.
#[hook(
    addr = 0x53FB80,
    conv = thiscall,
    name = "CPad::GetSteeringLeftRight",
    expect = "66 83 B9 0E 01 00 00 00",
    this = "CPad",
    entry_eax
)]
pub fn get_steering_left_right(this: GPtr<u8>) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    if mode(this) > 3 {
        return 0;
    }
    wr::<i16>(this, STEERING_BUFFER, rd::<i16>(this, NEW_STATE + LEFT_STICK_X));
    let i = rd::<i32>(this, DRUNK_BUFFER_USED);
    u32::from(rd::<u16>(this, STEERING_BUFFER + (i as isize) * 2))
}

/// The shape of the simple stick getters: 0 while disabled (caller's upper EAX), Mode 0..3 the field
/// zero-extended, other modes 0.
fn stick_getter(this: GPtr<u8>, off: isize) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    if mode(this) > 3 { 0 } else { zx(this, off) }
}

/// `CPad::GetSteeringUpDown` (0x53FBD0): `NewState.LeftStickY` (see [`stick_getter`]).
#[hook(addr = 0x53FBD0, conv = thiscall, name = "CPad::GetSteeringUpDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_steering_up_down(this: GPtr<u8>) -> u32 {
    stick_getter(this, LEFT_STICK_Y)
}

/// `CPad::GetPedWalkLeftRight()` (0x53FC90): `NewState.LeftStickX`.
#[hook(addr = 0x53FC90, conv = thiscall, name = "CPad::GetPedWalkLeftRight-", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_ped_walk_left_right(this: GPtr<u8>) -> u32 {
    stick_getter(this, LEFT_STICK_X)
}

/// `CPad::GetPedWalkUpDown()` (0x53FD30): `NewState.LeftStickY`.
#[hook(addr = 0x53FD30, conv = thiscall, name = "CPad::GetPedWalkUpDown-", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_ped_walk_up_down(this: GPtr<u8>) -> u32 {
    stick_getter(this, LEFT_STICK_Y)
}

/// `CPad::GetCarGunUpDown` (0x53FC10): Mode 0..2 `RightStickY`; Mode 3 `(DPadUp - DPadDown) / 2`
/// (signed, truncating, whole EAX); else 0.
#[hook(addr = 0x53FC10, conv = thiscall, name = "CPad::GetCarGunUpDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_car_gun_up_down(this: GPtr<u8>) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    match mode(this) {
        0..=2 => zx(this, RIGHT_STICK_Y),
        3 => ((sx(this, DPAD_UP) - sx(this, DPAD_DOWN)) / 2) as u32,
        _ => 0,
    }
}

/// `CPad::GetCarGunLeftRight` (0x53FC50): Mode 0..2 `RightStickX`; Mode 3 `(DPadRight - DPadLeft) / 2`.
#[hook(addr = 0x53FC50, conv = thiscall, name = "CPad::GetCarGunLeftRight", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_car_gun_left_right(this: GPtr<u8>) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    match mode(this) {
        0..=2 => zx(this, RIGHT_STICK_X),
        3 => ((sx(this, DPAD_RIGHT) - sx(this, DPAD_LEFT)) / 2) as u32,
        _ => 0,
    }
}

/// `CPad::GetCarGunFired` (0x53FF90): Mode 0..2: Circle → 1; else `DisableControls` byte +0x11C → 0;
/// else LeftShoulder1 → 2; Mode 3: RightShoulder1 → 1; else 0.
#[hook(addr = 0x53FF90, conv = thiscall, name = "CPad::GetCarGunFired", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_car_gun_fired(this: GPtr<u8>) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    match mode(this) {
        0..=2 => {
            if nz(this, BUTTON_CIRCLE) {
                1
            } else if byte_set(this, 0x11C) || !nz(this, LEFT_SHOULDER1) {
                0
            } else {
                2
            }
        }
        3 => u32::from(nz(this, RIGHT_SHOULDER1)),
        _ => 0,
    }
}

/// `CPad::CarGunJustDown` (0x53FFE0): the "just pressed" form of [`get_car_gun_fired`]
/// (`IsLeftShoulder1Pressed` called for the 2 case).
#[hook(addr = 0x53FFE0, conv = thiscall, name = "CPad::CarGunJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn car_gun_just_down(this: GPtr<u8>) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    match mode(this) {
        0..=2 => {
            if pressed(this, BUTTON_CIRCLE) != 0 {
                1
            } else if byte_set(this, 0x11C) {
                0
            } else if call_is_left_shoulder1_pressed(this) & 0xFF != 0 {
                2
            } else {
                0
            }
        }
        3 => pressed(this, RIGHT_SHOULDER1),
        _ => 0,
    }
}

/// `CPad::GetHandBrake` (0x540040): Mode 0/1 RightShoulder1, 2 Triangle, 3 LeftShoulder1.
#[hook(addr = 0x540040, conv = thiscall, name = "CPad::GetHandBrake", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_hand_brake(this: GPtr<u8>) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    match mode(this) {
        0 | 1 => zx(this, RIGHT_SHOULDER1),
        2 => zx(this, BUTTON_TRIANGLE),
        3 => zx(this, LEFT_SHOULDER1),
        _ => 0,
    }
}

/// `CPad::GetBrake` (0x540080): Mode 0..2 Square; Mode 3 `max(0, 2 * RightStickY)` (whole EAX).
#[hook(addr = 0x540080, conv = thiscall, name = "CPad::GetBrake", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_brake(this: GPtr<u8>) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    match mode(this) {
        0..=2 => zx(this, BUTTON_SQUARE),
        3 => {
            let v = sx(this, RIGHT_STICK_Y) << 1;
            if v < 0 { 0 } else { v as u32 }
        }
        _ => 0,
    }
}

/// `CPad::GetAccelerate` (0x5403F0): Mode 0..2 Cross; Mode 3 `max(0, -2 * RightStickY)`.
#[hook(addr = 0x5403F0, conv = thiscall, name = "CPad::GetAccelerate", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_accelerate(this: GPtr<u8>) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    match mode(this) {
        0..=2 => zx(this, BUTTON_CROSS),
        3 => {
            let v = sx(this, RIGHT_STICK_Y).wrapping_neg() << 1;
            if v < 0 { 0 } else { v as u32 }
        }
        _ => 0,
    }
}

/// `CPad::GetDisplayVitalStats(CPed*)` (0x5408B0): 0 when disabled or +0x11F; else, unless the ped
/// has a gun task (`GetTaskUseGun`, or simplest active task type 0x3FE), `LeftShoulder1 != 0` for
/// Mode 0..3.
#[hook(addr = 0x5408B0, conv = thiscall, name = "CPad::GetDisplayVitalStats", expect = "57 8B F9 66 83 BF 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_display_vital_stats(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    if disabled(this) || byte_set(this, 0x11F) {
        return with_entry_hi(0);
    }
    let mut gun = false;
    if !ped.is_null() {
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        if !get_task_use_gun(intel).is_null() {
            gun = true;
        } else {
            let intel = rd::<GPtr<u8>>(ped, 0x47C);
            if !get_simplest_active_task(intel.byte_add(4)).is_null() {
                let intel = rd::<GPtr<u8>>(ped, 0x47C);
                let t = get_simplest_active_task(intel.byte_add(4));
                gun = task_get_type(t) == 0x3FE;
            }
        }
    }
    if mode(this) > 3 || gun { 0 } else { u32::from(nz(this, LEFT_SHOULDER1)) }
}

/// Whether the ped is aiming (`GetWeapon`/`WeaponJustDown`): `GetTaskUseGun`, a simplest active
/// task of type 0x3FE, or attached (+0xFC).
fn ped_aiming(ped: GPtr<u8>) -> bool {
    if ped.is_null() {
        return false;
    }
    if !get_task_use_gun(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        return true;
    }
    if !get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4)).is_null() {
        let t = get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
        if task_get_type(t) == 0x3FE {
            return true;
        }
    }
    rd::<u32>(ped, 0xFC) != 0
}

/// `CPad::GetWeapon(CPed*)` (0x540180, whole EAX): 0 when disabled or +0x11B; aiming (cleared by
/// +0x11C); Mode 0/1 `Circle (+ LeftShoulder1 when aiming)` sign-extended, 2 Cross, 3
/// RightShoulder1; other modes 0.
#[hook(addr = 0x540180, conv = thiscall, name = "CPad::GetWeapon", expect = "56 8B F1 66 83 BE 0E 01 00 00 00", this = "CPad")]
pub fn get_weapon(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    if disabled(this) || byte_set(this, 0x11B) {
        return 0;
    }
    let mut aim = ped_aiming(ped);
    if byte_set(this, 0x11C) {
        aim = false;
    }
    (match mode(this) {
        0 | 1 => {
            let c = sx(this, BUTTON_CIRCLE);
            if aim { c.wrapping_add(sx(this, LEFT_SHOULDER1)) } else { c }
        }
        2 => sx(this, BUTTON_CROSS),
        3 => sx(this, RIGHT_SHOULDER1),
        _ => 0,
    }) as u32
}

/// `CPad::WeaponJustDown(CPed*)` (0x540250, AL): Mode 0/1 `IsCirclePressed() || (aiming &&
/// IsLeftShoulder1Pressed())`, 2 `IsCrossPressed()`, 3 `IsRightShoulder1Pressed()`.
#[hook(addr = 0x540250, conv = thiscall, name = "CPad::WeaponJustDown", expect = "57 8B F9 66 83 BF 0E 01 00 00 00", this = "CPad")]
pub fn weapon_just_down(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if disabled(this) || byte_set(this, 0x11B) {
        return 0;
    }
    let mut aim = ped_aiming(ped);
    if byte_set(this, 0x11C) {
        aim = false;
    }
    match mode(this) {
        0 | 1 => {
            u8::from(call_is_circle_pressed(this) as u8 != 0 || (aim && call_is_left_shoulder1_pressed(this) as u8 != 0))
        }
        2 => call_is_cross_pressed(this) as u8,
        3 => call_is_right_shoulder1_pressed(this) as u8,
        _ => 0,
    }
}

// --------------------------------------------------------------------------- AL getters ---

/// `CPad::GetLookLeft` (0x53FDD0, AL): false when disabled, when LeftShoulder2 was just pressed, or
/// RightShoulder2 just released; else `LeftShoulder2 && !RightShoulder2`.
#[hook(addr = 0x53FDD0, conv = thiscall, name = "CPad::GetLookLeft", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_look_left(this: GPtr<u8>) -> u8 {
    look_side(this, LEFT_SHOULDER2, RIGHT_SHOULDER2)
}

/// `CPad::GetLookRight` (0x53FE10, AL): [`get_look_left`] with the shoulders swapped.
#[hook(addr = 0x53FE10, conv = thiscall, name = "CPad::GetLookRight", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_look_right(this: GPtr<u8>) -> u8 {
    look_side(this, RIGHT_SHOULDER2, LEFT_SHOULDER2)
}

fn look_side(this: GPtr<u8>, me: isize, other: isize) -> u8 {
    if disabled(this) {
        return 0;
    }
    let d = nz(this, me);
    if d && !nz(this, OLD_STATE + me) {
        return 0;
    }
    let o = nz(this, other);
    if !o && nz(this, OLD_STATE + other) {
        return 0;
    }
    u8::from(d && !o)
}

/// `CPad::GetLookBehindForCar` (0x53FE70, AL): `(LS2 new || old) && (RS2 new || old)`.
#[hook(addr = 0x53FE70, conv = thiscall, name = "CPad::GetLookBehindForCar", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_look_behind_for_car(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    let l = nz(this, LEFT_SHOULDER2) || nz(this, OLD_STATE + LEFT_SHOULDER2);
    let r = nz(this, RIGHT_SHOULDER2) || nz(this, OLD_STATE + RIGHT_SHOULDER2);
    u8::from(l && r)
}

/// `CPad::GetLookBehindForPed` (0x53FEC0, AL): `ShockButtonR != 0`.
#[hook(addr = 0x53FEC0, conv = thiscall, name = "CPad::GetLookBehindForPed", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_look_behind_for_ped(this: GPtr<u8>) -> u8 {
    u8::from(!disabled(this) && nz(this, SHOCK_BUTTON_R))
}

/// `CPad::GetHydraulicJump` (0x53FF70, AL): `ShockButtonR != 0`.
#[hook(addr = 0x53FF70, conv = thiscall, name = "CPad::GetHydraulicJump", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_hydraulic_jump(this: GPtr<u8>) -> u8 {
    u8::from(!disabled(this) && nz(this, SHOCK_BUTTON_R))
}

/// `CPad::GetHorn` (0x53FEE0, AL): Mode 0/3 ShockButtonL, 1 LeftShoulder1, 2 RightShoulder1.
#[hook(addr = 0x53FEE0, conv = thiscall, name = "CPad::GetHorn", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_horn(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    u8::from(match mode(this) {
        0 | 3 => nz(this, SHOCK_BUTTON_L),
        1 => nz(this, LEFT_SHOULDER1),
        2 => nz(this, RIGHT_SHOULDER1),
        _ => false,
    })
}

/// `CPad::HornJustDown` (0x53FF30, AL): Mode 0/3 ShockButtonL pressed (an unnamed helper at
/// 0x509840), 1 `IsLeftShoulder1Pressed()`, 2 `IsRightShoulder1Pressed()` (tail jumps).
#[hook(addr = 0x53FF30, conv = thiscall, name = "CPad::HornJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn horn_just_down(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    match mode(this) {
        0 | 3 => pressed(this, SHOCK_BUTTON_L) as u8,
        1 => call_is_left_shoulder1_pressed(this) as u8,
        2 => call_is_right_shoulder1_pressed(this) as u8,
        _ => 0,
    }
}

/// `CPad::GetExitVehicle` (0x5400D0, AL): not when disabled or +0x119; Mode 2 LeftShoulder1, else
/// Triangle.
#[hook(addr = 0x5400D0, conv = thiscall, name = "CPad::GetExitVehicle", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_exit_vehicle(this: GPtr<u8>) -> u8 {
    if disabled(this) || byte_set(this, 0x119) {
        return 0;
    }
    u8::from(match mode(this) {
        0 | 1 | 3 => nz(this, BUTTON_TRIANGLE),
        2 => nz(this, LEFT_SHOULDER1),
        _ => false,
    })
}

/// `CPad::ExitVehicleJustDown` (0x540120, AL; not hookable: its entry bytes are read as data by the
/// protection at 0x156640F): Mode 2 `IsLeftShoulder1Pressed()`, else Triangle pressed.
pub fn exit_vehicle_just_down(this: GPtr<u8>) -> u8 {
    if disabled(this) || byte_set(this, 0x119) {
        return 0;
    }
    match mode(this) {
        0 | 1 | 3 => pressed(this, BUTTON_TRIANGLE) as u8,
        2 => call_is_left_shoulder1_pressed(this) as u8,
        _ => 0,
    }
}

/// `CPad::GetMeleeAttack(bool)` (0x540340, AL): Circle → 1; with the flag: Cross 2, Square 3,
/// Triangle 4.
#[hook(addr = 0x540340, conv = thiscall, name = "CPad::GetMeleeAttack", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_melee_attack(this: GPtr<u8>, all: u32) -> u8 {
    if disabled(this) {
        return 0;
    }
    if nz(this, BUTTON_CIRCLE) {
        return 1;
    }
    if all as u8 == 0 {
        return 0;
    }
    if nz(this, BUTTON_CROSS) {
        2
    } else if nz(this, BUTTON_SQUARE) {
        3
    } else if nz(this, BUTTON_TRIANGLE) {
        4
    } else {
        0
    }
}

/// `CPad::MeleeAttackJustDown(bool)` (0x540390, AL): Circle pressed → 1; with the flag: Cross
/// pressed 2, Square **held** 3 (original: no edge test), `IsTrianglePressed()` 4.
#[hook(addr = 0x540390, conv = thiscall, name = "CPad::MeleeAttackJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn melee_attack_just_down(this: GPtr<u8>, all: u32) -> u8 {
    if disabled(this) {
        return 0;
    }
    if pressed(this, BUTTON_CIRCLE) != 0 {
        return 1;
    }
    if all as u8 == 0 {
        return 0;
    }
    if pressed(this, BUTTON_CROSS) != 0 {
        2
    } else if nz(this, BUTTON_SQUARE) {
        3
    } else if call_is_triangle_pressed(this) as u8 != 0 {
        4
    } else {
        0
    }
}

/// `CPad::GetAccelerateJustDown` (0x540440, AL): Mode 0/2 Cross pressed, 1 `IsCrossPressed()`, 3
/// RightStickY "pressed" (an unnamed helper at 0x53ED90).
#[hook(addr = 0x540440, conv = thiscall, name = "CPad::GetAccelerateJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_accelerate_just_down(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    match mode(this) {
        0 | 2 => pressed(this, BUTTON_CROSS) as u8,
        1 => call_is_cross_pressed(this) as u8,
        3 => pressed(this, RIGHT_STICK_Y) as u8,
        _ => 0,
    }
}

/// `CPad::sub_5404F0` (0x5404F0, AL): DPadDown pressed while Mode == 1.
#[hook(addr = 0x5404F0, conv = thiscall, name = "CPad::sub_5404F0", expect = "0F B7 81 0A 01 00 00", this = "CPad")]
pub fn sub_5404f0(this: GPtr<u8>) -> u8 {
    if mode(this) == 1 { pressed(this, DPAD_DOWN) as u8 } else { 0 }
}

/// `CPad::sub_540530` (0x540530, whole EAX): Mode 0/2/3 Select pressed; Mode 1 DPadUp or DPadDown
/// pressed; else 0.
#[hook(addr = 0x540530, conv = thiscall, name = "CPad::sub_540530", expect = "0F B7 81 0A 01 00 00", this = "CPad")]
pub fn sub_540530(this: GPtr<u8>) -> u32 {
    match mode(this) {
        0 | 2 | 3 => pressed(this, SELECT),
        1 => u32::from(pressed(this, DPAD_UP) != 0 || pressed(this, DPAD_DOWN) != 0),
        _ => 0,
    }
}

/// "Just released": `NewState.f == 0 && OldState.f != 0`.
fn released(this: GPtr<u8>, f: isize) -> u8 {
    u8::from(!nz(this, f) && nz(this, OLD_STATE + f))
}

/// `CPad::NextStationJustUp` (0x5405B0, AL): DPadUp released.
#[hook(addr = 0x5405B0, conv = thiscall, name = "CPad::NextStationJustUp", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn next_station_just_up(this: GPtr<u8>) -> u8 {
    if disabled(this) { 0 } else { released(this, DPAD_UP) }
}

/// `CPad::LastStationJustUp` (0x5405E0, AL): DPadDown released.
#[hook(addr = 0x5405E0, conv = thiscall, name = "CPad::LastStationJustUp", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn last_station_just_up(this: GPtr<u8>) -> u8 {
    if disabled(this) { 0 } else { released(this, DPAD_DOWN) }
}

/// `pressed(f)` unless disabled or the `DisableControls` byte `b` is set (AL).
fn pressed_unless(this: GPtr<u8>, b: Option<isize>, f: isize) -> u8 {
    if disabled(this) || b.is_some_and(|b| byte_set(this, b)) { 0 } else { pressed(this, f) as u8 }
}

/// `held(f)` unless disabled or the byte `b` is set (AL).
fn held_unless(this: GPtr<u8>, b: Option<isize>, f: isize) -> u8 {
    if disabled(this) || b.is_some_and(|b| byte_set(this, b)) { 0 } else { u8::from(nz(this, f)) }
}

/// `CPad::CycleWeaponLeftJustDown` (0x540610, AL): LeftShoulder2 pressed (not with +0x11D).
#[hook(addr = 0x540610, conv = thiscall, name = "CPad::CycleWeaponLeftJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn cycle_weapon_left_just_down(this: GPtr<u8>) -> u8 {
    pressed_unless(this, Some(0x11D), LEFT_SHOULDER2)
}

/// `CPad::CycleWeaponRightJustDown` (0x540640, AL): RightShoulder2 pressed (not with +0x11D).
#[hook(addr = 0x540640, conv = thiscall, name = "CPad::CycleWeaponRightJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn cycle_weapon_right_just_down(this: GPtr<u8>) -> u8 {
    pressed_unless(this, Some(0x11D), RIGHT_SHOULDER2)
}

/// `CPad::GetTarget` (0x540670, AL): Mode 0..2 RightShoulder1, 3 LeftShoulder1.
#[hook(addr = 0x540670, conv = thiscall, name = "CPad::GetTarget", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_target(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    u8::from(match mode(this) {
        0..=2 => nz(this, RIGHT_SHOULDER1),
        3 => nz(this, LEFT_SHOULDER1),
        _ => false,
    })
}

/// `CPad::GetEnterTargeting` (0x5406B0, AL): Mode 0..2 RightShoulder1 pressed, 3 LeftShoulder1 pressed.
#[hook(addr = 0x5406B0, conv = thiscall, name = "CPad::GetEnterTargeting", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_enter_targeting(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    match mode(this) {
        0..=2 => pressed(this, RIGHT_SHOULDER1) as u8,
        3 => pressed(this, LEFT_SHOULDER1) as u8,
        _ => 0,
    }
}

/// `CPad::GetDuck` (0x540700, AL): ShockButtonL (not with +0x11A).
#[hook(addr = 0x540700, conv = thiscall, name = "CPad::GetDuck", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_duck(this: GPtr<u8>) -> u8 {
    held_unless(this, Some(0x11A), SHOCK_BUTTON_L)
}

/// `CPad::DuckJustDown` (0x540720, AL): ShockButtonL pressed (not with +0x11A).
#[hook(addr = 0x540720, conv = thiscall, name = "CPad::DuckJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn duck_just_down(this: GPtr<u8>) -> u8 {
    pressed_unless(this, Some(0x11A), SHOCK_BUTTON_L)
}

/// `CPad::GetJump` (0x540750, AL): Square (not with +0x11E).
#[hook(addr = 0x540750, conv = thiscall, name = "CPad::GetJump", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_jump(this: GPtr<u8>) -> u8 {
    held_unless(this, Some(0x11E), BUTTON_SQUARE)
}

/// `CPad::JumpJustDown` (0x540770, AL): Square pressed (not with +0x11E).
#[hook(addr = 0x540770, conv = thiscall, name = "CPad::JumpJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn jump_just_down(this: GPtr<u8>) -> u8 {
    pressed_unless(this, Some(0x11E), BUTTON_SQUARE)
}

/// `CPad::GetSprint` (0x5407A0, AL): Mode 2 Circle, else Cross.
#[hook(addr = 0x5407A0, conv = thiscall, name = "CPad::GetSprint", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_sprint(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    u8::from(match mode(this) {
        0 | 1 | 3 => nz(this, BUTTON_CROSS),
        2 => nz(this, BUTTON_CIRCLE),
        _ => false,
    })
}

/// `CPad::SprintJustDown` (0x5407F0, AL): Mode 2 Circle pressed, else Cross pressed.
#[hook(addr = 0x5407F0, conv = thiscall, name = "CPad::SprintJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn sprint_just_down(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    match mode(this) {
        0 | 1 | 3 => pressed(this, BUTTON_CROSS) as u8,
        2 => pressed(this, BUTTON_CIRCLE) as u8,
        _ => 0,
    }
}

/// `CPad::ShiftTargetLeftJustDown` (0x540850, AL): LeftShoulder2 pressed.
#[hook(addr = 0x540850, conv = thiscall, name = "CPad::ShiftTargetLeftJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn shift_target_left_just_down(this: GPtr<u8>) -> u8 {
    pressed_unless(this, None, LEFT_SHOULDER2)
}

/// `CPad::ShiftTargetRightJustDown` (0x540880, AL): RightShoulder2 pressed.
#[hook(addr = 0x540880, conv = thiscall, name = "CPad::ShiftTargetRightJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn shift_target_right_just_down(this: GPtr<u8>) -> u8 {
    pressed_unless(this, None, RIGHT_SHOULDER2)
}

/// `CPad::CollectPickupJustDown` (0x540A70, AL): Mode 0/1 LeftShoulder1 pressed, 2 Triangle, 3 Circle.
#[hook(addr = 0x540A70, conv = thiscall, name = "CPad::CollectPickupJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn collect_pickup_just_down(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    match mode(this) {
        0 | 1 => pressed(this, LEFT_SHOULDER1) as u8,
        2 => pressed(this, BUTTON_TRIANGLE) as u8,
        3 => pressed(this, BUTTON_CIRCLE) as u8,
        _ => 0,
    }
}

/// `CPad::GetForceCameraBehindPlayer` (0x540AE0, AL): Mode 0/1 LeftShoulder1, 2 Triangle, 3 Circle.
#[hook(addr = 0x540AE0, conv = thiscall, name = "CPad::GetForceCameraBehindPlayer", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_force_camera_behind_player(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    u8::from(match mode(this) {
        0 | 1 => nz(this, LEFT_SHOULDER1),
        2 => nz(this, BUTTON_TRIANGLE),
        3 => nz(this, BUTTON_CIRCLE),
        _ => false,
    })
}

/// `CPad::SniperZoomIn` (0x540B30, AL): Mode 2 Triangle, else LeftShoulder2 || Square.
#[hook(addr = 0x540B30, conv = thiscall, name = "CPad::SniperZoomIn", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn sniper_zoom_in(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    u8::from(match mode(this) {
        0 | 1 | 3 => nz(this, LEFT_SHOULDER2) || nz(this, BUTTON_SQUARE),
        2 => nz(this, BUTTON_TRIANGLE),
        _ => false,
    })
}

/// `CPad::SniperZoomOut` (0x540B80, AL): Mode 2 Square, else RightShoulder2 || Cross.
#[hook(addr = 0x540B80, conv = thiscall, name = "CPad::SniperZoomOut", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn sniper_zoom_out(this: GPtr<u8>) -> u8 {
    if disabled(this) {
        return 0;
    }
    u8::from(match mode(this) {
        0 | 1 | 3 => nz(this, RIGHT_SHOULDER2) || nz(this, BUTTON_CROSS),
        2 => nz(this, BUTTON_SQUARE),
        _ => false,
    })
}

/// `CPad::GetGroupControlForward` (0x541190, AL): DPadUp held.
#[hook(addr = 0x541190, conv = thiscall, name = "CPad::GetGroupControlForward", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_group_control_forward(this: GPtr<u8>) -> u8 {
    held_unless(this, None, DPAD_UP)
}

/// `CPad::GetGroupControlBack` (0x5411B0, AL): DPadDown held.
#[hook(addr = 0x5411B0, conv = thiscall, name = "CPad::GetGroupControlBack", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn get_group_control_back(this: GPtr<u8>) -> u8 {
    held_unless(this, None, DPAD_DOWN)
}

/// `CPad::ConversationYesJustDown` (0x5411D0, AL): DPadRight pressed.
#[hook(addr = 0x5411D0, conv = thiscall, name = "CPad::ConversationYesJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn conversation_yes_just_down(this: GPtr<u8>) -> u8 {
    pressed_unless(this, None, DPAD_RIGHT)
}

/// `CPad::ConversationNoJustDown` (0x541200, AL): DPadLeft pressed.
#[hook(addr = 0x541200, conv = thiscall, name = "CPad::ConversationNoJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn conversation_no_just_down(this: GPtr<u8>) -> u8 {
    pressed_unless(this, None, DPAD_LEFT)
}

/// `CPad::GroupControlForwardJustDown` (0x541230, AL): DPadUp pressed.
#[hook(addr = 0x541230, conv = thiscall, name = "CPad::GroupControlForwardJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn group_control_forward_just_down(this: GPtr<u8>) -> u8 {
    pressed_unless(this, None, DPAD_UP)
}

/// `CPad::GroupControlBackJustDown` (0x541260, AL): DPadDown pressed.
#[hook(addr = 0x541260, conv = thiscall, name = "CPad::GroupControlBackJustDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad")]
pub fn group_control_back_just_down(this: GPtr<u8>) -> u8 {
    pressed_unless(this, None, DPAD_DOWN)
}

// ------------------------------------------------------------ analogue pad (static, Pads[0]) ---

fn pad0_word(off: isize) -> i16 {
    rd::<i16>(PADS_ARRAY.ptr().cast(), off)
}

/// The shared shape of `GetAnaloguePad*` (AL): the stick value is stored as the new "old" value;
/// true when it passed `beyond` while the previous value had not passed `back`.
fn analogue(old: &sa_core::mem::Global<i16>, v: i16, hit: bool, was_back: bool) -> u8 {
    let r = hit && was_back;
    old.set(v);
    u8::from(r)
}

/// `CPad::GetAnaloguePadUp` (0x540950, static, AL; entry `nop; jmp` into a chunk at 0x407271):
/// `Pads[0].LeftStickY < -15` while the previous value was `>= -5`.
#[hook(addr = 0x540950, conv = cdecl, name = "CPad::GetAnaloguePadUp", expect = "90 E9 1B 69 EC FF")]
pub fn get_analogue_pad_up() -> u8 {
    let v = pad0_word(LEFT_STICK_Y);
    analogue(&ANALOGUE_OLD_UP, v, v < -15, ANALOGUE_OLD_UP.get() >= -5)
}

/// `CPad::GetAnaloguePadDown` (0x540980, AL): `LeftStickY > 15` while the previous was `<= 5`.
#[hook(addr = 0x540980, conv = cdecl, name = "CPad::GetAnaloguePadDown", expect = "66 A1 5A 34 B7 00")]
pub fn get_analogue_pad_down() -> u8 {
    let v = pad0_word(LEFT_STICK_Y);
    analogue(&ANALOGUE_OLD_DOWN, v, v > 15, ANALOGUE_OLD_DOWN.get() <= 5)
}

/// `CPad::GetAnaloguePadLeft` (0x5409B0, AL): `LeftStickX < -15` while the previous was `>= -5`.
#[hook(addr = 0x5409B0, conv = cdecl, name = "CPad::GetAnaloguePadLeft", expect = "A1 58 34 B7 00")]
pub fn get_analogue_pad_left() -> u8 {
    let v = pad0_word(LEFT_STICK_X);
    analogue(&ANALOGUE_OLD_LEFT, v, v < -15, ANALOGUE_OLD_LEFT.get() >= -5)
}

/// `CPad::GetAnaloguePadRight` (0x5409E0, AL): `LeftStickX > 15` while the previous was `<= 5`.
#[hook(addr = 0x5409E0, conv = cdecl, name = "CPad::GetAnaloguePadRight", expect = "A1 58 34 B7 00")]
pub fn get_analogue_pad_right() -> u8 {
    let v = pad0_word(LEFT_STICK_X);
    analogue(&ANALOGUE_OLD_RIGHT, v, v > 15, ANALOGUE_OLD_RIGHT.get() <= 5)
}

// ------------------------------------------------------------------- look / walk / aim ---

/// `|a| < |b|` as the x87 `fild; fabs` compare decides it (int16 values: exact).
fn abs_less(a: i16, b: i16) -> bool {
    i32::from(a).abs() < i32::from(b).abs()
}

/// The dead zone and scale of `LookAround*`: `|v| > 35` → `_ftol((v ∓ 35) * 1.3763441)` (low 32
/// bits), else 0.
fn look_scale(v: i16) -> u32 {
    let k = f32::from_bits(0x420C_0000); // 35.0 [0x85F070]
    let m = f32::from_bits(0x3FB0_2C0B); // 1.3763441 [0x863B48]
    let f = xi(i32::from(v));
    if i32::from(v).abs() <= 35 {
        return 0;
    }
    let r = if v > 0 { f.fsub_m(k) } else { f.fadd_m(k) };
    ftol32(r.fmul_m(m)) as u32
}

/// The stick pair of `LookAround*`: `byte_8CD782` swaps which stick is "look" (`dx`) and which the
/// alternative (`cx`); without mouse-3rd-person, the larger one wins (unless `byte_B73403` is set and
/// the ped is not attached).
fn look_pick(this: GPtr<u8>, ped: GPtr<u8>, left: isize, right: isize) -> i16 {
    let (mut dx, cx) = if BYTE_8CD782.get() != 0 {
        (rd::<i16>(this, right), rd::<i16>(this, left))
    } else {
        (rd::<i16>(this, left), rd::<i16>(this, right))
    };
    if USE_MOUSE_3RD_PERSON.get() == 0 {
        let cmp = if BYTE_B73403.get() != 0 {
            !ped.is_null() && rd::<u32>(ped, 0xFC) != 0
        } else {
            true
        };
        if cmp && abs_less(dx, cx) {
            dx = cx;
        }
    }
    dx
}

/// `CPad::LookAroundLeftRight(CPed*)` (0x540BD0): see [`look_pick`] / [`look_scale`]; 0 while
/// disabled (caller's upper EAX).
#[hook(addr = 0x540BD0, conv = thiscall, name = "CPad::LookAroundLeftRight", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn look_around_left_right(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    look_scale(look_pick(this, ped, LEFT_STICK_X, RIGHT_STICK_X))
}

/// `CPad::LookAroundUpDown(CPed*)` (0x540CC0): as [`look_around_left_right`] on the Y axes, negated
/// with `bInvertLook4Pad`.
#[hook(addr = 0x540CC0, conv = thiscall, name = "CPad::LookAroundUpDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn look_around_up_down(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    let mut v = look_pick(this, ped, LEFT_STICK_Y, RIGHT_STICK_Y);
    if INVERT_LOOK_4_PAD.get() != 0 {
        v = v.wrapping_neg();
    }
    look_scale(v)
}

/// `GetPedWalk*(CPed*)`: 0 unless `byte_B73403` or mouse-3rd-person; 0 when the ped is attached;
/// else the left stick (`byte_8CD782`) or the right one.
fn ped_walk(this: GPtr<u8>, ped: GPtr<u8>, left: isize, right: isize) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    if BYTE_B73403.get() == 0 && USE_MOUSE_3RD_PERSON.get() == 0 {
        return 0;
    }
    if !ped.is_null() && rd::<u32>(ped, 0xFC) != 0 {
        return 0;
    }
    // EAX held the ped pointer (or 0) before `mov ax, word`
    let hi = (ped.addr() as u32) & 0xFFFF_0000;
    hi | zx(this, if BYTE_8CD782.get() != 0 { left } else { right })
}

/// `CPad::GetPedWalkLeftRight(CPed*)` (0x540DC0).
#[hook(addr = 0x540DC0, conv = thiscall, name = "CPad::GetPedWalkLeftRight-ped", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_ped_walk_left_right_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    ped_walk(this, ped, LEFT_STICK_X, RIGHT_STICK_X)
}

/// `CPad::GetPedWalkUpDown(CPed*)` (0x540E20).
#[hook(addr = 0x540E20, conv = thiscall, name = "CPad::GetPedWalkUpDown-ped", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn get_ped_walk_up_down_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    ped_walk(this, ped, LEFT_STICK_Y, RIGHT_STICK_Y)
}

/// `AimWeapon*(CPed*)`: `si` = right stick, `cx` = left stick; the left one wins when larger,
/// compared only without mouse-3rd-person, with a ped that is attached (+0xFC) or in a vehicle
/// (+0x46C bit 8, +0x58C) it does not drive. Returns AX = the result and, as the original leaves it,
/// the upper half of EAX: the caller's (mouse-3rd-person), 0 (no ped), the ped pointer's (no
/// compare), or the sign extension of the left stick (compared).
fn aim(this: GPtr<u8>, ped: GPtr<u8>, left: isize, right: isize, invert: bool) -> u32 {
    if disabled(this) {
        return with_entry_hi(0);
    }
    let mut si = rd::<i16>(this, right);
    let cx = rd::<i16>(this, left);
    let hi: u32;
    if USE_MOUSE_3RD_PERSON.get() != 0 {
        hi = sa_core::abi::entry_eax() & 0xFFFF_0000;
    } else if ped.is_null() {
        hi = 0;
    } else {
        let attached = rd::<u32>(ped, 0xFC) != 0;
        let passenger = !attached && rd::<u32>(ped, 0x46C) & 0x100 != 0 && {
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            !v.is_null() && rd::<u32>(v, 0x460) != ped.addr() as u32
        };
        if attached || passenger {
            hi = (i32::from(cx) as u32) & 0xFFFF_0000;
            if abs_less(si, cx) {
                si = cx;
            }
        } else {
            hi = (ped.addr() as u32) & 0xFFFF_0000;
        }
    }
    if invert && INVERT_LOOK_4_PAD.get() != 0 {
        si = si.wrapping_neg();
    }
    hi | u32::from(si as u16)
}

/// `CPad::AimWeaponLeftRight(CPed*)` (0x541040): see [`aim`].
#[hook(addr = 0x541040, conv = thiscall, name = "CPad::AimWeaponLeftRight", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn aim_weapon_left_right(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    aim(this, ped, LEFT_STICK_X, RIGHT_STICK_X, false)
}

/// `CPad::AimWeaponUpDown(CPed*)` (0x5410C0): see [`aim`], negated with `bInvertLook4Pad`.
#[hook(addr = 0x5410C0, conv = thiscall, name = "CPad::AimWeaponUpDown", expect = "66 83 B9 0E 01 00 00 00", this = "CPad", entry_eax)]
pub fn aim_weapon_up_down(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    aim(this, ped, LEFT_STICK_Y, RIGHT_STICK_Y, true)
}

// --------------------------------------------------------------------- clear / update ---

fn clear_mouse_globals_new_old() {
    for k in 0..7 {
        wr::<u8>(NEW_MOUSE.ptr().cast(), k, 0);
    }
    for k in 0..7 {
        wr::<u8>(OLD_MOUSE.ptr().cast(), k, 0);
    }
}

/// `CPad::Clear(bool enablePlayerControls, bool resetPhase)` (0x541A70): the five controller
/// states, the three keyboard states, the mouse button bytes (new, old, temp), shake, steering
/// buffer, drunk delay; `resetPhase` → Phase = 0; `enablePlayerControls` → the disable word and
/// bytes +0x119..+0x11F = 0; then +0x111..+0x118, +0x120..+0x130 = 0.
#[hook(addr = 0x541A70, conv = thiscall, name = "CPad::Clear", expect = "56 8B F1 E8 08 D5 FF FF", this = "CPad")]
pub fn clear(this: GPtr<u8>, enable: u32, reset_phase: u32) {
    for off in [NEW_STATE, OLD_STATE, PC_TEMP_KEY_STATE, PC_TEMP_JOY_STATE, PC_TEMP_MOUSE_STATE] {
        call_controller_state_clear(this.byte_add(off));
    }
    call_keyboard_state_clear(NEW_KEY_STATE.ptr().cast());
    call_keyboard_state_clear(OLD_KEY_STATE.ptr().cast());
    call_keyboard_state_clear(TEMP_KEY_STATE.ptr().cast());
    clear_mouse_globals_new_old();
    for k in 0..7 {
        wr::<u8>(TEMP_MOUSE.ptr().cast(), k, 0);
    }
    if reset_phase as u8 != 0 {
        wr::<u8>(this, PHASE, 0);
    }
    wr::<u8>(this, SHAKE_FREQ, 0);
    wr::<i16>(this, SHAKE_DUR, 0);
    for k in 0..5 {
        wr::<u32>(this, STEERING_BUFFER + 4 * k, 0);
    }
    wr::<u32>(this, DRUNK_BUFFER_USED, 0);
    if enable as u8 != 0 {
        wr::<i16>(this, DISABLE_PLAYER_CONTROLS, 0);
        for k in 0x119..=0x11F {
            wr::<u8>(this, k, 0);
        }
    }
    wr::<u8>(this, 0x117, 0);
    wr::<u8>(this, 0x118, 0);
    wr::<u32>(this, 0x111, 0);
    wr::<u8>(this, 0x115, 0);
    wr::<u8>(this, 0x116, 0);
    for off in [0x124, 0x128, 0x120, 0x12C] {
        wr::<u32>(this, off, 0);
    }
    wr::<u8>(this, 0x130, 0);
}

/// `CPad::ClearMouseHistory` (0x541BD0, static): the button bytes of the temp, new and old mouse
/// states = 0.
#[hook(addr = 0x541BD0, conv = cdecl, name = "CPad::ClearMouseHistory", expect = "32 C0 A2 04 34 B7 00")]
pub fn clear_mouse_history() {
    for k in 0..7 {
        wr::<u8>(TEMP_MOUSE.ptr().cast(), k, 0);
    }
    clear_mouse_globals_new_old();
}

/// `CPad::Update(int16 padNumber)` (0x541C40; not hooked as a port: sa-hook's INF-5 input
/// recorder/replayer owns the entry): `OldState = NewState`; `NewState` = the reconciliation of the
/// keyboard and joystick temp states, then of the mouse temp state and that; the temp states
/// cleared; `LastTimeTouched` on input; the horn history ring (+0x111 + index, index +0x116 mod 5)
/// gets this frame's horn button; the steering buffer shifts by one; `JustOutOfFrontEnd` counts down.
pub fn update(this: GPtr<u8>, _n: u32) {
    let mut tmp = [0u8; 0x30];
    let t = GPtr::from_raw(tmp.as_mut_ptr());
    for k in 0..12 {
        wr::<u32>(this, OLD_STATE + 4 * k, rd::<u32>(this, NEW_STATE + 4 * k));
    }
    let r = call_reconcile(this, t, this.byte_add(PC_TEMP_KEY_STATE), this.byte_add(PC_TEMP_JOY_STATE));
    for k in 0..12 {
        wr::<u32>(this, 4 * k, rd::<u32>(r, 4 * k));
    }
    let r = call_reconcile(this, t, this.byte_add(PC_TEMP_MOUSE_STATE), this);
    for k in 0..12 {
        wr::<u32>(this, 4 * k, rd::<u32>(r, 4 * k));
    }
    call_controller_state_clear(this.byte_add(PC_TEMP_JOY_STATE));
    call_controller_state_clear(this.byte_add(PC_TEMP_KEY_STATE));
    call_controller_state_clear(this.byte_add(PC_TEMP_MOUSE_STATE));
    if call_controller_state_check_for_input(this) != 0 {
        wr::<u32>(this, LAST_TIME_TOUCHED, TIME_MS.get());
    }
    let mut i = rd::<u8>(this, 0x116).wrapping_add(1);
    wr::<u8>(this, 0x116, i);
    if i >= 5 {
        i = 0;
        wr::<u8>(this, 0x116, 0);
    }
    let horn = if disabled(this) {
        false
    } else {
        match mode(this) {
            0 | 3 => nz(this, SHOCK_BUTTON_L),
            1 => nz(this, LEFT_SHOULDER1),
            2 => nz(this, RIGHT_SHOULDER1),
            _ => false,
        }
    };
    wr::<u8>(this, 0x111 + isize::from(i), u8::from(horn));
    for k in (1..10).rev() {
        wr::<i16>(this, STEERING_BUFFER + 2 * k, rd::<i16>(this, STEERING_BUFFER + 2 * (k - 1)));
    }
    let j = rd::<u8>(this, 0x117);
    if j != 0 {
        wr::<u8>(this, 0x117, j - 1);
    }
}

/// `CPad::Initialise` (0x541D90, static): both pads `Clear(true, true)` and `Mode = 0`; then
/// `padNumber = 0`, `byte_B73401 = 0`.
#[hook(addr = 0x541D90, conv = cdecl, name = "CPad::Initialise", expect = "56 BE 62 35 B7 00")]
pub fn initialise() {
    for n in 0..2 {
        let p = get_pad(n);
        call_pad_clear(p, 1, 1);
        wr::<i16>(p, MODE, 0);
    }
    PAD_NUMBER.set(0);
    BYTE_B73401.set(0);
}

/// `CPad::UpdatePads` (0x541DD0, static): `Pads[0].UpdateMouse()`, `ProcessPad(0)`, the controls
/// manager's `ClearSimButtonPressCheckers`, `AffectPadFromKeyBoard`, `AffectPadFromMouse`, both pads'
/// `Update`, then `OldKeyState = NewKeyState`, `NewKeyState = TempKeyState`.
#[hook(addr = 0x541DD0, conv = cdecl, name = "CPad::UpdatePads", expect = "56 57 E9 36 1A EC FF")]
pub fn update_pads() {
    let p0 = get_pad(0);
    call_update_mouse(p0);
    call_process_pad(0);
    let m = GPtr::from_addr(CONTROLS_MANAGER);
    ccm_clear_sim_button_press_checkers(m);
    ccm_affect_pad_from_keyboard(m);
    ccm_affect_pad_from_mouse(m);
    call_pad_update(p0, 0);
    call_pad_update(get_pad(1), 1);
    OLD_KEY_STATE.set(NEW_KEY_STATE.get());
    NEW_KEY_STATE.set(TEMP_KEY_STATE.get());
}

/// `CPad::UpdateMouse` (0x53F3C0; not hooked as a port: sa-hook's INF-5 input recorder/replayer
/// owns the entry). Original (docs/re/specs/INPUT/CPad.deviations.md): only for the foreground app;
/// `diMouseInit` when the mouse device is missing and, if it then exists, continue in the same call;
/// `GetMouseState`; movement (inverted with the menu inactive) into the temp state's floats, wheel
/// `lZ` into +8, buttons as `rgbButtons[i] & 0x80`; old = new, new = temp; `LastTimeTouched` on input.
pub fn update_mouse(this: GPtr<u8>) {
    if is_foreground_app() == 0 {
        return;
    }
    let menu = g::<u8>(MENU_ACTIVE).read() != 0;
    if g::<u32>(DI_MOUSE).read() == 0 {
        let excl = !menu && is_video_mode_exclusive() != 0;
        di_mouse_init(u32::from(excl));
        if g::<u32>(DI_MOUSE).read() == 0 {
            return;
        }
    }
    let mut ms = [0u8; 0x14];
    let msp = GPtr::from_raw(ms.as_mut_ptr());
    if call_get_mouse_state(msp) < 0 {
        return;
    }
    let menu = g::<u8>(MENU_ACTIVE).read() != 0;
    let (mut mx, mut my) = (1i32, 1i32);
    if !menu {
        if g::<u8>(INVERT_MOUSE_Y).read() != 0 {
            my = -1;
        }
        if g::<u8>(INVERT_MOUSE_X).read() != 0 {
            mx = -1;
        }
    }
    let (lx, ly, lz) = (rd::<i32>(msp, 0), rd::<i32>(msp, 4), rd::<i32>(msp, 8));
    let t: GPtr<u8> = TEMP_MOUSE.ptr().cast();
    wr::<f32>(t, 0xC, xi(lx.wrapping_mul(mx)).to_f32());
    wr::<f32>(t, 0x10, xi(ly.wrapping_mul(my)).to_f32());
    wr::<f32>(t, 8, xi(lz).to_f32());
    wr::<u8>(t, 0, rd::<u8>(msp, 0xC) & 0x80);
    wr::<u8>(t, 1, rd::<u8>(msp, 0xD) & 0x80);
    wr::<u8>(t, 2, rd::<u8>(msp, 0xE) & 0x80);
    wr::<u8>(t, 3, u8::from(lz > 0));
    wr::<u8>(t, 4, u8::from(lz < 0));
    wr::<u8>(t, 5, rd::<u8>(msp, 0xF) & 0x80);
    wr::<u8>(t, 6, rd::<u8>(msp, 0x10) & 0x80);
    OLD_MOUSE.set(NEW_MOUSE.get());
    NEW_MOUSE.set(TEMP_MOUSE.get());
    if call_mouse_check_for_input(NEW_MOUSE.ptr().cast()) != 0 {
        wr::<u32>(this, LAST_TIME_TOUCHED, TIME_MS.get());
    }
}

/// `CPad::GetMouseState(DIMOUSESTATE2* out)` (0x746ED0): 1 without a mouse device; else zero `out`,
/// `GetDeviceState(0x14, out)`; on failure re-`Acquire` while it reports `DIERR_INPUTLOST`, zero `out`
/// again and return the second `GetDeviceState`'s result; success → 0.
#[hook(addr = 0x746ED0, conv = cdecl, name = "CPad::GetMouseState", expect = "A1 54 70 C1 00")]
pub fn get_mouse_state(out: GPtr<u8>) -> i32 {
    let dev = || rd::<GPtr<ComObj>>(g::<GPtr<u8>>(RS_PS).read(), 0x1C);
    if dev().is_null() {
        return 1;
    }
    for k in 0..5 {
        wr::<u32>(out, 4 * k, 0);
    }
    if did_get_device_state(dev(), 0x14, out) >= 0 {
        return 0;
    }
    let mut r = did_acquire(dev());
    while r == DIERR_INPUTLOST {
        r = did_acquire(dev());
    }
    for k in 0..5 {
        wr::<u32>(out, 4 * k, 0);
    }
    did_get_device_state(dev(), 0x14, out)
}

/// `CPad::ProcessPad(ePadID)` (0x746A10, returns 0): the DirectInput joystick of pad 0 / 1 (any other
/// id reads address 0 as the original does): poll (re-acquire while `DIERR_INPUTLOST`), read the
/// state, store old/new in the controls manager, post the pad events, convert the axes into the
/// persistent `PsGlobal` floats (POV hat → sin/cos), then write the sticks of
/// `GetPad(padNumber != 0)`'s `PCTempJoyState` with the invert/swap preferences (docs/re/specs/INPUT/
/// CPad.deviations.md lists the differences from gta-reversed, all kept).
#[hook(addr = 0x746A10, conv = cdecl, name = "CPad::ProcessPad", expect = "8B 44 24 04 81 EC 14 01 00 00")]
pub fn process_pad(id: i32) -> u32 {
    let ps = g::<GPtr<u8>>(RS_PS);
    let slot: GPtr<GPtr<ComObj>> = match id {
        0 => ps.read().byte_add(0x20).cast(),
        1 => ps.read().byte_add(0x24).cast(),
        _ => GPtr::NULL,
    };
    if slot.read().is_null() {
        return 0;
    }
    if did_poll(slot.read()) < 0 {
        let mut r = did_acquire(slot.read());
        while r == DIERR_INPUTLOST {
            r = did_acquire(slot.read());
        }
        if r < 0 || did_poll(slot.read()) < 0 {
            return 0;
        }
    }
    let mut js = [0u8; 0x110];
    let jp = GPtr::from_raw(js.as_mut_ptr());
    if did_get_device_state(slot.read(), 0x110, jp) < 0 {
        return 0;
    }
    let old = g::<[u8; 0x110]>(CONTROLS_MANAGER + 4);
    let new = g::<[u8; 0x110]>(CONTROLS_MANAGER + 0x114);
    let just = g::<u8>(CONTROLS_MANAGER);
    if just.read() == 1 {
        old.write(js);
        new.write(js);
        just.write(0);
    } else {
        old.write(new.read());
        new.write(js);
    }
    let mut arg: u32 = id as u32;
    rs_pad_event_handler(0x20, GPtr::from_raw(&mut arg));
    let k = f32::from_bits(0x3A03_126F); // 0.0005f [0x859978]
    if !slot.read().is_null() {
        g::<f32>(JOY_LX).write(xi(rd::<i32>(jp, 0)).fmul_m(k).to_f32());
        g::<f32>(JOY_LY).write(xi(rd::<i32>(jp, 4)).fmul_m(k).to_f32());
        let pov = rd::<u32>(jp, 0x20);
        if pov as u16 != 0xFFFF {
            let mut a = xi(pov as i32);
            if (pov as i32) < 0 {
                a = a.fadd_m(4_294_967_296.0);
            }
            let a = a.fmul_m(f32::from_bits(0x3C23_D70A)).fmul_m(f32::from_bits(0x3C8E_FA35));
            g::<f32>(JOY_LX).write(a.fsin().to_f32());
            g::<f32>(JOY_LY).write(a.fcos().fmul_m(-1.0).to_f32());
        }
        let j = ALL_VALID_WIN_JOYS.ptr().cast::<u8>().byte_add((arg as i32 as isize) << 4);
        if rd::<u8>(j, 6) != 0 && rd::<u8>(j, 5) != 0 {
            g::<f32>(JOY_RX).write(xi(rd::<i32>(jp, 8)).fmul_m(k).to_f32());
            g::<f32>(JOY_RY).write(xi(rd::<i32>(jp, 0x14)).fmul_m(k).to_f32());
        }
    }
    let mut n = u32::from(PAD_NUMBER.get() != 0);
    rs_pad_event_handler(0x20, GPtr::from_raw(&mut n));
    rs_pad_event_handler(0x1F, GPtr::from_raw(&mut n));
    let pad = get_pad(i32::from(PAD_NUMBER.get() != 0));
    // (axis global, invert flag, swap flag, field when not swapped, field when swapped)
    let axes = [
        (JOY_LX, 0xBA67F9usize, 0xBA67FDusize, 0xA8isize, 0xAAisize),
        (JOY_LY, 0xBA67FA, 0xBA67FD, 0xAA, 0xA8),
        (JOY_RX, 0xBA67FB, 0xBA67FE, 0xAC, 0xAE),
        (JOY_RY, 0xBA67FC, 0xBA67FE, 0xAE, 0xAC),
    ];
    for (axis, inv, swap, a, b) in axes {
        let v = g::<f32>(axis);
        if f64::from(v.read()).abs() > 0.300_000_011_920_928_96 {
            if g::<u8>(inv).read() != 0 {
                v.write(-v.read());
            }
            let w = ftol32(x(v.read()).fmul_m(128.0)) as i16;
            wr::<i16>(pad, if g::<u8>(swap).read() != 0 { b } else { a }, w);
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_PAD,
    HOOK_IS_CROSS_PRESSED,
    HOOK_IS_LEFT_SHOULDER1_PRESSED,
    HOOK_IS_RIGHT_SHOULDER1_PRESSED,
    HOOK_IS_TRIANGLE_PRESSED,
    HOOK_IS_CIRCLE_PRESSED,
    HOOK_CONTROLLER_STATE_CLEAR,
    HOOK_CONTROLLER_STATE_CHECK_FOR_INPUT,
    HOOK_KEYBOARD_STATE_CLEAR,
    HOOK_CLEAR_KEYBOARD_HISTORY,
    HOOK_SET_TOUCHED,
    HOOK_GET_TOUCHED_TIME_DELTA,
    HOOK_MOUSE_STATE_CONSTRUCTOR,
    HOOK_MOUSE_STATE_CLEAR,
    HOOK_MOUSE_CHECK_FOR_INPUT,
    HOOK_RECONCILE_TWO_CONTROLLERS_INPUT,
    HOOK_SET_DRUNK_INPUT_DELAY,
    HOOK_START_SHAKE,
    HOOK_START_SHAKE_DISTANCE,
    HOOK_START_SHAKE_TRAIN,
    HOOK_GET_STEERING_LEFT_RIGHT,
    HOOK_GET_STEERING_UP_DOWN,
    HOOK_GET_PED_WALK_LEFT_RIGHT,
    HOOK_GET_PED_WALK_UP_DOWN,
    HOOK_GET_CAR_GUN_UP_DOWN,
    HOOK_GET_CAR_GUN_LEFT_RIGHT,
    HOOK_GET_CAR_GUN_FIRED,
    HOOK_CAR_GUN_JUST_DOWN,
    HOOK_GET_HAND_BRAKE,
    HOOK_GET_BRAKE,
    HOOK_GET_ACCELERATE,
    HOOK_GET_DISPLAY_VITAL_STATS,
    HOOK_GET_WEAPON,
    HOOK_WEAPON_JUST_DOWN,
    HOOK_GET_LOOK_LEFT,
    HOOK_GET_LOOK_RIGHT,
    HOOK_GET_LOOK_BEHIND_FOR_CAR,
    HOOK_GET_LOOK_BEHIND_FOR_PED,
    HOOK_GET_HYDRAULIC_JUMP,
    HOOK_GET_HORN,
    HOOK_HORN_JUST_DOWN,
    HOOK_GET_EXIT_VEHICLE,
    HOOK_GET_MELEE_ATTACK,
    HOOK_MELEE_ATTACK_JUST_DOWN,
    HOOK_GET_ACCELERATE_JUST_DOWN,
    HOOK_SUB_5404F0,
    HOOK_SUB_540530,
    HOOK_NEXT_STATION_JUST_UP,
    HOOK_LAST_STATION_JUST_UP,
    HOOK_CYCLE_WEAPON_LEFT_JUST_DOWN,
    HOOK_CYCLE_WEAPON_RIGHT_JUST_DOWN,
    HOOK_GET_TARGET,
    HOOK_GET_ENTER_TARGETING,
    HOOK_GET_DUCK,
    HOOK_DUCK_JUST_DOWN,
    HOOK_GET_JUMP,
    HOOK_JUMP_JUST_DOWN,
    HOOK_GET_SPRINT,
    HOOK_SPRINT_JUST_DOWN,
    HOOK_SHIFT_TARGET_LEFT_JUST_DOWN,
    HOOK_SHIFT_TARGET_RIGHT_JUST_DOWN,
    HOOK_COLLECT_PICKUP_JUST_DOWN,
    HOOK_GET_FORCE_CAMERA_BEHIND_PLAYER,
    HOOK_SNIPER_ZOOM_IN,
    HOOK_SNIPER_ZOOM_OUT,
    HOOK_GET_GROUP_CONTROL_FORWARD,
    HOOK_GET_GROUP_CONTROL_BACK,
    HOOK_CONVERSATION_YES_JUST_DOWN,
    HOOK_CONVERSATION_NO_JUST_DOWN,
    HOOK_GROUP_CONTROL_FORWARD_JUST_DOWN,
    HOOK_GROUP_CONTROL_BACK_JUST_DOWN,
    HOOK_GET_ANALOGUE_PAD_UP,
    HOOK_GET_ANALOGUE_PAD_DOWN,
    HOOK_GET_ANALOGUE_PAD_LEFT,
    HOOK_GET_ANALOGUE_PAD_RIGHT,
    HOOK_LOOK_AROUND_LEFT_RIGHT,
    HOOK_LOOK_AROUND_UP_DOWN,
    HOOK_GET_PED_WALK_LEFT_RIGHT_PED,
    HOOK_GET_PED_WALK_UP_DOWN_PED,
    HOOK_AIM_WEAPON_LEFT_RIGHT,
    HOOK_AIM_WEAPON_UP_DOWN,
    HOOK_CLEAR,
    HOOK_CLEAR_MOUSE_HISTORY,
    HOOK_INITIALISE,
    HOOK_UPDATE_PADS,
    HOOK_GET_MOUSE_STATE,
    HOOK_PROCESS_PAD,
];
