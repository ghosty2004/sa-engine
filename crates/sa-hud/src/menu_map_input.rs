//! `CMenuManager::AdditionalOptionInput` (0x5773D0, PORT-MENU): the stats-file key, the briefs scroll and
//! the whole map screen input (waypoint, zoom, pan, cursor tracking, view clamp, legend options menu,
//! legend toggle). Ported from the 1.0 US bytes (0x5773D0..0x578D57); gta-reversed
//! `Frontend/MenuManager_Input.cpp` and `docs/re/specs/HUD/CMenuManager.deviations.md` as reading aids
//! (the bytes win). Replaces PORT-HUD-3's partial port (ADR-002 `fallback` on the map path).
//!
//! `this` fields (offsets from the bytes): +0x08 brief row, +0x2C target blip handle, +0x30 system menu
//! (i8), +0x45..+0x49 the five map blip prefs, +0x4A map legend, +0x5B all streaming stuff loaded, +0x64
//! map zoom, +0x68 / +0x6C map origin, +0x70 / +0x74 map cursor (world), +0xB8 display the mouse, +0xBC /
//! +0xC0 mouse position, +0xEA pad, +0x15D current screen. Two fields are addressed absolutely, as the
//! bytes do: `FrontEndMenuManager.m_bDrawingMap` (0xBA67A1) and the map's last action time (0xBA8280).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use crate::common::{f32_of, ftol32, g, rd, wr, x, xi};
use crate::menu_input::{check_front_end_down_input_call, check_front_end_up_input_call};
use crate::menu_mgr::{audio_report_frontend_event, get_pad, stretch_x_call, stretch_y_call};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `FrontEndMenuManager.m_bDrawingMap` (byte 0xBA67A1, written by address)
const DRAWING_MAP: usize = 0xBA67A1;
/// The map's last pan / zoom step time (0xBA8280, `FrontEndMenuManager` +0x1B38, by address)
const LAST_ACTION_TIME: usize = 0xBA8280;
/// `CTimer::m_snTimeInMillisecondsPauseMode`
const TIME_PAUSE_MODE: usize = 0xB7CB7C;
/// `CTheScripts::HideAllFrontEndMapBlips`, `CTheScripts::bPlayerIsOffTheMap`
const HIDE_MAP_BLIPS: usize = 0xA444A1;
const PLAYER_OFF_THE_MAP: usize = 0xA444A4;
/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CMessages::PreviousBriefs` (0xC1A570, entries of 0x20, +0 text)
const PREVIOUS_BRIEFS: usize = 0xC1A570;
/// `CPad::NewMouseControllerState`: lmb (0xB73418), wheel up (0xB7341B), wheel down (0xB7341C)
const MOUSE_LMB: usize = 0xB73418;
const MOUSE_WHEEL_UP: usize = 0xB7341B;
const MOUSE_WHEEL_DOWN: usize = 0xB7341C;
/// `CPad::NewKeyState` words: page up / down, up, down, left, right
const KEY_PGUP: usize = 0xB733B2;
const KEY_PGDN: usize = 0xB733B4;
const KEY_UP: usize = 0xB733B6;
const KEY_DOWN: usize = 0xB733B8;
const KEY_LEFT: usize = 0xB733BA;
const KEY_RIGHT: usize = 0xB733BC;
/// `CPad::NewKeyState.standardKeys` / `OldKeyState.standardKeys` (word per character)
const STD_KEYS_NEW: usize = 0xB731A8;
const STD_KEYS_OLD: usize = 0xB72F38;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;

// .rdata strings
/// `"CODEWAY"` (0x8653E8), `"FEP_OPT"` (0x8653D4), `"FED_BL1"`..`"FED_BL5"` (0x8653AC + 8 * i)
const STR_CODEWAY: usize = 0x8653E8;
const STR_FEP_OPT: usize = 0x8653D4;
const STR_FED_BL1: usize = 0x8653AC;
/// `"FEM_ON"` (0x86539C, 7 bytes copied: dword, word, byte) and `"FEM_OFF"` (0x8653A4, 8 bytes: two dwords)
const FEM_ON: [u8; 7] = *b"FEM_ON\0";
const FEM_OFF: [u8; 8] = *b"FEM_OFF\0";

// .rdata float constants
/// 320.0 (0x8653F4), 224.0 (0x8653F0), 1.0 (0x858624), 1100.0 (0x864F94), 7.0 (0x858F48), 21.0 (0x8653E4)
const K_320: f32 = f32::from_bits(0x43A0_0000);
const K_224: f32 = f32::from_bits(0x4360_0000);
const K_1: f32 = f32::from_bits(0x3F80_0000);
const K_1100: f32 = f32::from_bits(0x4489_8000);
const K_7: f32 = f32::from_bits(0x40E0_0000);
const K_21: f32 = f32::from_bits(0x41A8_0000);
/// 576.0 (0x8653E0), 64.0 (0x859A44), 384.0 (0x865254), 300.0 (0x858FD8), 0.6 (0x858CC8), 128.0 (0x858BF4)
const K_576: f32 = f32::from_bits(0x4410_0000);
const K_64: f32 = f32::from_bits(0x4280_0000);
const K_384: f32 = f32::from_bits(0x43C0_0000);
const K_300: f32 = f32::from_bits(0x4396_0000);
const K_0_6: f32 = f32::from_bits(0x3F19_999A);
const K_128: f32 = f32::from_bits(0x4300_0000);
/// 580.0 (0x865260), 1/128 (0x858B88), 14.0 (0x859030), -14.0 (0x8653DC), 60.0 (0x858B34), 388.0 (0x86525C)
const K_580: f32 = f32::from_bits(0x4411_0000);
const K_1_128: f32 = f32::from_bits(0x3C00_0000);
const K_14: f32 = f32::from_bits(0x4160_0000);
const K_M14: f32 = f32::from_bits(0xC160_0000);
const K_60: f32 = f32::from_bits(0x4270_0000);
const K_388: f32 = f32::from_bits(0x43C2_0000);
/// 3000.0 (0x859A94), -3000.0 (0x859A90): the cursor clamp (stored as the immediates 0x453B8000 / 0xC53B8000)
const K_3000: f32 = f32::from_bits(0x453B_8000);
const K_M3000: f32 = f32::from_bits(0xC53B_8000);
/// Immediates: zoom limits 1100.0 / 300.0, the legend menu's 77 / 250 / 100
const IMM_ZOOM_MAX: u32 = 0x4489_8000;
const IMM_ZOOM_MIN: u32 = 0x4396_0000;
const IMM_77: f32 = f32::from_bits(0x429A_0000);
const IMM_250: f32 = f32::from_bits(0x437A_0000);
const IMM_100: f32 = f32::from_bits(0x42C8_0000);

orig! {
    /// `CPad::IsMouseRButtonPressed()` (0x572E70; called with ECX = the pad, which it does not read)
    fn is_mouse_rbutton_pressed_call() -> u8 = cdecl 0x572E70 => crate::menu_mgr::is_mouse_rbutton_pressed;
    /// `CRadar::ClearBlip(int32)` (0x587CE0)
    fn clear_blip_call(h: i32) = cdecl 0x587CE0 => crate::radar::clear_blip;
    /// `CRadar::SetBlipSprite(int32, eRadarSprite)` (0x583D70)
    fn set_blip_sprite_call(h: i32, sprite: i32) = cdecl 0x583D70 => crate::radar::set_blip_sprite;
    /// `CRadar::TransformRealWorldPointToRadarSpace(CVector2D& out, const CVector2D& in)` (0x583530)
    fn transform_real_world_point_to_radar_space_call(out: GPtr<f32>, inp: GPtr<f32>) = cdecl 0x583530 => crate::radar::transform_real_world_point_to_radar_space;
    /// `CRadar::LimitRadarPoint(CVector2D&)` (0x5832F0, ST0; discarded here)
    fn limit_radar_point_call_st0(p: GPtr<f32>) -> f64 = cdecl 0x5832F0 => limit_radar_point_st0_rust;
    /// `CRadar::TransformRadarPointToScreenSpace(CVector2D& out, const CVector2D& in)` (0x583480)
    fn transform_radar_point_to_screen_space_call(out: GPtr<f32>, inp: GPtr<f32>) = cdecl 0x583480 => crate::radar::transform_radar_point_to_screen_space;
    /// `CMenuSystem::CreateNewMenu(type, title, x, y, width, columns, interactive, background, alignment)`
    /// (0x582300; AL)
    fn create_new_menu_call(kind: u32, title: GPtr<u8>, px: f32, py: f32, width: f32, columns: u32, interactive: u32, background: u32, alignment: u32) -> u8 = cdecl 0x582300 => create_new_menu_rust;
    /// `CMenuSystem::InsertMenu(menu, column, header, rows[12])` (0x581E00)
    fn insert_menu_call(menu: u32, column: u32, header: GPtr<u8>, r0: GPtr<u8>, r1: GPtr<u8>, r2: GPtr<u8>, r3: GPtr<u8>, r4: GPtr<u8>, r5: GPtr<u8>, r6: GPtr<u8>, r7: GPtr<u8>, r8: GPtr<u8>, r9: GPtr<u8>, r10: GPtr<u8>, r11: GPtr<u8>) = cdecl 0x581E00 => crate::menu2::insert_menu;
    /// `CMenuSystem::CheckForAccept(uint8)` (0x5807C0; AL)
    fn check_for_accept_call(menu: u32) -> u8 = cdecl 0x5807C0 => crate::menu2::check_for_accept;
    /// `CMenuSystem::CheckForSelected(uint8)` (0x5807E0; AL)
    fn check_for_selected_call(menu: u8) -> u8 = cdecl 0x5807E0 => crate::menu::check_for_selected;
    /// `CMenuSystem::SetActiveMenuItem(uint8 menu, uint8 row)` (0x5820C0)
    fn set_active_menu_item_call(menu: u32, row: u32) = cdecl 0x5820C0 => crate::menu2::set_active_menu_item;
    /// `CMenuSystem::SwitchOffMenu(uint8)` (0x580750)
    fn switch_off_menu_call(menu: u8) = cdecl 0x580750 => crate::menu::switch_off_menu;
    /// `CMenuManager::SaveStatsToFile()` (0x57DDE0, thiscall)
    fn save_stats_to_file_call(this: GPtr<u8>) = thiscall 0x57DDE0 => crate::menu_stats_file::save_stats_to_file;
}

/// The Rust path of [`limit_radar_point_call_st0`] (phases B/C).
#[allow(dead_code)]
fn limit_radar_point_st0_rust(p: GPtr<f32>) -> f64 {
    crate::radar::limit_radar_point(p).to_f64()
}

/// The Rust path of [`create_new_menu_call`] (phases B/C).
#[allow(dead_code, clippy::too_many_arguments)]
fn create_new_menu_rust(kind: u32, title: GPtr<u8>, px: f32, py: f32, width: f32, columns: u32, interactive: u32, background: u32, alignment: u32) -> u8 {
    crate::menu2::create_new_menu(kind, title, px.to_bits(), py.to_bits(), width.to_bits(), columns, interactive, background, alignment)
}

fn b8(p: GPtr<u8>, off: isize) -> u8 {
    rd(p, off)
}

fn f(p: GPtr<u8>, off: isize) -> f32 {
    rd(p, off)
}

fn local<T, const N: usize>(buf: &mut [T; N]) -> GPtr<T> {
    GPtr::from_raw(buf.as_mut_ptr())
}

/// `GetPad(this->m_nPlayerNumber)` (the result only where the bytes use it).
fn pad(this: GPtr<u8>) -> GPtr<u8> {
    get_pad(u32::from(b8(this, 0xEA)))
}

fn word(addr: usize) -> u16 {
    g::<u16>(addr).read()
}

fn byte(addr: usize) -> u8 {
    g::<u8>(addr).read()
}

/// `(u32)(CTimer::m_snTimeInMillisecondsPauseMode - lastActionTime) > 20`
fn step_due() -> bool {
    g::<u32>(TIME_PAUSE_MODE).read().wrapping_sub(g::<u32>(LAST_ACTION_TIME).read()) > 20
}

/// `test ah, 5; jp skip`: proceed only when ST0 < m (C0 alone).
fn below(c: Cmp) -> bool {
    !c.test_parity(0x05)
}

/// `test ah, 0x41; jne skip`: proceed only when ST0 > m (no C0 / C3).
fn above(c: Cmp) -> bool {
    !c.test_any(0x41)
}

/// The cursor's screen position: `TransformRealWorldPointToRadarSpace`, `LimitRadarPoint` (ST0 popped),
/// `TransformRadarPointToScreenSpace`. The input vector is built through `fld` / `fstp` (or an `fst` of the
/// value just computed), so a signalling NaN reaches the callee quiet.
fn cursor_screen(this: GPtr<u8>) -> [f32; 2] {
    let mut inp = [f32_of(x(f(this, 0x70))), f32_of(x(f(this, 0x74)))];
    let mut radar = [0f32; 2];
    let mut screen = [0f32; 2];
    transform_real_world_point_to_radar_space_call(local(&mut radar), local(&mut inp));
    limit_radar_point_call_st0(local(&mut radar));
    transform_radar_point_to_screen_space_call(local(&mut screen), local(&mut radar));
    screen
}

/// `cursor.c += d` (`fld; fadd/fsub m; fst`) and the new screen position.
fn nudge(this: GPtr<u8>, off: isize, v: X87) -> [f32; 2] {
    wr(this, off, f32_of(v));
    cursor_screen(this)
}

/// `ReportFrontendAudioEvent(AudioEngine, id, 0, 1.0)`
fn event(id: u32) {
    audio_report_frontend_event(g(AUDIO_ENGINE), id, 0, K_1.to_bits());
}

/// The five ON / OFF buffers of the legend options menu (10 bytes apart in the original frame; the
/// unused tail bytes of a buffer are not read by `InsertMenu`'s `strcpy`) and the column-1 insert.
fn insert_blip_prefs(this: GPtr<u8>, bufs: &mut [[u8; 10]; 5]) {
    for (i, buf) in bufs.iter_mut().enumerate() {
        if b8(this, 0x45 + i as isize) != 0 {
            buf[..7].copy_from_slice(&FEM_ON);
        } else {
            buf[..8].copy_from_slice(&FEM_OFF);
        }
    }
    let n = GPtr::<u8>::NULL;
    let menu = u32::from(b8(this, 0x30));
    let [b0, b1, b2, b3, b4] = bufs;
    insert_menu_call(menu, 1, n, local(b0), local(b1), local(b2), local(b3), local(b4), n, n, n, n, n, n, n);
}

/// `CMenuManager::AdditionalOptionInput(bool* up, bool* down)` (0x5773D0, thiscall, `ret 8`). By the
/// current screen (+0x15D, signed):
///
/// - 0 (stats): 'S' or 's' just pressed (standard key words, `GetPad(+0xEA)` before each test, results
///   unused) -> `SaveStatsToFile`.
/// - 2 (briefs): front-end up with the row (+8, byte) below 0x13 and `PreviousBriefs[row + 1]` with a
///   text: row + 1, `*up = 1`; then front-end down with row > 3: row - 1, `*down = 1`.
/// - 5 (map) with the streaming flag +0x5B clear: [`map_input`].
/// - otherwise (and screen 5 with +0x5B set): nothing.
#[hook(addr = 0x5773D0, conv = thiscall, name = "CMenuManager::AdditionalOptionInput", expect = "55 8B EC 83 E4 F8",
       unverified = "menu-1", route = "L2", hazards = "heap-writes",
       recs = "menus-hud,menus-map-controls",
       note = "PORT-MENU: the map path (pan / zoom / waypoint / legend menu) is now Rust; screen 0 calls SaveStatsToFile (file write)")]
pub fn additional_option_input(this: GPtr<u8>, up: GPtr<u8>, down: GPtr<u8>) {
    match b8(this, 0x15D) as i8 {
        0 => {
            let key = |c: usize, old: bool| {
                pad(this);
                word(if old { STD_KEYS_OLD } else { STD_KEYS_NEW } + c * 2)
            };
            let hit = (key(0x53, false) != 0 && key(0x53, true) == 0) || (key(0x73, false) != 0 && key(0x73, true) == 0);
            if hit {
                save_stats_to_file_call(this);
            }
        }
        2 => {
            if check_front_end_up_input_call(this) != 0 {
                let row = b8(this, 8);
                if row < 0x13 && g::<u32>(PREVIOUS_BRIEFS + 0x20 + usize::from(row) * 0x20).read() != 0 {
                    wr(this, 8, row + 1);
                    wr::<u8>(up, 0, 1);
                }
            }
            if check_front_end_down_input_call(this) != 0 {
                let row = b8(this, 8);
                if row > 3 {
                    wr(this, 8, row - 1);
                    wr::<u8>(down, 0, 1);
                }
            }
        }
        5 if b8(this, 0x5B) == 0 => map_input(this),
        _ => {}
    }
}

/// The view bounds the map input computes once at entry (all `fstp dword` locals).
struct View {
    /// origin.y - zoom, origin.y + zoom, origin.x - zoom, origin.x + zoom
    bottom: f32,
    top: f32,
    left: f32,
    right: f32,
    /// 320 - origin.x, 224 - origin.y
    dist_x: f32,
    dist_y: f32,
    /// dist_x * (1 / zoom), (1 / zoom) * dist_y
    rel_x: f32,
    rel_y: f32,
}

/// The map screen part of [`additional_option_input`] (0x57740E..0x578C74):
///
/// 1. `m_bDrawingMap` (0xBA67A1) = 1; the view bounds of [`View`].
/// 2. With no legend options menu (+0x30 < 0):
///    - **Waypoint**: circle just pressed, or the right button just pressed (`IsMouseRButtonPressed`)
///      with the left button up, or 'T' / 't' just pressed; unless the scripts hide the map blips or the
///      player is off the map: an existing target blip (+0x2C) is cleared (event 2), else event 1 and
///      `SetCoordBlip(4, {cursor.x, cursor.y, 0}, 0, 2, "CODEWAY")` with `SetBlipSprite(.., 0x29)`.
///    - **Zoom in**: LS2 held without RS2, wheel up or page up; every step (> 20 ms since 0xBA8280): zoom
///      < 1100: zoom += 7 (+ 21 more with the wheel), origin -= rel * zoom - dist; else zoom = 1100. Then
///      the cursor is walked back in 1-unit steps while its screen x > 576, x < 64, y < 64, y > 384.
///    - **Zoom out**: RS2 without LS2, wheel down or page down; each step: zoom > 300: zoom -= 7 (21), the
///      same origin update; else zoom = 300.
///    - **Pan input**: keys up / down / left / right (+-128), the left stick x, y, x again (raw words), the
///      d-pad (`ftol(+-word * 0.6)`); i16 values.
///    - **Mouse** (+0xB8): inside the frame (StretchY(60) < y < StretchY(388), StretchX(60) < x <
///      StretchX(580), against the f32 mouse): with the left button held the pan comes from the mouse's
///      offset to the screen centre (`(m / StretchX(320)) * 128 - 128` left of it, `(m - StretchX(320)) /
///      StretchX(320) * 128` right of it; same in y with 224) and the step is 50; without it the cursor
///      tracks the mouse in 14-unit steps (no pan, straight to step 3). Outside the frame: step 50, the
///      cursor moves only. Without the mouse: step 2, the map may pan.
///    - **Pan**: per axis and sign, the map origin moves by `pan / 128 * 7` (when due) while the view
///      still extends past the frame edge (60 / 580 / 60 / 388, the entry bounds), the cursor is on that
///      side of the centre (320 / 224) and the map may pan, then the cursor is walked back to the centre
///      line; otherwise the cursor moves by `pan / 128 * 7 * step` when due and inside 64..576 / 64..384.
/// 3. **Clamp**: the origin keeps the view inside 60..580 x 60..388 (fresh bounds), the cursor inside
///    +-3000.
/// 4. **Legend options menu**: space held (new and old key words) with no menu: event 1,
///    `CreateNewMenu(0, "FEP_OPT", StretchX(77), StretchY(250), StretchX(100), 2, 1, 1, 1)`, column 0
///    `FED_BL1..5`, column 1 ON / OFF from +0x45..+0x49. With the menu: an accepted row toggles its pref,
///    the ON / OFF column is rebuilt and the selection moves on (row 4 wraps to 0); space released (new
///    and old clear): event 2, `SwitchOffMenu`, +0x30 = -99.
/// 5. 'L' or 'l' just pressed toggles the legend (+0x4A). When due, 0xBA8280 = now. `m_bDrawingMap` = 0.
///
/// Differences from gta-reversed (the bytes win): the left stick x is tested twice (after y too); the
/// legend menu opens while space is *held* (new and old key words set) and closes when both are clear;
/// the cursor-tracking loops compare `StretchX(screen.x)` / `StretchY(screen.y)` against the mouse
/// position rounded to f32 (the original passes EDX = the last screen coordinate it loaded, which is
/// the current one on every path); the pan step multiplies in the order `pan * (1/128) * (7 * step)`
/// (`fimul`) / `(pan * (1/128)) * (7 * step)` (`fmulp`) as the bytes; gr's frame-rate scaled zoom is not
/// in the original.
fn map_input(this: GPtr<u8>) {
    g::<u8>(DRAWING_MAP).write(1);
    let zoom = f(this, 0x64);
    let inv = x(K_1).fdiv_m(zoom);
    let dist_x = f32_of(x(K_320).fsub_m(f(this, 0x68)));
    let dist_y = f32_of(x(K_224).fsub_m(f(this, 0x6C)));
    let v = View {
        bottom: f32_of(x(f(this, 0x6C)).fsub_m(f(this, 0x64))),
        top: f32_of(x(f(this, 0x64)).fadd_m(f(this, 0x6C))),
        left: f32_of(x(f(this, 0x68)).fsub_m(f(this, 0x64))),
        right: f32_of(x(f(this, 0x68)).fadd_m(f(this, 0x64))),
        dist_x,
        dist_y,
        rel_x: f32_of(x(dist_x).fmul(inv)),
        rel_y: f32_of(inv.fmul_m(dist_y)),
    };
    if (b8(this, 0x30) as i8) < 0 {
        map_controls(this, &v);
    }
    legend_menu(this);
    let l_just = |c: usize| {
        pad(this);
        if word(STD_KEYS_NEW + c * 2) == 0 {
            return false;
        }
        pad(this);
        word(STD_KEYS_OLD + c * 2) == 0
    };
    if l_just(0x4C) || l_just(0x6C) {
        wr::<u8>(this, 0x4A, u8::from(b8(this, 0x4A) == 0));
    }
    let now = g::<u32>(TIME_PAUSE_MODE).read();
    if now.wrapping_sub(g::<u32>(LAST_ACTION_TIME).read()) > 20 {
        g::<u32>(LAST_ACTION_TIME).write(now);
    }
    g::<u8>(DRAWING_MAP).write(0);
}

