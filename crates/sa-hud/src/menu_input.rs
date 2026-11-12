//! `CMenuManager::UserInput` (PORT-HUD-3; `AdditionalOptionInput` moved to `menu_map_input.rs`, PORT-MENU). Ported from the 1.0 US bytes;
//! gta-reversed `Frontend/MenuManager_Input.cpp` as reading aid (the bytes win).
//!
//! `this` fields used here (offsets from the bytes, see also `menu_mgr.rs`): +0x30 system menu flags
//! (bit 7 = keyboard enter allowed), +0x54 current item, +0xB8 display the mouse, +0xBC / +0xC0 mouse
//! position, +0xE0 / +0xE4 window mouse position, +0x15D current screen, +0x1AEA / +0x1AEB controller
//! error / user track scan, +0x1AF8 / +0x1AFC old mouse position, +0x1B00 mouse-in-bounds state,
//! +0x1B04 current mouse option. Items of `aScreens` (0x8CE008, 0xE2 per screen, items of 0x12 from
//! +0xA): +0 action, +1 text key[8], +0xE y (int16).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x, xi};
use crate::menu_mgr::{
    analogue_pad_left, analogue_pad_right, audio_report_frontend_event, check_slider_movement_call, get_number_of_menu_options_call,
    get_pad, mouse_wheel_down_call, mouse_wheel_up_call, stretch_y_call, text_get,
};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `aScreens` (0x8CE008)
const SCREENS: usize = 0x8CE008;
/// The option current when `UserInput` started (`oldOption`, byte 0x8CE005)
const OLD_OPTION: usize = 0x8CE005;
/// The previous mouse option (byte 0x8CE004, written only by `UserInput`)
const PREV_MOUSE_OPTION: usize = 0x8CE004;
/// `UserInput`'s static local: its guard dword (bit 0) and the screen it was initialised with
const HOVER_GUARD: usize = 0xBA82BC;
const HOVER_SCREEN: usize = 0xBA82B8;
/// The pause-mode times of the last left / right slider step (0xBA82B0 / 0xBA82B4)
const SLIDE_LEFT_TIME: usize = 0xBA82B0;
const SLIDE_RIGHT_TIME: usize = 0xBA82B4;
/// `CTimer::m_snTimeInMillisecondsPauseMode` (0xB7CB7C)
const TIME_PAUSE_MODE: usize = 0xB7CB7C;
/// `TheText` (0xC1B340), `AudioEngine` (0xB6BC90)
const THE_TEXT: usize = 0xC1B340;
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `CPad::NewKeyState` / `OldKeyState` words: enter, return, escape, left, right
const KEY_ENTER: (usize, usize) = (0xB733CC, 0xB7315C);
const KEY_RETURN: (usize, usize) = (0xB733EA, 0xB7317A);
const KEY_ESC: (usize, usize) = (0xB733A8, 0xB73138);
const KEY_LEFT: (usize, usize) = (0xB733BA, 0xB7314A);
const KEY_RIGHT: (usize, usize) = (0xB733BC, 0xB7314C);
/// `CPad::NewMouseControllerState` / `OldMouseControllerState` bytes: left button, wheel up, wheel down
const MOUSE_LMB: (usize, usize) = (0xB73418, 0xB7342C);
const MOUSE_WHEEL_UP: (usize, usize) = (0xB7341B, 0xB7342F);
const MOUSE_WHEEL_DOWN: (usize, usize) = (0xB7341C, 0xB73430);
/// 1.0 (event volume / speed argument), 388.0
const F1_BITS: u32 = 0x3F80_0000;
const K_388: f32 = f32::from_bits(0x43C2_0000);