/// Step 2 and 3 of [`map_input`] (0x577483..0x578874).
fn map_controls(this: GPtr<u8>, v: &View) {
    waypoint(this);
    zoom_in(this, v);
    zoom_out(this, v);
    let mut screen = cursor_screen(this);
    let (mut pan_x, mut pan_y) = pan_input(this);

    // the mouse (0x577C50)
    let mut step: u8 = 2;
    let mut free = false;
    if b8(this, 0xB8) == 0 {
        free = true;
    } else {
        step = 50;
        let my = f32_of(xi(rd(this, 0xC0)));
        let inside = below(stretch_y_call(this, K_60).fcom(x(my)))
            && above(stretch_y_call(this, K_388).fcom(x(my)))
            && {
                let mx = f32_of(xi(rd(this, 0xBC)));
                below(stretch_x_call(this, K_60).fcom(x(mx))) && above(stretch_x_call(this, K_580).fcom(x(mx)))
            };
        if inside {
            screen = cursor_screen(this);
            let lmb = byte(MOUSE_LMB);
            pad(this);
            if lmb == 0 {
                track_mouse(this, screen);
                clamp_view(this);
                return;
            }
            free = true;
            if let Some(p) = mouse_pan(this, rd(this, 0xBC), g::<i32>(RS_MAX_WIDTH).read(), stretch_x_call, K_320) {
                pan_x = p as u16 as i16;
            }
            if let Some(p) = mouse_pan(this, rd(this, 0xC0), g::<i32>(RS_MAX_HEIGHT).read(), stretch_y_call, K_224) {
                pan_y = p as u16 as i16;
            }
        }
    }
    let step7 = i32::from(step) * 7;

    // pan x > 0 (0x577E10)
    if pan_x > 0 {
        if x(v.right).fcom(x(K_580)).test_parity(0x41) && x(screen[0]).fcom(x(K_320)).test_parity(0x05) && free {
            if step_due() {
                let d = xi(i32::from(pan_x)).fmul_m(K_1_128).fmul_m(K_7);
                wr(this, 0x68, f32_of(d.fsubr_m(f(this, 0x68))));
            }
            screen = cursor_screen(this);
            while above(x(v.right).fcom(x(K_580))) && below(x(screen[0]).fcom(x(K_320))) {
                screen = nudge(this, 0x70, x(f(this, 0x70)).fadd_m(K_1));
            }
        } else if step_due() && below(x(screen[0]).fcom(x(K_576))) {
            let d = xi(-i32::from(pan_x)).fmul_m(K_1_128).fmul(xi(step7));
            wr(this, 0x70, f32_of(d.fsubr_m(f(this, 0x70))));
        }
    }
    // pan x < 0 (0x5782C3)
    if pan_x < 0 {
        if x(v.left).fcom(x(K_60)).test_any(0x01) && x(screen[0]).fcom(x(K_320)).test_any(0x41) && free {
            if step_due() {
                let d = xi(-i32::from(pan_x)).fmul_m(K_1_128).fmul_m(K_7);
                wr(this, 0x68, f32_of(d.fadd_m(f(this, 0x68))));
            }
            screen = cursor_screen(this);
            while below(x(v.left).fcom(x(K_60))) && above(x(screen[0]).fcom(x(K_320))) {
                screen = nudge(this, 0x70, x(f(this, 0x70)).fsub_m(K_1));
            }
        } else if step_due() && above(x(screen[0]).fcom(x(K_64))) {
            let d = xi(step7).fmul(xi(i32::from(pan_x)).fmul_m(K_1_128));
            wr(this, 0x70, f32_of(d.fadd_m(f(this, 0x70))));
        }
    }
    // pan y > 0 (0x578456)
    if pan_y > 0 {
        if x(v.top).fcom(x(K_388)).test_parity(0x41) && x(screen[1]).fcom(x(K_224)).test_parity(0x05) && free {
            if step_due() {
                let d = xi(i32::from(pan_y)).fmul_m(K_1_128).fmul_m(K_7);
                wr(this, 0x6C, f32_of(d.fsubr_m(f(this, 0x6C))));
            }
            screen = cursor_screen(this);
            while above(x(v.top).fcom(x(K_388))) && below(x(screen[1]).fcom(x(K_224))) {
                screen = nudge(this, 0x74, x(f(this, 0x74)).fsub_m(K_1));
            }
        } else if step_due() && below(x(screen[1]).fcom(x(K_384))) {
            let d = xi(step7).fmul(xi(i32::from(pan_y)).fmul_m(K_1_128));
            wr(this, 0x74, f32_of(d.fsubr_m(f(this, 0x74))));
        }
    }
    // pan y < 0 (0x5785EC)
    if pan_y < 0 {
        if x(v.bottom).fcom(x(K_60)).test_any(0x01) && x(screen[1]).fcom(x(K_224)).test_any(0x41) && free {
            if step_due() {
                let d = xi(-i32::from(pan_y)).fmul_m(K_1_128).fmul_m(K_7);
                wr(this, 0x6C, f32_of(d.fadd_m(f(this, 0x6C))));
            }
            screen = cursor_screen(this);
            while below(x(v.bottom).fcom(x(K_60))) && above(x(screen[1]).fcom(x(K_224))) {
                screen = nudge(this, 0x74, x(f(this, 0x74)).fadd_m(K_1));
            }
        } else if step_due() && above(x(screen[1]).fcom(x(K_64))) {
            let d = xi(-i32::from(pan_y)).fmul_m(K_1_128).fmul(xi(step7));
            wr(this, 0x74, f32_of(d.fadd_m(f(this, 0x74))));
        }
    }
    clamp_view(this);
}

/// The waypoint toggle (0x577483..0x5775E2).
fn waypoint(this: GPtr<u8>) {
    let circle = {
        let p = pad(this);
        rd::<u16>(p, 0x22) != 0 && rd::<u16>(pad(this), 0x52) == 0
    };
    let hit = circle
        || {
            pad(this);
            is_mouse_rbutton_pressed_call() != 0 && {
                let lmb = byte(MOUSE_LMB);
                pad(this);
                lmb == 0
            }
        }
        || {
            pad(this);
            word(STD_KEYS_NEW + 0x54 * 2) != 0 && {
                pad(this);
                word(STD_KEYS_OLD + 0x54 * 2) == 0
            }
        }
        || {
            pad(this);
            word(STD_KEYS_NEW + 0x74 * 2) != 0 && {
                pad(this);
                word(STD_KEYS_OLD + 0x74 * 2) == 0
            }
        };
    if !hit || byte(HIDE_MAP_BLIPS) != 0 || byte(PLAYER_OFF_THE_MAP) != 0 {
        return;
    }
    let blip: i32 = rd(this, 0x2C);
    if blip != 0 {
        event(2);
        clear_blip_call(blip);
        wr::<i32>(this, 0x2C, 0);
    } else {
        event(1);
        let px = f(this, 0x70);
        let py = f32_of(x(f(this, 0x74)));
        let h = crate::radar::set_coord_blip_call(4, px, py, 0.0, 0, 2, g(STR_CODEWAY));
        wr::<i32>(this, 0x2C, h);
        set_blip_sprite_call(h, 0x29);
    }
}