orig! {
    /// `CMenuManager::CheckFrontEndUpInput()` (0x573840, thiscall)
    pub fn check_front_end_up_input_call(this: GPtr<u8>) -> u8 = thiscall 0x573840 => crate::menu_mgr::check_front_end_up_input;
    /// `CMenuManager::CheckFrontEndDownInput()` (0x5738B0, thiscall)
    pub fn check_front_end_down_input_call(this: GPtr<u8>) -> u8 = thiscall 0x5738B0 => crate::menu_mgr::check_front_end_down_input;
    /// `CMenuManager::CheckFrontEndLeftInput()` (0x573920, thiscall)
    pub fn check_front_end_left_input_call(this: GPtr<u8>) -> u8 = thiscall 0x573920 => crate::menu_mgr::check_front_end_left_input;
    /// `CMenuManager::CheckFrontEndRightInput()` (0x573990, thiscall)
    pub fn check_front_end_right_input_call(this: GPtr<u8>) -> u8 = thiscall 0x573990 => crate::menu_mgr::check_front_end_right_input;
    /// `CMenuManager::RedefineScreenUserInput(bool* accept, bool* back)` (0x57EF50, thiscall)
    pub fn redefine_screen_user_input(this: GPtr<u8>, accept: GPtr<u8>, back: GPtr<u8>) = thiscall 0x57EF50 => crate::menu_controls::redefine_screen_user_input;
    /// `CMenuManager::AdditionalOptionInput(bool* up, bool* down)` (0x5773D0, thiscall)
    pub fn additional_option_input_call(this: GPtr<u8>, up: GPtr<u8>, down: GPtr<u8>) = thiscall 0x5773D0 => crate::menu_map_input::additional_option_input;
    /// `CMenuManager::ProcessUserInput(bool down, bool up, bool accept, bool back, int8 lr)` (0x57B480, thiscall)
    pub fn process_user_input_call(this: GPtr<u8>, down: u32, up: u32, accept: u32, back: u32, lr: u32) = thiscall 0x57B480 => crate::menu_mgr::process_user_input;
    /// `CPad::GetPedWalkLeftRight()` (0x53FC90, thiscall; AX)
    pub fn pad_get_ped_walk_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FC90 => unported;
    /// Unnamed `CPad` thunk (0x4410E0, thiscall; `jmp` into `.HOODLUM`): d-pad right just pressed,
    /// `[pad+0x16] != 0 && [pad+0x46] == 0` (AL)
    pub fn pad_dpad_right_just_pressed(pad: GPtr<u8>) -> u8 = thiscall 0x4410E0 => unported;
    /// `CFont::GetStringWidth(const GxtChar*, bool full, bool scriptText)` (0x71A0E0, ST0)
    pub fn get_string_width_st0(text: GPtr<u8>, full: u32, script: u32) -> f64 = cdecl 0x71A0E0 => crate::font_print::get_string_width_call_st0;
}

fn b8(p: GPtr<u8>, off: isize) -> u8 {
    rd(p, off)
}

/// Item `i` of the current screen (`aScreens[(int8)+0x15D]` + 0xA + i * 0x12).
fn item(this: GPtr<u8>, i: i32) -> GPtr<u8> {
    let s = i32::from(b8(this, 0x15D) as i8);
    g(SCREENS.wrapping_add(s.wrapping_mul(0xE2).wrapping_add(i.wrapping_mul(0x12)).wrapping_add(0xA) as u32 as usize))
}

/// The action byte of the current item (+0x54).
fn current_action(this: GPtr<u8>) -> u8 {
    b8(item(this, rd(this, 0x54)), 0)
}

/// A key word just pressed: new state set, old one clear.
fn key_just_pressed(k: (usize, usize)) -> bool {
    g::<u16>(k.0).read() != 0 && g::<u16>(k.1).read() == 0
}

/// A mouse byte just pressed: new state set, old one clear.
fn mouse_just_pressed(k: (usize, usize)) -> bool {
    g::<u8>(k.0).read() != 0 && g::<u8>(k.1).read() == 0
}

/// The mouse moved since the last frame: old position non-zero in both axes and different.
fn mouse_moved(this: GPtr<u8>) -> bool {
    let ox: i32 = rd(this, 0x1AF8);
    if ox == 0 {
        return false;
    }
    let oy: i32 = rd(this, 0x1AFC);
    oy != 0 && (ox != rd::<i32>(this, 0xBC) || oy != rd::<i32>(this, 0xC0))
}

fn event(id: u32) {
    audio_report_frontend_event(g(AUDIO_ENGINE), id, 0, F1_BITS);
}

/// Byte table 0x580708 of `UserInput` (action - 2, 0..0x3C), copied from the exe: 1 = the slider
/// sound, 0 = none.
const LR_SOUND: [u8; 61] = [
    0, 1, 1, 0, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0,
];