/// The zoom update both directions share: `origin -= rel * zoom - dist`.
fn zoom_origin(this: GPtr<u8>, v: &View) {
    let ox = x(v.rel_x).fmul_m(f(this, 0x64)).fsub_m(v.dist_x).fsubr_m(f(this, 0x68));
    wr(this, 0x68, f32_of(ox));
    let oy = x(v.rel_y).fmul_m(f(this, 0x64)).fsub_m(v.dist_y).fsubr_m(f(this, 0x6C));
    wr(this, 0x6C, f32_of(oy));
}

/// Zoom in (0x5775E5..0x577916).
fn zoom_in(this: GPtr<u8>, v: &View) {
    let held = {
        let p = pad(this);
        rd::<u16>(p, 0xA) != 0 && rd::<u16>(pad(this), 0xE) == 0
    } || {
        let wheel = byte(MOUSE_WHEEL_UP);
        pad(this);
        wheel != 0
    } || {
        pad(this);
        word(KEY_PGUP) != 0
    };
    if !held || !step_due() {
        return;
    }
    if below(x(f(this, 0x64)).fcom(x(K_1100))) {
        wr(this, 0x64, f32_of(x(f(this, 0x64)).fadd_m(K_7)));
        let wheel = byte(MOUSE_WHEEL_UP);
        pad(this);
        if wheel != 0 {
            wr(this, 0x64, f32_of(x(f(this, 0x64)).fadd_m(K_21)));
        }
        zoom_origin(this, v);
    } else {
        wr::<u32>(this, 0x64, IMM_ZOOM_MAX);
    }
    // the cursor back into the view, 1 unit per step
    let mut s = cursor_screen(this);
    while above(x(s[0]).fcom(x(K_576))) {
        s = nudge(this, 0x70, x(f(this, 0x70)).fsub_m(K_1));
    }
    while below(x(s[0]).fcom(x(K_64))) {
        s = nudge(this, 0x70, x(f(this, 0x70)).fadd_m(K_1));
    }
    while below(x(s[1]).fcom(x(K_64))) {
        s = nudge(this, 0x74, x(f(this, 0x74)).fsub_m(K_1));
    }
    while above(x(s[1]).fcom(x(K_384))) {
        s = nudge(this, 0x74, x(f(this, 0x74)).fadd_m(K_1));
    }
}

/// Zoom out (0x577918..0x5779F7).
fn zoom_out(this: GPtr<u8>, v: &View) {
    let held = {
        let p = pad(this);
        rd::<u16>(p, 0xE) != 0 && rd::<u16>(pad(this), 0xA) == 0
    } || {
        let wheel = byte(MOUSE_WHEEL_DOWN);
        pad(this);
        wheel != 0
    } || {
        pad(this);
        word(KEY_PGDN) != 0
    };
    if !held || !step_due() {
        return;
    }
    if above(x(f(this, 0x64)).fcom(x(K_300))) {
        wr(this, 0x64, f32_of(x(f(this, 0x64)).fsub_m(K_7)));
        let wheel = byte(MOUSE_WHEEL_DOWN);
        pad(this);
        if wheel != 0 {
            wr(this, 0x64, f32_of(x(f(this, 0x64)).fsub_m(K_21)));
        }
        zoom_origin(this, v);
    } else {
        wr::<u32>(this, 0x64, IMM_ZOOM_MIN);
    }
}

/// The keyboard / stick / d-pad pan values (0x577A3F..0x577C4E): (x, y) as the 16-bit words the
/// original tests (`di` / the low word of its local).
fn pan_input(this: GPtr<u8>) -> (i16, i16) {
    let (mut px, mut py): (i16, i16) = (0, 0);
    let key = |a: usize| {
        pad(this);
        word(a) != 0
    };
    if key(KEY_UP) {
        py = -128;
    }
    if key(KEY_DOWN) {
        py = 128;
    }
    if key(KEY_LEFT) {
        px = -128;
    }
    if key(KEY_RIGHT) {
        px = 128;
    }
    let stick = |off: isize| -> Option<i16> {
        if rd::<i16>(pad(this), off) != 0 { Some(rd::<i16>(pad(this), off)) } else { None }
    };
    if let Some(s) = stick(0) {
        px = s;
    }
    if let Some(s) = stick(2) {
        py = s;
    }
    if let Some(s) = stick(0) {
        px = s;
    }
    // d-pad: ftol(+-(word * 0.6)), the low word kept
    let dpad = |off: isize, neg: bool| -> Option<i16> {
        if rd::<i16>(pad(this), off) == 0 {
            return None;
        }
        let w = rd::<i16>(pad(this), off);
        let v = xi(i32::from(w)).fmul_m(K_0_6);
        Some(ftol32(if neg { v.fchs() } else { v }) as u16 as i16)
    };
    if let Some(s) = dpad(0x10, true) {
        py = s;
    }
    if let Some(s) = dpad(0x12, false) {
        py = s;
    }
    if let Some(s) = dpad(0x14, true) {
        px = s;
    }
    if let Some(s) = dpad(0x16, false) {
        px = s;
    }
    (px, py)
}