/// The actions of the slider items (byte tables 0x580630 / 0x5806A0 / 0x5806D4: 0x1B brightness,
/// 0x1C radio volume, 0x1D SFX volume, 0x3D draw distance, 0x3E mouse sensitivity).
fn is_slider(a: u8) -> bool {
    matches!(a, 0x1B | 0x1C | 0x1D | 0x3D | 0x3E)
}

/// `CMenuManager::UserInput()` (0x57FD70, thiscall): the front end's per-frame input. Nothing while a
/// user track scan (+0x1AEB) or a controller error (+0x1AEA) is pending.
///
/// 1. The mouse is shown (+0xB8) when its old position (+0x1AF8 / +0x1AFC) is non-zero in both axes and
///    differs from +0xBC / +0xC0. `oldOption` (0x8CE005) = the item byte.
/// 2. Hover: for each of the 12 items with an action other than 0 / 1, `GetStringWidth` of its text
///    (result discarded), then hovered when `StretchY(y) < mouseY` and `StretchY(y + 26) > mouseY`
///    (x87, mouse y and the item y through `fild` / `fstp dword`). A non-hovered item with the mouse
///    shown sets +0x1B04 and the item to `oldOption` (sign-extended). The first hovered item ends the
///    loop: the static local 0xBA82B8 / guard 0xBA82BC takes the screen once, the byte 0x8CE004 =
///    +0x1B04, +0x1B04 = the item, item and mouse shown when the mouse moved, and +0x1B00 = 2 unless the
///    item is a slider (0x1B, 0x1C, 0x1D, 0x3D, 0x3E).
/// 3. With the mouse shown and the item changed: a title item (action 1) moves the item and +0x1B04 on
///    by one; event 3.
/// 4. The old mouse position takes +0xBC / +0xC0, the new one +0xE0 / +0xE4 clamped to 0..width /
///    0..height (signed).
/// 5. Screen 0x26 (redefine controls): `RedefineScreenUserInput(&accept, &back)` when +0x1AEA is clear,
///    then straight to step 6. Other screens: with more than one option `AdditionalOptionInput(&up,
///    &down)`, then front-end down / up (mouse hidden); accept by enter / return just released (item 0
///    of the pause screen 0x29) or just pressed (else) with +0x30 bit 7, or the pad's cross released /
///    pressed (+0x20 / +0x50); accept by the left button released on a menu item (+0x1B00 == 2): on the
///    map screen below `StretchY(388)`, elsewhere with the mouse shown; the left button held on a
///    slider arrow (+0x1B00 6..15: even right, odd left) runs `CheckSliderMovement(+-1)`; left button,
///    left / right keys, d-pad left (+0x14 / +0x44), the d-pad right thunk 0x4410E0, analogue left /
///    right or the wheel just pressed play event 1 on 0x1B / 0x1C / 0x3D / 0x3E items and event 5 on
///    0x1D; triangle (+0x1E / +0x4E) or escape just pressed set back (mouse hidden) except on screens
///    0x22, 0x29, 0x10, 0x16, 0x12, 0xE, 0x2B; down, up or accept clear back.
/// 6. (All screens.) Left key held, `GetPedWalkLeftRight() < 0` or d-pad left held (+0x14): when more
///    than 200 ms of pause-mode time passed since 0xBA82B0, `lr = -1` on the 0x15 and slider items and
///    0xBA82B0 = now (for every item). Else right key, walk > 0 or d-pad right (+0x16) likewise with
///    0xBA82B4 and `lr = 1`.
/// 7. Wheel up / down just pressed off the map screen: `lr = 1` / `-1` (wheel up on the map screen skips
///    the wheel-down test). Front-end right / left: `lr = +-1`, mouse hidden. With `lr` set on screens
///    3, 0x1A, 4, 0x1B, 0x24, 0x27, event 1 unless the item's action is 2, 5, 7, 8, 9, 0x1B..0x1D, 0x20,
///    0x39, 0x3D or 0x3E (byte table 0x580708; actions 0, 1 and above 0x3E play it).
/// 8. `ProcessUserInput(down, up, accept, back, lr)`.
///
/// Differences from gta-reversed (the bytes win): the redefine screen 0x26 still runs steps 6 and 7
/// (gr skips them); the hover test and the reset to `oldOption` run per item inside the loop (gr
/// resets after the loop), the hover bounds are strict on both sides (gr `<=` / `>=`); gr does not
/// write 0x8CE004; the slider timers are the globals 0xBA82B0 / 0xBA82B4 (gr: fields) and are restarted
/// after 200 ms whatever the item (gr: only on slider items), and the 0x15 action also steps (gr: the
/// five sliders and `MENU_ACTION_STAT`); accept on item 0 of the pause screen tests "released" through
/// the key words directly; the slider sound table differs from gr's lists (step 5 / 7 above).
///
/// Uninitialised stack: the original passes `ProcessUserInput` five dwords whose low byte it wrote
/// (the upper three bytes are residue of this frame and of earlier callees, not derivable from the
/// bytes); every reader in `ProcessUserInput` and its callees reads the low byte only. The port passes
/// the bytes zero-extended.
#[hook(addr = 0x57FD70, conv = thiscall, name = "CMenuManager::UserInput", expect = "83 EC 18 56 8B F1")]
pub fn user_input(this: GPtr<u8>) {
    if b8(this, 0x1AEB) != 0 || b8(this, 0x1AEA) != 0 {
        return;
    }
    let mut accept = [0u8; 4];
    let mut back = [0u8; 4];
    let mut up = [0u8; 4];
    let mut down = [0u8; 4];
    if b8(this, 0xB8) == 0 && mouse_moved(this) {
        wr::<u8>(this, 0xB8, 1);
    }
    g::<u8>(OLD_OPTION).write(b8(this, 0x54));

    // 2. hover
    let mut i: i8 = 0;
    loop {
        let a = b8(item(this, i32::from(i)), 0);
        if a != 0 && a != 1 {
            let text = text_get(g(THE_TEXT), item(this, i32::from(i)).byte_add(1));
            get_string_width_st0(text, 1, 0);
            let my = xi(rd(this, 0xC0)).to_f32();
            let y = i32::from(rd::<i16>(item(this, i32::from(i)), 0xE));
            let top = stretch_y_call(this, xi(y).to_f32());
            let hovered = if top.fcom(x(my)).test_parity(0x05) {
                false
            } else {
                let my = xi(rd(this, 0xC0)).to_f32();
                let y = i32::from(rd::<i16>(item(this, i32::from(i)), 0xE)).wrapping_add(0x1A);
                let bottom = stretch_y_call(this, xi(y).to_f32());
                !bottom.fcom(x(my)).test_any(0x41)
            };
            if hovered {
                if g::<u8>(HOVER_GUARD).read() & 1 == 0 {
                    g::<u32>(HOVER_GUARD).write(g::<u32>(HOVER_GUARD).read() | 1);
                    g::<i32>(HOVER_SCREEN).write(i32::from(b8(this, 0x15D) as i8));
                }
                let ii = i32::from(i);
                g::<u8>(PREV_MOUSE_OPTION).write(b8(this, 0x1B04));
                wr(this, 0x1B04, ii);
                if mouse_moved(this) {
                    wr(this, 0x54, ii);
                    wr::<u8>(this, 0xB8, 1);
                }
                if !is_slider(b8(item(this, ii), 0)) {
                    wr::<u32>(this, 0x1B00, 2);
                }
                break;
            }
            if b8(this, 0xB8) != 0 {
                let o = i32::from(g::<u8>(OLD_OPTION).read() as i8);
                wr(this, 0x1B04, o);
                wr(this, 0x54, o);
            }
        }
        i += 1;
        if i >= 0xC {
            break;
        }
    }

    // 3. selection changed with the mouse
    if b8(this, 0xB8) != 0 {
        let cur: i32 = rd(this, 0x54);
        if i32::from(g::<u8>(OLD_OPTION).read() as i8) != cur {
            if b8(item(this, cur), 0) == 1 {
                wr(this, 0x54, cur.wrapping_add(1));
                wr(this, 0x1B04, rd::<i32>(this, 0x1B04).wrapping_add(1));
            }
            event(3);
        }
    }

    // 4. mouse position
    let wy: i32 = rd(this, 0xE4);
    wr(this, 0x1AFC, rd::<i32>(this, 0xC0));
    let wx: i32 = rd(this, 0xE0);
    wr(this, 0x1AF8, rd::<i32>(this, 0xBC));
    wr(this, 0xBC, wx);
    wr(this, 0xC0, wy);
    if wx < 0 {
        wr::<i32>(this, 0xBC, 0);
    }
    let w = g::<i32>(RS_MAX_WIDTH).read();
    if rd::<i32>(this, 0xBC) > w {
        wr(this, 0xBC, w);
    }
    if wy < 0 {
        wr::<i32>(this, 0xC0, 0);
    }
    let h = g::<i32>(RS_MAX_HEIGHT).read();
    if rd::<i32>(this, 0xC0) > h {
        wr(this, 0xC0, h);
    }

    // 5. navigation, accept, back
    let mut lr: u8 = 0;
    if b8(this, 0x15D) == 0x26 {
        if b8(this, 0x1AEA) == 0 {
            let ap = local(&mut accept);
            let bp = local(&mut back);
            redefine_screen_user_input(this, ap, bp);
        }
    } else {
        menu_navigation(this, &mut accept, &mut back, &mut up, &mut down);
    }

    // 6. held left / right with a 200 ms repeat
    let left = {
        get_pad(0);
        g::<u16>(KEY_LEFT.0).read() != 0 || {
            get_pad(0);
            let pad = get_pad(0);
            pad_get_ped_walk_left_right(pad) < 0
        } || rd::<u16>(get_pad(0), 0x14) != 0
    };
    if left {
        let now = g::<u32>(TIME_PAUSE_MODE).read();
        if now.wrapping_sub(g::<u32>(SLIDE_LEFT_TIME).read()) > 200 {
            if lr_steps(current_action(this)) {
                lr = 0xFF;
            }
            g::<u32>(SLIDE_LEFT_TIME).write(now);
        }
    } else {
        let right = {
            get_pad(0);
            g::<u16>(KEY_RIGHT.0).read() != 0 || {
                get_pad(0);
                let pad = get_pad(0);
                pad_get_ped_walk_left_right(pad) > 0
            } || rd::<u16>(get_pad(0), 0x16) != 0
        };
        if right {
            let now = g::<u32>(TIME_PAUSE_MODE).read();
            if now.wrapping_sub(g::<u32>(SLIDE_RIGHT_TIME).read()) > 200 {
                if lr_steps(current_action(this)) {
                    lr = 1;
                }
                g::<u32>(SLIDE_RIGHT_TIME).write(now);
            }
        }
    }

    // 7. wheel and front-end left / right
    get_pad(0);
    if mouse_just_pressed(MOUSE_WHEEL_UP) {
        if b8(this, 0x15D) != 5 {
            lr = 1;
        }
    } else {
        get_pad(0);
        if mouse_just_pressed(MOUSE_WHEEL_DOWN) && b8(this, 0x15D) != 5 {
            lr = 0xFF;
        }
    }
    let mut sound = true;
    if check_front_end_right_input_call(this) != 0 {
        wr::<u8>(this, 0xB8, 0);
        lr = 1;
    } else if check_front_end_left_input_call(this) != 0 {
        wr::<u8>(this, 0xB8, 0);
        lr = 0xFF;
    } else if lr == 0 {
        sound = false;
    }
    if sound && matches!(b8(this, 0x15D), 3 | 0x1A | 4 | 0x1B | 0x24 | 0x27) {
        let k = u32::from(current_action(this)).wrapping_sub(2);
        if k > 0x3C || LR_SOUND[k as usize] != 0 {
            event(1);
        }
    }

    process_user_input_call(
        this,
        u32::from(down[0]),
        u32::from(up[0]),
        u32::from(accept[0]),
        u32::from(back[0]),
        u32::from(lr),
    );
}