/// The mouse pan of one axis with the left button held (0x577D43..0x577E08, 0x577F5A..0x577F90): `m`
/// against `max / 2` (signed, truncated): below `ftol(-(128 - (m / S(c)) * 128))`, above
/// `ftol(f32(m - S(c)) / S(c) * 128)`, equal: none.
fn mouse_pan(this: GPtr<u8>, m: i32, max: i32, stretch: fn(GPtr<u8>, f32) -> X87, c: f32) -> Option<i32> {
    let half = max / 2;
    let mf = f32_of(xi(m));
    if m < half {
        let s = stretch(this, c);
        Some(ftol32(s.fdivr_m(mf).fmul_m(K_128).fsubr_m(K_128).fchs()))
    } else if m > half {
        let d = f32_of(stretch(this, c).fsubr_m(mf));
        let s = stretch(this, c);
        Some(ftol32(s.fdivr_m(d).fmul_m(K_128)))
    } else {
        None
    }
}

/// The cursor follows the mouse in 14-unit steps (0x577F95..0x578261), no pan.
fn track_mouse(this: GPtr<u8>, mut s: [f32; 2]) {
    let mx = || f32_of(xi(rd(this, 0xBC)));
    let my = || f32_of(xi(rd(this, 0xC0)));
    if above(stretch_x_call(this, s[0]).fcom(x(mx()))) {
        while above(x(s[0]).fcom(x(K_64))) {
            s = nudge(this, 0x70, x(f(this, 0x70)).fsub_m(K_14));
            if !above(stretch_x_call(this, s[0]).fcom(x(mx()))) {
                break;
            }
        }
    }
    if below(stretch_x_call(this, s[0]).fcom(x(mx()))) {
        while below(x(s[0]).fcom(x(K_576))) {
            s = nudge(this, 0x70, x(f(this, 0x70)).fsub_m(K_M14));
            if !below(stretch_x_call(this, s[0]).fcom(x(mx()))) {
                break;
            }
        }
    }
    if above(stretch_y_call(this, s[1]).fcom(x(my()))) {
        while above(x(s[1]).fcom(x(K_64))) {
            s = nudge(this, 0x74, x(f(this, 0x74)).fadd_m(K_14));
            if !above(stretch_y_call(this, s[1]).fcom(x(my()))) {
                break;
            }
        }
    }
    if below(stretch_y_call(this, s[1]).fcom(x(my()))) {
        while below(x(s[1]).fcom(x(K_384))) {
            s = nudge(this, 0x74, x(f(this, 0x74)).fsub_m(K_14));
            if !below(stretch_y_call(this, s[1]).fcom(x(my()))) {
                break;
            }
        }
    }
}

/// The view and cursor clamp (0x578785..0x578874). `left` and `bottom` stay on the x87 stack (not
/// rounded to f32), `top` and `right` are stored locals.
fn clamp_view(this: GPtr<u8>) {
    let bottom = x(f(this, 0x6C)).fsub_m(f(this, 0x64));
    let top = f32_of(x(f(this, 0x64)).fadd_m(f(this, 0x6C)));
    let left = x(f(this, 0x68)).fsub_m(f(this, 0x64));
    let right = f32_of(x(f(this, 0x68)).fadd_m(f(this, 0x64)));
    if above(left.fcom(x(K_60))) {
        wr(this, 0x68, f32_of(left.fsub_m(K_60).fsubr_m(f(this, 0x68))));
    }
    if below(x(right).fcom(x(K_580))) {
        wr(this, 0x68, f32_of(x(K_580).fsub_m(right).fadd_m(f(this, 0x68))));
    }
    if above(bottom.fcom(x(K_60))) {
        wr(this, 0x6C, f32_of(bottom.fsub_m(K_60).fsubr_m(f(this, 0x6C))));
    }
    if below(x(top).fcom(x(K_388))) {
        wr(this, 0x6C, f32_of(x(K_388).fsub_m(top).fadd_m(f(this, 0x6C))));
    }
    for off in [0x70, 0x74] {
        if above(x(f(this, off)).fcom(x(K_3000))) {
            wr::<u32>(this, off, K_3000.to_bits());
        }
        if below(x(f(this, off)).fcom(x(K_M3000))) {
            wr::<u32>(this, off, K_M3000.to_bits());
        }
    }
}