/// The held left / right step applies to this action (byte tables 0x5806A0 / 0x5806D4, action -
/// 0x15 in 0..0x29: 0x15 and the sliders).
fn lr_steps(a: u8) -> bool {
    a == 0x15 || is_slider(a)
}

fn local<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(core::ptr::from_mut(v)).cast()
}

/// Step 5 of [`user_input`] for every screen but 0x26 (0x580076..0x5803CF).
fn menu_navigation(this: GPtr<u8>, accept: &mut [u8; 4], back: &mut [u8; 4], up: &mut [u8; 4], down: &mut [u8; 4]) {
    if get_number_of_menu_options_call(this) > 1 {
        additional_option_input_call(this, local(up), local(down));
        if check_front_end_down_input_call(this) != 0 {
            down[0] = 1;
            wr::<u8>(this, 0xB8, 0);
        } else if check_front_end_up_input_call(this) != 0 {
            up[0] = 1;
            wr::<u8>(this, 0xB8, 0);
        }
    }

    // accept: enter / return (with +0x30 bit 7) or the pad's cross
    let pause_first = rd::<i32>(this, 0x54) == 0 && b8(this, 0x15D) == 0x29;
    get_pad(0);
    let key = if pause_first {
        let released = |k: (usize, usize)| g::<u16>(k.0).read() == 0 && g::<u16>(k.1).read() != 0;
        released(KEY_ENTER) || released(KEY_RETURN)
    } else {
        key_just_pressed(KEY_ENTER) || key_just_pressed(KEY_RETURN)
    };
    let mut hit = key && (b8(this, 0x30) as i8) < 0;
    if !hit {
        let pad = get_pad(0);
        hit = if pause_first {
            rd::<u16>(pad, 0x20) == 0 && rd::<u16>(pad, 0x50) != 0
        } else {
            rd::<u16>(pad, 0x20) != 0 && rd::<u16>(pad, 0x50) == 0
        };
    }
    if hit {
        wr::<u8>(this, 0xB8, 0);
        accept[0] = 1;
    }

    // accept: left button released on a menu item
    get_pad(0);
    if g::<u8>(MOUSE_LMB.0).read() == 0 && g::<u8>(MOUSE_LMB.1).read() != 0 && rd::<i32>(this, 0x1B00) == 2 {
        if b8(this, 0x15D) == 5 {
            let my = xi(rd(this, 0xC0)).to_f32();
            if !stretch_y_call(this, K_388).fcom(x(my)).test_parity(0x05) {
                accept[0] = 1;
            }
        } else if b8(this, 0xB8) != 0 {
            accept[0] = 1;
        }
    }

    // slider arrows under a held left button
    let lmb = g::<u8>(MOUSE_LMB.0).read();
    get_pad(0);
    if lmb != 0 {
        let k = rd::<u32>(this, 0x1B00).wrapping_sub(6);
        if k <= 9 {
            check_slider_movement_call(this, if k & 1 == 0 { 1 } else { 0xFFFF_FFFF });
        }
    }

    // the slider sound
    get_pad(0);
    let pressed = mouse_just_pressed(MOUSE_LMB)
        || {
            get_pad(0);
            key_just_pressed(KEY_LEFT)
        }
        || {
            get_pad(0);
            key_just_pressed(KEY_RIGHT)
        }
        || {
            let pad = get_pad(0);
            rd::<u16>(pad, 0x14) != 0 && rd::<u16>(pad, 0x44) == 0
        }
        || pad_dpad_right_just_pressed(get_pad(0)) != 0
        || analogue_pad_left(get_pad(0)) != 0
        || analogue_pad_right(get_pad(0)) != 0
        || {
            get_pad(0);
            mouse_wheel_up_call() != 0
        }
        || {
            get_pad(0);
            mouse_wheel_down_call() != 0
        };
    if pressed {
        match current_action(this) {
            0x1B | 0x1C | 0x3D | 0x3E => event(1),
            0x1D => event(5),
            _ => {}
        }
    }

    // back: triangle or escape
    let pad = get_pad(0);
    let tri = rd::<u16>(pad, 0x1E) != 0 && rd::<u16>(pad, 0x4E) == 0;
    let esc = tri || {
        get_pad(0);
        key_just_pressed(KEY_ESC)
    };
    if esc && !matches!(b8(this, 0x15D), 0x22 | 0x29 | 0x10 | 0x16 | 0x12 | 0xE | 0x2B) {
        wr::<u8>(this, 0xB8, 0);
        back[0] = 1;
    }
    if down[0] != 0 || up[0] != 0 || accept[0] != 0 {
        back[0] = 0;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_USER_INPUT, crate::menu_map_input::HOOK_ADDITIONAL_OPTION_INPUT];