/// The legend options menu (0x578877..0x578BE4).
fn legend_menu(this: GPtr<u8>) {
    let space_new = || {
        pad(this);
        word(STD_KEYS_NEW + 0x20 * 2) != 0
    };
    let space_old = || {
        pad(this);
        word(STD_KEYS_OLD + 0x20 * 2) != 0
    };
    let mut bufs = [[0u8; 10]; 5];
    if space_new() && space_old() && b8(this, 0x30) == 0x9D {
        event(1);
        let w = f32_of(stretch_x_call(this, IMM_100));
        let py = f32_of(stretch_y_call(this, IMM_250));
        let px = f32_of(stretch_x_call(this, IMM_77));
        let menu = create_new_menu_call(0, g(STR_FEP_OPT), px, py, w, 2, 1, 1, 1);
        wr::<u8>(this, 0x30, menu);
        let n = GPtr::<u8>::NULL;
        let r = |i: usize| g::<u8>(STR_FED_BL1 + 8 * i);
        insert_menu_call(u32::from(menu), 0, n, r(0), r(1), r(2), r(3), r(4), n, n, n, n, n, n, n);
        insert_blip_prefs(this, &mut bufs);
    }
    if (b8(this, 0x30) as i8) < 0 {
        return;
    }
    let item = check_for_accept_call(u32::from(b8(this, 0x30)));
    if (item as i8) >= 0 {
        let off = 0x45 + isize::from(item as i8);
        wr::<u8>(this, off, u8::from(b8(this, off) == 0));
        insert_blip_prefs(this, &mut bufs);
        let menu = b8(this, 0x30);
        if check_for_selected_call(menu) == 4 {
            set_active_menu_item_call(u32::from(b8(this, 0x30)), 0);
        } else {
            let row = check_for_selected_call(b8(this, 0x30)).wrapping_add(1);
            set_active_menu_item_call(u32::from(b8(this, 0x30)), u32::from(row));
        }
    }
    if !space_new() && !space_old() {
        event(2);
        switch_off_menu_call(b8(this, 0x30));
        wr::<u8>(this, 0x30, 0x9D);
    }
}

/// This module's registrations ([`HOOK_ADDITIONAL_OPTION_INPUT`] is listed in `menu_input::HOOKS`, where
/// the PORT-HUD-3 hook stood, so the table order is unchanged).
pub const HOOKS: &[HookReg] = &[];
